    #[test]
    fn window_app_mouse_binding_double_left_down_matches_second_click() {
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
                event = { Down = { streak = 2, button = 'Left' } },
                mods = 'NONE',
                action = act.PastePrimarySelection,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm double-click mouse binding config");
        app.set_config_overrides(overrides);
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();

        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert!(written.lock().unwrap().is_empty());
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        let expected = encode_window_paste("primary", false, DEFAULT_CANONICALIZE_PASTED_NEWLINES);
        assert_eq!(written.lock().unwrap().as_slice(), expected.as_slice());
    }

    #[test]
    fn window_app_mouse_binding_double_middle_down_matches_second_click() {
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
                event = { Down = { streak = 2, button = 'Middle' } },
                mods = 'ALT',
                action = act.PastePrimarySelection,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm double-click middle mouse binding config");
        app.set_config_overrides(overrides);
        app.modifiers = ModifiersState::ALT;
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();

        assert!(
            !app.handle_mouse_input(ElementState::Pressed, MouseButton::Middle)
                .unwrap()
        );
        assert!(written.lock().unwrap().is_empty());
        assert!(
            !app.handle_mouse_input(ElementState::Released, MouseButton::Middle)
                .unwrap()
        );
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Middle)
                .unwrap()
        );

        let expected = encode_window_paste("primary", false, DEFAULT_CANONICALIZE_PASTED_NEWLINES);
        assert_eq!(written.lock().unwrap().as_slice(), expected.as_slice());
    }

    #[test]
    fn window_app_disable_default_mouse_bindings_suppresses_default_mouse_actions() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.primary_selection_reader = Box::new(|| Some("primary".to_owned()));
        app.set_config_overrides(native_config_snapshot! {
            disable_default_mouse_bindings: Some(true),
            ..NativeConfigSnapshot::default()
        });

        app.modifiers = ModifiersState::SUPER;
        assert!(
            !app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert!(!app.window_drag_requested_for_test());

        app.modifiers = ModifiersState::empty();
        assert!(
            !app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert!(app.selection.is_none());
        assert!(!app.selecting);

        assert!(
            !app.handle_mouse_input(ElementState::Pressed, MouseButton::Middle)
                .unwrap()
        );
        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_dispatches_native_show_action() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::Hide));
        assert!(app.window_hide_requested_for_test());

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::Show));

        assert!(!app.window_hide_requested_for_test());
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_hide_application_command() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::HideApplication);

        assert!(app.take_application_hide_request());
        assert!(!app.take_application_hide_request());
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_hide_application_request_is_consumed_once() {
        let mut app = NativeWindowApp::new(None);

        app.hide_application();

        assert!(app.take_application_hide_request());
        assert!(!app.take_application_hide_request());
        assert!(!app.window_hide_requested);
    }

    #[test]
    fn window_app_dispatches_palette_quit_application_command() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::QuitApplication);

        assert!(app.application_quit_requested_for_test());
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_font_size_commands() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::IncreaseFontSize);
        assert!((app.font_size_scale_for_test() - 1.1).abs() < f64::EPSILON);
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::DecreaseFontSize);
        assert!((app.font_size_scale_for_test() - 1.0).abs() < f64::EPSILON);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::IncreaseFontSize);
        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ResetFontSize);
        assert!((app.font_size_scale_for_test() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn window_app_font_size_change_adjusts_window_size_by_default() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::IncreaseFontSize);

        assert_eq!(
            app.runtime.terminal().grid().size(),
            rterm_core::TerminalSize::new(TERMINAL_COLUMNS, TERMINAL_ROWS)
        );
        assert_eq!(app.frame_size_for_test(), (800, 500));
        assert_eq!(app.window_frame.width, 800);
        assert_eq!(app.window_frame.height, 500);
        assert!(
            app.native_effective_config()
                .adjust_window_size_when_changing_font_size
        );
    }

    #[test]
    fn window_app_font_size_change_can_keep_window_size_and_adjust_terminal_size() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            adjust_window_size_when_changing_font_size: Some(false),
            ..NativeConfigSnapshot::default()
        });

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::IncreaseFontSize);

        assert_eq!(app.window_frame.width, FRAME_WIDTH);
        assert_eq!(app.window_frame.height, FRAME_HEIGHT);
        assert_eq!(app.frame_size_for_test(), (FRAME_WIDTH, FRAME_HEIGHT));
        assert_eq!(
            app.runtime.terminal().grid().size(),
            rterm_core::TerminalSize::new(72, 21)
        );
        assert!(
            !app.native_effective_config()
                .adjust_window_size_when_changing_font_size
        );
    }

    #[test]
    fn window_app_font_size_override_scales_base_cell_geometry() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            font_size: Some(NativeFontSize::from_millipoints(24_000)),
            ..NativeConfigSnapshot::default()
        });

        assert_eq!(
            app.native_effective_config().font_size,
            NativeFontSize::from_millipoints(24_000)
        );
        assert_eq!(app.cell_width(), 14);
        assert_eq!(app.cell_height(), 29);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::IncreaseFontSize);
        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ResetFontSize);

        assert_eq!(app.cell_width(), 14);
        assert_eq!(app.cell_height(), 29);
    }

    #[test]
    fn window_app_line_height_override_scales_vertical_cell_geometry() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            line_height: Some(NativeLineHeight::from_per_mille(1_500)),
            ..NativeConfigSnapshot::default()
        });

        assert_eq!(
            app.native_effective_config().line_height,
            NativeLineHeight::from_per_mille(1_500)
        );
        assert_eq!(app.cell_width(), CELL_WIDTH);
        assert_eq!(app.cell_height(), CELL_HEIGHT * 3 / 2);
        assert_eq!(
            app.initial_frame_size(),
            PhysicalSize::new(
                FRAME_WIDTH,
                u32::from(TERMINAL_ROWS.saturating_add(TAB_BAR_ROWS)) * app.cell_height(),
            )
        );

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::IncreaseFontSize);
        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ResetFontSize);

        assert_eq!(app.cell_width(), CELL_WIDTH);
        assert_eq!(app.cell_height(), CELL_HEIGHT * 3 / 2);
    }

    #[test]
    fn window_app_cell_width_override_scales_horizontal_cell_geometry() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            cell_width: Some(NativeCellWidth::from_per_mille(1_500)),
            ..NativeConfigSnapshot::default()
        });

        assert_eq!(
            app.native_effective_config().cell_width,
            NativeCellWidth::from_per_mille(1_500)
        );
        assert_eq!(app.cell_width(), (CELL_WIDTH * 3).div_ceil(2));
        assert_eq!(app.cell_height(), CELL_HEIGHT);
        assert_eq!(
            app.initial_frame_size(),
            PhysicalSize::new(u32::from(TERMINAL_COLUMNS) * app.cell_width(), FRAME_HEIGHT,)
        );

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::IncreaseFontSize);
        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ResetFontSize);

        assert_eq!(app.cell_width(), (CELL_WIDTH * 3).div_ceil(2));
        assert_eq!(app.cell_height(), CELL_HEIGHT);
    }

    #[test]
    fn window_app_dispatches_palette_reset_font_and_window_size_command() {
        let mut app = NativeWindowApp::new(None);
        app.handle_window_resize(PhysicalSize::new(96, 80)).unwrap();
        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::IncreaseFontSize);

        assert_ne!(app.frame_size_for_test(), (FRAME_WIDTH, FRAME_HEIGHT));
        assert!((app.font_size_scale_for_test() - 1.1).abs() < f64::EPSILON);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ResetFontAndWindowSize);

        assert_eq!(app.frame_size_for_test(), (FRAME_WIDTH, FRAME_HEIGHT));
        assert!((app.font_size_scale_for_test() - 1.0).abs() < f64::EPSILON);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_reset_font_and_window_size_uses_configured_initial_size() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            initial_cols: Some(100),
            initial_rows: Some(30),
            ..NativeConfigSnapshot::default()
        });
        app.handle_window_resize(PhysicalSize::new(96, 80)).unwrap();
        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::IncreaseFontSize);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ResetFontAndWindowSize);

        assert_eq!(
            app.frame_size_for_test(),
            (
                100 * CELL_WIDTH,
                (30 + u32::from(TAB_BAR_ROWS)) * CELL_HEIGHT
            )
        );
        assert_eq!(
            app.runtime.terminal().grid().size(),
            rterm_core::TerminalSize::new(100, 30)
        );
        assert!((app.font_size_scale_for_test() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn window_app_dispatches_palette_show_debug_overlay_command() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ShowDebugOverlay);

        assert!(app.debug_overlay_active_for_test());
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_renders_debug_overlay_snapshot() {
        let mut app = NativeWindowApp::new(None);

        app.command_palette_execute(WindowCommand::ShowDebugOverlay);

        let snapshot = app.render_snapshot();
        let first_overlay_cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0)
            .expect("expected first debug overlay cell");
        assert_eq!(
            first_overlay_cell.foreground,
            DEFAULT_UI_SURFACE_FOREGROUND
        );
        assert_eq!(
            first_overlay_cell.background,
            DEFAULT_COMMAND_PALETTE_BG_COLOR
        );
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains("Debug Overlay"),
            "first terminal row was {first_terminal_row:?}"
        );
        assert!(
            first_terminal_row.contains("window=1"),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_debug_overlay_renders_recent_logs() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            debug_key_events: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;

        app.handle_keyboard_input_event(
            &Key::Character("A".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            Some("A"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        app.command_palette_execute(WindowCommand::ShowDebugOverlay);

        let snapshot = app.render_snapshot();
        let overlay_text = (0..TERMINAL_ROWS.saturating_add(TAB_BAR_ROWS))
            .map(|row| snapshot_row_text(&snapshot, row, TERMINAL_COLUMNS))
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            overlay_text.contains("Recent Logs"),
            "overlay text was {overlay_text:?}"
        );
        assert!(
            overlay_text.contains("INFO key_event"),
            "overlay text was {overlay_text:?}"
        );
        assert!(
            overlay_text.contains("key: Character(\"A\")"),
            "overlay text was {overlay_text:?}"
        );
    }

    #[test]
    fn window_app_escape_closes_debug_overlay_without_pty_input() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.command_palette_execute(WindowCommand::ShowDebugOverlay);
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Escape),
            PhysicalKey::Code(WinitKeyCode::Escape),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.debug_overlay_active_for_test());
        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_dispatches_palette_char_select_command() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::CharSelect);

        assert!(app.char_select_active_for_test());
        let char_select = app.char_select_for_test().expect("char select mode");
        assert!(char_select.copy_on_select);
        assert_eq!(
            char_select.copy_to,
            WindowCopyDestination::ClipboardAndPrimarySelection
        );
        assert_eq!(char_select.group.as_deref(), Some("SmileysAndEmotion"));
        assert!(
            app.effective_window_title()
                .contains("Char Select: SmileysAndEmotion")
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_char_select_args_action_payload() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::CharSelectArgs(WindowCharSelectOptions {
            copy_on_select: false,
            copy_to: WindowCopyDestination::PrimarySelection,
            group: Some("Smileys & Emotion".to_owned()),
        }));

        let char_select = app.char_select_for_test().expect("char select mode");
        assert!(!char_select.copy_on_select);
        assert_eq!(char_select.copy_to, WindowCopyDestination::PrimarySelection);
        assert_eq!(char_select.group.as_deref(), Some("Smileys & Emotion"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_char_select_args_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "char select copy_on_select false copy_to primary selection group PeopleAndBody"
                .to_owned(),
        );

        let expected = WindowCommand::CharSelectArgs(WindowCharSelectOptions {
            copy_on_select: false,
            copy_to: WindowCopyDestination::PrimarySelection,
            group: Some("PeopleAndBody".to_owned()),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        let char_select = app.char_select_for_test().expect("char select mode");
        assert!(!char_select.copy_on_select);
        assert_eq!(char_select.copy_to, WindowCopyDestination::PrimarySelection);
        assert_eq!(char_select.group.as_deref(), Some("PeopleAndBody"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_char_select_action_name_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "charselect copy_on_select false copy_to primary selection group PeopleAndBody"
                .to_owned(),
        );

        let expected = WindowCommand::CharSelectArgs(WindowCharSelectOptions {
            copy_on_select: false,
            copy_to: WindowCopyDestination::PrimarySelection,
            group: Some("PeopleAndBody".to_owned()),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
    }

    #[test]
    fn window_app_dispatches_palette_char_select_args_query_with_quoted_group() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "char select group \"Smileys & Emotion\" copy_on_select false copy_to clipboard"
                .to_owned(),
        );

        let expected = WindowCommand::CharSelectArgs(WindowCharSelectOptions {
            copy_on_select: false,
            copy_to: WindowCopyDestination::Clipboard,
            group: Some("Smileys & Emotion".to_owned()),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        let char_select = app.char_select_for_test().expect("char select mode");
        assert!(!char_select.copy_on_select);
        assert_eq!(char_select.copy_to, WindowCopyDestination::Clipboard);
        assert_eq!(char_select.group.as_deref(), Some("Smileys & Emotion"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_char_select_args_query_with_equals_fields() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "charselect copy-on-select=false copy-to=primaryselection group=PeopleAndBody"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::CharSelectArgs(WindowCharSelectOptions {
                copy_on_select: false,
                copy_to: WindowCopyDestination::PrimarySelection,
                group: Some("PeopleAndBody".to_owned()),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_char_select_wezterm_action_parenthesized_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.CharSelect({ copy_on_select = false, copy_to = 'PrimarySelection', group = 'PeopleAndBody' })"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::CharSelectArgs(WindowCharSelectOptions {
                copy_on_select: false,
                copy_to: WindowCopyDestination::PrimarySelection,
                group: Some("PeopleAndBody".to_owned()),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_char_select_wezterm_action_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.CharSelect { copy_on_select = false, copy_to = 'PrimarySelection', group = 'PeopleAndBody' }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::CharSelectArgs(WindowCharSelectOptions {
                copy_on_select: false,
                copy_to: WindowCopyDestination::PrimarySelection,
                group: Some("PeopleAndBody".to_owned()),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_char_select_wezterm_action_table_long_bracket_key_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.CharSelect { [[=[copy_on_select]=]] = false, [[=[copy_to]=]] = [[PrimarySelection]], [[=[group]=]] = [[PeopleAndBody]] }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::CharSelectArgs(WindowCharSelectOptions {
                copy_on_select: false,
                copy_to: WindowCopyDestination::PrimarySelection,
                group: Some("PeopleAndBody".to_owned()),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_char_select_wezterm_action_table_trailing_comma_query() {
        for query in [
            "wezterm.action.CharSelect { copy_on_select = false, copy_to = 'PrimarySelection', group = 'PeopleAndBody', }",
            "wezterm.action.CharSelect({ copy_on_select = false, copy_to = 'PrimarySelection', group = 'PeopleAndBody', })",
        ] {
            let mut app = NativeWindowApp::new(None);

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());

            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![WindowCommand::CharSelectArgs(WindowCharSelectOptions {
                    copy_on_select: false,
                    copy_to: WindowCopyDestination::PrimarySelection,
                    group: Some("PeopleAndBody".to_owned()),
                })]
            );
        }
    }

    #[test]
    fn window_app_dispatches_palette_char_select_args_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("char select=group=PeopleAndBody".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::CharSelectArgs(WindowCharSelectOptions {
                copy_on_select: true,
                copy_to: WindowCopyDestination::ClipboardAndPrimarySelection,
                group: Some("PeopleAndBody".to_owned()),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_char_select_args_compact_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("charselect=group=PeopleAndBody".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::CharSelectArgs(WindowCharSelectOptions {
                copy_on_select: true,
                copy_to: WindowCopyDestination::ClipboardAndPrimarySelection,
                group: Some("PeopleAndBody".to_owned()),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_char_select_args_query_with_spaced_equals_field() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "char select copy on select=false copy_to primary selection group PeopleAndBody"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::CharSelectArgs(WindowCharSelectOptions {
                copy_on_select: false,
                copy_to: WindowCopyDestination::PrimarySelection,
                group: Some("PeopleAndBody".to_owned()),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_char_select_args_query_with_spaced_copy_to_equals_field() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "char select copy on select=false copy to=primary selection group PeopleAndBody"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::CharSelectArgs(WindowCharSelectOptions {
                copy_on_select: false,
                copy_to: WindowCopyDestination::PrimarySelection,
                group: Some("PeopleAndBody".to_owned()),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_char_select_args_query_with_quoted_equals_group() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "charselect copy-on-select=false copy-to=primaryselection group=\"Smileys & Emotion\""
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::CharSelectArgs(WindowCharSelectOptions {
                copy_on_select: false,
                copy_to: WindowCopyDestination::PrimarySelection,
                group: Some("Smileys & Emotion".to_owned()),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_char_select_args_query_with_quoted_equals_copy_to() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "charselect copy-on-select=false copy-to=\"primary selection\" group=PeopleAndBody"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::CharSelectArgs(WindowCharSelectOptions {
                copy_on_select: false,
                copy_to: WindowCopyDestination::PrimarySelection,
                group: Some("PeopleAndBody".to_owned()),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_wezterm_formatted_choice_label_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.InputSelector { title = 'Pick Reply', choices = { { id = 'decline', label = wezterm.format { { Text = 'No' }, { Text = ' thanks' } } }, { label = 'LGTM' } }, alphabet = 'ab' }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::InputSelector(WindowInputSelectorOptions {
                title: "Pick Reply".to_owned(),
                choices: vec![
                    WindowInputSelectorChoice {
                        label: "No thanks".to_owned(),
                        id: Some("decline".to_owned()),
                    },
                    WindowInputSelectorChoice {
                        label: "LGTM".to_owned(),
                        id: None,
                    },
                ],
                alphabet: Some("ab".to_owned()),
                description: None,
                fuzzy_description: None,
                fuzzy: false,
                action: None,
            })]
        );
    }

    #[test]
    fn window_app_rejects_palette_char_select_args_query_with_duplicate_fields() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "char select group PeopleAndBody group UnicodeNames".to_owned(),
        );
        assert!(app.command_palette_filtered_commands().is_empty());

        app.command_palette_set_query(
            "char select copy_to clipboard copy_to primary selection".to_owned(),
        );
        assert!(app.command_palette_filtered_commands().is_empty());
    }

    #[test]
    fn window_app_emit_event_dispatches_native_event_payload() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.emit_event_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        app.enter_command_palette_mode();
        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "trigger-update".to_owned(),
            },))
        );

        assert!(app.command_palette.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeWindowEmitEvent {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                name: "trigger-update".to_owned(),
            }]
        );
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_performs_action() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action

            wezterm.on('send-greeting', function(window, pane)
              window:perform_action(act.SendString 'hello', pane)
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"hello");
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_performs_multiple_actions() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action

            wezterm.on('send-greeting', function(window, pane)
              window:perform_action(act.SendString 'hello', pane)
              window:perform_action(act.SendString ' world', pane)
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"hello world");
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_performs_callback_local_static_action() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action

            wezterm.on('send-greeting', function(window, pane)
              local send = act.SendString 'hello'
              window:perform_action(send, pane)
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"hello");
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_sends_pane_text() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('send-greeting', function(window, pane)
              pane:send_text('hello')
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"hello");
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_sends_pane_text_alias() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('send-greeting', function(window, pane)
              local target = pane
              target:send_text('hello-alias')
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler pane alias config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"hello-alias");
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_sends_parenless_pane_text() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('send-greeting', function(window, pane)
              pane:send_text 'hello'
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"hello");
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_sends_pane_paste() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('paste-greeting', function(window, pane)
              pane:send_paste('()（）!@#$%^&*，。！？\nworld')
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "paste-greeting".to_owned(),
            },))
        );

        let expected =
            encode_window_paste("()（）!@#$%^&*，。！？\nworld", false, DEFAULT_CANONICALIZE_PASTED_NEWLINES);
        assert_eq!(written.lock().unwrap().as_slice(), expected.as_slice());
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_emits_static_event() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action

            wezterm.on('send-greeting', function(window, pane)
              wezterm.emit('write-greeting', window, pane)
            end)

            wezterm.on('write-greeting', function(window, pane)
              window:perform_action(act.SendString 'hello', pane)
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"hello");
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_emits_static_event_bracket_field() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action

            wezterm.on('send-greeting', function(window, pane)
              wezterm['emit']('write-greeting', window, pane)
            end)

            wezterm.on('write-greeting', function(window, pane)
              window:perform_action(act.SendString 'hello-bracket', pane)
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler bracket-field config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"hello-bracket");
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_emits_static_event_static_key_field() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local emit_key = 'emit'

            wezterm.on('send-greeting', function(window, pane)
              wezterm[emit_key]('write-greeting', window, pane)
            end)

            wezterm.on('write-greeting', function(window, pane)
              window:perform_action(act.SendString 'hello-static-key-field', pane)
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler static-key field config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"hello-static-key-field"
        );
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_emits_static_event_alias() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local emit = wezterm.emit

            wezterm.on('send-greeting', function(window, pane)
              emit('write-greeting', window, pane)
            end)

            wezterm.on('write-greeting', function(window, pane)
              window:perform_action(act.SendString 'hello', pane)
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"hello");
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_emits_static_event_bracket_alias() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local emit = wezterm['emit']

            wezterm.on('send-greeting', function(window, pane)
              emit('write-greeting', window, pane)
            end)

            wezterm.on('write-greeting', function(window, pane)
              window:perform_action(act.SendString 'hello-bracket-alias', pane)
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler bracket alias config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"hello-bracket-alias");
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_emits_static_event_static_key_alias() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local emit_key = 'emit'
            local emit = wezterm[emit_key]

            wezterm.on('send-greeting', function(window, pane)
              emit('write-greeting', window, pane)
            end)

            wezterm.on('write-greeting', function(window, pane)
              window:perform_action(act.SendString 'hello-static-key', pane)
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler static-key alias config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"hello-static-key");
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_emits_static_event_module_alias() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wt = require 'wezterm'
            local act = wt.action

            wt.on('send-greeting', function(window, pane)
              wt.emit('write-greeting', window, pane)
            end)

            wt.on('write-greeting', function(window, pane)
              window:perform_action(act.SendString 'hello-module-alias', pane)
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler module alias config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"hello-module-alias");
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_emits_static_event_module_emit_alias() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wt = require 'wezterm'
            local act = wt.action
            local emit = wt.emit

            wt.on('send-greeting', function(window, pane)
              emit('write-greeting', window, pane)
            end)

            wt.on('write-greeting', function(window, pane)
              window:perform_action(act.SendString 'hello-module-emit-alias', pane)
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler module emit-alias config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"hello-module-emit-alias"
        );
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_emits_static_event_require_receiver() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action

            wezterm.on('send-greeting', function(window, pane)
              require('wezterm').emit('write-greeting', window, pane)
            end)

            wezterm.on('write-greeting', function(window, pane)
              window:perform_action(act.SendString 'hello-require', pane)
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler require receiver config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"hello-require");
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_emits_static_event_require_alias() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local emit = require('wezterm').emit

            wezterm.on('send-greeting', function(window, pane)
              emit('write-greeting', window, pane)
            end)

            wezterm.on('write-greeting', function(window, pane)
              window:perform_action(act.SendString 'hello-require-alias', pane)
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler require alias config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"hello-require-alias");
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_emits_static_event_alias_dotted_comment() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local emit = wezterm -- event emitter
              .emit

            wezterm.on('send-greeting', function(window, pane)
              emit('write-greeting', window, pane)
            end)

            wezterm.on('write-greeting', function(window, pane)
              window:perform_action(act.SendString 'hello', pane)
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler dotted-comment alias config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"hello");
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_emits_callback_local_static_event() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action

            wezterm.on('send-greeting', function(window, pane)
              local target = 'write-greeting'
              wezterm.emit(target, window, pane)
            end)

            wezterm.on('write-greeting', function(window, pane)
              window:perform_action(act.SendString 'hello', pane)
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"hello");
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_emits_static_event_window_pane_aliases() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action

            wezterm.on('send-greeting', function(window, pane)
              local win = window
              local target_pane = pane
              wezterm.emit('write-greeting', win, target_pane)
            end)

            wezterm.on('write-greeting', function(window, pane)
              window:perform_action(act.SendString 'hello-alias', pane)
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler window/pane alias config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"hello-alias");
    }

    #[test]
    fn window_app_emit_event_static_wezterm_on_handler_false_return_stops_later_handler() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action

            wezterm.on('send-greeting', function(window, pane)
              window:perform_action(act.SendString 'first', pane)
              return false
            end)

            wezterm.on('send-greeting', function(window, pane)
              window:perform_action(act.SendString ' second', pane)
            end)

            return {}
            "#,
        )
        .expect("expected static EmitEvent handler config");
        app.set_config_overrides(overrides);

        assert!(
            app.command_palette_execute(WindowCommand::EmitEvent(WindowEmitEvent {
                name: "send-greeting".to_owned(),
            },))
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"first");
    }

    #[test]
    fn window_app_dispatches_palette_emit_event_query() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.emit_event_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        app.enter_command_palette_mode();
        app.command_palette_set_query("emit event trigger-update".to_owned());

        let expected = WindowCommand::EmitEvent(WindowEmitEvent {
            name: "trigger-update".to_owned(),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert!(app.command_palette.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeWindowEmitEvent {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                name: "trigger-update".to_owned(),
            }]
        );
    }

    #[test]
    fn window_app_dispatches_palette_emit_event_query_with_quoted_name() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.emit_event_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        app.enter_command_palette_mode();
        app.command_palette_set_query("emit event \"trigger update\"".to_owned());

        let expected = WindowCommand::EmitEvent(WindowEmitEvent {
            name: "trigger update".to_owned(),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert!(app.command_palette.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeWindowEmitEvent {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                name: "trigger update".to_owned(),
            }]
        );
    }

    #[test]
    fn window_app_dispatches_palette_emit_event_wezterm_action_function_call_query() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.emit_event_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.EmitEvent(\"trigger-update\")".to_owned());

        let expected = WindowCommand::EmitEvent(WindowEmitEvent {
            name: "trigger-update".to_owned(),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert!(app.command_palette.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeWindowEmitEvent {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                name: "trigger-update".to_owned(),
            }]
        );
    }

    #[test]
    fn window_app_dispatches_palette_emit_event_wezterm_action_table_call_query() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.emit_event_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.EmitEvent { name = \"trigger-update\" }".to_owned(),
        );

        let expected = WindowCommand::EmitEvent(WindowEmitEvent {
            name: "trigger-update".to_owned(),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert!(app.command_palette.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeWindowEmitEvent {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                name: "trigger-update".to_owned(),
            }]
        );
    }

    #[test]
    fn window_app_dispatches_palette_emit_event_wezterm_action_table_long_bracket_key_query() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.emit_event_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.EmitEvent { [[=[name]=]] = [[trigger-update]] }".to_owned(),
        );

        let expected = WindowCommand::EmitEvent(WindowEmitEvent {
            name: "trigger-update".to_owned(),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert!(app.command_palette.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeWindowEmitEvent {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                name: "trigger-update".to_owned(),
            }]
        );
    }

    #[test]
    fn window_app_dispatches_palette_emit_event_wezterm_action_parenthesized_table_query() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.emit_event_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.EmitEvent({ name = \"trigger-update\" })".to_owned(),
        );

        let expected = WindowCommand::EmitEvent(WindowEmitEvent {
            name: "trigger-update".to_owned(),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert!(app.command_palette.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeWindowEmitEvent {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                name: "trigger-update".to_owned(),
            }]
        );
    }

    #[test]
    fn window_app_dispatches_palette_emit_event_wezterm_action_table_trailing_comma_query() {
        for query in [
            "wezterm.action.EmitEvent { name = \"trigger-update\", }",
            "wezterm.action.EmitEvent({ name = \"trigger-update\", })",
        ] {
            let mut app = NativeWindowApp::new(None);

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());

            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![WindowCommand::EmitEvent(WindowEmitEvent {
                    name: "trigger-update".to_owned(),
                })]
            );
        }
    }

    #[test]
    fn window_app_dispatches_palette_emit_event_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("emit event=trigger-update".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EmitEvent(WindowEmitEvent {
                name: "trigger-update".to_owned(),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_emit_event_compact_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("emitevent=trigger-update".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EmitEvent(WindowEmitEvent {
                name: "trigger-update".to_owned(),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_emit_event_name_assignment_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("emitevent name=trigger-update".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EmitEvent(WindowEmitEvent {
                name: "trigger-update".to_owned(),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_emit_event_action_name_query() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.emit_event_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        app.enter_command_palette_mode();
        app.command_palette_set_query("emitevent \"trigger update\"".to_owned());

        let expected = WindowCommand::EmitEvent(WindowEmitEvent {
            name: "trigger update".to_owned(),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        assert!(app.command_palette.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeWindowEmitEvent {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                name: "trigger update".to_owned(),
            }]
        );
    }

    #[test]
    fn window_app_dispatches_user_key_assignment_before_default_shortcuts() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            key_assignments: Some(vec![NativeUserKeyAssignment {
                keys: "CTRL+ALT+D".to_owned(),
                command: WindowCommand::ShowDebugOverlay,
            }]),
            ..NativeConfigSnapshot::default()
        });
        app.modifiers = ModifiersState::CONTROL | ModifiersState::ALT;

        assert!(!app.debug_overlay_active);

        app.handle_keyboard_input_event(
            &Key::Character("d".into()),
            PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
            Some("d"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(app.debug_overlay_active);
    }

    #[test]
    fn window_app_treats_left_ctrl_alt_text_input_as_altgr_when_configured() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.set_config_overrides(native_config_snapshot! {
            treat_left_ctrlalt_as_altgr: Some(true),
            key_map_preference: Some(NativeKeyMapPreference::Physical),
            key_assignments: Some(vec![NativeUserKeyAssignment {
                keys: "CTRL+ALT+D".to_owned(),
                command: WindowCommand::ShowDebugOverlay,
            }]),
            ..NativeConfigSnapshot::default()
        });
        app.modifiers = ModifiersState::CONTROL | ModifiersState::ALT;

        app.handle_keyboard_input_event(
            &Key::Character("ð".into()),
            PhysicalKey::Code(WinitKeyCode::KeyD),
            Some("ð"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.debug_overlay_active);
        assert_eq!(written.lock().unwrap().as_slice(), "ð".as_bytes());
    }

    #[test]
    fn window_app_sends_right_alt_composed_text_without_meta_prefix_by_default() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Alt),
            PhysicalKey::Code(WinitKeyCode::AltRight),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        app.modifiers = ModifiersState::ALT;
        app.handle_keyboard_input_event(
            &Key::Character("∂".into()),
            PhysicalKey::Code(WinitKeyCode::KeyD),
            Some("∂"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), "∂".as_bytes());
    }

    #[test]
    fn window_app_sends_right_alt_composed_release_without_alt_modifier_by_default() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.set_config_overrides(native_config_snapshot! {
            enable_kitty_keyboard: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[=2u").unwrap();

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Alt),
            PhysicalKey::Code(WinitKeyCode::AltRight),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        app.modifiers = ModifiersState::ALT;
        app.handle_keyboard_input_event(
            &Key::Character("∂".into()),
            PhysicalKey::Code(WinitKeyCode::KeyD),
            Some("∂"),
            ElementState::Released,
            KittyKeyEventKind::Release,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b[8706;1:3u");
    }

    #[test]
    fn window_app_matches_raw_alt_assignment_before_right_alt_composed_text() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.set_config_overrides(native_config_snapshot! {
            key_map_preference: Some(NativeKeyMapPreference::Physical),
            key_assignments: Some(vec![NativeUserKeyAssignment {
                keys: "ALT+D".to_owned(),
                command: WindowCommand::ShowDebugOverlay,
            }]),
            ..NativeConfigSnapshot::default()
        });

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Alt),
            PhysicalKey::Code(WinitKeyCode::AltRight),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        app.modifiers = ModifiersState::ALT;
        app.handle_keyboard_input_event(
            &Key::Character("∂".into()),
            PhysicalKey::Code(WinitKeyCode::KeyD),
            Some("∂"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(app.debug_overlay_active_for_test());
        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_key_assignments_accept_wezterm_pipe_modifier_separator() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            leader: Some(NativeLeaderKey {
                keys: "CTRL+A".to_owned(),
                timeout_milliseconds: Some(1_000),
            }),
            key_assignments: Some(vec![
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+D".to_owned(),
                    command: WindowCommand::CharSelect,
                },
                NativeUserKeyAssignment {
                    keys: "LEADER|SHIFT+|".to_owned(),
                    command: WindowCommand::ShowDebugOverlay,
                },
            ]),
            ..NativeConfigSnapshot::default()
        });

        app.modifiers = ModifiersState::CONTROL | ModifiersState::ALT;
        app.handle_keyboard_input_event(
            &Key::Character("d".into()),
            PhysicalKey::Code(WinitKeyCode::KeyD),
            Some("d"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(app.char_select_active_for_test());

        app.char_select = None;
        app.modifiers = ModifiersState::CONTROL;
        app.handle_keyboard_input_event(
            &Key::Character("a".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            Some("a"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        app.modifiers = ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character("|".into()),
            PhysicalKey::Code(WinitKeyCode::Backslash),
            Some("|"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(app.debug_overlay_active_for_test());
    }

    #[test]
    fn window_app_key_assignments_accept_wezterm_modifier_aliases() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            key_assignments: Some(vec![
                NativeUserKeyAssignment {
                    keys: "WIN+D".to_owned(),
                    command: WindowCommand::CharSelect,
                },
                NativeUserKeyAssignment {
                    keys: "OPT+D".to_owned(),
                    command: WindowCommand::ShowDebugOverlay,
                },
            ]),
            ..NativeConfigSnapshot::default()
        });

        app.modifiers = ModifiersState::SUPER;
        app.handle_keyboard_input_event(
            &Key::Character("d".into()),
            PhysicalKey::Code(WinitKeyCode::KeyD),
            Some("d"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(app.char_select_active_for_test());

        app.char_select = None;
        app.modifiers = ModifiersState::ALT;
        app.handle_keyboard_input_event(
            &Key::Character("d".into()),
            PhysicalKey::Code(WinitKeyCode::KeyD),
            Some("d"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(app.debug_overlay_active_for_test());

        let mut meta_app = NativeWindowApp::new(None);
        meta_app.set_config_overrides(native_config_snapshot! {
            key_assignments: Some(vec![NativeUserKeyAssignment {
                keys: "META+D".to_owned(),
                command: WindowCommand::ShowDebugOverlay,
            }]),
            ..NativeConfigSnapshot::default()
        });
        meta_app.modifiers = ModifiersState::ALT;
        meta_app
            .handle_keyboard_input_event(
                &Key::Character("d".into()),
                PhysicalKey::Code(WinitKeyCode::KeyD),
                Some("d"),
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
        assert!(meta_app.debug_overlay_active_for_test());
    }

    #[test]
    fn window_app_key_assignments_accept_wezterm_function_key_identifiers() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            key_assignments: Some(vec![
                NativeUserKeyAssignment {
                    keys: "F1".to_owned(),
                    command: WindowCommand::CharSelect,
                },
                NativeUserKeyAssignment {
                    keys: "SHIFT+F24".to_owned(),
                    command: WindowCommand::ShowDebugOverlay,
                },
            ]),
            ..NativeConfigSnapshot::default()
        });

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::F1),
            PhysicalKey::Code(WinitKeyCode::F1),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(app.char_select_active_for_test());

        app.char_select = None;
        app.modifiers = ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::F24),
            PhysicalKey::Code(WinitKeyCode::F24),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(app.debug_overlay_active_for_test());
    }

    #[test]
    fn window_app_key_assignments_accept_wezterm_named_key_identifiers() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            key_assignments: Some(vec![
                NativeUserKeyAssignment {
                    keys: "CAPSLOCK".to_owned(),
                    command: WindowCommand::CharSelect,
                },
                NativeUserKeyAssignment {
                    keys: "SHIFT+PRINTSCREEN".to_owned(),
                    command: WindowCommand::ShowDebugOverlay,
                },
            ]),
            ..NativeConfigSnapshot::default()
        });

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::CapsLock),
            PhysicalKey::Code(WinitKeyCode::CapsLock),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(app.char_select_active_for_test());

        app.char_select = None;
        app.modifiers = ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::PrintScreen),
            PhysicalKey::Code(WinitKeyCode::PrintScreen),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(app.debug_overlay_active_for_test());
    }

    #[test]
    fn window_app_key_assignments_accept_wezterm_numpad_and_browser_key_identifiers() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            key_assignments: Some(vec![
                NativeUserKeyAssignment {
                    keys: "NUMPAD0".to_owned(),
                    command: WindowCommand::CharSelect,
                },
                NativeUserKeyAssignment {
                    keys: "BROWSERBACK".to_owned(),
                    command: WindowCommand::ShowDebugOverlay,
                },
            ]),
            ..NativeConfigSnapshot::default()
        });

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("0".into()),
            PhysicalKey::Code(WinitKeyCode::Digit0),
            Some("0"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(!app.char_select_active_for_test());

        app.handle_keyboard_input_event(
            &Key::Character("0".into()),
            PhysicalKey::Code(WinitKeyCode::Numpad0),
            Some("0"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(app.char_select_active_for_test());

        app.char_select = None;
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::BrowserBack),
            PhysicalKey::Code(WinitKeyCode::BrowserBack),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(app.debug_overlay_active_for_test());
    }

    #[test]
    fn window_app_key_assignments_accept_wezterm_phys_and_mapped_prefixes() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            key_assignments: Some(vec![
                NativeUserKeyAssignment {
                    keys: "CTRL+phys:D".to_owned(),
                    command: WindowCommand::CharSelect,
                },
                NativeUserKeyAssignment {
                    keys: "CTRL+mapped:D".to_owned(),
                    command: WindowCommand::ShowDebugOverlay,
                },
            ]),
            ..NativeConfigSnapshot::default()
        });

        app.modifiers = ModifiersState::CONTROL;
        app.handle_keyboard_input_event(
            &Key::Character("x".into()),
            PhysicalKey::Code(WinitKeyCode::KeyD),
            Some("x"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(app.char_select_active_for_test());
        assert!(!app.debug_overlay_active_for_test());

        app.char_select = None;
        app.handle_keyboard_input_event(
            &Key::Character("d".into()),
            PhysicalKey::Code(WinitKeyCode::KeyX),
            Some("d"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(!app.char_select_active_for_test());
        assert!(app.debug_overlay_active_for_test());
    }

    #[test]
    fn window_app_key_assignments_accept_wezterm_raw_key_prefix() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            key_assignments: Some(vec![NativeUserKeyAssignment {
                keys: "CTRL+raw:123".to_owned(),
                command: WindowCommand::CharSelect,
            }]),
            ..NativeConfigSnapshot::default()
        });

        app.modifiers = ModifiersState::CONTROL;
        app.handle_keyboard_input_event(
            &Key::Unidentified(winit::keyboard::NativeKey::Windows(123)),
            PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Windows(122)),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(!app.char_select_active_for_test());

        app.handle_keyboard_input_event(
            &Key::Unidentified(winit::keyboard::NativeKey::Windows(123)),
            PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Windows(123)),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(app.char_select_active_for_test());
    }

    #[test]
    fn window_app_key_assignments_honor_wezterm_physical_key_map_preference() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            key_map_preference: Some(NativeKeyMapPreference::Physical),
            key_assignments: Some(vec![NativeUserKeyAssignment {
                keys: "CTRL+D".to_owned(),
                command: WindowCommand::CharSelect,
            }]),
            ..NativeConfigSnapshot::default()
        });

        app.modifiers = ModifiersState::CONTROL;
        app.handle_keyboard_input_event(
            &Key::Character("d".into()),
            PhysicalKey::Code(WinitKeyCode::KeyX),
            Some("d"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(!app.char_select_active_for_test());

        app.handle_keyboard_input_event(
            &Key::Character("x".into()),
            PhysicalKey::Code(WinitKeyCode::KeyD),
            Some("x"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(app.char_select_active_for_test());
    }

    #[test]
    fn window_app_leader_key_dispatches_leader_assignments() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            leader: Some(NativeLeaderKey {
                keys: "CTRL+A".to_owned(),
                timeout_milliseconds: Some(1_000),
            }),
            key_assignments: Some(vec![NativeUserKeyAssignment {
                keys: "LEADER+SHIFT+|".to_owned(),
                command: WindowCommand::ShowDebugOverlay,
            }]),
            ..NativeConfigSnapshot::default()
        });

        app.modifiers = ModifiersState::CONTROL;
        app.handle_keyboard_input_event(
            &Key::Character("a".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            Some("a"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(!app.debug_overlay_active_for_test());

        app.modifiers = ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character("|".into()),
            PhysicalKey::Code(WinitKeyCode::Backslash),
            Some("|"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(app.debug_overlay_active_for_test());
    }

    #[test]
    fn window_app_leader_key_swallows_unmatched_key_and_exits() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.set_config_overrides(native_config_snapshot! {
            leader: Some(NativeLeaderKey {
                keys: "CTRL+A".to_owned(),
                timeout_milliseconds: Some(1_000),
            }),
            key_assignments: Some(vec![NativeUserKeyAssignment {
                keys: "LEADER+SHIFT+|".to_owned(),
                command: WindowCommand::ShowDebugOverlay,
            }]),
            ..NativeConfigSnapshot::default()
        });

        app.modifiers = ModifiersState::CONTROL;
        app.handle_keyboard_input_event(
            &Key::Character("a".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            Some("a"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("x".into()),
            PhysicalKey::Code(WinitKeyCode::KeyX),
            Some("x"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(written.lock().unwrap().as_slice(), b"");

        app.handle_keyboard_input_event(
            &Key::Character("x".into()),
            PhysicalKey::Code(WinitKeyCode::KeyX),
            Some("x"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(written.lock().unwrap().as_slice(), b"x");
    }

    #[test]
    fn window_app_uses_wezterm_compose_cursor_color_while_leader_is_active() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.leader = { key = 'a', mods = 'CTRL', timeout_milliseconds = 1000 }
            config.colors = {
              compose_cursor = '#010203',
            }

            return config
            "##,
        )
        .expect("expected WezTerm leader and compose cursor config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL;
        app.handle_keyboard_input_event(
            &Key::Character("a".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            Some("a"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        assert_eq!(snapshot.cursor_color(), Some(Color::Rgb(1, 2, 3)));
    }

    #[test]
    fn window_app_uses_wezterm_compose_cursor_color_while_dead_key_is_active() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              compose_cursor = '#010203',
            }

            return config
            "##,
        )
        .expect("expected WezTerm compose cursor config");
        app.set_config_overrides(overrides);

        app.handle_keyboard_input_event(
            &Key::Dead(Some('^')),
            PhysicalKey::Code(WinitKeyCode::Quote),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        assert_eq!(snapshot.cursor_color(), Some(Color::Rgb(1, 2, 3)));

        app.handle_keyboard_input_event(
            &Key::Character("e".into()),
            PhysicalKey::Code(WinitKeyCode::KeyE),
            Some("ê"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        assert_eq!(snapshot.cursor_color(), None);
    }

    #[test]
    fn window_app_honors_wezterm_use_dead_keys_false() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.use_dead_keys = false
            config.colors = {
              compose_cursor = '#010203',
            }

            return config
            "##,
        )
        .expect("expected WezTerm use_dead_keys config");
        app.set_config_overrides(overrides);

        app.handle_keyboard_input_event(
            &Key::Dead(Some('^')),
            PhysicalKey::Code(WinitKeyCode::Quote),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        assert_eq!(snapshot.cursor_color(), None);
    }

    #[test]
    fn window_app_tracks_native_key_table_stack_actions() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::ActivateKeyTable(
            WindowActivateKeyTable {
                name: "resize_pane".to_owned(),
                timeout_milliseconds: Some(1_000),
                one_shot: false,
                replace_current: false,
                until_unknown: true,
                prevent_fallback: true,
            },
        )));
        assert!(app.command_palette.is_none());
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));
        assert!(
            app.effective_window_title()
                .contains("KeyTable: resize_pane")
        );

        assert!(app.command_palette_execute(WindowCommand::ActivateKeyTable(
            WindowActivateKeyTable {
                name: "leader".to_owned(),
                timeout_milliseconds: None,
                one_shot: true,
                replace_current: false,
                until_unknown: false,
                prevent_fallback: false,
            },
        )));
        assert_eq!(app.active_key_table_for_test(), Some("leader"));

        assert!(app.command_palette_execute(WindowCommand::PopKeyTable));
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        assert!(app.command_palette_execute(WindowCommand::ClearKeyTableStack));
        assert_eq!(app.active_key_table_for_test(), None);
        assert!(
            !app.effective_window_title()
                .contains("KeyTable: resize_pane")
        );
    }

    #[test]
    fn window_app_dispatches_palette_key_table_stack_queries() {
        let mut app = NativeWindowApp::new(None);

        assert!(app.command_palette_execute(WindowCommand::ActivateKeyTable(
            WindowActivateKeyTable {
                name: "resize_pane".to_owned(),
                timeout_milliseconds: Some(1_000),
                one_shot: false,
                replace_current: false,
                until_unknown: true,
                prevent_fallback: true,
            },
        )));
        assert!(app.command_palette_execute(WindowCommand::ActivateKeyTable(
            WindowActivateKeyTable {
                name: "leader".to_owned(),
                timeout_milliseconds: None,
                one_shot: true,
                replace_current: false,
                until_unknown: false,
                prevent_fallback: false,
            },
        )));
        assert_eq!(app.active_key_table_for_test(), Some("leader"));

        app.enter_command_palette_mode();
        app.command_palette_set_query("pop key table".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::PopKeyTable]
        );
        assert!(app.command_palette_execute(WindowCommand::PopKeyTable));
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_set_query("clear key table stack".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ClearKeyTableStack]
        );
        assert!(app.command_palette_execute(WindowCommand::ClearKeyTableStack));
        assert_eq!(app.active_key_table_for_test(), None);
        assert!(
            !app.effective_window_title()
                .contains("KeyTable: resize_pane")
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_key_table_stack_action_name_queries() {
        for (query, expected) in [
            ("PopKeyTable", WindowCommand::PopKeyTable),
            ("Clear Key Table Stack", WindowCommand::ClearKeyTableStack),
        ] {
            let mut app = NativeWindowApp::new(None);
            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());

            assert_eq!(app.command_palette_filtered_commands(), vec![expected]);
        }
    }

    #[test]
    fn window_app_dispatches_palette_activate_key_table_queries() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "activate key table resize_pane timeout 1000 one shot false replace current false until unknown true prevent fallback true"
                .to_owned(),
        );
        let command = WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
            name: "resize_pane".to_owned(),
            timeout_milliseconds: Some(1_000),
            one_shot: false,
            replace_current: false,
            until_unknown: true,
            prevent_fallback: true,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));
        assert!(
            app.effective_window_title()
                .contains("KeyTable: resize_pane")
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_key_table_action_name_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "activatekeytable resize_pane Timeout 1000 One Shot false Replace Current false Until Unknown true Prevent Fallback true"
                .to_owned(),
        );
        let command = WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
            name: "resize_pane".to_owned(),
            timeout_milliseconds: Some(1_000),
            one_shot: false,
            replace_current: false,
            until_unknown: true,
            prevent_fallback: true,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));
        assert!(
            app.effective_window_title()
                .contains("KeyTable: resize_pane")
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_key_table_wezterm_action_table_call_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.ActivateKeyTable { name = \"resize_pane\", timeout_milliseconds = 1000, one_shot = false, replace_current = true, until_unknown = true, prevent_fallback = true }"
                .to_owned(),
        );
        let command = WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
            name: "resize_pane".to_owned(),
            timeout_milliseconds: Some(1_000),
            one_shot: false,
            replace_current: true,
            until_unknown: true,
            prevent_fallback: true,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));
        assert!(
            app.effective_window_title()
                .contains("KeyTable: resize_pane")
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_key_table_wezterm_action_table_long_bracket_key_query()
     {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.ActivateKeyTable { [[=[name]=]] = [[resize_pane]], [[=[timeout_milliseconds]=]] = 1000, [[=[one_shot]=]] = false, [[=[replace_current]=]] = true, [[=[until_unknown]=]] = true, [[=[prevent_fallback]=]] = true }"
                .to_owned(),
        );
        let command = WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
            name: "resize_pane".to_owned(),
            timeout_milliseconds: Some(1_000),
            one_shot: false,
            replace_current: true,
            until_unknown: true,
            prevent_fallback: true,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));
        assert!(
            app.effective_window_title()
                .contains("KeyTable: resize_pane")
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_key_table_wezterm_action_parenthesized_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.ActivateKeyTable({ name = \"resize_pane\", timeout_milliseconds = 1000, one_shot = false, replace_current = true, until_unknown = true, prevent_fallback = true })"
                .to_owned(),
        );
        let command = WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
            name: "resize_pane".to_owned(),
            timeout_milliseconds: Some(1_000),
            one_shot: false,
            replace_current: true,
            until_unknown: true,
            prevent_fallback: true,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));
        assert!(
            app.effective_window_title()
                .contains("KeyTable: resize_pane")
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_key_table_wezterm_action_table_trailing_comma_query()
    {
        for query in [
            "wezterm.action.ActivateKeyTable { name = \"resize_pane\", timeout_milliseconds = 1000, one_shot = false, replace_current = true, until_unknown = true, prevent_fallback = true, }",
            "wezterm.action.ActivateKeyTable({ name = \"resize_pane\", timeout_milliseconds = 1000, one_shot = false, replace_current = true, until_unknown = true, prevent_fallback = true, })",
        ] {
            let mut app = NativeWindowApp::new(None);

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());

            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
                    name: "resize_pane".to_owned(),
                    timeout_milliseconds: Some(1_000),
                    one_shot: false,
                    replace_current: true,
                    until_unknown: true,
                    prevent_fallback: true,
                })]
            );
        }
    }

    #[test]
    fn window_app_dispatches_palette_activate_key_table_hyphenated_field_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "activatekeytable resize_pane timeout-milliseconds 1000 one-shot false replace-current true until-unknown true prevent-fallback true"
                .to_owned(),
        );
        let command = WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
            name: "resize_pane".to_owned(),
            timeout_milliseconds: Some(1_000),
            one_shot: false,
            replace_current: true,
            until_unknown: true,
            prevent_fallback: true,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));
        assert!(
            app.effective_window_title()
                .contains("KeyTable: resize_pane")
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_key_table_equals_field_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "activate key table resize_pane timeout_milliseconds=1000 one_shot=false replace-current=true until_unknown=true prevent-fallback=true"
                .to_owned(),
        );
        let command = WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
            name: "resize_pane".to_owned(),
            timeout_milliseconds: Some(1_000),
            one_shot: false,
            replace_current: true,
            until_unknown: true,
            prevent_fallback: true,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));
        assert!(
            app.effective_window_title()
                .contains("KeyTable: resize_pane")
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_key_table_action_name_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "activatekeytable resize_pane timeout=500 one-shot=false replace_current=true until-unknown=true prevent_fallback=true"
                .to_owned(),
        );
        let command = WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
            name: "resize_pane".to_owned(),
            timeout_milliseconds: Some(500),
            one_shot: false,
            replace_current: true,
            until_unknown: true,
            prevent_fallback: true,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));
        assert!(
            app.effective_window_title()
                .contains("KeyTable: resize_pane")
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_key_table_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "activate key table=resize_pane timeout=500 one-shot=false replace_current=true until-unknown=true prevent_fallback=true"
                .to_owned(),
        );
        let command = WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
            name: "resize_pane".to_owned(),
            timeout_milliseconds: Some(500),
            one_shot: false,
            replace_current: true,
            until_unknown: true,
            prevent_fallback: true,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));
        assert!(
            app.effective_window_title()
                .contains("KeyTable: resize_pane")
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_key_table_compact_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "activatekeytable=resize_pane timeout=500 one-shot=false replace_current=true until-unknown=true prevent_fallback=true"
                .to_owned(),
        );
        let command = WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
            name: "resize_pane".to_owned(),
            timeout_milliseconds: Some(500),
            one_shot: false,
            replace_current: true,
            until_unknown: true,
            prevent_fallback: true,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));
        assert!(
            app.effective_window_title()
                .contains("KeyTable: resize_pane")
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_key_table_mixed_case_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "ActivateKeyTable resize_pane Timeout=500 One-Shot=false Replace_Current=true Until-Unknown=true Prevent_Fallback=true"
                .to_owned(),
        );
        let command = WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
            name: "resize_pane".to_owned(),
            timeout_milliseconds: Some(500),
            one_shot: false,
            replace_current: true,
            until_unknown: true,
            prevent_fallback: true,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
    }

    #[test]
    fn window_app_dispatches_palette_activate_key_table_spaced_equals_field_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "activate key table resize_pane timeout milliseconds=500 one shot=false replace current=true until unknown=true prevent fallback=true"
                .to_owned(),
        );
        let command = WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
            name: "resize_pane".to_owned(),
            timeout_milliseconds: Some(500),
            one_shot: false,
            replace_current: true,
            until_unknown: true,
            prevent_fallback: true,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
    }

    #[test]
    fn window_app_dispatches_palette_activate_key_table_query_with_quoted_name() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "activate key table \"resize timeout mode\" timeout 1000 one shot true".to_owned(),
        );
        let command = WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
            name: "resize timeout mode".to_owned(),
            timeout_milliseconds: Some(1_000),
            one_shot: true,
            replace_current: false,
            until_unknown: false,
            prevent_fallback: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(app.active_key_table_for_test(), Some("resize timeout mode"));
        assert!(
            app.effective_window_title()
                .contains("KeyTable: resize timeout mode")
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_key_table_query_defaults_to_one_shot() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate key table resize_pane".to_owned());
        let command = WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
            name: "resize_pane".to_owned(),
            timeout_milliseconds: None,
            one_shot: true,
            replace_current: false,
            until_unknown: false,
            prevent_fallback: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_rejects_palette_activate_key_table_query_with_duplicate_fields() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "activate key table resize_pane timeout 1000 timeout 2000".to_owned(),
        );
        assert!(app.command_palette_filtered_commands().is_empty());

        app.command_palette_set_query(
            "activate key table resize_pane one shot true one_shot false".to_owned(),
        );
        assert!(app.command_palette_filtered_commands().is_empty());
    }

    #[test]
    fn window_app_expires_timed_key_table_activations() {
        let mut app = NativeWindowApp::new(None);

        assert!(app.command_palette_execute(WindowCommand::ActivateKeyTable(
            WindowActivateKeyTable {
                name: "activate_pane".to_owned(),
                timeout_milliseconds: Some(10_000),
                one_shot: true,
                replace_current: false,
                until_unknown: false,
                prevent_fallback: false,
            },
        )));
        let after_activation = Instant::now();
        assert_eq!(app.active_key_table_for_test(), Some("activate_pane"));

        assert!(
            !app.expire_key_table_stack_if_due(after_activation + Duration::from_millis(9_000))
        );
        assert_eq!(app.active_key_table_for_test(), Some("activate_pane"));

        assert!(
            app.expire_key_table_stack_if_due(after_activation + Duration::from_millis(10_000))
        );
        assert_eq!(app.active_key_table_for_test(), None);
        assert!(
            !app.effective_window_title()
                .contains("KeyTable: activate_pane")
        );
    }

    #[test]
    fn window_app_one_shot_key_table_pops_after_next_key_press() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        assert!(app.command_palette_execute(WindowCommand::ActivateKeyTable(
            WindowActivateKeyTable {
                name: "activate_pane".to_owned(),
                timeout_milliseconds: None,
                one_shot: true,
                replace_current: false,
                until_unknown: false,
                prevent_fallback: false,
            },
        )));
        assert_eq!(app.active_key_table_for_test(), Some("activate_pane"));

        app.handle_keyboard_input_event(
            &Key::Character("x".into()),
            PhysicalKey::Code(WinitKeyCode::KeyX),
            Some("x"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(app.active_key_table_for_test(), None);
        assert_eq!(written.lock().unwrap().as_slice(), b"x");
        assert!(
            !app.effective_window_title()
                .contains("KeyTable: activate_pane")
        );
    }

    #[test]
    fn window_app_until_unknown_key_table_pops_on_unmatched_key_press() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        assert!(app.command_palette_execute(WindowCommand::ActivateKeyTable(
            WindowActivateKeyTable {
                name: "resize_pane".to_owned(),
                timeout_milliseconds: None,
                one_shot: false,
                replace_current: false,
                until_unknown: true,
                prevent_fallback: false,
            },
        )));
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.handle_keyboard_input_event(
            &Key::Character("x".into()),
            PhysicalKey::Code(WinitKeyCode::KeyX),
            Some("x"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(app.active_key_table_for_test(), None);
        assert_eq!(written.lock().unwrap().as_slice(), b"x");
        assert!(
            !app.effective_window_title()
                .contains("KeyTable: resize_pane")
        );
    }

    #[test]
    fn window_app_prevent_fallback_key_table_consumes_unmatched_key_press() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        assert!(app.command_palette_execute(WindowCommand::ActivateKeyTable(
            WindowActivateKeyTable {
                name: "locked".to_owned(),
                timeout_milliseconds: None,
                one_shot: false,
                replace_current: false,
                until_unknown: false,
                prevent_fallback: true,
            },
        )));
        assert_eq!(app.active_key_table_for_test(), Some("locked"));

        app.handle_keyboard_input_event(
            &Key::Character("x".into()),
            PhysicalKey::Code(WinitKeyCode::KeyX),
            Some("x"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(app.active_key_table_for_test(), Some("locked"));
        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_active_key_table_executes_matching_native_assignment() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.set_config_overrides(native_config_snapshot! {
            key_tables: Some(BTreeMap::from([(
                "resize_pane".to_owned(),
                vec![NativeUserKeyAssignment {
                    keys: "h".to_owned(),
                    command: WindowCommand::AdjustPaneSize {
                        direction: ResizeDirection::Left,
                        amount: 1,
                    },
                }],
            )])),
            ..NativeConfigSnapshot::default()
        });
        app.command_palette_execute(WindowCommand::SplitRight);
        let split_delta_before = app
            .app_shell
            .active_tab()
            .panes()
            .iter()
            .find(|pane| pane.id() == rterm_core::PaneId::new(2))
            .unwrap()
            .split()
            .expect("split should be present")
            .source_size_delta;

        assert!(app.command_palette_execute(WindowCommand::ActivateKeyTable(
            WindowActivateKeyTable {
                name: "resize_pane".to_owned(),
                timeout_milliseconds: None,
                one_shot: false,
                replace_current: false,
                until_unknown: false,
                prevent_fallback: false,
            },
        )));

        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        let split_delta_after = app
            .app_shell
            .active_tab()
            .panes()
            .iter()
            .find(|pane| pane.id() == rterm_core::PaneId::new(2))
            .unwrap()
            .split()
            .expect("split should be present")
            .source_size_delta;
        assert_eq!(split_delta_after, split_delta_before - 1);
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));
        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_active_key_table_searches_stack_for_matching_native_assignment() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.set_config_overrides(native_config_snapshot! {
            key_tables: Some(BTreeMap::from([(
                "base".to_owned(),
                vec![NativeUserKeyAssignment {
                    keys: "x".to_owned(),
                    command: WindowCommand::SendString("matched-base".to_owned()),
                }],
            )])),
            ..NativeConfigSnapshot::default()
        });

        for name in ["base", "overlay"] {
            assert!(app.command_palette_execute(WindowCommand::ActivateKeyTable(
                WindowActivateKeyTable {
                    name: name.to_owned(),
                    timeout_milliseconds: None,
                    one_shot: false,
                    replace_current: false,
                    until_unknown: false,
                    prevent_fallback: false,
                },
            )));
        }
        assert_eq!(app.active_key_table_for_test(), Some("overlay"));

        app.handle_keyboard_input_event(
            &Key::Character("x".into()),
            PhysicalKey::Code(WinitKeyCode::KeyX),
            Some("x"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"matched-base");
        assert_eq!(app.active_key_table_for_test(), Some("overlay"));
    }

    #[test]
    fn window_app_matching_key_table_assignment_resets_activation_timeout() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            key_tables: Some(BTreeMap::from([(
                "repeatable".to_owned(),
                vec![NativeUserKeyAssignment {
                    keys: "x".to_owned(),
                    command: WindowCommand::Nop,
                }],
            )])),
            ..NativeConfigSnapshot::default()
        });
        assert!(app.command_palette_execute(WindowCommand::ActivateKeyTable(
            WindowActivateKeyTable {
                name: "repeatable".to_owned(),
                timeout_milliseconds: Some(1_000),
                one_shot: false,
                replace_current: false,
                until_unknown: false,
                prevent_fallback: false,
            },
        )));
        app.key_table_stack[0].activated_at = Instant::now()
            .checked_sub(Duration::from_millis(1_500))
            .expect("test timeout offset should be representable");

        app.handle_keyboard_input_event(
            &Key::Character("x".into()),
            PhysicalKey::Code(WinitKeyCode::KeyX),
            Some("x"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.expire_key_table_stack_if_due(Instant::now()));
        assert_eq!(app.active_key_table_for_test(), Some("repeatable"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_tables_into_runtime_assignments() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local keys = 'not the config keys table'
            local config = {}

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.leader = { key = 'a', mods = 'CTRL', timeout_milliseconds = 1000 }

            config.key_tables = {
              resize_pane = {
                { key = 'h', action = act.SendString 'left' },
              },
            }

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
        .expect("expected WezTerm key_tables config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.leader,
            Some(NativeLeaderKey {
                keys: "CTRL+a".to_owned(),
                timeout_milliseconds: Some(1_000),
            })
        );
        assert_eq!(
            effective.keys,
            vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+Space".to_owned(),
                command: WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
                    name: "resize_pane".to_owned(),
                    timeout_milliseconds: None,
                    one_shot: true,
                    replace_current: false,
                    until_unknown: false,
                    prevent_fallback: false,
                }),
            }]
        );
        assert_eq!(
            effective.key_tables,
            BTreeMap::from([(
                "resize_pane".to_owned(),
                vec![NativeUserKeyAssignment {
                    keys: "h".to_owned(),
                    command: WindowCommand::SendString("left".to_owned()),
                }],
            )])
        );
        assert_eq!(
            effective.mouse_bindings,
            vec![NativeUserMouseAssignment {
                event: NativeMouseAssignmentEvent {
                    kind: NativeMouseAssignmentEventKind::Drag,
                    button: NativeMouseAssignmentButton::Mouse(MouseButton::Left),
                    streak: 1,
                },
                modifiers: ModifiersState::ALT,
                mouse_reporting: false,
                alt_screen: NativeMouseAssignmentAltScreen::Any,
                command: WindowCommand::StartWindowDrag,
            }]
        );

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"left");
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_tables_static_variable_assignment() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local user_key_tables = {
              resize_pane = {
                { key = 'h', action = act.SendString 'from-table-variable' },
              },
            }

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = user_key_tables

            return config
            "#,
        )
        .expect("expected WezTerm static variable key_tables config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"from-table-variable");
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_tables_static_variable_nested_insert() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local user_key_tables = {
              resize_pane = {},
            }
            table.insert(user_key_tables.resize_pane, {
              key = 'h',
              action = act.SendString 'from-nested-variable-insert',
            })

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = user_key_tables

            return config
            "#,
        )
        .expect("expected WezTerm static variable nested table.insert key_tables config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"from-nested-variable-insert"
        );
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_key_tables_static_variable_index_field_assignments() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local user_key_tables = {
              resize_pane = {},
            }
            user_key_tables.resize_pane[1] = {}
            user_key_tables.resize_pane[1].key = 'h'
            user_key_tables.resize_pane[1].action = act.SendString 'from-variable-index-fields'

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = user_key_tables

            return config
            "#,
        )
        .expect("expected WezTerm static variable indexed field key_tables config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"from-variable-index-fields"
        );
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_key_tables_static_variable_index_static_field_name_assignments() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local key_field = 'key'
            local action_field = 'action'

            local user_key_tables = {
              resize_pane = {},
            }
            user_key_tables.resize_pane[1] = {}
            user_key_tables.resize_pane[1][key_field] = 'h'
            user_key_tables.resize_pane[1][action_field] =
              act.SendString 'from-variable-index-static-fields'

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = user_key_tables

            return config
            "#,
        )
        .expect("expected WezTerm static variable indexed static field key_tables config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"from-variable-index-static-fields"
        );
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_key_tables_variable_post_assignment_nested_insert() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local user_key_tables = {
              resize_pane = {},
            }

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = user_key_tables
            table.insert(user_key_tables.resize_pane, {
              key = 'h',
              action = act.SendString 'from-post-assignment-nested-insert',
            })

            return config
            "#,
        )
        .expect("expected WezTerm post-assignment nested table.insert key_tables config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"from-post-assignment-nested-insert"
        );
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_key_tables_variable_post_assignment_index_assignment() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local user_key_tables = {
              resize_pane = {
                { key = 'h', action = act.SendString 'before-post-assignment-index' },
              },
            }

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = user_key_tables
            user_key_tables.resize_pane[1] = {
              key = 'h',
              action = act.SendString 'from-post-assignment-index',
            }

            return config
            "#,
        )
        .expect("expected WezTerm post-assignment indexed key_tables config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"from-post-assignment-index"
        );
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_key_tables_variable_post_assignment_index_fields() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local user_key_tables = {
              resize_pane = {},
            }

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = user_key_tables
            user_key_tables.resize_pane[1] = {}
            user_key_tables.resize_pane[1].key = 'h'
            user_key_tables.resize_pane[1].action = act.SendString 'from-post-index-fields'

            return config
            "#,
        )
        .expect("expected WezTerm post-assignment indexed field key_tables config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"from-post-index-fields"
        );
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_key_tables_variable_post_assignment_field_assignment() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local user_key_tables = {}

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = user_key_tables
            user_key_tables.resize_pane = {
              { key = 'h', action = act.SendString 'from-post-assignment-field' },
            }

            return config
            "#,
        )
        .expect("expected WezTerm post-assignment field key_tables config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"from-post-assignment-field"
        );
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_tables_static_variable_field_assignment() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local user_key_tables = {}
            user_key_tables.resize_pane = {
              { key = 'h', action = act.SendString 'from-field-assignment' },
            }

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = user_key_tables

            return config
            "#,
        )
        .expect("expected WezTerm static variable field assignment key_tables config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"from-field-assignment");
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_tables_static_variable_index_assignment() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local user_key_tables = {
              resize_pane = {},
            }
            user_key_tables.resize_pane[1] = {
              key = 'h',
              action = act.SendString 'from-index-assignment',
            }

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = user_key_tables

            return config
            "#,
        )
        .expect("expected WezTerm static variable indexed key table assignment config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"from-index-assignment");
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_tables_static_variable_length_append_assignment() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local user_key_tables = {
              resize_pane = {},
            }
            user_key_tables.resize_pane[#user_key_tables.resize_pane + 1] = {
              key = 'h',
              action = act.SendString 'from-key-tables-length-append',
            }

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = user_key_tables

            return config
            "#,
        )
        .expect("expected WezTerm static variable length-append key table assignment config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"from-key-tables-length-append"
        );
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_tables_static_field_variables() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local move_key = 'h'
            local move_mods = 'NONE'

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = {
              resize_pane = {
                { key = move_key, mods = move_mods, action = act.SendString 'from-field-variable' },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm key_tables static field variable config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"from-field-variable");
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_tables_static_name_variable() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local mode_name = 'resize_pane'

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = {
              [mode_name] = {
                { key = 'h', action = act.SendString 'from-name-variable' },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm key_tables static name variable config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"from-name-variable");
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_copy_mode_static_assignment_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local assignment = 'MoveToStartOfLine'

            config.key_tables = {
              copy_mode = {
                { key = '0', action = act.CopyMode(assignment) },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm CopyMode static assignment variable config");

        assert_eq!(
            overrides.key_tables,
            Some(BTreeMap::from([(
                "copy_mode".to_owned(),
                vec![NativeUserKeyAssignment {
                    keys: "0".to_owned(),
                    command: WindowCommand::CopyMode(
                        super::WindowCopyModeAssignment::MoveToStartOfLine,
                    ),
                }],
            )]))
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_copy_mode_static_table_field_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local selection_mode = 'Block'
            local semantic_type = 'Prompt'
            local jump_before = true
            local page_amount = -0.5

            config.key_tables = {
              copy_mode = {
                { key = 'v', action = act.CopyMode { SetSelectionMode = selection_mode } },
                { key = 'p', action = act.CopyMode { MoveBackwardSemanticZoneOfType = semantic_type } },
                { key = 'f', action = act.CopyMode { JumpForward = { prev_char = jump_before } } },
                { key = 'u', action = act.CopyMode { MoveByPage = page_amount } },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm CopyMode static table field variable config");

        assert_eq!(
            overrides.key_tables,
            Some(BTreeMap::from([(
                "copy_mode".to_owned(),
                vec![
                    NativeUserKeyAssignment {
                        keys: "v".to_owned(),
                        command: WindowCommand::CopyMode(
                            super::WindowCopyModeAssignment::SetSelectionMode(
                                super::WindowCopySelectionMode::Block,
                            ),
                        ),
                    },
                    NativeUserKeyAssignment {
                        keys: "p".to_owned(),
                        command: WindowCommand::CopyMode(
                            super::WindowCopyModeAssignment::MoveSemanticZoneOfType {
                                delta: -1,
                                semantic_type: rterm_terminal::SemanticType::Prompt,
                            },
                        ),
                    },
                    NativeUserKeyAssignment {
                        keys: "f".to_owned(),
                        command: WindowCommand::CopyMode(
                            super::WindowCopyModeAssignment::StartJump {
                                forward: true,
                                prev_char: true,
                            },
                        ),
                    },
                    NativeUserKeyAssignment {
                        keys: "u".to_owned(),
                        command: WindowCommand::CopyMode(
                            super::WindowCopyModeAssignment::MoveByPage(
                                WindowScrollByPageAmount::from_per_mille(-500),
                            )
                        ),
                    },
                ],
            )]))
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_copy_mode_static_field_name_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local selection_field = 'SetSelectionMode'
            local jump_field = 'JumpForward'
            local prev_char_field = 'prev_char'

            config.key_tables = {
              copy_mode = {
                { key = 'v', action = act.CopyMode { [selection_field] = 'Line' } },
                { key = 'f', action = act.CopyMode { [jump_field] = { [prev_char_field] = true } } },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm CopyMode static field-name variable config");

        assert_eq!(
            overrides.key_tables,
            Some(BTreeMap::from([(
                "copy_mode".to_owned(),
                vec![
                    NativeUserKeyAssignment {
                        keys: "v".to_owned(),
                        command: WindowCommand::CopyMode(
                            super::WindowCopyModeAssignment::SetSelectionMode(
                                super::WindowCopySelectionMode::Line,
                            ),
                        ),
                    },
                    NativeUserKeyAssignment {
                        keys: "f".to_owned(),
                        command: WindowCommand::CopyMode(
                            super::WindowCopyModeAssignment::StartJump {
                                forward: true,
                                prev_char: true,
                            },
                        ),
                    },
                ],
            )]))
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_copy_mode_static_nested_jump_table_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local jump_before = true
            local jump_opts = {
              prev_char = jump_before,
            }

            config.key_tables = {
              copy_mode = {
                { key = 'f', action = act.CopyMode { JumpForward = jump_opts } },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm CopyMode static nested jump table variable config");

        assert_eq!(
            overrides.key_tables,
            Some(BTreeMap::from([(
                "copy_mode".to_owned(),
                vec![NativeUserKeyAssignment {
                    keys: "f".to_owned(),
                    command: WindowCommand::CopyMode(super::WindowCopyModeAssignment::StartJump {
                        forward: true,
                        prev_char: true,
                    },),
                }],
            )]))
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_copy_mode_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local selection_mode = 'Block'
            local copy_opts = {
              SetSelectionMode = selection_mode,
            }

            config.key_tables = {
              copy_mode = {
                { key = 'v', action = act.CopyMode(copy_opts) },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm CopyMode static table variable call config");

        assert_eq!(
            overrides.key_tables,
            Some(BTreeMap::from([(
                "copy_mode".to_owned(),
                vec![NativeUserKeyAssignment {
                    keys: "v".to_owned(),
                    command: WindowCommand::CopyMode(
                        super::WindowCopyModeAssignment::SetSelectionMode(
                            super::WindowCopySelectionMode::Block,
                        ),
                    ),
                }],
            )]))
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_table_stack_actions() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|ALT',
                action = act.PopKeyTable,
              },
              {
                key = 'C',
                mods = 'CTRL|ALT',
                action = act.ClearKeyTableStack(),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm key-table stack action config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+P".to_owned(),
                    command: WindowCommand::PopKeyTable,
                },
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+C".to_owned(),
                    command: WindowCommand::ClearKeyTableStack,
                },
            ])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_table_stack_action_wrappers() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|ALT',
                action = wezterm.action { PopKeyTable = {} },
              },
              {
                key = 'C',
                mods = 'CTRL|ALT',
                action = act({ ClearKeyTableStack = { } }),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm key-table stack wrapper action config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+P".to_owned(),
                    command: WindowCommand::PopKeyTable,
                },
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+C".to_owned(),
                    command: WindowCommand::ClearKeyTableStack,
                },
            ])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_static_action_wrapper_table_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local wrapper = {
              SendString = 'from-wrapper-variable',
            }

            config.keys = {
              {
                key = 'W',
                mods = 'CTRL|ALT',
                action = act(wrapper),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm static wrapper-table action config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+W".to_owned(),
                command: WindowCommand::SendString("from-wrapper-variable".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_static_action_wrapper_table_variable_inner_comment() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local wrapper = {
              SendString = 'from-wrapper-variable',
            }
            local action_value = act( -- wrapper action
              wrapper
            )

            config.keys = {
              {
                key = 'W',
                mods = 'CTRL|ALT',
                action = action_value,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm static wrapper-table action config with inner comment");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+W".to_owned(),
                command: WindowCommand::SendString("from-wrapper-variable".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_static_action_wrapper_field_name_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local action_field = 'SendString'

            config.keys = {
              {
                key = 'W',
                mods = 'CTRL|ALT',
                action = wezterm.action { [action_field] = 'from-wrapper-field' },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm static wrapper action field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+W".to_owned(),
                command: WindowCommand::SendString("from-wrapper-field".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_return_key_tables_static_variable_assignment() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action

            local user_key_tables = {
              resize_pane = {
                { key = 'h', action = act.SendString 'from-return-table-variable' },
              },
            }

            return {
              keys = {
                {
                  key = 'Space',
                  mods = 'CTRL|SHIFT',
                  action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
                },
              },
              key_tables = user_key_tables,
            }
            "#,
        )
        .expect("expected WezTerm return-table static variable key_tables config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"from-return-table-variable"
        );
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_table_nested_table_insert_assignments() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = {
              resize_pane = {},
            }
            table.insert(config.key_tables.resize_pane, {
              key = 'h',
              action = act.SendString 'left',
            })

            return config
            "#,
        )
        .expect("expected WezTerm key table table.insert config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"left");
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_table_variable_nested_insert() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local resize_left = {
              key = 'h',
              action = act.SendString 'left',
            }

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = {
              resize_pane = {},
            }
            table.insert(config.key_tables.resize_pane, resize_left)

            return config
            "#,
        )
        .expect("expected WezTerm key table table.insert variable config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"left");
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_table_index_assignment() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = {
              resize_pane = {},
            }
            config.key_tables.resize_pane[1] = {
              key = 'h',
              action = act.SendString 'from-config-index',
            }

            return config
            "#,
        )
        .expect("expected WezTerm indexed key table assignment config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"from-config-index");
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_static_key_key_table_index_assignment() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local key_tables_field = 'key_tables'
            local config = {}

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config[key_tables_field] = {
              resize_pane = {},
            }
            config[key_tables_field].resize_pane[1] = {
              key = 'h',
              action = act.SendString 'from-static-key-table-index',
            }

            return config
            "#,
        )
        .expect("expected WezTerm static field-name indexed key table assignment config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"from-static-key-table-index"
        );
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_table_index_field_assignments() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = { resize_pane = {} }
            config.key_tables.resize_pane[1] = {}
            config.key_tables.resize_pane[1].key = 'h'
            config.key_tables.resize_pane[1].action = act.SendString 'from-config-index-fields'

            return config
            "#,
        )
        .expect("expected WezTerm indexed key_table field config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"from-config-index-fields"
        );
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_key_table_index_static_field_name_assignments() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local key_field = 'key'
            local action_field = 'action'

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = { resize_pane = {} }
            config.key_tables.resize_pane[1] = {}
            config.key_tables.resize_pane[1][key_field] = 'h'
            config.key_tables.resize_pane[1][action_field] =
              act.SendString 'from-config-index-static-fields'

            return config
            "#,
        )
        .expect("expected WezTerm indexed key_table static field-name config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"from-config-index-static-fields"
        );
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_static_key_key_table_index_field_assignments() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local key_tables_field = 'key_tables'
            local config = {}

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config[key_tables_field] = { resize_pane = {} }
            config[key_tables_field].resize_pane[1] = {}
            config[key_tables_field].resize_pane[1].key = 'h'
            config[key_tables_field].resize_pane[1].action =
              act.SendString 'from-static-key-table-index-fields'

            return config
            "#,
        )
        .expect("expected WezTerm static field-name indexed key table field config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"from-static-key-table-index-fields"
        );
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_table_length_append_assignment() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = {
              resize_pane = {},
            }
            config.key_tables.resize_pane[#config.key_tables.resize_pane + 1] = {
              key = 'h',
              action = act.SendString 'from-config-key-table-length',
            }

            return config
            "#,
        )
        .expect("expected WezTerm length-append key table assignment config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"from-config-key-table-length"
        );
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_static_key_key_table_length_append_assignment() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local key_tables_field = 'key_tables'
            local config = {}

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config[key_tables_field] = {
              resize_pane = {},
            }
            config[key_tables_field].resize_pane[#config[key_tables_field].resize_pane + 1] = {
              key = 'h',
              action = act.SendString 'from-static-key-table-length',
            }

            return config
            "#,
        )
        .expect("expected WezTerm static field-name length-append key table assignment config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"from-static-key-table-length"
        );
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_table_bracket_field_nested_insert_assignments() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config['key_tables'] = {
              resize_pane = {},
            }
            table.insert(config['key_tables'].resize_pane, {
              key = 'h',
              action = act.SendString 'left',
            })

            return config
            "#,
        )
        .expect("expected WezTerm key table bracket-field table.insert config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"left");
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_static_key_key_table_nested_insert_assignments() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local key_tables_field = 'key_tables'
            local config = {}

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config[key_tables_field] = {
              resize_pane = {},
            }
            table.insert(config[key_tables_field].resize_pane, {
              key = 'h',
              action = act.SendString 'from-static-key-table-insert',
            })

            return config
            "#,
        )
        .expect("expected WezTerm static field-name key table insert config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"from-static-key-table-insert"
        );
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_table_static_name_variable_nested_insert() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local mode_name = 'resize_pane'

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = {
              resize_pane = {},
            }
            table.insert(config.key_tables[mode_name], {
              key = 'h',
              action = act.SendString 'from-insert-name-variable',
            })

            return config
            "#,
        )
        .expect("expected WezTerm key table static name variable insert config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"from-insert-name-variable"
        );
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_table_positioned_nested_table_insert_assignments() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = {
              resize_pane = {
                {
                  key = 'h',
                  action = act.SendString 'original',
                },
              },
            }
            table.insert(config.key_tables.resize_pane, 1, {
              key = 'h',
              action = act.SendString 'inserted',
            })

            return config
            "#,
        )
        .expect("expected WezTerm positioned key table table.insert config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"inserted");
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_table_positioned_variable_insert() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local inserted_binding = {
              key = 'h',
              action = act.SendString 'inserted',
            }

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = {
              resize_pane = {
                {
                  key = 'h',
                  action = act.SendString 'original',
                },
              },
            }
            table.insert(config.key_tables.resize_pane, 1, inserted_binding)

            return config
            "#,
        )
        .expect("expected WezTerm positioned key table variable insert config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"inserted");
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_tables_config_long_bracket_key_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
              },
            }

            config.key_tables = {
              [[=[resize_pane]=]] = {
                { [[=[key]=]] = 'h', [[=[action]=]] = act.SendString 'left' },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm key_tables config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character(" ".into()),
            PhysicalKey::Code(WinitKeyCode::Space),
            Some(" "),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("h".into()),
            PhysicalKey::Code(WinitKeyCode::KeyH),
            Some("h"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"left");
        assert_eq!(app.active_key_table_for_test(), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_action_callback_placeholder() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.keys = {
              {
                key = 'I',
                mods = 'CTRL|SHIFT',
                action = wezterm.action_callback(function(win, pane)
                  wezterm.log_info('callback', win:window_id(), pane:pane_id())
                end),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm action_callback key config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+I".to_owned(),
                command: WindowCommand::Nop,
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_action_callback_perform_action() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'I',
                mods = 'CTRL|SHIFT',
                action = wezterm.action_callback(function(window, pane)
                  window:perform_action(act.SendString 'from-callback', pane)
                end),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm action_callback perform_action key config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character("i".into()),
            PhysicalKey::Code(WinitKeyCode::KeyI),
            Some("i"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"from-callback");
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_action_callback_perform_action_aliases() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'I',
                mods = 'CTRL|SHIFT',
                action = wezterm.action_callback(function(window, pane)
                  local win = window
                  local target = pane
                  win:perform_action(act.SendString 'from-callback-alias', target)
                end),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm action_callback perform_action alias key config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character("i".into()),
            PhysicalKey::Code(WinitKeyCode::KeyI),
            Some("i"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"from-callback-alias");
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_table_insert_assignments() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {}
            table.insert(config.keys, {
              key = 'K',
              mods = 'CTRL|SHIFT',
              action = act.SendString 'inserted',
            })

            return config
            "#,
        )
        .expect("expected WezTerm table.insert key config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+K".to_owned(),
                command: WindowCommand::SendString("inserted".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_static_action_alias() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local action = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'C',
                mods = 'CTRL|SHIFT',
                action = action.CopyTo('Clipboard'),
              },
              {
                key = 'V',
                mods = 'CTRL|SHIFT',
                action = action["PasteFrom"]('PrimarySelection'),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm static action alias key config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![
                NativeUserKeyAssignment {
                    keys: "CTRL|SHIFT+C".to_owned(),
                    command: WindowCommand::CopyTo(WindowCopyDestination::Clipboard),
                },
                NativeUserKeyAssignment {
                    keys: "CTRL|SHIFT+V".to_owned(),
                    command: WindowCommand::PasteFrom(WindowPasteSource::PrimarySelection),
                },
            ])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_static_action_alias_dotted_comment() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local action = wezterm -- action namespace
              .action
            local config = {}

            config.keys = {
              {
                key = 'C',
                mods = 'CTRL|SHIFT',
                action = action.CopyTo('Clipboard'),
              },
              {
                key = 'V',
                mods = 'CTRL|SHIFT',
                action = action["PasteFrom"]('PrimarySelection'),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm static action alias dotted-comment key config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![
                NativeUserKeyAssignment {
                    keys: "CTRL|SHIFT+C".to_owned(),
                    command: WindowCommand::CopyTo(WindowCopyDestination::Clipboard),
                },
                NativeUserKeyAssignment {
                    keys: "CTRL|SHIFT+V".to_owned(),
                    command: WindowCommand::PasteFrom(WindowPasteSource::PrimarySelection),
                },
            ])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_static_action_alias_static_key_module() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wt = require 'wezterm'
            local action_key = 'action'
            local action = wt[action_key]
            local config = {}

            config.keys = {
              {
                key = 'C',
                mods = 'CTRL|SHIFT',
                action = action.CopyTo('Clipboard'),
              },
              {
                key = 'V',
                mods = 'CTRL|SHIFT',
                action = action["PasteFrom"]('PrimarySelection'),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm static-key action alias key config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![
                NativeUserKeyAssignment {
                    keys: "CTRL|SHIFT+C".to_owned(),
                    command: WindowCommand::CopyTo(WindowCopyDestination::Clipboard),
                },
                NativeUserKeyAssignment {
                    keys: "CTRL|SHIFT+V".to_owned(),
                    command: WindowCommand::PasteFrom(WindowPasteSource::PrimarySelection),
                },
            ])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_table_insert_static_field_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local binding = {}
            binding.key = 'K'
            binding.mods = 'CTRL|SHIFT'
            binding.action = act.SendString 'from-insert-field-variable'

            config.keys = {}
            table.insert(config.keys, binding)

            return config
            "#,
        )
        .expect("expected WezTerm table.insert field-built key config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+K".to_owned(),
                command: WindowCommand::SendString("from-insert-field-variable".to_owned()),
            }])
        );
    }

