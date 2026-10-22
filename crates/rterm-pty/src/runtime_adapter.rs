use std::fmt;
use std::io::{self, Read, Write};
use std::time::Duration;

use crate::{PtyCommand, PtyExitStatus, PtyMasterClose, PtySession, PtySessionInterrupt, PtySize};
use rterm_types::TerminalSize;

use rterm_runtime::{
    SessionControl, SessionExit, SessionExitSignal, SessionInterrupt, SessionParts,
    SessionTransport,
};

/// Local PTY session split into runtime-owned reader, writer, and control resources.
pub struct LocalPtyTransport {
    reader: Box<dyn Read + Send>,
    writer: Box<dyn Write + Send>,
    control: LocalPtyControl,
    interrupt: LocalPtyInterrupt,
}

impl fmt::Debug for LocalPtyTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LocalPtyTransport(..)")
    }
}

impl LocalPtyTransport {
    /// Spawns a command in a local PTY and prepares independent runtime parts.
    ///
    /// # Errors
    ///
    /// Returns contextual PTY spawn, size, or stream-acquisition failures.
    pub fn spawn(command: &PtyCommand, size: TerminalSize) -> io::Result<Self> {
        let pty_size = PtySize::try_new(size.columns, size.rows).map_err(local_error("size"))?;
        let session = PtySession::spawn(command, pty_size).map_err(local_error("spawn"))?;
        Self::from_session(session)
    }

    /// Converts an established PTY session into independently owned runtime parts.
    ///
    /// # Errors
    ///
    /// Returns an error if either stream was already moved from the session.
    pub fn from_session(mut session: PtySession) -> io::Result<Self> {
        let interrupt = LocalPtyInterrupt(session.interrupt_handle());
        let reader = session.take_reader().map_err(local_error("reader"))?;
        let writer = session.take_writer().map_err(local_error("writer"))?;
        Ok(Self {
            reader,
            writer,
            control: LocalPtyControl {
                session,
                master_close: None,
                closed: false,
            },
            interrupt,
        })
    }
}

impl SessionTransport for LocalPtyTransport {
    type Reader = Box<dyn Read + Send>;
    type Writer = Box<dyn Write + Send>;
    type Control = LocalPtyControl;
    type Interrupt = LocalPtyInterrupt;

    fn split(self) -> SessionParts<Self::Reader, Self::Writer, Self::Control, Self::Interrupt> {
        SessionParts::new(self.reader, self.writer, self.control, self.interrupt)
    }
}

/// Local PTY lifecycle operations retained by the pane worker.
pub struct LocalPtyControl {
    session: PtySession,
    master_close: Option<PtyMasterClose>,
    closed: bool,
}

impl LocalPtyControl {
    /// Returns the operating-system process identifier when the backend exposes one.
    #[must_use]
    pub fn process_id(&self) -> Option<u32> {
        self.session.process_id()
    }

    /// Resizes using the PTY crate's validated size type.
    ///
    /// # Errors
    ///
    /// Returns the backend resize error with local-session context.
    pub fn resize_pty(&mut self, size: PtySize) -> io::Result<()> {
        self.session.resize(size).map_err(local_error("resize"))
    }

    /// Polls the native PTY status without projecting away backend details.
    ///
    /// # Errors
    ///
    /// Returns the backend wait error with local-session context.
    pub fn try_wait_pty(&mut self) -> io::Result<Option<PtyExitStatus>> {
        self.session.try_wait().map_err(local_error("exit status"))
    }

    /// Terminates the child within the caller's bounded shutdown window.
    ///
    /// # Errors
    ///
    /// Returns the backend termination error with local-session context.
    pub fn terminate(&mut self, timeout: Duration) -> io::Result<PtyExitStatus> {
        self.session
            .terminate(timeout)
            .map_err(local_error("child cleanup"))
    }

    /// Takes ownership of the PTY master-close operation after marking all
    /// writer proxies as closing.
    #[must_use]
    pub fn begin_master_close(&mut self) -> PtyMasterClose {
        self.closed = true;
        self.master_close
            .take()
            .unwrap_or_else(|| self.session.begin_master_close())
    }
}

impl SessionControl for LocalPtyControl {
    fn resize(&mut self, size: TerminalSize) -> io::Result<()> {
        let size = PtySize::try_new(size.columns, size.rows).map_err(local_error("resize size"))?;
        self.resize_pty(size)
    }

    fn poll_exit(&mut self) -> io::Result<Option<SessionExit>> {
        self.try_wait_pty()
            .map(|status| status.as_ref().map(local_exit))
    }

    fn begin_close(&mut self) -> io::Result<()> {
        if !self.closed {
            self.master_close = Some(self.session.begin_master_close());
            self.closed = true;
        }
        Ok(())
    }
}

/// Cloneable local process interrupt retained by the runtime hub.
#[derive(Debug, Clone)]
pub struct LocalPtyInterrupt(PtySessionInterrupt);

impl SessionInterrupt for LocalPtyInterrupt {
    fn interrupt(&self) -> io::Result<()> {
        self.0.interrupt()
    }
}

fn local_exit(status: &PtyExitStatus) -> SessionExit {
    SessionExit {
        status: Some(status.exit_code()),
        signal: status.signal().map(|name| SessionExitSignal {
            name: name.to_owned(),
            core_dumped: false,
            error_message: String::new(),
            lang_tag: String::new(),
        }),
    }
}

fn local_error(context: &'static str) -> impl FnOnce(crate::PtyError) -> io::Error {
    move |error| io::Error::other(format!("local PTY {context} failed: {error}"))
}
