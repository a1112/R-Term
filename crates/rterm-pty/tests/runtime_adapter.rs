use std::io::{Read, Write};
use std::time::{Duration, Instant};

#[cfg(windows)]
use rterm_domain::PaneId;
use rterm_pty::{LocalPtyTransport, PtyCommand};
#[cfg(windows)]
use rterm_runtime::{PaneNotice, PaneWorkerConfig, RuntimeEffectKind, RuntimeHub, SystemClock};
use rterm_runtime::{
    RuntimeBuffers, RuntimeEffectRef, SessionControl, SessionInterrupt, SessionTransport,
    TerminalRuntime,
};
use rterm_types::TerminalSize;

#[test]
fn local_adapter_spawns_reads_resizes_and_preserves_exit_status() {
    let transport = LocalPtyTransport::spawn(
        &PtyCommand::platform_identity_command(),
        TerminalSize::new(80, 24),
    )
    .expect("spawn local PTY transport");
    let mut parts = transport.split();
    parts
        .control
        .resize(TerminalSize::new(100, 31))
        .expect("resize local PTY");

    let mut reader = parts.reader;
    let mut writer = parts.writer;
    let reader_thread = std::thread::spawn(move || {
        let mut output = Vec::new();
        let mut runtime = TerminalRuntime::new(TerminalSize::new(100, 31));
        let mut buffers = RuntimeBuffers::default();
        let mut chunk = [0_u8; 8192];
        loop {
            let count = reader.read(&mut chunk).expect("read local PTY output");
            if count == 0 {
                break;
            }
            output.extend_from_slice(&chunk[..count]);
            let delta = runtime.feed_into(&chunk[..count], &mut buffers);
            for effect in delta.effects() {
                if let RuntimeEffectRef::TransportWrite(bytes) = effect {
                    writer.write_all(bytes).expect("write terminal response");
                }
            }
        }
        output
    });
    let exit_deadline = Instant::now() + Duration::from_secs(5);
    let exit = loop {
        if let Some(exit) = parts.control.poll_exit().expect("poll local exit") {
            break exit;
        }
        assert!(Instant::now() < exit_deadline, "local PTY did not exit");
        std::thread::yield_now();
    };
    parts.control.begin_close().expect("close local PTY");
    let output = reader_thread.join().expect("join local PTY reader");
    assert!(!String::from_utf8_lossy(&output).trim().is_empty());
    assert_eq!(exit.status, Some(0));
    assert!(exit.signal.is_none());
    parts.control.begin_close().expect("idempotent local close");
}

#[test]
fn local_adapter_preserves_spawn_error_context() {
    let error = LocalPtyTransport::spawn(&PtyCommand::new(""), TerminalSize::new(80, 24))
        .expect_err("empty command must fail");
    assert!(error.to_string().contains("command"));
}

#[test]
fn local_adapter_interrupt_is_idempotent_and_releases_a_live_process() {
    let command = if cfg!(windows) {
        PtyCommand::new("cmd.exe").with_args(["/D", "/C", "ping -n 30 127.0.0.1 >NUL"])
    } else {
        PtyCommand::new("/bin/sh").with_args(["-lc", "sleep 30"])
    };
    let transport = LocalPtyTransport::spawn(&command, TerminalSize::new(80, 24))
        .expect("spawn interruptible PTY");
    let mut parts = transport.split();
    parts.interrupt.interrupt().expect("interrupt PTY");
    parts.interrupt.interrupt().expect("repeat PTY interrupt");

    let deadline = Instant::now() + Duration::from_secs(5);
    while parts
        .control
        .poll_exit()
        .expect("poll interrupted PTY")
        .is_none()
    {
        assert!(Instant::now() < deadline, "interrupted PTY did not exit");
        std::thread::yield_now();
    }
    parts.control.begin_close().expect("close interrupted PTY");
}

#[cfg(windows)]
#[test]
fn local_powershell_tail_reaches_runtime_before_closed_notice() {
    let marker = "RTERM-LINK-BEGIN|runtime-tail|RTERM-LINK-END";
    let command = PtyCommand::new("powershell.exe")
        .with_args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$bytes=[Text.Encoding]::UTF8.GetBytes($env:RTERM_TEST_MARKER); \
             $stdout=[Console]::OpenStandardOutput(); \
             $stdout.Write($bytes,0,$bytes.Length)",
        ])
        .with_env("RTERM_TEST_MARKER", marker);
    let transport =
        LocalPtyTransport::spawn(&command, TerminalSize::new(80, 24)).expect("spawn marker PTY");
    let mut hub = RuntimeHub::new(SystemClock);
    let handle = hub
        .open(
            PaneId::new(41),
            transport,
            PaneWorkerConfig {
                capture_host_stream: true,
                ..PaneWorkerConfig::default()
            },
        )
        .expect("open local marker runtime");
    let token = handle.token();
    let mut output = Vec::new();

    loop {
        match hub.recv_notice().expect("runtime notice") {
            PaneNotice::Wake(pane) if pane == token => {
                let drain = hub.drain_pane(token, usize::MAX).expect("drain marker");
                output.extend(
                    drain
                        .effects
                        .iter()
                        .filter_map(|effect| match effect.effect.kind() {
                            RuntimeEffectKind::HostStream(bytes) => Some(bytes.as_slice()),
                            _ => None,
                        })
                        .flatten()
                        .copied(),
                );
            }
            PaneNotice::Closed { pane, .. } if pane == token => break,
            _ => {}
        }
    }

    assert!(
        String::from_utf8_lossy(&output).contains(marker),
        "runtime output omitted PowerShell tail: {output:?}"
    );
    hub.shutdown();
}
