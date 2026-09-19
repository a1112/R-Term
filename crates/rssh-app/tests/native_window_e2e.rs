mod common;

use std::{
    env, fs,
    path::PathBuf,
    sync::{Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

use rssh_test_support::ChildGuard;
#[cfg(target_os = "windows")]
use rssh_test_support::windows::wait_for_owned_window_frame;
use std::process::Command;
use sysinfo::{Pid, System};

const RSSH_APP_EXECUTABLE: &str = env!("CARGO_BIN_EXE_rssh-app");
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
    assert!(command_helper.contains("RSSH_TEST_PRESENT_MODE"));
    assert!(command_helper.contains("RSSH_TEST_FORCE_FALLBACK_ADAPTER"));
    assert!(command_helper.contains("RSSH_TEST_NATIVE_SCALE_PRIMARY"));
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
        "RSSH_NATIVE_RELEASE_PROBE={}",
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
    let v2_path = env::temp_dir().join(format!("rssh-task19-v2-{unique}.log"));
    let _ = fs::remove_file(&v2_path);

    let v2 = common::run_ten_frame_native_window_with_log(&executable, None, Some(&v2_path));
    common::assert_ten_frame_native_metrics(&v2);

    let v2_log = fs::read(&v2_path).expect("read V2 session log");
    assert_session_log_matches_pty_linkage(&v2.metrics, &v2_log);
    let _ = fs::remove_file(v2_path);
}

#[test]
fn session_log_contract_compares_the_observed_pty_payload() {
    let observed = b"rssh-e2e|office ?????????";
    let metrics = serde_json::json!({
        "pty_linkage_digest": digest_bytes(observed),
    });
    let log = [b"RSSH-LINK-BEGIN|".as_slice(), observed, b"|RSSH-LINK-END"].concat();

    assert_session_log_matches_pty_linkage(&metrics, &log);
}

fn assert_session_log_matches_pty_linkage(metrics: &serde_json::Value, log: &[u8]) {
    const BEGIN: &[u8] = b"RSSH-LINK-BEGIN|";
    const END: &[u8] = b"|RSSH-LINK-END";

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
    let marker = format!("RSSH-TASK24-HOLD-V2-{}", std::process::id());
    let path = env::temp_dir().join(format!("rssh-task24-cleanup-v2-{}.log", std::process::id()));
    let _ = fs::remove_file(&path);
    let child = hold_open_marker_command(&marker);
    let mut command = Command::new(&executable);
    command
        .args(["-n", "window", "--frames", "60", "--metrics-json", "--log"])
        .arg(&path)
        .arg("--")
        .arg(child.get_program())
        .args(child.get_args())
        .env("RSSH_TEST_DIRECT_GPU_TEXT", "1");
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
    let marker = format!("RSSH-TASK19-EXIT-{unique}");
    let child = nonzero_exit_marker_command(&marker);
    let log_path = env::temp_dir().join(format!("rssh-task24-exit-v2-{unique}.log"));
    let _ = fs::remove_file(&log_path);
    let mut command = Command::new(&executable);
    command
        .args(["-n", "window", "--metrics-json", "--log"])
        .arg(&log_path)
        .arg("--")
        .arg(child.get_program())
        .args(child.get_args())
        .env("RSSH_TEST_DIRECT_GPU_TEXT", "1");
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
    if cfg!(target_os = "windows") && std::env::var_os("RSSH_TEST_NATIVE_SCALE_PRIMARY").is_none() {
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
    env::var_os("RSSH_TEST_APP_EXECUTABLE")
        .map_or_else(|| PathBuf::from(RSSH_APP_EXECUTABLE), PathBuf::from)
}

#[test]
fn workflow_contract_has_exact_pr_and_supplemental_runner_sets() {
    let ci = read_repo_file("docs/trial/legacy-workflows/ci.yml");
    let nightly = read_repo_file("docs/trial/legacy-workflows/nightly.yml");

    assert_eq!(
        runner_labels(&ci),
        ["windows-2025", "ubuntu-24.04", "macos-15"]
    );
    assert_eq!(
        runner_labels(&nightly),
        ["windows-11-arm", "ubuntu-24.04-arm", "macos-15-intel"]
    );
    assert!(ci.contains("permissions:\n  contents: read"));
    assert!(nightly.contains("permissions:\n  contents: read"));
    assert!(nightly.contains("tags:\n      - \"v*\""));
    for workflow in [&ci, &nightly] {
        assert_action_refs_are_pinned(workflow);
        assert!(workflow.contains("version --json"));
        assert!(workflow.contains("pty_backend"));
        assert!(workflow.contains("target"));
        assert!(workflow.contains("run-native-window.ps1"));
        assert!(workflow.contains("run-native-window.sh"));
    }
}

#[test]
fn dedicated_native_runners_own_heavy_window_scenarios() {
    let windows = read_repo_file("scripts/ci/run-native-window.ps1");
    let unix = read_repo_file("scripts/ci/run-native-window.sh");

    for (runner, contents) in [("Windows", windows), ("Unix", unix)] {
        let normalized = contents.replace(['"', ','], "");
        assert!(
            normalized.contains("--bin rssh-app --test openssh_loopback --test native_window_e2e"),
            "{runner} must prebuild only the product and the integration harnesses it runs"
        );
        assert!(
            !normalized.contains("-p rssh-app --all-targets"),
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
        "$env:RSSH_TEST_NATIVE_SCALE_PRIMARY = \"1\"",
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
    let ci = read_repo_file("docs/trial/legacy-workflows/ci.yml");
    let nightly = read_repo_file("docs/trial/legacy-workflows/nightly.yml");
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

    assert_linux_openssh_server_contract([("CI", &ci), ("nightly", &nightly)]);
    for workflow in [&ci, &nightly] {
        assert!(
            workflow.contains("libxkbcommon-x11-0"),
            "missing dynamically loaded X11 keyboard library"
        );
        assert!(workflow.contains("xvfb-run"), "missing X11 native E2E");
        assert!(workflow.contains("weston"), "missing Wayland native E2E");
        assert!(
            workflow.contains("XDG_RUNTIME_DIR"),
            "missing Wayland runtime"
        );
        assert!(workflow.contains("phase_status=$?"));
        assert!(workflow.contains("wait \"$weston_pid\" || true"));
    }
    for contract in [
        "$ErrorActionPreference = \"Stop\"",
        "$PSNativeCommandUseErrorActionPreference = $true",
        "WaitForExit",
        "job.Dispose()",
        "Complete-StreamBeforeDeadline",
        "Assert-WindowsCommandLineQuoting",
        "HarnessSelfTest",
        "RsshCiJobObject",
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
        "\"rssh-ssh\", \"--all-targets\"",
        "\"openssh_loopback\"",
        "RSSH_REQUIRE_OPENSSH = \"1\"",
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
        "rssh-ssh --all-targets",
        "openssh_loopback",
        "RSSH_REQUIRE_OPENSSH=1",
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

fn assert_linux_openssh_server_contract(workflows: [(&str, &str); 2]) {
    let install_marker = "- name: Install Linux native E2E dependencies";
    for (name, workflow) in workflows {
        let native_e2e_job = workflow_job(workflow, "native-terminal-e2e")
            .unwrap_or_else(|| panic!("{name} is missing the native-terminal-e2e job"));
        let install_start = native_e2e_job
            .find(install_marker)
            .unwrap_or_else(|| panic!("{name} is missing the Linux dependency step"));
        let after_marker = &native_e2e_job[install_start + install_marker.len()..];
        let install_end = after_marker
            .find("\n      - name: ")
            .map_or(native_e2e_job.len(), |offset| {
                install_start + install_marker.len() + offset
            });
        let install_step = &native_e2e_job[install_start..install_end];

        assert!(install_step.contains("if: runner.os == 'Linux'"));
        for package in ["openssh-client", "openssh-server", "xvfb", "weston"] {
            assert!(
                install_step.contains(package),
                "{name} Linux dependency step is missing {package}"
            );
        }
        for service_guard in [
            "policy-rc.d",
            "exit 101",
            "trap restore_policy_rc EXIT",
            "sudo cp -a \"$backup_path\" \"$policy_path\"",
            "sudo chmod 0755 \"$policy_path\"",
        ] {
            assert!(
                install_step.contains(service_guard),
                "{name} Linux dependency step can start system ssh service: missing {service_guard}"
            );
        }
        let mut previous = None;
        for ordered_marker in [
            "sudo cp -a \"$policy_path\" \"$backup_path\"",
            "trap restore_policy_rc EXIT",
            "sudo rm -f -- \"$policy_path\"",
            "sudo tee \"$policy_path\"",
            "apt-get install --yes",
        ] {
            let position = install_step.find(ordered_marker).unwrap_or_else(|| {
                panic!("{name} Linux dependency step is missing {ordered_marker}")
            });
            if let Some(previous) = previous {
                assert!(
                    previous < position,
                    "{name} Linux dependency step orders {ordered_marker} before its prerequisite"
                );
            }
            previous = Some(position);
        }
        for e2e_step in [
            "- name: Native E2E with Xvfb/X11",
            "- name: Native E2E with Weston/Wayland",
        ] {
            let e2e_start = native_e2e_job
                .find(e2e_step)
                .unwrap_or_else(|| panic!("{name} is missing {e2e_step}"));
            assert!(
                install_start < e2e_start,
                "{name} Linux dependencies must be installed before {e2e_step}"
            );
        }
    }

    let native_ssh = read_repo_file("crates/rssh-ssh/tests/loopback_native.rs");
    for contract in [
        concat!(
            "#[cfg(target_os = \"linux\")]\n",
            "#[test]\n",
            "fn native_client_interoperates_with_an_isolated_real_openssh_sshd()"
        ),
        "(\"sshd\", \"-V\")",
        "required Linux OpenSSH fixture tool {tool} missing",
    ] {
        assert!(
            native_ssh.contains(contract),
            "required Linux real-sshd probe is missing {contract}"
        );
    }
    let function_start = native_ssh
        .find("fn native_client_interoperates_with_an_isolated_real_openssh_sshd()")
        .expect("required Linux real-sshd test function");
    let attribute_start = native_ssh[..function_start]
        .rfind("\n\n")
        .map_or(0, |offset| offset + 2);
    let attributes = &native_ssh[attribute_start..function_start];
    assert!(
        !attributes.lines().any(|line| {
            let attribute = line.trim_start();
            attribute.starts_with("#[") && attribute.contains("ignore")
        }),
        "required Linux real-sshd probe must not be ignored"
    );
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

fn runner_labels(workflow: &str) -> Vec<&str> {
    workflow
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- runner: "))
        .collect()
}

fn assert_action_refs_are_pinned(workflow: &str) {
    for action in workflow
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- uses: "))
    {
        let revision = action
            .split_once('@')
            .unwrap_or_else(|| panic!("action has no revision: {action}"))
            .1
            .split_whitespace()
            .next()
            .expect("action revision after @");
        assert_eq!(
            revision.len(),
            40,
            "action is not pinned to a full SHA: {action}"
        );
        assert!(
            revision.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "action revision is not hexadecimal: {action}"
        );
        assert!(
            action.contains(" # "),
            "pinned action lacks version comment: {action}"
        );
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root above rssh-app")
        .to_owned()
}
