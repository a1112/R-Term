    #[test]
    fn window_app_command_palette_renders_augmented_entry_icon() {
        let mut app = NativeWindowApp::new(None);
        app.command_palette_augmenter = Box::new(|_| {
            vec![NativeCommandPaletteEntry {
                brief: "Zoom Native Pane".to_owned(),
                doc: None,
                icon: Some("md_rename_box".to_owned()),
                key_assignment: None,
                action: WindowCommand::TogglePaneZoom,
            }]
        });

        app.enter_command_palette_mode();
        app.command_palette_set_query("zoom native".to_owned());

        let snapshot = app.render_snapshot();
        let first_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_row.contains("\u{f0455} Zoom Native Pane"),
            "first command palette row was {first_row:?}"
        );
    }

    #[test]
    fn nerd_font_icon_for_name_accepts_wezterm_and_char_select_names() {
        assert_eq!(nerd_font_icon_for_name("fa_clock_o"), Some('\u{f017}'));
        assert_eq!(nerd_font_icon_for_name("fa_terminal"), Some('\u{f120}'));
        assert_eq!(nerd_font_icon_for_name("NF-FA-TERMINAL"), Some('\u{f120}'));
        assert_eq!(nerd_font_icon_for_name("cod_github"), Some('\u{ea84}'));
        assert_eq!(
            nerd_font_icon_for_name("md_magnify_plus"),
            Some('\u{f034b}')
        );
        assert_eq!(nerd_font_icon_for_name("md_rename_box"), Some('\u{f0455}'));
        assert_eq!(nerd_font_icon_for_name("missing_icon"), None);
    }

    #[test]
    fn window_app_command_palette_promotes_recently_executed_command_for_empty_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::ShowDebugOverlay));
        app.enter_command_palette_mode();

        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries[0].label(), "Show Debug Overlay");
    }

    #[test]
    fn window_app_command_palette_uses_recently_executed_command_as_fuzzy_tiebreaker() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::ToggleAlwaysOnBottom));
        app.enter_command_palette_mode();
        app.command_palette_set_query("toggle always o".to_owned());

        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries[0].label(), "Toggle Always On Bottom");
    }

    #[test]
    fn window_app_command_palette_persists_frecency_between_app_instances() {
        let path = temp_command_palette_frecency_path();
        let mut app = NativeWindowApp::new(None);
        app.set_command_palette_frecency_path_for_test(Some(path.clone()));

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::ShowDebugOverlay));

        let mut restored = NativeWindowApp::new(None);
        restored.set_command_palette_frecency_path_for_test(Some(path.clone()));
        restored.enter_command_palette_mode();

        let entries = restored.command_palette_filtered_entries();
        assert_eq!(entries[0].label(), "Show Debug Overlay");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn window_app_dispatches_palette_activate_last_tab_command() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ActivateLastTab);

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_multiple_commands_in_order() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::Multiple(vec![
            WindowCommand::NewTab,
            WindowCommand::NewTab,
        ])));

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 3);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_multiple_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query("multiple send string alpha;send string beta".to_owned());
        let expected = WindowCommand::Multiple(vec![
            WindowCommand::SendString("alpha".to_owned()),
            WindowCommand::SendString("beta".to_owned()),
        ]);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));
        assert_eq!(written.lock().unwrap().as_slice(), b"alphabeta");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_multiple_mixed_case_action_name_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("Multiple send string alpha ; send string beta".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::SendString("alpha".to_owned()),
                WindowCommand::SendString("beta".to_owned()),
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("multiple=send string alpha ; send string beta".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::SendString("alpha".to_owned()),
                WindowCommand::SendString("beta".to_owned()),
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_wezterm_action_table_call_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Multiple { wezterm.action.SendString \"alpha\", wezterm.action.SendString \"beta\" }"
                .to_owned(),
        );

        let expected = WindowCommand::Multiple(vec![
            WindowCommand::SendString("alpha".to_owned()),
            WindowCommand::SendString("beta".to_owned()),
        ]);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));
        assert_eq!(written.lock().unwrap().as_slice(), b"alphabeta");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_multiple_wezterm_action_table_indexed_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Multiple { [2] = wezterm.action.SendString \"beta\", [1] = wezterm.action.SendString \"alpha\" }"
                .to_owned(),
        );

        let expected = WindowCommand::Multiple(vec![
            WindowCommand::SendString("alpha".to_owned()),
            WindowCommand::SendString("beta".to_owned()),
        ]);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));
        assert_eq!(written.lock().unwrap().as_slice(), b"alphabeta");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_multiple_wezterm_action_send_string_function_call_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Multiple { wezterm.action.SendString(\"alpha\"), wezterm.action.SendString('beta') }"
                .to_owned(),
        );

        let expected = WindowCommand::Multiple(vec![
            WindowCommand::SendString("alpha".to_owned()),
            WindowCommand::SendString("beta".to_owned()),
        ]);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));
        assert_eq!(written.lock().unwrap().as_slice(), b"alphabeta");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_multiple_wezterm_action_parenthesized_table_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Multiple({ wezterm.action.SendString(\"alpha\"), wezterm.action.SendString('beta') })"
                .to_owned(),
        );

        let expected = WindowCommand::Multiple(vec![
            WindowCommand::SendString("alpha".to_owned()),
            WindowCommand::SendString("beta".to_owned()),
        ]);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));
        assert_eq!(written.lock().unwrap().as_slice(), b"alphabeta");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_multiple_mixed_case_nested_nop_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("Multiple ActivateCopyMode ; NoP".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::ActivateCopyMode,
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_spaced_nested_action_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("Multiple Show Debug Overlay ; NoP".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::ShowDebugOverlay,
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_show_launcher_args_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("Multiple showlauncherargs tabs ; NoP".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::ShowLauncherArgs(WindowShowLauncherArgs {
                    flags: WindowShowLauncherFlags::tabs(),
                    title: None,
                    alphabet: None,
                    help_text: None,
                    fuzzy_help_text: None,
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_char_select_args_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "Multiple charselect group PeopleAndBody copy_to clipboard ; NoP".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::CharSelectArgs(WindowCharSelectOptions {
                    copy_on_select: true,
                    copy_to: WindowCopyDestination::Clipboard,
                    group: Some("PeopleAndBody".to_owned()),
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_quick_select_args_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "Multiple quickselectargs action open uri pattern ticket-[0-9]+ ; NoP".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::QuickSelect(WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    action: Some(WindowQuickSelectAction::OpenUri),
                    ..WindowQuickSelectOptions::default()
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_quick_select_lua_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Multiple({ wezterm.action.QuickSelectArgs({ pattern = \"ticket-[0-9]+\", action = \"open-uri\" }), wezterm.action.Nop() })"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::QuickSelectArgs(WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    action: Some(WindowQuickSelectAction::OpenUri),
                    ..WindowQuickSelectOptions::default()
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_search_lua_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Multiple({ wezterm.action.Search({ Regex = \"ticket-[0-9]+\" }), wezterm.action.Nop() })"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::Search(WindowSearchCommandQuery::Pattern {
                    pattern: "ticket-[0-9]+".to_owned(),
                    match_type: WindowSearchMatchType::Regex,
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_switch_workspace_lua_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Multiple({ wezterm.action.SwitchToWorkspace({ name = \"monitoring\", spawn = { args = { \"top\", \"-d\", \"1\" }, cwd = \"C:/Mon\" } }), wezterm.action.Nop() })"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::SwitchToWorkspaceArgs(WindowSwitchToWorkspaceOptions {
                    name: Some("monitoring".to_owned()),
                    command: Some(WindowSpawnCommandQuery {
                        label: None,
                        program: "top".to_owned(),
                        args: vec!["-d".to_owned(), "1".to_owned()],
                        cwd: Some("C:/Mon".to_owned()),
                        environment: BTreeMap::new(),
                        domain: None,
                        window_position: None,
                    }),
                    command_options: None,
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_activate_tab_relative_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("Multiple activatetabrelative 2 ; NoP".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::ActivateTabRelative(2),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_spawn_command_in_new_tab_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "Multiple spawncommandinnewtab --cwd /tmp/project --env MODE=dev top -d 1 ; NoP"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::SpawnCommandInNewTab(WindowSpawnCommandQuery {
                    label: None,
                    domain: None,
                    cwd: Some("/tmp/project".to_owned()),
                    environment: BTreeMap::from([("MODE".to_owned(), "dev".to_owned())]),
                    window_position: None,
                    program: "top".to_owned(),
                    args: vec!["-d".to_owned(), "1".to_owned()],
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_multiple_nested_spawn_command_in_new_tab_options_query_applies_default_launch_options()
     {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "Multiple spawncommandinnewtab --cwd \"C:/Nested Dir\" ; NoP".to_owned(),
        );
        let command = app.command_palette_filtered_commands().remove(0);

        assert!(app.command_palette_execute(command));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-NoProfile"]);
        assert_eq!(launch.cwd(), Some("C:/Nested Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_multiple_nested_spawn_command_in_new_window_options_query_applies_default_launch_options()
     {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "Multiple spawncommandinnewwindow --cwd \"C:/Nested Window\" --position main:42,84 ; NoP"
                .to_owned(),
        );
        let command = app.command_palette_filtered_commands().remove(0);

        assert!(app.command_palette_execute(command));

        let detached_app = app
            .take_next_pending_window_app()
            .expect("spawn window options should request a pending detached window");
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
        assert_eq!(launch.cwd(), Some("C:/Nested Window"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_multiple_nested_rename_tab_query_applies_explicit_title() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("Multiple rename tab nested-build ; NoP".to_owned());
        let command = app.command_palette_filtered_commands().remove(0);

        assert!(app.command_palette_execute(command));

        assert_eq!(app.app_shell.active_tab().title(), Some("nested-build"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_multiple_nested_rename_workspace_query_applies_explicit_name() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("Multiple rename workspace deploy-west ; NoP".to_owned());
        let command = app.command_palette_filtered_commands().remove(0);

        assert!(app.command_palette_execute(command));

        assert_eq!(app.app_shell.active_workspace().name(), "deploy-west");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_multiple_nested_pane_select_alphabet_query_applies_explicit_alphabet() {
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
        app.command_palette_set_query("Multiple pane select alphabet 12 ; NoP".to_owned());
        let command = app.command_palette_filtered_commands().remove(0);

        assert!(app.command_palette_execute(command));

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
    }

    #[test]
    fn window_app_multiple_nested_pane_select_mode_show_ids_query_applies_explicit_options() {
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
        app.command_palette_set_query(
            "Multiple pane select swap show pane ids alphabet 12 ; NoP".to_owned(),
        );
        let command = app.command_palette_filtered_commands().remove(0);

        assert!(app.command_palette_execute(command));

        let pane_select = app
            .pane_select
            .as_ref()
            .expect("pane select should be active");
        assert_eq!(pane_select.mode, WindowPaneSelectMode::SwapWithActive);
        assert!(pane_select.show_pane_ids);
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[1].label, "2");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_split_horizontal_command_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("Multiple split horizontal top -d 1 ; NoP".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::SplitPane(WindowSplitPaneOptions {
                    direction: rterm_core::app_shell::SplitDirection::Right,
                    domain: None,
                    command: Some(WindowSpawnCommandQuery {
                        label: None,
                        program: "top".to_owned(),
                        args: vec!["-d".to_owned(), "1".to_owned()],
                        cwd: None,
                        environment: BTreeMap::new(),
                        domain: None,
                        window_position: None,
                    }),
                    command_options: None,
                    size: None,
                    top_level: false,
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_split_horizontal_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "Multiple SplitHorizontal={domain=\"CurrentPaneDomain\"} ; NoP".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::SplitPane(WindowSplitPaneOptions {
                    direction: rterm_core::app_shell::SplitDirection::Right,
                    domain: Some(WindowSpawnTabDomain::CurrentPaneDomain),
                    command: None,
                    command_options: None,
                    size: None,
                    top_level: false,
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_split_pane_spaced_table_assignment_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "Multiple SplitPane = { direction = \"Right\", domain = \"CurrentPaneDomain\" } ; NoP"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::SplitPane(WindowSplitPaneOptions {
                    direction: rterm_core::app_shell::SplitDirection::Right,
                    domain: Some(WindowSpawnTabDomain::CurrentPaneDomain),
                    command: None,
                    command_options: None,
                    size: None,
                    top_level: false,
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_split_vertical_spaced_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "Multiple SplitVertical={ domain = \"CurrentPaneDomain\" } ; NoP".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::SplitPane(WindowSplitPaneOptions {
                    direction: rterm_core::app_shell::SplitDirection::Down,
                    domain: Some(WindowSpawnTabDomain::CurrentPaneDomain),
                    command: None,
                    command_options: None,
                    size: None,
                    top_level: false,
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_split_horizontal_size_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "Multiple SplitHorizontal={ size = { Percent = 30 } } ; NoP".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::SplitPane(WindowSplitPaneOptions {
                    direction: rterm_core::app_shell::SplitDirection::Right,
                    domain: None,
                    command: None,
                    command_options: None,
                    size: Some(WindowSplitPaneSize::Percent(30)),
                    top_level: false,
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_split_horizontal_bracket_size_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "Multiple SplitHorizontal={ size = { [\"Percent\"] = 30 } } ; NoP".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::SplitPane(WindowSplitPaneOptions {
                    direction: rterm_core::app_shell::SplitDirection::Right,
                    domain: None,
                    command: None,
                    command_options: None,
                    size: Some(WindowSplitPaneSize::Percent(30)),
                    top_level: false,
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_split_vertical_domain_and_size_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "Multiple SplitVertical={ domain = \"CurrentPaneDomain\", size = { Cells = 20 }, } ; NoP"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::SplitPane(WindowSplitPaneOptions {
                    direction: rterm_core::app_shell::SplitDirection::Down,
                    domain: Some(WindowSpawnTabDomain::CurrentPaneDomain),
                    command: None,
                    command_options: None,
                    size: Some(WindowSplitPaneSize::Cells(20)),
                    top_level: false,
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_split_horizontal_top_level_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "Multiple SplitHorizontal={ domain = \"CurrentPaneDomain\", top_level = true } ; NoP"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::SplitPane(WindowSplitPaneOptions {
                    direction: rterm_core::app_shell::SplitDirection::Right,
                    domain: Some(WindowSpawnTabDomain::CurrentPaneDomain),
                    command: None,
                    command_options: None,
                    size: None,
                    top_level: true,
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_split_pane_direction_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "Multiple SplitPane={ direction = \"Right\", domain = \"CurrentPaneDomain\", size = { Percent = 40 } } ; NoP"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::SplitPane(WindowSplitPaneOptions {
                    direction: rterm_core::app_shell::SplitDirection::Right,
                    domain: Some(WindowSpawnTabDomain::CurrentPaneDomain),
                    command: None,
                    command_options: None,
                    size: Some(WindowSplitPaneSize::Percent(40)),
                    top_level: false,
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_split_pane_command_args_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "Multiple SplitPane={ direction = \"Down\", command = { args = { \"top\", \"-d\", \"1\" } } } ; NoP"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::SplitPane(WindowSplitPaneOptions {
                    direction: rterm_core::app_shell::SplitDirection::Down,
                    domain: None,
                    command: Some(WindowSpawnCommandQuery {
                        label: None,
                        program: "top".to_owned(),
                        args: vec!["-d".to_owned(), "1".to_owned()],
                        cwd: None,
                        environment: BTreeMap::new(),
                        domain: None,
                        window_position: None,
                    }),
                    command_options: None,
                    size: None,
                    top_level: false,
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_split_pane_bracket_option_key_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "Multiple SplitPane={ [\"direction\"] = \"Down\", [\"command\"] = { [\"args\"] = { \"top\", \"-d\", \"1\" } } } ; NoP"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::SplitPane(WindowSplitPaneOptions {
                    direction: rterm_core::app_shell::SplitDirection::Down,
                    domain: None,
                    command: Some(WindowSpawnCommandQuery {
                        label: None,
                        program: "top".to_owned(),
                        args: vec!["-d".to_owned(), "1".to_owned()],
                        cwd: None,
                        environment: BTreeMap::new(),
                        domain: None,
                        window_position: None,
                    }),
                    command_options: None,
                    size: None,
                    top_level: false,
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_split_pane_command_cwd_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "Multiple SplitPane={ direction = \"Down\", command = { cwd = \"C:/Project Dir\", args = { \"top\", \"-d\", \"1\" } } } ; NoP"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::SplitPane(WindowSplitPaneOptions {
                    direction: rterm_core::app_shell::SplitDirection::Down,
                    domain: None,
                    command: Some(WindowSpawnCommandQuery {
                        label: None,
                        program: "top".to_owned(),
                        args: vec!["-d".to_owned(), "1".to_owned()],
                        cwd: Some("C:/Project Dir".to_owned()),
                        environment: BTreeMap::new(),
                        domain: None,
                        window_position: None,
                    }),
                    command_options: None,
                    size: None,
                    top_level: false,
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_split_pane_command_env_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "Multiple SplitPane={ direction = \"Down\", command = { set_environment_variables = { MODE = \"dev\" }, args = { \"top\", \"-d\", \"1\" } } } ; NoP"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::SplitPane(WindowSplitPaneOptions {
                    direction: rterm_core::app_shell::SplitDirection::Down,
                    domain: None,
                    command: Some(WindowSpawnCommandQuery {
                        label: None,
                        program: "top".to_owned(),
                        args: vec!["-d".to_owned(), "1".to_owned()],
                        cwd: None,
                        environment: BTreeMap::from([("MODE".to_owned(), "dev".to_owned())]),
                        domain: None,
                        window_position: None,
                    }),
                    command_options: None,
                    size: None,
                    top_level: false,
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_nested_split_pane_command_domain_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "Multiple SplitPane={ direction = \"Down\", command = { domain = \"local\", args = { \"top\", \"-d\", \"1\" } } } ; NoP"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Multiple(vec![
                WindowCommand::SplitPane(WindowSplitPaneOptions {
                    direction: rterm_core::app_shell::SplitDirection::Down,
                    domain: None,
                    command: Some(WindowSpawnCommandQuery {
                        label: None,
                        program: "top".to_owned(),
                        args: vec!["-d".to_owned(), "1".to_owned()],
                        cwd: None,
                        environment: BTreeMap::new(),
                        domain: Some(WindowSpawnTabDomain::DomainName("local".to_owned())),
                        window_position: None,
                    }),
                    command_options: None,
                    size: None,
                    top_level: false,
                }),
                WindowCommand::Nop,
            ])]
        );
    }

    #[test]
    fn window_app_dispatches_palette_multiple_query_with_quoted_semicolon() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "multiple send string \"alpha ; beta\" ; send string gamma".to_owned(),
        );
        let expected = WindowCommand::Multiple(vec![
            WindowCommand::SendString("alpha ; beta".to_owned()),
            WindowCommand::SendString("gamma".to_owned()),
        ]);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));
        assert_eq!(written.lock().unwrap().as_slice(), b"alpha ; betagamma");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_palette_multiple_stops_after_failed_command() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        let error = app
            .command_palette_apply_command(WindowCommand::Multiple(vec![
                WindowCommand::CloseWorkspace,
                WindowCommand::NewTab,
            ]))
            .unwrap_err();

        assert_eq!(error, AppShellError::CannotCloseLastWorkspace);
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
    }

    #[test]
    fn window_app_palette_nop_has_no_window_side_effects() {
        let mut app = NativeWindowApp::new(None);
        let active_tab = app.active_tab_id();
        let active_pane = app.app_shell.active_pane_id();

        app.command_palette_apply_command(WindowCommand::Nop)
            .unwrap();

        assert_eq!(app.active_tab_id(), active_tab);
        assert_eq!(app.app_shell.active_pane_id(), active_pane);
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
        assert!(!app.window_close_requested_for_test());
        assert!(!app.application_quit_requested_for_test());
        assert!(!app.debug_overlay_active_for_test());
    }

    #[test]
    fn window_app_dispatches_palette_nop_query() {
        let mut app = NativeWindowApp::new(None);
        let active_tab = app.active_tab_id();
        let active_pane = app.app_shell.active_pane_id();

        app.enter_command_palette_mode();
        app.command_palette_set_query("nop".to_owned());
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Nop]
        );

        assert!(app.command_palette_execute(WindowCommand::Nop));

        assert_eq!(app.active_tab_id(), active_tab);
        assert_eq!(app.app_shell.active_pane_id(), active_pane);
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_palette_multiple_continues_after_nop() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::Multiple(vec![
            WindowCommand::Nop,
            WindowCommand::NewTab,
        ])));

        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_tab_relative_commands() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::NextTab);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::PreviousTab);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_tab_no_wrap_commands() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::NextTabNoWrap);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::PreviousTabNoWrap);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert!(app.command_palette.is_none());

        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::PreviousTabNoWrap);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_activate_tab_relative_payloads() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));

        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ActivateTabRelative(2));
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ActivateTabRelative(1));
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ActivateTabRelativeNoWrap(5));
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ActivateTabRelativeNoWrap(-5));
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_tab_relative_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));

        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        app.enter_command_palette_mode();
        app.command_palette_set_query("activate tab relative 2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivateTabRelative(2)]
        );

        app.command_palette_execute(WindowCommand::ActivateTabRelative(2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_tab_relative_wezterm_action_function_call_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));

        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.ActivateTabRelative(2)".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivateTabRelative(2)]
        );

        app.command_palette_execute(WindowCommand::ActivateTabRelative(2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_tab_relative_wezterm_action_table_wrapper_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action({ ActivateTabRelative = -1 })".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivateTabRelative(-1)]
        );

        app.command_palette_execute(WindowCommand::ActivateTabRelative(-1));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_paste_from_wezterm_action_table_wrapper_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action{PasteFrom=\"Clipboard\"}".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::PasteFrom(WindowPasteSource::Clipboard)]
        );
    }

    #[test]
    fn window_app_dispatches_palette_paste_from_wezterm_action_alias_table_wrapper_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("act{PasteFrom=\"Clipboard\"}".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::PasteFrom(WindowPasteSource::Clipboard)]
        );
    }

    #[test]
    fn window_app_dispatches_palette_activate_tab_relative_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));

        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        app.enter_command_palette_mode();
        app.command_palette_set_query("activate tab relative=2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivateTabRelative(2)]
        );

        app.command_palette_execute(WindowCommand::ActivateTabRelative(2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_tab_relative_offset_assignment_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));

        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        app.enter_command_palette_mode();
        app.command_palette_set_query("activatetabrelative offset=2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivateTabRelative(2)]
        );

        app.command_palette_execute(WindowCommand::ActivateTabRelative(2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_tab_relative_no_wrap_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate tab relative no wrap 2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivateTabRelativeNoWrap(2)]
        );

        app.command_palette_execute(WindowCommand::ActivateTabRelativeNoWrap(2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_tab_relative_no_wrap_wezterm_action_function_call_query()
     {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.ActivateTabRelativeNoWrap(2)".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivateTabRelativeNoWrap(2)]
        );

        app.command_palette_execute(WindowCommand::ActivateTabRelativeNoWrap(2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_tab_relative_no_wrap_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate tab relative no wrap=2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivateTabRelativeNoWrap(2)]
        );

        app.command_palette_execute(WindowCommand::ActivateTabRelativeNoWrap(2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_tab_relative_no_wrap_offset_assignment_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));

        app.enter_command_palette_mode();
        app.command_palette_set_query("activatetabrelativenowrap offset=2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivateTabRelativeNoWrap(2)]
        );

        app.command_palette_execute(WindowCommand::ActivateTabRelativeNoWrap(2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_tab_index_commands() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..8 {
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
        }
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(9));

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ActivateTab1);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ActivateTab3);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ActivateTab9);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(9));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_tab_query() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..3 {
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
        }
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(4));

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate tab 2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivateTab(2)]
        );

        app.command_palette_execute(WindowCommand::ActivateTab(2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_tab_wezterm_action_function_call_query() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..3 {
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
        }
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(4));

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.ActivateTab(2)".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivateTab(2)]
        );

        app.command_palette_execute(WindowCommand::ActivateTab(2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_tab_index_query() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..10 {
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
        }
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate tab index 10".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivateTab(10)]
        );

        app.command_palette_execute(WindowCommand::ActivateTab(10));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(11));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_tab_index_equals_query() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..10 {
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
        }
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate tab index=10".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivateTab(10)]
        );

        app.command_palette_execute(WindowCommand::ActivateTab(10));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(11));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_tab_index_assignment_query() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..10 {
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
        }
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));

        app.enter_command_palette_mode();
        app.command_palette_set_query("activatetab index=10".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivateTab(10)]
        );

        app.command_palette_execute(WindowCommand::ActivateTab(10));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(11));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_navigation_action_name_queries() {
        for (query, expected) in [
            ("activatetab 10", WindowCommand::ActivateTab(10)),
            (
                "activatetabrelative -2",
                WindowCommand::ActivateTabRelative(-2),
            ),
            (
                "activatetabrelativenowrap 2",
                WindowCommand::ActivateTabRelativeNoWrap(2),
            ),
            ("movetab 9", WindowCommand::MoveTab(9)),
            ("movetabrelative -2", WindowCommand::MoveTabRelative(-2)),
            (
                "activatepanebyindex 9",
                WindowCommand::ActivatePaneByIndex(9),
            ),
            ("activatewindow 2", WindowCommand::ActivateWindow(2)),
            (
                "activatewindowrelative -2",
                WindowCommand::ActivateWindowRelative(-2),
            ),
            (
                "activatewindowrelativenowrap -1",
                WindowCommand::ActivateWindowRelativeNoWrap(-1),
            ),
            ("renametab release shell", WindowCommand::RenameTab),
            ("renameworkspace ops", WindowCommand::RenameWorkspace),
            ("switchtoworkspace prod", WindowCommand::SwitchToWorkspace),
            (
                "switchworkspacerelative -1",
                WindowCommand::SwitchWorkspaceRelative(-1),
            ),
        ] {
            let mut app = NativeWindowApp::new(None);
            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());

            assert_eq!(app.command_palette_filtered_commands(), vec![expected]);
        }
    }

    #[test]
    fn window_app_dispatches_native_activate_tab_payload() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..3 {
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
        }
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(4));

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ActivateTab(1));
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ActivateTab(-1));
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(4));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_move_tab_to_index_commands() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        let tab_order = |app: &NativeWindowApp| -> Vec<rterm_core::TabId> {
            app.app_shell
                .active_workspace()
                .tabs()
                .iter()
                .map(rterm_core::app_shell::Tab::id)
                .collect()
        };

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::MoveTabTo3);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(
            tab_order(&app),
            vec![
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(3),
                rterm_core::TabId::new(1)
            ]
        );
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::MoveTabTo1);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(
            tab_order(&app),
            vec![
                rterm_core::TabId::new(1),
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(3)
            ]
        );
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        assert!(!app.command_palette_execute(WindowCommand::MoveTabTo8));
        assert_eq!(
            tab_order(&app),
            vec![
                rterm_core::TabId::new(1),
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(3)
            ]
        );
        assert!(app.command_palette.is_some());
        app.exit_command_palette_mode();

        for _ in 0..5 {
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
        }
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::MoveTabTo8));
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(
            tab_order(&app),
            vec![
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(3),
                rterm_core::TabId::new(4),
                rterm_core::TabId::new(5),
                rterm_core::TabId::new(6),
                rterm_core::TabId::new(7),
                rterm_core::TabId::new(8),
                rterm_core::TabId::new(1)
            ]
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_move_tab_payload() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        let tab_order = |app: &NativeWindowApp| -> Vec<rterm_core::TabId> {
            app.app_shell
                .active_workspace()
                .tabs()
                .iter()
                .map(rterm_core::app_shell::Tab::id)
                .collect()
        };

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::MoveTab(2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(
            tab_order(&app),
            vec![
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(3),
                rterm_core::TabId::new(1)
            ]
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_move_tab_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        let tab_order = |app: &NativeWindowApp| -> Vec<rterm_core::TabId> {
            app.app_shell
                .active_workspace()
                .tabs()
                .iter()
                .map(rterm_core::app_shell::Tab::id)
                .collect()
        };

        app.enter_command_palette_mode();
        app.command_palette_set_query("move tab 2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::MoveTab(2)]
        );

        app.command_palette_execute(WindowCommand::MoveTab(2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(
            tab_order(&app),
            vec![
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(3),
                rterm_core::TabId::new(1)
            ]
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_move_tab_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        let tab_order = |app: &NativeWindowApp| -> Vec<rterm_core::TabId> {
            app.app_shell
                .active_workspace()
                .tabs()
                .iter()
                .map(rterm_core::app_shell::Tab::id)
                .collect()
        };

        app.enter_command_palette_mode();
        app.command_palette_set_query("move tab=2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::MoveTab(2)]
        );

        app.command_palette_execute(WindowCommand::MoveTab(2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(
            tab_order(&app),
            vec![
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(3),
                rterm_core::TabId::new(1)
            ]
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_move_tab_index_assignment_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        let tab_order = |app: &NativeWindowApp| -> Vec<rterm_core::TabId> {
            app.app_shell
                .active_workspace()
                .tabs()
                .iter()
                .map(rterm_core::app_shell::Tab::id)
                .collect()
        };

        app.enter_command_palette_mode();
        app.command_palette_set_query("movetab index=2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::MoveTab(2)]
        );

        app.command_palette_execute(WindowCommand::MoveTab(2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(
            tab_order(&app),
            vec![
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(3),
                rterm_core::TabId::new(1)
            ]
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_move_tab_wezterm_action_function_call_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        let tab_order = |app: &NativeWindowApp| -> Vec<rterm_core::TabId> {
            app.app_shell
                .active_workspace()
                .tabs()
                .iter()
                .map(rterm_core::app_shell::Tab::id)
                .collect()
        };

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.MoveTab(2)".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::MoveTab(2)]
        );

        app.command_palette_execute(WindowCommand::MoveTab(2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(
            tab_order(&app),
            vec![
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(3),
                rterm_core::TabId::new(1)
            ]
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_move_tab_to_query() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..9 {
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
        }
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        let tab_order = |app: &NativeWindowApp| -> Vec<rterm_core::TabId> {
            app.app_shell
                .active_workspace()
                .tabs()
                .iter()
                .map(rterm_core::app_shell::Tab::id)
                .collect()
        };

        app.enter_command_palette_mode();
        app.command_palette_set_query("move tab to 9".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::MoveTab(9)]
        );

        app.command_palette_execute(WindowCommand::MoveTab(9));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(
            tab_order(&app),
            vec![
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(3),
                rterm_core::TabId::new(4),
                rterm_core::TabId::new(5),
                rterm_core::TabId::new(6),
                rterm_core::TabId::new(7),
                rterm_core::TabId::new(8),
                rterm_core::TabId::new(9),
                rterm_core::TabId::new(10),
                rterm_core::TabId::new(1),
            ]
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_move_tab_relative_commands() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        let tab_order = |app: &NativeWindowApp| -> Vec<rterm_core::TabId> {
            app.app_shell
                .active_workspace()
                .tabs()
                .iter()
                .map(rterm_core::app_shell::Tab::id)
                .collect()
        };

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::MoveTabRelativeLeft);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert_eq!(
            tab_order(&app),
            vec![
                rterm_core::TabId::new(1),
                rterm_core::TabId::new(3),
                rterm_core::TabId::new(2)
            ]
        );
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::MoveTabRelativeRight);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        assert_eq!(
            tab_order(&app),
            vec![
                rterm_core::TabId::new(1),
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(3)
            ]
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_move_tab_relative_query() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..3 {
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
        }
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(4));
        let tab_order = |app: &NativeWindowApp| -> Vec<rterm_core::TabId> {
            app.app_shell
                .active_workspace()
                .tabs()
                .iter()
                .map(rterm_core::app_shell::Tab::id)
                .collect()
        };

        app.enter_command_palette_mode();
        app.command_palette_set_query("move tab relative -2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::MoveTabRelative(-2)]
        );

        app.command_palette_execute(WindowCommand::MoveTabRelative(-2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(4));
        assert_eq!(
            tab_order(&app),
            vec![
                rterm_core::TabId::new(1),
                rterm_core::TabId::new(4),
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(3)
            ]
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_move_tab_relative_wezterm_action_function_call_query() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..3 {
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
        }
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(4));
        let tab_order = |app: &NativeWindowApp| -> Vec<rterm_core::TabId> {
            app.app_shell
                .active_workspace()
                .tabs()
                .iter()
                .map(rterm_core::app_shell::Tab::id)
                .collect()
        };

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.MoveTabRelative(-2)".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::MoveTabRelative(-2)]
        );

        app.command_palette_execute(WindowCommand::MoveTabRelative(-2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(4));
        assert_eq!(
            tab_order(&app),
            vec![
                rterm_core::TabId::new(1),
                rterm_core::TabId::new(4),
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(3)
            ]
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_move_tab_relative_equals_query() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..3 {
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
        }
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(4));
        let tab_order = |app: &NativeWindowApp| -> Vec<rterm_core::TabId> {
            app.app_shell
                .active_workspace()
                .tabs()
                .iter()
                .map(rterm_core::app_shell::Tab::id)
                .collect()
        };

        app.enter_command_palette_mode();
        app.command_palette_set_query("move tab relative=-2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::MoveTabRelative(-2)]
        );

        app.command_palette_execute(WindowCommand::MoveTabRelative(-2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(4));
        assert_eq!(
            tab_order(&app),
            vec![
                rterm_core::TabId::new(1),
                rterm_core::TabId::new(4),
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(3)
            ]
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_move_tab_relative_offset_assignment_query() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..3 {
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
        }
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(4));
        let tab_order = |app: &NativeWindowApp| -> Vec<rterm_core::TabId> {
            app.app_shell
                .active_workspace()
                .tabs()
                .iter()
                .map(rterm_core::app_shell::Tab::id)
                .collect()
        };

        app.enter_command_palette_mode();
        app.command_palette_set_query("movetabrelative offset=-2".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::MoveTabRelative(-2)]
        );

        app.command_palette_execute(WindowCommand::MoveTabRelative(-2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(4));
        assert_eq!(
            tab_order(&app),
            vec![
                rterm_core::TabId::new(1),
                rterm_core::TabId::new(4),
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(3)
            ]
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_move_tab_relative_payload() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..3 {
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
        }
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(2),
        })
        .unwrap();
        let tab_order = |app: &NativeWindowApp| -> Vec<rterm_core::TabId> {
            app.app_shell
                .active_workspace()
                .tabs()
                .iter()
                .map(rterm_core::app_shell::Tab::id)
                .collect()
        };

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::MoveTabRelative(2));

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(
            tab_order(&app),
            vec![
                rterm_core::TabId::new(1),
                rterm_core::TabId::new(3),
                rterm_core::TabId::new(4),
                rterm_core::TabId::new(2)
            ]
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_pane_left_command() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ActivatePaneLeft);

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_activate_pane_direction_payload() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..2 {
            app.dispatch_app_action(AppAction::SplitPane {
                pane: app.active_pane_id(),
                direction: rterm_core::app_shell::SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        }
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ActivatePaneDirection(
            rterm_core::app_shell::PaneDirection::Previous,
        ));

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_pane_query() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..2 {
            app.dispatch_app_action(AppAction::SplitPane {
                pane: app.active_pane_id(),
                direction: rterm_core::app_shell::SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        }
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate pane 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivatePaneByIndex(1)]
        );

        app.command_palette_execute(WindowCommand::ActivatePaneByIndex(1));

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_pane_by_index_query() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..9 {
            app.dispatch_app_action(AppAction::SplitPane {
                pane: app.active_pane_id(),
                direction: rterm_core::app_shell::SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        }
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(10));
        app.dispatch_app_action(AppAction::ActivatePaneByIndex { index: 0 })
            .unwrap();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate pane by index 9".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivatePaneByIndex(9)]
        );

        app.command_palette_execute(WindowCommand::ActivatePaneByIndex(9));

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(10));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_pane_by_index_equals_query() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..9 {
            app.dispatch_app_action(AppAction::SplitPane {
                pane: app.active_pane_id(),
                direction: rterm_core::app_shell::SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        }
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(10));
        app.dispatch_app_action(AppAction::ActivatePaneByIndex { index: 0 })
            .unwrap();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate pane by index=9".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivatePaneByIndex(9)]
        );

        app.command_palette_execute(WindowCommand::ActivatePaneByIndex(9));

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(10));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_pane_by_index_index_assignment_query() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..9 {
            app.dispatch_app_action(AppAction::SplitPane {
                pane: app.active_pane_id(),
                direction: rterm_core::app_shell::SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        }
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(10));
        app.dispatch_app_action(AppAction::ActivatePaneByIndex { index: 0 })
            .unwrap();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));

        app.enter_command_palette_mode();
        app.command_palette_set_query("activatepanebyindex index=9".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivatePaneByIndex(9)]
        );

        app.command_palette_execute(WindowCommand::ActivatePaneByIndex(9));

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(10));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_pane_by_index_wezterm_action_function_call_query() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..9 {
            app.dispatch_app_action(AppAction::SplitPane {
                pane: app.active_pane_id(),
                direction: rterm_core::app_shell::SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        }
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(10));
        app.dispatch_app_action(AppAction::ActivatePaneByIndex { index: 0 })
            .unwrap();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.ActivatePaneByIndex(9)".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivatePaneByIndex(9)]
        );

        app.command_palette_execute(WindowCommand::ActivatePaneByIndex(9));

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(10));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_pane_direction_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate pane direction left".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivatePaneDirection(
                rterm_core::app_shell::PaneDirection::Left
            )]
        );

        app.command_palette_execute(WindowCommand::ActivatePaneDirection(
            rterm_core::app_shell::PaneDirection::Left,
        ));

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_pane_direction_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate pane direction=left".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivatePaneDirection(
                rterm_core::app_shell::PaneDirection::Left
            )]
        );

        app.command_palette_execute(WindowCommand::ActivatePaneDirection(
            rterm_core::app_shell::PaneDirection::Left,
        ));

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_pane_direction_assignment_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));

        app.enter_command_palette_mode();
        app.command_palette_set_query("activatepanedirection direction=left".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivatePaneDirection(
                rterm_core::app_shell::PaneDirection::Left
            )]
        );

        app.command_palette_execute(WindowCommand::ActivatePaneDirection(
            rterm_core::app_shell::PaneDirection::Left,
        ));

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_pane_direction_wezterm_action_bare_string_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.ActivatePaneDirection 'Left'".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivatePaneDirection(
                rterm_core::app_shell::PaneDirection::Left
            )]
        );

        app.command_palette_execute(WindowCommand::ActivatePaneDirection(
            rterm_core::app_shell::PaneDirection::Left,
        ));

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_pane_direction_wezterm_action_function_call_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.ActivatePaneDirection(\"Left\")".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivatePaneDirection(
                rterm_core::app_shell::PaneDirection::Left
            )]
        );

        app.command_palette_execute(WindowCommand::ActivatePaneDirection(
            rterm_core::app_shell::PaneDirection::Left,
        ));

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_pane_direction_wezterm_action_table_wrapper_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action { ActivatePaneDirection = 'Left' }".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ActivatePaneDirection(
                rterm_core::app_shell::PaneDirection::Left
            )]
        );

        app.command_palette_execute(WindowCommand::ActivatePaneDirection(
            rterm_core::app_shell::PaneDirection::Left,
        ));

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_horizontal_spawn_command_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("split horizontal top -d 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitRight]
        );

        app.command_palette_execute(WindowCommand::SplitRight);

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_horizontal_equals_spawn_command_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("split horizontal=top -d 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitRight]
        );

        app.command_palette_execute(WindowCommand::SplitRight);

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_horizontal_wezterm_action_table_call_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SplitHorizontal { command = { args = { \"top\", \"-d\", \"1\" } } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitPane(WindowSplitPaneOptions {
                direction: rterm_core::app_shell::SplitDirection::Right,
                domain: None,
                command: Some(WindowSpawnCommandQuery {
                    label: None,
                    program: "top".to_owned(),
                    args: vec!["-d".to_owned(), "1".to_owned()],
                    cwd: None,
                    environment: BTreeMap::new(),
                    domain: None,
                    window_position: None,
                }),
                command_options: None,
                size: None,
                top_level: false,
            })]
        );

        app.command_palette_execute(WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: false,
        }));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_horizontal_wezterm_action_parenthesized_table_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SplitHorizontal({ command = { args = { \"top\", \"-d\", \"1\" } } })"
                .to_owned(),
        );

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_indexed_wezterm_action_table_constructor_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action[\"SpawnCommandInNewTab\"] { args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        app.command_palette_execute(WindowCommand::NewTab);

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
    }

    #[test]
    fn window_app_dispatches_long_bracket_indexed_wezterm_action_table_constructor_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action[ [[SpawnCommandInNewTab]] ] { args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        app.command_palette_execute(WindowCommand::NewTab);

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
    }

    #[test]
    fn window_app_dispatches_commented_indexed_wezterm_action_table_constructor_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action[\"SpawnCommandInNewTab\"] -- spawn options\n { args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        app.command_palette_execute(WindowCommand::NewTab);

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
    }

    #[test]
    fn window_app_dispatches_wezterm_action_comment_before_index_table_constructor_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action -- indexed action\n [\"SpawnCommandInNewTab\"] { args = { \"top\", \"-d\", \"1\" } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::NewTab]
        );
        app.command_palette_execute(WindowCommand::NewTab);

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
    }

    #[test]
    fn window_app_dispatches_palette_split_horizontal_wezterm_action_table_wrapper_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action{ SplitHorizontal = { domain = \"CurrentPaneDomain\" } }".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitPane(WindowSplitPaneOptions {
                direction: rterm_core::app_shell::SplitDirection::Right,
                domain: Some(WindowSpawnTabDomain::CurrentPaneDomain),
                command: None,
                command_options: None,
                size: None,
                top_level: false,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_split_vertical_wezterm_action_parenthesized_table_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SplitVertical({ command = { args = { \"top\", \"-d\", \"1\" } } })"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitPane(WindowSplitPaneOptions {
                direction: rterm_core::app_shell::SplitDirection::Down,
                domain: None,
                command: Some(WindowSpawnCommandQuery {
                    label: None,
                    program: "top".to_owned(),
                    args: vec!["-d".to_owned(), "1".to_owned()],
                    cwd: None,
                    environment: BTreeMap::new(),
                    domain: None,
                    window_position: None,
                }),
                command_options: None,
                size: None,
                top_level: false,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_split_horizontal_wezterm_spawn_command_table_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SplitHorizontal({ cwd = \"C:/Project Dir\", set_environment_variables = { MODE = \"dev\" }, args = { \"top\", \"-d\", \"1\" } })"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitPane(WindowSplitPaneOptions {
                direction: rterm_core::app_shell::SplitDirection::Right,
                domain: None,
                command: Some(WindowSpawnCommandQuery {
                    label: None,
                    program: "top".to_owned(),
                    args: vec!["-d".to_owned(), "1".to_owned()],
                    cwd: Some("C:/Project Dir".to_owned()),
                    environment: BTreeMap::from([("MODE".to_owned(), "dev".to_owned())]),
                    domain: None,
                    window_position: None,
                }),
                command_options: None,
                size: None,
                top_level: false,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_wezterm_action_parenthesized_table_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SplitPane({ direction = \"Left\", size = { Cells = 20 }, command = { cwd = \"C:/Project Dir\", args = { \"top\", \"-d\", \"1\" } } })"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitPane(WindowSplitPaneOptions {
                direction: rterm_core::app_shell::SplitDirection::Left,
                domain: None,
                command: Some(WindowSpawnCommandQuery {
                    label: None,
                    program: "top".to_owned(),
                    args: vec!["-d".to_owned(), "1".to_owned()],
                    cwd: Some("C:/Project Dir".to_owned()),
                    environment: BTreeMap::new(),
                    domain: None,
                    window_position: None,
                }),
                command_options: None,
                size: Some(WindowSplitPaneSize::Cells(20)),
                top_level: false,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_wezterm_action_table_long_bracket_key_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.SplitPane { [[=[direction]=]] = [[Left]], [[=[size]=]] = { [[=[Cells]=]] = 20 }, [[=[command]=]] = { [[=[cwd]=]] = [[C:/Project Dir]], [[=[args]=]] = { [[top]], [[-d]], [[1]] } } }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitPane(WindowSplitPaneOptions {
                direction: rterm_core::app_shell::SplitDirection::Left,
                domain: None,
                command: Some(WindowSpawnCommandQuery {
                    label: None,
                    program: "top".to_owned(),
                    args: vec!["-d".to_owned(), "1".to_owned()],
                    cwd: Some("C:/Project Dir".to_owned()),
                    environment: BTreeMap::new(),
                    domain: None,
                    window_position: None,
                }),
                command_options: None,
                size: Some(WindowSplitPaneSize::Cells(20)),
                top_level: false,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_native_split_horizontal_alias_payload() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        assert!(app.command_palette_execute(WindowCommand::SplitHorizontal));

        let split = app
            .app_shell
            .active_pane()
            .split()
            .expect("new pane should record split metadata");
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(split.direction, rterm_core::app_shell::SplitDirection::Right);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_horizontal_quoted_spawn_command_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "split horizontal --cwd \"C:/Project Dir\" --env \"GREETING=hello world\" powershell \"-No Logo\""
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitRight]
        );

        app.command_palette_execute(WindowCommand::SplitRight);

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
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
    fn window_app_dispatches_palette_mixed_case_split_pane_action_name_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "SplitPane Right --cwd \"C:/Project Dir\" --env \"GREETING=hello world\" powershell \"-No Logo\""
                .to_owned(),
        );

        let expected_command = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "powershell".to_owned(),
                args: vec!["-No Logo".to_owned()],
                cwd: Some("C:/Project Dir".to_owned()),
                environment: BTreeMap::from([("GREETING".to_owned(), "hello world".to_owned())]),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected_command.clone()]
        );

        app.command_palette_execute(expected_command);

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
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
    fn window_app_applies_palette_split_horizontal_spawn_options_without_program_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "split horizontal --domain local --cwd \"C:/Project Dir\" --env \"GREETING=hello world\""
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitRight]
        );
        assert!(app.command_palette_execute(WindowCommand::SplitRight));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
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
    fn window_app_dispatches_palette_split_pane_set_environment_variables_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "splitpane right --set_environment_variables=SPAWN_MODE=query top -d 1".to_owned(),
        );

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::from([("SPAWN_MODE".to_owned(), "query".to_owned())]),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: false,
        });

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(
            launch.environment().get("SPAWN_MODE"),
            Some(&"query".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_hyphenated_default_domain_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "splitpane right --domain=default-domain top -d 1".to_owned(),
        );

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: Some(WindowSpawnTabDomain::DefaultDomain),
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: false,
        });

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        assert!(app.command_palette_execute(expected));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_applies_palette_split_interleaved_spawn_and_size_options_without_program_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "split horizontal --cwd \"C:/Project Dir\" --percent 30 --env \"GREETING=hello world\""
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitRight]
        );
        assert!(app.command_palette_execute(WindowCommand::SplitRight));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 24);
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
    fn window_app_dispatches_palette_split_horizontal_percent_size_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("split horizontal --percent 30 top -d 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitRight]
        );

        app.command_palette_execute(WindowCommand::SplitRight);

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 24);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_horizontal_percent_equals_size_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("split horizontal --percent=30 top -d 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitRight]
        );

        app.command_palette_execute(WindowCommand::SplitRight);

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 24);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_horizontal_cells_size_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("split horizontal --cells 20 top -d 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitRight]
        );

        app.command_palette_execute(WindowCommand::SplitRight);

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 20);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_action_name_right_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "splitpane right --cwd \"C:/Project Dir\" --Percent 30 --env \"GREETING=hello world\" powershell \"-No Logo\""
                .to_owned(),
        );

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "powershell".to_owned(),
                args: vec!["-No Logo".to_owned()],
                cwd: Some("C:/Project Dir".to_owned()),
                environment: BTreeMap::from([("GREETING".to_owned(), "hello world".to_owned())]),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Percent(30)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 24);
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
    fn window_app_dispatches_palette_split_pane_action_name_right_equals_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "splitpane right=--cwd \"C:/Project Dir\" --Percent 30 --env \"GREETING=hello world\" powershell \"-No Logo\""
                .to_owned(),
        );

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "powershell".to_owned(),
                args: vec!["-No Logo".to_owned()],
                cwd: Some("C:/Project Dir".to_owned()),
                environment: BTreeMap::from([("GREETING".to_owned(), "hello world".to_owned())]),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Percent(30)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 24);
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
    fn window_app_dispatches_palette_split_pane_action_name_direction_right_equals_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane direction right=--cells 20 top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Cells(20)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 20);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_action_name_direction_assignment_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane direction=right --cells 20 top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Cells(20)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 20);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_action_name_top_level_assignment_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "splitpane direction=down top_level=true top -d 1".to_owned(),
        );

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Down,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: true,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let new_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == rterm_core::PaneId::new(3))
            .expect("top-level split pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));
        assert_eq!(new_rect.column, 0);
        assert_eq!(new_rect.columns, 80);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_action_name_down_equals_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane down=--cells 3 top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Down,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Cells(3)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.rows, 3);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_action_name_direction_down_equals_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane direction down=--cells 3 top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Down,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Cells(3)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.rows, 3);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_action_name_left_equals_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane left=--cells 20 top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Left,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Cells(20)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 20);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_action_name_direction_left_equals_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane direction left=--cells 20 top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Left,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Cells(20)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 20);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_action_name_up_equals_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane up=--cells 3 top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Up,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Cells(3)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.rows, 3);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_action_name_direction_up_equals_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane direction up=--cells 3 top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Up,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Cells(3)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.rows, 3);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_action_name_direction_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane direction down --cells 3 top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Down,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Cells(3)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.rows, 3);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_action_name_cells_equals_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane direction down --cells=3 top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Down,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Cells(3)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.rows, 3);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_action_name_cells_assignment_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane direction=down cells=3 top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Down,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Cells(3)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.rows, 3);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_action_name_percent_assignment_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane direction=right percent=30 top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Percent(30)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 24);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_unordered_structured_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane percent=30 direction=right top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Percent(30)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 24);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_unordered_direction_word_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane percent=30 direction down top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Down,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Percent(30)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.rows, 3);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_percent_word_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane direction=right percent 30 top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Percent(30)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 24);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_cells_word_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane direction=down cells 3 top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Down,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Cells(3)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.rows, 3);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_top_level_word_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "splitpane direction=right top_level true top -d 1".to_owned(),
        );

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: true,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 40);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_top_level_on_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane direction=right top_level on top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: true,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 40);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_spaced_top_level_word_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "splitpane direction=right top level true top -d 1".to_owned(),
        );

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: true,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 40);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_spaced_top_level_assignment_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "splitpane direction=right top level=true top -d 1".to_owned(),
        );

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: true,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 40);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_action_name_command_assignment_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane direction=right command=top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_action_name_command_args_assignment_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("splitpane direction=right command args=top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_action_name_command_cwd_assignment_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "splitpane direction=right command cwd=\"C:/Project Dir\" args=top -d 1".to_owned(),
        );

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: Some("C:/Project Dir".to_owned()),
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_action_name_command_env_assignment_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "splitpane direction=right command set_environment_variables=\"GREETING=hello world\" args=top -d 1"
                .to_owned(),
        );

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::from([("GREETING".to_owned(), "hello world".to_owned())]),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(
            launch.environment().get("GREETING"),
            Some(&"hello world".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_pane_action_name_command_domain_assignment_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "splitpane direction=right command domain=local args=top -d 1".to_owned(),
        );

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: Some(WindowSpawnTabDomain::DomainName("local".to_owned())),
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );
        assert!(app.command_palette_execute(expected));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_vertical_top_level_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("split vertical --top-level top -d 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitDown]
        );

        app.command_palette_execute(WindowCommand::SplitDown);

        let layout = app.pane_render_layout();
        let new_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == rterm_core::PaneId::new(3))
            .expect("top-level split pane rect");
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));
        assert_eq!(new_rect.column, 0);
        assert_eq!(new_rect.columns, 80);
        assert_eq!(app.app_shell.active_pane().launch().program(), "top");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_vertical_top_level_equals_true_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("split vertical --top-level=true top -d 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitDown]
        );

        app.command_palette_execute(WindowCommand::SplitDown);

        let layout = app.pane_render_layout();
        let new_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == rterm_core::PaneId::new(3))
            .expect("top-level split pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));
        assert_eq!(new_rect.column, 0);
        assert_eq!(new_rect.columns, 80);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_vertical_top_level_equals_false_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("split vertical --top-level=false top -d 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitDown]
        );

        app.command_palette_execute(WindowCommand::SplitDown);

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("split pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 80);
        assert_eq!(active_rect.rows, 5);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_applies_palette_top_level_split_spawn_options_without_program_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "split vertical --top-level --cwd \"C:/Top Dir\" --env \"SPLIT_MODE=top\"".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitDown]
        );
        assert!(app.command_palette_execute(WindowCommand::SplitDown));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-NoProfile"]);
        assert_eq!(launch.cwd(), Some("C:/Top Dir"));
        assert_eq!(
            launch.environment().get("SPLIT_MODE"),
            Some(&"top".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_left_spawn_command_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("split left top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Left,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        let layout = app.pane_render_layout();
        let source_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == rterm_core::PaneId::new(1))
            .expect("source pane rect");
        let new_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("new pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(new_rect.column, 0);
        assert!(source_rect.column > new_rect.column);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_left_equals_spawn_command_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("split left=top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Left,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        let layout = app.pane_render_layout();
        let source_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == rterm_core::PaneId::new(1))
            .expect("source pane rect");
        let new_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("new pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(new_rect.column, 0);
        assert!(source_rect.column > new_rect.column);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_up_cells_size_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("split up --cells 3 top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Up,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Cells(3)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        let layout = app.pane_render_layout();
        let source_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == rterm_core::PaneId::new(1))
            .expect("source pane rect");
        let new_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("new pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(new_rect.row, TAB_BAR_ROWS);
        assert!(source_rect.row > new_rect.row);
        assert_eq!(new_rect.rows, 3);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_up_equals_cells_size_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_set_query("split up=--cells 3 top -d 1".to_owned());

        let expected = WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Up,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: Some(WindowSplitPaneSize::Cells(3)),
            top_level: false,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        let layout = app.pane_render_layout();
        let source_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == rterm_core::PaneId::new(1))
            .expect("source pane rect");
        let new_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("new pane rect");
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(new_rect.row, TAB_BAR_ROWS);
        assert!(source_rect.row > new_rect.row);
        assert_eq!(new_rect.rows, 3);
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_split_pane_action_payload() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: Some("/tmp/project".to_owned()),
                environment: BTreeMap::from([("SPAWN_MODE".to_owned(), "split".to_owned())]),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: false,
        }));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert_eq!(launch.cwd(), Some("/tmp/project"));
        assert_eq!(
            launch.environment().get("SPAWN_MODE"),
            Some(&"split".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_split_pane_top_level_payload() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Down,
            domain: None,
            command: Some(WindowSpawnCommandQuery {
                label: None,
                program: "top".to_owned(),
                args: vec!["-d".to_owned(), "1".to_owned()],
                cwd: None,
                environment: BTreeMap::new(),
                domain: None,
                window_position: None,
            }),
            command_options: None,
            size: None,
            top_level: true,
        }));

        let layout = app.pane_render_layout();
        let new_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == rterm_core::PaneId::new(3))
            .expect("top-level split pane rect");
        assert_eq!(new_rect.column, 0);
        assert_eq!(new_rect.columns, 80);
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));
        assert_eq!(app.app_shell.active_pane().launch().program(), "top");
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_split_pane_action_payload_with_percent_size() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: rterm_core::app_shell::SplitDirection::Right,
            domain: None,
            command: None,
            command_options: None,
            size: Some(WindowSplitPaneSize::Percent(30)),
            top_level: false,
        }));

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(active_rect.columns, 24);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_split_pane_left_places_new_pane_before_source() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Left,
            launch: None,
        })
        .unwrap();

        let layout = app.pane_render_layout();
        let source_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == rterm_core::PaneId::new(1))
            .expect("source pane rect");
        let new_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == rterm_core::PaneId::new(2))
            .expect("new pane rect");

        assert_eq!(new_rect.column, 0);
        assert!(source_rect.column > new_rect.column);
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
    }

    #[test]
    fn window_split_pane_up_places_new_pane_before_source() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(80, 10));

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Up,
            launch: None,
        })
        .unwrap();

        let layout = app.pane_render_layout();
        let source_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == rterm_core::PaneId::new(1))
            .expect("source pane rect");
        let new_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == rterm_core::PaneId::new(2))
            .expect("new pane rect");

        assert_eq!(new_rect.row, TAB_BAR_ROWS);
        assert!(source_rect.row > new_rect.row);
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
    }

    #[test]
    fn window_split_pane_size_cells_maps_to_source_delta() {
        assert_eq!(
            split_pane_source_size_delta(80, WindowSplitPaneSize::Cells(20)),
            20
        );
    }

    #[test]
    fn window_app_dispatches_palette_split_vertical_spawn_command_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("split vertical top -d 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitDown]
        );

        app.command_palette_execute(WindowCommand::SplitDown);

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_split_vertical_equals_spawn_command_query() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query("split vertical=top -d 1".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::SplitDown]
        );

        app.command_palette_execute(WindowCommand::SplitDown);

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-d", "1"]);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_split_vertical_alias_payload() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        assert!(app.command_palette_execute(WindowCommand::SplitVertical));

        let split = app
            .app_shell
            .active_pane()
            .split()
            .expect("new pane should record split metadata");
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(split.direction, rterm_core::app_shell::SplitDirection::Down);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_activate_pane_by_index_commands() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..7 {
            app.dispatch_app_action(AppAction::SplitPane {
                pane: app.active_pane_id(),
                direction: rterm_core::app_shell::SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        }
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(8));

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ActivatePane1);
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ActivatePane3);
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ActivatePane8);
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(8));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_activate_pane_by_index_payload() {
        let mut app = NativeWindowApp::new(None);
        for _ in 0..9 {
            app.dispatch_app_action(AppAction::SplitPane {
                pane: app.active_pane_id(),
                direction: rterm_core::app_shell::SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        }
        app.dispatch_app_action(AppAction::ActivatePaneByIndex { index: 0 })
            .unwrap();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ActivatePaneByIndex(9));

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(10));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_rotate_panes_commands() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(2),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::RotatePanesClockwise);
        assert_eq!(
            app.app_shell
                .active_tab()
                .panes()
                .iter()
                .map(rterm_core::app_shell::Pane::id)
                .collect::<Vec<_>>(),
            vec![
                rterm_core::PaneId::new(3),
                rterm_core::PaneId::new(1),
                rterm_core::PaneId::new(2)
            ]
        );
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));
        assert!(app.command_palette.is_none());

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::RotatePanesCounterClockwise);
        assert_eq!(
            app.app_shell
                .active_tab()
                .panes()
                .iter()
                .map(rterm_core::app_shell::Pane::id)
                .collect::<Vec<_>>(),
            vec![
                rterm_core::PaneId::new(1),
                rterm_core::PaneId::new(2),
                rterm_core::PaneId::new(3)
            ]
        );
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_native_rotate_panes_payload() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(2),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::RotatePanes(
            rterm_core::app_shell::PaneRotationDirection::CounterClockwise,
        ));

        assert_eq!(
            app.app_shell
                .active_tab()
                .panes()
                .iter()
                .map(rterm_core::app_shell::Pane::id)
                .collect::<Vec<_>>(),
            vec![
                rterm_core::PaneId::new(2),
                rterm_core::PaneId::new(3),
                rterm_core::PaneId::new(1)
            ]
        );
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_rotate_panes_clockwise_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(2),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("rotate panes clockwise".to_owned());

        let expected =
            WindowCommand::RotatePanes(rterm_core::app_shell::PaneRotationDirection::Clockwise);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(
            app.app_shell
                .active_tab()
                .panes()
                .iter()
                .map(rterm_core::app_shell::Pane::id)
                .collect::<Vec<_>>(),
            vec![
                rterm_core::PaneId::new(3),
                rterm_core::PaneId::new(1),
                rterm_core::PaneId::new(2)
            ]
        );
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_rotate_panes_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(2),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("rotate panes=clockwise".to_owned());

        let expected =
            WindowCommand::RotatePanes(rterm_core::app_shell::PaneRotationDirection::Clockwise);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(
            app.app_shell
                .active_tab()
                .panes()
                .iter()
                .map(rterm_core::app_shell::Pane::id)
                .collect::<Vec<_>>(),
            vec![
                rterm_core::PaneId::new(3),
                rterm_core::PaneId::new(1),
                rterm_core::PaneId::new(2)
            ]
        );
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_rotate_panes_compact_equals_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(2),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("rotatepanes=clockwise".to_owned());

        let expected =
            WindowCommand::RotatePanes(rterm_core::app_shell::PaneRotationDirection::Clockwise);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(
            app.app_shell
                .active_tab()
                .panes()
                .iter()
                .map(rterm_core::app_shell::Pane::id)
                .collect::<Vec<_>>(),
            vec![
                rterm_core::PaneId::new(3),
                rterm_core::PaneId::new(1),
                rterm_core::PaneId::new(2)
            ]
        );
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_rotate_panes_direction_assignment_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(2),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("rotatepanes direction=clockwise".to_owned());

        let expected =
            WindowCommand::RotatePanes(rterm_core::app_shell::PaneRotationDirection::Clockwise);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(
            app.app_shell
                .active_tab()
                .panes()
                .iter()
                .map(rterm_core::app_shell::Pane::id)
                .collect::<Vec<_>>(),
            vec![
                rterm_core::PaneId::new(3),
                rterm_core::PaneId::new(1),
                rterm_core::PaneId::new(2)
            ]
        );
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_rotate_panes_wezterm_action_bare_string_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(2),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.RotatePanes 'Clockwise'".to_owned());

        let expected =
            WindowCommand::RotatePanes(rterm_core::app_shell::PaneRotationDirection::Clockwise);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(
            app.app_shell
                .active_tab()
                .panes()
                .iter()
                .map(rterm_core::app_shell::Pane::id)
                .collect::<Vec<_>>(),
            vec![
                rterm_core::PaneId::new(3),
                rterm_core::PaneId::new(1),
                rterm_core::PaneId::new(2)
            ]
        );
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_rotate_panes_wezterm_action_function_call_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(2),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action.RotatePanes(\"Clockwise\")".to_owned());

        let expected =
            WindowCommand::RotatePanes(rterm_core::app_shell::PaneRotationDirection::Clockwise);
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(
            app.app_shell
                .active_tab()
                .panes()
                .iter()
                .map(rterm_core::app_shell::Pane::id)
                .collect::<Vec<_>>(),
            vec![
                rterm_core::PaneId::new(3),
                rterm_core::PaneId::new(1),
                rterm_core::PaneId::new(2)
            ]
        );
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_rotate_panes_snake_case_direction_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("rotatepanes direction=counter_clockwise".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::RotatePanes(
                rterm_core::app_shell::PaneRotationDirection::CounterClockwise
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_rotate_panes_quoted_query() {
        let mut app = NativeWindowApp::new(None);
        app.enter_command_palette_mode();
        app.command_palette_set_query("rotate panes \"clockwise\"".to_owned());

        let expected =
            WindowCommand::RotatePanes(rterm_core::app_shell::PaneRotationDirection::Clockwise);
        assert_eq!(app.command_palette_filtered_commands(), vec![expected]);
    }

    #[test]
    fn window_app_dispatches_palette_rotate_panes_counterclockwise_query() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(2),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("rotate panes counterclockwise".to_owned());

        let expected = WindowCommand::RotatePanes(
            rterm_core::app_shell::PaneRotationDirection::CounterClockwise,
        );
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![expected.clone()]
        );

        app.command_palette_execute(expected);

        assert_eq!(
            app.app_shell
                .active_tab()
                .panes()
                .iter()
                .map(rterm_core::app_shell::Pane::id)
                .collect::<Vec<_>>(),
            vec![
                rterm_core::PaneId::new(2),
                rterm_core::PaneId::new(3),
                rterm_core::PaneId::new(1)
            ]
        );
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_enter_copy_mode_command() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::EnterCopyMode);

        assert!(copy_mode_for_test(&app).is_some());
        assert!(app.command_palette.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(quick_select_for_test(&app).is_none());
    }

    #[test]
    fn window_app_exposes_wezterm_activate_copy_mode_command() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("activate copy mode".to_owned());
        let command = app
            .command_palette_filtered_commands()
            .into_iter()
            .find(|command| command.label() == "Activate Copy Mode")
            .expect("expected activate copy mode command");
        app.command_palette_execute(command);

        assert!(copy_mode_for_test(&app).is_some());
        assert!(app.command_palette.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(quick_select_for_test(&app).is_none());
    }

    #[test]
    fn window_app_dispatches_native_activate_copy_mode_payload() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::ActivateCopyMode);

        assert!(copy_mode_for_test(&app).is_some());
        assert!(app.command_palette.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(quick_select_for_test(&app).is_none());
    }

    #[test]
    fn window_app_dispatches_palette_enter_quick_select_command() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::EnterQuickSelect);

        assert!(quick_select_for_test(&app).is_some());
        assert!(app.command_palette.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(copy_mode_for_test(&app).is_none());
    }

    #[test]
    fn window_app_exposes_wezterm_quick_select_command() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("quick select".to_owned());
        let command = app
            .command_palette_filtered_commands()
            .into_iter()
            .find(|command| command.label() == "Quick Select")
            .expect("expected quick select command");
        app.command_palette_execute(command);

        assert!(quick_select_for_test(&app).is_some());
        assert!(app.command_palette.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(copy_mode_for_test(&app).is_none());
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_wezterm_action_name_queries() {
        for query in [
            "wezterm.action.QuickSelect",
            "wezterm.action.QuickSelectArgs",
        ] {
            let mut app = NativeWindowApp::new(None);
            app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
            app.handle_pty_output(b"https://example.test").unwrap();

            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());

            assert_eq!(
                app.command_palette_filtered_commands(),
                vec![WindowCommand::EnterQuickSelect]
            );

            app.command_palette_execute(WindowCommand::EnterQuickSelect);

            let quick_select = active_quick_select_for_test(&app);
            assert_eq!(quick_select.matches.len(), 1);
            assert_eq!(app.selected_text().as_deref(), Some("https://example.test"));
            assert!(app.command_palette.is_none());
            assert!(search_for_test(&app).is_none());
            assert!(copy_mode_for_test(&app).is_none());
        }
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_alphabet_query() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            quick_select_alphabet: Some("xy".to_owned()),
            ..NativeConfigSnapshot::default()
        });
        app.runtime.resize(rterm_core::TerminalSize::new(40, 1));
        app.handle_pty_output(b"https://one.test https://two.test")
            .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("quick select alphabet 12".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );

        app.command_palette_execute(WindowCommand::EnterQuickSelect);

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.labels.as_slice(), ["2", "1"]);
        assert!(app.command_palette.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(copy_mode_for_test(&app).is_none());
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_quoted_alphabet_query() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            quick_select_alphabet: Some("xy".to_owned()),
            ..NativeConfigSnapshot::default()
        });
        app.runtime.resize(rterm_core::TerminalSize::new(32, 1));
        app.handle_pty_output(b"https://one.test https://two.test")
            .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("quick select alphabet \"12\"".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );

        app.command_palette_execute(WindowCommand::EnterQuickSelect);

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.labels.as_slice(), ["2", "1"]);
        assert!(app.command_palette.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(copy_mode_for_test(&app).is_none());
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_pattern_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(48, 1));
        app.handle_pty_output(b"ticket-1234 https://default.test")
            .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query("quick select pattern ticket-[0-9]+".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );

        app.command_palette_execute(WindowCommand::EnterQuickSelect);

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 1);
        assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
        assert!(app.command_palette.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(copy_mode_for_test(&app).is_none());
    }

    #[test]
    fn window_app_dispatches_palette_quick_select_patterns_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(64, 1));
        app.handle_pty_output(b"ticket-1234 change-5678 https://default.test")
            .unwrap();

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "quick select patterns ticket-[0-9]+;change-[0-9]+".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::EnterQuickSelect]
        );

        app.command_palette_execute(WindowCommand::EnterQuickSelect);

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 2);
        assert_eq!(app.selected_text().as_deref(), Some("ticket-1234"));
        assert!(app.command_palette.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(copy_mode_for_test(&app).is_none());
    }

