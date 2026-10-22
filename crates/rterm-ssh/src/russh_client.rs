use std::{
    collections::VecDeque,
    future::Future,
    io::{Read, Seek, SeekFrom, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering},
    },
    time::Duration,
};

use rterm_types::TerminalSize;
use tokio::io::AsyncWriteExt;

use crate::{
    AsyncHostKeyVerifier, HostKeyChallenge, HostKeyStatus, HostKeyVerifier, REDACTED_SECRET,
    SecretPrompt, SecretProvider, SshAuthMethod, SshChannel, SshChannelOpenPlan, SshChannelOpener,
    SshConnectRequest, SshConnectionPhase, SshExitSignal, SshSessionError, SshSessionResult,
    SshSessionStartup, SshShellReader, SshShellWriter,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RusshHostKeyPolicy {
    RejectUnknown,
    /// Ask the configured [`HostKeyVerifier`] for unknown keys and report
    /// changed keys for display. Changed keys remain unconditionally rejected.
    Prompt,
    TrustOnFirstUse,
    AcceptUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RusshKnownHosts {
    path: PathBuf,
}

impl RusshKnownHosts {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Checks whether `key` matches the recorded host key for `host:port`.
    ///
    /// # Errors
    ///
    /// Returns russh key errors when the known-hosts file cannot be parsed or
    /// contains a changed key for the same host and algorithm.
    pub fn matches(
        &self,
        host: &str,
        port: u16,
        key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, russh::keys::Error> {
        russh::keys::known_hosts::check_known_hosts_path(host, port, key, &self.path)
    }

    /// Records `key` for `host:port` in this known-hosts file.
    ///
    /// # Errors
    ///
    /// Returns russh key errors when the file cannot be created or written.
    pub fn learn(
        &self,
        host: &str,
        port: u16,
        key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<(), russh::keys::Error> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&self.path)?;
        let mut needs_newline = false;
        if file.seek(SeekFrom::End(-1)).is_ok() {
            let mut last_byte = [0; 1];
            file.read_exact(&mut last_byte)?;
            needs_newline = last_byte[0] != b'\n';
        }

        file.seek(SeekFrom::End(0))?;
        if needs_newline {
            file.write_all(b"\n")?;
        }

        if port == 22 {
            write!(file, "{host} ")?;
        } else {
            write!(file, "[{host}]:{port} ")?;
        }
        file.write_all(key.to_openssh()?.as_bytes())?;
        file.write_all(b"\n")?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct RusshPrivateKeyAuth {
    key: Arc<russh::keys::PrivateKey>,
}

impl RusshPrivateKeyAuth {
    /// Loads an OpenSSH-compatible private key from disk.
    ///
    /// # Errors
    ///
    /// Returns russh key errors when the key file cannot be read, decoded, or
    /// decrypted with the supplied passphrase.
    pub fn load(
        path: impl AsRef<std::path::Path>,
        passphrase: Option<&str>,
    ) -> Result<Self, russh::keys::Error> {
        let key = russh::keys::load_secret_key(path, passphrase)?;
        Ok(Self { key: Arc::new(key) })
    }

    /// Returns whether a private key requires a passphrase to decrypt.
    ///
    /// # Errors
    ///
    /// Returns russh key errors when the key file cannot be read or decoded.
    pub fn needs_passphrase(path: impl AsRef<std::path::Path>) -> Result<bool, russh::keys::Error> {
        match Self::load(path, None) {
            Ok(_) => Ok(false),
            Err(russh::keys::Error::KeyIsEncrypted) => Ok(true),
            Err(error) => Err(error),
        }
    }

    #[must_use]
    pub fn algorithm(&self) -> russh::keys::ssh_key::Algorithm {
        self.key.algorithm()
    }

    #[must_use]
    fn into_private_key_with_hash_alg(
        self,
        hash_alg: Option<russh::keys::HashAlg>,
    ) -> russh::keys::PrivateKeyWithHashAlg {
        russh::keys::PrivateKeyWithHashAlg::new(self.key, hash_alg)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum RusshAuthRequest {
    Password {
        password: String,
    },
    PasswordPrompt,
    PrivateKey {
        path: PathBuf,
        passphrase: Option<String>,
    },
    Agent,
}

impl std::fmt::Debug for RusshAuthRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Password { .. } => formatter
                .debug_struct("Password")
                .field("password", &REDACTED_SECRET)
                .finish(),
            Self::PasswordPrompt => formatter.write_str("PasswordPrompt"),
            Self::PrivateKey { path, passphrase } => formatter
                .debug_struct("PrivateKey")
                .field("path", path)
                .field("passphrase", &passphrase.as_ref().map(|_| REDACTED_SECRET))
                .finish(),
            Self::Agent => formatter.write_str("Agent"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RusshAuthOutcome {
    Authenticated,
}

impl RusshAuthOutcome {
    /// Converts russh's authentication result into the crate-local session
    /// contract used by the native SSH adapter.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the server rejects the attempted
    /// authentication method.
    pub fn from_auth_result(
        auth_result: &russh::client::AuthResult,
    ) -> Result<Self, SshSessionError> {
        if auth_result.success() {
            return Ok(Self::Authenticated);
        }

        Err(SshSessionError::new("SSH authentication failed"))
    }
}

type RusshAuthFuture<'a> =
    Pin<Box<dyn Future<Output = Result<russh::client::AuthResult, SshSessionError>> + 'a>>;

trait RusshAuthenticationBackend {
    fn authenticate_password<'a>(
        &'a mut self,
        username: &'a str,
        password: &'a str,
    ) -> RusshAuthFuture<'a>;

    fn authenticate_private_key<'a>(
        &'a mut self,
        username: &'a str,
        path: &'a Path,
        passphrase: Option<&'a str>,
    ) -> RusshAuthFuture<'a>;

    fn authenticate_agent<'a>(&'a mut self, username: &'a str) -> RusshAuthFuture<'a>;
}

async fn authenticate_auth_plan_with_backend(
    backend: &mut impl RusshAuthenticationBackend,
    auth_plan: &RusshAuthPlan,
    secret_provider: Option<&SecretProvider>,
) -> Result<RusshAuthOutcome, SshSessionError> {
    let result = match auth_plan.request() {
        RusshAuthRequest::Password { password } => {
            backend
                .authenticate_password(auth_plan.username(), password)
                .await?
        }
        RusshAuthRequest::PasswordPrompt => {
            let Some(provider) = secret_provider else {
                return Err(SshSessionError::new(
                    "SSH password prompt requires a secret provider",
                ));
            };
            let Some(password) = provider
                .prompt(SecretPrompt::password(auth_plan.username().to_owned()))
                .await
            else {
                return Err(SshSessionError::new("SSH password prompt was cancelled"));
            };
            let result = backend
                .authenticate_password(auth_plan.username(), &password)
                .await?;
            drop(password);
            result
        }
        RusshAuthRequest::PrivateKey { path, passphrase } => {
            let prompted_passphrase = if passphrase.is_none()
                && RusshPrivateKeyAuth::needs_passphrase(path).map_err(|error| {
                    SshSessionError::new(format!("SSH private-key inspection failed: {error}"))
                })? {
                let Some(provider) = secret_provider else {
                    return Err(SshSessionError::new(
                        "encrypted SSH private key requires a secret provider",
                    ));
                };
                let Some(secret) = provider
                    .prompt(SecretPrompt::private_key_passphrase(
                        auth_plan.username().to_owned(),
                    ))
                    .await
                else {
                    return Err(SshSessionError::new(
                        "SSH private-key passphrase prompt was cancelled",
                    ));
                };
                Some(secret)
            } else {
                None
            };
            backend
                .authenticate_private_key(
                    auth_plan.username(),
                    path,
                    prompted_passphrase.as_deref().or(passphrase.as_deref()),
                )
                .await?
        }
        RusshAuthRequest::Agent => backend.authenticate_agent(auth_plan.username()).await?,
    };

    RusshAuthOutcome::from_auth_result(&result)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RusshAuthPlan {
    username: String,
    request: RusshAuthRequest,
}

impl RusshAuthPlan {
    #[must_use]
    pub fn from_request(request: &SshConnectRequest) -> Self {
        let auth_request = match &request.auth {
            SshAuthMethod::Password { password } => RusshAuthRequest::Password {
                password: password.clone(),
            },
            SshAuthMethod::PasswordPrompt => RusshAuthRequest::PasswordPrompt,
            SshAuthMethod::PrivateKey { path, passphrase } => RusshAuthRequest::PrivateKey {
                path: path.clone(),
                passphrase: passphrase.clone(),
            },
            SshAuthMethod::Agent => RusshAuthRequest::Agent,
        };

        Self {
            username: request.config.username.clone(),
            request: auth_request,
        }
    }

    #[must_use]
    pub fn username(&self) -> &str {
        &self.username
    }

    #[must_use]
    pub const fn request(&self) -> &RusshAuthRequest {
        &self.request
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RusshChannelStartupRequest {
    RequestPty {
        term: String,
        columns: u32,
        rows: u32,
        pixel_width: u32,
        pixel_height: u32,
    },
    RequestShell,
    Exec {
        command: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RusshChannelStartupPlan {
    requests: Vec<RusshChannelStartupRequest>,
}

impl RusshChannelStartupPlan {
    #[must_use]
    pub fn from_open_plan(open_plan: &SshChannelOpenPlan) -> Self {
        let mut requests = Vec::new();

        if let Some(size) = open_plan.pty_size {
            requests.push(RusshChannelStartupRequest::RequestPty {
                term: "xterm-256color".to_owned(),
                columns: u32::from(size.columns),
                rows: u32::from(size.rows),
                pixel_width: 0,
                pixel_height: 0,
            });
        }

        match &open_plan.startup {
            SshSessionStartup::Shell => {
                requests.push(RusshChannelStartupRequest::RequestShell);
            }
            SshSessionStartup::Command(command) => {
                requests.push(RusshChannelStartupRequest::Exec {
                    command: encode_posix_remote_command(command),
                });
            }
            SshSessionStartup::NoShell => {}
        }

        Self { requests }
    }

    #[must_use]
    pub fn requests(&self) -> &[RusshChannelStartupRequest] {
        &self.requests
    }
}

fn encode_posix_remote_command(arguments: &[String]) -> String {
    arguments
        .iter()
        .map(|argument| {
            let mut quoted = String::with_capacity(argument.len() + 2);
            quoted.push('\'');
            for character in argument.chars() {
                if character == '\'' {
                    quoted.push_str("'\"'\"'");
                } else {
                    quoted.push(character);
                }
            }
            quoted.push('\'');
            quoted
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RusshDirectTcpIpOpenPlan {
    target_host: String,
    target_port: u16,
    originator_host: String,
    originator_port: u16,
}

impl RusshDirectTcpIpOpenPlan {
    #[must_use]
    pub fn new(
        target_host: impl Into<String>,
        target_port: u16,
        originator_host: impl Into<String>,
        originator_port: u16,
    ) -> Self {
        Self {
            target_host: target_host.into(),
            target_port,
            originator_host: originator_host.into(),
            originator_port,
        }
    }

    #[must_use]
    pub fn target(&self) -> (&str, u16) {
        (&self.target_host, self.target_port)
    }

    #[must_use]
    pub fn originator(&self) -> (&str, u16) {
        (&self.originator_host, self.originator_port)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RusshRemoteTcpIpForwardPlan {
    bind_host: String,
    bind_port: u16,
    target_host: String,
    target_port: u16,
}

#[derive(Debug, Clone, Default)]
pub struct RusshForwardCancellation {
    state: Arc<RusshForwardCancellationState>,
}

#[derive(Debug, Default)]
struct RusshForwardCancellationState {
    cancelled: AtomicBool,
    wake: tokio::sync::Notify,
}

impl RusshForwardCancellation {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.state.cancelled.store(true, Ordering::Release);
        self.state.wake.notify_waiters();
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.state.cancelled.load(Ordering::Acquire)
    }

    async fn cancelled(&self) {
        loop {
            if self.is_cancelled() {
                return;
            }
            let notified = self.state.wake.notified();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}

/// One-shot cancellation handle for a native SSH connection attempt.
///
/// Clones observe the same state. Cancelling an opener only affects the
/// connect/authenticate/open/startup operation carrying this handle; openers
/// without a handle retain the existing timeout-only behavior.
#[derive(Debug, Clone, Default)]
pub struct RusshConnectionCancellation {
    state: Arc<RusshConnectionCancellationState>,
}

#[derive(Debug, Default)]
struct RusshConnectionCancellationState {
    cancelled: AtomicBool,
    wake: tokio::sync::Notify,
}

impl RusshConnectionCancellation {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.state.cancelled.store(true, Ordering::Release);
        self.state.wake.notify_waiters();
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.state.cancelled.load(Ordering::Acquire)
    }

    async fn cancelled(&self) {
        loop {
            if self.is_cancelled() {
                return;
            }
            let notified = self.state.wake.notified();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RusshForwardDeadlines {
    startup: Duration,
    shutdown: Duration,
}

impl RusshForwardDeadlines {
    #[must_use]
    pub const fn new(startup: Duration, shutdown: Duration) -> Self {
        Self { startup, shutdown }
    }
}

#[derive(Debug, Default)]
struct RemoteForwardTaskTracker {
    active: AtomicUsize,
    changed: tokio::sync::Notify,
}

impl RemoteForwardTaskTracker {
    fn register(self: &Arc<Self>) -> RemoteForwardTaskGuard {
        self.active.fetch_add(1, Ordering::AcqRel);
        RemoteForwardTaskGuard {
            tracker: Arc::clone(self),
        }
    }

    fn active(&self) -> usize {
        self.active.load(Ordering::Acquire)
    }

    async fn wait_for_empty(&self) {
        loop {
            if self.active() == 0 {
                return;
            }
            let changed = self.changed.notified();
            if self.active() == 0 {
                return;
            }
            changed.await;
        }
    }
}

struct RemoteForwardTaskGuard {
    tracker: Arc<RemoteForwardTaskTracker>,
}

impl Drop for RemoteForwardTaskGuard {
    fn drop(&mut self) {
        if self.tracker.active.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.tracker.changed.notify_waiters();
        }
    }
}

impl RusshRemoteTcpIpForwardPlan {
    #[must_use]
    pub fn new(
        bind_host: impl Into<String>,
        bind_port: u16,
        target_host: impl Into<String>,
        target_port: u16,
    ) -> Self {
        Self {
            bind_host: bind_host.into(),
            bind_port,
            target_host: target_host.into(),
            target_port,
        }
    }

    #[must_use]
    pub fn bind(&self) -> (&str, u16) {
        (&self.bind_host, self.bind_port)
    }

    #[must_use]
    pub fn target(&self) -> (&str, u16) {
        (&self.target_host, self.target_port)
    }
}

#[derive(Debug, Clone)]
struct ResolvedRemoteForward {
    plan: RusshRemoteTcpIpForwardPlan,
    resolved_bind_port: Arc<AtomicU16>,
}

impl ResolvedRemoteForward {
    fn new(plan: RusshRemoteTcpIpForwardPlan) -> Self {
        let configured_port = plan.bind_port;
        Self {
            plan,
            resolved_bind_port: Arc::new(AtomicU16::new(configured_port)),
        }
    }

    fn resolve_bind_port(&self, bind_port: u16) {
        self.resolved_bind_port.store(bind_port, Ordering::Release);
    }

    fn matches_connected_endpoint(&self, connected_address: &str, connected_port: u32) -> bool {
        let resolved_port = self.resolved_bind_port.load(Ordering::Acquire);
        resolved_port != 0
            && resolved_port == u16::try_from(connected_port).unwrap_or_default()
            && (self.plan.bind_host == connected_address
                || self.plan.bind_host == "0.0.0.0"
                || self.plan.bind_host == "::")
    }

    fn target(&self) -> (&str, u16) {
        self.plan.target()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RusshConnectPlan {
    host: String,
    port: u16,
    username: String,
    auth_plan: RusshAuthPlan,
    channel_open_plan: SshChannelOpenPlan,
}

impl RusshConnectPlan {
    #[must_use]
    pub fn from_request(request: &SshConnectRequest) -> Self {
        Self {
            host: request.config.host.clone(),
            port: request.config.port,
            username: request.config.username.clone(),
            auth_plan: RusshAuthPlan::from_request(request),
            channel_open_plan: SshChannelOpenPlan::from_request(request),
        }
    }

    #[must_use]
    pub fn socket_addr(&self) -> (&str, u16) {
        (&self.host, self.port)
    }

    #[must_use]
    pub fn username(&self) -> &str {
        &self.username
    }

    #[must_use]
    pub const fn channel_open_plan(&self) -> &SshChannelOpenPlan {
        &self.channel_open_plan
    }

    #[must_use]
    pub const fn auth_plan(&self) -> &RusshAuthPlan {
        &self.auth_plan
    }

    #[must_use]
    pub fn channel_startup_plan(&self) -> RusshChannelStartupPlan {
        RusshChannelStartupPlan::from_open_plan(&self.channel_open_plan)
    }
}

/// Bounded worker configuration for the application-owned native SSH runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RusshRuntimeConfig {
    worker_threads: usize,
    max_blocking_threads: usize,
}

impl RusshRuntimeConfig {
    #[must_use]
    pub const fn production() -> Self {
        Self {
            worker_threads: 2,
            max_blocking_threads: 8,
        }
    }

    #[must_use]
    pub const fn worker_threads(self) -> usize {
        self.worker_threads
    }

    #[must_use]
    pub const fn max_blocking_threads(self) -> usize {
        self.max_blocking_threads
    }
}

/// Cloneable access to a Tokio runtime shared by native SSH connections.
#[derive(Clone)]
pub struct RusshRuntimeHandle {
    runtime: Arc<tokio::runtime::Runtime>,
    config: RusshRuntimeConfig,
}

impl std::fmt::Debug for RusshRuntimeHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RusshRuntimeHandle")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl RusshRuntimeHandle {
    fn build_production() -> Result<Self, SshSessionError> {
        let config = RusshRuntimeConfig::production();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(config.worker_threads())
            .max_blocking_threads(config.max_blocking_threads())
            .thread_name("rterm-ssh-io")
            .enable_io()
            .enable_time()
            .build()
            .map_err(|error| {
                SshSessionError::new(format!("SSH async runtime creation failed: {error}"))
            })?;
        Ok(Self {
            runtime: Arc::new(runtime),
            config,
        })
    }

    #[must_use]
    pub const fn config(&self) -> RusshRuntimeConfig {
        self.config
    }

    #[must_use]
    pub fn shares_runtime_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.runtime, &other.runtime)
    }
}

/// Lazily creates exactly one bounded runtime, even under concurrent access.
#[derive(Debug, Default)]
pub struct LazyRusshRuntime {
    runtime: std::sync::OnceLock<RusshRuntimeHandle>,
    initialization: std::sync::Mutex<()>,
}

impl LazyRusshRuntime {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            runtime: std::sync::OnceLock::new(),
            initialization: std::sync::Mutex::new(()),
        }
    }

    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.runtime.get().is_some()
    }

    /// Returns the shared runtime, constructing it on the calling thread only
    /// when the first native SSH connection asks for it.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the initialization lock is poisoned or
    /// Tokio cannot construct the bounded runtime.
    pub fn get_or_try_init(&self) -> Result<RusshRuntimeHandle, SshSessionError> {
        if let Some(runtime) = self.runtime.get() {
            return Ok(runtime.clone());
        }
        let _initialization = self.initialization.lock().map_err(|_| {
            SshSessionError::new("SSH async runtime initialization lock is poisoned")
        })?;
        if let Some(runtime) = self.runtime.get() {
            return Ok(runtime.clone());
        }
        let runtime = RusshRuntimeHandle::build_production()?;
        self.runtime
            .set(runtime.clone())
            .map_err(|_| SshSessionError::new("SSH async runtime initialized concurrently"))?;
        Ok(runtime)
    }
}

#[derive(Clone)]
pub struct RusshChannelOpener {
    client_config: Arc<russh::client::Config>,
    runtime_handle: Option<RusshRuntimeHandle>,
    host_key_policy: RusshHostKeyPolicy,
    known_hosts_path: Option<PathBuf>,
    host_key_verifier: Option<HostKeyVerifier>,
    secret_provider: Option<SecretProvider>,
    phase_reporter: Option<ConnectionPhaseReporter>,
    connection_cancellation: Option<RusshConnectionCancellation>,
    operation_timeout: Duration,
    channel_inactivity_timeout: Option<Duration>,
}

#[derive(Clone)]
struct ConnectionPhaseReporter {
    callback: Arc<dyn Fn(SshConnectionPhase) + Send + Sync>,
}

impl std::fmt::Debug for ConnectionPhaseReporter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ConnectionPhaseReporter(..)")
    }
}

impl std::fmt::Debug for RusshChannelOpener {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RusshChannelOpener")
            .field("runtime_handle", &self.runtime_handle)
            .field("host_key_policy", &self.host_key_policy)
            .field("known_hosts_path", &self.known_hosts_path)
            .field("host_key_verifier", &self.host_key_verifier)
            .field("secret_provider", &self.secret_provider)
            .field("phase_reporter", &self.phase_reporter)
            .field("connection_cancellation", &self.connection_cancellation)
            .field("operation_timeout", &self.operation_timeout)
            .field(
                "channel_inactivity_timeout",
                &self.channel_inactivity_timeout,
            )
            .finish_non_exhaustive()
    }
}

const DEFAULT_SSH_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

impl Default for RusshChannelOpener {
    fn default() -> Self {
        let client_config = russh::client::Config {
            keepalive_interval: Some(Duration::from_secs(30)),
            ..Default::default()
        };

        Self {
            client_config: Arc::new(client_config),
            runtime_handle: None,
            host_key_policy: RusshHostKeyPolicy::RejectUnknown,
            known_hosts_path: None,
            host_key_verifier: None,
            secret_provider: None,
            phase_reporter: None,
            connection_cancellation: None,
            operation_timeout: DEFAULT_SSH_OPERATION_TIMEOUT,
            channel_inactivity_timeout: None,
        }
    }
}

impl RusshChannelOpener {
    /// Reuses an application-owned asynchronous runtime for connections opened
    /// by this value instead of constructing a runtime per connection.
    #[must_use]
    pub fn with_runtime_handle(mut self, runtime_handle: RusshRuntimeHandle) -> Self {
        self.runtime_handle = Some(runtime_handle);
        self
    }

    #[must_use]
    pub const fn runtime_handle(&self) -> Option<&RusshRuntimeHandle> {
        self.runtime_handle.as_ref()
    }

    fn runtime_handle_or_create(&self) -> Result<RusshRuntimeHandle, SshSessionError> {
        self.runtime_handle
            .clone()
            .map_or_else(RusshRuntimeHandle::build_production, Ok)
    }

    #[must_use]
    pub fn new(client_config: russh::client::Config) -> Self {
        Self {
            client_config: Arc::new(client_config),
            runtime_handle: None,
            host_key_policy: RusshHostKeyPolicy::RejectUnknown,
            known_hosts_path: None,
            host_key_verifier: None,
            secret_provider: None,
            phase_reporter: None,
            connection_cancellation: None,
            operation_timeout: DEFAULT_SSH_OPERATION_TIMEOUT,
            channel_inactivity_timeout: None,
        }
    }

    #[must_use]
    pub const fn with_host_key_policy(mut self, host_key_policy: RusshHostKeyPolicy) -> Self {
        self.host_key_policy = host_key_policy;
        self
    }

    /// Installs an asynchronous host-key verifier used by
    /// [`RusshHostKeyPolicy::Prompt`]. Known keys still bypass the verifier;
    /// changed keys are reported to it for display but remain unconditionally
    /// rejected regardless of the returned decision.
    #[must_use]
    pub fn with_host_key_verifier<V>(mut self, verifier: V) -> Self
    where
        V: AsyncHostKeyVerifier + 'static,
    {
        self.host_key_verifier = Some(HostKeyVerifier::new(verifier));
        self
    }

    /// Installs a previously constructed cloneable verifier.
    #[must_use]
    pub fn with_host_key_verifier_handle(mut self, verifier: HostKeyVerifier) -> Self {
        self.host_key_verifier = Some(verifier);
        self
    }

    /// Installs an asynchronous provider for password and encrypted-key
    /// passphrase prompts. The provider is invoked only when the request uses
    /// a prompt auth method; plaintext secrets are never retained by the
    /// opener.
    #[must_use]
    pub fn with_secret_provider<V>(mut self, provider: V) -> Self
    where
        V: crate::AsyncSecretProvider + 'static,
    {
        self.secret_provider = Some(SecretProvider::new(provider));
        self
    }

    #[must_use]
    pub fn with_secret_provider_handle(mut self, provider: SecretProvider) -> Self {
        self.secret_provider = Some(provider);
        self
    }

    /// Installs a callback that receives connection milestones. The callback
    /// runs on the async opener task and must remain non-blocking.
    #[must_use]
    pub fn with_phase_reporter<F>(mut self, reporter: F) -> Self
    where
        F: Fn(SshConnectionPhase) + Send + Sync + 'static,
    {
        self.phase_reporter = Some(ConnectionPhaseReporter {
            callback: Arc::new(reporter),
        });
        self
    }

    /// Installs a one-shot cancellation handle for the complete connection
    /// operation, including connect, authentication, channel open, and
    /// channel startup.
    #[must_use]
    pub fn with_connection_cancellation(
        mut self,
        cancellation: RusshConnectionCancellation,
    ) -> Self {
        self.connection_cancellation = Some(cancellation);
        self
    }

    /// Reports a milestone through the configured callback. This is public so
    /// adapters that compose multiple startup operations can share the same
    /// lifecycle stream without initiating a network operation.
    pub fn report_phase(&self, phase: SshConnectionPhase) {
        if let Some(reporter) = &self.phase_reporter {
            (reporter.callback)(phase);
        }
    }

    #[must_use]
    pub fn with_known_hosts_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.known_hosts_path = Some(path.into());
        self
    }

    /// Sets the absolute deadline for opening one SSH operation, including
    /// connect, authentication, channel open, and channel startup requests.
    #[must_use]
    pub const fn with_operation_timeout(mut self, timeout: Duration) -> Self {
        self.operation_timeout = timeout;
        self
    }

    /// Sets the maximum time a channel reader may wait without receiving any
    /// SSH channel message.
    #[must_use]
    pub const fn with_channel_inactivity_timeout(mut self, timeout: Duration) -> Self {
        self.channel_inactivity_timeout = Some(timeout);
        self
    }

    /// Returns the opt-in channel inactivity timeout. `None` leaves legitimate
    /// quiet sessions unbounded by an inactivity policy.
    #[must_use]
    pub const fn channel_inactivity_timeout(&self) -> Option<Duration> {
        self.channel_inactivity_timeout
    }

    #[must_use]
    pub fn client_config(&self) -> &russh::client::Config {
        self.client_config.as_ref()
    }

    #[must_use]
    pub const fn host_key_policy(&self) -> RusshHostKeyPolicy {
        self.host_key_policy
    }

    #[must_use]
    pub fn known_hosts_path(&self) -> Option<&std::path::Path> {
        self.known_hosts_path.as_deref()
    }

    #[must_use]
    pub fn host_key_verifier(&self) -> Option<&HostKeyVerifier> {
        self.host_key_verifier.as_ref()
    }

    #[must_use]
    pub fn secret_provider(&self) -> Option<&SecretProvider> {
        self.secret_provider.as_ref()
    }

    #[must_use]
    pub fn into_handler(self) -> RusshClientHandler {
        RusshClientHandler {
            host_key_policy: self.host_key_policy,
            host: None,
            port: None,
            known_hosts_path: self.known_hosts_path,
            host_key_verifier: self.host_key_verifier,
            remote_forwards: Vec::new(),
            forward_cancellation: None,
            forward_task_tracker: None,
            forward_deadlines: None,
        }
    }

    #[must_use]
    pub fn handler(&self) -> RusshClientHandler {
        RusshClientHandler {
            host_key_policy: self.host_key_policy,
            host: None,
            port: None,
            known_hosts_path: self.known_hosts_path.clone(),
            host_key_verifier: self.host_key_verifier.clone(),
            remote_forwards: Vec::new(),
            forward_cancellation: None,
            forward_task_tracker: None,
            forward_deadlines: None,
        }
    }

    #[must_use]
    pub fn handler_for_host(&self, host: impl Into<String>, port: u16) -> RusshClientHandler {
        RusshClientHandler {
            host_key_policy: self.host_key_policy,
            host: Some(host.into()),
            port: Some(port),
            known_hosts_path: self.known_hosts_path.clone(),
            host_key_verifier: self.host_key_verifier.clone(),
            remote_forwards: Vec::new(),
            forward_cancellation: None,
            forward_task_tracker: None,
            forward_deadlines: None,
        }
    }

    #[must_use]
    pub fn handler_for_host_with_remote_forward(
        &self,
        host: impl Into<String>,
        port: u16,
        remote_forward: RusshRemoteTcpIpForwardPlan,
    ) -> RusshClientHandler {
        self.handler_for_host_with_remote_forward_lifecycle(
            host,
            port,
            ResolvedRemoteForward::new(remote_forward),
            RusshForwardCancellation::new(),
            Arc::new(RemoteForwardTaskTracker::default()),
            RusshForwardDeadlines::new(Duration::from_secs(30), Duration::from_secs(1)),
        )
    }

    #[must_use]
    fn handler_for_host_with_remote_forward_lifecycle(
        &self,
        host: impl Into<String>,
        port: u16,
        remote_forward: ResolvedRemoteForward,
        forward_cancellation: RusshForwardCancellation,
        forward_task_tracker: Arc<RemoteForwardTaskTracker>,
        forward_deadlines: RusshForwardDeadlines,
    ) -> RusshClientHandler {
        RusshClientHandler {
            host_key_policy: self.host_key_policy,
            host: Some(host.into()),
            port: Some(port),
            known_hosts_path: self.known_hosts_path.clone(),
            host_key_verifier: self.host_key_verifier.clone(),
            remote_forwards: vec![remote_forward],
            forward_cancellation: Some(forward_cancellation),
            forward_task_tracker: Some(forward_task_tracker),
            forward_deadlines: Some(forward_deadlines),
        }
    }

    #[must_use]
    pub fn connect_plan(&self, request: &SshConnectRequest) -> RusshConnectPlan {
        RusshConnectPlan::from_request(request)
    }

    /// Connects the underlying russh transport and returns a connected session
    /// handle. Authentication and channel opening are layered on top of this
    /// transport entry point.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the TCP connection or SSH handshake
    /// fails.
    pub async fn connect_async(
        &self,
        request: SshConnectRequest,
    ) -> Result<russh::client::Handle<RusshClientHandler>, SshSessionError> {
        self.report_phase(SshConnectionPhase::Connecting);
        let plan = self.connect_plan(&request);
        let (host, port) = plan.socket_addr();
        tokio::time::timeout(
            self.operation_timeout,
            russh::client::connect(
                Arc::clone(&self.client_config),
                (host, port),
                self.handler_for_host(host, port),
            ),
        )
        .await
        .map_err(|_| self.operation_deadline_error("connect"))?
        .map_err(|error| SshSessionError::new(format!("SSH connect failed: {error}")))
    }

    /// Authenticates a connected russh handle using the planned authentication
    /// branch.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when authentication fails or when the
    /// requested authentication branch is not wired into the native russh
    /// adapter yet.
    pub async fn authenticate_async(
        &self,
        handle: &mut russh::client::Handle<RusshClientHandler>,
        auth_plan: &RusshAuthPlan,
    ) -> Result<RusshAuthOutcome, SshSessionError> {
        self.report_phase(SshConnectionPhase::Authenticating);
        let mut backend = RusshHandleAuthenticationBackend { handle };
        tokio::time::timeout(
            self.operation_timeout,
            authenticate_auth_plan_with_backend(
                &mut backend,
                auth_plan,
                self.secret_provider.as_ref(),
            ),
        )
        .await
        .map_err(|_| self.operation_deadline_error("authentication"))?
    }

    /// Opens a russh session channel on an authenticated handle.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the server rejects or fails the session
    /// channel open request.
    pub async fn open_session_channel_async(
        &self,
        handle: &russh::client::Handle<RusshClientHandler>,
    ) -> Result<russh::Channel<russh::client::Msg>, SshSessionError> {
        self.report_phase(SshConnectionPhase::Opening);
        tokio::time::timeout(self.operation_timeout, handle.channel_open_session())
            .await
            .map_err(|_| self.operation_deadline_error("channel open"))?
            .map_err(|error| SshSessionError::new(format!("SSH channel open failed: {error}")))
    }

    /// Opens a russh direct-tcpip channel on an authenticated handle.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the server rejects or fails the direct
    /// TCP channel open request.
    pub async fn open_direct_tcpip_channel_async(
        &self,
        handle: &russh::client::Handle<RusshClientHandler>,
        plan: &RusshDirectTcpIpOpenPlan,
    ) -> Result<russh::Channel<russh::client::Msg>, SshSessionError> {
        let (target_host, target_port) = plan.target();
        let (originator_host, originator_port) = plan.originator();

        tokio::time::timeout(
            self.operation_timeout,
            handle.channel_open_direct_tcpip(
                target_host,
                target_port.into(),
                originator_host,
                originator_port.into(),
            ),
        )
        .await
        .map_err(|_| self.operation_deadline_error("direct-tcpip channel open"))?
        .map_err(|error| {
            SshSessionError::new(format!("SSH direct-tcpip channel open failed: {error}"))
        })
    }

    /// Opens an authenticated direct-tcpip channel using the same blocking
    /// adapter style as shell channels.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when runtime creation, SSH connection,
    /// authentication, or direct-tcpip channel opening fails.
    pub fn open_direct_tcpip_channel(
        &mut self,
        request: SshConnectRequest,
        direct_tcpip_plan: &RusshDirectTcpIpOpenPlan,
    ) -> Result<RusshSshChannel, SshSessionError> {
        let plan = self.connect_plan(&request);
        let runtime = self.runtime_handle_or_create()?;

        let operation_timeout = self.operation_timeout;
        let (handle, channel) = runtime.runtime.block_on(async {
            tokio::time::timeout(operation_timeout, async {
                let mut handle = self.connect_async(request).await?;
                self.authenticate_async(&mut handle, plan.auth_plan())
                    .await?;
                let channel = self
                    .open_direct_tcpip_channel_async(&handle, direct_tcpip_plan)
                    .await?;

                Ok::<_, SshSessionError>((handle, channel))
            })
            .await
            .map_err(|_| {
                SshSessionError::new(format!(
                    "SSH direct-tcpip operation deadline exceeded after {operation_timeout:?}"
                ))
            })?
        })?;

        Ok(RusshSshChannel::new_with_inactivity_timeout(
            channel,
            handle,
            Arc::clone(&runtime.runtime),
            self.channel_inactivity_timeout,
        ))
    }

    /// Opens a dedicated direct-tcpip session and bridges it to `local_stream`
    /// until EOF or cancellation. Both establishment and shutdown are bounded.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when establishment, transfer, or bounded
    /// shutdown fails.
    pub fn forward_direct_tcpip_stream(
        &self,
        request: SshConnectRequest,
        direct_tcpip_plan: &RusshDirectTcpIpOpenPlan,
        local_stream: TcpStream,
        cancellation: &RusshForwardCancellation,
        deadlines: RusshForwardDeadlines,
    ) -> Result<(), SshSessionError> {
        self.forward_direct_tcpip_stream_with_ready(
            request,
            direct_tcpip_plan,
            local_stream,
            cancellation,
            deadlines,
            || Ok(()),
        )
    }

    /// Bridges a direct-tcpip stream and invokes `ready` after the SSH channel
    /// is established but before bytes are transferred.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] under the same conditions as
    /// [`Self::forward_direct_tcpip_stream`] or when `ready` fails.
    pub fn forward_direct_tcpip_stream_with_ready<F>(
        &self,
        request: SshConnectRequest,
        direct_tcpip_plan: &RusshDirectTcpIpOpenPlan,
        local_stream: TcpStream,
        cancellation: &RusshForwardCancellation,
        deadlines: RusshForwardDeadlines,
        ready: F,
    ) -> Result<(), SshSessionError>
    where
        F: FnOnce() -> Result<(), SshSessionError>,
    {
        local_stream.set_nonblocking(true).map_err(|error| {
            SshSessionError::new(format!("local forwarding stream setup failed: {error}"))
        })?;
        let runtime = build_direct_forward_runtime()?;
        let plan = self.connect_plan(&request);
        let direct_tcpip_plan = direct_tcpip_plan.clone();

        runtime.block_on(async {
            let (mut handle, channel) =
                run_forward_startup(cancellation, deadlines.startup, async {
                    let mut handle = self.connect_async(request).await?;
                    self.authenticate_async(&mut handle, plan.auth_plan())
                        .await?;
                    let channel = self
                        .open_direct_tcpip_channel_async(&handle, &direct_tcpip_plan)
                        .await?;
                    Ok((handle, channel))
                })
                .await?;
            let mut channel_stream = channel.into_stream();
            let transfer_result = match ready() {
                Err(error) => Err(error),
                Ok(()) => match tokio::net::TcpStream::from_std(local_stream) {
                    Err(error) => Err(SshSessionError::new(format!(
                        "local forwarding stream setup failed: {error}"
                    ))),
                    Ok(mut local_stream) => tokio::select! {
                        biased;
                        () = cancellation.cancelled() => Ok(()),
                        result = tokio::io::copy_bidirectional(&mut local_stream, &mut channel_stream) => {
                            result.map(|_| ()).map_err(|error| {
                                SshSessionError::new(format!("SSH forwarding transfer failed: {error}"))
                            })
                        }
                    },
                }
            };

            complete_established_forward(transfer_result, async {
                tokio::time::timeout(deadlines.shutdown, async {
                    let channel_result = channel_stream.shutdown().await.map_err(|error| {
                        SshSessionError::new(format!(
                            "SSH forwarding channel shutdown failed: {error}"
                        ))
                    });
                    drop(channel_stream);
                    let disconnect_result = RemoteForwardSession::disconnect(&mut handle).await;
                    let wait_result = RemoteForwardSession::wait(&mut handle).await;
                    channel_result.and(disconnect_result).and(wait_result)
                })
                .await
                .map_err(|_| SshSessionError::new("SSH forwarding shutdown timed out"))?
            })
            .await
        })
    }

    /// Requests a server-side TCP listener and handles incoming forwarded
    /// channels by connecting them to the requested local target.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when runtime creation, SSH connection,
    /// authentication, or the remote forwarding request fails.
    pub fn start_remote_tcpip_forward(
        &mut self,
        request: &SshConnectRequest,
        remote_forward_plan: &RusshRemoteTcpIpForwardPlan,
    ) -> Result<(), SshSessionError> {
        let mut forward = self.open_remote_tcpip_forward(request, remote_forward_plan)?;
        forward.wait()
    }

    /// Opens a server-side TCP listener and returns an explicit lifecycle
    /// handle after the server confirms the forwarding request.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when runtime creation, SSH connection,
    /// authentication, or the remote forwarding request fails.
    pub fn open_remote_tcpip_forward(
        &mut self,
        request: &SshConnectRequest,
        remote_forward_plan: &RusshRemoteTcpIpForwardPlan,
    ) -> Result<RusshRemoteTcpIpForward, SshSessionError> {
        self.open_remote_tcpip_forward_with_lifecycle(
            request,
            remote_forward_plan,
            &RusshForwardCancellation::new(),
            RusshForwardDeadlines::new(Duration::from_secs(30), Duration::from_secs(1)),
        )
    }

    /// Opens a remote forwarding listener with cancellation and a total
    /// establishment deadline covering connect, authentication, and request.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when establishment fails, is cancelled, or
    /// exceeds `deadlines.startup`.
    pub fn open_remote_tcpip_forward_with_lifecycle(
        &self,
        request: &SshConnectRequest,
        remote_forward_plan: &RusshRemoteTcpIpForwardPlan,
        cancellation: &RusshForwardCancellation,
        deadlines: RusshForwardDeadlines,
    ) -> Result<RusshRemoteTcpIpForward, SshSessionError> {
        let plan = self.connect_plan(request);
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|error| {
                SshSessionError::new(format!("SSH async runtime creation failed: {error}"))
            })?;
        let remote_forward_plan = remote_forward_plan.clone();
        let resolved_remote_forward = ResolvedRemoteForward::new(remote_forward_plan.clone());
        let task_tracker = Arc::new(RemoteForwardTaskTracker::default());

        let (handle, bound_port) = runtime.block_on(run_forward_startup(
            cancellation,
            deadlines.startup,
            async {
                let (host, port) = plan.socket_addr();
                let mut handle = russh::client::connect(
                    Arc::clone(&self.client_config),
                    (host, port),
                    self.handler_for_host_with_remote_forward_lifecycle(
                        host,
                        port,
                        resolved_remote_forward.clone(),
                        cancellation.clone(),
                        Arc::clone(&task_tracker),
                        deadlines,
                    ),
                )
                .await
                .map_err(|error| SshSessionError::new(format!("SSH connect failed: {error}")))?;
                self.authenticate_async(&mut handle, plan.auth_plan())
                    .await?;

                let (bind_host, bind_port) = remote_forward_plan.bind();
                let bound_port = handle
                    .tcpip_forward(bind_host, u32::from(bind_port))
                    .await
                    .map_err(|error| {
                        SshSessionError::new(format!("SSH remote TCP forwarding failed: {error}"))
                    })?;
                let bound_port = resolved_remote_forward_bound_port(bind_port, bound_port)?;
                resolved_remote_forward.resolve_bind_port(bound_port);

                Ok::<_, SshSessionError>((handle, bound_port))
            },
        ))?;

        Ok(RusshRemoteTcpIpForward {
            runtime,
            handle: Some(handle),
            bind_host: remote_forward_plan.bind_host.clone(),
            bind_port: bound_port,
            cancellation: cancellation.clone(),
            task_tracker,
            deadlines,
        })
    }

    /// Sends the planned PTY, shell, or exec requests to an opened russh
    /// session channel.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when a planned channel startup request fails.
    pub async fn start_channel_async(
        &self,
        channel: &russh::Channel<russh::client::Msg>,
        startup_plan: &RusshChannelStartupPlan,
    ) -> Result<(), SshSessionError> {
        tokio::time::timeout(self.operation_timeout, async {
            for request in startup_plan.requests() {
                match request {
                    RusshChannelStartupRequest::RequestPty {
                        term,
                        columns,
                        rows,
                        pixel_width,
                        pixel_height,
                    } => {
                        channel
                            .request_pty(
                                true,
                                term,
                                *columns,
                                *rows,
                                *pixel_width,
                                *pixel_height,
                                &[],
                            )
                            .await
                            .map_err(|error| {
                                SshSessionError::new(format!("SSH PTY request failed: {error}"))
                            })?;
                    }
                    RusshChannelStartupRequest::RequestShell => {
                        channel.request_shell(true).await.map_err(|error| {
                            SshSessionError::new(format!("SSH shell request failed: {error}"))
                        })?;
                    }
                    RusshChannelStartupRequest::Exec { command } => {
                        channel
                            .exec(true, command.as_bytes())
                            .await
                            .map_err(|error| {
                                SshSessionError::new(format!("SSH exec request failed: {error}"))
                            })?;
                    }
                }
            }

            Ok::<(), SshSessionError>(())
        })
        .await
        .map_err(|_| self.operation_deadline_error("channel startup"))?
    }

    fn operation_deadline_error(&self, stage: &str) -> SshSessionError {
        SshSessionError::new(format!(
            "SSH {stage} deadline exceeded after {:?}",
            self.operation_timeout
        ))
    }
}

fn validated_remote_forward_bound_port(bound_port: u32) -> Result<u16, SshSessionError> {
    let bound_port_u16 = u16::try_from(bound_port).map_err(|_| {
        SshSessionError::new(format!(
            "SSH remote TCP forwarding returned invalid port {bound_port}"
        ))
    })?;
    if bound_port_u16 == 0 {
        return Err(SshSessionError::new(
            "SSH remote TCP forwarding returned invalid port 0",
        ));
    }
    Ok(bound_port_u16)
}

fn resolved_remote_forward_bound_port(
    requested_port: u16,
    returned_port: u32,
) -> Result<u16, SshSessionError> {
    if requested_port == 0 {
        validated_remote_forward_bound_port(returned_port)
    } else if returned_port == 0 || returned_port == u32::from(requested_port) {
        Ok(requested_port)
    } else {
        validated_remote_forward_bound_port(returned_port)
    }
}

struct RusshHandleAuthenticationBackend<'a> {
    handle: &'a mut russh::client::Handle<RusshClientHandler>,
}

impl RusshAuthenticationBackend for RusshHandleAuthenticationBackend<'_> {
    fn authenticate_password<'a>(
        &'a mut self,
        username: &'a str,
        password: &'a str,
    ) -> RusshAuthFuture<'a> {
        Box::pin(async move {
            self.handle
                .authenticate_password(username, password)
                .await
                .map_err(|error| {
                    SshSessionError::new(format!("SSH password authentication failed: {error}"))
                })
        })
    }

    fn authenticate_private_key<'a>(
        &'a mut self,
        username: &'a str,
        path: &'a Path,
        passphrase: Option<&'a str>,
    ) -> RusshAuthFuture<'a> {
        Box::pin(async move {
            let key = RusshPrivateKeyAuth::load(path, passphrase).map_err(|error| {
                SshSessionError::new(format!("SSH private-key load failed: {error}"))
            })?;
            let rsa_hash = self
                .handle
                .best_supported_rsa_hash()
                .await
                .map_err(|error| {
                    SshSessionError::new(format!(
                        "SSH private-key algorithm negotiation failed: {error}"
                    ))
                })?
                .flatten();
            self.handle
                .authenticate_publickey(username, key.into_private_key_with_hash_alg(rsa_hash))
                .await
                .map_err(|error| {
                    SshSessionError::new(format!("SSH private-key authentication failed: {error}"))
                })
        })
    }

    fn authenticate_agent<'a>(&'a mut self, username: &'a str) -> RusshAuthFuture<'a> {
        Box::pin(async move { authenticate_agent_with_default_client(self.handle, username).await })
    }
}

type DynamicAgentClient<'a> = russh::keys::agent::client::AgentClient<
    Box<dyn russh::keys::agent::client::AgentStream + Send + Unpin + 'a>,
>;

#[cfg(unix)]
async fn connect_default_agent_client() -> Result<DynamicAgentClient<'static>, russh::keys::Error> {
    russh::keys::agent::client::AgentClient::connect_env()
        .await
        .map(russh::keys::agent::client::AgentClient::dynamic)
}

#[cfg(windows)]
async fn connect_default_agent_client() -> Result<DynamicAgentClient<'static>, russh::keys::Error> {
    if let Ok(path) = std::env::var("SSH_AUTH_SOCK") {
        return russh::keys::agent::client::AgentClient::connect_named_pipe(path)
            .await
            .map(russh::keys::agent::client::AgentClient::dynamic);
    }

    if let Ok(client) = russh::keys::agent::client::AgentClient::connect_pageant().await {
        return Ok(client.dynamic());
    }

    russh::keys::agent::client::AgentClient::connect_named_pipe(r"\\.\pipe\openssh-ssh-agent")
        .await
        .map(russh::keys::agent::client::AgentClient::dynamic)
}

async fn authenticate_agent_with_default_client(
    handle: &mut russh::client::Handle<RusshClientHandler>,
    username: &str,
) -> Result<russh::client::AuthResult, SshSessionError> {
    let mut agent = connect_default_agent_client()
        .await
        .map_err(|error| SshSessionError::new(format!("SSH agent connection failed: {error}")))?;

    authenticate_agent_with_client(handle, username, &mut agent).await
}

async fn authenticate_agent_with_client<S>(
    handle: &mut russh::client::Handle<RusshClientHandler>,
    username: &str,
    agent: &mut russh::keys::agent::client::AgentClient<S>,
) -> Result<russh::client::AuthResult, SshSessionError>
where
    S: russh::keys::agent::client::AgentStream + Send + Unpin,
{
    let identities = agent.request_identities().await.map_err(|error| {
        SshSessionError::new(format!("SSH agent identity lookup failed: {error}"))
    })?;
    if identities.is_empty() {
        return Err(SshSessionError::new("SSH agent has no identities"));
    }

    let rsa_hash = handle
        .best_supported_rsa_hash()
        .await
        .map_err(|error| {
            SshSessionError::new(format!("SSH agent algorithm negotiation failed: {error}"))
        })?
        .flatten();

    for identity in identities {
        let result = match identity {
            russh::keys::agent::AgentIdentity::PublicKey { key, .. } => {
                handle
                    .authenticate_publickey_with(username, key, rsa_hash, agent)
                    .await
            }
            russh::keys::agent::AgentIdentity::Certificate { certificate, .. } => {
                handle
                    .authenticate_certificate_with(username, certificate, rsa_hash, agent)
                    .await
            }
        }
        .map_err(|error| {
            SshSessionError::new(format!("SSH agent authentication failed: {error}"))
        })?;

        if result.success() {
            return Ok(result);
        }
    }

    Ok(russh::client::AuthResult::Failure {
        remaining_methods: russh::MethodSet::empty(),
        partial_success: false,
    })
}

#[derive(Debug, Clone)]
pub struct RusshClientHandler {
    host_key_policy: RusshHostKeyPolicy,
    host: Option<String>,
    port: Option<u16>,
    known_hosts_path: Option<PathBuf>,
    host_key_verifier: Option<HostKeyVerifier>,
    remote_forwards: Vec<ResolvedRemoteForward>,
    forward_cancellation: Option<RusshForwardCancellation>,
    forward_task_tracker: Option<Arc<RemoteForwardTaskTracker>>,
    forward_deadlines: Option<RusshForwardDeadlines>,
}

fn known_hosts_contains_endpoint(path: &Path, host: &str, port: u16) -> bool {
    let endpoint = if port == 22 {
        host.to_owned()
    } else {
        format!("[{host}]:{port}")
    };
    let Ok(contents) = std::fs::read_to_string(path) else {
        return false;
    };

    contents
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .any(|hosts| {
            hosts.split(',').any(|candidate| {
                candidate == endpoint
                    || (port == 22 && candidate == host)
                    || (port == 22 && candidate == format!("[{host}]:22"))
            })
        })
}

impl RusshClientHandler {
    #[must_use]
    pub const fn accepts_unknown_host_keys(&self) -> bool {
        matches!(self.host_key_policy, RusshHostKeyPolicy::AcceptUnknown)
    }
}

impl russh::client::Handler for RusshClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        let (Some(host), Some(port)) = (self.host.as_deref(), self.port) else {
            return Ok(false);
        };

        let (status, known_hosts) = if let Some(path) = &self.known_hosts_path {
            let known_hosts = RusshKnownHosts::new(path.clone());
            let status = match known_hosts.matches(host, port, server_public_key) {
                Ok(true) => HostKeyStatus::Known,
                Ok(false) => {
                    if known_hosts_contains_endpoint(path, host, port) {
                        HostKeyStatus::Changed
                    } else {
                        HostKeyStatus::Unknown
                    }
                }
                Err(russh::keys::Error::KeyChanged { .. }) => HostKeyStatus::Changed,
                Err(_) => return Ok(false),
            };
            (status, Some(known_hosts))
        } else {
            let status =
                match russh::keys::known_hosts::check_known_hosts(host, port, server_public_key) {
                    Ok(true) => HostKeyStatus::Known,
                    Ok(false) => HostKeyStatus::Unknown,
                    Err(russh::keys::Error::KeyChanged { .. }) => HostKeyStatus::Changed,
                    Err(_) => return Ok(false),
                };
            (status, None)
        };

        if status == HostKeyStatus::Known {
            return Ok(true);
        }
        let challenge = || {
            let mut challenge = HostKeyChallenge::new(
                host,
                port,
                server_public_key.algorithm().to_string(),
                server_public_key
                    .fingerprint(russh::keys::HashAlg::Sha256)
                    .to_string(),
                status,
            );
            if let Some(path) = &self.known_hosts_path {
                challenge = challenge.with_known_hosts_path(path.clone());
            }
            challenge
        };
        // A changed key must never be accepted by an interactive prompt or by
        // the permissive unknown-key policy. This protects an existing trust
        // record from being silently replaced. Prompt verifiers still receive
        // the challenge so a GUI can display the changed fingerprint and
        // known-hosts path, but their decision is deliberately ignored.
        if status == HostKeyStatus::Changed {
            if self.host_key_policy == RusshHostKeyPolicy::Prompt
                && let Some(verifier) = &self.host_key_verifier
            {
                let _ = verifier.verify(challenge()).await;
            }
            return Ok(false);
        }

        match self.host_key_policy {
            RusshHostKeyPolicy::RejectUnknown => Ok(false),
            RusshHostKeyPolicy::AcceptUnknown => Ok(true),
            RusshHostKeyPolicy::TrustOnFirstUse => {
                let Some(known_hosts) = known_hosts else {
                    return Ok(false);
                };
                known_hosts
                    .learn(host, port, server_public_key)
                    .map_err(russh::Error::from)?;
                Ok(true)
            }
            RusshHostKeyPolicy::Prompt => {
                let Some(verifier) = &self.host_key_verifier else {
                    return Ok(false);
                };
                let decision = verifier.verify(challenge()).await;
                if !decision.accepts() {
                    return Ok(false);
                }
                if decision.stores() {
                    let Some(known_hosts) = known_hosts else {
                        return Ok(false);
                    };
                    known_hosts
                        .learn(host, port, server_public_key)
                        .map_err(russh::Error::from)?;
                }
                Ok(true)
            }
        }
    }

    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: russh::Channel<russh::client::Msg>,
        connected_address: &str,
        connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        _session: &mut russh::client::Session,
    ) -> Result<(), Self::Error> {
        let Some(forward) = self
            .remote_forwards
            .iter()
            .find(|forward| forward.matches_connected_endpoint(connected_address, connected_port))
            .cloned()
        else {
            channel.close().await?;
            return Ok(());
        };
        let cancellation = self.forward_cancellation.clone().unwrap_or_default();
        let task_tracker = self
            .forward_task_tracker
            .clone()
            .unwrap_or_else(|| Arc::new(RemoteForwardTaskTracker::default()));
        let deadlines = self.forward_deadlines.unwrap_or_else(|| {
            RusshForwardDeadlines::new(Duration::from_secs(30), Duration::from_secs(1))
        });
        let task_guard = task_tracker.register();

        tokio::spawn(async move {
            let _task_guard = task_guard;
            let mut channel_stream = channel.into_stream();
            let (target_host, target_port) = forward.target();
            if let Ok(mut local_stream) =
                run_forward_startup(&cancellation, deadlines.startup, async {
                    tokio::net::TcpStream::connect((target_host, target_port))
                        .await
                        .map_err(|error| {
                            SshSessionError::new(format!(
                                "SSH remote forwarding target connect failed: {error}"
                            ))
                        })
                })
                .await
            {
                tokio::select! {
                    biased;
                    () = cancellation.cancelled() => {}
                    _ = tokio::io::copy_bidirectional(&mut channel_stream, &mut local_stream) => {}
                }
            }
            let _ = tokio::time::timeout(deadlines.shutdown, channel_stream.shutdown()).await;
        });

        Ok(())
    }
}

trait RemoteForwardSession {
    async fn cancel_tcpip_forward(
        &mut self,
        bind_host: &str,
        bind_port: u32,
    ) -> Result<(), SshSessionError>;

    async fn disconnect(&mut self) -> Result<(), SshSessionError>;

    async fn wait(&mut self) -> Result<(), SshSessionError>;

    async fn wait_after_disconnect(&mut self) -> Result<(), SshSessionError> {
        self.wait().await
    }
}

impl RemoteForwardSession for russh::client::Handle<RusshClientHandler> {
    async fn cancel_tcpip_forward(
        &mut self,
        bind_host: &str,
        bind_port: u32,
    ) -> Result<(), SshSessionError> {
        russh::client::Handle::cancel_tcpip_forward(self, bind_host, bind_port)
            .await
            .map_err(|error| {
                SshSessionError::new(format!(
                    "SSH remote forwarding cancellation failed: {error}"
                ))
            })
    }

    async fn disconnect(&mut self) -> Result<(), SshSessionError> {
        russh::client::Handle::disconnect(
            self,
            russh::Disconnect::ByApplication,
            "remote forwarding stopped",
            "en",
        )
        .await
        .map_err(|error| SshSessionError::new(format!("SSH disconnect failed: {error}")))
    }

    async fn wait(&mut self) -> Result<(), SshSessionError> {
        (&mut *self).await.map_err(|error| {
            SshSessionError::new(format!("SSH forwarding session failed: {error}"))
        })
    }

    async fn wait_after_disconnect(&mut self) -> Result<(), SshSessionError> {
        match (&mut *self).await {
            Ok(()) | Err(russh::Error::Disconnect) => Ok(()),
            Err(error) => Err(SshSessionError::new(format!(
                "SSH forwarding session failed: {error}"
            ))),
        }
    }
}

#[cfg(test)]
async fn shutdown_remote_forward_session<S>(
    session: &mut S,
    bind_host: &str,
    bind_port: u32,
    timeout: Duration,
) -> Result<(), SshSessionError>
where
    S: RemoteForwardSession,
{
    shutdown_remote_forward_session_outcome(session, bind_host, bind_port, timeout)
        .await
        .result
}

struct RemoteForwardShutdownOutcome {
    result: Result<(), SshSessionError>,
    session_terminal: bool,
    terminal: bool,
}

#[cfg(test)]
async fn shutdown_remote_forward_session_outcome<S>(
    session: &mut S,
    bind_host: &str,
    bind_port: u32,
    timeout: Duration,
) -> RemoteForwardShutdownOutcome
where
    S: RemoteForwardSession,
{
    let shutdown = async {
        let cancel_result = session.cancel_tcpip_forward(bind_host, bind_port).await;
        let disconnect_result = session.disconnect().await;
        let wait_result = session.wait_after_disconnect().await;

        let result = cancel_result.and(disconnect_result).and(wait_result);
        RemoteForwardShutdownOutcome {
            result,
            session_terminal: true,
            terminal: true,
        }
    };

    tokio::time::timeout(timeout, shutdown)
        .await
        .unwrap_or_else(|_| RemoteForwardShutdownOutcome {
            result: Err(SshSessionError::new(
                "SSH remote forwarding shutdown timed out",
            )),
            session_terminal: false,
            terminal: false,
        })
}

async fn shutdown_remote_forward_session_and_tasks<S>(
    session: &mut S,
    bind_host: &str,
    bind_port: u32,
    task_tracker: &RemoteForwardTaskTracker,
    timeout: Duration,
) -> RemoteForwardShutdownOutcome
where
    S: RemoteForwardSession,
{
    let session_terminal = Arc::new(AtomicBool::new(false));
    let completed_session = Arc::clone(&session_terminal);
    let shutdown = async {
        let cancel_result = session.cancel_tcpip_forward(bind_host, bind_port).await;
        let disconnect_result = session.disconnect().await;
        let wait_result = session.wait_after_disconnect().await;
        completed_session.store(true, Ordering::Release);
        task_tracker.wait_for_empty().await;

        RemoteForwardShutdownOutcome {
            result: cancel_result.and(disconnect_result).and(wait_result),
            session_terminal: true,
            terminal: true,
        }
    };

    tokio::time::timeout(timeout, shutdown)
        .await
        .unwrap_or_else(|_| RemoteForwardShutdownOutcome {
            result: Err(SshSessionError::new(
                "SSH remote forwarding shutdown timed out",
            )),
            session_terminal: session_terminal.load(Ordering::Acquire),
            terminal: false,
        })
}

async fn run_forward_startup<T, F>(
    cancellation: &RusshForwardCancellation,
    timeout: Duration,
    startup: F,
) -> Result<T, SshSessionError>
where
    F: std::future::Future<Output = Result<T, SshSessionError>>,
{
    tokio::time::timeout(timeout, async {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => {
                Err(SshSessionError::new("SSH forwarding startup cancelled"))
            }
            result = startup => result,
        }
    })
    .await
    .map_err(|_| SshSessionError::new("SSH forwarding startup timed out"))?
}

fn build_direct_forward_runtime() -> Result<tokio::runtime::Runtime, SshSessionError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            SshSessionError::new(format!("SSH async runtime creation failed: {error}"))
        })
}

async fn complete_established_forward<F>(
    operation_result: Result<(), SshSessionError>,
    cleanup: F,
) -> Result<(), SshSessionError>
where
    F: std::future::Future<Output = Result<(), SshSessionError>>,
{
    let cleanup_result = cleanup.await;
    operation_result.and(cleanup_result)
}

async fn complete_remote_forward_session(
    session_result: Result<(), SshSessionError>,
    cancellation: &RusshForwardCancellation,
    task_tracker: &RemoteForwardTaskTracker,
    shutdown_timeout: Duration,
) -> Result<(), SshSessionError> {
    cancellation.cancel();
    let drain_result = tokio::time::timeout(shutdown_timeout, task_tracker.wait_for_empty())
        .await
        .map_err(|_| SshSessionError::new("SSH remote forwarding shutdown timed out"));
    session_result.and(drain_result)
}

fn remote_forward_cleanup_required(
    has_session_handle: bool,
    task_tracker: &RemoteForwardTaskTracker,
) -> bool {
    has_session_handle || task_tracker.active() > 0
}

pub struct RusshRemoteTcpIpForward {
    runtime: tokio::runtime::Runtime,
    handle: Option<russh::client::Handle<RusshClientHandler>>,
    bind_host: String,
    bind_port: u16,
    cancellation: RusshForwardCancellation,
    task_tracker: Arc<RemoteForwardTaskTracker>,
    deadlines: RusshForwardDeadlines,
}

impl RusshRemoteTcpIpForward {
    /// Returns the actual server listener port. This differs from the request
    /// when port zero asks the SSH server to allocate an available port.
    #[must_use]
    pub const fn bound_port(&self) -> u16 {
        self.bind_port
    }

    /// Waits until the remote forwarding session ends.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the SSH session fails.
    pub fn wait(&mut self) -> Result<(), SshSessionError> {
        let Some(handle) = self.handle.as_mut() else {
            return self.runtime.block_on(complete_remote_forward_session(
                Ok(()),
                &self.cancellation,
                &self.task_tracker,
                self.deadlines.shutdown,
            ));
        };
        let result = self.runtime.block_on(RemoteForwardSession::wait(handle));
        self.handle = None;
        self.runtime.block_on(complete_remote_forward_session(
            result,
            &self.cancellation,
            &self.task_tracker,
            self.deadlines.shutdown,
        ))
    }

    /// Cancels the server listener, disconnects, and joins within `timeout`.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when cancellation, disconnect, joining, or
    /// the deadline fails.
    pub fn shutdown(&mut self, timeout: Duration) -> Result<(), SshSessionError> {
        self.cancellation.cancel();
        let Some(handle) = self.handle.as_mut() else {
            return self.runtime.block_on(complete_remote_forward_session(
                Ok(()),
                &self.cancellation,
                &self.task_tracker,
                timeout,
            ));
        };
        let outcome = self
            .runtime
            .block_on(shutdown_remote_forward_session_and_tasks(
                handle,
                &self.bind_host,
                u32::from(self.bind_port),
                &self.task_tracker,
                timeout,
            ));
        if outcome.terminal {
            debug_assert_eq!(self.task_tracker.active(), 0);
        }
        if outcome.session_terminal {
            self.handle = None;
        }
        outcome.result
    }

    /// Waits for session completion or a cancellation request. Cancellation
    /// wins when both become ready in the same poll.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the SSH session or bounded shutdown
    /// fails.
    pub fn wait_until_cancelled(
        &mut self,
        cancellation: &RusshForwardCancellation,
        shutdown_timeout: Duration,
    ) -> Result<(), SshSessionError> {
        enum ForwardStop {
            Cancelled,
            Session(Result<(), SshSessionError>),
        }

        let Some(handle) = self.handle.as_mut() else {
            return self.runtime.block_on(complete_remote_forward_session(
                Ok(()),
                &self.cancellation,
                &self.task_tracker,
                shutdown_timeout,
            ));
        };
        let stop = self.runtime.block_on(async {
            tokio::select! {
                biased;
                () = cancellation.cancelled() => ForwardStop::Cancelled,
                result = RemoteForwardSession::wait(handle) => ForwardStop::Session(result),
            }
        });
        match stop {
            ForwardStop::Cancelled => self.shutdown(shutdown_timeout),
            ForwardStop::Session(result) => {
                self.handle = None;
                self.runtime.block_on(complete_remote_forward_session(
                    result,
                    &self.cancellation,
                    &self.task_tracker,
                    shutdown_timeout,
                ))
            }
        }
    }
}

impl Drop for RusshRemoteTcpIpForward {
    fn drop(&mut self) {
        if remote_forward_cleanup_required(self.handle.is_some(), &self.task_tracker) {
            let _ = self.shutdown(Duration::from_millis(250));
        }
    }
}

pub struct RusshSshChannel {
    reader: RusshChannelReader,
    writer: RusshChannelWriter,
}

impl RusshSshChannel {
    #[must_use]
    pub fn new(
        channel: russh::Channel<russh::client::Msg>,
        handle: russh::client::Handle<RusshClientHandler>,
        runtime: tokio::runtime::Runtime,
    ) -> Self {
        Self::new_with_inactivity_timeout(channel, handle, Arc::new(runtime), None)
    }

    fn new_with_inactivity_timeout(
        channel: russh::Channel<russh::client::Msg>,
        handle: russh::client::Handle<RusshClientHandler>,
        runtime: Arc<tokio::runtime::Runtime>,
        inactivity_timeout: Option<Duration>,
    ) -> Self {
        let (read_half, write_half) = channel.split();

        Self {
            reader: RusshChannelReader::new(read_half, Arc::clone(&runtime), inactivity_timeout),
            writer: RusshChannelWriter::new(write_half, handle, runtime),
        }
    }

    #[must_use]
    pub fn into_read_writer(self) -> (RusshChannelReader, RusshChannelWriter) {
        (self.reader, self.writer)
    }
}

pub struct RusshChannelReader {
    read_half: russh::ChannelReadHalf,
    runtime: Arc<tokio::runtime::Runtime>,
    pending_read: VecDeque<u8>,
    result: SshSessionResult,
    finished: bool,
    inactivity_timeout: Option<Duration>,
}

impl RusshChannelReader {
    #[must_use]
    fn new(
        read_half: russh::ChannelReadHalf,
        runtime: Arc<tokio::runtime::Runtime>,
        inactivity_timeout: Option<Duration>,
    ) -> Self {
        Self {
            read_half,
            runtime,
            pending_read: VecDeque::new(),
            result: SshSessionResult::default(),
            finished: false,
            inactivity_timeout,
        }
    }

    fn fill_from_pending(&mut self, buffer: &mut [u8]) -> usize {
        let count = buffer.len().min(self.pending_read.len());
        for slot in buffer.iter_mut().take(count) {
            if let Some(byte) = self.pending_read.pop_front() {
                *slot = byte;
            }
        }
        count
    }

    fn queue_read_bytes(&mut self, bytes: &[u8]) {
        self.pending_read.extend(bytes);
    }

    fn read_blocking(&mut self, buffer: &mut [u8]) -> Result<usize, SshSessionError> {
        let cancelled = std::sync::atomic::AtomicBool::new(false);
        self.read_cancellable_blocking(buffer, &cancelled)?
            .ok_or_else(|| SshSessionError::new("SSH channel read unexpectedly cancelled"))
    }

    fn read_cancellable_blocking(
        &mut self,
        buffer: &mut [u8],
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<Option<usize>, SshSessionError> {
        if buffer.is_empty() {
            return Ok(Some(0));
        }

        let pending_count = self.fill_from_pending(buffer);
        if pending_count > 0 {
            return Ok(Some(pending_count));
        }
        if self.finished {
            return Ok(Some(0));
        }

        loop {
            let runtime = Arc::clone(&self.runtime);
            let selected = match self.inactivity_timeout {
                Some(inactivity_timeout) => runtime
                    .block_on(async {
                        tokio::time::timeout(
                            inactivity_timeout,
                            select_read_or_cancellation(
                                self.read_half.wait(),
                                wait_for_write_cancellation(cancelled),
                            ),
                        )
                        .await
                    })
                    .map_err(|_| {
                        SshSessionError::new(format!(
                            "SSH channel read inactivity deadline exceeded after \
                             {inactivity_timeout:?}"
                        ))
                    })?,
                None => runtime.block_on(select_read_or_cancellation(
                    self.read_half.wait(),
                    wait_for_write_cancellation(cancelled),
                )),
            };
            let Some(message) = selected else {
                return Ok(None);
            };
            match message.map_or(RusshReadAction::Finished, |message| {
                apply_channel_message(&mut self.result, message)
            }) {
                RusshReadAction::Data(data) => {
                    self.queue_read_bytes(&data);
                    return Ok(Some(self.fill_from_pending(buffer)));
                }
                RusshReadAction::Finished => {
                    self.finished = true;
                    return Ok(Some(0));
                }
                RusshReadAction::Continue => {}
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum RusshReadAction {
    Data(Vec<u8>),
    Continue,
    Finished,
}

fn apply_channel_message(
    result: &mut SshSessionResult,
    message: russh::ChannelMsg,
) -> RusshReadAction {
    match message {
        russh::ChannelMsg::Data { data } | russh::ChannelMsg::ExtendedData { data, .. } => {
            RusshReadAction::Data(data.to_vec())
        }
        russh::ChannelMsg::ExitStatus { exit_status } => {
            result.exit_status = Some(exit_status);
            RusshReadAction::Continue
        }
        russh::ChannelMsg::ExitSignal {
            signal_name,
            core_dumped,
            error_message,
            lang_tag,
        } => {
            result.exit_signal = Some(SshExitSignal {
                name: signal_name_text(signal_name),
                core_dumped,
                error_message,
                lang_tag,
            });
            RusshReadAction::Continue
        }
        russh::ChannelMsg::Close => RusshReadAction::Finished,
        // EOF only half-closes remote data; exit metadata may still arrive
        // before Close, so it follows the non-terminal path here.
        _ => RusshReadAction::Continue,
    }
}

fn signal_name_text(signal: russh::Sig) -> String {
    match signal {
        russh::Sig::ABRT => "ABRT".to_owned(),
        russh::Sig::ALRM => "ALRM".to_owned(),
        russh::Sig::FPE => "FPE".to_owned(),
        russh::Sig::HUP => "HUP".to_owned(),
        russh::Sig::ILL => "ILL".to_owned(),
        russh::Sig::INT => "INT".to_owned(),
        russh::Sig::KILL => "KILL".to_owned(),
        russh::Sig::PIPE => "PIPE".to_owned(),
        russh::Sig::QUIT => "QUIT".to_owned(),
        russh::Sig::SEGV => "SEGV".to_owned(),
        russh::Sig::TERM => "TERM".to_owned(),
        russh::Sig::USR1 => "USR1".to_owned(),
        russh::Sig::Custom(name) => name,
    }
}

impl Read for RusshChannelReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.read_blocking(buffer)
            .map_err(|error| std::io::Error::other(error.to_string()))
    }
}

impl SshShellReader for RusshChannelReader {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, SshSessionError> {
        self.read_blocking(buffer)
    }

    fn read_cancellable(
        &mut self,
        buffer: &mut [u8],
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<Option<usize>, SshSessionError> {
        self.read_cancellable_blocking(buffer, cancelled)
    }

    fn session_result(&self) -> SshSessionResult {
        self.result.clone()
    }
}

pub struct RusshChannelWriter {
    write_half: russh::ChannelWriteHalf<russh::client::Msg>,
    handle: russh::client::Handle<RusshClientHandler>,
    runtime: Arc<tokio::runtime::Runtime>,
    lifecycle: RusshWriterLifecycle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RusshWriterAction {
    Execute,
    AlreadyDone,
}

#[derive(Debug, Default)]
struct RusshWriterLifecycle {
    input_finished: bool,
    closed: bool,
}

impl RusshWriterLifecycle {
    fn ensure_write_allowed(&self) -> Result<(), SshSessionError> {
        if self.closed {
            Err(SshSessionError::new("SSH channel is closed"))
        } else if self.input_finished {
            Err(SshSessionError::new(
                "SSH channel input is already finished",
            ))
        } else {
            Ok(())
        }
    }

    fn prepare_finish_input(&self) -> RusshWriterAction {
        if self.closed || self.input_finished {
            RusshWriterAction::AlreadyDone
        } else {
            RusshWriterAction::Execute
        }
    }

    fn mark_input_finished(&mut self) {
        self.input_finished = true;
    }

    fn prepare_close(&self) -> RusshWriterAction {
        if self.closed {
            RusshWriterAction::AlreadyDone
        } else {
            RusshWriterAction::Execute
        }
    }

    fn mark_closed(&mut self) {
        self.input_finished = true;
        self.closed = true;
    }
}

impl RusshChannelWriter {
    #[must_use]
    fn new(
        write_half: russh::ChannelWriteHalf<russh::client::Msg>,
        handle: russh::client::Handle<RusshClientHandler>,
        runtime: Arc<tokio::runtime::Runtime>,
    ) -> Self {
        Self {
            write_half,
            handle,
            runtime,
            lifecycle: RusshWriterLifecycle::default(),
        }
    }

    fn write_blocking(&mut self, bytes: &[u8]) -> Result<usize, SshSessionError> {
        self.lifecycle.ensure_write_allowed()?;
        self.runtime
            .block_on(self.write_half.data_bytes(bytes.to_vec()))
            .map_err(|error| SshSessionError::new(format!("SSH channel write failed: {error}")))?;

        Ok(bytes.len())
    }

    fn write_cancellable_blocking(
        &mut self,
        bytes: &[u8],
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<Option<usize>, SshSessionError> {
        self.lifecycle.ensure_write_allowed()?;
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(None);
        }

        let write_result = self.runtime.block_on(select_write_or_cancellation(
            self.write_half.data_bytes(bytes.to_vec()),
            wait_for_write_cancellation(cancelled),
        ));
        match write_result {
            Some(result) => {
                result.map_err(|error| {
                    SshSessionError::new(format!("SSH channel write failed: {error}"))
                })?;
                Ok(Some(bytes.len()))
            }
            None => Ok(None),
        }
    }

    fn resize_pty(&mut self, size: TerminalSize) -> Result<(), SshSessionError> {
        self.runtime
            .block_on(self.write_half.window_change(
                u32::from(size.columns),
                u32::from(size.rows),
                0,
                0,
            ))
            .map_err(|error| SshSessionError::new(format!("SSH PTY resize failed: {error}")))
    }

    fn resize_cancellable_blocking(
        &mut self,
        size: TerminalSize,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<Option<()>, SshSessionError> {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(None);
        }

        let resize_result = self.runtime.block_on(select_resize_or_cancellation(
            self.write_half
                .window_change(u32::from(size.columns), u32::from(size.rows), 0, 0),
            wait_for_write_cancellation(cancelled),
        ));
        match resize_result {
            Some(result) => {
                result.map_err(|error| {
                    SshSessionError::new(format!("SSH PTY resize failed: {error}"))
                })?;
                Ok(Some(()))
            }
            None => Ok(None),
        }
    }

    fn send_keepalive(&mut self) -> Result<(), SshSessionError> {
        self.runtime
            .block_on(self.handle.send_keepalive(false))
            .map_err(|error| SshSessionError::new(format!("SSH keepalive failed: {error}")))
    }

    fn close_channel(&mut self) -> Result<(), SshSessionError> {
        if self.lifecycle.prepare_close() == RusshWriterAction::AlreadyDone {
            return Ok(());
        }
        self.runtime
            .block_on(self.write_half.close())
            .map_err(|error| SshSessionError::new(format!("SSH channel close failed: {error}")))?;
        self.lifecycle.mark_closed();
        Ok(())
    }

    fn finish_input_blocking(&mut self) -> Result<(), SshSessionError> {
        if self.lifecycle.prepare_finish_input() == RusshWriterAction::AlreadyDone {
            return Ok(());
        }
        self.runtime
            .block_on(self.write_half.eof())
            .map_err(|error| SshSessionError::new(format!("SSH channel EOF failed: {error}")))?;
        self.lifecycle.mark_input_finished();
        Ok(())
    }
}

impl Write for RusshChannelWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.write_blocking(bytes)
            .map_err(|error| std::io::Error::other(error.to_string()))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl SshShellWriter for RusshChannelWriter {
    fn write(&mut self, bytes: &[u8]) -> Result<usize, SshSessionError> {
        self.write_blocking(bytes)
    }

    fn write_cancellable(
        &mut self,
        bytes: &[u8],
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<Option<usize>, SshSessionError> {
        self.write_cancellable_blocking(bytes, cancelled)
    }

    fn resize(&mut self, size: TerminalSize) -> Result<(), SshSessionError> {
        self.resize_pty(size)
    }

    fn resize_cancellable(
        &mut self,
        size: TerminalSize,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<Option<()>, SshSessionError> {
        self.resize_cancellable_blocking(size, cancelled)
    }

    fn keepalive(&mut self) -> Result<(), SshSessionError> {
        self.send_keepalive()
    }

    fn finish_input(&mut self) -> Result<(), SshSessionError> {
        self.finish_input_blocking()
    }

    fn close(&mut self) -> Result<(), SshSessionError> {
        self.close_channel()
    }
}

async fn wait_for_write_cancellation(cancelled: &std::sync::atomic::AtomicBool) {
    while !cancelled.load(std::sync::atomic::Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn select_write_or_cancellation<W, C>(
    write: W,
    cancellation: C,
) -> Option<Result<(), russh::Error>>
where
    W: Future<Output = Result<(), russh::Error>>,
    C: Future<Output = ()>,
{
    tokio::pin!(write);
    tokio::pin!(cancellation);
    tokio::select! {
        biased;
        () = &mut cancellation => None,
        result = &mut write => Some(result),
    }
}

async fn select_resize_or_cancellation<R, C>(
    resize: R,
    cancellation: C,
) -> Option<Result<(), russh::Error>>
where
    R: Future<Output = Result<(), russh::Error>>,
    C: Future<Output = ()>,
{
    select_write_or_cancellation(resize, cancellation).await
}

async fn select_read_or_cancellation<R, C>(read: R, cancellation: C) -> Option<R::Output>
where
    R: Future,
    C: Future<Output = ()>,
{
    tokio::pin!(read);
    tokio::pin!(cancellation);
    tokio::select! {
        biased;
        () = &mut cancellation => None,
        result = &mut read => Some(result),
    }
}

impl SshChannel for RusshSshChannel {
    fn read_channel(&mut self, buffer: &mut [u8]) -> Result<usize, SshSessionError> {
        self.reader.read_blocking(buffer)
    }

    fn write_channel(&mut self, bytes: &[u8]) -> Result<usize, SshSessionError> {
        self.writer.write_blocking(bytes)
    }

    fn resize_pty(&mut self, size: TerminalSize) -> Result<(), SshSessionError> {
        self.writer.resize_pty(size)
    }

    fn send_keepalive(&mut self) -> Result<(), SshSessionError> {
        self.writer.send_keepalive()
    }

    fn close_channel(&mut self) -> Result<(), SshSessionError> {
        self.writer.close_channel()
    }

    fn into_read_writer(self) -> (Box<dyn SshShellReader>, Box<dyn SshShellWriter>) {
        let (reader, writer) = RusshSshChannel::into_read_writer(self);
        (Box::new(reader), Box::new(writer))
    }
}

impl SshChannelOpener for RusshChannelOpener {
    type Channel = RusshSshChannel;

    fn open_channel(
        &mut self,
        request: SshConnectRequest,
    ) -> Result<Self::Channel, SshSessionError> {
        let plan = self.connect_plan(&request);
        let startup_plan = plan.channel_startup_plan();
        let runtime = self.runtime_handle_or_create()?;

        let operation_timeout = self.operation_timeout;
        let connection_cancellation = self.connection_cancellation.clone();
        let (handle, channel) = runtime.runtime.block_on(async {
            let operation = async {
                tokio::time::timeout(operation_timeout, async {
                    let mut handle = self.connect_async(request).await?;
                    self.authenticate_async(&mut handle, plan.auth_plan())
                        .await?;
                    let channel = self.open_session_channel_async(&handle).await?;
                    self.start_channel_async(&channel, &startup_plan).await?;

                    Ok::<_, SshSessionError>((handle, channel))
                })
                .await
                .map_err(|_| {
                    SshSessionError::new(format!(
                        "SSH session operation deadline exceeded after {operation_timeout:?}"
                    ))
                })?
            };
            if let Some(cancellation) = connection_cancellation {
                tokio::select! {
                    biased;
                    () = cancellation.cancelled() => {
                        Err(SshSessionError::new("SSH connection cancelled"))
                    }
                    result = operation => result,
                }
            } else {
                operation.await
            }
        })?;
        self.report_phase(SshConnectionPhase::Connected);

        Ok(RusshSshChannel::new_with_inactivity_timeout(
            channel,
            handle,
            Arc::clone(&runtime.runtime),
            self.channel_inactivity_timeout,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::{future::Future, path::Path, pin::Pin, time::Duration};

    use rterm_types::TerminalSize;

    use super::*;
    use crate::{SshConnectRequest, SshSessionConfig};

    #[test]
    fn lazy_russh_runtime_is_uninitialized_until_first_access_and_bounded() {
        let runtime = LazyRusshRuntime::new();

        assert!(!runtime.is_initialized());

        let handle = runtime.get_or_try_init().expect("initialize SSH runtime");

        assert!(runtime.is_initialized());
        assert_eq!(handle.config(), RusshRuntimeConfig::production());
        assert_eq!(handle.config().worker_threads(), 2);
        assert_eq!(handle.config().max_blocking_threads(), 8);
    }

    #[test]
    fn concurrent_lazy_russh_runtime_access_reuses_one_runtime() {
        let runtime = Arc::new(LazyRusshRuntime::new());
        let handles = (0..8)
            .map(|_| {
                let runtime = Arc::clone(&runtime);
                std::thread::spawn(move || runtime.get_or_try_init().expect("initialize runtime"))
            })
            .map(|thread| thread.join().expect("join runtime initializer"))
            .collect::<Vec<_>>();

        assert!(
            handles
                .iter()
                .skip(1)
                .all(|handle| handles[0].shares_runtime_with(handle))
        );
    }

    #[test]
    fn channel_openers_can_share_an_injected_russh_runtime() {
        let runtime = LazyRusshRuntime::new();
        let handle = runtime.get_or_try_init().expect("initialize SSH runtime");
        let first = RusshChannelOpener::default().with_runtime_handle(handle.clone());
        let second = RusshChannelOpener::default().with_runtime_handle(handle);

        assert!(
            first
                .runtime_handle()
                .expect("first runtime")
                .shares_runtime_with(second.runtime_handle().expect("second runtime"))
        );
    }

    type TestAuthFuture<'a> =
        Pin<Box<dyn Future<Output = Result<russh::client::AuthResult, SshSessionError>> + 'a>>;

    #[test]
    fn remote_forward_matcher_rejects_pending_and_wrong_ports_then_accepts_assigned_port() {
        let forward = ResolvedRemoteForward::new(RusshRemoteTcpIpForwardPlan::new(
            "127.0.0.1",
            0,
            "127.0.0.1",
            9,
        ));
        assert!(!forward.matches_connected_endpoint("127.0.0.1", 0));
        assert!(!forward.matches_connected_endpoint("127.0.0.1", 42_000));

        forward.resolve_bind_port(42_000);

        assert!(forward.matches_connected_endpoint("127.0.0.1", 42_000));
        assert!(!forward.matches_connected_endpoint("127.0.0.1", 42_001));
        assert!(!forward.matches_connected_endpoint("192.0.2.1", 42_000));
    }

    #[test]
    fn remote_forward_bound_port_validation_rejects_zero_and_out_of_range_values() {
        assert!(validated_remote_forward_bound_port(0).is_err());
        assert!(validated_remote_forward_bound_port(u32::from(u16::MAX) + 1).is_err());
        assert_eq!(validated_remote_forward_bound_port(42_000).unwrap(), 42_000);
    }

    #[test]
    fn fixed_remote_forward_uses_requested_port_when_success_has_no_allocated_port() {
        assert_eq!(
            resolved_remote_forward_bound_port(42_000, 0).unwrap(),
            42_000
        );
        assert_eq!(
            resolved_remote_forward_bound_port(0, 42_001).unwrap(),
            42_001
        );
        assert!(resolved_remote_forward_bound_port(0, 0).is_err());
    }

    #[test]
    fn remote_exit_status_is_preserved() {
        let mut result = crate::SshSessionResult::default();

        assert_eq!(
            apply_channel_message(&mut result, russh::ChannelMsg::Eof),
            RusshReadAction::Continue
        );
        assert_eq!(
            apply_channel_message(
                &mut result,
                russh::ChannelMsg::ExitStatus { exit_status: 7 },
            ),
            RusshReadAction::Continue
        );
        let status_action = apply_channel_message(
            &mut result,
            russh::ChannelMsg::ExitStatus { exit_status: 23 },
        );
        let data_action = apply_channel_message(
            &mut result,
            russh::ChannelMsg::Data {
                data: b"late output".as_slice().into(),
            },
        );
        let close_action = apply_channel_message(&mut result, russh::ChannelMsg::Close);

        assert_eq!(status_action, RusshReadAction::Continue);
        assert_eq!(data_action, RusshReadAction::Data(b"late output".to_vec()));
        assert_eq!(close_action, RusshReadAction::Finished);
        assert_eq!(result.exit_status, Some(23));
        assert_eq!(result.exit_signal, None);
    }

    #[test]
    fn remote_exit_signal_is_preserved() {
        let mut result = crate::SshSessionResult::default();

        assert_eq!(
            apply_channel_message(&mut result, russh::ChannelMsg::Eof),
            RusshReadAction::Continue
        );
        assert_eq!(
            apply_channel_message(
                &mut result,
                russh::ChannelMsg::ExitSignal {
                    signal_name: russh::Sig::HUP,
                    core_dumped: false,
                    error_message: String::new(),
                    lang_tag: String::new(),
                },
            ),
            RusshReadAction::Continue
        );
        let action = apply_channel_message(
            &mut result,
            russh::ChannelMsg::ExitSignal {
                signal_name: russh::Sig::TERM,
                core_dumped: true,
                error_message: "terminated by policy".to_owned(),
                lang_tag: "en-US".to_owned(),
            },
        );

        assert_eq!(action, RusshReadAction::Continue);
        assert_eq!(
            apply_channel_message(&mut result, russh::ChannelMsg::Close),
            RusshReadAction::Finished
        );
        assert_eq!(
            result.exit_signal,
            Some(crate::SshExitSignal {
                name: "TERM".to_owned(),
                core_dumped: true,
                error_message: "terminated by policy".to_owned(),
                lang_tag: "en-US".to_owned(),
            })
        );
        assert_eq!(result.exit_status, None);
    }

    #[test]
    fn remote_exit_status_and_signal_coexist_in_either_order() {
        let signal = || russh::ChannelMsg::ExitSignal {
            signal_name: russh::Sig::TERM,
            core_dumped: true,
            error_message: "terminated by policy".to_owned(),
            lang_tag: "en-US".to_owned(),
        };
        let expected_signal = SshExitSignal {
            name: "TERM".to_owned(),
            core_dumped: true,
            error_message: "terminated by policy".to_owned(),
            lang_tag: "en-US".to_owned(),
        };

        let mut status_then_signal = SshSessionResult::default();
        apply_channel_message(
            &mut status_then_signal,
            russh::ChannelMsg::ExitStatus { exit_status: 23 },
        );
        apply_channel_message(&mut status_then_signal, signal());
        assert_eq!(status_then_signal.exit_status, Some(23));
        assert_eq!(
            status_then_signal.exit_signal.as_ref(),
            Some(&expected_signal)
        );

        let mut signal_then_status = SshSessionResult::default();
        apply_channel_message(&mut signal_then_status, signal());
        apply_channel_message(
            &mut signal_then_status,
            russh::ChannelMsg::ExitStatus { exit_status: 42 },
        );
        assert_eq!(signal_then_status.exit_status, Some(42));
        assert_eq!(
            signal_then_status.exit_signal.as_ref(),
            Some(&expected_signal)
        );
    }

    #[test]
    fn russh_writer_lifecycle_makes_eof_and_close_idempotent() {
        let mut lifecycle = RusshWriterLifecycle::default();

        assert_eq!(lifecycle.prepare_finish_input(), RusshWriterAction::Execute);
        lifecycle.mark_input_finished();
        assert_eq!(
            lifecycle.prepare_finish_input(),
            RusshWriterAction::AlreadyDone
        );
        assert!(lifecycle.ensure_write_allowed().is_err());

        assert_eq!(lifecycle.prepare_close(), RusshWriterAction::Execute);
        lifecycle.mark_closed();
        assert_eq!(lifecycle.prepare_close(), RusshWriterAction::AlreadyDone);
        assert_eq!(
            lifecycle.prepare_finish_input(),
            RusshWriterAction::AlreadyDone
        );
        assert!(lifecycle.ensure_write_allowed().is_err());
    }

    #[test]
    fn russh_write_cancellation_wins_when_both_futures_are_ready() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let selected = runtime.block_on(select_write_or_cancellation(
            std::future::ready(Ok(())),
            std::future::ready(()),
        ));

        assert!(selected.is_none());
    }

    #[test]
    fn russh_resize_cancellation_wins_when_both_futures_are_ready() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let selected = runtime.block_on(select_resize_or_cancellation(
            std::future::ready(Ok(())),
            std::future::ready(()),
        ));

        assert!(selected.is_none());
    }

    #[test]
    fn russh_read_cancellation_wins_when_both_futures_are_ready() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let selected = runtime.block_on(select_read_or_cancellation(
            std::future::ready(17_u8),
            std::future::ready(()),
        ));
        assert_eq!(selected, None);
    }

    #[derive(Default)]
    struct MockRemoteForwardSession {
        calls: Vec<&'static str>,
        join_never_finishes: bool,
        cancel_fails: bool,
        disconnect_fails: bool,
    }

    impl RemoteForwardSession for MockRemoteForwardSession {
        async fn cancel_tcpip_forward(
            &mut self,
            _bind_host: &str,
            _bind_port: u32,
        ) -> Result<(), SshSessionError> {
            self.calls.push("cancel");
            if self.cancel_fails {
                Err(SshSessionError::new("cancel failed"))
            } else {
                Ok(())
            }
        }

        async fn disconnect(&mut self) -> Result<(), SshSessionError> {
            self.calls.push("disconnect");
            if self.disconnect_fails {
                Err(SshSessionError::new("disconnect failed"))
            } else {
                Ok(())
            }
        }

        async fn wait(&mut self) -> Result<(), SshSessionError> {
            self.calls.push("join");
            if self.join_never_finishes {
                std::future::pending::<()>().await;
            }
            Ok(())
        }
    }

    #[test]
    fn remote_forward_shutdown_cancels_disconnects_and_joins_in_order() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mut session = MockRemoteForwardSession::default();

        runtime
            .block_on(shutdown_remote_forward_session(
                &mut session,
                "127.0.0.1",
                8022,
                Duration::from_millis(250),
            ))
            .unwrap();

        assert_eq!(session.calls, ["cancel", "disconnect", "join"]);
    }

    #[test]
    fn remote_forward_shutdown_join_is_bounded_by_deadline() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mut session = MockRemoteForwardSession {
            join_never_finishes: true,
            ..MockRemoteForwardSession::default()
        };

        let error = runtime
            .block_on(shutdown_remote_forward_session(
                &mut session,
                "127.0.0.1",
                8022,
                Duration::from_millis(25),
            ))
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "SSH remote forwarding shutdown timed out"
        );
        assert_eq!(session.calls, ["cancel", "disconnect", "join"]);
    }

    #[test]
    fn remote_forward_shutdown_is_terminal_after_partial_errors_when_join_completes() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mut session = MockRemoteForwardSession {
            cancel_fails: true,
            disconnect_fails: true,
            ..MockRemoteForwardSession::default()
        };

        let outcome = runtime.block_on(shutdown_remote_forward_session_outcome(
            &mut session,
            "127.0.0.1",
            8022,
            Duration::from_millis(250),
        ));

        assert!(outcome.terminal);
        assert_eq!(outcome.result.unwrap_err().to_string(), "cancel failed");
        assert_eq!(session.calls, ["cancel", "disconnect", "join"]);
    }

    #[test]
    fn remote_forward_shutdown_waits_for_registered_children() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let tracker = Arc::new(RemoteForwardTaskTracker::default());
        let guard = tracker.register();
        runtime.spawn(async move {
            tokio::time::sleep(Duration::from_millis(25)).await;
            drop(guard);
        });
        let mut session = MockRemoteForwardSession::default();
        let started = std::time::Instant::now();

        let outcome = runtime.block_on(shutdown_remote_forward_session_and_tasks(
            &mut session,
            "127.0.0.1",
            8022,
            &tracker,
            Duration::from_millis(250),
        ));

        assert!(outcome.terminal);
        outcome.result.unwrap();
        assert!(started.elapsed() >= Duration::from_millis(20));
        assert_eq!(tracker.active(), 0);
    }

    #[test]
    fn remote_forward_shutdown_child_join_is_bounded_by_total_deadline() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let tracker = Arc::new(RemoteForwardTaskTracker::default());
        let guard = tracker.register();
        let mut session = MockRemoteForwardSession::default();

        let outcome = runtime.block_on(shutdown_remote_forward_session_and_tasks(
            &mut session,
            "127.0.0.1",
            8022,
            &tracker,
            Duration::from_millis(20),
        ));

        assert!(!outcome.terminal);
        assert!(outcome.session_terminal);
        assert_eq!(
            outcome.result.unwrap_err().to_string(),
            "SSH remote forwarding shutdown timed out"
        );
        drop(guard);
    }

    #[test]
    fn established_forward_error_still_runs_cleanup_and_preserves_primary_error() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let cleaned = Arc::new(AtomicBool::new(false));
        let cleanup_flag = Arc::clone(&cleaned);

        let error = runtime
            .block_on(complete_established_forward(
                Err(SshSessionError::new("ready failed")),
                async move {
                    cleanup_flag.store(true, Ordering::Release);
                    Ok(())
                },
            ))
            .unwrap_err();

        assert!(cleaned.load(Ordering::Acquire));
        assert_eq!(error.to_string(), "ready failed");
    }

    #[test]
    fn natural_remote_forward_completion_cancels_and_bounded_drains_children() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let cancellation = RusshForwardCancellation::new();
        let tracker = Arc::new(RemoteForwardTaskTracker::default());
        let guard = tracker.register();
        let child_cancellation = cancellation.clone();
        runtime.spawn(async move {
            child_cancellation.cancelled().await;
            tokio::time::sleep(Duration::from_millis(20)).await;
            drop(guard);
        });

        runtime
            .block_on(complete_remote_forward_session(
                Ok(()),
                &cancellation,
                &tracker,
                Duration::from_millis(250),
            ))
            .unwrap();

        assert!(cancellation.is_cancelled());
        assert_eq!(tracker.active(), 0);
    }

    #[test]
    fn natural_remote_forward_drain_is_bounded_and_preserves_session_error() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let cancellation = RusshForwardCancellation::new();
        let tracker = Arc::new(RemoteForwardTaskTracker::default());
        let guard = tracker.register();
        let started = std::time::Instant::now();

        let error = runtime
            .block_on(complete_remote_forward_session(
                Err(SshSessionError::new("session failed")),
                &cancellation,
                &tracker,
                Duration::from_millis(20),
            ))
            .unwrap_err();

        assert!(started.elapsed() < Duration::from_millis(200));
        assert!(cancellation.is_cancelled());
        assert_eq!(error.to_string(), "session failed");
        drop(guard);
    }

    #[test]
    fn remote_forward_drop_cleanup_is_required_for_unjoined_children_without_session_handle() {
        let tracker = Arc::new(RemoteForwardTaskTracker::default());
        let guard = tracker.register();

        assert!(remote_forward_cleanup_required(false, &tracker));

        drop(guard);
        assert!(!remote_forward_cleanup_required(false, &tracker));
    }

    #[test]
    fn direct_forward_runtime_uses_one_current_thread_executor() {
        let runtime = build_direct_forward_runtime().unwrap();

        assert_eq!(
            runtime.handle().runtime_flavor(),
            tokio::runtime::RuntimeFlavor::CurrentThread
        );
    }

    #[test]
    fn forwarding_startup_is_bounded_and_cancellation_wins() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let cancellation = RusshForwardCancellation::new();
        let timeout_error = runtime
            .block_on(run_forward_startup(
                &cancellation,
                Duration::from_millis(20),
                std::future::pending::<Result<(), SshSessionError>>(),
            ))
            .unwrap_err();
        assert_eq!(
            timeout_error.to_string(),
            "SSH forwarding startup timed out"
        );

        cancellation.cancel();
        let cancellation_error = runtime
            .block_on(run_forward_startup(
                &cancellation,
                Duration::from_secs(1),
                std::future::ready(Ok(())),
            ))
            .unwrap_err();
        assert_eq!(
            cancellation_error.to_string(),
            "SSH forwarding startup cancelled"
        );
    }

    #[derive(Default)]
    struct MockAuthBackend {
        calls: Vec<String>,
    }

    impl RusshAuthenticationBackend for MockAuthBackend {
        fn authenticate_password<'a>(
            &'a mut self,
            username: &'a str,
            _password: &'a str,
        ) -> TestAuthFuture<'a> {
            self.calls.push(format!("password:{username}"));
            Box::pin(async { Ok(russh::client::AuthResult::Success) })
        }

        fn authenticate_private_key<'a>(
            &'a mut self,
            username: &'a str,
            _path: &'a Path,
            _passphrase: Option<&'a str>,
        ) -> TestAuthFuture<'a> {
            self.calls.push(format!("private-key:{username}"));
            Box::pin(async { Ok(russh::client::AuthResult::Success) })
        }

        fn authenticate_agent<'a>(&'a mut self, username: &'a str) -> TestAuthFuture<'a> {
            self.calls.push(format!("agent:{username}"));
            Box::pin(async { Ok(russh::client::AuthResult::Success) })
        }
    }

    #[test]
    fn authenticate_auth_plan_uses_agent_backend_for_agent_authentication() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24)).unwrap(),
        );
        let plan = RusshAuthPlan::from_request(&request);
        let mut backend = MockAuthBackend::default();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let outcome = runtime
            .block_on(authenticate_auth_plan_with_backend(
                &mut backend,
                &plan,
                None,
            ))
            .unwrap();

        assert_eq!(outcome, RusshAuthOutcome::Authenticated);
        assert_eq!(backend.calls, ["agent:ops"]);
    }

    #[test]
    fn authenticate_auth_plan_uses_secret_provider_for_password_prompt() {
        let request = SshConnectRequest::password_prompt(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24)).unwrap(),
        );
        let plan = RusshAuthPlan::from_request(&request);
        let provider = SecretProvider::new(|prompt: crate::SecretPrompt| async move {
            assert_eq!(prompt.username, "ops");
            assert_eq!(prompt.kind, crate::SecretPromptKind::Password);
            Some("test-only-secret".to_owned())
        });
        let mut backend = MockAuthBackend::default();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let outcome = runtime
            .block_on(authenticate_auth_plan_with_backend(
                &mut backend,
                &plan,
                Some(&provider),
            ))
            .unwrap();

        assert_eq!(outcome, RusshAuthOutcome::Authenticated);
        assert_eq!(backend.calls, ["password:ops"]);
    }
}
