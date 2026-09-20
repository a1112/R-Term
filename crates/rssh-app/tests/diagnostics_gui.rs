#![cfg(windows)]

use std::{
    collections::HashSet,
    io::{Read, Write},
    process::{Command, Stdio},
    sync::{Mutex, MutexGuard, mpsc},
    thread,
    time::{Duration, Instant},
};

use rssh_test_support::ChildGuard;

const RSSH_APP_EXECUTABLE: &str = env!("CARGO_BIN_EXE_rterm");
const MARKER_PREFIX: &str = "rssh_diagnostic ";
static DIAGNOSTIC_GUI_TEST_LOCK: Mutex<()> = Mutex::new(());

fn diagnostic_gui_test_lock() -> MutexGuard<'static, ()> {
    DIAGNOSTIC_GUI_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[test]
fn diagnostics_marker_empty_window_emits_readiness_without_starting_a_transport() {
    let _test_lock = diagnostic_gui_test_lock();
    let run_id = format!("empty-window-{}", std::process::id());
    let hold_ms = 100_u64;
    let started = Instant::now();
    let mut command = Command::new(RSSH_APP_EXECUTABLE);
    command.args([
        "diagnostic-gui",
        "--run-id",
        run_id.as_str(),
        "--scenario",
        "empty-window",
        "--hold-ms",
        &hold_ms.to_string(),
        "--renderer",
        "cpu",
        "--cols",
        "80",
        "--rows",
        "24",
    ]);

    let guard =
        ChildGuard::spawn(command, Duration::from_secs(20)).expect("spawn empty-window diagnostic");
    let process_id = guard.process_id().expect("diagnostic process ID");
    let output = guard
        .wait()
        .expect("empty-window diagnostic exits within its bounded hold");
    let elapsed = started.elapsed();
    let stdout = String::from_utf8(output.stdout).expect("diagnostic stdout is UTF-8");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "empty-window diagnostic failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let records = stdout
        .lines()
        .filter_map(|line| line.strip_prefix(MARKER_PREFIX))
        .map(|json| serde_json::from_str::<serde_json::Value>(json).expect("valid v2 marker JSON"))
        .collect::<Vec<_>>();
    assert!(
        !records.is_empty(),
        "diagnostic emitted no markers: {stdout}"
    );
    assert!(
        records
            .iter()
            .all(|record| record["schema"] == "rssh.diagnostics/v2"
                && record["run_id"] == run_id
                && record["pid"] == process_id
                && record["scenario"] == "empty_window"),
        "marker identity changed within the run: {records:#?}"
    );

    let kinds = records
        .iter()
        .filter_map(|record| record["kind"].as_str())
        .collect::<HashSet<_>>();
    for required in [
        "process_started",
        "window_created",
        "first_present",
        "config_ready",
        "scenario_ready",
        "process_exited",
    ] {
        assert!(kinds.contains(required), "missing {required}: {records:#?}");
    }
    assert!(!kinds.contains("transport_started"));
    assert!(!kinds.contains("transport_ready"));
    let first_present = records
        .iter()
        .find(|record| record["kind"] == "first_present")
        .expect("first_present marker");
    assert!(
        first_present["visible_cell_count"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "first present did not prove a non-empty frame: {first_present:#}"
    );
    let scenario_ready_ms = records
        .iter()
        .find(|record| record["kind"] == "scenario_ready")
        .and_then(|record| record["elapsed_ms"].as_u64())
        .expect("scenario_ready elapsed time");
    let process_exited_ms = records
        .iter()
        .find(|record| record["kind"] == "process_exited")
        .and_then(|record| record["elapsed_ms"].as_u64())
        .expect("process_exited elapsed time");
    assert!(
        process_exited_ms.saturating_sub(scenario_ready_ms) >= hold_ms,
        "hold began before scenario readiness: {records:#?}"
    );
    assert!(
        elapsed >= Duration::from_millis(hold_ms),
        "bounded hold ended too early: {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "bounded hold did not terminate promptly: {elapsed:?}"
    );
}

#[test]
fn diagnostics_marker_gpu_fallback_reports_cpu_as_the_final_renderer() {
    let _test_lock = diagnostic_gui_test_lock();
    let run_id = format!("gpu-fallback-{}", std::process::id());
    let mut command = Command::new(RSSH_APP_EXECUTABLE);
    command
        .args([
            "diagnostic-gui",
            "--run-id",
            run_id.as_str(),
            "--scenario",
            "empty-window",
            "--hold-ms",
            "300",
            "--renderer",
            "auto",
        ])
        .env("RSSH_TEST_DEFERRED_GPU_INIT_FAILURE", "1");
    let output = ChildGuard::spawn(command, Duration::from_secs(20))
        .expect("spawn forced GPU fallback diagnostic")
        .wait()
        .expect("forced GPU fallback diagnostic exits");
    let stdout = String::from_utf8(output.stdout).expect("diagnostic stdout is UTF-8");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let process_exited = stdout
        .lines()
        .filter_map(|line| line.strip_prefix(MARKER_PREFIX))
        .map(|json| serde_json::from_str::<serde_json::Value>(json).expect("valid marker JSON"))
        .find(|record| record["kind"] == "process_exited")
        .expect("process_exited marker");
    assert_eq!(process_exited["renderer"], "cpu", "{process_exited:#}");
    assert_eq!(
        process_exited["connection_state"], "not_started",
        "{process_exited:#}"
    );
    assert!(
        !stdout.contains("\"kind\":\"gpu_ready\""),
        "GPU fallback emitted false readiness: {stdout}"
    );
}

#[test]
fn diagnostics_marker_gpu_mode_reports_ready_after_a_real_present() {
    let _test_lock = diagnostic_gui_test_lock();
    let run_id = format!("gpu-ready-{}", std::process::id());
    let mut command = Command::new(RSSH_APP_EXECUTABLE);
    command.args([
        "diagnostic-gui",
        "--run-id",
        run_id.as_str(),
        "--scenario",
        "empty-window",
        "--hold-ms",
        "100",
        "--renderer",
        "gpu",
    ]);
    let output = ChildGuard::spawn(command, Duration::from_secs(20))
        .expect("spawn GPU diagnostic")
        .wait()
        .expect("GPU diagnostic exits");
    let stdout = String::from_utf8(output.stdout).expect("diagnostic stdout is UTF-8");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let records = stdout
        .lines()
        .filter_map(|line| line.strip_prefix(MARKER_PREFIX))
        .map(|json| serde_json::from_str::<serde_json::Value>(json).expect("valid marker JSON"))
        .collect::<Vec<_>>();
    let first_present = records
        .iter()
        .find(|record| record["kind"] == "first_present")
        .expect("first_present marker");
    let gpu_ready = records
        .iter()
        .find(|record| record["kind"] == "gpu_ready")
        .expect("gpu_ready marker after successful GPU present");
    let process_exited = records
        .iter()
        .find(|record| record["kind"] == "process_exited")
        .expect("process_exited marker");
    assert_eq!(first_present["renderer"], "gpu");
    assert_eq!(gpu_ready["renderer"], "gpu");
    assert_eq!(process_exited["renderer"], "gpu");
    assert!(
        gpu_ready["elapsed_ms"].as_u64() >= first_present["elapsed_ms"].as_u64(),
        "GPU readiness preceded the real present: {records:#?}"
    );
}

#[test]
fn ordinary_commands_emit_no_v2_diagnostic_markers() {
    let output = Command::new(RSSH_APP_EXECUTABLE)
        .args(["version", "--json"])
        .output()
        .expect("run ordinary version command");
    assert!(output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains(MARKER_PREFIX));
    assert!(!String::from_utf8_lossy(&output.stderr).contains(MARKER_PREFIX));
}

#[test]
fn diagnostic_ssh1_without_fixture_coordinates_is_rejected_before_opening_a_window() {
    let _test_lock = diagnostic_gui_test_lock();
    let mut command = Command::new(RSSH_APP_EXECUTABLE);
    command.args([
        "diagnostic-gui",
        "--run-id",
        "premature-ssh1",
        "--scenario",
        "ssh1",
        "--hold-ms",
        "1",
        "--renderer",
        "cpu",
    ]);
    let output = ChildGuard::spawn(command, Duration::from_secs(10))
        .expect("spawn incomplete ssh1 diagnostic")
        .wait()
        .expect("incomplete ssh1 must fail without opening a window");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("ssh1 diagnostic requires --ssh-host"),
        "unexpected error: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn attribution_stage_cli_reaches_the_private_owner_path() {
    let _test_lock = diagnostic_gui_test_lock();
    let run_id = format!("attribution-cpu-window-{}", std::process::id());
    let mut command = Command::new(RSSH_APP_EXECUTABLE);
    command.args([
        "diagnostic-gui",
        "--run-id",
        run_id.as_str(),
        "--scenario",
        "empty-window",
        "--hold-ms",
        "1",
        "--renderer",
        "cpu",
        "--attribution-stage",
        "cpu-window",
    ]);

    let output = ChildGuard::spawn(command, Duration::from_secs(20))
        .expect("spawn cpu-window attribution diagnostic")
        .wait()
        .expect("cpu-window attribution diagnostic exits within its bounded hold");
    let stdout = String::from_utf8(output.stdout).expect("diagnostic stdout is UTF-8");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "cpu-window attribution diagnostic failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let ready = stdout
        .lines()
        .filter_map(|line| line.strip_prefix(MARKER_PREFIX))
        .map(|json| serde_json::from_str::<serde_json::Value>(json).expect("valid marker JSON"))
        .filter(|record| record["kind"] == "attribution_stage_ready")
        .collect::<Vec<_>>();
    assert_eq!(
        ready.len(),
        1,
        "expected exactly one owner marker: {stdout}"
    );
    assert_eq!(ready[0]["requested_stage"], "cpu-window");
    assert_eq!(ready[0]["final_stage"], "cpu-window");
    assert_eq!(
        ready[0]["resource_summary_schema"],
        "rssh.project-owned-resources/v1"
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the integration test keeps the owner lifetime and shutdown assertions together"
)]
fn attribution_stage_owner_ready_precedes_hold_and_owners_live_until_shutdown() {
    let _test_lock = diagnostic_gui_test_lock();
    let run_id = format!("attribution-owner-hold-{}", std::process::id());
    let mut child = Command::new(RSSH_APP_EXECUTABLE)
        .args([
            "diagnostic-gui",
            "--run-id",
            run_id.as_str(),
            "--scenario",
            "empty-window",
            "--hold-ms",
            "10000",
            "--renderer",
            "cpu",
            "--attribution-stage",
            "cpu-window",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn attribution owner hold diagnostic");
    let stdout = child.stdout.take().expect("piped stdout");
    let (sender, receiver) = mpsc::channel();
    let stdout_thread = thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        for line in std::io::BufRead::lines(reader) {
            if sender.send(line.expect("read marker line")).is_err() {
                break;
            }
        }
    });
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut records = Vec::new();
    loop {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            drop(receiver);
            let _ = stdout_thread.join();
            panic!("owner readiness timed out");
        }
        let line = match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => line,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let status = child.wait().expect("reap early diagnostic exit");
                let _ = stdout_thread.join();
                panic!("diagnostic exited before owner readiness: {status}");
            }
        };
        let Some(json) = line.strip_prefix(MARKER_PREFIX) else {
            continue;
        };
        let record: serde_json::Value = serde_json::from_str(json).expect("valid marker JSON");
        let kind = record["kind"].as_str().expect("typed marker kind");
        if matches!(
            kind,
            "scenario_ready"
                | "config_ready"
                | "transport_started"
                | "transport_ready"
                | "gpu_ready"
                | "font_ownership_ready"
        ) {
            let _ = child.kill();
            let _ = child.wait();
            drop(receiver);
            let _ = stdout_thread.join();
            panic!("product/later marker preceded attribution owner readiness: {record:#}");
        }
        let is_ready = kind == "attribution_stage_ready";
        records.push(record);
        if is_ready {
            break;
        }
    }
    assert!(
        child.try_wait().unwrap().is_none(),
        "owner dropped at readiness"
    );
    thread::sleep(Duration::from_millis(200));
    assert!(
        child.try_wait().unwrap().is_none(),
        "owner process exited before launcher shutdown"
    );
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(b"shutdown\n")
        .expect("request owner shutdown");
    let exit_deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll owner shutdown") {
            break status;
        }
        assert!(Instant::now() < exit_deadline, "owner ignored shutdown");
        thread::sleep(Duration::from_millis(20));
    };
    stdout_thread.join().expect("stdout reader exits");
    assert!(status.success(), "attribution owner exited unsuccessfully");
    assert_eq!(
        records
            .iter()
            .filter(|record| record["kind"] == "attribution_stage_ready")
            .count(),
        1
    );
}

#[test]
fn diagnostics_marker_launcher_shutdown_ends_the_hold_early() {
    let _test_lock = diagnostic_gui_test_lock();
    let run_id = format!("shutdown-{}", std::process::id());
    let mut child = Command::new(RSSH_APP_EXECUTABLE)
        .args([
            "diagnostic-gui",
            "--run-id",
            run_id.as_str(),
            "--scenario",
            "empty-window",
            "--hold-ms",
            "3000",
            "--renderer",
            "cpu",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn diagnostic with launcher shutdown channel");
    let started = Instant::now();
    thread::sleep(Duration::from_millis(500));
    child
        .stdin
        .take()
        .expect("piped diagnostic stdin")
        .write_all(b"shutdown\n")
        .expect("request graceful diagnostic shutdown");

    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll diagnostic shutdown") {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("diagnostic ignored launcher shutdown");
        }
        thread::sleep(Duration::from_millis(20));
    };
    let elapsed = started.elapsed();
    let mut stdout = String::new();
    child
        .stdout
        .take()
        .expect("piped diagnostic stdout")
        .read_to_string(&mut stdout)
        .expect("read diagnostic stdout");
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("piped diagnostic stderr")
        .read_to_string(&mut stderr)
        .expect("read diagnostic stderr");

    assert!(status.success(), "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        elapsed < Duration::from_millis(1500),
        "launcher shutdown waited for the full hold: {elapsed:?}\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(stdout.contains("\"kind\":\"scenario_ready\""));
    assert!(stdout.contains("\"kind\":\"process_exited\""));
}
