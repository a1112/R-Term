use std::{
    fmt, io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use super::lifecycle::{ShutdownDeadline, ThreadJoinOutcome, join_thread_until};

const POLL_INTERVAL: Duration = Duration::from_millis(5);
const DROP_DEADLINE: Duration = Duration::from_millis(500);

/// A validated loopback-only TCP endpoint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopbackEndpoint {
    host: String,
    port: u16,
    addresses: Vec<SocketAddr>,
}

impl LoopbackEndpoint {
    /// Validates a numeric loopback address or the literal `localhost`.
    ///
    /// # Errors
    ///
    /// Returns [`LoopbackPolicyError`] for names and addresses which are not loopback.
    pub fn new(host: impl Into<String>, port: u16) -> Result<Self, LoopbackPolicyError> {
        let host = host.into();
        let addresses = if host == "localhost" {
            vec![
                SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
                SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), port),
            ]
        } else {
            let address = host
                .parse::<IpAddr>()
                .map_err(|_| LoopbackPolicyError { host: host.clone() })?;
            if !address.is_loopback() {
                return Err(LoopbackPolicyError { host });
            }
            vec![SocketAddr::new(address, port)]
        };
        Ok(Self {
            host,
            port,
            addresses,
        })
    }

    /// Returns the original validated host.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns the validated port.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Returns only loopback socket addresses.
    #[must_use]
    pub fn socket_addrs(&self) -> &[SocketAddr] {
        &self.addresses
    }
}

/// A rejected forwarding endpoint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopbackPolicyError {
    host: String,
}

impl fmt::Display for LoopbackPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "SSH test forwarding is restricted to loopback; rejected host {:?}",
            self.host
        )
    }
}

impl std::error::Error for LoopbackPolicyError {}

/// A real loopback TCP echo service used as a forwarding target.
pub struct LoopbackEchoServer {
    address: SocketAddr,
    control: Arc<EchoControl>,
    connection_probe: LoopbackEchoProbe,
    completion: mpsc::Receiver<io::Result<()>>,
    worker: Option<thread::JoinHandle<()>>,
}

/// A cloneable observation handle for echo connection worker ownership.
#[derive(Clone)]
pub struct LoopbackEchoProbe {
    active: Arc<AtomicUsize>,
}

impl LoopbackEchoProbe {
    /// Returns the number of connection threads which have not completed.
    #[must_use]
    pub fn active(&self) -> usize {
        self.active.load(Ordering::Acquire)
    }
}

struct EchoControl {
    cancelled: AtomicBool,
    deadline: Mutex<Option<ShutdownDeadline>>,
}

impl EchoControl {
    fn cancel(&self, deadline: ShutdownDeadline) {
        let mut stored = self
            .deadline
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if stored.is_none() {
            *stored = Some(deadline);
        }
        self.cancelled.store(true, Ordering::Release);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    fn shutdown_deadline(&self) -> ShutdownDeadline {
        self.deadline
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .unwrap_or_else(|| ShutdownDeadline::after(DROP_DEADLINE))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EchoWorkerPanic {
    BeforeReady,
    BeforeCompletion,
}

#[derive(Clone, Copy, Default)]
struct EchoConnectionBehavior {
    slow_shutdown: Option<Duration>,
    fail_timeout_setup: bool,
    panic_after_accept: bool,
    worker_panic: Option<EchoWorkerPanic>,
}

impl LoopbackEchoServer {
    /// Binds a ready echo server within `deadline`.
    ///
    /// # Errors
    ///
    /// Returns an error if loopback binding, worker creation, or readiness exceeds
    /// the supplied deadline.
    pub fn start(deadline: Duration) -> io::Result<Self> {
        Self::start_configured(deadline, EchoConnectionBehavior::default())
    }

    #[cfg(test)]
    fn start_with_slow_connection(deadline: Duration, delay: Duration) -> io::Result<Self> {
        Self::start_configured(
            deadline,
            EchoConnectionBehavior {
                slow_shutdown: Some(delay),
                ..EchoConnectionBehavior::default()
            },
        )
    }

    #[cfg(test)]
    fn start_with_timeout_setup_failure(deadline: Duration) -> io::Result<Self> {
        Self::start_configured(
            deadline,
            EchoConnectionBehavior {
                fail_timeout_setup: true,
                ..EchoConnectionBehavior::default()
            },
        )
    }

    #[cfg(test)]
    fn start_with_connection_panic(deadline: Duration) -> io::Result<Self> {
        Self::start_configured(
            deadline,
            EchoConnectionBehavior {
                panic_after_accept: true,
                ..EchoConnectionBehavior::default()
            },
        )
    }

    #[cfg(test)]
    fn start_with_worker_panic(deadline: Duration) -> io::Result<Self> {
        Self::start_configured(
            deadline,
            EchoConnectionBehavior {
                worker_panic: Some(EchoWorkerPanic::BeforeCompletion),
                ..EchoConnectionBehavior::default()
            },
        )
    }

    #[cfg(test)]
    fn start_with_startup_worker_panic(deadline: Duration) -> io::Result<Self> {
        Self::start_configured(
            deadline,
            EchoConnectionBehavior {
                worker_panic: Some(EchoWorkerPanic::BeforeReady),
                ..EchoConnectionBehavior::default()
            },
        )
    }

    fn start_configured(deadline: Duration, behavior: EchoConnectionBehavior) -> io::Result<Self> {
        let startup = ShutdownDeadline::after(deadline);
        super::lifecycle::ensure_process_reaper()?;
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let control = Arc::new(EchoControl {
            cancelled: AtomicBool::new(false),
            deadline: Mutex::new(None),
        });
        let worker_control = Arc::clone(&control);
        let connection_probe = LoopbackEchoProbe {
            active: Arc::new(AtomicUsize::new(0)),
        };
        let worker_active = Arc::clone(&connection_probe.active);
        let (completion_tx, completion) = mpsc::sync_channel(1);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("rterm-loopback-echo".to_owned())
            .spawn(move || {
                assert!(
                    behavior.worker_panic != Some(EchoWorkerPanic::BeforeReady),
                    "injected echo startup worker panic"
                );
                let _ = ready_tx.send(());
                let result = run_echo_listener(listener, &worker_control, &worker_active, behavior);
                assert!(
                    behavior.worker_panic != Some(EchoWorkerPanic::BeforeCompletion),
                    "injected echo worker panic"
                );
                let _ = completion_tx.send(result);
            })?;
        match ready_rx.recv_timeout(startup.remaining()) {
            Ok(()) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {
                control.cancel(startup);
                defer_echo_join(worker);
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "loopback echo server did not become ready before deadline",
                ));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return finish_missing_echo_readiness(worker, startup);
            }
        }
        Ok(Self {
            address,
            control,
            connection_probe,
            completion,
            worker: Some(worker),
        })
    }

    /// Returns the bound loopback address.
    #[must_use]
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// Returns a probe for asserting that connection threads are eventually reaped.
    #[must_use]
    pub fn connection_probe(&self) -> LoopbackEchoProbe {
        self.connection_probe.clone()
    }

    /// Cancels and joins the echo listener within `deadline`.
    ///
    /// # Errors
    ///
    /// Returns an error if teardown exceeds the deadline or a worker fails.
    pub fn stop(mut self, deadline: Duration) -> io::Result<()> {
        self.stop_inner(deadline)
    }

    fn stop_inner(&mut self, deadline: Duration) -> io::Result<()> {
        let shutdown = ShutdownDeadline::after(deadline);
        self.control.cancel(shutdown);
        let result = match self.completion.recv_timeout(shutdown.remaining()) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(worker) = self.worker.take() {
                    defer_echo_join(worker);
                }
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("loopback echo teardown exceeded {deadline:?}"),
                ));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return finish_missing_echo_completion(self.worker.take(), shutdown);
            }
        };
        if let Some(worker) = self.worker.take() {
            join_echo_worker_until(worker, shutdown)?;
        }
        result
    }
}

fn defer_echo_join(worker: thread::JoinHandle<()>) {
    super::lifecycle::defer_thread(worker);
}

fn join_echo_worker_until(
    worker: thread::JoinHandle<()>,
    deadline: ShutdownDeadline,
) -> io::Result<()> {
    match join_thread_until(worker, deadline) {
        ThreadJoinOutcome::Completed => Ok(()),
        ThreadJoinOutcome::Panicked => Err(io::Error::other("loopback echo worker panicked")),
        ThreadJoinOutcome::Deferred => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!("loopback echo worker join exceeded {:?}", deadline.budget()),
        )),
    }
}

fn finish_missing_echo_completion(
    worker: Option<thread::JoinHandle<()>>,
    deadline: ShutdownDeadline,
) -> io::Result<()> {
    if let Some(worker) = worker {
        join_echo_worker_until(worker, deadline)?;
    }
    Err(io::Error::new(
        io::ErrorKind::BrokenPipe,
        "loopback echo worker exited without completion",
    ))
}

fn finish_missing_echo_readiness(
    worker: thread::JoinHandle<()>,
    deadline: ShutdownDeadline,
) -> io::Result<LoopbackEchoServer> {
    join_echo_worker_until(worker, deadline)?;
    Err(io::Error::new(
        io::ErrorKind::BrokenPipe,
        "loopback echo worker exited without readiness",
    ))
}

impl Drop for LoopbackEchoServer {
    fn drop(&mut self) {
        if self.worker.is_some() {
            let _ = self.stop_inner(DROP_DEADLINE);
        }
    }
}

fn run_echo_listener(
    listener: TcpListener,
    control: &Arc<EchoControl>,
    active: &Arc<AtomicUsize>,
    behavior: EchoConnectionBehavior,
) -> io::Result<()> {
    let mut connections = Vec::new();
    let mut outcome = Ok(());
    while !control.is_cancelled() {
        match listener.accept() {
            Ok((stream, _)) => match spawn_echo_connection(stream, control, active, behavior) {
                Ok(connection) => connections.push(connection),
                Err(error) => {
                    outcome = Err(error);
                    control.cancel(ShutdownDeadline::after(DROP_DEADLINE));
                }
            },
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::park_timeout(POLL_INTERVAL);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => {
                outcome = Err(error);
                control.cancel(ShutdownDeadline::after(DROP_DEADLINE));
            }
        }
        if let Err(error) = join_finished_connections(&mut connections) {
            if outcome.is_ok() {
                outcome = Err(error);
            }
            control.cancel(ShutdownDeadline::after(DROP_DEADLINE));
        }
    }
    drop(listener);
    if let Err(error) = drain_connection_threads(&mut connections, control.shutdown_deadline())
        && outcome.is_ok()
    {
        outcome = Err(error);
    }
    outcome
}

struct EchoConnection {
    worker: thread::JoinHandle<()>,
    completion: mpsc::Receiver<io::Result<()>>,
}

fn spawn_echo_connection(
    stream: TcpStream,
    control: &Arc<EchoControl>,
    active: &Arc<AtomicUsize>,
    behavior: EchoConnectionBehavior,
) -> io::Result<EchoConnection> {
    configure_echo_stream(&stream, behavior)?;
    let connection_control = Arc::clone(control);
    let connection_active = Arc::clone(active);
    let (completion_tx, completion) = mpsc::sync_channel(1);
    connection_active.fetch_add(1, Ordering::AcqRel);
    let guard = ActiveConnectionGuard(Arc::clone(&connection_active));
    let worker = thread::Builder::new()
        .name("rterm-loopback-echo-connection".to_owned())
        .spawn(move || {
            let _guard = guard;
            assert!(
                !behavior.panic_after_accept,
                "injected echo connection panic"
            );
            let result = echo_connection(stream, &connection_control, behavior.slow_shutdown);
            let _ = completion_tx.send(result);
        })?;
    Ok(EchoConnection { worker, completion })
}

fn configure_echo_stream(stream: &TcpStream, behavior: EchoConnectionBehavior) -> io::Result<()> {
    if behavior.fail_timeout_setup {
        return Err(io::Error::other("injected timeout setup failure"));
    }
    stream.set_read_timeout(Some(Duration::from_millis(100)))?;
    stream.set_write_timeout(Some(Duration::from_millis(100)))?;
    Ok(())
}

fn join_finished_connections(connections: &mut Vec<EchoConnection>) -> io::Result<()> {
    let mut first_error = None;
    let mut index = 0;
    while index < connections.len() {
        if connections[index].worker.is_finished() {
            let connection = connections.swap_remove(index);
            if let Err(error) = join_connection(connection) {
                first_error.get_or_insert(error);
            }
        } else {
            index += 1;
        }
    }
    first_error.map_or(Ok(()), Err)
}

fn join_connection(connection: EchoConnection) -> io::Result<()> {
    connection
        .worker
        .join()
        .map_err(|_| io::Error::other("loopback echo connection worker panicked"))?;
    connection.completion.recv().map_err(|error| {
        io::Error::other(format!("loopback echo connection result missing: {error}"))
    })?
}

fn drain_connection_threads(
    connections: &mut Vec<EchoConnection>,
    deadline: ShutdownDeadline,
) -> io::Result<()> {
    let mut first_error = None;
    while !connections.is_empty() && deadline.remaining() > Duration::ZERO {
        if let Err(error) = join_finished_connections(connections) {
            first_error.get_or_insert(error);
        }
        if !connections.is_empty() {
            thread::park_timeout(deadline.remaining().min(POLL_INTERVAL));
        }
    }
    if let Err(error) = join_finished_connections(connections) {
        first_error.get_or_insert(error);
    }
    if !connections.is_empty() {
        for connection in connections.drain(..) {
            drop(connection.completion);
            defer_echo_join(connection.worker);
        }
        first_error.get_or_insert_with(|| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "loopback echo connection drain exceeded {:?}",
                    deadline.budget()
                ),
            )
        });
    }
    first_error.map_or(Ok(()), Err)
}

struct ActiveConnectionGuard(Arc<AtomicUsize>);

impl Drop for ActiveConnectionGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

fn echo_connection(
    mut stream: TcpStream,
    control: &EchoControl,
    slow_shutdown: Option<Duration>,
) -> io::Result<()> {
    let mut buffer = [0_u8; 8192];
    while !control.is_cancelled() {
        match io::Read::read(&mut stream, &mut buffer) {
            Ok(0) => return Ok(()),
            Ok(length) => {
                io::Write::write_all(&mut stream, &buffer[..length])?;
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted
                ) =>
            {
                return Ok(());
            }
            Err(error) => return Err(error),
        }
    }
    if let Some(delay) = slow_shutdown {
        thread::sleep(delay);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read as _, Write as _},
        net::{IpAddr, Ipv4Addr, TcpListener, TcpStream},
        time::{Duration, Instant},
    };

    use super::{LoopbackEchoServer, LoopbackEndpoint, POLL_INTERVAL};

    #[test]
    fn endpoint_accepts_only_loopback_hosts() {
        for host in ["127.0.0.1", "::1", "localhost"] {
            let endpoint = LoopbackEndpoint::new(host, 22).expect("loopback accepted");
            assert!(
                endpoint
                    .socket_addrs()
                    .iter()
                    .all(|address| address.ip().is_loopback())
            );
        }
        for host in ["0.0.0.0", "::", "192.0.2.1", "example.com"] {
            assert!(LoopbackEndpoint::new(host, 22).is_err(), "accepted {host}");
        }
    }

    #[test]
    fn echo_server_uses_real_tcp_and_releases_its_port() {
        let echo = LoopbackEchoServer::start(Duration::from_secs(2)).expect("start echo server");
        let address = echo.address();
        assert_eq!(address.ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(1)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        stream.write_all(b"loopback-forward").unwrap();
        let mut response = [0; 16];
        stream.read_exact(&mut response).unwrap();
        assert_eq!(&response, b"loopback-forward");
        drop(stream);

        let started = Instant::now();
        echo.stop(Duration::from_secs(2)).expect("stop echo server");
        assert!(started.elapsed() < Duration::from_secs(2));
        TcpListener::bind(address).expect("echo port released");
    }

    #[test]
    fn echo_stop_closes_established_connections_before_returning() {
        let echo = LoopbackEchoServer::start(Duration::from_secs(2)).expect("start echo server");
        let mut stream = TcpStream::connect(echo.address()).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        stream.write_all(b"ready").unwrap();
        let mut ready = [0_u8; 5];
        stream.read_exact(&mut ready).unwrap();
        assert_eq!(&ready, b"ready");
        echo.stop(Duration::from_secs(2)).expect("stop echo server");
        let _ = stream.write_all(b"after-stop");
        let mut byte = [0_u8; 1];
        assert_eq!(stream.read(&mut byte).unwrap_or(0), 0);
    }

    #[test]
    fn echo_stop_hands_a_slow_connection_to_the_shared_reaper() {
        let echo = LoopbackEchoServer::start_with_slow_connection(
            Duration::from_secs(2),
            Duration::from_millis(500),
        )
        .expect("start slow echo server");
        let address = echo.address();
        let probe = echo.connection_probe();
        let _stream = TcpStream::connect(address).unwrap();
        let active_deadline = Instant::now() + Duration::from_secs(1);
        while probe.active() == 0 && Instant::now() < active_deadline {
            std::thread::sleep(POLL_INTERVAL);
        }
        assert_eq!(probe.active(), 1);

        let started = Instant::now();
        let error = echo
            .stop(Duration::from_millis(30))
            .expect_err("slow connection must exceed stop deadline");
        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_millis(150));
        let release_deadline = Instant::now() + Duration::from_millis(250);
        let released_listener = loop {
            match TcpListener::bind(address) {
                Ok(listener) => break listener,
                Err(error)
                    if error.kind() == std::io::ErrorKind::AddrInUse
                        && Instant::now() < release_deadline =>
                {
                    std::thread::sleep(POLL_INTERVAL);
                }
                Err(error) => {
                    panic!("echo port released independently of slow connection: {error}")
                }
            }
        };
        assert_eq!(probe.active(), 1);
        drop(released_listener);

        let reaped_deadline = Instant::now() + Duration::from_secs(1);
        while probe.active() != 0 && Instant::now() < reaped_deadline {
            std::thread::sleep(POLL_INTERVAL);
        }
        assert_eq!(probe.active(), 0);
    }

    #[test]
    fn echo_timeout_setup_failure_is_observable_without_spawning_a_connection() {
        let echo = LoopbackEchoServer::start_with_timeout_setup_failure(Duration::from_secs(2))
            .expect("start echo server with timeout setup seam");
        let address = echo.address();
        let probe = echo.connection_probe();
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut byte = [0_u8; 1];
        assert_eq!(stream.read(&mut byte).unwrap(), 0);

        let error = echo
            .stop(Duration::from_secs(1))
            .expect_err("timeout setup failure must be returned by stop");
        assert_eq!(error.kind(), std::io::ErrorKind::Other);
        assert!(error.to_string().contains("injected timeout setup failure"));
        assert_eq!(probe.active(), 0);
        TcpListener::bind(address).expect("failed setup releases echo port");
    }

    #[test]
    fn echo_connection_panic_is_observed_by_the_listener() {
        let echo = LoopbackEchoServer::start_with_connection_panic(Duration::from_secs(2))
            .expect("start echo server with panic seam");
        let address = echo.address();
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut byte = [0_u8; 1];
        assert_eq!(stream.read(&mut byte).unwrap(), 0);
        let error = echo
            .stop(Duration::from_secs(1))
            .expect_err("connection panic must be returned by stop");
        assert!(error.to_string().contains("connection worker panicked"));
        TcpListener::bind(address).expect("connection panic releases echo port");
    }

    #[test]
    fn echo_top_level_worker_panic_is_not_misclassified_as_timeout() {
        let echo = LoopbackEchoServer::start_with_worker_panic(Duration::from_secs(2))
            .expect("start echo worker panic seam");
        let error = echo
            .stop(Duration::from_secs(1))
            .expect_err("echo worker panic must be observable");
        assert!(error.to_string().contains("echo worker panicked"));
        assert_ne!(error.kind(), std::io::ErrorKind::TimedOut);
    }

    #[test]
    fn echo_startup_worker_panic_is_not_misclassified_as_timeout() {
        let Err(error) =
            LoopbackEchoServer::start_with_startup_worker_panic(Duration::from_secs(1))
        else {
            panic!("echo startup worker panic must be observable");
        };
        assert!(error.to_string().contains("echo worker panicked"));
        assert_ne!(error.kind(), std::io::ErrorKind::TimedOut);
    }
}
