use std::{
    collections::VecDeque,
    error::Error,
    fs::File,
    io::{self, IsTerminal, Read, Write},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU8, AtomicU16, AtomicU64, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use base64::{Engine, engine::general_purpose::STANDARD};
use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyEventState, KeyModifiers, MediaKeyCode, ModifierKeyCode, MouseButton, MouseEvent,
        MouseEventKind,
    },
    execute, terminal,
};
use rterm_core::{
    SessionId, TerminalSize,
    session::{SessionLifecycle, SessionState},
};
use rterm_pty::{PtyBackend, PtyExitStatus, PtyMasterClose, PtyMasterCloseStatus, PtySize};
use rterm_runtime::{
    RuntimeBuffers, RuntimeDelta, RuntimeEffectRef, SessionTransport,
    TerminalRuntime as SharedTerminalRuntime,
};
#[cfg(test)]
use rterm_terminal::{Cell, Color, CursorShape, Terminal, UnderlineStyle, VerticalAlign};
use serde::Serialize;

use crate::{
    cli::{LocalOptions, Osc52Policy},
    diagnostics,
    terminal_input::{TerminalKey, encode_terminal_key},
    terminal_modes::{
        KITTY_KEYBOARD_ALTERNATE_KEYS, KITTY_KEYBOARD_ASSOCIATED_TEXT, KITTY_KEYBOARD_DISAMBIGUATE,
        KITTY_KEYBOARD_REPORT_ALL, KITTY_KEYBOARD_REPORT_EVENTS, MouseInputMode, MouseProtocolMode,
        MouseReportingMode, TerminalModeChange, mouse_input_mode_allows,
    },
    visible_output::TerminalVisibleOutputFilter,
};
#[cfg(test)]
use crate::{
    terminal_modes::TerminalModeTracker,
    terminal_queries::{
        ClipboardCommand, FixedQuery, OscColorKind as SharedOscColorKind,
        OscColorRequest as SharedOscColorRequest, ScannedSegmentRef, SemanticControl,
        StringTerminator, TerminalQueryScanner, WindowReportRequest,
        XtSmGraphicsRequest as SharedXtSmGraphicsRequest,
    },
    terminal_query_dcs::{
        DcsTerminator, DecrqssKind as SharedDecrqssKind, DecrqssRequest as SharedDecrqssRequest,
        MAX_XTGETTCAP_RESPONSE_BYTES, XtGetTcapRequest as SharedXtGetTcapRequest,
    },
};

#[path = "local_terminal_runtime.rs"]
mod local_terminal_runtime;
use local_terminal_runtime::{
    LocalTerminalRuntime, SessionLogWriter, local_control_event_from_mode_change,
};

const LOCAL_CONSOLE_SESSION_ID: SessionId = SessionId::new(1);
const DEFAULT_TERMINAL_NAME: &str = "xterm-256color";
const LOCAL_WORKER_SHUTDOWN_BUDGET: Duration = Duration::from_secs(2);

static LOCAL_WORKER_REAPER: OnceLock<LocalWorkerReaper> = OnceLock::new();

#[derive(Clone, Copy)]
struct LocalPtyTrace {
    enabled: bool,
    started_at: Instant,
}

impl LocalPtyTrace {
    fn from_environment(started_at: Instant) -> Self {
        Self {
            enabled: std::env::var_os("RTERM_LOCAL_PTY_TRACE").is_some(),
            started_at,
        }
    }

    fn event(&self, message: std::fmt::Arguments<'_>) {
        if self.enabled {
            eprintln!("local-pty +{:?}: {message}", self.started_at.elapsed());
        }
    }
}

struct LocalTraceMarker {
    pattern: Vec<u8>,
    prefix: Vec<usize>,
    matched: usize,
    observed: bool,
}

impl LocalTraceMarker {
    fn from_environment() -> Option<Self> {
        let pattern = std::env::var_os("RTERM_LOCAL_PTY_TRACE_MARKER")?
            .to_string_lossy()
            .into_owned()
            .into_bytes();
        Self::new(pattern)
    }

    fn new(pattern: Vec<u8>) -> Option<Self> {
        if pattern.is_empty() {
            return None;
        }
        let mut prefix = vec![0; pattern.len()];
        let mut matched = 0;
        for index in 1..pattern.len() {
            while matched > 0 && pattern[index] != pattern[matched] {
                matched = prefix[matched - 1];
            }
            if pattern[index] == pattern[matched] {
                matched += 1;
                prefix[index] = matched;
            }
        }
        Some(Self {
            pattern,
            prefix,
            matched: 0,
            observed: false,
        })
    }

    fn feed(&mut self, bytes: &[u8]) -> bool {
        if self.observed {
            return false;
        }
        for &byte in bytes {
            while self.matched > 0 && byte != self.pattern[self.matched] {
                self.matched = self.prefix[self.matched - 1];
            }
            if byte == self.pattern[self.matched] {
                self.matched += 1;
                if self.matched == self.pattern.len() {
                    self.observed = true;
                    return true;
                }
            }
        }
        false
    }
}

enum LocalCloseProgress {
    Completed,
    Deferred,
    Failed(Box<dyn Error + Send + Sync>),
    Panicked,
    Retained,
}

trait LocalMasterCloseOperation: Send {
    fn finish_before(&mut self, deadline: Instant) -> LocalCloseProgress;
}

impl LocalMasterCloseOperation for PtyMasterClose {
    fn finish_before(&mut self, deadline: Instant) -> LocalCloseProgress {
        match PtyMasterClose::finish_before(self, deadline) {
            PtyMasterCloseStatus::Completed => LocalCloseProgress::Completed,
            PtyMasterCloseStatus::Deferred => LocalCloseProgress::Deferred,
            PtyMasterCloseStatus::Failed(error) => LocalCloseProgress::Failed(Box::new(error)),
            PtyMasterCloseStatus::Panicked => LocalCloseProgress::Panicked,
            PtyMasterCloseStatus::Retained => LocalCloseProgress::Retained,
        }
    }
}

struct LocalPtyCloseGroup {
    close: Option<Box<dyn LocalMasterCloseOperation>>,
    reader: Option<thread::JoinHandle<()>>,
    writer: Option<thread::JoinHandle<()>>,
    reader_done: mpsc::Receiver<io::Result<()>>,
    writer_done: mpsc::Receiver<io::Result<()>>,
    reader_done_observed: bool,
    writer_done_observed: bool,
}

impl LocalPtyCloseGroup {
    fn new(
        close: Box<dyn LocalMasterCloseOperation>,
        reader: thread::JoinHandle<()>,
        writer: thread::JoinHandle<()>,
        reader_done: mpsc::Receiver<io::Result<()>>,
        writer_done: mpsc::Receiver<io::Result<()>>,
        reader_done_observed: bool,
        writer_done_observed: bool,
    ) -> Self {
        Self {
            close: Some(close),
            reader: Some(reader),
            writer: Some(writer),
            reader_done,
            writer_done,
            reader_done_observed,
            writer_done_observed,
        }
    }

    fn poll_before(&mut self, deadline: Instant, errors: &mut Vec<io::Error>) -> bool {
        if let Some(close) = self.close.as_mut() {
            match close.finish_before(deadline) {
                LocalCloseProgress::Deferred => {}
                LocalCloseProgress::Completed => {
                    self.close.take();
                }
                LocalCloseProgress::Retained => {
                    errors.push(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "local PTY master close ownership was retained by the PTY global reaper",
                    ));
                    self.close.take();
                }
                LocalCloseProgress::Failed(error) => {
                    errors.push(io::Error::other(error));
                    self.close.take();
                }
                LocalCloseProgress::Panicked => {
                    errors.push(io::Error::other("local PTY master close worker panicked"));
                    self.close.take();
                }
            }
        }
        poll_local_group_worker(
            &mut self.reader,
            &self.reader_done,
            self.reader_done_observed,
            "local PTY reader",
            errors,
        );
        poll_local_group_worker(
            &mut self.writer,
            &self.writer_done,
            self.writer_done_observed,
            "local PTY writer",
            errors,
        );
        self.close.is_none() && self.reader.is_none() && self.writer.is_none()
    }
}

fn poll_local_group_worker(
    worker: &mut Option<thread::JoinHandle<()>>,
    done: &mpsc::Receiver<io::Result<()>>,
    done_already_observed: bool,
    label: &str,
    errors: &mut Vec<io::Error>,
) {
    if !worker.as_ref().is_some_and(thread::JoinHandle::is_finished) {
        return;
    }
    let worker = worker.take().expect("finished local worker remains owned");
    if worker.join().is_err() {
        errors.push(io::Error::other(format!(
            "{label} panicked while shutting down"
        )));
        return;
    }
    if done_already_observed {
        return;
    }
    match done.try_recv() {
        Ok(Ok(())) => {}
        Ok(Err(error)) => errors.push(io::Error::new(
            error.kind(),
            format!("{label} failed: {error}"),
        )),
        Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => errors.push(
            io::Error::other(format!("{label} completed without reporting its outcome")),
        ),
    }
}

enum LocalWorkerJob {
    Thread {
        worker: Option<thread::JoinHandle<()>>,
        label: String,
    },
    PtyClose(LocalPtyCloseGroup),
}

impl LocalWorkerJob {
    fn poll_before(&mut self, deadline: Instant, errors: &mut Vec<io::Error>) -> bool {
        match self {
            Self::Thread { worker, label } => {
                if !worker.as_ref().is_some_and(thread::JoinHandle::is_finished) {
                    return false;
                }
                if worker
                    .take()
                    .expect("finished local worker remains owned")
                    .join()
                    .is_err()
                {
                    errors.push(io::Error::other(format!(
                        "{label} panicked after transfer to the local worker reaper"
                    )));
                }
                true
            }
            Self::PtyClose(group) => group.poll_before(deadline, errors),
        }
    }
}

#[derive(Default)]
struct LocalWorkerReaperState {
    pending: AtomicUsize,
    fallback: Mutex<Vec<LocalWorkerJob>>,
    deferred_errors: Mutex<VecDeque<io::Error>>,
}

struct LocalWorkerReaper {
    sender: mpsc::Sender<LocalWorkerJob>,
    state: Arc<LocalWorkerReaperState>,
}

#[derive(Clone, Copy)]
enum LocalWorkerTransfer {
    ActiveSet,
    Fallback,
}

impl LocalWorkerTransfer {
    #[cfg(test)]
    fn is_fallback(self) -> bool {
        matches!(self, Self::Fallback)
    }
}

impl LocalWorkerReaperState {
    fn poll_job(&self, job: &mut LocalWorkerJob) -> bool {
        let mut errors = Vec::new();
        let finished = job.poll_before(Instant::now(), &mut errors);
        self.deferred_errors
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend(errors);
        finished
    }

    fn poll_fallback(&self) {
        let mut fallback = self
            .fallback
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut index = 0;
        while index < fallback.len() {
            if self.poll_job(&mut fallback[index]) {
                fallback.swap_remove(index);
                self.pending.fetch_sub(1, Ordering::SeqCst);
            } else {
                index += 1;
            }
        }
    }

    fn take_errors(&self) -> Vec<io::Error> {
        self.poll_fallback();
        self.deferred_errors
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .drain(..)
            .collect()
    }
}

impl LocalWorkerReaper {
    fn start(name: &str) -> Self {
        let (sender, receiver) = mpsc::channel();
        let state = Arc::new(LocalWorkerReaperState::default());
        let worker_state = Arc::clone(&state);
        let spawn_result = thread::Builder::new()
            .name(format!("rterm-local-worker-reaper-{name}"))
            .spawn(move || local_worker_reaper_loop(&receiver, &worker_state));
        if spawn_result.is_err() {
            // Dropping the receiver makes every transfer use the process-lifetime
            // fallback active set. Callers remain bounded and ownership is kept.
        }
        Self { sender, state }
    }

    #[cfg(test)]
    fn disconnected(_name: &str) -> Self {
        let (sender, receiver) = mpsc::channel();
        drop(receiver);
        Self {
            sender,
            state: Arc::new(LocalWorkerReaperState::default()),
        }
    }

    fn enqueue(&self, worker: thread::JoinHandle<()>, label: &str) -> LocalWorkerTransfer {
        self.enqueue_job(LocalWorkerJob::Thread {
            worker: Some(worker),
            label: label.to_owned(),
        })
    }

    fn enqueue_group(&self, group: LocalPtyCloseGroup) -> LocalWorkerTransfer {
        self.enqueue_job(LocalWorkerJob::PtyClose(group))
    }

    fn enqueue_job(&self, job: LocalWorkerJob) -> LocalWorkerTransfer {
        self.state.pending.fetch_add(1, Ordering::SeqCst);
        match self.sender.send(job) {
            Ok(()) => LocalWorkerTransfer::ActiveSet,
            Err(error) => {
                self.state
                    .fallback
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push(error.0);
                LocalWorkerTransfer::Fallback
            }
        }
    }

    #[cfg(test)]
    fn pending(&self) -> usize {
        self.state.poll_fallback();
        self.state.pending.load(Ordering::SeqCst)
    }

    fn take_errors(&self) -> Vec<io::Error> {
        self.state.take_errors()
    }
}

fn local_worker_reaper_loop(
    receiver: &mpsc::Receiver<LocalWorkerJob>,
    state: &LocalWorkerReaperState,
) {
    let mut active = Vec::new();
    loop {
        let disconnected = match receiver.recv_timeout(Duration::from_millis(2)) {
            Ok(job) => {
                active.push(job);
                false
            }
            Err(mpsc::RecvTimeoutError::Timeout) => false,
            Err(mpsc::RecvTimeoutError::Disconnected) => true,
        };
        active.extend(receiver.try_iter());

        let mut index = 0;
        while index < active.len() {
            if state.poll_job(&mut active[index]) {
                active.swap_remove(index);
                state.pending.fetch_sub(1, Ordering::SeqCst);
            } else {
                index += 1;
            }
        }
        if disconnected && active.is_empty() {
            return;
        }
    }
}

fn local_worker_reaper() -> &'static LocalWorkerReaper {
    LOCAL_WORKER_REAPER.get_or_init(|| LocalWorkerReaper::start("global"))
}

fn local_worker_reaper_take_errors() -> Vec<io::Error> {
    local_worker_reaper().take_errors()
}

fn join_local_worker_before(
    worker: thread::JoinHandle<()>,
    deadline: Instant,
    label: &str,
) -> io::Result<()> {
    join_local_worker_before_with_reaper(worker, deadline, label, local_worker_reaper())
}

fn join_local_worker_before_with_reaper(
    worker: thread::JoinHandle<()>,
    deadline: Instant,
    label: &str,
    reaper: &LocalWorkerReaper,
) -> io::Result<()> {
    while !worker.is_finished() && Instant::now() < deadline {
        thread::park_timeout(
            deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(2)),
        );
    }

    if worker.is_finished() {
        return worker
            .join()
            .map_err(|_| io::Error::other(format!("{label} panicked while shutting down")));
    }

    let destination = reaper.enqueue(worker, label);
    let destination = match destination {
        LocalWorkerTransfer::ActiveSet => "transferred to reaper active set",
        LocalWorkerTransfer::Fallback => "retained by fallback active set",
    };
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        format!("{label} did not stop before the deadline; {destination}"),
    ))
}

#[allow(clippy::too_many_arguments)]
fn shutdown_local_pty(
    mut session: rterm_pty::LocalPtyControl,
    reader_thread: thread::JoinHandle<()>,
    writer_thread: thread::JoinHandle<()>,
    input_thread: Option<thread::JoinHandle<()>>,
    input_stop: &AtomicBool,
    pty_input_sender: mpsc::Sender<Vec<u8>>,
    reader_done_receiver: mpsc::Receiver<io::Result<()>>,
    writer_done_receiver: mpsc::Receiver<io::Result<()>>,
    reader_done_observed: bool,
    writer_done_observed: bool,
    child_reaped: bool,
) -> Vec<io::Error> {
    let deadline = Instant::now() + LOCAL_WORKER_SHUTDOWN_BUDGET;
    let mut errors = local_worker_reaper_take_errors();

    input_stop.store(true, Ordering::Release);
    if let Some(input_thread) = input_thread
        && let Err(error) = join_local_worker_before(input_thread, deadline, "local input worker")
    {
        errors.push(error);
    }

    if !child_reaped {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if let Err(error) = session.terminate(remaining) {
            errors.push(error);
        }
    }

    // `begin_master_close` marks every writer proxy as closing before the final
    // external sender is dropped and the writer worker can exit normally.
    let master_close =
        begin_close_before_sender_drop(|| session.begin_master_close(), pty_input_sender);
    drop(session);
    let group = LocalPtyCloseGroup::new(
        Box::new(master_close),
        reader_thread,
        writer_thread,
        reader_done_receiver,
        writer_done_receiver,
        reader_done_observed,
        writer_done_observed,
    );
    finish_local_pty_close_group_before(group, deadline, local_worker_reaper(), &mut errors);

    errors.extend(local_worker_reaper_take_errors());

    errors
}

fn finish_local_pty_close_group_before(
    mut group: LocalPtyCloseGroup,
    deadline: Instant,
    reaper: &LocalWorkerReaper,
    errors: &mut Vec<io::Error>,
) {
    let mut first_poll = true;
    loop {
        let close_deadline = if first_poll { deadline } else { Instant::now() };
        first_poll = false;
        if group.poll_before(close_deadline, errors) {
            return;
        }
        if Instant::now() >= deadline {
            break;
        }
        thread::park_timeout(
            deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(2)),
        );
    }

    let destination = match reaper.enqueue_group(group) {
        LocalWorkerTransfer::ActiveSet => "transferred to reaper active set",
        LocalWorkerTransfer::Fallback => "retained by fallback active set",
    };
    errors.push(io::Error::new(
        io::ErrorKind::TimedOut,
        format!("local PTY close group did not stop before the deadline; {destination}"),
    ));
}

fn begin_close_before_sender_drop<Close, Sender>(
    begin_close: impl FnOnce() -> Close,
    sender: Sender,
) -> Close {
    let close = begin_close();
    drop(sender);
    close
}

#[derive(Debug)]
struct LocalCleanupFailures {
    errors: Vec<io::Error>,
}

impl std::fmt::Display for LocalCleanupFailures {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "local PTY cleanup failed: ")?;
        for (index, error) in self.errors.iter().enumerate() {
            if index > 0 {
                formatter.write_str("; ")?;
            }
            write!(formatter, "{error}")?;
        }
        Ok(())
    }
}

impl Error for LocalCleanupFailures {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.errors.first().map(|error| error as &dyn Error)
    }
}

#[derive(Debug)]
struct LocalPtyCompositeError {
    primary: Box<dyn Error>,
    cleanup: LocalCleanupFailures,
}

impl std::fmt::Display for LocalPtyCompositeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}; secondary cleanup failures: ", self.primary)?;
        for (index, error) in self.cleanup.errors.iter().enumerate() {
            if index > 0 {
                formatter.write_str("; ")?;
            }
            write!(formatter, "{error}")?;
        }
        Ok(())
    }
}

impl Error for LocalPtyCompositeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.primary.as_ref())
    }
}

fn combine_local_result<T>(
    primary: Result<T, Box<dyn Error>>,
    cleanup_errors: Vec<io::Error>,
) -> Result<T, Box<dyn Error>> {
    if cleanup_errors.is_empty() {
        return primary;
    }

    let cleanup = LocalCleanupFailures {
        errors: cleanup_errors,
    };
    match primary {
        Ok(_) => Err(Box::new(cleanup)),
        Err(primary) => Err(Box::new(LocalPtyCompositeError { primary, cleanup })),
    }
}

#[allow(clippy::too_many_lines)]
pub fn run(options: &LocalOptions) -> Result<PtyExitStatus, Box<dyn Error>> {
    if options.console.preflight {
        diagnostics::ensure_console_dependencies()?;
    }
    // Initialize the process-lifetime owner before spawning a child or worker,
    // so every later timeout has a guaranteed transfer destination.
    let _ = local_worker_reaper();

    let metrics_started_at = Instant::now();
    let trace = LocalPtyTrace::from_environment(metrics_started_at);
    let size = resolve_local_size(options.size);
    let mut lifecycle = SessionLifecycle::new(LOCAL_CONSOLE_SESSION_ID);
    lifecycle.start_connecting()?;
    let transport =
        rterm_pty::LocalPtyTransport::spawn(&options.command, terminal_size_from_pty(size))?;
    let parts = transport.split();
    let mut session = parts.control;
    trace.event(format_args!("spawned child pid={:?}", session.process_id()));
    lifecycle.mark_connected()?;
    let mut reader = parts.reader;
    let mut writer = parts.writer;
    let mut log_file = match &options.log {
        Some(path) => Some(File::create(path)?),
        None => None,
    };
    let (reader_done_sender, reader_done_receiver) = mpsc::channel();
    let (writer_done_sender, writer_done_receiver) = mpsc::channel();
    let (pty_input_sender, pty_input_receiver) = mpsc::channel();
    let (control_sender, control_receiver) = mpsc::channel();
    let terminal_response_sender = pty_input_sender.clone();
    let output_control_sender = control_sender.clone();
    let runtime_state = LocalRuntimeState::new(size, options.mouse, local_terminal_name(options));
    let metrics = LocalMetricsCounters::default();
    let output_context = LocalOutputWorkerContext {
        output_state: runtime_state.output_state(),
        metrics: metrics.clone(),
        osc52_policy: options.osc52_policy,
        trace,
    };
    let input_metrics = metrics.clone();

    let mut raw_mode = RawMode::enable()?;
    let reader_thread = thread::spawn(move || {
        trace.event(format_args!("reader started"));
        let result = copy_pty_output(
            &mut reader,
            &terminal_response_sender,
            &output_control_sender,
            log_file.as_mut().map(|file| file as &mut dyn Write),
            output_context,
        );
        trace.event(format_args!("reader completed result={result:?}"));
        let _ = reader_done_sender.send(result);
    });
    let writer_thread = thread::spawn(move || {
        trace.event(format_args!("writer started"));
        let result = copy_pty_input(&mut writer, &pty_input_receiver, &input_metrics, trace);
        trace.event(format_args!("writer completed result={result:?}"));
        let _ = writer_done_sender.send(result);
    });

    let (input_thread, input_stop) = spawn_input_thread(
        pty_input_sender.clone(),
        control_sender,
        runtime_state.input_reporting.clone(),
    );
    let mut reader_done_observed = false;
    let mut writer_done_observed = false;
    let run_result = run_input_loop(
        &mut session,
        &reader_done_receiver,
        &writer_done_receiver,
        &control_receiver,
        LocalInputLoopContext {
            raw_mode: &mut raw_mode,
            runtime_state: &runtime_state,
            metrics: &metrics,
            trace,
            reader_done_observed: &mut reader_done_observed,
            writer_done_observed: &mut writer_done_observed,
        },
    );

    trace.event(format_args!("cleanup started"));
    let cleanup_errors = shutdown_local_pty(
        session,
        reader_thread,
        writer_thread,
        input_thread,
        &input_stop,
        pty_input_sender,
        reader_done_receiver,
        writer_done_receiver,
        reader_done_observed,
        writer_done_observed,
        run_result.is_ok(),
    );
    trace.event(format_args!(
        "cleanup completed errors={}",
        cleanup_errors.len()
    ));
    let run_result = combine_local_result(run_result, cleanup_errors);

    if run_result.is_ok() {
        lifecycle.mark_disconnected()?;
        lifecycle.close()?;
    }

    let session_state = lifecycle.state();

    report_local_metrics(
        options,
        size,
        &metrics,
        metrics_started_at,
        session_state,
        run_result.as_ref().ok(),
    )?;

    run_result
}

fn report_local_metrics(
    options: &LocalOptions,
    size: PtySize,
    metrics: &LocalMetricsCounters,
    started_at: Instant,
    session_state: SessionState,
    status: Option<&PtyExitStatus>,
) -> Result<(), Box<dyn Error>> {
    let Some(status) = status else {
        return Ok(());
    };
    let snapshot = LocalMetricsSnapshot::from_status(
        &options.command,
        size,
        metrics.snapshot(),
        started_at.elapsed(),
        session_state,
        status,
    );
    if options.console.metrics_json {
        println!("{}", snapshot.json_report()?);
    } else if options.console.metrics {
        print!("{}", snapshot.report());
    }
    Ok(())
}

fn local_terminal_name(options: &LocalOptions) -> String {
    options
        .command
        .env_value("TERM")
        .unwrap_or(DEFAULT_TERMINAL_NAME)
        .to_owned()
}

#[derive(Clone)]
struct SharedTerminalSize {
    columns: Arc<AtomicU16>,
    rows: Arc<AtomicU16>,
}

impl SharedTerminalSize {
    fn new(size: PtySize) -> Self {
        Self {
            columns: Arc::new(AtomicU16::new(size.columns())),
            rows: Arc::new(AtomicU16::new(size.rows())),
        }
    }

    fn snapshot(&self) -> PtySize {
        PtySize::try_new(
            self.columns.load(Ordering::Relaxed),
            self.rows.load(Ordering::Relaxed),
        )
        .expect("shared terminal size remains valid")
    }

    fn set(&self, size: PtySize) {
        self.columns.store(size.columns(), Ordering::Relaxed);
        self.rows.store(size.rows(), Ordering::Relaxed);
    }
}

impl Default for SharedTerminalSize {
    fn default() -> Self {
        Self::new(fallback_pty_size())
    }
}

#[derive(Clone, Copy)]
enum LocalControlEvent {
    Resize(PtySize),
    SetApplicationCursorKeys(bool),
    SetApplicationKeypad(bool),
    SetBracketedPaste(bool),
    SetMouseReporting(MouseInputMode),
    SetFocusReporting(bool),
    SetKittyKeyboardFlags(u16),
    SetModifyOtherKeys(u8),
    SetWin32InputMode(bool),
}

#[derive(Clone, Default)]
struct InputReporting {
    application_cursor_keys: Arc<AtomicBool>,
    application_keypad: Arc<AtomicBool>,
    bracketed_paste: Arc<AtomicBool>,
    mouse: Arc<AtomicU8>,
    focus: Arc<AtomicBool>,
    kitty_keyboard_flags: Arc<AtomicU16>,
    modify_other_keys: Arc<AtomicU8>,
    win32_input_mode: Arc<AtomicBool>,
}

impl InputReporting {
    fn snapshot(&self) -> InputModes {
        InputModes::default()
            .with_application_cursor_keys(self.application_cursor_keys_enabled())
            .with_application_keypad(self.application_keypad_enabled())
            .with_bracketed_paste(self.bracketed_paste_enabled())
            .with_mouse_input_mode(self.mouse_input_mode())
            .with_focus_reporting(self.focus_enabled())
            .with_kitty_keyboard_flags(self.kitty_keyboard_flags())
            .with_modify_other_keys(self.modify_other_keys())
            .with_win32_input_mode(self.win32_input_mode())
    }

    fn application_cursor_keys_enabled(&self) -> bool {
        self.application_cursor_keys.load(Ordering::Relaxed)
    }

    fn application_keypad_enabled(&self) -> bool {
        self.application_keypad.load(Ordering::Relaxed)
    }

    fn bracketed_paste_enabled(&self) -> bool {
        self.bracketed_paste.load(Ordering::Relaxed)
    }

    fn mouse_input_mode(&self) -> MouseInputMode {
        MouseInputMode::from_bits(self.mouse.load(Ordering::Relaxed))
    }

    fn focus_enabled(&self) -> bool {
        self.focus.load(Ordering::Relaxed)
    }

    fn kitty_keyboard_flags(&self) -> u16 {
        self.kitty_keyboard_flags.load(Ordering::Relaxed)
    }

    fn modify_other_keys(&self) -> u8 {
        self.modify_other_keys.load(Ordering::Relaxed)
    }

    fn win32_input_mode(&self) -> bool {
        self.win32_input_mode.load(Ordering::Relaxed)
    }

    fn set_mouse(&self, mode: MouseInputMode) {
        self.mouse.store(mode.bits(), Ordering::Relaxed);
    }

    fn set_focus(&self, enabled: bool) {
        self.focus.store(enabled, Ordering::Relaxed);
    }

    fn set_bracketed_paste(&self, enabled: bool) {
        self.bracketed_paste.store(enabled, Ordering::Relaxed);
    }

    fn set_application_cursor_keys(&self, enabled: bool) {
        self.application_cursor_keys
            .store(enabled, Ordering::Relaxed);
    }

    fn set_application_keypad(&self, enabled: bool) {
        self.application_keypad.store(enabled, Ordering::Relaxed);
    }

    fn set_kitty_keyboard_flags(&self, flags: u16) {
        self.kitty_keyboard_flags.store(flags, Ordering::Relaxed);
    }

    fn set_modify_other_keys(&self, mode: u8) {
        self.modify_other_keys.store(mode, Ordering::Relaxed);
    }

    fn set_win32_input_mode(&self, enabled: bool) {
        self.win32_input_mode.store(enabled, Ordering::Relaxed);
    }
}

struct LocalRuntimeState {
    input_reporting: InputReporting,
    terminal_size: SharedTerminalSize,
    terminal_name: String,
    allow_application_reporting: bool,
}

impl LocalRuntimeState {
    fn new(size: PtySize, allow_application_reporting: bool, terminal_name: String) -> Self {
        Self {
            input_reporting: InputReporting::default(),
            terminal_size: SharedTerminalSize::new(size),
            terminal_name,
            allow_application_reporting,
        }
    }

    fn output_state(&self) -> LocalOutputState {
        LocalOutputState {
            terminal_size: self.terminal_size.clone(),
            terminal_name: self.terminal_name.clone(),
        }
    }
}

struct LocalOutputState {
    terminal_size: SharedTerminalSize,
    terminal_name: String,
}

#[derive(Clone, Default)]
struct LocalMetricsCounters {
    pty_input_bytes: Arc<AtomicU64>,
    pty_output_bytes: Arc<AtomicU64>,
    terminal_output_bytes: Arc<AtomicU64>,
    resize_events: Arc<AtomicU64>,
}

impl LocalMetricsCounters {
    fn add_pty_input(&self, bytes: u64) {
        self.pty_input_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    fn add_pty_output(&self, bytes: u64) {
        self.pty_output_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    fn add_terminal_output(&self, bytes: u64) {
        self.terminal_output_bytes
            .fetch_add(bytes, Ordering::Relaxed);
    }

    fn add_resize_event(&self) {
        self.resize_events.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> LocalMetricsCountersSnapshot {
        LocalMetricsCountersSnapshot {
            pty_input_bytes: self.pty_input_bytes.load(Ordering::Relaxed),
            pty_output_bytes: self.pty_output_bytes.load(Ordering::Relaxed),
            terminal_output_bytes: self.terminal_output_bytes.load(Ordering::Relaxed),
            resize_events: self.resize_events.load(Ordering::Relaxed),
        }
    }
}

#[derive(Clone, Copy)]
struct LocalMetricsCountersSnapshot {
    pty_input_bytes: u64,
    pty_output_bytes: u64,
    terminal_output_bytes: u64,
    resize_events: u64,
}

#[derive(Serialize)]
struct LocalMetricsSnapshot {
    command: String,
    backend: String,
    columns: u16,
    rows: u16,
    session_state: String,
    pty_input_bytes: u64,
    pty_output_bytes: u64,
    terminal_output_bytes: u64,
    resize_events: u64,
    elapsed_ms: u128,
    exit_code: u32,
    signal: Option<String>,
    success: bool,
}

impl LocalMetricsSnapshot {
    fn from_status(
        command: &rterm_pty::PtyCommand,
        size: PtySize,
        counters: LocalMetricsCountersSnapshot,
        elapsed: Duration,
        session_state: SessionState,
        status: &PtyExitStatus,
    ) -> Self {
        Self {
            command: command_line(command),
            backend: format!("{:?}", PtyBackend::current_platform()),
            columns: size.columns(),
            rows: size.rows(),
            session_state: session_state.as_str().to_owned(),
            pty_input_bytes: counters.pty_input_bytes,
            pty_output_bytes: counters.pty_output_bytes,
            terminal_output_bytes: counters.terminal_output_bytes,
            resize_events: counters.resize_events,
            elapsed_ms: elapsed.as_millis(),
            exit_code: status.exit_code(),
            signal: status.signal().map(str::to_owned),
            success: status.success(),
        }
    }

    fn report(&self) -> String {
        format!(
            "\
R-Term console metrics
command={}
backend={}
columns={}
rows={}
session_state={}
pty_input_bytes={}
pty_output_bytes={}
terminal_output_bytes={}
resize_events={}
elapsed_ms={}
exit_code={}
signal={}
success={}
",
            self.command,
            self.backend,
            self.columns,
            self.rows,
            self.session_state,
            self.pty_input_bytes,
            self.pty_output_bytes,
            self.terminal_output_bytes,
            self.resize_events,
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

fn command_line(command: &rterm_pty::PtyCommand) -> String {
    std::iter::once(command.program())
        .chain(command.args().iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod metrics_tests {
    use rterm_core::session::SessionState;
    use rterm_pty::{PtyBackend, PtyExitStatus};

    #[test]
    fn console_metrics_report_includes_command_timing_and_exit_status() {
        let command = rterm_pty::PtyCommand::new("cmd.exe").with_args(["/C", "echo hi"]);
        let report = super::LocalMetricsSnapshot::from_status(
            &command,
            rterm_pty::PtySize::try_new(100, 30).unwrap(),
            super::LocalMetricsCountersSnapshot {
                pty_input_bytes: 3,
                pty_output_bytes: 8,
                terminal_output_bytes: 5,
                resize_events: 2,
            },
            std::time::Duration::from_millis(42),
            SessionState::Closed,
            &PtyExitStatus::from_exit_code(0),
        )
        .report();

        let expected_backend = format!("{:?}", PtyBackend::current_platform());

        assert_eq!(
            report,
            format!(
                "R-Term console metrics\n\
command=cmd.exe /C echo hi\n\
backend={expected_backend}\n\
columns=100\n\
rows=30\n\
session_state=closed\n\
pty_input_bytes=3\n\
pty_output_bytes=8\n\
terminal_output_bytes=5\n\
resize_events=2\n\
elapsed_ms=42\n\
exit_code=0\n\
signal=none\n\
success=true\n"
            )
        );
    }

    #[test]
    fn console_metrics_json_report_is_machine_readable() {
        let command = rterm_pty::PtyCommand::new("cmd.exe").with_args(["/C", "echo hi"]);
        let report = super::LocalMetricsSnapshot::from_status(
            &command,
            rterm_pty::PtySize::try_new(100, 30).unwrap(),
            super::LocalMetricsCountersSnapshot {
                pty_input_bytes: 3,
                pty_output_bytes: 8,
                terminal_output_bytes: 5,
                resize_events: 2,
            },
            std::time::Duration::from_millis(42),
            SessionState::Closed,
            &PtyExitStatus::from_exit_code(0),
        )
        .json_report()
        .unwrap();

        let expected_backend = format!("{:?}", PtyBackend::current_platform());

        assert_eq!(
            report,
            format!(
                "{{\"command\":\"cmd.exe /C echo hi\",\"backend\":\"{expected_backend}\",\"columns\":100,\"rows\":30,\"session_state\":\"closed\",\"pty_input_bytes\":3,\"pty_output_bytes\":8,\"terminal_output_bytes\":5,\"resize_events\":2,\"elapsed_ms\":42,\"exit_code\":0,\"signal\":null,\"success\":true}}"
            )
        );
    }
}

fn spawn_input_thread(
    pty_input_sender: mpsc::Sender<Vec<u8>>,
    control_sender: mpsc::Sender<LocalControlEvent>,
    input_reporting: InputReporting,
) -> (Option<thread::JoinHandle<()>>, Arc<AtomicBool>) {
    let stop = Arc::new(AtomicBool::new(false));
    spawn_input_thread_for_terminal(
        io::stdin().is_terminal(),
        pty_input_sender,
        control_sender,
        input_reporting,
        stop,
    )
}

fn spawn_input_thread_for_terminal(
    is_terminal: bool,
    pty_input_sender: mpsc::Sender<Vec<u8>>,
    control_sender: mpsc::Sender<LocalControlEvent>,
    input_reporting: InputReporting,
    stop: Arc<AtomicBool>,
) -> (Option<thread::JoinHandle<()>>, Arc<AtomicBool>) {
    if !is_terminal {
        return (None, stop);
    }

    let worker_stop = Arc::clone(&stop);
    let worker = thread::spawn(move || {
        while !worker_stop.load(Ordering::Acquire) {
            match event::poll(Duration::from_millis(20)) {
                Ok(false) => continue,
                Err(_) => return,
                Ok(true) => {}
            }

            match event::read() {
                Ok(
                    event @ (Event::Key(_)
                    | Event::Paste(_)
                    | Event::Mouse(_)
                    | Event::FocusGained
                    | Event::FocusLost),
                ) => {
                    let Some(bytes) = encode_input_event(event, input_reporting.snapshot()) else {
                        continue;
                    };
                    if pty_input_sender.send(bytes).is_err() {
                        return;
                    }
                }
                Ok(Event::Resize(columns, rows)) => {
                    let Ok(size) = PtySize::try_new(columns, rows) else {
                        continue;
                    };
                    if control_sender
                        .send(LocalControlEvent::Resize(size))
                        .is_err()
                    {
                        return;
                    }
                }
                Err(_) => return,
            }
        }
    });
    (Some(worker), stop)
}

fn resolve_local_size(explicit: Option<PtySize>) -> PtySize {
    if let Some(size) = explicit {
        return size;
    }

    terminal::size()
        .ok()
        .and_then(|(columns, rows)| PtySize::try_new(columns, rows).ok())
        .unwrap_or_else(fallback_pty_size)
}

fn fallback_pty_size() -> PtySize {
    PtySize::try_new(80, 24).expect("fallback PTY size is valid")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RawModeState {
    Disabled,
    Enabled,
}

struct RawMode {
    state: RawModeState,
    bracketed_paste: bool,
    mouse_capture: bool,
    focus_change: bool,
}

impl RawMode {
    fn enable() -> io::Result<Self> {
        Self::enable_for_terminal(io::stdin().is_terminal())
    }

    fn enable_for_terminal(is_terminal: bool) -> io::Result<Self> {
        if !is_terminal {
            return Ok(Self {
                state: RawModeState::Disabled,
                bracketed_paste: false,
                mouse_capture: false,
                focus_change: false,
            });
        }

        terminal::enable_raw_mode()?;

        let bracketed_paste = if io::stdout().is_terminal() {
            let mut stdout = io::stdout();
            execute!(stdout, EnableBracketedPaste).is_ok()
        } else {
            false
        };

        Ok(Self {
            state: RawModeState::Enabled,
            bracketed_paste,
            mouse_capture: false,
            focus_change: false,
        })
    }

    fn set_mouse_capture(&mut self, enabled: bool) -> io::Result<bool> {
        if enabled == self.mouse_capture {
            return Ok(self.mouse_capture);
        }

        if enabled {
            if !io::stdout().is_terminal() {
                return Ok(false);
            }
            let mut stdout = io::stdout();
            execute!(stdout, EnableMouseCapture)?;
            self.mouse_capture = true;
        } else {
            let mut stdout = io::stdout();
            execute!(stdout, DisableMouseCapture)?;
            self.mouse_capture = false;
        }

        Ok(self.mouse_capture)
    }

    fn set_focus_change(&mut self, enabled: bool) -> io::Result<bool> {
        if enabled == self.focus_change {
            return Ok(self.focus_change);
        }

        if enabled {
            if !io::stdout().is_terminal() {
                return Ok(false);
            }
            let mut stdout = io::stdout();
            execute!(stdout, EnableFocusChange)?;
            self.focus_change = true;
        } else {
            let mut stdout = io::stdout();
            execute!(stdout, DisableFocusChange)?;
            self.focus_change = false;
        }

        Ok(self.focus_change)
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        if self.focus_change {
            let mut stdout = io::stdout();
            let _ = execute!(stdout, DisableFocusChange);
        }
        if self.mouse_capture {
            let mut stdout = io::stdout();
            let _ = execute!(stdout, DisableMouseCapture);
        }
        if self.bracketed_paste {
            let mut stdout = io::stdout();
            let _ = execute!(stdout, DisableBracketedPaste);
        }
        if self.state == RawModeState::Enabled {
            let _ = terminal::disable_raw_mode();
        }
    }
}

struct LocalOutputWorkerContext {
    output_state: LocalOutputState,
    metrics: LocalMetricsCounters,
    osc52_policy: Osc52Policy,
    trace: LocalPtyTrace,
}

fn copy_pty_output(
    reader: &mut dyn Read,
    pty_input_sender: &mpsc::Sender<Vec<u8>>,
    control_sender: &mpsc::Sender<LocalControlEvent>,
    log: Option<&mut dyn Write>,
    context: LocalOutputWorkerContext,
) -> io::Result<()> {
    let LocalOutputWorkerContext {
        output_state,
        metrics,
        osc52_policy,
        trace,
    } = context;
    let mut stdout = io::stdout().lock();
    let mut output = SessionLogWriter::new(&mut stdout, log, metrics.clone());
    let mut buffer = [0; 8192];
    let mut terminal_runtime = LocalTerminalRuntime::new(
        output_state.terminal_size,
        output_state.terminal_name,
        osc52_policy,
    );
    let mut first_output_seen = false;
    let mut trace_marker = LocalTraceMarker::from_environment();
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => {
                terminal_runtime.finish(
                    &mut output,
                    |response| {
                        trace.event(format_args!("queued terminal response {response:?}"));
                        pty_input_sender.send(response.to_vec()).map_err(|_| {
                            io::Error::new(io::ErrorKind::BrokenPipe, "PTY input closed")
                        })
                    },
                    write_local_clipboard_text,
                    read_local_clipboard_text,
                    |change| {
                        if let Some(event) = local_control_event_from_mode_change(change) {
                            let _ = control_sender.send(event);
                        }
                    },
                )?;
                output.flush()?;
                return Ok(());
            }
            Ok(count) => {
                if !first_output_seen {
                    trace.event(format_args!("first PTY output"));
                    first_output_seen = true;
                }
                if trace_marker
                    .as_mut()
                    .is_some_and(|marker| marker.feed(&buffer[..count]))
                {
                    trace.event(format_args!("trace marker observed"));
                }
                metrics.add_pty_output(count as u64);
                terminal_runtime.write_with_clipboard(
                    &buffer[..count],
                    &mut output,
                    |response| {
                        trace.event(format_args!("queued terminal response {response:?}"));
                        pty_input_sender.send(response.to_vec()).map_err(|_| {
                            io::Error::new(io::ErrorKind::BrokenPipe, "PTY input closed")
                        })
                    },
                    write_local_clipboard_text,
                    read_local_clipboard_text,
                    |change| {
                        if let Some(event) = local_control_event_from_mode_change(change) {
                            let _ = control_sender.send(event);
                        }
                    },
                )?;
                output.flush()?;
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}

fn copy_pty_input(
    writer: &mut dyn Write,
    pty_input_receiver: &mpsc::Receiver<Vec<u8>>,
    metrics: &LocalMetricsCounters,
    trace: LocalPtyTrace,
) -> io::Result<()> {
    for bytes in pty_input_receiver {
        trace.event(format_args!(
            "writing {} PTY input bytes {:?}",
            bytes.len(),
            bytes
        ));
        writer.write_all(&bytes)?;
        metrics.add_pty_input(bytes.len() as u64);
        writer.flush()?;
    }

    Ok(())
}

struct LocalInputLoopContext<'a> {
    raw_mode: &'a mut RawMode,
    runtime_state: &'a LocalRuntimeState,
    metrics: &'a LocalMetricsCounters,
    trace: LocalPtyTrace,
    reader_done_observed: &'a mut bool,
    writer_done_observed: &'a mut bool,
}

fn run_input_loop(
    session: &mut rterm_pty::LocalPtyControl,
    reader_done_receiver: &mpsc::Receiver<io::Result<()>>,
    writer_done_receiver: &mpsc::Receiver<io::Result<()>>,
    control_receiver: &mpsc::Receiver<LocalControlEvent>,
    context: LocalInputLoopContext<'_>,
) -> Result<PtyExitStatus, Box<dyn Error>> {
    let LocalInputLoopContext {
        raw_mode,
        runtime_state,
        metrics,
        trace,
        reader_done_observed,
        writer_done_observed,
    } = context;
    let mut exited_status: Option<PtyExitStatus> = None;
    let mut completion_deadline: Option<Instant> = None;
    let mut child_wait_logged = false;

    loop {
        if let Ok(reader_result) = reader_done_receiver.try_recv() {
            *reader_done_observed = true;
            reader_result?;
            completion_deadline
                .get_or_insert_with(|| Instant::now() + LOCAL_WORKER_SHUTDOWN_BUDGET);
        }

        if let Ok(writer_result) = writer_done_receiver.try_recv() {
            *writer_done_observed = true;
            writer_result?;
        }

        while let Ok(control_event) = control_receiver.try_recv() {
            apply_local_control_event(control_event, session, raw_mode, runtime_state, metrics)?;
        }

        if exited_status.is_none() {
            match session.try_wait_pty()? {
                Some(status) => {
                    trace.event(format_args!("child reaped status={status:?}"));
                    exited_status = Some(status);
                    completion_deadline
                        .get_or_insert_with(|| Instant::now() + LOCAL_WORKER_SHUTDOWN_BUDGET);
                }
                None if trace.enabled && !child_wait_logged => {
                    trace.event(format_args!("child still running"));
                    child_wait_logged = true;
                }
                None => {}
            }
        }

        // The retained PTY master can keep the reader open even after the child
        // has exited. Hand the reaped status to the ordered cleanup path, which
        // closes the master before joining the reader and writer.
        if let Some(status) = exited_status.take() {
            return Ok(status);
        }

        if completion_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "local PTY child and reader did not finish within the shared completion budget",
            )
            .into());
        }

        match control_receiver.recv_timeout(Duration::from_millis(10)) {
            Ok(event) => {
                apply_local_control_event(event, session, raw_mode, runtime_state, metrics)?;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => thread::yield_now(),
        }
    }
}

fn apply_local_control_event(
    control_event: LocalControlEvent,
    session: &mut rterm_pty::LocalPtyControl,
    raw_mode: &mut RawMode,
    runtime_state: &LocalRuntimeState,
    metrics: &LocalMetricsCounters,
) -> Result<(), Box<dyn Error>> {
    match control_event {
        LocalControlEvent::Resize(size) => {
            session.resize_pty(size)?;
            runtime_state.terminal_size.set(size);
            metrics.add_resize_event();
        }
        LocalControlEvent::SetApplicationCursorKeys(enabled) => runtime_state
            .input_reporting
            .set_application_cursor_keys(enabled),
        LocalControlEvent::SetApplicationKeypad(enabled) => runtime_state
            .input_reporting
            .set_application_keypad(enabled),
        LocalControlEvent::SetBracketedPaste(enabled) => {
            runtime_state.input_reporting.set_bracketed_paste(enabled);
        }
        LocalControlEvent::SetMouseReporting(mode) => {
            let mode = if runtime_state.allow_application_reporting
                && raw_mode.set_mouse_capture(mode.reporting_enabled())?
            {
                mode
            } else {
                mode.with_reporting(MouseReportingMode::None)
            };
            runtime_state.input_reporting.set_mouse(mode);
        }
        LocalControlEvent::SetFocusReporting(enabled) => {
            let enabled = if runtime_state.allow_application_reporting {
                raw_mode.set_focus_change(enabled)?
            } else {
                false
            };
            runtime_state.input_reporting.set_focus(enabled);
        }
        LocalControlEvent::SetKittyKeyboardFlags(flags) => runtime_state
            .input_reporting
            .set_kitty_keyboard_flags(flags),
        LocalControlEvent::SetModifyOtherKeys(mode) => {
            runtime_state.input_reporting.set_modify_other_keys(mode);
        }
        LocalControlEvent::SetWin32InputMode(enabled) => {
            runtime_state.input_reporting.set_win32_input_mode(enabled);
        }
    }
    Ok(())
}

fn encode_osc52_clipboard_response(selection: &str, text: &str) -> Vec<u8> {
    format!(
        "\x1b]52;{};{}\x07",
        selection,
        STANDARD.encode(text.as_bytes())
    )
    .into_bytes()
}

fn read_local_clipboard_text() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

fn write_local_clipboard_text(text: &str) -> bool {
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.set_text(text.to_owned()))
        .is_ok()
}

fn terminal_size_from_pty(size: PtySize) -> TerminalSize {
    TerminalSize::new(size.columns(), size.rows())
}

#[cfg(test)]
mod legacy_terminal_output {
    use super::*;

    pub(super) struct LegacyTerminalOutputFilter {
        query_scanner: TerminalQueryScanner,
        synchronized_output_buffer: Vec<u8>,
        size: SharedTerminalSize,
        pub(super) mirror: Terminal,
        mirror_size: PtySize,
        pub(super) mode_tracker: TerminalModeTracker,
        mode_changes: Vec<TerminalModeChange>,
        color_state: TerminalColorState,
        terminal_name: String,
    }

    impl LegacyTerminalOutputFilter {
        const CELL_HEIGHT_PIXELS: u16 = 16;
        const CELL_WIDTH_PIXELS: u16 = 8;
        const PRIMARY_DEVICE_ATTRIBUTES: &'static [u8] = b"\x1b[?65;4;6;18;22;52c";
        const SECONDARY_DEVICE_ATTRIBUTES: &'static [u8] = b"\x1b[>1;277;0c";
        const TERTIARY_DEVICE_ATTRIBUTES: &'static [u8] = b"\x1bP!|00000000\x1b\\";
        const TERMINAL_PARAMETERS_0: &'static [u8] = b"\x1b[2;1;1;128;128;1;0x";
        const TERMINAL_PARAMETERS_1: &'static [u8] = b"\x1b[3;1;1;128;128;1;0x";

        #[cfg(test)]
        pub(super) fn new(size: PtySize) -> Self {
            Self::with_shared_size(SharedTerminalSize::new(size))
        }

        fn with_shared_size(size: SharedTerminalSize) -> Self {
            let mirror_size = size.snapshot();
            Self {
                query_scanner: TerminalQueryScanner::new(),
                synchronized_output_buffer: Vec::new(),
                size,
                mirror: Terminal::new(terminal_size_from_pty(mirror_size)),
                mirror_size,
                mode_tracker: TerminalModeTracker::default(),
                mode_changes: Vec::new(),
                color_state: TerminalColorState::default(),
                terminal_name: DEFAULT_TERMINAL_NAME.to_owned(),
            }
        }

        pub(super) fn set_terminal_name(&mut self, terminal_name: impl Into<String>) {
            self.terminal_name = terminal_name.into();
        }

        #[cfg(test)]
        pub(super) fn write(
            &mut self,
            bytes: &[u8],
            output: &mut dyn Write,
            respond: impl FnMut(&[u8]) -> io::Result<()>,
        ) -> io::Result<()> {
            self.write_with_clipboard(bytes, output, respond, |_| false, || None, Osc52Policy::Off)
        }

        pub(super) fn write_with_clipboard(
            &mut self,
            bytes: &[u8],
            output: &mut dyn Write,
            mut respond: impl FnMut(&[u8]) -> io::Result<()>,
            mut write_clipboard: impl FnMut(&str) -> bool,
            mut read_clipboard: impl FnMut() -> Option<String>,
            osc52_policy: Osc52Policy,
        ) -> io::Result<()> {
            let mut scanner = std::mem::take(&mut self.query_scanner);
            let mut result = Ok(());
            scanner.for_each_segment(bytes, |segment| {
                if result.is_err() {
                    return;
                }
                result = (|| -> io::Result<()> {
                    match segment {
                        ScannedSegmentRef::Bytes(visible) => {
                            self.write_visible_bytes_and_update_state(visible, output)?;
                        }
                        ScannedSegmentRef::Control {
                            bytes: sequence,
                            semantic,
                            ..
                        } => {
                            let response = match &semantic {
                                SemanticControl::Fixed(query) => Some(Self::fixed_response(*query)),
                                SemanticControl::WindowReport(query) => {
                                    Self::window_response(*query)
                                }
                                SemanticControl::PrivateModeStatus(mode) => {
                                    Some(TerminalResponse::PrivateModeStatus(*mode))
                                }
                                SemanticControl::AnsiModeStatus(mode) => {
                                    Some(TerminalResponse::AnsiModeStatus(*mode))
                                }
                                SemanticControl::OscColor(query) => {
                                    Some(TerminalResponse::OscColor(Self::osc_color_response(
                                        query.clone(),
                                    )))
                                }
                                SemanticControl::ItermReportCellSize => {
                                    Some(TerminalResponse::ItermReportCellSize)
                                }
                                SemanticControl::Decrqss(request) => Some(
                                    TerminalResponse::Decrqss(Self::decrqss_response(*request)),
                                ),
                                SemanticControl::XtGetTcap(request) => Some(
                                    TerminalResponse::XtGetTcap(self.xtgettcap_response(request)),
                                ),
                                SemanticControl::XtSmGraphics(request) => {
                                    Some(TerminalResponse::XtSmGraphics(
                                        Self::xtsmgraphics_request(*request),
                                    ))
                                }
                                SemanticControl::KittyKeyboardFlags => {
                                    Some(TerminalResponse::KittyKeyboardFlags)
                                }
                                SemanticControl::KeyModifierOptionsQuery(resource) => {
                                    Some(TerminalResponse::KeyModifierOptions(*resource))
                                }
                                SemanticControl::Osc52(ClipboardCommand::Write {
                                    contents,
                                    ..
                                }) => Some(TerminalResponse::Osc52Write(contents.clone())),
                                SemanticControl::Osc52(ClipboardCommand::Query(selection)) => {
                                    Some(TerminalResponse::Osc52Query(selection.clone()))
                                }
                                SemanticControl::Osc8Hyperlink => {
                                    Some(TerminalResponse::Osc8Hyperlink)
                                }
                                _ => None,
                            };
                            if let Some(response) = response {
                                match response {
                                    TerminalResponse::Osc8Hyperlink => {
                                        self.feed_mirror_bytes(sequence);
                                    }
                                    TerminalResponse::Osc52Write(text) => {
                                        if osc52_policy.allows_write() {
                                            let _ = write_clipboard(&text);
                                        }
                                    }
                                    TerminalResponse::Osc52Query(selection) => {
                                        if osc52_policy.allows_query()
                                            && let Some(text) = read_clipboard()
                                        {
                                            let response_bytes =
                                                encode_osc52_clipboard_response(&selection, &text);
                                            respond(&response_bytes)?;
                                        }
                                    }
                                    response => respond(&self.response_bytes(response))?,
                                }
                            } else {
                                self.write_unanswered_control(semantic, sequence, output)?;
                            }
                        }
                    }
                    Ok(())
                })();
            });
            self.query_scanner = scanner;
            result
        }

        fn write_unanswered_control(
            &mut self,
            semantic: SemanticControl,
            sequence: &[u8],
            output: &mut dyn Write,
        ) -> io::Result<()> {
            match semantic {
                SemanticControl::SynchronizedOutputMode(mode_sequence) => {
                    let enabled = mode_sequence.enabled;
                    self.mode_tracker
                        .apply_private_mode_sequence(&mode_sequence, |change| {
                            self.mode_changes.push(change);
                        });
                    self.feed_mirror_bytes(sequence);
                    if !enabled {
                        self.flush_synchronized_output_buffer(output)?;
                    }
                }
                SemanticControl::KittyKeyboardMode(mode_sequence) => {
                    self.mode_tracker
                        .apply_kitty_keyboard_sequence(mode_sequence, |change| {
                            self.mode_changes.push(change);
                        });
                }
                SemanticControl::KeyModifierOptionsSequence(mode_sequence) => {
                    self.mode_tracker.apply_key_modifier_options_sequence(
                        mode_sequence,
                        |change| {
                            self.mode_changes.push(change);
                        },
                    );
                }
                SemanticControl::Decrqcra(_)
                | SemanticControl::WindowReport(_)
                | SemanticControl::Notification(_)
                | SemanticControl::DeviceAttributesResponse
                | SemanticControl::StandaloneSt
                | SemanticControl::Cancelled
                | SemanticControl::Ignored => {}
                _ => self.write_visible_bytes_and_update_state(sequence, output)?,
            }
            Ok(())
        }

        fn fixed_response(query: FixedQuery) -> TerminalResponse {
            match query {
                FixedQuery::CursorPosition => TerminalResponse::CursorPosition { private: false },
                FixedQuery::PrimaryDeviceAttributes => {
                    TerminalResponse::Static(Self::PRIMARY_DEVICE_ATTRIBUTES)
                }
                FixedQuery::SecondaryDeviceAttributes => {
                    TerminalResponse::Static(Self::SECONDARY_DEVICE_ATTRIBUTES)
                }
                FixedQuery::TertiaryDeviceAttributes => {
                    TerminalResponse::Static(Self::TERTIARY_DEVICE_ATTRIBUTES)
                }
                FixedQuery::TerminalParameters0 => {
                    TerminalResponse::Static(Self::TERMINAL_PARAMETERS_0)
                }
                FixedQuery::TerminalParameters1 => {
                    TerminalResponse::Static(Self::TERMINAL_PARAMETERS_1)
                }
                FixedQuery::XtVersion => TerminalResponse::XtVersion,
                FixedQuery::OperatingStatus => TerminalResponse::Static(b"\x1b[0n"),
                FixedQuery::WindowPixelSize => TerminalResponse::WindowPixelSize,
                FixedQuery::CharacterCellSize => TerminalResponse::CharacterCellSize,
                FixedQuery::TextAreaSize => TerminalResponse::TextAreaSize,
            }
        }

        fn window_response(query: WindowReportRequest) -> Option<TerminalResponse> {
            match query {
                WindowReportRequest::WindowPixelSize => Some(TerminalResponse::WindowPixelSize),
                WindowReportRequest::CharacterCellSize => Some(TerminalResponse::CharacterCellSize),
                WindowReportRequest::TextAreaSize => Some(TerminalResponse::TextAreaSize),
                WindowReportRequest::WindowTitle | WindowReportRequest::Ignored => None,
            }
        }

        fn osc_color_response(query: SharedOscColorRequest) -> OscColorResponse {
            OscColorResponse {
                kinds: query
                    .kinds
                    .into_iter()
                    .map(|kind| match kind {
                        SharedOscColorKind::DefaultForeground => OscColorKind::DefaultForeground,
                        SharedOscColorKind::DefaultBackground => OscColorKind::DefaultBackground,
                        SharedOscColorKind::Cursor => OscColorKind::Cursor,
                        SharedOscColorKind::Palette(index) => OscColorKind::Palette(index),
                    })
                    .collect(),
                terminator: match query.terminator {
                    StringTerminator::Bel => OscResponseTerminator::Bel,
                    StringTerminator::St => OscResponseTerminator::St,
                    StringTerminator::C1St => OscResponseTerminator::C1St,
                },
            }
        }

        fn xtsmgraphics_request(request: SharedXtSmGraphicsRequest) -> XtSmGraphicsRequest {
            XtSmGraphicsRequest {
                item: request.item,
                action: request.action,
            }
        }

        fn decrqss_response(request: SharedDecrqssRequest) -> DecrqssResponse {
            DecrqssResponse {
                kind: match request.kind {
                    SharedDecrqssKind::Sgr => Some(DecrqssKind::Sgr),
                    SharedDecrqssKind::CursorShape => Some(DecrqssKind::CursorShape),
                    SharedDecrqssKind::ScrollRegion => Some(DecrqssKind::ScrollRegion),
                    SharedDecrqssKind::ConformanceLevel => Some(DecrqssKind::ConformanceLevel),
                    SharedDecrqssKind::LeftRightMargins => Some(DecrqssKind::LeftRightMargins),
                    SharedDecrqssKind::Unknown => None,
                },
                terminator: match request.terminator {
                    DcsTerminator::SevenBit => OscResponseTerminator::St,
                    DcsTerminator::EightBit => OscResponseTerminator::C1St,
                },
            }
        }

        fn xtgettcap_response(&self, request: &SharedXtGetTcapRequest) -> XtGetTcapResponse {
            let size = self.size.snapshot();
            let entries = request
                .names
                .iter()
                .map(|requested| {
                    let name = requested.decoded.as_deref().unwrap_or(&requested.encoded);
                    let name = String::from_utf8_lossy(name).into_owned().into_bytes();
                    XtGetTcapEntry {
                        name_hex: encode_ascii_hex(&name),
                        value_hex: xtgettcap_value_hex(&name, size, &self.terminal_name),
                    }
                })
                .collect();
            XtGetTcapResponse { entries }
        }

        fn write_visible_bytes(&mut self, bytes: &[u8], output: &mut dyn Write) -> io::Result<()> {
            if self.mode_tracker.synchronized_output() {
                self.synchronized_output_buffer.extend_from_slice(bytes);
            } else {
                output.write_all(bytes)?;
            }
            Ok(())
        }

        fn flush_synchronized_output_buffer(&mut self, output: &mut dyn Write) -> io::Result<()> {
            if self.synchronized_output_buffer.is_empty() {
                return Ok(());
            }

            output.write_all(&self.synchronized_output_buffer)?;
            self.synchronized_output_buffer.clear();
            Ok(())
        }

        fn write_visible_bytes_and_update_state(
            &mut self,
            bytes: &[u8],
            output: &mut dyn Write,
        ) -> io::Result<()> {
            let was_synchronized = self.mode_tracker.synchronized_output();
            self.write_visible_bytes(bytes, output)?;
            self.color_state.process(bytes);
            self.mode_tracker
                .process(bytes, |change| self.mode_changes.push(change));
            if was_synchronized && !self.mode_tracker.synchronized_output() {
                self.flush_synchronized_output_buffer(output)?;
            }
            self.feed_mirror_bytes(bytes);
            Ok(())
        }

        pub(super) fn flush(&mut self, output: &mut dyn Write) -> io::Result<()> {
            self.query_scanner.discard_incomplete();
            self.flush_synchronized_output_buffer(output)?;
            Ok(())
        }

        fn feed_mirror_bytes(&mut self, bytes: &[u8]) {
            self.sync_mirror_size();
            self.mirror.feed(bytes);
        }

        fn response_bytes(&mut self, response: TerminalResponse) -> Vec<u8> {
            match response {
                TerminalResponse::Static(bytes) => bytes.to_vec(),
                TerminalResponse::CursorPosition { private } => {
                    self.sync_mirror_size();
                    let (row, column) = self.mirror.cursor();
                    if private {
                        format!(
                            "\x1b[?{};{}R",
                            row.saturating_add(1),
                            column.saturating_add(1)
                        )
                        .into_bytes()
                    } else {
                        format!(
                            "\x1b[{};{}R",
                            row.saturating_add(1),
                            column.saturating_add(1)
                        )
                        .into_bytes()
                    }
                }
                TerminalResponse::WindowPixelSize => {
                    let size = self.size.snapshot();
                    format!(
                        "\x1b[4;{};{}t",
                        u32::from(size.rows()) * u32::from(Self::CELL_HEIGHT_PIXELS),
                        u32::from(size.columns()) * u32::from(Self::CELL_WIDTH_PIXELS)
                    )
                    .into_bytes()
                }
                TerminalResponse::CharacterCellSize => format!(
                    "\x1b[6;{};{}t",
                    Self::CELL_HEIGHT_PIXELS,
                    Self::CELL_WIDTH_PIXELS
                )
                .into_bytes(),
                TerminalResponse::TextAreaSize => {
                    let size = self.size.snapshot();
                    format!("\x1b[8;{};{}t", size.rows(), size.columns()).into_bytes()
                }
                TerminalResponse::PrivateModeStatus(mode) => format!(
                    "\x1b[?{};{}$y",
                    mode,
                    self.mode_tracker.private_mode_report_value(mode)
                )
                .into_bytes(),
                TerminalResponse::AnsiModeStatus(mode) => format!(
                    "\x1b[{};{}$y",
                    mode,
                    self.mode_tracker.ansi_mode_report_value(mode)
                )
                .into_bytes(),
                TerminalResponse::OscColor(query) => self.color_state.response(query),
                TerminalResponse::ItermReportCellSize => format!(
                    "\x1b]1337;ReportCellSize={:.1};{:.1}\x1b\\",
                    f32::from(Self::CELL_HEIGHT_PIXELS),
                    f32::from(Self::CELL_WIDTH_PIXELS)
                )
                .into_bytes(),
                TerminalResponse::Decrqss(query) => query.response(&self.mirror),
                TerminalResponse::XtGetTcap(query) => query.response(),
                TerminalResponse::XtSmGraphics(request) => request.response(self.size.snapshot()),
                TerminalResponse::XtVersion => xtversion_response(),
                TerminalResponse::KittyKeyboardFlags => {
                    format!("\x1b[?{}u", self.mode_tracker.kitty_keyboard_flags()).into_bytes()
                }
                TerminalResponse::KeyModifierOptions(resource) => {
                    let value = if resource == 4 {
                        self.mode_tracker.modify_other_keys()
                    } else {
                        0
                    };
                    format!("\x1b[>{resource};{value}m").into_bytes()
                }
                TerminalResponse::Osc8Hyperlink
                | TerminalResponse::Osc52Write(_)
                | TerminalResponse::Osc52Query(_) => Vec::new(),
            }
        }

        fn sync_mirror_size(&mut self) {
            let size = self.size.snapshot();
            if size != self.mirror_size {
                self.mirror.resize(terminal_size_from_pty(size));
                self.mirror_size = size;
            }
        }
    }

    #[derive(Clone)]
    enum TerminalResponse {
        Static(&'static [u8]),
        CursorPosition { private: bool },
        WindowPixelSize,
        CharacterCellSize,
        TextAreaSize,
        PrivateModeStatus(u16),
        AnsiModeStatus(u16),
        OscColor(OscColorResponse),
        ItermReportCellSize,
        Decrqss(DecrqssResponse),
        XtGetTcap(XtGetTcapResponse),
        XtSmGraphics(XtSmGraphicsRequest),
        XtVersion,
        KittyKeyboardFlags,
        KeyModifierOptions(u16),
        Osc8Hyperlink,
        Osc52Write(String),
        Osc52Query(String),
    }

    fn xtversion_response() -> Vec<u8> {
        format!("\x1bP>|R-Term {}\x1b\\", env!("CARGO_PKG_VERSION")).into_bytes()
    }

    fn encode_osc52_clipboard_response(selection: &str, text: &str) -> Vec<u8> {
        format!(
            "\x1b]52;{};{}\x07",
            selection,
            STANDARD.encode(text.as_bytes())
        )
        .into_bytes()
    }

    impl Default for LegacyTerminalOutputFilter {
        fn default() -> Self {
            Self::with_shared_size(SharedTerminalSize::default())
        }
    }

    fn terminal_size_from_pty(size: PtySize) -> TerminalSize {
        TerminalSize::new(size.columns(), size.rows())
    }

    fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.is_empty() {
            return Some(0);
        }

        haystack
            .windows(needle.len())
            .enumerate()
            .find_map(|(index, window)| {
                (window == needle && !raw_c1_prefix_is_utf8_continuation(haystack, index, needle))
                    .then_some(index)
            })
    }

    fn raw_c1_prefix_is_utf8_continuation(bytes: &[u8], index: usize, prefix: &[u8]) -> bool {
        prefix
            .first()
            .is_some_and(|byte| is_raw_c1_control_byte(*byte))
            && is_utf8_continuation_in_potential_sequence(bytes, index)
    }

    fn is_raw_c1_control_byte(byte: u8) -> bool {
        (0x80..=0x9f).contains(&byte)
    }

    fn is_utf8_continuation_in_potential_sequence(bytes: &[u8], index: usize) -> bool {
        if index == 0
            || bytes
                .get(index)
                .is_none_or(|byte| !is_utf8_continuation(*byte))
        {
            return false;
        }

        let mut start = index;
        while start > 0 && is_utf8_continuation(bytes[start]) {
            start -= 1;
        }
        if start == index {
            return false;
        }

        let Some(expected_len) = utf8_sequence_len(bytes[start]) else {
            return false;
        };

        index < start + expected_len
            && bytes[start + 1..=index]
                .iter()
                .all(|byte| is_utf8_continuation(*byte))
    }

    fn utf8_sequence_len(byte: u8) -> Option<usize> {
        match byte {
            0x00..=0x7f => Some(1),
            0xc2..=0xdf => Some(2),
            0xe0..=0xef => Some(3),
            0xf0..=0xf4 => Some(4),
            _ => None,
        }
    }

    fn is_utf8_continuation(byte: u8) -> bool {
        byte & 0b1100_0000 == 0b1000_0000
    }

    const UTF8_C1_OSC: &[u8] = b"\xc2\x9d";
    const UTF8_C1_DCS: &[u8] = b"\xc2\x90";
    const UTF8_C1_ST: &[u8] = b"\xc2\x9c";
    const OSC_START_PREFIXES: &[(&[u8], usize)] = &[
        (b"\x1b]".as_slice(), 2),
        (b"\x9d".as_slice(), 1),
        (UTF8_C1_OSC, UTF8_C1_OSC.len()),
    ];

    fn is_inside_osc_or_st_control_string(bytes: &[u8], index: usize) -> bool {
        is_inside_control_string(bytes, index, find_next_osc_start, find_osc_terminator)
            || is_inside_control_string(
                bytes,
                index,
                find_next_st_control_string_start,
                find_dcs_terminator,
            )
    }

    fn is_inside_control_string(
        bytes: &[u8],
        index: usize,
        mut find_next_start: impl FnMut(&[u8]) -> Option<(usize, usize)>,
        mut find_terminator: impl FnMut(&[u8]) -> Option<OscColorTerminator>,
    ) -> bool {
        let mut offset = 0;
        while offset < bytes.len() {
            let Some((relative_start, prefix_len)) = find_next_start(&bytes[offset..]) else {
                return false;
            };
            let start = offset + relative_start;
            if start >= index {
                return false;
            }

            let content_start = start + prefix_len;
            let Some(terminator) = find_terminator(&bytes[content_start..]) else {
                return true;
            };
            let end = content_start + terminator.index + terminator.length;
            if index < end {
                return true;
            }
            offset = end;
        }

        false
    }

    fn incomplete_osc_control_sequence_suffix_len(bytes: &[u8]) -> usize {
        find_incomplete_osc_control_sequence_start(bytes)
            .map_or(0, |start| bytes.len() - start)
            .max(suffix_len_matching_prefix(bytes, b"\x1b]"))
            .max(suffix_len_matching_prefix(bytes, UTF8_C1_OSC))
    }

    fn find_incomplete_osc_control_sequence_start(bytes: &[u8]) -> Option<usize> {
        let mut offset = 0;
        while offset < bytes.len() {
            let Some((relative_index, prefix_len)) = find_next_osc_start(&bytes[offset..]) else {
                break;
            };
            let index = offset + relative_index;
            let content_start = index + prefix_len;
            let Some(terminator) = find_osc_terminator(&bytes[content_start..]) else {
                return Some(index);
            };
            offset = content_start + terminator.index + terminator.length;
        }

        None
    }

    fn incomplete_st_control_sequence_suffix_len(bytes: &[u8]) -> usize {
        find_incomplete_st_control_sequence_start(bytes)
            .map_or(0, |start| bytes.len() - start)
            .max(
                [
                    b"\x1bP".as_slice(),
                    b"\x1bX".as_slice(),
                    b"\x1b^".as_slice(),
                    b"\x1b_".as_slice(),
                ]
                .into_iter()
                .map(|prefix| suffix_len_matching_prefix(bytes, prefix))
                .max()
                .unwrap_or(0),
            )
    }

    fn find_incomplete_st_control_sequence_start(bytes: &[u8]) -> Option<usize> {
        let mut offset = 0;
        while offset < bytes.len() {
            let Some((relative_index, prefix_len)) =
                find_next_st_control_string_start(&bytes[offset..])
            else {
                break;
            };
            let index = offset + relative_index;
            let content_start = index + prefix_len;
            let Some(terminator) = find_dcs_terminator(&bytes[content_start..]) else {
                return Some(index);
            };
            offset = content_start + terminator.index + terminator.length;
        }

        None
    }

    fn find_next_st_control_string_start(bytes: &[u8]) -> Option<(usize, usize)> {
        [
            (b"\x1bP".as_slice(), 2),
            (b"\x1bX".as_slice(), 2),
            (b"\x1b^".as_slice(), 2),
            (b"\x1b_".as_slice(), 2),
            (UTF8_C1_DCS, UTF8_C1_DCS.len()),
            (b"\x90".as_slice(), 1),
            (b"\x98".as_slice(), 1),
            (b"\x9e".as_slice(), 1),
            (b"\x9f".as_slice(), 1),
        ]
        .into_iter()
        .filter_map(|(prefix, prefix_len)| {
            find_subslice(bytes, prefix).map(|index| (index, prefix_len))
        })
        .min_by_key(|(index, _)| *index)
    }

    #[derive(Clone)]
    struct DecrqssResponse {
        kind: Option<DecrqssKind>,
        terminator: OscResponseTerminator,
    }

    #[derive(Clone, Copy)]
    enum DecrqssKind {
        Sgr,
        CursorShape,
        ScrollRegion,
        ConformanceLevel,
        LeftRightMargins,
    }

    impl DecrqssResponse {
        fn response(&self, terminal: &Terminal) -> Vec<u8> {
            let mut response = if let Some(kind) = self.kind {
                let mut bytes = b"\x1bP1$r".to_vec();
                match kind {
                    DecrqssKind::Sgr => append_sgr_state(terminal.active_style(), &mut bytes),
                    DecrqssKind::CursorShape => {
                        append_cursor_shape_state(terminal.cursor_shape(), &mut bytes);
                    }
                    DecrqssKind::ScrollRegion => {
                        append_scroll_region_state(terminal.scroll_region(), &mut bytes);
                    }
                    DecrqssKind::ConformanceLevel => bytes.extend_from_slice(b"61;1\"p"),
                    DecrqssKind::LeftRightMargins => {
                        append_left_right_margin_state(terminal.left_right_margins(), &mut bytes);
                    }
                }
                bytes
            } else {
                b"\x1bP0$r".to_vec()
            };
            response.extend_from_slice(self.terminator.bytes());
            response
        }
    }

    fn append_sgr_state(style: &Cell, bytes: &mut Vec<u8>) {
        let mut params = Vec::new();
        if style.bold {
            params.push("1".to_owned());
        }
        if style.faint {
            params.push("2".to_owned());
        }
        if style.italic {
            params.push("3".to_owned());
        }
        append_underline_style_sgr(style, &mut params);
        if style.blink {
            params.push("5".to_owned());
        }
        if style.inverse {
            params.push("7".to_owned());
        }
        if style.conceal {
            params.push("8".to_owned());
        }
        if style.strikethrough {
            params.push("9".to_owned());
        }
        if style.double_underline {
            params.push("21".to_owned());
        }
        if style.overline {
            params.push("53".to_owned());
        }
        match style.vertical_align {
            VerticalAlign::Baseline => {}
            VerticalAlign::Superscript => params.push("73".to_owned()),
            VerticalAlign::Subscript => params.push("74".to_owned()),
        }
        append_color_sgr(58, style.underline_color, &mut params);
        append_color_sgr(38, style.foreground, &mut params);
        append_color_sgr(48, style.background, &mut params);

        if params.is_empty() {
            bytes.push(b'0');
        } else {
            bytes.extend_from_slice(params.join(";").as_bytes());
        }
        bytes.push(b'm');
    }

    fn append_underline_style_sgr(style: &Cell, params: &mut Vec<String>) {
        match style.underline_style {
            UnderlineStyle::None if style.double_underline => params.push("21".to_owned()),
            UnderlineStyle::None if style.underline => params.push("4".to_owned()),
            UnderlineStyle::None => {}
            UnderlineStyle::Single => params.push("4".to_owned()),
            UnderlineStyle::Double => params.push("21".to_owned()),
            UnderlineStyle::Curly => params.push("4:3".to_owned()),
            UnderlineStyle::Dotted => params.push("4:4".to_owned()),
            UnderlineStyle::Dashed => params.push("4:5".to_owned()),
        }
    }

    fn append_color_sgr(prefix: u8, color: Color, params: &mut Vec<String>) {
        match color {
            Color::Default => {}
            Color::Indexed(index) => {
                params.push(prefix.to_string());
                params.push("5".to_owned());
                params.push(index.to_string());
            }
            Color::Rgb(red, green, blue) => {
                params.push(prefix.to_string());
                params.push("2".to_owned());
                params.push(red.to_string());
                params.push(green.to_string());
                params.push(blue.to_string());
            }
            Color::Rgba(red, green, blue, alpha) => {
                params.push(prefix.to_string());
                params.push("6".to_owned());
                params.push(red.to_string());
                params.push(green.to_string());
                params.push(blue.to_string());
                params.push(alpha.to_string());
            }
        }
    }

    fn append_cursor_shape_state(shape: CursorShape, bytes: &mut Vec<u8>) {
        let value = match shape {
            CursorShape::Block => 2,
            CursorShape::Underline => 3,
            CursorShape::Bar => 5,
        };
        bytes.extend_from_slice(value.to_string().as_bytes());
        bytes.extend_from_slice(b" q");
    }

    fn append_scroll_region_state((top, bottom): (u16, u16), bytes: &mut Vec<u8>) {
        bytes.extend_from_slice(top.saturating_add(1).to_string().as_bytes());
        bytes.push(b';');
        bytes.extend_from_slice(bottom.saturating_add(1).to_string().as_bytes());
        bytes.push(b'r');
    }

    fn append_left_right_margin_state((left, right): (u16, u16), bytes: &mut Vec<u8>) {
        bytes.extend_from_slice(left.saturating_add(1).to_string().as_bytes());
        bytes.push(b';');
        bytes.extend_from_slice(right.saturating_add(1).to_string().as_bytes());
        bytes.push(b's');
    }

    #[derive(Clone)]
    struct XtGetTcapResponse {
        entries: Vec<XtGetTcapEntry>,
    }

    #[derive(Clone)]
    struct XtGetTcapEntry {
        name_hex: Vec<u8>,
        value_hex: Option<Vec<u8>>,
    }

    impl XtGetTcapResponse {
        fn response(&self) -> Vec<u8> {
            let mut response = Vec::new();
            if self.entries.is_empty() {
                response.extend_from_slice(b"\x1bP0+r\x1b\\");
                return response;
            }

            for entry in &self.entries {
                let entry_start = response.len();
                if let Some(value_hex) = &entry.value_hex {
                    response.extend_from_slice(b"\x1bP1+r");
                    extend_ascii_hex_uppercase(&mut response, &entry.name_hex);
                    response.push(b'=');
                    extend_ascii_hex_uppercase(&mut response, value_hex);
                } else {
                    response.extend_from_slice(b"\x1bP0+r");
                    extend_ascii_hex_uppercase(&mut response, &entry.name_hex);
                }
                response.extend_from_slice(b"\x1b\\");
                if response.len() > MAX_XTGETTCAP_RESPONSE_BYTES {
                    response.truncate(entry_start);
                    break;
                }
            }
            response
        }
    }

    fn find_dcs_terminator(bytes: &[u8]) -> Option<OscColorTerminator> {
        let st =
            find_subslice(bytes, b"\x1b\\").map(|index| OscColorTerminator { index, length: 2 });
        let c1_st = bytes
            .iter()
            .position(|byte| *byte == 0x9c)
            .map(|index| OscColorTerminator { index, length: 1 });
        let utf8_c1_st = find_subslice(bytes, UTF8_C1_ST).map(|index| OscColorTerminator {
            index,
            length: UTF8_C1_ST.len(),
        });

        [st, c1_st, utf8_c1_st]
            .into_iter()
            .flatten()
            .min_by_key(|terminator| terminator.index)
    }

    #[allow(clippy::too_many_lines)]
    fn xtgettcap_value_hex(name: &[u8], size: PtySize, terminal_name: &str) -> Option<Vec<u8>> {
        match name {
        b"Co" | b"colors" => Some(b"323536".to_vec()),
        b"TN" | b"name" => Some(encode_ascii_hex(terminal_name.as_bytes())),
        b"RGB" => Some(b"382f382f38".to_vec()),
        b"Tc" | b"am" | b"bce" | b"ccc" | b"hs" | b"km" | b"mc5i" | b"mir" | b"msgr"
        | b"npc" | b"Su" | b"xenl" => Some(b"31".to_vec()),
        b"Ms" => Some(b"1b5d35323b25703125733b257032257307".to_vec()),
        b"dsl" => Some(encode_ascii_hex(b"\x1b]2;\x1b\\")),
        b"fsl" => Some(encode_ascii_hex(b"\x1b\\")),
        b"tsl" => Some(encode_ascii_hex(b"\x1b]0;")),
        b"initc" => Some(encode_ascii_hex(
            b"\x1b]4;%p1%d;rgb:%p2%{255}%*%{1000}%/%2.2X/%p3%{255}%*%{1000}%/%2.2X/%p4%{255}%*%{1000}%/%2.2X\x1b\\",
        )),
        b"Smulx" => Some(b"1b5b343a25703125646d".to_vec()),
        b"Setulc" => Some(
            b"1b5b35383a323a3a257031257b36353533367d252f25643a257031257b3235367d252f257b3235357d252625643a257031257b3235357d25262564253b6d"
                .to_vec(),
        ),
        b"Cr" => Some(encode_ascii_hex(b"\x1b]112\x07")),
        b"Cs" => Some(encode_ascii_hex(b"\x1b]12;%p1%s\x07")),
        b"Se" => Some(encode_ascii_hex(b"\x1b[2 q")),
        b"Ss" => Some(encode_ascii_hex(b"\x1b[%p1%d q")),
        b"Sync" => Some(encode_ascii_hex(b"\x1b[?2026%?%p1%{1}%-%tl%eh%;")),
        b"sitm" => Some(b"1b5b336d".to_vec()),
        b"ritm" => Some(b"1b5b32336d".to_vec()),
        b"Smol" => Some(encode_ascii_hex(b"\x1b[53m")),
        b"smxx" => Some(encode_ascii_hex(b"\x1b[9m")),
        b"rmxx" => Some(encode_ascii_hex(b"\x1b[29m")),
        b"flash" => Some(encode_ascii_hex(b"\x1b[?5h$<100/>\x1b[?5l")),
        b"op" => Some(encode_ascii_hex(b"\x1b[39;49m")),
        b"oc" => Some(encode_ascii_hex(b"\x1b]104\x07")),
        b"bel" => Some(encode_ascii_hex(b"\x07")),
        b"cr" => Some(encode_ascii_hex(b"\r")),
        b"ind" | b"cud1" => Some(encode_ascii_hex(b"\n")),
        b"ri" => Some(encode_ascii_hex(b"\x1bM")),
        b"sc" => Some(encode_ascii_hex(b"\x1b7")),
        b"rc" => Some(encode_ascii_hex(b"\x1b8")),
        b"u6" => Some(encode_ascii_hex(b"\x1b[%i%d;%dR")),
        b"u7" => Some(encode_ascii_hex(b"\x1b[6n")),
        b"u8" => Some(encode_ascii_hex(b"\x1b[?%[;0123456789]c")),
        b"u9" => Some(encode_ascii_hex(b"\x1b[c")),
        b"clear" => Some(encode_ascii_hex(b"\x1b[H\x1b[2J")),
        b"cup" => Some(encode_ascii_hex(b"\x1b[%i%p1%d;%p2%dH")),
        b"home" => Some(encode_ascii_hex(b"\x1b[H")),
        b"el" => Some(encode_ascii_hex(b"\x1b[K")),
        b"ed" => Some(encode_ascii_hex(b"\x1b[J")),
        b"el1" => Some(encode_ascii_hex(b"\x1b[1K")),
        b"dch" => Some(encode_ascii_hex(b"\x1b[%p1%dP")),
        b"dch1" => Some(encode_ascii_hex(b"\x1b[P")),
        b"ich" => Some(encode_ascii_hex(b"\x1b[%p1%d@")),
        b"ich1" => Some(encode_ascii_hex(b"\x1b[@")),
        b"il" => Some(encode_ascii_hex(b"\x1b[%p1%dL")),
        b"il1" => Some(encode_ascii_hex(b"\x1b[L")),
        b"dl" => Some(encode_ascii_hex(b"\x1b[%p1%dM")),
        b"dl1" => Some(encode_ascii_hex(b"\x1b[M")),
        b"cuu" => Some(encode_ascii_hex(b"\x1b[%p1%dA")),
        b"cuu1" => Some(encode_ascii_hex(b"\x1b[A")),
        b"cud" => Some(encode_ascii_hex(b"\x1b[%p1%dB")),
        b"cub" => Some(encode_ascii_hex(b"\x1b[%p1%dD")),
        b"cub1" => Some(encode_ascii_hex(b"\x08")),
        b"cuf" => Some(encode_ascii_hex(b"\x1b[%p1%dC")),
        b"cuf1" => Some(encode_ascii_hex(b"\x1b[C")),
        b"hpa" => Some(encode_ascii_hex(b"\x1b[%i%p1%dG")),
        b"vpa" => Some(encode_ascii_hex(b"\x1b[%i%p1%dd")),
        b"cbt" | b"kcbt" => Some(encode_ascii_hex(b"\x1b[Z")),
        b"ht" => Some(encode_ascii_hex(b"\t")),
        b"hts" => Some(encode_ascii_hex(b"\x1bH")),
        b"tbc" => Some(encode_ascii_hex(b"\x1b[3g")),
        b"ech" => Some(encode_ascii_hex(b"\x1b[%p1%dX")),
        b"rep" => Some(encode_ascii_hex(b"%p1%c\x1b[%p2%{1}%-%db")),
        b"csr" => Some(encode_ascii_hex(b"\x1b[%i%p1%d;%p2%dr")),
        b"indn" => Some(encode_ascii_hex(b"\x1b[%p1%dS")),
        b"rin" => Some(encode_ascii_hex(b"\x1b[%p1%dT")),
        b"kmous" => Some(encode_ascii_hex(b"\x1b[<")),
        b"XM" => Some(encode_ascii_hex(
            b"\x1b[?1006;1000%?%p1%{1}%=%th%el%;",
        )),
        b"xm" => Some(encode_ascii_hex(
            b"\x1b[<%i%p3%d;%p1%d;%p2%d;%?%p4%tM%em%;",
        )),
        b"civis" => Some(encode_ascii_hex(b"\x1b[?25l")),
        b"cnorm" => Some(encode_ascii_hex(b"\x1b[?12l\x1b[?25h")),
        b"cvvis" => Some(encode_ascii_hex(b"\x1b[?12;25h")),
        b"smcup" => Some(encode_ascii_hex(b"\x1b[?1049h\x1b[22;0;0t")),
        b"rmcup" => Some(encode_ascii_hex(b"\x1b[?1049l\x1b[23;0;0t")),
        b"is2" | b"rs2" => Some(encode_ascii_hex(b"\x1b[!p\x1b[?3;4l\x1b[4l\x1b>")),
        b"rs1" => Some(encode_ascii_hex(b"\x1bc\x1b]104\x07")),
        b"smir" => Some(encode_ascii_hex(b"\x1b[4h")),
        b"rmir" => Some(encode_ascii_hex(b"\x1b[4l")),
        b"smam" => Some(encode_ascii_hex(b"\x1b[?7h")),
        b"rmam" => Some(encode_ascii_hex(b"\x1b[?7l")),
        b"smm" => Some(encode_ascii_hex(b"\x1b[?1034h")),
        b"rmm" => Some(encode_ascii_hex(b"\x1b[?1034l")),
        b"mc0" => Some(encode_ascii_hex(b"\x1b[i")),
        b"mc4" => Some(encode_ascii_hex(b"\x1b[4i")),
        b"mc5" => Some(encode_ascii_hex(b"\x1b[5i")),
        b"meml" => Some(encode_ascii_hex(b"\x1bl")),
        b"memu" => Some(encode_ascii_hex(b"\x1bm")),
        b"smkx" => Some(encode_ascii_hex(b"\x1b[?1h\x1b=")),
        b"rmkx" => Some(encode_ascii_hex(b"\x1b[?1l\x1b>")),
        b"sgr0" => Some(encode_ascii_hex(b"\x1b(B\x1b[m")),
        b"sgr" => Some(encode_ascii_hex(
            b"%?%p9%t\x1b(0%e\x1b(B%;\x1b[0%?%p6%t;1%;%?%p5%t;2%;%?%p2%t;4%;%?%p1%p3%|%t;7%;%?%p4%t;5%;%?%p7%t;8%;m",
        )),
        b"bold" => Some(encode_ascii_hex(b"\x1b[1m")),
        b"dim" => Some(encode_ascii_hex(b"\x1b[2m")),
        b"blink" => Some(encode_ascii_hex(b"\x1b[5m")),
        b"rev" | b"smso" => Some(encode_ascii_hex(b"\x1b[7m")),
        b"rmso" => Some(encode_ascii_hex(b"\x1b[27m")),
        b"invis" => Some(encode_ascii_hex(b"\x1b[8m")),
        b"smul" => Some(encode_ascii_hex(b"\x1b[4m")),
        b"rmul" => Some(encode_ascii_hex(b"\x1b[24m")),
        b"setaf" => Some(encode_ascii_hex(
            b"\x1b[%?%p1%{8}%<%t3%p1%d%e%p1%{16}%<%t9%p1%{8}%-%d%e38;5;%p1%d%;m",
        )),
        b"setab" => Some(encode_ascii_hex(
            b"\x1b[%?%p1%{8}%<%t4%p1%d%e%p1%{16}%<%t10%p1%{8}%-%d%e48;5;%p1%d%;m",
        )),
        b"kcuu1" => Some(encode_ascii_hex(b"\x1bOA")),
        b"kcud1" => Some(encode_ascii_hex(b"\x1bOB")),
        b"kcuf1" => Some(encode_ascii_hex(b"\x1bOC")),
        b"kcub1" => Some(encode_ascii_hex(b"\x1bOD")),
        b"kb2" => Some(encode_ascii_hex(b"\x1bOE")),
        b"kbs" => Some(encode_ascii_hex(b"\x7f")),
        b"khome" => Some(encode_ascii_hex(b"\x1bOH")),
        b"kend" => Some(encode_ascii_hex(b"\x1bOF")),
        b"kich1" => Some(encode_ascii_hex(b"\x1b[2~")),
        b"kdch1" => Some(encode_ascii_hex(b"\x1b[3~")),
        b"kpp" => Some(encode_ascii_hex(b"\x1b[5~")),
        b"knp" => Some(encode_ascii_hex(b"\x1b[6~")),
        b"kHOM" => Some(encode_ascii_hex(b"\x1b[1;2H")),
        b"kEND" => Some(encode_ascii_hex(b"\x1b[1;2F")),
        b"kIC" => Some(encode_ascii_hex(b"\x1b[2;2~")),
        b"kDC" => Some(encode_ascii_hex(b"\x1b[3;2~")),
        b"kPRV" => Some(encode_ascii_hex(b"\x1b[5;2~")),
        b"kNXT" => Some(encode_ascii_hex(b"\x1b[6;2~")),
        b"kLFT" => Some(encode_ascii_hex(b"\x1b[1;2D")),
        b"kRIT" => Some(encode_ascii_hex(b"\x1b[1;2C")),
        b"kri" => Some(encode_ascii_hex(b"\x1b[1;2A")),
        b"kind" => Some(encode_ascii_hex(b"\x1b[1;2B")),
        b"kent" => Some(encode_ascii_hex(b"\x1bOM")),
        b"kf1" => Some(encode_ascii_hex(b"\x1bOP")),
        b"kf2" => Some(encode_ascii_hex(b"\x1bOQ")),
        b"kf3" => Some(encode_ascii_hex(b"\x1bOR")),
        b"kf4" => Some(encode_ascii_hex(b"\x1bOS")),
        b"kf5" => Some(encode_ascii_hex(b"\x1b[15~")),
        b"kf6" => Some(encode_ascii_hex(b"\x1b[17~")),
        b"kf7" => Some(encode_ascii_hex(b"\x1b[18~")),
        b"kf8" => Some(encode_ascii_hex(b"\x1b[19~")),
        b"kf9" => Some(encode_ascii_hex(b"\x1b[20~")),
        b"kf10" => Some(encode_ascii_hex(b"\x1b[21~")),
        b"kf11" => Some(encode_ascii_hex(b"\x1b[23~")),
        b"kf12" => Some(encode_ascii_hex(b"\x1b[24~")),
        name if name.starts_with(b"kf") => xtgettcap_modified_function_key_hex(name),
        b"enacs" => Some(encode_ascii_hex(b"\x1b)0")),
        b"smacs" => Some(encode_ascii_hex(b"\x1b(0")),
        b"rmacs" => Some(encode_ascii_hex(b"\x1b(B")),
        b"acsc" => Some(encode_ascii_hex(
            b"``aaffggiijjkkllmmnnooppqqrrssttuuvvwwxxyyzz{{||}}~~",
        )),
        b"co" | b"cols" => Some(decimal_value_hex(size.columns())),
        b"li" | b"lines" => Some(decimal_value_hex(size.rows())),
        b"it" => Some(decimal_value_hex(8)),
        b"pairs" => Some(decimal_value_hex(0x7fff)),
        _ => None,
    }
    }

    fn xtgettcap_modified_function_key_hex(name: &[u8]) -> Option<Vec<u8>> {
        let number = parse_ascii_decimal_u8(name.strip_prefix(b"kf")?)?;
        let (function_key, modifier) = match number {
            13..=24 => (number - 12, 2),
            25..=36 => (number - 24, 5),
            37..=48 => (number - 36, 6),
            49..=60 => (number - 48, 3),
            61..=63 => (number - 60, 4),
            _ => return None,
        };

        let sequence = match function_key {
            1 => format!("\x1b[1;{modifier}P"),
            2 => format!("\x1b[1;{modifier}Q"),
            3 => format!("\x1b[1;{modifier}R"),
            4 => format!("\x1b[1;{modifier}S"),
            5 => format!("\x1b[15;{modifier}~"),
            6 => format!("\x1b[17;{modifier}~"),
            7 => format!("\x1b[18;{modifier}~"),
            8 => format!("\x1b[19;{modifier}~"),
            9 => format!("\x1b[20;{modifier}~"),
            10 => format!("\x1b[21;{modifier}~"),
            11 => format!("\x1b[23;{modifier}~"),
            12 => format!("\x1b[24;{modifier}~"),
            _ => return None,
        };

        Some(encode_ascii_hex(sequence.as_bytes()))
    }

    fn parse_ascii_decimal_u8(bytes: &[u8]) -> Option<u8> {
        if bytes.is_empty() {
            return None;
        }

        let mut value = 0u8;
        for &byte in bytes {
            if !byte.is_ascii_digit() {
                return None;
            }
            value = value.checked_mul(10)?.checked_add(byte - b'0')?;
        }
        Some(value)
    }

    fn decimal_value_hex(value: u16) -> Vec<u8> {
        encode_ascii_hex(value.to_string().as_bytes())
    }

    pub(super) fn encode_ascii_hex(bytes: &[u8]) -> Vec<u8> {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut encoded = Vec::with_capacity(bytes.len() * 2);
        for byte in bytes {
            encoded.push(HEX[usize::from(byte >> 4)]);
            encoded.push(HEX[usize::from(byte & 0x0f)]);
        }
        encoded
    }

    fn extend_ascii_hex_uppercase(output: &mut Vec<u8>, hex: &[u8]) {
        output.extend(hex.iter().map(u8::to_ascii_uppercase));
    }

    #[derive(Clone, Copy)]
    struct XtSmGraphicsRequest {
        item: u64,
        action: u64,
    }

    impl XtSmGraphicsRequest {
        const ACTION_READ_ATTRIBUTE: u64 = 1;
        const ACTION_RESET_TO_DEFAULT: u64 = 2;
        const ACTION_READ_MAXIMUM_ALLOWED_VALUE: u64 = 4;
        const ITEM_NUMBER_OF_COLOR_REGISTERS: u64 = 1;
        const ITEM_SIXEL_GRAPHICS_GEOMETRY: u64 = 2;
        const ITEM_REGIS_GRAPHICS_GEOMETRY: u64 = 3;
        const STATUS_SUCCESS: u64 = 0;
        const STATUS_INVALID_ITEM: u64 = 1;
        const STATUS_INVALID_ACTION: u64 = 2;

        fn response(self, size: PtySize) -> Vec<u8> {
            let (status, values) = self.status_and_values(size);
            let mut response = format!("\x1b[?{};{}", self.item, status);
            for value in values {
                response.push(';');
                response.push_str(&value.to_string());
            }
            response.push('S');
            response.into_bytes()
        }

        fn status_and_values(self, size: PtySize) -> (u64, Vec<u32>) {
            if !matches!(
                self.item,
                Self::ITEM_NUMBER_OF_COLOR_REGISTERS
                    | Self::ITEM_SIXEL_GRAPHICS_GEOMETRY
                    | Self::ITEM_REGIS_GRAPHICS_GEOMETRY
            ) {
                return (Self::STATUS_INVALID_ITEM, Vec::new());
            }

            match self.action {
                Self::ACTION_READ_ATTRIBUTE | Self::ACTION_READ_MAXIMUM_ALLOWED_VALUE => {
                    (Self::STATUS_SUCCESS, self.values(size))
                }
                Self::ACTION_RESET_TO_DEFAULT => (Self::STATUS_SUCCESS, Vec::new()),
                _ => (Self::STATUS_INVALID_ACTION, Vec::new()),
            }
        }

        fn values(self, size: PtySize) -> Vec<u32> {
            match self.item {
                Self::ITEM_NUMBER_OF_COLOR_REGISTERS => vec![65_536],
                Self::ITEM_SIXEL_GRAPHICS_GEOMETRY | Self::ITEM_REGIS_GRAPHICS_GEOMETRY => vec![
                    u32::from(size.columns())
                        * u32::from(LegacyTerminalOutputFilter::CELL_WIDTH_PIXELS),
                    u32::from(size.rows())
                        * u32::from(LegacyTerminalOutputFilter::CELL_HEIGHT_PIXELS),
                ],
                _ => Vec::new(),
            }
        }
    }

    #[derive(Clone)]
    struct OscColorResponse {
        kinds: Vec<OscColorKind>,
        terminator: OscResponseTerminator,
    }

    #[derive(Clone, Copy)]
    enum OscColorKind {
        DefaultForeground,
        DefaultBackground,
        Cursor,
        Palette(u8),
    }

    #[derive(Clone, Copy)]
    enum OscResponseTerminator {
        Bel,
        St,
        C1St,
    }

    struct OscColorTerminator {
        index: usize,
        length: usize,
    }

    fn find_osc_terminator(bytes: &[u8]) -> Option<OscColorTerminator> {
        let bel = bytes
            .iter()
            .position(|byte| *byte == b'\x07')
            .map(|index| OscColorTerminator { index, length: 1 });
        let st =
            find_subslice(bytes, b"\x1b\\").map(|index| OscColorTerminator { index, length: 2 });
        let c1_st = bytes
            .iter()
            .position(|byte| *byte == 0x9c)
            .map(|index| OscColorTerminator { index, length: 1 });
        let utf8_c1_st = find_subslice(bytes, UTF8_C1_ST).map(|index| OscColorTerminator {
            index,
            length: UTF8_C1_ST.len(),
        });

        [bel, st, c1_st, utf8_c1_st]
            .into_iter()
            .flatten()
            .min_by_key(|terminator| terminator.index)
    }

    struct TerminalColorState {
        foreground: DynamicColor,
        background: DynamicColor,
        cursor: DynamicColor,
        palette_overrides: Vec<(u8, [u8; 3])>,
        pending: Vec<u8>,
    }

    impl Default for TerminalColorState {
        fn default() -> Self {
            Self {
                foreground: DynamicColor::rgb8(DEFAULT_FOREGROUND),
                background: DynamicColor::rgb8(DEFAULT_BACKGROUND),
                cursor: DynamicColor::rgb8(DEFAULT_CURSOR),
                palette_overrides: Vec::new(),
                pending: Vec::new(),
            }
        }
    }

    impl TerminalColorState {
        const MAX_PENDING: usize = 1024 * 1024;

        fn process(&mut self, bytes: &[u8]) {
            self.pending.extend_from_slice(bytes);
            if self.pending.len() > Self::MAX_PENDING {
                self.pending.clear();
                return;
            }

            loop {
                let Some((index, prefix_len)) = find_next_osc_start(&self.pending) else {
                    self.retain_possible_prefix();
                    return;
                };
                if is_inside_osc_or_st_control_string(&self.pending, index) {
                    self.pending.drain(..index.saturating_add(1));
                    continue;
                }
                if index > 0 {
                    self.pending.drain(..index);
                }

                let content_start = prefix_len;
                let Some(terminator) = find_osc_terminator(&self.pending[content_start..]) else {
                    return;
                };
                let content_end = content_start + terminator.index;
                if let Some(change) =
                    parse_osc_color_change(&self.pending[content_start..content_end])
                {
                    self.apply(change);
                }
                self.pending.drain(..content_end + terminator.length);
            }
        }

        fn response(&self, query: OscColorResponse) -> Vec<u8> {
            let mut response = Vec::new();
            for kind in query.kinds {
                let mut item = match kind {
                    OscColorKind::DefaultForeground => {
                        format!("\x1b]10;{}", color_response(self.foreground)).into_bytes()
                    }
                    OscColorKind::DefaultBackground => {
                        format!("\x1b]11;{}", color_response(self.background)).into_bytes()
                    }
                    OscColorKind::Cursor => {
                        format!("\x1b]12;{}", color_response(self.cursor)).into_bytes()
                    }
                    OscColorKind::Palette(index) => format!(
                        "\x1b]4;{};{}",
                        index,
                        palette_color_response(self.palette_color(index))
                    )
                    .into_bytes(),
                };
                item.extend_from_slice(query.terminator.bytes());
                response.extend(item);
            }
            response
        }

        fn apply(&mut self, change: OscColorChange) {
            match change {
                OscColorChange::DefaultForeground(color) => self.foreground = color,
                OscColorChange::DefaultBackground(color) => self.background = color,
                OscColorChange::Cursor(color) => self.cursor = color,
                OscColorChange::ResetDefaultForeground => {
                    self.foreground = DynamicColor::rgb8(DEFAULT_FOREGROUND);
                }
                OscColorChange::ResetDefaultBackground => {
                    self.background = DynamicColor::rgb8(DEFAULT_BACKGROUND);
                }
                OscColorChange::ResetCursor => self.cursor = DynamicColor::rgb8(DEFAULT_CURSOR),
                OscColorChange::ResetPalette(indices) => self
                    .palette_overrides
                    .retain(|(palette_index, _)| !indices.contains(palette_index)),
                OscColorChange::ResetPaletteAll => self.palette_overrides.clear(),
                OscColorChange::Palette(changes) => {
                    for (index, color) in changes {
                        if let Some((_, existing)) = self
                            .palette_overrides
                            .iter_mut()
                            .find(|(palette_index, _)| *palette_index == index)
                        {
                            *existing = color;
                        } else {
                            self.palette_overrides.push((index, color));
                        }
                    }
                }
            }
        }

        fn palette_color(&self, index: u8) -> [u8; 3] {
            self.palette_overrides
                .iter()
                .find_map(|(palette_index, color)| (*palette_index == index).then_some(*color))
                .unwrap_or_else(|| indexed_color(index))
        }

        fn retain_possible_prefix(&mut self) {
            let retained = OSC_START_PREFIXES
                .iter()
                .map(|(prefix, _)| suffix_len_matching_prefix(&self.pending, prefix))
                .max()
                .unwrap_or(0);
            let retained = retained
                .max(incomplete_osc_control_sequence_suffix_len(&self.pending))
                .max(incomplete_st_control_sequence_suffix_len(&self.pending));
            let writable = self.pending.len().saturating_sub(retained);
            if writable > 0 {
                self.pending.drain(..writable);
            }
        }
    }

    #[derive(Clone)]
    enum OscColorChange {
        DefaultForeground(DynamicColor),
        DefaultBackground(DynamicColor),
        Cursor(DynamicColor),
        ResetDefaultForeground,
        ResetDefaultBackground,
        ResetCursor,
        ResetPalette(Vec<u8>),
        ResetPaletteAll,
        Palette(Vec<(u8, [u8; 3])>),
    }

    fn find_next_osc_start(bytes: &[u8]) -> Option<(usize, usize)> {
        OSC_START_PREFIXES
            .iter()
            .filter_map(|(prefix, prefix_len)| {
                find_subslice(bytes, prefix).map(|index| (index, *prefix_len))
            })
            .min_by_key(|(index, _)| *index)
    }

    fn parse_osc_color_change(content: &[u8]) -> Option<OscColorChange> {
        if let Some(color) = content.strip_prefix(b"10;").and_then(parse_color_spec) {
            return Some(OscColorChange::DefaultForeground(color));
        }
        if let Some(color) = content.strip_prefix(b"11;").and_then(parse_color_spec) {
            return Some(OscColorChange::DefaultBackground(color));
        }
        if let Some(color) = content.strip_prefix(b"12;").and_then(parse_color_spec) {
            return Some(OscColorChange::Cursor(color));
        }
        if matches!(content, b"110" | b"110;") {
            return Some(OscColorChange::ResetDefaultForeground);
        }
        if matches!(content, b"111" | b"111;") {
            return Some(OscColorChange::ResetDefaultBackground);
        }
        if matches!(content, b"112" | b"112;") {
            return Some(OscColorChange::ResetCursor);
        }
        if let Some(change) = parse_palette_reset_change(content) {
            return Some(change);
        }
        parse_palette_color_change(content)
    }

    fn parse_palette_reset_change(content: &[u8]) -> Option<OscColorChange> {
        if matches!(content, b"104" | b"104;") {
            return Some(OscColorChange::ResetPaletteAll);
        }
        let rest = content.strip_prefix(b"104;")?;
        let mut indices = Vec::new();
        for index in rest.split(|byte| *byte == b';') {
            indices.push(parse_u8_decimal(index)?);
        }
        (!indices.is_empty()).then_some(OscColorChange::ResetPalette(indices))
    }

    fn parse_palette_color_change(content: &[u8]) -> Option<OscColorChange> {
        let rest = content.strip_prefix(b"4;")?;
        let mut changes = Vec::new();
        let mut parts = rest.split(|byte| *byte == b';');

        while let Some(index) = parts.next() {
            let color = parts.next()?;
            changes.push((parse_u8_decimal(index)?, parse_color_spec(color)?.to_rgb8()));
        }

        (!changes.is_empty()).then_some(OscColorChange::Palette(changes))
    }

    fn parse_u8_decimal(bytes: &[u8]) -> Option<u8> {
        if bytes.is_empty() {
            return None;
        }
        bytes.iter().try_fold(0_u8, |value, byte| {
            let digit = byte.checked_sub(b'0')?;
            (digit <= 9)
                .then_some(value)?
                .checked_mul(10)?
                .checked_add(digit)
        })
    }

    fn parse_color_spec(value: &[u8]) -> Option<DynamicColor> {
        if let Some(hex) = value.strip_prefix(b"#") {
            return parse_hex_color_spec(hex);
        }
        if let Some(rest) = value.strip_prefix(b"rgba:") {
            return parse_slash_rgba_color_spec(rest);
        }
        if value.starts_with(b"rgba(") {
            return parse_function_rgba_color_spec(value);
        }

        let rest = value.strip_prefix(b"rgb:")?;
        let mut components = rest.split(|byte| *byte == b'/');
        let red = parse_rgb_component(components.next()?)?;
        let green = parse_rgb_component(components.next()?)?;
        let blue = parse_rgb_component(components.next()?)?;
        components
            .next()
            .is_none()
            .then_some(DynamicColor::rgb(red, green, blue))
    }

    fn parse_hex_color_spec(hex: &[u8]) -> Option<DynamicColor> {
        match hex.len() {
            3 => Some(DynamicColor::rgb8([
                parse_hex_digit(hex[0])? * 17,
                parse_hex_digit(hex[1])? * 17,
                parse_hex_digit(hex[2])? * 17,
            ])),
            6 => Some(DynamicColor::rgb8([
                parse_hex_byte(&hex[0..2])?,
                parse_hex_byte(&hex[2..4])?,
                parse_hex_byte(&hex[4..6])?,
            ])),
            _ => None,
        }
    }

    fn parse_slash_rgba_color_spec(value: &[u8]) -> Option<DynamicColor> {
        let mut components = value.split(|byte| *byte == b'/');
        let red = parse_hex_component16(components.next()?)?;
        let green = parse_hex_component16(components.next()?)?;
        let blue = parse_hex_component16(components.next()?)?;
        let alpha = parse_hex_component16(components.next()?)?;
        components
            .next()
            .is_none()
            .then_some(DynamicColor::rgba(red, green, blue, alpha))
    }

    fn parse_function_rgba_color_spec(value: &[u8]) -> Option<DynamicColor> {
        let inner = value.strip_prefix(b"rgba(")?.strip_suffix(b")")?;
        let mut components = inner.split(|byte| *byte == b',');
        let red = parse_u8_decimal(components.next()?.trim_ascii())?;
        let green = parse_u8_decimal(components.next()?.trim_ascii())?;
        let blue = parse_u8_decimal(components.next()?.trim_ascii())?;
        let alpha = parse_alpha_float_component(components.next()?.trim_ascii())?;
        components
            .next()
            .is_none()
            .then_some(DynamicColor::rgba8(red, green, blue, alpha))
    }

    fn parse_rgb_component(component: &[u8]) -> Option<u16> {
        match component.len() {
            1 => parse_hex_digit(component[0]).map(|value| u16::from(value) * 0x1111),
            2 => parse_hex_byte(component).map(DynamicColor::expand_byte),
            3 | 4 => parse_hex_component16(component),
            _ => None,
        }
    }

    fn parse_hex_component16(component: &[u8]) -> Option<u16> {
        match component.len() {
            1 => parse_hex_digit(component[0]).map(|value| u16::from(value) * 0x1111),
            2 => parse_hex_byte(component).map(DynamicColor::expand_byte),
            3 => Some(
                parse_hex_digit(component[0]).map(u16::from)? * 0x1000
                    + parse_hex_digit(component[1]).map(u16::from)? * 0x100
                    + parse_hex_digit(component[2]).map(u16::from)? * 0x10,
            ),
            4 => Some(
                parse_hex_digit(component[0]).map(u16::from)? * 0x1000
                    + parse_hex_digit(component[1]).map(u16::from)? * 0x100
                    + parse_hex_digit(component[2]).map(u16::from)? * 0x10
                    + parse_hex_digit(component[3]).map(u16::from)?,
            ),
            _ => None,
        }
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn parse_alpha_float_component(component: &[u8]) -> Option<u16> {
        let text = std::str::from_utf8(component).ok()?;
        let value = text.parse::<f32>().ok()?;
        if !(0.0..=1.0).contains(&value) {
            return None;
        }
        Some((value * f32::from(u16::MAX)).round() as u16)
    }

    fn parse_hex_byte(bytes: &[u8]) -> Option<u8> {
        Some(parse_hex_digit(bytes[0])? * 16 + parse_hex_digit(bytes[1])?)
    }

    fn parse_hex_digit(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }

    impl OscResponseTerminator {
        const fn bytes(self) -> &'static [u8] {
            match self {
                Self::Bel => b"\x07",
                Self::St => b"\x1b\\",
                Self::C1St => b"\x9c",
            }
        }
    }

    const DEFAULT_FOREGROUND: [u8; 3] = [229, 229, 229];
    const DEFAULT_BACKGROUND: [u8; 3] = [12, 12, 12];
    const DEFAULT_CURSOR: [u8; 3] = DEFAULT_FOREGROUND;

    #[derive(Clone, Copy)]
    struct DynamicColor {
        red: u16,
        green: u16,
        blue: u16,
        alpha: Option<u16>,
    }

    impl DynamicColor {
        const fn rgb8(color: [u8; 3]) -> Self {
            Self::rgb(
                color[0] as u16 * 0x101,
                color[1] as u16 * 0x101,
                color[2] as u16 * 0x101,
            )
        }

        const fn rgb(red: u16, green: u16, blue: u16) -> Self {
            Self {
                red,
                green,
                blue,
                alpha: None,
            }
        }

        const fn rgba(red: u16, green: u16, blue: u16, alpha: u16) -> Self {
            Self {
                red,
                green,
                blue,
                alpha: Some(alpha),
            }
        }

        const fn rgba8(red: u8, green: u8, blue: u8, alpha: u16) -> Self {
            Self::rgba(
                Self::expand_byte(red),
                Self::expand_byte(green),
                Self::expand_byte(blue),
                alpha,
            )
        }

        const fn expand_byte(value: u8) -> u16 {
            value as u16 * 0x101
        }

        const fn to_rgb8(self) -> [u8; 3] {
            [
                (self.red >> 8) as u8,
                (self.green >> 8) as u8,
                (self.blue >> 8) as u8,
            ]
        }
    }

    fn color_response(color: DynamicColor) -> String {
        match color.alpha {
            Some(alpha) => format!(
                "rgba:{:04x}/{:04x}/{:04x}/{:04x}",
                color.red, color.green, color.blue, alpha
            ),
            None => format!(
                "rgb:{:04x}/{:04x}/{:04x}",
                color.red, color.green, color.blue
            ),
        }
    }

    fn palette_color_response(color: [u8; 3]) -> String {
        color_response(DynamicColor::rgb8(color))
    }

    fn indexed_color(index: u8) -> [u8; 3] {
        const ANSI: [[u8; 3]; 16] = [
            [0, 0, 0],
            [205, 49, 49],
            [13, 188, 121],
            [229, 229, 16],
            [36, 114, 200],
            [188, 63, 188],
            [17, 168, 205],
            [229, 229, 229],
            [102, 102, 102],
            [241, 76, 76],
            [35, 209, 139],
            [245, 245, 67],
            [59, 142, 234],
            [214, 112, 214],
            [41, 184, 219],
            [255, 255, 255],
        ];

        if let Some(color) = ANSI.get(usize::from(index)) {
            return *color;
        }

        if (16..=231).contains(&index) {
            let cube_index = index - 16;
            return [
                xterm_color_cube_intensity(cube_index / 36),
                xterm_color_cube_intensity((cube_index / 6) % 6),
                xterm_color_cube_intensity(cube_index % 6),
            ];
        }

        let level = 8 + (index - 232) * 10;
        [level, level, level]
    }

    const fn xterm_color_cube_intensity(value: u8) -> u8 {
        if value == 0 { 0 } else { 55 + value * 40 }
    }

    fn suffix_len_matching_prefix(haystack: &[u8], needle: &[u8]) -> usize {
        let max = haystack.len().min(needle.len().saturating_sub(1));
        (1..=max)
            .rev()
            .find(|&length| {
                let suffix_start = haystack.len() - length;
                haystack[suffix_start..] == needle[..length]
                    && !raw_c1_prefix_is_utf8_continuation(
                        haystack,
                        suffix_start,
                        &needle[..length],
                    )
            })
            .unwrap_or(0)
    }
}

#[cfg(test)]
use legacy_terminal_output::{
    LegacyTerminalOutputFilter as TerminalOutputFilter, encode_ascii_hex,
};

#[derive(Clone, Copy, Default)]
struct InputModes {
    bits: u16,
    kitty_keyboard_flags: u16,
    modify_other_keys: u8,
    win32_input_mode: bool,
}

impl InputModes {
    const APPLICATION_CURSOR_KEYS: u16 = 1;
    const APPLICATION_KEYPAD: u16 = 1 << 1;
    const BRACKETED_PASTE: u16 = 1 << 2;
    const FOCUS_REPORTING: u16 = 1 << 3;
    const MOUSE_INPUT_MASK: u16 = 0b1_1111_0000;
    const MOUSE_REPORTING_SHIFT: u8 = 4;

    fn application_cursor_keys(self) -> bool {
        self.enabled(Self::APPLICATION_CURSOR_KEYS)
    }

    fn bracketed_paste(self) -> bool {
        self.enabled(Self::BRACKETED_PASTE)
    }

    fn application_keypad(self) -> bool {
        self.enabled(Self::APPLICATION_KEYPAD)
    }

    fn mouse_reporting(self) -> bool {
        self.mouse_input_mode().reporting_enabled()
    }

    fn mouse_input_mode(self) -> MouseInputMode {
        MouseInputMode::from_bits(
            ((self.bits & Self::MOUSE_INPUT_MASK) >> Self::MOUSE_REPORTING_SHIFT) as u8,
        )
    }

    fn focus_reporting(self) -> bool {
        self.enabled(Self::FOCUS_REPORTING)
    }

    fn kitty_keyboard_flags(self) -> u16 {
        self.kitty_keyboard_flags
    }

    fn modify_other_keys(self) -> u8 {
        self.modify_other_keys
    }

    fn win32_input_mode(self) -> bool {
        self.win32_input_mode
    }

    fn with_application_cursor_keys(self, enabled: bool) -> Self {
        self.with_flag(Self::APPLICATION_CURSOR_KEYS, enabled)
    }

    fn with_application_keypad(self, enabled: bool) -> Self {
        self.with_flag(Self::APPLICATION_KEYPAD, enabled)
    }

    fn with_bracketed_paste(self, enabled: bool) -> Self {
        self.with_flag(Self::BRACKETED_PASTE, enabled)
    }

    fn with_mouse_input_mode(mut self, mode: MouseInputMode) -> Self {
        self.bits &= !Self::MOUSE_INPUT_MASK;
        self.bits |= u16::from(mode.bits()) << Self::MOUSE_REPORTING_SHIFT;
        self
    }

    fn with_focus_reporting(self, enabled: bool) -> Self {
        self.with_flag(Self::FOCUS_REPORTING, enabled)
    }

    fn with_kitty_keyboard_flags(mut self, flags: u16) -> Self {
        self.kitty_keyboard_flags = flags;
        self
    }

    fn with_modify_other_keys(mut self, mode: u8) -> Self {
        self.modify_other_keys = mode;
        self
    }

    fn with_win32_input_mode(mut self, enabled: bool) -> Self {
        self.win32_input_mode = enabled;
        self
    }

    fn enabled(self, flag: u16) -> bool {
        self.bits & flag != 0
    }

    fn with_flag(mut self, flag: u16, enabled: bool) -> Self {
        if enabled {
            self.bits |= flag;
        } else {
            self.bits &= !flag;
        }
        self
    }
}

fn encode_input_event(event: Event, modes: InputModes) -> Option<Vec<u8>> {
    match event {
        Event::Key(key) if modes.win32_input_mode() => encode_key_with_mode(key, modes),
        Event::Key(key) if key.kind == KeyEventKind::Press => encode_key_with_mode(key, modes),
        Event::Key(key) if reports_kitty_key_event_type(key.kind, modes.kitty_keyboard_flags()) => {
            encode_key_with_mode(key, modes)
        }
        Event::Paste(text) if modes.bracketed_paste() => Some(encode_bracketed_paste(&text)),
        Event::Paste(text) => Some(text.into_bytes()),
        Event::Mouse(event) if modes.mouse_reporting() => {
            encode_mouse_event(event, modes.mouse_input_mode())
        }
        Event::FocusGained if modes.focus_reporting() => Some(b"\x1b[I".to_vec()),
        Event::FocusLost if modes.focus_reporting() => Some(b"\x1b[O".to_vec()),
        _ => None,
    }
}

fn encode_bracketed_paste(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(b"\x1b[200~".len() + text.len() + b"\x1b[201~".len());
    bytes.extend_from_slice(b"\x1b[200~");
    bytes.extend_from_slice(text.as_bytes());
    bytes.extend_from_slice(b"\x1b[201~");
    bytes
}

fn encode_mouse_event(event: MouseEvent, mode: MouseInputMode) -> Option<Vec<u8>> {
    if !mouse_input_mode_allows(mode, event.kind) {
        return None;
    }

    let mut code = match event.kind {
        MouseEventKind::Down(button) | MouseEventKind::Up(button) => mouse_button_code(button),
        MouseEventKind::Drag(button) => mouse_button_code(button) + 32,
        MouseEventKind::Moved => 35,
        MouseEventKind::ScrollUp => 64,
        MouseEventKind::ScrollDown => 65,
        MouseEventKind::ScrollLeft => 66,
        MouseEventKind::ScrollRight => 67,
    };

    if event.modifiers.contains(KeyModifiers::SHIFT) {
        code += 4;
    }
    if event.modifiers.contains(KeyModifiers::ALT) {
        code += 8;
    }
    if event.modifiers.contains(KeyModifiers::CONTROL) {
        code += 16;
    }

    let column = event.column.checked_add(1)?;
    let row = event.row.checked_add(1)?;

    match mode.protocol() {
        MouseProtocolMode::Sgr | MouseProtocolMode::SgrPixels => {
            let final_byte = if matches!(event.kind, MouseEventKind::Up(_)) {
                b'm'
            } else {
                b'M'
            };
            Some(format!("\x1b[<{code};{column};{row}{}", final_byte as char).into_bytes())
        }
        MouseProtocolMode::Utf8 => encode_utf8_mouse_event(event.kind, code, column, row),
        MouseProtocolMode::Urxvt => encode_urxvt_mouse_event(event.kind, code, column, row),
        MouseProtocolMode::X10 => encode_legacy_mouse_event(event.kind, code, column, row),
    }
}

fn encode_legacy_mouse_event(
    kind: MouseEventKind,
    mut code: u16,
    column: u16,
    row: u16,
) -> Option<Vec<u8>> {
    if matches!(kind, MouseEventKind::Up(_)) {
        code = legacy_mouse_release_code(code);
    }

    Some(vec![
        0x1b,
        b'[',
        b'M',
        legacy_mouse_byte(code)?,
        legacy_mouse_byte(column)?,
        legacy_mouse_byte(row)?,
    ])
}

fn encode_utf8_mouse_event(
    kind: MouseEventKind,
    mut code: u16,
    column: u16,
    row: u16,
) -> Option<Vec<u8>> {
    if matches!(kind, MouseEventKind::Up(_)) {
        code = legacy_mouse_release_code(code);
    }

    let mut bytes = b"\x1b[M".to_vec();
    push_utf8_mouse_value(&mut bytes, code)?;
    push_utf8_mouse_value(&mut bytes, column)?;
    push_utf8_mouse_value(&mut bytes, row)?;
    Some(bytes)
}

fn encode_urxvt_mouse_event(
    kind: MouseEventKind,
    mut code: u16,
    column: u16,
    row: u16,
) -> Option<Vec<u8>> {
    if matches!(kind, MouseEventKind::Up(_)) {
        code = legacy_mouse_release_code(code);
    }

    let encoded_code = code.checked_add(32)?;
    Some(format!("\x1b[{encoded_code};{column};{row}M").into_bytes())
}

fn legacy_mouse_byte(value: u16) -> Option<u8> {
    u8::try_from(value.checked_add(32)?).ok()
}

fn push_utf8_mouse_value(bytes: &mut Vec<u8>, value: u16) -> Option<()> {
    let ch = char::from_u32(u32::from(value.checked_add(32)?))?;
    let mut buffer = [0; 4];
    bytes.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
    Some(())
}

const fn legacy_mouse_release_code(code: u16) -> u16 {
    3 + (code & !0b11)
}

const fn mouse_button_code(button: MouseButton) -> u16 {
    match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
    }
}

#[cfg(test)]
fn encode_key(key: KeyEvent) -> Option<Vec<u8>> {
    encode_key_with_mode(key, InputModes::default())
}

fn encode_key_with_mode(key: KeyEvent, modes: InputModes) -> Option<Vec<u8>> {
    if modes.win32_input_mode() {
        return encode_win32_key(key);
    }

    let alt = key.modifiers.contains(KeyModifiers::ALT);
    if key.kind != KeyEventKind::Press {
        return encode_kitty_event_key(key, modes.kitty_keyboard_flags());
    }

    if let Some(bytes) = encode_kitty_modifier_key(key, modes.kitty_keyboard_flags()) {
        return Some(bytes);
    }
    if let Some(bytes) = encode_modified_key(key) {
        return Some(bytes);
    }
    if let Some(bytes) = encode_kitty_keypad_key(key, modes.kitty_keyboard_flags()) {
        return Some(bytes);
    }
    if let Some(bytes) = encode_kitty_functional_key(key, modes.kitty_keyboard_flags()) {
        return Some(bytes);
    }
    if let Some(bytes) = encode_kitty_report_all_key(key, modes.kitty_keyboard_flags()) {
        return Some(bytes);
    }
    if let Some(bytes) = encode_kitty_disambiguated_key(key, modes.kitty_keyboard_flags()) {
        return Some(bytes);
    }
    if let Some(bytes) = encode_xterm_modify_other_key(key, modes.modify_other_keys()) {
        return Some(bytes);
    }
    if modes.application_keypad()
        && let Some(bytes) = encode_application_keypad_key(key)
    {
        return Some(bytes);
    }
    if modes.application_cursor_keys()
        && let Some(bytes) = encode_application_cursor_key(key)
    {
        return Some(bytes);
    }

    let terminal_key = match key.code {
        KeyCode::Char(character) if key.modifiers.contains(KeyModifiers::CONTROL) => {
            TerminalKey::Control(character)
        }
        KeyCode::Char(character) => TerminalKey::Text(character),
        KeyCode::Enter => TerminalKey::Enter,
        KeyCode::Backspace => TerminalKey::Backspace,
        KeyCode::Tab => TerminalKey::Tab,
        KeyCode::Esc => TerminalKey::Escape,
        KeyCode::Left => TerminalKey::Left,
        KeyCode::Right => TerminalKey::Right,
        KeyCode::Up => TerminalKey::Up,
        KeyCode::Down => TerminalKey::Down,
        KeyCode::Home => TerminalKey::Home,
        KeyCode::End => TerminalKey::End,
        KeyCode::Delete => TerminalKey::Delete,
        KeyCode::Insert => TerminalKey::Insert,
        KeyCode::PageUp => TerminalKey::PageUp,
        KeyCode::PageDown => TerminalKey::PageDown,
        KeyCode::BackTab => TerminalKey::BackTab,
        KeyCode::Menu => TerminalKey::Menu,
        KeyCode::F(key) => TerminalKey::Function(key),
        _ => return None,
    };

    let mut bytes = encode_terminal_key(terminal_key)?;
    if alt && matches!(key.code, KeyCode::Char(_)) {
        bytes.insert(0, 0x1b);
    }

    Some(bytes)
}

fn encode_win32_key(key: KeyEvent) -> Option<Vec<u8>> {
    let virtual_key = win32_virtual_key_code(key.code, key.state)
        .or_else(|| matches!(key.code, KeyCode::Char(_)).then_some(0))?;
    let unicode = if key.kind == KeyEventKind::Release {
        0
    } else {
        win32_unicode_char(key.code)
    };
    let key_down = u8::from(key.kind != KeyEventKind::Release);
    let control_key_state = win32_control_key_state(key.code, key.modifiers, key.state);

    Some(format!("\x1b[{virtual_key};0;{unicode};{key_down};{control_key_state};1_").into_bytes())
}

fn win32_unicode_char(code: KeyCode) -> u32 {
    match code {
        KeyCode::Char(character) => u32::from(character),
        KeyCode::Enter => u32::from('\r'),
        KeyCode::Tab => u32::from('\t'),
        KeyCode::Backspace => u32::from('\u{8}'),
        KeyCode::Esc => u32::from('\u{1b}'),
        _ => 0,
    }
}

fn win32_virtual_key_code(code: KeyCode, state: KeyEventState) -> Option<u16> {
    if state.contains(KeyEventState::KEYPAD)
        && let Some(virtual_key) = win32_keypad_virtual_key_code(code)
    {
        return Some(virtual_key);
    }

    match code {
        KeyCode::Char(character) => win32_virtual_key_code_from_character(character),
        KeyCode::Modifier(modifier) => win32_modifier_virtual_key_code(modifier),
        KeyCode::Backspace => Some(0x08),
        KeyCode::Tab | KeyCode::BackTab => Some(0x09),
        KeyCode::Enter => Some(0x0d),
        KeyCode::Esc => Some(0x1b),
        KeyCode::PageUp => Some(0x21),
        KeyCode::PageDown => Some(0x22),
        KeyCode::End => Some(0x23),
        KeyCode::Home => Some(0x24),
        KeyCode::Left => Some(0x25),
        KeyCode::Up => Some(0x26),
        KeyCode::Right => Some(0x27),
        KeyCode::Down => Some(0x28),
        KeyCode::Insert => Some(0x2d),
        KeyCode::Delete => Some(0x2e),
        KeyCode::F(number @ 1..=24) => Some(0x6f + u16::from(number)),
        _ => None,
    }
}

fn win32_modifier_virtual_key_code(modifier: ModifierKeyCode) -> Option<u16> {
    match modifier {
        ModifierKeyCode::LeftShift => Some(0xa0),
        ModifierKeyCode::RightShift => Some(0xa1),
        ModifierKeyCode::LeftControl => Some(0xa2),
        ModifierKeyCode::RightControl => Some(0xa3),
        ModifierKeyCode::LeftAlt => Some(0xa4),
        ModifierKeyCode::RightAlt => Some(0xa5),
        _ => None,
    }
}

fn win32_keypad_virtual_key_code(code: KeyCode) -> Option<u16> {
    match code {
        KeyCode::Char('0') => Some(0x60),
        KeyCode::Char('1') => Some(0x61),
        KeyCode::Char('2') => Some(0x62),
        KeyCode::Char('3') => Some(0x63),
        KeyCode::Char('4') => Some(0x64),
        KeyCode::Char('5') => Some(0x65),
        KeyCode::Char('6') => Some(0x66),
        KeyCode::Char('7') => Some(0x67),
        KeyCode::Char('8') => Some(0x68),
        KeyCode::Char('9') => Some(0x69),
        KeyCode::Char('*') => Some(0x6a),
        KeyCode::Char('+') => Some(0x6b),
        KeyCode::Char(',') => Some(0x6c),
        KeyCode::Char('-') => Some(0x6d),
        KeyCode::Char('.') => Some(0x6e),
        KeyCode::Char('/') => Some(0x6f),
        KeyCode::KeypadBegin => Some(0x0c),
        _ => None,
    }
}

fn win32_virtual_key_code_from_character(character: char) -> Option<u16> {
    match character {
        ' ' => return Some(0x20),
        ';' | ':' => return Some(0xba),
        '=' | '+' => return Some(0xbb),
        ',' | '<' => return Some(0xbc),
        '-' | '_' => return Some(0xbd),
        '.' | '>' => return Some(0xbe),
        '/' | '?' => return Some(0xbf),
        '`' | '~' => return Some(0xc0),
        '[' | '{' => return Some(0xdb),
        '\\' | '|' => return Some(0xdc),
        ']' | '}' => return Some(0xdd),
        '\'' | '"' => return Some(0xde),
        _ => {}
    }

    let character = character.to_ascii_uppercase();
    if character.is_ascii_alphabetic() || character.is_ascii_digit() {
        Some(character as u16)
    } else {
        None
    }
}

fn win32_control_key_state(code: KeyCode, modifiers: KeyModifiers, state: KeyEventState) -> u16 {
    let mut control_key_state = 0_u16;
    if modifiers.contains(KeyModifiers::ALT) {
        control_key_state |= match code {
            KeyCode::Modifier(ModifierKeyCode::RightAlt) => 0x0001,
            _ => 0x0002,
        };
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        control_key_state |= match code {
            KeyCode::Modifier(ModifierKeyCode::RightControl) => 0x0004,
            _ => 0x0008,
        };
    }
    if modifiers.contains(KeyModifiers::SHIFT) {
        control_key_state |= 0x0010;
    }
    if modifiers.intersects(KeyModifiers::SUPER | KeyModifiers::HYPER | KeyModifiers::META) {
        control_key_state |= 0x0100;
    }
    if state.contains(KeyEventState::NUM_LOCK) {
        control_key_state |= 0x0020;
    }
    if state.contains(KeyEventState::CAPS_LOCK) {
        control_key_state |= 0x0080;
    }
    control_key_state
}

fn encode_kitty_event_key(key: KeyEvent, kitty_keyboard_flags: u16) -> Option<Vec<u8>> {
    encode_kitty_modifier_key(key, kitty_keyboard_flags)
        .or_else(|| encode_kitty_keypad_key(key, kitty_keyboard_flags))
        .or_else(|| encode_kitty_functional_key(key, kitty_keyboard_flags))
        .or_else(|| encode_kitty_report_all_key(key, kitty_keyboard_flags))
        .or_else(|| encode_kitty_disambiguated_key(key, kitty_keyboard_flags))
}

fn encode_kitty_modifier_key(key: KeyEvent, kitty_keyboard_flags: u16) -> Option<Vec<u8>> {
    let event_type = kitty_event_type(key.kind, kitty_keyboard_flags);
    if kitty_keyboard_flags
        & (KITTY_KEYBOARD_DISAMBIGUATE | KITTY_KEYBOARD_REPORT_ALL | KITTY_KEYBOARD_REPORT_EVENTS)
        == 0
    {
        return None;
    }
    if key.kind == KeyEventKind::Press
        && kitty_keyboard_flags & (KITTY_KEYBOARD_DISAMBIGUATE | KITTY_KEYBOARD_REPORT_ALL) == 0
    {
        return None;
    }

    let key_code = kitty_local_modifier_key_code(key.code)?;
    Some(kitty_csi_u_key_with_event(
        key_code,
        kitty_modifier(key),
        event_type,
        None,
    ))
}

fn encode_kitty_keypad_key(key: KeyEvent, kitty_keyboard_flags: u16) -> Option<Vec<u8>> {
    let event_type = kitty_event_type(key.kind, kitty_keyboard_flags);
    if kitty_keyboard_flags
        & (KITTY_KEYBOARD_DISAMBIGUATE | KITTY_KEYBOARD_REPORT_ALL | KITTY_KEYBOARD_REPORT_EVENTS)
        == 0
    {
        return None;
    }
    if key.kind == KeyEventKind::Press
        && kitty_keyboard_flags & (KITTY_KEYBOARD_DISAMBIGUATE | KITTY_KEYBOARD_REPORT_ALL) == 0
    {
        return None;
    }

    let modifier = kitty_modifier(key);
    let key_code = kitty_local_keypad_code(key)?;
    if key.code == KeyCode::KeypadBegin {
        return Some(kitty_csi_tilde_key_with_event(
            key_code, modifier, event_type,
        ));
    }
    Some(kitty_csi_u_key_with_event(
        key_code, modifier, event_type, None,
    ))
}

fn encode_kitty_functional_key(key: KeyEvent, kitty_keyboard_flags: u16) -> Option<Vec<u8>> {
    let event_type = kitty_event_type(key.kind, kitty_keyboard_flags);
    if kitty_keyboard_flags
        & (KITTY_KEYBOARD_DISAMBIGUATE | KITTY_KEYBOARD_REPORT_ALL | KITTY_KEYBOARD_REPORT_EVENTS)
        == 0
    {
        return None;
    }
    if key.kind == KeyEventKind::Press
        && kitty_keyboard_flags & (KITTY_KEYBOARD_DISAMBIGUATE | KITTY_KEYBOARD_REPORT_ALL) == 0
    {
        return None;
    }

    let modifier = kitty_modifier(key);
    match key.code {
        KeyCode::Esc => Some(kitty_csi_u_key_with_event(27, modifier, event_type, None)),
        KeyCode::Enter if kitty_keyboard_flags & KITTY_KEYBOARD_REPORT_ALL != 0 => {
            let associated_text =
                associated_text_from_local_control_key(key.kind, kitty_keyboard_flags, 13);
            Some(kitty_csi_u_key_with_event(
                13,
                modifier,
                event_type,
                associated_text.as_deref(),
            ))
        }
        KeyCode::Tab | KeyCode::BackTab
            if kitty_reports_canonical_tab(key.code, key.modifiers, kitty_keyboard_flags) =>
        {
            Some(kitty_csi_u_key_with_event(9, modifier, event_type, None))
        }
        KeyCode::Backspace if kitty_keyboard_flags & KITTY_KEYBOARD_REPORT_ALL != 0 => {
            Some(kitty_csi_u_key_with_event(127, modifier, event_type, None))
        }
        KeyCode::Up => Some(kitty_csi_final_key_with_event(b'A', modifier, event_type)),
        KeyCode::Down => Some(kitty_csi_final_key_with_event(b'B', modifier, event_type)),
        KeyCode::Right => Some(kitty_csi_final_key_with_event(b'C', modifier, event_type)),
        KeyCode::Left => Some(kitty_csi_final_key_with_event(b'D', modifier, event_type)),
        KeyCode::End => Some(kitty_csi_final_key_with_event(b'F', modifier, event_type)),
        KeyCode::Home => Some(kitty_csi_final_key_with_event(b'H', modifier, event_type)),
        KeyCode::Insert => Some(kitty_csi_tilde_key_with_event(2, modifier, event_type)),
        KeyCode::Delete => Some(kitty_csi_tilde_key_with_event(3, modifier, event_type)),
        KeyCode::PageUp => Some(kitty_csi_tilde_key_with_event(5, modifier, event_type)),
        KeyCode::PageDown => Some(kitty_csi_tilde_key_with_event(6, modifier, event_type)),
        KeyCode::F(1) => Some(kitty_csi_final_key_with_event(b'P', modifier, event_type)),
        KeyCode::F(2) => Some(kitty_csi_final_key_with_event(b'Q', modifier, event_type)),
        KeyCode::F(3) => Some(kitty_csi_tilde_key_with_event(13, modifier, event_type)),
        KeyCode::F(4) => Some(kitty_csi_final_key_with_event(b'S', modifier, event_type)),
        KeyCode::F(key @ 5..=12) => {
            let number = match key {
                5 => 15,
                6 => 17,
                7 => 18,
                8 => 19,
                9 => 20,
                10 => 21,
                11 => 23,
                12 => 24,
                _ => unreachable!(),
            };
            Some(kitty_csi_tilde_key_with_event(number, modifier, event_type))
        }
        KeyCode::F(key @ 13..=35) => Some(kitty_csi_u_key_with_event(
            57376 + u32::from(key - 13),
            modifier,
            event_type,
            None,
        )),
        _ => kitty_local_pua_function_key_code(key.code)
            .map(|key_code| kitty_csi_u_key_with_event(key_code, modifier, event_type, None)),
    }
}

fn kitty_reports_canonical_tab(
    code: KeyCode,
    modifiers: KeyModifiers,
    kitty_keyboard_flags: u16,
) -> bool {
    if !matches!(code, KeyCode::Tab | KeyCode::BackTab) {
        return false;
    }
    if kitty_keyboard_flags & KITTY_KEYBOARD_REPORT_ALL != 0 {
        return true;
    }
    kitty_keyboard_flags & KITTY_KEYBOARD_DISAMBIGUATE != 0
        && modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT)
}

fn kitty_local_pua_function_key_code(code: KeyCode) -> Option<u32> {
    match code {
        KeyCode::CapsLock => Some(57358),
        KeyCode::ScrollLock => Some(57359),
        KeyCode::NumLock => Some(57360),
        KeyCode::PrintScreen => Some(57361),
        KeyCode::Pause => Some(57362),
        KeyCode::Menu => Some(57363),
        KeyCode::Media(media) => Some(kitty_local_media_key_code(media)),
        _ => None,
    }
}

fn kitty_local_media_key_code(media: MediaKeyCode) -> u32 {
    match media {
        MediaKeyCode::Play => 57428,
        MediaKeyCode::Pause => 57429,
        MediaKeyCode::PlayPause => 57430,
        MediaKeyCode::Reverse => 57431,
        MediaKeyCode::Stop => 57432,
        MediaKeyCode::FastForward => 57433,
        MediaKeyCode::Rewind => 57434,
        MediaKeyCode::TrackNext => 57435,
        MediaKeyCode::TrackPrevious => 57436,
        MediaKeyCode::Record => 57437,
        MediaKeyCode::LowerVolume => 57438,
        MediaKeyCode::RaiseVolume => 57439,
        MediaKeyCode::MuteVolume => 57440,
    }
}

fn kitty_local_modifier_key_code(code: KeyCode) -> Option<u32> {
    let KeyCode::Modifier(modifier) = code else {
        return None;
    };

    match modifier {
        ModifierKeyCode::LeftShift => Some(57441),
        ModifierKeyCode::LeftControl => Some(57442),
        ModifierKeyCode::LeftAlt => Some(57443),
        ModifierKeyCode::LeftSuper => Some(57444),
        ModifierKeyCode::LeftHyper => Some(57445),
        ModifierKeyCode::LeftMeta => Some(57446),
        ModifierKeyCode::RightShift => Some(57447),
        ModifierKeyCode::RightControl => Some(57448),
        ModifierKeyCode::RightAlt => Some(57449),
        ModifierKeyCode::RightSuper => Some(57450),
        ModifierKeyCode::RightHyper => Some(57451),
        ModifierKeyCode::RightMeta => Some(57452),
        ModifierKeyCode::IsoLevel3Shift => Some(57453),
        ModifierKeyCode::IsoLevel5Shift => Some(57454),
    }
}

fn encode_kitty_report_all_key(key: KeyEvent, kitty_keyboard_flags: u16) -> Option<Vec<u8>> {
    let report_all = kitty_keyboard_flags & KITTY_KEYBOARD_REPORT_ALL != 0;
    let report_text_event =
        kitty_keyboard_flags & KITTY_KEYBOARD_REPORT_EVENTS != 0 && key.kind != KeyEventKind::Press;
    if !report_all && !report_text_event {
        return None;
    }

    let key_code = match key.code {
        KeyCode::Char(character) => {
            kitty_local_report_all_key_code(character, key.modifiers, kitty_keyboard_flags)
        }
        KeyCode::Enter if report_all => 13.to_string(),
        KeyCode::Tab if report_all => 9.to_string(),
        KeyCode::Backspace if report_all => 127.to_string(),
        KeyCode::Esc if report_all => 27.to_string(),
        _ => return None,
    };
    Some(kitty_csi_u_key_with_event(
        key_code,
        kitty_modifier(key),
        kitty_event_type(key.kind, kitty_keyboard_flags),
        associated_text_from_local_key(key, kitty_keyboard_flags).as_deref(),
    ))
}

fn encode_kitty_disambiguated_key(key: KeyEvent, kitty_keyboard_flags: u16) -> Option<Vec<u8>> {
    if kitty_keyboard_flags & (KITTY_KEYBOARD_DISAMBIGUATE | KITTY_KEYBOARD_REPORT_ALL) == 0 {
        return None;
    }
    if !(key.modifiers.contains(KeyModifiers::CONTROL)
        || key.modifiers.contains(KeyModifiers::ALT)
        || key.modifiers.contains(KeyModifiers::SUPER)
        || key.modifiers.contains(KeyModifiers::HYPER)
        || key.modifiers.contains(KeyModifiers::META))
    {
        return None;
    }

    let KeyCode::Char(character) = key.code else {
        return None;
    };
    let key_code = if kitty_keyboard_flags & KITTY_KEYBOARD_ALTERNATE_KEYS != 0 {
        kitty_local_key_code(character, key.modifiers, kitty_keyboard_flags)
    } else {
        kitty_ascii_key_code(character)?.to_string()
    };
    let modifier = kitty_modifier(key)?;
    Some(kitty_csi_u_key_with_event(
        key_code,
        Some(modifier),
        kitty_event_type(key.kind, kitty_keyboard_flags),
        None,
    ))
}

fn kitty_ascii_key_code(character: char) -> Option<u32> {
    if let Some(key_code) = kitty_unshifted_ascii_key_code(character) {
        Some(key_code)
    } else if character.is_ascii_graphic() || character == ' ' {
        Some(u32::from(character))
    } else {
        None
    }
}

fn kitty_local_report_all_key_code(
    character: char,
    modifiers: KeyModifiers,
    kitty_keyboard_flags: u16,
) -> String {
    if character.is_ascii() {
        kitty_local_key_code(character, modifiers, kitty_keyboard_flags)
    } else {
        "0".to_owned()
    }
}

fn kitty_key_code(character: char) -> u32 {
    if character.is_ascii_alphabetic() {
        u32::from(character.to_ascii_lowercase())
    } else {
        u32::from(character)
    }
}

fn kitty_unshifted_ascii_key_code(character: char) -> Option<u32> {
    let unshifted = match character {
        'A'..='Z' => character.to_ascii_lowercase(),
        '~' => '`',
        '!' => '1',
        '@' => '2',
        '#' => '3',
        '$' => '4',
        '%' => '5',
        '^' => '6',
        '&' => '7',
        '*' => '8',
        '(' => '9',
        ')' => '0',
        '_' => '-',
        '+' => '=',
        '{' => '[',
        '}' => ']',
        '|' => '\\',
        ':' => ';',
        '"' => '\'',
        '<' => ',',
        '>' => '.',
        '?' => '/',
        _ => return None,
    };
    Some(u32::from(unshifted))
}

fn kitty_local_keypad_code(key: KeyEvent) -> Option<u32> {
    if !key.state.contains(KeyEventState::KEYPAD) && !matches!(key.code, KeyCode::KeypadBegin) {
        return None;
    }

    match key.code {
        KeyCode::Char('0') => Some(57399),
        KeyCode::Char('1') => Some(57400),
        KeyCode::Char('2') => Some(57401),
        KeyCode::Char('3') => Some(57402),
        KeyCode::Char('4') => Some(57403),
        KeyCode::Char('5') => Some(57404),
        KeyCode::Char('6') => Some(57405),
        KeyCode::Char('7') => Some(57406),
        KeyCode::Char('8') => Some(57407),
        KeyCode::Char('9') => Some(57408),
        KeyCode::Char('.') => Some(57409),
        KeyCode::Char('/') => Some(57410),
        KeyCode::Char('*') => Some(57411),
        KeyCode::Char('-') => Some(57412),
        KeyCode::Char('+') => Some(57413),
        KeyCode::Enter => Some(57414),
        KeyCode::Char('=') => Some(57415),
        KeyCode::Char(',') => Some(57416),
        KeyCode::Left => Some(57417),
        KeyCode::Right => Some(57418),
        KeyCode::Up => Some(57419),
        KeyCode::Down => Some(57420),
        KeyCode::PageUp => Some(57421),
        KeyCode::PageDown => Some(57422),
        KeyCode::Home => Some(57423),
        KeyCode::End => Some(57424),
        KeyCode::Insert => Some(57425),
        KeyCode::Delete => Some(57426),
        KeyCode::KeypadBegin => Some(57427),
        _ => None,
    }
}

fn kitty_local_key_code(
    character: char,
    modifiers: KeyModifiers,
    kitty_keyboard_flags: u16,
) -> String {
    let primary = if modifiers.contains(KeyModifiers::SHIFT) {
        kitty_unshifted_ascii_key_code(character).unwrap_or_else(|| kitty_key_code(character))
    } else {
        kitty_key_code(character)
    };
    if kitty_keyboard_flags & KITTY_KEYBOARD_ALTERNATE_KEYS == 0
        || !modifiers.contains(KeyModifiers::SHIFT)
    {
        return primary.to_string();
    }

    let shifted = u32::from(character);
    if shifted == primary {
        primary.to_string()
    } else {
        format!("{primary}:{shifted}")
    }
}

fn associated_text_from_local_key(key: KeyEvent, kitty_keyboard_flags: u16) -> Option<String> {
    if kitty_keyboard_flags & (KITTY_KEYBOARD_REPORT_ALL | KITTY_KEYBOARD_ASSOCIATED_TEXT)
        != (KITTY_KEYBOARD_REPORT_ALL | KITTY_KEYBOARD_ASSOCIATED_TEXT)
    {
        return None;
    }
    if key.kind == KeyEventKind::Release {
        return None;
    }

    let KeyCode::Char(character) = key.code else {
        return None;
    };
    associated_text_codepoints(std::iter::once(character))
}

fn associated_text_from_local_control_key(
    kind: KeyEventKind,
    kitty_keyboard_flags: u16,
    codepoint: u32,
) -> Option<String> {
    if kitty_keyboard_flags & (KITTY_KEYBOARD_REPORT_ALL | KITTY_KEYBOARD_ASSOCIATED_TEXT)
        != (KITTY_KEYBOARD_REPORT_ALL | KITTY_KEYBOARD_ASSOCIATED_TEXT)
    {
        return None;
    }
    if kind == KeyEventKind::Release {
        return None;
    }

    Some(codepoint.to_string())
}

fn associated_text_codepoints(characters: impl IntoIterator<Item = char>) -> Option<String> {
    let mut encoded = String::new();
    for character in characters {
        if character.is_control() {
            return None;
        }
        if !encoded.is_empty() {
            encoded.push(':');
        }
        encoded.push_str(&u32::from(character).to_string());
    }

    if encoded.is_empty() {
        None
    } else {
        Some(encoded)
    }
}

fn kitty_csi_u_key_with_event(
    key_code: impl std::fmt::Display,
    modifier: Option<u16>,
    event_type: Option<u8>,
    associated_text: Option<&str>,
) -> Vec<u8> {
    let modifier = match (modifier, event_type) {
        (Some(modifier), Some(event_type)) => Some(format!("{modifier}:{event_type}")),
        (Some(modifier), None) => Some(modifier.to_string()),
        (None, Some(event_type)) => Some(format!("1:{event_type}")),
        (None, None) => None,
    };

    match (modifier, associated_text) {
        (Some(modifier), Some(text)) => format!("\x1b[{key_code};{modifier};{text}u").into_bytes(),
        (Some(modifier), None) => format!("\x1b[{key_code};{modifier}u").into_bytes(),
        (None, Some(text)) => format!("\x1b[{key_code};;{text}u").into_bytes(),
        (None, None) => format!("\x1b[{key_code}u").into_bytes(),
    }
}

fn kitty_csi_final_key_with_event(
    final_byte: u8,
    modifier: Option<u16>,
    event_type: Option<u8>,
) -> Vec<u8> {
    match modifier {
        Some(modifier) => match event_type {
            Some(event_type) => {
                format!("\x1b[1;{}:{}{}", modifier, event_type, final_byte as char).into_bytes()
            }
            None => format!("\x1b[1;{}{}", modifier, final_byte as char).into_bytes(),
        },
        None => match event_type {
            Some(event_type) => {
                format!("\x1b[1;1:{}{}", event_type, final_byte as char).into_bytes()
            }
            None => vec![0x1b, b'[', final_byte],
        },
    }
}

fn kitty_csi_tilde_key_with_event(
    number: impl std::fmt::Display,
    modifier: Option<u16>,
    event_type: Option<u8>,
) -> Vec<u8> {
    match modifier {
        Some(modifier) => match event_type {
            Some(event_type) => format!("\x1b[{number};{modifier}:{event_type}~").into_bytes(),
            None => format!("\x1b[{number};{modifier}~").into_bytes(),
        },
        None => match event_type {
            Some(event_type) => format!("\x1b[{number};1:{event_type}~").into_bytes(),
            None => format!("\x1b[{number}~").into_bytes(),
        },
    }
}

fn reports_kitty_key_event_type(kind: KeyEventKind, kitty_keyboard_flags: u16) -> bool {
    kind != KeyEventKind::Press && kitty_event_type(kind, kitty_keyboard_flags).is_some()
}

fn kitty_event_type(kind: KeyEventKind, kitty_keyboard_flags: u16) -> Option<u8> {
    if kitty_keyboard_flags & KITTY_KEYBOARD_REPORT_EVENTS == 0 {
        return None;
    }

    match kind {
        KeyEventKind::Press => None,
        KeyEventKind::Repeat => Some(2),
        KeyEventKind::Release => Some(3),
    }
}

fn encode_xterm_modify_other_key(key: KeyEvent, modify_other_keys: u8) -> Option<Vec<u8>> {
    if modify_other_keys == 0 {
        return None;
    }
    let modifier = xterm_modifier(key.modifiers)?;
    let key_code = match key.code {
        KeyCode::Char(character) => u32::from(character),
        KeyCode::Enter => 13,
        KeyCode::Tab | KeyCode::BackTab => 9,
        KeyCode::Backspace => 127,
        KeyCode::Esc => 27,
        _ => return None,
    };

    Some(format!("\x1b[27;{modifier};{key_code}~").into_bytes())
}

fn encode_application_keypad_key(key: KeyEvent) -> Option<Vec<u8>> {
    if !key.modifiers.is_empty() {
        return None;
    }
    if !key.state.contains(KeyEventState::KEYPAD) && !matches!(key.code, KeyCode::KeypadBegin) {
        return None;
    }

    let final_byte = match key.code {
        KeyCode::Tab => b'I',
        KeyCode::Enter => b'M',
        KeyCode::Char(' ') => b' ',
        KeyCode::Char('*') => b'j',
        KeyCode::Char('+') => b'k',
        KeyCode::Char(',') => b'l',
        KeyCode::Char('-') => b'm',
        KeyCode::Char('.') => b'n',
        KeyCode::Char('/') => b'o',
        KeyCode::Char('0') => b'p',
        KeyCode::Char('1') => b'q',
        KeyCode::Char('2') => b'r',
        KeyCode::Char('3') => b's',
        KeyCode::Char('4') => b't',
        KeyCode::Char('5') => b'u',
        KeyCode::KeypadBegin => b'E',
        KeyCode::Char('6') => b'v',
        KeyCode::Char('7') => b'w',
        KeyCode::Char('8') => b'x',
        KeyCode::Char('9') => b'y',
        KeyCode::Char('=') => b'X',
        _ => return None,
    };

    Some(vec![0x1b, b'O', final_byte])
}

fn encode_application_cursor_key(key: KeyEvent) -> Option<Vec<u8>> {
    match key.code {
        KeyCode::Up => Some(b"\x1bOA".to_vec()),
        KeyCode::Down => Some(b"\x1bOB".to_vec()),
        KeyCode::Right => Some(b"\x1bOC".to_vec()),
        KeyCode::Left => Some(b"\x1bOD".to_vec()),
        _ => None,
    }
}

fn encode_modified_key(key: KeyEvent) -> Option<Vec<u8>> {
    let modifier = xterm_modifier(key.modifiers)?;

    match key.code {
        KeyCode::Left => Some(format!("\x1b[1;{modifier}D").into_bytes()),
        KeyCode::Right => Some(format!("\x1b[1;{modifier}C").into_bytes()),
        KeyCode::Up => Some(format!("\x1b[1;{modifier}A").into_bytes()),
        KeyCode::Down => Some(format!("\x1b[1;{modifier}B").into_bytes()),
        KeyCode::Home => Some(format!("\x1b[1;{modifier}H").into_bytes()),
        KeyCode::End => Some(format!("\x1b[1;{modifier}F").into_bytes()),
        KeyCode::Insert => Some(format!("\x1b[2;{modifier}~").into_bytes()),
        KeyCode::Delete => Some(format!("\x1b[3;{modifier}~").into_bytes()),
        KeyCode::PageUp => Some(format!("\x1b[5;{modifier}~").into_bytes()),
        KeyCode::PageDown => Some(format!("\x1b[6;{modifier}~").into_bytes()),
        KeyCode::F(1) => Some(format!("\x1b[1;{modifier}P").into_bytes()),
        KeyCode::F(2) => Some(format!("\x1b[1;{modifier}Q").into_bytes()),
        KeyCode::F(3) => Some(format!("\x1b[1;{modifier}R").into_bytes()),
        KeyCode::F(4) => Some(format!("\x1b[1;{modifier}S").into_bytes()),
        KeyCode::F(key) => modified_tilde_function_key(key, modifier),
        _ => None,
    }
}

fn modified_tilde_function_key(key: u8, modifier: u8) -> Option<Vec<u8>> {
    let base = match key {
        5 => 15,
        6 => 17,
        7 => 18,
        8 => 19,
        9 => 20,
        10 => 21,
        11 => 23,
        12 => 24,
        _ => return None,
    };

    Some(format!("\x1b[{base};{modifier}~").into_bytes())
}

fn xterm_modifier(modifiers: KeyModifiers) -> Option<u8> {
    let shift = modifiers.contains(KeyModifiers::SHIFT);
    let alt = modifiers.contains(KeyModifiers::ALT);
    let control = modifiers.contains(KeyModifiers::CONTROL);
    if !(shift || alt || control) {
        return None;
    }

    Some(1 + u8::from(shift) + u8::from(alt) * 2 + u8::from(control) * 4)
}

fn kitty_modifier(key: KeyEvent) -> Option<u16> {
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    let super_key = key.modifiers.contains(KeyModifiers::SUPER);
    let hyper = key.modifiers.contains(KeyModifiers::HYPER);
    let meta = key.modifiers.contains(KeyModifiers::META);
    let caps_lock = key.state.contains(KeyEventState::CAPS_LOCK);
    let num_lock = key.state.contains(KeyEventState::NUM_LOCK);
    if !(shift || alt || control || super_key || hyper || meta || caps_lock || num_lock) {
        return None;
    }

    Some(
        1 + u16::from(shift)
            + u16::from(alt) * 2
            + u16::from(control) * 4
            + u16::from(super_key) * 8
            + u16::from(hyper) * 16
            + u16::from(meta) * 32
            + u16::from(caps_lock) * 64
            + u16::from(num_lock) * 128,
    )
}

#[cfg(test)]
mod tests {
    use std::{
        io::Write,
        sync::mpsc,
        time::{Duration, Instant},
    };

    use crossterm::event::{
        Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MediaKeyCode,
        ModifierKeyCode, MouseButton, MouseEvent, MouseEventKind,
    };

    use crate::terminal_modes::{
        MouseInputMode, MouseProtocolMode, MouseReportingMode, TerminalModeChange,
        TerminalModeTracker,
    };
    use crate::terminal_queries::{
        KeyModifierOptions, KittyKeyboardApplyMode, KittyKeyboardMode, KittyKeyboardOperation,
    };

    use super::{
        InputModes, InputReporting, LocalCloseProgress, LocalMasterCloseOperation,
        LocalPtyCloseGroup, LocalTraceMarker, LocalWorkerReaper, Osc52Policy, RawMode,
        RawModeState, TerminalOutputFilter, begin_close_before_sender_drop, combine_local_result,
        encode_input_event, encode_key, join_local_worker_before,
        join_local_worker_before_with_reaper, resolve_local_size, spawn_input_thread_for_terminal,
    };

    const TEST_ASYNC_FINALITY_BUDGET: Duration = Duration::from_secs(5);

    fn wait_for_test_condition(description: &str, mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + TEST_ASYNC_FINALITY_BUDGET;
        while !condition() {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {description}"
            );
            std::thread::yield_now();
        }
    }

    fn wait_for_thread_finished(worker: &std::thread::JoinHandle<()>, description: &str) {
        wait_for_test_condition(description, || worker.is_finished());
    }

    #[test]
    fn local_worker_timeout_transfers_to_observable_reaper() {
        let reaper = LocalWorkerReaper::start("test-timeout-transfer");
        let (release_sender, release_receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let _ = release_receiver.recv();
        });

        let error =
            join_local_worker_before_with_reaper(worker, Instant::now(), "test worker", &reaper)
                .unwrap_err();
        assert!(error.to_string().contains("transferred to reaper"));
        assert_eq!(reaper.pending(), 1);

        release_sender.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while reaper.pending() != 0 && Instant::now() < deadline {
            std::thread::park_timeout(Duration::from_millis(2));
        }
        assert_eq!(reaper.pending(), 0);
    }

    #[test]
    fn local_worker_panic_is_observable() {
        let worker = std::thread::spawn(|| panic!("local worker panic seam"));
        wait_for_thread_finished(&worker, "local panic worker");

        let error = join_local_worker_before(worker, Instant::now(), "panic worker").unwrap_err();

        assert!(error.to_string().contains("panicked"));
    }

    #[test]
    fn permanently_blocked_reaper_job_does_not_starve_later_jobs() {
        let reaper = LocalWorkerReaper::start("test-active-set");
        let (permanent_sender, permanent_receiver) = mpsc::channel::<()>();
        let permanent = std::thread::spawn(move || {
            let _ = permanent_receiver.recv();
        });
        let permanent_error = join_local_worker_before_with_reaper(
            permanent,
            Instant::now(),
            "permanent worker",
            &reaper,
        )
        .unwrap_err();
        assert!(
            permanent_error
                .to_string()
                .contains("transferred to reaper")
        );

        let (release_sender, release_receiver) = mpsc::channel();
        let later = std::thread::spawn(move || {
            release_receiver.recv().unwrap();
        });
        join_local_worker_before_with_reaper(later, Instant::now(), "later worker", &reaper)
            .unwrap_err();
        release_sender.send(()).unwrap();

        let deadline = Instant::now() + Duration::from_secs(1);
        while reaper.pending() != 1 && Instant::now() < deadline {
            std::thread::park_timeout(Duration::from_millis(2));
        }
        assert_eq!(
            reaper.pending(),
            1,
            "later worker was starved by queue head"
        );
        permanent_sender.send(()).unwrap();
        let teardown_deadline = Instant::now() + Duration::from_secs(1);
        while reaper.pending() != 0 && Instant::now() < teardown_deadline {
            std::thread::park_timeout(Duration::from_millis(2));
        }
        assert_eq!(reaper.pending(), 0);
    }

    #[test]
    fn deferred_worker_panic_is_observable() {
        let reaper = LocalWorkerReaper::start("test-deferred-panic");
        let (release_sender, release_receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            release_receiver.recv().unwrap();
            panic!("deferred panic seam");
        });
        join_local_worker_before_with_reaper(
            worker,
            Instant::now(),
            "deferred panic worker",
            &reaper,
        )
        .unwrap_err();
        release_sender.send(()).unwrap();

        wait_for_test_condition("deferred panic worker reaping", || reaper.pending() == 0);
        let errors = reaper.take_errors();
        assert!(
            errors
                .iter()
                .any(|error| error.to_string().contains("deferred panic worker panicked")),
            "deferred panic was not reported: {errors:?}"
        );
    }

    #[test]
    fn disconnected_reaper_retains_worker_without_blocking_caller() {
        let reaper = LocalWorkerReaper::disconnected("test-fallback");
        let (release_sender, release_receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || release_receiver.recv().unwrap());
        let started = Instant::now();

        let error = join_local_worker_before_with_reaper(
            worker,
            Instant::now(),
            "fallback worker",
            &reaper,
        )
        .unwrap_err();

        assert!(started.elapsed() < Duration::from_millis(100));
        assert!(error.to_string().contains("fallback active set"));
        assert_eq!(reaper.pending(), 1);
        release_sender.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while reaper.pending() != 0 && Instant::now() < deadline {
            std::thread::park_timeout(Duration::from_millis(2));
        }
        assert_eq!(reaper.pending(), 0);
    }

    struct TestMasterClose {
        complete: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    impl LocalMasterCloseOperation for TestMasterClose {
        fn finish_before(&mut self, _deadline: Instant) -> LocalCloseProgress {
            if self.complete.load(std::sync::atomic::Ordering::Acquire) {
                LocalCloseProgress::Completed
            } else {
                LocalCloseProgress::Deferred
            }
        }
    }

    #[test]
    fn pty_close_group_transfers_close_reader_writer_and_channels_atomically() {
        let reaper = LocalWorkerReaper::disconnected("test-pty-group-fallback");
        let close_complete = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (release_reader, reader_release) = mpsc::channel();
        let (reader_done_sender, reader_done_receiver) = mpsc::channel();
        let reader_worker = std::thread::spawn(move || {
            reader_release.recv().unwrap();
            reader_done_sender.send(Ok(())).unwrap();
        });
        let (release_writer, writer_release) = mpsc::channel();
        let (writer_done_sender, writer_done_receiver) = mpsc::channel();
        let writer_worker = std::thread::spawn(move || {
            writer_release.recv().unwrap();
            writer_done_sender.send(Ok(())).unwrap();
        });
        let group = LocalPtyCloseGroup::new(
            Box::new(TestMasterClose {
                complete: std::sync::Arc::clone(&close_complete),
            }),
            reader_worker,
            writer_worker,
            reader_done_receiver,
            writer_done_receiver,
            false,
            false,
        );

        let transfer = reaper.enqueue_group(group);
        assert!(transfer.is_fallback());
        assert_eq!(reaper.pending(), 1);
        release_writer.send(()).unwrap();
        assert_eq!(
            reaper.pending(),
            1,
            "partial group completion lost ownership"
        );

        close_complete.store(true, std::sync::atomic::Ordering::Release);
        release_reader.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while reaper.pending() != 0 && Instant::now() < deadline {
            std::thread::park_timeout(Duration::from_millis(2));
        }
        assert_eq!(reaper.pending(), 0);
        assert!(reaper.take_errors().is_empty());
    }

    #[test]
    fn local_close_flag_is_set_before_external_sender_drop() {
        struct DropProbe {
            closing: std::sync::Arc<std::sync::atomic::AtomicBool>,
            observed: std::sync::Arc<std::sync::atomic::AtomicBool>,
        }

        impl Drop for DropProbe {
            fn drop(&mut self) {
                self.observed.store(
                    self.closing.load(std::sync::atomic::Ordering::Acquire),
                    std::sync::atomic::Ordering::Release,
                );
            }
        }

        let closing = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let observed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let probe = DropProbe {
            closing: std::sync::Arc::clone(&closing),
            observed: std::sync::Arc::clone(&observed),
        };

        begin_close_before_sender_drop(
            || closing.store(true, std::sync::atomic::Ordering::Release),
            probe,
        );

        assert!(observed.load(std::sync::atomic::Ordering::Acquire));
    }

    #[derive(Debug)]
    struct TestCloseFailure;

    impl std::fmt::Display for TestCloseFailure {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("deferred close failure seam")
        }
    }

    impl std::error::Error for TestCloseFailure {}

    struct FailingMasterClose;

    impl LocalMasterCloseOperation for FailingMasterClose {
        fn finish_before(&mut self, _deadline: Instant) -> LocalCloseProgress {
            LocalCloseProgress::Failed(Box::new(TestCloseFailure))
        }
    }

    #[test]
    fn deferred_pty_close_failure_preserves_its_source() {
        let reaper = LocalWorkerReaper::disconnected("test-pty-group-error");
        let (reader_done_sender, reader_done_receiver) = mpsc::channel();
        let reader_worker = std::thread::spawn(move || reader_done_sender.send(Ok(())).unwrap());
        let (writer_done_sender, writer_done_receiver) = mpsc::channel();
        let writer_worker = std::thread::spawn(move || writer_done_sender.send(Ok(())).unwrap());
        reaper.enqueue_group(LocalPtyCloseGroup::new(
            Box::new(FailingMasterClose),
            reader_worker,
            writer_worker,
            reader_done_receiver,
            writer_done_receiver,
            false,
            false,
        ));

        let deadline = Instant::now() + Duration::from_secs(1);
        while reaper.pending() != 0 && Instant::now() < deadline {
            std::thread::park_timeout(Duration::from_millis(2));
        }
        let errors = reaper.take_errors();
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0]
                .get_ref()
                .and_then(|source| source.downcast_ref::<TestCloseFailure>())
                .is_some(),
            "deferred close error lost its structured source: {errors:?}"
        );
    }

    struct RetainedMasterClose;

    impl LocalMasterCloseOperation for RetainedMasterClose {
        fn finish_before(&mut self, _deadline: Instant) -> LocalCloseProgress {
            LocalCloseProgress::Retained
        }
    }

    #[test]
    fn retained_pty_close_is_observable() {
        let owner = LocalWorkerReaper::disconnected("test-pty-group-retained");
        let (reader_done_sender, reader_done_receiver) = mpsc::channel();
        let reader_worker = std::thread::spawn(move || reader_done_sender.send(Ok(())).unwrap());
        let (writer_done_sender, writer_done_receiver) = mpsc::channel();
        let writer_worker = std::thread::spawn(move || writer_done_sender.send(Ok(())).unwrap());
        owner.enqueue_group(LocalPtyCloseGroup::new(
            Box::new(RetainedMasterClose),
            reader_worker,
            writer_worker,
            reader_done_receiver,
            writer_done_receiver,
            false,
            false,
        ));

        let deadline = Instant::now() + Duration::from_secs(1);
        while owner.pending() != 0 && Instant::now() < deadline {
            std::thread::park_timeout(Duration::from_millis(2));
        }
        let errors = owner.take_errors();
        assert!(
            errors.iter().any(|error| error
                .to_string()
                .contains("retained by the PTY global reaper")),
            "retained close was silent: {errors:?}"
        );
    }

    #[test]
    fn nonterminal_input_does_not_spawn_a_worker() {
        let (pty_sender, _pty_receiver) = mpsc::channel();
        let (control_sender, _control_receiver) = mpsc::channel();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        let (worker, returned_stop) = spawn_input_thread_for_terminal(
            false,
            pty_sender,
            control_sender,
            InputReporting::default(),
            std::sync::Arc::clone(&stop),
        );

        assert!(worker.is_none());
        assert!(std::sync::Arc::ptr_eq(&stop, &returned_stop));
    }

    #[test]
    fn nonterminal_input_does_not_enable_raw_mode() {
        let raw_mode = RawMode::enable_for_terminal(false).unwrap();

        assert_eq!(raw_mode.state, RawModeState::Disabled);
        assert!(!raw_mode.bracketed_paste);
        assert!(!raw_mode.mouse_capture);
        assert!(!raw_mode.focus_change);
    }

    #[test]
    fn local_cleanup_keeps_primary_error_ahead_of_secondary_errors() {
        #[derive(Debug)]
        struct TypedPrimary;

        impl std::fmt::Display for TypedPrimary {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("primary failure")
            }
        }

        impl std::error::Error for TypedPrimary {}

        let primary: Result<(), Box<dyn std::error::Error>> = Err(Box::new(TypedPrimary));

        let error = combine_local_result(primary, vec![std::io::Error::other("secondary failure")])
            .unwrap_err();
        let message = error.to_string();

        assert!(message.starts_with("primary failure"));
        assert!(message.contains("secondary cleanup failures: secondary failure"));
        assert!(
            error
                .source()
                .and_then(|source| source.downcast_ref::<TypedPrimary>())
                .is_some(),
            "structured composite lost the primary source type"
        );
    }

    #[test]
    fn trace_marker_is_streaming_and_reports_only_the_first_match() {
        let mut marker = LocalTraceMarker::new(b"needle".to_vec()).unwrap();

        assert!(!marker.feed(b"noise nee"));
        assert!(marker.feed(b"dle suffix needle"));
        assert!(!marker.feed(b"needle"));
    }

    #[test]
    fn encodes_text_input_as_utf8() {
        assert_eq!(
            encode_key(KeyEvent::new(KeyCode::Char('中'), KeyModifiers::NONE)).unwrap(),
            "中".as_bytes()
        );
    }

    #[test]
    fn encodes_enter_for_shells() {
        assert_eq!(
            encode_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)).unwrap(),
            b"\r"
        );
    }

    #[test]
    fn encodes_control_c() {
        assert_eq!(
            encode_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)).unwrap(),
            vec![3]
        );
    }

    #[test]
    fn encodes_legacy_control_digit_keys() {
        assert_eq!(
            encode_key(KeyEvent::new(KeyCode::Char('0'), KeyModifiers::CONTROL)).unwrap(),
            b"0"
        );
        assert_eq!(
            encode_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::CONTROL)).unwrap(),
            vec![0]
        );
        assert_eq!(
            encode_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::CONTROL)).unwrap(),
            vec![0x1b]
        );
        assert_eq!(
            encode_key(KeyEvent::new(KeyCode::Char('8'), KeyModifiers::CONTROL)).unwrap(),
            vec![0x7f]
        );
    }

    #[test]
    fn encodes_arrow_keys_as_escape_sequences() {
        assert_eq!(
            encode_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)).unwrap(),
            b"\x1b[A"
        );
    }

    #[test]
    fn encodes_application_cursor_keys_when_enabled() {
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
                InputModes::default().with_application_cursor_keys(true),
            )
            .unwrap(),
            b"\x1bOA"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
                InputModes::default().with_application_cursor_keys(true),
            )
            .unwrap(),
            b"\x1bOC"
        );
    }

    #[test]
    fn encodes_keypad_keys_when_application_keypad_is_enabled() {
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Char('5'),
                    KeyModifiers::NONE,
                    KeyEventKind::Press,
                    KeyEventState::KEYPAD
                )),
                InputModes::default().with_application_keypad(true),
            )
            .unwrap(),
            b"\x1bOu"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::KeypadBegin, KeyModifiers::NONE)),
                InputModes::default().with_application_keypad(true),
            )
            .unwrap(),
            b"\x1bOE"
        );
    }

    #[test]
    fn encodes_modified_navigation_keys_as_xterm_sequences() {
        assert_eq!(
            encode_key(KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL)).unwrap(),
            b"\x1b[1;5D"
        );
        assert_eq!(
            encode_key(KeyEvent::new(
                KeyCode::Right,
                KeyModifiers::SHIFT | KeyModifiers::ALT
            ))
            .unwrap(),
            b"\x1b[1;4C"
        );
        assert_eq!(
            encode_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::CONTROL)).unwrap(),
            b"\x1b[3;5~"
        );
        assert_eq!(
            encode_key(KeyEvent::new(KeyCode::F(5), KeyModifiers::SHIFT)).unwrap(),
            b"\x1b[15;2~"
        );
    }

    #[test]
    fn encodes_backtab_and_function_keys_as_escape_sequences() {
        assert_eq!(
            encode_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT)).unwrap(),
            b"\x1b[Z"
        );
        assert_eq!(
            encode_key(KeyEvent::new(KeyCode::F(12), KeyModifiers::NONE)).unwrap(),
            b"\x1b[24~"
        );
    }

    #[test]
    fn encodes_menu_key_as_legacy_functional_sequence() {
        assert_eq!(
            encode_key(KeyEvent::new(KeyCode::Menu, KeyModifiers::NONE)).unwrap(),
            b"\x1b[29~"
        );
    }

    #[test]
    fn encodes_alt_text_with_escape_prefix() {
        assert_eq!(
            encode_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT)).unwrap(),
            b"\x1bx"
        );
    }

    #[test]
    fn encodes_win32_input_mode_key_release_events() {
        let modes = InputModes::default().with_win32_input_mode(true);

        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Char('a'),
                    KeyModifiers::NONE,
                    KeyEventKind::Release,
                    KeyEventState::empty(),
                )),
                modes,
            )
            .unwrap(),
            b"\x1b[65;0;0;0;0;1_"
        );
    }

    #[test]
    fn encodes_win32_input_mode_oem_punctuation_keys() {
        let modes = InputModes::default().with_win32_input_mode(true);

        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Char('+'), KeyModifiers::SHIFT)),
                modes,
            )
            .unwrap(),
            b"\x1b[187;0;43;1;16;1_"
        );
    }

    #[test]
    fn encodes_win32_input_mode_unicode_text_without_a_virtual_key() {
        let event = Event::Key(KeyCode::Char('终').into());
        assert_eq!(
            encode_input_event(event, InputModes::default().with_win32_input_mode(true)).unwrap(),
            b"\x1b[0;0;32456;1;0;1_"
        );
    }

    #[test]
    fn encodes_win32_input_mode_keypad_virtual_keys() {
        let modes = InputModes::default().with_win32_input_mode(true);

        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Char('1'),
                    KeyModifiers::NONE,
                    KeyEventKind::Press,
                    KeyEventState::KEYPAD,
                )),
                modes,
            )
            .unwrap(),
            b"\x1b[97;0;49;1;0;1_"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Char('+'),
                    KeyModifiers::NONE,
                    KeyEventKind::Press,
                    KeyEventState::KEYPAD,
                )),
                modes,
            )
            .unwrap(),
            b"\x1b[107;0;43;1;0;1_"
        );
    }

    #[test]
    fn encodes_win32_input_mode_extended_function_keys() {
        let modes = InputModes::default().with_win32_input_mode(true);

        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::F(13), KeyModifiers::NONE)),
                modes,
            )
            .unwrap(),
            b"\x1b[124;0;0;1;0;1_"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::F(24), KeyModifiers::SHIFT)),
                modes,
            )
            .unwrap(),
            b"\x1b[135;0;0;1;16;1_"
        );
    }

    #[test]
    fn encodes_win32_input_mode_modifier_virtual_keys() {
        let modes = InputModes::default().with_win32_input_mode(true);

        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Modifier(ModifierKeyCode::LeftShift),
                    KeyModifiers::SHIFT,
                    KeyEventKind::Press,
                    KeyEventState::empty(),
                )),
                modes,
            )
            .unwrap(),
            b"\x1b[160;0;0;1;16;1_"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Modifier(ModifierKeyCode::RightControl),
                    KeyModifiers::CONTROL,
                    KeyEventKind::Press,
                    KeyEventState::empty(),
                )),
                modes,
            )
            .unwrap(),
            b"\x1b[163;0;0;1;4;1_"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Modifier(ModifierKeyCode::RightAlt),
                    KeyModifiers::ALT,
                    KeyEventKind::Press,
                    KeyEventState::empty(),
                )),
                modes,
            )
            .unwrap(),
            b"\x1b[165;0;0;1;1;1_"
        );
    }

    #[test]
    fn win32_input_mode_takes_precedence_over_kitty_keyboard() {
        let modes = InputModes::default()
            .with_win32_input_mode(true)
            .with_kitty_keyboard_flags(1);

        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Char('i'),
                    KeyModifiers::CONTROL,
                    KeyEventKind::Press,
                    KeyEventState::empty(),
                )),
                modes,
            )
            .unwrap(),
            b"\x1b[73;0;105;1;8;1_"
        );
    }

    #[test]
    fn input_reporting_snapshot_includes_win32_input_mode() {
        let reporting = InputReporting::default();
        reporting.set_win32_input_mode(true);

        assert!(reporting.snapshot().win32_input_mode());
    }

    #[test]
    fn encodes_kitty_disambiguated_ascii_keys_when_enabled() {
        let modes = InputModes::default().with_kitty_keyboard_flags(1);

        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::CONTROL)),
                modes
            )
            .unwrap(),
            b"\x1b[105;5u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(
                    KeyCode::Char('i'),
                    KeyModifiers::CONTROL | KeyModifiers::SHIFT
                )),
                modes
            )
            .unwrap(),
            b"\x1b[105;6u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(
                    KeyCode::Char('+'),
                    KeyModifiers::CONTROL | KeyModifiers::SHIFT
                )),
                modes
            )
            .unwrap(),
            b"\x1b[61;6u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::ALT)),
                modes
            )
            .unwrap(),
            b"\x1b[105;3u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)),
                modes
            )
            .unwrap(),
            b"i"
        );
    }

    #[test]
    fn encodes_kitty_extended_modifier_bits_when_enabled() {
        let disambiguate = InputModes::default().with_kitty_keyboard_flags(1);
        let report_all = InputModes::default().with_kitty_keyboard_flags(8);

        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Char('i'),
                    KeyModifiers::SUPER,
                    KeyEventKind::Press,
                    KeyEventState::empty()
                )),
                disambiguate
            )
            .unwrap(),
            b"\x1b[105;9u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Char('i'),
                    KeyModifiers::HYPER | KeyModifiers::META,
                    KeyEventKind::Press,
                    KeyEventState::empty()
                )),
                disambiguate
            )
            .unwrap(),
            b"\x1b[105;49u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Up,
                    KeyModifiers::NONE,
                    KeyEventKind::Press,
                    KeyEventState::NUM_LOCK
                )),
                disambiguate
            )
            .unwrap(),
            b"\x1b[1;129A"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Char('a'),
                    KeyModifiers::NONE,
                    KeyEventKind::Press,
                    KeyEventState::CAPS_LOCK | KeyEventState::NUM_LOCK
                )),
                report_all
            )
            .unwrap(),
            b"\x1b[97;193u"
        );
    }

    #[test]
    fn encodes_kitty_modifier_keys_when_enabled() {
        let disambiguate = InputModes::default().with_kitty_keyboard_flags(1);
        let events = InputModes::default().with_kitty_keyboard_flags(2);

        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Modifier(ModifierKeyCode::LeftShift),
                    KeyModifiers::SHIFT,
                    KeyEventKind::Press,
                    KeyEventState::empty()
                )),
                disambiguate
            )
            .unwrap(),
            b"\x1b[57441;2u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Modifier(ModifierKeyCode::RightSuper),
                    KeyModifiers::SUPER,
                    KeyEventKind::Press,
                    KeyEventState::empty()
                )),
                disambiguate
            )
            .unwrap(),
            b"\x1b[57450;9u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Modifier(ModifierKeyCode::LeftHyper),
                    KeyModifiers::HYPER,
                    KeyEventKind::Press,
                    KeyEventState::empty()
                )),
                disambiguate
            )
            .unwrap(),
            b"\x1b[57445;17u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Modifier(ModifierKeyCode::IsoLevel3Shift),
                    KeyModifiers::NONE,
                    KeyEventKind::Press,
                    KeyEventState::empty()
                )),
                disambiguate
            )
            .unwrap(),
            b"\x1b[57453u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Modifier(ModifierKeyCode::RightMeta),
                    KeyModifiers::NONE,
                    KeyEventKind::Release,
                    KeyEventState::empty()
                )),
                events
            )
            .unwrap(),
            b"\x1b[57452;1:3u"
        );
    }

    #[test]
    fn encodes_kitty_report_all_ascii_and_basic_functional_keys_when_enabled() {
        let modes = InputModes::default().with_kitty_keyboard_flags(8);

        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
                modes
            )
            .unwrap(),
            b"\x1b[97u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT)),
                modes
            )
            .unwrap(),
            b"\x1b[97;2u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::CONTROL)),
                modes
            )
            .unwrap(),
            b"\x1b[105;5u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
                modes
            )
            .unwrap(),
            b"\x1b[13u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
                modes
            )
            .unwrap(),
            b"\x1b[9u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
                modes
            )
            .unwrap(),
            b"\x1b[127u"
        );
    }

    #[test]
    fn encodes_kitty_associated_text_when_enabled() {
        let modes = InputModes::default().with_kitty_keyboard_flags(24);

        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT)),
                modes
            )
            .unwrap(),
            b"\x1b[97;2;65u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
                modes
            )
            .unwrap(),
            b"\x1b[97;;97u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Char('å'), KeyModifiers::NONE)),
                modes
            )
            .unwrap(),
            b"\x1b[0;;229u"
        );

        let event_modes = InputModes::default().with_kitty_keyboard_flags(26);
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Char('a'),
                    KeyModifiers::NONE,
                    KeyEventKind::Repeat,
                    KeyEventState::empty()
                )),
                event_modes
            )
            .unwrap(),
            b"\x1b[97;1:2;97u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Char('a'),
                    KeyModifiers::NONE,
                    KeyEventKind::Release,
                    KeyEventState::empty()
                )),
                event_modes
            )
            .unwrap(),
            b"\x1b[97;1:3u"
        );
    }

    #[test]
    fn encodes_kitty_enter_associated_text_when_enabled() {
        let modes = InputModes::default().with_kitty_keyboard_flags(24);

        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL)),
                modes
            )
            .unwrap(),
            b"\x1b[13;5;13u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(
                    KeyCode::Enter,
                    KeyModifiers::CONTROL | KeyModifiers::SHIFT
                )),
                modes
            )
            .unwrap(),
            b"\x1b[13;6;13u"
        );

        let event_modes = InputModes::default().with_kitty_keyboard_flags(26);
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Enter,
                    KeyModifiers::CONTROL,
                    KeyEventKind::Release,
                    KeyEventState::empty()
                )),
                event_modes
            )
            .unwrap(),
            b"\x1b[13;5:3u"
        );
    }

    #[test]
    fn encodes_kitty_shifted_alternate_key_when_enabled() {
        let report_all_alternate = InputModes::default().with_kitty_keyboard_flags(12);
        let disambiguate_alternate = InputModes::default().with_kitty_keyboard_flags(5);

        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT)),
                report_all_alternate
            )
            .unwrap(),
            b"\x1b[97:65;2u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(
                    KeyCode::Char('A'),
                    KeyModifiers::CONTROL | KeyModifiers::SHIFT
                )),
                disambiguate_alternate
            )
            .unwrap(),
            b"\x1b[97:65;6u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(
                    KeyCode::Char('+'),
                    KeyModifiers::CONTROL | KeyModifiers::SHIFT
                )),
                disambiguate_alternate
            )
            .unwrap(),
            b"\x1b[61:43;6u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::SHIFT)),
                report_all_alternate
            )
            .unwrap(),
            b"\x1b[49:33;2u"
        );

        let report_all = InputModes::default().with_kitty_keyboard_flags(8);
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::SHIFT)),
                report_all
            )
            .unwrap(),
            b"\x1b[49;2u"
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn encodes_kitty_canonical_functional_keys_when_enabled() {
        let disambiguate = InputModes::default().with_kitty_keyboard_flags(1);
        let report_all = InputModes::default().with_kitty_keyboard_flags(8);

        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE)),
                disambiguate
            )
            .unwrap(),
            b"\x1b[P"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
                disambiguate
            )
            .unwrap(),
            b"\x1b[27u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
                disambiguate.with_application_cursor_keys(true)
            )
            .unwrap(),
            b"\x1b[A"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::F(13), KeyModifiers::NONE)),
                report_all
            )
            .unwrap(),
            b"\x1b[57376u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE)),
                disambiguate
            )
            .unwrap(),
            b"\x1b[13~"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::CapsLock, KeyModifiers::NONE)),
                disambiguate
            )
            .unwrap(),
            b"\x1b[57358u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::ScrollLock, KeyModifiers::NONE)),
                disambiguate
            )
            .unwrap(),
            b"\x1b[57359u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::NumLock, KeyModifiers::NONE)),
                disambiguate
            )
            .unwrap(),
            b"\x1b[57360u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::PrintScreen, KeyModifiers::NONE)),
                disambiguate
            )
            .unwrap(),
            b"\x1b[57361u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Pause, KeyModifiers::NONE)),
                disambiguate
            )
            .unwrap(),
            b"\x1b[57362u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Menu, KeyModifiers::NONE)),
                disambiguate
            )
            .unwrap(),
            b"\x1b[57363u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(
                    KeyCode::Media(MediaKeyCode::Play),
                    KeyModifiers::NONE
                )),
                disambiguate
            )
            .unwrap(),
            b"\x1b[57428u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(
                    KeyCode::Media(MediaKeyCode::Pause),
                    KeyModifiers::NONE
                )),
                disambiguate
            )
            .unwrap(),
            b"\x1b[57429u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(
                    KeyCode::Media(MediaKeyCode::FastForward),
                    KeyModifiers::NONE
                )),
                disambiguate
            )
            .unwrap(),
            b"\x1b[57433u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(
                    KeyCode::Media(MediaKeyCode::TrackNext),
                    KeyModifiers::NONE
                )),
                disambiguate
            )
            .unwrap(),
            b"\x1b[57435u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(
                    KeyCode::Media(MediaKeyCode::MuteVolume),
                    KeyModifiers::NONE
                )),
                disambiguate
            )
            .unwrap(),
            b"\x1b[57440u"
        );
    }

    #[test]
    fn encodes_kitty_keypad_keys_when_enabled() {
        let disambiguate = InputModes::default().with_kitty_keyboard_flags(1);
        let report_all = InputModes::default().with_kitty_keyboard_flags(8);

        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Enter,
                    KeyModifiers::NONE,
                    KeyEventKind::Press,
                    KeyEventState::KEYPAD
                )),
                disambiguate
            )
            .unwrap(),
            b"\x1b[57414u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Char('5'),
                    KeyModifiers::NONE,
                    KeyEventKind::Press,
                    KeyEventState::KEYPAD
                )),
                report_all
            )
            .unwrap(),
            b"\x1b[57404u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::KeypadBegin, KeyModifiers::NONE)),
                disambiguate
            )
            .unwrap(),
            b"\x1b[57427~"
        );
    }

    #[test]
    fn encodes_kitty_event_types_when_enabled() {
        let event_types = InputModes::default().with_kitty_keyboard_flags(2);
        let disambiguate_events = InputModes::default().with_kitty_keyboard_flags(3);
        let report_all_events = InputModes::default().with_kitty_keyboard_flags(10);

        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Up,
                    KeyModifiers::NONE,
                    KeyEventKind::Repeat,
                    KeyEventState::empty()
                )),
                event_types
            )
            .unwrap(),
            b"\x1b[1;1:2A"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Char('i'),
                    KeyModifiers::CONTROL,
                    KeyEventKind::Release,
                    KeyEventState::empty()
                )),
                disambiguate_events
            )
            .unwrap(),
            b"\x1b[105;5:3u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Char('a'),
                    KeyModifiers::NONE,
                    KeyEventKind::Release,
                    KeyEventState::empty()
                )),
                report_all_events
            )
            .unwrap(),
            b"\x1b[97;1:3u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Char('a'),
                    KeyModifiers::NONE,
                    KeyEventKind::Release,
                    KeyEventState::empty()
                )),
                event_types
            )
            .unwrap(),
            b"\x1b[97;1:3u"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Char('+'),
                    KeyModifiers::CONTROL | KeyModifiers::SHIFT,
                    KeyEventKind::Release,
                    KeyEventState::empty()
                )),
                InputModes::default().with_kitty_keyboard_flags(6)
            )
            .unwrap(),
            b"\x1b[61:43;6:3u"
        );
        assert!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Enter,
                    KeyModifiers::NONE,
                    KeyEventKind::Release,
                    KeyEventState::empty()
                )),
                disambiguate_events
            )
            .is_none()
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new_with_kind_and_state(
                    KeyCode::Enter,
                    KeyModifiers::NONE,
                    KeyEventKind::Release,
                    KeyEventState::empty()
                )),
                report_all_events
            )
            .unwrap(),
            b"\x1b[13;1:3u"
        );
    }

    #[test]
    fn encodes_kitty_ctrl_shift_tab_as_canonical_tab_when_disambiguated() {
        let disambiguate = InputModes::default().with_kitty_keyboard_flags(1);

        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(
                    KeyCode::BackTab,
                    KeyModifiers::CONTROL | KeyModifiers::SHIFT,
                )),
                disambiguate,
            )
            .unwrap(),
            b"\x1b[9;6u"
        );
    }

    #[test]
    fn encodes_xterm_modify_other_keys_when_enabled() {
        let modes = InputModes::default().with_modify_other_keys(2);

        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL)),
                modes
            )
            .unwrap(),
            b"\x1b[27;5;13~"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(
                    KeyCode::Char('I'),
                    KeyModifiers::CONTROL | KeyModifiers::SHIFT
                )),
                modes
            )
            .unwrap(),
            b"\x1b[27;6;73~"
        );
        assert_eq!(
            encode_input_event(
                Event::Key(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::ALT)),
                modes
            )
            .unwrap(),
            b"\x1b[27;3;46~"
        );
    }

    #[test]
    fn input_reporting_snapshot_includes_kitty_keyboard_flags() {
        let reporting = InputReporting::default();

        reporting.set_kitty_keyboard_flags(1);

        assert_eq!(reporting.snapshot().kitty_keyboard_flags(), 1);
    }

    #[test]
    fn encodes_paste_event_as_utf8_bytes() {
        assert_eq!(
            encode_input_event(Event::Paste("line 1\n中".to_owned()), InputModes::default())
                .unwrap(),
            "line 1\n中".as_bytes()
        );
    }

    #[test]
    fn encodes_paste_event_as_bracketed_paste_when_enabled() {
        assert_eq!(
            encode_input_event(
                Event::Paste("line 1\n中".to_owned()),
                InputModes::default().with_bracketed_paste(true)
            )
            .unwrap(),
            b"\x1b[200~line 1\n\xe4\xb8\xad\x1b[201~"
        );
    }

    #[test]
    fn ignores_mouse_events_unless_enabled() {
        assert!(encode_input_event(left_mouse_down(), InputModes::default()).is_none());
    }

    #[test]
    fn encodes_mouse_events_as_sgr_sequences_when_enabled() {
        let modes = InputModes::default().with_mouse_input_mode(MouseInputMode::new(
            MouseReportingMode::Normal,
            MouseProtocolMode::Sgr,
        ));

        assert_eq!(
            encode_input_event(left_mouse_down(), modes).unwrap(),
            b"\x1b[<0;1;2M"
        );
        assert_eq!(
            encode_input_event(left_mouse_release(), modes).unwrap(),
            b"\x1b[<0;1;2m"
        );
        assert_eq!(
            encode_input_event(
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::ScrollDown,
                    column: 4,
                    row: 5,
                    modifiers: KeyModifiers::CONTROL,
                }),
                modes
            )
            .unwrap(),
            b"\x1b[<81;5;6M"
        );
    }

    #[test]
    fn encodes_mouse_events_as_sgr_pixels_sequences_with_cell_fallback() {
        let modes = InputModes::default().with_mouse_input_mode(MouseInputMode::new(
            MouseReportingMode::Normal,
            MouseProtocolMode::SgrPixels,
        ));

        assert_eq!(
            modes.mouse_input_mode(),
            MouseInputMode::new(MouseReportingMode::Normal, MouseProtocolMode::SgrPixels)
        );
        assert_eq!(
            encode_input_event(left_mouse_down(), modes).unwrap(),
            b"\x1b[<0;1;2M"
        );
    }

    #[test]
    fn encodes_mouse_events_as_legacy_sequences_without_sgr_protocol() {
        let modes = InputModes::default().with_mouse_input_mode(MouseInputMode::new(
            MouseReportingMode::Normal,
            MouseProtocolMode::X10,
        ));

        assert_eq!(
            encode_input_event(left_mouse_down(), modes).unwrap(),
            b"\x1b[M !\""
        );
        assert_eq!(
            encode_input_event(left_mouse_release(), modes).unwrap(),
            b"\x1b[M#!\""
        );
    }

    #[test]
    fn encodes_mouse_events_as_utf8_sequences_when_enabled() {
        let modes = InputModes::default().with_mouse_input_mode(MouseInputMode::new(
            MouseReportingMode::Normal,
            MouseProtocolMode::Utf8,
        ));

        assert_eq!(
            encode_input_event(
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: 95,
                    row: 96,
                    modifiers: KeyModifiers::NONE,
                }),
                modes
            )
            .unwrap(),
            b"\x1b[M \xc2\x80\xc2\x81"
        );
        assert_eq!(
            encode_input_event(
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::Up(MouseButton::Left),
                    column: 95,
                    row: 96,
                    modifiers: KeyModifiers::NONE,
                }),
                modes
            )
            .unwrap(),
            b"\x1b[M#\xc2\x80\xc2\x81"
        );
    }

    #[test]
    fn encodes_mouse_events_as_urxvt_sequences_when_enabled() {
        let modes = InputModes::default().with_mouse_input_mode(MouseInputMode::new(
            MouseReportingMode::Normal,
            MouseProtocolMode::Urxvt,
        ));

        assert_eq!(
            encode_input_event(left_mouse_down(), modes).unwrap(),
            b"\x1b[32;1;2M"
        );
        assert_eq!(
            encode_input_event(left_mouse_release(), modes).unwrap(),
            b"\x1b[35;1;2M"
        );
    }

    #[test]
    fn normal_mouse_reporting_ignores_motion_without_buttons() {
        let modes = InputModes::default().with_mouse_input_mode(MouseInputMode::new(
            MouseReportingMode::Normal,
            MouseProtocolMode::X10,
        ));

        assert!(encode_input_event(mouse_moved(), modes).is_none());
    }

    #[test]
    fn any_event_mouse_reporting_encodes_motion_without_buttons() {
        let modes = InputModes::default().with_mouse_input_mode(MouseInputMode::new(
            MouseReportingMode::AnyEvent,
            MouseProtocolMode::Sgr,
        ));

        assert_eq!(
            encode_input_event(mouse_moved(), modes).unwrap(),
            b"\x1b[<35;3;4M"
        );
    }

    #[test]
    fn encodes_focus_events_when_focus_reporting_is_enabled() {
        let modes = InputModes::default().with_focus_reporting(true);

        assert_eq!(
            encode_input_event(Event::FocusGained, modes).unwrap(),
            b"\x1b[I"
        );
        assert_eq!(
            encode_input_event(Event::FocusLost, modes).unwrap(),
            b"\x1b[O"
        );
    }

    #[test]
    fn encodes_focus_events_only_when_focus_reporting_is_enabled() {
        assert!(
            encode_input_event(
                Event::FocusGained,
                InputModes::default().with_mouse_input_mode(MouseInputMode::new(
                    MouseReportingMode::Normal,
                    MouseProtocolMode::X10,
                ))
            )
            .is_none()
        );
        assert_eq!(
            encode_input_event(
                Event::FocusGained,
                InputModes::default().with_focus_reporting(true)
            )
            .unwrap(),
            b"\x1b[I"
        );
    }

    #[test]
    fn tracks_mouse_reporting_from_pty_output_modes() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.process(b"\x1b[?1006h", |change| changes.push(change));
        assert!(changes.is_empty());

        tracker.process(b"\x1b[?1000h", |change| changes.push(change));
        tracker.process(b"\x1b[?1000l", |change| changes.push(change));

        assert_eq!(
            changes,
            vec![
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::Normal,
                    MouseProtocolMode::Sgr,
                )),
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::None,
                    MouseProtocolMode::Sgr,
                ))
            ]
        );
    }

    #[test]
    fn tracks_combined_mouse_reporting_modes_from_pty_output() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.process(b"\x1b[?1002;1006h", |change| changes.push(change));
        tracker.process(b"\x1b[?1006l", |change| changes.push(change));
        tracker.process(b"\x1b[?1002l", |change| changes.push(change));

        assert_eq!(
            changes,
            vec![
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::ButtonEvent,
                    MouseProtocolMode::Sgr,
                )),
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::ButtonEvent,
                    MouseProtocolMode::X10,
                )),
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::None,
                    MouseProtocolMode::X10,
                ))
            ]
        );
    }

    #[test]
    fn tracks_sgr_mouse_protocol_from_pty_output() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.process(b"\x1b[?1000h", |change| changes.push(change));
        tracker.process(b"\x1b[?1006h", |change| changes.push(change));
        tracker.process(b"\x1b[?1006l", |change| changes.push(change));

        assert_eq!(
            changes,
            vec![
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::Normal,
                    MouseProtocolMode::X10,
                )),
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::Normal,
                    MouseProtocolMode::Sgr,
                )),
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::Normal,
                    MouseProtocolMode::X10,
                ))
            ]
        );
    }

    #[test]
    fn tracks_utf8_and_urxvt_mouse_protocols_from_pty_output() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.process(b"\x1b[?1000;1005h", |change| changes.push(change));
        tracker.process(b"\x1b[?1005l", |change| changes.push(change));
        tracker.process(b"\x1b[?1015h", |change| changes.push(change));

        assert_eq!(
            changes,
            vec![
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::Normal,
                    MouseProtocolMode::Utf8,
                )),
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::Normal,
                    MouseProtocolMode::X10,
                )),
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::Normal,
                    MouseProtocolMode::Urxvt,
                )),
            ]
        );
    }

    #[test]
    fn reports_extended_mouse_protocol_status_including_sgr_pixels() {
        let mut tracker = TerminalModeTracker::default();

        assert_eq!(tracker.private_mode_report_value(1005), 2);
        assert_eq!(tracker.private_mode_report_value(1015), 2);
        assert_eq!(tracker.private_mode_report_value(1016), 2);

        tracker.process(b"\x1b[?1005;1015;1016h", |_| {});

        assert_eq!(tracker.private_mode_report_value(1005), 2);
        assert_eq!(tracker.private_mode_report_value(1015), 2);
        assert_eq!(tracker.private_mode_report_value(1016), 1);

        tracker.process(b"\x1b[?1005;1015;1016l", |_| {});

        assert_eq!(tracker.private_mode_report_value(1005), 2);
        assert_eq!(tracker.private_mode_report_value(1015), 2);
        assert_eq!(tracker.private_mode_report_value(1016), 2);
    }

    #[test]
    fn resets_mouse_encoding_modes_to_x10_without_protocol_fallback() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.process(b"\x1b[?1000;1005;1015;1006;1016h", |change| {
            changes.push(change);
        });
        tracker.process(b"\x1b[?1016l", |change| changes.push(change));
        tracker.process(b"\x1b[?1006h", |change| changes.push(change));
        tracker.process(b"\x1b[?1006l", |change| changes.push(change));

        assert_eq!(
            changes,
            vec![
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::Normal,
                    MouseProtocolMode::SgrPixels,
                )),
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::Normal,
                    MouseProtocolMode::X10,
                )),
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::Normal,
                    MouseProtocolMode::Sgr,
                )),
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::Normal,
                    MouseProtocolMode::X10,
                )),
            ]
        );
    }

    #[test]
    fn tracks_mouse_reporting_mode_granularity_from_pty_output() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.process(b"\x1b[?1000h", |change| changes.push(change));
        tracker.process(b"\x1b[?1002h", |change| changes.push(change));
        tracker.process(b"\x1b[?1003h", |change| changes.push(change));
        tracker.process(b"\x1b[?1003l", |change| changes.push(change));
        tracker.process(b"\x1b[?1002l", |change| changes.push(change));
        tracker.process(b"\x1b[?1000l", |change| changes.push(change));

        assert_eq!(
            changes,
            vec![
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::Normal,
                    MouseProtocolMode::X10,
                )),
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::ButtonEvent,
                    MouseProtocolMode::X10,
                )),
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::AnyEvent,
                    MouseProtocolMode::X10,
                )),
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::ButtonEvent,
                    MouseProtocolMode::X10,
                )),
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::Normal,
                    MouseProtocolMode::X10,
                )),
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::None,
                    MouseProtocolMode::X10,
                ))
            ]
        );
    }

    #[test]
    fn tracks_split_focus_reporting_from_pty_output_modes() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.process(b"before\x1b[?", |change| changes.push(change));
        tracker.process(b"1004hafter\x1b[?1004l", |change| changes.push(change));

        assert_eq!(
            changes,
            vec![
                TerminalModeChange::Focus(true),
                TerminalModeChange::Focus(false)
            ]
        );
    }

    #[test]
    fn ignores_private_input_modes_inside_control_strings() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.process(
            b"\x1b]0;title \x1b[?1004h\x07\x1bPpayload \x1b[?2004h\x1b\\",
            |change| changes.push(change),
        );

        assert!(changes.is_empty());
        assert!(!tracker.focus_reporting());
        assert!(!tracker.bracketed_paste());
    }

    #[test]
    fn ignores_split_private_input_modes_inside_control_strings() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.process(b"\x1bPpayload ", |change| changes.push(change));
        tracker.process(b"\x1b[?1004;2004h\x1b\\", |change| changes.push(change));

        assert!(changes.is_empty());
        assert!(!tracker.focus_reporting());
        assert!(!tracker.bracketed_paste());
    }

    #[test]
    fn tracks_bracketed_paste_from_pty_output_modes() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.process(b"\x1b[?2004h", |change| changes.push(change));
        tracker.process(b"\x1b[?2004l", |change| changes.push(change));

        assert_eq!(
            changes,
            vec![
                TerminalModeChange::BracketedPaste(true),
                TerminalModeChange::BracketedPaste(false)
            ]
        );
    }

    #[test]
    fn tracks_synchronized_output_from_pty_output_modes() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.process(b"\x1b[?2026h", |change| changes.push(change));
        tracker.process(b"\x1b[?2026l", |change| changes.push(change));

        assert_eq!(
            changes,
            vec![
                TerminalModeChange::SynchronizedOutput(true),
                TerminalModeChange::SynchronizedOutput(false)
            ]
        );
        assert!(!tracker.synchronized_output());
    }

    #[test]
    fn tracks_c1_private_input_modes_from_pty_output() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.process(b"\x9b?1;1004;2004;2026h", |change| changes.push(change));

        assert_eq!(
            changes,
            vec![
                TerminalModeChange::ApplicationCursorKeys(true),
                TerminalModeChange::Focus(true),
                TerminalModeChange::BracketedPaste(true),
                TerminalModeChange::SynchronizedOutput(true)
            ]
        );
    }

    #[test]
    fn tracks_application_cursor_keys_from_pty_output_modes() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.process(b"\x1b[?1h", |change| changes.push(change));
        tracker.process(b"\x1b[?1l", |change| changes.push(change));

        assert_eq!(
            changes,
            vec![
                TerminalModeChange::ApplicationCursorKeys(true),
                TerminalModeChange::ApplicationCursorKeys(false)
            ]
        );
    }

    #[test]
    fn tracks_application_keypad_from_pty_output_modes() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.process(b"before\x1b", |change| changes.push(change));
        tracker.process(b"=after\x1b>", |change| changes.push(change));

        assert_eq!(
            changes,
            vec![
                TerminalModeChange::ApplicationKeypad(true),
                TerminalModeChange::ApplicationKeypad(false)
            ]
        );
    }

    #[test]
    fn resets_tracked_modes_on_ris_from_pty_output() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.process(b"\x1b[?1;1004;2004;2026h\x1b[?1002;1006h\x1b=", |_| {});
        tracker.apply_kitty_keyboard_sequence(
            KittyKeyboardMode {
                operation: KittyKeyboardOperation::Push,
                value: 1,
                apply_mode: KittyKeyboardApplyMode::Replace,
            },
            |_| {},
        );
        tracker.process(b"\x1bc", |change| changes.push(change));

        assert_eq!(
            changes,
            vec![
                TerminalModeChange::ApplicationCursorKeys(false),
                TerminalModeChange::ApplicationKeypad(false),
                TerminalModeChange::BracketedPaste(false),
                TerminalModeChange::Mouse(MouseInputMode::new(
                    MouseReportingMode::None,
                    MouseProtocolMode::X10,
                )),
                TerminalModeChange::Focus(false),
                TerminalModeChange::SynchronizedOutput(false),
                TerminalModeChange::KittyKeyboardFlags(0),
            ]
        );
        assert!(!tracker.application_cursor_keys());
        assert!(!tracker.application_keypad());
        assert!(!tracker.bracketed_paste());
        assert!(!tracker.focus_reporting());
        assert!(!tracker.synchronized_output());
        assert_eq!(tracker.mouse_input_mode(), MouseInputMode::default());
        assert_eq!(tracker.kitty_keyboard_flags(), 0);
    }

    #[test]
    fn soft_reset_restores_insert_and_origin_modes_from_pty_output() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.process(b"\x1b[?6h\x1b[4h", |_| {});
        assert_eq!(tracker.private_mode_report_value(6), 1);
        assert_eq!(tracker.ansi_mode_report_value(4), 1);

        tracker.process(b"\x1b[!p", |change| changes.push(change));

        assert!(changes.is_empty());
        assert_eq!(tracker.private_mode_report_value(6), 2);
        assert_eq!(tracker.ansi_mode_report_value(4), 2);
    }

    #[test]
    fn soft_reset_restores_wezterm_input_modes_from_pty_output() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        tracker.process(b"\x1b[?1h\x1b=", |_| {});
        tracker.apply_key_modifier_options_sequence(
            KeyModifierOptions {
                resource: Some(4),
                value: Some(2),
            },
            |_| {},
        );
        assert!(tracker.application_cursor_keys());
        assert!(tracker.application_keypad());
        assert_eq!(tracker.modify_other_keys(), 2);

        tracker.process(b"\x1b[!p", |change| changes.push(change));

        assert_eq!(changes.len(), 3);
        assert!(changes.contains(&TerminalModeChange::ApplicationCursorKeys(false)));
        assert!(changes.contains(&TerminalModeChange::ApplicationKeypad(false)));
        assert!(changes.contains(&TerminalModeChange::ModifyOtherKeys(0)));
        assert!(!tracker.application_cursor_keys());
        assert!(!tracker.application_keypad());
        assert_eq!(tracker.modify_other_keys(), 0);
        assert_eq!(tracker.private_mode_report_value(1), 2);
    }

    #[test]
    fn tracks_automatic_newline_mode_from_pty_output() {
        let mut tracker = TerminalModeTracker::default();

        assert_eq!(tracker.ansi_mode_report_value(20), 2);

        tracker.process(b"\x1b[20h", |_| {});
        assert_eq!(tracker.ansi_mode_report_value(20), 1);

        tracker.process(b"\x1b[20l", |_| {});
        assert_eq!(tracker.ansi_mode_report_value(20), 2);
    }

    #[test]
    fn tracks_bidirectional_support_mode_from_pty_output() {
        let mut tracker = TerminalModeTracker::default();

        assert_eq!(tracker.ansi_mode_report_value(8), 2);

        tracker.process(b"\x1b[8h", |_| {});
        assert_eq!(tracker.ansi_mode_report_value(8), 1);

        tracker.process(b"\x1b[8l", |_| {});
        assert_eq!(tracker.ansi_mode_report_value(8), 2);
    }

    #[test]
    fn soft_reset_restores_bidirectional_support_mode_from_pty_output() {
        let mut tracker = TerminalModeTracker::default();

        tracker.process(b"\x1b[8h", |_| {});
        assert_eq!(tracker.ansi_mode_report_value(8), 1);

        tracker.process(b"\x1b[!p", |_| {});
        assert_eq!(tracker.ansi_mode_report_value(8), 2);
    }

    #[test]
    fn tracks_meta_key_mode_from_pty_output() {
        let mut tracker = TerminalModeTracker::default();
        let mut changes = Vec::new();

        assert_eq!(tracker.private_mode_report_value(1034), 2);

        tracker.process(b"\x1b[?1034h", |change| changes.push(change));

        assert!(changes.is_empty());
        assert_eq!(tracker.private_mode_report_value(1034), 1);

        tracker.process(b"\x1b[?1034l", |change| changes.push(change));

        assert!(changes.is_empty());
        assert_eq!(tracker.private_mode_report_value(1034), 2);
    }

    fn left_mouse_down() -> Event {
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 1,
            modifiers: KeyModifiers::NONE,
        })
    }

    fn left_mouse_release() -> Event {
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 0,
            row: 1,
            modifiers: KeyModifiers::NONE,
        })
    }

    fn mouse_moved() -> Event {
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Moved,
            column: 2,
            row: 3,
            modifiers: KeyModifiers::NONE,
        })
    }

    fn mirror_text(filter: &TerminalOutputFilter) -> String {
        let grid = filter.mirror.grid();
        let size = grid.size();
        let mut text = String::new();

        for row in 0..size.rows {
            for column in 0..size.columns {
                text.push_str(grid.get(row, column).unwrap().text());
            }
        }

        text
    }

    fn xtgettcap_query(names: &[&[u8]]) -> Vec<u8> {
        let mut query = b"\x1bP+q".to_vec();
        for (index, name) in names.iter().enumerate() {
            if index > 0 {
                query.push(b';');
            }
            query.extend_from_slice(&super::encode_ascii_hex(name));
        }
        query.extend_from_slice(b"\x1b\\");
        query
    }

    fn xtgettcap_response(entries: &[(&[u8], &[u8])]) -> Vec<u8> {
        let mut response = Vec::new();
        for (name, value) in entries {
            response.extend_from_slice(b"\x1bP1+r");
            response.extend_from_slice(&encode_ascii_hex_upper(name));
            response.push(b'=');
            response.extend_from_slice(&encode_ascii_hex_upper(value));
            response.extend_from_slice(b"\x1b\\");
        }
        response
    }

    fn encode_ascii_hex_upper(bytes: &[u8]) -> Vec<u8> {
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        let mut encoded = Vec::with_capacity(bytes.len() * 2);
        for byte in bytes {
            encoded.push(HEX[usize::from(byte >> 4)]);
            encoded.push(HEX[usize::from(byte & 0x0f)]);
        }
        encoded
    }

    #[test]
    fn explicit_local_size_overrides_console_size() {
        let size = rterm_pty::PtySize::try_new(101, 31).unwrap();

        let resolved = resolve_local_size(Some(size));

        assert_eq!(resolved.columns(), 101);
        assert_eq!(resolved.rows(), 31);
    }

    #[test]
    fn terminal_output_filter_omits_st_controls_without_rendering() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"ab\x1b\\cd\x9cef", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write("gh\u{9c}ij".as_bytes(), &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write(b"kl\x1b", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write(b"\\mn", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"abcdefghijklmn");
        assert!(responses.is_empty());
        assert!(mirror_text(&filter).contains("abcdefghijklmn"));
    }

    #[test]
    fn terminal_output_filter_does_not_hold_unrelated_tail_bytes() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"console-smoke", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();

        assert_eq!(output, b"console-smoke");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_buffers_synchronized_output_until_mode_resets() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[?2026hmid", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();

        assert_eq!(output, b"before");
        assert!(responses.is_empty());
        assert!(filter.mode_tracker.synchronized_output());
        assert!(mirror_text(&filter).contains("beforemid"));

        filter
            .write(b"after\x1b[?2026$p", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();

        assert_eq!(output, b"before");
        assert_eq!(responses, b"\x1b[?2026;1$y");
        assert!(mirror_text(&filter).contains("beforemidafter"));

        filter
            .write(b"\x1b[?2026l done", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforemidafter done");
        assert_eq!(responses, b"\x1b[?2026;1$y");
        assert!(!filter.mode_tracker.synchronized_output());
    }

    #[test]
    fn terminal_output_filter_passes_malformed_mode_controls_through_unchanged() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let malformed = b"\x1b[?2026;badh\x1b[?2026;;l\x1b[>badu\x1b[=1;4u\x1b[>badm";

        filter
            .write(malformed, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();

        assert_eq!(output, malformed);
        assert!(responses.is_empty());
        assert!(!filter.mode_tracker.synchronized_output());
        assert_eq!(filter.mode_tracker.kitty_keyboard_flags(), 0);
        assert_eq!(filter.mode_tracker.modify_other_keys(), 0);
    }

    #[test]
    fn terminal_output_filter_fail_closes_malformed_reserved_clipboard_controls() {
        for policy in [Osc52Policy::Off, Osc52Policy::ReadWrite] {
            let mut filter = TerminalOutputFilter::default();
            let mut output = Vec::new();
            let mut responses = Vec::new();

            filter
                .write_with_clipboard(
                    b"\x1b]052;c;not-base64!\x07\x1b]00052\x07\x9d00052;c;not-base64!\x9c\xc2\x9d052;c;not-base64!\xc2\x9c\x1b]001337;Copy=;not-base64!\x07",
                    &mut output,
                    |response| {
                        responses.extend_from_slice(response);
                        Ok(())
                    },
                    |_| panic!("malformed clipboard control must not reach the host"),
                    || panic!("malformed clipboard query must not reach the host"),
                    policy,
                )
                .unwrap();

            assert!(output.is_empty());
            assert!(responses.is_empty());
        }

        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        filter
            .write_with_clipboard(
                b"\x1b]00052;c;Y29weQ==\x07",
                &mut output,
                |_| panic!("clipboard write must not produce a response"),
                |_| panic!("OSC52 Off must block a valid leading-zero selector"),
                || panic!("clipboard query callback must not run"),
                Osc52Policy::Off,
            )
            .unwrap();
        assert!(output.is_empty());
    }

    #[test]
    fn terminal_output_filter_omits_osc8_hyperlink_sequences() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"a\x1b]8;;https://example.com\x1b\\bc\x1b]8;;\x1b\\d",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"abcd");
        assert!(responses.is_empty());
        assert_eq!(
            filter.mirror.grid().get(0, 1).unwrap().hyperlink.as_deref(),
            Some("https://example.com")
        );
        assert_eq!(
            filter.mirror.grid().get(0, 2).unwrap().hyperlink.as_deref(),
            Some("https://example.com")
        );
        assert_eq!(filter.mirror.grid().get(0, 0).unwrap().hyperlink, None);
        assert_eq!(filter.mirror.grid().get(0, 3).unwrap().hyperlink, None);
    }

    #[test]
    fn terminal_output_filter_omits_c1_osc8_hyperlink_sequences() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"a\x9d8;;https://example.com\x9cbc\x9d8;;\x9cd",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"abcd");
        assert!(responses.is_empty());
        assert_eq!(
            filter.mirror.grid().get(0, 1).unwrap().hyperlink.as_deref(),
            Some("https://example.com")
        );
        assert_eq!(
            filter.mirror.grid().get(0, 2).unwrap().hyperlink.as_deref(),
            Some("https://example.com")
        );
        assert_eq!(filter.mirror.grid().get(0, 0).unwrap().hyperlink, None);
        assert_eq!(filter.mirror.grid().get(0, 3).unwrap().hyperlink, None);
    }

    #[test]
    fn terminal_output_filter_omits_utf8_c1_osc8_hyperlink_sequences() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                "a\u{9d}8;;https://example.com\u{9c}bc\u{9d}8;;\u{9c}d".as_bytes(),
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"abcd");
        assert!(responses.is_empty());
        assert_eq!(
            filter.mirror.grid().get(0, 1).unwrap().hyperlink.as_deref(),
            Some("https://example.com")
        );
        assert_eq!(
            filter.mirror.grid().get(0, 2).unwrap().hyperlink.as_deref(),
            Some("https://example.com")
        );
        assert_eq!(filter.mirror.grid().get(0, 0).unwrap().hyperlink, None);
        assert_eq!(filter.mirror.grid().get(0, 3).unwrap().hyperlink, None);
    }

    #[test]
    fn terminal_output_filter_omits_split_osc8_hyperlink_sequences() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"a\x1b]8;;https://example.com", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write(b"\x1b\\bc\x1b]8;;", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write(b"\x1b\\d", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"abcd");
        assert!(responses.is_empty());
        assert_eq!(
            filter.mirror.grid().get(0, 1).unwrap().hyperlink.as_deref(),
            Some("https://example.com")
        );
        assert_eq!(
            filter.mirror.grid().get(0, 2).unwrap().hyperlink.as_deref(),
            Some("https://example.com")
        );
        assert_eq!(filter.mirror.grid().get(0, 3).unwrap().hyperlink, None);
    }

    #[test]
    fn terminal_output_filter_omits_split_c1_osc8_hyperlink_sequences() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"a\x9d8;;https://example.com", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write(b"\x9cbc\x9d8;;", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write(b"\x9cd", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"abcd");
        assert!(responses.is_empty());
        assert_eq!(
            filter.mirror.grid().get(0, 1).unwrap().hyperlink.as_deref(),
            Some("https://example.com")
        );
        assert_eq!(
            filter.mirror.grid().get(0, 2).unwrap().hyperlink.as_deref(),
            Some("https://example.com")
        );
        assert_eq!(filter.mirror.grid().get(0, 3).unwrap().hyperlink, None);
    }

    #[test]
    fn terminal_output_filter_omits_split_utf8_c1_osc8_hyperlink_sequences() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"a\xc2", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write(b"\x9d8;;https://example.com", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write(b"\xc2", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write(b"\x9cbc\xc2\x9d8;;\xc2\x9cd", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"abcd");
        assert!(responses.is_empty());
        assert_eq!(
            filter.mirror.grid().get(0, 1).unwrap().hyperlink.as_deref(),
            Some("https://example.com")
        );
        assert_eq!(
            filter.mirror.grid().get(0, 2).unwrap().hyperlink.as_deref(),
            Some("https://example.com")
        );
        assert_eq!(filter.mirror.grid().get(0, 3).unwrap().hyperlink, None);
    }

    #[test]
    fn terminal_output_filter_drops_incomplete_osc8_on_flush() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"a\x1b]8;;https://example.com", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"a");
        assert!(responses.is_empty());
        assert_eq!(filter.mirror.grid().get(0, 0).unwrap().primary_char(), 'a');
        assert_eq!(filter.mirror.grid().get(0, 0).unwrap().hyperlink, None);
    }

    #[test]
    fn terminal_output_filter_drops_partial_osc8_prefix_on_flush() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"a\x1b]8", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"a");
        assert!(responses.is_empty());
        assert_eq!(filter.mirror.grid().get(0, 0).unwrap().primary_char(), 'a');
    }

    #[test]
    fn terminal_output_filter_holds_split_osc_title_until_terminated() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b]0;op", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();

        assert_eq!(output, b"before");
        assert!(responses.is_empty());
        assert_eq!(filter.mirror.title(), None);

        filter
            .write(b"s\x07after", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before\x1b]0;ops\x07after");
        assert!(responses.is_empty());
        assert_eq!(filter.mirror.title(), Some("ops"));
    }

    #[test]
    fn terminal_output_filter_drops_incomplete_osc_title_on_flush() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b]0;ops", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before");
        assert!(responses.is_empty());
        assert_eq!(filter.mirror.title(), None);
    }

    #[test]
    fn terminal_output_filter_holds_split_dcs_until_terminated() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1bPignored", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();

        assert_eq!(output, b"before");
        assert!(responses.is_empty());

        filter
            .write(b"\x1b\\after", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before\x1bPignored\x1b\\after");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_drops_incomplete_dcs_on_flush() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1bPignored", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_drops_trailing_escape_prefix_on_flush() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_resynchronizes_queries_after_non_st_escape_in_osc() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b]0;title \x1b[6n\x07after",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before\x07after");
        assert_eq!(responses, b"\x1b[1;7R");
    }

    #[test]
    fn terminal_output_filter_holds_split_csi_until_final_byte() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[31", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();

        assert_eq!(output, b"before");
        assert!(responses.is_empty());

        filter
            .write(b"mafter", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before\x1b[31mafter");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_consumes_decrqcra_without_response_by_default() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[7;1;1;1;1;5*yafter", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_drops_incomplete_csi_on_flush() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[31", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_answers_cursor_position_query() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[6nafter", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, b"\x1b[1;7R");
    }

    #[test]
    fn session_log_writer_records_visible_output() {
        let mut screen = Vec::new();
        let mut log = Vec::new();
        let metrics = super::LocalMetricsCounters::default();
        let mut output = super::SessionLogWriter::new(&mut screen, Some(&mut log), metrics.clone());

        output.write_all(b"visible").unwrap();
        output.flush().unwrap();

        assert_eq!(screen, b"visible");
        assert_eq!(log, b"visible");
        assert_eq!(metrics.snapshot().terminal_output_bytes, 7);
    }

    #[test]
    fn session_log_writer_omits_bell_from_log() {
        let mut screen = Vec::new();
        let mut log = Vec::new();
        let mut output = super::SessionLogWriter::new(
            &mut screen,
            Some(&mut log),
            super::LocalMetricsCounters::default(),
        );

        output.write_all(b"before\x07after").unwrap();
        output.flush().unwrap();

        assert_eq!(screen, b"before\x07after");
        assert_eq!(log, b"beforeafter");
    }

    #[test]
    fn session_log_writer_omits_title_sequence_from_log() {
        let mut screen = Vec::new();
        let mut log = Vec::new();
        let mut output = super::SessionLogWriter::new(
            &mut screen,
            Some(&mut log),
            super::LocalMetricsCounters::default(),
        );

        output.write_all(b"before\x1b]0;ops\x07after").unwrap();
        output.flush().unwrap();

        assert_eq!(screen, b"before\x1b]0;ops\x07after");
        assert_eq!(log, b"beforeafter");
    }

    #[test]
    fn session_log_writer_omits_split_title_sequence_from_log() {
        let mut screen = Vec::new();
        let mut log = Vec::new();
        let mut output = super::SessionLogWriter::new(
            &mut screen,
            Some(&mut log),
            super::LocalMetricsCounters::default(),
        );

        output.write_all(b"before\x1b]0;op").unwrap();
        output.write_all(b"s\x07after").unwrap();
        output.flush().unwrap();

        assert_eq!(screen, b"before\x1b]0;ops\x07after");
        assert_eq!(log, b"beforeafter");
    }

    #[test]
    fn terminal_output_filter_answers_current_cursor_position_query() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"abc\x1b[6n", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();

        assert_eq!(output, b"abc");
        assert_eq!(responses, b"\x1b[1;4R");
    }

    #[test]
    fn terminal_output_filter_answers_c1_cursor_position_query() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"abc\x9b6n", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();

        assert_eq!(output, b"abc");
        assert_eq!(responses, b"\x1b[1;4R");
    }

    #[test]
    fn terminal_output_filter_does_not_match_raw_c1_inside_utf8_text() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before \xc3\x9b6n after", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before \xc3\x9b6n after");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_does_not_retain_raw_c1_prefix_inside_utf8_text() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before \xc3\x9b", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before \xc3\x9b");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_consumes_private_cursor_position_queries_like_wezterm() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let csi = '\u{9b}';
        let mut input = b"abc\x1b[?6n def".to_vec();
        input.extend_from_slice(b"\x9b?6n ghi");
        input.extend_from_slice(format!("{csi}?6n").as_bytes());

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();

        assert_eq!(output, b"abc def ghi");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_consumes_device_attribute_responses_like_wezterm() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let csi = '\u{9b}';
        let mut input =
            b"a\x1b[?1;0c b\x1b[?1;2c c\x1b[?6c d\x1b[?62;4;6;22c e\x1b[?63;1c f\x1b[?64c g"
                .to_vec();
        input.extend_from_slice(b"\x9b?1;2c h");
        input.extend_from_slice(format!("{csi}?62;4c i").as_bytes());

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();

        assert_eq!(output, b"a b c d e f g h i");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_answers_device_and_status_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"a\x1b[c b\x1b[0c c\x1b[>c d\x1b[>0c e\x1b[=c f\x1b[=0c g\x1b[5n h",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"a b c d e f g h");
        assert_eq!(
            responses,
            b"\x1b[?65;4;6;18;22;52c\x1b[?65;4;6;18;22;52c\x1b[>1;277;0c\x1b[>1;277;0c\x1bP!|00000000\x1b\\\x1bP!|00000000\x1b\\\x1b[0n"
        );
    }

    #[test]
    fn terminal_output_filter_answers_c1_device_and_status_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"a\x9bc b\x9b0c c\x9b>c d\x9b>0c e\x9b=c f\x9b=0c g\x9b5n h",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"a b c d e f g h");
        assert_eq!(
            responses,
            b"\x1b[?65;4;6;18;22;52c\x1b[?65;4;6;18;22;52c\x1b[>1;277;0c\x1b[>1;277;0c\x1bP!|00000000\x1b\\\x1bP!|00000000\x1b\\\x1b[0n"
        );
    }

    #[test]
    fn terminal_output_filter_answers_utf8_c1_device_and_status_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let csi = '\u{9b}';
        let input = format!("a{csi}c b{csi}0c c{csi}>c d{csi}>0c e{csi}=c f{csi}=0c g{csi}5n h");

        filter
            .write(input.as_bytes(), &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"a b c d e f g h");
        assert_eq!(
            responses,
            b"\x1b[?65;4;6;18;22;52c\x1b[?65;4;6;18;22;52c\x1b[>1;277;0c\x1b[>1;277;0c\x1bP!|00000000\x1b\\\x1bP!|00000000\x1b\\\x1b[0n"
        );
    }

    #[test]
    fn terminal_output_filter_answers_terminal_parameter_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"a\x1b[x b\x1b[0x c\x1b[1x d", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"a b c d");
        assert_eq!(
            responses,
            b"\x1b[2;1;1;128;128;1;0x\x1b[2;1;1;128;128;1;0x\x1b[3;1;1;128;128;1;0x"
        );
    }

    #[test]
    fn terminal_output_filter_answers_c1_terminal_parameter_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"a\x9bx b\x9b0x c\x9b1x d", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"a b c d");
        assert_eq!(
            responses,
            b"\x1b[2;1;1;128;128;1;0x\x1b[2;1;1;128;128;1;0x\x1b[3;1;1;128;128;1;0x"
        );
    }

    #[test]
    fn terminal_output_filter_answers_utf8_c1_terminal_parameter_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let csi = '\u{9b}';
        let input = format!("a{csi}x b{csi}0x c{csi}1x d");

        filter
            .write(input.as_bytes(), &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"a b c d");
        assert_eq!(
            responses,
            b"\x1b[2;1;1;128;128;1;0x\x1b[2;1;1;128;128;1;0x\x1b[3;1;1;128;128;1;0x"
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtsmgraphics_queries() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"a\x1b[?1;1S b\x1b[?1;4S c\x1b[?2;1S d\x1b[?2;4S e\x1b[?3;1S f\x1b[?3;4S g\x1b[?2;2S h\x1b[?9;1S i\x1b[?1;3;10S j",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"a b c d e f g h i j");
        assert_eq!(
            responses,
            b"\x1b[?1;0;65536S\x1b[?1;0;65536S\x1b[?2;0;1056;688S\x1b[?2;0;1056;688S\x1b[?3;0;1056;688S\x1b[?3;0;1056;688S\x1b[?2;0S\x1b[?9;1S\x1b[?1;2S"
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtsmgraphics_large_numeric_parameters_like_wezterm() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"a\x1b[?70000;1S b\x1b[?1;70000S c\x1b[?1;1;70000S d",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"a b c d");
        assert_eq!(responses, b"\x1b[?70000;1S\x1b[?1;2S\x1b[?1;0;65536S");
    }

    #[test]
    fn terminal_output_filter_answers_c1_xtsmgraphics_queries() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"a\x9b?1;1S b\x9b?2;4S c\x9b?9;1S d",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"a b c d");
        assert_eq!(responses, b"\x1b[?1;0;65536S\x1b[?2;0;1056;688S\x1b[?9;1S");
    }

    #[test]
    fn terminal_output_filter_answers_utf8_c1_xtsmgraphics_queries() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let csi = '\u{9b}';
        let input = format!("a{csi}?1;1S b{csi}?2;4S c{csi}?9;1S d");

        filter
            .write(input.as_bytes(), &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"a b c d");
        assert_eq!(responses, b"\x1b[?1;0;65536S\x1b[?2;0;1056;688S\x1b[?9;1S");
    }

    #[test]
    fn terminal_output_filter_answers_text_area_size_query() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[18tafter", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, b"\x1b[8;43;132t");
    }

    #[test]
    fn terminal_output_filter_answers_window_pixel_size_query() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[14tafter", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, b"\x1b[4;688;1056t");
    }

    #[test]
    fn terminal_output_filter_consumes_window_position_query_without_response() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[13tafter", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_consumes_screen_pixel_size_query_without_response() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[15tafter", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_consumes_wezterm_unanswered_window_query_variants() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b[13;2t middle\x1b[14;2tafter",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before middleafter");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_consumes_wezterm_no_response_window_controls() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b[1t middle\x1b[8;24;80t after\x1b[22;0t end\x1b[23;2t",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before middle after end");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_consumes_malformed_window_title_stack_controls() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b]0;main\x07\x1b[22;?t middle\x1b]0;alternate\x07\x1b[23;?tafter",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(
            output,
            b"before\x1b]0;main\x07 middle\x1b]0;alternate\x07after"
        );
        assert!(responses.is_empty());
        assert_eq!(filter.mirror.title(), Some("alternate"));
    }

    #[test]
    fn terminal_output_filter_consumes_unanswered_window_control_with_malformed_parameter() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[13;?tafter", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_consumes_unanswered_window_control_with_extra_malformed_parameter() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[13;2;?tafter", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_consumes_unknown_window_control_without_response() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[999tafter", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_consumes_wezterm_empty_window_report_parameters() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[13;tafter", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_answers_character_cell_size_query() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[16tafter", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, b"\x1b[6;16;8t");
    }

    #[test]
    fn terminal_output_filter_answers_iterm_report_cell_size_query() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b]1337;ReportCellSize\x07after",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, b"\x1b]1337;ReportCellSize=16.0;8.0\x1b\\");
    }

    #[test]
    fn terminal_output_filter_answers_wezterm_window_reports_with_empty_and_extra_parameters() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b[14;t middle\x1b[16;0;99t after\x1b[18;1t",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before middle after");
        assert_eq!(responses, b"\x1b[4;688;1056t\x1b[6;16;8t\x1b[8;43;132t");
    }

    #[test]
    fn terminal_output_filter_consumes_screen_size_query_without_response() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[19tafter", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_consumes_window_state_query_without_response() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[11tafter", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_consumes_window_title_queries_without_response_by_default() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x1b]0;ops\x07before\x1b[20t middle\x1b[21tafter",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"\x1b]0;ops\x07before middleafter");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_answers_kitty_keyboard_protocol_flags_queries_and_tracks_push_pop() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[?u", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        assert_eq!(output, b"before");
        assert_eq!(responses, b"\x1b[?0u");

        filter
            .write(b"\x1b[>1u\x1b[?u", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        assert_eq!(output, b"before");
        assert_eq!(responses, b"\x1b[?0u\x1b[?1u");

        filter
            .write(
                b"\x1b[>9u\x1b[?u\x1b[<u\x1b[?u\x1b[<1u\x1b[?uafter",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, b"\x1b[?0u\x1b[?1u\x1b[?9u\x1b[?1u\x1b[?0u");
    }

    #[test]
    fn terminal_output_filter_answers_kitty_keyboard_protocol_flags_queries_and_tracks_set_reset() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b[=1u\x1b[?u\x1b[=8;2u\x1b[?u\x1b[=1;3u\x1b[?uafter",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, b"\x1b[?1u\x1b[?9u\x1b[?8u");
    }

    #[test]
    fn terminal_output_filter_answers_modify_other_keys_queries_and_tracks_set_reset() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[?4m", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        assert_eq!(output, b"before");
        assert_eq!(responses, b"\x1b[>4;0m");

        filter
            .write(b"\x1b[>4;2m\x1b[?4mafter", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, b"\x1b[>4;0m\x1b[>4;2m");
    }

    #[test]
    fn terminal_output_filter_answers_c1_terminal_size_queries() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x9b18t middle\x9b19tafter",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before middleafter");
        assert_eq!(responses, b"\x1b[8;43;132t");
    }

    #[test]
    fn terminal_output_filter_answers_c1_window_pixel_and_cell_size_queries() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x9b14t middle\x9b16tafter",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before middleafter");
        assert_eq!(responses, b"\x1b[4;688;1056t\x1b[6;16;8t");
    }

    #[test]
    fn terminal_output_filter_consumes_c1_window_position_and_screen_pixel_size_queries_without_response()
     {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x9b13t middle\x9b15tafter",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before middleafter");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_consumes_c1_window_state_and_title_queries_without_response_by_default()
     {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x1b]0;ops\x07before\x9b11t middle\x9b20t after\x9b21t",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"\x1b]0;ops\x07before middle after");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_answers_private_mode_status_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b[?1h\x1b[?1$p middle\x1b[?1004$p after\x1b[?9999$p",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before\x1b[?1h middle after");
        assert_eq!(responses, b"\x1b[?1;1$y\x1b[?1004;2$y\x1b[?9999;0$y");
    }

    #[test]
    fn terminal_output_filter_answers_display_private_mode_status_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x1b[?1034$p \x1b[?1034h\x1b[?1034$p\x1b[?1034l\x1b[?1034$p \
                  \x1b[?7$p \x1b[?7l\x1b[?7$p \
                  \x1b[?25$p \x1b[?25l\x1b[?25$p \
                  \x1b[?45$p \x1b[?45h\x1b[?45$p\x1b[?45l\x1b[?45$p \
                  \x1b[?6$p \x1b[?6h\x1b[?6$p \
                  \x1b[?80$p \x1b[?80h\x1b[?80$p\x1b[?80l\x1b[?80$p \
                  \x1b[?8452$p \x1b[?8452h\x1b[?8452$p\x1b[?8452l\x1b[?8452$p \
                  \x1b[?47$p \x1b[?47h\x1b[?47$p\x1b[?47l\x1b[?47$p \
                  \x1b[?1048$p \x1b[?1048h\x1b[?1048$p\x1b[?1048l\x1b[?1048$p \
                  \x1b[?1047$p \x1b[?1047h\x1b[?1047$p\x1b[?1047l\x1b[?1047$p \
                  \x1b[?1049$p \x1b[?1049h\x1b[?1049$p\x1b[?1049l\x1b[?1049$p",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(
            output,
            b" \x1b[?1034h\x1b[?1034l  \x1b[?7l  \x1b[?25l  \x1b[?45h\x1b[?45l  \x1b[?6h  \x1b[?80h\x1b[?80l  \x1b[?8452h\x1b[?8452l  \x1b[?47h\x1b[?47l  \x1b[?1048h\x1b[?1048l  \x1b[?1047h\x1b[?1047l  \x1b[?1049h\x1b[?1049l"
        );
        assert_eq!(
            responses,
            b"\x1b[?1034;2$y\x1b[?1034;1$y\x1b[?1034;2$y\
              \x1b[?7;1$y\x1b[?7;2$y\
              \x1b[?25;1$y\x1b[?25;2$y\
              \x1b[?45;2$y\x1b[?45;1$y\x1b[?45;2$y\
              \x1b[?6;2$y\x1b[?6;1$y\
              \x1b[?80;2$y\x1b[?80;1$y\x1b[?80;2$y\
              \x1b[?8452;2$y\x1b[?8452;1$y\x1b[?8452;2$y\
              \x1b[?47;0$y\x1b[?47;0$y\x1b[?47;0$y\
              \x1b[?1048;0$y\x1b[?1048;0$y\x1b[?1048;0$y\
              \x1b[?1047;0$y\x1b[?1047;0$y\x1b[?1047;0$y\
              \x1b[?1049;0$y\x1b[?1049;0$y\x1b[?1049;0$y"
        );
    }

    #[test]
    fn terminal_output_filter_reports_wezterm_unknown_alternate_screen_private_mode_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x1b[?47$p\x1b[?47h\x1b[?47$p\
                  \x1b[?1047$p\x1b[?1047h\x1b[?1047$p\
                  \x1b[?1048$p\x1b[?1048h\x1b[?1048$p\
                  \x1b[?1049$p\x1b[?1049h\x1b[?1049$p",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();

        assert_eq!(output, b"\x1b[?47h\x1b[?1047h\x1b[?1048h\x1b[?1049h");
        assert_eq!(
            responses,
            b"\x1b[?47;0$y\x1b[?47;0$y\
              \x1b[?1047;0$y\x1b[?1047;0$y\
              \x1b[?1048;0$y\x1b[?1048;0$y\
              \x1b[?1049;0$y\x1b[?1049;0$y"
        );
    }

    #[test]
    fn terminal_output_filter_answers_declrmm_mode_status_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x1b[?69$p\x1b[?69h\x1b[?69$p\x1b[?69l\x1b[?69$p",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"\x1b[?69h\x1b[?69l");
        assert_eq!(responses, b"\x1b[?69;2$y\x1b[?69;1$y\x1b[?69;2$y");
    }

    #[test]
    fn terminal_output_filter_answers_wezterm_private_mode_reports() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x1b[?3$p \x1b[?2027$p \x1b[?2027l\x1b[?2027$p \
                  \x1b[?1070$p \x1b[?1070h\x1b[?1070$p\x1b[?1070l\x1b[?1070$p",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"  \x1b[?2027l  \x1b[?1070h\x1b[?1070l");
        assert_eq!(
            responses,
            b"\x1b[?3;2$y\x1b[?2027;3$y\x1b[?2027;3$y\
              \x1b[?1070;2$y\x1b[?1070;1$y\x1b[?1070;2$y"
        );
    }

    #[test]
    fn terminal_output_filter_answers_dec_ansi_mode_status_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x1b[?2$p\x1b[?2h\x1b[?2$p\x1b[?2l\x1b[?2$p",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"\x1b[?2h\x1b[?2l");
        assert_eq!(responses, b"\x1b[?2;2$y\x1b[?2;1$y\x1b[?2;2$y");
    }

    #[test]
    fn terminal_output_filter_answers_screen_reverse_mode_status_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x1b[?5$p\x1b[?5h\x1b[?5$p\x1b[?5l\x1b[?5$p",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"\x1b[?5h\x1b[?5l");
        assert_eq!(responses, b"\x1b[?5;2$y\x1b[?5;1$y\x1b[?5;2$y");
    }

    #[test]
    fn terminal_output_filter_answers_private_mode_status_defaults_after_terminal_reset() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x1b[?1;6;25;47;1048;1049;1000;1006;1004;2004h\x1b[?7l\x1b=\x1bc\
                  \x1b[?1$p\x1b[?6$p\x1b[?7$p\x1b[?25$p\x1b[?47$p\x1b[?1048$p\
                  \x1b[?1049$p\x1b[?1000$p\x1b[?1006$p\x1b[?1004$p\x1b[?2004$p",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(
            responses,
            b"\x1b[?1;2$y\x1b[?6;2$y\x1b[?7;1$y\x1b[?25;1$y\x1b[?47;0$y\x1b[?1048;0$y\x1b[?1049;0$y\x1b[?1000;2$y\x1b[?1006;2$y\x1b[?1004;2$y\x1b[?2004;2$y"
        );
    }

    #[test]
    fn terminal_output_filter_flushes_synchronized_output_on_terminal_reset() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b[?2026hmid\x1bcafter\x1b[?2026$p",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforemid\x1bcafter");
        assert_eq!(responses, b"\x1b[?2026;2$y");
        assert!(!filter.mode_tracker.synchronized_output());
    }

    #[test]
    fn terminal_output_filter_answers_ansi_mode_status_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b[4$p \x1b[4h\x1b[4$p \x1b[4l\x1b[4$p \x1b[999$p",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before \x1b[4h \x1b[4l ");
        assert_eq!(responses, b"\x1b[4;2$y\x1b[4;1$y\x1b[4;2$y\x1b[999;0$y");
        assert!(!mirror_text(&filter).contains("$p"));
    }

    #[test]
    fn terminal_output_filter_answers_automatic_newline_mode_status_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b[20$p \x1b[20h\x1b[20$p \x1b[20l\x1b[20$p",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before \x1b[20h \x1b[20l");
        assert_eq!(responses, b"\x1b[20;2$y\x1b[20;1$y\x1b[20;2$y");
        assert!(!mirror_text(&filter).contains("$p"));
    }

    #[test]
    fn terminal_output_filter_answers_bidirectional_support_mode_status_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b[8$p \x1b[8h\x1b[8$p \x1b[8l\x1b[8$p",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before \x1b[8h \x1b[8l");
        assert_eq!(responses, b"\x1b[8;2$y\x1b[8;1$y\x1b[8;2$y");
        assert!(!mirror_text(&filter).contains("$p"));
    }

    #[test]
    fn terminal_output_filter_answers_c1_ansi_mode_status_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"\x9b4h\x9b4$p", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"\x9b4h");
        assert_eq!(responses, b"\x1b[4;1$y");
    }

    #[test]
    fn terminal_output_filter_answers_c1_automatic_newline_mode_status_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"\x9b20h\x9b20$p", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"\x9b20h");
        assert_eq!(responses, b"\x1b[20;1$y");
    }

    #[test]
    fn terminal_output_filter_answers_c1_bidirectional_support_mode_status_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"\x9b8h\x9b8$p", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"\x9b8h");
        assert_eq!(responses, b"\x1b[8;1$y");
    }

    #[test]
    fn terminal_output_filter_answers_osc_color_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b]10;?\x07 middle\x1b]11;?\x1b\\ after\x1b]4;1;?\x07done",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before middle afterdone");
        assert_eq!(
            responses,
            b"\x1b]10;rgb:e5e5/e5e5/e5e5\x07\x1b]11;rgb:0c0c/0c0c/0c0c\x1b\\\x1b]4;1;rgb:cdcd/3131/3131\x07"
        );
    }

    #[test]
    fn terminal_output_filter_answers_c1_osc_color_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"\x9d4;196;?\x9c", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert!(output.is_empty());
        assert_eq!(responses, b"\x1b]4;196;rgb:ffff/0000/0000\x9c");
    }

    #[test]
    fn terminal_output_filter_answers_utf8_c1_osc_color_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write("\u{9d}4;196;?\u{9c}".as_bytes(), &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert!(output.is_empty());
        assert_eq!(responses, b"\x1b]4;196;rgb:ffff/0000/0000\x9c");
    }

    #[test]
    fn terminal_output_filter_answers_split_utf8_c1_osc_color_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"\xc2", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write(b"\x9d4;196;?\xc2", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write(b"\x9c", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert!(output.is_empty());
        assert_eq!(responses, b"\x1b]4;196;rgb:ffff/0000/0000\x9c");
    }

    #[test]
    fn terminal_output_filter_answers_cursor_color_queries_after_changes_and_reset() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b]12;rgb:aa/bb/cc\x07 middle\x1b]12;?\x07 after\x1b]112\x07 reset\x1b]12;?\x1b\\done",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(
            output,
            b"before\x1b]12;rgb:aa/bb/cc\x07 middle after\x1b]112\x07 resetdone"
        );
        assert_eq!(
            responses,
            b"\x1b]12;rgb:aaaa/bbbb/cccc\x07\x1b]12;rgb:e5e5/e5e5/e5e5\x1b\\"
        );
    }

    #[test]
    fn terminal_output_filter_answers_c1_cursor_color_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x9d12;rgb:01/02/03\x9c\x9d12;?\x9c",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"\x9d12;rgb:01/02/03\x9c");
        assert_eq!(responses, b"\x1b]12;rgb:0101/0202/0303\x9c");
    }

    #[test]
    fn terminal_output_filter_answers_osc_color_queries_after_color_changes() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b]10;rgb:11/22/33\x07 middle\x1b]10;?\x07 after\x1b]4;1;rgb:01/02/03\x1b\\ done\x1b]4;1;?\x1b\\",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(
            output,
            b"before\x1b]10;rgb:11/22/33\x07 middle after\x1b]4;1;rgb:01/02/03\x1b\\ done"
        );
        assert_eq!(
            responses,
            b"\x1b]10;rgb:1111/2222/3333\x07\x1b]4;1;rgb:0101/0202/0303\x1b\\"
        );
    }

    #[test]
    fn terminal_output_filter_applies_hex_osc_color_changes() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x1b]10;#112233\x07\x1b]4;2;#445566\x07\x1b]10;?\x07\x1b]4;2;?\x07",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(
            responses,
            b"\x1b]10;rgb:1111/2222/3333\x07\x1b]4;2;rgb:4444/5555/6666\x07"
        );
        assert_eq!(output, b"\x1b]10;#112233\x07\x1b]4;2;#445566\x07");
    }

    #[test]
    fn terminal_output_filter_applies_rgba_osc_dynamic_color_changes() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x1b]10;rgba(127,127,127,0.4)\x07\
                  \x1b]11;rgba:efff/ecff/f4ff/d000\x1b\\\
                  \x1b]12;rgba(1,2,3,1)\x07\
                  \x1b]10;?\x07\x1b]11;?\x1b\\\x1b]12;?\x07",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(
            responses,
            b"\x1b]10;rgba:7f7f/7f7f/7f7f/6666\x07\x1b]11;rgba:efff/ecff/f4ff/d000\x1b\\\x1b]12;rgba:0101/0202/0303/ffff\x07"
        );
        assert_eq!(
            output,
            b"\x1b]10;rgba(127,127,127,0.4)\x07\x1b]11;rgba:efff/ecff/f4ff/d000\x1b\\\x1b]12;rgba(1,2,3,1)\x07"
        );
    }

    #[test]
    fn terminal_output_filter_applies_multiple_palette_color_changes_from_one_osc4_sequence() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x1b]4;1;rgb:01/02/03;2;rgb:04/05/06\x07\
                  \x1b]4;1;?\x07\x1b]4;2;?\x07",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(
            responses,
            b"\x1b]4;1;rgb:0101/0202/0303\x07\x1b]4;2;rgb:0404/0505/0606\x07"
        );
        assert_eq!(output, b"\x1b]4;1;rgb:01/02/03;2;rgb:04/05/06\x07");
    }

    #[test]
    fn terminal_output_filter_answers_multiple_palette_color_queries_from_one_osc4_sequence() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x1b]4;1;rgb:01/02/03;2;rgb:04/05/06\x07\
                  \x1b]4;1;?;2;?\x07",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(
            responses,
            b"\x1b]4;1;rgb:0101/0202/0303\x07\x1b]4;2;rgb:0404/0505/0606\x07"
        );
        assert_eq!(output, b"\x1b]4;1;rgb:01/02/03;2;rgb:04/05/06\x07");
    }

    #[test]
    fn terminal_output_filter_resets_dynamic_and_palette_colors() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\
                  \x1b]10;rgb:11/22/33\x07\x1b]11;rgb:44/55/66\x07\
                  \x1b]4;1;rgb:01/02/03\x07\
                  \x1b]10;?\x07\x1b]11;?\x07\x1b]4;1;?\x07\
                  \x1b]110\x07\x1b]111\x07\x1b]104;1\x07\
                  \x1b]10;?\x07\x1b]11;?\x07\x1b]4;1;?\x07after",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(
            responses,
            b"\x1b]10;rgb:1111/2222/3333\x07\
              \x1b]11;rgb:4444/5555/6666\x07\
              \x1b]4;1;rgb:0101/0202/0303\x07\
              \x1b]10;rgb:e5e5/e5e5/e5e5\x07\
              \x1b]11;rgb:0c0c/0c0c/0c0c\x07\
              \x1b]4;1;rgb:cdcd/3131/3131\x07"
        );
        assert!(!String::from_utf8_lossy(&output).contains(";?"));
    }

    #[test]
    fn terminal_output_filter_resets_all_palette_colors() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x1b]4;1;rgb:01/02/03\x07\x1b]4;2;rgb:04/05/06\x07\
                  \x1b]104\x07\x1b]4;1;?\x07\x1b]4;2;?\x07",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(
            responses,
            b"\x1b]4;1;rgb:cdcd/3131/3131\x07\x1b]4;2;rgb:0d0d/bcbc/7979\x07"
        );
    }

    #[test]
    fn terminal_output_filter_resets_multiple_palette_colors() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x1b]4;1;rgb:01/02/03\x07\x1b]4;2;rgb:04/05/06\x07\x1b]4;3;rgb:07/08/09\x07\
                  \x1b]104;1;2\x07\x1b]4;1;?\x07\x1b]4;2;?\x07\x1b]4;3;?\x07",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(
            responses,
            b"\x1b]4;1;rgb:cdcd/3131/3131\x07\
              \x1b]4;2;rgb:0d0d/bcbc/7979\x07\
              \x1b]4;3;rgb:0707/0808/0909\x07"
        );
    }

    #[test]
    fn terminal_output_filter_resynchronizes_osc_color_after_non_st_escape_in_dcs() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x1bPpayload \x1b]10;rgb:11/22/33\x1b\\ after\x1b]10;?\x07",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"\x1b]10;rgb:11/22/33\x1b\\ after");
        assert_eq!(responses, b"\x1b]10;rgb:1111/2222/3333\x07");
    }

    #[test]
    fn terminal_output_filter_resynchronizes_split_osc_color_after_non_st_escape_in_dcs() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"\x1bPpayload ", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write(
                b"\x1b]10;rgb:11/22/33\x1b\\ after\x1b]10;?\x07",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"\x1b]10;rgb:11/22/33\x1b\\ after");
        assert_eq!(responses, b"\x1b]10;rgb:1111/2222/3333\x07");
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1bP+q436f\x1b\\ middle\x90+q544e;524742;6e616d65\x9c after\x1bP+q666f6f\x1b\\done",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before middle afterdone");
        assert_eq!(
            responses,
            b"\x1bP1+r436F=323536\x1b\\\x1bP1+r544E=787465726D2D323536636F6C6F72\x1b\\\x1bP1+r524742=382F382F38\x1b\\\x1bP1+r6E616D65=787465726D2D323536636F6C6F72\x1b\\\x1bP0+r666F6F\x1b\\"
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_terminal_name_from_configured_term() {
        let mut filter = TerminalOutputFilter::default();
        filter.set_terminal_name("wezterm");
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1bP+q544e;6e616d65\x1b\\after",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            b"\x1bP1+r544E=77657A7465726D\x1b\\\x1bP1+r6E616D65=77657A7465726D\x1b\\"
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_invalid_hex_names_like_wezterm() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1bP+qZZ;544e", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        assert_eq!(output, b"before");
        assert!(responses.is_empty());

        filter
            .write(b";5\x1b\\after", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            b"\x1bP0+r5A5A\x1b\\\x1bP1+r544E=787465726D2D323536636F6C6F72\x1b\\\x1bP0+r35\x1b\\"
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_non_utf8_hex_names_like_wezterm() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1bP+qff;436f\x1b\\after",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, b"\x1bP0+rEFBFBD\x1b\\\x1bP1+r436F=323536\x1b\\");
    }

    #[test]
    fn terminal_output_filter_answers_utf8_c1_dcs_xtgettcap_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let dcs = '\u{90}';
        let st = '\u{9c}';
        let input = format!("before{dcs}+q436f{st}after");

        filter
            .write(input.as_bytes(), &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, b"\x1bP1+r436F=323536\x1b\\");
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_size_queries_from_current_size() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1bP+q636f;6c69\x1b\\after",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            b"\x1bP1+r636F=313332\x1b\\\x1bP1+r6C69=3433\x1b\\"
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_official_numeric_capability_names() {
        let mut filter = TerminalOutputFilter::new(rterm_pty::PtySize::try_new(132, 43).unwrap());
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let query = xtgettcap_query(&[
            b"cols".as_slice(),
            b"lines".as_slice(),
            b"it".as_slice(),
            b"pairs".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            xtgettcap_response(&[
                (b"cols".as_slice(), b"132".as_slice()),
                (b"lines".as_slice(), b"43".as_slice()),
                (b"it".as_slice(), b"8".as_slice()),
                (b"pairs".as_slice(), b"32767".as_slice()),
            ])
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_modern_style_and_color_capabilities() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let query = xtgettcap_query(&[
            b"Tc".as_slice(),
            b"Smulx".as_slice(),
            b"Setulc".as_slice(),
            b"sitm".as_slice(),
            b"ritm".as_slice(),
            b"Smol".as_slice(),
            b"smxx".as_slice(),
            b"rmxx".as_slice(),
            b"op".as_slice(),
            b"oc".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            xtgettcap_response(&[
                (b"Tc".as_slice(), b"1".as_slice()),
                (b"Smulx".as_slice(), b"\x1b[4:%p1%dm".as_slice()),
                (
                    b"Setulc".as_slice(),
                    b"\x1b[58:2::%p1%{65536}%/%d:%p1%{256}%/%{255}%&%d:%p1%{255}%&%d%;m".as_slice()
                ),
                (b"sitm".as_slice(), b"\x1b[3m".as_slice()),
                (b"ritm".as_slice(), b"\x1b[23m".as_slice()),
                (b"Smol".as_slice(), b"\x1b[53m".as_slice()),
                (b"smxx".as_slice(), b"\x1b[9m".as_slice()),
                (b"rmxx".as_slice(), b"\x1b[29m".as_slice()),
                (b"op".as_slice(), b"\x1b[39;49m".as_slice()),
                (b"oc".as_slice(), b"\x1b]104\x07".as_slice()),
            ])
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_wezterm_official_boolean_capabilities() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let query = xtgettcap_query(&[
            b"am".as_slice(),
            b"bce".as_slice(),
            b"ccc".as_slice(),
            b"hs".as_slice(),
            b"mc5i".as_slice(),
            b"mir".as_slice(),
            b"msgr".as_slice(),
            b"npc".as_slice(),
            b"Su".as_slice(),
            b"xenl".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            xtgettcap_response(&[
                (b"am".as_slice(), b"1".as_slice()),
                (b"bce".as_slice(), b"1".as_slice()),
                (b"ccc".as_slice(), b"1".as_slice()),
                (b"hs".as_slice(), b"1".as_slice()),
                (b"mc5i".as_slice(), b"1".as_slice()),
                (b"mir".as_slice(), b"1".as_slice()),
                (b"msgr".as_slice(), b"1".as_slice()),
                (b"npc".as_slice(), b"1".as_slice()),
                (b"Su".as_slice(), b"1".as_slice()),
                (b"xenl".as_slice(), b"1".as_slice()),
            ])
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_wezterm_official_printer_memory_and_reset_templates()
     {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let query = xtgettcap_query(&[
            b"flash".as_slice(),
            b"mc0".as_slice(),
            b"mc4".as_slice(),
            b"mc5".as_slice(),
            b"meml".as_slice(),
            b"memu".as_slice(),
            b"rs1".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            xtgettcap_response(&[
                (b"flash".as_slice(), b"\x1b[?5h$<100/>\x1b[?5l".as_slice()),
                (b"mc0".as_slice(), b"\x1b[i".as_slice()),
                (b"mc4".as_slice(), b"\x1b[4i".as_slice()),
                (b"mc5".as_slice(), b"\x1b[5i".as_slice()),
                (b"meml".as_slice(), b"\x1bl".as_slice()),
                (b"memu".as_slice(), b"\x1bm".as_slice()),
                (b"rs1".as_slice(), b"\x1bc\x1b]104\x07".as_slice()),
            ])
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_wezterm_title_and_palette_templates() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let query = xtgettcap_query(&[
            b"dsl".as_slice(),
            b"fsl".as_slice(),
            b"tsl".as_slice(),
            b"initc".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            xtgettcap_response(&[
                (b"dsl".as_slice(), b"\x1b]2;\x1b\\".as_slice()),
                (b"fsl".as_slice(), b"\x1b\\".as_slice()),
                (b"tsl".as_slice(), b"\x1b]0;".as_slice()),
                (
                    b"initc".as_slice(),
                    b"\x1b]4;%p1%d;rgb:%p2%{255}%*%{1000}%/%2.2X/%p3%{255}%*%{1000}%/%2.2X/%p4%{255}%*%{1000}%/%2.2X\x1b\\".as_slice()
                ),
            ])
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_tmux_cursor_capabilities() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let query = xtgettcap_query(&[
            b"Cr".as_slice(),
            b"Cs".as_slice(),
            b"Se".as_slice(),
            b"Ss".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            xtgettcap_response(&[
                (b"Cr".as_slice(), b"\x1b]112\x07".as_slice()),
                (b"Cs".as_slice(), b"\x1b]12;%p1%s\x07".as_slice()),
                (b"Se".as_slice(), b"\x1b[2 q".as_slice()),
                (b"Ss".as_slice(), b"\x1b[%p1%d q".as_slice()),
            ])
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_synchronized_output_capability() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let query = xtgettcap_query(&[b"Sync".as_slice()]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            xtgettcap_response(&[(
                b"Sync".as_slice(),
                b"\x1b[?2026%?%p1%{1}%-%tl%eh%;".as_slice()
            )])
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_mouse_capabilities() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let query = xtgettcap_query(&[b"kmous".as_slice(), b"XM".as_slice(), b"xm".as_slice()]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            xtgettcap_response(&[
                (b"kmous".as_slice(), b"\x1b[<".as_slice()),
                (
                    b"XM".as_slice(),
                    b"\x1b[?1006;1000%?%p1%{1}%=%th%el%;".as_slice()
                ),
                (
                    b"xm".as_slice(),
                    b"\x1b[<%i%p3%d;%p1%d;%p2%d;%?%p4%tM%em%;".as_slice()
                ),
            ])
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_foundational_terminal_capabilities() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let query = xtgettcap_query(&[
            b"clear".as_slice(),
            b"cup".as_slice(),
            b"home".as_slice(),
            b"civis".as_slice(),
            b"cnorm".as_slice(),
            b"cvvis".as_slice(),
            b"smcup".as_slice(),
            b"rmcup".as_slice(),
            b"sgr0".as_slice(),
            b"sgr".as_slice(),
            b"bold".as_slice(),
            b"dim".as_slice(),
            b"blink".as_slice(),
            b"rev".as_slice(),
            b"smso".as_slice(),
            b"rmso".as_slice(),
            b"invis".as_slice(),
            b"smul".as_slice(),
            b"rmul".as_slice(),
            b"setaf".as_slice(),
            b"setab".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            xtgettcap_response(&[
                (b"clear".as_slice(), b"\x1b[H\x1b[2J".as_slice()),
                (b"cup".as_slice(), b"\x1b[%i%p1%d;%p2%dH".as_slice()),
                (b"home".as_slice(), b"\x1b[H".as_slice()),
                (b"civis".as_slice(), b"\x1b[?25l".as_slice()),
                (b"cnorm".as_slice(), b"\x1b[?12l\x1b[?25h".as_slice()),
                (b"cvvis".as_slice(), b"\x1b[?12;25h".as_slice()),
                (b"smcup".as_slice(), b"\x1b[?1049h\x1b[22;0;0t".as_slice()),
                (b"rmcup".as_slice(), b"\x1b[?1049l\x1b[23;0;0t".as_slice()),
                (b"sgr0".as_slice(), b"\x1b(B\x1b[m".as_slice()),
                (
                    b"sgr".as_slice(),
                    b"%?%p9%t\x1b(0%e\x1b(B%;\x1b[0%?%p6%t;1%;%?%p5%t;2%;%?%p2%t;4%;%?%p1%p3%|%t;7%;%?%p4%t;5%;%?%p7%t;8%;m".as_slice()
                ),
                (b"bold".as_slice(), b"\x1b[1m".as_slice()),
                (b"dim".as_slice(), b"\x1b[2m".as_slice()),
                (b"blink".as_slice(), b"\x1b[5m".as_slice()),
                (b"rev".as_slice(), b"\x1b[7m".as_slice()),
                (b"smso".as_slice(), b"\x1b[7m".as_slice()),
                (b"rmso".as_slice(), b"\x1b[27m".as_slice()),
                (b"invis".as_slice(), b"\x1b[8m".as_slice()),
                (b"smul".as_slice(), b"\x1b[4m".as_slice()),
                (b"rmul".as_slice(), b"\x1b[24m".as_slice()),
                (
                    b"setaf".as_slice(),
                    b"\x1b[%?%p1%{8}%<%t3%p1%d%e%p1%{16}%<%t9%p1%{8}%-%d%e38;5;%p1%d%;m".as_slice()
                ),
                (
                    b"setab".as_slice(),
                    b"\x1b[%?%p1%{8}%<%t4%p1%d%e%p1%{16}%<%t10%p1%{8}%-%d%e48;5;%p1%d%;m".as_slice()
                ),
            ])
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_common_control_capabilities() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let query = xtgettcap_query(&[
            b"el".as_slice(),
            b"ed".as_slice(),
            b"el1".as_slice(),
            b"dch1".as_slice(),
            b"ich1".as_slice(),
            b"il1".as_slice(),
            b"dl1".as_slice(),
            b"cuu".as_slice(),
            b"cud".as_slice(),
            b"cub".as_slice(),
            b"cuf".as_slice(),
            b"hpa".as_slice(),
            b"vpa".as_slice(),
            b"cbt".as_slice(),
            b"ht".as_slice(),
            b"hts".as_slice(),
            b"tbc".as_slice(),
            b"ech".as_slice(),
            b"rep".as_slice(),
            b"csr".as_slice(),
            b"indn".as_slice(),
            b"rin".as_slice(),
            b"smir".as_slice(),
            b"rmir".as_slice(),
            b"smam".as_slice(),
            b"rmam".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            xtgettcap_response(&[
                (b"el".as_slice(), b"\x1b[K".as_slice()),
                (b"ed".as_slice(), b"\x1b[J".as_slice()),
                (b"el1".as_slice(), b"\x1b[1K".as_slice()),
                (b"dch1".as_slice(), b"\x1b[P".as_slice()),
                (b"ich1".as_slice(), b"\x1b[@".as_slice()),
                (b"il1".as_slice(), b"\x1b[L".as_slice()),
                (b"dl1".as_slice(), b"\x1b[M".as_slice()),
                (b"cuu".as_slice(), b"\x1b[%p1%dA".as_slice()),
                (b"cud".as_slice(), b"\x1b[%p1%dB".as_slice()),
                (b"cub".as_slice(), b"\x1b[%p1%dD".as_slice()),
                (b"cuf".as_slice(), b"\x1b[%p1%dC".as_slice()),
                (b"hpa".as_slice(), b"\x1b[%i%p1%dG".as_slice()),
                (b"vpa".as_slice(), b"\x1b[%i%p1%dd".as_slice()),
                (b"cbt".as_slice(), b"\x1b[Z".as_slice()),
                (b"ht".as_slice(), b"\t".as_slice()),
                (b"hts".as_slice(), b"\x1bH".as_slice()),
                (b"tbc".as_slice(), b"\x1b[3g".as_slice()),
                (b"ech".as_slice(), b"\x1b[%p1%dX".as_slice()),
                (b"rep".as_slice(), b"%p1%c\x1b[%p2%{1}%-%db".as_slice()),
                (b"csr".as_slice(), b"\x1b[%i%p1%d;%p2%dr".as_slice()),
                (b"indn".as_slice(), b"\x1b[%p1%dS".as_slice()),
                (b"rin".as_slice(), b"\x1b[%p1%dT".as_slice()),
                (b"smir".as_slice(), b"\x1b[4h".as_slice()),
                (b"rmir".as_slice(), b"\x1b[4l".as_slice()),
                (b"smam".as_slice(), b"\x1b[?7h".as_slice()),
                (b"rmam".as_slice(), b"\x1b[?7l".as_slice()),
            ])
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_common_key_capabilities() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let query = xtgettcap_query(&[
            b"kcuu1".as_slice(),
            b"kcud1".as_slice(),
            b"kcuf1".as_slice(),
            b"kcub1".as_slice(),
            b"kb2".as_slice(),
            b"kbs".as_slice(),
            b"kcbt".as_slice(),
            b"khome".as_slice(),
            b"kend".as_slice(),
            b"kich1".as_slice(),
            b"kdch1".as_slice(),
            b"kpp".as_slice(),
            b"knp".as_slice(),
            b"kHOM".as_slice(),
            b"kEND".as_slice(),
            b"kIC".as_slice(),
            b"kDC".as_slice(),
            b"kPRV".as_slice(),
            b"kNXT".as_slice(),
            b"kLFT".as_slice(),
            b"kRIT".as_slice(),
            b"kri".as_slice(),
            b"kind".as_slice(),
            b"kent".as_slice(),
            b"kf1".as_slice(),
            b"kf2".as_slice(),
            b"kf3".as_slice(),
            b"kf4".as_slice(),
            b"kf5".as_slice(),
            b"kf6".as_slice(),
            b"kf7".as_slice(),
            b"kf8".as_slice(),
            b"kf9".as_slice(),
            b"kf10".as_slice(),
            b"kf11".as_slice(),
            b"kf12".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            xtgettcap_response(&[
                (b"kcuu1".as_slice(), b"\x1bOA".as_slice()),
                (b"kcud1".as_slice(), b"\x1bOB".as_slice()),
                (b"kcuf1".as_slice(), b"\x1bOC".as_slice()),
                (b"kcub1".as_slice(), b"\x1bOD".as_slice()),
                (b"kb2".as_slice(), b"\x1bOE".as_slice()),
                (b"kbs".as_slice(), b"\x7f".as_slice()),
                (b"kcbt".as_slice(), b"\x1b[Z".as_slice()),
                (b"khome".as_slice(), b"\x1bOH".as_slice()),
                (b"kend".as_slice(), b"\x1bOF".as_slice()),
                (b"kich1".as_slice(), b"\x1b[2~".as_slice()),
                (b"kdch1".as_slice(), b"\x1b[3~".as_slice()),
                (b"kpp".as_slice(), b"\x1b[5~".as_slice()),
                (b"knp".as_slice(), b"\x1b[6~".as_slice()),
                (b"kHOM".as_slice(), b"\x1b[1;2H".as_slice()),
                (b"kEND".as_slice(), b"\x1b[1;2F".as_slice()),
                (b"kIC".as_slice(), b"\x1b[2;2~".as_slice()),
                (b"kDC".as_slice(), b"\x1b[3;2~".as_slice()),
                (b"kPRV".as_slice(), b"\x1b[5;2~".as_slice()),
                (b"kNXT".as_slice(), b"\x1b[6;2~".as_slice()),
                (b"kLFT".as_slice(), b"\x1b[1;2D".as_slice()),
                (b"kRIT".as_slice(), b"\x1b[1;2C".as_slice()),
                (b"kri".as_slice(), b"\x1b[1;2A".as_slice()),
                (b"kind".as_slice(), b"\x1b[1;2B".as_slice()),
                (b"kent".as_slice(), b"\x1bOM".as_slice()),
                (b"kf1".as_slice(), b"\x1bOP".as_slice()),
                (b"kf2".as_slice(), b"\x1bOQ".as_slice()),
                (b"kf3".as_slice(), b"\x1bOR".as_slice()),
                (b"kf4".as_slice(), b"\x1bOS".as_slice()),
                (b"kf5".as_slice(), b"\x1b[15~".as_slice()),
                (b"kf6".as_slice(), b"\x1b[17~".as_slice()),
                (b"kf7".as_slice(), b"\x1b[18~".as_slice()),
                (b"kf8".as_slice(), b"\x1b[19~".as_slice()),
                (b"kf9".as_slice(), b"\x1b[20~".as_slice()),
                (b"kf10".as_slice(), b"\x1b[21~".as_slice()),
                (b"kf11".as_slice(), b"\x1b[23~".as_slice()),
                (b"kf12".as_slice(), b"\x1b[24~".as_slice()),
            ])
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_wezterm_keypad_transmit_capabilities() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let query = xtgettcap_query(&[b"smkx".as_slice(), b"rmkx".as_slice()]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            xtgettcap_response(&[
                (b"smkx".as_slice(), b"\x1b[?1h\x1b=".as_slice()),
                (b"rmkx".as_slice(), b"\x1b[?1l\x1b>".as_slice()),
            ])
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_modified_function_key_capabilities() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let entries: &[(&[u8], &[u8])] = &[
            (b"kf13".as_slice(), b"\x1b[1;2P".as_slice()),
            (b"kf14".as_slice(), b"\x1b[1;2Q".as_slice()),
            (b"kf15".as_slice(), b"\x1b[1;2R".as_slice()),
            (b"kf16".as_slice(), b"\x1b[1;2S".as_slice()),
            (b"kf17".as_slice(), b"\x1b[15;2~".as_slice()),
            (b"kf18".as_slice(), b"\x1b[17;2~".as_slice()),
            (b"kf19".as_slice(), b"\x1b[18;2~".as_slice()),
            (b"kf20".as_slice(), b"\x1b[19;2~".as_slice()),
            (b"kf21".as_slice(), b"\x1b[20;2~".as_slice()),
            (b"kf22".as_slice(), b"\x1b[21;2~".as_slice()),
            (b"kf23".as_slice(), b"\x1b[23;2~".as_slice()),
            (b"kf24".as_slice(), b"\x1b[24;2~".as_slice()),
            (b"kf25".as_slice(), b"\x1b[1;5P".as_slice()),
            (b"kf26".as_slice(), b"\x1b[1;5Q".as_slice()),
            (b"kf27".as_slice(), b"\x1b[1;5R".as_slice()),
            (b"kf28".as_slice(), b"\x1b[1;5S".as_slice()),
            (b"kf29".as_slice(), b"\x1b[15;5~".as_slice()),
            (b"kf30".as_slice(), b"\x1b[17;5~".as_slice()),
            (b"kf31".as_slice(), b"\x1b[18;5~".as_slice()),
            (b"kf32".as_slice(), b"\x1b[19;5~".as_slice()),
            (b"kf33".as_slice(), b"\x1b[20;5~".as_slice()),
            (b"kf34".as_slice(), b"\x1b[21;5~".as_slice()),
            (b"kf35".as_slice(), b"\x1b[23;5~".as_slice()),
            (b"kf36".as_slice(), b"\x1b[24;5~".as_slice()),
            (b"kf37".as_slice(), b"\x1b[1;6P".as_slice()),
            (b"kf38".as_slice(), b"\x1b[1;6Q".as_slice()),
            (b"kf39".as_slice(), b"\x1b[1;6R".as_slice()),
            (b"kf40".as_slice(), b"\x1b[1;6S".as_slice()),
            (b"kf41".as_slice(), b"\x1b[15;6~".as_slice()),
            (b"kf42".as_slice(), b"\x1b[17;6~".as_slice()),
            (b"kf43".as_slice(), b"\x1b[18;6~".as_slice()),
            (b"kf44".as_slice(), b"\x1b[19;6~".as_slice()),
            (b"kf45".as_slice(), b"\x1b[20;6~".as_slice()),
            (b"kf46".as_slice(), b"\x1b[21;6~".as_slice()),
            (b"kf47".as_slice(), b"\x1b[23;6~".as_slice()),
            (b"kf48".as_slice(), b"\x1b[24;6~".as_slice()),
            (b"kf49".as_slice(), b"\x1b[1;3P".as_slice()),
            (b"kf50".as_slice(), b"\x1b[1;3Q".as_slice()),
            (b"kf51".as_slice(), b"\x1b[1;3R".as_slice()),
            (b"kf52".as_slice(), b"\x1b[1;3S".as_slice()),
            (b"kf53".as_slice(), b"\x1b[15;3~".as_slice()),
            (b"kf54".as_slice(), b"\x1b[17;3~".as_slice()),
            (b"kf55".as_slice(), b"\x1b[18;3~".as_slice()),
            (b"kf56".as_slice(), b"\x1b[19;3~".as_slice()),
            (b"kf57".as_slice(), b"\x1b[20;3~".as_slice()),
            (b"kf58".as_slice(), b"\x1b[21;3~".as_slice()),
            (b"kf59".as_slice(), b"\x1b[23;3~".as_slice()),
            (b"kf60".as_slice(), b"\x1b[24;3~".as_slice()),
            (b"kf61".as_slice(), b"\x1b[1;4P".as_slice()),
            (b"kf62".as_slice(), b"\x1b[1;4Q".as_slice()),
            (b"kf63".as_slice(), b"\x1b[1;4R".as_slice()),
        ];
        let names: Vec<&[u8]> = entries.iter().map(|(name, _)| *name).collect();
        let query = xtgettcap_query(&names);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, xtgettcap_response(entries));
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_acs_capabilities() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let query = xtgettcap_query(&[
            b"enacs".as_slice(),
            b"smacs".as_slice(),
            b"rmacs".as_slice(),
            b"acsc".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            xtgettcap_response(&[
                (b"enacs".as_slice(), b"\x1b)0".as_slice()),
                (b"smacs".as_slice(), b"\x1b(0".as_slice()),
                (b"rmacs".as_slice(), b"\x1b(B".as_slice()),
                (
                    b"acsc".as_slice(),
                    b"``aaffggiijjkkllmmnnooppqqrrssttuuvvwwxxyyzz{{||}}~~".as_slice()
                ),
            ])
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_wezterm_control_sequence_capabilities() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let query = xtgettcap_query(&[
            b"bel".as_slice(),
            b"cr".as_slice(),
            b"ind".as_slice(),
            b"ri".as_slice(),
            b"sc".as_slice(),
            b"rc".as_slice(),
            b"cuu1".as_slice(),
            b"cud1".as_slice(),
            b"cuf1".as_slice(),
            b"cub1".as_slice(),
            b"dch".as_slice(),
            b"ich".as_slice(),
            b"dl".as_slice(),
            b"il".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            xtgettcap_response(&[
                (b"bel".as_slice(), b"\x07".as_slice()),
                (b"cr".as_slice(), b"\r".as_slice()),
                (b"ind".as_slice(), b"\n".as_slice()),
                (b"ri".as_slice(), b"\x1bM".as_slice()),
                (b"sc".as_slice(), b"\x1b7".as_slice()),
                (b"rc".as_slice(), b"\x1b8".as_slice()),
                (b"cuu1".as_slice(), b"\x1b[A".as_slice()),
                (b"cud1".as_slice(), b"\n".as_slice()),
                (b"cuf1".as_slice(), b"\x1b[C".as_slice()),
                (b"cub1".as_slice(), b"\x08".as_slice()),
                (b"dch".as_slice(), b"\x1b[%p1%dP".as_slice()),
                (b"ich".as_slice(), b"\x1b[%p1%d@".as_slice()),
                (b"dl".as_slice(), b"\x1b[%p1%dM".as_slice()),
                (b"il".as_slice(), b"\x1b[%p1%dL".as_slice()),
            ])
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_wezterm_meta_key_capabilities() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let query = xtgettcap_query(&[b"km".as_slice(), b"smm".as_slice(), b"rmm".as_slice()]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            xtgettcap_response(&[
                (b"km".as_slice(), b"1".as_slice()),
                (b"smm".as_slice(), b"\x1b[?1034h".as_slice()),
                (b"rmm".as_slice(), b"\x1b[?1034l".as_slice()),
            ])
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_wezterm_reset_templates() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let query = xtgettcap_query(&[b"is2".as_slice(), b"rs2".as_slice()]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            xtgettcap_response(&[
                (
                    b"is2".as_slice(),
                    b"\x1b[!p\x1b[?3;4l\x1b[4l\x1b>".as_slice()
                ),
                (
                    b"rs2".as_slice(),
                    b"\x1b[!p\x1b[?3;4l\x1b[4l\x1b>".as_slice()
                ),
            ])
        );
    }

    #[test]
    fn terminal_output_filter_answers_xtgettcap_wezterm_query_templates() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let query = xtgettcap_query(&[
            b"u6".as_slice(),
            b"u7".as_slice(),
            b"u8".as_slice(),
            b"u9".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        filter
            .write(&input, &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(
            responses,
            xtgettcap_response(&[
                (b"u6".as_slice(), b"\x1b[%i%d;%dR".as_slice()),
                (b"u7".as_slice(), b"\x1b[6n".as_slice()),
                (b"u8".as_slice(), b"\x1b[?%[;0123456789]c".as_slice()),
                (b"u9".as_slice(), b"\x1b[c".as_slice()),
            ])
        );
    }

    #[test]
    fn terminal_output_filter_answers_decrqss_state_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b[1;2;4:3;5;8;9;53;74;58;5;34;38;6;4;5;6;7;48;2;1;2;3m\x1bP$qm\x1b\\ middle\x1b[5 q\x90$q q\x9c after\x1b[2;5r\x1bP$qr\x1b\\done",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(
            output,
            b"before\x1b[1;2;4:3;5;8;9;53;74;58;5;34;38;6;4;5;6;7;48;2;1;2;3m middle\x1b[5 q after\x1b[2;5rdone"
        );
        assert_eq!(
            responses,
            b"\x1bP1$r1;2;4:3;5;8;9;53;74;58;5;34;38;6;4;5;6;7;48;2;1;2;3m\x1b\\\x1bP1$r5 q\x9c\x1bP1$r2;5r\x1b\\"
        );
    }

    #[test]
    fn terminal_output_filter_answers_wezterm_decrqss_conformance_and_left_right_margins() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1bP$q\"p\x1b\\ middle\x90$qs\x9c after",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before middle after");
        assert_eq!(responses, b"\x1bP1$r61;1\"p\x1b\\\x1bP1$r1;80s\x9c");
    }

    #[test]
    fn terminal_output_filter_answers_utf8_c1_dcs_decrqss_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let dcs = '\u{90}';
        let st = '\u{9c}';
        let input = format!("before{dcs}$q\"p{st} middle{dcs}$qs{st} after");

        filter
            .write(input.as_bytes(), &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before middle after");
        assert_eq!(responses, b"\x1bP1$r61;1\"p\x9c\x1bP1$r1;80s\x9c");
    }

    #[test]
    fn terminal_output_filter_answers_split_utf8_c1_dcs_decrqss_query() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\xc2", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write(b"\x90$q\"p\xc2", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write(b"\x9cafter", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, b"\x1bP1$r61;1\"p\x9c");
    }

    #[test]
    fn terminal_output_filter_resynchronizes_queries_after_escape_in_utf8_c1_dcs() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let dcs = '\u{90}';
        let st = '\u{9c}';
        let input = format!("before{dcs}payload \x1b[6n{st}after");

        filter
            .write(input.as_bytes(), &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, b"\x1b[1;7R");
    }

    #[test]
    fn terminal_output_filter_answers_split_wezterm_decrqss_conformance_and_left_right_margins() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1bP$q\"", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write(b"p\x1b\\ middle\x90$q", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write(b"s\x9c after", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before middle after");
        assert_eq!(responses, b"\x1bP1$r61;1\"p\x1b\\\x1bP1$r1;80s\x9c");
    }

    #[test]
    fn terminal_output_filter_answers_decrqss_left_right_margin_query_from_declrmm_state() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b[?69h\x1b[3;6s\x1bP$qs\x1b\\after",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before\x1b[?69h\x1b[3;6safter");
        assert_eq!(responses, b"\x1bP1$r3;6s\x1b\\");
    }

    #[test]
    fn terminal_output_filter_answers_xtversion_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b[>q middle\x1b[>0q after\x9b>q done",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before middle after done");
        assert_eq!(
            responses,
            b"\x1bP>|R-Term 0.1.0\x1b\\\x1bP>|R-Term 0.1.0\x1b\\\x1bP>|R-Term 0.1.0\x1b\\"
        );
    }

    #[test]
    fn terminal_output_filter_writes_osc52_clipboard_text() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let mut writes = Vec::new();

        filter
            .write_with_clipboard(
                b"before\x1b]52;c;Y29weQ==\x07after",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |text| {
                    writes.push(text.to_owned());
                    true
                },
                || Some("ignored".to_owned()),
                Osc52Policy::ReadWrite,
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert!(responses.is_empty());
        assert_eq!(writes, vec!["copy"]);
    }

    #[test]
    fn terminal_output_filter_writes_iterm_copy_clipboard_text() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let mut writes = Vec::new();

        filter
            .write_with_clipboard(
                b"before\x1b]1337;Copy=;Y29weQ==\x07after",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |text| {
                    writes.push(text.to_owned());
                    true
                },
                || Some("ignored".to_owned()),
                Osc52Policy::ReadWrite,
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert!(responses.is_empty());
        assert_eq!(writes, vec!["copy"]);
    }

    #[test]
    fn terminal_output_filter_consumes_wezterm_osc9_and_osc777_notifications() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"before\x1b]9;build done\x07 middle\x1b]9;4;1;42\x07 more\x9d777;notify;Build;failed\x9c after",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before middle more after");
        assert!(responses.is_empty());
    }

    #[test]
    fn terminal_output_filter_writes_c1_osc52_clipboard_text() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let mut writes = Vec::new();

        filter
            .write_with_clipboard(
                b"before\x9d52;c;Y29weQ==\x9cafter",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |text| {
                    writes.push(text.to_owned());
                    true
                },
                || Some("ignored".to_owned()),
                Osc52Policy::ReadWrite,
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert!(responses.is_empty());
        assert_eq!(writes, vec!["copy"]);
    }

    #[test]
    fn terminal_output_filter_writes_c1_iterm_copy_clipboard_text() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let mut writes = Vec::new();

        filter
            .write_with_clipboard(
                b"before\x9d1337;Copy=;Y29weQ==\x9cafter",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |text| {
                    writes.push(text.to_owned());
                    true
                },
                || Some("ignored".to_owned()),
                Osc52Policy::ReadWrite,
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert!(responses.is_empty());
        assert_eq!(writes, vec!["copy"]);
    }

    #[test]
    fn terminal_output_filter_writes_utf8_c1_osc52_clipboard_text() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let mut writes = Vec::new();

        filter
            .write_with_clipboard(
                "before\u{9d}52;c;Y29weQ==\u{9c}after".as_bytes(),
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |text| {
                    writes.push(text.to_owned());
                    true
                },
                || Some("ignored".to_owned()),
                Osc52Policy::ReadWrite,
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert!(responses.is_empty());
        assert_eq!(writes, vec!["copy"]);
    }

    #[test]
    fn terminal_output_filter_writes_utf8_c1_iterm_copy_clipboard_text() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let mut writes = Vec::new();

        filter
            .write_with_clipboard(
                "before\u{9d}1337;Copy=;Y29weQ==\u{9c}after".as_bytes(),
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |text| {
                    writes.push(text.to_owned());
                    true
                },
                || Some("ignored".to_owned()),
                Osc52Policy::ReadWrite,
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert!(responses.is_empty());
        assert_eq!(writes, vec!["copy"]);
    }

    #[test]
    fn terminal_output_filter_resynchronizes_osc52_after_non_st_escape() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let mut writes = Vec::new();

        filter
            .write_with_clipboard(
                b"\x1b]0;title \x1b]52;c;Y29weQ==\x07\x1bPpayload \x1b]52;c;?\x1b\\done",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |text| {
                    writes.push(text.to_owned());
                    true
                },
                || Some("copy".to_owned()),
                Osc52Policy::ReadWrite,
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"done");
        assert_eq!(responses, b"\x1b]52;c;Y29weQ==\x07");
        assert_eq!(writes, vec!["copy"]);
    }

    #[test]
    fn terminal_output_filter_writes_split_c1_osc52_clipboard_text() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let mut writes = Vec::new();

        filter
            .write_with_clipboard(
                b"before\x9d52;c;Y2",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |text| {
                    writes.push(text.to_owned());
                    true
                },
                || Some("ignored".to_owned()),
                Osc52Policy::ReadWrite,
            )
            .unwrap();
        assert_eq!(output, b"before");
        assert!(writes.is_empty());

        filter
            .write_with_clipboard(
                b"9weQ==\x9cafter",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |text| {
                    writes.push(text.to_owned());
                    true
                },
                || Some("ignored".to_owned()),
                Osc52Policy::ReadWrite,
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert!(responses.is_empty());
        assert_eq!(writes, vec!["copy"]);
    }

    #[test]
    fn terminal_output_filter_writes_split_utf8_c1_osc52_clipboard_text() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let mut writes = Vec::new();

        filter
            .write_with_clipboard(
                b"before\xc2",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |text| {
                    writes.push(text.to_owned());
                    true
                },
                || Some("ignored".to_owned()),
                Osc52Policy::ReadWrite,
            )
            .unwrap();
        filter
            .write_with_clipboard(
                b"\x9d52;c;Y29weQ==\xc2",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |text| {
                    writes.push(text.to_owned());
                    true
                },
                || Some("ignored".to_owned()),
                Osc52Policy::ReadWrite,
            )
            .unwrap();
        filter
            .write_with_clipboard(
                b"\x9cafter",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |text| {
                    writes.push(text.to_owned());
                    true
                },
                || Some("ignored".to_owned()),
                Osc52Policy::ReadWrite,
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert!(responses.is_empty());
        assert_eq!(writes, vec!["copy"]);
    }

    #[test]
    fn terminal_output_filter_answers_osc52_clipboard_query() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write_with_clipboard(
                b"before\x1b]52;c;?\x07after",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |_| true,
                || Some("copy".to_owned()),
                Osc52Policy::ReadWrite,
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, b"\x1b]52;c;Y29weQ==\x07");
    }

    #[test]
    fn terminal_output_filter_answers_c1_osc52_clipboard_query() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write_with_clipboard(
                b"before\x9d52;c;?\x9cafter",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |_| true,
                || Some("copy".to_owned()),
                Osc52Policy::ReadWrite,
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, b"\x1b]52;c;Y29weQ==\x07");
    }

    #[test]
    fn terminal_output_filter_answers_utf8_c1_osc52_clipboard_query() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write_with_clipboard(
                "before\u{9d}52;c;?\u{9c}after".as_bytes(),
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |_| true,
                || Some("copy".to_owned()),
                Osc52Policy::ReadWrite,
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, b"\x1b]52;c;Y29weQ==\x07");
    }

    #[test]
    fn terminal_output_filter_drops_incomplete_osc52_on_flush() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let mut writes = Vec::new();

        filter
            .write_with_clipboard(
                b"before\x1b]52;c;Y29weQ==",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |text| {
                    writes.push(text.to_owned());
                    true
                },
                || Some("copy".to_owned()),
                Osc52Policy::ReadWrite,
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before");
        assert!(responses.is_empty());
        assert!(writes.is_empty());
    }

    #[test]
    fn terminal_output_filter_drops_partial_osc52_prefix_on_flush() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let mut writes = Vec::new();

        filter
            .write_with_clipboard(
                b"before\x1b]52",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |text| {
                    writes.push(text.to_owned());
                    true
                },
                || Some("copy".to_owned()),
                Osc52Policy::ReadWrite,
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before");
        assert!(responses.is_empty());
        assert!(writes.is_empty());
    }

    #[test]
    fn terminal_output_filter_blocks_osc52_when_policy_is_off() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let mut writes = Vec::new();

        filter
            .write_with_clipboard(
                b"before\x1b]52;c;Y29weQ==\x07 middle\x1b]52;c;?\x07after",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |text| {
                    writes.push(text.to_owned());
                    true
                },
                || Some("copy".to_owned()),
                Osc52Policy::Off,
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before middleafter");
        assert!(responses.is_empty());
        assert!(writes.is_empty());
    }

    #[test]
    fn terminal_output_filter_write_only_osc52_policy_blocks_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let mut writes = Vec::new();

        filter
            .write_with_clipboard(
                b"before\x1b]52;c;Y29weQ==\x07 middle\x1b]52;c;?\x07after",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
                |text| {
                    writes.push(text.to_owned());
                    true
                },
                || Some("copy".to_owned()),
                Osc52Policy::WriteOnly,
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"before middleafter");
        assert!(responses.is_empty());
        assert_eq!(writes, vec!["copy"]);
    }

    #[test]
    fn terminal_output_filter_answers_c1_private_mode_status_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x9b?1000;1006h\x1b[?2004;2026h\x9b?1000$p \x9b?1006$p \x9b?2004$p \x1b[?2026$p",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"\x9b?1000;1006h   ");
        assert_eq!(
            responses,
            b"\x1b[?1000;1$y\x1b[?1006;1$y\x1b[?2004;1$y\x1b[?2026;1$y"
        );
    }

    #[test]
    fn terminal_output_filter_answers_c1_wezterm_private_mode_reports() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(
                b"\x9b?2027$p\x9b?1070h\x9b?1070$p",
                &mut output,
                |response| {
                    responses.extend_from_slice(response);
                    Ok(())
                },
            )
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"\x9b?1070h");
        assert_eq!(responses, b"\x1b[?2027;3$y\x1b[?1070;1$y");
    }

    #[test]
    fn terminal_output_filter_answers_c1_dec_ansi_mode_status_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"\x9b?2h\x9b?2$p", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"\x9b?2h");
        assert_eq!(responses, b"\x1b[?2;1$y");
    }

    #[test]
    fn terminal_output_filter_answers_utf8_c1_private_mode_status_queries() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();
        let csi = '\u{9b}';
        let input = format!("{csi}?45$p {csi}?45h{csi}?45$p");

        filter
            .write(input.as_bytes(), &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, format!(" {csi}?45h").as_bytes());
        assert_eq!(responses, b"\x1b[?45;2$y\x1b[?45;1$y");
    }

    #[test]
    fn terminal_output_filter_handles_split_cursor_position_query() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write(b"6nafter", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, b"\x1b[1;7R");
    }

    #[test]
    fn terminal_output_filter_handles_split_device_attribute_query() {
        let mut filter = TerminalOutputFilter::default();
        let mut output = Vec::new();
        let mut responses = Vec::new();

        filter
            .write(b"before\x1b[", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter
            .write(b">cafter", &mut output, |response| {
                responses.extend_from_slice(response);
                Ok(())
            })
            .unwrap();
        filter.flush(&mut output).unwrap();

        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, b"\x1b[>1;277;0c");
    }
}
