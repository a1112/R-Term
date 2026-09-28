use rssh_test_support::ChildGuard;
use std::{process::Command, time::Duration};

fn run(mut command: Command) -> rssh_test_support::ChildOutput {
    command.env_remove("RSSH_TEST_WINDOW_SCALE_FACTOR");
    // PTY byte metrics intentionally require this opt-in observation path.
    command.env("RSSH_TEST_PTY_LINKAGE", "1");
    ChildGuard::spawn(command, Duration::from_secs(30))
        .expect("launch bounded terminal")
        .wait()
        .expect("terminal exits within deadline")
}

#[test]
fn renderer_and_auto_backend_preserve_command_arguments() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rterm"));
    command.args([
        "-n",
        "start",
        "--renderer",
        "cpu",
        "--display-backend",
        "auto",
        "--cwd",
        "directory with 中文",
        "--state-json",
        "--",
        "program with spaces",
        "argument with 中文",
        "--renderer",
        "not-an-option",
    ]);
    let output = run(command);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let launch = &report["workspaces"][0]["tabs"][0]["panes"][0]["launch"];
    assert_eq!(launch["program"], "program with spaces");
    assert_eq!(launch["cwd"], "directory with 中文");
    assert_eq!(
        launch["args"],
        serde_json::json!(["argument with 中文", "--renderer", "not-an-option"])
    );
}

#[test]
fn explicit_backend_failure_is_reported() {
    for backend in ["wayland", "x11"] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rterm"));
        command
            .args([
                "-n",
                "window",
                "--renderer",
                "cpu",
                "--display-backend",
                backend,
            ])
            .env("WAYLAND_DISPLAY", "rterm-nonexistent-wayland-socket")
            .env("DISPLAY", ":65530");
        let output = run(command);
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!error.contains("unexpected window option"), "{error}");
        #[cfg(not(target_os = "linux"))]
        assert!(error.contains("only supported on Linux"), "{error}");
    }
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "requires a real native Wayland compositor"]
fn native_wayland_cpu_window() {
    assert!(std::env::var_os("WAYLAND_DISPLAY").is_some());
    let mut command = Command::new(env!("CARGO_BIN_EXE_rterm"));
    command
        .env_remove("DISPLAY")
        .env("WAYLAND_DEBUG", "1")
        .args([
            "-n",
            "window",
            "--renderer",
            "cpu",
            "--display-backend",
            "wayland",
            "--class",
            "org.forge.RTerm",
            "--frames",
            "10",
            "--metrics-json",
            "--",
            "/bin/sh",
            "-c",
            "printf 'RSSH-LINK-BEGIN|RTERM-WAYLAND-CPU|中文|RSSH-LINK-END\n'; sleep 10",
        ]);
    let output = run(command);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metrics: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let protocol_log = String::from_utf8_lossy(&output.stderr);
    assert!(
        protocol_log
            .lines()
            .any(|line| line.contains("set_app_id") && line.contains("org.forge.RTerm")),
        "native app ID missing: {protocol_log}"
    );
    assert_eq!(metrics["gpu_rendered_frames"], 0);
    assert_eq!(metrics["render_frames"], 10);
    assert!(metrics["pty_bytes"].as_u64().unwrap() > 0);
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "requires a live Wayland session to prove X11 does not fall back"]
fn forced_x11_does_not_fall_back_to_live_wayland() {
    assert!(std::env::var_os("WAYLAND_DISPLAY").is_some());
    let mut command = Command::new(env!("CARGO_BIN_EXE_rterm"));
    command
        .args([
            "-n",
            "window",
            "--renderer",
            "cpu",
            "--display-backend",
            "x11",
            "--frames",
            "1",
        ])
        .env("DISPLAY", ":65530");
    let output = run(command);
    assert!(
        !output.status.success(),
        "X11 request must not fall back to working Wayland"
    );
}

#[test]
#[cfg(target_os = "windows")]
fn cpu_renderer_starts_local_pty_and_preserves_exit_status() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rterm"));
    command.args([
        "-n",
        "window",
        "--renderer",
        "cpu",
        "--metrics-json",
        "--",
        "cmd.exe",
        "/d",
        "/c",
        "echo RTERM-CPU-PTY & exit /b 7",
    ]);
    let output = run(command);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metrics: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(metrics["last_exit_code"], 7);
    assert_eq!(metrics["runtime_live_threads"], 0);
    assert_eq!(metrics["gpu_rendered_frames"], 0);
    assert!(metrics["pty_bytes"].as_u64().unwrap() > 0);
}

#[test]
#[cfg(any(target_os = "windows", target_os = "linux"))]
#[cfg_attr(target_os = "linux", ignore = "requires a native Wayland compositor")]
fn cpu_renderer_closes_failed_program_window() {
    let missing =
        std::env::temp_dir().join(format!("rterm-missing-program-{}", std::process::id()));
    assert!(!missing.exists());
    let mut command = Command::new(env!("CARGO_BIN_EXE_rterm"));
    command
        .args(["-n", "window", "--renderer", "cpu", "--"])
        .arg(missing);
    let output = run(command);
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.contains("transport start error"), "{diagnostic}");
}

#[test]
#[cfg(any(target_os = "windows", target_os = "linux"))]
#[cfg_attr(target_os = "linux", ignore = "requires a native Wayland compositor")]
fn cpu_renderer_closes_failed_working_directory_window() {
    let missing =
        std::env::temp_dir().join(format!("rterm-missing-directory-{}", std::process::id()));
    assert!(!missing.exists());
    let mut command = Command::new(env!("CARGO_BIN_EXE_rterm"));
    command
        .args(["-n", "window", "--renderer", "cpu", "--cwd"])
        .arg(missing);
    let output = run(command);
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.contains("transport start error"), "{diagnostic}");
}
