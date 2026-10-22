    #[test]
    fn window_app_dispatches_palette_clear_scrollback_mode_assignment_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("clearscrollback mode=scrollback only".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            [WindowCommand::ClearScrollback(
                WindowClearScrollbackMode::ScrollbackOnly
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_clear_scrollback_wezterm_action_table_call_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.ClearScrollback { mode = \"ScrollbackAndViewport\" }".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            [WindowCommand::ClearScrollback(
                WindowClearScrollbackMode::ScrollbackAndViewport
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_clear_scrollback_wezterm_action_table_long_bracket_key_query()
    {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.ClearScrollback { [[=[mode]=]] = [[ScrollbackAndViewport]] }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            [WindowCommand::ClearScrollback(
                WindowClearScrollbackMode::ScrollbackAndViewport
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_clear_scrollback_wezterm_action_parenthesized_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.ClearScrollback({ mode = \"ScrollbackOnly\" })".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            [WindowCommand::ClearScrollback(
                WindowClearScrollbackMode::ScrollbackOnly
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_clear_scrollback_wezterm_action_table_trailing_comma_query() {
        for query in [
            "wezterm.action.ClearScrollback { mode = \"ScrollbackAndViewport\", }",
            "wezterm.action.ClearScrollback({ mode = \"ScrollbackAndViewport\", })",
        ] {
            let mut app = NativeWindowApp::new(None);

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());

            assert_eq!(
                app.command_palette_filtered_commands(),
                [WindowCommand::ClearScrollback(
                    WindowClearScrollbackMode::ScrollbackAndViewport
                )]
            );
        }
    }

    #[test]
    fn window_app_dispatches_palette_clear_scrollback_wezterm_action_function_string_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.ClearScrollback('ScrollbackAndViewport')".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            [WindowCommand::ClearScrollback(
                WindowClearScrollbackMode::ScrollbackAndViewport
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_clear_scrollback_and_viewport_command() {
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
        app.command_palette_set_query("clear viewport".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].label(), "Clear Scrollback And Viewport");
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
    fn window_app_dispatches_palette_reset_terminal_command() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\x1b[31;1m\x1b[?25l")
            .unwrap();
        assert!(!app.runtime.terminal().cursor_visible());
        assert!(!app.runtime.terminal().scrollback().is_empty());
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query("reset terminal".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].label(), "Reset Terminal");
        app.command_palette_execute(commands[0].clone());

        assert!(app.runtime.terminal().scrollback().is_empty());
        assert_eq!(app.current_scrollback_offset(), 0);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "    ");
        assert_eq!(snapshot_row_text(&app.snapshot, 1, 4), "    ");
        assert_eq!(app.runtime.terminal().cursor(), (0, 0));
        assert!(app.runtime.terminal().cursor_visible());
        let reset_cell = app.runtime.terminal().grid().get(0, 0).unwrap();
        assert_eq!(reset_cell.foreground, rterm_terminal::Color::Default);
        assert!(!reset_cell.bold);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scrollback_navigation_commands() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 3);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ScrollToTop);
        assert_eq!(app.current_scrollback_offset(), 3);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('a'));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ScrollToBottom);
        assert_eq!(app.current_scrollback_offset(), 0);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('d'));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ScrollPageUp);
        assert_eq!(app.current_scrollback_offset(), 2);
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ScrollLineDown);
        assert_eq!(app.current_scrollback_offset(), 1);
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ScrollLineUp);
        assert_eq!(app.current_scrollback_offset(), 2);
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ScrollPageDown);
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scrollback_action_name_queries() {
        for (query, expected) in [
            (
                "scrollbypage -0.5",
                WindowCommand::ScrollByPage(WindowScrollByPageAmount::from_per_mille(-500)),
            ),
            ("scrollbyline -2", WindowCommand::ScrollByLine(-2)),
            ("scrolltoprompt 1", WindowCommand::ScrollToPrompt(1)),
        ] {
            let mut app = NativeWindowApp::new(None);
            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());

            assert_eq!(app.command_palette_filtered_commands(), vec![expected]);
        }
    }

    #[test]
    fn window_app_dispatches_native_scroll_by_page_payload() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 3);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ScrollByPage(
            WindowScrollByPageAmount::from_per_mille(-500),
        ));
        assert_eq!(app.current_scrollback_offset(), 1);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('c'));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ScrollByPage(
            WindowScrollByPageAmount::from_per_mille(1_000),
        ));
        assert_eq!(app.current_scrollback_offset(), 0);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('d'));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_scroll_by_line_payload() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 3);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ScrollByLine(-2));
        assert_eq!(app.current_scrollback_offset(), 2);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('b'));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ScrollByLine(1));
        assert_eq!(app.current_scrollback_offset(), 1);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('c'));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_scroll_to_prompt_payload() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(
            b"\x1b]133;A\x07> one\r\nout1\r\n\x1b]133;A\x07> two\r\nout2\r\n\x1b]133;A\x07> three\r\nlive",
        )
        .unwrap();
        assert_eq!(app.runtime.terminal().semantic_prompt_rows(), &[0, 2, 4]);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ScrollToPrompt(-2));
        assert_eq!(app.current_scrollback_offset(), 4);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "> one   ");
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ScrollToPrompt(1));
        assert_eq!(app.current_scrollback_offset(), 2);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "> two   ");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_by_line_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 3);

        app.enter_command_palette_mode();
        app.command_palette_set_query("scroll by line -2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ScrollByLine(-2)]
        );

        app.command_palette_execute(WindowCommand::ScrollByLine(-2));

        assert_eq!(app.current_scrollback_offset(), 2);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('b'));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_by_line_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 3);

        app.enter_command_palette_mode();
        app.command_palette_set_query("scroll by line=-2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ScrollByLine(-2)]
        );

        app.command_palette_execute(WindowCommand::ScrollByLine(-2));

        assert_eq!(app.current_scrollback_offset(), 2);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('b'));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_by_line_compact_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 3);

        app.enter_command_palette_mode();
        app.command_palette_set_query("scrollbyline=-2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ScrollByLine(-2)]
        );

        app.command_palette_execute(WindowCommand::ScrollByLine(-2));

        assert_eq!(app.current_scrollback_offset(), 2);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('b'));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_by_line_amount_assignment_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 3);

        app.enter_command_palette_mode();
        app.command_palette_set_query("scrollbyline amount=-2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ScrollByLine(-2)]
        );

        app.command_palette_execute(WindowCommand::ScrollByLine(-2));

        assert_eq!(app.current_scrollback_offset(), 2);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('b'));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_by_line_wezterm_action_function_call_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 3);

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.ScrollByLine(-2)".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ScrollByLine(-2)]
        );

        app.command_palette_execute(WindowCommand::ScrollByLine(-2));

        assert_eq!(app.current_scrollback_offset(), 2);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('b'));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_by_line_wezterm_action_table_wrapper_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 3);

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action { ScrollByLine = -2 }".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ScrollByLine(-2)]
        );

        app.command_palette_execute(WindowCommand::ScrollByLine(-2));

        assert_eq!(app.current_scrollback_offset(), 2);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('b'));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_by_page_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 3);

        app.enter_command_palette_mode();
        app.command_palette_set_query("scroll by page -0.5".to_owned());

        let expected = WindowCommand::ScrollByPage(WindowScrollByPageAmount::from_per_mille(-500));
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(app.current_scrollback_offset(), 1);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('c'));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_by_page_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 3);

        app.enter_command_palette_mode();
        app.command_palette_set_query("scroll by page=-0.5".to_owned());

        let expected = WindowCommand::ScrollByPage(WindowScrollByPageAmount::from_per_mille(-500));
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(app.current_scrollback_offset(), 1);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('c'));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_by_page_compact_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 3);

        app.enter_command_palette_mode();
        app.command_palette_set_query("scrollbypage=-0.5".to_owned());

        let expected = WindowCommand::ScrollByPage(WindowScrollByPageAmount::from_per_mille(-500));
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(app.current_scrollback_offset(), 1);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('c'));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_by_page_amount_assignment_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 3);

        app.enter_command_palette_mode();
        app.command_palette_set_query("scrollbypage amount=-0.5".to_owned());

        let expected = WindowCommand::ScrollByPage(WindowScrollByPageAmount::from_per_mille(-500));
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(app.current_scrollback_offset(), 1);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('c'));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_by_page_wezterm_action_function_call_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 3);

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.ScrollByPage(-0.5)".to_owned());

        let expected = WindowCommand::ScrollByPage(WindowScrollByPageAmount::from_per_mille(-500));
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(app.current_scrollback_offset(), 1);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('c'));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_by_page_wezterm_action_table_wrapper_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        assert_eq!(app.runtime.terminal().scrollback().len(), 3);

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action { ScrollByPage = -0.5 }".to_owned());

        let expected = WindowCommand::ScrollByPage(WindowScrollByPageAmount::from_per_mille(-500));
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(app.current_scrollback_offset(), 1);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('c'));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_to_prompt_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(
            b"\x1b]133;A\x07> one\r\nout1\r\n\x1b]133;A\x07> two\r\nout2\r\n\x1b]133;A\x07> three\r\nlive",
        )
        .unwrap();
        assert_eq!(app.runtime.terminal().semantic_prompt_rows(), &[0, 2, 4]);

        app.enter_command_palette_mode();
        app.command_palette_set_query("scroll to prompt -2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ScrollToPrompt(-2)]
        );

        app.command_palette_execute(WindowCommand::ScrollToPrompt(-2));

        assert_eq!(app.current_scrollback_offset(), 4);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "> one   ");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_to_prompt_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(
            b"\x1b]133;A\x07> one\r\nout1\r\n\x1b]133;A\x07> two\r\nout2\r\n\x1b]133;A\x07> three\r\nlive",
        )
        .unwrap();
        assert_eq!(app.runtime.terminal().semantic_prompt_rows(), &[0, 2, 4]);

        app.enter_command_palette_mode();
        app.command_palette_set_query("scroll to prompt=-2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ScrollToPrompt(-2)]
        );

        app.command_palette_execute(WindowCommand::ScrollToPrompt(-2));

        assert_eq!(app.current_scrollback_offset(), 4);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "> one   ");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_to_prompt_compact_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(
            b"\x1b]133;A\x07> one\r\nout1\r\n\x1b]133;A\x07> two\r\nout2\r\n\x1b]133;A\x07> three\r\nlive",
        )
        .unwrap();
        assert_eq!(app.runtime.terminal().semantic_prompt_rows(), &[0, 2, 4]);

        app.enter_command_palette_mode();
        app.command_palette_set_query("scrolltoprompt=-2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ScrollToPrompt(-2)]
        );

        app.command_palette_execute(WindowCommand::ScrollToPrompt(-2));

        assert_eq!(app.current_scrollback_offset(), 4);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "> one   ");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_to_prompt_amount_assignment_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(
            b"\x1b]133;A\x07> one\r\nout1\r\n\x1b]133;A\x07> two\r\nout2\r\n\x1b]133;A\x07> three\r\nlive",
        )
        .unwrap();
        assert_eq!(app.runtime.terminal().semantic_prompt_rows(), &[0, 2, 4]);

        app.enter_command_palette_mode();
        app.command_palette_set_query("scrolltoprompt amount=-2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ScrollToPrompt(-2)]
        );

        app.command_palette_execute(WindowCommand::ScrollToPrompt(-2));

        assert_eq!(app.current_scrollback_offset(), 4);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "> one   ");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_to_prompt_wezterm_action_function_call_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(
            b"\x1b]133;A\x07> one\r\nout1\r\n\x1b]133;A\x07> two\r\nout2\r\n\x1b]133;A\x07> three\r\nlive",
        )
        .unwrap();
        assert_eq!(app.runtime.terminal().semantic_prompt_rows(), &[0, 2, 4]);

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.ScrollToPrompt(-2)".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ScrollToPrompt(-2)]
        );

        app.command_palette_execute(WindowCommand::ScrollToPrompt(-2));

        assert_eq!(app.current_scrollback_offset(), 4);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "> one   ");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_to_prompt_wezterm_action_table_wrapper_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(
            b"\x1b]133;A\x07> one\r\nout1\r\n\x1b]133;A\x07> two\r\nout2\r\n\x1b]133;A\x07> three\r\nlive",
        )
        .unwrap();
        assert_eq!(app.runtime.terminal().semantic_prompt_rows(), &[0, 2, 4]);

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action { ScrollToPrompt = -2 }".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ScrollToPrompt(-2)]
        );

        app.command_palette_execute(WindowCommand::ScrollToPrompt(-2));

        assert_eq!(app.current_scrollback_offset(), 4);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "> one   ");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_scroll_to_prompt_commands() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(
            b"\x1b]133;A\x07> one\r\nout1\r\n\x1b]133;A\x07> two\r\nout2\r\n\x1b]133;A\x07> three\r\nlive",
        )
        .unwrap();

        assert_eq!(app.runtime.terminal().scrollback().len(), 4);
        assert_eq!(app.runtime.terminal().semantic_prompt_rows(), &[0, 2, 4]);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "> three ");
        assert_eq!(snapshot_row_text(&app.snapshot, 1, 8), "live    ");

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ScrollToPreviousPrompt);

        assert_eq!(app.current_scrollback_offset(), 2);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "> two   ");
        assert_eq!(snapshot_row_text(&app.snapshot, 1, 8), "out2    ");
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ScrollToPreviousPrompt);

        assert_eq!(app.current_scrollback_offset(), 4);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "> one   ");
        assert_eq!(snapshot_row_text(&app.snapshot, 1, 8), "out1    ");
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ScrollToNextPrompt);

        assert_eq!(app.current_scrollback_offset(), 2);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 8), "> two   ");
        assert_eq!(snapshot_row_text(&app.snapshot, 1, 8), "out2    ");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_clear_selection_command() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 1));
        app.handle_pty_output(b"abcd").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 1 },
            SelectionCell { row: 0, column: 2 },
        );
        app.refresh_snapshot();
        assert!(rendered_active_pane_cell(&app, 0, 1).unwrap().inverse);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ClearSelection);

        assert!(app.selection.is_none());
        assert!(!rendered_active_pane_cell(&app, 0, 1).unwrap().inverse);
        assert!(!rendered_active_pane_cell(&app, 0, 2).unwrap().inverse);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_copy_to_clipboard_command() {
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&copied);
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 1));
        app.handle_pty_output(b"abcd").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 1 },
            SelectionCell { row: 0, column: 2 },
        );
        app.refresh_snapshot();
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded.lock().unwrap().push(text.to_owned());
            true
        });

        app.enter_command_palette_mode();
        app.command_palette_set_query("copy clipboard".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands
            .into_iter()
            .find(|command| command.label() == "Copy To Clipboard")
            .expect("expected clipboard copy command");
        app.command_palette_execute(command);

        assert_eq!(copied.lock().unwrap().as_slice(), ["bc"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_copy_to_primary_selection_command() {
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
        app.command_palette_set_query("copy primary".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands
            .into_iter()
            .find(|command| command.label() == "Copy To Primary Selection")
            .expect("expected primary selection copy command");
        app.command_palette_execute(command);

        assert!(clipboard_copied.lock().unwrap().is_empty());
        assert_eq!(primary_copied.lock().unwrap().as_slice(), ["bc"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_copy_to_clipboard_and_primary_selection_command() {
        let clipboard_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_clipboard = Arc::clone(&clipboard_copied);
        let primary_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_primary = Arc::clone(&primary_copied);
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 1));
        app.handle_pty_output(b"abcd").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 1 },
            SelectionCell { row: 0, column: 2 },
        );
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
        app.command_palette_set_query("copy clipboard primary".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(
            commands[0].label(),
            "Copy To Clipboard And Primary Selection"
        );
        app.command_palette_execute(commands[0].clone());

        assert_eq!(clipboard_copied.lock().unwrap().as_slice(), ["bc"]);
        assert_eq!(primary_copied.lock().unwrap().as_slice(), ["bc"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_copy_to_clipboard_and_primary_selection_query() {
        let clipboard_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_clipboard = Arc::clone(&clipboard_copied);
        let primary_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_primary = Arc::clone(&primary_copied);
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 1));
        app.handle_pty_output(b"abcd").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 1 },
            SelectionCell { row: 0, column: 2 },
        );
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
        app.command_palette_set_query("copy to clipboard and primary selection".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(
            commands,
            [WindowCommand::CopyTo(
                WindowCopyDestination::ClipboardAndPrimarySelection
            )]
        );
        app.command_palette_execute(commands[0].clone());

        assert_eq!(clipboard_copied.lock().unwrap().as_slice(), ["bc"]);
        assert_eq!(primary_copied.lock().unwrap().as_slice(), ["bc"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_copy_to_quoted_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("copy to \"clipboard and primary selection\"".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            [WindowCommand::CopyTo(
                WindowCopyDestination::ClipboardAndPrimarySelection
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_copy_to_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("copy to=primary selection".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            [WindowCommand::CopyTo(
                WindowCopyDestination::PrimarySelection
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_copy_to_destination_assignment_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("copyto destination=clipboard".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            [WindowCommand::CopyTo(WindowCopyDestination::Clipboard)]
        );
    }

    #[test]
    fn window_app_dispatches_palette_copy_to_wezterm_action_bare_string_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.CopyTo 'ClipboardAndPrimarySelection'".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            [WindowCommand::CopyTo(
                WindowCopyDestination::ClipboardAndPrimarySelection
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_copy_to_wezterm_action_function_call_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.CopyTo('PrimarySelection')".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            [WindowCommand::CopyTo(
                WindowCopyDestination::PrimarySelection
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_copy_text_to_wezterm_action_table_query() {
        let clipboard_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_clipboard = Arc::clone(&clipboard_copied);
        let primary_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_primary = Arc::clone(&primary_copied);
        let mut app = NativeWindowApp::new(None);
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_clipboard.lock().unwrap().push(text.to_owned());
            true
        });
        app.primary_selection_writer = Box::new(move |text: &str| {
            recorded_primary.lock().unwrap().push(text.to_owned());
            true
        });

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.CopyTextTo { text = 'literal text', destination = 'PrimarySelection' }"
                .to_owned(),
        );

        let expected = WindowCommand::CopyTextTo {
            text: "literal text".to_owned(),
            destination: WindowCopyDestination::PrimarySelection,
        };
        assert_eq!(
            app.command_palette_filtered_commands(),
            std::slice::from_ref(&expected)
        );
        assert!(app.command_palette_execute(expected));

        assert!(clipboard_copied.lock().unwrap().is_empty());
        assert_eq!(primary_copied.lock().unwrap().as_slice(), ["literal text"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_copy_text_to_wezterm_action_table_wrapper_query() {
        let clipboard_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_clipboard = Arc::clone(&clipboard_copied);
        let primary_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_primary = Arc::clone(&primary_copied);
        let mut app = NativeWindowApp::new(None);
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_clipboard.lock().unwrap().push(text.to_owned());
            true
        });
        app.primary_selection_writer = Box::new(move |text: &str| {
            recorded_primary.lock().unwrap().push(text.to_owned());
            true
        });

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action { CopyTextTo = { text = 'literal text', destination = 'ClipboardAndPrimarySelection' } }"
                .to_owned(),
        );

        let expected = WindowCommand::CopyTextTo {
            text: "literal text".to_owned(),
            destination: WindowCopyDestination::ClipboardAndPrimarySelection,
        };
        assert_eq!(
            app.command_palette_filtered_commands(),
            std::slice::from_ref(&expected)
        );
        assert!(app.command_palette_execute(expected));

        assert_eq!(
            clipboard_copied.lock().unwrap().as_slice(),
            ["literal text"]
        );
        assert_eq!(primary_copied.lock().unwrap().as_slice(), ["literal text"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_copy_to_clipboard_and_primary_selection_payload() {
        let clipboard_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_clipboard = Arc::clone(&clipboard_copied);
        let primary_copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_primary = Arc::clone(&primary_copied);
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 1));
        app.handle_pty_output(b"abcd").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 1 },
            SelectionCell { row: 0, column: 2 },
        );
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
        app.command_palette_execute(WindowCommand::CopyTo(
            WindowCopyDestination::ClipboardAndPrimarySelection,
        ));

        assert_eq!(clipboard_copied.lock().unwrap().as_slice(), ["bc"]);
        assert_eq!(primary_copied.lock().unwrap().as_slice(), ["bc"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_complete_selection_to_primary_selection_payload() {
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
        app.command_palette_execute(WindowCommand::CompleteSelectionTo(
            WindowCopyDestination::PrimarySelection,
        ));

        assert!(!app.selecting);
        assert!(clipboard_copied.lock().unwrap().is_empty());
        assert_eq!(primary_copied.lock().unwrap().as_slice(), ["bc"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_complete_selection_or_open_link_to_primary_selection_payload() {
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
        app.command_palette_execute(WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(
            WindowCopyDestination::PrimarySelection,
        ));

        assert!(!app.selecting);
        assert!(clipboard_copied.lock().unwrap().is_empty());
        assert_eq!(primary_copied.lock().unwrap().as_slice(), ["bc"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_deprecated_copy_alias_to_clipboard() {
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&copied);
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 1));
        app.handle_pty_output(b"abcd").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 1 },
            SelectionCell { row: 0, column: 2 },
        );
        app.refresh_snapshot();
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded.lock().unwrap().push(text.to_owned());
            true
        });

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::Copy);

        assert_eq!(copied.lock().unwrap().as_slice(), ["bc"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_paste_from_clipboard_command() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.clipboard_reader = Box::new(|| Some("paste\ntext".to_owned()));

        app.enter_command_palette_mode();
        app.command_palette_set_query("paste clipboard".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].label(), "Paste From Clipboard");
        app.command_palette_execute(commands[0].clone());

        let expected =
            encode_window_paste("paste\ntext", false, DEFAULT_CANONICALIZE_PASTED_NEWLINES);
        assert_eq!(written.lock().unwrap().as_slice(), expected.as_slice());
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_can_disable_paste_newline_canonicalization() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.clipboard_reader = Box::new(|| Some("paste\ntext".to_owned()));
        app.set_config_overrides(native_config_snapshot! {
            canonicalize_pasted_newlines: Some(NativeCanonicalizePastedNewlines::None),
            ..NativeConfigSnapshot::default()
        });

        app.command_palette_execute(WindowCommand::PasteFrom(WindowPasteSource::Clipboard));

        assert_eq!(written.lock().unwrap().as_slice(), b"paste\ntext");
    }

    #[test]
    fn window_app_dispatches_palette_paste_from_primary_selection_command() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.clipboard_reader = Box::new(|| Some("clipboard".to_owned()));
        app.primary_selection_reader = Box::new(|| Some("primary\ntext".to_owned()));

        app.enter_command_palette_mode();
        app.command_palette_set_query("paste primary".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].label(), "Paste From Primary Selection");
        app.command_palette_execute(commands[0].clone());

        let expected =
            encode_window_paste("primary\ntext", false, DEFAULT_CANONICALIZE_PASTED_NEWLINES);
        assert_eq!(written.lock().unwrap().as_slice(), expected.as_slice());
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_deprecated_paste_alias_to_clipboard() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.clipboard_reader = Box::new(|| Some("paste\ntext".to_owned()));

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::Paste);

        let expected =
            encode_window_paste("paste\ntext", false, DEFAULT_CANONICALIZE_PASTED_NEWLINES);
        assert_eq!(written.lock().unwrap().as_slice(), expected.as_slice());
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_deprecated_paste_primary_selection_alias() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.clipboard_reader = Box::new(|| Some("clipboard".to_owned()));
        app.primary_selection_reader = Box::new(|| Some("primary\ntext".to_owned()));

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::PastePrimarySelection);

        let expected =
            encode_window_paste("primary\ntext", false, DEFAULT_CANONICALIZE_PASTED_NEWLINES);
        assert_eq!(written.lock().unwrap().as_slice(), expected.as_slice());
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_paste_from_primary_selection_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.clipboard_reader = Box::new(|| Some("clipboard".to_owned()));
        app.primary_selection_reader = Box::new(|| Some("primary\ntext".to_owned()));

        app.enter_command_palette_mode();
        app.command_palette_set_query("paste from primary selection".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(
            commands,
            [WindowCommand::PasteFrom(
                WindowPasteSource::PrimarySelection
            )]
        );
        app.command_palette_execute(commands[0].clone());

        let expected =
            encode_window_paste("primary\ntext", false, DEFAULT_CANONICALIZE_PASTED_NEWLINES);
        assert_eq!(written.lock().unwrap().as_slice(), expected.as_slice());
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_paste_from_quoted_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("paste from \"primary selection\"".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            [WindowCommand::PasteFrom(
                WindowPasteSource::PrimarySelection
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_paste_from_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("paste from=primary selection".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            [WindowCommand::PasteFrom(
                WindowPasteSource::PrimarySelection
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_paste_from_source_assignment_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("pastefrom source=primary selection".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            [WindowCommand::PasteFrom(
                WindowPasteSource::PrimarySelection
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_paste_from_wezterm_action_bare_string_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.PasteFrom 'PrimarySelection'".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            [WindowCommand::PasteFrom(
                WindowPasteSource::PrimarySelection
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_paste_from_wezterm_action_function_call_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.PasteFrom('PrimarySelection')".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            [WindowCommand::PasteFrom(
                WindowPasteSource::PrimarySelection
            )]
        );
    }

    #[test]
    fn window_app_dispatches_native_paste_from_primary_selection_payload() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.clipboard_reader = Box::new(|| Some("clipboard".to_owned()));
        app.primary_selection_reader = Box::new(|| Some("primary\ntext".to_owned()));

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::PasteFrom(
            WindowPasteSource::PrimarySelection,
        ));

        let expected =
            encode_window_paste("primary\ntext", false, DEFAULT_CANONICALIZE_PASTED_NEWLINES);
        assert_eq!(written.lock().unwrap().as_slice(), expected.as_slice());
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_send_string_payload() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::SendString("\x1b b".to_owned()));

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b b");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_string_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query("send string alpha beta".to_owned());

        let expected = WindowCommand::SendString("alpha beta".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(written.lock().unwrap().as_slice(), b"alpha beta");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_string_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("send string=alpha beta".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SendString("alpha beta".to_owned())]
        );
    }

    #[test]
    fn window_app_dispatches_palette_send_string_compact_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("sendstring=alpha beta".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SendString("alpha beta".to_owned())]
        );
    }

    #[test]
    fn window_app_dispatches_palette_send_string_string_assignment_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("sendstring string=alpha beta".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SendString("alpha beta".to_owned())]
        );
    }

    #[test]
    fn window_app_dispatches_palette_send_string_action_name_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query("sendstring \"alpha beta\"".to_owned());

        let expected = WindowCommand::SendString("alpha beta".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(written.lock().unwrap().as_slice(), b"alpha beta");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_string_wezterm_action_table_call_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SendString { string = \"alpha beta\" }".to_owned(),
        );

        let expected = WindowCommand::SendString("alpha beta".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(written.lock().unwrap().as_slice(), b"alpha beta");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_string_wezterm_action_table_long_bracket_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SendString { string = [[alpha, beta]] }".to_owned(),
        );

        let expected = WindowCommand::SendString("alpha, beta".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(written.lock().unwrap().as_slice(), b"alpha, beta");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_string_wezterm_action_table_long_bracket_key_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SendString { [[=[string]=]] = [[alpha beta]] }".to_owned(),
        );

        let expected = WindowCommand::SendString("alpha beta".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(written.lock().unwrap().as_slice(), b"alpha beta");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_string_wezterm_action_table_wrapper_long_bracket_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action { SendString = [[alpha beta]] }".to_owned());

        let expected = WindowCommand::SendString("alpha beta".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(written.lock().unwrap().as_slice(), b"alpha beta");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_string_table_wrapper_long_bracket_key_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action { [[=[SendString]=]] = [[alpha beta]] }".to_owned(),
        );

        let expected = WindowCommand::SendString("alpha beta".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(written.lock().unwrap().as_slice(), b"alpha beta");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_string_wezterm_action_index_long_bracket_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query("act[\"SendString\"] [[alpha beta]]".to_owned());

        let expected = WindowCommand::SendString("alpha beta".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(written.lock().unwrap().as_slice(), b"alpha beta");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_string_wezterm_action_parenthesized_table_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SendString({ string = \"alpha beta\" })".to_owned(),
        );

        let expected = WindowCommand::SendString("alpha beta".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(written.lock().unwrap().as_slice(), b"alpha beta");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_string_wezterm_action_table_trailing_comma_query() {
        for query in [
            "wezterm.action.SendString { string = \"alpha beta\", }",
            "wezterm.action.SendString({ string = \"alpha beta\", })",
        ] {
            let mut app = NativeWindowApp::new(None);

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());

            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![WindowCommand::SendString("alpha beta".to_owned())]
            );
        }
    }

    #[test]
    fn window_app_dispatches_palette_send_string_wezterm_action_index_function_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query("act[\"SendString\"](\"alpha beta\")".to_owned());

        let expected = WindowCommand::SendString("alpha beta".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(written.lock().unwrap().as_slice(), b"alpha beta");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_string_wezterm_action_index_bare_string_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query("act[\"SendString\"] \"alpha beta\"".to_owned());

        let expected = WindowCommand::SendString("alpha beta".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(written.lock().unwrap().as_slice(), b"alpha beta");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_string_wezterm_action_lua_hex_escape_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.SendString '\\x1b b'".to_owned());

        let expected = WindowCommand::SendString("\x1b b".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b b");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_string_wezterm_action_lua_decimal_escape_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.SendString '\\027 b'".to_owned());

        let expected = WindowCommand::SendString("\x1b b".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b b");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_string_wezterm_action_lua_unicode_escape_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.SendString '\\u{1b} b'".to_owned());

        let expected = WindowCommand::SendString("\x1b b".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b b");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_string_wezterm_action_lua_z_escape_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.SendString 'alpha\\z   beta'".to_owned());

        let expected = WindowCommand::SendString("alphabeta".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(written.lock().unwrap().as_slice(), b"alphabeta");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_string_wezterm_action_lua_long_bracket_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.SendString [[alpha beta]]".to_owned());

        let expected = WindowCommand::SendString("alpha beta".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(written.lock().unwrap().as_slice(), b"alpha beta");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_send_key_payload() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::SendKey(WindowSendKey {
            key: Key::Character("b".into()),
            modifiers: ModifiersState::ALT,
        }));

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1bb");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_key_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query("send key ALT+B".to_owned());

        let expected = WindowCommand::SendKey(WindowSendKey {
            key: Key::Character("b".into()),
            modifiers: ModifiersState::ALT,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1bb");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_key_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("send key=ALT+B".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SendKey(WindowSendKey {
                key: Key::Character("b".into()),
                modifiers: ModifiersState::ALT,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_send_key_compact_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("sendkey=ALT+B".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SendKey(WindowSendKey {
                key: Key::Character("b".into()),
                modifiers: ModifiersState::ALT,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_send_key_action_name_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();

        app.enter_command_palette_mode();
        app.command_palette_set_query("sendkey ALT+B".to_owned());

        let expected = WindowCommand::SendKey(WindowSendKey {
            key: Key::Character("b".into()),
            modifiers: ModifiersState::ALT,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1bb");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_send_key_field_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("SendKey key=LeftArrow mods=ALT".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SendKey(WindowSendKey {
                key: Key::Named(NamedKey::ArrowLeft),
                modifiers: ModifiersState::ALT,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_send_key_wezterm_action_table_call_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SendKey { key = \"LeftArrow\", mods = \"ALT\" }".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SendKey(WindowSendKey {
                key: Key::Named(NamedKey::ArrowLeft),
                modifiers: ModifiersState::ALT,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_send_key_wezterm_action_table_long_bracket_key_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SendKey { [[=[key]=]] = [[LeftArrow]], [[=[mods]=]] = [[ALT]] }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SendKey(WindowSendKey {
                key: Key::Named(NamedKey::ArrowLeft),
                modifiers: ModifiersState::ALT,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_send_key_wezterm_action_parenthesized_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SendKey({ key = \"LeftArrow\", mods = \"ALT\" })".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SendKey(WindowSendKey {
                key: Key::Named(NamedKey::ArrowLeft),
                modifiers: ModifiersState::ALT,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_send_key_wezterm_action_table_trailing_comma_query() {
        for query in [
            "wezterm.action.SendKey { key = \"LeftArrow\", mods = \"ALT\", }",
            "wezterm.action.SendKey({ key = \"LeftArrow\", mods = \"ALT\", })",
        ] {
            let mut app = NativeWindowApp::new(None);

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());

            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![WindowCommand::SendKey(WindowSendKey {
                    key: Key::Named(NamedKey::ArrowLeft),
                    modifiers: ModifiersState::ALT,
                })]
            );
        }
    }

    #[test]
    fn window_app_dispatches_palette_send_key_field_query_pipe_mods() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("SendKey mods=CTRL|SHIFT key=F5".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SendKey(WindowSendKey {
                key: Key::Named(NamedKey::F5),
                modifiers: ModifiersState::CONTROL | ModifiersState::SHIFT,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_send_key_query_pipe_mods() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("send key CTRL|SHIFT+F5".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SendKey(WindowSendKey {
                key: Key::Named(NamedKey::F5),
                modifiers: ModifiersState::CONTROL | ModifiersState::SHIFT,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_send_key_named_key_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query("send key ALT+LeftArrow".to_owned());
        let expected = WindowCommand::SendKey(WindowSendKey {
            key: Key::Named(NamedKey::ArrowLeft),
            modifiers: ModifiersState::ALT,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);
        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b[1;3D");

        app.enter_command_palette_mode();
        app.command_palette_set_query("send key CTRL+SHIFT+F5".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SendKey(WindowSendKey {
                key: Key::Named(NamedKey::F5),
                modifiers: ModifiersState::CONTROL | ModifiersState::SHIFT,
            })]
        );

        app.command_palette_set_query("send key CTRL+SHIFT+F35".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SendKey(WindowSendKey {
                key: Key::Named(NamedKey::F35),
                modifiers: ModifiersState::CONTROL | ModifiersState::SHIFT,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_rename_workspace_command() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::RenameWorkspace);

        assert_eq!(app.app_shell.active_workspace().name(), "default (renamed)");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_rename_workspace_command_with_query_name() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("rename workspace deploy".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected rename command");
        app.command_palette_execute(command);

        assert_eq!(app.app_shell.active_workspace().name(), "deploy");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_rename_workspace_command_with_equals_query_name() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("rename workspace=deploy".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected rename command");
        app.command_palette_execute(command);

        assert_eq!(app.app_shell.active_workspace().name(), "deploy");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_rename_workspace_command_with_name_assignment() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("renameworkspace name=deploy".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected rename command");
        app.command_palette_execute(command);

        assert_eq!(app.app_shell.active_workspace().name(), "deploy");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_rename_workspace_command_with_quoted_name_assignment() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("renameworkspace name=\"deploy west\"".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected rename command");
        app.command_palette_execute(command);

        assert_eq!(app.app_shell.active_workspace().name(), "deploy west");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_rename_workspace_command_with_quoted_query_name() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("rename workspace \"deploy west\"".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected rename command");
        app.command_palette_execute(command);

        assert_eq!(app.app_shell.active_workspace().name(), "deploy west");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_rename_tab_command() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::RenameTab);

        assert_eq!(
            app.app_shell.active_tab().title(),
            Some("PowerShell (renamed)")
        );
        assert!(app.command_palette.is_none());

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("PowerShell (renamed)"),
            "tab bar was {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_dispatches_palette_rename_tab_command_with_query_title() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("rename tab build-prod".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected rename command");
        app.command_palette_execute(command);

        assert_eq!(app.app_shell.active_tab().title(), Some("build-prod"));
        assert!(app.command_palette.is_none());

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("build-prod"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("PowerShell"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_dispatches_palette_rename_tab_command_with_equals_query_title() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("rename tab=build-prod".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected rename command");
        app.command_palette_execute(command);

        assert_eq!(app.app_shell.active_tab().title(), Some("build-prod"));
        assert!(app.command_palette.is_none());

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("build-prod"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("PowerShell"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_dispatches_palette_rename_tab_command_with_title_assignment() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("renametab title=build-prod".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected rename command");
        app.command_palette_execute(command);

        assert_eq!(app.app_shell.active_tab().title(), Some("build-prod"));
        assert!(app.command_palette.is_none());

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("build-prod"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("PowerShell"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_dispatches_palette_rename_tab_command_with_quoted_title_assignment() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("renametab title=\"build prod\"".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected rename command");
        app.command_palette_execute(command);

        assert_eq!(app.app_shell.active_tab().title(), Some("build prod"));
        assert!(app.command_palette.is_none());

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("build prod"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("PowerShell"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_dispatches_palette_rename_tab_command_with_quoted_query_title() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("rename tab \"build prod\"".to_owned());
        let commands = app.command_palette_filtered_commands();
        let command = commands.first().cloned().expect("expected rename command");
        app.command_palette_execute(command);

        assert_eq!(app.app_shell.active_tab().title(), Some("build prod"));
        assert!(app.command_palette.is_none());

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("build prod"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("PowerShell"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_dispatches_palette_resize_pane_command() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ResizePaneLeft);

        let split = app.app_shell.active_tab().panes()[1]
            .split()
            .expect("split should be present");
        assert_eq!(split.source_size_delta, -1);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_adjust_pane_size_payload() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::AdjustPaneSize {
            direction: rterm_core::app_shell::ResizeDirection::Left,
            amount: 3,
        });

        let split = app.app_shell.active_tab().panes()[1]
            .split()
            .expect("split should be present");
        assert_eq!(split.source_size_delta, -3);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_adjust_pane_size_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("adjust pane size left 4".to_owned());

        let expected = WindowCommand::AdjustPaneSize {
            direction: rterm_core::app_shell::ResizeDirection::Left,
            amount: 4,
        };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        let split = app.app_shell.active_tab().panes()[1]
            .split()
            .expect("split should be present");
        assert_eq!(split.source_size_delta, -4);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_adjust_pane_size_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("adjust pane size=left 4".to_owned());

        let expected = WindowCommand::AdjustPaneSize {
            direction: rterm_core::app_shell::ResizeDirection::Left,
            amount: 4,
        };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        let split = app.app_shell.active_tab().panes()[1]
            .split()
            .expect("split should be present");
        assert_eq!(split.source_size_delta, -4);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_adjust_pane_size_compact_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("adjustpanesize=left 4".to_owned());

        let expected = WindowCommand::AdjustPaneSize {
            direction: rterm_core::app_shell::ResizeDirection::Left,
            amount: 4,
        };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        let split = app.app_shell.active_tab().panes()[1]
            .split()
            .expect("split should be present");
        assert_eq!(split.source_size_delta, -4);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_adjust_pane_size_wezterm_action_table_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.AdjustPaneSize { 'Left', 4 }".to_owned());

        let expected = WindowCommand::AdjustPaneSize {
            direction: rterm_core::app_shell::ResizeDirection::Left,
            amount: 4,
        };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        let split = app.app_shell.active_tab().panes()[1]
            .split()
            .expect("split should be present");
        assert_eq!(split.source_size_delta, -4);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_adjust_pane_size_wezterm_action_parenthesized_table_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.AdjustPaneSize({ 'Left', 4 })".to_owned());

        let expected = WindowCommand::AdjustPaneSize {
            direction: rterm_core::app_shell::ResizeDirection::Left,
            amount: 4,
        };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        let split = app.app_shell.active_tab().panes()[1]
            .split()
            .expect("split should be present");
        assert_eq!(split.source_size_delta, -4);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_adjust_pane_size_wezterm_action_table_wrapper_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action { AdjustPaneSize = { 'Left', 4 } }".to_owned(),
        );

        let expected = WindowCommand::AdjustPaneSize {
            direction: rterm_core::app_shell::ResizeDirection::Left,
            amount: 4,
        };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        let split = app.app_shell.active_tab().panes()[1]
            .split()
            .expect("split should be present");
        assert_eq!(split.source_size_delta, -4);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_adjust_pane_size_wezterm_action_comment_before_table_wrapper_query()
     {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action -- resize options\n { AdjustPaneSize = { 'Left', 4 } }".to_owned(),
        );

        let expected = WindowCommand::AdjustPaneSize {
            direction: rterm_core::app_shell::ResizeDirection::Left,
            amount: 4,
        };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        let split = app.app_shell.active_tab().panes()[1]
            .split()
            .expect("split should be present");
        assert_eq!(split.source_size_delta, -4);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_adjust_pane_size_wezterm_action_trailing_table_wrapper_comment_query()
     {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action { AdjustPaneSize = { 'Left', 4 } } -- resize options".to_owned(),
        );

        let expected = WindowCommand::AdjustPaneSize {
            direction: rterm_core::app_shell::ResizeDirection::Left,
            amount: 4,
        };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        let split = app.app_shell.active_tab().panes()[1]
            .split()
            .expect("split should be present");
        assert_eq!(split.source_size_delta, -4);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_adjust_pane_size_wezterm_action_parenthesized_table_wrapper_inner_comment_query()
     {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action({ AdjustPaneSize = { 'Left', 4 } } -- resize options\n)".to_owned(),
        );

        let expected = WindowCommand::AdjustPaneSize {
            direction: rterm_core::app_shell::ResizeDirection::Left,
            amount: 4,
        };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        let split = app.app_shell.active_tab().panes()[1]
            .split()
            .expect("split should be present");
        assert_eq!(split.source_size_delta, -4);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_adjust_pane_size_wezterm_action_parenthesized_table_wrapper_leading_inner_comment_query()
     {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action( -- resize options\n { AdjustPaneSize = { 'Left', 4 } })".to_owned(),
        );

        let expected = WindowCommand::AdjustPaneSize {
            direction: rterm_core::app_shell::ResizeDirection::Left,
            amount: 4,
        };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        let split = app.app_shell.active_tab().panes()[1]
            .split()
            .expect("split should be present");
        assert_eq!(split.source_size_delta, -4);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_adjust_pane_size_wezterm_action_table_trailing_comma_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action { AdjustPaneSize = { 'Left', 4, } }".to_owned(),
        );

        let expected = WindowCommand::AdjustPaneSize {
            direction: rterm_core::app_shell::ResizeDirection::Left,
            amount: 4,
        };
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        let split = app.app_shell.active_tab().panes()[1]
            .split()
            .expect("split should be present");
        assert_eq!(split.source_size_delta, -4);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_toggle_pane_zoom_command() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::TogglePaneZoom);

        assert_eq!(
            app.app_shell.active_tab().zoomed_pane_id(),
            Some(rterm_core::PaneId::new(2))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_set_pane_zoom_state_commands() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ZoomPane);
        assert_eq!(
            app.app_shell.active_tab().zoomed_pane_id(),
            Some(rterm_core::PaneId::new(2))
        );
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ZoomPane);
        assert_eq!(
            app.app_shell.active_tab().zoomed_pane_id(),
            Some(rterm_core::PaneId::new(2))
        );
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::UnzoomPane);
        assert_eq!(app.app_shell.active_tab().zoomed_pane_id(), None);
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::UnzoomPane);
        assert_eq!(app.app_shell.active_tab().zoomed_pane_id(), None);
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_set_pane_zoom_state_payload() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::SetPaneZoomState(true));
        assert_eq!(
            app.app_shell.active_tab().zoomed_pane_id(),
            Some(rterm_core::PaneId::new(2))
        );
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::SetPaneZoomState(true));
        assert_eq!(
            app.app_shell.active_tab().zoomed_pane_id(),
            Some(rterm_core::PaneId::new(2))
        );
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::SetPaneZoomState(false));
        assert_eq!(app.app_shell.active_tab().zoomed_pane_id(), None);
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::SetPaneZoomState(false));
        assert_eq!(app.app_shell.active_tab().zoomed_pane_id(), None);
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_set_pane_zoom_state_true_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("set pane zoom state true".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SetPaneZoomState(true)]
        );

        app.command_palette_execute(WindowCommand::SetPaneZoomState(true));

        assert_eq!(
            app.app_shell.active_tab().zoomed_pane_id(),
            Some(rterm_core::PaneId::new(2))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_set_pane_zoom_state_action_name_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("setpanezoomstate true".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SetPaneZoomState(true)]
        );

        app.command_palette_execute(WindowCommand::SetPaneZoomState(true));

        assert_eq!(
            app.app_shell.active_tab().zoomed_pane_id(),
            Some(rterm_core::PaneId::new(2))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_set_pane_zoom_state_action_name_equals_true_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("setpanezoomstate=true".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SetPaneZoomState(true)]
        );

        app.command_palette_execute(WindowCommand::SetPaneZoomState(true));

        assert_eq!(
            app.app_shell.active_tab().zoomed_pane_id(),
            Some(rterm_core::PaneId::new(2))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_set_pane_zoom_state_zoomed_assignment_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("setpanezoomstate zoomed=true".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SetPaneZoomState(true)]
        );

        app.command_palette_execute(WindowCommand::SetPaneZoomState(true));

        assert_eq!(
            app.app_shell.active_tab().zoomed_pane_id(),
            Some(rterm_core::PaneId::new(2))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_set_pane_zoom_state_wezterm_action_function_call_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.SetPaneZoomState(true)".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SetPaneZoomState(true)]
        );

        app.command_palette_execute(WindowCommand::SetPaneZoomState(true));

        assert_eq!(
            app.app_shell.active_tab().zoomed_pane_id(),
            Some(rterm_core::PaneId::new(2))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_set_pane_zoom_state_false_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SetPaneZoomState {
            pane: rterm_core::PaneId::new(2),
            zoomed: true,
        })
        .unwrap();
        assert_eq!(
            app.app_shell.active_tab().zoomed_pane_id(),
            Some(rterm_core::PaneId::new(2))
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("set pane zoom state false".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SetPaneZoomState(false)]
        );

        app.command_palette_execute(WindowCommand::SetPaneZoomState(false));

        assert_eq!(app.app_shell.active_tab().zoomed_pane_id(), None);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_set_pane_zoom_state_equals_false_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SetPaneZoomState {
            pane: rterm_core::PaneId::new(2),
            zoomed: true,
        })
        .unwrap();
        assert_eq!(
            app.app_shell.active_tab().zoomed_pane_id(),
            Some(rterm_core::PaneId::new(2))
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("set pane zoom state=false".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SetPaneZoomState(false)]
        );

        app.command_palette_execute(WindowCommand::SetPaneZoomState(false));

        assert_eq!(app.app_shell.active_tab().zoomed_pane_id(), None);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_unzoom_on_switch_pane_false_blocks_directional_switch_when_zoomed() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            unzoom_on_switch_pane: Some(false),
            ..NativeConfigSnapshot::default()
        });
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
        app.dispatch_app_action(AppAction::TogglePaneZoom {
            pane: rterm_core::PaneId::new(1),
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ActivatePaneRight);

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
        assert_eq!(
            app.app_shell.active_tab().zoomed_pane_id(),
            Some(rterm_core::PaneId::new(1))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_unzoom_on_switch_pane_false_blocks_next_previous_pane_when_zoomed() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            unzoom_on_switch_pane: Some(false),
            ..NativeConfigSnapshot::default()
        });
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
        app.dispatch_app_action(AppAction::TogglePaneZoom {
            pane: rterm_core::PaneId::new(1),
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::NextPane);
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
        assert_eq!(
            app.app_shell.active_tab().zoomed_pane_id(),
            Some(rterm_core::PaneId::new(1))
        );
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::PreviousPane);
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
        assert_eq!(
            app.app_shell.active_tab().zoomed_pane_id(),
            Some(rterm_core::PaneId::new(1))
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn unmodified_t_is_not_shell_shortcut() {
        let app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        assert!(
            app.app_shell_action_for_key(&Key::Character("t".into()), ModifiersState::CONTROL)
                .is_none()
        );
    }

    #[test]
    fn ctrl_shift_r_is_not_workspace_rename_shortcut() {
        let app = NativeWindowApp::new(None);

        let action = app.app_shell_action_for_key(
            &Key::Character("r".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );

        assert!(action.is_none());
    }

    #[test]
    fn command_palette_switches_to_next_workspace() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "zeta".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "alpha".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SwitchWorkspace {
            workspace: rterm_core::WorkspaceId::new(1),
        })
        .unwrap();

        app.command_palette_execute(WindowCommand::NextWorkspace);
        assert_eq!(app.app_shell.active_workspace().name(), "zeta");
    }

    #[test]
    fn command_palette_switches_to_previous_workspace() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "zeta".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "alpha".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SwitchWorkspace {
            workspace: rterm_core::WorkspaceId::new(1),
        })
        .unwrap();

        app.command_palette_execute(WindowCommand::PreviousWorkspace);
        assert_eq!(app.app_shell.active_workspace().name(), "alpha");
    }

    #[test]
    fn window_app_dispatches_native_switch_workspace_relative_payload() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "zeta".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "alpha".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SwitchWorkspace {
            workspace: rterm_core::WorkspaceId::new(1),
        })
        .unwrap();

        app.command_palette_execute(WindowCommand::SwitchWorkspaceRelative(2));
        assert_eq!(app.app_shell.active_workspace().name(), "alpha");

        app.command_palette_execute(WindowCommand::SwitchWorkspaceRelative(-1));
        assert_eq!(app.app_shell.active_workspace().name(), "zeta");
    }

    #[test]
    fn window_app_dispatches_palette_switch_workspace_relative_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "zeta".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "alpha".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SwitchWorkspace {
            workspace: rterm_core::WorkspaceId::new(1),
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("switch workspace relative 2".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::SwitchWorkspaceRelative(2)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(app.app_shell.active_workspace().name(), "alpha");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_switch_workspace_relative_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "zeta".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "alpha".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SwitchWorkspace {
            workspace: rterm_core::WorkspaceId::new(1),
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("switch workspace relative=2".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::SwitchWorkspaceRelative(2)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(app.app_shell.active_workspace().name(), "alpha");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_switch_to_workspace_relative_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "zeta".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "alpha".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SwitchWorkspace {
            workspace: rterm_core::WorkspaceId::new(1),
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("switch to workspace relative=2".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::SwitchWorkspaceRelative(2)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(app.app_shell.active_workspace().name(), "alpha");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_switch_workspace_relative_action_name_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "zeta".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "alpha".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SwitchWorkspace {
            workspace: rterm_core::WorkspaceId::new(1),
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("switchworkspacerelative=2".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::SwitchWorkspaceRelative(2)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(app.app_shell.active_workspace().name(), "alpha");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_switch_workspace_relative_offset_assignment_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "zeta".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "alpha".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SwitchWorkspace {
            workspace: rterm_core::WorkspaceId::new(1),
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("switchworkspacerelative offset=2".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::SwitchWorkspaceRelative(2)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(app.app_shell.active_workspace().name(), "alpha");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_switch_workspace_relative_wezterm_action_function_call_query()
    {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "zeta".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "alpha".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SwitchWorkspace {
            workspace: rterm_core::WorkspaceId::new(1),
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.SwitchWorkspaceRelative(2)".to_owned());
        let commands = app.command_palette_filtered_commands();
        assert_eq!(commands, [WindowCommand::SwitchWorkspaceRelative(2)]);
        app.command_palette_execute(commands[0].clone());

        assert_eq!(app.app_shell.active_workspace().name(), "alpha");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn command_palette_switch_to_workspace_query_creates_named_workspace() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("switch workspace ops".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Switch To Workspace");
        assert!(app.command_palette_execute(WindowCommand::SwitchToWorkspace));

        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_eq!(app.app_shell.active_workspace().name(), "ops");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn command_palette_switch_to_workspace_query_accepts_equals_name() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("switch workspace=ops".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Switch To Workspace");
        assert!(app.command_palette_execute(WindowCommand::SwitchToWorkspace));

        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_eq!(app.app_shell.active_workspace().name(), "ops");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn command_palette_switch_to_workspace_query_accepts_name_assignment() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("switchtoworkspace name=ops".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Switch To Workspace");
        assert!(app.command_palette_execute(WindowCommand::SwitchToWorkspace));

        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_eq!(app.app_shell.active_workspace().name(), "ops");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn command_palette_switch_to_workspace_query_can_spawn_command() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("switch workspace monitoring spawn top -d 1".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Switch To Workspace");
        assert!(app.command_palette_execute(WindowCommand::SwitchToWorkspace));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_eq!(app.app_shell.active_workspace().name(), "monitoring");
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn command_palette_switch_to_workspace_wezterm_action_parenthesized_table_query_can_spawn_command()
     {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SwitchToWorkspace({ name = \"monitoring\", spawn = { args = { \"top\", \"-d\", \"1\" }, cwd = \"C:/Mon\" } })"
                .to_owned(),
        );
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Switch To Workspace");
        assert!(app.command_palette_execute(WindowCommand::SwitchToWorkspace));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_eq!(app.app_shell.active_workspace().name(), "monitoring");
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Mon"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn command_palette_switch_to_workspace_wezterm_action_table_query_can_spawn_command() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SwitchToWorkspace { name = \"monitoring\", spawn = { args = { \"top\", \"-d\", \"1\" }, cwd = \"C:/Mon\" } }"
                .to_owned(),
        );
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Switch To Workspace");
        assert!(app.command_palette_execute(WindowCommand::SwitchToWorkspace));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_eq!(app.app_shell.active_workspace().name(), "monitoring");
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Mon"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn command_palette_switch_workspace_table_long_bracket_key_query_can_spawn_command() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SwitchToWorkspace { [[=[name]=]] = [[monitoring]], [[=[spawn]=]] = { [[=[args]=]] = { [[top]], [[-d]], [[1]] }, [[=[cwd]=]] = [[C:/Mon]] } }"
                .to_owned(),
        );
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Switch To Workspace");
        assert!(app.command_palette_execute(WindowCommand::SwitchToWorkspace));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_eq!(app.app_shell.active_workspace().name(), "monitoring");
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Mon"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn command_palette_switch_workspace_spawn_options_long_bracket_key_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SwitchToWorkspace { name = [[monitoring]], spawn = { [[=[cwd]=]] = [[C:/Project Dir]], [[=[set_environment_variables]=]] = { [[=[SPAWN_MODE]=]] = [[query]] } } }"
                .to_owned(),
        );
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Switch To Workspace");
        assert!(app.command_palette_execute(WindowCommand::SwitchToWorkspace));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_eq!(app.app_shell.active_workspace().name(), "monitoring");
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-NoProfile"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert_eq!(
            launch.environment().get("SPAWN_MODE"),
            Some(&"query".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn command_palette_switch_to_workspace_query_accepts_mixed_case_spawn_marker() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("switch workspace monitoring Spawn top -d 1".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Switch To Workspace");
        assert!(app.command_palette_execute(WindowCommand::SwitchToWorkspace));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_eq!(app.app_shell.active_workspace().name(), "monitoring");
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn command_palette_switch_to_workspace_query_applies_spawn_options_without_program() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "switch workspace monitoring spawn --cwd \"C:/Project Dir\" --env \"SPAWN_MODE=query\""
                .to_owned(),
        );
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Switch To Workspace");
        assert!(app.command_palette_execute(WindowCommand::SwitchToWorkspace));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_eq!(app.app_shell.active_workspace().name(), "monitoring");
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-NoProfile"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert_eq!(
            launch.environment().get("SPAWN_MODE"),
            Some(&"query".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn command_palette_switch_to_workspace_query_keeps_spawn_word_in_quoted_name() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "switch workspace \"ops spawn review\" spawn --cwd \"C:/Project Dir\" powershell \"-No Logo\""
                .to_owned(),
        );
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Switch To Workspace");
        assert!(app.command_palette_execute(WindowCommand::SwitchToWorkspace));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_eq!(app.app_shell.active_workspace().name(), "ops spawn review");
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-No Logo"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn command_palette_switch_to_workspace_query_can_spawn_without_name() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "switch workspace spawn --cwd \"C:/Project Dir\" powershell \"-No Logo\"".to_owned(),
        );
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Switch To Workspace");
        assert!(app.command_palette_execute(WindowCommand::SwitchToWorkspace));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_ne!(app.app_shell.active_workspace().name(), "spawn");
        assert_ne!(
            app.app_shell.active_workspace().name(),
            "spawn --cwd \"C:/Project Dir\" powershell \"-No Logo\""
        );
        assert_eq!(app.active_workspace_id(), rterm_core::WorkspaceId::new(2));
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-No Logo"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn command_palette_switch_to_workspace_query_accepts_mixed_case_spawn_without_name() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "switch workspace Spawn --cwd \"C:/Project Dir\" powershell \"-No Logo\"".to_owned(),
        );
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Switch To Workspace");
        assert!(app.command_palette_execute(WindowCommand::SwitchToWorkspace));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_ne!(app.app_shell.active_workspace().name(), "Spawn");
        assert_ne!(
            app.app_shell.active_workspace().name(),
            "Spawn --cwd \"C:/Project Dir\" powershell \"-No Logo\""
        );
        assert_eq!(app.active_workspace_id(), rterm_core::WorkspaceId::new(2));
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-No Logo"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn command_palette_switch_to_workspace_query_applies_nameless_spawn_options_without_program() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "switch workspace spawn --cwd \"C:/Scratch Dir\" --env \"SPAWN_MODE=random\""
                .to_owned(),
        );
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Switch To Workspace");
        assert!(app.command_palette_execute(WindowCommand::SwitchToWorkspace));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_ne!(app.app_shell.active_workspace().name(), "spawn");
        assert_eq!(app.active_workspace_id(), rterm_core::WorkspaceId::new(2));
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-NoProfile"]);
        assert_eq!(launch.cwd(), Some("C:/Scratch Dir"));
        assert_eq!(
            launch.environment().get("SPAWN_MODE"),
            Some(&"random".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn command_palette_switch_to_workspace_without_query_creates_random_workspace() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::SwitchToWorkspace));

        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_ne!(app.app_shell.active_workspace().name(), "workspace-2");
        assert_eq!(app.active_workspace_id(), rterm_core::WorkspaceId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn command_palette_switch_to_workspace_wezterm_action_no_arg_query_creates_random_workspace() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.SwitchToWorkspace".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Switch To Workspace");
        assert!(app.command_palette_execute(WindowCommand::SwitchToWorkspace));

        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_ne!(app.app_shell.active_workspace().name(), "workspace-2");
        assert_eq!(app.active_workspace_id(), rterm_core::WorkspaceId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn command_palette_switch_to_workspace_query_selects_existing_named_workspace() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "ops".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "monitoring".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SwitchWorkspace {
            workspace: rterm_core::WorkspaceId::new(1),
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("switch workspace ops".to_owned());
        assert!(app.command_palette_execute(WindowCommand::SwitchToWorkspace));

        assert_eq!(app.app_shell.workspaces().len(), 3);
        assert_eq!(app.app_shell.active_workspace().name(), "ops");
        assert_eq!(app.active_workspace_id(), rterm_core::WorkspaceId::new(2));
    }

    #[test]
    fn window_app_dispatches_native_switch_to_workspace_args_spawn_payload() {
        let mut app = NativeWindowApp::new(None);

        assert!(
            app.command_palette_execute(WindowCommand::SwitchToWorkspaceArgs(
                WindowSwitchToWorkspaceOptions {
                    name: Some("monitoring".to_owned()),
                    command: Some(WindowSpawnCommandQuery {
                        label: None,
                        program: "top".to_owned(),
                        args: vec!["-d".to_owned(), "1".to_owned()],
                        cwd: Some("/tmp/project".to_owned()),
                        environment: BTreeMap::from([(
                            "WORKSPACE_MODE".to_owned(),
                            "native".to_owned()
                        )]),
                        domain: None,
                        window_position: None,
                    }),
                    command_options: None,
                },
            ))
        );

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_eq!(app.app_shell.active_workspace().name(), "monitoring");
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("/tmp/project"));
        assert_eq!(
            launch.environment().get("WORKSPACE_MODE"),
            Some(&"native".to_owned())
        );
    }

    #[test]
    fn window_app_switch_to_workspace_without_spawn_uses_default_prog_for_new_workspace() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            default_prog: Some(vec!["top".to_owned(), "-H".to_owned()]),
            ..NativeConfigSnapshot::default()
        });

        assert!(
            app.command_palette_execute(WindowCommand::SwitchToWorkspaceArgs(
                WindowSwitchToWorkspaceOptions {
                    name: Some("monitoring".to_owned()),
                    command: None,
                    command_options: None,
                },
            ))
        );

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.app_shell.active_workspace().name(), "monitoring");
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-H"]);
    }

    #[test]
    fn window_app_dispatches_native_show_launcher_args_workspace_payload() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "ops".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::NewWorkspace {
            name: "monitoring".to_owned(),
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SwitchWorkspace {
            workspace: rterm_core::WorkspaceId::new(1),
        })
        .unwrap();

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::workspaces(),
                title: Some("Pick Workspace".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));
        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Workspace: Select an item and press Enter=launch Esc=cancel /=filter [1 / 3] Switch To Workspace: default"
        );

        app.command_palette_set_query("ops".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Switch To Workspace: ops");
        assert!(app.command_palette_execute_entry(entries[0].clone()));

        assert_eq!(app.app_shell.active_workspace().name(), "ops");
        assert_eq!(app.active_workspace_id(), rterm_core::WorkspaceId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_show_launcher_args_tabs_payload() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Shell\x07").unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.handle_pty_output(b"\x1b]2;Logs\x07").unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.handle_pty_output(b"\x1b]2;Deploy\x07").unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::tabs(),
                title: Some("Pick Tab".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));
        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Tab: Select an item and press Enter=launch Esc=cancel /=filter [1 / 3] Activate Tab: Shell"
        );

        app.command_palette_set_query("deploy".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Activate Tab: Deploy");
        assert!(app.command_palette_execute_entry(entries[0].clone()));

        assert_eq!(app.app_shell.active_tab_id(), rterm_core::TabId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_show_launcher_args_commands_payload() {
        let mut app = NativeWindowApp::new(None);

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::commands(),
                title: Some("Pick Command".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));
        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            format!(
                "Pick Command: Select an item and press Enter=launch Esc=cancel /=filter [1 / {}] New Tab",
                WINDOW_COMMANDS.len()
            )
        );

        app.command_palette_set_query("toggle fullscreen".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Toggle Full Screen");
        assert!(app.command_palette_execute_entry(entries[0].clone()));

        assert!(app.full_screen_for_test());
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_show_launcher_args_key_assignments_payload() {
        let mut app = NativeWindowApp::new(None);

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::key_assignments(),
                title: Some("Pick Key".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));
        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            format!(
                "Pick Key: Select an item and press Enter=launch Esc=cancel /=filter [1 / {}] CTRL+SHIFT+T: New Tab",
                native_window_key_assignment_entries().len()
            )
        );

        app.command_palette_set_query("ctrl+shift+t: new".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "CTRL+SHIFT+T: New Tab");
        assert!(app.command_palette_execute_entry(entries[0].clone()));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_show_launcher_args_key_assignments_include_user_overrides() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            key_assignments: Some(vec![NativeUserKeyAssignment {
                keys: "CTRL+ALT+D".to_owned(),
                command: WindowCommand::ShowDebugOverlay,
            }]),
            ..NativeConfigSnapshot::default()
        });

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::key_assignments(),
                title: Some("Pick Key".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));

        app.command_palette_set_query("ctrl+alt+d".to_owned());
        let entries = app.command_palette_filtered_entries();
        let entry = entries
            .into_iter()
            .find(|entry| entry.label() == "CTRL+ALT+D: Show Debug Overlay")
            .expect("expected user key assignment entry");
        assert!(app.command_palette_execute_entry(entry));

        assert!(app.debug_overlay_active_for_test());
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_show_launcher_args_fuzzy_only_payload() {
        let mut app = NativeWindowApp::new(None);

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::fuzzy(),
                title: Some("Pick Fuzzy".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));
        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");

        assert!(app.command_palette_filtered_entries().is_empty());
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Fuzzy: Fuzzy matching: no commands"
        );
    }

    #[test]
    fn window_app_dispatches_native_show_launcher_args_domains_payload() {
        let mut app = NativeWindowApp::new(None);

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::domains(),
                title: Some("Pick Domain".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));
        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Domain: Select an item and press Enter=launch Esc=cancel /=filter [1 / 2] Spawn In Domain: local"
        );

        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].label(), "Spawn In Domain: local");
        assert_eq!(entries[1].label(), "Spawn In Domain: unix");

        app.command_palette_set_query("local".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Spawn In Domain: local");
        assert!(app.command_palette_execute_entry(entries[0].clone()));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_show_launcher_args_domains_payload_marks_local_entry_supported()
    {
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

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::domains(),
                title: Some("Pick Domain".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));
        let entries = app.command_palette_filtered_entries();
        assert!(entries.len() >= 2);

        let local_entry = entries
            .iter()
            .find(|entry| entry.label() == "Spawn In Domain: local")
            .expect("expected Spawn In Domain: local entry");
        let remote_entry = entries
            .iter()
            .find(|entry| entry.label() == "Spawn In Domain: remote-default")
            .expect("expected remote-default domain entry");

        if let WindowCommandPaletteEntry::Augmented(entry) = local_entry {
            assert!(entry.doc.is_none());
        } else {
            panic!("expected local domain entry to be augmented");
        }
        if let WindowCommandPaletteEntry::Augmented(entry) = remote_entry {
            assert_eq!(
                entry.doc.as_deref(),
                Some("Attach Domain actions are currently unsupported")
            );
        } else {
            panic!("expected remote domain entry to be augmented");
        }
    }

    #[test]
    fn window_app_dispatches_native_show_launcher_args_domains_payload_with_custom_domains() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            default_domain: Some("remote-default".to_owned()),
            exec_domains: Some(vec![NativeExecDomain {
                name: "ops".to_owned(),
                fixup_command: "wezterm cli spawn".to_owned(),
                label: None,
            }]),
            wsl_domains: Some(vec![NativeWslDomain {
                name: "wsl-ubuntu".to_owned(),
                distribution: Some("Ubuntu".to_owned()),
                username: Some("ops".to_owned()),
                default_cwd: Some("~".to_owned()),
                default_prog: Some(vec!["zsh".to_owned()]),
            }]),
            unix_domains: Some(vec![NativeUnixDomain {
                name: "ops-unix".to_owned(),
                socket_path: Some("/tmp/ops.sock".to_owned()),
                connect_automatically: true,
                no_serve_automatically: true,
                serve_command: None,
                proxy_command: None,
                skip_permissions_check: true,
                read_timeout_ms: 45_000,
                write_timeout_ms: 30_000,
                local_echo_threshold_ms: Some(12),
                overlay_lag_indicator: true,
            }]),
            ssh_domains: Some(vec![NativeSshDomain {
                name: "ops-ssh".to_owned(),
                remote_address: "ops.example.com:2222".to_owned(),
                no_agent_auth: true,
                username: Some("ops".to_owned()),
                connect_automatically: true,
                timeout_ms: 45_000,
                local_echo_threshold_ms: Some(12),
                overlay_lag_indicator: true,
                remote_wezterm_path: Some("/opt/wezterm/wezterm".to_owned()),
                override_proxy_command: Some("wezterm cli proxy --stdio".to_owned()),
                ssh_backend: Some(NativeSshBackend::Ssh2),
                multiplexing: NativeSshMultiplexing::None,
                ssh_option: BTreeMap::from([("compression".to_owned(), "yes".to_owned())]),
                default_prog: Some(vec!["zsh".to_owned(), "-l".to_owned()]),
                assume_shell: NativeShellAssumption::Posix,
            }]),
            tls_clients: Some(vec![NativeTlsClientDomain {
                name: "ops-tls".to_owned(),
                bootstrap_via_ssh: Some("ops@bastion.example.com:22".to_owned()),
                remote_address: "ops.example.com:8443".to_owned(),
                pem_private_key: Some("/home/ops/client.key".to_owned()),
                pem_cert: Some("/home/ops/client.crt".to_owned()),
                pem_ca: Some("/home/ops/ca.pem".to_owned()),
                pem_root_certs: vec![
                    "/etc/ssl/certs".to_owned(),
                    "/opt/wezterm/ca.pem".to_owned(),
                ],
                accept_invalid_hostnames: true,
                expected_cn: Some("ops.internal".to_owned()),
                connect_automatically: true,
                read_timeout_ms: 45_000,
                write_timeout_ms: 30_000,
                local_echo_threshold_ms: Some(12),
                remote_wezterm_path: Some("/opt/wezterm/wezterm".to_owned()),
                overlay_lag_indicator: true,
            }]),
            serial_ports: Some(vec![NativeSerialDomain {
                name: "ops-console".to_owned(),
                port: Some("/dev/ttyUSB0".to_owned()),
                baud: Some(115_200),
            }]),
            ..NativeConfigSnapshot::default()
        });

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::domains(),
                title: Some("Pick Domain".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));
        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Domain: Select an item and press Enter=launch Esc=cancel /=filter [1 / 8] Spawn In Domain: local"
        );

        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 8);
        assert_eq!(entries[0].label(), "Spawn In Domain: local");
        assert_eq!(entries[1].label(), "Spawn In Domain: remote-default");
        assert_eq!(entries[2].label(), "Spawn In Domain: ops");
        assert_eq!(entries[3].label(), "Spawn In Domain: wsl-ubuntu");
        assert_eq!(entries[4].label(), "Spawn In Domain: ops-unix");
        assert_eq!(entries[5].label(), "Spawn In Domain: ops-ssh");
        assert_eq!(entries[6].label(), "Spawn In Domain: ops-tls");
        assert_eq!(entries[7].label(), "Spawn In Domain: ops-console");

        app.command_palette_set_query("wsl-ubuntu".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Spawn In Domain: wsl-ubuntu");

        assert!(!app.command_palette_execute_entry(entries[0].clone()));
        assert!(app.command_palette.is_some());
    }

    #[test]
    fn window_app_dispatches_native_show_launcher_args_domains_payload_deduplicates_domain_names() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            default_domain: Some("Remote-Default".to_owned()),
            exec_domains: Some(vec![NativeExecDomain {
                name: "remote-default".to_owned(),
                fixup_command: "wezterm cli spawn".to_owned(),
                label: None,
            }]),
            wsl_domains: Some(vec![NativeWslDomain {
                name: "REMOTE-DEFAULT".to_owned(),
                distribution: Some("Ubuntu".to_owned()),
                username: Some("ops".to_owned()),
                default_cwd: Some("~".to_owned()),
                default_prog: None,
            }]),
            unix_domains: Some(vec![NativeUnixDomain {
                name: "reMoTe-DEFAULT".to_owned(),
                socket_path: Some("/tmp/ops.sock".to_owned()),
                connect_automatically: true,
                no_serve_automatically: true,
                serve_command: None,
                proxy_command: None,
                skip_permissions_check: true,
                read_timeout_ms: 45_000,
                write_timeout_ms: 30_000,
                local_echo_threshold_ms: Some(12),
                overlay_lag_indicator: true,
            }]),
            ssh_domains: Some(vec![NativeSshDomain {
                name: "rEmOtE-default".to_owned(),
                remote_address: "ops.example.com:2222".to_owned(),
                no_agent_auth: true,
                username: Some("ops".to_owned()),
                connect_automatically: true,
                timeout_ms: 45_000,
                local_echo_threshold_ms: Some(12),
                overlay_lag_indicator: true,
                remote_wezterm_path: Some("/opt/wezterm/wezterm".to_owned()),
                override_proxy_command: Some("wezterm cli proxy --stdio".to_owned()),
                ssh_backend: Some(NativeSshBackend::Ssh2),
                multiplexing: NativeSshMultiplexing::None,
                ssh_option: BTreeMap::from([("compression".to_owned(), "yes".to_owned())]),
                default_prog: Some(vec!["zsh".to_owned(), "-l".to_owned()]),
                assume_shell: NativeShellAssumption::Posix,
            }]),
            tls_clients: Some(vec![NativeTlsClientDomain {
                name: "REMOTE-default".to_owned(),
                bootstrap_via_ssh: Some("ops@bastion.example.com:22".to_owned()),
                remote_address: "ops.example.com:8443".to_owned(),
                pem_private_key: Some("/home/ops/client.key".to_owned()),
                pem_cert: Some("/home/ops/client.crt".to_owned()),
                pem_ca: Some("/home/ops/ca.pem".to_owned()),
                pem_root_certs: vec![],
                accept_invalid_hostnames: true,
                expected_cn: Some("ops.internal".to_owned()),
                connect_automatically: true,
                read_timeout_ms: 45_000,
                write_timeout_ms: 30_000,
                local_echo_threshold_ms: Some(12),
                remote_wezterm_path: Some("/opt/wezterm/wezterm".to_owned()),
                overlay_lag_indicator: true,
            }]),
            serial_ports: Some(vec![NativeSerialDomain {
                name: "ops-console".to_owned(),
                port: Some("/dev/ttyUSB0".to_owned()),
                baud: Some(115_200),
            }]),
            ..NativeConfigSnapshot::default()
        });

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::domains(),
                title: Some("Pick Domain".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));
        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Domain: Select an item and press Enter=launch Esc=cancel /=filter [1 / 3] Spawn In Domain: local"
        );

        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].label(), "Spawn In Domain: local");
        assert_eq!(entries[1].label(), "Spawn In Domain: Remote-Default");
        assert_eq!(entries[2].label(), "Spawn In Domain: ops-console");
    }

    #[test]
    fn window_app_dispatches_native_show_launcher_args_domains_payload_with_local_default_domain_case_deduplicates()
     {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            default_domain: Some("LOCAL".to_owned()),
            exec_domains: Some(vec![NativeExecDomain {
                name: "LoCaL".to_owned(),
                fixup_command: "wezterm cli spawn".to_owned(),
                label: None,
            }]),
            unix_domains: Some(vec![NativeUnixDomain {
                name: "ops-unix".to_owned(),
                socket_path: Some("/tmp/ops.sock".to_owned()),
                connect_automatically: true,
                no_serve_automatically: true,
                serve_command: None,
                proxy_command: None,
                skip_permissions_check: true,
                read_timeout_ms: 45_000,
                write_timeout_ms: 30_000,
                local_echo_threshold_ms: Some(12),
                overlay_lag_indicator: true,
            }]),
            ..NativeConfigSnapshot::default()
        });

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::domains(),
                title: Some("Pick Domain".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));
        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Domain: Select an item and press Enter=launch Esc=cancel /=filter [1 / 2] Spawn In Domain: local"
        );

        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].label(), "Spawn In Domain: local");
        assert_eq!(entries[1].label(), "Spawn In Domain: ops-unix");

        app.command_palette_set_query("local".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Spawn In Domain: local");
        assert!(app.command_palette_execute_entry(entries[0].clone()));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_show_launcher_args_domains_payload_with_explicit_local_domain_deduplicated()
     {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            default_domain: Some("remote-default".to_owned()),
            exec_domains: Some(vec![
                NativeExecDomain {
                    name: "Local".to_owned(),
                    fixup_command: "wezterm cli spawn".to_owned(),
                    label: None,
                },
                NativeExecDomain {
                    name: "ops".to_owned(),
                    fixup_command: "wezterm cli spawn".to_owned(),
                    label: None,
                },
            ]),
            unix_domains: Some(vec![NativeUnixDomain {
                name: "local".to_owned(),
                socket_path: Some("/tmp/ops.sock".to_owned()),
                connect_automatically: true,
                no_serve_automatically: true,
                serve_command: None,
                proxy_command: None,
                skip_permissions_check: true,
                read_timeout_ms: 45_000,
                write_timeout_ms: 30_000,
                local_echo_threshold_ms: Some(12),
                overlay_lag_indicator: true,
            }]),
            ..NativeConfigSnapshot::default()
        });

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::domains(),
                title: Some("Pick Domain".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));
        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Domain: Select an item and press Enter=launch Esc=cancel /=filter [1 / 3] Spawn In Domain: local"
        );

        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].label(), "Spawn In Domain: local");
        assert_eq!(entries[1].label(), "Spawn In Domain: remote-default");
        assert_eq!(entries[2].label(), "Spawn In Domain: ops");

        app.command_palette_set_query("local".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Spawn In Domain: local");
        assert!(app.command_palette_execute_entry(entries[0].clone()));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_show_launcher_args_launch_menu_items_payload() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            launch_menu: Some(vec![NativeLaunchMenuItem {
                label: Some("System Monitor".to_owned()),
                command: NativeLaunchMenuCommand::Command(WindowSpawnCommandQuery {
                    label: None,
                    program: "top".to_owned(),
                    args: vec!["-H".to_owned()],
                    cwd: Some("/tmp/project".to_owned()),
                    environment: BTreeMap::from([("LAUNCH_MENU".to_owned(), "1".to_owned())]),
                    domain: None,
                    window_position: None,
                }),
            }]),
            ..NativeConfigSnapshot::default()
        });

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::launch_menu_items(),
                title: Some("Pick Launch".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));
        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Launch: Select an item and press Enter=launch Esc=cancel /=filter [1 / 1] System Monitor"
        );

        app.command_palette_set_query("monitor".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "System Monitor");
        assert!(app.command_palette_execute_entry(entries[0].clone()));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-H"]);
        assert_eq!(launch.cwd(), Some("/tmp/project"));
        assert_eq!(
            launch.environment().get("LAUNCH_MENU"),
            Some(&"1".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_show_launcher_default_payload() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            launch_menu: Some(vec![NativeLaunchMenuItem {
                label: Some("System Monitor".to_owned()),
                command: NativeLaunchMenuCommand::Command(WindowSpawnCommandQuery {
                    label: None,
                    program: "top".to_owned(),
                    args: Vec::new(),
                    cwd: None,
                    environment: BTreeMap::new(),
                    domain: None,
                    window_position: None,
                }),
            }]),
            ..NativeConfigSnapshot::default()
        });

        assert!(app.command_palette_execute(WindowCommand::ShowLauncher));
        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            "Launcher: Select an item and press Enter=launch Esc=cancel /=filter [1 / 3] Spawn In Domain: local"
        );

        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].label(), "Spawn In Domain: local");
        assert_eq!(entries[1].label(), "Spawn In Domain: unix");
        assert_eq!(entries[2].label(), "System Monitor");
    }

    #[test]
    fn parses_wezterm_show_launcher_args_pipe_separated_flags() {
        let args = show_launcher_args_from_query(
            "show launcher FUZZY|TABS|DOMAINS|KEY_ASSIGNMENTS|LAUNCH_MENU_ITEMS|WORKSPACES|COMMANDS title Pick Target",
        )
        .expect("expected ShowLauncherArgs query");

        assert!(args.flags.fuzzy);
        assert!(args.flags.tabs);
        assert!(args.flags.domains);
        assert!(args.flags.key_assignments);
        assert!(args.flags.launch_menu_items);
        assert!(args.flags.workspaces);
        assert!(args.flags.commands);
        assert_eq!(args.title.as_deref(), Some("Pick Target"));
        let args = show_launcher_args_from_query("show launcher TABS alphabet asdf title Pick Tab")
            .expect("expected ShowLauncherArgs alphabet query");
        assert_eq!(args.alphabet.as_deref(), Some("asdf"));
        assert_eq!(args.title.as_deref(), Some("Pick Tab"));
        let args = show_launcher_args_from_query(
            "show launcher TABS help_text Choose a tab fuzzy_help_text Filter tabs: title Pick Tab",
        )
        .expect("expected ShowLauncherArgs help text query");
        assert_eq!(args.help_text.as_deref(), Some("Choose a tab"));
        assert_eq!(args.fuzzy_help_text.as_deref(), Some("Filter tabs:"));
        assert_eq!(args.title.as_deref(), Some("Pick Tab"));
        assert!(show_launcher_args_from_query("show launcher BOGUS").is_none());
        assert!(show_launcher_args_from_query("show launcher TABS|").is_none());
    }

    #[test]
    fn parses_show_launcher_args_query_with_normalized_flag_aliases() {
        let args = show_launcher_args_from_query(
            "showlauncher fuzzy|tabs|key-assignments|launchmenuitems title Pick Target",
        )
        .expect("expected ShowLauncherArgs query");

        assert!(args.flags.fuzzy);
        assert!(args.flags.tabs);
        assert!(args.flags.key_assignments);
        assert!(args.flags.launch_menu_items);
        assert_eq!(args.title.as_deref(), Some("Pick Target"));

        let args = show_launcher_args_from_query("showlauncherargs tabs|workspaces title Jump")
            .expect("expected action-name ShowLauncherArgs query");
        assert!(args.flags.tabs);
        assert!(args.flags.workspaces);
        assert_eq!(args.title.as_deref(), Some("Jump"));
    }

    #[test]
    fn rejects_show_launcher_args_query_with_unknown_trailing_field() {
        assert!(show_launcher_args_from_query("show launcher TABS unknown").is_none());
        assert!(
            show_launcher_args_from_query("show launcher TABS alphabet asdf unknown").is_none()
        );
    }

    #[test]
    fn parses_show_launcher_args_query_with_field_words_in_help_text() {
        let args = show_launcher_args_from_query(
            "show launcher TABS help_text Press title to rename title Pick Tab",
        )
        .expect("expected ShowLauncherArgs query");

        assert_eq!(args.help_text.as_deref(), Some("Press title to rename"));
        assert_eq!(args.title.as_deref(), Some("Pick Tab"));
        assert_eq!(args.flags, WindowShowLauncherFlags::tabs());
    }

    #[test]
    fn parses_show_launcher_args_query_with_quoted_text_fields() {
        let args = show_launcher_args_from_query(
            "show launcher TABS help_text \"Press title to rename\" fuzzy_help_text \"Filter tabs:\" title \"Pick Tab\"",
        )
        .expect("expected ShowLauncherArgs query");

        assert_eq!(args.help_text.as_deref(), Some("Press title to rename"));
        assert_eq!(args.fuzzy_help_text.as_deref(), Some("Filter tabs:"));
        assert_eq!(args.title.as_deref(), Some("Pick Tab"));
        assert_eq!(args.flags, WindowShowLauncherFlags::tabs());
    }

    #[test]
    fn parses_show_launcher_args_query_with_spaced_field_aliases() {
        let args = show_launcher_args_from_query(
            "showlauncher TABS help text Choose a tab fuzzy help text Filter tabs: title Pick Tab",
        )
        .expect("expected ShowLauncherArgs query");

        assert_eq!(args.help_text.as_deref(), Some("Choose a tab"));
        assert_eq!(args.fuzzy_help_text.as_deref(), Some("Filter tabs:"));
        assert_eq!(args.title.as_deref(), Some("Pick Tab"));
        assert_eq!(args.flags, WindowShowLauncherFlags::tabs());
    }

    #[test]
    fn parses_show_launcher_args_query_with_quoted_alphabet() {
        let args =
            show_launcher_args_from_query("show launcher TABS alphabet \"ab\" title Pick Tab")
                .expect("expected ShowLauncherArgs query");

        assert_eq!(args.alphabet.as_deref(), Some("ab"));
        assert_eq!(args.title.as_deref(), Some("Pick Tab"));
        assert_eq!(args.flags, WindowShowLauncherFlags::tabs());
    }

    #[test]
    fn parses_show_launcher_args_query_with_equals_fields() {
        let args = show_launcher_args_from_query(
            "showlauncherargs TABS alphabet=ab help-text=Choose title=Pick fuzzy-help-text=Filter",
        )
        .expect("expected ShowLauncherArgs equals-field query");

        assert_eq!(args.alphabet.as_deref(), Some("ab"));
        assert_eq!(args.help_text.as_deref(), Some("Choose"));
        assert_eq!(args.title.as_deref(), Some("Pick"));
        assert_eq!(args.fuzzy_help_text.as_deref(), Some("Filter"));
        assert_eq!(args.flags, WindowShowLauncherFlags::tabs());
    }

    #[test]
    fn parses_show_launcher_args_equals_query() {
        let args = show_launcher_args_from_query("show launcher args=TABS alphabet=ab title=Pick")
            .expect("expected ShowLauncherArgs equals query");

        assert_eq!(args.alphabet.as_deref(), Some("ab"));
        assert_eq!(args.title.as_deref(), Some("Pick"));
        assert_eq!(args.flags, WindowShowLauncherFlags::tabs());
    }

    #[test]
    fn parses_show_launcher_args_compact_equals_query() {
        let args = show_launcher_args_from_query("showlauncherargs=TABS alphabet=ab title=Pick")
            .expect("expected compact ShowLauncherArgs equals query");

        assert_eq!(args.alphabet.as_deref(), Some("ab"));
        assert_eq!(args.title.as_deref(), Some("Pick"));
        assert_eq!(args.flags, WindowShowLauncherFlags::tabs());
    }

    #[test]
    fn window_app_dispatches_palette_show_launcher_args_equals_query_before_quick_select() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("showlauncherargs TABS alphabet=ab title=Pick".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ShowLauncherArgs(WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::tabs(),
                title: Some("Pick".to_owned()),
                alphabet: Some("ab".to_owned()),
                help_text: None,
                fuzzy_help_text: None,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_show_launcher_args_wezterm_action_parenthesized_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.ShowLauncherArgs({ flags = \"TABS|WORKSPACES\", title = \"Jump\", alphabet = \"ab\" })"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ShowLauncherArgs(WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags {
                    tabs: true,
                    workspaces: true,
                    ..WindowShowLauncherFlags::default()
                },
                title: Some("Jump".to_owned()),
                alphabet: Some("ab".to_owned()),
                help_text: None,
                fuzzy_help_text: None,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_show_launcher_args_wezterm_action_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.ShowLauncherArgs { flags = \"TABS|WORKSPACES\", title = \"Jump\", alphabet = \"ab\" }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ShowLauncherArgs(WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags {
                    tabs: true,
                    workspaces: true,
                    ..WindowShowLauncherFlags::default()
                },
                title: Some("Jump".to_owned()),
                alphabet: Some("ab".to_owned()),
                help_text: None,
                fuzzy_help_text: None,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_show_launcher_args_wezterm_action_table_long_bracket_key_query()
     {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.ShowLauncherArgs { [[=[flags]=]] = [[TABS|WORKSPACES]], [[=[title]=]] = [[Jump]], [[=[alphabet]=]] = [[ab]], [[=[help_text]=]] = [[Pick]], [[=[fuzzy_help_text]=]] = [[Filter]] }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ShowLauncherArgs(WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags {
                    tabs: true,
                    workspaces: true,
                    ..WindowShowLauncherFlags::default()
                },
                title: Some("Jump".to_owned()),
                alphabet: Some("ab".to_owned()),
                help_text: Some("Pick".to_owned()),
                fuzzy_help_text: Some("Filter".to_owned()),
            })]
        );
    }

    #[test]
    fn parses_show_launcher_args_query_with_flags_equals_field() {
        let args = show_launcher_args_from_query(
            "showlauncherargs flags=tabs|workspaces alphabet=ab title=Jump",
        )
        .expect("expected ShowLauncherArgs flags field query");

        assert_eq!(
            args.flags,
            WindowShowLauncherFlags {
                tabs: true,
                workspaces: true,
                ..WindowShowLauncherFlags::default()
            }
        );
        assert_eq!(args.alphabet.as_deref(), Some("ab"));
        assert_eq!(args.title.as_deref(), Some("Jump"));
    }

    #[test]
    fn parses_show_launcher_args_query_with_flags_after_text_fields() {
        let args = show_launcher_args_from_query(
            "showlauncherargs title Jump flags tabs|workspaces alphabet ab",
        )
        .expect("expected ShowLauncherArgs flags field query");

        assert_eq!(
            args.flags,
            WindowShowLauncherFlags {
                tabs: true,
                workspaces: true,
                ..WindowShowLauncherFlags::default()
            }
        );
        assert_eq!(args.title.as_deref(), Some("Jump"));
        assert_eq!(args.alphabet.as_deref(), Some("ab"));
    }

    #[test]
    fn parses_show_launcher_args_query_with_mixed_case_equals_fields() {
        let args = show_launcher_args_from_query(
            "showlauncherargs Flags=tabs|workspaces Alphabet=ab Title=Jump Help_Text=Choose Fuzzy_Help_Text=Filter",
        )
        .expect("expected ShowLauncherArgs mixed-case field query");

        assert_eq!(
            args.flags,
            WindowShowLauncherFlags {
                tabs: true,
                workspaces: true,
                ..WindowShowLauncherFlags::default()
            }
        );
        assert_eq!(args.alphabet.as_deref(), Some("ab"));
        assert_eq!(args.title.as_deref(), Some("Jump"));
        assert_eq!(args.help_text.as_deref(), Some("Choose"));
        assert_eq!(args.fuzzy_help_text.as_deref(), Some("Filter"));
    }

    #[test]
    fn window_app_dispatches_command_palette_show_launcher_args_query() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Shell\x07").unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.handle_pty_output(b"\x1b]2;Logs\x07").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("show launcher FUZZY|TABS title Pick Tab".to_owned());
        let commands = app.command_palette_filtered_commands();

        assert_eq!(commands.len(), 1);
        assert!(app.command_palette_execute(commands[0].clone()));
        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Tab: Fuzzy matching: [1 / 2] Activate Tab: Shell"
        );
    }

    #[test]
    fn window_app_dispatches_command_palette_show_launcher_action_name_query() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Shell\x07").unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.handle_pty_output(b"\x1b]2;Logs\x07").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("showlauncher FUZZY|TABS title Pick Tab".to_owned());
        let commands = app.command_palette_filtered_commands();

        assert_eq!(commands.len(), 1);
        assert!(app.command_palette_execute(commands[0].clone()));
        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Tab: Fuzzy matching: [1 / 2] Activate Tab: Shell"
        );
    }

    #[test]
    fn window_app_dispatches_command_palette_show_launcher_args_help_text_query() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Shell\x07").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "show launcher TABS help_text Pick with Enter fuzzy_help_text Narrow: title Pick Tab"
                .to_owned(),
        );
        let commands = app.command_palette_filtered_commands();

        assert_eq!(commands.len(), 1);
        assert!(app.command_palette_execute(commands[0].clone()));
        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Tab: Pick with Enter [1 / 1] Activate Tab: Shell"
        );

        app.handle_command_palette_logical_key(
            &Key::Character("/".into()),
            ModifiersState::empty(),
        );
        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Tab: Narrow: [1 / 1] Activate Tab: Shell"
        );
    }

    #[test]
    fn window_app_show_launcher_args_alphabet_key_executes_matching_entry() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Shell\x07").unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.handle_pty_output(b"\x1b]2;Logs\x07").unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::tabs(),
                title: Some("Pick Tab".to_owned()),
                alphabet: Some("ab".to_owned()),
                help_text: None,
                fuzzy_help_text: None,
            },
        )));

        assert!(app.handle_command_palette_logical_key(
            &Key::Character("b".into()),
            ModifiersState::empty()
        ));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_show_launcher_args_alphabet_accepts_two_key_shortcut() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Shell\x07").unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.handle_pty_output(b"\x1b]2;Logs\x07").unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.handle_pty_output(b"\x1b]2;Deploy\x07").unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::tabs(),
                title: Some("Pick Tab".to_owned()),
                alphabet: Some("ab".to_owned()),
                help_text: None,
                fuzzy_help_text: None,
            },
        )));

        assert!(app.handle_command_palette_logical_key(
            &Key::Character("b".into()),
            ModifiersState::empty()
        ));
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert!(app.command_palette.is_some());

        assert!(app.handle_command_palette_logical_key(
            &Key::Character("a".into()),
            ModifiersState::empty()
        ));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_show_launcher_args_uses_configured_launcher_alphabet_by_default() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            launcher_alphabet: Some("ab".to_owned()),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b]2;Shell\x07").unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.handle_pty_output(b"\x1b]2;Logs\x07").unwrap();

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::tabs(),
                title: Some("Pick Tab".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));

        assert!(app.handle_command_palette_logical_key(
            &Key::Character("b".into()),
            ModifiersState::empty()
        ));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_show_launcher_args_default_mode_j_moves_selection() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Shell\x07").unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.handle_pty_output(b"\x1b]2;Logs\x07").unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::tabs(),
                title: Some("Pick Tab".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));

        app.handle_command_palette_logical_key(
            &Key::Character("j".into()),
            ModifiersState::empty(),
        );
        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should remain open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Tab: Select an item and press Enter=launch Esc=cancel /=filter [2 / 2] Activate Tab: Logs"
        );

        assert!(app.handle_command_palette_logical_key(
            &Key::Named(NamedKey::Enter),
            ModifiersState::empty()
        ));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_show_launcher_args_default_mode_k_moves_selection_up() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Shell\x07").unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.handle_pty_output(b"\x1b]2;Logs\x07").unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::tabs(),
                title: Some("Pick Tab".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));

        app.handle_command_palette_logical_key(
            &Key::Character("k".into()),
            ModifiersState::empty(),
        );
        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should remain open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Tab: Select an item and press Enter=launch Esc=cancel /=filter [2 / 2] Activate Tab: Logs"
        );
    }

    #[test]
    fn window_app_show_launcher_args_slash_enters_fuzzy_filter_mode() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Shell\x07").unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.handle_pty_output(b"\x1b]2;Logs\x07").unwrap();

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::tabs(),
                title: Some("Pick Tab".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));

        app.handle_command_palette_logical_key(
            &Key::Character("/".into()),
            ModifiersState::empty(),
        );
        app.handle_command_palette_logical_key(
            &Key::Character("g".into()),
            ModifiersState::empty(),
        );

        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should remain open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Tab: Fuzzy matching: \"g\" [1 / 1] Activate Tab: Logs"
        );
    }

    #[test]
    fn window_app_show_launcher_args_displays_default_mode_help_text() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Shell\x07").unwrap();

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::tabs(),
                title: Some("Pick Tab".to_owned()),
                alphabet: None,
                help_text: Some("Enter selects, slash filters".to_owned()),
                fuzzy_help_text: None,
            },
        )));

        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Tab: Enter selects, slash filters [1 / 1] Activate Tab: Shell"
        );
    }

    #[test]
    fn window_app_show_launcher_args_displays_default_help_text_when_omitted() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Shell\x07").unwrap();

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::tabs(),
                title: Some("Pick Tab".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));

        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Tab: Select an item and press Enter=launch Esc=cancel /=filter [1 / 1] Activate Tab: Shell"
        );
    }

    #[test]
    fn window_app_show_launcher_args_displays_fuzzy_help_text_after_slash() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Shell\x07").unwrap();

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::tabs(),
                title: Some("Pick Tab".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: Some("Filter: ".to_owned()),
            },
        )));

        app.handle_command_palette_logical_key(
            &Key::Character("/".into()),
            ModifiersState::empty(),
        );

        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Tab: Filter: [1 / 1] Activate Tab: Shell"
        );
    }

    #[test]
    fn window_app_show_launcher_args_displays_default_fuzzy_help_text_when_omitted() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Shell\x07").unwrap();

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::tabs(),
                title: Some("Pick Tab".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));

        app.handle_command_palette_logical_key(
            &Key::Character("/".into()),
            ModifiersState::empty(),
        );

        let palette = app
            .command_palette
            .as_ref()
            .expect("launcher should be open");
        assert_eq!(
            app.command_palette_status(palette),
            "Pick Tab: Fuzzy matching: [1 / 1] Activate Tab: Shell"
        );
    }

    #[test]
    fn native_effective_config_reports_default_launcher_alphabet() {
        let app = NativeWindowApp::new(None);

        assert_eq!(
            app.native_effective_config().launcher_alphabet,
            DEFAULT_LAUNCHER_ALPHABET
        );
    }

    #[test]
    fn ctrl_shift_k_is_not_default_close_workspace_shortcut() {
        let app = NativeWindowApp::new(None);

        assert!(
            app.app_shell_action_for_key(
                &Key::Character("k".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT,
            )
            .is_none()
        );
    }

    #[test]
    fn window_app_unmodified_page_key_stays_available_for_pty() {
        let mut app = NativeWindowApp::new(None);

        assert!(
            !app.handle_scrollback_shortcut(&Key::Named(NamedKey::PageUp), ModifiersState::empty())
        );
    }

    #[test]
    fn window_app_tracks_runtime_title_from_pty_output() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();

        assert_eq!(app.window_title, "PowerShell");
    }

    #[test]
    fn window_app_writes_osc52_clipboard_text_from_pty_output() {
        let writes = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&writes);
        let mut app = NativeWindowApp::new(None);
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded.lock().unwrap().push(text.to_owned());
            true
        });

        app.handle_pty_output(b"\x1b]52;c;Y29weQ==\x07").unwrap();

        assert_eq!(writes.lock().unwrap().as_slice(), ["copy"]);
    }

    #[test]
    fn window_app_writes_iterm_copy_clipboard_text_from_active_pane_output() {
        let writes = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&writes);
        let mut app = NativeWindowApp::new(None);
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded.lock().unwrap().push(text.to_owned());
            true
        });

        app.handle_pty_output(b"\x1b]1337;Copy=;Y29weQ==\x07")
            .unwrap();

        assert_eq!(writes.lock().unwrap().as_slice(), ["copy"]);
    }

    #[test]
    fn window_app_writes_c1_iterm_copy_clipboard_text_from_active_pane_output() {
        let writes = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&writes);
        let mut app = NativeWindowApp::new(None);
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded.lock().unwrap().push(text.to_owned());
            true
        });

        app.handle_pty_output(b"\x9d1337;Copy=;Y29weQ==\x9c")
            .unwrap();

        assert_eq!(writes.lock().unwrap().as_slice(), ["copy"]);
    }

    #[test]
    fn window_app_writes_iterm_copy_clipboard_text_from_inactive_pane_output() {
        let writes = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&writes);
        let mut app = NativeWindowApp::new(None);
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded.lock().unwrap().push(text.to_owned());
            true
        });
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        app.handle_pane_pty_output(rterm_core::PaneId::new(1), b"\x1b]1337;Copy=;Y29weQ==\x07")
            .unwrap();

        assert_eq!(writes.lock().unwrap().as_slice(), ["copy"]);
    }

    #[test]
    fn window_app_dispatches_wezterm_notifications_from_active_and_inactive_panes() {
        let notifications = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&notifications);
        let mut app = NativeWindowApp::new(None);
        app.notification_handler = Box::new(move |notification| {
            recorded.lock().unwrap().push(notification.clone());
            true
        });
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        app.handle_pty_output(b"\x1b]9;active done\x07").unwrap();
        app.handle_pane_pty_output(
            rterm_core::PaneId::new(1),
            b"\x9d777;notify;Inactive;done\x9c",
        )
        .unwrap();

        assert_eq!(
            notifications.lock().unwrap().as_slice(),
            [
                TerminalNotification {
                    title: None,
                    body: "active done".to_owned(),
                },
                TerminalNotification {
                    title: Some("Inactive".to_owned()),
                    body: "done".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn window_app_honors_never_show_notification_handling() {
        let notifications = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&notifications);
        let mut app = NativeWindowApp::new(None);
        app.notification_handler = Box::new(move |notification| {
            recorded.lock().unwrap().push(notification.clone());
            true
        });
        app.set_config_overrides(native_config_snapshot! {
            notification_handling: Some(NativeNotificationHandling::NeverShow),
            ..NativeConfigSnapshot::default()
        });

        app.handle_pty_output(b"\x1b]9;hidden\x07").unwrap();

        assert!(notifications.lock().unwrap().is_empty());
        assert!(
            !app.effective_window_title()
                .contains("Notification: hidden"),
            "window title was {:?}",
            app.effective_window_title()
        );
    }

    #[test]
    fn window_app_suppresses_focused_pane_notifications_only_for_active_pane() {
        let notifications = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&notifications);
        let mut app = NativeWindowApp::new(None);
        app.notification_handler = Box::new(move |notification| {
            recorded.lock().unwrap().push(notification.clone());
            true
        });
        app.set_config_overrides(native_config_snapshot! {
            notification_handling: Some(NativeNotificationHandling::SuppressFromFocusedPane),
            ..NativeConfigSnapshot::default()
        });
        assert!(app.handle_focus_changed(true).unwrap());
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        app.handle_pty_output(b"\x1b]9;active hidden\x07").unwrap();
        app.handle_pane_pty_output(rterm_core::PaneId::new(1), b"\x1b]9;inactive shown\x07")
            .unwrap();

        assert_eq!(
            notifications.lock().unwrap().as_slice(),
            [TerminalNotification {
                title: None,
                body: "inactive shown".to_owned(),
            }]
        );
    }

    #[test]
    fn window_app_suppresses_focused_tab_notifications_only_for_active_tab() {
        let notifications = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&notifications);
        let mut app = NativeWindowApp::new(None);
        app.notification_handler = Box::new(move |notification| {
            recorded.lock().unwrap().push(notification.clone());
            true
        });
        app.set_config_overrides(native_config_snapshot! {
            notification_handling: Some(NativeNotificationHandling::SuppressFromFocusedTab),
            ..NativeConfigSnapshot::default()
        });
        assert!(app.handle_focus_changed(true).unwrap());
        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.active_pane_id(),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));

        app.handle_pane_pty_output(rterm_core::PaneId::new(1), b"\x1b]9;same tab hidden\x07")
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        app.handle_pane_pty_output(rterm_core::PaneId::new(1), b"\x1b]9;inactive tab shown\x07")
            .unwrap();

        assert_eq!(
            notifications.lock().unwrap().as_slice(),
            [TerminalNotification {
                title: None,
                body: "inactive tab shown".to_owned(),
            }]
        );
    }

    #[test]
    fn window_app_suppresses_focused_window_notifications_only_while_focused() {
        let notifications = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&notifications);
        let mut app = NativeWindowApp::new(None);
        app.notification_handler = Box::new(move |notification| {
            recorded.lock().unwrap().push(notification.clone());
            true
        });
        app.set_config_overrides(native_config_snapshot! {
            notification_handling: Some(NativeNotificationHandling::SuppressFromFocusedWindow),
            ..NativeConfigSnapshot::default()
        });

        assert!(app.handle_focus_changed(true).unwrap());
        app.handle_pty_output(b"\x1b]9;focused hidden\x07").unwrap();
        assert!(app.handle_focus_changed(false).unwrap());
        app.handle_pty_output(b"\x1b]9;unfocused shown\x07")
            .unwrap();

        assert_eq!(
            notifications.lock().unwrap().as_slice(),
            [TerminalNotification {
                title: None,
                body: "unfocused shown".to_owned(),
            }]
        );
    }

    #[test]
    fn window_title_includes_latest_wezterm_notification_status() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]9;active done\x07").unwrap();
        assert!(
            app.effective_window_title()
                .contains("Notification: active done"),
            "window title was {:?}",
            app.effective_window_title()
        );

        app.handle_pty_output(b"\x1b]777;notify;Build;failed\x07")
            .unwrap();
        assert!(
            app.effective_window_title()
                .contains("Notification: Build - failed"),
            "window title was {:?}",
            app.effective_window_title()
        );
    }

    #[test]
    fn window_title_omits_empty_wezterm_notification_status_segments() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]777;notify;Build;\x07")
            .unwrap();

        assert!(
            app.effective_window_title().contains("Notification: Build"),
            "window title was {:?}",
            app.effective_window_title()
        );
        assert!(
            !app.effective_window_title().contains("Build - "),
            "window title was {:?}",
            app.effective_window_title()
        );
    }

    #[test]
    fn window_app_dispatches_user_var_changed_from_active_and_inactive_panes() {
        let changes = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&changes);
        let mut app = NativeWindowApp::new(None);
        app.user_var_change_handler = Box::new(move |change| {
            recorded.lock().unwrap().push(change.clone());
            true
        });
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        let active_pane = app.app_shell.active_pane_id();

        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_PROG=YmFy\x07")
            .unwrap();
        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_PROG=YmFy\x07")
            .unwrap();
        app.handle_pane_pty_output(
            rterm_core::PaneId::new(1),
            b"\x9d1337;SetUserVar=WEZTERM_HOST=YmF6\x9c",
        )
        .unwrap();

        assert_eq!(
            changes.lock().unwrap().as_slice(),
            [
                NativeWindowUserVarChange {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                    name: "WEZTERM_PROG".to_owned(),
                    value: "bar".to_owned(),
                },
                NativeWindowUserVarChange {
                    window_id: rterm_core::WindowId::new(1),
                    pane: rterm_core::PaneId::new(1),
                    name: "WEZTERM_HOST".to_owned(),
                    value: "baz".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_user_var_changed_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('user-var-changed', function(window, pane, name, value)
              window:set_right_status('var=' .. name .. ':' .. value)
            end)
            "#,
        )
        .expect("expected static WezTerm user-var-changed event status setter");
        app.set_config_overrides(overrides);

        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_PROG=cHNo\x07")
            .unwrap();

        assert_eq!(app.right_status, "var=WEZTERM_PROG:psh");
    }

    #[test]
    fn window_app_parses_static_wezterm_user_var_changed_window_pane_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('user-var-changed', function(window, pane, name, value)
              window:set_right_status(
                'win=' .. window:window_id()
                  .. ' pane=' .. pane:pane_id()
                  .. ' ' .. name .. '=' .. value
              )
            end)
            "#,
        )
        .expect("expected static WezTerm user-var-changed window pane status setter");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        app.handle_pane_pty_output(
            rterm_core::PaneId::new(1),
            b"\x1b]1337;SetUserVar=WEZTERM_HOST=YmF6\x07",
        )
        .unwrap();

        assert_eq!(app.right_status, "win=1 pane=1 WEZTERM_HOST=baz");
    }

    #[test]
    fn window_app_parses_static_wezterm_user_var_changed_local_window_pane_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('user-var-changed', function(window, pane, name, value)
              local wid = window:window_id()
              local pid = pane:pane_id()
              window:set_right_status(
                'win=' .. tostring(wid)
                  .. ' pane=' .. tostring(pid)
                  .. ' ' .. name .. '=' .. value
              )
            end)
            "#,
        )
        .expect("expected static WezTerm user-var-changed local window pane status setter");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        app.handle_pane_pty_output(
            rterm_core::PaneId::new(1),
            b"\x1b]1337;SetUserVar=WEZTERM_HOST=YmF6\x07",
        )
        .unwrap();

        assert_eq!(app.right_status, "win=1 pane=1 WEZTERM_HOST=baz");
    }

    #[test]
    fn window_app_parses_static_wezterm_user_var_changed_local_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('user-var-changed', function(window, pane, name, value)
              local status = 'var=' .. name .. ':' .. tostring(value)
              window:set_right_status(status)
            end)
            "#,
        )
        .expect("expected static WezTerm user-var-changed local status setter");
        app.set_config_overrides(overrides);

        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_PROG=cHNo\x07")
            .unwrap();

        assert_eq!(app.right_status, "var=WEZTERM_PROG:psh");
    }

    #[test]
    fn window_app_parses_static_wezterm_user_var_changed_local_event_param_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('user-var-changed', function(window, pane, name, value)
              local changed_name = name
              local changed_value = value
              window:set_right_status('var=' .. changed_name .. ':' .. tostring(changed_value))
            end)
            "#,
        )
        .expect("expected static WezTerm user-var-changed local event param status setter");
        app.set_config_overrides(overrides);

        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_PROG=cHNo\x07")
            .unwrap();

        assert_eq!(app.right_status, "var=WEZTERM_PROG:psh");
    }

    #[test]
    fn window_app_parses_static_wezterm_user_var_changed_local_event_param_fallback_status_setter()
    {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('user-var-changed', function(window, pane, name, value)
              local changed_name = name or 'unknown'
              local changed_value = value or ''
              window:set_right_status('var=' .. changed_name .. ':' .. tostring(changed_value))
            end)
            "#,
        )
        .expect(
            "expected static WezTerm user-var-changed local event param fallback status setter",
        );
        app.set_config_overrides(overrides);

        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_PROG=cHNo\x07")
            .unwrap();

        assert_eq!(app.right_status, "var=WEZTERM_PROG:psh");
    }

    #[test]
    fn window_app_parses_user_var_changed_local_event_param_variable_fallback_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('user-var-changed', function(window, pane, name, value)
              local missing_name = 'unknown'
              local empty_value = ''
              local changed_name = name or missing_name
              local changed_value = value or empty_value
              window:set_right_status('var=' .. changed_name .. ':' .. tostring(changed_value))
            end)
            "#,
        )
        .expect(
            "expected static WezTerm user-var-changed local event param variable fallback status setter",
        );
        app.set_config_overrides(overrides);

        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_PROG=cHNo\x07")
            .unwrap();

        assert_eq!(app.right_status, "var=WEZTERM_PROG:psh");
    }

    #[test]
    fn window_app_parses_user_var_changed_local_event_param_top_level_fallback_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local missing_name = 'unknown'
            local empty_value = ''

            wezterm.on('user-var-changed', function(window, pane, name, value)
              local changed_name = name or missing_name
              local changed_value = value or empty_value
              window:set_right_status('var=' .. changed_name .. ':' .. tostring(changed_value))
            end)
            "#,
        )
        .expect(
            "expected static WezTerm user-var-changed local event param top-level fallback status setter",
        );
        app.set_config_overrides(overrides);

        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_PROG=cHNo\x07")
            .unwrap();

        assert_eq!(app.right_status, "var=WEZTERM_PROG:psh");
    }

    #[test]
    fn window_app_parses_user_var_changed_local_event_param_tostring_fallback_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local missing_name = 'unknown'
            local empty_value = ''

            wezterm.on('user-var-changed', function(window, pane, name, value)
              local changed_name = tostring(name or missing_name)
              local changed_value = tostring(value or empty_value)
              window:set_right_status('var=' .. changed_name .. ':' .. changed_value)
            end)
            "#,
        )
        .expect(
            "expected static WezTerm user-var-changed local event param tostring fallback status setter",
        );
        app.set_config_overrides(overrides);

        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_PROG=cHNo\x07")
            .unwrap();

        assert_eq!(app.right_status, "var=WEZTERM_PROG:psh");
    }

    #[test]
    fn window_app_parses_static_wezterm_user_var_changed_direct_event_param_fallback_status_setter()
    {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('user-var-changed', function(window, pane, name, value)
              window:set_right_status('var=' .. (name or 'unknown') .. ':' .. tostring(value or ''))
            end)
            "#,
        )
        .expect(
            "expected static WezTerm user-var-changed direct event param fallback status setter",
        );
        app.set_config_overrides(overrides);

        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_PROG=cHNo\x07")
            .unwrap();

        assert_eq!(app.right_status, "var=WEZTERM_PROG:psh");
    }

    #[test]
    fn window_app_parses_user_var_changed_event_param_variable_fallback_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('user-var-changed', function(window, pane, name, value)
              local missing_name = 'unknown'
              local empty_value = ''
              window:set_right_status('var=' .. (name or missing_name) .. ':' .. tostring(value or empty_value))
            end)
            "#,
        )
        .expect(
            "expected static WezTerm user-var-changed event param static variable fallback status setter",
        );
        app.set_config_overrides(overrides);

        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_PROG=cHNo\x07")
            .unwrap();

        assert_eq!(app.right_status, "var=WEZTERM_PROG:psh");
    }

    #[test]
    fn window_app_parses_static_wezterm_user_var_changed_static_string_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local prefix = 'var='

            wezterm.on('user-var-changed', function(window, pane, name, value)
              local sep = ':'
              window:set_right_status(prefix .. name .. sep .. value)
            end)
            "#,
        )
        .expect("expected static WezTerm user-var-changed static string status setter");
        app.set_config_overrides(overrides);

        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_PROG=cHNo\x07")
            .unwrap();

        assert_eq!(app.right_status, "var=WEZTERM_PROG:psh");
    }

