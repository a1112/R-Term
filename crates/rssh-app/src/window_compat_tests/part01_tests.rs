    use super::{
        NativeConfigLifecycle, NativeSshWriter, PaneRenderRect, PaneRuntimeTransportKind,
        PresentationOwner,
        deferred_gpu_initialization_owner, presentation_owner_after_gpu_frame,
        validate_cli_config_overrides,
    };
    use std::collections::{BTreeMap, HashMap, HashSet};
    use std::io::{self, Write};
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command};
    #[cfg(target_os = "windows")]
    use std::process::Stdio;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    };
    use std::thread;
    use std::time::{Duration, Instant};

    use base64::{Engine, engine::general_purpose::STANDARD};
    use winit::dpi::{PhysicalPosition, PhysicalSize};
    use winit::event::{ElementState, MouseButton, MouseScrollDelta};
    use winit::keyboard::{Key, KeyCode as WinitKeyCode, ModifiersState, NamedKey, PhysicalKey};
    use winit::window::CursorIcon;

    use rssh_core::{
        TerminalSize,
        app_shell::{
            PaneLaunch, SshAuthDescription, SshKnownHostsPolicy, SshPaneLaunch, SplitDirection,
        },
    };
    use rssh_pty::{PtyCommand, PtyExitStatus, PtySession, PtySize};
    use rterm_render_core::{RenderGeometry, SCROLLBAR_THUMB_COLOR, TerminalRenderSnapshot};
    use rterm_render_cpu::{
        RenderBackgroundImageAttachment, RenderScrollbarThumbSize, color_to_rgba,
    };
    use rterm_render_wgpu::gpu::GpuFrameStatus;
    use rssh_terminal::{
        Color, CursorShape, StableRowIndex, StableSelectionCoordinate, StableSelectionRange,
        Terminal, TerminalScreenDomain,
    };

    use crate::{
        cli::{Osc52Policy, WindowConfigOptions, WindowOptions},
        config_lifecycle::ConfigDiscoveryInputs,
        terminal_modes::{MouseInputMode, MouseProtocolMode, MouseReportingMode},
        terminal_runtime::TerminalNotification,
        window::builtin_color_scheme_toml,
    };

    #[cfg(debug_assertions)]
    use crate::window::parse_test_window_scale_factor;

    #[test]
    fn native_ssh_writer_discards_input_before_connection_without_caching() {
        let (sender, receiver) = mpsc::channel();
        let mut writer = NativeSshWriter {
            sender,
            connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        writer.write_all(b"not-replayed").unwrap();
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn pane_runtime_transport_kind_moves_with_the_active_pane_owner() {
        let mut app = NativeWindowApp::new(None);
        assert_eq!(app.active_runtime_transport, None);
        let mut runtime = app.new_inactive_pane_runtime();
        runtime.transport = Some(PaneRuntimeTransportKind::NativeSsh);

        app.install_active_runtime(runtime);
        assert_eq!(
            app.active_runtime_transport,
            Some(PaneRuntimeTransportKind::NativeSsh)
        );

        let runtime = app.take_active_runtime();
        assert_eq!(runtime.transport, Some(PaneRuntimeTransportKind::NativeSsh));
        assert_eq!(app.active_runtime_transport, None);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn ssh_title_exposes_target_and_connection_status_without_secrets() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.set_initial_pane_launch(PaneLaunch::ssh(
            SshPaneLaunch::new(
                "ops@example.test:2222",
                SshAuthDescription::PasswordPrompt,
                SshKnownHostsPolicy::Prompt,
            )
            .with_remote_command(["uname", "-a"]),
        ));

        let title = app.effective_window_title();
        assert!(title.contains("SSH ops@example.test:2222"));
        assert!(title.contains("[pending]"));
        assert!(!title.contains("PasswordPrompt"));
    }

    #[test]
    fn ssh_write_error_preserves_the_pane_and_terminal_presentation() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.set_initial_pane_launch(PaneLaunch::ssh(SshPaneLaunch::new(
            "ops@example.test:2222",
            SshAuthDescription::PasswordPrompt,
            SshKnownHostsPolicy::Prompt,
        )));
        app.runtime.resize(TerminalSize::new(24, 2));
        app.handle_pty_output(b"preserved scrollback").unwrap();
        app.active_runtime_transport = Some(PaneRuntimeTransportKind::NativeSsh);
        let pane = app.active_pane_id();
        let panes_before = app.app_shell.pane_ids();
        let snapshot_before = app.snapshot.clone();

        let close_window = app.handle_pane_runtime_write_error(pane, "network unavailable");

        assert!(!close_window, "SSH transport errors must not close the GUI window");
        assert_eq!(app.app_shell.pane_ids(), panes_before);
        assert_eq!(app.snapshot, snapshot_before);
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn deferred_ssh_start_error_marks_failed_without_exiting_or_clearing_output() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.set_initial_pane_launch(PaneLaunch::ssh(SshPaneLaunch::new(
            "ops@example.test:2222",
            SshAuthDescription::PasswordPrompt,
            SshKnownHostsPolicy::Prompt,
        )));
        app.handle_pty_output(b"bootstrap output").unwrap();
        let snapshot_before = app.snapshot.clone();

        let should_exit = app.handle_transport_start_error(&io::Error::other("spawn failed"));

        assert!(!should_exit);
        assert_eq!(app.snapshot, snapshot_before);
        assert_eq!(
            app.ssh_connection_state_for_pane(app.active_pane_id()),
            crate::startup_metrics::ConnectionState::Failed
        );
    }

    #[test]
    fn ime_cursor_area_tracks_tab_bar_and_split_pane_offsets() {
        let rect = PaneRenderRect {
            pane_id: rssh_core::PaneId::new(2),
            row: 1,
            column: 17,
            rows: 10,
            columns: 20,
        };
        let (position, size) = NativeWindowApp::ime_cursor_area_pixels(
            14, 8, 9, 18, rect, 3, 4,
        )
        .expect("non-empty pane has an IME area");

        assert_eq!(position, PhysicalPosition::new(14 + 17 * 9 + 4 * 9, 8 + 18 + 3 * 18));
        assert_eq!(size, PhysicalSize::new(9, 18));
    }

    #[test]
    fn modern_default_palette_and_padding() {
        let app = NativeWindowApp::new_with_visual_defaults(None);
        let palette = app.native_resolved_palette();

        assert_eq!(
            palette.foreground,
            Color::Rgb(0xd8, 0xe2, 0xf0),
            "modern terminal foreground"
        );
        assert_eq!(
            palette.background,
            Color::Rgb(0x0b, 0x12, 0x20),
            "modern terminal background"
        );
        assert_eq!(
            palette.cursor_bg,
            Color::Rgb(0x67, 0xe8, 0xf9),
            "modern terminal cursor"
        );
        assert_eq!(
            palette.cursor_fg,
            Some(Color::Rgb(0x0b, 0x12, 0x20)),
            "modern terminal cursor foreground"
        );
        assert_eq!(
            palette.selection_bg,
            Some(Color::Rgba(0x33, 0x41, 0x55, 0xb3)),
            "modern terminal selection background"
        );
        assert_eq!(
            app.window_padding,
            NativeWindowPadding {
                left: NativeWindowPaddingDimension::Pixels(14),
                right: NativeWindowPaddingDimension::Pixels(14),
                top: NativeWindowPaddingDimension::Pixels(10),
                bottom: NativeWindowPaddingDimension::Pixels(10),
            },
            "modern terminal content padding"
        );

        assert_eq!(
            palette.ansi,
            [
                Color::Rgb(0x11, 0x18, 0x27),
                Color::Rgb(0xf8, 0x71, 0x71),
                Color::Rgb(0x4a, 0xde, 0x80),
                Color::Rgb(0xfb, 0xbf, 0x24),
                Color::Rgb(0x60, 0xa5, 0xfa),
                Color::Rgb(0xc0, 0x84, 0xfc),
                Color::Rgb(0x22, 0xd3, 0xee),
                Color::Rgb(0xcb, 0xd5, 0xe1),
            ],
            "modern ANSI base palette"
        );
        assert_eq!(
            palette.brights,
            [
                Color::Rgb(0x64, 0x74, 0x8b),
                Color::Rgb(0xfb, 0x71, 0x85),
                Color::Rgb(0x86, 0xef, 0xac),
                Color::Rgb(0xfd, 0xe0, 0x47),
                Color::Rgb(0x93, 0xc5, 0xfd),
                Color::Rgb(0xd8, 0xb4, 0xfe),
                Color::Rgb(0x67, 0xe8, 0xf9),
                Color::Rgb(0xf8, 0xfa, 0xfc),
            ],
            "modern ANSI bright palette"
        );
    }

    #[test]
    fn modern_default_overrides_retain_color_and_padding_precedence() {
        let override_ansi = [Color::Rgb(1, 2, 3); 16];
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.set_config_overrides(native_config_snapshot! {
            foreground_color: Some(Color::Rgb(4, 5, 6)),
            background_color: Some(Color::Rgb(7, 8, 9)),
            cursor_bg_color: Some(Color::Rgb(10, 11, 12)),
            ansi_palette: Some(override_ansi),
            window_padding: Some(NativeWindowPadding {
                left: NativeWindowPaddingDimension::Pixels(1),
                right: NativeWindowPaddingDimension::Pixels(2),
                top: NativeWindowPaddingDimension::Pixels(3),
                bottom: NativeWindowPaddingDimension::Pixels(4),
            }),
            ..NativeConfigSnapshot::default()
        });

        let palette = app.native_resolved_palette();
        assert_eq!(palette.foreground, Color::Rgb(4, 5, 6));
        assert_eq!(palette.background, Color::Rgb(7, 8, 9));
        assert_eq!(palette.cursor_bg, Color::Rgb(10, 11, 12));
        assert_eq!(palette.ansi, [Color::Rgb(1, 2, 3); 8]);
        assert_eq!(palette.brights, [Color::Rgb(1, 2, 3); 8]);
        assert_eq!(
            app.window_padding,
            NativeWindowPadding {
                left: NativeWindowPaddingDimension::Pixels(1),
                right: NativeWindowPaddingDimension::Pixels(2),
                top: NativeWindowPaddingDimension::Pixels(3),
                bottom: NativeWindowPaddingDimension::Pixels(4),
            }
        );
    }

    #[test]
    fn modern_default_cursor_and_selection_survive_empty_overrides() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.set_config_overrides(NativeConfigSnapshot::default());

        let palette = app.native_resolved_palette();
        assert_eq!(palette.cursor_fg, Some(Color::Rgb(0x0b, 0x12, 0x20)));
        assert_eq!(
            palette.selection_bg,
            Some(Color::Rgba(0x33, 0x41, 0x55, 0xb3))
        );
    }

    #[test]
    fn modern_default_tab_bar_colors() {
        let app = NativeWindowApp::new_with_visual_defaults(None);
        let palette = app.native_resolved_palette();

        assert_eq!(
            palette.tab_bar_background,
            Some(Color::Rgb(0x08, 0x0d, 0x18)),
            "modern tab bar background"
        );
        assert_eq!(
            palette.tab_bar_active_tab.test_projection(),
            (
                Some(Color::Rgb(0xf8, 0xfa, 0xfc)),
                Some(Color::Rgb(0x1b, 0x2b, 0x44)),
                Some("Normal"),
                None,
                None,
                None,
            ),
            "modern active tab style"
        );
        assert_eq!(
            palette.tab_bar_inactive_tab.test_projection(),
            (
                Some(Color::Rgb(0x84, 0x92, 0xa6)),
                Some(Color::Rgb(0x10, 0x18, 0x27)),
                None,
                None,
                None,
                None,
            ),
            "modern inactive tab style"
        );
        assert_eq!(
            palette.tab_bar_inactive_tab_hover.test_projection(),
            (
                Some(Color::Rgb(0xd8, 0xe2, 0xf0)),
                Some(Color::Rgb(0x1e, 0x29, 0x3b)),
                None,
                None,
                None,
                None,
            ),
            "modern hovered tab style"
        );
        assert_eq!(
            palette.tab_bar_new_tab.test_projection(),
            (
                Some(Color::Rgb(0xd8, 0xe2, 0xf0)),
                Some(Color::Rgb(0x08, 0x0d, 0x18)),
                None,
                None,
                None,
                None,
            ),
            "modern new-tab button style"
        );
        assert_eq!(
            app.window_frame_title_bar_background_color(),
            Color::Rgb(0x08, 0x0d, 0x18),
            "window frame should inherit the modern tab bar background"
        );
        assert_eq!(
            app.window_frame_title_bar_foreground_color(),
            Color::Rgb(0xd8, 0xe2, 0xf0),
            "window frame should inherit the modern terminal foreground"
        );
    }

    #[test]
    fn modern_default_tab_bar_explicit_colors_override_each_value() {
        let explicit_active = NativeTabBarItemColors {
            fg_color: Some(Color::Rgb(1, 2, 3)),
            bg_color: Some(Color::Rgb(4, 5, 6)),
            intensity: Some(NativeFormatIntensity::Normal),
            underline: Some(NativeFormatUnderline::Single),
            italic: Some(true),
            strikethrough: Some(true),
        };
        let explicit_inactive = NativeTabBarItemColors {
            fg_color: Some(Color::Rgb(7, 8, 9)),
            bg_color: Some(Color::Rgb(10, 11, 12)),
            intensity: Some(NativeFormatIntensity::Half),
            underline: Some(NativeFormatUnderline::Double),
            italic: Some(false),
            strikethrough: Some(false),
        };
        let explicit_hover = NativeTabBarItemColors {
            fg_color: Some(Color::Rgb(13, 14, 15)),
            bg_color: Some(Color::Rgb(16, 17, 18)),
            intensity: Some(NativeFormatIntensity::Bold),
            underline: Some(NativeFormatUnderline::Curly),
            italic: Some(true),
            strikethrough: Some(false),
        };
        let explicit_new_tab = NativeTabBarItemColors {
            fg_color: Some(Color::Rgb(19, 20, 21)),
            bg_color: Some(Color::Rgb(22, 23, 24)),
            intensity: Some(NativeFormatIntensity::Normal),
            underline: Some(NativeFormatUnderline::Dotted),
            italic: Some(false),
            strikethrough: Some(true),
        };

        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.set_config_overrides(native_config_snapshot! {
            tab_bar_background_color: Some(Color::Rgb(25, 26, 27)),
            tab_bar_active_tab_colors: explicit_active,
            tab_bar_inactive_tab_colors: explicit_inactive,
            tab_bar_inactive_tab_hover_colors: explicit_hover,
            tab_bar_new_tab_colors: explicit_new_tab,
            ..NativeConfigSnapshot::default()
        });

        let palette = app.native_resolved_palette();
        assert_eq!(palette.tab_bar_background, Some(Color::Rgb(25, 26, 27)));
        assert_eq!(palette.tab_bar_active_tab, explicit_active);
        assert_eq!(palette.tab_bar_inactive_tab, explicit_inactive);
        assert_eq!(palette.tab_bar_inactive_tab_hover, explicit_hover);
        assert_eq!(palette.tab_bar_new_tab, explicit_new_tab);
    }

    #[test]
    fn modern_default_tab_bar_uses_native_window_glyphs() {
        assert_eq!(
            integrated_title_button_default_tab_bar_label(NativeIntegratedTitleButton::Hide),
            " — "
        );
        assert_eq!(
            integrated_title_button_default_tab_bar_label(NativeIntegratedTitleButton::Maximize),
            " □ "
        );
        assert_eq!(
            integrated_title_button_default_tab_bar_label(NativeIntegratedTitleButton::Close),
            " × "
        );
    }

    #[test]
    fn modern_default_window_controls_keep_target_spacing() {
        let app = NativeWindowApp::new_with_visual_defaults(None);
        for button in [
            NativeIntegratedTitleButton::Hide,
            NativeIntegratedTitleButton::Maximize,
            NativeIntegratedTitleButton::Close,
        ] {
            assert_eq!(
                super::native_format_items_visible_width(
                    &app.integrated_title_button_tab_bar_items(button, false)
                ),
                5,
                "modern default window control should reserve two breathing columns per side"
            );
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn modern_default_window_controls_use_bright_foreground() {
        let app = NativeWindowApp::new_with_visual_defaults(None);
        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let hide_column = tab_bar
            .find('—')
            .and_then(|column| u16::try_from(column).ok())
            .expect("modern hide button should be visible");
        assert_eq!(
            snapshot_cell(&snapshot, 0, hide_column)
                .expect("modern hide button cell should be visible")
                .foreground,
            Color::Rgb(0xf8, 0xfa, 0xfc),
            "modern window controls should use the bright title-bar foreground"
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn modern_default_window_controls_use_surface_on_hover() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        let snapshot = app.render_snapshot();
        let close_column = (0..TERMINAL_COLUMNS)
            .find(|column| {
                app.integrated_title_button_for_tab_bar_column(*column)
                    == Some(NativeIntegratedTitleButton::Close)
                    && snapshot_cell(&snapshot, 0, *column)
                        .is_some_and(|cell| cell.ch == '×')
            })
            .expect("modern close control should have a tab-bar hit column");
        let resting = snapshot_cell(&snapshot, 0, close_column)
            .expect("resting modern close control should be visible");
        assert_eq!(resting.ch, '×');
        assert_eq!(resting.background, Color::Rgb(0x08, 0x0d, 0x18));
        assert_eq!(resting.foreground, Color::Rgb(0xf8, 0xfa, 0xfc));

        let x = app
            .frame_content_pixel_left()
            .saturating_add(
                u32::from(close_column).saturating_mul(super::MODERN_CELL_WIDTH),
            )
            .saturating_add(super::MODERN_CELL_WIDTH / 2);
        let y = app
            .frame_content_pixel_top()
            .saturating_add(super::MODERN_CELL_HEIGHT / 2);
        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), f64::from(y)))
            .expect("window control hover should be accepted");
        let hovered_snapshot = app.render_snapshot();
        let hovered = snapshot_cell(&hovered_snapshot, 0, close_column)
            .expect("hovered modern close control should be visible");

        assert_eq!(hovered.ch, '×');
        assert_eq!(hovered.foreground, Color::Rgb(0xf8, 0xfa, 0xfc));
        assert_eq!(
            hovered.background,
            Color::Rgb(0x1e, 0x29, 0x3b),
            "modern window controls should lift onto a slate hover surface"
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn modern_default_active_tab_paints_breathing_room_without_moving_hits() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.handle_pty_output(b"\x1b]2;Command Prompt\x07")
            .expect("default shell title should be accepted");
        assert_eq!(app.modern_tab_bar_brand_label(), Some(" [>_] R-SSH "));
        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("R-SSH"));
        assert_eq!(
            tab_bar.matches("R-SSH").count(),
            2,
            "modern chrome should show the brand and default tab title: {tab_bar:?}"
        );
        assert!(
            !tab_bar.contains("Command Prompt"),
            "default shell title should use the product label: {tab_bar:?}"
        );
        let prompt_column = tab_bar
            .find(">_")
            .expect("modern brand prompt mark should be visible");
        assert_eq!(
            snapshot_cell(&snapshot, 0, u16::try_from(prompt_column).unwrap())
                .expect("prompt mark cell should be visible")
                .foreground,
            Color::Rgb(0xd8, 0xe2, 0xf0),
            "modern badge prompt should use the readable terminal foreground"
        );
        assert_eq!(
            snapshot_cell(&snapshot, 0, u16::try_from(prompt_column).unwrap())
                .expect("prompt mark cell should be visible")
                .background,
            Color::Rgb(0x1b, 0x2b, 0x44)
        );
        assert_eq!(
            snapshot_cell(&snapshot, 0, u16::try_from(prompt_column - 1).unwrap())
                .expect("badge leading bracket cell should be visible")
                .background,
            Color::Rgb(0x1b, 0x2b, 0x44)
        );
        assert_eq!(
            snapshot_cell(&snapshot, 0, u16::try_from(prompt_column - 1).unwrap())
                .expect("badge leading bracket cell should be visible")
                .foreground,
            Color::Rgb(0x38, 0xbd, 0xf8),
            "modern badge outline should keep its cyan accent"
        );
        assert_eq!(
            snapshot_cell(&snapshot, 0, u16::try_from(prompt_column + 2).unwrap())
                .expect("badge trailing bracket cell should be visible")
                .background,
            Color::Rgb(0x1b, 0x2b, 0x44)
        );
        assert_eq!(
            snapshot_cell(&snapshot, 0, u16::try_from(prompt_column + 2).unwrap())
                .expect("badge trailing bracket cell should be visible")
                .foreground,
            Color::Rgb(0x38, 0xbd, 0xf8),
            "modern badge outline should keep its cyan accent"
        );
        assert!(
            snapshot_cell(&snapshot, 0, u16::try_from(prompt_column - 1).unwrap())
                .expect("badge leading bracket cell should be visible")
                .bold,
            "modern badge outline should carry the stronger edge treatment"
        );
        assert!(
            !snapshot_cell(&snapshot, 0, u16::try_from(prompt_column).unwrap())
                .expect("prompt mark cell should be visible")
                .bold,
            "modern badge prompt should stay at normal weight inside the outline"
        );
        assert!(
            snapshot_cell(&snapshot, 0, u16::try_from(prompt_column + 2).unwrap())
                .expect("badge trailing bracket cell should be visible")
                .bold,
            "modern badge outline should carry the stronger edge treatment"
        );
        assert_eq!(
            snapshot_cell(&snapshot, 0, u16::try_from(prompt_column - 2).unwrap())
                .expect("badge leading spacer cell should be visible")
                .background,
            Color::Rgb(0x08, 0x0d, 0x18)
        );
        let product_column = tab_bar
            .find("R-SSH")
            .expect("modern product name should be visible");
        assert_eq!(
            snapshot_cell(&snapshot, 0, u16::try_from(product_column).unwrap())
                .expect("product name cell should be visible")
                .foreground,
            Color::Rgb(0xf8, 0xfa, 0xfc),
            "modern product name should share the high-emphasis title foreground"
        );
        assert!(
            !snapshot_cell(&snapshot, 0, u16::try_from(product_column).unwrap())
                .expect("product name cell should be visible")
                .bold,
            "modern product name should stay at normal weight beside the outlined badge"
        );
        assert!(!tab_bar.contains("panes:"), "modern tab bar was {tab_bar:?}");
        assert!(tab_bar.contains('×'), "modern tab close marker was {tab_bar:?}");
        assert!(tab_bar.contains('▾'), "modern new-tab chevron was {tab_bar:?}");
        let plus_column = tab_bar
            .chars()
            .position(|character| character == '+')
            .expect("modern new-tab plus marker should be visible");
        assert_eq!(
            snapshot_cell(&snapshot, 0, u16::try_from(plus_column).unwrap())
                .expect("new-tab plus cell should be visible")
                .foreground,
            Color::Rgb(0xf8, 0xfa, 0xfc),
            "modern new-tab plus should share the high-emphasis title foreground"
        );
        let chevron_column = tab_bar
            .chars()
            .position(|character| character == '▾')
            .expect("modern new-tab chevron should be visible");
        assert!(
            chevron_column >= plus_column.saturating_add(3),
            "modern new-tab controls should keep a breathing column between + and ▾: +={plus_column}, ▾={chevron_column}"
        );
        assert_eq!(
            snapshot_cell(&snapshot, 0, u16::try_from(chevron_column).unwrap())
                .expect("new-tab chevron cell should be visible")
                .foreground,
            Color::Rgb(0xa5, 0xb4, 0xc7)
        );
        let layout = app.rendered_tab_bar_layout.borrow();
        assert_eq!(
            layout
                .as_ref()
                .and_then(|layout| layout.new_tab_end_column),
            Some(u16::try_from(chevron_column).unwrap()),
            "modern chevron should begin immediately after the interactive + segment"
        );
        assert_eq!(
            layout
                .as_ref()
                .and_then(|layout| layout.new_tab_start_column)
                .zip(layout.as_ref().and_then(|layout| layout.new_tab_end_column))
                .map(|(start, end)| end.saturating_sub(start)),
            Some(4),
            "modern + control should reserve its leading/trailing breathing columns"
        );
        let tab = layout
            .as_ref()
            .and_then(|layout| layout.tabs.first())
            .expect("default tab should be laid out");
        assert_eq!(tab.label.prefix, "  ");
        assert_eq!(tab.label.suffix, "            ×  ");
        assert_eq!(
            tab.close_column,
            Some(tab.end_column.saturating_sub(3)),
            "modern close target should sit near the active tile's trailing edge"
        );
        assert!(
            tab.end_column.saturating_sub(tab.start_column) >= 22,
            "modern active tab should retain target-like horizontal breathing room: {}..{}",
            tab.start_column,
            tab.end_column
        );
        let margin_column = tab.start_column.saturating_sub(1);

        assert_eq!(
            snapshot_cell(&snapshot, 0, margin_column)
                .expect("active tab margin cell should be visible")
                .background,
            Color::Rgb(0x1b, 0x2b, 0x44),
            "modern active tab should use the concept's visible blue surface"
        );
        assert_eq!(
            snapshot_cell(&snapshot, 0, tab.start_column)
                .expect("active tab leading corner cell should be visible")
                .background,
            Color::Rgb(0x08, 0x0d, 0x18),
            "modern active tile should clip its leading corner"
        );
        assert_eq!(
            snapshot_cell(&snapshot, 0, tab.end_column.saturating_sub(1))
                .expect("active tab trailing corner cell should be visible")
                .background,
            Color::Rgb(0x08, 0x0d, 0x18),
            "modern active tile should clip its trailing corner"
        );
        assert_eq!(app.tab_for_tab_bar_column(margin_column), None);
        assert_eq!(app.tab_for_tab_bar_column(tab.start_column), Some(tab.tab_id));
    }

    #[test]
    fn modern_default_tab_close_hover_uses_destructive_surface() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        let snapshot = app.render_snapshot();
        let close_column = app
            .rendered_tab_bar_layout
            .borrow()
            .as_ref()
            .and_then(|layout| layout.tabs.first())
            .and_then(|tab| tab.close_column)
            .expect("modern default tab should expose a close target");

        let resting_cell = snapshot_cell(&snapshot, 0, close_column)
            .expect("resting tab close cell should be visible");
        assert_eq!(resting_cell.ch, '×');
        assert_eq!(resting_cell.foreground, Color::Rgb(0xf8, 0xfa, 0xfc));
        assert_eq!(resting_cell.background, Color::Rgb(0x1b, 0x2b, 0x44));

        let x = app
            .frame_content_pixel_left()
            .saturating_add(
                u32::from(close_column).saturating_mul(super::MODERN_CELL_WIDTH),
            )
            .saturating_add(super::MODERN_CELL_WIDTH / 2);
        let y = app
            .frame_content_pixel_top()
            .saturating_add(super::MODERN_CELL_HEIGHT / 2);
        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), f64::from(y)))
            .expect("tab close hover should be accepted");
        let hovered_snapshot = app.render_snapshot();
        let hovered_cell = snapshot_cell(&hovered_snapshot, 0, close_column)
            .expect("hovered tab close cell should be visible");

        assert_eq!(hovered_cell.ch, '×');
        assert_eq!(
            hovered_cell.foreground,
            Color::Rgb(0x0b, 0x12, 0x20),
            "hovered tab close should use the dark modern foreground"
        );
        assert_eq!(
            hovered_cell.background,
            Color::Rgb(0xf8, 0x71, 0x71),
            "hovered tab close should use the modern destructive surface"
        );
    }

    #[test]
    fn modern_default_compact_header_hides_default_workspace_label() {
        let app = NativeWindowApp::new_with_visual_defaults(None);

        let tab_bar = snapshot_row_text(&app.render_snapshot(), 0, TERMINAL_COLUMNS);

        assert!(
            !tab_bar.contains("ws:default"),
            "modern compact header should reserve the workspace slot for the concept hierarchy: {tab_bar:?}"
        );
    }

    #[test]
    fn modern_compact_header_keeps_custom_workspace_label() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        let workspace = app.app_shell.active_workspace_id();
        app.app_shell
            .apply_action(AppAction::RenameWorkspace {
                workspace,
                name: "ops".to_owned(),
            })
            .expect("workspace rename should succeed");

        let tab_bar = snapshot_row_text(&app.render_snapshot(), 0, TERMINAL_COLUMNS);

        assert!(tab_bar.contains("ws:ops"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn modern_default_header_separates_brand_from_active_tab() {
        let app = NativeWindowApp::new_with_visual_defaults(None);
        let snapshot = app.render_snapshot();
        let tab_start_column = app
            .rendered_tab_bar_layout
            .borrow()
            .as_ref()
            .and_then(|layout| layout.tabs.first())
            .map(|tab| tab.start_column)
            .expect("default tab should be laid out");
        let brand_width = app
            .modern_tab_bar_brand_label()
            .expect("modern brand should be present")
            .chars()
            .count();

        assert_eq!(
            usize::from(tab_start_column),
            app.macos_native_integrated_title_button_spacer_width()
                + usize::from(super::MODERN_TAB_BAR_BRAND_INSET_COLUMNS)
                + brand_width
                + 3
        );
        let brand_end_column = app.macos_native_integrated_title_button_spacer_width()
            + usize::from(super::MODERN_TAB_BAR_BRAND_INSET_COLUMNS)
            + brand_width;
        assert_eq!(
            snapshot_cell(&snapshot, 0, u16::try_from(brand_end_column).unwrap())
                .unwrap()
                .ch,
            ' '
        );
        assert_eq!(snapshot_cell(&snapshot, 0, 0).unwrap().ch, ' ');
        assert_eq!(snapshot_cell(&snapshot, 0, 1).unwrap().ch, ' ');
    }

    #[test]
    fn modern_default_header_shows_launch_program_before_osc_title() {
        let app = NativeWindowApp::new_with_visual_defaults(None);
        let snapshot = app.render_snapshot();
        let layout = app.rendered_tab_bar_layout.borrow();
        let tab = layout
            .as_ref()
            .and_then(|layout| layout.tabs.first())
            .expect("default tab should be laid out");

        assert!(
            !tab.title.plain_text().trim().is_empty(),
            "modern title bar should not leave a newly-created shell tab blank: {:?}",
            snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS)
        );
    }

    #[test]
    fn modern_default_tab_bar_preserves_custom_osc_title() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.handle_pty_output(b"\x1b]2;build-server\x07")
            .expect("custom OSC title should be accepted");

        let tab_bar = snapshot_row_text(&app.render_snapshot(), 0, TERMINAL_COLUMNS);

        assert!(tab_bar.contains("build-server"), "tab bar was {tab_bar:?}");
        assert_eq!(tab_bar.matches("R-SSH").count(), 1);
    }

    #[test]
    fn modern_default_font_matches_gpu_shaping_baseline() {
        let app = NativeWindowApp::new_with_visual_defaults(None);

        assert_eq!(app.font_size, NativeFontSize::from_millipoints(17_000));
        assert_eq!(app.cell_width(), 10);
        assert_eq!(app.cell_height(), 21);
    }

    #[test]
    fn modern_default_visual_density_matches_concept_target() {
        assert_eq!(
            super::MODERN_DEFAULT_FONT_SIZE,
            NativeFontSize::from_millipoints(17_000),
            "the concept uses a more readable default terminal scale"
        );
        assert_eq!(super::MODERN_CELL_WIDTH, 10);
        assert_eq!(super::MODERN_CELL_HEIGHT, 21);
        assert_eq!(
            super::MODERN_DEFAULT_WINDOW_PADDING,
            NativeWindowPadding {
                left: NativeWindowPaddingDimension::Pixels(14),
                right: NativeWindowPaddingDimension::Pixels(14),
                top: NativeWindowPaddingDimension::Pixels(10),
                bottom: NativeWindowPaddingDimension::Pixels(10),
            }
        );
        assert_eq!(super::MODERN_DEFAULT_TAB_MAX_WIDTH, 20);
    }

    #[test]
    fn modern_default_geometry_scales_for_high_dpi_displays() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);

        app.apply_window_scale_factor(2.0);

        assert_eq!(app.window_dpi, 192);
        assert!((app.gpu_dpi_scale() - 2.0).abs() < f32::EPSILON);
        assert_eq!(app.cell_width(), 20);
        assert_eq!(app.cell_height(), 42);
        assert_eq!(
            app.initial_frame_size(),
            PhysicalSize::new(1_656, 1_090),
            "80x24 modern content should retain its logical size at 200% DPI"
        );
        assert_eq!(
            terminal_size_from_window_pixels_with_padding(
                1_656,
                1_090,
                app.cell_width(),
                app.cell_height(),
                app.window_padding,
                app.window_dpi,
            ),
            TerminalSize::new(80, 24),
            "physical resizing must not double the PTY columns or rows"
        );
    }

    #[test]
    fn modern_default_window_decorations_match_platform_visual_target() {
        let app = NativeWindowApp::new_with_visual_defaults(None);

        #[cfg(target_os = "windows")]
        {
            assert_eq!(
                app.window_decorations,
                NativeWindowDecorations {
                    title: false,
                    resize: true,
                    integrated_buttons: true,
                    macos_force_disable_shadow: false,
                    macos_force_enable_shadow: false,
                    macos_force_square_corners: false,
                    macos_use_background_color_as_titlebar_color: false,
                }
            );
            assert!(
                !app.window_decorations.winit_decorations_enabled(),
                "integrated Windows buttons require a borderless surface"
            );
        }

        #[cfg(target_os = "macos")]
        assert_eq!(
            app.window_decorations,
            NativeWindowDecorations {
                title: true,
                resize: true,
                integrated_buttons: true,
                macos_force_disable_shadow: false,
                macos_force_enable_shadow: false,
                macos_force_square_corners: false,
                macos_use_background_color_as_titlebar_color: true,
            }
        );

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        assert_eq!(
            app.window_decorations,
            NativeWindowDecorations {
                title: true,
                resize: true,
                integrated_buttons: false,
                macos_force_disable_shadow: false,
                macos_force_enable_shadow: false,
                macos_force_square_corners: false,
                macos_use_background_color_as_titlebar_color: false,
            }
        );
    }

    #[test]
    fn macos_default_chrome_uses_unified_titlebar_and_native_shadow() {
        let policy = super::native_macos_window_chrome_policy_for_platform(
            "macos",
            NativeWindowDecorations {
                title: true,
                resize: true,
                integrated_buttons: true,
                macos_force_disable_shadow: false,
                macos_force_enable_shadow: false,
                macos_force_square_corners: false,
                macos_use_background_color_as_titlebar_color: true,
            },
        );

        assert!(policy.unified_titlebar);
        assert!(policy.has_shadow);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_fancy_tab_bar_reserves_native_traffic_light_space() {
        let app = NativeWindowApp::new_with_visual_defaults(None);

        assert_eq!(app.macos_native_integrated_title_button_spacer_width(), 10);
        assert!(app.tab_bar_provides_window_drag_region());
        assert!(app.use_fancy_tab_bar);
    }

    #[test]
    fn compact_terminal_tab_title_keeps_tabs_readable() {
        assert_eq!(
            compact_terminal_tab_title(r"C:\\WINDOWS\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"),
            "PowerShell"
        );
        assert_eq!(
            compact_terminal_tab_title(r"/usr/bin/pwsh.exe"),
            "PowerShell"
        );
        assert_eq!(
            compact_terminal_tab_title(r"C:\\Windows\\System32\\cmd.exe"),
            "Command Prompt"
        );
        assert_eq!(compact_terminal_tab_title("opencode"), "opencode");
        assert_eq!(compact_terminal_tab_title("  custom title  "), "custom title");
    }

    #[test]
    fn cursor_fallback_distinguishes_unconfigured_and_color_scheme_defaults() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.set_config_overrides(NativeConfigSnapshot::default());
        assert_eq!(
            app.native_effective_config().cursor_bg_color,
            DEFAULT_CURSOR_BG_COLOR
        );

        app.set_config_overrides(native_config_snapshot! {
            color_scheme: Some("Builtin Dark".to_owned()),
            ..NativeConfigSnapshot::default()
        });
        assert_eq!(
            app.native_effective_config().cursor_bg_color,
            LEGACY_COLOR_SCHEME_CURSOR_BG_COLOR
        );
    }

    #[test]
    fn custom_color_scheme_map_uses_legacy_cursor_fallback() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
                local config = {}
                config.color_schemes = {
                  ['Minimal'] = {
                    foreground = '#010203',
                    background = '#040506',
                  },
                }
                config.color_scheme = 'Minimal'
                return config
            "#,
        )
        .expect("expected custom color scheme without cursor_bg");

        let scheme = overrides
            .color_schemes
            .as_ref()
            .and_then(|schemes| schemes.get("Minimal"))
            .expect("expected Minimal custom color scheme");
        assert_eq!(scheme.cursor_bg, LEGACY_COLOR_SCHEME_CURSOR_BG_COLOR);

        app.set_config_overrides(overrides);
        assert_eq!(
            app.native_effective_config().cursor_bg_color,
            LEGACY_COLOR_SCHEME_CURSOR_BG_COLOR
        );
    }

    #[test]
    fn modern_default_geometry_uses_target_grid() {
        assert_eq!(super::MODERN_CELL_WIDTH, 10, "modern terminal cell width");
        assert_eq!(super::MODERN_CELL_HEIGHT, 21, "modern terminal cell height");
        assert_eq!(super::MODERN_FRAME_WIDTH, 800, "80-column frame width");
        assert_eq!(super::MODERN_FRAME_HEIGHT, 525, "24-row frame plus tab bar height");
    }

    #[test]
    fn modern_default_padding_is_outer_physical_margin() {
        let app = NativeWindowApp::new_with_visual_defaults(None);
        assert_eq!(
            app.initial_frame_size(),
            PhysicalSize::new(
                super::MODERN_FRAME_WIDTH + super::MODERN_WINDOW_PADDING_HORIZONTAL_PIXELS,
                super::MODERN_FRAME_HEIGHT + super::MODERN_WINDOW_PADDING_VERTICAL_PIXELS,
            )
        );

        let terminal_size = terminal_size_from_window_pixels_with_padding(
            super::MODERN_FRAME_WIDTH + super::MODERN_WINDOW_PADDING_HORIZONTAL_PIXELS,
            super::MODERN_FRAME_HEIGHT + super::MODERN_WINDOW_PADDING_VERTICAL_PIXELS,
            super::MODERN_CELL_WIDTH,
            super::MODERN_CELL_HEIGHT,
            super::MODERN_DEFAULT_WINDOW_PADDING,
            96,
        );
        assert_eq!(
            terminal_size,
            rssh_core::TerminalSize::new(TERMINAL_COLUMNS, TERMINAL_ROWS)
        );
    }

    #[test]
    fn modern_default_padding_does_not_consume_terminal_rows_or_columns() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        let frame_size = app.initial_frame_size();
        app.handle_window_resize(frame_size)
            .expect("default frame should resize");

        assert_eq!(
            app.frame_size_for_test(),
            (
                super::MODERN_FRAME_WIDTH + super::MODERN_WINDOW_PADDING_HORIZONTAL_PIXELS,
                super::MODERN_FRAME_HEIGHT + super::MODERN_WINDOW_PADDING_VERTICAL_PIXELS,
            )
        );
        assert_eq!(
            app.runtime.terminal().grid().size(),
            rssh_core::TerminalSize::new(TERMINAL_COLUMNS, TERMINAL_ROWS)
        );
    }

    #[test]
    fn modern_default_frame_chrome_border_stays_outside_terminal_content() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.handle_window_resize(app.initial_frame_size())
            .expect("default frame should resize");
        let (frame_width, frame_height) = app.frame_size_for_test();
        let mut frame =
            vec![0; usize::try_from(frame_width.saturating_mul(frame_height) * 4).unwrap()];

        assert_eq!(
            super::DEFAULT_WINDOW_CHROME_BORDER_RGBA,
            [0x47, 0x55, 0x69, 0xff],
            "modern chrome should use the concept's quiet slate outline"
        );
        assert_eq!(
            super::DEFAULT_TAB_BAR_SEPARATOR_RGBA,
            [0x2b, 0x3b, 0x53, 0xff],
            "modern chrome should keep the tab/content boundary visible"
        );
        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let width = usize::try_from(frame_width).unwrap();
        assert_eq!(
            frame_pixel_at(&frame, width, 0, 0),
            super::DEFAULT_RENDER_BACKGROUND_RGBA,
            "modern chrome should leave the outer frame corner rounded"
        );
        assert_eq!(
            frame_pixel_at(&frame, width, 1, 0),
            super::DEFAULT_WINDOW_CHROME_BORDER_RGBA,
            "modern chrome should begin the rounded border one pixel inward"
        );
        assert_eq!(
            frame_pixel_at(&frame, width, 0, 1),
            super::DEFAULT_WINDOW_CHROME_BORDER_RGBA,
            "modern chrome should preserve the rounded border side"
        );
        assert_eq!(
            frame_pixel_at(&frame, width, 4, 4),
            super::DEFAULT_RENDER_BACKGROUND_RGBA,
            "chrome must preserve the existing physical padding"
        );
        let tab_bar_separator_y = 10 + super::MODERN_CELL_HEIGHT - 1;
        assert_eq!(
            frame_pixel_at(&frame, width, 4, tab_bar_separator_y as usize),
            super::DEFAULT_TAB_BAR_SEPARATOR_RGBA,
            "modern chrome should separate the tab row from terminal content"
        );
        assert_eq!(
            app.runtime.terminal().grid().size(),
            rssh_core::TerminalSize::new(TERMINAL_COLUMNS, TERMINAL_ROWS),
            "chrome must not consume terminal rows or columns"
        );
    }

    #[test]
    fn modern_default_padding_mouse_mapping_excludes_margin() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.handle_window_resize(app.initial_frame_size())
            .expect("default frame should resize");

        let inside = PhysicalPosition::new(
            f64::from(14 + super::MODERN_CELL_WIDTH / 2),
            f64::from(
                10 + super::MODERN_CELL_HEIGHT + super::MODERN_CELL_HEIGHT / 2,
            ),
        );
        assert_eq!(app.window_mouse_cell(inside), Some((0, 0)));
        assert_eq!(
            app.window_mouse_cell(PhysicalPosition::new(4.0, inside.y)),
            None,
            "left outer padding is not a terminal cell"
        );
        assert_eq!(
            app.window_mouse_cell(PhysicalPosition::new(inside.x, 8.0)),
            None,
            "tab bar and top outer padding are not terminal cells"
        );
    }

    use super::{
        AppAction, AppShellError, CELL_HEIGHT, CELL_WIDTH,
        DEFAULT_ADJUST_WINDOW_SIZE_WHEN_CHANGING_FONT_SIZE, DEFAULT_ALLOW_DOWNLOAD_PROTOCOLS,
        DEFAULT_ALLOW_SQUARE_GLYPHS_TO_OVERFLOW_WIDTH, DEFAULT_ALLOW_WIN32_INPUT_MODE,
        DEFAULT_ALTERNATE_BUFFER_WHEEL_SCROLL_SPEED, DEFAULT_ANIMATION_FPS,
        DEFAULT_ANSI_PALETTE_COLORS, DEFAULT_ANTI_ALIAS_CUSTOM_BLOCK_GLYPHS,
        DEFAULT_AUTOMATICALLY_RELOAD_CONFIG, DEFAULT_BACKGROUND_COLOR, DEFAULT_BIDI_DIRECTION,
        DEFAULT_BIDI_ENABLED, DEFAULT_BOLD_BRIGHTENS_ANSI_COLORS,
        DEFAULT_CANONICALIZE_PASTED_NEWLINES, DEFAULT_CELL_WIDTH, DEFAULT_CHAR_SELECT_BG_COLOR,
        DEFAULT_CHAR_SELECT_FG_COLOR, DEFAULT_CHAR_SELECT_FONT_SIZE, DEFAULT_CHECK_FOR_UPDATES,
        DEFAULT_CHECK_FOR_UPDATES_INTERVAL_SECONDS, DEFAULT_COMMAND_PALETTE_BG_COLOR,
        DEFAULT_COMMAND_PALETTE_FG_COLOR, DEFAULT_COMMAND_PALETTE_FONT_SIZE,
        DEFAULT_UI_ACCENT_BACKGROUND, DEFAULT_UI_ACCENT_FOREGROUND, DEFAULT_UI_SURFACE_BACKGROUND,
        DEFAULT_UI_SURFACE_FOREGROUND,
        DEFAULT_CURSOR_BG_COLOR, DEFAULT_CUSTOM_BLOCK_GLYPHS, DEFAULT_DEBUG_KEY_EVENTS,
        DEFAULT_DETECT_PASSWORD_INPUT, DEFAULT_DISABLE_DEFAULT_KEY_BINDINGS,
        DEFAULT_DISABLE_DEFAULT_MOUSE_BINDINGS, DEFAULT_DISPLAY_PIXEL_GEOMETRY,
        DEFAULT_ENABLE_CHECKSUM_RECTANGULAR_AREA, DEFAULT_ENABLE_CSI_U_KEY_ENCODING,
        DEFAULT_ENABLE_KITTY_GRAPHICS, DEFAULT_ENABLE_KITTY_KEYBOARD,
        DEFAULT_ENABLE_TITLE_REPORTING, DEFAULT_ENABLE_WAYLAND, DEFAULT_ENABLE_ZWLR_OUTPUT_MANAGER,
        DEFAULT_ENQ_ANSWERBACK, DEFAULT_EXPERIMENTAL_PIXEL_POSITIONING, DEFAULT_FONT_ANTIALIAS,
        DEFAULT_FONT_COLR_RASTERIZER, DEFAULT_FONT_HINTING, DEFAULT_FONT_LOCATOR,
        DEFAULT_FONT_RASTERIZER, DEFAULT_FONT_SHAPER, DEFAULT_FONT_SIZE,
        DEFAULT_FORCE_REVERSE_VIDEO_CURSOR, DEFAULT_FOREGROUND_COLOR, DEFAULT_FOREGROUND_TEXT_HSB,
        DEFAULT_FREETYPE_LOAD_TARGET, DEFAULT_FREETYPE_PCF_LONG_FAMILY_NAMES,
        DEFAULT_GLYPH_CACHE_IMAGE_CACHE_SIZE, DEFAULT_HIDE_MOUSE_CURSOR_WHEN_TYPING,
        DEFAULT_IGNORE_SVG_FONTS, DEFAULT_IME_PREEDIT_RENDERING, DEFAULT_INACTIVE_PANE_HSB,
        DEFAULT_INTEGRATED_TITLE_BUTTON_ALIGNMENT, DEFAULT_INTEGRATED_TITLE_BUTTON_COLOR,
        DEFAULT_INTEGRATED_TITLE_BUTTON_STYLE, DEFAULT_LAUNCHER_ALPHABET, DEFAULT_LINE_HEIGHT,
        DEFAULT_LINE_QUAD_CACHE_SIZE, DEFAULT_LINE_STATE_CACHE_SIZE,
        DEFAULT_LINE_TO_ELE_SHAPE_CACHE_SIZE, DEFAULT_LOG_UNKNOWN_ESCAPE_SEQUENCES,
        DEFAULT_MACOS_FORWARD_TO_IME_MODIFIER_MASK, DEFAULT_MACOS_FULLSCREEN_EXTEND_BEHIND_NOTCH,
        DEFAULT_MACOS_WINDOW_BACKGROUND_BLUR, DEFAULT_MAX_FPS,
        DEFAULT_MUX_ENABLE_SSH_AGENT, DEFAULT_MUX_OUTPUT_PARSER_BUFFER_SIZE,
        DEFAULT_MUX_OUTPUT_PARSER_COALESCE_DELAY_MS, DEFAULT_NATIVE_MACOS_FULLSCREEN_MODE,
        DEFAULT_NOTIFICATION_HANDLING, DEFAULT_PALETTE_MAX_KEY_ASSIGMENTS_FOR_ACTION,
        DEFAULT_PANE_SELECT_BG_COLOR, DEFAULT_PANE_SELECT_FG_COLOR, DEFAULT_PANE_SELECT_FONT_SIZE,
        DEFAULT_PERIODIC_STAT_LOGGING, DEFAULT_PREFER_EGL, DEFAULT_QUICK_SELECT_ALPHABET,
        DEFAULT_QUOTE_DROPPED_FILES, DEFAULT_RATELIMIT_MUX_LINE_PREFETCHES_PER_SECOND,
        DEFAULT_RENDER_FRONT_END, DEFAULT_REVERSE_VIDEO_CURSOR_MIN_CONTRAST,
        DEFAULT_SCROLLBACK_LIMIT, DEFAULT_SEARCH_FONT_DIRS_FOR_FALLBACK,
        DEFAULT_SELECTION_WORD_BOUNDARY, DEFAULT_SEND_COMPOSED_KEY_WHEN_LEFT_ALT_IS_PRESSED,
        DEFAULT_SEND_COMPOSED_KEY_WHEN_RIGHT_ALT_IS_PRESSED, DEFAULT_SHAPE_CACHE_SIZE,
        DEFAULT_SHOW_UPDATE_WINDOW, DEFAULT_SORT_FALLBACK_FONTS_BY_COVERAGE,
        DEFAULT_STRIKETHROUGH_POSITION, DEFAULT_TEXT_BACKGROUND_OPACITY,
        DEFAULT_TREAT_EAST_ASIAN_AMBIGUOUS_WIDTH_AS_WIDE, DEFAULT_TREAT_LEFT_CTRLALT_AS_ALTGR,
        DEFAULT_ULIMIT_NOFILE, DEFAULT_ULIMIT_NPROC, DEFAULT_UNDERLINE_POSITION,
        DEFAULT_UNDERLINE_THICKNESS, DEFAULT_UNICODE_VERSION, DEFAULT_USE_BOX_MODEL_RENDER,
        DEFAULT_USE_CAP_HEIGHT_TO_SCALE_FALLBACK_FONTS, DEFAULT_USE_DEAD_KEYS, DEFAULT_USE_IME,
        DEFAULT_USE_RESIZE_INCREMENTS, DEFAULT_WARN_ABOUT_MISSING_GLYPHS,
        DEFAULT_WEBGPU_FORCE_FALLBACK_ADAPTER, DEFAULT_WEBGPU_POWER_PREFERENCE,
        DEFAULT_WIN32_SYSTEM_BACKDROP, DEFAULT_WINDOW_BACKGROUND_OPACITY,
        DEFAULT_WINDOW_CONTENT_ALIGNMENT, DEFAULT_WINDOW_DECORATIONS,
        DamageRegion, FRAME_HEIGHT, FRAME_WIDTH, FrameRenderMode, KittyKeyEventKind,
        LEGACY_COLOR_SCHEME_CURSOR_BG_COLOR, NativeAnsiColor, NativeAudibleBell,
        NativeBidiDirection, NativeBoldBrightensAnsiColors, NativeCanonicalizePastedNewlines,
        NativeCellWidth, NativeCellWidthOverride, NativeColorSpec, NativeCommandPaletteAugment,
        NativeCommandPaletteEntry, NativeConfigSnapshot, NativeConfirmation, NativeContrastRatio,
        NativeCubicBezier, NativeCursorStyle, NativeCursorThickness, NativeDaemonOptions,
        NativeDisplayPixelGeometry, NativeEasingFunction, NativeConfigView, NativeExecDomain,
        NativeExecDomainLabel, NativeExitBehavior, NativeExitBehaviorMessaging,
        NativeFontAntialias, NativeFontAttributes, NativeFontHinting, NativeFontLocator,
        NativeFontRasterizer, NativeFontRule, NativeFontRuleBlink, NativeFontShaper,
        NativeFontSize, NativeFormatAttribute, NativeFormatIntensity, NativeFormatItem,
        NativeFormatUnderline, NativeFreetypeLoadFlags, NativeFreetypeTarget,
        NativeHorizontalContentAlignment, NativeHsbMultiplier, NativeHyperlinkRule,
        NativeImePreeditRendering, NativeInactivePaneHsb, NativeInputSelector,
        NativeIntegratedTitleButton, NativeIntegratedTitleButtonAlignment,
        NativeIntegratedTitleButtonColor, NativeIntegratedTitleButtonStyle, NativeKeyMapPreference,
        NativeLaunchMenuCommand, NativeLaunchMenuItem, NativeLeaderKey, NativeLineHeight,
        NativeLuaNewTabButtonClick, NativeLuaNewTabButtonClickAllowDefault, NativeLuaOpenUri,
        NativeLuaPaneCursorPositionField, NativeLuaPaneDimensionsField, NativeLuaTabTitle,
        NativeLuaWindowStatusText, NativeLuaWindowStatusUpdate, NativeLuaWindowTitle,
        NativeMouseAssignmentAltScreen, NativeMouseAssignmentButton, NativeMouseAssignmentEvent,
        NativeMouseAssignmentEventKind, NativeNotificationHandling, NativePalette,
        NativePromptInputLine, NativeQuoteDroppedFiles, NativeRenderFrontEnd,
        NativeResolvedPalette, NativeScrollBarHeight, NativeSerialDomain, NativeShellAssumption,
        NativeSquareGlyphOverflow, NativeSshBackend, NativeSshDomain, NativeSshMultiplexing,
        NativeStrikethroughPosition, NativeTabBarItemColors, NativeTabBarStyle, NativeTabTitle,
        NativeTextBackgroundOpacity, NativeTextMinContrastRatio, NativeTlsClientDomain,
        NativeTlsServerDomain, NativeUiKeyCapRendering, NativeUnderlinePosition,
        NativeUnderlineThickness, NativeUnixDomain, NativeUserKeyAssignment,
        NativeUserMouseAssignment, NativeVerticalContentAlignment, NativeVisualBell,
        NativeVisualBellTarget, NativeWebGpuPowerPreference, NativeWebGpuPreferredAdapter,
        NativeWin32SystemBackdrop, NativeWindowApp, NativeWindowBackgroundGradient,
        NativeWindowBackgroundGradientBlend, NativeWindowBackgroundGradientInterpolation,
        NativeWindowBackgroundGradientOrientation, NativeWindowBackgroundGradientPreset,
        NativeWindowBackgroundGradientSegment, NativeWindowBell, NativeWindowCloseConfirmation,
        NativeWindowConfigReloaded, NativeWindowContentAlignment, NativeWindowDecorations,
        NativeWindowEmitEvent, NativeWindowFocusChange, NativeWindowFrameAppearance,
        NativeWindowLevel, NativeWindowManager, NativeWindowNewTabButtonClick, NativeWindowOpenUri,
        NativeWindowPadding, NativeWindowPaddingDimension, NativeWindowResize,
        NativeWindowStatusUpdate, NativeWindowStatusUpdateEvent, NativeWindowUserVarChange,
        NativeWslDomain, ProcessCwdCandidate, ResizeDirection, SearchDirection,
        SelectionCell, SelectionSourceCell, StableOrdinarySelection, TAB_BAR_ROWS,
        TERMINAL_COLUMNS, TERMINAL_ROWS, WINDOW_COMMANDS, WindowActivateKeyTable,
        WindowActivateWindowRequest, WindowCharSelectOptions, WindowClearScrollbackMode,
        WindowClick, WindowCloseTarget, WindowCommand, WindowCommandPaletteEntry,
        WindowConfirmationOptions, WindowCopyDestination, WindowDomainSelector, WindowEmitEvent,
        WindowFocusCoordinator, WindowFocusTransitions, WindowFontSizeAction,
        WindowInputSelectorAction, WindowInputSelectorChoice, WindowInputSelectorOptions,
        WindowMetrics, WindowMouseAssignmentClick, WindowMouseEvent, WindowMouseEventKind,
        WindowMouseSelectionMode, WindowPaneSelectMode, WindowPaneSelectOptions, WindowPasteSource,
        WindowPromptInputLineAction, WindowPromptInputLineOptions, WindowQuickSelect,
        WindowQuickSelectAction, WindowQuickSelectOptions, WindowScrollByPageAmount, WindowSearch,
        WindowSearchCommandQuery, WindowSearchMatch, WindowSearchMatchType, WindowSelection,
        WindowSendKey, WindowShowLauncherArgs, WindowShowLauncherFlags, WindowSourceSelection,
        WindowSpawnCommandQuery, WindowSpawnTabDomain, WindowSplitPaneOptions, WindowSplitPaneSize,
        WindowSwitchToWorkspaceOptions, WindowUserEvent, activate_window_absolute_index,
        activate_window_relative_index, command_palette_basic_structured_query_command,
        compact_terminal_tab_title,
        integrated_title_button_default_tab_bar_label,
        default_gui_startup_args, default_hyperlink_rules, default_integrated_title_buttons,
        default_mux_env_remove, default_native_unix_domains,
        default_skip_close_confirmation_for_processes_named, default_tiling_desktop_environments,
        demo_snapshot, dispatch_window_focus_changed, encode_window_focus_event, encode_window_key,
        encode_window_key_with_kitty, encode_window_key_with_kitty_event,
        encode_window_mouse_event, encode_window_mouse_event_with_pixels, encode_window_paste,
        finalize_native_gpu_frame, input_selector_options_from_query,
        native_window_key_assignment_entries, native_window_resize_increments_supported,
        nerd_font_icon_for_name, pane_select_activate_alphabet_from_query,
        pane_select_activate_show_pane_ids_alphabet_from_query, pane_select_alphabet_from_query,
        pane_select_mode_alphabet_from_query, pane_select_mode_show_pane_ids_from_query,
        pane_select_options_from_query, pane_select_show_pane_ids_alphabet_from_query,
        process_tree_current_working_dir, pty_command_from_pane_launch,
        pty_command_from_pane_launch_with_default_cwd,
        pty_command_from_pane_launch_with_environment, pty_command_from_pane_launch_with_term,
        pty_command_from_pane_launch_with_term_session_id, quick_select_options_from_query,
        quote_dropped_file_name, should_focus_materialized_window, show_launcher_args_from_query,
        split_pane_source_size_delta, tab_bar_new_tab_label, tab_bar_pixel_height,
        tab_bar_tab_label, terminal_size_from_window_pixels,
        terminal_size_from_window_pixels_with_padding, window_application_hide_shortcut,
        window_char_select_shortcut, window_clear_scrollback_shortcut,
        window_copy_destination_for_shortcut, window_copy_mode_shortcut, window_font_size_shortcut,
        window_hide_shortcut, window_paste_source_for_shortcut, window_quick_select_shortcut,
        window_reload_configuration_shortcut, window_search_shortcut,
        window_show_debug_overlay_shortcut, window_toggle_full_screen_shortcut,
        winit_window_level_for_native,
    };

    struct RefusingPaneLifecycle {
        reaper_started: Arc<AtomicUsize>,
        release_reaper: mpsc::Receiver<()>,
        dropped: Arc<AtomicUsize>,
    }

    struct ImmediatePaneMasterClose;

    impl super::PanePtyMasterCloseLifecycle for ImmediatePaneMasterClose {
        fn finish_before(&mut self, _deadline: Instant) -> super::PanePtyMasterCloseOutcome {
            super::PanePtyMasterCloseOutcome::Completed
        }
    }

    struct GatedPaneMasterClose {
        released: Arc<AtomicUsize>,
    }

    impl super::PanePtyMasterCloseLifecycle for GatedPaneMasterClose {
        fn finish_before(&mut self, _deadline: Instant) -> super::PanePtyMasterCloseOutcome {
            if self.released.load(Ordering::Acquire) == 0 {
                super::PanePtyMasterCloseOutcome::Deferred
            } else {
                super::PanePtyMasterCloseOutcome::Completed
            }
        }
    }

    struct GatedPaneLifecycle {
        close_released: Arc<AtomicUsize>,
        writer_dropped: Arc<AtomicUsize>,
    }

    impl super::PanePtySessionLifecycle for GatedPaneLifecycle {
        type MasterClose = GatedPaneMasterClose;

        fn stop_before(&mut self, _timeout: Duration) -> Result<PtyExitStatus, String> {
            Ok(PtyExitStatus::from_exit_code(0))
        }

        fn finish_before(&mut self, _timeout: Duration) -> Result<PtyExitStatus, String> {
            Ok(PtyExitStatus::from_exit_code(0))
        }

        fn begin_master_close(&mut self) -> Self::MasterClose {
            assert_eq!(
                self.writer_dropped.load(Ordering::Acquire),
                0,
                "master close must begin before the external writer is dropped"
            );
            GatedPaneMasterClose {
                released: Arc::clone(&self.close_released),
            }
        }

        fn reap_until_exit(&mut self) {}
    }

    struct ObservedPaneWriter(Arc<AtomicUsize>);

    impl Write for ObservedPaneWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Drop for ObservedPaneWriter {
        fn drop(&mut self) {
            self.0.store(1, Ordering::Release);
        }
    }

    #[derive(Clone, Copy)]
    enum FixedPaneMasterCloseMode {
        Failed,
        Panicked,
        Retained,
    }

    struct FixedPaneMasterClose(FixedPaneMasterCloseMode);

    impl super::PanePtyMasterCloseLifecycle for FixedPaneMasterClose {
        fn finish_before(&mut self, _deadline: Instant) -> super::PanePtyMasterCloseOutcome {
            match self.0 {
                FixedPaneMasterCloseMode::Failed => {
                    super::PanePtyMasterCloseOutcome::Failed("synthetic close failure".to_owned())
                }
                FixedPaneMasterCloseMode::Panicked => super::PanePtyMasterCloseOutcome::Panicked,
                FixedPaneMasterCloseMode::Retained => super::PanePtyMasterCloseOutcome::Retained,
            }
        }
    }

    struct FixedPaneLifecycle(FixedPaneMasterCloseMode);

    impl super::PanePtySessionLifecycle for FixedPaneLifecycle {
        type MasterClose = FixedPaneMasterClose;

        fn stop_before(&mut self, _timeout: Duration) -> Result<PtyExitStatus, String> {
            Ok(PtyExitStatus::from_exit_code(0))
        }

        fn finish_before(&mut self, _timeout: Duration) -> Result<PtyExitStatus, String> {
            Ok(PtyExitStatus::from_exit_code(0))
        }

        fn begin_master_close(&mut self) -> Self::MasterClose {
            FixedPaneMasterClose(self.0)
        }

        fn reap_until_exit(&mut self) {}
    }

    struct RefusingFixedPaneLifecycle;

    impl super::PanePtySessionLifecycle for RefusingFixedPaneLifecycle {
        type MasterClose = FixedPaneMasterClose;

        fn stop_before(&mut self, _timeout: Duration) -> Result<PtyExitStatus, String> {
            Err("synthetic stop refusal".to_owned())
        }

        fn finish_before(&mut self, _timeout: Duration) -> Result<PtyExitStatus, String> {
            Err("synthetic finish refusal".to_owned())
        }

        fn begin_master_close(&mut self) -> Self::MasterClose {
            FixedPaneMasterClose(FixedPaneMasterCloseMode::Failed)
        }

        fn reap_until_exit(&mut self) {}
    }

    impl super::PanePtySessionLifecycle for RefusingPaneLifecycle {
        type MasterClose = ImmediatePaneMasterClose;

        fn stop_before(&mut self, _timeout: Duration) -> Result<PtyExitStatus, String> {
            Err("synthetic terminate refusal".to_owned())
        }

        fn finish_before(&mut self, _timeout: Duration) -> Result<PtyExitStatus, String> {
            Err("synthetic wait refusal".to_owned())
        }

        fn begin_master_close(&mut self) -> Self::MasterClose {
            ImmediatePaneMasterClose
        }

        fn reap_until_exit(&mut self) {
            self.reaper_started.store(1, Ordering::Release);
            self.release_reaper.recv().unwrap();
        }
    }

    impl Drop for RefusingPaneLifecycle {
        fn drop(&mut self) {
            self.dropped.store(1, Ordering::Release);
        }
    }

    static PANE_PTY_TEST_LOCK: Mutex<()> = Mutex::new(());
    const TEST_ASYNC_FINALITY_BUDGET: Duration = Duration::from_secs(5);

    fn wait_for_test_condition(description: &str, mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + TEST_ASYNC_FINALITY_BUDGET;
        while !condition() {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {description}"
            );
            thread::yield_now();
        }
    }

    fn wait_for_thread_finished(worker: &thread::JoinHandle<()>, description: &str) {
        wait_for_test_condition(description, || worker.is_finished());
    }

    #[test]
    fn pane_pty_timeout_transfers_close_and_reader_as_one_reaper_job() {
        let _test_lock = PANE_PTY_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let baseline = super::pane_pty_reaper_pending();
        let close_released = Arc::new(AtomicUsize::new(0));
        let writer_dropped = Arc::new(AtomicUsize::new(0));
        let reader_finished = Arc::new(AtomicUsize::new(0));
        let (reader_release_sender, reader_release_receiver) = mpsc::channel();
        let reader_finished_for_worker = Arc::clone(&reader_finished);
        let reader_thread = thread::spawn(move || {
            reader_release_receiver.recv().unwrap();
            reader_finished_for_worker.store(1, Ordering::Release);
        });
        let ownership = super::PanePtyOwnership {
            session: Some(GatedPaneLifecycle {
                close_released: Arc::clone(&close_released),
                writer_dropped: Arc::clone(&writer_dropped),
            }),
            writer: Some(Box::new(ObservedPaneWriter(Arc::clone(&writer_dropped)))),
            master_close: None,
            reader_thread: Some(reader_thread),
            writer_thread: None,
        };

        let outcome = super::cleanup_pane_pty_ownership(
            ownership,
            super::PanePtyCleanupOperation::Stop,
            Instant::now(),
        );

        assert!(outcome.transferred_to_reaper);
        assert_eq!(writer_dropped.load(Ordering::Acquire), 1);
        assert_eq!(super::pane_pty_reaper_pending(), baseline + 1);
        close_released.store(1, Ordering::Release);
        let close_only_deadline = Instant::now() + Duration::from_millis(50);
        while Instant::now() < close_only_deadline {
            std::thread::yield_now();
        }
        assert_eq!(super::pane_pty_reaper_pending(), baseline + 1);
        assert_eq!(reader_finished.load(Ordering::Acquire), 0);

        reader_release_sender.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while super::pane_pty_reaper_pending() != baseline && Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert_eq!(super::pane_pty_reaper_pending(), baseline);
        assert_eq!(reader_finished.load(Ordering::Acquire), 1);
    }

    #[test]
    fn pane_pty_close_failure_and_panic_are_observable() {
        let _test_lock = PANE_PTY_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for (mode, expected) in [
            (FixedPaneMasterCloseMode::Failed, "synthetic close failure"),
            (FixedPaneMasterCloseMode::Panicked, "close worker panicked"),
        ] {
            let outcome = super::cleanup_pane_pty_ownership(
                super::PanePtyOwnership {
                    session: Some(FixedPaneLifecycle(mode)),
                    writer: None,
                    master_close: None,
                    reader_thread: None,
                    writer_thread: None,
                },
                super::PanePtyCleanupOperation::Stop,
                Instant::now() + Duration::from_secs(1),
            );

            assert!(!outcome.transferred_to_reaper);
            assert!(outcome.issue.as_deref().unwrap().contains(expected));
        }
    }

    #[test]
    fn pane_pty_timeout_preserves_terminal_close_issue_when_reader_is_gated() {
        let _test_lock = PANE_PTY_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for (mode, expected) in [
            (FixedPaneMasterCloseMode::Failed, "synthetic close failure"),
            (FixedPaneMasterCloseMode::Retained, "ownership was retained"),
        ] {
            let baseline = super::pane_pty_reaper_pending();
            let (reader_release_sender, reader_release_receiver) = mpsc::channel();
            let reader_thread = thread::spawn(move || reader_release_receiver.recv().unwrap());

            let outcome = super::cleanup_pane_pty_ownership(
                super::PanePtyOwnership {
                    session: Some(FixedPaneLifecycle(mode)),
                    writer: None,
                    master_close: None,
                    reader_thread: Some(reader_thread),
                    writer_thread: None,
                },
                super::PanePtyCleanupOperation::Stop,
                Instant::now(),
            );

            let issue = outcome.issue.as_deref().unwrap();
            assert!(outcome.transferred_to_reaper);
            assert!(issue.contains(expected), "missing terminal issue: {issue}");
            assert!(
                issue.contains("cleanup deadline"),
                "missing timeout: {issue}"
            );
            assert!(
                issue.contains("transferred to reaper"),
                "missing transfer: {issue}"
            );

            reader_release_sender.send(()).unwrap();
            let deadline = Instant::now() + Duration::from_secs(1);
            while super::pane_pty_reaper_pending() != baseline && Instant::now() < deadline {
                std::thread::yield_now();
            }
            assert_eq!(super::pane_pty_reaper_pending(), baseline);
        }
    }

    #[test]
    fn pane_pty_reaper_reports_close_failure_before_gated_reader_finishes() {
        let _test_lock = PANE_PTY_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let baseline = super::pane_pty_reaper_pending();
        drop(super::take_pane_pty_reaper_reported_issues());
        let (reader_release_sender, reader_release_receiver) = mpsc::channel();
        let reader_thread = thread::spawn(move || reader_release_receiver.recv().unwrap());

        let outcome = super::cleanup_pane_pty_ownership(
            super::PanePtyOwnership {
                session: Some(RefusingFixedPaneLifecycle),
                writer: None,
                master_close: None,
                reader_thread: Some(reader_thread),
                writer_thread: None,
            },
            super::PanePtyCleanupOperation::Stop,
            Instant::now(),
        );

        assert!(outcome.transferred_to_reaper);
        let report_deadline = Instant::now() + Duration::from_secs(1);
        let mut reported = Vec::new();
        while Instant::now() < report_deadline {
            reported.extend(super::take_pane_pty_reaper_reported_issues());
            if reported
                .iter()
                .any(|issue| issue.contains("synthetic close failure"))
            {
                break;
            }
            std::thread::yield_now();
        }
        assert!(
            reported
                .iter()
                .any(|issue| issue.contains("synthetic close failure")),
            "close failure was not reported while the reader remained gated: {reported:?}"
        );
        assert_eq!(super::pane_pty_reaper_pending(), baseline + 1);

        reader_release_sender.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while super::pane_pty_reaper_pending() != baseline && Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert_eq!(super::pane_pty_reaper_pending(), baseline);
    }

    fn process_exists_for_pane_pty_test(process_id: u32) -> bool {
        let process_id = sysinfo::Pid::from_u32(process_id);
        let mut system = sysinfo::System::new();
        let _ = system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[process_id]), true);
        system.process(process_id).is_some()
    }

    #[cfg(target_os = "windows")]
    fn sleeping_pane_pty_command() -> PtyCommand {
        // A built-in input wait needs no managed runtime or descendant process.
        // /D also prevents user AutoRun commands from changing this fixture.
        PtyCommand::new("cmd.exe").with_args([
            "/D",
            "/Q",
            "/C",
            "echo RSSH-PTY-READY & set /p RSSH_TEST_WAIT=",
        ])
    }

    #[cfg(not(target_os = "windows"))]
    fn sleeping_pane_pty_command() -> PtyCommand {
        PtyCommand::new("/bin/sh").with_args(["-c", "printf 'RSSH-PTY-READY\\n'; exec sleep 30"])
    }

    #[test]
    fn pane_pty_stop_reaps_real_child_and_joins_real_reader() {
        let _test_lock = PANE_PTY_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut session = PtySession::spawn(
            &sleeping_pane_pty_command(),
            PtySize::try_new(80, 24).unwrap(),
        )
        .unwrap();
        let process_id = session
            .process_id()
            .expect("real PTY must expose its child PID");
        let mut reader = session.take_reader().unwrap();
        let mut writer = session.take_writer().unwrap();
        let (reader_ready_tx, reader_ready_rx) = mpsc::channel();
        let (reader_finished_tx, reader_finished_rx) = mpsc::channel();
        let reader_thread = thread::spawn(move || {
            use std::io::Read;
            let marker = b"RSSH-PTY-READY";
            let mut startup = Vec::new();
            let mut byte = [0_u8];
            while !startup.ends_with(marker) {
                reader.read_exact(&mut byte).expect("PTY readiness marker");
                startup.push(byte[0]);
                if startup.ends_with(b"\x1b[6n") {
                    // Act as the terminal during startup. The PTY close helper
                    // only answers outstanding cursor queries once closing.
                    writer.write_all(b"\x1b[1;1R").unwrap();
                    writer.flush().unwrap();
                }
            }
            reader_ready_tx
                .send(writer)
                .unwrap_or_else(|_| panic!("PTY readiness receiver closed"));
            let result = io::copy(&mut reader, &mut io::sink()).map(|_| ());
            reader_finished_tx.send(result).unwrap();
        });
        // Measure cleanup of a running shell, not ConPTY/shell startup
        // racing termination while the parallel suite is creating processes.
        let writer = reader_ready_rx
            .recv_timeout(Duration::from_secs(15))
            .expect("real PTY child and reader must be ready before cleanup");
        assert!(
            process_exists_for_pane_pty_test(process_id),
            "real PTY child {process_id} must be running before cleanup"
        );

        let outcome = super::cleanup_pane_pty_ownership(
            super::PanePtyOwnership {
                session: Some(session),
                writer: Some(writer),
                master_close: None,
                reader_thread: Some(reader_thread),
                writer_thread: None,
            },
            super::PanePtyCleanupOperation::Stop,
            Instant::now() + Duration::from_secs(2),
        );

        assert!(outcome.status.is_some());
        assert_eq!(outcome.issue, None);
        assert!(!outcome.transferred_to_reaper);
        reader_finished_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("real PTY reader worker must finish before cleanup returns")
            .expect("real PTY reader must finish without an I/O error");
        let reaped_deadline = Instant::now() + Duration::from_secs(1);
        while process_exists_for_pane_pty_test(process_id) && Instant::now() < reaped_deadline {
            std::thread::yield_now();
        }
        assert!(
            !process_exists_for_pane_pty_test(process_id),
            "real PTY child {process_id} must be reaped after cleanup returns"
        );
    }

    #[test]
    fn pane_pty_stop_refusal_transfers_session_and_reader_to_observable_reaper() {
        let _test_lock = PANE_PTY_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let baseline = super::pane_pty_reaper_pending();
        let reaper_started = Arc::new(AtomicUsize::new(0));
        let dropped = Arc::new(AtomicUsize::new(0));
        let reader_finished = Arc::new(AtomicUsize::new(0));
        let (release_reaper_tx, release_reaper_rx) = mpsc::channel();
        let (release_reader_tx, release_reader_rx) = mpsc::channel();
        let reader_finished_in_thread = Arc::clone(&reader_finished);
        let reader = thread::spawn(move || {
            release_reader_rx.recv().unwrap();
            reader_finished_in_thread.store(1, Ordering::Release);
        });
        let ownership = super::PanePtyOwnership {
            session: Some(RefusingPaneLifecycle {
                reaper_started: Arc::clone(&reaper_started),
                release_reaper: release_reaper_rx,
                dropped: Arc::clone(&dropped),
            }),
            writer: None,
            master_close: None,
            reader_thread: Some(reader),
            writer_thread: None,
        };

        let outcome = super::cleanup_pane_pty_ownership(
            ownership,
            super::PanePtyCleanupOperation::Stop,
            Instant::now() + Duration::from_millis(20),
        );

        assert!(outcome.transferred_to_reaper);
        assert!(
            outcome
                .issue
                .as_deref()
                .unwrap()
                .contains("terminate refusal")
        );
        assert!(super::pane_pty_reaper_pending() > baseline);
        assert_eq!(dropped.load(Ordering::Acquire), 0);
        assert_eq!(reader_finished.load(Ordering::Acquire), 0);
        let start_deadline = Instant::now() + Duration::from_secs(1);
        while reaper_started.load(Ordering::Acquire) == 0 && Instant::now() < start_deadline {
            std::thread::yield_now();
        }
        assert_eq!(reaper_started.load(Ordering::Acquire), 1);
        release_reaper_tx.send(()).unwrap();
        assert_eq!(dropped.load(Ordering::Acquire), 0);
        assert!(super::pane_pty_reaper_pending() > baseline);
        assert_eq!(reader_finished.load(Ordering::Acquire), 0);
        release_reader_tx.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while super::pane_pty_reaper_pending() > baseline && Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert_eq!(super::pane_pty_reaper_pending(), baseline);
        assert_eq!(dropped.load(Ordering::Acquire), 1);
        assert_eq!(reader_finished.load(Ordering::Acquire), 1);
    }

    #[test]
    fn pane_pty_reader_panic_is_observable_after_session_cleanup() {
        let _test_lock = PANE_PTY_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let reader_thread = thread::spawn(|| panic!("synthetic reader panic"));
        wait_for_thread_finished(&reader_thread, "pane PTY panic reader");
        let ownership = super::PanePtyOwnership {
            session: None::<RefusingPaneLifecycle>,
            writer: None,
            master_close: None,
            reader_thread: Some(reader_thread),
            writer_thread: None,
        };

        let outcome = super::cleanup_pane_pty_ownership(
            ownership,
            super::PanePtyCleanupOperation::Finish,
            Instant::now() + Duration::from_secs(1),
        );

        assert!(!outcome.transferred_to_reaper);
        assert!(
            outcome
                .issue
                .as_deref()
                .unwrap()
                .contains("reader thread panicked")
        );
    }

    #[test]
    fn pane_pty_cleanup_joins_input_writer_worker() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut writer: Option<Box<dyn Write + Send>> =
            Some(Box::new(SharedWriter(Arc::clone(&written))));
        let mut writer_thread = None;
        super::start_pane_input_queue(&mut writer, &mut writer_thread, None).unwrap();
        writer
            .as_mut()
            .unwrap()
            .write_all(b"queued-before-close")
            .unwrap();

        let outcome = super::cleanup_pane_pty_ownership(
            super::PanePtyOwnership {
                session: None::<RefusingPaneLifecycle>,
                writer,
                master_close: None,
                reader_thread: None,
                writer_thread,
            },
            super::PanePtyCleanupOperation::Finish,
            Instant::now() + Duration::from_secs(1),
        );

        assert_eq!(outcome.issue, None);
        assert!(!outcome.transferred_to_reaper);
        assert_eq!(written.lock().unwrap().as_slice(), b"queued-before-close");
    }

    #[test]
    fn pane_pty_finish_refusal_is_observable_and_never_restores_runtime_ownership() {
        let _test_lock = PANE_PTY_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let baseline = super::pane_pty_reaper_pending();
        let reaper_started = Arc::new(AtomicUsize::new(0));
        let dropped = Arc::new(AtomicUsize::new(0));
        let (release_tx, release_rx) = mpsc::channel();
        let ownership = super::PanePtyOwnership {
            session: Some(RefusingPaneLifecycle {
                reaper_started: Arc::clone(&reaper_started),
                release_reaper: release_rx,
                dropped: Arc::clone(&dropped),
            }),
            writer: None,
            master_close: None,
            reader_thread: None,
            writer_thread: None,
        };

        let outcome = super::cleanup_pane_pty_ownership(
            ownership,
            super::PanePtyCleanupOperation::Finish,
            Instant::now() + Duration::from_millis(20),
        );

        assert!(outcome.status.is_none());
        assert!(outcome.transferred_to_reaper);
        assert!(outcome.issue.as_deref().unwrap().contains("wait refusal"));
        assert_eq!(dropped.load(Ordering::Acquire), 0);
        let start_deadline = Instant::now() + Duration::from_secs(1);
        while reaper_started.load(Ordering::Acquire) == 0 && Instant::now() < start_deadline {
            std::thread::yield_now();
        }
        assert_eq!(reaper_started.load(Ordering::Acquire), 1);
        release_tx.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while super::pane_pty_reaper_pending() > baseline && Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert_eq!(super::pane_pty_reaper_pending(), baseline);
        assert_eq!(dropped.load(Ordering::Acquire), 1);
    }

    struct StartupTestDir(PathBuf);

    impl Drop for StartupTestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn startup_test_dir(label: &str) -> StartupTestDir {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "rssh-window-startup-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        StartupTestDir(path)
    }

    fn startup_test_options(config: WindowConfigOptions) -> WindowOptions {
        WindowOptions {
            config,
            frame_limit: None,
            workspace: None,
            window_class: None,
            position: None,
            osc52_policy: Osc52Policy::default(),
            metrics: false,
            metrics_json: false,
            state: false,
            state_json: false,
            command: rssh_pty::PtyCommand::default_shell(),
            log: None,
        }
    }

    fn startup_test_discovery() -> ConfigDiscoveryInputs {
        ConfigDiscoveryInputs {
            is_windows: false,
            is_unix: false,
            current_exe: None,
            home_dir: None,
            xdg_config_home: None,
            xdg_config_dirs: Vec::new(),
            environment_config_file: None,
        }
    }

    fn install_reload_transaction_observers(
        manager: &mut NativeWindowManager,
    ) -> (Arc<Mutex<Vec<bool>>>, Vec<Arc<AtomicUsize>>) {
        let app_count = manager.all_apps_for_test().len();
        let applied = Arc::new(Mutex::new(vec![false; app_count]));
        let callbacks = (0..app_count)
            .map(|_| Arc::new(AtomicUsize::new(0)))
            .collect::<Vec<_>>();
        for (index, app) in manager.all_apps_mut_for_test().into_iter().enumerate() {
            let applied_by_app = Arc::clone(&applied);
            app.base_config_apply_observer = Some(Box::new(move |_| {
                applied_by_app.lock().unwrap()[index] = true;
            }));
            let all_applied = Arc::clone(&applied);
            let callback = Arc::clone(&callbacks[index]);
            app.config_reloaded_handler = Box::new(move |_| {
                assert!(
                    all_applied.lock().unwrap().iter().all(|applied| *applied),
                    "no reload callback may run until every managed app finished applying"
                );
                callback.fetch_add(1, Ordering::Relaxed);
                true
            });
        }
        (applied, callbacks)
    }

    #[test]
    fn deferred_startup_config_does_not_parse_on_the_event_loop_turn() {
        let root = startup_test_dir("deferred-config-worker");
        let path = root.0.join("wezterm.lua");
        std::fs::write(&path, "return { term = 'loaded-off-thread' }").unwrap();
        let lifecycle = Box::new(NativeConfigLifecycle::new(
            startup_test_discovery(),
            false,
            Some(path),
            validate_cli_config_overrides(&[]).unwrap(),
        ));
        let mut visible = NativeWindowApp::new(None);
        visible.rendered_frames = 1;
        visible.transport_start_requested = true;
        let mut manager = NativeWindowManager::new_for_test(NativeWindowApp::new(None))
            .with_config_lifecycle(lifecycle)
            .with_deferred_config();
        manager
            .windows
            .insert(winit::window::WindowId::dummy(), Box::new(visible));

        assert_eq!(manager.config_generation_for_test(), Some(0));
        manager.start_deferred_config_if_ready();

        assert_eq!(
            manager.config_generation_for_test(),
            Some(0),
            "the first event-loop turn must only schedule configuration loading"
        );
    }

    #[test]
    fn deferred_startup_config_applies_after_the_worker_finishes() {
        let root = startup_test_dir("deferred-config-completion");
        let path = root.0.join("wezterm.lua");
        std::fs::write(&path, "return { term = 'loaded-off-thread' }").unwrap();
        let lifecycle = Box::new(NativeConfigLifecycle::new(
            startup_test_discovery(),
            false,
            Some(path),
            validate_cli_config_overrides(&[]).unwrap(),
        ));
        let mut visible = NativeWindowApp::new(None);
        visible.rendered_frames = 1;
        visible.transport_start_requested = true;
        let window_id = winit::window::WindowId::dummy();
        let mut manager = NativeWindowManager::new_for_test(NativeWindowApp::new(None))
            .with_config_lifecycle(lifecycle)
            .with_deferred_config();
        manager.windows.insert(window_id, Box::new(visible));

        manager.start_deferred_config_if_ready();
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline && manager.config_generation_for_test() == Some(0) {
            manager.finish_deferred_config_if_ready();
            thread::yield_now();
        }

        assert_eq!(manager.config_generation_for_test(), Some(1));
        assert_eq!(
            manager.windows.get(&window_id).unwrap().term,
            "loaded-off-thread"
        );
    }

    fn arm_reload_event_transaction_sentinels(
        manager: &mut NativeWindowManager,
    ) -> Vec<Arc<AtomicUsize>> {
        let apps = manager.all_apps_mut_for_test();
        assert_eq!(
            apps.len(),
            2,
            "the event fixture must cover primary and detached owners"
        );
        apps.into_iter()
            .enumerate()
            .map(|(index, app)| {
                let table = format!("reload-event-sentinel-{index}");
                assert!(app.command_palette_execute(WindowCommand::ActivateKeyTable(
                    WindowActivateKeyTable {
                        name: table,
                        timeout_milliseconds: None,
                        one_shot: false,
                        replace_current: false,
                        until_unknown: false,
                        prevent_fallback: false,
                    },
                )));
                app.leader_active_since = Some(Instant::now());
                let callbacks = Arc::new(AtomicUsize::new(0));
                let callback = Arc::clone(&callbacks);
                app.config_reloaded_handler = Box::new(move |_| {
                    callback.fetch_add(1, Ordering::Relaxed);
                    true
                });
                callbacks
            })
            .collect()
    }

    fn take_reload_event_before_global_dispatch(
        manager: &NativeWindowManager,
        queued: &Arc<Mutex<Vec<WindowUserEvent>>>,
        callbacks: &[Arc<AtomicUsize>],
    ) -> WindowUserEvent {
        assert_eq!(
            manager.config_generation_for_test(),
            Some(1),
            "enqueueing must not install the next global generation inline"
        );
        assert!(
            callbacks
                .iter()
                .all(|callback| callback.load(Ordering::Relaxed) == 0),
            "enqueueing must not invoke any app callback inline"
        );
        for (index, app) in manager.all_apps_for_test().into_iter().enumerate() {
            assert_eq!(
                app.active_key_table_for_test(),
                Some(format!("reload-event-sentinel-{index}").as_str()),
                "enqueueing must not clear any app key table inline"
            );
            assert!(
                app.leader_active_since.is_some(),
                "enqueueing must not clear any app leader inline"
            );
        }

        let mut queued = queued.lock().unwrap();
        assert_eq!(queued.len(), 1, "the entry point must enqueue exactly once");
        let event = queued.remove(0);
        assert!(matches!(
            event,
            WindowUserEvent::ReloadConfigurationRequested
        ));
        event
    }

    fn assert_global_reload_event_completed(
        manager: &NativeWindowManager,
        callbacks: &[Arc<AtomicUsize>],
        expected_term: &str,
    ) {
        assert_eq!(manager.config_generation_for_test(), Some(2));
        assert!(
            callbacks
                .iter()
                .all(|callback| callback.load(Ordering::Relaxed) == 1),
            "the manager transaction must notify each app exactly once"
        );
        for app in manager.all_apps_for_test() {
            assert_eq!(app.term, expected_term);
            assert_eq!(app.active_key_table_for_test(), None);
            assert!(app.leader_active_since.is_none());
        }
    }

    struct ReloadRuntimeSentinels {
        active_pane: rssh_core::PaneId,
        inactive_pane: rssh_core::PaneId,
        active_writer: Arc<Mutex<Vec<u8>>>,
        inactive_writer: Arc<Mutex<Vec<u8>>>,
        active_process_id: u32,
        inactive_process_id: u32,
        active_reader_id: std::thread::ThreadId,
        inactive_reader_id: std::thread::ThreadId,
        spawn_attempts: Arc<AtomicUsize>,
    }

    fn install_reload_runtime_sentinels(app: &mut NativeWindowApp) -> ReloadRuntimeSentinels {
        app.handle_pty_output(b"\x1b[38;2;100;150;200mI\x1b[0mD")
            .unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"\x1b[38;2;100;150;200mA\x1b[0mD")
            .unwrap();

        let active_pane = app.app_shell.active_pane_id();
        let inactive_pane = app
            .pane_runtimes
            .keys()
            .copied()
            .find(|pane| *pane != active_pane)
            .expect("split fixture should own one inactive pane runtime");
        let active_writer = Arc::new(Mutex::new(Vec::new()));
        let inactive_writer = Arc::new(Mutex::new(Vec::new()));
        let active_process_id = 41_001;
        let inactive_process_id = 41_002;
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&active_writer))));
        app.session_process_id = Some(active_process_id);
        app.reader_thread = Some(std::thread::spawn(|| {}));
        let active_reader_id = app.reader_thread.as_ref().unwrap().thread().id();
        let inactive = app.pane_runtimes.get_mut(&inactive_pane).unwrap();
        inactive.writer = Some(Box::new(SharedWriter(Arc::clone(&inactive_writer))));
        inactive.session_process_id = Some(inactive_process_id);
        inactive.reader_thread = Some(std::thread::spawn(|| {}));
        let inactive_reader_id = inactive.reader_thread.as_ref().unwrap().thread().id();
        let spawn_attempts = Arc::new(AtomicUsize::new(0));
        app.pty_spawn_observer = Some(Arc::clone(&spawn_attempts));

        ReloadRuntimeSentinels {
            active_pane,
            inactive_pane,
            active_writer,
            inactive_writer,
            active_process_id,
            inactive_process_id,
            active_reader_id,
            inactive_reader_id,
            spawn_attempts,
        }
    }

    fn assert_reload_runtime_sentinels_preserved(
        app: &mut NativeWindowApp,
        sentinels: &ReloadRuntimeSentinels,
    ) {
        assert_eq!(sentinels.spawn_attempts.load(Ordering::Relaxed), 0);
        assert_eq!(app.active_pane_id(), sentinels.active_pane);
        assert_eq!(app.session_process_id, Some(sentinels.active_process_id));
        assert_eq!(
            app.reader_thread.as_ref().unwrap().thread().id(),
            sentinels.active_reader_id
        );
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('A'));
        app.writer
            .as_mut()
            .unwrap()
            .write_all(b"active-sentinel")
            .unwrap();
        assert_eq!(
            sentinels.active_writer.lock().unwrap().as_slice(),
            b"active-sentinel"
        );

        let inactive = app.pane_runtimes.get_mut(&sentinels.inactive_pane).unwrap();
        assert_eq!(
            inactive.session_process_id,
            Some(sentinels.inactive_process_id)
        );
        assert_eq!(
            inactive.reader_thread.as_ref().unwrap().thread().id(),
            sentinels.inactive_reader_id
        );
        assert_eq!(snapshot_char(&inactive.snapshot, 0, 0), Some('I'));
        inactive
            .writer
            .as_mut()
            .unwrap()
            .write_all(b"inactive-sentinel")
            .unwrap();
        assert_eq!(
            sentinels.inactive_writer.lock().unwrap().as_slice(),
            b"inactive-sentinel"
        );
    }

    #[test]
    fn window_run_configures_app_before_first_spawn() {
        let root = startup_test_dir("before-spawn");
        let path = root.0.join("wezterm.lua");
        std::fs::write(
            &path,
            "return {
                term = 'startup-term',
                default_prog = { 'configured-shell', '--login' },
                default_cwd = 'file-cwd',
            }",
        )
        .unwrap();
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path),
            config_overrides: Vec::new(),
        });

        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();

        assert_eq!(configured.lifecycle.effective().generation, 1);
        assert_eq!(configured.app.term, "startup-term");
        assert_eq!(configured.app.startup_command.program(), "configured-shell");
        assert_eq!(configured.app.startup_command.args(), ["--login"]);
        let command = pty_command_from_pane_launch_with_environment(
            configured.app.app_shell.active_pane().launch(),
            &configured.app.term,
            &configured.app.pane_environment_variables(),
            configured.app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("file-cwd")));
        assert!(configured.app.window.is_none());
        assert!(configured.app.session.is_none());
    }

    #[test]
    fn configured_startup_app_stays_within_stack_budget() {
        let actual = std::mem::size_of::<super::ConfiguredStartupApp>();
        assert!(
            actual <= 16 * 1024,
            "ConfiguredStartupApp is {actual} bytes; startup ownership must stay within the 16 KiB stack budget"
        );
    }

    #[test]
    fn native_config_overrides_stays_within_stack_budget() {
        let actual = std::mem::size_of::<NativeConfigSnapshot>();
        assert!(
            actual <= 16 * 1024,
            "NativeConfigSnapshot is {actual} bytes; config parsing and reload values must stay within the 16 KiB stack budget"
        );
    }

    #[test]
    fn window_manager_stays_within_stack_budget() {
        let actual = std::mem::size_of::<NativeWindowManager>();
        assert!(
            actual <= 16 * 1024,
            "NativeWindowManager is {actual} bytes; event-loop ownership must stay within the 16 KiB stack budget"
        );
    }

    #[test]
    fn native_gpu_ownership_stays_pointer_indirect() {
        fn gpu_field(app: &NativeWindowApp) -> Option<&crate::window_gpu::WindowGpu> {
            app.gpu_owners.active.as_deref()
        }

        std::hint::black_box(gpu_field);
        assert_eq!(
            std::mem::size_of::<Option<Box<crate::window_gpu::WindowGpu>>>(),
            std::mem::size_of::<usize>(),
            "native GPU state must stay heap-indirect on the 1 MiB Windows main stack"
        );
    }

    #[test]
    fn initial_invalid_source_uses_generation_zero_defaults_and_diagnostic() {
        let root = startup_test_dir("invalid-source");
        let path = root.0.join("wezterm.lua");
        std::fs::write(&path, "return { term = dynamic_term() }").unwrap();
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path.clone()),
            config_overrides: Vec::new(),
        });

        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();

        assert_eq!(configured.lifecycle.effective().generation, 0);
        assert!(configured.lifecycle.effective().source.is_none());
        assert!(
            configured
                .lifecycle
                .effective()
                .publication
                .variables()
                .is_empty()
        );
        assert!(configured.app.derived_config_environment.is_empty());
        assert_eq!(configured.app.term, super::DEFAULT_TERM);
        let diagnostic = configured.lifecycle.latest_diagnostic().unwrap();
        assert_eq!(diagnostic.path, path);
        assert!(diagnostic.to_string().contains("unsupported dynamic Lua"));
        assert!(configured.app.window.is_none());
        assert!(configured.app.session.is_none());
    }

    #[test]
    fn invalid_cli_override_fails_before_app_construction() {
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: None,
            config_overrides: vec![("not_a_config_field".to_owned(), "true".to_owned())],
        });

        let error = super::configured_startup_app_with_constructor(
            &options,
            startup_test_discovery(),
            |_| panic!("app constructor must not run for invalid CLI overrides"),
        )
        .err()
        .expect("invalid CLI override should be fatal");

        assert!(matches!(
            error,
            crate::config_lifecycle::NativeConfigLoadError::UnknownField { .. }
        ));
    }

    #[test]
    fn explicit_cli_program_and_cwd_beat_file_defaults() {
        let root = startup_test_dir("cli-precedence");
        let path = root.0.join("wezterm.lua");
        std::fs::write(
            &path,
            "return { default_prog = { 'file-shell', '--login' }, default_cwd = 'file-cwd' }",
        )
        .unwrap();
        let cli_cwd = root.0.join("cli-cwd");
        let mut options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path),
            config_overrides: Vec::new(),
        });
        options.command = rssh_pty::PtyCommand::new("cli-program").with_cwd(cli_cwd.clone());

        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();

        assert_eq!(configured.app.startup_command.program(), "cli-program");
        assert_eq!(
            configured.app.startup_command.cwd(),
            Some(cli_cwd.as_path())
        );
        assert_eq!(configured.app.default_cwd.as_deref(), Some("file-cwd"));
        assert!(configured.app.session.is_none());
    }

    #[test]
    fn successful_source_publishes_wezterm_config_environment() {
        let root = startup_test_dir("publication");
        let path = root.0.join("wezterm.lua");
        std::fs::write(&path, "return {}").unwrap();
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path.clone()),
            config_overrides: Vec::new(),
        });

        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();
        let environment = configured.app.pane_environment_variables();

        assert_eq!(
            configured
                .app
                .derived_config_environment
                .get("WEZTERM_CONFIG_FILE"),
            Some(&path.to_string_lossy().into_owned())
        );
        assert_eq!(
            environment.get("WEZTERM_CONFIG_FILE"),
            Some(&path.to_string_lossy().into_owned())
        );
        assert_eq!(
            environment.get("WEZTERM_CONFIG_DIR"),
            Some(&root.0.to_string_lossy().into_owned())
        );
    }

    #[test]
    fn successful_default_or_skip_clears_config_environment() {
        let defaults = super::configured_startup_app_for_test(
            &startup_test_options(WindowConfigOptions::default()),
            startup_test_discovery(),
        )
        .unwrap();
        assert_eq!(defaults.lifecycle.effective().generation, 1);
        assert!(defaults.app.derived_config_environment.is_empty());
        assert!(
            !defaults
                .app
                .pane_environment_variables()
                .contains_key("WEZTERM_CONFIG_FILE")
        );

        let mut stale_discovery = startup_test_discovery();
        stale_discovery.environment_config_file = Some(PathBuf::from("stale/wezterm.lua"));
        let skipped = super::configured_startup_app_for_test(
            &startup_test_options(WindowConfigOptions {
                skip_config: true,
                config_file: None,
                config_overrides: vec![(
                    "set_environment_variables".to_owned(),
                    "{ WEZTERM_CONFIG_FILE = 'user-stale' }".to_owned(),
                )],
            }),
            stale_discovery,
        )
        .unwrap();
        assert_eq!(skipped.lifecycle.effective().generation, 1);
        assert!(skipped.app.derived_config_environment.is_empty());
        assert_eq!(
            skipped
                .app
                .pane_environment_variables()
                .get("WEZTERM_CONFIG_FILE")
                .map(String::as_str),
            Some("user-stale")
        );
    }

    #[test]
    fn window_manager_successful_reload_advances_one_generation_for_all_apps() {
        let root = startup_test_dir("manager-success");
        let path = root.0.join("wezterm.lua");
        std::fs::write(&path, "return { term = 'generation-one' }").unwrap();
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path.clone()),
            config_overrides: Vec::new(),
        });
        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);
        manager
            .primary_app_mut_for_test()
            .dispatch_app_action(AppAction::SpawnWindow { launch: None })
            .unwrap();
        manager.collect_pending_window_apps_from_primary_for_test();
        let current_base = manager
            .config_lifecycle
            .as_ref()
            .unwrap()
            .effective()
            .clone();
        let mut materialized = NativeWindowApp::new(None);
        materialized.app_window_id = rssh_core::WindowId::new(3);
        materialized.set_base_config(&current_base, super::ReloadDisposition::SilentStartup);
        manager
            .windows
            .insert(winit::window::WindowId::dummy(), Box::new(materialized));

        let (applied, callbacks) = install_reload_transaction_observers(&mut manager);

        std::fs::write(&path, "return { term = 'generation-two' }").unwrap();
        assert!(manager.reload_configuration_attempt());

        assert_eq!(manager.config_generation_for_test(), Some(2));
        let apps = manager.all_apps_for_test();
        assert_eq!(apps.len(), 3);
        for app in apps {
            assert_eq!(app.base_config_generation_for_test(), 2);
            assert_eq!(app.base_config_source.as_ref(), Some(&path));
            assert_eq!(app.term, "generation-two");
            assert_eq!(
                app.derived_config_environment.get("WEZTERM_CONFIG_FILE"),
                Some(&path.to_string_lossy().into_owned())
            );
        }
        assert!(applied.lock().unwrap().iter().all(|applied| *applied));
        assert!(
            callbacks
                .iter()
                .all(|callback| callback.load(Ordering::Relaxed) == 1),
            "every managed app must receive exactly one callback"
        );
    }

    #[test]
    fn window_manager_reload_rebuilds_input_runtime_renderer_and_future_launch_state() {
        let root = startup_test_dir("manager-runtime");
        let path = root.0.join("wezterm.lua");
        std::fs::write(&path, "return { term = 'generation-one' }").unwrap();
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path.clone()),
            config_overrides: Vec::new(),
        });
        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);
        let sentinels = install_reload_runtime_sentinels(manager.primary_app_mut_for_test());

        std::fs::write(
            &path,
            r"return {
                term = 'generation-two',
                default_prog = { 'new-shell', '--login' },
                default_cwd = 'new-cwd',
                colors = { foreground = '#010203', background = '#040506' },
                keys = {
                    { key = 'k', mods = 'CTRL', action = wezterm.action.SendString('new') },
                },
            }",
        )
        .unwrap();

        assert!(manager.reload_configuration_attempt());
        let app = manager.primary_app_mut_for_test();
        assert_reload_runtime_sentinels_preserved(app, &sentinels);
        assert_eq!(app.term, "generation-two");
        assert_eq!(
            app.default_prog.as_deref(),
            Some(&["new-shell".to_owned(), "--login".to_owned()][..])
        );
        assert_eq!(app.default_cwd.as_deref(), Some("new-cwd"));
        assert_eq!(
            app.native_resolved_palette().foreground,
            Color::Rgb(1, 2, 3)
        );
        assert_eq!(
            app.native_resolved_palette().background,
            Color::Rgb(4, 5, 6)
        );
        assert_eq!(
            app.key_assignments
                .iter()
                .map(NativeUserKeyAssignment::test_projection)
                .collect::<Vec<_>>(),
            [("CTRL+k", Some("new"))]
        );

        sentinels.active_writer.lock().unwrap().clear();
        app.handle_pty_output(b"\x1bP+q544e\x1b\\").unwrap();
        assert_eq!(
            sentinels.active_writer.lock().unwrap().as_slice(),
            b"\x1bP1+r544E=67656E65726174696F6E2D74776F\x1b\\"
        );
        let layout = app.pane_render_layout();
        let active = layout
            .panes
            .iter()
            .find(|pane| pane.pane_id == sentinels.active_pane)
            .unwrap();
        let snapshot = app.render_snapshot();
        assert_eq!(
            snapshot_cell(&snapshot, active.row, active.column)
                .unwrap()
                .foreground,
            Color::Rgb(100, 150, 200),
            "the composed renderer snapshot must retain the live pane contents"
        );
        assert_eq!(
            snapshot_cell(&snapshot, active.row, active.column + 1)
                .unwrap()
                .foreground,
            Color::Default,
            "the default-colored live cell must remain palette-relative"
        );
        let (frame_width, frame_height) = app.frame_size_for_test();
        let mut frame =
            vec![0; usize::try_from(frame_width.saturating_mul(frame_height) * 4).unwrap()];
        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);
        assert!(
            frame.chunks_exact(4).any(|pixel| pixel == [1, 2, 3, 255]),
            "the renderer must project palette-relative text through the reloaded foreground"
        );
    }

    #[test]
    fn window_manager_failed_reload_keeps_lkg_generation_and_effective_state() {
        let root = startup_test_dir("manager-failure");
        let path = root.0.join("wezterm.lua");
        std::fs::write(
            &path,
            "return { term = 'last-good', colors = { background = '#102030' } }",
        )
        .unwrap();
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path.clone()),
            config_overrides: Vec::new(),
        });
        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();
        let expected_effective = configured.lifecycle.effective().clone();
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);
        let sentinels = install_reload_runtime_sentinels(manager.primary_app_mut_for_test());

        std::fs::write(&path, "return { term = dynamic_term() }").unwrap();
        assert!(!manager.reload_configuration_attempt());

        let lifecycle = manager.config_lifecycle.as_ref().unwrap();
        assert_eq!(lifecycle.effective(), &expected_effective);
        assert!(lifecycle.latest_diagnostic().is_some());
        let app = manager.primary_app_mut_for_test();
        assert_reload_runtime_sentinels_preserved(app, &sentinels);
        assert_eq!(app.base_config_generation_for_test(), 1);
        assert_eq!(app.base_config_source.as_ref(), Some(&path));
        assert_eq!(app.term, "last-good");
        assert_eq!(
            app.native_resolved_palette().background,
            Color::Rgb(16, 32, 48)
        );
        assert_eq!(
            app.derived_config_environment.get("WEZTERM_CONFIG_FILE"),
            Some(&path.to_string_lossy().into_owned())
        );
    }

    #[test]
    fn window_manager_failed_reload_still_notifies_each_window_once() {
        let root = startup_test_dir("manager-failure-notify");
        let path = root.0.join("wezterm.lua");
        std::fs::write(&path, "return {}").unwrap();
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path.clone()),
            config_overrides: Vec::new(),
        });
        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);
        manager
            .primary_app_mut_for_test()
            .dispatch_app_action(AppAction::SpawnWindow { launch: None })
            .unwrap();
        manager.collect_pending_window_apps_from_primary_for_test();
        let current_base = manager
            .config_lifecycle
            .as_ref()
            .unwrap()
            .effective()
            .clone();
        let mut materialized = NativeWindowApp::new(None);
        materialized.app_window_id = rssh_core::WindowId::new(3);
        materialized.set_base_config(&current_base, super::ReloadDisposition::SilentStartup);
        manager
            .windows
            .insert(winit::window::WindowId::dummy(), Box::new(materialized));
        let (applied, callbacks) = install_reload_transaction_observers(&mut manager);

        std::fs::write(&path, "return { unknown_runtime_field = true }").unwrap();
        assert!(!manager.reload_configuration_attempt());
        assert!(applied.lock().unwrap().iter().all(|applied| *applied));
        assert!(
            callbacks
                .iter()
                .all(|callback| callback.load(Ordering::Relaxed) == 1),
            "every managed app must receive exactly one failure callback"
        );
        assert_eq!(manager.config_generation_for_test(), Some(1));
    }

    #[test]
    fn window_manager_reload_rediscovers_optional_fallback_and_required_failure() {
        let root = startup_test_dir("manager-rediscovery");
        let home_source = root.0.join(".wezterm.lua");
        let xdg_root = root.0.join("xdg");
        let xdg_source = xdg_root.join("wezterm").join("wezterm.lua");
        std::fs::create_dir_all(xdg_source.parent().unwrap()).unwrap();
        std::fs::write(&home_source, "return { term = 'home' }").unwrap();
        std::fs::write(&xdg_source, "return { term = 'xdg' }").unwrap();
        let discovery = ConfigDiscoveryInputs {
            home_dir: Some(root.0.clone()),
            xdg_config_home: Some(xdg_root),
            ..startup_test_discovery()
        };
        let options = startup_test_options(WindowConfigOptions::default());
        let configured = super::configured_startup_app_for_test(&options, discovery).unwrap();
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);
        assert_eq!(manager.primary_app_mut_for_test().term, "home");

        std::fs::remove_file(&home_source).unwrap();
        assert!(manager.reload_configuration_attempt());
        assert_eq!(manager.primary_app_mut_for_test().term, "xdg");
        assert_eq!(manager.config_generation_for_test(), Some(2));
        std::fs::remove_file(&xdg_source).unwrap();
        assert!(manager.reload_configuration_attempt());
        assert_eq!(manager.primary_app_mut_for_test().term, super::DEFAULT_TERM);
        assert_eq!(manager.config_generation_for_test(), Some(3));
        assert!(
            manager
                .primary_app_mut_for_test()
                .derived_config_environment
                .is_empty()
        );
        assert!(
            manager
                .config_lifecycle
                .as_ref()
                .unwrap()
                .effective()
                .source
                .is_none()
        );

        let required = root.0.join("required.lua");
        std::fs::write(&required, "return { term = 'required' }").unwrap();
        let required_options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(required.clone()),
            config_overrides: Vec::new(),
        });
        let configured =
            super::configured_startup_app_for_test(&required_options, startup_test_discovery())
                .unwrap();
        let mut required_manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);
        std::fs::remove_file(required).unwrap();
        assert!(!required_manager.reload_configuration_attempt());
        assert_eq!(required_manager.primary_app_mut_for_test().term, "required");
        assert_eq!(required_manager.config_generation_for_test(), Some(1));
    }

    #[test]
    fn window_manager_successful_reload_clears_latest_diagnostic() {
        let root = startup_test_dir("manager-clear-diagnostic");
        let path = root.0.join("wezterm.lua");
        std::fs::write(&path, "return { term = dynamic_term() }").unwrap();
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path.clone()),
            config_overrides: Vec::new(),
        });
        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();
        assert!(configured.lifecycle.latest_diagnostic().is_some());
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);

        std::fs::write(&path, "return { term = 'recovered' }").unwrap();
        assert!(manager.reload_configuration_attempt());
        assert_eq!(manager.config_generation_for_test(), Some(1));
        assert!(
            manager
                .config_lifecycle
                .as_ref()
                .unwrap()
                .latest_diagnostic()
                .is_none()
        );
        assert_eq!(manager.primary_app_mut_for_test().term, "recovered");
    }

    #[test]
    fn window_override_survives_base_reload_and_remains_highest_precedence() {
        let root = startup_test_dir("manager-window-layer");
        let path = root.0.join("wezterm.lua");
        std::fs::write(
            &path,
            "return { term = 'base-one', default_cwd = 'base-one-cwd' }",
        )
        .unwrap();
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path.clone()),
            config_overrides: Vec::new(),
        });
        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);
        manager
            .primary_app_mut_for_test()
            .set_window_config_overrides(
                Some(super::NativeWindowConfigPatch::from_values(
                    native_window_config_patch_values! {
                    term: Some("window-term".to_owned()),
                    ..super::NativeWindowConfigPatchValues::default()
                    },
                )),
                super::ReloadDisposition::SilentStartup,
            );

        std::fs::write(
            &path,
            "return { term = 'base-two', default_cwd = 'base-two-cwd' }",
        )
        .unwrap();
        assert!(manager.reload_configuration_attempt());

        let app = manager.primary_app_mut_for_test();
        assert_eq!(app.base_config_generation_for_test(), 2);
        assert_eq!(app.base_config_overrides.term.as_deref(), Some("base-two"));
        assert_eq!(app.term, "window-term");
        assert_eq!(app.default_cwd.as_deref(), Some("base-two-cwd"));
        assert_eq!(
            app.window_config_overrides
                .as_ref()
                .and_then(|overrides| overrides.term.as_deref()),
            Some("window-term")
        );
    }

    #[test]
    fn pending_window_is_refreshed_to_current_generation_before_spawn() {
        let root = startup_test_dir("manager-pending-refresh");
        let path = root.0.join("wezterm.lua");
        std::fs::write(&path, "return { term = 'one' }").unwrap();
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path.clone()),
            config_overrides: Vec::new(),
        });
        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);
        manager
            .primary_app_mut_for_test()
            .dispatch_app_action(AppAction::SpawnWindow { launch: None })
            .unwrap();
        manager.collect_pending_window_apps_from_primary_for_test();
        assert_eq!(
            manager
                .pending_app_for_test(0)
                .unwrap()
                .base_config_generation_for_test(),
            1
        );
        {
            let pending = manager.pending_apps.get_mut(0).unwrap();
            assert!(
                pending.command_palette_execute(WindowCommand::ActivateKeyTable(
                    WindowActivateKeyTable {
                        name: "pending".to_owned(),
                        timeout_milliseconds: None,
                        one_shot: false,
                        replace_current: false,
                        until_unknown: false,
                        prevent_fallback: false,
                    },
                ))
            );
            pending.leader_active_since = Some(Instant::now());
        }

        std::fs::write(&path, "return { term = 'two' }").unwrap();
        manager.install_lifecycle_attempt_without_fanout_for_test();
        assert_eq!(manager.config_generation_for_test(), Some(2));
        assert_eq!(
            manager
                .pending_app_for_test(0)
                .unwrap()
                .base_config_generation_for_test(),
            1,
            "the fixture must represent a pending app queued before the new generation"
        );

        manager.refresh_pending_app_before_spawn_for_test(0);
        let pending = manager.pending_app_for_test(0).unwrap();
        assert_eq!(pending.base_config_generation_for_test(), 2);
        assert_eq!(pending.term, "two");
        assert_eq!(pending.active_key_table_for_test(), Some("pending"));
        assert!(
            pending.leader_active_since.is_some(),
            "silent pre-spawn refresh must not clear transient input state"
        );
    }

    #[test]
    fn detached_window_inherits_base_generation_and_window_layer() {
        let root = startup_test_dir("manager-detached-layer");
        let path = root.0.join("wezterm.lua");
        std::fs::write(
            &path,
            "return { term = 'base-one', default_cwd = 'base-one-cwd' }",
        )
        .unwrap();
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path.clone()),
            config_overrides: Vec::new(),
        });
        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);
        {
            let app = manager.primary_app_mut_for_test();
            app.set_window_config_overrides(
                Some(super::NativeWindowConfigPatch::from_values(
                    native_window_config_patch_values! {
                    term: Some("window".to_owned()),
                    ..super::NativeWindowConfigPatchValues::default()
                    },
                )),
                super::ReloadDisposition::SilentStartup,
            );
        }

        std::fs::write(
            &path,
            "return { term = 'base-two', default_cwd = 'base-two-cwd' }",
        )
        .unwrap();
        assert!(manager.install_lifecycle_attempt_without_fanout_for_test());
        assert_eq!(manager.config_generation_for_test(), Some(2));
        {
            let app = manager.primary_app_mut_for_test();
            assert_eq!(
                app.base_config_generation_for_test(),
                1,
                "the source app must remain on the old generation for this fixture"
            );
            assert_eq!(app.term, "window");
            app.dispatch_app_action(AppAction::SpawnWindow { launch: None })
                .unwrap();
        }
        manager.collect_pending_window_apps_from_primary_for_test();

        let detached = manager.pending_app_for_test(0).unwrap();
        assert_eq!(detached.base_config_generation_for_test(), 2);
        assert_eq!(
            detached.base_config_source.as_ref(),
            Some(&path),
            "the detached app must publish the manager's current source"
        );
        assert_eq!(
            detached
                .derived_config_environment
                .get("WEZTERM_CONFIG_FILE"),
            Some(&path.to_string_lossy().into_owned())
        );
        assert_eq!(
            detached.base_config_overrides.term.as_deref(),
            Some("base-two")
        );
        assert_eq!(detached.default_cwd.as_deref(), Some("base-two-cwd"));
        assert_eq!(detached.term, "window");
        assert_eq!(
            detached
                .window_config_overrides
                .as_ref()
                .and_then(|overrides| overrides.term.as_deref()),
            Some("window")
        );
    }

    fn write_reloaded_future_launch_config(path: &Path) {
        std::fs::write(
            path,
            r"return {
                default_prog = { 'reloaded-shell', '--login' },
                default_cwd = 'reloaded-cwd',
                term = 'reloaded-term',
                set_environment_variables = { USER_MARKER = 'reloaded-user-env' },
                colors = { foreground = '#010203', background = '#040506' },
                keys = {
                    { key = 'k', mods = 'CTRL', action = wezterm.action.SendString('reloaded-key') },
                },
            }",
        )
        .unwrap();
    }

    fn spawn_reloaded_split_tab_and_window(app: &mut NativeWindowApp) {
        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        let split = app
            .app_shell
            .active_tab()
            .panes()
            .iter()
            .find(|pane| pane.id() != rssh_core::PaneId::new(1))
            .unwrap();
        assert_eq!(split.launch().program(), "reloaded-shell");
        assert_eq!(split.launch().args(), ["--login"]);
        assert_eq!(split.launch().cwd(), Some("reloaded-cwd"));

        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        let tab_launch = app.app_shell.active_pane().launch();
        assert_eq!(tab_launch.program(), "reloaded-shell");
        assert_eq!(tab_launch.args(), ["--login"]);
        assert_eq!(tab_launch.cwd(), Some("reloaded-cwd"));
        let command = pty_command_from_pane_launch_with_environment(
            tab_launch,
            &app.term,
            &app.pane_environment_variables(),
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("TERM"), Some("reloaded-term"));

        app.dispatch_app_action(AppAction::SpawnWindow { launch: None })
            .unwrap();
    }

    fn assert_reloaded_detached_launch_and_environment(
        detached: &NativeWindowApp,
        path: &Path,
        config_dir: &Path,
    ) {
        assert_eq!(detached.base_config_generation_for_test(), 2);
        assert_eq!(detached.term, "reloaded-term");
        assert_eq!(
            detached.default_prog.as_deref(),
            Some(&["reloaded-shell".to_owned(), "--login".to_owned()][..])
        );
        assert_eq!(detached.default_cwd.as_deref(), Some("reloaded-cwd"));
        assert_eq!(detached.startup_command().program(), "reloaded-shell");
        assert_eq!(detached.startup_command().args(), ["--login"]);
        assert_eq!(
            detached.startup_command().cwd(),
            Some(std::path::Path::new("reloaded-cwd"))
        );
        assert_eq!(
            detached
                .derived_config_environment
                .get("WEZTERM_CONFIG_FILE"),
            Some(&path.to_string_lossy().into_owned())
        );
        assert_eq!(
            detached
                .derived_config_environment
                .get("WEZTERM_CONFIG_DIR"),
            Some(&config_dir.to_string_lossy().into_owned())
        );
        let environment = detached.pane_environment_variables();
        assert_eq!(
            environment.get("USER_MARKER").map(String::as_str),
            Some("reloaded-user-env")
        );
        let command = pty_command_from_pane_launch_with_environment(
            detached.app_shell.active_pane().launch(),
            &detached.term,
            &environment,
            detached.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("TERM"), Some("reloaded-term"));
        assert_eq!(
            command.env_value("WEZTERM_CONFIG_FILE"),
            Some(path.to_string_lossy().as_ref())
        );
        assert_eq!(
            command.env_value("WEZTERM_CONFIG_DIR"),
            Some(config_dir.to_string_lossy().as_ref())
        );
        assert_eq!(command.env_value("USER_MARKER"), Some("reloaded-user-env"));
        assert_eq!(
            detached
                .key_assignments
                .iter()
                .map(NativeUserKeyAssignment::test_projection)
                .collect::<Vec<_>>(),
            [("CTRL+k", Some("reloaded-key"))]
        );
        assert_eq!(
            detached.native_resolved_palette().foreground,
            Color::Rgb(1, 2, 3)
        );
        assert_eq!(
            detached.native_resolved_palette().background,
            Color::Rgb(4, 5, 6)
        );
    }

    fn assert_reloaded_detached_input_and_renderer(mut detached: Box<NativeWindowApp>) {
        let written = Arc::new(Mutex::new(Vec::new()));
        detached.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        detached.modifiers = ModifiersState::CONTROL;
        detached
            .handle_keyboard_input_event(
                &Key::Character("k".into()),
                PhysicalKey::Code(WinitKeyCode::KeyK),
                Some("k"),
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"reloaded-key",
            "the detached input path must execute the reloaded key assignment"
        );
        assert_eq!(detached.active_key_table_for_test(), None);
        assert!(detached.leader_active_since.is_none());
        let (frame_width, frame_height) = detached.frame_size_for_test();
        let mut frame =
            vec![0; usize::try_from(frame_width.saturating_mul(frame_height) * 4).unwrap()];
        assert_eq!(
            detached.render_framebuffer(&mut frame),
            FrameRenderMode::Full
        );
        assert!(
            frame.chunks_exact(4).any(|pixel| pixel == [4, 5, 6, 255]),
            "the detached renderer must inherit the reloaded background projection"
        );
    }

    #[test]
    fn new_split_and_tab_launches_use_reloaded_defaults() {
        let root = startup_test_dir("manager-future-launch");
        let path = root.0.join("wezterm.lua");
        std::fs::write(&path, "return {}").unwrap();
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path.clone()),
            config_overrides: Vec::new(),
        });
        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);

        write_reloaded_future_launch_config(&path);
        assert!(manager.reload_configuration_attempt());
        spawn_reloaded_split_tab_and_window(manager.primary_app_mut_for_test());
        manager.collect_pending_window_apps_from_primary_for_test();

        assert_reloaded_detached_launch_and_environment(
            manager.pending_app_for_test(0).unwrap(),
            &path,
            &root.0,
        );
        assert_reloaded_detached_input_and_renderer(manager.pending_apps.remove(0));
    }

    #[test]
    fn reload_clears_key_table_and_leader_state_once_per_attempt() {
        let root = startup_test_dir("manager-clear-transient");
        let path = root.0.join("wezterm.lua");
        std::fs::write(&path, "return {}").unwrap();
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path.clone()),
            config_overrides: Vec::new(),
        });
        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);
        let callbacks = Arc::new(AtomicUsize::new(0));
        {
            let app = manager.primary_app_mut_for_test();
            assert!(app.command_palette_execute(WindowCommand::ActivateKeyTable(
                WindowActivateKeyTable {
                    name: "transient".to_owned(),
                    timeout_milliseconds: None,
                    one_shot: false,
                    replace_current: false,
                    until_unknown: false,
                    prevent_fallback: false,
                },
            )));
            app.leader_active_since = Some(Instant::now());
            let callbacks = Arc::clone(&callbacks);
            app.config_reloaded_handler = Box::new(move |_| {
                callbacks.fetch_add(1, Ordering::Relaxed);
                true
            });
        }

        std::fs::write(&path, "return { term = 'new' }").unwrap();
        assert!(manager.reload_configuration_attempt());
        let app = manager.primary_app_mut_for_test();
        assert_eq!(app.active_key_table_for_test(), None);
        assert!(app.leader_active_since.is_none());
        assert_eq!(callbacks.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn window_manager_reload_user_event_runs_global_transaction() {
        let root = startup_test_dir("manager-event");
        let path = root.0.join("wezterm.lua");
        std::fs::write(&path, "return { term = 'before-event' }").unwrap();
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path.clone()),
            config_overrides: Vec::new(),
        });
        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);
        std::fs::write(&path, "return { term = 'after-event' }").unwrap();

        assert!(manager.handle_manager_user_event(&WindowUserEvent::ReloadConfigurationRequested));
        assert_eq!(manager.config_generation_for_test(), Some(2));
        assert_eq!(manager.primary_app_mut_for_test().term, "after-event");
    }

    #[test]
    fn initial_invalid_watched_config_recovers_to_generation_one() {
        let root = startup_test_dir("auto-initial-invalid");
        let config_dir = root.0.join("config");
        std::fs::create_dir_all(&config_dir).unwrap();
        let path = config_dir.join("wezterm.lua");
        std::fs::write(&path, "return dynamic_config").unwrap();
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path.clone()),
            config_overrides: Vec::new(),
        });
        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);
        manager
            .install_config_watcher_for_test(Duration::from_millis(1), Arc::new(|| true))
            .unwrap();
        assert_eq!(manager.config_generation_for_test(), Some(0));
        assert!(manager.watched_config_paths_for_test().contains(&path));

        std::fs::write(
            &path,
            "return { automatically_reload_config = true, term = 'recovered' }",
        )
        .unwrap();
        assert!(manager.handle_manager_user_event(&WindowUserEvent::ConfigFileChanged));

        assert_eq!(manager.config_generation_for_test(), Some(1));
        assert_eq!(manager.primary_app_mut_for_test().term, "recovered");
    }

    #[test]
    fn automatic_reload_updates_all_windows_once_after_burst() {
        let root = startup_test_dir("auto-all-windows");
        let path = root.0.join("wezterm.lua");
        std::fs::write(
            &path,
            "return { automatically_reload_config = true, term = 'one' }",
        )
        .unwrap();
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path.clone()),
            config_overrides: Vec::new(),
        });
        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);
        manager
            .primary_app_mut_for_test()
            .dispatch_app_action(AppAction::SpawnWindow { launch: None })
            .unwrap();
        manager.collect_pending_window_apps_from_primary_for_test();
        let current_base = manager
            .config_lifecycle
            .as_ref()
            .unwrap()
            .effective()
            .clone();
        let mut materialized = NativeWindowApp::new(None);
        materialized.app_window_id = rssh_core::WindowId::new(3);
        materialized.set_base_config(&current_base, super::ReloadDisposition::SilentStartup);
        manager
            .windows
            .insert(winit::window::WindowId::dummy(), Box::new(materialized));
        let (_, callbacks) = install_reload_transaction_observers(&mut manager);
        std::fs::write(
            &path,
            "return { automatically_reload_config = true, term = 'two' }",
        )
        .unwrap();
        let (event_sender, event_receiver) = mpsc::channel();
        manager
            .install_config_watcher_for_test(
                Duration::from_millis(10),
                Arc::new(move || {
                    event_sender
                        .send(WindowUserEvent::ConfigFileChanged)
                        .is_ok()
                }),
            )
            .unwrap();

        manager.enqueue_config_watch_burst_for_test(3);
        let event = event_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("one debounced manager event");
        assert!(
            event_receiver
                .recv_timeout(Duration::from_millis(50))
                .is_err(),
            "one watcher burst must enqueue only one manager event"
        );
        assert!(manager.handle_manager_user_event(&event));

        assert_eq!(manager.config_generation_for_test(), Some(2));
        assert_eq!(manager.all_apps_for_test().len(), 3);
        assert!(
            manager
                .all_apps_for_test()
                .iter()
                .all(|app| app.term == "two")
        );
        assert!(
            callbacks
                .iter()
                .all(|callback| callback.load(Ordering::Relaxed) == 1),
            "startup, pending, and materialized apps each receive one callback"
        );
    }

    #[test]
    fn per_window_auto_reload_override_does_not_control_global_watcher() {
        let root = startup_test_dir("auto-base-policy-only");
        let disabled_path = root.0.join("disabled.lua");
        std::fs::write(
            &disabled_path,
            "return { automatically_reload_config = false }",
        )
        .unwrap();
        let disabled = super::configured_startup_app_for_test(
            &startup_test_options(WindowConfigOptions {
                skip_config: false,
                config_file: Some(disabled_path),
                config_overrides: Vec::new(),
            }),
            startup_test_discovery(),
        )
        .unwrap();
        let mut disabled_manager = NativeWindowManager::new_for_test(disabled.app)
            .with_config_lifecycle(disabled.lifecycle);
        disabled_manager
            .primary_app_mut_for_test()
            .set_window_config_overrides(
                Some(super::NativeWindowConfigPatch::from_values(
                    native_window_config_patch_values! {
                    automatically_reload_config: Some(true),
                    ..super::NativeWindowConfigPatchValues::default()
                    },
                )),
                super::ReloadDisposition::SilentStartup,
            );
        assert!(
            disabled_manager
                .primary_app_mut_for_test()
                .automatically_reload_config
        );
        disabled_manager
            .install_config_watcher_for_test(Duration::from_millis(1), Arc::new(|| true))
            .unwrap();
        assert!(
            !disabled_manager.config_watcher_exists_for_test(),
            "a per-window true override cannot enable the global watcher"
        );

        let enabled_path = root.0.join("enabled.lua");
        std::fs::write(
            &enabled_path,
            "return { automatically_reload_config = true }",
        )
        .unwrap();
        let enabled = super::configured_startup_app_for_test(
            &startup_test_options(WindowConfigOptions {
                skip_config: false,
                config_file: Some(enabled_path.clone()),
                config_overrides: Vec::new(),
            }),
            startup_test_discovery(),
        )
        .unwrap();
        let mut enabled_manager =
            NativeWindowManager::new_for_test(enabled.app).with_config_lifecycle(enabled.lifecycle);
        enabled_manager
            .primary_app_mut_for_test()
            .set_window_config_overrides(
                Some(super::NativeWindowConfigPatch::from_values(
                    native_window_config_patch_values! {
                    automatically_reload_config: Some(false),
                    ..super::NativeWindowConfigPatchValues::default()
                    },
                )),
                super::ReloadDisposition::SilentStartup,
            );
        assert!(
            !enabled_manager
                .primary_app_mut_for_test()
                .automatically_reload_config
        );
        enabled_manager
            .install_config_watcher_for_test(Duration::from_millis(1), Arc::new(|| true))
            .unwrap();
        assert!(enabled_manager.config_watcher_exists_for_test());
        assert!(
            enabled_manager
                .watched_config_paths_for_test()
                .contains(&enabled_path),
            "a per-window false override cannot disable the manager-owned watcher"
        );
    }

    #[test]
    fn automatic_reload_recovers_after_invalid_intermediate_file() {
        let root = startup_test_dir("auto-invalid-intermediate");
        let path = root.0.join("wezterm.lua");
        std::fs::write(
            &path,
            "return { automatically_reload_config = true, term = 'good-one' }",
        )
        .unwrap();
        let configured = super::configured_startup_app_for_test(
            &startup_test_options(WindowConfigOptions {
                skip_config: false,
                config_file: Some(path.clone()),
                config_overrides: Vec::new(),
            }),
            startup_test_discovery(),
        )
        .unwrap();
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);
        manager
            .install_config_watcher_for_test(Duration::from_millis(1), Arc::new(|| true))
            .unwrap();
        let callbacks = Arc::new(AtomicUsize::new(0));
        {
            let callbacks = Arc::clone(&callbacks);
            manager.primary_app_mut_for_test().config_reloaded_handler = Box::new(move |_| {
                callbacks.fetch_add(1, Ordering::Relaxed);
                true
            });
        }

        std::fs::write(&path, "return dynamic_config").unwrap();
        assert!(manager.handle_manager_user_event(&WindowUserEvent::ConfigFileChanged));
        assert_eq!(manager.config_generation_for_test(), Some(1));
        assert_eq!(manager.primary_app_mut_for_test().term, "good-one");
        assert!(
            manager
                .config_lifecycle
                .as_ref()
                .unwrap()
                .latest_diagnostic()
                .is_some()
        );
        assert_eq!(callbacks.load(Ordering::Relaxed), 1);
        assert!(manager.config_watcher_exists_for_test());

        std::fs::write(
            &path,
            "return { automatically_reload_config = true, term = 'good-two' }",
        )
        .unwrap();
        assert!(manager.handle_manager_user_event(&WindowUserEvent::ConfigFileChanged));
        assert_eq!(manager.config_generation_for_test(), Some(2));
        assert_eq!(manager.primary_app_mut_for_test().term, "good-two");
        assert!(
            manager
                .config_lifecycle
                .as_ref()
                .unwrap()
                .latest_diagnostic()
                .is_none()
        );
        assert_eq!(callbacks.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn automatic_reload_atomic_replace_rediscovers_non_home_source() {
        let root = startup_test_dir("auto-atomic-replace");
        let config_dir = root.0.join("config");
        std::fs::create_dir_all(&config_dir).unwrap();
        let path = config_dir.join("wezterm.lua");
        std::fs::write(
            &path,
            "return { automatically_reload_config = true, term = 'before' }",
        )
        .unwrap();
        let configured = super::configured_startup_app_for_test(
            &startup_test_options(WindowConfigOptions {
                skip_config: false,
                config_file: Some(path.clone()),
                config_overrides: Vec::new(),
            }),
            ConfigDiscoveryInputs {
                home_dir: Some(root.0.join("home")),
                ..startup_test_discovery()
            },
        )
        .unwrap();
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);
        manager
            .install_config_watcher_for_test(Duration::from_millis(1), Arc::new(|| true))
            .unwrap();
        let watched = manager.watched_config_paths_for_test();
        assert!(watched.contains(&path));
        assert!(watched.contains(&config_dir));

        let replacement = config_dir.join("wezterm.lua.replacement");
        std::fs::write(
            &replacement,
            "return { automatically_reload_config = true, term = 'after' }",
        )
        .unwrap();
        std::fs::remove_file(&path).unwrap();
        std::fs::rename(&replacement, &path).unwrap();
        assert!(manager.handle_manager_user_event(&WindowUserEvent::ConfigFileChanged));

        assert_eq!(manager.config_generation_for_test(), Some(2));
        assert_eq!(manager.primary_app_mut_for_test().term, "after");
        assert_eq!(
            manager
                .primary_app_mut_for_test()
                .base_config_source
                .as_ref(),
            Some(&path)
        );
    }

    #[test]
    fn disabled_generation_does_not_add_new_watch_paths_but_existing_watch_remains() {
        let root = startup_test_dir("auto-disabled-new-source");
        let home = root.0.join("home");
        let xdg = root.0.join("xdg");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(xdg.join("wezterm")).unwrap();
        let home_file = home.join(".wezterm.lua");
        let xdg_file = xdg.join("wezterm/wezterm.lua");
        std::fs::write(
            &home_file,
            "return { automatically_reload_config = true, term = 'home' }",
        )
        .unwrap();
        let configured = super::configured_startup_app_for_test(
            &startup_test_options(WindowConfigOptions {
                skip_config: false,
                config_file: None,
                config_overrides: Vec::new(),
            }),
            ConfigDiscoveryInputs {
                is_windows: false,
                is_unix: false,
                current_exe: None,
                home_dir: Some(home.clone()),
                xdg_config_home: Some(xdg.clone()),
                xdg_config_dirs: Vec::new(),
                environment_config_file: None,
            },
        )
        .unwrap();
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);
        manager
            .install_config_watcher_for_test(Duration::from_millis(1), Arc::new(|| true))
            .unwrap();
        let watched_before = manager.watched_config_paths_for_test();
        assert_eq!(watched_before, [home_file.clone()].into_iter().collect());

        std::fs::remove_file(&home_file).unwrap();
        std::fs::write(
            &xdg_file,
            "return { automatically_reload_config = false, term = 'xdg-disabled' }",
        )
        .unwrap();
        assert!(manager.handle_manager_user_event(&WindowUserEvent::ConfigFileChanged));

        assert_eq!(manager.config_generation_for_test(), Some(2));
        assert_eq!(manager.primary_app_mut_for_test().term, "xdg-disabled");
        assert!(manager.config_watcher_exists_for_test());
        assert_eq!(
            manager.watched_config_paths_for_test(),
            watched_before,
            "disabled base generation retains the watcher but adds no new path"
        );
        assert!(!manager.watched_config_paths_for_test().contains(&xdg_file));
    }

    #[test]
    fn window_reload_command_enqueues_one_manager_event_before_global_transaction() {
        let root = startup_test_dir("manager-command-event");
        let path = root.0.join("wezterm.lua");
        std::fs::write(&path, "return { term = 'before-command' }").unwrap();
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path.clone()),
            config_overrides: Vec::new(),
        });
        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);
        let queued = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&queued);
        manager.primary_app_mut_for_test().reload_request_sender = Some(Arc::new(move |event| {
            assert!(matches!(
                event,
                WindowUserEvent::ReloadConfigurationRequested
            ));
            captured.lock().unwrap().push(event);
            true
        }));
        manager
            .primary_app_mut_for_test()
            .dispatch_app_action(AppAction::SpawnWindow { launch: None })
            .unwrap();
        manager.collect_pending_window_apps_from_primary_for_test();
        let callbacks = arm_reload_event_transaction_sentinels(&mut manager);
        std::fs::write(&path, "return { term = 'after-command' }").unwrap();

        assert!(
            manager
                .primary_app_mut_for_test()
                .command_palette_execute(WindowCommand::ReloadConfiguration)
        );
        let event = take_reload_event_before_global_dispatch(&manager, &queued, &callbacks);

        assert!(manager.handle_manager_user_event(&event));
        assert_global_reload_event_completed(&manager, &callbacks, "after-command");
    }

    #[test]
    fn window_reload_shortcut_enqueues_one_manager_event_before_global_transaction() {
        let root = startup_test_dir("manager-shortcut-event");
        let path = root.0.join("wezterm.lua");
        std::fs::write(&path, "return { term = 'before-shortcut' }").unwrap();
        let options = startup_test_options(WindowConfigOptions {
            skip_config: false,
            config_file: Some(path.clone()),
            config_overrides: Vec::new(),
        });
        let configured =
            super::configured_startup_app_for_test(&options, startup_test_discovery()).unwrap();
        let mut manager = NativeWindowManager::new_for_test(configured.app)
            .with_config_lifecycle(configured.lifecycle);
        let queued = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&queued);
        manager.primary_app_mut_for_test().reload_request_sender = Some(Arc::new(move |event| {
            assert!(matches!(
                event,
                WindowUserEvent::ReloadConfigurationRequested
            ));
            captured.lock().unwrap().push(event);
            true
        }));
        manager
            .primary_app_mut_for_test()
            .dispatch_app_action(AppAction::SpawnWindow { launch: None })
            .unwrap();
        manager.collect_pending_window_apps_from_primary_for_test();
        let callbacks = arm_reload_event_transaction_sentinels(&mut manager);
        std::fs::write(&path, "return { term = 'after-shortcut' }").unwrap();

        assert!(
            manager
                .pending_apps
                .get_mut(0)
                .unwrap()
                .handle_reload_configuration_shortcut(
                    &Key::Character("r".into()),
                    ModifiersState::CONTROL | ModifiersState::SHIFT
                )
        );
        let event = take_reload_event_before_global_dispatch(&manager, &queued, &callbacks);

        assert!(manager.handle_manager_user_event(&event));
        assert_global_reload_event_completed(&manager, &callbacks, "after-shortcut");
    }

    #[test]
    fn user_config_environment_overrides_derived_publication() {
        let root = startup_test_dir("publication-user-wins");
        let path = root.0.join("wezterm.lua");
        std::fs::write(
            &path,
            "return { set_environment_variables = {
                WEZTERM_CONFIG_FILE = 'user-file',
                WEZTERM_CONFIG_DIR = 'user-dir',
            } }",
        )
        .unwrap();
        let configured = super::configured_startup_app_for_test(
            &startup_test_options(WindowConfigOptions {
                skip_config: false,
                config_file: Some(path.clone()),
                config_overrides: Vec::new(),
            }),
            startup_test_discovery(),
        )
        .unwrap();

        assert_eq!(
            configured
                .app
                .derived_config_environment
                .get("WEZTERM_CONFIG_FILE"),
            Some(&path.to_string_lossy().into_owned())
        );
        let environment = configured.app.pane_environment_variables();
        assert_eq!(
            environment.get("WEZTERM_CONFIG_FILE").map(String::as_str),
            Some("user-file")
        );
        assert_eq!(
            environment.get("WEZTERM_CONFIG_DIR").map(String::as_str),
            Some("user-dir")
        );
    }

    #[test]
    fn window_app_applies_wezterm_lua_config_default_prog() {
        let overrides = crate::config_lifecycle::parse_native_config_document(
            "return { default_prog = { 'nu', '--login' } }",
            &[],
        )
        .unwrap();
        let mut app =
            NativeWindowApp::new_with_command(None, rssh_pty::PtyCommand::default_shell());

        app.apply_config_overrides_silently(overrides);

        assert_eq!(app.startup_command().program(), "nu");
        assert_eq!(app.startup_command().args(), ["--login"]);
        assert!(app.session.is_none());
    }

    #[test]
    fn window_app_applies_wezterm_lua_config_term() {
        let overrides = crate::config_lifecycle::parse_native_config_document(
            "return { term = 'wezterm-test-term' }",
            &[],
        )
        .unwrap();
        let mut app = NativeWindowApp::new(None);

        app.apply_config_overrides_silently(overrides);

        assert_eq!(app.term, "wezterm-test-term");
        let command = pty_command_from_pane_launch_with_environment(
            app.app_shell.active_pane().launch(),
            &app.term,
            &app.pane_environment_variables(),
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("TERM"), Some("wezterm-test-term"));
        assert!(app.session.is_none());
    }

    fn pane_overlay_copy_mode(
        row: u16,
        column: u16,
        selection_mode: super::WindowCopySelectionMode,
    ) -> super::WindowCopyMode {
        let source_cursor = SelectionSourceCell {
            domain: TerminalScreenDomain::Main,
            row: isize::try_from(row).expect("u16 row fits StableRowIndex on supported targets"),
            column: usize::from(column),
        };
        super::WindowCopyMode {
            cursor: SelectionCell { row, column },
            source_cursor,
            pending_jump: Some(super::WindowCopyPendingJump {
                forward: true,
                prev_char: false,
            }),
            last_jump: Some(super::WindowCopyJump {
                forward: false,
                prev_char: true,
                target: 'q',
            }),
            search_direction: Some(SearchDirection::Previous),
            selection_mode,
            anchor: Some(SelectionCell {
                row: row + 1,
                column: column + 1,
            }),
            source_anchor: Some(SelectionSourceCell {
                row: source_cursor.row + 1,
                column: source_cursor.column + 1,
                ..source_cursor
            }),
        }
    }

    fn set_app_search_for_test(window: &mut NativeWindowApp, search: WindowSearch) {
        let current = search.current;
        let initial_copy_mode = window.initial_copy_mode();
        window.active_ui.enter_search(initial_copy_mode, search);
        window.active_ui.set_search_current(current);
    }

    fn set_app_quick_select_for_test(
        window: &mut NativeWindowApp,
        quick_select: WindowQuickSelect,
    ) {
        window.active_ui.enter_quick_select(quick_select);
    }

    fn assert_app_search_mode(window: &NativeWindowApp) {
        assert_eq!(
            copy_search_mode_for_test(window),
            Some(super::WindowCopySearchMode::Search)
        );
        assert!(search_for_test(window).is_some());
    }

    fn search_for_test(window: &NativeWindowApp) -> Option<&WindowSearch> {
        window.active_ui.retained_search()
    }

    fn quick_select_for_test(window: &NativeWindowApp) -> Option<&WindowQuickSelect> {
        window.active_ui.quick_select()
    }

    fn copy_mode_for_test(window: &NativeWindowApp) -> Option<&super::WindowCopyMode> {
        window.active_ui.retained_copy_mode()
    }

    fn copy_search_mode_for_test(window: &NativeWindowApp) -> Option<super::WindowCopySearchMode> {
        window.active_ui.copy_search_mode()
    }

    fn overlay_active_for_test(window: &NativeWindowApp) -> bool {
        window.active_ui.overlay_active()
    }

    fn ordinary_selection_for_test(window: &NativeWindowApp) -> Option<StableOrdinarySelection> {
        window.active_ui.ordinary_selection
    }

    fn set_ordinary_selection_for_test(
        window: &mut NativeWindowApp,
        selection: Option<StableOrdinarySelection>,
    ) {
        window.active_ui.ordinary_selection = selection;
    }

    fn active_search_for_test(app: &NativeWindowApp) -> &WindowSearch {
        search_for_test(app).expect("search mode should be active")
    }

    fn active_quick_select_for_test(app: &NativeWindowApp) -> &WindowQuickSelect {
        quick_select_for_test(app).expect("quick select mode should be active")
    }

    fn active_copy_mode_for_test(app: &NativeWindowApp) -> &super::WindowCopyMode {
        copy_mode_for_test(app).expect("copy mode should be active")
    }

    fn retained_copy_mode_mut_for_test(app: &mut NativeWindowApp) -> &mut super::WindowCopyMode {
        app.active_ui
            .retained_copy_mode_mut()
            .expect("copy mode state should be retained")
    }

    fn assert_pane_overlay_copy_mode(
        copy_mode: &super::WindowCopyMode,
        row: u16,
        column: u16,
        selection_mode: super::WindowCopySelectionMode,
    ) {
        assert_eq!(copy_mode.cursor, SelectionCell { row, column });
        assert_eq!(
            copy_mode.source_cursor,
            SelectionSourceCell {
                domain: TerminalScreenDomain::Main,
                row: isize::try_from(row)
                    .expect("u16 row fits StableRowIndex on supported targets"),
                column: usize::from(column),
            }
        );
        assert_eq!(
            copy_mode.pending_jump,
            Some(super::WindowCopyPendingJump {
                forward: true,
                prev_char: false,
            })
        );
        assert_eq!(
            copy_mode.last_jump,
            Some(super::WindowCopyJump {
                forward: false,
                prev_char: true,
                target: 'q',
            })
        );
        assert_eq!(copy_mode.search_direction, Some(SearchDirection::Previous));
        assert_eq!(copy_mode.selection_mode, selection_mode);
        assert_eq!(
            copy_mode.anchor,
            Some(SelectionCell {
                row: row + 1,
                column: column + 1,
            })
        );
        assert_eq!(
            copy_mode.source_anchor,
            Some(SelectionSourceCell {
                domain: TerminalScreenDomain::Main,
                row: isize::try_from(row)
                    .expect("u16 row fits StableRowIndex on supported targets")
                    + 1,
                column: usize::from(column) + 1,
            })
        );
    }

    fn pane_overlay_search(
        query: &str,
        match_type: WindowSearchMatchType,
        current: Option<WindowSearchMatch>,
        editing: bool,
    ) -> WindowSearch {
        WindowSearch {
            query: query.to_owned(),
            current,
            match_type,
            editing,
        }
    }

    fn pane_overlay_match(column: u16) -> WindowSearchMatch {
        WindowSearchMatch {
            domain: TerminalScreenDomain::Main,
            source_row: 0,
            start_column: column,
            end_source_row: 0,
            end_column: column + 2,
        }
    }

    fn pane_overlay_quick_select(label: &str) -> WindowQuickSelect {
        WindowQuickSelect {
            matches: vec![pane_overlay_match(4)],
            labels: vec![label.to_owned()],
            ..WindowQuickSelect::default()
        }
    }

    #[test]
    fn pane_transient_overlay_search_and_copy_share_one_slot() {
        let initial_copy_mode = pane_overlay_copy_mode(3, 7, super::WindowCopySelectionMode::Word);
        let retained_match = pane_overlay_match(7);
        let mut state = super::PaneUiState::default();

        state.enter_search(
            initial_copy_mode,
            pane_overlay_search("needle", WindowSearchMatchType::CaseSensitive, None, false),
        );
        assert!(state.set_search_current(Some(retained_match)));
        assert!(state.overlay_active());
        assert_eq!(
            state.copy_search_mode(),
            Some(super::WindowCopySearchMode::Search)
        );
        assert!(state.copy_mode().is_none());
        assert_pane_overlay_copy_mode(
            state
                .retained_copy_mode()
                .expect("Search controller must own its hidden Copy state"),
            3,
            7,
            super::WindowCopySelectionMode::Word,
        );
        assert!(state.search().is_some_and(|search| search.editing));

        state.enter_copy_mode(pane_overlay_copy_mode(
            9,
            11,
            super::WindowCopySelectionMode::Line,
        ));
        assert_eq!(
            state.copy_search_mode(),
            Some(super::WindowCopySearchMode::Copy)
        );
        assert_pane_overlay_copy_mode(
            state
                .copy_mode()
                .expect("copy mode should retain the search controller"),
            3,
            7,
            super::WindowCopySelectionMode::Word,
        );
        assert_eq!(
            state.retained_search().and_then(|search| search.current),
            Some(retained_match)
        );
        assert!(
            state
                .retained_search()
                .is_some_and(|search| !search.editing)
        );

        state.enter_search(
            pane_overlay_copy_mode(1, 1, super::WindowCopySelectionMode::Cell),
            pane_overlay_search("needle", WindowSearchMatchType::CaseSensitive, None, false),
        );
        assert_eq!(
            state.copy_search_mode(),
            Some(super::WindowCopySearchMode::Search)
        );
        assert_pane_overlay_copy_mode(
            state
                .retained_copy_mode()
                .expect("search should retain the copy-mode state"),
            3,
            7,
            super::WindowCopySelectionMode::Word,
        );
        assert_eq!(
            state.search().and_then(|search| search.current),
            Some(retained_match)
        );
        assert!(state.search().is_some_and(|search| search.editing));
    }

    #[test]
    fn pane_transient_overlay_new_search_pattern_invalidates_results() {
        let mut state = super::PaneUiState::default();
        let copy_mode = pane_overlay_copy_mode(2, 5, super::WindowCopySelectionMode::Block);

        state.enter_copy_mode(copy_mode);
        state.enter_search(
            pane_overlay_copy_mode(7, 7, super::WindowCopySelectionMode::Line),
            pane_overlay_search(
                "alpha",
                WindowSearchMatchType::CaseSensitive,
                Some(pane_overlay_match(1)),
                true,
            ),
        );
        assert_eq!(
            state.search().and_then(|search| search.current),
            None,
            "a newly initialized pattern must wait for recalculation"
        );
        assert!(state.set_search_current(Some(pane_overlay_match(1))));
        state.enter_copy_mode(pane_overlay_copy_mode(
            8,
            8,
            super::WindowCopySelectionMode::Line,
        ));
        state.enter_search(
            pane_overlay_copy_mode(9, 9, super::WindowCopySelectionMode::Cell),
            pane_overlay_search(
                "beta",
                WindowSearchMatchType::CaseSensitive,
                Some(pane_overlay_match(9)),
                false,
            ),
        );
        let search = state
            .search()
            .expect("different query should still enter search");
        assert_eq!(search.query, "beta");
        assert_eq!(search.match_type, WindowSearchMatchType::CaseSensitive);
        assert_eq!(search.current, None);
        assert_pane_overlay_copy_mode(
            state
                .retained_copy_mode()
                .expect("new query should preserve copy-mode state"),
            2,
            5,
            super::WindowCopySelectionMode::Block,
        );

        assert!(state.set_search_current(Some(pane_overlay_match(2))));
        assert_eq!(
            state.replace_search_pattern("beta".to_owned(), WindowSearchMatchType::Regex),
            Some(true)
        );
        let search = state
            .search()
            .expect("pattern replacement should keep search mode active");
        assert_eq!(search.query, "beta");
        assert_eq!(search.match_type, WindowSearchMatchType::Regex);
        assert_eq!(search.current, None);
        assert!(search.editing);
        assert_pane_overlay_copy_mode(
            state
                .retained_copy_mode()
                .expect("new match type should preserve copy-mode state"),
            2,
            5,
            super::WindowCopySelectionMode::Block,
        );
    }

    #[test]
    fn pane_transient_overlay_quick_select_replaces_copy_search_without_restore() {
        let mut state = super::PaneUiState::default();
        state.enter_search(
            pane_overlay_copy_mode(0, 3, super::WindowCopySelectionMode::Cell),
            pane_overlay_search(
                "before-quick",
                WindowSearchMatchType::CaseInsensitive,
                Some(pane_overlay_match(3)),
                true,
            ),
        );

        state.enter_quick_select(pane_overlay_quick_select("a"));
        assert!(state.copy_search_mode().is_none());
        assert!(state.search().is_none());
        assert_eq!(
            state
                .quick_select_mut()
                .map(|quick_select| quick_select.input.push('a')),
            Some(())
        );
        assert_eq!(
            state
                .quick_select()
                .map(|quick_select| quick_select.input.as_str()),
            Some("a")
        );

        state.exit_overlay();
        assert!(!state.overlay_active());
        assert!(state.copy_search_mode().is_none());
        assert!(state.search().is_none());
        assert!(state.quick_select().is_none());
    }

    #[test]
    fn pane_transient_overlay_search_mode_always_has_search_state() {
        let mut state = super::PaneUiState::default();
        state.enter_search(
            pane_overlay_copy_mode(1, 2, super::WindowCopySelectionMode::None),
            pane_overlay_search("first", WindowSearchMatchType::CaseSensitive, None, false),
        );
        assert_eq!(
            state.copy_search_mode(),
            Some(super::WindowCopySearchMode::Search)
        );
        assert!(state.search().is_some_and(|search| search.editing));

        state.enter_copy_mode(pane_overlay_copy_mode(
            4,
            5,
            super::WindowCopySelectionMode::Word,
        ));
        assert_eq!(
            state.copy_search_mode(),
            Some(super::WindowCopySearchMode::Copy)
        );
        assert!(state.search().is_none());
        assert!(
            state
                .retained_search()
                .is_some_and(|search| !search.editing)
        );
        assert_eq!(
            state.replace_search_pattern("copy-pattern".to_owned(), WindowSearchMatchType::Regex,),
            Some(true)
        );
        assert_eq!(
            state.retained_search().map(|search| (
                search.query.as_str(),
                search.current,
                search.match_type,
                search.editing,
            )),
            Some(("copy-pattern", None, WindowSearchMatchType::Regex, false))
        );

        state.enter_search(
            pane_overlay_copy_mode(8, 9, super::WindowCopySelectionMode::Line),
            pane_overlay_search("copy-pattern", WindowSearchMatchType::Regex, None, false),
        );
        assert_eq!(
            state.copy_search_mode(),
            Some(super::WindowCopySearchMode::Search)
        );
        assert!(state.search().is_some_and(|search| search.editing));
    }

    #[test]
    fn pane_transient_overlay_empty_slot_enters_copy_mode() {
        let mut state = super::PaneUiState::default();

        state.enter_copy_mode(pane_overlay_copy_mode(
            6,
            4,
            super::WindowCopySelectionMode::SemanticZone,
        ));

        assert_eq!(
            state.copy_search_mode(),
            Some(super::WindowCopySearchMode::Copy)
        );
        assert_pane_overlay_copy_mode(
            state
                .copy_mode()
                .expect("empty slot should create a copy/search controller"),
            6,
            4,
            super::WindowCopySelectionMode::SemanticZone,
        );
        assert!(state.retained_search().is_none());
        assert!(state.search().is_none());
        assert!(state.quick_select().is_none());
        assert!(state.overlay_active());

        state
            .copy_mode_mut()
            .expect("copy-mode data should be safely mutable")
            .selection_mode = super::WindowCopySelectionMode::Cell;
        assert_eq!(
            state.copy_mode().map(|copy_mode| copy_mode.selection_mode),
            Some(super::WindowCopySelectionMode::Cell)
        );
    }

    #[test]
    fn pane_transient_overlay_standalone_search_accepts_as_copy() {
        let mut state = super::PaneUiState::default();
        state.enter_search(
            pane_overlay_copy_mode(2, 4, super::WindowCopySelectionMode::Word),
            pane_overlay_search(
                "standalone",
                WindowSearchMatchType::CaseSensitive,
                Some(pane_overlay_match(2)),
                true,
            ),
        );
        assert!(state.set_search_current(Some(pane_overlay_match(2))));

        assert!(state.set_search_editing(false));

        assert_eq!(
            state.copy_search_mode(),
            Some(super::WindowCopySearchMode::Copy)
        );
        assert!(
            state
                .retained_search()
                .is_some_and(|search| search.query == "standalone"
                    && search.current == Some(pane_overlay_match(2))
                    && !search.editing)
        );
        assert_pane_overlay_copy_mode(
            state
                .retained_copy_mode()
                .expect("accepted Search must retain its controller's Copy state"),
            2,
            4,
            super::WindowCopySelectionMode::Word,
        );
        assert_pane_overlay_copy_mode(
            state
                .copy_mode()
                .expect("accepted Search must promote the same controller to active Copy"),
            2,
            4,
            super::WindowCopySelectionMode::Word,
        );
    }

    #[test]
    fn pane_transient_overlay_copy_accept_requires_search_state() {
        let mut empty = super::PaneUiState::default();
        assert!(!empty.set_search_editing(false));
        assert!(!empty.overlay_active());

        let mut copy = super::PaneUiState::default();
        copy.enter_copy_mode(pane_overlay_copy_mode(
            3,
            5,
            super::WindowCopySelectionMode::Line,
        ));
        assert!(!copy.set_search_editing(false));
        assert_eq!(
            copy.copy_search_mode(),
            Some(super::WindowCopySearchMode::Copy)
        );
        assert!(copy.retained_search().is_none());
        assert_pane_overlay_copy_mode(
            copy.copy_mode()
                .expect("rejected accept must preserve the existing Copy controller"),
            3,
            5,
            super::WindowCopySelectionMode::Line,
        );
    }

    #[test]
    fn pane_transient_overlay_search_or_copy_replaces_quick_select() {
        let mut search_state = super::PaneUiState::default();
        search_state.enter_quick_select(pane_overlay_quick_select("s"));
        search_state.enter_search(
            pane_overlay_copy_mode(3, 1, super::WindowCopySelectionMode::Cell),
            pane_overlay_search("replacement", WindowSearchMatchType::Regex, None, false),
        );
        assert!(search_state.quick_select().is_none());
        assert_eq!(
            search_state
                .search()
                .map(|search| (search.query.as_str(), search.editing)),
            Some(("replacement", true))
        );

        let mut copy_state = super::PaneUiState::default();
        copy_state.enter_quick_select(pane_overlay_quick_select("c"));
        copy_state.enter_copy_mode(pane_overlay_copy_mode(
            5,
            2,
            super::WindowCopySelectionMode::Block,
        ));
        assert!(copy_state.quick_select().is_none());
        assert_eq!(
            copy_state.copy_search_mode(),
            Some(super::WindowCopySearchMode::Copy)
        );
        assert!(copy_state.search().is_none());
    }

    fn set_badge_format(app: &mut NativeWindowApp, format: &str) {
        let sequence = format!(
            "\x1b]1337;SetBadgeFormat={}\x07",
            STANDARD.encode(format.as_bytes())
        );
        app.handle_pty_output(sequence.as_bytes()).unwrap();
    }

    struct ChildProcessGuard(Child);

    impl ChildProcessGuard {
        fn id(&self) -> u32 {
            self.0.id()
        }
    }

    impl Drop for ChildProcessGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[cfg(target_os = "windows")]
    fn spawn_sleeping_child(cwd: &Path) -> io::Result<ChildProcessGuard> {
        // Use a leaf process: PowerShell can create helper descendants whose
        // working directory is unrelated to the requested one, while the
        // production resolver deliberately prefers the deepest descendant.
        Command::new("ping.exe")
            .args(["-n", "31", "127.0.0.1"])
            .current_dir(cwd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(ChildProcessGuard)
    }

    #[cfg(not(target_os = "windows"))]
    fn spawn_sleeping_child(cwd: &Path) -> io::Result<ChildProcessGuard> {
        Command::new("sh")
            .args(["-c", "sleep 30"])
            .current_dir(cwd)
            .spawn()
            .map(ChildProcessGuard)
    }

    fn test_home_dir() -> Option<PathBuf> {
        std::env::var_os("USERPROFILE")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                let drive = std::env::var_os("HOMEDRIVE")?;
                let path = std::env::var_os("HOMEPATH")?;
                if drive.is_empty() || path.is_empty() {
                    return None;
                }
                let mut home = drive;
                home.push(path);
                Some(PathBuf::from(home))
            })
            .or_else(|| {
                std::env::var_os("HOME")
                    .filter(|value| !value.is_empty())
                    .map(PathBuf::from)
            })
    }

    #[test]
    fn wezterm_default_colors_palette_matches_pinned_upstream() {
        const CUBE_RAMP: [u8; 6] = [0x00, 0x5f, 0x87, 0xaf, 0xd7, 0xff];

        let palette = super::native_wezterm_default_colors_palette();

        assert_eq!(palette.foreground, Color::Rgb(0xb2, 0xb2, 0xb2));
        assert_eq!(palette.background, Color::Rgb(0x00, 0x00, 0x00));
        assert_eq!(palette.cursor_fg, Some(Color::Rgb(0x00, 0x00, 0x00)));
        assert_eq!(palette.cursor_bg, Color::Rgb(0x52, 0xad, 0x70));
        assert_eq!(palette.cursor_border, Some(Color::Rgb(0x52, 0xad, 0x70)));
        assert_eq!(palette.selection_fg, Some(None));
        assert_eq!(palette.selection_bg, Some(Color::Rgba(127, 102, 153, 127)));
        assert_eq!(palette.scrollbar_thumb, Some(Color::Rgb(0x22, 0x22, 0x22)));
        assert_eq!(palette.split, Some(Color::Rgb(0x44, 0x44, 0x44)));

        assert_eq!(
            palette.ansi,
            [
                Color::Rgb(0x00, 0x00, 0x00),
                Color::Rgb(0xcc, 0x55, 0x55),
                Color::Rgb(0x55, 0xcc, 0x55),
                Color::Rgb(0xcd, 0xcd, 0x55),
                Color::Rgb(0x54, 0x55, 0xcb),
                Color::Rgb(0xcc, 0x55, 0xcc),
                Color::Rgb(0x7a, 0xca, 0xca),
                Color::Rgb(0xcc, 0xcc, 0xcc),
            ]
        );
        assert_eq!(
            palette.brights,
            [
                Color::Rgb(0x55, 0x55, 0x55),
                Color::Rgb(0xff, 0x55, 0x55),
                Color::Rgb(0x55, 0xff, 0x55),
                Color::Rgb(0xff, 0xff, 0x55),
                Color::Rgb(0x55, 0x55, 0xff),
                Color::Rgb(0xff, 0x55, 0xff),
                Color::Rgb(0x55, 0xff, 0xff),
                Color::Rgb(0xff, 0xff, 0xff),
            ]
        );

        assert!(palette.indexed[..16].iter().all(Option::is_none));
        assert_eq!(
            palette.indexed[16..]
                .iter()
                .filter(|color| color.is_some())
                .count(),
            240
        );

        let mut index = 16;
        for red in CUBE_RAMP {
            for green in CUBE_RAMP {
                for blue in CUBE_RAMP {
                    assert_eq!(
                        palette.indexed[index],
                        Some(Color::Rgb(red, green, blue)),
                        "unexpected xterm color cube entry at index {index}"
                    );
                    index += 1;
                }
            }
        }
        assert_eq!(index, 232);

        for index in 232..256 {
            let grey = 8 + 10 * u8::try_from(index - 232).expect("grey index fits in u8");
            assert_eq!(
                palette.indexed[index],
                Some(Color::Rgb(grey, grey, grey)),
                "unexpected xterm grey ramp entry at index {index}"
            );
        }

        for (index, expected) in [
            (16, Color::Rgb(0x00, 0x00, 0x00)),
            (17, Color::Rgb(0x00, 0x00, 0x5f)),
            (21, Color::Rgb(0x00, 0x00, 0xff)),
            (22, Color::Rgb(0x00, 0x5f, 0x00)),
            (51, Color::Rgb(0x00, 0xff, 0xff)),
            (52, Color::Rgb(0x5f, 0x00, 0x00)),
            (88, Color::Rgb(0x87, 0x00, 0x00)),
            (124, Color::Rgb(0xaf, 0x00, 0x00)),
            (160, Color::Rgb(0xd7, 0x00, 0x00)),
            (196, Color::Rgb(0xff, 0x00, 0x00)),
            (231, Color::Rgb(0xff, 0xff, 0xff)),
            (232, Color::Rgb(0x08, 0x08, 0x08)),
            (249, Color::Rgb(0xb2, 0xb2, 0xb2)),
            (255, Color::Rgb(0xee, 0xee, 0xee)),
        ] {
            assert_eq!(palette.indexed[index], Some(expected));
        }

        assert_eq!(palette.tab_bar_background, None);
        assert_eq!(palette.tab_bar_inactive_tab_edge, None);
        assert_eq!(
            palette.tab_bar_active_tab,
            NativeTabBarItemColors::default()
        );
        assert_eq!(
            palette.tab_bar_inactive_tab,
            NativeTabBarItemColors::default()
        );
        assert_eq!(
            palette.tab_bar_inactive_tab_hover,
            NativeTabBarItemColors::default()
        );
        assert_eq!(palette.tab_bar_new_tab, NativeTabBarItemColors::default());
        assert_eq!(
            palette.tab_bar_new_tab_hover,
            NativeTabBarItemColors::default()
        );
        assert_eq!(palette.visual_bell, None);
        assert_eq!(palette.compose_cursor, None);
        assert_eq!(palette.copy_mode_active_highlight_fg, None);
        assert_eq!(palette.copy_mode_active_highlight_bg, None);
        assert_eq!(palette.copy_mode_inactive_highlight_fg, None);
        assert_eq!(palette.copy_mode_inactive_highlight_bg, None);
        assert_eq!(palette.quick_select_label_fg, None);
        assert_eq!(palette.quick_select_label_bg, None);
        assert_eq!(palette.quick_select_match_fg, None);
        assert_eq!(palette.quick_select_match_bg, None);
        assert_eq!(palette.input_selector_label_fg, None);
        assert_eq!(palette.input_selector_label_bg, None);
        assert_eq!(palette.launcher_label_fg, None);
        assert_eq!(palette.launcher_label_bg, None);
    }

    #[test]
    fn demo_snapshot_contains_visible_terminal_cells() {
        let snapshot = demo_snapshot();

        assert!(!snapshot.cells().is_empty());
    }

    #[test]
    fn encodes_window_text_input_for_pty() {
        let bytes = encode_window_key(
            &Key::Character("中".into()),
            PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
            Some("中"),
            ModifiersState::empty(),
            false,
            false,
        );

        assert_eq!(bytes, "中".as_bytes());
    }

    #[test]
    fn encodes_window_control_input_for_pty() {
        let bytes = encode_window_key(
            &Key::Character("c".into()),
            PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
            None,
            ModifiersState::CONTROL,
            false,
            false,
        );

        assert_eq!(bytes, vec![3]);
    }

    #[test]
    fn encodes_window_alt_text_with_escape_prefix() {
        let bytes = encode_window_key(
            &Key::Character("x".into()),
            PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
            Some("x"),
            ModifiersState::ALT,
            false,
            false,
        );

        assert_eq!(bytes, b"\x1bx");
    }

    #[test]
    fn encodes_window_kitty_disambiguated_ascii_keys_when_enabled() {
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("i".into()),
                PhysicalKey::Code(WinitKeyCode::KeyI),
                None,
                ModifiersState::CONTROL,
                false,
                false,
                1,
                0
            ),
            b"\x1b[105;5u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("I".into()),
                PhysicalKey::Code(WinitKeyCode::KeyI),
                None,
                ModifiersState::CONTROL | ModifiersState::SHIFT,
                false,
                false,
                1,
                0
            ),
            b"\x1b[105;6u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("+".into()),
                PhysicalKey::Code(WinitKeyCode::Equal),
                Some("+"),
                ModifiersState::CONTROL | ModifiersState::SHIFT,
                false,
                false,
                1,
                0
            ),
            b"\x1b[61;6u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("i".into()),
                PhysicalKey::Code(WinitKeyCode::KeyI),
                Some("i"),
                ModifiersState::ALT,
                false,
                false,
                1,
                0
            ),
            b"\x1b[105;3u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("i".into()),
                PhysicalKey::Code(WinitKeyCode::KeyI),
                None,
                ModifiersState::SUPER,
                false,
                false,
                8,
                0
            ),
            b"\x1b[105;9u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("I".into()),
                PhysicalKey::Code(WinitKeyCode::KeyI),
                None,
                ModifiersState::SUPER | ModifiersState::SHIFT,
                false,
                false,
                8,
                0
            ),
            b"\x1b[105;10u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("i".into()),
                PhysicalKey::Code(WinitKeyCode::KeyI),
                None,
                ModifiersState::CONTROL,
                false,
                false,
                0,
                0
            ),
            b"\t"
        );
    }

    #[test]
    fn encodes_window_kitty_report_all_ascii_and_basic_functional_keys_when_enabled() {
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("a".into()),
                PhysicalKey::Code(WinitKeyCode::KeyA),
                Some("a"),
                ModifiersState::empty(),
                false,
                false,
                8,
                0
            ),
            b"\x1b[97u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("A".into()),
                PhysicalKey::Code(WinitKeyCode::KeyA),
                Some("A"),
                ModifiersState::SHIFT,
                false,
                false,
                8,
                0
            ),
            b"\x1b[97;2u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("i".into()),
                PhysicalKey::Code(WinitKeyCode::KeyI),
                None,
                ModifiersState::CONTROL,
                false,
                false,
                8,
                0
            ),
            b"\x1b[105;5u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::Enter),
                PhysicalKey::Code(WinitKeyCode::Enter),
                Some("\r"),
                ModifiersState::empty(),
                false,
                false,
                8,
                0
            ),
            b"\x1b[13u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::Tab),
                PhysicalKey::Code(WinitKeyCode::Tab),
                Some("\t"),
                ModifiersState::empty(),
                false,
                false,
                8,
                0
            ),
            b"\x1b[9u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::Backspace),
                PhysicalKey::Code(WinitKeyCode::Backspace),
                None,
                ModifiersState::empty(),
                false,
                false,
                8,
                0
            ),
            b"\x1b[127u"
        );
    }

    #[test]
    fn encodes_window_kitty_associated_text_when_enabled() {
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("A".into()),
                PhysicalKey::Code(WinitKeyCode::KeyA),
                Some("A"),
                ModifiersState::SHIFT,
                false,
                false,
                24,
                0
            ),
            b"\x1b[97;2;65u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("a".into()),
                PhysicalKey::Code(WinitKeyCode::KeyA),
                Some("a"),
                ModifiersState::empty(),
                false,
                false,
                24,
                0
            ),
            b"\x1b[97;;97u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("e\u{301}".into()),
                PhysicalKey::Code(WinitKeyCode::KeyE),
                Some("e\u{301}"),
                ModifiersState::empty(),
                false,
                false,
                24,
                0
            ),
            b"\x1b[101;;101:769u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("å".into()),
                PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
                Some("å"),
                ModifiersState::empty(),
                false,
                false,
                24,
                0
            ),
            b"\x1b[0;;229u"
        );
        assert_eq!(
            encode_window_key_with_kitty_event(
                &Key::Character("a".into()),
                PhysicalKey::Code(WinitKeyCode::KeyA),
                Some("a"),
                ModifiersState::empty(),
                false,
                false,
                26,
                0,
                KittyKeyEventKind::Repeat
            ),
            b"\x1b[97;1:2;97u"
        );
        assert_eq!(
            encode_window_key_with_kitty_event(
                &Key::Character("a".into()),
                PhysicalKey::Code(WinitKeyCode::KeyA),
                Some("a"),
                ModifiersState::empty(),
                false,
                false,
                26,
                0,
                KittyKeyEventKind::Release
            ),
            b"\x1b[97;1:3u"
        );
    }

    #[test]
    fn encodes_window_kitty_enter_associated_text_when_enabled() {
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::Enter),
                PhysicalKey::Code(WinitKeyCode::Enter),
                Some("\r"),
                ModifiersState::CONTROL,
                false,
                false,
                24,
                0
            ),
            b"\x1b[13;5;13u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::Enter),
                PhysicalKey::Code(WinitKeyCode::Enter),
                Some("\r"),
                ModifiersState::CONTROL | ModifiersState::SHIFT,
                false,
                false,
                24,
                0
            ),
            b"\x1b[13;6;13u"
        );
        assert_eq!(
            encode_window_key_with_kitty_event(
                &Key::Named(NamedKey::Enter),
                PhysicalKey::Code(WinitKeyCode::Enter),
                Some("\r"),
                ModifiersState::CONTROL,
                false,
                false,
                26,
                0,
                KittyKeyEventKind::Release
            ),
            b"\x1b[13;5:3u"
        );
    }

    #[test]
    fn encodes_window_kitty_alternate_keys_when_enabled() {
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("+".into()),
                PhysicalKey::Code(WinitKeyCode::Equal),
                Some("+"),
                ModifiersState::CONTROL | ModifiersState::SHIFT,
                false,
                false,
                5,
                0
            ),
            b"\x1b[61:43;6u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("A".into()),
                PhysicalKey::Code(WinitKeyCode::KeyA),
                Some("A"),
                ModifiersState::SHIFT,
                false,
                false,
                12,
                0
            ),
            b"\x1b[97:65;2u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character(">".into()),
                PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
                Some(">"),
                ModifiersState::SHIFT,
                false,
                false,
                12,
                0
            ),
            b"\x1b[46:62;2u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("\u{441}".into()),
                PhysicalKey::Code(WinitKeyCode::KeyC),
                Some("\u{441}"),
                ModifiersState::CONTROL,
                false,
                false,
                5,
                0
            ),
            b"\x1b[1089::99;5u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("\u{3bb}".into()),
                PhysicalKey::Code(WinitKeyCode::IntlBackslash),
                Some("\u{3bb}"),
                ModifiersState::CONTROL,
                false,
                false,
                5,
                0
            ),
            b"\x1b[955::92;5u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("\u{308d}".into()),
                PhysicalKey::Code(WinitKeyCode::IntlRo),
                Some("\u{308d}"),
                ModifiersState::CONTROL,
                false,
                false,
                5,
                0
            ),
            b"\x1b[12429::92;5u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("\u{a5}".into()),
                PhysicalKey::Code(WinitKeyCode::IntlYen),
                Some("\u{a5}"),
                ModifiersState::CONTROL,
                false,
                false,
                5,
                0
            ),
            b"\x1b[165::92;5u"
        );
    }

    #[test]
    fn encodes_window_kitty_shifted_ascii_primary_without_alternate_flag() {
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character(">".into()),
                PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
                Some(">"),
                ModifiersState::SHIFT,
                false,
                false,
                8,
                0
            ),
            b"\x1b[46;2u"
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn encodes_window_kitty_canonical_functional_keys_when_enabled() {
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::F1),
                PhysicalKey::Code(WinitKeyCode::F1),
                None,
                ModifiersState::empty(),
                false,
                false,
                1,
                0
            ),
            b"\x1b[P"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::Escape),
                PhysicalKey::Code(WinitKeyCode::Escape),
                None,
                ModifiersState::empty(),
                false,
                false,
                1,
                0
            ),
            b"\x1b[27u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::ArrowUp),
                PhysicalKey::Code(WinitKeyCode::ArrowUp),
                None,
                ModifiersState::empty(),
                true,
                false,
                1,
                0
            ),
            b"\x1b[A"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::F13),
                PhysicalKey::Code(WinitKeyCode::F13),
                None,
                ModifiersState::empty(),
                false,
                false,
                8,
                0
            ),
            b"\x1b[57376u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::F3),
                PhysicalKey::Code(WinitKeyCode::F3),
                None,
                ModifiersState::empty(),
                false,
                false,
                1,
                0
            ),
            b"\x1b[13~"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::CapsLock),
                PhysicalKey::Code(WinitKeyCode::CapsLock),
                None,
                ModifiersState::empty(),
                false,
                false,
                1,
                0
            ),
            b"\x1b[57358u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::ScrollLock),
                PhysicalKey::Code(WinitKeyCode::ScrollLock),
                None,
                ModifiersState::empty(),
                false,
                false,
                1,
                0
            ),
            b"\x1b[57359u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::NumLock),
                PhysicalKey::Code(WinitKeyCode::NumLock),
                None,
                ModifiersState::empty(),
                false,
                false,
                1,
                0
            ),
            b"\x1b[57360u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::PrintScreen),
                PhysicalKey::Code(WinitKeyCode::PrintScreen),
                None,
                ModifiersState::empty(),
                false,
                false,
                1,
                0
            ),
            b"\x1b[57361u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::Pause),
                PhysicalKey::Code(WinitKeyCode::Pause),
                None,
                ModifiersState::empty(),
                false,
                false,
                1,
                0
            ),
            b"\x1b[57362u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::ContextMenu),
                PhysicalKey::Code(WinitKeyCode::ContextMenu),
                None,
                ModifiersState::empty(),
                false,
                false,
                1,
                0
            ),
            b"\x1b[57363u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::MediaPlayPause),
                PhysicalKey::Code(WinitKeyCode::MediaPlayPause),
                None,
                ModifiersState::empty(),
                false,
                false,
                1,
                0
            ),
            b"\x1b[57430u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::MediaStop),
                PhysicalKey::Code(WinitKeyCode::MediaStop),
                None,
                ModifiersState::empty(),
                false,
                false,
                1,
                0
            ),
            b"\x1b[57432u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::MediaTrackNext),
                PhysicalKey::Code(WinitKeyCode::MediaTrackNext),
                None,
                ModifiersState::empty(),
                false,
                false,
                1,
                0
            ),
            b"\x1b[57435u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::AudioVolumeMute),
                PhysicalKey::Code(WinitKeyCode::AudioVolumeMute),
                None,
                ModifiersState::empty(),
                false,
                false,
                1,
                0
            ),
            b"\x1b[57440u"
        );
    }

    #[test]
    fn encodes_window_kitty_keypad_keys_when_enabled() {
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::Enter),
                PhysicalKey::Code(WinitKeyCode::NumpadEnter),
                Some("\r"),
                ModifiersState::empty(),
                false,
                false,
                1,
                0
            ),
            b"\x1b[57414u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("5".into()),
                PhysicalKey::Code(WinitKeyCode::Numpad5),
                Some("5"),
                ModifiersState::empty(),
                false,
                false,
                8,
                0
            ),
            b"\x1b[57404u"
        );
    }

    #[test]
    fn encodes_window_kitty_modifier_keys_when_enabled() {
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::Shift),
                PhysicalKey::Code(WinitKeyCode::ShiftLeft),
                None,
                ModifiersState::SHIFT,
                false,
                false,
                1,
                0
            ),
            b"\x1b[57441;2u"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::Super),
                PhysicalKey::Code(WinitKeyCode::SuperLeft),
                None,
                ModifiersState::SUPER,
                false,
                false,
                1,
                0
            ),
            b"\x1b[57444;9u"
        );
        assert_eq!(
            encode_window_key_with_kitty_event(
                &Key::Named(NamedKey::Control),
                PhysicalKey::Code(WinitKeyCode::ControlRight),
                None,
                ModifiersState::empty(),
                false,
                false,
                2,
                0,
                KittyKeyEventKind::Release
            ),
            b"\x1b[57448;1:3u"
        );
    }

    #[test]
    fn encodes_window_kitty_event_types_when_enabled() {
        assert_eq!(
            encode_window_key_with_kitty_event(
                &Key::Named(NamedKey::ArrowUp),
                PhysicalKey::Code(WinitKeyCode::ArrowUp),
                None,
                ModifiersState::empty(),
                false,
                false,
                2,
                0,
                KittyKeyEventKind::Repeat
            ),
            b"\x1b[1;1:2A"
        );
        assert_eq!(
            encode_window_key_with_kitty_event(
                &Key::Character("i".into()),
                PhysicalKey::Code(WinitKeyCode::KeyI),
                None,
                ModifiersState::CONTROL,
                false,
                false,
                3,
                0,
                KittyKeyEventKind::Release
            ),
            b"\x1b[105;5:3u"
        );
        assert_eq!(
            encode_window_key_with_kitty_event(
                &Key::Named(NamedKey::ArrowUp),
                PhysicalKey::Code(WinitKeyCode::ArrowUp),
                None,
                ModifiersState::SUPER,
                false,
                false,
                2,
                0,
                KittyKeyEventKind::Repeat
            ),
            b"\x1b[1;9:2A"
        );
        assert_eq!(
            encode_window_key_with_kitty_event(
                &Key::Character("a".into()),
                PhysicalKey::Code(WinitKeyCode::KeyA),
                Some("a"),
                ModifiersState::empty(),
                false,
                false,
                10,
                0,
                KittyKeyEventKind::Release
            ),
            b"\x1b[97;1:3u"
        );
        assert_eq!(
            encode_window_key_with_kitty_event(
                &Key::Character("a".into()),
                PhysicalKey::Code(WinitKeyCode::KeyA),
                Some("a"),
                ModifiersState::empty(),
                false,
                false,
                2,
                0,
                KittyKeyEventKind::Release
            ),
            b"\x1b[97;1:3u"
        );
        assert_eq!(
            encode_window_key_with_kitty_event(
                &Key::Character("+".into()),
                PhysicalKey::Code(WinitKeyCode::Equal),
                Some("+"),
                ModifiersState::CONTROL | ModifiersState::SHIFT,
                false,
                false,
                6,
                0,
                KittyKeyEventKind::Release
            ),
            b"\x1b[61:43;6:3u"
        );
        assert!(
            encode_window_key_with_kitty_event(
                &Key::Named(NamedKey::Enter),
                PhysicalKey::Code(WinitKeyCode::Enter),
                Some("\r"),
                ModifiersState::empty(),
                false,
                false,
                3,
                0,
                KittyKeyEventKind::Release
            )
            .is_empty()
        );
    }

    #[test]
    fn encodes_repeated_window_keys_without_kitty_protocol() {
        // macOS sends auto-repeat as a second Pressed event.  The default
        // terminal path must keep emitting the same byte as the initial key
        // press so a long-held Backspace continues deleting input.
        assert_eq!(
            encode_window_key_with_kitty_event(
                &Key::Named(NamedKey::Backspace),
                PhysicalKey::Code(WinitKeyCode::Backspace),
                None,
                ModifiersState::empty(),
                false,
                false,
                0,
                0,
                KittyKeyEventKind::Repeat,
            ),
            b"\x7f"
        );
        assert_eq!(
            encode_window_key_with_kitty_event(
                &Key::Character("a".into()),
                PhysicalKey::Code(WinitKeyCode::KeyA),
                Some("a"),
                ModifiersState::empty(),
                false,
                false,
                0,
                0,
                KittyKeyEventKind::Repeat,
            ),
            b"a"
        );
        assert!(
            encode_window_key_with_kitty_event(
                &Key::Named(NamedKey::Backspace),
                PhysicalKey::Code(WinitKeyCode::Backspace),
                None,
                ModifiersState::empty(),
                false,
                false,
                0,
                0,
                KittyKeyEventKind::Release,
            )
            .is_empty()
        );
    }

    #[test]
    fn encodes_window_kitty_ctrl_shift_tab_as_canonical_tab_when_disambiguated() {
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::Tab),
                PhysicalKey::Code(WinitKeyCode::Tab),
                Some("\t"),
                ModifiersState::CONTROL | ModifiersState::SHIFT,
                false,
                false,
                1,
                0
            ),
            b"\x1b[9;6u"
        );
    }

    #[test]
    fn encodes_window_xterm_modify_other_keys_when_enabled() {
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Named(NamedKey::Enter),
                PhysicalKey::Code(WinitKeyCode::Enter),
                Some("\r"),
                ModifiersState::CONTROL,
                false,
                false,
                0,
                2
            ),
            b"\x1b[27;5;13~"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character("I".into()),
                PhysicalKey::Code(WinitKeyCode::KeyI),
                None,
                ModifiersState::CONTROL | ModifiersState::SHIFT,
                false,
                false,
                0,
                2
            ),
            b"\x1b[27;6;73~"
        );
        assert_eq!(
            encode_window_key_with_kitty(
                &Key::Character(".".into()),
                PhysicalKey::Code(WinitKeyCode::Period),
                Some("."),
                ModifiersState::ALT,
                false,
                false,
                0,
                2
            ),
            b"\x1b[27;3;46~"
        );
    }

    #[test]
    fn quotes_dropped_file_names_using_wezterm_modes() {
        assert_eq!(
            quote_dropped_file_name("hello ($world)", NativeQuoteDroppedFiles::None),
            "hello ($world)"
        );
        assert_eq!(
            quote_dropped_file_name("hello ($world)", NativeQuoteDroppedFiles::SpacesOnly),
            "hello\\ ($world)"
        );
        assert_eq!(
            quote_dropped_file_name("hello ($world)", NativeQuoteDroppedFiles::Posix),
            "\"hello (\\$world)\""
        );
        assert_eq!(
            quote_dropped_file_name("hello ($world)", NativeQuoteDroppedFiles::Windows),
            "\"hello ($world)\""
        );
        assert_eq!(
            quote_dropped_file_name("hello", NativeQuoteDroppedFiles::Windows),
            "hello"
        );
        assert_eq!(
            quote_dropped_file_name("hello", NativeQuoteDroppedFiles::WindowsAlwaysQuoted),
            "\"hello\""
        );
    }

    #[test]
    fn window_app_writes_quoted_dropped_file_to_active_pane() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.set_config_overrides(native_config_snapshot! {
            quote_dropped_files: Some(NativeQuoteDroppedFiles::Posix),
            ..NativeConfigSnapshot::default()
        });

        app.handle_dropped_file_path(std::path::Path::new("hello ($world)"))
            .unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"\"hello (\\$world)\"");
    }

    #[test]
    fn window_app_xtgettcap_terminal_name_follows_term_override() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.set_config_overrides(native_config_snapshot! {
            term: Some("wezterm".to_owned()),
            ..NativeConfigSnapshot::default()
        });

        app.handle_pty_output(b"\x1bP+q544e;6e616d65\x1b\\")
            .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"\x1bP1+r544E=77657A7465726D\x1b\\\x1bP1+r6E616D65=77657A7465726D\x1b\\"
        );
    }

    #[test]
    fn recognizes_window_paste_shortcuts() {
        assert!(
            window_paste_source_for_shortcut(
                &Key::Character("v".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT
            )
            .is_some()
        );
        assert!(
            window_paste_source_for_shortcut(
                &Key::Character("V".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT
            )
            .is_some()
        );
        assert!(
            window_paste_source_for_shortcut(&Key::Character("v".into()), ModifiersState::SUPER)
                .is_some()
        );
        assert!(
            window_paste_source_for_shortcut(&Key::Named(NamedKey::Insert), ModifiersState::SHIFT)
                .is_some()
        );
        assert!(
            window_paste_source_for_shortcut(&Key::Named(NamedKey::Paste), ModifiersState::empty())
                .is_some()
        );
        assert!(
            window_paste_source_for_shortcut(&Key::Character("v".into()), ModifiersState::CONTROL)
                .is_none()
        );
        assert!(
            window_paste_source_for_shortcut(&Key::Character("v".into()), ModifiersState::empty())
                .is_none()
        );
    }

    #[test]
    fn maps_window_paste_shortcuts_to_wezterm_sources() {
        assert_eq!(
            window_paste_source_for_shortcut(
                &Key::Character("v".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT
            ),
            Some(WindowPasteSource::Clipboard)
        );
        assert_eq!(
            window_paste_source_for_shortcut(
                &Key::Character("V".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT
            ),
            Some(WindowPasteSource::Clipboard)
        );
        assert_eq!(
            window_paste_source_for_shortcut(&Key::Character("v".into()), ModifiersState::SUPER),
            Some(WindowPasteSource::Clipboard)
        );
        assert_eq!(
            window_paste_source_for_shortcut(&Key::Named(NamedKey::Insert), ModifiersState::SHIFT),
            Some(WindowPasteSource::PrimarySelection)
        );
        assert_eq!(
            window_paste_source_for_shortcut(&Key::Named(NamedKey::Paste), ModifiersState::empty()),
            Some(WindowPasteSource::Clipboard)
        );
        assert_eq!(
            window_paste_source_for_shortcut(&Key::Character("v".into()), ModifiersState::CONTROL),
            None
        );
        assert_eq!(
            window_paste_source_for_shortcut(&Key::Character("v".into()), ModifiersState::empty()),
            None
        );
    }

    #[test]
    fn window_pane_launch_command_uses_plain_current_working_dir() {
        let launch = PaneLaunch::local("powershell")
            .with_args(["-NoProfile"])
            .with_cwd("/tmp/project");

        let command = pty_command_from_pane_launch(&launch);

        assert_eq!(command.program(), "powershell");
        assert_eq!(command.args(), ["-NoProfile"]);
        assert_eq!(command.cwd(), Some(std::path::Path::new("/tmp/project")));
    }

    #[test]
    fn window_pane_launch_command_decodes_file_uri_current_working_dir() {
        let launch = PaneLaunch::local("powershell").with_cwd("file://host/home/ops%20team");

        let command = pty_command_from_pane_launch(&launch);

        assert_eq!(command.cwd(), Some(std::path::Path::new("/home/ops team")));
    }

    #[test]
    fn window_pane_launch_command_honors_configured_term() {
        let launch = PaneLaunch::local("powershell").with_args(["-NoProfile"]);

        let command = pty_command_from_pane_launch_with_term(&launch, "wezterm");

        assert_eq!(command.program(), "powershell");
        assert_eq!(command.args(), ["-NoProfile"]);
        assert_eq!(command.env_value("TERM"), Some("wezterm"));
        assert_eq!(command.env_value("COLORTERM"), Some("truecolor"));
    }

    #[test]
    fn window_pane_launch_command_honors_configured_environment_variables() {
        let launch = PaneLaunch::local("powershell").with_args(["-NoProfile"]);
        let environment = BTreeMap::from([
            ("WEZTERM_CONFIG_DIR".to_owned(), "/tmp/wezterm".to_owned()),
            ("RSSH_PROFILE".to_owned(), "ops".to_owned()),
        ]);

        let command =
            pty_command_from_pane_launch_with_environment(&launch, "wezterm", &environment, None);

        assert_eq!(command.program(), "powershell");
        assert_eq!(command.args(), ["-NoProfile"]);
        assert_eq!(command.env_value("TERM"), Some("wezterm"));
        assert_eq!(command.env_value("COLORTERM"), Some("truecolor"));
        assert_eq!(
            command.env_value("WEZTERM_CONFIG_DIR"),
            Some("/tmp/wezterm")
        );
        assert_eq!(command.env_value("RSSH_PROFILE"), Some("ops"));
    }

    #[test]
    fn window_pane_launch_command_honors_launch_environment_variables() {
        let launch = PaneLaunch::local("powershell")
            .with_args(["-NoProfile"])
            .with_environment(BTreeMap::from([(
                "SPAWN_MODE".to_owned(),
                "native".to_owned(),
            )]));

        let command = pty_command_from_pane_launch_with_environment(
            &launch,
            "wezterm",
            &BTreeMap::new(),
            None,
        );

        assert_eq!(command.env_value("SPAWN_MODE"), Some("native"));
    }

    #[test]
    fn window_pane_launch_command_sets_term_session_id_environment() {
        let launch = PaneLaunch::local("powershell").with_environment(BTreeMap::from([(
            "TERM_SESSION_ID".to_owned(),
            "stale".to_owned(),
        )]));

        let command = pty_command_from_pane_launch_with_term_session_id(
            &launch,
            "wezterm",
            &BTreeMap::from([("TERM_SESSION_ID".to_owned(), "configured".to_owned())]),
            None,
            "w4t2p9",
        );

        assert_eq!(command.env_value("TERM_SESSION_ID"), Some("w4t2p9"));
    }

    #[test]
    fn window_pane_launch_command_uses_default_cwd_when_launch_has_none() {
        let launch = PaneLaunch::local("powershell").with_args(["-NoProfile"]);

        let command = pty_command_from_pane_launch_with_default_cwd(
            &launch,
            "xterm-256color",
            &BTreeMap::new(),
            Some("/tmp/default"),
        );

        assert_eq!(command.program(), "powershell");
        assert_eq!(command.args(), ["-NoProfile"]);
        assert_eq!(command.cwd(), Some(std::path::Path::new("/tmp/default")));
    }

    #[test]
    fn window_pane_launch_command_prefers_launch_cwd_over_default_cwd() {
        let launch = PaneLaunch::local("powershell").with_cwd("/tmp/launch");

        let command = pty_command_from_pane_launch_with_default_cwd(
            &launch,
            "xterm-256color",
            &BTreeMap::new(),
            Some("/tmp/default"),
        );

        assert_eq!(command.cwd(), Some(std::path::Path::new("/tmp/launch")));
    }

    #[test]
    fn window_pane_launch_command_uses_home_directory_when_no_cwd_is_resolved() {
        let home = test_home_dir().expect("test host should expose a home directory");
        let launch = PaneLaunch::local("powershell");

        let command = pty_command_from_pane_launch_with_default_cwd(
            &launch,
            "xterm-256color",
            &BTreeMap::new(),
            None,
        );

        assert_eq!(command.cwd(), Some(home.as_path()));
    }

    #[test]
    fn window_app_new_tab_uses_default_prog_when_launch_has_no_prog() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            default_prog: Some(vec!["top".to_owned(), "-H".to_owned()]),
            ..NativeConfigSnapshot::default()
        });

        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-H"]);
    }

    #[test]
    fn window_app_new_tab_prefers_explicit_launch_over_default_prog() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            default_prog: Some(vec!["top".to_owned(), "-H".to_owned()]),
            ..NativeConfigSnapshot::default()
        });

        app.dispatch_app_action(AppAction::NewTab {
            launch: Some(PaneLaunch::local("pwsh").with_args(["-NoLogo"])),
        })
        .unwrap();

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "pwsh");
        assert_eq!(launch.args(), ["-NoLogo"]);
    }

    #[test]
    fn window_app_new_tab_inherits_ssh_domain_even_when_default_prog_is_configured() {
        let mut app = NativeWindowApp::new(None);
        app.set_initial_pane_launch(PaneLaunch::ssh(
            SshPaneLaunch::new(
                "ops@example.test:2222",
                SshAuthDescription::PasswordPrompt,
                SshKnownHostsPolicy::Prompt,
            )
            .with_remote_command(["uptime"]),
        ));
        app.set_config_overrides(native_config_snapshot! {
            default_prog: Some(vec!["local-shell".to_owned(), "--login".to_owned()]),
            ..NativeConfigSnapshot::default()
        });
        assert!(matches!(
            app.app_shell.active_pane().launch().domain(),
            rssh_core::app_shell::PaneLaunchDomain::Ssh(_)
        ));
        assert!(matches!(
            app.implicit_child_launch(app.active_pane_id())
                .expect("SSH child launch")
                .domain(),
            rssh_core::app_shell::PaneLaunchDomain::Ssh(_)
        ));

        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let launch = app.app_shell.active_pane().launch();
        let rssh_core::app_shell::PaneLaunchDomain::Ssh(ssh) = launch.domain() else {
            panic!("implicit child launch must remain in the SSH domain");
        };
        assert_eq!(ssh.target(), "ops@example.test:2222");
        assert!(ssh.remote_command().is_empty());
    }

    #[test]
    fn window_app_split_inherits_ssh_domain_even_when_default_prog_is_configured() {
        let mut app = NativeWindowApp::new(None);
        app.set_initial_pane_launch(PaneLaunch::ssh(SshPaneLaunch::new(
            "ops@example.test:2222",
            SshAuthDescription::PasswordPrompt,
            SshKnownHostsPolicy::Prompt,
        )));
        app.set_config_overrides(native_config_snapshot! {
            default_prog: Some(vec!["local-shell".to_owned(), "--login".to_owned()]),
            ..NativeConfigSnapshot::default()
        });
        let source = app.active_pane_id();
        assert!(matches!(
            app.implicit_child_launch(source)
                .expect("SSH child launch")
                .domain(),
            rssh_core::app_shell::PaneLaunchDomain::Ssh(_)
        ));

        app.dispatch_app_action(AppAction::SplitPane {
            pane: source,
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        assert!(matches!(
            app.app_shell.active_pane().launch().domain(),
            rssh_core::app_shell::PaneLaunchDomain::Ssh(_)
        ));
    }

    #[test]
    fn window_app_split_uses_default_prog_and_inherited_cwd_when_launch_has_no_prog() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rssh_pty::PtyCommand::new("pwsh").with_cwd("/tmp/project"),
        );
        app.set_config_overrides(native_config_snapshot! {
            default_prog: Some(vec!["nu".to_owned(), "--login".to_owned()]),
            ..NativeConfigSnapshot::default()
        });

        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.app_shell.active_pane_id(),
            direction: rssh_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);
        assert_eq!(launch.cwd(), Some("/tmp/project"));
    }

    #[test]
    fn recognizes_window_copy_shortcuts() {
        assert!(
            window_copy_destination_for_shortcut(
                &Key::Character("c".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT
            )
            .is_some()
        );
        assert!(
            window_copy_destination_for_shortcut(
                &Key::Character("C".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT
            )
            .is_some()
        );
        assert!(
            window_copy_destination_for_shortcut(
                &Key::Character("c".into()),
                ModifiersState::SUPER
            )
            .is_some()
        );
        assert!(
            window_copy_destination_for_shortcut(
                &Key::Named(NamedKey::Insert),
                ModifiersState::CONTROL
            )
            .is_some()
        );
        assert!(
            window_copy_destination_for_shortcut(
                &Key::Named(NamedKey::Copy),
                ModifiersState::empty()
            )
            .is_some()
        );
        assert!(
            window_copy_destination_for_shortcut(
                &Key::Character("c".into()),
                ModifiersState::CONTROL
            )
            .is_none()
        );
    }

    #[test]
    fn maps_window_copy_shortcuts_to_wezterm_destinations() {
        assert_eq!(
            window_copy_destination_for_shortcut(
                &Key::Character("c".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT
            ),
            Some(WindowCopyDestination::Clipboard)
        );
        assert_eq!(
            window_copy_destination_for_shortcut(
                &Key::Character("C".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT
            ),
            Some(WindowCopyDestination::Clipboard)
        );
        assert_eq!(
            window_copy_destination_for_shortcut(
                &Key::Character("c".into()),
                ModifiersState::SUPER
            ),
            Some(WindowCopyDestination::Clipboard)
        );
        assert_eq!(
            window_copy_destination_for_shortcut(
                &Key::Named(NamedKey::Insert),
                ModifiersState::CONTROL
            ),
            Some(WindowCopyDestination::PrimarySelection)
        );
        assert_eq!(
            window_copy_destination_for_shortcut(
                &Key::Named(NamedKey::Copy),
                ModifiersState::empty()
            ),
            Some(WindowCopyDestination::Clipboard)
        );
        assert_eq!(
            window_copy_destination_for_shortcut(
                &Key::Character("c".into()),
                ModifiersState::CONTROL
            ),
            None
        );
    }

    #[test]
    fn encodes_window_named_keys_for_pty() {
        assert_eq!(
            encode_window_key(
                &Key::Named(NamedKey::ArrowUp),
                PhysicalKey::Code(WinitKeyCode::ArrowUp),
                None,
                ModifiersState::empty(),
                false,
                false
            ),
            b"\x1b[A"
        );
        assert_eq!(
            encode_window_key(
                &Key::Named(NamedKey::Enter),
                PhysicalKey::Code(WinitKeyCode::Enter),
                Some("\r"),
                ModifiersState::empty(),
                false,
                false
            ),
            b"\r"
        );
    }

    #[test]
    fn encodes_window_menu_key_as_legacy_functional_sequence() {
        assert_eq!(
            encode_window_key(
                &Key::Named(NamedKey::ContextMenu),
                PhysicalKey::Code(WinitKeyCode::ContextMenu),
                None,
                ModifiersState::empty(),
                false,
                false
            ),
            b"\x1b[29~"
        );
    }

    #[test]
    fn encodes_window_modified_navigation_and_function_keys() {
        assert_eq!(
            encode_window_key(
                &Key::Named(NamedKey::ArrowLeft),
                PhysicalKey::Code(WinitKeyCode::ArrowLeft),
                None,
                ModifiersState::CONTROL,
                false,
                false
            ),
            b"\x1b[1;5D"
        );
        assert_eq!(
            encode_window_key(
                &Key::Named(NamedKey::Delete),
                PhysicalKey::Code(WinitKeyCode::Delete),
                None,
                ModifiersState::SHIFT | ModifiersState::ALT,
                false,
                false
            ),
            b"\x1b[3;4~"
        );
        assert_eq!(
            encode_window_key(
                &Key::Named(NamedKey::F5),
                PhysicalKey::Code(WinitKeyCode::F5),
                None,
                ModifiersState::SHIFT,
                false,
                false
            ),
            b"\x1b[15;2~"
        );
    }

    #[test]
    fn encodes_window_application_cursor_keys_when_enabled() {
        assert_eq!(
            encode_window_key(
                &Key::Named(NamedKey::ArrowUp),
                PhysicalKey::Code(WinitKeyCode::ArrowUp),
                None,
                ModifiersState::empty(),
                true,
                false
            ),
            b"\x1bOA"
        );
        assert_eq!(
            encode_window_key(
                &Key::Named(NamedKey::ArrowUp),
                PhysicalKey::Code(WinitKeyCode::ArrowUp),
                None,
                ModifiersState::CONTROL,
                true,
                false
            ),
            b"\x1b[1;5A"
        );
    }

    #[test]
    fn encodes_window_focus_events_when_enabled() {
        assert_eq!(
            encode_window_focus_event(true, true),
            Some(b"\x1b[I".to_vec())
        );
        assert_eq!(
            encode_window_focus_event(false, true),
            Some(b"\x1b[O".to_vec())
        );
        assert_eq!(encode_window_focus_event(true, false), None);
    }

    #[test]
    fn window_app_starts_unfocused_until_the_os_reports_focus() {
        let app = NativeWindowApp::new(None);
        assert!(!app.window_focused);
        assert!(!app.mouse_click_may_focus_window);
    }

    #[test]
    fn window_app_focus_changes_are_idempotent() {
        let changes = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&changes);
        let mut app = NativeWindowApp::new(None);
        app.focus_change_handler = Box::new(move |change| {
            recorded.lock().unwrap().push(*change);
            true
        });

        assert!(app.handle_focus_changed(true).unwrap());
        assert!(!app.handle_focus_changed(true).unwrap());
        assert!(app.handle_focus_changed(false).unwrap());
        assert!(!app.handle_focus_changed(false).unwrap());

        assert_eq!(
            changes
                .lock()
                .unwrap()
                .iter()
                .map(|change| change.focused)
                .collect::<Vec<_>>(),
            [true, false]
        );
    }

    #[test]
    fn window_app_focus_reporting_suppresses_duplicate_sequences() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.feed_pty_output(b"\x1b[?1004h");

        assert!(app.handle_focus_changed(true).unwrap());
        assert!(!app.handle_focus_changed(true).unwrap());
        assert!(app.handle_focus_changed(false).unwrap());
        assert!(!app.handle_focus_changed(false).unwrap());

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b[I\x1b[O");
    }

    #[test]
    fn window_app_dispatches_focus_changed_for_active_pane() {
        let changes = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&changes);
        let mut app = NativeWindowApp::new(None);
        app.focus_change_handler = Box::new(move |change| {
            recorded.lock().unwrap().push(*change);
            true
        });
        let active_pane = app.app_shell.active_pane_id();

        assert!(app.handle_focus_changed(true).unwrap());
        assert!(app.handle_focus_changed(false).unwrap());

        assert_eq!(
            changes.lock().unwrap().as_slice(),
            [
                NativeWindowFocusChange {
                    window_id: rssh_core::WindowId::new(1),
                    pane: active_pane,
                    focused: true,
                },
                NativeWindowFocusChange {
                    window_id: rssh_core::WindowId::new(1),
                    pane: active_pane,
                    focused: false,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_focus_changed_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('window-focus-changed', function(window, pane)
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
        .expect("expected static WezTerm focus-changed event status setter");
        app.set_config_overrides(overrides);

        assert!(app.handle_focus_changed(true).unwrap());
        assert_eq!(app.right_status, "FOCUSED");

        assert!(app.handle_focus_changed(false).unwrap());
        assert_eq!(app.right_status, "BLURRED");
    }
