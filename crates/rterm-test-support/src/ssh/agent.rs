use std::{
    io,
    path::Path,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use russh::keys::{
    Algorithm, PrivateKey,
    agent::{
        client::AgentClient,
        server::{Agent, MessageType},
    },
    ssh_key::LineEnding,
};

use super::lifecycle::{ShutdownDeadline, ThreadJoinOutcome, join_thread_until};

const AGENT_START_DEADLINE: Duration = Duration::from_secs(2);
const AGENT_DROP_DEADLINE: Duration = Duration::from_millis(500);
const AGENT_STREAM_CAPACITY: usize = 256 * 1024;

/// A runtime-generated OpenSSH identity, optionally encrypted on disk.
pub struct IdentityFixture {
    directory: tempfile::TempDir,
    identity_path: std::path::PathBuf,
    private_key: Arc<PrivateKey>,
    passphrase: Option<String>,
}

impl IdentityFixture {
    /// Generates a runtime Ed25519 identity and encrypts its OpenSSH private-key file.
    ///
    /// # Errors
    ///
    /// Returns an error if key generation, encryption, temporary storage, encoding,
    /// writing, or private-key permission setup fails.
    pub fn runtime_ed25519_encrypted(passphrase: impl Into<String>) -> io::Result<Self> {
        let passphrase = passphrase.into();
        if passphrase.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "fixture identity passphrase must not be empty",
            ));
        }
        let directory = tempfile::Builder::new()
            .prefix("rterm-ed25519-identity-")
            .tempdir()?;
        let private_key =
            PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).map_err(|error| {
                io::Error::other(format!("generate fixture Ed25519 identity: {error}"))
            })?;
        let encrypted = private_key
            .encrypt(&mut rand::rng(), passphrase.as_bytes())
            .map_err(|error| {
                io::Error::other(format!("encrypt fixture Ed25519 identity: {error}"))
            })?;
        let encoded = encrypted.to_openssh(LineEnding::LF).map_err(|error| {
            io::Error::other(format!("encode fixture Ed25519 identity: {error}"))
        })?;
        let identity_path = directory.path().join("id_ed25519");
        std::fs::write(&identity_path, encoded.as_bytes())?;
        restrict_private_key_permissions(&identity_path)?;
        Ok(Self {
            directory,
            identity_path,
            private_key: Arc::new(private_key),
            passphrase: Some(passphrase),
        })
    }

    /// Returns the temporary directory owning the identity file.
    #[must_use]
    pub fn directory(&self) -> &Path {
        self.directory.path()
    }

    /// Returns the encrypted OpenSSH private-key path.
    #[must_use]
    pub fn identity_path(&self) -> &Path {
        &self.identity_path
    }

    /// Returns the public key corresponding to the encrypted private key.
    #[must_use]
    pub fn public_key(&self) -> &russh::keys::ssh_key::PublicKey {
        self.private_key.public_key()
    }

    /// Returns the in-memory decrypted key for native protocol tests.
    #[must_use]
    pub fn private_key(&self) -> &Arc<PrivateKey> {
        &self.private_key
    }

    /// Returns the passphrase needed to decrypt the OpenSSH key file.
    #[must_use]
    pub fn passphrase(&self) -> Option<&str> {
        self.passphrase.as_deref()
    }
}

/// A runtime-generated client identity stored only in an isolated temporary directory.
pub struct AgentFixture {
    directory: tempfile::TempDir,
    identity_path: std::path::PathBuf,
    private_key: Arc<PrivateKey>,
    connection_sender: Option<tokio::sync::mpsc::UnboundedSender<tokio::io::DuplexStream>>,
    completion: mpsc::Receiver<io::Result<()>>,
    worker: Option<thread::JoinHandle<()>>,
}

impl AgentFixture {
    /// Generates an Ed25519 identity and writes an OpenSSH-compatible private key.
    ///
    /// # Errors
    ///
    /// Returns an error if temporary storage, key generation, encoding, writing, or
    /// private-key permission setup fails.
    pub fn new() -> io::Result<Self> {
        Self::new_configured(None, false)
    }

    pub(super) fn new_configured(
        teardown_delay: Option<Duration>,
        panic_on_teardown: bool,
    ) -> io::Result<Self> {
        super::lifecycle::ensure_process_reaper()?;
        let directory = tempfile::Builder::new()
            .prefix("rterm-agent-fixture-")
            .tempdir()?;
        let private_key = PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519)
            .map_err(|error| io::Error::other(format!("generate fixture identity: {error}")))?;
        let encoded = private_key
            .to_openssh(LineEnding::LF)
            .map_err(|error| io::Error::other(format!("encode fixture identity: {error}")))?;
        let identity_path = directory.path().join("id_ed25519");
        std::fs::write(&identity_path, encoded.as_bytes())?;
        restrict_private_key_permissions(&identity_path)?;
        let (connection_sender, connection_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (completion_sender, completion) = mpsc::sync_channel(1);
        let sealed = Arc::new(AtomicBool::new(false));
        let worker_policy = FixtureAgentPolicy {
            sealed: Arc::clone(&sealed),
        };
        let worker = thread::Builder::new()
            .name("rterm-in-memory-agent".to_owned())
            .spawn(move || {
                let result = run_agent(connection_receiver, worker_policy, teardown_delay);
                assert!(!panic_on_teardown, "injected agent worker panic");
                let _ = completion_sender.send(result);
            })?;
        let fixture = Self {
            directory,
            identity_path,
            private_key: Arc::new(private_key),
            connection_sender: Some(connection_sender),
            completion,
            worker: Some(worker),
        };
        fixture.initialize_agent()?;
        sealed.store(true, Ordering::Release);
        Ok(fixture)
    }

    #[cfg(test)]
    fn new_with_worker_panic() -> io::Result<Self> {
        Self::new_configured(None, true)
    }

    /// Returns the directory which owns all identity state.
    #[must_use]
    pub fn directory(&self) -> &Path {
        self.directory.path()
    }

    /// Returns the OpenSSH private-key path.
    #[must_use]
    pub fn identity_path(&self) -> &Path {
        &self.identity_path
    }

    /// Returns the public identity accepted by the fixture server.
    #[must_use]
    pub fn public_key(&self) -> &russh::keys::ssh_key::PublicKey {
        self.private_key.public_key()
    }

    /// Returns the in-memory private identity for native russh tests.
    #[must_use]
    pub fn private_key(&self) -> &Arc<PrivateKey> {
        &self.private_key
    }

    /// Opens a private in-memory connection to the fixture agent.
    ///
    /// # Errors
    ///
    /// Returns an error if the bounded agent worker has already stopped.
    pub fn connect(&self) -> io::Result<AgentClient<tokio::io::DuplexStream>> {
        let (client, server) = tokio::io::duplex(AGENT_STREAM_CAPACITY);
        self.connection_sender
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "fixture agent stopped"))?
            .send(server)
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "fixture agent stopped"))?;
        Ok(AgentClient::connect(client))
    }

    /// Configures an OpenSSH command to use only this identity and no ambient agent.
    pub fn configure_ssh_command(&self, command: &mut Command) {
        command
            .arg("-i")
            .arg(&self.identity_path)
            .arg("-o")
            .arg("IdentitiesOnly=yes")
            .arg("-o")
            .arg("IdentityAgent=none")
            .env_remove("SSH_AUTH_SOCK");
    }

    /// Stops the in-memory protocol server and joins its worker within `deadline`.
    ///
    /// # Errors
    ///
    /// Returns an error when teardown exceeds the deadline, the worker panics, or the
    /// protocol server reports an I/O failure.
    pub fn stop(self, deadline: Duration) -> io::Result<()> {
        self.stop_until(ShutdownDeadline::after(deadline))
    }

    fn initialize_agent(&self) -> io::Result<()> {
        let mut client = self.connect()?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(async {
            tokio::time::timeout(
                AGENT_START_DEADLINE,
                client.add_identity(self.private_key.as_ref(), &[]),
            )
            .await
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::TimedOut,
                    "in-memory fixture agent startup exceeded its deadline",
                )
            })?
            .map_err(|error| io::Error::other(format!("initialize fixture agent: {error}")))
        })
    }

    pub(super) fn stop_until(mut self, deadline: ShutdownDeadline) -> io::Result<()> {
        self.stop_inner(deadline)
    }

    fn stop_inner(&mut self, deadline: ShutdownDeadline) -> io::Result<()> {
        if self.worker.is_none() {
            return Ok(());
        }
        self.connection_sender.take();
        let result = match self.completion.recv_timeout(deadline.remaining()) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(worker) = self.worker.take() {
                    defer_agent_join(worker);
                }
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!(
                        "in-memory fixture agent teardown exceeded {:?}",
                        deadline.budget()
                    ),
                ));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return finish_missing_agent_completion(self.worker.take(), deadline);
            }
        };
        if let Some(worker) = self.worker.take() {
            join_agent_worker_until(worker, deadline)?;
        }
        result
    }
}

impl Drop for AgentFixture {
    fn drop(&mut self) {
        let _ = self.stop_inner(ShutdownDeadline::after(AGENT_DROP_DEADLINE));
    }
}

fn defer_agent_join(worker: thread::JoinHandle<()>) {
    super::lifecycle::defer_thread(worker);
}

fn join_agent_worker_until(
    worker: thread::JoinHandle<()>,
    deadline: ShutdownDeadline,
) -> io::Result<()> {
    match join_thread_until(worker, deadline) {
        ThreadJoinOutcome::Completed => Ok(()),
        ThreadJoinOutcome::Panicked => {
            Err(io::Error::other("in-memory fixture agent worker panicked"))
        }
        ThreadJoinOutcome::Deferred => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!(
                "in-memory fixture agent worker join exceeded {:?}",
                deadline.budget()
            ),
        )),
    }
}

fn finish_missing_agent_completion(
    worker: Option<thread::JoinHandle<()>>,
    deadline: ShutdownDeadline,
) -> io::Result<()> {
    if let Some(worker) = worker {
        join_agent_worker_until(worker, deadline)?;
    }
    Err(io::Error::new(
        io::ErrorKind::BrokenPipe,
        "in-memory fixture agent worker exited without completion",
    ))
}

fn run_agent(
    mut connections: tokio::sync::mpsc::UnboundedReceiver<tokio::io::DuplexStream>,
    policy: FixtureAgentPolicy,
    teardown_delay: Option<Duration>,
) -> io::Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let result = runtime.block_on(async move {
        let incoming = futures::stream::poll_fn(move |context| {
            connections.poll_recv(context).map(|stream| stream.map(Ok))
        });
        russh::keys::agent::server::serve(incoming, policy)
            .await
            .map_err(|error| io::Error::other(format!("in-memory fixture agent failed: {error}")))
    });
    if let Some(delay) = teardown_delay {
        thread::sleep(delay);
    }
    result
}

#[derive(Clone)]
struct FixtureAgentPolicy {
    sealed: Arc<AtomicBool>,
}

impl Agent for FixtureAgentPolicy {
    async fn confirm_request(&self, message: MessageType) -> bool {
        match message {
            MessageType::RequestKeys | MessageType::Sign => true,
            MessageType::AddKeys => !self.sealed.load(Ordering::Acquire),
            MessageType::RemoveKeys
            | MessageType::RemoveAllKeys
            | MessageType::Lock
            | MessageType::Unlock => false,
        }
    }
}

#[cfg(unix)]
fn restrict_private_key_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn restrict_private_key_permissions(path: &Path) -> io::Result<()> {
    std::fs::metadata(path).map(|_| ())
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::{AgentFixture, Algorithm, PrivateKey};

    #[test]
    fn runtime_ed25519_identity_exposes_encrypted_key_material() {
        let identity = super::IdentityFixture::runtime_ed25519_encrypted("fixture-passphrase")
            .expect("generate encrypted Ed25519 identity");
        assert_eq!(identity.public_key().algorithm(), Algorithm::Ed25519);
        assert_eq!(identity.passphrase(), Some("fixture-passphrase"));
        let encrypted = std::fs::read_to_string(identity.identity_path()).unwrap();
        assert!(encrypted.contains("BEGIN OPENSSH PRIVATE KEY"));
        let decrypted =
            russh::keys::load_secret_key(identity.identity_path(), identity.passphrase())
                .expect("decrypt generated Ed25519 identity");
        assert_eq!(decrypted.public_key(), identity.public_key());
    }

    #[test]
    fn identity_is_generated_at_runtime_and_injected_without_a_real_agent() {
        let first = AgentFixture::new().expect("generate first identity");
        let second = AgentFixture::new().expect("generate second identity");
        assert_ne!(first.public_key(), second.public_key());
        assert!(first.identity_path().starts_with(first.directory()));
        assert!(!std::fs::read(first.identity_path()).unwrap().is_empty());

        let mut command = Command::new("ssh");
        first.configure_ssh_command(&mut command);
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let identity = first.identity_path().to_string_lossy();
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "-i" && pair[1] == identity)
        );
        assert!(args.iter().any(|arg| arg == "IdentitiesOnly=yes"));
        assert!(args.iter().any(|arg| arg == "IdentityAgent=none"));
    }

    #[test]
    fn identity_is_available_through_an_in_memory_agent() {
        let fixture = AgentFixture::new().expect("start in-memory agent");
        let expected = fixture.public_key().clone();
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                let mut client = fixture.connect().expect("connect in-memory agent");
                let identities = client
                    .request_identities()
                    .await
                    .expect("request identities");
                assert_eq!(identities.len(), 1);
                assert_eq!(identities[0].public_key().as_ref(), &expected);
                client
                    .sign_request(&identities[0], None, b"agent-proof".to_vec())
                    .await
                    .expect("agent signs with injected identity");
            });
    }

    #[test]
    fn explicit_agent_stop_is_bounded_and_closes_active_clients() {
        let fixture = AgentFixture::new().expect("start in-memory agent");
        let mut client = fixture.connect().expect("connect in-memory agent");
        let started = std::time::Instant::now();
        fixture
            .stop(std::time::Duration::from_secs(2))
            .expect("stop in-memory agent");
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                let result = tokio::time::timeout(
                    std::time::Duration::from_secs(1),
                    client.request_identities(),
                )
                .await;
                assert!(matches!(result, Ok(Err(_))));
            });
    }

    #[test]
    fn agent_worker_panic_is_not_misclassified_as_timeout() {
        let fixture = AgentFixture::new_with_worker_panic().expect("start panicking agent fixture");
        let error = fixture
            .stop(std::time::Duration::from_secs(1))
            .expect_err("agent worker panic must be observable");
        assert!(error.to_string().contains("agent worker panicked"));
        assert_ne!(error.kind(), std::io::ErrorKind::TimedOut);
    }

    #[test]
    fn initialized_agent_rejects_identity_mutation() {
        let fixture = AgentFixture::new().expect("start sealed in-memory agent");
        let injected = fixture.public_key().clone();
        let replacement = PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).unwrap();
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                let mut client = fixture.connect().expect("connect sealed agent");
                assert!(client.add_identity(&replacement, &[]).await.is_err());
                let identities = client.request_identities().await.unwrap();
                assert_eq!(identities.len(), 1);
                assert_eq!(identities[0].public_key().as_ref(), &injected);
            });
    }

    #[cfg(unix)]
    #[test]
    fn private_identity_is_owner_read_write_only() {
        use std::os::unix::fs::PermissionsExt as _;

        let fixture = AgentFixture::new().expect("generate identity");
        let mode = std::fs::metadata(fixture.identity_path())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
}
