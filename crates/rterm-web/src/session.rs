use std::{
    io::{self, Read, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use rterm_pty::{PtyCommand, PtyExitStatus, PtySession, PtySize};
use tokio::sync::mpsc as async_mpsc;

use crate::protocol::TerminalDimensions;

pub const INPUT_QUEUE_CAPACITY: usize = 64;
pub const CONTROL_QUEUE_CAPACITY: usize = 32;
pub const OUTPUT_QUEUE_CAPACITY: usize = 512;
pub const READ_CHUNK_BYTES: usize = 8192;
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(750);
pub const READER_DRAIN_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub enum SessionEvent {
    Output(Vec<u8>),
    Exit(PtyExitStatus),
    Error {
        code: &'static str,
        message: &'static str,
        fatal: bool,
    },
}

#[derive(Debug)]
enum SessionControl {
    Resize(PtySize),
    Close,
    WriterFailed,
    ReaderFailed,
}

pub struct WebPtySession {
    input_tx: mpsc::SyncSender<Vec<u8>>,
    control_tx: mpsc::SyncSender<SessionControl>,
    events: async_mpsc::Receiver<SessionEvent>,
    writer_stop: Arc<AtomicBool>,
    close_requested: Arc<AtomicBool>,
}

impl WebPtySession {
    /// Starts a bounded PTY worker group for the configured command and size.
    ///
    /// # Errors
    ///
    /// Returns the PTY error when the operating-system terminal cannot be
    /// created or its reader/writer cannot be acquired.
    pub fn spawn(
        command: &PtyCommand,
        dimensions: TerminalDimensions,
    ) -> Result<Self, rterm_pty::PtyError> {
        let size = PtySize::try_new(dimensions.cols, dimensions.rows)?;
        let mut pty = PtySession::spawn(command, size)?;
        let reader = match pty.take_reader() {
            Ok(reader) => reader,
            Err(error) => {
                drop(pty);
                return Err(error);
            }
        };
        let writer = match pty.take_writer() {
            Ok(writer) => writer,
            Err(error) => {
                drop(pty);
                return Err(error);
            }
        };

        let (input_tx, input_rx) = bounded_input_queue();
        let (control_tx, control_rx) = bounded_control_queue();
        let (events_tx, events) = async_mpsc::channel(OUTPUT_QUEUE_CAPACITY);
        let (reader_done_tx, reader_done_rx) = mpsc::sync_channel(1);
        let writer_stop = Arc::new(AtomicBool::new(false));
        let close_requested = Arc::new(AtomicBool::new(false));

        spawn_reader(
            reader,
            events_tx.clone(),
            control_tx.clone(),
            reader_done_tx,
            Arc::clone(&close_requested),
        );
        spawn_writer(
            writer,
            input_rx,
            events_tx.clone(),
            control_tx.clone(),
            Arc::clone(&writer_stop),
            Arc::clone(&close_requested),
        );
        spawn_supervisor(
            pty,
            control_rx,
            events_tx,
            reader_done_rx,
            Arc::clone(&writer_stop),
            Arc::clone(&close_requested),
        );

        Ok(Self {
            input_tx,
            control_tx,
            events,
            writer_stop,
            close_requested,
        })
    }

    /// Enqueues terminal input without waiting for a slow writer.
    ///
    /// # Errors
    ///
    /// Returns `Full` when the bounded input queue is saturated or `Closed`
    /// after the session has begun shutting down.
    pub fn try_send_input(&self, bytes: Vec<u8>) -> Result<(), InputSendError> {
        try_enqueue_input(&self.input_tx, bytes)
    }

    /// Enqueues a validated terminal resize without blocking the WebSocket task.
    ///
    /// # Errors
    ///
    /// Returns `Invalid`, `Full`, or `Closed` when the resize cannot be queued.
    pub fn try_resize(&self, dimensions: TerminalDimensions) -> Result<(), ResizeSendError> {
        try_enqueue_resize(&self.control_tx, dimensions)
    }

    pub fn request_close(&self) {
        self.close_requested.store(true, Ordering::Release);
        self.writer_stop.store(true, Ordering::Release);
        let _ = self.control_tx.try_send(SessionControl::Close);
    }

    pub fn events(&mut self) -> &mut async_mpsc::Receiver<SessionEvent> {
        &mut self.events
    }
}

fn bounded_input_queue() -> (mpsc::SyncSender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) {
    mpsc::sync_channel(INPUT_QUEUE_CAPACITY)
}

fn bounded_control_queue() -> (
    mpsc::SyncSender<SessionControl>,
    mpsc::Receiver<SessionControl>,
) {
    mpsc::sync_channel(CONTROL_QUEUE_CAPACITY)
}

fn try_enqueue_input(
    sender: &mpsc::SyncSender<Vec<u8>>,
    bytes: Vec<u8>,
) -> Result<(), InputSendError> {
    sender.try_send(bytes).map_err(|error| match error {
        mpsc::TrySendError::Full(_) => InputSendError::Full,
        mpsc::TrySendError::Disconnected(_) => InputSendError::Closed,
    })
}

fn try_enqueue_resize(
    sender: &mpsc::SyncSender<SessionControl>,
    dimensions: TerminalDimensions,
) -> Result<(), ResizeSendError> {
    let size =
        PtySize::try_new(dimensions.cols, dimensions.rows).map_err(|_| ResizeSendError::Invalid)?;
    sender
        .try_send(SessionControl::Resize(size))
        .map_err(|error| match error {
            mpsc::TrySendError::Full(_) => ResizeSendError::Full,
            mpsc::TrySendError::Disconnected(_) => ResizeSendError::Closed,
        })
}

impl Drop for WebPtySession {
    fn drop(&mut self) {
        self.request_close();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSendError {
    Full,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeSendError {
    Invalid,
    Full,
    Closed,
}

fn spawn_reader(
    mut reader: Box<dyn Read + Send>,
    events: async_mpsc::Sender<SessionEvent>,
    control: mpsc::SyncSender<SessionControl>,
    done: mpsc::SyncSender<()>,
    close_requested: Arc<AtomicBool>,
) {
    thread::Builder::new()
        .name("rterm-web-pty-reader".to_owned())
        .spawn(move || {
            let mut buffer = [0_u8; READ_CHUNK_BYTES];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        if events
                            .blocking_send(SessionEvent::Output(buffer[..count].to_vec()))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(_) => {
                        close_requested.store(true, Ordering::Release);
                        let _ = events.blocking_send(SessionEvent::Error {
                            code: "PTY_READ_FAILED",
                            message: "terminal output could not be read",
                            fatal: true,
                        });
                        let _ = control.try_send(SessionControl::ReaderFailed);
                        break;
                    }
                }
            }
            let _ = done.send(());
        })
        .expect("PTY reader worker must spawn");
}

fn spawn_writer(
    mut writer: Box<dyn Write + Send>,
    input: mpsc::Receiver<Vec<u8>>,
    events: async_mpsc::Sender<SessionEvent>,
    control: mpsc::SyncSender<SessionControl>,
    stop: Arc<AtomicBool>,
    close_requested: Arc<AtomicBool>,
) {
    thread::Builder::new()
        .name("rterm-web-pty-writer".to_owned())
        .spawn(move || {
            loop {
                if stop.load(Ordering::Acquire) {
                    break;
                }
                match input.recv_timeout(Duration::from_millis(50)) {
                    Ok(bytes) => {
                        if let Err(error) = writer.write_all(&bytes).and_then(|()| writer.flush()) {
                            close_requested.store(true, Ordering::Release);
                            stop.store(true, Ordering::Release);
                            let _ = events.blocking_send(SessionEvent::Error {
                                code: "PTY_WRITE_FAILED",
                                message: "terminal input could not be written",
                                fatal: true,
                            });
                            let _ = control.try_send(SessionControl::WriterFailed);
                            let _ = error;
                            break;
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        })
        .expect("PTY writer worker must spawn");
}

fn spawn_supervisor(
    mut pty: PtySession,
    control: mpsc::Receiver<SessionControl>,
    events: async_mpsc::Sender<SessionEvent>,
    reader_done: mpsc::Receiver<()>,
    writer_stop: Arc<AtomicBool>,
    close_requested: Arc<AtomicBool>,
) {
    thread::Builder::new()
        .name("rterm-web-pty-supervisor".to_owned())
        .spawn(move || {
            let exit_status = loop {
                if close_requested.load(Ordering::Acquire) {
                    writer_stop.store(true, Ordering::Release);
                    break pty.terminate(SHUTDOWN_TIMEOUT).ok();
                }
                match pty.try_wait() {
                    Ok(Some(status)) => {
                        break Some(status);
                    }
                    Ok(None) => {}
                    Err(_) => {
                        let _ = events.blocking_send(SessionEvent::Error {
                            code: "PTY_STATUS_FAILED",
                            message: "terminal process status could not be read",
                            fatal: true,
                        });
                        break pty.terminate(SHUTDOWN_TIMEOUT).ok();
                    }
                }

                match control.recv_timeout(Duration::from_millis(10)) {
                    Ok(SessionControl::Resize(size)) => {
                        if pty.resize(size).is_err() {
                            let _ = events.blocking_send(SessionEvent::Error {
                                code: "PTY_RESIZE_FAILED",
                                message: "terminal size could not be changed",
                                fatal: false,
                            });
                        }
                    }
                    Ok(
                        SessionControl::Close
                        | SessionControl::WriterFailed
                        | SessionControl::ReaderFailed,
                    )
                    | Err(mpsc::RecvTimeoutError::Disconnected) => {
                        writer_stop.store(true, Ordering::Release);
                        break pty.terminate(SHUTDOWN_TIMEOUT).ok();
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
            };

            writer_stop.store(true, Ordering::Release);
            let _master_close = pty.begin_master_close();
            if reader_done.recv_timeout(READER_DRAIN_TIMEOUT).is_err() {
                let _ = events.blocking_send(SessionEvent::Error {
                    code: "PTY_DRAIN_TIMEOUT",
                    message: "terminal output did not finish draining",
                    fatal: true,
                });
                return;
            }
            if let Some(status) = exit_status {
                let _ = events.blocking_send(SessionEvent::Exit(status));
            } else {
                let _ = events.blocking_send(SessionEvent::Error {
                    code: "PTY_EXIT_UNKNOWN",
                    message: "terminal process exit status was unavailable",
                    fatal: true,
                });
            }
        })
        .expect("PTY supervisor worker must spawn");
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        CONTROL_QUEUE_CAPACITY, INPUT_QUEUE_CAPACITY, InputSendError, ResizeSendError,
        SessionEvent, WebPtySession, bounded_control_queue, bounded_input_queue, try_enqueue_input,
        try_enqueue_resize,
    };
    use crate::protocol::TerminalDimensions;

    #[test]
    fn local_echo_session_round_trips_output() {
        let command = if cfg!(windows) {
            rterm_pty::PtyCommand::new("cmd.exe").with_args(["/C", "echo", "web-pty-test"])
        } else {
            rterm_pty::PtyCommand::new("/bin/sh").with_args(["-c", "printf web-pty-test"])
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut session =
                WebPtySession::spawn(&command, TerminalDimensions { cols: 80, rows: 24 }).unwrap();
            let mut output = Vec::new();
            let mut answered_cursor_query = false;
            let deadline = tokio::time::sleep(Duration::from_secs(3));
            tokio::pin!(deadline);
            loop {
                tokio::select! {
                    event = session.events().recv() => match event {
                        Some(SessionEvent::Output(bytes)) => {
                            output.extend(bytes);
                            if cfg!(windows)
                                && !answered_cursor_query
                                && output.windows(4).any(|window| window == b"\x1b[6n")
                            {
                                session.try_send_input(b"\x1b[1;1R".to_vec()).unwrap();
                                answered_cursor_query = true;
                            }
                        }
                        Some(SessionEvent::Exit(status)) => {
                            assert!(status.success());
                            break;
                        }
                        Some(SessionEvent::Error { code, message, .. }) => panic!("{code}: {message}"),
                        None => panic!("session events closed before exit"),
                    },
                    () = &mut deadline => panic!("PTY session did not exit"),
                }
            }
            assert!(String::from_utf8_lossy(&output).contains("web-pty-test"));
        });
    }

    #[test]
    fn bounded_input_queue_reports_backpressure_and_cleans_up() {
        let (sender, receiver) = bounded_input_queue();
        for index in 0..INPUT_QUEUE_CAPACITY {
            sender
                .try_send(vec![u8::try_from(index).unwrap_or(u8::MAX)])
                .expect("bounded input slot");
        }
        let full = try_enqueue_input(&sender, vec![0]);
        assert_eq!(full, Err(InputSendError::Full));

        drop(receiver);
        let closed = try_enqueue_input(&sender, vec![0]);
        assert_eq!(closed, Err(InputSendError::Closed));

        let (control, control_receiver) = bounded_control_queue();
        let dimensions = TerminalDimensions { cols: 90, rows: 30 };
        for _ in 0..CONTROL_QUEUE_CAPACITY {
            try_enqueue_resize(&control, dimensions).expect("bounded resize slot");
        }
        assert_eq!(
            try_enqueue_resize(&control, dimensions),
            Err(ResizeSendError::Full)
        );
        drop(control_receiver);
        assert_eq!(
            try_enqueue_resize(&control, dimensions),
            Err(ResizeSendError::Closed)
        );
    }
}
