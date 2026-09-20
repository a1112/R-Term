use std::{process::Command, time::Duration};

#[cfg(not(feature = "ssh"))]
use rssh_test_support::TempHome;
use rssh_test_support::{ChildGuard, ChildOutput};

fn run(args: &[&str]) -> ChildOutput {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rterm"));
    command.args(args);
    ChildGuard::spawn(command, Duration::from_secs(10))
        .expect("launch rterm")
        .wait()
        .expect("rterm completes within its deadline")
}

#[test]
fn command_identity_and_help_match_compiled_capabilities() {
    let output = run(&["version", "--json"]);
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["name"], "rterm");
    assert_eq!(report["console"], true);
    assert_eq!(
        report["native_ssh_backend"],
        if cfg!(feature = "ssh") {
            "russh"
        } else {
            "disabled"
        }
    );

    let output = run(&["--help"]);
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.starts_with("R-Term\n"));
    assert!(help.contains("rterm local "));
    assert!(help.contains("rterm window "));
    assert!(!help.contains("rssh-app"));
    assert_eq!(help.contains("rterm ssh "), cfg!(feature = "ssh"));
    assert_eq!(
        help.contains("rterm scp "),
        cfg!(feature = "transfer-tools")
    );
}

#[cfg(not(feature = "ssh"))]
#[test]
fn disabled_remote_commands_fail_before_resolving_or_starting_a_transport() {
    for command in ["ssh", "scp", "sftp"] {
        let output = run(&[command, "user@must-not-resolve.invalid"]);
        assert!(!output.status.success());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains("SSH support is disabled"), "{error}");
        assert!(output.stdout.is_empty());
    }
}

#[cfg(not(feature = "ssh"))]
#[test]
fn remote_profile_cannot_bypass_disabled_extension() {
    let home = TempHome::new().unwrap();
    let path = home.path().join("profiles.toml");
    std::fs::write(
        &path,
        "[profiles.remote]\nkind = \"ssh\"\ntarget = \"user@must-not-resolve.invalid\"\n",
    )
    .unwrap();
    let output = run(&["profile", "remote", "--file", path.to_str().unwrap()]);
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("SSH support is disabled"), "{error}");
}
