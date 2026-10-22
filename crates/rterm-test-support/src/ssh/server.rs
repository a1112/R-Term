use std::{
    collections::{HashMap, HashSet},
    fmt,
    future::Future,
    io,
    net::{Ipv4Addr, SocketAddr, TcpListener},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use russh::{
    Channel, ChannelId, Pty,
    keys::{Algorithm, PrivateKey, ssh_key},
    server::{self, Msg, Session},
};
use tokio::{io::AsyncWriteExt as _, sync::watch, task::JoinSet};

use super::{
    AgentFixture, LoopbackEndpoint, SftpRoot,
    lifecycle::{
        OwnedTask, ReapFuture, ShutdownDeadline, ThreadJoinOutcome, defer_future,
        ensure_process_reaper, join_thread_until,
    },
    scp::{parse_scp_request, serve_scp},
    sftp::SandboxedSftpSession,
};
use crate::TempHome;

const DROP_DEADLINE: Duration = Duration::from_millis(750);
const SESSION_TEARDOWN: Duration = Duration::from_millis(500);
const SECONDARY_DRAIN: Duration = Duration::from_millis(100);
const MAX_SHELL_COMMAND: usize = 4096;

/// Selects the command-line port convention used by an OpenSSH client tool.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpenSshTool {
    Ssh,
    Sftp,
    Scp,
}

/// A deterministic response emitted by a configured fixture command.
#[derive(Clone, Debug)]
pub struct CommandResponse {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    termination: CommandTermination,
}

#[derive(Clone, Debug)]
enum CommandTermination {
    Status(u32),
    Stall,
    Signal {
        signal: russh::Sig,
        core_dumped: bool,
        error_message: String,
    },
}

impl CommandResponse {
    /// Builds a response ending with an SSH exit-status request.
    #[must_use]
    pub fn status(stdout: impl AsRef<[u8]>, stderr: impl AsRef<[u8]>, status: u32) -> Self {
        Self {
            stdout: stdout.as_ref().to_vec(),
            stderr: stderr.as_ref().to_vec(),
            termination: CommandTermination::Status(status),
        }
    }

    /// Builds a response ending with an SSH exit-signal request.
    #[must_use]
    pub fn signal(
        stdout: impl AsRef<[u8]>,
        stderr: impl AsRef<[u8]>,
        signal: russh::Sig,
        core_dumped: bool,
        error_message: impl Into<String>,
    ) -> Self {
        Self {
            stdout: stdout.as_ref().to_vec(),
            stderr: stderr.as_ref().to_vec(),
            termination: CommandTermination::Signal {
                signal,
                core_dumped,
                error_message: error_message.into(),
            },
        }
    }
}

/// Authentication and command configuration for a hermetic SSH fixture.
#[derive(Default)]
pub struct HermeticSshServerBuilder {
    passwords: HashMap<String, String>,
    authorized_keys: Vec<ssh_key::PublicKey>,
    commands: HashMap<String, CommandResponse>,
    authentication_delay: Option<Duration>,
    channel_open_delay: Option<Duration>,
    #[cfg(test)]
    never_finish_child_drop_delay: Option<Duration>,
    #[cfg(test)]
    agent_teardown_delay: Option<Duration>,
    #[cfg(test)]
    worker_panic_before_ready: bool,
    #[cfg(test)]
    worker_panic_after_ready: bool,
}

struct FixtureBehavior {
    authorized_keys: Vec<ssh_key::PublicKey>,
    passwords: HashMap<String, String>,
    commands: HashMap<String, CommandResponse>,
    authentication_delay: Option<Duration>,
    channel_open_delay: Option<Duration>,
}

impl HermeticSshServerBuilder {
    /// Adds one exact username/password credential.
    #[must_use]
    pub fn password(mut self, user: impl Into<String>, password: impl Into<String>) -> Self {
        self.passwords.insert(user.into(), password.into());
        self
    }

    /// Authorizes an additional public key for any fixture username.
    #[must_use]
    pub fn authorize_public_key(mut self, public_key: ssh_key::PublicKey) -> Self {
        self.authorized_keys.push(public_key);
        self
    }

    /// Adds one exact, shell-free command and its deterministic response.
    #[must_use]
    pub fn command(mut self, command: impl Into<String>, response: CommandResponse) -> Self {
        self.commands.insert(command.into(), response);
        self
    }

    /// Delays every authentication decision to exercise client deadlines.
    #[must_use]
    pub const fn authentication_delay(mut self, delay: Duration) -> Self {
        self.authentication_delay = Some(delay);
        self
    }

    /// Delays session-channel confirmation to exercise client deadlines.
    #[must_use]
    pub const fn channel_open_delay(mut self, delay: Duration) -> Self {
        self.channel_open_delay = Some(delay);
        self
    }

    /// Accepts an exact command without sending output, EOF, or close.
    #[must_use]
    pub fn stalled_command(mut self, command: impl Into<String>) -> Self {
        self.commands.insert(
            command.into(),
            CommandResponse {
                stdout: Vec::new(),
                stderr: Vec::new(),
                termination: CommandTermination::Stall,
            },
        );
        self
    }

    #[cfg(test)]
    fn test_never_finish_child(mut self, drop_delay: Duration) -> Self {
        self.never_finish_child_drop_delay = Some(drop_delay);
        self
    }

    #[cfg(test)]
    fn test_agent_teardown_delay(mut self, delay: Duration) -> Self {
        self.agent_teardown_delay = Some(delay);
        self
    }

    #[cfg(test)]
    fn test_worker_panic_before_ready(mut self) -> Self {
        self.worker_panic_before_ready = true;
        self
    }

    #[cfg(test)]
    fn test_worker_panic_after_ready(mut self) -> Self {
        self.worker_panic_after_ready = true;
        self
    }

    /// Starts the configured server within `deadline`.
    ///
    /// # Errors
    ///
    /// Returns an error when isolated state, keys, TCP binding, or worker startup fails.
    pub fn start(self, deadline: Duration) -> Result<HermeticSshServer, SshFixtureError> {
        HermeticSshServer::start_configured(self, deadline)
    }
}

/// Observable events emitted by the hermetic SSH server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SshEvent {
    Connection {
        peer: SocketAddr,
    },
    PublicKeyAuth {
        user: String,
        fingerprint: String,
        accepted: bool,
    },
    SessionOpened,
    Pty {
        term: String,
        columns: u32,
        rows: u32,
        pixel_width: u32,
        pixel_height: u32,
    },
    Shell,
    Exec {
        command: String,
        accepted: bool,
    },
    Resize {
        columns: u32,
        rows: u32,
        pixel_width: u32,
        pixel_height: u32,
    },
    DirectTcpip {
        target: String,
        port: u32,
        accepted: bool,
    },
    RemoteForward {
        address: String,
        port: u32,
        accepted: bool,
    },
    RemoteForwardCancelled {
        address: String,
        port: u32,
        accepted: bool,
    },
    AgentForward {
        accepted: bool,
    },
    Subsystem {
        name: String,
        accepted: bool,
    },
}

/// A start, runtime, or bounded teardown failure from the fixture.
#[derive(Debug)]
pub enum SshFixtureError {
    Io {
        operation: &'static str,
        source: io::Error,
    },
    TimedOut {
        operation: &'static str,
        deadline: Duration,
    },
    WorkerPanicked,
}

impl fmt::Display for SshFixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { operation, source } => {
                write!(formatter, "SSH fixture {operation}: {source}")
            }
            Self::TimedOut {
                operation,
                deadline,
            } => write!(
                formatter,
                "SSH fixture {operation} exceeded its {deadline:?} deadline"
            ),
            Self::WorkerPanicked => formatter.write_str("SSH fixture worker panicked"),
        }
    }
}

impl std::error::Error for SshFixtureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::TimedOut { .. } | Self::WorkerPanicked => None,
        }
    }
}

/// A real loopback russh server with isolated keys, identity, known-hosts, and SFTP root.
pub struct HermeticSshServer {
    address: SocketAddr,
    host_key: ssh_key::PublicKey,
    _known_hosts_directory: tempfile::TempDir,
    known_hosts_path: PathBuf,
    isolated_ssh_config_path: PathBuf,
    temp_home: TempHome,
    agent: Option<AgentFixture>,
    sftp: SftpRoot,
    events: Arc<Mutex<Vec<SshEvent>>>,
    task_probe: SshTaskProbe,
    cancellation: watch::Sender<Option<ShutdownDeadline>>,
    completion: mpsc::Receiver<Result<(), SshFixtureError>>,
    worker: Option<thread::JoinHandle<()>>,
}

/// A cloneable observation handle for fixture-owned asynchronous child tasks.
#[derive(Clone)]
pub struct SshTaskProbe {
    active: Arc<AtomicUsize>,
}

impl SshTaskProbe {
    /// Returns the number of registered child tasks which have not completed.
    #[must_use]
    pub fn active(&self) -> usize {
        self.active.load(Ordering::Acquire)
    }
}

impl HermeticSshServer {
    /// Returns a builder for optional authentication and command behavior.
    #[must_use]
    pub fn builder() -> HermeticSshServerBuilder {
        HermeticSshServerBuilder::default()
    }

    /// Starts a ready server bound exclusively to `127.0.0.1` within `deadline`.
    ///
    /// # Errors
    ///
    /// Returns an error when isolated state cannot be created, loopback binding or
    /// runtime startup fails, or readiness exceeds the supplied deadline.
    pub fn start(deadline: Duration) -> Result<Self, SshFixtureError> {
        Self::builder().start(deadline)
    }

    fn start_configured(
        builder: HermeticSshServerBuilder,
        deadline: Duration,
    ) -> Result<Self, SshFixtureError> {
        let startup = ShutdownDeadline::after(deadline);
        ensure_fixture_reaper()?;
        let agent = create_fixture_agent(builder_agent_teardown_delay(&builder))?;
        let sftp = SftpRoot::new().map_err(|source| SshFixtureError::Io {
            operation: "SFTP root setup failed",
            source,
        })?;
        let host_private =
            PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).map_err(|error| {
                SshFixtureError::Io {
                    operation: "host key generation failed",
                    source: io::Error::other(error.to_string()),
                }
            })?;
        let host_key = host_private.public_key().clone();
        let (listener, address) = bind_fixture_listener()?;
        let (known_hosts_directory, known_hosts_path, isolated_ssh_config_path, temp_home) =
            create_openssh_state(address, &host_key)?;

        let events = Arc::new(Mutex::new(Vec::new()));
        let (cancellation, cancellation_rx) = watch::channel(None);
        let (completion_tx, completion) = mpsc::sync_channel(1);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let task_probe = SshTaskProbe {
            active: Arc::new(AtomicUsize::new(0)),
        };
        #[cfg(test)]
        let worker_panic_before_ready = builder.worker_panic_before_ready;
        #[cfg(test)]
        let worker_panic_after_ready = builder.worker_panic_after_ready;
        let handler = build_fixture_handler(
            builder,
            agent.public_key().clone(),
            Arc::clone(&events),
            sftp.path().to_path_buf(),
            task_probe.clone(),
        );
        let worker = thread::Builder::new()
            .name("rterm-hermetic-ssh".to_owned())
            .spawn(move || {
                #[cfg(test)]
                assert!(
                    !worker_panic_before_ready,
                    "injected SSH startup worker panic"
                );
                let result =
                    run_server_worker(listener, host_private, handler, cancellation_rx, ready_tx);
                #[cfg(test)]
                assert!(
                    !worker_panic_after_ready,
                    "injected SSH teardown worker panic"
                );
                let _ = completion_tx.send(result);
            })
            .map_err(|source| SshFixtureError::Io {
                operation: "worker spawn failed",
                source,
            })?;

        match ready_rx.recv_timeout(startup.remaining()) {
            Ok(Ok(())) => Ok(Self {
                address,
                host_key,
                _known_hosts_directory: known_hosts_directory,
                known_hosts_path,
                isolated_ssh_config_path,
                temp_home,
                agent: Some(agent),
                sftp,
                events,
                task_probe,
                cancellation,
                completion,
                worker: Some(worker),
            }),
            Ok(Err(error)) => match join_thread_until(worker, startup) {
                ThreadJoinOutcome::Completed => Err(error),
                ThreadJoinOutcome::Panicked => Err(SshFixtureError::WorkerPanicked),
                ThreadJoinOutcome::Deferred => Err(SshFixtureError::TimedOut {
                    operation: "startup worker join",
                    deadline,
                }),
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let _ = cancellation.send(Some(startup));
                defer_join(worker);
                Err(SshFixtureError::TimedOut {
                    operation: "startup",
                    deadline,
                })
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                finish_missing_server_completion(worker, startup, "startup completion")
            }
        }
    }

    /// Returns the ready server address.
    #[must_use]
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// Returns the runtime-generated host public key.
    #[must_use]
    pub fn host_key(&self) -> &ssh_key::PublicKey {
        &self.host_key
    }

    /// Returns the isolated known-hosts path containing only this server.
    #[must_use]
    pub fn known_hosts_path(&self) -> &Path {
        &self.known_hosts_path
    }

    /// Returns the empty OpenSSH configuration file forced with `-F`.
    #[must_use]
    pub fn isolated_ssh_config_path(&self) -> &Path {
        &self.isolated_ssh_config_path
    }

    /// Returns the isolated HOME/USERPROFILE environment used for system clients.
    #[must_use]
    pub fn temp_home(&self) -> &TempHome {
        &self.temp_home
    }

    /// Returns the injected client identity fixture.
    ///
    /// # Panics
    ///
    /// Panics only if invoked by an internal teardown path after agent ownership
    /// was taken. Public teardown consumes the server, so callers cannot observe it.
    #[must_use]
    pub fn agent(&self) -> &AgentFixture {
        self.agent.as_ref().expect("fixture agent available")
    }

    /// Returns the isolated transfer root.
    #[must_use]
    pub fn sftp(&self) -> &SftpRoot {
        &self.sftp
    }

    /// Returns a probe for asserting that fixture-owned child tasks are drained.
    #[must_use]
    pub fn task_probe(&self) -> SshTaskProbe {
        self.task_probe.clone()
    }

    /// Configures an OpenSSH command to trust and authenticate only this fixture.
    pub fn configure_ssh_command(&self, command: &mut Command, user: &str) {
        self.configure_openssh_command(command, OpenSshTool::Ssh);
        command.arg(format!("{user}@127.0.0.1"));
    }

    /// Applies common isolated options for the system `ssh`, `sftp`, or `scp` client.
    ///
    /// The caller remains responsible for appending the remote target and any
    /// tool-specific operation arguments.
    pub fn configure_openssh_command(&self, command: &mut Command, tool: OpenSshTool) {
        self.temp_home.apply_to(command);
        self.agent().configure_ssh_command(command);
        let port_flag = match tool {
            OpenSshTool::Ssh => "-p",
            OpenSshTool::Sftp | OpenSshTool::Scp => "-P",
        };
        command
            .arg("-F")
            .arg(&self.isolated_ssh_config_path)
            .arg(port_flag)
            .arg(self.address.port().to_string())
            .arg("-o")
            .arg("StrictHostKeyChecking=yes")
            .arg("-o")
            .arg(format!(
                "UserKnownHostsFile={}",
                self.known_hosts_path.display()
            ))
            .arg("-o")
            .arg("GlobalKnownHostsFile=none")
            .arg("-o")
            .arg("IdentityAgent=none")
            .arg("-o")
            .arg("IdentitiesOnly=yes")
            .arg("-o")
            .arg("BatchMode=yes")
            .arg("-o")
            .arg("PreferredAuthentications=publickey")
            .arg("-o")
            .arg("PasswordAuthentication=no")
            .arg("-o")
            .arg("KbdInteractiveAuthentication=no")
            .arg("-o")
            .arg("NumberOfPasswordPrompts=0")
            .env_remove("SSH_AUTH_SOCK")
            .env_remove("SSH_ASKPASS")
            .env_remove("SSH_ASKPASS_REQUIRE")
            .env_remove("DISPLAY");
    }

    /// Returns a point-in-time copy of recorded server events.
    #[must_use]
    pub fn events(&self) -> Vec<SshEvent> {
        lock_events(&self.events).clone()
    }

    /// Cancels all listeners and sessions and joins the worker within `deadline`.
    ///
    /// # Errors
    ///
    /// Returns an error if teardown exceeds the deadline or the server worker fails.
    pub fn stop(mut self, deadline: Duration) -> Result<(), SshFixtureError> {
        self.stop_inner(deadline)
    }

    fn stop_inner(&mut self, deadline: Duration) -> Result<(), SshFixtureError> {
        let shutdown = ShutdownDeadline::after(deadline);
        let primary = self.stop_ssh_worker(shutdown);
        let agent = self.agent.take().map_or(Ok(()), |agent| {
            agent
                .stop_until(shutdown)
                .map_err(|source| map_agent_teardown_error(source, shutdown))
        });
        match (primary, agent) {
            (Err(primary), _) => Err(primary),
            (Ok(()), result) => result,
        }
    }

    fn stop_ssh_worker(&mut self, shutdown: ShutdownDeadline) -> Result<(), SshFixtureError> {
        if self.worker.is_none() {
            return Ok(());
        }
        let _ = self.cancellation.send(Some(shutdown));
        let result = match self.completion.recv_timeout(shutdown.remaining()) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(worker) = self.worker.take() {
                    defer_join(worker);
                }
                return Err(SshFixtureError::TimedOut {
                    operation: "teardown",
                    deadline: shutdown.budget(),
                });
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let Some(worker) = self.worker.take() else {
                    return Err(missing_server_completion("teardown completion"));
                };
                return finish_missing_server_completion(worker, shutdown, "teardown completion");
            }
        };
        if let Some(worker) = self.worker.take() {
            match join_thread_until(worker, shutdown) {
                ThreadJoinOutcome::Completed => {}
                ThreadJoinOutcome::Panicked => return Err(SshFixtureError::WorkerPanicked),
                ThreadJoinOutcome::Deferred => {
                    return Err(SshFixtureError::TimedOut {
                        operation: "worker join",
                        deadline: shutdown.budget(),
                    });
                }
            }
        }
        result
    }
}

fn build_fixture_handler(
    builder: HermeticSshServerBuilder,
    agent_public_key: ssh_key::PublicKey,
    events: Arc<Mutex<Vec<SshEvent>>>,
    sftp_root: PathBuf,
    task_probe: SshTaskProbe,
) -> FixtureHandler {
    let mut authorized_keys = builder.authorized_keys;
    authorized_keys.push(agent_public_key);
    FixtureHandler::new(
        FixtureBehavior {
            authorized_keys,
            passwords: builder.passwords,
            commands: builder.commands,
            authentication_delay: builder.authentication_delay,
            channel_open_delay: builder.channel_open_delay,
        },
        events,
        sftp_root,
        task_probe,
        #[cfg(test)]
        builder.never_finish_child_drop_delay,
    )
}

fn map_agent_teardown_error(source: io::Error, shutdown: ShutdownDeadline) -> SshFixtureError {
    if source.kind() == io::ErrorKind::TimedOut {
        SshFixtureError::TimedOut {
            operation: "agent teardown",
            deadline: shutdown.budget(),
        }
    } else {
        SshFixtureError::Io {
            operation: "agent teardown failed",
            source,
        }
    }
}

fn create_fixture_agent(teardown_delay: Option<Duration>) -> Result<AgentFixture, SshFixtureError> {
    AgentFixture::new_configured(teardown_delay, false).map_err(|source| SshFixtureError::Io {
        operation: "client identity setup failed",
        source,
    })
}

#[cfg(test)]
fn builder_agent_teardown_delay(builder: &HermeticSshServerBuilder) -> Option<Duration> {
    builder.agent_teardown_delay
}

#[cfg(not(test))]
fn builder_agent_teardown_delay(_builder: &HermeticSshServerBuilder) -> Option<Duration> {
    None
}

fn bind_fixture_listener() -> Result<(TcpListener, SocketAddr), SshFixtureError> {
    let listener =
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|source| SshFixtureError::Io {
            operation: "loopback bind failed",
            source,
        })?;
    listener
        .set_nonblocking(true)
        .map_err(|source| SshFixtureError::Io {
            operation: "listener nonblocking setup failed",
            source,
        })?;
    let address = listener
        .local_addr()
        .map_err(|source| SshFixtureError::Io {
            operation: "listener address lookup failed",
            source,
        })?;
    Ok((listener, address))
}

fn ensure_fixture_reaper() -> Result<(), SshFixtureError> {
    ensure_process_reaper().map_err(|source| SshFixtureError::Io {
        operation: "process reaper startup failed",
        source,
    })
}

impl Drop for HermeticSshServer {
    fn drop(&mut self) {
        if self.worker.is_some() {
            let _ = self.stop_inner(DROP_DEADLINE);
        }
    }
}

fn write_known_hosts(
    path: &Path,
    address: SocketAddr,
    host_key: &ssh_key::PublicKey,
) -> Result<(), SshFixtureError> {
    let encoded = host_key.to_openssh().map_err(|error| SshFixtureError::Io {
        operation: "host public key encoding failed",
        source: io::Error::other(error.to_string()),
    })?;
    std::fs::write(path, format!("[127.0.0.1]:{} {encoded}\n", address.port())).map_err(|source| {
        SshFixtureError::Io {
            operation: "known-hosts write failed",
            source,
        }
    })
}

fn create_openssh_state(
    address: SocketAddr,
    host_key: &ssh_key::PublicKey,
) -> Result<(tempfile::TempDir, PathBuf, PathBuf, TempHome), SshFixtureError> {
    let directory = tempfile::Builder::new()
        .prefix("rterm-known-hosts-")
        .tempdir()
        .map_err(|source| SshFixtureError::Io {
            operation: "known-hosts directory setup failed",
            source,
        })?;
    let known_hosts = directory.path().join("known_hosts");
    write_known_hosts(&known_hosts, address, host_key)?;
    let ssh_config = directory.path().join("ssh_config");
    std::fs::write(&ssh_config, []).map_err(|source| SshFixtureError::Io {
        operation: "isolated ssh_config write failed",
        source,
    })?;
    let temp_home = TempHome::new().map_err(|source| SshFixtureError::Io {
        operation: "isolated HOME setup failed",
        source,
    })?;
    Ok((directory, known_hosts, ssh_config, temp_home))
}

fn run_server_worker(
    listener: TcpListener,
    host_key: PrivateKey,
    handler: FixtureHandler,
    cancellation: watch::Receiver<Option<ShutdownDeadline>>,
    ready: mpsc::SyncSender<Result<(), SshFixtureError>>,
) -> Result<(), SshFixtureError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|source| SshFixtureError::Io {
            operation: "runtime setup failed",
            source,
        });
    let runtime = match runtime {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = ready.send(Err(error));
            return Ok(());
        }
    };
    let runtime_shutdown = cancellation.clone();
    let result = runtime.block_on(async move {
        let listener =
            tokio::net::TcpListener::from_std(listener).map_err(|source| SshFixtureError::Io {
                operation: "async listener setup failed",
                source,
            });
        let listener = match listener {
            Ok(listener) => listener,
            Err(error) => {
                let _ = ready.send(Err(error));
                return Ok(());
            }
        };
        let config = Arc::new(server::Config {
            auth_rejection_time: Duration::ZERO,
            auth_rejection_time_initial: Some(Duration::ZERO),
            inactivity_timeout: Some(Duration::from_secs(30)),
            nodelay: true,
            keys: vec![host_key],
            ..server::Config::default()
        });
        let _ = ready.send(Ok(()));
        run_accept_loop(listener, config, handler, cancellation).await
    });
    let shutdown =
        (*runtime_shutdown.borrow()).unwrap_or_else(|| ShutdownDeadline::after(SESSION_TEARDOWN));
    runtime.shutdown_timeout(shutdown.remaining());
    result
}

async fn run_accept_loop(
    listener: tokio::net::TcpListener,
    config: Arc<server::Config>,
    handler: FixtureHandler,
    mut cancellation: watch::Receiver<Option<ShutdownDeadline>>,
) -> Result<(), SshFixtureError> {
    let mut sessions = JoinSet::new();
    let mut session_error = None;
    let shutdown = loop {
        tokio::select! {
            biased;
            deadline = shutdown_requested(&mut cancellation) => break deadline,
            accepted = listener.accept() => {
                let (socket, peer) = accepted.map_err(|source| SshFixtureError::Io {
                    operation: "accept failed",
                    source,
                })?;
                record(&handler.events, SshEvent::Connection { peer });
                let config = Arc::clone(&config);
                let session_handler = handler.clone_for_session();
                let session_cancellation = cancellation.clone();
                sessions.spawn(run_session(socket, config, session_handler, session_cancellation));
            }
            joined = sessions.join_next(), if !sessions.is_empty() => {
                record_session_result(joined, &mut session_error);
            }
        }
    };
    drop(listener);
    while !sessions.is_empty() {
        match shutdown.timeout(sessions.join_next()).await {
            Ok(joined) => record_session_result(joined, &mut session_error),
            Err(_) => break,
        }
    }
    if !sessions.is_empty() {
        sessions.abort_all();
        let fallback = ShutdownDeadline::after(SECONDARY_DRAIN);
        while !sessions.is_empty() {
            match fallback.timeout(sessions.join_next()).await {
                Ok(joined) => record_session_result(joined, &mut session_error),
                Err(_) => break,
            }
        }
        if !sessions.is_empty() {
            let future = Box::pin(async move { while sessions.join_next().await.is_some() {} });
            let _ = defer_future(future);
        }
        return Err(SshFixtureError::TimedOut {
            operation: "session drain",
            deadline: shutdown.budget(),
        });
    }
    session_error.map_or(Ok(()), Err)
}

fn record_session_result(
    joined: Option<Result<Result<(), SshFixtureError>, tokio::task::JoinError>>,
    first_error: &mut Option<SshFixtureError>,
) {
    let error = match joined {
        Some(Ok(Ok(()))) | None => return,
        Some(Ok(Err(error))) => error,
        Some(Err(_)) => SshFixtureError::WorkerPanicked,
    };
    first_error.get_or_insert(error);
}

async fn run_session(
    socket: tokio::net::TcpStream,
    config: Arc<server::Config>,
    handler: FixtureHandler,
    mut cancellation: watch::Receiver<Option<ShutdownDeadline>>,
) -> Result<(), SshFixtureError> {
    let tasks = handler.tasks.clone();
    let running = tokio::select! {
        biased;
        deadline = shutdown_requested(&mut cancellation) => {
            let completed = tasks.shutdown(deadline).await;
            return if completed { Ok(()) } else { Err(shutdown_timeout("session startup", deadline)) };
        },
        result = server::run_stream(config, socket, handler) => if let Ok(running) = result {
            running
        } else {
                let deadline = ShutdownDeadline::after(SESSION_TEARDOWN);
                let completed = tasks.shutdown(deadline).await;
                return if completed { Ok(()) } else { Err(shutdown_timeout("session startup", deadline)) };
        },
    };
    let handle = running.handle();
    let mut completion = ReapOnDrop::new(Box::pin(async move {
        let _ = running.await;
    }));
    tokio::select! {
        biased;
        deadline = shutdown_requested(&mut cancellation) => {
            let disconnected = deadline.timeout(handle.disconnect(
                russh::Disconnect::ByApplication,
                "fixture shutdown".to_owned(),
                String::new(),
            )).await.is_ok();
            let (session_completed, children_completed) = tokio::join!(
                completion.wait_until(deadline),
                tasks.shutdown(deadline),
            );
            if disconnected && session_completed && children_completed {
                Ok(())
            } else {
                Err(shutdown_timeout("session teardown", deadline))
            }
        }
        () = completion.wait() => {
            let deadline = ShutdownDeadline::after(SESSION_TEARDOWN);
            if tasks.shutdown(deadline).await {
                Ok(())
            } else {
                Err(shutdown_timeout("session child drain", deadline))
            }
        }
    }
}

fn shutdown_timeout(operation: &'static str, deadline: ShutdownDeadline) -> SshFixtureError {
    SshFixtureError::TimedOut {
        operation,
        deadline: deadline.budget(),
    }
}

async fn shutdown_requested(
    cancellation: &mut watch::Receiver<Option<ShutdownDeadline>>,
) -> ShutdownDeadline {
    loop {
        if let Some(deadline) = *cancellation.borrow() {
            return deadline;
        }
        if cancellation.changed().await.is_err() {
            return ShutdownDeadline::after(Duration::ZERO);
        }
    }
}

async fn cancelled(cancellation: &mut watch::Receiver<bool>) {
    if *cancellation.borrow() {
        return;
    }
    let _ = cancellation.changed().await;
}

fn defer_join(worker: thread::JoinHandle<()>) {
    super::lifecycle::defer_thread(worker);
}

fn finish_missing_server_completion<T>(
    worker: thread::JoinHandle<()>,
    deadline: ShutdownDeadline,
    operation: &'static str,
) -> Result<T, SshFixtureError> {
    match join_thread_until(worker, deadline) {
        ThreadJoinOutcome::Completed => Err(missing_server_completion(operation)),
        ThreadJoinOutcome::Panicked => Err(SshFixtureError::WorkerPanicked),
        ThreadJoinOutcome::Deferred => Err(SshFixtureError::TimedOut {
            operation,
            deadline: deadline.budget(),
        }),
    }
}

fn missing_server_completion(operation: &'static str) -> SshFixtureError {
    SshFixtureError::Io {
        operation,
        source: io::Error::new(
            io::ErrorKind::BrokenPipe,
            "worker exited without completion",
        ),
    }
}

struct ReapOnDrop {
    future: Option<ReapFuture>,
}

impl ReapOnDrop {
    fn new(future: ReapFuture) -> Self {
        Self {
            future: Some(future),
        }
    }

    async fn wait_until(&mut self, deadline: ShutdownDeadline) -> bool {
        let Some(future) = self.future.as_mut() else {
            return true;
        };
        if deadline.timeout(future).await.is_ok() {
            self.future.take();
            true
        } else {
            false
        }
    }

    async fn wait(&mut self) {
        if let Some(future) = self.future.as_mut() {
            future.await;
            self.future.take();
        }
    }
}

impl Drop for ReapOnDrop {
    fn drop(&mut self) {
        if let Some(future) = self.future.take() {
            let _ = defer_future(future);
        }
    }
}

#[derive(Clone)]
struct ChildTaskTracker {
    state: Arc<Mutex<ChildTaskState>>,
    active: Arc<AtomicUsize>,
}

struct ChildTaskState {
    closed: bool,
    tasks: Vec<OwnedTask>,
}

impl ChildTaskTracker {
    fn new(probe: &SshTaskProbe) -> Self {
        Self {
            state: Arc::new(Mutex::new(ChildTaskState {
                closed: false,
                tasks: Vec::new(),
            })),
            active: Arc::clone(&probe.active),
        }
    }

    fn spawn<F>(&self, future: F) -> Option<tokio::task::AbortHandle>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closed {
            return None;
        }
        self.active.fetch_add(1, Ordering::AcqRel);
        let active = Arc::clone(&self.active);
        let guard = ActiveTaskGuard(active);
        let join = tokio::spawn(async move {
            let _guard = guard;
            future.await;
        });
        let abort = join.abort_handle();
        state.tasks.push(OwnedTask::from_join(join));
        Some(abort)
    }

    async fn shutdown(&self, deadline: ShutdownDeadline) -> bool {
        let tasks = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.closed = true;
            std::mem::take(&mut state.tasks)
        };
        let mut pending = drain_owned_tasks(tasks, deadline.abort_at()).await;
        for task in &pending {
            task.abort();
        }
        pending = drain_owned_tasks(pending, deadline.at()).await;
        let completed = pending.is_empty();
        for task in pending {
            task.defer();
        }
        completed
    }
}

async fn drain_owned_tasks(tasks: Vec<OwnedTask>, until: Instant) -> Vec<OwnedTask> {
    let mut pending = Vec::new();
    for mut task in tasks {
        if !task.wait_until(until).await {
            pending.push(task);
        }
    }
    pending
}

struct ActiveTaskGuard(Arc<AtomicUsize>);

impl Drop for ActiveTaskGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
struct NeverFinishChild {
    drop_delay: Duration,
}

#[cfg(test)]
impl Future for NeverFinishChild {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::task::Poll::Pending
    }
}

#[cfg(test)]
impl Drop for NeverFinishChild {
    fn drop(&mut self) {
        thread::sleep(self.drop_delay);
    }
}

mod forwarding;
mod handler;

use handler::FixtureHandler;

fn record(events: &Mutex<Vec<SshEvent>>, event: SshEvent) {
    lock_events(events).push(event);
}

fn lock_events(events: &Mutex<Vec<SshEvent>>) -> std::sync::MutexGuard<'_, Vec<SshEvent>> {
    events
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
