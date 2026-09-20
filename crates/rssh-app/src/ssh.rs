use std::{
    error::Error,
    io::{self, Read, Write},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};

use rssh_core::{
    SessionId,
    session::{SessionLifecycle, SessionState},
};
use rssh_pty::{PtyCommand, PtyExitStatus, PtySize};
use rssh_ssh::{
    RusshChannelOpener, RusshDirectTcpIpOpenPlan, RusshForwardCancellation, RusshForwardDeadlines,
    RusshHostKeyPolicy, RusshPrivateKeyAuth, RusshRemoteTcpIpForwardPlan, SshAuthMethod,
    SshChannelConnector, SshConnectRequest, SshInputEvent, SshInputEventReceiver, SshSessionConfig,
    SshSessionStartup, SshShellConnector, SshTransport, ssh_input_event_channel,
};
use serde::Serialize;

use crate::{
    cli::{LocalOptions, NativeHostKeyPolicy, OpenSshTarget, SshForward, SshOptions, SshTarget},
    local,
};

type SecretPrompt<'a> = dyn FnMut(&SshConnectRequest) -> Result<String, Box<dyn Error>> + 'a;
type KeyPassphrasePrompt<'a> = dyn FnMut(&Path) -> Result<String, Box<dyn Error>> + 'a;
type KeyPassphraseDetector<'a> = dyn FnMut(&Path) -> Result<bool, Box<dyn Error>> + 'a;
type OpenSshConfigResolver<'a> = dyn FnMut(&OpenSshTarget) -> Result<String, Box<dyn Error>> + 'a;

const NATIVE_SSH_SESSION_ID: SessionId = SessionId::new(2);
const NATIVE_FORWARD_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
const NATIVE_FORWARD_STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
const NATIVE_REMOTE_FORWARD_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(1);
const NATIVE_FORWARD_REPLY_TIMEOUT: Duration = Duration::from_secs(1);
const NATIVE_FORWARD_ACCEPT_POLL: Duration = Duration::from_millis(5);

struct NativeSecretPrompts<'a> {
    password_prompt: &'a mut SecretPrompt<'a>,
    key_passphrase_prompt: &'a mut KeyPassphrasePrompt<'a>,
    key_needs_passphrase: &'a mut KeyPassphraseDetector<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeLocalForward {
    bind_host: String,
    bind_port: u16,
    target_host: String,
    target_port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeDynamicForward {
    bind_host: String,
    bind_port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeRemoteForward {
    bind_host: String,
    bind_port: u16,
    target_host: String,
    target_port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct NativeForwardPlan {
    local: Vec<NativeLocalForward>,
    dynamic: Vec<NativeDynamicForward>,
    remote: Vec<NativeRemoteForward>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Socks5ConnectRequest {
    target_host: String,
    target_port: u16,
}

trait NativeLocalForwardStarter {
    fn start(
        &mut self,
        request: SshConnectRequest,
        forward: NativeLocalForward,
    ) -> Result<Box<dyn NativeLocalForwardHandle>, Box<dyn Error>>;

    fn start_dynamic(
        &mut self,
        request: SshConnectRequest,
        forward: NativeDynamicForward,
    ) -> Result<Box<dyn NativeLocalForwardHandle>, Box<dyn Error>>;

    fn start_remote(
        &mut self,
        request: SshConnectRequest,
        forward: NativeRemoteForward,
    ) -> Result<Box<dyn NativeLocalForwardHandle>, Box<dyn Error>>;
}

trait NativeLocalForwardHandle {
    fn is_finished(&self) -> bool;

    fn cancel(&mut self);

    fn shutdown(&mut self, timeout: Duration) -> Result<(), Box<dyn Error>>;

    fn wait(&mut self) -> Result<(), Box<dyn Error>>;
}

#[derive(Clone)]
struct ThreadedNativeLocalForwardStarter {
    opener: RusshChannelOpener,
}

impl ThreadedNativeLocalForwardStarter {
    fn new(opener: RusshChannelOpener) -> Self {
        Self { opener }
    }
}

impl NativeLocalForwardStarter for ThreadedNativeLocalForwardStarter {
    fn start(
        &mut self,
        request: SshConnectRequest,
        forward: NativeLocalForward,
    ) -> Result<Box<dyn NativeLocalForwardHandle>, Box<dyn Error>> {
        let listener = TcpListener::bind((forward.bind_host.as_str(), forward.bind_port))?;
        listener.set_nonblocking(true)?;
        let opener = self.opener.clone();
        Ok(Box::new(ThreadedNativeLocalForwardHandle::spawn(
            move |cancellation| {
                run_native_local_forward_listener(
                    &listener,
                    &opener,
                    &request,
                    &forward,
                    &cancellation,
                )
                .map_err(|error| error.to_string())
            },
        )))
    }

    fn start_dynamic(
        &mut self,
        request: SshConnectRequest,
        forward: NativeDynamicForward,
    ) -> Result<Box<dyn NativeLocalForwardHandle>, Box<dyn Error>> {
        let listener = TcpListener::bind((forward.bind_host.as_str(), forward.bind_port))?;
        listener.set_nonblocking(true)?;
        let opener = self.opener.clone();
        Ok(Box::new(ThreadedNativeLocalForwardHandle::spawn(
            move |cancellation| {
                run_native_dynamic_forward_listener(&listener, &opener, &request, &cancellation)
                    .map_err(|error| error.to_string())
            },
        )))
    }

    fn start_remote(
        &mut self,
        request: SshConnectRequest,
        forward: NativeRemoteForward,
    ) -> Result<Box<dyn NativeLocalForwardHandle>, Box<dyn Error>> {
        let remote_forward_plan = native_remote_tcpip_plan_for_remote_forward(&forward);
        let cancellation = RusshForwardCancellation::new();
        let mut remote_forward = self.opener.open_remote_tcpip_forward_with_lifecycle(
            &request,
            &remote_forward_plan,
            &cancellation,
            RusshForwardDeadlines::new(
                NATIVE_FORWARD_STARTUP_TIMEOUT,
                NATIVE_REMOTE_FORWARD_SHUTDOWN_TIMEOUT,
            ),
        )?;
        Ok(Box::new(
            ThreadedNativeLocalForwardHandle::spawn_with_cancellation(
                cancellation,
                move |cancellation| {
                    remote_forward
                        .wait_until_cancelled(&cancellation, NATIVE_REMOTE_FORWARD_SHUTDOWN_TIMEOUT)
                        .map_err(|error| error.to_string())
                },
            ),
        ))
    }
}

struct ThreadedNativeLocalForwardHandle {
    cancellation: RusshForwardCancellation,
    completion: std::sync::mpsc::Receiver<Result<(), String>>,
    join_handle: Option<thread::JoinHandle<Result<(), String>>>,
}

impl ThreadedNativeLocalForwardHandle {
    fn spawn(
        worker: impl FnOnce(RusshForwardCancellation) -> Result<(), String> + Send + 'static,
    ) -> Self {
        let cancellation = RusshForwardCancellation::new();
        Self::spawn_with_cancellation(cancellation, worker)
    }

    fn spawn_with_cancellation(
        cancellation: RusshForwardCancellation,
        worker: impl FnOnce(RusshForwardCancellation) -> Result<(), String> + Send + 'static,
    ) -> Self {
        let worker_cancellation = cancellation.clone();
        let (completion_sender, completion) = std::sync::mpsc::sync_channel(1);
        let join_handle = thread::spawn(move || {
            let result = worker(worker_cancellation);
            let completion_result = match &result {
                Ok(()) => Ok(()),
                Err(error) => Err(error.clone()),
            };
            let _ = completion_sender.send(completion_result);
            result
        });
        Self {
            cancellation,
            completion,
            join_handle: Some(join_handle),
        }
    }

    fn join_after_completion(&mut self, timeout: Option<Duration>) -> Result<(), Box<dyn Error>> {
        if self.join_handle.is_none() {
            return Ok(());
        }
        let completion = match timeout {
            Some(timeout) => match self.completion.recv_timeout(timeout) {
                Ok(completion) => Some(completion),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    return Err("native SSH forwarding shutdown timed out".into());
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => None,
            },
            None => self.completion.recv().ok(),
        };
        let join_handle = self.join_handle.take().expect("join handle checked above");
        let joined = join_handle
            .join()
            .map_err(|_| "native SSH forwarding worker panicked")?;
        if let Some(completion) = completion {
            completion?;
        }
        joined.map_err(Into::into)
    }
}

impl NativeLocalForwardHandle for ThreadedNativeLocalForwardHandle {
    fn is_finished(&self) -> bool {
        self.join_handle
            .as_ref()
            .is_none_or(thread::JoinHandle::is_finished)
    }

    fn cancel(&mut self) {
        self.cancellation.cancel();
    }

    fn shutdown(&mut self, timeout: Duration) -> Result<(), Box<dyn Error>> {
        self.cancel();
        self.join_after_completion(Some(timeout))
    }

    fn wait(&mut self) -> Result<(), Box<dyn Error>> {
        self.join_after_completion(None)
    }
}

impl Drop for ThreadedNativeLocalForwardHandle {
    fn drop(&mut self) {
        // Best-effort bounded fallback for cooperative workers. Normal paths
        // call shutdown with the full forwarding deadline before Drop.
        let _ = self.shutdown(Duration::from_millis(250));
    }
}

fn shutdown_native_forward_handles(
    handles: &mut [Box<dyn NativeLocalForwardHandle>],
    timeout: Duration,
) -> Result<(), Box<dyn Error>> {
    for handle in handles.iter_mut() {
        handle.cancel();
    }

    let deadline = Instant::now() + timeout;
    let mut first_error = None;
    for handle in handles.iter_mut() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            first_error.get_or_insert_with(|| {
                Box::<dyn Error>::from("native SSH forwarding shutdown timed out")
            });
            continue;
        }
        if let Err(error) = handle.shutdown(remaining) {
            first_error.get_or_insert(error);
        }
    }

    first_error.map_or(Ok(()), Err)
}

fn wait_for_native_forward_handles(
    handles: &mut [Box<dyn NativeLocalForwardHandle>],
    shutdown_timeout: Duration,
) -> Result<(), Box<dyn Error>> {
    loop {
        if let Some(index) = handles.iter().position(|handle| handle.is_finished()) {
            let completion_result = handles[index].wait();
            let shutdown_result = shutdown_native_forward_handles(handles, shutdown_timeout);
            return match completion_result {
                Err(error) => Err(error),
                Ok(()) => shutdown_result,
            };
        }
        thread::park_timeout(NATIVE_FORWARD_ACCEPT_POLL);
    }
}

fn retain_started_native_forward(
    handles: &mut Vec<Box<dyn NativeLocalForwardHandle>>,
    started: Result<Box<dyn NativeLocalForwardHandle>, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    match started {
        Ok(handle) => {
            handles.push(handle);
            Ok(())
        }
        Err(startup_error) => {
            let _ = shutdown_native_forward_handles(handles, NATIVE_FORWARD_SHUTDOWN_TIMEOUT);
            Err(startup_error)
        }
    }
}

pub fn run(options: &SshOptions) -> Result<PtyExitStatus, Box<dyn Error>> {
    if options.native {
        return run_native(options);
    }

    let local_options = local_options_for_options(options)?;

    local::run(&local_options)
}

fn run_native(options: &SshOptions) -> Result<PtyExitStatus, Box<dyn Error>> {
    let mut connector = SshChannelConnector::new(native_channel_opener_for_options(options));
    let mut forward_starter =
        ThreadedNativeLocalForwardStarter::new(native_channel_opener_for_options(options));
    let stdout = io::stdout();
    let mut output = stdout.lock();

    run_native_with_connector_prompt_and_io(
        options,
        &mut connector,
        &mut forward_starter,
        &mut |request| native_password_prompt(request),
        NativeSshInput::StdinBroker,
        &mut output,
    )
}

enum NativeSshInput {
    Events(SshInputEventReceiver),
    StdinBroker,
}

impl NativeSshInput {
    fn acquire_after_connect(self) -> Result<SshInputEventReceiver, Box<dyn Error>> {
        match self {
            Self::Events(receiver) => Ok(receiver),
            Self::StdinBroker => native_stdin_event_receiver(),
        }
    }
}

impl From<SshInputEventReceiver> for NativeSshInput {
    fn from(receiver: SshInputEventReceiver) -> Self {
        Self::Events(receiver)
    }
}

struct NativeStdinBroker {
    receiver: std::sync::Mutex<Option<SshInputEventReceiver>>,
}

impl NativeStdinBroker {
    fn start() -> Result<Self, String> {
        let (sender, receiver) = ssh_input_event_channel(32);
        thread::Builder::new()
            .name("rssh-native-stdin-broker".to_owned())
            .spawn(move || {
                let mut stdin = io::stdin();
                let mut buffer = [0_u8; 8192];
                loop {
                    match stdin.read(&mut buffer) {
                        Ok(0) => {
                            let _ = sender.send(SshInputEvent::Eof);
                            return;
                        }
                        Ok(count) => {
                            if sender
                                .send(SshInputEvent::Data(buffer[..count].to_vec()))
                                .is_err()
                            {
                                return;
                            }
                        }
                        Err(error) => {
                            let _ = sender.send(SshInputEvent::Error(error.to_string()));
                            return;
                        }
                    }
                }
            })
            .map_err(|error| format!("native stdin broker creation failed: {error}"))?;
        Ok(Self {
            receiver: std::sync::Mutex::new(Some(receiver)),
        })
    }

    fn take_receiver(&self) -> Result<SshInputEventReceiver, Box<dyn Error>> {
        self.receiver
            .lock()
            .map_err(|_| "native stdin broker lock poisoned")?
            .take()
            .ok_or_else(|| {
                "native stdin broker supports one CLI SSH session per process"
                    .to_owned()
                    .into()
            })
    }
}

/// Returns the process-lifetime native stdin event stream.
///
/// The broker thread is created once, owns `stdin`, and serves the single SSH
/// CLI session supported by this process. Dropping the session receiver
/// disconnects any pending bounded send; a broker blocked inside the OS stdin
/// read is intentionally allowed to live until process exit.
fn native_stdin_event_receiver() -> Result<SshInputEventReceiver, Box<dyn Error>> {
    static BROKER: std::sync::OnceLock<Result<NativeStdinBroker, String>> =
        std::sync::OnceLock::new();
    match BROKER.get_or_init(NativeStdinBroker::start) {
        Ok(broker) => broker.take_receiver(),
        Err(error) => Err(error.clone().into()),
    }
}

fn native_channel_opener_for_options(options: &SshOptions) -> RusshChannelOpener {
    let opener = RusshChannelOpener::default();
    match options.native_host_key_policy {
        NativeHostKeyPolicy::RejectUnknown => opener,
        NativeHostKeyPolicy::AcceptUnknown => {
            opener.with_host_key_policy(RusshHostKeyPolicy::AcceptUnknown)
        }
        NativeHostKeyPolicy::TrustOnFirstUse => {
            let opener = opener.with_host_key_policy(RusshHostKeyPolicy::TrustOnFirstUse);
            if let Some(path) = default_known_hosts_path() {
                return opener.with_known_hosts_path(path);
            }
            opener
        }
    }
}

fn default_known_hosts_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".ssh").join("known_hosts"))
}

#[must_use]
fn openssh_command_for_options(options: &SshOptions) -> PtyCommand {
    let mut command = match &options.target {
        SshTarget::Direct(request) => openssh_command_for_request(request, options),
        SshTarget::OpenSsh(target) => openssh_command_for_target(target, options),
    };

    if !options.remote_command.is_empty() {
        command = command.with_args(options.remote_command.iter().cloned());
    }

    command
}

#[cfg(test)]
fn native_request_for_options(options: &SshOptions) -> Result<SshConnectRequest, Box<dyn Error>> {
    native_request_for_options_with_password_prompt(options, &mut |_| {
        Err("native SSH password prompt requires a password provider".into())
    })
}

#[cfg(test)]
fn native_request_for_options_with_password_prompt(
    options: &SshOptions,
    password_prompt: &mut SecretPrompt<'_>,
) -> Result<SshConnectRequest, Box<dyn Error>> {
    native_request_for_options_with_secret_prompts(
        options,
        password_prompt,
        &mut |_| {
            Err("native SSH private-key passphrase prompt requires a passphrase provider".into())
        },
        &mut native_key_needs_passphrase,
    )
}

#[cfg(test)]
fn native_request_for_options_with_secret_prompts(
    options: &SshOptions,
    password_prompt: &mut SecretPrompt<'_>,
    key_passphrase_prompt: &mut KeyPassphrasePrompt<'_>,
    key_needs_passphrase: &mut KeyPassphraseDetector<'_>,
) -> Result<SshConnectRequest, Box<dyn Error>> {
    native_request_for_options_with_resolver_secret_prompts(
        options,
        &mut resolve_openssh_config_target,
        password_prompt,
        key_passphrase_prompt,
        key_needs_passphrase,
    )
}

fn native_request_for_options_with_resolver_secret_prompts(
    options: &SshOptions,
    openssh_config_resolver: &mut OpenSshConfigResolver<'_>,
    password_prompt: &mut SecretPrompt<'_>,
    key_passphrase_prompt: &mut KeyPassphrasePrompt<'_>,
    key_needs_passphrase: &mut KeyPassphraseDetector<'_>,
) -> Result<SshConnectRequest, Box<dyn Error>> {
    let request = match &options.target {
        SshTarget::Direct(request) => request.clone(),
        SshTarget::OpenSsh(target) => {
            let config_output = openssh_config_resolver(target)?;
            native_request_for_openssh_target_with_config_output(target, &config_output)?
        }
    };

    native_forward_plan_for_options(options)?;

    let startup = if options.no_shell {
        SshSessionStartup::NoShell
    } else if options.remote_command.is_empty() {
        SshSessionStartup::Shell
    } else {
        SshSessionStartup::command(options.remote_command.clone())?
    };

    let mut request = request.with_startup(startup);
    if matches!(request.auth, SshAuthMethod::PasswordPrompt) {
        request.auth = SshAuthMethod::password(password_prompt(&request)?)?;
    }
    if let SshAuthMethod::PrivateKey { path, passphrase } = &mut request.auth
        && passphrase.is_none()
        && key_needs_passphrase(path)?
    {
        *passphrase = Some(key_passphrase_prompt(path)?);
    }

    Ok(request)
}

fn native_request_for_openssh_target_with_config_output(
    target: &OpenSshTarget,
    config_output: &str,
) -> Result<SshConnectRequest, Box<dyn Error>> {
    let mut resolved_host = None;
    let mut resolved_user = None;
    let mut resolved_port = None;

    for line in config_output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, char::is_whitespace);
        let Some(key) = parts.next() else {
            continue;
        };
        let Some(value) = parts
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };

        match key.to_ascii_lowercase().as_str() {
            "hostname" => resolved_host = Some(value.to_owned()),
            "user" => resolved_user = Some(value.to_owned()),
            "port" => resolved_port = Some(value.parse::<u16>()?),
            _ => {}
        }
    }

    let host = resolved_host.unwrap_or_else(|| target.target.clone());
    let username = target
        .username
        .clone()
        .or(resolved_user)
        .ok_or("OpenSSH target did not resolve a user")?;
    let port = target.port.or(resolved_port).unwrap_or(22);
    let config = SshSessionConfig::try_new(host, port, username, target.initial_size.into())?;

    Ok(SshConnectRequest::new(config, target.auth.clone()))
}

fn resolve_openssh_config_target(target: &OpenSshTarget) -> Result<String, Box<dyn Error>> {
    let output = Command::new("ssh").arg("-G").arg(&target.target).output()?;
    if !output.status.success() {
        return Err(format!(
            "OpenSSH config resolution failed for target {}",
            target.target
        )
        .into());
    }

    String::from_utf8(output.stdout).map_err(Into::into)
}

fn native_forward_plan_for_options(
    options: &SshOptions,
) -> Result<NativeForwardPlan, Box<dyn Error>> {
    let mut plan = NativeForwardPlan::default();

    for forward in &options.forwards {
        match forward {
            SshForward::Local(spec) => plan.local.push(parse_native_local_forward(spec)?),
            SshForward::Dynamic(spec) => {
                plan.dynamic.push(parse_native_dynamic_forward(spec)?);
            }
            SshForward::Remote(spec) => {
                plan.remote.push(parse_native_remote_forward(spec)?);
            }
        }
    }

    Ok(plan)
}

fn parse_native_local_forward(spec: &str) -> Result<NativeLocalForward, Box<dyn Error>> {
    let parts = spec.split(':').collect::<Vec<_>>();
    let (bind_host, bind_port, target_host, target_port) = match parts.as_slice() {
        [bind_port, target_host, target_port] => {
            ("127.0.0.1", *bind_port, *target_host, *target_port)
        }
        [bind_host, bind_port, target_host, target_port] => {
            (*bind_host, *bind_port, *target_host, *target_port)
        }
        _ => {
            return Err(format!(
                "invalid native local-forward spec {spec:?}; expected [bind_host:]bind_port:target_host:target_port"
            )
            .into());
        }
    };

    if bind_host.trim().is_empty() || target_host.trim().is_empty() {
        return Err(
            format!("invalid native local-forward spec {spec:?}; host cannot be empty").into(),
        );
    }

    Ok(NativeLocalForward {
        bind_host: bind_host.to_owned(),
        bind_port: parse_forward_port(bind_port, "bind port")?,
        target_host: target_host.to_owned(),
        target_port: parse_forward_port(target_port, "target port")?,
    })
}

fn parse_native_dynamic_forward(spec: &str) -> Result<NativeDynamicForward, Box<dyn Error>> {
    let parts = spec.split(':').collect::<Vec<_>>();
    let (bind_host, bind_port) = match parts.as_slice() {
        [bind_port] => ("127.0.0.1", *bind_port),
        [bind_host, bind_port] => (*bind_host, *bind_port),
        _ => {
            return Err(format!(
                "invalid native dynamic-forward spec {spec:?}; expected [bind_host:]bind_port"
            )
            .into());
        }
    };

    if bind_host.trim().is_empty() {
        return Err(
            format!("invalid native dynamic-forward spec {spec:?}; host cannot be empty").into(),
        );
    }
    if !bind_host.eq_ignore_ascii_case("localhost")
        && !bind_host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
    {
        return Err(format!(
            "native dynamic-forward rejects non-loopback bind address {bind_host:?}; the SOCKS5 listener has no authentication"
        )
        .into());
    }

    Ok(NativeDynamicForward {
        bind_host: bind_host.to_owned(),
        bind_port: parse_forward_port(bind_port, "bind port")?,
    })
}

fn parse_native_remote_forward(spec: &str) -> Result<NativeRemoteForward, Box<dyn Error>> {
    let parts = spec.split(':').collect::<Vec<_>>();
    let (bind_host, bind_port, target_host, target_port) = match parts.as_slice() {
        [bind_port, target_host, target_port] => {
            ("127.0.0.1", *bind_port, *target_host, *target_port)
        }
        [bind_host, bind_port, target_host, target_port] => {
            (*bind_host, *bind_port, *target_host, *target_port)
        }
        _ => {
            return Err(format!(
                "invalid native remote-forward spec {spec:?}; expected [bind_host:]bind_port:target_host:target_port"
            )
            .into());
        }
    };

    if bind_host.trim().is_empty() || target_host.trim().is_empty() {
        return Err(
            format!("invalid native remote-forward spec {spec:?}; host cannot be empty").into(),
        );
    }

    Ok(NativeRemoteForward {
        bind_host: bind_host.to_owned(),
        bind_port: parse_forward_port(bind_port, "bind port")?,
        target_host: target_host.to_owned(),
        target_port: parse_forward_port(target_port, "target port")?,
    })
}

fn parse_forward_port(value: &str, name: &str) -> Result<u16, Box<dyn Error>> {
    let port = value
        .parse::<u16>()
        .map_err(|_| format!("invalid native local-forward {name}: {value}"))?;
    if port == 0 {
        return Err(format!("invalid native local-forward {name}: {value}").into());
    }
    Ok(port)
}

fn read_socks5_connect_request(
    input: &mut dyn Read,
    output: &mut dyn Write,
) -> Result<Socks5ConnectRequest, Box<dyn Error>> {
    let mut greeting = [0; 2];
    input.read_exact(&mut greeting)?;
    if greeting[0] != 0x05 {
        return Err("SOCKS5 greeting must start with version 5".into());
    }

    let method_count = usize::from(greeting[1]);
    let mut methods = vec![0; method_count];
    input.read_exact(&mut methods)?;
    if !methods.contains(&0x00) {
        output.write_all(&[0x05, 0xff])?;
        output.flush()?;
        return Err("SOCKS5 client did not offer no-auth authentication".into());
    }
    output.write_all(&[0x05, 0x00])?;
    output.flush()?;

    let mut header = [0; 4];
    input.read_exact(&mut header)?;
    if header[0] != 0x05 || header[1] != 0x01 || header[2] != 0x00 {
        return Err("SOCKS5 request must be a CONNECT request".into());
    }

    let target_host = match header[3] {
        0x01 => {
            let mut octets = [0; 4];
            input.read_exact(&mut octets)?;
            Ipv4Addr::from(octets).to_string()
        }
        0x03 => {
            let mut length = [0; 1];
            input.read_exact(&mut length)?;
            let mut host = vec![0; usize::from(length[0])];
            input.read_exact(&mut host)?;
            String::from_utf8(host)?
        }
        0x04 => {
            let mut octets = [0; 16];
            input.read_exact(&mut octets)?;
            Ipv6Addr::from(octets).to_string()
        }
        _ => return Err("SOCKS5 address type is not supported".into()),
    };

    let mut port = [0; 2];
    input.read_exact(&mut port)?;
    let target_port = u16::from_be_bytes(port);
    if target_host.trim().is_empty() || target_port == 0 {
        return Err("SOCKS5 CONNECT target cannot be empty".into());
    }

    Ok(Socks5ConnectRequest {
        target_host,
        target_port,
    })
}

fn native_direct_tcpip_plan_for_local_forward(
    forward: &NativeLocalForward,
    originator_host: impl Into<String>,
    originator_port: u16,
) -> RusshDirectTcpIpOpenPlan {
    RusshDirectTcpIpOpenPlan::new(
        forward.target_host.clone(),
        forward.target_port,
        originator_host,
        originator_port,
    )
}

fn native_remote_tcpip_plan_for_remote_forward(
    forward: &NativeRemoteForward,
) -> RusshRemoteTcpIpForwardPlan {
    RusshRemoteTcpIpForwardPlan::new(
        forward.bind_host.clone(),
        forward.bind_port,
        forward.target_host.clone(),
        forward.target_port,
    )
}

fn run_native_local_forward_listener(
    listener: &TcpListener,
    opener: &RusshChannelOpener,
    request: &SshConnectRequest,
    forward: &NativeLocalForward,
    cancellation: &RusshForwardCancellation,
) -> Result<(), Box<dyn Error>> {
    let opener = opener.clone();
    let request = request.clone();
    let forward = forward.clone();
    run_native_forward_listener(listener, cancellation, move |stream, cancellation| {
        run_native_local_forward_connection(
            stream,
            &opener,
            request.clone(),
            &forward,
            cancellation,
        )
    })
}

fn run_native_local_forward_connection(
    local_stream: TcpStream,
    opener: &RusshChannelOpener,
    request: SshConnectRequest,
    forward: &NativeLocalForward,
    cancellation: &RusshForwardCancellation,
) -> Result<(), Box<dyn Error>> {
    let peer_addr = local_stream.peer_addr()?;
    let direct_tcpip_plan = native_direct_tcpip_plan_for_local_forward(
        forward,
        peer_addr.ip().to_string(),
        peer_addr.port(),
    );
    opener
        .forward_direct_tcpip_stream(
            request,
            &direct_tcpip_plan,
            local_stream,
            cancellation,
            RusshForwardDeadlines::new(
                NATIVE_FORWARD_STARTUP_TIMEOUT,
                NATIVE_REMOTE_FORWARD_SHUTDOWN_TIMEOUT,
            ),
        )
        .map_err(Into::into)
}

fn run_native_dynamic_forward_listener(
    listener: &TcpListener,
    opener: &RusshChannelOpener,
    request: &SshConnectRequest,
    cancellation: &RusshForwardCancellation,
) -> Result<(), Box<dyn Error>> {
    let opener = opener.clone();
    let request = request.clone();
    run_native_forward_listener(listener, cancellation, move |stream, cancellation| {
        run_native_dynamic_forward_connection(stream, &opener, request.clone(), cancellation)
    })
}

fn run_native_dynamic_forward_connection(
    local_stream: TcpStream,
    opener: &RusshChannelOpener,
    request: SshConnectRequest,
    cancellation: &RusshForwardCancellation,
) -> Result<(), Box<dyn Error>> {
    local_stream.set_nonblocking(true)?;
    let peer_addr = local_stream.peer_addr()?;
    let mut socks_input = local_stream.try_clone()?;
    let mut socks_output = local_stream.try_clone()?;
    let startup_deadline = Instant::now() + NATIVE_FORWARD_STARTUP_TIMEOUT;
    let mut cancellable_input = CancellableForwardReader {
        stream: &mut socks_input,
        cancellation,
        deadline: startup_deadline,
    };
    let mut cancellable_output = CancellableForwardWriter {
        output: &mut socks_output,
        cancellation,
        deadline: startup_deadline,
    };
    let socks_request =
        read_socks5_connect_request(&mut cancellable_input, &mut cancellable_output)?;
    if cancellation.is_cancelled() {
        return Err("native SSH forwarding cancelled".into());
    }
    let direct_tcpip_plan = RusshDirectTcpIpOpenPlan::new(
        socks_request.target_host,
        socks_request.target_port,
        peer_addr.ip().to_string(),
        peer_addr.port(),
    );
    let mut ready_sent = false;
    let result = opener.forward_direct_tcpip_stream_with_ready(
        request,
        &direct_tcpip_plan,
        local_stream,
        cancellation,
        RusshForwardDeadlines::new(
            NATIVE_FORWARD_STARTUP_TIMEOUT,
            NATIVE_REMOTE_FORWARD_SHUTDOWN_TIMEOUT,
        ),
        || {
            cancellable_output.reset_deadline(NATIVE_FORWARD_REPLY_TIMEOUT);
            write_socks5_connect_reply(&mut cancellable_output, 0x00)
                .map_err(|error| rssh_ssh::SshSessionError::new(error.to_string()))?;
            ready_sent = true;
            Ok(())
        },
    );
    if result.is_err() && !ready_sent && !cancellation.is_cancelled() {
        cancellable_output.reset_deadline(NATIVE_FORWARD_REPLY_TIMEOUT);
        let _ = write_socks5_connect_reply(&mut cancellable_output, 0x01);
    }
    result.map_err(Into::into)
}

fn run_native_forward_listener<F>(
    listener: &TcpListener,
    cancellation: &RusshForwardCancellation,
    connection: F,
) -> Result<(), Box<dyn Error>>
where
    F: Fn(TcpStream, &RusshForwardCancellation) -> Result<(), Box<dyn Error>>
        + Clone
        + Send
        + 'static,
{
    let mut workers: Vec<thread::JoinHandle<Result<(), String>>> = Vec::new();
    let mut worker_panicked = false;
    let mut listener_error = None;
    while !cancellation.is_cancelled() {
        let mut index = 0;
        while index < workers.len() {
            if workers[index].is_finished() {
                let worker = workers.swap_remove(index);
                if worker.join().is_err() {
                    worker_panicked = true;
                    cancellation.cancel();
                }
            } else {
                index += 1;
            }
        }

        match listener.accept() {
            Ok((stream, _)) => {
                if cancellation.is_cancelled() {
                    break;
                }
                let connection = connection.clone();
                let worker_cancellation = cancellation.clone();
                workers.push(thread::spawn(move || {
                    connection(stream, &worker_cancellation).map_err(|error| error.to_string())
                }));
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::park_timeout(NATIVE_FORWARD_ACCEPT_POLL);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => {
                listener_error = Some(error);
                cancellation.cancel();
            }
        }
    }

    for worker in workers {
        if worker.join().is_err() {
            worker_panicked = true;
        }
    }
    if worker_panicked {
        return Err("native SSH forwarding connection worker panicked".into());
    }
    if let Some(error) = listener_error {
        return Err(error.into());
    }
    Ok(())
}

struct CancellableForwardReader<'a> {
    stream: &'a mut TcpStream,
    cancellation: &'a RusshForwardCancellation,
    deadline: Instant,
}

impl Read for CancellableForwardReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.cancellation.is_cancelled() {
                return Err(io::Error::other("native SSH forwarding cancelled"));
            }
            if Instant::now() >= self.deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "native SSH forwarding startup timed out",
                ));
            }
            match self.stream.read(buffer) {
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
                {
                    thread::park_timeout(NATIVE_FORWARD_ACCEPT_POLL);
                }
                result => return result,
            }
        }
    }
}

struct CancellableForwardWriter<'a> {
    output: &'a mut dyn Write,
    cancellation: &'a RusshForwardCancellation,
    deadline: Instant,
}

impl CancellableForwardWriter<'_> {
    fn reset_deadline(&mut self, timeout: Duration) {
        self.deadline = Instant::now() + timeout;
    }

    fn check_lifecycle(&self) -> io::Result<()> {
        if self.cancellation.is_cancelled() {
            return Err(io::Error::other("native SSH forwarding cancelled"));
        }
        if Instant::now() >= self.deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "native SSH forwarding startup timed out",
            ));
        }
        Ok(())
    }
}

impl Write for CancellableForwardWriter<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        loop {
            self.check_lifecycle()?;
            match self.output.write(buffer) {
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
                {
                    thread::park_timeout(NATIVE_FORWARD_ACCEPT_POLL);
                }
                result => return result,
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        loop {
            self.check_lifecycle()?;
            match self.output.flush() {
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
                {
                    thread::park_timeout(NATIVE_FORWARD_ACCEPT_POLL);
                }
                result => return result,
            }
        }
    }
}

fn write_socks5_connect_reply(output: &mut dyn Write, status: u8) -> Result<(), Box<dyn Error>> {
    output.write_all(&[0x05, status, 0x00, 0x01, 0, 0, 0, 0, 0, 0])?;
    output.flush()?;
    Ok(())
}

#[must_use]
fn openssh_command_for_request(request: &SshConnectRequest, options: &SshOptions) -> PtyCommand {
    let mut args = openssh_start_args(options);

    args.extend(options.openssh_args.clone());
    append_auth_args(&mut args, &request.auth);
    append_forward_args(&mut args, &options.forwards);

    if request.config.port != 22 {
        args.push("-p".to_owned());
        args.push(request.config.port.to_string());
    }

    args.push(format!(
        "{}@{}",
        request.config.username, request.config.host
    ));

    PtyCommand::new("ssh").with_args(args)
}

#[must_use]
fn openssh_command_for_target(target: &OpenSshTarget, options: &SshOptions) -> PtyCommand {
    let mut args = openssh_start_args(options);

    args.extend(options.openssh_args.clone());
    append_auth_args(&mut args, &target.auth);
    append_forward_args(&mut args, &options.forwards);

    if let Some(username) = &target.username {
        args.push("-l".to_owned());
        args.push(username.clone());
    }
    if let Some(port) = target.port {
        args.push("-p".to_owned());
        args.push(port.to_string());
    }

    args.push(target.target.clone());

    PtyCommand::new("ssh").with_args(args)
}

fn openssh_start_args(options: &SshOptions) -> Vec<String> {
    if options.no_shell {
        vec!["-N".to_owned()]
    } else {
        vec!["-tt".to_owned()]
    }
}

fn append_auth_args(args: &mut Vec<String>, auth: &SshAuthMethod) {
    match auth {
        SshAuthMethod::PasswordPrompt | SshAuthMethod::Password { .. } => {
            args.push("-o".to_owned());
            args.push("PreferredAuthentications=password,keyboard-interactive".to_owned());
        }
        SshAuthMethod::PrivateKey { path, .. } => {
            args.push("-i".to_owned());
            args.push(path.to_string_lossy().into_owned());
        }
        SshAuthMethod::Agent => {}
    }
}

fn append_forward_args(args: &mut Vec<String>, forwards: &[SshForward]) {
    for forward in forwards {
        match forward {
            SshForward::Local(spec) => {
                args.push("-L".to_owned());
                args.push(spec.clone());
            }
            SshForward::Remote(spec) => {
                args.push("-R".to_owned());
                args.push(spec.clone());
            }
            SshForward::Dynamic(spec) => {
                args.push("-D".to_owned());
                args.push(spec.clone());
            }
        }
    }
}

fn local_options_for_options(options: &SshOptions) -> Result<LocalOptions, Box<dyn Error>> {
    let size = match &options.target {
        SshTarget::Direct(request) => request.config.initial_size.into(),
        SshTarget::OpenSsh(target) => target.initial_size,
    };

    Ok(LocalOptions {
        command: openssh_command_for_options(options),
        size: Some(PtySize::try_new(size.columns, size.rows)?),
        mouse: true,
        console: options.console,
        osc52_policy: options.osc52_policy,
        log: options.log.clone(),
    })
}

#[cfg(test)]
fn local_options_for_request(request: &SshConnectRequest) -> Result<LocalOptions, Box<dyn Error>> {
    local_options_for_options(&SshOptions {
        target: SshTarget::Direct(request.clone()),
        remote_command: Vec::new(),
        forwards: Vec::new(),
        openssh_args: Vec::new(),
        no_shell: false,
        native: false,
        gui: false,
        renderer: crate::cli::RendererMode::Auto,
        benchmark_startup: false,
        native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
        console: crate::cli::ConsoleOptions::default(),
        osc52_policy: crate::cli::Osc52Policy::default(),
        log: None,
    })
}

#[cfg(test)]
fn run_with_connector_and_io(
    options: &SshOptions,
    connector: &mut dyn SshShellConnector,
    input: SshInputEventReceiver,
    output: &mut dyn Write,
) -> Result<(), Box<dyn Error>> {
    let request = native_request_for_options(options)?;
    let session = connector.connect(request)?;
    rssh_ssh::run_connected_shell_with_events(session, input, output)
        .map(|_| ())
        .map_err(Into::into)
}

#[cfg(test)]
fn run_native_with_connector_and_io(
    options: &SshOptions,
    connector: &mut dyn SshShellConnector,
    input: impl Into<NativeSshInput>,
    output: &mut dyn Write,
) -> Result<PtyExitStatus, Box<dyn Error>> {
    let mut forward_starter = RejectingNativeLocalForwardStarter;
    run_native_with_connector_forward_starter_and_io(
        options,
        connector,
        &mut forward_starter,
        input,
        output,
    )
}

#[cfg(test)]
fn run_native_with_connector_forward_starter_and_io(
    options: &SshOptions,
    connector: &mut dyn SshShellConnector,
    forward_starter: &mut dyn NativeLocalForwardStarter,
    input: impl Into<NativeSshInput>,
    output: &mut dyn Write,
) -> Result<PtyExitStatus, Box<dyn Error>> {
    let mut prompts = NativeSecretPrompts {
        password_prompt: &mut |_| Err("password prompt should not be used".into()),
        key_passphrase_prompt: &mut |_| {
            Err("native SSH private-key passphrase prompt requires a passphrase provider".into())
        },
        key_needs_passphrase: &mut native_key_needs_passphrase,
    };
    run_native_with_connector_forward_starter_resolver_secret_prompts_and_io(
        options,
        connector,
        forward_starter,
        &mut |_| Err("OpenSSH config target resolver is not available in this path".into()),
        &mut prompts,
        input,
        output,
    )
}

#[cfg(test)]
fn run_native_with_connector_forward_starter_resolver_and_io(
    options: &SshOptions,
    connector: &mut dyn SshShellConnector,
    forward_starter: &mut dyn NativeLocalForwardStarter,
    openssh_config_resolver: &mut OpenSshConfigResolver<'_>,
    input: impl Into<NativeSshInput>,
    output: &mut dyn Write,
) -> Result<PtyExitStatus, Box<dyn Error>> {
    let mut prompts = NativeSecretPrompts {
        password_prompt: &mut |_| Err("password prompt should not be used".into()),
        key_passphrase_prompt: &mut |_| {
            Err("native SSH private-key passphrase prompt requires a passphrase provider".into())
        },
        key_needs_passphrase: &mut native_key_needs_passphrase,
    };
    run_native_with_connector_forward_starter_resolver_secret_prompts_and_io(
        options,
        connector,
        forward_starter,
        openssh_config_resolver,
        &mut prompts,
        input,
        output,
    )
}

fn run_native_with_connector_prompt_and_io(
    options: &SshOptions,
    connector: &mut dyn SshShellConnector,
    forward_starter: &mut dyn NativeLocalForwardStarter,
    password_prompt: &mut SecretPrompt<'_>,
    input: impl Into<NativeSshInput>,
    output: &mut dyn Write,
) -> Result<PtyExitStatus, Box<dyn Error>> {
    let mut prompts = NativeSecretPrompts {
        password_prompt,
        key_passphrase_prompt: &mut native_key_passphrase_prompt,
        key_needs_passphrase: &mut native_key_needs_passphrase,
    };
    run_native_with_connector_forward_starter_resolver_secret_prompts_and_io(
        options,
        connector,
        forward_starter,
        &mut resolve_openssh_config_target,
        &mut prompts,
        input,
        output,
    )
}

#[cfg(test)]
fn run_native_with_connector_prompt_resolver_and_io(
    options: &SshOptions,
    connector: &mut dyn SshShellConnector,
    forward_starter: &mut dyn NativeLocalForwardStarter,
    openssh_config_resolver: &mut OpenSshConfigResolver<'_>,
    password_prompt: &mut SecretPrompt<'_>,
    input: impl Into<NativeSshInput>,
    output: &mut dyn Write,
) -> Result<PtyExitStatus, Box<dyn Error>> {
    let mut prompts = NativeSecretPrompts {
        password_prompt,
        key_passphrase_prompt: &mut native_key_passphrase_prompt,
        key_needs_passphrase: &mut native_key_needs_passphrase,
    };
    run_native_with_connector_forward_starter_resolver_secret_prompts_and_io(
        options,
        connector,
        forward_starter,
        openssh_config_resolver,
        &mut prompts,
        input,
        output,
    )
}

#[cfg(test)]
fn run_native_with_connector_secret_prompts_and_io(
    options: &SshOptions,
    connector: &mut dyn SshShellConnector,
    password_prompt: &mut SecretPrompt<'_>,
    key_passphrase_prompt: &mut KeyPassphrasePrompt<'_>,
    key_needs_passphrase: &mut KeyPassphraseDetector<'_>,
    input: impl Into<NativeSshInput>,
    output: &mut dyn Write,
) -> Result<PtyExitStatus, Box<dyn Error>> {
    let mut forward_starter = RejectingNativeLocalForwardStarter;
    let mut prompts = NativeSecretPrompts {
        password_prompt,
        key_passphrase_prompt,
        key_needs_passphrase,
    };
    run_native_with_connector_forward_starter_resolver_secret_prompts_and_io(
        options,
        connector,
        &mut forward_starter,
        &mut |_| Err("OpenSSH config target resolver is not available in this path".into()),
        &mut prompts,
        input,
        output,
    )
}

fn run_native_with_connector_forward_starter_resolver_secret_prompts_and_io(
    options: &SshOptions,
    connector: &mut dyn SshShellConnector,
    forward_starter: &mut dyn NativeLocalForwardStarter,
    openssh_config_resolver: &mut OpenSshConfigResolver<'_>,
    prompts: &mut NativeSecretPrompts<'_>,
    input: impl Into<NativeSshInput>,
    output: &mut dyn Write,
) -> Result<PtyExitStatus, Box<dyn Error>> {
    let input = input.into();
    let metrics_started_at = Instant::now();
    let mut lifecycle = SessionLifecycle::new(NATIVE_SSH_SESSION_ID);
    lifecycle.start_connecting()?;
    let forward_plan = native_forward_plan_for_options(options)?;
    let request = native_request_for_options_with_resolver_secret_prompts(
        options,
        openssh_config_resolver,
        &mut *prompts.password_prompt,
        &mut *prompts.key_passphrase_prompt,
        &mut *prompts.key_needs_passphrase,
    )?;

    let mut forward_handles = Vec::new();
    for forward in forward_plan.local {
        let started = forward_starter.start(request.clone(), forward);
        retain_started_native_forward(&mut forward_handles, started)?;
    }
    for forward in forward_plan.dynamic {
        let started = forward_starter.start_dynamic(request.clone(), forward);
        retain_started_native_forward(&mut forward_handles, started)?;
    }
    for forward in forward_plan.remote {
        let started = forward_starter.start_remote(request.clone(), forward);
        retain_started_native_forward(&mut forward_handles, started)?;
    }

    if options.no_shell && !forward_handles.is_empty() {
        lifecycle.mark_connected()?;
        wait_for_native_forward_handles(&mut forward_handles, NATIVE_FORWARD_SHUTDOWN_TIMEOUT)?;
        return finish_native_ssh_success(
            options,
            &request,
            NativeSshIoCounters::default(),
            &rssh_ssh::SshSessionResult::default(),
            &mut lifecycle,
            metrics_started_at.elapsed(),
            output,
        );
    }

    let shell_result = (|| -> Result<_, Box<dyn Error>> {
        let transport = connect_native_transport(connector, request.clone(), &mut lifecycle)?;
        let input = input.acquire_after_connect()?;
        let (reader, writer) = transport.into_shell_halves();
        let outcome = rssh_ssh::run_split_shell_with_events(reader, writer, input, output)?;
        let io_counters = NativeSshIoCounters {
            ssh_input_bytes: outcome.input_bytes,
            ssh_output_bytes: outcome.output_bytes,
        };
        Ok((outcome, io_counters))
    })();
    let forward_shutdown_result =
        shutdown_native_forward_handles(&mut forward_handles, NATIVE_FORWARD_SHUTDOWN_TIMEOUT);
    let (outcome, io_counters) = match shell_result {
        Ok(result) => {
            forward_shutdown_result?;
            result
        }
        Err(error) => {
            let _ = forward_shutdown_result;
            return Err(error);
        }
    };

    finish_native_ssh_success(
        options,
        &request,
        io_counters,
        &outcome.result,
        &mut lifecycle,
        metrics_started_at.elapsed(),
        output,
    )
}

fn finish_native_ssh_success(
    options: &SshOptions,
    request: &SshConnectRequest,
    io_counters: NativeSshIoCounters,
    session_result: &rssh_ssh::SshSessionResult,
    lifecycle: &mut SessionLifecycle,
    elapsed: Duration,
    output: &mut dyn Write,
) -> Result<PtyExitStatus, Box<dyn Error>> {
    lifecycle.mark_disconnected()?;
    lifecycle.close()?;

    let status = pty_status_for_ssh_result(session_result);
    write_native_ssh_metrics_if_requested(
        options,
        request,
        io_counters,
        lifecycle.state(),
        elapsed,
        &status,
        output,
    )?;

    Ok(status)
}

fn connect_native_transport(
    connector: &mut dyn SshShellConnector,
    request: SshConnectRequest,
    lifecycle: &mut SessionLifecycle,
) -> Result<SshTransport, Box<dyn Error>> {
    let transport = SshTransport::connect(connector, request)?;
    lifecycle.mark_connected()?;
    Ok(transport)
}

fn pty_status_for_ssh_result(result: &rssh_ssh::SshSessionResult) -> PtyExitStatus {
    // Intentional application-boundary projection: SshSessionResult retains
    // the complete four-field signal metadata, while PtyExitStatus and the
    // existing native metrics schema expose only a numeric code plus signal
    // name. A numeric status wins; signal-only maps to code 1 plus its name;
    // a server that reports neither remains backward-compatible exit code 0.
    if let Some(exit_status) = result.exit_status {
        PtyExitStatus::from_exit_code(exit_status)
    } else if let Some(exit_signal) = &result.exit_signal {
        PtyExitStatus::from_signal(exit_signal.name.clone())
    } else {
        PtyExitStatus::from_exit_code(0)
    }
}

fn write_native_ssh_metrics_if_requested(
    options: &SshOptions,
    request: &SshConnectRequest,
    io_counters: NativeSshIoCounters,
    session_state: SessionState,
    elapsed: Duration,
    status: &PtyExitStatus,
    output: &mut dyn Write,
) -> Result<(), Box<dyn Error>> {
    if !options.console.metrics && !options.console.metrics_json {
        return Ok(());
    }

    let snapshot =
        NativeSshMetricsSnapshot::from_status(request, io_counters, elapsed, session_state, status);
    if options.console.metrics_json {
        writeln!(output, "{}", snapshot.json_report()?)?;
    } else {
        write!(output, "{}", snapshot.report())?;
    }
    output.flush()?;

    Ok(())
}

#[derive(Clone, Copy, Default)]
struct NativeSshIoCounters {
    ssh_input_bytes: u64,
    ssh_output_bytes: u64,
}

#[derive(Serialize)]
struct NativeSshMetricsSnapshot {
    backend: String,
    host: String,
    username: String,
    port: u16,
    columns: u16,
    rows: u16,
    session_state: String,
    ssh_input_bytes: u64,
    ssh_output_bytes: u64,
    elapsed_ms: u128,
    exit_code: u32,
    signal: Option<String>,
    success: bool,
}

impl NativeSshMetricsSnapshot {
    fn from_status(
        request: &SshConnectRequest,
        io_counters: NativeSshIoCounters,
        elapsed: Duration,
        session_state: SessionState,
        status: &PtyExitStatus,
    ) -> Self {
        Self {
            backend: "NativeRussh".to_owned(),
            host: request.config.host.clone(),
            username: request.config.username.clone(),
            port: request.config.port,
            columns: request.config.initial_size.columns,
            rows: request.config.initial_size.rows,
            session_state: session_state.as_str().to_owned(),
            ssh_input_bytes: io_counters.ssh_input_bytes,
            ssh_output_bytes: io_counters.ssh_output_bytes,
            elapsed_ms: elapsed.as_millis(),
            exit_code: status.exit_code(),
            signal: status.signal().map(str::to_owned),
            success: status.success(),
        }
    }

    fn report(&self) -> String {
        format!(
            "\
R-SSH native SSH metrics
backend={}
host={}
username={}
port={}
columns={}
rows={}
session_state={}
ssh_input_bytes={}
ssh_output_bytes={}
elapsed_ms={}
exit_code={}
signal={}
success={}
",
            self.backend,
            self.host,
            self.username,
            self.port,
            self.columns,
            self.rows,
            self.session_state,
            self.ssh_input_bytes,
            self.ssh_output_bytes,
            self.elapsed_ms,
            self.exit_code,
            self.signal.as_deref().unwrap_or("none"),
            self.success
        )
    }

    fn json_report(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
struct RejectingNativeLocalForwardStarter;

#[cfg(test)]
impl NativeLocalForwardStarter for RejectingNativeLocalForwardStarter {
    fn start(
        &mut self,
        _request: SshConnectRequest,
        _forward: NativeLocalForward,
    ) -> Result<Box<dyn NativeLocalForwardHandle>, Box<dyn Error>> {
        Err("native SSH local forwarding starter is not available in this path".into())
    }

    fn start_dynamic(
        &mut self,
        _request: SshConnectRequest,
        _forward: NativeDynamicForward,
    ) -> Result<Box<dyn NativeLocalForwardHandle>, Box<dyn Error>> {
        Err("native SSH dynamic forwarding starter is not available in this path".into())
    }

    fn start_remote(
        &mut self,
        _request: SshConnectRequest,
        _forward: NativeRemoteForward,
    ) -> Result<Box<dyn NativeLocalForwardHandle>, Box<dyn Error>> {
        Err("native SSH remote forwarding starter is not available in this path".into())
    }
}

fn native_password_prompt(request: &SshConnectRequest) -> Result<String, Box<dyn Error>> {
    rpassword::prompt_password(format!(
        "Password for {}@{}: ",
        request.config.username, request.config.host
    ))
    .map_err(Into::into)
}

fn native_key_passphrase_prompt(path: &Path) -> Result<String, Box<dyn Error>> {
    rpassword::prompt_password(format!("Passphrase for key {}: ", path.display()))
        .map_err(Into::into)
}

fn native_key_needs_passphrase(path: &Path) -> Result<bool, Box<dyn Error>> {
    RusshPrivateKeyAuth::needs_passphrase(path).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use std::{
        io,
        net::{TcpListener, TcpStream},
        path::{Path, PathBuf},
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };

    use rssh_core::{
        TerminalSize,
        session::{SessionLifecycle, SessionState},
    };
    use rssh_ssh::{
        SshConnectRequest, SshExitSignal, SshInputEvent, SshInputEventReceiver, SshSessionConfig,
        SshSessionError, SshSessionResult, SshSessionStartup, SshShellConnector, SshShellReader,
        SshShellSession, SshShellWriter, ssh_input_event_channel,
    };

    use crate::cli::{NativeHostKeyPolicy, OpenSshTarget, Osc52Policy, SshOptions, SshTarget};

    fn input_events(bytes: &[u8]) -> SshInputEventReceiver {
        let (sender, receiver) = ssh_input_event_channel(2);
        if !bytes.is_empty() {
            sender.send(SshInputEvent::Data(bytes.to_vec())).unwrap();
        }
        sender.send(SshInputEvent::Eof).unwrap();
        receiver
    }

    fn empty_input_events() -> SshInputEventReceiver {
        input_events(&[])
    }

    fn unused_loopback_port() -> u16 {
        TcpListener::bind(("127.0.0.1", 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    fn listener_is_released_within(port: u16, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            match TcpListener::bind(("127.0.0.1", port)) {
                Ok(listener) => {
                    drop(listener);
                    return true;
                }
                Err(_) if Instant::now() < deadline => std::thread::yield_now(),
                Err(_) => return false,
            }
        }
    }

    #[test]
    fn dropping_native_local_forward_releases_listener_within_deadline() {
        let port = unused_loopback_port();
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );
        let mut starter =
            super::ThreadedNativeLocalForwardStarter::new(rssh_ssh::RusshChannelOpener::default());
        let handle = super::NativeLocalForwardStarter::start(
            &mut starter,
            request,
            super::NativeLocalForward {
                bind_host: "127.0.0.1".to_owned(),
                bind_port: port,
                target_host: "127.0.0.1".to_owned(),
                target_port: 9,
            },
        )
        .unwrap();

        drop(handle);

        assert!(listener_is_released_within(
            port,
            Duration::from_millis(250)
        ));
    }

    #[test]
    fn dropping_native_dynamic_forward_releases_listener_within_deadline() {
        let port = unused_loopback_port();
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );
        let mut starter =
            super::ThreadedNativeLocalForwardStarter::new(rssh_ssh::RusshChannelOpener::default());
        let handle = super::NativeLocalForwardStarter::start_dynamic(
            &mut starter,
            request,
            super::NativeDynamicForward {
                bind_host: "127.0.0.1".to_owned(),
                bind_port: port,
            },
        )
        .unwrap();

        drop(handle);

        assert!(listener_is_released_within(
            port,
            Duration::from_millis(250)
        ));
    }

    #[test]
    fn explicit_native_forward_shutdown_is_bounded_and_idempotent() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );
        let mut starter =
            super::ThreadedNativeLocalForwardStarter::new(rssh_ssh::RusshChannelOpener::default());

        let local_port = unused_loopback_port();
        let mut local = super::NativeLocalForwardStarter::start(
            &mut starter,
            request.clone(),
            super::NativeLocalForward {
                bind_host: "127.0.0.1".to_owned(),
                bind_port: local_port,
                target_host: "127.0.0.1".to_owned(),
                target_port: 9,
            },
        )
        .unwrap();
        local.shutdown(Duration::from_millis(250)).unwrap();
        local.shutdown(Duration::from_millis(250)).unwrap();
        assert!(listener_is_released_within(
            local_port,
            Duration::from_millis(250)
        ));

        let dynamic_port = unused_loopback_port();
        let mut dynamic = super::NativeLocalForwardStarter::start_dynamic(
            &mut starter,
            request,
            super::NativeDynamicForward {
                bind_host: "127.0.0.1".to_owned(),
                bind_port: dynamic_port,
            },
        )
        .unwrap();
        dynamic.shutdown(Duration::from_millis(250)).unwrap();
        dynamic.shutdown(Duration::from_millis(250)).unwrap();
        assert!(listener_is_released_within(
            dynamic_port,
            Duration::from_millis(250)
        ));
    }

    #[test]
    fn native_forward_wait_reports_worker_panic() {
        let mut handle = super::ThreadedNativeLocalForwardHandle::spawn(|_| {
            panic!("forward worker panic");
        });

        let error = super::NativeLocalForwardHandle::wait(&mut handle).unwrap_err();

        assert_eq!(error.to_string(), "native SSH forwarding worker panicked");
    }

    #[test]
    fn dropping_native_forward_best_effort_joins_cooperative_worker() {
        let exited = Arc::new(AtomicBool::new(false));
        let worker_exited = Arc::clone(&exited);
        let handle = super::ThreadedNativeLocalForwardHandle::spawn(move |cancellation| {
            while !cancellation.is_cancelled() {
                std::thread::yield_now();
            }
            std::thread::sleep(Duration::from_millis(25));
            worker_exited.store(true, Ordering::Release);
            Ok(())
        });

        drop(handle);

        assert!(exited.load(Ordering::Acquire));
    }

    #[test]
    fn native_listener_joins_active_connection_workers_on_cancellation() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let connection_started = Arc::new(AtomicBool::new(false));
        let connection_exited = Arc::new(AtomicBool::new(false));
        let started = Arc::clone(&connection_started);
        let exited = Arc::clone(&connection_exited);
        let handle = super::ThreadedNativeLocalForwardHandle::spawn(move |cancellation| {
            super::run_native_forward_listener(&listener, &cancellation, move |_, cancellation| {
                started.store(true, Ordering::Release);
                while !cancellation.is_cancelled() {
                    std::thread::yield_now();
                }
                std::thread::sleep(Duration::from_millis(25));
                exited.store(true, Ordering::Release);
                Ok(())
            })
            .map_err(|error| error.to_string())
        });
        let _client = TcpStream::connect(address).unwrap();
        let started_deadline = Instant::now() + Duration::from_millis(250);
        while !connection_started.load(Ordering::Acquire) && Instant::now() < started_deadline {
            std::thread::yield_now();
        }
        assert!(connection_started.load(Ordering::Acquire));

        drop(handle);

        assert!(connection_exited.load(Ordering::Acquire));
    }

    #[test]
    fn dynamic_forward_cancels_partial_socks_handshake() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let client = TcpStream::connect(address).unwrap();
        let (server, _) = listener.accept().unwrap();
        let cancellation = rssh_ssh::RusshForwardCancellation::new();
        let worker_cancellation = cancellation.clone();
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );
        let (completion_sender, completion) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let opener = rssh_ssh::RusshChannelOpener::default();
            let result = super::run_native_dynamic_forward_connection(
                server,
                &opener,
                request,
                &worker_cancellation,
            )
            .map_err(|error| error.to_string());
            let _ = completion_sender.send(result);
        });
        cancellation.cancel();

        let result = completion
            .recv_timeout(Duration::from_millis(250))
            .expect("partial SOCKS handshake worker did not stop after cancellation");

        assert_eq!(
            result.unwrap_err().to_string(),
            "native SSH forwarding cancelled"
        );
        drop(client);
    }

    #[test]
    fn socks_writer_cancels_nonblocking_would_block_loop() {
        struct NeverWritable;

        impl std::io::Write for NeverWritable {
            fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
                Err(io::Error::from(io::ErrorKind::WouldBlock))
            }

            fn flush(&mut self) -> io::Result<()> {
                Err(io::Error::from(io::ErrorKind::WouldBlock))
            }
        }

        let cancellation = rssh_ssh::RusshForwardCancellation::new();
        let delayed_cancellation = cancellation.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            delayed_cancellation.cancel();
        });
        let mut output = NeverWritable;
        let mut writer = super::CancellableForwardWriter {
            output: &mut output,
            cancellation: &cancellation,
            deadline: Instant::now() + Duration::from_secs(1),
        };

        let error = std::io::Write::write_all(&mut writer, b"reply").unwrap_err();

        assert_eq!(error.to_string(), "native SSH forwarding cancelled");
    }

    #[test]
    fn socks_writer_resets_expired_deadline_for_success_and_failure_replies() {
        let cancellation = rssh_ssh::RusshForwardCancellation::new();
        let mut output = Vec::new();
        let mut writer = super::CancellableForwardWriter {
            output: &mut output,
            cancellation: &cancellation,
            deadline: Instant::now()
                .checked_sub(Duration::from_millis(1))
                .unwrap(),
        };

        writer.reset_deadline(Duration::from_secs(1));
        super::write_socks5_connect_reply(&mut writer, 0x00).unwrap();
        writer.deadline = Instant::now()
            .checked_sub(Duration::from_millis(1))
            .unwrap();
        writer.reset_deadline(Duration::from_secs(1));
        super::write_socks5_connect_reply(&mut writer, 0x01).unwrap();

        assert_eq!(output[1], 0x00);
        assert_eq!(output[11], 0x01);
    }

    #[test]
    fn no_shell_waits_for_any_forward_then_cancels_and_joins_all() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut handles: Vec<Box<dyn super::NativeLocalForwardHandle>> = vec![
            Box::new(CompletionMockForwardHandle::new(
                "first",
                false,
                None,
                Arc::clone(&events),
            )),
            Box::new(CompletionMockForwardHandle::new(
                "second",
                true,
                Some("second forward failed"),
                Arc::clone(&events),
            )),
            Box::new(CompletionMockForwardHandle::new(
                "third",
                false,
                None,
                Arc::clone(&events),
            )),
        ];

        let error =
            super::wait_for_native_forward_handles(&mut handles, Duration::from_millis(250))
                .unwrap_err();

        assert_eq!(error.to_string(), "second forward failed");
        assert_eq!(
            *events.lock().unwrap(),
            [
                "wait:second",
                "cancel:first",
                "cancel:second",
                "cancel:third",
                "shutdown:first",
                "shutdown:second",
                "shutdown:third",
            ]
        );
    }

    #[test]
    fn native_forward_partial_startup_failure_rolls_back_started_handles() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );
        let shell_state = Arc::new(Mutex::new(MockState::default()));
        let mut connector = MockConnector {
            state: Arc::clone(&shell_state),
        };
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut starter = FailingSecondForwardStarter {
            events: Arc::clone(&events),
        };

        let error = super::run_native_with_connector_forward_starter_and_io(
            &SshOptions {
                target: SshTarget::Direct(request),
                remote_command: Vec::new(),
                forwards: vec![
                    crate::cli::SshForward::Local("127.0.0.1:15432:db.internal:5432".to_owned()),
                    crate::cli::SshForward::Dynamic("127.0.0.1:1080".to_owned()),
                ],
                openssh_args: Vec::new(),
                no_shell: false,
                native: true,
                gui: false,
                renderer: crate::cli::RendererMode::Auto,
                benchmark_startup: false,
                native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
                console: crate::cli::ConsoleOptions::default(),
                osc52_policy: Osc52Policy::default(),
                log: None,
            },
            &mut connector,
            &mut starter,
            empty_input_events(),
            &mut Vec::new(),
        )
        .unwrap_err();

        assert_eq!(error.to_string(), "second forward startup failed");
        assert!(shell_state.lock().unwrap().last_request.is_none());
        assert_eq!(*events.lock().unwrap(), ["cancel:first", "shutdown:first"]);
    }

    #[test]
    fn ssh_runner_streams_remote_output_and_closes_session() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );
        let state = Arc::new(Mutex::new(MockState::default()));
        let mut connector = MockConnector {
            state: Arc::clone(&state),
        };
        let mut output = Vec::new();

        super::run_with_connector_and_io(
            &SshOptions {
                target: SshTarget::Direct(request.clone()),
                remote_command: Vec::new(),
                forwards: Vec::new(),
                openssh_args: Vec::new(),
                no_shell: false,
                native: false,
                gui: false,
                renderer: crate::cli::RendererMode::Auto,
                benchmark_startup: false,
                native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
                console: crate::cli::ConsoleOptions::default(),
                osc52_policy: Osc52Policy::default(),
                log: None,
            },
            &mut connector,
            empty_input_events(),
            &mut output,
        )
        .unwrap();

        let state = state.lock().unwrap();
        assert_eq!(state.last_request.as_ref(), Some(&request));
        assert_eq!(output, b"remote\n");
        assert!(state.closed);
    }

    #[test]
    fn ssh_runner_writes_local_input_to_remote_session() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );
        let state = Arc::new(Mutex::new(MockState::default()));
        let mut connector = MockConnector {
            state: Arc::clone(&state),
        };
        let input = input_events(b"echo hi\n");
        let mut output = Vec::new();

        super::run_with_connector_and_io(
            &SshOptions {
                target: SshTarget::Direct(request),
                remote_command: Vec::new(),
                forwards: Vec::new(),
                openssh_args: Vec::new(),
                no_shell: false,
                native: false,
                gui: false,
                renderer: crate::cli::RendererMode::Auto,
                benchmark_startup: false,
                native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
                console: crate::cli::ConsoleOptions::default(),
                osc52_policy: Osc52Policy::default(),
                log: None,
            },
            &mut connector,
            input,
            &mut output,
        )
        .unwrap();

        let state = state.lock().unwrap();
        assert_eq!(state.written, b"echo hi\n");
        assert_eq!(output, b"remote\n");
        assert!(state.closed);
    }

    #[test]
    fn ssh_runner_passes_remote_command_startup_to_native_connector() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );
        let state = Arc::new(Mutex::new(MockState::default()));
        let mut connector = MockConnector {
            state: Arc::clone(&state),
        };
        let mut output = Vec::new();

        super::run_with_connector_and_io(
            &SshOptions {
                target: SshTarget::Direct(request.clone()),
                remote_command: vec!["uname".to_owned(), "-a".to_owned()],
                forwards: Vec::new(),
                openssh_args: Vec::new(),
                no_shell: false,
                native: false,
                gui: false,
                renderer: crate::cli::RendererMode::Auto,
                benchmark_startup: false,
                native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
                console: crate::cli::ConsoleOptions::default(),
                osc52_policy: Osc52Policy::default(),
                log: None,
            },
            &mut connector,
            empty_input_events(),
            &mut output,
        )
        .unwrap();

        let state = state.lock().unwrap();
        let request = state.last_request.as_ref().unwrap();
        assert_eq!(
            request.startup,
            SshSessionStartup::Command(vec!["uname".to_owned(), "-a".to_owned()])
        );
    }

    #[test]
    fn ssh_runner_passes_no_shell_startup_to_native_connector() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );
        let state = Arc::new(Mutex::new(MockState::default()));
        let mut connector = MockConnector {
            state: Arc::clone(&state),
        };
        let mut output = Vec::new();

        super::run_with_connector_and_io(
            &SshOptions {
                target: SshTarget::Direct(request.clone()),
                remote_command: Vec::new(),
                forwards: Vec::new(),
                openssh_args: Vec::new(),
                no_shell: true,
                native: false,
                gui: false,
                renderer: crate::cli::RendererMode::Auto,
                benchmark_startup: false,
                native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
                console: crate::cli::ConsoleOptions::default(),
                osc52_policy: Osc52Policy::default(),
                log: None,
            },
            &mut connector,
            empty_input_events(),
            &mut output,
        )
        .unwrap();

        let state = state.lock().unwrap();
        let request = state.last_request.as_ref().unwrap();
        assert_eq!(request.startup, SshSessionStartup::NoShell);
    }

    #[test]
    fn native_ssh_runner_uses_connector_and_returns_success_status() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );
        let state = Arc::new(Mutex::new(MockState::default()));
        let mut connector = MockConnector {
            state: Arc::clone(&state),
        };
        let mut output = Vec::new();

        let status = super::run_native_with_connector_and_io(
            &SshOptions {
                target: SshTarget::Direct(request.clone()),
                remote_command: Vec::new(),
                forwards: Vec::new(),
                openssh_args: Vec::new(),
                no_shell: false,
                native: true,
                gui: false,
                renderer: crate::cli::RendererMode::Auto,
                benchmark_startup: false,
                native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
                console: crate::cli::ConsoleOptions::default(),
                osc52_policy: Osc52Policy::default(),
                log: None,
            },
            &mut connector,
            empty_input_events(),
            &mut output,
        )
        .unwrap();

        let state = state.lock().unwrap();
        assert!(status.success());
        assert_eq!(state.last_request.as_ref(), Some(&request));
        assert_eq!(output, b"remote\n");
        assert!(state.closed);
    }

    #[test]
    fn native_session_marks_connected_immediately_after_connect() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );
        let state = Arc::new(Mutex::new(MockState::default()));
        let mut connector = MockConnector {
            state: Arc::clone(&state),
        };
        let mut lifecycle = SessionLifecycle::new(rssh_core::SessionId::new(77));
        lifecycle.start_connecting().unwrap();

        let session =
            super::connect_native_transport(&mut connector, request, &mut lifecycle).unwrap();

        assert_eq!(lifecycle.state(), SessionState::Connected);
        drop(session);
    }

    #[test]
    fn native_ssh_runner_returns_remote_exit_status() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );
        let state = Arc::new(Mutex::new(MockState {
            remote_exit_status: Some(23),
            remote_exit_signal: Some(SshExitSignal {
                name: "TERM".to_owned(),
                core_dumped: false,
                error_message: String::new(),
                lang_tag: String::new(),
            }),
            ..MockState::default()
        }));
        let mut connector = MockConnector {
            state: Arc::clone(&state),
        };
        let mut output = Vec::new();

        let status = super::run_native_with_connector_and_io(
            &SshOptions {
                target: SshTarget::Direct(request),
                remote_command: vec!["false".to_owned()],
                forwards: Vec::new(),
                openssh_args: Vec::new(),
                no_shell: false,
                native: true,
                gui: false,
                renderer: crate::cli::RendererMode::Auto,
                benchmark_startup: false,
                native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
                console: crate::cli::ConsoleOptions::default(),
                osc52_policy: Osc52Policy::default(),
                log: None,
            },
            &mut connector,
            empty_input_events(),
            &mut output,
        )
        .unwrap();

        assert_eq!(status.exit_code(), 23);
        assert_eq!(status.signal(), None);
    }

    #[test]
    fn native_ssh_runner_preserves_remote_exit_signal_and_metrics() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );
        let state = Arc::new(Mutex::new(MockState {
            remote_exit_signal: Some(SshExitSignal {
                name: "TERM".to_owned(),
                core_dumped: true,
                error_message: "terminated by policy".to_owned(),
                lang_tag: "en-US".to_owned(),
            }),
            ..MockState::default()
        }));
        let mut connector = MockConnector {
            state: Arc::clone(&state),
        };
        let mut output = Vec::new();

        let status = super::run_native_with_connector_and_io(
            &SshOptions {
                target: SshTarget::Direct(request),
                remote_command: vec!["long-running-command".to_owned()],
                forwards: Vec::new(),
                openssh_args: Vec::new(),
                no_shell: false,
                native: true,
                gui: false,
                renderer: crate::cli::RendererMode::Auto,
                benchmark_startup: false,
                native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
                console: crate::cli::ConsoleOptions {
                    metrics_json: true,
                    ..crate::cli::ConsoleOptions::default()
                },
                osc52_policy: Osc52Policy::default(),
                log: None,
            },
            &mut connector,
            empty_input_events(),
            &mut output,
        )
        .unwrap();

        assert_eq!(status.exit_code(), 1);
        assert_eq!(status.signal(), Some("TERM"));
        let metrics = String::from_utf8(output)
            .unwrap()
            .lines()
            .last()
            .map(serde_json::from_str::<serde_json::Value>)
            .transpose()
            .unwrap()
            .expect("metrics JSON line");
        assert_eq!(metrics["exit_code"], 1);
        assert_eq!(metrics["signal"], "TERM");
    }

    #[test]
    fn ssh_session_result_mapping_policy_preserves_source_metadata() {
        let status_and_signal = SshSessionResult {
            exit_status: Some(23),
            exit_signal: Some(SshExitSignal {
                name: "TERM".to_owned(),
                core_dumped: true,
                error_message: "terminated by policy".to_owned(),
                lang_tag: "en-US".to_owned(),
            }),
        };

        let status = super::pty_status_for_ssh_result(&status_and_signal);

        assert_eq!(status.exit_code(), 23);
        assert_eq!(status.signal(), None);
        assert_eq!(
            status_and_signal.exit_signal,
            Some(SshExitSignal {
                name: "TERM".to_owned(),
                core_dumped: true,
                error_message: "terminated by policy".to_owned(),
                lang_tag: "en-US".to_owned(),
            })
        );

        let signal_only = SshSessionResult {
            exit_status: None,
            exit_signal: status_and_signal.exit_signal.clone(),
        };
        let signal_status = super::pty_status_for_ssh_result(&signal_only);
        assert_eq!(signal_status.exit_code(), 1);
        assert_eq!(signal_status.signal(), Some("TERM"));

        let no_remote_status = super::pty_status_for_ssh_result(&SshSessionResult::default());
        assert_eq!(no_remote_status.exit_code(), 0);
        assert_eq!(no_remote_status.signal(), None);
    }

    #[test]
    fn native_ssh_runner_prints_json_metrics_when_requested() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(100, 30).into())
                .unwrap(),
        );
        let state = Arc::new(Mutex::new(MockState::default()));
        let mut connector = MockConnector {
            state: Arc::clone(&state),
        };
        let input = input_events(b"whoami\n");
        let mut output = Vec::new();

        let status = super::run_native_with_connector_and_io(
            &SshOptions {
                target: SshTarget::Direct(request),
                remote_command: Vec::new(),
                forwards: Vec::new(),
                openssh_args: Vec::new(),
                no_shell: false,
                native: true,
                gui: false,
                renderer: crate::cli::RendererMode::Auto,
                benchmark_startup: false,
                native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
                console: crate::cli::ConsoleOptions {
                    metrics_json: true,
                    ..crate::cli::ConsoleOptions::default()
                },
                osc52_policy: Osc52Policy::default(),
                log: None,
            },
            &mut connector,
            input,
            &mut output,
        )
        .unwrap();

        assert!(status.success());
        assert_eq!(state.lock().unwrap().written, b"whoami\n");
        let output = String::from_utf8(output).unwrap();
        let mut lines = output.lines();
        assert_eq!(lines.next(), Some("remote"));
        let metrics: serde_json::Value = serde_json::from_str(lines.next().unwrap()).unwrap();

        assert_eq!(metrics["backend"], "NativeRussh");
        assert_eq!(metrics["host"], "example.com");
        assert_eq!(metrics["username"], "ops");
        assert_eq!(metrics["port"], 22);
        assert_eq!(metrics["columns"], 100);
        assert_eq!(metrics["rows"], 30);
        assert_eq!(metrics["session_state"], "closed");
        assert_eq!(metrics["ssh_input_bytes"], 7);
        assert_eq!(metrics["ssh_output_bytes"], 7);
        assert_eq!(metrics["exit_code"], 0);
        assert_eq!(metrics["signal"], serde_json::Value::Null);
        assert_eq!(metrics["success"], true);
    }

    #[test]
    fn native_ssh_runner_starts_local_forward_before_shell() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );
        let state = Arc::new(Mutex::new(MockState::default()));
        let mut connector = MockConnector {
            state: Arc::clone(&state),
        };
        let forward_state = Arc::new(Mutex::new(MockForwardState::default()));
        let mut forward_starter = MockForwardStarter {
            state: Arc::clone(&forward_state),
        };
        let mut output = Vec::new();

        super::run_native_with_connector_forward_starter_and_io(
            &SshOptions {
                target: SshTarget::Direct(request.clone()),
                remote_command: Vec::new(),
                forwards: vec![crate::cli::SshForward::Local(
                    "127.0.0.1:15432:db.internal:5432".to_owned(),
                )],
                openssh_args: Vec::new(),
                no_shell: false,
                native: true,
                gui: false,
                renderer: crate::cli::RendererMode::Auto,
                benchmark_startup: false,
                native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
                console: crate::cli::ConsoleOptions::default(),
                osc52_policy: Osc52Policy::default(),
                log: None,
            },
            &mut connector,
            &mut forward_starter,
            empty_input_events(),
            &mut output,
        )
        .unwrap();

        assert_eq!(state.lock().unwrap().last_request.as_ref(), Some(&request));
        assert_eq!(
            forward_state.lock().unwrap().local,
            [super::NativeLocalForward {
                bind_host: "127.0.0.1".to_owned(),
                bind_port: 15432,
                target_host: "db.internal".to_owned(),
                target_port: 5432,
            }]
        );
    }

    #[test]
    fn native_ssh_runner_starts_dynamic_forward_before_shell() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );
        let state = Arc::new(Mutex::new(MockState::default()));
        let mut connector = MockConnector {
            state: Arc::clone(&state),
        };
        let forward_state = Arc::new(Mutex::new(MockForwardState::default()));
        let mut forward_starter = MockForwardStarter {
            state: Arc::clone(&forward_state),
        };
        let mut output = Vec::new();

        super::run_native_with_connector_forward_starter_and_io(
            &SshOptions {
                target: SshTarget::Direct(request.clone()),
                remote_command: Vec::new(),
                forwards: vec![crate::cli::SshForward::Dynamic("127.0.0.1:1080".to_owned())],
                openssh_args: Vec::new(),
                no_shell: false,
                native: true,
                gui: false,
                renderer: crate::cli::RendererMode::Auto,
                benchmark_startup: false,
                native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
                console: crate::cli::ConsoleOptions::default(),
                osc52_policy: Osc52Policy::default(),
                log: None,
            },
            &mut connector,
            &mut forward_starter,
            empty_input_events(),
            &mut output,
        )
        .unwrap();

        assert_eq!(state.lock().unwrap().last_request.as_ref(), Some(&request));
        assert_eq!(
            forward_state.lock().unwrap().dynamic,
            [super::NativeDynamicForward {
                bind_host: "127.0.0.1".to_owned(),
                bind_port: 1080,
            }]
        );
    }

    #[test]
    fn native_ssh_runner_shuts_down_all_forward_modes_after_shell() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );
        let state = Arc::new(Mutex::new(MockState::default()));
        let mut connector = MockConnector { state };
        let forward_state = Arc::new(Mutex::new(MockForwardState::default()));
        let mut forward_starter = MockForwardStarter {
            state: Arc::clone(&forward_state),
        };
        let mut output = Vec::new();

        super::run_native_with_connector_forward_starter_and_io(
            &SshOptions {
                target: SshTarget::Direct(request),
                remote_command: Vec::new(),
                forwards: vec![
                    crate::cli::SshForward::Local("127.0.0.1:15432:db.internal:5432".to_owned()),
                    crate::cli::SshForward::Dynamic("127.0.0.1:1080".to_owned()),
                    crate::cli::SshForward::Remote("127.0.0.1:18080:127.0.0.1:8080".to_owned()),
                ],
                openssh_args: Vec::new(),
                no_shell: false,
                native: true,
                gui: false,
                renderer: crate::cli::RendererMode::Auto,
                benchmark_startup: false,
                native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
                console: crate::cli::ConsoleOptions::default(),
                osc52_policy: Osc52Policy::default(),
                log: None,
            },
            &mut connector,
            &mut forward_starter,
            empty_input_events(),
            &mut output,
        )
        .unwrap();

        assert_eq!(forward_state.lock().unwrap().shutdowns, 3);
    }

    #[test]
    fn native_ssh_runner_keeps_no_shell_remote_forward_open_without_shell() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );
        let state = Arc::new(Mutex::new(MockState::default()));
        let mut connector = MockConnector {
            state: Arc::clone(&state),
        };
        let forward_state = Arc::new(Mutex::new(MockForwardState::default()));
        let mut forward_starter = MockForwardStarter {
            state: Arc::clone(&forward_state),
        };
        let mut output = Vec::new();

        let status = super::run_native_with_connector_forward_starter_and_io(
            &SshOptions {
                target: SshTarget::Direct(request),
                remote_command: Vec::new(),
                forwards: vec![crate::cli::SshForward::Remote(
                    "8080:127.0.0.1:80".to_owned(),
                )],
                openssh_args: Vec::new(),
                no_shell: true,
                native: true,
                gui: false,
                renderer: crate::cli::RendererMode::Auto,
                benchmark_startup: false,
                native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
                console: crate::cli::ConsoleOptions::default(),
                osc52_policy: Osc52Policy::default(),
                log: None,
            },
            &mut connector,
            &mut forward_starter,
            empty_input_events(),
            &mut output,
        )
        .unwrap();

        assert!(status.success());
        assert!(state.lock().unwrap().last_request.is_none());
        assert_eq!(
            forward_state.lock().unwrap().remote,
            [super::NativeRemoteForward {
                bind_host: "127.0.0.1".to_owned(),
                bind_port: 8080,
                target_host: "127.0.0.1".to_owned(),
                target_port: 80,
            }]
        );
    }

    #[test]
    fn native_ssh_runner_resolves_openssh_target_before_connecting() {
        let state = Arc::new(Mutex::new(MockState::default()));
        let mut connector = MockConnector {
            state: Arc::clone(&state),
        };
        let forward_state = Arc::new(Mutex::new(MockForwardState::default()));
        let mut forward_starter = MockForwardStarter {
            state: Arc::clone(&forward_state),
        };
        let mut output = Vec::new();

        super::run_native_with_connector_forward_starter_resolver_and_io(
            &SshOptions {
                target: SshTarget::OpenSsh(OpenSshTarget {
                    target: "prod".to_owned(),
                    username: Some("override".to_owned()),
                    port: Some(2200),
                    initial_size: TerminalSize::new(100, 40),
                    auth: rssh_ssh::SshAuthMethod::Agent,
                }),
                remote_command: Vec::new(),
                forwards: Vec::new(),
                openssh_args: Vec::new(),
                no_shell: false,
                native: true,
                gui: false,
                renderer: crate::cli::RendererMode::Auto,
                benchmark_startup: false,
                native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
                console: crate::cli::ConsoleOptions::default(),
                osc52_policy: Osc52Policy::default(),
                log: None,
            },
            &mut connector,
            &mut forward_starter,
            &mut |target| {
                assert_eq!(target.target, "prod");
                Ok("hostname ssh.example.com\nuser deploy\nport 2222\n".to_owned())
            },
            empty_input_events(),
            &mut output,
        )
        .unwrap();

        let state = state.lock().unwrap();
        let request = state.last_request.as_ref().unwrap();
        assert_eq!(request.config.host, "ssh.example.com");
        assert_eq!(request.config.username, "override");
        assert_eq!(request.config.port, 2200);
        assert_eq!(
            TerminalSize::from(request.config.initial_size),
            TerminalSize::new(100, 40)
        );
    }

    #[test]
    fn native_local_forward_plan_parses_bind_and_target_endpoint() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );

        let plan = super::native_forward_plan_for_options(&SshOptions {
            target: SshTarget::Direct(request),
            remote_command: Vec::new(),
            forwards: vec![crate::cli::SshForward::Local(
                "127.0.0.1:15432:db.internal:5432".to_owned(),
            )],
            openssh_args: Vec::new(),
            no_shell: true,
            native: true,
            gui: false,
            renderer: crate::cli::RendererMode::Auto,
            benchmark_startup: false,
            native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
            console: crate::cli::ConsoleOptions::default(),
            osc52_policy: Osc52Policy::default(),
            log: None,
        })
        .unwrap();

        assert_eq!(
            plan.local,
            [super::NativeLocalForward {
                bind_host: "127.0.0.1".to_owned(),
                bind_port: 15432,
                target_host: "db.internal".to_owned(),
                target_port: 5432,
            }]
        );
    }

    #[test]
    fn native_local_forward_builds_direct_tcpip_plan_from_target_and_originator() {
        let forward = super::NativeLocalForward {
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 15432,
            target_host: "db.internal".to_owned(),
            target_port: 5432,
        };

        let plan = super::native_direct_tcpip_plan_for_local_forward(&forward, "127.0.0.1", 61234);

        assert_eq!(plan.target(), ("db.internal", 5432));
        assert_eq!(plan.originator(), ("127.0.0.1", 61234));
    }

    #[test]
    fn native_forward_plan_accepts_remote_forwards() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );

        let plan = super::native_forward_plan_for_options(&SshOptions {
            target: SshTarget::Direct(request),
            remote_command: Vec::new(),
            forwards: vec![crate::cli::SshForward::Remote(
                "8080:127.0.0.1:80".to_owned(),
            )],
            openssh_args: Vec::new(),
            no_shell: true,
            native: true,
            gui: false,
            renderer: crate::cli::RendererMode::Auto,
            benchmark_startup: false,
            native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
            console: crate::cli::ConsoleOptions::default(),
            osc52_policy: Osc52Policy::default(),
            log: None,
        })
        .unwrap();

        assert_eq!(
            plan.remote,
            [super::NativeRemoteForward {
                bind_host: "127.0.0.1".to_owned(),
                bind_port: 8080,
                target_host: "127.0.0.1".to_owned(),
                target_port: 80,
            }]
        );
    }

    #[test]
    fn native_forward_plan_parses_dynamic_bind_endpoint() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );

        let plan = super::native_forward_plan_for_options(&SshOptions {
            target: SshTarget::Direct(request),
            remote_command: Vec::new(),
            forwards: vec![crate::cli::SshForward::Dynamic("127.0.0.1:1080".to_owned())],
            openssh_args: Vec::new(),
            no_shell: true,
            native: true,
            gui: false,
            renderer: crate::cli::RendererMode::Auto,
            benchmark_startup: false,
            native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
            console: crate::cli::ConsoleOptions::default(),
            osc52_policy: Osc52Policy::default(),
            log: None,
        })
        .unwrap();

        assert_eq!(
            plan.dynamic,
            [super::NativeDynamicForward {
                bind_host: "127.0.0.1".to_owned(),
                bind_port: 1080,
            }]
        );
    }

    #[test]
    fn native_dynamic_forward_parser_rejects_non_loopback_listeners() {
        for spec in ["0.0.0.0:1080", "192.0.2.10:1080", "proxy:1080"] {
            let error = super::parse_native_dynamic_forward(spec).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("rejects non-loopback bind address"),
                "{error}"
            );
        }

        for spec in ["1080", "127.0.0.1:1080", "localhost:1080"] {
            super::parse_native_dynamic_forward(spec).unwrap();
        }
    }

    #[test]
    fn native_openssh_target_request_uses_resolved_host_user_and_port() {
        let target = OpenSshTarget {
            target: "prod".to_owned(),
            username: None,
            port: None,
            initial_size: TerminalSize::new(100, 40),
            auth: rssh_ssh::SshAuthMethod::Agent,
        };

        let request = super::native_request_for_openssh_target_with_config_output(
            &target,
            "hostname ssh.example.com\nuser deploy\nport 2222\n",
        )
        .unwrap();

        assert_eq!(request.config.host, "ssh.example.com");
        assert_eq!(request.config.username, "deploy");
        assert_eq!(request.config.port, 2222);
        assert_eq!(
            TerminalSize::from(request.config.initial_size),
            TerminalSize::new(100, 40)
        );
        assert_eq!(request.auth, rssh_ssh::SshAuthMethod::Agent);
    }

    #[test]
    fn socks5_connect_request_parses_domain_target_and_selects_no_auth() {
        let mut input = io::Cursor::new([
            0x05, 0x01, 0x00, 0x05, 0x01, 0x00, 0x03, 0x0b, b'e', b'x', b'a', b'm', b'p', b'l',
            b'e', b'.', b'c', b'o', b'm', 0x01, 0xbb,
        ]);
        let mut output = Vec::new();

        let request = super::read_socks5_connect_request(&mut input, &mut output).unwrap();

        assert_eq!(
            request,
            super::Socks5ConnectRequest {
                target_host: "example.com".to_owned(),
                target_port: 443,
            }
        );
        assert_eq!(output, [0x05, 0x00]);
    }

    #[test]
    fn native_ssh_runner_resolves_password_prompt_before_connecting() {
        let request = SshConnectRequest::new(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
            rssh_ssh::SshAuthMethod::PasswordPrompt,
        );
        let state = Arc::new(Mutex::new(MockState::default()));
        let mut connector = MockConnector {
            state: Arc::clone(&state),
        };
        let mut forward_starter = MockForwardStarter {
            state: Arc::new(Mutex::new(MockForwardState::default())),
        };
        let mut output = Vec::new();

        super::run_native_with_connector_prompt_and_io(
            &SshOptions {
                target: SshTarget::Direct(request),
                remote_command: Vec::new(),
                forwards: Vec::new(),
                openssh_args: Vec::new(),
                no_shell: false,
                native: true,
                gui: false,
                renderer: crate::cli::RendererMode::Auto,
                benchmark_startup: false,
                native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
                console: crate::cli::ConsoleOptions::default(),
                osc52_policy: Osc52Policy::default(),
                log: None,
            },
            &mut connector,
            &mut forward_starter,
            &mut |request| {
                assert_eq!(request.config.username, "ops");
                assert_eq!(request.config.host, "example.com");
                Ok("secret".to_owned())
            },
            empty_input_events(),
            &mut output,
        )
        .unwrap();

        let state = state.lock().unwrap();
        let request = state.last_request.as_ref().unwrap();
        assert_eq!(
            request.auth,
            rssh_ssh::SshAuthMethod::Password {
                password: "secret".to_owned()
            }
        );
    }

    #[test]
    fn native_ssh_runner_prompts_for_resolved_openssh_target_password() {
        let state = Arc::new(Mutex::new(MockState::default()));
        let mut connector = MockConnector {
            state: Arc::clone(&state),
        };
        let mut forward_starter = MockForwardStarter {
            state: Arc::new(Mutex::new(MockForwardState::default())),
        };
        let mut output = Vec::new();
        let prompted = Arc::new(Mutex::new(None));
        let prompted_clone = Arc::clone(&prompted);

        super::run_native_with_connector_prompt_resolver_and_io(
            &SshOptions {
                target: SshTarget::OpenSsh(OpenSshTarget {
                    target: "prod".to_owned(),
                    username: None,
                    port: None,
                    initial_size: TerminalSize::new(80, 24),
                    auth: rssh_ssh::SshAuthMethod::PasswordPrompt,
                }),
                remote_command: Vec::new(),
                forwards: Vec::new(),
                openssh_args: Vec::new(),
                no_shell: false,
                native: true,
                gui: false,
                renderer: crate::cli::RendererMode::Auto,
                benchmark_startup: false,
                native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
                console: crate::cli::ConsoleOptions::default(),
                osc52_policy: Osc52Policy::default(),
                log: None,
            },
            &mut connector,
            &mut forward_starter,
            &mut |target| {
                assert_eq!(target.target, "prod");
                Ok("hostname ssh.example.com\nuser deploy\nport 2222\n".to_owned())
            },
            &mut |request| {
                *prompted_clone.lock().unwrap() =
                    Some((request.config.username.clone(), request.config.host.clone()));
                Ok("secret".to_owned())
            },
            empty_input_events(),
            &mut output,
        )
        .unwrap();

        assert_eq!(
            *prompted.lock().unwrap(),
            Some(("deploy".to_owned(), "ssh.example.com".to_owned()))
        );
        let state = state.lock().unwrap();
        let request = state.last_request.as_ref().unwrap();
        assert_eq!(
            request.auth,
            rssh_ssh::SshAuthMethod::Password {
                password: "secret".to_owned()
            }
        );
    }

    #[test]
    fn native_ssh_runner_resolves_private_key_passphrase_before_connecting() {
        let key_path = PathBuf::from("C:/Users/ops/.ssh/id_ed25519");
        let request = SshConnectRequest::private_key(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
            key_path.clone(),
            None::<String>,
        )
        .unwrap();
        let state = Arc::new(Mutex::new(MockState::default()));
        let mut connector = MockConnector {
            state: Arc::clone(&state),
        };
        let mut output = Vec::new();

        super::run_native_with_connector_secret_prompts_and_io(
            &SshOptions {
                target: SshTarget::Direct(request),
                remote_command: Vec::new(),
                forwards: Vec::new(),
                openssh_args: Vec::new(),
                no_shell: false,
                native: true,
                gui: false,
                renderer: crate::cli::RendererMode::Auto,
                benchmark_startup: false,
                native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
                console: crate::cli::ConsoleOptions::default(),
                osc52_policy: Osc52Policy::default(),
                log: None,
            },
            &mut connector,
            &mut |_| Err("password prompt should not be used".into()),
            &mut |path: &Path| {
                assert_eq!(path, key_path.as_path());
                Ok("key-secret".to_owned())
            },
            &mut |path: &Path| {
                assert_eq!(path, key_path.as_path());
                Ok(true)
            },
            empty_input_events(),
            &mut output,
        )
        .unwrap();

        let state = state.lock().unwrap();
        let request = state.last_request.as_ref().unwrap();
        assert_eq!(
            request.auth,
            rssh_ssh::SshAuthMethod::PrivateKey {
                path: key_path,
                passphrase: Some("key-secret".to_owned()),
            }
        );
    }

    #[test]
    fn native_ssh_opener_uses_explicit_accept_unknown_host_key_policy() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );

        let opener = super::native_channel_opener_for_options(&SshOptions {
            target: SshTarget::Direct(request),
            remote_command: Vec::new(),
            forwards: Vec::new(),
            openssh_args: Vec::new(),
            no_shell: false,
            native: true,
            gui: false,
            renderer: crate::cli::RendererMode::Auto,
            benchmark_startup: false,
            native_host_key_policy: NativeHostKeyPolicy::AcceptUnknown,
            console: crate::cli::ConsoleOptions::default(),
            osc52_policy: Osc52Policy::default(),
            log: None,
        });

        assert_eq!(
            opener.host_key_policy(),
            rssh_ssh::RusshHostKeyPolicy::AcceptUnknown
        );
    }

    #[test]
    fn native_ssh_opener_uses_trust_on_first_use_host_key_policy() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );

        let opener = super::native_channel_opener_for_options(&SshOptions {
            target: SshTarget::Direct(request),
            remote_command: Vec::new(),
            forwards: Vec::new(),
            openssh_args: Vec::new(),
            no_shell: false,
            native: true,
            gui: false,
            renderer: crate::cli::RendererMode::Auto,
            benchmark_startup: false,
            native_host_key_policy: NativeHostKeyPolicy::TrustOnFirstUse,
            console: crate::cli::ConsoleOptions::default(),
            osc52_policy: Osc52Policy::default(),
            log: None,
        });

        assert_eq!(
            opener.host_key_policy(),
            rssh_ssh::RusshHostKeyPolicy::TrustOnFirstUse
        );
    }

    #[test]
    fn native_ssh_opener_uses_default_known_hosts_for_trust_on_first_use() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );

        let opener = super::native_channel_opener_for_options(&SshOptions {
            target: SshTarget::Direct(request),
            remote_command: Vec::new(),
            forwards: Vec::new(),
            openssh_args: Vec::new(),
            no_shell: false,
            native: true,
            gui: false,
            renderer: crate::cli::RendererMode::Auto,
            benchmark_startup: false,
            native_host_key_policy: NativeHostKeyPolicy::TrustOnFirstUse,
            console: crate::cli::ConsoleOptions::default(),
            osc52_policy: Osc52Policy::default(),
            log: None,
        });

        let known_hosts_path = opener.known_hosts_path().unwrap();
        assert_eq!(
            known_hosts_path.file_name().unwrap(),
            std::ffi::OsStr::new("known_hosts")
        );
        assert_eq!(
            known_hosts_path.parent().unwrap().file_name().unwrap(),
            std::ffi::OsStr::new(".ssh")
        );
    }

    #[test]
    fn openssh_command_uses_target_port_and_tty() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new(
                "example.com",
                2222,
                "ops",
                TerminalSize::new(120, 30).into(),
            )
            .unwrap(),
        );

        let command = super::openssh_command_for_options(&direct_options(request));

        assert_eq!(command.program(), "ssh");
        assert_eq!(command.args(), ["-tt", "-p", "2222", "ops@example.com"]);
    }

    #[test]
    fn openssh_command_adds_private_key_without_leaking_passphrase() {
        let request = SshConnectRequest::private_key(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
            "C:/Users/ops/.ssh/id_ed25519",
            Some("secret"),
        )
        .unwrap();

        let command = super::openssh_command_for_options(&direct_options(request));
        let joined = command.args().join(" ");

        assert_eq!(
            command.args(),
            [
                "-tt",
                "-i",
                "C:/Users/ops/.ssh/id_ed25519",
                "ops@example.com"
            ]
        );
        assert!(!joined.contains("secret"));
    }

    #[test]
    fn openssh_command_uses_password_prompt_policy_without_leaking_password() {
        let request = SshConnectRequest::password(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
            "secret",
        )
        .unwrap();

        let command = super::openssh_command_for_options(&direct_options(request));
        let joined = command.args().join(" ");

        assert_eq!(
            command.args(),
            [
                "-tt",
                "-o",
                "PreferredAuthentications=password,keyboard-interactive",
                "ops@example.com"
            ]
        );
        assert!(!joined.contains("secret"));
    }

    #[test]
    fn openssh_local_options_use_requested_terminal_size_and_mouse() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(132, 43).into())
                .unwrap(),
        );

        let options = super::local_options_for_request(&request).unwrap();

        let size = options.size.unwrap();
        assert_eq!(size.columns(), 132);
        assert_eq!(size.rows(), 43);
        assert!(options.mouse);
    }

    #[test]
    fn openssh_local_options_preserve_osc52_policy() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 22, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );
        let options = SshOptions {
            target: SshTarget::Direct(request),
            remote_command: Vec::new(),
            forwards: Vec::new(),
            openssh_args: Vec::new(),
            no_shell: false,
            native: false,
            gui: false,
            renderer: crate::cli::RendererMode::Auto,
            benchmark_startup: false,
            native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
            console: crate::cli::ConsoleOptions::default(),
            osc52_policy: Osc52Policy::Off,
            log: None,
        };

        let local_options = super::local_options_for_options(&options).unwrap();

        assert_eq!(local_options.osc52_policy, Osc52Policy::Off);
    }

    #[test]
    fn openssh_command_uses_config_target_with_overrides() {
        let options = SshOptions {
            target: SshTarget::OpenSsh(OpenSshTarget {
                target: "prod".to_owned(),
                username: Some("ops".to_owned()),
                port: Some(2222),
                initial_size: TerminalSize::new(120, 30),
                auth: rssh_ssh::SshAuthMethod::PrivateKey {
                    path: "C:/Users/ops/.ssh/id_ed25519".into(),
                    passphrase: None,
                },
            }),
            remote_command: Vec::new(),
            forwards: Vec::new(),
            openssh_args: Vec::new(),
            no_shell: false,
            native: false,
            gui: false,
            renderer: crate::cli::RendererMode::Auto,
            benchmark_startup: false,
            native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
            console: crate::cli::ConsoleOptions::default(),
            osc52_policy: Osc52Policy::default(),
            log: None,
        };

        let command = super::openssh_command_for_options(&options);

        assert_eq!(
            command.args(),
            [
                "-tt",
                "-i",
                "C:/Users/ops/.ssh/id_ed25519",
                "-l",
                "ops",
                "-p",
                "2222",
                "prod"
            ]
        );
    }

    #[test]
    fn openssh_command_preserves_passthrough_options_before_target() {
        let options = SshOptions {
            target: SshTarget::OpenSsh(OpenSshTarget {
                target: "prod".to_owned(),
                username: None,
                port: None,
                initial_size: TerminalSize::new(80, 24),
                auth: rssh_ssh::SshAuthMethod::Agent,
            }),
            remote_command: Vec::new(),
            forwards: Vec::new(),
            openssh_args: vec![
                "-F".to_owned(),
                "C:/Users/ops/.ssh/prod_config".to_owned(),
                "-o".to_owned(),
                "ProxyJump=bastion".to_owned(),
            ],
            no_shell: false,
            native: false,
            gui: false,
            renderer: crate::cli::RendererMode::Auto,
            benchmark_startup: false,
            native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
            console: crate::cli::ConsoleOptions::default(),
            osc52_policy: Osc52Policy::default(),
            log: None,
        };

        let command = super::openssh_command_for_options(&options);

        assert_eq!(
            command.args(),
            [
                "-tt",
                "-F",
                "C:/Users/ops/.ssh/prod_config",
                "-o",
                "ProxyJump=bastion",
                "prod"
            ]
        );
    }

    #[test]
    fn openssh_command_appends_remote_command_after_target() {
        let options = SshOptions {
            target: SshTarget::OpenSsh(OpenSshTarget {
                target: "prod".to_owned(),
                username: None,
                port: None,
                initial_size: TerminalSize::new(80, 24),
                auth: rssh_ssh::SshAuthMethod::Agent,
            }),
            remote_command: vec!["uname".to_owned(), "-a".to_owned()],
            forwards: Vec::new(),
            openssh_args: Vec::new(),
            no_shell: false,
            native: false,
            gui: false,
            renderer: crate::cli::RendererMode::Auto,
            benchmark_startup: false,
            native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
            console: crate::cli::ConsoleOptions::default(),
            osc52_policy: Osc52Policy::default(),
            log: None,
        };

        let command = super::openssh_command_for_options(&options);

        assert_eq!(command.args(), ["-tt", "prod", "uname", "-a"]);
    }

    #[test]
    fn openssh_command_adds_forwarding_and_no_shell_before_target() {
        let options = SshOptions {
            target: SshTarget::OpenSsh(OpenSshTarget {
                target: "prod".to_owned(),
                username: None,
                port: None,
                initial_size: TerminalSize::new(80, 24),
                auth: rssh_ssh::SshAuthMethod::Agent,
            }),
            remote_command: Vec::new(),
            forwards: vec![
                crate::cli::SshForward::Local("127.0.0.1:15432:db.internal:5432".to_owned()),
                crate::cli::SshForward::Remote("8080:127.0.0.1:80".to_owned()),
                crate::cli::SshForward::Dynamic("127.0.0.1:1080".to_owned()),
            ],
            openssh_args: Vec::new(),
            no_shell: true,
            native: false,
            gui: false,
            renderer: crate::cli::RendererMode::Auto,
            benchmark_startup: false,
            native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
            console: crate::cli::ConsoleOptions::default(),
            osc52_policy: Osc52Policy::default(),
            log: None,
        };

        let command = super::openssh_command_for_options(&options);

        assert_eq!(
            command.args(),
            [
                "-N",
                "-L",
                "127.0.0.1:15432:db.internal:5432",
                "-R",
                "8080:127.0.0.1:80",
                "-D",
                "127.0.0.1:1080",
                "prod"
            ]
        );
    }

    #[derive(Default)]
    struct MockState {
        last_request: Option<SshConnectRequest>,
        written: Vec<u8>,
        input_finished: bool,
        closed: bool,
        remote_exit_status: Option<u32>,
        remote_exit_signal: Option<SshExitSignal>,
    }

    struct MockConnector {
        state: Arc<Mutex<MockState>>,
    }

    impl SshShellConnector for MockConnector {
        fn connect(
            &mut self,
            request: SshConnectRequest,
        ) -> Result<Box<dyn SshShellSession>, SshSessionError> {
            self.state.lock().unwrap().last_request = Some(request);
            Ok(Box::new(MockSession {
                state: Arc::clone(&self.state),
                read_once: false,
            }))
        }
    }

    struct MockSession {
        state: Arc<Mutex<MockState>>,
        read_once: bool,
    }

    impl SshShellSession for MockSession {
        fn read(&mut self, buffer: &mut [u8]) -> Result<usize, SshSessionError> {
            if self.read_once {
                return Ok(0);
            }
            self.read_once = true;
            buffer[..7].copy_from_slice(b"remote\n");
            Ok(7)
        }

        fn write(&mut self, bytes: &[u8]) -> Result<usize, SshSessionError> {
            self.state.lock().unwrap().written.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn resize(&mut self, _size: rssh_ssh::SshTerminalSize) -> Result<(), SshSessionError> {
            Ok(())
        }

        fn keepalive(&mut self) -> Result<(), SshSessionError> {
            Ok(())
        }

        fn close(&mut self) -> Result<(), SshSessionError> {
            self.state.lock().unwrap().closed = true;
            Ok(())
        }

        fn into_read_writer(self: Box<Self>) -> (Box<dyn SshShellReader>, Box<dyn SshShellWriter>) {
            (
                Box::new(MockReader {
                    state: Arc::clone(&self.state),
                    read_once: self.read_once,
                }),
                Box::new(MockWriter { state: self.state }),
            )
        }
    }

    struct MockReader {
        state: Arc<Mutex<MockState>>,
        read_once: bool,
    }

    impl SshShellReader for MockReader {
        fn read(&mut self, buffer: &mut [u8]) -> Result<usize, SshSessionError> {
            if self.read_once {
                while !self.state.lock().unwrap().input_finished {
                    std::thread::yield_now();
                }
                return Ok(0);
            }
            self.read_once = true;
            buffer[..7].copy_from_slice(b"remote\n");
            Ok(7)
        }

        fn session_result(&self) -> SshSessionResult {
            let state = self.state.lock().unwrap();
            SshSessionResult {
                exit_status: state.remote_exit_status,
                exit_signal: state.remote_exit_signal.clone(),
            }
        }
    }

    struct MockWriter {
        state: Arc<Mutex<MockState>>,
    }

    impl SshShellWriter for MockWriter {
        fn write(&mut self, bytes: &[u8]) -> Result<usize, SshSessionError> {
            self.state.lock().unwrap().written.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn resize(&mut self, _size: rssh_ssh::SshTerminalSize) -> Result<(), SshSessionError> {
            Ok(())
        }

        fn keepalive(&mut self) -> Result<(), SshSessionError> {
            Ok(())
        }

        fn finish_input(&mut self) -> Result<(), SshSessionError> {
            self.state.lock().unwrap().input_finished = true;
            Ok(())
        }

        fn close(&mut self) -> Result<(), SshSessionError> {
            self.state.lock().unwrap().closed = true;
            Ok(())
        }
    }

    #[derive(Default)]
    struct MockForwardState {
        local: Vec<super::NativeLocalForward>,
        dynamic: Vec<super::NativeDynamicForward>,
        remote: Vec<super::NativeRemoteForward>,
        shutdowns: usize,
    }

    struct MockForwardStarter {
        state: Arc<Mutex<MockForwardState>>,
    }

    impl super::NativeLocalForwardStarter for MockForwardStarter {
        fn start(
            &mut self,
            _request: SshConnectRequest,
            forward: super::NativeLocalForward,
        ) -> Result<Box<dyn super::NativeLocalForwardHandle>, Box<dyn std::error::Error>> {
            self.state.lock().unwrap().local.push(forward);
            Ok(Box::new(MockForwardHandle {
                state: Arc::clone(&self.state),
            }))
        }

        fn start_dynamic(
            &mut self,
            _request: SshConnectRequest,
            forward: super::NativeDynamicForward,
        ) -> Result<Box<dyn super::NativeLocalForwardHandle>, Box<dyn std::error::Error>> {
            self.state.lock().unwrap().dynamic.push(forward);
            Ok(Box::new(MockForwardHandle {
                state: Arc::clone(&self.state),
            }))
        }

        fn start_remote(
            &mut self,
            _request: SshConnectRequest,
            forward: super::NativeRemoteForward,
        ) -> Result<Box<dyn super::NativeLocalForwardHandle>, Box<dyn std::error::Error>> {
            self.state.lock().unwrap().remote.push(forward);
            Ok(Box::new(MockForwardHandle {
                state: Arc::clone(&self.state),
            }))
        }
    }

    struct MockForwardHandle {
        state: Arc<Mutex<MockForwardState>>,
    }

    impl super::NativeLocalForwardHandle for MockForwardHandle {
        fn is_finished(&self) -> bool {
            true
        }

        fn cancel(&mut self) {}

        fn shutdown(&mut self, _timeout: Duration) -> Result<(), Box<dyn std::error::Error>> {
            self.state.lock().unwrap().shutdowns += 1;
            Ok(())
        }

        fn wait(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }
    }

    struct CompletionMockForwardHandle {
        name: &'static str,
        finished: bool,
        wait_error: Option<&'static str>,
        events: Arc<Mutex<Vec<String>>>,
    }

    impl CompletionMockForwardHandle {
        fn new(
            name: &'static str,
            finished: bool,
            wait_error: Option<&'static str>,
            events: Arc<Mutex<Vec<String>>>,
        ) -> Self {
            Self {
                name,
                finished,
                wait_error,
                events,
            }
        }

        fn record(&self, action: &str) {
            self.events
                .lock()
                .unwrap()
                .push(format!("{action}:{}", self.name));
        }
    }

    impl super::NativeLocalForwardHandle for CompletionMockForwardHandle {
        fn is_finished(&self) -> bool {
            self.finished
        }

        fn cancel(&mut self) {
            self.record("cancel");
        }

        fn shutdown(&mut self, _timeout: Duration) -> Result<(), Box<dyn std::error::Error>> {
            self.record("shutdown");
            Ok(())
        }

        fn wait(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            self.record("wait");
            self.wait_error.map_or(Ok(()), |error| Err(error.into()))
        }
    }

    struct FailingSecondForwardStarter {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl super::NativeLocalForwardStarter for FailingSecondForwardStarter {
        fn start(
            &mut self,
            _request: SshConnectRequest,
            _forward: super::NativeLocalForward,
        ) -> Result<Box<dyn super::NativeLocalForwardHandle>, Box<dyn std::error::Error>> {
            Ok(Box::new(CompletionMockForwardHandle::new(
                "first",
                false,
                None,
                Arc::clone(&self.events),
            )))
        }

        fn start_dynamic(
            &mut self,
            _request: SshConnectRequest,
            _forward: super::NativeDynamicForward,
        ) -> Result<Box<dyn super::NativeLocalForwardHandle>, Box<dyn std::error::Error>> {
            Err("second forward startup failed".into())
        }

        fn start_remote(
            &mut self,
            _request: SshConnectRequest,
            _forward: super::NativeRemoteForward,
        ) -> Result<Box<dyn super::NativeLocalForwardHandle>, Box<dyn std::error::Error>> {
            unreachable!("remote forwarding is not part of this test")
        }
    }

    fn direct_options(request: SshConnectRequest) -> SshOptions {
        SshOptions {
            target: SshTarget::Direct(request),
            remote_command: Vec::new(),
            forwards: Vec::new(),
            openssh_args: Vec::new(),
            no_shell: false,
            native: false,
            gui: false,
            renderer: crate::cli::RendererMode::Auto,
            benchmark_startup: false,
            native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
            console: crate::cli::ConsoleOptions::default(),
            osc52_policy: Osc52Policy::default(),
            log: None,
        }
    }
}
