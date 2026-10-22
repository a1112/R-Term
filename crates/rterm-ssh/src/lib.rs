use std::{future::Future, path::PathBuf, pin::Pin, sync::Arc};

pub use rterm_types::TerminalSize as SshTerminalSize;
use rterm_types::TerminalSize;

mod runtime_adapter;
mod russh_client;

const REDACTED_SECRET: &str = "<redacted>";

pub use runtime_adapter::{SshControl, SshInterrupt, SshReader, SshTransport, SshWriter};
pub use russh_client::{
    LazyRusshRuntime, RusshAuthOutcome, RusshAuthPlan, RusshAuthRequest, RusshChannelOpener,
    RusshChannelStartupPlan, RusshChannelStartupRequest, RusshClientHandler, RusshConnectPlan,
    RusshConnectionCancellation, RusshDirectTcpIpOpenPlan, RusshForwardCancellation,
    RusshForwardDeadlines, RusshHostKeyPolicy, RusshKnownHosts, RusshPrivateKeyAuth,
    RusshRemoteTcpIpForward, RusshRemoteTcpIpForwardPlan, RusshRuntimeConfig, RusshRuntimeHandle,
    RusshSshChannel,
};

/// The kind of secret requested while establishing a native SSH session.
/// Secret values are supplied only through the asynchronous provider and are
/// never part of a connection request or host-key challenge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretPromptKind {
    Password,
    PrivateKeyPassphrase,
}

/// Non-secret metadata for a password or private-key passphrase prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretPrompt {
    pub username: String,
    pub kind: SecretPromptKind,
}

impl SecretPrompt {
    #[must_use]
    pub fn password(username: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            kind: SecretPromptKind::Password,
        }
    }

    #[must_use]
    pub fn private_key_passphrase(username: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            kind: SecretPromptKind::PrivateKeyPassphrase,
        }
    }
}

/// Future returned by an asynchronous secret provider. `None` means that the
/// user cancelled the prompt; the returned string is consumed by the auth
/// operation and is not retained by the SSH request.
pub type SecretPromptFuture = Pin<Box<dyn Future<Output = Option<String>> + Send>>;

pub trait AsyncSecretProvider: Send + Sync {
    fn prompt(&self, prompt: SecretPrompt) -> SecretPromptFuture;
}

impl<F, Fut> AsyncSecretProvider for F
where
    F: Fn(SecretPrompt) -> Fut + Send + Sync,
    Fut: Future<Output = Option<String>> + Send + 'static,
{
    fn prompt(&self, prompt: SecretPrompt) -> SecretPromptFuture {
        Box::pin(self(prompt))
    }
}

impl<T> AsyncSecretProvider for Arc<T>
where
    T: AsyncSecretProvider + ?Sized,
{
    fn prompt(&self, prompt: SecretPrompt) -> SecretPromptFuture {
        (**self).prompt(prompt)
    }
}

#[derive(Clone)]
pub struct SecretProvider {
    inner: Arc<dyn AsyncSecretProvider>,
}

impl SecretProvider {
    #[must_use]
    pub fn new<V>(provider: V) -> Self
    where
        V: AsyncSecretProvider + 'static,
    {
        Self {
            inner: Arc::new(provider),
        }
    }

    #[must_use]
    pub fn prompt(&self, prompt: SecretPrompt) -> SecretPromptFuture {
        self.inner.prompt(prompt)
    }
}

impl std::fmt::Debug for SecretProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SecretProvider(..)")
    }
}

impl AsyncSecretProvider for SecretProvider {
    fn prompt(&self, prompt: SecretPrompt) -> SecretPromptFuture {
        self.inner.prompt(prompt)
    }
}

/// The outcome of an asynchronous host-key prompt.
///
/// `AcceptOnce` keeps the key in memory for the current connection, while
/// `AcceptAndStore` also asks the native backend to append the key to its
/// configured `known_hosts` file. `Reject` and `Cancel` are intentionally
/// separate names for callers that want to distinguish a negative choice
/// from dismissing a prompt; both deny the connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKeyDecision {
    AcceptOnce,
    AcceptAndStore,
    Reject,
    Cancel,
}

impl HostKeyDecision {
    #[must_use]
    pub const fn accepts(self) -> bool {
        matches!(self, Self::AcceptOnce | Self::AcceptAndStore)
    }

    #[must_use]
    pub const fn stores(self) -> bool {
        matches!(self, Self::AcceptAndStore)
    }
}

/// Whether the key supplied in a [`HostKeyChallenge`] is already trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKeyStatus {
    /// The key matches the configured known-hosts entry.
    Known,
    /// No known-hosts entry exists for this host and port.
    Unknown,
    /// An entry exists for this host and port, but its key changed.
    Changed,
}

/// Non-secret information supplied to an asynchronous host-key verifier.
///
/// The challenge deliberately contains no authentication material or channel
/// data. It is safe to pass to a GUI prompt and to retain only for the prompt
/// lifetime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostKeyChallenge {
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub fingerprint: String,
    pub status: HostKeyStatus,
    pub known_hosts_path: Option<PathBuf>,
}

impl HostKeyChallenge {
    #[must_use]
    pub fn new(
        host: impl Into<String>,
        port: u16,
        algorithm: impl Into<String>,
        fingerprint: impl Into<String>,
        status: HostKeyStatus,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            algorithm: algorithm.into(),
            fingerprint: fingerprint.into(),
            status,
            known_hosts_path: None,
        }
    }

    #[must_use]
    pub fn with_known_hosts_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.known_hosts_path = Some(path.into());
        self
    }
}

/// Future returned by an asynchronous host-key verifier.
pub type HostKeyVerificationFuture = Pin<Box<dyn Future<Output = HostKeyDecision> + Send>>;

/// Async callback contract used by [`HostKeyVerifier`].
pub trait AsyncHostKeyVerifier: Send + Sync {
    fn verify(&self, challenge: HostKeyChallenge) -> HostKeyVerificationFuture;
}

impl<F, Fut> AsyncHostKeyVerifier for F
where
    F: Fn(HostKeyChallenge) -> Fut + Send + Sync,
    Fut: Future<Output = HostKeyDecision> + Send + 'static,
{
    fn verify(&self, challenge: HostKeyChallenge) -> HostKeyVerificationFuture {
        Box::pin(self(challenge))
    }
}

impl<T> AsyncHostKeyVerifier for Arc<T>
where
    T: AsyncHostKeyVerifier + ?Sized,
{
    fn verify(&self, challenge: HostKeyChallenge) -> HostKeyVerificationFuture {
        (**self).verify(challenge)
    }
}

/// Cloneable asynchronous host-key verifier.
#[derive(Clone)]
pub struct HostKeyVerifier {
    inner: Arc<dyn AsyncHostKeyVerifier>,
}

impl HostKeyVerifier {
    #[must_use]
    pub fn new<V>(verifier: V) -> Self
    where
        V: AsyncHostKeyVerifier + 'static,
    {
        Self {
            inner: Arc::new(verifier),
        }
    }

    #[must_use]
    pub fn verify(&self, challenge: HostKeyChallenge) -> HostKeyVerificationFuture {
        self.inner.verify(challenge)
    }
}

impl std::fmt::Debug for HostKeyVerifier {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("HostKeyVerifier(..)")
    }
}

impl AsyncHostKeyVerifier for HostKeyVerifier {
    fn verify(&self, challenge: HostKeyChallenge) -> HostKeyVerificationFuture {
        self.verify(challenge)
    }
}

/// Connection milestones emitted by the native opener.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshConnectionPhase {
    Connecting,
    Authenticating,
    Opening,
    Connected,
}

/// Compatibility alias for clients that call these milestones stages.
pub use SshConnectionPhase as SshConnectionStage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshConfigError {
    EmptyHost,
    EmptyUsername,
    InvalidPort,
    InvalidTerminalSize,
}

impl std::fmt::Display for SshConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::EmptyHost => "SSH host cannot be empty",
            Self::EmptyUsername => "SSH username cannot be empty",
            Self::InvalidPort => "SSH port must be greater than zero",
            Self::InvalidTerminalSize => "SSH terminal size must have non-zero columns and rows",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for SshConfigError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshAuthError {
    EmptyPassword,
    EmptyPrivateKeyPath,
}

impl std::fmt::Display for SshAuthError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::EmptyPassword => "SSH password cannot be empty",
            Self::EmptyPrivateKeyPath => "SSH private key path cannot be empty",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for SshAuthError {}

#[derive(Clone, PartialEq, Eq)]
pub enum SshAuthMethod {
    PasswordPrompt,
    Password {
        password: String,
    },
    PrivateKey {
        path: PathBuf,
        passphrase: Option<String>,
    },
    Agent,
}

impl std::fmt::Debug for SshAuthMethod {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PasswordPrompt => formatter.write_str("PasswordPrompt"),
            Self::Password { .. } => formatter
                .debug_struct("Password")
                .field("password", &REDACTED_SECRET)
                .finish(),
            Self::PrivateKey { path, passphrase } => formatter
                .debug_struct("PrivateKey")
                .field("path", path)
                .field("passphrase", &passphrase.as_ref().map(|_| REDACTED_SECRET))
                .finish(),
            Self::Agent => formatter.write_str("Agent"),
        }
    }
}

impl SshAuthMethod {
    #[must_use]
    pub const fn password_prompt() -> Self {
        Self::PasswordPrompt
    }

    /// Creates password authentication data.
    ///
    /// # Errors
    ///
    /// Returns [`SshAuthError::EmptyPassword`] when the password is empty or
    /// only whitespace.
    pub fn password(password: impl Into<String>) -> Result<Self, SshAuthError> {
        let password = password.into();
        if password.trim().is_empty() {
            return Err(SshAuthError::EmptyPassword);
        }

        Ok(Self::Password { password })
    }

    /// Creates private-key authentication data.
    ///
    /// # Errors
    ///
    /// Returns [`SshAuthError::EmptyPrivateKeyPath`] when `path` is empty.
    pub fn private_key(
        path: impl Into<PathBuf>,
        passphrase: Option<impl Into<String>>,
    ) -> Result<Self, SshAuthError> {
        let path = path.into();
        if path.as_os_str().is_empty() {
            return Err(SshAuthError::EmptyPrivateKeyPath);
        }

        Ok(Self::PrivateKey {
            path,
            passphrase: passphrase.map(Into::into),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshStartupError {
    EmptyCommand,
}

impl std::fmt::Display for SshStartupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::EmptyCommand => "SSH remote command cannot be empty",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for SshStartupError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshSessionStartup {
    Shell,
    /// Execute one program with the provided argument vector.
    ///
    /// The native SSH backend preserves these argument boundaries by quoting
    /// every token for the remote POSIX shell. Callers must not place shell
    /// syntax such as pipelines or redirections in this vector and expect it
    /// to be interpreted.
    Command(Vec<String>),
    NoShell,
}

impl SshSessionStartup {
    /// Creates a remote command startup request with argv semantics.
    ///
    /// # Errors
    ///
    /// Returns [`SshStartupError::EmptyCommand`] when no command tokens are
    /// provided or every token is empty.
    pub fn command(command: impl IntoIterator<Item = String>) -> Result<Self, SshStartupError> {
        let command = command.into_iter().collect::<Vec<_>>();
        if command.iter().all(|argument| argument.trim().is_empty()) {
            return Err(SshStartupError::EmptyCommand);
        }

        Ok(Self::Command(command))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshSessionConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub initial_size: TerminalSize,
}

impl SshSessionConfig {
    /// Creates a config without validation for callers that already normalized
    /// trusted fields. Use [`Self::try_new`] for user-facing input.
    #[must_use]
    pub fn new(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        initial_size: TerminalSize,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            username: username.into(),
            initial_size,
        }
    }

    /// Creates a validated SSH session config from user-facing fields.
    ///
    /// # Errors
    ///
    /// Returns [`SshConfigError`] when host or username is empty, port is zero,
    /// or the requested terminal size has zero columns or rows.
    pub fn try_new(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        initial_size: TerminalSize,
    ) -> Result<Self, SshConfigError> {
        let host = host.into();
        let username = username.into();
        let host = host.trim();
        let username = username.trim();

        if host.is_empty() {
            return Err(SshConfigError::EmptyHost);
        }
        if username.is_empty() {
            return Err(SshConfigError::EmptyUsername);
        }
        if port == 0 {
            return Err(SshConfigError::InvalidPort);
        }
        if initial_size.columns == 0 || initial_size.rows == 0 {
            return Err(SshConfigError::InvalidTerminalSize);
        }

        Ok(Self::new(host, port, username, initial_size))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshConnectRequest {
    pub config: SshSessionConfig,
    pub auth: SshAuthMethod,
    pub startup: SshSessionStartup,
}

impl SshConnectRequest {
    #[must_use]
    pub fn new(config: SshSessionConfig, auth: SshAuthMethod) -> Self {
        Self {
            config,
            auth,
            startup: SshSessionStartup::Shell,
        }
    }

    #[must_use]
    pub fn with_startup(mut self, startup: SshSessionStartup) -> Self {
        self.startup = startup;
        self
    }

    /// Creates a connection request with password authentication.
    ///
    /// # Errors
    ///
    /// Returns [`SshAuthError::EmptyPassword`] when the password is empty or
    /// only whitespace.
    pub fn password(
        config: SshSessionConfig,
        password: impl Into<String>,
    ) -> Result<Self, SshAuthError> {
        Ok(Self::new(config, SshAuthMethod::password(password)?))
    }

    /// Creates a connection request that should prompt for a password through
    /// the active terminal or a future secure prompt.
    #[must_use]
    pub fn password_prompt(config: SshSessionConfig) -> Self {
        Self::new(config, SshAuthMethod::PasswordPrompt)
    }

    /// Creates a connection request with private-key authentication.
    ///
    /// # Errors
    ///
    /// Returns [`SshAuthError::EmptyPrivateKeyPath`] when `path` is empty.
    pub fn private_key(
        config: SshSessionConfig,
        path: impl Into<PathBuf>,
        passphrase: Option<impl Into<String>>,
    ) -> Result<Self, SshAuthError> {
        Ok(Self::new(
            config,
            SshAuthMethod::private_key(path, passphrase)?,
        ))
    }

    #[must_use]
    pub fn agent(config: SshSessionConfig) -> Self {
        Self::new(config, SshAuthMethod::Agent)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshSessionError {
    message: String,
}

impl SshSessionError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SshSessionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SshSessionError {}

/// Complete SSH exit-signal metadata from the remote channel.
///
/// This backend result remains the source of truth even when an application
/// later projects it into a narrower process-status or metrics schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshExitSignal {
    /// SSH signal name such as `TERM`.
    pub name: String,
    /// Whether the remote process reported producing a core dump.
    pub core_dumped: bool,
    /// Optional remote diagnostic text; empty when the server omitted it.
    pub error_message: String,
    /// Language tag associated with `error_message`.
    pub lang_tag: String,
}

/// Complete remote session termination result.
///
/// SSH status and signal events are independent and may both be present.
/// Repeated events overwrite the previous event of the same kind without
/// clearing the other kind, so no exit-signal metadata is lost at this layer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SshSessionResult {
    /// Last remote numeric exit status, when reported.
    pub exit_status: Option<u32>,
    /// Last complete remote exit signal, when reported.
    pub exit_signal: Option<SshExitSignal>,
}

/// Bounded local-input message consumed by the full-duplex shell runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshInputEvent {
    /// Bytes to write to the remote input stream.
    Data(Vec<u8>),
    /// Resize the remote PTY without writing terminal bytes.
    Resize(TerminalSize),
    /// Cancel the active connection and stop both input and output pumps.
    Cancel,
    /// Local input ended cleanly; send SSH channel EOF.
    Eof,
    /// Local input failed before EOF.
    Error(String),
}

/// Receiving half of a bounded SSH local-input event channel.
pub struct SshInputEventReceiver {
    receiver: std::sync::mpsc::Receiver<SshInputEvent>,
}

/// Creates a bounded local-input channel for the full-duplex shell runner.
///
/// A zero capacity request is normalized to one because the native stdin
/// broker must never use an unbounded or rendezvous-only queue.
#[must_use]
pub fn ssh_input_event_channel(
    capacity: usize,
) -> (
    std::sync::mpsc::SyncSender<SshInputEvent>,
    SshInputEventReceiver,
) {
    let (sender, receiver) = std::sync::mpsc::sync_channel(capacity.max(1));
    (sender, SshInputEventReceiver { receiver })
}

/// Remote exit metadata and exact byte counts from a completed SSH pump.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SshSessionOutcome {
    /// Remote exit status and signal metadata observed before channel close.
    pub result: SshSessionResult,
    /// Bytes accepted by the remote writer.
    pub input_bytes: u64,
    /// Bytes successfully written to the local output sink.
    pub output_bytes: u64,
}

pub trait SshShellConnector {
    /// Connects to an SSH server and starts the requested shell, command, or
    /// no-shell session.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when connecting, authenticating, requesting
    /// the remote PTY, or starting the requested channel mode fails.
    fn connect(
        &mut self,
        request: SshConnectRequest,
    ) -> Result<Box<dyn SshShellSession>, SshSessionError>;
}

pub trait SshShellReader: Send {
    /// Reads bytes from the remote shell channel into `buffer`.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the adapter cannot read from the active
    /// SSH channel.
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, SshSessionError>;

    /// Reads while observing an out-of-band runtime cancellation request.
    ///
    /// Backends whose read can block must override this method. `Ok(None)`
    /// means cancellation won and no bytes were read.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the active channel read fails.
    fn read_cancellable(
        &mut self,
        buffer: &mut [u8],
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<Option<usize>, SshSessionError> {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            Ok(None)
        } else {
            self.read(buffer).map(Some)
        }
    }

    /// Returns the final remote status observed while draining the channel.
    #[must_use]
    fn session_result(&self) -> SshSessionResult;
}

pub trait SshShellWriter: Send {
    /// Writes bytes to the remote shell channel.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the adapter cannot write to or flush the
    /// active SSH channel.
    fn write(&mut self, bytes: &[u8]) -> Result<usize, SshSessionError>;

    /// Writes bytes while observing runner cancellation.
    ///
    /// The compatibility default checks cancellation before entering the
    /// legacy blocking [`Self::write`] method. Backends whose write operation
    /// can wait on remote flow control must override this method and interrupt
    /// that wait when `cancelled` becomes true.
    ///
    /// `Ok(None)` means the write was cancelled and the caller should stop
    /// pumping input. `Ok(Some(count))` has the same meaning as [`Self::write`].
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the adapter cannot write to the active
    /// SSH channel.
    fn write_cancellable(
        &mut self,
        bytes: &[u8],
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<Option<usize>, SshSessionError> {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            Ok(None)
        } else {
            self.write(bytes).map(Some)
        }
    }

    /// Resizes the remote PTY attached to the shell channel.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the remote PTY resize request fails.
    fn resize(&mut self, size: TerminalSize) -> Result<(), SshSessionError>;

    /// Resizes the remote PTY while observing runner cancellation.
    ///
    /// The compatibility default checks cancellation before entering the
    /// legacy blocking [`Self::resize`] method. Backends whose resize request
    /// can wait on remote flow control must override this method and interrupt
    /// that wait when `cancelled` becomes true.
    ///
    /// `Ok(None)` means the resize was cancelled. `Ok(Some(()))` means it was
    /// accepted by the remote writer.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the remote PTY resize request fails.
    fn resize_cancellable(
        &mut self,
        size: TerminalSize,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<Option<()>, SshSessionError> {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            Ok(None)
        } else {
            self.resize(size).map(Some)
        }
    }

    /// Sends a keepalive through the underlying SSH connection.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the keepalive request fails.
    fn keepalive(&mut self) -> Result<(), SshSessionError>;

    /// Sends EOF for the local-input direction without closing the channel.
    ///
    /// The remote side may still send trailing data and exit metadata after
    /// receiving this half-close.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the backend cannot send channel EOF.
    fn finish_input(&mut self) -> Result<(), SshSessionError>;

    /// Closes the remote shell session.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the adapter cannot close the SSH channel
    /// cleanly.
    fn close(&mut self) -> Result<(), SshSessionError>;
}

pub trait SshShellSession: Send {
    /// Compatibility read entrypoint for callers that still own an unsplit
    /// session. Concurrent runners use [`Self::into_read_writer`].
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the backend cannot read the channel.
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, SshSessionError>;

    /// Compatibility write entrypoint for callers that still own an unsplit
    /// session.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the backend cannot write the channel.
    fn write(&mut self, bytes: &[u8]) -> Result<usize, SshSessionError>;

    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the backend rejects the PTY resize.
    fn resize(&mut self, size: TerminalSize) -> Result<(), SshSessionError>;

    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the backend cannot send a keepalive.
    fn keepalive(&mut self) -> Result<(), SshSessionError>;

    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the backend cannot close the channel.
    fn close(&mut self) -> Result<(), SshSessionError>;

    /// Splits the remote channel into independently owned read and write halves.
    ///
    /// The split contract permits the local-input pump and remote-output pump
    /// to block independently without serializing the whole session. This
    /// compatibility default shares the legacy session behind a mutex and
    /// therefore provides source compatibility only: it does not guarantee
    /// full-duplex progress. A backend used with the concurrent runner must
    /// override this method with genuinely independent native halves to make
    /// that guarantee.
    #[must_use]
    fn into_read_writer(self: Box<Self>) -> (Box<dyn SshShellReader>, Box<dyn SshShellWriter>)
    where
        Self: 'static,
    {
        let session = std::sync::Arc::new(std::sync::Mutex::new(self));
        (
            Box::new(SharedSessionReader {
                session: std::sync::Arc::clone(&session),
            }),
            Box::new(SharedSessionWriter { session }),
        )
    }
}

pub trait SshChannel: Send {
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the backend cannot read the channel.
    fn read_channel(&mut self, buffer: &mut [u8]) -> Result<usize, SshSessionError>;

    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the backend cannot write the channel.
    fn write_channel(&mut self, bytes: &[u8]) -> Result<usize, SshSessionError>;

    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the backend rejects the PTY resize.
    fn resize_pty(&mut self, size: TerminalSize) -> Result<(), SshSessionError>;

    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the backend cannot send a keepalive.
    fn send_keepalive(&mut self) -> Result<(), SshSessionError>;

    /// # Errors
    ///
    /// Returns [`SshSessionError`] when the backend cannot close the channel.
    fn close_channel(&mut self) -> Result<(), SshSessionError>;

    /// Splits an established backend channel into independently owned halves.
    /// The compatibility default shares the channel behind a mutex and is
    /// source-compatible, but does not guarantee full-duplex progress. A
    /// backend used with the concurrent runner must override this method with
    /// genuinely independent native halves.
    #[must_use]
    fn into_read_writer(self) -> (Box<dyn SshShellReader>, Box<dyn SshShellWriter>)
    where
        Self: Sized + 'static,
    {
        let channel = std::sync::Arc::new(std::sync::Mutex::new(self));
        (
            Box::new(SharedChannelReader {
                channel: std::sync::Arc::clone(&channel),
            }),
            Box::new(SharedChannelWriter { channel }),
        )
    }
}

struct SharedSessionReader<T: SshShellSession + ?Sized> {
    session: std::sync::Arc<std::sync::Mutex<Box<T>>>,
}

impl<T: SshShellSession + ?Sized> SshShellReader for SharedSessionReader<T> {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, SshSessionError> {
        self.session
            .lock()
            .map_err(|_| SshSessionError::new("shared SSH session lock poisoned"))?
            .read(buffer)
    }

    fn session_result(&self) -> SshSessionResult {
        SshSessionResult::default()
    }
}

struct SharedSessionWriter<T: SshShellSession + ?Sized> {
    session: std::sync::Arc<std::sync::Mutex<Box<T>>>,
}

impl<T: SshShellSession + ?Sized> SshShellWriter for SharedSessionWriter<T> {
    fn write(&mut self, bytes: &[u8]) -> Result<usize, SshSessionError> {
        self.session
            .lock()
            .map_err(|_| SshSessionError::new("shared SSH session lock poisoned"))?
            .write(bytes)
    }

    fn resize(&mut self, size: TerminalSize) -> Result<(), SshSessionError> {
        self.session
            .lock()
            .map_err(|_| SshSessionError::new("shared SSH session lock poisoned"))?
            .resize(size)
    }

    fn keepalive(&mut self) -> Result<(), SshSessionError> {
        self.session
            .lock()
            .map_err(|_| SshSessionError::new("shared SSH session lock poisoned"))?
            .keepalive()
    }

    fn finish_input(&mut self) -> Result<(), SshSessionError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), SshSessionError> {
        self.session
            .lock()
            .map_err(|_| SshSessionError::new("shared SSH session lock poisoned"))?
            .close()
    }
}

struct SharedChannelReader<C: SshChannel> {
    channel: std::sync::Arc<std::sync::Mutex<C>>,
}

impl<C: SshChannel> SshShellReader for SharedChannelReader<C> {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, SshSessionError> {
        self.channel
            .lock()
            .map_err(|_| SshSessionError::new("shared SSH channel lock poisoned"))?
            .read_channel(buffer)
    }

    fn session_result(&self) -> SshSessionResult {
        SshSessionResult::default()
    }
}

struct SharedChannelWriter<C: SshChannel> {
    channel: std::sync::Arc<std::sync::Mutex<C>>,
}

impl<C: SshChannel> SshShellWriter for SharedChannelWriter<C> {
    fn write(&mut self, bytes: &[u8]) -> Result<usize, SshSessionError> {
        self.channel
            .lock()
            .map_err(|_| SshSessionError::new("shared SSH channel lock poisoned"))?
            .write_channel(bytes)
    }

    fn resize(&mut self, size: TerminalSize) -> Result<(), SshSessionError> {
        self.channel
            .lock()
            .map_err(|_| SshSessionError::new("shared SSH channel lock poisoned"))?
            .resize_pty(size)
    }

    fn keepalive(&mut self) -> Result<(), SshSessionError> {
        self.channel
            .lock()
            .map_err(|_| SshSessionError::new("shared SSH channel lock poisoned"))?
            .send_keepalive()
    }

    fn finish_input(&mut self) -> Result<(), SshSessionError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), SshSessionError> {
        self.channel
            .lock()
            .map_err(|_| SshSessionError::new("shared SSH channel lock poisoned"))?
            .close_channel()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshChannelOpenPlan {
    pub pty_size: Option<TerminalSize>,
    pub startup: SshSessionStartup,
}

impl SshChannelOpenPlan {
    #[must_use]
    pub fn from_request(request: &SshConnectRequest) -> Self {
        let pty_size = match request.startup {
            SshSessionStartup::Shell | SshSessionStartup::Command(_) => {
                Some(request.config.initial_size)
            }
            SshSessionStartup::NoShell => None,
        };

        Self {
            pty_size,
            startup: request.startup.clone(),
        }
    }
}

pub struct SshChannelSession<C> {
    channel: C,
}

impl<C> SshChannelSession<C> {
    #[must_use]
    pub const fn new(channel: C) -> Self {
        Self { channel }
    }

    #[must_use]
    pub fn into_channel(self) -> C {
        self.channel
    }
}

impl<C> SshShellSession for SshChannelSession<C>
where
    C: SshChannel,
{
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, SshSessionError> {
        self.channel.read_channel(buffer)
    }

    fn write(&mut self, bytes: &[u8]) -> Result<usize, SshSessionError> {
        self.channel.write_channel(bytes)
    }

    fn resize(&mut self, size: TerminalSize) -> Result<(), SshSessionError> {
        self.channel.resize_pty(size)
    }

    fn keepalive(&mut self) -> Result<(), SshSessionError> {
        self.channel.send_keepalive()
    }

    fn close(&mut self) -> Result<(), SshSessionError> {
        self.channel.close_channel()
    }

    fn into_read_writer(self: Box<Self>) -> (Box<dyn SshShellReader>, Box<dyn SshShellWriter>)
    where
        Self: 'static,
    {
        self.channel.into_read_writer()
    }
}

pub trait SshChannelOpener {
    type Channel: SshChannel;

    /// Opens an authenticated remote channel for the requested SSH session.
    ///
    /// # Errors
    ///
    /// Returns [`SshSessionError`] when connecting, authenticating, requesting a
    /// PTY, or starting the requested shell, command, or no-shell channel
    /// fails.
    fn open_channel(
        &mut self,
        request: SshConnectRequest,
    ) -> Result<Self::Channel, SshSessionError>;
}

pub struct SshChannelConnector<O> {
    opener: O,
}

impl<O> SshChannelConnector<O> {
    #[must_use]
    pub const fn new(opener: O) -> Self {
        Self { opener }
    }

    #[must_use]
    pub fn into_opener(self) -> O {
        self.opener
    }
}

impl<O> SshShellConnector for SshChannelConnector<O>
where
    O: SshChannelOpener,
    O::Channel: 'static,
{
    fn connect(
        &mut self,
        request: SshConnectRequest,
    ) -> Result<Box<dyn SshShellSession>, SshSessionError> {
        let channel = self.opener.open_channel(request)?;
        Ok(Box::new(SshChannelSession::new(channel)))
    }
}

/// Runs the legacy sequential borrowed-I/O adapter.
///
/// This compatibility API copies all input before reading remote output. It is
/// not full duplex and does not return remote exit metadata. New code should
/// use [`run_connected_shell_with_events`].
///
/// # Errors
///
/// Returns [`SshSessionError`] when connecting, reading local input, writing to
/// the SSH session, reading remote output, writing output, or closing the
/// session fails.
pub fn run_shell_with_io(
    connector: &mut dyn SshShellConnector,
    request: SshConnectRequest,
    input: &mut dyn std::io::Read,
    output: &mut dyn std::io::Write,
) -> Result<(), SshSessionError> {
    let mut session = connector.connect(request)?;
    copy_input_to_legacy_session(input, session.as_mut())?;

    let mut buffer = [0; 8192];
    loop {
        let count = session.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        output
            .write_all(&buffer[..count])
            .map_err(|error| SshSessionError::new(error.to_string()))?;
        output
            .flush()
            .map_err(|error| SshSessionError::new(error.to_string()))?;
    }
    session.close()
}

#[derive(Clone)]
struct SshCancellation {
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl SshCancellation {
    fn new() -> Self {
        Self {
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    fn cancel(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::Release);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::Acquire)
    }

    fn flag(&self) -> &std::sync::atomic::AtomicBool {
        self.cancelled.as_ref()
    }
}

enum SshInputPoll {
    Event(SshInputEvent),
    Cancelled,
}

fn input_event_after_wakeup(event: SshInputEvent, cancellation: &SshCancellation) -> SshInputPoll {
    if cancellation.is_cancelled() {
        SshInputPoll::Cancelled
    } else {
        SshInputPoll::Event(event)
    }
}

impl SshInputEventReceiver {
    fn recv_cancellable(&self, cancellation: &SshCancellation) -> SshInputPoll {
        loop {
            if cancellation.is_cancelled() {
                return SshInputPoll::Cancelled;
            }
            let event = match self
                .receiver
                .recv_timeout(std::time::Duration::from_millis(20))
            {
                Ok(event) => event,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => SshInputEvent::Eof,
            };
            return input_event_after_wakeup(event, cancellation);
        }
    }
}

/// Pumps an already connected session using cancellable bounded input events.
///
/// Remote Close or output failure cancels an idle input receiver, so joining
/// the writer pump does not depend on an arbitrary blocking [`std::io::Read`].
/// Full-duplex progress additionally requires the backend to override
/// [`SshShellSession::into_read_writer`] with independent native halves. If its
/// writes can wait on remote flow control, its writer must also override
/// [`SshShellWriter::write_cancellable`] and
/// [`SshShellWriter::resize_cancellable`]. The compatibility defaults
/// preserve existing implementations but do not provide these concurrency
/// guarantees.
///
/// # Errors
///
/// Returns [`SshSessionError`] for input events, channel I/O, output I/O, or
/// channel shutdown failures.
pub fn run_connected_shell_with_events(
    session: Box<dyn SshShellSession>,
    input: SshInputEventReceiver,
    output: &mut dyn std::io::Write,
) -> Result<SshSessionOutcome, SshSessionError> {
    let (reader, writer) = session.into_read_writer();
    run_split_shell_with_events(reader, writer, input, output)
}

/// Pumps independently owned SSH reader and writer halves.
///
/// This is the runtime-adapter entry point corresponding to
/// [`run_connected_shell_with_events`]. It preserves the same cancellation,
/// partial-write, close-order, and exit-metadata behavior after transport
/// ownership has already been split.
///
/// # Errors
///
/// Returns [`SshSessionError`] for input events, channel I/O, output I/O, or
/// channel shutdown failures.
pub fn run_split_shell_with_events(
    mut reader: Box<dyn SshShellReader>,
    mut writer: Box<dyn SshShellWriter>,
    input: SshInputEventReceiver,
    output: &mut dyn std::io::Write,
) -> Result<SshSessionOutcome, SshSessionError> {
    let cancellation = SshCancellation::new();
    let writer_cancellation = cancellation.clone();

    std::thread::scope(move |scope| {
        let input_pump = scope.spawn(move || {
            let input_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                pump_input_events(&input, &writer_cancellation, writer.as_mut())
            }))
            .unwrap_or_else(|_| Err(SshSessionError::new("SSH input pump panicked")));
            let closed_on_error = input_result.is_err();
            let emergency_close_result = closed_on_error.then(|| writer.close());
            (
                writer,
                input_result,
                closed_on_error,
                emergency_close_result,
            )
        });

        let mut output_bytes = 0;
        let output_result = copy_session_to_output(reader.as_mut(), output, &mut output_bytes);
        cancellation.cancel();
        let Ok((mut writer, input_result, closed_on_error, emergency_close_result)) =
            input_pump.join()
        else {
            output_result?;
            return Err(SshSessionError::new("SSH input pump panicked"));
        };
        let close_result = if closed_on_error {
            emergency_close_result.unwrap_or(Ok(()))
        } else {
            writer.close()
        };

        output_result?;
        let input_bytes = input_result?;
        close_result?;
        Ok(SshSessionOutcome {
            result: reader.session_result(),
            input_bytes,
            output_bytes,
        })
    })
}

fn pump_input_events(
    input: &SshInputEventReceiver,
    cancellation: &SshCancellation,
    writer: &mut dyn SshShellWriter,
) -> Result<u64, SshSessionError> {
    let mut input_bytes = 0_u64;
    loop {
        match input.recv_cancellable(cancellation) {
            SshInputPoll::Cancelled => return Ok(input_bytes),
            SshInputPoll::Event(SshInputEvent::Data(bytes)) => {
                if !write_all_to_shell(writer, &bytes, cancellation)? {
                    return Ok(input_bytes);
                }
                input_bytes = input_bytes.saturating_add(bytes.len() as u64);
            }
            SshInputPoll::Event(SshInputEvent::Resize(size)) => {
                if writer
                    .resize_cancellable(size, cancellation.flag())?
                    .is_none()
                {
                    return Ok(input_bytes);
                }
            }
            SshInputPoll::Event(SshInputEvent::Cancel) => {
                cancellation.cancel();
                writer.close()?;
                return Ok(input_bytes);
            }
            SshInputPoll::Event(SshInputEvent::Eof) => {
                writer.finish_input()?;
                return Ok(input_bytes);
            }
            SshInputPoll::Event(SshInputEvent::Error(message)) => {
                return Err(SshSessionError::new(message));
            }
        }
    }
}

fn write_all_to_shell(
    writer: &mut dyn SshShellWriter,
    bytes: &[u8],
    cancellation: &SshCancellation,
) -> Result<bool, SshSessionError> {
    let mut written = 0;
    while written < bytes.len() {
        let Some(next) = writer.write_cancellable(&bytes[written..], cancellation.flag())? else {
            return Ok(false);
        };
        if next == 0 {
            return Err(SshSessionError::new(
                "SSH session write returned zero bytes",
            ));
        }
        written += next;
    }
    Ok(true)
}

fn copy_input_to_legacy_session(
    input: &mut dyn std::io::Read,
    session: &mut dyn SshShellSession,
) -> Result<(), SshSessionError> {
    let mut buffer = [0; 8192];

    loop {
        let count = input
            .read(&mut buffer)
            .map_err(|error| SshSessionError::new(error.to_string()))?;
        if count == 0 {
            return Ok(());
        }

        let mut written = 0;
        while written < count {
            let next = session.write(&buffer[written..count])?;
            if next == 0 {
                return Err(SshSessionError::new(
                    "SSH session write returned zero bytes",
                ));
            }
            written += next;
        }
    }
}

fn copy_session_to_output(
    reader: &mut dyn SshShellReader,
    output: &mut dyn std::io::Write,
    output_bytes: &mut u64,
) -> Result<(), SshSessionError> {
    let mut buffer = [0; 8192];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            return Ok(());
        }

        output
            .write_all(&buffer[..count])
            .map_err(|error| SshSessionError::new(error.to_string()))?;
        output
            .flush()
            .map_err(|error| SshSessionError::new(error.to_string()))?;
        *output_bytes = (*output_bytes).saturating_add(count as u64);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io,
        path::PathBuf,
        sync::{
            Arc, Condvar, Mutex,
            mpsc::{self, SyncSender},
        },
        time::Duration,
    };

    use rterm_types::TerminalSize;

    use super::{
        SshAuthError, SshAuthMethod, SshChannel, SshChannelConnector, SshChannelOpenPlan,
        SshChannelOpener, SshChannelSession, SshConnectRequest, SshInputEvent, SshSessionConfig,
        SshSessionResult, SshSessionStartup, SshShellConnector, SshShellReader, SshShellSession,
        SshShellWriter, SshStartupError, ssh_input_event_channel,
    };

    use crate::{
        AsyncHostKeyVerifier, HostKeyChallenge, HostKeyDecision, HostKeyStatus, HostKeyVerifier,
        RusshAuthOutcome, RusshAuthPlan, RusshAuthRequest, RusshChannelStartupPlan,
        RusshChannelStartupRequest, RusshConnectPlan, RusshHostKeyPolicy, RusshKnownHosts,
        RusshPrivateKeyAuth, RusshRemoteTcpIpForwardPlan, SshConnectionPhase,
    };

    #[test]
    fn host_key_prompt_policy_exposes_a_non_secret_challenge() {
        let challenge = HostKeyChallenge::new(
            "ssh.example.com",
            2222,
            "ssh-ed25519",
            "SHA256:example",
            HostKeyStatus::Unknown,
        )
        .with_known_hosts_path("C:/Users/test/.ssh/known_hosts");

        assert_eq!(challenge.host, "ssh.example.com");
        assert_eq!(challenge.port, 2222);
        assert_eq!(challenge.algorithm, "ssh-ed25519");
        assert_eq!(challenge.fingerprint, "SHA256:example");
        assert_eq!(challenge.status, HostKeyStatus::Unknown);
        assert_eq!(
            challenge.known_hosts_path.as_deref(),
            Some(std::path::Path::new("C:/Users/test/.ssh/known_hosts"))
        );
        assert!(!format!("{challenge:?}").contains("password"));
    }

    #[test]
    fn async_host_key_verifier_can_choose_accept_once() {
        let verifier = HostKeyVerifier::new(|challenge: HostKeyChallenge| async move {
            assert_eq!(challenge.status, HostKeyStatus::Unknown);
            HostKeyDecision::AcceptOnce
        });
        let challenge = HostKeyChallenge::new(
            "ssh.example.com",
            22,
            "ssh-ed25519",
            "SHA256:example",
            HostKeyStatus::Unknown,
        );
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let decision = runtime.block_on(verifier.verify(challenge));

        assert_eq!(decision, HostKeyDecision::AcceptOnce);
    }

    #[test]
    fn host_key_verifier_accepts_shared_async_trait_objects() {
        let callback: Arc<dyn AsyncHostKeyVerifier> =
            Arc::new(|_challenge: HostKeyChallenge| async move { HostKeyDecision::AcceptOnce });
        let verifier = HostKeyVerifier::new(callback);
        let challenge = HostKeyChallenge::new(
            "ssh.example.com",
            22,
            "ssh-ed25519",
            "SHA256:example",
            HostKeyStatus::Unknown,
        );
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        assert_eq!(
            runtime.block_on(verifier.verify(challenge)),
            HostKeyDecision::AcceptOnce
        );
    }

    #[test]
    fn input_events_include_resize_and_cancel_without_losing_legacy_variants() {
        let resize = SshInputEvent::Resize(TerminalSize::new(120, 40));
        let cancel = SshInputEvent::Cancel;

        assert_eq!(resize, SshInputEvent::Resize(TerminalSize::new(120, 40)));
        assert_eq!(cancel, SshInputEvent::Cancel);
        assert_eq!(
            SshInputEvent::Data(vec![1, 2]),
            SshInputEvent::Data(vec![1, 2])
        );
    }

    #[test]
    fn input_event_pump_applies_resize_and_stops_on_cancel() {
        let state = Arc::new(Mutex::new(MockRunnerState::default()));
        let mut writer = MockRunnerWriter {
            state: Arc::clone(&state),
        };
        let (input_tx, input_rx) = ssh_input_event_channel(3);
        input_tx
            .send(SshInputEvent::Resize(TerminalSize::new(132, 43)))
            .unwrap();
        input_tx.send(SshInputEvent::Data(b"abc".to_vec())).unwrap();
        input_tx.send(SshInputEvent::Cancel).unwrap();

        let cancellation = super::SshCancellation::new();
        let input_bytes = super::pump_input_events(&input_rx, &cancellation, &mut writer).unwrap();

        let state = state.lock().unwrap();
        assert_eq!(input_bytes, 3);
        assert_eq!(state.written, b"abc");
        assert_eq!(state.resized, vec![TerminalSize::new(132, 43)]);
        assert!(state.closed);
        assert!(cancellation.is_cancelled());
    }

    #[test]
    fn phase_reporter_can_observe_connection_lifecycle() {
        let phases = Arc::new(Mutex::new(Vec::new()));
        let observed = Arc::clone(&phases);
        let opener = super::RusshChannelOpener::default().with_phase_reporter(move |phase| {
            observed.lock().unwrap().push(phase);
        });

        // The reporter is intentionally exercised through the public callback
        // helper so callers can wire it before the first network operation.
        opener.report_phase(SshConnectionPhase::Connecting);

        assert_eq!(
            phases.lock().unwrap().as_slice(),
            &[SshConnectionPhase::Connecting]
        );
    }

    #[test]
    fn session_config_keeps_terminal_size() {
        let config = SshSessionConfig::new("example.com", 22, "ops", TerminalSize::new(100, 40));

        assert_eq!(config.host, "example.com");
        assert_eq!(config.port, 22);
        assert_eq!(config.username, "ops");
        assert_eq!(config.initial_size, TerminalSize::new(100, 40));
    }

    #[test]
    fn session_config_try_new_trims_and_validates_fields() {
        let config =
            SshSessionConfig::try_new(" example.com ", 22, " ops ", TerminalSize::new(100, 40))
                .unwrap();

        assert_eq!(config.host, "example.com");
        assert_eq!(config.port, 22);
        assert_eq!(config.username, "ops");
        assert_eq!(config.initial_size, TerminalSize::new(100, 40));
    }

    #[test]
    fn session_config_rejects_empty_host() {
        assert_eq!(
            SshSessionConfig::try_new(" ", 22, "ops", TerminalSize::new(100, 40)),
            Err(super::SshConfigError::EmptyHost)
        );
    }

    #[test]
    fn session_config_rejects_empty_username() {
        assert_eq!(
            SshSessionConfig::try_new("example.com", 22, " ", TerminalSize::new(100, 40)),
            Err(super::SshConfigError::EmptyUsername)
        );
    }

    #[test]
    fn session_config_rejects_zero_port() {
        assert_eq!(
            SshSessionConfig::try_new("example.com", 0, "ops", TerminalSize::new(100, 40)),
            Err(super::SshConfigError::InvalidPort)
        );
    }

    #[test]
    fn session_config_rejects_zero_terminal_dimensions() {
        assert_eq!(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(0, 40)),
            Err(super::SshConfigError::InvalidTerminalSize)
        );
        assert_eq!(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(100, 0)),
            Err(super::SshConfigError::InvalidTerminalSize)
        );
    }

    #[test]
    fn ssh_shell_session_trait_models_channel_io_and_resize() {
        let mut session = MockSshSession::default();
        let mut output = [0; 4];

        assert_eq!(session.read(&mut output).unwrap(), 4);
        assert_eq!(&output, b"pong");
        assert_eq!(session.write(b"ping").unwrap(), 4);
        session.resize(TerminalSize::new(120, 30)).unwrap();
        session.keepalive().unwrap();
        session.close().unwrap();

        assert_eq!(session.written, b"ping");
        assert_eq!(session.size, TerminalSize::new(120, 30));
        assert!(session.kept_alive);
        assert!(session.closed);
    }

    #[test]
    fn legacy_shell_runner_accepts_non_send_reader_and_returns_unit() {
        let request = SshConnectRequest::agent(valid_config());
        let state = Arc::new(Mutex::new(MockRunnerState::default()));
        let mut connector = MockRunnerConnector {
            state: Arc::clone(&state),
        };
        let shared = std::rc::Rc::new(std::cell::RefCell::new(io::Cursor::new(Vec::<u8>::new())));
        let mut input = NonSendReader { shared };
        let mut output = Vec::new();

        let result: Result<(), super::SshSessionError> =
            super::run_shell_with_io(&mut connector, request, &mut input, &mut output);

        result.unwrap();
        assert_eq!(output, b"remote\n");
    }

    #[test]
    fn legacy_session_and_channel_implementers_do_not_need_split_methods() {
        fn assert_session<T: SshShellSession>() {}
        fn assert_channel<T: SshChannel>() {}

        assert_session::<LegacyCompatibleSession>();
        assert_channel::<LegacyCompatibleChannel>();
    }

    #[test]
    fn ssh_channel_session_delegates_shell_io_and_lifecycle() {
        let channel = MockSshChannel::new(b"pong".to_vec());
        let mut session = SshChannelSession::new(channel);
        let mut output = [0; 4];

        assert_eq!(session.read(&mut output).unwrap(), 4);
        assert_eq!(&output, b"pong");
        assert_eq!(session.write(b"ping").unwrap(), 4);
        session.resize(TerminalSize::new(132, 43)).unwrap();
        session.keepalive().unwrap();
        session.close().unwrap();

        let channel = session.into_channel();
        assert_eq!(channel.written, b"ping");
        assert_eq!(channel.sizes, vec![TerminalSize::new(132, 43)]);
        assert_eq!(channel.keepalives, 1);
        assert!(channel.closed);
    }

    #[test]
    fn ssh_channel_connector_opens_channel_and_returns_shell_session() {
        let request = SshConnectRequest::agent(valid_config());
        let mut connector = SshChannelConnector::new(MockSshOpener::new(b"pong".to_vec()));

        let mut session = connector.connect(request.clone()).unwrap();
        let mut output = [0; 4];

        assert_eq!(session.read(&mut output).unwrap(), 4);
        assert_eq!(&output, b"pong");
        assert_eq!(session.write(b"ping").unwrap(), 4);
        session.resize(TerminalSize::new(132, 43)).unwrap();
        session.keepalive().unwrap();
        session.close().unwrap();

        let opener = connector.into_opener();
        assert_eq!(opener.last_request, Some(request));
        let channel = opener.recorded.lock().unwrap().take().unwrap();
        assert_eq!(channel.written, b"ping");
        assert_eq!(channel.sizes, vec![TerminalSize::new(132, 43)]);
        assert_eq!(channel.keepalives, 1);
        assert!(channel.closed);
    }

    #[test]
    fn connect_request_accepts_password_auth() {
        let config = valid_config();
        let request = SshConnectRequest::password(config.clone(), "secret").unwrap();

        assert_eq!(request.config, config);
        assert_eq!(
            request.auth,
            SshAuthMethod::Password {
                password: "secret".to_owned()
            }
        );
    }

    #[test]
    fn ssh_auth_debug_redacts_passwords_and_private_key_passphrases() {
        let password = format!("credential-{}", std::process::id());
        let passphrase = format!(
            "phrase-{}",
            std::thread::current().name().unwrap_or("unnamed")
        );

        let password_request =
            SshConnectRequest::password(valid_config(), password.clone()).unwrap();
        let private_key_request = SshConnectRequest::private_key(
            valid_config(),
            PathBuf::from("C:/Users/ops/.ssh/id_ed25519"),
            Some(passphrase.clone()),
        )
        .unwrap();
        let password_plan = RusshAuthPlan::from_request(&password_request);
        let private_key_plan = RusshAuthPlan::from_request(&private_key_request);

        for (rendered, secret) in [
            (format!("{:?}", password_request.auth), password.as_str()),
            (format!("{password_request:?}"), password.as_str()),
            (format!("{password_plan:?}"), password.as_str()),
            (
                format!("{:?}", private_key_request.auth),
                passphrase.as_str(),
            ),
            (format!("{private_key_request:?}"), passphrase.as_str()),
            (format!("{private_key_plan:?}"), passphrase.as_str()),
        ] {
            assert!(
                !rendered.contains(secret),
                "SSH auth Debug output leaked a secret"
            );
            assert!(
                rendered.contains("<redacted>"),
                "SSH auth Debug output omitted the redaction marker"
            );
        }
    }

    #[test]
    fn connect_request_rejects_empty_password() {
        let error = SshConnectRequest::password(valid_config(), " ").unwrap_err();

        assert_eq!(error, SshAuthError::EmptyPassword);
    }

    #[test]
    fn connect_request_accepts_password_prompt_auth() {
        let config = valid_config();
        let request = SshConnectRequest::password_prompt(config.clone());

        assert_eq!(request.config, config);
        assert_eq!(request.auth, SshAuthMethod::PasswordPrompt);
    }

    #[test]
    fn connect_request_accepts_private_key_auth() {
        let config = valid_config();
        let request = SshConnectRequest::private_key(
            config.clone(),
            PathBuf::from("C:/Users/ops/.ssh/id_ed25519"),
            Some("secret"),
        )
        .unwrap();

        assert_eq!(request.config, config);
        assert_eq!(
            request.auth,
            SshAuthMethod::PrivateKey {
                path: PathBuf::from("C:/Users/ops/.ssh/id_ed25519"),
                passphrase: Some("secret".to_owned())
            }
        );
    }

    #[test]
    fn connect_request_rejects_empty_private_key_path() {
        let error = SshConnectRequest::private_key(valid_config(), PathBuf::new(), None::<String>)
            .unwrap_err();

        assert_eq!(error, SshAuthError::EmptyPrivateKeyPath);
    }

    #[test]
    fn connect_request_accepts_agent_auth() {
        let config = valid_config();
        let request = SshConnectRequest::agent(config.clone());

        assert_eq!(request.config, config);
        assert_eq!(request.auth, SshAuthMethod::Agent);
    }

    #[test]
    fn connect_request_defaults_to_shell_startup() {
        let request = SshConnectRequest::agent(valid_config());

        assert_eq!(request.startup, SshSessionStartup::Shell);
    }

    #[test]
    fn connect_request_accepts_remote_command_startup() {
        let request = SshConnectRequest::agent(valid_config()).with_startup(
            SshSessionStartup::command(["uname".to_owned(), "-a".to_owned()]).unwrap(),
        );

        assert_eq!(
            request.startup,
            SshSessionStartup::Command(vec!["uname".to_owned(), "-a".to_owned()])
        );
    }

    #[test]
    fn session_startup_rejects_empty_remote_command() {
        assert_eq!(
            SshSessionStartup::command(Vec::<String>::new()),
            Err(SshStartupError::EmptyCommand)
        );
    }

    #[test]
    fn channel_open_plan_requests_pty_for_shell_startup() {
        let request = SshConnectRequest::agent(valid_config());

        let plan = SshChannelOpenPlan::from_request(&request);

        assert_eq!(plan.pty_size, Some(TerminalSize::new(100, 40)));
        assert_eq!(plan.startup, SshSessionStartup::Shell);
    }

    #[test]
    fn channel_open_plan_requests_pty_for_remote_command_startup() {
        let request = SshConnectRequest::agent(valid_config()).with_startup(
            SshSessionStartup::command(["uname".to_owned(), "-a".to_owned()]).unwrap(),
        );

        let plan = SshChannelOpenPlan::from_request(&request);

        assert_eq!(plan.pty_size, Some(TerminalSize::new(100, 40)));
        assert_eq!(
            plan.startup,
            SshSessionStartup::Command(vec!["uname".to_owned(), "-a".to_owned()])
        );
    }

    #[test]
    fn channel_open_plan_skips_pty_for_no_shell_startup() {
        let request =
            SshConnectRequest::agent(valid_config()).with_startup(SshSessionStartup::NoShell);

        let plan = SshChannelOpenPlan::from_request(&request);

        assert_eq!(plan.pty_size, None);
        assert_eq!(plan.startup, SshSessionStartup::NoShell);
    }

    #[test]
    fn ssh_shell_connector_trait_creates_shell_session_from_request() {
        let request = SshConnectRequest::agent(valid_config());
        let mut connector = MockSshConnector::default();

        let mut session = connector.connect(request.clone()).unwrap();
        let mut output = [0; 4];

        assert_eq!(session.read(&mut output).unwrap(), 4);
        assert_eq!(&output, b"pong");
        assert_eq!(session.write(b"ping").unwrap(), 4);
        assert_eq!(connector.last_request, Some(request));
    }

    #[test]
    fn shell_runner_streams_remote_output_and_closes_session() {
        let request = SshConnectRequest::agent(valid_config());
        let state = Arc::new(Mutex::new(MockRunnerState::default()));
        let mut connector = MockRunnerConnector {
            state: Arc::clone(&state),
        };
        let mut output = Vec::new();

        super::run_shell_with_io(
            &mut connector,
            request.clone(),
            &mut io::empty(),
            &mut output,
        )
        .unwrap();

        let state = state.lock().unwrap();
        assert_eq!(state.last_request.as_ref(), Some(&request));
        assert_eq!(output, b"remote\n");
        assert!(state.closed);
    }

    #[test]
    fn shell_runner_writes_local_input_to_remote_session() {
        let request = SshConnectRequest::agent(valid_config());
        let state = Arc::new(Mutex::new(MockRunnerState::default()));
        let mut connector = MockRunnerConnector {
            state: Arc::clone(&state),
        };
        let mut input = &b"echo hi\n"[..];
        let mut output = Vec::new();

        super::run_shell_with_io(&mut connector, request, &mut input, &mut output).unwrap();

        let state = state.lock().unwrap();
        assert_eq!(state.written, b"echo hi\n");
        assert_eq!(output, b"remote\n");
        assert!(state.closed);
    }

    #[test]
    fn shell_runner_streams_output_before_input_eof() {
        let request = SshConnectRequest::agent(valid_config());
        let state = Arc::new(Mutex::new(MockRunnerState::default()));
        let mut connector = MockRunnerConnector {
            state: Arc::clone(&state),
        };
        let session = connector.connect(request).unwrap();
        let (input_tx, input_rx) = ssh_input_event_channel(1);
        let (output_tx, output_rx) = mpsc::sync_channel(1);
        let mut output = ChannelOutput {
            sender: output_tx,
            pending: Vec::new(),
        };
        let (done_tx, done_rx) = mpsc::sync_channel(1);

        let worker = std::thread::spawn(move || {
            let result = super::run_connected_shell_with_events(session, input_rx, &mut output);
            done_tx.send(result).unwrap();
        });

        let remote_output = output_rx.recv_timeout(Duration::from_millis(250));
        let _ = input_tx.send(SshInputEvent::Eof);
        done_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("shell runner did not finish after input EOF")
            .unwrap();
        worker.join().unwrap();

        assert_eq!(
            remote_output.expect("remote output was blocked on local input EOF"),
            b"remote\n"
        );
    }

    #[test]
    fn shell_runner_remote_close_cancels_open_input() {
        let request = SshConnectRequest::agent(valid_config());
        let state = Arc::new(Mutex::new(MockRunnerState::default()));
        let mut connector = MockRunnerConnector {
            state: Arc::clone(&state),
        };
        let session = connector.connect(request).unwrap();
        let (input_tx, input_rx) = ssh_input_event_channel(1);
        let mut output = Vec::new();
        let (done_tx, done_rx) = mpsc::sync_channel(1);

        let worker = std::thread::spawn(move || {
            let result = super::run_connected_shell_with_events(session, input_rx, &mut output);
            done_tx.send(result).unwrap();
        });

        let completed_while_input_open = done_rx
            .recv_timeout(Duration::from_millis(250))
            .expect("remote Close did not cancel open input");
        worker.join().unwrap();
        drop(input_tx);

        completed_while_input_open.unwrap();
    }

    #[test]
    fn input_receiver_prioritizes_cancellation_over_queued_events() {
        let (input_tx, input_rx) = ssh_input_event_channel(1);
        input_tx.send(SshInputEvent::Eof).unwrap();
        let cancellation = super::SshCancellation::new();
        cancellation.cancel();

        assert!(matches!(
            input_rx.recv_cancellable(&cancellation),
            super::SshInputPoll::Cancelled
        ));
    }

    #[test]
    fn input_receiver_post_wakeup_arbitration_prefers_cancellation() {
        let cancellation = super::SshCancellation::new();
        cancellation.cancel();

        assert!(matches!(
            super::input_event_after_wakeup(SshInputEvent::Data(vec![1]), &cancellation),
            super::SshInputPoll::Cancelled
        ));
        assert!(matches!(
            super::input_event_after_wakeup(SshInputEvent::Eof, &cancellation),
            super::SshInputPoll::Cancelled
        ));
        assert!(matches!(
            super::input_event_after_wakeup(
                SshInputEvent::Error("late input error".to_owned()),
                &cancellation,
            ),
            super::SshInputPoll::Cancelled
        ));
    }

    #[test]
    fn shell_runner_output_broken_pipe_cancels_open_input() {
        let request = SshConnectRequest::agent(valid_config());
        let state = Arc::new(Mutex::new(MockRunnerState::default()));
        let mut connector = MockRunnerConnector {
            state: Arc::clone(&state),
        };
        let session = connector.connect(request).unwrap();
        let (input_tx, input_rx) = ssh_input_event_channel(1);
        let mut output = FailingOutput;
        let (done_tx, done_rx) = mpsc::sync_channel(1);

        let worker = std::thread::spawn(move || {
            let result = super::run_connected_shell_with_events(session, input_rx, &mut output);
            done_tx.send(result).unwrap();
        });

        let error = done_rx
            .recv_timeout(Duration::from_millis(250))
            .expect("BrokenPipe did not cancel open input")
            .unwrap_err();
        worker.join().unwrap();
        drop(input_tx);

        assert_eq!(error.to_string(), "output failure");
    }

    #[test]
    fn shell_runner_remote_close_cancels_blocked_writer() {
        let session = BlockingWriteSession::new(false);
        let state = Arc::clone(&session.state);
        let (input_tx, input_rx) = ssh_input_event_channel(1);
        input_tx
            .send(SshInputEvent::Data(b"blocked write".to_vec()))
            .unwrap();
        let (done_tx, done_rx) = mpsc::sync_channel(1);

        let worker = std::thread::spawn(move || {
            let mut output = Vec::new();
            done_tx
                .send(super::run_connected_shell_with_events(
                    Box::new(session),
                    input_rx,
                    &mut output,
                ))
                .unwrap();
        });

        done_rx
            .recv_timeout(Duration::from_millis(250))
            .expect("remote Close did not cancel a blocked writer")
            .unwrap();
        worker.join().unwrap();
        drop(input_tx);
        assert!(state.0.lock().unwrap().closed);
    }

    #[test]
    fn shell_runner_output_error_cancels_blocked_writer() {
        let session = BlockingWriteSession::new(true);
        let state = Arc::clone(&session.state);
        let (input_tx, input_rx) = ssh_input_event_channel(1);
        input_tx
            .send(SshInputEvent::Data(b"blocked write".to_vec()))
            .unwrap();
        let (done_tx, done_rx) = mpsc::sync_channel(1);

        let worker = std::thread::spawn(move || {
            let mut output = FailingOutput;
            done_tx
                .send(super::run_connected_shell_with_events(
                    Box::new(session),
                    input_rx,
                    &mut output,
                ))
                .unwrap();
        });

        let error = done_rx
            .recv_timeout(Duration::from_millis(250))
            .expect("output failure did not cancel a blocked writer")
            .unwrap_err();
        worker.join().unwrap();
        drop(input_tx);
        assert_eq!(error.to_string(), "output failure");
        assert!(state.0.lock().unwrap().closed);
    }

    #[test]
    fn cancellable_resize_unblocks_when_cancelled_out_of_band() {
        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        let (done_tx, done_rx) = mpsc::sync_channel(1);

        let worker = std::thread::spawn(move || {
            let mut writer = BlockingResizeWriter {
                started: started_tx,
            };
            done_tx
                .send(
                    writer
                        .resize_cancellable(TerminalSize::new(132, 43), worker_cancelled.as_ref()),
                )
                .unwrap();
        });

        started_rx
            .recv_timeout(Duration::from_millis(250))
            .expect("fake resize did not start");
        cancelled.store(true, std::sync::atomic::Ordering::Release);
        assert_eq!(
            done_rx
                .recv_timeout(Duration::from_millis(250))
                .expect("cancellation did not unblock the fake resize")
                .unwrap(),
            None
        );
        worker.join().unwrap();
    }

    #[test]
    fn shell_event_pump_dispatches_resize_through_cancellable_api() {
        let observed = Arc::new(Mutex::new(Vec::new()));
        let mut writer = CancellableResizeWriter {
            observed: Arc::clone(&observed),
        };
        let (input_tx, input_rx) = ssh_input_event_channel(2);
        let expected = TerminalSize::new(120, 37);
        input_tx.send(SshInputEvent::Resize(expected)).unwrap();
        input_tx.send(SshInputEvent::Cancel).unwrap();

        let cancellation = super::SshCancellation::new();
        assert_eq!(
            super::pump_input_events(&input_rx, &cancellation, &mut writer).unwrap(),
            0
        );
        assert_eq!(*observed.lock().unwrap(), vec![expected]);
    }

    #[test]
    fn cancellable_resize_default_preserves_legacy_writers() {
        let state = Arc::new(Mutex::new(MockRunnerState::default()));
        let mut writer = MockRunnerWriter {
            state: Arc::clone(&state),
        };
        let cancelled = std::sync::atomic::AtomicBool::new(false);
        let expected = TerminalSize::new(101, 29);

        assert_eq!(
            writer.resize_cancellable(expected, &cancelled).unwrap(),
            Some(())
        );
        cancelled.store(true, std::sync::atomic::Ordering::Release);
        assert_eq!(
            writer
                .resize_cancellable(TerminalSize::new(102, 30), &cancelled)
                .unwrap(),
            None
        );
        assert_eq!(state.lock().unwrap().resized, vec![expected]);
    }

    #[test]
    fn shell_runner_drains_late_output_and_status_after_input_eof() {
        let request = SshConnectRequest::agent(valid_config());
        let shared = Arc::new((Mutex::new(false), Condvar::new()));
        let mut connector = EofAwareConnector {
            shared: Arc::clone(&shared),
        };
        let session = connector.connect(request).unwrap();
        let (input_tx, input_rx) = ssh_input_event_channel(2);
        input_tx
            .send(SshInputEvent::Data(b"request\n".to_vec()))
            .unwrap();
        input_tx.send(SshInputEvent::Eof).unwrap();
        let (done_tx, done_rx) = mpsc::sync_channel(1);

        let worker = std::thread::spawn(move || {
            let mut output = Vec::new();
            let result = super::run_connected_shell_with_events(session, input_rx, &mut output);
            done_tx.send((result, output)).unwrap();
        });

        let (result, output) = done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("runner did not drain output after local input EOF");
        worker.join().unwrap();

        assert_eq!(output, b"late\n");
        assert_eq!(result.unwrap().result.exit_status, Some(37));
    }

    #[test]
    fn shell_runner_closes_channel_when_finish_input_fails() {
        let state = Arc::new((Mutex::new(FaultState::default()), Condvar::new()));
        let mut connector = FaultConnector {
            state: Arc::clone(&state),
            emit_output: false,
        };
        let request = SshConnectRequest::agent(valid_config());
        let session = connector.connect(request).unwrap();
        let (input_tx, input_rx) = ssh_input_event_channel(1);
        input_tx.send(SshInputEvent::Eof).unwrap();
        let (done_tx, done_rx) = mpsc::sync_channel(1);

        let worker = std::thread::spawn(move || {
            let mut output = Vec::new();
            let result = super::run_connected_shell_with_events(session, input_rx, &mut output)
                .map_err(|error| error.to_string());
            done_tx.send(result).unwrap();
        });

        let error = done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("finish-input failure left the output pump blocked")
            .unwrap_err();
        worker.join().unwrap();

        assert_eq!(error, "finish input failure");
        let state = state.0.lock().unwrap();
        assert!(state.finish_called);
        assert!(state.closed);
    }

    #[test]
    fn shell_runner_prioritizes_output_error_over_input_finish_and_close_errors() {
        let state = Arc::new((Mutex::new(FaultState::default()), Condvar::new()));
        let mut connector = FaultConnector {
            state: Arc::clone(&state),
            emit_output: true,
        };
        let request = SshConnectRequest::agent(valid_config());
        let session = connector.connect(request).unwrap();
        let (input_tx, input_rx) = ssh_input_event_channel(1);
        input_tx
            .send(SshInputEvent::Error("input failure".to_owned()))
            .unwrap();
        let (done_tx, done_rx) = mpsc::sync_channel(1);

        let worker = std::thread::spawn(move || {
            let mut output = FailingOutput;
            let result = super::run_connected_shell_with_events(session, input_rx, &mut output)
                .map_err(|error| error.to_string());
            done_tx.send(result).unwrap();
        });

        let error = done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("fault-injected pumps did not finish")
            .unwrap_err();
        worker.join().unwrap();

        assert_eq!(error, "output failure");
        assert!(state.0.lock().unwrap().closed);
    }

    #[test]
    fn russh_channel_opener_exposes_client_config() {
        let opener = super::RusshChannelOpener::default();

        let config = opener.client_config();

        assert!(config.keepalive_interval.is_some());
    }

    #[test]
    fn russh_channel_opener_defaults_to_rejecting_unknown_host_keys() {
        let opener = super::RusshChannelOpener::default();

        assert_eq!(opener.host_key_policy(), RusshHostKeyPolicy::RejectUnknown);
    }

    #[test]
    fn russh_channel_opener_can_be_configured_to_accept_unknown_host_keys() {
        let opener = super::RusshChannelOpener::default()
            .with_host_key_policy(RusshHostKeyPolicy::AcceptUnknown);

        assert_eq!(opener.host_key_policy(), RusshHostKeyPolicy::AcceptUnknown);
        assert!(opener.into_handler().accepts_unknown_host_keys());
    }

    #[test]
    fn russh_known_hosts_can_learn_and_match_host_key() {
        let path = temp_known_hosts_path("learn");
        let _ = std::fs::remove_file(&path);
        let store = RusshKnownHosts::new(path.clone());
        let key = test_public_key();

        store.learn("ssh.example.com", 2222, &key).unwrap();

        assert!(store.matches("ssh.example.com", 2222, &key).unwrap());
        assert!(!store.matches("ssh.example.com", 22, &key).unwrap());

        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.starts_with("[ssh.example.com]:2222 ssh-ed25519 "));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn russh_handler_accepts_known_host_key_from_configured_store() {
        let path = temp_known_hosts_path("handler");
        let _ = std::fs::remove_file(&path);
        let store = RusshKnownHosts::new(path.clone());
        let key = test_public_key();
        store.learn("ssh.example.com", 22, &key).unwrap();
        let mut handler = super::RusshChannelOpener::default()
            .with_known_hosts_path(path.clone())
            .handler_for_host("ssh.example.com", 22);

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let accepted = runtime
            .block_on(russh::client::Handler::check_server_key(&mut handler, &key))
            .unwrap();

        assert!(accepted);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn russh_handler_trusts_first_unknown_host_key_and_records_it() {
        let path = temp_known_hosts_path("trust-first");
        let _ = std::fs::remove_file(&path);
        let key = test_public_key();
        let mut handler = super::RusshChannelOpener::default()
            .with_host_key_policy(RusshHostKeyPolicy::TrustOnFirstUse)
            .with_known_hosts_path(path.clone())
            .handler_for_host("ssh.example.com", 2222);

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let accepted = runtime
            .block_on(russh::client::Handler::check_server_key(&mut handler, &key))
            .unwrap();

        assert!(accepted);
        assert!(
            RusshKnownHosts::new(path.clone())
                .matches("ssh.example.com", 2222, &key)
                .unwrap()
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn russh_handler_prompt_can_accept_once_or_store_unknown_host_key() {
        let path = temp_known_hosts_path("prompt-store");
        let _ = std::fs::remove_file(&path);
        let key = test_public_key();
        let verifier = HostKeyVerifier::new(|challenge: HostKeyChallenge| async move {
            assert_eq!(challenge.status, HostKeyStatus::Unknown);
            assert_eq!(challenge.port, 2222);
            HostKeyDecision::AcceptAndStore
        });
        let mut handler = super::RusshChannelOpener::default()
            .with_host_key_policy(RusshHostKeyPolicy::Prompt)
            .with_host_key_verifier(verifier)
            .with_known_hosts_path(path.clone())
            .handler_for_host("ssh.example.com", 2222);

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let accepted = runtime
            .block_on(russh::client::Handler::check_server_key(&mut handler, &key))
            .unwrap();

        assert!(accepted);
        assert!(
            RusshKnownHosts::new(path.clone())
                .matches("ssh.example.com", 2222, &key)
                .unwrap()
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn russh_handler_prompt_rejects_changed_host_key_even_when_verifier_accepts() {
        let path = temp_known_hosts_path("prompt-changed");
        let _ = std::fs::remove_file(&path);
        let known_key = test_public_key();
        let changed_key = test_public_key_alt();
        RusshKnownHosts::new(path.clone())
            .learn("ssh.example.com", 2222, &known_key)
            .unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        for decision in [HostKeyDecision::AcceptOnce, HostKeyDecision::AcceptAndStore] {
            let challenges = Arc::new(Mutex::new(Vec::new()));
            let observed_challenges = Arc::clone(&challenges);
            let verifier = HostKeyVerifier::new(move |challenge: HostKeyChallenge| {
                let observed_challenges = Arc::clone(&observed_challenges);
                async move {
                    observed_challenges.lock().unwrap().push(challenge);
                    decision
                }
            });
            let mut handler = super::RusshChannelOpener::default()
                .with_host_key_policy(RusshHostKeyPolicy::Prompt)
                .with_host_key_verifier(verifier)
                .with_known_hosts_path(path.clone())
                .handler_for_host("ssh.example.com", 2222);

            let accepted = runtime
                .block_on(russh::client::Handler::check_server_key(
                    &mut handler,
                    &changed_key,
                ))
                .unwrap();

            assert!(!accepted);
            let challenges = challenges.lock().unwrap();
            assert_eq!(challenges.len(), 1);
            assert_eq!(challenges[0].host, "ssh.example.com");
            assert_eq!(challenges[0].port, 2222);
            assert_eq!(challenges[0].status, HostKeyStatus::Changed);
            assert_eq!(
                challenges[0].fingerprint,
                changed_key
                    .fingerprint(russh::keys::HashAlg::Sha256)
                    .to_string()
            );
            assert_eq!(challenges[0].known_hosts_path.as_ref(), Some(&path));
        }
        assert!(
            RusshKnownHosts::new(path.clone())
                .matches("ssh.example.com", 2222, &known_key)
                .unwrap()
        );
        assert!(
            RusshKnownHosts::new(path.clone())
                .matches("ssh.example.com", 2222, &changed_key)
                .is_err()
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn russh_connect_plan_builds_socket_address_and_username_from_request() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new(
                "ssh.example.com",
                2222,
                "deploy",
                TerminalSize::new(120, 30),
            )
            .unwrap(),
        );

        let plan = RusshConnectPlan::from_request(&request);

        assert_eq!(plan.socket_addr(), ("ssh.example.com", 2222));
        assert_eq!(plan.username(), "deploy");
    }

    #[test]
    fn russh_connect_plan_carries_channel_open_plan_from_request() {
        let request = SshConnectRequest::agent(valid_config())
            .with_startup(SshSessionStartup::command(["uptime".to_owned()]).unwrap());

        let plan = RusshConnectPlan::from_request(&request);

        assert_eq!(
            plan.channel_open_plan(),
            &SshChannelOpenPlan {
                pty_size: Some(TerminalSize::new(100, 40)),
                startup: SshSessionStartup::Command(vec!["uptime".to_owned()])
            }
        );
    }

    #[test]
    fn russh_channel_opener_builds_connect_plan_for_request() {
        let request =
            SshConnectRequest::agent(valid_config()).with_startup(SshSessionStartup::NoShell);
        let opener = super::RusshChannelOpener::default();

        let plan = opener.connect_plan(&request);

        assert_eq!(plan.socket_addr(), ("example.com", 22));
        assert_eq!(plan.username(), "ops");
        assert_eq!(
            plan.channel_open_plan(),
            &SshChannelOpenPlan {
                pty_size: None,
                startup: SshSessionStartup::NoShell
            }
        );
    }

    #[test]
    fn russh_channel_opener_exposes_async_connect_entrypoint() {
        let opener = super::RusshChannelOpener::default();
        let request = SshConnectRequest::agent(valid_config());

        let future = opener.connect_async(request);

        drop(future);
    }

    #[test]
    fn russh_connect_plan_derives_channel_startup_plan() {
        let request = SshConnectRequest::agent(valid_config());
        let plan = RusshConnectPlan::from_request(&request);

        assert_eq!(
            plan.channel_startup_plan().requests(),
            &[
                RusshChannelStartupRequest::RequestPty {
                    term: "xterm-256color".to_owned(),
                    columns: 100,
                    rows: 40,
                    pixel_width: 0,
                    pixel_height: 0,
                },
                RusshChannelStartupRequest::RequestShell
            ]
        );
    }

    #[test]
    fn russh_auth_plan_maps_password_value_authentication() {
        let request = SshConnectRequest::password(valid_config(), "secret").unwrap();

        let plan = RusshAuthPlan::from_request(&request);

        assert_eq!(plan.username(), "ops");
        assert_eq!(
            plan.request(),
            &RusshAuthRequest::Password {
                password: "secret".to_owned()
            }
        );
    }

    #[test]
    fn russh_auth_plan_maps_password_prompt_authentication() {
        let request = SshConnectRequest::password_prompt(valid_config());

        let plan = RusshAuthPlan::from_request(&request);

        assert_eq!(plan.username(), "ops");
        assert_eq!(plan.request(), &RusshAuthRequest::PasswordPrompt);
    }

    #[test]
    fn russh_auth_plan_maps_private_key_authentication() {
        let request = SshConnectRequest::private_key(
            valid_config(),
            PathBuf::from("C:/Users/ops/.ssh/id_ed25519"),
            Some("secret"),
        )
        .unwrap();

        let plan = RusshAuthPlan::from_request(&request);

        assert_eq!(plan.username(), "ops");
        assert_eq!(
            plan.request(),
            &RusshAuthRequest::PrivateKey {
                path: PathBuf::from("C:/Users/ops/.ssh/id_ed25519"),
                passphrase: Some("secret".to_owned())
            }
        );
    }

    #[test]
    fn russh_private_key_auth_loads_unencrypted_private_key_file() {
        let path = temp_private_key_path("plain");
        std::fs::write(&path, TEST_ED25519_PRIVATE_KEY).unwrap();

        let auth = RusshPrivateKeyAuth::load(&path, None).unwrap();

        assert_eq!(auth.algorithm(), russh::keys::ssh_key::Algorithm::Ed25519);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn russh_private_key_auth_loads_encrypted_private_key_file_with_passphrase() {
        let path = temp_private_key_path("encrypted");
        std::fs::write(&path, TEST_ENCRYPTED_ED25519_PRIVATE_KEY).unwrap();

        let auth = RusshPrivateKeyAuth::load(&path, Some("test")).unwrap();

        assert_eq!(auth.algorithm(), russh::keys::ssh_key::Algorithm::Ed25519);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn russh_private_key_auth_detects_encrypted_private_key_file() {
        let path = temp_private_key_path("encrypted-needs-passphrase");
        std::fs::write(&path, TEST_ENCRYPTED_ED25519_PRIVATE_KEY).unwrap();

        assert!(RusshPrivateKeyAuth::needs_passphrase(&path).unwrap());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn russh_private_key_auth_reports_unencrypted_private_key_file_does_not_need_passphrase() {
        let path = temp_private_key_path("plain-no-passphrase");
        std::fs::write(&path, TEST_ED25519_PRIVATE_KEY).unwrap();

        assert!(!RusshPrivateKeyAuth::needs_passphrase(&path).unwrap());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn russh_auth_plan_maps_agent_authentication() {
        let request = SshConnectRequest::agent(valid_config());

        let plan = RusshAuthPlan::from_request(&request);

        assert_eq!(plan.username(), "ops");
        assert_eq!(plan.request(), &RusshAuthRequest::Agent);
    }

    #[test]
    fn russh_connect_plan_derives_auth_plan() {
        let request = SshConnectRequest::agent(valid_config());
        let plan = RusshConnectPlan::from_request(&request);

        assert_eq!(plan.auth_plan(), &RusshAuthPlan::from_request(&request));
    }

    #[test]
    fn russh_direct_tcpip_plan_carries_target_and_originator_endpoint() {
        let plan = super::RusshDirectTcpIpOpenPlan::new("db.internal", 5432, "127.0.0.1", 15432);

        assert_eq!(plan.target(), ("db.internal", 5432));
        assert_eq!(plan.originator(), ("127.0.0.1", 15432));
    }

    #[test]
    fn russh_remote_tcpip_forward_plan_carries_bind_and_target_endpoint() {
        let plan = RusshRemoteTcpIpForwardPlan::new("127.0.0.1", 8080, "127.0.0.1", 80);

        assert_eq!(plan.bind(), ("127.0.0.1", 8080));
        assert_eq!(plan.target(), ("127.0.0.1", 80));
    }

    #[test]
    fn russh_auth_outcome_accepts_successful_authentication() {
        let outcome =
            RusshAuthOutcome::from_auth_result(&russh::client::AuthResult::Success).unwrap();

        assert_eq!(outcome, RusshAuthOutcome::Authenticated);
    }

    #[test]
    fn russh_auth_outcome_rejects_failed_authentication() {
        let error = RusshAuthOutcome::from_auth_result(&russh::client::AuthResult::Failure {
            remaining_methods: russh::MethodSet::empty(),
            partial_success: false,
        })
        .unwrap_err();

        assert!(error.to_string().contains("SSH authentication failed"));
    }

    #[test]
    fn russh_channel_opener_exposes_async_authentication_entrypoint() {
        let authenticate = super::RusshChannelOpener::authenticate_async;

        let _ = authenticate;
    }

    #[test]
    fn russh_channel_opener_exposes_async_session_channel_entrypoint() {
        let open_channel = super::RusshChannelOpener::open_session_channel_async;

        let _ = open_channel;
    }

    #[test]
    fn russh_channel_opener_exposes_async_direct_tcpip_entrypoint() {
        let open_channel = super::RusshChannelOpener::open_direct_tcpip_channel_async;

        let _ = open_channel;
    }

    #[test]
    fn russh_channel_opener_exposes_blocking_direct_tcpip_entrypoint() {
        let open_channel = super::RusshChannelOpener::open_direct_tcpip_channel;

        let _ = open_channel;
    }

    #[test]
    fn russh_channel_opener_exposes_blocking_remote_tcpip_forward_entrypoint() {
        let start_forward = super::RusshChannelOpener::start_remote_tcpip_forward;

        let _ = start_forward;
    }

    #[test]
    fn russh_channel_opener_exposes_async_channel_startup_entrypoint() {
        let start_channel = super::RusshChannelOpener::start_channel_async;

        let _ = start_channel;
    }

    #[test]
    fn russh_ssh_channel_implements_ssh_channel_trait() {
        fn assert_ssh_channel<T: SshChannel>() {}

        assert_ssh_channel::<super::RusshSshChannel>();
    }

    #[test]
    fn russh_ssh_channel_exposes_live_channel_constructor() {
        let new_channel = super::RusshSshChannel::new;

        let _ = new_channel;
    }

    #[test]
    fn russh_ssh_channel_exposes_split_io_entrypoint() {
        let split_io = super::RusshSshChannel::into_read_writer;

        let _ = split_io;
    }

    #[test]
    fn russh_channel_opener_implements_ssh_channel_opener_trait() {
        fn assert_channel_opener<T: SshChannelOpener<Channel = super::RusshSshChannel>>() {}

        assert_channel_opener::<super::RusshChannelOpener>();
    }

    #[test]
    fn russh_channel_opener_is_cloneable_for_forward_listeners() {
        fn assert_clone<T: Clone>() {}

        assert_clone::<super::RusshChannelOpener>();
    }

    #[test]
    fn russh_channel_startup_plan_requests_pty_then_shell_for_shell_startup() {
        let open_plan = SshChannelOpenPlan {
            pty_size: Some(TerminalSize::new(120, 30)),
            startup: SshSessionStartup::Shell,
        };

        let plan = RusshChannelStartupPlan::from_open_plan(&open_plan);

        assert_eq!(
            plan.requests(),
            &[
                RusshChannelStartupRequest::RequestPty {
                    term: "xterm-256color".to_owned(),
                    columns: 120,
                    rows: 30,
                    pixel_width: 0,
                    pixel_height: 0,
                },
                RusshChannelStartupRequest::RequestShell
            ]
        );
    }

    #[test]
    fn russh_channel_startup_plan_requests_pty_then_exec_for_remote_command() {
        let open_plan = SshChannelOpenPlan {
            pty_size: Some(TerminalSize::new(100, 40)),
            startup: SshSessionStartup::Command(vec!["uptime".to_owned()]),
        };

        let plan = RusshChannelStartupPlan::from_open_plan(&open_plan);

        assert_eq!(
            plan.requests(),
            &[
                RusshChannelStartupRequest::RequestPty {
                    term: "xterm-256color".to_owned(),
                    columns: 100,
                    rows: 40,
                    pixel_width: 0,
                    pixel_height: 0,
                },
                RusshChannelStartupRequest::Exec {
                    command: "'uptime'".to_owned()
                }
            ]
        );
    }

    #[test]
    fn russh_remote_command_preserves_posix_argument_boundaries() {
        let open_plan = SshChannelOpenPlan {
            pty_size: None,
            startup: SshSessionStartup::Command(vec![
                "printf".to_owned(),
                "%s".to_owned(),
                "a b".to_owned(),
                "\"quoted\"".to_owned(),
                "a;b".to_owned(),
                "$(id)".to_owned(),
                "line1\nline2".to_owned(),
                "'single quote'".to_owned(),
                String::new(),
            ]),
        };

        let plan = RusshChannelStartupPlan::from_open_plan(&open_plan);

        assert_eq!(
            plan.requests(),
            &[RusshChannelStartupRequest::Exec {
                command: "'printf' '%s' 'a b' '\"quoted\"' 'a;b' '$(id)' \
                          'line1\nline2' ''\"'\"'single quote'\"'\"'' ''"
                    .to_owned(),
            }]
        );
    }

    #[test]
    fn russh_channel_startup_plan_skips_channel_requests_for_no_shell_startup() {
        let open_plan = SshChannelOpenPlan {
            pty_size: None,
            startup: SshSessionStartup::NoShell,
        };

        let plan = RusshChannelStartupPlan::from_open_plan(&open_plan);

        assert_eq!(plan.requests(), &[]);
    }

    fn valid_config() -> SshSessionConfig {
        SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(100, 40)).unwrap()
    }

    fn temp_known_hosts_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "rterm-ssh-known-hosts-{name}-{}",
            std::process::id()
        ))
    }

    fn temp_private_key_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "rterm-ssh-private-key-{name}-{}",
            std::process::id()
        ))
    }

    fn test_public_key() -> russh::keys::ssh_key::PublicKey {
        russh::keys::parse_public_key_base64(
            "AAAAC3NzaC1lZDI1NTE5AAAAIJdD7y3aLq454yWBdwLWbieU1ebz9/cu7/QEXn9OIeZJ",
        )
        .unwrap()
    }

    fn test_public_key_alt() -> russh::keys::ssh_key::PublicKey {
        russh::keys::parse_public_key_base64(
            "AAAAC3NzaC1lZDI1NTE5AAAAILagOJFgwaMNhBWQINinKOXmqS4Gh5NgxgriXwdOoINJ",
        )
        .unwrap()
    }

    const TEST_ED25519_PRIVATE_KEY: &str = "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEINTuctv5E1hK1bbY8fdp+K06/nwoy/HU++CXqI9EdVhC\n-----END PRIVATE KEY-----\n";

    const TEST_ENCRYPTED_ED25519_PRIVATE_KEY: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAACmFlczI1Ni1jdHIAAAAGYmNyeXB0AAAAGAAAABD1phlku5\nA2G7Q9iP+DcOc9AAAAEAAAAAEAAAAzAAAAC3NzaC1lZDI1NTE5AAAAIHeLC1lWiCYrXsf/\n85O/pkbUFZ6OGIt49PX3nw8iRoXEAAAAkKRF0st5ZI7xxo9g6A4m4l6NarkQre3mycqNXQ\ndP3jryYgvsCIBAA5jMWSjrmnOTXhidqcOy4xYCrAttzSnZ/cUadfBenL+DQq6neffw7j8r\n0tbCxVGp6yCQlKrgSZf6c0Hy7dNEIU2bJFGxLe6/kWChcUAt/5Ll5rI7DVQPJdLgehLzvv\nsJWR7W+cGvJ/vLsw==\n-----END OPENSSH PRIVATE KEY-----\n";

    #[derive(Default)]
    struct MockSshConnector {
        last_request: Option<SshConnectRequest>,
    }

    impl SshShellConnector for MockSshConnector {
        fn connect(
            &mut self,
            request: SshConnectRequest,
        ) -> Result<Box<dyn SshShellSession>, super::SshSessionError> {
            self.last_request = Some(request);
            Ok(Box::new(MockSshSession::default()))
        }
    }

    struct MockSshSession {
        written: Vec<u8>,
        size: TerminalSize,
        kept_alive: bool,
        closed: bool,
    }

    struct NonSendReader {
        shared: std::rc::Rc<std::cell::RefCell<io::Cursor<Vec<u8>>>>,
    }

    impl io::Read for NonSendReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.shared.borrow_mut().read(buffer)
        }
    }

    struct LegacyCompatibleSession;

    impl SshShellSession for LegacyCompatibleSession {
        fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, super::SshSessionError> {
            Ok(0)
        }

        fn write(&mut self, bytes: &[u8]) -> Result<usize, super::SshSessionError> {
            Ok(bytes.len())
        }

        fn resize(&mut self, _size: TerminalSize) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn keepalive(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn close(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }
    }

    struct LegacyCompatibleChannel;

    impl SshChannel for LegacyCompatibleChannel {
        fn read_channel(&mut self, _buffer: &mut [u8]) -> Result<usize, super::SshSessionError> {
            Ok(0)
        }

        fn write_channel(&mut self, bytes: &[u8]) -> Result<usize, super::SshSessionError> {
            Ok(bytes.len())
        }

        fn resize_pty(&mut self, _size: TerminalSize) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn send_keepalive(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn close_channel(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }
    }

    impl Default for MockSshSession {
        fn default() -> Self {
            Self {
                written: Vec::new(),
                size: TerminalSize::new(80, 24),
                kept_alive: false,
                closed: false,
            }
        }
    }

    impl super::SshShellSession for MockSshSession {
        fn read(&mut self, buffer: &mut [u8]) -> Result<usize, super::SshSessionError> {
            buffer.copy_from_slice(b"pong");
            Ok(4)
        }

        fn write(&mut self, bytes: &[u8]) -> Result<usize, super::SshSessionError> {
            self.written.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn resize(&mut self, size: TerminalSize) -> Result<(), super::SshSessionError> {
            self.size = size;
            Ok(())
        }

        fn keepalive(&mut self) -> Result<(), super::SshSessionError> {
            self.kept_alive = true;
            Ok(())
        }

        fn close(&mut self) -> Result<(), super::SshSessionError> {
            self.closed = true;
            Ok(())
        }

        fn into_read_writer(self: Box<Self>) -> (Box<dyn SshShellReader>, Box<dyn SshShellWriter>) {
            (
                Box::new(MockChannelReader {
                    output: b"pong".to_vec(),
                }),
                Box::new(MockChannelWriter {
                    written: self.written,
                    sizes: Vec::new(),
                    keepalives: 0,
                    closed: self.closed,
                    recorded: None,
                }),
            )
        }
    }

    #[derive(Default)]
    struct MockRunnerState {
        last_request: Option<SshConnectRequest>,
        written: Vec<u8>,
        resized: Vec<TerminalSize>,
        closed: bool,
    }

    struct MockRunnerConnector {
        state: Arc<Mutex<MockRunnerState>>,
    }

    impl SshShellConnector for MockRunnerConnector {
        fn connect(
            &mut self,
            request: SshConnectRequest,
        ) -> Result<Box<dyn SshShellSession>, super::SshSessionError> {
            self.state.lock().unwrap().last_request = Some(request);
            Ok(Box::new(MockRunnerSession {
                state: Arc::clone(&self.state),
                read_once: false,
            }))
        }
    }

    struct MockRunnerSession {
        state: Arc<Mutex<MockRunnerState>>,
        read_once: bool,
    }

    struct ChannelOutput {
        sender: SyncSender<Vec<u8>>,
        pending: Vec<u8>,
    }

    impl io::Write for ChannelOutput {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.pending.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.sender
                .send(std::mem::take(&mut self.pending))
                .map_err(|error| io::Error::new(io::ErrorKind::BrokenPipe, error))?;
            Ok(())
        }
    }

    impl SshShellSession for MockRunnerSession {
        fn read(&mut self, buffer: &mut [u8]) -> Result<usize, super::SshSessionError> {
            if self.read_once {
                return Ok(0);
            }
            self.read_once = true;
            buffer[..7].copy_from_slice(b"remote\n");
            Ok(7)
        }

        fn write(&mut self, bytes: &[u8]) -> Result<usize, super::SshSessionError> {
            self.state.lock().unwrap().written.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn resize(&mut self, size: TerminalSize) -> Result<(), super::SshSessionError> {
            self.state.lock().unwrap().resized.push(size);
            Ok(())
        }

        fn keepalive(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn close(&mut self) -> Result<(), super::SshSessionError> {
            self.state.lock().unwrap().closed = true;
            Ok(())
        }

        fn into_read_writer(self: Box<Self>) -> (Box<dyn SshShellReader>, Box<dyn SshShellWriter>) {
            let writer = MockRunnerWriter {
                state: Arc::clone(&self.state),
            };
            (self, Box::new(writer))
        }
    }

    impl SshShellReader for MockRunnerSession {
        fn read(&mut self, buffer: &mut [u8]) -> Result<usize, super::SshSessionError> {
            SshShellSession::read(self, buffer)
        }

        fn session_result(&self) -> SshSessionResult {
            SshSessionResult::default()
        }
    }

    struct MockRunnerWriter {
        state: Arc<Mutex<MockRunnerState>>,
    }

    impl SshShellWriter for MockRunnerWriter {
        fn write(&mut self, bytes: &[u8]) -> Result<usize, super::SshSessionError> {
            self.state.lock().unwrap().written.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn resize(&mut self, size: TerminalSize) -> Result<(), super::SshSessionError> {
            self.state.lock().unwrap().resized.push(size);
            Ok(())
        }

        fn keepalive(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn finish_input(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn close(&mut self) -> Result<(), super::SshSessionError> {
            self.state.lock().unwrap().closed = true;
            Ok(())
        }
    }

    #[derive(Default)]
    struct BlockingWriteState {
        write_started: bool,
        closed: bool,
    }

    struct BlockingWriteSession {
        state: Arc<(Mutex<BlockingWriteState>, Condvar)>,
        emit_output: bool,
    }

    impl BlockingWriteSession {
        fn new(emit_output: bool) -> Self {
            Self {
                state: Arc::new((Mutex::new(BlockingWriteState::default()), Condvar::new())),
                emit_output,
            }
        }
    }

    impl SshShellSession for BlockingWriteSession {
        fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, super::SshSessionError> {
            unreachable!("blocking-write tests use the split reader")
        }

        fn write(&mut self, _bytes: &[u8]) -> Result<usize, super::SshSessionError> {
            unreachable!("blocking-write tests use the split writer")
        }

        fn resize(&mut self, _size: TerminalSize) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn keepalive(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn close(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn into_read_writer(self: Box<Self>) -> (Box<dyn SshShellReader>, Box<dyn SshShellWriter>) {
            (
                Box::new(BlockingWriteReader {
                    state: Arc::clone(&self.state),
                    emit_output: self.emit_output,
                    emitted: false,
                }),
                Box::new(BlockingWriteWriter { state: self.state }),
            )
        }
    }

    struct BlockingWriteReader {
        state: Arc<(Mutex<BlockingWriteState>, Condvar)>,
        emit_output: bool,
        emitted: bool,
    }

    impl SshShellReader for BlockingWriteReader {
        fn read(&mut self, buffer: &mut [u8]) -> Result<usize, super::SshSessionError> {
            let (state, wake) = &*self.state;
            let mut state = state.lock().unwrap();
            while !state.write_started {
                state = wake.wait(state).unwrap();
            }
            if self.emit_output && !self.emitted {
                self.emitted = true;
                buffer[0] = b'x';
                Ok(1)
            } else {
                Ok(0)
            }
        }

        fn session_result(&self) -> SshSessionResult {
            SshSessionResult::default()
        }
    }

    struct BlockingWriteWriter {
        state: Arc<(Mutex<BlockingWriteState>, Condvar)>,
    }

    impl SshShellWriter for BlockingWriteWriter {
        fn write(&mut self, _bytes: &[u8]) -> Result<usize, super::SshSessionError> {
            Err(super::SshSessionError::new(
                "runner bypassed cancellable SSH write",
            ))
        }

        fn write_cancellable(
            &mut self,
            _bytes: &[u8],
            cancelled: &std::sync::atomic::AtomicBool,
        ) -> Result<Option<usize>, super::SshSessionError> {
            let (state, wake) = &*self.state;
            {
                let mut state = state.lock().unwrap();
                state.write_started = true;
                wake.notify_all();
            }
            while !cancelled.load(std::sync::atomic::Ordering::Acquire) {
                std::thread::yield_now();
            }
            Ok(None)
        }

        fn resize(&mut self, _size: TerminalSize) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn keepalive(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn finish_input(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn close(&mut self) -> Result<(), super::SshSessionError> {
            let (state, wake) = &*self.state;
            state.lock().unwrap().closed = true;
            wake.notify_all();
            Ok(())
        }
    }

    struct BlockingResizeWriter {
        started: SyncSender<()>,
    }

    impl SshShellWriter for BlockingResizeWriter {
        fn write(&mut self, bytes: &[u8]) -> Result<usize, super::SshSessionError> {
            Ok(bytes.len())
        }

        fn resize(&mut self, _size: TerminalSize) -> Result<(), super::SshSessionError> {
            Err(super::SshSessionError::new(
                "caller bypassed cancellable SSH resize",
            ))
        }

        fn resize_cancellable(
            &mut self,
            _size: TerminalSize,
            cancelled: &std::sync::atomic::AtomicBool,
        ) -> Result<Option<()>, super::SshSessionError> {
            self.started.send(()).unwrap();
            while !cancelled.load(std::sync::atomic::Ordering::Acquire) {
                std::thread::yield_now();
            }
            Ok(None)
        }

        fn keepalive(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn finish_input(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn close(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }
    }

    struct CancellableResizeWriter {
        observed: Arc<Mutex<Vec<TerminalSize>>>,
    }

    impl SshShellWriter for CancellableResizeWriter {
        fn write(&mut self, bytes: &[u8]) -> Result<usize, super::SshSessionError> {
            Ok(bytes.len())
        }

        fn resize(&mut self, _size: TerminalSize) -> Result<(), super::SshSessionError> {
            Err(super::SshSessionError::new(
                "event pump bypassed cancellable SSH resize",
            ))
        }

        fn resize_cancellable(
            &mut self,
            size: TerminalSize,
            cancelled: &std::sync::atomic::AtomicBool,
        ) -> Result<Option<()>, super::SshSessionError> {
            if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                return Ok(None);
            }
            self.observed.lock().unwrap().push(size);
            Ok(Some(()))
        }

        fn keepalive(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn finish_input(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn close(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }
    }

    struct EofAwareConnector {
        shared: Arc<(Mutex<bool>, Condvar)>,
    }

    impl SshShellConnector for EofAwareConnector {
        fn connect(
            &mut self,
            _request: SshConnectRequest,
        ) -> Result<Box<dyn SshShellSession>, super::SshSessionError> {
            Ok(Box::new(EofAwareSession {
                shared: Arc::clone(&self.shared),
            }))
        }
    }

    struct EofAwareSession {
        shared: Arc<(Mutex<bool>, Condvar)>,
    }

    impl SshShellSession for EofAwareSession {
        fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, super::SshSessionError> {
            Err(super::SshSessionError::new("split session required"))
        }

        fn write(&mut self, _bytes: &[u8]) -> Result<usize, super::SshSessionError> {
            Err(super::SshSessionError::new("split session required"))
        }

        fn resize(&mut self, _size: TerminalSize) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn keepalive(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn close(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn into_read_writer(self: Box<Self>) -> (Box<dyn SshShellReader>, Box<dyn SshShellWriter>) {
            (
                Box::new(EofAwareReader {
                    shared: Arc::clone(&self.shared),
                    emitted: false,
                }),
                Box::new(EofAwareWriter {
                    shared: self.shared,
                }),
            )
        }
    }

    struct EofAwareReader {
        shared: Arc<(Mutex<bool>, Condvar)>,
        emitted: bool,
    }

    impl SshShellReader for EofAwareReader {
        fn read(&mut self, buffer: &mut [u8]) -> Result<usize, super::SshSessionError> {
            if self.emitted {
                return Ok(0);
            }

            let (input_finished, wake) = &*self.shared;
            let input_finished = input_finished.lock().unwrap();
            let (input_finished, timeout) = wake
                .wait_timeout_while(input_finished, Duration::from_secs(1), |finished| {
                    !*finished
                })
                .unwrap();
            if timeout.timed_out() || !*input_finished {
                return Err(super::SshSessionError::new(
                    "local input EOF was not sent before the deadline",
                ));
            }

            self.emitted = true;
            buffer[..5].copy_from_slice(b"late\n");
            Ok(5)
        }

        fn session_result(&self) -> SshSessionResult {
            SshSessionResult {
                exit_status: self.emitted.then_some(37),
                exit_signal: None,
            }
        }
    }

    struct EofAwareWriter {
        shared: Arc<(Mutex<bool>, Condvar)>,
    }

    #[derive(Default)]
    struct FaultState {
        finish_called: bool,
        closed: bool,
    }

    struct FaultConnector {
        state: Arc<(Mutex<FaultState>, Condvar)>,
        emit_output: bool,
    }

    impl SshShellConnector for FaultConnector {
        fn connect(
            &mut self,
            _request: SshConnectRequest,
        ) -> Result<Box<dyn SshShellSession>, super::SshSessionError> {
            Ok(Box::new(FaultSession {
                state: Arc::clone(&self.state),
                emit_output: self.emit_output,
            }))
        }
    }

    struct FaultSession {
        state: Arc<(Mutex<FaultState>, Condvar)>,
        emit_output: bool,
    }

    impl SshShellSession for FaultSession {
        fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, super::SshSessionError> {
            Err(super::SshSessionError::new("split session required"))
        }

        fn write(&mut self, _bytes: &[u8]) -> Result<usize, super::SshSessionError> {
            Err(super::SshSessionError::new("split session required"))
        }

        fn resize(&mut self, _size: TerminalSize) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn keepalive(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn close(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn into_read_writer(self: Box<Self>) -> (Box<dyn SshShellReader>, Box<dyn SshShellWriter>) {
            (
                Box::new(FaultReader {
                    state: Arc::clone(&self.state),
                    emit_output: self.emit_output,
                    emitted: false,
                }),
                Box::new(FaultWriter { state: self.state }),
            )
        }
    }

    struct FaultReader {
        state: Arc<(Mutex<FaultState>, Condvar)>,
        emit_output: bool,
        emitted: bool,
    }

    impl SshShellReader for FaultReader {
        fn read(&mut self, buffer: &mut [u8]) -> Result<usize, super::SshSessionError> {
            if self.emitted {
                return Ok(0);
            }

            let (state, wake) = &*self.state;
            let state = state.lock().unwrap();
            let (state, timeout) = wake
                .wait_timeout_while(state, Duration::from_secs(1), |state| !state.closed)
                .unwrap();
            if timeout.timed_out() || !state.closed {
                return Err(super::SshSessionError::new(
                    "reader remained blocked after input failure",
                ));
            }
            drop(state);

            self.emitted = true;
            if self.emit_output {
                buffer[..6].copy_from_slice(b"fault\n");
                Ok(6)
            } else {
                Ok(0)
            }
        }

        fn session_result(&self) -> SshSessionResult {
            SshSessionResult::default()
        }
    }

    struct FaultWriter {
        state: Arc<(Mutex<FaultState>, Condvar)>,
    }

    impl SshShellWriter for FaultWriter {
        fn write(&mut self, bytes: &[u8]) -> Result<usize, super::SshSessionError> {
            Ok(bytes.len())
        }

        fn resize(&mut self, _size: TerminalSize) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn keepalive(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn finish_input(&mut self) -> Result<(), super::SshSessionError> {
            self.state.0.lock().unwrap().finish_called = true;
            Err(super::SshSessionError::new("finish input failure"))
        }

        fn close(&mut self) -> Result<(), super::SshSessionError> {
            self.state.0.lock().unwrap().closed = true;
            self.state.1.notify_all();
            Err(super::SshSessionError::new("close failure"))
        }
    }

    struct FailingOutput;

    impl io::Write for FailingOutput {
        fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("output failure"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl SshShellWriter for EofAwareWriter {
        fn write(&mut self, bytes: &[u8]) -> Result<usize, super::SshSessionError> {
            Ok(bytes.len())
        }

        fn resize(&mut self, _size: TerminalSize) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn keepalive(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn finish_input(&mut self) -> Result<(), super::SshSessionError> {
            let (input_finished, wake) = &*self.shared;
            *input_finished.lock().unwrap() = true;
            wake.notify_all();
            Ok(())
        }

        fn close(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }
    }

    struct MockSshChannel {
        output: Vec<u8>,
        written: Vec<u8>,
        sizes: Vec<TerminalSize>,
        keepalives: u32,
        closed: bool,
    }

    impl MockSshChannel {
        fn new(output: Vec<u8>) -> Self {
            Self {
                output,
                written: Vec::new(),
                sizes: Vec::new(),
                keepalives: 0,
                closed: false,
            }
        }
    }

    impl SshChannel for MockSshChannel {
        fn read_channel(&mut self, buffer: &mut [u8]) -> Result<usize, super::SshSessionError> {
            let count = buffer.len().min(self.output.len());
            buffer[..count].copy_from_slice(&self.output[..count]);
            self.output.drain(..count);
            Ok(count)
        }

        fn write_channel(&mut self, bytes: &[u8]) -> Result<usize, super::SshSessionError> {
            self.written.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn resize_pty(&mut self, size: TerminalSize) -> Result<(), super::SshSessionError> {
            self.sizes.push(size);
            Ok(())
        }

        fn send_keepalive(&mut self) -> Result<(), super::SshSessionError> {
            self.keepalives += 1;
            Ok(())
        }

        fn close_channel(&mut self) -> Result<(), super::SshSessionError> {
            self.closed = true;
            Ok(())
        }

        fn into_read_writer(self) -> (Box<dyn SshShellReader>, Box<dyn SshShellWriter>) {
            split_mock_channel(self, None)
        }
    }

    struct MockSshOpener {
        output: Vec<u8>,
        last_request: Option<SshConnectRequest>,
        recorded: Arc<Mutex<Option<MockSshChannel>>>,
    }

    impl MockSshOpener {
        fn new(output: Vec<u8>) -> Self {
            Self {
                output,
                last_request: None,
                recorded: Arc::new(Mutex::new(None)),
            }
        }
    }

    impl SshChannelOpener for MockSshOpener {
        type Channel = RecordingSshChannel;

        fn open_channel(
            &mut self,
            request: SshConnectRequest,
        ) -> Result<Self::Channel, super::SshSessionError> {
            self.last_request = Some(request);
            Ok(RecordingSshChannel {
                channel: MockSshChannel::new(std::mem::take(&mut self.output)),
                recorded: Arc::clone(&self.recorded),
            })
        }
    }

    struct RecordingSshChannel {
        channel: MockSshChannel,
        recorded: Arc<Mutex<Option<MockSshChannel>>>,
    }

    impl SshChannel for RecordingSshChannel {
        fn read_channel(&mut self, buffer: &mut [u8]) -> Result<usize, super::SshSessionError> {
            self.channel.read_channel(buffer)
        }

        fn write_channel(&mut self, bytes: &[u8]) -> Result<usize, super::SshSessionError> {
            self.channel.write_channel(bytes)
        }

        fn resize_pty(&mut self, size: TerminalSize) -> Result<(), super::SshSessionError> {
            self.channel.resize_pty(size)
        }

        fn send_keepalive(&mut self) -> Result<(), super::SshSessionError> {
            self.channel.send_keepalive()
        }

        fn close_channel(&mut self) -> Result<(), super::SshSessionError> {
            self.channel.close_channel()?;
            *self.recorded.lock().unwrap() = Some(std::mem::replace(
                &mut self.channel,
                MockSshChannel::new(Vec::new()),
            ));
            Ok(())
        }

        fn into_read_writer(self) -> (Box<dyn SshShellReader>, Box<dyn SshShellWriter>) {
            split_mock_channel(self.channel, Some(self.recorded))
        }
    }

    struct MockChannelReader {
        output: Vec<u8>,
    }

    impl SshShellReader for MockChannelReader {
        fn read(&mut self, buffer: &mut [u8]) -> Result<usize, super::SshSessionError> {
            let count = buffer.len().min(self.output.len());
            buffer[..count].copy_from_slice(&self.output[..count]);
            self.output.drain(..count);
            Ok(count)
        }

        fn session_result(&self) -> SshSessionResult {
            SshSessionResult::default()
        }
    }

    struct MockChannelWriter {
        written: Vec<u8>,
        sizes: Vec<TerminalSize>,
        keepalives: u32,
        closed: bool,
        recorded: Option<Arc<Mutex<Option<MockSshChannel>>>>,
    }

    impl SshShellWriter for MockChannelWriter {
        fn write(&mut self, bytes: &[u8]) -> Result<usize, super::SshSessionError> {
            self.written.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn resize(&mut self, size: TerminalSize) -> Result<(), super::SshSessionError> {
            self.sizes.push(size);
            Ok(())
        }

        fn keepalive(&mut self) -> Result<(), super::SshSessionError> {
            self.keepalives += 1;
            Ok(())
        }

        fn finish_input(&mut self) -> Result<(), super::SshSessionError> {
            Ok(())
        }

        fn close(&mut self) -> Result<(), super::SshSessionError> {
            self.closed = true;
            if let Some(recorded) = &self.recorded {
                *recorded.lock().unwrap() = Some(MockSshChannel {
                    output: Vec::new(),
                    written: std::mem::take(&mut self.written),
                    sizes: std::mem::take(&mut self.sizes),
                    keepalives: self.keepalives,
                    closed: self.closed,
                });
            }
            Ok(())
        }
    }

    fn split_mock_channel(
        channel: MockSshChannel,
        recorded: Option<Arc<Mutex<Option<MockSshChannel>>>>,
    ) -> (Box<dyn SshShellReader>, Box<dyn SshShellWriter>) {
        (
            Box::new(MockChannelReader {
                output: channel.output,
            }),
            Box::new(MockChannelWriter {
                written: channel.written,
                sizes: channel.sizes,
                keepalives: channel.keepalives,
                closed: channel.closed,
                recorded,
            }),
        )
    }
}
