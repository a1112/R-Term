use std::io::Read;
use std::path::PathBuf;
use rssh_ssh::{RusshChannelOpener, SshAuthMethod, SshChannelConnector, SshConnectRequest,
    SshInputEvent, SshSessionConfig, SshSessionStartup, SshShellConnector,
    run_connected_shell_with_events, ssh_input_event_channel};
use rssh_types::TerminalSize;

const HELP: &str = "rssh [--known-hosts FILE] [-p PORT] [-i KEY | --password] USER@HOST [COMMAND ...]\n\nNative SSH trial. Default authentication: agent. Unknown/changed keys are rejected.\nCommands run through a remote PTY; stdin/stdout are streamed in both directions.\nInteractive input currently uses the host console's cooked mode.\nOpenSSH config, scp/sftp and forwarding CLI options are not implemented.";

#[derive(Debug)]
struct Options {
    host: String,
    user: String,
    port: u16,
    known_hosts: PathBuf,
    auth: SshAuthMethod,
    command: Vec<String>,
}

fn parse(args: Vec<String>) -> Result<Option<Options>, String> {
    if args.as_slice() == ["--help"] || args.as_slice() == ["-h"] {
        println!("{HELP}");
        return Ok(None);
    }
    if args.as_slice() == ["--version"] {
        println!("rssh {} (SSH-only trial)", env!("CARGO_PKG_VERSION"));
        return Ok(None);
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    let mut known_hosts = home.map(|p| PathBuf::from(p).join(".ssh/known_hosts"));
    let mut port = 22;
    let mut auth = SshAuthMethod::Agent;
    let mut auth_selected = false;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-p" => {
                port = iter.next().ok_or("-p requires a port")?.parse::<u16>().map_err(|_| "invalid port")?;
                if port == 0 { return Err("port must be nonzero".into()); }
            }
            "--known-hosts" => known_hosts = Some(iter.next().ok_or("--known-hosts requires a file")?.into()),
            "-i" | "--password" => {
                if auth_selected { return Err("select only one authentication method".into()); }
                auth_selected = true;
                auth = if arg == "-i" {
                    SshAuthMethod::private_key(iter.next().ok_or("-i requires a key file")?, None::<String>)
                        .map_err(|e| format!("{e:?}"))?
                } else { SshAuthMethod::PasswordPrompt };
            }
            _ if arg.starts_with('-') => return Err(format!("unknown option: {arg}")),
            _ => {
                let (user, host) = arg.split_once('@').ok_or("destination must be USER@HOST")?;
                if user.is_empty() || host.is_empty() { return Err("empty user or host".into()); }
                return Ok(Some(Options { host: host.into(), user: user.into(), port,
                    known_hosts: known_hosts.ok_or("home unavailable; supply --known-hosts")?, auth,
                    command: iter.collect() }));
            }
        }
    }
    Err(format!("missing destination\n{HELP}"))
}

fn run(options: Options) -> Result<u8, String> {
    let config = SshSessionConfig::try_new(options.host, options.port, options.user, TerminalSize::new(80, 24))
        .map_err(|e| format!("{e:?}"))?;
    let mut request = SshConnectRequest::new(config, options.auth);
    if !options.command.is_empty() {
        request = request.with_startup(SshSessionStartup::command(options.command).map_err(|e| format!("{e:?}"))?);
    }
    let opener = RusshChannelOpener::default().with_known_hosts_path(options.known_hosts)
        .with_secret_provider(|prompt| async move {
            rpassword::prompt_password(format!("{} {:?}: ", prompt.username, prompt.kind)).ok()
        });
    let session = SshChannelConnector::new(opener).connect(request).map_err(|e| e.to_string())?;
    let (sender, receiver) = ssh_input_event_channel(32);
    // Do not join a blocked console read after the remote closes. The process
    // exits after the SSH pumps have completed their own cancellation/cleanup.
    std::thread::spawn(move || {
        let mut input = std::io::stdin().lock();
        let mut bytes = [0; 8192];
        loop {
            let event = match input.read(&mut bytes) {
                Ok(0) => SshInputEvent::Eof,
                Ok(n) => SshInputEvent::Data(bytes[..n].to_vec()),
                Err(e) => SshInputEvent::Error(e.to_string()),
            };
            let done = matches!(event, SshInputEvent::Eof | SshInputEvent::Error(_));
            if sender.send(event).is_err() || done { break; }
        }
    });
    let outcome = run_connected_shell_with_events(session, receiver, &mut std::io::stdout().lock())
        .map_err(|e| e.to_string())?;
    Ok(if outcome.result.exit_signal.is_some() { 255 } else {
        outcome.result.exit_status.and_then(|status| u8::try_from(status).ok()).unwrap_or(255)
    })
}

fn main() -> std::process::ExitCode {
    match parse(std::env::args().skip(1).collect()).and_then(|options| options.map_or(Ok(0), run)) {
        Ok(code) => std::process::ExitCode::from(code),
        Err(error) => { eprintln!("rssh: {error}"); std::process::ExitCode::from(255) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> Vec<String> { values.iter().map(|s| (*s).into()).collect() }
    #[test]
    fn rejects_ambiguous_auth_invalid_ports_and_unknown_options() {
        for values in [vec!["-p", "0", "u@h"], vec!["--password", "-i", "key", "u@h"],
                       vec!["-o", "StrictHostKeyChecking=no", "u@h"], vec!["@host"], vec!["user@"]] {
            assert!(parse(args(&values)).is_err());
        }
    }
    #[test]
    fn preserves_remote_command_and_explicit_connection_settings() {
        let options = parse(args(&["--known-hosts", "hosts", "-p", "2222", "-i", "key", "user@host", "printf", "%s", "hello world"]))
            .unwrap().unwrap();
        assert_eq!(options.port, 2222);
        assert_eq!(options.known_hosts, PathBuf::from("hosts"));
        assert_eq!(options.command, ["printf", "%s", "hello world"]);
        assert!(matches!(options.auth, SshAuthMethod::PrivateKey { .. }));
    }
}
