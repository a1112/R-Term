use std::{path::Path, process::Command};

use rterm_pty::PtyCommand;

const BLOCKED_PROXY: &str = "http://127.0.0.1:9";
const LOOPBACK_BYPASS: &str = "localhost,127.0.0.1,::1";

pub(crate) fn hermetic_app_command(app: &Path) -> Command {
    let mut command = Command::new(app);
    apply_loopback_only_environment(&mut command);
    command
}

pub(crate) fn hermetic_pty_command(program: impl Into<String>) -> PtyCommand {
    let mut command = PtyCommand::new(program);
    for key in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ] {
        command = command.with_env(key, BLOCKED_PROXY);
    }
    command
        .with_env("NO_PROXY", LOOPBACK_BYPASS)
        .with_env("no_proxy", LOOPBACK_BYPASS)
        .with_env("RTERM_FUNCTIONAL_NETWORK_POLICY", "loopback-only")
}

pub(crate) fn apply_loopback_only_environment(command: &mut Command) {
    for key in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ] {
        command.env(key, BLOCKED_PROXY);
    }
    command
        .env("NO_PROXY", LOOPBACK_BYPASS)
        .env("no_proxy", LOOPBACK_BYPASS)
        .env("RTERM_FUNCTIONAL_NETWORK_POLICY", "loopback-only");
}

#[cfg(test)]
mod tests {
    use super::{hermetic_app_command, hermetic_pty_command};

    #[test]
    fn child_environment_blocks_proxy_traffic_and_bypasses_only_loopback() {
        let command = hermetic_app_command(std::path::Path::new("fixture"));
        let environment: std::collections::BTreeMap<_, _> = command
            .get_envs()
            .filter_map(|(key, value)| value.map(|value| (key.to_owned(), value.to_owned())))
            .collect();
        assert_eq!(
            environment[std::ffi::OsStr::new("HTTP_PROXY")],
            "http://127.0.0.1:9"
        );
        assert_eq!(
            environment[std::ffi::OsStr::new("NO_PROXY")],
            "localhost,127.0.0.1,::1"
        );
        assert_eq!(
            environment[std::ffi::OsStr::new("RTERM_FUNCTIONAL_NETWORK_POLICY")],
            "loopback-only"
        );
    }

    #[test]
    fn pty_fixture_receives_the_same_loopback_only_policy() {
        let command = hermetic_pty_command("fixture");
        assert_eq!(command.env_value("HTTP_PROXY"), Some("http://127.0.0.1:9"));
        assert_eq!(
            command.env_value("NO_PROXY"),
            Some("localhost,127.0.0.1,::1")
        );
        assert_eq!(
            command.env_value("RTERM_FUNCTIONAL_NETWORK_POLICY"),
            Some("loopback-only")
        );
    }
}
