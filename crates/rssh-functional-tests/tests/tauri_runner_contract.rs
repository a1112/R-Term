use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn tauri_scenario_uses_os_input_and_the_shared_observer_driver() {
    let runner =
        fs::read_to_string(root().join("crates/rssh-functional-tests/src/runner.rs")).unwrap();
    let scenario =
        fs::read_to_string(root().join("functional-tests/scenarios/tauri.local-pty.toml")).unwrap();
    assert!(runner.contains("Surface::Tauri"));
    assert!(runner.contains("execute_tauri_window_scenario("));
    assert!(runner.contains("execute_observed_window_scenario"));
    assert!(runner.contains("observed_window_platform_options(scenario, target, x11_class)"));
    assert!(runner.contains("platform_driver(target, process_id, options)"));
    assert!(runner.contains(
        "(scenario.surface == Surface::NativeWindow).then_some(FUNCTIONAL_X11_WINDOW_CLASS)"
    ));
    assert!(runner.contains("xtest_paste_key: tauri_wayland.then_some(\"ctrl+v\")"));
    assert!(runner.contains("xtest_wayland_clipboard: tauri_wayland"));
    assert!(runner.contains("tauri_web_close_point(&scenario.actions)"));
    assert!(scenario.contains("type = \"resize_window\""));
    assert!(scenario.contains("text = \"echo tauri-probe\""));
    assert!(!scenario.contains("sleep"));
}

#[test]
fn tauri_full_matrix_routes_macos_accessibility_through_a_manual_self_hosted_job() {
    let root = root();
    let matrix = fs::read_to_string(root.join("functional-tests/matrix.toml")).unwrap();
    let workflow =
        fs::read_to_string(root.join("docs/trial/legacy-workflows/functional.yml")).unwrap();

    let tauri_run = matrix
        .split("[[scenario_runs]]")
        .find(|entry| entry.contains("scenario_id = \"tauri.local-pty\""))
        .expect("Tauri run must be present");
    assert!(tauri_run.contains("macos-accessibility"));

    let start = workflow
        .find("  tauri-platform-macos:")
        .expect("privileged Tauri macOS job must be present");
    let end = start
        + workflow[start..]
            .find("  production-package-smoke:")
            .expect("job following privileged Tauri macOS must be present");
    let job = &workflow[start..end];
    assert!(job.contains("if: github.event_name == 'workflow_dispatch'"));
    assert!(!job.contains("github.event.pull_request"));
    assert!(job.contains("runs-on: [self-hosted, macOS, X64, rssh-accessibility]"));
    assert!(job.contains("--target macos-accessibility"));
    assert!(job.contains("--capability macos_accessibility"));
}
