use std::{fmt::Write as _, path::Path, process::Command, time::Duration};

use rssh_test_support::{ChildGuard, ChildOutput, platform_marker_command_for_window_frames};

const PROCESS_DEADLINE: Duration = Duration::from_secs(120);
// Keep the framed marker below 80 columns so ConPTY does not inject a line wrap.
const DETERMINISTIC_PAYLOAD: &str = "rssh-e2e|office 中 مرحبا नमस्ते שלום 😀 █";
const PTY_LINK_BEGIN: &str = "RSSH-LINK-BEGIN|";
const PTY_LINK_END: &str = "|RSSH-LINK-END";
const STACK_OVERFLOW_MESSAGE: &str = "overflowed its stack";

pub struct NativeWindowProbe {
    pub output: ChildOutput,
    pub metrics: serde_json::Value,
}

pub fn run_ten_frame_native_window(executable: impl AsRef<Path>) -> NativeWindowProbe {
    run_ten_frame_native_window_at_scale(executable, None)
}

pub fn run_ten_frame_native_window_at_scale(
    executable: impl AsRef<Path>,
    scale_factor: impl Into<Option<f64>>,
) -> NativeWindowProbe {
    let scale_factor = scale_factor.into();
    if scale_factor.is_some() {
        run_ten_frame_native_window_with_command(
            executable,
            scale_factor,
            None,
            platform_marker_command_for_window_frames,
        )
    } else {
        run_ten_frame_native_window_with_log(executable, None, None)
    }
}

pub fn run_ten_frame_native_window_with_log(
    executable: impl AsRef<Path>,
    scale_factor: impl Into<Option<f64>>,
    log: Option<&Path>,
) -> NativeWindowProbe {
    run_ten_frame_native_window_with_command(
        executable,
        scale_factor.into(),
        log,
        platform_marker_command_for_window_frames,
    )
}

fn run_ten_frame_native_window_with_command(
    executable: impl AsRef<Path>,
    scale_factor: Option<f64>,
    log: Option<&Path>,
    marker_command: impl FnOnce(&str) -> Command,
) -> NativeWindowProbe {
    let executable = executable.as_ref();
    let framed_marker = format!("{PTY_LINK_BEGIN}{DETERMINISTIC_PAYLOAD}{PTY_LINK_END}");
    let marker_command = marker_command(&framed_marker);
    let mut command = Command::new(executable);
    command.args([
        "-n",
        "window",
        "--renderer",
        "gpu",
        "--frames",
        "10",
        "--metrics-json",
    ]);
    if let Some(log) = log {
        command.arg("--log").arg(log);
    }
    command
        .arg("--")
        .arg(marker_command.get_program())
        .args(marker_command.get_args())
        .env("RSSH_TEST_DIRECT_GPU_TEXT", "1")
        .env("RSSH_TEST_PTY_LINKAGE", "1");
    if cfg!(target_os = "windows") || scale_factor.is_some() {
        // Hosted Windows runners can block indefinitely in FIFO presentation
        // while a native surface is being resized or first exposed. Immediate
        // is still a real GPU presentation path and falls back to FIFO when
        // the adapter does not expose it.
        command.env("RSSH_TEST_PRESENT_MODE", "immediate");
    }
    if let Some(scale_factor) = scale_factor {
        command.env(
            "RSSH_TEST_WINDOW_SCALE_FACTOR",
            format!("{scale_factor:.2}"),
        );
        #[cfg(target_os = "windows")]
        if std::env::var_os("RSSH_TEST_NATIVE_SCALE_PRIMARY").is_none() {
            // The baseline probe still exercises the hosted runner's primary
            // adapter. Repeated DPI probes use the software fallback so a
            // transient hosted DX12 driver teardown cannot poison later
            // scenarios on the VM. The runner can opt back into the primary
            // adapter for a bounded retry when the software adapter itself is
            // wedged by the hosted desktop stack.
            command.env("RSSH_TEST_FORCE_FALLBACK_ADAPTER", "1");
        }
    }
    for (name, value) in marker_command.get_envs() {
        if let Some(value) = value {
            command.env(name, value);
        }
    }

    let child = ChildGuard::spawn(command, PROCESS_DEADLINE)
        .expect("launch deadline-bounded ten-frame native window");
    let process_id = child.process_id();
    let output = child.wait().unwrap_or_else(|error| {
        panic!(
            "ten-frame native window exits within its deadline (scale_factor={scale_factor:?}, process_id={process_id:?}): {error:?}"
        )
    });
    let diagnostics = diagnostics(executable, &output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "ten-frame native window did not exit cleanly\n{diagnostics}"
    );
    assert!(
        !stderr.contains(STACK_OVERFLOW_MESSAGE),
        "ten-frame native window reported a stack overflow\n{diagnostics}"
    );
    let metrics = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!("ten-frame native window emitted invalid metrics JSON: {error}\n{diagnostics}")
    });

    NativeWindowProbe { output, metrics }
}

pub fn assert_ten_frame_native_metrics(probe: &NativeWindowProbe) {
    let diagnostics = diagnostics("rssh-app", &probe.output);
    let metrics = &probe.metrics;

    assert_eq!(metrics["gpu_rendered_frames"], 10, "{diagnostics}");
    assert_eq!(metrics["gpu_presented_frames"], 10, "{diagnostics}");
    assert_eq!(metrics["gpu_text_rendered_frames"], 10, "{diagnostics}");
    assert_eq!(
        metrics["gpu_compatibility_frame_uploads"], 0,
        "{diagnostics}"
    );
    assert!(
        metrics["pty_bytes"].as_u64().is_some_and(|bytes| bytes > 0),
        "deterministic marker produced no PTY bytes\n{diagnostics}"
    );
    assert!(metrics["first_pty_byte_ms"].is_number(), "{diagnostics}");
    assert!(
        metrics["first_rendered_cell_ms"].is_number(),
        "{diagnostics}"
    );
    for field in [
        "gpu_backend",
        "gpu_adapter_name",
        "gpu_adapter_type",
        "gpu_surface_format",
        "gpu_present_mode",
    ] {
        assert!(
            metrics[field]
                .as_str()
                .is_some_and(|value| !value.is_empty()),
            "missing {field}\n{diagnostics}"
        );
    }
    assert!(
        metrics["gpu_software_adapter"].is_boolean(),
        "{diagnostics}"
    );
    assert!(
        metrics["gpu_surface_width"]
            .as_u64()
            .is_some_and(|width| width > 0),
        "{diagnostics}"
    );
    assert!(
        metrics["gpu_surface_height"]
            .as_u64()
            .is_some_and(|height| height > 0),
        "{diagnostics}"
    );
    assert_eq!(metrics["gpu_uncaptured_errors"], 0, "{diagnostics}");
    assert_eq!(metrics["gpu_device_losses"], 0, "{diagnostics}");
    assert_eq!(metrics["text_backend"], "shaped-gpu-atlas", "{diagnostics}");
    assert_gpu_text_glyph_activity(metrics, &diagnostics);

    assert_eq!(metrics["pty_linkage_found"], true, "{diagnostics}");
    if cfg!(windows) {
        assert!(
            metrics["pty_linkage_digest"]
                .as_str()
                .is_some_and(|digest| {
                    digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
                }),
            "Windows ConPTY did not report a valid raw PTY digest\n{diagnostics}"
        );
    } else {
        assert_eq!(
            metrics["pty_linkage_digest"],
            digest_hex(rterm_render_core::terminal_bytes_content_digest(
                DETERMINISTIC_PAYLOAD.as_bytes(),
            )),
            "{diagnostics}"
        );
    }
    assert_eq!(
        metrics["terminal_linkage_nonce_found"], true,
        "{diagnostics}"
    );
    if metrics["runtime_api"] == "v2-runtime-hub" {
        assert_eq!(metrics["runtime_live_threads"], 0, "{diagnostics}");
    }
}

pub fn assert_gpu_text_glyph_activity(metrics: &serde_json::Value, diagnostics: &str) {
    assert!(
        metrics["gpu_text_prepared_glyphs"]
            .as_u64()
            .is_some_and(|glyphs| glyphs > 0),
        "font specimen did not prepare GPU text glyphs\n{diagnostics}"
    );
    let rendered_glyphs = [
        "gpu_text_mask_glyphs",
        "gpu_text_color_glyphs",
        "gpu_text_block_glyphs",
    ]
    .into_iter()
    .map(|field| {
        metrics[field]
            .as_u64()
            .unwrap_or_else(|| panic!("missing {field}\n{diagnostics}"))
    })
    .sum::<u64>();
    assert!(
        rendered_glyphs > 0,
        "font specimen did not rasterize GPU text glyphs\n{diagnostics}"
    );
}

fn digest_hex(digest: rterm_render_core::TerminalContentDigest) -> String {
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

fn diagnostics(executable: impl std::fmt::Debug, output: &ChildOutput) -> String {
    format!(
        "executable: {executable:?}\nexit status: {:?} (code: {:?})\nstdout: {:?}\nstderr: {:?}",
        output.status,
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    )
}
