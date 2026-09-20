use std::{collections::BTreeMap, error::Error, fs, io};

use serde::{Deserialize, Serialize};

use crate::cli::{
    self, AppCommand, ProfileCheckOptions, ProfileInitOptions, ProfileListOptions, ProfileOptions,
    ProfileShowOptions,
};

#[cfg(feature = "ssh")]
const PROFILE_TEMPLATE: &str = include_str!("../../../examples/rssh-profiles.toml");
#[cfg(not(feature = "ssh"))]
const PROFILE_TEMPLATE: &str = include_str!("../../../examples/rterm-profiles-local.toml");
const PROFILE_NAME_ENV: &str = "RSSH_PROFILE";

#[derive(Deserialize)]
struct ProfileDocument {
    profiles: BTreeMap<String, ProfileDefinition>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileDefinition {
    kind: String,
    host: Option<String>,
    target: Option<String>,
    user: Option<String>,
    port: Option<u16>,
    cols: Option<u16>,
    rows: Option<u16>,
    frames: Option<u64>,
    mouse: Option<bool>,
    metrics: Option<ProfileMetrics>,
    preflight: Option<bool>,
    native: Option<bool>,
    gui: Option<bool>,
    renderer: Option<String>,
    benchmark_startup: Option<bool>,
    host_key_policy: Option<String>,
    osc52: Option<String>,
    log: Option<String>,
    command: Option<Vec<String>>,
    auth: Option<String>,
    key: Option<String>,
    remote_command: Option<Vec<String>>,
    local_forward: Option<Vec<String>>,
    remote_forward: Option<Vec<String>>,
    dynamic_forward: Option<Vec<String>>,
    no_shell: Option<bool>,
    recursive: Option<bool>,
    upload: Option<Vec<String>>,
    download: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ProfileMetrics {
    Enabled(bool),
    Format(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct ProfileSummary {
    pub name: String,
    pub kind: String,
}

#[derive(Serialize)]
struct ProfileListJsonEntry {
    name: String,
    kind: String,
    command: String,
    argv: Vec<String>,
}

#[derive(Serialize)]
struct ProfileCheckJsonReport {
    ok: bool,
    profiles: Vec<ProfileCheckJsonEntry>,
}

#[derive(Serialize)]
struct ProfileCheckJsonEntry {
    name: String,
    kind: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub fn load_command(options: &ProfileOptions) -> Result<AppCommand, Box<dyn Error>> {
    let contents = fs::read_to_string(&options.file)?;
    command_from_toml(&options.name, &contents)
        .map(|command| command_with_profile_name(command, &options.name))
        .map_err(profile_error)
}

pub fn profile_command_line(options: &ProfileShowOptions) -> Result<String, Box<dyn Error>> {
    let contents = fs::read_to_string(&options.file)?;
    let args = args_from_toml(&options.name, &contents).map_err(profile_error)?;
    Ok(command_line_from_args(&args))
}

pub fn print_profile_show(options: &ProfileShowOptions) -> Result<(), Box<dyn Error>> {
    if options.json {
        println!("{}", profile_show_json(options)?);
        return Ok(());
    }

    println!("{}", profile_command_line(options)?);

    Ok(())
}

pub fn profile_show_json(options: &ProfileShowOptions) -> Result<String, Box<dyn Error>> {
    let contents = fs::read_to_string(&options.file)?;
    let document = toml::from_str::<ProfileDocument>(&contents)
        .map_err(|error| profile_error(error.to_string()))?;
    let profile = document
        .profiles
        .get(&options.name)
        .ok_or_else(|| profile_error(format!("profile not found: {}", options.name)))?;
    let argv = profile.to_args().map_err(profile_error)?;
    let entry = ProfileListJsonEntry {
        name: options.name.clone(),
        kind: profile.kind.clone(),
        command: command_line_from_args(&argv),
        argv,
    };

    Ok(serde_json::to_string(&entry)?)
}

pub fn list_profiles(options: &ProfileListOptions) -> Result<Vec<ProfileSummary>, Box<dyn Error>> {
    let contents = fs::read_to_string(&options.file)?;
    summaries_from_toml(&contents).map_err(profile_error)
}

pub fn check_profiles(options: &ProfileCheckOptions) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(&options.file)?;
    validate_profiles_from_toml(&contents).map_err(profile_error)
}

pub fn print_profile_check(options: &ProfileCheckOptions) -> Result<(), Box<dyn Error>> {
    if options.json {
        println!("{}", profile_check_json(options)?);
        check_profiles(options)?;
        return Ok(());
    }

    check_profiles(options)?;
    println!("profile check ok");

    Ok(())
}

pub fn profile_check_json(options: &ProfileCheckOptions) -> Result<String, Box<dyn Error>> {
    let contents = fs::read_to_string(&options.file)?;
    let document = toml::from_str::<ProfileDocument>(&contents)
        .map_err(|error| profile_error(error.to_string()))?;
    let profiles = document
        .profiles
        .iter()
        .map(|(name, profile)| match profile.to_command() {
            Ok(_) => ProfileCheckJsonEntry {
                name: name.clone(),
                kind: profile.kind.clone(),
                ok: true,
                error: None,
            },
            Err(error) => ProfileCheckJsonEntry {
                name: name.clone(),
                kind: profile.kind.clone(),
                ok: false,
                error: Some(error),
            },
        })
        .collect::<Vec<_>>();
    let report = ProfileCheckJsonReport {
        ok: profiles.iter().all(|profile| profile.ok),
        profiles,
    };

    Ok(serde_json::to_string(&report)?)
}

pub fn init_profile_file(options: &ProfileInitOptions) -> Result<(), Box<dyn Error>> {
    if options.file.exists() && !options.force {
        return Err(profile_error(format!(
            "profile file already exists: {}; use --force to overwrite",
            options.file.display()
        )));
    }

    fs::write(&options.file, PROFILE_TEMPLATE)?;
    Ok(())
}

pub fn print_profile_init(options: &ProfileInitOptions) -> Result<(), Box<dyn Error>> {
    init_profile_file(options)?;
    println!("profile file initialized: {}", options.file.display());

    Ok(())
}

pub fn print_profile_list(options: &ProfileListOptions) -> Result<(), Box<dyn Error>> {
    if options.json {
        println!("{}", profile_list_json(options)?);
        return Ok(());
    }

    for line in profile_list_lines(options)? {
        println!("{line}");
    }

    Ok(())
}

pub fn profile_list_json(options: &ProfileListOptions) -> Result<String, Box<dyn Error>> {
    let contents = fs::read_to_string(&options.file)?;
    let profiles = summaries_from_toml(&contents).map_err(profile_error)?;
    let entries = profiles
        .into_iter()
        .map(|profile| {
            let argv = args_from_toml(&profile.name, &contents).map_err(profile_error)?;
            Ok(ProfileListJsonEntry {
                name: profile.name,
                kind: profile.kind,
                command: command_line_from_args(&argv),
                argv,
            })
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;

    Ok(serde_json::to_string(&entries)?)
}

pub fn profile_list_lines(options: &ProfileListOptions) -> Result<Vec<String>, Box<dyn Error>> {
    if !options.verbose {
        return Ok(list_profiles(options)?
            .into_iter()
            .map(|profile| format!("{}\t{}", profile.name, profile.kind))
            .collect());
    }

    let contents = fs::read_to_string(&options.file)?;
    let profiles = summaries_from_toml(&contents).map_err(profile_error)?;

    profiles
        .into_iter()
        .map(|profile| {
            let args = args_from_toml(&profile.name, &contents).map_err(profile_error)?;
            Ok(format!(
                "{}\t{}\t{}",
                profile.name,
                profile.kind,
                command_line_from_args(&args)
            ))
        })
        .collect()
}

fn command_from_toml(name: &str, contents: &str) -> Result<AppCommand, String> {
    cli::parse_args(args_from_toml(name, contents)?)
}

fn command_with_profile_name(command: AppCommand, name: &str) -> AppCommand {
    match command {
        AppCommand::Window(mut options) => {
            options.command = options.command.with_env(PROFILE_NAME_ENV, name);
            AppCommand::Window(options)
        }
        command => command,
    }
}

fn args_from_toml(name: &str, contents: &str) -> Result<Vec<String>, String> {
    let document =
        toml::from_str::<ProfileDocument>(contents).map_err(|error| error.to_string())?;
    let profile = document
        .profiles
        .get(name)
        .ok_or_else(|| format!("profile not found: {name}"))?;

    profile.to_args()
}

fn command_line_from_args(args: &[String]) -> String {
    args.iter()
        .map(|argument| quote_command_argument(argument))
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote_command_argument(argument: &str) -> String {
    if !argument.is_empty()
        && !argument
            .chars()
            .any(|character| character.is_whitespace() || matches!(character, '"' | '\\'))
    {
        return argument.to_owned();
    }

    let escaped = argument.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn summaries_from_toml(contents: &str) -> Result<Vec<ProfileSummary>, String> {
    let document =
        toml::from_str::<ProfileDocument>(contents).map_err(|error| error.to_string())?;

    Ok(document
        .profiles
        .into_iter()
        .map(|(name, profile)| ProfileSummary {
            name,
            kind: profile.kind,
        })
        .collect())
}

fn validate_profiles_from_toml(contents: &str) -> Result<(), String> {
    let document =
        toml::from_str::<ProfileDocument>(contents).map_err(|error| error.to_string())?;
    let errors = document
        .profiles
        .iter()
        .filter_map(|(name, profile)| {
            profile
                .to_command()
                .err()
                .map(|error| format!("{name} ({}): {error}", profile.kind))
        })
        .collect::<Vec<_>>();

    if errors.is_empty() {
        return Ok(());
    }

    Err(format!("profile check failed: {}", errors.join("; ")))
}

impl ProfileDefinition {
    fn to_command(&self) -> Result<AppCommand, String> {
        cli::parse_args(self.to_args()?)
    }

    fn to_args(&self) -> Result<Vec<String>, String> {
        let mut args = vec!["rterm".to_owned()];
        if self.kind != "ssh"
            && (self.gui.is_some()
                || self.renderer.is_some()
                || self.benchmark_startup.is_some()
                || self.host_key_policy.is_some())
        {
            return Err(
                "gui, renderer, benchmark_startup, and host_key_policy are valid only for ssh profiles"
                    .to_owned(),
            );
        }
        match self.kind.as_str() {
            "local" => self.append_local_args(&mut args)?,
            "scp" => self.append_scp_args(&mut args)?,
            "sftp" => self.append_sftp_args(&mut args)?,
            "ssh" => self.append_ssh_args(&mut args)?,
            "window" => self.append_window_args(&mut args)?,
            value => return Err(format!("invalid profile kind: {value}")),
        }

        Ok(args)
    }

    fn append_local_args(&self, args: &mut Vec<String>) -> Result<(), String> {
        args.push("local".to_owned());
        self.append_preflight(args);
        self.append_console_metrics(args)?;
        append_dimensions(args, self.cols, self.rows);
        if self.mouse.unwrap_or(false) {
            args.push("--mouse".to_owned());
        }
        append_optional(args, "--osc52", self.osc52.as_ref());
        append_optional(args, "--log", self.log.as_ref());
        append_command(args, self.command.as_ref(), "local command")?;
        Ok(())
    }

    fn append_ssh_args(&self, args: &mut Vec<String>) -> Result<(), String> {
        args.push("ssh".to_owned());
        append_optional(args, "--host", self.host.as_ref());
        append_optional(args, "--target", self.target.as_ref());
        self.append_preflight(args);
        self.append_console_metrics(args)?;
        self.append_native_ssh_args(args)?;
        if self.gui.unwrap_or(false) {
            args.push("--gui".to_owned());
        }
        if let Some(renderer) = self.renderer.as_deref() {
            args.push("--renderer".to_owned());
            args.push(renderer.to_owned());
        }
        if self.benchmark_startup.unwrap_or(false) {
            args.push("--benchmark-startup".to_owned());
        }
        append_optional(args, "--user", self.user.as_ref());
        append_optional_u16(args, "--port", self.port);
        append_dimensions(args, self.cols, self.rows);
        append_auth_args(args, self.auth.as_deref(), self.key.as_ref())?;
        append_optional(args, "--osc52", self.osc52.as_ref());
        append_optional(args, "--log", self.log.as_ref());
        append_forwards(args, "--local-forward", self.local_forward.as_ref());
        append_forwards(args, "--remote-forward", self.remote_forward.as_ref());
        append_forwards(args, "--dynamic-forward", self.dynamic_forward.as_ref());
        if self.no_shell.unwrap_or(false) {
            args.push("--no-shell".to_owned());
        }
        append_command(args, self.remote_command.as_ref(), "remote command")?;
        Ok(())
    }

    fn append_sftp_args(&self, args: &mut Vec<String>) -> Result<(), String> {
        args.push("sftp".to_owned());
        append_optional(args, "--host", self.host.as_ref());
        append_optional(args, "--target", self.target.as_ref());
        self.append_preflight(args);
        self.append_console_metrics(args)?;
        append_optional(args, "--user", self.user.as_ref());
        append_optional_u16(args, "--port", self.port);
        append_dimensions(args, self.cols, self.rows);
        append_auth_args(args, self.auth.as_deref(), self.key.as_ref())?;
        append_optional(args, "--log", self.log.as_ref());
        Ok(())
    }

    fn append_scp_args(&self, args: &mut Vec<String>) -> Result<(), String> {
        args.push("scp".to_owned());
        append_optional(args, "--host", self.host.as_ref());
        append_optional(args, "--target", self.target.as_ref());
        self.append_preflight(args);
        self.append_console_metrics(args)?;
        append_optional(args, "--user", self.user.as_ref());
        append_optional_u16(args, "--port", self.port);
        append_dimensions(args, self.cols, self.rows);
        append_auth_args(args, self.auth.as_deref(), self.key.as_ref())?;
        if self.recursive.unwrap_or(false) {
            args.push("--recursive".to_owned());
        }
        append_optional(args, "--log", self.log.as_ref());
        append_transfer(args, "--upload", self.upload.as_ref(), "scp upload")?;
        append_transfer(args, "--download", self.download.as_ref(), "scp download")?;
        Ok(())
    }

    fn append_preflight(&self, args: &mut Vec<String>) {
        if self.preflight.unwrap_or(false) {
            args.push("--preflight".to_owned());
        }
    }

    fn append_console_metrics(&self, args: &mut Vec<String>) -> Result<(), String> {
        if let Some(flag) = self.metrics_flag()? {
            args.push(flag.to_owned());
        }

        Ok(())
    }

    fn append_native_ssh_args(&self, args: &mut Vec<String>) -> Result<(), String> {
        if self.native.unwrap_or(false) {
            args.push("--native".to_owned());
        }

        match self.host_key_policy.as_deref() {
            None | Some("reject-unknown") => {}
            Some("prompt") if self.gui.unwrap_or(false) => {}
            Some("trust-on-first-use") => args.push("--trust-on-first-use".to_owned()),
            Some("accept-unknown") => args.push("--accept-unknown-host-key".to_owned()),
            Some(value) => {
                return Err(format!(
                    "invalid host_key_policy: {value}; expected \"prompt\", \"reject-unknown\", \"trust-on-first-use\", or \"accept-unknown\""
                ));
            }
        }

        Ok(())
    }

    fn metrics_flag(&self) -> Result<Option<&'static str>, String> {
        match self.metrics.as_ref() {
            None | Some(ProfileMetrics::Enabled(false)) => Ok(None),
            Some(ProfileMetrics::Enabled(true)) => Ok(Some("--metrics")),
            Some(ProfileMetrics::Format(format)) if format == "text" => Ok(Some("--metrics")),
            Some(ProfileMetrics::Format(format)) if format == "json" => Ok(Some("--metrics-json")),
            Some(ProfileMetrics::Format(format)) => Err(format!(
                "invalid metrics format: {format}; expected true, false, \"text\", or \"json\""
            )),
        }
    }

    fn append_window_args(&self, args: &mut Vec<String>) -> Result<(), String> {
        args.push("window".to_owned());
        append_optional_u64(args, "--frames", self.frames);
        append_optional(args, "--osc52", self.osc52.as_ref());
        match self.metrics_flag()? {
            Some("--metrics") => args.push("--metrics".to_owned()),
            Some("--metrics-json") => args.push("--metrics-json".to_owned()),
            Some(_) => unreachable!("validated metrics flag"),
            None => {}
        }
        append_optional(args, "--log", self.log.as_ref());
        append_command(args, self.command.as_ref(), "window command")?;
        Ok(())
    }
}

fn append_optional(args: &mut Vec<String>, name: &str, value: Option<&String>) {
    if let Some(value) = value {
        args.push(name.to_owned());
        args.push(value.clone());
    }
}

fn append_optional_u16(args: &mut Vec<String>, name: &str, value: Option<u16>) {
    if let Some(value) = value {
        args.push(name.to_owned());
        args.push(value.to_string());
    }
}

fn append_optional_u64(args: &mut Vec<String>, name: &str, value: Option<u64>) {
    if let Some(value) = value {
        args.push(name.to_owned());
        args.push(value.to_string());
    }
}

fn append_dimensions(args: &mut Vec<String>, cols: Option<u16>, rows: Option<u16>) {
    append_optional_u16(args, "--cols", cols);
    append_optional_u16(args, "--rows", rows);
}

fn append_forwards(args: &mut Vec<String>, name: &str, values: Option<&Vec<String>>) {
    if let Some(values) = values {
        for value in values {
            args.push(name.to_owned());
            args.push(value.clone());
        }
    }
}

fn append_transfer(
    args: &mut Vec<String>,
    name: &str,
    transfer: Option<&Vec<String>>,
    label: &str,
) -> Result<(), String> {
    let Some(transfer) = transfer else {
        return Ok(());
    };
    if transfer.len() != 2 {
        return Err(format!("{label} requires exactly two paths"));
    }

    args.push(name.to_owned());
    args.extend(transfer.iter().cloned());
    Ok(())
}

fn append_command(
    args: &mut Vec<String>,
    command: Option<&Vec<String>>,
    label: &str,
) -> Result<(), String> {
    let Some(command) = command else {
        return Ok(());
    };
    if command.is_empty() {
        return Err(format!("{label} cannot be empty"));
    }

    args.push("--".to_owned());
    args.extend(command.iter().cloned());
    Ok(())
}

fn append_auth_args(
    args: &mut Vec<String>,
    auth: Option<&str>,
    key: Option<&String>,
) -> Result<(), String> {
    match (auth, key) {
        (None, None) => Ok(()),
        (None | Some("key"), Some(key)) => {
            args.push("--key".to_owned());
            args.push(key.clone());
            Ok(())
        }
        (Some("agent"), None) => {
            args.push("--agent".to_owned());
            Ok(())
        }
        (Some("password"), None) => {
            args.push("--password".to_owned());
            Ok(())
        }
        (Some("key"), None) => Err("profile ssh auth = \"key\" requires key".to_owned()),
        (Some("agent" | "password"), Some(_)) => {
            Err("profile ssh key cannot be combined with agent or password auth".to_owned())
        }
        (Some(value), _) => Err(format!("invalid profile ssh auth: {value}")),
    }
}

fn profile_error(message: String) -> Box<dyn Error> {
    Box::new(io::Error::new(io::ErrorKind::InvalidData, message))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    #[cfg(feature = "ssh")]
    use rssh_core::TerminalSize;
    #[cfg(feature = "ssh")]
    use rssh_ssh::SshAuthMethod;

    use crate::cli::{
        AppCommand, LocalOptions, ProfileOptions, WindowConfigOptions, WindowOptions,
    };
    #[cfg(feature = "ssh")]
    use crate::cli::{NativeHostKeyPolicy, OpenSshTarget, SftpOptions, SshForward, SshTarget};

    fn temp_profile_file(name: &str, contents: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("rssh-{name}-{}.toml", std::process::id()));
        fs::write(&path, contents).unwrap();
        path
    }

    fn remove_file(path: &Path) {
        let _ = fs::remove_file(path);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn loads_ssh_profile_from_toml_file() {
        let file = temp_profile_file(
            "ssh-profile",
            r#"
[profiles.prod]
kind = "ssh"
target = "prod"
user = "ops"
port = 2222
auth = "agent"
cols = 120
rows = 30
osc52 = "off"
remote_command = ["uname", "-a"]
local_forward = ["127.0.0.1:15432:db.internal:5432"]
dynamic_forward = ["127.0.0.1:1080"]
"#,
        );

        let command = super::load_command(&ProfileOptions {
            name: "prod".to_owned(),
            file: file.clone(),
        })
        .unwrap();

        remove_file(&file);

        assert_eq!(
            command,
            AppCommand::Ssh(crate::cli::SshOptions {
                target: SshTarget::OpenSsh(OpenSshTarget {
                    target: "prod".to_owned(),
                    username: Some("ops".to_owned()),
                    port: Some(2222),
                    initial_size: TerminalSize::new(120, 30),
                    auth: SshAuthMethod::Agent,
                }),
                remote_command: vec!["uname".to_owned(), "-a".to_owned()],
                forwards: vec![
                    SshForward::Local("127.0.0.1:15432:db.internal:5432".to_owned()),
                    SshForward::Dynamic("127.0.0.1:1080".to_owned()),
                ],
                openssh_args: Vec::new(),
                no_shell: false,
                native: false,
                gui: false,
                renderer: crate::cli::RendererMode::Auto,
                benchmark_startup: false,
                native_host_key_policy: NativeHostKeyPolicy::RejectUnknown,
                console: crate::cli::ConsoleOptions::default(),
                osc52_policy: crate::cli::Osc52Policy::Off,
                log: None,
            })
        );
    }

    #[test]
    fn loads_local_profile_from_toml_file() {
        let file = temp_profile_file(
            "local-profile",
            r#"
[profiles.dev]
kind = "local"
cols = 100
rows = 32
mouse = true
osc52 = "write"
log = "dev.log"
command = ["pwsh", "-NoLogo"]
"#,
        );

        let command = super::load_command(&ProfileOptions {
            name: "dev".to_owned(),
            file: file.clone(),
        })
        .unwrap();

        remove_file(&file);

        assert_eq!(
            command,
            AppCommand::Local(LocalOptions {
                command: rssh_pty::PtyCommand::new("pwsh").with_args(["-NoLogo"]),
                size: Some(rssh_pty::PtySize::try_new(100, 32).unwrap()),
                mouse: true,
                console: crate::cli::ConsoleOptions::default(),
                osc52_policy: crate::cli::Osc52Policy::WriteOnly,
                log: Some(PathBuf::from("dev.log")),
            })
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn loads_sftp_profile_from_toml_file() {
        let file = temp_profile_file(
            "sftp-profile",
            r#"
[profiles.files]
kind = "sftp"
target = "prod"
user = "ops"
port = 2222
auth = "key"
key = "C:/Users/ops/.ssh/id_ed25519"
log = "sftp.log"
"#,
        );

        let command = super::load_command(&ProfileOptions {
            name: "files".to_owned(),
            file: file.clone(),
        })
        .unwrap();

        remove_file(&file);

        assert_eq!(
            command,
            AppCommand::Sftp(SftpOptions {
                target: SshTarget::OpenSsh(OpenSshTarget {
                    target: "prod".to_owned(),
                    username: Some("ops".to_owned()),
                    port: Some(2222),
                    initial_size: TerminalSize::new(80, 24),
                    auth: SshAuthMethod::PrivateKey {
                        path: "C:/Users/ops/.ssh/id_ed25519".into(),
                        passphrase: None,
                    },
                }),
                openssh_args: Vec::new(),
                console: crate::cli::ConsoleOptions::default(),
                log: Some(PathBuf::from("sftp.log")),
            })
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn loads_scp_upload_profile_from_toml_file() {
        let file = temp_profile_file(
            "scp-profile",
            r#"
[profiles.upload]
kind = "scp"
target = "prod"
auth = "agent"
recursive = true
upload = ["local", "/tmp/remote"]
log = "scp.log"
"#,
        );

        let command = super::load_command(&ProfileOptions {
            name: "upload".to_owned(),
            file: file.clone(),
        })
        .unwrap();

        remove_file(&file);

        assert_eq!(
            command,
            AppCommand::Scp(crate::cli::ScpOptions {
                target: SshTarget::OpenSsh(OpenSshTarget {
                    target: "prod".to_owned(),
                    username: None,
                    port: None,
                    initial_size: TerminalSize::new(80, 24),
                    auth: SshAuthMethod::Agent,
                }),
                openssh_args: Vec::new(),
                transfer: crate::cli::ScpTransfer::Upload {
                    local: "local".into(),
                    remote: "/tmp/remote".to_owned(),
                },
                recursive: true,
                console: crate::cli::ConsoleOptions::default(),
                log: Some(PathBuf::from("scp.log")),
            })
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn console_profiles_can_enable_preflight() {
        let contents = r#"
[profiles.local-dev]
kind = "local"
preflight = true

[profiles.prod-shell]
kind = "ssh"
target = "prod"
preflight = true

[profiles.prod-files]
kind = "sftp"
target = "prod"
preflight = true

[profiles.prod-upload]
kind = "scp"
target = "prod"
preflight = true
upload = ["local", "/tmp/remote"]
"#;

        assert_eq!(
            super::args_from_toml("local-dev", contents).unwrap(),
            ["rterm", "local", "--preflight"]
        );
        assert_eq!(
            super::args_from_toml("prod-shell", contents).unwrap(),
            ["rterm", "ssh", "--target", "prod", "--preflight"]
        );
        assert_eq!(
            super::args_from_toml("prod-files", contents).unwrap(),
            ["rterm", "sftp", "--target", "prod", "--preflight"]
        );
        assert_eq!(
            super::args_from_toml("prod-upload", contents).unwrap(),
            [
                "rterm",
                "scp",
                "--target",
                "prod",
                "--preflight",
                "--upload",
                "local",
                "/tmp/remote"
            ]
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn console_profiles_can_enable_metrics() {
        let contents = r#"
[profiles.local-dev]
kind = "local"
metrics = true

[profiles.prod-shell]
kind = "ssh"
target = "prod"
metrics = true

[profiles.prod-files]
kind = "sftp"
target = "prod"
metrics = true

[profiles.prod-upload]
kind = "scp"
target = "prod"
metrics = true
upload = ["local", "/tmp/remote"]
"#;

        assert_eq!(
            super::args_from_toml("local-dev", contents).unwrap(),
            ["rterm", "local", "--metrics"]
        );
        assert_eq!(
            super::args_from_toml("prod-shell", contents).unwrap(),
            ["rterm", "ssh", "--target", "prod", "--metrics"]
        );
        assert_eq!(
            super::args_from_toml("prod-files", contents).unwrap(),
            ["rterm", "sftp", "--target", "prod", "--metrics"]
        );
        assert_eq!(
            super::args_from_toml("prod-upload", contents).unwrap(),
            [
                "rterm",
                "scp",
                "--target",
                "prod",
                "--metrics",
                "--upload",
                "local",
                "/tmp/remote"
            ]
        );
    }

    #[test]
    fn console_profiles_can_enable_json_metrics() {
        let contents = r#"
[profiles.local-dev]
kind = "local"
metrics = "json"

[profiles.prod-shell]
kind = "ssh"
target = "prod"
metrics = "json"
"#;

        assert_eq!(
            super::args_from_toml("local-dev", contents).unwrap(),
            ["rterm", "local", "--metrics-json"]
        );
        assert_eq!(
            super::args_from_toml("prod-shell", contents).unwrap(),
            ["rterm", "ssh", "--target", "prod", "--metrics-json"]
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn ssh_profiles_can_select_native_backend_and_host_key_policy() {
        let contents = r#"
[profiles.native-prod]
kind = "ssh"
target = "prod"
native = true
host_key_policy = "trust-on-first-use"
auth = "agent"
metrics = "json"
"#;

        assert_eq!(
            super::args_from_toml("native-prod", contents).unwrap(),
            [
                "rterm",
                "ssh",
                "--target",
                "prod",
                "--metrics-json",
                "--native",
                "--trust-on-first-use",
                "--agent"
            ]
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn ssh_profiles_reject_invalid_host_key_policy() {
        let contents = r#"
[profiles.native-prod]
kind = "ssh"
target = "prod"
native = true
host_key_policy = "trust-everything"
"#;

        let error = super::args_from_toml("native-prod", contents).unwrap_err();

        assert!(error.contains("invalid host_key_policy: trust-everything"));
    }

    #[test]
    fn loads_window_profile_from_toml_file() {
        let file = temp_profile_file(
            "window-profile",
            r#"
[profiles.ops-window]
kind = "window"
frames = 120
metrics = true
osc52 = "write"
log = "window.log"
command = ["cmd.exe", "/K", "echo", "window-profile-smoke"]
"#,
        );

        let command = super::load_command(&ProfileOptions {
            name: "ops-window".to_owned(),
            file: file.clone(),
        })
        .unwrap();

        remove_file(&file);

        assert_eq!(
            command,
            AppCommand::Window(WindowOptions {
                config: WindowConfigOptions::default(),
                frame_limit: Some(120),
                workspace: None,
                window_class: None,
                position: None,
                osc52_policy: crate::cli::Osc52Policy::WriteOnly,
                metrics: true,
                metrics_json: false,
                state: false,
                state_json: false,
                command: rssh_pty::PtyCommand::new("cmd.exe")
                    .with_args(["/K", "echo", "window-profile-smoke"])
                    .with_env("RSSH_PROFILE", "ops-window"),
                log: Some(PathBuf::from("window.log")),
            })
        );
    }

    #[test]
    fn window_profile_command_carries_profile_name_environment() {
        let file = temp_profile_file(
            "window-profile-env",
            r#"
[profiles.ops-window]
kind = "window"
command = ["cmd.exe", "/K", "echo", "window-profile-smoke"]
"#,
        );

        let command = super::load_command(&ProfileOptions {
            name: "ops-window".to_owned(),
            file: file.clone(),
        })
        .unwrap();

        remove_file(&file);

        let AppCommand::Window(options) = command else {
            panic!("expected window profile command");
        };

        assert_eq!(
            options.command.env_value("RSSH_PROFILE"),
            Some("ops-window")
        );
    }

    #[test]
    fn window_profiles_can_enable_json_metrics() {
        let file = temp_profile_file(
            "window-json-metrics-profile",
            r#"
[profiles.ops-window]
kind = "window"
frames = 3
metrics = "json"
"#,
        );

        let command = super::load_command(&ProfileOptions {
            name: "ops-window".to_owned(),
            file: file.clone(),
        })
        .unwrap();

        remove_file(&file);

        assert_eq!(
            command,
            AppCommand::Window(WindowOptions {
                config: WindowConfigOptions::default(),
                frame_limit: Some(3),
                workspace: None,
                window_class: None,
                position: None,
                osc52_policy: crate::cli::Osc52Policy::WriteOnly,
                metrics: false,
                metrics_json: true,
                state: false,
                state_json: false,
                command: rssh_pty::PtyCommand::default_shell()
                    .with_env("RSSH_PROFILE", "ops-window"),
                log: None,
            })
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn lists_profile_names_and_kinds_from_toml_file() {
        let file = temp_profile_file(
            "list-profile",
            r#"
[profiles.prod-shell]
kind = "ssh"
target = "prod"
auth = "agent"

[profiles.local-smoke]
kind = "local"
command = ["pwsh", "-NoLogo"]
"#,
        );

        let profiles = super::list_profiles(&crate::cli::ProfileListOptions {
            file: file.clone(),
            verbose: false,
            json: false,
        })
        .unwrap();

        remove_file(&file);

        assert_eq!(
            profiles,
            vec![
                super::ProfileSummary {
                    name: "local-smoke".to_owned(),
                    kind: "local".to_owned(),
                },
                super::ProfileSummary {
                    name: "prod-shell".to_owned(),
                    kind: "ssh".to_owned(),
                },
            ]
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn lists_verbose_profile_lines_with_resolved_commands() {
        let file = temp_profile_file(
            "verbose-list-profile",
            r#"
[profiles.prod-shell]
kind = "ssh"
target = "prod"
auth = "agent"

[profiles.local-smoke]
kind = "local"
command = ["pwsh", "-NoLogo"]
"#,
        );

        let lines = super::profile_list_lines(&crate::cli::ProfileListOptions {
            file: file.clone(),
            verbose: true,
            json: false,
        })
        .unwrap();

        remove_file(&file);

        assert_eq!(
            lines,
            vec![
                "local-smoke\tlocal\trterm local -- pwsh -NoLogo".to_owned(),
                "prod-shell\tssh\trterm ssh --target prod --agent".to_owned(),
            ]
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn lists_profiles_as_json_with_resolved_commands() {
        let file = temp_profile_file(
            "json-list-profile",
            r#"
[profiles.prod-shell]
kind = "ssh"
target = "prod"
auth = "agent"

[profiles.local-smoke]
kind = "local"
command = ["pwsh", "-NoLogo"]
"#,
        );

        let json = super::profile_list_json(&crate::cli::ProfileListOptions {
            file: file.clone(),
            verbose: false,
            json: true,
        })
        .unwrap();

        remove_file(&file);

        assert_eq!(
            json,
            "[{\"name\":\"local-smoke\",\"kind\":\"local\",\"command\":\"rterm local -- pwsh -NoLogo\",\"argv\":[\"rterm\",\"local\",\"--\",\"pwsh\",\"-NoLogo\"]},{\"name\":\"prod-shell\",\"kind\":\"ssh\",\"command\":\"rterm ssh --target prod --agent\",\"argv\":[\"rterm\",\"ssh\",\"--target\",\"prod\",\"--agent\"]}]"
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn checks_all_profiles_and_reports_invalid_entries() {
        let file = temp_profile_file(
            "check-profile",
            r#"
[profiles.good]
kind = "local"
command = ["pwsh", "-NoLogo"]

[profiles.bad]
kind = "ssh"
auth = "agent"
"#,
        );

        let error = super::check_profiles(&crate::cli::ProfileCheckOptions {
            file: file.clone(),
            json: false,
        })
        .unwrap_err();

        remove_file(&file);

        assert_eq!(
            error.to_string(),
            "profile check failed: bad (ssh): --host or --target is required"
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn checks_profiles_as_json_with_per_profile_results() {
        let file = temp_profile_file(
            "check-json-profile",
            r#"
[profiles.good]
kind = "local"
command = ["pwsh", "-NoLogo"]

[profiles.bad]
kind = "ssh"
auth = "agent"
"#,
        );

        let json = super::profile_check_json(&crate::cli::ProfileCheckOptions {
            file: file.clone(),
            json: true,
        })
        .unwrap();

        remove_file(&file);

        assert_eq!(
            json,
            "{\"ok\":false,\"profiles\":[{\"name\":\"bad\",\"kind\":\"ssh\",\"ok\":false,\"error\":\"--host or --target is required\"},{\"name\":\"good\",\"kind\":\"local\",\"ok\":true}]}"
        );
    }

    #[test]
    fn initializes_missing_profile_file_from_template() {
        let mut file = std::env::temp_dir();
        file.push(format!("rssh-init-profile-{}.toml", std::process::id()));
        remove_file(&file);

        super::init_profile_file(&crate::cli::ProfileInitOptions {
            file: file.clone(),
            force: false,
        })
        .unwrap();

        let contents = fs::read_to_string(&file).unwrap();
        remove_file(&file);

        assert!(contents.contains("[profiles.local-smoke]"));
        assert_eq!(
            contents.contains("[profiles.prod-shell]"),
            cfg!(feature = "ssh")
        );
        assert!(contents.contains("rterm profile --check"));
        super::validate_profiles_from_toml(&contents).expect("generated profiles match this build");
    }

    #[test]
    fn refuses_to_overwrite_existing_profile_file_without_force() {
        let file = temp_profile_file("init-existing-profile", "existing");

        let error = super::init_profile_file(&crate::cli::ProfileInitOptions {
            file: file.clone(),
            force: false,
        })
        .unwrap_err();

        let contents = fs::read_to_string(&file).unwrap();
        remove_file(&file);

        assert_eq!(
            error.to_string(),
            format!(
                "profile file already exists: {}; use --force to overwrite",
                file.display()
            )
        );
        assert_eq!(contents, "existing");
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn shows_profile_as_resolved_command_line_from_toml_file() {
        let file = temp_profile_file(
            "show-profile",
            r#"
[profiles.prod]
kind = "ssh"
target = "prod"
auth = "agent"
log = "prod.log"
"#,
        );

        let command_line = super::profile_command_line(&crate::cli::ProfileShowOptions {
            name: "prod".to_owned(),
            file: file.clone(),
            json: false,
        })
        .unwrap();

        remove_file(&file);

        assert_eq!(
            command_line,
            "rterm ssh --target prod --agent --log prod.log"
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn shows_profile_as_json_with_resolved_command() {
        let file = temp_profile_file(
            "show-json-profile",
            r#"
[profiles.prod]
kind = "ssh"
target = "prod"
auth = "agent"
log = "prod.log"
"#,
        );

        let json = super::profile_show_json(&crate::cli::ProfileShowOptions {
            name: "prod".to_owned(),
            file: file.clone(),
            json: true,
        })
        .unwrap();

        remove_file(&file);

        assert_eq!(
            json,
            "{\"name\":\"prod\",\"kind\":\"ssh\",\"command\":\"rterm ssh --target prod --agent --log prod.log\",\"argv\":[\"rterm\",\"ssh\",\"--target\",\"prod\",\"--agent\",\"--log\",\"prod.log\"]}"
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn ssh_profile_gui_true_maps_to_native_gui_renderer_options() {
        let contents = r#"
[profiles.gui-prod]
kind = "ssh"
target = "prod"
gui = true
renderer = "cpu"
benchmark_startup = true
auth = "agent"
"#;

        assert_eq!(
            super::args_from_toml("gui-prod", contents).unwrap(),
            [
                "rterm",
                "ssh",
                "--target",
                "prod",
                "--gui",
                "--renderer",
                "cpu",
                "--benchmark-startup",
                "--agent"
            ]
        );

        let AppCommand::Ssh(options) = super::command_from_toml("gui-prod", contents).unwrap()
        else {
            panic!("expected SSH command");
        };
        assert!(options.gui);
        assert!(options.native);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn gui_ssh_profile_prompt_policy_maps_to_interactive_verification() {
        let contents = r#"
[profiles.gui-prod]
kind = "ssh"
target = "prod"
gui = true
host_key_policy = "prompt"
auth = "agent"
"#;

        assert_eq!(
            super::args_from_toml("gui-prod", contents).unwrap(),
            ["rterm", "ssh", "--target", "prod", "--gui", "--agent"]
        );

        let AppCommand::Ssh(options) = super::command_from_toml("gui-prod", contents).unwrap()
        else {
            panic!("expected SSH command");
        };
        assert!(options.gui);
        assert!(options.native);
        assert_eq!(
            options.native_host_key_policy,
            NativeHostKeyPolicy::RejectUnknown,
            "the GUI boundary translates the safe CLI default into an interactive prompt"
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn gui_ssh_profile_rejects_forwarding_and_no_shell() {
        for extra in [
            "local_forward = [\"127.0.0.1:1234:db:5432\"]",
            "no_shell = true",
        ] {
            let contents = format!(
                "[profiles.gui-prod]\nkind = \"ssh\"\ntarget = \"prod\"\ngui = true\n{extra}\n"
            );
            let error = super::command_from_toml("gui-prod", &contents).unwrap_err();
            assert!(
                error.contains("GUI"),
                "unexpected GUI profile error: {error}"
            );
        }
    }
}
