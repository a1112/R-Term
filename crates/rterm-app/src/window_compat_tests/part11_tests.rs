    #[test]
    fn window_search_finds_matches_in_scrollback() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(6, 2));
        app.handle_pty_output(b"alpha\r\nbeta\r\ngamma").unwrap();

        assert!(app.update_search_query("alpha"));

        assert_eq!(app.selected_text().as_deref(), Some("alpha"));
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('a'));
        assert!(rendered_active_pane_cell(&app, 0, 0).unwrap().inverse);
        assert!(app.current_scrollback_offset() > 0);
    }

    #[test]
    fn window_search_maps_complete_grapheme_to_its_terminal_span() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(6, 1));
        app.handle_pty_output("A👍🏽B".as_bytes()).unwrap();

        assert!(app.update_search_query("👍🏽"));

        assert_eq!(app.selected_text().as_deref(), Some("👍🏽"));
        assert!(rendered_active_pane_cell(&app, 0, 1).unwrap().inverse);
        assert!(rendered_active_pane_cell(&app, 0, 2).unwrap().inverse);
    }

    #[test]
    fn window_search_prefills_current_selection_as_single_line() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 3));
        app.handle_pty_output(b"alpha\r\nbeta\r\nalpha beta")
            .unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 1, column: 3 },
        );

        assert_eq!(app.selected_text().as_deref(), Some("alpha\nbeta"));

        app.enter_search_mode();

        assert_eq!(
            search_for_test(&app).map(|search| search.query.as_str()),
            Some("alpha beta")
        );
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 2, column: 0 },
                SelectionCell { row: 2, column: 9 },
            ))
        );
        assert!(rendered_active_pane_cell(&app, 2, 0).unwrap().inverse);
    }

    #[test]
    fn window_app_default_search_shortcut_starts_empty_case_sensitive_search() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 3));
        app.handle_pty_output(b"alpha\r\nbeta\r\nalpha beta")
            .unwrap();
        app.selection = Some(WindowSelection::new(
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 1, column: 3 },
        ));
        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;

        app.handle_keyboard_input_event(
            &Key::Character("f".into()),
            PhysicalKey::Code(WinitKeyCode::KeyF),
            Some("f"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        let search = active_search_for_test(&app);
        assert_eq!(search.query, "");
        assert_eq!(search.match_type, WindowSearchMatchType::CaseSensitive);
        assert!(app.selection.is_none());
    }

    #[test]
    fn window_search_paste_shortcut_edits_query_and_renders_visible_result() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(40, 2));
        app.handle_pty_output("独立 terminal".as_bytes()).unwrap();
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.clipboard_reader = Box::new(|| Some("独立".to_owned()));
        app.enter_search_mode();

        assert!(app.handle_search_key(&Key::Character("v".into()), ModifiersState::SUPER));
        assert!(written.lock().unwrap().is_empty(), "Cmd+V leaked into the PTY");
        assert_eq!(
            search_for_test(&app).map(|search| search.query.as_str()),
            Some("独立")
        );
        assert!(app.selection.is_some());

        let snapshot = app.render_snapshot();
        let row = app
            .terminal_frame_row_offset()
            .saturating_add(app.runtime.terminal().grid().size().rows);
        let row_text = snapshot_row_text(&snapshot, row, 40);
        assert!(row_text.contains('独'), "visible search query: {row_text:?}");
        assert!(row_text.contains('立'), "visible search query: {row_text:?}");
        assert!(row_text.contains("1 / 1"), "visible search result: {row_text:?}");
    }

    #[test]
    fn window_search_visible_status_reports_no_match() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(40, 2));
        app.handle_pty_output(b"terminal").unwrap();
        app.enter_search_mode();

        assert!(app.update_search_query("terminal"));
        assert!(app.selection.is_some());
        assert!(!app.update_search_query("missing"));
        assert!(app.selection.is_none(), "stale search selection survived no match");

        let snapshot = app.render_snapshot();
        let row = app
            .terminal_frame_row_offset()
            .saturating_add(app.runtime.terminal().grid().size().rows);
        let row_text = snapshot_row_text(&snapshot, row, 40);
        assert!(row_text.contains("missing"), "visible search query: {row_text:?}");
        assert!(row_text.contains("No matches"), "visible search result: {row_text:?}");
    }

    #[test]
    fn window_search_overlay_drops_partial_wide_query_at_frame_edge() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(14, 2));
        app.enter_search_mode();
        assert!(!app.update_search_query("123456界"));

        let cells = app.search_overlay_cells();
        assert!(
            cells
                .iter()
                .all(|cell| cell.text.as_ref() != "界" || cell.columns == 2),
            "wide query grapheme was clipped to one column: {cells:?}"
        );
        assert!(
            cells.iter().all(|cell| !cell.continuation),
            "clipped wide query left a continuation cell: {cells:?}"
        );
    }

    #[test]
    fn window_search_finds_match_across_scrollback_and_grid_rows() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(6, 2));
        app.handle_pty_output(b"alpha\r\nbeta\r\ngamma").unwrap();

        assert!(app.update_search_query("habeta"));

        assert_eq!(app.selected_text().as_deref(), Some("ha\nbeta"));
        assert_eq!(snapshot_char(&app.snapshot, 0, 3), Some('h'));
        assert_eq!(snapshot_char(&app.snapshot, 1, 0), Some('b'));
        assert!(rendered_active_pane_cell(&app, 0, 3).unwrap().inverse);
        assert!(rendered_active_pane_cell(&app, 1, 3).unwrap().inverse);
        assert_eq!(app.current_scrollback_offset(), 1);
    }

    #[test]
    fn window_search_supports_regex_prefix_across_terminal_rows() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(6, 2));
        app.handle_pty_output(b"alpha\r\nbeta\r\ngamma").unwrap();

        assert!(app.update_search_query("regex:h.*beta"));

        assert_eq!(app.selected_text().as_deref(), Some("ha\nbeta"));
        assert_eq!(snapshot_char(&app.snapshot, 0, 3), Some('h'));
        assert_eq!(snapshot_char(&app.snapshot, 1, 3), Some('a'));
        assert!(rendered_active_pane_cell(&app, 0, 3).unwrap().inverse);
        assert!(rendered_active_pane_cell(&app, 1, 3).unwrap().inverse);
        assert_eq!(app.current_scrollback_offset(), 1);
    }

    #[test]
    fn window_search_accepts_mixed_case_regex_prefix() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(6, 2));
        app.handle_pty_output(b"alpha\r\nbeta\r\ngamma").unwrap();

        assert!(app.update_search_query("Regex:h.*beta"));

        assert_eq!(app.selected_text().as_deref(), Some("ha\nbeta"));
        assert_eq!(snapshot_char(&app.snapshot, 0, 3), Some('h'));
        assert_eq!(snapshot_char(&app.snapshot, 1, 3), Some('a'));
        assert!(rendered_active_pane_cell(&app, 0, 3).unwrap().inverse);
        assert!(rendered_active_pane_cell(&app, 1, 3).unwrap().inverse);
        assert_eq!(app.current_scrollback_offset(), 1);
    }

    #[test]
    fn window_search_ignores_zero_width_regex_matches() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(5, 1));
        app.handle_pty_output(b"ab cd").unwrap();

        assert!(!app.update_search_query("regex:\\b"));

        assert!(app.selection.is_none());
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Search: regex:\\b (no match)"
        );
    }

    #[test]
    fn window_search_supports_literal_prefix_for_regex_like_text() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"regex:h.*beta").unwrap();

        assert!(app.update_search_query("literal:regex:h.*beta"));

        assert_eq!(app.selected_text().as_deref(), Some("regex:h.*beta"));
        assert!(rendered_active_pane_cell(&app, 0, 0).unwrap().inverse);
        assert!(rendered_active_pane_cell(&app, 0, 12).unwrap().inverse);
    }

    #[test]
    fn window_search_literal_prefix_stays_literal_in_regex_match_type() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"foo\r\nf.o").unwrap();

        assert!(app.update_search_query_with_type(
            "literal:f.o",
            SearchDirection::Next,
            WindowSearchMatchType::Regex
        ));

        assert_eq!(app.selected_text().as_deref(), Some("f.o"));
        assert!(rendered_active_pane_cell(&app, 1, 0).unwrap().inverse);
    }

    #[test]
    fn window_search_steps_between_matches() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 3));
        app.handle_pty_output(b"foo one\r\nmiddle\r\nfoo two")
            .unwrap();

        assert!(app.update_search_query("foo"));
        assert_eq!(app.selected_text().as_deref(), Some("foo"));
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('f'));

        assert!(app.step_search(SearchDirection::Next));

        assert_eq!(app.selected_text().as_deref(), Some("foo"));
        assert_eq!(snapshot_char(&app.snapshot, 2, 0), Some('f'));

        assert!(app.step_search(SearchDirection::Previous));

        assert_eq!(app.selected_text().as_deref(), Some("foo"));
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('f'));
    }

    #[test]
    fn window_search_uses_wezterm_search_mode_navigation_bindings() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 3));
        app.handle_pty_output(b"foo one\r\nmiddle\r\nfoo two")
            .unwrap();

        assert!(app.update_search_query("foo"));
        assert!(!rendered_active_pane_cell(&app, 0, 0).unwrap().inverse);
        assert!(rendered_active_pane_cell(&app, 2, 0).unwrap().inverse);

        assert!(app.handle_search_key(&Key::Named(NamedKey::ArrowDown), ModifiersState::empty()));
        assert!(rendered_active_pane_cell(&app, 0, 0).unwrap().inverse);
        assert!(!rendered_active_pane_cell(&app, 2, 0).unwrap().inverse);

        assert!(app.handle_search_key(&Key::Named(NamedKey::ArrowUp), ModifiersState::empty()));
        assert!(!rendered_active_pane_cell(&app, 0, 0).unwrap().inverse);
        assert!(rendered_active_pane_cell(&app, 2, 0).unwrap().inverse);

        assert!(app.handle_search_key(&Key::Character("n".into()), ModifiersState::CONTROL));
        assert!(rendered_active_pane_cell(&app, 0, 0).unwrap().inverse);
        assert!(!rendered_active_pane_cell(&app, 2, 0).unwrap().inverse);

        assert!(app.handle_search_key(&Key::Character("p".into()), ModifiersState::CONTROL));
        assert!(!rendered_active_pane_cell(&app, 0, 0).unwrap().inverse);
        assert!(rendered_active_pane_cell(&app, 2, 0).unwrap().inverse);
    }

    #[test]
    fn window_search_uses_wezterm_search_mode_page_bindings() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 3));
        app.handle_pty_output(b"foo 0\r\nfoo 1\r\nfoo 2\r\nfoo 3\r\nfoo 4")
            .unwrap();

        assert!(app.update_search_query("foo"));
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "foo 2   ");
        assert!(rendered_active_pane_cell(&app, 2, 0).unwrap().inverse);

        assert!(app.handle_search_key(&Key::Named(NamedKey::PageDown), ModifiersState::empty()));
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "foo 0   ");
        assert!(rendered_active_pane_cell(&app, 0, 0).unwrap().inverse);

        assert!(app.handle_search_key(&Key::Named(NamedKey::PageUp), ModifiersState::empty()));
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "foo 0   ");
        assert!(rendered_active_pane_cell(&app, 0, 0).unwrap().inverse);
    }

    #[test]
    fn window_search_uses_wezterm_search_mode_pattern_bindings() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"foo\r\nFOO").unwrap();

        app.enter_search_mode();
        assert!(app.handle_search_key(&Key::Character("f".into()), ModifiersState::empty()));
        assert!(app.handle_search_key(&Key::Character("o".into()), ModifiersState::empty()));
        assert!(app.handle_search_key(&Key::Character("o".into()), ModifiersState::empty()));
        assert_eq!(
            search_for_test(&app).map(|search| search.query.as_str()),
            Some("foo")
        );
        assert!(rendered_active_pane_cell(&app, 0, 0).unwrap().inverse);
        assert!(!rendered_active_pane_cell(&app, 1, 0).unwrap().inverse);

        assert!(app.handle_search_key(&Key::Character("r".into()), ModifiersState::CONTROL));
        assert!(app.handle_search_key(&Key::Named(NamedKey::ArrowDown), ModifiersState::empty()));
        assert!(!rendered_active_pane_cell(&app, 0, 0).unwrap().inverse);
        assert!(rendered_active_pane_cell(&app, 1, 0).unwrap().inverse);

        assert!(app.handle_search_key(&Key::Character("u".into()), ModifiersState::CONTROL));
        assert_eq!(
            search_for_test(&app).map(|search| search.query.as_str()),
            Some("")
        );
        assert!(app.selection.is_none());

        assert!(app.handle_search_key(&Key::Character("\u{1b}".into()), ModifiersState::empty()));
        assert!(search_for_test(&app).is_none());
    }

    #[test]
    fn recognizes_window_search_shortcuts() {
        assert!(window_search_shortcut(
            &Key::Character("f".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));
        assert!(window_search_shortcut(
            &Key::Character("F".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));
        assert!(!window_search_shortcut(
            &Key::Character("f".into()),
            ModifiersState::CONTROL
        ));
        assert!(!window_search_shortcut(
            &Key::Character("F".into()),
            ModifiersState::CONTROL
        ));
        assert!(window_search_shortcut(
            &Key::Character("f".into()),
            ModifiersState::SUPER
        ));
        assert!(!window_search_shortcut(
            &Key::Character("f".into()),
            ModifiersState::empty()
        ));
    }

    #[test]
    fn recognizes_window_clear_scrollback_shortcut() {
        assert!(window_clear_scrollback_shortcut(
            &Key::Character("k".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));
        assert!(window_clear_scrollback_shortcut(
            &Key::Character("K".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));
        assert!(window_clear_scrollback_shortcut(
            &Key::Character("k".into()),
            ModifiersState::SUPER
        ));
        assert!(!window_clear_scrollback_shortcut(
            &Key::Character("k".into()),
            ModifiersState::CONTROL
        ));
    }

    #[test]
    fn recognizes_window_quick_select_shortcut() {
        assert!(window_quick_select_shortcut(
            &Key::Named(NamedKey::Space),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));
        assert!(!window_quick_select_shortcut(
            &Key::Named(NamedKey::Space),
            ModifiersState::SHIFT
        ));
    }

    #[test]
    fn recognizes_new_tab_shortcut() {
        let app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        assert!(
            app.app_shell_action_for_key(
                &Key::Character("T".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT
            )
            .is_some()
        );
    }

    #[test]
    fn window_app_default_key_assignments_honor_physical_key_map_preference() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            key_map_preference: Some(NativeKeyMapPreference::Physical),
            ..NativeConfigSnapshot::default()
        });
        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;

        app.handle_keyboard_input_event(
            &Key::Character("t".into()),
            PhysicalKey::Code(WinitKeyCode::KeyX),
            Some("t"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));

        app.handle_keyboard_input_event(
            &Key::Character("x".into()),
            PhysicalKey::Code(WinitKeyCode::KeyT),
            Some("x"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
    }

    #[test]
    fn window_app_disable_default_assignment_suppresses_app_shell_shortcut() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            key_assignments: Some(vec![NativeUserKeyAssignment {
                keys: "CTRL+SHIFT+T".to_owned(),
                command: WindowCommand::DisableDefaultAssignment,
            }]),
            ..NativeConfigSnapshot::default()
        });

        assert!(
            app.app_shell_action_for_key(
                &Key::Character("T".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT,
            )
            .is_none()
        );
    }

    #[test]
    fn window_app_disable_default_key_bindings_suppresses_app_shell_shortcuts() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            disable_default_key_bindings: Some(true),
            ..NativeConfigSnapshot::default()
        });

        assert!(
            app.app_shell_action_for_key(
                &Key::Character("T".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT,
            )
            .is_none()
        );
    }

    #[test]
    fn window_app_disable_default_bindings_suppress_close_tab_shortcut_confirmation() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("python.exe"));
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            key_assignments: Some(vec![NativeUserKeyAssignment {
                keys: "CTRL+SHIFT+W".to_owned(),
                command: WindowCommand::DisableDefaultAssignment,
            }]),
            ..NativeConfigSnapshot::default()
        });
        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;

        app.handle_keyboard_input_event(
            &Key::Character("w".into()),
            PhysicalKey::Code(WinitKeyCode::KeyW),
            Some("w"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert!(app.close_confirmation.is_none());

        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("python.exe"));
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            disable_default_key_bindings: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.modifiers = ModifiersState::SUPER;

        app.handle_keyboard_input_event(
            &Key::Character("w".into()),
            PhysicalKey::Code(WinitKeyCode::KeyW),
            Some("w"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert!(app.close_confirmation.is_none());
    }

    #[test]
    fn recognizes_super_window_and_tab_shortcuts() {
        let app = NativeWindowApp::new(None);

        let action =
            app.app_shell_action_for_key(&Key::Character("n".into()), ModifiersState::SUPER);
        assert!(matches!(
            action,
            Some(AppAction::SpawnWindow { launch: None })
        ));

        let action =
            app.app_shell_action_for_key(&Key::Character("t".into()), ModifiersState::SUPER);
        assert!(matches!(action, Some(AppAction::NewTab { launch: None })));

        let action =
            app.app_shell_action_for_key(&Key::Character("w".into()), ModifiersState::SUPER);
        let Some(AppAction::CloseTab {
            tab,
            switch_to_last_active,
        }) = action
        else {
            panic!("expected close tab action");
        };
        assert_eq!(tab, rterm_core::TabId::new(1));
        assert!(!switch_to_last_active);
    }

    #[test]
    fn close_tab_shortcuts_honor_last_active_tab_config() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            switch_to_last_active_tab_when_closing_tab: Some(true),
            ..NativeConfigSnapshot::default()
        });

        for modifiers in [
            ModifiersState::SUPER,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        ] {
            let Some(AppAction::CloseTab {
                switch_to_last_active,
                ..
            }) = app.app_shell_action_for_key(&Key::Character("w".into()), modifiers)
            else {
                panic!("expected close tab action");
            };
            assert!(switch_to_last_active);
        }
    }

    #[test]
    fn recognizes_super_shift_default_domain_new_tab_shortcut() {
        let mut app = NativeWindowApp::new(None);

        let action = app.app_shell_action_for_key(
            &Key::Character("T".into()),
            ModifiersState::SUPER | ModifiersState::SHIFT,
        );

        assert!(matches!(action, Some(AppAction::NewTab { launch: None })));

        app.set_config_overrides(native_config_snapshot! {
            default_domain: Some("ssh-prod".to_owned()),
            ..NativeConfigSnapshot::default()
        });

        assert!(
            app.app_shell_action_for_key(
                &Key::Character("T".into()),
                ModifiersState::SUPER | ModifiersState::SHIFT,
            )
            .is_none()
        );
    }

    #[test]
    fn recognizes_super_tab_number_shortcuts() {
        let app = NativeWindowApp::new(None);

        let action =
            app.app_shell_action_for_key(&Key::Character("1".into()), ModifiersState::SUPER);
        let AppAction::ActivateTabIndex { index } = action.expect("expected activate tab 1") else {
            panic!("expected activate tab index");
        };
        assert_eq!(index, 0);

        let action =
            app.app_shell_action_for_key(&Key::Character("9".into()), ModifiersState::SUPER);
        let AppAction::ActivateTabIndex { index } = action.expect("expected activate last tab")
        else {
            panic!("expected activate tab index");
        };
        assert_eq!(index, -1);
    }

    #[test]
    fn recognizes_super_shift_bracket_tab_navigation_shortcuts() {
        let app = NativeWindowApp::new(None);

        let action = app.app_shell_action_for_key(
            &Key::Character("]".into()),
            ModifiersState::SUPER | ModifiersState::SHIFT,
        );
        let AppAction::ActivateTabRelative { offset } = action.expect("expected next tab") else {
            panic!("expected activate tab relative");
        };
        assert_eq!(offset, 1);

        let action = app.app_shell_action_for_key(
            &Key::Character("[".into()),
            ModifiersState::SUPER | ModifiersState::SHIFT,
        );
        let AppAction::ActivateTabRelative { offset } = action.expect("expected previous tab")
        else {
            panic!("expected activate tab relative");
        };
        assert_eq!(offset, -1);
    }

    #[test]
    fn ctrl_shift_brackets_are_not_default_tab_navigation_shortcuts() {
        let app = NativeWindowApp::new(None);

        assert!(
            app.app_shell_action_for_key(
                &Key::Character("]".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT,
            )
            .is_none()
        );
        assert!(
            app.app_shell_action_for_key(
                &Key::Character("[".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT,
            )
            .is_none()
        );
    }

    #[test]
    fn recognizes_default_tab_navigation_shortcuts() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(2),
        })
        .unwrap();

        let action =
            app.app_shell_action_for_key(&Key::Named(NamedKey::Tab), ModifiersState::CONTROL);
        let AppAction::ActivateTabRelative { offset } = action.expect("expected activate next tab")
        else {
            panic!("expected activate tab relative");
        };
        assert_eq!(offset, 1);

        let action = app.app_shell_action_for_key(
            &Key::Named(NamedKey::Tab),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );
        let AppAction::ActivateTabRelative { offset } =
            action.expect("expected activate previous tab")
        else {
            panic!("expected activate tab relative");
        };
        assert_eq!(offset, -1);

        let action =
            app.app_shell_action_for_key(&Key::Named(NamedKey::PageUp), ModifiersState::CONTROL);
        let AppAction::ActivateTabRelative { offset } =
            action.expect("expected activate previous tab")
        else {
            panic!("expected activate tab relative");
        };
        assert_eq!(offset, -1);

        let action =
            app.app_shell_action_for_key(&Key::Named(NamedKey::PageDown), ModifiersState::CONTROL);
        let AppAction::ActivateTabRelative { offset } = action.expect("expected activate next tab")
        else {
            panic!("expected activate tab relative");
        };
        assert_eq!(offset, 1);
    }

    #[test]
    fn native_key_assignment_entries_include_wezterm_default_tab_navigation_shortcuts() {
        let entries = native_window_key_assignment_entries();
        let briefs: Vec<&str> = entries
            .iter()
            .map(WindowCommandPaletteEntry::label)
            .collect();

        for expected in [
            "CTRL+SHIFT+T: New Tab",
            "SUPER+T: New Tab",
            "SUPER+SHIFT+T: New Tab",
            "CTRL+SHIFT+W: Close Current Tab",
            "SUPER+W: Close Current Tab",
            "CTRL+SHIFT+X: Activate Copy Mode",
            "CTRL+SHIFT+SPACE: Quick Select",
            "CTRL+SHIFT+F: Search",
            "SUPER+F: Search",
            "CTRL+SHIFT+1: Activate Tab 1",
            "CTRL+SHIFT+9: Activate Tab 9",
            "SUPER+1: Activate Tab 1",
            "SUPER+9: Activate Tab 9",
            "SUPER+SHIFT+[: Activate Tab Relative",
            "SUPER+SHIFT+]: Activate Tab Relative",
            "CTRL+TAB: Activate Tab Relative",
            "CTRL+SHIFT+TAB: Activate Tab Relative",
            "CTRL+PAGEUP: Activate Tab Relative",
            "CTRL+PAGEDOWN: Activate Tab Relative",
            "CTRL+SHIFT+PAGEUP: Move Tab Relative",
            "CTRL+SHIFT+PAGEDOWN: Move Tab Relative",
            "SHIFT+PAGEUP: Scroll By Page",
            "SHIFT+PAGEDOWN: Scroll By Page",
            "CTRL+SHIFT+ALT+\": Split Vertical",
            "CTRL+SHIFT+ALT+%: Split Horizontal",
            "CTRL+SHIFT+Z: Toggle Pane Zoom State",
            "CTRL+SHIFT+ALT+LEFTARROW: Adjust Pane Size",
            "CTRL+SHIFT+ALT+RIGHTARROW: Adjust Pane Size",
            "CTRL+SHIFT+ALT+UPARROW: Adjust Pane Size",
            "CTRL+SHIFT+ALT+DOWNARROW: Adjust Pane Size",
            "CTRL+SHIFT+LEFTARROW: Activate Pane Direction",
            "CTRL+SHIFT+RIGHTARROW: Activate Pane Direction",
            "CTRL+SHIFT+UPARROW: Activate Pane Direction",
            "CTRL+SHIFT+DOWNARROW: Activate Pane Direction",
        ] {
            assert!(
                briefs.contains(&expected),
                "missing native default key assignment entry {expected}"
            );
        }
        #[cfg(target_os = "macos")]
        assert!(briefs.contains(&"SUPER+H: Hide Application"));
        #[cfg(not(target_os = "macos"))]
        assert!(!briefs.contains(&"SUPER+H: Hide Application"));

        for (label, expected_command) in [
            (
                "CTRL+SHIFT+T: New Tab",
                WindowCommand::SpawnTab(WindowSpawnTabDomain::CurrentPaneDomain),
            ),
            (
                "SUPER+T: New Tab",
                WindowCommand::SpawnTab(WindowSpawnTabDomain::CurrentPaneDomain),
            ),
            (
                "SUPER+SHIFT+T: New Tab",
                WindowCommand::SpawnTab(WindowSpawnTabDomain::DefaultDomain),
            ),
            (
                "CTRL+SHIFT+1: Activate Tab 1",
                WindowCommand::ActivateTab(0),
            ),
            (
                "CTRL+SHIFT+9: Activate Tab 9",
                WindowCommand::ActivateTab(-1),
            ),
            ("SUPER+1: Activate Tab 1", WindowCommand::ActivateTab(0)),
            ("SUPER+9: Activate Tab 9", WindowCommand::ActivateTab(-1)),
            (
                "CTRL+TAB: Activate Tab Relative",
                WindowCommand::ActivateTabRelative(1),
            ),
            (
                "CTRL+SHIFT+W: Close Current Tab",
                WindowCommand::CloseCurrentTab { confirm: true },
            ),
            (
                "SUPER+W: Close Current Tab",
                WindowCommand::CloseCurrentTab { confirm: true },
            ),
            (
                "CTRL+SHIFT+X: Activate Copy Mode",
                WindowCommand::ActivateCopyMode,
            ),
            (
                "CTRL+SHIFT+SPACE: Quick Select",
                WindowCommand::QuickSelect(WindowQuickSelectOptions::default()),
            ),
            (
                "CTRL+SHIFT+F: Search",
                WindowCommand::Search(WindowSearchCommandQuery::Pattern {
                    pattern: String::new(),
                    match_type: WindowSearchMatchType::CaseSensitive,
                }),
            ),
            (
                "SUPER+F: Search",
                WindowCommand::Search(WindowSearchCommandQuery::Pattern {
                    pattern: String::new(),
                    match_type: WindowSearchMatchType::CaseSensitive,
                }),
            ),
            (
                "SHIFT+PAGEUP: Scroll By Page",
                WindowCommand::ScrollByPage(WindowScrollByPageAmount::from_per_mille(-1_000)),
            ),
            (
                "SHIFT+PAGEDOWN: Scroll By Page",
                WindowCommand::ScrollByPage(WindowScrollByPageAmount::from_per_mille(1_000)),
            ),
            (
                "CTRL+SHIFT+Z: Toggle Pane Zoom State",
                WindowCommand::TogglePaneZoomState,
            ),
        ] {
            let entry = entries
                .iter()
                .find(|entry| entry.label() == label)
                .unwrap_or_else(|| panic!("missing native default key assignment entry {label}"));
            assert_eq!(entry.clone().into_command(), expected_command);
        }

        for (label, expected_direction) in [
            (
                "CTRL+SHIFT+ALT+\": Split Vertical",
                rterm_core::app_shell::SplitDirection::Down,
            ),
            (
                "CTRL+SHIFT+ALT+%: Split Horizontal",
                rterm_core::app_shell::SplitDirection::Right,
            ),
        ] {
            let entry = entries
                .iter()
                .find(|entry| entry.label() == label)
                .unwrap_or_else(|| panic!("missing native default key assignment entry {label}"));
            let command = entry.clone().into_command();
            match &command {
                WindowCommand::SplitPane(WindowSplitPaneOptions {
                    direction,
                    command,
                    size,
                    top_level,
                    ..
                }) => {
                    assert_eq!(*direction, expected_direction);
                    assert!(command.is_none());
                    assert_eq!(*size, None);
                    assert!(!top_level);
                }
                command => panic!("expected {label} to use SplitPane payload, got {command:?}"),
            }
            assert!(
                format!("{command:?}").contains("CurrentPaneDomain"),
                "expected {label} to preserve WezTerm CurrentPaneDomain split default"
            );
        }
    }

    #[test]
    fn ctrl_n_and_p_are_not_default_workspace_shortcuts() {
        let app = NativeWindowApp::new(None);

        assert!(
            app.app_shell_action_for_key(&Key::Character("n".into()), ModifiersState::CONTROL)
                .is_none()
        );
        assert!(
            app.app_shell_action_for_key(&Key::Character("p".into()), ModifiersState::CONTROL)
                .is_none()
        );
    }

    #[test]
    fn recognizes_default_tab_move_shortcuts() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();

        let action = app.app_shell_action_for_key(
            &Key::Named(NamedKey::PageDown),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );
        let AppAction::MoveTabRelative { offset } = action.expect("expected move tab right") else {
            panic!("expected move tab relative");
        };
        assert_eq!(offset, 1);

        let action = app.app_shell_action_for_key(
            &Key::Named(NamedKey::PageUp),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );
        let AppAction::MoveTabRelative { offset } = action.expect("expected move tab left") else {
            panic!("expected move tab relative");
        };
        assert_eq!(offset, -1);
    }

    #[test]
    fn ctrl_shift_alt_page_keys_are_not_default_tab_move_shortcuts() {
        let app = NativeWindowApp::new(None);

        assert!(
            app.app_shell_action_for_key(
                &Key::Named(NamedKey::PageDown),
                ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT,
            )
            .is_none()
        );
        assert!(
            app.app_shell_action_for_key(
                &Key::Named(NamedKey::PageUp),
                ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT,
            )
            .is_none()
        );
    }

    #[test]
    fn recognizes_default_tab_number_shortcuts() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let action = app.app_shell_action_for_key(
            &Key::Character("1".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );
        let AppAction::ActivateTabIndex { index } = action.expect("expected activate tab 1") else {
            panic!("expected activate tab index");
        };
        assert_eq!(index, 0);

        let action = app.app_shell_action_for_key(
            &Key::Character("2".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );
        let AppAction::ActivateTabIndex { index } = action.expect("expected activate tab 2") else {
            panic!("expected activate tab index");
        };
        assert_eq!(index, 1);

        let action = app.app_shell_action_for_key(
            &Key::Character("(".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );
        let AppAction::ActivateTabIndex { index } = action.expect("expected activate last tab")
        else {
            panic!("expected activate tab index");
        };
        assert_eq!(index, -1);
    }

    #[test]
    fn recognizes_default_alt_split_shortcuts() {
        let app = NativeWindowApp::new(None);

        let action = app.app_shell_action_for_key(
            &Key::Character("\"".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT,
        );
        let AppAction::SplitPane { direction, .. } = action.expect("expected split pane action")
        else {
            panic!("expected split pane action");
        };
        assert_eq!(direction, rterm_core::app_shell::SplitDirection::Down);

        let action = app.app_shell_action_for_key(
            &Key::Character("%".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT,
        );
        let AppAction::SplitPane { direction, .. } = action.expect("expected split pane action")
        else {
            panic!("expected split pane action");
        };
        assert_eq!(direction, rterm_core::app_shell::SplitDirection::Right);
    }

    #[test]
    fn ctrl_shift_d_and_e_are_not_default_split_shortcuts() {
        let app = NativeWindowApp::new(None);

        assert!(
            app.app_shell_action_for_key(
                &Key::Character("d".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT,
            )
            .is_none()
        );
        assert!(
            app.app_shell_action_for_key(
                &Key::Character("e".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT,
            )
            .is_none()
        );
    }

    #[test]
    fn ctrl_shift_alt_single_quote_is_not_default_split_shortcut() {
        let app = NativeWindowApp::new(None);

        assert!(
            app.app_shell_action_for_key(
                &Key::Character("'".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT,
            )
            .is_none()
        );
    }

    #[test]
    fn recognizes_default_pane_navigation_shortcuts() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));

        let action = app.app_shell_action_for_key(
            &Key::Named(NamedKey::ArrowLeft),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );
        let AppAction::ActivatePaneDirection { direction } =
            action.expect("expected activate pane left")
        else {
            panic!("expected activate pane direction");
        };
        assert_eq!(direction, rterm_core::app_shell::PaneDirection::Left);

        let action = app.app_shell_action_for_key(
            &Key::Named(NamedKey::ArrowRight),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );
        let AppAction::ActivatePaneDirection { direction } =
            action.expect("expected activate pane right")
        else {
            panic!("expected activate pane direction");
        };
        assert_eq!(direction, rterm_core::app_shell::PaneDirection::Right);

        let action = app.app_shell_action_for_key(
            &Key::Named(NamedKey::ArrowUp),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );
        let AppAction::ActivatePaneDirection { direction } =
            action.expect("expected activate pane up")
        else {
            panic!("expected activate pane direction");
        };
        assert_eq!(direction, rterm_core::app_shell::PaneDirection::Up);

        let action = app.app_shell_action_for_key(
            &Key::Named(NamedKey::ArrowDown),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );
        let AppAction::ActivatePaneDirection { direction } =
            action.expect("expected activate pane down")
        else {
            panic!("expected activate pane direction");
        };
        assert_eq!(direction, rterm_core::app_shell::PaneDirection::Down);
    }

    #[test]
    fn recognizes_default_pane_resize_shortcuts() {
        let app = NativeWindowApp::new(None);

        let action = app.app_shell_action_for_key(
            &Key::Named(NamedKey::ArrowLeft),
            ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT,
        );
        let AppAction::ResizePane {
            direction, amount, ..
        } = action.expect("expected resize pane")
        else {
            panic!("expected resize pane");
        };
        assert_eq!(direction, rterm_core::app_shell::ResizeDirection::Left);
        assert_eq!(amount, 1);

        let action = app.app_shell_action_for_key(
            &Key::Named(NamedKey::ArrowDown),
            ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT,
        );
        let AppAction::ResizePane { direction, .. } = action.expect("expected resize pane") else {
            panic!("expected resize pane");
        };
        assert_eq!(direction, rterm_core::app_shell::ResizeDirection::Down);
    }

    #[test]
    fn recognizes_default_pane_zoom_shortcut() {
        let app = NativeWindowApp::new(None);

        let action = app.app_shell_action_for_key(
            &Key::Character("Z".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );

        let AppAction::TogglePaneZoom { pane } = action.expect("expected toggle pane zoom") else {
            panic!("expected toggle pane zoom");
        };
        assert_eq!(pane, rterm_core::PaneId::new(1));
    }

    #[test]
    fn recognizes_window_command_palette_shortcut() {
        assert!(NativeWindowApp::command_palette_shortcut(
            &Key::Character("p".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));
        assert!(!NativeWindowApp::command_palette_shortcut(
            &Key::Character("p".into()),
            ModifiersState::CONTROL
        ));
    }

    #[test]
    fn recognizes_window_reload_configuration_shortcut() {
        assert!(window_reload_configuration_shortcut(
            &Key::Character("r".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));
        assert!(window_reload_configuration_shortcut(
            &Key::Character("R".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));
        assert!(window_reload_configuration_shortcut(
            &Key::Character("r".into()),
            ModifiersState::SUPER
        ));
        assert!(!window_reload_configuration_shortcut(
            &Key::Character("r".into()),
            ModifiersState::CONTROL
        ));
    }

    #[test]
    fn recognizes_window_toggle_full_screen_shortcut() {
        assert!(window_toggle_full_screen_shortcut(
            &Key::Named(NamedKey::Enter),
            ModifiersState::ALT
        ));
        assert!(!window_toggle_full_screen_shortcut(
            &Key::Named(NamedKey::Enter),
            ModifiersState::empty()
        ));
        assert!(!window_toggle_full_screen_shortcut(
            &Key::Named(NamedKey::Enter),
            ModifiersState::ALT | ModifiersState::SHIFT
        ));
    }

    #[test]
    fn recognizes_window_hide_shortcut() {
        assert!(window_hide_shortcut(
            &Key::Character("m".into()),
            ModifiersState::SUPER
        ));
        assert!(window_hide_shortcut(
            &Key::Character("M".into()),
            ModifiersState::SUPER
        ));
        assert!(!window_hide_shortcut(
            &Key::Character("m".into()),
            ModifiersState::empty()
        ));
        assert!(!window_hide_shortcut(
            &Key::Character("m".into()),
            ModifiersState::SUPER | ModifiersState::SHIFT
        ));
    }

    #[test]
    fn recognizes_window_application_hide_shortcut() {
        #[cfg(target_os = "macos")]
        assert!(window_application_hide_shortcut(
            &Key::Character("h".into()),
            ModifiersState::SUPER
        ));
        #[cfg(not(target_os = "macos"))]
        assert!(!window_application_hide_shortcut(
            &Key::Character("h".into()),
            ModifiersState::SUPER
        ));
        #[cfg(target_os = "macos")]
        assert!(window_application_hide_shortcut(
            &Key::Character("H".into()),
            ModifiersState::SUPER
        ));
        #[cfg(not(target_os = "macos"))]
        assert!(!window_application_hide_shortcut(
            &Key::Character("H".into()),
            ModifiersState::SUPER
        ));
        assert!(!window_application_hide_shortcut(
            &Key::Character("h".into()),
            ModifiersState::empty()
        ));
        assert!(!window_application_hide_shortcut(
            &Key::Character("h".into()),
            ModifiersState::SUPER | ModifiersState::SHIFT
        ));
    }

    #[test]
    fn recognizes_window_font_size_shortcuts() {
        assert_eq!(
            window_font_size_shortcut(&Key::Character("-".into()), ModifiersState::CONTROL),
            Some(WindowFontSizeAction::Decrease)
        );
        assert_eq!(
            window_font_size_shortcut(&Key::Character("-".into()), ModifiersState::SUPER),
            Some(WindowFontSizeAction::Decrease)
        );
        assert_eq!(
            window_font_size_shortcut(&Key::Character("=".into()), ModifiersState::CONTROL),
            Some(WindowFontSizeAction::Increase)
        );
        assert_eq!(
            window_font_size_shortcut(&Key::Character("=".into()), ModifiersState::SUPER),
            Some(WindowFontSizeAction::Increase)
        );
        assert_eq!(
            window_font_size_shortcut(&Key::Character("0".into()), ModifiersState::CONTROL),
            Some(WindowFontSizeAction::Reset)
        );
        assert_eq!(
            window_font_size_shortcut(&Key::Character("0".into()), ModifiersState::SUPER),
            Some(WindowFontSizeAction::Reset)
        );
        assert_eq!(
            window_font_size_shortcut(&Key::Character("-".into()), ModifiersState::empty()),
            None
        );
        assert_eq!(
            window_font_size_shortcut(
                &Key::Character("=".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT
            ),
            None
        );
    }

    #[test]
    fn recognizes_window_show_debug_overlay_shortcut() {
        assert!(window_show_debug_overlay_shortcut(
            &Key::Character("l".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));
        assert!(window_show_debug_overlay_shortcut(
            &Key::Character("L".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));
        assert!(!window_show_debug_overlay_shortcut(
            &Key::Character("l".into()),
            ModifiersState::CONTROL
        ));
        assert!(!window_show_debug_overlay_shortcut(
            &Key::Character("l".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT
        ));
    }

    #[test]
    fn recognizes_window_char_select_shortcut() {
        assert!(window_char_select_shortcut(
            &Key::Character("u".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));
        assert!(window_char_select_shortcut(
            &Key::Character("U".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));
        assert!(!window_char_select_shortcut(
            &Key::Character("u".into()),
            ModifiersState::CONTROL
        ));
        assert!(!window_char_select_shortcut(
            &Key::Character("u".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT
        ));
    }

    #[test]
    fn window_command_palette_labels_wezterm_current_pane_and_tab_close_actions() {
        assert_eq!(WindowCommand::ClosePane.label(), "Close Current Pane");
        assert_eq!(WindowCommand::CloseTab.label(), "Close Current Tab");
    }

    #[test]
    fn window_command_palette_labels_wezterm_split_horizontal_and_vertical_actions() {
        assert_eq!(WindowCommand::SplitRight.label(), "Split Horizontal");
        assert_eq!(WindowCommand::SplitDown.label(), "Split Vertical");
        assert_eq!(WindowCommand::SplitHorizontal.label(), "Split Horizontal");
        assert_eq!(WindowCommand::SplitVertical.label(), "Split Vertical");
    }

    #[test]
    fn window_command_palette_labels_wezterm_adjust_pane_size_actions() {
        assert_eq!(
            WindowCommand::ResizePaneLeft.label(),
            "Adjust Pane Size Left"
        );
        assert_eq!(
            WindowCommand::ResizePaneRight.label(),
            "Adjust Pane Size Right"
        );
        assert_eq!(WindowCommand::ResizePaneUp.label(), "Adjust Pane Size Up");
        assert_eq!(
            WindowCommand::ResizePaneDown.label(),
            "Adjust Pane Size Down"
        );
    }

    #[test]
    fn window_command_palette_labels_wezterm_toggle_pane_zoom_state_action() {
        assert_eq!(
            WindowCommand::TogglePaneZoom.label(),
            "Toggle Pane Zoom State"
        );
    }

    #[test]
    fn window_command_palette_labels_wezterm_font_size_actions() {
        assert_eq!(
            WindowCommand::DecreaseFontSize.label(),
            "Decrease Font Size"
        );
        assert_eq!(
            WindowCommand::IncreaseFontSize.label(),
            "Increase Font Size"
        );
        assert_eq!(WindowCommand::ResetFontSize.label(), "Reset Font Size");
        assert_eq!(
            WindowCommand::ResetFontAndWindowSize.label(),
            "Reset Font And Window Size"
        );
    }

    #[test]
    fn window_command_palette_labels_wezterm_quit_application_action() {
        assert_eq!(WindowCommand::QuitApplication.label(), "Quit Application");
    }

    #[test]
    fn window_command_palette_labels_wezterm_show_action() {
        assert_eq!(WindowCommand::Show.label(), "Show");
    }

    #[test]
    fn window_command_palette_labels_wezterm_switch_to_workspace_action() {
        assert_eq!(
            WindowCommand::SwitchToWorkspace.label(),
            "Switch To Workspace"
        );
        assert_eq!(
            WindowCommand::SwitchWorkspaceRelative(1).label(),
            "Switch Workspace Relative"
        );
    }

    #[test]
    fn window_command_palette_labels_wezterm_move_tab_relative_actions() {
        assert_eq!(
            WindowCommand::MoveTabRelativeLeft.label(),
            "Move Tab Relative Left"
        );
        assert_eq!(
            WindowCommand::MoveTabRelativeRight.label(),
            "Move Tab Relative Right"
        );
    }

    #[test]
    fn window_command_palette_labels_wezterm_activate_tab_actions() {
        assert_eq!(WindowCommand::ActivateTab1.label(), "Activate Tab 1");
        assert_eq!(WindowCommand::ActivateTab2.label(), "Activate Tab 2");
        assert_eq!(WindowCommand::ActivateTab3.label(), "Activate Tab 3");
        assert_eq!(WindowCommand::ActivateTab4.label(), "Activate Tab 4");
        assert_eq!(WindowCommand::ActivateTab5.label(), "Activate Tab 5");
        assert_eq!(WindowCommand::ActivateTab6.label(), "Activate Tab 6");
        assert_eq!(WindowCommand::ActivateTab7.label(), "Activate Tab 7");
        assert_eq!(WindowCommand::ActivateTab8.label(), "Activate Tab 8");
        assert_eq!(WindowCommand::ActivateTab9.label(), "Activate Tab 9");
    }

    #[test]
    fn window_command_palette_labels_wezterm_show_tab_navigator_action() {
        assert_eq!(
            WindowCommand::ShowTabNavigator.label(),
            "Show Tab Navigator"
        );
    }

    #[test]
    fn window_command_palette_labels_wezterm_show_launcher_action() {
        assert_eq!(WindowCommand::ShowLauncher.label(), "Show Launcher");
    }

    #[test]
    fn window_command_palette_labels_wezterm_set_window_level_action() {
        assert_eq!(
            WindowCommand::SetWindowLevel(NativeWindowLevel::AlwaysOnTop).label(),
            "Set Window Level"
        );
    }

    #[test]
    fn window_native_level_maps_to_winit_window_level() {
        assert_eq!(
            winit_window_level_for_native(NativeWindowLevel::AlwaysOnBottom),
            winit::window::WindowLevel::AlwaysOnBottom
        );
        assert_eq!(
            winit_window_level_for_native(NativeWindowLevel::Normal),
            winit::window::WindowLevel::Normal
        );
        assert_eq!(
            winit_window_level_for_native(NativeWindowLevel::AlwaysOnTop),
            winit::window::WindowLevel::AlwaysOnTop
        );
    }

    #[test]
    fn window_command_palette_labels_wezterm_toggle_window_level_actions() {
        assert_eq!(
            WindowCommand::ToggleAlwaysOnTop.label(),
            "Toggle Always On Top"
        );
        assert_eq!(
            WindowCommand::ToggleAlwaysOnBottom.label(),
            "Toggle Always On Bottom"
        );
    }

    #[test]
    fn window_command_palette_labels_wezterm_start_window_drag_action() {
        assert_eq!(WindowCommand::StartWindowDrag.label(), "Start Window Drag");
    }

    #[test]
    fn window_tab_navigator_uses_modern_selected_surface_by_default() {
        let mut app = NativeWindowApp::new(None);
        app.command_palette_execute(WindowCommand::ShowTabNavigator);

        let snapshot = app.render_snapshot();
        let selected = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0)
            .expect("expected selected tab-navigator row");
        assert_eq!(selected.foreground, DEFAULT_UI_ACCENT_FOREGROUND);
        assert_eq!(selected.background, DEFAULT_UI_ACCENT_BACKGROUND);
    }

    #[test]
    fn window_command_palette_labels_wezterm_activate_window_relative_actions() {
        assert_eq!(WindowCommand::ActivateWindow(2).label(), "Activate Window");
        assert_eq!(
            WindowCommand::ActivateWindowRelative(1).label(),
            "Activate Window Relative"
        );
        assert_eq!(
            WindowCommand::ActivateWindowRelativeNoWrap(-1).label(),
            "Activate Window Relative No Wrap"
        );
    }

    #[test]
    fn window_app_show_tab_navigator_selects_and_activates_tab() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        app.command_palette_execute(WindowCommand::ShowTabNavigator);

        assert_eq!(
            app.tab_navigator
                .as_ref()
                .map(|navigator| navigator.selected),
            Some(2)
        );
        assert_eq!(app.app_shell.active_tab_id(), rterm_core::TabId::new(3));

        assert!(
            app.handle_tab_navigator_key(&Key::Named(NamedKey::ArrowUp), ModifiersState::empty())
        );
        assert_eq!(
            app.tab_navigator
                .as_ref()
                .map(|navigator| navigator.selected),
            Some(1)
        );
        assert!(
            app.handle_tab_navigator_key(&Key::Named(NamedKey::Enter), ModifiersState::empty())
        );

        assert!(app.tab_navigator.is_none());
        assert_eq!(app.app_shell.active_tab_id(), rterm_core::TabId::new(2));
    }

    #[test]
    fn window_command_palette_labels_wezterm_move_tab_actions() {
        assert_eq!(WindowCommand::MoveTabTo1.label(), "Move Tab To 1");
        assert_eq!(WindowCommand::MoveTabTo2.label(), "Move Tab To 2");
        assert_eq!(WindowCommand::MoveTabTo3.label(), "Move Tab To 3");
        assert_eq!(WindowCommand::MoveTabTo4.label(), "Move Tab To 4");
        assert_eq!(WindowCommand::MoveTabTo5.label(), "Move Tab To 5");
        assert_eq!(WindowCommand::MoveTabTo6.label(), "Move Tab To 6");
        assert_eq!(WindowCommand::MoveTabTo7.label(), "Move Tab To 7");
        assert_eq!(WindowCommand::MoveTabTo8.label(), "Move Tab To 8");
    }

    #[test]
    fn window_command_palette_labels_wezterm_activate_pane_direction_actions() {
        assert_eq!(
            WindowCommand::ActivatePaneLeft.label(),
            "Activate Pane Direction Left"
        );
        assert_eq!(
            WindowCommand::ActivatePaneRight.label(),
            "Activate Pane Direction Right"
        );
        assert_eq!(
            WindowCommand::ActivatePaneUp.label(),
            "Activate Pane Direction Up"
        );
        assert_eq!(
            WindowCommand::ActivatePaneDown.label(),
            "Activate Pane Direction Down"
        );
        assert_eq!(
            WindowCommand::NextPane.label(),
            "Activate Pane Direction Next"
        );
        assert_eq!(
            WindowCommand::PreviousPane.label(),
            "Activate Pane Direction Previous"
        );
    }

    #[test]
    fn window_command_palette_labels_wezterm_activate_pane_by_index_actions() {
        assert_eq!(
            WindowCommand::ActivatePane1.label(),
            "Activate Pane By Index 1"
        );
        assert_eq!(
            WindowCommand::ActivatePane2.label(),
            "Activate Pane By Index 2"
        );
        assert_eq!(
            WindowCommand::ActivatePane3.label(),
            "Activate Pane By Index 3"
        );
        assert_eq!(
            WindowCommand::ActivatePane4.label(),
            "Activate Pane By Index 4"
        );
        assert_eq!(
            WindowCommand::ActivatePane5.label(),
            "Activate Pane By Index 5"
        );
        assert_eq!(
            WindowCommand::ActivatePane6.label(),
            "Activate Pane By Index 6"
        );
        assert_eq!(
            WindowCommand::ActivatePane7.label(),
            "Activate Pane By Index 7"
        );
        assert_eq!(
            WindowCommand::ActivatePane8.label(),
            "Activate Pane By Index 8"
        );
    }

    #[test]
    fn window_command_palette_labels_wezterm_select_text_at_mouse_cursor_actions() {
        assert_eq!(
            WindowCommand::SelectTextAtMouseCursorCell.label(),
            "Select Text At Mouse Cursor Cell"
        );
        assert_eq!(
            WindowCommand::SelectTextAtMouseCursorWord.label(),
            "Select Text At Mouse Cursor Word"
        );
        assert_eq!(
            WindowCommand::SelectTextAtMouseCursorLine.label(),
            "Select Text At Mouse Cursor Line"
        );
        assert_eq!(
            WindowCommand::SelectTextAtMouseCursorBlock.label(),
            "Select Text At Mouse Cursor Block"
        );
        assert_eq!(
            WindowCommand::SelectTextAtMouseCursorSemanticZone.label(),
            "Select Text At Mouse Cursor Semantic Zone"
        );
        assert_eq!(
            WindowCommand::SelectTextAtMouseCursor(WindowMouseSelectionMode::SemanticZone).label(),
            "Select Text At Mouse Cursor"
        );
        assert_eq!(
            WindowCommand::ExtendSelectionToMouseCursorCell.label(),
            "Extend Selection To Mouse Cursor Cell"
        );
        assert_eq!(
            WindowCommand::ExtendSelectionToMouseCursorWord.label(),
            "Extend Selection To Mouse Cursor Word"
        );
        assert_eq!(
            WindowCommand::ExtendSelectionToMouseCursorLine.label(),
            "Extend Selection To Mouse Cursor Line"
        );
        assert_eq!(
            WindowCommand::ExtendSelectionToMouseCursorBlock.label(),
            "Extend Selection To Mouse Cursor Block"
        );
        assert_eq!(
            WindowCommand::ExtendSelectionToMouseCursorSemanticZone.label(),
            "Extend Selection To Mouse Cursor Semantic Zone"
        );
        assert_eq!(
            WindowCommand::ExtendSelectionToMouseCursor(WindowMouseSelectionMode::Block).label(),
            "Extend Selection To Mouse Cursor"
        );
    }

    #[test]
    fn window_app_dispatches_palette_mouse_selection_action_name_queries() {
        for (query, expected) in [
            (
                "selecttextatmousecursor semanticzone",
                WindowCommand::SelectTextAtMouseCursor(WindowMouseSelectionMode::SemanticZone),
            ),
            (
                "extendselectiontomousecursor block",
                WindowCommand::ExtendSelectionToMouseCursor(WindowMouseSelectionMode::Block),
            ),
        ] {
            let mut app = NativeWindowApp::new(None);
            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());

            assert_eq!(app.command_palette_filtered_commands(), vec![expected]);
        }
    }

    #[test]
    fn window_app_exposes_palette_extend_selection_to_mouse_cursor_semantic_zone_command() {
        assert!(
            super::WINDOW_COMMANDS
                .contains(&WindowCommand::ExtendSelectionToMouseCursorSemanticZone)
        );
    }

    #[test]
    fn window_app_dispatches_palette_select_text_at_mouse_cursor_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.enter_command_palette_mode();
        app.command_palette_set_query("select text at mouse cursor=semanticzone".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SelectTextAtMouseCursor(
                WindowMouseSelectionMode::SemanticZone
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_select_text_at_mouse_cursor_compact_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.enter_command_palette_mode();
        app.command_palette_set_query("selecttextatmousecursor=semanticzone".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SelectTextAtMouseCursor(
                WindowMouseSelectionMode::SemanticZone
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_select_text_at_mouse_cursor_mode_assignment_query() {
        let mut app = NativeWindowApp::new(None);
        app.enter_command_palette_mode();
        app.command_palette_set_query("selecttextatmousecursor mode=semanticzone".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SelectTextAtMouseCursor(
                WindowMouseSelectionMode::SemanticZone
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_select_text_at_mouse_cursor_wezterm_action_bare_string_query()
    {
        let mut app = NativeWindowApp::new(None);
        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SelectTextAtMouseCursor 'SemanticZone'".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SelectTextAtMouseCursor(
                WindowMouseSelectionMode::SemanticZone
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_select_text_at_mouse_cursor_wezterm_action_function_call_query()
     {
        let mut app = NativeWindowApp::new(None);
        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SelectTextAtMouseCursor('SemanticZone')".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SelectTextAtMouseCursor(
                WindowMouseSelectionMode::SemanticZone
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_extend_selection_to_mouse_cursor_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.enter_command_palette_mode();
        app.command_palette_set_query("extend selection to mouse cursor=block".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ExtendSelectionToMouseCursor(
                WindowMouseSelectionMode::Block
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_extend_selection_to_mouse_cursor_compact_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.enter_command_palette_mode();
        app.command_palette_set_query("extendselectiontomousecursor=block".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ExtendSelectionToMouseCursor(
                WindowMouseSelectionMode::Block
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_extend_selection_to_mouse_cursor_mode_assignment_query() {
        let mut app = NativeWindowApp::new(None);
        app.enter_command_palette_mode();
        app.command_palette_set_query("extendselectiontomousecursor mode=block".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ExtendSelectionToMouseCursor(
                WindowMouseSelectionMode::Block
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_extend_selection_to_mouse_cursor_wezterm_action_bare_string_query()
     {
        let mut app = NativeWindowApp::new(None);
        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.ExtendSelectionToMouseCursor 'Block'".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ExtendSelectionToMouseCursor(
                WindowMouseSelectionMode::Block
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_extend_selection_to_mouse_cursor_wezterm_action_function_call_query()
     {
        let mut app = NativeWindowApp::new(None);
        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.ExtendSelectionToMouseCursor('Block')".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ExtendSelectionToMouseCursor(
                WindowMouseSelectionMode::Block
            )]
        );
    }

    #[test]
    fn window_command_palette_labels_wezterm_scrollback_navigation_actions() {
        assert_eq!(
            WindowCommand::ScrollByPage(WindowScrollByPageAmount::from_per_mille(-500)).label(),
            "Scroll By Page"
        );
        assert_eq!(WindowCommand::ScrollByLine(-1).label(), "Scroll By Line");
        assert_eq!(WindowCommand::ScrollPageUp.label(), "Scroll By Page Up");
        assert_eq!(WindowCommand::ScrollPageDown.label(), "Scroll By Page Down");
        assert_eq!(WindowCommand::ScrollLineUp.label(), "Scroll By Line Up");
        assert_eq!(WindowCommand::ScrollLineDown.label(), "Scroll By Line Down");
        assert_eq!(
            WindowCommand::ScrollToPrompt(-1).label(),
            "Scroll To Prompt"
        );
        assert_eq!(
            WindowCommand::ScrollToPreviousPrompt.label(),
            "Scroll To Prompt Previous"
        );
        assert_eq!(
            WindowCommand::ScrollToNextPrompt.label(),
            "Scroll To Prompt Next"
        );
    }

    #[test]
    fn window_app_dispatches_palette_new_tab_command() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::NewTab);

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_spawn_tab_local_domain_subset() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::SpawnTab(
            WindowSpawnTabDomain::CurrentPaneDomain,
        )));
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::SpawnTab(
            WindowSpawnTabDomain::DefaultDomain,
        )));
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::SpawnTab(
            WindowSpawnTabDomain::DomainName("local".to_owned()),
        )));
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(4));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_tab_domain_queries() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        for (query, expected_tab, expected_domain) in [
            (
                "spawn tab current pane domain",
                rterm_core::TabId::new(2),
                WindowSpawnTabDomain::CurrentPaneDomain,
            ),
            (
                "spawn tab default domain",
                rterm_core::TabId::new(3),
                WindowSpawnTabDomain::DefaultDomain,
            ),
            (
                "spawn tab domain local",
                rterm_core::TabId::new(4),
                WindowSpawnTabDomain::DomainName("local".to_owned()),
            ),
            (
                "spawn tab domain \"local\"",
                rterm_core::TabId::new(5),
                WindowSpawnTabDomain::DomainName("local".to_owned()),
            ),
        ] {
            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected_command = WindowCommand::SpawnTab(expected_domain);
            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected_command.clone()]
            );

            assert!(app.command_palette_execute(expected_command));
            assert_eq!(app.active_tab_id(), expected_tab);
            assert!(app.command_palette.is_none());
        }
    }

    #[test]
    fn window_app_dispatches_palette_spawn_tab_action_name_queries() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        for (query, expected_tab, expected_domain) in [
            (
                "spawntab current pane domain",
                rterm_core::TabId::new(2),
                WindowSpawnTabDomain::CurrentPaneDomain,
            ),
            (
                "spawntab default domain",
                rterm_core::TabId::new(3),
                WindowSpawnTabDomain::DefaultDomain,
            ),
            (
                "spawntab domain \"local\"",
                rterm_core::TabId::new(4),
                WindowSpawnTabDomain::DomainName("local".to_owned()),
            ),
            (
                "SpawnTab Domain Name \"local\"",
                rterm_core::TabId::new(5),
                WindowSpawnTabDomain::DomainName("local".to_owned()),
            ),
        ] {
            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected_command = WindowCommand::SpawnTab(expected_domain);
            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected_command.clone()]
            );

            assert!(app.command_palette_execute(expected_command));
            assert_eq!(app.active_tab_id(), expected_tab);
            assert!(app.command_palette.is_none());
        }
    }

    #[test]
    fn window_app_dispatches_palette_spawn_tab_wezterm_action_function_call_queries() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        for (query, expected_tab, expected_domain) in [
            (
                "wezterm.action.SpawnTab('CurrentPaneDomain')",
                rterm_core::TabId::new(2),
                WindowSpawnTabDomain::CurrentPaneDomain,
            ),
            (
                "wezterm.action.SpawnTab(\"DefaultDomain\")",
                rterm_core::TabId::new(3),
                WindowSpawnTabDomain::DefaultDomain,
            ),
        ] {
            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected_command = WindowCommand::SpawnTab(expected_domain);
            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected_command.clone()]
            );

            assert!(app.command_palette_execute(expected_command));
            assert_eq!(app.active_tab_id(), expected_tab);
            assert!(app.command_palette.is_none());
        }
    }

    #[test]
    fn window_app_dispatches_palette_spawn_tab_wezterm_action_bare_string_queries() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        for (query, expected_tab, expected_domain) in [
            (
                "wezterm.action.SpawnTab 'CurrentPaneDomain'",
                rterm_core::TabId::new(2),
                WindowSpawnTabDomain::CurrentPaneDomain,
            ),
            (
                "wezterm.action.SpawnTab \"DefaultDomain\"",
                rterm_core::TabId::new(3),
                WindowSpawnTabDomain::DefaultDomain,
            ),
        ] {
            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected_command = WindowCommand::SpawnTab(expected_domain);
            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected_command.clone()]
            );

            assert!(app.command_palette_execute(expected_command));
            assert_eq!(app.active_tab_id(), expected_tab);
            assert!(app.command_palette.is_none());
        }
    }

    #[test]
    fn window_app_dispatches_palette_spawn_tab_wezterm_action_domain_name_table_queries() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        for (query, expected_tab) in [
            (
                "wezterm.action.SpawnTab { DomainName = 'local' }",
                rterm_core::TabId::new(2),
            ),
            (
                "wezterm.action.SpawnTab({ DomainName = 'local' })",
                rterm_core::TabId::new(3),
            ),
        ] {
            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected_command =
                WindowCommand::SpawnTab(WindowSpawnTabDomain::DomainName("local".to_owned()));
            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected_command.clone()]
            );

            assert!(app.command_palette_execute(expected_command));
            assert_eq!(app.active_tab_id(), expected_tab);
            assert!(app.command_palette.is_none());
        }
    }

    #[test]
    fn window_app_dispatches_palette_spawn_tab_table_long_bracket_key_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SpawnTab { [[=[DomainName]=]] = [[local]] }".to_owned(),
        );
        let expected_command =
            WindowCommand::SpawnTab(WindowSpawnTabDomain::DomainName("local".to_owned()));
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected_command.clone()]
        );

        assert!(app.command_palette_execute(expected_command));
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_parses_wezterm_attach_domain_action_queries_as_unsupported_actions() {
        let mut app = NativeWindowApp::new(None);

        for query in [
            "wezterm.action.AttachDomain('devhost')",
            "wezterm.action.AttachDomain 'devhost'",
            "act.AttachDomain \"devhost\"",
            "attach domain name devhost",
        ] {
            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected = WindowCommand::AttachDomain("devhost".to_owned());

            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected.clone()]
            );
            assert!(!app.command_palette_execute(expected));
            assert!(app.command_palette.is_some());
        }
    }

    #[test]
    fn window_app_dispatches_wezterm_attach_domain_action_queries_as_supported_local_actions() {
        for (query, expected_domain) in [
            ("wezterm.action.AttachDomain('local')", "local"),
            ("wezterm.action.AttachDomain \"Local\"", "Local"),
            ("attach domain DefaultDomain", "DefaultDomain"),
            ("act.AttachDomain \"currentpane\"", "currentpane"),
            ("attach domain current-pane-domain", "current-pane-domain"),
            ("attach domain current_pane_domain", "current_pane_domain"),
            ("attach domain name current", "current"),
            ("attach domain default-domain", "default-domain"),
            ("attach domain default_domain", "default_domain"),
        ] {
            let mut app = NativeWindowApp::new_with_command(
                None,
                rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
            );

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected = WindowCommand::AttachDomain(expected_domain.to_owned());

            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected.clone()]
            );
            assert!(app.command_palette_execute(expected));
            assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
            assert!(app.command_palette.is_none());
        }
    }

    #[test]
    fn window_app_parses_wezterm_attach_domain_action_queries_as_table_actions() {
        for (query, expected_domain) in [
            (
                "wezterm.action.AttachDomain({ DomainName = 'local' })",
                "local",
            ),
            ("act.AttachDomain { DomainName = \"local\" }", "local"),
            ("act.AttachDomain('default-domain')", "default-domain"),
            (
                "wezterm.action { AttachDomain = \"default_domain\" }",
                "default_domain",
            ),
        ] {
            let mut app = NativeWindowApp::new_with_command(
                None,
                rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
            );
            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected = WindowCommand::AttachDomain(expected_domain.to_owned());

            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected.clone()]
            );
            assert!(app.command_palette_execute(expected));
            assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
            assert!(app.command_palette.is_none());
        }
    }

    #[test]
    fn window_app_dispatches_wezterm_attach_domain_action_table_wrapper_queries() {
        for (query, expected_domain) in [
            ("wezterm.action { AttachDomain = 'local' }", "local"),
            ("act { AttachDomain = { DomainName = \"local\" } }", "local"),
            (
                "wezterm.action { AttachDomain = { DomainName = 'default' } }",
                "default",
            ),
            (
                "wezterm.action { AttachDomain = { DomainName = 'default domain' } }",
                "default domain",
            ),
            (
                "act { AttachDomain = { DomainName = 'default-domain' } }",
                "default-domain",
            ),
            (
                "wezterm.action { AttachDomain = { DomainName = 'default_domain' } }",
                "default_domain",
            ),
        ] {
            let mut app = NativeWindowApp::new_with_command(
                None,
                rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
            );
            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected = WindowCommand::AttachDomain(expected_domain.to_owned());

            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected.clone()]
            );
            assert!(app.command_palette_execute(expected));
            assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
            assert!(app.command_palette.is_none());
        }
    }

    #[test]
    fn window_app_parses_wezterm_attach_domain_action_queries_as_domain_id_unsupported_actions() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.AttachDomain({ DomainId = 7 })".to_owned());
        let expected = WindowCommand::AttachDomain("domainid:7".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(!app.command_palette_execute(expected));
        assert!(app.command_palette.is_some());
    }

    #[test]
    fn window_app_rejects_wezterm_attach_domain_action_table_wrapper_domain_id_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action { AttachDomain = { DomainId = 7 } }".to_owned(),
        );
        let expected = WindowCommand::AttachDomain("domainid:7".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(!app.command_palette_execute(expected));
        assert!(app.command_palette.is_some());
    }

    #[test]
    fn window_app_rejects_default_domain_attach_domain_when_default_is_non_local() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            default_domain: Some("remote-default".to_owned()),
            exec_domains: Some(vec![NativeExecDomain {
                name: "remote-default".to_owned(),
                fixup_command: "wezterm cli spawn".to_owned(),
                label: None,
            }]),
            ..NativeConfigSnapshot::default()
        });

        app.enter_command_palette_mode();
        app.command_palette_set_query("attach domain defaultdomain".to_owned());
        let expected = WindowCommand::AttachDomain("defaultdomain".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(!app.command_palette_execute(expected));
        assert!(app.command_palette.is_some());
    }

    #[test]
    fn window_app_dispatches_wezterm_detach_domain_action_queries_as_supported_local_actions() {
        let mut app = NativeWindowApp::new(None);

        for (query, expected_domain) in [
            (
                "wezterm.action.DetachDomain 'CurrentPaneDomain'",
                WindowDomainSelector::CurrentPaneDomain,
            ),
            (
                "wezterm.action.DetachDomain 'DefaultDomain'",
                WindowDomainSelector::DefaultDomain,
            ),
            (
                "detach domain defaultdomain",
                WindowDomainSelector::DefaultDomain,
            ),
            ("detach domain default", WindowDomainSelector::DefaultDomain),
            (
                "detach domain default-domain",
                WindowDomainSelector::DefaultDomain,
            ),
            (
                "detach domain default_domain",
                WindowDomainSelector::DefaultDomain,
            ),
            (
                "wezterm.action { DetachDomain = 'CurrentPaneDomain' }",
                WindowDomainSelector::CurrentPaneDomain,
            ),
            (
                "wezterm.action { DetachDomain = 'DefaultDomain' }",
                WindowDomainSelector::DefaultDomain,
            ),
            (
                "wezterm.action { DetachDomain = { DomainName = 'local' } }",
                WindowDomainSelector::DomainName("local".to_owned()),
            ),
            (
                "act { DetachDomain = { DomainName = 'default-domain' } }",
                WindowDomainSelector::DomainName("default-domain".to_owned()),
            ),
            (
                "wezterm.action { DetachDomain = { DomainName = 'default' } }",
                WindowDomainSelector::DomainName("default".to_owned()),
            ),
            (
                "wezterm.action { DetachDomain = { DomainName = 'default domain' } }",
                WindowDomainSelector::DomainName("default domain".to_owned()),
            ),
            (
                "act { DetachDomain = { DomainName = 'default_domain' } }",
                WindowDomainSelector::DomainName("default_domain".to_owned()),
            ),
        ] {
            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected = WindowCommand::DetachDomain(expected_domain);

            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected.clone()]
            );
            assert!(app.command_palette_execute(expected));
            assert!(app.command_palette.is_none());
        }
    }

    #[test]
    fn window_app_parses_wezterm_detach_domain_action_queries_as_unsupported_actions() {
        let mut app = NativeWindowApp::new(None);

        for (query, expected_domain) in [
            (
                "wezterm.action.DetachDomain({ DomainName = 'devhost' })",
                WindowDomainSelector::DomainName("devhost".to_owned()),
            ),
            (
                "act.DetachDomain { DomainName = 'devhost' }",
                WindowDomainSelector::DomainName("devhost".to_owned()),
            ),
            (
                "detach domain name devhost",
                WindowDomainSelector::DomainName("devhost".to_owned()),
            ),
        ] {
            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected = WindowCommand::DetachDomain(expected_domain);

            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected.clone()]
            );
            assert!(!app.command_palette_execute(expected));
            assert!(app.command_palette.is_some());
        }
    }

    #[test]
    fn window_app_rejects_default_domain_detach_domain_when_default_is_non_local() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            default_domain: Some("remote-default".to_owned()),
            exec_domains: Some(vec![NativeExecDomain {
                name: "remote-default".to_owned(),
                fixup_command: "wezterm cli spawn".to_owned(),
                label: None,
            }]),
            ..NativeConfigSnapshot::default()
        });

        app.enter_command_palette_mode();
        app.command_palette_set_query("detach domain defaultdomain".to_owned());
        let expected = WindowCommand::DetachDomain(WindowDomainSelector::DefaultDomain);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(!app.command_palette_execute(expected));
        assert!(app.command_palette.is_some());
    }

    #[test]
    fn window_app_rejects_wezterm_detach_domain_action_table_wrapper_queries() {
        let mut app = NativeWindowApp::new(None);

        for (query, expected_domain) in [
            (
                "act { DetachDomain = { DomainName = 'devhost' } }",
                WindowDomainSelector::DomainName("devhost".to_owned()),
            ),
            (
                "act { DetachDomain = { DomainId = 7 } }",
                WindowDomainSelector::DomainId(7),
            ),
        ] {
            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected = WindowCommand::DetachDomain(expected_domain);

            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected.clone()]
            );
            assert!(!app.command_palette_execute(expected));
            assert!(app.command_palette.is_some());
        }
    }

    #[test]
    fn window_app_parses_wezterm_detach_domain_table_long_bracket_key_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.DetachDomain({ [[=[DomainName]=]] = [[devhost]] })".to_owned(),
        );
        let expected =
            WindowCommand::DetachDomain(WindowDomainSelector::DomainName("devhost".to_owned()));

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(!app.command_palette_execute(expected));
        assert!(app.command_palette.is_some());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_tab_equals_queries() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        for (query, expected_tab, expected_domain) in [
            (
                "spawn tab=current pane domain",
                rterm_core::TabId::new(2),
                WindowSpawnTabDomain::CurrentPaneDomain,
            ),
            (
                "spawntab=domain \"local\"",
                rterm_core::TabId::new(3),
                WindowSpawnTabDomain::DomainName("local".to_owned()),
            ),
            (
                "spawntab=domain name=\"local\"",
                rterm_core::TabId::new(4),
                WindowSpawnTabDomain::DomainName("local".to_owned()),
            ),
        ] {
            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected_command = WindowCommand::SpawnTab(expected_domain);
            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected_command.clone()]
            );

            assert!(app.command_palette_execute(expected_command));
            assert_eq!(app.active_tab_id(), expected_tab);
            assert!(app.command_palette.is_none());
        }
    }

    #[test]
    fn window_app_rejects_default_domain_spawn_tab_when_configured_domain_is_not_local() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.set_config_overrides(native_config_snapshot! {
            default_domain: Some("ssh-prod".to_owned()),
            ..NativeConfigSnapshot::default()
        });

        app.enter_command_palette_mode();
        assert!(!app.command_palette_execute(WindowCommand::SpawnTab(
            WindowSpawnTabDomain::DefaultDomain,
        )));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
    }

    #[test]
    fn window_app_dispatches_palette_new_tab_spawn_command_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("new tab top -d 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );

        app.command_palette_execute(WindowCommand::NewTab);

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_new_tab_spawn_command_equals_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("new tab=top -d 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );

        app.command_palette_execute(WindowCommand::NewTab);

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_new_tab_spawn_command_local_domain_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("new tab --domain local top -d 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_new_tab_spawn_command_hyphenated_current_pane_domain_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("new tab --domain current-pane-domain top -d 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_new_tab_spawn_command_cwd_and_env_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "new tab --cwd /tmp/project --env SPAWN_MODE=query top -d 1".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("/tmp/project"));
        assert_eq!(
            launch.environment().get("SPAWN_MODE"),
            Some(&"query".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_applies_palette_new_tab_spawn_options_without_program_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "new tab --domain local --cwd \"C:/Project Dir\" --env \"GREETING=hello world\""
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-NoProfile"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert_eq!(
            launch.environment().get("GREETING"),
            Some(&"hello world".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_new_tab_spawn_command_quoted_query_values() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "new tab --cwd \"C:/Project Dir\" --env \"GREETING=hello world\" powershell \"-No Logo\""
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-No Logo"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert_eq!(
            launch.environment().get("GREETING"),
            Some(&"hello world".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_new_tab_spawn_command_set_environment_variables_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "new tab --cwd \"C:/Project Dir\" --set-environment-variables \"GREETING=hello world\" powershell \"-No Logo\""
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-No Logo"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert_eq!(
            launch.environment().get("GREETING"),
            Some(&"hello world".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_action_name_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "spawncommandinnewtab --cwd \"C:/Project Dir\" --env \"GREETING=hello world\" powershell \"-No Logo\""
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-No Logo"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert_eq!(
            launch.environment().get("GREETING"),
            Some(&"hello world".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_equals_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "spawncommandinnewtab=--cwd \"C:/Project Dir\" --env \"GREETING=hello world\" powershell \"-No Logo\""
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-No Logo"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert_eq!(
            launch.environment().get("GREETING"),
            Some(&"hello world".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_mixed_case_spawn_command_in_new_tab_action_name_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "SpawnCommandInNewTab --CWD \"C:/Project Dir\" --env \"GREETING=hello world\" powershell \"-No Logo\""
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-No Logo"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert_eq!(
            launch.environment().get("GREETING"),
            Some(&"hello world".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_wezterm_action_comment_before_dot_spawn_command_table_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action -- action namespace\n .SpawnCommandInNewTab { args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_wezterm_action_spawn_command_comment_before_table_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SpawnCommandInNewTab -- spawn options\n { args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_args_assignment_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("spawncommandinnewtab args=top -d 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_cwd_assignment_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "spawncommandinnewtab cwd=\"C:/Project Dir\" args=top -d 1".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_environment_assignment_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "spawncommandinnewtab set_environment_variables=\"GREETING=hello world\" args=top -d 1"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(
            launch.environment().get("GREETING"),
            Some(&"hello world".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_domain_assignment_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("spawncommandinnewtab domain=local args=top -d 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_table_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "SpawnCommandInNewTab={ cwd = \"C:/Project Dir\", args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_table_call_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "SpawnCommandInNewTab { cwd = \"C:/Project Dir\", args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_wezterm_action_table_call_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SpawnCommandInNewTab { cwd = \"C:/Project Dir\", args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_wezterm_action_spawn_command_trailing_table_call_comment_query()
     {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SpawnCommandInNewTab { cwd = \"C:/Project Dir\", args = { \"top\", \"-d\", \"1\" } } -- spawn options"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_domain_name_table_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SpawnCommandInNewTab { domain = { DomainName = \"local\" }, args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_label_table_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SpawnCommandInNewTab { label = \"System Monitor\", cwd = \"C:/Project Dir\", args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_options_label_table_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SpawnCommandInNewTab { label = \"Project Shell\", cwd = \"C:/Project Dir\" }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-NoProfile"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_wezterm_action_parenthesized_table_query()
     {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SpawnCommandInNewTab({ cwd = \"C:/Project Dir\", args = { \"top\", \"-d\", \"1\" } })"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_wezterm_action_spawn_command_comment_before_parenthesized_table_query()
     {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SpawnCommandInNewTab -- spawn options\n ({ cwd = \"C:/Project Dir\", args = { \"top\", \"-d\", \"1\" } })"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_wezterm_action_spawn_command_comment_inside_parenthesized_table_query()
     {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SpawnCommandInNewTab( -- spawn options\n { cwd = \"C:/Project Dir\", args = { \"top\", \"-d\", \"1\" } })"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_wezterm_action_spawn_command_comment_before_call_close_query()
    {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SpawnCommandInNewTab({ cwd = \"C:/Project Dir\", args = { \"top\", \"-d\", \"1\" } } -- spawn options\n)"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_wezterm_action_spawn_command_trailing_call_comment_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SpawnCommandInNewTab({ cwd = \"C:/Project Dir\", args = { \"top\", \"-d\", \"1\" } }) -- spawn options"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_mixed_case_wezterm_action_table_call_query()
     {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "WezTerm.action.SpawnCommandInNewTab { cwd = \"C:/Project Dir\", args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_table_semicolon_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "SpawnCommandInNewTab={ cwd = \"C:/Project Dir\"; args = { \"top\"; \"-d\"; \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_table_indexed_args_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "SpawnCommandInNewTab={ args = { [1] = \"top\", [2] = \"-d\", [3] = \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_table_bracket_option_key_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "SpawnCommandInNewTab={ [\"cwd\"] = \"C:/Project Dir\", [\"args\"] = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_table_environment_bracket_key_query()
    {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "SpawnCommandInNewTab={ set_environment_variables = { [\"MODE\"] = \"dev\" }, args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.environment().get("MODE"), Some(&"dev".to_owned()));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_spaced_table_assignment_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "SpawnCommandInNewTab = { cwd = \"C:/Project Dir\", args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_comment_before_table_assignment_query()
     {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "SpawnCommandInNewTab -- spawn options\n = { cwd = \"C:/Project Dir\", args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_table_options_without_program_query()
    {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "SpawnCommandInNewTab={ cwd = \"C:/Project Dir\" }".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-NoProfile"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_tab_bracket_options_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "SpawnCommandInNewTab={ [\"cwd\"] = \"C:/Project Dir\" }".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-NoProfile"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_rejects_palette_new_tab_spawn_command_remote_domain_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("new tab --domain remote.example top".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        assert!(!app.command_palette_execute(WindowCommand::NewTab));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert!(app.command_palette.is_some());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_window_command() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::SpawnWindow);

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.app_shell.pending_windows().len(), 1);
        let pending_window = app
            .app_shell
            .pending_windows()
            .first()
            .expect("spawn window should request a pending window");
        assert_eq!(pending_window.id(), rterm_core::WindowId::new(2));
        assert_eq!(pending_window.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(pending_window.active_pane_id(), rterm_core::PaneId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_prefer_to_spawn_tabs_routes_palette_spawn_window_to_new_tab() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.set_config_overrides(native_config_snapshot! {
            prefer_to_spawn_tabs: Some(true),
            ..NativeConfigSnapshot::default()
        });

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(app.app_shell.pending_windows().len(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_prefer_to_spawn_tabs_preserves_positioned_spawn_window() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.set_config_overrides(native_config_snapshot! {
            prefer_to_spawn_tabs: Some(true),
            ..NativeConfigSnapshot::default()
        });

        app.enter_command_palette_mode();
        app.command_palette_set_query("spawn window --position Main:42,84 top".to_owned());
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("positioned spawn window should remain a detached window");
        assert_eq!(
            detached_app.initial_window_position(),
            Some(crate::cli::WindowPosition {
                origin: crate::cli::WindowPositionOrigin::Main,
                x: 42,
                y: 84,
            })
        );
        assert_eq!(
            detached_app.app_shell.active_pane().launch().program(),
            "top"
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_window_command_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("spawn window top -d 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );

        app.command_palette_execute(WindowCommand::SpawnWindow);

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.app_shell.pending_windows().len(), 1);
        let pending_window = app
            .app_shell
            .pending_windows()
            .first()
            .expect("spawn window should request a pending window");
        let launch = pending_window.tab().panes()[0].launch();
        assert_eq!(pending_window.id(), rterm_core::WindowId::new(2));
        assert_eq!(pending_window.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(pending_window.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_window_command_equals_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("spawn window=top -d 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );

        app.command_palette_execute(WindowCommand::SpawnWindow);

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.app_shell.pending_windows().len(), 1);
        let pending_window = app
            .app_shell
            .pending_windows()
            .first()
            .expect("spawn window should request a pending window");
        let launch = pending_window.tab().panes()[0].launch();
        assert_eq!(pending_window.id(), rterm_core::WindowId::new(2));
        assert_eq!(pending_window.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(pending_window.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_new_window_command_equals_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("new window=htop --tree".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );

        app.command_palette_execute(WindowCommand::SpawnWindow);

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.app_shell.pending_windows().len(), 1);
        let pending_window = app
            .app_shell
            .pending_windows()
            .first()
            .expect("new window should request a pending window");
        let launch = pending_window.tab().panes()[0].launch();
        assert_eq!(pending_window.id(), rterm_core::WindowId::new(2));
        assert_eq!(pending_window.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(pending_window.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(launch.program(), "htop");
        assert_eq!(launch.args(), ["--tree"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_applies_palette_spawn_window_position_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("spawn window --position Main:42,84 top".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn window query should request a pending detached window");
        assert_eq!(
            detached_app.initial_window_position(),
            Some(crate::cli::WindowPosition {
                origin: crate::cli::WindowPositionOrigin::Main,
                x: 42,
                y: 84,
            })
        );
        assert_eq!(
            detached_app.app_shell.active_pane().launch().program(),
            "top"
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_applies_palette_spawn_window_position_without_program_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("spawn window --position main:42,84".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn window query should request a pending detached window");
        assert_eq!(
            detached_app.initial_window_position(),
            Some(crate::cli::WindowPosition {
                origin: crate::cli::WindowPositionOrigin::Main,
                x: 42,
                y: 84,
            })
        );
        let launch = detached_app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-NoProfile"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_applies_palette_spawn_window_options_without_program_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "spawn window --domain local --cwd \"C:/Spawn Dir\" --env \"SPAWN_MODE=query mode\" --position main:42,84"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn window query should request a pending detached window");
        assert_eq!(
            detached_app.initial_window_position(),
            Some(crate::cli::WindowPosition {
                origin: crate::cli::WindowPositionOrigin::Main,
                x: 42,
                y: 84,
            })
        );
        let launch = detached_app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-NoProfile"]);
        assert_eq!(launch.cwd(), Some("C:/Spawn Dir"));
        assert_eq!(
            launch.environment().get("SPAWN_MODE"),
            Some(&"query mode".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_window_action_name_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "spawncommandinnewwindow --cwd \"C:/Spawn Dir\" --env \"SPAWN_MODE=query mode\" --position main:42,84 powershell \"-No Logo\""
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn command query should request a pending detached window");
        assert_eq!(
            detached_app.initial_window_position(),
            Some(crate::cli::WindowPosition {
                origin: crate::cli::WindowPositionOrigin::Main,
                x: 42,
                y: 84,
            })
        );
        let launch = detached_app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-No Logo"]);
        assert_eq!(launch.cwd(), Some("C:/Spawn Dir"));
        assert_eq!(
            launch.environment().get("SPAWN_MODE"),
            Some(&"query mode".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_window_equals_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "spawncommandinnewwindow=--cwd \"C:/Spawn Dir\" --env \"SPAWN_MODE=query mode\" --position main:42,84 powershell \"-No Logo\""
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn command query should request a pending detached window");
        assert_eq!(
            detached_app.initial_window_position(),
            Some(crate::cli::WindowPosition {
                origin: crate::cli::WindowPositionOrigin::Main,
                x: 42,
                y: 84,
            })
        );
        let launch = detached_app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-No Logo"]);
        assert_eq!(launch.cwd(), Some("C:/Spawn Dir"));
        assert_eq!(
            launch.environment().get("SPAWN_MODE"),
            Some(&"query mode".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_window_table_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "SpawnCommandInNewWindow={ position = \"main:42,84\", cwd = \"C:/Spawn Dir\", args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn command table query should request a pending detached window");
        assert_eq!(
            detached_app.initial_window_position(),
            Some(crate::cli::WindowPosition {
                origin: crate::cli::WindowPositionOrigin::Main,
                x: 42,
                y: 84,
            })
        );
        let launch = detached_app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Spawn Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_window_wezterm_action_table_call_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SpawnCommandInNewWindow { position = \"main:42,84\", cwd = \"C:/Spawn Dir\", args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn command table query should request a pending detached window");
        assert_eq!(
            detached_app.initial_window_position(),
            Some(crate::cli::WindowPosition {
                origin: crate::cli::WindowPositionOrigin::Main,
                x: 42,
                y: 84,
            })
        );
        let launch = detached_app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Spawn Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_window_position_table_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SpawnCommandInNewWindow { position = { x = 10, y = 300, origin = { Named = \"HDMI-1\" } }, args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn command table position query should request a pending detached window");
        assert_eq!(
            detached_app.initial_window_position(),
            Some(crate::cli::WindowPosition {
                origin: crate::cli::WindowPositionOrigin::Monitor("HDMI-1".to_owned()),
                x: 10,
                y: 300,
            })
        );
        let launch = detached_app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_window_position_long_bracket_key_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SpawnCommandInNewWindow { position = { [[=[x]=]] = 10, [[=[y]=]] = 300, [[=[origin]=]] = { [[=[Named]=]] = [[HDMI-1]] } }, args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn command table position query should request a pending detached window");
        assert_eq!(
            detached_app.initial_window_position(),
            Some(crate::cli::WindowPosition {
                origin: crate::cli::WindowPositionOrigin::Monitor("HDMI-1".to_owned()),
                x: 10,
                y: 300,
            })
        );
        let launch = detached_app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_window_position_table_default_origin_query()
     {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SpawnCommandInNewWindow { position = { x = 12, y = 34 }, args = { \"top\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn command table position query should request a pending detached window");
        assert_eq!(
            detached_app.initial_window_position(),
            Some(crate::cli::WindowPosition {
                origin: crate::cli::WindowPositionOrigin::Screen,
                x: 12,
                y: 34,
            })
        );
        let launch = detached_app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "top");
        assert!(launch.args().is_empty());
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_window_wezterm_action_parenthesized_table_query()
     {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SpawnCommandInNewWindow({ position = \"main:42,84\", cwd = \"C:/Spawn Dir\", args = { \"top\", \"-d\", \"1\" } })"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn command table query should request a pending detached window");
        assert_eq!(
            detached_app.initial_window_position(),
            Some(crate::cli::WindowPosition {
                origin: crate::cli::WindowPositionOrigin::Main,
                x: 42,
                y: 84,
            })
        );
        let launch = detached_app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Spawn Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_window_table_options_without_program_query()
     {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "SpawnCommandInNewWindow={ position = \"main:42,84\" }".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn command table options query should request a pending detached window");
        assert_eq!(
            detached_app.initial_window_position(),
            Some(crate::cli::WindowPosition {
                origin: crate::cli::WindowPositionOrigin::Main,
                x: 42,
                y: 84,
            })
        );
        let launch = detached_app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-NoProfile"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_window_position_table_without_program_query()
     {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "SpawnCommandInNewWindow={ position = { x = 42, y = 84, origin = \"MainScreen\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn command table options query should request a pending detached window");
        assert_eq!(
            detached_app.initial_window_position(),
            Some(crate::cli::WindowPosition {
                origin: crate::cli::WindowPositionOrigin::Main,
                x: 42,
                y: 84,
            })
        );
        let launch = detached_app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-NoProfile"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_spawn_command_in_new_window_position_assignment_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "spawncommandinnewwindow position=main:42,84 args=top -d 1".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn command query should request a pending detached window");
        assert_eq!(
            detached_app.initial_window_position(),
            Some(crate::cli::WindowPosition {
                origin: crate::cli::WindowPositionOrigin::Main,
                x: 42,
                y: 84,
            })
        );
        let launch = detached_app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_applies_palette_spawn_command_in_new_window_position_assignment_without_program_query()
     {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("spawncommandinnewwindow position=main:42,84".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn command query should request a pending detached window");
        assert_eq!(
            detached_app.initial_window_position(),
            Some(crate::cli::WindowPosition {
                origin: crate::cli::WindowPositionOrigin::Main,
                x: 42,
                y: 84,
            })
        );
        let launch = detached_app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-NoProfile"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_applies_palette_spawn_command_in_new_window_cwd_assignment_without_program_query()
    {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("spawncommandinnewwindow cwd=\"C:/Spawn Dir\"".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn command query should request a pending detached window");
        let launch = detached_app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-NoProfile"]);
        assert_eq!(launch.cwd(), Some("C:/Spawn Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_applies_palette_spawn_command_in_new_window_environment_assignment_without_program_query()
     {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "spawncommandinnewwindow set_environment_variables=\"GREETING=hello world\"".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn command query should request a pending detached window");
        let launch = detached_app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-NoProfile"]);
        assert_eq!(
            launch.environment().get("GREETING"),
            Some(&"hello world".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_applies_palette_spawn_command_in_new_window_domain_assignment_without_program_query()
     {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("spawncommandinnewwindow domain=local".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SpawnWindow]
        );
        assert!(app.command_palette_execute(WindowCommand::SpawnWindow));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn command query should request a pending detached window");
        let launch = detached_app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-NoProfile"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_spawn_command_in_new_tab_action_payload() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::SpawnCommandInNewTab(
            WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: Some("/tmp/project".to_owned()),
                environment: BTreeMap::from([("SPAWN_MODE".to_owned(), "native".to_owned())]),
                domain: None,
                window_position: None,
            },
        ));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("/tmp/project"));
        assert_eq!(
            launch.environment().get("SPAWN_MODE"),
            Some(&"native".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_spawn_command_local_domain_payload() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        assert!(
            app.command_palette_execute(WindowCommand::SpawnCommandInNewTab(
                WindowSpawnCommandQuery {
                    label: None,
                    program: "top".to_owned(),
                    args: vec!["-d".to_owned(), "1".to_owned()],
                    cwd: None,
                    environment: BTreeMap::new(),
                    domain: Some(WindowSpawnTabDomain::DomainName("local".to_owned())),
                    window_position: None,
                },
            ))
        );

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_spawn_command_in_new_window_action_payload() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::SpawnCommandInNewWindow(
            WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: Some("/tmp/project".to_owned()),
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            },
        ));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.app_shell.pending_windows().len(), 1);
        let pending_window = app
            .app_shell
            .pending_windows()
            .first()
            .expect("spawn window should request a pending window");
        let launch = pending_window.tab().panes()[0].launch();
        assert_eq!(pending_window.id(), rterm_core::WindowId::new(2));
        assert_eq!(pending_window.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(pending_window.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("/tmp/project"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_applies_native_spawn_command_new_window_position_payload() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        let position = crate::cli::WindowPosition {
            origin: crate::cli::WindowPositionOrigin::Main,
            x: 42,
            y: 84,
        };

        app.enter_command_palette_mode();
        assert!(
            app.command_palette_execute(WindowCommand::SpawnCommandInNewWindow(
                WindowSpawnCommandQuery {
                    label: None,
                    program: "top".to_owned(),
                    args: Vec::new(),
                    cwd: None,
                    environment: BTreeMap::new(),
                    domain: None,
                    window_position: Some(position.clone()),
                },
            ))
        );

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn command should request a pending detached window");
        assert_eq!(detached_app.initial_window_position(), Some(position));
    }

    #[test]
    fn window_app_dispatches_palette_toggle_full_screen_command() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ToggleFullScreen);

        assert!(app.full_screen_for_test());
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ToggleFullScreen);

        assert!(!app.full_screen_for_test());
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_hide_command() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::Hide);

        assert!(app.window_hide_requested_for_test());
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_start_window_drag_command() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::StartWindowDrag);

        assert!(app.window_drag_requested_for_test());
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_activate_window_action_payloads() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::ActivateWindow(2)));

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Index(2))
        );
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::ActivateWindowRelative(1)));

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Relative {
                offset: 1,
                wrap: true,
            })
        );
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::ActivateWindowRelativeNoWrap(-1)));

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Relative {
                offset: -1,
                wrap: false,
            })
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate window 2".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindow(2)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Index(2))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate window=2".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindow(2)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Index(2))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_index_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate window index 2".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindow(2)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Index(2))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_index_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate window index=2".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindow(2)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Index(2))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_action_name_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activatewindow=2".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindow(2)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Index(2))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_index_assignment_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activatewindow index=2".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindow(2)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Index(2))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_wezterm_action_function_call_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.ActivateWindow(2)".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindow(2)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Index(2))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_relative_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate window relative -2".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindowRelative(-2)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Relative {
                offset: -2,
                wrap: true,
            })
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_relative_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate window relative=-2".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindowRelative(-2)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Relative {
                offset: -2,
                wrap: true,
            })
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_relative_action_name_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activatewindowrelative=-2".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindowRelative(-2)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Relative {
                offset: -2,
                wrap: true,
            })
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_relative_offset_assignment_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activatewindowrelative offset=-2".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindowRelative(-2)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Relative {
                offset: -2,
                wrap: true,
            })
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_relative_wezterm_action_function_call_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.ActivateWindowRelative(-2)".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindowRelative(-2)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Relative {
                offset: -2,
                wrap: true,
            })
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_relative_no_wrap_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate window relative no wrap -1".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindowRelativeNoWrap(-1)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Relative {
                offset: -1,
                wrap: false,
            })
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_relative_no_wrap_wezterm_action_function_call_query()
     {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.ActivateWindowRelativeNoWrap(-1)".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindowRelativeNoWrap(-1)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Relative {
                offset: -1,
                wrap: false,
            })
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_relative_no_wrap_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate window relative no wrap=-1".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindowRelativeNoWrap(-1)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Relative {
                offset: -1,
                wrap: false,
            })
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_relative_no_dash_wrap_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate window relative no-wrap=-1".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindowRelativeNoWrap(-1)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Relative {
                offset: -1,
                wrap: false,
            })
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_relative_nowrap_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate window relative nowrap=-1".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindowRelativeNoWrap(-1)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Relative {
                offset: -1,
                wrap: false,
            })
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_relative_nowrap_action_name_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activatewindowrelativenowrap=-1".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindowRelativeNoWrap(-1)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Relative {
                offset: -1,
                wrap: false,
            })
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_relative_no_wrap_offset_assignment_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activatewindowrelativenowrap offset=-1".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindowRelativeNoWrap(-1)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Relative {
                offset: -1,
                wrap: false,
            })
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_relative_no_dash_wrap_action_name_equals_query()
     {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activatewindowrelativeno-wrap=-1".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindowRelativeNoWrap(-1)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Relative {
                offset: -1,
                wrap: false,
            })
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_window_relative_no_space_wrap_action_name_equals_query()
     {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activatewindowrelativeno wrap=-1".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::ActivateWindowRelativeNoWrap(-1)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(
            app.take_activate_window_request_for_test(),
            Some(WindowActivateWindowRequest::Relative {
                offset: -1,
                wrap: false,
            })
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_activate_relative_index_wraps_and_nowrap_stops_at_edges() {
        assert_eq!(activate_window_relative_index(0, 3, 1, true), Some(1));
        assert_eq!(activate_window_relative_index(2, 3, 1, true), Some(0));
        assert_eq!(activate_window_relative_index(0, 3, -1, true), Some(2));
        assert_eq!(activate_window_relative_index(0, 3, -1, false), None);
        assert_eq!(activate_window_relative_index(2, 3, 1, false), None);
        assert_eq!(activate_window_relative_index(1, 3, 0, true), Some(1));
        assert_eq!(activate_window_relative_index(1, 0, 1, true), None);
    }

    #[test]
    fn window_activate_absolute_index_uses_zero_based_gui_window_order() {
        assert_eq!(activate_window_absolute_index(0, 3), Some(0));
        assert_eq!(activate_window_absolute_index(2, 3), Some(2));
        assert_eq!(activate_window_absolute_index(3, 3), None);
        assert_eq!(activate_window_absolute_index(0, 0), None);
    }

    #[test]
    fn pending_window_batch_focuses_only_the_last_materialized_window() {
        assert!(!should_focus_materialized_window(0, 3));
        assert!(!should_focus_materialized_window(1, 3));
        assert!(should_focus_materialized_window(2, 3));
        assert!(!should_focus_materialized_window(0, 0));
    }

    #[test]
    fn window_app_start_window_drag_default_mouse_bindings_wait_for_drag() {
        let mut super_app = NativeWindowApp::new(None);
        super_app.window_focused = true;
        super_app.modifiers = ModifiersState::SUPER;
        let terminal_y = f64::from(super_app.terminal_pixel_top()) + 1.0;
        super_app
            .handle_cursor_moved(PhysicalPosition::new(1.0, terminal_y))
            .unwrap();
        assert!(
            !super_app
                .handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert!(!super_app.window_drag_requested_for_test());

        assert!(
            super_app
                .handle_cursor_moved(PhysicalPosition::new(
                    f64::from(CELL_WIDTH) + 1.0,
                    terminal_y
                ))
                .unwrap()
        );
        assert!(super_app.window_drag_requested_for_test());

        let mut ctrl_shift_app = NativeWindowApp::new(None);
        ctrl_shift_app.window_focused = true;
        ctrl_shift_app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        ctrl_shift_app
            .handle_cursor_moved(PhysicalPosition::new(1.0, terminal_y))
            .unwrap();
        assert!(
            !ctrl_shift_app
                .handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert!(!ctrl_shift_app.window_drag_requested_for_test());

        assert!(
            ctrl_shift_app
                .handle_cursor_moved(PhysicalPosition::new(
                    f64::from(CELL_WIDTH) + 1.0,
                    terminal_y
                ))
                .unwrap()
        );
        assert!(ctrl_shift_app.window_drag_requested_for_test());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_start_window_drag_mouse_binding() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.mouse_bindings = {
              {
                event = { Drag = { streak = 1, button = 'Left' } },
                mods = 'ALT',
                action = act.StartWindowDrag,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding config");
        app.set_config_overrides(overrides);
        app.modifiers = ModifiersState::ALT;
        let terminal_y = f64::from(TAB_BAR_ROWS) * f64::from(CELL_HEIGHT) + 1.0;

        app.handle_cursor_moved(PhysicalPosition::new(1.0, terminal_y))
            .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH) + 1.0,
            terminal_y,
        ))
        .unwrap();

        assert!(app.window_drag_requested_for_test());
        assert!(app.selection.is_none());
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_custom_mouse_drag_clears_stable_ordinary_selection() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"ordinary").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 3 },
        );
        app.modifiers = ModifiersState::ALT;
        app.mouse_assignments = vec![NativeUserMouseAssignment {
            event: NativeMouseAssignmentEvent {
                kind: NativeMouseAssignmentEventKind::Drag,
                button: NativeMouseAssignmentButton::Mouse(MouseButton::Left),
                streak: 1,
            },
            modifiers: ModifiersState::ALT,
            mouse_reporting: false,
            alt_screen: NativeMouseAssignmentAltScreen::Any,
            command: WindowCommand::StartWindowDrag,
        }];

        assert!(app.handle_user_mouse_assignment(
            NativeMouseAssignmentEventKind::Drag,
            NativeMouseAssignmentButton::Mouse(MouseButton::Left),
            1,
            false,
            false,
        ));
        app.refresh_snapshot();

        assert!(ordinary_selection_for_test(&app).is_none());
        assert!(app.selection.is_none());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_mouse_binding_static_action_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local drag_window = act.StartWindowDrag

            config.mouse_bindings = {
              {
                event = { Drag = { streak = 1, button = 'Left' } },
                mods = 'ALT',
                action = drag_window,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding static action variable config");
        app.set_config_overrides(overrides);
        app.modifiers = ModifiersState::ALT;
        let terminal_y = f64::from(TAB_BAR_ROWS) * f64::from(CELL_HEIGHT) + 1.0;

        app.handle_cursor_moved(PhysicalPosition::new(1.0, terminal_y))
            .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH) + 1.0,
            terminal_y,
        ))
        .unwrap();

        assert!(app.window_drag_requested_for_test());
        assert!(app.selection.is_none());
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_mouse_binding_static_event_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local drag_event = { Drag = { streak = 1, button = 'Left' } }

            config.mouse_bindings = {
              {
                event = drag_event,
                mods = 'ALT',
                action = act.StartWindowDrag,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding static event variable config");
        app.set_config_overrides(overrides);
        app.modifiers = ModifiersState::ALT;
        let terminal_y = f64::from(TAB_BAR_ROWS) * f64::from(CELL_HEIGHT) + 1.0;

        app.handle_cursor_moved(PhysicalPosition::new(1.0, terminal_y))
            .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH) + 1.0,
            terminal_y,
        ))
        .unwrap();

        assert!(app.window_drag_requested_for_test());
        assert!(app.selection.is_none());
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_mouse_binding_static_event_kind_field_name() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local drag_field = 'Drag'

            config.mouse_bindings = {
              {
                event = { [drag_field] = { streak = 1, button = 'Left' } },
                mods = 'ALT',
                action = act.StartWindowDrag,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding static event kind field-name config");

        assert_eq!(
            overrides.mouse_assignments,
            Some(vec![NativeUserMouseAssignment {
                event: NativeMouseAssignmentEvent {
                    kind: NativeMouseAssignmentEventKind::Drag,
                    button: NativeMouseAssignmentButton::Mouse(MouseButton::Left),
                    streak: 1,
                },
                modifiers: ModifiersState::ALT,
                mouse_reporting: false,
                alt_screen: NativeMouseAssignmentAltScreen::Any,
                command: WindowCommand::StartWindowDrag,
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_mouse_binding_static_event_payload_fields() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local drag_button = 'Left'
            local drag_streak = 1

            config.mouse_bindings = {
              {
                event = { Drag = { streak = drag_streak, button = drag_button } },
                mods = 'ALT',
                action = act.StartWindowDrag,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding static event payload field config");
        app.set_config_overrides(overrides);
        app.modifiers = ModifiersState::ALT;
        let terminal_y = f64::from(TAB_BAR_ROWS) * f64::from(CELL_HEIGHT) + 1.0;

        app.handle_cursor_moved(PhysicalPosition::new(1.0, terminal_y))
            .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH) + 1.0,
            terminal_y,
        ))
        .unwrap();

        assert!(app.window_drag_requested_for_test());
        assert!(app.selection.is_none());
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_mouse_binding_static_event_payload_field_names() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local button_field = 'button'
            local streak_field = 'streak'

            config.mouse_bindings = {
              {
                event = { Drag = { [streak_field] = 1, [button_field] = 'Left' } },
                mods = 'ALT',
                action = act.StartWindowDrag,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding static event payload field-name config");

        assert_eq!(
            overrides.mouse_assignments,
            Some(vec![NativeUserMouseAssignment {
                event: NativeMouseAssignmentEvent {
                    kind: NativeMouseAssignmentEventKind::Drag,
                    button: NativeMouseAssignmentButton::Mouse(MouseButton::Left),
                    streak: 1,
                },
                modifiers: ModifiersState::ALT,
                mouse_reporting: false,
                alt_screen: NativeMouseAssignmentAltScreen::Any,
                command: WindowCommand::StartWindowDrag,
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_mouse_binding_static_event_payload_table_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local drag_payload = {
              streak = 1,
              button = 'Left',
            }

            config.mouse_bindings = {
              {
                event = { Drag = drag_payload },
                mods = 'ALT',
                action = act.StartWindowDrag,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding static event payload table config");
        app.set_config_overrides(overrides);
        app.modifiers = ModifiersState::ALT;
        let terminal_y = f64::from(TAB_BAR_ROWS) * f64::from(CELL_HEIGHT) + 1.0;

        app.handle_cursor_moved(PhysicalPosition::new(1.0, terminal_y))
            .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH) + 1.0,
            terminal_y,
        ))
        .unwrap();

        assert!(app.window_drag_requested_for_test());
        assert!(app.selection.is_none());
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_mouse_binding_static_mods_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local drag_mods = 'ALT'

            config.mouse_bindings = {
              {
                event = { Drag = { streak = 1, button = 'Left' } },
                mods = drag_mods,
                action = act.StartWindowDrag,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding static mods variable config");
        app.set_config_overrides(overrides);
        app.modifiers = ModifiersState::ALT;
        let terminal_y = f64::from(TAB_BAR_ROWS) * f64::from(CELL_HEIGHT) + 1.0;

        app.handle_cursor_moved(PhysicalPosition::new(1.0, terminal_y))
            .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH) + 1.0,
            terminal_y,
        ))
        .unwrap();

        assert!(app.window_drag_requested_for_test());
        assert!(app.selection.is_none());
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_mouse_binding_static_mouse_reporting_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local report_mouse = false

            config.mouse_bindings = {
              {
                event = { Drag = { streak = 1, button = 'Left' } },
                mods = 'ALT',
                mouse_reporting = report_mouse,
                action = act.StartWindowDrag,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding static mouse_reporting variable config");
        app.set_config_overrides(overrides);
        app.modifiers = ModifiersState::ALT;
        let terminal_y = f64::from(TAB_BAR_ROWS) * f64::from(CELL_HEIGHT) + 1.0;

        app.handle_cursor_moved(PhysicalPosition::new(1.0, terminal_y))
            .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH) + 1.0,
            terminal_y,
        ))
        .unwrap();

        assert!(app.window_drag_requested_for_test());
        assert!(app.selection.is_none());
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_mouse_binding_static_alt_screen_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local normal_screen = false

            config.mouse_bindings = {
              {
                event = { Drag = { streak = 1, button = 'Left' } },
                mods = 'ALT',
                alt_screen = normal_screen,
                action = act.StartWindowDrag,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding static alt_screen variable config");
        app.set_config_overrides(overrides);
        app.modifiers = ModifiersState::ALT;
        let terminal_y = f64::from(TAB_BAR_ROWS) * f64::from(CELL_HEIGHT) + 1.0;

        app.handle_cursor_moved(PhysicalPosition::new(1.0, terminal_y))
            .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH) + 1.0,
            terminal_y,
        ))
        .unwrap();

        assert!(app.window_drag_requested_for_test());
        assert!(app.selection.is_none());
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_mouse_binding_static_field_variable_item() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local binding = {}
            binding.event = { Drag = { streak = 1, button = 'Left' } }
            binding.mods = 'ALT'
            binding.action = act.StartWindowDrag

            config.mouse_bindings = { binding }

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding field-built item variable config");
        app.set_config_overrides(overrides);
        app.modifiers = ModifiersState::ALT;
        let terminal_y = f64::from(TAB_BAR_ROWS) * f64::from(CELL_HEIGHT) + 1.0;

        app.handle_cursor_moved(PhysicalPosition::new(1.0, terminal_y))
            .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH) + 1.0,
            terminal_y,
        ))
        .unwrap();

        assert!(app.window_drag_requested_for_test());
        assert!(app.selection.is_none());
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_mouse_binding_static_field_name_variables() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local event_field = 'event'
            local mods_field = 'mods'
            local action_field = 'action'

            config.mouse_bindings = {
              {
                [event_field] = { Drag = { streak = 1, button = 'Left' } },
                [mods_field] = 'ALT',
                [action_field] = act.StartWindowDrag,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding static field-name variable config");
        app.set_config_overrides(overrides);
        app.modifiers = ModifiersState::ALT;
        let terminal_y = f64::from(TAB_BAR_ROWS) * f64::from(CELL_HEIGHT) + 1.0;

        app.handle_cursor_moved(PhysicalPosition::new(1.0, terminal_y))
            .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH) + 1.0,
            terminal_y,
        ))
        .unwrap();

        assert!(app.window_drag_requested_for_test());
        assert!(app.selection.is_none());
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_mouse_binding_table_insert_entries() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.mouse_bindings = {}
            table.insert(config.mouse_bindings, {
              event = { Drag = { streak = 1, button = 'Left' } },
              mods = 'ALT',
              action = act.StartWindowDrag,
            })

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding table.insert config");
        app.set_config_overrides(overrides);
        app.modifiers = ModifiersState::ALT;
        let terminal_y = f64::from(TAB_BAR_ROWS) * f64::from(CELL_HEIGHT) + 1.0;

        app.handle_cursor_moved(PhysicalPosition::new(1.0, terminal_y))
            .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH) + 1.0,
            terminal_y,
        ))
        .unwrap();

        assert!(app.window_drag_requested_for_test());
        assert!(app.selection.is_none());
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_middle_click_pastes_primary_selection_by_default() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.primary_selection_reader = Box::new(|| Some("primary\ntext".to_owned()));
        // The initial cursor is in the tab bar, where middle-click closes tabs.
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH),
            f64::from(tab_bar_pixel_height()) + 1.0,
        ))
        .unwrap();

        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Middle)
                .unwrap()
        );

        let expected =
            encode_window_paste("primary\ntext", false, DEFAULT_CANONICALIZE_PASTED_NEWLINES);
        assert_eq!(written.lock().unwrap().as_slice(), expected.as_slice());
    }

    #[test]
    fn window_app_mouse_disable_default_assignment_suppresses_default_without_consuming() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.primary_selection_reader = Box::new(|| Some("primary\ntext".to_owned()));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.mouse_bindings = {
              {
                event = { Down = { streak = 1, button = 'Middle' } },
                mods = 'NONE',
                action = act.DisableDefaultAssignment,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm DisableDefaultAssignment mouse binding config");
        app.set_config_overrides(overrides);

        assert!(
            !app.handle_mouse_input(ElementState::Pressed, MouseButton::Middle)
                .unwrap()
        );

        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_middle_click_reports_to_pane_when_mouse_reporting_enabled() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.primary_selection_reader = Box::new(|| Some("primary".to_owned()));
        app.handle_pty_output(b"\x1b[?1000;1006h").unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();

        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Middle)
                .unwrap()
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b[<1;2;1M");
    }

    #[test]
    fn window_app_user_middle_up_mouse_binding_suppresses_default_middle_paste() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.primary_selection_reader = Box::new(|| Some("primary\ntext".to_owned()));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.mouse_bindings = {
              {
                event = { Up = { streak = 1, button = 'Middle' } },
                mods = 'NONE',
                action = act.PastePrimarySelection,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding config");
        app.set_config_overrides(overrides);

        let _ = app.handle_mouse_input(ElementState::Pressed, MouseButton::Middle);
        assert!(
            written.lock().unwrap().is_empty(),
            "press should not run the default middle-click paste when a user binding covers the button"
        );

        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Middle)
                .unwrap()
        );

        let expected =
            encode_window_paste("primary\ntext", false, DEFAULT_CANONICALIZE_PASTED_NEWLINES);
        assert_eq!(written.lock().unwrap().as_slice(), expected.as_slice());
    }

    #[test]
    fn window_app_mouse_binding_mouse_reporting_true_matches_reporting_mode() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.primary_selection_reader = Box::new(|| Some("primary".to_owned()));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.mouse_bindings = {
              {
                event = { Down = { streak = 1, button = 'Middle' } },
                mods = 'NONE',
                mouse_reporting = true,
                action = act.PastePrimarySelection,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b[?1000;1006h").unwrap();

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Middle)
                .unwrap()
        );

        let expected = encode_window_paste("primary", false, DEFAULT_CANONICALIZE_PASTED_NEWLINES);
        assert_eq!(written.lock().unwrap().as_slice(), expected.as_slice());
    }

    #[test]
    fn window_app_drag_mouse_binding_honors_mouse_reporting_bypass_modifier() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.mouse_bindings = {
              {
                event = { Drag = { streak = 1, button = 'Left' } },
                mods = 'SHIFT',
                mouse_reporting = false,
                action = act.StartWindowDrag,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm drag mouse binding config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b[?1000;1006h").unwrap();
        app.modifiers = ModifiersState::SHIFT;

        let terminal_y = f64::from(tab_bar_pixel_height()) + 1.0;
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH) + 1.0,
            terminal_y,
        ))
        .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap();
        assert!(!app.window_drag_requested_for_test());

        assert!(
            app.handle_cursor_moved(PhysicalPosition::new(
                f64::from(CELL_WIDTH) * 2.0 + 1.0,
                terminal_y,
            ))
            .unwrap()
        );

        assert!(app.window_drag_requested_for_test());
        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_mouse_binding_alt_screen_true_matches_only_alternate_screen() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.primary_selection_reader = Box::new(|| Some("primary".to_owned()));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.mouse_bindings = {
              {
                event = { Down = { streak = 1, button = 'Right' } },
                mods = 'NONE',
                alt_screen = true,
                action = act.PastePrimarySelection,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding config");
        app.set_config_overrides(overrides);

        let _ = app.handle_mouse_input(ElementState::Pressed, MouseButton::Right);
        assert!(written.lock().unwrap().is_empty());

        app.handle_mouse_input(ElementState::Released, MouseButton::Right)
            .unwrap();
        app.handle_pty_output(b"\x1b[?1049h").unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Right)
                .unwrap()
        );

        let expected = encode_window_paste("primary", false, DEFAULT_CANONICALIZE_PASTED_NEWLINES);
        assert_eq!(written.lock().unwrap().as_slice(), expected.as_slice());
    }

    #[test]
    fn window_app_mouse_binding_wheel_up_increases_font_size() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.mouse_bindings = {
              {
                event = { Down = { streak = 1, button = { WheelUp = 1 } } },
                mods = 'CTRL',
                action = act.IncreaseFontSize,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm wheel mouse binding config");
        app.set_config_overrides(overrides);
        app.modifiers = ModifiersState::CONTROL;
        let active = app.active_pane_id();
        move_wheel_to_pane_cell_for_test(&mut app, active, 0, 0, 1.0, 1.0);

        assert!((app.font_size_scale_for_test() - 1.0).abs() < f64::EPSILON);
        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert!((app.font_size_scale_for_test() - 1.1).abs() < f64::EPSILON);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_mouse_binding_static_wheel_button_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local wheel_button = { WheelUp = 1 }

            config.mouse_bindings = {
              {
                event = { Down = { streak = 1, button = wheel_button } },
                mods = 'CTRL',
                action = act.IncreaseFontSize,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding static wheel button config");
        app.set_config_overrides(overrides);
        app.modifiers = ModifiersState::CONTROL;
        let active = app.active_pane_id();
        move_wheel_to_pane_cell_for_test(&mut app, active, 0, 0, 1.0, 1.0);

        assert!((app.font_size_scale_for_test() - 1.0).abs() < f64::EPSILON);
        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert!((app.font_size_scale_for_test() - 1.1).abs() < f64::EPSILON);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_mouse_binding_static_wheel_button_field_name() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local wheel_field = 'WheelUp'

            config.mouse_bindings = {
              {
                event = { Down = { streak = 1, button = { [wheel_field] = 1 } } },
                mods = 'CTRL',
                action = act.IncreaseFontSize,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding static wheel button field-name config");

        assert_eq!(
            overrides.mouse_assignments,
            Some(vec![NativeUserMouseAssignment {
                event: NativeMouseAssignmentEvent {
                    kind: NativeMouseAssignmentEventKind::Down,
                    button: NativeMouseAssignmentButton::WheelUp,
                    streak: 1,
                },
                modifiers: ModifiersState::CONTROL,
                mouse_reporting: false,
                alt_screen: NativeMouseAssignmentAltScreen::Any,
                command: WindowCommand::IncreaseFontSize,
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_mouse_binding_static_wheel_button_amount_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local wheel_amount = 1

            config.mouse_bindings = {
              {
                event = { Down = { streak = 1, button = { WheelUp = wheel_amount } } },
                mods = 'CTRL',
                action = act.IncreaseFontSize,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm mouse binding static wheel button amount config");
        app.set_config_overrides(overrides);
        app.modifiers = ModifiersState::CONTROL;
        let active = app.active_pane_id();
        move_wheel_to_pane_cell_for_test(&mut app, active, 0, 0, 1.0, 1.0);

        assert!((app.font_size_scale_for_test() - 1.0).abs() < f64::EPSILON);
        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert!((app.font_size_scale_for_test() - 1.1).abs() < f64::EPSILON);
    }

    #[test]
    fn window_app_mouse_binding_scroll_by_current_event_uses_wheel_delta() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd").unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.mouse_bindings = {
              {
                event = { Down = { streak = 1, button = { WheelUp = 1 } } },
                mods = 'CTRL',
                action = act.ScrollByCurrentEventWheelDelta,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm wheel-current-event mouse binding config");
        app.set_config_overrides(overrides);
        app.modifiers = ModifiersState::CONTROL;
        let active = app.active_pane_id();
        move_wheel_to_pane_cell_for_test(&mut app, active, 0, 0, 1.0, 1.0);

        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 2.0))
                .unwrap()
        );

        assert_eq!(app.current_scrollback_offset(), 2);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('a'));
    }
