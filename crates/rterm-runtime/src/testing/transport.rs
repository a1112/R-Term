use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

use rterm_types::TerminalSize;

use crate::{SessionControl, SessionExit, SessionInterrupt, SessionParts, SessionTransport};

/// One deterministic result in a scripted reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadAction {
    /// Return these bytes, retaining any remainder for the next read.
    Bytes(Vec<u8>),
    /// Return an I/O error once.
    Error(io::ErrorKind),
    /// Wait until the driver supplies another action or interrupts the session.
    Block,
    /// Return end-of-file.
    Eof,
}

impl ReadAction {
    /// Creates a byte-producing action.
    #[must_use]
    pub fn bytes(bytes: impl AsRef<[u8]>) -> Self {
        Self::Bytes(bytes.as_ref().to_vec())
    }

    /// Creates an error action.
    #[must_use]
    pub const fn error(kind: io::ErrorKind) -> Self {
        Self::Error(kind)
    }
}

/// One deterministic result in a scripted writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteAction {
    /// Accept at most this many bytes from one write call.
    Accept(usize),
    /// Return an I/O error once.
    Error(io::ErrorKind),
    /// Wait until the session is interrupted.
    Block,
}

impl WriteAction {
    /// Creates a partial or complete accept action.
    #[must_use]
    pub const fn accept(max_bytes: usize) -> Self {
        Self::Accept(max_bytes)
    }

    /// Creates an error action.
    #[must_use]
    pub const fn error(kind: io::ErrorKind) -> Self {
        Self::Error(kind)
    }
}

/// One deterministic result from [`SessionControl::poll_exit`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitAction {
    /// The session is still running.
    Pending,
    /// The session completed with this record.
    Exited(SessionExit),
    /// Exit status polling returns an I/O error once.
    Error(io::ErrorKind),
}

impl ExitAction {
    /// Creates an exit polling error action.
    #[must_use]
    pub const fn error(kind: io::ErrorKind) -> Self {
        Self::Error(kind)
    }
}

/// Calls observed on the scripted control plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlCall {
    /// A resize request.
    Resize(TerminalSize),
    /// An exit polling request.
    PollExit,
    /// An orderly-close request.
    BeginClose,
}

/// Calls observed on the scripted control plane.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ControlLog {
    /// All calls in their linearized order.
    pub calls: Vec<ControlCall>,
    /// Resize requests in call order.
    pub resizes: Vec<TerminalSize>,
    /// Number of exit polling calls.
    pub poll_exit_calls: u64,
    /// Number of orderly-close calls.
    pub begin_close_calls: u64,
}

#[derive(Debug)]
struct PendingBytes {
    bytes: Vec<u8>,
    offset: usize,
}

#[derive(Debug)]
enum PendingReadAction {
    Bytes(PendingBytes),
    Error(io::ErrorKind),
    Block,
    Eof,
}

impl From<ReadAction> for PendingReadAction {
    fn from(action: ReadAction) -> Self {
        match action {
            ReadAction::Bytes(bytes) => Self::Bytes(PendingBytes { bytes, offset: 0 }),
            ReadAction::Error(kind) => Self::Error(kind),
            ReadAction::Block => Self::Block,
            ReadAction::Eof => Self::Eof,
        }
    }
}

#[derive(Debug)]
struct State {
    reads: VecDeque<PendingReadAction>,
    writes: VecDeque<WriteAction>,
    exits: VecDeque<ExitAction>,
    resize_errors: VecDeque<io::ErrorKind>,
    close_errors: VecDeque<io::ErrorKind>,
    accepted_writes: Vec<u8>,
    control_log: ControlLog,
    interrupted: bool,
    interrupt_calls: u64,
    reader_blocked: bool,
    writer_blocked: bool,
}

#[derive(Debug)]
struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn wait<'a>(&self, state: MutexGuard<'a, State>) -> MutexGuard<'a, State> {
        self.changed
            .wait(state)
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// Deterministic transport consumed through the production session contract.
#[derive(Debug)]
pub struct ScriptedTransport {
    shared: Arc<Shared>,
}

/// Test-driver handle for observing and advancing a scripted session.
#[derive(Debug, Clone)]
pub struct ScriptedSessionDriver {
    shared: Arc<Shared>,
}

/// Scripted reader half.
#[derive(Debug)]
pub struct ScriptedReader {
    shared: Arc<Shared>,
}

/// Scripted writer half.
#[derive(Debug)]
pub struct ScriptedWriter {
    shared: Arc<Shared>,
}

/// Scripted control half.
#[derive(Debug)]
pub struct ScriptedControl {
    shared: Arc<Shared>,
}

/// Cloneable interrupt that releases both scripted blocking halves.
#[derive(Debug, Clone)]
pub struct ScriptedInterrupt {
    shared: Arc<Shared>,
}

impl ScriptedTransport {
    /// Creates a transport and its out-of-band deterministic test driver.
    #[must_use]
    pub fn new(
        reads: impl IntoIterator<Item = ReadAction>,
        writes: impl IntoIterator<Item = WriteAction>,
        exits: impl IntoIterator<Item = ExitAction>,
    ) -> (Self, ScriptedSessionDriver) {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                reads: reads.into_iter().map(PendingReadAction::from).collect(),
                writes: writes.into_iter().collect(),
                exits: exits.into_iter().collect(),
                resize_errors: VecDeque::new(),
                close_errors: VecDeque::new(),
                accepted_writes: Vec::new(),
                control_log: ControlLog::default(),
                interrupted: false,
                interrupt_calls: 0,
                reader_blocked: false,
                writer_blocked: false,
            }),
            changed: Condvar::new(),
        });
        (
            Self {
                shared: Arc::clone(&shared),
            },
            ScriptedSessionDriver { shared },
        )
    }
}

impl ScriptedSessionDriver {
    /// Waits until the reader has entered its scripted blocking action.
    pub fn wait_until_reader_blocked(&self) {
        let mut state = self.shared.lock();
        while !state.reader_blocked && !state.interrupted {
            state = self.shared.wait(state);
        }
    }

    /// Waits until the writer has entered its scripted blocking action.
    pub fn wait_until_writer_blocked(&self) {
        let mut state = self.shared.lock();
        while !state.writer_blocked && !state.interrupted {
            state = self.shared.wait(state);
        }
    }

    /// Waits until successful writes have accepted at least `length` bytes.
    pub fn wait_until_accepted_write_len(&self, length: usize) {
        let mut state = self.shared.lock();
        while state.accepted_writes.len() < length && !state.interrupted {
            state = self.shared.wait(state);
        }
    }

    /// Waits until the control plane has observed at least `count` calls.
    pub fn wait_until_control_call_count(&self, count: usize) {
        let mut state = self.shared.lock();
        while state.control_log.calls.len() < count && !state.interrupted {
            state = self.shared.wait(state);
        }
    }

    /// Supplies a reader action, replacing a blocking marker at the front.
    pub fn push_read(&self, action: ReadAction) {
        let mut state = self.shared.lock();
        if matches!(state.reads.front(), Some(PendingReadAction::Block)) {
            state.reads.pop_front();
        }
        state.reads.push_front(action.into());
        state.reader_blocked = false;
        drop(state);
        self.shared.changed.notify_all();
    }

    /// Supplies an ordered reader script, replacing a front blocking marker.
    pub fn push_reads(&self, actions: impl IntoIterator<Item = ReadAction>) {
        let actions = actions
            .into_iter()
            .map(PendingReadAction::from)
            .collect::<Vec<_>>();
        let mut state = self.shared.lock();
        if matches!(state.reads.front(), Some(PendingReadAction::Block)) {
            state.reads.pop_front();
        }
        for action in actions.into_iter().rev() {
            state.reads.push_front(action);
        }
        state.reader_blocked = false;
        drop(state);
        self.shared.changed.notify_all();
    }

    /// Supplies a writer action, replacing a blocking marker at the front.
    pub fn push_write(&self, action: WriteAction) {
        let mut state = self.shared.lock();
        if matches!(state.writes.front(), Some(WriteAction::Block)) {
            state.writes.pop_front();
        }
        state.writes.push_front(action);
        state.writer_blocked = false;
        drop(state);
        self.shared.changed.notify_all();
    }

    /// Queues a one-shot resize error.
    pub fn push_resize_error(&self, kind: io::ErrorKind) {
        self.shared.lock().resize_errors.push_back(kind);
    }

    /// Queues a one-shot orderly-close error.
    pub fn push_close_error(&self, kind: io::ErrorKind) {
        self.shared.lock().close_errors.push_back(kind);
    }

    /// Returns all bytes accepted by successful scripted writes.
    #[must_use]
    pub fn accepted_writes(&self) -> Vec<u8> {
        self.shared.lock().accepted_writes.clone()
    }

    /// Returns a snapshot of control calls.
    #[must_use]
    pub fn control_log(&self) -> ControlLog {
        self.shared.lock().control_log.clone()
    }

    /// Returns the number of interrupt calls.
    #[must_use]
    pub fn interrupt_calls(&self) -> u64 {
        self.shared.lock().interrupt_calls
    }
}

impl Read for ScriptedReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let mut state = self.shared.lock();
        loop {
            if state.interrupted {
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            }
            match state.reads.front_mut() {
                Some(PendingReadAction::Bytes(pending)) => {
                    let remaining = &pending.bytes[pending.offset..];
                    let count = remaining.len().min(buffer.len());
                    buffer[..count].copy_from_slice(&remaining[..count]);
                    pending.offset += count;
                    if pending.offset == pending.bytes.len() {
                        state.reads.pop_front();
                    }
                    return Ok(count);
                }
                Some(PendingReadAction::Error(kind)) => {
                    let kind = *kind;
                    state.reads.pop_front();
                    return Err(io::Error::from(kind));
                }
                Some(PendingReadAction::Eof) | None => return Ok(0),
                Some(PendingReadAction::Block) => {
                    state.reader_blocked = true;
                    self.shared.changed.notify_all();
                    state = self.shared.wait(state);
                }
            }
        }
    }
}

impl Write for ScriptedWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let mut state = self.shared.lock();
        loop {
            if state.interrupted {
                return Err(io::Error::from(io::ErrorKind::BrokenPipe));
            }
            match state.writes.front().copied() {
                Some(WriteAction::Accept(max_bytes)) => {
                    state.writes.pop_front();
                    let count = max_bytes.min(buffer.len());
                    state.accepted_writes.extend_from_slice(&buffer[..count]);
                    drop(state);
                    self.shared.changed.notify_all();
                    return Ok(count);
                }
                Some(WriteAction::Error(kind)) => {
                    state.writes.pop_front();
                    return Err(io::Error::from(kind));
                }
                Some(WriteAction::Block) => {
                    state.writer_blocked = true;
                    self.shared.changed.notify_all();
                    state = self.shared.wait(state);
                }
                None => return Err(io::Error::from(io::ErrorKind::WriteZero)),
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl SessionControl for ScriptedControl {
    fn resize(&mut self, size: TerminalSize) -> io::Result<()> {
        let mut state = self.shared.lock();
        state.control_log.calls.push(ControlCall::Resize(size));
        state.control_log.resizes.push(size);
        let result = match state.resize_errors.pop_front() {
            Some(kind) => Err(io::Error::from(kind)),
            None => Ok(()),
        };
        drop(state);
        self.shared.changed.notify_all();
        result
    }

    fn poll_exit(&mut self) -> io::Result<Option<SessionExit>> {
        let mut state = self.shared.lock();
        state.control_log.calls.push(ControlCall::PollExit);
        state.control_log.poll_exit_calls = state.control_log.poll_exit_calls.saturating_add(1);
        let result = match state.exits.pop_front().unwrap_or(ExitAction::Pending) {
            ExitAction::Pending => Ok(None),
            ExitAction::Exited(exit) => Ok(Some(exit)),
            ExitAction::Error(kind) => Err(io::Error::from(kind)),
        };
        drop(state);
        self.shared.changed.notify_all();
        result
    }

    fn begin_close(&mut self) -> io::Result<()> {
        let mut state = self.shared.lock();
        state.control_log.calls.push(ControlCall::BeginClose);
        state.control_log.begin_close_calls = state.control_log.begin_close_calls.saturating_add(1);
        let result = match state.close_errors.pop_front() {
            Some(kind) => Err(io::Error::from(kind)),
            None => Ok(()),
        };
        drop(state);
        self.shared.changed.notify_all();
        result
    }
}

impl SessionInterrupt for ScriptedInterrupt {
    fn interrupt(&self) -> io::Result<()> {
        let mut state = self.shared.lock();
        state.interrupt_calls = state.interrupt_calls.saturating_add(1);
        state.interrupted = true;
        drop(state);
        self.shared.changed.notify_all();
        Ok(())
    }
}

impl SessionTransport for ScriptedTransport {
    type Reader = ScriptedReader;
    type Writer = ScriptedWriter;
    type Control = ScriptedControl;
    type Interrupt = ScriptedInterrupt;

    fn split(self) -> SessionParts<Self::Reader, Self::Writer, Self::Control, Self::Interrupt> {
        SessionParts::new(
            ScriptedReader {
                shared: Arc::clone(&self.shared),
            },
            ScriptedWriter {
                shared: Arc::clone(&self.shared),
            },
            ScriptedControl {
                shared: Arc::clone(&self.shared),
            },
            ScriptedInterrupt {
                shared: self.shared,
            },
        )
    }
}
