use std::fmt;
use std::io::{self, Read, Write};
use std::sync::{
    Arc, Mutex, MutexGuard, PoisonError,
    atomic::{AtomicBool, Ordering},
};

use crate::{
    SshConnectRequest, SshSessionError, SshSessionResult, SshShellConnector, SshShellReader,
    SshShellSession, SshShellWriter,
};
use rterm_types::TerminalSize;

use rterm_runtime::{
    SessionControl, SessionExit, SessionExitSignal, SessionInterrupt, SessionParts,
    SessionTransport,
};

type SharedWriter = Arc<Mutex<Box<dyn SshShellWriter>>>;

/// Established SSH shell adapted to the runtime session ownership contract.
pub struct SshTransport {
    reader: SshReader,
    writer: SshWriter,
    control: SshControl,
    interrupt: SshInterrupt,
}

impl fmt::Debug for SshTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SshTransport(..)")
    }
}

impl SshTransport {
    /// Connects and adapts a remote shell session.
    ///
    /// # Errors
    ///
    /// Returns the connector's complete authentication, startup, or channel error.
    pub fn connect(
        connector: &mut dyn SshShellConnector,
        request: SshConnectRequest,
    ) -> Result<Self, SshSessionError> {
        connector.connect(request).map(Self::from_session)
    }

    /// Adapts an already established SSH shell session.
    #[must_use]
    pub fn from_session(session: Box<dyn SshShellSession>) -> Self {
        let (reader, writer) = session.into_read_writer();
        let writer = Arc::new(Mutex::new(writer));
        let cancelled = Arc::new(AtomicBool::new(false));
        let finished = Arc::new(AtomicBool::new(false));
        let result = Arc::new(Mutex::new(SshSessionResult::default()));
        Self {
            reader: SshReader {
                reader,
                cancelled: Arc::clone(&cancelled),
                finished: Arc::clone(&finished),
                result: Arc::clone(&result),
            },
            writer: SshWriter {
                writer: Arc::clone(&writer),
                cancelled: Arc::clone(&cancelled),
            },
            control: SshControl {
                writer,
                finished,
                result,
                closed: false,
            },
            interrupt: SshInterrupt { cancelled },
        }
    }

    /// Returns SSH-native halves for the compatibility pump after applying the
    /// runtime transport's cancellation and result tracking adapters.
    #[must_use]
    pub fn into_shell_halves(self) -> (Box<dyn SshShellReader>, Box<dyn SshShellWriter>) {
        (Box::new(self.reader), Box::new(self.writer))
    }
}

impl SessionTransport for SshTransport {
    type Reader = SshReader;
    type Writer = SshWriter;
    type Control = SshControl;
    type Interrupt = SshInterrupt;

    fn split(self) -> SessionParts<Self::Reader, Self::Writer, Self::Control, Self::Interrupt> {
        SessionParts::new(self.reader, self.writer, self.control, self.interrupt)
    }
}

/// Cancellable remote reader that records final status for the control plane.
pub struct SshReader {
    reader: Box<dyn SshShellReader>,
    cancelled: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    result: Arc<Mutex<SshSessionResult>>,
}

impl Read for SshReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = self
            .reader
            .read_cancellable(buffer, &self.cancelled)
            .map_err(|error| ssh_io_error(&error))?
            .ok_or_else(|| io::Error::from(io::ErrorKind::Interrupted))?;
        *lock(&self.result) = self.reader.session_result();
        if count == 0 {
            self.finished.store(true, Ordering::Release);
        }
        Ok(count)
    }
}

impl SshShellReader for SshReader {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, SshSessionError> {
        self.reader
            .read_cancellable(buffer, &self.cancelled)?
            .ok_or_else(|| SshSessionError::new("SSH runtime read interrupted"))
            .inspect(|count| {
                *lock(&self.result) = self.reader.session_result();
                if *count == 0 {
                    self.finished.store(true, Ordering::Release);
                }
            })
    }

    fn read_cancellable(
        &mut self,
        buffer: &mut [u8],
        cancelled: &AtomicBool,
    ) -> Result<Option<usize>, SshSessionError> {
        if self.cancelled.load(Ordering::Acquire) || cancelled.load(Ordering::Acquire) {
            return Ok(None);
        }
        let count = self.reader.read_cancellable(buffer, cancelled)?;
        *lock(&self.result) = self.reader.session_result();
        if count == Some(0) {
            self.finished.store(true, Ordering::Release);
        }
        Ok(count)
    }

    fn session_result(&self) -> SshSessionResult {
        lock(&self.result).clone()
    }
}

/// Partial-write-preserving remote writer owned by the pane worker.
pub struct SshWriter {
    writer: SharedWriter,
    cancelled: Arc<AtomicBool>,
}

impl Write for SshWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        lock(&self.writer)
            .write_cancellable(buffer, &self.cancelled)
            .map_err(|error| ssh_io_error(&error))?
            .ok_or_else(|| io::Error::from(io::ErrorKind::Interrupted))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl SshShellWriter for SshWriter {
    fn write(&mut self, bytes: &[u8]) -> Result<usize, SshSessionError> {
        lock(&self.writer)
            .write_cancellable(bytes, &self.cancelled)?
            .ok_or_else(|| SshSessionError::new("SSH runtime write interrupted"))
    }

    fn write_cancellable(
        &mut self,
        bytes: &[u8],
        cancelled: &AtomicBool,
    ) -> Result<Option<usize>, SshSessionError> {
        if self.cancelled.load(Ordering::Acquire) || cancelled.load(Ordering::Acquire) {
            return Ok(None);
        }
        lock(&self.writer).write_cancellable(bytes, cancelled)
    }

    fn resize(&mut self, size: rterm_types::SshTerminalSize) -> Result<(), SshSessionError> {
        lock(&self.writer).resize(size)
    }

    fn keepalive(&mut self) -> Result<(), SshSessionError> {
        lock(&self.writer).keepalive()
    }

    fn finish_input(&mut self) -> Result<(), SshSessionError> {
        lock(&self.writer).finish_input()
    }

    fn close(&mut self) -> Result<(), SshSessionError> {
        lock(&self.writer).close()
    }
}

/// SSH resize, exit, and orderly-close control retained by the pane worker.
pub struct SshControl {
    writer: SharedWriter,
    finished: Arc<AtomicBool>,
    result: Arc<Mutex<SshSessionResult>>,
    closed: bool,
}

impl SessionControl for SshControl {
    fn resize(&mut self, size: TerminalSize) -> io::Result<()> {
        lock(&self.writer)
            .resize(size.into())
            .map_err(|error| ssh_io_error(&error))
    }

    fn poll_exit(&mut self) -> io::Result<Option<SessionExit>> {
        Ok(self
            .finished
            .load(Ordering::Acquire)
            .then(|| ssh_exit(&lock(&self.result))))
    }

    fn begin_close(&mut self) -> io::Result<()> {
        if !self.closed {
            lock(&self.writer)
                .close()
                .map_err(|error| ssh_io_error(&error))?;
            self.closed = true;
        }
        Ok(())
    }
}

/// Cloneable cancellation flag observed by native SSH reader and writer waits.
#[derive(Debug, Clone)]
pub struct SshInterrupt {
    cancelled: Arc<AtomicBool>,
}

impl SessionInterrupt for SshInterrupt {
    fn interrupt(&self) -> io::Result<()> {
        self.cancelled.store(true, Ordering::Release);
        Ok(())
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn ssh_exit(result: &SshSessionResult) -> SessionExit {
    SessionExit {
        status: result.exit_status,
        signal: result.exit_signal.as_ref().map(|signal| SessionExitSignal {
            name: signal.name.clone(),
            core_dumped: signal.core_dumped,
            error_message: signal.error_message.clone(),
            lang_tag: signal.lang_tag.clone(),
        }),
    }
}

fn ssh_io_error(error: &SshSessionError) -> io::Error {
    io::Error::other(format!("SSH session operation failed: {error}"))
}
