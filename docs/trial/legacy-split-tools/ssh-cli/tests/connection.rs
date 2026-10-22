use rssh_test_support::{
    ChildGuard,
    ssh::{CommandResponse, HermeticSshServer},
};
use std::process::Command;
use std::time::Duration;

const DEADLINE: Duration = Duration::from_secs(10);

fn command(server: &HermeticSshServer) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rssh"));
    command
        .arg("--known-hosts")
        .arg(server.known_hosts_path())
        .arg("-p")
        .arg(server.address().port().to_string())
        .arg("-i")
        .arg(server.agent().identity_path())
        .arg("fixture-user@127.0.0.1")
        .arg("trial-status");
    command
}

#[test]
fn native_cli_returns_remote_output_and_exit_status() {
    let server = HermeticSshServer::builder()
        .command(
            "'trial-status'",
            CommandResponse::status(b"ssh-only-trial\n", b"", 42),
        )
        .start(DEADLINE)
        .unwrap();
    let output = ChildGuard::spawn(command(&server), DEADLINE)
        .unwrap()
        .wait()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(42),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"ssh-only-trial\n");
    server.stop(DEADLINE).unwrap();
}

#[test]
fn native_cli_rejects_an_unknown_host_key() {
    let server = HermeticSshServer::start(DEADLINE).unwrap();
    std::fs::write(server.known_hosts_path(), b"").unwrap();
    let output = ChildGuard::spawn(command(&server), DEADLINE)
        .unwrap()
        .wait()
        .unwrap();
    assert_eq!(output.status.code(), Some(255));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    server.stop(DEADLINE).unwrap();
}
