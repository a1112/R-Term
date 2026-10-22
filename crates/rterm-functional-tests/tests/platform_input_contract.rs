use std::collections::BTreeMap;

use rterm_functional_tests::{
    ActionV1, DriverOperation, InputBackend, PlatformInputDriver, PlatformInputError, WindowControl,
};

fn environment(values: &[(&str, &str)]) -> BTreeMap<String, String> {
    values
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

#[test]
fn x11_uses_xtest_and_never_falls_back_to_synthetic_stdin() {
    let driver = PlatformInputDriver::from_environment(
        InputBackend::X11Xtest,
        &environment(&[
            ("DISPLAY", ":99"),
            ("RTERM_FUNCTIONAL_XDOTOOL", "/usr/bin/xdotool"),
            ("RTERM_FUNCTIONAL_X11_WINDOW", "4194305"),
        ]),
    )
    .expect("x11 capability");

    let operations = driver
        .plan(&ActionV1::TypeText {
            text: "R-Term 终端".to_owned(),
        })
        .expect("type plan");
    assert_eq!(operations.len(), 3);
    assert!(matches!(
        &operations[0],
        DriverOperation::Command { program, arguments, .. }
            if program == "/usr/bin/xdotool"
                && arguments == &["windowactivate", "--sync", "4194305"]
    ));
    assert!(matches!(
        &operations[1],
        DriverOperation::Command { arguments, .. }
            if arguments == &["type", "--clearmodifiers", "--delay", "0", "R-Term 终端"]
    ));
    assert!(matches!(
        &operations[2],
        DriverOperation::Command { arguments, .. }
            if arguments == &["sleep", "0.1"]
    ));
}

#[test]
fn wayland_requires_nested_weston_x11_backend_and_injects_through_its_seat() {
    let headless = PlatformInputDriver::from_environment(
        InputBackend::WaylandWestonSeat,
        &environment(&[
            ("DISPLAY", ":99"),
            ("WAYLAND_DISPLAY", "wayland-rterm"),
            ("RTERM_FUNCTIONAL_WESTON_BACKEND", "headless"),
            ("RTERM_FUNCTIONAL_XDOTOOL", "/usr/bin/xdotool"),
            ("RTERM_FUNCTIONAL_WESTON_WINDOW", "6291457"),
        ]),
    );
    assert!(matches!(
        headless,
        Err(PlatformInputError::CapabilityGate { capability, .. })
            if capability == "wayland_weston_x11_seat"
    ));

    let driver = PlatformInputDriver::from_environment(
        InputBackend::WaylandWestonSeat,
        &environment(&[
            ("DISPLAY", ":99"),
            ("WAYLAND_DISPLAY", "wayland-rterm"),
            ("RTERM_FUNCTIONAL_WESTON_BACKEND", "x11"),
            ("RTERM_FUNCTIONAL_XDOTOOL", "/usr/bin/xdotool"),
            ("RTERM_FUNCTIONAL_WESTON_WINDOW", "6291457"),
        ]),
    )
    .expect("nested Weston seat");
    let operations = driver
        .plan(&ActionV1::MouseClick {
            x: 120,
            y: 80,
            button: rterm_functional_tests::MouseButton::Left,
        })
        .expect("pointer plan");
    assert!(matches!(
        &operations[0],
        DriverOperation::Command { arguments, .. }
            if arguments == &["windowfocus", "--sync", "6291457"]
    ));
    assert!(matches!(
        &operations[1],
        DriverOperation::Command { arguments, .. }
            if arguments == &["sleep", "0.2"]
    ));
    assert!(operations.iter().all(|operation| matches!(
        operation,
        DriverOperation::Command { environment, .. }
            if environment.get("DISPLAY").map(String::as_str) == Some(":99")
    )));
    assert!(operations.iter().any(|operation| matches!(
        operation,
        DriverOperation::Command { arguments, .. }
            if arguments.iter().any(|argument| argument == "mousemove")
    )));
    assert!(operations.iter().all(|operation| matches!(
        operation,
        DriverOperation::Command { arguments, .. }
            if arguments.first().map(String::as_str) != Some("click")
                || !arguments.iter().any(|argument| argument == "--window")
    )));
    assert!(matches!(
        operations.last(),
        Some(DriverOperation::Command { arguments, .. })
            if arguments == &["sleep", "0.1"]
    ));
}

#[test]
fn xtest_clipboard_paste_uses_xclip_before_the_focused_keyboard_binding() {
    let driver = PlatformInputDriver::from_environment(
        InputBackend::WaylandWestonSeat,
        &environment(&[
            ("DISPLAY", ":99"),
            ("WAYLAND_DISPLAY", "wayland-rterm"),
            ("RTERM_FUNCTIONAL_WESTON_BACKEND", "x11"),
            ("RTERM_FUNCTIONAL_XDOTOOL", "/usr/bin/xdotool"),
            ("RTERM_FUNCTIONAL_WESTON_WINDOW", "6291457"),
        ]),
    )
    .expect("nested Weston seat");
    let operations = driver
        .plan(&ActionV1::ClipboardPaste {
            text: "clipboard-probe".to_owned(),
        })
        .expect("clipboard plan");
    assert!(matches!(
        &operations[2],
        DriverOperation::Command { program, arguments, .. }
            if program == "bash"
                && arguments == &["scripts/functional/x11-set-clipboard.sh", "clipboard-probe"]
    ));
    assert!(matches!(
        &operations[3],
        DriverOperation::Command { program, arguments, .. }
            if program == "/usr/bin/xdotool"
                && arguments == &["key", "--clearmodifiers", "ctrl+shift+v"]
    ));
    assert!(matches!(
        operations.last(),
        Some(DriverOperation::Command { arguments, .. })
            if arguments == &["sleep", "0.1"]
    ));
}

#[test]
fn xtest_clipboard_paste_accepts_a_surface_specific_keyboard_binding() {
    let driver = PlatformInputDriver::from_environment(
        InputBackend::WaylandWestonSeat,
        &environment(&[
            ("DISPLAY", ":99"),
            ("WAYLAND_DISPLAY", "wayland-rterm"),
            ("RTERM_FUNCTIONAL_WESTON_BACKEND", "x11"),
            ("RTERM_FUNCTIONAL_XDOTOOL", "/usr/bin/xdotool"),
            ("RTERM_FUNCTIONAL_WESTON_WINDOW", "6291457"),
            ("RTERM_FUNCTIONAL_XTEST_PASTE_KEY", "shift+Insert"),
        ]),
    )
    .expect("nested Weston seat");
    let operations = driver
        .plan(&ActionV1::ClipboardPaste {
            text: "clipboard-probe".to_owned(),
        })
        .expect("clipboard plan");
    assert!(matches!(
        &operations[3],
        DriverOperation::Command { arguments, .. }
            if arguments == &["key", "--clearmodifiers", "shift+Insert"]
    ));
}

#[test]
fn wayland_clipboard_paste_clears_the_selection_after_the_browser_reads_it() {
    let driver = PlatformInputDriver::from_environment(
        InputBackend::WaylandWestonSeat,
        &environment(&[
            ("DISPLAY", ":99"),
            ("WAYLAND_DISPLAY", "wayland-rterm"),
            ("RTERM_FUNCTIONAL_WESTON_BACKEND", "x11"),
            ("RTERM_FUNCTIONAL_XDOTOOL", "/usr/bin/xdotool"),
            ("RTERM_FUNCTIONAL_WESTON_WINDOW", "6291457"),
            ("RTERM_FUNCTIONAL_WAYLAND_CLIPBOARD", "1"),
        ]),
    )
    .expect("nested Weston seat");
    let operations = driver
        .plan(&ActionV1::ClipboardPaste {
            text: "clipboard-probe".to_owned(),
        })
        .expect("clipboard plan");

    assert!(matches!(
        &operations[2],
        DriverOperation::Command { environment, .. }
            if environment.get("RTERM_FUNCTIONAL_WAYLAND_CLIPBOARD").map(String::as_str)
                == Some("1")
    ));
    assert!(matches!(
        operations.last(),
        Some(DriverOperation::Command { program, arguments, .. })
            if program == "bash"
                && arguments == &["scripts/functional/x11-set-clipboard.sh", "--clear"]
    ));
}

#[test]
fn nested_wayland_can_close_tauri_through_its_visible_web_control() {
    let driver = PlatformInputDriver::from_environment(
        InputBackend::WaylandWestonSeat,
        &environment(&[
            ("DISPLAY", ":99"),
            ("WAYLAND_DISPLAY", "wayland-rterm"),
            ("RTERM_FUNCTIONAL_WESTON_BACKEND", "x11"),
            ("RTERM_FUNCTIONAL_XDOTOOL", "/usr/bin/xdotool"),
            ("RTERM_FUNCTIONAL_WESTON_WINDOW", "6291457"),
            ("RTERM_FUNCTIONAL_XTEST_WEB_CLOSE_POINT", "856,35"),
        ]),
    )
    .expect("nested Weston seat");
    let operations = driver
        .plan(&ActionV1::WindowControl {
            operation: WindowControl::Close,
        })
        .expect("Tauri close plan");
    assert!(matches!(
        &operations[2],
        DriverOperation::Command { arguments, .. }
            if arguments == &["mousemove", "--window", "6291457", "856", "35"]
    ));
    assert!(matches!(
        &operations[3],
        DriverOperation::Command { arguments, .. }
            if arguments == &["click", "1"]
    ));
}

#[test]
fn nested_wayland_native_close_uses_the_apps_real_keyboard_binding() {
    let driver = PlatformInputDriver::from_environment(
        InputBackend::WaylandWestonSeat,
        &environment(&[
            ("DISPLAY", ":99"),
            ("WAYLAND_DISPLAY", "wayland-rterm"),
            ("RTERM_FUNCTIONAL_WESTON_BACKEND", "x11"),
            ("RTERM_FUNCTIONAL_XDOTOOL", "/usr/bin/xdotool"),
            ("RTERM_FUNCTIONAL_WESTON_WINDOW", "6291457"),
            ("RTERM_FUNCTIONAL_XTEST_CLOSE_KEY", "ctrl+shift+w"),
            ("RTERM_FUNCTIONAL_XTEST_CLOSE_CONFIRM_KEY", "Return"),
        ]),
    )
    .expect("nested Weston seat");
    let operations = driver
        .plan(&ActionV1::WindowControl {
            operation: WindowControl::Close,
        })
        .expect("native close plan");
    assert!(matches!(
        &operations[2],
        DriverOperation::Command { arguments, .. }
            if arguments == &["key", "--clearmodifiers", "ctrl+shift+w"]
    ));
    assert!(matches!(
        &operations[4],
        DriverOperation::Command { arguments, .. }
            if arguments == &["key", "--clearmodifiers", "Return"]
    ));
}

#[test]
fn macos_accessibility_is_a_hard_capability_gate() {
    let denied = PlatformInputDriver::from_environment(
        InputBackend::MacosCgEvent,
        &environment(&[(
            "RTERM_FUNCTIONAL_MACOS_CGEVENT_HELPER",
            "/opt/rterm/cgevent-helper",
        )]),
    );
    assert!(matches!(
        denied,
        Err(PlatformInputError::CapabilityGate { capability, .. })
            if capability == "macos_accessibility"
    ));

    let driver = PlatformInputDriver::from_environment(
        InputBackend::MacosCgEvent,
        &environment(&[
            ("RTERM_FUNCTIONAL_MACOS_ACCESSIBILITY", "authorized"),
            (
                "RTERM_FUNCTIONAL_MACOS_CGEVENT_HELPER",
                "/opt/rterm/cgevent-helper",
            ),
        ]),
    )
    .expect("authorized helper");
    assert!(matches!(
        &driver.plan(&ActionV1::FocusWindow).expect("focus plan")[0],
        DriverOperation::Command { program, arguments, .. }
            if program == "/usr/bin/xcrun"
                && arguments == &["swift", "/opt/rterm/cgevent-helper", "focus"]
    ));
}

#[test]
fn xtest_maps_logical_enter_to_the_x11_return_keysym() {
    let driver = PlatformInputDriver::from_environment(
        InputBackend::X11Xtest,
        &environment(&[
            ("DISPLAY", ":99"),
            ("RTERM_FUNCTIONAL_XDOTOOL", "/usr/bin/xdotool"),
            ("RTERM_FUNCTIONAL_X11_WINDOW", "0x4200007"),
        ]),
    )
    .expect("X11 target");
    let operations = driver
        .plan(&ActionV1::Key {
            key: "Enter".to_owned(),
            modifiers: Vec::new(),
        })
        .expect("key plan");

    assert!(matches!(
        &operations[1],
        DriverOperation::Command { arguments, .. }
            if arguments == &["key", "--clearmodifiers", "Return"]
    ));
}

#[test]
fn platform_driver_rejects_non_os_actions_instead_of_faking_them() {
    let driver = PlatformInputDriver::from_environment(
        InputBackend::WindowsSendInput,
        &environment(&[("RTERM_FUNCTIONAL_APP_PID", "4660")]),
    )
    .expect("windows target");
    assert!(matches!(
        driver.plan(&ActionV1::PtyInput {
            bytes_hex: "41".to_owned(),
        }),
        Err(PlatformInputError::UnsupportedAction(_))
    ));
}

#[test]
fn windows_clipboard_paste_uses_the_native_terminal_binding() {
    let driver = PlatformInputDriver::from_environment(
        InputBackend::WindowsSendInput,
        &environment(&[("RTERM_FUNCTIONAL_APP_PID", "4660")]),
    )
    .expect("windows target");
    let operations = driver
        .plan(&ActionV1::ClipboardPaste {
            text: "functional clipboard".to_owned(),
        })
        .expect("paste plan");
    let DriverOperation::Command { arguments, .. } = &operations[0];
    let action = arguments
        .iter()
        .position(|argument| argument == "-Action")
        .expect("explicit action flag");
    let action_arguments = arguments
        .iter()
        .position(|argument| argument == "-ActionArgumentsJson")
        .expect("explicit action-arguments flag");
    assert_eq!(arguments.get(action + 1).map(String::as_str), Some("paste"));
    let encoded = arguments
        .get(action_arguments + 1)
        .expect("encoded arguments");
    assert_eq!(
        serde_json::from_str::<Vec<String>>(encoded).unwrap(),
        ["functional clipboard"]
    );

    let script = include_str!("../../../scripts/functional/windows-send-input.ps1");
    assert!(script.contains("VirtualKey(0x10, $true)"));
    assert!(script.contains("VirtualKey(0x11, $true)"));
    assert!(script.contains("function Set-ClipboardWithRetry"));
    assert!(script.contains("Set-ClipboardWithRetry ($ActionArguments -join \" \")"));
    assert!(script.contains("Send-VirtualKey 0x56"));
    assert!(script.contains("VirtualKey(0x11, $false)"));
    assert!(script.contains("VirtualKey(0x10, $false)"));
}

#[test]
fn windows_mouse_actions_are_one_json_value_not_positional_powershell_arguments() {
    let driver = PlatformInputDriver::from_environment(
        InputBackend::WindowsSendInput,
        &environment(&[("RTERM_FUNCTIONAL_APP_PID", "4660")]),
    )
    .expect("windows target");
    let cases = [
        (
            ActionV1::MouseClick {
                x: 80,
                y: 81,
                button: rterm_functional_tests::MouseButton::Left,
            },
            vec!["80", "81", "left"],
        ),
        (
            ActionV1::MouseDrag {
                from_x: 1,
                from_y: 2,
                to_x: 3,
                to_y: 4,
                button: rterm_functional_tests::MouseButton::Right,
            },
            vec!["1", "2", "3", "4", "right"],
        ),
        (
            ActionV1::MouseWheel {
                delta_x: -2,
                delta_y: 3,
            },
            vec!["-2", "3"],
        ),
    ];
    for (action, expected) in cases {
        let operations = driver.plan(&action).expect("mouse plan");
        let DriverOperation::Command { arguments, .. } = &operations[0];
        let index = arguments
            .iter()
            .position(|argument| argument == "-ActionArgumentsJson")
            .expect("JSON flag");
        let decoded: Vec<String> = serde_json::from_str(&arguments[index + 1]).unwrap();
        assert_eq!(decoded, expected);
        assert_eq!(arguments.len(), index + 2);
    }
}

#[test]
fn windows_tauri_production_smoke_passes_json_arrays_to_the_input_helper() {
    let script = include_str!("../../../scripts/functional/smoke-production-tauri.ps1");
    for arguments in [
        r#"-Action click -ActionArgumentsJson '["80","80","left"]'"#,
        r#"-ActionArgumentsJson '["exit 7"]'"#,
        r#"-ActionArgumentsJson '["enter"]'"#,
        r#"-ActionArgumentsJson '["close-client-button"]'"#,
    ] {
        assert!(
            script.contains(arguments),
            "missing JSON helper argument {arguments}"
        );
    }
    assert!(!script.contains(" -ActionArguments "));
}

#[test]
fn windows_close_uses_focused_system_input_instead_of_an_unobserved_message() {
    let script = include_str!("../../../scripts/functional/windows-send-input.ps1");
    let close = script
        .split("\"close\" {")
        .nth(1)
        .expect("close window action")
        .split("default { throw \"unsupported window operation")
        .next()
        .expect("bounded close action");
    assert!(close.contains("VirtualKey(0x12, $true)"));
    assert!(close.contains("Send-VirtualKey 0x73"));
    assert!(close.contains("VirtualKey(0x12, $false)"));
    assert!(!close.contains("SendMessageTimeout"));
    assert!(!script.contains("PostMessage(WM_CLOSE)"));
}

#[test]
fn windows_tauri_driver_routes_close_to_the_visible_client_button() {
    let driver = PlatformInputDriver::from_environment(
        InputBackend::WindowsSendInput,
        &environment(&[
            ("RTERM_FUNCTIONAL_APP_PID", "4660"),
            ("RTERM_FUNCTIONAL_WINDOWS_CLIENT_CLOSE_BUTTON", "1"),
        ]),
    )
    .expect("Windows Tauri target");
    let operations = driver
        .plan(&ActionV1::WindowControl {
            operation: WindowControl::Close,
        })
        .expect("Tauri close plan");
    let DriverOperation::Command { arguments, .. } = &operations[0];
    let index = arguments
        .iter()
        .position(|argument| argument == "-ActionArgumentsJson")
        .expect("JSON flag");
    let decoded: Vec<String> = serde_json::from_str(&arguments[index + 1]).unwrap();
    assert_eq!(decoded, ["close-client-button"]);
}

#[test]
fn windows_tauri_close_clicks_its_visible_client_button_with_system_input() {
    let helper = include_str!("../../../scripts/functional/windows-send-input.ps1");
    let smoke = include_str!("../../../scripts/functional/smoke-production-tauri.ps1");
    let close_button = helper
        .split("\"close-client-button\" {")
        .nth(1)
        .expect("client close button action")
        .split("default { throw \"unsupported window operation")
        .next()
        .expect("bounded client close button action");

    assert!(helper.contains("GetClientRect"));
    assert!(close_button.contains("$closeCenterFromRight = 45"));
    assert!(close_button.contains("$closeCenterY = 36"));
    assert!(!close_button.contains("GetDpiForWindow"));
    assert!(
        close_button
            .contains("Set-ClientCursor ($rect.Right - $closeCenterFromRight) $closeCenterY")
    );
    assert!(close_button.contains("for ($attempt = 1; $attempt -le 12; $attempt++)"));
    assert!(close_button.contains("for ($poll = 1; $poll -le 20; $poll++)"));
    assert!(close_button.contains("IsWindowVisible($window)"));
    assert!(close_button.contains("Start-Sleep -Milliseconds 25"));
    assert!(close_button.contains("[RTermFunctionalInput]::Mouse"));
    assert!(smoke.contains("-ActionArgumentsJson '[\"close-client-button\"]'"));
}

#[test]
fn windows_process_target_prefers_the_titled_application_window() {
    let script = include_str!("../../../scripts/functional/windows-send-input.ps1");
    let find_window = script
        .split("public static IntPtr FindWindow(uint expectedProcessId)")
        .nth(1)
        .expect("process window lookup")
        .split("public static IntPtr FindWindowByTitle")
        .next()
        .expect("bounded process window lookup");

    assert!(find_window.contains("GetWindowText"));
    assert!(find_window.contains("title.Length > 0"));
}
