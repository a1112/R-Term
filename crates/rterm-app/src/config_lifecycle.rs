#![allow(dead_code)]

use std::{
    collections::BTreeSet,
    fmt, fs,
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

use notify::{Event, EventKind, RecommendedWatcher, Watcher};
use rterm_config::{
    ConfigLifecycle as ConfigLifecycleCore, ConfigLifecycleEvent, ConfigLoadAttempt,
    ConfigSnapshot, FixedWindowDebouncer, SourceChange,
};

pub(crate) use rterm_config::{
    ConfigDiscoveryInputs, ConfigSource, ConfigSourceError as NativeConfigSourceError,
    ResolvedConfigSource,
};

pub(crate) trait NativeConfigProjection: Clone + Default + PartialEq {
    fn project_canonical_config(source: &str) -> Option<Self>;

    fn automatically_reload_config(&self) -> Option<bool>;
}

macro_rules! bind_native_config_projection {
    ($payload:ty, $project:path, $reload:ident) => {
        type EffectiveNativeConfig = rterm_config::ConfigSnapshot<$payload>;
        type NativeConfigLifecycle = $crate::config_lifecycle::NativeConfigLifecycle<$payload>;

        impl $crate::config_lifecycle::NativeConfigProjection for $payload {
            fn project_canonical_config(source: &str) -> Option<Self> {
                $project(source)
            }

            fn automatically_reload_config(&self) -> Option<bool> {
                self.$reload
            }
        }
    };
}

pub(crate) use bind_native_config_projection;

const CONFIG_WATCH_DEBOUNCE: Duration = Duration::from_millis(200);

pub(crate) type ConfigFileChangedSink = Arc<dyn Fn() -> bool + Send + Sync + 'static>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeConfigWatchDiagnostic {
    pub(crate) path: Option<PathBuf>,
    pub(crate) detail: String,
}

enum NativeConfigWatcherMessage {
    Notify(notify::Result<Event>),
    Stop(mpsc::Sender<()>),
}

pub(crate) struct NativeConfigWatcher {
    watcher: Option<RecommendedWatcher>,
    worker_sender: Option<mpsc::Sender<NativeConfigWatcherMessage>>,
    worker: Option<thread::JoinHandle<()>>,
    watched_paths: BTreeSet<PathBuf>,
    diagnostics: Arc<Mutex<Vec<NativeConfigWatchDiagnostic>>>,
}

struct NativeConfigWatcherOptions {
    debounce: Duration,
    event_sink: ConfigFileChangedSink,
    worker_stopped: Option<mpsc::Sender<()>>,
}

impl NativeConfigWatcher {
    fn new(
        debounce: Duration,
        event_sink: ConfigFileChangedSink,
        worker_stopped: Option<mpsc::Sender<()>>,
    ) -> notify::Result<Self> {
        let (worker_sender, worker_receiver) = mpsc::channel();
        let callback_sender = worker_sender.clone();
        let watcher = notify::recommended_watcher(move |event| {
            let _ = callback_sender.send(NativeConfigWatcherMessage::Notify(event));
        })?;
        let diagnostics = Arc::new(Mutex::new(Vec::new()));
        let worker_diagnostics = Arc::clone(&diagnostics);
        let worker = thread::Builder::new()
            .name("rterm-config-watcher".to_owned())
            .spawn(move || {
                run_config_watcher_worker(
                    worker_receiver,
                    debounce,
                    event_sink,
                    &worker_diagnostics,
                );
                if let Some(worker_stopped) = worker_stopped {
                    let _ = worker_stopped.send(());
                }
            })
            .map_err(notify::Error::io)?;
        Ok(Self {
            watcher: Some(watcher),
            worker_sender: Some(worker_sender),
            worker: Some(worker),
            watched_paths: BTreeSet::new(),
            diagnostics,
        })
    }

    #[cfg(test)]
    fn new_for_test(
        debounce: Duration,
        event_sink: ConfigFileChangedSink,
        worker_stopped: Option<mpsc::Sender<()>>,
    ) -> notify::Result<Self> {
        Self::new(debounce, event_sink, worker_stopped)
    }

    #[cfg(test)]
    fn enqueue_relevant_event_for_test(&mut self) {
        let event = Event::new(EventKind::Modify(notify::event::ModifyKind::Any));
        self.enqueue_notify_event_for_test(event);
    }

    #[cfg(test)]
    fn enqueue_notify_event_for_test(&mut self, event: Event) {
        self.worker_sender
            .as_ref()
            .expect("live test watcher has a sender")
            .send(NativeConfigWatcherMessage::Notify(Ok(event)))
            .expect("live test watcher has a worker");
    }

    fn watch_path(&mut self, path: PathBuf) {
        if self.watched_paths.contains(&path) {
            return;
        }
        let Some(watcher) = self.watcher.as_mut() else {
            self.diagnostics
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(NativeConfigWatchDiagnostic {
                    path: Some(path),
                    detail: "watch registration attempted after watcher shutdown".to_owned(),
                });
            return;
        };
        match watcher.watch(&path, notify::RecursiveMode::NonRecursive) {
            Ok(()) => {
                self.watched_paths.insert(path);
            }
            Err(error) => {
                self.diagnostics
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push(NativeConfigWatchDiagnostic {
                        path: Some(path),
                        detail: error.to_string(),
                    });
            }
        }
    }
}

impl Drop for NativeConfigWatcher {
    fn drop(&mut self) {
        self.watcher.take();
        if let Some(sender) = self.worker_sender.take() {
            let (stopped_sender, stopped_receiver) = mpsc::channel();
            if sender
                .send(NativeConfigWatcherMessage::Stop(stopped_sender))
                .is_ok()
            {
                let _ = stopped_receiver.recv_timeout(Duration::from_secs(5));
            }
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run_config_watcher_worker(
    receiver: mpsc::Receiver<NativeConfigWatcherMessage>,
    debounce: Duration,
    event_sink: ConfigFileChangedSink,
    diagnostics: &Mutex<Vec<NativeConfigWatchDiagnostic>>,
) {
    let mut debouncer = FixedWindowDebouncer::new(debounce);
    'worker: while let Ok(message) = receiver.recv() {
        let change = match message {
            NativeConfigWatcherMessage::Notify(Ok(event)) => source_change_from_notify(&event),
            NativeConfigWatcherMessage::Notify(Err(error)) => {
                diagnostics
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push(NativeConfigWatchDiagnostic {
                        path: error.paths.first().cloned(),
                        detail: error.to_string(),
                    });
                SourceChange::Ignored
            }
            NativeConfigWatcherMessage::Stop(stopped) => {
                let _ = stopped.send(());
                break;
            }
        };
        if debouncer.observe(change, Instant::now()).is_none() {
            continue;
        }

        let mut stop = None;
        loop {
            let Some(remaining) = debouncer
                .remaining(Instant::now())
                .filter(|remaining| !remaining.is_zero())
            else {
                break;
            };
            match receiver.recv_timeout(remaining) {
                // Once a relevant event opens the fixed debounce window, coalesce every
                // subsequent successful notification in that burst.
                Ok(NativeConfigWatcherMessage::Notify(Ok(event))) => {
                    let _ = debouncer.observe(source_change_from_notify(&event), Instant::now());
                }
                Ok(NativeConfigWatcherMessage::Notify(Err(error))) => {
                    diagnostics
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .push(NativeConfigWatchDiagnostic {
                            path: error.paths.first().cloned(),
                            detail: error.to_string(),
                        });
                }
                Ok(NativeConfigWatcherMessage::Stop(stopped)) => {
                    stop = Some(stopped);
                    break;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => break 'worker,
            }
        }
        if let Some(stopped) = stop {
            let _ = stopped.send(());
            break;
        }
        if debouncer.take_ready(Instant::now()) {
            let _ = event_sink();
        }
    }
    drop(receiver);
    drop(event_sink);
}

pub(crate) type NativeConfigLoadAttempt<T> = ConfigLoadAttempt<T, NativeConfigLoadError>;
pub(crate) type EffectiveNativeConfig<T> = ConfigSnapshot<T>;

pub(crate) struct NativeConfigLifecycle<T: NativeConfigProjection> {
    core: ConfigLifecycleCore<T, NativeConfigLoadError>,
    cli: ValidatedNativeConfigAssignments<T>,
    watcher: Option<NativeConfigWatcher>,
    watcher_options: Option<NativeConfigWatcherOptions>,
    watcher_initialization_diagnostic: Option<NativeConfigWatchDiagnostic>,
    watch_current_dir: PathBuf,
}

fn source_change_from_notify(event: &Event) -> SourceChange {
    if matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ) {
        SourceChange::Changed
    } else {
        SourceChange::Ignored
    }
}

impl<T: NativeConfigProjection> NativeConfigLifecycle<T> {
    pub(crate) fn new(
        inputs: ConfigDiscoveryInputs,
        skip: bool,
        explicit: Option<PathBuf>,
        cli: ValidatedNativeConfigAssignments<T>,
    ) -> Self {
        let defaults = cli.default_overrides().clone();
        Self {
            core: ConfigLifecycleCore::new_with_initial(
                inputs,
                skip,
                explicit,
                T::default(),
                defaults,
            ),
            cli,
            watcher: None,
            watcher_options: None,
            watcher_initialization_diagnostic: None,
            watch_current_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    pub(crate) fn attempt_reload(&self) -> NativeConfigLoadAttempt<T> {
        self.core
            .attempt_reload(|source| parse_loaded_native_config_document(source, &self.cli))
    }

    pub(crate) fn reload_task(&self) -> impl FnOnce() -> NativeConfigLoadAttempt<T> + Send + 'static
    where
        T: Send + Sync + 'static,
    {
        let core = self.core.clone();
        let cli = self.cli.clone();
        move || core.attempt_reload(|source| parse_loaded_native_config_document(source, &cli))
    }

    pub(crate) fn validated_cli(&self) -> &[StaticNativeConfigAssignment] {
        self.cli.as_slice()
    }

    pub(crate) fn effective(&self) -> &EffectiveNativeConfig<T> {
        self.core.snapshot_ref()
    }

    pub(crate) fn latest_diagnostic(
        &self,
    ) -> Option<&NativeConfigSourceError<NativeConfigLoadError>> {
        self.core.latest_diagnostic()
    }

    pub(crate) fn latest_selection(&self) -> &ResolvedConfigSource {
        self.core.latest_selection()
    }

    pub(crate) fn install_initial_attempt(&mut self, attempt: NativeConfigLoadAttempt<T>) {
        let _ = self.core.install_attempt(attempt, |_, _| ());
        self.refresh_watched_paths();
    }

    pub(crate) fn install_runtime_attempt(&mut self, attempt: NativeConfigLoadAttempt<T>) -> bool {
        if self.latest_diagnostic().is_none()
            && self.latest_selection() == &attempt.resolved
            && self.effective().publication == attempt.publication
            && attempt
                .result
                .as_ref()
                .is_ok_and(|config| self.effective().config.as_ref() == config)
        {
            // File watchers can deliver a second notification after the fixed
            // debounce window for the same write. Treat the read as successful
            // without publishing a phantom configuration generation. A valid
            // read after a diagnostic still installs normally so that recovery
            // clears the diagnostic and advances the last-known-good state.
            return true;
        }
        let event = self.core.install_attempt(attempt, |_, _| ());
        self.refresh_watched_paths();
        matches!(event, ConfigLifecycleEvent::Applied { .. })
    }

    pub(crate) fn install_watcher_sink(
        &mut self,
        event_sink: ConfigFileChangedSink,
    ) -> Result<(), NativeConfigWatchDiagnostic> {
        self.install_watcher_sink_with_options(CONFIG_WATCH_DEBOUNCE, event_sink, None)
    }

    pub(crate) fn watch_diagnostics(&self) -> Vec<NativeConfigWatchDiagnostic> {
        let mut diagnostics = self
            .watcher_initialization_diagnostic
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        if let Some(watcher) = &self.watcher {
            diagnostics.extend(
                watcher
                    .diagnostics
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .iter()
                    .cloned(),
            );
        }
        diagnostics
    }

    fn install_watcher_sink_with_options(
        &mut self,
        debounce: Duration,
        event_sink: ConfigFileChangedSink,
        worker_stopped: Option<mpsc::Sender<()>>,
    ) -> Result<(), NativeConfigWatchDiagnostic> {
        if self.watcher.is_some() || self.watcher_options.is_some() {
            return Ok(());
        }
        self.watcher_options = Some(NativeConfigWatcherOptions {
            debounce,
            event_sink,
            worker_stopped,
        });
        self.refresh_watched_paths();
        if let Some(diagnostic) = self.watcher_initialization_diagnostic.clone() {
            Err(diagnostic)
        } else {
            Ok(())
        }
    }

    fn refresh_watched_paths(&mut self) {
        let enabled = if self.effective().generation == 0 {
            self.cli
                .default_overrides()
                .automatically_reload_config()
                .unwrap_or(true)
        } else {
            self.effective()
                .config
                .automatically_reload_config()
                .unwrap_or(true)
        };
        if !enabled {
            return;
        }
        let ResolvedConfigSource::File(source) = self.latest_selection() else {
            return;
        };
        let path = watcher_registration_path(&source.path, &self.watch_current_dir);
        let parent = path.parent().map(std::path::Path::to_path_buf);
        let home = self.core.inputs().home_dir.clone();
        if self.watcher.is_none() {
            let Some(options) = self.watcher_options.as_ref() else {
                return;
            };
            match NativeConfigWatcher::new(
                options.debounce,
                Arc::clone(&options.event_sink),
                options.worker_stopped.clone(),
            ) {
                Ok(watcher) => {
                    self.watcher = Some(watcher);
                    self.watcher_initialization_diagnostic = None;
                }
                Err(error) => {
                    self.watcher_initialization_diagnostic = Some(NativeConfigWatchDiagnostic {
                        path: None,
                        detail: error.to_string(),
                    });
                    return;
                }
            }
        }
        let Some(watcher) = self.watcher.as_mut() else {
            return;
        };
        watcher.watch_path(path);
        if let Some(parent) = parent
            && !home
                .as_deref()
                .is_some_and(|home| paths_refer_to_same_directory(home, &parent))
        {
            watcher.watch_path(parent);
        }
    }

    #[cfg(test)]
    pub(crate) fn install_watcher_sink_for_test(
        &mut self,
        debounce: Duration,
        event_sink: ConfigFileChangedSink,
        worker_stopped: Option<mpsc::Sender<()>>,
    ) -> Result<(), NativeConfigWatchDiagnostic> {
        self.install_watcher_sink_with_options(debounce, event_sink, worker_stopped)
    }

    #[cfg(test)]
    pub(crate) fn watched_paths_for_test(&self) -> BTreeSet<PathBuf> {
        self.watcher
            .as_ref()
            .map_or_else(BTreeSet::new, |watcher| watcher.watched_paths.clone())
    }

    #[cfg(test)]
    pub(crate) fn watcher_exists_for_test(&self) -> bool {
        self.watcher.is_some()
    }

    #[cfg(test)]
    pub(crate) fn enqueue_watcher_relevant_burst_for_test(&mut self, count: usize) {
        let watcher = self
            .watcher
            .as_mut()
            .expect("test lifecycle should own an active watcher");
        for _ in 0..count {
            watcher.enqueue_relevant_event_for_test();
        }
    }

    #[cfg(test)]
    pub(crate) fn watch_diagnostics_for_test(&self) -> Vec<NativeConfigWatchDiagnostic> {
        self.watch_diagnostics()
    }

    #[cfg(test)]
    pub(crate) fn set_watch_current_dir_for_test(&mut self, current_dir: PathBuf) {
        self.watch_current_dir = current_dir;
    }

    #[cfg(test)]
    pub(crate) fn install_source_for_test(&mut self, source: ConfigSource, config: T) {
        let attempt = NativeConfigLoadAttempt {
            preferred: Some(source.path.clone()),
            resolved: ResolvedConfigSource::File(source),
            result: Ok(config),
            publication: rterm_config::DerivedConfigEnvironment::default(),
        };
        self.install_initial_attempt(attempt);
    }

    fn candidate_sources(&self) -> Vec<ConfigSource> {
        self.core.candidate_sources()
    }

    #[cfg(test)]
    fn inputs(&self) -> &ConfigDiscoveryInputs {
        self.core.inputs()
    }
}

fn paths_refer_to_same_directory(left: &std::path::Path, right: &std::path::Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn watcher_registration_path(path: &std::path::Path, current_dir: &std::path::Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        current_dir.join(path)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceLocation {
    pub(crate) line: usize,
    pub(crate) column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum StaticLuaValue {
    Nil,
    Bool(bool),
    Integer(i64),
    Number(f64),
    String(String),
    Array(Vec<StaticLuaValue>),
    Table(Vec<(StaticLuaKey, StaticLuaValue)>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StaticLuaKey {
    String(String),
    Integer(i64),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StaticNativeConfigAssignment {
    pub(crate) field_path: Vec<String>,
    pub(crate) value: StaticLuaValue,
    pub(crate) value_source: String,
    pub(crate) location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ValidatedNativeConfigAssignments<T> {
    assignments: Vec<StaticNativeConfigAssignment>,
    default_overrides: T,
}

impl<T> std::ops::Deref for ValidatedNativeConfigAssignments<T> {
    type Target = [StaticNativeConfigAssignment];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T> ValidatedNativeConfigAssignments<T> {
    pub(crate) fn as_slice(&self) -> &[StaticNativeConfigAssignment] {
        &self.assignments
    }

    pub(crate) fn default_overrides(&self) -> &T {
        &self.default_overrides
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NativeConfigLoadError {
    InvalidSyntax {
        location: SourceLocation,
        message: String,
    },
    UnsupportedDynamicLua {
        location: SourceLocation,
        message: String,
    },
    UnknownField {
        location: SourceLocation,
        field: String,
    },
    InvalidFieldValue {
        location: SourceLocation,
        field: String,
        message: String,
    },
    InternalValidation {
        location: SourceLocation,
        message: String,
    },
}

impl fmt::Display for NativeConfigLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSyntax { location, message } => {
                write!(
                    formatter,
                    "{}:{}: invalid syntax: {message}",
                    location.line, location.column
                )
            }
            Self::UnsupportedDynamicLua { location, message } => write!(
                formatter,
                "{}:{}: unsupported dynamic Lua: {message}",
                location.line, location.column
            ),
            Self::UnknownField { location, field } => write!(
                formatter,
                "{}:{}: unknown config field `{field}`",
                location.line, location.column
            ),
            Self::InvalidFieldValue {
                location,
                field,
                message,
            } => write!(
                formatter,
                "{}:{}: invalid value for `{field}`: {message}",
                location.line, location.column
            ),
            Self::InternalValidation { location, message } => write!(
                formatter,
                "{}:{}: internal validation failed: {message}",
                location.line, location.column
            ),
        }
    }
}

impl std::error::Error for NativeConfigLoadError {}

pub(crate) fn validate_cli_config_overrides<T: NativeConfigProjection>(
    items: &[(String, String)],
) -> Result<ValidatedNativeConfigAssignments<T>, NativeConfigLoadError> {
    let mut assignments = Vec::with_capacity(items.len());
    for (field, source) in items {
        let mut parser = Parser::new(source);
        if parser.source.starts_with('\u{feff}') {
            parser.offset += '\u{feff}'.len_utf8();
        }
        parser.skip_trivia()?;
        let location = parser.location();
        let value = parser.parse_config_field_value(field)?;
        parser.skip_trivia()?;
        if !parser.is_eof() {
            return Err(parser.dynamic("unexpected trailing tokens after static CLI value"));
        }
        let assignment = StaticNativeConfigAssignment {
            field_path: vec![field.clone()],
            value,
            value_source: source.clone(),
            location,
        };
        validate_assignment(&assignment)?;
        assignments.push(assignment);
    }
    let default_overrides = parse_native_config_document("return {}", &assignments)?;
    Ok(ValidatedNativeConfigAssignments {
        assignments,
        default_overrides,
    })
}

pub(crate) fn parse_native_config_document<T: NativeConfigProjection>(
    source: &str,
    cli: &[StaticNativeConfigAssignment],
) -> Result<T, NativeConfigLoadError> {
    let mut assignments = Parser::new(source).parse_document()?;
    assignments.extend_from_slice(cli);
    if assignments.is_empty() {
        return Ok(T::default());
    }

    for assignment in &assignments {
        validate_assignment(assignment)?;
    }
    let canonical = canonical_document(&assignments);
    T::project_canonical_config(&canonical).ok_or_else(|| {
        NativeConfigLoadError::InternalValidation {
            location: SourceLocation { line: 1, column: 1 },
            message: "legacy extractor rejected strictly validated config".to_owned(),
        }
    })
}

fn parse_loaded_native_config_document<T: NativeConfigProjection>(
    source: &str,
    cli: &[StaticNativeConfigAssignment],
) -> Result<T, NativeConfigLoadError> {
    if source.starts_with('\u{feff}') {
        return Err(NativeConfigLoadError::InvalidSyntax {
            location: SourceLocation { line: 1, column: 1 },
            message: "unexpected second UTF-8 BOM".to_owned(),
        });
    }
    parse_native_config_document(source, cli)
}

fn validate_assignment(
    assignment: &StaticNativeConfigAssignment,
) -> Result<(), NativeConfigLoadError> {
    let field = assignment.field_path.join(".");
    let result = match field.as_str() {
        "term" | "default_cwd" | "default_workspace" | "color_scheme" => {
            validate_non_empty_string(&assignment.value)
        }
        "initial_cols" | "initial_rows" => {
            validate_integer_range(&assignment.value, 1, u64::from(u16::MAX))
        }
        "scrollback_lines" | "max_fps" => {
            validate_integer_range(&assignment.value, 0, usize::MAX as u64)
        }
        "automatically_reload_config" | "enable_tab_bar" => validate_bool(&assignment.value),
        "window_close_confirmation" => {
            validate_enum_string(&assignment.value, &["AlwaysPrompt", "NeverPrompt"])
        }
        "default_prog" | "default_gui_startup_args" => validate_string_array(&assignment.value),
        "colors" => validate_colors(&assignment.value),
        "set_environment_variables" => validate_environment(&assignment.value),
        "keys" => validate_keys(&assignment.value),
        _ => {
            return Err(NativeConfigLoadError::UnknownField {
                location: assignment.location,
                field,
            });
        }
    };
    result.map_err(|message| NativeConfigLoadError::InvalidFieldValue {
        location: assignment.location,
        field,
        message,
    })
}

fn validate_non_empty_string(value: &StaticLuaValue) -> Result<(), String> {
    match value {
        StaticLuaValue::String(value) if !value.is_empty() => Ok(()),
        StaticLuaValue::String(_) => Err("must not be empty".to_owned()),
        _ => Err("expected a string".to_owned()),
    }
}

fn validate_bool(value: &StaticLuaValue) -> Result<(), String> {
    match value {
        StaticLuaValue::Bool(_) => Ok(()),
        _ => Err("expected a boolean".to_owned()),
    }
}

fn validate_integer_range(
    value: &StaticLuaValue,
    minimum: u64,
    maximum: u64,
) -> Result<(), String> {
    let StaticLuaValue::Integer(value) = value else {
        return Err("expected an integer".to_owned());
    };
    let value = u64::try_from(*value).map_err(|_| "integer must not be negative".to_owned())?;
    if (minimum..=maximum).contains(&value) {
        Ok(())
    } else {
        Err(format!("integer must be in {minimum}..={maximum}"))
    }
}

fn validate_string_array(value: &StaticLuaValue) -> Result<(), String> {
    match value {
        StaticLuaValue::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                if !matches!(value, StaticLuaValue::String(_)) {
                    return Err(format!("array item {} must be a string", index + 1));
                }
            }
            Ok(())
        }
        StaticLuaValue::Table(entries) if entries.is_empty() => Ok(()),
        _ => Err("expected an array of strings".to_owned()),
    }
}

fn validate_colors(value: &StaticLuaValue) -> Result<(), String> {
    let entries = table_entries(value, "colors")?;
    reject_duplicate_keys(entries, "colors")?;
    for (key, value) in entries {
        let key = string_key(key, "colors")?;
        match key {
            "foreground" | "background" | "cursor_bg" | "cursor_fg" | "cursor_border"
            | "compose_cursor" | "selection_bg" => validate_color(value)?,
            "selection_fg" => match value {
                StaticLuaValue::String(value) if value.eq_ignore_ascii_case("none") => {}
                _ => validate_color(value)?,
            },
            "ansi" | "brights" => validate_color_array(value, key)?,
            "tab_bar" => validate_tab_bar(value)?,
            _ => return Err(format!("unknown colors key `{key}`")),
        }
    }
    Ok(())
}

fn validate_color(value: &StaticLuaValue) -> Result<(), String> {
    let StaticLuaValue::String(value) = value else {
        return Err("color must be a string".to_owned());
    };
    value
        .parse::<wezterm_color_types::SrgbaTuple>()
        .map(|_| ())
        .map_err(|()| format!("invalid color `{value}`"))
}

fn validate_color_array(value: &StaticLuaValue, field: &str) -> Result<(), String> {
    let StaticLuaValue::Array(values) = value else {
        return Err(format!("{field} must be an array"));
    };
    if values.len() != 8 {
        return Err(format!("{field} must contain exactly 8 colors"));
    }
    for value in values {
        validate_color(value)?;
    }
    Ok(())
}

fn validate_tab_bar(value: &StaticLuaValue) -> Result<(), String> {
    let entries = table_entries(value, "colors.tab_bar")?;
    reject_duplicate_keys(entries, "colors.tab_bar")?;
    for (key, value) in entries {
        let key = string_key(key, "colors.tab_bar")?;
        match key {
            "background" | "inactive_tab_edge" => validate_color(value)?,
            "active_tab" | "inactive_tab" | "inactive_tab_hover" | "new_tab" | "new_tab_hover" => {
                validate_tab_bar_item(value, key)?;
            }
            _ => return Err(format!("unknown colors.tab_bar key `{key}`")),
        }
    }
    Ok(())
}

fn validate_tab_bar_item(value: &StaticLuaValue, item: &str) -> Result<(), String> {
    let entries = table_entries(value, item)?;
    reject_duplicate_keys(entries, item)?;
    for (key, value) in entries {
        let key = string_key(key, item)?;
        match key {
            "fg_color" | "bg_color" => validate_color(value)?,
            "intensity" => validate_enum_string(value, &["Normal", "Bold", "Half"])?,
            "underline" => validate_enum_string(
                value,
                &["None", "Single", "Double", "Curly", "Dotted", "Dashed"],
            )?,
            "italic" | "strikethrough" => validate_bool(value)?,
            _ => return Err(format!("unknown colors.tab_bar.{item} key `{key}`")),
        }
    }
    Ok(())
}

fn validate_enum_string(value: &StaticLuaValue, allowed: &[&str]) -> Result<(), String> {
    let StaticLuaValue::String(value) = value else {
        return Err("expected a string".to_owned());
    };
    if allowed.contains(&value.as_str()) {
        Ok(())
    } else {
        Err(format!("unsupported value `{value}`"))
    }
}

fn validate_environment(value: &StaticLuaValue) -> Result<(), String> {
    let entries = table_entries(value, "set_environment_variables")?;
    reject_duplicate_keys(entries, "set_environment_variables")?;
    for (key, value) in entries {
        let key = string_key(key, "set_environment_variables")?;
        if key.is_empty() {
            return Err("environment variable name must not be empty".to_owned());
        }
        if !matches!(value, StaticLuaValue::String(_)) {
            return Err(format!(
                "environment variable `{key}` value must be a string"
            ));
        }
    }
    Ok(())
}

fn validate_keys(value: &StaticLuaValue) -> Result<(), String> {
    let values = match value {
        StaticLuaValue::Array(values) => values.as_slice(),
        StaticLuaValue::Table(entries) if entries.is_empty() => &[],
        _ => return Err("keys must be an array".to_owned()),
    };
    for (index, value) in values.iter().enumerate() {
        validate_key_entry(value)
            .map_err(|message| format!("keys item {}: {message}", index + 1))?;
    }
    Ok(())
}

fn validate_key_entry(value: &StaticLuaValue) -> Result<(), String> {
    let entries = table_entries(value, "key entry")?;
    reject_duplicate_keys(entries, "key entry")?;
    let mut key_seen = false;
    let mut modifiers_seen = false;
    let mut action_seen = false;
    for (key, value) in entries {
        match string_key(key, "key entry")? {
            "key" => {
                validate_non_empty_string(value)?;
                key_seen = true;
            }
            "mods" | "mod" => {
                if modifiers_seen {
                    return Err("duplicate modifier field via `mods`/`mod` alias".to_owned());
                }
                validate_modifiers(value)?;
                modifiers_seen = true;
            }
            "action" => {
                validate_action(value)?;
                action_seen = true;
            }
            key => return Err(format!("unknown key entry field `{key}`")),
        }
    }
    if !key_seen {
        return Err("missing `key`".to_owned());
    }
    if !action_seen {
        return Err("missing `action`".to_owned());
    }
    Ok(())
}

fn validate_modifiers(value: &StaticLuaValue) -> Result<(), String> {
    let StaticLuaValue::String(value) = value else {
        return Err("mods must be a string".to_owned());
    };
    if value.eq_ignore_ascii_case("NONE") {
        return Ok(());
    }
    for modifier in value.split(['|', '+']) {
        if !matches!(
            modifier.trim().to_ascii_uppercase().as_str(),
            "CTRL" | "SHIFT" | "ALT" | "SUPER" | "LEADER" | "CMD" | "WIN" | "OPT" | "META"
        ) {
            return Err(format!("unsupported modifier `{modifier}`"));
        }
    }
    Ok(())
}

fn validate_action(value: &StaticLuaValue) -> Result<(), String> {
    let entries = table_entries(value, "action")?;
    if entries.len() != 1 {
        return Err("action must contain exactly one supported action".to_owned());
    }
    let (key, payload) = &entries[0];
    match string_key(key, "action")? {
        "SendString" if matches!(payload, StaticLuaValue::String(_)) => Ok(()),
        "SendString" => Err("SendString payload must be a string".to_owned()),
        action => Err(format!("unsupported action `{action}`")),
    }
}

fn table_entries<'a>(
    value: &'a StaticLuaValue,
    context: &str,
) -> Result<&'a [(StaticLuaKey, StaticLuaValue)], String> {
    match value {
        StaticLuaValue::Table(entries) => Ok(entries),
        _ => Err(format!("{context} must be a table")),
    }
}

fn string_key<'a>(key: &'a StaticLuaKey, context: &str) -> Result<&'a str, String> {
    match key {
        StaticLuaKey::String(key) => Ok(key),
        StaticLuaKey::Integer(key) => Err(format!("{context} does not support integer key {key}")),
    }
}

fn reject_duplicate_keys(
    entries: &[(StaticLuaKey, StaticLuaValue)],
    context: &str,
) -> Result<(), String> {
    for (index, (key, _)) in entries.iter().enumerate() {
        if entries[..index].iter().any(|(previous, _)| previous == key) {
            return Err(format!("{context} contains duplicate key {key:?}"));
        }
    }
    Ok(())
}

fn canonical_document(assignments: &[StaticNativeConfigAssignment]) -> String {
    let mut output = String::from("return {\n");
    for (index, assignment) in assignments.iter().enumerate() {
        if assignments[index + 1..]
            .iter()
            .any(|later| later.field_path == assignment.field_path)
        {
            continue;
        }
        output.push_str(&assignment.field_path[0]);
        output.push('=');
        let context = if assignment.field_path[0] == "keys" {
            StaticValueContext::KeyBindings
        } else {
            StaticValueContext::General
        };
        write_canonical_value_with_context(&assignment.value, context, &mut output);
        output.push_str(",\n");
    }
    output.push('}');
    output
}

fn write_canonical_value(value: &StaticLuaValue, output: &mut String) {
    write_canonical_value_with_context(value, StaticValueContext::General, output);
}

fn write_canonical_value_with_context(
    value: &StaticLuaValue,
    context: StaticValueContext,
    output: &mut String,
) {
    match value {
        StaticLuaValue::Nil => output.push_str("nil"),
        StaticLuaValue::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        StaticLuaValue::Integer(value) => output.push_str(&value.to_string()),
        StaticLuaValue::Number(value) => output.push_str(&value.to_string()),
        StaticLuaValue::String(value) => {
            output.push('"');
            for character in value.chars() {
                match character {
                    '\\' => output.push_str("\\\\"),
                    '"' => output.push_str("\\\""),
                    '\n' => output.push_str("\\n"),
                    '\r' => output.push_str("\\r"),
                    '\t' => output.push_str("\\t"),
                    character if character.is_control() => {
                        std::fmt::Write::write_fmt(
                            output,
                            format_args!("\\u{{{:x}}}", character as u32),
                        )
                        .expect("writing to a String cannot fail");
                    }
                    character => output.push(character),
                }
            }
            output.push('"');
        }
        StaticLuaValue::Array(values) => {
            output.push('{');
            for value in values {
                write_canonical_value_with_context(value, context, output);
                output.push(',');
            }
            output.push('}');
        }
        StaticLuaValue::Table(entries) => {
            output.push('{');
            for (key, value) in entries {
                match key {
                    StaticLuaKey::String(key)
                        if is_identifier_start(key.chars().next().unwrap_or('_'))
                            && key.chars().all(is_identifier_continue) =>
                    {
                        if is_lua_reserved_keyword(key) {
                            output.push('[');
                            write_canonical_value(&StaticLuaValue::String(key.clone()), output);
                            output.push(']');
                        } else {
                            output.push_str(key);
                        }
                    }
                    StaticLuaKey::String(key) => {
                        output.push('[');
                        write_canonical_value(&StaticLuaValue::String(key.clone()), output);
                        output.push(']');
                    }
                    StaticLuaKey::Integer(key) => {
                        output.push('[');
                        output.push_str(&key.to_string());
                        output.push(']');
                    }
                }
                output.push('=');
                if let Some(payload) = (context == StaticValueContext::KeyBindings
                    && matches!(key, StaticLuaKey::String(key) if key == "action"))
                .then(|| static_send_string_payload(value))
                .flatten()
                {
                    write_canonical_action(payload, output);
                } else {
                    write_canonical_value_with_context(value, context, output);
                }
                output.push(',');
            }
            output.push('}');
        }
    }
}

fn static_send_string_payload(value: &StaticLuaValue) -> Option<&str> {
    let StaticLuaValue::Table(entries) = value else {
        return None;
    };
    match entries.as_slice() {
        [(StaticLuaKey::String(action), StaticLuaValue::String(payload))]
            if action == "SendString" =>
        {
            Some(payload)
        }
        _ => None,
    }
}

fn write_canonical_action(payload: &str, output: &mut String) {
    output.push_str("wezterm.action.SendString(");
    write_canonical_value(&StaticLuaValue::String(payload.to_owned()), output);
    output.push(')');
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StaticValueContext {
    General,
    KeyBindings,
}

struct Parser<'a> {
    source: &'a str,
    offset: usize,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self { source, offset: 0 }
    }

    fn parse_document(
        mut self,
    ) -> Result<Vec<StaticNativeConfigAssignment>, NativeConfigLoadError> {
        if self.source.starts_with('\u{feff}') {
            self.offset += '\u{feff}'.len_utf8();
        }
        self.skip_trivia()?;
        if self.consume_keyword("return") {
            self.skip_trivia()?;
            if self.peek() != Some('{') {
                return Err(self.dynamic("dynamic return root; expected a static table"));
            }
            let assignments = self.parse_root_config_table()?;
            self.skip_trivia()?;
            self.consume_char(';');
            self.skip_trivia()?;
            if !self.is_eof() {
                return Err(self.dynamic("unexpected trailing top-level statement"));
            }
            return Ok(assignments);
        }

        self.expect_keyword("local")?;
        self.skip_trivia()?;
        if self.parse_identifier()? != "wezterm" {
            return Err(self.dynamic(
                "the documented builder form must declare `local wezterm = require 'wezterm'`",
            ));
        }
        self.skip_trivia()?;
        self.expect_char('=')?;
        self.skip_trivia()?;
        self.parse_wezterm_require_expression()?;
        self.skip_trivia()?;
        self.consume_char(';');

        self.skip_trivia()?;
        if !self.consume_keyword("local") {
            return Err(self.dynamic(
                "only the documented config builder declarations are allowed before direct top-level statements",
            ));
        }
        self.skip_trivia()?;
        if self.parse_identifier()? != "config" {
            return Err(self.dynamic(
                "the documented builder form must declare `local config = wezterm.config_builder()`",
            ));
        }
        self.skip_trivia()?;
        self.expect_char('=')?;
        self.skip_trivia()?;
        self.parse_config_builder_expression()?;
        self.skip_trivia()?;
        self.consume_char(';');

        let mut assignments = Vec::new();
        loop {
            self.skip_trivia()?;
            if self.consume_keyword("return") {
                self.skip_trivia()?;
                self.expect_identifier("config")?;
                self.skip_trivia()?;
                self.consume_char(';');
                self.skip_trivia()?;
                if !self.is_eof() {
                    return Err(self.syntax("unexpected trailing top-level statement"));
                }
                return Ok(assignments);
            }

            let location = self.location();
            if self.parse_identifier()? != "config" {
                return Err(self.dynamic(
                    "only direct `config.FIELD = STATIC_VALUE` top-level statements are allowed",
                ));
            }
            self.expect_char('.')?;
            let field = self.parse_identifier()?;
            self.skip_trivia()?;
            self.expect_char('=')?;
            self.skip_trivia()?;
            let value_start = self.offset;
            let value = self.parse_config_field_value(&field)?;
            let value_source = self.source[value_start..self.offset].to_owned();
            assignments.push(StaticNativeConfigAssignment {
                field_path: vec![field],
                value,
                value_source,
                location,
            });
            self.skip_trivia()?;
            self.consume_char(';');
        }
    }

    fn parse_wezterm_require_expression(&mut self) -> Result<(), NativeConfigLoadError> {
        self.expect_identifier("require")?;
        self.skip_trivia()?;
        let module = if self.consume_char('(') {
            self.skip_trivia()?;
            let module = self.parse_string()?;
            self.skip_trivia()?;
            self.expect_char(')')?;
            module
        } else {
            self.parse_string()?
        };
        if module != "wezterm" {
            return Err(self.dynamic("config builder must require `wezterm`"));
        }
        Ok(())
    }

    fn parse_config_builder_expression(&mut self) -> Result<(), NativeConfigLoadError> {
        self.expect_identifier("wezterm")?;
        self.skip_trivia()?;
        self.expect_char('.')?;
        self.expect_identifier("config_builder")?;
        self.skip_trivia()?;
        self.expect_char('(')?;
        self.skip_trivia()?;
        self.expect_char(')')
    }

    fn parse_root_config_table(
        &mut self,
    ) -> Result<Vec<StaticNativeConfigAssignment>, NativeConfigLoadError> {
        self.expect_char('{')?;
        self.skip_trivia()?;
        let mut assignments = Vec::new();
        while !self.consume_char('}') {
            if self.is_eof() {
                return Err(self.syntax("unterminated config table"));
            }
            let location = self.location();
            let field = if self.peek() == Some('[') && self.long_bracket_level().is_none() {
                self.bump();
                self.skip_trivia()?;
                let StaticLuaValue::String(field) = self.parse_value()? else {
                    return Err(self.syntax("config table bracket key must be a string"));
                };
                self.skip_trivia()?;
                self.expect_char(']')?;
                field
            } else {
                self.parse_identifier()?
            };
            self.skip_trivia()?;
            self.expect_char('=')?;
            self.skip_trivia()?;
            let value_start = self.offset;
            let value = self.parse_config_field_value(&field)?;
            assignments.push(StaticNativeConfigAssignment {
                field_path: vec![field],
                value,
                value_source: self.source[value_start..self.offset].to_owned(),
                location,
            });
            self.skip_trivia()?;
            if self.is_eof() {
                return Err(self.syntax("unterminated config table"));
            }
            if self.consume_char(',') || self.consume_char(';') {
                self.skip_trivia()?;
            } else if self.peek() != Some('}') {
                return Err(self.syntax("expected `,`, `;`, or `}` in config table"));
            }
        }
        Ok(assignments)
    }

    fn parse_config_field_value(
        &mut self,
        field: &str,
    ) -> Result<StaticLuaValue, NativeConfigLoadError> {
        let context = if field == "keys" {
            StaticValueContext::KeyBindings
        } else {
            StaticValueContext::General
        };
        self.parse_value_with_context(context)
    }

    fn parse_value(&mut self) -> Result<StaticLuaValue, NativeConfigLoadError> {
        self.parse_value_with_context(StaticValueContext::General)
    }

    fn parse_value_with_context(
        &mut self,
        context: StaticValueContext,
    ) -> Result<StaticLuaValue, NativeConfigLoadError> {
        self.skip_trivia()?;
        match self.peek() {
            Some('\'' | '"') => self.parse_string().map(StaticLuaValue::String),
            Some('[') if self.long_bracket_level().is_some() => {
                self.parse_long_string().map(StaticLuaValue::String)
            }
            Some('{') => self.parse_table(context),
            Some('-' | '+' | '.' | '0'..='9') => self.parse_number(),
            Some(_) if self.consume_keyword("true") => Ok(StaticLuaValue::Bool(true)),
            Some(_) if self.consume_keyword("false") => Ok(StaticLuaValue::Bool(false)),
            Some(_) if self.consume_keyword("nil") => Ok(StaticLuaValue::Nil),
            Some(character) if is_identifier_start(character) => {
                Err(self.dynamic("variable-derived values are unsupported"))
            }
            Some(_) => Err(self.dynamic("value must be a static literal")),
            None => Err(self.syntax("expected static value")),
        }
    }

    fn parse_table(
        &mut self,
        context: StaticValueContext,
    ) -> Result<StaticLuaValue, NativeConfigLoadError> {
        self.expect_char('{')?;
        self.skip_trivia()?;
        if self.consume_char('}') {
            return Ok(StaticLuaValue::Table(Vec::new()));
        }
        let mut keyed_entries = Vec::new();
        let mut array_entries = Vec::new();
        while !self.consume_char('}') {
            if self.is_eof() {
                return Err(self.syntax("unterminated table literal"));
            }
            let item_start = self.offset;
            let named_key = if self.peek().is_some_and(is_identifier_start) {
                let key = self.parse_identifier()?;
                self.skip_trivia()?;
                if self.consume_char('=') {
                    Some(StaticLuaKey::String(key))
                } else {
                    self.offset = item_start;
                    None
                }
            } else {
                None
            };
            let bracket_key = if named_key.is_none()
                && self.peek() == Some('[')
                && self.long_bracket_level().is_none()
            {
                self.bump();
                let key = match self.parse_value()? {
                    StaticLuaValue::String(key) => StaticLuaKey::String(key),
                    StaticLuaValue::Integer(key) => StaticLuaKey::Integer(key),
                    _ => {
                        return Err(self.syntax("table bracket key must be a string or integer"));
                    }
                };
                self.skip_trivia()?;
                self.expect_char(']')?;
                self.skip_trivia()?;
                self.expect_char('=')?;
                Some(key)
            } else {
                None
            };
            if let Some(key) = named_key.or(bracket_key) {
                if !array_entries.is_empty() {
                    return Err(self.syntax("mixed keyed and positional tables are unsupported"));
                }
                let is_action = matches!(&key, StaticLuaKey::String(key) if key == "action");
                let value = if context == StaticValueContext::KeyBindings && is_action {
                    match self.parse_static_action()? {
                        Some(action) => action,
                        None => {
                            return Err(self.dynamic(
                                "action must be exactly `wezterm.action.SendString(STRING)`",
                            ));
                        }
                    }
                } else {
                    self.parse_value_with_context(context)?
                };
                keyed_entries.push((key, value));
            } else {
                if !keyed_entries.is_empty() {
                    return Err(self.syntax("mixed keyed and positional tables are unsupported"));
                }
                array_entries.push(self.parse_value_with_context(context)?);
            }
            self.skip_trivia()?;
            if self.is_eof() {
                return Err(self.syntax("unterminated table literal"));
            }
            if self.consume_char(',') || self.consume_char(';') {
                self.skip_trivia()?;
            } else if self.peek() != Some('}') {
                return Err(self.syntax("expected `,`, `;`, or `}`"));
            }
        }
        if keyed_entries.is_empty() && !array_entries.is_empty() {
            Ok(StaticLuaValue::Array(array_entries))
        } else {
            Ok(StaticLuaValue::Table(keyed_entries))
        }
    }

    fn parse_static_action(&mut self) -> Result<Option<StaticLuaValue>, NativeConfigLoadError> {
        let start = self.offset;
        self.skip_trivia()?;
        if !self.consume_keyword("wezterm") {
            self.offset = start;
            return Ok(None);
        }
        if self.expect_char('.').is_err()
            || self.expect_identifier("action").is_err()
            || self.expect_char('.').is_err()
            || self.expect_identifier("SendString").is_err()
        {
            return Err(self.dynamic("action must be exactly `wezterm.action.SendString(STRING)`"));
        }
        self.skip_trivia()?;
        if !self.consume_char('(') {
            return Err(self.dynamic("action must be exactly `wezterm.action.SendString(STRING)`"));
        }
        let payload = self.parse_value()?;
        if !matches!(payload, StaticLuaValue::String(_)) {
            return Err(self.dynamic("SendString payload must be one static string"));
        }
        self.skip_trivia()?;
        if !self.consume_char(')') {
            return Err(self.dynamic("SendString requires exactly one parenthesized string"));
        }
        Ok(Some(StaticLuaValue::Table(vec![(
            StaticLuaKey::String("SendString".to_owned()),
            payload,
        )])))
    }

    fn parse_string(&mut self) -> Result<String, NativeConfigLoadError> {
        let quote = self
            .bump()
            .filter(|character| matches!(character, '\'' | '"'))
            .ok_or_else(|| self.syntax("expected string literal"))?;
        let mut output = String::new();
        while let Some(character) = self.bump() {
            if character == quote {
                return Ok(output);
            }
            if character != '\\' {
                if matches!(character, '\n' | '\r') {
                    return Err(self.syntax("short string literals cannot contain raw newlines"));
                }
                output.push(character);
                continue;
            }
            let escaped = self
                .bump()
                .ok_or_else(|| self.syntax("unterminated string escape"))?;
            match escaped {
                'a' => output.push('\x07'),
                'b' => output.push('\x08'),
                'f' => output.push('\x0c'),
                'n' | '\n' => output.push('\n'),
                'r' => output.push('\r'),
                't' => output.push('\t'),
                'v' => output.push('\x0b'),
                '\\' => output.push('\\'),
                '\'' => output.push('\''),
                '"' => output.push('"'),
                '\r' => {
                    self.consume_char('\n');
                    output.push('\n');
                }
                'z' => {
                    while self.peek().is_some_and(char::is_whitespace) {
                        self.bump();
                    }
                }
                'x' => {
                    let value = self.parse_fixed_radix_digits(2, 16)?;
                    output.push(
                        char::from_u32(value)
                            .ok_or_else(|| self.syntax("invalid hexadecimal string escape"))?,
                    );
                }
                'u' => {
                    self.expect_char('{')?;
                    let start = self.offset;
                    while self
                        .peek()
                        .is_some_and(|character| character.is_ascii_hexdigit())
                    {
                        self.bump();
                    }
                    if start == self.offset {
                        return Err(self.syntax("empty unicode string escape"));
                    }
                    let value = u32::from_str_radix(&self.source[start..self.offset], 16)
                        .map_err(|_| self.syntax("invalid unicode string escape"))?;
                    self.expect_char('}')?;
                    output.push(
                        char::from_u32(value)
                            .ok_or_else(|| self.syntax("invalid unicode scalar value"))?,
                    );
                }
                digit if digit.is_ascii_digit() => {
                    let mut digits = String::from(digit);
                    for _ in 0..2 {
                        if self
                            .peek()
                            .is_some_and(|character| character.is_ascii_digit())
                        {
                            digits.push(self.bump().unwrap());
                        }
                    }
                    let value = digits
                        .parse::<u32>()
                        .map_err(|_| self.syntax("invalid decimal string escape"))?;
                    output.push(
                        char::from_u32(value)
                            .filter(|_| value <= 255)
                            .ok_or_else(|| self.syntax("decimal string escape exceeds 255"))?,
                    );
                }
                other => {
                    return Err(self.syntax(&format!("unknown short string escape `\\{other}`")));
                }
            }
        }
        Err(self.syntax("unterminated string literal"))
    }

    fn parse_long_string(&mut self) -> Result<String, NativeConfigLoadError> {
        let (level, opener_len) = self
            .long_bracket_level()
            .ok_or_else(|| self.syntax("expected long string"))?;
        self.offset += opener_len;
        let content_start = self.offset;
        let closer = format!("]{}]", "=".repeat(level));
        let Some(relative_end) = self.remaining().find(&closer) else {
            return Err(self.syntax("unterminated long string"));
        };
        let content_end = self.offset + relative_end;
        let mut content = &self.source[content_start..content_end];
        if let Some(after_newline) = content.strip_prefix("\r\n") {
            content = after_newline;
        } else if let Some(after_newline) = content.strip_prefix('\n') {
            content = after_newline;
        }
        self.offset = content_end + closer.len();
        Ok(content.to_owned())
    }

    fn parse_number(&mut self) -> Result<StaticLuaValue, NativeConfigLoadError> {
        let start = self.offset;
        self.consume_char('+');
        self.consume_char('-');
        let mut has_digits = false;
        while self
            .peek()
            .is_some_and(|character| character.is_ascii_digit())
        {
            has_digits = true;
            self.bump();
        }
        let mut is_float = false;
        if self.consume_char('.') {
            is_float = true;
            while self
                .peek()
                .is_some_and(|character| character.is_ascii_digit())
            {
                has_digits = true;
                self.bump();
            }
        }
        if !has_digits {
            return Err(self.syntax("invalid number literal"));
        }
        if self
            .peek()
            .is_some_and(|character| matches!(character, 'e' | 'E'))
        {
            is_float = true;
            self.bump();
            if self
                .peek()
                .is_some_and(|character| matches!(character, '+' | '-'))
            {
                self.bump();
            }
            let exponent_start = self.offset;
            while self
                .peek()
                .is_some_and(|character| character.is_ascii_digit())
            {
                self.bump();
            }
            if exponent_start == self.offset {
                return Err(self.syntax("number exponent requires digits"));
            }
        }
        if self.peek().is_some_and(is_identifier_continue) {
            return Err(self.syntax("invalid number suffix"));
        }
        let text = &self.source[start..self.offset];
        if !is_float {
            return text
                .parse::<i64>()
                .map(StaticLuaValue::Integer)
                .map_err(|_| self.syntax("integer is outside the supported i64 range"));
        }
        let number = text
            .parse::<f64>()
            .map_err(|_| self.syntax("invalid floating-point number"))?;
        if number.is_finite() {
            Ok(StaticLuaValue::Number(number))
        } else {
            Err(self.syntax("non-finite numbers are unsupported"))
        }
    }

    fn parse_fixed_radix_digits(
        &mut self,
        count: usize,
        radix: u32,
    ) -> Result<u32, NativeConfigLoadError> {
        let start = self.offset;
        for _ in 0..count {
            if !self
                .peek()
                .is_some_and(|character| character.is_digit(radix))
            {
                return Err(self.syntax("incomplete string escape"));
            }
            self.bump();
        }
        u32::from_str_radix(&self.source[start..self.offset], radix)
            .map_err(|_| self.syntax("invalid string escape"))
    }

    fn skip_trivia(&mut self) -> Result<(), NativeConfigLoadError> {
        loop {
            while self.peek().is_some_and(char::is_whitespace) {
                self.bump();
            }
            if !self.remaining().starts_with("--") {
                return Ok(());
            }
            self.offset += 2;
            if self.long_bracket_level().is_some() {
                self.parse_long_string()?;
                continue;
            }
            while let Some(character) = self.bump() {
                if character == '\n' {
                    break;
                }
            }
        }
    }

    fn long_bracket_level(&self) -> Option<(usize, usize)> {
        let bytes = self.remaining().as_bytes();
        if bytes.first() != Some(&b'[') {
            return None;
        }
        let mut index = 1;
        while bytes.get(index) == Some(&b'=') {
            index += 1;
        }
        (bytes.get(index) == Some(&b'[')).then_some((index - 1, index + 1))
    }

    fn parse_identifier(&mut self) -> Result<String, NativeConfigLoadError> {
        self.skip_trivia()?;
        let Some(first) = self.peek() else {
            return Err(self.syntax("expected identifier"));
        };
        if !is_identifier_start(first) {
            return Err(self.syntax("expected identifier"));
        }
        let start = self.offset;
        self.bump();
        while self.peek().is_some_and(is_identifier_continue) {
            self.bump();
        }
        Ok(self.source[start..self.offset].to_owned())
    }

    fn expect_identifier(&mut self, expected: &str) -> Result<(), NativeConfigLoadError> {
        let actual = self.parse_identifier()?;
        if actual == expected {
            Ok(())
        } else {
            Err(self.syntax(&format!("expected `{expected}`")))
        }
    }

    fn consume_keyword(&mut self, keyword: &str) -> bool {
        let remaining = self.remaining();
        if !remaining.starts_with(keyword) {
            return false;
        }
        let end = keyword.len();
        if remaining[end..]
            .chars()
            .next()
            .is_some_and(is_identifier_continue)
        {
            return false;
        }
        self.offset += end;
        true
    }

    fn expect_keyword(&mut self, keyword: &str) -> Result<(), NativeConfigLoadError> {
        if self.consume_keyword(keyword) {
            Ok(())
        } else {
            Err(self.syntax(&format!("expected `{keyword}`")))
        }
    }

    fn consume_char(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<(), NativeConfigLoadError> {
        self.skip_trivia()?;
        if self.consume_char(expected) {
            Ok(())
        } else {
            Err(self.syntax(&format!("expected `{expected}`")))
        }
    }

    fn remaining(&self) -> &'a str {
        &self.source[self.offset..]
    }

    fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.offset += character.len_utf8();
        Some(character)
    }

    fn is_eof(&self) -> bool {
        self.offset == self.source.len()
    }

    fn location(&self) -> SourceLocation {
        let prefix = &self.source[..self.offset];
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let column = prefix
            .rsplit_once('\n')
            .map_or(prefix, |(_, tail)| tail)
            .chars()
            .count()
            + 1;
        SourceLocation { line, column }
    }

    fn syntax(&self, message: &str) -> NativeConfigLoadError {
        NativeConfigLoadError::InvalidSyntax {
            location: self.location(),
            message: message.to_owned(),
        }
    }

    fn dynamic(&self, message: &str) -> NativeConfigLoadError {
        NativeConfigLoadError::UnsupportedDynamicLua {
            location: self.location(),
            message: message.to_owned(),
        }
    }
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

fn is_identifier_continue(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn is_lua_reserved_keyword(value: &str) -> bool {
    matches!(
        value,
        "and"
            | "break"
            | "do"
            | "else"
            | "elseif"
            | "end"
            | "false"
            | "for"
            | "function"
            | "goto"
            | "if"
            | "in"
            | "local"
            | "nil"
            | "not"
            | "or"
            | "repeat"
            | "return"
            | "then"
            | "true"
            | "until"
            | "while"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::NativeConfigSnapshot;
    use rterm_config::{
        ConfigEnvironmentSnapshot, ConfigSourceErrorKind as NativeConfigSourceErrorKind,
    };
    #[cfg(unix)]
    use std::ffi::OsString;
    use std::{
        fs,
        ops::Deref,
        path::{Path, PathBuf},
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
            mpsc,
        },
        time::Duration,
    };

    type TestNativeConfigLifecycle = super::NativeConfigLifecycle<NativeConfigSnapshot>;
    type TestValidatedNativeConfigAssignments =
        super::ValidatedNativeConfigAssignments<NativeConfigSnapshot>;

    fn parse_test_native_config_document(
        source: &str,
        cli: &[StaticNativeConfigAssignment],
    ) -> Result<NativeConfigSnapshot, NativeConfigLoadError> {
        super::parse_native_config_document(source, cli)
    }

    fn validate_test_cli_config_overrides(
        items: &[(String, String)],
    ) -> Result<TestValidatedNativeConfigAssignments, NativeConfigLoadError> {
        super::validate_cli_config_overrides(items)
    }

    struct TestDir(PathBuf);

    impl Deref for TestDir {
        type Target = Path;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn unique_temp_dir(label: &str) -> TestDir {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rterm-config-lifecycle-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        TestDir(path)
    }

    #[test]
    fn watcher_coalesces_modify_burst_into_one_reload_event() {
        let (event_sender, event_receiver) = mpsc::channel();
        let mut watcher = NativeConfigWatcher::new_for_test(
            Duration::from_millis(10),
            Arc::new(move || event_sender.send(()).is_ok()),
            None,
        )
        .unwrap();

        for _ in 0..3 {
            watcher.enqueue_relevant_event_for_test();
        }

        event_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("one debounced reload event");
        assert!(
            event_receiver
                .recv_timeout(Duration::from_millis(50))
                .is_err(),
            "one burst must not enqueue more than one reload"
        );
    }

    #[test]
    fn watcher_ready_deadline_is_not_starved_by_queued_notifications() {
        let (message_sender, message_receiver) = mpsc::channel();
        for _ in 0..2 {
            message_sender
                .send(NativeConfigWatcherMessage::Notify(Ok(Event::new(
                    EventKind::Modify(notify::event::ModifyKind::Any),
                ))))
                .unwrap();
        }
        drop(message_sender);

        let (event_sender, event_receiver) = mpsc::channel();
        let diagnostics = Mutex::new(Vec::new());
        run_config_watcher_worker(
            message_receiver,
            Duration::ZERO,
            Arc::new(move || event_sender.send(()).is_ok()),
            &diagnostics,
        );

        assert_eq!(
            event_receiver.try_iter().count(),
            2,
            "a ready fixed-window deadline must fire before later queued notifications"
        );
    }

    #[test]
    fn watcher_accepts_create_modify_remove_and_ignores_other_kinds() {
        use notify::event::{AccessKind, CreateKind, ModifyKind, RemoveKind, RenameMode};

        let (event_sender, event_receiver) = mpsc::channel();
        let mut watcher = NativeConfigWatcher::new_for_test(
            Duration::from_millis(1),
            Arc::new(move || event_sender.send(()).is_ok()),
            None,
        )
        .unwrap();
        let relevant = [
            Event::new(EventKind::Create(CreateKind::File)),
            Event::new(EventKind::Modify(ModifyKind::Data(
                notify::event::DataChange::Content,
            ))),
            Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both))),
            Event::new(EventKind::Remove(RemoveKind::File)),
        ];
        for event in relevant {
            watcher.enqueue_notify_event_for_test(event);
            event_receiver
                .recv_timeout(Duration::from_secs(1))
                .expect("create/modify/rename/remove should enqueue reload");
        }

        watcher.enqueue_notify_event_for_test(Event::new(EventKind::Access(AccessKind::Read)));
        watcher.enqueue_notify_event_for_test(Event::new(EventKind::Other));
        assert!(
            event_receiver
                .recv_timeout(Duration::from_millis(30))
                .is_err(),
            "access and other events must be ignored"
        );
    }

    #[test]
    fn watcher_watches_attempted_invalid_source_and_parent() {
        let root = unique_temp_dir("watch-invalid-source");
        let config_dir = root.join("config");
        fs::create_dir_all(&config_dir).unwrap();
        let path = config_dir.join("wezterm.lua");
        fs::write(&path, "return dynamic_config").unwrap();
        let mut lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: false,
                current_exe: None,
                home_dir: Some(root.join("home")),
                xdg_config_home: None,
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            Some(path.clone()),
            TestValidatedNativeConfigAssignments::default(),
        );
        let attempt = lifecycle.attempt_reload();
        lifecycle.install_initial_attempt(attempt);
        assert_eq!(lifecycle.effective().generation, 0);

        lifecycle
            .install_watcher_sink_for_test(Duration::from_millis(1), Arc::new(|| true), None)
            .unwrap();

        let watched = lifecycle.watched_paths_for_test();
        assert!(watched.contains(&path));
        assert!(watched.contains(&config_dir));
    }

    #[test]
    fn watcher_skips_home_parent_but_watches_home_file() {
        let root = unique_temp_dir("watch-home-source");
        let home = root.join("home");
        fs::create_dir_all(&home).unwrap();
        let path = home.join(".wezterm.lua");
        fs::write(&path, "return { automatically_reload_config = true }").unwrap();
        let mut lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: false,
                current_exe: None,
                home_dir: Some(home.clone()),
                xdg_config_home: None,
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            None,
            TestValidatedNativeConfigAssignments::default(),
        );
        let attempt = lifecycle.attempt_reload();
        lifecycle.install_initial_attempt(attempt);

        lifecycle
            .install_watcher_sink_for_test(Duration::from_millis(1), Arc::new(|| true), None)
            .unwrap();

        let watched = lifecycle.watched_paths_for_test();
        assert!(watched.contains(&path));
        assert!(
            !watched.contains(&home),
            "the user's home directory is intentionally too noisy to watch"
        );
    }

    #[test]
    fn watcher_normalizes_bare_relative_source_to_injected_current_directory() {
        let root = unique_temp_dir("watch-relative-source");
        let relative = PathBuf::from("wezterm.lua");
        let absolute = root.join(&relative);
        fs::write(&absolute, "return {}").unwrap();
        let mut lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: false,
                current_exe: None,
                home_dir: None,
                xdg_config_home: None,
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            None,
            TestValidatedNativeConfigAssignments::default(),
        );
        lifecycle.set_watch_current_dir_for_test(root.0.clone());
        let mut config = NativeConfigSnapshot::default();
        config.automatically_reload_config = Some(true);
        lifecycle.install_source_for_test(
            ConfigSource {
                path: relative.clone(),
                required: true,
            },
            config,
        );

        lifecycle
            .install_watcher_sink_for_test(Duration::from_millis(1), Arc::new(|| true), None)
            .unwrap();

        let watched = lifecycle.watched_paths_for_test();
        assert!(watched.contains(&absolute));
        assert!(watched.contains(&root.0));
        assert_eq!(
            lifecycle.effective().source.as_ref(),
            Some(&relative),
            "watch registration must not rewrite config source semantics"
        );
    }

    #[test]
    fn watcher_observes_atomic_replacement_for_bare_relative_source() {
        let root = unique_temp_dir("watch-relative-atomic-replace");
        let relative = PathBuf::from("wezterm.lua");
        let absolute = root.join(&relative);
        let replacement = root.join("wezterm.lua.replacement");
        fs::write(&absolute, "return { term = 'before' }").unwrap();
        fs::write(&replacement, "return { term = 'after' }").unwrap();
        let mut lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: false,
                current_exe: None,
                home_dir: None,
                xdg_config_home: None,
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            None,
            TestValidatedNativeConfigAssignments::default(),
        );
        lifecycle.set_watch_current_dir_for_test(root.0.clone());
        let mut config = NativeConfigSnapshot::default();
        config.automatically_reload_config = Some(true);
        lifecycle.install_source_for_test(
            ConfigSource {
                path: relative,
                required: true,
            },
            config,
        );
        let (event_sender, event_receiver) = mpsc::channel();
        lifecycle
            .install_watcher_sink_for_test(
                Duration::from_millis(20),
                Arc::new(move || event_sender.send(()).is_ok()),
                None,
            )
            .unwrap();

        fs::remove_file(&absolute).unwrap();
        fs::rename(&replacement, &absolute).unwrap();

        event_receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("watching the normalized parent must observe atomic replacement");
    }

    #[test]
    fn watcher_is_created_lazily_only_for_enabled_attempted_source() {
        let mut lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: false,
                current_exe: None,
                home_dir: None,
                xdg_config_home: None,
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            None,
            TestValidatedNativeConfigAssignments::default(),
        );
        let attempt = lifecycle.attempt_reload();
        lifecycle.install_initial_attempt(attempt);

        lifecycle
            .install_watcher_sink_for_test(Duration::from_millis(1), Arc::new(|| true), None)
            .unwrap();

        assert!(!lifecycle.watcher_exists_for_test());
    }

    #[test]
    fn failed_initial_attempt_uses_cli_only_auto_reload_policy() {
        let root = unique_temp_dir("watch-initial-cli-policy");
        let path = root.join("wezterm.lua");
        fs::write(&path, "return dynamic_config").unwrap();
        let lifecycle_with_cli = |enabled: bool| {
            let cli = validate_test_cli_config_overrides(&[
                (
                    "automatically_reload_config".to_owned(),
                    enabled.to_string(),
                ),
                ("term".to_owned(), "'cli-must-not-be-lkg'".to_owned()),
            ])
            .unwrap();
            let mut lifecycle = TestNativeConfigLifecycle::new(
                ConfigDiscoveryInputs {
                    is_windows: false,
                    is_unix: false,
                    current_exe: None,
                    home_dir: None,
                    xdg_config_home: None,
                    xdg_config_dirs: Vec::new(),
                    environment_config_file: None,
                },
                false,
                Some(path.clone()),
                cli,
            );
            let attempt = lifecycle.attempt_reload();
            lifecycle.install_initial_attempt(attempt);
            assert_eq!(lifecycle.effective().generation, 0);
            assert_eq!(lifecycle.effective().config.term, None);
            assert_eq!(
                lifecycle.effective().config.automatically_reload_config,
                None,
                "CLI values may control the generation-zero watcher policy but are not LKG"
            );
            lifecycle
        };

        let mut disabled = lifecycle_with_cli(false);
        disabled
            .install_watcher_sink_for_test(Duration::from_millis(1), Arc::new(|| true), None)
            .unwrap();
        assert!(!disabled.watcher_exists_for_test());

        let mut enabled = lifecycle_with_cli(true);
        enabled
            .install_watcher_sink_for_test(Duration::from_millis(1), Arc::new(|| true), None)
            .unwrap();
        assert!(enabled.watcher_exists_for_test());
        assert!(enabled.watched_paths_for_test().contains(&path));
    }

    #[test]
    fn watch_paths_accumulate_across_rediscovery() {
        let root = unique_temp_dir("watch-rediscovery");
        let home = root.join("home");
        let xdg = root.join("xdg");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(xdg.join("wezterm")).unwrap();
        let home_file = home.join(".wezterm.lua");
        let xdg_file = xdg.join("wezterm/wezterm.lua");
        fs::write(&home_file, "return { automatically_reload_config = true }").unwrap();
        let mut lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: false,
                current_exe: None,
                home_dir: Some(home.clone()),
                xdg_config_home: Some(xdg.clone()),
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            None,
            TestValidatedNativeConfigAssignments::default(),
        );
        let attempt = lifecycle.attempt_reload();
        lifecycle.install_initial_attempt(attempt);
        lifecycle
            .install_watcher_sink_for_test(Duration::from_millis(1), Arc::new(|| true), None)
            .unwrap();

        fs::remove_file(&home_file).unwrap();
        fs::write(&xdg_file, "return { automatically_reload_config = true }").unwrap();
        let attempt = lifecycle.attempt_reload();
        assert!(lifecycle.install_runtime_attempt(attempt));

        let watched = lifecycle.watched_paths_for_test();
        assert!(watched.contains(&home_file));
        assert!(watched.contains(&xdg_file));
        assert!(watched.contains(&xdg.join("wezterm")));
    }

    #[test]
    fn watcher_remains_after_later_config_disables_auto_reload() {
        let root = unique_temp_dir("watch-retained-after-disable");
        let config_dir = root.join("config");
        fs::create_dir_all(&config_dir).unwrap();
        let path = config_dir.join("wezterm.lua");
        fs::write(
            &path,
            "return { automatically_reload_config = true, term = 'enabled' }",
        )
        .unwrap();
        let mut lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: false,
                current_exe: None,
                home_dir: Some(root.join("home")),
                xdg_config_home: None,
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            Some(path.clone()),
            TestValidatedNativeConfigAssignments::default(),
        );
        let attempt = lifecycle.attempt_reload();
        lifecycle.install_initial_attempt(attempt);
        lifecycle
            .install_watcher_sink_for_test(Duration::from_millis(1), Arc::new(|| true), None)
            .unwrap();
        let watched_before = lifecycle.watched_paths_for_test();

        fs::write(
            &path,
            "return { automatically_reload_config = false, term = 'disabled' }",
        )
        .unwrap();
        let attempt = lifecycle.attempt_reload();
        assert!(lifecycle.install_runtime_attempt(attempt));

        assert!(lifecycle.watcher_exists_for_test());
        assert_eq!(lifecycle.watched_paths_for_test(), watched_before);
    }

    #[test]
    fn dropping_watcher_stops_and_joins_worker() {
        let (stopped_sender, stopped_receiver) = mpsc::channel();
        let watcher = NativeConfigWatcher::new_for_test(
            Duration::from_millis(1),
            Arc::new(|| true),
            Some(stopped_sender),
        )
        .unwrap();

        drop(watcher);

        stopped_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("drop must receive the worker's explicit stopped acknowledgement");
    }

    #[test]
    fn watch_registration_error_is_diagnostic_and_preserves_lkg() {
        let root = unique_temp_dir("watch-registration-error");
        let missing = root.join("missing.lua");
        let mut lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: false,
                current_exe: None,
                home_dir: None,
                xdg_config_home: None,
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            Some(missing.clone()),
            TestValidatedNativeConfigAssignments::default(),
        );
        let attempt = lifecycle.attempt_reload();
        lifecycle.install_initial_attempt(attempt);
        assert_eq!(lifecycle.effective().generation, 0);

        lifecycle
            .install_watcher_sink_for_test(Duration::from_millis(1), Arc::new(|| true), None)
            .unwrap();

        assert!(
            lifecycle
                .watch_diagnostics_for_test()
                .iter()
                .any(|diagnostic| diagnostic.path.as_ref() == Some(&missing)),
            "failed file watch registration must remain observable"
        );
        assert_eq!(lifecycle.effective().generation, 0);
        assert!(lifecycle.latest_diagnostic().is_some());
    }

    #[test]
    fn strict_parser_accepts_empty_direct_table() {
        let overrides =
            parse_test_native_config_document("return {} -- before separator\n ; -- eof", &[])
                .unwrap();

        assert_eq!(overrides, crate::window::NativeConfigSnapshot::default());
    }

    #[test]
    fn strict_parser_accepts_documented_config_builder_direct_assignments() {
        let source = r"
            local wezterm = require 'wezterm'
            local config = wezterm.config_builder()
            config.term = 'xterm-256color'
            config.enable_tab_bar = false
            return config
        ";

        let overrides = parse_test_native_config_document(source, &[]).unwrap();

        assert_eq!(overrides.term.as_deref(), Some("xterm-256color"));
        assert_eq!(overrides.enable_tab_bar, Some(false));
    }

    #[test]
    fn strict_parser_accepts_builder_and_assignment_semicolons_with_crlf_comments() {
        let source = "\u{feff}-- header\r\n\
            local wezterm = require -- module\r\n\
            ('wezterm') ; -- require\r\n\
            local config = wezterm.config_builder() ; -- builder\r\n\
            config.term = 'xterm-256color'; -- assignment\r\n\
            config.enable_tab_bar = false ;\r\n\
            return config; -- eof";

        let overrides = parse_test_native_config_document(source, &[]).unwrap();

        assert_eq!(overrides.term.as_deref(), Some("xterm-256color"));
        assert_eq!(overrides.enable_tab_bar, Some(false));
    }

    #[test]
    fn strict_parser_rejects_undocumented_builder_aliases_and_extra_statements() {
        let sources = [
            "local config = require 'wezterm'.config_builder()\nreturn config",
            "local wt = require 'wezterm'\nlocal config = wt.config_builder()\nreturn config",
            "local wezterm = require 'wezterm'\nlocal cfg = wezterm.config_builder()\nreturn cfg",
            "local wezterm = require 'other'\nlocal config = wezterm.config_builder()\nreturn config",
            "local wezterm = require 'wezterm'\nlocal config = other.config_builder()\nreturn config",
            "local wezterm = require 'wezterm'\nlocal helper = true\nlocal config = wezterm.config_builder()\nreturn config",
            "local wezterm = require 'wezterm'\nlocal config = wezterm.config_builder()\nlocal helper = true\nreturn config",
        ];

        for source in sources {
            let error = Parser::new(source).parse_document().unwrap_err();
            assert!(matches!(
                error,
                NativeConfigLoadError::UnsupportedDynamicLua { .. }
                    | NativeConfigLoadError::InvalidSyntax { .. }
            ));
        }
    }

    #[test]
    fn strict_parser_consumes_nested_tables_arrays_strings_and_comments() {
        let source = "\u{feff}--[=[ header\r\ncomment ]=]\r\nreturn {\r\n\
            term = [=[xterm-256color]=], -- trailing\r\n\
            colors = { ansi = { 'a\\n', \"b\\x21\", 3, -4.5e1, true, nil, {}, }, },\r\n\
        } -- eof";

        let assignments = Parser::new(source).parse_document().unwrap();

        assert_eq!(assignments.len(), 2);
        assert_eq!(
            assignments[0].value,
            StaticLuaValue::String("xterm-256color".to_owned())
        );
        let StaticLuaValue::Table(colors) = &assignments[1].value else {
            panic!("expected nested colors table");
        };
        assert!(matches!(colors[0].1, StaticLuaValue::Array(_)));
    }

    #[test]
    fn strict_parser_rejects_trailing_top_level_statement() {
        let error = Parser::new("return {}\nconfig.term = 'late'")
            .parse_document()
            .unwrap_err();

        assert!(matches!(
            error,
            NativeConfigLoadError::UnsupportedDynamicLua {
                location: SourceLocation { line: 2, column: 1 },
                ..
            }
        ));
    }

    #[test]
    fn strict_parser_rejects_dynamic_return_root() {
        let error = Parser::new("return config").parse_document().unwrap_err();

        assert!(matches!(
            error,
            NativeConfigLoadError::UnsupportedDynamicLua {
                location: SourceLocation { line: 1, column: 8 },
                ref message,
            } if message.contains("dynamic return root")
        ));
    }

    #[test]
    fn strict_parser_rejects_variable_derived_value() {
        let source = "local wezterm = require('wezterm')\n\
                      local config = wezterm.config_builder()\n\
                      config.term = dynamic_term\n\
                      return config";
        let error = Parser::new(source).parse_document().unwrap_err();

        assert!(matches!(
            error,
            NativeConfigLoadError::UnsupportedDynamicLua { ref message, .. }
                if message.contains("variable-derived")
        ));
    }

    #[test]
    fn strict_parser_rejects_event_callback_and_table_insert() {
        let event = "local wezterm = require 'wezterm'\n\
                     wezterm.on('update-right-status', function() end)\n\
                     return {}";
        let inserted = "local wezterm = require 'wezterm'\n\
                        local config = wezterm.config_builder()\n\
                        config.keys = {}\n\
                        table.insert(config.keys, { key = 'x' })\n\
                        return config";

        for source in [event, inserted] {
            let error = Parser::new(source).parse_document().unwrap_err();
            assert!(matches!(
                error,
                NativeConfigLoadError::UnsupportedDynamicLua { ref message, .. }
                    if message.contains("top-level statements")
            ));
        }
    }

    #[test]
    fn strict_parser_rejects_malformed_balanced_value() {
        for source in [
            "return { colors = { ansi = { 'a' } }",
            "return { term = 'unterminated }",
            "return { term = [=[unterminated }",
            "--[=[ unterminated comment\nreturn {}",
        ] {
            let error = Parser::new(source).parse_document().unwrap_err();
            assert!(matches!(
                error,
                NativeConfigLoadError::InvalidSyntax { ref message, .. }
                    if message.contains("unterminated")
            ));
        }
    }

    #[test]
    fn strict_short_strings_preserve_known_escapes_and_reject_invalid_forms() {
        let assignments =
            Parser::new(r#"return { term = "\a\b\f\n\r\t\v\\\"\'\x41\065\u{42}\z   C" }"#)
                .parse_document()
                .unwrap();
        assert_eq!(
            assignments[0].value,
            StaticLuaValue::String("\x07\x08\x0c\n\r\t\x0b\\\"'AABC".to_owned())
        );

        for source in [
            r#"return { term = "bad\q" }"#,
            "return { term = \"bare\nnewline\" }",
            "return { term = \"bare\rcarriage\" }",
        ] {
            assert!(matches!(
                parse_test_native_config_document(source, &[]),
                Err(NativeConfigLoadError::InvalidSyntax { .. })
            ));
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
    )]
    #[test]
    fn strict_registry_accepts_lifecycle_consumer_fields() {
        let source = r##"
            return {
                term = "xterm-256color",
                default_cwd = "C:\\work\"quoted",
                initial_cols = 132,
                initial_rows = 43,
                automatically_reload_config = true,
                scrollback_lines = 9001,
                max_fps = 120,
                enable_tab_bar = false,
                window_close_confirmation = "NeverPrompt",
                color_scheme = "Builtin Solarized Dark",
                default_prog = { "pwsh", "-NoLogo" },
                default_gui_startup_args = { "ssh", "host" },
                colors = {
                    foreground = "#c0c0c0",
                    background = "#101010",
                    cursor_bg = "#ffffff",
                    cursor_fg = "#000000",
                    cursor_border = "#eeeeee",
                    compose_cursor = "#123456",
                    selection_bg = "#303030",
                    selection_fg = "none",
                    ansi = {
                        "#000000", "#800000", "#008000", "#808000",
                        "#000080", "#800080", "#008080", "#c0c0c0",
                    },
                    brights = {
                        "#808080", "#ff0000", "#00ff00", "#ffff00",
                        "#0000ff", "#ff00ff", "#00ffff", "#ffffff",
                    },
                    tab_bar = {
                        background = "#111111",
                        inactive_tab_edge = "#222222",
                        active_tab = {
                            fg_color = "#ffffff",
                            bg_color = "#333333",
                            intensity = "Bold",
                            underline = "Single",
                            italic = true,
                            strikethrough = false,
                        },
                        inactive_tab = { fg_color = "#aaaaaa", bg_color = "#222222" },
                        inactive_tab_hover = { fg_color = "#bbbbbb", bg_color = "#333333" },
                        new_tab = { fg_color = "#cccccc", bg_color = "#444444" },
                        new_tab_hover = { fg_color = "#dddddd", bg_color = "#555555" },
                    },
                },
                set_environment_variables = {
                    FOO = "bar",
                    ["WITH-DASH"] = "value",
                },
                keys = {
                    {
                        key = "x",
                        mods = "CTRL|SHIFT",
                        action = wezterm.action.SendString("safe\"\\\nvalue"),
                    },
                },
            }
        "##;

        let overrides = parse_test_native_config_document(source, &[]).unwrap();

        assert_eq!(overrides.term.as_deref(), Some("xterm-256color"));
        assert_eq!(overrides.default_cwd.as_deref(), Some("C:\\work\"quoted"));
        assert_eq!(overrides.initial_cols, Some(132));
        assert_eq!(overrides.initial_rows, Some(43));
        assert_eq!(overrides.automatically_reload_config, Some(true));
        assert_eq!(overrides.scrollback_lines, Some(9001));
        assert_eq!(overrides.max_fps, Some(120));
        assert_eq!(overrides.enable_tab_bar, Some(false));
        assert_eq!(
            overrides.color_scheme.as_deref(),
            Some("Builtin Solarized Dark")
        );
        assert_eq!(
            overrides.default_prog.as_deref(),
            Some(["pwsh".to_owned(), "-NoLogo".to_owned()].as_slice())
        );
        assert_eq!(
            overrides.default_gui_startup_args.as_deref(),
            Some(["ssh".to_owned(), "host".to_owned()].as_slice())
        );
        let colors = overrides.colors.as_ref().unwrap();
        assert_eq!(
            colors.foreground,
            Some(rterm_terminal::Color::Rgb(0xc0, 0xc0, 0xc0))
        );
        assert_eq!(
            colors.background,
            Some(rterm_terminal::Color::Rgb(0x10, 0x10, 0x10))
        );
        assert_eq!(
            colors.cursor_bg,
            Some(rterm_terminal::Color::Rgb(0xff, 0xff, 0xff))
        );
        assert_eq!(
            colors.cursor_fg,
            Some(rterm_terminal::Color::Rgb(0x00, 0x00, 0x00))
        );
        assert_eq!(
            colors.cursor_border,
            Some(rterm_terminal::Color::Rgb(0xee, 0xee, 0xee))
        );
        assert_eq!(
            colors.compose_cursor,
            Some(rterm_terminal::Color::Rgb(0x12, 0x34, 0x56))
        );
        assert_eq!(colors.selection_fg, Some(None));
        assert_eq!(
            colors.selection_bg,
            Some(rterm_terminal::Color::Rgb(0x30, 0x30, 0x30))
        );
        assert_eq!(
            colors.ansi,
            Some([
                rterm_terminal::Color::Rgb(0x00, 0x00, 0x00),
                rterm_terminal::Color::Rgb(0x80, 0x00, 0x00),
                rterm_terminal::Color::Rgb(0x00, 0x80, 0x00),
                rterm_terminal::Color::Rgb(0x80, 0x80, 0x00),
                rterm_terminal::Color::Rgb(0x00, 0x00, 0x80),
                rterm_terminal::Color::Rgb(0x80, 0x00, 0x80),
                rterm_terminal::Color::Rgb(0x00, 0x80, 0x80),
                rterm_terminal::Color::Rgb(0xc0, 0xc0, 0xc0),
            ])
        );
        assert_eq!(
            colors.brights,
            Some([
                rterm_terminal::Color::Rgb(0x80, 0x80, 0x80),
                rterm_terminal::Color::Rgb(0xff, 0x00, 0x00),
                rterm_terminal::Color::Rgb(0x00, 0xff, 0x00),
                rterm_terminal::Color::Rgb(0xff, 0xff, 0x00),
                rterm_terminal::Color::Rgb(0x00, 0x00, 0xff),
                rterm_terminal::Color::Rgb(0xff, 0x00, 0xff),
                rterm_terminal::Color::Rgb(0x00, 0xff, 0xff),
                rterm_terminal::Color::Rgb(0xff, 0xff, 0xff),
            ])
        );
        assert_eq!(
            colors.tab_bar_background,
            Some(rterm_terminal::Color::Rgb(0x11, 0x11, 0x11))
        );
        assert_eq!(
            colors.tab_bar_inactive_tab_edge,
            Some(rterm_terminal::Color::Rgb(0x22, 0x22, 0x22))
        );
        assert_eq!(
            colors.tab_bar_active_tab.test_projection(),
            (
                Some(rterm_terminal::Color::Rgb(0xff, 0xff, 0xff)),
                Some(rterm_terminal::Color::Rgb(0x33, 0x33, 0x33)),
                Some("Bold"),
                Some("Single"),
                Some(true),
                Some(false),
            )
        );
        assert_eq!(
            colors.tab_bar_inactive_tab.test_projection(),
            (
                Some(rterm_terminal::Color::Rgb(0xaa, 0xaa, 0xaa)),
                Some(rterm_terminal::Color::Rgb(0x22, 0x22, 0x22)),
                None,
                None,
                None,
                None,
            )
        );
        assert_eq!(
            colors.tab_bar_inactive_tab_hover.test_projection(),
            (
                Some(rterm_terminal::Color::Rgb(0xbb, 0xbb, 0xbb)),
                Some(rterm_terminal::Color::Rgb(0x33, 0x33, 0x33)),
                None,
                None,
                None,
                None,
            )
        );
        assert_eq!(
            colors.tab_bar_new_tab.test_projection(),
            (
                Some(rterm_terminal::Color::Rgb(0xcc, 0xcc, 0xcc)),
                Some(rterm_terminal::Color::Rgb(0x44, 0x44, 0x44)),
                None,
                None,
                None,
                None,
            )
        );
        assert_eq!(
            colors.tab_bar_new_tab_hover.test_projection(),
            (
                Some(rterm_terminal::Color::Rgb(0xdd, 0xdd, 0xdd)),
                Some(rterm_terminal::Color::Rgb(0x55, 0x55, 0x55)),
                None,
                None,
                None,
                None,
            )
        );
        assert_eq!(
            overrides
                .set_environment_variables
                .as_ref()
                .and_then(|environment| environment.get("WITH-DASH"))
                .map(String::as_str),
            Some("value")
        );
        assert_eq!(
            overrides.key_assignments.as_ref().map(std::vec::Vec::len),
            Some(1)
        );
        assert_eq!(
            overrides.key_assignments.as_ref().unwrap()[0].test_projection(),
            ("CTRL|SHIFT+x", Some("safe\"\\\nvalue"))
        );
    }

    #[test]
    fn strict_registry_rejects_unknown_top_level_field() {
        let error =
            parse_test_native_config_document("return {\n    definitely_unknown = true,\n}", &[])
                .unwrap_err();

        assert!(matches!(
            error,
            NativeConfigLoadError::UnknownField {
                location: SourceLocation { line: 2, column: 5 },
                ref field,
            } if field == "definitely_unknown"
        ));
    }

    #[test]
    fn strict_registry_rejects_mixed_known_and_unknown_colors_keys() {
        let error = parse_test_native_config_document(
            r##"return {
                colors = {
                    cursor_bg = "#ffffff",
                    compose_cursor = "#123456",
                    unexpected_cursor_key = "#000000",
                },
            }"##,
            &[],
        )
        .unwrap_err();

        assert!(matches!(
            error,
            NativeConfigLoadError::InvalidFieldValue {
                ref field,
                ref message,
                ..
            } if field == "colors" && message.contains("unexpected_cursor_key")
        ));
    }

    #[test]
    fn strict_colors_accepts_compose_cursor_and_converts_it() {
        let overrides = parse_test_native_config_document(
            r##"return { colors = { compose_cursor = "#123456" } }"##,
            &[],
        )
        .unwrap();

        assert_eq!(
            overrides
                .colors
                .as_ref()
                .and_then(|colors| colors.compose_cursor),
            Some(rterm_terminal::Color::Rgb(0x12, 0x34, 0x56))
        );
    }

    #[test]
    fn strict_registry_rejects_mixed_valid_and_unsupported_key_entries() {
        let error = parse_test_native_config_document(
            r#"return {
                keys = {
                    {
                        key = "x",
                        mods = "CTRL",
                        action = wezterm.action.SendString("ok"),
                    },
                    { key = "y", mods = "ALT", action = { DynamicAction = "bad" } },
                },
            }"#,
            &[],
        )
        .unwrap_err();

        assert!(matches!(
            error,
            NativeConfigLoadError::UnsupportedDynamicLua { .. }
        ));
    }

    #[test]
    fn strict_keys_reject_noncanonical_send_string_action_forms() {
        for action in [
            r#"{ SendString = "x" }"#,
            r#"wezterm.action { SendString = "x" }"#,
            r#"wezterm.action.SendString "x""#,
        ] {
            let source = format!("return {{ keys = {{ {{ key = \"x\", action = {action} }} }} }}");
            assert!(matches!(
                parse_test_native_config_document(&source, &[]),
                Err(NativeConfigLoadError::UnsupportedDynamicLua { .. })
            ));
        }
    }

    #[test]
    fn strict_keys_reject_mods_and_mod_alias_duplicates_before_legacy() {
        let error = parse_test_native_config_document(
            r#"return {
                keys = {
                    {
                        key = "x",
                        mods = "CTRL",
                        mod = "SHIFT",
                        action = wezterm.action.SendString("x"),
                    },
                },
            }"#,
            &[],
        )
        .unwrap_err();

        assert!(matches!(
            error,
            NativeConfigLoadError::InvalidFieldValue {
                ref field,
                ref message,
                ..
            } if field == "keys" && message.contains("duplicate modifier")
        ));
    }

    #[test]
    fn strict_canonical_quotes_reserved_and_injection_like_environment_keys() {
        let source = r#"return {
                set_environment_variables = {
                    ["return"] = "reserved-return",
                    ["end"] = "reserved-end",
                    ["function"] = "reserved-function",
                    ["x\"] = true; injected = [\""] = "key-roundtrip",
                    ["VALUE"] = "\"]}; os.execute('never'); --",
                },
            }"#;
        let assignments = Parser::new(source).parse_document().unwrap();
        let canonical = canonical_document(&assignments);
        assert!(canonical.contains(r#"["return"]="reserved-return""#));
        assert!(canonical.contains(r#"["end"]="reserved-end""#));
        assert!(canonical.contains(r#"["function"]="reserved-function""#));

        let overrides = parse_test_native_config_document(source, &[]).unwrap();

        let environment = overrides
            .set_environment_variables
            .as_ref()
            .expect("expected parsed environment overrides");
        assert_eq!(
            environment.get("return").map(String::as_str),
            Some("reserved-return")
        );
        assert_eq!(
            environment.get("end").map(String::as_str),
            Some("reserved-end")
        );
        assert_eq!(
            environment.get("function").map(String::as_str),
            Some("reserved-function")
        );
        assert_eq!(
            environment
                .get("x\"] = true; injected = [\"")
                .map(String::as_str),
            Some("key-roundtrip")
        );
        assert_eq!(
            environment.get("VALUE").map(String::as_str),
            Some("\"]}; os.execute('never'); --")
        );
    }

    #[test]
    fn strict_environment_action_key_identifier_form_roundtrips() {
        let source = r#"return {
                set_environment_variables = {
                    action = "literal",
                },
            }"#;

        let assignments = Parser::new(source).parse_document().unwrap();
        let canonical = canonical_document(&assignments);
        assert!(canonical.contains(r#"action="literal""#));

        let overrides = parse_test_native_config_document(source, &[]).unwrap();
        assert_eq!(
            overrides
                .set_environment_variables
                .as_ref()
                .and_then(|environment| environment.get("action"))
                .map(String::as_str),
            Some("literal")
        );
    }

    #[test]
    fn strict_environment_action_key_bracketed_form_roundtrips() {
        let source = r#"return {
                set_environment_variables = {
                    ["action"] = [=[long
literal]=],
                },
            }"#;

        let assignments = Parser::new(source).parse_document().unwrap();
        let canonical = canonical_document(&assignments);
        assert!(canonical.contains(r#"action="long\nliteral""#));

        let overrides = parse_test_native_config_document(source, &[]).unwrap();
        assert_eq!(
            overrides
                .set_environment_variables
                .as_ref()
                .and_then(|environment| environment.get("action"))
                .map(String::as_str),
            Some("long\nliteral")
        );
    }

    #[test]
    fn strict_registry_rejects_trailing_tokens_inside_composite_value() {
        let error = validate_test_cli_config_overrides(&[(
            "colors".to_owned(),
            "{ cursor_bg = '#ffffff' } trailing".to_owned(),
        )])
        .unwrap_err();

        assert!(matches!(
            error,
            NativeConfigLoadError::UnsupportedDynamicLua {
                location: SourceLocation { line: 1, .. },
                ..
            }
        ));
    }

    #[test]
    fn strict_cli_overrides_validate_and_last_duplicate_wins() {
        let items = vec![
            ("term".to_owned(), "'first'".to_owned()),
            ("enable_tab_bar".to_owned(), "false -- comment".to_owned()),
            ("term".to_owned(), r#""last\"safe""#.to_owned()),
        ];

        let cli = validate_test_cli_config_overrides(&items).unwrap();
        assert_eq!(cli.len(), 3);
        assert_eq!(cli[0].field_path, ["term"]);
        assert_eq!(cli[2].value_source, r#""last\"safe""#);
        assert_eq!(cli[2].location, SourceLocation { line: 1, column: 1 });
        assert_eq!(
            cli.default_overrides().term.as_deref(),
            Some("last\"safe"),
            "validated CLI must carry a precomputed panic-free defaults projection"
        );

        let overrides =
            parse_test_native_config_document("return { term = 'from-file' }", &cli).unwrap();
        assert_eq!(overrides.term.as_deref(), Some("last\"safe"));
        assert_eq!(overrides.enable_tab_bar, Some(false));

        assert!(matches!(
            validate_test_cli_config_overrides(&[("unknown".to_owned(), "true".to_owned())]),
            Err(NativeConfigLoadError::UnknownField { .. })
        ));
        assert!(matches!(
            validate_test_cli_config_overrides(&[("initial_cols".to_owned(), "-1".to_owned())]),
            Err(NativeConfigLoadError::InvalidFieldValue { .. })
        ));
    }

    #[test]
    fn strict_default_workspace_supports_file_and_cli_with_unknown_field_boundary() {
        parse_test_native_config_document("return { default_workspace = 'file-space' }", &[])
            .unwrap();

        let cli = validate_test_cli_config_overrides(&[(
            "default_workspace".to_owned(),
            "'cli-space'".to_owned(),
        )])
        .unwrap();
        parse_test_native_config_document("return { default_workspace = 'file-space' }", &cli)
            .unwrap();

        assert!(matches!(
            validate_test_cli_config_overrides(&[(
                "default_workspace_typo".to_owned(),
                "'nope'".to_owned()
            )]),
            Err(NativeConfigLoadError::UnknownField { .. })
        ));
        assert!(matches!(
            validate_test_cli_config_overrides(&[(
                "default_workspace".to_owned(),
                "''".to_owned()
            )]),
            Err(NativeConfigLoadError::InvalidFieldValue { .. })
        ));
        for invalid in ["true", "nil", "123", "foo.bar"] {
            assert!(
                validate_test_cli_config_overrides(&[(
                    "default_workspace".to_owned(),
                    invalid.to_owned()
                )])
                .is_err(),
                "{invalid:?} must remain a fatal strict-Lua type error"
            );
        }
    }

    #[test]
    fn explicit_file_beats_environment_and_candidates() {
        let root = unique_temp_dir("explicit-priority");
        let explicit = root.join("explicit.lua");
        let environment = root.join("environment.lua");
        let portable = root.join("wezterm.lua");
        let home = root.join("home");
        fs::create_dir_all(&home).unwrap();
        fs::write(&explicit, "return { term = 'explicit' }").unwrap();
        fs::write(&environment, "return { term = 'environment' }").unwrap();
        fs::write(&portable, "return { term = 'portable' }").unwrap();
        fs::write(home.join(".wezterm.lua"), "return { term = 'home' }").unwrap();

        let inputs = ConfigDiscoveryInputs {
            is_windows: true,
            is_unix: false,
            current_exe: Some(root.join("rterm.exe")),
            home_dir: Some(home),
            xdg_config_home: None,
            xdg_config_dirs: Vec::new(),
            environment_config_file: Some(environment),
        };
        let lifecycle = TestNativeConfigLifecycle::new(
            inputs,
            false,
            Some(explicit.clone()),
            TestValidatedNativeConfigAssignments::default(),
        );
        let attempt = lifecycle.attempt_reload();

        assert_eq!(
            attempt.resolved,
            ResolvedConfigSource::File(ConfigSource {
                path: explicit.clone(),
                required: true,
            })
        );
        assert_eq!(attempt.preferred, Some(explicit.clone()));
        assert_eq!(attempt.result.unwrap().term.as_deref(), Some("explicit"));
    }

    #[test]
    fn environment_file_beats_portable_home_and_xdg_candidates() {
        let root = unique_temp_dir("environment-priority");
        let environment = root.join("environment.lua");
        let home = root.join("home");
        let xdg_home = root.join("xdg");
        fs::create_dir_all(xdg_home.join("wezterm")).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::write(&environment, "return { term = 'environment' }").unwrap();
        fs::write(root.join("wezterm.lua"), "return { term = 'portable' }").unwrap();
        fs::write(home.join(".wezterm.lua"), "return { term = 'home' }").unwrap();
        fs::write(
            xdg_home.join("wezterm/wezterm.lua"),
            "return { term = 'xdg' }",
        )
        .unwrap();

        let lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: true,
                is_unix: false,
                current_exe: Some(root.join("rterm.exe")),
                home_dir: Some(home),
                xdg_config_home: Some(xdg_home),
                xdg_config_dirs: Vec::new(),
                environment_config_file: Some(environment.clone()),
            },
            false,
            None,
            TestValidatedNativeConfigAssignments::default(),
        );
        let attempt = lifecycle.attempt_reload();

        assert_eq!(
            attempt.resolved,
            ResolvedConfigSource::File(ConfigSource {
                path: environment.clone(),
                required: true,
            })
        );
        assert_eq!(attempt.preferred, Some(environment));
        assert_eq!(attempt.result.unwrap().term.as_deref(), Some("environment"));
    }

    #[test]
    fn windows_portable_config_beats_home_and_xdg() {
        let root = unique_temp_dir("portable-priority");
        let portable = root.join("wezterm.lua");
        let home = root.join("home");
        let xdg_home = root.join("xdg");
        fs::create_dir_all(xdg_home.join("wezterm")).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::write(&portable, "return { term = 'portable' }").unwrap();
        fs::write(home.join(".wezterm.lua"), "return { term = 'home' }").unwrap();
        fs::write(
            xdg_home.join("wezterm/wezterm.lua"),
            "return { term = 'xdg' }",
        )
        .unwrap();

        let lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: true,
                is_unix: false,
                current_exe: Some(root.join("rterm.exe")),
                home_dir: Some(home),
                xdg_config_home: Some(xdg_home),
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            None,
            TestValidatedNativeConfigAssignments::default(),
        );
        let attempt = lifecycle.attempt_reload();

        assert_eq!(
            attempt.resolved,
            ResolvedConfigSource::File(ConfigSource {
                path: portable,
                required: false,
            })
        );
        assert_eq!(attempt.result.unwrap().term.as_deref(), Some("portable"));
    }

    #[test]
    fn home_dot_wezterm_beats_xdg() {
        let root = unique_temp_dir("home-priority");
        let home = root.join("home");
        let xdg_home = root.join("xdg");
        fs::create_dir_all(xdg_home.join("wezterm")).unwrap();
        fs::create_dir_all(&home).unwrap();
        let home_config = home.join(".wezterm.lua");
        fs::write(&home_config, "return { term = 'home' }").unwrap();
        fs::write(
            xdg_home.join("wezterm/wezterm.lua"),
            "return { term = 'xdg' }",
        )
        .unwrap();

        let lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: true,
                current_exe: None,
                home_dir: Some(home),
                xdg_config_home: Some(xdg_home),
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            None,
            TestValidatedNativeConfigAssignments::default(),
        );
        let attempt = lifecycle.attempt_reload();

        assert_eq!(
            attempt.resolved,
            ResolvedConfigSource::File(ConfigSource {
                path: home_config,
                required: false,
            })
        );
        assert_eq!(attempt.result.unwrap().term.as_deref(), Some("home"));
    }

    #[test]
    fn xdg_config_home_then_unix_xdg_config_dirs_retain_source_order() {
        let root = unique_temp_dir("xdg-order");
        let xdg_home = root.join("xdg-home");
        let xdg_first = root.join("xdg-first");
        let xdg_second = root.join("xdg-second");
        for dir in [&xdg_home, &xdg_first, &xdg_second] {
            fs::create_dir_all(dir.join("wezterm")).unwrap();
        }
        let home_path = xdg_home.join("wezterm/wezterm.lua");
        let first_path = xdg_first.join("wezterm/wezterm.lua");
        let second_path = xdg_second.join("wezterm/wezterm.lua");
        fs::write(&home_path, "return { term = 'xdg-home' }").unwrap();
        fs::write(&first_path, "return { term = 'xdg-first' }").unwrap();
        fs::write(&second_path, "return { term = 'xdg-second' }").unwrap();

        let inputs = ConfigDiscoveryInputs {
            is_windows: false,
            is_unix: true,
            current_exe: None,
            home_dir: None,
            xdg_config_home: Some(xdg_home),
            xdg_config_dirs: vec![xdg_first, xdg_second],
            environment_config_file: None,
        };
        let lifecycle = TestNativeConfigLifecycle::new(
            inputs,
            false,
            None,
            TestValidatedNativeConfigAssignments::default(),
        );
        let first_attempt = lifecycle.attempt_reload();
        assert_eq!(
            first_attempt.result.unwrap().term.as_deref(),
            Some("xdg-home")
        );

        fs::remove_file(home_path).unwrap();
        let second_attempt = lifecycle.attempt_reload();
        assert_eq!(
            second_attempt.resolved,
            ResolvedConfigSource::File(ConfigSource {
                path: first_path,
                required: false,
            })
        );
        assert_eq!(
            second_attempt.result.unwrap().term.as_deref(),
            Some("xdg-first")
        );
    }

    #[test]
    fn unset_xdg_config_home_uses_home_dot_config() {
        let root = unique_temp_dir("xdg-home-fallback");
        let home = root.join("home");
        let fallback = home.join(".config/wezterm/wezterm.lua");
        fs::create_dir_all(fallback.parent().unwrap()).unwrap();
        fs::write(&fallback, "return { term = 'home-config' }").unwrap();

        let lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: true,
                current_exe: None,
                home_dir: Some(home),
                xdg_config_home: None,
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            None,
            TestValidatedNativeConfigAssignments::default(),
        );
        let attempt = lifecycle.attempt_reload();

        assert_eq!(
            attempt.resolved,
            ResolvedConfigSource::File(ConfigSource {
                path: fallback,
                required: false,
            })
        );
        assert_eq!(attempt.result.unwrap().term.as_deref(), Some("home-config"));
    }

    #[test]
    fn missing_required_path_is_failed_attempt_without_fallthrough() {
        let root = unique_temp_dir("required-missing");
        let missing = root.join("missing.lua");
        let home = root.join("home");
        fs::create_dir_all(&home).unwrap();
        fs::write(home.join(".wezterm.lua"), "return { term = 'home' }").unwrap();

        let lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: true,
                current_exe: None,
                home_dir: Some(home),
                xdg_config_home: None,
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            Some(missing.clone()),
            TestValidatedNativeConfigAssignments::default(),
        );
        let attempt = lifecycle.attempt_reload();

        assert_eq!(
            attempt.resolved,
            ResolvedConfigSource::File(ConfigSource {
                path: missing.clone(),
                required: true,
            })
        );
        let error = attempt.result.unwrap_err();
        assert_eq!(error.path, missing.clone());
        assert!(matches!(
            error.kind,
            NativeConfigSourceErrorKind::Io(std::io::ErrorKind::NotFound)
        ));
        assert!(
            error
                .to_string()
                .starts_with(&format!("{}: I/O error: not found: ", missing.display()))
        );
        assert!(!error.detail.is_empty());

        let environment_missing = root.join("environment-missing.lua");
        let environment_lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: true,
                current_exe: None,
                home_dir: lifecycle.inputs().home_dir.clone(),
                xdg_config_home: None,
                xdg_config_dirs: Vec::new(),
                environment_config_file: Some(environment_missing.clone()),
            },
            false,
            None,
            TestValidatedNativeConfigAssignments::default(),
        );
        let environment_attempt = environment_lifecycle.attempt_reload();
        assert_eq!(
            environment_attempt.preferred,
            Some(environment_missing.clone())
        );
        assert!(matches!(
            environment_attempt.result.unwrap_err(),
            NativeConfigSourceError {
                path,
                kind: NativeConfigSourceErrorKind::Io(std::io::ErrorKind::NotFound),
                ..
            } if path == environment_missing
        ));
    }

    #[test]
    fn missing_optional_path_falls_through() {
        let root = unique_temp_dir("optional-missing");
        let home = root.join("home");
        let xdg_home = root.join("xdg");
        let xdg_config = xdg_home.join("wezterm/wezterm.lua");
        fs::create_dir_all(xdg_config.parent().unwrap()).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::write(&xdg_config, "return { term = 'xdg' }").unwrap();

        let lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: true,
                is_unix: false,
                current_exe: Some(root.join("rterm.exe")),
                home_dir: Some(home),
                xdg_config_home: Some(xdg_home),
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            None,
            TestValidatedNativeConfigAssignments::default(),
        );
        let attempt = lifecycle.attempt_reload();

        assert_eq!(attempt.preferred, Some(xdg_config.clone()));
        assert_eq!(
            attempt.resolved,
            ResolvedConfigSource::File(ConfigSource {
                path: xdg_config,
                required: false,
            })
        );
        assert_eq!(attempt.result.unwrap().term.as_deref(), Some("xdg"));
    }

    #[test]
    fn optional_non_not_found_open_error_fails_without_fallthrough() {
        let root = unique_temp_dir("optional-open-error");
        let portable = root.join("wezterm.lua");
        let home = root.join("home");
        fs::create_dir_all(&portable).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::write(home.join(".wezterm.lua"), "return { term = 'home' }").unwrap();

        let lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: true,
                is_unix: false,
                current_exe: Some(root.join("rterm.exe")),
                home_dir: Some(home),
                xdg_config_home: None,
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            None,
            TestValidatedNativeConfigAssignments::default(),
        );
        let attempt = lifecycle.attempt_reload();

        assert_eq!(attempt.preferred, Some(portable.clone()));
        assert_eq!(
            attempt.resolved,
            ResolvedConfigSource::File(ConfigSource {
                path: portable.clone(),
                required: false,
            })
        );
        let error = attempt.result.unwrap_err();
        assert!(
            !error.detail.is_empty(),
            "the original OS error detail must be retained"
        );
        assert!(error.to_string().contains(&error.detail));
        assert!(matches!(
            error,
            NativeConfigSourceError {
                path,
                kind: NativeConfigSourceErrorKind::Io(kind),
                ..
            } if path == portable && kind != std::io::ErrorKind::NotFound
        ));
    }

    #[test]
    fn environment_capture_preserves_empty_values_as_set() {
        let captured = ConfigDiscoveryInputs::from_environment_snapshot(
            false,
            true,
            None,
            ConfigEnvironmentSnapshot {
                home: Some("home".into()),
                user_profile: Some("profile".into()),
                home_drive: None,
                home_path: None,
                xdg_config_home: Some("".into()),
                xdg_config_dirs: Some("".into()),
                wezterm_config_file: Some("".into()),
            },
        );

        assert_eq!(captured.home_dir, Some(PathBuf::from("home")));
        assert_eq!(captured.xdg_config_home, Some(PathBuf::new()));
        assert_eq!(captured.xdg_config_dirs, vec![PathBuf::new()]);
        assert_eq!(captured.environment_config_file, Some(PathBuf::new()));

        let unset = ConfigDiscoveryInputs::from_environment_snapshot(
            false,
            true,
            None,
            ConfigEnvironmentSnapshot::default(),
        );
        assert_eq!(unset.home_dir, None);
        assert_eq!(unset.xdg_config_home, None);
        assert!(unset.xdg_config_dirs.is_empty());
        assert_eq!(unset.environment_config_file, None);
    }

    #[test]
    fn environment_capture_uses_platform_home_precedence_and_ignores_empty_home() {
        let windows_profile_wins = ConfigDiscoveryInputs::from_environment_snapshot(
            true,
            false,
            None,
            ConfigEnvironmentSnapshot {
                home: Some("wrong-home".into()),
                user_profile: Some("windows-profile".into()),
                ..ConfigEnvironmentSnapshot::default()
            },
        );
        assert_eq!(
            windows_profile_wins.home_dir,
            Some(PathBuf::from("windows-profile"))
        );

        let windows_empty_home_uses_profile = ConfigDiscoveryInputs::from_environment_snapshot(
            true,
            false,
            None,
            ConfigEnvironmentSnapshot {
                home: Some("".into()),
                user_profile: Some("windows-profile".into()),
                ..ConfigEnvironmentSnapshot::default()
            },
        );
        assert_eq!(
            windows_empty_home_uses_profile.home_dir,
            Some(PathBuf::from("windows-profile"))
        );

        let unix_does_not_use_user_profile = ConfigDiscoveryInputs::from_environment_snapshot(
            false,
            true,
            None,
            ConfigEnvironmentSnapshot {
                home: Some("".into()),
                user_profile: Some("windows-profile".into()),
                ..ConfigEnvironmentSnapshot::default()
            },
        );
        assert_eq!(unix_does_not_use_user_profile.home_dir, None);
    }

    #[test]
    fn empty_environment_config_file_is_required_and_empty_xdg_home_suppresses_home_fallback() {
        let root = unique_temp_dir("empty-environment-values");
        let home = root.join("home");
        let home_fallback = home.join(".config/wezterm/wezterm.lua");
        fs::create_dir_all(home_fallback.parent().unwrap()).unwrap();
        fs::write(&home_fallback, "return { term = 'must-not-load' }").unwrap();

        let required_empty = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs::from_environment_snapshot(
                false,
                true,
                None,
                ConfigEnvironmentSnapshot {
                    home: Some(home.clone().into_os_string()),
                    wezterm_config_file: Some("".into()),
                    ..ConfigEnvironmentSnapshot::default()
                },
            ),
            false,
            None,
            TestValidatedNativeConfigAssignments::default(),
        )
        .attempt_reload();
        assert_eq!(
            required_empty.resolved,
            ResolvedConfigSource::File(ConfigSource {
                path: PathBuf::new(),
                required: true,
            })
        );
        assert!(matches!(
            required_empty.result.unwrap_err().kind,
            NativeConfigSourceErrorKind::Io(_)
        ));

        let empty_xdg_home = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs::from_environment_snapshot(
                false,
                true,
                None,
                ConfigEnvironmentSnapshot {
                    home: Some(home.clone().into_os_string()),
                    xdg_config_home: Some("".into()),
                    ..ConfigEnvironmentSnapshot::default()
                },
            ),
            false,
            None,
            TestValidatedNativeConfigAssignments::default(),
        );
        let candidates = empty_xdg_home.candidate_sources();
        assert_eq!(
            candidates
                .iter()
                .map(|source| source.path.clone())
                .collect::<Vec<_>>(),
            vec![
                home.join(".wezterm.lua"),
                PathBuf::from("wezterm/wezterm.lua"),
            ],
            "an explicitly empty XDG_CONFIG_HOME must use a relative XDG candidate and not HOME/.config"
        );
    }

    #[test]
    fn skip_disables_file_discovery_but_retains_and_applies_validated_cli_ir() {
        let root = unique_temp_dir("skip");
        let environment = root.join("environment.lua");
        fs::write(&environment, "return { term = 'environment' }").unwrap();
        let cli =
            validate_test_cli_config_overrides(&[("term".to_owned(), "'cli'".to_owned())]).unwrap();
        let lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: true,
                is_unix: false,
                current_exe: Some(root.join("rterm.exe")),
                home_dir: None,
                xdg_config_home: None,
                xdg_config_dirs: Vec::new(),
                environment_config_file: Some(environment),
            },
            true,
            None,
            cli.clone(),
        );
        let attempt = lifecycle.attempt_reload();

        assert_eq!(attempt.resolved, ResolvedConfigSource::Disabled);
        assert_eq!(lifecycle.validated_cli(), cli.as_slice());
        assert_eq!(attempt.result.unwrap().term.as_deref(), Some("cli"));
    }

    #[test]
    fn every_attempt_reload_reruns_discovery_and_reads_fresh_file_state() {
        let root = unique_temp_dir("rediscovery");
        let home = root.join("home");
        let xdg_home = root.join("xdg");
        let portable = root.join("wezterm.lua");
        let home_config = home.join(".wezterm.lua");
        let xdg_config = xdg_home.join("wezterm/wezterm.lua");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(xdg_config.parent().unwrap()).unwrap();
        fs::write(&home_config, "return { term = 'home-v1' }").unwrap();
        fs::write(&xdg_config, "return { term = 'xdg' }").unwrap();
        let lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: true,
                is_unix: false,
                current_exe: Some(root.join("rterm.exe")),
                home_dir: Some(home),
                xdg_config_home: Some(xdg_home),
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            None,
            TestValidatedNativeConfigAssignments::default(),
        );

        assert_eq!(
            lifecycle.attempt_reload().result.unwrap().term.as_deref(),
            Some("home-v1")
        );
        fs::write(&home_config, "return { term = 'home-v2' }").unwrap();
        assert_eq!(
            lifecycle.attempt_reload().result.unwrap().term.as_deref(),
            Some("home-v2")
        );
        fs::remove_file(home_config).unwrap();
        assert_eq!(
            lifecycle.attempt_reload().result.unwrap().term.as_deref(),
            Some("xdg")
        );
        fs::write(&portable, "return { term = 'portable' }").unwrap();
        let attempt = lifecycle.attempt_reload();
        assert_eq!(attempt.preferred, Some(portable));
        assert_eq!(attempt.result.unwrap().term.as_deref(), Some("portable"));
    }

    #[test]
    fn loader_removes_exactly_one_utf8_bom() {
        let root = unique_temp_dir("bom");
        let path = root.join("wezterm.lua");
        fs::write(&path, "\u{feff}return { term = 'single-bom' }".as_bytes()).unwrap();
        let lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: false,
                current_exe: None,
                home_dir: None,
                xdg_config_home: None,
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            Some(path.clone()),
            TestValidatedNativeConfigAssignments::default(),
        );
        assert_eq!(
            lifecycle.attempt_reload().result.unwrap().term.as_deref(),
            Some("single-bom")
        );

        fs::write(&path, "\u{feff}\u{feff}return {}".as_bytes()).unwrap();
        let error = lifecycle.attempt_reload().result.unwrap_err();
        assert_eq!(error.path, path);
        assert!(matches!(
            error.kind,
            NativeConfigSourceErrorKind::Strict(NativeConfigLoadError::InvalidSyntax { .. })
        ));
    }

    #[test]
    fn invalid_utf8_and_parser_errors_carry_preferred_path_diagnostic() {
        let root = unique_temp_dir("diagnostic-path");
        let path = root.join("wezterm.lua");
        fs::write(&path, [0xff, 0xfe]).unwrap();
        let lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: false,
                current_exe: None,
                home_dir: None,
                xdg_config_home: None,
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            Some(path.clone()),
            TestValidatedNativeConfigAssignments::default(),
        );

        let utf8_error = lifecycle.attempt_reload().result.unwrap_err();
        assert_eq!(utf8_error.path, path);
        assert!(matches!(
            utf8_error.kind,
            NativeConfigSourceErrorKind::InvalidUtf8
        ));
        assert!(utf8_error.to_string().ends_with(": invalid UTF-8"));

        fs::write(&path, "return { term = dynamic_value() }").unwrap();
        let parser_error = lifecycle.attempt_reload().result.unwrap_err();
        assert_eq!(parser_error.path, path.clone());
        assert!(matches!(
            parser_error.kind,
            NativeConfigSourceErrorKind::Strict(
                NativeConfigLoadError::UnsupportedDynamicLua { .. }
            )
        ));
        assert!(
            parser_error
                .to_string()
                .starts_with(&path.display().to_string())
        );
    }

    #[test]
    fn successful_initial_file_install_advances_generation_and_publishes_source() {
        let root = unique_temp_dir("initial-success");
        let path = root.join("wezterm.lua");
        fs::write(&path, "return { term = 'loaded' }").unwrap();
        let mut lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: false,
                current_exe: None,
                home_dir: None,
                xdg_config_home: None,
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            Some(path.clone()),
            TestValidatedNativeConfigAssignments::default(),
        );

        assert_eq!(lifecycle.effective().generation, 0);
        assert_eq!(lifecycle.effective().source, None);
        let attempt = lifecycle.attempt_reload();
        assert_eq!(
            attempt.publication.variables().get("WEZTERM_CONFIG_FILE"),
            Some(&path.to_str().unwrap().to_owned())
        );
        lifecycle.install_initial_attempt(attempt);

        assert_eq!(lifecycle.effective().generation, 1);
        assert_eq!(lifecycle.effective().source.as_ref(), Some(&path));
        assert_eq!(lifecycle.effective().config.term.as_deref(), Some("loaded"));
        assert_eq!(
            lifecycle
                .effective()
                .publication
                .variables()
                .get("WEZTERM_CONFIG_DIR"),
            Some(&root.to_str().unwrap().to_owned())
        );
        assert!(lifecycle.latest_diagnostic().is_none());
    }

    #[test]
    fn duplicate_valid_runtime_notification_keeps_generation_stable() {
        let root = unique_temp_dir("duplicate-valid-runtime-notification");
        let path = root.join("wezterm.lua");
        fs::write(&path, "return { term = 'stable' }").unwrap();
        let mut lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: false,
                current_exe: None,
                home_dir: None,
                xdg_config_home: None,
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            Some(path.clone()),
            TestValidatedNativeConfigAssignments::default(),
        );

        let attempt = lifecycle.attempt_reload();
        lifecycle.install_initial_attempt(attempt);
        assert_eq!(lifecycle.effective().generation, 1);

        let duplicate = lifecycle.attempt_reload();
        assert!(lifecycle.install_runtime_attempt(duplicate));
        assert_eq!(
            lifecycle.effective().generation,
            1,
            "a delayed duplicate notification for unchanged contents must not create a generation"
        );

        fs::write(&path, "return { term = }").unwrap();
        let invalid = lifecycle.attempt_reload();
        assert!(!lifecycle.install_runtime_attempt(invalid));
        assert!(lifecycle.latest_diagnostic().is_some());

        fs::write(&path, "return { term = 'stable' }").unwrap();
        let recovered = lifecycle.attempt_reload();
        assert!(lifecycle.install_runtime_attempt(recovered));
        assert_eq!(lifecycle.effective().generation, 2);
        assert!(lifecycle.latest_diagnostic().is_none());
    }

    // APFS cannot create the non-UTF-8 file required by this Unix filesystem test.
    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn non_utf8_source_path_is_diagnostic_and_never_installed() {
        use std::os::unix::ffi::OsStringExt;

        let root = unique_temp_dir("non-utf8-source");
        let path = root.join(OsString::from_vec(b"wezterm-\xff.lua".to_vec()));
        fs::write(&path, "return { term = 'loaded' }").unwrap();
        let mut lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: true,
                current_exe: None,
                home_dir: None,
                xdg_config_home: None,
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            Some(path.clone()),
            TestValidatedNativeConfigAssignments::default(),
        );

        let attempt = lifecycle.attempt_reload();
        assert!(matches!(
            &attempt.result,
            Err(NativeConfigSourceError {
                path: error_path,
                kind: NativeConfigSourceErrorKind::NonUnicodePath,
                ..
            }) if error_path == &path
        ));
        lifecycle.install_initial_attempt(attempt);
        assert_eq!(lifecycle.effective().generation, 0);
        assert!(lifecycle.effective().publication.variables().is_empty());
        assert_eq!(
            lifecycle.latest_diagnostic().map(|error| &error.path),
            Some(&path)
        );
    }

    #[cfg(unix)]
    #[test]
    fn missing_optional_non_utf8_path_still_falls_through() {
        use std::os::unix::ffi::OsStringExt;

        let root = unique_temp_dir("missing-non-utf8-source");
        let missing_home = root.join(OsString::from_vec(b"home-\xff".to_vec()));
        let xdg_home = root.join("xdg");
        let xdg_config = xdg_home.join("wezterm/wezterm.lua");
        fs::create_dir_all(xdg_config.parent().unwrap()).unwrap();
        fs::write(&xdg_config, "return { term = 'xdg' }").unwrap();
        let lifecycle = TestNativeConfigLifecycle::new(
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: true,
                current_exe: None,
                home_dir: Some(missing_home),
                xdg_config_home: Some(xdg_home),
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
            false,
            None,
            TestValidatedNativeConfigAssignments::default(),
        );

        let attempt = lifecycle.attempt_reload();
        assert_eq!(
            attempt.resolved,
            ResolvedConfigSource::File(ConfigSource {
                path: xdg_config,
                required: false,
            })
        );
        assert_eq!(attempt.result.unwrap().term.as_deref(), Some("xdg"));
    }
}
