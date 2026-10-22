use super::*;

fn snapshot_contains_text(snapshot: &TerminalRenderSnapshot, text: &str) -> bool {
    snapshot
        .cells()
        .iter()
        .map(|cell| cell.text.as_ref())
        .collect::<String>()
        .contains(text)
}

#[test]
fn app_consumers_use_borrowed_feed_contract() {
    let bench = include_str!("bench.rs");
    let local = include_str!("local.rs").replace("\r\n", "\n");
    let local_runtime_source = include_str!("local_terminal_runtime.rs");
    let window = include_str!("window_parts/part09.rs");
    let shim = include_str!("terminal_runtime.rs");

    assert!(shim.contains("RuntimeBuffers"));
    assert!(bench.contains("inner.feed_into(bytes"));
    assert!(!bench.contains("feed_pty_output_with_display(bytes)"));
    assert!(window.contains("inner.feed_into(bytes"));
    assert!(!window.contains("feed_pty_output_with_display(bytes)"));

    let copy_pty_output = local
        .split("fn copy_pty_output")
        .nth(1)
        .and_then(|tail| tail.split("fn copy_pty_input").next())
        .expect("local PTY copy loop source");
    let local_runtime = local_runtime_source
        .split("impl LocalTerminalRuntime")
        .nth(1)
        .and_then(|tail| tail.split("fn apply_local_runtime_delta").next())
        .expect("local borrowed runtime adapter source");
    let local_host = local_runtime_source
        .split("fn apply_local_runtime_delta")
        .nth(1)
        .and_then(|tail| tail.split("pub(super) struct SessionLogWriter").next())
        .expect("local borrowed host effect adapter source");
    assert!(local.contains("mod local_terminal_runtime"));
    assert!(copy_pty_output.contains("LocalTerminalRuntime::new"));
    assert!(copy_pty_output.contains("terminal_runtime.write_with_clipboard"));
    assert!(copy_pty_output.contains("terminal_runtime.finish"));
    assert!(local_runtime.contains("self.runtime.feed_into"));
    assert!(local_runtime.contains("self.runtime.finish_into"));
    assert!(local_runtime.contains("std::mem::take(&mut self.buffers)"));
    assert!(local_host.contains("RuntimeEffectRef::ConsoleWrite"));
    assert!(local_host.contains("RuntimeEffectRef::TransportWrite"));
    assert!(local_host.contains("delta.mode_changes()"));
    let (production_local, legacy_tail) = local
        .split_once("#[cfg(test)]\nmod legacy_terminal_output")
        .expect("legacy local transcript reference is test-only");
    let legacy_reference = legacy_tail
        .split("#[cfg(test)]\nuse legacy_terminal_output")
        .next()
        .expect("legacy local transcript reference source");
    assert!(legacy_reference.contains("struct LegacyTerminalOutputFilter"));
    assert!(!production_local.contains("LegacyTerminalOutputFilter"));
    for source in [copy_pty_output, local_runtime, local_host] {
        assert!(!source.contains("LegacyTerminalOutputFilter"));
        assert!(!source.contains("feed_pty_output_with_display"));
        assert!(!source.contains("TerminalRuntimeOutput"));
        assert!(!source.contains("response_bytes("));
    }

    let active = window
        .split("fn apply_active_pane_delta")
        .nth(1)
        .and_then(|tail| tail.split("fn finish_active_pane_output").next())
        .expect("active host adapter source");
    let phase_positions = [
        "delta.diagnostics()",
        "delta.responses()",
        "delta.clipboard_writes()",
        "delta.clipboard_reads()",
        "delta.notifications()",
        "delta.bell_count()",
    ]
    .map(|needle| active.find(needle).unwrap_or(usize::MAX));
    assert!(
        phase_positions.windows(2).all(|pair| pair[0] < pair[1]),
        "active host effects must retain legacy phase order: {phase_positions:?}"
    );
    let active_finish = window
        .split("fn finish_active_pane_output")
        .nth(1)
        .and_then(|tail| tail.split("fn retire_active_terminal_identity_state").next())
        .expect("active EOF adapter source");
    let inactive_finish = window
        .split("fn finish_inactive_pane_output")
        .nth(1)
        .and_then(|tail| tail.split("fn record_unknown_escape_sequence_warning").next())
        .expect("inactive EOF adapter source");
    for source in [active_finish, inactive_finish] {
        assert!(source.contains("inner.finish_into"));
        assert!(source.contains("apply_"));
        assert!(source.contains("buffers = buffers"));
    }
}

#[test]
fn window_app_records_iterm_user_var_on_active_pane_metadata() {
    let mut app = NativeWindowApp::new(None);

    app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_PROG=YmFy\x07")
        .unwrap();

    assert_eq!(
        app.app_shell
            .active_pane()
            .user_vars()
            .get("WEZTERM_PROG")
            .map(String::as_str),
        Some("bar")
    );
}

#[test]
fn window_app_records_iterm_user_var_on_inactive_pane_metadata() {
    let mut app = NativeWindowApp::new(None);
    app.dispatch_app_action(AppAction::NewTab { launch: None })
        .unwrap();

    app.handle_pane_pty_output(
        rterm_core::PaneId::new(1),
        b"\x1b]1337;SetUserVar=WEZTERM_PROG=YmFy\x07",
    )
    .unwrap();

    assert_eq!(
        app.app_shell.active_workspace().tabs()[0].panes()[0]
            .user_vars()
            .get("WEZTERM_PROG")
            .map(String::as_str),
        Some("bar")
    );
}

#[test]
fn window_app_records_iterm_badge_format_on_active_pane_metadata() {
    let mut app = NativeWindowApp::new(None);

    app.handle_pty_output(b"\x1b]1337;SetBadgeFormat=aGVsbG8=\x07")
        .unwrap();

    assert_eq!(app.app_shell.active_pane().badge_format(), Some("hello"));
}

#[test]
fn window_app_records_iterm_badge_format_on_inactive_pane_metadata() {
    let mut app = NativeWindowApp::new(None);
    app.dispatch_app_action(AppAction::NewTab { launch: None })
        .unwrap();

    app.handle_pane_pty_output(
        rterm_core::PaneId::new(1),
        b"\x1b]1337;SetBadgeFormat=aGVsbG8=\x07",
    )
    .unwrap();

    assert_eq!(
        app.app_shell.active_workspace().tabs()[0].panes()[0].badge_format(),
        Some("hello")
    );
}

#[test]
fn activating_pane_publishes_title_consumed_while_inactive() {
    let mut app = NativeWindowApp::new(None);
    app.handle_pty_output(b"\x1b]2;alpha\x07").unwrap();
    app.dispatch_app_action(AppAction::NewTab { launch: None })
        .unwrap();
    let pane_b = app.app_shell.active_pane_id();
    app.dispatch_app_action(AppAction::ActivateTab {
        tab: rterm_core::TabId::new(1),
    })
    .unwrap();

    app.handle_pane_pty_output(pane_b, b"\x1b]2;bravo\x07")
        .unwrap();
    assert_eq!(app.window_title, "alpha");
    assert_eq!(
        app.pane_runtimes
            .get(&pane_b)
            .unwrap()
            .runtime
            .terminal()
            .title(),
        Some("bravo")
    );

    app.dispatch_app_action(AppAction::ActivateTab {
        tab: rterm_core::TabId::new(2),
    })
    .unwrap();
    app.handle_pty_output(b"plain").unwrap();

    assert_eq!(app.window_title, "bravo");

    app.dispatch_app_action(AppAction::ActivateTab {
        tab: rterm_core::TabId::new(1),
    })
    .unwrap();

    assert_eq!(app.window_title, "alpha");
}

#[test]
fn same_active_tab_title_update_does_not_replace_explicit_window_title() {
    let mut app = NativeWindowApp::new(None);
    app.handle_pty_output(b"\x1b]2;PaneShell\x07").unwrap();
    app.window_title = "Project Window".to_owned();

    app.dispatch_app_action(AppAction::SetTabTitle {
        tab: rterm_core::TabId::new(1),
        title: "explicit".to_owned(),
    })
    .unwrap();

    assert_eq!(app.window_title, "Project Window");
    assert_eq!(app.runtime.terminal().title(), Some("PaneShell"));
    assert_eq!(
        app.app_shell.active_workspace().tabs()[0].title(),
        Some("explicit")
    );
}

#[test]
fn active_pane_exit_finishes_synchronized_terminal_damage_once() {
    let mut app = NativeWindowApp::new(None);
    let pane = app.app_shell.active_pane_id();
    app.handle_pty_output(b"\x1b[?2026habc").unwrap();
    assert!(!snapshot_contains_text(&app.snapshot, "abc"));

    app.finish_pane_runtime_after_exit(pane, 0);

    assert!(snapshot_contains_text(&app.snapshot, "abc"));
    let cells = app.snapshot.cells().to_vec();
    app.finish_pane_runtime_after_exit(pane, 0);
    assert_eq!(app.snapshot.cells(), cells);
}

#[test]
fn inactive_pane_exit_finishes_synchronized_terminal_damage_once() {
    let mut app = NativeWindowApp::new(None);
    app.dispatch_app_action(AppAction::NewTab { launch: None })
        .unwrap();
    let pane = app.app_shell.active_pane_id();
    app.dispatch_app_action(AppAction::ActivateTab {
        tab: rterm_core::TabId::new(1),
    })
    .unwrap();
    app.handle_pane_pty_output(pane, b"\x1b[?2026hxyz")
        .unwrap();
    assert!(!snapshot_contains_text(
        &app.pane_runtimes.get(&pane).unwrap().snapshot,
        "xyz"
    ));

    app.finish_pane_runtime_after_exit(pane, 0);

    assert!(snapshot_contains_text(
        &app.pane_runtimes.get(&pane).unwrap().snapshot,
        "xyz"
    ));
    assert_eq!(app.pane_has_unseen_output(pane), Some(true));
    let cells = app.pane_runtimes.get(&pane).unwrap().snapshot.cells().to_vec();
    app.finish_pane_runtime_after_exit(pane, 0);
    assert_eq!(app.pane_runtimes.get(&pane).unwrap().snapshot.cells(), cells);
    assert_eq!(app.pane_has_unseen_output(pane), Some(true));
}

#[test]
fn inactive_read_error_atomically_removes_runtime_and_shell_pane() {
    let mut app = NativeWindowApp::new(None);
    app.dispatch_app_action(AppAction::NewTab { launch: None })
        .unwrap();
    let pane = app.app_shell.active_pane_id();
    app.dispatch_app_action(AppAction::ActivateTab {
        tab: rterm_core::TabId::new(1),
    })
    .unwrap();
    app.handle_pane_pty_output(pane, b"\x1b[?2026hread-error")
        .unwrap();

    assert!(!app.handle_pane_runtime_read_error(pane, "broken reader"));

    assert!(!app.pane_runtimes.contains_key(&pane));
    assert!(!app
        .app_shell
        .active_workspace()
        .tabs()
        .iter()
        .flat_map(rterm_core::app_shell::Tab::panes)
        .any(|shell_pane| shell_pane.id() == pane));
}

#[test]
fn inactive_write_error_atomically_removes_runtime_and_shell_pane() {
    let mut app = NativeWindowApp::new(None);
    app.dispatch_app_action(AppAction::NewTab { launch: None })
        .unwrap();
    let pane = app.app_shell.active_pane_id();
    app.dispatch_app_action(AppAction::ActivateTab {
        tab: rterm_core::TabId::new(1),
    })
    .unwrap();
    app.handle_pane_pty_output(pane, b"\x1b[?2026hwrite-error")
        .unwrap();

    assert!(!app.handle_pane_runtime_write_error(pane, "broken writer"));

    assert!(!app.pane_runtimes.contains_key(&pane));
    assert!(!app
        .app_shell
        .active_workspace()
        .tabs()
        .iter()
        .flat_map(rterm_core::app_shell::Tab::panes)
        .any(|shell_pane| shell_pane.id() == pane));
}

#[test]
fn manager_exited_event_publishes_final_damage_before_hold_and_is_idempotent() {
    let mut app = NativeWindowApp::new(None);
    app.exit_behavior = NativeExitBehavior::Hold;
    app.exit_behavior_messaging = NativeExitBehaviorMessaging::None;
    app.handle_pty_output(b"\x1b[?2026hmanager-tail")
        .unwrap();
    let window_id = app.app_window_id;
    let pane_id = app.app_shell.active_pane_id();
    let generation = app.active_runtime_generation;
    let mut manager = NativeWindowManager::new_for_test(app);
    let exit = || WindowUserEvent::Exited {
        window_id,
        pane_id,
        runtime_generation: generation,
    };

    assert_eq!(manager.dispatch_user_event_to_owner(exit()), Some(false));
    let first = manager.startup_app.as_ref().expect("held window");
    assert!(snapshot_contains_text(&first.snapshot, "manager-tail"));
    let cells = first.snapshot.cells().to_vec();

    assert_eq!(manager.dispatch_user_event_to_owner(exit()), Some(false));
    assert_eq!(
        manager
            .startup_app
            .as_ref()
            .expect("held window")
            .snapshot
            .cells(),
        cells
    );
}

#[test]
fn active_plain_chunks_probe_process_cwd_once_per_refresh_window() {
    let mut app = NativeWindowApp::new(None);
    app.session_process_id = Some(std::process::id());
    reset_process_cwd_probe_count();

    for bytes in [b"one".as_slice(), b"two", b"three"] {
        app.handle_pty_output(bytes).unwrap();
    }

    assert_eq!(process_cwd_probe_count(), 1);
}

#[test]
fn inactive_plain_chunks_probe_process_cwd_once_per_refresh_window() {
    let mut app = NativeWindowApp::new(None);
    app.dispatch_app_action(AppAction::NewTab { launch: None })
        .unwrap();
    let pane = rterm_core::PaneId::new(1);
    app.pane_runtimes.get_mut(&pane).unwrap().session_process_id = Some(std::process::id());
    reset_process_cwd_probe_count();

    for bytes in [b"one".as_slice(), b"two", b"three"] {
        app.handle_pane_pty_output(pane, bytes).unwrap();
    }

    assert_eq!(process_cwd_probe_count(), 1);
}

#[test]
fn osc7_cwd_prevents_fallback_process_probes() {
    let mut app = NativeWindowApp::new(None);
    app.session_process_id = Some(std::process::id());
    reset_process_cwd_probe_count();

    app.handle_pty_output(b"before").unwrap();
    assert_eq!(process_cwd_probe_count(), 1);
    app.handle_pty_output(b"\x1b]7;file://host/from-osc7\x07")
        .unwrap();
    app.handle_pty_output(b"after").unwrap();

    assert_eq!(process_cwd_probe_count(), 1);
    assert_eq!(
        app.app_shell.active_pane().launch().cwd(),
        Some("file://host/from-osc7")
    );
}

#[test]
fn window_app_ctrl_click_opens_hyperlink_cell() {
    let opened = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&opened);
    let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
    app.hyperlink_opener = Box::new(move |url: &str| {
        recorded.lock().unwrap().push(url.to_owned());
        true
    });
    app.runtime.resize(rterm_core::TerminalSize::new(8, 1));
    app.handle_pty_output(b"\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\")
        .unwrap();

    app.modifiers = ModifiersState::CONTROL;
    app.handle_cursor_moved(PhysicalPosition::new(
        0.0,
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();

    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );
    assert_eq!(opened.lock().unwrap().as_slice(), ["https://example.com"]);
    assert!(app.selection.is_none());
    assert!(!app.selecting);
}

#[test]
fn window_app_ctrl_click_opens_default_hyperlink_rule_url() {
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
    app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
    app.handle_pty_output(b"visit https://example.com/path")
        .unwrap();

    app.modifiers = ModifiersState::CONTROL;
    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(CELL_WIDTH * 8),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();

    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );

    assert_eq!(
        open_uris.lock().unwrap().as_slice(),
        [NativeWindowOpenUri {
            window_id: rterm_core::WindowId::new(1),
            pane: active_pane,
            uri: "https://example.com/path".to_owned(),
        }]
    );
    assert_eq!(
        opened.lock().unwrap().as_slice(),
        ["https://example.com/path"]
    );
    assert!(app.selection.is_none());
    assert!(!app.selecting);
}

#[test]
fn window_app_hyperlink_rules_override_defaults_and_format_captures() {
    let mut app = NativeWindowApp::new(None);
    app.set_config_overrides(native_config_snapshot! {
        hyperlink_rules: Some(vec![NativeHyperlinkRule {
            regex: r"\bT(\d+)\b".to_owned(),
            format: "https://tickets.example/$1".to_owned(),
            highlight: 1,
        }]),
        ..NativeConfigSnapshot::default()
    });
    app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
    app.handle_pty_output(b"https://example.test T123").unwrap();

    app.modifiers = ModifiersState::CONTROL;
    app.handle_cursor_moved(PhysicalPosition::new(
        0.0,
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();
    assert_eq!(app.hyperlink_at_mouse_position(), None);

    app.handle_cursor_moved(PhysicalPosition::new(
        f64::from(CELL_WIDTH * 22),
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();
    assert_eq!(
        app.hyperlink_at_mouse_position().as_deref(),
        Some("https://tickets.example/123")
    );

    let snapshot = app.hyperlink_rules_snapshot(app.snapshot.clone());
    let linked = snapshot
        .cells()
        .iter()
        .filter_map(|cell| cell.hyperlink.as_ref())
        .collect::<Vec<_>>();
    assert_eq!(linked.len(), 3);
    assert!(
        linked
            .iter()
            .all(|hyperlink| hyperlink.as_ptr() == linked[0].as_ptr())
    );
}

#[test]
fn window_app_hyperlink_rule_cache_reuses_and_invalidates_on_config_change() {
    let mut app = NativeWindowApp::new(None);
    app.set_config_overrides(native_config_snapshot! {
        hyperlink_rules: Some(vec![NativeHyperlinkRule {
            regex: r"\bT(\d+)\b".to_owned(),
            format: "https://tickets.example/$1".to_owned(),
            highlight: 1,
        }]),
        ..NativeConfigSnapshot::default()
    });
    app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
    app.handle_pty_output(b"T123 U456").unwrap();

    let first = app.hyperlink_rules_snapshot(app.snapshot.clone());
    let first_cache = app.compiled_hyperlink_rules();
    let first_cache_address = std::ptr::from_ref(&*first_cache) as usize;
    assert_eq!(first_cache.rules.len(), 1);
    drop(first_cache);
    let second_cache = app.compiled_hyperlink_rules();
    assert_eq!(
        std::ptr::from_ref(&*second_cache) as usize,
        first_cache_address
    );
    drop(second_cache);
    let first_links = first
        .cells()
        .iter()
        .filter_map(|cell| cell.hyperlink.as_deref())
        .collect::<Vec<_>>();
    assert_eq!(first_links.len(), 3);
    assert!(first_links.iter().all(|link| *link == "https://tickets.example/123"));

    app.set_config_overrides(native_config_snapshot! {
        hyperlink_rules: Some(vec![NativeHyperlinkRule {
            regex: r"\bU(\d+)\b".to_owned(),
            format: "https://users.example/$1".to_owned(),
            highlight: 1,
        }]),
        ..NativeConfigSnapshot::default()
    });
    assert!(app.hyperlink_rule_cache.borrow().is_none());

    let changed = app.hyperlink_rules_snapshot(app.snapshot.clone());
    let changed_links = changed
        .cells()
        .iter()
        .filter_map(|cell| cell.hyperlink.as_deref())
        .collect::<Vec<_>>();
    assert_eq!(changed_links.len(), 3);
    assert!(changed_links.iter().all(|link| *link == "https://users.example/456"));
}

#[test]
fn window_app_hyperlink_rule_cache_preserves_invalid_pattern_skip() {
    let mut app = NativeWindowApp::new(None);
    app.set_config_overrides(native_config_snapshot! {
        hyperlink_rules: Some(vec![NativeHyperlinkRule {
            regex: "[".to_owned(),
            format: "https://invalid.example/$0".to_owned(),
            highlight: 0,
        }]),
        ..NativeConfigSnapshot::default()
    });
    app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
    app.handle_pty_output(b"T123").unwrap();

    let snapshot = app.hyperlink_rules_snapshot(app.snapshot.clone());
    let cache = app.compiled_hyperlink_rules();
    assert_eq!(cache.rules.len(), 1);
    assert!(cache.rules[0].regex.is_none());
    assert!(
        snapshot
            .cells()
            .iter()
            .all(|cell| cell.hyperlink.is_none())
    );
}

#[test]
fn window_app_open_uri_hook_can_prevent_default_hyperlink_open() {
    let open_uris = Arc::new(Mutex::new(Vec::new()));
    let recorded_uri = Arc::clone(&open_uris);
    let opened = Arc::new(Mutex::new(Vec::new()));
    let recorded_open = Arc::clone(&opened);
    let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
    app.open_uri_handler = Box::new(move |event| {
        recorded_uri.lock().unwrap().push(event.clone());
        false
    });
    app.hyperlink_opener = Box::new(move |url: &str| {
        recorded_open.lock().unwrap().push(url.to_owned());
        true
    });
    let active_pane = app.app_shell.active_pane_id();
    app.runtime.resize(rterm_core::TerminalSize::new(8, 1));
    app.handle_pty_output(b"\x1b]8;;mailto:ops@example.com\x1b\\mail\x1b]8;;\x1b\\")
        .unwrap();

    app.modifiers = ModifiersState::CONTROL;
    app.handle_cursor_moved(PhysicalPosition::new(
        0.0,
        f64::from(tab_bar_pixel_height()),
    ))
    .unwrap();

    assert!(
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap()
    );
    assert_eq!(
        open_uris.lock().unwrap().as_slice(),
        [NativeWindowOpenUri {
            window_id: rterm_core::WindowId::new(1),
            pane: active_pane,
            uri: "mailto:ops@example.com".to_owned(),
        }]
    );
    assert!(opened.lock().unwrap().is_empty());
    assert!(app.selection.is_none());
    assert!(!app.selecting);
}
