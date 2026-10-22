use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;
use std::time::Duration;

use crate::{
    DiagnosticAttributionStage, DiagnosticFontMode, DiagnosticFontSpecimen, DiagnosticGpuBackend,
    DiagnosticRendererMode, MarkerKind, RunConfiguration, Scenario,
};

pub const LAUNCHER_USAGE: &str = "Usage: rterm-bench-launcher --app PATH --scenario empty-window|ssh1 [--renderer auto|cpu|gpu] [--product-gui] [--gpu-backend dx12|vulkan|gl] [--font-mode current|shared|lazy --font-specimen ascii|cjk|emoji] [--attribution-stage cpu-window|instance-surface|adapter-device|configured-surface-clear|layer-pipelines|fixture-font-text|platform-font-index|full-frame] [--stabilization-ms N] [--sample-interval-ms N] [--sample-count N] [--shutdown-timeout-ms N] [--cols N] [--rows N] [--json]";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherOptions {
    pub app: PathBuf,
    pub scenario: Scenario,
    pub stabilization: Duration,
    pub sample_interval: Duration,
    pub sample_count: u32,
    pub shutdown_timeout: Duration,
    pub columns: u16,
    pub rows: u16,
    pub renderer: DiagnosticRendererMode,
    pub gpu_backend: Option<DiagnosticGpuBackend>,
    pub font_mode: Option<DiagnosticFontMode>,
    pub font_specimen: Option<DiagnosticFontSpecimen>,
    pub attribution_stage: Option<DiagnosticAttributionStage>,
    pub product_gui: bool,
    pub json: bool,
}

impl LauncherOptions {
    /// Parses and validates the benchmark launcher command line.
    ///
    /// # Errors
    ///
    /// Returns an error for help, missing or repeated required arguments, unknown
    /// arguments, invalid scenario/value syntax, zero sampling values, or an app path
    /// that is not an existing file.
    #[expect(
        clippy::too_many_lines,
        reason = "the launcher parser keeps each private diagnostic option and validation branch explicit"
    )]
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, LauncherCliError> {
        let mut arguments = args.into_iter();
        let _program = arguments.next();
        let mut app = None;
        let mut scenario = None;
        let mut stabilization_ms = 5_000_u64;
        let mut sample_interval_ms = 100_u64;
        let mut sample_count = 10_u32;
        let mut shutdown_timeout_ms = 2_000_u64;
        let mut columns = 80_u16;
        let mut rows = 24_u16;
        let mut renderer = None;
        let mut gpu_backend = None;
        let mut font_mode = None;
        let mut font_specimen = None;
        let mut attribution_stage = None;
        let mut product_gui = false;
        let mut json = false;

        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--help" | "-h" => return Err(LauncherCliError::HelpRequested),
                "--app" => {
                    assign_once(
                        &mut app,
                        PathBuf::from(next_value(&mut arguments, "--app")?),
                        "--app",
                    )?;
                }
                "--scenario" => {
                    let value = next_value(&mut arguments, "--scenario")?;
                    let parsed = match value.as_str() {
                        "empty-window" => Scenario::EmptyWindow,
                        "ssh1" => Scenario::Ssh1,
                        _ => return Err(LauncherCliError::InvalidScenario(value)),
                    };
                    assign_once(&mut scenario, parsed, "--scenario")?;
                }
                "--stabilization-ms" => {
                    stabilization_ms = parse_positive(
                        &next_value(&mut arguments, "--stabilization-ms")?,
                        "--stabilization-ms",
                    )?;
                }
                "--sample-interval-ms" => {
                    sample_interval_ms = parse_positive(
                        &next_value(&mut arguments, "--sample-interval-ms")?,
                        "--sample-interval-ms",
                    )?;
                }
                "--sample-count" => {
                    sample_count = parse_positive(
                        &next_value(&mut arguments, "--sample-count")?,
                        "--sample-count",
                    )?;
                }
                "--shutdown-timeout-ms" => {
                    shutdown_timeout_ms = parse_positive(
                        &next_value(&mut arguments, "--shutdown-timeout-ms")?,
                        "--shutdown-timeout-ms",
                    )?;
                }
                "--cols" => {
                    columns = parse_positive(&next_value(&mut arguments, "--cols")?, "--cols")?;
                }
                "--rows" => {
                    rows = parse_positive(&next_value(&mut arguments, "--rows")?, "--rows")?;
                }
                "--renderer" => {
                    let value = next_value(&mut arguments, "--renderer")?;
                    assign_once(&mut renderer, parse_renderer(value)?, "--renderer")?;
                }
                "--gpu-backend" => {
                    let value = next_value(&mut arguments, "--gpu-backend")?;
                    assign_once(&mut gpu_backend, parse_gpu_backend(value)?, "--gpu-backend")?;
                }
                "--font-mode" => {
                    parse_font_mode_option(&mut arguments, &mut font_mode)?;
                }
                "--font-specimen" => {
                    parse_font_specimen_option(&mut arguments, &mut font_specimen)?;
                }
                "--attribution-stage" => {
                    let value = next_value(&mut arguments, "--attribution-stage")?;
                    let parsed = value
                        .parse()
                        .map_err(|_| LauncherCliError::InvalidAttributionStage(value))?;
                    assign_once(&mut attribution_stage, parsed, "--attribution-stage")?;
                }
                "--product-gui" => {
                    if product_gui {
                        return Err(LauncherCliError::RepeatedArgument("--product-gui"));
                    }
                    product_gui = true;
                }
                "--json" => json = true,
                _ => return Err(LauncherCliError::UnknownArgument(argument)),
            }
        }

        let app = validate_app(app)?;
        let scenario = scenario.ok_or(LauncherCliError::MissingArgument("--scenario"))?;
        let renderer = validate_options(scenario, renderer, gpu_backend, font_mode, font_specimen)?;
        if attribution_stage.is_some() && scenario != Scenario::EmptyWindow {
            return Err(LauncherCliError::AttributionRequiresEmptyWindow);
        }
        if attribution_stage.is_some() && font_mode.is_some() {
            return Err(LauncherCliError::AttributionWithFontProof);
        }
        if product_gui
            && (renderer != DiagnosticRendererMode::Auto
                || gpu_backend.is_some()
                || font_mode.is_some()
                || attribution_stage.is_some())
        {
            return Err(LauncherCliError::ProductGuiDiagnosticOverride);
        }
        Ok(Self {
            app,
            scenario,
            stabilization: Duration::from_millis(stabilization_ms),
            sample_interval: Duration::from_millis(sample_interval_ms),
            sample_count,
            shutdown_timeout: Duration::from_millis(shutdown_timeout_ms),
            columns,
            rows,
            renderer,
            gpu_backend,
            font_mode,
            font_specimen,
            attribution_stage,
            product_gui,
            json,
        })
    }

    #[must_use]
    pub fn configuration(&self) -> RunConfiguration {
        RunConfiguration {
            stabilization_ms: duration_millis(self.stabilization),
            sample_interval_ms: duration_millis(self.sample_interval),
            sample_count: self.sample_count,
            columns: self.columns,
            rows: self.rows,
            requested_renderer: self.renderer,
            requested_gpu_backend: self.gpu_backend,
            requested_font_mode: self.font_mode,
            requested_font_specimen: self.font_specimen,
            requested_attribution_stage: self.attribution_stage,
            ..RunConfiguration::default()
        }
    }
}

fn duration_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn parse_renderer(value: String) -> Result<DiagnosticRendererMode, LauncherCliError> {
    value
        .parse()
        .map_err(|_| LauncherCliError::InvalidRenderer(value))
}

fn parse_gpu_backend(value: String) -> Result<DiagnosticGpuBackend, LauncherCliError> {
    value
        .parse()
        .map_err(|_| LauncherCliError::InvalidGpuBackend(value))
}

fn parse_font_mode_option(
    arguments: &mut impl Iterator<Item = String>,
    font_mode: &mut Option<DiagnosticFontMode>,
) -> Result<(), LauncherCliError> {
    let value = next_value(arguments, "--font-mode")?;
    let parsed = value
        .parse()
        .map_err(|_| LauncherCliError::InvalidFontMode(value))?;
    assign_once(font_mode, parsed, "--font-mode")
}

fn parse_font_specimen_option(
    arguments: &mut impl Iterator<Item = String>,
    font_specimen: &mut Option<DiagnosticFontSpecimen>,
) -> Result<(), LauncherCliError> {
    let value = next_value(arguments, "--font-specimen")?;
    let parsed = value
        .parse()
        .map_err(|_| LauncherCliError::InvalidFontSpecimen(value))?;
    assign_once(font_specimen, parsed, "--font-specimen")
}

fn validate_app(app: Option<PathBuf>) -> Result<PathBuf, LauncherCliError> {
    let app = app.ok_or(LauncherCliError::MissingArgument("--app"))?;
    if !app.is_file() {
        return Err(LauncherCliError::AppDoesNotExist(app));
    }
    Ok(app)
}

fn validate_options(
    scenario: Scenario,
    renderer: Option<DiagnosticRendererMode>,
    gpu_backend: Option<DiagnosticGpuBackend>,
    font_mode: Option<DiagnosticFontMode>,
    font_specimen: Option<DiagnosticFontSpecimen>,
) -> Result<DiagnosticRendererMode, LauncherCliError> {
    let renderer = renderer.unwrap_or_default();
    if renderer == DiagnosticRendererMode::Cpu && gpu_backend.is_some() {
        return Err(LauncherCliError::CpuRendererWithGpuBackend);
    }
    if font_mode.is_some() != font_specimen.is_some() {
        return Err(LauncherCliError::IncompleteFontProofOptions);
    }
    if renderer == DiagnosticRendererMode::Cpu && font_mode.is_some() {
        return Err(LauncherCliError::CpuRendererWithFontProof);
    }
    if font_mode.is_some() && scenario != Scenario::EmptyWindow {
        return Err(LauncherCliError::FontProofRequiresEmptyWindow);
    }
    Ok(renderer)
}

fn next_value(
    arguments: &mut impl Iterator<Item = String>,
    option: &'static str,
) -> Result<String, LauncherCliError> {
    arguments
        .next()
        .ok_or(LauncherCliError::MissingValue(option))
}

fn assign_once<T>(
    destination: &mut Option<T>,
    value: T,
    option: &'static str,
) -> Result<(), LauncherCliError> {
    if destination.replace(value).is_some() {
        return Err(LauncherCliError::RepeatedArgument(option));
    }
    Ok(())
}

fn parse_positive<T>(value: &str, option: &'static str) -> Result<T, LauncherCliError>
where
    T: TryFrom<u64>,
{
    let parsed = value
        .parse::<u64>()
        .map_err(|_| LauncherCliError::InvalidPositiveValue {
            option,
            value: value.to_owned(),
        })?;
    if parsed == 0 {
        return Err(LauncherCliError::InvalidPositiveValue {
            option,
            value: value.to_owned(),
        });
    }
    T::try_from(parsed).map_err(|_| LauncherCliError::InvalidPositiveValue {
        option,
        value: value.to_owned(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherCliError {
    HelpRequested,
    MissingArgument(&'static str),
    MissingValue(&'static str),
    RepeatedArgument(&'static str),
    UnknownArgument(String),
    InvalidScenario(String),
    InvalidRenderer(String),
    InvalidGpuBackend(String),
    InvalidFontMode(String),
    InvalidFontSpecimen(String),
    InvalidAttributionStage(String),
    CpuRendererWithGpuBackend,
    IncompleteFontProofOptions,
    CpuRendererWithFontProof,
    FontProofRequiresEmptyWindow,
    AttributionRequiresEmptyWindow,
    AttributionWithFontProof,
    ProductGuiDiagnosticOverride,
    InvalidPositiveValue { option: &'static str, value: String },
    AppDoesNotExist(PathBuf),
}

impl Display for LauncherCliError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::HelpRequested => formatter.write_str(LAUNCHER_USAGE),
            Self::MissingArgument(option) => write!(formatter, "missing required {option}"),
            Self::MissingValue(option) => write!(formatter, "missing value for {option}"),
            Self::RepeatedArgument(option) => write!(formatter, "repeated argument {option}"),
            Self::UnknownArgument(argument) => write!(formatter, "unknown argument {argument}"),
            Self::InvalidScenario(value) => write!(
                formatter,
                "invalid scenario {value}; expected empty-window or ssh1"
            ),
            Self::InvalidRenderer(value) => {
                write!(
                    formatter,
                    "invalid value '{value}' for --renderer; expected auto, cpu, or gpu"
                )
            }
            Self::InvalidGpuBackend(value) => write!(
                formatter,
                "invalid value '{value}' for --gpu-backend; expected dx12, vulkan, or gl"
            ),
            Self::InvalidFontMode(value) => write!(
                formatter,
                "invalid value '{value}' for --font-mode; expected current, shared, or lazy"
            ),
            Self::InvalidFontSpecimen(value) => write!(
                formatter,
                "invalid value '{value}' for --font-specimen; expected ascii, cjk, or emoji"
            ),
            Self::InvalidAttributionStage(value) => write!(
                formatter,
                "invalid value '{value}' for --attribution-stage; expected cpu-window, instance-surface, adapter-device, configured-surface-clear, layer-pipelines, fixture-font-text, platform-font-index, or full-frame"
            ),
            Self::CpuRendererWithGpuBackend => {
                formatter.write_str("--gpu-backend cannot be used with --renderer cpu")
            }
            Self::IncompleteFontProofOptions => {
                formatter.write_str("--font-mode and --font-specimen must be provided together")
            }
            Self::CpuRendererWithFontProof => formatter
                .write_str("--font-mode and --font-specimen require --renderer auto or gpu"),
            Self::FontProofRequiresEmptyWindow => {
                formatter.write_str("font proof requires the empty-window scenario")
            }
            Self::AttributionRequiresEmptyWindow => {
                formatter.write_str("attribution stage requires the empty-window scenario")
            }
            Self::AttributionWithFontProof => formatter
                .write_str("--attribution-stage cannot be combined with font proof options"),
            Self::ProductGuiDiagnosticOverride => formatter.write_str(
                "--product-gui requires --renderer auto and forbids diagnostic backend, font, and attribution overrides",
            ),
            Self::InvalidPositiveValue { option, value } => {
                write!(
                    formatter,
                    "{option} must be a positive integer, observed {value}"
                )
            }
            Self::AppDoesNotExist(path) => {
                write!(formatter, "app path does not exist: {}", path.display())
            }
        }
    }
}

impl Error for LauncherCliError {}

#[cfg(test)]
mod font_mode_tests {
    use super::*;

    fn fixture_app() -> PathBuf {
        std::env::current_exe().expect("current test executable")
    }

    fn parse(extra: &[&str]) -> Result<LauncherOptions, LauncherCliError> {
        let mut args = vec![
            "rterm-bench-launcher".to_owned(),
            "--app".to_owned(),
            fixture_app().to_string_lossy().into_owned(),
            "--scenario".to_owned(),
            "empty-window".to_owned(),
        ];
        args.extend(extra.iter().map(|value| (*value).to_owned()));
        LauncherOptions::parse(args)
    }

    #[test]
    fn font_mode_launcher_requires_and_forwards_a_complete_pair() {
        let options = parse(&[
            "--renderer",
            "auto",
            "--font-mode",
            "shared",
            "--font-specimen",
            "cjk",
        ])
        .expect("private font proof options");
        assert_eq!(
            options.font_mode,
            Some(crate::DiagnosticFontMode::SharedAll)
        );
        assert_eq!(
            options.font_specimen,
            Some(crate::DiagnosticFontSpecimen::Cjk)
        );
        assert_eq!(
            options.configuration().requested_font_mode,
            Some(crate::DiagnosticFontMode::SharedAll)
        );

        assert!(matches!(
            parse(&["--font-mode", "lazy"]),
            Err(LauncherCliError::IncompleteFontProofOptions)
        ));
        assert!(matches!(
            parse(&["--font-specimen", "emoji"]),
            Err(LauncherCliError::IncompleteFontProofOptions)
        ));
    }

    #[test]
    fn font_mode_launcher_rejects_cpu_renderer() {
        assert!(matches!(
            parse(&[
                "--renderer",
                "cpu",
                "--font-mode",
                "current",
                "--font-specimen",
                "ascii",
            ]),
            Err(LauncherCliError::CpuRendererWithFontProof)
        ));
    }

    #[test]
    fn font_mode_launcher_rejects_ssh1_before_starting_the_app() {
        let app = fixture_app().to_string_lossy().into_owned();
        let error = LauncherOptions::parse([
            "rterm-bench-launcher".to_owned(),
            "--app".to_owned(),
            app,
            "--scenario".to_owned(),
            "ssh1".to_owned(),
            "--font-mode".to_owned(),
            "lazy".to_owned(),
            "--font-specimen".to_owned(),
            "ascii".to_owned(),
        ])
        .unwrap_err();
        assert!(matches!(
            error,
            LauncherCliError::FontProofRequiresEmptyWindow
        ));
    }

    #[test]
    fn font_proof_stabilization_starts_only_at_font_ownership_ready() {
        let configuration = RunConfiguration {
            stabilization_ms: 5_000,
            requested_font_mode: Some(crate::DiagnosticFontMode::Lazy),
            requested_font_specimen: Some(crate::DiagnosticFontSpecimen::Cjk),
            ..RunConfiguration::default()
        };
        let mut state = LauncherStateMachine::new(configuration);
        state.child_started(42).unwrap();
        state.observe_marker(MarkerKind::FirstPresent, 10).unwrap();
        state.observe_marker(MarkerKind::GpuReady, 100).unwrap();
        assert_eq!(state.phase(), LauncherPhase::AwaitScenarioReady);

        state
            .observe_marker(MarkerKind::FontOwnershipReady, 120)
            .unwrap();
        assert_eq!(state.phase(), LauncherPhase::Stabilize);
        assert_eq!(state.next_deadline_ms(), Some(5_120));
        state
            .observe_marker(MarkerKind::ScenarioReady, 121)
            .unwrap();
        assert_eq!(state.next_deadline_ms(), Some(5_120));
    }
}

#[cfg(test)]
mod product_gui_tests {
    use super::*;

    fn fixture_app() -> String {
        std::env::current_exe()
            .expect("current test executable")
            .to_string_lossy()
            .into_owned()
    }

    fn parse(extra: &[&str]) -> Result<LauncherOptions, LauncherCliError> {
        let mut args = vec![
            "rterm-bench-launcher".to_owned(),
            "--app".to_owned(),
            fixture_app(),
            "--scenario".to_owned(),
            "empty-window".to_owned(),
        ];
        args.extend(extra.iter().map(|value| (*value).to_owned()));
        LauncherOptions::parse(args)
    }

    #[test]
    fn parses_product_gui_mode() {
        let options =
            parse(&["--renderer", "auto", "--product-gui"]).expect("private product GUI mode");

        assert!(options.product_gui);
        assert_eq!(options.renderer, DiagnosticRendererMode::Auto);
        assert_eq!(options.configuration(), RunConfiguration::default());
    }

    #[test]
    fn product_gui_rejects_diagnostic_overrides() {
        for extra in [
            vec!["--product-gui", "--renderer", "gpu"],
            vec!["--product-gui", "--gpu-backend", "dx12"],
            vec![
                "--product-gui",
                "--font-mode",
                "lazy",
                "--font-specimen",
                "ascii",
            ],
            vec!["--product-gui", "--attribution-stage", "cpu-window"],
        ] {
            assert!(matches!(
                parse(&extra),
                Err(LauncherCliError::ProductGuiDiagnosticOverride)
            ));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherPhase {
    Launch,
    AwaitMarkers,
    AwaitScenarioReady,
    Stabilize,
    Sample,
    RequestShutdown,
    Reap,
    EmitResult,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherFailureCode {
    InvalidTransition,
    DecreasingClock,
    SampleBeforeDeadline,
    ChildExitedEarly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherFailure {
    pub code: LauncherFailureCode,
    pub phase: LauncherPhase,
    pub message: String,
}

impl Display for LauncherFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} ({:?})", self.message, self.phase)
    }
}

impl Error for LauncherFailure {}

#[derive(Debug, Clone)]
pub struct LauncherStateMachine {
    configuration: RunConfiguration,
    phase: LauncherPhase,
    pid: Option<u32>,
    last_elapsed_ms: u64,
    next_deadline_ms: Option<u64>,
    sample_bytes: Vec<u64>,
    forced_shutdown: bool,
    exit_code: Option<i32>,
}

impl LauncherStateMachine {
    #[must_use]
    pub const fn new(configuration: RunConfiguration) -> Self {
        Self {
            configuration,
            phase: LauncherPhase::Launch,
            pid: None,
            last_elapsed_ms: 0,
            next_deadline_ms: None,
            sample_bytes: Vec::new(),
            forced_shutdown: false,
            exit_code: None,
        }
    }

    #[must_use]
    pub const fn phase(&self) -> LauncherPhase {
        self.phase
    }

    #[must_use]
    pub const fn next_deadline_ms(&self) -> Option<u64> {
        self.next_deadline_ms
    }

    #[must_use]
    pub fn sample_bytes(&self) -> &[u64] {
        &self.sample_bytes
    }

    #[must_use]
    pub const fn forced_shutdown(&self) -> bool {
        self.forced_shutdown
    }

    #[must_use]
    pub const fn readiness_marker(&self) -> MarkerKind {
        if self.configuration.requested_attribution_stage.is_some() {
            MarkerKind::AttributionStageReady
        } else if self.configuration.requested_font_mode.is_some()
            && self.configuration.requested_font_specimen.is_some()
        {
            MarkerKind::FontOwnershipReady
        } else {
            MarkerKind::ScenarioReady
        }
    }

    /// Records successful child creation.
    ///
    /// # Errors
    ///
    /// Returns an invalid-transition failure unless called exactly once in `Launch`.
    pub fn child_started(&mut self, pid: u32) -> Result<(), LauncherFailure> {
        self.require_phase(LauncherPhase::Launch)?;
        self.pid = Some(pid);
        self.phase = LauncherPhase::AwaitMarkers;
        Ok(())
    }

    /// Applies a validated marker to launcher readiness state.
    ///
    /// # Errors
    ///
    /// Returns a failure if elapsed time moves backwards or the state is terminal.
    pub fn observe_marker(
        &mut self,
        kind: MarkerKind,
        elapsed_ms: u64,
    ) -> Result<(), LauncherFailure> {
        self.observe_time(elapsed_ms)?;
        if matches!(
            self.phase,
            LauncherPhase::Failed | LauncherPhase::EmitResult
        ) {
            return Err(self.invalid_transition("cannot observe markers in terminal state"));
        }
        match kind {
            MarkerKind::FirstPresent
                if matches!(
                    self.phase,
                    LauncherPhase::AwaitMarkers | LauncherPhase::AwaitScenarioReady
                ) =>
            {
                self.phase = LauncherPhase::AwaitScenarioReady;
            }
            kind if matches!(
                self.phase,
                LauncherPhase::AwaitMarkers | LauncherPhase::AwaitScenarioReady
            ) && kind == self.readiness_marker() =>
            {
                self.phase = LauncherPhase::Stabilize;
                self.next_deadline_ms =
                    Some(elapsed_ms.saturating_add(self.configuration.stabilization_ms));
            }
            _ => {}
        }
        Ok(())
    }

    /// Advances the launcher's monotonic clock and performs due transitions.
    ///
    /// # Errors
    ///
    /// Returns a failure when time moves backwards.
    pub fn advance_to(&mut self, elapsed_ms: u64) -> Result<(), LauncherFailure> {
        self.observe_time(elapsed_ms)?;
        if self.phase == LauncherPhase::Stabilize
            && self.next_deadline_ms.is_some_and(|due| elapsed_ms >= due)
        {
            self.phase = LauncherPhase::Sample;
        }
        Ok(())
    }

    /// Records one child-memory sample at or after its scheduled deadline.
    ///
    /// # Errors
    ///
    /// Returns a failure outside `Sample`, when time decreases, or when sampling is
    /// attempted before the next deadline.
    pub fn record_sample(&mut self, elapsed_ms: u64, bytes: u64) -> Result<(), LauncherFailure> {
        self.require_phase(LauncherPhase::Sample)?;
        self.observe_time(elapsed_ms)?;
        let due = self
            .next_deadline_ms
            .ok_or_else(|| self.invalid_transition("sample deadline is missing"))?;
        if elapsed_ms < due {
            return Err(LauncherFailure {
                code: LauncherFailureCode::SampleBeforeDeadline,
                phase: self.phase,
                message: format!("sample at {elapsed_ms} ms precedes deadline {due} ms"),
            });
        }
        self.sample_bytes.push(bytes);
        if self.sample_bytes.len()
            == usize::try_from(self.configuration.sample_count).unwrap_or(usize::MAX)
        {
            self.phase = LauncherPhase::RequestShutdown;
            self.next_deadline_ms = None;
        } else {
            self.next_deadline_ms =
                Some(elapsed_ms.saturating_add(self.configuration.sample_interval_ms));
        }
        Ok(())
    }

    /// Records child exit, failing if it occurred before shutdown was requested.
    ///
    /// # Errors
    ///
    /// Returns `ChildExitedEarly` for an exit before `RequestShutdown` or `Reap`.
    pub fn child_exited(
        &mut self,
        exit_code: Option<i32>,
        elapsed_ms: u64,
    ) -> Result<(), LauncherFailure> {
        self.observe_time(elapsed_ms)?;
        self.exit_code = exit_code;
        if matches!(
            self.phase,
            LauncherPhase::RequestShutdown | LauncherPhase::Reap
        ) {
            self.phase = LauncherPhase::EmitResult;
            return Ok(());
        }
        let failure = LauncherFailure {
            code: LauncherFailureCode::ChildExitedEarly,
            phase: self.phase,
            message: format!("child exited before sampling completed with {exit_code:?}"),
        };
        self.phase = LauncherPhase::Failed;
        Err(failure)
    }

    /// Moves from completed sampling to graceful shutdown/reaping.
    ///
    /// # Errors
    ///
    /// Returns an invalid-transition failure unless sampling requested shutdown.
    pub fn graceful_shutdown_requested(&mut self, elapsed_ms: u64) -> Result<(), LauncherFailure> {
        self.require_phase(LauncherPhase::RequestShutdown)?;
        self.observe_time(elapsed_ms)?;
        self.phase = LauncherPhase::Reap;
        Ok(())
    }

    /// Marks escalation from graceful shutdown to forced termination.
    ///
    /// # Errors
    ///
    /// Returns an invalid-transition failure outside `Reap`.
    pub fn force_shutdown(&mut self, elapsed_ms: u64) -> Result<(), LauncherFailure> {
        self.require_phase(LauncherPhase::Reap)?;
        self.observe_time(elapsed_ms)?;
        self.forced_shutdown = true;
        Ok(())
    }

    /// Records that the owned child was reaped.
    ///
    /// # Errors
    ///
    /// Returns an invalid-transition failure outside `Reap`.
    pub fn child_reaped(
        &mut self,
        exit_code: Option<i32>,
        elapsed_ms: u64,
    ) -> Result<(), LauncherFailure> {
        self.require_phase(LauncherPhase::Reap)?;
        self.observe_time(elapsed_ms)?;
        self.exit_code = exit_code;
        self.phase = LauncherPhase::EmitResult;
        Ok(())
    }

    fn observe_time(&mut self, elapsed_ms: u64) -> Result<(), LauncherFailure> {
        if elapsed_ms < self.last_elapsed_ms {
            return Err(LauncherFailure {
                code: LauncherFailureCode::DecreasingClock,
                phase: self.phase,
                message: format!(
                    "launcher clock decreased from {} ms to {elapsed_ms} ms",
                    self.last_elapsed_ms
                ),
            });
        }
        self.last_elapsed_ms = elapsed_ms;
        Ok(())
    }

    fn require_phase(&self, expected: LauncherPhase) -> Result<(), LauncherFailure> {
        if self.phase == expected {
            Ok(())
        } else {
            Err(self.invalid_transition(&format!(
                "expected phase {expected:?}, observed {:?}",
                self.phase
            )))
        }
    }

    fn invalid_transition(&self, message: &str) -> LauncherFailure {
        LauncherFailure {
            code: LauncherFailureCode::InvalidTransition,
            phase: self.phase,
            message: message.to_owned(),
        }
    }
}
