use std::{net::IpAddr, path::PathBuf};

use rssh_core::TerminalSize;
use rssh_pty::{PtyCommand, PtySize};
#[cfg(feature = "ssh")]
use rssh_ssh::{SshAuthMethod, SshConnectRequest, SshSessionConfig};

const DEFAULT_SSH_COLUMNS: u16 = 80;
const DEFAULT_SSH_ROWS: u16 = 24;
const DEFAULT_BENCH_BYTES: usize = 1_048_576;
const DEFAULT_BENCH_CHUNK_SIZE: usize = 8192;
const DEFAULT_BENCH_RENDER_FRAMES: usize = 30;
const DEFAULT_BENCH_IDLE_MS: usize = 200;
const DEFAULT_BENCH_COLUMNS: u16 = 120;
const DEFAULT_BENCH_ROWS: u16 = 30;
const DEFAULT_PROFILE_FILE: &str = "rssh-profiles.toml";

#[derive(Debug, PartialEq, Eq)]
pub enum AppCommand {
    Bench(BenchOptions),
    Doctor(DoctorOptions),
    DiagnosticGui(DiagnosticGuiOptions),
    Local(LocalOptions),
    Profile(ProfileOptions),
    ProfileCheck(ProfileCheckOptions),
    ProfileInit(ProfileInitOptions),
    ProfileList(ProfileListOptions),
    ProfileShow(ProfileShowOptions),
    Scp(ScpOptions),
    SelfTest(SelfTestOptions),
    Sftp(SftpOptions),
    Ssh(SshOptions),
    Version(VersionOptions),
    Window(WindowOptions),
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticGuiOptions {
    pub run_id: String,
    pub scenario: rssh_diagnostics::Scenario,
    pub hold_ms: u64,
    pub renderer: RendererMode,
    pub gpu_backend: Option<rssh_diagnostics::DiagnosticGpuBackend>,
    pub font_mode: Option<rssh_diagnostics::DiagnosticFontMode>,
    pub font_specimen: Option<rssh_diagnostics::DiagnosticFontSpecimen>,
    pub attribution_stage: Option<rssh_diagnostics::DiagnosticAttributionStage>,
    pub columns: u16,
    pub rows: u16,
    pub ssh_host: Option<IpAddr>,
    pub ssh_port: Option<u16>,
    pub ssh_user: Option<String>,
    pub log: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BenchOptions {
    pub json: bool,
    pub workload: BenchWorkload,
    pub bytes: usize,
    pub chunk_size: usize,
    pub render_frames: usize,
    pub idle_ms: usize,
    pub thresholds: BenchThresholds,
    pub size: TerminalSize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BenchWorkload {
    PlainScroll,
    AnsiScroll,
    #[default]
    AnsiScrollQuery,
}

impl BenchWorkload {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PlainScroll => "plain-scroll",
            Self::AnsiScroll => "ansi-scroll",
            Self::AnsiScrollQuery => "ansi-scroll-query",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "plain-scroll" => Ok(Self::PlainScroll),
            "ansi-scroll" => Ok(Self::AnsiScroll),
            "ansi-scroll-query" => Ok(Self::AnsiScrollQuery),
            value => Err(format!(
                "unknown bench workload: {value} (expected plain-scroll, ansi-scroll, or ansi-scroll-query)"
            )),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct BenchThresholds {
    pub min_throughput_bytes_per_sec: Option<usize>,
    pub max_chunk_p95_us: Option<usize>,
    pub max_render_frame_p95_us: Option<usize>,
    pub max_idle_cpu_percent: Option<u16>,
    pub max_process_memory_bytes: Option<usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DoctorOptions {
    pub json: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct VersionOptions {
    pub json: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SelfTestOptions {
    pub json: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct LocalOptions {
    pub command: PtyCommand,
    pub size: Option<PtySize>,
    pub mouse: bool,
    pub console: ConsoleOptions,
    pub osc52_policy: Osc52Policy,
    pub log: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConsoleOptions {
    pub preflight: bool,
    pub metrics: bool,
    pub metrics_json: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ProfileOptions {
    pub name: String,
    pub file: PathBuf,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ProfileCheckOptions {
    pub file: PathBuf,
    pub json: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ProfileInitOptions {
    pub file: PathBuf,
    pub force: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ProfileListOptions {
    pub file: PathBuf,
    pub verbose: bool,
    pub json: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ProfileShowOptions {
    pub name: String,
    pub file: PathBuf,
    pub json: bool,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, PartialEq, Eq)]
#[cfg(feature = "ssh")]
pub struct SshOptions {
    pub target: SshTarget,
    pub remote_command: Vec<String>,
    pub forwards: Vec<SshForward>,
    pub openssh_args: Vec<String>,
    pub no_shell: bool,
    pub native: bool,
    pub gui: bool,
    pub renderer: RendererMode,
    pub benchmark_startup: bool,
    pub native_host_key_policy: NativeHostKeyPolicy,
    pub console: ConsoleOptions,
    pub osc52_policy: Osc52Policy,
    pub log: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
#[cfg(feature = "ssh")]
pub struct SftpOptions {
    pub target: SshTarget,
    pub openssh_args: Vec<String>,
    pub console: ConsoleOptions,
    pub log: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
#[cfg(feature = "ssh")]
pub struct ScpOptions {
    pub target: SshTarget,
    pub transfer: ScpTransfer,
    pub recursive: bool,
    pub openssh_args: Vec<String>,
    pub console: ConsoleOptions,
    pub log: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ScpTransfer {
    Upload {
        local: PathBuf,
        remote: String,
    },
    UploadMany {
        locals: Vec<PathBuf>,
        remote: String,
    },
    Download {
        remote: String,
        local: PathBuf,
    },
    DownloadMany {
        remotes: Vec<String>,
        local: PathBuf,
    },
}

#[derive(Debug, PartialEq, Eq)]
#[cfg(feature = "ssh")]
pub enum SshTarget {
    Direct(SshConnectRequest),
    OpenSsh(OpenSshTarget),
}

#[derive(Debug, PartialEq, Eq)]
#[cfg(feature = "ssh")]
pub struct OpenSshTarget {
    pub target: String,
    pub username: Option<String>,
    pub port: Option<u16>,
    pub initial_size: TerminalSize,
    pub auth: SshAuthMethod,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshForward {
    Local(String),
    Remote(String),
    Dynamic(String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NativeHostKeyPolicy {
    #[default]
    RejectUnknown,
    TrustOnFirstUse,
    AcceptUnknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RendererMode {
    #[default]
    Auto,
    Cpu,
    Gpu,
}

impl RendererMode {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(Self::Auto),
            "cpu" => Ok(Self::Cpu),
            "gpu" => Ok(Self::Gpu),
            value => Err(format!(
                "invalid renderer: {value}; expected auto, cpu, or gpu"
            )),
        }
    }
}

#[allow(clippy::struct_excessive_bools)]
#[cfg(feature = "ssh")]
struct SshParseState {
    host: Option<String>,
    target: Option<String>,
    username: Option<String>,
    port: Option<u16>,
    columns: Option<u16>,
    rows: Option<u16>,
    auth: Option<SshAuthMethod>,
    remote_command: Vec<String>,
    forwards: Vec<SshForward>,
    openssh_args: Vec<String>,
    no_shell: bool,
    native: bool,
    gui: bool,
    renderer: RendererMode,
    renderer_selected: bool,
    benchmark_startup: bool,
    native_host_key_policy: NativeHostKeyPolicy,
    console: ConsoleOptions,
    osc52_policy: Osc52Policy,
    log: Option<PathBuf>,
}

#[cfg(feature = "ssh")]
impl Default for SshParseState {
    fn default() -> Self {
        Self {
            host: None,
            target: None,
            username: None,
            port: None,
            columns: None,
            rows: None,
            auth: None,
            remote_command: Vec::new(),
            forwards: Vec::new(),
            openssh_args: Vec::new(),
            no_shell: false,
            native: false,
            gui: false,
            renderer: RendererMode::Auto,
            renderer_selected: false,
            benchmark_startup: false,
            native_host_key_policy: NativeHostKeyPolicy::default(),
            console: ConsoleOptions::default(),
            osc52_policy: Osc52Policy::Off,
            log: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WindowConfigOptions {
    pub skip_config: bool,
    pub config_file: Option<PathBuf>,
    pub config_overrides: Vec<(String, String)>,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent compatibility flags represent valid combinations"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowOptions {
    pub config: WindowConfigOptions,
    pub frame_limit: Option<u64>,
    pub workspace: Option<String>,
    pub window_class: Option<String>,
    pub position: Option<WindowPosition>,
    pub osc52_policy: Osc52Policy,
    pub metrics: bool,
    pub metrics_json: bool,
    pub state: bool,
    pub state_json: bool,
    pub command: PtyCommand,
    pub log: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowPosition {
    pub origin: WindowPositionOrigin,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowPositionOrigin {
    Screen,
    Main,
    Active,
    Monitor(String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Osc52Policy {
    Off,
    #[default]
    WriteOnly,
    ReadWrite,
}

impl Osc52Policy {
    pub const fn allows_write(self) -> bool {
        matches!(self, Self::WriteOnly | Self::ReadWrite)
    }

    pub const fn allows_query(self) -> bool {
        matches!(self, Self::ReadWrite)
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
pub fn parse_args<I, S>(args: I) -> Result<AppCommand, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);
    let _program = args.next();
    let args = args.collect::<Vec<_>>();
    let (config, command_index) = parse_global_window_config_prefix(&args)?;
    let Some(command) = args.get(command_index) else {
        return Ok(AppCommand::Window(WindowOptions {
            config,
            frame_limit: None,
            workspace: None,
            window_class: None,
            position: None,
            osc52_policy: Osc52Policy::default(),
            metrics: false,
            metrics_json: false,
            state: false,
            state_json: false,
            command: PtyCommand::default_shell(),
            log: None,
        }));
    };
    let args = args[command_index + 1..].iter().cloned();

    if config != WindowConfigOptions::default()
        && matches!(
            command.as_str(),
            "bench"
                | "local"
                | "console"
                | "doctor"
                | "version"
                | "self-test"
                | "profile"
                | "scp"
                | "ssh"
                | "sftp"
        )
    {
        return Err(
            "global WezTerm config options cannot be used with non-GUI commands".to_owned(),
        );
    }

    match command.as_str() {
        "bench" => {
            let bench_args = args.collect::<Vec<_>>();
            if subcommand_help_requested(&bench_args) {
                return Ok(AppCommand::Help);
            }
            parse_bench(&bench_args)
        }
        "local" | "console" => {
            let local_args = args.collect::<Vec<_>>();
            if subcommand_help_requested(&local_args) {
                return Ok(AppCommand::Help);
            }
            parse_local(&local_args)
        }
        "doctor" => {
            let doctor_args = args.collect::<Vec<_>>();
            if subcommand_help_requested(&doctor_args) {
                return Ok(AppCommand::Help);
            }
            parse_doctor(&doctor_args)
        }
        "diagnostic-gui" => parse_diagnostic_gui(&args.collect::<Vec<_>>()),
        "version" => {
            let version_args = args.collect::<Vec<_>>();
            if subcommand_help_requested(&version_args) {
                return Ok(AppCommand::Help);
            }
            parse_version(&version_args)
        }
        "self-test" => {
            let self_test_args = args.collect::<Vec<_>>();
            if subcommand_help_requested(&self_test_args) {
                return Ok(AppCommand::Help);
            }
            parse_self_test(&self_test_args)
        }
        "profile" => {
            let profile_args = args.collect::<Vec<_>>();
            if subcommand_help_requested(&profile_args) {
                return Ok(AppCommand::Help);
            }
            parse_profile(&profile_args)
        }
        "scp" => {
            let scp_args = args.collect::<Vec<_>>();
            if subcommand_help_requested(&scp_args) {
                return Ok(AppCommand::Help);
            }
            parse_scp(&scp_args)
        }
        "ssh" => {
            let ssh_args = args.collect::<Vec<_>>();
            if subcommand_help_requested(&ssh_args) {
                return Ok(AppCommand::Help);
            }
            parse_ssh(&ssh_args)
        }
        "sftp" => {
            let sftp_args = args.collect::<Vec<_>>();
            if subcommand_help_requested(&sftp_args) {
                return Ok(AppCommand::Help);
            }
            parse_sftp(&sftp_args)
        }
        "window" | "start" => {
            let window_args = args.collect::<Vec<_>>();
            if subcommand_help_requested(&window_args) {
                return Ok(AppCommand::Help);
            }
            parse_window(&window_args, config)
        }
        "-h" | "--help" | "help" => Ok(AppCommand::Help),
        unknown => Err(format!("unknown command: {unknown}")),
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the private diagnostic parser keeps every option and cross-field validation explicit"
)]
fn parse_diagnostic_gui(args: &[String]) -> Result<AppCommand, String> {
    let mut run_id = None;
    let mut scenario = None;
    let mut hold_ms = None;
    let mut renderer = RendererMode::Auto;
    let mut gpu_backend = None;
    let mut font_mode = None;
    let mut font_specimen = None;
    let mut attribution_stage = None;
    let mut columns = DEFAULT_SSH_COLUMNS;
    let mut rows = DEFAULT_SSH_ROWS;
    let mut ssh_host = None;
    let mut ssh_port = None;
    let mut ssh_user = None;
    let mut log = None;
    let mut index = 0;

    while index < args.len() {
        let option = args[index].as_str();
        index += 1;
        let value = required_option_value(args.get(index), option)?;
        match option {
            "--run-id" => run_id = Some(value.to_owned()),
            "--scenario" => {
                scenario = Some(match value {
                    "empty-window" => rssh_diagnostics::Scenario::EmptyWindow,
                    "ssh1" => rssh_diagnostics::Scenario::Ssh1,
                    _ => {
                        return Err(format!(
                            "invalid diagnostic scenario: {value}; expected empty-window or ssh1"
                        ));
                    }
                });
            }
            "--hold-ms" => {
                hold_ms = Some(parse_positive_u64(value, "--hold-ms")?);
            }
            "--renderer" => renderer = RendererMode::parse(value)?,
            "--gpu-backend" => gpu_backend = Some(value.parse()?),
            "--font-mode" => font_mode = Some(value.parse()?),
            "--font-specimen" => font_specimen = Some(value.parse()?),
            "--attribution-stage" => attribution_stage = Some(value.parse()?),
            "--cols" => columns = parse_dimension(Some(&args[index]), "--cols")?,
            "--rows" => rows = parse_dimension(Some(&args[index]), "--rows")?,
            "--ssh-host" => {
                let host = value
                    .parse::<IpAddr>()
                    .map_err(|_| "--ssh-host must be an IP address".to_owned())?;
                if !host.is_loopback() {
                    return Err("--ssh-host must be a loopback address".to_owned());
                }
                ssh_host = Some(host);
            }
            "--ssh-port" => ssh_port = Some(parse_dimension(Some(&args[index]), "--ssh-port")?),
            "--ssh-user" => ssh_user = Some(value.to_owned()),
            "--log" => log = Some(PathBuf::from(value)),
            value => return Err(format!("unexpected diagnostic GUI option: {value}")),
        }
        index += 1;
    }

    let run_id = run_id.ok_or_else(|| "diagnostic-gui requires --run-id".to_owned())?;
    if run_id.trim().is_empty() {
        return Err("--run-id must not be empty".to_owned());
    }
    let scenario = scenario.ok_or_else(|| "diagnostic-gui requires --scenario".to_owned())?;
    if renderer == RendererMode::Cpu && gpu_backend.is_some() {
        return Err("--gpu-backend cannot be used with --renderer cpu".to_owned());
    }
    if font_mode.is_some() != font_specimen.is_some() {
        return Err("--font-mode and --font-specimen must be provided together".to_owned());
    }
    if renderer == RendererMode::Cpu && font_mode.is_some() {
        return Err("--font-mode and --font-specimen require --renderer auto or gpu".to_owned());
    }
    if font_mode.is_some() && scenario != rssh_diagnostics::Scenario::EmptyWindow {
        return Err("font proof requires the empty-window scenario".to_owned());
    }
    if attribution_stage.is_some() && scenario != rssh_diagnostics::Scenario::EmptyWindow {
        return Err("attribution stage requires the empty-window scenario".to_owned());
    }
    if attribution_stage.is_some() && font_mode.is_some() {
        return Err("--attribution-stage cannot be combined with font proof options".to_owned());
    }
    if scenario == rssh_diagnostics::Scenario::Ssh1 {
        if ssh_host.is_none() || ssh_port.is_none() || ssh_user.as_deref().is_none_or(str::is_empty)
        {
            return Err(
                "ssh1 diagnostic requires --ssh-host, --ssh-port, and --ssh-user".to_owned(),
            );
        }
    } else if ssh_host.is_some() || ssh_port.is_some() || ssh_user.is_some() || log.is_some() {
        return Err("SSH diagnostic options are only valid for the ssh1 scenario".to_owned());
    }
    Ok(AppCommand::DiagnosticGui(DiagnosticGuiOptions {
        run_id,
        scenario,
        hold_ms: hold_ms.ok_or_else(|| "diagnostic-gui requires --hold-ms".to_owned())?,
        renderer,
        gpu_backend,
        font_mode,
        font_specimen,
        attribution_stage,
        columns,
        rows,
        ssh_host,
        ssh_port,
        ssh_user,
        log,
    }))
}

fn parse_positive_u64(value: &str, option: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{option} must be a positive integer"))
}

fn parse_global_window_config_prefix(
    args: &[String],
) -> Result<(WindowConfigOptions, usize), String> {
    let mut config = WindowConfigOptions::default();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "-n" | "--skip-config" => config.skip_config = true,
            "--config-file" => {
                index += 1;
                config.config_file = Some(PathBuf::from(required_option_value(
                    args.get(index),
                    "--config-file",
                )?));
            }
            "--config" => {
                index += 1;
                let value = required_option_value(args.get(index), "--config")?;
                config.config_overrides.push(parse_config_override(value)?);
            }
            _ => break,
        }
        index += 1;
    }

    if config.skip_config && config.config_file.is_some() {
        return Err("--skip-config conflicts with --config-file".to_owned());
    }

    Ok((config, index))
}

fn parse_config_override(value: &str) -> Result<(String, String), String> {
    const INVALID_CONFIG_OVERRIDE: &str =
        "invalid value for --config: expected NAME=VALUE with non-empty NAME and VALUE";

    let Some((name, value)) = value.split_once('=') else {
        return Err(INVALID_CONFIG_OVERRIDE.to_owned());
    };
    let name = name.trim();
    if name.is_empty() || value.trim().is_empty() {
        return Err(INVALID_CONFIG_OVERRIDE.to_owned());
    }

    Ok((name.to_owned(), value.to_owned()))
}

fn all_command_help() -> &'static str {
    r"R-Term

Usage:
  rterm [window]
  rterm doctor [--json]
  rterm version [--json]
  rterm self-test [--json]
  rterm bench [--json] [--workload plain-scroll|ansi-scroll|ansi-scroll-query] [--bytes N] [--chunk-size N] [--render-frames N] [--idle-ms N] [--min-throughput-bytes-per-sec N] [--max-chunk-p95-us N] [--max-render-frame-p95-us N] [--max-idle-cpu-percent N] [--max-process-memory-bytes N] [--cols N --rows N]
  rterm window [--frames N] [--cwd CWD] [--workspace WORKSPACE] [--class CLASS] [--position POSITION] [--domain DOMAIN] [--attach] [--no-auto-connect] [--always-new-process] [--new-tab] [--osc52 off|write|read-write] [--metrics | --metrics-json | --state | --state-json] [--log PATH] [-e <program> [args...] | -- <program> [args...] | <program> [args...]]
  rterm start [--frames N] [--cwd CWD] [--workspace WORKSPACE] [--class CLASS] [--position POSITION] [--domain DOMAIN] [--attach] [--no-auto-connect] [--always-new-process] [--new-tab] [--osc52 off|write|read-write] [--metrics | --metrics-json | --state | --state-json] [--log PATH] [-e <program> [args...] | -- <program> [args...] | <program> [args...]]
  rterm local [--preflight] [--metrics | --metrics-json] [--cols N] [--rows N] [--cwd CWD] [--mouse] [--osc52 off|write|read-write] [--log PATH] [-- <program> [args...]]
  rterm console [--preflight] [--metrics | --metrics-json] [--cols N] [--rows N] [--cwd CWD] [--mouse] [--osc52 off|write|read-write] [--log PATH] [-- <program> [args...]]
  rterm ssh ([USER@]HOST | --host HOST --user USER | --target NAME) [--preflight] [--metrics | --metrics-json] [--native | --gui] [--renderer auto|cpu|gpu] [--benchmark-startup] [--accept-unknown-host-key | --trust-on-first-use] [-l USER | --user USER] [-p N | --port N] [-J DEST] [-F PATH] [-o OPTION] [-4 | -6] [-A | -a] [-C] [-q] [-v | -vv | -vvv] [-B IFACE] [-b ADDR] [-c CIPHER] [-E LOG] [-e CHAR] [-I PKCS11] [-m MAC] [-O CTL] [-P TAG] [-Q QUERY] [-S CTL_PATH] [-W HOST:PORT] [-w TUN] [-f] [-G] [-g] [-K | -k] [-M] [-n] [-s] [-T | -t | -tt] [-X | -x | -Y | -y] [--cols N --rows N] [--agent | --password | -i PATH | --key PATH] [-L SPEC | --local-forward SPEC] [-R SPEC | --remote-forward SPEC] [-D SPEC | --dynamic-forward SPEC] [-N | --no-shell] [--osc52 off|write|read-write] [--log PATH] [COMMAND [ARGS...]]
  rterm sftp ([USER@]HOST | --host HOST --user USER | --target NAME) [--preflight] [--metrics | --metrics-json] [-l LIMIT | --user USER] [-P N | --port N] [-J DEST] [-F PATH] [-o OPTION] [-4 | -6] [-A | -a] [-C] [-q] [-v | -vv | -vvv] [-b FILE] [-B N] [-R N] [-D COMMAND] [-S PROGRAM] [-s SUBSYSTEM] [-X OPTION] [-c CIPHER] [--cols N --rows N] [--agent | --password | -i PATH | --key PATH] [--log PATH]
  rterm scp [--preflight] [--metrics | --metrics-json] [-l LIMIT] [-P N | --port N] [-J DEST] [-F PATH] [-o OPTION] [-4 | -6] [-A | -a] [-C] [-q] [-v | -vv | -vvv] [-3] [-O] [-T] [-B] [-p] [-R] [-s] [-D PATH] [-S PROGRAM] [-X OPTION] [-c CIPHER] [-i PATH | --key PATH] [-r | --recursive] [--log PATH] LOCAL... [USER@]HOST:REMOTE
  rterm scp [--preflight] [--metrics | --metrics-json] [-l LIMIT] [-P N | --port N] [-J DEST] [-F PATH] [-o OPTION] [-4 | -6] [-A | -a] [-C] [-q] [-v | -vv | -vvv] [-3] [-O] [-T] [-B] [-p] [-R] [-s] [-D PATH] [-S PROGRAM] [-X OPTION] [-c CIPHER] [-i PATH | --key PATH] [-r | --recursive] [--log PATH] [USER@]HOST:REMOTE... LOCAL
  rterm scp ([USER@]HOST | --host HOST --user USER | --target NAME) [--preflight] [--metrics | --metrics-json] [-l LIMIT | --user USER] [-P N | --port N] [-J DEST] [-F PATH] [-o OPTION] [-4 | -6] [-A | -a] [-C] [-q] [-v | -vv | -vvv] [-3] [-O] [-T] [-B] [-p] [-R] [-s] [-D PATH] [-S PROGRAM] [-X OPTION] [-c CIPHER] [--cols N --rows N] [--agent | --password | -i PATH | --key PATH] [-r | --recursive] [--log PATH] (--upload LOCAL REMOTE | --download REMOTE LOCAL)
  rterm profile NAME [--file PATH]
  rterm profile --check [--json] [--file PATH]
  rterm profile --init [--file PATH] [--force]
  rterm profile --list [--verbose | --json] [--file PATH]
  rterm profile --show NAME [--json] [--file PATH]
  rterm --help
  rterm <command> --help

Global WezTerm configuration options (before window/start only):
  -n, --skip-config    Skip loading a configuration file
      --config-file PATH
                       Load PATH (conflicts with --skip-config)
      --config NAME=VALUE
                       Override a config value (repeatable; may be used with --skip-config)
"
}

fn subcommand_help_requested(args: &[String]) -> bool {
    args.iter()
        .take_while(|argument| argument.as_str() != "--")
        .any(|argument| matches!(argument.as_str(), "-h" | "--help"))
}

fn parse_doctor(args: &[String]) -> Result<AppCommand, String> {
    let mut json = false;

    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            value => return Err(format!("unexpected doctor option: {value}")),
        }
    }

    Ok(AppCommand::Doctor(DoctorOptions { json }))
}

fn parse_version(args: &[String]) -> Result<AppCommand, String> {
    let mut json = false;

    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            value => return Err(format!("unexpected version option: {value}")),
        }
    }

    Ok(AppCommand::Version(VersionOptions { json }))
}

fn parse_self_test(args: &[String]) -> Result<AppCommand, String> {
    let mut json = false;

    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            value => return Err(format!("unexpected self-test option: {value}")),
        }
    }

    Ok(AppCommand::SelfTest(SelfTestOptions { json }))
}

fn parse_bench(args: &[String]) -> Result<AppCommand, String> {
    let mut json = false;
    let mut workload = BenchWorkload::default();
    let mut bytes = DEFAULT_BENCH_BYTES;
    let mut chunk_size = DEFAULT_BENCH_CHUNK_SIZE;
    let mut render_frames = DEFAULT_BENCH_RENDER_FRAMES;
    let mut idle_ms = DEFAULT_BENCH_IDLE_MS;
    let mut thresholds = BenchThresholds::default();
    let mut columns = DEFAULT_BENCH_COLUMNS;
    let mut rows = DEFAULT_BENCH_ROWS;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--json" => json = true,
            "--workload" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "missing value for --workload".to_owned())?;
                workload = BenchWorkload::parse(value)?;
            }
            "--bytes" => {
                index += 1;
                bytes = parse_nonzero_usize(args.get(index), "--bytes")?;
            }
            "--chunk-size" => {
                index += 1;
                chunk_size = parse_nonzero_usize(args.get(index), "--chunk-size")?;
            }
            "--render-frames" => {
                index += 1;
                render_frames = parse_nonzero_usize(args.get(index), "--render-frames")?;
            }
            "--idle-ms" => {
                index += 1;
                idle_ms = parse_nonzero_usize(args.get(index), "--idle-ms")?;
            }
            "--min-throughput-bytes-per-sec" => {
                index += 1;
                thresholds.min_throughput_bytes_per_sec = Some(parse_nonzero_usize(
                    args.get(index),
                    "--min-throughput-bytes-per-sec",
                )?);
            }
            "--max-chunk-p95-us" => {
                index += 1;
                thresholds.max_chunk_p95_us =
                    Some(parse_nonzero_usize(args.get(index), "--max-chunk-p95-us")?);
            }
            "--max-render-frame-p95-us" => {
                index += 1;
                thresholds.max_render_frame_p95_us = Some(parse_nonzero_usize(
                    args.get(index),
                    "--max-render-frame-p95-us",
                )?);
            }
            "--max-idle-cpu-percent" => {
                index += 1;
                thresholds.max_idle_cpu_percent = Some(parse_nonzero_dimension(
                    args.get(index),
                    "--max-idle-cpu-percent",
                )?);
            }
            "--max-process-memory-bytes" => {
                index += 1;
                thresholds.max_process_memory_bytes = Some(parse_nonzero_usize(
                    args.get(index),
                    "--max-process-memory-bytes",
                )?);
            }
            "--cols" => {
                index += 1;
                columns = parse_nonzero_dimension(args.get(index), "--cols")?;
            }
            "--rows" => {
                index += 1;
                rows = parse_nonzero_dimension(args.get(index), "--rows")?;
            }
            value => return Err(format!("unexpected bench option: {value}")),
        }
        index += 1;
    }

    Ok(AppCommand::Bench(BenchOptions {
        json,
        workload,
        bytes,
        chunk_size,
        render_frames,
        idle_ms,
        thresholds,
        size: TerminalSize::new(columns, rows),
    }))
}

fn parse_local(args: &[String]) -> Result<AppCommand, String> {
    let mut columns = None;
    let mut rows = None;
    let mut mouse = false;
    let mut preflight = false;
    let mut metrics = false;
    let mut metrics_json = false;
    let mut osc52_policy = Osc52Policy::default();
    let mut log = None;
    let mut cwd = None;
    let mut command_args = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--cols" => {
                index += 1;
                columns = Some(parse_dimension(args.get(index), "--cols")?);
            }
            "--rows" => {
                index += 1;
                rows = Some(parse_dimension(args.get(index), "--rows")?);
            }
            "--mouse" => {
                mouse = true;
            }
            "--preflight" => {
                preflight = true;
            }
            "--metrics" => {
                set_console_metrics(&mut metrics, &mut metrics_json, "--metrics")?;
            }
            "--metrics-json" => {
                set_console_metrics(&mut metrics, &mut metrics_json, "--metrics-json")?;
            }
            "--osc52" => {
                index += 1;
                osc52_policy = parse_osc52_policy(args.get(index))?;
            }
            "--log" => {
                index += 1;
                log = Some(PathBuf::from(required_option_value(
                    args.get(index),
                    "--log",
                )?));
            }
            "--cwd" => {
                index += 1;
                cwd = Some(PathBuf::from(required_option_value(
                    args.get(index),
                    "--cwd",
                )?));
            }
            "--" => {
                command_args.extend(args[index + 1..].iter().cloned());
                break;
            }
            value => return Err(format!("unexpected local option: {value}")),
        }
        index += 1;
    }

    let command = if command_args.is_empty() {
        PtyCommand::default_shell()
    } else {
        let mut iter = command_args.into_iter();
        let program = iter.next().expect("command_args is not empty");
        PtyCommand::new(program).with_args(iter)
    };
    let command = match cwd {
        Some(cwd) => command.with_cwd(cwd),
        None => command,
    };

    let size = match (columns, rows) {
        (None, None) => None,
        (Some(columns), Some(rows)) => {
            Some(PtySize::try_new(columns, rows).map_err(|error| error.to_string())?)
        }
        (None, Some(_)) => return Err("--rows requires --cols".to_owned()),
        (Some(_), None) => return Err("--cols requires --rows".to_owned()),
    };

    Ok(AppCommand::Local(LocalOptions {
        command,
        size,
        mouse,
        console: ConsoleOptions {
            preflight,
            metrics,
            metrics_json,
        },
        osc52_policy,
        log,
    }))
}

fn parse_profile(args: &[String]) -> Result<AppCommand, String> {
    let mut name = None;
    let mut file = PathBuf::from(DEFAULT_PROFILE_FILE);
    let mut check = false;
    let mut init = false;
    let mut force = false;
    let mut json = false;
    let mut list = false;
    let mut show = false;
    let mut verbose = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--check" => {
                check = true;
            }
            "--force" => {
                force = true;
            }
            "--init" => {
                init = true;
            }
            "--json" => {
                json = true;
            }
            "--list" => {
                list = true;
            }
            "--show" => {
                show = true;
            }
            "--verbose" => {
                verbose = true;
            }
            "--file" => {
                index += 1;
                file = PathBuf::from(required_option_value(args.get(index), "--file")?);
            }
            value if !value.starts_with('-') && name.is_none() => {
                name = Some(value.to_owned());
            }
            value if !value.starts_with('-') => {
                return Err(format!("unexpected profile argument: {value}"));
            }
            value => return Err(format!("unexpected profile option: {value}")),
        }
        index += 1;
    }

    let selected_modes =
        usize::from(check) + usize::from(init) + usize::from(list) + usize::from(show);
    if selected_modes > 1 {
        return Err("profile mode flags cannot be combined".to_owned());
    }

    if force && !init {
        return Err("profile --force requires --init".to_owned());
    }

    if verbose && !list {
        return Err("profile --verbose requires --list".to_owned());
    }

    if json && !(check || list || show) {
        return Err("profile --json requires --check, --list, or --show".to_owned());
    }

    if json && verbose {
        return Err("profile --json cannot be combined with --verbose".to_owned());
    }

    if check {
        if name.is_some() {
            return Err("profile --check cannot be combined with a profile name".to_owned());
        }
        return Ok(AppCommand::ProfileCheck(ProfileCheckOptions { file, json }));
    }

    if init {
        if name.is_some() {
            return Err("profile --init cannot be combined with a profile name".to_owned());
        }
        return Ok(AppCommand::ProfileInit(ProfileInitOptions { file, force }));
    }

    if list {
        if name.is_some() {
            return Err("profile --list cannot be combined with a profile name".to_owned());
        }
        return Ok(AppCommand::ProfileList(ProfileListOptions {
            file,
            verbose,
            json,
        }));
    }

    if show {
        let Some(name) = name else {
            return Err("profile --show requires a profile name".to_owned());
        };
        return Ok(AppCommand::ProfileShow(ProfileShowOptions {
            name,
            file,
            json,
        }));
    }

    let Some(name) = name else {
        return Err("profile name is required".to_owned());
    };

    Ok(AppCommand::Profile(ProfileOptions { name, file }))
}

#[cfg(feature = "ssh")]
fn parse_ssh(args: &[String]) -> Result<AppCommand, String> {
    let mut state = SshParseState::default();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--" => {
                state
                    .remote_command
                    .extend(args[index + 1..].iter().cloned());
                break;
            }
            value if !value.starts_with('-') && ssh_target_selected(&state) => {
                state.remote_command.extend(args[index..].iter().cloned());
                break;
            }
            _ => parse_ssh_option(args, &mut index, &mut state)?,
        }
        index += 1;
    }

    Ok(AppCommand::Ssh(ssh_options_from_state(state)?))
}

#[cfg(feature = "ssh")]
fn parse_sftp(args: &[String]) -> Result<AppCommand, String> {
    let mut state = SshParseState::default();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--" => return Err("sftp does not accept a remote command".to_owned()),
            _ => parse_sftp_option(args, &mut index, &mut state)?,
        }
        index += 1;
    }

    let options = ssh_options_from_state(state)?;
    Ok(AppCommand::Sftp(SftpOptions {
        target: options.target,
        openssh_args: options.openssh_args,
        console: options.console,
        log: options.log,
    }))
}

#[cfg(feature = "ssh")]
fn parse_scp(args: &[String]) -> Result<AppCommand, String> {
    let mut state = SshParseState::default();
    let mut recursive = false;
    let mut transfer = None;
    let mut positionals = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--" => return Err("scp does not accept a remote command separator".to_owned()),
            "-r" | "--recursive" => recursive = true,
            "--upload" => {
                index += 1;
                let local = PathBuf::from(required_option_value(args.get(index), "--upload")?);
                index += 1;
                let remote = required_option_value(args.get(index), "--upload")?.to_owned();
                set_scp_transfer(&mut transfer, ScpTransfer::Upload { local, remote })?;
            }
            "--download" => {
                index += 1;
                let remote = required_option_value(args.get(index), "--download")?.to_owned();
                index += 1;
                let local = PathBuf::from(required_option_value(args.get(index), "--download")?);
                set_scp_transfer(&mut transfer, ScpTransfer::Download { remote, local })?;
            }
            value if is_scp_value_passthrough(value) => {
                parse_ssh_passthrough_option(args, &mut index, &mut state)?;
            }
            value if is_scp_flag_passthrough(value) => {
                state.openssh_args.push(value.to_owned());
            }
            value if !value.starts_with('-') => positionals.push(value.to_owned()),
            _ => parse_sftp_option(args, &mut index, &mut state)?,
        }
        index += 1;
    }

    apply_scp_positionals(&mut state, &mut transfer, &positionals)?;
    let transfer = transfer.ok_or_else(|| "scp requires --upload or --download".to_owned())?;
    let options = ssh_options_from_state(state)?;
    Ok(AppCommand::Scp(ScpOptions {
        target: options.target,
        transfer,
        recursive,
        openssh_args: options.openssh_args,
        console: options.console,
        log: options.log,
    }))
}

#[cfg(feature = "ssh")]
fn apply_scp_positionals(
    state: &mut SshParseState,
    transfer: &mut Option<ScpTransfer>,
    positionals: &[String],
) -> Result<(), String> {
    if transfer.is_some() {
        match positionals {
            [] => Ok(()),
            [target] => set_positional_ssh_target(state, target, "scp"),
            _ => {
                Err("scp accepts only one positional target with --upload or --download".to_owned())
            }
        }
    } else {
        let Some((destination, sources)) = positionals.split_last() else {
            return Ok(());
        };
        if sources.is_empty() {
            return Ok(());
        }
        infer_scp_transfer_from_operands(state, transfer, sources, destination)
    }
}

#[cfg(feature = "ssh")]
fn infer_scp_transfer_from_operands(
    state: &mut SshParseState,
    transfer: &mut Option<ScpTransfer>,
    sources: &[String],
    destination: &str,
) -> Result<(), String> {
    let destination_remote = split_scp_remote_operand(destination);
    if let Some((target, remote)) = destination_remote
        && sources
            .iter()
            .all(|source| split_scp_remote_operand(source).is_none())
    {
        set_positional_ssh_target(state, target, "scp")?;
        let locals = sources.iter().map(PathBuf::from).collect::<Vec<_>>();
        return if let [local] = locals.as_slice() {
            set_scp_transfer(
                transfer,
                ScpTransfer::Upload {
                    local: local.clone(),
                    remote: remote.to_owned(),
                },
            )
        } else {
            set_scp_transfer(
                transfer,
                ScpTransfer::UploadMany {
                    locals,
                    remote: remote.to_owned(),
                },
            )
        };
    }

    let remote_sources = sources
        .iter()
        .map(|source| split_scp_remote_operand(source))
        .collect::<Option<Vec<_>>>();
    if let Some(remote_sources) = remote_sources
        && let Some((target, _)) = remote_sources.first()
    {
        if remote_sources
            .iter()
            .any(|(next_target, _)| next_target != target)
        {
            return Err("scp multiple remote sources must use the same target".to_owned());
        }
        set_positional_ssh_target(state, target, "scp")?;
        let remotes = remote_sources
            .iter()
            .map(|(_, remote)| (*remote).to_owned())
            .collect::<Vec<_>>();
        return if let [remote] = remotes.as_slice() {
            set_scp_transfer(
                transfer,
                ScpTransfer::Download {
                    remote: remote.clone(),
                    local: PathBuf::from(destination),
                },
            )
        } else {
            set_scp_transfer(
                transfer,
                ScpTransfer::DownloadMany {
                    remotes,
                    local: PathBuf::from(destination),
                },
            )
        };
    }

    Ok(())
}

fn split_scp_remote_operand(operand: &str) -> Option<(&str, &str)> {
    let (target, remote) = operand.split_once(':')?;
    if target.is_empty() || remote.is_empty() || looks_like_windows_drive(target) {
        return None;
    }

    Some((target, remote))
}

fn looks_like_windows_drive(value: &str) -> bool {
    value.len() == 1 && value.as_bytes()[0].is_ascii_alphabetic()
}

#[cfg(feature = "ssh")]
fn ssh_target_selected(state: &SshParseState) -> bool {
    state.target.is_some()
}

fn set_scp_transfer(transfer: &mut Option<ScpTransfer>, next: ScpTransfer) -> Result<(), String> {
    if transfer.is_some() {
        return Err("only one scp transfer direction can be selected".to_owned());
    }

    *transfer = Some(next);
    Ok(())
}

fn set_console_metrics(
    metrics: &mut bool,
    metrics_json: &mut bool,
    selected: &str,
) -> Result<(), String> {
    if *metrics || *metrics_json {
        return Err("only one console metrics format can be selected".to_owned());
    }

    match selected {
        "--metrics" => *metrics = true,
        "--metrics-json" => *metrics_json = true,
        _ => unreachable!("validated console metrics flag"),
    }

    Ok(())
}

fn set_window_report_format(
    metrics: &mut bool,
    metrics_json: &mut bool,
    state: &mut bool,
    state_json: &mut bool,
    selected: &str,
) -> Result<(), String> {
    if *metrics || *metrics_json || *state || *state_json {
        return Err("only one window report format can be selected".to_owned());
    }

    match selected {
        "--metrics" => *metrics = true,
        "--metrics-json" => *metrics_json = true,
        "--state" => *state = true,
        "--state-json" => *state_json = true,
        _ => unreachable!("validated window report format"),
    }
    Ok(())
}

fn set_ssh_console_metrics(console: &mut ConsoleOptions, selected: &str) -> Result<(), String> {
    set_console_metrics(&mut console.metrics, &mut console.metrics_json, selected)
}

#[cfg(feature = "ssh")]
fn parse_sftp_option(
    args: &[String],
    index: &mut usize,
    state: &mut SshParseState,
) -> Result<(), String> {
    match args[*index].as_str() {
        "--host" => {
            *index += 1;
            state.host = Some(required_option_value(args.get(*index), "--host")?.to_owned());
        }
        "--target" => {
            *index += 1;
            set_explicit_ssh_target(state, required_option_value(args.get(*index), "--target")?)?;
        }
        "-l" => {
            *index += 1;
            let value = required_option_value(args.get(*index), "-l")?;
            if is_bandwidth_limit(value) {
                state.openssh_args.push("-l".to_owned());
                state.openssh_args.push(value.to_owned());
            } else {
                state.username = Some(value.to_owned());
            }
        }
        "--user" => {
            *index += 1;
            state.username = Some(required_option_value(args.get(*index), "--user")?.to_owned());
        }
        "-P" | "--port" => {
            *index += 1;
            state.port = Some(parse_port(args.get(*index), args[*index - 1].as_str())?);
        }
        "--cols" => {
            *index += 1;
            state.columns = Some(parse_dimension(args.get(*index), "--cols")?);
        }
        "--rows" => {
            *index += 1;
            state.rows = Some(parse_dimension(args.get(*index), "--rows")?);
        }
        "--agent" => {
            set_ssh_auth(&mut state.auth, SshAuthMethod::Agent)?;
        }
        "--password" => {
            set_ssh_auth(&mut state.auth, SshAuthMethod::PasswordPrompt)?;
        }
        "-i" | "--key" => {
            *index += 1;
            let path = required_option_value(args.get(*index), args[*index - 1].as_str())?;
            set_ssh_auth(
                &mut state.auth,
                SshAuthMethod::private_key(path, None::<String>)
                    .map_err(|error| error.to_string())?,
            )?;
        }
        "--passphrase" => {
            return Err(
                "--passphrase is not accepted on the command line; use the terminal prompt"
                    .to_owned(),
            );
        }
        value if is_openssh_value_passthrough(value) => {
            parse_ssh_passthrough_option(args, index, state)?;
        }
        value if is_sftp_value_passthrough(value) => {
            parse_ssh_passthrough_option(args, index, state)?;
        }
        value if is_openssh_flag_passthrough(value) => {
            state.openssh_args.push(value.to_owned());
        }
        "--preflight" => {
            state.console.preflight = true;
        }
        metrics_flag @ ("--metrics" | "--metrics-json") => {
            set_ssh_console_metrics(&mut state.console, metrics_flag)?;
        }
        "--log" => {
            state.log = Some(parse_path_option(args, index, "--log")?);
        }
        value if !value.starts_with('-') => set_positional_ssh_target(state, value, "sftp")?,
        value => return Err(format!("unexpected sftp option: {value}")),
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
#[cfg(feature = "ssh")]
fn parse_ssh_option(
    args: &[String],
    index: &mut usize,
    state: &mut SshParseState,
) -> Result<(), String> {
    match args[*index].as_str() {
        "--host" => {
            *index += 1;
            state.host = Some(required_option_value(args.get(*index), "--host")?.to_owned());
        }
        "--target" => {
            *index += 1;
            set_explicit_ssh_target(state, required_option_value(args.get(*index), "--target")?)?;
        }
        "-l" | "--user" => {
            *index += 1;
            state.username = Some(
                required_option_value(args.get(*index), args[*index - 1].as_str())?.to_owned(),
            );
        }
        "-p" | "--port" => {
            *index += 1;
            state.port = Some(parse_port(args.get(*index), args[*index - 1].as_str())?);
        }
        "--cols" => {
            *index += 1;
            state.columns = Some(parse_dimension(args.get(*index), "--cols")?);
        }
        "--rows" => {
            *index += 1;
            state.rows = Some(parse_dimension(args.get(*index), "--rows")?);
        }
        "--agent" => {
            set_ssh_auth(&mut state.auth, SshAuthMethod::Agent)?;
        }
        "--password" => {
            set_ssh_auth(&mut state.auth, SshAuthMethod::PasswordPrompt)?;
        }
        "-i" | "--key" => {
            *index += 1;
            let path = required_option_value(args.get(*index), args[*index - 1].as_str())?;
            set_ssh_auth(
                &mut state.auth,
                SshAuthMethod::private_key(path, None::<String>)
                    .map_err(|error| error.to_string())?,
            )?;
        }
        "--passphrase" => {
            return Err(
                "--passphrase is not accepted on the command line; use the terminal prompt"
                    .to_owned(),
            );
        }
        value if is_openssh_value_passthrough(value) => {
            parse_ssh_passthrough_option(args, index, state)?;
        }
        value if is_ssh_value_passthrough(value) => {
            parse_ssh_passthrough_option(args, index, state)?;
        }
        value if is_openssh_flag_passthrough(value) => {
            state.openssh_args.push(value.to_owned());
        }
        value if is_ssh_flag_passthrough(value) => {
            state.openssh_args.push(value.to_owned());
        }
        "-L" | "--local-forward" | "-R" | "--remote-forward" | "-D" | "--dynamic-forward" => {
            parse_ssh_forward_option(args, index, state)?;
        }
        "-N" | "--no-shell" => {
            state.no_shell = true;
        }
        "--native" => {
            state.native = true;
        }
        "--gui" => {
            state.gui = true;
            state.native = true;
        }
        "--renderer" => {
            *index += 1;
            state.renderer_selected = true;
            state.renderer =
                RendererMode::parse(required_option_value(args.get(*index), "--renderer")?)?;
        }
        "--benchmark-startup" => {
            state.benchmark_startup = true;
        }
        "--preflight" => {
            state.console.preflight = true;
        }
        metrics_flag @ ("--metrics" | "--metrics-json") => {
            set_ssh_console_metrics(&mut state.console, metrics_flag)?;
        }
        "--accept-unknown-host-key" | "--trust-on-first-use" => {
            parse_native_host_key_policy(args[*index].as_str(), state)?;
        }
        "--osc52" => {
            *index += 1;
            state.osc52_policy = parse_osc52_policy(args.get(*index))?;
        }
        "--log" => {
            state.log = Some(parse_path_option(args, index, "--log")?);
        }
        value if !value.starts_with('-') => set_positional_ssh_target(state, value, "ssh")?,
        value => return Err(format!("unexpected ssh option: {value}")),
    }

    Ok(())
}

#[cfg(feature = "ssh")]
fn parse_ssh_passthrough_option(
    args: &[String],
    index: &mut usize,
    state: &mut SshParseState,
) -> Result<(), String> {
    let option_name = args[*index].clone();
    *index += 1;
    let value = required_option_value(args.get(*index), option_name.as_str())?;
    state.openssh_args.push(option_name);
    state.openssh_args.push(value.to_owned());
    Ok(())
}

fn is_openssh_value_passthrough(value: &str) -> bool {
    matches!(value, "-F" | "-o" | "-J")
}

fn is_sftp_value_passthrough(value: &str) -> bool {
    matches!(value, "-b" | "-B" | "-c" | "-D" | "-R" | "-S" | "-s" | "-X")
}

fn is_scp_value_passthrough(value: &str) -> bool {
    matches!(value, "-c" | "-D" | "-S" | "-X")
}

fn is_bandwidth_limit(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|character| character.is_ascii_digit())
}

fn is_scp_flag_passthrough(value: &str) -> bool {
    matches!(value, "-3" | "-O" | "-T" | "-B" | "-p" | "-R" | "-s")
}

fn is_ssh_value_passthrough(value: &str) -> bool {
    matches!(
        value,
        "-B" | "-b" | "-c" | "-E" | "-e" | "-I" | "-m" | "-O" | "-P" | "-Q" | "-S" | "-W" | "-w"
    )
}

fn is_ssh_flag_passthrough(value: &str) -> bool {
    matches!(
        value,
        "-f" | "-G" | "-g" | "-K" | "-k" | "-M" | "-n" | "-s" | "-T" | "-X" | "-x" | "-Y" | "-y"
    ) || is_tty_request_flag(value)
}

fn is_tty_request_flag(value: &str) -> bool {
    let Some(rest) = value.strip_prefix('-') else {
        return false;
    };
    !rest.is_empty() && rest.chars().all(|character| character == 't')
}

fn is_openssh_flag_passthrough(value: &str) -> bool {
    matches!(value, "-4" | "-6" | "-A" | "-a" | "-C" | "-q") || is_verbose_flag(value)
}

fn is_verbose_flag(value: &str) -> bool {
    let Some(rest) = value.strip_prefix('-') else {
        return false;
    };
    !rest.is_empty() && rest.chars().all(|character| character == 'v')
}

#[cfg(feature = "ssh")]
fn parse_ssh_forward_option(
    args: &[String],
    index: &mut usize,
    state: &mut SshParseState,
) -> Result<(), String> {
    let option_name = args[*index].clone();
    *index += 1;
    let spec = required_forward_spec(args.get(*index), option_name.as_str())?;
    match option_name.as_str() {
        "-L" | "--local-forward" => state.forwards.push(SshForward::Local(spec)),
        "-R" | "--remote-forward" => state.forwards.push(SshForward::Remote(spec)),
        "-D" | "--dynamic-forward" => {
            require_loopback_dynamic_forward(&spec)?;
            state.forwards.push(SshForward::Dynamic(spec));
        }
        _ => unreachable!("only SSH forwarding options call this helper"),
    }
    Ok(())
}

fn require_loopback_dynamic_forward(spec: &str) -> Result<(), String> {
    let Some((host, _port)) = spec.rsplit_once(':') else {
        return Ok(());
    };
    let host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    let is_loopback = host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback());
    if is_loopback {
        Ok(())
    } else {
        Err(format!(
            "dynamic forwarding rejects non-loopback bind address {host:?}; the SOCKS5 listener has no authentication"
        ))
    }
}

#[cfg(feature = "ssh")]
fn set_explicit_ssh_target(state: &mut SshParseState, target: &str) -> Result<(), String> {
    if state.target.is_some() {
        return Err("only one SSH target can be selected".to_owned());
    }

    state.target = Some(target.to_owned());
    Ok(())
}

#[cfg(feature = "ssh")]
fn set_positional_ssh_target(
    state: &mut SshParseState,
    target: &str,
    command: &str,
) -> Result<(), String> {
    if state.host.is_some() {
        return Err(format!("unexpected {command} option: {target}"));
    }

    set_explicit_ssh_target(state, target)
}

fn parse_path_option(
    args: &[String],
    index: &mut usize,
    option_name: &str,
) -> Result<PathBuf, String> {
    *index += 1;
    Ok(PathBuf::from(required_option_value(
        args.get(*index),
        option_name,
    )?))
}

#[cfg(feature = "ssh")]
fn ssh_options_from_state(state: SshParseState) -> Result<SshOptions, String> {
    let SshParseState {
        host,
        target,
        username,
        port,
        columns,
        rows,
        auth,
        remote_command,
        forwards,
        openssh_args,
        no_shell,
        native,
        gui,
        renderer,
        renderer_selected,
        benchmark_startup,
        native_host_key_policy,
        console,
        osc52_policy,
        log,
    } = state;

    if no_shell && !remote_command.is_empty() {
        return Err("--no-shell cannot be combined with a remote command".to_owned());
    }

    if gui && !forwards.is_empty() {
        return Err("GUI SSH does not support forwarding options".to_owned());
    }
    if gui && no_shell {
        return Err("GUI SSH does not support --no-shell".to_owned());
    }
    if gui && !openssh_args.is_empty() {
        return Err("GUI SSH does not support OpenSSH passthrough options".to_owned());
    }
    if gui && console.preflight {
        return Err("GUI SSH does not support --preflight".to_owned());
    }
    if !gui && renderer_selected {
        return Err("--renderer requires --gui".to_owned());
    }
    if !gui && benchmark_startup {
        return Err("--benchmark-startup requires --gui".to_owned());
    }

    let size = ssh_terminal_size(columns, rows)?;
    let auth = auth.unwrap_or(SshAuthMethod::Agent);
    let target = match (host, target) {
        (Some(_), Some(_)) => {
            return Err("only one of --host or --target can be selected".to_owned());
        }
        (Some(host), None) => {
            let Some(username) = username else {
                return Err("--user is required with --host".to_owned());
            };
            let config = SshSessionConfig::try_new(host, port.unwrap_or(22), username, size.into())
                .map_err(|error| error.to_string())?;
            SshTarget::Direct(SshConnectRequest::new(config, auth))
        }
        (None, Some(target)) => SshTarget::OpenSsh(OpenSshTarget {
            target,
            username,
            port,
            initial_size: size,
            auth,
        }),
        (None, None) => return Err("--host or --target is required".to_owned()),
    };

    let native_backend = native || gui;
    if matches!(native_host_key_policy, NativeHostKeyPolicy::AcceptUnknown) && !native_backend {
        return Err("--accept-unknown-host-key requires --native or --gui".to_owned());
    }
    if matches!(native_host_key_policy, NativeHostKeyPolicy::TrustOnFirstUse) && !native_backend {
        return Err("--trust-on-first-use requires --native or --gui".to_owned());
    }
    if native && !openssh_args.is_empty() {
        return Err(
            "OpenSSH passthrough options require the OpenSSH console backend; remove --native"
                .to_owned(),
        );
    }

    Ok(SshOptions {
        target,
        remote_command,
        forwards,
        openssh_args,
        no_shell,
        native: native || gui,
        gui,
        renderer,
        benchmark_startup,
        native_host_key_policy,
        console,
        osc52_policy,
        log,
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn parse_window(args: &[String], config: WindowConfigOptions) -> Result<AppCommand, String> {
    let mut frame_limit = None;
    let mut workspace = None;
    let mut window_class = None;
    let mut position = None;
    let mut osc52_policy = Osc52Policy::default();
    let mut metrics = false;
    let mut metrics_json = false;
    let mut state = false;
    let mut state_json = false;
    let mut log = None;
    let mut cwd = None;
    let mut command_args = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--frames" => {
                index += 1;
                frame_limit = Some(parse_frame_limit(args.get(index))?);
            }
            "--osc52" => {
                index += 1;
                osc52_policy = parse_osc52_policy(args.get(index))?;
            }
            "--metrics" => {
                set_window_report_format(
                    &mut metrics,
                    &mut metrics_json,
                    &mut state,
                    &mut state_json,
                    "--metrics",
                )?;
            }
            "--metrics-json" => {
                set_window_report_format(
                    &mut metrics,
                    &mut metrics_json,
                    &mut state,
                    &mut state_json,
                    "--metrics-json",
                )?;
            }
            "--state" => {
                set_window_report_format(
                    &mut metrics,
                    &mut metrics_json,
                    &mut state,
                    &mut state_json,
                    "--state",
                )?;
            }
            "--state-json" => {
                set_window_report_format(
                    &mut metrics,
                    &mut metrics_json,
                    &mut state,
                    &mut state_json,
                    "--state-json",
                )?;
            }
            "--log" => {
                index += 1;
                log = Some(PathBuf::from(required_option_value(
                    args.get(index),
                    "--log",
                )?));
            }
            "--cwd" => {
                index += 1;
                cwd = Some(PathBuf::from(required_option_value(
                    args.get(index),
                    "--cwd",
                )?));
            }
            "--workspace" => {
                index += 1;
                workspace = Some(required_option_value(args.get(index), "--workspace")?.to_owned());
            }
            "--class" => {
                index += 1;
                window_class = Some(parse_window_class(required_option_value(
                    args.get(index),
                    "--class",
                )?)?);
            }
            "--position" => {
                index += 1;
                position = Some(parse_window_position(required_option_value(
                    args.get(index),
                    "--position",
                )?)?);
            }
            "--domain" => {
                index += 1;
                parse_window_domain(required_option_value(args.get(index), "--domain")?)?;
            }
            "--attach" | "--no-auto-connect" | "--always-new-process" | "--new-tab" => {}
            "-e" => {
                command_args = parse_window_exec_alias_command(args, index)?;
                break;
            }
            "--" => {
                command_args.extend(args[index + 1..].iter().cloned());
                break;
            }
            value if !value.starts_with('-') => {
                command_args.extend(args[index..].iter().cloned());
                break;
            }
            value => return Err(format!("unexpected window option: {value}")),
        }
        index += 1;
    }

    let command = window_command_from_args(command_args, cwd);

    Ok(AppCommand::Window(WindowOptions {
        config,
        frame_limit,
        workspace,
        window_class,
        position,
        osc52_policy,
        metrics,
        metrics_json,
        state,
        state_json,
        command,
        log,
    }))
}

fn window_command_from_args(command_args: Vec<String>, cwd: Option<PathBuf>) -> PtyCommand {
    let command = if command_args.is_empty() {
        PtyCommand::default_shell()
    } else {
        let mut iter = command_args.into_iter();
        let program = iter.next().expect("command_args is not empty");
        PtyCommand::new(program).with_args(iter)
    };

    match cwd {
        Some(cwd) => command.with_cwd(cwd),
        None => command,
    }
}

fn parse_window_exec_alias_command(args: &[String], index: usize) -> Result<Vec<String>, String> {
    if index + 1 >= args.len() {
        return Err("missing program for -e".to_owned());
    }

    Ok(args[index + 1..].to_vec())
}

fn parse_window_class(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Err("invalid value for --class: expected non-empty CLASS".to_owned());
    }

    Ok(value.to_owned())
}

fn parse_window_domain(value: &str) -> Result<(), String> {
    if value.eq_ignore_ascii_case("local") {
        return Ok(());
    }

    Err(format!(
        "unsupported --domain value: {value}; only the local domain is currently supported"
    ))
}

fn parse_window_position(value: &str) -> Result<WindowPosition, String> {
    let original_value = value;
    let (origin, value) = if let Some(value) = value.strip_prefix("screen:") {
        (WindowPositionOrigin::Screen, value)
    } else if let Some(value) = value.strip_prefix("main:") {
        (WindowPositionOrigin::Main, value)
    } else if let Some(value) = value.strip_prefix("active:") {
        (WindowPositionOrigin::Active, value)
    } else if let Some((monitor, coordinates)) = value.split_once(':') {
        if monitor.is_empty() {
            return Err(format!(
                "unsupported --position value: {original_value}; expected X,Y, screen:X,Y, main:X,Y, active:X,Y, or <monitor>:X,Y"
            ));
        }
        (
            WindowPositionOrigin::Monitor(monitor.to_owned()),
            coordinates,
        )
    } else {
        (WindowPositionOrigin::Screen, value)
    };

    if value.contains(':') {
        return Err(format!(
            "unsupported --position value: {original_value}; expected X,Y, screen:X,Y, main:X,Y, active:X,Y, or <monitor>:X,Y"
        ));
    }

    let Some((x, y)) = value.split_once(',') else {
        return Err(format!(
            "invalid value for --position: {original_value}; expected X,Y, screen:X,Y, main:X,Y, active:X,Y, or <monitor>:X,Y"
        ));
    };
    if y.contains(',') {
        return Err(format!(
            "invalid value for --position: {original_value}; expected X,Y, screen:X,Y, main:X,Y, active:X,Y, or <monitor>:X,Y"
        ));
    }

    let x = x
        .parse::<i32>()
        .map_err(|_| format!("invalid X coordinate for --position: {x}"))?;
    let y = y
        .parse::<i32>()
        .map_err(|_| format!("invalid Y coordinate for --position: {y}"))?;

    Ok(WindowPosition { origin, x, y })
}

fn parse_osc52_policy(value: Option<&String>) -> Result<Osc52Policy, String> {
    let Some(value) = value else {
        return Err("missing value for --osc52".to_owned());
    };

    match value.as_str() {
        "off" => Ok(Osc52Policy::Off),
        "write" => Ok(Osc52Policy::WriteOnly),
        "read-write" => Ok(Osc52Policy::ReadWrite),
        _ => Err(format!("invalid value for --osc52: {value}")),
    }
}

fn parse_frame_limit(value: Option<&String>) -> Result<u64, String> {
    let Some(value) = value else {
        return Err("missing value for --frames".to_owned());
    };

    value
        .parse::<u64>()
        .map_err(|_| format!("invalid value for --frames: {value}"))
}

fn parse_port(value: Option<&String>, name: &str) -> Result<u16, String> {
    let Some(value) = value else {
        return Err(format!("missing value for {name}"));
    };

    value
        .parse::<u16>()
        .map_err(|_| format!("invalid value for {name}: {value}"))
}

fn parse_dimension(value: Option<&String>, name: &str) -> Result<u16, String> {
    let Some(value) = value else {
        return Err(format!("missing value for {name}"));
    };

    value
        .parse::<u16>()
        .map_err(|_| format!("invalid value for {name}: {value}"))
}

fn parse_nonzero_dimension(value: Option<&String>, name: &str) -> Result<u16, String> {
    let dimension = parse_dimension(value, name)?;
    if dimension == 0 {
        return Err(format!("{name} must be greater than zero"));
    }

    Ok(dimension)
}

fn parse_nonzero_usize(value: Option<&String>, name: &str) -> Result<usize, String> {
    let Some(value) = value else {
        return Err(format!("missing value for {name}"));
    };

    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("invalid value for {name}: {value}"))?;
    if parsed == 0 {
        return Err(format!("{name} must be greater than zero"));
    }

    Ok(parsed)
}

fn required_option_value<'a>(value: Option<&'a String>, name: &str) -> Result<&'a str, String> {
    value
        .map(String::as_str)
        .ok_or_else(|| format!("missing value for {name}"))
}

fn required_forward_spec(value: Option<&String>, name: &str) -> Result<String, String> {
    let value = required_option_value(value, name)?;
    if value.trim().is_empty() {
        return Err(format!("{name} cannot be empty"));
    }

    Ok(value.to_owned())
}

#[cfg(feature = "ssh")]
fn set_ssh_auth(auth: &mut Option<SshAuthMethod>, next: SshAuthMethod) -> Result<(), String> {
    if auth.is_some() {
        return Err("only one ssh authentication method can be selected".to_owned());
    }

    *auth = Some(next);
    Ok(())
}

#[cfg(feature = "ssh")]
fn parse_native_host_key_policy(option: &str, state: &mut SshParseState) -> Result<(), String> {
    let policy = match option {
        "--accept-unknown-host-key" => NativeHostKeyPolicy::AcceptUnknown,
        "--trust-on-first-use" => NativeHostKeyPolicy::TrustOnFirstUse,
        _ => unreachable!("only native host-key policy options call this helper"),
    };

    set_native_host_key_policy(&mut state.native_host_key_policy, policy)
}

#[cfg(feature = "ssh")]
fn set_native_host_key_policy(
    policy: &mut NativeHostKeyPolicy,
    next: NativeHostKeyPolicy,
) -> Result<(), String> {
    if !matches!(policy, NativeHostKeyPolicy::RejectUnknown) {
        return Err("only one native SSH host-key policy can be selected".to_owned());
    }

    *policy = next;
    Ok(())
}

fn ssh_terminal_size(columns: Option<u16>, rows: Option<u16>) -> Result<TerminalSize, String> {
    match (columns, rows) {
        (None, None) => Ok(ssh_default_terminal_size()),
        (Some(columns), Some(rows)) => Ok(TerminalSize::new(columns, rows)),
        (None, Some(_)) => Err("--rows requires --cols".to_owned()),
        (Some(_), None) => Err("--cols requires --rows".to_owned()),
    }
}

const fn ssh_default_terminal_size() -> TerminalSize {
    TerminalSize::new(DEFAULT_SSH_COLUMNS, DEFAULT_SSH_ROWS)
}

#[cfg(not(feature = "ssh"))]
#[derive(Debug, PartialEq, Eq)]
pub struct SshOptions;
#[cfg(not(feature = "ssh"))]
#[derive(Debug, PartialEq, Eq)]
pub struct ScpOptions;
#[cfg(not(feature = "ssh"))]
#[derive(Debug, PartialEq, Eq)]
pub struct SftpOptions;
#[cfg(not(feature = "ssh"))]
fn parse_ssh(_: &[String]) -> Result<AppCommand, String> {
    Err("SSH support is disabled; rebuild rterm with --features ssh".to_owned())
}
#[cfg(not(feature = "ssh"))]
fn parse_scp(args: &[String]) -> Result<AppCommand, String> {
    parse_ssh(args)
}
#[cfg(not(feature = "ssh"))]
fn parse_sftp(args: &[String]) -> Result<AppCommand, String> {
    parse_ssh(args)
}

pub fn help_text() -> String {
    let mut text = all_command_help()
        .lines()
        .filter(|line| {
            let command = line.trim_start();
            (cfg!(feature = "ssh") || !command.starts_with("rterm ssh "))
                && (cfg!(feature = "transfer-tools")
                    || !(command.starts_with("rterm scp ") || command.starts_with("rterm sftp ")))
                && (cfg!(feature = "diagnostic-tools")
                    || ![
                        "rterm doctor ",
                        "rterm bench ",
                        "rterm self-test ",
                        "rterm diagnostic-gui ",
                    ]
                    .iter()
                    .any(|prefix| command.starts_with(prefix)))
        })
        .collect::<Vec<_>>()
        .join("\n");
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "ssh")]
    use rssh_ssh::SshAuthMethod;

    #[cfg(feature = "ssh")]
    use super::NativeHostKeyPolicy;
    use super::{AppCommand, parse_args};

    #[test]
    fn parses_default_window_command() {
        assert_eq!(
            parse_args(["rterm"]).unwrap(),
            AppCommand::Window(super::WindowOptions {
                config: super::WindowConfigOptions::default(),
                frame_limit: None,
                workspace: None,
                window_class: None,
                position: None,
                osc52_policy: super::Osc52Policy::WriteOnly,
                metrics: false,
                metrics_json: false,
                state: false,
                state_json: false,
                command: rssh_pty::PtyCommand::default_shell(),
                log: None
            })
        );
    }

    #[test]
    fn parses_global_wezterm_config_options_for_default_window() {
        let parsed = parse_args([
            "rterm",
            "-n",
            "--config",
            "color_scheme=Builtin Solarized Dark",
        ])
        .unwrap();

        let AppCommand::Window(options) = parsed else {
            panic!("expected window command");
        };

        assert_eq!(
            options.config,
            super::WindowConfigOptions {
                skip_config: true,
                config_file: None,
                config_overrides: vec![(
                    "color_scheme".to_owned(),
                    "Builtin Solarized Dark".to_owned()
                )],
            }
        );
    }

    #[test]
    fn parses_repeated_global_config_overrides_in_order() {
        let parsed = parse_args([
            "rterm",
            "--config",
            "color_scheme=Builtin Solarized Dark",
            "--config",
            "font=JetBrains=Mono",
        ])
        .unwrap();

        let AppCommand::Window(options) = parsed else {
            panic!("expected window command");
        };

        assert_eq!(
            options.config.config_overrides,
            [
                (
                    "color_scheme".to_owned(),
                    "Builtin Solarized Dark".to_owned()
                ),
                ("font".to_owned(), "JetBrains=Mono".to_owned()),
            ]
        );
    }

    #[test]
    fn parses_global_config_options_before_window_and_start() {
        let parsed_window = parse_args([
            "rterm",
            "--config-file",
            "C:/Users/test/.wezterm.lua",
            "window",
            "--frames",
            "1",
        ])
        .unwrap();
        let parsed_start = parse_args([
            "rterm",
            "--skip-config",
            "--config",
            "term=xterm-256color",
            "start",
            "-e",
            "cmd.exe",
            "/K",
        ])
        .unwrap();

        let AppCommand::Window(window) = parsed_window else {
            panic!("expected window command");
        };
        let AppCommand::Window(start) = parsed_start else {
            panic!("expected start alias to produce window command");
        };

        assert_eq!(
            window.config,
            super::WindowConfigOptions {
                skip_config: false,
                config_file: Some("C:/Users/test/.wezterm.lua".into()),
                config_overrides: Vec::new(),
            }
        );
        assert_eq!(window.frame_limit, Some(1));
        assert_eq!(
            start.config,
            super::WindowConfigOptions {
                skip_config: true,
                config_file: None,
                config_overrides: vec![("term".to_owned(), "xterm-256color".to_owned())],
            }
        );
        assert_eq!(start.command.program(), "cmd.exe");
        assert_eq!(start.command.args(), ["/K"]);
    }

    #[test]
    fn rejects_skip_config_with_config_file() {
        for args in [
            vec!["rterm", "--skip-config", "--config-file", "wezterm.lua"],
            vec!["rterm", "--config-file", "wezterm.lua", "-n"],
        ] {
            assert_eq!(
                parse_args(args).unwrap_err(),
                "--skip-config conflicts with --config-file"
            );
        }
    }

    #[test]
    fn rejects_malformed_global_config_override() {
        for value in ["=value", "name=   ", "name"] {
            assert_eq!(
                parse_args(["rterm", "--config", value]).unwrap_err(),
                "invalid value for --config: expected NAME=VALUE with non-empty NAME and VALUE"
            );
        }
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_global_config_options_for_non_gui_commands() {
        for command in [
            "bench",
            "local",
            "console",
            "doctor",
            "profile",
            "scp",
            "sftp",
            "ssh",
            "version",
            "self-test",
        ] {
            assert_eq!(
                parse_args(["rterm", "--config", "term=xterm-256color", command]).unwrap_err(),
                "global WezTerm config options cannot be used with non-GUI commands"
            );
        }
    }

    #[test]
    fn rejects_wezterm_config_options_after_window_and_start() {
        assert_eq!(
            parse_args(["rterm", "window", "--skip-config"]).unwrap_err(),
            "unexpected window option: --skip-config"
        );
        assert_eq!(
            parse_args(["rterm", "start", "--config-file", "wezterm.lua"]).unwrap_err(),
            "unexpected window option: --config-file"
        );
        assert_eq!(
            parse_args(["rterm", "window", "--config", "term=xterm"]).unwrap_err(),
            "unexpected window option: --config"
        );
    }

    #[test]
    fn parses_explicit_window_command() {
        assert_eq!(
            parse_args(["rterm", "window"]).unwrap(),
            AppCommand::Window(super::WindowOptions {
                config: super::WindowConfigOptions::default(),
                frame_limit: None,
                workspace: None,
                window_class: None,
                position: None,
                osc52_policy: super::Osc52Policy::WriteOnly,
                metrics: false,
                metrics_json: false,
                state: false,
                state_json: false,
                command: rssh_pty::PtyCommand::default_shell(),
                log: None
            })
        );
    }

    #[test]
    fn parses_start_alias_as_window_command() {
        assert_eq!(
            parse_args(["rterm", "start"]).unwrap(),
            AppCommand::Window(super::WindowOptions {
                config: super::WindowConfigOptions::default(),
                frame_limit: None,
                workspace: None,
                window_class: None,
                position: None,
                osc52_policy: super::Osc52Policy::WriteOnly,
                metrics: false,
                metrics_json: false,
                state: false,
                state_json: false,
                command: rssh_pty::PtyCommand::default_shell(),
                log: None,
            })
        );
    }

    #[test]
    fn parses_start_alias_exec_command() {
        let parsed = parse_args([
            "rterm",
            "start",
            "-e",
            "powershell",
            "-NoProfile",
            "-Command",
            "Write-Output start-smoke",
        ])
        .unwrap();

        let AppCommand::Window(options) = parsed else {
            panic!("expected window command");
        };

        assert_eq!(options.command.program(), "powershell");
        assert_eq!(
            options.command.args(),
            ["-NoProfile", "-Command", "Write-Output start-smoke"]
        );
    }

    #[test]
    fn parses_start_alias_bare_program_arguments() {
        let parsed = parse_args([
            "rterm",
            "start",
            "--cwd",
            "E:\\project",
            "powershell",
            "-NoProfile",
            "-Command",
            "Write-Output start-bare-smoke",
        ])
        .unwrap();

        let AppCommand::Window(options) = parsed else {
            panic!("expected window command");
        };

        assert_eq!(
            options.command.cwd(),
            Some(std::path::Path::new("E:\\project"))
        );
        assert_eq!(options.command.program(), "powershell");
        assert_eq!(
            options.command.args(),
            ["-NoProfile", "-Command", "Write-Output start-bare-smoke"]
        );
    }

    #[test]
    fn parses_start_alias_help() {
        assert_eq!(
            parse_args(["rterm", "start", "--help"]).unwrap(),
            AppCommand::Help
        );
    }

    #[test]
    fn parses_doctor_command() {
        assert_eq!(
            parse_args(["rterm", "doctor"]).unwrap(),
            AppCommand::Doctor(super::DoctorOptions { json: false })
        );
    }

    #[test]
    fn parses_json_doctor_command() {
        assert_eq!(
            parse_args(["rterm", "doctor", "--json"]).unwrap(),
            AppCommand::Doctor(super::DoctorOptions { json: true })
        );
    }

    #[test]
    fn parses_version_command() {
        assert_eq!(
            parse_args(["rterm", "version"]).unwrap(),
            AppCommand::Version(super::VersionOptions { json: false })
        );
    }

    #[test]
    fn parses_json_version_command() {
        assert_eq!(
            parse_args(["rterm", "version", "--json"]).unwrap(),
            AppCommand::Version(super::VersionOptions { json: true })
        );
    }

    #[test]
    fn parses_self_test_command() {
        assert_eq!(
            parse_args(["rterm", "self-test"]).unwrap(),
            AppCommand::SelfTest(super::SelfTestOptions { json: false })
        );
    }

    #[test]
    fn parses_json_self_test_command() {
        assert_eq!(
            parse_args(["rterm", "self-test", "--json"]).unwrap(),
            AppCommand::SelfTest(super::SelfTestOptions { json: true })
        );
    }

    #[test]
    fn parses_profile_command_with_config_file() {
        assert_eq!(
            parse_args(["rterm", "profile", "prod", "--file", "profiles.toml"]).unwrap(),
            AppCommand::Profile(super::ProfileOptions {
                name: "prod".to_owned(),
                file: std::path::PathBuf::from("profiles.toml"),
            })
        );
    }

    #[test]
    fn parses_profile_list_command_with_config_file() {
        assert_eq!(
            parse_args(["rterm", "profile", "--list", "--file", "profiles.toml"]).unwrap(),
            AppCommand::ProfileList(super::ProfileListOptions {
                file: std::path::PathBuf::from("profiles.toml"),
                verbose: false,
                json: false,
            })
        );
    }

    #[test]
    fn parses_verbose_profile_list_command_with_config_file() {
        assert_eq!(
            parse_args([
                "rterm",
                "profile",
                "--list",
                "--verbose",
                "--file",
                "profiles.toml"
            ])
            .unwrap(),
            AppCommand::ProfileList(super::ProfileListOptions {
                file: std::path::PathBuf::from("profiles.toml"),
                verbose: true,
                json: false,
            })
        );
    }

    #[test]
    fn parses_json_profile_list_command_with_config_file() {
        assert_eq!(
            parse_args([
                "rterm",
                "profile",
                "--list",
                "--json",
                "--file",
                "profiles.toml"
            ])
            .unwrap(),
            AppCommand::ProfileList(super::ProfileListOptions {
                file: std::path::PathBuf::from("profiles.toml"),
                verbose: false,
                json: true,
            })
        );
    }

    #[test]
    fn parses_profile_check_command_with_config_file() {
        assert_eq!(
            parse_args(["rterm", "profile", "--check", "--file", "profiles.toml"]).unwrap(),
            AppCommand::ProfileCheck(super::ProfileCheckOptions {
                file: std::path::PathBuf::from("profiles.toml"),
                json: false,
            })
        );
    }

    #[test]
    fn parses_json_profile_check_command_with_config_file() {
        assert_eq!(
            parse_args([
                "rterm",
                "profile",
                "--check",
                "--json",
                "--file",
                "profiles.toml"
            ])
            .unwrap(),
            AppCommand::ProfileCheck(super::ProfileCheckOptions {
                file: std::path::PathBuf::from("profiles.toml"),
                json: true,
            })
        );
    }

    #[test]
    fn parses_profile_init_command_with_force_and_config_file() {
        assert_eq!(
            parse_args([
                "rterm",
                "profile",
                "--init",
                "--force",
                "--file",
                "profiles.toml"
            ])
            .unwrap(),
            AppCommand::ProfileInit(super::ProfileInitOptions {
                file: std::path::PathBuf::from("profiles.toml"),
                force: true,
            })
        );
    }

    #[test]
    fn parses_profile_show_command_with_config_file() {
        assert_eq!(
            parse_args([
                "rterm",
                "profile",
                "--show",
                "prod",
                "--file",
                "profiles.toml"
            ])
            .unwrap(),
            AppCommand::ProfileShow(super::ProfileShowOptions {
                name: "prod".to_owned(),
                file: std::path::PathBuf::from("profiles.toml"),
                json: false,
            })
        );
    }

    #[test]
    fn parses_json_profile_show_command_with_config_file() {
        assert_eq!(
            parse_args([
                "rterm",
                "profile",
                "--show",
                "prod",
                "--json",
                "--file",
                "profiles.toml"
            ])
            .unwrap(),
            AppCommand::ProfileShow(super::ProfileShowOptions {
                name: "prod".to_owned(),
                file: std::path::PathBuf::from("profiles.toml"),
                json: true,
            })
        );
    }

    #[test]
    fn parses_window_frame_limit() {
        assert_eq!(
            parse_args(["rterm", "window", "--frames", "1"]).unwrap(),
            AppCommand::Window(super::WindowOptions {
                config: super::WindowConfigOptions::default(),
                frame_limit: Some(1),
                workspace: None,
                window_class: None,
                position: None,
                osc52_policy: super::Osc52Policy::WriteOnly,
                metrics: false,
                metrics_json: false,
                state: false,
                state_json: false,
                command: rssh_pty::PtyCommand::default_shell(),
                log: None
            })
        );
    }

    #[test]
    fn parses_window_metrics_flag() {
        assert_eq!(
            parse_args(["rterm", "window", "--metrics"]).unwrap(),
            AppCommand::Window(super::WindowOptions {
                config: super::WindowConfigOptions::default(),
                frame_limit: None,
                workspace: None,
                window_class: None,
                position: None,
                osc52_policy: super::Osc52Policy::WriteOnly,
                metrics: true,
                metrics_json: false,
                state: false,
                state_json: false,
                command: rssh_pty::PtyCommand::default_shell(),
                log: None
            })
        );
    }

    #[test]
    fn parses_window_metrics_json_flag() {
        assert_eq!(
            parse_args(["rterm", "window", "--metrics-json"]).unwrap(),
            AppCommand::Window(super::WindowOptions {
                config: super::WindowConfigOptions::default(),
                frame_limit: None,
                workspace: None,
                window_class: None,
                position: None,
                osc52_policy: super::Osc52Policy::WriteOnly,
                metrics: false,
                metrics_json: true,
                state: false,
                state_json: false,
                command: rssh_pty::PtyCommand::default_shell(),
                log: None
            })
        );
    }

    #[test]
    fn parses_window_state_flags() {
        let AppCommand::Window(text) = parse_args(["rterm", "window", "--state"]).unwrap() else {
            panic!("expected window command");
        };
        assert!(text.state);
        assert!(!text.state_json);
        assert!(!text.metrics);
        assert!(!text.metrics_json);

        let AppCommand::Window(json) = parse_args(["rterm", "window", "--state-json"]).unwrap()
        else {
            panic!("expected window command");
        };
        assert!(!json.state);
        assert!(json.state_json);
        assert!(!json.metrics);
        assert!(!json.metrics_json);
    }

    #[test]
    fn window_report_formats_are_strictly_mutually_exclusive() {
        let flags = ["--metrics", "--metrics-json", "--state", "--state-json"];
        for left in flags {
            for right in flags {
                let error = parse_args(["rterm", "window", left, right]).unwrap_err();
                assert!(
                    error.contains("only one window report format can be selected"),
                    "{left} with {right}: {error}"
                );
            }
        }
    }

    #[test]
    fn window_help_lists_state_and_metrics_as_one_choice() {
        assert!(
            super::help_text().contains("[--metrics | --metrics-json | --state | --state-json]")
        );
    }

    #[test]
    fn parses_window_custom_command_after_separator() {
        let parsed = parse_args([
            "rterm",
            "window",
            "--",
            "powershell",
            "-NoProfile",
            "-Command",
            "Write-Output window-smoke",
        ])
        .unwrap();

        let AppCommand::Window(options) = parsed else {
            panic!("expected window command");
        };

        assert_eq!(options.command.program(), "powershell");
        assert_eq!(
            options.command.args(),
            ["-NoProfile", "-Command", "Write-Output window-smoke"]
        );
    }

    #[test]
    fn parses_window_exec_alias_for_initial_command() {
        let parsed = parse_args([
            "rterm",
            "window",
            "-e",
            "powershell",
            "-NoProfile",
            "-Command",
            "Write-Output window-smoke",
        ])
        .unwrap();

        let AppCommand::Window(options) = parsed else {
            panic!("expected window command");
        };

        assert_eq!(options.command.program(), "powershell");
        assert_eq!(
            options.command.args(),
            ["-NoProfile", "-Command", "Write-Output window-smoke"]
        );
    }

    #[test]
    fn rejects_window_exec_alias_without_program() {
        let error =
            parse_args(["rterm", "window", "-e"]).expect_err("exec alias requires a program");

        assert_eq!(error, "missing program for -e");
    }

    #[test]
    fn parses_window_cwd_for_initial_command() {
        let parsed = parse_args([
            "rterm",
            "window",
            "--cwd",
            "E:\\project",
            "--",
            "powershell",
            "-NoProfile",
        ])
        .unwrap();

        let AppCommand::Window(options) = parsed else {
            panic!("expected window command");
        };

        assert_eq!(
            options.command.cwd(),
            Some(std::path::Path::new("E:\\project"))
        );
        assert_eq!(options.command.program(), "powershell");
        assert_eq!(options.command.args(), ["-NoProfile"]);
    }

    #[test]
    fn parses_window_workspace_for_initial_window() {
        let parsed = parse_args(["rterm", "window", "--workspace", "ops"]).unwrap();

        let AppCommand::Window(options) = parsed else {
            panic!("expected window command");
        };

        assert_eq!(options.workspace.as_deref(), Some("ops"));
    }

    #[test]
    fn parses_window_position_for_initial_window() {
        let parsed = parse_args(["rterm", "window", "--position", "10,20"]).unwrap();

        let AppCommand::Window(options) = parsed else {
            panic!("expected window command");
        };

        assert_eq!(
            options.position,
            Some(super::WindowPosition {
                origin: super::WindowPositionOrigin::Screen,
                x: 10,
                y: 20
            })
        );
    }

    #[test]
    fn parses_window_screen_position_for_initial_window() {
        let parsed = parse_args(["rterm", "window", "--position", "screen:10,20"]).unwrap();

        let AppCommand::Window(options) = parsed else {
            panic!("expected window command");
        };

        assert_eq!(
            options.position,
            Some(super::WindowPosition {
                origin: super::WindowPositionOrigin::Screen,
                x: 10,
                y: 20
            })
        );
    }

    #[test]
    fn parses_window_main_monitor_position_for_initial_window() {
        let parsed = parse_args(["rterm", "window", "--position", "main:10,20"]).unwrap();

        let AppCommand::Window(options) = parsed else {
            panic!("expected window command");
        };

        assert_eq!(
            options.position,
            Some(super::WindowPosition {
                origin: super::WindowPositionOrigin::Main,
                x: 10,
                y: 20
            })
        );
    }

    #[test]
    fn parses_window_active_monitor_position_for_initial_window() {
        let parsed = parse_args(["rterm", "window", "--position", "active:10,20"]).unwrap();

        let AppCommand::Window(options) = parsed else {
            panic!("expected window command");
        };

        let position = options.position.expect("expected initial window position");
        assert_eq!(position.origin, super::WindowPositionOrigin::Active);
        assert_eq!(position.x, 10);
        assert_eq!(position.y, 20);
    }

    #[test]
    fn parses_window_named_monitor_position_for_initial_window() {
        let parsed = parse_args(["rterm", "window", "--position", "HDMI-1:10,20"]).unwrap();

        let AppCommand::Window(options) = parsed else {
            panic!("expected window command");
        };

        assert_eq!(
            options.position,
            Some(super::WindowPosition {
                origin: super::WindowPositionOrigin::Monitor("HDMI-1".to_owned()),
                x: 10,
                y: 20
            })
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_window_class_for_initial_window() {
        let parsed = parse_args(["rterm", "window", "--class", "org.example.RSsh"]).unwrap();

        let AppCommand::Window(options) = parsed else {
            panic!("expected window command");
        };

        assert_eq!(options.window_class.as_deref(), Some("org.example.RSsh"));
    }

    #[test]
    fn accepts_wezterm_startup_compatibility_flags_for_window() {
        let parsed = parse_args([
            "rterm",
            "window",
            "--no-auto-connect",
            "--always-new-process",
            "--new-tab",
            "--workspace",
            "ops",
            "--",
            "powershell",
            "-NoProfile",
        ])
        .unwrap();

        let AppCommand::Window(options) = parsed else {
            panic!("expected window command");
        };

        assert_eq!(options.workspace.as_deref(), Some("ops"));
        assert_eq!(options.command.program(), "powershell");
        assert_eq!(options.command.args(), ["-NoProfile"]);
    }

    #[test]
    fn accepts_local_domain_and_attach_for_window_startup() {
        let parsed = parse_args([
            "rterm",
            "window",
            "--domain",
            "local",
            "--attach",
            "--",
            "powershell",
            "-NoProfile",
        ])
        .unwrap();

        let AppCommand::Window(options) = parsed else {
            panic!("expected window command");
        };

        assert_eq!(options.command.program(), "powershell");
        assert_eq!(options.command.args(), ["-NoProfile"]);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_remote_domain_for_window_startup() {
        let error = parse_args(["rterm", "window", "--domain", "ssh-prod"])
            .expect_err("remote domains are not implemented");

        assert_eq!(
            error,
            "unsupported --domain value: ssh-prod; only the local domain is currently supported"
        );
    }

    #[test]
    fn parses_window_log_path() {
        let parsed = parse_args(["rterm", "window", "--log", "window.log"]).unwrap();

        let AppCommand::Window(options) = parsed else {
            panic!("expected window command");
        };

        assert_eq!(options.log, Some(std::path::PathBuf::from("window.log")));
    }

    #[test]
    fn parses_window_osc52_policy() {
        assert_eq!(
            parse_args(["rterm", "window", "--osc52", "off"]).unwrap(),
            AppCommand::Window(super::WindowOptions {
                config: super::WindowConfigOptions::default(),
                frame_limit: None,
                workspace: None,
                window_class: None,
                position: None,
                osc52_policy: super::Osc52Policy::Off,
                metrics: false,
                metrics_json: false,
                state: false,
                state_json: false,
                command: rssh_pty::PtyCommand::default_shell(),
                log: None
            })
        );
        assert_eq!(
            parse_args(["rterm", "window", "--osc52", "write"]).unwrap(),
            AppCommand::Window(super::WindowOptions {
                config: super::WindowConfigOptions::default(),
                frame_limit: None,
                workspace: None,
                window_class: None,
                position: None,
                osc52_policy: super::Osc52Policy::WriteOnly,
                metrics: false,
                metrics_json: false,
                state: false,
                state_json: false,
                command: rssh_pty::PtyCommand::default_shell(),
                log: None
            })
        );
        assert!(parse_args(["rterm", "window", "--osc52", "bad"]).is_err());
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn defaults_local_osc52_to_write_only_and_remote_ssh_to_off() {
        assert_eq!(super::Osc52Policy::default(), super::Osc52Policy::WriteOnly);
        assert!(super::Osc52Policy::default().allows_write());
        assert!(!super::Osc52Policy::default().allows_query());

        let parsed = parse_args(["rterm"]).unwrap();
        let AppCommand::Window(options) = parsed else {
            panic!("expected default window command");
        };
        assert_eq!(options.osc52_policy, super::Osc52Policy::WriteOnly);

        let parsed = parse_args(["rterm", "window"]).unwrap();
        let AppCommand::Window(options) = parsed else {
            panic!("expected window command");
        };
        assert_eq!(options.osc52_policy, super::Osc52Policy::WriteOnly);

        let parsed = parse_args(["rterm", "local"]).unwrap();
        let AppCommand::Local(options) = parsed else {
            panic!("expected local command");
        };
        assert_eq!(options.osc52_policy, super::Osc52Policy::WriteOnly);

        let parsed = parse_args(["rterm", "ssh", "example.com"]).unwrap();
        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };
        assert_eq!(options.osc52_policy, super::Osc52Policy::Off);
    }

    #[test]
    fn parses_local_default_shell() {
        let parsed = parse_args(["rterm", "local"]).unwrap();

        let AppCommand::Local(options) = parsed else {
            panic!("expected local command");
        };

        assert!(!options.command.program().is_empty());
        assert_eq!(options.size, None);
        assert!(!options.mouse);
        assert!(!options.console.preflight);
        assert!(!options.console.metrics);
    }

    #[test]
    fn parses_console_alias_as_local_command() {
        let parsed = parse_args([
            "rterm",
            "console",
            "--preflight",
            "--",
            "cmd.exe",
            "/C",
            "echo console-alias",
        ])
        .unwrap();

        let AppCommand::Local(options) = parsed else {
            panic!("expected local command");
        };

        assert_eq!(options.command.program(), "cmd.exe");
        assert_eq!(options.command.args(), ["/C", "echo console-alias"]);
        assert!(options.console.preflight);
    }

    #[test]
    fn parses_console_benchmark_command() {
        let parsed = parse_args(["rterm", "bench"]).unwrap();

        let AppCommand::Bench(options) = parsed else {
            panic!("expected bench command");
        };

        assert!(!options.json);
        assert_eq!(options.workload, super::BenchWorkload::AnsiScrollQuery);
        assert_eq!(options.bytes, 1_048_576);
        assert_eq!(options.chunk_size, 8192);
        assert_eq!(options.render_frames, 30);
        assert_eq!(options.idle_ms, 200);
        assert_eq!(options.thresholds, super::BenchThresholds::default());
        assert_eq!(options.size, rssh_core::TerminalSize::new(120, 30));
    }

    #[test]
    fn parses_console_benchmark_options() {
        let parsed = parse_args([
            "rterm",
            "bench",
            "--json",
            "--workload",
            "ansi-scroll",
            "--bytes",
            "4096",
            "--chunk-size",
            "512",
            "--render-frames",
            "7",
            "--idle-ms",
            "250",
            "--min-throughput-bytes-per-sec",
            "100000",
            "--max-chunk-p95-us",
            "2000",
            "--max-render-frame-p95-us",
            "16000",
            "--max-idle-cpu-percent",
            "3",
            "--max-process-memory-bytes",
            "268435456",
            "--cols",
            "100",
            "--rows",
            "40",
        ])
        .unwrap();

        let AppCommand::Bench(options) = parsed else {
            panic!("expected bench command");
        };

        assert!(options.json);
        assert_eq!(options.workload, super::BenchWorkload::AnsiScroll);
        assert_eq!(options.bytes, 4096);
        assert_eq!(options.chunk_size, 512);
        assert_eq!(options.render_frames, 7);
        assert_eq!(options.idle_ms, 250);
        assert_eq!(
            options.thresholds,
            super::BenchThresholds {
                min_throughput_bytes_per_sec: Some(100_000),
                max_chunk_p95_us: Some(2_000),
                max_render_frame_p95_us: Some(16_000),
                max_idle_cpu_percent: Some(3),
                max_process_memory_bytes: Some(268_435_456),
            }
        );
        assert_eq!(options.size, rssh_core::TerminalSize::new(100, 40));
    }

    #[test]
    fn rejects_invalid_console_benchmark_options() {
        assert!(parse_args(["rterm", "bench", "--bytes", "0"]).is_err());
        assert!(parse_args(["rterm", "bench", "--chunk-size", "0"]).is_err());
        assert!(parse_args(["rterm", "bench", "--render-frames", "0"]).is_err());
        assert!(parse_args(["rterm", "bench", "--idle-ms", "0"]).is_err());
        assert!(parse_args(["rterm", "bench", "--max-chunk-p95-us", "0"]).is_err());
        assert!(parse_args(["rterm", "bench", "--max-idle-cpu-percent", "0"]).is_err());
        assert!(parse_args(["rterm", "bench", "--cols", "0"]).is_err());
        assert!(parse_args(["rterm", "bench", "--rows", "0"]).is_err());
        assert_eq!(
            parse_args(["rterm", "bench", "--workload", "unknown"]).unwrap_err(),
            "unknown bench workload: unknown (expected plain-scroll, ansi-scroll, or ansi-scroll-query)"
        );
    }

    #[test]
    fn parses_console_benchmark_workloads() {
        for (name, expected) in [
            ("plain-scroll", super::BenchWorkload::PlainScroll),
            ("ansi-scroll", super::BenchWorkload::AnsiScroll),
            ("ansi-scroll-query", super::BenchWorkload::AnsiScrollQuery),
        ] {
            let parsed =
                parse_args(["rterm", "bench", "--workload", name]).expect("parse workload");
            let AppCommand::Bench(options) = parsed else {
                panic!("expected bench command");
            };
            assert_eq!(options.workload, expected);
            assert_eq!(options.workload.as_str(), name);
        }
    }

    #[test]
    fn parses_local_size() {
        let parsed = parse_args(["rterm", "local", "--cols", "100", "--rows", "30"]).unwrap();

        let AppCommand::Local(options) = parsed else {
            panic!("expected local command");
        };

        let size = options.size.unwrap();
        assert_eq!(size.columns(), 100);
        assert_eq!(size.rows(), 30);
    }

    #[test]
    fn parses_custom_local_command_after_separator() {
        let parsed = parse_args(["rterm", "local", "--", "cmd.exe", "/K"]).unwrap();

        let AppCommand::Local(options) = parsed else {
            panic!("expected local command");
        };

        assert_eq!(options.command.program(), "cmd.exe");
        assert_eq!(options.command.args(), ["/K"]);
        assert_eq!(options.size, None);
        assert!(!options.mouse);
    }

    #[test]
    fn parses_local_cwd_for_initial_command() {
        let parsed = parse_args([
            "rterm",
            "local",
            "--cwd",
            "E:\\project",
            "--",
            "cmd.exe",
            "/K",
        ])
        .unwrap();

        let AppCommand::Local(options) = parsed else {
            panic!("expected local command");
        };

        assert_eq!(
            options.command.cwd(),
            Some(std::path::Path::new("E:\\project"))
        );
        assert_eq!(options.command.program(), "cmd.exe");
        assert_eq!(options.command.args(), ["/K"]);
    }

    #[test]
    fn parses_local_mouse_capture() {
        let parsed = parse_args(["rterm", "local", "--mouse"]).unwrap();

        let AppCommand::Local(options) = parsed else {
            panic!("expected local command");
        };

        assert!(options.mouse);
    }

    #[test]
    fn parses_local_preflight() {
        let parsed = parse_args(["rterm", "local", "--preflight"]).unwrap();

        let AppCommand::Local(options) = parsed else {
            panic!("expected local command");
        };

        assert!(options.console.preflight);
    }

    #[test]
    fn parses_local_metrics() {
        let parsed = parse_args(["rterm", "local", "--metrics"]).unwrap();

        let AppCommand::Local(options) = parsed else {
            panic!("expected local command");
        };

        assert!(options.console.metrics);
    }

    #[test]
    fn parses_local_metrics_json() {
        let parsed = parse_args(["rterm", "local", "--metrics-json"]).unwrap();

        let AppCommand::Local(options) = parsed else {
            panic!("expected local command");
        };

        assert!(options.console.metrics_json);
    }

    #[test]
    fn parses_local_log_path() {
        let parsed = parse_args(["rterm", "local", "--log", "session.log"]).unwrap();

        let AppCommand::Local(options) = parsed else {
            panic!("expected local command");
        };

        assert_eq!(options.log, Some(std::path::PathBuf::from("session.log")));
    }

    #[test]
    fn parses_local_osc52_policy() {
        let parsed = parse_args(["rterm", "local", "--osc52", "write"]).unwrap();

        let AppCommand::Local(options) = parsed else {
            panic!("expected local command");
        };

        assert_eq!(options.osc52_policy, super::Osc52Policy::WriteOnly);
        assert!(parse_args(["rterm", "local", "--osc52", "bad"]).is_err());
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_agent_connection_request() {
        let parsed =
            parse_args(["rterm", "ssh", "--host", "example.com", "--user", "ops"]).unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        let super::SshTarget::Direct(request) = options.target else {
            panic!("expected direct SSH target");
        };

        assert_eq!(request.config.host, "example.com");
        assert_eq!(request.config.port, 22);
        assert_eq!(request.config.username, "ops");
        assert_eq!(request.auth, SshAuthMethod::Agent);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_native_direct_backend() {
        let parsed = parse_args([
            "rterm",
            "ssh",
            "--native",
            "--host",
            "example.com",
            "--user",
            "ops",
        ])
        .unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert!(options.native);
        assert_eq!(
            options.native_host_key_policy,
            NativeHostKeyPolicy::RejectUnknown
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_native_accept_unknown_host_key_flag() {
        let parsed = parse_args([
            "rterm",
            "ssh",
            "--native",
            "--accept-unknown-host-key",
            "--host",
            "example.com",
            "--user",
            "ops",
        ])
        .unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert_eq!(
            options.native_host_key_policy,
            NativeHostKeyPolicy::AcceptUnknown
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_native_trust_on_first_use_host_key_flag() {
        let parsed = parse_args([
            "rterm",
            "ssh",
            "--native",
            "--trust-on-first-use",
            "--host",
            "example.com",
            "--user",
            "ops",
        ])
        .unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert_eq!(
            options.native_host_key_policy,
            NativeHostKeyPolicy::TrustOnFirstUse
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_openssh_config_target() {
        let parsed = parse_args(["rterm", "ssh", "--target", "prod"]).unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert_eq!(
            options.target,
            super::SshTarget::OpenSsh(super::OpenSshTarget {
                target: "prod".to_owned(),
                username: None,
                port: None,
                initial_size: super::ssh_default_terminal_size(),
                auth: SshAuthMethod::Agent
            })
        );
        assert!(options.remote_command.is_empty());
        assert!(!options.native);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_positional_openssh_target() {
        let parsed = parse_args(["rterm", "ssh", "ops@example.com", "--preflight"]).unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert_eq!(
            options.target,
            super::SshTarget::OpenSsh(super::OpenSshTarget {
                target: "ops@example.com".to_owned(),
                username: None,
                port: None,
                initial_size: super::ssh_default_terminal_size(),
                auth: SshAuthMethod::Agent
            })
        );
        assert!(options.console.preflight);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_openssh_short_connection_options() {
        let parsed = parse_args([
            "rterm",
            "ssh",
            "-p",
            "2222",
            "-l",
            "ops",
            "-i",
            "C:/Users/ops/.ssh/id_ed25519",
            "example.com",
        ])
        .unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert_eq!(
            options.target,
            super::SshTarget::OpenSsh(super::OpenSshTarget {
                target: "example.com".to_owned(),
                username: Some("ops".to_owned()),
                port: Some(2222),
                initial_size: super::ssh_default_terminal_size(),
                auth: SshAuthMethod::PrivateKey {
                    path: "C:/Users/ops/.ssh/id_ed25519".into(),
                    passphrase: None
                }
            })
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_openssh_passthrough_options() {
        let parsed = parse_args([
            "rterm",
            "ssh",
            "-F",
            "C:/Users/ops/.ssh/prod_config",
            "-o",
            "ProxyJump=bastion",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "prod",
        ])
        .unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert_eq!(
            options.openssh_args,
            [
                "-F",
                "C:/Users/ops/.ssh/prod_config",
                "-o",
                "ProxyJump=bastion",
                "-o",
                "StrictHostKeyChecking=accept-new"
            ]
        );
        assert!(matches!(options.target, super::SshTarget::OpenSsh(_)));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_openssh_jump_and_flag_passthrough_options() {
        let parsed = parse_args(["rterm", "ssh", "-J", "bastion", "-C", "-vv", "prod"]).unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert_eq!(options.openssh_args, ["-J", "bastion", "-C", "-vv"]);
        assert!(matches!(options.target, super::SshTarget::OpenSsh(_)));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_openssh_control_value_passthrough_options() {
        let parsed = parse_args([
            "rterm",
            "ssh",
            "-B",
            "Ethernet",
            "-b",
            "127.0.0.1",
            "-c",
            "aes128-gcm@openssh.com",
            "-E",
            "ssh-debug.log",
            "-e",
            "none",
            "-I",
            "pkcs11.dll",
            "-m",
            "hmac-sha2-256",
            "-O",
            "check",
            "-P",
            "release",
            "-Q",
            "cipher",
            "-S",
            "control.sock",
            "-W",
            "db.internal:5432",
            "-w",
            "0:1",
            "prod",
        ])
        .unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert_eq!(
            options.openssh_args,
            [
                "-B",
                "Ethernet",
                "-b",
                "127.0.0.1",
                "-c",
                "aes128-gcm@openssh.com",
                "-E",
                "ssh-debug.log",
                "-e",
                "none",
                "-I",
                "pkcs11.dll",
                "-m",
                "hmac-sha2-256",
                "-O",
                "check",
                "-P",
                "release",
                "-Q",
                "cipher",
                "-S",
                "control.sock",
                "-W",
                "db.internal:5432",
                "-w",
                "0:1",
            ]
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_openssh_control_flag_passthrough_options() {
        let parsed = parse_args([
            "rterm", "ssh", "-f", "-G", "-g", "-K", "-k", "-M", "-n", "-s", "-T", "-t", "-tt",
            "-X", "-x", "-Y", "-y", "prod",
        ])
        .unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert_eq!(
            options.openssh_args,
            [
                "-f", "-G", "-g", "-K", "-k", "-M", "-n", "-s", "-T", "-t", "-tt", "-X", "-x",
                "-Y", "-y"
            ]
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_positional_target_with_remote_command() {
        let parsed = parse_args([
            "rterm",
            "ssh",
            "ops@example.com",
            "--preflight",
            "uptime",
            "-p",
        ])
        .unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert_eq!(options.remote_command, ["uptime", "-p"]);
        assert!(options.console.preflight);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_explicit_target_with_remote_command_without_separator() {
        let parsed = parse_args(["rterm", "ssh", "--target", "prod", "uname", "-a"]).unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert_eq!(options.remote_command, ["uname", "-a"]);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_openssh_config_target_with_overrides() {
        let parsed = parse_args([
            "rterm",
            "ssh",
            "--target",
            "prod",
            "--user",
            "ops",
            "--port",
            "2222",
            "--cols",
            "120",
            "--rows",
            "30",
            "--key",
            "C:/Users/ops/.ssh/id_ed25519",
        ])
        .unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert_eq!(
            options.target,
            super::SshTarget::OpenSsh(super::OpenSshTarget {
                target: "prod".to_owned(),
                username: Some("ops".to_owned()),
                port: Some(2222),
                initial_size: rssh_core::TerminalSize::new(120, 30),
                auth: SshAuthMethod::PrivateKey {
                    path: "C:/Users/ops/.ssh/id_ed25519".into(),
                    passphrase: None
                }
            })
        );
        assert!(options.remote_command.is_empty());
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_remote_command_for_openssh_config_target() {
        let parsed = parse_args(["rterm", "ssh", "--target", "prod", "--", "uname", "-a"]).unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert_eq!(options.remote_command, ["uname", "-a"]);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_remote_command_for_direct_target() {
        let parsed = parse_args([
            "rterm",
            "ssh",
            "--host",
            "example.com",
            "--user",
            "ops",
            "--",
            "whoami",
        ])
        .unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        let super::SshTarget::Direct(request) = options.target else {
            panic!("expected direct SSH target");
        };

        assert_eq!(request.config.host, "example.com");
        assert_eq!(options.remote_command, ["whoami"]);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_accept_unknown_host_key_without_native() {
        let error = parse_args([
            "rterm",
            "ssh",
            "--accept-unknown-host-key",
            "--host",
            "example.com",
            "--user",
            "ops",
        ])
        .unwrap_err();

        assert!(error.contains("--accept-unknown-host-key requires --native"));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_trust_on_first_use_without_native() {
        let error = parse_args([
            "rterm",
            "ssh",
            "--trust-on-first-use",
            "--host",
            "example.com",
            "--user",
            "ops",
        ])
        .unwrap_err();

        assert!(error.contains("--trust-on-first-use requires --native"));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_conflicting_native_host_key_policies() {
        let error = parse_args([
            "rterm",
            "ssh",
            "--native",
            "--accept-unknown-host-key",
            "--trust-on-first-use",
            "--host",
            "example.com",
            "--user",
            "ops",
        ])
        .unwrap_err();

        assert!(error.contains("only one native SSH host-key policy"));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_openssh_passthrough_options_with_native_ssh() {
        let error = parse_args([
            "rterm",
            "ssh",
            "--native",
            "-o",
            "ProxyJump=bastion",
            "prod",
        ])
        .unwrap_err();

        assert!(error.contains("OpenSSH passthrough options require the OpenSSH console backend"));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_common_openssh_passthrough_options_with_native_ssh() {
        let error = parse_args(["rterm", "ssh", "--native", "-J", "bastion", "prod"]).unwrap_err();

        assert!(error.contains("OpenSSH passthrough options require the OpenSSH console backend"));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_log_path() {
        let parsed = parse_args(["rterm", "ssh", "--target", "prod", "--log", "ssh.log"]).unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert_eq!(options.log, Some(std::path::PathBuf::from("ssh.log")));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_preflight() {
        let parsed = parse_args(["rterm", "ssh", "--target", "prod", "--preflight"]).unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert!(options.console.preflight);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_metrics() {
        let parsed = parse_args(["rterm", "ssh", "--target", "prod", "--metrics"]).unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert!(options.console.metrics);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_metrics_json() {
        let parsed = parse_args(["rterm", "ssh", "--target", "prod", "--metrics-json"]).unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert!(options.console.metrics_json);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_osc52_policy() {
        let parsed = parse_args(["rterm", "ssh", "--target", "prod", "--osc52", "off"]).unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert_eq!(options.osc52_policy, super::Osc52Policy::Off);
        assert!(parse_args(["rterm", "ssh", "--target", "prod", "--osc52", "bad"]).is_err());
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_forwarding_and_no_shell_options() {
        let parsed = parse_args([
            "rterm",
            "ssh",
            "--target",
            "prod",
            "--local-forward",
            "127.0.0.1:15432:db.internal:5432",
            "--remote-forward",
            "8080:127.0.0.1:80",
            "--dynamic-forward",
            "127.0.0.1:1080",
            "--no-shell",
        ])
        .unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert_eq!(
            options.forwards,
            [
                super::SshForward::Local("127.0.0.1:15432:db.internal:5432".to_owned()),
                super::SshForward::Remote("8080:127.0.0.1:80".to_owned()),
                super::SshForward::Dynamic("127.0.0.1:1080".to_owned())
            ]
        );
        assert!(options.no_shell);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_unauthenticated_socks5_on_non_loopback_bind_addresses() {
        for spec in ["0.0.0.0:1080", "192.0.2.10:1080", "[::]:1080", "proxy:1080"] {
            let error = parse_args([
                "rterm",
                "ssh",
                "--target",
                "prod",
                "--dynamic-forward",
                spec,
            ])
            .unwrap_err();
            assert!(
                error.contains("rejects non-loopback bind address"),
                "{error}"
            );
            assert!(error.contains("no authentication"), "{error}");
        }
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn accepts_loopback_only_socks5_bind_addresses() {
        for spec in ["1080", "127.0.0.1:1080", "localhost:1080", "[::1]:1080"] {
            let parsed = parse_args([
                "rterm",
                "ssh",
                "--target",
                "prod",
                "--dynamic-forward",
                spec,
            ])
            .unwrap();
            let AppCommand::Ssh(options) = parsed else {
                panic!("expected ssh command");
            };
            assert_eq!(
                options.forwards,
                [super::SshForward::Dynamic(spec.to_owned())]
            );
        }
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_openssh_short_forwarding_options() {
        let parsed = parse_args([
            "rterm",
            "ssh",
            "-L",
            "127.0.0.1:15432:db.internal:5432",
            "-R",
            "8080:127.0.0.1:80",
            "-D",
            "127.0.0.1:1080",
            "-N",
            "prod",
        ])
        .unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        assert_eq!(
            options.forwards,
            [
                super::SshForward::Local("127.0.0.1:15432:db.internal:5432".to_owned()),
                super::SshForward::Remote("8080:127.0.0.1:80".to_owned()),
                super::SshForward::Dynamic("127.0.0.1:1080".to_owned())
            ]
        );
        assert!(options.no_shell);
        assert!(matches!(options.target, super::SshTarget::OpenSsh(_)));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_empty_ssh_forwarding_spec() {
        let error =
            parse_args(["rterm", "ssh", "--target", "prod", "--local-forward", " "]).unwrap_err();

        assert!(error.contains("--local-forward cannot be empty"));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_no_shell_with_remote_command() {
        let error = parse_args([
            "rterm",
            "ssh",
            "--target",
            "prod",
            "--no-shell",
            "--",
            "uptime",
        ])
        .unwrap_err();

        assert!(error.contains("--no-shell cannot be combined with a remote command"));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_password_connection_request() {
        let parsed = parse_args([
            "rterm",
            "ssh",
            "--host",
            "example.com",
            "--user",
            "ops",
            "--port",
            "2222",
            "--cols",
            "120",
            "--rows",
            "30",
            "--password",
        ])
        .unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        let super::SshTarget::Direct(request) = options.target else {
            panic!("expected direct SSH target");
        };

        assert_eq!(request.config.port, 2222);
        assert_eq!(request.config.initial_size.columns, 120);
        assert_eq!(request.config.initial_size.rows, 30);
        assert_eq!(request.auth, SshAuthMethod::PasswordPrompt);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_private_key_connection_request() {
        let parsed = parse_args([
            "rterm",
            "ssh",
            "--host",
            "example.com",
            "--user",
            "ops",
            "--key",
            "C:/Users/ops/.ssh/id_ed25519",
        ])
        .unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };

        let super::SshTarget::Direct(request) = options.target else {
            panic!("expected direct SSH target");
        };

        assert_eq!(
            request.auth,
            SshAuthMethod::PrivateKey {
                path: "C:/Users/ops/.ssh/id_ed25519".into(),
                passphrase: None
            }
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_sftp_openssh_config_target_with_key_and_log() {
        let parsed = parse_args([
            "rterm",
            "sftp",
            "--target",
            "prod",
            "--key",
            "C:/Users/ops/.ssh/id_ed25519",
            "--log",
            "sftp.log",
        ])
        .unwrap();

        let AppCommand::Sftp(options) = parsed else {
            panic!("expected sftp command");
        };

        assert_eq!(
            options.target,
            super::SshTarget::OpenSsh(super::OpenSshTarget {
                target: "prod".to_owned(),
                username: None,
                port: None,
                initial_size: super::ssh_default_terminal_size(),
                auth: SshAuthMethod::PrivateKey {
                    path: "C:/Users/ops/.ssh/id_ed25519".into(),
                    passphrase: None
                }
            })
        );
        assert_eq!(options.log, Some(std::path::PathBuf::from("sftp.log")));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_sftp_positional_openssh_target() {
        let parsed = parse_args(["rterm", "sftp", "ops@example.com", "--port", "2222"]).unwrap();

        let AppCommand::Sftp(options) = parsed else {
            panic!("expected sftp command");
        };

        assert_eq!(
            options.target,
            super::SshTarget::OpenSsh(super::OpenSshTarget {
                target: "ops@example.com".to_owned(),
                username: None,
                port: Some(2222),
                initial_size: super::ssh_default_terminal_size(),
                auth: SshAuthMethod::Agent
            })
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_sftp_openssh_short_connection_options() {
        let parsed = parse_args([
            "rterm",
            "sftp",
            "-P",
            "2222",
            "-i",
            "C:/Users/ops/.ssh/id_ed25519",
            "ops@example.com",
        ])
        .unwrap();

        let AppCommand::Sftp(options) = parsed else {
            panic!("expected sftp command");
        };

        assert_eq!(
            options.target,
            super::SshTarget::OpenSsh(super::OpenSshTarget {
                target: "ops@example.com".to_owned(),
                username: None,
                port: Some(2222),
                initial_size: super::ssh_default_terminal_size(),
                auth: SshAuthMethod::PrivateKey {
                    path: "C:/Users/ops/.ssh/id_ed25519".into(),
                    passphrase: None
                }
            })
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_sftp_openssh_bandwidth_limit_option() {
        let parsed = parse_args(["rterm", "sftp", "-l", "4096", "prod"]).unwrap();

        let AppCommand::Sftp(options) = parsed else {
            panic!("expected sftp command");
        };

        assert_eq!(options.openssh_args, ["-l", "4096"]);
        assert_eq!(
            options.target,
            super::SshTarget::OpenSsh(super::OpenSshTarget {
                target: "prod".to_owned(),
                username: None,
                port: None,
                initial_size: super::ssh_default_terminal_size(),
                auth: SshAuthMethod::Agent
            })
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_sftp_openssh_passthrough_options() {
        let parsed = parse_args([
            "rterm",
            "sftp",
            "-F",
            "C:/Users/ops/.ssh/prod_config",
            "-o",
            "ProxyJump=bastion",
            "prod",
        ])
        .unwrap();

        let AppCommand::Sftp(options) = parsed else {
            panic!("expected sftp command");
        };

        assert_eq!(
            options.openssh_args,
            [
                "-F",
                "C:/Users/ops/.ssh/prod_config",
                "-o",
                "ProxyJump=bastion"
            ]
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_sftp_openssh_jump_and_flag_passthrough_options() {
        let parsed = parse_args(["rterm", "sftp", "-J", "bastion", "-C", "-vv", "prod"]).unwrap();

        let AppCommand::Sftp(options) = parsed else {
            panic!("expected sftp command");
        };

        assert_eq!(options.openssh_args, ["-J", "bastion", "-C", "-vv"]);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_sftp_openssh_batch_and_transfer_passthrough_options() {
        let parsed = parse_args([
            "rterm",
            "sftp",
            "-b",
            "batch.txt",
            "-B",
            "32768",
            "-R",
            "64",
            "-D",
            "C:/tools/sftp-server.exe",
            "-S",
            "ssh",
            "-s",
            "sftp",
            "-X",
            "nrequests=128",
            "-c",
            "aes128-gcm@openssh.com",
            "prod",
        ])
        .unwrap();

        let AppCommand::Sftp(options) = parsed else {
            panic!("expected sftp command");
        };

        assert_eq!(
            options.openssh_args,
            [
                "-b",
                "batch.txt",
                "-B",
                "32768",
                "-R",
                "64",
                "-D",
                "C:/tools/sftp-server.exe",
                "-S",
                "ssh",
                "-s",
                "sftp",
                "-X",
                "nrequests=128",
                "-c",
                "aes128-gcm@openssh.com",
            ]
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_sftp_preflight() {
        let parsed = parse_args(["rterm", "sftp", "--target", "prod", "--preflight"]).unwrap();

        let AppCommand::Sftp(options) = parsed else {
            panic!("expected sftp command");
        };

        assert!(options.console.preflight);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_sftp_metrics() {
        let parsed = parse_args(["rterm", "sftp", "--target", "prod", "--metrics"]).unwrap();

        let AppCommand::Sftp(options) = parsed else {
            panic!("expected sftp command");
        };

        assert!(options.console.metrics);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_sftp_metrics_json() {
        let parsed = parse_args(["rterm", "sftp", "--target", "prod", "--metrics-json"]).unwrap();

        let AppCommand::Sftp(options) = parsed else {
            panic!("expected sftp command");
        };

        assert!(options.console.metrics_json);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_scp_upload_for_openssh_config_target() {
        let parsed = parse_args([
            "rterm",
            "scp",
            "--target",
            "prod",
            "--key",
            "C:/Users/ops/.ssh/id_ed25519",
            "--recursive",
            "--log",
            "scp.log",
            "--upload",
            "local.txt",
            "/tmp/remote.txt",
        ])
        .unwrap();

        let AppCommand::Scp(options) = parsed else {
            panic!("expected scp command");
        };

        assert_eq!(
            options.target,
            super::SshTarget::OpenSsh(super::OpenSshTarget {
                target: "prod".to_owned(),
                username: None,
                port: None,
                initial_size: super::ssh_default_terminal_size(),
                auth: SshAuthMethod::PrivateKey {
                    path: "C:/Users/ops/.ssh/id_ed25519".into(),
                    passphrase: None
                }
            })
        );
        assert_eq!(
            options.transfer,
            super::ScpTransfer::Upload {
                local: "local.txt".into(),
                remote: "/tmp/remote.txt".to_owned(),
            }
        );
        assert!(options.recursive);
        assert_eq!(options.log, Some(std::path::PathBuf::from("scp.log")));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_scp_positional_openssh_target() {
        let parsed = parse_args([
            "rterm",
            "scp",
            "ops@example.com",
            "--upload",
            "local.txt",
            "/tmp/remote.txt",
        ])
        .unwrap();

        let AppCommand::Scp(options) = parsed else {
            panic!("expected scp command");
        };

        assert_eq!(
            options.target,
            super::SshTarget::OpenSsh(super::OpenSshTarget {
                target: "ops@example.com".to_owned(),
                username: None,
                port: None,
                initial_size: super::ssh_default_terminal_size(),
                auth: SshAuthMethod::Agent
            })
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_scp_openssh_style_upload() {
        let parsed = parse_args([
            "rterm",
            "scp",
            "local.txt",
            "ops@example.com:/tmp/remote.txt",
        ])
        .unwrap();

        let AppCommand::Scp(options) = parsed else {
            panic!("expected scp command");
        };

        assert_eq!(
            options.target,
            super::SshTarget::OpenSsh(super::OpenSshTarget {
                target: "ops@example.com".to_owned(),
                username: None,
                port: None,
                initial_size: super::ssh_default_terminal_size(),
                auth: SshAuthMethod::Agent
            })
        );
        assert_eq!(
            options.transfer,
            super::ScpTransfer::Upload {
                local: "local.txt".into(),
                remote: "/tmp/remote.txt".to_owned()
            }
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_scp_openssh_style_upload_with_multiple_sources() {
        let parsed = parse_args([
            "rterm",
            "scp",
            "app.log",
            "audit.log",
            "ops@example.com:/tmp/logs/",
        ])
        .unwrap();

        let AppCommand::Scp(options) = parsed else {
            panic!("expected scp command");
        };

        assert_eq!(
            options.target,
            super::SshTarget::OpenSsh(super::OpenSshTarget {
                target: "ops@example.com".to_owned(),
                username: None,
                port: None,
                initial_size: super::ssh_default_terminal_size(),
                auth: SshAuthMethod::Agent
            })
        );
        assert_eq!(
            options.transfer,
            super::ScpTransfer::UploadMany {
                locals: vec!["app.log".into(), "audit.log".into()],
                remote: "/tmp/logs/".to_owned()
            }
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_scp_openssh_short_connection_options() {
        let parsed = parse_args([
            "rterm",
            "scp",
            "-P",
            "2222",
            "-i",
            "C:/Users/ops/.ssh/id_ed25519",
            "-r",
            "logs",
            "ops@example.com:/tmp/logs",
        ])
        .unwrap();

        let AppCommand::Scp(options) = parsed else {
            panic!("expected scp command");
        };

        assert_eq!(
            options.target,
            super::SshTarget::OpenSsh(super::OpenSshTarget {
                target: "ops@example.com".to_owned(),
                username: None,
                port: Some(2222),
                initial_size: super::ssh_default_terminal_size(),
                auth: SshAuthMethod::PrivateKey {
                    path: "C:/Users/ops/.ssh/id_ed25519".into(),
                    passphrase: None
                }
            })
        );
        assert!(options.recursive);
        assert_eq!(
            options.transfer,
            super::ScpTransfer::Upload {
                local: "logs".into(),
                remote: "/tmp/logs".to_owned()
            }
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_scp_openssh_bandwidth_limit_option() {
        let parsed = parse_args([
            "rterm",
            "scp",
            "-l",
            "4096",
            "local.txt",
            "prod:/tmp/remote.txt",
        ])
        .unwrap();

        let AppCommand::Scp(options) = parsed else {
            panic!("expected scp command");
        };

        assert_eq!(options.openssh_args, ["-l", "4096"]);
        assert_eq!(
            options.target,
            super::SshTarget::OpenSsh(super::OpenSshTarget {
                target: "prod".to_owned(),
                username: None,
                port: None,
                initial_size: super::ssh_default_terminal_size(),
                auth: SshAuthMethod::Agent
            })
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_scp_openssh_preserve_times_option() {
        let parsed =
            parse_args(["rterm", "scp", "-p", "local.txt", "prod:/tmp/remote.txt"]).unwrap();

        let AppCommand::Scp(options) = parsed else {
            panic!("expected scp command");
        };

        assert_eq!(options.openssh_args, ["-p"]);
        assert_eq!(
            options.transfer,
            super::ScpTransfer::Upload {
                local: "local.txt".into(),
                remote: "/tmp/remote.txt".to_owned()
            }
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_scp_openssh_remote_remote_and_subsystem_flags() {
        let parsed = parse_args([
            "rterm",
            "scp",
            "-R",
            "-s",
            "local.txt",
            "prod:/tmp/remote.txt",
        ])
        .unwrap();

        let AppCommand::Scp(options) = parsed else {
            panic!("expected scp command");
        };

        assert_eq!(options.openssh_args, ["-R", "-s"]);
        assert_eq!(
            options.transfer,
            super::ScpTransfer::Upload {
                local: "local.txt".into(),
                remote: "/tmp/remote.txt".to_owned()
            }
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_scp_openssh_remote_remote_flag_without_consuming_source() {
        let parsed =
            parse_args(["rterm", "scp", "-R", "local.txt", "prod:/tmp/remote.txt"]).unwrap();

        let AppCommand::Scp(options) = parsed else {
            panic!("expected scp command");
        };

        assert_eq!(options.openssh_args, ["-R"]);
        assert_eq!(
            options.transfer,
            super::ScpTransfer::Upload {
                local: "local.txt".into(),
                remote: "/tmp/remote.txt".to_owned()
            }
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_scp_openssh_subsystem_flag_without_consuming_source() {
        let parsed =
            parse_args(["rterm", "scp", "-s", "local.txt", "prod:/tmp/remote.txt"]).unwrap();

        let AppCommand::Scp(options) = parsed else {
            panic!("expected scp command");
        };

        assert_eq!(options.openssh_args, ["-s"]);
        assert_eq!(
            options.transfer,
            super::ScpTransfer::Upload {
                local: "local.txt".into(),
                remote: "/tmp/remote.txt".to_owned()
            }
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_scp_openssh_passthrough_options() {
        let parsed = parse_args([
            "rterm",
            "scp",
            "-F",
            "C:/Users/ops/.ssh/prod_config",
            "-o",
            "ProxyJump=bastion",
            "local.txt",
            "prod:/tmp/remote.txt",
        ])
        .unwrap();

        let AppCommand::Scp(options) = parsed else {
            panic!("expected scp command");
        };

        assert_eq!(
            options.openssh_args,
            [
                "-F",
                "C:/Users/ops/.ssh/prod_config",
                "-o",
                "ProxyJump=bastion"
            ]
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_scp_openssh_jump_and_flag_passthrough_options() {
        let parsed = parse_args([
            "rterm",
            "scp",
            "-J",
            "bastion",
            "-C",
            "-vv",
            "local.txt",
            "prod:/tmp/remote.txt",
        ])
        .unwrap();

        let AppCommand::Scp(options) = parsed else {
            panic!("expected scp command");
        };

        assert_eq!(options.openssh_args, ["-J", "bastion", "-C", "-vv"]);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_scp_openssh_protocol_and_transfer_passthrough_options() {
        let parsed = parse_args([
            "rterm",
            "scp",
            "-3",
            "-O",
            "-T",
            "-B",
            "-D",
            "C:/tools/sftp-server.exe",
            "-S",
            "ssh",
            "-X",
            "nrequests=128",
            "-c",
            "aes128-gcm@openssh.com",
            "local.txt",
            "prod:/tmp/remote.txt",
        ])
        .unwrap();

        let AppCommand::Scp(options) = parsed else {
            panic!("expected scp command");
        };

        assert_eq!(
            options.openssh_args,
            [
                "-3",
                "-O",
                "-T",
                "-B",
                "-D",
                "C:/tools/sftp-server.exe",
                "-S",
                "ssh",
                "-X",
                "nrequests=128",
                "-c",
                "aes128-gcm@openssh.com",
            ]
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_scp_openssh_style_download() {
        let parsed = parse_args([
            "rterm",
            "scp",
            "ops@example.com:/tmp/remote.txt",
            "local.txt",
        ])
        .unwrap();

        let AppCommand::Scp(options) = parsed else {
            panic!("expected scp command");
        };

        assert_eq!(
            options.target,
            super::SshTarget::OpenSsh(super::OpenSshTarget {
                target: "ops@example.com".to_owned(),
                username: None,
                port: None,
                initial_size: super::ssh_default_terminal_size(),
                auth: SshAuthMethod::Agent
            })
        );
        assert_eq!(
            options.transfer,
            super::ScpTransfer::Download {
                remote: "/tmp/remote.txt".to_owned(),
                local: "local.txt".into()
            }
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_scp_openssh_style_download_with_multiple_sources() {
        let parsed = parse_args([
            "rterm",
            "scp",
            "ops@example.com:/var/log/app.log",
            "ops@example.com:/var/log/audit.log",
            "logs",
        ])
        .unwrap();

        let AppCommand::Scp(options) = parsed else {
            panic!("expected scp command");
        };

        assert_eq!(
            options.target,
            super::SshTarget::OpenSsh(super::OpenSshTarget {
                target: "ops@example.com".to_owned(),
                username: None,
                port: None,
                initial_size: super::ssh_default_terminal_size(),
                auth: SshAuthMethod::Agent
            })
        );
        assert_eq!(
            options.transfer,
            super::ScpTransfer::DownloadMany {
                remotes: vec![
                    "/var/log/app.log".to_owned(),
                    "/var/log/audit.log".to_owned()
                ],
                local: "logs".into()
            }
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_scp_preflight() {
        let parsed = parse_args([
            "rterm",
            "scp",
            "--target",
            "prod",
            "--preflight",
            "--upload",
            "local.txt",
            "/tmp/remote.txt",
        ])
        .unwrap();

        let AppCommand::Scp(options) = parsed else {
            panic!("expected scp command");
        };

        assert!(options.console.preflight);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_scp_metrics() {
        let parsed = parse_args([
            "rterm",
            "scp",
            "--target",
            "prod",
            "--metrics",
            "--upload",
            "local.txt",
            "/tmp/remote.txt",
        ])
        .unwrap();

        let AppCommand::Scp(options) = parsed else {
            panic!("expected scp command");
        };

        assert!(options.console.metrics);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_scp_metrics_json() {
        let parsed = parse_args([
            "rterm",
            "scp",
            "--target",
            "prod",
            "--metrics-json",
            "--upload",
            "local.txt",
            "/tmp/remote.txt",
        ])
        .unwrap();

        let AppCommand::Scp(options) = parsed else {
            panic!("expected scp command");
        };

        assert!(options.console.metrics_json);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_ssh_password_command_line_secret() {
        let error = parse_args([
            "rterm",
            "ssh",
            "--host",
            "example.com",
            "--user",
            "ops",
            "--password",
            "secret",
        ])
        .unwrap_err();

        assert!(error.contains("unexpected ssh option: secret"));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_ssh_passphrase_command_line_secret() {
        let error = parse_args([
            "rterm",
            "ssh",
            "--host",
            "example.com",
            "--user",
            "ops",
            "--key",
            "C:/Users/ops/.ssh/id_ed25519",
            "--passphrase",
            "secret",
        ])
        .unwrap_err();

        assert!(error.contains("--passphrase is not accepted"));
    }

    #[test]
    fn help_text_does_not_request_secret_values() {
        let help = super::help_text();

        assert_eq!(help.contains("--password"), cfg!(feature = "ssh"));
        assert_eq!(help.contains("--native"), cfg!(feature = "ssh"));
        assert_eq!(
            help.contains("--accept-unknown-host-key"),
            cfg!(feature = "ssh")
        );
        assert_eq!(help.contains("--target"), cfg!(feature = "ssh"));
        assert!(help.contains("rterm console"));
        assert!(help.contains("--cwd CWD"));
        assert!(help.contains("--workspace WORKSPACE"));
        assert!(help.contains("--class CLASS"));
        assert!(help.contains("--no-auto-connect"));
        assert!(help.contains("--always-new-process"));
        assert!(help.contains("--new-tab"));
        assert!(help.contains("rterm <command> --help"));
        assert!(!help.contains("PASSWORD"));
        assert!(!help.contains("PASSPHRASE"));
    }

    #[test]
    fn help_text_describes_global_wezterm_config_options() {
        let help = super::help_text();

        assert!(help.contains("Global WezTerm configuration options (before window/start only):"));
        assert!(help.contains("-n, --skip-config"));
        assert!(help.contains("--config-file PATH"));
        assert!(help.contains("conflicts with --skip-config"));
        assert!(help.contains("--config NAME=VALUE"));
        assert!(help.contains("repeatable"));
        assert!(help.contains("may be used with --skip-config"));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_subcommand_help_before_command_separator() {
        assert_eq!(
            parse_args(["rterm", "local", "--help"]).unwrap(),
            AppCommand::Help
        );
        assert_eq!(
            parse_args(["rterm", "window", "-h"]).unwrap(),
            AppCommand::Help
        );
        assert_eq!(
            parse_args(["rterm", "ssh", "--help"]).unwrap(),
            AppCommand::Help
        );
        assert_eq!(
            parse_args(["rterm", "profile", "--help"]).unwrap(),
            AppCommand::Help
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_ssh_missing_host_or_user() {
        assert!(parse_args(["rterm", "ssh", "--user", "ops"]).is_err());
        assert!(parse_args(["rterm", "ssh", "--host", "example.com"]).is_err());
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_ssh_host_and_target_together() {
        let error = parse_args([
            "rterm",
            "ssh",
            "--host",
            "example.com",
            "--target",
            "prod",
            "--user",
            "ops",
        ])
        .unwrap_err();

        assert!(error.contains("only one of --host or --target can be selected"));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_positional_ssh_target_with_explicit_target() {
        let error =
            parse_args(["rterm", "ssh", "ops@example.com", "--target", "prod"]).unwrap_err();

        assert!(error.contains("only one SSH target can be selected"));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_ssh_native_openssh_config_target() {
        let parsed = parse_args(["rterm", "ssh", "--native", "--target", "prod"]).unwrap();

        let AppCommand::Ssh(options) = parsed else {
            panic!("expected ssh command");
        };
        assert!(options.native);
        assert!(matches!(options.target, super::SshTarget::OpenSsh(_)));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_ssh_conflicting_auth_methods() {
        assert!(
            parse_args([
                "rterm",
                "ssh",
                "--host",
                "example.com",
                "--user",
                "ops",
                "--password",
                "--key",
                "C:/Users/ops/.ssh/id_ed25519",
            ])
            .is_err()
        );
    }

    #[test]
    fn rejects_unknown_command() {
        assert!(parse_args(["rterm", "wat"]).is_err());
    }

    #[test]
    fn rejects_partial_local_size() {
        assert!(parse_args(["rterm", "local", "--cols", "100"]).is_err());
        assert!(parse_args(["rterm", "local", "--rows", "30"]).is_err());
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_gui_ssh_with_hybrid_renderer_by_default() {
        let AppCommand::Ssh(options) =
            parse_args(["rterm", "ssh", "--gui", "--target", "prod"]).unwrap()
        else {
            panic!("expected SSH command");
        };

        assert!(options.gui);
        assert!(options.native);
        assert_eq!(options.renderer, super::RendererMode::Auto);
        assert!(!options.benchmark_startup);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_gui_ssh_renderer_override_and_startup_benchmark() {
        let AppCommand::Ssh(options) = parse_args([
            "rterm",
            "ssh",
            "--target",
            "prod",
            "--gui",
            "--renderer",
            "cpu",
            "--benchmark-startup",
        ])
        .unwrap() else {
            panic!("expected SSH command");
        };

        assert!(options.gui);
        assert_eq!(options.renderer, super::RendererMode::Cpu);
        assert!(options.benchmark_startup);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_gui_ssh_forwarding_no_shell_and_openssh_passthrough() {
        for args in [
            vec![
                "rterm",
                "ssh",
                "--gui",
                "--target",
                "prod",
                "-L",
                "127.0.0.1:1:db:1",
            ],
            vec!["rterm", "ssh", "--gui", "--target", "prod", "--no-shell"],
            vec!["rterm", "ssh", "--gui", "--target", "prod", "-J", "bastion"],
        ] {
            let error = parse_args(args).unwrap_err();
            assert!(
                error.contains("GUI") || error.contains("passthrough"),
                "unexpected GUI rejection: {error}"
            );
        }
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn renderer_parser_rejects_unknown_values() {
        let error = parse_args([
            "rterm",
            "ssh",
            "--gui",
            "--target",
            "prod",
            "--renderer",
            "vulkan",
        ])
        .unwrap_err();

        assert!(error.contains("renderer"));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_renderer_without_gui_even_when_auto_is_selected() {
        let error =
            parse_args(["rterm", "ssh", "--target", "prod", "--renderer", "auto"]).unwrap_err();

        assert!(error.contains("--renderer requires --gui"));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn parses_hermetic_ssh1_diagnostic_fixture_options() {
        let AppCommand::DiagnosticGui(options) = parse_args([
            "rterm",
            "diagnostic-gui",
            "--run-id",
            "fixture-run",
            "--scenario",
            "ssh1",
            "--hold-ms",
            "250",
            "--renderer",
            "cpu",
            "--ssh-host",
            "127.0.0.1",
            "--ssh-port",
            "2222",
            "--ssh-user",
            "fixture-user",
            "--log",
            "fixture.log",
        ])
        .unwrap() else {
            panic!("expected diagnostic GUI command");
        };

        assert_eq!(options.scenario, rssh_diagnostics::Scenario::Ssh1);
        assert_eq!(options.ssh_host, Some("127.0.0.1".parse().unwrap()));
        assert_eq!(options.ssh_port, Some(2222));
        assert_eq!(options.ssh_user.as_deref(), Some("fixture-user"));
        assert_eq!(
            options.log.as_deref(),
            Some(std::path::Path::new("fixture.log"))
        );
    }

    #[test]
    fn diagnostic_gpu_backend_parses_supported_values() {
        use rssh_diagnostics::DiagnosticGpuBackend;

        for (value, expected) in [
            ("dx12", DiagnosticGpuBackend::Dx12),
            ("vulkan", DiagnosticGpuBackend::Vulkan),
            ("gl", DiagnosticGpuBackend::Gl),
        ] {
            let AppCommand::DiagnosticGui(options) = parse_args([
                "rterm",
                "diagnostic-gui",
                "--run-id",
                "backend-probe",
                "--scenario",
                "empty-window",
                "--hold-ms",
                "250",
                "--renderer",
                "gpu",
                "--gpu-backend",
                value,
            ])
            .unwrap() else {
                panic!("expected diagnostic GUI command");
            };

            assert_eq!(options.gpu_backend, Some(expected));
        }
    }

    #[test]
    fn diagnostic_gpu_backend_rejects_unsupported_value() {
        let error = parse_args([
            "rterm",
            "diagnostic-gui",
            "--run-id",
            "backend-probe",
            "--scenario",
            "empty-window",
            "--hold-ms",
            "250",
            "--gpu-backend",
            "metal",
        ])
        .unwrap_err();

        assert!(error.contains("unsupported diagnostic GPU backend"));
    }

    #[test]
    fn diagnostic_gpu_backend_rejects_cpu_renderer_combination() {
        let error = parse_args([
            "rterm",
            "diagnostic-gui",
            "--run-id",
            "backend-probe",
            "--scenario",
            "empty-window",
            "--hold-ms",
            "250",
            "--renderer",
            "cpu",
            "--gpu-backend",
            "dx12",
        ])
        .unwrap_err();

        assert!(error.contains("--gpu-backend cannot be used with --renderer cpu"));
    }

    #[test]
    fn diagnostic_attribution_stage_parses_exact_private_values_and_rejects_mixes() {
        use rssh_diagnostics::DiagnosticAttributionStage;

        for (value, expected) in [
            ("cpu-window", DiagnosticAttributionStage::CpuWindow),
            (
                "instance-surface",
                DiagnosticAttributionStage::InstanceSurface,
            ),
            ("adapter-device", DiagnosticAttributionStage::AdapterDevice),
            (
                "configured-surface-clear",
                DiagnosticAttributionStage::ConfiguredSurfaceClear,
            ),
            (
                "layer-pipelines",
                DiagnosticAttributionStage::LayerPipelines,
            ),
            (
                "fixture-font-text",
                DiagnosticAttributionStage::FixtureFontText,
            ),
            (
                "platform-font-index",
                DiagnosticAttributionStage::PlatformFontIndex,
            ),
            ("full-frame", DiagnosticAttributionStage::FullFrame),
        ] {
            let AppCommand::DiagnosticGui(options) = parse_args([
                "rterm",
                "diagnostic-gui",
                "--run-id",
                "attribution-stage",
                "--scenario",
                "empty-window",
                "--hold-ms",
                "250",
                "--attribution-stage",
                value,
            ])
            .unwrap() else {
                panic!("expected diagnostic GUI command");
            };
            assert_eq!(options.attribution_stage, Some(expected));
        }

        let invalid = parse_args([
            "rterm",
            "diagnostic-gui",
            "--run-id",
            "attribution-stage",
            "--scenario",
            "empty-window",
            "--hold-ms",
            "250",
            "--attribution-stage",
            "fixture_font_text",
        ])
        .unwrap_err();
        assert!(invalid.contains("unsupported diagnostic attribution stage"));

        let ssh1 = parse_args([
            "rterm",
            "diagnostic-gui",
            "--run-id",
            "attribution-stage",
            "--scenario",
            "ssh1",
            "--hold-ms",
            "250",
            "--attribution-stage",
            "cpu-window",
        ])
        .unwrap_err();
        assert!(ssh1.contains("attribution stage requires the empty-window scenario"));
    }

    #[test]
    fn diagnostic_font_mode_requires_a_complete_private_pair() {
        use rssh_diagnostics::{DiagnosticFontMode, DiagnosticFontSpecimen};

        let AppCommand::DiagnosticGui(options) = parse_args([
            "rterm",
            "diagnostic-gui",
            "--run-id",
            "font-proof",
            "--scenario",
            "empty-window",
            "--hold-ms",
            "250",
            "--renderer",
            "auto",
            "--font-mode",
            "lazy",
            "--font-specimen",
            "emoji",
        ])
        .expect("private font proof CLI") else {
            panic!("expected diagnostic GUI command");
        };
        assert_eq!(options.font_mode, Some(DiagnosticFontMode::Lazy));
        assert_eq!(options.font_specimen, Some(DiagnosticFontSpecimen::Emoji));

        for incomplete in [
            vec!["--font-mode", "shared"],
            vec!["--font-specimen", "cjk"],
        ] {
            let mut args = vec![
                "rterm",
                "diagnostic-gui",
                "--run-id",
                "font-proof",
                "--scenario",
                "empty-window",
                "--hold-ms",
                "250",
            ];
            args.extend(incomplete);
            let error = parse_args(args).unwrap_err();
            assert!(error.contains("must be provided together"));
        }
    }

    #[test]
    fn diagnostic_font_mode_rejects_cpu_renderer() {
        let error = parse_args([
            "rterm",
            "diagnostic-gui",
            "--run-id",
            "font-proof",
            "--scenario",
            "empty-window",
            "--hold-ms",
            "250",
            "--renderer",
            "cpu",
            "--font-mode",
            "current",
            "--font-specimen",
            "ascii",
        ])
        .unwrap_err();
        assert!(error.contains("require --renderer auto or gpu"));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn diagnostic_font_mode_rejects_ssh1_scenario() {
        let error = parse_args([
            "rterm",
            "diagnostic-gui",
            "--run-id",
            "font-proof",
            "--scenario",
            "ssh1",
            "--hold-ms",
            "250",
            "--renderer",
            "auto",
            "--font-mode",
            "lazy",
            "--font-specimen",
            "ascii",
            "--ssh-host",
            "127.0.0.1",
            "--ssh-port",
            "22",
            "--ssh-user",
            "fixture-user",
        ])
        .unwrap_err();
        assert!(error.contains("font proof requires the empty-window scenario"));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn rejects_non_loopback_or_incomplete_ssh1_diagnostic_fixture() {
        let non_loopback = parse_args([
            "rterm",
            "diagnostic-gui",
            "--run-id",
            "fixture-run",
            "--scenario",
            "ssh1",
            "--hold-ms",
            "250",
            "--ssh-host",
            "203.0.113.1",
            "--ssh-port",
            "22",
            "--ssh-user",
            "fixture-user",
        ])
        .unwrap_err();
        assert!(non_loopback.contains("loopback"));

        let incomplete = parse_args([
            "rterm",
            "diagnostic-gui",
            "--run-id",
            "fixture-run",
            "--scenario",
            "ssh1",
            "--hold-ms",
            "250",
        ])
        .unwrap_err();
        assert!(incomplete.contains("requires --ssh-host"));
    }
}
