#[test]
fn window_app_can_disable_hide_mouse_cursor_when_typing() {
    let mut app = NativeWindowApp::new(None);
    app.set_config_overrides(native_config_snapshot! {
        hide_mouse_cursor_when_typing: Some(false),
        ..NativeConfigSnapshot::default()
    });
    app.handle_cursor_moved(PhysicalPosition::new(8.0, 8.0))
        .unwrap();

    app.handle_keyboard_input_event(
        &Key::Character("a".into()),
        PhysicalKey::Code(WinitKeyCode::KeyA),
        Some("a"),
        ElementState::Pressed,
        KittyKeyEventKind::Press,
    )
    .unwrap();

    assert!(app.mouse_cursor_visible);
    assert!(!app.native_effective_config().hide_mouse_cursor_when_typing);
}

#[test]
fn window_app_shift_home_end_are_not_default_scrollback_shortcuts() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
    app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
        .unwrap();

    assert!(!app.handle_scrollback_shortcut(&Key::Named(NamedKey::Home), ModifiersState::SHIFT));

    assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('d'));

    assert!(!app.handle_scrollback_shortcut(&Key::Named(NamedKey::End), ModifiersState::SHIFT));

    assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('d'));
    assert!(app.snapshot.cursor().is_some());
}

#[test]
fn window_title_omits_scrollback_position_after_scrollbar_overlay() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
    app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
        .unwrap();

    assert_eq!(app.runtime.terminal().scrollback().len(), 3);
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1]"
    );

    app.scroll_viewport_lines(1);
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1]"
    );

    app.scroll_viewport_lines(99);
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1]"
    );

    app.scroll_viewport_lines(-99);
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1]"
    );
}

#[test]
fn window_title_combines_scrollback_and_search_status() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(6, 2));
    app.handle_pty_output(b"alpha\r\nbeta\r\ngamma").unwrap();

    assert!(app.update_search_query("alpha"));

    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1] - Search: alpha"
    );
}

#[test]
fn window_title_includes_command_palette_status() {
    let mut app = NativeWindowApp::new(None);
    app.enter_command_palette_mode();

    assert_eq!(
        app.effective_window_title(),
        format!(
            "R-Term [workspace:1 tab:1 pane:1] - Command Palette: [1 / {}] New Tab",
            WINDOW_COMMANDS.len()
        )
    );

    app.command_palette_set_query("split".to_owned());

    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1] - Command Palette: \"split\" [1 / 3] Split Horizontal"
    );

    app.command_palette_set_query("zzz".to_owned());

    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1] - Command Palette: \"zzz\" (no match)"
    );
}

#[test]
fn window_command_palette_rows_limits_visible_overlay_entries() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(48, 8));
    app.set_config_overrides(native_config_snapshot! {
        command_palette_rows: Some(2),
        ..NativeConfigSnapshot::default()
    });

    app.enter_command_palette_mode();
    let snapshot = app.render_snapshot();

    assert!(
        snapshot_row_text(&snapshot, TAB_BAR_ROWS, 48).contains("> New Tab"),
        "first palette row was {:?}",
        snapshot_row_text(&snapshot, TAB_BAR_ROWS, 48)
    );
    assert!(
        snapshot_row_text(&snapshot, TAB_BAR_ROWS + 1, 48).contains("  Spawn Window"),
        "second palette row was {:?}",
        snapshot_row_text(&snapshot, TAB_BAR_ROWS + 1, 48)
    );
    assert!(
        !snapshot_row_text(&snapshot, TAB_BAR_ROWS + 2, 48).contains("Close Tab"),
        "third row should stay terminal content when command_palette_rows=2: {:?}",
        snapshot_row_text(&snapshot, TAB_BAR_ROWS + 2, 48)
    );
}

#[test]
fn window_app_workspace_overlays_hide_rendered_cursor_without_mutating_terminal() {
    let mut palette = NativeWindowApp::new(None);
    palette.runtime.resize(rterm_core::TerminalSize::new(24, 4));
    palette.handle_pty_output(b"palette").unwrap();
    assert!(palette.snapshot.cursor().is_some());

    palette.enter_command_palette_mode();
    assert!(
        palette.render_snapshot().cursor().is_none(),
        "command palette must not expose the terminal cursor to the GPU graph"
    );
    assert!(
        palette.snapshot.cursor().is_some(),
        "overlay presentation must not mutate terminal cursor state"
    );
    palette.exit_command_palette_mode();
    assert!(
        palette.render_snapshot().cursor().is_some(),
        "closing the command palette must restore the presentation cursor"
    );

    let mut search = NativeWindowApp::new(None);
    search.runtime.resize(rterm_core::TerminalSize::new(24, 4));
    search.handle_pty_output(b"search").unwrap();
    assert!(search.snapshot.cursor().is_some());

    search.enter_search_mode();
    assert!(
        search.render_snapshot().cursor().is_none(),
        "search overlay must not expose the terminal cursor to the GPU graph"
    );
    assert!(
        search.snapshot.cursor().is_some(),
        "search presentation must not mutate terminal cursor state"
    );
    search.exit_search_mode();
    assert!(
        search.render_snapshot().cursor().is_some(),
        "closing search must restore the presentation cursor"
    );

    let mut quick = NativeWindowApp::new(None);
    quick.runtime.resize(rterm_core::TerminalSize::new(40, 2));
    quick.handle_pty_output(b"https://example.com").unwrap();
    quick.enter_quick_select_mode();
    assert!(
        quick.render_snapshot().cursor().is_some(),
        "quick select owns its presentation cursor and must retain it"
    );

    let mut copy = NativeWindowApp::new(None);
    copy.runtime.resize(rterm_core::TerminalSize::new(24, 4));
    copy.handle_pty_output(b"copy").unwrap();
    copy.enter_copy_mode();
    assert!(
        copy.render_snapshot().cursor().is_some(),
        "copy mode owns its presentation cursor and must retain it"
    );
}

#[test]
fn window_app_applies_wezterm_command_palette_colors() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(48, 8));
    let overrides = super::native_config_overrides_from_wezterm_lua_config(
        r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.command_palette_bg_color = 'rgba(1,2,3,0.5)'
            config.command_palette_fg_color = 'rgba(4,5,6,0.5)'

            return config
            "##,
    )
    .expect("expected WezTerm command palette color config");
    app.set_config_overrides(overrides);

    app.enter_command_palette_mode();
    let snapshot = app.render_snapshot();
    let second_row =
        snapshot_cell(&snapshot, TAB_BAR_ROWS + 1, 0).expect("expected second command palette row");

    assert_eq!(second_row.background, Color::Rgba(1, 2, 3, 127));
    assert_eq!(second_row.foreground, Color::Rgba(4, 5, 6, 127));
}

#[test]
fn window_command_palette_uses_modern_selected_surface_by_default() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(48, 8));
    app.enter_command_palette_mode();

    let snapshot = app.render_snapshot();
    let selected =
        snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("expected selected command palette row");

    assert_eq!(selected.foreground, DEFAULT_UI_ACCENT_FOREGROUND);
    assert_eq!(selected.background, DEFAULT_UI_ACCENT_BACKGROUND);
}

#[test]
fn window_title_includes_quick_select_status() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
    app.handle_pty_output(b"https://example.com 10.0.0.1 test@x.io")
        .unwrap();

    app.enter_quick_select_mode();
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1] - Quick Select: [1 / 3]"
    );
    assert_eq!(app.selected_text().as_deref(), Some("https://example.com"));

    assert!(app.quick_select_step(SearchDirection::Next));
    assert_eq!(app.selected_text().as_deref(), Some("10.0.0.1"));
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1] - Quick Select: [2 / 3]"
    );

    assert!(app.quick_select_step(SearchDirection::Previous));
    assert_eq!(app.selected_text().as_deref(), Some("https://example.com"));

    app.exit_quick_select_mode();
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1]"
    );
}

#[test]
fn window_quick_select_uses_configured_match_colors() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
    app.set_config_overrides(native_config_snapshot! {
        quick_select_match_bg: Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Navy)),
        quick_select_match_fg: Some(NativeColorSpec::Color(Color::Rgb(1, 2, 3))),
        ..NativeConfigSnapshot::default()
    });
    app.handle_pty_output(b"https://example.com").unwrap();

    app.enter_quick_select_mode();
    let snapshot = app.render_snapshot();
    let match_cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 1).expect("quick-select match cell");

    assert_eq!(match_cell.foreground, Color::Rgb(1, 2, 3));
    assert_eq!(match_cell.background, Color::Indexed(4));
    assert!(!match_cell.inverse);
}

#[test]
fn window_quick_select_renders_configured_label_colors() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
    app.set_config_overrides(native_config_snapshot! {
        quick_select_label_bg: Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Navy)),
        quick_select_label_fg: Some(NativeColorSpec::Color(Color::Rgb(4, 5, 6))),
        ..NativeConfigSnapshot::default()
    });
    app.handle_pty_output(b"https://example.com").unwrap();

    app.enter_quick_select_mode();
    let snapshot = app.render_snapshot();
    let label_cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("quick-select label cell");

    assert_eq!(label_cell.ch, 'a');
    assert_eq!(label_cell.foreground, Color::Rgb(4, 5, 6));
    assert_eq!(label_cell.background, Color::Indexed(4));
    assert!(!label_cell.inverse);
}

#[test]
fn window_quick_select_uses_modern_default_label_colors() {
    let mut app = NativeWindowApp::new_with_visual_defaults(None);
    app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
    app.handle_pty_output(b"https://example.com")
        .expect("quick-select fixture output");

    app.enter_quick_select_mode();
    let snapshot = app.render_snapshot();
    let label_cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("quick-select label cell");

    assert_eq!(
        label_cell.foreground,
        Color::Rgb(0x0b, 0x12, 0x20),
        "modern quick-select labels should use the terminal background as ink"
    );
    assert_eq!(
        label_cell.background,
        Color::Rgb(0x38, 0xbd, 0xf8),
        "modern quick-select labels should use the cyan focus accent"
    );
    assert!(!label_cell.inverse);
}

#[test]
fn window_quick_select_hides_non_matching_labels_while_typing() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(40, 3));
    app.set_config_overrides(native_config_snapshot! {
        quick_select_alphabet: Some("ab".to_owned()),
        ..NativeConfigSnapshot::default()
    });
    app.handle_pty_output(b"https://one.test\r\nhttps://two.test\r\nhttps://three.test")
        .unwrap();

    app.enter_quick_select_mode();
    assert_eq!(
        active_quick_select_for_test(&app).labels.as_slice(),
        ["bb", "ba", "a"]
    );

    assert!(
        app.handle_quick_select_logical_key(&Key::Character("b".into()), ModifiersState::empty())
    );

    let snapshot = app.render_snapshot();
    let first_label = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("first label cell");
    let second_label = snapshot_cell(&snapshot, TAB_BAR_ROWS + 1, 0).expect("second label cell");
    let hidden_label = snapshot_cell(&snapshot, TAB_BAR_ROWS + 2, 0).expect("hidden label cell");

    assert_eq!(first_label.ch, 'b');
    assert_eq!(second_label.ch, 'b');
    assert_eq!(hidden_label.ch, 'h');
}

#[test]
fn window_quick_select_deduplicates_same_text_candidates() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(80, 1));
    app.handle_pty_output(b"dup@example.com dup@example.com unique@example.com")
        .unwrap();

    app.enter_quick_select_mode();

    let quick_select = active_quick_select_for_test(&app);
    assert_eq!(quick_select.matches.len(), 2);
    assert_eq!(quick_select.labels.len(), 2);
    assert_eq!(app.selected_text().as_deref(), Some("dup@example.com"));
    assert!(app.quick_select_step(SearchDirection::Next));
    assert_eq!(app.selected_text().as_deref(), Some("unique@example.com"));
}

#[test]
fn window_input_selector_renders_configured_label_colors() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(40, 3));
    app.set_config_overrides(native_config_snapshot! {
        input_selector_label_bg: Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Navy)),
        input_selector_label_fg: Some(NativeColorSpec::Color(Color::Rgb(4, 5, 6))),
        ..NativeConfigSnapshot::default()
    });

    app.enter_input_selector_mode(WindowInputSelectorOptions {
        title: "Pick".to_owned(),
        choices: vec![WindowInputSelectorChoice {
            label: "Alpha".to_owned(),
            id: Some("alpha".to_owned()),
        }],
        alphabet: Some("ab".to_owned()),
        description: None,
        fuzzy_description: None,
        fuzzy: false,
        action: None,
    });
    let snapshot = app.render_snapshot();
    let label_cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("input-selector label cell");

    assert_eq!(label_cell.ch, 'a');
    assert_eq!(label_cell.foreground, Color::Rgb(4, 5, 6));
    assert_eq!(label_cell.background, Color::Indexed(4));
    assert!(!label_cell.inverse);
}

#[test]
fn window_launcher_renders_configured_label_colors() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(40, 3));
    app.set_config_overrides(native_config_snapshot! {
        launcher_label_bg: Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Navy)),
        launcher_label_fg: Some(NativeColorSpec::Color(Color::Rgb(7, 8, 9))),
        ..NativeConfigSnapshot::default()
    });

    assert!(
        app.command_palette_execute(WindowCommand::ShowLauncherArgs(WindowShowLauncherArgs {
            flags: WindowShowLauncherFlags::commands(),
            title: Some("Pick Command".to_owned()),
            alphabet: Some("ab".to_owned()),
            help_text: None,
            fuzzy_help_text: None,
        },))
    );
    let snapshot = app.render_snapshot();
    let label_cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("launcher label cell");

    assert_eq!(label_cell.ch, 'a');
    assert_eq!(label_cell.foreground, Color::Rgb(7, 8, 9));
    assert_eq!(label_cell.background, Color::Indexed(4));
    assert!(!label_cell.inverse);
}

#[test]
fn window_quick_select_remove_styling_strips_pane_styles_before_highlighting() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
    app.set_config_overrides(native_config_snapshot! {
        quick_select_remove_styling: Some(true),
        quick_select_match_bg: Some(NativeColorSpec::Color(Color::Rgb(1, 2, 3))),
        quick_select_match_fg: Some(NativeColorSpec::Color(Color::Rgb(4, 5, 6))),
        quick_select_label_bg: Some(NativeColorSpec::Color(Color::Rgb(7, 8, 9))),
        quick_select_label_fg: Some(NativeColorSpec::Color(Color::Rgb(10, 11, 12))),
        ..NativeConfigSnapshot::default()
    });
    app.handle_pty_output(b"\x1b[31;1;4mhttps://example.com tail\x1b[0m")
        .unwrap();

    app.enter_quick_select_mode();
    let snapshot = app.render_snapshot();
    let label_cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("quick-select label cell");
    let match_cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 1).expect("quick-select match cell");
    let plain_cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 20).expect("unstyled pane cell");

    assert_eq!(label_cell.foreground, Color::Rgb(10, 11, 12));
    assert_eq!(label_cell.background, Color::Rgb(7, 8, 9));
    assert_eq!(match_cell.foreground, Color::Rgb(4, 5, 6));
    assert_eq!(match_cell.background, Color::Rgb(1, 2, 3));
    assert_eq!(plain_cell.ch, 't');
    assert_eq!(plain_cell.foreground, Color::Default);
    assert_eq!(plain_cell.background, Color::Default);
    assert!(!plain_cell.bold);
    assert!(!plain_cell.underline);
}

#[test]
fn window_title_includes_char_select_status() {
    let mut app = NativeWindowApp::new(None);

    app.enter_char_select_mode();
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1] - Char Select: SmileysAndEmotion"
    );

    app.enter_char_select_mode_with_options(WindowCharSelectOptions {
        copy_on_select: false,
        copy_to: WindowCopyDestination::PrimarySelection,
        group: Some("Smileys & Emotion".to_owned()),
    });
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1] - Char Select: Smileys & Emotion"
    );
}

#[test]
fn window_quick_select_replaces_pane_overlay_and_survives_higher_level_ui() {
    let mut app = NativeWindowApp::new(None);
    app.enter_search_mode();
    app.update_search_query("example");
    assert!(search_for_test(&app).is_some());

    app.enter_quick_select_mode();
    assert!(search_for_test(&app).is_none());
    assert!(quick_select_for_test(&app).is_some());

    app.enter_command_palette_mode();
    assert!(quick_select_for_test(&app).is_some());
    assert!(app.command_palette.is_some());
    assert!(!app.effective_window_title().contains("Quick Select"));
    app.exit_command_palette_mode();
    assert!(quick_select_for_test(&app).is_some());
    assert!(app.effective_window_title().contains("Quick Select"));
}

#[test]
fn window_quick_select_mode_has_no_match_status() {
    let mut app = NativeWindowApp::new(None);
    app.handle_pty_output(b"no links here").unwrap();

    app.enter_quick_select_mode();
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1] - Quick Select: no match"
    );
    assert_eq!(active_quick_select_for_test(&app).matches.len(), 0);
}

#[test]
fn window_quick_select_label_input_copies_matching_candidate() {
    let copied = Arc::new(Mutex::new(Vec::new()));
    let primary_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&copied);
    let recorded_primary = Arc::clone(&primary_copied);
    let mut app = NativeWindowApp::new(None);
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded.lock().unwrap().push(text.to_owned());
        true
    });
    app.primary_selection_writer = Box::new(move |text: &str| {
        recorded_primary.lock().unwrap().push(text.to_owned());
        true
    });
    app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
    app.handle_pty_output(b"https://example.com 10.0.0.1 test@x.io")
        .unwrap();

    app.enter_quick_select_mode();
    assert_eq!(active_quick_select_for_test(&app).matches.len(), 3);

    assert!(
        app.handle_quick_select_logical_key(&Key::Character("a".into()), ModifiersState::empty())
    );

    assert!(quick_select_for_test(&app).is_none());
    assert!(app.selection.is_none());
    assert_eq!(copied.lock().unwrap().as_slice(), ["test@x.io"]);
    assert_eq!(primary_copied.lock().unwrap().as_slice(), ["test@x.io"]);
}

#[test]
fn window_quick_select_uses_configured_alphabet_for_labels() {
    let copied = Arc::new(Mutex::new(Vec::new()));
    let primary_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&copied);
    let recorded_primary = Arc::clone(&primary_copied);
    let mut app = NativeWindowApp::new(None);
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded.lock().unwrap().push(text.to_owned());
        true
    });
    app.primary_selection_writer = Box::new(move |text: &str| {
        recorded_primary.lock().unwrap().push(text.to_owned());
        true
    });
    app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
    app.handle_pty_output(b"https://one.test https://two.test")
        .unwrap();
    app.set_config_overrides(native_config_snapshot! {
        quick_select_alphabet: Some("xy".to_owned()),
        ..NativeConfigSnapshot::default()
    });

    app.enter_quick_select_mode();
    let quick_select = active_quick_select_for_test(&app);
    assert_eq!(quick_select.labels.as_slice(), ["y", "x"]);

    assert!(
        app.handle_quick_select_logical_key(&Key::Character("x".into()), ModifiersState::empty())
    );

    assert!(quick_select_for_test(&app).is_none());
    assert!(app.selection.is_none());
    assert_eq!(copied.lock().unwrap().as_slice(), ["https://two.test"]);
    assert_eq!(
        primary_copied.lock().unwrap().as_slice(),
        ["https://two.test"]
    );
}

#[test]
fn window_quick_select_appends_configured_patterns_to_defaults() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
    app.handle_pty_output(b"ticket-1234 https://example.test")
        .unwrap();
    app.set_config_overrides(native_config_snapshot! {
        quick_select_patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
        ..NativeConfigSnapshot::default()
    });

    app.enter_quick_select_mode();

    let quick_select = active_quick_select_for_test(&app);
    assert_eq!(quick_select.matches.len(), 2);
    assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
    assert!(app.quick_select_step(SearchDirection::Next));
    assert_eq!(app.selected_text().as_deref(), Some("https://example.test"));
}

#[test]
fn window_quick_select_can_disable_default_patterns() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
    app.handle_pty_output(b"ticket-1234 https://example.test")
        .unwrap();
    app.set_config_overrides(native_config_snapshot! {
        quick_select_patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
        disable_default_quick_select_patterns: Some(true),
        ..NativeConfigSnapshot::default()
    });

    app.enter_quick_select_mode();

    let quick_select = active_quick_select_for_test(&app);
    assert_eq!(quick_select.matches.len(), 1);
    assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
}

#[test]
fn window_quick_select_enter_uses_wezterm_prior_match_binding() {
    let copied = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&copied);
    let mut app = NativeWindowApp::new(None);
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded.lock().unwrap().push(text.to_owned());
        true
    });
    app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
    app.handle_pty_output(b"https://example.com 10.0.0.1 test@x.io")
        .unwrap();

    app.enter_quick_select_mode();
    assert_eq!(app.selected_text().as_deref(), Some("https://example.com"));

    assert!(
        app.handle_quick_select_logical_key(&Key::Named(NamedKey::Enter), ModifiersState::empty())
    );

    assert!(quick_select_for_test(&app).is_some());
    assert!(copied.lock().unwrap().is_empty());
    assert_eq!(app.selected_text().as_deref(), Some("test@x.io"));
}

#[test]
fn window_quick_select_ctrl_n_and_ctrl_p_use_wezterm_match_navigation() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
    app.handle_pty_output(b"https://example.com 10.0.0.1 test@x.io")
        .unwrap();

    app.enter_quick_select_mode();
    assert_eq!(app.selected_text().as_deref(), Some("https://example.com"));

    assert!(
        app.handle_quick_select_logical_key(&Key::Character("n".into()), ModifiersState::CONTROL)
    );
    assert!(quick_select_for_test(&app).is_some());
    assert_eq!(app.selected_text().as_deref(), Some("10.0.0.1"));

    assert!(
        app.handle_quick_select_logical_key(&Key::Character("p".into()), ModifiersState::CONTROL)
    );
    assert!(quick_select_for_test(&app).is_some());
    assert_eq!(app.selected_text().as_deref(), Some("https://example.com"));
}

#[test]
fn window_quick_select_page_keys_skip_visible_page_matches() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(20, 3));
    app.handle_pty_output(
        b"u0@example.com\r\nu1@example.com\r\nu2@example.com\r\nu3@example.com\r\nu4@example.com",
    )
    .unwrap();

    app.enter_quick_select_mode();
    assert_eq!(active_quick_select_for_test(&app).matches.len(), 5);
    assert_eq!(app.selected_text().as_deref(), Some("u0@example.com"));

    assert!(
        app.handle_quick_select_logical_key(
            &Key::Named(NamedKey::PageDown),
            ModifiersState::empty()
        )
    );
    assert!(quick_select_for_test(&app).is_some());
    assert_eq!(app.selected_text().as_deref(), Some("u3@example.com"));

    assert!(
        app.handle_quick_select_logical_key(&Key::Named(NamedKey::PageUp), ModifiersState::empty())
    );
    assert!(quick_select_for_test(&app).is_some());
    assert_eq!(app.selected_text().as_deref(), Some("u0@example.com"));
}

#[test]
fn window_quick_select_uppercase_label_input_pastes_matching_candidate() {
    let written = Arc::new(Mutex::new(Vec::new()));
    let copied = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&copied);
    let mut app = NativeWindowApp::new(None);
    app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded.lock().unwrap().push(text.to_owned());
        true
    });
    app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
    app.handle_pty_output(b"https://example.com 10.0.0.1 test@x.io")
        .unwrap();

    app.enter_quick_select_mode();
    assert_eq!(active_quick_select_for_test(&app).matches.len(), 3);

    assert!(
        app.handle_quick_select_logical_key(&Key::Character("A".into()), ModifiersState::SHIFT)
    );

    assert!(quick_select_for_test(&app).is_none());
    assert!(app.selection.is_none());
    assert!(copied.lock().unwrap().is_empty());
    assert_eq!(written.lock().unwrap().as_slice(), b"test@x.io");
}

#[test]
fn window_quick_select_matches_wezterm_non_http_url_schemes() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(48, 1));
    app.handle_pty_output(b"git@github.com:wezterm/wezterm.git")
        .unwrap();

    app.enter_quick_select_mode();

    assert_eq!(active_quick_select_for_test(&app).matches.len(), 1);
    assert_eq!(
        app.selected_text().as_deref(),
        Some("git@github.com:wezterm/wezterm.git")
    );
}

#[test]
fn window_quick_select_matches_wezterm_path_and_hash_patterns() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
    app.handle_pty_output(b"/var/log/rterm.log deadbeef")
        .unwrap();

    app.enter_quick_select_mode();

    assert_eq!(active_quick_select_for_test(&app).matches.len(), 2);
    assert_eq!(app.selected_text().as_deref(), Some("/var/log/rterm.log"));
    assert!(app.quick_select_step(SearchDirection::Next));
    assert_eq!(app.selected_text().as_deref(), Some("deadbeef"));
}

#[test]
fn window_quick_select_matches_wezterm_capture_group_patterns() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(80, 3));
    app.handle_pty_output(
            b"[docs](https://example.com/path)\r\n--- a/src/main.rs\r\nsha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        )
        .unwrap();

    app.enter_quick_select_mode();

    assert_eq!(active_quick_select_for_test(&app).matches.len(), 3);
    assert_eq!(
        app.selected_text().as_deref(),
        Some("https://example.com/path")
    );
    assert!(app.quick_select_step(SearchDirection::Next));
    assert_eq!(app.selected_text().as_deref(), Some("src/main.rs"));
    assert!(app.quick_select_step(SearchDirection::Next));
    assert_eq!(
        app.selected_text().as_deref(),
        Some("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
    );
}

#[test]
fn window_title_includes_pane_select_status() {
    let mut app = NativeWindowApp::new(None);
    app.dispatch_app_action(AppAction::SplitPane {
        pane: rterm_core::PaneId::new(1),
        direction: rterm_core::app_shell::SplitDirection::Right,
        launch: None,
    })
    .unwrap();

    app.enter_pane_select_mode();

    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:2] - Pane Select: [2 panes]"
    );
}

#[test]
fn window_pane_select_activates_labelled_pane() {
    let mut app = NativeWindowApp::new(None);
    app.dispatch_app_action(AppAction::SplitPane {
        pane: rterm_core::PaneId::new(1),
        direction: rterm_core::app_shell::SplitDirection::Right,
        launch: None,
    })
    .unwrap();
    app.dispatch_app_action(AppAction::ActivatePane {
        pane: rterm_core::PaneId::new(1),
    })
    .unwrap();

    app.enter_pane_select_mode();

    let pane_select = app
        .pane_select
        .as_ref()
        .expect("pane select should be active");
    assert_eq!(pane_select.labels[0].label, "a");
    assert_eq!(pane_select.labels[0].pane_id, rterm_core::PaneId::new(1));
    assert_eq!(pane_select.labels[1].label, "s");
    assert_eq!(pane_select.labels[1].pane_id, rterm_core::PaneId::new(2));

    assert!(app.handle_pane_select_key(&Key::Character("s".into()), ModifiersState::empty()));

    assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
    assert!(app.pane_select.is_none());
}

#[test]
fn window_pane_select_uses_configured_quick_select_alphabet_for_labels() {
    let mut app = NativeWindowApp::new(None);
    app.set_config_overrides(native_config_snapshot! {
        quick_select_alphabet: Some("xy".to_owned()),
        ..NativeConfigSnapshot::default()
    });
    app.dispatch_app_action(AppAction::SplitPane {
        pane: rterm_core::PaneId::new(1),
        direction: rterm_core::app_shell::SplitDirection::Right,
        launch: None,
    })
    .unwrap();

    app.enter_pane_select_mode();

    let pane_select = app
        .pane_select
        .as_ref()
        .expect("pane select should be active");
    assert_eq!(pane_select.labels[0].label, "x");
    assert_eq!(pane_select.labels[0].pane_id, rterm_core::PaneId::new(1));
    assert_eq!(pane_select.labels[1].label, "y");
    assert_eq!(pane_select.labels[1].pane_id, rterm_core::PaneId::new(2));

    assert!(app.handle_pane_select_key(&Key::Character("y".into()), ModifiersState::empty()));

    assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
    assert!(app.pane_select.is_none());
}

#[test]
fn window_pane_select_renders_labels_over_panes() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(20, 4));
    app.refresh_snapshot();
    app.dispatch_app_action(AppAction::SplitPane {
        pane: rterm_core::PaneId::new(1),
        direction: rterm_core::app_shell::SplitDirection::Right,
        launch: None,
    })
    .unwrap();

    app.enter_pane_select_mode();

    let snapshot = app.render_snapshot();

    assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS + 2, 4), Some('a'));
    assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS + 2, 15), Some('s'));
}

#[test]
fn window_pane_select_renders_configured_overlay_colors() {
    let mut app = NativeWindowApp::new(None);
    app.set_config_overrides(native_config_snapshot! {
        pane_select_bg_color: Some(Color::Rgb(11, 22, 33)),
        pane_select_fg_color: Some(Color::Rgb(44, 55, 66)),
        ..NativeConfigSnapshot::default()
    });
    app.runtime.resize(rterm_core::TerminalSize::new(20, 4));
    app.refresh_snapshot();
    app.dispatch_app_action(AppAction::SplitPane {
        pane: rterm_core::PaneId::new(1),
        direction: rterm_core::app_shell::SplitDirection::Right,
        launch: None,
    })
    .unwrap();

    app.enter_pane_select_mode();

    let snapshot = app.render_snapshot();
    let label_cell =
        snapshot_cell(&snapshot, TAB_BAR_ROWS + 2, 4).expect("expected pane-select label cell");
    assert_eq!(label_cell.foreground, Color::Rgb(44, 55, 66));
    assert_eq!(label_cell.background, Color::Rgb(11, 22, 33));
}

#[test]
fn window_pane_select_can_render_pane_ids_alongside_labels() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(20, 4));
    app.refresh_snapshot();
    app.dispatch_app_action(AppAction::SplitPane {
        pane: rterm_core::PaneId::new(1),
        direction: rterm_core::app_shell::SplitDirection::Right,
        launch: None,
    })
    .unwrap();

    app.enter_command_palette_mode();
    app.command_palette_set_query("pane select show pane ids".to_owned());
    let command = app
        .command_palette_filtered_commands()
        .into_iter()
        .find(|command| command.label() == "Pane Select Show Pane IDs")
        .expect("expected pane select show ids command");
    app.command_palette_execute(command);

    let pane_select = app
        .pane_select
        .as_ref()
        .expect("pane select should be active");
    assert!(pane_select.show_pane_ids);

    let snapshot = app.render_snapshot();

    assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS + 2, 3), Some('a'));
    assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS + 2, 4), Some(':'));
    assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS + 2, 5), Some('1'));
    assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS + 2, 14), Some('s'));
    assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS + 2, 15), Some(':'));
    assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS + 2, 16), Some('2'));
}

#[test]
fn window_pane_select_swap_mode_swaps_layout_and_focuses_selected() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(20, 4));
    app.refresh_snapshot();
    app.dispatch_app_action(AppAction::SplitPane {
        pane: rterm_core::PaneId::new(1),
        direction: rterm_core::app_shell::SplitDirection::Right,
        launch: None,
    })
    .unwrap();
    app.dispatch_app_action(AppAction::ActivatePane {
        pane: rterm_core::PaneId::new(1),
    })
    .unwrap();

    app.enter_pane_select_mode_with_mode(WindowPaneSelectMode::SwapWithActive);
    assert!(app.handle_pane_select_key(&Key::Character("s".into()), ModifiersState::empty()));

    let layout = app.pane_render_layout();
    let selected_rect = layout
        .panes
        .iter()
        .find(|rect| rect.pane_id == rterm_core::PaneId::new(2))
        .expect("selected pane should still render");
    let active_rect = layout
        .panes
        .iter()
        .find(|rect| rect.pane_id == rterm_core::PaneId::new(1))
        .expect("active pane should still render");

    assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
    assert_eq!(selected_rect.column, 0);
    assert_eq!(active_rect.column, 10);
    assert!(app.pane_select.is_none());
}

#[test]
fn window_pane_select_swap_keep_focus_keeps_active_pane() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(20, 4));
    app.refresh_snapshot();
    app.dispatch_app_action(AppAction::SplitPane {
        pane: rterm_core::PaneId::new(1),
        direction: rterm_core::app_shell::SplitDirection::Right,
        launch: None,
    })
    .unwrap();
    app.dispatch_app_action(AppAction::ActivatePane {
        pane: rterm_core::PaneId::new(1),
    })
    .unwrap();

    app.enter_pane_select_mode_with_mode(WindowPaneSelectMode::SwapWithActiveKeepFocus);
    assert!(app.handle_pane_select_key(&Key::Character("s".into()), ModifiersState::empty()));

    let layout = app.pane_render_layout();
    let selected_rect = layout
        .panes
        .iter()
        .find(|rect| rect.pane_id == rterm_core::PaneId::new(2))
        .expect("selected pane should still render");
    let active_rect = layout
        .panes
        .iter()
        .find(|rect| rect.pane_id == rterm_core::PaneId::new(1))
        .expect("active pane should still render");

    assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
    assert_eq!(selected_rect.column, 0);
    assert_eq!(active_rect.column, 10);
    assert!(app.pane_select.is_none());
}

#[test]
fn window_pane_select_move_to_new_tab_moves_selected_pane_and_activates_tab() {
    let mut app = NativeWindowApp::new(None);
    app.dispatch_app_action(AppAction::SplitPane {
        pane: rterm_core::PaneId::new(1),
        direction: rterm_core::app_shell::SplitDirection::Right,
        launch: None,
    })
    .unwrap();
    app.dispatch_app_action(AppAction::ActivatePane {
        pane: rterm_core::PaneId::new(1),
    })
    .unwrap();

    app.enter_pane_select_mode_with_mode(WindowPaneSelectMode::MoveToNewTab);
    assert!(app.handle_pane_select_key(&Key::Character("s".into()), ModifiersState::empty()));

    assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
    assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
    assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
    assert_eq!(app.app_shell.active_workspace().tabs()[0].panes().len(), 1);
    assert_eq!(
        app.app_shell.active_workspace().tabs()[0].panes()[0].id(),
        rterm_core::PaneId::new(1)
    );
    assert_eq!(app.app_shell.active_tab().panes().len(), 1);
    assert_eq!(
        app.app_shell.active_tab().panes()[0].id(),
        rterm_core::PaneId::new(2)
    );
    assert!(app.pane_select.is_none());
}

#[test]
fn window_pane_select_move_to_new_window_detaches_selected_pane_and_requests_window() {
    let mut app = NativeWindowApp::new(None);
    app.dispatch_app_action(AppAction::SplitPane {
        pane: rterm_core::PaneId::new(1),
        direction: rterm_core::app_shell::SplitDirection::Right,
        launch: None,
    })
    .unwrap();
    app.dispatch_app_action(AppAction::ActivatePane {
        pane: rterm_core::PaneId::new(1),
    })
    .unwrap();

    app.enter_pane_select_mode_with_mode(WindowPaneSelectMode::MoveToNewWindow);
    assert!(app.handle_pane_select_key(&Key::Character("s".into()), ModifiersState::empty()));

    assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
    assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
    assert_eq!(app.app_shell.active_tab().panes().len(), 1);
    assert_eq!(
        app.app_shell.active_tab().panes()[0].id(),
        rterm_core::PaneId::new(1)
    );

    let pending_window = app
        .app_shell
        .pending_windows()
        .first()
        .expect("move should request a new window");
    assert_eq!(pending_window.id(), rterm_core::WindowId::new(2));
    assert_eq!(pending_window.active_tab_id(), rterm_core::TabId::new(2));
    assert_eq!(pending_window.active_pane_id(), rterm_core::PaneId::new(2));
    assert_eq!(pending_window.tab().panes().len(), 1);
    assert!(pending_window.tab().panes()[0].split().is_none());
    assert!(app.pane_select.is_none());
}

#[test]
fn window_app_consumes_pending_new_window_with_detached_pane_runtime() {
    let mut app = NativeWindowApp::new(None);
    app.handle_pty_output(b"left").unwrap();
    app.dispatch_app_action(AppAction::SplitPane {
        pane: rterm_core::PaneId::new(1),
        direction: rterm_core::app_shell::SplitDirection::Right,
        launch: None,
    })
    .unwrap();
    app.handle_pty_output(b"right").unwrap();
    app.dispatch_app_action(AppAction::ActivatePane {
        pane: rterm_core::PaneId::new(1),
    })
    .unwrap();

    app.dispatch_app_action(AppAction::MovePaneToNewWindow {
        pane: rterm_core::PaneId::new(2),
    })
    .unwrap();

    let detached_app = app
        .take_next_pending_window_app()
        .expect("pending window should create a detached app");

    assert!(app.app_shell.pending_windows().is_empty());
    assert_eq!(app.app_shell.pane_ids(), vec![rterm_core::PaneId::new(1)]);
    assert!(!app.pane_runtimes.contains_key(&rterm_core::PaneId::new(2)));
    assert_eq!(
        detached_app.active_workspace_id(),
        rterm_core::WorkspaceId::new(1)
    );
    assert_eq!(app.app_window_id_for_test(), rterm_core::WindowId::new(1));
    assert_eq!(
        detached_app.app_window_id_for_test(),
        rterm_core::WindowId::new(2)
    );
    assert_eq!(detached_app.active_tab_id(), rterm_core::TabId::new(2));
    assert_eq!(detached_app.active_pane_id(), rterm_core::PaneId::new(2));
    assert_eq!(snapshot_char(&detached_app.snapshot, 0, 0), Some('r'));
    assert_eq!(snapshot_char(&detached_app.snapshot, 0, 4), Some('t'));
}

#[test]
fn window_app_detached_new_window_inherits_effective_config() {
    let mut app = NativeWindowApp::new(None);
    app.set_config_overrides(sample_native_config_overrides!());
    let expected_config = app.native_effective_config();
    app.dispatch_app_action(AppAction::SplitPane {
        pane: rterm_core::PaneId::new(1),
        direction: rterm_core::app_shell::SplitDirection::Right,
        launch: None,
    })
    .unwrap();

    app.dispatch_app_action(AppAction::MovePaneToNewWindow {
        pane: rterm_core::PaneId::new(2),
    })
    .unwrap();

    let detached_app = app
        .take_next_pending_window_app()
        .expect("pending window should create a detached app");

    assert_eq!(detached_app.native_effective_config(), expected_config);
}

#[test]
fn detached_windows_inherit_cpu_and_auto_renderer_policy_without_renderer_instances() {
    for (mode, benchmark_startup) in [
        (crate::cli::RendererMode::Cpu, false),
        (crate::cli::RendererMode::Auto, false),
        (crate::cli::RendererMode::Cpu, true),
    ] {
        let mut app = NativeWindowApp::new(None);
        app.set_renderer_mode(mode);
        app.set_benchmark_startup(benchmark_startup);
        app.dispatch_app_action(AppAction::SpawnWindow { launch: None })
            .unwrap();

        let detached = app
            .take_next_pending_window_app()
            .expect("spawn should create a detached app");

        assert_eq!(detached.renderer_mode, mode);
        assert_eq!(detached.presentation_owner, PresentationOwner::Bootstrap);
        assert_eq!(detached.is_benchmark_startup(), benchmark_startup);
        assert!(detached.gpu_owners.active.is_none());
        assert!(detached.bootstrap_surface.is_none());
        assert!(detached.bootstrap_frame.is_empty());
    }
}

#[test]
fn manager_collected_auto_window_retains_bootstrap_renderer_policy() {
    let mut app = NativeWindowApp::new(None);
    app.set_renderer_mode(crate::cli::RendererMode::Auto);
    app.dispatch_app_action(AppAction::SpawnWindow { launch: None })
        .unwrap();
    let mut manager = NativeWindowManager::new_for_test(app);

    manager.collect_pending_window_apps_from_primary_for_test();

    let pending = manager
        .pending_app_for_test(0)
        .expect("manager should own the pending window");
    assert_eq!(pending.renderer_mode, crate::cli::RendererMode::Auto);
    assert_eq!(pending.presentation_owner, PresentationOwner::Bootstrap);
    assert!(pending.gpu_owners.active.is_none());
    assert!(pending.bootstrap_surface.is_none());
}

#[test]
fn benchmark_pending_window_never_starts_transport_during_materialization() {
    let mut app = NativeWindowApp::new(None);
    app.set_renderer_mode(crate::cli::RendererMode::Gpu);
    app.set_benchmark_startup(true);
    app.dispatch_app_action(AppAction::SpawnWindow { launch: None })
        .unwrap();

    let detached = app
        .take_next_pending_window_app()
        .expect("benchmark spawn should retain a detached app");

    assert_eq!(detached.renderer_mode, crate::cli::RendererMode::Gpu);
    assert_eq!(
        detached.presentation_owner,
        PresentationOwner::GpuInitializing
    );
    assert!(detached.is_benchmark_startup());
    assert!(!super::should_spawn_transport_during_materialization(
        detached.renderer_mode,
        detached.is_benchmark_startup(),
        detached.suppresses_transport_start(),
    ));
}

#[test]
fn window_focus_coordinator_transfers_exclusive_focus() {
    let mut focus = WindowFocusCoordinator::default();

    assert_eq!(
        focus.apply(10_u64, true),
        WindowFocusTransitions {
            blur: None,
            focus: Some(10),
        }
    );
    assert_eq!(focus.focused(), Some(10));
    assert_eq!(focus.apply(10, true), WindowFocusTransitions::default());
    assert_eq!(
        focus.apply(20, true),
        WindowFocusTransitions {
            blur: Some(10),
            focus: Some(20),
        }
    );
    assert_eq!(focus.focused(), Some(20));
    assert_eq!(focus.apply(10, false), WindowFocusTransitions::default());
    assert_eq!(
        focus.apply(20, false),
        WindowFocusTransitions {
            blur: Some(20),
            focus: None,
        }
    );
    assert_eq!(focus.focused(), None);
}

#[test]
fn window_manager_focus_dispatches_exclusive_app_transitions() {
    let first = rterm_core::WindowId::new(1);
    let second = rterm_core::WindowId::new(2);
    let mut focus = WindowFocusCoordinator::default();
    let changes = Arc::new(Mutex::new(Vec::new()));
    let mut apps = HashMap::new();

    let mut first_app = NativeWindowApp::new(None);
    let recorded = Arc::clone(&changes);
    first_app.focus_change_handler = Box::new(move |change| {
        recorded
            .lock()
            .unwrap()
            .push((change.window_id, change.focused));
        true
    });
    apps.insert(first, Box::new(first_app));

    let mut second_app = NativeWindowApp::new(None);
    second_app.app_window_id = second;
    let recorded = Arc::clone(&changes);
    second_app.focus_change_handler = Box::new(move |change| {
        recorded
            .lock()
            .unwrap()
            .push((change.window_id, change.focused));
        true
    });
    apps.insert(second, Box::new(second_app));

    for (window_id, focused) in [
        (first, true),
        (second, true),
        (first, false),
        (second, false),
    ] {
        dispatch_window_focus_changed(&mut focus, &mut apps, window_id, focused).unwrap();
    }

    assert_eq!(
        changes.lock().unwrap().as_slice(),
        [
            (first, true),
            (first, false),
            (second, true),
            (second, false),
        ]
    );
}

#[test]
fn window_manager_focus_ignores_unknown_and_removed_window_ids() {
    let known = rterm_core::WindowId::new(1);
    let unknown = rterm_core::WindowId::new(99);
    let mut focus = WindowFocusCoordinator::default();
    let changes = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&changes);
    let mut app = NativeWindowApp::new(None);
    app.focus_change_handler = Box::new(move |change| {
        recorded
            .lock()
            .unwrap()
            .push((change.window_id, change.focused));
        true
    });
    let mut apps = HashMap::from([(known, Box::new(app))]);

    dispatch_window_focus_changed(&mut focus, &mut apps, known, true).unwrap();
    dispatch_window_focus_changed(&mut focus, &mut apps, unknown, true).unwrap();

    assert_eq!(focus.focused(), Some(known));
    assert_eq!(changes.lock().unwrap().as_slice(), [(known, true)]);

    apps.remove(&known);
    assert!(focus.remove(known));
    dispatch_window_focus_changed(&mut focus, &mut apps, known, true).unwrap();

    assert_eq!(focus.focused(), None);
    assert_eq!(changes.lock().unwrap().as_slice(), [(known, true)]);
}

#[test]
fn window_focus_coordinator_forgets_removed_focus_owner() {
    let mut focus = WindowFocusCoordinator::default();
    focus.apply(10_u64, true);

    assert!(!focus.remove(20));
    assert!(focus.remove(10));
    assert_eq!(focus.focused(), None);
}

#[test]
fn window_manager_collects_detached_app_after_move_to_new_window() {
    let mut app = NativeWindowApp::new(None);
    app.handle_pty_output(b"left").unwrap();
    app.dispatch_app_action(AppAction::SplitPane {
        pane: rterm_core::PaneId::new(1),
        direction: rterm_core::app_shell::SplitDirection::Right,
        launch: None,
    })
    .unwrap();
    app.handle_pty_output(b"right").unwrap();
    app.dispatch_app_action(AppAction::MovePaneToNewWindow {
        pane: rterm_core::PaneId::new(2),
    })
    .unwrap();

    let mut manager = NativeWindowManager::new_for_test(app);
    manager.collect_pending_window_apps_from_primary_for_test();

    assert_eq!(manager.pending_app_count_for_test(), 1);
    let detached_app = manager
        .pending_app_for_test(0)
        .expect("manager should hold detached window app");
    assert_eq!(detached_app.active_pane_id(), rterm_core::PaneId::new(2));
    assert_eq!(snapshot_char(&detached_app.snapshot, 0, 0), Some('r'));
    assert_eq!(snapshot_char(&detached_app.snapshot, 0, 4), Some('t'));
}

#[test]
fn window_manager_detached_app_inherits_derived_config_environment() {
    let mut app = NativeWindowApp::new(None);
    app.derived_config_environment = BTreeMap::from([
        (
            "WEZTERM_CONFIG_FILE".to_owned(),
            "/derived/wezterm.lua".to_owned(),
        ),
        ("WEZTERM_CONFIG_DIR".to_owned(), "/derived".to_owned()),
    ]);
    app.set_environment_variables = BTreeMap::from([(
        "WEZTERM_CONFIG_FILE".to_owned(),
        "/user/wezterm.lua".to_owned(),
    )]);
    app.dispatch_app_action(AppAction::SpawnWindow { launch: None })
        .unwrap();

    let mut manager = NativeWindowManager::new_for_test(app);
    manager.collect_pending_window_apps_from_primary_for_test();
    let detached = manager
        .pending_app_for_test(0)
        .expect("manager should collect the detached app");
    let environment = detached.pane_environment_variables();
    assert_eq!(
        environment.get("WEZTERM_CONFIG_FILE").map(String::as_str),
        Some("/user/wezterm.lua"),
        "the user environment must continue to override lifecycle publication"
    );
    assert_eq!(
        environment.get("WEZTERM_CONFIG_DIR").map(String::as_str),
        Some("/derived")
    );

    let command = pty_command_from_pane_launch_with_environment(
        detached.app_shell.active_pane().launch(),
        &detached.term,
        &environment,
        detached.default_cwd.as_deref(),
    );
    assert_eq!(
        command.env_value("WEZTERM_CONFIG_FILE"),
        Some("/user/wezterm.lua")
    );
    assert_eq!(command.env_value("WEZTERM_CONFIG_DIR"), Some("/derived"));
}

#[test]
fn window_manager_collects_spawn_window_app_from_default_shortcut() {
    let app = NativeWindowApp::new_with_command(
        None,
        rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
    );
    let mut manager = NativeWindowManager::new_for_test(app);
    {
        let primary = manager.primary_app_mut_for_test();
        let action = primary
            .app_shell_action_for_key(
                &Key::Character("n".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT,
            )
            .expect("expected spawn window action");
        assert!(matches!(&action, AppAction::SpawnWindow { launch: None }));
        primary.dispatch_app_action(action).unwrap();
        primary
            .dispatch_app_action(AppAction::SpawnWindow { launch: None })
            .unwrap();
    }

    manager.collect_pending_window_apps_from_primary_for_test();

    assert_eq!(manager.pending_app_count_for_test(), 2);
    let spawned_app = manager
        .pending_app_for_test(0)
        .expect("manager should hold the first spawned window app");
    assert_eq!(
        spawned_app.app_window_id_for_test(),
        rterm_core::WindowId::new(2)
    );
    assert_eq!(spawned_app.active_tab_id(), rterm_core::TabId::new(2));
    assert_eq!(spawned_app.active_pane_id(), rterm_core::PaneId::new(2));
    let second_spawned_app = manager
        .pending_app_for_test(1)
        .expect("manager should hold the second spawned window app");
    assert_eq!(
        second_spawned_app.app_window_id_for_test(),
        rterm_core::WindowId::new(3)
    );
}

#[test]
fn window_manager_quit_application_drops_startup_and_pending_apps() {
    let mut manager = NativeWindowManager::new_for_test(NativeWindowApp::new(None));
    {
        let primary = manager.primary_app_mut_for_test();
        primary
            .dispatch_app_action(AppAction::SpawnWindow { launch: None })
            .unwrap();
    }
    manager.collect_pending_window_apps_from_primary_for_test();
    assert_eq!(manager.pending_app_count_for_test(), 1);

    manager.quit_application_from_primary_for_test();

    assert_eq!(manager.pending_app_count_for_test(), 0);
    assert_eq!(manager.startup_app_count_for_test(), 0);
    assert!(manager.last_metrics_for_test().is_some());
}

#[test]
fn window_manager_can_keep_running_after_last_window_closes() {
    let mut app = NativeWindowApp::new(None);
    app.set_config_overrides(native_config_snapshot! {
        quit_when_all_windows_are_closed: Some(false),
        ..NativeConfigSnapshot::default()
    });
    let mut manager = NativeWindowManager::new_for_test(app);
    manager.discard_startup_app_for_test();

    assert!(!manager.should_exit_when_idle_for_test());
}

#[test]
fn window_manager_defers_closed_gpu_destruction_until_a_safe_event_loop_point() {
    let mut active = NativeWindowApp::new(None);
    active.set_config_overrides(native_config_snapshot! {
        quit_when_all_windows_are_closed: Some(false),
        ..NativeConfigSnapshot::default()
    });
    let mut closing = NativeWindowApp::new(None);
    closing.gpu_owners.active = Some(Box::new(
        crate::window_gpu::WindowGpu::for_manager_close_test(false, true),
    ));
    let mut manager = NativeWindowManager::new_for_test(active);
    let closing_id = winit::window::WindowId::dummy();
    manager.windows.insert(closing_id, Box::new(closing));

    assert!(!manager.close_window(closing_id));
    assert_eq!(manager.retired_app_count_for_test(), 1);
    assert!(manager.windows.is_empty());

    manager.reap_retired_apps();
    assert_eq!(manager.retired_app_count_for_test(), 0);
}

#[test]
fn window_manager_abandons_recovered_eligible_window_while_another_window_remains_active() {
    let mut recovered = NativeWindowApp::new(None);
    recovered.gpu_owners.active = Some(Box::new(
        crate::window_gpu::WindowGpu::for_manager_close_test(true, true),
    ));
    let mut manager = NativeWindowManager::new_for_test(NativeWindowApp::new(None));
    let mut survivor = manager
        .startup_app
        .take()
        .expect("surviving app starts manager-owned");
    survivor
        .handle_pty_output(b"alive")
        .expect("survivor accepts output before peer close");
    let survivor_id = winit::window::WindowId::from(1_u64);
    let recovered_id = winit::window::WindowId::from(2_u64);
    manager.windows.insert(survivor_id, survivor);
    manager.windows.insert(recovered_id, Box::new(recovered));

    assert!(
        !manager.close_window(recovered_id),
        "the other materialized window must keep the application alive"
    );
    assert_eq!(manager.windows.len(), 1);
    assert!(manager.windows.contains_key(&survivor_id));
    manager
        .windows
        .get_mut(&survivor_id)
        .expect("surviving materialized window")
        .handle_pty_output(b"!")
        .expect("survivor continues processing output after peer close");
    let metrics: serde_json::Value =
        serde_json::from_str(&manager.metrics_json_report().expect("aggregate metrics"))
            .expect("metrics JSON");
    assert_eq!(metrics["pty_chunks"], 2);
    assert_eq!(metrics["pty_bytes"], 6);
    assert_eq!(metrics["gpu_abandoned_lost_surfaces"], 1);
    assert_eq!(manager.closed_gpu_abandonments_for_test(), 1);

    for (eligible, replaced) in [(false, true), (true, false)] {
        let active = NativeWindowApp::new(None);
        let mut ordinary = NativeWindowApp::new(None);
        ordinary.gpu_owners.active = Some(Box::new(
            crate::window_gpu::WindowGpu::for_manager_close_test(eligible, replaced),
        ));
        let mut manager = NativeWindowManager::new_for_test(active);
        let ordinary_id = winit::window::WindowId::dummy();
        manager.windows.insert(ordinary_id, Box::new(ordinary));

        assert!(!manager.close_window(ordinary_id));
        assert_eq!(manager.closed_gpu_abandonments_for_test(), 0);
    }
}

#[test]
fn window_manager_aggregates_closed_and_all_live_gpu_abandonments_on_application_exit() {
    let mut manager = NativeWindowManager::new_for_test(NativeWindowApp::new(None));
    manager.startup_app = None;
    for (raw_id, chunks) in [(1_u64, 1_usize), (2_u64, 2_usize), (3_u64, 0_usize)] {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..chunks {
            app.handle_pty_output(b"x")
                .expect("record representative PTY metric");
        }
        app.gpu_owners.active = Some(Box::new(
            crate::window_gpu::WindowGpu::for_manager_close_test(true, true),
        ));
        manager
            .windows
            .insert(winit::window::WindowId::from(raw_id), Box::new(app));
    }

    assert!(!manager.close_window(winit::window::WindowId::from(3_u64)));
    assert_eq!(manager.closed_gpu_abandonments_for_test(), 1);
    manager.shutdown_gpu_for_application_exit();

    let selected = manager
        .metrics_app()
        .expect("one live app remains selected for representative metrics")
        .metrics_snapshot();
    let aggregated = manager.aggregated_metrics_snapshot();
    assert_eq!(aggregated.gpu_abandoned_lost_surfaces, 3);
    assert_eq!(aggregated.pty_chunks, selected.pty_chunks);
    assert_eq!(aggregated.pty_bytes, selected.pty_bytes);
}

#[test]
fn window_manager_application_exit_applies_native_close_policy_to_every_gpu_owner() {
    fn native_close_gpu_app() -> Box<NativeWindowApp> {
        let mut app = NativeWindowApp::new(None);
        app.gpu_owners.active = Some(Box::new(
            crate::window_gpu::WindowGpu::for_manager_close_test(true, false),
        ));
        Box::new(app)
    }

    let mut manager = NativeWindowManager::new_for_test(native_close_gpu_app());
    manager
        .windows
        .insert(winit::window::WindowId::from(1_u64), native_close_gpu_app());
    manager.pending_apps.push(native_close_gpu_app());
    manager.retired_apps.push(native_close_gpu_app());

    manager.shutdown_gpu_for_application_exit();

    let owners_left_for_unsafe_native_teardown = manager
        .startup_app
        .iter_mut()
        .chain(manager.windows.values_mut())
        .chain(manager.pending_apps.iter_mut())
        .chain(manager.retired_apps.iter_mut())
        .map(|app| {
            app.gpu_owners
                .active
                .as_mut()
                .is_some_and(|gpu| gpu.shutdown_after_native_window_close())
        })
        .filter(|left_for_unsafe_teardown| *left_for_unsafe_teardown)
        .count();
    assert_eq!(
        owners_left_for_unsafe_native_teardown, 0,
        "application exit must finalize every GPU owner with the native-close policy"
    );
}

#[test]
fn direct_window_application_exit_uses_native_close_gpu_policy() {
    let source = include_str!("../window_parts/part15.rs");
    let direct_handler = source
        .split("impl ApplicationHandler<WindowUserEvent> for NativeWindowApp {")
        .nth(1)
        .expect("direct NativeWindowApp event handler");
    let exiting = direct_handler
        .split("fn exiting")
        .nth(1)
        .expect("direct application-exit callback")
        .split("fn window_event")
        .next()
        .expect("bounded direct application-exit callback");

    assert!(
        exiting.contains("self.shutdown_gpu_after_native_window_close();"),
        "direct application exit must apply the final native-close GPU policy"
    );
}

#[test]
fn window_pane_select_cancel_keys_exit_without_focusing() {
    let mut app = NativeWindowApp::new(None);
    app.dispatch_app_action(AppAction::SplitPane {
        pane: rterm_core::PaneId::new(1),
        direction: rterm_core::app_shell::SplitDirection::Right,
        launch: None,
    })
    .unwrap();
    app.dispatch_app_action(AppAction::ActivatePane {
        pane: rterm_core::PaneId::new(1),
    })
    .unwrap();

    app.enter_pane_select_mode();
    assert!(app.handle_pane_select_key(&Key::Named(NamedKey::Escape), ModifiersState::empty()));

    assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
    assert!(app.pane_select.is_none());

    app.enter_pane_select_mode();
    assert!(app.handle_pane_select_key(&Key::Character("g".into()), ModifiersState::CONTROL));

    assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
    assert!(app.pane_select.is_none());
}

#[test]
fn recognizes_window_copy_mode_shortcut() {
    assert!(window_copy_mode_shortcut(
        &Key::Character("x".into()),
        ModifiersState::CONTROL | ModifiersState::SHIFT
    ));
    assert!(window_copy_mode_shortcut(
        &Key::Character("X".into()),
        ModifiersState::CONTROL | ModifiersState::SHIFT
    ));
    assert!(!window_copy_mode_shortcut(
        &Key::Character("x".into()),
        ModifiersState::CONTROL
    ));
}

#[test]
fn window_title_includes_copy_mode_status() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 1));
    app.handle_pty_output(b"abcd").unwrap();

    app.enter_copy_mode();
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1] - Copy Mode"
    );

    app.handle_copy_mode_key(&Key::Character("v".into()), ModifiersState::empty());
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1] - Copy Mode: Cell"
    );

    app.handle_copy_mode_key(&Key::Named(NamedKey::Escape), ModifiersState::empty());
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1]"
    );
}

#[test]
fn window_copy_mode_escape_character_closes_copy_mode() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 1));
    app.handle_pty_output(b"abcd").unwrap();

    app.enter_copy_mode();
    assert!(copy_mode_for_test(&app).is_some());

    assert!(app.handle_copy_mode_key(&Key::Character("\u{1b}".into()), ModifiersState::empty()));

    assert!(copy_mode_for_test(&app).is_none());
    assert!(search_for_test(&app).is_none());
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1]"
    );
}

#[test]
fn window_copy_mode_allows_command_palette_fallback_shortcut() {
    let mut app = NativeWindowApp::new(None);
    app.enter_copy_mode();

    assert!(copy_mode_for_test(&app).is_some());

    assert!(app.handle_copy_mode_key(
        &Key::Character("p".into()),
        ModifiersState::CONTROL | ModifiersState::SHIFT
    ));

    assert!(copy_mode_for_test(&app).is_some());
    assert!(app.command_palette.is_some());
    assert_eq!(
        app.effective_window_title(),
        format!(
            "R-Term [workspace:1 tab:1 pane:1] - Command Palette: [1 / {}] New Tab",
            WINDOW_COMMANDS.len()
        )
    );
}

#[test]
fn window_copy_mode_search_allows_command_palette_fallback_shortcut() {
    let mut app = NativeWindowApp::new(None);
    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));

    assert!(copy_mode_for_test(&app).is_some());
    assert!(search_for_test(&app).is_some());

    assert!(app.handle_copy_mode_key(
        &Key::Character("p".into()),
        ModifiersState::CONTROL | ModifiersState::SHIFT
    ));

    assert!(copy_mode_for_test(&app).is_some());
    assert!(search_for_test(&app).is_some());
    assert!(app.command_palette.is_some());
    assert_eq!(
        app.effective_window_title(),
        format!(
            "R-Term [workspace:1 tab:1 pane:1] - Command Palette: [1 / {}] New Tab",
            WINDOW_COMMANDS.len()
        )
    );
}

#[test]
fn window_copy_mode_allows_new_tab_fallback_shortcut() {
    let mut app = NativeWindowApp::new(None);
    app.enter_copy_mode();
    let tab_order = |app: &NativeWindowApp| -> Vec<rterm_core::TabId> {
        app.app_shell
            .active_workspace()
            .tabs()
            .iter()
            .map(rterm_core::app_shell::Tab::id)
            .collect()
    };

    assert_eq!(tab_order(&app), vec![rterm_core::TabId::new(1)]);
    assert!(copy_mode_for_test(&app).is_some());

    assert!(app.handle_copy_mode_key(
        &Key::Character("t".into()),
        ModifiersState::CONTROL | ModifiersState::SHIFT
    ));

    assert_eq!(
        tab_order(&app),
        vec![rterm_core::TabId::new(1), rterm_core::TabId::new(2)]
    );
    assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
    assert!(copy_mode_for_test(&app).is_none());
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:2 pane:2]"
    );
}

#[test]
fn window_copy_mode_search_allows_new_tab_fallback_shortcut() {
    let mut app = NativeWindowApp::new(None);
    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    let tab_order = |app: &NativeWindowApp| -> Vec<rterm_core::TabId> {
        app.app_shell
            .active_workspace()
            .tabs()
            .iter()
            .map(rterm_core::app_shell::Tab::id)
            .collect()
    };

    assert_eq!(tab_order(&app), vec![rterm_core::TabId::new(1)]);
    assert!(copy_mode_for_test(&app).is_some());
    assert!(search_for_test(&app).is_some());

    assert!(app.handle_copy_mode_key(
        &Key::Character("t".into()),
        ModifiersState::CONTROL | ModifiersState::SHIFT
    ));

    assert_eq!(
        tab_order(&app),
        vec![rterm_core::TabId::new(1), rterm_core::TabId::new(2)]
    );
    assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
    assert!(copy_mode_for_test(&app).is_none());
    assert!(search_for_test(&app).is_none());
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:2 pane:2]"
    );
}

#[test]
fn window_copy_mode_cell_selection_copies_and_exits() {
    let copied = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&copied);
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 1));
    app.handle_pty_output(b"abcd").unwrap();
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded.lock().unwrap().push(text.to_owned());
        true
    });

    app.enter_copy_mode();
    app.handle_copy_mode_key(&Key::Character("v".into()), ModifiersState::empty());
    app.handle_copy_mode_key(&Key::Character("h".into()), ModifiersState::empty());
    app.handle_copy_mode_key(&Key::Character("h".into()), ModifiersState::empty());
    app.handle_copy_mode_key(&Key::Character("h".into()), ModifiersState::empty());
    app.handle_copy_mode_key(&Key::Character("y".into()), ModifiersState::empty());

    assert!(copy_mode_for_test(&app).is_none());
    assert_eq!(copied.lock().unwrap().as_slice(), ["abcd"]);
}

#[test]
fn window_copy_mode_uses_configured_active_highlight_colors() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 1));
    app.set_config_overrides(native_config_snapshot! {
        copy_mode_active_highlight_bg: Some(NativeColorSpec::Color(Color::Rgb(1, 2, 3))),
        copy_mode_active_highlight_fg: Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Navy)),
        ..NativeConfigSnapshot::default()
    });
    app.handle_pty_output(b"abcd").unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("v".into()), ModifiersState::empty()));
    let selected_cell = rendered_active_pane_cell(&app, 0, 3).expect("copy-mode selected cell");

    assert_eq!(selected_cell.foreground, Color::Indexed(4));
    assert_eq!(selected_cell.background, Color::Rgb(1, 2, 3));
    assert!(!selected_cell.inverse);
}

#[test]
fn window_copy_mode_search_uses_configured_inactive_highlight_colors() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 3));
    app.set_config_overrides(native_config_snapshot! {
        copy_mode_active_highlight_bg: Some(NativeColorSpec::Color(Color::Rgb(1, 2, 3))),
        copy_mode_active_highlight_fg: Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Navy)),
        copy_mode_inactive_highlight_bg: Some(NativeColorSpec::Color(Color::Rgb(4, 5, 6))),
        copy_mode_inactive_highlight_fg: Some(NativeColorSpec::AnsiColor(
            NativeAnsiColor::White,
        )),
        ..NativeConfigSnapshot::default()
    });
    app.handle_pty_output(b"foo one\r\nmiddle\r\nfoo two")
        .unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    for character in ["f", "o", "o"] {
        assert!(
            app.handle_copy_mode_key(&Key::Character(character.into()), ModifiersState::empty())
        );
    }

    let active_cell = rendered_active_pane_cell(&app, 0, 0).expect("active copy-mode match");
    let inactive_cell = rendered_active_pane_cell(&app, 2, 0).expect("inactive copy-mode match");

    assert_eq!(active_cell.foreground, Color::Indexed(4));
    assert_eq!(active_cell.background, Color::Rgb(1, 2, 3));
    assert_eq!(inactive_cell.foreground, Color::Indexed(15));
    assert_eq!(inactive_cell.background, Color::Rgb(4, 5, 6));
    assert!(!active_cell.inverse);
    assert!(!inactive_cell.inverse);
}

#[test]
fn window_copy_mode_y_copies_to_clipboard_and_primary_then_scrolls_bottom() {
    let clipboard = Arc::new(Mutex::new(Vec::new()));
    let primary = Arc::new(Mutex::new(Vec::new()));
    let recorded_clipboard = Arc::clone(&clipboard);
    let recorded_primary = Arc::clone(&primary);
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
    app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee\r\nff")
        .unwrap();
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded_clipboard.lock().unwrap().push(text.to_owned());
        true
    });
    app.primary_selection_writer = Box::new(move |text: &str| {
        recorded_primary.lock().unwrap().push(text.to_owned());
        true
    });

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("g".into()), ModifiersState::empty()));
    assert!(app.handle_copy_mode_key(&Key::Character("V".into()), ModifiersState::empty()));
    assert_eq!(app.current_scrollback_offset(), 4);

    assert!(app.handle_copy_mode_key(&Key::Character("y".into()), ModifiersState::empty()));

    assert!(copy_mode_for_test(&app).is_none());
    assert_eq!(app.current_scrollback_offset(), 0);
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "ee  ");
    assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "ff  ");
    assert_eq!(clipboard.lock().unwrap().as_slice(), ["aa"]);
    assert_eq!(primary.lock().unwrap().as_slice(), ["aa"]);
}

#[test]
fn window_copy_mode_space_uses_wezterm_cell_selection_binding() {
    let copied = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&copied);
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 1));
    app.handle_pty_output(b"abcd").unwrap();
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded.lock().unwrap().push(text.to_owned());
        true
    });

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character(" ".into()), ModifiersState::empty()));
    app.handle_copy_mode_key(&Key::Character("h".into()), ModifiersState::empty());
    app.handle_copy_mode_key(&Key::Character("h".into()), ModifiersState::empty());
    app.handle_copy_mode_key(&Key::Character("h".into()), ModifiersState::empty());
    app.handle_copy_mode_key(&Key::Character("y".into()), ModifiersState::empty());

    assert!(copy_mode_for_test(&app).is_none());
    assert_eq!(copied.lock().unwrap().as_slice(), ["abcd"]);
}

#[test]
fn window_copy_mode_line_selection() {
    let copied = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&copied);
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(6, 1));
    app.handle_pty_output(b"abcdef").unwrap();
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded.lock().unwrap().push(text.to_owned());
        true
    });

    app.enter_copy_mode();
    app.handle_copy_mode_key(&Key::Character("V".into()), ModifiersState::SHIFT);
    app.handle_copy_mode_key(&Key::Character("y".into()), ModifiersState::empty());

    assert!(copy_mode_for_test(&app).is_none());
    assert_eq!(copied.lock().unwrap().as_slice(), ["abcdef"]);
}

#[test]
fn window_copy_mode_uppercase_v_no_modifier_uses_line_selection_binding() {
    let copied = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&copied);
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(6, 1));
    app.handle_pty_output(b"abcdef").unwrap();
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded.lock().unwrap().push(text.to_owned());
        true
    });

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("V".into()), ModifiersState::empty()));
    app.handle_copy_mode_key(&Key::Character("y".into()), ModifiersState::empty());

    assert!(copy_mode_for_test(&app).is_none());
    assert_eq!(copied.lock().unwrap().as_slice(), ["abcdef"]);
}

#[test]
fn window_copy_mode_moves_by_semantic_zone() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(12, 4));
    app.handle_pty_output(
        b"ready\r\n\x1b]133;A\x07> \x1b]133;B\x07ls -l\r\n\x1b]133;C\x07file.txt",
    )
    .unwrap();

    app.enter_copy_mode();
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 2, column: 8 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("z".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 2, column: 0 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("z".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 2 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("Z".into()), ModifiersState::SHIFT));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 2, column: 0 })
    );
}

#[test]
fn window_copy_mode_moves_by_output_semantic_zone_type() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(12, 4));
    app.handle_pty_output(
        b"ready\r\n\x1b]133;A\x07> \x1b]133;B\x07ls -l\r\n\x1b]133;C\x07file.txt",
    )
    .unwrap();

    app.enter_copy_mode();
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 2, column: 8 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("z".into()), ModifiersState::ALT));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 2, column: 0 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("z".into()), ModifiersState::ALT));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 0 })
    );

    assert!(app.handle_copy_mode_key(
        &Key::Character("Z".into()),
        ModifiersState::ALT | ModifiersState::SHIFT
    ));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 2, column: 0 })
    );
}

#[test]
fn window_copy_mode_moves_by_prompt_and_input_semantic_zone_types() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(12, 4));
    app.handle_pty_output(
        b"ready\r\n\x1b]133;A\x07> \x1b]133;B\x07ls -l\r\n\x1b]133;C\x07file.txt",
    )
    .unwrap();

    app.enter_copy_mode();
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 2, column: 8 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("p".into()), ModifiersState::ALT));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 0 })
    );

    assert!(app.handle_copy_mode_key(
        &Key::Character("I".into()),
        ModifiersState::ALT | ModifiersState::SHIFT
    ));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 2 })
    );
}

#[test]
fn window_copy_mode_dispatches_wezterm_semantic_zone_type_assignment_queries() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(12, 4));
    app.handle_pty_output(
        b"ready\r\n\x1b]133;A\x07> \x1b]133;B\x07ls -l\r\n\x1b]133;C\x07file.txt",
    )
    .unwrap();

    app.enter_copy_mode();
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 2, column: 8 })
    );

    let command = super::command_palette_structured_query_command(
        "wezterm.action.CopyMode { MoveBackwardSemanticZoneOfType = 'Prompt' }",
    )
    .expect("expected CopyMode backward semantic-zone type assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode backward semantic-zone type should dispatch");

    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 0 })
    );

    let command = super::command_palette_structured_query_command(
        "wezterm.action.CopyMode { MoveForwardSemanticZoneOfType = 'Input' }",
    )
    .expect("expected CopyMode forward semantic-zone type assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode forward semantic-zone type should dispatch");

    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 2 })
    );
}

#[test]
fn window_copy_mode_dispatches_wezterm_zone_type_assignment_queries() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(12, 4));
    app.handle_pty_output(
        b"ready\r\n\x1b]133;A\x07> \x1b]133;B\x07ls -l\r\n\x1b]133;C\x07file.txt",
    )
    .unwrap();

    app.enter_copy_mode();
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 2, column: 8 })
    );

    let command = super::command_palette_structured_query_command(
        "wezterm.action.CopyMode { MoveBackwardZoneOfType = 'Prompt' }",
    )
    .expect("expected CopyMode backward zone type assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode backward zone type should dispatch");

    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 0 })
    );

    let command = super::command_palette_structured_query_command(
        "wezterm.action.CopyMode { MoveForwardZoneOfType = 'Input' }",
    )
    .expect("expected CopyMode forward zone type assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode forward zone type should dispatch");

    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 2 })
    );
}

#[test]
fn window_copy_mode_semantic_zone_movement_scrolls_into_scrollback() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(12, 2));
    app.handle_pty_output(
            b"\x1b]133;C\x07oldout\r\n\x1b]133;A\x07> one\r\n\x1b]133;C\x07midout\r\n\x1b]133;A\x07> two\r\n\x1b]133;C\x07live",
        )
        .unwrap();

    assert_eq!(app.runtime.terminal().scrollback().len(), 3);
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 12), "> two       ");
    assert_eq!(snapshot_row_text(&app.snapshot, 1, 12), "live        ");

    app.enter_copy_mode();
    assert_eq!(app.current_scrollback_offset(), 0);
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 4 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("z".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 0 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("z".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 0 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("z".into()), ModifiersState::empty()));
    assert_eq!(app.current_scrollback_offset(), 1);
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 0 })
    );
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 12), "midout      ");
    assert_eq!(snapshot_row_text(&app.snapshot, 1, 12), "> two       ");
}

#[test]
fn window_copy_mode_selection_copies_across_scrollback_viewports() {
    let copied = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&copied);
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(12, 2));
    app.handle_pty_output(
            b"\x1b]133;C\x07oldout\r\n\x1b]133;A\x07> one\r\n\x1b]133;C\x07midout\r\n\x1b]133;A\x07> two\r\n\x1b]133;C\x07live",
        )
        .unwrap();
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded.lock().unwrap().push(text.to_owned());
        true
    });

    app.enter_copy_mode();
    app.handle_copy_mode_key(&Key::Character("v".into()), ModifiersState::empty());
    app.handle_copy_mode_key(&Key::Character("z".into()), ModifiersState::empty());
    app.handle_copy_mode_key(&Key::Character("z".into()), ModifiersState::empty());
    app.handle_copy_mode_key(&Key::Character("z".into()), ModifiersState::empty());

    assert_eq!(app.current_scrollback_offset(), 1);
    assert_eq!(app.selected_text().as_deref(), Some("midout\n> two\nlive"));
    assert!(rendered_active_pane_cell(&app, 0, 0).unwrap().inverse);
    assert!(rendered_active_pane_cell(&app, 1, 0).unwrap().inverse);

    app.handle_copy_mode_key(&Key::Character("y".into()), ModifiersState::empty());

    assert!(copy_mode_for_test(&app).is_none());
    assert_eq!(copied.lock().unwrap().as_slice(), ["midout\n> two\nlive"]);
}

#[test]
fn window_copy_mode_o_moves_to_selection_other_end() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 3));
    app.handle_pty_output(b"abcd\r\nefgh\r\nijkl").unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("v".into()), ModifiersState::empty()));
    assert!(app.handle_copy_mode_key(&Key::Character("k".into()), ModifiersState::empty()));
    assert!(app.handle_copy_mode_key(&Key::Character("h".into()), ModifiersState::empty()));
    assert!(app.handle_copy_mode_key(&Key::Character("h".into()), ModifiersState::empty()));
    assert_eq!(app.selected_text().as_deref(), Some("gh\nijkl"));

    assert!(app.handle_copy_mode_key(&Key::Character("o".into()), ModifiersState::empty()));

    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| (
            copy_mode.source_cursor.row,
            copy_mode.source_cursor.column,
            copy_mode
                .source_anchor
                .map(|anchor| (anchor.row, anchor.column))
        )),
        Some((
            app.runtime.terminal().stable_dimensions().physical_top + 2,
            4,
            Some((
                app.runtime.terminal().stable_dimensions().physical_top + 1,
                2
            ))
        ))
    );
    assert_eq!(app.selected_text().as_deref(), Some("gh\nijkl"));
}

#[test]
fn window_copy_mode_shift_o_moves_to_selection_other_horizontal_end() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 3));
    app.handle_pty_output(b"abcd\r\nefgh\r\nijkl").unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("v".into()), ModifiersState::empty()));
    assert!(app.handle_copy_mode_key(&Key::Character("k".into()), ModifiersState::empty()));
    assert!(app.handle_copy_mode_key(&Key::Character("h".into()), ModifiersState::empty()));
    assert!(app.handle_copy_mode_key(&Key::Character("h".into()), ModifiersState::empty()));

    assert!(app.handle_copy_mode_key(&Key::Character("O".into()), ModifiersState::SHIFT));

    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| (
            copy_mode.source_cursor.row,
            copy_mode.source_cursor.column,
            copy_mode
                .source_anchor
                .map(|anchor| (anchor.row, anchor.column))
        )),
        Some((
            app.runtime.terminal().stable_dimensions().physical_top + 1,
            4,
            Some((
                app.runtime.terminal().stable_dimensions().physical_top + 2,
                2
            ))
        ))
    );
}

#[test]
fn window_copy_mode_ctrl_v_uses_block_selection() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(5, 3));
    app.handle_pty_output(b"abcde\r\nfghij\r\nklmno").unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("v".into()), ModifiersState::CONTROL));
    assert!(app.handle_copy_mode_key(&Key::Character("k".into()), ModifiersState::empty()));
    assert!(app.handle_copy_mode_key(&Key::Character("k".into()), ModifiersState::empty()));
    assert!(app.handle_copy_mode_key(&Key::Character("h".into()), ModifiersState::empty()));
    assert!(app.handle_copy_mode_key(&Key::Character("h".into()), ModifiersState::empty()));

    assert_eq!(app.selected_text().as_deref(), Some("cde\nhij\nmno"));
    assert!(rendered_active_pane_cell(&app, 0, 2).unwrap().inverse);
    assert!(rendered_active_pane_cell(&app, 1, 2).unwrap().inverse);
    assert!(rendered_active_pane_cell(&app, 1, 4).unwrap().inverse);
    assert!(!rendered_active_pane_cell(&app, 1, 1).unwrap().inverse);
}

#[test]
fn window_copy_mode_vertical_movement_scrolls_across_scrollback() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
    app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
        .unwrap();

    app.enter_copy_mode();
    assert_eq!(app.current_scrollback_offset(), 0);
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 2 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("k".into()), ModifiersState::empty()));
    assert_eq!(app.current_scrollback_offset(), 0);
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 2 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("k".into()), ModifiersState::empty()));
    assert_eq!(app.current_scrollback_offset(), 1);
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "cc  ");
    assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "dd  ");
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 2 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("j".into()), ModifiersState::empty()));
    assert_eq!(app.current_scrollback_offset(), 1);
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 2 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("j".into()), ModifiersState::empty()));
    assert_eq!(app.current_scrollback_offset(), 0);
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "dd  ");
    assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "ee  ");
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 2 })
    );
}

#[test]
fn window_copy_mode_page_movement_scrolls_across_scrollback() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
    app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee\r\nff")
        .unwrap();

    app.enter_copy_mode();
    assert_eq!(app.current_scrollback_offset(), 0);
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "ee  ");
    assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "ff  ");

    assert!(app.handle_copy_mode_key(&Key::Named(NamedKey::PageUp), ModifiersState::empty()));
    assert_eq!(app.current_scrollback_offset(), 1);
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "dd  ");
    assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "ee  ");
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 2 })
    );

    assert!(app.handle_copy_mode_key(&Key::Named(NamedKey::PageDown), ModifiersState::empty()));
    assert_eq!(app.current_scrollback_offset(), 0);
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "ee  ");
    assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "ff  ");
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 2 })
    );
}

#[test]
fn window_copy_mode_dispatches_wezterm_page_movement_assignment_queries() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
    app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee\r\nff")
        .unwrap();

    app.enter_copy_mode();

    let command =
        super::command_palette_structured_query_command("wezterm.action.CopyMode 'PageUp'")
            .expect("expected CopyMode PageUp assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode PageUp should dispatch");

    assert_eq!(app.current_scrollback_offset(), 1);
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "dd  ");
    assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "ee  ");
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 2 })
    );

    let command =
        super::command_palette_structured_query_command("wezterm.action.CopyMode 'PageDown'")
            .expect("expected CopyMode PageDown assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode PageDown should dispatch");

    assert_eq!(app.current_scrollback_offset(), 0);
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "ee  ");
    assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "ff  ");
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 2 })
    );
}

#[test]
fn window_copy_mode_dispatches_wezterm_scroll_to_bottom_assignment_query() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
    app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee\r\nff")
        .unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Named(NamedKey::PageUp), ModifiersState::empty()));
    assert_eq!(app.current_scrollback_offset(), 1);

    let command =
        super::command_palette_structured_query_command("wezterm.action.CopyMode 'ScrollToBottom'")
            .expect("expected CopyMode ScrollToBottom assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode ScrollToBottom should dispatch");

    assert_eq!(app.current_scrollback_offset(), 0);
    assert!(copy_mode_for_test(&app).is_some());
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "ee  ");
    assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "ff  ");
}

#[test]
fn window_copy_mode_dispatches_wezterm_move_by_page_assignment_queries() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 4));
    app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee\r\nff\r\ngg\r\nhh")
        .unwrap();

    app.enter_copy_mode();
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 3, column: 2 })
    );

    let command = super::command_palette_structured_query_command(
        "wezterm.action.CopyMode { MoveByPage = -0.5 }",
    )
    .expect("expected CopyMode MoveByPage assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode MoveByPage up should dispatch");

    assert_eq!(app.current_scrollback_offset(), 0);
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 2 })
    );

    let command = super::command_palette_structured_query_command(
        "wezterm.action.CopyMode { MoveByPage = 0.5 }",
    )
    .expect("expected CopyMode MoveByPage assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode MoveByPage down should dispatch");

    assert_eq!(app.current_scrollback_offset(), 0);
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 3, column: 2 })
    );
}

#[test]
fn window_copy_mode_g_and_shift_g_move_to_scrollback_extents() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
    app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee\r\nff")
        .unwrap();

    app.enter_copy_mode();
    assert_eq!(app.current_scrollback_offset(), 0);
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "ee  ");
    assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "ff  ");

    assert!(app.handle_copy_mode_key(&Key::Character("g".into()), ModifiersState::empty()));
    assert_eq!(app.current_scrollback_offset(), 4);
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "aa  ");
    assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "bb  ");
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 2 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("G".into()), ModifiersState::SHIFT));
    assert_eq!(app.current_scrollback_offset(), 0);
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "ee  ");
    assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "ff  ");
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 2 })
    );
}

#[test]
fn window_copy_mode_close_scrolls_to_bottom_before_exiting() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
    app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee\r\nff")
        .unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("g".into()), ModifiersState::empty()));
    assert_eq!(app.current_scrollback_offset(), 4);
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "aa  ");

    assert!(app.handle_copy_mode_key(&Key::Character("q".into()), ModifiersState::empty()));

    assert!(copy_mode_for_test(&app).is_none());
    assert_eq!(app.current_scrollback_offset(), 0);
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "ee  ");
    assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "ff  ");
}

#[test]
fn window_copy_mode_carriage_return_moves_to_start_of_next_line() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 3));
    app.handle_pty_output(b"abcd\r\nefgh\r\nijkl").unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("g".into()), ModifiersState::empty()));
    assert!(app.handle_copy_mode_key(&Key::Character("\r".into()), ModifiersState::empty()));

    assert_eq!(
        copy_mode_for_test(&app)
            .map(|copy_mode| (copy_mode.source_cursor.row, copy_mode.source_cursor.column)),
        Some((
            app.runtime.terminal().stable_dimensions().physical_top + 1,
            0
        ))
    );
}

#[test]
fn window_copy_mode_uppercase_no_modifier_uses_wezterm_default_bindings() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 4));
    app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee\r\nff")
        .unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("g".into()), ModifiersState::empty()));
    assert_eq!(app.current_scrollback_offset(), 2);
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "aa  ");
    assert_eq!(snapshot_row_text(&app.snapshot, 3, 4), "dd  ");

    assert!(app.handle_copy_mode_key(&Key::Character("G".into()), ModifiersState::empty()));
    assert_eq!(app.current_scrollback_offset(), 0);
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "cc  ");
    assert_eq!(snapshot_row_text(&app.snapshot, 3, 4), "ff  ");
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 3, column: 2 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("H".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 0 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("M".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 2, column: 0 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("L".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 3, column: 0 })
    );
}

#[test]
fn window_copy_mode_line_content_movement_uses_non_space_cells() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
    app.handle_pty_output(b"  aa  \r\n  bb  \r\n  cc  ")
        .unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("g".into()), ModifiersState::empty()));
    assert_eq!(app.current_scrollback_offset(), 1);
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "  aa    ");
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 6 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("^".into()), ModifiersState::SHIFT));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 2 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("$".into()), ModifiersState::SHIFT));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 3 })
    );
}

#[test]
fn window_copy_mode_end_uses_line_content_end_binding() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
    app.handle_pty_output(b"  aa  \r\n  bb  \r\n  cc  ")
        .unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("g".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 6 })
    );

    assert!(app.handle_copy_mode_key(&Key::Named(NamedKey::End), ModifiersState::empty()));

    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 3 })
    );
}

#[test]
fn window_copy_mode_dispatches_wezterm_copy_mode_assignment_queries() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
    app.handle_pty_output(b"  aa  \r\n  bb  \r\n  cc  ")
        .unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("g".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 6 })
    );

    let command = super::command_palette_structured_query_command(
        "wezterm.action.CopyMode 'MoveToStartOfLine'",
    )
    .expect("expected CopyMode assignment query");
    app.command_palette_apply_command(command)
        .expect("copy mode assignment should dispatch");

    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 0 })
    );
}

#[test]
fn window_copy_mode_dispatches_wezterm_set_selection_mode_assignment_queries() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
    app.handle_pty_output(b"abcd\r\nefgh").unwrap();

    app.enter_copy_mode();
    assert!(matches!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.selection_mode),
        Some(super::WindowCopySelectionMode::None)
    ));

    let command = super::command_palette_structured_query_command(
        "wezterm.action.CopyMode { SetSelectionMode = 'Block' }",
    )
    .expect("expected CopyMode SetSelectionMode query");
    app.command_palette_apply_command(command)
        .expect("copy mode assignment should dispatch");

    assert!(matches!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.selection_mode),
        Some(super::WindowCopySelectionMode::Block)
    ));
    assert!(app.selection.is_some());
}

#[test]
fn window_copy_mode_dispatches_wezterm_word_selection_mode_assignment_query() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
    app.handle_pty_output(b"alpha beta").unwrap();

    app.enter_copy_mode();
    assert!(app.set_copy_mode_cursor(0, 8));
    let command = super::command_palette_structured_query_command(
        "wezterm.action.CopyMode { SetSelectionMode = 'Word' }",
    )
    .expect("expected CopyMode SetSelectionMode Word query");
    app.command_palette_apply_command(command)
        .expect("copy mode assignment should dispatch");

    assert_eq!(app.selected_text().as_deref(), Some("beta"));
    assert!(app.selection.is_some());
}

#[test]
fn window_copy_mode_dispatches_wezterm_semantic_zone_selection_mode_assignment_query() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(12, 4));
    app.handle_pty_output(b"ready\r\n\x1b]133;A\x07> \x1b]133;B\x07cargo test\r\n\x1b]133;C\x07ok")
        .unwrap();

    app.enter_copy_mode();
    assert!(app.set_copy_mode_cursor(1, 4));
    let command = super::command_palette_structured_query_command(
        "wezterm.action.CopyMode { SetSelectionMode = 'SemanticZone' }",
    )
    .expect("expected CopyMode SetSelectionMode SemanticZone query");
    app.command_palette_apply_command(command)
        .expect("copy mode assignment should dispatch");

    assert_eq!(app.selected_text().as_deref(), Some("cargo test"));
    assert!(app.selection.is_some());
}

#[test]
fn window_copy_mode_dispatches_wezterm_lua_table_name_assignment_queries() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
    app.handle_pty_output(b"abcd\r\nefgh").unwrap();

    app.enter_copy_mode();
    let command = super::command_palette_structured_query_command(
        "wezterm.action.CopyMode { SetSelectionMode = 'Cell' }",
    )
    .expect("expected CopyMode SetSelectionMode query");
    app.command_palette_apply_command(command)
        .expect("copy mode assignment should dispatch");
    assert!(matches!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.selection_mode),
        Some(super::WindowCopySelectionMode::Cell)
    ));
    assert!(app.selection.is_some());

    let command = super::command_palette_structured_query_command(
        "wezterm.action.CopyMode { 'ClearSelectionMode' }",
    )
    .expect("expected CopyMode table name assignment query");
    app.command_palette_apply_command(command)
        .expect("copy mode assignment should dispatch");

    assert!(matches!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.selection_mode),
        Some(super::WindowCopySelectionMode::None)
    ));
    assert!(app.selection.is_none());
}

#[test]
fn window_copy_mode_dispatches_wezterm_lua_table_long_bracket_key_query() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
    app.handle_pty_output(b"abcd\r\nefgh").unwrap();

    app.enter_copy_mode();
    let command = super::command_palette_structured_query_command(
        "wezterm.action.CopyMode { [[=[SetSelectionMode]=]] = [[Cell]] }",
    )
    .expect("expected CopyMode SetSelectionMode query");
    app.command_palette_apply_command(command)
        .expect("copy mode assignment should dispatch");
    assert!(matches!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.selection_mode),
        Some(super::WindowCopySelectionMode::Cell)
    ));
    assert!(app.selection.is_some());
}

#[test]
fn window_copy_mode_dispatches_wezterm_lua_jump_table_long_bracket_key_query() {
    let assignment = super::copy_mode_assignment_from_query(
        "wezterm.action.CopyMode { [[=[JumpForward]=]] = { [[=[prev_char]=]] = true } }",
    )
    .expect("expected CopyMode JumpForward query");

    assert_eq!(
        assignment,
        super::WindowCopyModeAssignment::StartJump {
            forward: true,
            prev_char: true,
        }
    );
}

#[test]
fn window_copy_mode_dispatches_wezterm_lua_jump_table_trailing_comma_query() {
    let assignment = super::copy_mode_assignment_from_query(
        "wezterm.action.CopyMode { JumpForward = { prev_char = true, } }",
    )
    .expect("expected CopyMode JumpForward query");

    assert_eq!(
        assignment,
        super::WindowCopyModeAssignment::StartJump {
            forward: true,
            prev_char: true,
        }
    );
}

#[test]
fn window_copy_mode_alt_m_uses_line_content_start_binding() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
    app.handle_pty_output(b"  aa  \r\n  bb  \r\n  cc  ")
        .unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("g".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 6 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("m".into()), ModifiersState::ALT));

    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 2 })
    );
}

#[test]
fn window_copy_mode_word_movement_uses_wezterm_default_bindings() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(24, 1));
    app.handle_pty_output(b"  alpha  beta gamma  ").unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("^".into()), ModifiersState::SHIFT));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 2 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("w".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 9 })
    );

    assert!(app.handle_copy_mode_key(&Key::Named(NamedKey::Tab), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 14 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("e".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 18 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("b".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 14 })
    );

    assert!(app.handle_copy_mode_key(&Key::Named(NamedKey::Tab), ModifiersState::SHIFT));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 9 })
    );

    assert!(app.handle_copy_mode_key(&Key::Named(NamedKey::ArrowRight), ModifiersState::ALT));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 14 })
    );

    assert!(app.handle_copy_mode_key(&Key::Named(NamedKey::ArrowLeft), ModifiersState::ALT));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 9 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("f".into()), ModifiersState::ALT));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 14 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("b".into()), ModifiersState::ALT));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 9 })
    );
}

#[test]
fn window_copy_mode_word_movement_crosses_scrollback_rows() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
    app.handle_pty_output(b"aa bb\r\n  cc dd\r\n  ee").unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("g".into()), ModifiersState::empty()));
    assert_eq!(app.current_scrollback_offset(), 1);
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "aa bb   ");
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 4 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("w".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 2 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("e".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 3 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("b".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 1, column: 2 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("b".into()), ModifiersState::empty()));
    assert_eq!(app.current_scrollback_offset(), 1);
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 3 })
    );
}

#[test]
fn window_copy_mode_jump_forward_repeat_and_reverse_use_wezterm_bindings() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 1));
    app.handle_pty_output(b"abacad").unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("0".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 0 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("f".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 0 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("a".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 2 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character(";".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 4 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character(",".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 2 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("t".into()), ModifiersState::empty()));
    assert!(app.handle_copy_mode_key(&Key::Character("d".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 4 })
    );
}

#[test]
fn window_copy_mode_dispatches_wezterm_jump_repeat_assignment_queries() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 1));
    app.handle_pty_output(b"abacad").unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("0".into()), ModifiersState::empty()));
    assert!(app.handle_copy_mode_key(&Key::Character("f".into()), ModifiersState::empty()));
    assert!(app.handle_copy_mode_key(&Key::Character("a".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 2 })
    );

    let command =
        super::command_palette_structured_query_command("wezterm.action.CopyMode 'JumpAgain'")
            .expect("expected CopyMode JumpAgain assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode JumpAgain should dispatch");

    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 4 })
    );

    let command =
        super::command_palette_structured_query_command("wezterm.action.CopyMode 'JumpReverse'")
            .expect("expected CopyMode JumpReverse assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode JumpReverse should dispatch");

    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 2 })
    );
}

#[test]
fn window_copy_mode_dispatches_wezterm_jump_start_assignment_queries() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 1));
    app.handle_pty_output(b"abacad").unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("0".into()), ModifiersState::empty()));

    let command = super::command_palette_structured_query_command(
        "wezterm.action.CopyMode { JumpForward = { prev_char = false } }",
    )
    .expect("expected CopyMode JumpForward assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode JumpForward should dispatch");
    assert!(app.handle_copy_mode_key(&Key::Character("a".into()), ModifiersState::empty()));

    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 2 })
    );

    let command = super::command_palette_structured_query_command(
        "wezterm.action.CopyMode { JumpBackward = { prev_char = true } }",
    )
    .expect("expected CopyMode JumpBackward assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode JumpBackward should dispatch");
    assert!(app.handle_copy_mode_key(&Key::Character("a".into()), ModifiersState::empty()));

    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 1 })
    );
}

#[test]
fn window_copy_mode_jump_backward_uses_wezterm_bindings() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 1));
    app.handle_pty_output(b"abacad").unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("$".into()), ModifiersState::SHIFT));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 5 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("F".into()), ModifiersState::SHIFT));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 5 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("a".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 4 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("T".into()), ModifiersState::SHIFT));
    assert!(app.handle_copy_mode_key(&Key::Character("b".into()), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 2 })
    );
}

#[test]
fn window_copy_mode_search_keeps_copy_mode_and_steps_matches() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 3));
    app.handle_pty_output(b"foo one\r\nmiddle\r\nfoo two")
        .unwrap();

    app.enter_copy_mode();
    assert!(copy_mode_for_test(&app).is_some());
    assert!(app.move_copy_mode_to_viewport_top());
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    assert!(copy_mode_for_test(&app).is_some());
    assert!(search_for_test(&app).is_some());

    for character in ["f", "o", "o"] {
        assert!(
            app.handle_copy_mode_key(&Key::Character(character.into()), ModifiersState::empty())
        );
    }

    assert_eq!(app.selected_text().as_deref(), Some("foo"));
    assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('f'));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 0 })
    );

    assert!(app.handle_copy_mode_key(&Key::Named(NamedKey::ArrowDown), ModifiersState::empty()));
    assert_eq!(app.selected_text().as_deref(), Some("foo"));
    assert_eq!(snapshot_char(&app.snapshot, 2, 0), Some('f'));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 2, column: 0 })
    );

    assert!(app.handle_copy_mode_key(&Key::Named(NamedKey::ArrowUp), ModifiersState::empty()));
    assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('f'));

    assert!(app.handle_copy_mode_key(&Key::Character("n".into()), ModifiersState::CONTROL));
    assert_eq!(snapshot_char(&app.snapshot, 2, 0), Some('f'));

    assert!(app.handle_copy_mode_key(&Key::Character("p".into()), ModifiersState::CONTROL));
    assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('f'));
}

#[test]
fn window_copy_mode_search_dispatches_wezterm_match_assignment_queries() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 3));
    app.handle_pty_output(b"foo one\r\nmiddle\r\nfoo two")
        .unwrap();

    app.enter_copy_mode();
    assert!(app.move_copy_mode_to_viewport_top());
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    for character in ["f", "o", "o"] {
        assert!(
            app.handle_copy_mode_key(&Key::Character(character.into()), ModifiersState::empty())
        );
    }
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 0 })
    );

    for (query, expected_row) in [
        ("wezterm.action.CopyMode 'NextMatch'", 2),
        ("wezterm.action.CopyMode 'PriorMatch'", 0),
    ] {
        let command = super::command_palette_structured_query_command(query)
            .expect("expected CopyMode search assignment query");
        app.command_palette_apply_command(command)
            .expect("copy mode search assignment should dispatch");
        assert_eq!(
            copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
            Some(SelectionCell {
                row: expected_row,
                column: 0
            })
        );
    }
}

#[test]
fn window_copy_mode_search_dispatches_wezterm_page_match_assignment_queries() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 3));
    app.handle_pty_output(b"foo 0\r\nfoo 1\r\nfoo 2\r\nfoo 3\r\nfoo 4")
        .unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    for character in ["f", "o", "o"] {
        assert!(
            app.handle_copy_mode_key(&Key::Character(character.into()), ModifiersState::empty())
        );
    }
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "foo 0   ");

    let command =
        super::command_palette_structured_query_command("wezterm.action.CopyMode 'NextMatchPage'")
            .expect("expected CopyMode NextMatchPage assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode NextMatchPage should dispatch");
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "foo 2   ");
    assert_eq!(
        copy_mode_for_test(&app)
            .map(|copy_mode| (copy_mode.source_cursor.row, copy_mode.source_cursor.column)),
        Some((app.runtime.terminal().retained_stable_range().start + 3, 0))
    );

    let command =
        super::command_palette_structured_query_command("wezterm.action.CopyMode 'PriorMatchPage'")
            .expect("expected CopyMode PriorMatchPage assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode PriorMatchPage should dispatch");
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "foo 0   ");
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 0 })
    );
}

#[test]
fn window_copy_mode_search_dispatches_wezterm_pattern_assignment_queries() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 3));
    app.handle_pty_output(b"foo\r\nFOO\r\nfao").unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    for character in ["f", "o", "o"] {
        assert!(
            app.handle_copy_mode_key(&Key::Character(character.into()), ModifiersState::empty())
        );
    }
    assert_eq!(
        search_for_test(&app).map(|search| search.match_type),
        Some(WindowSearchMatchType::CaseSensitive)
    );
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "foo     ");

    let command =
        super::command_palette_structured_query_command("wezterm.action.CopyMode 'CycleMatchType'")
            .expect("expected CopyMode CycleMatchType assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode CycleMatchType should dispatch");
    assert_eq!(
        search_for_test(&app).map(|search| search.match_type),
        Some(WindowSearchMatchType::CaseInsensitive)
    );

    let command =
        super::command_palette_structured_query_command("wezterm.action.CopyMode 'ClearPattern'")
            .expect("expected CopyMode ClearPattern assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode ClearPattern should dispatch");
    assert_eq!(
        search_for_test(&app).map(|search| search.query.as_str()),
        Some("")
    );
    assert!(app.selection.is_none());
    assert!(copy_mode_for_test(&app).is_some());
}

#[test]
fn window_copy_mode_search_dispatches_wezterm_edit_pattern_assignment_queries() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(16, 2));
    app.handle_pty_output(b"foo\r\nfoobar").unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    for character in ["f", "o", "o"] {
        assert!(
            app.handle_copy_mode_key(&Key::Character(character.into()), ModifiersState::empty())
        );
    }
    assert_eq!(
        search_for_test(&app).map(|search| search.query.as_str()),
        Some("foo")
    );

    let command =
        super::command_palette_structured_query_command("wezterm.action.CopyMode 'AcceptPattern'")
            .expect("expected CopyMode AcceptPattern assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode AcceptPattern should dispatch");
    assert!(app.handle_copy_mode_key(&Key::Character("x".into()), ModifiersState::empty()));
    assert_eq!(
        search_for_test(&app).map(|search| search.query.as_str()),
        Some("foo")
    );

    let command =
        super::command_palette_structured_query_command("wezterm.action.CopyMode 'EditPattern'")
            .expect("expected CopyMode EditPattern assignment query");
    app.command_palette_apply_command(command)
        .expect("CopyMode EditPattern should dispatch");
    assert!(app.handle_copy_mode_key(&Key::Character("b".into()), ModifiersState::empty()));
    assert_eq!(
        search_for_test(&app).map(|search| search.query.as_str()),
        Some("foob")
    );
}

#[test]
fn window_copy_mode_search_carriage_return_uses_prior_match_binding() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 3));
    app.handle_pty_output(b"foo one\r\nmiddle\r\nfoo two")
        .unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    for character in ["f", "o", "o"] {
        assert!(
            app.handle_copy_mode_key(&Key::Character(character.into()), ModifiersState::empty())
        );
    }
    assert!(app.handle_copy_mode_key(&Key::Named(NamedKey::ArrowDown), ModifiersState::empty()));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 2, column: 0 })
    );

    assert!(app.handle_copy_mode_key(&Key::Character("\r".into()), ModifiersState::empty()));

    assert_eq!(
        search_for_test(&app).map(|search| search.query.as_str()),
        Some("foo")
    );
    assert_eq!(app.selected_text().as_deref(), Some("foo"));
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 0 })
    );
}

#[test]
fn window_copy_mode_search_escape_character_closes_copy_mode() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 3));
    app.handle_pty_output(b"foo one\r\nmiddle\r\nfoo two")
        .unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    assert!(copy_mode_for_test(&app).is_some());
    assert!(search_for_test(&app).is_some());

    assert!(app.handle_copy_mode_key(&Key::Character("\u{1b}".into()), ModifiersState::empty()));

    assert!(copy_mode_for_test(&app).is_none());
    assert!(search_for_test(&app).is_none());
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1]"
    );
}

#[test]
fn window_copy_mode_search_page_navigation_skips_visible_page_matches() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 3));
    app.handle_pty_output(b"foo 0\r\nfoo 1\r\nfoo 2\r\nfoo 3\r\nfoo 4")
        .unwrap();

    app.enter_copy_mode();
    assert!(app.move_copy_mode_to_viewport_top());
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    for character in ["f", "o", "o"] {
        assert!(
            app.handle_copy_mode_key(&Key::Character(character.into()), ModifiersState::empty())
        );
    }

    assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "foo 0   ");
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 0 })
    );

    assert!(app.handle_copy_mode_key(&Key::Named(NamedKey::PageDown), ModifiersState::empty()));
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "foo 2   ");
    assert_eq!(
        copy_mode_for_test(&app)
            .map(|copy_mode| (copy_mode.source_cursor.row, copy_mode.source_cursor.column)),
        Some((app.runtime.terminal().retained_stable_range().start + 3, 0))
    );

    assert!(app.handle_copy_mode_key(&Key::Named(NamedKey::PageUp), ModifiersState::empty()));
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "foo 0   ");
    assert_eq!(
        copy_mode_for_test(&app).map(|copy_mode| copy_mode.cursor),
        Some(SelectionCell { row: 0, column: 0 })
    );
}

#[test]
fn window_copy_mode_search_cycles_match_type() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 3));
    app.handle_pty_output(b"foo\r\nFOO\r\nfao").unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    for character in ["f", "o", "o"] {
        assert!(
            app.handle_copy_mode_key(&Key::Character(character.into()), ModifiersState::empty())
        );
    }

    assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "foo     ");
    assert_eq!(app.selected_text().as_deref(), Some("foo"));
    assert!(app.handle_copy_mode_key(&Key::Named(NamedKey::ArrowDown), ModifiersState::empty()));
    assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "foo     ");

    assert!(app.handle_copy_mode_key(&Key::Character("r".into()), ModifiersState::CONTROL));
    assert!(app.handle_copy_mode_key(&Key::Named(NamedKey::ArrowDown), ModifiersState::empty()));
    assert_eq!(snapshot_row_text(&app.snapshot, 1, 8), "FOO     ");
    assert_eq!(
        copy_mode_for_test(&app)
            .map(|copy_mode| (copy_mode.source_cursor.row, copy_mode.source_cursor.column)),
        Some((app.runtime.terminal().retained_stable_range().start + 1, 0))
    );

    assert!(app.handle_copy_mode_key(&Key::Character("r".into()), ModifiersState::CONTROL));
    assert!(app.handle_copy_mode_key(&Key::Character("u".into()), ModifiersState::CONTROL));
    for character in ["f", ".", "o"] {
        assert!(
            app.handle_copy_mode_key(&Key::Character(character.into()), ModifiersState::empty())
        );
    }

    assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "foo     ");
    assert!(app.handle_copy_mode_key(&Key::Named(NamedKey::ArrowDown), ModifiersState::empty()));
    assert_eq!(snapshot_row_text(&app.snapshot, 2, 8), "fao     ");
    assert_eq!(
        copy_mode_for_test(&app)
            .map(|copy_mode| (copy_mode.source_cursor.row, copy_mode.source_cursor.column)),
        Some((app.runtime.terminal().retained_stable_range().start + 2, 0))
    );
}

#[test]
fn window_copy_mode_transitions_shared_search_and_replaces_quick_select() {
    let mut app = NativeWindowApp::new(None);
    app.enter_search_mode();
    app.update_search_query("example");
    assert!(search_for_test(&app).is_some());
    assert_eq!(
        copy_search_mode_for_test(&app),
        Some(super::WindowCopySearchMode::Search)
    );

    app.enter_copy_mode();
    assert!(search_for_test(&app).is_some());
    assert!(copy_mode_for_test(&app).is_some());
    assert_eq!(
        copy_search_mode_for_test(&app),
        Some(super::WindowCopySearchMode::Copy)
    );

    app.enter_quick_select_mode();
    assert!(copy_mode_for_test(&app).is_none());
    assert!(quick_select_for_test(&app).is_some());
}

#[test]
fn window_app_search_and_copy_transition_in_one_active_overlay() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
    app.handle_pty_output(b"alpha beta").unwrap();

    app.enter_search_mode_with_query(&WindowSearchCommandQuery::Pattern {
        pattern: "alpha".to_owned(),
        match_type: WindowSearchMatchType::CaseSensitive,
    });
    assert_eq!(
        search_for_test(&app).map(|search| search.query.as_str()),
        Some("alpha")
    );

    app.enter_copy_mode();

    assert!(copy_mode_for_test(&app).is_some());
    assert_eq!(
        search_for_test(&app).map(|search| search.query.as_str()),
        Some("alpha"),
        "Search and Copy must transition within one retained controller"
    );
}

#[test]
fn window_app_quick_select_replaces_copy_search_and_exit_does_not_restore_it() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(32, 1));
    app.handle_pty_output(b"https://example.com").unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    app.enter_quick_select_mode();

    assert!(quick_select_for_test(&app).is_some());
    assert!(copy_mode_for_test(&app).is_none());
    assert!(search_for_test(&app).is_none());

    app.exit_quick_select_mode();

    assert!(!overlay_active_for_test(&app));
    assert!(copy_mode_for_test(&app).is_none());
    assert!(search_for_test(&app).is_none());
}

#[test]
fn window_app_title_uses_only_active_overlay_variant() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
    app.handle_pty_output(b"alpha beta").unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    for character in ["a", "l", "p", "h", "a"] {
        assert!(
            app.handle_copy_mode_key(&Key::Character(character.into()), ModifiersState::empty())
        );
    }

    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1] - Search: alpha",
        "the active Search variant must not also render Copy status"
    );
}

#[test]
fn window_app_search_exit_rebuilds_presentation_projection_immediately() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 1));
    app.handle_pty_output(b"hit base").unwrap();

    app.enter_search_mode_with_query(&WindowSearchCommandQuery::Pattern {
        pattern: "hit".to_owned(),
        match_type: WindowSearchMatchType::CaseSensitive,
    });
    assert!(rendered_active_pane_cell(&app, 0, 0).unwrap().inverse);

    assert!(app.handle_search_key(&Key::Named(NamedKey::Escape), ModifiersState::empty()));

    assert!(search_for_test(&app).is_none());
    assert!(app.selection.is_none());
    assert!(
        !rendered_active_pane_cell(&app, 0, 0).unwrap().inverse,
        "Search exit must rebuild the presentation projection without a manual refresh"
    );
}

#[test]
fn window_app_new_search_pattern_recomputes_results_without_resetting_copy_cursor() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(16, 2));
    app.handle_pty_output(b"foo one\r\nfoo two").unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    for character in ["f", "o", "o"] {
        assert!(
            app.handle_copy_mode_key(&Key::Character(character.into()), ModifiersState::empty())
        );
    }
    {
        let copy_mode = retained_copy_mode_mut_for_test(&mut app);
        copy_mode.selection_mode = super::WindowCopySelectionMode::Cell;
        copy_mode.anchor = Some(copy_mode.cursor);
        copy_mode.source_anchor = Some(copy_mode.source_cursor);
    }
    let copy_mode = active_copy_mode_for_test(&app);
    let before_cursor = copy_mode.cursor;
    let before_source_cursor = copy_mode.source_cursor;
    let before_anchor = copy_mode.anchor;
    let before_source_anchor = copy_mode.source_anchor;
    let before_selection_mode = copy_mode.selection_mode;
    let before_pending_jump = copy_mode.pending_jump;
    let before_last_jump = copy_mode.last_jump;
    let before_search_direction = copy_mode.search_direction;
    assert!(active_search_for_test(&app).current.is_some());

    assert!(app.update_search_query_with_type(
        "f.o",
        SearchDirection::Next,
        WindowSearchMatchType::Regex,
    ));

    let search = active_search_for_test(&app);
    assert_eq!(search.query, "f.o");
    assert_eq!(search.match_type, WindowSearchMatchType::Regex);
    assert!(
        search.current.is_some(),
        "the new pattern must be recomputed"
    );
    let copy_mode = active_copy_mode_for_test(&app);
    assert_eq!(copy_mode.cursor, before_cursor);
    assert_eq!(copy_mode.source_cursor, before_source_cursor);
    assert_eq!(copy_mode.anchor, before_anchor);
    assert_eq!(copy_mode.source_anchor, before_source_anchor);
    assert_eq!(copy_mode.selection_mode, before_selection_mode);
    assert_eq!(copy_mode.pending_jump, before_pending_jump);
    assert_eq!(copy_mode.last_jump, before_last_jump);
    assert_eq!(copy_mode.search_direction, before_search_direction);
}

#[test]
fn window_app_copy_search_mode_and_editing_state_change_atomically() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
    app.handle_pty_output(b"alpha beta").unwrap();

    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    assert!(search_for_test(&app).is_some_and(|search| search.editing));

    assert!(app.perform_copy_mode_assignment(super::WindowCopyModeAssignment::AcceptPattern));
    assert!(search_for_test(&app).is_some_and(|search| !search.editing));
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1] - Copy Mode"
    );

    assert!(app.perform_copy_mode_assignment(super::WindowCopyModeAssignment::EditPattern));
    assert!(search_for_test(&app).is_some_and(|search| search.editing));
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1] - Search"
    );

    assert!(app.handle_copy_mode_key(&Key::Named(NamedKey::Escape), ModifiersState::empty()));
    assert!(copy_mode_for_test(&app).is_none());
    assert!(search_for_test(&app).is_none());
    assert!(app.selection.is_none());
}

#[test]
fn window_app_keyboard_routes_by_active_overlay_variant() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(16, 2));
    app.handle_pty_output(b"foo one\r\nfoo two").unwrap();
    app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
    app.handle_keyboard_input_event(
        &Key::Character("f".into()),
        PhysicalKey::Code(WinitKeyCode::KeyF),
        Some("f"),
        ElementState::Pressed,
        KittyKeyEventKind::Press,
    )
    .unwrap();
    app.modifiers = ModifiersState::empty();
    for (character, physical_key) in [
        ("f", WinitKeyCode::KeyF),
        ("o", WinitKeyCode::KeyO),
        ("o", WinitKeyCode::KeyO),
    ] {
        app.handle_keyboard_input_event(
            &Key::Character(character.into()),
            PhysicalKey::Code(physical_key),
            Some(character),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
    }
    assert_eq!(active_search_for_test(&app).query, "foo");
    let initial = active_search_for_test(&app).current;
    app.handle_keyboard_input_event(
        &Key::Named(NamedKey::ArrowDown),
        PhysicalKey::Code(WinitKeyCode::ArrowDown),
        None,
        ElementState::Pressed,
        KittyKeyEventKind::Press,
    )
    .unwrap();
    assert_ne!(active_search_for_test(&app).current, initial);

    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(20, 1));
    app.handle_pty_output(b"copy old search").unwrap();
    app.enter_copy_mode();
    {
        let copy_mode = retained_copy_mode_mut_for_test(&mut app);
        copy_mode.cursor = SelectionCell { row: 0, column: 3 };
        copy_mode.source_cursor.column = 3;
        copy_mode.selection_mode = super::WindowCopySelectionMode::Cell;
        copy_mode.anchor = Some(SelectionCell { row: 0, column: 0 });
        copy_mode.source_anchor = Some(SelectionSourceCell {
            column: 0,
            ..copy_mode.source_cursor
        });
    }
    app.apply_copy_mode_selection();
    assert_eq!(app.selected_text().as_deref(), Some("copy"));
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    assert!(app.update_search_query("search"));
    assert_eq!(
        app.selected_text().as_deref(),
        Some("search"),
        "active Search must hide the retained Copy selection"
    );
}

#[test]
fn window_app_same_search_pattern_preserves_current_match() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(16, 2));
    app.handle_pty_output(b"foo one\r\nfoo two").unwrap();

    assert!(app.update_search_query_with_type(
        "foo",
        SearchDirection::Next,
        WindowSearchMatchType::CaseSensitive,
    ));
    assert!(app.step_search(SearchDirection::Next));
    let stepped = active_search_for_test(&app)
        .current
        .expect("stepped search match");

    assert!(app.update_search_query_with_type(
        "foo",
        SearchDirection::Next,
        WindowSearchMatchType::CaseSensitive,
    ));
    assert_eq!(
        active_search_for_test(&app).current,
        Some(stepped),
        "same query and match type must retain the current match"
    );
}

#[test]
fn window_app_copy_search_new_pattern_without_old_match_preserves_full_copy_state() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(16, 2));
    app.handle_pty_output(b"foo one\r\nfoo two").unwrap();
    app.enter_copy_mode();
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    assert!(!app.update_search_query("missing"));
    assert!(active_search_for_test(&app).current.is_none());

    *retained_copy_mode_mut_for_test(&mut app) =
        pane_overlay_copy_mode(0, 1, super::WindowCopySelectionMode::Cell);
    let before = copy_mode_for_test(&app)
        .map(|copy_mode| {
            (
                copy_mode.cursor,
                copy_mode.source_cursor,
                copy_mode.anchor,
                copy_mode.source_anchor,
                copy_mode.selection_mode,
                copy_mode.pending_jump,
                copy_mode.last_jump,
                copy_mode.search_direction,
            )
        })
        .expect("retained Copy state");

    assert!(app.update_search_query_with_type(
        "foo",
        SearchDirection::Next,
        WindowSearchMatchType::CaseSensitive,
    ));

    let after = copy_mode_for_test(&app)
        .map(|copy_mode| {
            (
                copy_mode.cursor,
                copy_mode.source_cursor,
                copy_mode.anchor,
                copy_mode.source_anchor,
                copy_mode.selection_mode,
                copy_mode.pending_jump,
                copy_mode.last_jump,
                copy_mode.search_direction,
            )
        })
        .expect("retained Copy state after new pattern");
    assert_eq!(after, before);
}

#[test]
fn window_app_copy_search_reconcile_prunes_retained_copy_coordinates() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
    app.runtime.set_scrollback_limit(2);
    app.handle_pty_output(b"old-0\r\nold-1\r\nlive").unwrap();
    app.enter_copy_mode();
    assert!(app.move_copy_mode_to_scrollback_top());
    let stale_source_row = active_copy_mode_for_test(&app).source_cursor.row;
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));

    app.handle_pty_output(b"\r\nnew-1\r\nnew-2\r\nnew-3")
        .unwrap();

    assert!(
        app.runtime.terminal().retained_stable_range().start > stale_source_row,
        "test setup must prune the retained Copy cursor"
    );
    assert!(
        !overlay_active_for_test(&app),
        "Copy-search must retire when its retained Copy coordinates are pruned"
    );
    assert!(
        !app.perform_copy_mode_assignment(super::WindowCopyModeAssignment::AcceptPattern),
        "AcceptPattern must not revive stale Copy coordinates"
    );
}

#[test]
fn window_app_standalone_search_prune_retires_copy_search_before_accept() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(12, 2));
    app.runtime.set_scrollback_limit(2);
    app.handle_pty_output(b"old-0\r\nneedle\r\nlive").unwrap();
    let initial_copy_cursor = app.initial_copy_mode().source_cursor;

    app.enter_search_mode_with_query(&WindowSearchCommandQuery::Pattern {
        pattern: "needle".to_owned(),
        match_type: WindowSearchMatchType::CaseSensitive,
    });
    assert!(active_search_for_test(&app).current.is_some());

    app.handle_pty_output(b"\r\nnew-1\r\nnew-2\r\nnew-3\r\nnew-4")
        .unwrap();

    assert!(
        !app.runtime
            .terminal()
            .retained_stable_range()
            .contains(&initial_copy_cursor.row),
        "test setup must prune the standalone Search controller's Copy cursor"
    );
    assert!(
        !overlay_active_for_test(&app),
        "pruning the hidden Copy cursor must retire the whole CopySearch controller"
    );
    assert!(search_for_test(&app).is_none());
    assert!(copy_mode_for_test(&app).is_none());
    assert!(
        !app.perform_copy_mode_assignment(super::WindowCopyModeAssignment::AcceptPattern),
        "AcceptPattern must not revive pruned standalone Search Copy coordinates"
    );
}

#[test]
fn window_app_clear_scrollback_and_viewport_reconciles_overlay_projection() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(12, 2));
    app.handle_pty_output(b"copy-owner\r\nvisible").unwrap();
    set_ordinary_viewport_range_for_test(
        &mut app,
        SelectionCell { row: 0, column: 0 },
        SelectionCell { row: 0, column: 3 },
    );
    app.enter_copy_mode();
    assert!(app.set_copy_mode_cursor(0, 0));
    assert!(app.set_copy_mode_selection_mode(super::WindowCopySelectionMode::Cell));
    let source_cursor = active_copy_mode_for_test(&app).source_cursor;
    assert!(app.selection.is_some());

    app.clear_scrollback_and_viewport();

    assert!(
        app.runtime
            .terminal()
            .retained_stable_range()
            .contains(&source_cursor.row),
        "test must prove destructive erase retirement does not rely on pruning"
    );
    assert!(!overlay_active_for_test(&app));
    assert!(ordinary_selection_for_test(&app).is_none());
    assert!(app.selection.is_none());
}

#[test]
fn window_app_clear_scrollback_only_reconciles_without_identity_retirement() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(12, 2));
    app.handle_pty_output(b"needle-old\r\nkeep\r\nlive")
        .unwrap();
    app.enter_search_mode();
    assert!(app.update_search_query("needle-old"));
    let stale_match = active_search_for_test(&app).current.expect("search match");

    app.clear_scrollback();

    assert!(!stale_match.is_retained(app.runtime.terminal()));
    let search = active_search_for_test(&app);
    assert_eq!(search.query, "needle-old");
    assert_eq!(search.current, None);
    assert_eq!(
        copy_search_mode_for_test(&app),
        Some(super::WindowCopySearchMode::Search)
    );
    assert!(app.selection.is_none());
}

#[test]
fn window_app_active_prune_reconciles_copy_search_and_quick_overlay() {
    let mut copy = NativeWindowApp::new(None);
    copy.runtime.resize(rterm_core::TerminalSize::new(12, 2));
    copy.runtime.set_scrollback_limit(2);
    copy.handle_pty_output(b"copy-old\r\nneedle\r\nlive")
        .unwrap();
    copy.enter_copy_mode();
    assert!(copy.move_copy_mode_to_scrollback_top());
    assert!(copy.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    assert!(copy.update_search_query("needle"));
    assert!(copy.perform_copy_mode_assignment(super::WindowCopyModeAssignment::AcceptPattern));
    let stale_cursor = active_copy_mode_for_test(&copy).source_cursor;

    copy.handle_pty_output(b"\r\nnew-1\r\nnew-2").unwrap();

    assert!(
        !copy
            .runtime
            .terminal()
            .retained_stable_range()
            .contains(&stale_cursor.row)
    );
    assert!(!overlay_active_for_test(&copy));
    assert!(copy.selection.is_none());

    let mut quick = NativeWindowApp::new(None);
    quick.runtime.resize(rterm_core::TerminalSize::new(32, 2));
    quick.runtime.set_scrollback_limit(2);
    quick
        .handle_pty_output(b"https://old.test\r\nhttps://keep.test\r\nlive")
        .unwrap();
    quick.enter_quick_select_mode();
    let old = active_quick_select_for_test(&quick).matches[0];
    let keep = active_quick_select_for_test(&quick).matches[1];
    let keep_label = active_quick_select_for_test(&quick).labels[1].clone();
    quick
        .active_ui
        .quick_select_mut()
        .expect("quick-select state")
        .current = 1;
    quick.update_transient_selection_projection();

    quick.handle_pty_output(b"\r\nnew-1\r\nnew-2").unwrap();

    assert!(!old.is_retained(quick.runtime.terminal()));
    assert!(keep.is_retained(quick.runtime.terminal()));
    let retained = active_quick_select_for_test(&quick);
    assert_eq!(retained.matches, [keep]);
    assert_eq!(retained.labels, [keep_label]);
    assert_eq!(retained.current, 0);
    assert_eq!(retained.current_match(), Some(keep));
    assert_eq!(
        quick.selection,
        keep.viewport_selection_for_top(
            quick.runtime.terminal().stable_dimensions().domain,
            quick.current_viewport_stable_top(),
            quick.runtime.terminal().grid().size(),
        )
    );

    let mut loss = NativeWindowApp::new(None);
    loss.runtime.resize(rterm_core::TerminalSize::new(32, 2));
    loss.runtime.set_scrollback_limit(2);
    loss.handle_pty_output(b"https://old.test\r\nhttps://keep.test\r\nlive")
        .unwrap();
    loss.enter_quick_select_mode();
    let old = active_quick_select_for_test(&loss).matches[0];
    let keep = active_quick_select_for_test(&loss).matches[1];
    loss.handle_pty_output(b"\r\nnew-1\r\nnew-2").unwrap();
    assert!(!old.is_retained(loss.runtime.terminal()));
    assert!(keep.is_retained(loss.runtime.terminal()));
    assert!(quick_select_for_test(&loss).is_none());
    assert!(loss.selection.is_none());
}

#[test]
fn window_app_active_overlay_without_match_hides_ordinary_projection() {
    let mut search_app = NativeWindowApp::new(None);
    search_app
        .runtime
        .resize(rterm_core::TerminalSize::new(12, 1));
    search_app.handle_pty_output(b"base plain").unwrap();
    set_ordinary_viewport_range_for_test(
        &mut search_app,
        SelectionCell { row: 0, column: 0 },
        SelectionCell { row: 0, column: 3 },
    );
    search_app.enter_search_mode();
    assert!(!search_app.update_search_query("missing"));

    let mut quick_app = NativeWindowApp::new(None);
    quick_app
        .runtime
        .resize(rterm_core::TerminalSize::new(12, 1));
    quick_app.handle_pty_output(b"base plain").unwrap();
    set_ordinary_viewport_range_for_test(
        &mut quick_app,
        SelectionCell { row: 0, column: 0 },
        SelectionCell { row: 0, column: 3 },
    );
    quick_app.disable_default_quick_select_patterns = true;
    quick_app.quick_select_patterns.clear();
    quick_app.enter_quick_select_mode();

    assert!(ordinary_selection_for_test(&search_app).is_some());
    assert!(ordinary_selection_for_test(&quick_app).is_some());
    assert!(search_app.selected_text().is_none());
    assert!(quick_app.selected_text().is_none());
    assert!(
        quick_select_for_test(&quick_app)
            .is_some_and(|quick_select| quick_select.matches.is_empty())
    );
    assert_eq!(
        (
            search_app.selection,
            rendered_active_pane_cell(&search_app, 0, 0)
                .expect("Search snapshot cell")
                .inverse,
            quick_app.selection,
            rendered_active_pane_cell(&quick_app, 0, 0)
                .expect("QuickSelect snapshot cell")
                .inverse,
        ),
        (None, false, None, false),
        "active overlays without a match must hide ordinary projection and highlighting"
    );

    assert!(search_app.handle_search_key(&Key::Named(NamedKey::Escape), ModifiersState::empty()));
    quick_app.exit_quick_select_mode();
    assert!(ordinary_selection_for_test(&search_app).is_some());
    assert!(ordinary_selection_for_test(&quick_app).is_some());
    assert_eq!(search_app.selected_text().as_deref(), Some("base"));
    assert_eq!(quick_app.selected_text().as_deref(), Some("base"));
    assert!(search_app.selection.is_some());
    assert!(quick_app.selection.is_some());
    assert!(
        rendered_active_pane_cell(&search_app, 0, 0)
            .unwrap()
            .inverse
    );
    assert!(rendered_active_pane_cell(&quick_app, 0, 0).unwrap().inverse);
}

#[test]
fn window_app_copy_search_accept_pattern_keeps_current_projection() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(16, 2));
    app.set_config_overrides(native_config_snapshot! {
        copy_mode_active_highlight_bg: Some(NativeColorSpec::Color(Color::Rgb(1, 2, 3))),
        ..NativeConfigSnapshot::default()
    });
    app.handle_pty_output(b"foo one\r\nfoo two").unwrap();
    app.enter_copy_mode();
    assert!(app.move_copy_mode_to_viewport_top());
    assert_eq!(
        active_copy_mode_for_test(&app).selection_mode,
        super::WindowCopySelectionMode::None
    );
    assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
    for character in ["f", "o", "o"] {
        assert!(app.handle_search_key(&Key::Character(character.into()), ModifiersState::empty()));
    }
    let current = active_search_for_test(&app)
        .current
        .expect("Copy-search current match");
    let expected = current
        .viewport_selection(app.runtime.terminal())
        .expect("current match viewport projection")
        .1;

    assert!(app.perform_copy_mode_assignment(super::WindowCopyModeAssignment::AcceptPattern));

    assert_eq!(
        copy_search_mode_for_test(&app),
        Some(super::WindowCopySearchMode::Copy)
    );
    assert!(!active_search_for_test(&app).editing);
    assert_eq!(active_search_for_test(&app).current, Some(current));
    assert_eq!(
        app.selection,
        Some(expected),
        "accepted Search current must remain the active Copy projection when Copy has no selection"
    );
    let active_cell =
        rendered_active_pane_cell(&app, expected.anchor.row, expected.anchor.column).unwrap();
    assert_eq!(active_cell.background, Color::Rgb(1, 2, 3));
    assert_eq!(app.selected_text().as_deref(), Some("foo"));

    assert!(app.set_copy_mode_selection_mode(super::WindowCopySelectionMode::Cell));
    assert_ne!(app.selection, Some(expected));
    assert_eq!(
        app.selected_text().as_deref(),
        Some("f"),
        "a real Copy selection must take precedence over retained Search current"
    );
}

#[test]
fn window_app_standalone_search_accept_pattern_promotes_same_controller() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
    app.handle_pty_output(b"foo bar").unwrap();
    let initial_copy_mode = app.initial_copy_mode();
    let initial_copy_state = (
        initial_copy_mode.cursor,
        initial_copy_mode.source_cursor,
        initial_copy_mode.anchor,
        initial_copy_mode.source_anchor,
        initial_copy_mode.selection_mode,
        initial_copy_mode.pending_jump,
        initial_copy_mode.last_jump,
        initial_copy_mode.search_direction,
    );
    app.enter_search_mode_with_query(&WindowSearchCommandQuery::Pattern {
        pattern: "foo".to_owned(),
        match_type: WindowSearchMatchType::CaseSensitive,
    });
    let current = active_search_for_test(&app).current;
    let selection = app.selection;
    assert!(current.is_some());

    let command =
        super::command_palette_structured_query_command("wezterm.action.CopyMode 'AcceptPattern'")
            .expect("standalone Search AcceptPattern command");
    app.command_palette_apply_command(command)
        .expect("CopyMode AcceptPattern command dispatch");

    assert_eq!(
        copy_search_mode_for_test(&app),
        Some(super::WindowCopySearchMode::Copy)
    );
    assert!(!active_search_for_test(&app).editing);
    assert_eq!(active_search_for_test(&app).query, "foo");
    assert_eq!(active_search_for_test(&app).current, current);
    assert_eq!(app.selection, selection);
    assert_eq!(
        app.effective_window_title(),
        "R-Term [workspace:1 tab:1 pane:1] - Copy Mode"
    );
    assert_eq!(app.selected_text().as_deref(), Some("foo"));
    let retained_copy_mode = app
        .active_ui
        .retained_copy_mode()
        .expect("accepted Search must establish retained Copy ownership");
    assert_eq!(
        (
            retained_copy_mode.cursor,
            retained_copy_mode.source_cursor,
            retained_copy_mode.anchor,
            retained_copy_mode.source_anchor,
            retained_copy_mode.selection_mode,
            retained_copy_mode.pending_jump,
            retained_copy_mode.last_jump,
            retained_copy_mode.search_direction,
        ),
        initial_copy_state,
        "AcceptPattern must preserve the same controller's complete Copy state"
    );
    assert!(
        app.active_ui.copy_mode().is_some(),
        "the retained controller must also become the active Copy owner"
    );
}

#[test]
fn window_selection_extracts_text_across_rows() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
    app.handle_pty_output(b"abcd\r\nwxyz").unwrap();

    let selection = WindowSelection::new(
        SelectionCell { row: 0, column: 1 },
        SelectionCell { row: 1, column: 2 },
    );

    assert_eq!(
        selection.text_from_snapshot(&app.snapshot, app.runtime.terminal().grid().size()),
        "bcd\nwxy"
    );

    let reversed = WindowSelection::new(
        SelectionCell { row: 1, column: 2 },
        SelectionCell { row: 0, column: 1 },
    );
    assert_eq!(
        reversed.text_from_snapshot(&app.snapshot, app.runtime.terminal().grid().size()),
        "bcd\nwxy"
    );
}

#[test]
fn window_app_highlights_active_selection() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 1));
    app.handle_pty_output(b"abcd").unwrap();
    set_ordinary_viewport_range_for_test(
        &mut app,
        SelectionCell { row: 0, column: 1 },
        SelectionCell { row: 0, column: 2 },
    );

    app.refresh_snapshot();

    assert!(!rendered_active_pane_cell(&app, 0, 0).unwrap().inverse);
    assert!(rendered_active_pane_cell(&app, 0, 1).unwrap().inverse);
    assert!(rendered_active_pane_cell(&app, 0, 2).unwrap().inverse);
    assert!(!rendered_active_pane_cell(&app, 0, 3).unwrap().inverse);
}

#[test]
fn window_app_updates_selection_from_left_mouse_drag() {
    let mut app = NativeWindowApp::new(None);
    app.window_focused = true;
    app.runtime.resize(rterm_core::TerminalSize::new(4, 1));
    app.handle_pty_output(b"abcd").unwrap();

    app.handle_cursor_moved(PhysicalPosition::new(
        0.0,
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );

    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 2),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();

    assert_eq!(
        app.selection,
        Some(WindowSelection::new(
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 2 },
        ))
    );
    assert!(rendered_active_pane_cell(&app, 0, 1).unwrap().inverse);
    assert!(rendered_active_pane_cell(&app, 0, 2).unwrap().inverse);

    assert!(
        app.handle_mouse_input(ElementState::Released, MouseButton::Left)
            .unwrap()
    );
    assert!(!app.selecting);
}

#[test]
fn window_app_left_drag_release_copies_selection_to_clipboard_and_primary_by_default() {
    let clipboard_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded_clipboard = Arc::clone(&clipboard_copied);
    let primary_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded_primary = Arc::clone(&primary_copied);
    let mut app = NativeWindowApp::new(None);
    app.window_focused = true;
    app.runtime.resize(rterm_core::TerminalSize::new(4, 1));
    app.handle_pty_output(b"abcd").unwrap();
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded_clipboard.lock().unwrap().push(text.to_owned());
        true
    });
    app.primary_selection_writer = Box::new(move |text: &str| {
        recorded_primary.lock().unwrap().push(text.to_owned());
        true
    });

    app.handle_cursor_moved(PhysicalPosition::new(
        0.0,
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 2),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();

    assert!(
        app.handle_mouse_input(ElementState::Released, MouseButton::Left)
            .unwrap()
    );

    assert_eq!(clipboard_copied.lock().unwrap().as_slice(), ["abc"]);
    assert_eq!(primary_copied.lock().unwrap().as_slice(), ["abc"]);
    assert!(!app.selecting);
    assert_eq!(app.selected_text().as_deref(), Some("abc"));
}

#[test]
fn window_app_shift_left_click_extends_existing_selection_by_default() {
    let mut app = NativeWindowApp::new(None);
    app.window_focused = true;
    app.runtime.resize(rterm_core::TerminalSize::new(8, 1));
    app.handle_pty_output(b"abcdefgh").unwrap();
    set_ordinary_viewport_range_for_test(
        &mut app,
        SelectionCell { row: 0, column: 1 },
        SelectionCell { row: 0, column: 3 },
    );
    app.modifiers = ModifiersState::SHIFT;
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 6),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();

    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );

    assert_eq!(
        app.selection,
        Some(WindowSelection::new(
            SelectionCell { row: 0, column: 1 },
            SelectionCell { row: 0, column: 6 },
        ))
    );
    assert_eq!(app.selected_text().as_deref(), Some("bcdefg"));
    assert!(!app.selecting);
}

#[test]
fn window_app_shift_left_click_release_copies_extended_selection_by_default() {
    let clipboard_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded_clipboard = Arc::clone(&clipboard_copied);
    let primary_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded_primary = Arc::clone(&primary_copied);
    let mut app = NativeWindowApp::new(None);
    app.window_focused = true;
    app.runtime.resize(rterm_core::TerminalSize::new(8, 1));
    app.handle_pty_output(b"abcdefgh").unwrap();
    set_ordinary_viewport_range_for_test(
        &mut app,
        SelectionCell { row: 0, column: 1 },
        SelectionCell { row: 0, column: 3 },
    );
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded_clipboard.lock().unwrap().push(text.to_owned());
        true
    });
    app.primary_selection_writer = Box::new(move |text: &str| {
        recorded_primary.lock().unwrap().push(text.to_owned());
        true
    });
    app.modifiers = ModifiersState::SHIFT;
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 6),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );

    assert!(
        app.handle_mouse_input(ElementState::Released, MouseButton::Left)
            .unwrap()
    );

    assert_eq!(clipboard_copied.lock().unwrap().as_slice(), ["bcdefg"]);
    assert_eq!(primary_copied.lock().unwrap().as_slice(), ["bcdefg"]);
    assert_eq!(app.selected_text().as_deref(), Some("bcdefg"));
    assert!(!app.selecting);
}

#[test]
fn window_app_alt_left_drag_uses_block_selection_by_default() {
    let mut app = NativeWindowApp::new(None);
    app.window_focused = true;
    app.runtime.resize(rterm_core::TerminalSize::new(6, 3));
    app.handle_pty_output(b"abcdef\r\nghijkl\r\nmnopqr")
        .unwrap();
    app.modifiers = ModifiersState::ALT;
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );

    assert!(
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(super::CELL_WIDTH * 3),
            f64::from(tab_bar_pixel_height() + super::CELL_HEIGHT * 2),
        ))
        .unwrap()
    );

    assert_eq!(
        app.selection,
        Some(WindowSelection::rectangular(
            SelectionCell { row: 0, column: 1 },
            SelectionCell { row: 2, column: 3 },
        ))
    );
    assert_eq!(app.selected_text().as_deref(), Some("bcd\nhij\nnop"));
    assert!(app.selecting);
}

#[test]
fn window_app_alt_shift_left_click_extends_block_selection_by_default() {
    let mut app = NativeWindowApp::new(None);
    app.window_focused = true;
    app.runtime.resize(rterm_core::TerminalSize::new(6, 3));
    app.handle_pty_output(b"abcdef\r\nghijkl\r\nmnopqr")
        .unwrap();
    set_ordinary_viewport_range_for_test(
        &mut app,
        SelectionCell { row: 0, column: 1 },
        SelectionCell { row: 0, column: 2 },
    );
    app.modifiers = ModifiersState::ALT | ModifiersState::SHIFT;
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 3),
        f64::from(tab_bar_pixel_height() + super::CELL_HEIGHT * 2),
    ))
    .unwrap();

    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );

    assert_eq!(
        app.selection,
        Some(WindowSelection::rectangular(
            SelectionCell { row: 0, column: 1 },
            SelectionCell { row: 2, column: 3 },
        ))
    );
    assert_eq!(app.selected_text().as_deref(), Some("bcd\nhij\nnop"));
    assert!(!app.selecting);
}

#[test]
fn window_app_alt_shift_left_click_release_copies_block_selection_by_default() {
    let clipboard_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded_clipboard = Arc::clone(&clipboard_copied);
    let primary_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded_primary = Arc::clone(&primary_copied);
    let mut app = NativeWindowApp::new(None);
    app.window_focused = true;
    app.runtime.resize(rterm_core::TerminalSize::new(6, 3));
    app.handle_pty_output(b"abcdef\r\nghijkl\r\nmnopqr")
        .unwrap();
    set_ordinary_viewport_range_for_test(
        &mut app,
        SelectionCell { row: 0, column: 1 },
        SelectionCell { row: 0, column: 2 },
    );
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded_clipboard.lock().unwrap().push(text.to_owned());
        true
    });
    app.primary_selection_writer = Box::new(move |text: &str| {
        recorded_primary.lock().unwrap().push(text.to_owned());
        true
    });
    app.modifiers = ModifiersState::ALT | ModifiersState::SHIFT;
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 3),
        f64::from(tab_bar_pixel_height() + super::CELL_HEIGHT * 2),
    ))
    .unwrap();

    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );

    assert!(
        app.handle_mouse_input(ElementState::Released, MouseButton::Left)
            .unwrap()
    );

    assert_eq!(
        clipboard_copied.lock().unwrap().as_slice(),
        ["bcd\nhij\nnop"]
    );
    assert_eq!(primary_copied.lock().unwrap().as_slice(), ["bcd\nhij\nnop"]);
    assert_eq!(app.selected_text().as_deref(), Some("bcd\nhij\nnop"));
    assert!(!app.selecting);
}

#[test]
fn window_app_double_click_selects_word() {
    let mut app = NativeWindowApp::new(None);
    app.window_focused = true;
    app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
    app.handle_pty_output(b"run alpha-beta").unwrap();

    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 6),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );
    assert!(
        app.handle_mouse_input(ElementState::Released, MouseButton::Left)
            .unwrap()
    );
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );

    assert_eq!(
        app.selection,
        Some(WindowSelection::new(
            SelectionCell { row: 0, column: 4 },
            SelectionCell { row: 0, column: 13 },
        ))
    );
    assert!(rendered_active_pane_cell(&app, 0, 4).unwrap().inverse);
    assert!(rendered_active_pane_cell(&app, 0, 13).unwrap().inverse);
}

#[test]
fn window_app_double_click_release_copies_word_to_clipboard_and_primary_by_default() {
    let clipboard_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded_clipboard = Arc::clone(&clipboard_copied);
    let primary_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded_primary = Arc::clone(&primary_copied);
    let mut app = NativeWindowApp::new(None);
    app.window_focused = true;
    app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
    app.handle_pty_output(b"run alpha-beta").unwrap();
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded_clipboard.lock().unwrap().push(text.to_owned());
        true
    });
    app.primary_selection_writer = Box::new(move |text: &str| {
        recorded_primary.lock().unwrap().push(text.to_owned());
        true
    });

    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 6),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );
    assert!(
        app.handle_mouse_input(ElementState::Released, MouseButton::Left)
            .unwrap()
    );
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );

    assert!(
        app.handle_mouse_input(ElementState::Released, MouseButton::Left)
            .unwrap()
    );

    assert_eq!(clipboard_copied.lock().unwrap().as_slice(), ["alpha-beta"]);
    assert_eq!(primary_copied.lock().unwrap().as_slice(), ["alpha-beta"]);
    assert!(!app.selecting);
    assert_eq!(app.selected_text().as_deref(), Some("alpha-beta"));
}

#[test]
fn window_app_double_click_drag_extends_selection_by_word_by_default() {
    let mut app = NativeWindowApp::new(None);
    app.window_focused = true;
    app.runtime.resize(rterm_core::TerminalSize::new(32, 1));
    app.handle_pty_output(b"run alpha-beta gamma_delta")
        .unwrap();

    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 6),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );
    assert!(
        app.handle_mouse_input(ElementState::Released, MouseButton::Left)
            .unwrap()
    );
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );

    assert!(
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(super::CELL_WIDTH * 19),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap()
    );

    assert_eq!(
        app.selection,
        Some(WindowSelection::new(
            SelectionCell { row: 0, column: 4 },
            SelectionCell { row: 0, column: 25 },
        ))
    );
    assert_eq!(
        app.selected_text().as_deref(),
        Some("alpha-beta gamma_delta")
    );
}

#[test]
fn window_app_double_click_uses_default_selection_word_boundary() {
    let mut app = NativeWindowApp::new(None);
    app.window_focused = true;
    app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
    app.handle_pty_output(b"foo{bar}baz").unwrap();

    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 5),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );
    assert!(
        app.handle_mouse_input(ElementState::Released, MouseButton::Left)
            .unwrap()
    );
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );

    assert_eq!(
        app.selection,
        Some(WindowSelection::new(
            SelectionCell { row: 0, column: 4 },
            SelectionCell { row: 0, column: 6 },
        ))
    );
}

#[test]
fn window_app_double_click_honors_selection_word_boundary_override() {
    let mut app = NativeWindowApp::new(None);
    app.window_focused = true;
    app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
    app.set_config_overrides(native_config_snapshot! {
        selection_word_boundary: Some(" :".to_owned()),
        ..NativeConfigSnapshot::default()
    });
    app.handle_pty_output(b"foo:bar-baz").unwrap();

    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 5),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );
    assert!(
        app.handle_mouse_input(ElementState::Released, MouseButton::Left)
            .unwrap()
    );
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );

    assert_eq!(
        app.selection,
        Some(WindowSelection::new(
            SelectionCell { row: 0, column: 4 },
            SelectionCell { row: 0, column: 10 },
        ))
    );
}

#[test]
fn window_app_triple_click_selects_line() {
    let mut app = NativeWindowApp::new(None);
    app.window_focused = true;
    app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
    app.handle_pty_output(b"run alpha-beta").unwrap();

    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 6),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );
    app.handle_mouse_input(ElementState::Released, MouseButton::Left)
        .unwrap();
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );
    app.handle_mouse_input(ElementState::Released, MouseButton::Left)
        .unwrap();
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );

    assert_eq!(
        app.selection,
        Some(WindowSelection::new(
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 15 },
        ))
    );
    assert_eq!(app.selected_text().as_deref(), Some("run alpha-beta"));
    assert!(rendered_active_pane_cell(&app, 0, 0).unwrap().inverse);
    assert!(
        app.selection
            .unwrap()
            .contains(0, 15, app.runtime.terminal().grid().size())
    );
}

#[test]
fn window_app_triple_click_release_copies_line_to_clipboard_and_primary_by_default() {
    let clipboard_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded_clipboard = Arc::clone(&clipboard_copied);
    let primary_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded_primary = Arc::clone(&primary_copied);
    let mut app = NativeWindowApp::new(None);
    app.window_focused = true;
    app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
    app.handle_pty_output(b"run alpha-beta").unwrap();
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded_clipboard.lock().unwrap().push(text.to_owned());
        true
    });
    app.primary_selection_writer = Box::new(move |text: &str| {
        recorded_primary.lock().unwrap().push(text.to_owned());
        true
    });

    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 6),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );
    app.handle_mouse_input(ElementState::Released, MouseButton::Left)
        .unwrap();
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );
    app.handle_mouse_input(ElementState::Released, MouseButton::Left)
        .unwrap();
    clipboard_copied.lock().unwrap().clear();
    primary_copied.lock().unwrap().clear();
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );

    assert!(
        app.handle_mouse_input(ElementState::Released, MouseButton::Left)
            .unwrap()
    );

    assert_eq!(
        clipboard_copied.lock().unwrap().as_slice(),
        ["run alpha-beta"]
    );
    assert_eq!(
        primary_copied.lock().unwrap().as_slice(),
        ["run alpha-beta"]
    );
    assert!(!app.selecting);
    assert_eq!(app.selected_text().as_deref(), Some("run alpha-beta"));
}

#[test]
fn window_app_triple_click_drag_extends_selection_by_line_by_default() {
    let mut app = NativeWindowApp::new(None);
    app.window_focused = true;
    app.runtime.resize(rterm_core::TerminalSize::new(16, 2));
    app.handle_pty_output(b"alpha beta\r\ngamma delta").unwrap();

    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 3),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );
    app.handle_mouse_input(ElementState::Released, MouseButton::Left)
        .unwrap();
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );
    app.handle_mouse_input(ElementState::Released, MouseButton::Left)
        .unwrap();
    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );

    assert!(
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(super::CELL_WIDTH * 2),
            f64::from(tab_bar_pixel_height() + super::CELL_HEIGHT),
        ))
        .unwrap()
    );

    assert_eq!(
        app.selection,
        Some(WindowSelection::new(
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 1, column: 15 },
        ))
    );
}

#[test]
fn window_app_dispatches_palette_select_text_at_mouse_cursor_cell_command() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
    app.handle_pty_output(b"run alpha-beta").unwrap();
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 6),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();

    app.enter_command_palette_mode();
    app.command_palette_execute(WindowCommand::SelectTextAtMouseCursorCell);

    assert_eq!(
        app.selection,
        Some(WindowSelection::new(
            SelectionCell { row: 0, column: 6 },
            SelectionCell { row: 0, column: 6 },
        ))
    );
    assert_eq!(app.selected_text().as_deref(), Some("p"));
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_dispatches_palette_select_text_at_mouse_cursor_word_command() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
    app.handle_pty_output(b"run alpha-beta").unwrap();
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 6),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();

    app.enter_command_palette_mode();
    app.command_palette_execute(WindowCommand::SelectTextAtMouseCursorWord);

    assert_eq!(
        app.selection,
        Some(WindowSelection::new(
            SelectionCell { row: 0, column: 4 },
            SelectionCell { row: 0, column: 13 },
        ))
    );
    assert_eq!(app.selected_text().as_deref(), Some("alpha-beta"));
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_dispatches_palette_select_text_at_mouse_cursor_line_command() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
    app.handle_pty_output(b"run alpha-beta").unwrap();
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 6),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();

    app.enter_command_palette_mode();
    app.command_palette_execute(WindowCommand::SelectTextAtMouseCursorLine);

    assert_eq!(
        app.selection,
        Some(WindowSelection::new(
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 15 },
        ))
    );
    assert_eq!(app.selected_text().as_deref(), Some("run alpha-beta"));
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_dispatches_palette_select_text_at_mouse_cursor_block_command() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(6, 3));
    app.handle_pty_output(b"abcdef\r\nghijkl\r\nmnopqr")
        .unwrap();
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 3),
        f64::from(tab_bar_pixel_height() + super::CELL_HEIGHT),
    ))
    .unwrap();

    app.enter_command_palette_mode();
    app.command_palette_execute(WindowCommand::SelectTextAtMouseCursorBlock);

    assert_eq!(
        app.selection,
        Some(WindowSelection::rectangular(
            SelectionCell { row: 1, column: 3 },
            SelectionCell { row: 1, column: 3 },
        ))
    );
    assert_eq!(app.selected_text().as_deref(), Some("j"));
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_dispatches_palette_select_text_at_mouse_cursor_semantic_zone_command() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(12, 4));
    app.handle_pty_output(b"ready\r\n\x1b]133;A\x07> \x1b]133;B\x07cargo test\r\n\x1b]133;C\x07ok")
        .unwrap();
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 4),
        f64::from(tab_bar_pixel_height() + super::CELL_HEIGHT),
    ))
    .unwrap();

    app.enter_command_palette_mode();
    app.command_palette_execute(WindowCommand::SelectTextAtMouseCursorSemanticZone);

    assert_eq!(
        app.selection,
        Some(WindowSelection::new(
            SelectionCell { row: 1, column: 2 },
            SelectionCell { row: 1, column: 11 },
        ))
    );
    assert_eq!(app.selected_text().as_deref(), Some("cargo test"));
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_dispatches_native_select_text_at_mouse_cursor_payload() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(12, 4));
    app.handle_pty_output(b"ready\r\n\x1b]133;A\x07> \x1b]133;B\x07cargo test\r\n\x1b]133;C\x07ok")
        .unwrap();
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 4),
        f64::from(tab_bar_pixel_height() + super::CELL_HEIGHT),
    ))
    .unwrap();

    app.enter_command_palette_mode();
    app.command_palette_execute(WindowCommand::SelectTextAtMouseCursor(
        WindowMouseSelectionMode::SemanticZone,
    ));

    assert_eq!(
        app.selection,
        Some(WindowSelection::new(
            SelectionCell { row: 1, column: 2 },
            SelectionCell { row: 1, column: 11 },
        ))
    );
    assert_eq!(app.selected_text().as_deref(), Some("cargo test"));
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_dispatches_palette_extend_selection_to_mouse_cursor_cell_command() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
    app.handle_pty_output(b"run alpha-beta").unwrap();
    set_ordinary_viewport_range_for_test(
        &mut app,
        SelectionCell { row: 0, column: 0 },
        SelectionCell { row: 0, column: 2 },
    );
    app.selecting = true;
    set_app_search_for_test(
        &mut app,
        WindowSearch {
            query: "alpha".to_owned(),
            match_type: WindowSearchMatchType::CaseSensitive,
            current: None,
            editing: true,
        },
    );
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 6),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();

    app.enter_command_palette_mode();
    app.command_palette_execute(WindowCommand::ExtendSelectionToMouseCursorCell);

    assert_eq!(
        app.selection,
        Some(WindowSelection::new(
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 6 },
        ))
    );
    assert_eq!(app.selected_text().as_deref(), Some("run alp"));
    assert!(!app.selecting);
    assert!(search_for_test(&app).is_none());
    assert!(copy_mode_for_test(&app).is_none());
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_dispatches_palette_extend_selection_to_mouse_cursor_word_command() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
    app.handle_pty_output(b"run alpha-beta").unwrap();
    set_ordinary_viewport_range_for_test(
        &mut app,
        SelectionCell { row: 0, column: 0 },
        SelectionCell { row: 0, column: 2 },
    );
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 6),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();

    app.enter_command_palette_mode();
    app.command_palette_execute(WindowCommand::ExtendSelectionToMouseCursorWord);

    assert_eq!(
        app.selection,
        Some(WindowSelection::new(
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 13 },
        ))
    );
    assert_eq!(app.selected_text().as_deref(), Some("run alpha-beta"));
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_dispatches_palette_extend_selection_to_mouse_cursor_line_command() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(16, 2));
    app.handle_pty_output(b"first\r\nsecond").unwrap();
    set_ordinary_viewport_range_for_test(
        &mut app,
        SelectionCell { row: 0, column: 1 },
        SelectionCell { row: 0, column: 3 },
    );
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 2),
        f64::from(tab_bar_pixel_height() + super::CELL_HEIGHT),
    ))
    .unwrap();

    app.enter_command_palette_mode();
    app.command_palette_execute(WindowCommand::ExtendSelectionToMouseCursorLine);

    assert_eq!(
        app.selection,
        Some(WindowSelection::new(
            SelectionCell { row: 0, column: 1 },
            SelectionCell { row: 1, column: 15 },
        ))
    );
    assert_eq!(app.selected_text().as_deref(), Some("irst\nsecond"));
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_dispatches_palette_extend_selection_to_mouse_cursor_block_command() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(6, 3));
    app.handle_pty_output(b"abcdef\r\nghijkl\r\nmnopqr")
        .unwrap();
    set_ordinary_viewport_range_for_test(
        &mut app,
        SelectionCell { row: 0, column: 1 },
        SelectionCell { row: 0, column: 2 },
    );
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 3),
        f64::from(tab_bar_pixel_height() + super::CELL_HEIGHT * 2),
    ))
    .unwrap();

    app.enter_command_palette_mode();
    app.command_palette_execute(WindowCommand::ExtendSelectionToMouseCursorBlock);

    assert_eq!(
        app.selection,
        Some(WindowSelection::rectangular(
            SelectionCell { row: 0, column: 1 },
            SelectionCell { row: 2, column: 3 },
        ))
    );
    assert_eq!(app.selected_text().as_deref(), Some("bcd\nhij\nnop"));
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_dispatches_native_extend_selection_to_mouse_cursor_payload() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(6, 3));
    app.handle_pty_output(b"abcdef\r\nghijkl\r\nmnopqr")
        .unwrap();
    set_ordinary_viewport_range_for_test(
        &mut app,
        SelectionCell { row: 0, column: 1 },
        SelectionCell { row: 0, column: 2 },
    );
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 3),
        f64::from(tab_bar_pixel_height() + super::CELL_HEIGHT * 2),
    ))
    .unwrap();

    app.enter_command_palette_mode();
    app.command_palette_execute(WindowCommand::ExtendSelectionToMouseCursor(
        WindowMouseSelectionMode::Block,
    ));

    assert_eq!(
        app.selection,
        Some(WindowSelection::rectangular(
            SelectionCell { row: 0, column: 1 },
            SelectionCell { row: 2, column: 3 },
        ))
    );
    assert_eq!(app.selected_text().as_deref(), Some("bcd\nhij\nnop"));
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_dispatches_palette_extend_selection_to_mouse_cursor_semantic_zone_query() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(12, 4));
    app.handle_pty_output(b"ready\r\n\x1b]133;A\x07> \x1b]133;B\x07cargo test\r\n\x1b]133;C\x07ok")
        .unwrap();
    set_ordinary_viewport_range_for_test(
        &mut app,
        SelectionCell { row: 1, column: 0 },
        SelectionCell { row: 1, column: 1 },
    );
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(super::CELL_WIDTH * 4),
        f64::from(tab_bar_pixel_height() + super::CELL_HEIGHT),
    ))
    .unwrap();

    app.enter_command_palette_mode();
    app.command_palette_set_query(
        "wezterm.action.ExtendSelectionToMouseCursor('SemanticZone')".to_owned(),
    );

    let expected =
        WindowCommand::ExtendSelectionToMouseCursor(WindowMouseSelectionMode::SemanticZone);
    assert_eq!(
        app.command_palette_filtered_commands(),
        vec![expected.clone()]
    );

    assert!(app.command_palette_execute(expected));

    assert_eq!(
        app.selection,
        Some(WindowSelection::new(
            SelectionCell { row: 1, column: 0 },
            SelectionCell { row: 1, column: 11 },
        ))
    );
    assert_eq!(app.selected_text().as_deref(), Some("> cargo test"));
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_parses_static_wezterm_open_uri_false_return() {
    let opened = Arc::new(Mutex::new(Vec::new()));
    let recorded_open = Arc::clone(&opened);
    let mut app = NativeWindowApp::new(None);
    app.hyperlink_opener = Box::new(move |url: &str| {
        recorded_open.lock().unwrap().push(url.to_owned());
        true
    });
    let overrides = super::native_config_overrides_from_wezterm_lua_config(
        r#"
            local wezterm = require 'wezterm'

            wezterm.on('open-uri', function(window, pane, uri)
              return false
            end)
            "#,
    )
    .expect("expected static WezTerm open-uri false return");
    app.set_config_overrides(overrides);

    assert!(
        app.command_palette_execute(WindowCommand::OpenUri("mailto:ops@example.com".to_owned()))
    );

    assert!(opened.lock().unwrap().is_empty());
}

#[test]
fn window_app_parses_static_wezterm_open_uri_mailto_prefix_return() {
    let opened = Arc::new(Mutex::new(Vec::new()));
    let recorded_open = Arc::clone(&opened);
    let mut app = NativeWindowApp::new(None);
    app.hyperlink_opener = Box::new(move |url: &str| {
        recorded_open.lock().unwrap().push(url.to_owned());
        true
    });
    let overrides = super::native_config_overrides_from_wezterm_lua_config(
        r#"
            local wezterm = require 'wezterm'

            wezterm.on('open-uri', function(window, pane, uri)
              local start, match_end = uri:find 'mailto:'
              if start == 1 then
                return false
              end
            end)
            "#,
    )
    .expect("expected static WezTerm open-uri mailto prefix return");
    app.set_config_overrides(overrides);

    assert!(
        app.command_palette_execute(WindowCommand::OpenUri("mailto:ops@example.com".to_owned()))
    );
    assert!(app.command_palette_execute(WindowCommand::OpenUri("https://example.com".to_owned())));

    assert_eq!(opened.lock().unwrap().as_slice(), ["https://example.com"]);
}

#[test]
fn window_app_parses_multiple_static_wezterm_open_uri_prefix_handlers() {
    let opened = Arc::new(Mutex::new(Vec::new()));
    let recorded_open = Arc::clone(&opened);
    let mut app = NativeWindowApp::new(None);
    app.hyperlink_opener = Box::new(move |url: &str| {
        recorded_open.lock().unwrap().push(url.to_owned());
        true
    });
    let overrides = super::native_config_overrides_from_wezterm_lua_config(
        r#"
            local wezterm = require 'wezterm'

            wezterm.on('open-uri', function(window, pane, uri)
              local start, match_end = uri:find 'mailto:'
              if start == 1 then
                return false
              end
            end)

            wezterm.on('open-uri', function(window, pane, uri)
              local start, match_end = uri:find 'ssh:'
              if start == 1 then
                return false
              end
            end)
            "#,
    )
    .expect("expected multiple static WezTerm open-uri prefix handlers");
    app.set_config_overrides(overrides);

    assert!(
        app.command_palette_execute(WindowCommand::OpenUri("mailto:ops@example.com".to_owned()))
    );
    assert!(app.command_palette_execute(WindowCommand::OpenUri("ssh://example.com".to_owned())));
    assert!(app.command_palette_execute(WindowCommand::OpenUri("https://example.com".to_owned())));

    assert_eq!(opened.lock().unwrap().as_slice(), ["https://example.com"]);
}

#[test]
fn window_app_parses_documented_wezterm_open_uri_mailto_spawn_action() {
    let opened = Arc::new(Mutex::new(Vec::new()));
    let recorded_open = Arc::clone(&opened);
    let mut app = NativeWindowApp::new_with_command(
        None,
        rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
    );
    app.hyperlink_opener = Box::new(move |url: &str| {
        recorded_open.lock().unwrap().push(url.to_owned());
        true
    });
    let overrides = super::native_config_overrides_from_wezterm_lua_config(
        r#"
            local wezterm = require 'wezterm'

            wezterm.on('open-uri', function(window, pane, uri)
              local start, match_end = uri:find 'mailto:'
              if start == 1 then
                local recipient = uri:sub(match_end + 1)
                window:perform_action(
                  wezterm.action.SpawnCommandInNewWindow {
                    args = { 'mutt', recipient },
                  },
                  pane
                )
                return false
              end
            end)
            "#,
    )
    .expect("expected documented static WezTerm open-uri mailto spawn action");
    app.set_config_overrides(overrides);

    assert!(
        app.command_palette_execute(WindowCommand::OpenUri("mailto:ops@example.com".to_owned()))
    );

    assert!(opened.lock().unwrap().is_empty());
    assert_eq!(app.app_shell.pending_windows().len(), 1);
    let pending_window = app
        .app_shell
        .pending_windows()
        .first()
        .expect("spawn window should request a pending window");
    let launch = pending_window.tab().panes()[0].launch();
    assert_eq!(launch.program(), "mutt");
    assert_eq!(launch.args(), ["ops@example.com"]);
}

#[test]
fn window_app_plain_left_click_release_opens_hyperlink_by_default() {
    let open_uris = Arc::new(Mutex::new(Vec::new()));
    let recorded_uri = Arc::clone(&open_uris);
    let opened = Arc::new(Mutex::new(Vec::new()));
    let recorded_open = Arc::clone(&opened);
    let mut app = NativeWindowApp::new(None);
    app.window_focused = true;
    app.open_uri_handler = Box::new(move |event| {
        recorded_uri.lock().unwrap().push(event.clone());
        true
    });
    app.hyperlink_opener = Box::new(move |url: &str| {
        recorded_open.lock().unwrap().push(url.to_owned());
        true
    });
    let active_pane = app.app_shell.active_pane_id();
    app.runtime.resize(rterm_core::TerminalSize::new(8, 1));
    app.handle_pty_output(b"\x1b]8;;https://example.test\x1b\\link\x1b]8;;\x1b\\")
        .unwrap();
    app.handle_cursor_moved(PhysicalPosition::new(
        0.0,
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();

    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );
    assert!(opened.lock().unwrap().is_empty());

    assert!(
        app.handle_mouse_input(ElementState::Released, MouseButton::Left)
            .unwrap()
    );

    assert_eq!(
        open_uris.lock().unwrap().as_slice(),
        [NativeWindowOpenUri {
            window_id: rterm_core::WindowId::new(1),
            pane: active_pane,
            uri: "https://example.test".to_owned(),
        }]
    );
    assert_eq!(opened.lock().unwrap().as_slice(), ["https://example.test"]);
    assert!(app.selection.is_none());
    assert!(!app.selecting);
}

#[test]
fn window_app_shift_left_click_release_opens_hyperlink_by_default() {
    let open_uris = Arc::new(Mutex::new(Vec::new()));
    let recorded_uri = Arc::clone(&open_uris);
    let opened = Arc::new(Mutex::new(Vec::new()));
    let recorded_open = Arc::clone(&opened);
    let mut app = NativeWindowApp::new(None);
    app.window_focused = true;
    app.open_uri_handler = Box::new(move |event| {
        recorded_uri.lock().unwrap().push(event.clone());
        true
    });
    app.hyperlink_opener = Box::new(move |url: &str| {
        recorded_open.lock().unwrap().push(url.to_owned());
        true
    });
    let active_pane = app.app_shell.active_pane_id();
    app.runtime.resize(rterm_core::TerminalSize::new(8, 1));
    app.handle_pty_output(b"\x1b]8;;https://shift.example\x1b\\link\x1b]8;;\x1b\\")
        .unwrap();
    app.modifiers = ModifiersState::SHIFT;
    app.handle_cursor_moved(PhysicalPosition::new(
        0.0,
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();

    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );

    assert!(
        app.handle_mouse_input(ElementState::Released, MouseButton::Left)
            .unwrap()
    );

    assert_eq!(
        open_uris.lock().unwrap().as_slice(),
        [NativeWindowOpenUri {
            window_id: rterm_core::WindowId::new(1),
            pane: active_pane,
            uri: "https://shift.example".to_owned(),
        }]
    );
    assert_eq!(opened.lock().unwrap().as_slice(), ["https://shift.example"]);
    assert!(app.selection.is_none());
    assert!(!app.selecting);
}

#[test]
fn window_app_dispatches_palette_complete_selection_or_open_link_opens_hyperlink_at_mouse_cursor() {
    let open_uris = Arc::new(Mutex::new(Vec::new()));
    let recorded_uri = Arc::clone(&open_uris);
    let opened = Arc::new(Mutex::new(Vec::new()));
    let recorded_open = Arc::clone(&opened);
    let mut app = NativeWindowApp::new(None);
    app.open_uri_handler = Box::new(move |event| {
        recorded_uri.lock().unwrap().push(event.clone());
        true
    });
    app.hyperlink_opener = Box::new(move |url: &str| {
        recorded_open.lock().unwrap().push(url.to_owned());
        true
    });
    let active_pane = app.app_shell.active_pane_id();
    app.runtime.resize(rterm_core::TerminalSize::new(8, 1));
    app.handle_pty_output(b"\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\")
        .unwrap();
    app.handle_cursor_moved(PhysicalPosition::new(
        0.0,
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();

    app.enter_command_palette_mode();
    app.command_palette_set_query("complete selection open link".to_owned());
    let command = app
        .command_palette_filtered_commands()
        .into_iter()
        .find(|command| command.label() == "Complete Selection Or Open Link At Mouse Cursor")
        .expect("expected complete selection/open-link command");
    app.command_palette_execute(command);

    assert_eq!(
        open_uris.lock().unwrap().as_slice(),
        [NativeWindowOpenUri {
            window_id: rterm_core::WindowId::new(1),
            pane: active_pane,
            uri: "https://example.com".to_owned(),
        }]
    );
    assert_eq!(opened.lock().unwrap().as_slice(), ["https://example.com"]);
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_dispatches_palette_complete_selection_or_open_link_completes_active_selection() {
    let clipboard_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded_clipboard = Arc::clone(&clipboard_copied);
    let primary_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded_primary = Arc::clone(&primary_copied);
    let opened = Arc::new(Mutex::new(Vec::new()));
    let recorded_open = Arc::clone(&opened);
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 1));
    app.handle_pty_output(b"abcd").unwrap();
    set_ordinary_viewport_range_for_test(
        &mut app,
        SelectionCell { row: 0, column: 1 },
        SelectionCell { row: 0, column: 2 },
    );
    app.selecting = true;
    app.refresh_snapshot();
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded_clipboard.lock().unwrap().push(text.to_owned());
        true
    });
    app.primary_selection_writer = Box::new(move |text: &str| {
        recorded_primary.lock().unwrap().push(text.to_owned());
        true
    });
    app.hyperlink_opener = Box::new(move |url: &str| {
        recorded_open.lock().unwrap().push(url.to_owned());
        true
    });

    app.enter_command_palette_mode();
    app.command_palette_set_query("complete selection open link".to_owned());
    let command = app
        .command_palette_filtered_commands()
        .into_iter()
        .find(|command| command.label() == "Complete Selection Or Open Link At Mouse Cursor")
        .expect("expected complete selection/open-link command");
    app.command_palette_execute(command);

    assert_eq!(clipboard_copied.lock().unwrap().as_slice(), ["bc"]);
    assert_eq!(primary_copied.lock().unwrap().as_slice(), ["bc"]);
    assert!(opened.lock().unwrap().is_empty());
    assert!(!app.selecting);
    assert_eq!(app.selected_text().as_deref(), Some("bc"));
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_dispatches_palette_complete_selection_command() {
    let clipboard_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded_clipboard = Arc::clone(&clipboard_copied);
    let primary_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded_primary = Arc::clone(&primary_copied);
    let opened = Arc::new(Mutex::new(Vec::new()));
    let recorded_open = Arc::clone(&opened);
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 1));
    app.handle_pty_output(b"abcd").unwrap();
    set_ordinary_viewport_range_for_test(
        &mut app,
        SelectionCell { row: 0, column: 1 },
        SelectionCell { row: 0, column: 2 },
    );
    app.selecting = true;
    app.refresh_snapshot();
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded_clipboard.lock().unwrap().push(text.to_owned());
        true
    });
    app.primary_selection_writer = Box::new(move |text: &str| {
        recorded_primary.lock().unwrap().push(text.to_owned());
        true
    });
    app.hyperlink_opener = Box::new(move |url: &str| {
        recorded_open.lock().unwrap().push(url.to_owned());
        true
    });

    app.enter_command_palette_mode();
    app.command_palette_set_query("complete selection".to_owned());
    let command = app
        .command_palette_filtered_commands()
        .into_iter()
        .find(|command| command.label() == "Complete Selection")
        .expect("expected complete-selection command");
    app.command_palette_execute(command);

    assert_eq!(clipboard_copied.lock().unwrap().as_slice(), ["bc"]);
    assert_eq!(primary_copied.lock().unwrap().as_slice(), ["bc"]);
    assert!(opened.lock().unwrap().is_empty());
    assert!(!app.selecting);
    assert_eq!(app.selected_text().as_deref(), Some("bc"));
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_dispatches_palette_complete_selection_to_primary_selection_query() {
    let primary_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded_primary = Arc::clone(&primary_copied);
    let clipboard_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded_clipboard = Arc::clone(&clipboard_copied);
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 1));
    app.handle_pty_output(b"abcd").unwrap();
    set_ordinary_viewport_range_for_test(
        &mut app,
        SelectionCell { row: 0, column: 1 },
        SelectionCell { row: 0, column: 2 },
    );
    app.selecting = true;
    app.refresh_snapshot();
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded_clipboard.lock().unwrap().push(text.to_owned());
        true
    });
    app.primary_selection_writer = Box::new(move |text: &str| {
        recorded_primary.lock().unwrap().push(text.to_owned());
        true
    });

    app.enter_command_palette_mode();
    app.command_palette_set_query("complete selection to primary selection".to_owned());
    let commands = app.command_palette_filtered_commands();
    assert_eq!(
        commands,
        [WindowCommand::CompleteSelectionTo(
            WindowCopyDestination::PrimarySelection
        )]
    );
    app.command_palette_execute(commands[0].clone());

    assert!(!app.selecting);
    assert!(clipboard_copied.lock().unwrap().is_empty());
    assert_eq!(primary_copied.lock().unwrap().as_slice(), ["bc"]);
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_dispatches_palette_complete_selection_to_quoted_query() {
    let mut app = NativeWindowApp::new(None);

    app.enter_command_palette_mode();
    app.command_palette_set_query("complete selection to \"primary selection\"".to_owned());

    assert_eq!(
        app.command_palette_filtered_commands(),
        [WindowCommand::CompleteSelectionTo(
            WindowCopyDestination::PrimarySelection
        )]
    );
}

#[test]
fn window_app_dispatches_palette_complete_selection_to_equals_query() {
    let mut app = NativeWindowApp::new(None);

    app.enter_command_palette_mode();
    app.command_palette_set_query("complete selection to=primary selection".to_owned());

    assert_eq!(
        app.command_palette_filtered_commands(),
        [WindowCommand::CompleteSelectionTo(
            WindowCopyDestination::PrimarySelection
        )]
    );
}

#[test]
fn window_app_dispatches_palette_complete_selection_to_destination_assignment_query() {
    let mut app = NativeWindowApp::new(None);

    app.enter_command_palette_mode();
    app.command_palette_set_query("completeselectionto destination=primary selection".to_owned());

    assert_eq!(
        app.command_palette_filtered_commands(),
        [WindowCommand::CompleteSelectionTo(
            WindowCopyDestination::PrimarySelection
        )]
    );
}

#[test]
fn window_app_dispatches_palette_complete_selection_wezterm_action_bare_string_query() {
    let mut app = NativeWindowApp::new(None);

    app.enter_command_palette_mode();
    app.command_palette_set_query("wezterm.action.CompleteSelection 'Clipboard'".to_owned());

    assert_eq!(
        app.command_palette_filtered_commands(),
        [WindowCommand::CompleteSelectionTo(
            WindowCopyDestination::Clipboard
        )]
    );
}

#[test]
fn window_app_dispatches_palette_complete_selection_wezterm_action_function_call_query() {
    let mut app = NativeWindowApp::new(None);

    app.enter_command_palette_mode();
    app.command_palette_set_query(
        "wezterm.action.CompleteSelection('PrimarySelection')".to_owned(),
    );

    assert_eq!(
        app.command_palette_filtered_commands(),
        [WindowCommand::CompleteSelectionTo(
            WindowCopyDestination::PrimarySelection
        )]
    );
}

#[test]
fn window_app_dispatches_palette_complete_selection_action_name_queries() {
    for (query, expected) in [
        (
            "completeselectionto primary selection",
            WindowCommand::CompleteSelectionTo(WindowCopyDestination::PrimarySelection),
        ),
        (
            "completeselectionoropenlinkatmousecursorto clipboard and primary selection",
            WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(
                WindowCopyDestination::ClipboardAndPrimarySelection,
            ),
        ),
    ] {
        let mut app = NativeWindowApp::new(None);
        app.enter_command_palette_mode();
        app.command_palette_set_query(query.to_owned());

        assert_eq!(app.command_palette_filtered_commands(), vec![expected]);
    }
}

#[test]
fn window_app_dispatches_palette_complete_selection_or_open_link_to_primary_selection_query() {
    let primary_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded_primary = Arc::clone(&primary_copied);
    let clipboard_copied = Arc::new(Mutex::new(Vec::new()));
    let recorded_clipboard = Arc::clone(&clipboard_copied);
    let opened = Arc::new(Mutex::new(Vec::new()));
    let recorded_open = Arc::clone(&opened);
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rterm_core::TerminalSize::new(4, 1));
    app.handle_pty_output(b"abcd").unwrap();
    set_ordinary_viewport_range_for_test(
        &mut app,
        SelectionCell { row: 0, column: 1 },
        SelectionCell { row: 0, column: 2 },
    );
    app.selecting = true;
    app.refresh_snapshot();
    app.clipboard_writer = Box::new(move |text: &str| {
        recorded_clipboard.lock().unwrap().push(text.to_owned());
        true
    });
    app.primary_selection_writer = Box::new(move |text: &str| {
        recorded_primary.lock().unwrap().push(text.to_owned());
        true
    });
    app.hyperlink_opener = Box::new(move |url: &str| {
        recorded_open.lock().unwrap().push(url.to_owned());
        true
    });

    app.enter_command_palette_mode();
    app.command_palette_set_query("complete selection open link to primary selection".to_owned());
    let commands = app.command_palette_filtered_commands();
    assert_eq!(
        commands,
        [WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(
            WindowCopyDestination::PrimarySelection
        )]
    );
    app.command_palette_execute(commands[0].clone());

    assert!(!app.selecting);
    assert!(clipboard_copied.lock().unwrap().is_empty());
    assert_eq!(primary_copied.lock().unwrap().as_slice(), ["bc"]);
    assert!(opened.lock().unwrap().is_empty());
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_dispatches_palette_complete_selection_or_open_link_to_quoted_query() {
    let mut app = NativeWindowApp::new(None);

    app.enter_command_palette_mode();
    app.command_palette_set_query(
        "complete selection open link to \"primary selection\"".to_owned(),
    );

    assert_eq!(
        app.command_palette_filtered_commands(),
        [WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(
            WindowCopyDestination::PrimarySelection
        )]
    );
}

#[test]
fn window_app_dispatches_palette_complete_selection_or_open_link_wezterm_action_bare_string_query()
{
    let mut app = NativeWindowApp::new(None);

    app.enter_command_palette_mode();
    app.command_palette_set_query(
        "wezterm.action.CompleteSelectionOrOpenLinkAtMouseCursor 'Clipboard'".to_owned(),
    );

    assert_eq!(
        app.command_palette_filtered_commands(),
        [WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(
            WindowCopyDestination::Clipboard
        )]
    );
}

#[test]
fn window_app_dispatches_palette_complete_selection_or_open_link_wezterm_action_function_call_query()
 {
    let mut app = NativeWindowApp::new(None);

    app.enter_command_palette_mode();
    app.command_palette_set_query(
        "wezterm.action.CompleteSelectionOrOpenLinkAtMouseCursor('PrimarySelection')".to_owned(),
    );

    assert_eq!(
        app.command_palette_filtered_commands(),
        [WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(
            WindowCopyDestination::PrimarySelection
        )]
    );
}

#[test]
fn window_app_dispatches_palette_complete_selection_or_open_link_to_equals_query() {
    let mut app = NativeWindowApp::new(None);

    app.enter_command_palette_mode();
    app.command_palette_set_query("complete selection open link to=primary selection".to_owned());

    assert_eq!(
        app.command_palette_filtered_commands(),
        [WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(
            WindowCopyDestination::PrimarySelection
        )]
    );
}

#[test]
fn window_app_dispatches_palette_complete_selection_or_open_link_to_destination_assignment_query() {
    let mut app = NativeWindowApp::new(None);

    app.enter_command_palette_mode();
    app.command_palette_set_query(
        "completeselectionoropenlinkatmousecursorto destination=clipboard and primary selection"
            .to_owned(),
    );

    assert_eq!(
        app.command_palette_filtered_commands(),
        [WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(
            WindowCopyDestination::ClipboardAndPrimarySelection
        )]
    );
}

#[test]
fn window_app_dispatches_palette_open_link_at_mouse_cursor_command() {
    let open_uris = Arc::new(Mutex::new(Vec::new()));
    let recorded_uri = Arc::clone(&open_uris);
    let opened = Arc::new(Mutex::new(Vec::new()));
    let recorded_open = Arc::clone(&opened);
    let mut app = NativeWindowApp::new(None);
    app.open_uri_handler = Box::new(move |event| {
        recorded_uri.lock().unwrap().push(event.clone());
        true
    });
    app.hyperlink_opener = Box::new(move |url: &str| {
        recorded_open.lock().unwrap().push(url.to_owned());
        true
    });
    let active_pane = app.app_shell.active_pane_id();
    app.runtime.resize(rterm_core::TerminalSize::new(8, 1));
    app.handle_pty_output(b"\x1b]8;;ssh://example.com\x1b\\link\x1b]8;;\x1b\\")
        .unwrap();
    app.handle_cursor_moved(PhysicalPosition::new(
        0.0,
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();

    app.enter_command_palette_mode();
    app.command_palette_set_query("open link mouse".to_owned());
    let command = app
        .command_palette_filtered_commands()
        .into_iter()
        .find(|command| command.label() == "Open Link At Mouse Cursor")
        .expect("expected open-link command");
    app.command_palette_execute(command);

    assert_eq!(
        open_uris.lock().unwrap().as_slice(),
        [NativeWindowOpenUri {
            window_id: rterm_core::WindowId::new(1),
            pane: active_pane,
            uri: "ssh://example.com".to_owned(),
        }]
    );
    assert_eq!(opened.lock().unwrap().as_slice(), ["ssh://example.com"]);
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_dispatches_palette_open_uri_wezterm_action_function_call_query() {
    let open_uris = Arc::new(Mutex::new(Vec::new()));
    let recorded_uri = Arc::clone(&open_uris);
    let opened = Arc::new(Mutex::new(Vec::new()));
    let recorded_open = Arc::clone(&opened);
    let mut app = NativeWindowApp::new(None);
    app.open_uri_handler = Box::new(move |event| {
        recorded_uri.lock().unwrap().push(event.clone());
        true
    });
    app.hyperlink_opener = Box::new(move |url: &str| {
        recorded_open.lock().unwrap().push(url.to_owned());
        true
    });
    let active_pane = app.app_shell.active_pane_id();

    app.enter_command_palette_mode();
    app.command_palette_set_query("wezterm.action.OpenUri('https://example.com/docs')".to_owned());

    let expected = WindowCommand::OpenUri("https://example.com/docs".to_owned());
    assert_eq!(
        app.command_palette_filtered_commands(),
        vec![expected.clone()]
    );
    assert!(app.command_palette_execute(expected));

    assert_eq!(
        open_uris.lock().unwrap().as_slice(),
        [NativeWindowOpenUri {
            window_id: rterm_core::WindowId::new(1),
            pane: active_pane,
            uri: "https://example.com/docs".to_owned(),
        }]
    );
    assert_eq!(
        opened.lock().unwrap().as_slice(),
        ["https://example.com/docs"]
    );
    assert!(app.command_palette.is_none());
}

#[test]
fn window_app_dispatches_palette_open_uri_wezterm_action_table_wrapper_query() {
    let open_uris = Arc::new(Mutex::new(Vec::new()));
    let recorded_uri = Arc::clone(&open_uris);
    let opened = Arc::new(Mutex::new(Vec::new()));
    let recorded_open = Arc::clone(&opened);
    let mut app = NativeWindowApp::new(None);
    app.open_uri_handler = Box::new(move |event| {
        recorded_uri.lock().unwrap().push(event.clone());
        true
    });
    app.hyperlink_opener = Box::new(move |url: &str| {
        recorded_open.lock().unwrap().push(url.to_owned());
        true
    });
    let active_pane = app.app_shell.active_pane_id();

    app.enter_command_palette_mode();
    app.command_palette_set_query(
        "wezterm.action { OpenUri = 'https://example.com/table' }".to_owned(),
    );

    let expected = WindowCommand::OpenUri("https://example.com/table".to_owned());
    assert_eq!(
        app.command_palette_filtered_commands(),
        vec![expected.clone()]
    );
    assert!(app.command_palette_execute(expected));

    assert_eq!(
        open_uris.lock().unwrap().as_slice(),
        [NativeWindowOpenUri {
            window_id: rterm_core::WindowId::new(1),
            pane: active_pane,
            uri: "https://example.com/table".to_owned(),
        }]
    );
    assert_eq!(
        opened.lock().unwrap().as_slice(),
        ["https://example.com/table"]
    );
    assert!(app.command_palette.is_none());
}
