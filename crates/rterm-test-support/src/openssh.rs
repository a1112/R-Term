use std::{collections::BTreeSet, env, process::Command, time::Duration};

use crate::{ChildGuard, ChildGuardError, TempHome};

// Match the native E2E preflight budget: a cold hosted Windows client can
// exceed five seconds before emitting its version banner. This only bounds
// capability discovery; SSH operation and fixture deadlines stay separate.
const PROBE_DEADLINE: Duration = Duration::from_secs(15);

/// SSH client command-line capabilities exercised by interoperability gates.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OpenSshClientTool {
    Ssh,
    Sftp,
    Scp,
}

impl OpenSshClientTool {
    const fn program(self) -> &'static str {
        match self {
            Self::Ssh => "ssh",
            Self::Sftp => "sftp",
            Self::Scp => "scp",
        }
    }

    const fn probe_argument(self) -> &'static str {
        match self {
            Self::Ssh => "-V",
            Self::Sftp | Self::Scp => "-h",
        }
    }

    const fn diagnostic_marker(self) -> &'static str {
        match self {
            Self::Ssh => "openssh",
            Self::Sftp => "usage: sftp",
            Self::Scp => "usage: scp",
        }
    }
}

/// Whether missing OpenSSH tools fail the gate or produce an explicit local skip.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpenSshProbePolicy {
    Required,
    OptionalLocal,
}

impl OpenSshProbePolicy {
    /// Resolves policy without reading process-global state, for deterministic tests.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid policy value or any CI opt-out attempt.
    pub fn from_values(ci: Option<&str>, required: Option<&str>) -> Result<Self, String> {
        let ci = ci.is_some_and(truthy);
        let requested = match required {
            None => Self::Required,
            Some(value) if truthy(value) => Self::Required,
            Some(value) if falsey(value) => Self::OptionalLocal,
            Some(value) => {
                return Err(format!(
                    "RTERM_REQUIRE_OPENSSH must be one of 1/0, true/false, yes/no; got {value:?}"
                ));
            }
        };
        if ci && requested == Self::OptionalLocal {
            return Err(
                "CI cannot disable the required OpenSSH interoperability gate with \
                 RTERM_REQUIRE_OPENSSH=0"
                    .to_owned(),
            );
        }
        Ok(requested)
    }

    fn from_environment() -> Result<Self, String> {
        let ci = env::var("CI").ok();
        let required = env::var("RTERM_REQUIRE_OPENSSH").ok();
        Self::from_values(ci.as_deref(), required.as_deref())
    }
}

/// Probes system clients with bounded children and an isolated HOME.
///
/// The gate is required by default. A developer may explicitly set
/// `RTERM_REQUIRE_OPENSSH=0` outside CI to obtain a visible local skip. CI always
/// rejects that opt-out.
///
/// # Errors
///
/// Returns an error when policy is invalid, a required executable is missing,
/// a probe exceeds its deadline, or diagnostics do not identify OpenSSH.
pub fn probe_openssh_tools_from_environment(
    requested: &[OpenSshClientTool],
) -> Result<bool, String> {
    let policy = OpenSshProbePolicy::from_environment()?;
    let mut tools = requested.iter().copied().collect::<BTreeSet<_>>();
    // The ssh version banner identifies the required OpenSSH SSH client. The
    // sftp/scp usage probes establish only the requested command capability;
    // the real transfer tests provide the interoperability proof.
    tools.insert(OpenSshClientTool::Ssh);
    let home =
        TempHome::new().map_err(|error| format!("OpenSSH probe HOME setup failed: {error}"))?;

    for tool in tools {
        match probe_tool(tool, &home) {
            Ok(()) => {}
            Err(ProbeFailure::Missing(message)) if policy == OpenSshProbePolicy::OptionalLocal => {
                eprintln!("SKIP: {message}; local opt-out RTERM_REQUIRE_OPENSSH=0 is active");
                return Ok(false);
            }
            Err(ProbeFailure::Missing(message)) => {
                return Err(format!("required OpenSSH tool unavailable: {message}"));
            }
            Err(ProbeFailure::Invalid(message)) => return Err(message),
        }
    }
    Ok(true)
}

enum ProbeFailure {
    Missing(String),
    Invalid(String),
}

fn probe_tool(tool: OpenSshClientTool, home: &TempHome) -> Result<(), ProbeFailure> {
    let mut command = Command::new(tool.program());
    command.arg(tool.probe_argument());
    isolate_command(&mut command, home);
    let output = ChildGuard::spawn(command, PROBE_DEADLINE)
        .map_err(|error| classify_probe_error(tool, &error))?
        .wait()
        .map_err(|error| {
            ProbeFailure::Invalid(format!("{} probe failed: {error}", tool.program()))
        })?;
    let mut diagnostics = output.stdout;
    diagnostics.extend_from_slice(&output.stderr);
    let diagnostics = String::from_utf8_lossy(&diagnostics).to_ascii_lowercase();
    if !diagnostics.contains(tool.diagnostic_marker()) {
        let expectation = match tool {
            OpenSshClientTool::Ssh => "did not identify an OpenSSH SSH client",
            OpenSshClientTool::Sftp | OpenSshClientTool::Scp => {
                "did not expose the expected command-line capability"
            }
        };
        return Err(ProbeFailure::Invalid(format!(
            "{} probe {expectation}; status={:?}, diagnostics={diagnostics:?}",
            tool.program(),
            output.status.code()
        )));
    }
    Ok(())
}

fn classify_probe_error(tool: OpenSshClientTool, error: &ChildGuardError) -> ProbeFailure {
    if matches!(
        &error,
        ChildGuardError::Io {
            operation: "spawn",
            source,
            ..
        } if source.kind() == std::io::ErrorKind::NotFound
    ) {
        ProbeFailure::Missing(format!("{} was not found on PATH", tool.program()))
    } else {
        ProbeFailure::Invalid(format!("{} probe failed: {error}", tool.program()))
    }
}

fn isolate_command(command: &mut Command, home: &TempHome) {
    home.apply_to(command);
    for variable in [
        "SSH_AUTH_SOCK",
        "SSH_ASKPASS",
        "SSH_ASKPASS_REQUIRE",
        "DISPLAY",
    ] {
        command.env_remove(variable);
    }
}

fn truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes"
    )
}

fn falsey(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "no"
    )
}

#[cfg(test)]
mod tests {
    use super::OpenSshProbePolicy;

    #[test]
    fn openssh_gate_is_required_by_default_and_in_ci() {
        assert_eq!(
            OpenSshProbePolicy::from_values(None, None).unwrap(),
            OpenSshProbePolicy::Required
        );
        assert_eq!(
            OpenSshProbePolicy::from_values(Some("true"), None).unwrap(),
            OpenSshProbePolicy::Required
        );
    }

    #[test]
    fn local_opt_out_is_explicit_and_ci_cannot_use_it() {
        assert_eq!(
            OpenSshProbePolicy::from_values(None, Some("0")).unwrap(),
            OpenSshProbePolicy::OptionalLocal
        );
        assert!(OpenSshProbePolicy::from_values(Some("1"), Some("0")).is_err());
        assert!(OpenSshProbePolicy::from_values(None, Some("sometimes")).is_err());
    }
}
