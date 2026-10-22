use std::{
    collections::BTreeMap,
    fmt,
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

#[cfg(unix)]
use interprocess::local_socket::GenericFilePath;
#[cfg(windows)]
use interprocess::local_socket::GenericNamespaced;
use interprocess::local_socket::{Listener, ListenerOptions, Stream, prelude::*};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

const OBSERVER_SCHEMA: u16 = 1;
const OBSERVER_PROTOCOL: &str = "rterm-functional-observer-v1";
const SUBSCRIBE_TIMEOUT: Duration = Duration::from_millis(250);
const MAX_REQUEST_FRAME_BYTES: usize = 4 * 1024;
const MAX_RESPONSE_FRAME_BYTES: usize = 8 * 1024 * 1024;

/// A 256-bit capability used exactly once to authenticate an observer session.
#[derive(Clone, PartialEq, Eq)]
pub struct ObserverToken([u8; 32]);

impl ObserverToken {
    #[must_use]
    pub fn generate() -> Self {
        Self(rand::random())
    }

    /// Returns the token for transfer to the child through a protected process boundary.
    #[must_use]
    pub fn expose_for_child_process(&self) -> String {
        encode_hex(&self.0)
    }

    /// Parses the exact hexadecimal representation passed to an observed child.
    ///
    /// # Errors
    ///
    /// Returns an error unless the value is exactly one 256-bit hexadecimal token.
    pub fn from_child_process(value: &str) -> Result<Self, &'static str> {
        decode_token(value)
    }

    fn matches(&self, candidate: &Self) -> bool {
        self.0
            .iter()
            .zip(candidate.0.iter())
            .fold(0_u8, |difference, (left, right)| {
                difference | (left ^ right)
            })
            == 0
    }
}

impl fmt::Debug for ObserverToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ObserverToken([REDACTED])")
    }
}

impl Serialize for ObserverToken {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.expose_for_child_process())
    }
}

impl<'de> Deserialize<'de> for ObserverToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        decode_token(&encoded).map_err(de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObserverRequestV1 {
    Hello { token: ObserverToken },
    Snapshot,
    Subscribe { after_revision: u64 },
}

impl ObserverRequestV1 {
    #[must_use]
    pub const fn hello(token: ObserverToken) -> Self {
        Self::Hello { token }
    }

    #[must_use]
    pub const fn snapshot() -> Self {
        Self::Snapshot
    }

    #[must_use]
    pub const fn subscribe(after_revision: u64) -> Self {
        Self::Subscribe { after_revision }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "request", rename_all = "snake_case", deny_unknown_fields)]
enum ObserverRequestWire {
    Hello { schema: u16, token: ObserverToken },
    Snapshot { schema: u16 },
    Subscribe { schema: u16, after_revision: u64 },
}

impl Serialize for ObserverRequestV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let wire = match self {
            Self::Hello { token } => ObserverRequestWire::Hello {
                schema: OBSERVER_SCHEMA,
                token: token.clone(),
            },
            Self::Snapshot => ObserverRequestWire::Snapshot {
                schema: OBSERVER_SCHEMA,
            },
            Self::Subscribe { after_revision } => ObserverRequestWire::Subscribe {
                schema: OBSERVER_SCHEMA,
                after_revision: *after_revision,
            },
        };
        wire.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ObserverRequestV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ObserverRequestWire::deserialize(deserializer)?;
        let (schema, request) = match wire {
            ObserverRequestWire::Hello { schema, token } => (schema, Self::Hello { token }),
            ObserverRequestWire::Snapshot { schema } => (schema, Self::Snapshot),
            ObserverRequestWire::Subscribe {
                schema,
                after_revision,
            } => (schema, Self::Subscribe { after_revision }),
        };
        if schema != OBSERVER_SCHEMA {
            return Err(de::Error::custom(format!(
                "unsupported observer request schema {schema}"
            )));
        }
        Ok(request)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "response", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObserverResponseV1 {
    Hello {
        schema: u16,
        protocol: String,
        revision: u64,
    },
    Snapshot {
        schema: u16,
        snapshot: ObserverSnapshotV1,
    },
    Update {
        schema: u16,
        snapshot: ObserverSnapshotV1,
    },
    Timeout {
        schema: u16,
        after_revision: u64,
    },
    Unauthorized {
        schema: u16,
        message: String,
    },
    ProtocolError {
        schema: u16,
        message: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverSnapshotV1 {
    pub schema: u16,
    pub revision: u64,
    pub config_generation: u64,
    pub config_diagnostic_present: bool,
    pub terminal: TerminalObservationV1,
    pub window: WindowObservationV1,
    pub runtime: RuntimeObservationV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalObservationV1 {
    pub text: String,
    pub cursor_row: u32,
    pub cursor_column: u32,
    pub modes: BTreeMap<String, bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowObservationV1 {
    pub width: u32,
    pub height: u32,
    pub active_tab_id: Option<u64>,
    pub active_pane_id: Option<u64>,
    pub overlay: Option<String>,
    pub panes: Vec<PaneObservationV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaneObservationV1 {
    pub tab_id: u64,
    pub pane_id: u64,
    pub active: bool,
    pub row: u32,
    pub column: u32,
    pub rows: u32,
    pub columns: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeObservationV1 {
    pub transport_state: String,
    pub effects: Vec<HostEffectObservationV1>,
    pub render_digest: Option<String>,
    pub worker_count: u32,
    pub listener_count: u32,
    pub child_process_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostEffectObservationV1 {
    pub sequence: u64,
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObserverStateError {
    UnsupportedSchema(u16),
    RevisionDidNotAdvance { current: u64, proposed: u64 },
}

impl fmt::Display for ObserverStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema(schema) => {
                write!(formatter, "unsupported observer snapshot schema {schema}")
            }
            Self::RevisionDidNotAdvance { current, proposed } => write!(
                formatter,
                "observer revision must advance: current={current}, proposed={proposed}"
            ),
        }
    }
}

impl std::error::Error for ObserverStateError {}

#[derive(Clone)]
pub struct ObserverState {
    shared: Arc<ObserverShared>,
}

struct ObserverShared {
    snapshot: Mutex<ObserverSnapshotV1>,
    changed: Condvar,
    delivered_revision: Mutex<u64>,
    delivered: Condvar,
}

impl ObserverState {
    /// Creates observable state from an initial version-one snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when the snapshot schema is unsupported.
    pub fn new(snapshot: ObserverSnapshotV1) -> Result<Self, ObserverStateError> {
        validate_snapshot_schema(&snapshot)?;
        Ok(Self {
            shared: Arc::new(ObserverShared {
                snapshot: Mutex::new(snapshot),
                changed: Condvar::new(),
                delivered_revision: Mutex::new(0),
                delivered: Condvar::new(),
            }),
        })
    }

    /// Publishes a strictly newer snapshot and wakes subscribers.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported schema or non-advancing revision.
    pub fn publish(&self, snapshot: ObserverSnapshotV1) -> Result<(), ObserverStateError> {
        validate_snapshot_schema(&snapshot)?;
        let mut current = lock_recover(&self.shared.snapshot);
        if snapshot.revision <= current.revision {
            return Err(ObserverStateError::RevisionDidNotAdvance {
                current: current.revision,
                proposed: snapshot.revision,
            });
        }
        *current = snapshot;
        drop(current);
        self.shared.changed.notify_all();
        Ok(())
    }

    #[must_use]
    pub fn snapshot(&self) -> ObserverSnapshotV1 {
        lock_recover(&self.shared.snapshot).clone()
    }

    #[must_use]
    pub fn wait_after(&self, revision: u64, timeout: Duration) -> Option<ObserverSnapshotV1> {
        let current = lock_recover(&self.shared.snapshot);
        let (current, _) = self
            .shared
            .changed
            .wait_timeout_while(current, timeout, |snapshot| snapshot.revision <= revision)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        (current.revision > revision).then(|| current.clone())
    }

    #[must_use]
    pub fn wait_until_delivered(&self, revision: u64, timeout: Duration) -> bool {
        let delivered = lock_recover(&self.shared.delivered_revision);
        let (delivered, _) = self
            .shared
            .delivered
            .wait_timeout_while(delivered, timeout, |value| *value < revision)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *delivered >= revision
    }

    fn acknowledge_delivery(&self, revision: u64) {
        let mut delivered = lock_recover(&self.shared.delivered_revision);
        if revision > *delivered {
            *delivered = revision;
            drop(delivered);
            self.shared.delivered.notify_all();
        }
    }

    #[must_use]
    pub fn session(&self, token: ObserverToken) -> ObserverSession {
        ObserverSession {
            state: self.clone(),
            token,
            authenticated: false,
        }
    }
}

pub struct ObserverSession {
    state: ObserverState,
    token: ObserverToken,
    authenticated: bool,
}

impl ObserverSession {
    #[must_use]
    pub fn handle(&mut self, request: ObserverRequestV1) -> ObserverResponseV1 {
        match request {
            ObserverRequestV1::Hello { token } => {
                if self.authenticated {
                    return protocol_error("hello was already accepted".to_owned());
                }
                if !self.token.matches(&token) {
                    return unauthorized();
                }
                self.authenticated = true;
                ObserverResponseV1::Hello {
                    schema: OBSERVER_SCHEMA,
                    protocol: OBSERVER_PROTOCOL.to_owned(),
                    revision: self.state.snapshot().revision,
                }
            }
            ObserverRequestV1::Snapshot => {
                if let Some(response) = self.guard_read() {
                    return response;
                }
                ObserverResponseV1::Snapshot {
                    schema: OBSERVER_SCHEMA,
                    snapshot: self.state.snapshot(),
                }
            }
            ObserverRequestV1::Subscribe { after_revision } => {
                if let Some(response) = self.guard_read() {
                    return response;
                }
                self.state
                    .wait_after(after_revision, SUBSCRIBE_TIMEOUT)
                    .map_or(
                        ObserverResponseV1::Timeout {
                            schema: OBSERVER_SCHEMA,
                            after_revision,
                        },
                        |snapshot| ObserverResponseV1::Update {
                            schema: OBSERVER_SCHEMA,
                            snapshot,
                        },
                    )
            }
        }
    }

    fn guard_read(&self) -> Option<ObserverResponseV1> {
        if self.authenticated {
            None
        } else {
            Some(unauthorized())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObserverEndpoint {
    requested_path: PathBuf,
    #[cfg(unix)]
    socket_path: PathBuf,
    #[cfg(windows)]
    pipe_name: String,
}

impl ObserverEndpoint {
    /// Resolves a protected local endpoint from the runner-requested path.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid platform socket or pipe identity.
    pub fn from_requested_path(requested_path: &Path) -> io::Result<Self> {
        make_endpoint(requested_path)
    }

    #[must_use]
    pub fn requested_path(&self) -> &Path {
        &self.requested_path
    }

    #[cfg(unix)]
    #[must_use]
    pub fn transport_path(&self) -> &Path {
        &self.socket_path
    }
}

pub struct ObserverServer {
    endpoint: ObserverEndpoint,
    listener: Listener,
    token: ObserverToken,
    state: ObserverState,
}

impl ObserverServer {
    /// Binds a read-only one-token observer server.
    ///
    /// # Errors
    ///
    /// Returns an error when the protected UDS or named pipe cannot be created.
    pub fn bind(
        requested_path: &Path,
        token: ObserverToken,
        state: ObserverState,
    ) -> io::Result<Self> {
        let endpoint = make_endpoint(requested_path)?;
        let listener = create_listener(&endpoint)?;
        Ok(Self {
            endpoint,
            listener,
            token,
            state,
        })
    }

    #[must_use]
    pub const fn endpoint(&self) -> &ObserverEndpoint {
        &self.endpoint
    }

    /// Serves one authenticated client until it disconnects.
    ///
    /// # Errors
    ///
    /// Returns an error for listener, wire, or response I/O failures.
    pub fn serve_one(&mut self) -> io::Result<()> {
        let stream = self.listener.accept()?;
        let mut connection = BufReader::new(stream);
        let mut session = self.state.session(self.token.clone());
        let mut line = String::with_capacity(256);
        loop {
            if read_bounded_line(&mut connection, &mut line, MAX_REQUEST_FRAME_BYTES)? == 0 {
                return Ok(());
            }
            let response = match serde_json::from_str::<ObserverRequestV1>(&line) {
                Ok(request) => session.handle(request),
                Err(error) => protocol_error(format!("invalid request: {error}")),
            };
            write_json_line(connection.get_mut(), &response)?;
            if let Some(revision) = response_revision(&response) {
                self.state.acknowledge_delivery(revision);
            }
        }
    }
}

fn response_revision(response: &ObserverResponseV1) -> Option<u64> {
    match response {
        ObserverResponseV1::Hello { revision, .. } => Some(*revision),
        ObserverResponseV1::Snapshot { snapshot, .. }
        | ObserverResponseV1::Update { snapshot, .. } => Some(snapshot.revision),
        ObserverResponseV1::Timeout { .. }
        | ObserverResponseV1::Unauthorized { .. }
        | ObserverResponseV1::ProtocolError { .. } => None,
    }
}

pub struct ObserverClient {
    connection: BufReader<Stream>,
}

impl ObserverClient {
    /// Connects using the runner-requested endpoint path.
    ///
    /// # Errors
    ///
    /// Returns an error when endpoint resolution or connection fails.
    pub fn connect_path(requested_path: &Path) -> io::Result<Self> {
        let endpoint = ObserverEndpoint::from_requested_path(requested_path)?;
        Self::connect(&endpoint)
    }

    /// Connects to a resolved local observer endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error when the local transport cannot connect.
    pub fn connect(endpoint: &ObserverEndpoint) -> io::Result<Self> {
        let stream = Stream::connect(socket_name(endpoint)?)?;
        Ok(Self {
            connection: BufReader::new(stream),
        })
    }

    /// Authenticates this connection with its one-time capability.
    ///
    /// # Errors
    ///
    /// Returns an error for transport failure or rejected authentication.
    pub fn hello(&mut self, token: ObserverToken) -> io::Result<()> {
        match self.request(&ObserverRequestV1::hello(token))? {
            ObserverResponseV1::Hello { .. } => Ok(()),
            response => Err(unexpected_response("hello", &response)),
        }
    }

    /// Reads the latest immutable semantic snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error for transport failure or an unexpected response.
    pub fn snapshot(&mut self) -> io::Result<ObserverSnapshotV1> {
        match self.request(&ObserverRequestV1::snapshot())? {
            ObserverResponseV1::Snapshot { snapshot, .. } => Ok(snapshot),
            response => Err(unexpected_response("snapshot", &response)),
        }
    }

    /// Waits for a revision newer than `after_revision` or the protocol timeout.
    ///
    /// # Errors
    ///
    /// Returns an error for transport failure or an unexpected response.
    pub fn subscribe(&mut self, after_revision: u64) -> io::Result<Option<ObserverSnapshotV1>> {
        match self.request(&ObserverRequestV1::subscribe(after_revision))? {
            ObserverResponseV1::Update { snapshot, .. } => Ok(Some(snapshot)),
            ObserverResponseV1::Timeout { .. } => Ok(None),
            response => Err(unexpected_response("subscribe", &response)),
        }
    }

    fn request(&mut self, request: &ObserverRequestV1) -> io::Result<ObserverResponseV1> {
        write_json_line(self.connection.get_mut(), request)?;
        let mut line = String::with_capacity(256);
        if read_bounded_line(&mut self.connection, &mut line, MAX_RESPONSE_FRAME_BYTES)? == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "observer closed before replying",
            ));
        }
        serde_json::from_str(&line).map_err(invalid_wire_data)
    }
}

fn read_bounded_line(
    reader: &mut impl BufRead,
    line: &mut String,
    max_bytes: usize,
) -> io::Result<usize> {
    line.clear();
    let limit = u64::try_from(max_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let read = std::io::Read::take(reader, limit).read_line(line)?;
    if read > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("observer frame exceeded {max_bytes} bytes"),
        ));
    }
    if read > 0 && !line.ends_with('\n') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "observer frame was not newline terminated",
        ));
    }
    Ok(read)
}

fn create_listener(endpoint: &ObserverEndpoint) -> io::Result<Listener> {
    #[cfg(unix)]
    {
        #[cfg(not(target_os = "macos"))]
        use interprocess::os::unix::local_socket::ListenerOptionsExt;

        let parent = endpoint.socket_path.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "observer endpoint has no parent",
            )
        })?;
        ensure_private_socket_directory(parent)?;
        let name = endpoint
            .socket_path
            .as_path()
            .to_fs_name::<GenericFilePath>()?;
        #[cfg(not(target_os = "macos"))]
        {
            ListenerOptions::new().name(name).mode(0o600).create_sync()
        }
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::PermissionsExt;

            // Darwin rejects fchmod on an unbound socket. The already-private
            // 0700 parent protects the bind-to-chmod interval; keep the socket
            // itself owner-only too, without changing process-wide umask.
            let listener = ListenerOptions::new().name(name).create_sync()?;
            std::fs::set_permissions(
                &endpoint.socket_path,
                std::fs::Permissions::from_mode(0o600),
            )?;
            Ok(listener)
        }
    }
    #[cfg(windows)]
    {
        use interprocess::os::windows::{
            local_socket::ListenerOptionsExt, security_descriptor::SecurityDescriptor,
        };
        use widestring::U16CString;

        let name = endpoint
            .pipe_name
            .as_str()
            .to_ns_name::<GenericNamespaced>()?;
        let sddl = U16CString::from_str("D:P(A;;GA;;;OW)")
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        let descriptor = SecurityDescriptor::deserialize(&sddl)?;
        ListenerOptions::new()
            .name(name)
            .security_descriptor(descriptor)
            .create_sync()
    }
}

fn socket_name(endpoint: &ObserverEndpoint) -> io::Result<interprocess::local_socket::Name<'_>> {
    #[cfg(unix)]
    {
        endpoint
            .socket_path
            .as_path()
            .to_fs_name::<GenericFilePath>()
    }
    #[cfg(windows)]
    {
        endpoint
            .pipe_name
            .as_str()
            .to_ns_name::<GenericNamespaced>()
    }
}

fn make_endpoint(requested_path: &Path) -> io::Result<ObserverEndpoint> {
    if requested_path.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "observer endpoint path is empty",
        ));
    }
    Ok(ObserverEndpoint {
        requested_path: requested_path.to_owned(),
        #[cfg(unix)]
        socket_path: unix_socket_path(requested_path),
        #[cfg(windows)]
        pipe_name: format!(
            "rterm-functional-{:016x}",
            endpoint_path_hash(requested_path)
        ),
    })
}

fn endpoint_path_hash(path: &Path) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    path.to_string_lossy()
        .bytes()
        .fold(FNV_OFFSET_BASIS, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
        })
}

#[cfg(unix)]
fn unix_socket_path(requested_path: &Path) -> PathBuf {
    let owner_identity = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USER"))
        .unwrap_or_else(|| "unknown-owner".into());
    let owner_hash = endpoint_path_hash(Path::new(&owner_identity));
    Path::new("/tmp")
        .join(format!("rterm-f-{owner_hash:016x}"))
        .join(format!(
            "o-{:016x}.sock",
            endpoint_path_hash(requested_path)
        ))
}

#[cfg(unix)]
fn ensure_private_socket_directory(directory: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    match std::fs::symlink_metadata(directory) {
        Ok(metadata) if metadata.file_type().is_dir() => {}
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "observer runtime path is not a directory",
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            match std::fs::create_dir(directory) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    let metadata = std::fs::symlink_metadata(directory)?;
                    if !metadata.file_type().is_dir() {
                        return Err(io::Error::new(
                            io::ErrorKind::AlreadyExists,
                            "observer runtime path is not a directory",
                        ));
                    }
                }
                Err(error) => return Err(error),
            }
        }
        Err(error) => return Err(error),
    }
    std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))
}

fn write_json_line<T: Serialize>(writer: &mut impl Write, value: &T) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, value).map_err(invalid_wire_data)?;
    writer.write_all(b"\n")?;
    writer.flush()
}

fn validate_snapshot_schema(snapshot: &ObserverSnapshotV1) -> Result<(), ObserverStateError> {
    if snapshot.schema == OBSERVER_SCHEMA {
        Ok(())
    } else {
        Err(ObserverStateError::UnsupportedSchema(snapshot.schema))
    }
}

fn lock_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn unauthorized() -> ObserverResponseV1 {
    ObserverResponseV1::Unauthorized {
        schema: OBSERVER_SCHEMA,
        message: "observer authentication required".to_owned(),
    }
}

fn protocol_error(message: String) -> ObserverResponseV1 {
    ObserverResponseV1::ProtocolError {
        schema: OBSERVER_SCHEMA,
        message,
    }
}

fn unexpected_response(operation: &str, response: &ObserverResponseV1) -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!("observer rejected {operation}: {response:?}"),
    )
}

fn invalid_wire_data(error: impl std::error::Error + Send + Sync + 'static) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

fn encode_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
        encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn decode_token(encoded: &str) -> Result<ObserverToken, &'static str> {
    if encoded.len() != 64 {
        return Err("observer token must contain exactly 64 hexadecimal characters");
    }
    let mut decoded = [0_u8; 32];
    for (index, pair) in encoded.as_bytes().chunks_exact(2).enumerate() {
        decoded[index] = decode_nibble(pair[0])?
            .checked_shl(4)
            .ok_or("observer token hex overflow")?
            | decode_nibble(pair[1])?;
    }
    Ok(ObserverToken(decoded))
}

const fn decode_nibble(value: u8) -> Result<u8, &'static str> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err("observer token contains a non-hexadecimal character"),
    }
}

#[cfg(test)]
mod frame_tests {
    use std::io::{self, BufReader};

    use super::read_bounded_line;

    #[test]
    fn bounded_line_rejects_oversize_and_unterminated_frames() {
        let mut line = String::new();
        let mut oversize = BufReader::new(b"12345\n".as_slice());
        let error = read_bounded_line(&mut oversize, &mut line, 4).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("exceeded"));

        let mut unterminated = BufReader::new(b"1234".as_slice());
        let error = read_bounded_line(&mut unterminated, &mut line, 4).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("newline"));
    }

    #[test]
    fn bounded_line_accepts_a_complete_frame_at_the_limit() {
        let mut line = String::new();
        let mut frame = BufReader::new(b"123\n".as_slice());
        assert_eq!(read_bounded_line(&mut frame, &mut line, 4).unwrap(), 4);
        assert_eq!(line, "123\n");
    }

    #[cfg(unix)]
    #[test]
    fn private_socket_directory_creation_tolerates_concurrent_binders() {
        use std::sync::{Arc, Barrier};

        let root = tempfile::TempDir::new().unwrap();
        for attempt in 0..32 {
            let directory = root.path().join(attempt.to_string());
            let barrier = Arc::new(Barrier::new(17));
            let workers: Vec<_> = (0..16)
                .map(|_| {
                    let barrier = Arc::clone(&barrier);
                    let directory = directory.clone();
                    std::thread::spawn(move || {
                        barrier.wait();
                        super::ensure_private_socket_directory(&directory)
                    })
                })
                .collect();
            barrier.wait();
            for worker in workers {
                worker.join().unwrap().unwrap();
            }
        }
    }
}
