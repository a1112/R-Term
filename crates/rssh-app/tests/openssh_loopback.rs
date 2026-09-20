use std::{
    env,
    io::{self, Read as _, Write as _},
    net::{TcpListener, TcpStream},
    process::Command,
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use rssh_test_support::{
    ChildGuard, ChildOutput, OpenSshClientTool, probe_openssh_tools_from_environment,
    ssh::{
        AgentFixture, CommandResponse, HermeticSshServer, LoopbackEchoProbe, LoopbackEchoServer,
        OpenSshTool,
    },
};

const DEADLINE: Duration = Duration::from_secs(5);
const PROCESS_DEADLINE: Duration = Duration::from_secs(15);

fn packaged_or_cargo_app_executable() -> std::path::PathBuf {
    env::var_os("RSSH_TEST_APP_EXECUTABLE").map_or_else(
        || std::path::PathBuf::from(env!("CARGO_BIN_EXE_rterm")),
        std::path::PathBuf::from,
    )
}

#[test]
fn required_openssh_probe_rejects_an_empty_path_with_a_nonzero_child_exit() {
    let mut command = Command::new(std::env::current_exe().expect("current test executable"));
    command
        .args(["--exact", "required_openssh_probe_child", "--nocapture"])
        .env("RSSH_OPENSSH_PROBE_CHILD", "1")
        .env("RSSH_REQUIRE_OPENSSH", "1")
        .env("PATH", "");
    let output = run(command);
    assert!(!output.status.success());
    let diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        diagnostics.contains("required OpenSSH tool"),
        "unexpected child diagnostics: {diagnostics:?}"
    );
}

#[test]
fn required_openssh_probe_child() {
    if std::env::var_os("RSSH_OPENSSH_PROBE_CHILD").is_none() {
        let cache = OnceLock::new();
        let probes = AtomicUsize::new(0);
        thread::scope(|scope| {
            for _ in 0..8 {
                let cache = &cache;
                let probes = &probes;
                scope.spawn(move || {
                    assert_eq!(
                        cached_openssh_availability(cache, || {
                            probes.fetch_add(1, Ordering::SeqCst);
                            Ok(true)
                        }),
                        Ok(true)
                    );
                });
            }
        });
        assert_eq!(probes.load(Ordering::SeqCst), 1);
        return;
    }
    probe_openssh_tools_from_environment(&[OpenSshClientTool::Ssh])
        .expect("required OpenSSH tool must be present");
}

fn openssh_available() -> bool {
    static OPENSSH_AVAILABILITY: OnceLock<Result<bool, String>> = OnceLock::new();
    cached_openssh_availability(&OPENSSH_AVAILABILITY, || {
        probe_openssh_tools_from_environment(&[OpenSshClientTool::Ssh])
    })
    .expect("required OpenSSH ssh probe")
}

fn cached_openssh_availability(
    cache: &OnceLock<Result<bool, String>>,
    probe: impl FnOnce() -> Result<bool, String>,
) -> Result<bool, String> {
    cache.get_or_init(probe).clone()
}

fn run(command: Command) -> ChildOutput {
    ChildGuard::spawn(command, PROCESS_DEADLINE)
        .expect("spawn deadline-bound OpenSSH process")
        .wait()
        .expect("wait for deadline-bound OpenSSH process")
}

fn ssh_command(server: &HermeticSshServer) -> Command {
    let mut command = Command::new("ssh");
    server.configure_openssh_command(&mut command, OpenSshTool::Ssh);
    command
}

#[cfg(windows)]
fn prepare_identity_for_openssh(server: &HermeticSshServer) {
    prepare_private_key_for_openssh(server.agent().identity_path());
}

#[cfg(windows)]
fn prepare_private_key_for_openssh(path: &std::path::Path) {
    let principal = std::env::var("USERNAME").expect("Windows username");
    let mut command = Command::new("icacls.exe");
    command
        .arg(path)
        .arg("/inheritance:r")
        .arg("/grant:r")
        .arg(format!("{principal}:(R)"));
    let output = run(command);
    assert!(
        output.status.success(),
        "icacls failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(not(windows))]
fn prepare_identity_for_openssh(_server: &HermeticSshServer) {}

#[cfg(not(windows))]
fn prepare_private_key_for_openssh(_path: &std::path::Path) {}

fn target(command: &mut Command) {
    command.arg("fixture-user@127.0.0.1");
}

#[test]
fn system_openssh_enforces_fixture_auth_host_key_and_remote_exit_status() {
    if !openssh_available() {
        return;
    }
    let server = HermeticSshServer::builder()
        .command(
            "openssh-status",
            CommandResponse::status(b"openssh-stdout", b"openssh-stderr", 23),
        )
        .start(DEADLINE)
        .expect("start SSH fixture");
    prepare_identity_for_openssh(&server);
    let mut command = ssh_command(&server);
    target(&mut command);
    command.arg("openssh-status");
    let output = run(command);
    assert_eq!(
        output.status.code(),
        Some(23),
        "stdout={:?}; stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"openssh-stdout");
    assert_eq!(output.stderr, b"openssh-stderr");

    let unrelated = HermeticSshServer::start(DEADLINE).expect("start unrelated fixture");
    std::fs::write(
        server.known_hosts_path(),
        format!(
            "[127.0.0.1]:{} {}\n",
            server.address().port(),
            unrelated
                .host_key()
                .to_openssh()
                .expect("encode changed host key")
        ),
    )
    .expect("install changed host key");
    let mut rejected = ssh_command(&server);
    target(&mut rejected);
    rejected.arg("openssh-status");
    let rejected = run(rejected);
    assert!(!rejected.status.success());
    let diagnostics = String::from_utf8_lossy(&rejected.stderr);
    assert!(
        diagnostics.contains("REMOTE HOST IDENTIFICATION HAS CHANGED")
            || diagnostics.contains("Host key verification failed"),
        "unexpected host-key diagnostics: {diagnostics}"
    );

    unrelated.stop(DEADLINE).expect("stop unrelated fixture");
    server.stop(DEADLINE).expect("stop SSH fixture");
}

#[test]
fn rssh_app_system_openssh_entrypoint_runs_a_real_loopback_exec() {
    if !openssh_available() {
        return;
    }
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    prepare_identity_for_openssh(&server);
    let mut command = Command::new(packaged_or_cargo_app_executable());
    server.temp_home().apply_to(&mut command);
    command
        .env_remove("SSH_AUTH_SOCK")
        .env_remove("SSH_ASKPASS")
        .env_remove("SSH_ASKPASS_REQUIRE")
        .env_remove("DISPLAY")
        .args([
            "ssh",
            "-F",
            server
                .isolated_ssh_config_path()
                .to_str()
                .expect("UTF-8 config path"),
            "-p",
            &server.address().port().to_string(),
            "-i",
            server
                .agent()
                .identity_path()
                .to_str()
                .expect("UTF-8 identity path"),
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            &format!("UserKnownHostsFile={}", server.known_hosts_path().display()),
            "-o",
            "GlobalKnownHostsFile=none",
            "-o",
            "IdentityAgent=none",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "BatchMode=yes",
            "fixture-user@127.0.0.1",
            "rssh-test-marker",
            "app-openssh-entry",
        ]);
    let output = run(command);
    assert!(
        output.status.success(),
        "stderr={:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("app-openssh-entry"));
    server.stop(DEADLINE).expect("stop SSH fixture");
}

#[test]
fn rssh_app_native_ssh_disconnects_and_reconnects_with_closed_lifecycle() {
    if !openssh_available() {
        return;
    }
    let server = HermeticSshServer::builder()
        .command(
            "'rssh-test-marker' 'native-reconnect-first-()（）!@#$%^&*，。！？'",
            CommandResponse::status(
                "native-reconnect-first-()（）!@#$%^&*，。！？".as_bytes(),
                b"",
                0,
            ),
        )
        .command(
            "'rssh-test-marker' 'native-reconnect-second-()（）!@#$%^&*，。！？'",
            CommandResponse::status(
                "native-reconnect-second-()（）!@#$%^&*，。！？".as_bytes(),
                b"",
                0,
            ),
        )
        .start(DEADLINE)
        .expect("start SSH reconnect fixture");
    prepare_identity_for_openssh(&server);

    for marker in [
        "native-reconnect-first-()（）!@#$%^&*，。！？",
        "native-reconnect-second-()（）!@#$%^&*，。！？",
    ] {
        let mut command = Command::new(packaged_or_cargo_app_executable());
        server.temp_home().apply_to(&mut command);
        command
            .env_remove("SSH_AUTH_SOCK")
            .env_remove("SSH_ASKPASS")
            .env_remove("SSH_ASKPASS_REQUIRE")
            .env_remove("DISPLAY")
            .args([
                "ssh",
                "--native",
                "--accept-unknown-host-key",
                "--metrics-json",
                "-p",
                &server.address().port().to_string(),
                "-i",
                server
                    .agent()
                    .identity_path()
                    .to_str()
                    .expect("UTF-8 identity path"),
                "fixture-user@127.0.0.1",
                "rssh-test-marker",
                marker,
            ]);
        let output = run(command);
        assert!(
            output.status.success(),
            "native reconnect {marker} failed with {:?}; stdout={}; stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains(marker), "missing {marker}: {stdout}");
        assert!(
            stdout.contains(r#""session_state":"closed""#),
            "native reconnect did not close its lifecycle: {stdout}"
        );
    }

    server.stop(DEADLINE).expect("stop SSH reconnect fixture");
}

#[test]
fn rssh_app_required_matrix_enforces_auth_host_key_and_nonzero_remote_status() {
    if !openssh_available() {
        return;
    }
    let server = HermeticSshServer::builder()
        .command(
            "app-nonzero-status",
            CommandResponse::status(b"app-nonzero-out", b"app-nonzero-err", 37),
        )
        .start(DEADLINE)
        .expect("start SSH fixture");
    prepare_identity_for_openssh(&server);

    let mut nonzero = app_ssh_command(
        &server,
        server.agent().identity_path(),
        server.known_hosts_path(),
    );
    nonzero
        .arg("fixture-user@127.0.0.1")
        .arg("app-nonzero-status");
    let nonzero = run(nonzero);
    assert_eq!(nonzero.status.code(), Some(37));
    let nonzero_output = String::from_utf8_lossy(&nonzero.stdout);
    assert!(nonzero_output.contains("app-nonzero-out"));
    assert!(nonzero_output.contains("app-nonzero-err"));

    let untrusted = AgentFixture::new().expect("generate untrusted app identity");
    prepare_private_key_for_openssh(untrusted.identity_path());
    let mut rejected_auth = app_ssh_command(
        &server,
        untrusted.identity_path(),
        server.known_hosts_path(),
    );
    rejected_auth
        .arg("fixture-user@127.0.0.1")
        .args(["rssh-test-marker", "must-not-run"]);
    assert!(!run(rejected_auth).status.success());

    let unrelated = HermeticSshServer::start(DEADLINE).expect("start unrelated host-key fixture");
    let changed_hosts = server.temp_home().path().join("changed-known-hosts");
    std::fs::write(
        &changed_hosts,
        format!(
            "[127.0.0.1]:{} {}\n",
            server.address().port(),
            unrelated
                .host_key()
                .to_openssh()
                .expect("encode changed key")
        ),
    )
    .expect("write changed known-hosts");
    let mut rejected_host =
        app_ssh_command(&server, server.agent().identity_path(), &changed_hosts);
    rejected_host
        .arg("fixture-user@127.0.0.1")
        .args(["rssh-test-marker", "must-not-run"]);
    assert!(!run(rejected_host).status.success());

    unrelated.stop(DEADLINE).expect("stop unrelated fixture");
    server.stop(DEADLINE).expect("stop SSH fixture");
}

#[test]
fn rssh_app_openssh_local_dynamic_and_remote_forwarding_bridge_real_loopback_tcp() {
    if !openssh_available() {
        return;
    }
    let echo = LoopbackEchoServer::start(DEADLINE).expect("start echo target");
    let echo_probe = echo.connection_probe();
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    prepare_identity_for_openssh(&server);

    let occupied = TcpListener::bind(("127.0.0.1", 0)).expect("occupy first local-forward port");
    let (local, local_port) = start_forward_with_retry(
        Some(occupied),
        |local_port| {
            let mut command = app_ssh_command(
                &server,
                server.agent().identity_path(),
                server.known_hosts_path(),
            );
            command
                .arg("-N")
                .arg("-o")
                .arg("ExitOnForwardFailure=yes")
                .arg("-L")
                .arg(format!(
                    "127.0.0.1:{local_port}:127.0.0.1:{}",
                    echo.address().port()
                ));
            target(&mut command);
            command
        },
        |port| try_echo(port, b"local-readiness"),
    );
    assert_echo(local_port, b"openssh-local");
    wait_for_echo_idle(&echo_probe);
    drop(local);

    let (dynamic, dynamic_port) = start_forward_with_retry(
        None,
        |dynamic_port| {
            let mut command = app_ssh_command(
                &server,
                server.agent().identity_path(),
                server.known_hosts_path(),
            );
            command
                .arg("-N")
                .arg("-o")
                .arg("ExitOnForwardFailure=yes")
                .arg("-D")
                .arg(format!("127.0.0.1:{dynamic_port}"));
            target(&mut command);
            command
        },
        |port| try_socks_echo(port, echo.address().port(), b"dynamic-readiness"),
    );
    assert_socks_echo(dynamic_port, echo.address().port(), b"openssh-dynamic");
    wait_for_echo_idle(&echo_probe);
    drop(dynamic);

    let (remote, remote_port) = start_forward_with_retry(
        None,
        |remote_port| {
            let mut command = app_ssh_command(
                &server,
                server.agent().identity_path(),
                server.known_hosts_path(),
            );
            command
                .arg("-N")
                .arg("-o")
                .arg("ExitOnForwardFailure=yes")
                .arg("-R")
                .arg(format!(
                    "127.0.0.1:{remote_port}:127.0.0.1:{}",
                    echo.address().port()
                ));
            target(&mut command);
            command
        },
        |port| try_echo(port, b"remote-readiness"),
    );
    assert_echo(remote_port, b"openssh-remote");
    wait_for_echo_idle(&echo_probe);
    drop(remote);

    server.stop(DEADLINE).expect("stop SSH fixture");
    echo.stop(DEADLINE).expect("stop echo target");
}

fn app_ssh_command(
    server: &HermeticSshServer,
    identity: &std::path::Path,
    known_hosts: &std::path::Path,
) -> Command {
    let mut command = Command::new(packaged_or_cargo_app_executable());
    server.temp_home().apply_to(&mut command);
    command
        .env_remove("SSH_AUTH_SOCK")
        .env_remove("SSH_ASKPASS")
        .env_remove("SSH_ASKPASS_REQUIRE")
        .env_remove("DISPLAY")
        .arg("ssh")
        .arg("-F")
        .arg(server.isolated_ssh_config_path())
        .arg("-p")
        .arg(server.address().port().to_string())
        .arg("-i")
        .arg(identity)
        .arg("-o")
        .arg("StrictHostKeyChecking=yes")
        .arg("-o")
        .arg(format!("UserKnownHostsFile={}", known_hosts.display()))
        .arg("-o")
        .arg("GlobalKnownHostsFile=none")
        .arg("-o")
        .arg("IdentityAgent=none")
        .arg("-o")
        .arg("IdentitiesOnly=yes")
        .arg("-o")
        .arg("BatchMode=yes");
    command
}

fn free_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .expect("reserve loopback port")
        .local_addr()
        .expect("read loopback address")
        .port()
}

fn start_forward_with_retry<F, R>(
    mut forced_first_contention: Option<TcpListener>,
    mut command_for_port: F,
    mut readiness: R,
) -> (ChildGuard, u16)
where
    F: FnMut(u16) -> Command,
    R: FnMut(u16) -> io::Result<()>,
{
    const MAX_ATTEMPTS: usize = 5;
    for attempt in 0..MAX_ATTEMPTS {
        let forced_contention = attempt == 0 && forced_first_contention.is_some();
        let port = forced_first_contention
            .as_ref()
            .map_or_else(free_port, |listener| {
                listener.local_addr().expect("read occupied address").port()
            });
        let mut child = ChildGuard::spawn(command_for_port(port), PROCESS_DEADLINE)
            .expect("spawn app forward attempt");
        let started = Instant::now();
        loop {
            if let Some(output) = child.try_wait().expect("observe app forward attempt") {
                assert_retryable_bind_collision(&output);
                break;
            }
            if forced_contention {
                assert!(
                    started.elapsed() < PROCESS_DEADLINE,
                    "contended app forward did not report bind failure"
                );
                thread::sleep(Duration::from_millis(20));
                continue;
            }
            match readiness(port) {
                Ok(()) => match child.try_wait().expect("recheck ready app forward") {
                    None => return (child, port),
                    Some(output) => {
                        assert_retryable_bind_collision(&output);
                        break;
                    }
                },
                Err(error) if started.elapsed() < DEADLINE => {
                    if let Some(output) = child.try_wait().expect("recheck failed readiness") {
                        assert_retryable_bind_collision(&output);
                        break;
                    }
                    let _ = error;
                    thread::sleep(Duration::from_millis(20));
                }
                Err(error) => {
                    panic!("app forward listener {port} protocol readiness deadline: {error}")
                }
            }
        }
        drop(forced_first_contention.take());
    }
    panic!("app forward exhausted {MAX_ATTEMPTS} address-in-use retries")
}

fn assert_retryable_bind_collision(output: &ChildOutput) {
    let diagnostics = child_diagnostics(output);
    assert!(
        address_in_use_diagnostic(&diagnostics),
        "app forward exited for a non-retryable reason: {diagnostics:?}"
    );
}

fn try_echo(port: u16, payload: &[u8]) -> io::Result<()> {
    let mut stream = TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}")
            .parse()
            .expect("loopback address"),
        Duration::from_millis(250),
    )?;
    stream.set_read_timeout(Some(Duration::from_millis(250)))?;
    stream.set_write_timeout(Some(Duration::from_millis(250)))?;
    stream.write_all(payload)?;
    let mut echoed = vec![0_u8; payload.len()];
    stream.read_exact(&mut echoed)?;
    if echoed == payload {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "forward readiness endpoint did not echo the probe",
        ))
    }
}

fn try_socks_echo(proxy_port: u16, target_port: u16, payload: &[u8]) -> io::Result<()> {
    let mut stream = TcpStream::connect_timeout(
        &format!("127.0.0.1:{proxy_port}")
            .parse()
            .expect("loopback address"),
        Duration::from_millis(250),
    )?;
    stream.set_read_timeout(Some(Duration::from_millis(250)))?;
    stream.set_write_timeout(Some(Duration::from_millis(250)))?;
    stream.write_all(&[5, 1, 0])?;
    let mut greeting = [0_u8; 2];
    stream.read_exact(&mut greeting)?;
    if greeting != [5, 0] {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "forward readiness endpoint is not a SOCKS5 proxy",
        ));
    }
    let [port_high, port_low] = target_port.to_be_bytes();
    stream.write_all(&[5, 1, 0, 1, 127, 0, 0, 1, port_high, port_low])?;
    let mut response = [0_u8; 10];
    stream.read_exact(&mut response)?;
    if response[..2] != [5, 0] {
        return Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            "SOCKS5 readiness connect was rejected",
        ));
    }
    stream.write_all(payload)?;
    let mut echoed = vec![0_u8; payload.len()];
    stream.read_exact(&mut echoed)?;
    if echoed == payload {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "SOCKS5 readiness target did not echo the probe",
        ))
    }
}

fn child_diagnostics(output: &ChildOutput) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn address_in_use_diagnostic(diagnostics: &str) -> bool {
    let diagnostics = diagnostics.to_ascii_lowercase();
    let explicit_collision = [
        "address already in use",
        "only one usage of each socket address",
        "os error 98",
        "os error 10048",
    ]
    .iter()
    .any(|marker| diagnostics.contains(marker));
    let windows_openssh_collision = diagnostics.contains("bind [")
        && diagnostics.contains("permission denied")
        && diagnostics.contains("cannot listen to port");
    explicit_collision || windows_openssh_collision
}

fn connect_when_ready(port: u16) -> TcpStream {
    let started = Instant::now();
    loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(DEADLINE))
                    .expect("set read deadline");
                stream
                    .set_write_timeout(Some(DEADLINE))
                    .expect("set write deadline");
                return stream;
            }
            Err(error) if started.elapsed() < DEADLINE => {
                let _ = error;
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) => panic!("forward listener {port} readiness deadline: {error}"),
        }
    }
}

fn assert_echo(port: u16, payload: &[u8]) {
    let mut stream = connect_when_ready(port);
    stream.write_all(payload).expect("write forwarded payload");
    let mut echoed = vec![0_u8; payload.len()];
    stream
        .read_exact(&mut echoed)
        .expect("read forwarded payload");
    assert_eq!(echoed, payload);
}

fn assert_socks_echo(proxy_port: u16, target_port: u16, payload: &[u8]) {
    let mut stream = connect_when_ready(proxy_port);
    stream.write_all(&[5, 1, 0]).expect("write SOCKS greeting");
    let mut greeting = [0_u8; 2];
    stream
        .read_exact(&mut greeting)
        .expect("read SOCKS greeting");
    assert_eq!(greeting, [5, 0]);

    let [port_high, port_low] = target_port.to_be_bytes();
    stream
        .write_all(&[5, 1, 0, 1, 127, 0, 0, 1, port_high, port_low])
        .expect("write SOCKS connect request");
    let mut response = [0_u8; 10];
    stream
        .read_exact(&mut response)
        .expect("read SOCKS connect response");
    assert_eq!(&response[..2], &[5, 0]);

    stream.write_all(payload).expect("write SOCKS payload");
    let mut echoed = vec![0_u8; payload.len()];
    stream.read_exact(&mut echoed).expect("read SOCKS payload");
    assert_eq!(echoed, payload);
}

fn wait_for_echo_idle(probe: &LoopbackEchoProbe) {
    let started = Instant::now();
    while probe.active() != 0 {
        assert!(
            started.elapsed() < DEADLINE,
            "echo connection drain deadline"
        );
        thread::yield_now();
    }
}
