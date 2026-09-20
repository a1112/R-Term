    #[test]
    fn window_app_force_reverse_video_cursor_overrides_wezterm_cursor_bg() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.force_reverse_video_cursor = true
            config.colors = {
              foreground = '#010203',
              cursor_bg = '#070809',
            }

            return config
            "##,
        )
        .expect("expected WezTerm cursor config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"A").unwrap();
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        assert_eq!(
            frame_pixel_at(
                &frame,
                FRAME_WIDTH as usize,
                CELL_WIDTH as usize,
                terminal_origin_y
            ),
            [1, 2, 3, 255]
        );
    }

    #[cfg(feature = "image-gif")]
    #[test]
    fn window_app_advances_inline_gif_frame_between_native_renders() {
        const RED_GREEN_SLOW_GIF_SEQUENCE: &[u8] = b"\x1b[?25l\x1b]1337;File=inline=1;width=1;height=1:R0lGODlhAQABAIEAAP8AAAAAAAAAAAAAACH/C05FVFNDQVBFMi4wAwEAAAAh+QQICgAAACwAAAAAAQABAAAIBAABBAQAIfkECPQBAAAsAAAAAAEAAQCBAP8AAAAAAAAAAAAACAQAAQQEADs=\x07";

        let mut app = NativeWindowApp::new(None);
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.handle_pty_output(RED_GREEN_SLOW_GIF_SEQUENCE).unwrap();

        app.render_framebuffer(&mut frame);
        std::thread::sleep(Duration::from_millis(150));
        app.render_framebuffer(&mut frame);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        assert_eq!(
            frame_pixel_at(&frame, FRAME_WIDTH as usize, 0, terminal_origin_y),
            [0, 255, 0, 255]
        );
    }

    #[test]
    fn window_app_honors_wezterm_enable_kitty_graphics_false() {
        let mut app = NativeWindowApp::new(None);
        let written = Arc::new(Mutex::new(Vec::new()));
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.enable_kitty_graphics = false

            return config
            "#,
        )
        .expect("expected WezTerm enable_kitty_graphics config");
        app.set_config_overrides(overrides);

        app.handle_pty_output(b"\x1b_Ga=q,i=31,t=d,f=24,s=1,v=1;/wAA\x1b\\")
            .unwrap();

        assert!(written.lock().unwrap().is_empty());
        assert!(app.render_snapshot().inline_images().is_empty());
    }

    #[test]
    fn window_app_honors_wezterm_enable_checksum_rectangular_area_true() {
        let mut app = NativeWindowApp::new(None);
        let written = Arc::new(Mutex::new(Vec::new()));
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.enable_checksum_rectangular_area = true

            return config
            "#,
        )
        .expect("expected WezTerm checksum rectangular area config");
        app.set_config_overrides(overrides);

        app.handle_pty_output(b"ABC\x1b[7;1;1;1;1;3*y").unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1bP7!~00c6\x1b\\");
        let snapshot = app.render_snapshot();
        assert!(snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS).starts_with("ABC"));
    }

    #[test]
    fn window_app_honors_wezterm_enable_title_reporting_true() {
        let mut app = NativeWindowApp::new(None);
        let written = Arc::new(Mutex::new(Vec::new()));
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.enable_title_reporting = true

            return config
            "#,
        )
        .expect("expected WezTerm title reporting config");
        app.set_config_overrides(overrides);

        app.handle_pty_output(b"\x1b]0;ops\x07before\x1b[21tafter")
            .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b]lops\x1b\\");
        let snapshot = app.render_snapshot();
        assert!(
            snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS).contains("beforeafter")
        );
    }

    #[test]
    fn window_app_honors_wezterm_enq_answerback() {
        let mut app = NativeWindowApp::new(None);
        let written = Arc::new(Mutex::new(Vec::new()));
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.enq_answerback = 'rssh'

            return config
            "#,
        )
        .expect("expected WezTerm ENQ answerback config");
        app.set_config_overrides(overrides);

        app.handle_pty_output(b"before\x05after").unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"rssh");
        let snapshot = app.render_snapshot();
        assert!(
            snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS).contains("beforeafter")
        );
    }

    #[test]
    fn window_app_hides_scrollback_scrollbar_by_default() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rssh_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();

        assert!(app.scrollback_scrollbar().is_none());
    }

    #[test]
    fn window_app_renders_scrollback_scrollbar_to_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            enable_scroll_bar: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.runtime.resize(rssh_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        app.scroll_viewport_lines(99);
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        assert_eq!(
            frame_pixel_at(
                &frame,
                FRAME_WIDTH as usize,
                FRAME_WIDTH as usize - 1,
                tab_bar_pixel_height() as usize,
            ),
            SCROLLBAR_THUMB_COLOR
        );
    }

    #[test]
    fn window_app_renders_active_pane_scrollbar_with_split_layout() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            enable_scroll_bar: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.runtime.resize(rssh_core::TerminalSize::new(4, 2));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        app.scroll_viewport_lines(99);

        let scrollbar = app
            .scrollback_scrollbar()
            .expect("active pane scrollbar should remain visible for split layout");
        assert_eq!(scrollbar.scrollback_offset, 3);

        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);
        assert_eq!(
            frame_pixel_at(
                &frame,
                FRAME_WIDTH as usize,
                FRAME_WIDTH as usize - 1,
                tab_bar_pixel_height() as usize,
            ),
            SCROLLBAR_THUMB_COLOR
        );
    }

    #[test]
    fn window_app_split_scrollbar_follows_active_pane_runtime() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            enable_scroll_bar: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.runtime.resize(rssh_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"p1a\r\np1b\r\np1c\r\np1d\r\np1e")
            .unwrap();
        app.scroll_viewport_lines(2);
        let pane_one_scrollbar = app
            .scrollback_scrollbar()
            .expect("pane one scrollbar should be visible");

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"p2a\r\np2b\r\np2c\r\np2d\r\np2e\r\np2f\r\np2g")
            .unwrap();
        app.scroll_viewport_lines(3);
        let pane_two_scrollbar = app
            .scrollback_scrollbar()
            .expect("pane two scrollbar should be visible");
        assert_ne!(pane_one_scrollbar, pane_two_scrollbar);

        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rssh_core::PaneId::new(1),
        })
        .unwrap();
        assert_eq!(app.scrollback_scrollbar(), Some(pane_one_scrollbar));

        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rssh_core::PaneId::new(2),
        })
        .unwrap();
        assert_eq!(app.scrollback_scrollbar(), Some(pane_two_scrollbar));
    }

    #[test]
    fn window_app_split_scrollbar_input_only_updates_active_pane() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            enable_scroll_bar: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.runtime.resize(rssh_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"p1a\r\np1b\r\np1c\r\np1d\r\np1e")
            .unwrap();
        app.scroll_viewport_lines(1);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"p2a\r\np2b\r\np2c\r\np2d\r\np2e\r\np2f\r\np2g")
            .unwrap();
        assert_eq!(app.current_scrollback_offset(), 0);
        let inactive_offset = app
            .pane_runtimes
            .get(&rssh_core::PaneId::new(1))
            .expect("pane one runtime should be inactive")
            .ui
            .stable_viewport
            .scrollback_offset(
                app.pane_runtimes
                    .get(&rssh_core::PaneId::new(1))
                    .expect("pane one runtime should be inactive")
                    .runtime
                    .terminal(),
            );
        assert_eq!(inactive_offset, 1);

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(FRAME_WIDTH - 1),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert!(app.current_scrollback_offset() > 0);
        let inactive_runtime = app
            .pane_runtimes
            .get(&rssh_core::PaneId::new(1))
            .expect("pane one runtime should remain inactive");
        assert_eq!(
            inactive_runtime
                .ui
                .stable_viewport
                .scrollback_offset(inactive_runtime.runtime.terminal()),
            inactive_offset
        );
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );
    }

    #[test]
    fn window_app_applies_wezterm_scrollbar_thumb_color_to_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.enable_scroll_bar = true
            config.colors = {
              scrollbar_thumb = '#010203',
            }

            return config
            "##,
        )
        .expect("expected WezTerm scrollbar_thumb config");
        app.set_config_overrides(overrides);
        app.runtime.resize(rssh_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        app.scroll_viewport_lines(99);
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        assert_eq!(
            frame_pixel_at(
                &frame,
                FRAME_WIDTH as usize,
                FRAME_WIDTH as usize - 1,
                tab_bar_pixel_height() as usize,
            ),
            [1, 2, 3, 255]
        );
    }

    #[test]
    fn window_app_applies_configured_min_scroll_bar_height_to_scrollbar() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            enable_scroll_bar: Some(true),
            min_scroll_bar_height: Some(NativeScrollBarHeight::Pixels(12)),
            ..NativeConfigSnapshot::default()
        });
        app.runtime.resize(rssh_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();

        let scrollbar = app.scrollback_scrollbar().expect("scrollbar");

        assert_eq!(
            scrollbar.min_thumb_height,
            Some(RenderScrollbarThumbSize::Pixels(12))
        );
    }

    #[test]
    fn window_app_reports_default_wezterm_min_scroll_bar_height() {
        let mut app = NativeWindowApp::new(None);

        assert_eq!(
            app.native_effective_config().min_scroll_bar_height,
            Some(NativeScrollBarHeight::CellFractionPerMille(500))
        );

        app.set_config_overrides(native_config_snapshot! {
            enable_scroll_bar: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.runtime.resize(rssh_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();

        let scrollbar = app.scrollback_scrollbar().expect("scrollbar");
        assert_eq!(
            scrollbar.min_thumb_height,
            Some(RenderScrollbarThumbSize::CellFractionPerMille(500))
        );
    }

    #[test]
    fn window_app_scrollbar_hit_testing_uses_window_dpi_for_point_min_height() {
        let mut app = NativeWindowApp::new(None);
        app.apply_window_scale_factor(1.5);
        app.set_config_overrides(native_config_snapshot! {
            enable_scroll_bar: Some(true),
            min_scroll_bar_height: Some(NativeScrollBarHeight::Points(72)),
            ..NativeConfigSnapshot::default()
        });
        app.runtime.resize(rssh_core::TerminalSize::new(4, 2));
        let mut output = String::new();
        for line in 0..200 {
            std::fmt::Write::write_fmt(&mut output, format_args!("{line:03}\n")).unwrap();
        }
        app.handle_pty_output(output.as_bytes()).unwrap();

        let y_in_content = 200;
        let content_height = app.frame_height.saturating_sub(app.tab_bar_pixel_height());
        let geometry =
            RenderGeometry::new(app.frame_width, content_height, CELL_WIDTH, CELL_HEIGHT);
        let scrollbar = app.scrollback_scrollbar().expect("scrollbar");
        let expected =
            scrollbar.offset_from_pixel_y_with_dpi(y_in_content, geometry, app.window_dpi);

        assert_ne!(
            expected,
            scrollbar.offset_from_pixel_y(y_in_content, geometry)
        );
        assert_eq!(
            app.scrollbar_offset_from_pixel_y(f64::from(app.terminal_pixel_top() + y_in_content)),
            Some(expected)
        );
    }

    #[test]
    fn native_scroll_bar_height_parses_wezterm_unit_strings() {
        assert_eq!(
            NativeScrollBarHeight::parse("0.5cell"),
            Some(NativeScrollBarHeight::CellFractionPerMille(500))
        );
        assert_eq!(
            NativeScrollBarHeight::parse("25%"),
            Some(NativeScrollBarHeight::Percent(25))
        );
        assert_eq!(
            NativeScrollBarHeight::parse("1pt"),
            Some(NativeScrollBarHeight::Points(1))
        );
        assert_eq!(
            NativeScrollBarHeight::parse("12px"),
            Some(NativeScrollBarHeight::Pixels(12))
        );
        assert_eq!(
            NativeScrollBarHeight::parse("12"),
            Some(NativeScrollBarHeight::Pixels(12))
        );
        assert_eq!(NativeScrollBarHeight::parse("cell"), None);
    }

    #[test]
    fn window_app_clicking_scrollback_scrollbar_jumps_viewport() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            enable_scroll_bar: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.runtime.resize(rssh_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(FRAME_WIDTH - 1),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();

        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.current_scrollback_offset(), 3);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('a'));
        assert!(app.selection.is_none());
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_dragging_scrollback_scrollbar_updates_viewport() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            enable_scroll_bar: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.runtime.resize(rssh_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(FRAME_WIDTH - 1),
            f64::from(FRAME_HEIGHT - 1),
        ))
        .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(FRAME_WIDTH - 1),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();

        assert_eq!(app.current_scrollback_offset(), 3);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('a'));
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );
        assert!(!app.selecting);
    }

    #[test]
    fn window_app_collects_pty_processing_metrics() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"live").unwrap();

        let metrics = app.metrics_snapshot();
        assert_eq!(metrics.pty_chunks, 1);
        assert_eq!(metrics.pty_bytes, 4);
        assert_eq!(metrics.damage_regions, 1);
        assert_eq!(metrics.damaged_cells, 4);
        assert!(metrics.first_pty_byte_ms.is_some());
        assert!(metrics.first_rendered_cell_ms.is_some());
    }

    #[test]
    fn window_app_collects_bell_metrics() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x07live\x07").unwrap();

        let metrics = app.metrics_snapshot();
        assert_eq!(metrics.bells, 2);
        assert!(app.metrics_report().contains("bells=2"));
    }

    #[test]
    fn window_app_dispatches_bells_from_active_and_inactive_panes() {
        let bells = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&bells);
        let mut app = NativeWindowApp::new(None);
        app.bell_handler = Box::new(move |bell| {
            recorded.lock().unwrap().push(*bell);
            true
        });
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        let active_pane = app.app_shell.active_pane_id();

        app.handle_pty_output(b"\x07active\x07").unwrap();
        app.handle_pane_pty_output(rssh_core::PaneId::new(1), b"\x07inactive")
            .unwrap();

        assert_eq!(
            bells.lock().unwrap().as_slice(),
            [
                NativeWindowBell {
                    window_id: rssh_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowBell {
                    window_id: rssh_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowBell {
                    window_id: rssh_core::WindowId::new(1),
                    pane: rssh_core::PaneId::new(1),
                },
            ]
        );
    }

    #[test]
    fn window_app_rings_audible_bell_by_default() {
        let audible_bells = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&audible_bells);
        let mut app = NativeWindowApp::new(None);
        app.audible_bell_handler = Box::new(move |bell| {
            recorded.lock().unwrap().push(*bell);
            true
        });

        app.handle_pty_output(b"\x07live").unwrap();

        assert_eq!(
            audible_bells.lock().unwrap().as_slice(),
            [NativeWindowBell {
                window_id: rssh_core::WindowId::new(1),
                pane: app.app_shell.active_pane_id(),
            }]
        );
    }

    #[test]
    fn window_app_disabled_audible_bell_preserves_bell_event() {
        let audible_bells = Arc::new(Mutex::new(Vec::new()));
        let recorded_audible = Arc::clone(&audible_bells);
        let bells = Arc::new(Mutex::new(Vec::new()));
        let recorded_bells = Arc::clone(&bells);
        let mut app = NativeWindowApp::new(None);
        app.audible_bell_handler = Box::new(move |bell| {
            recorded_audible.lock().unwrap().push(*bell);
            true
        });
        app.bell_handler = Box::new(move |bell| {
            recorded_bells.lock().unwrap().push(*bell);
            true
        });
        app.set_config_overrides(native_config_snapshot! {
            audible_bell: Some(NativeAudibleBell::Disabled),
            ..NativeConfigSnapshot::default()
        });

        app.handle_pty_output(b"\x07live").unwrap();

        assert!(audible_bells.lock().unwrap().is_empty());
        assert_eq!(
            bells.lock().unwrap().as_slice(),
            [NativeWindowBell {
                window_id: rssh_core::WindowId::new(1),
                pane: app.app_shell.active_pane_id(),
            }]
        );
        assert_eq!(app.metrics_snapshot().bells, 1);
    }

    #[test]
    fn window_app_parses_static_wezterm_bell_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('bell', function(window, pane)
              window:set_right_status('BELL-LUA')
            end)
            "#,
        )
        .expect("expected static WezTerm bell event status setter");
        app.set_config_overrides(overrides);

        app.handle_pty_output(b"\x07").unwrap();

        assert_eq!(app.right_status, "BELL-LUA");
    }

    #[test]
    fn window_app_default_visual_bell_does_not_tint_background() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b[31mA\x07").unwrap();

        let snapshot = app.render_snapshot();
        let cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("visible cell");
        assert_eq!(cell.ch, 'A');
        assert_eq!(cell.foreground, Color::Indexed(1));
        assert_eq!(cell.background, Color::Default);
        assert_eq!(
            app.native_effective_config().visual_bell,
            NativeVisualBell::default()
        );
    }

    #[test]
    fn window_app_configured_visual_bell_tints_background_from_foreground() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            visual_bell: Some(NativeVisualBell {
                fade_in_duration_ms: 0,
                fade_out_duration_ms: 150,
                fade_in_function: NativeEasingFunction::Ease,
                fade_out_function: NativeEasingFunction::Ease,
                target: NativeVisualBellTarget::BackgroundColor,
            }),
            ..NativeConfigSnapshot::default()
        });

        app.handle_pty_output(b"\x1b[31mA\x07").unwrap();

        let snapshot = app.render_snapshot();
        let cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("visible cell");
        assert_eq!(cell.ch, 'A');
        assert_eq!(cell.foreground, Color::Indexed(1));
        assert_eq!(cell.background, Color::Indexed(1));
        let blank_cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 1).expect("visual bell background");
        assert_eq!(blank_cell.ch, ' ');
        assert_eq!(blank_cell.background, Color::Indexed(1));
        assert_eq!(
            app.native_effective_config().visual_bell,
            NativeVisualBell {
                fade_in_duration_ms: 0,
                fade_out_duration_ms: 150,
                fade_in_function: NativeEasingFunction::Ease,
                fade_out_function: NativeEasingFunction::Ease,
                target: NativeVisualBellTarget::BackgroundColor,
            }
        );
    }

    #[test]
    fn window_app_visual_bell_uses_default_text_foreground_for_default_cells() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            visual_bell: Some(NativeVisualBell {
                fade_in_duration_ms: 0,
                fade_out_duration_ms: 150,
                fade_in_function: NativeEasingFunction::Ease,
                fade_out_function: NativeEasingFunction::Ease,
                target: NativeVisualBellTarget::BackgroundColor,
            }),
            ..NativeConfigSnapshot::default()
        });

        app.handle_pty_output(b"A\x07").unwrap();

        let snapshot = app.render_snapshot();
        let cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("visible cell");
        let blank_cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 1).expect("visual bell background");
        assert_eq!(cell.foreground, Color::Default);
        assert_eq!(cell.background, Color::Rgb(229, 229, 229));
        assert_eq!(blank_cell.background, Color::Rgb(229, 229, 229));
    }

    #[test]
    fn window_app_visual_bell_uses_default_text_foreground_for_empty_pane() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            visual_bell: Some(NativeVisualBell {
                fade_in_duration_ms: 0,
                fade_out_duration_ms: 150,
                fade_in_function: NativeEasingFunction::Ease,
                fade_out_function: NativeEasingFunction::Ease,
                target: NativeVisualBellTarget::BackgroundColor,
            }),
            ..NativeConfigSnapshot::default()
        });

        app.handle_pty_output(b"\x07").unwrap();

        let snapshot = app.render_snapshot();
        let blank_cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("visual bell background");
        assert_eq!(blank_cell.background, Color::Rgb(229, 229, 229));
    }

    #[test]
    fn window_app_visual_bell_color_override_tints_background() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            visual_bell: Some(NativeVisualBell {
                fade_in_duration_ms: 0,
                fade_out_duration_ms: 150,
                fade_in_function: NativeEasingFunction::Ease,
                fade_out_function: NativeEasingFunction::Ease,
                target: NativeVisualBellTarget::BackgroundColor,
            }),
            visual_bell_color: Some(Color::Rgb(1, 2, 3)),
            ..NativeConfigSnapshot::default()
        });

        app.handle_pty_output(b"\x1b[31mA\x07").unwrap();

        let snapshot = app.render_snapshot();
        let cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("visible cell");
        let blank_cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 1).expect("visual bell background");
        assert_eq!(cell.foreground, Color::Indexed(1));
        assert_eq!(cell.background, Color::Rgb(1, 2, 3));
        assert_eq!(blank_cell.background, Color::Rgb(1, 2, 3));
        assert_eq!(
            app.native_effective_config().visual_bell_color,
            Some(Color::Rgb(1, 2, 3))
        );
    }

    #[test]
    fn window_app_visual_bell_linear_fade_out_blends_background_color() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            visual_bell: Some(NativeVisualBell {
                fade_in_duration_ms: 0,
                fade_out_duration_ms: 100_000,
                fade_in_function: NativeEasingFunction::Linear,
                fade_out_function: NativeEasingFunction::Linear,
                target: NativeVisualBellTarget::BackgroundColor,
            }),
            visual_bell_color: Some(Color::Rgb(100, 100, 100)),
            ..NativeConfigSnapshot::default()
        });

        app.handle_pty_output(b"A\x07").unwrap();
        let active_pane_id = app.active_pane_id();
        let now = Instant::now();
        app.visual_bell_test_now = Some(now);
        app.visual_bell_started_at.insert(
            active_pane_id,
            now
                .checked_sub(Duration::from_millis(50_000))
                .unwrap(),
        );

        let snapshot = app.render_snapshot();
        let cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("visible cell");
        let blank_cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 1).expect("visual bell background");
        assert_eq!(cell.background, Color::Rgb(56, 56, 56));
        assert_eq!(blank_cell.background, Color::Rgb(56, 56, 56));
    }

    #[test]
    fn window_app_visual_bell_cubic_bezier_solves_x_axis_progress() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            visual_bell: Some(NativeVisualBell {
                fade_in_duration_ms: 0,
                fade_out_duration_ms: 100_000,
                fade_in_function: NativeEasingFunction::Linear,
                fade_out_function: NativeEasingFunction::CubicBezier(NativeCubicBezier {
                    x1_per_mille: 0,
                    y1_per_mille: 0,
                    x2_per_mille: 0,
                    y2_per_mille: 1_000,
                }),
                target: NativeVisualBellTarget::BackgroundColor,
            }),
            visual_bell_color: Some(Color::Rgb(100, 100, 100)),
            ..NativeConfigSnapshot::default()
        });

        app.handle_pty_output(b"A\x07").unwrap();
        let active_pane_id = app.active_pane_id();
        let now = Instant::now();
        app.visual_bell_test_now = Some(now);
        app.visual_bell_started_at.insert(
            active_pane_id,
            now
                .checked_sub(Duration::from_millis(12_500))
                .unwrap(),
        );

        let snapshot = app.render_snapshot();
        let cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("visible cell");
        let blank_cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 1).expect("visual bell background");
        assert_eq!(cell.background, Color::Rgb(56, 56, 56));
        assert_eq!(blank_cell.background, Color::Rgb(56, 56, 56));
    }

    #[test]
    fn window_app_cursor_visual_bell_uses_foreground_without_tinting_background() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            visual_bell: Some(NativeVisualBell {
                fade_in_duration_ms: 0,
                fade_out_duration_ms: 150,
                fade_in_function: NativeEasingFunction::Ease,
                fade_out_function: NativeEasingFunction::Ease,
                target: NativeVisualBellTarget::CursorColor,
            }),
            ..NativeConfigSnapshot::default()
        });

        app.handle_pty_output(b"\x1b[31mA\x07").unwrap();
        app.visual_bell_test_now = app.visual_bell_started_at.get(&app.active_pane_id()).copied();

        let snapshot = app.render_snapshot();
        let cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("visible cell");
        assert_eq!(cell.foreground, Color::Indexed(1));
        assert_eq!(cell.background, Color::Default);
        assert_eq!(snapshot.cursor_color(), Some(Color::Indexed(1)));
    }

    #[test]
    fn window_app_cursor_visual_bell_fades_from_force_reverse_cursor_color() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            force_reverse_video_cursor: Some(true),
            visual_bell: Some(NativeVisualBell {
                fade_in_duration_ms: 0,
                fade_out_duration_ms: 100_000,
                fade_in_function: NativeEasingFunction::Linear,
                fade_out_function: NativeEasingFunction::Linear,
                target: NativeVisualBellTarget::CursorColor,
            }),
            visual_bell_color: Some(Color::Rgb(100, 100, 100)),
            ..NativeConfigSnapshot::default()
        });

        app.handle_pty_output(b"\x1b[31mA\x08\x07").unwrap();
        let active_pane_id = app.active_pane_id();
        let now = Instant::now();
        app.visual_bell_test_now = Some(now);
        app.visual_bell_started_at.insert(
            active_pane_id,
            now
                .checked_sub(Duration::from_millis(50_000))
                .unwrap(),
        );

        let snapshot = app.render_snapshot();
        let cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("visible cell");
        assert_eq!(cell.foreground, Color::Indexed(1));
        assert_eq!(snapshot.cursor_color(), Some(Color::Rgb(153, 75, 75)));
    }

    #[test]
    fn pty_linkage_matcher_spans_chunks_and_rejects_a_different_terminal_nonce() {
        let mut metrics = WindowMetrics::new();
        metrics.pty_linkage_enabled = true;
        metrics.record_active_pty_content(b"noiseRSSH-LI");
        metrics.record_active_pty_content(b"NK-BEGIN|nonce-one|office \xe4\xb8\xad|RSSH-LINK-");
        metrics.record_active_pty_content(b"ENDtail");
        assert_eq!(
            metrics.pty_linkage_payload.as_deref(),
            Some(b"nonce-one|office \xe4\xb8\xad".as_slice())
        );

        let mut matching = Terminal::new(rssh_core::TerminalSize::new(80, 1));
        matching.feed(b"RSSH-LINK-BEGIN|nonce-one|office \xe4\xb8\xad|RSSH-LINK-END");
        metrics.record_terminal_linkage_snapshot(&TerminalRenderSnapshot::from_terminal(&matching));
        assert!(metrics.terminal_linkage_nonce_found);

        let mut different = Terminal::new(rssh_core::TerminalSize::new(80, 1));
        different.feed(b"RSSH-LINK-BEGIN|nonce-two|office \xe4\xb8\xad|RSSH-LINK-END");
        metrics
            .record_terminal_linkage_snapshot(&TerminalRenderSnapshot::from_terminal(&different));
        assert!(!metrics.terminal_linkage_nonce_found);
    }

    #[test]
    fn window_metrics_json_report_is_machine_readable() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"live").unwrap();

        let json = app.metrics_json_report().unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["runtime_api"], "v2-runtime-hub");
        assert_eq!(value["runtime_live_threads"], 0);
        assert_eq!(value["pty_chunks"], 1);
        assert_eq!(value["pty_bytes"], 4);
        assert_eq!(value["damage_regions"], 1);
        assert_eq!(value["damaged_cells"], 4);
        assert_eq!(value["full_render_frames"], 0);
        assert_eq!(value["dirty_render_frames"], 0);
        assert_eq!(value["gpu_backend"], "uninitialized");
        assert_eq!(value["gpu_adapter_name"], "uninitialized");
        assert_eq!(value["gpu_adapter_type"], "unknown");
        assert_eq!(value["gpu_software_adapter"], false);
        assert!(value["gpu_surface_format"].is_null());
        assert!(value["gpu_present_mode"].is_null());
        assert!(value["gpu_surface_width"].is_null());
        assert!(value["gpu_surface_height"].is_null());
        assert_eq!(value["gpu_rendered_frames"], 0);
        assert_eq!(value["gpu_presented_frames"], 0);
        assert_eq!(value["gpu_uncaptured_errors"], 0);
        assert_eq!(value["gpu_device_losses"], 0);
        assert_eq!(value["text_backend"], "bitmap-emergency");
        assert!(value["first_pty_byte_ms"].is_number());
        assert!(value["first_rendered_cell_ms"].is_number());
    }

    const LEGACY_WINDOW_METRICS_JSON_FIELDS: &[&str] = &[
        "runtime_api",
        "runtime_live_threads",
        "last_exit_code",
        "first_pty_byte_ms",
        "first_rendered_cell_ms",
        "pty_chunks",
        "pty_bytes",
        "pty_linkage_found",
        "pty_linkage_digest",
        "terminal_linkage_nonce_found",
        "terminal_snapshot_content_digest",
        "pty_chunk_process_p95_us",
        "damage_regions",
        "damaged_cells",
        "snapshot_damage_updates",
        "snapshot_rebuilds",
        "render_frames",
        "full_render_frames",
        "dirty_render_frames",
        "render_frame_p95_us",
        "gpu_backend",
        "gpu_adapter_name",
        "gpu_adapter_vendor_id",
        "gpu_adapter_device_id",
        "gpu_adapter_type",
        "gpu_software_adapter",
        "gpu_surface_format",
        "gpu_present_mode",
        "gpu_surface_width",
        "gpu_surface_height",
        "gpu_rendered_frames",
        "gpu_presented_frames",
        "gpu_surface_reconfigurations",
        "gpu_surface_recreations",
        "gpu_surface_timeouts",
        "gpu_surface_occlusions",
        "gpu_surface_validation_errors",
        "gpu_compatibility_frame_uploads",
        "gpu_uncaptured_errors",
        "gpu_device_losses",
        "gpu_device_recoveries",
        "gpu_device_recovery_failures",
        "gpu_abandoned_lost_surfaces",
        "text_backend",
        "gpu_text_prepared_glyphs",
        "gpu_text_mask_glyphs",
        "gpu_text_color_glyphs",
        "gpu_text_block_glyphs",
        "gpu_text_content_digest",
        "gpu_text_rendered_frames",
        "input_writes",
        "input_bytes",
        "input_write_p95_us",
        "bells",
    ];

    const STARTUP_WINDOW_METRICS_JSON_FIELDS: &[&str] = &[
        "process_to_first_present_ms",
        "config_duration_ms",
        "gpu_duration_ms",
        "ssh_duration_ms",
        "first_frame_private_bytes",
        "final_renderer",
        "connection_state",
    ];

    #[test]
    fn window_metrics_json_preserves_legacy_and_startup_fields() {
        let app = NativeWindowApp::new(None);
        let json = app.metrics_json_report().unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let actual = value
            .as_object()
            .expect("window metrics JSON object")
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        let expected = LEGACY_WINDOW_METRICS_JSON_FIELDS
            .iter()
            .chain(STARTUP_WINDOW_METRICS_JSON_FIELDS)
            .copied()
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(actual, expected);
        assert_eq!(value["connection_state"], "not_started");
    }

    #[test]
    fn window_app_records_input_metrics_on_worker_completion() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        super::start_pane_input_queue(&mut app.writer, &mut app.writer_thread, None).unwrap();

        app.write_pty_bytes(b"abc").unwrap();

        assert_eq!(app.metrics_snapshot().input_writes, 0);
        wait_for_test_condition("input write metrics payload", || {
            written.lock().unwrap().as_slice() == b"abc"
        });
        app.handle_pane_input_write_completed(3, Duration::from_micros(25));

        let metrics = app.metrics_snapshot();
        assert_eq!(written.lock().unwrap().as_slice(), b"abc");
        assert_eq!(metrics.input_writes, 1);
        assert_eq!(metrics.input_bytes, 3);
    }

    #[test]
    fn windows_integrated_title_bar_requests_native_shadow_and_round_corners() {
        let policy = super::native_window_chrome_policy_for_platform(
            "windows",
            NativeWindowDecorations {
                title: false,
                resize: true,
                integrated_buttons: true,
                macos_force_disable_shadow: false,
                macos_force_enable_shadow: false,
                macos_force_square_corners: false,
                macos_use_background_color_as_titlebar_color: false,
            },
        );

        assert!(policy.undecorated_shadow);
        assert!(policy.rounded_corners);
        assert_eq!(
            super::native_window_chrome_policy_for_platform(
                "windows",
                NativeWindowDecorations {
                    title: true,
                    resize: true,
                    integrated_buttons: false,
                    macos_force_disable_shadow: false,
                    macos_force_enable_shadow: false,
                    macos_force_square_corners: false,
                    macos_use_background_color_as_titlebar_color: false,
                },
            ),
            super::NativeWindowChromePolicy::default()
        );
    }

    #[test]
    fn pane_input_queue_returns_before_blocking_writer() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(DelayedWriter {
            delay: Duration::from_millis(100),
            written: Arc::clone(&written),
        }));
        super::start_pane_input_queue(&mut app.writer, &mut app.writer_thread, None).unwrap();

        let started = Instant::now();
        app.write_pty_bytes(b"responsive").unwrap();
        let dispatch_elapsed = started.elapsed();

        assert!(
            dispatch_elapsed < Duration::from_millis(40),
            "PTY input dispatch blocked the window event loop for {dispatch_elapsed:?}"
        );
        wait_for_test_condition("delayed PTY input write", || {
            written.lock().unwrap().as_slice() == b"responsive"
        });
    }

    #[test]
    fn pane_input_queue_preserves_fifo_order() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        super::start_pane_input_queue(&mut app.writer, &mut app.writer_thread, None).unwrap();

        for chunk in [b"one".as_slice(), b"-two", b"-three"] {
            app.write_pty_bytes(chunk).unwrap();
        }

        wait_for_test_condition("ordered PTY input writes", || {
            written.lock().unwrap().as_slice() == b"one-two-three"
        });
    }

    #[test]
    fn window_app_uses_configured_startup_command() {
        let app = NativeWindowApp::new_with_command(
            None,
            rssh_pty::PtyCommand::new("powershell").with_args([
                "-NoProfile",
                "-Command",
                "Write-Output window-smoke",
            ]),
        );

        assert_eq!(app.startup_command().program(), "powershell");
        assert_eq!(
            app.startup_command().args(),
            ["-NoProfile", "-Command", "Write-Output window-smoke"]
        );
    }

    #[test]
    fn window_app_applies_default_prog_to_initial_default_shell_before_spawn() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rssh_pty::PtyCommand::default_shell().with_cwd("/tmp/project"),
        );

        app.set_config_overrides(native_config_snapshot! {
            default_prog: Some(vec!["nu".to_owned(), "--login".to_owned()]),
            ..NativeConfigSnapshot::default()
        });

        assert_eq!(app.startup_command().program(), "nu");
        assert_eq!(app.startup_command().args(), ["--login"]);
        assert_eq!(
            app.startup_command().cwd(),
            Some(std::path::Path::new("/tmp/project"))
        );
        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);
        assert_eq!(launch.cwd(), Some("/tmp/project"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_default_ssh_auth_sock_for_startup_command() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.default_ssh_auth_sock = '/tmp/wezterm-agent.sock'

            return config
            "#,
        )
        .expect("expected WezTerm SSH agent config");
        let mut app =
            NativeWindowApp::new_with_command(None, rssh_pty::PtyCommand::default_shell());

        app.set_config_overrides(overrides);

        assert_eq!(
            app.startup_command().env_value("SSH_AUTH_SOCK"),
            Some("/tmp/wezterm-agent.sock")
        );
    }

    #[test]
    fn window_app_respects_wezterm_lua_config_disabled_mux_ssh_agent_for_startup_command() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.default_ssh_auth_sock = '/tmp/wezterm-agent.sock'
            config.mux_enable_ssh_agent = false

            return config
            "#,
        )
        .expect("expected WezTerm SSH agent config");
        let mut app =
            NativeWindowApp::new_with_command(None, rssh_pty::PtyCommand::default_shell());

        app.set_config_overrides(overrides);

        assert_eq!(app.startup_command().env_value("SSH_AUTH_SOCK"), None);
    }

    #[test]
    fn window_app_clears_startup_ssh_auth_sock_when_mux_ssh_agent_is_disabled() {
        let mut app =
            NativeWindowApp::new_with_command(None, rssh_pty::PtyCommand::default_shell());

        app.set_config_overrides(native_config_snapshot! {
            default_ssh_auth_sock: Some("/tmp/wezterm-agent.sock".to_owned()),
            ..NativeConfigSnapshot::default()
        });
        app.set_config_overrides(native_config_snapshot! {
            default_ssh_auth_sock: Some("/tmp/wezterm-agent.sock".to_owned()),
            mux_enable_ssh_agent: Some(false),
            ..NativeConfigSnapshot::default()
        });

        assert_eq!(app.startup_command().env_value("SSH_AUTH_SOCK"), None);
    }

    #[test]
    fn window_app_applies_wezterm_default_ssh_auth_sock_to_spawned_window_command() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.default_ssh_auth_sock = '/tmp/wezterm-agent.sock'

            return config
            "#,
        )
        .expect("expected WezTerm SSH agent config");
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(overrides);

        app.dispatch_app_action(AppAction::SpawnWindow {
            launch: Some(PaneLaunch::local("pwsh").with_args(["-NoLogo"])),
        })
        .unwrap();
        let detached_app = app
            .take_next_pending_window_app()
            .expect("expected pending spawned window");

        assert_eq!(
            detached_app.startup_command().env_value("SSH_AUTH_SOCK"),
            Some("/tmp/wezterm-agent.sock")
        );
    }

    #[test]
    fn window_app_applies_default_wezterm_mux_env_remove_to_spawned_window_command() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.set_environment_variables = {
              KEEP_ME = '1',
              SSH_CLIENT = '192.0.2.1 12345 22',
              SSH_CONNECTION = '192.0.2.1 12345 198.51.100.2 22',
            }

            return config
            "#,
        )
        .expect("expected WezTerm environment config");
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(overrides);

        app.dispatch_app_action(AppAction::SpawnWindow {
            launch: Some(PaneLaunch::local("pwsh").with_args(["-NoLogo"])),
        })
        .unwrap();
        let detached_app = app
            .take_next_pending_window_app()
            .expect("expected pending spawned window");

        assert_eq!(
            detached_app.startup_command().env_value("KEEP_ME"),
            Some("1")
        );
        assert_eq!(detached_app.startup_command().env_value("SSH_CLIENT"), None);
        assert_eq!(
            detached_app.startup_command().env_value("SSH_CONNECTION"),
            None
        );
    }

    #[test]
    fn window_app_parses_wezterm_mux_env_remove_override_for_spawned_window_command() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.mux_env_remove = { 'REMOVE_ME' }
            config.set_environment_variables = {
              REMOVE_ME = 'gone',
              SSH_CLIENT = '192.0.2.1 12345 22',
            }

            return config
            "#,
        )
        .expect("expected WezTerm mux_env_remove config");
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(overrides);

        app.dispatch_app_action(AppAction::SpawnWindow {
            launch: Some(PaneLaunch::local("pwsh").with_args(["-NoLogo"])),
        })
        .unwrap();
        let detached_app = app
            .take_next_pending_window_app()
            .expect("expected pending spawned window");

        assert_eq!(detached_app.startup_command().env_value("REMOVE_ME"), None);
        assert_eq!(
            detached_app.startup_command().env_value("SSH_CLIENT"),
            Some("192.0.2.1 12345 22")
        );
    }

    #[test]
    fn window_app_uses_configured_startup_workspace() {
        let app = NativeWindowApp::new_with_workspace(
            None,
            rssh_pty::PtyCommand::default_shell(),
            Some("ops"),
        );

        assert_eq!(app.app_shell.active_workspace().name(), "ops");
        assert_eq!(app.active_workspace_id(), rssh_core::WorkspaceId::new(1));
    }

    #[test]
    fn window_app_applies_default_workspace_to_initial_default_workspace_before_spawn() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            default_workspace: Some("ops".to_owned()),
            ..NativeConfigSnapshot::default()
        });

        assert_eq!(app.app_shell.active_workspace().name(), "ops");
        assert_eq!(app.native_effective_config().default_workspace, "ops");
    }

    #[test]
    fn window_app_preserves_explicit_default_startup_workspace_over_default_workspace() {
        let mut app = NativeWindowApp::new_with_workspace(
            None,
            rssh_pty::PtyCommand::default_shell(),
            Some("default"),
        );

        app.set_config_overrides(native_config_snapshot! {
            default_workspace: Some("ops".to_owned()),
            ..NativeConfigSnapshot::default()
        });

        assert_eq!(app.app_shell.active_workspace().name(), "default");
        assert_eq!(app.native_effective_config().default_workspace, "ops");
    }

    #[test]
    fn window_app_reports_configured_default_domain_in_effective_config() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            default_domain: Some("ssh-prod".to_owned()),
            ..NativeConfigSnapshot::default()
        });

        assert_eq!(app.native_effective_config().default_domain, "ssh-prod");
    }

    #[test]
    fn window_app_reports_configured_automatically_reload_config_in_effective_config() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            automatically_reload_config: Some(false),
            ..NativeConfigSnapshot::default()
        });

        assert!(!app.native_effective_config().automatically_reload_config);
    }

    #[test]
    fn window_app_reports_configured_use_resize_increments_in_effective_config() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            use_resize_increments: Some(true),
            ..NativeConfigSnapshot::default()
        });

        assert!(app.native_effective_config().use_resize_increments);
    }

    #[test]
    fn window_app_uses_cell_geometry_for_resize_increments_when_configured() {
        let mut app = NativeWindowApp::new(None);

        assert_eq!(app.window_resize_increments(), None);

        app.set_config_overrides(native_config_snapshot! {
            use_resize_increments: Some(true),
            cell_width: Some(NativeCellWidth::from_per_mille(1_500)),
            line_height: Some(NativeLineHeight::from_per_mille(1_250)),
            ..NativeConfigSnapshot::default()
        });
        let configured_increment_cell_size =
            PhysicalSize::new((CELL_WIDTH * 3).div_ceil(2), (CELL_HEIGHT * 5).div_ceil(4));
        let expected_configured_increments =
            native_window_resize_increments_supported().then_some(configured_increment_cell_size);

        assert_eq!(
            app.window_resize_increment_cell_size(),
            configured_increment_cell_size
        );
        assert_eq!(
            app.window_resize_increments(),
            expected_configured_increments
        );

        app.enter_command_palette_mode();
        app.command_palette_execute(WindowCommand::IncreaseFontSize);
        let scaled_increment_cell_size = PhysicalSize::new(app.cell_width(), app.cell_height());
        let expected_scaled_increments =
            native_window_resize_increments_supported().then_some(scaled_increment_cell_size);

        assert_eq!(
            app.window_resize_increment_cell_size(),
            scaled_increment_cell_size
        );
        assert_eq!(app.window_resize_increments(), expected_scaled_increments);
    }

    #[test]
    fn fullscreen_strategy_selects_simple_macos_fullscreen_when_native_mode_is_disabled() {
        let request = super::native_fullscreen_request(
            true,
            false,
            false,
            super::NativeFullscreenPlatform::Macos,
        );

        assert_eq!(request, super::NativeFullscreenRequest::MacosSimple);
    }

    #[test]
    fn fullscreen_strategy_selects_native_macos_fullscreen_when_native_mode_is_enabled() {
        let request = super::native_fullscreen_request(
            true,
            true,
            true,
            super::NativeFullscreenPlatform::Macos,
        );

        assert_eq!(request, super::NativeFullscreenRequest::Borderless);
    }

    #[test]
    fn fullscreen_strategy_keeps_borderless_fullscreen_on_non_macos_platforms() {
        let fullscreen_request = super::native_fullscreen_request(
            true,
            false,
            true,
            super::NativeFullscreenPlatform::Other,
        );
        let windowed_request = super::native_fullscreen_request(
            false,
            false,
            true,
            super::NativeFullscreenPlatform::Macos,
        );

        assert_eq!(
            fullscreen_request,
            super::NativeFullscreenRequest::Borderless
        );
        assert_eq!(windowed_request, super::NativeFullscreenRequest::Windowed);
    }

    #[test]
    fn fullscreen_strategy_selects_simple_macos_fullscreen_behind_notch_when_enabled() {
        let request = super::native_fullscreen_request(
            true,
            false,
            true,
            super::NativeFullscreenPlatform::Macos,
        );

        assert_eq!(
            request,
            super::NativeFullscreenRequest::MacosSimpleExtendBehindNotch
        );
    }

    #[test]
    fn window_app_reports_configured_debug_key_events_in_effective_config() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            debug_key_events: Some(true),
            ..NativeConfigSnapshot::default()
        });

        assert!(app.native_effective_config().debug_key_events);
    }

    #[test]
    fn window_app_logs_key_events_when_debug_key_events_is_enabled() {
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
        app.handle_keyboard_input_event(
            &Key::Character("A".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            Some("A"),
            ElementState::Released,
            KittyKeyEventKind::Release,
        )
        .unwrap();

        let logs = app.debug_key_event_logs_for_test();
        assert_eq!(logs.len(), 2);
        assert!(logs[0].contains("INFO key_event"));
        assert!(logs[0].contains("key: Character(\"A\")"));
        assert!(logs[0].contains("physical_key: Code(KeyA)"));
        assert!(logs[0].contains("modifiers: "));
        assert!(logs[0].contains("state: Pressed"));
        assert!(logs[0].contains("kind: Press"));
        assert!(logs[0].contains("text: Some(\"A\")"));
        assert!(logs[1].contains("state: Released"));
        assert!(logs[1].contains("kind: Release"));
    }

    #[test]
    fn window_app_suppresses_key_event_logs_by_default() {
        let mut app = NativeWindowApp::new(None);

        app.handle_keyboard_input_event(
            &Key::Character("a".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            Some("a"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(app.debug_key_event_logs_for_test().is_empty());
    }

    #[test]
    fn window_app_reports_configured_log_unknown_escape_sequences_in_effective_config() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            log_unknown_escape_sequences: Some(true),
            ..NativeConfigSnapshot::default()
        });

        assert!(app.native_effective_config().log_unknown_escape_sequences);
    }

    #[test]
    fn window_app_reports_default_warn_about_missing_glyphs_in_effective_config() {
        let app = NativeWindowApp::new(None);

        assert!(app.native_effective_config().warn_about_missing_glyphs);
    }

    #[test]
    fn window_app_reports_configured_warn_about_missing_glyphs_in_effective_config() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            warn_about_missing_glyphs: Some(false),
            ..NativeConfigSnapshot::default()
        });

        assert!(!app.native_effective_config().warn_about_missing_glyphs);
    }

    #[test]
    fn window_app_warns_about_missing_glyphs_by_default() {
        let mut app = NativeWindowApp::new(None);
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        app.handle_pty_output("ok 中中".as_bytes()).unwrap();
        app.render_framebuffer(&mut frame);
        app.render_framebuffer(&mut frame);

        assert_eq!(
            app.missing_glyph_warnings_for_test(),
            ["CONFIG ERROR missing glyph for codepoint U+4E2D ('中')"]
        );
    }

    #[test]
    fn window_app_suppresses_missing_glyph_warnings_when_configured() {
        let mut app = NativeWindowApp::new(None);
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.set_config_overrides(native_config_snapshot! {
            warn_about_missing_glyphs: Some(false),
            ..NativeConfigSnapshot::default()
        });

        app.handle_pty_output("ok 中".as_bytes()).unwrap();
        app.render_framebuffer(&mut frame);

        assert!(app.missing_glyph_warnings_for_test().is_empty());
    }

    #[test]
    fn window_app_uses_configured_initial_window_position() {
        let position = crate::cli::WindowPosition {
            origin: crate::cli::WindowPositionOrigin::Screen,
            x: 10,
            y: 20,
        };
        let app = NativeWindowApp::new_with_window_position(
            None,
            rssh_pty::PtyCommand::default_shell(),
            Some(position.clone()),
        );

        assert_eq!(app.initial_window_position(), Some(position));
    }

    #[test]
    fn main_monitor_window_position_adds_primary_monitor_origin() {
        let position = crate::cli::WindowPosition {
            origin: crate::cli::WindowPositionOrigin::Main,
            x: 10,
            y: 20,
        };

        assert_eq!(
            super::resolve_initial_window_position(
                &position,
                Some(PhysicalPosition::new(100, 200)),
                None,
                &[]
            ),
            Some(PhysicalPosition::new(110, 220))
        );
    }

    #[test]
    fn active_monitor_window_position_adds_active_monitor_origin() {
        let position = crate::cli::WindowPosition {
            origin: crate::cli::WindowPositionOrigin::Active,
            x: 10,
            y: 20,
        };

        assert_eq!(
            super::resolve_initial_window_position(
                &position,
                None,
                Some(PhysicalPosition::new(300, 400)),
                &[]
            ),
            Some(PhysicalPosition::new(310, 420))
        );
    }

    #[test]
    fn named_monitor_window_position_adds_matching_monitor_origin() {
        let position = crate::cli::WindowPosition {
            origin: crate::cli::WindowPositionOrigin::Monitor("HDMI-1".to_owned()),
            x: 10,
            y: 20,
        };
        let monitors = [super::NativeMonitorPosition {
            name: Some("HDMI-1".to_owned()),
            position: PhysicalPosition::new(300, 400),
        }];

        assert_eq!(
            super::resolve_initial_window_position(&position, None, None, &monitors),
            Some(PhysicalPosition::new(310, 420))
        );
    }

    #[test]
    fn window_app_uses_configured_initial_window_class() {
        let app = NativeWindowApp::new_with_window_class(
            None,
            rssh_pty::PtyCommand::default_shell(),
            Some("org.example.RSsh".to_owned()),
        );

        assert_eq!(app.initial_window_class(), Some("org.example.RSsh"));
    }

    #[test]
    fn window_app_spawned_window_inherits_configured_initial_window_class() {
        let mut app = NativeWindowApp::new_with_window_class(
            None,
            rssh_pty::PtyCommand::default_shell(),
            Some("org.example.RSsh".to_owned()),
        );

        app.dispatch_app_action(AppAction::SpawnWindow { launch: None })
            .unwrap();
        let pending_app = app
            .take_next_pending_window_app()
            .expect("spawn window should create pending native window");

        assert_eq!(pending_app.initial_window_class(), Some("org.example.RSsh"));
    }

    #[test]
    fn window_app_starts_with_default_shell_state() {
        let app = NativeWindowApp::new_with_command(
            None,
            rssh_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        assert_eq!(app.active_workspace_id(), rssh_core::WorkspaceId::new(1));
        assert_eq!(app.active_tab_id(), rssh_core::TabId::new(1));
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(1));
        assert_eq!(
            app.effective_window_title(),
            "R-SSH [workspace:1 tab:1 pane:1]"
        );
    }

    #[test]
    fn window_app_dispatches_new_tab_action() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rssh_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        let action = app
            .app_shell_action_for_key(
                &Key::Character("T".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT,
            )
            .expect("expected new tab shortcut");
        app.dispatch_app_action(action).unwrap();

        assert_eq!(app.active_workspace_id(), rssh_core::WorkspaceId::new(1));
        assert_eq!(app.active_tab_id(), rssh_core::TabId::new(2));
    }

    #[test]
    fn window_app_records_osc7_current_working_dir_on_active_pane_launch() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]7;file://host/home/ops\x07")
            .unwrap();

        assert_eq!(
            app.app_shell.active_pane().launch().cwd(),
            Some("file://host/home/ops")
        );
    }

    #[test]
    fn window_app_records_process_current_working_dir_when_osc7_is_absent() {
        let mut app = NativeWindowApp::new(None);
        let cwd = std::env::current_dir().unwrap();
        let child = spawn_sleeping_child(&cwd).unwrap();
        let cwd = cwd.to_string_lossy().into_owned();
        app.session_process_id = Some(child.id());

        app.sync_active_pane_current_working_dir_from_runtime();

        assert_eq!(
            app.app_shell.active_pane().launch().cwd(),
            Some(cwd.as_str())
        );
    }

    #[test]
    fn process_tree_current_working_dir_prefers_local_child_process_cwd() {
        let root_cwd = PathBuf::from("root");
        let child_cwd = PathBuf::from("child");
        let root = sysinfo::Pid::from_u32(10);
        let child = sysinfo::Pid::from_u32(11);

        let resolved = process_tree_current_working_dir(
            &[
                ProcessCwdCandidate {
                    pid: root,
                    parent: None,
                    start_time: 1,
                    cwd: Some(&root_cwd),
                },
                ProcessCwdCandidate {
                    pid: child,
                    parent: Some(root),
                    start_time: 2,
                    cwd: Some(&child_cwd),
                },
            ],
            root,
        );

        assert_eq!(resolved, Some(child_cwd.as_path()));
    }

    #[test]
    fn window_app_records_osc7_current_working_dir_on_inactive_pane_launch() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"\x1b]7;file://host/home/ops\x07",
        )
        .unwrap();

        assert_eq!(
            app.app_shell.active_workspace().tabs()[0].panes()[0]
                .launch()
                .cwd(),
            Some("file://host/home/ops")
        );
    }

    #[test]
    fn window_app_records_inactive_process_current_working_dir_when_osc7_is_absent() {
        let mut app = NativeWindowApp::new(None);
        let cwd = std::env::current_dir().unwrap();
        let child = spawn_sleeping_child(&cwd).unwrap();
        let cwd = cwd.to_string_lossy().into_owned();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let mut runtime = app.new_inactive_pane_runtime();
        runtime.session_process_id = Some(child.id());
        app.pane_runtimes.insert(rssh_core::PaneId::new(1), runtime);

        app.handle_pane_pty_output(rssh_core::PaneId::new(1), b"prompt")
            .unwrap();

        assert_eq!(
            app.app_shell.active_workspace().tabs()[0].panes()[0]
                .launch()
                .cwd(),
            Some(cwd.as_str())
        );
    }

    #[test]
    fn window_app_renders_iterm_badge_format_in_active_pane() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"live\x1b]1337;SetBadgeFormat=cHJvZA==\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" prod "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_user_vars_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_HOST=cHJvZA==\x07")
            .unwrap();
        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=aG9zdDogXCh1c2VyLldFWlRFUk1fSE9TVCk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" host: prod "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_name_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]2;prod-shell\x07").unwrap();
        app.handle_pty_output(b"live\x1b]1337;SetBadgeFormat=bmFtZTogXChzZXNzaW9uLm5hbWUp\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" name: prod-shell "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_auto_name_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]1;auto-title\x07").unwrap();
        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=YXV0bzogXChzZXNzaW9uLmF1dG9OYW1lKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" auto: auto-title "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_auto_name_format_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]1;auto-title\x07").unwrap();
        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=YXV0byBmb3JtYXQ6IFwoc2Vzc2lvbi5hdXRvTmFtZUZvcm1hdCk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" auto format: auto-title "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_keeps_auto_name_format_from_profile_when_window_title_changes() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rssh_pty::PtyCommand::new("pwsh").with_env("RSSH_PROFILE", "ops-window"),
        );

        app.handle_pty_output(b"\x1b]2;window-title\x07").unwrap();
        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=YXV0byBmb3JtYXQ6IFwoc2Vzc2lvbi5hdXRvTmFtZUZvcm1hdCkgbmFtZTogXChzZXNzaW9uLm5hbWUp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" auto format: ops-window name: window-title "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_presentation_name_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]2;ops-title\x07").unwrap();
        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=cHJlc2VudGF0aW9uOiBcKHNlc3Npb24ucHJlc2VudGF0aW9uTmFtZSk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" presentation: ops-title "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_job_name_in_iterm_badge_format() {
        let mut app =
            NativeWindowApp::new_with_command(None, rssh_pty::PtyCommand::new("python.exe"));

        app.handle_pty_output(b"live\x1b]1337;SetBadgeFormat=am9iOiBcKHNlc3Npb24uam9iTmFtZSk=\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" job: python.exe "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_command_line_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rssh_pty::PtyCommand::new("python.exe").with_args(["-m", "http.server"]),
        );

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=Y21kOiBcKHNlc3Npb24uY29tbWFuZExpbmUp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" cmd: python.exe -m http.server "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_last_command_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"\x1b]133;A\x07> \x1b]133;B\x07cargo test\r\n\x1b]133;C\x07ok\x1b]133;D;0\x07",
        )
        .unwrap();
        app.handle_pty_output(
            b"\x1b]1337;SetBadgeFormat=bGFzdDogXChzZXNzaW9uLmxhc3RDb21tYW5kKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" last: cargo test "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_current_session_last_command_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"\x1b]133;A\x07> \x1b]133;B\x07npm run build\r\n\x1b]133;C\x07ok\x1b]133;D;0\x07",
        )
        .unwrap();
        app.handle_pty_output(
            b"\x1b]1337;SetBadgeFormat=Y3VycmVudCBsYXN0OiBcKHRhYi5jdXJyZW50U2Vzc2lvbi5sYXN0Q29tbWFuZCl8XCh0YWIud2luZG93LmN1cnJlbnRUYWIuY3VycmVudFNlc3Npb24ubGFzdENvbW1hbmQp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" current last: npm run build|npm run build "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_keeps_previous_last_command_while_new_prompt_input_is_unfinished() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"\x1b]133;A\x07> \x1b]133;B\x07cargo check\r\n\x1b]133;C\x07ok\x1b]133;D;0\x07\r\n\x1b]133;A\x07> \x1b]133;B\x07draft",
        )
        .unwrap();
        app.handle_pty_output(
            b"\x1b]1337;SetBadgeFormat=c3RpbGwgbGFzdDogXChzZXNzaW9uLmxhc3RDb21tYW5kKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" still last: cargo check "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_process_title_in_iterm_badge_format() {
        let mut app =
            NativeWindowApp::new_with_command(None, rssh_pty::PtyCommand::new("python.exe"));

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=cHJvYzogXChzZXNzaW9uLnByb2Nlc3NUaXRsZSk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" proc: python.exe "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_profile_name_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rssh_pty::PtyCommand::new("pwsh").with_env("RSSH_PROFILE", "ops-window"),
        );

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=cHJvZmlsZTogXChzZXNzaW9uLnByb2ZpbGVOYW1lKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" profile: ops-window "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_pid_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);
        app.session_process_id = Some(4242);

        app.handle_pty_output(b"live\x1b]1337;SetBadgeFormat=cGlkOiBcKHNlc3Npb24ucGlkKQ==\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" pid: 4242 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_job_pid_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);
        app.session_process_id = Some(4343);

        app.handle_pty_output(b"live\x1b]1337;SetBadgeFormat=am9iOiBcKHNlc3Npb24uam9iUGlkKQ==\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" job: 4343 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_tty_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);
        app.session_tty_name = Some("/dev/pts/7".to_owned());

        app.handle_pty_output(b"live\x1b]1337;SetBadgeFormat=dHR5OiBcKHNlc3Npb24udHR5KQ==\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" tty: /dev/pts/7 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_home_directory_in_iterm_badge_format() {
        let expected_home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .expect("test requires HOME or USERPROFILE");
        let expected_suffix = format!(" home: {expected_home} ");
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=aG9tZTogXChzZXNzaW9uLmhvbWVEaXJlY3Rvcnkp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(&expected_suffix),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_ssh_integration_level_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=c3NoOiBcKHNlc3Npb24uc3NoSW50ZWdyYXRpb25MZXZlbCk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" ssh: 0 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_username_in_iterm_badge_format() {
        let expected_user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .or_else(|_| std::env::var("LOGNAME"))
            .expect("test requires USER, USERNAME, or LOGNAME");
        let expected_suffix = format!(" user: {expected_user} ");
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=dXNlcjogXChzZXNzaW9uLnVzZXJuYW1lKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(&expected_suffix),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_hostname_in_iterm_badge_format() {
        let expected_suffix = super::local_host_name().map_or_else(
            || " host: ".to_owned(),
            |host| format!(" host: {host} "),
        );
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=aG9zdDogXChzZXNzaW9uLmhvc3RuYW1lKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(&expected_suffix),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_shell_in_iterm_badge_format() {
        let expected_shell = if cfg!(windows) {
            std::env::var("COMSPEC")
                .or_else(|_| std::env::var("SHELL"))
                .unwrap_or_else(|_| "cmd.exe".to_owned())
        } else {
            std::env::var("SHELL")
                .or_else(|_| std::env::var("COMSPEC"))
                .unwrap_or_else(|_| "/bin/sh".to_owned())
        };
        let expected_suffix = format!(" shell: {expected_shell} ");
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"live\x1b]1337;SetBadgeFormat=c2hlbGw6IFwoc2Vzc2lvbi5zaGVsbCk=\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(&expected_suffix),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_uname_in_iterm_badge_format() {
        let expected_uname = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
        let expected_suffix = format!(" uname: {expected_uname} ");
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"live\x1b]1337;SetBadgeFormat=dW5hbWU6IFwoc2Vzc2lvbi51bmFtZSk=\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(&expected_suffix),
            "first terminal row was {first_terminal_row:?}, expected suffix {expected_suffix:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_path_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]1337;CurrentDir=/tmp/project\x07")
            .unwrap();
        app.handle_pty_output(b"live\x1b]1337;SetBadgeFormat=cGF0aDogXChzZXNzaW9uLnBhdGgp\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" path: /tmp/project "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_terminal_titles_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]1;icon-name\x07\x1b]2;window-name\x07")
            .unwrap();
        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=aWNvbjogXChzZXNzaW9uLnRlcm1pbmFsSWNvbk5hbWUpIHdpbjogXChzZXNzaW9uLnRlcm1pbmFsV2luZG93TmFtZSk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" icon: icon-name win: window-name "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_pane_size_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=c2l6ZTogXChzZXNzaW9uLmNvbHVtbnMpeFwoc2Vzc2lvbi5yb3dzKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" size: 80x24 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_mouse_reporting_mode_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"idle\x1b]1337;SetBadgeFormat=bW91c2U6IFwoc2Vzc2lvbi5tb3VzZVJlcG9ydGluZ01vZGUp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" mouse: -1 "),
            "first terminal row was {first_terminal_row:?}"
        );

        app.handle_pty_output(b"\x1b[?1003h").unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" mouse: 3 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_mouse_info_indices_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b[?1000h").unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH * 3),
            f64::from(tab_bar_pixel_height() + CELL_HEIGHT * 4),
        ))
        .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap();
        app.handle_pty_output(
            b"\x1b]1337;SetBadgeFormat=bW91c2U6IFwoc2Vzc2lvbi5tb3VzZUluZm9bMF0pLFwoc2Vzc2lvbi5tb3VzZUluZm9bMV0pLFwoc2Vzc2lvbi5tb3VzZUluZm9bMl0pLFwoc2Vzc2lvbi5tb3VzZUluZm9bM10pLFwoc2Vzc2lvbi5tb3VzZUluZm9bNV0pLFwoc2Vzc2lvbi5tb3VzZUluZm9bNl0p\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" mouse: 3,4,0,1,8,1 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_current_session_mouse_info_indices_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b[?1000h").unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH * 7),
            f64::from(tab_bar_pixel_height() + CELL_HEIGHT * 2),
        ))
        .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Right)
            .unwrap();
        app.handle_pty_output(
            b"\x1b]1337;SetBadgeFormat=Y3VycmVudCBtb3VzZTogXCh0YWIuY3VycmVudFNlc3Npb24ubW91c2VJbmZvWzBdKSxcKHRhYi5jdXJyZW50U2Vzc2lvbi5tb3VzZUluZm9bMV0pfFwodGFiLndpbmRvdy5jdXJyZW50VGFiLmN1cnJlbnRTZXNzaW9uLm1vdXNlSW5mb1swXSksXCh0YWIud2luZG93LmN1cnJlbnRUYWIuY3VycmVudFNlc3Npb24ubW91c2VJbmZvWzFdKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" current mouse: 7,2|7,2 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_mouse_info_up_event_type_as_zero() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b[?1000h").unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH * 4),
            f64::from(tab_bar_pixel_height() + CELL_HEIGHT * 5),
        ))
        .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap();
        app.handle_mouse_input(ElementState::Released, MouseButton::Left)
            .unwrap();
        set_badge_format(
            &mut app,
            "up: \\(session.mouseInfo[0]),\\(session.mouseInfo[1]),\\(session.mouseInfo[2]),\\(session.mouseInfo[3]),\\(session.mouseInfo[5]),\\(session.mouseInfo[6])",
        );

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" up: 4,5,0,1,8,0 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_mouse_info_drag_event_type_and_side_effect() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b[?1002h").unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH * 2),
            f64::from(tab_bar_pixel_height() + CELL_HEIGHT * 3),
        ))
        .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH * 6),
            f64::from(tab_bar_pixel_height() + CELL_HEIGHT * 7),
        ))
        .unwrap();
        set_badge_format(
            &mut app,
            "drag: \\(session.mouseInfo[0]),\\(session.mouseInfo[1]),\\(session.mouseInfo[2]),\\(session.mouseInfo[3]),\\(session.mouseInfo[5]),\\(session.mouseInfo[6])",
        );

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" drag: 6,7,0,0,136,2 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_mouse_info_modifier_array() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b[?1000h").unwrap();
        app.bypass_mouse_reporting_modifiers = ModifiersState::empty();
        app.modifiers = ModifiersState::ALT | ModifiersState::SHIFT;
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH * 5),
            f64::from(tab_bar_pixel_height() + CELL_HEIGHT * 6),
        ))
        .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap();
        set_badge_format(
            &mut app,
            "mods: \\(session.mouseInfo[0])|\\(session.mouseInfo[4])",
        );

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" mods: 5|[2, 4] "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_mouse_info_array() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b[?1000h").unwrap();
        app.bypass_mouse_reporting_modifiers = ModifiersState::empty();
        app.modifiers = ModifiersState::ALT | ModifiersState::SHIFT;
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH * 5),
            f64::from(tab_bar_pixel_height() + CELL_HEIGHT * 6),
        ))
        .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
            .unwrap();
        set_badge_format(&mut app, "info: \\(session.mouseInfo)");

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" info: [5, 6, 0, 1, [2, 4], 8, 1] "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_current_session_mouse_info_arrays() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b[?1000h").unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH * 3),
            f64::from(tab_bar_pixel_height() + CELL_HEIGHT * 4),
        ))
        .unwrap();
        app.handle_mouse_input(ElementState::Pressed, MouseButton::Right)
            .unwrap();
        set_badge_format(
            &mut app,
            "info: \\(tab.currentSession.mouseInfo)|\\(tab.window.currentTab.currentSession.mouseInfo)",
        );

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" info: [3, 4, 1, 1, [], 8, 1]|[3, 4, 1, 1, [], 8, 1] "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_bell_count_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=YmVsbHM6IFwoc2Vzc2lvbi5iZWxsQ291bnQp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" bells: 0 "),
            "first terminal row was {first_terminal_row:?}"
        );

        app.handle_pty_output(b"\x07\x07").unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" bells: 2 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_application_keypad_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"idle\x1b]1337;SetBadgeFormat=a2V5cGFkOiBcKHNlc3Npb24uYXBwbGljYXRpb25LZXlwYWQp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" keypad: false "),
            "first terminal row was {first_terminal_row:?}"
        );

        app.handle_pty_output(b"\x1b=").unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" keypad: true "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_id_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"live\x1b]1337;SetBadgeFormat=aWQ6IFwoc2Vzc2lvbi5pZCk=\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" id: 1 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_session_termid_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"live\x1b]1337;SetBadgeFormat=dGVybTogXChzZXNzaW9uLnRlcm1pZCk=\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" term: w1t1p1 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_title_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rssh_core::TabId::new(1),
            title: "build-prod".to_owned(),
        })
        .unwrap();
        app.handle_pty_output(b"live\x1b]1337;SetBadgeFormat=dGFiOiBcKHRhYi50aXRsZSk=\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" tab: build-prod "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_title_override_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]2;pane-title\x07").unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rssh_core::TabId::new(1),
            title: "explicit-tab".to_owned(),
        })
        .unwrap();
        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=b3ZlcnJpZGU6IFwodGFiLnRpdGxlT3ZlcnJpZGUp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" override: explicit-tab "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_title_override_format_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]2;pane-title\x07").unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rssh_core::TabId::new(1),
            title: "explicit-tab".to_owned(),
        })
        .unwrap();
        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=b3ZlcnJpZGUgZm9ybWF0OiBcKHRhYi50aXRsZU92ZXJyaWRlRm9ybWF0KQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" override format: explicit-tab "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_id_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.active_tab_id(), rssh_core::TabId::new(1));
        assert_eq!(app.app_shell.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"live\x1b]1337;SetBadgeFormat=dGFiIGlkOiBcKHRhYi5pZCk=\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" tab id: 1 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_current_session_id_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=Y3VycmVudDogXCh0YWIuY3VycmVudFNlc3Npb24uaWQp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" current: 2 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_current_session_name_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"\x1b]2;right-shell\x07").unwrap();
        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=Y3VycmVudCBuYW1lOiBcKHRhYi5jdXJyZW50U2Vzc2lvbi5uYW1lKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" current name: right-shell "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_current_session_auto_name_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"\x1b]1;right-shell\x07").unwrap();
        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=Y3VycmVudCBhdXRvOiBcKHRhYi5jdXJyZW50U2Vzc2lvbi5hdXRvTmFtZSk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" current auto: right-shell "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_current_session_auto_name_format_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"\x1b]1;right-title\x07").unwrap();
        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=Y3VycmVudCBhdXRvIGZvcm1hdDogXCh0YWIuY3VycmVudFNlc3Npb24uYXV0b05hbWVGb3JtYXQp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" current auto format: right-title "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_current_session_presentation_name_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"\x1b]2;right-shell\x07").unwrap();
        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=Y3VycmVudCBwcmVzZW50YXRpb246IFwodGFiLmN1cnJlbnRTZXNzaW9uLnByZXNlbnRhdGlvbk5hbWUp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" current presentation: right-shell "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_current_session_process_fields_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: Some(PaneLaunch::local("p").with_args(["-m", "h"])),
        })
        .unwrap();
        assert_eq!(app.app_shell.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=Y3VycmVudCBwcm9jOiBcKHRhYi5jdXJyZW50U2Vzc2lvbi5qb2JOYW1lKXxcKHRhYi5jdXJyZW50U2Vzc2lvbi5wcm9jZXNzVGl0bGUpfFwodGFiLmN1cnJlbnRTZXNzaW9uLmNvbW1hbmRMaW5lKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" current proc: p|p|p -m h "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_current_session_process_identity_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.active_pane_id(), rssh_core::PaneId::new(2));
        app.session_process_id = Some(4242);
        app.session_tty_name = Some("/dev/pts/8".to_owned());

        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=Y3VycmVudCBpZHM6IFwodGFiLmN1cnJlbnRTZXNzaW9uLnBpZCl8XCh0YWIuY3VycmVudFNlc3Npb24uam9iUGlkKXxcKHRhYi5jdXJyZW50U2Vzc2lvbi50dHkp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" current ids: 4242|4242|/dev/pts/8 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_current_session_runtime_fields_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rssh_core::TerminalSize::new(160, 24));

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"rose bud\x1b=\x1b[?1003h\x07\x07")
            .unwrap();
        set_ordinary_viewport_selection_for_test(
            &mut app,
            WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 3 },
            ),
        );
        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=Y3VycmVudCBtZXRyaWNzOiBcKHRhYi5jdXJyZW50U2Vzc2lvbi5jb2x1bW5zKXhcKHRhYi5jdXJyZW50U2Vzc2lvbi5yb3dzKXxcKHRhYi5jdXJyZW50U2Vzc2lvbi5hcHBsaWNhdGlvbktleXBhZCl8XCh0YWIuY3VycmVudFNlc3Npb24ubW91c2VSZXBvcnRpbmdNb2RlKXxcKHRhYi5jdXJyZW50U2Vzc2lvbi5iZWxsQ291bnQpfFwodGFiLmN1cnJlbnRTZXNzaW9uLnNlbGVjdGlvbil8XCh0YWIuY3VycmVudFNlc3Npb24uc2VsZWN0aW9uTGVuZ3RoKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" current metrics: 80x24|true|3|2|rose|4 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_current_session_host_fields_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rssh_core::TerminalSize::new(400, 24));
        let host_context = super::local_badge_host_context();
        let expected = format!(
            " current host: 0|{}|{}|{}|{}|{} ",
            host_context.home_directory.unwrap_or_default(),
            host_context.username.unwrap_or_default(),
            host_context.hostname.unwrap_or_default(),
            host_context.shell.unwrap_or_default(),
            host_context.uname
        );

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=Y3VycmVudCBob3N0OiBcKHRhYi5jdXJyZW50U2Vzc2lvbi5zc2hJbnRlZ3JhdGlvbkxldmVsKXxcKHRhYi5jdXJyZW50U2Vzc2lvbi5ob21lRGlyZWN0b3J5KXxcKHRhYi5jdXJyZW50U2Vzc2lvbi51c2VybmFtZSl8XCh0YWIuY3VycmVudFNlc3Npb24uaG9zdG5hbWUpfFwodGFiLmN1cnJlbnRTZXNzaW9uLnNoZWxsKXxcKHRhYi5jdXJyZW50U2Vzc2lvbi51bmFtZSk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, 400);
        assert!(
            first_terminal_row.contains(&expected),
            "first terminal row was {first_terminal_row:?}, expected {expected:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_current_session_path_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"\x1b]1337;CurrentDir=/tmp/right\x07")
            .unwrap();
        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=Y3VycmVudCBwYXRoOiBcKHRhYi5jdXJyZW50U2Vzc2lvbi5wYXRoKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" current path: /tmp/right "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_current_session_terminal_icon_name_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"\x1b]1;right-icon\x07").unwrap();
        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=Y3VycmVudCBpY29uOiBcKHRhYi5jdXJyZW50U2Vzc2lvbi50ZXJtaW5hbEljb25OYW1lKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" current icon: right-icon "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_current_session_terminal_window_name_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"\x1b]2;right-window\x07").unwrap();
        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=Y3VycmVudCB3aW5kb3c6IFwodGFiLmN1cnJlbnRTZXNzaW9uLnRlcm1pbmFsV2luZG93TmFtZSk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" current window: right-window "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_current_session_profile_name_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: Some(
                PaneLaunch::local("pwsh").with_environment([("RSSH_PROFILE", "ops-right")]),
            ),
        })
        .unwrap();
        assert_eq!(app.app_shell.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=Y3VycmVudCBwcm9maWxlOiBcKHRhYi5jdXJyZW50U2Vzc2lvbi5wcm9maWxlTmFtZSk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" current profile: ops-right "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_id_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);
        app.app_window_id = rssh_core::WindowId::new(7);

        app.handle_pty_output(b"live\x1b]1337;SetBadgeFormat=d2luIGlkOiBcKHRhYi53aW5kb3cuaWQp\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" win id: 7 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_number_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);
        app.app_window_id = rssh_core::WindowId::new(7);

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=d2luIG51bWJlcjogXCh0YWIud2luZG93Lm51bWJlcik=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" win number: 7 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_frame_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=ZnJhbWU6IFwodGFiLndpbmRvdy5mcmFtZSk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        let (frame_width, frame_height) = app.frame_size_for_test();
        assert!(
            first_terminal_row.ends_with(&format!(
                " frame: [0, 0, {frame_width}, {frame_height}] "
            )),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_updates_tab_window_frame_for_move_and_resize() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=ZnJhbWU6IFwodGFiLndpbmRvdy5mcmFtZSk=\x07",
        )
        .unwrap();
        app.handle_window_moved(PhysicalPosition::new(12, -3));
        app.handle_window_resize(PhysicalSize::new(320, 80))
            .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" frame: [12, -3, 320, 80] "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_style_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=c3R5bGU6IFwodGFiLndpbmRvdy5zdHlsZSk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" style: normal "),
            "first terminal row was {first_terminal_row:?}"
        );

        app.toggle_full_screen();
        assert!(app.full_screen_for_test());

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" style: native full screen "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_is_hotkey_window_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=aG90a2V5OiBcKHRhYi53aW5kb3cuaXNIb3RrZXlXaW5kb3cp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" hotkey: false "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_title_override_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]2;ops-window\x07").unwrap();
        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=d2luZG93IG92ZXJyaWRlOiBcKHRhYi53aW5kb3cudGl0bGVPdmVycmlkZSk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" window override: ops-window "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_title_override_format_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]2;ops-window\x07").unwrap();
        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=d2luZG93IG92ZXJyaWRlIGZvcm1hdDogXCh0YWIud2luZG93LnRpdGxlT3ZlcnJpZGVGb3JtYXQp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" window override format: ops-window "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_current_tab_id_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rssh_core::TabId::new(2));
        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=Y3VycmVudCB0YWIgaWQ6IFwodGFiLndpbmRvdy5jdXJyZW50VGFiLmlkKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" current tab id: 2 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_current_tab_title_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rssh_core::TabId::new(1),
            title: "deploy".to_owned(),
        })
        .unwrap();
        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=Y3VycmVudCB0YWIgdGl0bGU6IFwodGFiLndpbmRvdy5jdXJyZW50VGFiLnRpdGxlKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" current tab title: deploy "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_current_tab_title_override_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]2;pane-title\x07").unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rssh_core::TabId::new(1),
            title: "deploy-override".to_owned(),
        })
        .unwrap();
        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=Y3VycmVudCBvdmVycmlkZTogXCh0YWIud2luZG93LmN1cnJlbnRUYWIudGl0bGVPdmVycmlkZSk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" current override: deploy-override "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_current_tab_title_override_format_in_iterm_badge_format()
    {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]2;pane-title\x07").unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rssh_core::TabId::new(1),
            title: "deploy-override".to_owned(),
        })
        .unwrap();
        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=Y3VycmVudCBvdmVycmlkZSBmb3JtYXQ6IFwodGFiLndpbmRvdy5jdXJyZW50VGFiLnRpdGxlT3ZlcnJpZGVGb3JtYXQp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" current override format: deploy-override "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_current_tab_current_session_id_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rssh_core::TabId::new(2));
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=Y3VycmVudCB0YWIgc2Vzc2lvbjogXCh0YWIud2luZG93LmN1cnJlbnRUYWIuY3VycmVudFNlc3Npb24uaWQp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" current tab session: 2 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_current_tab_current_session_name_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"\x1b]2;right-shell\x07").unwrap();
        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=bmFtZTogXCh0YWIud2luZG93LmN1cnJlbnRUYWIuY3VycmVudFNlc3Npb24ubmFtZSk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" name: right-shell "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_current_tab_current_session_auto_name_in_iterm_badge_format()
     {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"\x1b]1;right-shell\x07").unwrap();
        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=d2luZG93IGN1cnJlbnQgYXV0bzogXCh0YWIud2luZG93LmN1cnJlbnRUYWIuY3VycmVudFNlc3Npb24uYXV0b05hbWUp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" window current auto: right-shell "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_current_tab_current_session_auto_name_format_in_iterm_badge_format()
     {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"\x1b]1;right-title\x07").unwrap();
        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=dyBhdXRvIGZvcm1hdDogXCh0YWIud2luZG93LmN1cnJlbnRUYWIuY3VycmVudFNlc3Npb24uYXV0b05hbWVGb3JtYXQp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" w auto format: right-title "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_current_tab_current_session_presentation_name_in_iterm_badge_format()
     {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"\x1b]2;right-shell\x07").unwrap();
        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=dyBwcmVzOiBcKHRhYi53aW5kb3cuY3VycmVudFRhYi5jdXJyZW50U2Vzc2lvbi5wcmVzZW50YXRpb25OYW1lKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" w pres: right-shell "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_current_tab_current_session_process_fields_in_iterm_badge_format()
     {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: Some(PaneLaunch::local("p").with_args(["-m", "h"])),
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=dyBwcm9jOiBcKHRhYi53aW5kb3cuY3VycmVudFRhYi5jdXJyZW50U2Vzc2lvbi5qb2JOYW1lKXxcKHRhYi53aW5kb3cuY3VycmVudFRhYi5jdXJyZW50U2Vzc2lvbi5wcm9jZXNzVGl0bGUpfFwodGFiLndpbmRvdy5jdXJyZW50VGFiLmN1cnJlbnRTZXNzaW9uLmNvbW1hbmRMaW5lKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" w proc: p|p|p -m h "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_current_tab_current_session_process_identity_in_iterm_badge_format()
     {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));
        app.session_process_id = Some(4343);
        app.session_tty_name = Some("/dev/pts/9".to_owned());

        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=dyBpZHM6IFwodGFiLndpbmRvdy5jdXJyZW50VGFiLmN1cnJlbnRTZXNzaW9uLnBpZCl8XCh0YWIud2luZG93LmN1cnJlbnRUYWIuY3VycmVudFNlc3Npb24uam9iUGlkKXxcKHRhYi53aW5kb3cuY3VycmVudFRhYi5jdXJyZW50U2Vzc2lvbi50dHkp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" w ids: 4343|4343|/dev/pts/9 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_current_tab_current_session_runtime_fields_in_iterm_badge_format()
     {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rssh_core::TerminalSize::new(160, 24));

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.app_shell.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"rose bud\x1b=\x1b[?1003h\x07\x07")
            .unwrap();
        set_ordinary_viewport_selection_for_test(
            &mut app,
            WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 3 },
            ),
        );
        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=dyBjdXJyZW50IG1ldHJpY3M6IFwodGFiLndpbmRvdy5jdXJyZW50VGFiLmN1cnJlbnRTZXNzaW9uLmNvbHVtbnMpeFwodGFiLndpbmRvdy5jdXJyZW50VGFiLmN1cnJlbnRTZXNzaW9uLnJvd3MpfFwodGFiLndpbmRvdy5jdXJyZW50VGFiLmN1cnJlbnRTZXNzaW9uLmFwcGxpY2F0aW9uS2V5cGFkKXxcKHRhYi53aW5kb3cuY3VycmVudFRhYi5jdXJyZW50U2Vzc2lvbi5tb3VzZVJlcG9ydGluZ01vZGUpfFwodGFiLndpbmRvdy5jdXJyZW50VGFiLmN1cnJlbnRTZXNzaW9uLmJlbGxDb3VudCl8XCh0YWIud2luZG93LmN1cnJlbnRUYWIuY3VycmVudFNlc3Npb24uc2VsZWN0aW9uKXxcKHRhYi53aW5kb3cuY3VycmVudFRhYi5jdXJyZW50U2Vzc2lvbi5zZWxlY3Rpb25MZW5ndGgp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" w current metrics: 80x24|true|3|2|rose|4 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_current_tab_current_session_host_fields_in_iterm_badge_format()
     {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rssh_core::TerminalSize::new(400, 24));
        let host_context = super::local_badge_host_context();
        let expected = format!(
            " w host: 0|{}|{}|{}|{}|{} ",
            host_context.home_directory.unwrap_or_default(),
            host_context.username.unwrap_or_default(),
            host_context.hostname.unwrap_or_default(),
            host_context.shell.unwrap_or_default(),
            host_context.uname
        );

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=dyBob3N0OiBcKHRhYi53aW5kb3cuY3VycmVudFRhYi5jdXJyZW50U2Vzc2lvbi5zc2hJbnRlZ3JhdGlvbkxldmVsKXxcKHRhYi53aW5kb3cuY3VycmVudFRhYi5jdXJyZW50U2Vzc2lvbi5ob21lRGlyZWN0b3J5KXxcKHRhYi53aW5kb3cuY3VycmVudFRhYi5jdXJyZW50U2Vzc2lvbi51c2VybmFtZSl8XCh0YWIud2luZG93LmN1cnJlbnRUYWIuY3VycmVudFNlc3Npb24uaG9zdG5hbWUpfFwodGFiLndpbmRvdy5jdXJyZW50VGFiLmN1cnJlbnRTZXNzaW9uLnNoZWxsKXxcKHRhYi53aW5kb3cuY3VycmVudFRhYi5jdXJyZW50U2Vzc2lvbi51bmFtZSk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, 400);
        assert!(
            first_terminal_row.contains(&expected),
            "first terminal row was {first_terminal_row:?}, expected {expected:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_current_tab_current_session_path_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"\x1b]1337;CurrentDir=/tmp/right\x07")
            .unwrap();
        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=cGF0aDogXCh0YWIud2luZG93LmN1cnJlbnRUYWIuY3VycmVudFNlc3Npb24ucGF0aCk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" path: /tmp/right "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_current_tab_current_session_icon_name_in_iterm_badge_format()
     {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"\x1b]1;right-icon\x07").unwrap();
        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=aWNvbjogXCh0YWIud2luZG93LmN1cnJlbnRUYWIuY3VycmVudFNlc3Npb24udGVybWluYWxJY29uTmFtZSk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" icon: right-icon "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_current_tab_current_session_window_name_in_iterm_badge_format()
     {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pty_output(b"\x1b]2;right-window\x07").unwrap();
        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=d2luOiBcKHRhYi53aW5kb3cuY3VycmVudFRhYi5jdXJyZW50U2Vzc2lvbi50ZXJtaW5hbFdpbmRvd05hbWUp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" win: right-window "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_tab_window_current_tab_current_session_profile_name_in_badge_format()
    {
        let mut app = NativeWindowApp::new(None);

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: Some(
                PaneLaunch::local("pwsh").with_environment([("RSSH_PROFILE", "ops-current")]),
            ),
        })
        .unwrap();
        assert_eq!(app.app_shell.active_pane_id(), rssh_core::PaneId::new(2));

        app.handle_pane_pty_output(
            rssh_core::PaneId::new(1),
            b"left\x1b]1337;SetBadgeFormat=d2luIHByb2ZpbGU6IFwodGFiLndpbmRvdy5jdXJyZW50VGFiLmN1cnJlbnRTZXNzaW9uLnByb2ZpbGVOYW1lKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.contains(" win profile: ops-current "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_iterm2_pid_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"live\x1b]1337;SetBadgeFormat=cGlkOiBcKGl0ZXJtMi5waWQp\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        let expected = format!(" pid: {} ", std::process::id());
        assert!(
            first_terminal_row.ends_with(&expected),
            "first terminal row was {first_terminal_row:?}, expected suffix {expected:?}"
        );
    }

    #[test]
    fn window_app_interpolates_iterm2_localhost_name_in_iterm_badge_format() {
        let expected_suffix = super::local_host_name().map_or_else(
            || " local: ".to_owned(),
            |host| format!(" local: {host} "),
        );
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=bG9jYWw6IFwoaXRlcm0yLmxvY2FsaG9zdE5hbWUp\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(&expected_suffix),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_iterm2_effective_theme_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=dGhlbWU6IFwoaXRlcm0yLmVmZmVjdGl2ZVRoZW1lKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" theme: dark "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_nested_iterm2_effective_theme_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=dGhlbWVzOiBcKHRhYi5pdGVybTIuZWZmZWN0aXZlVGhlbWUpfFwodGFiLndpbmRvdy5pdGVybTIuZWZmZWN0aXZlVGhlbWUpfFwodGFiLndpbmRvdy5jdXJyZW50VGFiLml0ZXJtMi5lZmZlY3RpdmVUaGVtZSk=\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_terminal_row.ends_with(" themes: dark|dark|dark "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_selection_in_iterm_badge_format() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"alpha beta\x1b]1337;SetBadgeFormat=c2VsOiBcKHNlc3Npb24uc2VsZWN0aW9uKQ==\x07",
        )
        .unwrap();
        set_ordinary_viewport_selection_for_test(
            &mut app,
            WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 4 },
            ),
        );

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" sel: alpha "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_interpolates_selection_length_in_iterm_badge_format_as_utf8_bytes() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"caf\xc3\xa9 zip\x1b]1337;SetBadgeFormat=bGVuOiBcKHNlc3Npb24uc2VsZWN0aW9uTGVuZ3RoKQ==\x07",
        )
        .unwrap();
        set_ordinary_viewport_selection_for_test(
            &mut app,
            WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 3 },
            ),
        );

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" len: 5 "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_renders_undefined_iterm_badge_variables_as_empty() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=bWlzc2luZzpcKHVzZXIubm9wZSk6XChzZXNzaW9uLnBhdGgpOlwoc2Vzc2lvbi50ZXJtaW5hbEljb25OYW1lKTpcKHNlc3Npb24udGVybWluYWxXaW5kb3dOYW1lKQ==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" missing:::: "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_renders_unknown_iterm_badge_variables_as_empty() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(
            b"live\x1b]1337;SetBadgeFormat=dW5rbm93bjpcKGJvZ3VzLm5hbWUpOmVuZA==\x07",
        )
        .unwrap();

        let snapshot = app.render_snapshot();
        let first_terminal_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);

        assert!(
            first_terminal_row.ends_with(" unknown::end "),
            "first terminal row was {first_terminal_row:?}"
        );
    }

    #[test]
    fn window_app_records_progress_on_active_and_inactive_pane_metadata() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]9;4;1;42\x07").unwrap();
        assert_eq!(
            app.app_shell.active_pane().progress(),
            rssh_core::app_shell::PaneProgress::Percentage(42)
        );

        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.handle_pane_pty_output(rssh_core::PaneId::new(1), b"\x9d9;4;2;7\x9c")
            .unwrap();

        assert_eq!(
            app.app_shell.active_workspace().tabs()[0].panes()[0].progress(),
            rssh_core::app_shell::PaneProgress::Error(7)
        );
    }

    #[test]
    fn window_app_tab_bar_shows_active_pane_progress() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]9;4;1;42\x07").unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);

        assert!(tab_bar.contains("42%"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_tab_bar_shows_inactive_tab_active_pane_progress() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        app.handle_pane_pty_output(rssh_core::PaneId::new(1), b"\x1b]9;4;2;7\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);

        assert!(tab_bar.contains("err:7%"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_renders_tab_bar_above_terminal_snapshot() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"live").unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);

        assert!(tab_bar.contains("ws:default"));
        assert!(tab_bar.contains("1:1 panes:1"));
        assert!(tab_bar.contains("2:2* panes:1"));
        assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS, 0), Some('l'));
        assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS, 3), Some('e'));
    }

    #[test]
    fn window_app_can_render_tab_bar_at_bottom() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"live").unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            tab_bar_at_bottom: Some(true),
            ..NativeConfigSnapshot::default()
        });

        let snapshot = app.render_snapshot();
        let first_row = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let tab_bar = snapshot_row_text(&snapshot, TERMINAL_ROWS, TERMINAL_COLUMNS);

        assert!(
            !first_row.contains("ws:default"),
            "first row was {first_row:?}"
        );
        assert_eq!(snapshot_char(&snapshot, 0, 0), Some('l'));
        assert_eq!(snapshot_char(&snapshot, 0, 3), Some('e'));
        assert!(tab_bar.contains("ws:default"), "tab bar was {tab_bar:?}");
        assert!(tab_bar.contains("1:1 panes:1"), "tab bar was {tab_bar:?}");
        assert!(tab_bar.contains("2:2* panes:1"), "tab bar was {tab_bar:?}");

        let first_tab_column = app.tab_bar_workspace_label().chars().count() + 1;
        let x = u32::try_from(first_tab_column).unwrap_or(0) * CELL_WIDTH;
        let y = u32::from(TERMINAL_ROWS) * CELL_HEIGHT;
        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), f64::from(y)))
            .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rssh_core::TabId::new(1));
    }

    #[test]
    fn window_app_can_disable_tab_bar() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"live").unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            enable_tab_bar: Some(false),
            ..NativeConfigSnapshot::default()
        });

        let snapshot = app.render_snapshot();
        let first_row = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);

        assert!(
            !first_row.contains("ws:default"),
            "first row was {first_row:?}"
        );
        assert_eq!(snapshot_char(&snapshot, 0, 0), Some('l'));
        assert_eq!(snapshot_char(&snapshot, 0, 3), Some('e'));

        app.handle_cursor_moved(PhysicalPosition::new(f64::from(CELL_WIDTH), 0.0))
            .unwrap();
        let _ = app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0));
        assert_eq!(app.active_tab_id(), rssh_core::TabId::new(2));
    }

    #[test]
    fn window_app_can_hide_tab_bar_when_only_one_tab() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"live").unwrap();
        app.set_config_overrides(native_config_snapshot! {
            hide_tab_bar_if_only_one_tab: Some(true),
            ..NativeConfigSnapshot::default()
        });

        let snapshot = app.render_snapshot();
        let first_row = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            !first_row.contains("ws:default"),
            "first row was {first_row:?}"
        );
        assert_eq!(snapshot_char(&snapshot, 0, 0), Some('l'));

        let hidden_new_tab_x = u32::try_from(app.tab_bar_workspace_label().chars().count() + 1)
            .unwrap_or(0)
            * CELL_WIDTH;
        app.handle_cursor_moved(PhysicalPosition::new(f64::from(hidden_new_tab_x), 0.0))
            .unwrap();
        let _ = app.handle_mouse_input(ElementState::Pressed, MouseButton::Left);
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);

        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("ws:default"), "tab bar was {tab_bar:?}");
        assert!(tab_bar.contains("2:2* panes:1"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_update_status_handler_sets_tab_bar_status_text() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let mut app = NativeWindowApp::new(None);
        app.update_status_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            NativeWindowStatusUpdate {
                left_status: Some("LEFT".to_owned()),
                right_status: Some("RIGHT".to_owned()),
            }
        });

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("ws:default LEFT"),
            "tab bar was {tab_bar:?}"
        );
        assert!(tab_bar.contains("RIGHT"), "tab bar was {tab_bar:?}");

        assert_eq!(
            seen.lock().unwrap().as_slice(),
            [NativeWindowStatusUpdateEvent {
                window_id: rssh_core::WindowId::new(1),
                pane: rssh_core::PaneId::new(1)
            }]
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_update_status_event_status_setters() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_left_status('LEFT-LUA')
              window:set_right_status("RIGHT-LUA")
            end)
            "#,
        )
        .expect("expected static WezTerm update-status event status setters");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("ws:default LEFT-LUA"),
            "tab bar was {tab_bar:?}"
        );
        assert!(tab_bar.ends_with("RIGHT-LUA"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_update_status_event_string_concat_status_setters() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local event_prefix = 'update-'
            local event_kind = 'status'

            wezterm.on(event_prefix .. event_kind, function(window, pane)
              local left_prefix = 'LEFT-'
              local right_prefix = 'RIGHT-'
              window:set_left_status(left_prefix .. 'LUA')
              window:set_right_status(right_prefix .. 'LUA')
            end)
            "#,
        )
        .expect("expected static WezTerm update-status string concat status setters");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("ws:default LEFT-LUA"),
            "tab bar was {tab_bar:?}"
        );
        assert!(tab_bar.ends_with("RIGHT-LUA"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_update_right_status_event_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-right-status', function(window, pane)
              window:set_right_status('RIGHT-LEGACY-LUA')
            end)
            "#,
        )
        .expect("expected static WezTerm update-right-status event status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.ends_with("RIGHT-LEGACY-LUA"),
            "tab bar was {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_parses_wezterm_update_right_status_active_workspace_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let mut overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-right-status', function(window, pane)
              window:set_right_status(window:active_workspace())
            end)
            "#,
        )
        .expect("expected WezTerm active workspace status setter");
        overrides.default_workspace = Some("ops".to_owned());
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        assert_eq!(app.app_shell.active_workspace().name(), "ops");
        assert_eq!(app.right_status, "ops");
    }

    #[test]
    fn window_app_parses_update_status_active_workspace_concat_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let mut overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('ws=' .. window:active_workspace())
            end)
            "#,
        )
        .expect("expected WezTerm active workspace concat status setter");
        overrides.default_workspace = Some("ops".to_owned());
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        assert_eq!(app.app_shell.active_workspace().name(), "ops");
        assert_eq!(app.right_status, "ws=ops");
    }

    #[test]
    fn window_app_parses_update_status_tostring_active_workspace_concat_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let mut overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('ws=' .. tostring(window:active_workspace()))
            end)
            "#,
        )
        .expect("expected WezTerm tostring active workspace concat status setter");
        overrides.default_workspace = Some("ops".to_owned());
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        assert_eq!(app.app_shell.active_workspace().name(), "ops");
        assert_eq!(app.right_status, "ws=ops");
    }

    #[test]
    fn window_app_parses_update_status_tostring_active_workspace_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let mut overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status(tostring(window:active_workspace()))
            end)
            "#,
        )
        .expect("expected WezTerm tostring active workspace status setter");
        overrides.default_workspace = Some("ops".to_owned());
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        assert_eq!(app.app_shell.active_workspace().name(), "ops");
        assert_eq!(app.right_status, "ops");
    }

    #[test]
    fn window_app_parses_update_status_active_workspace_alias_concat_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let mut overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local ws = window:active_workspace()
              window:set_right_status('ws=' .. ws)
            end)
            "#,
        )
        .expect("expected WezTerm active workspace alias concat status setter");
        overrides.default_workspace = Some("ops".to_owned());
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        assert_eq!(app.app_shell.active_workspace().name(), "ops");
        assert_eq!(app.right_status, "ws=ops");
    }

    #[test]
    fn window_app_parses_documented_wezterm_update_right_status_active_key_table_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-right-status', function(window, pane)
              local name = window:active_key_table()
              if name then
                name = 'TABLE: ' .. name
              end
              window:set_right_status(name or '')
            end)
            "#,
        )
        .expect("expected documented WezTerm active key table status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "");

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
        app.dispatch_update_status();
        assert_eq!(app.active_key_table_for_test(), Some("resize_pane"));
        assert_eq!(app.right_status, "TABLE: resize_pane");

        assert!(app.command_palette_execute(WindowCommand::ClearKeyTableStack));
        app.dispatch_update_status();
        assert_eq!(app.right_status, "");
    }

    #[test]
    fn window_app_parses_documented_wezterm_update_right_status_leader_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            wezterm.on('update-right-status', function(window, pane)
              local leader = ''
              if window:leader_is_active() then
                leader = 'LEADER'
              end
              window:set_right_status(leader)
            end)

            config.leader = { key = 'a', mods = 'CTRL', timeout_milliseconds = 1000 }
            config.colors = {
              compose_cursor = 'orange',
            }

            return config
            "#,
        )
        .expect("expected documented WezTerm leader status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "");

        app.modifiers = ModifiersState::CONTROL;
        app.handle_keyboard_input_event(
            &Key::Character("a".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            Some("a"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "LEADER");

        app.modifiers = ModifiersState::empty();
        app.handle_keyboard_input_event(
            &Key::Character("x".into()),
            PhysicalKey::Code(WinitKeyCode::KeyX),
            Some("x"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_is_focused_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local focus = 'BLURRED'
              if window:is_focused() then
                focus = 'FOCUSED'
              else
                focus = 'BLURRED'
              end
              window:set_right_status(focus)
            end)
            "#,
        )
        .expect("expected WezTerm is_focused status setter");
        app.set_config_overrides(overrides);

        assert!(app.handle_focus_changed(true).unwrap());
        app.dispatch_update_status();
        assert_eq!(app.right_status, "FOCUSED");

        assert!(app.handle_focus_changed(false).unwrap());
        assert_eq!(app.right_status, "BLURRED");

        assert!(app.handle_focus_changed(true).unwrap());
        assert_eq!(app.right_status, "FOCUSED");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_window_id_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('win=' .. window:window_id())
            end)
            "#,
        )
        .expect("expected WezTerm window_id status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        assert_eq!(app.right_status, "win=1");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_pane_id_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('win=' .. window:window_id() .. ' pane=' .. pane:pane_id())
            end)
            "#,
        )
        .expect("expected WezTerm pane_id status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        assert_eq!(app.right_status, "win=1 pane=1");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_pane_title_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('title=' .. pane:get_title())
            end)
            "#,
        )
        .expect("expected WezTerm pane get_title status setter");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();

        app.dispatch_update_status();

        assert_eq!(app.right_status, "title=PowerShell");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_active_pane_alias_title_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local active = window:active_pane()
              window:set_right_status('title=' .. active:get_title())
            end)
            "#,
        )
        .expect("expected WezTerm active pane alias get_title status setter");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();

        app.dispatch_update_status();

        assert_eq!(app.right_status, "title=PowerShell");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_active_tab_title_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('tab=' .. window:active_tab():get_title())
            end)
            "#,
        )
        .expect("expected WezTerm active tab get_title status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "tab=");

        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: app.app_shell.active_tab_id(),
            title: "build".to_owned(),
        })
        .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "tab=build");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_active_tab_title_fallback_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status(window:active_tab():get_title() or '')
            end)
            "#,
        )
        .expect("expected WezTerm active tab get_title fallback status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "");

        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: app.app_shell.active_tab_id(),
            title: "build".to_owned(),
        })
        .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "build");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_active_tab_alias_title_fallback_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local tab = window:active_tab()
              window:set_right_status(tab:get_title() or '')
            end)
            "#,
        )
        .expect("expected WezTerm active tab alias get_title fallback status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "");

        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: app.app_shell.active_tab_id(),
            title: "build".to_owned(),
        })
        .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "build");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_active_tab_alias_title_fallback_concat_status_setter()
     {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local tab = window:active_tab()
              window:set_right_status('tab=' .. (tab:get_title() or ''))
            end)
            "#,
        )
        .expect("expected WezTerm active tab alias get_title fallback concat status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "tab=");

        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: app.app_shell.active_tab_id(),
            title: "build".to_owned(),
        })
        .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "tab=build");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_active_tab_alias_title_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local tab = window:active_tab()
              window:set_right_status('tab=' .. tab:get_title())
            end)
            "#,
        )
        .expect("expected WezTerm active tab alias get_title status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "tab=");

        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: app.app_shell.active_tab_id(),
            title: "build".to_owned(),
        })
        .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "tab=build");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_active_tab_alias_title_variable_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local tab = window:active_tab()
              local title = tab:get_title()
              window:set_right_status('tab=' .. title)
            end)
            "#,
        )
        .expect("expected WezTerm active tab alias title variable status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "tab=");

        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: app.app_shell.active_tab_id(),
            title: "build".to_owned(),
        })
        .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "tab=build");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_active_tab_alias_title_variable_fallback_status_setter()
     {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local tab = window:active_tab()
              local title = tab:get_title()
              window:set_right_status(title or '')
            end)
            "#,
        )
        .expect("expected WezTerm active tab alias title variable fallback status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "");

        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: app.app_shell.active_tab_id(),
            title: "build".to_owned(),
        })
        .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "build");
    }

    #[test]
    fn window_app_parses_update_status_title_variable_named_fallback_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local tab = window:active_tab()
              local title = tab:get_title()
              local fallback = ''
              window:set_right_status(title or fallback)
            end)
            "#,
        )
        .expect("expected WezTerm active tab alias title variable named fallback status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "");

        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: app.app_shell.active_tab_id(),
            title: "build".to_owned(),
        })
        .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "build");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_active_tab_alias_title_variable_fallback_concat_status_setter()
     {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local tab = window:active_tab()
              local title = tab:get_title()
              window:set_right_status('tab=' .. (title or ''))
            end)
            "#,
        )
        .expect("expected WezTerm active tab alias title variable fallback concat status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "tab=");

        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: app.app_shell.active_tab_id(),
            title: "build".to_owned(),
        })
        .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "tab=build");
    }

    #[test]
    fn window_app_parses_update_status_title_variable_top_level_named_fallback_concat_status_setter()
     {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local fallback = ''

            wezterm.on('update-status', function(window, pane)
              local tab = window:active_tab()
              local title = tab:get_title()
              window:set_right_status('tab=' .. (title or fallback))
            end)
            "#,
        )
        .expect("expected WezTerm active tab title variable top-level named fallback concat status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "tab=");

        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: app.app_shell.active_tab_id(),
            title: "build".to_owned(),
        })
        .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "tab=build");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_active_tab_id_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('tab=' .. window:active_tab():tab_id())
            end)
            "#,
        )
        .expect("expected WezTerm active tab tab_id status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "tab=1");

        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "tab=2");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_active_pane_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status(
                'pane=' .. window:active_pane():pane_id()
                  .. ' title=' .. window:active_pane():get_title()
              )
            end)
            "#,
        )
        .expect("expected WezTerm active pane status setter");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();

        app.dispatch_update_status();

        assert_eq!(app.right_status, "pane=1 title=PowerShell");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_pane_domain_name_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('domain=' .. pane:get_domain_name())
            end)
            "#,
        )
        .expect("expected WezTerm pane get_domain_name status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        assert_eq!(app.right_status, "domain=local");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_pane_cwd_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('cwd=' .. pane:get_current_working_dir())
            end)
            "#,
        )
        .expect("expected WezTerm pane get_current_working_dir status setter");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]7;file://host/home/ops\x07")
            .unwrap();

        app.dispatch_update_status();

        assert_eq!(app.right_status, "cwd=file://host/home/ops");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_pane_foreground_process_status_setter() {
        let mut app = NativeWindowApp::new_with_command(None, rssh_pty::PtyCommand::new("pwsh"));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('proc=' .. pane:get_foreground_process_name())
            end)
            "#,
        )
        .expect("expected WezTerm pane get_foreground_process_name status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        assert_eq!(app.right_status, "proc=pwsh");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_pane_tty_name_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('tty=' .. pane:get_tty_name())
            end)
            "#,
        )
        .expect("expected WezTerm pane get_tty_name status setter");
        app.set_config_overrides(overrides);
        app.session_tty_name = Some("/dev/pts/9".to_owned());

        app.dispatch_update_status();

        assert_eq!(app.right_status, "tty=/dev/pts/9");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_pane_dimensions_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local dims = pane:get_dimensions()
              window:set_right_status(
                dims.cols .. 'x' .. dims.viewport_rows
                  .. ' scroll=' .. dims.scrollback_rows
                  .. ' top=' .. dims.physical_top
                  .. '/' .. dims.scrollback_top
              )
            end)
            "#,
        )
        .expect("expected WezTerm pane get_dimensions status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        assert_eq!(app.right_status, "80x24 scroll=24 top=0/0");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_active_pane_dimensions_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local dims = window:active_pane():get_dimensions()
              window:set_right_status(
                dims.cols .. 'x' .. dims.viewport_rows
                  .. ' scroll=' .. dims.scrollback_rows
                  .. ' top=' .. dims.physical_top
                  .. '/' .. dims.scrollback_top
              )
            end)
            "#,
        )
        .expect("expected WezTerm active pane get_dimensions status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        assert_eq!(app.right_status, "80x24 scroll=24 top=0/0");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_pane_alt_screen_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local screen = 'main'
              if pane:is_alt_screen_active() then
                screen = 'alt'
              else
                screen = 'main'
              end
              window:set_right_status(screen)
            end)
            "#,
        )
        .expect("expected WezTerm pane is_alt_screen_active status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "main");

        app.handle_pty_output(b"\x1b[?1049h").unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "alt");

        app.handle_pty_output(b"\x1b[?1049l").unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "main");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_active_pane_alt_screen_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local screen = 'main'
              if window:active_pane():is_alt_screen_active() then
                screen = 'alt'
              else
                screen = 'main'
              end
              window:set_right_status(screen)
            end)
            "#,
        )
        .expect("expected WezTerm active pane is_alt_screen_active status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "main");

        app.handle_pty_output(b"\x1b[?1049h").unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "alt");

        app.handle_pty_output(b"\x1b[?1049l").unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "main");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_pane_unseen_output_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local visibility = 'seen'
              if pane:has_unseen_output() then
                visibility = 'unseen'
              else
                visibility = 'seen'
              end
              window:set_right_status(visibility)
            end)
            "#,
        )
        .expect("expected WezTerm pane has_unseen_output status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "seen");

        app.sync_pane_has_unseen_output_from_value(app.app_shell.active_pane_id(), true);
        app.dispatch_update_status();
        assert_eq!(app.right_status, "unseen");

        app.sync_pane_has_unseen_output_from_value(app.app_shell.active_pane_id(), false);
        app.dispatch_update_status();
        assert_eq!(app.right_status, "seen");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_active_pane_unseen_output_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local visibility = 'seen'
              if window:active_pane():has_unseen_output() then
                visibility = 'unseen'
              else
                visibility = 'seen'
              end
              window:set_right_status(visibility)
            end)
            "#,
        )
        .expect("expected WezTerm active pane has_unseen_output status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "seen");

        app.sync_pane_has_unseen_output_from_value(app.app_shell.active_pane_id(), true);
        app.dispatch_update_status();
        assert_eq!(app.right_status, "unseen");

        app.sync_pane_has_unseen_output_from_value(app.app_shell.active_pane_id(), false);
        app.dispatch_update_status();
        assert_eq!(app.right_status, "seen");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_pane_cursor_position_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local pos = pane:get_cursor_position()
              window:set_right_status(
                pos.x .. ',' .. pos.y
                  .. ' ' .. pos.shape
                  .. ' ' .. pos.visibility
              )
            end)
            "#,
        )
        .expect("expected WezTerm pane get_cursor_position status setter");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b[4;7H\x1b[6 q").unwrap();

        app.dispatch_update_status();
        assert_eq!(app.right_status, "6,3 Bar Visible");

        app.handle_pty_output(b"\x1b[?25l").unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "6,3 Bar Hidden");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_active_pane_cursor_position_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local pos = window:active_pane():get_cursor_position()
              window:set_right_status(
                pos.x .. ',' .. pos.y
                  .. ' ' .. pos.shape
                  .. ' ' .. pos.visibility
              )
            end)
            "#,
        )
        .expect("expected WezTerm active pane get_cursor_position status setter");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b[4;7H\x1b[6 q").unwrap();

        app.dispatch_update_status();
        assert_eq!(app.right_status, "6,3 Bar Visible");

        app.handle_pty_output(b"\x1b[?25l").unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "6,3 Bar Hidden");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_pane_user_vars_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local vars = pane:get_user_vars()
              window:set_right_status(
                'prog=' .. vars.WEZTERM_PROG .. ' host=' .. vars.WEZTERM_HOST
              )
            end)
            "#,
        )
        .expect("expected WezTerm pane get_user_vars status setter");
        app.set_config_overrides(overrides);
        app.handle_pty_output(
            b"\x1b]1337;SetUserVar=WEZTERM_PROG=cHNo\x07\x1b]1337;SetUserVar=WEZTERM_HOST=cHJvZA==\x07",
        )
        .unwrap();

        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog=psh host=prod");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_pane_user_vars_bracket_key_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local vars = pane:get_user_vars()
              window:set_right_status(
                'prog=' .. vars['WEZTERM-PROG']
                  .. ' host=' .. tostring(vars["WEZTERM-HOST"])
              )
            end)
            "#,
        )
        .expect("expected WezTerm pane get_user_vars bracket-key status setter");
        app.set_config_overrides(overrides);
        app.handle_pty_output(
            b"\x1b]1337;SetUserVar=WEZTERM-PROG=cHNo\x07\x1b]1337;SetUserVar=WEZTERM-HOST=cHJvZA==\x07",
        )
        .unwrap();

        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog=psh host=prod");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_pane_user_vars_fallback_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local vars = pane:get_user_vars()
              window:set_right_status(
                'prog=' .. (vars['WEZTERM-PROG'] or 'missing')
                  .. ' host=' .. (vars["WEZTERM-HOST"] or 'none')
              )
            end)
            "#,
        )
        .expect("expected WezTerm pane get_user_vars fallback status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog=missing host=none");

        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM-PROG=cHNo\x07")
            .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog=psh host=none");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_pane_user_vars_static_key_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local vars = pane:get_user_vars()
              local prog_key = 'WEZTERM-PROG'
              local host_key = "WEZTERM-HOST"
              window:set_right_status(
                'prog=' .. vars[prog_key]
                  .. ' host=' .. (vars[host_key] or 'none')
              )
            end)
            "#,
        )
        .expect("expected WezTerm pane get_user_vars static-key status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog= host=none");

        app.handle_pty_output(
            b"\x1b]1337;SetUserVar=WEZTERM-PROG=cHNo\x07\x1b]1337;SetUserVar=WEZTERM-HOST=cHJvZA==\x07",
        )
        .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog=psh host=prod");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_pane_user_vars_outer_static_key_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local prog_key = 'WEZTERM-PROG'
            local host_key = "WEZTERM-HOST"

            wezterm.on('update-status', function(window, pane)
              local vars = pane:get_user_vars()
              window:set_right_status(
                'prog=' .. vars[prog_key]
                  .. ' host=' .. (vars[host_key] or 'none')
              )
            end)
            "#,
        )
        .expect("expected WezTerm pane get_user_vars outer static-key status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog= host=none");

        app.handle_pty_output(
            b"\x1b]1337;SetUserVar=WEZTERM-PROG=cHNo\x07\x1b]1337;SetUserVar=WEZTERM-HOST=cHJvZA==\x07",
        )
        .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog=psh host=prod");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_pane_user_vars_tostring_fallback_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local prog_key = 'WEZTERM-PROG'

            wezterm.on('update-status', function(window, pane)
              local vars = pane:get_user_vars()
              window:set_right_status('prog=' .. tostring(vars[prog_key] or 'missing'))
            end)
            "#,
        )
        .expect("expected WezTerm pane get_user_vars tostring fallback status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog=missing");

        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM-PROG=cHNo\x07")
            .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog=psh");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_local_pane_user_var_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local vars = pane:get_user_vars()
              local host = vars.WEZTERM_HOST or 'none'
              window:set_right_status('host=' .. host)
            end)
            "#,
        )
        .expect("expected WezTerm local pane user var status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "host=none");

        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_HOST=cHJvZA==\x07")
            .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "host=prod");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_tostring_local_pane_user_var_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local vars = pane:get_user_vars()
              local host = vars.WEZTERM_HOST or 'none'
              window:set_right_status('host=' .. tostring(host))
            end)
            "#,
        )
        .expect("expected WezTerm tostring local pane user var status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "host=none");

        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_HOST=cHJvZA==\x07")
            .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "host=prod");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_direct_pane_user_vars_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status(
                'prog=' .. pane:get_user_vars().WEZTERM_PROG
                  .. ' host=' .. (window:active_pane():get_user_vars()['WEZTERM-HOST'] or 'none')
              )
            end)
            "#,
        )
        .expect("expected WezTerm direct pane get_user_vars status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog= host=none");

        app.handle_pty_output(
            b"\x1b]1337;SetUserVar=WEZTERM_PROG=cHNo\x07\x1b]1337;SetUserVar=WEZTERM-HOST=cHJvZA==\x07",
        )
        .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog=psh host=prod");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_active_pane_user_vars_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local vars = window:active_pane():get_user_vars()
              window:set_right_status(
                'prog=' .. vars.WEZTERM_PROG .. ' host=' .. vars.WEZTERM_HOST
              )
            end)
            "#,
        )
        .expect("expected WezTerm active pane get_user_vars status setter");
        app.set_config_overrides(overrides);
        app.handle_pty_output(
            b"\x1b]1337;SetUserVar=WEZTERM_PROG=cHNo\x07\x1b]1337;SetUserVar=WEZTERM_HOST=cHJvZA==\x07",
        )
        .unwrap();

        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog=psh host=prod");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_pane_progress_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local progress = pane:get_progress()
              local status = 'None'
              if progress.Percentage ~= nil then
                status = 'pct=' .. progress.Percentage
              elseif progress.Error ~= nil then
                status = 'err=' .. progress.Error
              elseif progress == 'Indeterminate' then
                status = progress
              end
              window:set_right_status(status)
            end)
            "#,
        )
        .expect("expected WezTerm pane get_progress status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "None");

        app.handle_pty_output(b"\x1b]9;4;1;42\x07").unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "pct=42");

        app.handle_pty_output(b"\x1b]9;4;2;7\x07").unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "err=7");

        app.handle_pty_output(b"\x1b]9;4;3;0\x07").unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "Indeterminate");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_active_pane_progress_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local progress = window:active_pane():get_progress()
              local status = 'None'
              if progress.Percentage ~= nil then
                status = 'pct=' .. progress.Percentage
              elseif progress.Error ~= nil then
                status = 'err=' .. progress.Error
              elseif progress == 'Indeterminate' then
                status = progress
              end
              window:set_right_status(status)
            end)
            "#,
        )
        .expect("expected WezTerm active pane get_progress status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "None");

        app.handle_pty_output(b"\x1b]9;4;1;42\x07").unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "pct=42");

        app.handle_pty_output(b"\x1b]9;4;2;7\x07").unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "err=7");

        app.handle_pty_output(b"\x1b]9;4;3;0\x07").unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "Indeterminate");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_window_dimensions_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local dims = window:get_dimensions()
              window:set_right_status(
                dims.pixel_width .. 'x' .. dims.pixel_height
                  .. '@' .. dims.dpi
                  .. ' full=' .. tostring(dims.is_full_screen)
              )
            end)
            "#,
        )
        .expect("expected WezTerm get_dimensions status setter");
        app.set_config_overrides(overrides);
        app.handle_window_resize(PhysicalSize::new(160, 96))
            .unwrap();

        app.dispatch_update_status();
        assert_eq!(app.right_status, "160x96@96 full=false");

        app.full_screen = true;
        app.dispatch_update_status();
        assert_eq!(app.right_status, "160x96@96 full=true");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_effective_config_font_size_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font_size = 13.5

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('font=' .. window:effective_config().font_size)
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config font_size status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "font=13.5");
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_font_size() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.config_reloaded_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(*event);
            true
        });
        let active_pane = app.app_shell.active_pane_id();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_config_overrides { font_size = 15.0 }
              window:set_right_status('font=' .. window:effective_config().font_size)
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides update-status callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        assert_eq!(
            app.native_effective_config().font_size,
            NativeFontSize::from_millipoints(15_000)
        );
        assert_eq!(app.right_status, "font=15");
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rssh_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rssh_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_font_family_fields() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.config_reloaded_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(*event);
            true
        });
        let active_pane = app.app_shell.active_pane_id();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_config_overrides({
                font = wezterm.font_with_fallback(
                  { 'JetBrains Mono', 'Noto Color Emoji' },
                  {
                    weight = 'DemiBold',
                    stretch = 'Condensed',
                    style = 'Italic',
                  }
                ),
                font_rules = {
                  {
                    italic = true,
                    intensity = 'Bold',
                    font = wezterm.font { family = 'Victor Mono', weight = 'Bold' },
                  },
                },
              })
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides font callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(effective.font.as_deref(), Some("JetBrains Mono"));
        assert_eq!(effective.font_fallbacks, vec!["Noto Color Emoji"]);
        assert_eq!(
            effective.font_attributes,
            NativeFontAttributes {
                weight: Some("DemiBold".to_owned()),
                stretch: Some("Condensed".to_owned()),
                style: Some("Italic".to_owned()),
                harfbuzz_features: Vec::new(),
                assume_emoji_presentation: None,
                freetype_load_target: None,
                freetype_render_target: None,
                freetype_load_flags: None,
            }
        );
        assert_eq!(effective.font_rules.len(), 1);
        assert_eq!(effective.font_rules[0].italic, Some(true));
        assert_eq!(
            effective.font_rules[0].intensity,
            Some(NativeFormatIntensity::Bold)
        );
        assert_eq!(effective.font_rules[0].font.as_deref(), Some("Victor Mono"));
        assert_eq!(
            effective.font_rules[0].font_attributes.weight.as_deref(),
            Some("Bold")
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rssh_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rssh_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_font_metrics_fields() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.config_reloaded_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(*event);
            true
        });
        let active_pane = app.app_shell.active_pane_id();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_config_overrides({
                font_size = 13.5,
                cell_width = 1.25,
                cell_widths = {
                  { first = 0x2606, last = 0x2606, width = 1 },
                  { first = 0xe000, last = 0xf8ff, width = 2 },
                },
                line_height = 1.5,
                font_antialias = 'Subpixel',
                font_hinting = 'VerticalSubpixel',
                font_rasterizer = 'Harfbuzz',
                font_colr_rasterizer = 'FreeType',
                font_shaper = 'Harfbuzz',
                harfbuzz_features = { 'liga=0', 'calt=0' },
              })
              local config = window:effective_config()
              local width = window:effective_config().cell_widths[2]
              window:set_right_status(
                'cell=' .. tostring(config.cell_width)
                  .. ' line=' .. tostring(config.line_height)
                  .. ' aa=' .. tostring(config.font_antialias)
                  .. ' hint=' .. tostring(config.font_hinting)
                  .. ' raster=' .. tostring(config.font_rasterizer)
                  .. ' colr=' .. tostring(config.font_colr_rasterizer)
                  .. ' shaper=' .. tostring(config.font_shaper)
                  .. ' hb=' .. tostring(config.harfbuzz_features[2])
                  .. ' width=' .. tostring(width.first)
                  .. '/' .. tostring(width.last)
                  .. '/' .. tostring(width.width)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides font metrics callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.font_size,
            NativeFontSize::from_millipoints(13_500)
        );
        assert_eq!(effective.cell_width, NativeCellWidth::from_per_mille(1_250));
        assert_eq!(
            effective.cell_widths,
            vec![
                NativeCellWidthOverride::new(0x2606, 0x2606, 1),
                NativeCellWidthOverride::new(0xe000, 0xf8ff, 2),
            ]
        );
        assert_eq!(
            effective.line_height,
            NativeLineHeight::from_per_mille(1_500)
        );
        assert_eq!(effective.font_antialias, NativeFontAntialias::Subpixel);
        assert_eq!(effective.font_hinting, NativeFontHinting::VerticalSubpixel);
        assert_eq!(effective.font_rasterizer, NativeFontRasterizer::Harfbuzz);
        assert_eq!(
            effective.font_colr_rasterizer,
            NativeFontRasterizer::FreeType
        );
        assert_eq!(effective.font_shaper, NativeFontShaper::Harfbuzz);
        assert_eq!(
            effective.harfbuzz_features,
            vec!["liga=0".to_owned(), "calt=0".to_owned()]
        );
        assert_eq!(
            app.right_status,
            "cell=1.25 line=1.5 aa=Subpixel hint=VerticalSubpixel raster=Harfbuzz colr=FreeType shaper=Harfbuzz hb=calt=0 width=57344/63743/2"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rssh_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rssh_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_font_locator_fields() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.config_reloaded_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(*event);
            true
        });
        let active_pane = app.app_shell.active_pane_id();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_config_overrides({
                font_dirs = { 'fonts', 'vendor/fonts' },
                font_locator = 'ConfigDirsOnly',
              })
              window:set_right_status(
                'dir=' .. tostring(window:effective_config().font_dirs[2])
                  .. ' locator=' .. tostring(window:effective_config().font_locator)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides font locator callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.font_dirs,
            vec!["fonts".to_owned(), "vendor/fonts".to_owned()]
        );
        assert_eq!(
            effective.font_locator,
            Some(NativeFontLocator::ConfigDirsOnly)
        );
        assert_eq!(app.right_status, "dir=vendor/fonts locator=ConfigDirsOnly");
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rssh_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rssh_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_font_render_fields() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.config_reloaded_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(*event);
            true
        });
        let active_pane = app.app_shell.active_pane_id();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_config_overrides({
                font_size = 13.5,
                use_cap_height_to_scale_fallback_fonts = true,
                ignore_svg_fonts = true,
                sort_fallback_fonts_by_coverage = true,
                search_font_dirs_for_fallback = true,
                custom_block_glyphs = false,
                anti_alias_custom_block_glyphs = false,
                allow_square_glyphs_to_overflow_width = 'Always',
                freetype_load_target = 'Light',
                freetype_render_target = 'HorizontalLcd',
                freetype_load_flags = 'NO_HINTING|MONOCHROME',
                freetype_interpreter_version = 38,
                freetype_pcf_long_family_names = true,
                display_pixel_geometry = 'BGR',
              })
              local config = window:effective_config()
              window:set_right_status(
                'cap=' .. tostring(config.use_cap_height_to_scale_fallback_fonts)
                  .. ' ignore=' .. tostring(config.ignore_svg_fonts)
                  .. ' sort=' .. tostring(config.sort_fallback_fonts_by_coverage)
                  .. ' search=' .. tostring(config.search_font_dirs_for_fallback)
                  .. ' blocks=' .. tostring(config.custom_block_glyphs)
                  .. '/' .. tostring(config.anti_alias_custom_block_glyphs)
                  .. ' overflow=' .. tostring(config.allow_square_glyphs_to_overflow_width)
                  .. ' ft=' .. tostring(config.freetype_load_target)
                  .. '/' .. tostring(config.freetype_render_target)
                  .. '/' .. tostring(config.freetype_load_flags)
                  .. '/' .. tostring(config.freetype_interpreter_version)
                  .. '/' .. tostring(config.freetype_pcf_long_family_names)
                  .. ' geometry=' .. tostring(config.display_pixel_geometry)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides font render callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.font_size,
            NativeFontSize::from_millipoints(13_500)
        );
        assert!(effective.use_cap_height_to_scale_fallback_fonts);
        assert!(effective.ignore_svg_fonts);
        assert!(effective.sort_fallback_fonts_by_coverage);
        assert!(effective.search_font_dirs_for_fallback);
        assert!(!effective.custom_block_glyphs);
        assert!(!effective.anti_alias_custom_block_glyphs);
        assert_eq!(
            effective.allow_square_glyphs_to_overflow_width,
            NativeSquareGlyphOverflow::Always
        );
        assert_eq!(effective.freetype_load_target, NativeFreetypeTarget::Light);
        assert_eq!(
            effective.freetype_render_target,
            NativeFreetypeTarget::HorizontalLcd
        );
        assert_eq!(
            effective.freetype_load_flags,
            NativeFreetypeLoadFlags::NO_HINTING.union(NativeFreetypeLoadFlags::MONOCHROME)
        );
        assert_eq!(effective.freetype_interpreter_version, Some(38));
        assert!(effective.freetype_pcf_long_family_names);
        assert_eq!(
            effective.display_pixel_geometry,
            NativeDisplayPixelGeometry::Bgr
        );
        assert_eq!(
            app.right_status,
            "cap=true ignore=true sort=true search=true blocks=false/false overflow=Always ft=Light/HorizontalLcd/NO_HINTING|MONOCHROME/38/true geometry=BGR"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rssh_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rssh_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_background_visual_fields() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.config_reloaded_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(*event);
            true
        });
        let active_pane = app.app_shell.active_pane_id();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_config_overrides({
                foreground_text_hsb = {
                  hue = 0.5,
                  saturation = 0.75,
                  brightness = 1.25,
                },
                inactive_pane_hsb = {
                  hue = 1.5,
                  saturation = 0.25,
                  brightness = 0.5,
                },
                text_background_opacity = 0.4,
                window_background_opacity = 0.6,
                kde_window_background_blur = true,
                macos_window_background_blur = 20,
                win32_system_backdrop = 'Mica',
                win32_acrylic_accent_color = '#112233',
              })
              local config = window:effective_config()
              window:set_right_status(
                'fg=' .. tostring(config.foreground_text_hsb.hue)
                  .. '/' .. tostring(config.foreground_text_hsb.saturation)
                  .. '/' .. tostring(config.foreground_text_hsb.brightness)
                  .. ' inactive=' .. tostring(config.inactive_pane_hsb.hue)
                  .. '/' .. tostring(config.inactive_pane_hsb.saturation)
                  .. '/' .. tostring(config.inactive_pane_hsb.brightness)
                  .. ' opacity=' .. tostring(config.text_background_opacity)
                  .. '/' .. tostring(config.window_background_opacity)
                  .. ' blur=' .. tostring(config.kde_window_background_blur)
                  .. '/' .. tostring(config.macos_window_background_blur)
                  .. ' backdrop=' .. tostring(config.win32_system_backdrop)
                  .. ' accent=' .. tostring(config.win32_acrylic_accent_color)
              )
            end)
            "##,
        )
        .expect("expected WezTerm set_config_overrides background visual callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.foreground_text_hsb,
            NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(0.5),
                saturation: NativeHsbMultiplier::from_f32(0.75),
                brightness: NativeHsbMultiplier::from_f32(1.25),
            }
        );
        assert_eq!(
            effective.inactive_pane_hsb,
            NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.5),
                saturation: NativeHsbMultiplier::from_f32(0.25),
                brightness: NativeHsbMultiplier::from_f32(0.5),
            }
        );
        assert_eq!(
            effective.text_background_opacity,
            NativeTextBackgroundOpacity::from_f32(0.4)
        );
        assert_eq!(
            effective.window_background_opacity,
            NativeTextBackgroundOpacity::from_f32(0.6)
        );
        assert!(effective.kde_window_background_blur);
        assert_eq!(effective.macos_window_background_blur, 20);
        assert_eq!(
            effective.win32_system_backdrop,
            NativeWin32SystemBackdrop::Mica
        );
        assert_eq!(
            effective.win32_acrylic_accent_color,
            Some(Color::Rgb(17, 34, 51))
        );
        assert_eq!(
            app.right_status,
            "fg=0.5/0.75/1.25 inactive=1.5/0.25/0.5 opacity=0.4/0.6 blur=true/20 backdrop=Mica accent=#112233"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rssh_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rssh_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }
