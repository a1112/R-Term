use std::{fs, path::PathBuf};

fn repo_file(path: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    fs::read_to_string(root.join(path)).unwrap()
}

#[test]
fn windows_helper_uses_send_input_and_targets_only_the_observed_process_window() {
    let source = repo_file("scripts/functional/windows-send-input.ps1");
    for contract in [
        "SendInput",
        "EnumWindows",
        "GetWindowThreadProcessId",
        "SetForegroundWindow",
        "ClientToScreen",
        "MovePointer",
        "MOUSEEVENTF_ABSOLUTE",
        "MOUSEEVENTF_VIRTUALDESK",
        "GetSystemMetrics",
    ] {
        assert!(source.contains(contract), "missing {contract}");
    }
    assert!(!source.contains("Invoke-Expression"));
    assert!(source.contains("[BitConverter]::ToUInt32"));
    assert!(!source.contains("[uint32]([int]$ActionArguments[1] * 120)"));
    assert!(source.contains("$CompileOnly"));
    assert!(source.contains("$AssemblyPath"));
    assert!(source.contains("-OutputAssembly $AssemblyPath"));
}

#[test]
fn x11_helper_discovers_the_pid_window_and_calls_xdotool_xtest_actions() {
    let source = repo_file("scripts/functional/x11-xtest-input.sh");
    let production_smoke = repo_file("scripts/functional/smoke-production-tauri.sh");
    assert!(source.contains("search --onlyvisible --pid"));
    assert!(source.contains("RSSH_FUNCTIONAL_X11_WINDOW_TITLE"));
    assert!(source.contains("search --onlyvisible --name"));
    assert!(source.contains("getwindowname"));
    assert!(source.contains("find_visible_window"));
    assert!(source.contains("pgrep -P"));
    assert!(source.contains("for _ in {1..600}"));
    assert!(source.contains("windowactivate --sync"));
    assert!(source.contains("xdotool"));
    assert!(source.contains("type --clearmodifiers --delay 0 -- \"$*\""));
    assert!(source.contains("Enter|enter) key=Return"));
    assert!(source.contains("xdotool key --clearmodifiers \"$key\""));
    assert!(source.contains("click \"$button\""));
    assert!(source.contains("sleep 0.1"));
    assert!(!source.contains("type --clearmodifiers --delay 0 --window"));
    assert!(!source.contains("click --window"));
    assert!(!source.contains("/dev/stdin"));
    assert!(production_smoke.contains("bash scripts/functional/x11-xtest-input.sh"));
    assert!(production_smoke.contains("RSSH_FUNCTIONAL_X11_WINDOW_TITLE=R-SSH"));
}

#[test]
fn production_tauri_tracks_linux_helpers_by_identity_and_ignores_zombies() {
    let source = repo_file("scripts/functional/smoke-production-tauri.sh");
    assert!(source.contains("owned_start_times=()"));
    assert!(source.contains("/proc/$pid/stat"));
    assert!(source.contains("[[ \"$state\" != Z && \"$state\" != X ]]"));
    assert!(source.contains("process_is_owned_live"));
    let all_owned_exited = source
        .split("all_owned_exited() {")
        .nth(1)
        .and_then(|body| body.split("dump_live_owned_processes() {").next())
        .expect("bounded all-owned-exited predicate");
    assert!(all_owned_exited.trim_end().ends_with("return 0\n}"));

    let root_wait = source
        .rfind("wait \"$root_pid\"")
        .expect("reap production Tauri root");
    let helper_wait = source
        .find("wait_condition 45 \"production Tauri left an owned helper process\"")
        .expect("verify owned helpers exited");
    assert!(root_wait < helper_wait);
    assert!(source[helper_wait..].starts_with(
        "wait_condition 45 \"production Tauri left an owned helper process\" all_owned_exited"
    ));
    assert!(source.contains("dump_live_owned_processes"));
    assert!(source.contains("ps -p \"$pid\" -o pid=,ppid=,stat=,comm=,args="));
}

#[test]
fn production_tauri_wait_rechecks_the_condition_at_the_deadline() {
    let source = repo_file("scripts/functional/smoke-production-tauri.sh");
    let wait_condition = source
        .split("wait_condition() {")
        .nth(1)
        .and_then(|body| body.split("root_alive()").next())
        .expect("wait_condition function");

    assert_eq!(
        wait_condition.matches("\"$@\" && return 0").count(),
        2,
        "the predicate must be checked once more after the deadline loop"
    );
}

#[test]
fn windows_production_tauri_wait_rechecks_the_condition_at_the_deadline() {
    let source = repo_file("scripts/functional/smoke-production-tauri.ps1");
    let wait_condition = source
        .split("function Wait-Condition")
        .nth(1)
        .and_then(|body| body.split("function Save-FailureScreenshot").next())
        .expect("Windows Wait-Condition function");

    assert_eq!(
        wait_condition
            .matches("if (& $Condition) { return }")
            .count(),
        2,
        "the Windows predicate must be checked once more after the deadline loop"
    );
}

#[test]
fn clipboard_helper_bounds_selection_ownership_for_each_backend() {
    let source = repo_file("scripts/functional/x11-set-clipboard.sh");
    assert!(source.contains("xclip -selection clipboard -loops 1"));
    assert!(source.contains("RSSH_FUNCTIONAL_WAYLAND_CLIPBOARD"));
    assert!(
        source
            .lines()
            .any(|line| line.trim() == "printf '%s' \"$1\" | wl-copy")
    );
    assert!(source.contains("wl-copy --clear"));
    assert!(!source.contains("--paste-once"));
    assert!(source.contains("printf '%s' \"$1\""));
    assert!(!source.contains("eval"));
}

#[test]
fn windows_helper_restores_before_focusing_and_verifies_foreground_ownership() {
    let source = repo_file("scripts/functional/windows-send-input.ps1");
    assert!(source.contains("GetForegroundWindow"));
    assert!(source.contains("IsIconic"));
    let restore = source
        .find("ShowWindow($window, 9)")
        .expect("restore before focus");
    let focus = source[restore..]
        .find("SetForegroundWindow($window)")
        .map(|offset| restore + offset)
        .expect("focus after restore");
    assert!(restore < focus);
    assert!(source.contains("GetForegroundWindow() -eq $window"));
}

#[test]
fn macos_helper_refuses_untrusted_accessibility_and_posts_cg_events() {
    let source = repo_file("scripts/functional/macos-cgevent.swift");
    assert!(source.contains("AXIsProcessTrustedWithOptions"));
    assert!(source.contains("CGEventPost"));
    assert!(source.contains("NSRunningApplication"));
    assert!(!source.contains("osascript"));
}

#[test]
fn wayland_harness_is_nested_weston_x11_not_headless_input_emulation() {
    let source = repo_file("scripts/functional/run-wayland-seat.sh");
    assert!(source.contains("--backend=x11-backend.so"));
    assert!(source.contains("RSSH_FUNCTIONAL_WESTON_SHELL:-kiosk-shell.so"));
    assert!(source.contains("--shell=\"$weston_shell\""));
    assert!(source.contains("RSSH_FUNCTIONAL_WESTON_BACKEND=x11"));
    assert!(source.contains("Xvfb"));
    assert!(source.contains("wait_for_x11_display"));
    assert!(source.contains("wait_for_weston_socket"));
    assert!(source.contains("wait_for_weston_window"));
    assert!(source.contains("dump_startup_logs"));
    assert!(source.contains("export RSSH_FUNCTIONAL_XDOTOOL="));
    assert!(source.contains("if ! rm -rf -- \"$runtime\""));
    let weston_socket_wait = source
        .split("wait_for_weston_socket()")
        .nth(1)
        .and_then(|body| body.split("wait_for_weston_window()").next())
        .expect("Weston socket readiness function");
    let weston_window_wait = source
        .split("wait_for_weston_window()")
        .nth(1)
        .and_then(|body| body.split("Xvfb ").next())
        .expect("Weston window readiness function");
    assert!(weston_socket_wait.contains("for _ in {1..300}"));
    assert!(weston_window_wait.contains("for _ in {1..300}"));
    assert!(!source.contains("--backend=headless-backend.so"));
}

#[test]
fn native_wayland_forced_close_uses_a_shell_with_close_bindings() {
    let workflow = repo_file("docs/trial/legacy-workflows/functional.yml");
    let forced_close = workflow
        .lines()
        .find(|line| {
            line.contains("run-wayland-seat.sh") && line.contains("window.forced-close-cleanup")
        })
        .expect("native Wayland forced-close command");
    assert!(forced_close.contains("RSSH_FUNCTIONAL_WESTON_SHELL=desktop-shell.so"));
}

#[test]
fn x11_harness_owns_runtime_dbus_display_and_window_manager_lifetimes() {
    let source = repo_file("scripts/functional/run-x11-seat.sh");
    for contract in [
        "mkdir -m 700",
        "export XDG_RUNTIME_DIR=",
        "export RSSH_FUNCTIONAL_XDOTOOL=",
        "dbus-run-session",
        "xvfb-run --auto-servernum",
        "openbox",
        "wait_for_openbox",
        "xprop -root _NET_SUPPORTING_WM_CHECK",
        "trap cleanup EXIT",
    ] {
        assert!(
            source.contains(contract),
            "missing X11 harness contract {contract}"
        );
    }
    assert!(source.contains("if ! rm -rf -- \"$runtime\""));
    assert!(!source.contains("eval"));
    let display = source
        .find("xvfb-run --auto-servernum")
        .expect("Xvfb harness");
    let session_bus = source
        .find("dbus-run-session")
        .expect("D-Bus session harness");
    assert!(
        display < session_bus,
        "Xvfb must publish DISPLAY before D-Bus captures the activation environment"
    );

    let workflow = repo_file("docs/trial/legacy-workflows/functional.yml");
    let openbox_install_steps = workflow
        .lines()
        .filter(|line| line.contains("apt-get install") && line.contains("openbox"))
        .collect::<Vec<_>>();
    assert_eq!(openbox_install_steps.len(), 4);
    assert!(
        openbox_install_steps
            .iter()
            .all(|line| line.contains("x11-utils")),
        "every Openbox job must install xprop for the readiness gate"
    );
}
