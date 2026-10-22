    #[test]
    fn window_app_dispatches_palette_quick_select_patterns_query_with_quoted_semicolon_pattern() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
        app.handle_pty_output(b"foo ; bar baz https://default.test")
            .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("quick select patterns \"foo ; bar\" ; baz".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );

        app.command_palette_execute(WindowCommand::EnterQuickSelect);

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 2);
        assert_eq!(app.selected_text().as_deref(), Some("foo ; bar"));
        assert!(app.command_palette.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(copy_mode_for_test(&app).is_none());
    }

    #[test]
    fn window_app_does_not_treat_bare_pattern_assignment_as_quick_select() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(48, 1));
        app.handle_pty_output(b"ticket-1234 https://default.test")
            .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("pattern=ticket-[0-9]+".to_owned());

        assert_ne!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );
    }

    #[test]
    fn window_app_does_not_treat_bare_action_assignment_as_quick_select() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(48, 1));
        app.handle_pty_output(b"https://default.test").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("action=open-uri".to_owned());

        assert_ne!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );
    }

    #[test]
    fn window_app_dispatches_native_quick_select_args_action() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
        app.handle_pty_output(b"ticket-1234 https://default.test")
            .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::QuickSelect(WindowQuickSelectOptions {
            patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
            alphabet: Some("12".to_owned()),
            label: Some("open ticket".to_owned()),
            action: Some(WindowQuickSelectAction::OpenUri),
            skip_action_on_paste: true,
            scope_lines: Some(25),
        }));

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 1);
        assert_eq!(quick_select.labels.as_slice(), ["1"]);
        assert_eq!(quick_select.action_label.as_deref(), Some("open ticket"));
        assert_eq!(quick_select.action, WindowQuickSelectAction::OpenUri);
        assert!(quick_select.skip_action_on_paste);
        assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
        assert!(app.command_palette.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(copy_mode_for_test(&app).is_none());
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_args_as_wezterm_named_command() {
        let query = "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', alphabet = '12' }";
        let expected = WindowCommand::QuickSelectArgs(WindowQuickSelectOptions {
            patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
            alphabet: Some("12".to_owned()),
            ..WindowQuickSelectOptions::default()
        });

        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
        app.handle_pty_output(b"ticket-1234 https://default.test")
            .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(query.to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        app.command_palette_execute(expected);

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 1);
        assert_eq!(quick_select.labels.as_slice(), ["1"]);
        assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_args_wezterm_action_parenthesized_table_query() {
        let query = "wezterm.action.QuickSelectArgs({ pattern = 'ticket-[0-9]+', action = 'open-uri', alphabet = '12', label = 'open ticket', skip_action_on_paste = true, scope_lines = 2 })";
        let expected_options = WindowQuickSelectOptions {
            patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
            alphabet: Some("12".to_owned()),
            label: Some("open ticket".to_owned()),
            action: Some(WindowQuickSelectAction::OpenUri),
            skip_action_on_paste: true,
            scope_lines: Some(2),
        };

        assert_eq!(quick_select_options_from_query(query), expected_options);

        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
        app.handle_pty_output(b"ticket-1234 https://default.test")
            .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(query.to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::QuickSelectArgs(expected_options.clone())]
        );
        app.command_palette_execute(WindowCommand::QuickSelectArgs(expected_options));

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 1);
        assert_eq!(quick_select.labels.as_slice(), ["1"]);
        assert_eq!(quick_select.action_label.as_deref(), Some("open ticket"));
        assert_eq!(quick_select.action, WindowQuickSelectAction::OpenUri);
        assert!(quick_select.skip_action_on_paste);
        assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_args_wezterm_nested_copy_action_queries() {
        for (query, expected_action) in [
            (
                "wezterm.action.QuickSelectArgs({ pattern = 'ticket-[0-9]+', action = wezterm.action.CopyTo 'Clipboard' })",
                WindowQuickSelectAction::CopyTo(WindowCopyDestination::Clipboard),
            ),
            (
                "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = act.CopyTo('PrimarySelection') }",
                WindowQuickSelectAction::CopyTo(WindowCopyDestination::PrimarySelection),
            ),
            (
                "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = wezterm.action { CopyTo = 'ClipboardAndPrimarySelection' } }",
                WindowQuickSelectAction::CopyTo(
                    WindowCopyDestination::ClipboardAndPrimarySelection,
                ),
            ),
        ] {
            let expected_options = WindowQuickSelectOptions {
                patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                action: Some(expected_action.clone()),
                ..WindowQuickSelectOptions::default()
            };

            assert_eq!(quick_select_options_from_query(query), expected_options);

            let mut app = NativeWindowApp::new(None);
            app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
            app.handle_pty_output(b"ticket-1234 https://default.test")
                .unwrap();

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![WindowCommand::QuickSelectArgs(expected_options.clone())]
            );
            app.command_palette_execute(WindowCommand::QuickSelectArgs(expected_options));

            let quick_select = active_quick_select_for_test(&app);
            assert_eq!(quick_select.matches.len(), 1);
            assert_eq!(quick_select.action, expected_action);
            assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
        }
    }

    #[test]
    fn window_app_dispatches_quick_select_args_nested_complete_selection_actions() {
        for (query, expected_action, expected_clipboard, expected_primary) in [
            (
                "wezterm.action.QuickSelectArgs({ pattern = 'ticket-[0-9]+', action = wezterm.action.CompleteSelection 'Clipboard' })",
                WindowQuickSelectAction::CopyTo(WindowCopyDestination::Clipboard),
                vec!["ticket-1234"],
                Vec::new(),
            ),
            (
                "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = act.CompleteSelectionOrOpenLinkAtMouseCursor('PrimarySelection') }",
                WindowQuickSelectAction::CopyTo(WindowCopyDestination::PrimarySelection),
                Vec::new(),
                vec!["ticket-1234"],
            ),
        ] {
            let expected_options = WindowQuickSelectOptions {
                patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                action: Some(expected_action.clone()),
                ..WindowQuickSelectOptions::default()
            };

            assert_eq!(quick_select_options_from_query(query), expected_options);

            let copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_copy = Arc::clone(&copied);
            let primary_copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_primary = Arc::clone(&primary_copied);
            let mut app = NativeWindowApp::new(None);
            app.clipboard_writer = Box::new(move |text: &str| {
                recorded_copy.lock().unwrap().push(text.to_owned());
                true
            });
            app.primary_selection_writer = Box::new(move |text: &str| {
                recorded_primary.lock().unwrap().push(text.to_owned());
                true
            });
            app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
            app.handle_pty_output(b"ticket-1234 https://default.test")
                .unwrap();

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![WindowCommand::QuickSelectArgs(expected_options.clone())]
            );
            app.command_palette_execute(WindowCommand::QuickSelectArgs(expected_options));

            let quick_select = active_quick_select_for_test(&app);
            assert_eq!(quick_select.matches.len(), 1);
            assert_eq!(quick_select.action, expected_action);
            assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
            let label = quick_select.labels[0].clone();

            assert!(app.handle_quick_select_logical_key(
                &Key::Character(label.into()),
                ModifiersState::empty()
            ));

            assert!(quick_select_for_test(&app).is_none());
            assert!(app.selection.is_none());
            assert_eq!(copied.lock().unwrap().as_slice(), expected_clipboard);
            assert_eq!(primary_copied.lock().unwrap().as_slice(), expected_primary);
        }
    }

    #[test]
    fn window_app_dispatches_quick_select_args_nested_paste_from_actions() {
        for (query, expected_written) in [
            (
                "wezterm.action.QuickSelectArgs({ pattern = 'ticket-[0-9]+', action = wezterm.action.PasteFrom 'Clipboard' })",
                encode_window_paste(
                    "clipboard\ntext",
                    false,
                    DEFAULT_CANONICALIZE_PASTED_NEWLINES,
                ),
            ),
            (
                "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = act.PasteFrom('PrimarySelection') }",
                encode_window_paste("primary\ntext", false, DEFAULT_CANONICALIZE_PASTED_NEWLINES),
            ),
            (
                "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = wezterm.action { PasteFrom = 'Clipboard' } }",
                encode_window_paste(
                    "clipboard\ntext",
                    false,
                    DEFAULT_CANONICALIZE_PASTED_NEWLINES,
                ),
            ),
        ] {
            let written = Arc::new(Mutex::new(Vec::new()));
            let copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_copy = Arc::clone(&copied);
            let primary_copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_primary = Arc::clone(&primary_copied);
            let mut app = NativeWindowApp::new(None);
            app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
            app.clipboard_reader = Box::new(|| Some("clipboard\ntext".to_owned()));
            app.primary_selection_reader = Box::new(|| Some("primary\ntext".to_owned()));
            app.clipboard_writer = Box::new(move |text: &str| {
                recorded_copy.lock().unwrap().push(text.to_owned());
                true
            });
            app.primary_selection_writer = Box::new(move |text: &str| {
                recorded_primary.lock().unwrap().push(text.to_owned());
                true
            });
            app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
            app.handle_pty_output(b"ticket-1234 https://default.test")
                .unwrap();

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected_command =
                WindowCommand::QuickSelectArgs(quick_select_options_from_query(query));
            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected_command.clone()]
            );
            app.command_palette_execute(expected_command);

            let quick_select = active_quick_select_for_test(&app);
            assert_eq!(quick_select.matches.len(), 1);
            assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
            let label = quick_select.labels[0].clone();

            assert!(app.handle_quick_select_logical_key(
                &Key::Character(label.into()),
                ModifiersState::empty()
            ));

            assert!(quick_select_for_test(&app).is_none());
            assert!(app.selection.is_none());
            assert_eq!(
                written.lock().unwrap().as_slice(),
                expected_written.as_slice()
            );
            assert!(copied.lock().unwrap().is_empty());
            assert!(primary_copied.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn window_app_dispatches_quick_select_args_nested_send_string_actions() {
        for (query, expected_written) in [
            (
                "wezterm.action.QuickSelectArgs({ pattern = 'ticket-[0-9]+', action = wezterm.action.SendString 'alpha' })",
                b"alpha".as_slice(),
            ),
            (
                "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = act.SendString('beta') }",
                b"beta".as_slice(),
            ),
            (
                "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = wezterm.action.SendString { string = 'gamma' } }",
                b"gamma".as_slice(),
            ),
            (
                "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = wezterm.action { SendString = { string = 'delta' } } }",
                b"delta".as_slice(),
            ),
        ] {
            let written = Arc::new(Mutex::new(Vec::new()));
            let copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_copy = Arc::clone(&copied);
            let primary_copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_primary = Arc::clone(&primary_copied);
            let mut app = NativeWindowApp::new(None);
            app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
            app.clipboard_writer = Box::new(move |text: &str| {
                recorded_copy.lock().unwrap().push(text.to_owned());
                true
            });
            app.primary_selection_writer = Box::new(move |text: &str| {
                recorded_primary.lock().unwrap().push(text.to_owned());
                true
            });
            app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
            app.handle_pty_output(b"ticket-1234 https://default.test")
                .unwrap();

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected_command =
                WindowCommand::QuickSelectArgs(quick_select_options_from_query(query));
            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected_command.clone()]
            );
            app.command_palette_execute(expected_command);

            let quick_select = active_quick_select_for_test(&app);
            assert_eq!(quick_select.matches.len(), 1);
            assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
            let label = quick_select.labels[0].clone();

            assert!(app.handle_quick_select_logical_key(
                &Key::Character(label.into()),
                ModifiersState::empty()
            ));

            assert!(quick_select_for_test(&app).is_none());
            assert!(app.selection.is_none());
            assert_eq!(written.lock().unwrap().as_slice(), expected_written);
            assert!(copied.lock().unwrap().is_empty());
            assert!(primary_copied.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn window_app_dispatches_quick_select_args_nested_send_key_actions() {
        for (query, expected_written) in [
            (
                "wezterm.action.QuickSelectArgs({ pattern = 'ticket-[0-9]+', action = wezterm.action.SendKey { key = 'b', mods = 'ALT' } })",
                b"\x1bb".as_slice(),
            ),
            (
                "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = act.SendKey({ key = 'LeftArrow', mods = 'ALT' }) }",
                b"\x1b[1;3D".as_slice(),
            ),
            (
                "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = wezterm.action { SendKey = { key = 'b', mods = 'ALT' } } }",
                b"\x1bb".as_slice(),
            ),
        ] {
            let written = Arc::new(Mutex::new(Vec::new()));
            let copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_copy = Arc::clone(&copied);
            let primary_copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_primary = Arc::clone(&primary_copied);
            let mut app = NativeWindowApp::new(None);
            app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
            app.clipboard_writer = Box::new(move |text: &str| {
                recorded_copy.lock().unwrap().push(text.to_owned());
                true
            });
            app.primary_selection_writer = Box::new(move |text: &str| {
                recorded_primary.lock().unwrap().push(text.to_owned());
                true
            });
            app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
            app.handle_pty_output(b"ticket-1234 https://default.test")
                .unwrap();

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected_command =
                WindowCommand::QuickSelectArgs(quick_select_options_from_query(query));
            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected_command.clone()]
            );
            app.command_palette_execute(expected_command);

            let quick_select = active_quick_select_for_test(&app);
            assert_eq!(quick_select.matches.len(), 1);
            assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
            let label = quick_select.labels[0].clone();

            assert!(app.handle_quick_select_logical_key(
                &Key::Character(label.into()),
                ModifiersState::empty()
            ));

            assert!(quick_select_for_test(&app).is_none());
            assert!(app.selection.is_none());
            assert_eq!(written.lock().unwrap().as_slice(), expected_written);
            assert!(copied.lock().unwrap().is_empty());
            assert!(primary_copied.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn window_app_dispatches_quick_select_args_nested_emit_event_actions() {
        for (query, expected_name) in [
            (
                "wezterm.action.QuickSelectArgs({ pattern = 'ticket-[0-9]+', action = wezterm.action.EmitEvent 'quick-select-hit' })",
                "quick-select-hit",
            ),
            (
                "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = act.EmitEvent({ name = 'quick-select-table' }) }",
                "quick-select-table",
            ),
            (
                "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = wezterm.action { EmitEvent = { name = 'quick-select-wrapper' } } }",
                "quick-select-wrapper",
            ),
        ] {
            let events = Arc::new(Mutex::new(Vec::new()));
            let recorded_events = Arc::clone(&events);
            let copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_copy = Arc::clone(&copied);
            let primary_copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_primary = Arc::clone(&primary_copied);
            let mut app = NativeWindowApp::new(None);
            app.emit_event_handler = Box::new(move |event| {
                recorded_events.lock().unwrap().push(event.clone());
                true
            });
            app.clipboard_writer = Box::new(move |text: &str| {
                recorded_copy.lock().unwrap().push(text.to_owned());
                true
            });
            app.primary_selection_writer = Box::new(move |text: &str| {
                recorded_primary.lock().unwrap().push(text.to_owned());
                true
            });
            app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
            app.handle_pty_output(b"ticket-1234 https://default.test")
                .unwrap();

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected_command =
                WindowCommand::QuickSelectArgs(quick_select_options_from_query(query));
            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected_command.clone()]
            );
            app.command_palette_execute(expected_command);

            let quick_select = active_quick_select_for_test(&app);
            assert_eq!(quick_select.matches.len(), 1);
            assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
            let label = quick_select.labels[0].clone();

            assert!(app.handle_quick_select_logical_key(
                &Key::Character(label.into()),
                ModifiersState::empty()
            ));

            assert!(quick_select_for_test(&app).is_none());
            assert!(app.selection.is_none());
            assert_eq!(
                events.lock().unwrap().as_slice(),
                [NativeWindowEmitEvent {
                    window_id: rterm_core::WindowId::new(1),
                    pane: rterm_core::PaneId::new(1),
                    name: expected_name.to_owned(),
                }]
            );
            assert!(copied.lock().unwrap().is_empty());
            assert!(primary_copied.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn window_app_dispatches_quick_select_args_nested_multiple_actions() {
        for query in [
            "wezterm.action.QuickSelectArgs({ pattern = 'ticket-[0-9]+', action = wezterm.action.Multiple { wezterm.action.SendString 'alpha', wezterm.action.EmitEvent 'quick-select-multiple' } })",
            "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = act.Multiple({ act.SendString('alpha'), act.EmitEvent({ name = 'quick-select-multiple' }) }) }",
        ] {
            let written = Arc::new(Mutex::new(Vec::new()));
            let events = Arc::new(Mutex::new(Vec::new()));
            let recorded_events = Arc::clone(&events);
            let copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_copy = Arc::clone(&copied);
            let primary_copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_primary = Arc::clone(&primary_copied);
            let mut app = NativeWindowApp::new(None);
            app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
            app.emit_event_handler = Box::new(move |event| {
                recorded_events.lock().unwrap().push(event.clone());
                true
            });
            app.clipboard_writer = Box::new(move |text: &str| {
                recorded_copy.lock().unwrap().push(text.to_owned());
                true
            });
            app.primary_selection_writer = Box::new(move |text: &str| {
                recorded_primary.lock().unwrap().push(text.to_owned());
                true
            });
            app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
            app.handle_pty_output(b"ticket-1234 https://default.test")
                .unwrap();

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected_command =
                WindowCommand::QuickSelectArgs(quick_select_options_from_query(query));
            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected_command.clone()]
            );
            app.command_palette_execute(expected_command);

            let quick_select = active_quick_select_for_test(&app);
            assert_eq!(quick_select.matches.len(), 1);
            assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
            let label = quick_select.labels[0].clone();

            assert!(app.handle_quick_select_logical_key(
                &Key::Character(label.into()),
                ModifiersState::empty()
            ));

            assert!(quick_select_for_test(&app).is_none());
            assert!(app.selection.is_none());
            assert_eq!(written.lock().unwrap().as_slice(), b"alpha");
            assert_eq!(
                events.lock().unwrap().as_slice(),
                [NativeWindowEmitEvent {
                    window_id: rterm_core::WindowId::new(1),
                    pane: rterm_core::PaneId::new(1),
                    name: "quick-select-multiple".to_owned(),
                }]
            );
            assert!(copied.lock().unwrap().is_empty());
            assert!(primary_copied.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn window_app_dispatches_quick_select_args_nested_activate_key_table_actions() {
        for query in [
            "wezterm.action.QuickSelectArgs({ pattern = 'ticket-[0-9]+', action = wezterm.action.ActivateKeyTable { name = 'resize_pane', one_shot = false, prevent_fallback = true } })",
            "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = act.ActivateKeyTable({ name = 'resize_pane', one_shot = false, prevent_fallback = true }) }",
        ] {
            let copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_copy = Arc::clone(&copied);
            let primary_copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_primary = Arc::clone(&primary_copied);
            let mut app = NativeWindowApp::new(None);
            app.clipboard_writer = Box::new(move |text: &str| {
                recorded_copy.lock().unwrap().push(text.to_owned());
                true
            });
            app.primary_selection_writer = Box::new(move |text: &str| {
                recorded_primary.lock().unwrap().push(text.to_owned());
                true
            });
            app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
            app.handle_pty_output(b"ticket-1234 https://default.test")
                .unwrap();

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected_command =
                WindowCommand::QuickSelectArgs(quick_select_options_from_query(query));
            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected_command.clone()]
            );
            app.command_palette_execute(expected_command);

            let quick_select = active_quick_select_for_test(&app);
            assert_eq!(quick_select.matches.len(), 1);
            assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
            let label = quick_select.labels[0].clone();

            assert!(app.handle_quick_select_logical_key(
                &Key::Character(label.into()),
                ModifiersState::empty()
            ));

            assert!(quick_select_for_test(&app).is_none());
            assert!(app.selection.is_none());
            assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));
            let active = app.key_table_stack.last().expect("active key table");
            assert!(!active.one_shot);
            assert!(active.prevent_fallback);
            assert!(copied.lock().unwrap().is_empty());
            assert!(primary_copied.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn window_app_dispatches_quick_select_args_nested_key_table_stack_actions() {
        for (query, expected_active_table) in [
            (
                "wezterm.action.QuickSelectArgs({ pattern = 'ticket-[0-9]+', action = wezterm.action.PopKeyTable })",
                Some("leader"),
            ),
            (
                "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = act.ClearKeyTableStack() }",
                None,
            ),
        ] {
            let copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_copy = Arc::clone(&copied);
            let primary_copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_primary = Arc::clone(&primary_copied);
            let mut app = NativeWindowApp::new(None);
            app.clipboard_writer = Box::new(move |text: &str| {
                recorded_copy.lock().unwrap().push(text.to_owned());
                true
            });
            app.primary_selection_writer = Box::new(move |text: &str| {
                recorded_primary.lock().unwrap().push(text.to_owned());
                true
            });
            app.activate_key_table(WindowActivateKeyTable {
                name: "leader".to_owned(),
                timeout_milliseconds: None,
                one_shot: false,
                replace_current: false,
                until_unknown: false,
                prevent_fallback: false,
            });
            app.activate_key_table(WindowActivateKeyTable {
                name: "resize_pane".to_owned(),
                timeout_milliseconds: None,
                one_shot: false,
                replace_current: false,
                until_unknown: false,
                prevent_fallback: false,
            });
            assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

            app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
            app.handle_pty_output(b"ticket-1234 https://default.test")
                .unwrap();

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let expected_command =
                WindowCommand::QuickSelectArgs(quick_select_options_from_query(query));
            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected_command.clone()]
            );
            app.command_palette_execute(expected_command);

            let quick_select = active_quick_select_for_test(&app);
            assert_eq!(quick_select.matches.len(), 1);
            assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
            let label = quick_select.labels[0].clone();

            assert!(app.handle_quick_select_logical_key(
                &Key::Character(label.into()),
                ModifiersState::empty()
            ));

            assert!(quick_select_for_test(&app).is_none());
            assert!(app.selection.is_none());
            assert_eq!(app.active_key_table_for_test(), expected_active_table);
            assert!(copied.lock().unwrap().is_empty());
            assert!(primary_copied.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_args_wezterm_nested_nop_action_queries() {
        for query in [
            "wezterm.action.QuickSelectArgs({ pattern = 'ticket-[0-9]+', action = wezterm.action.Nop })",
            "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = act.Nop() }",
        ] {
            let expected_options = WindowQuickSelectOptions {
                patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                action: Some(WindowQuickSelectAction::Nop),
                ..WindowQuickSelectOptions::default()
            };

            assert_eq!(quick_select_options_from_query(query), expected_options);

            let copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_copy = Arc::clone(&copied);
            let primary_copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_primary = Arc::clone(&primary_copied);
            let mut app = NativeWindowApp::new(None);
            app.clipboard_writer = Box::new(move |text: &str| {
                recorded_copy.lock().unwrap().push(text.to_owned());
                true
            });
            app.primary_selection_writer = Box::new(move |text: &str| {
                recorded_primary.lock().unwrap().push(text.to_owned());
                true
            });
            app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
            app.handle_pty_output(b"ticket-1234 https://default.test")
                .unwrap();

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![WindowCommand::QuickSelectArgs(expected_options.clone())]
            );
            app.command_palette_execute(WindowCommand::QuickSelectArgs(expected_options));

            let quick_select = active_quick_select_for_test(&app);
            assert_eq!(quick_select.matches.len(), 1);
            assert_eq!(quick_select.action, WindowQuickSelectAction::Nop);
            assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
            let label = quick_select.labels[0].clone();

            assert!(app.handle_quick_select_logical_key(
                &Key::Character(label.into()),
                ModifiersState::empty()
            ));

            assert!(quick_select_for_test(&app).is_none());
            assert!(app.selection.is_none());
            assert!(copied.lock().unwrap().is_empty());
            assert!(primary_copied.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_args_wezterm_action_callback_query() {
        let query = "wezterm.action.QuickSelectArgs { label = 'open url', pattern = 'ticket-[0-9]+', skip_action_on_paste = true, action = wezterm.action_callback(function(window, pane) end) }";
        let expected_options = WindowQuickSelectOptions {
            patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
            label: Some("open url".to_owned()),
            action: Some(WindowQuickSelectAction::Nop),
            skip_action_on_paste: true,
            ..WindowQuickSelectOptions::default()
        };

        assert_eq!(quick_select_options_from_query(query), expected_options);

        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_copy = Arc::clone(&copied);
        let primary_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_primary = Arc::clone(&primary_copied);
        let mut app = NativeWindowApp::new(None);
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_copy.lock().unwrap().push(text.to_owned());
            true
        });
        app.primary_selection_writer = Box::new(move |text: &str| {
            recorded_primary.lock().unwrap().push(text.to_owned());
            true
        });
        app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
        app.handle_pty_output(b"ticket-1234 https://default.test")
            .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(query.to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::QuickSelectArgs(expected_options.clone())]
        );
        app.command_palette_execute(WindowCommand::QuickSelectArgs(expected_options));

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 1);
        assert_eq!(quick_select.action, WindowQuickSelectAction::Nop);
        assert_eq!(quick_select.action_label.as_deref(), Some("open url"));
        assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
        let label = quick_select.labels[0].clone();

        assert!(app.handle_quick_select_logical_key(
            &Key::Character(label.into()),
            ModifiersState::empty()
        ));

        assert!(quick_select_for_test(&app).is_none());
        assert!(app.selection.is_none());
        assert!(copied.lock().unwrap().is_empty());
        assert!(primary_copied.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_dispatches_quick_select_args_callback_perform_action() {
        let query = "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = wezterm.action_callback(function(window, pane) window:perform_action(wezterm.action.SendString 'picked-ticket', pane) end) }";
        let expected_options = WindowQuickSelectOptions {
            patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
            action: Some(WindowQuickSelectAction::SendString(
                "picked-ticket".to_owned(),
            )),
            ..WindowQuickSelectOptions::default()
        };

        assert_eq!(quick_select_options_from_query(query), expected_options);

        let written = Arc::new(Mutex::new(Vec::new()));
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_copy = Arc::clone(&copied);
        let primary_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_primary = Arc::clone(&primary_copied);
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_copy.lock().unwrap().push(text.to_owned());
            true
        });
        app.primary_selection_writer = Box::new(move |text: &str| {
            recorded_primary.lock().unwrap().push(text.to_owned());
            true
        });
        app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
        app.handle_pty_output(b"ticket-1234 https://default.test")
            .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(query.to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::QuickSelectArgs(expected_options.clone())]
        );
        app.command_palette_execute(WindowCommand::QuickSelectArgs(expected_options));

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 1);
        assert_eq!(
            quick_select.action,
            WindowQuickSelectAction::SendString("picked-ticket".to_owned())
        );
        assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
        let label = quick_select.labels[0].clone();

        assert!(app.handle_quick_select_logical_key(
            &Key::Character(label.into()),
            ModifiersState::empty()
        ));

        assert!(quick_select_for_test(&app).is_none());
        assert!(app.selection.is_none());
        assert_eq!(written.lock().unwrap().as_slice(), b"picked-ticket");
        assert!(copied.lock().unwrap().is_empty());
        assert!(primary_copied.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_dispatches_quick_select_args_callback_sends_selected_text() {
        let query = "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = wezterm.action_callback(function(window, pane) pane:send_text(window:get_selection_text_for_pane(pane)) end) }";
        let expected_options = WindowQuickSelectOptions {
            patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
            action: Some(WindowQuickSelectAction::SendSelectedText),
            ..WindowQuickSelectOptions::default()
        };

        assert_eq!(quick_select_options_from_query(query), expected_options);

        let written = Arc::new(Mutex::new(Vec::new()));
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_copy = Arc::clone(&copied);
        let primary_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_primary = Arc::clone(&primary_copied);
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_copy.lock().unwrap().push(text.to_owned());
            true
        });
        app.primary_selection_writer = Box::new(move |text: &str| {
            recorded_primary.lock().unwrap().push(text.to_owned());
            true
        });
        app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
        app.handle_pty_output(b"ticket-1234 https://default.test")
            .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(query.to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::QuickSelectArgs(expected_options.clone())]
        );
        app.command_palette_execute(WindowCommand::QuickSelectArgs(expected_options));

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 1);
        assert_eq!(
            quick_select.action,
            WindowQuickSelectAction::SendSelectedText
        );
        assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
        let label = quick_select.labels[0].clone();

        assert!(app.handle_quick_select_logical_key(
            &Key::Character(label.into()),
            ModifiersState::empty()
        ));

        assert!(quick_select_for_test(&app).is_none());
        assert!(app.selection.is_none());
        assert_eq!(written.lock().unwrap().as_slice(), b"ticket-1234");
        assert!(copied.lock().unwrap().is_empty());
        assert!(primary_copied.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_dispatches_quick_select_args_callback_pastes_selected_text() {
        let query = "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = wezterm.action_callback(function(window, pane) pane:send_paste(window:get_selection_text_for_pane(pane)) end) }";
        let expected_options = WindowQuickSelectOptions {
            patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
            action: Some(WindowQuickSelectAction::PasteSelectedText),
            ..WindowQuickSelectOptions::default()
        };

        assert_eq!(quick_select_options_from_query(query), expected_options);

        let written = Arc::new(Mutex::new(Vec::new()));
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_copy = Arc::clone(&copied);
        let primary_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_primary = Arc::clone(&primary_copied);
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_copy.lock().unwrap().push(text.to_owned());
            true
        });
        app.primary_selection_writer = Box::new(move |text: &str| {
            recorded_primary.lock().unwrap().push(text.to_owned());
            true
        });
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());
        app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
        app.handle_pty_output(b"ticket-1234 https://default.test")
            .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(query.to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::QuickSelectArgs(expected_options.clone())]
        );
        app.command_palette_execute(WindowCommand::QuickSelectArgs(expected_options));

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 1);
        assert_eq!(
            quick_select.action,
            WindowQuickSelectAction::PasteSelectedText
        );
        assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
        let label = quick_select.labels[0].clone();

        assert!(app.handle_quick_select_logical_key(
            &Key::Character(label.into()),
            ModifiersState::empty()
        ));

        assert!(quick_select_for_test(&app).is_none());
        assert!(app.selection.is_none());
        let expected =
            encode_window_paste("ticket-1234", true, DEFAULT_CANONICALIZE_PASTED_NEWLINES);
        assert_eq!(written.lock().unwrap().as_slice(), expected.as_slice());
        assert!(copied.lock().unwrap().is_empty());
        assert!(primary_copied.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_dispatches_quick_select_args_callback_opens_selected_text() {
        let query = r#"
            wezterm.action.QuickSelectArgs {
              label = 'open url',
              pattern = 'https?://\\S+',
              skip_action_on_paste = true,
              action = wezterm.action_callback(function(window, pane)
                local url = window:get_selection_text_for_pane(pane)
                wezterm.log_info('opening: ' .. url)
                wezterm.open_with(url)
              end),
            }
        "#;
        let expected_options = WindowQuickSelectOptions {
            patterns: Some(vec!["https?://\\S+".to_owned()]),
            label: Some("open url".to_owned()),
            action: Some(WindowQuickSelectAction::OpenUri),
            skip_action_on_paste: true,
            ..WindowQuickSelectOptions::default()
        };

        assert_eq!(quick_select_options_from_query(query), expected_options);

        let open_uris = Arc::new(Mutex::new(Vec::new()));
        let recorded_uri = Arc::clone(&open_uris);
        let opened = Arc::new(Mutex::new(Vec::new()));
        let recorded_open = Arc::clone(&opened);
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_copy = Arc::clone(&copied);
        let primary_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_primary = Arc::clone(&primary_copied);
        let mut app = NativeWindowApp::new(None);
        app.open_uri_handler = Box::new(move |event| {
            recorded_uri.lock().unwrap().push(event.clone());
            true
        });
        app.hyperlink_opener = Box::new(move |url: &str| {
            recorded_open.lock().unwrap().push(url.to_owned());
            true
        });
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_copy.lock().unwrap().push(text.to_owned());
            true
        });
        app.primary_selection_writer = Box::new(move |text: &str| {
            recorded_primary.lock().unwrap().push(text.to_owned());
            true
        });
        let active_pane = app.app_shell.active_pane_id();
        app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
        app.handle_pty_output(b"https://example.test ticket-1234")
            .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(query.to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::QuickSelectArgs(expected_options.clone())]
        );
        app.command_palette_execute(WindowCommand::QuickSelectArgs(expected_options));

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 1);
        assert_eq!(quick_select.action, WindowQuickSelectAction::OpenUri);
        assert_eq!(app.selected_text().as_deref(), Some("https://example.test"));
        let label = quick_select.labels[0].clone();

        assert!(app.handle_quick_select_logical_key(
            &Key::Character(label.into()),
            ModifiersState::empty()
        ));

        assert!(quick_select_for_test(&app).is_none());
        assert!(app.selection.is_none());
        assert_eq!(
            open_uris.lock().unwrap().as_slice(),
            [NativeWindowOpenUri {
                window_id: rterm_core::WindowId::new(1),
                pane: active_pane,
                uri: "https://example.test".to_owned(),
            }]
        );
        assert_eq!(opened.lock().unwrap().as_slice(), ["https://example.test"]);
        assert!(copied.lock().unwrap().is_empty());
        assert!(primary_copied.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_dispatches_quick_select_args_callback_perform_action_clipboard_actions() {
        struct Case {
            query: &'static str,
            action: WindowQuickSelectAction,
            expected_written: Vec<u8>,
            expected_clipboard: Vec<String>,
            expected_primary: Vec<String>,
        }

        for case in [
            Case {
                query: "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = wezterm.action_callback(function(window, pane) window:perform_action(wezterm.action.CopyTo 'Clipboard', pane) end) }",
                action: WindowQuickSelectAction::CopyTo(WindowCopyDestination::Clipboard),
                expected_written: Vec::new(),
                expected_clipboard: vec!["ticket-1234".to_owned()],
                expected_primary: Vec::new(),
            },
            Case {
                query: "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = wezterm.action_callback(function(window, pane) window:perform_action(act.CompleteSelectionOrOpenLinkAtMouseCursor('PrimarySelection'), pane) end) }",
                action: WindowQuickSelectAction::CopyTo(WindowCopyDestination::PrimarySelection),
                expected_written: Vec::new(),
                expected_clipboard: Vec::new(),
                expected_primary: vec!["ticket-1234".to_owned()],
            },
            Case {
                query: "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = wezterm.action_callback(function(window, pane) window:perform_action(wezterm.action.PasteFrom 'PrimarySelection', pane) end) }",
                action: WindowQuickSelectAction::PasteFrom(WindowPasteSource::PrimarySelection),
                expected_written: encode_window_paste(
                    "primary\ntext",
                    false,
                    DEFAULT_CANONICALIZE_PASTED_NEWLINES,
                ),
                expected_clipboard: Vec::new(),
                expected_primary: Vec::new(),
            },
            Case {
                query: "wezterm.action.QuickSelectArgs { pattern = 'ticket-[0-9]+', action = wezterm.action_callback(function(window, pane) window:perform_action(wezterm.action.Nop, pane) end) }",
                action: WindowQuickSelectAction::Nop,
                expected_written: Vec::new(),
                expected_clipboard: Vec::new(),
                expected_primary: Vec::new(),
            },
        ] {
            let expected_options = WindowQuickSelectOptions {
                patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                action: Some(case.action.clone()),
                ..WindowQuickSelectOptions::default()
            };

            assert_eq!(
                quick_select_options_from_query(case.query),
                expected_options
            );

            let written = Arc::new(Mutex::new(Vec::new()));
            let copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_copy = Arc::clone(&copied);
            let primary_copied = Arc::new(Mutex::new(Vec::new()));
            let recorded_primary = Arc::clone(&primary_copied);
            let mut app = NativeWindowApp::new(None);
            app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
            app.clipboard_reader = Box::new(|| Some("clipboard\ntext".to_owned()));
            app.primary_selection_reader = Box::new(|| Some("primary\ntext".to_owned()));
            app.clipboard_writer = Box::new(move |text: &str| {
                recorded_copy.lock().unwrap().push(text.to_owned());
                true
            });
            app.primary_selection_writer = Box::new(move |text: &str| {
                recorded_primary.lock().unwrap().push(text.to_owned());
                true
            });
            app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
            app.handle_pty_output(b"ticket-1234 https://default.test")
                .unwrap();

            app.enter_command_palette_mode();
            app.command_palette_set_query(case.query.to_owned());
            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![WindowCommand::QuickSelectArgs(expected_options.clone())]
            );
            app.command_palette_execute(WindowCommand::QuickSelectArgs(expected_options));

            let quick_select = active_quick_select_for_test(&app);
            assert_eq!(quick_select.matches.len(), 1);
            assert_eq!(quick_select.action, case.action);
            assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
            let label = quick_select.labels[0].clone();

            assert!(app.handle_quick_select_logical_key(
                &Key::Character(label.into()),
                ModifiersState::empty()
            ));

            assert!(quick_select_for_test(&app).is_none());
            assert!(app.selection.is_none());
            assert_eq!(written.lock().unwrap().as_slice(), case.expected_written);
            assert_eq!(copied.lock().unwrap().as_slice(), case.expected_clipboard);
            assert_eq!(
                primary_copied.lock().unwrap().as_slice(),
                case.expected_primary
            );
        }
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_args_wezterm_action_patterns_table_query() {
        let query = "wezterm.action.QuickSelectArgs { patterns = { 'https?://\\\\S+', 'ticket-[0-9]+' }, alphabet = '12', label = 'open match' }";
        let expected_options = WindowQuickSelectOptions {
            patterns: Some(vec!["https?://\\S+".to_owned(), "ticket-[0-9]+".to_owned()]),
            alphabet: Some("12".to_owned()),
            label: Some("open match".to_owned()),
            action: None,
            skip_action_on_paste: false,
            scope_lines: None,
        };

        assert_eq!(quick_select_options_from_query(query), expected_options);

        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
        app.handle_pty_output(b"ticket-1234 https://example.test")
            .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(query.to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::QuickSelectArgs(expected_options.clone())]
        );
        app.command_palette_execute(WindowCommand::QuickSelectArgs(expected_options));

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 2);
        assert_eq!(quick_select.labels.len(), 2);
        assert_eq!(quick_select.action_label.as_deref(), Some("open match"));
        assert_eq!(quick_select.action, WindowQuickSelectAction::Copy);
        assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_args_wezterm_action_table_long_bracket_key_query()
    {
        let query = "wezterm.action.QuickSelectArgs { [[=[pattern]=]] = [[ticket-[0-9]+]], [[=[alphabet]=]] = [[12]], [[=[label]=]] = [[Pick]], [[=[skip_action_on_paste]=]] = true, [[=[scope_lines]=]] = 1 }";
        let expected_options = WindowQuickSelectOptions {
            patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
            alphabet: Some("12".to_owned()),
            label: Some("Pick".to_owned()),
            action: None,
            skip_action_on_paste: true,
            scope_lines: Some(1),
        };

        assert_eq!(quick_select_options_from_query(query), expected_options);

        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
        app.handle_pty_output(b"ticket-1234 https://default.test")
            .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(query.to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::QuickSelectArgs(expected_options.clone())]
        );
        app.command_palette_execute(WindowCommand::QuickSelectArgs(expected_options));

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 1);
        assert_eq!(quick_select.labels.as_slice(), ["1"]);
        assert_eq!(quick_select.action_label.as_deref(), Some("Pick"));
        assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_wrapped_copy_to_long_bracket_key_query() {
        let query = "wezterm.action.QuickSelectArgs { pattern = [[ticket-[0-9]+]], action = wezterm.action { [[=[CopyTo]=]] = [[PrimarySelection]] } }";
        let expected_options = WindowQuickSelectOptions {
            patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
            action: Some(WindowQuickSelectAction::CopyTo(
                WindowCopyDestination::PrimarySelection,
            )),
            ..WindowQuickSelectOptions::default()
        };

        assert_eq!(quick_select_options_from_query(query), expected_options);

        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
        app.handle_pty_output(b"ticket-1234 https://default.test")
            .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(query.to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::QuickSelectArgs(expected_options.clone())]
        );
        app.command_palette_execute(WindowCommand::QuickSelectArgs(expected_options));

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 1);
        assert_eq!(
            quick_select.action,
            WindowQuickSelectAction::CopyTo(WindowCopyDestination::PrimarySelection)
        );
        assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_action_name_queries() {
        for (query, expected_options) in quick_select_action_cases!() {
            assert_eq!(quick_select_options_from_query(query), expected_options);

            let mut app = NativeWindowApp::new(None);
            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());

            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![WindowCommand::EnterQuickSelect]
            );
        }
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_label_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
        app.handle_pty_output(b"https://example.test").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("quick select label open link".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );

        app.command_palette_execute(WindowCommand::EnterQuickSelect);

        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Quick Select open link: [1 / 1]"
        );
        assert!(app.command_palette.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(copy_mode_for_test(&app).is_none());
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_label_query_with_quoted_label() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
        app.handle_pty_output(b"https://example.test").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("quick select label \"open link\"".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );

        app.command_palette_execute(WindowCommand::EnterQuickSelect);

        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Quick Select open link: [1 / 1]"
        );
        assert!(app.command_palette.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(copy_mode_for_test(&app).is_none());
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_action_open_uri_query() {
        let open_uris = Arc::new(Mutex::new(Vec::new()));
        let recorded_uri = Arc::clone(&open_uris);
        let opened = Arc::new(Mutex::new(Vec::new()));
        let recorded_open = Arc::clone(&opened);
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_copy = Arc::clone(&copied);
        let primary_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_primary = Arc::clone(&primary_copied);
        let mut app = NativeWindowApp::new(None);
        app.open_uri_handler = Box::new(move |event| {
            recorded_uri.lock().unwrap().push(event.clone());
            true
        });
        app.hyperlink_opener = Box::new(move |url: &str| {
            recorded_open.lock().unwrap().push(url.to_owned());
            true
        });
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_copy.lock().unwrap().push(text.to_owned());
            true
        });
        app.primary_selection_writer = Box::new(move |text: &str| {
            recorded_primary.lock().unwrap().push(text.to_owned());
            true
        });
        let active_pane = app.app_shell.active_pane_id();
        app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
        app.handle_pty_output(b"https://example.test").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("quick select action open uri".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );

        app.command_palette_execute(WindowCommand::EnterQuickSelect);
        assert!(
            app.handle_quick_select_logical_key(
                &Key::Character("a".into()),
                ModifiersState::empty()
            )
        );

        assert!(quick_select_for_test(&app).is_none());
        assert!(app.selection.is_none());
        assert_eq!(
            open_uris.lock().unwrap().as_slice(),
            [NativeWindowOpenUri {
                window_id: rterm_core::WindowId::new(1),
                pane: active_pane,
                uri: "https://example.test".to_owned(),
            }]
        );
        assert_eq!(opened.lock().unwrap().as_slice(), ["https://example.test"]);
        assert!(copied.lock().unwrap().is_empty());
        assert!(primary_copied.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_action_quoted_open_uri_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("quick select action \"open uri\"".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_action_open_uri_skip_action_on_paste_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let open_uris = Arc::new(Mutex::new(Vec::new()));
        let recorded_uri = Arc::clone(&open_uris);
        let opened = Arc::new(Mutex::new(Vec::new()));
        let recorded_open = Arc::clone(&opened);
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_copy = Arc::clone(&copied);
        let primary_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_primary = Arc::clone(&primary_copied);
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.open_uri_handler = Box::new(move |event| {
            recorded_uri.lock().unwrap().push(event.clone());
            true
        });
        app.hyperlink_opener = Box::new(move |url: &str| {
            recorded_open.lock().unwrap().push(url.to_owned());
            true
        });
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_copy.lock().unwrap().push(text.to_owned());
            true
        });
        app.primary_selection_writer = Box::new(move |text: &str| {
            recorded_primary.lock().unwrap().push(text.to_owned());
            true
        });
        app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
        app.handle_pty_output(b"https://example.test").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "quick select action open uri skip action on paste".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );

        app.command_palette_execute(WindowCommand::EnterQuickSelect);
        assert!(
            app.handle_quick_select_logical_key(&Key::Character("A".into()), ModifiersState::SHIFT)
        );

        assert!(quick_select_for_test(&app).is_none());
        assert!(app.selection.is_none());
        assert_eq!(written.lock().unwrap().as_slice(), b"https://example.test");
        assert!(open_uris.lock().unwrap().is_empty());
        assert!(opened.lock().unwrap().is_empty());
        assert!(copied.lock().unwrap().is_empty());
        assert!(primary_copied.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_action_quoted_open_uri_skip_action_on_paste_query()
     {
        let open_uris = Arc::new(Mutex::new(Vec::new()));
        let recorded_uri = Arc::clone(&open_uris);
        let opened = Arc::new(Mutex::new(Vec::new()));
        let recorded_open = Arc::clone(&opened);
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_copy = Arc::clone(&copied);
        let mut app = NativeWindowApp::new(None);
        app.open_uri_handler = Box::new(move |event| {
            recorded_uri.lock().unwrap().push(event.clone());
            true
        });
        app.hyperlink_opener = Box::new(move |url: &str| {
            recorded_open.lock().unwrap().push(url.to_owned());
            true
        });
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_copy.lock().unwrap().push(text.to_owned());
            true
        });
        let active_pane = app.app_shell.active_pane_id();
        app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
        app.handle_pty_output(b"https://example.test").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "quick select action \"open uri\" skip action on paste false".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );

        app.command_palette_execute(WindowCommand::EnterQuickSelect);
        assert!(
            app.handle_quick_select_logical_key(
                &Key::Character("a".into()),
                ModifiersState::empty()
            )
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
        assert!(copied.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_action_equals_skip_action_on_paste_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let open_uris = Arc::new(Mutex::new(Vec::new()));
        let recorded_uri = Arc::clone(&open_uris);
        let opened = Arc::new(Mutex::new(Vec::new()));
        let recorded_open = Arc::clone(&opened);
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_copy = Arc::clone(&copied);
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.open_uri_handler = Box::new(move |event| {
            recorded_uri.lock().unwrap().push(event.clone());
            true
        });
        app.hyperlink_opener = Box::new(move |url: &str| {
            recorded_open.lock().unwrap().push(url.to_owned());
            true
        });
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_copy.lock().unwrap().push(text.to_owned());
            true
        });
        app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
        app.handle_pty_output(b"https://example.test").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "quick select action \"open uri\" skip-action-on-paste=true".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );

        app.command_palette_execute(WindowCommand::EnterQuickSelect);
        assert!(
            app.handle_quick_select_logical_key(&Key::Character("A".into()), ModifiersState::SHIFT)
        );

        assert!(quick_select_for_test(&app).is_none());
        assert!(app.selection.is_none());
        assert_eq!(written.lock().unwrap().as_slice(), b"https://example.test");
        assert!(open_uris.lock().unwrap().is_empty());
        assert!(opened.lock().unwrap().is_empty());
        assert!(copied.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_rejects_palette_quick_select_invalid_action_with_skip_action_on_paste_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("quick select action bogus skip action on paste".to_owned());

        assert!(app.command_palette_filtered_commands().is_empty());
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_action_copy_to_clipboard_query() {
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_copy = Arc::clone(&copied);
        let primary_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_primary = Arc::clone(&primary_copied);
        let mut app = NativeWindowApp::new(None);
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_copy.lock().unwrap().push(text.to_owned());
            true
        });
        app.primary_selection_writer = Box::new(move |text: &str| {
            recorded_primary.lock().unwrap().push(text.to_owned());
            true
        });
        app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
        app.handle_pty_output(b"https://example.test").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("quick select action copy to clipboard".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );

        app.command_palette_execute(WindowCommand::EnterQuickSelect);
        assert!(
            app.handle_quick_select_logical_key(
                &Key::Character("a".into()),
                ModifiersState::empty()
            )
        );

        assert!(quick_select_for_test(&app).is_none());
        assert!(app.selection.is_none());
        assert_eq!(copied.lock().unwrap().as_slice(), ["https://example.test"]);
        assert!(primary_copied.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_action_copy_to_primary_selection_query() {
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_copy = Arc::clone(&copied);
        let primary_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_primary = Arc::clone(&primary_copied);
        let mut app = NativeWindowApp::new(None);
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_copy.lock().unwrap().push(text.to_owned());
            true
        });
        app.primary_selection_writer = Box::new(move |text: &str| {
            recorded_primary.lock().unwrap().push(text.to_owned());
            true
        });
        app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
        app.handle_pty_output(b"https://example.test").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("quick select action copy to primary selection".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );

        app.command_palette_execute(WindowCommand::EnterQuickSelect);
        assert!(
            app.handle_quick_select_logical_key(
                &Key::Character("a".into()),
                ModifiersState::empty()
            )
        );

        assert!(quick_select_for_test(&app).is_none());
        assert!(app.selection.is_none());
        assert!(copied.lock().unwrap().is_empty());
        assert_eq!(
            primary_copied.lock().unwrap().as_slice(),
            ["https://example.test"]
        );
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_action_copy_to_quoted_query() {
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_copy = Arc::clone(&copied);
        let primary_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_primary = Arc::clone(&primary_copied);
        let mut app = NativeWindowApp::new(None);
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_copy.lock().unwrap().push(text.to_owned());
            true
        });
        app.primary_selection_writer = Box::new(move |text: &str| {
            recorded_primary.lock().unwrap().push(text.to_owned());
            true
        });
        app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
        app.handle_pty_output(b"https://example.test").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "quick select action Copy To \"Primary Selection\"".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );

        app.command_palette_execute(WindowCommand::EnterQuickSelect);
        assert!(
            app.handle_quick_select_logical_key(
                &Key::Character("a".into()),
                ModifiersState::empty()
            )
        );

        assert!(quick_select_for_test(&app).is_none());
        assert!(app.selection.is_none());
        assert!(copied.lock().unwrap().is_empty());
        assert_eq!(
            primary_copied.lock().unwrap().as_slice(),
            ["https://example.test"]
        );
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_action_copy_to_clipboard_and_primary_selection_query()
     {
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_copy = Arc::clone(&copied);
        let primary_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_primary = Arc::clone(&primary_copied);
        let mut app = NativeWindowApp::new(None);
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_copy.lock().unwrap().push(text.to_owned());
            true
        });
        app.primary_selection_writer = Box::new(move |text: &str| {
            recorded_primary.lock().unwrap().push(text.to_owned());
            true
        });
        app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
        app.handle_pty_output(b"https://example.test").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "quick select action copy to clipboard and primary selection".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );

        app.command_palette_execute(WindowCommand::EnterQuickSelect);
        assert!(
            app.handle_quick_select_logical_key(
                &Key::Character("a".into()),
                ModifiersState::empty()
            )
        );

        assert!(quick_select_for_test(&app).is_none());
        assert!(app.selection.is_none());
        assert_eq!(copied.lock().unwrap().as_slice(), ["https://example.test"]);
        assert_eq!(
            primary_copied.lock().unwrap().as_slice(),
            ["https://example.test"]
        );
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_scope_lines_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(24, 2));
        app.handle_pty_output(
            b"old@example.com\r\nfar@example.com\r\nnear@example.com\r\nmid@example.com\r\nvis0@example.com\r\nvis1@example.com",
        )
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("quick select scope lines 2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );

        app.command_palette_execute(WindowCommand::EnterQuickSelect);

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 4);
        let dimensions = app.runtime.terminal().stable_dimensions();
        let scope_start = dimensions.physical_top.saturating_sub(2);
        let scope_end = dimensions.physical_top.saturating_add(
            StableRowIndex::try_from(dimensions.viewport_rows.saturating_sub(1)).unwrap(),
        );
        assert!(quick_select.matches.iter().all(|quick_select_match| {
            (scope_start..=scope_end).contains(&quick_select_match.source_row)
        }));
        assert_eq!(app.selected_text().as_deref(), Some("near@example.com"));
        assert!(app.command_palette.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(copy_mode_for_test(&app).is_none());
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_hyphenated_scope_lines_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(24, 2));
        app.handle_pty_output(
            b"old@example.com\r\nfar@example.com\r\nnear@example.com\r\nmid@example.com\r\nvis0@example.com\r\nvis1@example.com",
        )
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("quick select scope-lines 2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );

        app.command_palette_execute(WindowCommand::EnterQuickSelect);

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 4);
        let dimensions = app.runtime.terminal().stable_dimensions();
        let scope_start = dimensions.physical_top.saturating_sub(2);
        let scope_end = dimensions.physical_top.saturating_add(
            StableRowIndex::try_from(dimensions.viewport_rows.saturating_sub(1)).unwrap(),
        );
        assert!(quick_select.matches.iter().all(|quick_select_match| {
            (scope_start..=scope_end).contains(&quick_select_match.source_row)
        }));
        assert_eq!(app.selected_text().as_deref(), Some("near@example.com"));
        assert!(app.command_palette.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(copy_mode_for_test(&app).is_none());
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_scope_lines_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(24, 2));
        app.handle_pty_output(
            b"old@example.com\r\nfar@example.com\r\nnear@example.com\r\nmid@example.com\r\nvis0@example.com\r\nvis1@example.com",
        )
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("quick select scope_lines=2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );

        app.command_palette_execute(WindowCommand::EnterQuickSelect);

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 4);
        let dimensions = app.runtime.terminal().stable_dimensions();
        let scope_start = dimensions.physical_top.saturating_sub(2);
        let scope_end = dimensions.physical_top.saturating_add(
            StableRowIndex::try_from(dimensions.viewport_rows.saturating_sub(1)).unwrap(),
        );
        assert!(quick_select.matches.iter().all(|quick_select_match| {
            (scope_start..=scope_end).contains(&quick_select_match.source_row)
        }));
        assert_eq!(app.selected_text().as_deref(), Some("near@example.com"));
        assert!(app.command_palette.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(copy_mode_for_test(&app).is_none());
    }

    #[test]
    fn window_app_rejects_palette_quick_select_scope_lines_query_with_trailing_text() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("quick select scope lines 2 extra".to_owned());

        assert!(app.command_palette_filtered_commands().is_empty());
    }

    #[test]
    fn window_app_dispatches_palette_enter_pane_select_command() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::EnterPaneSelect);

        assert!(app.pane_select.is_some());
        assert!(app.command_palette.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(copy_mode_for_test(&app).is_none());
        assert!(quick_select_for_test(&app).is_none());
    }

    #[test]
    fn window_app_exposes_wezterm_pane_select_command() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("pane select".to_owned());
        let command = app
            .command_palette_filtered_commands()
            .into_iter()
            .find(|command| command.label() == "Pane Select")
            .expect("expected pane select command");
        app.command_palette_execute(command);

        assert_eq!(
            app.pane_select.as_ref().map(|pane_select| pane_select.mode),
            Some(WindowPaneSelectMode::Activate)
        );
        assert!(app.command_palette.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(copy_mode_for_test(&app).is_none());
        assert!(quick_select_for_test(&app).is_none());
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_alphabet_query() {
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

        app.enter_command_palette_mode();
        app.command_palette_set_query("pane select alphabet 12".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterPaneSelect]
        );

        app.command_palette_execute(WindowCommand::EnterPaneSelect);

        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[0].pane_id, rterm_core::PaneId::new(1));
        assert_eq!(pane_select.labels[1].label, "2");
        assert_eq!(pane_select.labels[1].pane_id, rterm_core::PaneId::new(2));
        assert!(!pane_select.show_pane_ids);
        assert!(app.command_palette.is_none());

        assert!(app.handle_pane_select_key(&Key::Character("2".into()), ModifiersState::empty()));

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert!(app.pane_select.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_action_name_alphabet_query() {
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

        app.enter_command_palette_mode();
        app.command_palette_set_query("paneselect alphabet 12".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterPaneSelect]
        );

        app.command_palette_execute(WindowCommand::EnterPaneSelect);

        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[1].label, "2");
        assert!(!pane_select.show_pane_ids);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_action_name_alphabet_equals_query() {
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

        app.enter_command_palette_mode();
        app.command_palette_set_query("paneselect alphabet=12".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterPaneSelect]
        );

        app.command_palette_execute(WindowCommand::EnterPaneSelect);

        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[1].label, "2");
        assert!(!pane_select.show_pane_ids);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_quoted_alphabet_query() {
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

        app.enter_command_palette_mode();
        app.command_palette_set_query("pane select alphabet \"12\"".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterPaneSelect]
        );

        app.command_palette_execute(WindowCommand::EnterPaneSelect);

        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[1].label, "2");
        assert!(!pane_select.show_pane_ids);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn parses_pane_select_quoted_alphabet_query_variants() {
        assert_eq!(
            pane_select_show_pane_ids_alphabet_from_query(
                "pane select show pane ids alphabet \"12\""
            )
            .as_deref(),
            Some("12")
        );
        assert_eq!(
            pane_select_show_pane_ids_alphabet_from_query(
                "paneselect show pane ids alphabet \"12\""
            )
            .as_deref(),
            Some("12")
        );
        assert_eq!(
            pane_select_activate_alphabet_from_query("pane select activate alphabet \"12\"")
                .as_deref(),
            Some("12")
        );
        assert_eq!(
            pane_select_activate_show_pane_ids_alphabet_from_query(
                "pane select activate show pane ids alphabet \"12\""
            )
            .as_deref(),
            Some("12")
        );
        assert_eq!(
            pane_select_mode_alphabet_from_query("pane select swap alphabet \"12\"")
                .expect("expected pane select mode query")
                .alphabet,
            "12"
        );
        assert_eq!(
            pane_select_mode_alphabet_from_query("paneselect swap alphabet \"12\"")
                .expect("expected pane select action-name mode query")
                .alphabet,
            "12"
        );
        assert_eq!(
            pane_select_mode_show_pane_ids_from_query(
                "pane select swap show pane ids alphabet \"12\""
            )
            .expect("expected pane select mode show ids query")
            .alphabet
            .as_deref(),
            Some("12")
        );
        assert_eq!(
            pane_select_show_pane_ids_alphabet_from_query(
                "pane select show-pane-ids alphabet \"12\""
            )
            .as_deref(),
            Some("12")
        );
        assert_eq!(
            pane_select_mode_show_pane_ids_from_query(
                "paneselect swap show-pane-ids alphabet \"12\""
            )
            .expect("expected pane select action-name mode show ids query")
            .alphabet
            .as_deref(),
            Some("12")
        );
    }

    #[test]
    fn parses_pane_select_alphabet_equals_query_variants() {
        assert_eq!(
            pane_select_alphabet_from_query("paneselect alphabet=12").as_deref(),
            Some("12")
        );
        assert_eq!(
            pane_select_show_pane_ids_alphabet_from_query("pane select show-pane-ids alphabet=12")
                .as_deref(),
            Some("12")
        );
        assert_eq!(
            pane_select_activate_alphabet_from_query("pane select activate alphabet=12").as_deref(),
            Some("12")
        );
        assert_eq!(
            pane_select_activate_show_pane_ids_alphabet_from_query(
                "paneselect activate show pane ids alphabet=12"
            )
            .as_deref(),
            Some("12")
        );
        assert_eq!(
            pane_select_mode_alphabet_from_query("paneselect swap alphabet=12")
                .expect("expected pane select action-name mode query")
                .alphabet,
            "12"
        );
        assert_eq!(
            pane_select_mode_show_pane_ids_from_query("paneselect swap show-pane-ids alphabet=12")
                .expect("expected pane select action-name mode show ids query")
                .alphabet
                .as_deref(),
            Some("12")
        );
        assert_eq!(
            pane_select_options_from_query(
                "pane select mode swap_with_active show_pane_ids=true alphabet=12"
            ),
            Some(WindowPaneSelectOptions {
                mode: WindowPaneSelectMode::SwapWithActive,
                show_pane_ids: true,
                alphabet: Some("12".to_owned()),
            })
        );
        assert_eq!(
            pane_select_options_from_query(
                "pane select mode swap_with_active show pane ids=true alphabet=12"
            ),
            Some(WindowPaneSelectOptions {
                mode: WindowPaneSelectMode::SwapWithActive,
                show_pane_ids: true,
                alphabet: Some("12".to_owned()),
            })
        );
        assert_eq!(
            pane_select_options_from_query(
                "paneselect=mode=swap_with_active show_pane_ids=true alphabet=12"
            ),
            Some(WindowPaneSelectOptions {
                mode: WindowPaneSelectMode::SwapWithActive,
                show_pane_ids: true,
                alphabet: Some("12".to_owned()),
            })
        );
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_show_ids_alphabet_query() {
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

        app.enter_command_palette_mode();
        app.command_palette_set_query("pane select show pane ids alphabet 12".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterPaneSelectShowPaneIds]
        );

        app.command_palette_execute(WindowCommand::EnterPaneSelectShowPaneIds);

        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[0].pane_id, rterm_core::PaneId::new(1));
        assert_eq!(pane_select.labels[1].label, "2");
        assert_eq!(pane_select.labels[1].pane_id, rterm_core::PaneId::new(2));
        assert!(pane_select.show_pane_ids);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_hyphenated_show_ids_alphabet_query() {
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

        app.enter_command_palette_mode();
        app.command_palette_set_query("pane select show-pane-ids alphabet 12".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterPaneSelectShowPaneIds]
        );

        app.command_palette_execute(WindowCommand::EnterPaneSelectShowPaneIds);

        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[0].pane_id, rterm_core::PaneId::new(1));
        assert_eq!(pane_select.labels[1].label, "2");
        assert_eq!(pane_select.labels[1].pane_id, rterm_core::PaneId::new(2));
        assert!(pane_select.show_pane_ids);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_explicit_activate_alphabet_query() {
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

        app.enter_command_palette_mode();
        app.command_palette_set_query("pane select activate alphabet 12".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterPaneSelect]
        );

        app.command_palette_execute(WindowCommand::EnterPaneSelect);

        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(pane_select.mode, WindowPaneSelectMode::Activate);
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[1].label, "2");
        assert!(!pane_select.show_pane_ids);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_explicit_activate_show_ids_alphabet_query() {
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

        app.enter_command_palette_mode();
        app.command_palette_set_query("pane select activate show pane ids alphabet 12".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterPaneSelectShowPaneIds]
        );

        app.command_palette_execute(WindowCommand::EnterPaneSelectShowPaneIds);

        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(pane_select.mode, WindowPaneSelectMode::Activate);
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[1].label, "2");
        assert!(pane_select.show_pane_ids);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_mode_alphabet_queries() {
        let cases = [
            (
                "pane select swap alphabet 12",
                WindowCommand::EnterPaneSwap,
                WindowPaneSelectMode::SwapWithActive,
            ),
            (
                "PaneSelect Swap Alphabet 12",
                WindowCommand::EnterPaneSwap,
                WindowPaneSelectMode::SwapWithActive,
            ),
            (
                "pane select swap keep focus alphabet 12",
                WindowCommand::EnterPaneSwapKeepFocus,
                WindowPaneSelectMode::SwapWithActiveKeepFocus,
            ),
            (
                "pane select move to new tab alphabet 12",
                WindowCommand::EnterPaneMoveToNewTab,
                WindowPaneSelectMode::MoveToNewTab,
            ),
            (
                "pane select move to new window alphabet 12",
                WindowCommand::EnterPaneMoveToNewWindow,
                WindowPaneSelectMode::MoveToNewWindow,
            ),
        ];

        for (query, expected_command, expected_mode) in cases {
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

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());

            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected_command.clone()]
            );

            app.command_palette_execute(expected_command);

            let pane_select = app
                .pane_select
                .as_ref()
                .expect("pane select should be active");
            assert_eq!(pane_select.mode, expected_mode);
            assert_eq!(pane_select.labels[0].label, "1");
            assert_eq!(pane_select.labels[0].pane_id, rterm_core::PaneId::new(1));
            assert_eq!(pane_select.labels[1].label, "2");
            assert_eq!(pane_select.labels[1].pane_id, rterm_core::PaneId::new(2));
            assert!(!pane_select.show_pane_ids);
            assert!(app.command_palette.is_none());
        }
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_mode_show_ids_query() {
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

        app.enter_command_palette_mode();
        app.command_palette_set_query("pane select swap show pane ids".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterPaneSwap]
        );

        app.command_palette_execute(WindowCommand::EnterPaneSwap);

        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(pane_select.mode, WindowPaneSelectMode::SwapWithActive);
        assert_eq!(pane_select.labels[0].label, "x");
        assert_eq!(pane_select.labels[0].pane_id, rterm_core::PaneId::new(1));
        assert_eq!(pane_select.labels[1].label, "y");
        assert_eq!(pane_select.labels[1].pane_id, rterm_core::PaneId::new(2));
        assert!(pane_select.show_pane_ids);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_mode_show_ids_alphabet_queries() {
        let cases = [
            (
                "pane select swap show pane ids alphabet 12",
                WindowCommand::EnterPaneSwap,
                WindowPaneSelectMode::SwapWithActive,
            ),
            (
                "PaneSelect Swap Show Pane IDs Alphabet 12",
                WindowCommand::EnterPaneSwap,
                WindowPaneSelectMode::SwapWithActive,
            ),
            (
                "pane select swap show_pane_ids alphabet 12",
                WindowCommand::EnterPaneSwap,
                WindowPaneSelectMode::SwapWithActive,
            ),
            (
                "pane select swap keep focus show pane ids alphabet 12",
                WindowCommand::EnterPaneSwapKeepFocus,
                WindowPaneSelectMode::SwapWithActiveKeepFocus,
            ),
            (
                "pane select move to new tab show pane ids alphabet 12",
                WindowCommand::EnterPaneMoveToNewTab,
                WindowPaneSelectMode::MoveToNewTab,
            ),
            (
                "paneselect move to new tab show_pane_ids alphabet 12",
                WindowCommand::EnterPaneMoveToNewTab,
                WindowPaneSelectMode::MoveToNewTab,
            ),
            (
                "pane select move to new window show pane ids alphabet 12",
                WindowCommand::EnterPaneMoveToNewWindow,
                WindowPaneSelectMode::MoveToNewWindow,
            ),
        ];

        for (query, expected_command, expected_mode) in cases {
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

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());

            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![expected_command.clone()]
            );

            app.command_palette_execute(expected_command);

            let pane_select = app
                .pane_select
                .as_ref()
                .expect("pane select should be active");
            assert_eq!(pane_select.mode, expected_mode);
            assert_eq!(pane_select.labels[0].label, "1");
            assert_eq!(pane_select.labels[0].pane_id, rterm_core::PaneId::new(1));
            assert_eq!(pane_select.labels[1].label, "2");
            assert_eq!(pane_select.labels[1].pane_id, rterm_core::PaneId::new(2));
            assert!(pane_select.show_pane_ids);
            assert!(app.command_palette.is_none());
        }
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_structured_mode_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "pane select mode swap_with_active_keep_focus Show Pane IDs true Alphabet 12"
                .to_owned(),
        );

        let expected = WindowCommand::PaneSelect(WindowPaneSelectOptions {
            mode: WindowPaneSelectMode::SwapWithActiveKeepFocus,
            show_pane_ids: true,
            alphabet: Some("12".to_owned()),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));
        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(
            pane_select.mode,
            WindowPaneSelectMode::SwapWithActiveKeepFocus
        );
        assert!(pane_select.show_pane_ids);
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[1].label, "2");
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_wezterm_action_parenthesized_table_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.PaneSelect({ mode = 'SwapWithActive', show_pane_ids = true, alphabet = '12' })"
                .to_owned(),
        );

        let expected = WindowCommand::PaneSelect(WindowPaneSelectOptions {
            mode: WindowPaneSelectMode::SwapWithActive,
            show_pane_ids: true,
            alphabet: Some("12".to_owned()),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));
        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(pane_select.mode, WindowPaneSelectMode::SwapWithActive);
        assert!(pane_select.show_pane_ids);
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[1].label, "2");
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_wezterm_action_table_trailing_comma_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.PaneSelect({ mode = 'SwapWithActive', show_pane_ids = true, alphabet = '12', })"
                .to_owned(),
        );

        let expected = WindowCommand::PaneSelect(WindowPaneSelectOptions {
            mode: WindowPaneSelectMode::SwapWithActive,
            show_pane_ids: true,
            alphabet: Some("12".to_owned()),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));
        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(pane_select.mode, WindowPaneSelectMode::SwapWithActive);
        assert!(pane_select.show_pane_ids);
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[1].label, "2");
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_wezterm_action_table_query_with_default_mode() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.PaneSelect { alphabet = '12', show_pane_ids = true }".to_owned(),
        );

        let expected = WindowCommand::PaneSelect(WindowPaneSelectOptions {
            mode: WindowPaneSelectMode::Activate,
            show_pane_ids: true,
            alphabet: Some("12".to_owned()),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));
        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(pane_select.mode, WindowPaneSelectMode::Activate);
        assert!(pane_select.show_pane_ids);
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[1].label, "2");
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_wezterm_action_table_long_bracket_key_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.PaneSelect { [[=[mode]=]] = [[SwapWithActive]], [[=[show_pane_ids]=]] = true, [[=[alphabet]=]] = [[12]] }"
                .to_owned(),
        );

        let expected = WindowCommand::PaneSelect(WindowPaneSelectOptions {
            mode: WindowPaneSelectMode::SwapWithActive,
            show_pane_ids: true,
            alphabet: Some("12".to_owned()),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));
        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(pane_select.mode, WindowPaneSelectMode::SwapWithActive);
        assert!(pane_select.show_pane_ids);
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[1].label, "2");
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_structured_show_ids_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "pane select mode swap_with_active show_pane_ids=true alphabet 12".to_owned(),
        );

        let expected = WindowCommand::PaneSelect(WindowPaneSelectOptions {
            mode: WindowPaneSelectMode::SwapWithActive,
            show_pane_ids: true,
            alphabet: Some("12".to_owned()),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));
        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(pane_select.mode, WindowPaneSelectMode::SwapWithActive);
        assert!(pane_select.show_pane_ids);
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[1].label, "2");
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_unordered_structured_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "paneselect alphabet=12 show_pane_ids=true mode=swap_with_active".to_owned(),
        );

        let expected = WindowCommand::PaneSelect(WindowPaneSelectOptions {
            mode: WindowPaneSelectMode::SwapWithActive,
            show_pane_ids: true,
            alphabet: Some("12".to_owned()),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));
        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(pane_select.mode, WindowPaneSelectMode::SwapWithActive);
        assert!(pane_select.show_pane_ids);
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[1].label, "2");
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_action_name_show_ids_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "paneselect mode swap_with_active_keep_focus show-pane-ids=false alphabet 12"
                .to_owned(),
        );

        let expected = WindowCommand::PaneSelect(WindowPaneSelectOptions {
            mode: WindowPaneSelectMode::SwapWithActiveKeepFocus,
            show_pane_ids: false,
            alphabet: Some("12".to_owned()),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));
        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(
            pane_select.mode,
            WindowPaneSelectMode::SwapWithActiveKeepFocus
        );
        assert!(!pane_select.show_pane_ids);
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[1].label, "2");
    }

    #[test]
    fn window_app_dispatches_palette_pane_select_action_name_mode_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "paneselect mode=swap_with_active show_pane_ids=true alphabet=12".to_owned(),
        );

        let expected = WindowCommand::PaneSelect(WindowPaneSelectOptions {
            mode: WindowPaneSelectMode::SwapWithActive,
            show_pane_ids: true,
            alphabet: Some("12".to_owned()),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));
        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(pane_select.mode, WindowPaneSelectMode::SwapWithActive);
        assert!(pane_select.show_pane_ids);
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[1].label, "2");
    }

    #[test]
    fn window_app_rejects_palette_pane_select_structured_query_with_duplicate_fields() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "pane select mode swap_with_active mode activate show_pane_ids true".to_owned(),
        );
        assert!(app.command_palette_filtered_commands().is_empty());

        app.command_palette_set_query(
            "pane select mode swap_with_active show_pane_ids true show_pane_ids false".to_owned(),
        );
        assert!(app.command_palette_filtered_commands().is_empty());
    }

    #[test]
    fn window_app_dispatches_native_pane_select_option_action() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::PaneSelect(WindowPaneSelectOptions {
            mode: WindowPaneSelectMode::SwapWithActiveKeepFocus,
            show_pane_ids: true,
            alphabet: Some("12".to_owned()),
        }));

        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(
            pane_select.mode,
            WindowPaneSelectMode::SwapWithActiveKeepFocus
        );
        assert!(pane_select.show_pane_ids);
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[0].pane_id, rterm_core::PaneId::new(1));
        assert_eq!(pane_select.labels[1].label, "2");
        assert_eq!(pane_select.labels[1].pane_id, rterm_core::PaneId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_enter_pane_swap_commands() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("pane select swap".to_owned());
        let command = app
            .command_palette_filtered_commands()
            .into_iter()
            .find(|command| command.label() == "Pane Select Swap With Active")
            .expect("expected pane select swap with active command");
        app.command_palette_execute(command);
        assert_eq!(
            app.pane_select.as_ref().map(|pane_select| pane_select.mode),
            Some(WindowPaneSelectMode::SwapWithActive)
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("pane select swap".to_owned());
        let command = app
            .command_palette_filtered_commands()
            .into_iter()
            .find(|command| command.label() == "Pane Select Swap With Active Keep Focus")
            .expect("expected pane select swap with active keep focus command");
        app.command_palette_execute(command);
        assert_eq!(
            app.pane_select.as_ref().map(|pane_select| pane_select.mode),
            Some(WindowPaneSelectMode::SwapWithActiveKeepFocus)
        );
    }

    #[test]
    fn window_app_dispatches_palette_enter_pane_move_to_new_tab_command() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("pane select move to new tab".to_owned());
        let command = app
            .command_palette_filtered_commands()
            .into_iter()
            .find(|command| command.label() == "Pane Select Move To New Tab")
            .expect("expected pane select move to new tab command");
        app.command_palette_execute(command);

        assert_eq!(
            app.pane_select.as_ref().map(|pane_select| pane_select.mode),
            Some(WindowPaneSelectMode::MoveToNewTab)
        );
    }

    #[test]
    fn window_app_dispatches_palette_enter_pane_move_to_new_window_command() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("pane select move to new window".to_owned());
        let command = app
            .command_palette_filtered_commands()
            .into_iter()
            .find(|command| command.label() == "Pane Select Move To New Window")
            .expect("expected pane select move to new window command");
        app.command_palette_execute(command);

        assert_eq!(
            app.pane_select.as_ref().map(|pane_select| pane_select.mode),
            Some(WindowPaneSelectMode::MoveToNewWindow)
        );
    }

    #[test]
    fn window_app_palette_close_pane_requests_window_close_for_last_pane() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();

        assert!(app.command_palette_execute(WindowCommand::ClosePane));
        assert!(app.command_palette.is_none());
        assert!(app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_renders_and_hits_close_buttons_for_each_visible_pane() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        let layout = app.pane_render_layout();

        let close_cells = app.pane_close_button_cells(&layout);

        assert_eq!(close_cells.len(), 2);
        let snapshot = app.render_snapshot();
        for rect in &layout.panes {
            let (row, column) =
                super::pane_close_button_position(*rect).expect("close button fits pane");
            let cell = snapshot_cell(&snapshot, row, column).expect("rendered close button");
            assert_eq!(cell.ch, '×');
            assert_eq!(cell.foreground, Color::Rgb(0x0b, 0x12, 0x20));
            assert_eq!(cell.background, Color::Rgb(0xf8, 0x71, 0x71));

            app.mouse_position =
                Some((column, row.saturating_sub(app.terminal_frame_row_offset())));
            assert_eq!(
                app.pane_close_button_at_mouse_position(),
                Some(rect.pane_id)
            );
        }
    }

    #[test]
    fn window_app_omits_pane_close_button_for_single_pane() {
        let mut app = NativeWindowApp::new(None);
        let layout = app.pane_render_layout();
        let rect = layout.panes[0];
        let (row, column) =
            super::pane_close_button_position(rect).expect("button geometry fits pane");
        app.mouse_position = Some((column, row.saturating_sub(app.terminal_frame_row_offset())));

        assert!(app.pane_close_button_cells(&layout).is_empty());
        assert!(app.pane_close_button_at_mouse_position().is_none());
    }

    #[test]
    fn window_app_pane_close_button_targets_non_active_pane_confirmation_without_forwarding() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("pane-close-test-process"),
        );
        app.window_focused = true;
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"\x1b[?1000;1006h").unwrap();
        let active_pane = app.active_pane_id();
        let target_pane = rterm_core::PaneId::new(1);
        assert_ne!(active_pane, target_pane);
        let target_rect = app
            .pane_render_layout()
            .panes
            .into_iter()
            .find(|rect| rect.pane_id == target_pane)
            .expect("inactive pane rect");
        let (row, column) =
            super::pane_close_button_position(target_rect).expect("close button fits pane");
        app.mouse_position = Some((column, row.saturating_sub(app.terminal_frame_row_offset())));

        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_pane_id(), active_pane);
        assert_eq!(
            app.close_confirmation
                .as_ref()
                .map(|confirmation| confirmation.target.clone()),
            Some(WindowCloseTarget::Pane(target_pane))
        );
        assert_eq!(app.app_shell.pane_ids().len(), 2);
        assert!(written.lock().unwrap().is_empty());
        assert!(app.selection.is_none());
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_pane_close_button_consumes_full_active_pane_click() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("pane-close-test-process"),
        );
        app.window_focused = true;
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            mouse_assignments: Some(vec![NativeUserMouseAssignment {
                event: NativeMouseAssignmentEvent {
                    kind: NativeMouseAssignmentEventKind::Up,
                    button: NativeMouseAssignmentButton::Mouse(MouseButton::Left),
                    streak: 1,
                },
                modifiers: ModifiersState::empty(),
                mouse_reporting: true,
                alt_screen: NativeMouseAssignmentAltScreen::Any,
                command: WindowCommand::StartWindowDrag,
            }]),
            skip_close_confirmation_for_processes_named: Some(vec![
                "pane-close-test-process".to_owned(),
            ]),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[?1000;1006h").unwrap();
        let active_pane = app.active_pane_id();
        let active_rect = app
            .pane_render_layout()
            .panes
            .into_iter()
            .find(|rect| rect.pane_id == active_pane)
            .expect("active pane rect");
        let (row, column) =
            super::pane_close_button_position(active_rect).expect("close button fits pane");
        app.mouse_position = Some((column, row.saturating_sub(app.terminal_frame_row_offset())));

        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.app_shell.pane_ids(), vec![rterm_core::PaneId::new(1)]);
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
        assert!(app.close_confirmation.is_none());
        assert!(written.lock().unwrap().is_empty());
        assert!(!app.window_drag_requested_for_test());
        assert!(app.selection.is_none());
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_pane_badge_takes_render_and_hit_priority_over_close_button() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        set_badge_format(&mut app, "status");
        let active_pane = app.active_pane_id();
        let active_rect = app
            .pane_render_layout()
            .panes
            .into_iter()
            .find(|rect| rect.pane_id == active_pane)
            .expect("active pane rect");
        let (row, column) =
            super::pane_close_button_position(active_rect).expect("close button fits pane");

        let snapshot = app.render_snapshot();
        let cell = snapshot_cell(&snapshot, row, column).expect("badge cell at pane corner");
        assert_eq!(cell.ch, ' ');
        assert_eq!(cell.foreground, DEFAULT_UI_SURFACE_FOREGROUND);
        assert_eq!(cell.background, DEFAULT_UI_SURFACE_BACKGROUND);

        app.mouse_position = Some((column, row.saturating_sub(app.terminal_frame_row_offset())));
        assert!(app.pane_close_button_at_mouse_position().is_none());
    }

    #[test]
    fn window_app_command_palette_blocks_pane_close_button_full_click() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("pane-close-test-process"),
        );
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            mouse_assignments: Some(vec![NativeUserMouseAssignment {
                event: NativeMouseAssignmentEvent {
                    kind: NativeMouseAssignmentEventKind::Up,
                    button: NativeMouseAssignmentButton::Mouse(MouseButton::Left),
                    streak: 1,
                },
                modifiers: ModifiersState::empty(),
                mouse_reporting: true,
                alt_screen: NativeMouseAssignmentAltScreen::Any,
                command: WindowCommand::StartWindowDrag,
            }]),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[?1000;1006h").unwrap();
        let active_pane = app.active_pane_id();
        let active_rect = app
            .pane_render_layout()
            .panes
            .into_iter()
            .find(|rect| rect.pane_id == active_pane)
            .expect("active pane rect");
        let (row, column) =
            super::pane_close_button_position(active_rect).expect("close button fits pane");
        app.mouse_position = Some((column, row.saturating_sub(app.terminal_frame_row_offset())));
        app.enter_command_palette_mode();
        assert!(app.pane_close_button_at_mouse_position().is_none());

        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );

        assert!(app.command_palette.is_some());
        assert!(app.close_confirmation.is_none());
        assert_eq!(app.app_shell.pane_ids().len(), 2);
        assert_eq!(app.active_pane_id(), active_pane);
        assert!(written.lock().unwrap().is_empty());
        assert!(!app.window_drag_requested_for_test());
    }

    #[test]
    fn window_app_pane_select_consumes_paired_left_release() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            mouse_assignments: Some(vec![NativeUserMouseAssignment {
                event: NativeMouseAssignmentEvent {
                    kind: NativeMouseAssignmentEventKind::Up,
                    button: NativeMouseAssignmentButton::Mouse(MouseButton::Left),
                    streak: 1,
                },
                modifiers: ModifiersState::empty(),
                mouse_reporting: true,
                alt_screen: NativeMouseAssignmentAltScreen::Any,
                command: WindowCommand::StartWindowDrag,
            }]),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[?1000;1006h").unwrap();
        app.mouse_position = Some((0, 0));
        let active_pane = app.active_pane_id();
        app.enter_pane_select_mode();
        assert!(app.pane_select.is_some());

        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );

        assert!(app.pane_select.is_none());
        assert_eq!(app.active_pane_id(), active_pane);
        assert!(written.lock().unwrap().is_empty());
        assert!(!app.window_drag_requested_for_test());
        assert!(app.selection.is_none());
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_pane_close_button_latch_ignores_unmatched_release_and_clears_on_next_press() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        let active_pane = app.active_pane_id();
        let active_rect = app
            .pane_render_layout()
            .panes
            .into_iter()
            .find(|rect| rect.pane_id == active_pane)
            .expect("active pane rect");
        let (close_row, close_column) =
            super::pane_close_button_position(active_rect).expect("close button fits pane");
        let terminal_row = close_row.saturating_sub(app.terminal_frame_row_offset());
        app.mouse_position = Some((close_column, terminal_row));

        assert!(
            !app.handle_pane_close_button_mouse_input(ElementState::Released, MouseButton::Left)
        );

        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert_eq!(app.pressed_pane_close_button, Some(active_pane));
        app.mouse_position = Some((active_rect.column, terminal_row));
        assert!(
            !app.handle_pane_close_button_mouse_input(ElementState::Pressed, MouseButton::Left)
        );
        assert!(app.pressed_pane_close_button.is_none());

        app.mouse_position = Some((close_column, terminal_row));
        assert!(
            !app.handle_pane_close_button_mouse_input(ElementState::Released, MouseButton::Left)
        );
    }

    #[test]
    fn window_app_pane_close_button_consumes_release_after_window_loses_focus() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("pane-close-test-process"),
        );
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            mouse_assignments: Some(vec![NativeUserMouseAssignment {
                event: NativeMouseAssignmentEvent {
                    kind: NativeMouseAssignmentEventKind::Up,
                    button: NativeMouseAssignmentButton::Mouse(MouseButton::Left),
                    streak: 1,
                },
                modifiers: ModifiersState::empty(),
                mouse_reporting: true,
                alt_screen: NativeMouseAssignmentAltScreen::Any,
                command: WindowCommand::StartWindowDrag,
            }]),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[?1000;1006h").unwrap();
        assert!(app.handle_focus_changed(true).unwrap());
        app.mouse_click_may_focus_window = false;
        let active_pane = app.active_pane_id();
        let active_rect = app
            .pane_render_layout()
            .panes
            .into_iter()
            .find(|rect| rect.pane_id == active_pane)
            .expect("active pane rect");
        let (row, column) =
            super::pane_close_button_position(active_rect).expect("close button fits pane");
        app.mouse_position = Some((column, row.saturating_sub(app.terminal_frame_row_offset())));

        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert_eq!(app.pressed_pane_close_button, Some(active_pane));
        assert!(app.handle_focus_changed(false).unwrap());
        assert_eq!(app.pressed_pane_close_button, Some(active_pane));

        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );
        assert!(app.pressed_pane_close_button.is_none());
        assert!(written.lock().unwrap().is_empty());
        assert!(!app.window_drag_requested_for_test());
        assert_eq!(
            app.close_confirmation
                .as_ref()
                .map(|confirmation| confirmation.target.clone()),
            Some(WindowCloseTarget::Pane(active_pane))
        );
    }

    #[test]
    fn pane_close_button_geometry_rejects_empty_or_overflowing_rects() {
        let invalid_rects = [
            super::PaneRenderRect {
                pane_id: rterm_core::PaneId::new(1),
                row: 0,
                column: 0,
                rows: 0,
                columns: 1,
            },
            super::PaneRenderRect {
                pane_id: rterm_core::PaneId::new(1),
                row: 0,
                column: 0,
                rows: 1,
                columns: 0,
            },
            super::PaneRenderRect {
                pane_id: rterm_core::PaneId::new(1),
                row: 0,
                column: u16::MAX,
                rows: 1,
                columns: 2,
            },
            super::PaneRenderRect {
                pane_id: rterm_core::PaneId::new(1),
                row: u16::MAX,
                column: 0,
                rows: 2,
                columns: 1,
            },
        ];
        for rect in invalid_rects {
            assert!(super::pane_close_button_position(rect).is_none());
        }

        let app = NativeWindowApp::new(None);
        let layout = super::PaneRenderLayout {
            panes: invalid_rects
                .into_iter()
                .chain([super::PaneRenderRect {
                    pane_id: rterm_core::PaneId::new(2),
                    row: 2,
                    column: 3,
                    rows: 1,
                    columns: 1,
                }])
                .collect(),
            separators: Vec::new(),
        };
        let cells = app.pane_close_button_cells(&layout);

        assert_eq!(cells.len(), 1);
        assert_eq!((cells[0].row, cells[0].column), (2, 3));
    }

    #[test]
    fn pane_local_overlay_snapshot_consumes_base_and_preserves_later_overlay_priority() {
        let app = NativeWindowApp::new(None);
        let rect = super::PaneRenderRect {
            pane_id: app.active_pane_id(),
            row: 3,
            column: 4,
            rows: 2,
            columns: 2,
        };
        let close = super::ui_render_cell(
            3,
            5,
            'x',
            super::PANE_CLOSE_BUTTON_FOREGROUND,
            super::PANE_CLOSE_BUTTON_BACKGROUND,
            true,
        );
        let later =
            super::ui_render_cell(0, 1, 'q', Color::Rgb(1, 2, 3), Color::Rgb(4, 5, 6), true);

        let snapshot = super::pane_local_overlay_snapshot(app.snapshot.clone(), rect, &[close])
            .with_overlay_cells([later]);

        let cell = snapshot_cell(&snapshot, 0, 1).expect("later overlay cell");
        assert_eq!(cell.ch, 'q');
        assert_eq!(cell.background, Color::Rgb(4, 5, 6));
    }

    #[test]
    fn window_app_dispatches_native_close_current_pane_without_confirmation() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.pane_ids().len(), 2);

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::CloseCurrentPane { confirm: false }));

        assert_eq!(app.app_shell.pane_ids(), vec![rterm_core::PaneId::new(1)]);
        assert!(app.command_palette.is_none());
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_dispatches_native_close_current_tab_without_confirmation() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::CloseCurrentTab { confirm: false }));

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert!(app.command_palette.is_none());
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_default_close_selection_prefers_the_right_tab() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(2),
        })
        .unwrap();

        app.dispatch_app_action(AppAction::CloseTab {
            tab: rterm_core::TabId::new(2),
            switch_to_last_active: false,
        })
        .unwrap();

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
    }

    #[test]
    fn window_app_duplicate_and_reopen_closed_tab_restore_the_full_tab_layout() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: Some(PaneLaunch::local("ssh").with_args(["ops"])),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "operations".to_owned(),
        })
        .unwrap();

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::DuplicateTab));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.app_shell.active_tab().title(), Some("operations"));
        assert_eq!(app.app_shell.active_tab().panes().len(), 2);

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::CloseCurrentTab { confirm: false }));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
        assert_eq!(
            app.closed_tab_history
                .lock()
                .expect("history lock")
                .len(),
            1
        );

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::ReopenClosedTab));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.app_shell.active_tab().title(), Some("operations"));
        assert_eq!(app.app_shell.active_tab().panes().len(), 2);
        assert!(
            app.closed_tab_history
                .lock()
                .expect("history lock")
                .is_empty()
        );
    }

    #[test]
    fn window_app_pending_windows_share_recently_closed_tab_history() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::CloseTabWithSelection {
            tab: rterm_core::TabId::new(2),
            selection: rterm_core::app_shell::CloseTabSelection::Adjacent,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SpawnWindow { launch: None })
            .unwrap();
        let mut detached = app
            .take_next_pending_window_app()
            .expect("pending window should materialize");

        detached.enter_command_palette_mode();
        assert!(detached.command_palette_execute(WindowCommand::ReopenClosedTab));
        assert_eq!(detached.app_shell.active_workspace().tabs().len(), 2);
        assert!(
            detached
                .closed_tab_history
                .lock()
                .expect("history lock")
                .is_empty()
        );
    }

    #[test]
    fn window_app_move_tab_to_new_window_command_preserves_live_tab_ownership() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::MoveTabToNewWindow));

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
        let detached = app
            .take_next_pending_window_app()
            .expect("moved tab should become a pending window");
        assert_eq!(detached.app_shell.active_workspace().tabs().len(), 1);
        assert_eq!(detached.active_tab_id(), rterm_core::TabId::new(2));
    }

    #[test]
    fn window_app_dispatches_palette_close_current_pane_confirm_false_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.pane_ids().len(), 2);

        app.enter_command_palette_mode();
        app.command_palette_set_query("close current pane confirm false".to_owned());

        let expected = WindowCommand::CloseCurrentPane { confirm: false };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(app.app_shell.pane_ids(), vec![rterm_core::PaneId::new(1)]);
        assert!(app.command_palette.is_none());
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_dispatches_palette_close_current_pane_wezterm_action_table_call_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.pane_ids().len(), 2);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.CloseCurrentPane { confirm = false }".to_owned(),
        );

        let expected = WindowCommand::CloseCurrentPane { confirm: false };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(app.app_shell.pane_ids(), vec![rterm_core::PaneId::new(1)]);
        assert!(app.command_palette.is_none());
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_dispatches_palette_close_current_pane_table_long_bracket_key_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.pane_ids().len(), 2);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.CloseCurrentPane { [[=[confirm]=]] = false }".to_owned(),
        );

        let expected = WindowCommand::CloseCurrentPane { confirm: false };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(app.app_shell.pane_ids(), vec![rterm_core::PaneId::new(1)]);
        assert!(app.command_palette.is_none());
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_dispatches_palette_close_current_pane_confirm_equals_false_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.pane_ids().len(), 2);

        app.enter_command_palette_mode();
        app.command_palette_set_query("closecurrentpane confirm=false".to_owned());

        let expected = WindowCommand::CloseCurrentPane { confirm: false };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(app.app_shell.pane_ids(), vec![rterm_core::PaneId::new(1)]);
        assert!(app.command_palette.is_none());
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_dispatches_palette_close_current_pane_compact_confirm_equals_false_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.pane_ids().len(), 2);

        app.enter_command_palette_mode();
        app.command_palette_set_query("closecurrentpaneconfirm=false".to_owned());

        let expected = WindowCommand::CloseCurrentPane { confirm: false };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(app.app_shell.pane_ids(), vec![rterm_core::PaneId::new(1)]);
        assert!(app.command_palette.is_none());
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_dispatches_palette_close_current_tab_confirm_true_query() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("python.exe"));
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));

        app.enter_command_palette_mode();
        app.command_palette_set_query("close current tab confirm true".to_owned());

        let expected = WindowCommand::CloseCurrentTab { confirm: true };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:2 pane:2] - Close Current Tab? Enter/Y=yes Esc/N=no"
        );
        assert!(app.command_palette.is_none());
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_dispatches_palette_close_current_tab_wezterm_action_table_call_query() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("python.exe"));
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.CloseCurrentTab { confirm = true }".to_owned(),
        );

        let expected = WindowCommand::CloseCurrentTab { confirm: true };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:2 pane:2] - Close Current Tab? Enter/Y=yes Esc/N=no"
        );
        assert!(app.command_palette.is_none());
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_dispatches_palette_close_current_tab_table_long_bracket_key_query() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("python.exe"));
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.CloseCurrentTab { [[=[confirm]=]] = true }".to_owned(),
        );

        let expected = WindowCommand::CloseCurrentTab { confirm: true };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:2 pane:2] - Close Current Tab? Enter/Y=yes Esc/N=no"
        );
        assert!(app.command_palette.is_none());
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_dispatches_palette_close_current_wezterm_action_table_trailing_comma_queries() {
        for (query, expected) in [
            (
                "wezterm.action.CloseCurrentPane { confirm = false, }",
                WindowCommand::CloseCurrentPane { confirm: false },
            ),
            (
                "wezterm.action.CloseCurrentPane({ confirm = false, })",
                WindowCommand::CloseCurrentPane { confirm: false },
            ),
            (
                "wezterm.action.CloseCurrentTab { confirm = true, }",
                WindowCommand::CloseCurrentTab { confirm: true },
            ),
            (
                "wezterm.action.CloseCurrentTab({ confirm = true, })",
                WindowCommand::CloseCurrentTab { confirm: true },
            ),
        ] {
            let mut app = NativeWindowApp::new(None);

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());

            assert_eq!(app.command_palette_filtered_commands(), vec![expected]);
        }
    }

    #[test]
    fn window_app_dispatches_palette_close_current_tab_confirm_yes_query() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("python.exe"));
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));

        app.enter_command_palette_mode();
        app.command_palette_set_query("close current tab confirm yes".to_owned());

        let expected = WindowCommand::CloseCurrentTab { confirm: true };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:2 pane:2] - Close Current Tab? Enter/Y=yes Esc/N=no"
        );
        assert!(app.command_palette.is_none());
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_dispatches_palette_close_current_tab_confirm_equals_true_query() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("python.exe"));
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));

        app.enter_command_palette_mode();
        app.command_palette_set_query("close current tab confirm=true".to_owned());

        let expected = WindowCommand::CloseCurrentTab { confirm: true };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:2 pane:2] - Close Current Tab? Enter/Y=yes Esc/N=no"
        );
        assert!(app.command_palette.is_none());
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_dispatches_palette_close_current_tab_compact_confirm_equals_true_query() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("python.exe"));
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));

        app.enter_command_palette_mode();
        app.command_palette_set_query("closecurrenttabconfirm=true".to_owned());

        let expected = WindowCommand::CloseCurrentTab { confirm: true };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:2 pane:2] - Close Current Tab? Enter/Y=yes Esc/N=no"
        );
        assert!(app.command_palette.is_none());
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_close_current_tab_without_confirmation_honors_last_active_config() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(3),
        })
        .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            switch_to_last_active_tab_when_closing_tab: Some(true),
            ..NativeConfigSnapshot::default()
        });

        assert!(app.command_palette_execute(WindowCommand::CloseCurrentTab { confirm: false }));

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_close_tab_shortcut_honors_last_active_config() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("python.exe"));
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(3),
        })
        .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            switch_to_last_active_tab_when_closing_tab: Some(true),
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

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 3);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert_eq!(
            app.close_confirmation
                .as_ref()
                .map(|confirmation| confirmation.target.clone()),
            Some(WindowCloseTarget::Tab(rterm_core::TabId::new(3)))
        );

        assert!(
            app.handle_close_confirmation_key(
                &Key::Named(NamedKey::Enter),
                ModifiersState::empty()
            )
        );

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
    }

    #[test]
    fn window_app_dispatches_native_set_window_level_payloads() {
        let mut app = NativeWindowApp::new(None);

        assert_eq!(app.window_level_for_test(), NativeWindowLevel::Normal);

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::SetWindowLevel(
            NativeWindowLevel::AlwaysOnTop
        )));
        assert_eq!(app.window_level_for_test(), NativeWindowLevel::AlwaysOnTop);
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::SetWindowLevel(
            NativeWindowLevel::AlwaysOnBottom
        )));
        assert_eq!(
            app.window_level_for_test(),
            NativeWindowLevel::AlwaysOnBottom
        );
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        assert!(
            app.command_palette_execute(WindowCommand::SetWindowLevel(NativeWindowLevel::Normal))
        );
        assert_eq!(app.window_level_for_test(), NativeWindowLevel::Normal);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_set_window_level_always_on_top_query() {
        let mut app = NativeWindowApp::new(None);

        assert_eq!(app.window_level_for_test(), NativeWindowLevel::Normal);

        app.enter_command_palette_mode();
        app.command_palette_set_query("set window level always on top".to_owned());

        let expected = WindowCommand::SetWindowLevel(NativeWindowLevel::AlwaysOnTop);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(app.window_level_for_test(), NativeWindowLevel::AlwaysOnTop);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_set_window_level_equals_query() {
        let mut app = NativeWindowApp::new(None);

        assert_eq!(app.window_level_for_test(), NativeWindowLevel::Normal);

        app.enter_command_palette_mode();
        app.command_palette_set_query("set window level=always on top".to_owned());

        let expected = WindowCommand::SetWindowLevel(NativeWindowLevel::AlwaysOnTop);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(app.window_level_for_test(), NativeWindowLevel::AlwaysOnTop);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_set_window_level_compact_equals_query() {
        let mut app = NativeWindowApp::new(None);

        assert_eq!(app.window_level_for_test(), NativeWindowLevel::Normal);

        app.enter_command_palette_mode();
        app.command_palette_set_query("setwindowlevel=always on bottom".to_owned());

        let expected = WindowCommand::SetWindowLevel(NativeWindowLevel::AlwaysOnBottom);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(
            app.window_level_for_test(),
            NativeWindowLevel::AlwaysOnBottom
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_set_window_level_action_name_query() {
        let mut app = NativeWindowApp::new(None);

        assert_eq!(app.window_level_for_test(), NativeWindowLevel::Normal);

        app.enter_command_palette_mode();
        app.command_palette_set_query("setwindowlevel always on bottom".to_owned());

        let expected = WindowCommand::SetWindowLevel(NativeWindowLevel::AlwaysOnBottom);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(
            app.window_level_for_test(),
            NativeWindowLevel::AlwaysOnBottom
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_set_window_level_assignment_query() {
        let mut app = NativeWindowApp::new(None);

        assert_eq!(app.window_level_for_test(), NativeWindowLevel::Normal);

        app.enter_command_palette_mode();
        app.command_palette_set_query("setwindowlevel level=always on top".to_owned());

        let expected = WindowCommand::SetWindowLevel(NativeWindowLevel::AlwaysOnTop);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(app.window_level_for_test(), NativeWindowLevel::AlwaysOnTop);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_set_window_level_quoted_query() {
        let mut app = NativeWindowApp::new(None);

        assert_eq!(app.window_level_for_test(), NativeWindowLevel::Normal);

        app.enter_command_palette_mode();
        app.command_palette_set_query("set window level \"always on top\"".to_owned());

        let expected = WindowCommand::SetWindowLevel(NativeWindowLevel::AlwaysOnTop);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(app.window_level_for_test(), NativeWindowLevel::AlwaysOnTop);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_set_window_level_wezterm_action_bare_string_query() {
        let mut app = NativeWindowApp::new(None);

        assert_eq!(app.window_level_for_test(), NativeWindowLevel::Normal);

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.SetWindowLevel 'AlwaysOnTop'".to_owned());

        let expected = WindowCommand::SetWindowLevel(NativeWindowLevel::AlwaysOnTop);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(app.window_level_for_test(), NativeWindowLevel::AlwaysOnTop);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_set_window_level_wezterm_action_function_call_query() {
        let mut app = NativeWindowApp::new(None);

        assert_eq!(app.window_level_for_test(), NativeWindowLevel::Normal);

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.SetWindowLevel('AlwaysOnTop')".to_owned());

        let expected = WindowCommand::SetWindowLevel(NativeWindowLevel::AlwaysOnTop);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(app.window_level_for_test(), NativeWindowLevel::AlwaysOnTop);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_set_window_level_normal_query() {
        let mut app = NativeWindowApp::new(None);
        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::SetWindowLevel(
            NativeWindowLevel::AlwaysOnBottom
        )));
        assert_eq!(
            app.window_level_for_test(),
            NativeWindowLevel::AlwaysOnBottom
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("set window level normal".to_owned());

        let expected = WindowCommand::SetWindowLevel(NativeWindowLevel::Normal);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert_eq!(app.window_level_for_test(), NativeWindowLevel::Normal);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_toggle_window_level_actions() {
        let mut app = NativeWindowApp::new(None);

        assert_eq!(app.window_level_for_test(), NativeWindowLevel::Normal);

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::ToggleAlwaysOnTop));
        assert_eq!(app.window_level_for_test(), NativeWindowLevel::AlwaysOnTop);
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::ToggleAlwaysOnTop));
        assert_eq!(app.window_level_for_test(), NativeWindowLevel::Normal);
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::ToggleAlwaysOnBottom));
        assert_eq!(
            app.window_level_for_test(),
            NativeWindowLevel::AlwaysOnBottom
        );
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::ToggleAlwaysOnBottom));
        assert_eq!(app.window_level_for_test(), NativeWindowLevel::Normal);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_does_not_immediately_close_native_close_current_confirming_pane_or_tab() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("python.exe"));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::CloseCurrentPane { confirm: true }));
        assert_eq!(app.app_shell.pane_ids().len(), 3);

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::CloseCurrentTab { confirm: true }));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_accepts_native_close_current_pane_confirmation() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("python.exe"));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::CloseCurrentPane { confirm: true }));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:2] - Close Current Pane? Enter/Y=yes Esc/N=no"
        );

        assert!(
            app.handle_close_confirmation_key(
                &Key::Named(NamedKey::Enter),
                ModifiersState::empty()
            )
        );

        assert_eq!(app.app_shell.pane_ids(), vec![rterm_core::PaneId::new(1)]);
        assert!(!app.window_close_requested_for_test());
        assert!(app.close_confirmation.is_none());
    }

    #[test]
    fn window_app_cancels_native_close_current_tab_confirmation() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("python.exe"));
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::CloseCurrentTab { confirm: true }));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:2 pane:2] - Close Current Tab? Enter/Y=yes Esc/N=no"
        );

        assert!(
            app.handle_close_confirmation_key(&Key::Character("n".into()), ModifiersState::empty())
        );

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert!(app.close_confirmation.is_none());
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_default_close_tab_shortcuts_request_native_confirmation() {
        for modifiers in [
            ModifiersState::SUPER,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        ] {
            let mut app =
                NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("python.exe"));
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
            let active_tab = app.active_tab_id();
            app.modifiers = modifiers;

            app.handle_keyboard_input_event(
                &Key::Character("w".into()),
                PhysicalKey::Code(WinitKeyCode::KeyW),
                Some("w"),
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();

            assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
            assert_eq!(app.active_tab_id(), active_tab);
            assert_eq!(
                app.close_confirmation
                    .as_ref()
                    .map(|confirmation| confirmation.target.clone()),
                Some(WindowCloseTarget::Tab(active_tab))
            );
            assert_eq!(
                app.effective_window_title(),
                "R-Term [workspace:1 tab:2 pane:2] - Close Current Tab? Enter/Y=yes Esc/N=no"
            );
            assert!(!app.window_close_requested_for_test());
        }
    }

    #[test]
    fn window_app_accepts_native_close_current_tab_confirmation_with_y() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("python.exe"));
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::CloseCurrentTab { confirm: true }));

        assert!(
            app.handle_close_confirmation_key(&Key::Character("y".into()), ModifiersState::empty())
        );

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert!(app.close_confirmation.is_none());
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_close_current_tab_confirmation_honors_last_active_config() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("python.exe"));
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(3),
        })
        .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            switch_to_last_active_tab_when_closing_tab: Some(true),
            ..NativeConfigSnapshot::default()
        });

        assert!(app.command_palette_execute(WindowCommand::CloseCurrentTab { confirm: true }));
        assert!(
            app.handle_close_confirmation_key(
                &Key::Named(NamedKey::Enter),
                ModifiersState::empty()
            )
        );

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert!(app.close_confirmation.is_none());
    }

    #[test]
    fn window_app_window_close_request_prompts_for_stateful_process_by_default() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("python.exe"));

        app.handle_window_close_requested();

        assert!(!app.window_close_requested_for_test());
        assert_eq!(
            app.close_confirmation
                .as_ref()
                .map(|confirmation| confirmation.target.clone()),
            Some(WindowCloseTarget::Window)
        );
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Close Window? Enter/Y=yes Esc/N=no"
        );
    }

    #[test]
    fn window_app_window_close_request_skips_default_stateless_process_names() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("C:\\Windows\\System32\\cmd.exe"),
        );

        app.handle_window_close_requested();

        assert!(app.window_close_requested_for_test());
        assert!(app.close_confirmation.is_none());
    }

    #[test]
    fn window_app_accepts_window_close_confirmation() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("python.exe"));

        app.handle_window_close_requested();
        assert!(
            app.handle_close_confirmation_key(
                &Key::Named(NamedKey::Enter),
                ModifiersState::empty()
            )
        );

        assert!(app.window_close_requested_for_test());
        assert!(app.close_confirmation.is_none());
    }

    #[test]
    fn window_app_close_current_tab_confirmation_skips_configured_stateless_process() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("python.exe"));
        app.dispatch_app_action(AppAction::NewTab {
            launch: Some(PaneLaunch::local("top")),
        })
        .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            skip_close_confirmation_for_processes_named: Some(vec!["top".to_owned()]),
            ..NativeConfigSnapshot::default()
        });

        assert!(app.command_palette_execute(WindowCommand::CloseCurrentTab { confirm: true }));

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert!(app.close_confirmation.is_none());
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_window_close_confirmation_never_prompt_requests_close_immediately() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            window_close_confirmation: Some(NativeWindowCloseConfirmation::NeverPrompt),
            ..NativeConfigSnapshot::default()
        });

        app.handle_window_close_requested();

        assert!(app.window_close_requested_for_test());
        assert!(app.close_confirmation.is_none());
    }

    #[test]
    fn window_app_default_exit_behavior_closes_exited_pane_only() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.pane_ids().len(), 2);

        let close_window = app
            .apply_pane_exit_behavior(rterm_core::PaneId::new(2), &PtyExitStatus::from_exit_code(0));

        assert!(!close_window);
        assert_eq!(app.app_shell.pane_ids(), vec![rterm_core::PaneId::new(1)]);
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_exit_behavior_hold_keeps_exited_pane() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            exit_behavior: Some(NativeExitBehavior::Hold),
            ..NativeConfigSnapshot::default()
        });

        let close_window = app
            .apply_pane_exit_behavior(rterm_core::PaneId::new(1), &PtyExitStatus::from_exit_code(0));

        assert!(!close_window);
        assert_eq!(app.app_shell.pane_ids(), vec![rterm_core::PaneId::new(1)]);
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_exit_behavior_messaging_verbose_reports_held_exit_status() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("false"));
        app.handle_pty_output(b"ready\r\n").unwrap();
        app.set_config_overrides(native_config_snapshot! {
            exit_behavior: Some(NativeExitBehavior::Hold),
            ..NativeConfigSnapshot::default()
        });

        let close_window = app
            .apply_pane_exit_behavior(rterm_core::PaneId::new(1), &PtyExitStatus::from_exit_code(7));

        assert!(!close_window);
        assert!(
            snapshot_row_text(&app.snapshot, 1, TERMINAL_COLUMNS)
                .contains("Process \"false\" in domain \"local\" didn't exit cleanly")
        );
        assert!(
            snapshot_row_text(&app.snapshot, 2, TERMINAL_COLUMNS).contains("Exited with code 7")
        );
    }

    #[test]
    fn window_app_exit_behavior_messaging_verbose_uses_wezterm_failed_message_prefix() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("false"));
        app.set_config_overrides(native_config_snapshot! {
            exit_behavior: Some(NativeExitBehavior::Hold),
            ..NativeConfigSnapshot::default()
        });

        let message = app
            .exit_behavior_message(rterm_core::PaneId::new(1), &PtyExitStatus::from_exit_code(7))
            .expect("expected held-pane exit message");

        assert!(
            message
                .starts_with("⚠️  Process \"false\" in domain \"local\" didn't exit cleanly\r\n")
        );
        assert!(message.contains("\r\nThis message is shown because exit_behavior=\"Hold\""));
    }

    #[test]
    fn window_app_exit_behavior_messaging_verbose_uses_wezterm_success_message_prefix() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("true"));
        app.set_config_overrides(native_config_snapshot! {
            exit_behavior: Some(NativeExitBehavior::Hold),
            ..NativeConfigSnapshot::default()
        });

        let message = app
            .exit_behavior_message(rterm_core::PaneId::new(1), &PtyExitStatus::from_exit_code(0))
            .expect("expected held-pane exit message");

        assert!(message.starts_with("👍 Process \"true\" in domain \"local\" completed.\r\n"));
        assert!(message.contains("\r\nThis message is shown because exit_behavior=\"Hold\""));
    }

    #[test]
    fn window_app_exit_behavior_messaging_verbose_reports_actual_hold_reason() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("false"));
        app.handle_pty_output(b"ready\r\n").unwrap();
        app.set_config_overrides(native_config_snapshot! {
            exit_behavior: Some(NativeExitBehavior::CloseOnCleanExit),
            ..NativeConfigSnapshot::default()
        });

        let close_window = app
            .apply_pane_exit_behavior(rterm_core::PaneId::new(1), &PtyExitStatus::from_exit_code(7));

        assert!(!close_window);
        assert!(
            snapshot_row_text(&app.snapshot, 3, TERMINAL_COLUMNS)
                .contains("exit_behavior=\"CloseOnCleanExit\"")
        );
    }

    #[test]
    fn window_app_exit_behavior_messaging_terse_reports_failed_status() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("false"));
        app.handle_pty_output(b"ready\r\n").unwrap();
        app.set_config_overrides(native_config_snapshot! {
            exit_behavior: Some(NativeExitBehavior::Hold),
            exit_behavior_messaging: Some(NativeExitBehaviorMessaging::Terse),
            ..NativeConfigSnapshot::default()
        });

        let close_window = app
            .apply_pane_exit_behavior(rterm_core::PaneId::new(1), &PtyExitStatus::from_exit_code(7));

        assert!(!close_window);
        assert!(
            snapshot_row_text(&app.snapshot, 1, TERMINAL_COLUMNS).contains("[Exited with code 7]")
        );
    }

    #[test]
    fn window_app_exit_behavior_messaging_terse_reports_clean_status() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("true"));
        app.handle_pty_output(b"ready\r\n").unwrap();
        app.set_config_overrides(native_config_snapshot! {
            exit_behavior: Some(NativeExitBehavior::Hold),
            exit_behavior_messaging: Some(NativeExitBehaviorMessaging::Terse),
            ..NativeConfigSnapshot::default()
        });

        let close_window = app
            .apply_pane_exit_behavior(rterm_core::PaneId::new(1), &PtyExitStatus::from_exit_code(0));

        assert!(!close_window);
        assert!(snapshot_row_text(&app.snapshot, 1, TERMINAL_COLUMNS).contains("[done]"));
    }

    #[test]
    fn window_app_exit_behavior_messaging_brief_reports_clean_process() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("true"));
        app.handle_pty_output(b"ready\r\n").unwrap();
        app.set_config_overrides(native_config_snapshot! {
            exit_behavior: Some(NativeExitBehavior::Hold),
            exit_behavior_messaging: Some(NativeExitBehaviorMessaging::Brief),
            ..NativeConfigSnapshot::default()
        });

        let close_window = app
            .apply_pane_exit_behavior(rterm_core::PaneId::new(1), &PtyExitStatus::from_exit_code(0));

        assert!(!close_window);
        assert!(
            snapshot_row_text(&app.snapshot, 1, TERMINAL_COLUMNS)
                .contains("Process \"true\" in domain \"local\" completed.")
        );
    }

    #[test]
    fn window_app_exit_behavior_messaging_none_suppresses_held_exit_status() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"ready\r\n").unwrap();
        app.set_config_overrides(native_config_snapshot! {
            exit_behavior: Some(NativeExitBehavior::Hold),
            exit_behavior_messaging: Some(NativeExitBehaviorMessaging::None),
            ..NativeConfigSnapshot::default()
        });

        let close_window = app
            .apply_pane_exit_behavior(rterm_core::PaneId::new(1), &PtyExitStatus::from_exit_code(7));

        assert!(!close_window);
        assert!(
            !snapshot_row_text(&app.snapshot, 1, TERMINAL_COLUMNS)
                .contains("Process exited with status 7")
        );
    }

    #[test]
    fn window_app_exit_behavior_close_on_clean_exit_holds_failed_exit() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            exit_behavior: Some(NativeExitBehavior::CloseOnCleanExit),
            ..NativeConfigSnapshot::default()
        });

        let close_window = app
            .apply_pane_exit_behavior(rterm_core::PaneId::new(1), &PtyExitStatus::from_exit_code(7));

        assert!(!close_window);
        assert_eq!(app.app_shell.pane_ids(), vec![rterm_core::PaneId::new(1)]);
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_clean_exit_codes_default_does_not_treat_130_as_clean() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            exit_behavior: Some(NativeExitBehavior::CloseOnCleanExit),
            ..NativeConfigSnapshot::default()
        });

        let close_window = app.apply_pane_exit_behavior(
            rterm_core::PaneId::new(1),
            &PtyExitStatus::from_exit_code(130),
        );

        assert!(!close_window);
        assert_eq!(app.app_shell.pane_ids(), vec![rterm_core::PaneId::new(1)]);
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_clean_exit_codes_close_custom_clean_exit() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            exit_behavior: Some(NativeExitBehavior::CloseOnCleanExit),
            clean_exit_codes: Some(vec![130]),
            ..NativeConfigSnapshot::default()
        });

        let close_window = app.apply_pane_exit_behavior(
            rterm_core::PaneId::new(1),
            &PtyExitStatus::from_exit_code(130),
        );

        assert!(close_window);
        assert!(app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_exit_behavior_close_on_clean_exit_holds_unknown_exit() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            exit_behavior: Some(NativeExitBehavior::CloseOnCleanExit),
            ..NativeConfigSnapshot::default()
        });

        let close_window = app.apply_pane_exit_behavior_after_exit(rterm_core::PaneId::new(1), None);

        assert!(!close_window);
        assert_eq!(app.app_shell.pane_ids(), vec![rterm_core::PaneId::new(1)]);
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_exit_behavior_close_on_clean_exit_closes_clean_exit() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            exit_behavior: Some(NativeExitBehavior::CloseOnCleanExit),
            ..NativeConfigSnapshot::default()
        });

        let close_window = app
            .apply_pane_exit_behavior(rterm_core::PaneId::new(1), &PtyExitStatus::from_exit_code(0));

        assert!(close_window);
        assert!(app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_frame_limit_defers_short_child_automatic_close() {
        let mut app = NativeWindowApp::new(Some(10));
        let close_window = app
            .apply_pane_exit_behavior(rterm_core::PaneId::new(1), &PtyExitStatus::from_exit_code(0));

        assert!(close_window);
        assert!(!app.defer_automatic_close_for_frame_limit(close_window));
        assert!(!app.window_close_requested_for_test());
        assert!(app.frame_limit_redraw_pending());
    }

    #[test]
    fn window_app_frame_limit_waits_for_requested_linkage_after_limit() {
        let mut app = NativeWindowApp::new(Some(10));
        app.metrics.pty_linkage_enabled = true;
        app.rendered_frames = 10;

        assert!(!app.frame_limit_probe_ready());
        assert!(app.frame_limit_probe_pending());

        app.metrics.terminal_linkage_nonce_found = true;

        assert!(app.frame_limit_probe_ready());
        assert!(!app.frame_limit_probe_pending());
    }

    #[test]
    fn window_app_frame_limit_reserves_the_final_frame_for_requested_linkage() {
        let mut app = NativeWindowApp::new(Some(10));
        app.metrics.pty_linkage_enabled = true;
        app.rendered_frames = 9;

        assert!(!app.frame_limit_redraw_pending());
        assert!(app.frame_limit_redraw_deadline(Instant::now()).is_some());

        app.metrics.terminal_linkage_nonce_found = true;

        assert!(app.frame_limit_redraw_pending());
    }

    #[test]
    fn window_app_without_frame_limit_keeps_automatic_close_behavior() {
        let mut app = NativeWindowApp::new(None);
        let close_window = app
            .apply_pane_exit_behavior(rterm_core::PaneId::new(1), &PtyExitStatus::from_exit_code(0));

        assert!(app.defer_automatic_close_for_frame_limit(close_window));
        assert!(app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_palette_close_tab_requests_window_close_for_last_tab() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();

        assert!(app.command_palette_execute(WindowCommand::CloseTab));
        assert!(app.command_palette.is_none());
        assert!(app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_event_loop_exit_check_preserves_manager_owned_close_request() {
        let mut app = NativeWindowApp::new(None);

        app.request_window_close();

        assert!(app.event_loop_exit_requested());
        assert!(app.take_window_close_request());
    }

    #[test]
    fn window_app_dispatches_palette_close_workspace_command() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "next".to_owned(),
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::CloseWorkspace);

        assert_eq!(app.app_shell.workspaces().len(), 1);
        assert_eq!(app.active_workspace_id(), rterm_core::WorkspaceId::new(1));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_rejects_palette_close_workspace_with_single_workspace() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        let error = app
            .command_palette_apply_command(WindowCommand::CloseWorkspace)
            .unwrap_err();

        assert_eq!(error, AppShellError::CannotCloseLastWorkspace);
        assert_eq!(app.app_shell.workspaces().len(), 1);
        assert!(app.command_palette.is_some());
    }

    #[test]
    fn window_app_native_search_current_selection_joins_multiline_selection() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 3));
        app.handle_pty_output(b"alpha\r\nbeta\r\nalpha beta")
            .unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 1, column: 3 },
        );
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::Search(
            WindowSearchCommandQuery::CurrentSelectionOrEmptyString,
        ));

        let search = active_search_for_test(&app);
        assert_eq!(search.query, "alpha beta");
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 2, column: 0 },
                SelectionCell { row: 2, column: 9 },
            ))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_enter_search_command() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::EnterSearch);

        assert!(search_for_test(&app).is_some());
        assert!(app.command_palette.is_none());
        assert_app_search_mode(&app);
        assert!(quick_select_for_test(&app).is_none());
    }

    #[test]
    fn window_app_exposes_wezterm_search_command() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("search".to_owned());
        let command = app
            .command_palette_filtered_commands()
            .into_iter()
            .find(|command| command.label() == "Search")
            .expect("expected search command");
        app.command_palette_execute(command);

        assert!(search_for_test(&app).is_some());
        assert!(app.command_palette.is_none());
        assert_app_search_mode(&app);
        assert!(quick_select_for_test(&app).is_none());
    }

    #[test]
    fn window_app_dispatches_palette_search_command_with_query_pattern() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"alpha beta").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("search alpha".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected search command");
        app.command_palette_execute(command);

        let search = active_search_for_test(&app);
        assert_eq!(search.query, "alpha");
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 4 },
            ))
        );
        assert!(app.command_palette.is_none());
        assert_app_search_mode(&app);
        assert!(quick_select_for_test(&app).is_none());
    }

    #[test]
    fn window_app_dispatches_palette_search_command_with_quoted_query_pattern() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"alpha beta").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("search \"alpha beta\"".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected search command");
        app.command_palette_execute(command);

        let search = active_search_for_test(&app);
        assert_eq!(search.query, "alpha beta");
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 9 },
            ))
        );
    }

    #[test]
    fn window_app_dispatches_palette_search_command_with_regex_query_pattern() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"alpha 123").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("search regex \\d+".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected search command");
        app.command_palette_execute(command);

        let search = active_search_for_test(&app);
        assert_eq!(search.query, "\\d+");
        assert_eq!(search.match_type, WindowSearchMatchType::Regex);
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 6 },
                SelectionCell { row: 0, column: 8 },
            ))
        );
    }

    #[test]
    fn window_app_dispatches_palette_search_equals_regex_query_pattern() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"alpha 123").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("search=regex \\d+".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected search command");
        app.command_palette_execute(command);

        let search = active_search_for_test(&app);
        assert_eq!(search.query, "\\d+");
        assert_eq!(search.match_type, WindowSearchMatchType::Regex);
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 6 },
                SelectionCell { row: 0, column: 8 },
            ))
        );
    }

    #[test]
    fn window_app_dispatches_palette_search_mixed_case_regex_query_pattern() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"alpha 123").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("Search Regex \\d+".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected search command");
        app.command_palette_execute(command);

        let search = active_search_for_test(&app);
        assert_eq!(search.query, "\\d+");
        assert_eq!(search.match_type, WindowSearchMatchType::Regex);
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 6 },
                SelectionCell { row: 0, column: 8 },
            ))
        );
    }

    #[test]
    fn window_app_dispatches_palette_search_wezterm_action_table_queries() {
        for (query, expected_pattern, expected_match_type, expected_selection) in [
            (
                "wezterm.action.Search { Regex = '\\\\d+' }",
                "\\d+",
                WindowSearchMatchType::Regex,
                WindowSelection::new(
                    SelectionCell { row: 0, column: 6 },
                    SelectionCell { row: 0, column: 8 },
                ),
            ),
            (
                "wezterm.action.Search { CaseSensitiveString = 'Alpha' }",
                "Alpha",
                WindowSearchMatchType::CaseSensitive,
                WindowSelection::new(
                    SelectionCell { row: 0, column: 0 },
                    SelectionCell { row: 0, column: 4 },
                ),
            ),
            (
                "wezterm.action.Search { CaseInSensitiveString = 'alpha' }",
                "alpha",
                WindowSearchMatchType::CaseInsensitive,
                WindowSelection::new(
                    SelectionCell { row: 0, column: 10 },
                    SelectionCell { row: 0, column: 14 },
                ),
            ),
        ] {
            let mut app = NativeWindowApp::new(None);
            app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
            app.handle_pty_output(b"Alpha 123 alpha").unwrap();

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let commands = app.command_palette_filtered_commands();
            let command = commands.first().cloned().expect("expected search command");
            app.command_palette_execute(command);

            let search = active_search_for_test(&app);
            assert_eq!(search.query, expected_pattern);
            assert_eq!(search.match_type, expected_match_type);
            assert_eq!(app.selection, Some(expected_selection));
            assert!(app.command_palette.is_none());
            assert_app_search_mode(&app);
            assert!(quick_select_for_test(&app).is_none());
        }
    }

    #[test]
    fn window_app_dispatches_palette_search_wezterm_action_table_trailing_comma_query() {
        for query in [
            "wezterm.action.Search { Regex = 'ticket-[0-9]+', }",
            "wezterm.action.Search({ Regex = 'ticket-[0-9]+', })",
        ] {
            let mut app = NativeWindowApp::new(None);
            app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
            app.handle_pty_output(b"ticket-7 done").unwrap();

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let commands = app.command_palette_filtered_commands();
            let command = commands.first().cloned().expect("expected search command");
            app.command_palette_execute(command);

            let search = active_search_for_test(&app);
            assert_eq!(search.query, "ticket-[0-9]+");
            assert_eq!(search.match_type, WindowSearchMatchType::Regex);
            assert_eq!(
                app.selection,
                Some(WindowSelection::new(
                    SelectionCell { row: 0, column: 0 },
                    SelectionCell { row: 0, column: 7 },
                ))
            );
            assert!(app.command_palette.is_none());
            assert_app_search_mode(&app);
            assert!(quick_select_for_test(&app).is_none());
        }
    }

    #[test]
    fn window_app_dispatches_palette_search_wezterm_action_table_long_bracket_key_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"Alpha 123 alpha").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Search { [[=[Regex]=]] = [[\\d+]] }".to_owned(),
        );
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected search command");
        app.command_palette_execute(command);

        let search = active_search_for_test(&app);
        assert_eq!(search.query, "\\d+");
        assert_eq!(search.match_type, WindowSearchMatchType::Regex);
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 6 },
                SelectionCell { row: 0, column: 8 },
            ))
        );
        assert!(app.command_palette.is_none());
        assert_app_search_mode(&app);
        assert!(quick_select_for_test(&app).is_none());
    }

    #[test]
    fn window_app_dispatches_palette_search_regex_pattern_assignment_text() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"pattern=123").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("search regex pattern=\\d+".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, vec![WindowCommand::EnterSearch]);
        app.command_palette_execute(commands[0].clone());

        let search = active_search_for_test(&app);
        assert_eq!(search.query, "pattern=\\d+");
        assert_eq!(search.match_type, WindowSearchMatchType::Regex);
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 10 },
            ))
        );
        assert!(app.command_palette.is_none());
        assert_app_search_mode(&app);
        assert!(quick_select_for_test(&app).is_none());
    }

    #[test]
    fn window_app_dispatches_native_search_action_payload() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"alpha 123").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::Search(WindowSearchCommandQuery::Pattern {
            pattern: "\\d+".to_owned(),
            match_type: WindowSearchMatchType::Regex,
        }));

        let search = active_search_for_test(&app);
        assert_eq!(search.query, "\\d+");
        assert_eq!(search.match_type, WindowSearchMatchType::Regex);
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 6 },
                SelectionCell { row: 0, column: 8 },
            ))
        );
        assert!(app.command_palette.is_none());
        assert_app_search_mode(&app);
        assert!(quick_select_for_test(&app).is_none());
    }

    #[test]
    fn window_app_search_action_selects_bottom_most_initial_match() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 3));
        app.handle_pty_output(b"top hit\r\nmiddle\r\nbottom hit")
            .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::Search(WindowSearchCommandQuery::Pattern {
            pattern: "hit".to_owned(),
            match_type: WindowSearchMatchType::CaseSensitive,
        }));

        let search = active_search_for_test(&app);
        assert_eq!(search.query, "hit");
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 2, column: 7 },
                SelectionCell { row: 2, column: 9 },
            ))
        );
    }

    #[test]
    fn window_app_dispatches_native_search_current_selection_payload() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"alpha beta alpha").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 6 },
            SelectionCell { row: 0, column: 9 },
        );
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::Search(
            WindowSearchCommandQuery::CurrentSelectionOrEmptyString,
        ));

        let search = active_search_for_test(&app);
        assert_eq!(search.query, "beta");
        assert_eq!(search.match_type, WindowSearchMatchType::CaseSensitive);
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 6 },
                SelectionCell { row: 0, column: 9 },
            ))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_search_wezterm_action_current_selection_string_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"alpha beta alpha").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 6 },
            SelectionCell { row: 0, column: 9 },
        );
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Search(\"CurrentSelectionOrEmptyString\")".to_owned(),
        );
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected search command");
        app.command_palette_execute(command);

        let search = active_search_for_test(&app);
        assert_eq!(search.query, "beta");
        assert_eq!(search.match_type, WindowSearchMatchType::CaseSensitive);
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 6 },
                SelectionCell { row: 0, column: 9 },
            ))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_search_wezterm_action_bare_string_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"alpha beta alpha").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 6 },
            SelectionCell { row: 0, column: 9 },
        );
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Search 'CurrentSelectionOrEmptyString'".to_owned(),
        );
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected search command");
        app.command_palette_execute(command);

        let search = active_search_for_test(&app);
        assert_eq!(search.query, "beta");
        assert_eq!(search.match_type, WindowSearchMatchType::CaseSensitive);
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 6 },
                SelectionCell { row: 0, column: 9 },
            ))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_search_current_selection_payload_without_selection() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"alpha beta").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::Search(
            WindowSearchCommandQuery::CurrentSelectionOrEmptyString,
        ));

        let search = active_search_for_test(&app);
        assert!(search.query.is_empty());
        assert!(app.selection.is_none());
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_search_command_with_case_insensitive_query_pattern() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"Alpha beta").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("search case-insensitive alpha".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected search command");
        app.command_palette_execute(command);

        let search = active_search_for_test(&app);
        assert_eq!(search.query, "alpha");
        assert_eq!(search.match_type, WindowSearchMatchType::CaseInsensitive);
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 4 },
            ))
        );
    }

    #[test]
    fn window_app_dispatches_palette_search_command_with_case_insensitive_spaced_query_pattern() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"Alpha beta").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("search case insensitive alpha".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected search command");
        app.command_palette_execute(command);

        let search = active_search_for_test(&app);
        assert_eq!(search.query, "alpha");
        assert_eq!(search.match_type, WindowSearchMatchType::CaseInsensitive);
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 4 },
            ))
        );
    }

    #[test]
    fn window_app_dispatches_palette_search_action_name_pattern_queries() {
        for (query, expected_pattern, expected_match_type) in [
            (
                "search casesensitivestring alpha",
                "alpha",
                WindowSearchMatchType::CaseSensitive,
            ),
            (
                "search caseinsensitivestring alpha",
                "alpha",
                WindowSearchMatchType::CaseInsensitive,
            ),
            (
                "search caseinsensitivestring \"Alpha beta\"",
                "Alpha beta",
                WindowSearchMatchType::CaseInsensitive,
            ),
        ] {
            let mut app = NativeWindowApp::new(None);
            app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
            app.handle_pty_output(b"Alpha beta").unwrap();

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());
            let commands = app.command_palette_filtered_commands();
            let command = commands.first().cloned().expect("expected search command");
            app.command_palette_execute(command);

            let search = active_search_for_test(&app);
            assert_eq!(search.query, expected_pattern);
            assert_eq!(search.match_type, expected_match_type);
        }
    }

    #[test]
    fn window_app_dispatches_palette_search_current_selection_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"alpha beta alpha").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 6 },
            SelectionCell { row: 0, column: 9 },
        );
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query("search current selection or empty string".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, vec![WindowCommand::EnterSearch]);
        app.command_palette_execute(commands[0].clone());

        let search = active_search_for_test(&app);
        assert_eq!(search.query, "beta");
        assert_eq!(search.match_type, WindowSearchMatchType::CaseSensitive);
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 6 },
                SelectionCell { row: 0, column: 9 },
            ))
        );
    }

    #[test]
    fn window_app_dispatches_palette_search_command_with_case_sensitive_query_pattern() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"Alpha alpha").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("search case-sensitive alpha".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected search command");
        app.command_palette_execute(command);

        let search = active_search_for_test(&app);
        assert_eq!(search.query, "alpha");
        assert_eq!(search.match_type, WindowSearchMatchType::CaseSensitive);
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 6 },
                SelectionCell { row: 0, column: 10 },
            ))
        );
    }

    #[test]
    fn window_app_dispatches_palette_clear_scrollback_command() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 1);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "cd  ");
        assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "ef  ");
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ClearScrollback(
            WindowClearScrollbackMode::ScrollbackOnly,
        ));

        assert!(app.command_palette.is_none());
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.runtime.terminal().scrollback().is_empty());
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "cd  ");
        assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "ef  ");
        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_dispatches_native_clear_scrollback_scrollback_only_payload() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 1);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "cd  ");
        assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "ef  ");
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ClearScrollback(
            WindowClearScrollbackMode::ScrollbackOnly,
        ));

        assert!(app.command_palette.is_none());
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.runtime.terminal().scrollback().is_empty());
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "cd  ");
        assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "ef  ");
        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_dispatches_native_clear_scrollback_and_viewport_payload() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 1);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "cd  ");
        assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "ef  ");
        assert_eq!(app.runtime.terminal().cursor(), (1, 2));
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ClearScrollback(
            WindowClearScrollbackMode::ScrollbackAndViewport,
        ));

        assert!(app.command_palette.is_none());
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.runtime.terminal().scrollback().is_empty());
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "ef  ");
        assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "    ");
        assert_eq!(app.runtime.terminal().cursor(), (0, 2));
        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_dispatches_palette_clear_scrollback_scrollback_only_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 1);
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query("clear scrollback scrollback only".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(
            commands,
            [WindowCommand::ClearScrollback(
                WindowClearScrollbackMode::ScrollbackOnly
            )]
        );
        app.command_palette_execute(commands[0].clone());

        assert!(app.command_palette.is_none());
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.runtime.terminal().scrollback().is_empty());
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "cd  ");
        assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "ef  ");
        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_dispatches_palette_clear_scrollback_and_viewport_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 1);
        assert_eq!(app.runtime.terminal().cursor(), (1, 2));
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query("clear scrollback scrollback and viewport".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(
            commands,
            [WindowCommand::ClearScrollback(
                WindowClearScrollbackMode::ScrollbackAndViewport
            )]
        );
        app.command_palette_execute(commands[0].clone());

        assert!(app.command_palette.is_none());
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.runtime.terminal().scrollback().is_empty());
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "ef  ");
        assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "    ");
        assert_eq!(app.runtime.terminal().cursor(), (0, 2));
        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_dispatches_palette_clear_scrollback_quoted_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("clear scrollback \"scrollback and viewport\"".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            [WindowCommand::ClearScrollback(
                WindowClearScrollbackMode::ScrollbackAndViewport
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_clear_scrollback_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("clear scrollback=scrollback only".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            [WindowCommand::ClearScrollback(
                WindowClearScrollbackMode::ScrollbackOnly
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_clear_scrollback_compact_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("clearscrollback=scrollback and viewport".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            [WindowCommand::ClearScrollback(
                WindowClearScrollbackMode::ScrollbackAndViewport
            )]
        );
    }

    #[test]
    fn window_app_tab_session_config_prefers_new_values_and_maps_legacy_values() {
        let mut app = NativeWindowApp::new(None);
        let overrides = NativeConfigSnapshot {
            tab_min_width: Some(11),
            next: crate::window::NativeConfigSnapshot1 {
                next: crate::window::NativeConfigSnapshot2 {
                    next: crate::window::NativeConfigSnapshot3 {
                        next: crate::window::NativeConfigSnapshot4 {
                            tab_shortcut_style: Some(
                                crate::window::NativeTabShortcutStyle::Browser,
                            ),
                            closed_tab_history_size: Some(7),
                            close_tab_selection: Some(
                                rterm_core::app_shell::CloseTabSelection::Adjacent,
                            ),
                            tab_bar_wheel_behavior: Some(
                                crate::window::NativeTabBarWheelBehavior::Scroll,
                            ),
                            mouse_wheel_scrolls_tabs: Some(true),
                            switch_to_last_active_tab_when_closing_tab: Some(true),
                            ..crate::window::NativeConfigSnapshot4::default()
                        },
                        ..crate::window::NativeConfigSnapshot3::default()
                    },
                    ..crate::window::NativeConfigSnapshot2::default()
                },
                ..crate::window::NativeConfigSnapshot1::default()
            },
            ..NativeConfigSnapshot::default()
        };

        app.apply_config_overrides_silently(overrides);

        assert_eq!(app.tab_min_width, 11);
        assert_eq!(
            app.tab_shortcut_style,
            crate::window::NativeTabShortcutStyle::Browser
        );
        assert_eq!(app.closed_tab_history_size, 7);
        assert_eq!(
            app.close_tab_selection,
            rterm_core::app_shell::CloseTabSelection::Adjacent
        );
        assert_eq!(
            app.tab_bar_wheel_behavior,
            crate::window::NativeTabBarWheelBehavior::Scroll
        );
        assert_eq!(app.closed_tab_history.lock().unwrap().capacity(), 7);

        let legacy = NativeConfigSnapshot {
            next: crate::window::NativeConfigSnapshot1 {
                next: crate::window::NativeConfigSnapshot2 {
                    next: crate::window::NativeConfigSnapshot3 {
                        next: crate::window::NativeConfigSnapshot4 {
                            mouse_wheel_scrolls_tabs: Some(false),
                            switch_to_last_active_tab_when_closing_tab: Some(true),
                            ..crate::window::NativeConfigSnapshot4::default()
                        },
                        ..crate::window::NativeConfigSnapshot3::default()
                    },
                    ..crate::window::NativeConfigSnapshot2::default()
                },
                ..crate::window::NativeConfigSnapshot1::default()
            },
            ..NativeConfigSnapshot::default()
        };
        app.apply_config_overrides_silently(legacy);

        assert_eq!(
            app.tab_bar_wheel_behavior,
            crate::window::NativeTabBarWheelBehavior::Disabled
        );
        assert_eq!(
            app.close_tab_selection,
            rterm_core::app_shell::CloseTabSelection::LastActive
        );
    }

    #[test]
    fn window_app_browser_tab_shortcuts_open_launcher_reopen_and_activate_tabs() {
        let mut app = NativeWindowApp::new(None);
        app.tab_shortcut_style = crate::window::NativeTabShortcutStyle::Browser;

        assert!(app.handle_browser_tab_shortcut_event(
            &Key::Character("t".into()),
            PhysicalKey::Code(WinitKeyCode::KeyT),
            ModifiersState::CONTROL,
            false,
        ));
        assert_eq!(
            app.command_palette
                .as_ref()
                .expect("Ctrl+T should open the session launcher")
                .title(),
            "Launcher"
        );

        app.command_palette = None;
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .expect("expected second tab");
        let second = app.active_tab_id();
        assert!(app.handle_browser_tab_shortcut_event(
            &Key::Character("1".into()),
            PhysicalKey::Code(WinitKeyCode::Digit1),
            ModifiersState::CONTROL,
            false,
        ));
        assert_ne!(app.active_tab_id(), second);
    }

    #[test]
    fn window_app_parses_tab_session_configuration_fields() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            config.tab_min_width = 10
            config.tab_shortcut_style = 'browser'
            config.closed_tab_history_size = 12
            config.close_tab_selection = 'last-active'
            config.tab_bar_wheel_behavior = 'scroll'
            return config
            "#,
        )
        .expect("expected tab-session config values");

        assert_eq!(overrides.tab_min_width, Some(10));
        assert_eq!(
            overrides.tab_shortcut_style,
            Some(crate::window::NativeTabShortcutStyle::Browser)
        );
        assert_eq!(overrides.closed_tab_history_size, Some(12));
        assert_eq!(
            overrides.close_tab_selection,
            Some(rterm_core::app_shell::CloseTabSelection::LastActive)
        );
        assert_eq!(
            overrides.tab_bar_wheel_behavior,
            Some(crate::window::NativeTabBarWheelBehavior::Scroll)
        );
    }

    #[test]
    fn window_app_tab_context_menu_exposes_browser_tab_actions() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .expect("expected second tab");

        app.enter_tab_context_menu(rterm_core::TabId::new(1))
            .expect("expected tab context menu");

        assert_eq!(
            app.command_palette
                .as_ref()
                .expect("context menu should be visible")
                .title(),
            "Tab Actions"
        );
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![
                WindowCommand::NewTab,
                WindowCommand::DuplicateTab,
                WindowCommand::RenameTab,
                WindowCommand::MoveTabToNewWindow,
                WindowCommand::CloseTab,
                WindowCommand::CloseOtherTabs,
                WindowCommand::CloseTabsToRight,
                WindowCommand::ReopenClosedTab,
            ]
        );
    }

    #[test]
    fn window_app_tab_bar_wheel_scrolls_headers_without_switching_sessions() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..3 {
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .expect("expected additional tab");
        }
        let active = app.active_tab_id();
        app.tab_bar_wheel_behavior = crate::window::NativeTabBarWheelBehavior::Scroll;

        assert!(app.handle_tab_bar_mouse_wheel(MouseScrollDelta::LineDelta(
            0.0, -1.0,
        )));
        assert_eq!(app.active_tab_id(), active);
        assert_eq!(app.tab_bar_scroll_position, 1);
    }

    #[test]
    fn window_app_moving_its_final_tab_to_new_window_requests_source_close() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::MoveTabToNewWindow {
            tab: app.active_tab_id(),
        })
        .expect("moving a final tab should prepare a replacement window");

        assert!(app.window_close_requested_for_test());
        let detached = app
            .take_next_pending_window_app()
            .expect("the moved tab should retain its live runtime in a pending app");
        assert_eq!(detached.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(detached.active_pane_id(), rterm_core::PaneId::new(1));
    }

    #[test]
    fn window_app_batch_tab_close_uses_one_confirmation_for_the_whole_set() {
        let mut app = NativeWindowApp::new(None);
        app.skip_close_confirmation_for_processes_named.clear();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .expect("expected second tab");
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .expect("expected third tab");

        app.command_palette_apply_command(WindowCommand::CloseOtherTabs)
            .expect("expected batch close request");
        assert!(app.close_confirmation.is_some());
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 3);

        app.accept_close_confirmation();
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
    }

    #[test]
    fn window_app_parses_move_tab_to_window_as_a_bindable_parameterized_command() {
        assert_eq!(
            super::command_palette_structured_query_command("MoveTabToWindow(42)"),
            Some(WindowCommand::MoveTabToWindow(rterm_core::WindowId::new(42)))
        );
    }
