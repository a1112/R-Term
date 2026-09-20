#![cfg(windows)]

use std::{
    fs,
    process::Command,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rssh_diagnostics::MARKER_PREFIX;
use rssh_test_support::{
    ChildGuard,
    ssh::{HermeticSshServer, SshEvent},
};

const RSSH_APP_EXECUTABLE: &str = env!("CARGO_BIN_EXE_rterm");
const DEADLINE: Duration = Duration::from_secs(20);

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the end-to-end contract is intentionally linear so marker, fixture, and leak assertions share one process lifetime"
)]
fn ssh1_diagnostic_reaches_visible_connected_readiness_without_leaking_secret() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after Unix epoch")
        .as_nanos();
    let secret = format!("rssh-stage0-secret-{unique}");
    let user = "fixture-user";
    let server = HermeticSshServer::builder()
        .password(user, &secret)
        .start(Duration::from_secs(5))
        .expect("start isolated SSH1 fixture");
    let ssh_dir = server.temp_home().path().join(".ssh");
    fs::create_dir_all(&ssh_dir).expect("create ambient SSH directory");
    fs::write(
        ssh_dir.join("config"),
        "Host *\n  HostName 203.0.113.1\n  Port 1\n  User ambient-user\n",
    )
    .expect("write hostile ambient SSH config");
    let session_log = server.temp_home().path().join("ssh1-session.log");
    let run_id = format!("ssh1-{}-{unique}", std::process::id());
    let port = server.address().port().to_string();
    let mut command = Command::new(RSSH_APP_EXECUTABLE);
    command.args([
        "diagnostic-gui",
        "--run-id",
        run_id.as_str(),
        "--scenario",
        "ssh1",
        "--hold-ms",
        "300",
        "--renderer",
        "cpu",
        "--ssh-host",
        "127.0.0.1",
        "--ssh-port",
        port.as_str(),
        "--ssh-user",
        user,
        "--log",
        session_log.to_str().expect("UTF-8 session log path"),
    ]);
    server.temp_home().apply_to(&mut command);
    command
        .env("RSSH_DIAGNOSTIC_SSH_SECRET", &secret)
        .env("SSH_AUTH_SOCK", r"\\.\pipe\rssh-invalid-agent");

    let output = ChildGuard::spawn(command, DEADLINE)
        .expect("spawn SSH1 diagnostic")
        .wait()
        .expect("SSH1 diagnostic exits within its bounded hold");
    let stdout = String::from_utf8(output.stdout).expect("SSH1 stdout is UTF-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let session_log_bytes = fs::read(&session_log).unwrap_or_default();
    assert!(
        output.status.success(),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let records = stdout
        .lines()
        .filter_map(|line| line.strip_prefix(MARKER_PREFIX))
        .map(|json| serde_json::from_str::<serde_json::Value>(json).expect("valid SSH1 marker"))
        .collect::<Vec<_>>();
    let kinds = records
        .iter()
        .filter_map(|record| record["kind"].as_str())
        .collect::<Vec<_>>();
    for required in [
        "process_started",
        "window_created",
        "first_present",
        "transport_started",
        "transport_ready",
        "scenario_ready",
        "process_exited",
    ] {
        assert!(
            kinds.contains(&required),
            "missing {required}: {records:#?}"
        );
    }
    let elapsed = |kind: &str| {
        records
            .iter()
            .find(|record| record["kind"] == kind)
            .and_then(|record| record["elapsed_ms"].as_u64())
            .unwrap_or_else(|| panic!("missing elapsed time for {kind}"))
    };
    assert!(elapsed("transport_ready") >= elapsed("transport_started"));
    assert!(elapsed("scenario_ready") >= elapsed("transport_ready"));
    assert!(elapsed("scenario_ready") >= elapsed("first_present"));
    let ready = records
        .iter()
        .find(|record| record["kind"] == "scenario_ready")
        .expect("scenario readiness marker");
    assert_eq!(ready["connection_state"], "connected");
    assert_eq!(ready["visible_connection_state"], "connected");
    assert_eq!(ready["secret_prompt_presented"], true);
    assert!(
        ready["visible_cell_count"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "readiness lacks visible snapshot evidence: {ready:#}"
    );

    let event_log = server.events();
    assert!(
        event_log
            .iter()
            .any(|event| matches!(event, SshEvent::Connection { .. }))
    );
    assert!(event_log.contains(&SshEvent::SessionOpened));
    assert!(event_log.iter().any(|event| matches!(
        event,
        SshEvent::Pty {
            columns: 80,
            rows: 24,
            ..
        }
    )));
    assert!(event_log.contains(&SshEvent::Shell));

    for (surface, bytes) in [
        ("stdout", stdout.as_bytes()),
        ("stderr", stderr.as_bytes()),
        ("session log", session_log_bytes.as_slice()),
    ] {
        assert!(
            !bytes
                .windows(secret.len())
                .any(|window| window == secret.as_bytes()),
            "{surface} leaked the SSH fixture secret"
        );
    }

    let stopped_at = Instant::now();
    server
        .stop(Duration::from_secs(5))
        .expect("stop SSH1 fixture after app shutdown");
    assert!(stopped_at.elapsed() < Duration::from_secs(5));
}
