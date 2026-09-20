use std::collections::VecDeque;
use std::sync::{
    Arc, Mutex, PoisonError,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

use rssh_domain::PaneId;
use rssh_ssh::{
    SshConnectRequest, SshExitSignal, SshSessionConfig, SshSessionError, SshSessionResult,
    SshShellConnector, SshShellReader, SshShellSession, SshShellWriter, SshTransport,
};
use rterm_runtime::{
    PaneNotice, PaneWorkerConfig, RuntimeEffectKind, RuntimeHub, SessionControl, SessionInterrupt,
    SessionTransport,
};
use rterm_types::TerminalSize;

#[derive(Debug, Default)]
struct State {
    writes: Vec<u8>,
    resizes: Vec<TerminalSize>,
    closes: usize,
    events: Vec<&'static str>,
}

struct Reader {
    chunks: VecDeque<Vec<u8>>,
    result: SshSessionResult,
}

impl SshShellReader for Reader {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, SshSessionError> {
        let Some(mut chunk) = self.chunks.pop_front() else {
            return Ok(0);
        };
        let count = chunk.len().min(buffer.len());
        buffer[..count].copy_from_slice(&chunk[..count]);
        if count < chunk.len() {
            chunk.drain(..count);
            self.chunks.push_front(chunk);
        }
        Ok(count)
    }

    fn session_result(&self) -> SshSessionResult {
        self.result.clone()
    }
}

struct Writer {
    state: Arc<Mutex<State>>,
}

impl SshShellWriter for Writer {
    fn write(&mut self, bytes: &[u8]) -> Result<usize, SshSessionError> {
        let count = bytes.len().min(2);
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.writes.extend_from_slice(&bytes[..count]);
        state.events.push("write");
        Ok(count)
    }

    fn resize(&mut self, size: rssh_ssh::SshTerminalSize) -> Result<(), SshSessionError> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.resizes.push(size.into());
        state.events.push("resize");
        Ok(())
    }

    fn keepalive(&mut self) -> Result<(), SshSessionError> {
        Ok(())
    }

    fn finish_input(&mut self) -> Result<(), SshSessionError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), SshSessionError> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.closes += 1;
        state.events.push("close");
        Ok(())
    }
}

struct Session {
    reader: Reader,
    writer: Writer,
}

impl SshShellSession for Session {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, SshSessionError> {
        self.reader.read(buffer)
    }

    fn write(&mut self, bytes: &[u8]) -> Result<usize, SshSessionError> {
        self.writer.write(bytes)
    }

    fn resize(&mut self, size: rssh_ssh::SshTerminalSize) -> Result<(), SshSessionError> {
        self.writer.resize(size)
    }

    fn keepalive(&mut self) -> Result<(), SshSessionError> {
        self.writer.keepalive()
    }

    fn close(&mut self) -> Result<(), SshSessionError> {
        self.writer.close()
    }

    fn into_read_writer(self: Box<Self>) -> (Box<dyn SshShellReader>, Box<dyn SshShellWriter>) {
        (Box::new(self.reader), Box::new(self.writer))
    }
}

struct Connector {
    session: Option<Box<dyn SshShellSession>>,
}

struct InterruptibleReader {
    entered: Arc<AtomicBool>,
}

impl SshShellReader for InterruptibleReader {
    fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, SshSessionError> {
        Err(SshSessionError::new(
            "legacy blocking read must not be used by the runtime adapter",
        ))
    }

    fn read_cancellable(
        &mut self,
        _buffer: &mut [u8],
        cancelled: &AtomicBool,
    ) -> Result<Option<usize>, SshSessionError> {
        self.entered.store(true, Ordering::Release);
        while !cancelled.load(Ordering::Acquire) {
            thread::yield_now();
        }
        Ok(None)
    }

    fn session_result(&self) -> SshSessionResult {
        SshSessionResult::default()
    }
}

struct InterruptibleSession {
    reader: InterruptibleReader,
    writer: Writer,
}

impl SshShellSession for InterruptibleSession {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, SshSessionError> {
        self.reader.read(buffer)
    }

    fn write(&mut self, bytes: &[u8]) -> Result<usize, SshSessionError> {
        self.writer.write(bytes)
    }

    fn resize(&mut self, size: rssh_ssh::SshTerminalSize) -> Result<(), SshSessionError> {
        self.writer.resize(size)
    }

    fn keepalive(&mut self) -> Result<(), SshSessionError> {
        self.writer.keepalive()
    }

    fn close(&mut self) -> Result<(), SshSessionError> {
        self.writer.close()
    }

    fn into_read_writer(self: Box<Self>) -> (Box<dyn SshShellReader>, Box<dyn SshShellWriter>) {
        (Box::new(self.reader), Box::new(self.writer))
    }
}

fn request() -> SshConnectRequest {
    SshConnectRequest::agent(SshSessionConfig::new(
        "loopback",
        22,
        "tester",
        TerminalSize::new(80, 24).into(),
    ))
}

impl SshShellConnector for Connector {
    fn connect(
        &mut self,
        _request: SshConnectRequest,
    ) -> Result<Box<dyn SshShellSession>, SshSessionError> {
        self.session
            .take()
            .ok_or_else(|| SshSessionError::new("injected connect failure"))
    }
}

#[test]
fn ssh_adapter_connects_preserves_partial_io_resize_exit_and_close_order() {
    let state = Arc::new(Mutex::new(State::default()));
    let session = Session {
        reader: Reader {
            chunks: VecDeque::from([b"remote-output".to_vec()]),
            result: SshSessionResult {
                exit_status: Some(u32::MAX),
                exit_signal: Some(SshExitSignal {
                    name: "TERM".to_owned(),
                    core_dumped: true,
                    error_message: "remote stopped".to_owned(),
                    lang_tag: "en-US".to_owned(),
                }),
            },
        },
        writer: Writer {
            state: Arc::clone(&state),
        },
    };
    let mut connector = Connector {
        session: Some(Box::new(session)),
    };
    let transport = SshTransport::connect(&mut connector, request()).expect("connect SSH adapter");
    let mut parts = transport.split();

    std::io::Write::write_all(&mut parts.writer, b"hello").expect("partial SSH writes");
    parts
        .control
        .resize(TerminalSize::new(132, 43))
        .expect("resize SSH PTY");
    let mut output = Vec::new();
    std::io::Read::read_to_end(&mut parts.reader, &mut output).expect("read SSH output");
    assert_eq!(output, b"remote-output");
    assert_eq!(
        parts.control.poll_exit().expect("poll SSH exit"),
        Some(rterm_runtime::SessionExit {
            status: Some(u32::MAX),
            signal: Some(rterm_runtime::SessionExitSignal {
                name: "TERM".to_owned(),
                core_dumped: true,
                error_message: "remote stopped".to_owned(),
                lang_tag: "en-US".to_owned(),
            }),
        })
    );
    parts.control.begin_close().expect("close SSH channel");
    parts.control.begin_close().expect("idempotent SSH close");

    let state = state.lock().unwrap_or_else(PoisonError::into_inner);
    assert_eq!(state.writes, b"hello");
    assert_eq!(state.resizes, [TerminalSize::new(132, 43)]);
    assert_eq!(state.closes, 1);
    assert_eq!(state.events, ["write", "write", "write", "resize", "close"]);
}

#[test]
fn ssh_adapter_preserves_connect_error_context() {
    let mut connector = Connector { session: None };
    let error = SshTransport::connect(&mut connector, request()).expect_err("connect must fail");
    assert_eq!(error.to_string(), "injected connect failure");
}

#[test]
fn ssh_adapter_interrupt_releases_a_blocked_reader() {
    let entered = Arc::new(AtomicBool::new(false));
    let state = Arc::new(Mutex::new(State::default()));
    let transport = SshTransport::from_session(Box::new(InterruptibleSession {
        reader: InterruptibleReader {
            entered: Arc::clone(&entered),
        },
        writer: Writer { state },
    }));
    let parts = transport.split();
    let interrupt = parts.interrupt.clone();
    let reader = thread::spawn(move || {
        let mut reader = parts.reader;
        let mut byte = [0_u8; 1];
        std::io::Read::read(&mut reader, &mut byte).map(|_| ())
    });

    let deadline = Instant::now() + Duration::from_secs(1);
    while !entered.load(Ordering::Acquire) && Instant::now() < deadline {
        thread::yield_now();
    }
    assert!(
        entered.load(Ordering::Acquire),
        "reader entered cancellable wait"
    );
    interrupt.interrupt().expect("interrupt blocked SSH read");
    interrupt.interrupt().expect("repeat SSH interrupt");

    let error = reader
        .join()
        .expect("SSH reader thread")
        .expect_err("cancelled read must return an error");
    assert_eq!(error.kind(), std::io::ErrorKind::Interrupted);
}

#[test]
fn ssh_adapter_reports_clean_disconnect_without_inventing_exit_metadata() {
    let state = Arc::new(Mutex::new(State::default()));
    let transport = SshTransport::from_session(Box::new(Session {
        reader: Reader {
            chunks: VecDeque::new(),
            result: SshSessionResult::default(),
        },
        writer: Writer { state },
    }));
    let mut parts = transport.split();
    let mut output = Vec::new();
    std::io::Read::read_to_end(&mut parts.reader, &mut output).expect("read clean disconnect");
    assert!(output.is_empty());
    assert_eq!(
        parts.control.poll_exit().expect("poll clean disconnect"),
        Some(rterm_runtime::SessionExit {
            status: None,
            signal: None,
        })
    );
}

#[test]
fn runtime_hub_reconnects_ssh_with_a_fresh_generation_and_no_stale_output() {
    let mut hub = RuntimeHub::new(rterm_runtime::SystemClock);
    let config = PaneWorkerConfig {
        capture_host_stream: true,
        ..PaneWorkerConfig::default()
    };
    let first = SshTransport::from_session(Box::new(Session {
        reader: Reader {
            chunks: VecDeque::from([b"first-ssh-generation".to_vec()]),
            result: SshSessionResult {
                exit_status: Some(0),
                exit_signal: None,
            },
        },
        writer: Writer {
            state: Arc::new(Mutex::new(State::default())),
        },
    }));
    let first = hub
        .open(PaneId::new(401), first, config)
        .expect("open first SSH generation");
    assert_eq!(
        drain_host_stream_until_closed(&mut hub, first.token()),
        b"first-ssh-generation"
    );

    let second_transport = SshTransport::from_session(Box::new(Session {
        reader: Reader {
            chunks: VecDeque::from([b"second-ssh-generation".to_vec()]),
            result: SshSessionResult {
                exit_status: Some(7),
                exit_signal: None,
            },
        },
        writer: Writer {
            state: Arc::new(Mutex::new(State::default())),
        },
    }));
    let second = hub
        .restart(PaneId::new(401), second_transport, config)
        .expect("reconnect SSH generation");
    assert_ne!(first.token().generation(), second.token().generation());
    assert_eq!(
        drain_host_stream_until_closed(&mut hub, second.token()),
        b"second-ssh-generation"
    );
    hub.shutdown();
    assert_eq!(hub.live_thread_count(), 0);
}

fn drain_host_stream_until_closed(
    hub: &mut RuntimeHub,
    token: rterm_runtime::PaneToken,
) -> Vec<u8> {
    let mut output = Vec::new();
    loop {
        match hub.recv_notice().expect("SSH generation notice") {
            PaneNotice::Ready(pane) if pane == token => {}
            PaneNotice::Wake(pane) if pane == token => {
                let drain = hub
                    .drain_pane(token, usize::MAX)
                    .expect("drain current SSH generation");
                for published in drain.effects {
                    if let RuntimeEffectKind::HostStream(bytes) = published.effect.kind() {
                        output.extend_from_slice(bytes);
                    }
                }
            }
            PaneNotice::Closed { pane, .. } if pane == token => break,
            PaneNotice::FirstPtyByte { pane, .. } if pane == token => {}
            notice => panic!("unexpected SSH generation notice: {notice:?}"),
        }
    }
    output
}
