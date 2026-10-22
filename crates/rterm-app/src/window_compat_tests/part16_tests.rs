    #[test]
    fn window_app_dispatches_palette_input_selector_compact_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("inputselector=title=Pick choices=yes=Yes".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::InputSelector(WindowInputSelectorOptions {
                title: "Pick".to_owned(),
                choices: vec![WindowInputSelectorChoice {
                    label: "Yes".to_owned(),
                    id: Some("yes".to_owned()),
                }],
                alphabet: None,
                description: None,
                fuzzy_description: None,
                fuzzy: false,
                action: None,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_mixed_case_equals_field_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "InputSelector Title=Pick Choices=yes=Yes ; no=No Alphabet=ab Description=Choose Fuzzy_Description=Filter Fuzzy=false"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::InputSelector(WindowInputSelectorOptions {
                title: "Pick".to_owned(),
                choices: vec![
                    WindowInputSelectorChoice {
                        label: "Yes".to_owned(),
                        id: Some("yes".to_owned()),
                    },
                    WindowInputSelectorChoice {
                        label: "No".to_owned(),
                        id: Some("no".to_owned()),
                    },
                ],
                alphabet: Some("ab".to_owned()),
                description: Some("Choose".to_owned()),
                fuzzy_description: Some("Filter".to_owned()),
                fuzzy: false,
                action: None,
            })]
        );
    }

    #[test]
    fn window_app_rejects_palette_input_selector_query_with_duplicate_fuzzy_field() {
        assert!(
            input_selector_options_from_query(
                "input selector title Pick choices yes=Yes ; no=No fuzzy true fuzzy false"
            )
            .is_none()
        );
    }

    #[test]
    fn window_app_input_selector_default_mode_shortcut_selects_choice() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.input_selector_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        assert!(app.command_palette_execute(WindowCommand::InputSelector(
            WindowInputSelectorOptions {
                title: "Pick Reply".to_owned(),
                choices: vec![
                    WindowInputSelectorChoice {
                        label: "No thanks".to_owned(),
                        id: Some("decline".to_owned()),
                    },
                    WindowInputSelectorChoice {
                        label: "LGTM".to_owned(),
                        id: Some("lgtm".to_owned()),
                    },
                ],
                alphabet: Some("ab".to_owned()),
                description: Some("Choose:".to_owned()),
                fuzzy_description: None,
                fuzzy: false,
                action: None,
            },
        )));

        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Pick Reply: Choose: [1 / 2] No thanks"
        );
        assert!(
            app.handle_input_selector_key(&Key::Character("b".into()), ModifiersState::empty())
        );

        assert!(app.input_selector.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeInputSelector {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                id: Some("lgtm".to_owned()),
                label: Some("LGTM".to_owned()),
            }]
        );
    }

    #[test]
    fn window_input_selector_uses_modern_selected_surface_by_default() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(48, 8));

        assert!(app.command_palette_execute(WindowCommand::InputSelector(
            WindowInputSelectorOptions {
                title: "Pick Reply".to_owned(),
                choices: vec![
                    WindowInputSelectorChoice {
                        label: "No thanks".to_owned(),
                        id: Some("decline".to_owned()),
                    },
                    WindowInputSelectorChoice {
                        label: "LGTM".to_owned(),
                        id: Some("lgtm".to_owned()),
                    },
                ],
                alphabet: Some("ab".to_owned()),
                ..WindowInputSelectorOptions::default()
            },
        )));

        let snapshot = app.render_snapshot();
        let shortcut = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0)
            .expect("expected input-selector shortcut label");
        let selected = snapshot_cell(&snapshot, TAB_BAR_ROWS, 2)
            .expect("expected selected input-selector row");
        assert_eq!(shortcut.foreground, DEFAULT_UI_ACCENT_FOREGROUND);
        assert_eq!(shortcut.background, DEFAULT_UI_ACCENT_BACKGROUND);
        assert_eq!(selected.foreground, DEFAULT_UI_ACCENT_FOREGROUND);
        assert_eq!(selected.background, DEFAULT_UI_ACCENT_BACKGROUND);
    }

    #[test]
    fn window_app_input_selector_default_mode_number_selects_choice() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.input_selector_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        assert!(app.command_palette_execute(WindowCommand::InputSelector(
            WindowInputSelectorOptions {
                title: "Pick Reply".to_owned(),
                choices: vec![
                    WindowInputSelectorChoice {
                        label: "No thanks".to_owned(),
                        id: Some("decline".to_owned()),
                    },
                    WindowInputSelectorChoice {
                        label: "LGTM".to_owned(),
                        id: Some("lgtm".to_owned()),
                    },
                ],
                alphabet: Some("ab".to_owned()),
                description: Some("Choose:".to_owned()),
                fuzzy_description: None,
                fuzzy: false,
                action: None,
            },
        )));

        assert!(
            app.handle_input_selector_key(&Key::Character("2".into()), ModifiersState::empty())
        );

        assert!(app.input_selector.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeInputSelector {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                id: Some("lgtm".to_owned()),
                label: Some("LGTM".to_owned()),
            }]
        );
    }

    #[test]
    fn window_app_input_selector_left_click_selects_row() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.input_selector_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        assert!(app.command_palette_execute(WindowCommand::InputSelector(
            WindowInputSelectorOptions {
                title: "Pick Reply".to_owned(),
                choices: vec![
                    WindowInputSelectorChoice {
                        label: "No thanks".to_owned(),
                        id: Some("decline".to_owned()),
                    },
                    WindowInputSelectorChoice {
                        label: "LGTM".to_owned(),
                        id: Some("lgtm".to_owned()),
                    },
                ],
                alphabet: Some("ab".to_owned()),
                description: Some("Choose:".to_owned()),
                fuzzy_description: None,
                fuzzy: false,
                action: None,
            },
        )));

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH),
            f64::from(tab_bar_pixel_height() + CELL_HEIGHT),
        ))
        .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert!(app.input_selector.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeInputSelector {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                id: Some("lgtm".to_owned()),
                label: Some("LGTM".to_owned()),
            }]
        );
    }

    #[test]
    fn window_app_input_selector_default_mode_ignores_non_alphabet_text() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.input_selector_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        assert!(app.command_palette_execute(WindowCommand::InputSelector(
            WindowInputSelectorOptions {
                title: "Pick Reply".to_owned(),
                choices: vec![
                    WindowInputSelectorChoice {
                        label: "No thanks".to_owned(),
                        id: Some("decline".to_owned()),
                    },
                    WindowInputSelectorChoice {
                        label: "LGTM".to_owned(),
                        id: Some("lgtm".to_owned()),
                    },
                ],
                alphabet: Some("ab".to_owned()),
                description: Some("Choose:".to_owned()),
                fuzzy_description: None,
                fuzzy: false,
                action: None,
            },
        )));

        assert!(
            app.handle_input_selector_key(&Key::Character("x".into()), ModifiersState::empty())
        );
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Pick Reply: Choose: [1 / 2] No thanks"
        );

        assert!(
            app.handle_input_selector_key(&Key::Named(NamedKey::Enter), ModifiersState::empty())
        );
        assert!(app.input_selector.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeInputSelector {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                id: Some("decline".to_owned()),
                label: Some("No thanks".to_owned()),
            }]
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_query() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.input_selector_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "input selector title Pick Reply choices decline=No thanks ; lgtm=LGTM alphabet ab description Choose: fuzzy false"
                .to_owned(),
        );
        let command = WindowCommand::InputSelector(WindowInputSelectorOptions {
            title: "Pick Reply".to_owned(),
            choices: vec![
                WindowInputSelectorChoice {
                    label: "No thanks".to_owned(),
                    id: Some("decline".to_owned()),
                },
                WindowInputSelectorChoice {
                    label: "LGTM".to_owned(),
                    id: Some("lgtm".to_owned()),
                },
            ],
            alphabet: Some("ab".to_owned()),
            description: Some("Choose:".to_owned()),
            fuzzy_description: None,
            fuzzy: false,
            action: None,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Pick Reply: Choose: [1 / 2] No thanks"
        );

        assert!(
            app.handle_input_selector_key(&Key::Character("b".into()), ModifiersState::empty())
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeInputSelector {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                id: Some("lgtm".to_owned()),
                label: Some("LGTM".to_owned()),
            }]
        );
    }

    #[test]
    fn window_app_input_selector_fuzzy_mode_filters_and_accepts_choice() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.input_selector_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        assert!(app.command_palette_execute(WindowCommand::InputSelector(
            WindowInputSelectorOptions {
                title: "Choose Workspace".to_owned(),
                choices: vec![
                    WindowInputSelectorChoice {
                        label: "Home".to_owned(),
                        id: Some("/home/me".to_owned()),
                    },
                    WindowInputSelectorChoice {
                        label: "Work".to_owned(),
                        id: Some("/home/me/work".to_owned()),
                    },
                    WindowInputSelectorChoice {
                        label: "Personal".to_owned(),
                        id: Some("/home/me/personal".to_owned()),
                    },
                ],
                alphabet: None,
                description: None,
                fuzzy_description: Some("Fuzzy find:".to_owned()),
                fuzzy: true,
                action: None,
            },
        )));

        assert!(
            app.handle_input_selector_key(&Key::Character("w".into()), ModifiersState::empty())
        );
        assert!(
            app.handle_input_selector_key(&Key::Character("o".into()), ModifiersState::empty())
        );
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Choose Workspace: Fuzzy find: \"wo\" [1 / 1] Work"
        );
        assert!(
            app.handle_input_selector_key(&Key::Named(NamedKey::Enter), ModifiersState::empty())
        );

        assert!(app.input_selector.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeInputSelector {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                id: Some("/home/me/work".to_owned()),
                label: Some("Work".to_owned()),
            }]
        );
    }

    #[test]
    fn window_app_input_selector_cancel_emits_none() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.input_selector_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        assert!(app.command_palette_execute(WindowCommand::InputSelector(
            WindowInputSelectorOptions {
                title: "Pick".to_owned(),
                choices: vec![WindowInputSelectorChoice {
                    label: "One".to_owned(),
                    id: None,
                }],
                alphabet: None,
                description: None,
                fuzzy_description: None,
                fuzzy: false,
                action: None,
            },
        )));
        assert!(
            app.handle_input_selector_key(&Key::Named(NamedKey::Escape), ModifiersState::empty())
        );

        assert!(app.input_selector.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeInputSelector {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                id: None,
                label: None,
            }]
        );
    }

    #[test]
    fn window_app_dispatches_palette_activate_command_palette_command() {
        let mut app = NativeWindowApp::new(None);
        app.enter_command_palette_mode();
        app.command_palette_set_query("activate command palette".to_owned());
        let command = app
            .command_palette_filtered_commands()
            .into_iter()
            .find(|command| command.label() == "Activate Command Palette")
            .expect("expected activate command palette command");

        app.command_palette_execute(command);

        let palette = app
            .command_palette
            .as_ref()
            .expect("expected command palette to reopen");
        assert!(palette.query.is_empty());
        assert_eq!(
            app.effective_window_title(),
            format!(
                "R-Term [workspace:1 tab:1 pane:1] - Command Palette: [1 / {}] New Tab",
                WINDOW_COMMANDS.len()
            )
        );
    }

    #[test]
    fn window_app_dispatches_palette_reload_configuration_command() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.config_reloaded_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(*event);
            true
        });
        let active_pane = app.app_shell.active_pane_id();
        app.enter_command_palette_mode();
        app.command_palette_set_query("reload configuration".to_owned());
        let command = app
            .command_palette_filtered_commands()
            .into_iter()
            .find(|command| command.label() == "Reload Configuration")
            .expect("expected reload configuration command");

        assert!(app.command_palette_execute(command));

        assert!(app.command_palette.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeWindowConfigReloaded {
                window_id: rterm_core::WindowId::new(1),
                pane: active_pane,
            }]
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_config_reloaded_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('window-config-reloaded', function(window, pane)
              window:set_right_status('CONFIG-RELOADED')
            end)
            "#,
        )
        .expect("expected static WezTerm config-reloaded event status setter");
        app.set_config_overrides(overrides);
        app.right_status.clear();

        assert!(app.command_palette_execute(WindowCommand::ReloadConfiguration));

        assert_eq!(app.right_status, "CONFIG-RELOADED");
    }

    #[test]
    fn window_app_dispatches_palette_reload_configuration_wezterm_action_table_wrapper_query() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.config_reloaded_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(*event);
            true
        });
        let active_pane = app.app_shell.active_pane_id();
        app.enter_command_palette_mode();
        app.command_palette_set_query("wezterm.action { ReloadConfiguration = {} }".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ReloadConfiguration]
        );
        assert!(app.command_palette_execute(WindowCommand::ReloadConfiguration));

        assert!(app.command_palette.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeWindowConfigReloaded {
                window_id: rterm_core::WindowId::new(1),
                pane: active_pane,
            }]
        );
    }

    #[test]
    fn window_app_dispatches_palette_reload_configuration_wezterm_action_table_wrapper_comment_empty_payload_query()
     {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.config_reloaded_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(*event);
            true
        });
        let active_pane = app.app_shell.active_pane_id();
        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action { ReloadConfiguration = { -- reload options\n } }".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ReloadConfiguration]
        );
        assert!(app.command_palette_execute(WindowCommand::ReloadConfiguration));

        assert!(app.command_palette.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeWindowConfigReloaded {
                window_id: rterm_core::WindowId::new(1),
                pane: active_pane,
            }]
        );
    }

    #[test]
    fn window_app_dispatches_palette_reload_configuration_wezterm_action_function_comment_empty_payload_query()
     {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.config_reloaded_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(*event);
            true
        });
        let active_pane = app.app_shell.active_pane_id();
        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.ReloadConfiguration({ -- reload options\n })".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ReloadConfiguration]
        );
        assert!(app.command_palette_execute(WindowCommand::ReloadConfiguration));

        assert!(app.command_palette.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeWindowConfigReloaded {
                window_id: rterm_core::WindowId::new(1),
                pane: active_pane,
            }]
        );
    }

    #[test]
    fn window_app_dispatches_palette_reload_configuration_wezterm_action_function_comment_empty_args_query()
     {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.config_reloaded_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(*event);
            true
        });
        let active_pane = app.app_shell.active_pane_id();
        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.ReloadConfiguration( -- reload options\n )".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ReloadConfiguration]
        );
        assert!(app.command_palette_execute(WindowCommand::ReloadConfiguration));

        assert!(app.command_palette.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeWindowConfigReloaded {
                window_id: rterm_core::WindowId::new(1),
                pane: active_pane,
            }]
        );
    }

    #[test]
    fn window_app_dispatches_palette_reload_configuration_wezterm_action_function_trailing_comment_query()
     {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.config_reloaded_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(*event);
            true
        });
        let active_pane = app.app_shell.active_pane_id();
        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.ReloadConfiguration() -- reload options".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::ReloadConfiguration]
        );
        assert!(app.command_palette_execute(WindowCommand::ReloadConfiguration));

        assert!(app.command_palette.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeWindowConfigReloaded {
                window_id: rterm_core::WindowId::new(1),
                pane: active_pane,
            }]
        );
    }

    #[test]
    fn window_app_dispatches_palette_action_name_queries() {
        for (query, expected) in [
            ("restartpane", WindowCommand::RestartPane),
            ("reloadconfiguration", WindowCommand::ReloadConfiguration),
            (
                "activatecommandpalette",
                WindowCommand::ActivateCommandPalette,
            ),
            ("togglefullscreen", WindowCommand::ToggleFullScreen),
            ("startwindowdrag", WindowCommand::StartWindowDrag),
            ("togglealwaysontop", WindowCommand::ToggleAlwaysOnTop),
            ("togglealwaysonbottom", WindowCommand::ToggleAlwaysOnBottom),
            ("hideapplication", WindowCommand::HideApplication),
            ("quitapplication", WindowCommand::QuitApplication),
            ("decreasefontsize", WindowCommand::DecreaseFontSize),
            ("increasefontsize", WindowCommand::IncreaseFontSize),
            ("resetfontsize", WindowCommand::ResetFontSize),
            (
                "resetfontandwindowsize",
                WindowCommand::ResetFontAndWindowSize,
            ),
            ("showdebugoverlay", WindowCommand::ShowDebugOverlay),
            ("resetterminal", WindowCommand::ResetTerminal),
            ("showtabnavigator", WindowCommand::ShowTabNavigator),
            (
                "activatepanedirection left",
                WindowCommand::ActivatePaneDirection(rterm_core::app_shell::PaneDirection::Left),
            ),
            (
                "adjustpanesize left 4",
                WindowCommand::AdjustPaneSize {
                    direction: ResizeDirection::Left,
                    amount: 4,
                },
            ),
            (
                "adjustpanesize direction=left amount=4",
                WindowCommand::AdjustPaneSize {
                    direction: ResizeDirection::Left,
                    amount: 4,
                },
            ),
            (
                "adjustpanesize Direction left Amount 4",
                WindowCommand::AdjustPaneSize {
                    direction: ResizeDirection::Left,
                    amount: 4,
                },
            ),
            (
                "rotatepanes clockwise",
                WindowCommand::RotatePanes(rterm_core::app_shell::PaneRotationDirection::Clockwise),
            ),
        ] {
            let mut app = NativeWindowApp::new(None);
            app.enter_command_palette_mode();
            app.command_palette_set_query(query.to_owned());

            assert_eq!(app.command_palette_filtered_commands(), vec![expected]);
        }
    }

    #[test]
    fn parses_activate_copy_mode_action_name_query() {
        for (query, expected) in [
            ("activatecopymode", WindowCommand::ActivateCopyMode),
            ("entercopymode", WindowCommand::EnterCopyMode),
        ] {
            assert_eq!(
                command_palette_basic_structured_query_command(query),
                Some(expected)
            );
        }
    }

    #[test]
    fn parses_noop_action_name_queries() {
        for (query, expected) in [
            ("nop", WindowCommand::Nop),
            ("NoP", WindowCommand::Nop),
            (
                "disabledefaultassignment",
                WindowCommand::DisableDefaultAssignment,
            ),
        ] {
            assert_eq!(
                command_palette_basic_structured_query_command(query),
                Some(expected)
            );
        }
    }

    #[test]
    fn parses_selection_action_name_queries() {
        for (query, expected) in [
            ("clearselection", WindowCommand::ClearSelection),
            ("completeselection", WindowCommand::CompleteSelection),
            (
                "openlinkatmousecursor",
                WindowCommand::OpenLinkAtMouseCursor,
            ),
            (
                "completeselectionoropenlinkatmousecursor",
                WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursor,
            ),
        ] {
            assert_eq!(
                command_palette_basic_structured_query_command(query),
                Some(expected)
            );
        }
    }

    #[test]
    fn parses_clipboard_action_name_queries() {
        for (query, expected) in [
            ("copytoclipboard", WindowCommand::CopyToClipboard),
            (
                "copytoprimaryselection",
                WindowCommand::CopyToPrimarySelection,
            ),
            (
                "copytoclipboardandprimaryselection",
                WindowCommand::CopyToClipboardAndPrimarySelection,
            ),
            ("pastefromclipboard", WindowCommand::PasteFromClipboard),
            (
                "pastefromprimaryselection",
                WindowCommand::PasteFromPrimarySelection,
            ),
        ] {
            assert_eq!(
                command_palette_basic_structured_query_command(query),
                Some(expected)
            );
        }
    }

    #[test]
    fn parses_legacy_io_action_name_queries() {
        for (query, expected) in [
            ("copy", WindowCommand::Copy),
            ("paste", WindowCommand::Paste),
            (
                "pasteprimaryselection",
                WindowCommand::PastePrimarySelection,
            ),
            (
                "clearscrollbackandviewport",
                WindowCommand::ClearScrollbackAndViewport,
            ),
        ] {
            assert_eq!(
                command_palette_basic_structured_query_command(query),
                Some(expected)
            );
        }
    }

    #[test]
    fn parses_scrollback_action_name_queries() {
        for (query, expected) in [
            ("scrolltotop", WindowCommand::ScrollToTop),
            ("scrolltobottom", WindowCommand::ScrollToBottom),
            ("scrollpageup", WindowCommand::ScrollPageUp),
            ("scrollpagedown", WindowCommand::ScrollPageDown),
            ("scrolllineup", WindowCommand::ScrollLineUp),
            ("scrolllinedown", WindowCommand::ScrollLineDown),
            (
                "scrollbycurrenteventwheeldelta",
                WindowCommand::ScrollByCurrentEventWheelDelta,
            ),
            (
                "scrolltopreviousprompt",
                WindowCommand::ScrollToPreviousPrompt,
            ),
            ("scrolltonextprompt", WindowCommand::ScrollToNextPrompt),
        ] {
            assert_eq!(
                command_palette_basic_structured_query_command(query),
                Some(expected)
            );
        }
    }

    #[test]
    fn parses_pane_navigation_action_name_queries() {
        for (query, expected) in [
            ("activatepaneleft", WindowCommand::ActivatePaneLeft),
            ("activatepaneright", WindowCommand::ActivatePaneRight),
            ("activatepaneup", WindowCommand::ActivatePaneUp),
            ("activatepanedown", WindowCommand::ActivatePaneDown),
            ("nextpane", WindowCommand::NextPane),
            ("previouspane", WindowCommand::PreviousPane),
            ("activatepane1", WindowCommand::ActivatePane1),
            ("activatepane2", WindowCommand::ActivatePane2),
            ("activatepane3", WindowCommand::ActivatePane3),
            ("activatepane4", WindowCommand::ActivatePane4),
            ("activatepane5", WindowCommand::ActivatePane5),
            ("activatepane6", WindowCommand::ActivatePane6),
            ("activatepane7", WindowCommand::ActivatePane7),
            ("activatepane8", WindowCommand::ActivatePane8),
        ] {
            assert_eq!(
                command_palette_basic_structured_query_command(query),
                Some(expected)
            );
        }
    }

    #[test]
    fn parses_tab_navigation_action_name_queries() {
        for (query, expected) in [
            ("activatelasttab", WindowCommand::ActivateLastTab),
            ("activatetab1", WindowCommand::ActivateTab1),
            ("activatetab2", WindowCommand::ActivateTab2),
            ("activatetab3", WindowCommand::ActivateTab3),
            ("activatetab4", WindowCommand::ActivateTab4),
            ("activatetab5", WindowCommand::ActivateTab5),
            ("activatetab6", WindowCommand::ActivateTab6),
            ("activatetab7", WindowCommand::ActivateTab7),
            ("activatetab8", WindowCommand::ActivateTab8),
            ("activatetab9", WindowCommand::ActivateTab9),
            ("nexttab", WindowCommand::NextTab),
            ("previoustab", WindowCommand::PreviousTab),
            ("nexttabnowrap", WindowCommand::NextTabNoWrap),
            ("previoustabnowrap", WindowCommand::PreviousTabNoWrap),
            ("movetabrelativeleft", WindowCommand::MoveTabRelativeLeft),
            ("movetabrelativeright", WindowCommand::MoveTabRelativeRight),
            ("movetabto1", WindowCommand::MoveTabTo1),
            ("movetabto2", WindowCommand::MoveTabTo2),
            ("movetabto3", WindowCommand::MoveTabTo3),
            ("movetabto4", WindowCommand::MoveTabTo4),
            ("movetabto5", WindowCommand::MoveTabTo5),
            ("movetabto6", WindowCommand::MoveTabTo6),
            ("movetabto7", WindowCommand::MoveTabTo7),
            ("movetabto8", WindowCommand::MoveTabTo8),
        ] {
            assert_eq!(
                command_palette_basic_structured_query_command(query),
                Some(expected)
            );
        }
    }

    #[test]
    fn parses_pane_zoom_action_name_queries() {
        for (query, expected) in [
            ("togglepanezoomstate", WindowCommand::TogglePaneZoomState),
            ("togglepanezoom", WindowCommand::TogglePaneZoom),
            ("zoompane", WindowCommand::ZoomPane),
            ("unzoompane", WindowCommand::UnzoomPane),
        ] {
            assert_eq!(
                command_palette_basic_structured_query_command(query),
                Some(expected)
            );
        }
    }

    #[test]
    fn parses_window_and_overlay_action_name_queries() {
        for (query, expected) in [
            ("reloadconfiguration", WindowCommand::ReloadConfiguration),
            (
                "activatecommandpalette",
                WindowCommand::ActivateCommandPalette,
            ),
            ("togglefullscreen", WindowCommand::ToggleFullScreen),
            ("startwindowdrag", WindowCommand::StartWindowDrag),
            ("togglealwaysontop", WindowCommand::ToggleAlwaysOnTop),
            ("togglealwaysonbottom", WindowCommand::ToggleAlwaysOnBottom),
            ("show", WindowCommand::Show),
            ("hide", WindowCommand::Hide),
            ("resetterminal", WindowCommand::ResetTerminal),
            ("showtabnavigator", WindowCommand::ShowTabNavigator),
            ("showlauncher", WindowCommand::ShowLauncher),
            ("charselect", WindowCommand::CharSelect),
            ("enterquickselect", WindowCommand::EnterQuickSelect),
            ("enterpaneselect", WindowCommand::EnterPaneSelect),
        ] {
            assert_eq!(
                command_palette_basic_structured_query_command(query),
                Some(expected)
            );
        }
    }

    #[test]
    fn parses_wezterm_zero_arg_function_action_queries() {
        for (query, expected) in [
            (
                "wezterm.action.ActivateLastTab()",
                WindowCommand::ActivateLastTab,
            ),
            (
                "wezterm.action.ShowTabNavigator()",
                WindowCommand::ShowTabNavigator,
            ),
            (
                "wezterm.action.ToggleFullScreen()",
                WindowCommand::ToggleFullScreen,
            ),
            (
                "wezterm.action({ ToggleFullScreen = { } })",
                WindowCommand::ToggleFullScreen,
            ),
            (
                "wezterm.action[\"ToggleFullScreen\"]",
                WindowCommand::ToggleFullScreen,
            ),
        ] {
            assert_eq!(
                super::command_palette_structured_query_command(query),
                Some(expected)
            );
        }
    }

    #[test]
    fn parses_wezterm_parenthesized_table_action_queries() {
        for (query, expected) in [
            (
                "wezterm.action.CloseCurrentPane({ confirm = false })",
                WindowCommand::CloseCurrentPane { confirm: false },
            ),
            (
                "wezterm.action.CloseCurrentTab({ confirm = true })",
                WindowCommand::CloseCurrentTab { confirm: true },
            ),
        ] {
            assert_eq!(
                super::command_palette_structured_query_command(query),
                Some(expected)
            );
        }
    }

    #[test]
    fn parses_close_action_name_queries() {
        for (query, expected) in [
            ("closepane", WindowCommand::ClosePane),
            ("closetab", WindowCommand::CloseTab),
        ] {
            assert_eq!(
                command_palette_basic_structured_query_command(query),
                Some(expected)
            );
        }
    }

    #[test]
    fn parses_spawn_and_split_action_name_queries() {
        for (query, expected) in [
            (
                "spawntab",
                WindowCommand::SpawnTab(WindowSpawnTabDomain::CurrentPaneDomain),
            ),
            ("spawnwindow", WindowCommand::SpawnWindow),
            ("splithorizontal", WindowCommand::SplitHorizontal),
            ("splitvertical", WindowCommand::SplitVertical),
        ] {
            assert_eq!(
                command_palette_basic_structured_query_command(query),
                Some(expected)
            );
        }
    }

    #[test]
    fn parses_pane_select_mode_action_name_queries() {
        for (query, expected) in [
            (
                "enterpaneselectshowpaneids",
                WindowCommand::EnterPaneSelectShowPaneIds,
            ),
            ("enterpaneswap", WindowCommand::EnterPaneSwap),
            (
                "enterpaneswapkeepfocus",
                WindowCommand::EnterPaneSwapKeepFocus,
            ),
            (
                "enterpanemovetonewtab",
                WindowCommand::EnterPaneMoveToNewTab,
            ),
            (
                "enterpanemovetonewwindow",
                WindowCommand::EnterPaneMoveToNewWindow,
            ),
        ] {
            assert_eq!(
                command_palette_basic_structured_query_command(query),
                Some(expected)
            );
        }
    }

    #[test]
    fn window_app_dispatches_palette_io_action_name_queries() {
        for (query, expected) in [
            (
                "clearscrollback scrollback and viewport",
                WindowCommand::ClearScrollback(WindowClearScrollbackMode::ScrollbackAndViewport),
            ),
            (
                "copyto clipboard and primary selection",
                WindowCommand::CopyTo(WindowCopyDestination::ClipboardAndPrimarySelection),
            ),
            (
                "pastefrom primary selection",
                WindowCommand::PasteFrom(WindowPasteSource::PrimarySelection),
            ),
            (
                "closecurrentpane confirm false",
                WindowCommand::CloseCurrentPane { confirm: false },
            ),
            (
                "CloseCurrentPane Confirm=False",
                WindowCommand::CloseCurrentPane { confirm: false },
            ),
            (
                "closecurrenttab confirm true",
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
    fn window_app_reload_configuration_shortcut_dispatches_reload_without_renaming_workspace() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.config_reloaded_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(*event);
            true
        });
        let active_pane = app.app_shell.active_pane_id();

        assert!(app.handle_reload_configuration_shortcut(
            &Key::Character("r".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));

        assert_eq!(app.app_shell.active_workspace().name(), "default");
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeWindowConfigReloaded {
                window_id: rterm_core::WindowId::new(1),
                pane: active_pane,
            }]
        );
    }

    #[test]
    fn window_app_reload_configuration_clears_key_table_stack() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.config_reloaded_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(*event);
            true
        });

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

        assert!(app.command_palette_execute(WindowCommand::ReloadConfiguration));

        assert_eq!(app.active_key_table_for_test(), None);
        assert!(!app.effective_window_title().contains("KeyTable: leader"));
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeWindowConfigReloaded {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
            }]
        );
    }

    #[test]
    fn window_app_toggle_full_screen_shortcut_toggles_window_state() {
        let mut app = NativeWindowApp::new(None);

        assert!(!app.full_screen_for_test());
        assert!(
            app.handle_toggle_full_screen_shortcut(
                &Key::Named(NamedKey::Enter),
                ModifiersState::ALT
            )
        );
        assert!(app.full_screen_for_test());
        assert!(
            app.handle_toggle_full_screen_shortcut(
                &Key::Named(NamedKey::Enter),
                ModifiersState::ALT
            )
        );
        assert!(!app.full_screen_for_test());
    }

    #[test]
    fn window_app_toggle_full_screen_shortcut_dispatches_resize_event() {
        let resizes = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&resizes);
        let mut app = NativeWindowApp::new(None);
        app.resize_handler = Box::new(move |resize| {
            recorded.lock().unwrap().push(*resize);
            true
        });
        let active_pane = app.app_shell.active_pane_id();

        assert!(
            app.handle_toggle_full_screen_shortcut(
                &Key::Named(NamedKey::Enter),
                ModifiersState::ALT
            )
        );
        assert!(
            app.handle_toggle_full_screen_shortcut(
                &Key::Named(NamedKey::Enter),
                ModifiersState::ALT
            )
        );

        assert_eq!(
            resizes.lock().unwrap().as_slice(),
            [
                NativeWindowResize {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                    pixel_width: FRAME_WIDTH,
                    pixel_height: FRAME_HEIGHT,
                    terminal_size: app.runtime.terminal().grid().size(),
                    is_full_screen: true,
                },
                NativeWindowResize {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                    pixel_width: FRAME_WIDTH,
                    pixel_height: FRAME_HEIGHT,
                    terminal_size: app.runtime.terminal().grid().size(),
                    is_full_screen: false,
                },
            ]
        );
    }

    #[test]
    fn window_app_disable_default_assignment_suppresses_scrollback_shortcut() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc").unwrap();
        app.set_config_overrides(native_config_snapshot! {
            key_assignments: Some(vec![NativeUserKeyAssignment {
                keys: "SHIFT+PAGEUP".to_owned(),
                command: WindowCommand::DisableDefaultAssignment,
            }]),
            ..NativeConfigSnapshot::default()
        });

        assert!(
            !app.handle_scrollback_shortcut(&Key::Named(NamedKey::PageUp), ModifiersState::SHIFT)
        );
        assert_eq!(app.current_scrollback_offset(), 0);
    }

    #[test]
    fn window_app_disable_default_key_bindings_suppresses_scrollback_shortcuts() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc").unwrap();
        app.set_config_overrides(native_config_snapshot! {
            disable_default_key_bindings: Some(true),
            ..NativeConfigSnapshot::default()
        });

        assert!(
            !app.handle_scrollback_shortcut(&Key::Named(NamedKey::PageUp), ModifiersState::SHIFT)
        );
        assert_eq!(app.current_scrollback_offset(), 0);
    }

    #[test]
    fn window_app_disable_default_assignment_suppresses_window_shortcut() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            key_assignments: Some(vec![NativeUserKeyAssignment {
                keys: "ALT+ENTER".to_owned(),
                command: WindowCommand::DisableDefaultAssignment,
            }]),
            ..NativeConfigSnapshot::default()
        });

        assert!(
            !app.handle_toggle_full_screen_shortcut(
                &Key::Named(NamedKey::Enter),
                ModifiersState::ALT,
            )
        );
        assert!(!app.full_screen_for_test());
    }

    #[test]
    fn window_app_disable_default_key_bindings_suppresses_window_shortcuts() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            disable_default_key_bindings: Some(true),
            ..NativeConfigSnapshot::default()
        });

        assert!(
            !app.handle_toggle_full_screen_shortcut(
                &Key::Named(NamedKey::Enter),
                ModifiersState::ALT,
            )
        );
        assert!(!app.full_screen_for_test());
    }

    #[test]
    fn window_app_hide_shortcut_requests_hide() {
        let mut app = NativeWindowApp::new(None);

        assert!(!app.window_hide_requested_for_test());
        assert!(app.handle_hide_shortcut(&Key::Character("m".into()), ModifiersState::SUPER));
        assert!(app.window_hide_requested_for_test());
        assert!(!app.handle_hide_shortcut(
            &Key::Character("m".into()),
            ModifiersState::SUPER | ModifiersState::SHIFT
        ));
    }

    #[test]
    fn window_app_hide_application_shortcut_requests_application_hide() {
        let mut app = NativeWindowApp::new(None);

        assert!(!app.take_application_hide_request());
        #[cfg(target_os = "macos")]
        assert!(
            app.handle_application_hide_shortcut(
                &Key::Character("h".into()),
                ModifiersState::SUPER
            )
        );
        #[cfg(not(target_os = "macos"))]
        assert!(
            !app.handle_application_hide_shortcut(
                &Key::Character("h".into()),
                ModifiersState::SUPER
            )
        );
        #[cfg(target_os = "macos")]
        {
            assert!(app.take_application_hide_request());
            assert!(!app.take_application_hide_request());
        }
        #[cfg(not(target_os = "macos"))]
        assert!(!app.take_application_hide_request());
        assert!(!app.handle_application_hide_shortcut(
            &Key::Character("h".into()),
            ModifiersState::SUPER | ModifiersState::SHIFT
        ));
    }

    #[test]
    fn window_app_font_size_shortcuts_adjust_logical_font_scale() {
        let mut app = NativeWindowApp::new(None);

        assert!((app.font_size_scale_for_test() - 1.0).abs() < f64::EPSILON);
        assert!(
            app.handle_font_size_shortcut(&Key::Character("=".into()), ModifiersState::CONTROL)
        );
        assert!((app.font_size_scale_for_test() - 1.1).abs() < f64::EPSILON);
        assert!(app.handle_font_size_shortcut(&Key::Character("-".into()), ModifiersState::SUPER));
        assert!((app.font_size_scale_for_test() - 1.0).abs() < f64::EPSILON);
        assert!(app.handle_font_size_shortcut(&Key::Character("=".into()), ModifiersState::SUPER));
        assert!(
            app.handle_font_size_shortcut(&Key::Character("0".into()), ModifiersState::CONTROL)
        );
        assert!((app.font_size_scale_for_test() - 1.0).abs() < f64::EPSILON);
        assert!(!app.handle_font_size_shortcut(
            &Key::Character("=".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));
    }

    #[test]
    fn window_app_show_debug_overlay_shortcut_opens_debug_overlay() {
        let mut app = NativeWindowApp::new(None);

        assert!(!app.debug_overlay_active_for_test());
        assert!(app.handle_show_debug_overlay_shortcut(
            &Key::Character("l".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));
        assert!(app.debug_overlay_active_for_test());
        assert!(!app.handle_show_debug_overlay_shortcut(
            &Key::Character("l".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT
        ));
    }

    #[test]
    fn window_app_default_window_shortcuts_honor_physical_key_map_preference() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            key_map_preference: Some(NativeKeyMapPreference::Physical),
            ..NativeConfigSnapshot::default()
        });
        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;

        app.handle_keyboard_input_event(
            &Key::Character("l".into()),
            PhysicalKey::Code(WinitKeyCode::KeyX),
            Some("l"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(!app.debug_overlay_active_for_test());

        app.handle_keyboard_input_event(
            &Key::Character("x".into()),
            PhysicalKey::Code(WinitKeyCode::KeyL),
            Some("x"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(app.debug_overlay_active_for_test());
    }

    #[test]
    fn window_app_char_select_shortcut_opens_char_select_mode() {
        let mut app = NativeWindowApp::new(None);

        assert!(!app.char_select_active_for_test());
        assert!(app.handle_char_select_shortcut(
            &Key::Character("u".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));
        assert!(app.char_select_active_for_test());
        assert!(!app.handle_char_select_shortcut(
            &Key::Character("u".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT
        ));
    }

    #[test]
    fn window_app_swap_backspace_and_delete_switches_pty_input_bytes() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.set_config_overrides(native_config_snapshot! {
            swap_backspace_and_delete: Some(true),
            ..NativeConfigSnapshot::default()
        });

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Backspace),
            PhysicalKey::Code(WinitKeyCode::Backspace),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Delete),
            PhysicalKey::Code(WinitKeyCode::Delete),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b[3~\x7f");
    }

    #[test]
    fn window_app_repeated_backspace_continues_writing_to_pty() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        for kind in [KittyKeyEventKind::Press, KittyKeyEventKind::Repeat] {
            app.handle_keyboard_input_event(
                &Key::Named(NamedKey::Backspace),
                PhysicalKey::Code(WinitKeyCode::Backspace),
                None,
                ElementState::Pressed,
                kind,
            )
            .unwrap();
        }

        assert_eq!(written.lock().unwrap().as_slice(), b"\x7f\x7f");
    }

    #[test]
    fn window_app_enable_csi_u_key_encoding_switches_modified_ascii_input() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.set_config_overrides(native_config_snapshot! {
            enable_csi_u_key_encoding: Some(true),
            ..NativeConfigSnapshot::default()
        });

        app.handle_keyboard_input_event(
            &Key::Character("A".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b[97;6u");
    }

    #[test]
    fn window_app_default_csi_u_key_encoding_keeps_legacy_modified_ascii_input() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;

        app.handle_keyboard_input_event(
            &Key::Character("A".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"\x01");
    }

    #[test]
    fn window_app_ignores_kitty_keyboard_protocol_sequences_by_default() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.handle_pty_output(b"\x1b[=1u").unwrap();
        app.modifiers = ModifiersState::CONTROL;

        app.handle_keyboard_input_event(
            &Key::Character("i".into()),
            PhysicalKey::Code(WinitKeyCode::KeyI),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"\t");
    }

    #[test]
    fn window_app_enable_kitty_keyboard_honors_protocol_sequences() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.set_config_overrides(native_config_snapshot! {
            enable_kitty_keyboard: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[=1u").unwrap();
        app.modifiers = ModifiersState::CONTROL;

        app.handle_keyboard_input_event(
            &Key::Character("i".into()),
            PhysicalKey::Code(WinitKeyCode::KeyI),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b[105;5u");
    }

    #[test]
    fn window_app_default_win32_input_mode_encodes_key_release_events() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.handle_pty_output(b"\x1b[?9001h").unwrap();
        app.handle_keyboard_input_event(
            &Key::Character("a".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            None,
            ElementState::Released,
            KittyKeyEventKind::Release,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b[65;0;0;0;0;1_");
    }

    #[test]
    fn window_app_win32_input_mode_encodes_oem_punctuation_without_physical_key() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.handle_pty_output(b"\x1b[?9001h").unwrap();
        app.modifiers = ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character("+".into()),
            PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
            Some("+"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b[187;0;43;1;16;1_");
    }

    #[test]
    fn window_app_win32_input_mode_encodes_numpad_virtual_keys() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.handle_pty_output(b"\x1b[?9001h").unwrap();
        app.handle_keyboard_input_event(
            &Key::Character("1".into()),
            PhysicalKey::Code(WinitKeyCode::Numpad1),
            Some("1"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        app.handle_keyboard_input_event(
            &Key::Character("+".into()),
            PhysicalKey::Code(WinitKeyCode::NumpadAdd),
            Some("+"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"\x1b[97;0;49;1;0;1_\x1b[107;0;43;1;0;1_"
        );
    }

    #[test]
    fn window_app_win32_input_mode_encodes_extended_function_keys() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.handle_pty_output(b"\x1b[?9001h").unwrap();
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::F13),
            PhysicalKey::Code(WinitKeyCode::F13),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        app.modifiers = ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::F24),
            PhysicalKey::Code(WinitKeyCode::F24),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"\x1b[124;0;0;1;0;1_\x1b[135;0;0;1;16;1_"
        );
    }

    #[test]
    fn window_app_win32_input_mode_encodes_modifier_virtual_keys() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.handle_pty_output(b"\x1b[?9001h").unwrap();
        app.modifiers = ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Shift),
            PhysicalKey::Code(WinitKeyCode::ShiftLeft),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        app.modifiers = ModifiersState::CONTROL;
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Control),
            PhysicalKey::Code(WinitKeyCode::ControlRight),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        app.modifiers = ModifiersState::ALT;
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Alt),
            PhysicalKey::Code(WinitKeyCode::AltRight),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"\x1b[160;0;0;1;16;1_\x1b[163;0;0;1;4;1_\x1b[165;0;0;1;1;1_"
        );
    }

    #[test]
    fn window_app_allow_win32_input_mode_false_ignores_conpty_sequence() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.set_config_overrides(native_config_snapshot! {
            allow_win32_input_mode: Some(false),
            ..NativeConfigSnapshot::default()
        });

        app.handle_pty_output(b"\x1b[?9001h").unwrap();
        app.handle_keyboard_input_event(
            &Key::Character("a".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            None,
            ElementState::Released,
            KittyKeyEventKind::Release,
        )
        .unwrap();

        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_char_select_escape_closes_without_writing_to_pty() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_char_select_mode();
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Escape),
            PhysicalKey::Code(WinitKeyCode::Escape),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.char_select_active_for_test());
        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_char_select_ctrl_g_closes_without_writing_to_pty() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.modifiers = ModifiersState::CONTROL;

        app.enter_char_select_mode();
        app.handle_keyboard_input_event(
            &Key::Character("g".into()),
            PhysicalKey::Code(WinitKeyCode::KeyG),
            Some("g"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.char_select_active_for_test());
        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_char_select_ctrl_u_clears_text_input_without_writing_to_pty() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_char_select_mode();
        app.handle_keyboard_input_event(
            &Key::Character("s".into()),
            PhysicalKey::Code(WinitKeyCode::KeyS),
            Some("s"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        app.handle_keyboard_input_event(
            &Key::Character("m".into()),
            PhysicalKey::Code(WinitKeyCode::KeyM),
            Some("m"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Char Select: SmileysAndEmotion [sm]"
        );

        app.modifiers = ModifiersState::CONTROL;
        app.handle_keyboard_input_event(
            &Key::Character("u".into()),
            PhysicalKey::Code(WinitKeyCode::KeyU),
            Some("u"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(app.char_select_active_for_test());
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Char Select: SmileysAndEmotion"
        );
        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_char_select_default_group_renders_categorized_candidates() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            copy_on_select: false,
            ..WindowCharSelectOptions::default()
        });

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.starts_with("> 😀 U+1F600"),
            "first terminal row was {first_terminal_row:?}"
        );
        let selected = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0)
            .expect("expected selected char-select row");
        assert_eq!(selected.foreground, DEFAULT_UI_ACCENT_FOREGROUND);
        assert_eq!(selected.background, DEFAULT_UI_ACCENT_BACKGROUND);

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Enter),
            PhysicalKey::Code(WinitKeyCode::Enter),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.char_select_active_for_test());
        assert_eq!(written.lock().unwrap().as_slice(), "\u{1f600}".as_bytes());
    }

    #[test]
    fn window_app_applies_wezterm_char_select_colors() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(48, 8));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.char_select_bg_color = 'rgba(7,8,9,0.5)'
            config.char_select_fg_color = 'rgba(10,11,12,0.5)'

            return config
            "##,
        )
        .expect("expected WezTerm char select color config");
        app.set_config_overrides(overrides);

        app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            copy_on_select: false,
            ..WindowCharSelectOptions::default()
        });
        let snapshot = app.render_snapshot();
        let second_row =
            snapshot_cell(&snapshot, TAB_BAR_ROWS + 1, 0).expect("expected second char select row");

        assert_eq!(second_row.background, Color::Rgba(7, 8, 9, 127));
        assert_eq!(second_row.foreground, Color::Rgba(10, 11, 12, 127));
    }

    #[test]
    fn window_app_char_select_category_navigation_scrolls_beyond_visible_rows() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            copy_on_select: false,
            ..WindowCharSelectOptions::default()
        });

        for _ in 0..5 {
            app.handle_keyboard_input_event(
                &Key::Named(NamedKey::ArrowDown),
                PhysicalKey::Code(WinitKeyCode::ArrowDown),
                None,
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
        }

        let snapshot = app.render_snapshot();
        let last_visible_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS + 4, TERMINAL_COLUMNS);

        assert!(
            last_visible_row.starts_with("> 😅 U+1F605"),
            "last visible row was {last_visible_row:?}"
        );

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Enter),
            PhysicalKey::Code(WinitKeyCode::Enter),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.char_select_active_for_test());
        assert_eq!(written.lock().unwrap().as_slice(), "\u{1f605}".as_bytes());
    }

    #[test]
    fn window_app_char_select_nerd_fonts_group_renders_private_use_candidates() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            copy_on_select: false,
            group: Some("NerdFonts".to_owned()),
            ..WindowCharSelectOptions::default()
        });

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.starts_with("> \u{f09b} U+F09B NF-FA-GITHUB"),
            "first terminal row was {first_terminal_row:?}"
        );

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Enter),
            PhysicalKey::Code(WinitKeyCode::Enter),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.char_select_active_for_test());
        assert_eq!(written.lock().unwrap().as_slice(), "\u{f09b}".as_bytes());
    }

    #[test]
    fn window_app_char_select_fuzzy_search_finds_nerd_font_names() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            copy_on_select: false,
            group: Some("NerdFonts".to_owned()),
            ..WindowCharSelectOptions::default()
        });
        for (key, physical) in [
            ("g", WinitKeyCode::KeyG),
            ("i", WinitKeyCode::KeyI),
            ("t", WinitKeyCode::KeyT),
            ("h", WinitKeyCode::KeyH),
            ("u", WinitKeyCode::KeyU),
            ("b", WinitKeyCode::KeyB),
        ] {
            app.modifiers = ModifiersState::empty();
            app.handle_keyboard_input_event(
                &Key::Character(key.into()),
                PhysicalKey::Code(physical),
                Some(key),
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
        }

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.starts_with("> \u{f09b} U+F09B NF-FA-GITHUB"),
            "first terminal row was {first_terminal_row:?}"
        );

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Enter),
            PhysicalKey::Code(WinitKeyCode::Enter),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.char_select_active_for_test());
        assert_eq!(written.lock().unwrap().as_slice(), "\u{f09b}".as_bytes());
    }

    #[test]
    fn window_app_char_select_hex_codepoint_finds_nerd_font_private_use_glyph() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            copy_on_select: false,
            ..WindowCharSelectOptions::default()
        });
        for (key, physical) in [
            ("f", WinitKeyCode::KeyF),
            ("0", WinitKeyCode::Digit0),
            ("9", WinitKeyCode::Digit9),
            ("b", WinitKeyCode::KeyB),
        ] {
            app.modifiers = ModifiersState::empty();
            app.handle_keyboard_input_event(
                &Key::Character(key.into()),
                PhysicalKey::Code(physical),
                Some(key),
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
        }

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.starts_with("> \u{f09b} U+F09B NF-FA-GITHUB"),
            "first terminal row was {first_terminal_row:?}"
        );

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Enter),
            PhysicalKey::Code(WinitKeyCode::Enter),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.char_select_active_for_test());
        assert_eq!(written.lock().unwrap().as_slice(), "\u{f09b}".as_bytes());
    }

    #[test]
    fn window_app_char_select_backspace_edits_text_input_without_writing_to_pty() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            copy_on_select: false,
            ..WindowCharSelectOptions::default()
        });
        for (key, physical) in [
            ("2", WinitKeyCode::Digit2),
            ("6", WinitKeyCode::Digit6),
            ("3", WinitKeyCode::Digit3),
            ("x", WinitKeyCode::KeyX),
        ] {
            app.modifiers = ModifiersState::empty();
            app.handle_keyboard_input_event(
                &Key::Character(key.into()),
                PhysicalKey::Code(physical),
                Some(key),
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
        }

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Backspace),
            PhysicalKey::Code(WinitKeyCode::Backspace),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Char Select: SmileysAndEmotion [263]"
        );
        assert!(written.lock().unwrap().is_empty());

        app.handle_keyboard_input_event(
            &Key::Character("a".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            Some("a"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Enter),
            PhysicalKey::Code(WinitKeyCode::Enter),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.char_select_active_for_test());
        assert_eq!(written.lock().unwrap().as_slice(), "\u{263a}".as_bytes());
    }

    #[test]
    fn window_app_char_select_ctrl_r_cycles_groups_without_writing_to_pty() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            group: Some("SmileysAndEmotion".to_owned()),
            ..WindowCharSelectOptions::default()
        });

        app.modifiers = ModifiersState::CONTROL;
        app.handle_keyboard_input_event(
            &Key::Character("r".into()),
            PhysicalKey::Code(WinitKeyCode::KeyR),
            Some("r"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Char Select: PeopleAndBody"
        );

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character("R".into()),
            PhysicalKey::Code(WinitKeyCode::KeyR),
            Some("R"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(app.char_select_active_for_test());
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Char Select: SmileysAndEmotion"
        );
        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_char_select_enter_inserts_hex_codepoint_and_closes() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            copy_on_select: false,
            ..WindowCharSelectOptions::default()
        });
        for (key, physical) in [
            ("2", WinitKeyCode::Digit2),
            ("6", WinitKeyCode::Digit6),
            ("3", WinitKeyCode::Digit3),
            ("a", WinitKeyCode::KeyA),
        ] {
            app.modifiers = ModifiersState::empty();
            app.handle_keyboard_input_event(
                &Key::Character(key.into()),
                PhysicalKey::Code(physical),
                Some(key),
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
        }

        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Char Select: SmileysAndEmotion [263a]"
        );

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Enter),
            PhysicalKey::Code(WinitKeyCode::Enter),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.char_select_active_for_test());
        assert_eq!(written.lock().unwrap().as_slice(), "\u{263a}".as_bytes());
    }

    #[test]
    fn window_app_char_select_enter_inserts_unicode_name_match_and_closes() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            copy_on_select: false,
            ..WindowCharSelectOptions::default()
        });
        for (key, physical) in [
            ("g", WinitKeyCode::KeyG),
            ("r", WinitKeyCode::KeyR),
            ("i", WinitKeyCode::KeyI),
            ("n", WinitKeyCode::KeyN),
            ("n", WinitKeyCode::KeyN),
            ("i", WinitKeyCode::KeyI),
            ("n", WinitKeyCode::KeyN),
            ("g", WinitKeyCode::KeyG),
            (" ", WinitKeyCode::Space),
            ("f", WinitKeyCode::KeyF),
            ("a", WinitKeyCode::KeyA),
            ("c", WinitKeyCode::KeyC),
            ("e", WinitKeyCode::KeyE),
        ] {
            app.modifiers = ModifiersState::empty();
            app.handle_keyboard_input_event(
                &Key::Character(key.into()),
                PhysicalKey::Code(physical),
                Some(key),
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
        }

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Enter),
            PhysicalKey::Code(WinitKeyCode::Enter),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.char_select_active_for_test());
        assert_eq!(written.lock().unwrap().as_slice(), "\u{1f600}".as_bytes());
    }

    #[test]
    fn window_app_char_select_enter_inserts_fuzzy_unicode_name_match_and_closes() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            copy_on_select: false,
            ..WindowCharSelectOptions::default()
        });
        for (key, physical) in [
            ("g", WinitKeyCode::KeyG),
            ("r", WinitKeyCode::KeyR),
            ("i", WinitKeyCode::KeyI),
            ("n", WinitKeyCode::KeyN),
            (" ", WinitKeyCode::Space),
            ("f", WinitKeyCode::KeyF),
            ("a", WinitKeyCode::KeyA),
            ("c", WinitKeyCode::KeyC),
            ("e", WinitKeyCode::KeyE),
        ] {
            app.modifiers = ModifiersState::empty();
            app.handle_keyboard_input_event(
                &Key::Character(key.into()),
                PhysicalKey::Code(physical),
                Some(key),
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
        }

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Enter),
            PhysicalKey::Code(WinitKeyCode::Enter),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.char_select_active_for_test());
        assert_eq!(written.lock().unwrap().as_slice(), "\u{1f600}".as_bytes());
    }

    #[test]
    fn window_app_char_select_renders_fuzzy_unicode_name_candidate() {
        let mut app = NativeWindowApp::new(None);

        app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            copy_on_select: false,
            ..WindowCharSelectOptions::default()
        });
        for (key, physical) in [
            ("g", WinitKeyCode::KeyG),
            ("r", WinitKeyCode::KeyR),
            ("i", WinitKeyCode::KeyI),
            ("n", WinitKeyCode::KeyN),
            (" ", WinitKeyCode::Space),
            ("f", WinitKeyCode::KeyF),
            ("a", WinitKeyCode::KeyA),
            ("c", WinitKeyCode::KeyC),
            ("e", WinitKeyCode::KeyE),
        ] {
            app.handle_keyboard_input_event(
                &Key::Character(key.into()),
                PhysicalKey::Code(physical),
                Some(key),
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
        }

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.contains("GRINNING FACE"),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_char_select_arrow_down_selects_next_unicode_name_candidate() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            copy_on_select: false,
            ..WindowCharSelectOptions::default()
        });
        for (key, physical) in [
            ("g", WinitKeyCode::KeyG),
            ("r", WinitKeyCode::KeyR),
            ("i", WinitKeyCode::KeyI),
            ("n", WinitKeyCode::KeyN),
            (" ", WinitKeyCode::Space),
            ("f", WinitKeyCode::KeyF),
            ("a", WinitKeyCode::KeyA),
            ("c", WinitKeyCode::KeyC),
            ("e", WinitKeyCode::KeyE),
        ] {
            app.modifiers = ModifiersState::empty();
            app.handle_keyboard_input_event(
                &Key::Character(key.into()),
                PhysicalKey::Code(physical),
                Some(key),
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
        }

        let second_candidate = {
            let char_select = app.char_select_for_test().expect("char select mode");
            assert!(
                char_select.matches.len() >= 2,
                "expected at least two fuzzy unicode candidates"
            );
            char_select.matches[1].text.clone()
        };
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::ArrowDown),
            PhysicalKey::Code(WinitKeyCode::ArrowDown),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        let second_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS + 1, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.starts_with("  "),
            "first terminal row was {first_terminal_row:?}"
        );
        assert!(
            second_terminal_row.starts_with("> "),
            "second terminal row was {second_terminal_row:?}"
        );

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Enter),
            PhysicalKey::Code(WinitKeyCode::Enter),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.char_select_active_for_test());
        assert_eq!(
            written.lock().unwrap().as_slice(),
            second_candidate.as_bytes()
        );
    }

    #[test]
    fn window_app_char_select_arrow_up_wraps_to_previous_unicode_name_candidate() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            copy_on_select: false,
            ..WindowCharSelectOptions::default()
        });
        for (key, physical) in [
            ("g", WinitKeyCode::KeyG),
            ("r", WinitKeyCode::KeyR),
            ("i", WinitKeyCode::KeyI),
            ("n", WinitKeyCode::KeyN),
            (" ", WinitKeyCode::Space),
            ("f", WinitKeyCode::KeyF),
            ("a", WinitKeyCode::KeyA),
            ("c", WinitKeyCode::KeyC),
            ("e", WinitKeyCode::KeyE),
        ] {
            app.modifiers = ModifiersState::empty();
            app.handle_keyboard_input_event(
                &Key::Character(key.into()),
                PhysicalKey::Code(physical),
                Some(key),
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
        }

        let (last_candidate, last_index) = {
            let char_select = app.char_select_for_test().expect("char select mode");
            assert!(
                char_select.matches.len() >= 2,
                "expected at least two fuzzy unicode candidates"
            );
            let last_index = char_select.matches.len() - 1;
            (char_select.matches[last_index].text.clone(), last_index)
        };
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::ArrowUp),
            PhysicalKey::Code(WinitKeyCode::ArrowUp),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        let last_terminal_row = snapshot_row_text(
            &snapshot,
            TAB_BAR_ROWS + u16::try_from(last_index).unwrap(),
            TERMINAL_COLUMNS,
        );

        assert!(
            first_terminal_row.starts_with("  "),
            "first terminal row was {first_terminal_row:?}"
        );
        assert!(
            last_terminal_row.starts_with("> "),
            "last terminal row was {last_terminal_row:?}"
        );

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Enter),
            PhysicalKey::Code(WinitKeyCode::Enter),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.char_select_active_for_test());
        assert_eq!(
            written.lock().unwrap().as_slice(),
            last_candidate.as_bytes()
        );
    }

    #[test]
    fn window_app_char_select_defaults_to_recently_used_after_selection() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            copy_on_select: false,
            ..WindowCharSelectOptions::default()
        });
        for (key, physical) in [
            ("2", WinitKeyCode::Digit2),
            ("6", WinitKeyCode::Digit6),
            ("3", WinitKeyCode::Digit3),
            ("a", WinitKeyCode::KeyA),
        ] {
            app.modifiers = ModifiersState::empty();
            app.handle_keyboard_input_event(
                &Key::Character(key.into()),
                PhysicalKey::Code(physical),
                Some(key),
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
        }
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Enter),
            PhysicalKey::Code(WinitKeyCode::Enter),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.char_select_active_for_test());

        app.enter_char_select_mode();

        let char_select = app.char_select_for_test().expect("char select mode");
        assert_eq!(char_select.group.as_deref(), Some("RecentlyUsed"));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Char Select: RecentlyUsed"
        );
    }

    #[test]
    fn window_app_char_select_persists_recently_used_between_app_instances() {
        let path = temp_char_select_recently_used_path();
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.set_char_select_recently_used_path_for_test(Some(path.clone()));

        app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            copy_on_select: false,
            ..WindowCharSelectOptions::default()
        });
        for (key, physical) in [
            ("2", WinitKeyCode::Digit2),
            ("6", WinitKeyCode::Digit6),
            ("3", WinitKeyCode::Digit3),
            ("a", WinitKeyCode::KeyA),
        ] {
            app.modifiers = ModifiersState::empty();
            app.handle_keyboard_input_event(
                &Key::Character(key.into()),
                PhysicalKey::Code(physical),
                Some(key),
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
        }
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Enter),
            PhysicalKey::Code(WinitKeyCode::Enter),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        let mut restored = NativeWindowApp::new(None);
        restored.set_char_select_recently_used_path_for_test(Some(path.clone()));
        restored.enter_char_select_mode();

        let char_select = restored.char_select_for_test().expect("char select mode");
        assert_eq!(char_select.group.as_deref(), Some("RecentlyUsed"));
        assert_eq!(char_select.selected_text().as_deref(), Some("\u{263a}"));

        let snapshot = restored.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.starts_with("> ☺ U+263A"),
            "first terminal row was {first_terminal_row:?}"
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn window_app_char_select_recently_used_uses_persisted_last_used_for_ties() {
        let path = temp_char_select_recently_used_path();
        std::fs::write(
            &path,
            r#"{
  "entries": [
    { "text": "A", "selections": 1, "last_used": 1 },
    { "text": "B", "selections": 1, "last_used": 2 }
  ]
}"#,
        )
        .unwrap();
        let mut app = NativeWindowApp::new(None);
        app.set_char_select_recently_used_path_for_test(Some(path.clone()));

        app.enter_char_select_mode();

        let char_select = app.char_select_for_test().expect("char select mode");
        assert_eq!(char_select.group.as_deref(), Some("RecentlyUsed"));
        assert_eq!(char_select.selected_text().as_deref(), Some("B"));

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.starts_with("> B U+0042"),
            "first terminal row was {first_terminal_row:?}"
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn window_app_char_select_recently_used_enter_reselects_latest_character() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            copy_on_select: false,
            ..WindowCharSelectOptions::default()
        });
        for (key, physical) in [
            ("2", WinitKeyCode::Digit2),
            ("6", WinitKeyCode::Digit6),
            ("3", WinitKeyCode::Digit3),
            ("a", WinitKeyCode::KeyA),
        ] {
            app.modifiers = ModifiersState::empty();
            app.handle_keyboard_input_event(
                &Key::Character(key.into()),
                PhysicalKey::Code(physical),
                Some(key),
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
        }
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Enter),
            PhysicalKey::Code(WinitKeyCode::Enter),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        written.lock().unwrap().clear();

        app.enter_char_select_mode();
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Enter),
            PhysicalKey::Code(WinitKeyCode::Enter),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.char_select_active_for_test());
        assert_eq!(written.lock().unwrap().as_slice(), "\u{263a}".as_bytes());
    }

    #[test]
    fn window_app_char_select_recently_used_renders_and_selects_prior_candidates() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        for input in [
            &[
                ("2", WinitKeyCode::Digit2),
                ("6", WinitKeyCode::Digit6),
                ("3", WinitKeyCode::Digit3),
                ("a", WinitKeyCode::KeyA),
            ][..],
            &[
                ("1", WinitKeyCode::Digit1),
                ("f", WinitKeyCode::KeyF),
                ("6", WinitKeyCode::Digit6),
                ("0", WinitKeyCode::Digit0),
                ("0", WinitKeyCode::Digit0),
            ][..],
        ] {
            app.enter_char_select_mode_with_options(WindowCharSelectOptions {
                copy_on_select: false,
                ..WindowCharSelectOptions::default()
            });
            for &(key, physical) in input {
                app.modifiers = ModifiersState::empty();
                app.handle_keyboard_input_event(
                    &Key::Character(key.into()),
                    PhysicalKey::Code(physical),
                    Some(key),
                    ElementState::Pressed,
                    KittyKeyEventKind::Press,
                )
                .unwrap();
            }
            app.handle_keyboard_input_event(
                &Key::Named(NamedKey::Enter),
                PhysicalKey::Code(WinitKeyCode::Enter),
                None,
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
        }
        written.lock().unwrap().clear();

        app.enter_char_select_mode();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        let second_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS + 1, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.starts_with("> 😀 U+1F600"),
            "first terminal row was {first_terminal_row:?}"
        );
        assert!(
            second_terminal_row.starts_with("  ☺ U+263A"),
            "second terminal row was {second_terminal_row:?}"
        );

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::ArrowDown),
            PhysicalKey::Code(WinitKeyCode::ArrowDown),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Enter),
            PhysicalKey::Code(WinitKeyCode::Enter),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.char_select_active_for_test());
        assert_eq!(written.lock().unwrap().as_slice(), "\u{263a}".as_bytes());
    }

    #[test]
    fn window_app_char_select_recently_used_orders_candidates_by_frequency_then_recency() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        for input in [
            &[
                ("2", WinitKeyCode::Digit2),
                ("6", WinitKeyCode::Digit6),
                ("3", WinitKeyCode::Digit3),
                ("a", WinitKeyCode::KeyA),
            ][..],
            &[
                ("2", WinitKeyCode::Digit2),
                ("6", WinitKeyCode::Digit6),
                ("3", WinitKeyCode::Digit3),
                ("a", WinitKeyCode::KeyA),
            ][..],
            &[
                ("1", WinitKeyCode::Digit1),
                ("f", WinitKeyCode::KeyF),
                ("6", WinitKeyCode::Digit6),
                ("0", WinitKeyCode::Digit0),
                ("0", WinitKeyCode::Digit0),
            ][..],
        ] {
            app.enter_char_select_mode_with_options(WindowCharSelectOptions {
                copy_on_select: false,
                ..WindowCharSelectOptions::default()
            });
            for &(key, physical) in input {
                app.modifiers = ModifiersState::empty();
                app.handle_keyboard_input_event(
                    &Key::Character(key.into()),
                    PhysicalKey::Code(physical),
                    Some(key),
                    ElementState::Pressed,
                    KittyKeyEventKind::Press,
                )
                .unwrap();
            }
            app.handle_keyboard_input_event(
                &Key::Named(NamedKey::Enter),
                PhysicalKey::Code(WinitKeyCode::Enter),
                None,
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
        }

        app.enter_char_select_mode();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        let second_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS + 1, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.starts_with("> ☺ U+263A"),
            "first terminal row was {first_terminal_row:?}"
        );
        assert!(
            second_terminal_row.starts_with("  😀 U+1F600"),
            "second terminal row was {second_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_char_select_enter_accepts_prefixed_hex_codepoints() {
        for input in [
            [
                ("U", WinitKeyCode::KeyU),
                ("+", WinitKeyCode::Equal),
                ("2", WinitKeyCode::Digit2),
                ("6", WinitKeyCode::Digit6),
                ("3", WinitKeyCode::Digit3),
                ("a", WinitKeyCode::KeyA),
            ],
            [
                ("0", WinitKeyCode::Digit0),
                ("x", WinitKeyCode::KeyX),
                ("2", WinitKeyCode::Digit2),
                ("6", WinitKeyCode::Digit6),
                ("3", WinitKeyCode::Digit3),
                ("a", WinitKeyCode::KeyA),
            ],
        ] {
            let written = Arc::new(Mutex::new(Vec::new()));
            let mut app = NativeWindowApp::new(None);
            app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

            app.enter_char_select_mode_with_options(WindowCharSelectOptions {
                copy_on_select: false,
                ..WindowCharSelectOptions::default()
            });
            for (key, physical) in input {
                app.modifiers = ModifiersState::empty();
                app.handle_keyboard_input_event(
                    &Key::Character(key.into()),
                    PhysicalKey::Code(physical),
                    Some(key),
                    ElementState::Pressed,
                    KittyKeyEventKind::Press,
                )
                .unwrap();
            }

            app.handle_keyboard_input_event(
                &Key::Named(NamedKey::Enter),
                PhysicalKey::Code(WinitKeyCode::Enter),
                None,
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();

            assert!(!app.char_select_active_for_test());
            assert_eq!(written.lock().unwrap().as_slice(), "\u{263a}".as_bytes());
        }
    }

    #[test]
    fn window_app_char_select_enter_copies_selected_character_to_configured_destination() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let clipboard = Arc::new(Mutex::new(Vec::new()));
        let primary = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let clipboard_recorded = Arc::clone(&clipboard);
        app.clipboard_writer = Box::new(move |text: &str| {
            clipboard_recorded.lock().unwrap().push(text.to_owned());
            true
        });
        let primary_recorded = Arc::clone(&primary);
        app.primary_selection_writer = Box::new(move |text: &str| {
            primary_recorded.lock().unwrap().push(text.to_owned());
            true
        });

        app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            copy_to: WindowCopyDestination::PrimarySelection,
            ..WindowCharSelectOptions::default()
        });
        for (key, physical) in [
            ("2", WinitKeyCode::Digit2),
            ("6", WinitKeyCode::Digit6),
            ("3", WinitKeyCode::Digit3),
            ("a", WinitKeyCode::KeyA),
        ] {
            app.handle_keyboard_input_event(
                &Key::Character(key.into()),
                PhysicalKey::Code(physical),
                Some(key),
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
        }

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Enter),
            PhysicalKey::Code(WinitKeyCode::Enter),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.char_select_active_for_test());
        assert_eq!(written.lock().unwrap().as_slice(), "\u{263a}".as_bytes());
        assert!(clipboard.lock().unwrap().is_empty());
        assert_eq!(primary.lock().unwrap().as_slice(), ["\u{263a}".to_owned()]);
    }

    #[test]
    fn window_app_set_config_overrides_updates_effective_config_and_dispatches_reload() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.config_reloaded_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(*event);
            true
        });
        let active_pane = app.app_shell.active_pane_id();
        let overrides = sample_native_config_overrides!();

        app.set_config_overrides(overrides.clone());

        assert_eq!(app.get_config_overrides(), overrides);
        assert_eq!(app.native_effective_config(), sample_effective_config!());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeWindowConfigReloaded {
                window_id: rterm_core::WindowId::new(1),
                pane: active_pane,
            }]
        );

        app.set_config_overrides(NativeConfigSnapshot::default());

        assert_eq!(app.get_config_overrides(), NativeConfigSnapshot::default());
        assert_eq!(app.native_effective_config(), default_effective_config());
        assert_eq!(events.lock().unwrap().len(), 2);
    }

    #[test]
    fn window_app_applies_configured_scrollback_lines_to_runtime() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            scrollback_lines: Some(1),
            ..NativeConfigSnapshot::default()
        });
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();

        assert_eq!(app.native_effective_config().scrollback_lines, 1);
        assert_eq!(app.runtime.terminal().scrollback().len(), 1);
    }

    #[test]
    fn window_app_applies_foreground_text_hsb_to_render_snapshot() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            foreground_text_hsb: Some(NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(1.0),
                brightness: NativeHsbMultiplier::from_f32(0.5),
            }),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[38;2;100;150;200;48;2;20;40;60mA\x1b[0m")
            .unwrap();

        let snapshot = app.render_snapshot();
        let cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("visible cell");

        assert_eq!(
            app.native_effective_config().foreground_text_hsb,
            NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(1.0),
                brightness: NativeHsbMultiplier::from_f32(0.5),
            }
        );
        assert_eq!(cell.foreground, rterm_terminal::Color::Rgb(50, 75, 100));
        assert_eq!(cell.background, rterm_terminal::Color::Rgb(20, 40, 60));
    }

    #[test]
    fn window_app_applies_wezterm_text_min_contrast_ratio_to_render_snapshot() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.text_min_contrast_ratio = 4.5
            config.colors = {
              foreground = '#111111',
              background = '#101010',
            }

            return config
            "##,
        )
        .expect("expected WezTerm text_min_contrast_ratio config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"A").unwrap();

        let snapshot = app.render_snapshot();
        let cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("visible cell");
        let foreground = color_to_rgba(cell.foreground, [17, 17, 17, 255]);
        let background = color_to_rgba(cell.background, [16, 16, 16, 255]);

        assert_eq!(background, [16, 16, 16, 255]);
        assert_ne!(foreground, [17, 17, 17, 255]);
        assert!(
            test_contrast_ratio(foreground, background) >= 4.5,
            "foreground {foreground:?} background {background:?} did not reach 4.5 contrast"
        );
    }

    #[test]
    fn window_app_applies_wezterm_window_content_alignment_to_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_content_alignment = {
              horizontal = 'Center',
              vertical = 'Bottom',
            }
            config.colors = {
              background = '#010203',
            }

            return config
            "##,
        )
        .expect("expected WezTerm window content alignment config");
        app.set_config_overrides(overrides);
        app.handle_window_resize(PhysicalSize::new(FRAME_WIDTH + 5, FRAME_HEIGHT + 7))
            .unwrap();
        app.handle_pty_output(b"\x1b[48;2;10;20;30m \x1b[0m")
            .unwrap();
        let (frame_width, frame_height) = app.frame_size_for_test();
        let mut frame =
            vec![0; usize::try_from(frame_width.saturating_mul(frame_height) * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let width = usize::try_from(frame_width).unwrap();
        let content_x = 2usize;
        let content_y = 7usize;
        let first_terminal_cell_y = content_y + usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        assert_eq!(
            frame_pixel_at(&frame, width, 0, first_terminal_cell_y),
            [1, 2, 3, 255]
        );
        assert_eq!(
            frame_pixel_at(&frame, width, content_x, first_terminal_cell_y),
            [10, 20, 30, 255]
        );
    }

    #[test]
    fn window_app_maps_mouse_through_wezterm_window_content_alignment_gap() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            window_content_alignment: Some(NativeWindowContentAlignment {
                horizontal: NativeHorizontalContentAlignment::Center,
                vertical: NativeVerticalContentAlignment::Bottom,
            }),
            ..NativeConfigSnapshot::default()
        });
        app.handle_window_resize(PhysicalSize::new(FRAME_WIDTH + 5, FRAME_HEIGHT + 7))
            .unwrap();
        let first_terminal_cell_y = 7.0 + f64::from(TAB_BAR_ROWS) * f64::from(CELL_HEIGHT);

        assert_eq!(
            app.window_mouse_cell(PhysicalPosition::new(1.0, first_terminal_cell_y)),
            None
        );
        assert_eq!(
            app.window_mouse_cell(PhysicalPosition::new(2.0, first_terminal_cell_y)),
            Some((0, 0))
        );
    }

    #[test]
    fn window_app_applies_text_background_opacity_to_non_default_backgrounds() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            text_background_opacity: Some(NativeTextBackgroundOpacity::from_f32(0.5)),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[48;2;20;40;60mA\x1b[0mB")
            .unwrap();

        let snapshot = app.render_snapshot();
        let colored_cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("colored cell");
        let default_cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 1).expect("default cell");

        assert_eq!(
            app.native_effective_config().text_background_opacity,
            NativeTextBackgroundOpacity::from_f32(0.5)
        );
        assert_eq!(
            colored_cell.background,
            rterm_terminal::Color::Rgba(20, 40, 60, 127)
        );
        assert_eq!(default_cell.background, rterm_terminal::Color::Default);
    }

    #[test]
    fn window_app_applies_window_padding_pixels_to_pane_render_layout() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(20, 6));
        app.refresh_snapshot();
        app.set_config_overrides(native_config_snapshot! {
            window_padding: Some(NativeWindowPadding {
                left: NativeWindowPaddingDimension::Pixels(8),
                right: NativeWindowPaddingDimension::Pixels(16),
                top: NativeWindowPaddingDimension::Pixels(16),
                bottom: NativeWindowPaddingDimension::Pixels(32),
            }),
            ..NativeConfigSnapshot::default()
        });

        let layout = app.pane_render_layout();
        let rect = layout.panes.first().expect("pane rect");

        assert_eq!(
            app.native_effective_config().window_padding,
            NativeWindowPadding {
                left: NativeWindowPaddingDimension::Pixels(8),
                right: NativeWindowPaddingDimension::Pixels(16),
                top: NativeWindowPaddingDimension::Pixels(16),
                bottom: NativeWindowPaddingDimension::Pixels(32),
            }
        );
        assert_eq!(rect.row, TAB_BAR_ROWS);
        assert_eq!(rect.column, 0);
        assert_eq!(rect.rows, 6);
        assert_eq!(rect.columns, 20);
    }

    const PANE_OVERLAY_SEARCH_BG: Color = Color::Rgb(41, 42, 43);
    const PANE_OVERLAY_COPY_ACTIVE_BG: Color = Color::Rgb(51, 52, 53);
    const PANE_OVERLAY_COPY_INACTIVE_BG: Color = Color::Rgb(61, 62, 63);
    const PANE_OVERLAY_QUICK_MATCH_BG: Color = Color::Rgb(71, 72, 73);
    const PANE_OVERLAY_QUICK_LABEL_BG: Color = Color::Rgb(81, 82, 83);

    fn pane_overlay_identity_hsb() -> NativeInactivePaneHsb {
        NativeInactivePaneHsb {
            hue: NativeHsbMultiplier::from_f32(1.0),
            saturation: NativeHsbMultiplier::from_f32(1.0),
            brightness: NativeHsbMultiplier::from_f32(1.0),
        }
    }

    fn configure_pane_overlay_presentation_test(
        app: &mut NativeWindowApp,
        inactive_pane_hsb: NativeInactivePaneHsb,
    ) {
        app.set_config_overrides(native_config_snapshot! {
            inactive_pane_hsb: Some(inactive_pane_hsb),
            quick_select_remove_styling: Some(true),
            selection_bg_color: Some(PANE_OVERLAY_SEARCH_BG),
            copy_mode_active_highlight_bg: Some(NativeColorSpec::Color(
                PANE_OVERLAY_COPY_ACTIVE_BG,
            )),
            copy_mode_inactive_highlight_bg: Some(NativeColorSpec::Color(
                PANE_OVERLAY_COPY_INACTIVE_BG,
            )),
            quick_select_match_bg: Some(NativeColorSpec::Color(PANE_OVERLAY_QUICK_MATCH_BG)),
            quick_select_label_bg: Some(NativeColorSpec::Color(PANE_OVERLAY_QUICK_LABEL_BG)),
            ..NativeConfigSnapshot::default()
        });
    }

    fn pane_overlay_test_match(
        app: &NativeWindowApp,
        row: u16,
        start_column: u16,
        end_column: u16,
    ) -> WindowSearchMatch {
        WindowSearchMatch {
            domain: app.runtime.terminal().stable_dimensions().domain,
            source_row: app
                .current_viewport_stable_top()
                .saturating_add(StableRowIndex::try_from(row).unwrap()),
            start_column,
            end_source_row: app
                .current_viewport_stable_top()
                .saturating_add(StableRowIndex::try_from(row).unwrap()),
            end_column,
        }
    }

    fn install_pane_search_presentation_for_test(
        app: &mut NativeWindowApp,
        query: &str,
        matched: WindowSearchMatch,
    ) {
        let initial_copy_mode = app.initial_copy_mode();
        app.active_ui.enter_search(
            initial_copy_mode,
            WindowSearch {
                query: query.to_owned(),
                current: None,
                match_type: WindowSearchMatchType::CaseSensitive,
                editing: true,
            },
        );
        assert!(app.active_ui.set_search_current(Some(matched)));
        app.refresh_snapshot();
    }

    fn install_pane_copy_presentation_for_test(
        app: &mut NativeWindowApp,
        query: &str,
        copy_column: u16,
    ) -> (WindowSearchMatch, WindowSearchMatch) {
        let matches = super::window_search_matches_with_type(
            app.runtime.terminal(),
            query,
            WindowSearchMatchType::CaseSensitive,
        );
        assert_eq!(matches.len(), 2, "copy fixture needs two stable matches");
        let inactive_match = matches[0];
        let retained_current = matches[1];
        let source_cursor = SelectionSourceCell {
            domain: app.runtime.terminal().stable_dimensions().domain,
            row: app.current_viewport_stable_top(),
            column: usize::from(copy_column),
        };
        let mut copy_mode = app.initial_copy_mode();
        copy_mode.cursor = SelectionCell {
            row: 0,
            column: copy_column,
        };
        copy_mode.source_cursor = source_cursor;
        copy_mode.selection_mode = super::WindowCopySelectionMode::Cell;
        copy_mode.anchor = Some(copy_mode.cursor);
        copy_mode.source_anchor = Some(source_cursor);
        copy_mode.search_direction = Some(SearchDirection::Next);
        app.active_ui.enter_search(
            copy_mode,
            WindowSearch {
                query: query.to_owned(),
                current: None,
                match_type: WindowSearchMatchType::CaseSensitive,
                editing: true,
            },
        );
        assert!(app.active_ui.set_search_current(Some(retained_current)));
        let ignored_initial_copy_mode = app.initial_copy_mode();
        app.active_ui.enter_copy_mode(ignored_initial_copy_mode);
        app.refresh_snapshot();
        (inactive_match, retained_current)
    }

    fn install_pane_quick_presentation_for_test(
        app: &mut NativeWindowApp,
        label: &str,
        owner: &str,
        matched: WindowSearchMatch,
    ) {
        app.active_ui.enter_quick_select(WindowQuickSelect {
            current: 0,
            matches: vec![matched],
            labels: vec![label.to_owned()],
            input: String::new(),
            reflow_config: None,
            action_label: Some(format!("owner-{owner}")),
            action: WindowQuickSelectAction::Nop,
            skip_action_on_paste: false,
        });
        app.refresh_snapshot();
        app.apply_window_title();
    }

    fn pane_rect_for_test(
        app: &NativeWindowApp,
        pane_id: rterm_core::PaneId,
    ) -> super::PaneRenderRect {
        app.pane_render_layout()
            .panes
            .into_iter()
            .find(|rect| rect.pane_id == pane_id)
            .expect("pane must be visible")
    }

    fn assert_snapshot_cell_background(
        snapshot: &TerminalRenderSnapshot,
        row: u16,
        column: u16,
        expected: Color,
        context: &str,
    ) {
        assert_eq!(
            snapshot_cell(snapshot, row, column)
                .unwrap_or_else(|| panic!("{context}: missing cell at {row},{column}"))
                .background,
            expected,
            "{context}"
        );
    }

    fn quick_label_cells_for_test(
        app: &NativeWindowApp,
        label: &str,
        start_column: u16,
        rect_columns: u16,
    ) -> Vec<(u16, char)> {
        let matched = pane_overlay_test_match(app, 0, start_column, start_column);
        let quick_select = WindowQuickSelect {
            current: 0,
            matches: vec![matched],
            labels: vec![label.to_owned()],
            input: String::new(),
            reflow_config: None,
            action_label: None,
            action: WindowQuickSelectAction::Nop,
            skip_action_on_paste: false,
        };
        super::quick_select_cells_for_pane(
            app.runtime.terminal(),
            app.active_ui.stable_viewport,
            &quick_select,
            super::PaneRenderRect {
                pane_id: app.app_shell.active_pane_id(),
                row: 0,
                column: 0,
                rows: 1,
                columns: rect_columns,
            },
            &app.native_resolved_palette(),
        )
        .into_iter()
        .map(|cell| (cell.column, cell.ch))
        .collect()
    }

    fn reset_search_match_cache_recompute_counts(app: &NativeWindowApp) {
        app.active_ui.reset_search_match_cache_recompute_count();
        for runtime in app.pane_runtimes.values() {
            runtime.ui.reset_search_match_cache_recompute_count();
        }
    }

    fn search_match_cache_recompute_count(app: &NativeWindowApp) -> usize {
        app.active_ui
            .search_match_cache_recompute_count()
            .saturating_add(
                app.pane_runtimes
                    .values()
                    .map(|runtime| runtime.ui.search_match_cache_recompute_count())
                    .sum(),
            )
    }

    #[test]
    fn window_app_quick_labels_advance_by_terminal_display_width() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 1));

        let default_wide = quick_label_cells_for_test(&app, "界a", 1, 8);
        app.runtime
            .set_treat_east_asian_ambiguous_width_as_wide(true);
        let ambiguous_wide = quick_label_cells_for_test(&app, "☆a", 1, 8);
        app.runtime
            .set_cell_width_overrides(vec![rterm_terminal::CellWidthOverride::new(
                u32::from('☆'),
                u32::from('☆'),
                3,
            )]);
        let overridden_wide = quick_label_cells_for_test(&app, "☆a", 1, 8);

        assert_eq!(
            (default_wide, ambiguous_wide, overridden_wide),
            (
                vec![(1, '界'), (2, ' '), (3, 'a')],
                vec![(1, '☆'), (2, ' '), (3, 'a')],
                vec![(1, '☆'), (2, ' '), (3, ' '), (4, 'a')],
            ),
            "Quick labels must use the owner terminal's effective display-width rules"
        );
    }

    #[test]
    fn window_app_quick_labels_clip_whole_wide_glyph_spans() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 1));

        assert!(
            quick_label_cells_for_test(&app, "界a", 3, 4).is_empty(),
            "a glyph whose full [start,end) span crosses the owner rect must not be emitted"
        );
    }

    #[test]
    fn window_app_pane_overlay_redraws_reuse_owner_search_match_cache() {
        let mut app = NativeWindowApp::new(None);
        configure_pane_overlay_presentation_test(&mut app, pane_overlay_identity_hsb());
        app.runtime.resize(rterm_core::TerminalSize::new(24, 4));
        app.handle_pty_output(b"x-left-x").unwrap();
        install_pane_copy_presentation_for_test(&mut app, "x", 3);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"x-right-x").unwrap();
        install_pane_copy_presentation_for_test(&mut app, "x", 4);

        reset_search_match_cache_recompute_counts(&app);
        let first = app.render_snapshot();
        let second = app.render_snapshot();
        assert_eq!(first.cells(), second.cells());
        assert_eq!(
            search_match_cache_recompute_count(&app),
            0,
            "pure redraws must consume each owner cache without rescanning terminal history"
        );
    }

    #[test]
    fn window_app_search_match_cache_reprojects_viewport_without_rescan() {
        let mut app = NativeWindowApp::new(None);
        configure_pane_overlay_presentation_test(&mut app, pane_overlay_identity_hsb());
        app.runtime.resize(rterm_core::TerminalSize::new(12, 2));
        app.handle_pty_output(b"x-old\r\nplain\r\nx-new\r\nplain")
            .unwrap();
        install_pane_copy_presentation_for_test(&mut app, "x", 3);

        let bottom = app.render_snapshot();
        assert_ne!(
            rendered_active_pane_cell(&app, 0, 0)
                .expect("bottom viewport cell")
                .background,
            PANE_OVERLAY_COPY_INACTIVE_BG,
            "the old match begins outside the bottom viewport"
        );
        reset_search_match_cache_recompute_counts(&app);
        app.scroll_viewport_lines(2);
        let scrolled = app.render_snapshot();
        assert_ne!(bottom.cells(), scrolled.cells());
        assert_eq!(
            rendered_active_pane_cell(&app, 0, 0)
                .expect("scrolled match cell")
                .background,
            PANE_OVERLAY_COPY_INACTIVE_BG,
            "stable cached matches must be reprojected into the moved viewport"
        );
        assert_eq!(
            search_match_cache_recompute_count(&app),
            0,
            "viewport-only movement must not invalidate stable source matches"
        );
    }

    #[test]
    fn window_app_search_match_cache_invalidates_once_for_query_and_type() {
        let mut app = NativeWindowApp::new(None);
        configure_pane_overlay_presentation_test(&mut app, pane_overlay_identity_hsb());
        app.runtime.resize(rterm_core::TerminalSize::new(24, 4));
        app.handle_pty_output(b"x-left-x y-left-y").unwrap();
        install_pane_copy_presentation_for_test(&mut app, "x", 3);

        reset_search_match_cache_recompute_counts(&app);
        assert!(app.update_search_query_with_type(
            "y",
            SearchDirection::Next,
            WindowSearchMatchType::CaseSensitive,
        ));
        let _ = app.render_snapshot();
        assert_eq!(
            search_match_cache_recompute_count(&app),
            1,
            "a query change must populate once and redraw from that result"
        );

        reset_search_match_cache_recompute_counts(&app);
        assert!(app.update_search_query_with_type(
            "y",
            SearchDirection::Next,
            WindowSearchMatchType::CaseInsensitive,
        ));
        let _ = app.render_snapshot();
        assert_eq!(
            search_match_cache_recompute_count(&app),
            1,
            "a match-type change must populate once and redraw from that result"
        );
    }

    #[test]
    fn window_app_search_match_cache_invalidates_only_mutated_pane_owner() {
        let mut app = NativeWindowApp::new(None);
        configure_pane_overlay_presentation_test(&mut app, pane_overlay_identity_hsb());
        app.runtime.resize(rterm_core::TerminalSize::new(24, 4));
        app.handle_pty_output(b"x-left-x").unwrap();
        install_pane_copy_presentation_for_test(&mut app, "x", 3);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"x-right-x").unwrap();
        install_pane_copy_presentation_for_test(&mut app, "x", 4);

        reset_search_match_cache_recompute_counts(&app);
        app.handle_pty_output(b"x").unwrap();
        let _ = app.render_snapshot();
        assert_eq!(
            search_match_cache_recompute_count(&app),
            1,
            "active output must repopulate only the active owner"
        );

        reset_search_match_cache_recompute_counts(&app);
        app.handle_pane_pty_output(rterm_core::PaneId::new(1), b"x")
            .unwrap();
        let _ = app.render_snapshot();
        assert_eq!(
            search_match_cache_recompute_count(&app),
            1,
            "inactive output must repopulate only the inactive owner"
        );
    }

    #[test]
    fn pane_search_match_cache_key_tracks_terminal_identity_resize_and_prune() {
        let key = |terminal: &rterm_terminal::Terminal, owner_epoch| {
            super::pane_transient_overlay::search_match_cache_key_for_test(
                terminal,
                owner_epoch,
                "x",
                WindowSearchMatchType::CaseSensitive,
            )
        };

        let mut resized = rterm_terminal::Terminal::new(rterm_core::TerminalSize::new(8, 2));
        let before_resize = key(&resized, 0);
        resized.resize(rterm_core::TerminalSize::new(10, 3));
        assert_ne!(before_resize, key(&resized, 0));

        let mut pruned = rterm_terminal::Terminal::new(rterm_core::TerminalSize::new(8, 2));
        pruned.set_scrollback_limit(1);
        let before_prune = key(&pruned, 0);
        pruned.feed(b"one\r\ntwo\r\nthree\r\nfour");
        assert_ne!(before_prune, key(&pruned, 0));

        let mut alternate = rterm_terminal::Terminal::new(rterm_core::TerminalSize::new(8, 2));
        let before_domain_change = key(&alternate, 0);
        alternate.feed(b"\x1b[?1049h");
        assert_ne!(before_domain_change, key(&alternate, 0));

        let stable_terminal = rterm_terminal::Terminal::new(rterm_core::TerminalSize::new(8, 2));
        assert_ne!(
            key(&stable_terminal, 0),
            key(&stable_terminal, 1),
            "owner terminal replacement epoch must disambiguate otherwise equal terminals"
        );
    }

    #[test]
    fn window_app_visible_split_panes_render_distinct_search_overlays() {
        let mut app = NativeWindowApp::new(None);
        configure_pane_overlay_presentation_test(&mut app, pane_overlay_identity_hsb());
        app.runtime.resize(rterm_core::TerminalSize::new(24, 4));
        app.handle_pty_output(
            b"old\r\nLEFT-search-owner\r\nfiller-2\r\nfiller-3\r\nfiller-4\r\nfiller-5",
        )
        .unwrap();
        app.scroll_viewport_lines(1);
        let left_viewport_top = app
            .runtime
            .terminal()
            .stable_dimensions()
            .physical_top
            .saturating_sub(1);
        assert_eq!(app.current_viewport_stable_top(), left_viewport_top);
        let left_match = pane_overlay_test_match(&app, 0, 1, 3);
        install_pane_search_presentation_for_test(&mut app, "LEFT", left_match);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.current_viewport_stable_top(), 0);
        assert_eq!(
            app.pane_runtimes
                .get(&rterm_core::PaneId::new(1))
                .and_then(|runtime| {
                    runtime
                        .ui
                        .stable_viewport
                        .active_top(runtime.runtime.terminal())
                }),
            Some(left_viewport_top),
            "inactive owner keeps a viewport top distinct from the active pane"
        );
        app.handle_pty_output(b"RIGHT-search-owner").unwrap();
        let right_match = pane_overlay_test_match(&app, 0, 2, 5);
        install_pane_search_presentation_for_test(&mut app, "RIGHT", right_match);

        let left = pane_rect_for_test(&app, rterm_core::PaneId::new(1));
        let right = pane_rect_for_test(&app, rterm_core::PaneId::new(2));
        assert_ne!(
            snapshot_cell(&app.snapshot, 0, 2)
                .expect("active base snapshot before presentation")
                .background,
            PANE_OVERLAY_SEARCH_BG,
            "active cached snapshot must remain selection-free"
        );
        let snapshot = app.render_snapshot();
        assert_eq!(
            snapshot_char(&snapshot, left.row, left.column.saturating_add(1)).unwrap_or(' '),
            'E'
        );
        assert_eq!(
            snapshot_char(&snapshot, right.row, right.column.saturating_add(2)).unwrap_or(' '),
            'G'
        );
        assert_snapshot_cell_background(
            &snapshot,
            left.row,
            left.column.saturating_add(1),
            PANE_OVERLAY_SEARCH_BG,
            "inactive left Search owner",
        );
        assert_snapshot_cell_background(
            &snapshot,
            right.row,
            right.column.saturating_add(2),
            PANE_OVERLAY_SEARCH_BG,
            "active right Search owner",
        );
        assert_ne!(
            snapshot_cell(&snapshot, left.row, left.column)
                .expect("left non-match")
                .background,
            PANE_OVERLAY_SEARCH_BG
        );
        assert_ne!(
            snapshot_cell(&snapshot, right.row, right.column)
                .expect("right non-match")
                .background,
            PANE_OVERLAY_SEARCH_BG
        );
        assert_ne!(
            snapshot_cell(&app.snapshot, 0, 2)
                .expect("active base snapshot")
                .background,
            PANE_OVERLAY_SEARCH_BG,
            "active cached snapshot must remain selection-free"
        );
        assert_ne!(
            snapshot_cell(
                &app.pane_runtimes
                    .get(&rterm_core::PaneId::new(1))
                    .expect("inactive left runtime")
                    .snapshot,
                0,
                1,
            )
            .expect("inactive base snapshot")
            .background,
            PANE_OVERLAY_SEARCH_BG,
            "inactive cached snapshot must remain selection-free"
        );

        let expected_left = snapshot_cell(
            &snapshot,
            left.row,
            left.column.saturating_add(left_match.start_column),
        )
        .expect("left presentation before focus round-trip")
        .background;
        let expected_right = snapshot_cell(
            &snapshot,
            right.row,
            right.column.saturating_add(right_match.start_column),
        )
        .expect("right presentation before focus round-trip")
        .background;
        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rterm_core::PaneId::new(1),
        })
        .unwrap();
        let focused_left = app.render_snapshot();
        assert_eq!(
            snapshot_cell(
                &focused_left,
                left.row,
                left.column.saturating_add(left_match.start_column),
            )
            .expect("left presentation while focused")
            .background,
            expected_left,
            "focus must not leave or double-apply cached owner presentation"
        );
        assert_eq!(
            snapshot_cell(
                &focused_left,
                right.row,
                right.column.saturating_add(right_match.start_column),
            )
            .expect("right presentation while inactive")
            .background,
            expected_right,
            "focus must not leave or double-apply cached owner presentation"
        );
        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rterm_core::PaneId::new(2),
        })
        .unwrap();
        let focused_right = app.render_snapshot();
        assert_eq!(
            snapshot_cell(
                &focused_right,
                left.row,
                left.column.saturating_add(left_match.start_column),
            )
            .expect("left presentation after focus round-trip")
            .background,
            expected_left,
        );
        assert_eq!(
            snapshot_cell(
                &focused_right,
                right.row,
                right.column.saturating_add(right_match.start_column),
            )
            .expect("right presentation after focus round-trip")
            .background,
            expected_right,
        );
    }

    #[test]
    fn window_app_visible_split_panes_render_distinct_copy_overlays() {
        let mut app = NativeWindowApp::new(None);
        configure_pane_overlay_presentation_test(&mut app, pane_overlay_identity_hsb());
        app.runtime.resize(rterm_core::TerminalSize::new(24, 4));
        app.handle_pty_output(b"xLxLC-left").unwrap();
        let (left_inactive_match, left_current) =
            install_pane_copy_presentation_for_test(&mut app, "x", 4);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"RxRxRC-right").unwrap();
        let (right_inactive_match, right_current) =
            install_pane_copy_presentation_for_test(&mut app, "x", 5);

        let left = pane_rect_for_test(&app, rterm_core::PaneId::new(1));
        let right = pane_rect_for_test(&app, rterm_core::PaneId::new(2));
        let snapshot = app.render_snapshot();
        assert_snapshot_cell_background(
            &snapshot,
            left.row,
            left.column.saturating_add(4),
            PANE_OVERLAY_COPY_ACTIVE_BG,
            "inactive left Copy selection",
        );
        assert_snapshot_cell_background(
            &snapshot,
            right.row,
            right.column.saturating_add(5),
            PANE_OVERLAY_COPY_ACTIVE_BG,
            "active right Copy selection",
        );
        assert_snapshot_cell_background(
            &snapshot,
            left.row,
            left.column.saturating_add(left_inactive_match.start_column),
            PANE_OVERLAY_COPY_INACTIVE_BG,
            "inactive left non-current search match",
        );
        assert_snapshot_cell_background(
            &snapshot,
            right.row,
            right
                .column
                .saturating_add(right_inactive_match.start_column),
            PANE_OVERLAY_COPY_INACTIVE_BG,
            "active right non-current search match",
        );
        assert_ne!(
            snapshot_cell(
                &snapshot,
                left.row,
                left.column.saturating_add(left_current.start_column),
            )
            .expect("left retained current")
            .background,
            PANE_OVERLAY_COPY_INACTIVE_BG,
            "retained current is not an inactive match"
        );
        assert_ne!(
            snapshot_cell(
                &snapshot,
                right.row,
                right.column.saturating_add(right_current.start_column),
            )
            .expect("right retained current")
            .background,
            PANE_OVERLAY_COPY_INACTIVE_BG,
            "retained current is not an inactive match"
        );
        assert_ne!(
            snapshot_cell(&app.snapshot, 0, 5)
                .expect("active Copy base snapshot")
                .background,
            PANE_OVERLAY_COPY_ACTIVE_BG,
            "active cached snapshot must remain selection-free"
        );
    }

    #[test]
    fn window_app_visible_split_panes_render_distinct_quick_overlays() {
        let mut app = NativeWindowApp::new(None);
        let inactive_hsb = NativeInactivePaneHsb {
            hue: NativeHsbMultiplier::from_f32(1.0),
            saturation: NativeHsbMultiplier::from_f32(1.0),
            brightness: NativeHsbMultiplier::from_f32(0.5),
        };
        configure_pane_overlay_presentation_test(&mut app, inactive_hsb);
        app.runtime.resize(rterm_core::TerminalSize::new(24, 4));
        app.handle_pty_output(b"\x1b[31;1mLEFT-quick-owner\x1b[0m\x1b[1;2H")
            .unwrap();
        let left_match = pane_overlay_test_match(&app, 0, 1, 5);
        install_pane_quick_presentation_for_test(&mut app, "L", "left", left_match);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"\x1b[32;1mRIGHT-quick-owner\x1b[0m\x1b[1;3H")
            .unwrap();
        let right_match = pane_overlay_test_match(&app, 0, 2, 6);
        install_pane_quick_presentation_for_test(&mut app, "R", "right", right_match);

        let left = pane_rect_for_test(&app, rterm_core::PaneId::new(1));
        let right = pane_rect_for_test(&app, rterm_core::PaneId::new(2));
        let snapshot = app.render_snapshot();
        let cursor = snapshot.cursor().expect("active owner cursor");
        assert_eq!(
            (cursor.row, cursor.column),
            (right.row, right.column.saturating_add(2)),
            "active-first composition must retain and offset only the active pane cursor"
        );
        assert_ne!(
            (cursor.row, cursor.column),
            (left.row, left.column.saturating_add(1)),
            "inactive pane cursor must not replace the active owner cursor"
        );
        assert_eq!(
            snapshot_char(&snapshot, left.row, left.column.saturating_add(1)).unwrap_or(' '),
            'L',
            "left owner label"
        );
        assert_eq!(
            snapshot_char(&snapshot, right.row, right.column.saturating_add(2)).unwrap_or(' '),
            'R',
            "right owner label"
        );
        assert_snapshot_cell_background(
            &snapshot,
            left.row,
            left.column.saturating_add(1),
            super::inactive_pane_color(
                rterm_render_core::RenderCellColorRole::Background,
                PANE_OVERLAY_QUICK_LABEL_BG,
                inactive_hsb,
                DEFAULT_FOREGROUND_COLOR,
                DEFAULT_BACKGROUND_COLOR,
            ),
            "inactive left Quick label",
        );
        assert_snapshot_cell_background(
            &snapshot,
            right.row,
            right.column.saturating_add(2),
            PANE_OVERLAY_QUICK_LABEL_BG,
            "active right Quick label",
        );
        assert_snapshot_cell_background(
            &snapshot,
            left.row,
            left.column.saturating_add(3),
            super::inactive_pane_color(
                rterm_render_core::RenderCellColorRole::Background,
                PANE_OVERLAY_QUICK_MATCH_BG,
                inactive_hsb,
                DEFAULT_FOREGROUND_COLOR,
                DEFAULT_BACKGROUND_COLOR,
            ),
            "inactive left Quick match",
        );
        assert_snapshot_cell_background(
            &snapshot,
            right.row,
            right.column.saturating_add(4),
            PANE_OVERLAY_QUICK_MATCH_BG,
            "active right Quick match",
        );
        assert_ne!(
            snapshot_cell(&snapshot, left.row, left.column.saturating_add(1))
                .expect("inactive left Quick label")
                .background,
            PANE_OVERLAY_QUICK_LABEL_BG,
            "inactive HSB must transform owner labels exactly in pane presentation"
        );
        assert!(
            !snapshot_cell(&snapshot, left.row, left.column.saturating_add(3))
                .expect("left Quick styled cell")
                .bold,
            "Quick remove_styling applies to inactive owner presentation"
        );
        assert!(
            !snapshot_cell(&snapshot, right.row, right.column.saturating_add(4))
                .expect("right Quick styled cell")
                .bold,
            "Quick remove_styling applies to active owner presentation"
        );
        assert!(
            snapshot_cell(&app.snapshot, 0, 4)
                .expect("active Quick base snapshot")
                .bold,
            "active cached snapshot must retain terminal styling"
        );
        assert!(app.effective_window_title().contains("owner-right"));
        assert!(!app.effective_window_title().contains("owner-left"));
    }

    #[test]
    fn window_app_inactive_quick_labels_use_owner_pane_row_and_column_offsets() {
        let mut app = NativeWindowApp::new(None);
        configure_pane_overlay_presentation_test(&mut app, pane_overlay_identity_hsb());
        app.runtime.resize(rterm_core::TerminalSize::new(30, 8));
        app.refresh_snapshot();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"\r\nRIGHT-pane").unwrap();
        let right_match = pane_overlay_test_match(&app, 1, 2, 4);
        install_pane_quick_presentation_for_test(&mut app, "R", "right-offset", right_match);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(2),
            direction: SplitDirection::Down,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"\r\nBOT-pane").unwrap();
        let bottom_match = pane_overlay_test_match(&app, 1, 3, 5);
        install_pane_quick_presentation_for_test(&mut app, "B", "bottom-offset", bottom_match);
        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rterm_core::PaneId::new(1),
        })
        .unwrap();

        let right = pane_rect_for_test(&app, rterm_core::PaneId::new(2));
        let bottom = pane_rect_for_test(&app, rterm_core::PaneId::new(3));
        assert!(right.column > 0, "right fixture must have a column offset");
        assert!(
            bottom.row > app.terminal_frame_row_offset() && bottom.column > 0,
            "nested bottom-right fixture must have both row and column offsets"
        );
        let snapshot = app.render_snapshot();
        assert_eq!(
            snapshot_char(
                &snapshot,
                right.row.saturating_add(1),
                right.column.saturating_add(2),
            )
            .unwrap_or(' '),
            'R'
        );
        assert_eq!(
            snapshot_char(
                &snapshot,
                bottom.row.saturating_add(1),
                bottom.column.saturating_add(3),
            )
            .unwrap_or(' '),
            'B'
        );
        assert_ne!(
            snapshot_char(&snapshot, right.row.saturating_add(1), 2).unwrap_or(' '),
            'R',
            "right label must not use a window-origin column"
        );
        assert_ne!(
            snapshot_char(
                &snapshot,
                app.terminal_frame_row_offset().saturating_add(1),
                bottom.column.saturating_add(3),
            )
            .unwrap_or(' '),
            'B',
            "bottom label must not use a window-origin row"
        );
    }

    #[test]
    fn window_app_quick_labels_are_clipped_to_owner_pane_rect() {
        let mut app = NativeWindowApp::new(None);
        configure_pane_overlay_presentation_test(&mut app, pane_overlay_identity_hsb());
        app.runtime.resize(rterm_core::TerminalSize::new(16, 4));
        app.handle_pty_output(b"LLLLLLLLLLLL").unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"NNNNNNNNNNNN").unwrap();
        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rterm_core::PaneId::new(1),
        })
        .unwrap();
        let left = pane_rect_for_test(&app, rterm_core::PaneId::new(1));
        let local_last_column = left.columns.saturating_sub(1);
        let matched = pane_overlay_test_match(&app, 0, local_last_column, local_last_column);
        install_pane_quick_presentation_for_test(&mut app, "WXYZ", "clip", matched);
        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rterm_core::PaneId::new(2),
        })
        .unwrap();

        let layout = app.pane_render_layout();
        let left = pane_rect_for_test(&app, rterm_core::PaneId::new(1));
        let right = pane_rect_for_test(&app, rterm_core::PaneId::new(2));
        let separator = layout.separators.first().copied().expect("split separator");
        let snapshot = app.render_snapshot();
        assert_eq!(
            snapshot_char(
                &snapshot,
                left.row,
                left.column.saturating_add(left.columns.saturating_sub(1)),
            )
            .unwrap_or(' '),
            'W',
            "first label cell remains in owner rect"
        );
        assert_eq!(
            snapshot_char(&snapshot, separator.row, separator.column).unwrap_or(' '),
            '|',
            "long Quick label must not overwrite separator"
        );
        assert_eq!(
            snapshot_char(&snapshot, right.row, right.column).unwrap_or(' '),
            'N',
            "long Quick label must not overwrite neighboring pane"
        );
        assert_snapshot_cell_background(
            &snapshot,
            left.row,
            left.column.saturating_add(left.columns.saturating_sub(1)),
            PANE_OVERLAY_QUICK_LABEL_BG,
            "clipped owner label cell",
        );
        assert_ne!(
            snapshot_cell(&snapshot, separator.row, separator.column)
                .expect("separator cell")
                .background,
            PANE_OVERLAY_QUICK_LABEL_BG
        );
        assert_ne!(
            snapshot_cell(&snapshot, right.row, right.column)
                .expect("neighbor cell")
                .background,
            PANE_OVERLAY_QUICK_LABEL_BG
        );
        app.mouse_position = Some((
            left.column.saturating_add(left.columns.saturating_sub(1)),
            left.row.saturating_sub(app.terminal_frame_row_offset()),
        ));
        assert!(
            app.pane_close_button_at_mouse_position().is_none(),
            "a visible Quick label must take hit priority over the pane close button"
        );
    }

    #[test]
    fn window_app_applies_inactive_pane_hsb_to_split_render_snapshot() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            inactive_pane_hsb: Some(NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(1.0),
                brightness: NativeHsbMultiplier::from_f32(0.5),
            }),
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
        app.handle_pane_pty_output(
            rterm_core::PaneId::new(1),
            b"\x1b[38;2;100;150;200;48;2;20;40;60mI\x1b[0m",
        )
        .unwrap();
        app.handle_pty_output(b"\x1b[38;2;100;150;200mA\x1b[0m")
            .unwrap();

        let layout = app.pane_render_layout();
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let inactive_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == rterm_core::PaneId::new(1))
            .expect("inactive pane rect");
        let snapshot = app.render_snapshot();
        let inactive_cell = snapshot_cell(&snapshot, inactive_rect.row, inactive_rect.column)
            .expect("inactive cell");
        let active_cell =
            snapshot_cell(&snapshot, active_rect.row, active_rect.column).expect("active cell");

        assert_eq!(
            active_cell.foreground,
            rterm_terminal::Color::Rgb(100, 150, 200)
        );
        assert_eq!(
            inactive_cell.foreground,
            rterm_terminal::Color::Rgb(50, 75, 100)
        );
        assert_eq!(
            inactive_cell.background,
            rterm_terminal::Color::Rgb(10, 20, 30)
        );
    }

    #[test]
    fn window_app_renders_active_and_inactive_pane_selections_together() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            selection_bg_color: Some(Color::Rgb(100, 120, 140)),
            inactive_pane_hsb: Some(NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(1.0),
                brightness: NativeHsbMultiplier::from_f32(0.5),
            }),
            ..NativeConfigSnapshot::default()
        });
        app.runtime.resize(rterm_core::TerminalSize::new(20, 4));
        app.handle_pty_output(b"A").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 0 },
        );
        app.refresh_snapshot();

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"B").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 0 },
        );
        app.refresh_snapshot();

        let layout = app.pane_render_layout();
        let inactive_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == rterm_core::PaneId::new(1))
            .expect("inactive pane rect");
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let snapshot = app.render_snapshot();
        let inactive_cell = snapshot_cell(&snapshot, inactive_rect.row, inactive_rect.column)
            .expect("inactive selected cell");
        let active_cell =
            snapshot_cell(&snapshot, active_rect.row, active_rect.column).expect("active cell");

        assert_eq!(
            active_cell.background,
            rterm_terminal::Color::Rgb(100, 120, 140)
        );
        assert_eq!(
            inactive_cell.background,
            rterm_terminal::Color::Rgb(50, 60, 70)
        );
    }

    #[test]
    fn window_app_keeps_inactive_selection_after_unselected_inactive_pty_output() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            selection_bg_color: Some(Color::Rgb(90, 110, 130)),
            inactive_pane_hsb: Some(NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(1.0),
                brightness: NativeHsbMultiplier::from_f32(1.0),
            }),
            ..NativeConfigSnapshot::default()
        });
        app.runtime.resize(rterm_core::TerminalSize::new(20, 4));
        app.handle_pty_output(b"A").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 0 },
        );
        app.refresh_snapshot();

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pane_pty_output(rterm_core::PaneId::new(1), b"\x1b[2;1HZ")
            .unwrap();

        let layout = app.pane_render_layout();
        let inactive_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == rterm_core::PaneId::new(1))
            .expect("inactive pane rect");
        let snapshot = app.render_snapshot();
        let inactive_cell = snapshot_cell(&snapshot, inactive_rect.row, inactive_rect.column)
            .expect("inactive selected cell");

        assert_eq!(
            inactive_cell.background,
            rterm_terminal::Color::Rgb(90, 110, 130)
        );
    }

    #[test]
    fn window_app_single_pane_applies_translucent_selection_once() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            selection_bg_color: Some(Color::Rgba(100, 120, 140, 128)),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[48;2;20;40;60mA").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 0 },
        );
        app.refresh_snapshot();

        let snapshot = app.render_snapshot();
        let selected_cell =
            snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("selected terminal cell");

        assert_eq!(
            selected_cell.background,
            rterm_terminal::Color::Rgb(60, 80, 100)
        );
    }

    #[test]
    fn window_app_applies_inactive_pane_hsb_to_indexed_and_default_colors() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            inactive_pane_hsb: Some(NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(1.0),
                brightness: NativeHsbMultiplier::from_f32(0.5),
            }),
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
        app.handle_pane_pty_output(rterm_core::PaneId::new(1), b"\x1b[31mI\x1b[0m")
            .unwrap();
        app.handle_pty_output(b"\x1b[31mA\x1b[0m").unwrap();

        let layout = app.pane_render_layout();
        let inactive_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == rterm_core::PaneId::new(1))
            .expect("inactive pane rect");
        let active_rect = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        let snapshot = app.render_snapshot();
        let inactive_cell = snapshot_cell(&snapshot, inactive_rect.row, inactive_rect.column)
            .expect("inactive cell");
        let active_cell =
            snapshot_cell(&snapshot, active_rect.row, active_rect.column).expect("active cell");

        assert_eq!(active_cell.foreground, rterm_terminal::Color::Indexed(1));
        assert_eq!(
            inactive_cell.foreground,
            rterm_terminal::Color::Rgb(103, 25, 25)
        );
        assert_eq!(active_cell.background, rterm_terminal::Color::Default);
        assert_eq!(inactive_cell.background, rterm_terminal::Color::Rgb(6, 6, 6));
    }

    fn sample_ansi_palette() -> [Color; 16] {
        let mut palette = DEFAULT_ANSI_PALETTE_COLORS;
        palette[1] = Color::Rgb(31, 32, 33);
        palette[9] = Color::Rgb(41, 42, 43);
        palette
    }

    fn sample_indexed_palette() -> [Option<Color>; 256] {
        let mut palette = [None; 256];
        palette[136] = Some(Color::Rgb(51, 52, 53));
        palette
    }

    fn sample_palette() -> NativePalette {
        let (ansi, brights) = super::native_split_ansi_palette(sample_ansi_palette());
        NativePalette {
            foreground: Some(Color::Rgb(7, 8, 9)),
            background: Some(Color::Rgb(4, 5, 6)),
            cursor_fg: Some(Color::Rgb(13, 14, 15)),
            cursor_bg: Some(Color::Rgb(10, 11, 12)),
            cursor_border: Some(Color::Rgb(16, 17, 18)),
            selection_fg: Some(Some(Color::Rgb(61, 62, 63))),
            selection_bg: Some(Color::Rgb(71, 72, 73)),
            ansi: Some(ansi),
            brights: Some(brights),
            indexed: sample_indexed_palette(),
            tab_bar_background: Some(Color::Rgb(25, 26, 27)),
            tab_bar_inactive_tab_edge: Some(Color::Rgb(27, 28, 29)),
            tab_bar_active_tab: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(28, 29, 30)),
                bg_color: Some(Color::Rgb(31, 32, 33)),
                intensity: Some(NativeFormatIntensity::Bold),
                ..Default::default()
            },
            tab_bar_inactive_tab: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(34, 35, 36)),
                bg_color: Some(Color::Rgb(37, 38, 39)),
                ..Default::default()
            },
            tab_bar_inactive_tab_hover: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(46, 47, 48)),
                bg_color: Some(Color::Rgb(49, 50, 51)),
                ..Default::default()
            },
            tab_bar_new_tab: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(40, 41, 42)),
                bg_color: Some(Color::Rgb(43, 44, 45)),
                ..Default::default()
            },
            tab_bar_new_tab_hover: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(52, 53, 54)),
                bg_color: Some(Color::Rgb(55, 56, 57)),
                ..Default::default()
            },
            scrollbar_thumb: Some(Color::Rgb(22, 23, 24)),
            split: Some(Color::Rgb(19, 20, 21)),
            visual_bell: Some(Color::Rgb(1, 2, 3)),
            compose_cursor: Some(Color::Rgb(22, 23, 24)),
            copy_mode_active_highlight_fg: Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Black)),
            copy_mode_active_highlight_bg: Some(NativeColorSpec::Color(Color::Rgb(21, 22, 23))),
            copy_mode_inactive_highlight_fg: Some(NativeColorSpec::AnsiColor(
                NativeAnsiColor::White,
            )),
            copy_mode_inactive_highlight_bg: Some(NativeColorSpec::Color(Color::Rgb(24, 25, 26))),
            quick_select_label_fg: Some(NativeColorSpec::Color(Color::Rgb(30, 31, 32))),
            quick_select_label_bg: Some(NativeColorSpec::Color(Color::Rgb(27, 28, 29))),
            quick_select_match_fg: Some(NativeColorSpec::Color(Color::Rgb(33, 34, 35))),
            quick_select_match_bg: Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Navy)),
            input_selector_label_fg: Some(NativeColorSpec::Color(Color::Rgb(37, 38, 39))),
            input_selector_label_bg: Some(NativeColorSpec::Color(Color::Rgb(34, 35, 36))),
            launcher_label_fg: Some(NativeColorSpec::Color(Color::Rgb(40, 41, 42))),
            launcher_label_bg: Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Black)),
        }
    }

    fn sample_resolved_palette() -> NativeResolvedPalette {
        let (ansi, brights) = super::native_split_ansi_palette(sample_ansi_palette());
        NativeResolvedPalette {
            foreground: Color::Rgb(7, 8, 9),
            background: Color::Rgb(4, 5, 6),
            cursor_fg: Some(Color::Rgb(13, 14, 15)),
            cursor_bg: Color::Rgb(10, 11, 12),
            cursor_border: Some(Color::Rgb(16, 17, 18)),
            selection_fg: Some(Some(Color::Rgb(61, 62, 63))),
            selection_bg: Some(Color::Rgb(71, 72, 73)),
            ansi,
            brights,
            indexed: sample_indexed_palette(),
            tab_bar_background: Some(Color::Rgb(25, 26, 27)),
            tab_bar_inactive_tab_edge: Some(Color::Rgb(27, 28, 29)),
            tab_bar_active_tab: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(28, 29, 30)),
                bg_color: Some(Color::Rgb(31, 32, 33)),
                intensity: Some(NativeFormatIntensity::Bold),
                ..Default::default()
            },
            tab_bar_inactive_tab: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(34, 35, 36)),
                bg_color: Some(Color::Rgb(37, 38, 39)),
                ..Default::default()
            },
            tab_bar_inactive_tab_hover: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(46, 47, 48)),
                bg_color: Some(Color::Rgb(49, 50, 51)),
                ..Default::default()
            },
            tab_bar_new_tab: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(40, 41, 42)),
                bg_color: Some(Color::Rgb(43, 44, 45)),
                ..Default::default()
            },
            tab_bar_new_tab_hover: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(52, 53, 54)),
                bg_color: Some(Color::Rgb(55, 56, 57)),
                ..Default::default()
            },
            scrollbar_thumb: Some(Color::Rgb(22, 23, 24)),
            split: Some(Color::Rgb(19, 20, 21)),
            visual_bell: Some(Color::Rgb(1, 2, 3)),
            compose_cursor: Some(Color::Rgb(22, 23, 24)),
            copy_mode_active_highlight_fg: Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Black)),
            copy_mode_active_highlight_bg: Some(NativeColorSpec::Color(Color::Rgb(21, 22, 23))),
            copy_mode_inactive_highlight_fg: Some(NativeColorSpec::AnsiColor(
                NativeAnsiColor::White,
            )),
            copy_mode_inactive_highlight_bg: Some(NativeColorSpec::Color(Color::Rgb(24, 25, 26))),
            quick_select_label_fg: Some(NativeColorSpec::Color(Color::Rgb(30, 31, 32))),
            quick_select_label_bg: Some(NativeColorSpec::Color(Color::Rgb(27, 28, 29))),
            quick_select_match_fg: Some(NativeColorSpec::Color(Color::Rgb(33, 34, 35))),
            quick_select_match_bg: Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Navy)),
            input_selector_label_fg: Some(NativeColorSpec::Color(Color::Rgb(37, 38, 39))),
            input_selector_label_bg: Some(NativeColorSpec::Color(Color::Rgb(34, 35, 36))),
            launcher_label_fg: Some(NativeColorSpec::Color(Color::Rgb(40, 41, 42))),
            launcher_label_bg: Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Black)),
        }
    }

    fn sample_color_schemes() -> HashMap<String, NativeResolvedPalette> {
        HashMap::from([("Project Scheme".to_owned(), sample_resolved_palette())])
    }

    fn sample_window_frame_appearance() -> NativeWindowFrameAppearance {
        NativeWindowFrameAppearance {
            inactive_titlebar_bg: Some(Color::Rgb(1, 2, 3)),
            active_titlebar_bg: Some(Color::Rgb(4, 5, 6)),
            inactive_titlebar_fg: Some(Color::Rgb(7, 8, 9)),
            active_titlebar_fg: Some(Color::Rgb(10, 11, 12)),
            inactive_titlebar_border_bottom: Some(Color::Rgb(13, 14, 15)),
            active_titlebar_border_bottom: Some(Color::Rgb(16, 17, 18)),
            button_fg: Some(Color::Rgb(19, 20, 21)),
            button_bg: Some(Color::Rgb(22, 23, 24)),
            button_hover_fg: Some(Color::Rgb(25, 26, 27)),
            button_hover_bg: Some(Color::Rgb(28, 29, 30)),
            border_left_width: Some(NativeWindowPaddingDimension::Pixels(3)),
            border_right_width: Some(NativeWindowPaddingDimension::Pixels(4)),
            border_top_height: Some(NativeWindowPaddingDimension::Pixels(5)),
            border_bottom_height: Some(NativeWindowPaddingDimension::Pixels(6)),
            border_left_color: Some(Color::Rgb(31, 32, 33)),
            border_right_color: Some(Color::Rgb(34, 35, 36)),
            border_top_color: Some(Color::Rgb(37, 38, 39)),
            border_bottom_color: Some(Color::Rgb(40, 41, 42)),
            font: Some("Monaco".to_owned()),
            font_size: Some(NativeFontSize::from_millipoints(13_000)),
        }
    }

    fn default_effective_config() -> NativeConfigView {
        let resolved_palette = NativeResolvedPalette {
            foreground: super::LEGACY_TEST_FOREGROUND_COLOR,
            background: super::LEGACY_TEST_BACKGROUND_COLOR,
            cursor_fg: super::LEGACY_TEST_CURSOR_FG_COLOR,
            cursor_bg: super::LEGACY_TEST_CURSOR_BG_COLOR,
            ..NativeResolvedPalette::default()
        };

        native_config_view! {
            dpi: super::DEFAULT_WINDOW_DPI,
            dpi_by_screen: BTreeMap::new(),
            tab_max_width: 16,
            tab_min_width: 8,
            status_update_interval: 1_000,
            status_update_interval_ms: 1_000,
            max_fps: DEFAULT_MAX_FPS,
            animation_fps: DEFAULT_ANIMATION_FPS,
            front_end: DEFAULT_RENDER_FRONT_END,
            webgpu_power_preference: DEFAULT_WEBGPU_POWER_PREFERENCE,
            webgpu_force_fallback_adapter: DEFAULT_WEBGPU_FORCE_FALLBACK_ADAPTER,
            webgpu_preferred_adapter: None,
            prefer_egl: DEFAULT_PREFER_EGL,
            enable_wayland: DEFAULT_ENABLE_WAYLAND,
            enable_zwlr_output_manager: DEFAULT_ENABLE_ZWLR_OUTPUT_MANAGER,
            use_box_model_render: DEFAULT_USE_BOX_MODEL_RENDER,
            experimental_pixel_positioning: DEFAULT_EXPERIMENTAL_PIXEL_POSITIONING,
            shape_cache_size: DEFAULT_SHAPE_CACHE_SIZE,
            line_state_cache_size: DEFAULT_LINE_STATE_CACHE_SIZE,
            line_quad_cache_size: DEFAULT_LINE_QUAD_CACHE_SIZE,
            line_to_ele_shape_cache_size: DEFAULT_LINE_TO_ELE_SHAPE_CACHE_SIZE,
            glyph_cache_image_cache_size: DEFAULT_GLYPH_CACHE_IMAGE_CACHE_SIZE,
            cursor_blink_rate: 800,
            cursor_blink_rate_ms: 800,
            cursor_blink_ease_in: NativeEasingFunction::Linear,
            cursor_blink_ease_out: NativeEasingFunction::Linear,
            text_blink_rate: 500,
            text_blink_rate_ms: 500,
            text_blink_rate_rapid: 250,
            text_blink_rate_rapid_ms: 250,
            text_blink_ease_in: NativeEasingFunction::Linear,
            text_blink_ease_out: NativeEasingFunction::Linear,
            text_blink_rapid_ease_in: NativeEasingFunction::Linear,
            text_blink_rapid_ease_out: NativeEasingFunction::Linear,
            font: None,
            font_fallbacks: Vec::new(),
            font_attributes: NativeFontAttributes::default(),
            font_rules: Vec::new(),
            font_size: DEFAULT_FONT_SIZE,
            cell_width: DEFAULT_CELL_WIDTH,
            cell_widths: Vec::new(),
            line_height: DEFAULT_LINE_HEIGHT,
            font_antialias: DEFAULT_FONT_ANTIALIAS,
            font_hinting: DEFAULT_FONT_HINTING,
            font_rasterizer: DEFAULT_FONT_RASTERIZER,
            font_colr_rasterizer: DEFAULT_FONT_COLR_RASTERIZER,
            font_shaper: DEFAULT_FONT_SHAPER,
            harfbuzz_features: Vec::new(),
            font_dirs: Vec::new(),
            font_locator: DEFAULT_FONT_LOCATOR,
            use_cap_height_to_scale_fallback_fonts: DEFAULT_USE_CAP_HEIGHT_TO_SCALE_FALLBACK_FONTS,
            ignore_svg_fonts: DEFAULT_IGNORE_SVG_FONTS,
            sort_fallback_fonts_by_coverage: DEFAULT_SORT_FALLBACK_FONTS_BY_COVERAGE,
            search_font_dirs_for_fallback: DEFAULT_SEARCH_FONT_DIRS_FOR_FALLBACK,
            custom_block_glyphs: DEFAULT_CUSTOM_BLOCK_GLYPHS,
            anti_alias_custom_block_glyphs: DEFAULT_ANTI_ALIAS_CUSTOM_BLOCK_GLYPHS,
            allow_square_glyphs_to_overflow_width: DEFAULT_ALLOW_SQUARE_GLYPHS_TO_OVERFLOW_WIDTH,
            freetype_load_target: DEFAULT_FREETYPE_LOAD_TARGET,
            freetype_render_target: DEFAULT_FREETYPE_LOAD_TARGET,
            freetype_load_flags: NativeFreetypeLoadFlags::DEFAULT,
            freetype_interpreter_version: None,
            freetype_pcf_long_family_names: DEFAULT_FREETYPE_PCF_LONG_FAMILY_NAMES,
            display_pixel_geometry: DEFAULT_DISPLAY_PIXEL_GEOMETRY,
            foreground_text_hsb: DEFAULT_FOREGROUND_TEXT_HSB,
            bold_brightens_ansi_colors: DEFAULT_BOLD_BRIGHTENS_ANSI_COLORS,
            text_background_opacity: DEFAULT_TEXT_BACKGROUND_OPACITY,
            window_background_opacity: DEFAULT_WINDOW_BACKGROUND_OPACITY,
            background: Vec::new(),
            window_background_image: None,
            window_background_image_hsb: None,
            window_background_gradient: None,
            window_background_images: Vec::new(),
            window_background_layers: Vec::new(),
            kde_window_background_blur: false,
            macos_window_background_blur: DEFAULT_MACOS_WINDOW_BACKGROUND_BLUR,
            win32_system_backdrop: DEFAULT_WIN32_SYSTEM_BACKDROP,
            win32_acrylic_accent_color: None,
            window_decorations: DEFAULT_WINDOW_DECORATIONS,
            window_frame: NativeWindowFrameAppearance::default(),
            window_frame_appearance: NativeWindowFrameAppearance::default(),
            integrated_title_buttons: default_integrated_title_buttons(),
            integrated_title_button_alignment: DEFAULT_INTEGRATED_TITLE_BUTTON_ALIGNMENT,
            integrated_title_button_color: DEFAULT_INTEGRATED_TITLE_BUTTON_COLOR,
            integrated_title_button_style: DEFAULT_INTEGRATED_TITLE_BUTTON_STYLE,
            default_cursor_style: NativeCursorStyle::SteadyBlock,
            cursor_thickness: None,
            underline_thickness: DEFAULT_UNDERLINE_THICKNESS,
            underline_position: DEFAULT_UNDERLINE_POSITION,
            strikethrough_position: DEFAULT_STRIKETHROUGH_POSITION,
            force_reverse_video_cursor: DEFAULT_FORCE_REVERSE_VIDEO_CURSOR,
            reverse_video_cursor_min_contrast: DEFAULT_REVERSE_VIDEO_CURSOR_MIN_CONTRAST,
            text_min_contrast_ratio: None,
            window_padding: NativeWindowPadding::default(),
            window_content_alignment: DEFAULT_WINDOW_CONTENT_ALIGNMENT,
            initial_cols: TERMINAL_COLUMNS,
            initial_rows: TERMINAL_ROWS,
            inactive_pane_hsb: DEFAULT_INACTIVE_PANE_HSB,
            command_palette_rows: None,
            command_palette_font: Some(super::native_font_config(super::DEFAULT_WINDOW_FRAME_FONT)),
            command_palette_font_size: DEFAULT_COMMAND_PALETTE_FONT_SIZE,
            command_palette_bg_color: Some(DEFAULT_COMMAND_PALETTE_BG_COLOR),
            command_palette_fg_color: Some(DEFAULT_COMMAND_PALETTE_FG_COLOR),
            char_select_font: Some(super::native_font_config(super::DEFAULT_WINDOW_FRAME_FONT)),
            char_select_font_size: DEFAULT_CHAR_SELECT_FONT_SIZE,
            char_select_bg_color: Some(DEFAULT_CHAR_SELECT_BG_COLOR),
            char_select_fg_color: Some(DEFAULT_CHAR_SELECT_FG_COLOR),
            pane_select_font: Some(super::native_font_config(super::DEFAULT_WINDOW_FRAME_FONT)),
            pane_select_font_size: DEFAULT_PANE_SELECT_FONT_SIZE,
            pane_select_bg_color: Some(DEFAULT_PANE_SELECT_BG_COLOR),
            pane_select_fg_color: Some(DEFAULT_PANE_SELECT_FG_COLOR),
            launcher_alphabet: DEFAULT_LAUNCHER_ALPHABET.to_owned(),
            quick_select_alphabet: DEFAULT_QUICK_SELECT_ALPHABET.to_owned(),
            quick_select_patterns: Vec::new(),
            disable_default_quick_select_patterns: false,
            quick_select_remove_styling: false,
            hyperlink_rules: default_hyperlink_rules(),
            copy_mode_active_highlight_bg: None,
            copy_mode_active_highlight_fg: None,
            copy_mode_inactive_highlight_bg: None,
            copy_mode_inactive_highlight_fg: None,
            quick_select_label_bg: None,
            quick_select_label_fg: None,
            quick_select_match_bg: None,
            quick_select_match_fg: None,
            input_selector_label_bg: None,
            input_selector_label_fg: None,
            launcher_label_bg: None,
            launcher_label_fg: None,
            selection_word_boundary: DEFAULT_SELECTION_WORD_BOUNDARY.to_owned(),
            term: "xterm-256color".to_owned(),
            enq_answerback: DEFAULT_ENQ_ANSWERBACK.to_owned(),
            audible_bell: NativeAudibleBell::SystemBeep,
            visual_bell: NativeVisualBell::default(),
            colors: None,
            color_scheme: None,
            color_scheme_dirs: Vec::new(),
            color_schemes: HashMap::new(),
            resolved_palette: resolved_palette,
            foreground_color: super::LEGACY_TEST_FOREGROUND_COLOR,
            background_color: super::LEGACY_TEST_BACKGROUND_COLOR,
            ansi_palette: None,
            indexed_palette: None,
            selection_fg_color: None,
            selection_bg_color: None,
            cursor_bg_color: super::LEGACY_TEST_CURSOR_BG_COLOR,
            cursor_border_color: None,
            cursor_fg_color: None,
            compose_cursor_color: None,
            split_color: None,
            scrollbar_thumb_color: None,
            tab_bar_background_color: None,
            tab_bar_inactive_tab_edge_color: None,
            tab_bar_active_tab_colors: NativeTabBarItemColors::default(),
            tab_bar_inactive_tab_colors: NativeTabBarItemColors::default(),
            tab_bar_inactive_tab_hover_colors: NativeTabBarItemColors::default(),
            tab_bar_new_tab_colors: NativeTabBarItemColors::default(),
            tab_bar_new_tab_hover_colors: NativeTabBarItemColors::default(),
            tab_bar_style: NativeTabBarStyle::default(),
            visual_bell_color: None,
            notification_handling: DEFAULT_NOTIFICATION_HANDLING,
            default_prog: None,
            default_gui_startup_args: default_gui_startup_args(),
            default_domain: "local".to_owned(),
            default_workspace: "default".to_owned(),
            prefer_to_spawn_tabs: super::DEFAULT_PREFER_TO_SPAWN_TABS,
            automatically_reload_config: DEFAULT_AUTOMATICALLY_RELOAD_CONFIG,
            check_for_updates: DEFAULT_CHECK_FOR_UPDATES,
            check_for_updates_interval_seconds: DEFAULT_CHECK_FOR_UPDATES_INTERVAL_SECONDS,
            show_update_window: DEFAULT_SHOW_UPDATE_WINDOW,
            native_macos_fullscreen_mode: DEFAULT_NATIVE_MACOS_FULLSCREEN_MODE,
            macos_fullscreen_extend_behind_notch: DEFAULT_MACOS_FULLSCREEN_EXTEND_BEHIND_NOTCH,
            use_resize_increments: DEFAULT_USE_RESIZE_INCREMENTS,
            debug_key_events: DEFAULT_DEBUG_KEY_EVENTS,
            log_unknown_escape_sequences: DEFAULT_LOG_UNKNOWN_ESCAPE_SEQUENCES,
            warn_about_missing_glyphs: DEFAULT_WARN_ABOUT_MISSING_GLYPHS,
            default_cwd: None,
            default_ssh_auth_sock: None,
            default_mux_server_domain: None,
            daemon_options: NativeDaemonOptions::default(),
            exec_domains: Vec::new(),
            wsl_domains: Vec::new(),
            unix_domains: default_native_unix_domains(),
            ssh_domains: Vec::new(),
            tls_servers: Vec::new(),
            tls_clients: Vec::new(),
            serial_ports: Vec::new(),
            mux_enable_ssh_agent: DEFAULT_MUX_ENABLE_SSH_AGENT,
            ssh_backend: NativeSshBackend::LibSsh,
            ratelimit_mux_line_prefetches_per_second:
                DEFAULT_RATELIMIT_MUX_LINE_PREFETCHES_PER_SECOND,
            mux_output_parser_buffer_size: DEFAULT_MUX_OUTPUT_PARSER_BUFFER_SIZE,
            mux_output_parser_coalesce_delay_ms: DEFAULT_MUX_OUTPUT_PARSER_COALESCE_DELAY_MS,
            periodic_stat_logging: DEFAULT_PERIODIC_STAT_LOGGING,
            ulimit_nofile: DEFAULT_ULIMIT_NOFILE,
            ulimit_nproc: DEFAULT_ULIMIT_NPROC,
            mux_env_remove: default_mux_env_remove(),
            tiling_desktop_environments: default_tiling_desktop_environments(),
            set_environment_variables: BTreeMap::new(),
            launch_menu: Vec::new(),
            leader: None,
            keys: Vec::new(),
            key_tables: BTreeMap::new(),
            mouse_bindings: Vec::new(),
            key_map_preference: NativeKeyMapPreference::Mapped,
            ui_key_cap_rendering: super::DEFAULT_UI_KEY_CAP_RENDERING,
            swap_backspace_and_delete: false,
            enable_kitty_graphics: DEFAULT_ENABLE_KITTY_GRAPHICS,
            enable_checksum_rectangular_area: DEFAULT_ENABLE_CHECKSUM_RECTANGULAR_AREA,
            enable_title_reporting: DEFAULT_ENABLE_TITLE_REPORTING,
            enable_csi_u_key_encoding: DEFAULT_ENABLE_CSI_U_KEY_ENCODING,
            enable_kitty_keyboard: DEFAULT_ENABLE_KITTY_KEYBOARD,
            allow_download_protocols: DEFAULT_ALLOW_DOWNLOAD_PROTOCOLS,
            xcursor_theme: None,
            xcursor_size: None,
            palette_max_key_assigments_for_action: DEFAULT_PALETTE_MAX_KEY_ASSIGMENTS_FOR_ACTION,
            allow_win32_input_mode: DEFAULT_ALLOW_WIN32_INPUT_MODE,
            treat_left_ctrlalt_as_altgr: DEFAULT_TREAT_LEFT_CTRLALT_AS_ALTGR,
            send_composed_key_when_left_alt_is_pressed:
                DEFAULT_SEND_COMPOSED_KEY_WHEN_LEFT_ALT_IS_PRESSED,
            send_composed_key_when_right_alt_is_pressed:
                DEFAULT_SEND_COMPOSED_KEY_WHEN_RIGHT_ALT_IS_PRESSED,
            treat_east_asian_ambiguous_width_as_wide:
                DEFAULT_TREAT_EAST_ASIAN_AMBIGUOUS_WIDTH_AS_WIDE,
            normalize_output_to_unicode_nfc: super::DEFAULT_NORMALIZE_OUTPUT_TO_UNICODE_NFC,
            unicode_version: DEFAULT_UNICODE_VERSION,
            bidi_enabled: DEFAULT_BIDI_ENABLED,
            bidi_direction: DEFAULT_BIDI_DIRECTION,
            use_ime: DEFAULT_USE_IME,
            use_dead_keys: DEFAULT_USE_DEAD_KEYS,
            ime_preedit_rendering: DEFAULT_IME_PREEDIT_RENDERING,
            macos_forward_to_ime_modifier_mask: DEFAULT_MACOS_FORWARD_TO_IME_MODIFIER_MASK,
            xim_im_name: None,
            detect_password_input: DEFAULT_DETECT_PASSWORD_INPUT,
            scroll_to_bottom_on_input: true,
            adjust_window_size_when_changing_font_size:
                DEFAULT_ADJUST_WINDOW_SIZE_WHEN_CHANGING_FONT_SIZE,
            canonicalize_pasted_newlines: DEFAULT_CANONICALIZE_PASTED_NEWLINES,
            quote_dropped_files: DEFAULT_QUOTE_DROPPED_FILES,
            disable_default_key_bindings: DEFAULT_DISABLE_DEFAULT_KEY_BINDINGS,
            disable_default_mouse_bindings: DEFAULT_DISABLE_DEFAULT_MOUSE_BINDINGS,
            hide_mouse_cursor_when_typing: DEFAULT_HIDE_MOUSE_CURSOR_WHEN_TYPING,
            alternate_buffer_wheel_scroll_speed: DEFAULT_ALTERNATE_BUFFER_WHEEL_SCROLL_SPEED,
            pane_focus_follows_mouse: false,
            swallow_mouse_click_on_pane_focus: false,
            swallow_mouse_click_on_window_focus: cfg!(target_os = "macos"),
            bypass_mouse_reporting_modifiers: ModifiersState::SHIFT,
            enable_scroll_bar: false,
            scrollback_lines: DEFAULT_SCROLLBACK_LIMIT,
            min_scroll_bar_height: Some(NativeScrollBarHeight::CellFractionPerMille(500)),
            enable_tab_bar: true,
            hide_tab_bar_if_only_one_tab: false,
            use_fancy_tab_bar: super::DEFAULT_USE_FANCY_TAB_BAR,
            unzoom_on_switch_pane: true,
            tab_bar_at_bottom: false,
            tab_and_split_indices_are_zero_based: false,
            mouse_wheel_scrolls_tabs: true,
            switch_to_last_active_tab_when_closing_tab: false,
            tab_shortcut_style: crate::window::NativeTabShortcutStyle::Terminal,
            closed_tab_history_size: 25,
            close_tab_selection: rterm_core::app_shell::CloseTabSelection::Adjacent,
            tab_bar_wheel_behavior: crate::window::NativeTabBarWheelBehavior::Scroll,
            quit_when_all_windows_are_closed: true,
            window_close_confirmation: NativeWindowCloseConfirmation::AlwaysPrompt,
            exit_behavior: NativeExitBehavior::Close,
            clean_exit_codes: Vec::new(),
            exit_behavior_messaging: NativeExitBehaviorMessaging::Verbose,
            skip_close_confirmation_for_processes_named:
                default_skip_close_confirmation_for_processes_named(),
            show_close_tab_button_in_tabs: true,
            show_new_tab_button_in_tab_bar: true,
            show_tab_index_in_tab_bar: true,
            show_tabs_in_tab_bar: true,
        }
    }

    fn sample_environment() -> BTreeMap<String, String> {
        BTreeMap::from([("WEZTERM_CONFIG_DIR".to_owned(), "/tmp/wezterm".to_owned())])
    }

    fn temp_state_path(name: &str) -> PathBuf {
        static NEXT_TEMP_STATE_ID: AtomicUsize = AtomicUsize::new(0);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "rterm-{name}-{}-{}.json",
            std::process::id(),
            NEXT_TEMP_STATE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&path);
        path
    }

    fn temp_command_palette_frecency_path() -> PathBuf {
        temp_state_path("command-palette-frecency")
    }

    fn temp_char_select_recently_used_path() -> PathBuf {
        temp_state_path("char-select-recently-used")
    }

    #[test]
    fn window_app_augments_command_palette_with_native_entries() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        let active_pane = app.app_shell.active_pane_id();
        app.command_palette_augmenter = Box::new(move |event| {
            recorded.lock().unwrap().push(*event);
            vec![NativeCommandPaletteEntry {
                brief: "Zoom Native Pane".to_owned(),
                doc: Some("Toggle zoom from an augmented native command palette entry".to_owned()),
                icon: Some("md_magnify_plus".to_owned()),
                key_assignment: None,
                action: WindowCommand::TogglePaneZoom,
            }]
        });

        app.enter_command_palette_mode();
        app.command_palette_set_query("zoom native".to_owned());

        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeCommandPaletteAugment {
                window_id: rterm_core::WindowId::new(1),
                pane: active_pane,
            }]
        );
        let entries = app.command_palette_filtered_entries();
        assert_eq!(
            entries,
            [WindowCommandPaletteEntry::Augmented(
                NativeCommandPaletteEntry {
                    brief: "Zoom Native Pane".to_owned(),
                    doc: Some(
                        "Toggle zoom from an augmented native command palette entry".to_owned()
                    ),
                    icon: Some("md_magnify_plus".to_owned()),
                    key_assignment: None,
                    action: WindowCommand::TogglePaneZoom,
                }
            )]
        );
        let palette = app
            .command_palette
            .as_ref()
            .expect("expected command palette to be open");
        assert_eq!(
            app.command_palette_status(palette),
            "Command Palette: \"zoom native\" [1 / 1] Zoom Native Pane"
        );

        assert!(app.command_palette_execute_entry(entries[0].clone()));

        assert_eq!(
            app.app_shell.active_tab().zoomed_pane_id(),
            Some(active_pane)
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_parses_static_wezterm_augment_command_palette_return() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        let active_pane = app.app_shell.active_pane_id();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action

            wezterm.on('augment-command-palette', function(window, pane)
              return {
                {
                  brief = 'Lua Zoom Pane',
                  doc = 'Zoom the active pane from Lua config',
                  icon = 'md_rename_box',
                  action = act.SetPaneZoomState(true),
                },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm augment-command-palette return");
        app.set_config_overrides(overrides);

        app.enter_command_palette_mode();
        app.command_palette_set_query("lua zoom".to_owned());

        let entries = app.command_palette_filtered_entries();
        assert_eq!(
            entries,
            [WindowCommandPaletteEntry::Augmented(
                NativeCommandPaletteEntry {
                    brief: "Lua Zoom Pane".to_owned(),
                    doc: Some("Zoom the active pane from Lua config".to_owned()),
                    icon: Some("md_rename_box".to_owned()),
                    key_assignment: None,
                    action: WindowCommand::SetPaneZoomState(true),
                }
            )]
        );

        assert!(app.command_palette_execute_entry(entries[0].clone()));

        assert_eq!(
            app.app_shell.active_tab().zoomed_pane_id(),
            Some(active_pane)
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_command_palette_renders_augmented_entry_doc() {
        let mut app = NativeWindowApp::new(None);
        app.command_palette_augmenter = Box::new(|_| {
            vec![NativeCommandPaletteEntry {
                brief: "Zoom Native Pane".to_owned(),
                doc: Some("Toggle zoom from an augmented native command palette entry".to_owned()),
                icon: None,
                key_assignment: None,
                action: WindowCommand::TogglePaneZoom,
            }]
        });

        app.enter_command_palette_mode();
        app.command_palette_set_query("zoom native".to_owned());

        let snapshot = app.render_snapshot();
        let first_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_row.contains("Zoom Native Pane"),
            "first command palette row was {first_row:?}"
        );
        assert!(
            first_row.contains("Toggle zoom from an augmented native command palette entry"),
            "first command palette row was {first_row:?}"
        );
    }
