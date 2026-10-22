mod common;

use std::{
    env, fs,
    path::PathBuf,
    sync::{Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

use rterm_test_support::ChildGuard;
#[cfg(target_os = "windows")]
use rterm_test_support::windows::wait_for_owned_window_frame;
use std::process::Command;
use sysinfo::{Pid, System};

const RTERM_APP_EXECUTABLE: &str = env!("CARGO_BIN_EXE_rterm");
static NATIVE_WINDOW_E2E_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn native_window_e2e_presents_ten_frames_from_a_real_pty() {
    let _native_window = native_window_e2e_guard();
    let executable = packaged_or_cargo_app_executable();
    let probe = common::run_ten_frame_native_window(&executable);

    common::assert_ten_frame_native_metrics(&probe);
    assert_eq!(probe.metrics["runtime_api"], "v2-runtime-hub");
}

#[test]
fn native_window_frame_probe_keeps_its_marker_process_alive() {
    let source = include_str!("common/mod.rs");
    let start = source
        .find("pub fn run_ten_frame_native_window_with_log(")
        .expect("logged native window probe");
    let end = source[start..]
        .find("\nfn run_ten_frame_native_window_with_command(")
        .map(|offset| start + offset)
        .expect("native window command helper");

    assert!(source[start..end].contains("platform_marker_command_for_window_frames"));
}

#[cfg(target_os = "windows")]
#[test]
fn scaled_native_window_probes_use_deterministic_present_path() {
    let source = include_str!("common/mod.rs");
    let start = source
        .find("fn run_ten_frame_native_window_with_command(")
        .expect("native window command helper");
    let end = source[start..]
        .find("\npub fn assert_ten_frame_native_metrics(")
        .map(|offset| start + offset)
        .expect("native window metrics assertions");
    let command_helper = &source[start..end];

    assert!(command_helper.contains("cfg!(target_os = \"windows\")"));
    assert!(command_helper.contains("if let Some(scale_factor) = scale_factor"));
    assert!(command_helper.contains("RTERM_TEST_PRESENT_MODE"));
    assert!(command_helper.contains("RTERM_TEST_FORCE_FALLBACK_ADAPTER"));
    assert!(command_helper.contains("RTERM_TEST_NATIVE_SCALE_PRIMARY"));
}

#[test]
#[ignore = "release-native-window scorecard probe"]
fn native_window_release_performance_probe() {
    let _native_window = native_window_e2e_guard();
    let executable = packaged_or_cargo_app_executable();
    let started = Instant::now();
    let probe = common::run_ten_frame_native_window(&executable);
    let elapsed = started.elapsed();

    common::assert_ten_frame_native_metrics(&probe);
    println!(
        "RTERM_NATIVE_RELEASE_PROBE={}",
        serde_json::json!({
            "elapsed_us": elapsed.as_micros(),
            "requested_runtime": "v2",
            "metrics": probe.metrics,
        })
    );
}

#[test]
fn native_window_local_pane_v2_has_the_expected_observable_transcript() {
    let _native_window = native_window_e2e_guard();
    let executable = packaged_or_cargo_app_executable();
    let v2 = common::run_ten_frame_native_window(&executable);
    common::assert_ten_frame_native_metrics(&v2);
    assert_eq!(v2.metrics["runtime_api"], "v2-runtime-hub");
    assert_eq!(v2.metrics["runtime_live_threads"], 0);
}

#[test]
#[ignore = "dedicated native-window runner scenario"]
fn native_window_local_pane_v2_writes_visible_session_log() {
    let _native_window = native_window_e2e_guard();
    let executable = packaged_or_cargo_app_executable();
    let unique = format!("{}-{}", std::process::id(), env!("CARGO_PKG_VERSION"));
    let v2_path = env::temp_dir().join(format!("rterm-task19-v2-{unique}.log"));
    let _ = fs::remove_file(&v2_path);

    let v2 = common::run_ten_frame_native_window_with_log(&executable, None, Some(&v2_path));
    common::assert_ten_frame_native_metrics(&v2);

    let v2_log = fs::read(&v2_path).expect("read V2 session log");
    assert_session_log_matches_pty_linkage(&v2.metrics, &v2_log);
    let _ = fs::remove_file(v2_path);
}

#[test]
fn session_log_contract_compares_the_observed_pty_payload() {
    let observed = b"rterm-e2e|office ?????????";
    let metrics = serde_json::json!({
        "pty_linkage_digest": digest_bytes(observed),
    });
    let log = [
        b"RTERM-LINK-BEGIN|".as_slice(),
        observed,
        b"|RTERM-LINK-END",
    ]
    .concat();

    assert_session_log_matches_pty_linkage(&metrics, &log);
}

fn assert_session_log_matches_pty_linkage(metrics: &serde_json::Value, log: &[u8]) {
    const BEGIN: &[u8] = b"RTERM-LINK-BEGIN|";
    const END: &[u8] = b"|RTERM-LINK-END";

    let begin = log
        .windows(BEGIN.len())
        .position(|window| window == BEGIN)
        .unwrap_or_else(|| panic!("session log omitted PTY linkage start: {log:?}"));
    let payload_start = begin + BEGIN.len();
    let end = log[payload_start..]
        .windows(END.len())
        .position(|window| window == END)
        .map_or_else(
            || panic!("session log omitted PTY linkage end: {log:?}"),
            |offset| payload_start + offset,
        );
    let observed = &log[payload_start..end];
    let expected = metrics["pty_linkage_digest"]
        .as_str()
        .expect("native metrics include the observed PTY linkage digest");

    assert_eq!(
        digest_bytes(observed),
        expected,
        "session log did not preserve the PTY bytes delivered by the platform: {log:?}"
    );
}

fn digest_bytes(bytes: &[u8]) -> String {
    rterm_render_core::terminal_bytes_content_digest(bytes)
        .into_iter()
        .fold(String::new(), |mut encoded, byte| {
            use std::fmt::Write as _;
            write!(encoded, "{byte:02x}").expect("writing to String cannot fail");
            encoded
        })
}

#[test]
fn native_window_v2_reaps_a_hold_open_pty_child() {
    let _native_window = native_window_e2e_guard();
    let executable = packaged_or_cargo_app_executable();
    let marker = format!("RTERM-TASK24-HOLD-V2-{}", std::process::id());
    let path = env::temp_dir().join(format!(
        "rterm-task24-cleanup-v2-{}.log",
        std::process::id()
    ));
    let _ = fs::remove_file(&path);
    let child = hold_open_marker_command(&marker);
    let mut command = Command::new(&executable);
    command
        .args(["-n", "window", "--frames", "60", "--metrics-json", "--log"])
        .arg(&path)
        .arg("--")
        .arg(child.get_program())
        .args(child.get_args())
        .env("RTERM_TEST_DIRECT_GPU_TEXT", "1");
    let guard = ChildGuard::spawn(command, Duration::from_secs(30))
        .expect("spawn frame-limited hold-open window");
    let app_pid = guard.process_id().expect("app process ID");
    let startup_deadline = Instant::now() + Duration::from_secs(20);
    while !fs::read_to_string(&path).is_ok_and(|log| log.contains(&marker)) {
        assert!(
            Instant::now() < startup_deadline,
            "V2 PTY child never wrote its startup marker"
        );
        thread::sleep(Duration::from_millis(25));
    }
    let observe_deadline = Instant::now() + Duration::from_secs(5);
    let child_pid = loop {
        if let Some(pid) = marker_process(&marker, app_pid) {
            break pid;
        }
        assert!(
            Instant::now() < observe_deadline,
            "V2 never spawned the hold-open PTY child"
        );
        thread::sleep(Duration::from_millis(10));
    };
    let output = guard.wait().expect("frame-limited window exits");
    assert!(output.status.success(), "V2 app failed: {output:?}");
    let metrics: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("native metrics JSON");
    assert_eq!(metrics["runtime_api"], "v2-runtime-hub");
    assert_eq!(metrics["runtime_live_threads"], 0);
    let deadline = Instant::now() + Duration::from_secs(5);
    while process_exists(child_pid) && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(25));
    }
    assert!(
        !process_exists(child_pid),
        "V2 left PTY child {child_pid} alive after window cleanup"
    );
    let _ = fs::remove_file(path);
}

#[test]
fn native_window_v2_preserves_nonzero_child_exit_status() {
    let _native_window = native_window_e2e_guard();
    let executable = packaged_or_cargo_app_executable();
    let unique = format!("{}-{}", std::process::id(), env!("CARGO_PKG_VERSION"));
    let marker = format!("RTERM-TASK19-EXIT-{unique}");
    let child = nonzero_exit_marker_command(&marker);
    let log_path = env::temp_dir().join(format!("rterm-task24-exit-v2-{unique}.log"));
    let _ = fs::remove_file(&log_path);
    let mut command = Command::new(&executable);
    command
        .args(["-n", "window", "--metrics-json", "--log"])
        .arg(&log_path)
        .arg("--")
        .arg(child.get_program())
        .args(child.get_args())
        .env("RTERM_TEST_DIRECT_GPU_TEXT", "1");
    let output = ChildGuard::spawn(command, Duration::from_secs(30))
        .expect("spawn nonzero-exit native window")
        .wait()
        .expect("nonzero-exit native window closes");
    assert!(output.status.success(), "V2 app failed: {output:?}");
    let metrics: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("native metrics JSON");
    assert_eq!(metrics["runtime_api"], "v2-runtime-hub");
    assert_eq!(metrics["runtime_live_threads"], 0);
    assert_eq!(
        metrics["last_exit_code"], 7,
        "V2 lost the real child exit status: {output:?}"
    );
    let log = fs::read(&log_path).expect("read nonzero-exit session log");
    let transcript = String::from_utf8_lossy(&log);
    assert!(transcript.contains(&marker));
    let _ = fs::remove_file(log_path);
}

#[cfg(unix)]
#[test]
fn native_window_initial_grid_survives_detected_display_scale() {
    let _native_window = native_window_e2e_guard();
    let log_path = env::temp_dir().join(format!("rterm-initial-grid-{}.log", std::process::id()));
    let _ = fs::remove_file(&log_path);
    let mut command = Command::new(packaged_or_cargo_app_executable());
    command
        .args([
            "-n",
            "--config",
            "initial_cols=120",
            "--config",
            "initial_rows=32",
            "window",
            "--metrics-json",
            "--log",
        ])
        .arg(&log_path)
        .args([
            "--",
            "/bin/sh",
            "-c",
            "sleep 6; printf 'RTERM-GRID-1='; stty size; sleep 1; printf 'RTERM-GRID-2='; stty size; sleep 0.5",
        ]);
    let output = ChildGuard::spawn(command, Duration::from_secs(30))
        .expect("spawn native window with configured grid")
        .wait()
        .expect("configured-grid native window exits");
    assert!(output.status.success(), "native app failed: {output:?}");
    let transcript = fs::read_to_string(&log_path).expect("read actual PTY size");
    for marker in ["RTERM-GRID-1=", "RTERM-GRID-2="] {
        let actual = transcript
            .lines()
            .find_map(|line| line.strip_prefix(marker))
            .unwrap_or_else(|| panic!("real PTY reports {marker} in {transcript:?}"))
            .split_whitespace()
            .collect::<Vec<_>>();
        assert_eq!(actual, ["32", "120"], "transcript: {transcript:?}");
    }
    let _ = fs::remove_file(log_path);
}

fn marker_process(marker: &str, excluded_process_id: u32) -> Option<u32> {
    let mut system = System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    system
        .processes()
        .iter()
        .find_map(|(pid, process)| {
            let process_id = pid.as_u32();
            (process_id != excluded_process_id
                && process
                    .cmd()
                    .iter()
                    .any(|argument| argument.to_string_lossy().contains(marker)))
            .then_some(process_id)
        })
        .or_else(|| {
            system.processes().iter().find_map(|(pid, process)| {
                let name = process.name().to_string_lossy();
                (is_descendant_of(&system, pid.as_u32(), excluded_process_id)
                    && ["cmd.exe", "sh", "sleep", "ping.exe"]
                        .iter()
                        .any(|candidate| name.eq_ignore_ascii_case(candidate)))
                .then_some(pid.as_u32())
            })
        })
}

fn is_descendant_of(system: &System, process_id: u32, ancestor_id: u32) -> bool {
    let mut current = Pid::from_u32(process_id);
    for _ in 0..16 {
        let Some(parent) = system.process(current).and_then(sysinfo::Process::parent) else {
            return false;
        };
        if parent.as_u32() == ancestor_id {
            return true;
        }
        current = parent;
    }
    false
}

fn process_exists(process_id: u32) -> bool {
    let mut system = System::new();
    system.refresh_processes(
        sysinfo::ProcessesToUpdate::Some(&[Pid::from_u32(process_id)]),
        true,
    );
    system.process(Pid::from_u32(process_id)).is_some()
}

#[cfg(target_os = "windows")]
fn hold_open_marker_command(marker: &str) -> Command {
    let mut command = Command::new("cmd.exe");
    command.args([
        "/D",
        "/Q",
        "/C",
        &format!("echo {marker} & ping -n 300 127.0.0.1 >nul"),
    ]);
    command
}

#[cfg(target_os = "windows")]
fn nonzero_exit_marker_command(marker: &str) -> Command {
    let mut command = Command::new("cmd.exe");
    command.args(["/D", "/Q", "/C", &format!("echo {marker} & exit 7")]);
    command
}

#[cfg(not(target_os = "windows"))]
fn hold_open_marker_command(marker: &str) -> Command {
    let mut command = Command::new("sh");
    command.args(["-c", &format!("printf '%s\\n' '{marker}'; sleep 300")]);
    command
}

#[cfg(not(target_os = "windows"))]
fn nonzero_exit_marker_command(marker: &str) -> Command {
    let mut command = Command::new("sh");
    command.args(["-c", &format!("printf '%s\\n' '{marker}'; exit 7")]);
    command
}

#[test]
#[ignore = "dedicated native-window runner scenario"]
fn native_window_e2e_preserves_gpu_text_at_scale_100() {
    assert_native_window_scale(1.0);
}

#[test]
#[ignore = "dedicated native-window runner scenario"]
fn native_window_e2e_preserves_gpu_text_at_scale_125() {
    assert_native_window_scale(1.25);
}

#[test]
#[ignore = "dedicated native-window runner scenario"]
fn native_window_e2e_preserves_gpu_text_at_scale_150() {
    assert_native_window_scale(1.5);
}

#[test]
#[ignore = "dedicated native-window runner scenario"]
fn native_window_e2e_preserves_gpu_text_at_scale_200() {
    assert_native_window_scale(2.0);
}

fn assert_native_window_scale(scale_factor: f64) {
    let _native_window = native_window_e2e_guard();
    let executable = packaged_or_cargo_app_executable();
    let probe = common::run_ten_frame_native_window_at_scale(&executable, Some(scale_factor));
    common::assert_ten_frame_native_metrics(&probe);
    assert_eq!(probe.metrics["runtime_api"], "v2-runtime-hub");
    if cfg!(target_os = "windows") && std::env::var_os("RTERM_TEST_NATIVE_SCALE_PRIMARY").is_none()
    {
        assert_eq!(probe.metrics["gpu_software_adapter"], true);
    }
}

#[cfg(target_os = "windows")]
#[test]
fn native_window_e2e_uses_borderless_integrated_titlebar() {
    let _native_window = native_window_e2e_guard();
    let executable = packaged_or_cargo_app_executable();
    let mut command = Command::new(&executable);
    command.args(["--skip-config", "-n", "window"]);
    let process = ChildGuard::spawn(command, Duration::from_secs(35))
        .expect("spawn native window decoration fixture");
    let process_id = process.process_id().expect("native window process ID");
    let observation =
        wait_for_owned_window_frame(process_id, Instant::now() + Duration::from_secs(30))
            .expect("observe native window frame");

    assert!(
        observation.has_borderless_client_area(),
        "integrated titlebar window retained a native frame inset: {observation:#?}"
    );
}

fn native_window_e2e_guard() -> MutexGuard<'static, ()> {
    NATIVE_WINDOW_E2E_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn packaged_or_cargo_app_executable() -> PathBuf {
    env::var_os("RTERM_TEST_APP_EXECUTABLE")
        .map_or_else(|| PathBuf::from(RTERM_APP_EXECUTABLE), PathBuf::from)
}

#[test]
fn active_workflow_contract_uses_current_rterm_jobs() {
    let workflow = read_repo_file(".github/workflows/windows.yml");

    assert!(workflow.contains("name: R-Term independence"));
    assert!(workflow.contains("permissions:\n  contents: read"));
    assert!(workflow.contains(
        "cargo test --locked -p rterm-app --no-default-features --test feature_boundary"
    ));
    assert!(workflow.contains("cargo test --locked -p rterm-ssh --all-targets"));
    assert!(workflow.contains("cargo test --locked -p rterm-app --test native_window_e2e"));
    assert!(!workflow.contains("docs/trial/legacy-workflows"));
    assert!(!workflow.contains("rssh-ssh"));
}

#[test]
fn dedicated_native_runners_own_heavy_window_scenarios() {
    let windows = read_repo_file("scripts/ci/run-native-window.ps1");
    let unix = read_repo_file("scripts/ci/run-native-window.sh");

    for (runner, contents) in [("Windows", windows), ("Unix", unix)] {
        let normalized = contents.replace(['"', ','], "");
        assert!(
            normalized.contains("--bin rterm --test openssh_loopback --test native_window_e2e"),
            "{runner} must prebuild only the product and the integration harnesses it runs"
        );
        assert!(
            !normalized.contains("-p rterm-app --all-targets"),
            "{runner} must not compile unrelated app unit tests for an exact integration scenario"
        );
        for scenario in [
            "native_window_e2e_preserves_gpu_text_at_scale_100",
            "native_window_e2e_preserves_gpu_text_at_scale_125",
            "native_window_e2e_preserves_gpu_text_at_scale_150",
            "native_window_e2e_preserves_gpu_text_at_scale_200",
            "native_window_local_pane_v2_writes_visible_session_log",
        ] {
            assert!(
                contents.contains(scenario),
                "{runner} native runner omitted heavy scenario {scenario}"
            );
        }
        assert!(
            contents.contains("--ignored"),
            "{runner} native runner must opt in to heavy ignored scenarios"
        );
    }
}

#[test]
fn windows_native_runner_retries_each_heavy_scenario_once_after_bounded_cleanup() {
    let windows = read_repo_file("scripts/ci/run-native-window.ps1");

    for contract in [
        "$nativeScenarioAttempts = 2",
        "$attempt -le $nativeScenarioAttempts",
        "$isScaledScenario = $scenario.StartsWith(",
        "$isScaledScenario -and $attempt -eq 1",
        "$isScaledScenario -and $attempt -eq 2",
        "$env:RTERM_TEST_NATIVE_SCALE_PRIMARY = \"1\"",
        "catch",
        "$attempt -ge $nativeScenarioAttempts",
        "throw",
        "retrying after bounded cleanup",
    ] {
        assert!(
            windows.contains(contract),
            "Windows native runner is missing bounded retry contract {contract}"
        );
    }
}

#[test]
fn linux_display_and_strict_script_contracts_are_explicit() {
    let workflow = read_repo_file(".github/workflows/windows.yml");
    let powershell = format!(
        "{}\n{}",
        read_repo_file("scripts/ci/run-native-window.ps1"),
        read_repo_file("scripts/ci/process-harness.ps1")
    );
    let shell = format!(
        "{}\n{}",
        read_repo_file("scripts/ci/run-native-window.sh"),
        read_repo_file("scripts/ci/process-harness.sh")
    );

    assert!(workflow.contains("libxkbcommon-x11-0"));
    assert!(workflow.contains("xvfb-run -a env -u WAYLAND_DISPLAY"));
    assert!(workflow.contains("openssh-client"));
    assert!(!workflow.contains("docs/trial/legacy-workflows"));
    for contract in [
        "$ErrorActionPreference = \"Stop\"",
        "$PSNativeCommandUseErrorActionPreference = $true",
        "WaitForExit",
        "job.Dispose()",
        "Complete-StreamBeforeDeadline",
        "Assert-WindowsCommandLineQuoting",
        "HarnessSelfTest",
        "RTermCiJobObject",
        "KILL_ON_JOB_CLOSE",
        "CreateProcessW",
        "CREATE_SUSPENDED",
        "ResumeThread",
        "FailAssignmentForTest",
        "timeout-stdout-marker",
        "timeout-stderr-marker",
        "C:\\path with space\\",
        "--locked",
        "--all-targets",
        "OpenSSH client probe",
        "-FilePath \"ssh\"",
        "@(\"-V\")",
        "\"rterm-ssh\", \"--all-targets\"",
        "\"openssh_loopback\"",
        "RTERM_REQUIRE_OPENSSH = \"1\"",
        "version",
        "--json",
    ] {
        assert!(
            powershell.contains(contract),
            "PowerShell script is missing {contract}"
        );
    }
    assert!(!powershell.contains("GetAwaiter().GetResult"));
    for contract in [
        "set -euo pipefail",
        "--harness-self-test",
        "start_new_session=True",
        "os.killpg(process_group, 0)",
        "signal.SIGKILL",
        "leader-exit process-group self-test",
        "process-group timeout self-test",
        "timeout-stdout-marker",
        "timeout-stderr-marker",
        "timeout",
        "trap",
        "kill",
        "--locked",
        "--all-targets",
        "ssh -V",
        "rterm-ssh --all-targets",
        "openssh_loopback",
        "RTERM_REQUIRE_OPENSSH=1",
        "version --json",
    ] {
        assert!(
            shell.contains(contract),
            "shell script is missing {contract}"
        );
    }
    assert!(powershell.contains("process-harness.ps1"));
    assert!(shell.contains("source \"$script_dir/process-harness.sh\""));
}

fn workflow_job<'a>(workflow: &'a str, job_name: &str) -> Option<&'a str> {
    let marker = format!("\n  {job_name}:\n");
    let start = workflow.find(&marker)? + 1;
    let after_header = start + marker.len() - 1;
    let remainder = &workflow[after_header..];
    let end = remainder
        .match_indices('\n')
        .find_map(|(offset, _)| {
            let next_line = &remainder[offset + 1..];
            let line = next_line.lines().next()?;
            (line.starts_with("  ") && !line.starts_with("   ") && line.ends_with(':'))
                .then_some(after_header + offset)
        })
        .unwrap_or(workflow.len());
    Some(&workflow[start..end])
}

#[test]
fn linux_openssh_contract_is_scoped_to_the_native_e2e_job() {
    let workflow = r"
jobs:
  quality:
    steps:
      - name: Install Linux native E2E dependencies
  native-terminal-e2e:
    steps:
      - name: Native E2E with Xvfb/X11
";

    let job = workflow_job(workflow, "native-terminal-e2e").expect("native E2E job");
    assert!(!job.contains("Install Linux native E2E dependencies"));
}

#[test]
fn workflow_contract_normalizes_windows_line_endings() {
    assert_eq!(
        normalize_line_endings("permissions:\r\n  contents: read\r\n"),
        "permissions:\n  contents: read\n"
    );
}

fn read_repo_file(path: &str) -> String {
    let path = repo_root().join(path);
    let contents = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    normalize_line_endings(&contents)
}

fn normalize_line_endings(contents: &str) -> String {
    contents.replace("\r\n", "\n").replace('\r', "\n")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root above rterm-app")
        .to_owned()
}
