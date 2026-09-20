use std::error::Error;

use rssh_pty::{PtyCommand, PtyExitStatus, PtySize};
use rssh_ssh::{SshAuthMethod, SshConnectRequest};

use crate::{
    cli::{LocalOptions, OpenSshTarget, Osc52Policy, ScpOptions, ScpTransfer, SshTarget},
    local,
};

pub fn run(options: &ScpOptions) -> Result<PtyExitStatus, Box<dyn Error>> {
    let local_options = local_options_for_options(options)?;

    local::run(&local_options)
}

fn local_options_for_options(options: &ScpOptions) -> Result<LocalOptions, Box<dyn Error>> {
    let size = match &options.target {
        SshTarget::Direct(request) => request.config.initial_size.into(),
        SshTarget::OpenSsh(target) => target.initial_size,
    };

    Ok(LocalOptions {
        command: scp_command_for_options(options),
        size: Some(PtySize::try_new(size.columns, size.rows)?),
        mouse: false,
        console: options.console,
        osc52_policy: Osc52Policy::default(),
        log: options.log.clone(),
    })
}

fn scp_command_for_options(options: &ScpOptions) -> PtyCommand {
    match &options.target {
        SshTarget::Direct(request) => scp_command_for_request(options, request),
        SshTarget::OpenSsh(target) => scp_command_for_target(options, target),
    }
}

fn scp_command_for_request(options: &ScpOptions, request: &SshConnectRequest) -> PtyCommand {
    let mut args = Vec::new();

    args.extend(options.openssh_args.clone());
    append_common_args(&mut args, &request.auth, options.recursive);
    if request.config.port != 22 {
        args.push("-P".to_owned());
        args.push(request.config.port.to_string());
    }

    let remote_prefix = format!("{}@{}:", request.config.username, request.config.host);
    append_transfer_args(&mut args, &options.transfer, &remote_prefix);

    PtyCommand::new("scp").with_args(args)
}

fn scp_command_for_target(options: &ScpOptions, target: &OpenSshTarget) -> PtyCommand {
    let mut args = Vec::new();

    args.extend(options.openssh_args.clone());
    append_common_args(&mut args, &target.auth, options.recursive);
    if let Some(username) = &target.username {
        args.push("-o".to_owned());
        args.push(format!("User={username}"));
    }
    if let Some(port) = target.port {
        args.push("-P".to_owned());
        args.push(port.to_string());
    }

    let remote_prefix = format!("{}:", target.target);
    append_transfer_args(&mut args, &options.transfer, &remote_prefix);

    PtyCommand::new("scp").with_args(args)
}

fn append_common_args(args: &mut Vec<String>, auth: &SshAuthMethod, recursive: bool) {
    if recursive {
        args.push("-r".to_owned());
    }

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

fn append_transfer_args(args: &mut Vec<String>, transfer: &ScpTransfer, remote_prefix: &str) {
    match transfer {
        ScpTransfer::Upload { local, remote } => {
            args.push(local.to_string_lossy().into_owned());
            args.push(format!("{remote_prefix}{remote}"));
        }
        ScpTransfer::UploadMany { locals, remote } => {
            args.extend(
                locals
                    .iter()
                    .map(|local| local.to_string_lossy().into_owned()),
            );
            args.push(format!("{remote_prefix}{remote}"));
        }
        ScpTransfer::Download { remote, local } => {
            args.push(format!("{remote_prefix}{remote}"));
            args.push(local.to_string_lossy().into_owned());
        }
        ScpTransfer::DownloadMany { remotes, local } => {
            args.extend(
                remotes
                    .iter()
                    .map(|remote| format!("{remote_prefix}{remote}")),
            );
            args.push(local.to_string_lossy().into_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use rssh_core::TerminalSize;
    use rssh_ssh::{SshAuthMethod, SshConnectRequest, SshSessionConfig};

    use super::*;
    use crate::cli::{OpenSshTarget, ScpOptions, ScpTransfer, SshTarget};

    #[test]
    fn scp_command_uploads_to_direct_target_with_port() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 2222, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );

        let command = scp_command_for_options(&ScpOptions {
            target: SshTarget::Direct(request),
            openssh_args: Vec::new(),
            transfer: ScpTransfer::Upload {
                local: "local.txt".into(),
                remote: "/tmp/remote.txt".to_owned(),
            },
            recursive: false,
            console: crate::cli::ConsoleOptions::default(),
            log: None,
        });

        assert_eq!(command.program(), "scp");
        assert_eq!(
            command.args(),
            ["-P", "2222", "local.txt", "ops@example.com:/tmp/remote.txt"]
        );
    }

    #[test]
    fn scp_command_downloads_from_config_target_with_key_and_recursive_flag() {
        let command = scp_command_for_options(&ScpOptions {
            target: SshTarget::OpenSsh(OpenSshTarget {
                target: "prod".to_owned(),
                username: Some("deploy".to_owned()),
                port: Some(2200),
                initial_size: TerminalSize::new(100, 40),
                auth: SshAuthMethod::PrivateKey {
                    path: "C:/Users/ops/.ssh/id_ed25519".into(),
                    passphrase: None,
                },
            }),
            openssh_args: Vec::new(),
            transfer: ScpTransfer::Download {
                remote: "/var/log/app".to_owned(),
                local: "logs".into(),
            },
            recursive: true,
            console: crate::cli::ConsoleOptions::default(),
            log: None,
        });

        assert_eq!(command.program(), "scp");
        assert_eq!(
            command.args(),
            [
                "-r",
                "-i",
                "C:/Users/ops/.ssh/id_ed25519",
                "-o",
                "User=deploy",
                "-P",
                "2200",
                "prod:/var/log/app",
                "logs"
            ]
        );
    }

    #[test]
    fn scp_command_preserves_passthrough_options_before_transfer_operands() {
        let command = scp_command_for_options(&ScpOptions {
            target: SshTarget::OpenSsh(OpenSshTarget {
                target: "prod".to_owned(),
                username: None,
                port: None,
                initial_size: TerminalSize::new(100, 40),
                auth: SshAuthMethod::Agent,
            }),
            transfer: ScpTransfer::Upload {
                local: "local.txt".into(),
                remote: "/tmp/remote.txt".to_owned(),
            },
            recursive: false,
            openssh_args: vec![
                "-F".to_owned(),
                "C:/Users/ops/.ssh/prod_config".to_owned(),
                "-o".to_owned(),
                "ProxyJump=bastion".to_owned(),
            ],
            console: crate::cli::ConsoleOptions::default(),
            log: None,
        });

        assert_eq!(
            command.args(),
            [
                "-F",
                "C:/Users/ops/.ssh/prod_config",
                "-o",
                "ProxyJump=bastion",
                "local.txt",
                "prod:/tmp/remote.txt"
            ]
        );
    }

    #[test]
    fn scp_command_uploads_multiple_sources_to_config_target() {
        let command = scp_command_for_options(&ScpOptions {
            target: SshTarget::OpenSsh(OpenSshTarget {
                target: "prod".to_owned(),
                username: None,
                port: None,
                initial_size: TerminalSize::new(100, 40),
                auth: SshAuthMethod::Agent,
            }),
            transfer: ScpTransfer::UploadMany {
                locals: vec!["app.log".into(), "audit.log".into()],
                remote: "/tmp/logs/".to_owned(),
            },
            recursive: false,
            openssh_args: Vec::new(),
            console: crate::cli::ConsoleOptions::default(),
            log: None,
        });

        assert_eq!(command.args(), ["app.log", "audit.log", "prod:/tmp/logs/"]);
    }

    #[test]
    fn scp_command_downloads_multiple_sources_from_config_target() {
        let command = scp_command_for_options(&ScpOptions {
            target: SshTarget::OpenSsh(OpenSshTarget {
                target: "prod".to_owned(),
                username: None,
                port: None,
                initial_size: TerminalSize::new(100, 40),
                auth: SshAuthMethod::Agent,
            }),
            transfer: ScpTransfer::DownloadMany {
                remotes: vec![
                    "/var/log/app.log".to_owned(),
                    "/var/log/audit.log".to_owned(),
                ],
                local: "logs".into(),
            },
            recursive: false,
            openssh_args: Vec::new(),
            console: crate::cli::ConsoleOptions::default(),
            log: None,
        });

        assert_eq!(
            command.args(),
            ["prod:/var/log/app.log", "prod:/var/log/audit.log", "logs"]
        );
    }
}
