use std::{process::Command, time::Duration};

use rterm_diagnostics::{DiagnosticsResult, ProcessExitKind, ReadinessStatus};

const LAUNCHER: &str = env!("CARGO_BIN_EXE_rterm-bench-launcher");
const FIXTURE: &str = env!("CARGO_BIN_EXE_rterm-diagnostic-fixture");

fn run_fixture(mode: &str) -> std::process::Output {
    Command::new(LAUNCHER)
        .args([
            "--app",
            FIXTURE,
            "--scenario",
            "empty-window",
            "--stabilization-ms",
            "20",
            "--sample-interval-ms",
            "10",
            "--sample-count",
            "3",
            "--shutdown-timeout-ms",
            "100",
            "--json",
        ])
        .env("RTERM_DIAGNOSTIC_FIXTURE_MODE", mode)
        .output()
        .expect("run production launcher against marker fixture")
}

fn parse_result(output: &std::process::Output) -> DiagnosticsResult {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "launcher did not emit one result JSON object: {error}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn production_launcher_samples_only_the_owned_child_and_reaps_gracefully() {
    let output = run_fixture("success");
    let result = parse_result(&output);

    assert!(output.status.success(), "{result:#?}");
    assert_eq!(result.readiness.status, ReadinessStatus::Ready);
    assert_eq!(result.memory.samples.len(), 3);
    assert_eq!(result.memory.statistics.count, 3);
    assert_eq!(result.process.exit_kind, ProcessExitKind::Requested);
    assert_ne!(result.process.pid, std::process::id());
    assert!(result.milestones.sampling_started_ms.is_some());
    assert!(result.milestones.sampling_finished_ms.is_some());
    assert!(
        result
            .memory
            .samples
            .windows(2)
            .all(|pair| pair[1].elapsed_ms.saturating_sub(pair[0].elapsed_ms) >= 10),
        "samples did not honor the configured interval: {:?}",
        result.memory.samples
    );
    result
        .validate()
        .expect("successful production result validates");
}

#[test]
fn production_launcher_emits_structured_json_when_the_child_exits_early() {
    let output = run_fixture("early-exit");
    let result = parse_result(&output);
    assert!(!output.status.success());
    assert_eq!(result.readiness.status, ReadinessStatus::Failed);
    assert!(
        result
            .failures
            .iter()
            .any(|failure| failure.code == "child_exited_early")
    );
}

#[test]
fn production_launcher_escalates_to_a_bounded_forced_shutdown() {
    let started = std::time::Instant::now();
    let output = run_fixture("ignore-shutdown");
    let result = parse_result(&output);

    assert!(output.status.success(), "{result:#?}");
    assert_eq!(result.process.exit_kind, ProcessExitKind::Forced);
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
#[ignore = "requires a native GUI session and an explicitly built rterm-app"]
fn real_empty_window_produces_a_valid_native_result() {
    run_real_scenario("empty-window");
}

#[test]
#[ignore = "requires a native GUI session and an explicitly built rterm-app"]
fn real_ssh1_produces_a_valid_native_result() {
    run_real_scenario("ssh1");
}

fn run_real_scenario(scenario: &str) {
    let app = std::env::var("RTERM_DIAGNOSTIC_REAL_APP")
        .expect("set RTERM_DIAGNOSTIC_REAL_APP to the rterm-app executable");
    let output = Command::new(LAUNCHER)
        .args([
            "--app",
            app.as_str(),
            "--scenario",
            scenario,
            "--stabilization-ms",
            "100",
            "--sample-interval-ms",
            "20",
            "--sample-count",
            "3",
            "--json",
        ])
        .output()
        .expect("run native diagnostics scenario");
    let result = parse_result(&output);
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("rterm-diagnostic-secret-"),
        "launcher result exposed its isolated SSH secret"
    );
    assert!(output.status.success(), "{result:#?}");
    result.validate().expect("native result validates");
}
