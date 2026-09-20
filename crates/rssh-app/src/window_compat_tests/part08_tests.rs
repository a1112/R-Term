    fn active_pane_presentation_for_higher_level_ui_test(
        app: &NativeWindowApp,
    ) -> TerminalRenderSnapshot {
        let rect = app
            .pane_render_layout()
            .panes
            .into_iter()
            .find(|rect| rect.pane_id == app.active_pane_id())
            .expect("active pane rect");
        super::pane_presentation_snapshot(
            app.snapshot.clone(),
            app.runtime.terminal(),
            &app.active_ui,
            rect,
            &app.native_resolved_palette(),
            &app.selection_word_boundary,
            app.higher_level_ui_suppresses_pane_overlay(),
            app.quick_select_remove_styling,
            app.foreground_text_hsb,
            app.text_background_opacity,
            app.window_background_opacity,
            None,
            app.text_min_contrast_ratio,
            app.bold_brightens_ansi_colors,
        )
    }

    #[test]
    fn window_app_each_higher_level_ui_preserves_pane_overlay_slots() {
        for ui_case in 0..9 {
            let mut app = NativeWindowApp::new(None);
            app.handle_pty_output(format!("{:70}x", "").as_bytes())
                .unwrap();
            set_app_quick_select_for_test(
                &mut app,
                WindowQuickSelect {
                    current: 0,
                    matches: vec![pane_overlay_match(70)],
                    labels: vec!["a".to_owned()],
                    ..WindowQuickSelect::default()
                },
            );
            app.update_selection_projection();
            app.apply_window_title();
            let base_background = snapshot_cell(&app.snapshot, 0, 70)
                .expect("base pane cell")
                .background;
            let overlay_background = snapshot_cell(
                &active_pane_presentation_for_higher_level_ui_test(&app),
                0,
                70,
            )
            .expect("overlay pane cell")
            .background;
            assert_ne!(
                overlay_background, base_background,
                "fixture exposes pane transient styling for case {ui_case}"
            );
            match ui_case {
                0 => app.enter_command_palette_mode(),
                1 => app.enter_launcher_mode(),
                2 => app.enter_close_confirmation_mode(WindowCloseTarget::Window),
                3 => app.enter_confirmation_mode(WindowConfirmationOptions {
                    message: "confirm".to_owned(),
                    action: Box::new(WindowCommand::Nop),
                    cancel: None,
                }),
                4 => app.enter_input_selector_mode(WindowInputSelectorOptions::default()),
                5 => app.enter_prompt_input_line_mode(WindowPromptInputLineOptions::default()),
                6 => app.enter_pane_select_mode(),
                7 => app.enter_tab_navigator_mode(),
                8 => app.enter_char_select_mode(),
                _ => unreachable!(),
            }
            assert!(overlay_active_for_test(&app), "entry case {ui_case}");
            assert!(
                !app.effective_window_title().contains("Quick Select"),
                "higher-level UI keeps title/input precedence for case {ui_case}"
            );
            assert_eq!(
                snapshot_cell(
                    &active_pane_presentation_for_higher_level_ui_test(&app),
                    0,
                    70,
                )
                .expect("suppressed pane overlay cell")
                .background,
                base_background,
                "higher-level UI suppresses transient styling for case {ui_case}"
            );
            app.handle_keyboard_input_event(
                &Key::Named(NamedKey::Escape),
                PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
                None,
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
            assert!(overlay_active_for_test(&app), "exit case {ui_case}");
            assert!(
                app.effective_window_title().contains("Quick Select"),
                "pane overlay is reprojected after case {ui_case}"
            );
            assert_eq!(
                snapshot_cell(
                    &active_pane_presentation_for_higher_level_ui_test(&app),
                    0,
                    70,
                )
                .expect("restored pane overlay cell")
                .background,
                overlay_background,
                "pane overlay styling restores after case {ui_case}"
            );
        }
    }

    #[test]
    fn window_app_higher_level_ui_suppresses_only_active_transient_pane_overlay() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rssh_core::TerminalSize::new(90, 2));
        app.selection_bg_color = Some(Color::Rgb(240, 20, 220));
        app.copy_mode_active_highlight_bg = Some(NativeColorSpec::Color(Color::Rgb(20, 220, 40)));
        app.quick_select_match_bg = Some(NativeColorSpec::Color(Color::Rgb(30, 60, 240)));
        app.handle_pty_output(b"abcdefghijklmnopqrst").unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"abcdefghijklmnopqrst").unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(2),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"abcdefghijklmnopqrst").unwrap();

        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rssh_core::PaneId::new(1),
        })
        .unwrap();
        install_distinct_pane_overlay_for_lifecycle_test(
            &mut app,
            PaneOverlayLifecycleClass::Search,
            "inactive-search",
            0,
            1,
        );
        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rssh_core::PaneId::new(2),
        })
        .unwrap();
        install_distinct_pane_overlay_for_lifecycle_test(
            &mut app,
            PaneOverlayLifecycleClass::Quick,
            "inactive-quick",
            0,
            4,
        );
        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rssh_core::PaneId::new(3),
        })
        .unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 0 },
        );
        let ordinary = ordinary_selection_for_test(&app).expect("deferred ordinary selection");
        install_distinct_pane_overlay_for_lifecycle_test(
            &mut app,
            PaneOverlayLifecycleClass::Quick,
            "active-quick",
            0,
            10,
        );

        let layout = app.pane_render_layout();
        let pane_cell_background =
            |snapshot: &TerminalRenderSnapshot, pane_id: rssh_core::PaneId, column: u16| {
                let rect = layout
                    .panes
                    .iter()
                    .find(|rect| rect.pane_id == pane_id)
                    .expect("pane render rect");
                snapshot_cell(snapshot, rect.row, rect.column.saturating_add(column))
                    .expect("pane render cell")
                    .background
            };
        let before = app.render_snapshot();
        let inactive_search_before = pane_cell_background(&before, rssh_core::PaneId::new(1), 1);
        let inactive_quick_before = pane_cell_background(&before, rssh_core::PaneId::new(2), 7);
        let active_transient_before = pane_cell_background(&before, rssh_core::PaneId::new(3), 13);
        let active_ordinary_before = pane_cell_background(&before, rssh_core::PaneId::new(3), 0);
        assert_ne!(inactive_search_before, active_ordinary_before);
        assert_ne!(inactive_quick_before, active_ordinary_before);
        assert_ne!(active_transient_before, active_ordinary_before);

        app.enter_confirmation_mode(WindowConfirmationOptions {
            message: "modal".to_owned(),
            action: Box::new(WindowCommand::Nop),
            cancel: None,
        });
        let suppressed = app.render_snapshot();
        assert_eq!(ordinary_selection_for_test(&app), Some(ordinary));
        assert!(quick_select_for_test(&app).is_some());
        assert!(!app.effective_window_title().contains("Quick Select"));
        assert_eq!(
            pane_cell_background(&suppressed, rssh_core::PaneId::new(1), 1),
            inactive_search_before,
            "inactive Search owner remains visible"
        );
        assert_eq!(
            pane_cell_background(&suppressed, rssh_core::PaneId::new(2), 7),
            inactive_quick_before,
            "inactive Quick owner remains visible"
        );
        assert_eq!(
            pane_cell_background(&suppressed, rssh_core::PaneId::new(3), 0),
            Color::Rgb(240, 20, 220),
            "active deferred ordinary selection keeps its legacy presentation"
        );
        assert_eq!(
            pane_cell_background(&suppressed, rssh_core::PaneId::new(3), 13),
            active_ordinary_before,
            "active transient Quick styling is suppressed"
        );

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::Escape),
            PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        let restored = app.render_snapshot();
        assert_eq!(ordinary_selection_for_test(&app), Some(ordinary));
        assert!(app.effective_window_title().contains("Quick Select"));
        assert_eq!(
            pane_cell_background(&restored, rssh_core::PaneId::new(3), 13),
            active_transient_before,
            "active transient presentation restores exactly"
        );
    }

    #[test]
    fn window_app_blank_and_release_mouse_events_do_not_clear_pane_overlay() {
        let mut app = NativeWindowApp::new(None);
        install_distinct_pane_overlay_for_lifecycle_test(
            &mut app,
            PaneOverlayLifecycleClass::Quick,
            "mouse-owner",
            0,
            0,
        );
        app.handle_cursor_moved(PhysicalPosition::new(-10.0, -10.0))
            .unwrap();
        assert!(
            !app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert_eq!(
            quick_select_for_test(&app).map(|quick| quick.input.as_str()),
            Some("mouse-owner")
        );

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();
        app.handle_mouse_input(ElementState::Released, MouseButton::Left)
            .unwrap();
        assert_eq!(
            quick_select_for_test(&app).map(|quick| quick.input.as_str()),
            Some("mouse-owner")
        );
    }

    #[test]
    fn window_app_pending_runtime_survives_sync_and_continues_output_until_materialized() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rssh_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"before").unwrap();
        install_distinct_pane_overlay_for_lifecycle_test(
            &mut app,
            PaneOverlayLifecycleClass::Search,
            "pending-owner",
            0,
            0,
        );
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::MovePaneToNewWindow {
            pane: rssh_core::PaneId::new(1),
        })
        .unwrap();
        assert!(app.pane_runtimes.contains_key(&rssh_core::PaneId::new(1)));

        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert!(app.pane_runtimes.contains_key(&rssh_core::PaneId::new(1)));
        app.handle_pane_pty_output(rssh_core::PaneId::new(1), b"-after")
            .unwrap();

        let detached = app.take_next_pending_window_app().expect("pending runtime");
        assert_eq!(snapshot_row_text(&detached.snapshot, 0, 12), "before-after");
        assert!(!detached.active_ui.overlay_active());
    }

    fn install_local_v2_worker_for_window_transfer(
        app: &mut NativeWindowApp,
        pane: rssh_core::PaneId,
    ) -> rterm_runtime::testing::ScriptedSessionDriver {
        let size = app
            .pane_runtime_ref(pane)
            .expect("transfer pane presentation")
            .terminal()
            .grid()
            .size();
        let (transport, driver) = rterm_runtime::testing::ScriptedTransport::new(
            [rterm_runtime::testing::ReadAction::Block],
            [rterm_runtime::testing::WriteAction::accept(usize::MAX)],
            [],
        );
        let worker = crate::runtime_composition::WindowPaneRuntime::open_transport(
            crate::runtime_composition::PaneRuntimeRoute {
                window: app.app_window_id,
                pane,
            },
            transport,
            size,
            rterm_runtime::TerminalRuntime::new(size),
            crate::runtime_composition::PaneCapturePolicy {
                host_stream: false,
                visible_output: false,
            },
            Arc::new(|| {}),
        )
        .expect("source local worker");
        app.runtime.install_worker(Some(worker));
        app.active_runtime_transport = Some(PaneRuntimeTransportKind::LocalPty);
        driver.wait_until_reader_blocked();
        driver
    }

    fn install_restarted_local_worker_after_window_transfer(
        app: &mut NativeWindowApp,
        pane: rssh_core::PaneId,
    ) -> rterm_runtime::testing::ScriptedSessionDriver {
        let (transport, driver) = rterm_runtime::testing::ScriptedTransport::new(
            [rterm_runtime::testing::ReadAction::Block],
            std::iter::repeat_n(
                rterm_runtime::testing::WriteAction::accept(usize::MAX),
                4,
            ),
            [],
        );
        app.install_transferred_local_transport(
            pane,
            transport,
            None,
            None,
            Arc::new(|| {}),
        )
        .expect("restart transferred local worker");
        driver.wait_until_reader_blocked();
        driver
    }

    fn publish_restarted_local_output(
        app: &mut NativeWindowApp,
        driver: &rterm_runtime::testing::ScriptedSessionDriver,
        pane: rssh_core::PaneId,
        bytes: &'static [u8],
        expected: &str,
    ) {
        let write_barrier = b"target-output-barrier";
        let accepted_before = driver.accepted_writes().len();
        driver.push_reads([
            rterm_runtime::testing::ReadAction::bytes(bytes),
            rterm_runtime::testing::ReadAction::Block,
        ]);
        app.write_pty_bytes_to_pane(pane, write_barrier)
            .expect("target output barrier input");
        driver.wait_until_accepted_write_len(accepted_before + write_barrier.len());
        driver.wait_until_reader_blocked();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            app.poll_active_v2_runtime()
                .expect("poll restarted target local worker");
            let actual = snapshot_row_text(
                &app.snapshot,
                0,
                u16::try_from(expected.len()).unwrap(),
            );
            if actual == expected {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "restarted target output did not reach the preserved presentation: \
                 expected={expected:?}, actual={actual:?}, worker_needs_poll={:?}",
                app.runtime
                    .worker()
                    .map(crate::runtime_composition::WindowPaneRuntime::needs_poll)
            );
            std::thread::yield_now();
        }
    }

    #[test]
    fn pending_window_transfer_retires_the_source_local_worker() {
        let mut source = NativeWindowApp::new(None);
        source.handle_pty_output(b"pending-local-scrollback").unwrap();
        source
            .handle_pty_output(b"\x1b]7;file://host/pending-transfer-cwd\x07")
            .unwrap();
        let moved_pane = source.active_pane_id();
        let source_driver =
            install_local_v2_worker_for_window_transfer(&mut source, moved_pane);
        source
            .dispatch_app_action(AppAction::SplitPane {
                pane: moved_pane,
                direction: SplitDirection::Right,
                launch: Some(PaneLaunch::local("source-stays")),
            })
            .unwrap();
        source
            .dispatch_app_action(AppAction::MovePaneToNewWindow { pane: moved_pane })
            .unwrap();

        let mut detached = source
            .take_next_pending_window_app()
            .expect("pending local window");

        assert!(
            source
                .runtime
                .worker()
                .is_none_or(|worker| !worker.contains_pane(moved_pane)),
            "the source hub must not retain a moved local transport"
        );
        assert!(source_driver.interrupt_calls() > 0);
        assert_eq!(detached.active_pane_id(), moved_pane);
        assert_eq!(
            detached.app_shell.active_pane().launch().cwd(),
            Some("file://host/pending-transfer-cwd")
        );
        assert_eq!(
            snapshot_row_text(
                &detached.snapshot,
                0,
                u16::try_from("pending-local-scrollback".len()).unwrap(),
            ),
            "pending-local-scrollback"
        );
        let target_driver =
            install_restarted_local_worker_after_window_transfer(&mut detached, moved_pane);
        detached
            .write_pty_bytes_to_pane(moved_pane, b"pending-target-input")
            .unwrap();
        target_driver.wait_until_accepted_write_len("pending-target-input".len());
        assert_eq!(target_driver.accepted_writes(), b"pending-target-input");
        let resized = TerminalSize::new(91, 31);
        detached.resize_terminal_runtimes(resized).unwrap();
        target_driver.wait_until_control_call_count(1);
        assert_eq!(target_driver.control_log().resizes, vec![resized]);
        assert!(source_driver.accepted_writes().is_empty());
        publish_restarted_local_output(
            &mut detached,
            &target_driver,
            moved_pane,
            b"-continued",
            "pending-local-scrollback-continued",
        );
        assert_eq!(
            snapshot_row_text(
                &detached.snapshot,
                0,
                u16::try_from("pending-local-scrollback-continued".len()).unwrap(),
            ),
            "pending-local-scrollback-continued"
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn window_app_pending_ssh_window_moves_all_auxiliary_ownership() {
        let mut source = NativeWindowApp::new(None);
        source.set_initial_pane_launch(PaneLaunch::ssh(SshPaneLaunch::new(
            "ops@pending-transfer.example:2222",
            SshAuthDescription::PasswordPrompt,
            SshKnownHostsPolicy::Prompt,
        )));
        let ssh_pane = source.active_pane_id();
        let secret = "pending-window-transfer-secret";
        let (command_sender, command_receiver) = mpsc::channel();
        let (decision_sender, decision_receiver) = mpsc::sync_channel(1);
        let (response_sender, response_receiver) = mpsc::sync_channel(1);
        source
            .ssh_writer_senders
            .insert(ssh_pane, command_sender);
        source.ssh_connection_states.insert(
            ssh_pane,
            super::ConnectionState::AwaitingSecret,
        );
        source.ssh_host_key_prompts.insert(
            ssh_pane,
            (
                rssh_ssh::HostKeyChallenge::new(
                    "pending-transfer.example",
                    2222,
                    "ssh-ed25519",
                    "SHA256:pending-transfer",
                    rssh_ssh::HostKeyStatus::Unknown,
                ),
                decision_sender,
            ),
        );
        source.ssh_secret_prompts.insert(
            ssh_pane,
            super::SshSecretPromptState {
                prompt: rssh_ssh::SecretPrompt::password("ops"),
                response: response_sender,
                input: secret.to_owned(),
            },
        );

        source
            .dispatch_app_action(AppAction::SplitPane {
                pane: ssh_pane,
                direction: SplitDirection::Right,
                launch: Some(PaneLaunch::local("local-shell")),
            })
            .unwrap();
        source
            .dispatch_app_action(AppAction::MovePaneToNewWindow { pane: ssh_pane })
            .unwrap();
        let mut detached = source
            .take_next_pending_window_app()
            .expect("pending SSH window");

        assert!(!source.ssh_writer_senders.contains_key(&ssh_pane));
        assert!(!source.ssh_connection_states.contains_key(&ssh_pane));
        assert!(!source.ssh_host_key_prompts.contains_key(&ssh_pane));
        assert!(!source.ssh_secret_prompts.contains_key(&ssh_pane));
        assert!(detached.ssh_writer_senders.contains_key(&ssh_pane));
        assert_eq!(
            detached.ssh_connection_state_for_pane(ssh_pane),
            super::ConnectionState::AwaitingSecret
        );
        assert!(detached.ssh_host_key_prompts.contains_key(&ssh_pane));
        assert!(detached.ssh_secret_prompts.contains_key(&ssh_pane));
        assert!(
            detached
                .effective_window_title()
                .contains("ops@pending-transfer.example:2222 [awaiting_secret]")
        );
        assert_eq!(
            detached.metrics.connection_state(),
            super::ConnectionState::AwaitingSecret
        );

        drop(source);
        detached
            .ssh_writer_senders
            .get(&ssh_pane)
            .unwrap()
            .send(super::NativeSshCommand::Resize(TerminalSize::new(132, 43)))
            .unwrap();
        assert!(matches!(
            command_receiver.recv().unwrap(),
            super::NativeSshCommand::Resize(size) if size == TerminalSize::new(132, 43)
        ));
        detached.resolve_host_key_prompt_for_pane(
            ssh_pane,
            rssh_ssh::HostKeyDecision::AcceptOnce,
        );
        assert_eq!(
            decision_receiver.recv().unwrap(),
            rssh_ssh::HostKeyDecision::AcceptOnce
        );
        assert!(detached.handle_ssh_prompt_key_event(&Key::Named(NamedKey::Enter), None));
        assert_eq!(response_receiver.recv().unwrap(), Some(secret.to_owned()));
    }

    #[test]
    fn window_app_multiple_pending_windows_materialize_fifo_without_runtime_crossing() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"first").unwrap();
        install_distinct_pane_overlay_for_lifecycle_test(
            &mut app,
            PaneOverlayLifecycleClass::Search,
            "first-owner",
            0,
            0,
        );
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"second").unwrap();
        install_distinct_pane_overlay_for_lifecycle_test(
            &mut app,
            PaneOverlayLifecycleClass::Quick,
            "second-owner",
            0,
            0,
        );
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(2),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::MovePaneToNewWindow {
            pane: rssh_core::PaneId::new(1),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::MovePaneToNewWindow {
            pane: rssh_core::PaneId::new(2),
        })
        .unwrap();
        assert_eq!(app.app_shell.pending_windows().len(), 2);

        let first = app.take_next_pending_window_app().expect("first pending");
        let second = app.take_next_pending_window_app().expect("second pending");
        assert_eq!(first.active_pane_id(), rssh_core::PaneId::new(1));
        assert_eq!(second.active_pane_id(), rssh_core::PaneId::new(2));
        assert_eq!(snapshot_row_text(&first.snapshot, 0, 5), "first");
        assert_eq!(snapshot_row_text(&second.snapshot, 0, 6), "second");
        assert!(!first.active_ui.overlay_active());
        assert!(!second.active_ui.overlay_active());
    }

    #[test]
    fn window_app_failed_multiple_move_restores_owner_without_phantom_pending_window() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"owner").unwrap();
        install_distinct_pane_overlay_for_lifecycle_test(
            &mut app,
            PaneOverlayLifecycleClass::Quick,
            "rollback-owner",
            0,
            0,
        );
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rssh_core::PaneId::new(1),
        })
        .unwrap();

        assert!(
            app.dispatch_app_action(AppAction::Multiple {
                actions: vec![
                    AppAction::MovePaneToNewWindow {
                        pane: rssh_core::PaneId::new(1),
                    },
                    AppAction::ActivatePane {
                        pane: rssh_core::PaneId::new(999),
                    },
                ],
            })
            .is_err()
        );
        assert!(app.app_shell.pending_windows().is_empty());
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(1));
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 5), "owner");
        assert_eq!(
            quick_select_for_test(&app).map(|quick| quick.input.as_str()),
            Some("rollback-owner")
        );
    }

    #[test]
    fn window_manager_routes_queued_old_window_events_to_relocated_pane_owner() {
        let mut primary = NativeWindowApp::new(None);
        primary.runtime.resize(rssh_core::TerminalSize::new(16, 1));
        primary.handle_pty_output(b"relocated").unwrap();
        primary.active_runtime_generation = 41;
        primary
            .dispatch_app_action(AppAction::SplitPane {
                pane: rssh_core::PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        primary
            .dispatch_app_action(AppAction::MovePaneToNewWindow {
                pane: rssh_core::PaneId::new(1),
            })
            .unwrap();
        let mut manager = NativeWindowManager::new_for_test(primary);
        manager.collect_pending_window_apps_from_primary_for_test();

        assert_eq!(
            manager.dispatch_user_event_to_owner(WindowUserEvent::Output {
                window_id: rssh_core::WindowId::new(1),
                pane_id: rssh_core::PaneId::new(1),
                runtime_generation: 41,
                bytes: b"-queued".to_vec(),
            }),
            Some(false)
        );
        assert_eq!(
            manager
                .pending_apps
                .first()
                .map(|app| snapshot_row_text(&app.snapshot, 0, 16)),
            Some("relocated-queued".to_owned())
        );
        assert_eq!(
            manager.startup_app.as_ref().map(|app| app.active_pane_id()),
            Some(rssh_core::PaneId::new(2))
        );

        assert_eq!(
            manager.dispatch_user_event_to_owner(WindowUserEvent::Exited {
                window_id: rssh_core::WindowId::new(1),
                pane_id: rssh_core::PaneId::new(1),
                runtime_generation: 41,
            }),
            Some(true)
        );
        assert!(manager.pending_apps.is_empty());
        assert!(
            manager
                .pane_event_routes
                .keys()
                .all(|(_, pane_id)| *pane_id != rssh_core::PaneId::new(1))
        );
        assert_eq!(
            manager.startup_app.as_ref().map(|app| app.active_pane_id()),
            Some(rssh_core::PaneId::new(2))
        );
    }

    #[test]
    fn window_manager_follows_repeated_pane_relocation_chain_for_queued_events() {
        let mut primary = NativeWindowApp::new(None);
        primary.handle_pty_output(b"twice").unwrap();
        primary
            .dispatch_app_action(AppAction::SplitPane {
                pane: rssh_core::PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        primary
            .dispatch_app_action(AppAction::MovePaneToNewWindow {
                pane: rssh_core::PaneId::new(1),
            })
            .unwrap();
        let mut manager = NativeWindowManager::new_for_test(primary);
        manager.collect_pending_window_apps_from_primary_for_test();

        let mut relocated = manager.pending_apps.remove(0);
        assert_eq!(relocated.app_window_id, rssh_core::WindowId::new(2));
        relocated
            .dispatch_app_action(AppAction::SplitPane {
                pane: rssh_core::PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        relocated
            .dispatch_app_action(AppAction::MovePaneToNewWindow {
                pane: rssh_core::PaneId::new(1),
            })
            .unwrap();
        manager.collect_pending_window_apps_from_app(&mut relocated);
        manager.pending_apps.push(relocated);
        assert!(
            manager
                .pane_event_routes
                .contains_key(&(rssh_core::WindowId::new(1), rssh_core::PaneId::new(1)))
        );
        assert!(
            manager
                .pane_event_routes
                .contains_key(&(rssh_core::WindowId::new(2), rssh_core::PaneId::new(1)))
        );

        assert_eq!(
            manager.dispatch_user_event_to_owner(WindowUserEvent::Output {
                window_id: rssh_core::WindowId::new(1),
                pane_id: rssh_core::PaneId::new(1),
                runtime_generation: 0,
                bytes: b"-queued".to_vec(),
            }),
            Some(false)
        );
        let final_owner = manager
            .pending_apps
            .iter()
            .find(|app| app.app_window_id == rssh_core::WindowId::new(3))
            .expect("second relocation destination");
        assert_eq!(
            snapshot_row_text(&final_owner.snapshot, 0, 12),
            "twice-queued"
        );
    }

    #[test]
    fn window_manager_keeps_captured_routes_after_repeated_move_and_source_close() {
        let mut primary = NativeWindowApp::new(None);
        primary.handle_pty_output(b"relocated").unwrap();
        primary
            .dispatch_app_action(AppAction::SplitPane {
                pane: rssh_core::PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        primary
            .dispatch_app_action(AppAction::MovePaneToNewWindow {
                pane: rssh_core::PaneId::new(1),
            })
            .unwrap();
        let mut manager = NativeWindowManager::new_for_test(primary);
        manager.collect_pending_window_apps_from_primary_for_test();

        let mut intermediate = manager.pending_apps.remove(0);
        intermediate
            .dispatch_app_action(AppAction::SplitPane {
                pane: rssh_core::PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        intermediate
            .dispatch_app_action(AppAction::MovePaneToNewWindow {
                pane: rssh_core::PaneId::new(1),
            })
            .unwrap();
        manager.collect_pending_window_apps_from_app(&mut intermediate);
        manager.pending_apps.push(intermediate);

        let mut collision = NativeWindowApp::new(None);
        collision.app_window_id = rssh_core::WindowId::new(4);
        collision.handle_pty_output(b"collision").unwrap();
        manager.pending_apps.push(Box::new(collision));

        let source = manager.startup_app.take().expect("source window");
        drop(source);
        manager.remove_pane_event_routes_for_window(rssh_core::WindowId::new(1));
        let intermediate_index = manager
            .pending_apps
            .iter()
            .position(|app| app.app_window_id == rssh_core::WindowId::new(2))
            .expect("intermediate window");
        drop(manager.pending_apps.remove(intermediate_index));
        manager.remove_pane_event_routes_for_window(rssh_core::WindowId::new(2));

        assert_eq!(
            manager.dispatch_user_event_to_owner(WindowUserEvent::Output {
                window_id: rssh_core::WindowId::new(1),
                pane_id: rssh_core::PaneId::new(1),
                runtime_generation: 0,
                bytes: b"-from-a".to_vec(),
            }),
            Some(false)
        );
        assert_eq!(
            manager
                .pending_apps
                .iter()
                .find(|app| app.app_window_id == rssh_core::WindowId::new(3))
                .map(|app| snapshot_row_text(&app.snapshot, 0, 16)),
            Some("relocated-from-a".to_owned())
        );
        assert_eq!(
            manager.dispatch_user_event_to_owner(WindowUserEvent::Exited {
                window_id: rssh_core::WindowId::new(2),
                pane_id: rssh_core::PaneId::new(1),
                runtime_generation: 0,
            }),
            Some(true)
        );
        assert!(
            manager
                .pending_apps
                .iter()
                .all(|app| app.app_window_id != rssh_core::WindowId::new(3))
        );
        assert_eq!(
            manager
                .pending_apps
                .iter()
                .find(|app| app.app_window_id == rssh_core::WindowId::new(4))
                .map(|app| snapshot_row_text(&app.snapshot, 0, 9)),
            Some("collision".to_owned())
        );
    }

    #[test]
    fn window_manager_stale_relocation_route_without_owner_never_uses_unique_fallback() {
        let mut declared = NativeWindowApp::new(None);
        declared.handle_pty_output(b"untouched").unwrap();
        let mut manager = NativeWindowManager::new_for_test(declared);
        manager.pane_event_routes.insert(
            (rssh_core::WindowId::new(99), rssh_core::PaneId::new(1)),
            crate::window::PaneEventRoute {
                window_id: rssh_core::WindowId::new(100),
                pane_id: rssh_core::PaneId::new(1),
            },
        );

        assert_eq!(
            manager.dispatch_user_event_to_owner(WindowUserEvent::Output {
                window_id: rssh_core::WindowId::new(99),
                pane_id: rssh_core::PaneId::new(1),
                runtime_generation: 0,
                bytes: b"-wrong".to_vec(),
            }),
            None
        );
        assert_eq!(
            manager
                .startup_app
                .as_ref()
                .map(|app| snapshot_row_text(&app.snapshot, 0, 9)),
            Some("untouched".to_owned())
        );
    }

    fn lifecycle_move_fixture(
        class: PaneOverlayLifecycleClass,
        source_active: bool,
    ) -> NativeWindowApp {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rssh_core::TerminalSize::new(12, 2));
        app.handle_pty_output(b"one\r\ntwo\r\nthree\r\nfour")
            .unwrap();
        app.scroll_viewport_lines(1);
        let first = ordinary_source_cell_for_viewport(&app, 0, 0);
        set_ordinary_stable_selection_for_test(
            &mut app,
            first,
            SelectionSourceCell { column: 2, ..first },
            false,
        );
        install_distinct_pane_overlay_for_lifecycle_test(&mut app, class, "move-source", 0, 0);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        install_distinct_pane_overlay_for_lifecycle_test(&mut app, class, "move-survivor", 0, 1);
        if source_active {
            app.dispatch_app_action(AppAction::ActivatePane {
                pane: rssh_core::PaneId::new(1),
            })
            .unwrap();
        }
        app
    }

    #[test]
    fn window_app_move_active_or_inactive_pane_to_new_tab_preserves_overlay() {
        for class in [
            PaneOverlayLifecycleClass::Search,
            PaneOverlayLifecycleClass::Copy,
            PaneOverlayLifecycleClass::Quick,
        ] {
            for source_active in [true, false] {
                let mut app = lifecycle_move_fixture(class, source_active);
                let (expected_ordinary_selection, expected_stable_viewport) =
                    if app.active_pane_id() == rssh_core::PaneId::new(1) {
                        (
                            app.active_ui.ordinary_selection,
                            app.active_ui.stable_viewport,
                        )
                    } else {
                        let source = app
                            .pane_runtimes
                            .get(&rssh_core::PaneId::new(1))
                            .expect("same-window move source runtime");
                        (source.ui.ordinary_selection, source.ui.stable_viewport)
                    };
                assert!(expected_ordinary_selection.is_some());
                assert_ne!(
                    expected_stable_viewport,
                    super::PaneStableViewport::default()
                );
                app.dispatch_app_action(AppAction::MovePaneToNewTab {
                    pane: rssh_core::PaneId::new(1),
                })
                .unwrap();
                assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(1));
                assert_pane_overlay_class_for_lifecycle_test(&app, class);
                assert_eq!(
                    app.active_ui.ordinary_selection, expected_ordinary_selection,
                    "{class:?} ordinary selection"
                );
                assert_eq!(
                    app.active_ui.stable_viewport, expected_stable_viewport,
                    "{class:?} stable viewport"
                );
                match class {
                    PaneOverlayLifecycleClass::Search => assert_eq!(
                        search_for_test(&app).map(|search| search.query.as_str()),
                        Some("move-source")
                    ),
                    PaneOverlayLifecycleClass::Copy => {
                        assert!(
                            search_for_test(&app)
                                .is_some_and(|search| search.query == "move-source")
                        );
                    }
                    PaneOverlayLifecycleClass::Quick => assert!(
                        quick_select_for_test(&app)
                            .is_some_and(|quick| quick.input == "move-source")
                    ),
                }
            }
        }
    }

    fn prepare_new_window_overlay_source(
        app: &mut NativeWindowApp,
        class: PaneOverlayLifecycleClass,
        written: Arc<Mutex<Vec<u8>>>,
    ) -> (Vec<String>, usize, Option<StableRowIndex>) {
        app.runtime.resize(rssh_core::TerminalSize::new(10, 2));
        app.handle_pty_output(b"one\r\ntwo\r\nthree\r\nfour")
            .unwrap();
        app.scroll_viewport_lines(1);
        let first = ordinary_source_cell_for_viewport(app, 0, 0);
        set_ordinary_stable_selection_for_test(
            app,
            first,
            SelectionSourceCell { column: 2, ..first },
            false,
        );
        install_distinct_pane_overlay_for_lifecycle_test(app, class, "detached-source", 0, 0);
        app.writer = Some(Box::new(SharedWriter(written)));
        app.rebuild_snapshot();
        (
            vec![
                snapshot_row_text(&app.snapshot, 0, 10),
                snapshot_row_text(&app.snapshot, 1, 10),
            ],
            app.runtime.terminal().scrollback().len(),
            app.current_stable_viewport_top(),
        )
    }

    fn assert_detached_overlay_source(
        detached: &mut NativeWindowApp,
        expected_rows: &[String],
        expected_scrollback: usize,
        expected_top: Option<StableRowIndex>,
        written: &Arc<Mutex<Vec<u8>>>,
    ) {
        assert_eq!(detached.current_stable_viewport_top(), expected_top);
        assert_eq!(
            detached.runtime.terminal().scrollback().len(),
            expected_scrollback
        );
        assert_eq!(
            [
                snapshot_row_text(&detached.snapshot, 0, 10),
                snapshot_row_text(&detached.snapshot, 1, 10),
            ],
            expected_rows
        );
        assert!(detached.active_ui.ordinary_selection.is_none());
        assert!(!detached.active_ui.overlay_active());
        assert!(detached.selection.is_none());
        detached.write_pty_bytes(b"alive").unwrap();
        assert_eq!(written.lock().unwrap().as_slice(), b"alive");
    }

    #[test]
    fn window_app_move_active_pane_to_new_window_transfers_runtime_then_clears_gui_ui() {
        for class in [
            PaneOverlayLifecycleClass::Search,
            PaneOverlayLifecycleClass::Copy,
            PaneOverlayLifecycleClass::Quick,
        ] {
            let written = Arc::new(Mutex::new(Vec::new()));
            let mut app = NativeWindowApp::new(None);
            let (rows, scrollback, top) =
                prepare_new_window_overlay_source(&mut app, class, Arc::clone(&written));
            app.dispatch_app_action(AppAction::SplitPane {
                pane: rssh_core::PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
            app.dispatch_app_action(AppAction::ActivatePane {
                pane: rssh_core::PaneId::new(1),
            })
            .unwrap();
            app.dispatch_app_action(AppAction::MovePaneToNewWindow {
                pane: rssh_core::PaneId::new(1),
            })
            .unwrap();
            let mut detached = app.take_next_pending_window_app().expect("detached app");
            assert_detached_overlay_source(&mut detached, &rows, scrollback, top, &written);
        }
    }

    #[test]
    fn window_app_move_inactive_pane_to_new_window_transfers_runtime_then_clears_gui_ui() {
        for class in [
            PaneOverlayLifecycleClass::Search,
            PaneOverlayLifecycleClass::Copy,
            PaneOverlayLifecycleClass::Quick,
        ] {
            let written = Arc::new(Mutex::new(Vec::new()));
            let mut app = NativeWindowApp::new(None);
            let (rows, scrollback, top) =
                prepare_new_window_overlay_source(&mut app, class, Arc::clone(&written));
            app.dispatch_app_action(AppAction::SplitPane {
                pane: rssh_core::PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
            install_distinct_pane_overlay_for_lifecycle_test(
                &mut app,
                class,
                "move-survivor",
                0,
                1,
            );
            app.dispatch_app_action(AppAction::MovePaneToNewWindow {
                pane: rssh_core::PaneId::new(1),
            })
            .unwrap();
            assert_pane_overlay_class_for_lifecycle_test(&app, class);
            let mut detached = app.take_next_pending_window_app().expect("detached app");
            assert_detached_overlay_source(&mut detached, &rows, scrollback, top, &written);
        }
    }

    #[test]
    fn window_app_move_tab_to_new_window_transfers_every_pane_runtime() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(2),
            direction: SplitDirection::Right,
            launch: Some(PaneLaunch::local("ssh").with_args(["ops"])),
        })
        .unwrap();

        app.dispatch_app_action(AppAction::MoveTabToNewWindow {
            tab: rssh_core::TabId::new(2),
        })
        .unwrap();
        let detached = app
            .take_next_pending_window_app()
            .expect("tab should materialize as a detached window");

        assert_eq!(detached.app_shell.active_workspace().tabs().len(), 1);
        assert_eq!(detached.app_shell.active_tab().panes().len(), 2);
        assert!(
            detached
                .pane_runtimes
                .contains_key(&rssh_core::PaneId::new(2))
        );
        assert_eq!(detached.active_pane_id(), rssh_core::PaneId::new(3));
    }

    #[test]
    fn window_app_close_inactive_pane_or_tab_drops_only_target_overlays() {
        for class in [
            PaneOverlayLifecycleClass::Search,
            PaneOverlayLifecycleClass::Copy,
            PaneOverlayLifecycleClass::Quick,
        ] {
            let mut pane_app = lifecycle_move_fixture(class, false);
            pane_app
                .dispatch_app_action(AppAction::ClosePane {
                    pane: rssh_core::PaneId::new(1),
                })
                .unwrap();
            assert_eq!(pane_app.active_pane_id(), rssh_core::PaneId::new(2));
            assert_pane_overlay_tag_for_lifecycle_test(&pane_app, class, "move-survivor");
            assert!(
                !pane_app
                    .pane_runtimes
                    .contains_key(&rssh_core::PaneId::new(1))
            );

            let mut tab_app = NativeWindowApp::new(None);
            install_distinct_pane_overlay_for_lifecycle_test(
                &mut tab_app,
                class,
                "closed-tab",
                0,
                0,
            );
            tab_app
                .dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
            install_distinct_pane_overlay_for_lifecycle_test(
                &mut tab_app,
                class,
                "surviving-tab",
                0,
                0,
            );
            tab_app
                .dispatch_app_action(AppAction::CloseTab {
                    tab: rssh_core::TabId::new(1),
                    switch_to_last_active: false,
                })
                .unwrap();
            assert_pane_overlay_tag_for_lifecycle_test(&tab_app, class, "surviving-tab");
            assert!(
                !tab_app
                    .pane_runtimes
                    .contains_key(&rssh_core::PaneId::new(1))
            );
        }
    }

    #[test]
    fn window_app_close_active_pane_or_tab_restores_survivor_overlay() {
        for class in [
            PaneOverlayLifecycleClass::Search,
            PaneOverlayLifecycleClass::Copy,
            PaneOverlayLifecycleClass::Quick,
        ] {
            let mut pane_app = lifecycle_move_fixture(class, false);
            pane_app
                .dispatch_app_action(AppAction::ClosePane {
                    pane: rssh_core::PaneId::new(2),
                })
                .unwrap();
            assert_eq!(pane_app.active_pane_id(), rssh_core::PaneId::new(1));
            assert_pane_overlay_tag_for_lifecycle_test(&pane_app, class, "move-source");

            let mut tab_app = NativeWindowApp::new(None);
            install_distinct_pane_overlay_for_lifecycle_test(
                &mut tab_app,
                class,
                "surviving-tab",
                0,
                0,
            );
            tab_app
                .dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
            install_distinct_pane_overlay_for_lifecycle_test(
                &mut tab_app,
                class,
                "closed-tab",
                0,
                0,
            );
            tab_app
                .dispatch_app_action(AppAction::CloseTab {
                    tab: rssh_core::TabId::new(2),
                    switch_to_last_active: false,
                })
                .unwrap();
            assert_pane_overlay_tag_for_lifecycle_test(&tab_app, class, "surviving-tab");
        }
    }

    #[test]
    fn window_app_new_split_starts_with_empty_overlay_slot() {
        for class in [
            PaneOverlayLifecycleClass::Search,
            PaneOverlayLifecycleClass::Copy,
            PaneOverlayLifecycleClass::Quick,
        ] {
            let mut app = NativeWindowApp::new(None);
            install_distinct_pane_overlay_for_lifecycle_test(&mut app, class, "split-source", 0, 0);
            app.dispatch_app_action(AppAction::SplitPane {
                pane: rssh_core::PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
            assert!(!overlay_active_for_test(&app));
            assert!(
                app.pane_runtimes
                    .get(&rssh_core::PaneId::new(1))
                    .is_some_and(|runtime| runtime.ui.overlay_active())
            );
        }
    }

    #[test]
    fn window_app_pane_focus_saves_and_restores_each_overlay_class() {
        for class in [
            PaneOverlayLifecycleClass::Search,
            PaneOverlayLifecycleClass::Copy,
            PaneOverlayLifecycleClass::Quick,
        ] {
            let mut app = NativeWindowApp::new(None);
            let a_title =
                install_distinct_pane_overlay_for_lifecycle_test(&mut app, class, "pane-a", 1, 2);
            app.dispatch_app_action(AppAction::SplitPane {
                pane: rssh_core::PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
            let b_title =
                install_distinct_pane_overlay_for_lifecycle_test(&mut app, class, "pane-b", 3, 4);
            assert_ne!(a_title, b_title, "{class:?} pane owner titles");

            for _ in 0..2 {
                app.dispatch_app_action(AppAction::ActivatePane {
                    pane: rssh_core::PaneId::new(1),
                })
                .unwrap();
                assert_distinct_pane_overlay_for_lifecycle_test(
                    &app, class, "pane-a", 1, 2, &a_title,
                );
                app.dispatch_app_action(AppAction::ActivatePane {
                    pane: rssh_core::PaneId::new(2),
                })
                .unwrap();
                assert_distinct_pane_overlay_for_lifecycle_test(
                    &app, class, "pane-b", 3, 4, &b_title,
                );
            }
        }
    }

    #[test]
    fn window_app_tab_switch_saves_and_restores_each_overlay_class() {
        for class in [
            PaneOverlayLifecycleClass::Search,
            PaneOverlayLifecycleClass::Copy,
            PaneOverlayLifecycleClass::Quick,
        ] {
            let mut app = NativeWindowApp::new(None);
            let a_title =
                install_distinct_pane_overlay_for_lifecycle_test(&mut app, class, "tab-a", 1, 2);
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
            let b_title =
                install_distinct_pane_overlay_for_lifecycle_test(&mut app, class, "tab-b", 3, 4);
            assert_ne!(a_title, b_title, "{class:?} tab owner titles");

            for _ in 0..2 {
                app.dispatch_app_action(AppAction::ActivateTab {
                    tab: rssh_core::TabId::new(1),
                })
                .unwrap();
                assert_distinct_pane_overlay_for_lifecycle_test(
                    &app, class, "tab-a", 1, 2, &a_title,
                );
                app.dispatch_app_action(AppAction::ActivateTab {
                    tab: rssh_core::TabId::new(2),
                })
                .unwrap();
                assert_distinct_pane_overlay_for_lifecycle_test(
                    &app, class, "tab-b", 3, 4, &b_title,
                );
            }
        }
    }

    #[test]
    fn window_app_workspace_switch_saves_and_restores_each_overlay_class() {
        for class in [
            PaneOverlayLifecycleClass::Search,
            PaneOverlayLifecycleClass::Copy,
            PaneOverlayLifecycleClass::Quick,
        ] {
            let mut app = NativeWindowApp::new(None);
            let a_title = install_distinct_pane_overlay_for_lifecycle_test(
                &mut app,
                class,
                "workspace-a",
                1,
                2,
            );
            app.dispatch_app_action(AppAction::NewWorkspace {
                name: "overlay-workspace".to_owned(),
                launch: None,
            })
            .unwrap();
            let b_title = install_distinct_pane_overlay_for_lifecycle_test(
                &mut app,
                class,
                "workspace-b",
                3,
                4,
            );
            assert_ne!(a_title, b_title, "{class:?} workspace owner titles");

            for _ in 0..2 {
                app.dispatch_app_action(AppAction::SwitchWorkspace {
                    workspace: rssh_core::WorkspaceId::new(1),
                })
                .unwrap();
                assert_distinct_pane_overlay_for_lifecycle_test(
                    &app,
                    class,
                    "workspace-a",
                    1,
                    2,
                    &a_title,
                );
                app.dispatch_app_action(AppAction::SwitchWorkspace {
                    workspace: rssh_core::WorkspaceId::new(2),
                })
                .unwrap();
                assert_distinct_pane_overlay_for_lifecycle_test(
                    &app,
                    class,
                    "workspace-b",
                    3,
                    4,
                    &b_title,
                );
            }
        }
    }

    #[test]
    fn window_app_multiple_partial_failure_restores_shell_runtime_and_ui_owner() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;OwnerOne\x07owner-one")
            .unwrap();
        let expected_title = install_distinct_pane_overlay_for_lifecycle_test(
            &mut app,
            PaneOverlayLifecycleClass::Search,
            "rollback-owner",
            0,
            1,
        );
        let shell_before = app.app_shell.clone();
        let pane_ids_before = app.app_shell.pane_ids();
        app.refresh_snapshot();
        let snapshot_before = app.snapshot.clone();

        assert!(
            app.dispatch_app_action(AppAction::Multiple {
                actions: vec![
                    AppAction::NewTab { launch: None },
                    AppAction::SpawnWindow { launch: None },
                    AppAction::ActivatePane {
                        pane: rssh_core::PaneId::new(999),
                    },
                ],
            })
            .is_err()
        );

        assert_eq!(app.app_shell, shell_before);
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(1));
        assert_eq!(app.app_shell.pane_ids(), pane_ids_before);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 9), "owner-one");
        assert_eq!(app.snapshot, snapshot_before);
        assert_eq!(app.effective_window_title(), expected_title);
        assert_eq!(
            search_for_test(&app).map(|search| search.query.as_str()),
            Some("rollback-owner")
        );
        assert!(app.pane_runtimes.is_empty());
        assert!(app.app_shell.pending_windows().is_empty());
    }

    #[test]
    fn window_app_pane_switch_never_promotes_overlay_projection_to_ordinary_selection() {
        for class in [
            PaneOverlayLifecycleClass::Search,
            PaneOverlayLifecycleClass::Copy,
            PaneOverlayLifecycleClass::Quick,
        ] {
            let mut app = NativeWindowApp::new(None);
            install_distinct_pane_overlay_for_lifecycle_test(&mut app, class, "projection", 1, 2);
            app.selection = Some(WindowSelection::new(
                SelectionCell { row: 0, column: 1 },
                SelectionCell { row: 0, column: 4 },
            ));
            app.dispatch_app_action(AppAction::SplitPane {
                pane: rssh_core::PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();

            let saved = app
                .pane_runtimes
                .get(&rssh_core::PaneId::new(1))
                .expect("saved overlay owner");
            assert!(saved.ui.ordinary_selection.is_none(), "{class:?}");
            app.dispatch_app_action(AppAction::ActivatePane {
                pane: rssh_core::PaneId::new(1),
            })
            .unwrap();
            assert!(overlay_active_for_test(&app), "{class:?}");
        }
    }

    #[test]
    fn window_app_dirty_ordinary_selection_remains_deferred_in_saved_inactive_overlay() {
        for class in [
            PaneOverlayLifecycleClass::Search,
            PaneOverlayLifecycleClass::Copy,
            PaneOverlayLifecycleClass::Quick,
        ] {
            let mut app = NativeWindowApp::new(None);
            app.runtime.resize(rssh_core::TerminalSize::new(16, 2));
            app.handle_pty_output(b"selected\r\nother").unwrap();
            set_ordinary_viewport_range_for_test(
                &mut app,
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 3 },
            );
            install_distinct_pane_overlay_for_lifecycle_test(&mut app, class, "dirty", 0, 2);
            app.handle_pty_output(b"\x1b[1;1HX").unwrap();

            app.dispatch_app_action(AppAction::SplitPane {
                pane: rssh_core::PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
            let saved = app
                .pane_runtimes
                .get(&rssh_core::PaneId::new(1))
                .expect("saved dirty overlay owner");
            assert!(saved.ui.ordinary_selection.is_some(), "{class:?}");

            app.dispatch_app_action(AppAction::ActivatePane {
                pane: rssh_core::PaneId::new(1),
            })
            .unwrap();
            assert!(overlay_active_for_test(&app), "{class:?}");
            app.active_ui.exit_overlay();
            app.refresh_snapshot();
            assert!(ordinary_selection_for_test(&app).is_none(), "{class:?}");
        }
    }

    #[test]
    fn window_app_clicking_inactive_pane_passes_click_through_by_default() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(1));
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 1 },
                SelectionCell { row: 0, column: 1 },
            ))
        );
    }

    #[test]
    fn window_app_swallow_mouse_click_on_pane_focus_only_focuses_inactive_pane() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            swallow_mouse_click_on_pane_focus: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(1));
        assert!(app.selection.is_none());
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_shift_click_bypasses_mouse_reporting_by_default() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.handle_pty_output(b"\x1b[?1000;1006h").unwrap();
        app.modifiers = ModifiersState::SHIFT;

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
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 1 },
                SelectionCell { row: 0, column: 1 },
            ))
        );
        assert!(app.selecting);
    }

    #[test]
    fn window_app_bypass_mouse_reporting_modifiers_can_be_reconfigured() {
        let shift_written = Arc::new(Mutex::new(Vec::new()));
        let mut shift_app = NativeWindowApp::new(None);
        shift_app.writer = Some(Box::new(SharedWriter(Arc::clone(&shift_written))));
        shift_app.set_config_overrides(native_config_snapshot! {
            bypass_mouse_reporting_modifiers: Some(ModifiersState::ALT),
            ..NativeConfigSnapshot::default()
        });
        shift_app.handle_pty_output(b"\x1b[?1000;1006h").unwrap();
        shift_app.modifiers = ModifiersState::SHIFT;
        shift_app
            .handle_cursor_moved(PhysicalPosition::new(
                f64::from(CELL_WIDTH),
                f64::from(tab_bar_pixel_height()),
            ))
            .unwrap();

        assert!(
            shift_app
                .handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(shift_written.lock().unwrap().as_slice(), b"\x1b[<4;2;1M");
        assert!(shift_app.selection.is_none());

        let alt_written = Arc::new(Mutex::new(Vec::new()));
        let mut alt_app = NativeWindowApp::new(None);
        alt_app.writer = Some(Box::new(SharedWriter(Arc::clone(&alt_written))));
        alt_app.set_config_overrides(native_config_snapshot! {
            bypass_mouse_reporting_modifiers: Some(ModifiersState::ALT),
            ..NativeConfigSnapshot::default()
        });
        alt_app.handle_pty_output(b"\x1b[?1000;1006h").unwrap();
        alt_app.modifiers = ModifiersState::ALT;
        alt_app
            .handle_cursor_moved(PhysicalPosition::new(
                f64::from(CELL_WIDTH),
                f64::from(tab_bar_pixel_height()),
            ))
            .unwrap();

        assert!(
            alt_app
                .handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert!(alt_written.lock().unwrap().is_empty());
        assert_eq!(
            alt_app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 1 },
                SelectionCell { row: 0, column: 1 },
            ))
        );
    }

    #[test]
    fn window_app_mouse_input_before_os_focus_does_not_claim_focus() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let changes = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&changes);
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.focus_change_handler = Box::new(move |change| {
            recorded.lock().unwrap().push(*change);
            true
        });
        app.set_config_overrides(native_config_snapshot! {
            swallow_mouse_click_on_window_focus: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[?1000;1006h").unwrap();
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
        assert!(!app.window_focused);
        assert!(!app.mouse_click_may_focus_window);
        assert!(app.handle_focus_changed(true).unwrap());
        assert!(app.mouse_click_may_focus_window);

        assert_eq!(
            changes.lock().unwrap().as_slice(),
            [NativeWindowFocusChange {
                window_id: rssh_core::WindowId::new(1),
                pane: rssh_core::PaneId::new(1),
                focused: true,
            }]
        );

        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );
        assert!(!app.mouse_click_may_focus_window);
        written.lock().unwrap().clear();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b[<0;2;1M");
    }

    #[test]
    fn window_app_swallow_mouse_click_on_window_focus_consumes_focus_click() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.set_config_overrides(native_config_snapshot! {
            swallow_mouse_click_on_window_focus: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[?1000;1006h").unwrap();
        assert!(!app.handle_focus_changed(false).unwrap());
        assert!(app.handle_focus_changed(true).unwrap());

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
        assert!(app.selection.is_none());
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_mouse_click_on_window_focus_passes_through_when_disabled() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.set_config_overrides(native_config_snapshot! {
            swallow_mouse_click_on_window_focus: Some(false),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[?1000;1006h").unwrap();
        assert!(!app.handle_focus_changed(false).unwrap());
        assert!(app.handle_focus_changed(true).unwrap());

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b[<0;2;1M");
        assert!(app.selection.is_none());
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_pane_focus_follows_mouse_moves_focus_when_enabled() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));

        app.set_config_overrides(native_config_snapshot! {
            pane_focus_follows_mouse: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH * 2),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();

        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(1));
    }

    #[test]
    fn window_app_wheel_target_hits_inactive_pane_without_focusing() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rssh_core::TerminalSize::new(80, 4));
        app.refresh_snapshot();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
        let inactive = app
            .pane_render_layout()
            .panes
            .into_iter()
            .find(|rect| rect.pane_id == rssh_core::PaneId::new(1))
            .expect("inactive pane render rect");
        app.mouse_pixel_position = Some(PhysicalPosition::new(
            f64::from(app.frame_content_pixel_left())
                + f64::from(inactive.column) * f64::from(CELL_WIDTH),
            f64::from(app.terminal_pixel_top())
                + f64::from(inactive.row - app.terminal_frame_row_offset())
                    * f64::from(CELL_HEIGHT),
        ));

        let target = app
            .wheel_hit_target_at_mouse_position()
            .expect("inactive pane wheel target");

        assert!(matches!(
            target,
            super::WheelHitTarget::PaneSurface(super::WheelTarget { pane_id, .. })
                if pane_id == rssh_core::PaneId::new(1)
        ));
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    #[test]
    fn window_app_wheel_target_cell_is_local_to_inactive_split() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rssh_core::TerminalSize::new(80, 6));
        app.refresh_snapshot();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        let inactive = app
            .pane_render_layout()
            .panes
            .into_iter()
            .find(|rect| rect.pane_id == rssh_core::PaneId::new(1))
            .expect("inactive pane render rect");
        let local_x = f64::from(3 * CELL_WIDTH) + 2.5;
        let local_y = f64::from(CELL_HEIGHT) + 4.25;
        app.mouse_pixel_position = Some(PhysicalPosition::new(
            f64::from(app.frame_content_pixel_left())
                + f64::from(inactive.column) * f64::from(CELL_WIDTH)
                + local_x,
            f64::from(app.terminal_pixel_top())
                + f64::from(inactive.row - app.terminal_frame_row_offset())
                    * f64::from(CELL_HEIGHT)
                + local_y,
        ));

        let super::WheelHitTarget::PaneSurface(target) = app
            .wheel_hit_target_at_mouse_position()
            .expect("inactive pane wheel target")
        else {
            panic!("expected pane surface target");
        };

        assert_eq!(target.pane_id, rssh_core::PaneId::new(1));
        assert_eq!(target.rect, inactive);
        assert_eq!(
            target.cell,
            super::PaneMouseCell {
                pane_id: rssh_core::PaneId::new(1),
                row: 1,
                column: 3,
            }
        );
        assert_eq!(
            target.pixel_position,
            PhysicalPosition::new(local_x, local_y)
        );
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    #[test]
    fn window_app_wheel_target_rejects_split_separator_and_outside_terminal() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rssh_core::TerminalSize::new(80, 6));
        app.refresh_snapshot();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        let separator = app.pane_render_layout().separators[0];
        app.mouse_pixel_position = Some(PhysicalPosition::new(
            f64::from(app.frame_content_pixel_left())
                + f64::from(separator.column) * f64::from(CELL_WIDTH),
            f64::from(app.terminal_pixel_top())
                + f64::from(separator.row - app.terminal_frame_row_offset())
                    * f64::from(CELL_HEIGHT),
        ));
        assert_eq!(app.wheel_hit_target_at_mouse_position(), None);

        let first_pane = app.pane_render_layout().panes[0];
        app.mouse_pixel_position = Some(PhysicalPosition::new(
            f64::from(app.frame_content_pixel_left())
                + f64::from(first_pane.column) * f64::from(CELL_WIDTH),
            f64::from(app.terminal_pixel_bottom()),
        ));
        assert_eq!(app.wheel_hit_target_at_mouse_position(), None);

        app.set_config_overrides(native_config_snapshot! {
            window_padding: Some(NativeWindowPadding {
                left: NativeWindowPaddingDimension::Pixels(CELL_WIDTH),
                right: NativeWindowPaddingDimension::Pixels(0),
                top: NativeWindowPaddingDimension::Pixels(CELL_HEIGHT),
                bottom: NativeWindowPaddingDimension::Pixels(0),
            }),
            ..NativeConfigSnapshot::default()
        });
        app.mouse_pixel_position = Some(PhysicalPosition::new(
            f64::from(app.frame_content_pixel_left().saturating_sub(1)),
            f64::from(app.terminal_pixel_top().saturating_sub(1)),
        ));
        assert_eq!(app.wheel_hit_target_at_mouse_position(), None);

        app.mouse_pixel_position = None;
        assert_eq!(app.wheel_hit_target_at_mouse_position(), None);
    }

    #[test]
    fn window_app_wheel_target_zoom_uses_only_visible_pane() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rssh_core::TerminalSize::new(80, 4));
        app.refresh_snapshot();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        let inactive_before_zoom = app
            .pane_render_layout()
            .panes
            .into_iter()
            .find(|rect| rect.pane_id == rssh_core::PaneId::new(1))
            .expect("inactive pane render rect");
        app.dispatch_app_action(AppAction::SetPaneZoomState {
            pane: rssh_core::PaneId::new(2),
            zoomed: true,
        })
        .unwrap();
        app.mouse_pixel_position = Some(PhysicalPosition::new(
            f64::from(app.frame_content_pixel_left())
                + f64::from(inactive_before_zoom.column) * f64::from(CELL_WIDTH),
            f64::from(app.terminal_pixel_top())
                + f64::from(inactive_before_zoom.row - app.terminal_frame_row_offset())
                    * f64::from(CELL_HEIGHT),
        ));

        let super::WheelHitTarget::PaneSurface(target) = app
            .wheel_hit_target_at_mouse_position()
            .expect("zoomed pane wheel target")
        else {
            panic!("expected pane surface target");
        };

        assert_eq!(target.pane_id, rssh_core::PaneId::new(2));
        assert_eq!(app.pane_render_layout().panes, vec![target.rect]);
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    #[test]
    fn window_app_wheel_target_scrollbar_over_inactive_right_split_targets_active_left() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rssh_core::TerminalSize::new(
            u16::try_from(FRAME_WIDTH / CELL_WIDTH).unwrap(),
            4,
        ));
        app.refresh_snapshot();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rssh_core::PaneId::new(1),
        })
        .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            enable_scroll_bar: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"00\r\n01\r\n02\r\n03\r\n04\r\n05")
            .unwrap();
        let position = PhysicalPosition::new(
            f64::from(FRAME_WIDTH - 1),
            f64::from(app.terminal_pixel_top()),
        );
        let inactive_right = app
            .pane_render_rect(rssh_core::PaneId::new(2))
            .expect("inactive right pane render rect");
        assert!(
            app.wheel_target_for_rect(inactive_right, position)
                .is_some(),
            "fixture must place the inactive right pane beneath the scrollbar overlay"
        );
        assert!(app.scrollbar_hit_test(position));
        app.mouse_pixel_position = Some(position);

        assert_eq!(
            app.wheel_hit_target_at_mouse_position(),
            Some(super::WheelHitTarget::ActiveScrollbar {
                pane_id: rssh_core::PaneId::new(1),
            })
        );
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(1));
    }

    #[test]
    fn window_app_wheel_target_scrollbar_over_active_right_split_targets_active_right() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rssh_core::TerminalSize::new(
            u16::try_from(FRAME_WIDTH / CELL_WIDTH).unwrap(),
            4,
        ));
        app.refresh_snapshot();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            enable_scroll_bar: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"00\r\n01\r\n02\r\n03\r\n04\r\n05")
            .unwrap();
        let position = PhysicalPosition::new(
            f64::from(FRAME_WIDTH - 1),
            f64::from(app.terminal_pixel_top()),
        );
        let active_right = app
            .pane_render_rect(rssh_core::PaneId::new(2))
            .expect("active right pane render rect");
        assert!(
            app.wheel_target_for_rect(active_right, position).is_some(),
            "fixture must place the active right pane beneath the scrollbar overlay"
        );
        assert!(app.scrollbar_hit_test(position));
        app.mouse_pixel_position = Some(position);

        assert_eq!(
            app.wheel_hit_target_at_mouse_position(),
            Some(super::WheelHitTarget::ActiveScrollbar {
                pane_id: rssh_core::PaneId::new(2),
            })
        );
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    fn move_wheel_to_pane_cell_for_test(
        app: &mut NativeWindowApp,
        pane_id: rssh_core::PaneId,
        row: u16,
        column: u16,
        pixel_x: f64,
        pixel_y: f64,
    ) {
        let rect = app.pane_render_rect(pane_id).expect("visible pane rect");
        let cell_width = app.cell_width();
        let cell_height = app.cell_height();
        app.mouse_pixel_position = Some(PhysicalPosition::new(
            f64::from(app.frame_content_pixel_left())
                + f64::from(rect.column.saturating_add(column)) * f64::from(cell_width)
                + pixel_x,
            f64::from(app.terminal_pixel_top())
                + f64::from(
                    rect.row
                        .saturating_sub(app.terminal_frame_row_offset())
                        .saturating_add(row),
                ) * f64::from(cell_height)
                + pixel_y,
        ));
    }

    fn wheel_split_with_inactive_history_for_test() -> NativeWindowApp {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rssh_core::TerminalSize::new(20, 2));
        app.handle_pty_output(b"left-0\r\nleft-1\r\nleft-2\r\nleft-live")
            .unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"right-live").unwrap();
        app
    }

    #[test]
    fn window_app_wheel_scrolls_inactive_pane_without_focus_transfer() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let active = rssh_core::PaneId::new(2);
        let active_snapshot = app.snapshot.clone();
        let inactive_snapshot = app.pane_snapshot(inactive).unwrap().clone();
        let active_title = app.effective_window_title();
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert_eq!(app.active_pane_id(), active);
        assert_eq!(app.current_scrollback_offset(), 0);
        assert_eq!(app.snapshot, active_snapshot);
        assert_eq!(app.effective_window_title(), active_title);
        assert_ne!(app.pane_snapshot(inactive).unwrap(), &inactive_snapshot);
        assert!(
            app.pane_runtimes
                .get(&inactive)
                .unwrap()
                .ui
                .stable_viewport
                .main_top
                .is_some()
        );
    }

    #[test]
    fn window_app_wheel_refreshes_only_target_selection_overlay_and_composite() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let selection_word_boundary = app.selection_word_boundary.clone();
        let inactive_runtime = app.pane_runtimes.get_mut(&inactive).unwrap();
        let dimensions = inactive_runtime.runtime.terminal().stable_dimensions();
        inactive_runtime.ui.ordinary_selection = Some(StableOrdinarySelection::new(
            SelectionSourceCell {
                domain: dimensions.domain,
                row: dimensions.physical_top,
                column: 0,
            },
            SelectionSourceCell {
                domain: dimensions.domain,
                row: dimensions.physical_top,
                column: 1,
            },
            inactive_runtime.runtime.terminal().current_seqno(),
        ));
        let inactive_projection_before = super::pane_overlay_viewport_selection(
            inactive_runtime.runtime.terminal(),
            &inactive_runtime.ui,
            &selection_word_boundary,
        );
        assert!(inactive_projection_before.is_some());
        let active_match = pane_overlay_test_match(&app, 0, 0, 1);
        install_pane_search_presentation_for_test(&mut app, "right", active_match);
        let active_snapshot = app.snapshot.clone();
        let before = app.render_snapshot();
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert_eq!(app.snapshot, active_snapshot);
        assert_ne!(app.render_snapshot(), before);
        let inactive_runtime = app.pane_runtimes.get(&inactive).unwrap();
        assert!(inactive_runtime.ui.ordinary_selection.is_some());
        assert_ne!(
            super::pane_overlay_viewport_selection(
                inactive_runtime.runtime.terminal(),
                &inactive_runtime.ui,
                &app.selection_word_boundary,
            ),
            inactive_projection_before
        );
    }

    #[test]
    fn window_app_wheel_inactive_pane_without_history_matches_active_noop() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rssh_core::TerminalSize::new(20, 2));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        let inactive = rssh_core::PaneId::new(1);
        let before = app.pane_snapshot(inactive).unwrap().clone();
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
        assert_eq!(app.pane_snapshot(inactive).unwrap(), &before);
    }

    #[test]
    fn window_app_wheel_active_scrollbar_over_inactive_right_uses_active_scrollback_only() {
        let active_written = Arc::new(Mutex::new(Vec::new()));
        let mut app = wheel_split_with_inactive_history_for_test();
        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rssh_core::PaneId::new(1),
        })
        .unwrap();
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&active_written))));
        app.handle_pty_output(b"\x1b[?1000;1006h").unwrap();
        app.mouse_assignments = vec![NativeUserMouseAssignment {
            event: NativeMouseAssignmentEvent {
                kind: NativeMouseAssignmentEventKind::Down,
                button: NativeMouseAssignmentButton::WheelUp,
                streak: 1,
            },
            modifiers: ModifiersState::empty(),
            mouse_reporting: true,
            alt_screen: NativeMouseAssignmentAltScreen::Any,
            command: WindowCommand::SendString("bound".to_owned()),
        }];
        app.set_config_overrides(native_config_snapshot! {
            enable_scroll_bar: Some(true),
            ..NativeConfigSnapshot::default()
        });
        let inactive_before = app
            .pane_snapshot(rssh_core::PaneId::new(2))
            .unwrap()
            .clone();
        app.mouse_pixel_position = Some(PhysicalPosition::new(
            f64::from(FRAME_WIDTH - 1),
            f64::from(app.terminal_pixel_top()),
        ));

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );
        assert!(app.current_scrollback_offset() > 0);
        assert_eq!(
            app.pane_snapshot(rssh_core::PaneId::new(2)).unwrap(),
            &inactive_before
        );
        assert!(active_written.lock().unwrap().is_empty());
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(1));
    }

    #[test]
    fn window_app_wheel_active_scrollbar_over_active_right_uses_active_scrollback_only() {
        let active_written = Arc::new(Mutex::new(Vec::new()));
        let mut app = wheel_split_with_inactive_history_for_test();
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&active_written))));
        app.handle_pty_output(b"right-0\r\nright-1\r\nright-2\r\nright-live")
            .unwrap();
        app.handle_pty_output(b"\x1b[?1049h").unwrap();
        app.set_config_overrides(native_config_snapshot! {
            enable_scroll_bar: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.mouse_pixel_position = Some(PhysicalPosition::new(
            f64::from(FRAME_WIDTH - 1),
            f64::from(app.terminal_pixel_top()),
        ));

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );
        assert!(active_written.lock().unwrap().is_empty());
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    fn install_inactive_wheel_writer_for_test(
        app: &mut NativeWindowApp,
        pane_id: rssh_core::PaneId,
    ) -> Arc<Mutex<Vec<u8>>> {
        let written = Arc::new(Mutex::new(Vec::new()));
        app.pane_runtimes.get_mut(&pane_id).unwrap().writer =
            Some(Box::new(SharedWriter(Arc::clone(&written))));
        written
    }

    #[test]
    fn window_app_wheel_reports_local_cell_to_inactive_target_writer() {
        let active_written = Arc::new(Mutex::new(Vec::new()));
        let mut app = wheel_split_with_inactive_history_for_test();
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&active_written))));
        let inactive = rssh_core::PaneId::new(1);
        let inactive_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        app.handle_pane_pty_output(inactive, b"\x1b[?1000;1006h")
            .unwrap();
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 1, 3, 2.5, 4.25);

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert_eq!(
            inactive_written.lock().unwrap().as_slice(),
            b"\x1b[<64;4;2M"
        );
        assert!(active_written.lock().unwrap().is_empty());
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    #[test]
    fn window_app_wheel_reports_local_sgr_pixel_to_inactive_target_writer() {
        let active_written = Arc::new(Mutex::new(Vec::new()));
        let mut app = wheel_split_with_inactive_history_for_test();
        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rssh_core::PaneId::new(1),
        })
        .unwrap();
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&active_written))));
        let inactive = rssh_core::PaneId::new(2);
        let cell_width = app.cell_width();
        let cell_height = app.cell_height();
        let unpadded_rect = app.pane_render_rect(inactive).unwrap();
        app.set_config_overrides(native_config_snapshot! {
            window_padding: Some(NativeWindowPadding {
                left: NativeWindowPaddingDimension::Pixels(cell_width.saturating_add(3)),
                right: NativeWindowPaddingDimension::Pixels(5),
                top: NativeWindowPaddingDimension::Pixels(cell_height.saturating_add(2)),
                bottom: NativeWindowPaddingDimension::Pixels(7),
            }),
            ..NativeConfigSnapshot::default()
        });
        let inactive_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        app.handle_pane_pty_output(inactive, b"\x1b[?1000;1016h")
            .unwrap();
        let inactive_rect = app.pane_render_rect(inactive).unwrap();
        assert!(inactive_rect.column > 0, "target must be the right split");
        assert_eq!(
            inactive_rect.column, unpadded_rect.column,
            "physical padding is an outer margin, not a terminal-cell offset"
        );
        assert_eq!(
            inactive_rect.row, unpadded_rect.row,
            "physical padding is an outer margin, not a terminal-row offset"
        );
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 3, 2.5, 4.25);

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        let expected = format!("\x1b[<64;{};5M", 3 * cell_width + 3);
        assert_eq!(
            inactive_written.lock().unwrap().as_slice(),
            expected.as_bytes()
        );
        assert!(active_written.lock().unwrap().is_empty());
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(1));
    }

    #[test]
    fn window_app_wheel_reporting_scrolls_target_to_bottom_before_report() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let inactive_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        app.handle_pane_pty_output(inactive, b"\x1b[?1000;1006h")
            .unwrap();
        app.pane_runtimes
            .get_mut(&inactive)
            .unwrap()
            .ui
            .stable_viewport
            .main_top = Some(0);
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert_eq!(
            app.pane_runtimes
                .get(&inactive)
                .unwrap()
                .ui
                .stable_viewport
                .main_top,
            None
        );
        assert_eq!(
            inactive_written.lock().unwrap().as_slice(),
            b"\x1b[<64;1;1M"
        );
    }

    #[test]
    fn window_app_wheel_reporting_bypass_keeps_target_viewport_and_scrolls_normally() {
        let active_written = Arc::new(Mutex::new(Vec::new()));
        let mut app = wheel_split_with_inactive_history_for_test();
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&active_written))));
        let inactive = rssh_core::PaneId::new(1);
        let inactive_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        app.handle_pane_pty_output(
            inactive,
            b"left-extra-0\r\nleft-extra-1\r\nleft-extra-2\r\nleft-extra-3\r\nleft-live",
        )
        .unwrap();
        app.handle_pane_pty_output(inactive, b"\x1b[?1000;1006h")
            .unwrap();
        assert!(app.set_pane_scrollback_offset(inactive, 2));
        let target_viewport_before = app.pane_runtimes.get(&inactive).unwrap().ui.stable_viewport;
        let target_offset_before = target_viewport_before
            .scrollback_offset(app.pane_runtimes.get(&inactive).unwrap().runtime.terminal());
        assert_eq!(target_offset_before, 2);
        let target_snapshot_before = app.pane_snapshot(inactive).unwrap().clone();
        let active = app.active_pane_id();
        app.modifiers = ModifiersState::SHIFT;
        app.mouse_assignments = vec![
            NativeUserMouseAssignment {
                event: NativeMouseAssignmentEvent {
                    kind: NativeMouseAssignmentEventKind::Down,
                    button: NativeMouseAssignmentButton::WheelUp,
                    streak: 1,
                },
                modifiers: ModifiersState::empty(),
                mouse_reporting: false,
                alt_screen: NativeMouseAssignmentAltScreen::Any,
                command: WindowCommand::Nop,
            },
            NativeUserMouseAssignment {
                event: NativeMouseAssignmentEvent {
                    kind: NativeMouseAssignmentEventKind::Down,
                    button: NativeMouseAssignmentButton::WheelUp,
                    streak: 1,
                },
                modifiers: ModifiersState::SHIFT,
                mouse_reporting: false,
                alt_screen: NativeMouseAssignmentAltScreen::Any,
                command: WindowCommand::SendString("wrong-owner".to_owned()),
            },
        ];
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert!(inactive_written.lock().unwrap().is_empty());
        assert!(active_written.lock().unwrap().is_empty());
        assert_eq!(app.modifiers, ModifiersState::SHIFT);
        assert_eq!(
            app.pane_runtimes
                .get(&inactive)
                .unwrap()
                .ui
                .stable_viewport
                .main_top,
            target_viewport_before.main_top,
            "bypass assignment must not force the target viewport to bottom"
        );
        assert_eq!(
            app.pane_snapshot(inactive).unwrap(),
            &target_snapshot_before
        );
        assert_eq!(app.active_pane_id(), active);
        let active_snapshot_after_binding = app.snapshot.clone();
        app.mouse_assignments.clear();

        let normal_delta = MouseScrollDelta::LineDelta(0.0, 1.0);
        let history_len = app
            .pane_runtime_ref(inactive)
            .unwrap()
            .terminal()
            .scrollback()
            .len();
        let expected_offset = target_offset_before
            .saturating_add(super::scrollback_lines_from_mouse_delta(normal_delta).unsigned_abs())
            .min(history_len);
        assert!(app.handle_window_mouse_wheel(normal_delta).unwrap());
        assert!(inactive_written.lock().unwrap().is_empty());
        assert!(active_written.lock().unwrap().is_empty());
        assert_eq!(app.modifiers, ModifiersState::SHIFT);
        assert_eq!(
            app.pane_ui_ref(inactive)
                .unwrap()
                .stable_viewport
                .scrollback_offset(app.pane_runtime_ref(inactive).unwrap().terminal()),
            expected_offset,
            "normal bypass scroll must continue from the preserved target viewport"
        );
        assert_ne!(
            app.pane_snapshot(inactive).unwrap(),
            &target_snapshot_before
        );
        assert_eq!(app.snapshot, active_snapshot_after_binding);
        assert_eq!(app.active_pane_id(), active);
    }

    #[test]
    fn window_app_wheel_alternate_arrows_use_inactive_target_modes_and_writer() {
        let active_written = Arc::new(Mutex::new(Vec::new()));
        let mut app = wheel_split_with_inactive_history_for_test();
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&active_written))));
        let inactive = rssh_core::PaneId::new(1);
        let inactive_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        app.set_config_overrides(native_config_snapshot! {
            enable_kitty_keyboard: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[?1h\x1b[=0u").unwrap();
        app.handle_pane_pty_output(inactive, b"\x1b[?1049h\x1b[?1l\x1b[=1u")
            .unwrap();
        assert_eq!(app.runtime.kitty_keyboard_flags(), 0);
        assert!(app.runtime.application_cursor_keys());
        assert_eq!(
            app.pane_runtimes
                .get(&inactive)
                .unwrap()
                .runtime
                .kitty_keyboard_flags(),
            super::KITTY_KEYBOARD_DISAMBIGUATE
        );
        assert!(
            !app.pane_runtimes
                .get(&inactive)
                .unwrap()
                .runtime
                .application_cursor_keys()
        );
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert_eq!(
            inactive_written.lock().unwrap().as_slice(),
            b"\x1b[A\x1b[A\x1b[A"
        );
        assert!(active_written.lock().unwrap().is_empty());
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    #[test]
    fn window_app_wheel_missing_target_writer_is_consumed_without_active_fallback() {
        let active_written = Arc::new(Mutex::new(Vec::new()));
        let mut app = wheel_split_with_inactive_history_for_test();
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&active_written))));
        let inactive = rssh_core::PaneId::new(1);
        app.pane_runtimes.get_mut(&inactive).unwrap().writer = None;
        app.handle_pane_pty_output(inactive, b"\x1b[?1000;1006h")
            .unwrap();
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );
        assert!(active_written.lock().unwrap().is_empty());
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    #[test]
    fn window_app_wheel_restores_previous_delta_after_success_and_unhandled() {
        let sentinel = MouseScrollDelta::PixelDelta(PhysicalPosition::new(13.0, -17.0));
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        app.current_mouse_wheel_delta = Some(sentinel);
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );
        assert_eq!(app.current_mouse_wheel_delta, Some(sentinel));

        app.mouse_pixel_position = None;
        assert!(
            !app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );
        assert_eq!(app.current_mouse_wheel_delta, Some(sentinel));
    }

    #[test]
    fn window_app_wheel_restores_delta_and_refreshes_target_after_assignment_error() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let sentinel = MouseScrollDelta::PixelDelta(PhysicalPosition::new(-19.0, 23.0));
        app.current_mouse_wheel_delta = Some(sentinel);
        app.handle_pane_pty_output(inactive, b"\x1b[?1000h")
            .unwrap();
        assert!(app.set_pane_scrollback_offset(inactive, 1));
        let snapshot_before = app.pane_snapshot(inactive).unwrap().clone();
        let rebuilds_before = app.metrics.snapshot().snapshot_rebuilds;
        app.mouse_assignments = vec![NativeUserMouseAssignment {
            event: NativeMouseAssignmentEvent {
                kind: NativeMouseAssignmentEventKind::Down,
                button: NativeMouseAssignmentButton::WheelUp,
                streak: 1,
            },
            modifiers: ModifiersState::empty(),
            mouse_reporting: true,
            alt_screen: NativeMouseAssignmentAltScreen::Any,
            command: WindowCommand::CloseWorkspace,
        }];
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);

        let error = app
            .handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
            .expect_err("last workspace close must propagate");

        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(
            error.to_string(),
            "wheel action 'Close Workspace' failed: CannotCloseLastWorkspace"
        );
        assert_eq!(app.current_mouse_wheel_delta, Some(sentinel));
        assert_eq!(
            app.pane_ui_ref(inactive).unwrap().stable_viewport.main_top,
            None
        );
        assert_ne!(app.pane_snapshot(inactive).unwrap(), &snapshot_before);
        assert!(app.metrics.snapshot().snapshot_rebuilds > rebuilds_before);
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    #[test]
    fn window_app_wheel_restores_delta_and_refreshes_target_after_pty_error() {
        struct FailingWriter;

        impl Write for FailingWriter {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("intentional wheel writer failure"))
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let sentinel = MouseScrollDelta::LineDelta(5.0, -7.0);
        app.current_mouse_wheel_delta = Some(sentinel);
        app.pane_runtimes.get_mut(&inactive).unwrap().writer = Some(Box::new(FailingWriter));
        app.handle_pane_pty_output(inactive, b"\x1b[?1000;1006h")
            .unwrap();
        assert!(app.set_pane_scrollback_offset(inactive, 1));
        let snapshot_before = app.pane_snapshot(inactive).unwrap().clone();
        let rebuilds_before = app.metrics.snapshot().snapshot_rebuilds;
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);

        let error = app
            .handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
            .expect_err("target writer failure must propagate");

        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(error.to_string(), "intentional wheel writer failure");
        assert_eq!(app.current_mouse_wheel_delta, Some(sentinel));
        assert_eq!(
            app.pane_ui_ref(inactive).unwrap().stable_viewport.main_top,
            None
        );
        assert_ne!(app.pane_snapshot(inactive).unwrap(), &snapshot_before);
        assert!(app.metrics.snapshot().snapshot_rebuilds > rebuilds_before);
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    #[test]
    fn window_app_wheel_missing_runtime_returns_false_without_active_fallback() {
        let active_written = Arc::new(Mutex::new(Vec::new()));
        let mut app = wheel_split_with_inactive_history_for_test();
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&active_written))));
        let inactive = rssh_core::PaneId::new(1);
        let active_ui_before = app.active_ui.stable_viewport;
        let active_snapshot_before = app.snapshot.clone();
        app.pane_runtimes.remove(&inactive).unwrap();
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);

        assert!(
            !app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );
        assert!(active_written.lock().unwrap().is_empty());
        assert_eq!(app.active_ui.stable_viewport, active_ui_before);
        assert_eq!(app.snapshot, active_snapshot_before);
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    #[test]
    fn window_app_wheel_reporting_unencodable_returns_false_without_default_scroll() {
        let active_written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rssh_core::TerminalSize::new(500, 2));
        app.handle_pty_output(b"left-0\r\nleft-1\r\nleft-2\r\nleft-live")
            .unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&active_written))));
        let inactive = rssh_core::PaneId::new(1);
        let inactive_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        app.handle_pane_pty_output(inactive, b"\x1b[?1000h")
            .unwrap();
        assert!(app.set_pane_scrollback_offset(inactive, 1));
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 240, 1.0, 1.0);

        assert!(
            !app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );
        assert_eq!(
            app.pane_ui_ref(inactive).unwrap().stable_viewport.main_top,
            None,
            "an unencodable report must not fall through to default scrollback"
        );
        assert!(inactive_written.lock().unwrap().is_empty());
        assert!(active_written.lock().unwrap().is_empty());
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    #[test]
    fn window_app_wheel_sgr_pixels_rejects_unencodable_inactive_target_pixels() {
        let active_written = Arc::new(Mutex::new(Vec::new()));
        let mut app = wheel_split_with_inactive_history_for_test();
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&active_written))));
        let inactive = rssh_core::PaneId::new(1);
        let inactive_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        app.handle_pane_pty_output(inactive, b"\x1b[?1000;1016h")
            .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            font_size: Some(NativeFontSize::from_millipoints(120_000_000)),
            ..NativeConfigSnapshot::default()
        });
        assert!(
            app.cell_width() > u32::from(u16::MAX),
            "fixture needs a pane-local pixel offset that exceeds the protocol range"
        );
        assert!(app.set_pane_scrollback_offset(inactive, 1));
        let history_len_before = app
            .pane_runtime_ref(inactive)
            .unwrap()
            .terminal()
            .scrollback()
            .len();
        let active = app.active_pane_id();
        let sentinel_delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(29.0, -31.0));
        app.current_mouse_wheel_delta = Some(sentinel_delta);
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, f64::from(u16::MAX), 1.0);

        assert!(
            !app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert!(inactive_written.lock().unwrap().is_empty());
        assert!(active_written.lock().unwrap().is_empty());
        assert_eq!(
            app.pane_ui_ref(inactive).unwrap().stable_viewport.main_top,
            None,
            "an unencodable pixel report must not fall through to default scrollback"
        );
        assert_eq!(
            app.pane_runtime_ref(inactive)
                .unwrap()
                .terminal()
                .scrollback()
                .len(),
            history_len_before
        );
        assert_eq!(app.active_pane_id(), active);
        assert_eq!(app.current_mouse_wheel_delta, Some(sentinel_delta));
    }

    #[test]
    fn window_app_wheel_disable_default_suppresses_inactive_scrollback_and_returns_false() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let active = app.active_pane_id();
        let viewport_before = app.pane_ui_ref(inactive).unwrap().stable_viewport;
        app.current_mouse_wheel_delta = Some(MouseScrollDelta::LineDelta(7.0, 0.0));
        bind_wheel_command_for_test(&mut app, WindowCommand::DisableDefaultAssignment);
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);

        assert!(
            !app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert_eq!(app.active_pane_id(), active);
        assert_eq!(
            app.pane_ui_ref(inactive).unwrap().stable_viewport,
            viewport_before
        );
        assert_eq!(
            app.current_mouse_wheel_delta,
            Some(MouseScrollDelta::LineDelta(7.0, 0.0))
        );
    }

    #[test]
    fn window_app_wheel_disable_default_reporting_scrolls_bottom_but_emits_no_report() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let active = app.active_pane_id();
        let written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        app.handle_pane_pty_output(inactive, b"\x1b[?1000;1006h")
            .unwrap();
        assert!(app.set_pane_scrollback_offset(inactive, 1));
        let target_snapshot_before = app.pane_snapshot(inactive).unwrap().clone();
        let snapshot_rebuilds_before = app.metrics.snapshot().snapshot_rebuilds;
        let sentinel_delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(17.0, -23.0));
        app.current_mouse_wheel_delta = Some(sentinel_delta);
        app.mouse_assignments = vec![NativeUserMouseAssignment {
            event: NativeMouseAssignmentEvent {
                kind: NativeMouseAssignmentEventKind::Down,
                button: NativeMouseAssignmentButton::WheelUp,
                streak: 1,
            },
            modifiers: ModifiersState::empty(),
            mouse_reporting: true,
            alt_screen: NativeMouseAssignmentAltScreen::Any,
            command: WindowCommand::DisableDefaultAssignment,
        }];
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);

        assert!(
            !app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert_eq!(app.active_pane_id(), active);
        assert_eq!(
            app.pane_ui_ref(inactive).unwrap().stable_viewport.main_top,
            None
        );
        assert_ne!(
            app.pane_snapshot(inactive).unwrap(),
            &target_snapshot_before
        );
        assert!(app.metrics.snapshot().snapshot_rebuilds > snapshot_rebuilds_before);
        assert!(written.lock().unwrap().is_empty());
        assert_eq!(app.current_mouse_wheel_delta, Some(sentinel_delta));
    }

    #[test]
    fn window_app_wheel_disable_default_alternate_emits_no_arrow() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let active = app.active_pane_id();
        let written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        app.handle_pane_pty_output(inactive, b"\x1b[?1049h")
            .unwrap();
        let sentinel_delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(-31.0, 29.0));
        app.current_mouse_wheel_delta = Some(sentinel_delta);
        bind_wheel_command_for_test(&mut app, WindowCommand::DisableDefaultAssignment);
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);

        assert!(
            !app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert_eq!(app.active_pane_id(), active);
        assert!(written.lock().unwrap().is_empty());
        assert_eq!(app.current_mouse_wheel_delta, Some(sentinel_delta));
    }

    #[test]
    fn window_app_wheel_disable_default_bypass_matches_effective_modifiers() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let active = app.active_pane_id();
        let written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        app.handle_pane_pty_output(inactive, b"\x1b[?1000;1006h")
            .unwrap();
        assert!(app.set_pane_scrollback_offset(inactive, 1));
        let viewport_before = app.pane_ui_ref(inactive).unwrap().stable_viewport;
        app.modifiers = ModifiersState::SHIFT;
        app.current_mouse_wheel_delta = Some(MouseScrollDelta::LineDelta(9.0, 0.0));
        app.mouse_assignments = vec![
            NativeUserMouseAssignment {
                event: NativeMouseAssignmentEvent {
                    kind: NativeMouseAssignmentEventKind::Down,
                    button: NativeMouseAssignmentButton::WheelUp,
                    streak: 1,
                },
                modifiers: ModifiersState::empty(),
                mouse_reporting: false,
                alt_screen: NativeMouseAssignmentAltScreen::Any,
                command: WindowCommand::DisableDefaultAssignment,
            },
            NativeUserMouseAssignment {
                event: NativeMouseAssignmentEvent {
                    kind: NativeMouseAssignmentEventKind::Down,
                    button: NativeMouseAssignmentButton::WheelUp,
                    streak: 1,
                },
                modifiers: ModifiersState::SHIFT,
                mouse_reporting: false,
                alt_screen: NativeMouseAssignmentAltScreen::Any,
                command: WindowCommand::SendString("wrong-owner".to_owned()),
            },
        ];
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);

        assert!(
            !app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert_eq!(app.active_pane_id(), active);
        assert_eq!(
            app.pane_ui_ref(inactive).unwrap().stable_viewport,
            viewport_before
        );
        assert!(written.lock().unwrap().is_empty());
        assert_eq!(app.modifiers, ModifiersState::SHIFT);
        assert_eq!(
            app.current_mouse_wheel_delta,
            Some(MouseScrollDelta::LineDelta(9.0, 0.0))
        );
    }

    #[test]
    fn window_app_wheel_keeps_inactive_focus_title_and_active_scrollbar_state() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let active = rssh_core::PaneId::new(2);
        app.handle_pty_output(
            b"right-0\r\nright-1\r\nright-2\r\nright-live\x1b]0;active-right\x07",
        )
        .unwrap();
        assert!(app.set_pane_scrollback_offset(active, 1));
        app.left_status = "left-status-sentinel".to_owned();
        app.right_status = "right-status-sentinel".to_owned();
        let active_viewport_before = app.pane_ui_ref(active).unwrap().stable_viewport;
        let title_before = app.effective_window_title();
        let left_status_before = app.left_status.clone();
        let right_status_before = app.right_status.clone();
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert_eq!(app.active_pane_id(), active);
        assert_eq!(app.effective_window_title(), title_before);
        assert_eq!(app.left_status, left_status_before);
        assert_eq!(app.right_status, right_status_before);
        assert_eq!(
            app.pane_ui_ref(active).unwrap().stable_viewport,
            active_viewport_before,
            "inactive-pane wheel must not move the active scrollbar viewport"
        );
    }

    #[test]
    fn window_app_wheel_does_not_replace_independent_pane_selections_or_overlays() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let active = rssh_core::PaneId::new(2);
        app.dispatch_app_action(AppAction::ActivatePane { pane: inactive })
            .unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 1 },
        );
        install_distinct_pane_overlay_for_lifecycle_test(
            &mut app,
            PaneOverlayLifecycleClass::Search,
            "inactive-wheel-a",
            0,
            0,
        );
        app.dispatch_app_action(AppAction::ActivatePane { pane: active })
            .unwrap();
        app.handle_pty_output(b"active-overlay-owner").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 1 },
        );
        install_distinct_pane_overlay_for_lifecycle_test(
            &mut app,
            PaneOverlayLifecycleClass::Copy,
            "active-wheel-b",
            0,
            0,
        );

        let active_selection_before = app.active_ui.ordinary_selection;
        let active_search_before = app.active_ui.retained_search().unwrap();
        let active_copy_before = app.active_ui.retained_copy_mode().unwrap();
        let active_overlay_state_before = (
            app.active_ui.copy_search_mode(),
            active_search_before.query.clone(),
            active_search_before.match_type,
            active_search_before.current,
            active_search_before.editing,
            active_copy_before.selection_mode,
            active_copy_before.cursor,
        );
        let active_snapshot_before = app.snapshot.clone();
        let inactive_owner = app.pane_runtimes.get(&inactive).unwrap();
        let inactive_selection_before = inactive_owner.ui.ordinary_selection;
        let inactive_search_before = inactive_owner.ui.retained_search().unwrap();
        let inactive_search_state_before = (
            inactive_owner.ui.copy_search_mode(),
            inactive_search_before.query.clone(),
            inactive_search_before.match_type,
            inactive_search_before.current,
            inactive_search_before.editing,
        );
        let inactive_snapshot_before = inactive_owner.snapshot.clone();
        let active_rect = app.pane_render_rect(active).unwrap();
        let inactive_rect = app.pane_render_rect(inactive).unwrap();
        let composite_before = app.render_snapshot();
        let snapshot_rebuilds_before = app.metrics.snapshot().snapshot_rebuilds;
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert_eq!(app.active_pane_id(), active);
        assert_eq!(app.active_ui.ordinary_selection, active_selection_before);
        let active_search_after = app.active_ui.retained_search().unwrap();
        let active_copy_after = app.active_ui.retained_copy_mode().unwrap();
        assert_eq!(
            (
                app.active_ui.copy_search_mode(),
                active_search_after.query.clone(),
                active_search_after.match_type,
                active_search_after.current,
                active_search_after.editing,
                active_copy_after.selection_mode,
                active_copy_after.cursor,
            ),
            active_overlay_state_before
        );
        assert_eq!(app.snapshot, active_snapshot_before);
        let inactive_owner = app.pane_runtimes.get(&inactive).unwrap();
        assert_eq!(
            inactive_owner.ui.ordinary_selection,
            inactive_selection_before
        );
        let inactive_search_after = inactive_owner.ui.retained_search().unwrap();
        assert_eq!(
            (
                inactive_owner.ui.copy_search_mode(),
                inactive_search_after.query.clone(),
                inactive_search_after.match_type,
                inactive_search_after.current,
                inactive_search_after.editing,
            ),
            inactive_search_state_before
        );
        assert!(inactive_owner.ui.overlay_active());
        assert_ne!(inactive_owner.snapshot, inactive_snapshot_before);
        assert!(app.metrics.snapshot().snapshot_rebuilds > snapshot_rebuilds_before);

        let composite_after = app.render_snapshot();
        for row in active_rect.row..active_rect.row.saturating_add(active_rect.rows) {
            for column in active_rect.column..active_rect.column.saturating_add(active_rect.columns)
            {
                assert_eq!(
                    snapshot_cell(&composite_after, row, column),
                    snapshot_cell(&composite_before, row, column),
                    "active composite presentation changed at ({row}, {column})"
                );
            }
        }
        assert!(
            (inactive_rect.row..inactive_rect.row.saturating_add(inactive_rect.rows)).any(|row| {
                (inactive_rect.column..inactive_rect.column.saturating_add(inactive_rect.columns))
                    .any(|column| {
                        snapshot_cell(&composite_after, row, column)
                            != snapshot_cell(&composite_before, row, column)
                    })
            }),
            "target composite must visibly refresh while retaining its overlay owner"
        );
    }

    #[test]
    fn window_app_wheel_preserves_focus_follows_mouse_click_and_swallow_semantics() {
        let mut move_app = NativeWindowApp::new(None);
        move_app
            .dispatch_app_action(AppAction::SplitPane {
                pane: rssh_core::PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        move_app.set_config_overrides(native_config_snapshot! {
            pane_focus_follows_mouse: Some(true),
            ..NativeConfigSnapshot::default()
        });
        move_wheel_to_pane_cell_for_test(&mut move_app, rssh_core::PaneId::new(1), 0, 0, 1.0, 1.0);
        move_app
            .handle_cursor_moved(move_app.mouse_pixel_position.unwrap())
            .unwrap();
        assert_eq!(move_app.active_pane_id(), rssh_core::PaneId::new(1));

        let mut click_app = NativeWindowApp::new(None);
        click_app
            .dispatch_app_action(AppAction::SplitPane {
                pane: rssh_core::PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        move_wheel_to_pane_cell_for_test(&mut click_app, rssh_core::PaneId::new(1), 0, 0, 1.0, 1.0);
        click_app
            .handle_cursor_moved(click_app.mouse_pixel_position.unwrap())
            .unwrap();
        assert!(
            click_app
                .handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert_eq!(click_app.active_pane_id(), rssh_core::PaneId::new(1));
        assert!(click_app.selection.is_some());

        let mut swallow_app = NativeWindowApp::new(None);
        swallow_app.set_config_overrides(native_config_snapshot! {
            swallow_mouse_click_on_pane_focus: Some(true),
            ..NativeConfigSnapshot::default()
        });
        swallow_app
            .dispatch_app_action(AppAction::SplitPane {
                pane: rssh_core::PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        move_wheel_to_pane_cell_for_test(
            &mut swallow_app,
            rssh_core::PaneId::new(1),
            0,
            0,
            1.0,
            1.0,
        );
        swallow_app
            .handle_cursor_moved(swallow_app.mouse_pixel_position.unwrap())
            .unwrap();
        assert!(
            swallow_app
                .handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert_eq!(swallow_app.active_pane_id(), rssh_core::PaneId::new(1));
        assert!(swallow_app.selection.is_none());
        assert!(!swallow_app.selecting);
    }

    #[test]
    fn window_app_wheel_tab_bar_precedes_pane_routing() {
        let mut app = wheel_split_with_inactive_history_for_test();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        let tab_before = app.active_tab_id();
        let tabs_len_before = app.app_shell.active_workspace().tabs().len();
        let pane_count_before = app.app_shell.pane_ids().len();
        let active_offset_before = app.current_scrollback_offset();
        app.mouse_pixel_position = Some(PhysicalPosition::new(f64::from(CELL_WIDTH), 0.0));
        app.set_config_overrides(native_config_snapshot! {
            mouse_wheel_scrolls_tabs: Some(true),
            ..NativeConfigSnapshot::default()
        });

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert_ne!(app.active_tab_id(), tab_before);
        assert_eq!(
            app.app_shell.active_workspace().tabs().len(),
            tabs_len_before
        );
        assert_eq!(app.app_shell.pane_ids().len(), pane_count_before);
        assert_eq!(app.current_scrollback_offset(), active_offset_before);
    }

    #[test]
    fn window_app_wheel_separator_and_no_hit_return_false_without_state_change() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let shell_before = app.app_shell.clone();
        let active_snapshot_before = app.snapshot.clone();
        let inactive_snapshot_before = app
            .pane_snapshot(rssh_core::PaneId::new(1))
            .unwrap()
            .clone();
        let separator = app.pane_render_layout().separators[0];
        app.mouse_pixel_position = Some(PhysicalPosition::new(
            f64::from(app.frame_content_pixel_left())
                + f64::from(separator.column) * f64::from(CELL_WIDTH),
            f64::from(app.terminal_pixel_top())
                + f64::from(separator.row - app.terminal_frame_row_offset())
                    * f64::from(CELL_HEIGHT),
        ));
        assert!(
            !app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );
        app.mouse_pixel_position = None;
        assert!(
            !app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 0.0))
                .unwrap()
        );
        move_wheel_to_pane_cell_for_test(&mut app, rssh_core::PaneId::new(1), 0, 0, 1.0, 1.0);
        assert!(
            !app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(1.0, 0.0))
                .unwrap()
        );

        assert_eq!(app.app_shell, shell_before);
        assert_eq!(app.snapshot, active_snapshot_before);
        assert_eq!(
            app.pane_snapshot(rssh_core::PaneId::new(1)).unwrap(),
            &inactive_snapshot_before
        );
    }

    #[test]
    fn window_app_wheel_zoomed_layout_routes_only_visible_pane() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let hidden = rssh_core::PaneId::new(1);
        let visible = rssh_core::PaneId::new(2);
        app.handle_pty_output(b"right-0\r\nright-1\r\nright-2\r\nright-live")
            .unwrap();
        let hidden_viewport_before = app.pane_ui_ref(hidden).unwrap().stable_viewport;
        app.dispatch_app_action(AppAction::SetPaneZoomState {
            pane: visible,
            zoomed: true,
        })
        .unwrap();
        move_wheel_to_pane_cell_for_test(&mut app, visible, 0, 0, 1.0, 1.0);

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert_eq!(app.active_pane_id(), visible);
        assert!(app.current_scrollback_offset() > 0);
        assert_eq!(
            app.pane_ui_ref(hidden).unwrap().stable_viewport,
            hidden_viewport_before
        );
    }

    fn bind_wheel_command_for_test(app: &mut NativeWindowApp, command: WindowCommand) {
        app.mouse_assignments = vec![NativeUserMouseAssignment {
            event: NativeMouseAssignmentEvent {
                kind: NativeMouseAssignmentEventKind::Down,
                button: NativeMouseAssignmentButton::WheelUp,
                streak: 1,
            },
            modifiers: ModifiersState::empty(),
            mouse_reporting: false,
            alt_screen: NativeMouseAssignmentAltScreen::Any,
            command,
        }];
    }

    fn run_wheel_command_on_pane_for_test(
        app: &mut NativeWindowApp,
        pane_id: rssh_core::PaneId,
        command: WindowCommand,
    ) -> io::Result<bool> {
        bind_wheel_command_for_test(app, command);
        move_wheel_to_pane_cell_for_test(app, pane_id, 0, 1, 1.0, 1.0);
        app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 2.0))
    }

    #[test]
    fn wheel_action_io_error_includes_stable_command_and_app_error_context() {
        let error = super::wheel_action_io_error(
            &WindowCommand::CloseWorkspace,
            AppShellError::CannotCloseLastWorkspace,
        );

        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(
            error.to_string(),
            "wheel action 'Close Workspace' failed: CannotCloseLastWorkspace"
        );
    }

    #[test]
    fn window_app_wheel_binding_viewport_actions_use_hovered_pane() {
        let commands = [
            WindowCommand::ScrollByLine(-1),
            WindowCommand::ScrollByPage(WindowScrollByPageAmount::from_per_mille(-1_000)),
            WindowCommand::ScrollToTop,
            WindowCommand::ScrollToBottom,
            WindowCommand::ScrollToPrompt(-1),
        ];
        for command in commands {
            let command_label = format!("{command:?}");
            let mut app = wheel_split_with_inactive_history_for_test();
            let inactive = rssh_core::PaneId::new(1);
            app.handle_pane_pty_output(
                inactive,
                b"\r\n\x1b]133;A\x07prompt-one\r\nout\r\n\x1b]133;A\x07prompt-two\r\nlive",
            )
            .unwrap();
            if command == WindowCommand::ScrollToBottom {
                assert!(app.set_pane_scrollback_offset(inactive, 1));
            }
            let before = app
                .pane_ui_ref(inactive)
                .unwrap()
                .stable_viewport
                .scrollback_offset(app.pane_runtime_ref(inactive).unwrap().terminal());
            assert!(run_wheel_command_on_pane_for_test(&mut app, inactive, command).unwrap());
            let after = app
                .pane_ui_ref(inactive)
                .unwrap()
                .stable_viewport
                .scrollback_offset(app.pane_runtime_ref(inactive).unwrap().terminal());
            assert_ne!(after, before, "{command_label}");
            assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
            assert_eq!(app.current_scrollback_offset(), 0);
        }
    }

    #[test]
    fn window_app_wheel_binding_current_delta_uses_hovered_pane_and_current_event() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::ScrollByCurrentEventWheelDelta,
            )
            .unwrap()
        );
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
        assert_eq!(app.current_scrollback_offset(), 0);
        assert_eq!(
            app.pane_ui_ref(inactive)
                .unwrap()
                .stable_viewport
                .scrollback_offset(app.pane_runtime_ref(inactive).unwrap().terminal()),
            2
        );
    }

    #[test]
    fn window_app_wheel_binding_multiple_recursively_retains_target() {
        let mut app = wheel_split_with_inactive_history_for_test();
        app.set_config_overrides(native_config_snapshot! {
            scroll_to_bottom_on_input: Some(false),
            ..NativeConfigSnapshot::default()
        });
        let inactive = rssh_core::PaneId::new(1);
        let written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::Multiple(vec![
                    WindowCommand::ScrollByLine(-1),
                    WindowCommand::SendString("target".to_owned()),
                ]),
            )
            .unwrap()
        );
        assert_eq!(written.lock().unwrap().as_slice(), b"target");
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
        assert_eq!(app.current_scrollback_offset(), 0);
        assert_eq!(
            app.pane_ui_ref(inactive)
                .unwrap()
                .stable_viewport
                .scrollback_offset(app.pane_runtime_ref(inactive).unwrap().terminal()),
            1
        );
    }

    #[test]
    fn window_app_wheel_binding_writer_actions_use_hovered_pane_modes_and_writer() {
        let active_written = Arc::new(Mutex::new(Vec::new()));
        let mut app = wheel_split_with_inactive_history_for_test();
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&active_written))));
        let inactive = rssh_core::PaneId::new(1);
        let target_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        app.clipboard_reader = Box::new(|| Some("clip".to_owned()));
        app.primary_selection_reader = Box::new(|| Some("primary".to_owned()));
        app.handle_pane_pty_output(inactive, b"\x1b[?2004h\x1b[?1h\x1b[=1u")
            .unwrap();
        let commands = [
            WindowCommand::SendString("raw".to_owned()),
            WindowCommand::SendPaste("paste".to_owned()),
            WindowCommand::SendKey(WindowSendKey {
                key: Key::Named(NamedKey::ArrowUp),
                modifiers: ModifiersState::empty(),
            }),
            WindowCommand::PasteFromClipboard,
            WindowCommand::PasteFromPrimarySelection,
        ];
        for command in commands {
            assert!(run_wheel_command_on_pane_for_test(&mut app, inactive, command).unwrap());
        }
        assert!(active_written.lock().unwrap().is_empty());
        let bytes = target_written.lock().unwrap().clone();
        assert!(bytes.starts_with(b"raw\x1b[200~paste\x1b[201~"));
        assert!(bytes.windows(4).any(|window| window == b"clip"));
        assert!(bytes.windows(7).any(|window| window == b"primary"));
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    #[test]
    fn window_app_wheel_binding_copy_overlay_actions_use_hovered_pane_ui() {
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&copied);
        let mut app = wheel_split_with_inactive_history_for_test();
        app.clipboard_writer = Box::new(move |text| {
            recorded.lock().unwrap().push(text.to_owned());
            true
        });
        let inactive = rssh_core::PaneId::new(1);
        let runtime = app.pane_runtimes.get_mut(&inactive).unwrap();
        let dimensions = runtime.runtime.terminal().stable_dimensions();
        runtime.ui.ordinary_selection = Some(StableOrdinarySelection::new(
            SelectionSourceCell {
                domain: dimensions.domain,
                row: dimensions.physical_top,
                column: 0,
            },
            SelectionSourceCell {
                domain: dimensions.domain,
                row: dimensions.physical_top,
                column: 3,
            },
            runtime.runtime.terminal().current_seqno(),
        ));
        assert!(
            run_wheel_command_on_pane_for_test(&mut app, inactive, WindowCommand::CopyToClipboard,)
                .unwrap()
        );
        assert_eq!(copied.lock().unwrap().as_slice(), ["left"]);
        assert!(
            run_wheel_command_on_pane_for_test(&mut app, inactive, WindowCommand::ClearSelection,)
                .unwrap()
        );
        assert!(
            app.pane_ui_ref(inactive)
                .unwrap()
                .ordinary_selection
                .is_none()
        );
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    #[test]
    fn window_app_wheel_binding_copy_search_quick_select_use_hovered_pane_ui() {
        let commands = [
            WindowCommand::EnterCopyMode,
            WindowCommand::Search(WindowSearchCommandQuery::Pattern {
                pattern: "left".to_owned(),
                match_type: WindowSearchMatchType::CaseSensitive,
            }),
            WindowCommand::QuickSelect(WindowQuickSelectOptions::default()),
        ];
        for command in commands {
            let mut app = wheel_split_with_inactive_history_for_test();
            let inactive = rssh_core::PaneId::new(1);
            let expected = command.clone();
            assert!(run_wheel_command_on_pane_for_test(&mut app, inactive, command).unwrap());
            let target_ui = app.pane_ui_ref(inactive).unwrap();
            match expected {
                WindowCommand::EnterCopyMode => assert!(target_ui.copy_mode().is_some()),
                WindowCommand::Search(_) => assert!(target_ui.retained_search().is_some()),
                WindowCommand::QuickSelect(_) => assert!(target_ui.quick_select().is_some()),
                _ => unreachable!(),
            }
            assert!(!app.active_ui.overlay_active());
            assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
        }
    }

    #[test]
    fn window_app_wheel_binding_search_and_quick_select_match_active_owner_semantics() {
        let commands = [
            WindowCommand::Search(WindowSearchCommandQuery::Pattern {
                pattern: "left".to_owned(),
                match_type: WindowSearchMatchType::CaseSensitive,
            }),
            WindowCommand::QuickSelect(WindowQuickSelectOptions {
                patterns: Some(vec!["left".to_owned()]),
                alphabet: Some("asdf".to_owned()),
                ..WindowQuickSelectOptions::default()
            }),
        ];
        for command in commands {
            let mut active = NativeWindowApp::new(None);
            active.runtime.resize(rssh_core::TerminalSize::new(20, 2));
            active
                .handle_pty_output(b"left-0\r\nleft-1\r\nleft-2\r\nleft-live")
                .unwrap();
            let active_pane = active.active_pane_id();
            assert!(
                run_wheel_command_on_pane_for_test(&mut active, active_pane, command.clone())
                    .unwrap()
            );

            let mut inactive = wheel_split_with_inactive_history_for_test();
            let inactive_pane = rssh_core::PaneId::new(1);
            assert!(
                run_wheel_command_on_pane_for_test(&mut inactive, inactive_pane, command.clone())
                    .unwrap()
            );
            let target_ui = inactive.pane_ui_ref(inactive_pane).unwrap();
            match command {
                WindowCommand::Search(_) => {
                    assert_eq!(
                        target_ui.retained_search(),
                        active.active_ui.retained_search()
                    );
                }
                WindowCommand::QuickSelect(_) => {
                    let target = target_ui.quick_select().unwrap();
                    let active = active.active_ui.quick_select().unwrap();
                    assert_eq!(target.matches, active.matches);
                    assert_eq!(target.labels, active.labels);
                }
                _ => unreachable!(),
            }
        }
    }

    fn wheel_copy_owner_fixture_for_test(
        inactive_target: bool,
    ) -> (NativeWindowApp, rssh_core::PaneId) {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rssh_core::TerminalSize::new(24, 4));
        app.handle_pty_output(
            b"\x1b]133;A\x07> hit zero\r\noutput zero\r\n\x1b]133;A\x07> hit one\r\noutput one\r\n\x1b]133;A\x07> hit two\r\noutput two\r\n\x1b]133;A\x07> hit three\r\noutput three\r\n\x1b]133;A\x07> hit four\r\nlive",
        )
        .unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        if !inactive_target {
            app.dispatch_app_action(AppAction::ActivatePane {
                pane: rssh_core::PaneId::new(1),
            })
            .unwrap();
        }
        (app, rssh_core::PaneId::new(1))
    }

    fn all_copy_mode_assignments_for_wheel_test() -> Vec<super::WindowCopyModeAssignment> {
        use super::WindowCopyModeAssignment as A;
        vec![
            A::AcceptPattern,
            A::Close,
            A::ClearPattern,
            A::ClearSelectionMode,
            A::CycleMatchType,
            A::EditPattern,
            A::JumpAgain,
            A::JumpReverse,
            A::StartJump {
                forward: true,
                prev_char: false,
            },
            A::MoveBackwardSemanticZone,
            A::MoveSemanticZoneOfType {
                delta: -1,
                semantic_type: rssh_terminal::SemanticType::Prompt,
            },
            A::MoveBackwardWord,
            A::MoveDown,
            A::MoveForwardSemanticZone,
            A::MoveForwardWord,
            A::MoveForwardWordEnd,
            A::MoveLeft,
            A::MoveRight,
            A::MoveToEndOfLineContent,
            A::MoveToScrollbackBottom,
            A::MoveToScrollbackTop,
            A::MoveToSelectionOtherEnd,
            A::MoveToSelectionOtherEndHoriz,
            A::MoveToStartOfLine,
            A::MoveToStartOfLineContent,
            A::MoveToStartOfNextLine,
            A::MoveToViewportBottom,
            A::MoveToViewportMiddle,
            A::MoveToViewportTop,
            A::MoveUp,
            A::MoveByPage(WindowScrollByPageAmount::from_per_mille(-1_000)),
            A::PageDown,
            A::PageUp,
            A::NextMatch,
            A::NextMatchPage,
            A::PriorMatch,
            A::PriorMatchPage,
            A::SetSelectionMode(super::WindowCopySelectionMode::None),
            A::SetSelectionMode(super::WindowCopySelectionMode::Cell),
            A::SetSelectionMode(super::WindowCopySelectionMode::Word),
            A::SetSelectionMode(super::WindowCopySelectionMode::Line),
            A::SetSelectionMode(super::WindowCopySelectionMode::Block),
            A::SetSelectionMode(super::WindowCopySelectionMode::SemanticZone),
        ]
    }

    fn wheel_copy_assignment_sequence_for_test(
        assignment: super::WindowCopyModeAssignment,
    ) -> WindowCommand {
        use super::WindowCopyModeAssignment as A;
        let mut commands = if matches!(
            assignment,
            A::AcceptPattern
                | A::ClearPattern
                | A::CycleMatchType
                | A::EditPattern
                | A::NextMatch
                | A::NextMatchPage
                | A::PriorMatch
                | A::PriorMatchPage
        ) {
            vec![WindowCommand::Search(WindowSearchCommandQuery::Pattern {
                pattern: "hit".to_owned(),
                match_type: WindowSearchMatchType::CaseSensitive,
            })]
        } else {
            vec![WindowCommand::EnterCopyMode]
        };
        if matches!(
            assignment,
            A::MoveToSelectionOtherEnd | A::MoveToSelectionOtherEndHoriz
        ) {
            commands.extend([
                WindowCommand::CopyMode(A::SetSelectionMode(super::WindowCopySelectionMode::Block)),
                WindowCommand::CopyMode(A::MoveRight),
            ]);
        }
        commands.push(WindowCommand::CopyMode(assignment));
        WindowCommand::Multiple(commands)
    }

    #[test]
    fn window_app_wheel_binding_all_copy_mode_assignments_match_active_owner() {
        for assignment in all_copy_mode_assignments_for_wheel_test() {
            let (mut active, active_pane) = wheel_copy_owner_fixture_for_test(false);
            assert!(
                run_wheel_command_on_pane_for_test(
                    &mut active,
                    active_pane,
                    wheel_copy_assignment_sequence_for_test(assignment),
                )
                .unwrap()
            );
            let (mut inactive, inactive_pane) = wheel_copy_owner_fixture_for_test(true);
            assert!(
                run_wheel_command_on_pane_for_test(
                    &mut inactive,
                    inactive_pane,
                    wheel_copy_assignment_sequence_for_test(assignment),
                )
                .unwrap()
            );

            let active_ui = &active.active_ui;
            let target_ui = inactive.pane_ui_ref(inactive_pane).unwrap();
            assert_eq!(
                target_ui.copy_search_mode(),
                active_ui.copy_search_mode(),
                "{assignment:?}"
            );
            assert_eq!(
                target_ui.retained_search(),
                active_ui.retained_search(),
                "{assignment:?}"
            );
            match (target_ui.copy_mode(), active_ui.copy_mode()) {
                (Some(target), Some(active)) => {
                    assert_eq!(target.cursor, active.cursor, "{assignment:?}");
                    assert_eq!(target.source_cursor, active.source_cursor, "{assignment:?}");
                    assert_eq!(target.pending_jump, active.pending_jump, "{assignment:?}");
                    assert_eq!(target.last_jump, active.last_jump, "{assignment:?}");
                    assert_eq!(
                        target.search_direction, active.search_direction,
                        "{assignment:?}"
                    );
                    assert_eq!(
                        target.selection_mode, active.selection_mode,
                        "{assignment:?}"
                    );
                    assert_eq!(target.anchor, active.anchor, "{assignment:?}");
                    assert_eq!(target.source_anchor, active.source_anchor, "{assignment:?}");
                }
                (None, None) => {}
                state => panic!("copy mode owner mismatch for {assignment:?}: {state:?}"),
            }
            assert_eq!(
                target_ui.stable_viewport.scrollback_offset(
                    inactive.pane_runtime_ref(inactive_pane).unwrap().terminal(),
                ),
                active.current_scrollback_offset(),
                "{assignment:?}",
            );
            assert_eq!(inactive.active_pane_id(), rssh_core::PaneId::new(2));
            assert!(!inactive.active_ui.overlay_active());
        }
    }

    #[test]
    fn window_app_wheel_copy_mode_core_never_installs_inactive_owner_as_active() {
        let (mut app, inactive) = wheel_copy_owner_fixture_for_test(true);
        app.enter_search_mode_with_query(&WindowSearchCommandQuery::Pattern {
            pattern: "active-only".to_owned(),
            match_type: WindowSearchMatchType::CaseSensitive,
        });
        app.selecting = true;
        let active_pane = app.active_pane_id();
        let active_snapshot = app.snapshot.clone();
        let active_search = app.active_ui.retained_search().cloned();
        let active_copy = format!("{:?}", app.active_ui.retained_copy_mode());
        let active_selection = app.selection;
        let active_viewport = app.active_ui.stable_viewport;

        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                wheel_copy_assignment_sequence_for_test(
                    super::WindowCopyModeAssignment::NextMatchPage,
                ),
            )
            .unwrap()
        );
        assert_eq!(app.active_pane_id(), active_pane);
        assert_eq!(app.snapshot, active_snapshot);
        assert_eq!(app.active_ui.retained_search(), active_search.as_ref());
        assert_eq!(
            format!("{:?}", app.active_ui.retained_copy_mode()),
            active_copy
        );
        assert_eq!(app.selection, active_selection);
        assert_eq!(app.active_ui.stable_viewport, active_viewport);
        assert!(app.selecting);

        let error = app
            .apply_wheel_copy_mode_assignment(
                rssh_core::PaneId::new(999),
                super::WindowCopyModeAssignment::MoveDown,
            )
            .expect_err("invalid owner must fail before any mutation");
        assert_eq!(
            error,
            AppShellError::InvalidPane(rssh_core::PaneId::new(999))
        );
        assert_eq!(app.active_pane_id(), active_pane);
        assert_eq!(app.snapshot, active_snapshot);
        assert_eq!(app.active_ui.retained_search(), active_search.as_ref());
        assert_eq!(
            format!("{:?}", app.active_ui.retained_copy_mode()),
            active_copy
        );
        assert_eq!(app.selection, active_selection);
        assert_eq!(app.active_ui.stable_viewport, active_viewport);
        assert!(app.selecting);
    }

    #[test]
    fn window_app_wheel_copy_mode_noop_consumes_without_refreshing_owner() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        app.enter_wheel_target_copy_mode(inactive);
        app.frame_needs_full_repaint = false;
        app.pending_frame_damage.clear();
        let snapshot = app.pane_snapshot(inactive).unwrap().clone();
        let rebuilds = app.metrics_snapshot().snapshot_rebuilds;

        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::CopyMode(super::WindowCopyModeAssignment::NextMatch),
            )
            .unwrap()
        );
        assert_eq!(app.pane_snapshot(inactive).unwrap(), &snapshot);
        assert_eq!(app.metrics_snapshot().snapshot_rebuilds, rebuilds);
        assert!(!app.frame_needs_full_repaint);
        assert!(app.pending_frame_damage.is_empty());
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    fn activate_third_pane_with_writer_for_wheel_deferred_test(
        app: &mut NativeWindowApp,
    ) -> Arc<Mutex<Vec<u8>>> {
        let source = app.active_pane_id();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: source,
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        let written = Arc::new(Mutex::new(Vec::new()));
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        written
    }

    #[test]
    fn window_app_wheel_emit_event_and_open_uri_use_hovered_pane_context() {
        let emitted = Arc::new(Mutex::new(Vec::new()));
        let emitted_events = Arc::clone(&emitted);
        let opened = Arc::new(Mutex::new(Vec::new()));
        let opened_events = Arc::clone(&opened);
        let mut app = wheel_split_with_inactive_history_for_test();
        app.emit_event_handler = Box::new(move |event| {
            emitted_events.lock().unwrap().push(event.pane);
            true
        });
        app.open_uri_handler = Box::new(move |event| {
            opened_events.lock().unwrap().push(event.pane);
            false
        });
        let inactive = rssh_core::PaneId::new(1);

        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::EmitEvent(WindowEmitEvent {
                    name: "wheel-target".to_owned()
                }),
            )
            .unwrap()
        );
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::OpenUri("https://inactive.test".to_owned()),
            )
            .unwrap()
        );
        assert_eq!(emitted.lock().unwrap().as_slice(), [inactive]);
        assert_eq!(opened.lock().unwrap().as_slice(), [inactive]);
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    #[test]
    fn window_app_wheel_char_select_completion_retains_hovered_pane_context() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let target_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::CharSelectArgs(WindowCharSelectOptions {
                    copy_on_select: false,
                    ..WindowCharSelectOptions::default()
                }),
            )
            .unwrap()
        );
        let active_written = activate_third_pane_with_writer_for_wheel_deferred_test(&mut app);

        assert!(app.handle_char_select_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()));
        assert!(!target_written.lock().unwrap().is_empty());
        assert!(active_written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_wheel_prompt_input_completion_retains_hovered_pane_context() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let target_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
                    action: Some(WindowPromptInputLineAction::SendLineText),
                    ..WindowPromptInputLineOptions::default()
                }),
            )
            .unwrap()
        );
        let active_written = activate_third_pane_with_writer_for_wheel_deferred_test(&mut app);

        app.submit_prompt_input_line(Some("prompt-target".to_owned()));
        assert_eq!(target_written.lock().unwrap().as_slice(), b"prompt-target");
        assert!(active_written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_wheel_input_selector_completion_retains_hovered_pane_context() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let target_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        let choice = WindowInputSelectorChoice {
            label: "Target".to_owned(),
            id: Some("selector-target".to_owned()),
        };
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::InputSelector(WindowInputSelectorOptions {
                    choices: vec![choice.clone()],
                    action: Some(WindowInputSelectorAction::SendIdText),
                    ..WindowInputSelectorOptions::default()
                }),
            )
            .unwrap()
        );
        let active_written = activate_third_pane_with_writer_for_wheel_deferred_test(&mut app);

        app.submit_input_selector(Some(choice));
        assert_eq!(
            target_written.lock().unwrap().as_slice(),
            b"selector-target"
        );
        assert!(active_written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_wheel_confirmation_completion_retains_hovered_pane_context() {
        let confirmed = Arc::new(Mutex::new(Vec::new()));
        let confirmation_events = Arc::clone(&confirmed);
        let mut app = wheel_split_with_inactive_history_for_test();
        app.confirmation_handler = Box::new(move |event| {
            confirmation_events.lock().unwrap().push(event.pane);
            true
        });
        let inactive = rssh_core::PaneId::new(1);
        let target_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::Confirmation(WindowConfirmationOptions {
                    message: "Confirm".to_owned(),
                    action: Box::new(WindowCommand::SendString("confirm-target".to_owned())),
                    cancel: None,
                }),
            )
            .unwrap()
        );
        let active_written = activate_third_pane_with_writer_for_wheel_deferred_test(&mut app);

        app.submit_confirmation(true);
        assert_eq!(confirmed.lock().unwrap().as_slice(), [inactive]);
        assert_eq!(target_written.lock().unwrap().as_slice(), b"confirm-target");
        assert!(active_written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_wheel_palette_and_launcher_completion_retain_full_target_context() {
        for opener in [
            WindowCommand::ActivateCommandPalette,
            WindowCommand::ShowLauncher,
            WindowCommand::ShowLauncherArgs(WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::commands(),
                title: None,
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            }),
        ] {
            let mut app = wheel_split_with_inactive_history_for_test();
            let inactive = rssh_core::PaneId::new(1);
            let target_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
            move_wheel_to_pane_cell_for_test(&mut app, inactive, 1, 2, 1.0, 1.0);
            bind_wheel_command_for_test(&mut app, opener);
            assert!(
                app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                    .unwrap()
            );
            let active_written = activate_third_pane_with_writer_for_wheel_deferred_test(&mut app);

            assert!(
                app.command_palette_execute(
                    WindowCommand::SendString("palette-target".to_owned(),)
                )
            );
            assert_eq!(target_written.lock().unwrap().as_slice(), b"palette-target");
            assert!(active_written.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn window_app_wheel_palette_nested_ui_keeps_target_after_old_ui_exits() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let target_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::ActivateCommandPalette,
            )
            .unwrap()
        );
        assert!(app.command_palette_execute(WindowCommand::Confirmation(
            WindowConfirmationOptions {
                message: "Nested".to_owned(),
                action: Box::new(WindowCommand::SendString("nested-target".to_owned())),
                cancel: None,
            },
        )));
        let active_written = activate_third_pane_with_writer_for_wheel_deferred_test(&mut app);

        app.submit_confirmation(true);
        assert_eq!(target_written.lock().unwrap().as_slice(), b"nested-target");
        assert!(active_written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_wheel_palette_failure_retains_stale_target_without_active_fallback() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::ActivateCommandPalette,
            )
            .unwrap()
        );
        app.dispatch_app_action(AppAction::ClosePane { pane: inactive })
            .unwrap();
        let active_written = Arc::new(Mutex::new(Vec::new()));
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&active_written))));

        assert!(
            !app.command_palette_execute(WindowCommand::SendString("first-attempt".to_owned(),))
        );
        assert_eq!(app.deferred_wheel_context.unwrap().pane_id, inactive);
        assert!(
            !app.command_palette_execute(WindowCommand::SendString("second-attempt".to_owned(),))
        );
        assert!(active_written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_wheel_palette_query_resolves_before_target_dispatch() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        app.dispatch_app_action(AppAction::SetPaneCurrentWorkingDir {
            pane: inactive,
            cwd: Some("C:/hovered-palette".to_owned()),
        })
        .unwrap();
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::ActivateCommandPalette,
            )
            .unwrap()
        );
        app.command_palette_set_query("switch workspace wheel-ops".to_owned());

        assert!(app.handle_command_palette_logical_key(
            &Key::Named(NamedKey::Enter),
            ModifiersState::empty(),
        ));
        assert_eq!(app.app_shell.active_workspace().name(), "wheel-ops");
        assert_eq!(
            app.app_shell.active_pane().launch().cwd(),
            Some("C:/hovered-palette")
        );
    }

    #[test]
    fn window_app_wheel_palette_search_query_uses_hovered_owner() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::ActivateCommandPalette,
            )
            .unwrap()
        );
        app.command_palette_set_query("search left-1".to_owned());

        assert!(app.handle_command_palette_logical_key(
            &Key::Named(NamedKey::Enter),
            ModifiersState::empty(),
        ));
        assert!(
            app.pane_ui_ref(inactive)
                .unwrap()
                .retained_search()
                .is_some()
        );
        assert!(app.active_ui.retained_search().is_none());
    }

    #[test]
    fn window_app_wheel_palette_pane_select_query_keeps_hovered_swap_source() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::ActivateCommandPalette,
            )
            .unwrap()
        );
        app.command_palette_set_query("pane select swap alphabet 12".to_owned());

        assert!(app.handle_command_palette_logical_key(
            &Key::Named(NamedKey::Enter),
            ModifiersState::empty(),
        ));
        let pane_select = app.pane_select.as_ref().expect("pane select should open");
        assert_eq!(pane_select.mode, WindowPaneSelectMode::SwapWithActive);
        assert_eq!(pane_select.labels[0].label, "1");
        assert_eq!(pane_select.labels[1].label, "2");
        assert_eq!(app.deferred_wheel_context.unwrap().pane_id, inactive);
    }

    #[test]
    fn window_app_wheel_palette_spawn_and_split_queries_use_hovered_reference() {
        for (query, command) in [
            ("new tab --env WHEEL_OWNER=hovered", WindowCommand::NewTab),
            (
                "split horizontal --env WHEEL_OWNER=hovered",
                WindowCommand::SplitRight,
            ),
        ] {
            let mut app = wheel_split_with_inactive_history_for_test();
            let inactive = rssh_core::PaneId::new(1);
            app.dispatch_app_action(AppAction::SetPaneCurrentWorkingDir {
                pane: inactive,
                cwd: Some("C:/hovered-query".to_owned()),
            })
            .unwrap();
            assert!(
                run_wheel_command_on_pane_for_test(
                    &mut app,
                    inactive,
                    WindowCommand::ActivateCommandPalette,
                )
                .unwrap()
            );
            app.command_palette_set_query(query.to_owned());
            assert_eq!(app.command_palette_filtered_commands(), [command]);

            assert!(app.handle_command_palette_logical_key(
                &Key::Named(NamedKey::Enter),
                ModifiersState::empty(),
            ));
            let launch = app.app_shell.active_pane().launch();
            assert_eq!(launch.cwd(), Some("C:/hovered-query"));
            assert_eq!(
                launch.environment().get("WHEEL_OWNER").map(String::as_str),
                Some("hovered")
            );
        }
    }

    #[test]
    fn window_app_wheel_emit_event_nested_mouse_action_keeps_exact_cell() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        app.lua_emit_event_handlers.insert(
            "select-wheel-cell".to_owned(),
            vec![super::NativeLuaEmitEventHandler {
                command: Some(WindowCommand::SelectTextAtMouseCursorCell),
                stop_propagation: false,
            }],
        );
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 1, 2, 1.0, 1.0);
        bind_wheel_command_for_test(
            &mut app,
            WindowCommand::EmitEvent(WindowEmitEvent {
                name: "select-wheel-cell".to_owned(),
            }),
        );

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );
        let selection = app
            .pane_ui_ref(inactive)
            .unwrap()
            .ordinary_selection
            .expect("nested action should select the hovered cell");
        assert_eq!(selection.anchor.column, 2);
        assert_eq!(selection.focus.column, 2);
        assert!(app.active_ui.ordinary_selection.is_none());
    }

    #[test]
    fn ordinary_emit_event_without_mouse_does_not_fabricate_nested_mouse_target() {
        let mut app = NativeWindowApp::new(None);
        app.lua_emit_event_handlers.insert(
            "ordinary-no-mouse".to_owned(),
            vec![super::NativeLuaEmitEventHandler {
                command: Some(WindowCommand::SelectTextAtMouseCursorCell),
                stop_propagation: false,
            }],
        );
        assert!(app.mouse_pixel_position.is_none());

        app.emit_event(WindowEmitEvent {
            name: "ordinary-no-mouse".to_owned(),
        });

        assert!(app.active_ui.ordinary_selection.is_none());
    }

    #[test]
    fn ordinary_open_uri_nested_command_uses_legacy_dispatch_without_mouse_target() {
        let mut app = NativeWindowApp::new(None);
        app.lua_open_uri = Some(super::NativeLuaOpenUri::UriPrefix {
            prefix: "ssh://".to_owned(),
            allow_default: false,
            action: Some(super::NativeLuaOpenUriAction::SpawnCommandInNewWindow {
                args: vec![super::NativeLuaOpenUriArg::Static("ssh".to_owned())],
            }),
        });
        let label = WindowCommand::SpawnCommandInNewWindow(WindowSpawnCommandQuery {
            label: None,
            program: "ssh".to_owned(),
            args: Vec::new(),
            cwd: None,
            environment: BTreeMap::new(),
            domain: None,
            window_position: None,
        })
        .label();
        assert_eq!(app.command_palette_frecency(label).uses, 0);
        assert!(app.mouse_pixel_position.is_none());

        assert!(app.open_uri("ssh://host"));

        assert_eq!(app.command_palette_frecency(label).uses, 1);
        assert!(app.active_ui.ordinary_selection.is_none());
    }

    #[test]
    fn ordinary_deferred_ui_completion_uses_completion_time_active_pane() {
        let completion_pane = rssh_core::PaneId::new(1);

        let mut char_app = wheel_split_with_inactive_history_for_test();
        char_app.enter_char_select_mode_with_options(WindowCharSelectOptions {
            copy_on_select: false,
            ..WindowCharSelectOptions::default()
        });
        assert!(char_app.deferred_wheel_context.is_none());
        char_app
            .dispatch_app_action(AppAction::ActivatePane {
                pane: completion_pane,
            })
            .unwrap();
        let char_written = Arc::new(Mutex::new(Vec::new()));
        char_app.writer = Some(Box::new(SharedWriter(Arc::clone(&char_written))));
        assert!(
            char_app.handle_char_select_key(&Key::Named(NamedKey::Enter), ModifiersState::empty())
        );
        assert!(!char_written.lock().unwrap().is_empty());

        let mut prompt_app = wheel_split_with_inactive_history_for_test();
        prompt_app.enter_prompt_input_line_mode(WindowPromptInputLineOptions {
            action: Some(WindowPromptInputLineAction::SendLineText),
            ..WindowPromptInputLineOptions::default()
        });
        assert!(prompt_app.deferred_wheel_context.is_none());
        prompt_app
            .dispatch_app_action(AppAction::ActivatePane {
                pane: completion_pane,
            })
            .unwrap();
        let prompt_written = Arc::new(Mutex::new(Vec::new()));
        prompt_app.writer = Some(Box::new(SharedWriter(Arc::clone(&prompt_written))));
        prompt_app.submit_prompt_input_line(Some("prompt-completion".to_owned()));
        assert_eq!(
            prompt_written.lock().unwrap().as_slice(),
            b"prompt-completion"
        );

        let mut input_app = wheel_split_with_inactive_history_for_test();
        let choice = WindowInputSelectorChoice {
            label: "Completion".to_owned(),
            id: Some("input-completion".to_owned()),
        };
        input_app.enter_input_selector_mode(WindowInputSelectorOptions {
            choices: vec![choice.clone()],
            action: Some(WindowInputSelectorAction::SendIdText),
            ..WindowInputSelectorOptions::default()
        });
        assert!(input_app.deferred_wheel_context.is_none());
        input_app
            .dispatch_app_action(AppAction::ActivatePane {
                pane: completion_pane,
            })
            .unwrap();
        let input_written = Arc::new(Mutex::new(Vec::new()));
        input_app.writer = Some(Box::new(SharedWriter(Arc::clone(&input_written))));
        input_app.submit_input_selector(Some(choice));
        assert_eq!(
            input_written.lock().unwrap().as_slice(),
            b"input-completion"
        );

        let confirmation_panes = Arc::new(Mutex::new(Vec::new()));
        let recorded_panes = Arc::clone(&confirmation_panes);
        let mut confirmation_app = wheel_split_with_inactive_history_for_test();
        confirmation_app.confirmation_handler = Box::new(move |event| {
            recorded_panes.lock().unwrap().push(event.pane);
            true
        });
        confirmation_app.enter_confirmation_mode(WindowConfirmationOptions {
            message: "Completion".to_owned(),
            action: Box::new(WindowCommand::SendString(
                "confirmation-completion".to_owned(),
            )),
            cancel: None,
        });
        assert!(confirmation_app.deferred_wheel_context.is_none());
        confirmation_app
            .dispatch_app_action(AppAction::ActivatePane {
                pane: completion_pane,
            })
            .unwrap();
        let confirmation_written = Arc::new(Mutex::new(Vec::new()));
        confirmation_app.writer = Some(Box::new(SharedWriter(Arc::clone(&confirmation_written))));
        confirmation_app.submit_confirmation(true);
        assert_eq!(
            confirmation_panes.lock().unwrap().as_slice(),
            [completion_pane]
        );
        assert_eq!(
            confirmation_written.lock().unwrap().as_slice(),
            b"confirmation-completion"
        );
    }

    #[test]
    fn window_app_wheel_pane_swap_uses_hovered_pane_as_source_after_focus_changes() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        assert!(
            run_wheel_command_on_pane_for_test(&mut app, inactive, WindowCommand::EnterPaneSwap,)
                .unwrap()
        );
        let before = app.pane_render_layout();
        let pane_one_column = before
            .panes
            .iter()
            .find(|rect| rect.pane_id == inactive)
            .unwrap()
            .column;
        let pane_two_column = before
            .panes
            .iter()
            .find(|rect| rect.pane_id == rssh_core::PaneId::new(2))
            .unwrap()
            .column;
        activate_third_pane_with_writer_for_wheel_deferred_test(&mut app);

        assert!(app.handle_pane_select_key(&Key::Character("s".into()), ModifiersState::empty()));
        let after = app.pane_render_layout();
        assert_eq!(
            after
                .panes
                .iter()
                .find(|rect| rect.pane_id == inactive)
                .unwrap()
                .column,
            pane_two_column
        );
        assert_eq!(
            after
                .panes
                .iter()
                .find(|rect| rect.pane_id == rssh_core::PaneId::new(2))
                .unwrap()
                .column,
            pane_one_column
        );
    }

    #[test]
    fn wheel_pane_select_classification_is_variant_sensitive() {
        let options = |mode| {
            WindowCommand::PaneSelect(WindowPaneSelectOptions {
                mode,
                show_pane_ids: false,
                alphabet: None,
            })
        };
        assert_eq!(
            options(WindowPaneSelectMode::SwapWithActive).wheel_command_class(),
            super::WheelCommandClass::ContextualUi
        );
        assert_eq!(
            options(WindowPaneSelectMode::SwapWithActiveKeepFocus).wheel_command_class(),
            super::WheelCommandClass::ContextualUi
        );
        for mode in [
            WindowPaneSelectMode::Activate,
            WindowPaneSelectMode::MoveToNewTab,
            WindowPaneSelectMode::MoveToNewWindow,
        ] {
            assert_eq!(
                options(mode).wheel_command_class(),
                super::WheelCommandClass::Global
            );
        }
    }

    #[test]
    fn window_app_wheel_workspace_creation_inherits_hovered_pane_cwd() {
        let commands = [
            WindowCommand::NewWorkspace,
            WindowCommand::SwitchToWorkspace,
            WindowCommand::SwitchToWorkspaceArgs(WindowSwitchToWorkspaceOptions {
                name: Some("wheel-args".to_owned()),
                command: None,
                command_options: None,
            }),
            WindowCommand::SwitchToWorkspaceName("wheel-name".to_owned()),
        ];
        for command in commands {
            let mut app = wheel_split_with_inactive_history_for_test();
            let inactive = rssh_core::PaneId::new(1);
            app.dispatch_app_action(AppAction::SetPaneCurrentWorkingDir {
                pane: inactive,
                cwd: Some("C:/hovered-workspace".to_owned()),
            })
            .unwrap();
            app.dispatch_app_action(AppAction::SetPaneCurrentWorkingDir {
                pane: rssh_core::PaneId::new(2),
                cwd: Some("C:/active-workspace".to_owned()),
            })
            .unwrap();

            assert!(run_wheel_command_on_pane_for_test(&mut app, inactive, command).unwrap());
            assert_eq!(
                app.app_shell.active_pane().launch().cwd(),
                Some("C:/hovered-workspace")
            );
        }
    }

    #[test]
    fn wheel_deferred_target_disappearance_returns_stable_error_without_active_fallback() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 1, 2, 1.0, 1.0);
        let super::WheelHitTarget::PaneSurface(target) =
            app.wheel_hit_target_at_mouse_position().unwrap()
        else {
            panic!("test pointer must resolve to the pane surface");
        };
        app.dispatch_app_action(AppAction::ClosePane { pane: inactive })
            .unwrap();

        let error = app
            .apply_command_for_target_context(
                target,
                WindowCommand::SendString("must-not-fallback".to_owned()),
            )
            .expect_err("removed wheel target must fail");
        assert_eq!(
            error.to_string(),
            "wheel action 'Send String' failed: InvalidPane(PaneId(1))"
        );
    }

    #[test]
    fn window_app_wheel_binding_copy_mode_page_matches_use_viewport_page_semantics() {
        for (page, step) in [
            (
                super::WindowCopyModeAssignment::NextMatchPage,
                super::WindowCopyModeAssignment::NextMatch,
            ),
            (
                super::WindowCopyModeAssignment::PriorMatchPage,
                super::WindowCopyModeAssignment::PriorMatch,
            ),
        ] {
            let sequence = |assignment| {
                WindowCommand::Multiple(vec![
                    WindowCommand::Search(WindowSearchCommandQuery::Pattern {
                        pattern: "hit".to_owned(),
                        match_type: WindowSearchMatchType::CaseSensitive,
                    }),
                    WindowCommand::ScrollToTop,
                    WindowCommand::CopyMode(assignment),
                ])
            };
            let (mut page_app, target) = wheel_copy_owner_fixture_for_test(true);
            run_wheel_command_on_pane_for_test(&mut page_app, target, sequence(page)).unwrap();
            let page_row = page_app
                .pane_ui_ref(target)
                .unwrap()
                .retained_search()
                .and_then(|search| search.current)
                .map(|matched| matched.source_row);

            let (mut step_app, target) = wheel_copy_owner_fixture_for_test(true);
            run_wheel_command_on_pane_for_test(&mut step_app, target, sequence(step)).unwrap();
            let step_row = step_app
                .pane_ui_ref(target)
                .unwrap()
                .retained_search()
                .and_then(|search| search.current)
                .map(|matched| matched.source_row);
            assert_ne!(page_row, step_row, "{page:?} must move by a viewport page");
        }
    }

    #[test]
    fn window_app_wheel_binding_search_uses_hovered_selection_and_initial_match() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let runtime = app.pane_runtimes.get_mut(&inactive).unwrap();
        let dimensions = runtime.runtime.terminal().stable_dimensions();
        runtime.ui.ordinary_selection = Some(StableOrdinarySelection::new(
            SelectionSourceCell {
                domain: dimensions.domain,
                row: dimensions.physical_top,
                column: 0,
            },
            SelectionSourceCell {
                domain: dimensions.domain,
                row: dimensions.physical_top,
                column: 3,
            },
            runtime.runtime.terminal().current_seqno(),
        ));
        let active_before = app.snapshot.clone();

        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::Search(WindowSearchCommandQuery::CurrentSelectionOrEmptyString),
            )
            .unwrap()
        );
        let target_ui = app.pane_ui_ref(inactive).unwrap();
        assert_eq!(target_ui.retained_search().unwrap().query, "left");
        assert!(target_ui.retained_search().unwrap().current.is_some());
        assert!(target_ui.retained_copy_mode().is_some());
        assert_eq!(app.snapshot, active_before);
        assert!(!app.active_ui.overlay_active());
    }

    #[test]
    fn window_app_wheel_binding_search_pattern_applies_initial_target_match_and_projection() {
        let (mut app, inactive) = wheel_copy_owner_fixture_for_test(true);
        let target_before = app.pane_snapshot(inactive).unwrap().clone();
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::Search(WindowSearchCommandQuery::Pattern {
                    pattern: "hit zero".to_owned(),
                    match_type: WindowSearchMatchType::CaseSensitive,
                }),
            )
            .unwrap()
        );
        let ui = app.pane_ui_ref(inactive).unwrap();
        let _current = ui
            .retained_search()
            .unwrap()
            .current
            .expect("initial match");
        assert!(ui.retained_copy_mode().is_some());
        assert!(
            super::pane_overlay_viewport_selection(
                app.pane_runtime_ref(inactive).unwrap().terminal(),
                ui,
                &app.selection_word_boundary,
            )
            .is_some()
        );
        assert_ne!(app.pane_snapshot(inactive).unwrap(), &target_before);
        assert!(!app.active_ui.overlay_active());
    }

    #[test]
    fn window_app_wheel_binding_selection_mouse_families_exit_target_overlay() {
        for command in [
            WindowCommand::SelectTextAtMouseCursorCell,
            WindowCommand::ExtendSelectionToMouseCursorCell,
        ] {
            let mut app = wheel_split_with_inactive_history_for_test();
            let inactive = rssh_core::PaneId::new(1);
            app.handle_pane_pty_output(inactive, b"\x1b]133;A\x07prompt")
                .unwrap();
            assert!(
                !app.pane_runtime_ref(inactive)
                    .unwrap()
                    .terminal()
                    .stable_semantic_prompt_rows()
                    .is_empty()
            );
            app.enter_wheel_target_copy_mode(inactive);
            if matches!(command, WindowCommand::ExtendSelectionToMouseCursorCell) {
                let source = app
                    .wheel_target_source_cell(super::WheelTarget {
                        pane_id: inactive,
                        rect: app.pane_render_rect(inactive).unwrap(),
                        cell: super::PaneMouseCell {
                            pane_id: inactive,
                            row: 0,
                            column: 0,
                        },
                        pixel_position: PhysicalPosition::new(1.0, 1.0),
                    })
                    .unwrap();
                app.pane_runtimes
                    .get_mut(&inactive)
                    .unwrap()
                    .ui
                    .ordinary_selection = Some(StableOrdinarySelection::new(source, source, 0));
            }
            move_wheel_to_pane_cell_for_test(&mut app, inactive, 1, 2, 1.0, 1.0);
            bind_wheel_command_for_test(&mut app, command);
            assert!(
                app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                    .unwrap()
            );
            assert!(!app.pane_ui_ref(inactive).unwrap().overlay_active());
            assert!(!app.active_ui.overlay_active());
        }
    }

    #[test]
    fn window_app_wheel_binding_complete_selection_distinguishes_drag_single_cell_and_link() {
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_copy = Arc::clone(&copied);
        let opened = Arc::new(Mutex::new(Vec::new()));
        let recorded_open = Arc::clone(&opened);
        let mut app = wheel_split_with_inactive_history_for_test();
        app.clipboard_writer = Box::new(move |text| {
            recorded_copy.lock().unwrap().push(text.to_owned());
            true
        });
        app.hyperlink_opener = Box::new(move |url| {
            recorded_open.lock().unwrap().push(url.to_owned());
            true
        });
        let inactive = rssh_core::PaneId::new(1);
        app.handle_pane_pty_output(
            inactive,
            b"\r\n\x1b]8;;https://inactive.test\x1b\\link\x1b]8;;\x1b\\",
        )
        .unwrap();
        let active = rssh_core::PaneId::new(2);
        let active_dimensions = app.runtime.terminal().stable_dimensions();
        let active_single = SelectionSourceCell {
            domain: active_dimensions.domain,
            row: active_dimensions.physical_top,
            column: 1,
        };
        app.active_ui.ordinary_selection = Some(StableOrdinarySelection::new(
            active_single,
            active_single,
            0,
        ));
        let terminal = app.pane_runtime_ref(inactive).unwrap().terminal();
        let dimensions = terminal.stable_dimensions();
        let single = SelectionSourceCell {
            domain: dimensions.domain,
            row: dimensions.physical_top.saturating_add(1),
            column: 1,
        };
        app.selecting = true;
        move_wheel_to_pane_cell_for_test(&mut app, active, 0, 1, 1.0, 1.0);
        bind_wheel_command_for_test(
            &mut app,
            WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(
                WindowCopyDestination::Clipboard,
            ),
        );
        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );
        assert!(app.active_ui.ordinary_selection.is_none());
        assert!(copied.lock().unwrap().is_empty());

        app.pane_runtimes
            .get_mut(&inactive)
            .unwrap()
            .ui
            .ordinary_selection = Some(StableOrdinarySelection::new(single, single, 0));
        app.selecting = false;
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 1, 1, 1.0, 1.0);
        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );
        assert_eq!(opened.lock().unwrap().as_slice(), ["https://inactive.test"]);
    }

    fn assert_wheel_complete_selection_semantics(command: &WindowCommand) {
        for single_cell in [true, false] {
            let clipboard = Arc::new(Mutex::new(Vec::new()));
            let clipboard_output = Arc::clone(&clipboard);
            let primary = Arc::new(Mutex::new(Vec::new()));
            let primary_output = Arc::clone(&primary);
            let mut app = wheel_split_with_inactive_history_for_test();
            app.clipboard_writer = Box::new(move |text| {
                clipboard_output.lock().unwrap().push(text.to_owned());
                true
            });
            app.primary_selection_writer = Box::new(move |text| {
                primary_output.lock().unwrap().push(text.to_owned());
                true
            });
            app.selecting = true;
            let inactive = rssh_core::PaneId::new(1);
            let terminal = app.pane_runtime_ref(inactive).unwrap().terminal();
            let dimensions = terminal.stable_dimensions();
            let start = SelectionSourceCell {
                domain: dimensions.domain,
                row: terminal.retained_stable_range().start,
                column: 0,
            };
            let end = SelectionSourceCell {
                column: if single_cell { 0 } else { 3 },
                ..start
            };
            app.pane_runtimes
                .get_mut(&inactive)
                .unwrap()
                .ui
                .ordinary_selection = Some(StableOrdinarySelection::new(start, end, 0));
            move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);
            bind_wheel_command_for_test(&mut app, command.clone());

            assert!(
                app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                    .unwrap()
            );
            let target_selection = app.pane_ui_ref(inactive).unwrap().ordinary_selection;
            if single_cell {
                assert!(target_selection.is_none(), "{command:?}");
                assert!(clipboard.lock().unwrap().is_empty(), "{command:?}");
                assert!(primary.lock().unwrap().is_empty(), "{command:?}");
            } else {
                assert!(target_selection.is_some(), "{command:?}");
                assert_eq!(
                    clipboard.lock().unwrap().as_slice(),
                    ["left"],
                    "{command:?}"
                );
                if *command == WindowCommand::CompleteSelection {
                    assert_eq!(primary.lock().unwrap().as_slice(), ["left"]);
                } else {
                    assert!(primary.lock().unwrap().is_empty());
                }
            }
            assert!(
                app.selecting,
                "inactive completion must not alter active selecting"
            );
            assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
        }
    }

    #[test]
    fn window_app_wheel_binding_complete_selection_uses_hovered_owner_semantics() {
        assert_wheel_complete_selection_semantics(&WindowCommand::CompleteSelection);
    }

    #[test]
    fn window_app_wheel_binding_complete_selection_to_uses_hovered_owner_semantics() {
        assert_wheel_complete_selection_semantics(&WindowCommand::CompleteSelectionTo(
            WindowCopyDestination::Clipboard,
        ));
    }

    #[test]
    fn window_app_wheel_binding_clear_scrollback_and_viewport_uses_target_terminal_core() {
        for command in [
            WindowCommand::ClearScrollback(WindowClearScrollbackMode::ScrollbackAndViewport),
            WindowCommand::ClearScrollbackAndViewport,
        ] {
            let mut app = wheel_split_with_inactive_history_for_test();
            let inactive = rssh_core::PaneId::new(1);
            app.enter_wheel_target_copy_mode(inactive);
            let active_before = app.snapshot.clone();
            assert!(run_wheel_command_on_pane_for_test(&mut app, inactive, command).unwrap());
            let runtime = app.pane_runtimes.get(&inactive).unwrap();
            assert!(runtime.runtime.terminal().scrollback().is_empty());
            assert_eq!(runtime.runtime.terminal().cursor().0, 0);
            assert!(
                runtime
                    .runtime
                    .terminal()
                    .stable_semantic_prompt_rows()
                    .is_empty()
            );
            assert!(!runtime.ui.overlay_active());
            assert!(runtime.ui.ordinary_selection.is_none());
            assert_eq!(
                runtime.ui.stable_viewport,
                super::PaneStableViewport::default()
            );
            assert_eq!(app.snapshot, active_before);
            assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
        }
    }

    #[test]
    fn window_app_wheel_binding_error_restores_event_state_without_fallback() {
        let active_written = Arc::new(Mutex::new(Vec::new()));
        let mut app = wheel_split_with_inactive_history_for_test();
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&active_written))));
        let inactive = rssh_core::PaneId::new(1);
        let target_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        app.handle_pane_pty_output(inactive, b"\x1b[?1000h")
            .unwrap();
        app.set_pane_scrollback_offset(inactive, 1);
        let target_offset_before = app
            .pane_ui_ref(inactive)
            .unwrap()
            .stable_viewport
            .scrollback_offset(app.pane_runtime_ref(inactive).unwrap().terminal());
        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.current_mouse_wheel_delta = Some(MouseScrollDelta::LineDelta(0.0, -7.0));
        app.mouse_assignments = vec![NativeUserMouseAssignment {
            event: NativeMouseAssignmentEvent {
                kind: NativeMouseAssignmentEventKind::Down,
                button: NativeMouseAssignmentButton::WheelUp,
                streak: 1,
            },
            modifiers: ModifiersState::CONTROL,
            mouse_reporting: false,
            alt_screen: NativeMouseAssignmentAltScreen::Any,
            command: WindowCommand::CloseWorkspace,
        }];
        app.bypass_mouse_reporting_modifiers = ModifiersState::SHIFT;
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);
        let error = app
            .handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
            .expect_err("last workspace close must propagate");
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(
            error.to_string(),
            "wheel action 'Close Workspace' failed: CannotCloseLastWorkspace"
        );
        assert_eq!(
            app.modifiers,
            ModifiersState::CONTROL | ModifiersState::SHIFT
        );
        assert_eq!(
            app.current_mouse_wheel_delta,
            Some(MouseScrollDelta::LineDelta(0.0, -7.0))
        );
        assert!(active_written.lock().unwrap().is_empty());
        assert!(target_written.lock().unwrap().is_empty());
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
        assert_eq!(app.current_scrollback_offset(), 0);
        assert_eq!(
            app.pane_ui_ref(inactive)
                .unwrap()
                .stable_viewport
                .scrollback_offset(app.pane_runtime_ref(inactive).unwrap().terminal()),
            target_offset_before
        );
    }

    #[test]
    fn window_app_wheel_binding_nested_command_stops_on_typed_missing_owner_error() {
        let active_written = Arc::new(Mutex::new(Vec::new()));
        let mut app = wheel_split_with_inactive_history_for_test();
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&active_written))));
        let inactive = rssh_core::PaneId::new(1);
        let target_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        app.handle_pane_pty_output(inactive, b"\x1b[?1000h")
            .unwrap();
        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.current_mouse_wheel_delta = Some(MouseScrollDelta::LineDelta(0.0, -9.0));
        app.bypass_mouse_reporting_modifiers = ModifiersState::SHIFT;
        app.mouse_assignments = vec![NativeUserMouseAssignment {
            event: NativeMouseAssignmentEvent {
                kind: NativeMouseAssignmentEventKind::Down,
                button: NativeMouseAssignmentButton::WheelUp,
                streak: 1,
            },
            modifiers: ModifiersState::CONTROL,
            mouse_reporting: false,
            alt_screen: NativeMouseAssignmentAltScreen::Any,
            command: WindowCommand::Multiple(vec![
                WindowCommand::ClosePane,
                WindowCommand::ClearScrollbackAndViewport,
            ]),
        }];
        move_wheel_to_pane_cell_for_test(&mut app, inactive, 0, 0, 1.0, 1.0);

        let error = app
            .handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
            .expect_err("second nested command must report the deleted owner");
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(
            error.to_string(),
            "wheel action 'Clear Scrollback And Viewport' failed: InvalidPane(PaneId(1))"
        );
        assert_eq!(
            app.modifiers,
            ModifiersState::CONTROL | ModifiersState::SHIFT
        );
        assert_eq!(
            app.current_mouse_wheel_delta,
            Some(MouseScrollDelta::LineDelta(0.0, -9.0))
        );
        assert!(!app.pane_runtimes.contains_key(&inactive));
        assert!(active_written.lock().unwrap().is_empty());
        assert!(target_written.lock().unwrap().is_empty());
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
        assert_eq!(app.current_scrollback_offset(), 0);
    }

    #[test]
    fn window_app_wheel_binding_copy_mode_assignment_retains_hovered_owner() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::Multiple(vec![
                    WindowCommand::EnterCopyMode,
                    WindowCommand::CopyMode(super::WindowCopyModeAssignment::SetSelectionMode(
                        super::WindowCopySelectionMode::Block,
                    )),
                ]),
            )
            .unwrap()
        );
        assert_eq!(
            app.pane_ui_ref(inactive)
                .unwrap()
                .copy_mode()
                .unwrap()
                .selection_mode,
            super::WindowCopySelectionMode::Block
        );
        assert!(app.active_ui.copy_mode().is_none());
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    #[test]
    fn window_app_wheel_binding_copy_mode_families_match_active_owner_semantics() {
        let assignments = [
            super::WindowCopyModeAssignment::MoveUp,
            super::WindowCopyModeAssignment::MoveRight,
            super::WindowCopyModeAssignment::MoveToScrollbackTop,
            super::WindowCopyModeAssignment::MoveToViewportTop,
            super::WindowCopyModeAssignment::MoveToStartOfLine,
            super::WindowCopyModeAssignment::PageUp,
            super::WindowCopyModeAssignment::SetSelectionMode(
                super::WindowCopySelectionMode::Block,
            ),
            super::WindowCopyModeAssignment::StartJump {
                forward: true,
                prev_char: false,
            },
        ];
        for assignment in assignments {
            let mut active = NativeWindowApp::new(None);
            active.runtime.resize(rssh_core::TerminalSize::new(20, 2));
            active
                .handle_pty_output(b"left-0\r\nleft-1\r\nleft-2\r\nleft-live")
                .unwrap();
            let active_pane = active.active_pane_id();
            assert!(
                run_wheel_command_on_pane_for_test(
                    &mut active,
                    active_pane,
                    WindowCommand::Multiple(vec![
                        WindowCommand::EnterCopyMode,
                        WindowCommand::CopyMode(assignment),
                    ]),
                )
                .unwrap()
            );

            let mut inactive = wheel_split_with_inactive_history_for_test();
            let inactive_pane = rssh_core::PaneId::new(1);
            assert!(
                run_wheel_command_on_pane_for_test(
                    &mut inactive,
                    inactive_pane,
                    WindowCommand::Multiple(vec![
                        WindowCommand::EnterCopyMode,
                        WindowCommand::CopyMode(assignment),
                    ]),
                )
                .unwrap()
            );

            let active_mode = active.active_ui.copy_mode().unwrap();
            let inactive_mode = inactive
                .pane_ui_ref(inactive_pane)
                .unwrap()
                .copy_mode()
                .unwrap();
            assert_eq!(inactive_mode.cursor, active_mode.cursor, "{assignment:?}");
            assert_eq!(
                inactive_mode.source_cursor, active_mode.source_cursor,
                "{assignment:?}"
            );
            assert_eq!(
                inactive_mode.selection_mode, active_mode.selection_mode,
                "{assignment:?}"
            );
            assert_eq!(
                inactive_mode.pending_jump, active_mode.pending_jump,
                "{assignment:?}"
            );
        }
    }

    #[test]
    fn window_app_wheel_binding_pane_actions_use_hovered_pane_id() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        assert!(
            run_wheel_command_on_pane_for_test(&mut app, inactive, WindowCommand::ResetTerminal)
                .unwrap()
        );
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::ClearScrollback(WindowClearScrollbackMode::ScrollbackOnly),
            )
            .unwrap()
        );
        assert!(
            app.pane_runtime_ref(inactive)
                .unwrap()
                .terminal()
                .scrollback()
                .is_empty()
        );
        assert!(
            run_wheel_command_on_pane_for_test(&mut app, inactive, WindowCommand::ClosePane)
                .unwrap()
        );
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
        assert!(app.pane_runtime_ref(inactive).is_none());
    }

    #[test]
    fn window_app_wheel_binding_global_action_keeps_window_scope() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let target_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        assert!((app.font_size_scale_for_test() - 1.0).abs() < f64::EPSILON);
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::Multiple(vec![
                    WindowCommand::IncreaseFontSize,
                    WindowCommand::SendString("target-after-global".to_owned()),
                ]),
            )
            .unwrap()
        );
        assert!((app.font_size_scale_for_test() - 1.1).abs() < f64::EPSILON);
        assert_eq!(
            target_written.lock().unwrap().as_slice(),
            b"target-after-global"
        );
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    #[test]
    fn window_app_wheel_binding_select_text_at_mouse_uses_hovered_local_cell() {
        let commands = [
            WindowCommand::SelectTextAtMouseCursorCell,
            WindowCommand::SelectTextAtMouseCursorWord,
            WindowCommand::SelectTextAtMouseCursorLine,
            WindowCommand::SelectTextAtMouseCursorBlock,
            WindowCommand::SelectTextAtMouseCursorSemanticZone,
            WindowCommand::SelectTextAtMouseCursor(WindowMouseSelectionMode::Cell),
        ];
        for command in commands {
            let mut app = wheel_split_with_inactive_history_for_test();
            let inactive = rssh_core::PaneId::new(1);
            move_wheel_to_pane_cell_for_test(&mut app, inactive, 1, 2, 1.0, 1.0);
            bind_wheel_command_for_test(&mut app, command);
            assert!(
                app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                    .unwrap()
            );
            assert!(
                app.pane_ui_ref(inactive)
                    .unwrap()
                    .ordinary_selection
                    .is_some()
            );
            assert!(app.active_ui.ordinary_selection.is_none());
            assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
        }
    }

    #[test]
    fn window_app_wheel_binding_extend_selection_uses_hovered_local_cell() {
        let commands = [
            WindowCommand::ExtendSelectionToMouseCursorCell,
            WindowCommand::ExtendSelectionToMouseCursorWord,
            WindowCommand::ExtendSelectionToMouseCursorLine,
            WindowCommand::ExtendSelectionToMouseCursorBlock,
            WindowCommand::ExtendSelectionToMouseCursorSemanticZone,
            WindowCommand::ExtendSelectionToMouseCursor(WindowMouseSelectionMode::Cell),
        ];
        for command in commands {
            let mut app = wheel_split_with_inactive_history_for_test();
            let inactive = rssh_core::PaneId::new(1);
            let runtime = app.pane_runtimes.get_mut(&inactive).unwrap();
            let dimensions = runtime.runtime.terminal().stable_dimensions();
            let anchor = SelectionSourceCell {
                domain: dimensions.domain,
                row: dimensions.physical_top,
                column: 0,
            };
            runtime.ui.ordinary_selection = Some(StableOrdinarySelection::new(
                anchor,
                anchor,
                runtime.runtime.terminal().current_seqno(),
            ));
            move_wheel_to_pane_cell_for_test(&mut app, inactive, 1, 3, 1.0, 1.0);
            bind_wheel_command_for_test(&mut app, command);
            assert!(
                app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                    .unwrap()
            );
            let selection = app
                .pane_ui_ref(inactive)
                .unwrap()
                .ordinary_selection
                .unwrap();
            assert_ne!(selection.focus, anchor);
            assert!(app.active_ui.ordinary_selection.is_none());
        }
    }

    #[test]
    fn window_app_wheel_binding_open_link_uses_hovered_snapshot_and_local_cell() {
        let commands = [
            WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursor,
            WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(
                WindowCopyDestination::Clipboard,
            ),
            WindowCommand::OpenLinkAtMouseCursor,
        ];
        for command in commands {
            let opened = Arc::new(Mutex::new(Vec::new()));
            let recorded = Arc::clone(&opened);
            let mut app = wheel_split_with_inactive_history_for_test();
            app.hyperlink_opener = Box::new(move |url| {
                recorded.lock().unwrap().push(url.to_owned());
                true
            });
            let inactive = rssh_core::PaneId::new(1);
            app.handle_pane_pty_output(
                inactive,
                b"\r\n\x1b]8;;https://inactive.test\x1b\\link\x1b]8;;\x1b\\",
            )
            .unwrap();
            move_wheel_to_pane_cell_for_test(&mut app, inactive, 1, 1, 1.0, 1.0);
            bind_wheel_command_for_test(&mut app, command);
            assert!(
                app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                    .unwrap()
            );
            assert_eq!(opened.lock().unwrap().as_slice(), ["https://inactive.test"]);
            assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
        }
    }

    #[test]
    fn window_app_wheel_binding_direction_focus_uses_hovered_pane_as_reference() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Down,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rssh_core::PaneId::new(2),
        })
        .unwrap();
        let hovered = rssh_core::PaneId::new(1);
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                hovered,
                WindowCommand::ActivatePaneDirection(rssh_core::app_shell::PaneDirection::Down),
            )
            .unwrap()
        );
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(3));
    }

    #[test]
    fn window_app_wheel_binding_by_index_keeps_tab_index_scope() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let target_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::Multiple(vec![
                    WindowCommand::ActivatePaneByIndex(1),
                    WindowCommand::SendString("retained-target".to_owned()),
                ]),
            )
            .unwrap()
        );
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
        assert_eq!(
            target_written.lock().unwrap().as_slice(),
            b"retained-target"
        );
    }

    #[test]
    fn window_app_wheel_binding_new_tab_keeps_explicit_creation_semantics() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let target_written = install_inactive_wheel_writer_for_test(&mut app, inactive);
        let before = app.app_shell.active_workspace().tabs().len();
        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::Multiple(vec![
                    WindowCommand::NewTab,
                    WindowCommand::SendString("retained-target".to_owned()),
                ]),
            )
            .unwrap()
        );
        assert_eq!(app.app_shell.active_workspace().tabs().len(), before + 1);
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(3));
        assert_eq!(
            target_written.lock().unwrap().as_slice(),
            b"retained-target"
        );
    }

    fn configure_wheel_default_prog_fixture(app: &mut NativeWindowApp, pane: rssh_core::PaneId) {
        app.dispatch_app_action(AppAction::SetPaneCurrentWorkingDir {
            pane,
            cwd: Some("C:/hovered-default-cwd".to_owned()),
        })
        .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            default_prog: Some(vec![
                "wheel-default-shell".to_owned(),
                "--wheel-login".to_owned(),
            ]),
            ..NativeConfigSnapshot::default()
        });
    }

    fn wheel_creation_fixture_for_test() -> NativeWindowApp {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rssh_pty::PtyCommand::new("hovered-original-shell").with_args(["--hovered-original"]),
        );
        app.runtime.resize(rssh_core::TerminalSize::new(20, 4));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: Some(
                PaneLaunch::local("active-original-shell")
                    .with_args(["--active-original"])
                    .with_cwd("C:/active-original-cwd"),
            ),
        })
        .unwrap();
        app
    }

    fn assert_wheel_default_prog_launch(launch: &PaneLaunch) {
        assert_eq!(launch.program(), "wheel-default-shell");
        assert_eq!(launch.args(), ["--wheel-login"]);
        assert_eq!(launch.cwd(), Some("C:/hovered-default-cwd"));
    }

    #[test]
    fn window_app_wheel_implicit_creations_use_default_prog_with_hovered_cwd() {
        let cases = [
            (WindowCommand::SplitRight, false),
            (WindowCommand::SplitDown, false),
            (WindowCommand::NewTab, false),
            (
                WindowCommand::SpawnTab(WindowSpawnTabDomain::CurrentPaneDomain),
                false,
            ),
            (
                WindowCommand::SpawnTab(WindowSpawnTabDomain::DefaultDomain),
                false,
            ),
            (
                WindowCommand::SpawnTab(WindowSpawnTabDomain::DomainName("local".to_owned())),
                false,
            ),
            (WindowCommand::SpawnWindow, true),
        ];
        for (command, pending_window) in cases {
            let mut app = wheel_creation_fixture_for_test();
            let hovered = rssh_core::PaneId::new(1);
            let original_program = app
                .wheel_reference_launch(hovered)
                .unwrap()
                .program()
                .to_owned();
            assert_eq!(original_program, "hovered-original-shell");
            assert_eq!(
                app.app_shell.active_pane().launch().program(),
                "active-original-shell"
            );
            configure_wheel_default_prog_fixture(&mut app, hovered);

            assert!(run_wheel_command_on_pane_for_test(&mut app, hovered, command).unwrap());

            if pending_window {
                let pending = app.app_shell.pending_windows().last().unwrap();
                let pane = pending
                    .tab()
                    .panes()
                    .iter()
                    .find(|pane| pane.id() == pending.active_pane_id())
                    .unwrap();
                assert_wheel_default_prog_launch(pane.launch());
            } else {
                assert_wheel_default_prog_launch(app.app_shell.active_pane().launch());
            }
        }
    }

    #[test]
    fn window_app_wheel_optionless_split_pane_uses_default_prog_and_hovered_source() {
        let mut app = wheel_creation_fixture_for_test();
        let hovered = rssh_core::PaneId::new(1);
        configure_wheel_default_prog_fixture(&mut app, hovered);
        let options = WindowSplitPaneOptions {
            direction: SplitDirection::Right,
            domain: Some(WindowSpawnTabDomain::CurrentPaneDomain),
            command: None,
            command_options: None,
            size: Some(WindowSplitPaneSize::Cells(3)),
            top_level: false,
        };

        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                hovered,
                WindowCommand::SplitPane(options),
            )
            .unwrap()
        );

        let pane = app.app_shell.active_pane();
        assert_wheel_default_prog_launch(pane.launch());
        assert_eq!(pane.split().unwrap().source_pane, hovered);
    }

    #[test]
    fn window_app_wheel_implicit_creation_without_default_prog_clones_hovered_launch() {
        let mut app = wheel_creation_fixture_for_test();
        let hovered = rssh_core::PaneId::new(1);
        app.dispatch_app_action(AppAction::SetPaneCurrentWorkingDir {
            pane: hovered,
            cwd: Some("C:/hovered-fallback".to_owned()),
        })
        .unwrap();
        let expected = app.wheel_reference_launch(hovered).unwrap();
        assert!(app.default_prog.is_none());

        assert!(
            run_wheel_command_on_pane_for_test(&mut app, hovered, WindowCommand::SplitDown,)
                .unwrap()
        );

        assert_eq!(app.app_shell.active_pane().launch(), &expected);
    }

    #[test]
    fn window_app_wheel_implicit_creation_keeps_hovered_ssh_domain_with_default_prog() {
        let mut app = NativeWindowApp::new(None);
        app.set_initial_pane_launch(PaneLaunch::ssh(
            SshPaneLaunch::new(
                "ops@example.test:2222",
                SshAuthDescription::PasswordPrompt,
                SshKnownHostsPolicy::Prompt,
            )
            .with_remote_command(["uptime"]),
        ));
        let hovered = app.active_pane_id();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: hovered,
            direction: SplitDirection::Right,
            launch: Some(PaneLaunch::local("active-local")),
        })
        .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            default_prog: Some(vec!["wheel-default-shell".to_owned()]),
            ..NativeConfigSnapshot::default()
        });

        assert!(
            run_wheel_command_on_pane_for_test(&mut app, hovered, WindowCommand::SplitDown)
                .unwrap()
        );

        let rssh_core::app_shell::PaneLaunchDomain::Ssh(ssh) =
            app.app_shell.active_pane().launch().domain()
        else {
            panic!("wheel child must inherit the hovered SSH domain");
        };
        assert_eq!(ssh.target(), "ops@example.test:2222");
        assert!(ssh.remote_command().is_empty());
    }

    #[test]
    fn window_app_wheel_explicit_creation_preserves_program_cwd_and_domain() {
        for command in [
            WindowCommand::SplitPane(WindowSplitPaneOptions {
                direction: SplitDirection::Right,
                domain: Some(WindowSpawnTabDomain::DefaultDomain),
                command: Some(WindowSpawnCommandQuery {
                    label: None,
                    program: "explicit-wheel-program".to_owned(),
                    args: vec!["--explicit".to_owned()],
                    cwd: Some("C:/explicit-wheel-cwd".to_owned()),
                    environment: BTreeMap::new(),
                    domain: Some(WindowSpawnTabDomain::DefaultDomain),
                    window_position: None,
                }),
                command_options: None,
                size: None,
                top_level: false,
            }),
            WindowCommand::SpawnCommandInNewTab(WindowSpawnCommandQuery {
                label: None,
                program: "explicit-wheel-program".to_owned(),
                args: vec!["--explicit".to_owned()],
                cwd: Some("C:/explicit-wheel-cwd".to_owned()),
                environment: BTreeMap::new(),
                domain: Some(WindowSpawnTabDomain::DefaultDomain),
                window_position: None,
            }),
        ] {
            let mut app = wheel_creation_fixture_for_test();
            let hovered = rssh_core::PaneId::new(1);
            configure_wheel_default_prog_fixture(&mut app, hovered);

            assert!(run_wheel_command_on_pane_for_test(&mut app, hovered, command).unwrap());

            let launch = app.app_shell.active_pane().launch();
            assert_eq!(launch.program(), "explicit-wheel-program");
            assert_eq!(launch.args(), ["--explicit"]);
            assert_eq!(launch.cwd(), Some("C:/explicit-wheel-cwd"));
        }
    }

    #[test]
    fn window_app_wheel_binding_split_uses_hovered_pane_as_source() {
        let mut app = wheel_split_with_inactive_history_for_test();
        app.runtime.resize(rssh_core::TerminalSize::new(20, 6));
        let inactive = rssh_core::PaneId::new(1);
        assert!(
            run_wheel_command_on_pane_for_test(&mut app, inactive, WindowCommand::SplitDown)
                .unwrap()
        );
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(3));
        let layout = app.pane_render_layout();
        let target_split_source = layout
            .separators
            .iter()
            .find(|separator| separator.new_pane == rssh_core::PaneId::new(3))
            .map(|separator| separator.source_pane);
        assert_eq!(target_split_source, Some(inactive), "{layout:?}");
    }

    #[test]
    fn window_app_wheel_binding_split_pane_options_use_hovered_source_and_size() {
        let mut app = wheel_split_with_inactive_history_for_test();
        app.runtime.resize(rssh_core::TerminalSize::new(40, 6));
        let inactive = rssh_core::PaneId::new(1);
        let target_columns = app.pane_render_rect(inactive).unwrap().columns;
        let size = WindowSplitPaneSize::Cells(3);
        let expected_delta = super::split_pane_source_size_delta(target_columns, size);
        let options = WindowSplitPaneOptions {
            direction: SplitDirection::Right,
            domain: Some(WindowSpawnTabDomain::CurrentPaneDomain),
            command: None,
            command_options: None,
            size: Some(size),
            top_level: false,
        };

        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::SplitPane(options),
            )
            .unwrap()
        );

        let split = app
            .app_shell
            .active_tab()
            .panes()
            .iter()
            .find(|pane| pane.id() == rssh_core::PaneId::new(3))
            .and_then(rssh_core::app_shell::Pane::split)
            .unwrap();
        assert_eq!(split.source_pane, inactive);
        assert_eq!(split.source_size_delta, expected_delta);
    }

    #[test]
    fn window_app_wheel_binding_new_tab_current_domain_uses_hovered_reference() {
        for command in [
            WindowCommand::NewTab,
            WindowCommand::SpawnTab(WindowSpawnTabDomain::CurrentPaneDomain),
        ] {
            let mut app = wheel_split_with_inactive_history_for_test();
            let inactive = rssh_core::PaneId::new(1);
            app.dispatch_app_action(AppAction::SetPaneCurrentWorkingDir {
                pane: inactive,
                cwd: Some("C:/hovered".to_owned()),
            })
            .unwrap();
            app.dispatch_app_action(AppAction::SetPaneCurrentWorkingDir {
                pane: rssh_core::PaneId::new(2),
                cwd: Some("C:/active".to_owned()),
            })
            .unwrap();

            assert!(run_wheel_command_on_pane_for_test(&mut app, inactive, command).unwrap());

            assert_eq!(
                app.app_shell.active_pane().launch().cwd(),
                Some("C:/hovered")
            );
        }
    }

    #[test]
    fn window_app_wheel_binding_spawn_window_uses_hovered_reference() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        app.dispatch_app_action(AppAction::SetPaneCurrentWorkingDir {
            pane: inactive,
            cwd: Some("C:/hovered".to_owned()),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SetPaneCurrentWorkingDir {
            pane: rssh_core::PaneId::new(2),
            cwd: Some("C:/active".to_owned()),
        })
        .unwrap();

        assert!(
            run_wheel_command_on_pane_for_test(&mut app, inactive, WindowCommand::SpawnWindow,)
                .unwrap()
        );

        let pending = app.app_shell.pending_windows().last().unwrap();
        let pending_pane = pending
            .tab()
            .panes()
            .iter()
            .find(|pane| pane.id() == pending.active_pane_id())
            .unwrap();
        assert_eq!(pending_pane.launch().cwd(), Some("C:/hovered"));
    }

    #[test]
    fn window_app_wheel_binding_rotate_panes_keeps_tab_scope() {
        let mut app = wheel_split_with_inactive_history_for_test();
        let inactive = rssh_core::PaneId::new(1);
        let before = app
            .app_shell
            .active_tab()
            .panes()
            .iter()
            .map(rssh_core::app_shell::Pane::id)
            .collect::<Vec<_>>();

        assert!(
            run_wheel_command_on_pane_for_test(
                &mut app,
                inactive,
                WindowCommand::RotatePanesClockwise,
            )
            .unwrap()
        );

        let after = app
            .app_shell
            .active_tab()
            .panes()
            .iter()
            .map(rssh_core::app_shell::Pane::id)
            .collect::<Vec<_>>();
        assert_ne!(after, before);
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
    }

    #[test]
    fn window_app_mouse_wheel_scrolls_tabs_from_tab_bar() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rssh_core::TabId::new(3));
        app.set_config_overrides(native_config_snapshot! {
            mouse_wheel_scrolls_tabs: Some(true),
            ..NativeConfigSnapshot::default()
        });

        app.handle_cursor_moved(PhysicalPosition::new(f64::from(CELL_WIDTH), 0.0))
            .unwrap();
        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );
        assert_eq!(app.active_tab_id(), rssh_core::TabId::new(2));

        app.set_config_overrides(native_config_snapshot! {
            mouse_wheel_scrolls_tabs: Some(false),
            ..NativeConfigSnapshot::default()
        });
        assert!(
            !app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );
        assert_eq!(app.active_tab_id(), rssh_core::TabId::new(2));
    }

    #[test]
    fn window_app_resizes_right_split_pane_left() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"left").unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"right").unwrap();

        app.dispatch_app_action(AppAction::ResizePane {
            pane: rssh_core::PaneId::new(2),
            direction: rssh_core::app_shell::ResizeDirection::Left,
            amount: 5,
        })
        .unwrap();

        let snapshot = app.render_snapshot();

        assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS, 34), Some('|'));
        assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS, 35), Some('r'));
    }

    #[test]
    fn split_resize_cursor_icon_matches_wezterm_split_axes() {
        assert_eq!(
            super::split_resize_cursor_icon(SplitDirection::Left),
            CursorIcon::EwResize
        );
        assert_eq!(
            super::split_resize_cursor_icon(SplitDirection::Right),
            CursorIcon::EwResize
        );
        assert_eq!(
            super::split_resize_cursor_icon(SplitDirection::Up),
            CursorIcon::NsResize
        );
        assert_eq!(
            super::split_resize_cursor_icon(SplitDirection::Down),
            CursorIcon::NsResize
        );
    }

    #[test]
    fn window_app_tracks_mouse_cursor_icon_without_native_window() {
        let mut app = NativeWindowApp::new(None);
        assert_eq!(app.mouse_cursor_icon, CursorIcon::Default);

        app.set_mouse_cursor_icon(CursorIcon::EwResize);

        assert_eq!(app.mouse_cursor_icon, CursorIcon::EwResize);
    }

    #[test]
    fn window_app_uses_resize_cursor_for_split_separator() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(39_u32 * CELL_WIDTH),
            f64::from(app.terminal_pixel_top()),
        ))
        .unwrap();
        assert_eq!(app.mouse_cursor_icon, CursorIcon::EwResize);

        app.handle_cursor_moved(PhysicalPosition::new(
            0.0,
            f64::from(app.terminal_pixel_top()),
        ))
        .unwrap();
        assert_eq!(app.mouse_cursor_icon, CursorIcon::Default);

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(39_u32 * CELL_WIDTH),
            f64::from(app.terminal_pixel_top()),
        ))
        .unwrap();
        app.handle_cursor_left();
        assert_eq!(app.mouse_cursor_icon, CursorIcon::Default);
    }

    #[test]
    fn window_app_uses_vertical_resize_cursor_for_down_split_separator() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Down,
            launch: None,
        })
        .unwrap();
        let separator = app.pane_render_layout().separators[0];
        let terminal_row = separator
            .row
            .saturating_sub(app.terminal_frame_row_offset());

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(u32::from(separator.column) * CELL_WIDTH),
            f64::from(
                app.terminal_pixel_top()
                    .saturating_add(u32::from(terminal_row) * CELL_HEIGHT),
            ),
        ))
        .unwrap();

        assert_eq!(app.mouse_cursor_icon, CursorIcon::NsResize);
    }

    #[test]
    fn window_app_dragging_right_split_separator_resizes_panes() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"left").unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"right").unwrap();

        let separator_x = 39_u32 * CELL_WIDTH;
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(separator_x),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();
        assert_eq!(app.mouse_cursor_icon, CursorIcon::EwResize);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert_eq!(app.mouse_cursor_icon, CursorIcon::EwResize);
        assert!(app.split_resize_dragging.is_some());

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(43_u32 * CELL_WIDTH),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();
        assert_eq!(app.mouse_cursor_icon, CursorIcon::EwResize);
        assert!(app.split_resize_dragging.is_some());
        assert_eq!(app.pane_render_layout().separators[0].column, 43);

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(47_u32 * CELL_WIDTH),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();
        assert_eq!(app.mouse_cursor_icon, CursorIcon::EwResize);
        assert!(app.split_resize_dragging.is_some());
        assert_eq!(app.pane_render_layout().separators[0].column, 47);
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );
        assert_eq!(app.mouse_cursor_icon, CursorIcon::EwResize);
        assert!(app.split_resize_dragging.is_none());

        let snapshot = app.render_snapshot();
        assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS, 47), Some('|'));
        assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS, 48), Some('r'));
    }

    #[test]
    fn window_app_preserves_split_ratio_when_window_width_changes() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::ResizePane {
            pane: rssh_core::PaneId::new(1),
            direction: ResizeDirection::Right,
            amount: 8,
        })
        .unwrap();

        let before = app.pane_render_layout();
        let before_source = before
            .panes
            .iter()
            .find(|pane| pane.pane_id == rssh_core::PaneId::new(1))
            .expect("source pane before resize");
        let before_usable = before.panes.iter().map(|pane| pane.columns).sum::<u16>();
        let next_size =
            rssh_core::TerminalSize::new(160, app.runtime.terminal().grid().size().rows);

        app.handle_window_resize(app.frame_size_for_terminal_size(next_size))
            .unwrap();

        let after = app.pane_render_layout();
        let after_source = after
            .panes
            .iter()
            .find(|pane| pane.pane_id == rssh_core::PaneId::new(1))
            .expect("source pane after resize");
        let after_usable = after.panes.iter().map(|pane| pane.columns).sum::<u16>();
        let expected = (u32::from(before_source.columns) * u32::from(after_usable)
            + u32::from(before_usable) / 2)
            / u32::from(before_usable);

        assert!(
            u32::from(after_source.columns).abs_diff(expected) <= 1,
            "source pane changed from {}/{} to {}/{}, expected {expected}±1 cells",
            before_source.columns,
            before_usable,
            after_source.columns,
            after_usable
        );
    }

    #[test]
    fn window_app_uses_separator_drag_ratio_as_resize_baseline() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        let separator = app.pane_render_layout().separators[0];
        let separator_x = u32::from(separator.column) * CELL_WIDTH;
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(separator_x),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(separator_x + 12 * CELL_WIDTH),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );
        let before = app.pane_render_layout();
        let before_source = before
            .panes
            .iter()
            .find(|pane| pane.pane_id == rssh_core::PaneId::new(1))
            .expect("source pane before resize");
        let before_other = before
            .panes
            .iter()
            .find(|pane| pane.pane_id == rssh_core::PaneId::new(2))
            .expect("new pane before resize");
        let next_size =
            rssh_core::TerminalSize::new(160, app.runtime.terminal().grid().size().rows);

        app.handle_window_resize(app.frame_size_for_terminal_size(next_size))
            .unwrap();

        let after = app.pane_render_layout();
        let after_source = after
            .panes
            .iter()
            .find(|pane| pane.pane_id == rssh_core::PaneId::new(1))
            .expect("source pane after resize");
        let after_other = after
            .panes
            .iter()
            .find(|pane| pane.pane_id == rssh_core::PaneId::new(2))
            .expect("new pane after resize");
        let old_usable = u32::from(before_source.columns + before_other.columns);
        let new_usable = u32::from(after_source.columns + after_other.columns);
        let expected =
            (u32::from(before_source.columns) * new_usable + old_usable / 2) / old_usable;

        assert!(
            u32::from(after_source.columns).abs_diff(expected) <= 1,
            "dragged ratio changed from {}/{} to {}/{}, expected {expected}±1 cells",
            before_source.columns,
            old_usable,
            after_source.columns,
            new_usable
        );
    }

    #[test]
    fn window_app_preserves_split_ratio_with_percentage_padding() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            window_padding: Some(NativeWindowPadding {
                left: NativeWindowPaddingDimension::Percent(10),
                right: NativeWindowPaddingDimension::Pixels(CELL_WIDTH),
                top: NativeWindowPaddingDimension::Percent(10),
                bottom: NativeWindowPaddingDimension::Pixels(CELL_HEIGHT),
            }),
            ..NativeConfigSnapshot::default()
        });
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::ResizePane {
            pane: rssh_core::PaneId::new(1),
            direction: ResizeDirection::Right,
            amount: 7,
        })
        .unwrap();
        let before = app.pane_render_layout();
        let before_source = before
            .panes
            .iter()
            .find(|pane| pane.pane_id == rssh_core::PaneId::new(1))
            .expect("source pane before resize");
        let before_other = before
            .panes
            .iter()
            .find(|pane| pane.pane_id == rssh_core::PaneId::new(2))
            .expect("new pane before resize");
        let owner_before = app.active_pane_id();
        let mut inactive_owners_before = app
            .pane_runtimes
            .keys()
            .map(|pane_id| pane_id.get())
            .collect::<Vec<_>>();
        inactive_owners_before.sort_unstable();
        let next_size =
            rssh_core::TerminalSize::new(160, app.runtime.terminal().grid().size().rows);

        app.handle_window_resize(app.frame_size_for_terminal_size(next_size))
            .unwrap();

        let after = app.pane_render_layout();
        let after_source = after
            .panes
            .iter()
            .find(|pane| pane.pane_id == rssh_core::PaneId::new(1))
            .expect("source pane after resize");
        let after_other = after
            .panes
            .iter()
            .find(|pane| pane.pane_id == rssh_core::PaneId::new(2))
            .expect("new pane after resize");
        let old_usable = u32::from(before_source.columns + before_other.columns);
        let new_usable = u32::from(after_source.columns + after_other.columns);
        let expected =
            (u32::from(before_source.columns) * new_usable + old_usable / 2) / old_usable;
        let mut inactive_owners_after = app
            .pane_runtimes
            .keys()
            .map(|pane_id| pane_id.get())
            .collect::<Vec<_>>();
        inactive_owners_after.sort_unstable();

        assert!(u32::from(after_source.columns).abs_diff(expected) <= 1);
        assert_eq!(app.active_pane_id(), owner_before);
        assert_eq!(inactive_owners_after, inactive_owners_before);
    }

    #[test]
    fn window_app_preserves_down_split_ratio_when_rows_change() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Down,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::ResizePane {
            pane: rssh_core::PaneId::new(1),
            direction: ResizeDirection::Down,
            amount: 4,
        })
        .unwrap();
        let before = app.pane_render_layout();
        let before_source = before
            .panes
            .iter()
            .find(|pane| pane.pane_id == rssh_core::PaneId::new(1))
            .expect("source pane before resize");
        let before_other = before
            .panes
            .iter()
            .find(|pane| pane.pane_id == rssh_core::PaneId::new(2))
            .expect("new pane before resize");
        let owner_before = app.active_pane_id();
        let next_size =
            rssh_core::TerminalSize::new(app.runtime.terminal().grid().size().columns, 48);

        app.handle_window_resize(app.frame_size_for_terminal_size(next_size))
            .unwrap();

        let after = app.pane_render_layout();
        let after_source = after
            .panes
            .iter()
            .find(|pane| pane.pane_id == rssh_core::PaneId::new(1))
            .expect("source pane after resize");
        let after_other = after
            .panes
            .iter()
            .find(|pane| pane.pane_id == rssh_core::PaneId::new(2))
            .expect("new pane after resize");
        let old_usable = u32::from(before_source.rows + before_other.rows);
        let new_usable = u32::from(after_source.rows + after_other.rows);
        let expected = (u32::from(before_source.rows) * new_usable + old_usable / 2) / old_usable;

        assert!(u32::from(after_source.rows).abs_diff(expected) <= 1);
        assert_eq!(app.active_pane_id(), owner_before);
    }

    #[test]
    fn window_app_failed_shell_action_restores_split_resize_pointer_state() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(39_u32 * CELL_WIDTH),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        let drag = app.split_resize_dragging.expect("split resize drag");
        assert_eq!(app.active_mouse_button, Some(MouseButton::Left));

        assert!(
            app.dispatch_app_action(AppAction::ActivatePane {
                pane: rssh_core::PaneId::new(999),
            })
            .is_err()
        );

        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
        assert_eq!(app.split_resize_dragging, Some(drag));
        assert_eq!(app.active_mouse_button, Some(MouseButton::Left));
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );
        assert!(app.split_resize_dragging.is_none());
    }

    #[test]
    fn window_app_split_resize_start_clears_stable_ordinary_selection() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"right").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 4 },
        );

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(39_u32 * CELL_WIDTH),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        app.refresh_snapshot();

        assert!(ordinary_selection_for_test(&app).is_none());
        assert!(app.selection.is_none());
    }

    #[test]
    fn window_app_zoomed_split_pane_fills_tab_region() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"left").unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"right").unwrap();

        app.dispatch_app_action(AppAction::TogglePaneZoom {
            pane: rssh_core::PaneId::new(2),
        })
        .unwrap();

        let snapshot = app.render_snapshot();
        assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS, 0), Some('r'));
        assert_ne!(snapshot_char(&snapshot, TAB_BAR_ROWS, 39), Some('|'));

        app.dispatch_app_action(AppAction::TogglePaneZoom {
            pane: rssh_core::PaneId::new(2),
        })
        .unwrap();

        let snapshot = app.render_snapshot();
        assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS, 39), Some('|'));
        assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS, 40), Some('r'));
    }

    #[test]
    fn window_title_reports_app_shell_state() {
        let app = NativeWindowApp::new_with_command(
            None,
            rssh_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        assert_eq!(
            app.effective_window_title(),
            "R-SSH [workspace:1 tab:1 pane:1]"
        );
    }

    #[test]
    fn window_title_formatter_can_override_default_title() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let mut app = NativeWindowApp::new(None);
        app.window_title_formatter = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            Some(format!(
                "tab:{} pane:{} tabs:{} panes:{}",
                event.active_tab.get(),
                event.active_pane.get(),
                event.tab_count,
                event.pane_count
            ))
        });

        assert_eq!(app.effective_window_title(), "tab:1 pane:1 tabs:1 panes:1");
        let events = seen.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].default_title, "R-SSH [workspace:1 tab:1 pane:1]");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_event_string_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return 'STATIC LUA TITLE'
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title event string return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "STATIC LUA TITLE");
    }

    #[test]
    fn window_app_uses_first_static_wezterm_format_window_title_handler() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return 'FIRST LUA TITLE'
            end)

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return 'SECOND LUA TITLE'
            end)
            "#,
        )
        .expect("expected first static WezTerm format-window-title handler");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "FIRST LUA TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_tostring_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return tostring('TOSTRING LUA TITLE')
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title tostring return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "TOSTRING LUA TITLE");
    }

    #[test]
    fn window_manager_transfers_a_live_tab_between_windows_and_remaps_events() {
        let mut source = NativeWindowApp::new(None);
        source
            .dispatch_app_action(AppAction::NewTab { launch: None })
            .expect("expected second source tab");
        let mut target = NativeWindowApp::new(None);
        target.app_window_id = rssh_core::WindowId::new(2);

        let mut manager = NativeWindowManager::new_for_test(source);
        manager.pending_apps.push(Box::new(target));
        manager
            .transfer_tab_between_windows(
                rssh_core::WindowId::new(1),
                rssh_core::WindowId::new(2),
                rssh_core::TabId::new(1),
                1,
            )
            .expect("expected live tab transfer");

        let source = manager
            .startup_app
            .as_ref()
            .expect("source remains open with its other tab");
        assert_eq!(source.app_shell.active_workspace().tabs().len(), 1);
        assert_eq!(source.app_shell.active_tab_id(), rssh_core::TabId::new(2));

        let target = manager
            .pending_apps
            .first()
            .expect("target window remains managed");
        assert_eq!(target.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(target.app_shell.active_tab_id(), rssh_core::TabId::new(2));
        assert_eq!(
            manager
                .pane_event_routes
                .get(&(rssh_core::WindowId::new(1), rssh_core::PaneId::new(1))),
            Some(&crate::window::PaneEventRoute {
                window_id: rssh_core::WindowId::new(2),
                pane_id: rssh_core::PaneId::new(2),
            })
        );
    }

    #[test]
    fn existing_window_transfer_retires_source_local_worker_and_preserves_projection() {
        let mut source = NativeWindowApp::new(None);
        source.handle_pty_output(b"existing-local-scrollback").unwrap();
        source
            .handle_pty_output(b"\x1b]7;file://host/existing-transfer-cwd\x07")
            .unwrap();
        let source_pane = source.active_pane_id();
        let source_driver =
            install_local_v2_worker_for_window_transfer(&mut source, source_pane);
        source
            .dispatch_app_action(AppAction::NewTab {
                launch: Some(PaneLaunch::local("source-stays")),
            })
            .unwrap();
        let mut target = NativeWindowApp::new(None);
        target.app_window_id = rssh_core::WindowId::new(2);
        let mut manager = NativeWindowManager::new_for_test(source);
        manager.pending_apps.push(Box::new(target));

        manager
            .transfer_tab_between_windows(
                rssh_core::WindowId::new(1),
                rssh_core::WindowId::new(2),
                rssh_core::TabId::new(1),
                1,
            )
            .unwrap();

        let source = manager.startup_app.as_ref().unwrap();
        assert!(
            source
                .runtime
                .worker()
                .is_none_or(|worker| !worker.contains_pane(source_pane)),
            "the source hub must retire the moved local transport"
        );
        assert!(source_driver.interrupt_calls() > 0);
        let target = manager.pending_apps.first().unwrap();
        let target_pane = target.active_pane_id();
        assert_ne!(target_pane, source_pane, "import assigns a fresh pane id");
        assert_eq!(
            target.app_shell.active_pane().launch().cwd(),
            Some("file://host/existing-transfer-cwd")
        );
        assert_eq!(
            snapshot_row_text(
                &target.snapshot,
                0,
                u16::try_from("existing-local-scrollback".len()).unwrap(),
            ),
            "existing-local-scrollback"
        );
        let target = manager.pending_apps.first_mut().unwrap();
        let target_driver =
            install_restarted_local_worker_after_window_transfer(target, target_pane);
        target
            .write_pty_bytes_to_pane(target_pane, b"existing-target-input")
            .unwrap();
        target_driver.wait_until_accepted_write_len("existing-target-input".len());
        assert_eq!(target_driver.accepted_writes(), b"existing-target-input");
        let resized = TerminalSize::new(101, 33);
        target.resize_terminal_runtimes(resized).unwrap();
        target_driver.wait_until_control_call_count(1);
        assert_eq!(target_driver.control_log().resizes, vec![resized]);
        assert!(source_driver.accepted_writes().is_empty());
        publish_restarted_local_output(
            target,
            &target_driver,
            target_pane,
            b"-continued",
            "existing-local-scrollback-continued",
        );
        assert_eq!(
            snapshot_row_text(
                &target.snapshot,
                0,
                u16::try_from("existing-local-scrollback-continued".len()).unwrap(),
            ),
            "existing-local-scrollback-continued"
        );
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn window_manager_remaps_ssh_auxiliary_ownership_with_a_live_tab() {
        let mut source = NativeWindowApp::new(None);
        source.set_initial_pane_launch(PaneLaunch::ssh(SshPaneLaunch::new(
            "ops@managed-transfer.example:2222",
            SshAuthDescription::PasswordPrompt,
            SshKnownHostsPolicy::Prompt,
        )));
        let source_ssh_pane = source.active_pane_id();
        source.active_runtime_generation = 77;
        source.handle_ssh_state(source_ssh_pane, super::ConnectionState::Connecting);
        let (command_sender, command_receiver) = mpsc::channel();
        source
            .ssh_writer_senders
            .insert(source_ssh_pane, command_sender);
        source
            .dispatch_app_action(AppAction::NewTab {
                launch: Some(PaneLaunch::local("local-shell")),
            })
            .unwrap();
        let mut target = NativeWindowApp::new(None);
        target.app_window_id = rssh_core::WindowId::new(2);
        let mut manager = NativeWindowManager::new_for_test(source);
        manager.pending_apps.push(Box::new(target));

        manager
            .transfer_tab_between_windows(
                rssh_core::WindowId::new(1),
                rssh_core::WindowId::new(2),
                rssh_core::TabId::new(1),
                1,
            )
            .unwrap();

        let source = manager.startup_app.as_ref().unwrap();
        assert!(!source.ssh_writer_senders.contains_key(&source_ssh_pane));
        assert!(!source.ssh_connection_states.contains_key(&source_ssh_pane));
        let target = manager.pending_apps.first().unwrap();
        let target_ssh_pane = target.active_pane_id();
        assert_ne!(source_ssh_pane, target_ssh_pane);
        assert!(target.ssh_writer_senders.contains_key(&target_ssh_pane));
        assert_eq!(
            target.ssh_connection_state_for_pane(target_ssh_pane),
            super::ConnectionState::Connecting
        );
        assert_eq!(
            target.metrics.connection_state(),
            super::ConnectionState::Connecting
        );
        target
            .ssh_writer_senders
            .get(&target_ssh_pane)
            .unwrap()
            .send(super::NativeSshCommand::Resize(TerminalSize::new(100, 30)))
            .unwrap();
        assert!(matches!(
            command_receiver.recv().unwrap(),
            super::NativeSshCommand::Resize(size) if size == TerminalSize::new(100, 30)
        ));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn window_manager_routes_moved_ssh_events_through_the_target_pane_identity() {
        let mut source = NativeWindowApp::new(None);
        source.set_initial_pane_launch(PaneLaunch::ssh(SshPaneLaunch::new(
            "ops@event-transfer.example:2222",
            SshAuthDescription::PasswordPrompt,
            SshKnownHostsPolicy::Prompt,
        )));
        let source_ssh_pane = source.active_pane_id();
        source.active_runtime_generation = 77;
        source.handle_ssh_state(source_ssh_pane, super::ConnectionState::Connecting);
        source
            .dispatch_app_action(AppAction::NewTab {
                launch: Some(PaneLaunch::local("local-shell")),
            })
            .unwrap();
        let mut target = NativeWindowApp::new(None);
        target.app_window_id = rssh_core::WindowId::new(2);
        let mut manager = NativeWindowManager::new_for_test(source);
        manager.pending_apps.push(Box::new(target));
        manager
            .transfer_tab_between_windows(
                rssh_core::WindowId::new(1),
                rssh_core::WindowId::new(2),
                rssh_core::TabId::new(1),
                1,
            )
            .unwrap();
        let target_ssh_pane = manager.pending_apps.first().unwrap().active_pane_id();

        assert_eq!(
            manager.dispatch_user_event_to_owner(super::WindowUserEvent::SshState {
                window_id: rssh_core::WindowId::new(1),
                pane_id: source_ssh_pane,
                runtime_generation: 77,
                state: super::ConnectionState::Connected,
            }),
            Some(false)
        );
        assert_eq!(
            manager
                .pending_apps
                .first()
                .unwrap()
                .ssh_connection_state_for_pane(target_ssh_pane),
            super::ConnectionState::Connected
        );

        let (decision_sender, _decision_receiver) = mpsc::sync_channel(1);
        assert_eq!(
            manager.dispatch_user_event_to_owner(super::WindowUserEvent::HostKeyPrompt {
                window_id: rssh_core::WindowId::new(1),
                pane_id: source_ssh_pane,
                runtime_generation: 77,
                challenge: rssh_ssh::HostKeyChallenge::new(
                    "event-transfer.example",
                    2222,
                    "ssh-ed25519",
                    "SHA256:event-transfer",
                    rssh_ssh::HostKeyStatus::Unknown,
                ),
                decision: decision_sender,
            }),
            Some(false)
        );
        let target = manager.pending_apps.first().unwrap();
        assert!(target.ssh_host_key_prompts.contains_key(&target_ssh_pane));
        assert!(!target.ssh_host_key_prompts.contains_key(&source_ssh_pane));

        let (response_sender, _response_receiver) = mpsc::sync_channel(1);
        assert_eq!(
            manager.dispatch_user_event_to_owner(super::WindowUserEvent::SecretPrompt {
                window_id: rssh_core::WindowId::new(1),
                pane_id: source_ssh_pane,
                runtime_generation: 77,
                prompt: rssh_ssh::SecretPrompt::password("ops"),
                response: response_sender,
            }),
            Some(false)
        );
        let target = manager.pending_apps.first().unwrap();
        assert!(target.ssh_secret_prompts.contains_key(&target_ssh_pane));
        assert!(!target.ssh_secret_prompts.contains_key(&source_ssh_pane));
        assert_eq!(
            target.metrics.connection_state(),
            super::ConnectionState::AwaitingSecret
        );
    }

    #[test]
    fn window_manager_transfers_its_final_tab_and_retires_the_source_window() {
        let source = NativeWindowApp::new(None);
        let mut target = NativeWindowApp::new(None);
        target.app_window_id = rssh_core::WindowId::new(2);

        let mut manager = NativeWindowManager::new_for_test(source);
        manager.pending_apps.push(Box::new(target));
        manager
            .transfer_tab_between_windows(
                rssh_core::WindowId::new(1),
                rssh_core::WindowId::new(2),
                rssh_core::TabId::new(1),
                1,
            )
            .expect("expected final live tab transfer");

        assert!(manager.startup_app.is_none());
        let target = manager
            .pending_apps
            .first()
            .expect("target window remains managed");
        assert_eq!(target.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(target.app_shell.active_tab_id(), rssh_core::TabId::new(2));
    }
