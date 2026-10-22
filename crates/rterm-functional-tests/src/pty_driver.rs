use std::{
    error::Error,
    fmt,
    io::{self, Read, Write},
    path::Path,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use rterm_pty::{PtyMasterCloseStatus, PtySession, PtySize};

use crate::hermetic_network::hermetic_pty_command;

const CURSOR_QUERY: &[u8] = b"\x1b[6n";
const CURSOR_RESPONSE: &[u8] = b"\x1b[1;1R";
pub const TRANSPORT_WRITE_MARKER: &str = "fixture-transport-write-observed";

pub struct PtyFixtureDriver {
    session: PtySession,
    writer: Option<Box<dyn Write + Send>>,
    chunks: mpsc::Receiver<io::Result<Vec<u8>>>,
    reader_thread: Option<thread::JoinHandle<()>>,
    deadline: Instant,
    timeout: Duration,
    output: Vec<u8>,
    query_match: usize,
    terminal_query_responses: u64,
    reader_eof: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyFixtureResult {
    pub exit_code: u32,
    pub output: Vec<u8>,
    pub terminal_query_responses: u64,
    pub child_process_reaped: bool,
    pub reader_joined: bool,
    pub master_closed: bool,
}

impl PtyFixtureResult {
    #[must_use]
    pub const fn resources_zero(&self) -> bool {
        self.child_process_reaped && self.reader_joined && self.master_closed
    }

    /// Reports whether the fixture received a terminal response written by the
    /// application under test.
    #[must_use]
    pub fn transport_write_observed(&self) -> bool {
        self.output
            .windows(TRANSPORT_WRITE_MARKER.len())
            .any(|window| window == TRANSPORT_WRITE_MARKER.as_bytes())
    }
}

#[derive(Debug)]
pub enum PtyFixtureError {
    Pty(rterm_pty::PtyError),
    Io(io::Error),
    TimedOut(Duration),
    ReaderDisconnected,
    ReaderPanicked,
    MasterClose(PtyMasterCloseStatus),
}

impl fmt::Display for PtyFixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pty(source) => write!(formatter, "fixture PTY: {source}"),
            Self::Io(source) => write!(formatter, "fixture I/O: {source}"),
            Self::TimedOut(timeout) => write!(formatter, "fixture exceeded {timeout:?}"),
            Self::ReaderDisconnected => formatter.write_str("fixture reader disconnected"),
            Self::ReaderPanicked => formatter.write_str("fixture reader panicked"),
            Self::MasterClose(status) => write!(formatter, "fixture master close: {status:?}"),
        }
    }
}

impl Error for PtyFixtureError {}

impl From<rterm_pty::PtyError> for PtyFixtureError {
    fn from(source: rterm_pty::PtyError) -> Self {
        Self::Pty(source)
    }
}

impl From<io::Error> for PtyFixtureError {
    fn from(source: io::Error) -> Self {
        Self::Io(source)
    }
}

impl PtyFixtureDriver {
    /// Starts one fixture mode behind a real platform PTY.
    ///
    /// # Errors
    ///
    /// Returns an error when PTY creation, process spawn, or deadline setup fails.
    pub fn spawn(
        executable: &Path,
        mode: &str,
        columns: u16,
        rows: u16,
        timeout: Duration,
    ) -> Result<Self, PtyFixtureError> {
        Self::spawn_with_args(executable, [mode], columns, rows, timeout)
    }

    /// Starts a fixture with explicit arguments behind a real platform PTY.
    ///
    /// # Errors
    ///
    /// Returns an error when PTY creation, process spawn, reader setup, or deadline setup fails.
    pub fn spawn_with_args<I, S>(
        executable: &Path,
        args: I,
        columns: u16,
        rows: u16,
        timeout: Duration,
    ) -> Result<Self, PtyFixtureError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let command = hermetic_pty_command(executable.to_string_lossy()).with_args(args);
        let size = PtySize::try_new(columns, rows)?;
        let mut session = PtySession::spawn(&command, size)?;
        let mut reader = session.take_reader()?;
        let writer = session.take_writer()?;
        let (sender, chunks) = mpsc::channel();
        let reader_thread = thread::Builder::new()
            .name("rterm-functional-pty-reader".to_owned())
            .spawn(move || {
                let mut buffer = [0_u8; 8192];
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => {
                            let _ = sender.send(Ok(Vec::new()));
                            return;
                        }
                        Ok(count) => {
                            if sender.send(Ok(buffer[..count].to_vec())).is_err() {
                                return;
                            }
                        }
                        Err(error) => {
                            let _ = sender.send(Err(error));
                            return;
                        }
                    }
                }
            })?;
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or(PtyFixtureError::TimedOut(timeout))?;
        Ok(Self {
            session,
            writer: Some(writer),
            chunks,
            reader_thread: Some(reader_thread),
            deadline,
            timeout,
            output: Vec::new(),
            query_match: 0,
            terminal_query_responses: 0,
            reader_eof: false,
        })
    }

    /// Writes bytes through the PTY after draining currently available output.
    ///
    /// # Errors
    ///
    /// Returns an error when PTY input or output handling fails.
    pub fn write(mut self, bytes: &[u8]) -> Result<Self, PtyFixtureError> {
        self.drain_available()?;
        let writer = self.writer.as_mut().ok_or_else(|| {
            PtyFixtureError::Io(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "fixture writer closed",
            ))
        })?;
        writer.write_all(bytes)?;
        writer.flush()?;
        Ok(self)
    }

    /// Waits until a real PTY output marker has been observed.
    ///
    /// # Errors
    ///
    /// Returns an error when the reader fails or the original deadline expires.
    pub fn wait_for_output(mut self, marker: &[u8]) -> Result<Self, PtyFixtureError> {
        while !self
            .output
            .windows(marker.len())
            .any(|window| window == marker)
        {
            let remaining = self.deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(PtyFixtureError::TimedOut(self.timeout));
            }
            self.receive_one(remaining.min(Duration::from_millis(20)))?;
        }
        Ok(self)
    }

    /// Caps the remaining fixture lifetime for the cleanup phase.
    ///
    /// The bound can only be shortened; the original scenario deadline is
    /// never extended.
    #[must_use]
    pub fn cap_remaining_timeout(mut self, timeout: Duration) -> Self {
        let now = Instant::now();
        let requested = now.checked_add(timeout).unwrap_or(now);
        self.deadline = self.deadline.min(requested);
        self.timeout = self.deadline.saturating_duration_since(now);
        self
    }

    /// Interrupts a live fixture, reaps its child, joins its reader, and closes
    /// the PTY master before returning the captured generation.
    ///
    /// # Errors
    ///
    /// Returns an error when termination or any cleanup ownership proof fails.
    pub fn disconnect(mut self) -> Result<PtyFixtureResult, PtyFixtureError> {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(PtyFixtureError::TimedOut(self.timeout));
        }
        let status = self.session.terminate(remaining)?;
        self.finish_with_status(&status)
    }

    /// Waits for fixture exit and proves reader, child, and PTY-master cleanup.
    ///
    /// # Errors
    ///
    /// Returns an error on timeout, I/O failure, reader panic, or incomplete close.
    pub fn finish(mut self) -> Result<PtyFixtureResult, PtyFixtureError> {
        let status = loop {
            self.drain_available()?;
            if let Some(status) = self.session.try_wait()? {
                break status;
            }
            let remaining = self.deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                let _ = self.session.terminate(Duration::from_secs(2));
                return Err(PtyFixtureError::TimedOut(self.timeout));
            }
            self.receive_one(remaining.min(Duration::from_millis(20)))?;
        };

        self.finish_with_status(&status)
    }

    fn finish_with_status(
        mut self,
        status: &rterm_pty::PtyExitStatus,
    ) -> Result<PtyFixtureResult, PtyFixtureError> {
        let mut close = self.session.begin_master_close();
        drop(self.writer.take());
        while Instant::now() < self.deadline {
            match self.chunks.recv_timeout(
                self.deadline
                    .saturating_duration_since(Instant::now())
                    .min(Duration::from_millis(20)),
            ) {
                Ok(Ok(chunk)) if chunk.is_empty() => break,
                Ok(Ok(chunk)) => self.observe_chunk(&chunk)?,
                Ok(Err(error)) if error.kind() == io::ErrorKind::BrokenPipe => break,
                Ok(Err(error)) => return Err(PtyFixtureError::Io(error)),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        let reader_joined = self
            .reader_thread
            .take()
            .is_none_or(|reader| reader.join().is_ok());
        if !reader_joined {
            return Err(PtyFixtureError::ReaderPanicked);
        }
        let close_status = close.finish_before(self.deadline);
        let master_closed = matches!(close_status, PtyMasterCloseStatus::Completed);
        if !master_closed {
            return Err(PtyFixtureError::MasterClose(close_status));
        }
        Ok(PtyFixtureResult {
            exit_code: status.exit_code(),
            output: self.output,
            terminal_query_responses: self.terminal_query_responses,
            child_process_reaped: true,
            reader_joined,
            master_closed,
        })
    }

    fn drain_available(&mut self) -> Result<(), PtyFixtureError> {
        loop {
            match self.chunks.try_recv() {
                Ok(Ok(chunk)) if chunk.is_empty() => {
                    self.reader_eof = true;
                    return Ok(());
                }
                Ok(Ok(chunk)) => self.observe_chunk(&chunk)?,
                Ok(Err(error)) => return Err(PtyFixtureError::Io(error)),
                Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => return Ok(()),
            }
        }
    }

    fn receive_one(&mut self, timeout: Duration) -> Result<(), PtyFixtureError> {
        match self.chunks.recv_timeout(timeout) {
            Ok(Ok(chunk)) if chunk.is_empty() => {
                self.reader_eof = true;
                Ok(())
            }
            Ok(Ok(chunk)) => self.observe_chunk(&chunk),
            Ok(Err(error)) => Err(PtyFixtureError::Io(error)),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(()),
            Err(mpsc::RecvTimeoutError::Disconnected) if self.reader_eof => Ok(()),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(PtyFixtureError::ReaderDisconnected),
        }
    }

    fn observe_chunk(&mut self, chunk: &[u8]) -> Result<(), PtyFixtureError> {
        self.output.extend_from_slice(chunk);
        let queries = observe_queries(&mut self.query_match, chunk);
        if queries > 0 {
            let writer = self.writer.as_mut().ok_or_else(|| {
                PtyFixtureError::Io(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "fixture query arrived after writer closed",
                ))
            })?;
            for _ in 0..queries {
                writer.write_all(CURSOR_RESPONSE)?;
            }
            writer.flush()?;
            self.terminal_query_responses = self
                .terminal_query_responses
                .saturating_add(u64::try_from(queries).unwrap_or(u64::MAX));
        }
        Ok(())
    }
}

fn observe_queries(matched: &mut usize, chunk: &[u8]) -> usize {
    let mut queries = 0;
    for byte in chunk {
        if *byte == CURSOR_QUERY[*matched] {
            *matched += 1;
            if *matched == CURSOR_QUERY.len() {
                queries += 1;
                *matched = 0;
            }
        } else {
            *matched = usize::from(*byte == CURSOR_QUERY[0]);
        }
    }
    queries
}
