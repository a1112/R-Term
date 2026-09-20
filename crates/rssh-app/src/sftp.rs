use std::error::Error;

use rssh_pty::{PtyCommand, PtyExitStatus, PtySize};
use rssh_ssh::{SshAuthMethod, SshConnectRequest};

use crate::{
    cli::{LocalOptions, OpenSshTarget, Osc52Policy, SftpOptions, SshTarget},
    local,
};

pub fn run(options: &SftpOptions) -> Result<PtyExitStatus, Box<dyn Error>> {
    let local_options = local_options_for_options(options)?;

    local::run(&local_options)
}

fn local_options_for_options(options: &SftpOptions) -> Result<LocalOptions, Box<dyn Error>> {
    let size = match &options.target {
        SshTarget::Direct(request) => request.config.initial_size.into(),
        SshTarget::OpenSsh(target) => target.initial_size,
    };

    Ok(LocalOptions {
        command: sftp_command_for_options(options),
        size: Some(PtySize::try_new(size.columns, size.rows)?),
        mouse: true,
        console: options.console,
        osc52_policy: Osc52Policy::default(),
        log: options.log.clone(),
    })
}

fn sftp_command_for_options(options: &SftpOptions) -> PtyCommand {
    match &options.target {
        SshTarget::Direct(request) => sftp_command_for_request(options, request),
        SshTarget::OpenSsh(target) => sftp_command_for_target(options, target),
    }
}

fn sftp_command_for_request(options: &SftpOptions, request: &SshConnectRequest) -> PtyCommand {
    let mut args = Vec::new();

    args.extend(options.openssh_args.clone());
    append_auth_args(&mut args, &request.auth);
    if request.config.port != 22 {
        args.push("-P".to_owned());
        args.push(request.config.port.to_string());
    }
    args.push(format!(
        "{}@{}",
        request.config.username, request.config.host
    ));

    PtyCommand::new("sftp").with_args(args)
}

fn sftp_command_for_target(options: &SftpOptions, target: &OpenSshTarget) -> PtyCommand {
    let mut args = Vec::new();

    args.extend(options.openssh_args.clone());
    append_auth_args(&mut args, &target.auth);
    if let Some(username) = &target.username {
        args.push("-o".to_owned());
        args.push(format!("User={username}"));
    }
    if let Some(port) = target.port {
        args.push("-P".to_owned());
        args.push(port.to_string());
    }
    args.push(target.target.clone());

    PtyCommand::new("sftp").with_args(args)
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

#[cfg(test)]
mod tests {
    use rssh_core::TerminalSize;
    use rssh_ssh::{SshAuthMethod, SshConnectRequest, SshSessionConfig};

    use super::*;
    use crate::cli::{OpenSshTarget, SftpOptions, SshTarget};

    #[test]
    fn sftp_command_uses_direct_host_user_and_sftp_port_flag() {
        let request = SshConnectRequest::agent(
            SshSessionConfig::try_new("example.com", 2222, "ops", TerminalSize::new(80, 24).into())
                .unwrap(),
        );

        let command = sftp_command_for_options(&SftpOptions {
            target: SshTarget::Direct(request),
            openssh_args: Vec::new(),
            console: crate::cli::ConsoleOptions::default(),
            log: None,
        });

        assert_eq!(command.program(), "sftp");
        assert_eq!(command.args(), ["-P", "2222", "ops@example.com"]);
    }

    #[test]
    fn sftp_command_uses_config_target_overrides_and_private_key() {
        let command = sftp_command_for_options(&SftpOptions {
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
            console: crate::cli::ConsoleOptions::default(),
            log: None,
        });

        assert_eq!(command.program(), "sftp");
        assert_eq!(
            command.args(),
            [
                "-i",
                "C:/Users/ops/.ssh/id_ed25519",
                "-o",
                "User=deploy",
                "-P",
                "2200",
                "prod"
            ]
        );
    }

    #[test]
    fn sftp_command_preserves_passthrough_options_before_target() {
        let command = sftp_command_for_options(&SftpOptions {
            target: SshTarget::OpenSsh(OpenSshTarget {
                target: "prod".to_owned(),
                username: None,
                port: None,
                initial_size: TerminalSize::new(100, 40),
                auth: SshAuthMethod::Agent,
            }),
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
                "prod"
            ]
        );
    }
}
