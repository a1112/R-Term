use std::io::{self, Read, Write};

use rterm_types::TerminalSize;

/// Signal metadata reported when a terminal session exits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionExitSignal {
    /// Transport-provided signal name.
    pub name: String,
    /// Whether the session reported producing a core dump.
    pub core_dumped: bool,
    /// Transport-provided diagnostic message, including an empty message.
    pub error_message: String,
    /// Language tag associated with the diagnostic message.
    pub lang_tag: String,
}

/// Platform-neutral terminal-session completion record.
///
/// A transport can report both status and signal metadata. A record with both
/// fields absent is still a completed session; [`Option::None`] from
/// [`SessionControl::poll_exit`] alone means the session remains pending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionExit {
    /// Full unsigned status reported by a local or remote transport.
    pub status: Option<u32>,
    /// Signal metadata reported alongside or instead of a status.
    pub signal: Option<SessionExitSignal>,
}

/// A cloneable out-of-band handle that interrupts blocked session I/O.
///
/// Implementations must be safe to call concurrently, fast, and idempotent.
/// After a successful call, blocked reader and writer operations must make
/// progress toward returning an error or end-of-file without requiring the
/// session worker to regain control first.
pub trait SessionInterrupt: Clone + Send + Sync + 'static {
    /// Interrupts blocked reads and writes for the associated session.
    ///
    /// # Errors
    ///
    /// Returns a transport-specific I/O error when the interrupt request
    /// cannot be delivered.
    fn interrupt(&self) -> io::Result<()>;
}

/// Independently owned read, write, lifecycle, and interrupt resources.
#[derive(Debug)]
pub struct SessionParts<R, W, C, I> {
    /// Blocking or asynchronous-compatible byte reader.
    pub reader: R,
    /// Ordered byte writer.
    pub writer: W,
    /// Resize, exit, and close control plane.
    pub control: C,
    /// Out-of-band handle that can release blocked reader and writer calls.
    pub interrupt: I,
}

impl<R, W, C, I> SessionParts<R, W, C, I> {
    /// Creates session parts from transport-owned resources.
    #[must_use]
    pub fn new(reader: R, writer: W, control: C, interrupt: I) -> Self {
        Self {
            reader,
            writer,
            control,
            interrupt,
        }
    }
}

/// Lifecycle operations retained after a session transport is split.
pub trait SessionControl: Send + 'static {
    /// Resizes the remote terminal grid.
    ///
    /// # Errors
    ///
    /// Returns a transport-specific I/O error when the resize cannot be sent.
    fn resize(&mut self, size: TerminalSize) -> io::Result<()>;

    /// Polls for a terminal session exit without imposing a wait strategy.
    ///
    /// # Errors
    ///
    /// Returns a transport-specific I/O error when status cannot be queried.
    fn poll_exit(&mut self) -> io::Result<Option<SessionExit>>;

    /// Begins the transport's orderly close operation.
    ///
    /// Calling this method more than once should be idempotent for concrete
    /// transports.
    ///
    /// # Errors
    ///
    /// Returns a transport-specific I/O error when closing cannot begin.
    fn begin_close(&mut self) -> io::Result<()>;
}

/// A source of terminal bytes with an independent ordered writer and control plane.
pub trait SessionTransport: Send + 'static {
    /// Reader owned by the runtime's transport-read path.
    type Reader: Read + Send + 'static;
    /// Writer owned by the pane worker's ordered write path.
    type Writer: Write + Send + 'static;
    /// Lifecycle control retained by the pane worker.
    type Control: SessionControl + Send + 'static;
    /// Out-of-band interrupt retained by the runtime hub or shutdown path.
    type Interrupt: SessionInterrupt;

    /// Transfers each independent session resource to the runtime.
    #[must_use]
    fn split(self) -> SessionParts<Self::Reader, Self::Writer, Self::Control, Self::Interrupt>;
}
