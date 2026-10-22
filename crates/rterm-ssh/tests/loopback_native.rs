use std::{
    io::{Read as _, Write as _},
    net::{TcpListener, TcpStream},
    time::Duration,
};

#[cfg(target_os = "linux")]
use std::io;

use rterm_ssh::{
    LazyRusshRuntime, RusshChannelOpener, RusshConnectionCancellation, RusshDirectTcpIpOpenPlan,
    RusshForwardCancellation, RusshForwardDeadlines, RusshHostKeyPolicy,
    RusshRemoteTcpIpForwardPlan, SshChannel as _, SshChannelConnector, SshChannelOpener as _,
    SshConnectRequest, SshSessionConfig, SshSessionStartup, SshShellConnector as _,
    SshShellWriter as _,
};
use rterm_test_support::ssh::{
    CommandResponse, HermeticSshServer, IdentityFixture, LoopbackEchoServer, SshEvent,
};
use rterm_types::TerminalSize;

const DEADLINE: Duration = Duration::from_secs(5);

#[test]
fn two_native_sessions_reuse_one_injected_lazy_runtime() {
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let runtime = LazyRusshRuntime::new();
    assert!(!runtime.is_initialized());
    let handle = runtime
        .get_or_try_init()
        .expect("initialize shared runtime");
    let mut first = trusted_opener(&server).with_runtime_handle(handle.clone());
    let mut second = trusted_opener(&server).with_runtime_handle(handle);

    let mut first_channel = first
        .open_channel(key_request(&server, SshSessionStartup::NoShell))
        .expect("open first shared-runtime channel");
    first_channel
        .close_channel()
        .expect("close first shared-runtime channel");
    let mut second_channel = second
        .open_channel(key_request(&server, SshSessionStartup::NoShell))
        .expect("open second shared-runtime channel");
    second_channel
        .close_channel()
        .expect("close second shared-runtime channel");

    assert!(
        first
            .runtime_handle()
            .expect("first runtime")
            .shares_runtime_with(second.runtime_handle().expect("second runtime"))
    );
    server.stop(DEADLINE).expect("stop SSH fixture");
}

fn config(server: &HermeticSshServer, user: &str) -> SshSessionConfig {
    SshSessionConfig::new(
        "127.0.0.1",
        server.address().port(),
        user,
        TerminalSize::new(80, 24),
    )
}

fn trusted_opener(server: &HermeticSshServer) -> RusshChannelOpener {
    RusshChannelOpener::default().with_known_hosts_path(server.known_hosts_path())
}

fn key_request(server: &HermeticSshServer, startup: SshSessionStartup) -> SshConnectRequest {
    SshConnectRequest::private_key(
        config(server, "fixture-user"),
        server.agent().identity_path(),
        None::<String>,
    )
    .expect("build private-key request")
    .with_startup(startup)
}

fn read_channel(
    mut reader: Box<dyn rterm_ssh::SshShellReader>,
) -> (Vec<u8>, rterm_ssh::SshSessionResult) {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let count = reader.read(&mut buffer).expect("read SSH channel");
        if count == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..count]);
    }
    (output, reader.session_result())
}

#[test]
fn native_authentication_matrix_accepts_password_and_encrypted_ed25519_and_rejects_bad_password() {
    let identity = IdentityFixture::runtime_ed25519_encrypted("native-ed25519-passphrase")
        .expect("generate encrypted Ed25519 identity");
    let server = HermeticSshServer::builder()
        .password("password-user", "correct-password")
        .authorize_public_key(identity.public_key().clone())
        .start(DEADLINE)
        .expect("start SSH fixture");
    let mut opener = trusted_opener(&server);

    let password =
        SshConnectRequest::password(config(&server, "password-user"), "correct-password")
            .expect("build password request")
            .with_startup(SshSessionStartup::NoShell);
    let mut password_channel = opener
        .open_channel(password)
        .expect("password authentication");
    password_channel
        .close_channel()
        .expect("close password channel");

    let rejected = SshConnectRequest::password(config(&server, "password-user"), "wrong-password")
        .expect("build rejected password request")
        .with_startup(SshSessionStartup::NoShell);
    assert!(opener.open_channel(rejected).is_err());

    let key_request = SshConnectRequest::private_key(
        config(&server, "ed25519-user"),
        identity.identity_path(),
        identity.passphrase().map(str::to_owned),
    )
    .expect("build Ed25519 request")
    .with_startup(SshSessionStartup::NoShell);
    let mut key_channel = opener
        .open_channel(key_request)
        .expect("encrypted Ed25519 authentication");
    key_channel.close_channel().expect("close Ed25519 channel");

    server.stop(DEADLINE).expect("stop SSH fixture");
}

#[test]
fn native_host_key_matrix_enforces_reject_unknown_tofu_accept_unknown_and_changed_keys() {
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let unknown_hosts = server.temp_home().path().join("unknown-known-hosts");
    std::fs::write(&unknown_hosts, b"").expect("create empty known-hosts");
    let request = key_request(&server, SshSessionStartup::NoShell);

    let mut reject_unknown = RusshChannelOpener::default().with_known_hosts_path(&unknown_hosts);
    assert!(reject_unknown.open_channel(request.clone()).is_err());

    let mut tofu = RusshChannelOpener::default()
        .with_host_key_policy(RusshHostKeyPolicy::TrustOnFirstUse)
        .with_known_hosts_path(&unknown_hosts);
    let mut learned = tofu
        .open_channel(request.clone())
        .expect("TOFU first connection");
    learned.close_channel().expect("close TOFU channel");
    assert!(
        !std::fs::read(&unknown_hosts)
            .expect("read learned hosts")
            .is_empty()
    );

    let mut reject_known = RusshChannelOpener::default().with_known_hosts_path(&unknown_hosts);
    let mut known = reject_known
        .open_channel(request.clone())
        .expect("known host accepted");
    known.close_channel().expect("close known-host channel");

    let unrelated = HermeticSshServer::start(DEADLINE).expect("start unrelated fixture");
    std::fs::write(
        &unknown_hosts,
        format!(
            "[127.0.0.1]:{} {}\n",
            server.address().port(),
            unrelated
                .host_key()
                .to_openssh()
                .expect("encode unrelated host key")
        ),
    )
    .expect("replace known host with changed key");
    let mut changed = RusshChannelOpener::default().with_known_hosts_path(&unknown_hosts);
    assert!(changed.open_channel(request.clone()).is_err());

    let mut accept_unknown =
        RusshChannelOpener::default().with_host_key_policy(RusshHostKeyPolicy::AcceptUnknown);
    let mut accepted = accept_unknown
        .open_channel(request)
        .expect("accept unknown host key");
    accepted
        .close_channel()
        .expect("close accept-unknown channel");

    unrelated.stop(DEADLINE).expect("stop unrelated fixture");
    server.stop(DEADLINE).expect("stop SSH fixture");
}

#[test]
fn native_connect_auth_and_channel_open_share_one_total_operation_deadline() {
    use std::sync::mpsc;

    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind stalled SSH listener");
    let address = listener
        .local_addr()
        .expect("read stalled listener address");
    let (stop_tx, stop_rx) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let (_connection, _) = listener.accept().expect("accept stalled SSH client");
        let _ = stop_rx.recv_timeout(DEADLINE);
    });
    let identity = IdentityFixture::runtime_ed25519_encrypted("stalled-listener-key")
        .expect("create isolated client key");
    let stalled_request = SshConnectRequest::private_key(
        SshSessionConfig::new(
            "127.0.0.1",
            address.port(),
            "fixture-user",
            TerminalSize::new(80, 24),
        ),
        identity.identity_path(),
        identity.passphrase().map(str::to_owned),
    )
    .expect("build stalled request")
    .with_startup(SshSessionStartup::NoShell);
    let mut opener = RusshChannelOpener::default()
        .with_host_key_policy(RusshHostKeyPolicy::AcceptUnknown)
        .with_operation_timeout(Duration::from_millis(150));
    let started = std::time::Instant::now();
    let error = opener
        .open_channel(stalled_request)
        .err()
        .expect("stalled handshake must time out");
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(error.to_string().contains("deadline"));
    let _ = stop_tx.send(());
    worker.join().expect("join stalled listener");

    let auth_server = HermeticSshServer::builder()
        .authentication_delay(Duration::from_secs(2))
        .start(DEADLINE)
        .expect("start delayed-auth fixture");
    let mut auth_opener =
        trusted_opener(&auth_server).with_operation_timeout(Duration::from_millis(150));
    assert!(
        auth_opener
            .open_channel(key_request(&auth_server, SshSessionStartup::NoShell))
            .err()
            .expect("delayed auth must time out")
            .to_string()
            .contains("deadline")
    );
    auth_server.stop(DEADLINE).expect("stop auth fixture");

    let open_server = HermeticSshServer::builder()
        .channel_open_delay(Duration::from_secs(2))
        .start(DEADLINE)
        .expect("start delayed-channel fixture");
    let mut open_opener =
        trusted_opener(&open_server).with_operation_timeout(Duration::from_millis(150));
    assert!(
        open_opener
            .open_channel(key_request(&open_server, SshSessionStartup::NoShell))
            .err()
            .expect("delayed channel open must time out")
            .to_string()
            .contains("deadline")
    );
    open_server.stop(DEADLINE).expect("stop channel fixture");
}

#[test]
fn native_connector_cancels_a_stalled_ssh_handshake() {
    use std::sync::mpsc;

    const CANCEL_DEADLINE: Duration = Duration::from_secs(1);

    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind stalled SSH listener");
    let address = listener
        .local_addr()
        .expect("read stalled listener address");
    let (accepted_tx, accepted_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let server = std::thread::spawn(move || {
        let (connection, _) = listener.accept().expect("accept stalled SSH client");
        accepted_tx.send(()).expect("report accepted client");
        let _connection = connection;
        let _ = release_rx.recv_timeout(DEADLINE);
    });

    let request = SshConnectRequest::agent(SshSessionConfig::new(
        "127.0.0.1",
        address.port(),
        "fixture-user",
        TerminalSize::new(80, 24),
    ))
    .with_startup(SshSessionStartup::NoShell);
    let cancellation = RusshConnectionCancellation::new();
    let worker_cancellation = cancellation.clone();
    let (done_tx, done_rx) = mpsc::sync_channel(1);
    let connector_worker = std::thread::spawn(move || {
        let opener = RusshChannelOpener::default()
            .with_host_key_policy(RusshHostKeyPolicy::AcceptUnknown)
            .with_operation_timeout(DEADLINE)
            .with_connection_cancellation(worker_cancellation);
        let mut connector = SshChannelConnector::new(opener);
        let result = connector
            .connect(request)
            .map(|_| ())
            .map_err(|error| error.to_string());
        let _ = done_tx.send(result);
    });

    accepted_rx
        .recv_timeout(DEADLINE)
        .expect("SSH client must reach listener");
    let started = std::time::Instant::now();
    cancellation.cancel();
    let result = done_rx.recv_timeout(CANCEL_DEADLINE);
    let elapsed = started.elapsed();

    let _ = release_tx.send(());
    server.join().expect("join stalled server");
    connector_worker.join().expect("join connector worker");

    let error = result
        .expect("cancellation must unblock connector")
        .expect_err("cancelled connect must fail");
    assert!(elapsed < CANCEL_DEADLINE, "cancel took {elapsed:?}");
    assert!(error.contains("cancelled"), "unexpected error: {error}");
}

#[test]
fn native_channel_read_and_missing_eof_have_a_bounded_inactivity_deadline() {
    let server = HermeticSshServer::builder()
        .stalled_command("'native-stalled-output'")
        .start(DEADLINE)
        .expect("start stalled-output fixture");
    let probe = server.task_probe();
    let mut opener = trusted_opener(&server)
        .with_operation_timeout(DEADLINE)
        .with_channel_inactivity_timeout(Duration::from_millis(150));
    let request = key_request(
        &server,
        SshSessionStartup::command(["native-stalled-output".to_owned()])
            .expect("build stalled command"),
    );
    let channel = opener.open_channel(request).expect("open stalled channel");
    let (mut reader, _writer) = channel.into_read_writer();
    let started = std::time::Instant::now();
    let mut byte = [0_u8; 1];
    let error = rterm_ssh::SshShellReader::read(&mut reader, &mut byte)
        .expect_err("stalled read must time out");
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(error.to_string().contains("inactivity deadline"));
    server.stop(DEADLINE).expect("stop stalled fixture");
    assert_eq!(probe.active(), 0);
}

#[test]
fn default_channel_reader_allows_legitimate_silence_before_output() {
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let mut opener = trusted_opener(&server);
    assert_eq!(opener.channel_inactivity_timeout(), None);
    let channel = opener
        .open_channel(key_request(&server, SshSessionStartup::Shell))
        .expect("open shell channel");
    let (mut reader, mut writer) = channel.into_read_writer();
    let started = std::time::Instant::now();
    let mut output = Vec::new();

    std::thread::scope(|scope| {
        let write = scope.spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            rterm_ssh::SshShellWriter::write(&mut writer, b"echo after-legitimate-silence\n")
                .expect("write after silence");
            writer.finish_input().expect("finish delayed input");
            writer
        });
        std::io::Read::read_to_end(&mut reader, &mut output).expect("read delayed output");
        let _writer = write.join().expect("join delayed writer");
    });

    assert!(started.elapsed() >= Duration::from_millis(250));
    assert_eq!(output, b"after-legitimate-silence\n");
    server.stop(DEADLINE).expect("stop SSH fixture");
}

#[test]
fn native_exec_and_shell_preserve_pty_resize_output_and_exit_status() {
    let server = HermeticSshServer::builder()
        .command(
            "'native-status'",
            CommandResponse::status(b"native-stdout", b"", 42),
        )
        .start(DEADLINE)
        .expect("start SSH fixture");
    let mut opener = trusted_opener(&server);

    let exec = key_request(
        &server,
        SshSessionStartup::command(["native-status".to_owned()]).expect("command startup"),
    );
    let exec_channel = opener.open_channel(exec).expect("open exec channel");
    let (reader, _writer) = exec_channel.into_read_writer();
    let (output, result) = read_channel(Box::new(reader));
    assert_eq!(output, b"native-stdout");
    assert_eq!(result.exit_status, Some(42));

    let shell = key_request(&server, SshSessionStartup::Shell);
    let shell_channel = opener.open_channel(shell).expect("open shell channel");
    let (reader, mut writer) = shell_channel.into_read_writer();
    writer
        .resize(TerminalSize::new(132, 43))
        .expect("resize native PTY");
    rterm_ssh::SshShellWriter::write(&mut writer, b"echo native-shell\n")
        .expect("write shell input");
    writer.finish_input().expect("finish shell input");
    let (output, result) = read_channel(Box::new(reader));
    assert_eq!(output, b"native-shell\n");
    assert_eq!(result.exit_status, Some(0));

    let events = server.events();
    assert!(events.iter().any(|event| matches!(
        event,
        SshEvent::Pty {
            columns: 80,
            rows: 24,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        SshEvent::Resize {
            columns: 132,
            rows: 43,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(event, SshEvent::Shell)));
    server.stop(DEADLINE).expect("stop SSH fixture");
}

#[test]
fn native_direct_and_remote_forwarding_bridge_real_loopback_tcp() {
    let target = LoopbackEchoServer::start(DEADLINE).expect("start echo target");
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let request = key_request(&server, SshSessionStartup::NoShell);
    let mut opener = trusted_opener(&server);

    let plan =
        RusshDirectTcpIpOpenPlan::new("127.0.0.1", target.address().port(), "127.0.0.1", 45000);
    let mut direct = opener
        .open_direct_tcpip_channel(request.clone(), &plan)
        .expect("open direct forward");
    direct
        .write_channel(b"direct-native")
        .expect("write direct forward");
    let mut direct_echo = [0_u8; 13];
    let mut direct_read = 0;
    while direct_read < direct_echo.len() {
        direct_read += direct
            .read_channel(&mut direct_echo[direct_read..])
            .expect("read direct forward");
    }
    assert_eq!(&direct_echo, b"direct-native");
    direct.close_channel().expect("close direct forward");

    for round in 0..3 {
        let remote_plan =
            RusshRemoteTcpIpForwardPlan::new("127.0.0.1", 0, "127.0.0.1", target.address().port());
        let cancellation = RusshForwardCancellation::new();
        let mut remote = opener
            .open_remote_tcpip_forward_with_lifecycle(
                &request,
                &remote_plan,
                &cancellation,
                RusshForwardDeadlines::new(DEADLINE, DEADLINE),
            )
            .expect("open remote forward on server-assigned port");
        let remote_port = remote.bound_port();
        assert_ne!(remote_port, 0, "round {round} must report assigned port");
        let mut stream =
            TcpStream::connect(("127.0.0.1", remote_port)).expect("connect remote listener");
        stream
            .set_read_timeout(Some(DEADLINE))
            .expect("set read deadline");
        stream
            .write_all(b"remote-native")
            .expect("write remote forward");
        let mut remote_echo = [0_u8; 13];
        stream
            .read_exact(&mut remote_echo)
            .expect("read remote forward");
        assert_eq!(&remote_echo, b"remote-native");
        drop(stream);
        cancellation.cancel();
        remote.shutdown(DEADLINE).expect("stop remote forward");
    }

    server.stop(DEADLINE).expect("stop SSH fixture");
    target.stop(DEADLINE).expect("stop echo target");
}

#[test]
fn isolated_openssh_sshd_launcher_declares_hardened_effective_configuration_checks() {
    let script =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/ci/openssh-sshd.sh");
    let source = std::fs::read_to_string(script).expect("read OpenSSH sshd launcher");
    for required in [
        "PermitUserRC no",
        "DisableForwarding yes",
        "\"$SSHD\" -T -f",
        "-C \"user=$USER_NAME,host=localhost,addr=127.0.0.1\"",
        "grep -qx 'permituserrc no'",
        "grep -qx 'disableforwarding yes'",
        "STATE_DIR != *$'\\r'*",
        "STATE_DIR != *$'\\n'*",
        "quote_sshd_value",
    ] {
        assert!(
            source.contains(required),
            "missing script hardening {required:?}"
        );
    }
    assert_eq!(
        source
            .matches("[[ -n $STATE_DIR && $STATE_DIR != *$'\\r'* && $STATE_DIR != *$'\\n'* ]]")
            .count(),
        2,
        "state directory must be rejected both before and after canonicalization"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn native_client_interoperates_with_an_isolated_real_openssh_sshd() {
    use std::path::Path;

    use rterm_test_support::TempHome;

    require_linux_openssh_fixture_tools();

    let state = TempHome::new().expect("create isolated sshd state");
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/ci/openssh-sshd.sh");
    let (sshd, port, state_directory) = launch_isolated_openssh_sshd(&state, &script);

    let user = std::fs::read_to_string(state_directory.join("user"))
        .expect("read sshd user")
        .trim()
        .to_owned();
    let request = SshConnectRequest::private_key(
        SshSessionConfig::new("127.0.0.1", port, user, TerminalSize::new(80, 24)),
        state_directory.join("client_key"),
        None::<String>,
    )
    .expect("build real sshd request")
    .with_startup(
        SshSessionStartup::command(["printf".to_owned(), "native-openssh-sshd".to_owned()])
            .expect("build real sshd command"),
    );
    let mut opener =
        RusshChannelOpener::default().with_known_hosts_path(state_directory.join("known_hosts"));
    let channel = opener
        .open_channel(request)
        .expect("connect to real OpenSSH sshd");
    let (reader, _writer) = channel.into_read_writer();
    let (output, result) = read_channel(Box::new(reader));
    assert_eq!(output, b"native-openssh-sshd");
    assert_eq!(result.exit_status, Some(0));
    drop(sshd);
}

#[cfg(target_os = "linux")]
fn require_linux_openssh_fixture_tools() {
    use std::process::Command;

    use rterm_test_support::ChildGuard;

    for (tool, argument) in [("bash", "--version"), ("sshd", "-V"), ("ssh-keygen", "-?")] {
        let mut availability = Command::new(tool);
        availability.arg(argument);
        ChildGuard::spawn(availability, DEADLINE)
            .unwrap_or_else(|error| {
                panic!("required Linux OpenSSH fixture tool {tool} missing: {error}")
            })
            .wait()
            .unwrap_or_else(|error| panic!("required Linux tool {tool} probe failed: {error}"));
    }
}

#[cfg(target_os = "linux")]
fn launch_isolated_openssh_sshd(
    state: &rterm_test_support::TempHome,
    script: &std::path::Path,
) -> (rterm_test_support::ChildGuard, u16, std::path::PathBuf) {
    use std::{process::Command, thread, time::Instant};

    use rterm_test_support::ChildGuard;

    let mut launched = None;
    let mut forced_contention = Some(
        TcpListener::bind(("127.0.0.1", 0)).expect("occupy first isolated sshd candidate port"),
    );
    for attempt in 0..5 {
        let state_directory = state
            .path()
            .join(format!("state with spaces attempt {attempt}"));
        std::fs::create_dir(&state_directory).expect("create spaced sshd state path");
        let forced_attempt = attempt == 0 && forced_contention.is_some();
        let port = forced_contention.as_ref().map_or_else(
            || {
                TcpListener::bind(("127.0.0.1", 0))
                    .expect("select sshd candidate port")
                    .local_addr()
                    .expect("read sshd candidate port")
                    .port()
            },
            |listener| {
                listener
                    .local_addr()
                    .expect("read occupied sshd port")
                    .port()
            },
        );
        let mut command = Command::new("bash");
        command
            .arg(script)
            .arg(&state_directory)
            .arg(port.to_string());
        let mut sshd =
            ChildGuard::spawn(command, Duration::from_secs(20)).expect("launch isolated sshd");
        let started = Instant::now();
        loop {
            if let Some(output) = sshd.try_wait().expect("observe isolated sshd") {
                assert_retryable_sshd_collision(&output);
                break;
            }
            if forced_attempt {
                assert!(
                    started.elapsed() < DEADLINE,
                    "contended isolated sshd did not report bind failure"
                );
                thread::yield_now();
                continue;
            }
            match probe_ssh_banner(port) {
                Ok(()) => match sshd.try_wait().expect("recheck ready isolated sshd") {
                    None => {
                        launched = Some((sshd, port, state_directory));
                        break;
                    }
                    Some(output) => {
                        assert_retryable_sshd_collision(&output);
                        break;
                    }
                },
                Err(error) => {
                    if let Some(output) = sshd.try_wait().expect("recheck failed sshd readiness") {
                        assert_retryable_sshd_collision(&output);
                        break;
                    }
                    assert!(
                        started.elapsed() < DEADLINE,
                        "isolated sshd SSH-banner readiness deadline: {error}"
                    );
                }
            }
            thread::sleep(Duration::from_millis(20));
        }
        drop(forced_contention.take());
        if launched.is_some() {
            break;
        }
    }
    launched.expect("isolated sshd exhausted five address-in-use retries")
}

#[cfg(target_os = "linux")]
fn assert_retryable_sshd_collision(output: &rterm_test_support::ChildOutput) {
    let diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        diagnostics
            .to_ascii_lowercase()
            .contains("address already in use"),
        "isolated sshd exited for a non-retryable reason: {diagnostics:?}"
    );
}

#[cfg(target_os = "linux")]
fn probe_ssh_banner(port: u16) -> io::Result<()> {
    let mut stream = TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}")
            .parse()
            .expect("loopback socket address"),
        Duration::from_millis(250),
    )?;
    stream.set_read_timeout(Some(Duration::from_millis(250)))?;
    let mut prefix = [0_u8; 4];
    stream.read_exact(&mut prefix)?;
    if prefix == *b"SSH-" {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "readiness endpoint did not emit an SSH identification banner",
        ))
    }
}

#[test]
fn linux_real_openssh_probe_remains_required_after_extraction() {
    let native_ssh = include_str!("loopback_native.rs").replace("\r\n", "\n");
    for contract in [
        concat!(
            "#[cfg(target_os = \"linux\")]\n",
            "#[test]\n",
            "fn native_client_interoperates_with_an_isolated_real_openssh_sshd()"
        ),
        "(\"sshd\", \"-V\")",
        "required Linux OpenSSH fixture tool {tool} missing",
    ] {
        assert!(
            native_ssh.contains(contract),
            "required Linux real-sshd probe is missing {contract}"
        );
    }
    let function_start = native_ssh
        .find("fn native_client_interoperates_with_an_isolated_real_openssh_sshd()")
        .expect("required Linux real-sshd test function");
    let attribute_start = native_ssh[..function_start]
        .rfind("\n\n")
        .map_or(0, |offset| offset + 2);
    let attributes = &native_ssh[attribute_start..function_start];
    assert!(
        !attributes.lines().any(|line| {
            let attribute = line.trim_start();
            attribute.starts_with("#[") && attribute.contains("ignore")
        }),
        "required Linux real-sshd probe must not be ignored"
    );
}
