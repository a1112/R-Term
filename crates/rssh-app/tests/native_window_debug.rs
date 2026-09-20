#![cfg(windows)]

mod common;

use std::{
    process::Command,
    sync::{Mutex, MutexGuard},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rssh_test_support::{ChildGuard, ChildOutput};
use rterm_render_wgpu::gpu::should_abandon_recovered_window_surface;

const PROCESS_DEADLINE: Duration = Duration::from_secs(30);
const RSSH_APP_EXECUTABLE: &str = env!("CARGO_BIN_EXE_rterm");
const DIRECT_GPU_TEXT_SPECIMEN: &str = "office 中 مرحبا नमस्ते שלום 😀 █";
const PTY_LINK_BEGIN: &str = "RSSH-LINK-BEGIN|";
const PTY_LINK_END: &str = "|RSSH-LINK-END";
const STACK_OVERFLOW_MESSAGE: &str = "overflowed its stack";
static NATIVE_WINDOW_TEST_LOCK: Mutex<()> = Mutex::new(());
const CDB_FRAME_EVIDENCE: &str = "\
CDB frame evidence for the existing Windows debug failure:
  window::run: ~617,680 B
  configured_startup_app_with_constructor: ~365,440 B
  validate_cli_config_overrides: ~59,616 B
  combined startup stack: ~1,042,736 B
Root-cause context: large startup state is passed by value through these frames; this test
must keep launching the real debug executable instead of constructing a winit event loop
on the test worker thread.";

#[test]
fn state_json_control_exits_successfully() {
    const COMMAND_INTENT: &str = "rssh-app -n window --state-json";
    const ARGUMENTS: &[&str] = &["-n", "window", "--state-json"];
    let _native_window = native_window_test_guard();
    let output = run_rssh_app(COMMAND_INTENT, ARGUMENTS);
    let diagnostics = diagnostics(COMMAND_INTENT, ARGUMENTS, &output);

    assert!(
        output.status.success(),
        "state-report control did not exit successfully\n{diagnostics}"
    );
    serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap_or_else(|error| {
        panic!("state-report control emitted invalid JSON: {error}\n{diagnostics}")
    });
}

#[test]
fn one_frame_native_window_does_not_overflow_the_debug_stack() {
    const COMMAND_INTENT: &str = "rssh-app -n window --frames 1";
    const ARGUMENTS: &[&str] = &["-n", "window", "--frames", "1"];
    let _native_window = native_window_test_guard();
    let output = run_rssh_app(COMMAND_INTENT, ARGUMENTS);
    let diagnostics = diagnostics(COMMAND_INTENT, ARGUMENTS, &output);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "native window did not exit successfully; expected the current RED failure to report \
         Windows status 0xC00000FD until Task 4 removes the oversized startup frames\n\
         {diagnostics}"
    );
    assert!(
        !stderr.contains(STACK_OVERFLOW_MESSAGE),
        "native window reported a stack overflow\n{diagnostics}"
    );
}

#[test]
fn default_native_window_uses_direct_gpu_text_without_compatibility_uploads() {
    const COMMAND_INTENT: &str = "rssh-app -n window --frames 1 --metrics-json";
    const ARGUMENTS: &[&str] = &["-n", "window", "--frames", "1", "--metrics-json"];
    let _native_window = native_window_test_guard();
    let output = run_rssh_app(COMMAND_INTENT, ARGUMENTS);
    let diagnostics = diagnostics(COMMAND_INTENT, ARGUMENTS, &output);
    let metrics: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!("default native window emitted invalid metrics JSON: {error}\n{diagnostics}")
        });

    assert!(output.status.success(), "{diagnostics}");
    assert_eq!(metrics["text_backend"], "shaped-gpu-atlas", "{diagnostics}");
    assert_eq!(metrics["gpu_text_rendered_frames"], 1, "{diagnostics}");
    assert_eq!(
        metrics["gpu_compatibility_frame_uploads"], 0,
        "{diagnostics}"
    );
    assert!(
        metrics["gpu_backend"]
            .as_str()
            .is_some_and(|value| !value.is_empty()),
        "{diagnostics}"
    );
    assert!(
        metrics["gpu_adapter_name"]
            .as_str()
            .is_some_and(|value| !value.is_empty()),
        "{diagnostics}"
    );
    assert!(metrics["gpu_surface_format"].is_string(), "{diagnostics}");
}

#[test]
fn injected_device_loss_rebuilds_direct_gpu_state_and_presents_the_same_frame() {
    const BASELINE_COMMAND_INTENT: &str = "rssh-app -n window --frames 1 --metrics-json";
    const COMMAND_INTENT: &str =
        "RSSH_TEST_GPU_DEVICE_LOSS=1 rssh-app -n window --frames 1 --metrics-json";
    const ARGUMENTS: &[&str] = &["-n", "window", "--frames", "1", "--metrics-json"];
    let _native_window = native_window_test_guard();
    let baseline_output = run_rssh_app(BASELINE_COMMAND_INTENT, ARGUMENTS);
    let baseline_diagnostics = diagnostics(BASELINE_COMMAND_INTENT, ARGUMENTS, &baseline_output);
    assert!(baseline_output.status.success(), "{baseline_diagnostics}");
    let baseline_metrics: serde_json::Value = serde_json::from_slice(&baseline_output.stdout)
        .unwrap_or_else(|error| {
            panic!(
                "device-loss baseline emitted invalid metrics JSON: {error}\n{baseline_diagnostics}"
            )
        });
    // wgpu's test injection panics before recovery on software-only adapters.
    if baseline_metrics["gpu_software_adapter"]
        .as_bool()
        .unwrap_or(false)
    {
        return;
    }

    let mut command = Command::new(RSSH_APP_EXECUTABLE);
    command
        .args(ARGUMENTS)
        .env("RSSH_TEST_GPU_DEVICE_LOSS", "1");
    let output = ChildGuard::spawn(command, PROCESS_DEADLINE)
        .expect("launch device-loss recovery probe")
        .wait()
        .expect("bounded device-loss recovery probe");
    let diagnostics = diagnostics(COMMAND_INTENT, ARGUMENTS, &output);
    let metrics: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!("device-loss probe emitted invalid metrics JSON: {error}\n{diagnostics}")
        });

    assert!(output.status.success(), "{diagnostics}");
    assert_eq!(metrics["gpu_device_recoveries"], 1, "{diagnostics}");
    assert_eq!(metrics["gpu_device_recovery_failures"], 0, "{diagnostics}");
    let backend = metrics["gpu_backend"]
        .as_str()
        .expect("GPU backend string in metrics");
    let vendor_id = u32::try_from(
        metrics["gpu_adapter_vendor_id"]
            .as_u64()
            .expect("GPU adapter vendor id in metrics"),
    )
    .expect("GPU adapter vendor id fits u32");
    assert!(
        metrics["gpu_adapter_device_id"].as_u64().is_some(),
        "{diagnostics}"
    );
    let expected_abandonment = u64::from(should_abandon_recovered_window_surface(
        std::env::consts::OS,
        backend,
        vendor_id,
        true,
        true,
    ));
    assert_eq!(
        metrics["gpu_abandoned_lost_surfaces"], expected_abandonment,
        "{diagnostics}"
    );
    assert_eq!(metrics["gpu_device_losses"], 1, "{diagnostics}");
    assert_eq!(metrics["gpu_presented_frames"], 1, "{diagnostics}");
    assert_eq!(metrics["gpu_text_rendered_frames"], 1, "{diagnostics}");
    assert_eq!(
        metrics["gpu_compatibility_frame_uploads"], 0,
        "{diagnostics}"
    );
}

#[test]
fn static_native_window_reaches_ten_frames_without_external_damage() {
    let _native_window = native_window_test_guard();
    let probe = common::run_ten_frame_native_window(RSSH_APP_EXECUTABLE);

    common::assert_ten_frame_native_metrics(&probe);
}

#[test]
fn native_window_reports_real_gpu_presentation_for_one_and_ten_frames() {
    let _native_window = native_window_test_guard();
    let nonce = format!(
        "rssh-native-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after Unix epoch")
            .as_nanos()
    );
    let linkage_payload = format!("{nonce}|{DIRECT_GPU_TEXT_SPECIMEN}");
    let powershell_command = format!(
        "[Console]::OutputEncoding=[Text.UTF8Encoding]::new(); \
         [Console]::Write('{PTY_LINK_BEGIN}{linkage_payload}{PTY_LINK_END}')"
    );
    let expected_raw_digest = digest_hex(rterm_render_core::terminal_bytes_content_digest(
        linkage_payload.as_bytes(),
    ));

    for frames in [1_u64, 10] {
        let frame_text = frames.to_string();
        let mut arguments = vec![
            "-n".to_owned(),
            "window".to_owned(),
            "--frames".to_owned(),
            frame_text,
            "--metrics-json".to_owned(),
        ];
        if frames == 10 {
            arguments.extend([
                "--".to_owned(),
                "powershell.exe".to_owned(),
                "-NoProfile".to_owned(),
                "-NonInteractive".to_owned(),
                "-Command".to_owned(),
                powershell_command.clone(),
            ]);
        }
        let argument_refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
        let command_intent = format!("rssh-app -n window --frames {frames} --metrics-json");
        let output =
            run_rssh_app_with_direct_gpu_text(&command_intent, &argument_refs, frames == 10);
        let diagnostics = diagnostics(&command_intent, &argument_refs, &output);

        assert!(
            output.status.success(),
            "native GPU metrics smoke failed\n{diagnostics}"
        );
        let metrics: serde_json::Value =
            serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
                panic!("native window emitted invalid metrics JSON: {error}\n{diagnostics}")
            });
        assert!(
            metrics["gpu_backend"]
                .as_str()
                .is_some_and(|value| !value.is_empty()),
            "missing GPU backend\n{diagnostics}"
        );
        assert!(
            metrics["gpu_adapter_name"]
                .as_str()
                .is_some_and(|value| !value.is_empty()),
            "missing GPU adapter name\n{diagnostics}"
        );
        assert!(metrics["gpu_adapter_type"].is_string(), "{diagnostics}");
        assert!(
            metrics["gpu_software_adapter"].is_boolean(),
            "{diagnostics}"
        );
        assert!(metrics["gpu_surface_format"].is_string(), "{diagnostics}");
        assert!(metrics["gpu_present_mode"].is_string(), "{diagnostics}");
        assert_eq!(metrics["gpu_rendered_frames"], frames, "{diagnostics}");
        assert_eq!(metrics["gpu_presented_frames"], frames, "{diagnostics}");
        assert_eq!(
            metrics["gpu_compatibility_frame_uploads"], 0,
            "{diagnostics}"
        );
        assert_eq!(metrics["gpu_uncaptured_errors"], 0, "{diagnostics}");
        assert_eq!(metrics["gpu_device_losses"], 0, "{diagnostics}");
        assert_eq!(metrics["text_backend"], "shaped-gpu-atlas", "{diagnostics}");
        assert_eq!(metrics["gpu_text_rendered_frames"], frames, "{diagnostics}");
        if frames == 10 {
            common::assert_gpu_text_glyph_activity(&metrics, &diagnostics);
            assert_native_unicode_linkage(&metrics, &expected_raw_digest, &diagnostics);
        }
    }
}

fn assert_native_unicode_linkage(
    metrics: &serde_json::Value,
    expected_raw_digest: &str,
    diagnostics: &str,
) {
    assert!(
        metrics["pty_bytes"].as_u64().is_some_and(|bytes| bytes > 0),
        "Unicode specimen produced no PTY bytes\n{diagnostics}"
    );
    assert_eq!(
        metrics["pty_linkage_found"], true,
        "raw PTY matcher did not find the nonce/specimen payload\n{diagnostics}"
    );
    assert_eq!(
        metrics["pty_linkage_digest"], expected_raw_digest,
        "raw PTY matcher found different nonce/specimen bytes\n{diagnostics}"
    );
    assert_eq!(
        metrics["terminal_linkage_nonce_found"], true,
        "the unique PTY nonce did not reach the active terminal snapshot\n{diagnostics}"
    );
    assert_eq!(
        metrics["terminal_snapshot_content_digest"], metrics["gpu_text_content_digest"],
        "the actual active terminal plan did not reach GPU text preparation unchanged\n\
         {diagnostics}"
    );
    assert!(
        metrics["terminal_snapshot_content_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64 && digest.bytes().any(|byte| byte != b'0')),
        "missing SHA-256 terminal plan digest\n{diagnostics}"
    );
    assert!(
        metrics["first_rendered_cell_ms"].is_number(),
        "Unicode specimen never reached a rendered cell\n{diagnostics}"
    );
}

fn digest_hex(digest: rterm_render_core::TerminalContentDigest) -> String {
    use std::fmt::Write as _;

    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[test]
fn native_window_reconfigures_the_direct_surface_after_resize() {
    const COMMAND_INTENT: &str =
        "rssh-app -n window --frames 2 --metrics-json (resize after first present)";
    const ARGUMENTS: &[&str] = &["-n", "window", "--frames", "2", "--metrics-json"];
    let _native_window = native_window_test_guard();
    let mut command = Command::new(RSSH_APP_EXECUTABLE);
    command
        .args(ARGUMENTS)
        .env("RSSH_TEST_DIRECT_GPU_TEXT", "1")
        .env("RSSH_TEST_RESIZE_AFTER_FIRST_PRESENT", "800x480");
    let output = ChildGuard::spawn(command, PROCESS_DEADLINE)
        .expect("spawn deterministic native resize smoke")
        .wait()
        .expect("deterministic native resize smoke completes");
    let diagnostics = diagnostics(COMMAND_INTENT, ARGUMENTS, &output);
    let metrics: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!("native resize smoke emitted invalid metrics JSON: {error}\n{diagnostics}")
        });

    assert!(output.status.success(), "{diagnostics}");
    assert_eq!(metrics["gpu_rendered_frames"], 2, "{diagnostics}");
    assert_eq!(metrics["gpu_presented_frames"], 2, "{diagnostics}");
    assert_eq!(metrics["gpu_surface_width"], 800, "{diagnostics}");
    assert_eq!(metrics["gpu_surface_height"], 480, "{diagnostics}");
    assert_eq!(metrics["text_backend"], "shaped-gpu-atlas", "{diagnostics}");
    assert_eq!(metrics["gpu_text_rendered_frames"], 2, "{diagnostics}");
    assert!(
        metrics["gpu_surface_reconfigurations"]
            .as_u64()
            .is_some_and(|count| count >= 2),
        "resize did not reconfigure the direct surface\n{diagnostics}"
    );
}

#[test]
fn ssh_gui_auto_presents_cpu_then_gpu_on_the_same_window() {
    const COMMAND_INTENT: &str =
        "rssh-app ssh --gui --renderer auto (CPU first frame then GPU candidate)";
    const ARGUMENTS: &[&str] = &[
        "ssh",
        "--gui",
        "--renderer",
        "auto",
        "--host",
        "127.0.0.1",
        "--user",
        "test",
        "--port",
        "9",
        "--metrics-json",
    ];
    let _native_window = native_window_test_guard();
    let output = run_hybrid_ssh_gui(COMMAND_INTENT, ARGUMENTS, false);
    let diagnostics = diagnostics(COMMAND_INTENT, ARGUMENTS, &output);
    let metrics: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!("hybrid GUI emitted invalid metrics: {error}\n{diagnostics}")
        });
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "{diagnostics}");
    assert!(
        stderr.contains("first_present") && stderr.contains("final_renderer=cpu"),
        "the bootstrap frame was not presented by the CPU renderer\n{diagnostics}"
    );
    assert_eq!(metrics["render_frames"], 2, "{diagnostics}");
    assert_eq!(metrics["final_renderer"], "gpu", "{diagnostics}");
    assert_eq!(metrics["gpu_presented_frames"], 1, "{diagnostics}");
}

#[test]
fn ssh_gui_deferred_gpu_init_failure_presents_a_second_cpu_frame() {
    const COMMAND_INTENT: &str =
        "rssh-app ssh --gui --renderer auto (forced GPU init failure to CPU fallback)";
    const ARGUMENTS: &[&str] = &[
        "ssh",
        "--gui",
        "--renderer",
        "auto",
        "--host",
        "127.0.0.1",
        "--user",
        "test",
        "--port",
        "9",
        "--metrics-json",
    ];
    let _native_window = native_window_test_guard();
    let output = run_hybrid_ssh_gui(COMMAND_INTENT, ARGUMENTS, true);
    let diagnostics = diagnostics(COMMAND_INTENT, ARGUMENTS, &output);
    let metrics: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!("fallback GUI emitted invalid metrics: {error}\n{diagnostics}")
        });
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "{diagnostics}");
    assert!(
        stderr.contains("deferred GPU initialization failed; using CPU renderer"),
        "forced failure did not enter the CPU fallback path\n{diagnostics}"
    );
    assert_eq!(metrics["render_frames"], 2, "{diagnostics}");
    assert_eq!(metrics["final_renderer"], "cpu", "{diagnostics}");
    assert_eq!(metrics["gpu_presented_frames"], 0, "{diagnostics}");
}

fn native_window_test_guard() -> MutexGuard<'static, ()> {
    NATIVE_WINDOW_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn run_hybrid_ssh_gui(
    command_intent: &str,
    args: &[&str],
    force_gpu_init_failure: bool,
) -> ChildOutput {
    let mut command = Command::new(RSSH_APP_EXECUTABLE);
    command.args(args).env("RSSH_TEST_SSH_GUI_FRAME_LIMIT", "2");
    if force_gpu_init_failure {
        command.env("RSSH_TEST_DEFERRED_GPU_INIT_FAILURE", "1");
    }
    ChildGuard::spawn(command, PROCESS_DEADLINE)
        .unwrap_or_else(|error| panic!("failed to launch `{command_intent}`: {error}"))
        .wait()
        .unwrap_or_else(|error| panic!("`{command_intent}` did not complete: {error}"))
}

fn run_rssh_app(command_intent: &str, args: &[&str]) -> ChildOutput {
    let mut command = Command::new(RSSH_APP_EXECUTABLE);
    command.args(args);

    let guard = ChildGuard::spawn(command, PROCESS_DEADLINE).unwrap_or_else(|error| {
        panic!(
            "failed to launch `{command_intent}` with a {PROCESS_DEADLINE:?} deadline\n\
             executable: {RSSH_APP_EXECUTABLE:?}\n\
             arguments: {args:?}\n\
             error: {error}\n\
             {CDB_FRAME_EVIDENCE}"
        )
    });
    guard.wait().unwrap_or_else(|error| {
        panic!(
            "`{command_intent}` did not complete within its bounded subprocess harness\n\
             executable: {RSSH_APP_EXECUTABLE:?}\n\
             arguments: {args:?}\n\
             error: {error}\n\
             {CDB_FRAME_EVIDENCE}"
        )
    })
}

fn run_rssh_app_with_direct_gpu_text(
    command_intent: &str,
    args: &[&str],
    require_pty_linkage: bool,
) -> ChildOutput {
    let mut command = Command::new(RSSH_APP_EXECUTABLE);
    command.args(args).env("RSSH_TEST_DIRECT_GPU_TEXT", "1");
    if require_pty_linkage {
        command.env("RSSH_TEST_PTY_LINKAGE", "1");
    }
    ChildGuard::spawn(command, PROCESS_DEADLINE)
        .unwrap_or_else(|error| {
            panic!(
                "failed to launch direct GPU text `{command_intent}` with a {PROCESS_DEADLINE:?} deadline\n\
                 executable: {RSSH_APP_EXECUTABLE:?}\n\
                 arguments: {args:?}\n\
                 error: {error}\n\
                 {CDB_FRAME_EVIDENCE}"
            )
        })
        .wait()
        .unwrap_or_else(|error| {
            panic!(
                "`{command_intent}` direct GPU text did not complete within its bounded subprocess harness\n\
                 executable: {RSSH_APP_EXECUTABLE:?}\n\
                 arguments: {args:?}\n\
                 error: {error}\n\
                 {CDB_FRAME_EVIDENCE}"
            )
        })
}

fn diagnostics(command_intent: &str, args: &[&str], output: &ChildOutput) -> String {
    format!(
        "command intent: `{command_intent}`\n\
         executable: {RSSH_APP_EXECUTABLE:?}\n\
         arguments: {args:?}\n\
         exit status: {:?} (code: {:?})\n\
         bounded stdout: {:?}\n\
         bounded stderr: {:?}\n\
         {CDB_FRAME_EVIDENCE}",
        output.status,
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    )
}
