use std::{
    net::{IpAddr, SocketAddr},
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    Router,
    extract::{Path as AxumPath, Query, State, WebSocketUpgrade},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use futures_util::StreamExt;
use rand::RngExt;
use rterm_pty::PtyCommand;
use serde::Deserialize;
use tokio::{net::TcpListener, sync::Semaphore, time};

use crate::{
    protocol::{
        ClientMessage, DimensionError, MAX_CONTROL_FRAME_BYTES, MAX_INPUT_FRAME_BYTES,
        PROTOCOL_VERSION, ServerMessage, TerminalDimensions,
    },
    session::{InputSendError, ResizeSendError, SessionEvent, WebPtySession},
};

pub const DEFAULT_MAX_SESSIONS: usize = 8;
const BOOTSTRAP_TOKEN_BYTES: usize = 32;
const SESSION_TOKEN_BYTES: usize = 32;
const BOOTSTRAP_TTL: Duration = Duration::from_secs(60);
const SESSION_ID_BYTES: usize = 16;
const OPEN_TIMEOUT: Duration = Duration::from_secs(5);
const SOCKET_SEND_TIMEOUT: Duration = Duration::from_secs(2);
const COOKIE_NAME: &str = "rterm_web_auth";
const CSP: &str = "default-src 'self'; connect-src 'self' ws: wss:; style-src 'self'; script-src 'self'; img-src 'self' data:; font-src 'self'; base-uri 'none'; frame-ancestors 'none'";

#[derive(Debug, Clone)]
pub struct WebServerConfig {
    pub listen: SocketAddr,
    pub web_root: PathBuf,
    pub max_sessions: usize,
    pub allowed_origin: Option<String>,
}

#[derive(Clone)]
struct WebAppState {
    web_root: Arc<PathBuf>,
    auth: Arc<AuthState>,
    host: Arc<str>,
    origin: Arc<str>,
    sessions: Arc<Semaphore>,
}

pub struct WebServer {
    listener: TcpListener,
    state: WebAppState,
    bootstrap_token: Arc<str>,
}

#[derive(Debug)]
struct AuthState {
    tokens: Mutex<AuthTokens>,
}

#[derive(Debug)]
struct AuthTokens {
    bootstrap: Option<BootstrapTicket>,
    session_token: Option<String>,
}

#[derive(Debug)]
struct BootstrapTicket {
    token: Arc<str>,
    expires_at: Instant,
}

impl AuthState {
    fn new(token: Arc<str>, expires_at: Instant) -> Self {
        Self {
            tokens: Mutex::new(AuthTokens {
                bootstrap: Some(BootstrapTicket { token, expires_at }),
                session_token: None,
            }),
        }
    }

    fn redeem(&self, candidate: &str, now: Instant) -> Option<String> {
        let mut tokens = self.lock_tokens();
        let ticket = tokens.bootstrap.as_ref()?;
        if now >= ticket.expires_at
            || !constant_time_equal(candidate.as_bytes(), ticket.token.as_bytes())
        {
            return None;
        }

        tokens.bootstrap = None;
        let session_token = generate_token(SESSION_TOKEN_BYTES);
        tokens.session_token = Some(session_token.clone());
        Some(session_token)
    }

    fn matches_session(&self, candidate: &str) -> bool {
        self.lock_tokens()
            .session_token
            .as_deref()
            .is_some_and(|token| constant_time_equal(candidate.as_bytes(), token.as_bytes()))
    }

    fn revoke(&self) {
        let mut tokens = self.lock_tokens();
        tokens.bootstrap = None;
        tokens.session_token = None;
    }

    fn lock_tokens(&self) -> std::sync::MutexGuard<'_, AuthTokens> {
        self.tokens
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl WebServer {
    /// Binds an authenticated loopback Web server.
    ///
    /// # Errors
    ///
    /// Returns an error when the listener is not loopback-only, the session
    /// limit is invalid, or the TCP listener cannot be created.
    pub async fn bind(config: WebServerConfig) -> Result<Self, ServerError> {
        if !config.listen.ip().is_loopback() {
            return Err(ServerError::LoopbackOnly(config.listen));
        }
        if config.max_sessions == 0 {
            return Err(ServerError::InvalidMaxSessions);
        }
        let listener = TcpListener::bind(config.listen).await?;
        let address = listener.local_addr()?;
        let host: Arc<str> = host_for_address(address).into();
        let origin: Arc<str> = config
            .allowed_origin
            .unwrap_or_else(|| format!("http://{host}"))
            .into();
        let bootstrap_token: Arc<str> = generate_token(BOOTSTRAP_TOKEN_BYTES).into();
        Ok(Self {
            listener,
            state: WebAppState {
                web_root: Arc::new(config.web_root),
                auth: Arc::new(AuthState::new(
                    bootstrap_token.clone(),
                    Instant::now() + BOOTSTRAP_TTL,
                )),
                host,
                origin,
                sessions: Arc::new(Semaphore::new(config.max_sessions)),
            },
            bootstrap_token,
        })
    }

    #[must_use]
    pub fn bootstrap_url(&self) -> String {
        format!("http://{}/?token={}", self.state.host, self.bootstrap_token)
    }

    #[cfg(test)]
    #[must_use]
    /// Returns the bound listener address.
    ///
    /// # Panics
    ///
    /// Panics if the already-bound listener no longer exposes its local address.
    pub fn local_addr(&self) -> SocketAddr {
        self.listener
            .local_addr()
            .expect("bound WebServer listener must expose its local address")
    }

    /// Runs the server until the process receives Ctrl+C.
    ///
    /// # Errors
    ///
    /// Returns an error when the HTTP server cannot continue serving requests.
    pub async fn run_until_shutdown(self) -> Result<(), ServerError> {
        self.run_until(shutdown_signal()).await
    }

    /// Runs the server until the supplied shutdown future resolves.
    ///
    /// # Errors
    ///
    /// Returns an error when the HTTP server cannot continue serving requests.
    pub async fn run_until<F>(self, shutdown: F) -> Result<(), ServerError>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let auth = self.state.auth.clone();
        let router = app_router(self.state);
        let listener = self.listener;
        let result = axum::serve(listener, router)
            .with_graceful_shutdown(shutdown)
            .await
            .map_err(ServerError::Io);
        auth.revoke();
        result
    }
}

fn app_router(state: impl Into<WebAppState>) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/api/v1/terminal", get(websocket))
        .route("/assets/{*path}", get(asset))
        .with_state(state.into())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

async fn root(
    State(state): State<WebAppState>,
    Query(query): Query<BootstrapQuery>,
    headers: HeaderMap,
) -> Response {
    if let Some(token) = query.token.as_deref() {
        if let Some(session_token) = state.auth.redeem(token, Instant::now()) {
            let mut response = Redirect::temporary("/").into_response();
            response
                .headers_mut()
                .insert(header::SET_COOKIE, cookie_header(&session_token));
            return harden_bootstrap_response(response);
        }
        return harden_bootstrap_response(error_response(
            StatusCode::UNAUTHORIZED,
            "invalid or expired bootstrap token",
        ));
    }
    if !http_authenticated(&headers, &state) {
        return harden_bootstrap_response(error_response(
            StatusCode::UNAUTHORIZED,
            "open the bootstrap URL printed by rterm-web",
        ));
    }
    harden_bootstrap_response(serve_file(&state.web_root, Path::new("index.html")).await)
}

#[derive(Debug, Deserialize)]
struct BootstrapQuery {
    token: Option<String>,
}

async fn asset(
    State(state): State<WebAppState>,
    AxumPath(path): AxumPath<String>,
    headers: HeaderMap,
) -> Response {
    if !http_authenticated(&headers, &state) {
        return error_response(StatusCode::UNAUTHORIZED, "web authentication required");
    }
    serve_file(&state.web_root, Path::new("assets").join(path).as_path()).await
}

async fn websocket(
    State(state): State<WebAppState>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response {
    if !websocket_authenticated(&headers, &state) {
        return error_response(StatusCode::FORBIDDEN, "websocket authentication failed");
    }
    let Ok(permit) = state.sessions.clone().try_acquire_owned() else {
        return error_response(StatusCode::TOO_MANY_REQUESTS, "session limit reached");
    };
    upgrade
        .max_message_size(MAX_INPUT_FRAME_BYTES)
        .max_frame_size(MAX_INPUT_FRAME_BYTES)
        .on_upgrade(move |socket| handle_socket(socket, permit))
}

#[expect(
    clippy::too_many_lines,
    reason = "the WebSocket protocol state machine is kept in one ordered loop"
)]
async fn handle_socket(
    mut socket: axum::extract::ws::WebSocket,
    _permit: tokio::sync::OwnedSemaphorePermit,
) {
    let Ok(Some(Ok(first))) = time::timeout(OPEN_TIMEOUT, socket.next()).await else {
        return;
    };
    let open = match first {
        axum::extract::ws::Message::Text(text) => {
            if text.len() > MAX_CONTROL_FRAME_BYTES {
                send_error_and_close(
                    &mut socket,
                    "MESSAGE_TOO_LARGE",
                    "control message is too large",
                )
                .await;
                return;
            }
            match serde_json::from_str::<ClientMessage>(text.as_ref()) {
                Ok(ClientMessage::Open {
                    protocol,
                    cols,
                    rows,
                    profile,
                }) => OpenRequest {
                    protocol,
                    cols,
                    rows,
                    profile,
                },
                Ok(_) => {
                    send_error_and_close(
                        &mut socket,
                        "OPEN_REQUIRED",
                        "first message must open a session",
                    )
                    .await;
                    return;
                }
                Err(_) => {
                    send_error_and_close(&mut socket, "INVALID_MESSAGE", "invalid control message")
                        .await;
                    return;
                }
            }
        }
        axum::extract::ws::Message::Close(_) => return,
        _ => {
            send_error_and_close(
                &mut socket,
                "OPEN_REQUIRED",
                "first message must open a session",
            )
            .await;
            return;
        }
    };

    let dimensions = match validate_open(&open) {
        Ok(dimensions) => dimensions,
        Err((code, message)) => {
            send_error_and_close(&mut socket, code, message).await;
            return;
        }
    };
    let initial_directory = std::env::current_dir().ok();
    let mut command = PtyCommand::default_shell();
    if let Some(directory) = &initial_directory {
        command = command.with_cwd(directory);
    }
    let shell = Path::new(command.program())
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_else(|| command.program());
    let cwd = initial_directory
        .as_ref()
        .map(|path| path.to_string_lossy());
    let Ok(mut session) = WebPtySession::spawn(&command, dimensions) else {
        send_error_and_close(
            &mut socket,
            "PTY_SPAWN_FAILED",
            "terminal could not be started",
        )
        .await;
        return;
    };
    let session_id = generate_token(SESSION_ID_BYTES);
    if send_json(
        &mut socket,
        &ServerMessage::Opened {
            protocol: PROTOCOL_VERSION,
            session_id: &session_id,
            cols: dimensions.cols,
            rows: dimensions.rows,
            shell: Some(shell),
            cwd: cwd.as_deref(),
        },
    )
    .await
    .is_err()
    {
        return;
    }

    let mut closing = false;
    loop {
        tokio::select! {
            event = session.events().recv() => {
                match event {
                    Some(SessionEvent::Output(bytes)) => {
                        if send_socket_message(
                            &mut socket,
                            axum::extract::ws::Message::Binary(bytes.into()),
                        )
                        .await
                        .is_err()
                        {
                            break;
                        }
                    }
                    Some(SessionEvent::Error { code, message, fatal }) => {
                        if send_json(&mut socket, &ServerMessage::Error { code, message, fatal }).await.is_err() {
                            break;
                        }
                    }
                    Some(SessionEvent::Exit(status)) => {
                        let signal = status.signal();
                        let _ = send_json(&mut socket, &ServerMessage::Exit { code: status.exit_code(), signal }).await;
                        let _ = send_socket_message(
                            &mut socket,
                            axum::extract::ws::Message::Close(None),
                        )
                        .await;
                        break;
                    }
                    None => break,
                }
            }
            message = socket.next() => {
                let Some(message) = message else { break };
                let Ok(message) = message else { break };
                match message {
                    axum::extract::ws::Message::Binary(bytes) => {
                        if closing {
                            continue;
                        }
                        match session.try_send_input(bytes.to_vec()) {
                            Ok(()) => {}
                            Err(InputSendError::Full) => {
                                let _ = send_json(&mut socket, &ServerMessage::Error { code: "INPUT_BACKPRESSURE", message: "terminal input queue is full", fatal: true }).await;
                                closing = true;
                                session.request_close();
                            }
                            Err(InputSendError::Closed) => break,
                        }
                    }
                    axum::extract::ws::Message::Text(text) => {
                        if text.len() > MAX_CONTROL_FRAME_BYTES {
                            send_error_and_close(&mut socket, "MESSAGE_TOO_LARGE", "control message is too large").await;
                            break;
                        }
                        let Ok(message) = serde_json::from_str::<ClientMessage>(text.as_ref()) else {
                            send_error_and_close(&mut socket, "INVALID_MESSAGE", "invalid control message").await;
                            break;
                        };
                        match message {
                            ClientMessage::Open { .. } => {
                                send_error_and_close(&mut socket, "OPEN_ALREADY_COMPLETE", "session is already open").await;
                                break;
                            }
                            ClientMessage::Resize { cols, rows } => {
                                if closing {
                                    continue;
                                }
                                let dimensions = match TerminalDimensions::validate(cols, rows) {
                                    Ok(dimensions) => dimensions,
                                    Err(error) => {
                                        send_error_and_close(
                                            &mut socket,
                                            error.code(),
                                            "terminal dimensions are out of range",
                                        )
                                        .await;
                                        break;
                                    }
                                };
                                match session.try_resize(dimensions) {
                                    Ok(()) => {}
                                    Err(ResizeSendError::Invalid) => {
                                        let _ = send_json(&mut socket, &ServerMessage::Error { code: "INVALID_SIZE", message: "terminal dimensions are invalid", fatal: false }).await;
                                    }
                                    Err(ResizeSendError::Full) => {
                                        let _ = send_json(&mut socket, &ServerMessage::Error { code: "RESIZE_BACKPRESSURE", message: "terminal resize queue is full", fatal: false }).await;
                                    }
                                    Err(ResizeSendError::Closed) => break,
                                }
                            }
                            ClientMessage::Close => {
                                closing = true;
                                session.request_close();
                            }
                        }
                    }
                    axum::extract::ws::Message::Ping(bytes) => {
                        if send_socket_message(
                            &mut socket,
                            axum::extract::ws::Message::Pong(bytes),
                        )
                        .await
                        .is_err()
                        {
                            break;
                        }
                    }
                    axum::extract::ws::Message::Pong(_) => {}
                    axum::extract::ws::Message::Close(_) => break,
                }
            }
        }
    }
}

#[derive(Debug)]
struct OpenRequest {
    protocol: u16,
    cols: u16,
    rows: u16,
    profile: String,
}

fn validate_open(
    request: &OpenRequest,
) -> Result<TerminalDimensions, (&'static str, &'static str)> {
    if request.protocol != PROTOCOL_VERSION {
        return Err(("UNSUPPORTED_PROTOCOL", "unsupported protocol version"));
    }
    if request.profile != "local-default" {
        return Err(("UNKNOWN_PROFILE", "unknown terminal profile"));
    }
    TerminalDimensions::validate(request.cols, request.rows).map_err(|error| match error {
        DimensionError::Columns(_) => ("INVALID_COLUMNS", "terminal columns are out of range"),
        DimensionError::Rows(_) => ("INVALID_ROWS", "terminal rows are out of range"),
    })
}

async fn send_json(
    socket: &mut axum::extract::ws::WebSocket,
    message: &ServerMessage<'_>,
) -> Result<(), ()> {
    let text = serde_json::to_string(message).expect("server protocol messages must serialize");
    send_socket_message(socket, axum::extract::ws::Message::Text(text.into())).await
}

async fn send_socket_message(
    socket: &mut axum::extract::ws::WebSocket,
    message: axum::extract::ws::Message,
) -> Result<(), ()> {
    match time::timeout(SOCKET_SEND_TIMEOUT, socket.send(message)).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(_)) | Err(_) => Err(()),
    }
}

async fn send_error_and_close(
    socket: &mut axum::extract::ws::WebSocket,
    code: &'static str,
    message: &'static str,
) {
    let _ = send_json(
        socket,
        &ServerMessage::Error {
            code,
            message,
            fatal: true,
        },
    )
    .await;
    let _ = send_socket_message(socket, axum::extract::ws::Message::Close(None)).await;
}

fn http_authenticated(headers: &HeaderMap, state: &WebAppState) -> bool {
    cookie_matches(headers, &state.auth)
}

fn websocket_authenticated(headers: &HeaderMap, state: &WebAppState) -> bool {
    let host_matches = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|host| host == state.host.as_ref());
    let origin_matches = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|origin| origin == state.origin.as_ref());
    host_matches && origin_matches && cookie_matches(headers, &state.auth)
}

fn cookie_matches(headers: &HeaderMap, auth: &AuthState) -> bool {
    let Some(cookie) = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    cookie
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .any(|(name, value)| name == COOKIE_NAME && auth.matches_session(value))
}

fn cookie_header(token: &str) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{COOKIE_NAME}={token}; HttpOnly; Path=/; SameSite=Strict"
    ))
    .expect("session token is a valid cookie value")
}

fn harden_bootstrap_response(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, max-age=0"),
    );
    response
        .headers_mut()
        .insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    apply_browser_security_headers(&mut response);
    response
}

async fn serve_file(root: &Path, relative: &Path) -> Response {
    if relative
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return error_response(StatusCode::NOT_FOUND, "asset not found");
    }
    let path = root.join(relative);
    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let content_type = content_type(&path);
            let mut response = Response::new(bytes.into());
            response
                .headers_mut()
                .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
            response.headers_mut().insert(
                header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_static(CSP),
            );
            apply_browser_security_headers(&mut response);
            response
        }
        Err(_) => error_response(
            StatusCode::NOT_FOUND,
            "web asset not found; run npm run build",
        ),
    }
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn error_response(status: StatusCode, message: &'static str) -> Response {
    let mut response = (status, message).into_response();
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CSP),
    );
    apply_browser_security_headers(&mut response);
    response
}

fn apply_browser_security_headers(response: &mut Response) {
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(
        "permissions-policy",
        HeaderValue::from_static("clipboard-read=(self), clipboard-write=(self)"),
    );
}

fn generate_token(bytes: usize) -> String {
    let mut token = vec![0_u8; bytes];
    rand::rng().fill(token.as_mut_slice());
    URL_SAFE_NO_PAD.encode(token)
}

fn host_for_address(address: SocketAddr) -> String {
    match address.ip() {
        IpAddr::V4(_) => address.to_string(),
        IpAddr::V6(ip) => format!("[{ip}]:{}", address.port()),
    }
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[derive(Debug)]
pub enum ServerError {
    LoopbackOnly(SocketAddr),
    InvalidMaxSessions,
    Io(std::io::Error),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LoopbackOnly(address) => {
                write!(formatter, "rterm-web only listens on loopback: {address}")
            }
            Self::InvalidMaxSessions => {
                formatter.write_str("maximum session count must be greater than zero")
            }
            Self::Io(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ServerError {}

impl From<std::io::Error> for ServerError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        path::{Path, PathBuf},
        sync::Arc,
        time::{Duration, Instant},
    };

    use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
    use futures_util::{SinkExt, StreamExt};
    use rterm_pty::PtyCommand;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
        sync::oneshot,
        time,
    };
    use tokio_tungstenite::{
        connect_async,
        tungstenite::{Message, client::IntoClientRequest},
    };

    use super::{
        AuthState, WebAppState, WebServer, WebServerConfig, constant_time_equal, cookie_matches,
        error_response, harden_bootstrap_response, host_for_address, validate_open,
        websocket_authenticated,
    };
    use crate::protocol::TerminalDimensions;

    fn state() -> WebAppState {
        let auth = Arc::new(AuthState::new(
            "bootstrap-ticket".into(),
            Instant::now() + Duration::from_secs(60),
        ));
        auth.lock_tokens().session_token = Some("test-token".to_owned());
        WebAppState {
            web_root: PathBuf::from("web/dist").into(),
            auth,
            host: "127.0.0.1:7798".into(),
            origin: "http://127.0.0.1:7798".into(),
            sessions: std::sync::Arc::new(tokio::sync::Semaphore::new(1)),
        }
    }

    #[test]
    fn host_formats_ipv4_and_ipv6_addresses() {
        assert_eq!(
            host_for_address(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 7798)),
            "127.0.0.1:7798"
        );
        assert_eq!(
            host_for_address(SocketAddr::new("::1".parse().unwrap(), 7798)),
            "[::1]:7798"
        );
    }

    #[test]
    fn cookie_authentication_requires_the_named_cookie() {
        let state = state();
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("other=ok; rterm_web_auth=test-token"),
        );
        assert!(cookie_matches(&headers, &state.auth));
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("rterm_web_auth=wrong"),
        );
        assert!(!cookie_matches(&headers, &state.auth));
    }

    #[test]
    fn bootstrap_ticket_is_single_use_and_rotates_into_a_session_token() {
        let now = Instant::now();
        let auth = AuthState::new("bootstrap-ticket".into(), now + Duration::from_secs(60));

        let session_token = auth.redeem("bootstrap-ticket", now).unwrap();
        assert_ne!(session_token, "bootstrap-ticket");
        assert!(auth.matches_session(&session_token));
        assert!(auth.redeem("bootstrap-ticket", now).is_none());
    }

    #[test]
    fn invalid_or_expired_bootstrap_ticket_cannot_create_a_session() {
        let now = Instant::now();
        let auth = AuthState::new("bootstrap-ticket".into(), now + Duration::from_secs(60));
        assert!(auth.redeem("wrong-ticket", now).is_none());
        assert!(
            auth.redeem("bootstrap-ticket", now + Duration::from_secs(60))
                .is_none()
        );
        assert!(!auth.matches_session("bootstrap-ticket"));
    }

    #[test]
    fn websocket_authentication_requires_exact_host_and_origin() {
        let state = state();
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("127.0.0.1:7798"));
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("http://127.0.0.1:7798"),
        );
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("rterm_web_auth=test-token"),
        );
        assert!(websocket_authenticated(&headers, &state));

        headers.insert(header::HOST, HeaderValue::from_static("localhost:7798"));
        assert!(!websocket_authenticated(&headers, &state));
        headers.insert(header::HOST, HeaderValue::from_static("127.0.0.1:7798"));
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("http://localhost:5173"),
        );
        assert!(!websocket_authenticated(&headers, &state));
    }

    #[test]
    fn token_comparison_checks_content_and_length() {
        assert!(constant_time_equal(b"abc", b"abc"));
        assert!(!constant_time_equal(b"abc", b"abd"));
        assert!(!constant_time_equal(b"abc", b"ab"));
    }

    #[test]
    fn open_validation_accepts_only_the_server_profile() {
        let request = super::OpenRequest {
            protocol: 1,
            cols: 80,
            rows: 24,
            profile: "local-default".to_owned(),
        };
        assert_eq!(
            validate_open(&request),
            Ok(TerminalDimensions { cols: 80, rows: 24 })
        );
        let mut request = request;
        request.profile = "arbitrary-command".to_owned();
        assert!(validate_open(&request).is_err());
    }

    #[test]
    fn http_responses_emit_security_and_cache_headers() {
        let response = harden_bootstrap_response(error_response(
            StatusCode::UNAUTHORIZED,
            "authentication required",
        ));
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store, max-age=0"
        );
        assert_eq!(
            response.headers().get("referrer-policy").unwrap(),
            "no-referrer"
        );
        assert!(
            response
                .headers()
                .get(header::CONTENT_SECURITY_POLICY)
                .unwrap()
                .to_str()
                .unwrap()
                .contains("frame-ancestors 'none'")
        );
        assert_eq!(
            response.headers().get("x-content-type-options").unwrap(),
            "nosniff"
        );
        assert_eq!(
            response.headers().get("permissions-policy").unwrap(),
            "clipboard-read=(self), clipboard-write=(self)"
        );
    }

    #[tokio::test]
    async fn second_server_cannot_bind_the_same_listener() {
        let first = WebServer::bind(WebServerConfig {
            listen: "127.0.0.1:0".parse().unwrap(),
            web_root: PathBuf::from("web/dist"),
            max_sessions: 1,
            allowed_origin: None,
        })
        .await
        .unwrap();
        let address = first.local_addr();
        let second = WebServer::bind(WebServerConfig {
            listen: address,
            web_root: PathBuf::from("web/dist"),
            max_sessions: 1,
            allowed_origin: None,
        })
        .await;
        assert!(second.is_err());
        assert_eq!(first.local_addr(), address);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[expect(
        clippy::too_many_lines,
        reason = "the full WebSocket and PTY protocol round trip belongs in one integration test"
    )]
    async fn authenticated_websocket_round_trips_a_real_pty() {
        let server = WebServer::bind(WebServerConfig {
            listen: "127.0.0.1:0".parse().unwrap(),
            web_root: PathBuf::from("web/dist"),
            max_sessions: 1,
            allowed_origin: None,
        })
        .await
        .unwrap();
        let address = server.local_addr();
        let host = host_for_address(address);
        let token = server.bootstrap_token.to_string();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let server_task = tokio::spawn(async move {
            server
                .run_until(async move {
                    let _ = shutdown_rx.await;
                })
                .await
                .unwrap();
        });

        let cookie = bootstrap_cookie(address, &token).await;
        assert!(
            !cookie.ends_with(&token),
            "bootstrap ticket must not become the session cookie"
        );
        let mut request = format!("ws://{host}/api/v1/terminal")
            .into_client_request()
            .unwrap();
        request
            .headers_mut()
            .insert("Host", HeaderValue::from_str(&host).unwrap());
        request.headers_mut().insert(
            "Origin",
            HeaderValue::from_str(&format!("http://{host}")).unwrap(),
        );
        request
            .headers_mut()
            .insert("Cookie", HeaderValue::from_str(&cookie).unwrap());
        let (mut socket, _) = connect_async(request).await.unwrap();
        socket
            .send(Message::Text(
                r#"{"type":"open","protocol":1,"cols":80,"rows":24,"profile":"local-default"}"#
                    .into(),
            ))
            .await
            .unwrap();

        let opened = time::timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let Message::Text(opened) = opened else {
            panic!("opening a PTY must return session metadata");
        };
        let opened: serde_json::Value = serde_json::from_str(&opened).unwrap();
        assert_eq!(opened["type"], "opened");
        let command = PtyCommand::default_shell();
        let shell = Path::new(command.program())
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| command.program());
        assert_eq!(opened["shell"], shell);
        assert_eq!(
            opened["cwd"],
            std::env::current_dir().unwrap().to_string_lossy().as_ref()
        );

        let input = if cfg!(windows) {
            b"echo web-socket-test\r\nexit\r\n".to_vec()
        } else {
            b"printf web-socket-test\nexit\n".to_vec()
        };
        let mut input_sent = false;
        if !cfg!(windows) {
            socket
                .send(Message::Binary(input.clone().into()))
                .await
                .unwrap();
            input_sent = true;
        }
        let mut output = Vec::new();
        let mut saw_exit = false;
        while let Some(message) = time::timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
        {
            match message.unwrap() {
                Message::Binary(bytes) => {
                    output.extend_from_slice(&bytes);
                    if cfg!(windows)
                        && !input_sent
                        && output.windows(4).any(|window| window == b"\x1b[6n")
                    {
                        socket
                            .send(Message::Binary(b"\x1b[1;1R".to_vec().into()))
                            .await
                            .unwrap();
                        socket
                            .send(Message::Binary(input.clone().into()))
                            .await
                            .unwrap();
                        input_sent = true;
                    }
                }
                Message::Text(text) if text.contains("\"exit\"") => {
                    saw_exit = true;
                    break;
                }
                _ => {}
            }
        }
        assert!(input_sent, "terminal never became ready for input");
        assert!(saw_exit, "websocket never delivered PTY exit message");
        assert!(String::from_utf8_lossy(&output).contains("web-socket-test"));
        drop(socket);

        let _ = shutdown_tx.send(());
        time::timeout(Duration::from_secs(3), server_task)
            .await
            .expect("web server did not shut down")
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn authenticated_websocket_reports_real_input_backpressure_and_cleans_up() {
        let server = WebServer::bind(WebServerConfig {
            listen: "127.0.0.1:0".parse().unwrap(),
            web_root: PathBuf::from("web/dist"),
            max_sessions: 1,
            allowed_origin: None,
        })
        .await
        .unwrap();
        let address = server.local_addr();
        let host = host_for_address(address);
        let token = server.bootstrap_token.to_string();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let server_task = tokio::spawn(async move {
            server
                .run_until(async move {
                    let _ = shutdown_rx.await;
                })
                .await
                .unwrap();
        });

        let cookie = bootstrap_cookie(address, &token).await;
        let mut request = format!("ws://{host}/api/v1/terminal")
            .into_client_request()
            .unwrap();
        request
            .headers_mut()
            .insert("Host", HeaderValue::from_str(&host).unwrap());
        request.headers_mut().insert(
            "Origin",
            HeaderValue::from_str(&format!("http://{host}")).unwrap(),
        );
        request
            .headers_mut()
            .insert("Cookie", HeaderValue::from_str(&cookie).unwrap());
        let (mut socket, _) = connect_async(request).await.unwrap();
        socket
            .send(Message::Text(
                r#"{"type":"open","protocol":1,"cols":80,"rows":24,"profile":"local-default"}"#
                    .into(),
            ))
            .await
            .unwrap();
        let opened = time::timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(matches!(opened, Message::Text(text) if text.contains("\"opened\"")));

        let saw_backpressure = {
            let (mut sender, mut receiver) = socket.split();
            let payload = vec![b'x'; crate::protocol::MAX_INPUT_FRAME_BYTES];
            let send_frames = async {
                for _ in 0..(crate::session::INPUT_QUEUE_CAPACITY * 2) {
                    sender.send(Message::Binary(payload.clone().into())).await?;
                    tokio::task::yield_now().await;
                }
                Ok::<(), tokio_tungstenite::tungstenite::Error>(())
            };
            let observe_backpressure = async {
                loop {
                    match time::timeout(Duration::from_secs(3), receiver.next()).await {
                        Ok(Some(Ok(Message::Text(text))))
                            if text.contains("\"code\":\"INPUT_BACKPRESSURE\"") =>
                        {
                            return true;
                        }
                        Ok(Some(Ok(_))) => {}
                        Ok(Some(Err(error))) => {
                            panic!("WebSocket failed before reporting input pressure: {error}")
                        }
                        Ok(None) | Err(_) => return false,
                    }
                }
            };
            tokio::pin!(send_frames);
            tokio::pin!(observe_backpressure);
            tokio::select! {
                biased;
                observed = &mut observe_backpressure => observed,
                sent = &mut send_frames => {
                    if let Err(error) = sent {
                        let observed = observe_backpressure.await;
                        assert!(observed, "WebSocket send failed before input pressure was observable: {error}");
                        observed
                    } else {
                        observe_backpressure.await
                    }
                }
            }
        };
        assert!(
            saw_backpressure,
            "real authenticated WebSocket never reported bounded PTY input pressure"
        );

        let _ = shutdown_tx.send(());
        time::timeout(Duration::from_secs(3), server_task)
            .await
            .expect("web server did not shut down after backpressure")
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn second_authenticated_websocket_is_rejected_at_the_session_limit() {
        let server = WebServer::bind(WebServerConfig {
            listen: "127.0.0.1:0".parse().unwrap(),
            web_root: PathBuf::from("web/dist"),
            max_sessions: 1,
            allowed_origin: None,
        })
        .await
        .unwrap();
        let address = server.local_addr();
        let host = host_for_address(address);
        let token = server.bootstrap_token.to_string();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let server_task = tokio::spawn(async move {
            server
                .run_until(async move {
                    let _ = shutdown_rx.await;
                })
                .await
                .unwrap();
        });
        let cookie = bootstrap_cookie(address, &token).await;
        let request = || {
            let mut request = format!("ws://{host}/api/v1/terminal")
                .into_client_request()
                .unwrap();
            request
                .headers_mut()
                .insert("Host", HeaderValue::from_str(&host).unwrap());
            request.headers_mut().insert(
                "Origin",
                HeaderValue::from_str(&format!("http://{host}")).unwrap(),
            );
            request
                .headers_mut()
                .insert("Cookie", HeaderValue::from_str(&cookie).unwrap());
            request
        };

        let (first, _) = connect_async(request()).await.unwrap();
        let error = connect_async(request())
            .await
            .expect_err("second active session must be rejected");
        let tokio_tungstenite::tungstenite::Error::Http(response) = error else {
            panic!("expected HTTP rejection, observed {error:?}");
        };
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        drop(first);
        let _ = shutdown_tx.send(());
        time::timeout(Duration::from_secs(3), server_task)
            .await
            .expect("web server did not shut down")
            .unwrap();
    }

    async fn bootstrap_cookie(address: SocketAddr, token: &str) -> String {
        let mut stream = TcpStream::connect(address).await.unwrap();
        let request = format!(
            "GET /?token={token} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            host_for_address(address)
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        let response = String::from_utf8(response).unwrap();
        response
            .lines()
            .find_map(|line| {
                line.strip_prefix("set-cookie: ")
                    .or_else(|| line.strip_prefix("Set-Cookie: "))
            })
            .and_then(|cookie| cookie.split(';').next())
            .map(str::to_owned)
            .expect("bootstrap response should set the authentication cookie")
    }
}
