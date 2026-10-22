macro_rules! quick_select_action_cases {
    () => {
[
            (
                "quickselect pattern ticket-[0-9]+",
                WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselect alphabet 12",
                WindowQuickSelectOptions {
                    alphabet: Some("12".to_owned()),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselect action copy to primary selection skip action on paste",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::CopyTo(
                        WindowCopyDestination::PrimarySelection,
                    )),
                    skip_action_on_paste: true,
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselect action copy to=primary selection",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::CopyTo(
                        WindowCopyDestination::PrimarySelection,
                    )),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs action=\"copy to destination=primary selection\"",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::CopyTo(
                        WindowCopyDestination::PrimarySelection,
                    )),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "QuickSelect Action Open URI Skip Action On Paste",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::OpenUri),
                    skip_action_on_paste: true,
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselect action copy to primary selection skip_action_on_paste",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::CopyTo(
                        WindowCopyDestination::PrimarySelection,
                    )),
                    skip_action_on_paste: true,
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs action copy to primary selection skip-action-on-paste",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::CopyTo(
                        WindowCopyDestination::PrimarySelection,
                    )),
                    skip_action_on_paste: true,
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs action copy to primary selection skip-action-on-paste=false",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::CopyTo(
                        WindowCopyDestination::PrimarySelection,
                    )),
                    skip_action_on_paste: false,
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselect scope lines 2",
                WindowQuickSelectOptions {
                    scope_lines: Some(2),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselect scope_lines 2",
                WindowQuickSelectOptions {
                    scope_lines: Some(2),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs scope-lines 2",
                WindowQuickSelectOptions {
                    scope_lines: Some(2),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs scope_lines=2",
                WindowQuickSelectOptions {
                    scope_lines: Some(2),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs alphabet 12",
                WindowQuickSelectOptions {
                    alphabet: Some("12".to_owned()),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs pattern=ticket-[0-9]+",
                WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs patterns=ticket-[0-9]+",
                WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs patterns=ticket-[0-9]+;bug-[0-9]+",
                WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned(), "bug-[0-9]+".to_owned()]),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs pattern ticket-[0-9]+ action open uri",
                WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    action: Some(WindowQuickSelectAction::OpenUri),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs pattern ticket-[0-9]+ alphabet 12",
                WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    alphabet: Some("12".to_owned()),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs pattern ticket-[0-9]+ scope lines=2",
                WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    scope_lines: Some(2),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs pattern ticket-[0-9]+ label open ticket",
                WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    label: Some("open ticket".to_owned()),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs alphabet=12",
                WindowQuickSelectOptions {
                    alphabet: Some("12".to_owned()),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs=alphabet=12",
                WindowQuickSelectOptions {
                    alphabet: Some("12".to_owned()),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs label=open-ticket",
                WindowQuickSelectOptions {
                    label: Some("open-ticket".to_owned()),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs label open ticket action open uri",
                WindowQuickSelectOptions {
                    label: Some("open ticket".to_owned()),
                    action: Some(WindowQuickSelectAction::OpenUri),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs label open ticket alphabet 12",
                WindowQuickSelectOptions {
                    label: Some("open ticket".to_owned()),
                    alphabet: Some("12".to_owned()),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs label open ticket skip action on paste",
                WindowQuickSelectOptions {
                    label: Some("open ticket".to_owned()),
                    skip_action_on_paste: true,
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs action=open-uri",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::OpenUri),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs action=Open-URI",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::OpenUri),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs action=open-uri skip_action_on_paste=true",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::OpenUri),
                    skip_action_on_paste: true,
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs action=open-uri skip-action-on-paste",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::OpenUri),
                    skip_action_on_paste: true,
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs action=open-uri skip action on paste=true",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::OpenUri),
                    skip_action_on_paste: true,
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs action=open-uri skip action on Paste=True",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::OpenUri),
                    skip_action_on_paste: true,
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs action=open-uri skip action on paste false",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::OpenUri),
                    skip_action_on_paste: false,
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs action=open-uri scope_lines=2",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::OpenUri),
                    scope_lines: Some(2),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs action open uri scope lines=2",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::OpenUri),
                    scope_lines: Some(2),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs action open uri alphabet 12",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::OpenUri),
                    alphabet: Some("12".to_owned()),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs action open uri pattern ticket-[0-9]+",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::OpenUri),
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quickselectargs action open uri scope lines=2 skip action on paste",
                WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::OpenUri),
                    skip_action_on_paste: true,
                    scope_lines: Some(2),
                    ..WindowQuickSelectOptions::default()
                },
            ),
            (
                "quick select scope_lines 2",
                WindowQuickSelectOptions {
                    scope_lines: Some(2),
                    ..WindowQuickSelectOptions::default()
                },
            ),
        ]
    };
}

macro_rules! sample_native_config_overrides {
    () => {{
        let mut snapshot = native_config_snapshot! {
            effective: Arc::new(rterm_config::EffectiveConfig::default()),
            dpi: Some(144),
            dpi_by_screen: Some(BTreeMap::from([
                ("Built-in Retina Display".to_owned(), 144),
                ("HDMI".to_owned(), 96),
            ])),
            tab_max_width: Some(32),
            status_update_interval_ms: Some(250),
            max_fps: Some(144),
            animation_fps: Some(24),
            front_end: Some(NativeRenderFrontEnd::WebGpu),
            webgpu_power_preference: Some(NativeWebGpuPowerPreference::HighPerformance),
            webgpu_force_fallback_adapter: Some(true),
            webgpu_preferred_adapter: Some(NativeWebGpuPreferredAdapter {
                backend: Some("Vulkan".to_owned()),
                device: Some(29_730),
                device_type: Some("DiscreteGpu".to_owned()),
                driver: Some("radv".to_owned()),
                driver_info: Some("Mesa 22.3.4".to_owned()),
                name: Some("AMD Radeon Pro W6400 (RADV NAVI24)".to_owned()),
                vendor: Some(4_098),
            }),
            prefer_egl: Some(false),
            enable_wayland: Some(false),
            enable_zwlr_output_manager: Some(true),
            use_box_model_render: Some(true),
            experimental_pixel_positioning: Some(true),
            shape_cache_size: Some(2_048),
            line_state_cache_size: Some(512),
            line_quad_cache_size: Some(768),
            line_to_ele_shape_cache_size: Some(1_536),
            glyph_cache_image_cache_size: Some(128),
            cursor_blink_rate_ms: Some(375),
            cursor_blink_ease_in: Some(NativeEasingFunction::EaseIn),
            cursor_blink_ease_out: Some(NativeEasingFunction::EaseOut),
            text_blink_rate_ms: Some(525),
            text_blink_rate_rapid_ms: Some(175),
            text_blink_ease_in: Some(NativeEasingFunction::EaseIn),
            text_blink_ease_out: Some(NativeEasingFunction::EaseOut),
            text_blink_rapid_ease_in: Some(NativeEasingFunction::EaseInOut),
            text_blink_rapid_ease_out: Some(NativeEasingFunction::Constant),
            font: Some("JetBrains Mono".to_owned()),
            font_fallbacks: Some(vec!["Noto Color Emoji".to_owned()]),
            font_attributes: Some(NativeFontAttributes {
                weight: Some("Bold".to_owned()),
                stretch: Some("Expanded".to_owned()),
                style: Some("Italic".to_owned()),
                harfbuzz_features: Vec::new(),
                assume_emoji_presentation: None,
                freetype_load_target: None,
                freetype_render_target: None,
                freetype_load_flags: None,
            }),
            font_rules: Some(vec![NativeFontRule {
                italic: Some(true),
                intensity: Some(NativeFormatIntensity::Bold),
                font: Some("Victor Mono".to_owned()),
                font_fallbacks: Vec::new(),
                ..NativeFontRule::default()
            }]),
            font_size: Some(NativeFontSize::from_millipoints(13_500)),
            cell_width: Some(NativeCellWidth::from_per_mille(1_250)),
            cell_widths: Some(vec![NativeCellWidthOverride::new(0xe000, 0xf8ff, 2)]),
            line_height: Some(NativeLineHeight::from_per_mille(1_250)),
            font_antialias: Some(NativeFontAntialias::Subpixel),
            font_hinting: Some(NativeFontHinting::VerticalSubpixel),
            font_rasterizer: Some(NativeFontRasterizer::FreeType),
            font_colr_rasterizer: Some(NativeFontRasterizer::FreeType),
            font_shaper: Some(NativeFontShaper::Harfbuzz),
            harfbuzz_features: Some(vec!["kern".to_owned(), "liga=0".to_owned()]),
            font_dirs: Some(vec!["fonts".to_owned(), "vendor/fonts".to_owned()]),
            font_locator: Some(NativeFontLocator::ConfigDirsOnly),
            use_cap_height_to_scale_fallback_fonts: Some(true),
            ignore_svg_fonts: Some(true),
            sort_fallback_fonts_by_coverage: Some(true),
            search_font_dirs_for_fallback: Some(true),
            custom_block_glyphs: Some(false),
            anti_alias_custom_block_glyphs: Some(false),
            allow_square_glyphs_to_overflow_width: Some(NativeSquareGlyphOverflow::Always),
            freetype_load_target: Some(NativeFreetypeTarget::Mono),
            freetype_render_target: Some(NativeFreetypeTarget::HorizontalLcd),
            freetype_load_flags: Some(
                NativeFreetypeLoadFlags::NO_HINTING.union(NativeFreetypeLoadFlags::MONOCHROME),
            ),
            freetype_interpreter_version: Some(38),
            freetype_pcf_long_family_names: Some(true),
            display_pixel_geometry: Some(NativeDisplayPixelGeometry::Bgr),
            foreground_text_hsb: Some(NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(0.8),
                brightness: NativeHsbMultiplier::from_f32(0.7),
            }),
            bold_brightens_ansi_colors: Some(NativeBoldBrightensAnsiColors::BrightOnly),
            text_background_opacity: Some(NativeTextBackgroundOpacity::from_f32(0.4)),
            window_background_opacity: Some(NativeTextBackgroundOpacity::from_f32(0.5)),
            background: Some(vec![super::NativeWindowBackgroundVisualLayer::Color(
                Color::Rgb(42, 43, 44),
            )]),
            window_background_image: Some("wallpaper.png".to_owned()),
            window_background_image_hsb: Some(NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(0.9),
                brightness: NativeHsbMultiplier::from_f32(0.6),
            }),
            window_background_gradient: Some(NativeWindowBackgroundGradient {
                orientation: NativeWindowBackgroundGradientOrientation::Vertical,
                interpolation: NativeWindowBackgroundGradientInterpolation::Linear,
                blend: NativeWindowBackgroundGradientBlend::Rgb,
                noise: None,
                segment: None,
                preset: None,
                opacity_alpha: u8::MAX,
                blend_with_background_color: false,
                hsb: super::native_identity_hsb(),
                colors: vec![Color::Rgb(1, 2, 3), Color::Rgb(17, 18, 19)],
            }),
            window_background_images: None,
            window_background_layers: None,
            kde_window_background_blur: Some(true),
            macos_window_background_blur: Some(20),
            win32_system_backdrop: Some(NativeWin32SystemBackdrop::Mica),
            win32_acrylic_accent_color: Some(Color::Rgb(17, 34, 51)),
            window_decorations: Some(NativeWindowDecorations {
                title: false,
                resize: true,
                integrated_buttons: true,
                macos_force_disable_shadow: true,
                macos_force_enable_shadow: false,
                macos_force_square_corners: false,
                macos_use_background_color_as_titlebar_color: true,
            }),
            window_frame_appearance: Some(sample_window_frame_appearance()),
            integrated_title_buttons: Some(vec![
                NativeIntegratedTitleButton::Close,
                NativeIntegratedTitleButton::Hide,
            ]),
            integrated_title_button_alignment: Some(NativeIntegratedTitleButtonAlignment::Left),
            integrated_title_button_color: Some(NativeIntegratedTitleButtonColor::Color(
                Color::Rgb(1, 2, 3),
            )),
            integrated_title_button_style: Some(NativeIntegratedTitleButtonStyle::Gnome),
            default_cursor_style: Some(NativeCursorStyle::BlinkingUnderline),
            cursor_thickness: Some(NativeCursorThickness::Pixels(3)),
            underline_thickness: Some(NativeUnderlineThickness::Pixels(2)),
            underline_position: Some(NativeUnderlinePosition::Pixels(-2)),
            strikethrough_position: Some(NativeStrikethroughPosition::CellFractionPerMille(500)),
            force_reverse_video_cursor: Some(true),
            reverse_video_cursor_min_contrast: Some(NativeContrastRatio::from_centi(325)),
            text_min_contrast_ratio: Some(NativeTextMinContrastRatio::from_centi(450)),
            window_padding: Some(NativeWindowPadding {
                left: NativeWindowPaddingDimension::Pixels(1),
                right: NativeWindowPaddingDimension::Pixels(2),
                top: NativeWindowPaddingDimension::Pixels(3),
                bottom: NativeWindowPaddingDimension::Pixels(4),
            }),
            window_content_alignment: Some(NativeWindowContentAlignment {
                horizontal: NativeHorizontalContentAlignment::Center,
                vertical: NativeVerticalContentAlignment::Bottom,
            }),
            initial_cols: Some(100),
            initial_rows: Some(30),
            inactive_pane_hsb: Some(NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(0.7),
                brightness: NativeHsbMultiplier::from_f32(0.6),
            }),
            command_palette_rows: Some(12),
            command_palette_font: None,
            command_palette_font_size: Some(NativeFontSize::from_millipoints(15_500)),
            command_palette_bg_color: Some(Color::Rgb(15, 16, 17)),
            command_palette_fg_color: Some(Color::Rgb(18, 19, 20)),
            char_select_font: None,
            char_select_font_size: Some(NativeFontSize::from_millipoints(16_250)),
            char_select_bg_color: Some(Color::Rgb(21, 22, 23)),
            char_select_fg_color: Some(Color::Rgb(24, 25, 26)),
            pane_select_font: None,
            pane_select_font_size: Some(NativeFontSize::from_millipoints(36_500)),
            pane_select_bg_color: Some(Color::Rgb(27, 28, 29)),
            pane_select_fg_color: Some(Color::Rgb(30, 31, 32)),
            launcher_alphabet: Some("12".to_owned()),
            quick_select_alphabet: Some("xy".to_owned()),
            quick_select_patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
            disable_default_quick_select_patterns: Some(true),
            quick_select_remove_styling: Some(true),
            hyperlink_rules: Some(vec![NativeHyperlinkRule {
                regex: r"\bT(\d+)\b".to_owned(),
                format: "https://tickets.example/$1".to_owned(),
                highlight: 1,
            }]),
            copy_mode_active_highlight_bg: Some(NativeColorSpec::Color(Color::Rgb(21, 22, 23))),
            copy_mode_active_highlight_fg: Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Black)),
            copy_mode_inactive_highlight_bg: Some(NativeColorSpec::Color(Color::Rgb(24, 25, 26))),
            copy_mode_inactive_highlight_fg: Some(NativeColorSpec::AnsiColor(
                NativeAnsiColor::White,
            )),
            quick_select_label_bg: Some(NativeColorSpec::Color(Color::Rgb(27, 28, 29))),
            quick_select_label_fg: Some(NativeColorSpec::Color(Color::Rgb(30, 31, 32))),
            quick_select_match_bg: Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Navy)),
            quick_select_match_fg: Some(NativeColorSpec::Color(Color::Rgb(33, 34, 35))),
            input_selector_label_bg: Some(NativeColorSpec::Color(Color::Rgb(34, 35, 36))),
            input_selector_label_fg: Some(NativeColorSpec::Color(Color::Rgb(37, 38, 39))),
            launcher_label_bg: Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Black)),
            launcher_label_fg: Some(NativeColorSpec::Color(Color::Rgb(40, 41, 42))),
            selection_word_boundary: Some(" :".to_owned()),
            term: Some("wezterm".to_owned()),
            enq_answerback: Some("rterm".to_owned()),
            audible_bell: Some(NativeAudibleBell::Disabled),
            visual_bell: Some(NativeVisualBell {
                fade_in_duration_ms: 10,
                fade_out_duration_ms: 20,
                fade_in_function: NativeEasingFunction::EaseIn,
                fade_out_function: NativeEasingFunction::EaseOut,
                target: NativeVisualBellTarget::BackgroundColor,
            }),
            colors: Some(Box::new(sample_palette())),
            color_scheme: Some("Project Scheme".to_owned()),
            color_scheme_dirs: Some(vec!["colors".to_owned(), "more-colors".to_owned()]),
            color_schemes: Some(sample_color_schemes()),
            foreground_color: Some(Color::Rgb(7, 8, 9)),
            background_color: Some(Color::Rgb(4, 5, 6)),
            ansi_palette: Some(sample_ansi_palette()),
            indexed_palette: Some(sample_indexed_palette()),
            selection_fg_color: Some(Some(Color::Rgb(61, 62, 63))),
            selection_bg_color: Some(Color::Rgb(71, 72, 73)),
            cursor_bg_color: Some(Color::Rgb(10, 11, 12)),
            cursor_border_color: Some(Color::Rgb(16, 17, 18)),
            cursor_fg_color: Some(Color::Rgb(13, 14, 15)),
            compose_cursor_color: Some(Color::Rgb(22, 23, 24)),
            split_color: Some(Color::Rgb(19, 20, 21)),
            scrollbar_thumb_color: Some(Color::Rgb(22, 23, 24)),
            tab_bar_background_color: Some(Color::Rgb(25, 26, 27)),
            tab_bar_inactive_tab_edge_color: Some(Color::Rgb(27, 28, 29)),
            tab_bar_active_tab_colors: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(28, 29, 30)),
                bg_color: Some(Color::Rgb(31, 32, 33)),
                intensity: Some(NativeFormatIntensity::Bold),
                ..Default::default()
            },
            tab_bar_inactive_tab_colors: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(34, 35, 36)),
                bg_color: Some(Color::Rgb(37, 38, 39)),
                ..Default::default()
            },
            tab_bar_inactive_tab_hover_colors: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(46, 47, 48)),
                bg_color: Some(Color::Rgb(49, 50, 51)),
                ..Default::default()
            },
            tab_bar_new_tab_colors: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(40, 41, 42)),
                bg_color: Some(Color::Rgb(43, 44, 45)),
                ..Default::default()
            },
            tab_bar_new_tab_hover_colors: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(52, 53, 54)),
                bg_color: Some(Color::Rgb(55, 56, 57)),
                ..Default::default()
            },
            tab_bar_style: NativeTabBarStyle::default(),
            visual_bell_color: Some(Color::Rgb(1, 2, 3)),
            notification_handling: Some(NativeNotificationHandling::SuppressFromFocusedWindow),
            default_prog: Some(vec!["top".to_owned(), "-H".to_owned()]),
            default_gui_startup_args: Some(vec!["connect".to_owned(), "prod".to_owned()]),
            default_domain: Some("local".to_owned()),
            default_workspace: Some("ops".to_owned()),
            prefer_to_spawn_tabs: Some(true),
            automatically_reload_config: Some(false),
            check_for_updates: Some(false),
            check_for_updates_interval_seconds: Some(43_200),
            show_update_window: Some(true),
            native_macos_fullscreen_mode: Some(true),
            macos_fullscreen_extend_behind_notch: Some(true),
            use_resize_increments: Some(true),
            debug_key_events: Some(true),
            log_unknown_escape_sequences: Some(true),
            warn_about_missing_glyphs: Some(false),
            default_cwd: Some("/tmp/default".to_owned()),
            default_ssh_auth_sock: Some("/tmp/wezterm-agent.sock".to_owned()),
            default_mux_server_domain: Some("mux-main".to_owned()),
            daemon_options: Some(NativeDaemonOptions {
                pid_file: Some("run/wezterm.pid".to_owned()),
                stdout: Some("logs/wezterm.out".to_owned()),
                stderr: Some("logs/wezterm.err".to_owned()),
            }),
            exec_domains: Some(vec![NativeExecDomain {
                name: "ops".to_owned(),
                fixup_command: "exec-domain-ops".to_owned(),
                label: Some(NativeExecDomainLabel::Value("Ops".to_owned())),
            }]),
            wsl_domains: Some(vec![NativeWslDomain {
                name: "WSL:Ubuntu".to_owned(),
                distribution: Some("Ubuntu".to_owned()),
                username: Some("ops".to_owned()),
                default_cwd: Some("~".to_owned()),
                default_prog: Some(vec!["zsh".to_owned(), "-l".to_owned()]),
            }]),
            unix_domains: Some(vec![NativeUnixDomain {
                name: "ops-unix".to_owned(),
                socket_path: Some("/tmp/ops.sock".to_owned()),
                connect_automatically: true,
                no_serve_automatically: true,
                serve_command: Some(vec![
                    "wezterm-mux-server".to_owned(),
                    "--daemonize".to_owned(),
                ]),
                proxy_command: Some(vec![
                    "ssh".to_owned(),
                    "ops".to_owned(),
                    "wezterm".to_owned(),
                    "cli".to_owned(),
                    "proxy".to_owned(),
                ]),
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
            tls_servers: Some(vec![NativeTlsServerDomain {
                bind_address: "127.0.0.1:8080".to_owned(),
                pem_private_key: Some("/etc/wezterm/server.key".to_owned()),
                pem_cert: Some("/etc/wezterm/server.crt".to_owned()),
                pem_ca: Some("/etc/wezterm/ca.pem".to_owned()),
                pem_root_certs: vec![
                    "/etc/ssl/certs".to_owned(),
                    "/opt/wezterm/ca.pem".to_owned(),
                ],
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
            mux_enable_ssh_agent: Some(false),
            ssh_backend: Some(NativeSshBackend::Ssh2),
            ratelimit_mux_line_prefetches_per_second: Some(12),
            mux_output_parser_buffer_size: Some(4096),
            mux_output_parser_coalesce_delay_ms: Some(7),
            periodic_stat_logging: Some(15),
            ulimit_nofile: Some(4096),
            ulimit_nproc: Some(8192),
            mux_env_remove: Some(vec!["REMOVE_ME".to_owned(), "REMOVE_TOO".to_owned()]),
            tiling_desktop_environments: Some(vec!["X11 i3".to_owned(), "Wayland Sway".to_owned()]),
            set_environment_variables: Some(sample_environment()),
            key_map_preference: Some(NativeKeyMapPreference::Physical),
            ui_key_cap_rendering: Some(NativeUiKeyCapRendering::Emacs),
            swap_backspace_and_delete: Some(true),
            enable_kitty_graphics: Some(false),
            enable_checksum_rectangular_area: Some(true),
            enable_title_reporting: Some(true),
            enable_csi_u_key_encoding: Some(true),
            enable_kitty_keyboard: Some(true),
            allow_download_protocols: Some(false),
            xcursor_theme: Some("Adwaita".to_owned()),
            xcursor_size: Some(24),
            palette_max_key_assigments_for_action: Some(3),
            allow_win32_input_mode: Some(false),
            treat_left_ctrlalt_as_altgr: Some(true),
            send_composed_key_when_left_alt_is_pressed: Some(true),
            send_composed_key_when_right_alt_is_pressed: Some(false),
            treat_east_asian_ambiguous_width_as_wide: Some(true),
            normalize_output_to_unicode_nfc: Some(true),
            unicode_version: Some(14),
            bidi_enabled: Some(true),
            bidi_direction: Some(NativeBidiDirection::AutoRightToLeft),
            use_ime: Some(false),
            use_dead_keys: Some(false),
            ime_preedit_rendering: Some(NativeImePreeditRendering::System),
            macos_forward_to_ime_modifier_mask: Some(ModifiersState::ALT),
            xim_im_name: Some("fcitx".to_owned()),
            detect_password_input: Some(false),
            launch_menu: Some(vec![NativeLaunchMenuItem {
                label: Some("Top".to_owned()),
                command: NativeLaunchMenuCommand::Command(WindowSpawnCommandQuery {
                    label: None,
                    program: "top".to_owned(),
                    args: vec!["-H".to_owned()],
                    cwd: Some("/tmp/default".to_owned()),
                    environment: sample_environment(),
                    domain: None,
                    window_position: None,
                }),
            }]),
            leader: Some(NativeLeaderKey {
                keys: "CTRL+A".to_owned(),
                timeout_milliseconds: Some(750),
            }),
            key_assignments: Some(vec![NativeUserKeyAssignment {
                keys: "CTRL+ALT+D".to_owned(),
                command: WindowCommand::ShowDebugOverlay,
            }]),
            key_tables: None,
            mouse_assignments: Some(vec![NativeUserMouseAssignment {
                event: NativeMouseAssignmentEvent {
                    kind: NativeMouseAssignmentEventKind::Drag,
                    button: NativeMouseAssignmentButton::Mouse(MouseButton::Left),
                    streak: 1,
                },
                modifiers: ModifiersState::ALT,
                mouse_reporting: false,
                alt_screen: NativeMouseAssignmentAltScreen::Any,
                command: WindowCommand::StartWindowDrag,
            }]),
            lua_tab_title: Some(NativeLuaTabTitle::Static(NativeTabTitle::Text(
                "Lua Tab".to_owned(),
            ))),
            lua_window_title: Some(NativeLuaWindowTitle::Static("Lua Title".to_owned())),
            lua_update_status: Some(NativeLuaWindowStatusUpdate {
                left_status: Some(NativeLuaWindowStatusText::Static("LEFT".to_owned())),
                right_status: Some(NativeLuaWindowStatusText::Static("RIGHT".to_owned())),
            }),
            lua_update_status_config_overrides: None,
            lua_bell: None,
            lua_focus_changed: None,
            lua_resized: None,
            lua_config_reloaded: None,
            lua_user_var_changed: None,
            lua_open_uri: Some(NativeLuaOpenUri::Static {
                allow_default: false,
            }),
            lua_new_tab_button_click: Some(NativeLuaNewTabButtonClick {
                allow_default: NativeLuaNewTabButtonClickAllowDefault::Static(false),
                perform_default_action: false,
            }),
            lua_command_palette_entries: None,
            lua_emit_event_handlers: None,
            scroll_to_bottom_on_input: Some(false),
            adjust_window_size_when_changing_font_size: Some(false),
            canonicalize_pasted_newlines: Some(NativeCanonicalizePastedNewlines::LineFeed),
            quote_dropped_files: Some(NativeQuoteDroppedFiles::Posix),
            disable_default_key_bindings: Some(true),
            disable_default_mouse_bindings: Some(true),
            hide_mouse_cursor_when_typing: Some(false),
            alternate_buffer_wheel_scroll_speed: Some(1),
            pane_focus_follows_mouse: Some(true),
            swallow_mouse_click_on_pane_focus: Some(true),
            swallow_mouse_click_on_window_focus: Some(true),
            bypass_mouse_reporting_modifiers: Some(ModifiersState::ALT),
            enable_scroll_bar: Some(true),
            scrollback_lines: Some(12),
            min_scroll_bar_height: Some(NativeScrollBarHeight::Pixels(12)),
            enable_tab_bar: Some(false),
            hide_tab_bar_if_only_one_tab: Some(true),
            use_fancy_tab_bar: Some(false),
            unzoom_on_switch_pane: Some(false),
            tab_bar_at_bottom: Some(true),
            tab_and_split_indices_are_zero_based: Some(true),
            mouse_wheel_scrolls_tabs: Some(false),
            switch_to_last_active_tab_when_closing_tab: Some(true),
            quit_when_all_windows_are_closed: Some(false),
            window_close_confirmation: Some(NativeWindowCloseConfirmation::NeverPrompt),
            exit_behavior: Some(NativeExitBehavior::Hold),
            clean_exit_codes: Some(vec![130]),
            exit_behavior_messaging: Some(NativeExitBehaviorMessaging::Brief),
            skip_close_confirmation_for_processes_named: Some(vec!["top".to_owned()]),
            show_close_tab_button_in_tabs: Some(false),
            show_new_tab_button_in_tab_bar: Some(false),
            show_tab_index_in_tab_bar: Some(false),
            show_tabs_in_tab_bar: Some(false),
            ..NativeConfigSnapshot::default()
        };
        snapshot.refresh_effective_config();
        snapshot

    }};
}

macro_rules! sample_effective_config {
    () => {{
        native_config_view! {
            dpi: 144,
            dpi_by_screen: BTreeMap::from([
                ("Built-in Retina Display".to_owned(), 144),
                ("HDMI".to_owned(), 96),
            ]),
            tab_max_width: 32,
            tab_min_width: 8,
            status_update_interval: 250,
            status_update_interval_ms: 250,
            max_fps: 144,
            animation_fps: 24,
            front_end: NativeRenderFrontEnd::WebGpu,
            webgpu_power_preference: NativeWebGpuPowerPreference::HighPerformance,
            webgpu_force_fallback_adapter: true,
            webgpu_preferred_adapter: Some(NativeWebGpuPreferredAdapter {
                backend: Some("Vulkan".to_owned()),
                device: Some(29_730),
                device_type: Some("DiscreteGpu".to_owned()),
                driver: Some("radv".to_owned()),
                driver_info: Some("Mesa 22.3.4".to_owned()),
                name: Some("AMD Radeon Pro W6400 (RADV NAVI24)".to_owned()),
                vendor: Some(4_098),
            }),
            prefer_egl: false,
            enable_wayland: false,
            enable_zwlr_output_manager: true,
            use_box_model_render: true,
            experimental_pixel_positioning: true,
            shape_cache_size: 2_048,
            line_state_cache_size: 512,
            line_quad_cache_size: 768,
            line_to_ele_shape_cache_size: 1_536,
            glyph_cache_image_cache_size: 128,
            cursor_blink_rate: 375,
            cursor_blink_rate_ms: 375,
            cursor_blink_ease_in: NativeEasingFunction::EaseIn,
            cursor_blink_ease_out: NativeEasingFunction::EaseOut,
            text_blink_rate: 525,
            text_blink_rate_ms: 525,
            text_blink_rate_rapid: 175,
            text_blink_rate_rapid_ms: 175,
            text_blink_ease_in: NativeEasingFunction::EaseIn,
            text_blink_ease_out: NativeEasingFunction::EaseOut,
            text_blink_rapid_ease_in: NativeEasingFunction::EaseInOut,
            text_blink_rapid_ease_out: NativeEasingFunction::Constant,
            font: Some("JetBrains Mono".to_owned()),
            font_fallbacks: vec!["Noto Color Emoji".to_owned()],
            font_attributes: NativeFontAttributes {
                weight: Some("Bold".to_owned()),
                stretch: Some("Expanded".to_owned()),
                style: Some("Italic".to_owned()),
                harfbuzz_features: Vec::new(),
                assume_emoji_presentation: None,
                freetype_load_target: None,
                freetype_render_target: None,
                freetype_load_flags: None,
            },
            font_rules: vec![NativeFontRule {
                italic: Some(true),
                intensity: Some(NativeFormatIntensity::Bold),
                font: Some("Victor Mono".to_owned()),
                font_fallbacks: Vec::new(),
                ..NativeFontRule::default()
            }],
            font_size: NativeFontSize::from_millipoints(13_500),
            cell_width: NativeCellWidth::from_per_mille(1_250),
            cell_widths: vec![NativeCellWidthOverride::new(0xe000, 0xf8ff, 2)],
            line_height: NativeLineHeight::from_per_mille(1_250),
            font_antialias: NativeFontAntialias::Subpixel,
            font_hinting: NativeFontHinting::VerticalSubpixel,
            font_rasterizer: NativeFontRasterizer::FreeType,
            font_colr_rasterizer: NativeFontRasterizer::FreeType,
            font_shaper: NativeFontShaper::Harfbuzz,
            harfbuzz_features: vec!["kern".to_owned(), "liga=0".to_owned()],
            font_dirs: vec!["fonts".to_owned(), "vendor/fonts".to_owned()],
            font_locator: Some(NativeFontLocator::ConfigDirsOnly),
            use_cap_height_to_scale_fallback_fonts: true,
            ignore_svg_fonts: true,
            sort_fallback_fonts_by_coverage: true,
            search_font_dirs_for_fallback: true,
            custom_block_glyphs: false,
            anti_alias_custom_block_glyphs: false,
            allow_square_glyphs_to_overflow_width: NativeSquareGlyphOverflow::Always,
            freetype_load_target: NativeFreetypeTarget::Mono,
            freetype_render_target: NativeFreetypeTarget::HorizontalLcd,
            freetype_load_flags: NativeFreetypeLoadFlags::NO_HINTING
                .union(NativeFreetypeLoadFlags::MONOCHROME),
            freetype_interpreter_version: Some(38),
            freetype_pcf_long_family_names: true,
            display_pixel_geometry: NativeDisplayPixelGeometry::Bgr,
            foreground_text_hsb: NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(0.8),
                brightness: NativeHsbMultiplier::from_f32(0.7),
            },
            bold_brightens_ansi_colors: NativeBoldBrightensAnsiColors::BrightOnly,
            text_background_opacity: NativeTextBackgroundOpacity::from_f32(0.4),
            window_background_opacity: NativeTextBackgroundOpacity::from_f32(0.5),
            background: vec![super::NativeWindowBackgroundVisualLayer::Color(Color::Rgb(
                42, 43, 44,
            ))],
            window_background_image: Some("wallpaper.png".to_owned()),
            window_background_image_hsb: Some(NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(0.9),
                brightness: NativeHsbMultiplier::from_f32(0.6),
            }),
            window_background_gradient: Some(NativeWindowBackgroundGradient {
                orientation: NativeWindowBackgroundGradientOrientation::Vertical,
                interpolation: NativeWindowBackgroundGradientInterpolation::Linear,
                blend: NativeWindowBackgroundGradientBlend::Rgb,
                noise: None,
                segment: None,
                preset: None,
                opacity_alpha: u8::MAX,
                blend_with_background_color: false,
                hsb: super::native_identity_hsb(),
                colors: vec![Color::Rgb(1, 2, 3), Color::Rgb(17, 18, 19)],
            }),
            window_background_images: Vec::new(),
            window_background_layers: Vec::new(),
            kde_window_background_blur: true,
            macos_window_background_blur: 20,
            win32_system_backdrop: NativeWin32SystemBackdrop::Mica,
            win32_acrylic_accent_color: Some(Color::Rgb(17, 34, 51)),
            window_decorations: NativeWindowDecorations {
                title: false,
                resize: true,
                integrated_buttons: true,
                macos_force_disable_shadow: true,
                macos_force_enable_shadow: false,
                macos_force_square_corners: false,
                macos_use_background_color_as_titlebar_color: true,
            },
            window_frame: sample_window_frame_appearance(),
            window_frame_appearance: sample_window_frame_appearance(),
            integrated_title_buttons: vec![
                NativeIntegratedTitleButton::Close,
                NativeIntegratedTitleButton::Hide,
            ],
            integrated_title_button_alignment: NativeIntegratedTitleButtonAlignment::Left,
            integrated_title_button_color: NativeIntegratedTitleButtonColor::Color(Color::Rgb(
                1, 2, 3,
            )),
            integrated_title_button_style: NativeIntegratedTitleButtonStyle::Gnome,
            default_cursor_style: NativeCursorStyle::BlinkingUnderline,
            cursor_thickness: Some(NativeCursorThickness::Pixels(3)),
            underline_thickness: Some(NativeUnderlineThickness::Pixels(2)),
            underline_position: Some(NativeUnderlinePosition::Pixels(-2)),
            strikethrough_position: Some(NativeStrikethroughPosition::CellFractionPerMille(500)),
            force_reverse_video_cursor: true,
            reverse_video_cursor_min_contrast: NativeContrastRatio::from_centi(325),
            text_min_contrast_ratio: Some(NativeTextMinContrastRatio::from_centi(450)),
            window_padding: NativeWindowPadding {
                left: NativeWindowPaddingDimension::Pixels(1),
                right: NativeWindowPaddingDimension::Pixels(2),
                top: NativeWindowPaddingDimension::Pixels(3),
                bottom: NativeWindowPaddingDimension::Pixels(4),
            },
            window_content_alignment: NativeWindowContentAlignment {
                horizontal: NativeHorizontalContentAlignment::Center,
                vertical: NativeVerticalContentAlignment::Bottom,
            },
            initial_cols: 100,
            initial_rows: 30,
            inactive_pane_hsb: NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(0.7),
                brightness: NativeHsbMultiplier::from_f32(0.6),
            },
            command_palette_rows: Some(12),
            command_palette_font: Some(super::native_font_config("Monaco")),
            command_palette_font_size: NativeFontSize::from_millipoints(15_500),
            command_palette_bg_color: Some(Color::Rgb(15, 16, 17)),
            command_palette_fg_color: Some(Color::Rgb(18, 19, 20)),
            char_select_font: Some(super::native_font_config("Monaco")),
            char_select_font_size: NativeFontSize::from_millipoints(16_250),
            char_select_bg_color: Some(Color::Rgb(21, 22, 23)),
            char_select_fg_color: Some(Color::Rgb(24, 25, 26)),
            pane_select_font: Some(super::native_font_config("Monaco")),
            pane_select_font_size: NativeFontSize::from_millipoints(36_500),
            pane_select_bg_color: Some(Color::Rgb(27, 28, 29)),
            pane_select_fg_color: Some(Color::Rgb(30, 31, 32)),
            launcher_alphabet: "12".to_owned(),
            quick_select_alphabet: "xy".to_owned(),
            quick_select_patterns: vec!["ticket-[0-9]+".to_owned()],
            disable_default_quick_select_patterns: true,
            quick_select_remove_styling: true,
            hyperlink_rules: vec![NativeHyperlinkRule {
                regex: r"\bT(\d+)\b".to_owned(),
                format: "https://tickets.example/$1".to_owned(),
                highlight: 1,
            }],
            copy_mode_active_highlight_bg: Some(NativeColorSpec::Color(Color::Rgb(21, 22, 23))),
            copy_mode_active_highlight_fg: Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Black)),
            copy_mode_inactive_highlight_bg: Some(NativeColorSpec::Color(Color::Rgb(24, 25, 26))),
            copy_mode_inactive_highlight_fg: Some(NativeColorSpec::AnsiColor(
                NativeAnsiColor::White,
            )),
            quick_select_label_bg: Some(NativeColorSpec::Color(Color::Rgb(27, 28, 29))),
            quick_select_label_fg: Some(NativeColorSpec::Color(Color::Rgb(30, 31, 32))),
            quick_select_match_bg: Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Navy)),
            quick_select_match_fg: Some(NativeColorSpec::Color(Color::Rgb(33, 34, 35))),
            input_selector_label_bg: Some(NativeColorSpec::Color(Color::Rgb(34, 35, 36))),
            input_selector_label_fg: Some(NativeColorSpec::Color(Color::Rgb(37, 38, 39))),
            launcher_label_bg: Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Black)),
            launcher_label_fg: Some(NativeColorSpec::Color(Color::Rgb(40, 41, 42))),
            selection_word_boundary: " :".to_owned(),
            term: "wezterm".to_owned(),
            enq_answerback: "rterm".to_owned(),
            audible_bell: NativeAudibleBell::Disabled,
            visual_bell: NativeVisualBell {
                fade_in_duration_ms: 10,
                fade_out_duration_ms: 20,
                fade_in_function: NativeEasingFunction::EaseIn,
                fade_out_function: NativeEasingFunction::EaseOut,
                target: NativeVisualBellTarget::BackgroundColor,
            },
            colors: Some(Box::new(sample_palette())),
            color_scheme: Some("Project Scheme".to_owned()),
            color_scheme_dirs: vec!["colors".to_owned(), "more-colors".to_owned()],
            color_schemes: sample_color_schemes(),
            resolved_palette: sample_resolved_palette(),
            foreground_color: Color::Rgb(7, 8, 9),
            background_color: Color::Rgb(4, 5, 6),
            ansi_palette: Some(sample_ansi_palette()),
            indexed_palette: Some(sample_indexed_palette()),
            selection_fg_color: Some(Some(Color::Rgb(61, 62, 63))),
            selection_bg_color: Some(Color::Rgb(71, 72, 73)),
            cursor_bg_color: Color::Rgb(10, 11, 12),
            cursor_border_color: Some(Color::Rgb(16, 17, 18)),
            cursor_fg_color: Some(Color::Rgb(13, 14, 15)),
            compose_cursor_color: Some(Color::Rgb(22, 23, 24)),
            split_color: Some(Color::Rgb(19, 20, 21)),
            scrollbar_thumb_color: Some(Color::Rgb(22, 23, 24)),
            tab_bar_background_color: Some(Color::Rgb(25, 26, 27)),
            tab_bar_inactive_tab_edge_color: Some(Color::Rgb(27, 28, 29)),
            tab_bar_active_tab_colors: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(28, 29, 30)),
                bg_color: Some(Color::Rgb(31, 32, 33)),
                intensity: Some(NativeFormatIntensity::Bold),
                ..Default::default()
            },
            tab_bar_inactive_tab_colors: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(34, 35, 36)),
                bg_color: Some(Color::Rgb(37, 38, 39)),
                ..Default::default()
            },
            tab_bar_inactive_tab_hover_colors: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(46, 47, 48)),
                bg_color: Some(Color::Rgb(49, 50, 51)),
                ..Default::default()
            },
            tab_bar_new_tab_colors: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(40, 41, 42)),
                bg_color: Some(Color::Rgb(43, 44, 45)),
                ..Default::default()
            },
            tab_bar_new_tab_hover_colors: NativeTabBarItemColors {
                fg_color: Some(Color::Rgb(52, 53, 54)),
                bg_color: Some(Color::Rgb(55, 56, 57)),
                ..Default::default()
            },
            tab_bar_style: NativeTabBarStyle::default(),
            visual_bell_color: Some(Color::Rgb(1, 2, 3)),
            notification_handling: NativeNotificationHandling::SuppressFromFocusedWindow,
            default_prog: Some(vec!["top".to_owned(), "-H".to_owned()]),
            default_gui_startup_args: vec!["connect".to_owned(), "prod".to_owned()],
            default_domain: "local".to_owned(),
            default_workspace: "ops".to_owned(),
            prefer_to_spawn_tabs: true,
            automatically_reload_config: false,
            check_for_updates: false,
            check_for_updates_interval_seconds: 43_200,
            show_update_window: true,
            native_macos_fullscreen_mode: true,
            macos_fullscreen_extend_behind_notch: true,
            use_resize_increments: true,
            debug_key_events: true,
            log_unknown_escape_sequences: true,
            warn_about_missing_glyphs: false,
            default_cwd: Some("/tmp/default".to_owned()),
            default_ssh_auth_sock: Some("/tmp/wezterm-agent.sock".to_owned()),
            default_mux_server_domain: Some("mux-main".to_owned()),
            daemon_options: NativeDaemonOptions {
                pid_file: Some("run/wezterm.pid".to_owned()),
                stdout: Some("logs/wezterm.out".to_owned()),
                stderr: Some("logs/wezterm.err".to_owned()),
            },
            exec_domains: vec![NativeExecDomain {
                name: "ops".to_owned(),
                fixup_command: "exec-domain-ops".to_owned(),
                label: Some(NativeExecDomainLabel::Value("Ops".to_owned())),
            }],
            wsl_domains: vec![NativeWslDomain {
                name: "WSL:Ubuntu".to_owned(),
                distribution: Some("Ubuntu".to_owned()),
                username: Some("ops".to_owned()),
                default_cwd: Some("~".to_owned()),
                default_prog: Some(vec!["zsh".to_owned(), "-l".to_owned()]),
            }],
            unix_domains: vec![NativeUnixDomain {
                name: "ops-unix".to_owned(),
                socket_path: Some("/tmp/ops.sock".to_owned()),
                connect_automatically: true,
                no_serve_automatically: true,
                serve_command: Some(vec![
                    "wezterm-mux-server".to_owned(),
                    "--daemonize".to_owned(),
                ]),
                proxy_command: Some(vec![
                    "ssh".to_owned(),
                    "ops".to_owned(),
                    "wezterm".to_owned(),
                    "cli".to_owned(),
                    "proxy".to_owned(),
                ]),
                skip_permissions_check: true,
                read_timeout_ms: 45_000,
                write_timeout_ms: 30_000,
                local_echo_threshold_ms: Some(12),
                overlay_lag_indicator: true,
            }],
            ssh_domains: vec![NativeSshDomain {
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
            }],
            tls_servers: vec![NativeTlsServerDomain {
                bind_address: "127.0.0.1:8080".to_owned(),
                pem_private_key: Some("/etc/wezterm/server.key".to_owned()),
                pem_cert: Some("/etc/wezterm/server.crt".to_owned()),
                pem_ca: Some("/etc/wezterm/ca.pem".to_owned()),
                pem_root_certs: vec![
                    "/etc/ssl/certs".to_owned(),
                    "/opt/wezterm/ca.pem".to_owned(),
                ],
            }],
            tls_clients: vec![NativeTlsClientDomain {
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
            }],
            serial_ports: vec![NativeSerialDomain {
                name: "ops-console".to_owned(),
                port: Some("/dev/ttyUSB0".to_owned()),
                baud: Some(115_200),
            }],
            mux_enable_ssh_agent: false,
            ssh_backend: NativeSshBackend::Ssh2,
            ratelimit_mux_line_prefetches_per_second: 12,
            mux_output_parser_buffer_size: 4096,
            mux_output_parser_coalesce_delay_ms: 7,
            periodic_stat_logging: 15,
            ulimit_nofile: 4096,
            ulimit_nproc: 8192,
            mux_env_remove: vec!["REMOVE_ME".to_owned(), "REMOVE_TOO".to_owned()],
            tiling_desktop_environments: vec!["X11 i3".to_owned(), "Wayland Sway".to_owned()],
            set_environment_variables: sample_environment(),
            launch_menu: vec![NativeLaunchMenuItem {
                label: Some("Top".to_owned()),
                command: NativeLaunchMenuCommand::Command(WindowSpawnCommandQuery {
                    label: None,
                    program: "top".to_owned(),
                    args: vec!["-H".to_owned()],
                    cwd: Some("/tmp/default".to_owned()),
                    environment: sample_environment(),
                    domain: None,
                    window_position: None,
                }),
            }],
            leader: Some(NativeLeaderKey {
                keys: "CTRL+A".to_owned(),
                timeout_milliseconds: Some(750),
            }),
            keys: vec![NativeUserKeyAssignment {
                keys: "CTRL+ALT+D".to_owned(),
                command: WindowCommand::ShowDebugOverlay,
            }],
            key_tables: BTreeMap::new(),
            mouse_bindings: vec![NativeUserMouseAssignment {
                event: NativeMouseAssignmentEvent {
                    kind: NativeMouseAssignmentEventKind::Drag,
                    button: NativeMouseAssignmentButton::Mouse(MouseButton::Left),
                    streak: 1,
                },
                modifiers: ModifiersState::ALT,
                mouse_reporting: false,
                alt_screen: NativeMouseAssignmentAltScreen::Any,
                command: WindowCommand::StartWindowDrag,
            }],
            key_map_preference: NativeKeyMapPreference::Physical,
            ui_key_cap_rendering: NativeUiKeyCapRendering::Emacs,
            swap_backspace_and_delete: true,
            enable_kitty_graphics: false,
            enable_checksum_rectangular_area: true,
            enable_title_reporting: true,
            enable_csi_u_key_encoding: true,
            enable_kitty_keyboard: true,
            allow_download_protocols: false,
            xcursor_theme: Some("Adwaita".to_owned()),
            xcursor_size: Some(24),
            palette_max_key_assigments_for_action: 3,
            allow_win32_input_mode: false,
            treat_left_ctrlalt_as_altgr: true,
            send_composed_key_when_left_alt_is_pressed: true,
            send_composed_key_when_right_alt_is_pressed: false,
            treat_east_asian_ambiguous_width_as_wide: true,
            normalize_output_to_unicode_nfc: true,
            unicode_version: 14,
            bidi_enabled: true,
            bidi_direction: NativeBidiDirection::AutoRightToLeft,
            use_ime: false,
            use_dead_keys: false,
            ime_preedit_rendering: NativeImePreeditRendering::System,
            macos_forward_to_ime_modifier_mask: ModifiersState::ALT,
            xim_im_name: Some("fcitx".to_owned()),
            detect_password_input: false,
            scroll_to_bottom_on_input: false,
            adjust_window_size_when_changing_font_size: false,
            canonicalize_pasted_newlines: NativeCanonicalizePastedNewlines::LineFeed,
            quote_dropped_files: NativeQuoteDroppedFiles::Posix,
            disable_default_key_bindings: true,
            disable_default_mouse_bindings: true,
            hide_mouse_cursor_when_typing: false,
            alternate_buffer_wheel_scroll_speed: 1,
            pane_focus_follows_mouse: true,
            swallow_mouse_click_on_pane_focus: true,
            swallow_mouse_click_on_window_focus: true,
            bypass_mouse_reporting_modifiers: ModifiersState::ALT,
            enable_scroll_bar: true,
            scrollback_lines: 12,
            min_scroll_bar_height: Some(NativeScrollBarHeight::Pixels(12)),
            enable_tab_bar: false,
            hide_tab_bar_if_only_one_tab: true,
            use_fancy_tab_bar: false,
            unzoom_on_switch_pane: false,
            tab_bar_at_bottom: true,
            tab_and_split_indices_are_zero_based: true,
            mouse_wheel_scrolls_tabs: false,
            switch_to_last_active_tab_when_closing_tab: true,
            tab_shortcut_style: crate::window::NativeTabShortcutStyle::Terminal,
            closed_tab_history_size: 25,
            close_tab_selection: rterm_core::app_shell::CloseTabSelection::LastActive,
            tab_bar_wheel_behavior: crate::window::NativeTabBarWheelBehavior::Disabled,
            quit_when_all_windows_are_closed: false,
            window_close_confirmation: NativeWindowCloseConfirmation::NeverPrompt,
            exit_behavior: NativeExitBehavior::Hold,
            clean_exit_codes: vec![130],
            exit_behavior_messaging: NativeExitBehaviorMessaging::Brief,
            skip_close_confirmation_for_processes_named: vec!["top".to_owned()],
            show_close_tab_button_in_tabs: false,
            show_new_tab_button_in_tab_bar: false,
            show_tab_index_in_tab_bar: false,
            show_tabs_in_tab_bar: false,
        }

    }};
}

include!("window_compat_tests/part01_tests.rs");
include!("window_compat_tests/part02_tests.rs");
include!("window_compat_tests/part03_tests.rs");
include!("window_compat_tests/part04_tests.rs");
include!("window_compat_tests/part05_tests.rs");
include!("window_compat_tests/part06_tests.rs");
include!("window_compat_tests/part07_tests.rs");
include!("window_compat_tests/part08_tests.rs");
include!("window_compat_tests/part09_tests.rs");
include!("window_compat_tests/part10_tests.rs");
include!("window_compat_tests/part11_tests.rs");
include!("window_compat_tests/part12_tests.rs");
include!("window_compat_tests/part13_tests.rs");
include!("window_compat_tests/part14_tests.rs");
include!("window_compat_tests/part15_tests.rs");
include!("window_compat_tests/part16_tests.rs");
include!("window_compat_tests/part17_tests.rs");
include!("window_compat_tests/part18_tests.rs");
include!("window_compat_tests/part19_tests.rs");
include!("window_compat_tests/part20_tests.rs");
