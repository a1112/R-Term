use std::{fs, path::PathBuf, process::Command};

use tempfile::TempDir;

fn runner() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rterm-functional"))
}

fn fixture() -> &'static str {
    env!("CARGO_BIN_EXE_rterm-functional-fixture")
}

fn write_suite(root: &TempDir) {
    fs::create_dir_all(root.path().join("scenarios")).unwrap();
    fs::write(
        root.path().join("behaviors.toml"),
        r#"
schema = 1

[[behaviors]]
id = "BHV-CLI-VERSION"
subsystem = "startup"
summary = "version command exits successfully"
surfaces = ["console", "package"]

[[behaviors]]
id = "BHV-WINDOW-INPUT"
subsystem = "window"
summary = "native window accepts real keyboard input"
surfaces = ["native_window"]
"#,
    )
    .unwrap();
    fs::write(
        root.path().join("scenarios/cli.version.toml"),
        r#"
schema = 1
id = "cli.version"
behavior_ids = ["BHV-CLI-VERSION"]
surface = "console"
fixture = "none"
estimated_cost_ms = 100
actions = [{ type = "finish" }]
checkpoints = [{ type = "exit_status", code = 0 }]
required_evidence = ["event_log"]
"#,
    )
    .unwrap();
    fs::write(
        root.path().join("scenarios/window.input.toml"),
        r#"
schema = 1
id = "window.input"
behavior_ids = ["BHV-WINDOW-INPUT"]
surface = "native_window"
capabilities = ["real_os_keyboard"]
fixture = "terminal_probe"
estimated_cost_ms = 1000
actions = [{ type = "type_text", text = "probe" }, { type = "finish" }]
checkpoints = [{ type = "terminal_contains", text = "probe" }]
required_evidence = ["event_log", "screenshot_on_failure"]
"#,
    )
    .unwrap();
}

#[test]
fn list_and_validate_are_stable_machine_readable_contracts() {
    let suite = TempDir::new().unwrap();
    write_suite(&suite);

    let validate = runner()
        .args(["validate", "--suite"])
        .arg(suite.path())
        .output()
        .unwrap();
    assert!(
        validate.status.success(),
        "{}",
        String::from_utf8_lossy(&validate.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&validate.stdout).unwrap();
    assert_eq!(report["schema"], 1);
    assert_eq!(report["scenarios"], 2);
    assert_eq!(report["behaviors"], 2);

    let list = runner()
        .args(["list", "--suite"])
        .arg(suite.path())
        .output()
        .unwrap();
    assert!(list.status.success());
    let lines: Vec<_> = String::from_utf8(list.stdout)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(lines.len(), 2);
    let first: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
    let second: serde_json::Value = serde_json::from_str(&lines[1]).unwrap();
    assert_eq!(first["id"], "cli.version");
    assert_eq!(second["id"], "window.input");
}

#[test]
fn shard_output_is_complete_and_deterministic() {
    let suite = TempDir::new().unwrap();
    write_suite(&suite);
    let first = runner()
        .args(["shard", "--suite"])
        .arg(suite.path())
        .args(["--count", "2"])
        .output()
        .unwrap();
    let second = runner()
        .args(["shard", "--count", "2", "--suite"])
        .arg(suite.path())
        .output()
        .unwrap();
    assert!(first.status.success());
    assert_eq!(first.stdout, second.stdout);
    let report: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(report["shards"].as_array().unwrap().len(), 2);
}

#[test]
fn run_shard_executes_only_the_selected_surface_through_real_drivers() {
    let suite = TempDir::new().unwrap();
    write_suite(&suite);
    let evidence = suite.path().join("evidence");
    let output = runner()
        .args(["run-shard", "--suite"])
        .arg(suite.path())
        .args([
            "--count",
            "1",
            "--index",
            "0",
            "--surface",
            "console",
            "--target",
            "windows-x86_64",
            "--app",
            fixture(),
            "--evidence",
        ])
        .arg(&evidence)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "run-shard failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        evidence
            .join("cli.version.windows-x86_64.0.ndjson")
            .is_file()
    );
    assert!(
        !evidence
            .join("window.input.windows-x86_64.0.ndjson")
            .exists()
    );
}

#[test]
fn run_never_silently_skips_a_missing_required_capability() {
    let suite = TempDir::new().unwrap();
    write_suite(&suite);
    let evidence = suite.path().join("evidence");
    let output = runner()
        .args(["run", "--suite"])
        .arg(suite.path())
        .args([
            "--scenario",
            "window.input",
            "--target",
            "windows-x86_64",
            "--evidence",
        ])
        .arg(&evidence)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(4));
    let diagnostic = String::from_utf8(output.stderr).unwrap();
    assert!(diagnostic.contains("real_os_keyboard"));
    let event_log =
        fs::read_to_string(evidence.join("window.input.windows-x86_64.0.ndjson")).unwrap();
    assert!(event_log.contains("infrastructure_failed"));
    assert!(!event_log.contains("skipped"));
}

#[test]
fn run_executes_a_real_entrypoint_and_writes_required_evidence() {
    let suite = TempDir::new().unwrap();
    write_suite(&suite);
    let evidence = suite.path().join("evidence");
    let output = runner()
        .args(["run", "--suite"])
        .arg(suite.path())
        .args([
            "--scenario",
            "cli.version",
            "--target",
            "windows-x86_64",
            "--app",
            fixture(),
            "--evidence",
        ])
        .arg(&evidence)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "runner failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stem = "cli.version.windows-x86_64.0";
    assert!(
        fs::read_to_string(evidence.join(format!("{stem}.stdout")))
            .unwrap()
            .contains("rterm-app")
    );
    assert_eq!(
        fs::read(evidence.join(format!("{stem}.stderr"))).unwrap(),
        b""
    );
    let events = fs::read_to_string(evidence.join(format!("{stem}.ndjson"))).unwrap();
    assert!(events.contains("checkpoint_finished"));
    assert!(events.contains("\"outcome\":\"passed\""));
    let process_tree: serde_json::Value = serde_json::from_slice(
        &fs::read(evidence.join(format!("{stem}.process-tree.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(process_tree["reaped"], true);
}

#[test]
fn failed_run_writes_exactly_one_typed_terminal_event() {
    let suite = TempDir::new().unwrap();
    write_suite(&suite);
    fs::write(
        suite.path().join("scenarios/cli.version.toml"),
        r#"
schema = 1
id = "cli.version"
behavior_ids = ["BHV-CLI-VERSION"]
surface = "console"
fixture = "none"
estimated_cost_ms = 100
actions = [{ type = "type_text", text = "unsupported-by-process-driver" }]
checkpoints = [{ type = "exit_status", code = 0 }]
required_evidence = ["event_log"]
"#,
    )
    .unwrap();
    let evidence = suite.path().join("evidence");
    let output = runner()
        .args(["run", "--suite"])
        .arg(suite.path())
        .args([
            "--scenario",
            "cli.version",
            "--target",
            "windows-x86_64",
            "--app",
            fixture(),
            "--evidence",
        ])
        .arg(&evidence)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let events = fs::read_to_string(evidence.join("cli.version.windows-x86_64.0.ndjson")).unwrap();
    assert_eq!(events.matches("\"event\":\"scenario_finished\"").count(), 1);
    assert!(events.contains("\"outcome\":\"failed\""));
    assert!(!events.contains("\"outcome\":\"passed\""));

    let stem = "cli.version.windows-x86_64.0";
    for suffix in [
        "stdout",
        "stderr",
        "final-snapshot.json",
        "server-trace.json",
        "process-tree.json",
        "compositor.log",
    ] {
        assert!(
            evidence.join(format!("{stem}.{suffix}")).is_file(),
            "failed run did not preserve {suffix} evidence"
        );
    }
    let snapshot: serde_json::Value = serde_json::from_slice(
        &fs::read(evidence.join(format!("{stem}.final-snapshot.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(snapshot["available"], false);
    assert!(
        snapshot["failure"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    let process_tree: serde_json::Value = serde_json::from_slice(
        &fs::read(evidence.join(format!("{stem}.process-tree.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(process_tree["available"], false);
    assert_eq!(process_tree["reaped"], false);
    assert!(process_tree["remaining_owned_processes"].is_null());
}

#[test]
fn missing_required_artifact_marks_run_failed_instead_of_passed() {
    let suite = TempDir::new().unwrap();
    write_suite(&suite);
    let scenario = suite.path().join("scenarios/cli.version.toml");
    let contents = fs::read_to_string(&scenario).unwrap().replace(
        "required_evidence = [\"event_log\"]",
        "required_evidence = [\"event_log\", \"final_snapshot\"]",
    );
    fs::write(scenario, contents).unwrap();
    let evidence = suite.path().join("evidence");
    let output = runner()
        .args(["run", "--suite"])
        .arg(suite.path())
        .args([
            "--scenario",
            "cli.version",
            "--target",
            "windows-x86_64",
            "--app",
            fixture(),
            "--evidence",
        ])
        .arg(&evidence)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let events = fs::read_to_string(evidence.join("cli.version.windows-x86_64.0.ndjson")).unwrap();
    assert_eq!(events.matches("\"event\":\"scenario_finished\"").count(), 1);
    assert!(events.contains("\"outcome\":\"failed\""));
    assert!(!events.contains("\"outcome\":\"passed\""));
}

#[test]
fn missing_application_after_start_still_writes_one_failed_terminal_event() {
    let suite = TempDir::new().unwrap();
    write_suite(&suite);
    let evidence = suite.path().join("evidence");
    let output = runner()
        .args(["run", "--suite"])
        .arg(suite.path())
        .args([
            "--scenario",
            "cli.version",
            "--target",
            "windows-x86_64",
            "--evidence",
        ])
        .arg(&evidence)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let events = fs::read_to_string(evidence.join("cli.version.windows-x86_64.0.ndjson")).unwrap();
    assert_eq!(events.matches("\"event\":\"scenario_finished\"").count(), 1);
    assert!(events.contains("\"outcome\":\"failed\""));
}

#[test]
fn package_smoke_executes_version_and_self_test_from_the_packaged_binary() {
    let suite = TempDir::new().unwrap();
    write_suite(&suite);
    fs::write(
        suite.path().join("scenarios/cli.version.toml"),
        r#"
schema = 1
id = "package.startup-smoke"
behavior_ids = ["BHV-CLI-VERSION"]
surface = "package"
fixture = "packaged_binary"
estimated_cost_ms = 100
actions = [{ type = "finish" }]
checkpoints = [{ type = "exit_status", code = 0 }, { type = "resources_zero" }]
required_evidence = ["event_log", "stdout", "stderr", "process_tree"]
"#,
    )
    .unwrap();
    let evidence = suite.path().join("evidence");
    let output = runner()
        .args(["run", "--suite"])
        .arg(suite.path())
        .args([
            "--scenario",
            "package.startup-smoke",
            "--target",
            "windows-x86_64",
            "--app",
            fixture(),
            "--evidence",
        ])
        .arg(&evidence)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "runner failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout =
        fs::read_to_string(evidence.join("package.startup-smoke.windows-x86_64.0.stdout")).unwrap();
    assert!(stdout.contains("\"version\":\"fixture\""));
    assert!(stdout.contains("\"name\":\"local-pty\",\"ok\":true"));
}

#[test]
fn process_commands_keep_the_action_budget_instead_of_entering_cleanup_early() {
    let source = include_str!("../src/runner.rs");
    let process_driver = source
        .split("fn execute_process_scenario(")
        .nth(1)
        .expect("process driver")
        .split("fn execute_profile_lifecycle_scenario(")
        .next()
        .expect("bounded process driver");

    assert!(
        process_driver.contains("ChildGuard::spawn(command, action_timeout(scenario, started)?)")
    );
    assert!(!process_driver.contains("cap_remaining_timeout(cleanup_timeout"));
}

#[test]
fn run_drives_the_real_local_entrypoint_through_a_pty_fixture() {
    let suite = TempDir::new().unwrap();
    write_suite(&suite);
    fs::write(
        suite.path().join("scenarios/cli.version.toml"),
        r#"
schema = 1
id = "cli.version"
behavior_ids = ["BHV-CLI-VERSION"]
surface = "console"
capabilities = ["real_stdin_pty"]
fixture = "terminal_probe"
estimated_cost_ms = 100
actions = [{ type = "pty_input", bytes_hex = "522d5465726d20e7bb88e7abaf0d0a" }, { type = "finish" }]
checkpoints = [{ type = "terminal_contains", text = "fixture-echo:R-Term 终端" }, { type = "exit_status", code = 0 }, { type = "resources_zero" }]
required_evidence = ["event_log", "stdout", "stderr", "final_snapshot", "process_tree"]
"#,
    )
    .unwrap();
    let evidence = suite.path().join("evidence");
    let output = runner()
        .args(["run", "--suite"])
        .arg(suite.path())
        .args([
            "--scenario",
            "cli.version",
            "--target",
            "windows-x86_64",
            "--app",
            fixture(),
            "--fixture-bin",
            fixture(),
            "--capability",
            "real_stdin_pty",
            "--evidence",
        ])
        .arg(&evidence)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "runner failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree: serde_json::Value = serde_json::from_slice(
        &fs::read(evidence.join("cli.version.windows-x86_64.0.process-tree.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(tree["reaped"], true);
    assert_eq!(tree["reader_joined"], true);
    assert_eq!(tree["master_closed"], true);
}

#[test]
fn runner_executes_the_declared_console_stress_journey() {
    let suite = TempDir::new().unwrap();
    write_suite(&suite);
    fs::write(
        suite.path().join("scenarios/cli.version.toml"),
        r#"
schema = 1
id = "console.stress-cleanup"
behavior_ids = ["BHV-CLI-VERSION"]
surface = "console"
capabilities = ["real_stdin_pty"]
fixture = "terminal_stress"
estimated_cost_ms = 1000
actions = [{ type = "finish" }]
checkpoints = [{ type = "exit_status", code = 0 }, { type = "resources_zero" }]
required_evidence = ["event_log", "stdout", "stderr", "final_snapshot", "process_tree"]
"#,
    )
    .unwrap();
    let evidence = suite.path().join("evidence");
    let output = runner()
        .args(["run", "--suite"])
        .arg(suite.path())
        .args([
            "--scenario",
            "console.stress-cleanup",
            "--target",
            "windows-x86_64",
            "--app",
            fixture(),
            "--fixture-bin",
            fixture(),
            "--capability",
            "real_stdin_pty",
            "--evidence",
        ])
        .arg(&evidence)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "runner failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stem = "console.stress-cleanup.windows-x86_64.0";
    let snapshot: serde_json::Value = serde_json::from_slice(
        &fs::read(evidence.join(format!("{stem}.final-snapshot.json"))).unwrap(),
    )
    .unwrap();
    assert!(snapshot["high_output_bytes"].as_u64().unwrap() >= 1_048_576);
    assert_eq!(snapshot["nonzero_exit_code"], 37);
    assert_eq!(snapshot["synchronized_output_released"], true);
    assert_eq!(snapshot["slow_read_completed"], true);
}

#[test]
fn runner_executes_real_fixture_disconnect_and_reconnect_generations() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let evidence = TempDir::new().unwrap();
    let output = runner()
        .args(["run", "--suite"])
        .arg(root.join("functional-tests"))
        .args([
            "--scenario",
            "console.disconnect-reconnect",
            "--target",
            if cfg!(windows) {
                "windows-x86_64"
            } else if cfg!(target_os = "macos") {
                "macos-aarch64"
            } else {
                "linux-x86_64"
            },
            "--app",
            fixture(),
            "--fixture-bin",
            fixture(),
            "--evidence",
        ])
        .arg(evidence.path())
        .args(["--capability", "real_stdin_pty"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let target = if cfg!(windows) {
        "windows-x86_64"
    } else if cfg!(target_os = "macos") {
        "macos-aarch64"
    } else {
        "linux-x86_64"
    };
    let stem = format!("console.disconnect-reconnect.{target}.0");
    let snapshot: serde_json::Value = serde_json::from_slice(
        &fs::read(evidence.path().join(format!("{stem}.final-snapshot.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(snapshot["generations"], 2);
    let process_tree: serde_json::Value = serde_json::from_slice(
        &fs::read(evidence.path().join(format!("{stem}.process-tree.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(process_tree["remaining_owned_processes"], 0);
    let events = fs::read_to_string(evidence.path().join(format!("{stem}.ndjson"))).unwrap();
    assert!(events.contains("BHV-ACTION-FIXTURE-DISCONNECT"));
    assert!(events.contains("BHV-ACTION-FIXTURE-RECONNECT"));
}

#[test]
fn observer_disconnect_driver_uses_os_input_after_dropping_the_read_channel() {
    let source = include_str!("../src/runner.rs");
    assert!(source.contains("execute_observer_disconnect_scenario"));
    assert!(source.contains("drop(client)"));
    assert!(source.contains("observer channel intentionally disconnected"));
    assert!(source.contains("platform") && source.contains(".execute(action"));
}

#[test]
fn host_terminal_driver_requires_a_real_emulator_and_os_input_marker() {
    let source = include_str!("../src/runner.rs");
    let fixture = include_str!("../src/fixture_main.rs");
    assert!(source.contains("execute_host_terminal_scenario"));
    assert!(source.contains("RTERM_FUNCTIONAL_HOST_TERMINAL"));
    assert!(source.contains("host-terminal-probe"));
    assert!(source.contains("host-terminal-input-ok"));
    assert!(
        source.contains("cmd.exe") && source.contains("start") && source.contains("/wait"),
        "Windows must create the visible host console before launching rterm-app",
    );
    assert!(
        source.contains("HostTerminalChildGuard"),
        "every failure path must reap the host and its descendants",
    );
    assert!(
        source.contains(".then_some(launch.host_title.as_str())")
            && source.contains("PlatformDriverOptions::window_title(window_title)"),
        "OS input must target the fixture's unique OSC title",
    );
    assert!(
        source.matches("absolute_from_current(").count() >= 3,
        "host app, fixture, and marker paths must survive terminal cwd changes",
    );
    assert!(fixture.contains("host-terminal-ready"));
    let ready = source
        .find("wait_for_host_terminal_readiness(")
        .expect("host terminal readiness gate");
    let actions = source
        .find("run_host_terminal_actions(scenario")
        .expect("host terminal input actions");
    assert!(
        ready < actions,
        "host terminal input must wait for fixture readiness"
    );
}

#[test]
fn native_resource_gate_is_observer_backed_not_hard_coded_after_root_exit() {
    let source = include_str!("../src/runner.rs");
    assert!(source.matches("observed_resources_zero(").count() >= 4);
    assert!(source.contains("snapshot.runtime.worker_count == 0"));
    assert!(source.contains("snapshot.runtime.listener_count == 0"));
    assert!(source.contains("snapshot.runtime.child_process_count == 0"));

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manager =
        fs::read_to_string(root.join("crates/rterm-app/src/window_parts/part07.rs")).unwrap();
    let start = manager.find("fn finalize_app_close_at_location(").unwrap();
    let end = manager[start..]
        .find("\n    fn reap_retired_apps(")
        .map(|offset| start + offset)
        .unwrap();
    let close = &manager[start..end];
    let cleanup = close.find("stop_active_runtime()").expect("PTY cleanup");
    let publish = close
        .find("functional_observer::publish")
        .expect("closed resource snapshot");
    assert!(
        cleanup < publish,
        "cleanup must precede the final observer snapshot"
    );
    assert!(close.contains("Duration::from_secs(2)"));
}

#[test]
fn native_fixture_startup_and_stress_work_use_execution_not_cleanup_budgets() {
    let source = include_str!("../src/runner.rs");
    let ready = source
        .split("fn wait_for_native_fixture_ready(")
        .nth(1)
        .and_then(|body| body.split("fn wait_for_observer_change(").next())
        .expect("native readiness helper");
    assert!(ready.contains("startup_timeout(scenario, started)?"));
    assert!(!ready.contains("action_timeout(scenario, started)?"));

    let stress = source
        .split("fn execute_pty_stress_scenario(")
        .nth(1)
        .and_then(|body| body.split("struct PtyStressEvidence").next())
        .expect("PTY stress driver");
    assert!(!stress.contains("cap_remaining_timeout(cleanup_timeout"));
}

#[test]
fn text_input_does_not_claim_that_terminal_echo_must_advance_immediately() {
    let source = include_str!("../src/runner.rs");
    let start = source.find("fn action_must_publish(").unwrap();
    let end = source[start..]
        .find("\nfn action_name(")
        .map(|offset| start + offset)
        .unwrap();
    assert!(!source[start..end].contains("ActionV1::TypeText"));
}

#[test]
fn console_pty_waits_for_the_fixture_before_sending_scenario_input() {
    let source = include_str!("../src/runner.rs");
    let start = source
        .find("fn execute_pty_scenario(")
        .expect("PTY scenario executor");
    let end = source[start..]
        .find("\nfn write_pty_evidence(")
        .map(|offset| start + offset)
        .expect("PTY evidence writer");
    let executor = &source[start..end];

    assert!(executor.contains(".wait_for_output(b\"fixture-ready\")"));
}

#[test]
fn x11_window_discovery_waits_for_the_window_to_be_mapped() {
    let source = include_str!("../src/runner.rs");
    let start = source
        .find("fn discover_x11_window(")
        .expect("X11 discovery helper");
    let end = source[start..]
        .find("\nfn wait_for_observer_change(")
        .map(|offset| start + offset)
        .expect("following observer helper");
    let discovery = &source[start..end];

    assert!(discovery.contains("for _ in 0..100"));
    assert!(discovery.contains("Duration::from_millis(50)"));
    assert!(discovery.contains("--class"));
    assert!(source.contains("const FUNCTIONAL_X11_WINDOW_CLASS: &str = \"rterm-functional\""));
}

#[test]
fn native_window_actions_wait_for_the_fixture_ready_marker() {
    let source = include_str!("../src/runner.rs");
    let start = source
        .find("fn execute_observed_window_scenario(")
        .expect("observed-window executor");
    let end = source[start..]
        .find("\nfn run_observed_window_actions(")
        .map(|offset| start + offset)
        .expect("action executor");
    assert!(source[start..end].contains("wait_for_native_fixture_ready"));
    assert!(source.matches("wait_for_native_fixture_ready(").count() >= 4);
    assert!(source.matches("FUNCTIONAL_X11_WINDOW_CLASS").count() >= 7);
}

#[test]
fn early_native_window_failures_preserve_child_diagnostics() {
    let runner = include_str!("../src/runner.rs");
    let helper = runner
        .split("fn with_window_child_diagnostics(")
        .nth(1)
        .expect("window child diagnostics helper");

    assert!(runner.contains("with_window_child_diagnostics(&error, child, description)"));
    assert!(helper.contains("child.terminate()"));
    assert!(helper.contains("output.stderr"));
    assert!(helper.contains("output.stdout"));
}

#[test]
fn behavior_evidence_is_derived_from_executed_actions_drivers_and_checkpoints() {
    let source = include_str!("../src/runner.rs");
    assert!(!source.contains("for behavior in &scenario.behavior_ids"));
    assert!(!source.contains(".any(|declared| declared.as_ref() == behavior)"));
    for seam in [
        "record_action_behavior",
        "record_checkpoint_behavior",
        "record_driver_behavior",
    ] {
        assert!(source.contains(seam), "missing evidence seam {seam}");
    }
    assert!(source.contains("if result.transport_write_observed()"));
    assert!(!source.contains("if result.terminal_query_responses > 0"));
}

#[test]
fn coverage_command_reads_runtime_artifacts_and_emits_a_machine_report() {
    let suite = TempDir::new().unwrap();
    write_suite(&suite);
    let evidence = suite.path().join("evidence");
    fs::create_dir(&evidence).unwrap();
    fs::write(
        suite.path().join("evidence-map.toml"),
        r#"schema = 1
[[evidence]]
behavior_id = "BHV-WINDOW-INPUT"
source = "libtest"
identity = "window::real_input"
"#,
    )
    .unwrap();
    fs::write(
        evidence.join("window.libtest"),
        "test window::real_input ... ok\n",
    )
    .unwrap();
    fs::write(
        evidence.join("cli.version.windows.0.ndjson"),
        concat!(
            "{\"schema\":1,\"sequence\":1,\"run_id\":{\"scenario_id\":\"cli.version\",\"target\":\"windows\",\"attempt\":0},\"monotonic_ms\":0,\"event\":\"scenario_started\",\"capabilities\":[]}\n",
            "{\"schema\":1,\"sequence\":2,\"run_id\":{\"scenario_id\":\"cli.version\",\"target\":\"windows\",\"attempt\":0},\"monotonic_ms\":0,\"event\":\"behavior_observed\",\"behavior_id\":\"BHV-CLI-VERSION\",\"evidence\":\"asserted\"}\n",
            "{\"schema\":1,\"sequence\":3,\"run_id\":{\"scenario_id\":\"cli.version\",\"target\":\"windows\",\"attempt\":0},\"monotonic_ms\":1,\"event\":\"scenario_finished\",\"outcome\":\"passed\"}\n"
        ),
    )
    .unwrap();
    fs::write(
        evidence.join("window.input.windows.0.ndjson"),
        concat!(
            "{\"schema\":1,\"sequence\":1,\"run_id\":{\"scenario_id\":\"window.input\",\"target\":\"windows\",\"attempt\":0},\"monotonic_ms\":0,\"event\":\"scenario_started\",\"capabilities\":[\"real_os_keyboard\"]}\n",
            "{\"schema\":1,\"sequence\":2,\"run_id\":{\"scenario_id\":\"window.input\",\"target\":\"windows\",\"attempt\":0},\"monotonic_ms\":0,\"event\":\"behavior_observed\",\"behavior_id\":\"BHV-WINDOW-INPUT\",\"evidence\":\"asserted\"}\n",
            "{\"schema\":1,\"sequence\":3,\"run_id\":{\"scenario_id\":\"window.input\",\"target\":\"windows\",\"attempt\":0},\"monotonic_ms\":1,\"event\":\"scenario_finished\",\"outcome\":\"passed\"}\n"
        ),
    )
    .unwrap();

    let output = runner()
        .args(["coverage", "--suite"])
        .arg(suite.path())
        .args(["--map"])
        .arg(suite.path().join("evidence-map.toml"))
        .args(["--evidence-root"])
        .arg(&evidence)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "coverage failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["behaviors_total"], 2);
    assert_eq!(report["behaviors_covered"], 2);
}
