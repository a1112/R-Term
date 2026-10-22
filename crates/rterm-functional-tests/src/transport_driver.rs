use std::{
    error::Error,
    fmt, fs,
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant},
};

use rterm_test_support::{
    ChildGuard, TempHome,
    ssh::{CommandResponse, HermeticSshServer, LoopbackEchoServer},
};
use sha2::{Digest, Sha256};

use crate::hermetic_network::hermetic_app_command;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SshJourneyResult {
    pub native_stdout: String,
    pub system_stdout: String,
    pub local_forward_echo: Vec<u8>,
    pub dynamic_forward_echo: Vec<u8>,
    pub remote_forward_echo: Vec<u8>,
    pub server_trace: Vec<String>,
    pub resources_zero: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferJourneyResult {
    pub expected_sha256: Vec<u8>,
    pub sftp_download_sha256: Vec<u8>,
    pub scp_download_sha256: Vec<u8>,
    pub sftp_download: Vec<u8>,
    pub scp_download: Vec<u8>,
    pub server_trace: Vec<String>,
    pub resources_zero: bool,
}

/// Exercises native and system SSH plus real local, dynamic, and remote forwarding.
///
/// # Errors
///
/// Returns an error when any hermetic server, client process, forwarding path, or cleanup fails.
pub fn run_ssh_loopback_journey(
    app: &Path,
    deadline: Duration,
    cleanup: Duration,
) -> Result<SshJourneyResult, TransportJourneyError> {
    let budget = JourneyBudget::new(deadline, cleanup)?;
    let server = HermeticSshServer::builder()
        .command(
            "'rterm-test-marker' 'functional-ssh-native'",
            CommandResponse::status(b"functional-ssh-native", b"", 0),
        )
        .start(budget.remaining()?)
        .map_err(fixture_error)?;
    #[cfg(windows)]
    prepare_identity(&server, budget.remaining()?)?;
    let task_probe = server.task_probe();

    let mut native = isolated_app_command(app, &server);
    native
        .args([
            "ssh",
            "--native",
            "--accept-unknown-host-key",
            "--metrics-json",
        ])
        .arg("-p")
        .arg(server.address().port().to_string())
        .arg("-i")
        .arg(server.agent().identity_path())
        .arg("fixture-user@127.0.0.1")
        .args(["rterm-test-marker", "functional-ssh-native"]);
    let native = run(native, budget.remaining()?, "native SSH application entry")?;
    if !native.status.success() {
        return Err(process_failure("native SSH", &native));
    }

    let mut system = isolated_app_command(app, &server);
    system
        .arg("ssh")
        .args(system_app_common_args(&server, false))
        .arg("fixture-user@127.0.0.1")
        .args(["rterm-test-marker", "functional-ssh-system"]);
    let system = run(system, budget.remaining()?, "system SSH application entry")?;
    if !system.status.success() {
        return Err(process_failure("system SSH", &system));
    }
    let (local_forward_echo, dynamic_forward_echo, remote_forward_echo) =
        run_native_forwarding_journey(&server, app, &budget)?;
    let trace = server
        .events()
        .iter()
        .map(|event| format!("{event:?}"))
        .collect();
    server
        .stop(budget.cleanup_remaining()?)
        .map_err(fixture_error)?;
    Ok(SshJourneyResult {
        native_stdout: String::from_utf8_lossy(&native.stdout).into_owned(),
        system_stdout: String::from_utf8_lossy(&system.stdout).into_owned(),
        local_forward_echo,
        dynamic_forward_echo,
        remote_forward_echo,
        server_trace: trace,
        resources_zero: task_probe.active() == 0,
    })
}

type ForwardingEchoes = (Vec<u8>, Vec<u8>, Vec<u8>);

fn run_native_forwarding_journey(
    server: &HermeticSshServer,
    app: &Path,
    budget: &JourneyBudget,
) -> Result<ForwardingEchoes, TransportJourneyError> {
    let echo = LoopbackEchoServer::start(budget.remaining()?).map_err(|error| io_error(&error))?;
    let echo_probe = echo.connection_probe();
    let local_port = unused_loopback_port()?;
    let dynamic_port = unused_loopback_port()?;
    let remote_port = unused_loopback_port()?;
    let mut command = isolated_app_command(app, server);
    command
        .args([
            "ssh",
            "--native",
            "--accept-unknown-host-key",
            "--metrics-json",
        ])
        .arg("-p")
        .arg(server.address().port().to_string())
        .arg("-i")
        .arg(server.agent().identity_path())
        .arg("-L")
        .arg(format!(
            "127.0.0.1:{local_port}:127.0.0.1:{}",
            echo.address().port()
        ))
        .arg("-D")
        .arg(format!("127.0.0.1:{dynamic_port}"))
        .arg("-R")
        .arg(format!(
            "127.0.0.1:{remote_port}:127.0.0.1:{}",
            echo.address().port()
        ))
        .arg("--no-shell")
        .arg("fixture-user@127.0.0.1");
    let mut forward_process = ChildGuard::spawn(command, budget.remaining()?)
        .map_err(|error| TransportJourneyError(format!("native forwarding entry: {error}")))?;
    let local = echo_over_stream(
        connect_until(
            SocketAddr::from((Ipv4Addr::LOCALHOST, local_port)),
            budget,
            &mut forward_process,
        )?,
        b"functional-local-forward",
    )
    .map_err(|error| error.context("local -L forwarding"))?;
    let dynamic = echo_through_socks5(
        connect_until(
            SocketAddr::from((Ipv4Addr::LOCALHOST, dynamic_port)),
            budget,
            &mut forward_process,
        )?,
        echo.address().port(),
        b"functional-dynamic-forward",
    )
    .map_err(|error| error.context("dynamic -D forwarding"))?;
    let remote = echo_over_stream(
        connect_until(
            SocketAddr::from((Ipv4Addr::LOCALHOST, remote_port)),
            budget,
            &mut forward_process,
        )?,
        b"functional-remote-forward",
    )
    .map_err(|error| error.context("remote -R forwarding"))?;
    forward_process.cap_remaining_timeout(budget.cleanup_remaining()?);
    forward_process
        .terminate()
        .map_err(|error| TransportJourneyError(format!("terminate native forwarding: {error}")))?;
    echo.stop(budget.cleanup_remaining()?)
        .map_err(|error| io_error(&error))
        .map_err(|error| error.context("stop forwarding echo server"))?;
    if echo_probe.active() != 0 {
        return Err(TransportJourneyError(
            "forwarding echo workers remained active after shutdown".to_owned(),
        ));
    }
    Ok((local, dynamic, remote))
}

fn unused_loopback_port() -> Result<u16, TransportJourneyError> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|error| io_error(&error))?;
    listener
        .local_addr()
        .map(|address| address.port())
        .map_err(|error| io_error(&error))
}

fn connect_until(
    address: SocketAddr,
    budget: &JourneyBudget,
    process: &mut ChildGuard,
) -> Result<TcpStream, TransportJourneyError> {
    let mut last_error = None;
    while let Ok(remaining) = budget.remaining() {
        if let Some(output) = process
            .try_wait()
            .map_err(|error| TransportJourneyError(format!("observe native forwarding: {error}")))?
        {
            return Err(process_failure("native forwarding", &output));
        }
        match TcpStream::connect_timeout(&address, remaining.min(Duration::from_millis(100))) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(remaining.min(Duration::from_secs(5))))
                    .map_err(|error| io_error(&error))?;
                stream
                    .set_write_timeout(Some(remaining.min(Duration::from_secs(5))))
                    .map_err(|error| io_error(&error))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
        thread::yield_now();
    }
    Err(TransportJourneyError(format!(
        "forward listener {address} did not become ready: {}",
        last_error.map_or_else(
            || "no connection attempt".to_owned(),
            |error| error.to_string()
        )
    )))
}

fn echo_over_stream(
    mut stream: TcpStream,
    payload: &[u8],
) -> Result<Vec<u8>, TransportJourneyError> {
    stream
        .write_all(payload)
        .map_err(|error| io_error(&error))?;
    let mut echoed = vec![0; payload.len()];
    stream
        .read_exact(&mut echoed)
        .map_err(|error| io_error(&error))?;
    Ok(echoed)
}

fn echo_through_socks5(
    mut stream: TcpStream,
    target_port: u16,
    payload: &[u8],
) -> Result<Vec<u8>, TransportJourneyError> {
    stream
        .write_all(&[5, 1, 0])
        .map_err(|error| io_error(&error))?;
    let mut greeting = [0; 2];
    stream
        .read_exact(&mut greeting)
        .map_err(|error| io_error(&error))?;
    if greeting != [5, 0] {
        return Err(TransportJourneyError(format!(
            "SOCKS5 greeting was {greeting:?}"
        )));
    }
    let [port_high, port_low] = target_port.to_be_bytes();
    stream
        .write_all(&[5, 1, 0, 1, 127, 0, 0, 1, port_high, port_low])
        .map_err(|error| io_error(&error))?;
    let mut response = [0; 10];
    stream
        .read_exact(&mut response)
        .map_err(|error| io_error(&error))?;
    if response[0] != 5 || response[1] != 0 {
        return Err(TransportJourneyError(format!(
            "SOCKS5 connect failed with {response:?}"
        )));
    }
    echo_over_stream(stream, payload)
}

/// Exercises real SFTP and SCP upload/download round trips with SHA-256 evidence.
///
/// # Errors
///
/// Returns an error when fixture startup, transfer, content validation, or cleanup fails.
pub fn run_transfer_roundtrip_journey(
    app: &Path,
    deadline: Duration,
    cleanup: Duration,
) -> Result<TransferJourneyResult, TransportJourneyError> {
    let budget = JourneyBudget::new(deadline, cleanup)?;
    let server = HermeticSshServer::start(budget.remaining()?).map_err(fixture_error)?;
    #[cfg(windows)]
    prepare_identity(&server, budget.remaining()?)?;
    let task_probe = server.task_probe();
    let client = TempHome::new().map_err(|error| TransportJourneyError(error.to_string()))?;
    let payload = b"rterm-functional-transfer-content\0\xff";
    let source = client.path().join("source.bin");
    fs::write(&source, payload).map_err(|error| io_error(&error))?;

    let sftp_download = client.path().join("sftp-download.bin");
    let batch = client.path().join("commands.sftp");
    fs::write(
        &batch,
        format!(
            "put {} functional-sftp.bin\nget functional-sftp.bin {}\n",
            portable_path(&source),
            portable_path(&sftp_download)
        ),
    )
    .map_err(|error| io_error(&error))?;
    let mut sftp = isolated_app_command(app, &server);
    sftp.arg("sftp")
        .args(system_app_common_args(&server, true))
        .arg("-b")
        .arg(&batch)
        .arg("fixture-user@127.0.0.1");
    let sftp = run(sftp, budget.remaining()?, "application SFTP round trip")?;
    if !sftp.status.success() {
        return Err(process_failure("SFTP", &sftp));
    }

    let mut scp_upload = isolated_app_command(app, &server);
    scp_upload
        .arg("scp")
        .args(system_app_common_args(&server, true))
        .arg("-O")
        .arg(&source)
        .arg("fixture-user@127.0.0.1:functional-scp.bin");
    let upload = run(scp_upload, budget.remaining()?, "application SCP upload")?;
    if !upload.status.success() {
        return Err(process_failure("SCP upload", &upload));
    }
    let scp_download = client.path().join("scp-download.bin");
    let mut scp_get = isolated_app_command(app, &server);
    scp_get
        .arg("scp")
        .args(system_app_common_args(&server, true))
        .arg("-O")
        .arg("fixture-user@127.0.0.1:functional-scp.bin")
        .arg(&scp_download);
    let download = run(scp_get, budget.remaining()?, "application SCP download")?;
    if !download.status.success() {
        return Err(process_failure("SCP download", &download));
    }

    let expected = Sha256::digest(payload).to_vec();
    let sftp_download = fs::read(sftp_download).map_err(|error| io_error(&error))?;
    let scp_download = fs::read(scp_download).map_err(|error| io_error(&error))?;
    let sftp_hash = Sha256::digest(&sftp_download).to_vec();
    let scp_hash = Sha256::digest(&scp_download).to_vec();
    let trace = server
        .events()
        .iter()
        .map(|event| format!("{event:?}"))
        .collect();
    server
        .stop(budget.cleanup_remaining()?)
        .map_err(fixture_error)?;
    Ok(TransferJourneyResult {
        expected_sha256: expected,
        sftp_download_sha256: sftp_hash,
        scp_download_sha256: scp_hash,
        sftp_download,
        scp_download,
        server_trace: trace,
        resources_zero: task_probe.active() == 0,
    })
}

fn isolated_app_command(app: &Path, server: &HermeticSshServer) -> Command {
    let mut command = hermetic_app_command(app);
    server.temp_home().apply_to(&mut command);
    command
        .env_remove("SSH_AUTH_SOCK")
        .env_remove("SSH_ASKPASS")
        .env_remove("SSH_ASKPASS_REQUIRE")
        .env_remove("DISPLAY");
    command
}

fn system_app_common_args(server: &HermeticSshServer, transfer: bool) -> Vec<String> {
    vec![
        "-F".to_owned(),
        server.isolated_ssh_config_path().display().to_string(),
        if transfer { "-P" } else { "-p" }.to_owned(),
        server.address().port().to_string(),
        "-i".to_owned(),
        server.agent().identity_path().display().to_string(),
        "-o".to_owned(),
        "StrictHostKeyChecking=yes".to_owned(),
        "-o".to_owned(),
        format!("UserKnownHostsFile={}", server.known_hosts_path().display()),
        "-o".to_owned(),
        "GlobalKnownHostsFile=none".to_owned(),
        "-o".to_owned(),
        "IdentityAgent=none".to_owned(),
        "-o".to_owned(),
        "IdentitiesOnly=yes".to_owned(),
        "-o".to_owned(),
        "BatchMode=yes".to_owned(),
    ]
}

fn run(
    command: Command,
    deadline: Duration,
    operation: &'static str,
) -> Result<rterm_test_support::ChildOutput, TransportJourneyError> {
    ChildGuard::spawn(command, deadline)
        .map_err(|error| TransportJourneyError(format!("{operation}: {error}")))?
        .wait()
        .map_err(|error| TransportJourneyError(format!("{operation}: {error}")))
}

#[cfg(windows)]
fn prepare_identity(
    server: &HermeticSshServer,
    deadline: Duration,
) -> Result<(), TransportJourneyError> {
    let principal =
        std::env::var("USERNAME").map_err(|error| TransportJourneyError(error.to_string()))?;
    let mut command = Command::new("icacls.exe");
    command
        .arg(server.agent().identity_path())
        .arg("/inheritance:r")
        .arg("/grant:r")
        .arg(format!("{principal}:(R)"));
    let output = run(command, deadline, "restrict OpenSSH identity ACL")?;
    if output.status.success() {
        Ok(())
    } else {
        Err(process_failure("icacls", &output))
    }
}

fn process_failure(
    operation: &str,
    output: &rterm_test_support::ChildOutput,
) -> TransportJourneyError {
    TransportJourneyError(format!(
        "{operation} exited {:?}; stdout={}; stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn portable_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn fixture_error(error: impl fmt::Display) -> TransportJourneyError {
    TransportJourneyError(error.to_string())
}

fn io_error(error: &std::io::Error) -> TransportJourneyError {
    TransportJourneyError(error.to_string())
}

#[derive(Clone, Copy)]
struct JourneyBudget {
    deadline: Instant,
    cleanup: Duration,
}

impl JourneyBudget {
    fn new(total: Duration, cleanup: Duration) -> Result<Self, TransportJourneyError> {
        let deadline = Instant::now().checked_add(total).ok_or_else(|| {
            TransportJourneyError("transport journey deadline overflow".to_owned())
        })?;
        Ok(Self { deadline, cleanup })
    }

    fn remaining(self) -> Result<Duration, TransportJourneyError> {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            Err(TransportJourneyError(
                "transport journey exceeded its single absolute deadline".to_owned(),
            ))
        } else {
            Ok(remaining)
        }
    }

    fn cleanup_remaining(self) -> Result<Duration, TransportJourneyError> {
        self.remaining()
            .map(|remaining| remaining.min(self.cleanup))
    }
}

#[derive(Debug)]
pub struct TransportJourneyError(String);

impl fmt::Display for TransportJourneyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for TransportJourneyError {}

impl TransportJourneyError {
    fn context(self, operation: &str) -> Self {
        Self(format!("{operation}: {}", self.0))
    }
}
