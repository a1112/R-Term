use std::sync::Arc;

use super::{NativeConfigSnapshot, NativeWindowApp, NativeWindowConfigPatch};

#[test]
fn window_stores_one_shared_effective_config_and_small_runtime_patch() {
    let app = NativeWindowApp::new(Some(0));

    assert!(Arc::ptr_eq(
        &app.config_overrides.effective,
        &app.base_config_overrides.effective,
    ));
    assert!(
        std::mem::size_of::<NativeWindowConfigPatch>() <= std::mem::size_of::<usize>() * 2,
        "per-window runtime overrides must stay pointer-sized",
    );
}

#[test]
fn window_reload_reuses_unchanged_effective_config_subtrees() {
    let mut app = NativeWindowApp::new(Some(0));
    let before = Arc::clone(&app.config_overrides.effective);

    app.set_config_overrides(native_config_snapshot! {
        term: Some("xterm-rssh".to_owned()),
        ..NativeConfigSnapshot::default()
    });

    assert!(!Arc::ptr_eq(
        &before.terminal,
        &app.config_overrides.effective.terminal,
    ));
    assert!(Arc::ptr_eq(&before.font, &app.config_overrides.effective.font));
    assert!(Arc::ptr_eq(&before.input, &app.config_overrides.effective.input));
    assert!(Arc::ptr_eq(&before.window, &app.config_overrides.effective.window));
    assert!(Arc::ptr_eq(&before.render, &app.config_overrides.effective.render));
    assert!(Arc::ptr_eq(&before.domain, &app.config_overrides.effective.domain));
    assert!(Arc::ptr_eq(
        &before.lifecycle,
        &app.config_overrides.effective.lifecycle,
    ));
    assert_eq!(app.config_overrides.effective.terminal.term, "xterm-rssh");
}

#[test]
fn domain_and_lifecycle_values_live_in_one_copy_on_write_config_snapshot() {
    let mut app = NativeWindowApp::new(Some(0));
    let before = Arc::clone(&app.applied_config);

    app.set_config_overrides(native_config_snapshot! {
        default_domain: Some("local".to_owned()),
        default_workspace: Some("shared-config".to_owned()),
        ..NativeConfigSnapshot::default()
    });

    assert!(!Arc::ptr_eq(&before, &app.applied_config));
    assert_eq!(before.default_workspace, super::DEFAULT_WORKSPACE_NAME);
    assert_eq!(app.applied_config.default_workspace, "shared-config");
    assert_eq!(app.default_workspace, "shared-config");
    assert_eq!(
        std::mem::size_of_val(&app.applied_config),
        std::mem::size_of::<Arc<super::NativeAppliedConfig>>(),
    );

    let mut inherited = NativeWindowApp::new(Some(0));
    inherited.inherit_effective_config_from(&app);
    assert!(Arc::ptr_eq(&app.applied_config, &inherited.applied_config));
}

#[test]
fn input_terminal_and_tab_values_share_the_applied_config_snapshot() {
    let mut app = NativeWindowApp::new(Some(0));
    let before = Arc::clone(&app.applied_config);

    app.set_config_overrides(native_config_snapshot! {
        scroll_to_bottom_on_input: Some(false),
        enable_tab_bar: Some(false),
        enable_kitty_graphics: Some(false),
        ..NativeConfigSnapshot::default()
    });

    assert!(!Arc::ptr_eq(&before, &app.applied_config));
    assert!(!app.applied_config.scroll_to_bottom_on_input);
    assert!(!app.applied_config.enable_tab_bar);
    assert!(!app.applied_config.enable_kitty_graphics);
    assert!(!app.scroll_to_bottom_on_input);
    assert!(!app.enable_tab_bar);
}

#[test]
fn font_values_live_in_the_shared_applied_config_snapshot() {
    let mut app = NativeWindowApp::new(Some(0));
    let before = Arc::clone(&app.applied_config);
    let font_size = super::NativeFontSize::from_millipoints(24_000);

    app.set_config_overrides(native_config_snapshot! {
        font: Some("Cascadia Mono".to_owned()),
        font_size: Some(font_size),
        adjust_window_size_when_changing_font_size: Some(false),
        ..NativeConfigSnapshot::default()
    });

    assert!(!Arc::ptr_eq(&before, &app.applied_config));
    assert_eq!(app.applied_config.font.as_deref(), Some("Cascadia Mono"));
    assert_eq!(app.applied_config.font_size, font_size);
    assert!(!app
        .applied_config
        .adjust_window_size_when_changing_font_size);
    assert_eq!(app.font_size, font_size);
}

#[test]
fn render_and_overlay_values_live_in_the_shared_applied_config_snapshot() {
    let mut app = NativeWindowApp::new(Some(0));
    let before = Arc::clone(&app.applied_config);

    app.set_config_overrides(native_config_snapshot! {
        initial_cols: Some(132),
        tab_max_width: Some(42),
        launcher_alphabet: Some("abc".to_owned()),
        ..NativeConfigSnapshot::default()
    });

    assert!(!Arc::ptr_eq(&before, &app.applied_config));
    assert_eq!(app.applied_config.initial_cols, 132);
    assert_eq!(app.applied_config.tab_max_width, 42);
    assert_eq!(app.applied_config.launcher_alphabet, "abc");
    assert_eq!(app.initial_cols, 132);
    assert_eq!(app.tab_max_width, 42);
}

#[test]
fn terminal_identity_and_palette_values_share_the_applied_config_snapshot() {
    let mut app = NativeWindowApp::new(Some(0));
    let before = Arc::clone(&app.applied_config);

    app.set_config_overrides(native_config_snapshot! {
        term: Some("xterm-rssh-palette".to_owned()),
        foreground_color: Some(super::Color::Rgb(12, 34, 56)),
        ..NativeConfigSnapshot::default()
    });

    assert!(!Arc::ptr_eq(&before, &app.applied_config));
    assert_eq!(app.applied_config.term, "xterm-rssh-palette");
    assert_eq!(
        app.applied_config.foreground_color,
        super::Color::Rgb(12, 34, 56),
    );
    assert_eq!(app.term, "xterm-rssh-palette");
}

#[test]
fn renderer_timing_and_cursor_values_share_the_applied_config_snapshot() {
    let mut app = NativeWindowApp::new(Some(0));
    let before = Arc::clone(&app.applied_config);

    app.set_config_overrides(native_config_snapshot! {
        max_fps: Some(144),
        cursor_blink_rate_ms: Some(321),
        force_reverse_video_cursor: Some(true),
        ..NativeConfigSnapshot::default()
    });

    assert!(!Arc::ptr_eq(&before, &app.applied_config));
    assert_eq!(app.applied_config.max_fps, 144);
    assert_eq!(
        app.applied_config.cursor_blink_rate,
        std::time::Duration::from_millis(321),
    );
    assert!(app.applied_config.force_reverse_video_cursor);
    assert_eq!(app.max_fps, 144);
}

#[test]
fn focus_loss_clears_ime_composition() {
    let mut app = NativeWindowApp::new(Some(0));
    app.window_focused = true;
    app.handle_ime_preedit("zhong中文");
    assert!(app.ime_preedit.is_some());
    app.handle_focus_changed(false).unwrap();
    assert!(app.ime_preedit.is_none(), "stale composition must not cross focus changes");
}

#[test]
fn renderer_selection_survives_new_window_and_reload() {
    let mut app = NativeWindowApp::new(Some(0));
    app.set_renderer_mode(crate::cli::RendererMode::Cpu);
    let mut child = NativeWindowApp::new(Some(0));
    child.inherit_effective_config_from(&app);
    child.set_config_overrides(NativeConfigSnapshot::default());
    assert_eq!(child.renderer_mode, crate::cli::RendererMode::Cpu);
}

#[test]
fn chinese_preedit_uses_two_cells_per_character() {
    let mut app = NativeWindowApp::new(Some(0));
    app.handle_ime_preedit("中文");
    let layout = app.pane_render_layout();
    let cells = app.ime_preedit_cells(&layout);
    assert_eq!(cells.len(), 4);
    assert_eq!(cells[0].text.as_ref(), "中");
    assert_eq!(cells[0].columns, 2);
    assert!(cells[1].continuation);
    assert_eq!(cells[2].text.as_ref(), "文");
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "requires a Wayland compositor with data-control, with DISPLAY unset"]
fn native_wayland_clipboard_survives_copy_callback() {
    assert!(std::env::var_os("WAYLAND_DISPLAY").is_some());
    assert!(std::env::var_os("DISPLAY").is_none());
    assert!(super::write_window_clipboard_text("R-Term 中文 clipboard"));
    let received = std::process::Command::new("wl-paste")
        .args(["--no-newline", "--type", "text/plain;charset=utf-8"])
        .output()
        .expect("wl-paste installed in the Wayland test environment");
    assert!(received.status.success(), "{}", String::from_utf8_lossy(&received.stderr));
    assert_eq!(String::from_utf8(received.stdout).unwrap(), "R-Term 中文 clipboard");
}
