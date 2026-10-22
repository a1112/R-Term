#![cfg(target_os = "windows")]

use std::{
    process::Command,
    time::{Duration, Instant},
};

use rterm_test_support::{
    ChildGuard,
    windows::{WindowFrameObservation, WindowPoint, WindowRect, wait_for_owned_window_frame},
};

#[test]
fn missing_process_reports_pid_and_last_enumeration() {
    let missing_process_id = u32::MAX;
    let error = wait_for_owned_window_frame(
        missing_process_id,
        Instant::now() + Duration::from_millis(250),
    )
    .expect_err("an impossible process ID must not expose a window");
    let diagnostic = error.to_string();

    assert!(diagnostic.contains(&missing_process_id.to_string()));
    assert!(diagnostic.contains("last enumeration"));
}

#[test]
fn visible_owned_window_returns_complete_frame_observation() {
    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-NonInteractive",
        "-STA",
        "-Command",
        "Add-Type -AssemblyName System.Windows.Forms; \
         $form = [Windows.Forms.Form]::new(); \
         $form.Text = 'Rterm window probe fixture'; \
         [Windows.Forms.Application]::Run($form)",
    ]);
    let fixture =
        ChildGuard::spawn(command, Duration::from_secs(15)).expect("spawn window probe fixture");
    let process_id = fixture.process_id().expect("fixture process ID");

    let observation =
        wait_for_owned_window_frame(process_id, Instant::now() + Duration::from_secs(10))
            .expect("observe fixture window");

    assert_eq!(observation.process_id, process_id);
    assert_ne!(observation.hwnd, 0);
    assert!(observation.window_rect.width() > 0);
    assert!(observation.window_rect.height() > 0);
    assert!(observation.client_rect.width() > 0);
    assert!(observation.client_rect.height() > 0);
    assert!(observation.dpi >= 96);
    assert!(observation.title.contains("Rterm window probe fixture"));
}

#[test]
fn borderless_contract_accepts_winit_one_pixel_shadow_geometry() {
    let observation = frame_observation(
        WindowRect {
            left: 152,
            top: 152,
            right: 980,
            bottom: 705,
        },
        WindowRect {
            left: 0,
            top: 0,
            right: 828,
            bottom: 553,
        },
        WindowPoint { x: 152, y: 153 },
    );

    assert!(observation.has_borderless_client_area());
}

#[test]
fn borderless_contract_rejects_a_standard_native_frame() {
    let observation = frame_observation(
        WindowRect {
            left: 101,
            top: 101,
            right: 2021,
            bottom: 1126,
        },
        WindowRect {
            left: 0,
            top: 0,
            right: 1905,
            bottom: 987,
        },
        WindowPoint { x: 109, y: 131 },
    );

    assert!(!observation.has_borderless_client_area());
}

fn frame_observation(
    window_rect: WindowRect,
    client_rect: WindowRect,
    client_origin: WindowPoint,
) -> WindowFrameObservation {
    WindowFrameObservation {
        process_id: 42,
        hwnd: 1,
        style: 0x16cf_0000,
        ex_style: 0x0004_0110,
        window_rect,
        client_rect,
        client_origin,
        dpi: 144,
        title: "R-Term".to_owned(),
    }
}
