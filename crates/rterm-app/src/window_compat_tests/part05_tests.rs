    #[test]
    fn window_app_parses_update_status_set_config_overrides_window_background_gradient() {
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
                window_background_gradient = {
                  orientation = 'Vertical',
                  colors = { '#010203', '#111213' },
                  noise = 0,
                },
              })
            end)
            "##,
        )
        .expect("expected WezTerm set_config_overrides window_background_gradient callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        assert_eq!(
            app.native_effective_config().window_background_gradient,
            Some(NativeWindowBackgroundGradient {
                orientation: NativeWindowBackgroundGradientOrientation::Vertical,
                interpolation: NativeWindowBackgroundGradientInterpolation::Linear,
                blend: NativeWindowBackgroundGradientBlend::Rgb,
                noise: Some(0),
                segment: None,
                preset: None,
                opacity_alpha: u8::MAX,
                blend_with_background_color: false,
                hsb: super::native_identity_hsb(),
                colors: vec![Color::Rgb(1, 2, 3), Color::Rgb(17, 18, 19)],
            })
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_window_background_image_hsb() {
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
                window_background_image_hsb = {
                  hue = 1.25,
                  saturation = 0.75,
                  brightness = 0.5,
                },
              })
            end)
            "##,
        )
        .expect("expected WezTerm set_config_overrides window_background_image_hsb callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        assert_eq!(
            app.native_effective_config().window_background_image_hsb,
            Some(NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.25),
                saturation: NativeHsbMultiplier::from_f32(0.75),
                brightness: NativeHsbMultiplier::from_f32(0.5),
            })
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_window_background_image() {
        let image_path = write_test_png_file("wezterm-runtime-window-background-image.png");
        let lua_path = lua_string_path(&image_path);
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.config_reloaded_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(*event);
            true
        });
        let active_pane = app.app_shell.active_pane_id();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_config_overrides({{
                window_background_image = '{lua_path}',
              }})
            end)
            "##
        ))
        .expect("expected WezTerm set_config_overrides window_background_image callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.window_background_image,
            Some(image_path.to_string_lossy().to_string())
        );
        assert_eq!(effective.window_background_images.len(), 1);
        assert_eq!(effective.background.len(), 1);
        assert_eq!(
            effective.background,
            vec![super::NativeWindowBackgroundVisualLayer::Image(
                effective.window_background_images[0].clone(),
            )]
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );

        let _ = std::fs::remove_file(image_path);
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_window_background_layers() {
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
                background = {
                  {
                    source = {
                      Gradient = {
                        orientation = 'Horizontal',
                        colors = { '#ff0000', '#ff0000' },
                        noise = 0,
                      },
                    },
                  },
                  {
                    source = { Color = '#0000ff' },
                    opacity = 0.5,
                  },
                },
              })
            end)
            "##,
        )
        .expect("expected WezTerm set_config_overrides window_background_layers callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let expected = vec![
            super::NativeWindowBackgroundVisualLayer::Gradient(NativeWindowBackgroundGradient {
                orientation: NativeWindowBackgroundGradientOrientation::Horizontal,
                interpolation: NativeWindowBackgroundGradientInterpolation::Linear,
                blend: NativeWindowBackgroundGradientBlend::Rgb,
                noise: Some(0),
                segment: None,
                preset: None,
                opacity_alpha: u8::MAX,
                blend_with_background_color: false,
                hsb: super::native_identity_hsb(),
                colors: vec![Color::Rgb(255, 0, 0), Color::Rgb(255, 0, 0)],
            }),
            super::NativeWindowBackgroundVisualLayer::Color(Color::Rgba(0, 0, 255, 127)),
        ];
        let effective = app.native_effective_config();
        assert_eq!(effective.background, expected);
        assert_eq!(effective.window_background_layers, expected);
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_bell_notification_fields() {
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
                font_size = 13.5,
                audible_bell = 'Disabled',
                visual_bell = {
                  fade_in_duration_ms = 25,
                  fade_out_duration_ms = 175,
                  fade_in_function = 'EaseIn',
                  fade_out_function = 'EaseOut',
                  target = 'CursorColor',
                },
                colors = {
                  visual_bell = '#010203',
                },
                notification_handling = 'SuppressFromFocusedWindow',
              })
              window:set_right_status(
                'audible=' .. tostring(window:effective_config().audible_bell)
                  .. ' target=' .. tostring(window:effective_config().visual_bell.target)
                  .. ' notify=' .. tostring(window:effective_config().notification_handling)
              )
            end)
            "##,
        )
        .expect("expected WezTerm set_config_overrides bell notification callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.font_size,
            NativeFontSize::from_millipoints(13_500)
        );
        assert_eq!(effective.audible_bell, NativeAudibleBell::Disabled);
        assert_eq!(
            effective.visual_bell,
            NativeVisualBell {
                fade_in_duration_ms: 25,
                fade_out_duration_ms: 175,
                fade_in_function: NativeEasingFunction::EaseIn,
                fade_out_function: NativeEasingFunction::EaseOut,
                target: NativeVisualBellTarget::CursorColor,
            }
        );
        assert_eq!(effective.visual_bell_color, Some(Color::Rgb(1, 2, 3)));
        assert_eq!(
            effective.notification_handling,
            NativeNotificationHandling::SuppressFromFocusedWindow
        );
        assert_eq!(
            app.right_status,
            "audible=Disabled target=CursorColor notify=SuppressFromFocusedWindow"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_color_scheme_fields() {
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
                font_size = 14.0,
                color_scheme = 'Runtime Scheme',
                color_scheme_dirs = { 'runtime-colors', '/opt/runtime-colors' },
                color_schemes = {
                  ['Runtime Scheme'] = {
                    foreground = '#010203',
                    background = '#040506',
                    ansi = {
                      '#000000',
                      '#111213',
                      '#141516',
                      '#171819',
                      '#1a1b1c',
                      '#1d1e1f',
                      '#202122',
                      '#232425',
                    },
                    brights = {
                      '#262728',
                      '#292a2b',
                      '#2c2d2e',
                      '#2f3031',
                      '#323334',
                      '#353637',
                      '#38393a',
                      '#3b3c3d',
                    },
                    indexed = {
                      [136] = '#070809',
                    },
                  },
                },
                colors = {
                  compose_cursor = '#0d0e0f',
                },
              })
              local palette = window:effective_config().resolved_palette
              window:set_right_status(
                'scheme=' .. tostring(window:effective_config().color_scheme)
                  .. ' dir=' .. tostring(window:effective_config().color_scheme_dirs[2])
                  .. ' fg=' .. tostring(palette.foreground)
                  .. ' bg=' .. tostring(palette.background)
                  .. ' compose=' .. tostring(palette.compose_cursor)
              )
            end)
            "##,
        )
        .expect("expected WezTerm set_config_overrides color scheme callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.font_size,
            NativeFontSize::from_millipoints(14_000)
        );
        assert_eq!(effective.color_scheme, Some("Runtime Scheme".to_owned()));
        assert_eq!(
            effective.color_scheme_dirs,
            vec![
                "runtime-colors".to_owned(),
                "/opt/runtime-colors".to_owned()
            ]
        );
        let scheme = effective
            .color_schemes
            .get("Runtime Scheme")
            .expect("expected retained Runtime Scheme");
        assert_eq!(scheme.foreground, Color::Rgb(1, 2, 3));
        assert_eq!(scheme.background, Color::Rgb(4, 5, 6));
        assert_eq!(scheme.ansi[1], Color::Rgb(17, 18, 19));
        assert_eq!(scheme.brights[1], Color::Rgb(41, 42, 43));
        assert_eq!(scheme.indexed[136], Some(Color::Rgb(7, 8, 9)));
        assert_eq!(effective.foreground_color, Color::Rgb(1, 2, 3));
        assert_eq!(effective.background_color, Color::Rgb(4, 5, 6));
        assert_eq!(effective.compose_cursor_color, Some(Color::Rgb(13, 14, 15)));
        let resolved = effective.resolved_palette;
        assert_eq!(resolved.foreground, Color::Rgb(1, 2, 3));
        assert_eq!(resolved.background, Color::Rgb(4, 5, 6));
        assert_eq!(resolved.ansi[1], Color::Rgb(17, 18, 19));
        assert_eq!(resolved.brights[1], Color::Rgb(41, 42, 43));
        assert_eq!(resolved.indexed[136], Some(Color::Rgb(7, 8, 9)));
        assert_eq!(resolved.compose_cursor, Some(Color::Rgb(13, 14, 15)));
        assert_eq!(
            app.right_status,
            "scheme=Runtime Scheme dir=/opt/runtime-colors fg=#010203 bg=#040506 compose=#0d0e0f"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_core_fields() {
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
              local overrides = {
                font_size = 16.25,
                tab_max_width = 28,
                status_update_interval = 333,
                default_workspace = 'runtime',
                enable_tab_bar = false,
              }
              window:set_config_overrides(overrides)
              window:set_right_status(
                'font=' .. window:effective_config().font_size
                  .. ' tab=' .. window:effective_config().tab_max_width
                  .. ' interval=' .. window:effective_config().status_update_interval
                  .. ' workspace=' .. window:effective_config().default_workspace
                  .. ' tabbar=' .. tostring(window:effective_config().enable_tab_bar)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides core-field callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.font_size,
            NativeFontSize::from_millipoints(16_250)
        );
        assert_eq!(effective.tab_max_width, 28);
        assert_eq!(effective.status_update_interval, 333);
        assert_eq!(effective.default_workspace, "runtime");
        assert!(!effective.enable_tab_bar);
        assert_eq!(
            app.right_status,
            "font=16.25 tab=28 interval=333 workspace=runtime tabbar=false"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_dpi_adapter_fields() {
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
                font_size = 14.5,
                dpi = 144.0,
                dpi_by_screen = {
                  ['HDMI-A-1'] = 125.0,
                },
                webgpu_preferred_adapter = {
                  backend = 'Vulkan',
                  device = 29730,
                  device_type = 'DiscreteGpu',
                  driver = 'radv',
                  driver_info = 'Mesa 22.3.4',
                  name = 'AMD Radeon Pro W6400',
                  vendor = 4098,
                },
              })
              window:set_right_status(
                'dpi=' .. tostring(window:effective_config().dpi)
                  .. ' screen=' .. tostring(window:effective_config().dpi_by_screen['HDMI-A-1'])
                  .. ' adapter=' .. tostring(window:effective_config().webgpu_preferred_adapter.backend)
                  .. '/' .. tostring(window:effective_config().webgpu_preferred_adapter.device)
                  .. '/' .. tostring(window:effective_config().webgpu_preferred_adapter.name)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides DPI adapter callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.font_size,
            NativeFontSize::from_millipoints(14_500)
        );
        assert_eq!(effective.dpi, 144);
        assert_eq!(effective.dpi_by_screen.get("HDMI-A-1"), Some(&125));
        assert_eq!(
            effective.webgpu_preferred_adapter,
            Some(NativeWebGpuPreferredAdapter {
                backend: Some("Vulkan".to_owned()),
                device: Some(29_730),
                device_type: Some("DiscreteGpu".to_owned()),
                driver: Some("radv".to_owned()),
                driver_info: Some("Mesa 22.3.4".to_owned()),
                name: Some("AMD Radeon Pro W6400".to_owned()),
                vendor: Some(4_098),
            })
        );
        assert_eq!(
            app.right_status,
            "dpi=144 screen=125 adapter=Vulkan/29730/AMD Radeon Pro W6400"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_render_diagnostics_fields() {
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
              local overrides = {
                max_fps = 144,
                animation_fps = 24,
                front_end = 'WebGpu',
                webgpu_power_preference = 'HighPerformance',
                webgpu_force_fallback_adapter = true,
                prefer_egl = false,
                enable_wayland = false,
                enable_zwlr_output_manager = true,
                use_box_model_render = true,
                experimental_pixel_positioning = true,
                debug_key_events = true,
                log_unknown_escape_sequences = true,
                warn_about_missing_glyphs = false,
              }
              window:set_config_overrides(overrides)
              window:set_right_status(
                'max=' .. window:effective_config().max_fps
                  .. ' anim=' .. window:effective_config().animation_fps
                  .. ' front=' .. window:effective_config().front_end
                  .. ' power=' .. window:effective_config().webgpu_power_preference
                  .. ' fallback=' .. tostring(window:effective_config().webgpu_force_fallback_adapter)
                  .. ' egl=' .. tostring(window:effective_config().prefer_egl)
                  .. ' wayland=' .. tostring(window:effective_config().enable_wayland)
                  .. ' zwlr=' .. tostring(window:effective_config().enable_zwlr_output_manager)
                  .. ' box=' .. tostring(window:effective_config().use_box_model_render)
                  .. ' pixel=' .. tostring(window:effective_config().experimental_pixel_positioning)
                  .. ' debug=' .. tostring(window:effective_config().debug_key_events)
                  .. ' esc=' .. tostring(window:effective_config().log_unknown_escape_sequences)
                  .. ' glyph=' .. tostring(window:effective_config().warn_about_missing_glyphs)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides render diagnostics callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(effective.max_fps, 144);
        assert_eq!(effective.animation_fps, 24);
        assert_eq!(effective.front_end, NativeRenderFrontEnd::WebGpu);
        assert_eq!(
            effective.webgpu_power_preference,
            NativeWebGpuPowerPreference::HighPerformance
        );
        assert!(effective.webgpu_force_fallback_adapter);
        assert!(!effective.prefer_egl);
        assert!(!effective.enable_wayland);
        assert!(effective.enable_zwlr_output_manager);
        assert!(effective.use_box_model_render);
        assert!(effective.experimental_pixel_positioning);
        assert!(effective.debug_key_events);
        assert!(effective.log_unknown_escape_sequences);
        assert!(!effective.warn_about_missing_glyphs);
        assert_eq!(
            app.right_status,
            "max=144 anim=24 front=WebGpu power=HighPerformance fallback=true egl=false wayland=false zwlr=true box=true pixel=true debug=true esc=true glyph=false"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_cache_blink_fields() {
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
                shape_cache_size = 2048,
                line_state_cache_size = 512,
                line_quad_cache_size = 768,
                line_to_ele_shape_cache_size = 1536,
                glyph_cache_image_cache_size = 128,
                cursor_blink_rate = 375,
                cursor_blink_ease_in = 'EaseIn',
                cursor_blink_ease_out = 'EaseOut',
                text_blink_rate = 600,
                text_blink_rate_rapid = 150,
                text_blink_ease_in = 'EaseIn',
                text_blink_ease_out = 'EaseOut',
                text_blink_rapid_ease_in = 'EaseInOut',
                text_blink_rapid_ease_out = 'Constant',
              })
              local config = window:effective_config()
              window:set_right_status(
                'shape=' .. tostring(config.shape_cache_size)
                  .. ' line=' .. tostring(config.line_state_cache_size)
                  .. '/' .. tostring(config.line_quad_cache_size)
                  .. '/' .. tostring(config.line_to_ele_shape_cache_size)
                  .. ' glyph=' .. tostring(config.glyph_cache_image_cache_size)
                  .. ' cursor=' .. tostring(config.cursor_blink_rate)
                  .. '/' .. tostring(config.cursor_blink_ease_in)
                  .. '/' .. tostring(config.cursor_blink_ease_out)
                  .. ' text=' .. tostring(config.text_blink_rate)
                  .. '/' .. tostring(config.text_blink_rate_rapid)
                  .. '/' .. tostring(config.text_blink_ease_in)
                  .. '/' .. tostring(config.text_blink_ease_out)
                  .. '/' .. tostring(config.text_blink_rapid_ease_in)
                  .. '/' .. tostring(config.text_blink_rapid_ease_out)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides cache blink callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(effective.shape_cache_size, 2_048);
        assert_eq!(effective.line_state_cache_size, 512);
        assert_eq!(effective.line_quad_cache_size, 768);
        assert_eq!(effective.line_to_ele_shape_cache_size, 1_536);
        assert_eq!(effective.glyph_cache_image_cache_size, 128);
        assert_eq!(effective.cursor_blink_rate, 375);
        assert_eq!(effective.cursor_blink_rate_ms, 375);
        assert_eq!(effective.cursor_blink_ease_in, NativeEasingFunction::EaseIn);
        assert_eq!(
            effective.cursor_blink_ease_out,
            NativeEasingFunction::EaseOut
        );
        assert_eq!(effective.text_blink_rate, 600);
        assert_eq!(effective.text_blink_rate_ms, 600);
        assert_eq!(effective.text_blink_rate_rapid, 150);
        assert_eq!(effective.text_blink_rate_rapid_ms, 150);
        assert_eq!(effective.text_blink_ease_in, NativeEasingFunction::EaseIn);
        assert_eq!(effective.text_blink_ease_out, NativeEasingFunction::EaseOut);
        assert_eq!(
            effective.text_blink_rapid_ease_in,
            NativeEasingFunction::EaseInOut
        );
        assert_eq!(
            effective.text_blink_rapid_ease_out,
            NativeEasingFunction::Constant
        );
        assert_eq!(
            app.right_status,
            "shape=2048 line=512/768/1536 glyph=128 cursor=375/EaseIn/EaseOut text=600/150/EaseIn/EaseOut/EaseInOut/Constant"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_tab_bar_fields() {
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
              local overrides = {
                hide_tab_bar_if_only_one_tab = true,
                use_fancy_tab_bar = false,
                tab_bar_at_bottom = true,
                tab_and_split_indices_are_zero_based = true,
                mouse_wheel_scrolls_tabs = false,
                switch_to_last_active_tab_when_closing_tab = true,
                show_close_tab_button_in_tabs = false,
                show_new_tab_button_in_tab_bar = false,
                show_tab_index_in_tab_bar = false,
                show_tabs_in_tab_bar = false,
              }
              window:set_config_overrides(overrides)
              window:set_right_status(
                'hide=' .. tostring(window:effective_config().hide_tab_bar_if_only_one_tab)
                  .. ' fancy=' .. tostring(window:effective_config().use_fancy_tab_bar)
                  .. ' bottom=' .. tostring(window:effective_config().tab_bar_at_bottom)
                  .. ' zero=' .. tostring(window:effective_config().tab_and_split_indices_are_zero_based)
                  .. ' wheel=' .. tostring(window:effective_config().mouse_wheel_scrolls_tabs)
                  .. ' last=' .. tostring(window:effective_config().switch_to_last_active_tab_when_closing_tab)
                  .. ' close=' .. tostring(window:effective_config().show_close_tab_button_in_tabs)
                  .. ' new=' .. tostring(window:effective_config().show_new_tab_button_in_tab_bar)
                  .. ' index=' .. tostring(window:effective_config().show_tab_index_in_tab_bar)
                  .. ' tabs=' .. tostring(window:effective_config().show_tabs_in_tab_bar)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides tab-bar callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert!(effective.hide_tab_bar_if_only_one_tab);
        assert!(!effective.use_fancy_tab_bar);
        assert!(effective.tab_bar_at_bottom);
        assert!(effective.tab_and_split_indices_are_zero_based);
        assert!(!effective.mouse_wheel_scrolls_tabs);
        assert!(effective.switch_to_last_active_tab_when_closing_tab);
        assert!(!effective.show_close_tab_button_in_tabs);
        assert!(!effective.show_new_tab_button_in_tab_bar);
        assert!(!effective.show_tab_index_in_tab_bar);
        assert!(!effective.show_tabs_in_tab_bar);
        assert_eq!(
            app.right_status,
            "hide=true fancy=false bottom=true zero=true wheel=false last=true close=false new=false index=false tabs=false"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_tab_bar_style() {
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
                tab_bar_style = {
                  active_tab_left = wezterm.format({ { Text = '[' } }),
                  active_tab_right = wezterm.format({ { Text = ']' } }),
                  new_tab = wezterm.format({ { Text = 'NEW' } }),
                },
              })
            end)
            "##,
        )
        .expect("expected WezTerm set_config_overrides tab_bar_style callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.tab_bar_style.active_tab_left,
            Some(vec![NativeFormatItem::Text("[".to_owned())])
        );
        assert_eq!(
            effective.tab_bar_style.active_tab_right,
            Some(vec![NativeFormatItem::Text("]".to_owned())])
        );
        assert_eq!(
            effective.tab_bar_style.new_tab,
            Some(vec![NativeFormatItem::Text("NEW".to_owned())])
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_leader() {
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
                leader = { key = 'a', mods = 'CTRL', timeout_milliseconds = 1000 },
              })
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides leader callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        assert_eq!(
            app.native_effective_config().leader,
            Some(NativeLeaderKey {
                keys: "CTRL+a".to_owned(),
                timeout_milliseconds: Some(1000),
            })
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_input_bindings() {
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
            local act = wezterm.action

            wezterm.on('update-status', function(window, pane)
              window:set_config_overrides({
                keys = {
                  {
                    key = 'Space',
                    mods = 'CTRL|SHIFT',
                    action = act.ActivateKeyTable { name = 'resize_pane', one_shot = true },
                  },
                },
                key_tables = {
                  resize_pane = {
                    { key = 'h', action = act.SendString 'left' },
                  },
                },
                mouse_bindings = {
                  {
                    event = { Drag = { streak = 1, button = 'Left' } },
                    mods = 'ALT',
                    action = act.StartWindowDrag,
                  },
                },
              })
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides input bindings callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
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
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_update_identity_fields() {
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
              local overrides = {
                automatically_reload_config = false,
                check_for_updates = false,
                check_for_updates_interval_seconds = 123,
                show_update_window = true,
                term = 'wezterm-test',
                enq_answerback = 'RTERM',
              }
              window:set_config_overrides(overrides)
              window:set_right_status(
                'reload=' .. tostring(window:effective_config().automatically_reload_config)
                  .. ' check=' .. tostring(window:effective_config().check_for_updates)
                  .. ' interval=' .. window:effective_config().check_for_updates_interval_seconds
                  .. ' show=' .. tostring(window:effective_config().show_update_window)
                  .. ' term=' .. window:effective_config().term
                  .. ' enq=' .. window:effective_config().enq_answerback
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides update identity callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert!(!effective.automatically_reload_config);
        assert!(!effective.check_for_updates);
        assert_eq!(effective.check_for_updates_interval_seconds, 123);
        assert!(effective.show_update_window);
        assert_eq!(effective.term, "wezterm-test");
        assert_eq!(effective.enq_answerback, "RTERM");
        assert_eq!(
            app.right_status,
            "reload=false check=false interval=123 show=true term=wezterm-test enq=RTERM"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_input_mouse_fields() {
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
              local overrides = {
                scroll_to_bottom_on_input = false,
                disable_default_key_bindings = true,
                disable_default_mouse_bindings = true,
                hide_mouse_cursor_when_typing = false,
                pane_focus_follows_mouse = true,
                swallow_mouse_click_on_pane_focus = true,
                swallow_mouse_click_on_window_focus = true,
              }
              window:set_config_overrides(overrides)
              window:set_right_status(
                'scroll=' .. tostring(window:effective_config().scroll_to_bottom_on_input)
                  .. ' keys=' .. tostring(window:effective_config().disable_default_key_bindings)
                  .. ' mouse=' .. tostring(window:effective_config().disable_default_mouse_bindings)
                  .. ' hide=' .. tostring(window:effective_config().hide_mouse_cursor_when_typing)
                  .. ' focus=' .. tostring(window:effective_config().pane_focus_follows_mouse)
                  .. ' swallow-pane=' .. tostring(window:effective_config().swallow_mouse_click_on_pane_focus)
                  .. ' swallow-window=' .. tostring(window:effective_config().swallow_mouse_click_on_window_focus)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides input mouse callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert!(!effective.scroll_to_bottom_on_input);
        assert!(effective.disable_default_key_bindings);
        assert!(effective.disable_default_mouse_bindings);
        assert!(!effective.hide_mouse_cursor_when_typing);
        assert!(effective.pane_focus_follows_mouse);
        assert!(effective.swallow_mouse_click_on_pane_focus);
        assert!(effective.swallow_mouse_click_on_window_focus);
        assert_eq!(
            app.right_status,
            "scroll=false keys=true mouse=true hide=false focus=true swallow-pane=true swallow-window=true"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_scroll_paste_fields() {
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
              local overrides = {
                canonicalize_pasted_newlines = 'CarriageReturnAndLineFeed',
                quote_dropped_files = 'Posix',
                alternate_buffer_wheel_scroll_speed = 3,
                bypass_mouse_reporting_modifiers = 'ALT|SHIFT',
                enable_scroll_bar = true,
                scrollback_lines = 7,
                min_scroll_bar_height = '2cell',
                unzoom_on_switch_pane = false,
              }
              window:set_config_overrides(overrides)
              window:set_right_status(
                'paste=' .. tostring(window:effective_config().canonicalize_pasted_newlines)
                  .. ' quote=' .. tostring(window:effective_config().quote_dropped_files)
                  .. ' alt-wheel=' .. tostring(window:effective_config().alternate_buffer_wheel_scroll_speed)
                  .. ' bypass=' .. tostring(window:effective_config().bypass_mouse_reporting_modifiers)
                  .. ' scrollbar=' .. tostring(window:effective_config().enable_scroll_bar)
                  .. ' scrollback=' .. tostring(window:effective_config().scrollback_lines)
                  .. ' min=' .. tostring(window:effective_config().min_scroll_bar_height)
                  .. ' unzoom=' .. tostring(window:effective_config().unzoom_on_switch_pane)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides scroll paste callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.canonicalize_pasted_newlines,
            NativeCanonicalizePastedNewlines::CarriageReturnAndLineFeed
        );
        assert_eq!(
            effective.quote_dropped_files,
            NativeQuoteDroppedFiles::Posix
        );
        assert_eq!(effective.alternate_buffer_wheel_scroll_speed, 3);
        assert_eq!(
            effective.bypass_mouse_reporting_modifiers,
            ModifiersState::SHIFT | ModifiersState::ALT
        );
        assert!(effective.enable_scroll_bar);
        assert_eq!(effective.scrollback_lines, 7);
        assert_eq!(
            effective.min_scroll_bar_height,
            Some(NativeScrollBarHeight::CellFractionPerMille(2_000))
        );
        assert!(!effective.unzoom_on_switch_pane);
        assert_eq!(
            app.right_status,
            "paste=CarriageReturnAndLineFeed quote=Posix alt-wheel=3 bypass=SHIFT|ALT scrollbar=true scrollback=7 min=2cell unzoom=false"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_protocol_keyboard_fields() {
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
              local overrides = {
                enable_kitty_graphics = false,
                enable_checksum_rectangular_area = true,
                enable_title_reporting = false,
                enable_csi_u_key_encoding = true,
                enable_kitty_keyboard = true,
                allow_download_protocols = true,
                allow_win32_input_mode = false,
                treat_left_ctrlalt_as_altgr = true,
                send_composed_key_when_left_alt_is_pressed = true,
                send_composed_key_when_right_alt_is_pressed = false,
                treat_east_asian_ambiguous_width_as_wide = true,
                normalize_output_to_unicode_nfc = false,
                use_ime = false,
                use_dead_keys = false,
              }
              window:set_config_overrides(overrides)
              window:set_right_status(
                'kitty=' .. tostring(window:effective_config().enable_kitty_graphics)
                  .. ' checksum=' .. tostring(window:effective_config().enable_checksum_rectangular_area)
                  .. ' title=' .. tostring(window:effective_config().enable_title_reporting)
                  .. ' csiu=' .. tostring(window:effective_config().enable_csi_u_key_encoding)
                  .. ' keyboard=' .. tostring(window:effective_config().enable_kitty_keyboard)
                  .. ' dl=' .. tostring(window:effective_config().allow_download_protocols)
                  .. ' win32=' .. tostring(window:effective_config().allow_win32_input_mode)
                  .. ' altgr=' .. tostring(window:effective_config().treat_left_ctrlalt_as_altgr)
                  .. ' lalt=' .. tostring(window:effective_config().send_composed_key_when_left_alt_is_pressed)
                  .. ' ralt=' .. tostring(window:effective_config().send_composed_key_when_right_alt_is_pressed)
                  .. ' wide=' .. tostring(window:effective_config().treat_east_asian_ambiguous_width_as_wide)
                  .. ' nfc=' .. tostring(window:effective_config().normalize_output_to_unicode_nfc)
                  .. ' ime=' .. tostring(window:effective_config().use_ime)
                  .. ' dead=' .. tostring(window:effective_config().use_dead_keys)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides protocol keyboard callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert!(!effective.enable_kitty_graphics);
        assert!(effective.enable_checksum_rectangular_area);
        assert!(!effective.enable_title_reporting);
        assert!(effective.enable_csi_u_key_encoding);
        assert!(effective.enable_kitty_keyboard);
        assert!(effective.allow_download_protocols);
        assert!(!effective.allow_win32_input_mode);
        assert!(effective.treat_left_ctrlalt_as_altgr);
        assert!(effective.send_composed_key_when_left_alt_is_pressed);
        assert!(!effective.send_composed_key_when_right_alt_is_pressed);
        assert!(effective.treat_east_asian_ambiguous_width_as_wide);
        assert!(!effective.normalize_output_to_unicode_nfc);
        assert!(!effective.use_ime);
        assert!(!effective.use_dead_keys);
        assert_eq!(
            app.right_status,
            "kitty=false checksum=true title=false csiu=true keyboard=true dl=true win32=false altgr=true lalt=true ralt=false wide=true nfc=false ime=false dead=false"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_platform_input_fields() {
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
                key_map_preference = 'Physical',
                ui_key_cap_rendering = 'Emacs',
                swap_backspace_and_delete = true,
                xcursor_theme = 'Adwaita',
                xcursor_size = 24,
                palette_max_key_assigments_for_action = 3,
                unicode_version = 14,
                bidi_enabled = true,
                bidi_direction = 'AutoRightToLeft',
                ime_preedit_rendering = 'System',
                macos_forward_to_ime_modifier_mask = 'SHIFT|CTRL',
                xim_im_name = 'fcitx',
              })
              window:set_right_status(
                'key=' .. tostring(window:effective_config().key_map_preference)
                  .. ' caps=' .. tostring(window:effective_config().ui_key_cap_rendering)
                  .. ' swap=' .. tostring(window:effective_config().swap_backspace_and_delete)
                  .. ' theme=' .. tostring(window:effective_config().xcursor_theme)
                  .. ' size=' .. tostring(window:effective_config().xcursor_size)
                  .. ' palette=' .. tostring(window:effective_config().palette_max_key_assigments_for_action)
                  .. ' unicode=' .. tostring(window:effective_config().unicode_version)
                  .. ' bidi=' .. tostring(window:effective_config().bidi_enabled)
                  .. ' dir=' .. tostring(window:effective_config().bidi_direction)
                  .. ' preedit=' .. tostring(window:effective_config().ime_preedit_rendering)
                  .. ' mask=' .. tostring(window:effective_config().macos_forward_to_ime_modifier_mask)
                  .. ' xim=' .. tostring(window:effective_config().xim_im_name)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides platform input callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.key_map_preference,
            NativeKeyMapPreference::Physical
        );
        assert_eq!(
            effective.ui_key_cap_rendering,
            NativeUiKeyCapRendering::Emacs
        );
        assert!(effective.swap_backspace_and_delete);
        assert_eq!(effective.xcursor_theme.as_deref(), Some("Adwaita"));
        assert_eq!(effective.xcursor_size, Some(24));
        assert_eq!(effective.palette_max_key_assigments_for_action, 3);
        assert_eq!(effective.unicode_version, 14);
        assert!(effective.bidi_enabled);
        assert_eq!(
            effective.bidi_direction,
            NativeBidiDirection::AutoRightToLeft
        );
        assert_eq!(
            effective.ime_preedit_rendering,
            NativeImePreeditRendering::System
        );
        assert_eq!(
            effective.macos_forward_to_ime_modifier_mask,
            ModifiersState::SHIFT | ModifiersState::CONTROL
        );
        assert_eq!(effective.xim_im_name.as_deref(), Some("fcitx"));
        assert_eq!(
            app.right_status,
            "key=Physical caps=Emacs swap=true theme=Adwaita size=24 palette=3 unicode=14 bidi=true dir=AutoRightToLeft preedit=System mask=CTRL|SHIFT xim=fcitx"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_startup_resource_fields() {
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
              local overrides = {
                default_gui_startup_args = { 'connect', 'prod' },
                default_cwd = '/tmp/default',
                default_ssh_auth_sock = '/tmp/wezterm-agent.sock',
                default_mux_server_domain = 'mux-main',
                daemon_options = {
                  pid_file = 'run/runtime.pid',
                  stdout = 'logs/runtime.out',
                  stderr = 'logs/runtime.err',
                },
                exec_domains = {
                  wezterm.exec_domain('runtime-exec', function(cmd)
                    return cmd
                  end, 'Runtime Exec'),
                },
                wsl_domains = {
                  { name = 'WSL:Runtime', distribution = 'RuntimeLinux' },
                },
                unix_domains = {
                  { name = 'runtime-unix', socket_path = '/tmp/runtime.sock' },
                },
                ssh_domains = {
                  { name = 'runtime-ssh', remote_address = 'runtime.example.com' },
                },
                tls_servers = {
                  { bind_address = '127.0.0.1:9443', pem_cert = '/tmp/runtime.crt' },
                },
                tls_clients = {
                  { name = 'runtime-tls', remote_address = 'runtime-tls.example.com:443' },
                },
                serial_ports = {
                  { name = 'runtime-serial', port = 'COM9', baud = 115200 },
                },
                mux_enable_ssh_agent = false,
                ssh_backend = 'Ssh2',
                ratelimit_mux_line_prefetches_per_second = 12,
                mux_output_parser_buffer_size = 4096,
                mux_output_parser_coalesce_delay_ms = 7,
                mux_env_remove = { 'REMOVE_ME', 'REMOVE_TOO' },
                periodic_stat_logging = 15,
                ulimit_nofile = 4096,
                ulimit_nproc = 8192,
                tiling_desktop_environments = { 'X11 i3', 'Wayland Sway' },
                detect_password_input = false,
                native_macos_fullscreen_mode = true,
                macos_fullscreen_extend_behind_notch = true,
                use_resize_increments = true,
              }
              window:set_config_overrides(overrides)
              window:set_right_status(
                'startup=' .. tostring(window:effective_config().default_gui_startup_args[2])
                  .. ' cwd=' .. tostring(window:effective_config().default_cwd)
                  .. ' ssh-auth=' .. tostring(window:effective_config().default_ssh_auth_sock)
                  .. ' mux-domain=' .. tostring(window:effective_config().default_mux_server_domain)
                  .. ' daemon=' .. tostring(window:effective_config().daemon_options.pid_file)
                  .. ' mux-agent=' .. tostring(window:effective_config().mux_enable_ssh_agent)
                  .. ' ssh=' .. tostring(window:effective_config().ssh_backend)
                  .. ' prefetch=' .. tostring(window:effective_config().ratelimit_mux_line_prefetches_per_second)
                  .. ' buffer=' .. tostring(window:effective_config().mux_output_parser_buffer_size)
                  .. ' coalesce=' .. tostring(window:effective_config().mux_output_parser_coalesce_delay_ms)
                  .. ' mux-env=' .. tostring(window:effective_config().mux_env_remove[2])
                  .. ' stats=' .. tostring(window:effective_config().periodic_stat_logging)
                  .. ' nofile=' .. tostring(window:effective_config().ulimit_nofile)
                  .. ' nproc=' .. tostring(window:effective_config().ulimit_nproc)
                  .. ' tiling=' .. tostring(window:effective_config().tiling_desktop_environments[2])
                  .. ' detect=' .. tostring(window:effective_config().detect_password_input)
                  .. ' macos=' .. tostring(window:effective_config().native_macos_fullscreen_mode)
                  .. ' notch=' .. tostring(window:effective_config().macos_fullscreen_extend_behind_notch)
                  .. ' resize=' .. tostring(window:effective_config().use_resize_increments)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides startup resource callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.default_gui_startup_args,
            vec!["connect".to_owned(), "prod".to_owned()]
        );
        assert_eq!(effective.default_cwd.as_deref(), Some("/tmp/default"));
        assert_eq!(
            effective.default_ssh_auth_sock.as_deref(),
            Some("/tmp/wezterm-agent.sock")
        );
        assert_eq!(
            effective.default_mux_server_domain.as_deref(),
            Some("mux-main")
        );
        assert_eq!(
            effective.daemon_options,
            NativeDaemonOptions {
                pid_file: Some("run/runtime.pid".to_owned()),
                stdout: Some("logs/runtime.out".to_owned()),
                stderr: Some("logs/runtime.err".to_owned()),
            }
        );
        assert_eq!(
            effective.exec_domains,
            vec![NativeExecDomain {
                name: "runtime-exec".to_owned(),
                fixup_command: "exec-domain-runtime-exec".to_owned(),
                label: Some(NativeExecDomainLabel::Value("Runtime Exec".to_owned())),
            }]
        );
        assert_eq!(
            effective.wsl_domains,
            vec![NativeWslDomain {
                name: "WSL:Runtime".to_owned(),
                distribution: Some("RuntimeLinux".to_owned()),
                username: None,
                default_cwd: None,
                default_prog: None,
            }]
        );
        assert_eq!(
            effective.unix_domains,
            vec![NativeUnixDomain {
                name: "runtime-unix".to_owned(),
                socket_path: Some("/tmp/runtime.sock".to_owned()),
                connect_automatically: false,
                no_serve_automatically: false,
                serve_command: None,
                proxy_command: None,
                skip_permissions_check: false,
                read_timeout_ms: 60_000,
                write_timeout_ms: 60_000,
                local_echo_threshold_ms: None,
                overlay_lag_indicator: false,
            }]
        );
        assert_eq!(
            effective.ssh_domains,
            vec![NativeSshDomain {
                name: "runtime-ssh".to_owned(),
                remote_address: "runtime.example.com".to_owned(),
                no_agent_auth: false,
                username: None,
                connect_automatically: false,
                timeout_ms: 60_000,
                local_echo_threshold_ms: Some(100),
                overlay_lag_indicator: false,
                remote_wezterm_path: None,
                override_proxy_command: None,
                ssh_backend: None,
                multiplexing: NativeSshMultiplexing::WezTerm,
                ssh_option: BTreeMap::new(),
                default_prog: None,
                assume_shell: NativeShellAssumption::Unknown,
            }]
        );
        assert_eq!(
            effective.tls_servers,
            vec![NativeTlsServerDomain {
                bind_address: "127.0.0.1:9443".to_owned(),
                pem_private_key: None,
                pem_cert: Some("/tmp/runtime.crt".to_owned()),
                pem_ca: None,
                pem_root_certs: Vec::new(),
            }]
        );
        assert_eq!(
            effective.tls_clients,
            vec![NativeTlsClientDomain {
                name: "runtime-tls".to_owned(),
                bootstrap_via_ssh: None,
                remote_address: "runtime-tls.example.com:443".to_owned(),
                pem_private_key: None,
                pem_cert: None,
                pem_ca: None,
                pem_root_certs: Vec::new(),
                accept_invalid_hostnames: false,
                expected_cn: None,
                connect_automatically: false,
                read_timeout_ms: 60_000,
                write_timeout_ms: 60_000,
                local_echo_threshold_ms: Some(100),
                remote_wezterm_path: None,
                overlay_lag_indicator: false,
            }]
        );
        assert_eq!(
            effective.serial_ports,
            vec![NativeSerialDomain {
                name: "runtime-serial".to_owned(),
                port: Some("COM9".to_owned()),
                baud: Some(115_200),
            }]
        );
        assert!(!effective.mux_enable_ssh_agent);
        assert_eq!(effective.ssh_backend, NativeSshBackend::Ssh2);
        assert_eq!(effective.ratelimit_mux_line_prefetches_per_second, 12);
        assert_eq!(effective.mux_output_parser_buffer_size, 4096);
        assert_eq!(effective.mux_output_parser_coalesce_delay_ms, 7);
        assert_eq!(
            effective.mux_env_remove,
            vec!["REMOVE_ME".to_owned(), "REMOVE_TOO".to_owned()]
        );
        assert_eq!(effective.periodic_stat_logging, 15);
        assert_eq!(effective.ulimit_nofile, 4096);
        assert_eq!(effective.ulimit_nproc, 8192);
        assert_eq!(
            effective.tiling_desktop_environments,
            vec!["X11 i3".to_owned(), "Wayland Sway".to_owned()]
        );
        assert!(!effective.detect_password_input);
        assert!(effective.native_macos_fullscreen_mode);
        assert!(effective.macos_fullscreen_extend_behind_notch);
        assert!(effective.use_resize_increments);
        assert_eq!(
            app.right_status,
            "startup=prod cwd=/tmp/default ssh-auth=/tmp/wezterm-agent.sock mux-domain=mux-main daemon=run/runtime.pid mux-agent=false ssh=Ssh2 prefetch=12 buffer=4096 coalesce=7 mux-env=REMOVE_TOO stats=15 nofile=4096 nproc=8192 tiling=Wayland Sway detect=false macos=true notch=true resize=true"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_launch_defaults_fields() {
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
              local overrides = {
                default_prog = { 'nu', '--login' },
                default_domain = 'local',
                prefer_to_spawn_tabs = true,
                set_environment_variables = {
                  PROJECT_MODE = 'dev',
                  FEATURE_FLAG = 'on',
                },
              }
              window:set_config_overrides(overrides)
              window:set_right_status(
                'prog=' .. tostring(window:effective_config().default_prog[2])
                  .. ' domain=' .. tostring(window:effective_config().default_domain)
                  .. ' prefer=' .. tostring(window:effective_config().prefer_to_spawn_tabs)
                  .. ' env=' .. tostring(window:effective_config().set_environment_variables.PROJECT_MODE)
                  .. ' flag=' .. tostring(window:effective_config().set_environment_variables.FEATURE_FLAG)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides launch defaults callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.default_prog,
            Some(vec!["nu".to_owned(), "--login".to_owned()])
        );
        assert_eq!(effective.default_domain, "local");
        assert!(effective.prefer_to_spawn_tabs);
        assert_eq!(
            effective
                .set_environment_variables
                .get("PROJECT_MODE")
                .map(String::as_str),
            Some("dev")
        );
        assert_eq!(
            effective
                .set_environment_variables
                .get("FEATURE_FLAG")
                .map(String::as_str),
            Some("on")
        );
        assert_eq!(
            app.right_status,
            "prog=--login domain=local prefer=true env=dev flag=on"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_launch_menu_fields() {
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
                launch_menu = {
                  {
                    label = 'Runtime Monitor',
                    args = { 'top', '-H' },
                    cwd = '/tmp/runtime',
                    set_environment_variables = {
                      PROJECT_MODE = 'runtime',
                    },
                  },
                },
              })
              local item = window:effective_config().launch_menu[1]
              local env = window:effective_config().launch_menu[1].set_environment_variables
              window:set_right_status(
                'launch=' .. tostring(item.label)
                  .. ' program=' .. tostring(item.args[1])
                  .. ' arg=' .. tostring(item.args[2])
                  .. ' cwd=' .. tostring(item.cwd)
                  .. ' env=' .. tostring(env.PROJECT_MODE)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides launch_menu callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.launch_menu,
            vec![NativeLaunchMenuItem {
                label: Some("Runtime Monitor".to_owned()),
                command: NativeLaunchMenuCommand::Command(WindowSpawnCommandQuery {
                    label: Some("Runtime Monitor".to_owned()),
                    program: "top".to_owned(),
                    args: vec!["-H".to_owned()],
                    cwd: Some("/tmp/runtime".to_owned()),
                    environment: BTreeMap::from([(
                        "PROJECT_MODE".to_owned(),
                        "runtime".to_owned(),
                    )]),
                    domain: None,
                    window_position: None,
                }),
            }]
        );
        assert_eq!(
            app.right_status,
            "launch=Runtime Monitor program=top arg=-H cwd=/tmp/runtime env=runtime"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_close_exit_fields() {
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
              local overrides = {
                quit_when_all_windows_are_closed = false,
                window_close_confirmation = 'NeverPrompt',
                exit_behavior = 'CloseOnCleanExit',
                clean_exit_codes = { 130, 143 },
                exit_behavior_messaging = 'Terse',
                skip_close_confirmation_for_processes_named = { 'top', 'htop' },
              }
              window:set_config_overrides(overrides)
              window:set_right_status(
                'quit=' .. tostring(window:effective_config().quit_when_all_windows_are_closed)
                  .. ' close=' .. tostring(window:effective_config().window_close_confirmation)
                  .. ' exit=' .. tostring(window:effective_config().exit_behavior)
                  .. ' clean=' .. tostring(window:effective_config().clean_exit_codes[2])
                  .. ' msg=' .. tostring(window:effective_config().exit_behavior_messaging)
                  .. ' skip=' .. tostring(window:effective_config().skip_close_confirmation_for_processes_named[2])
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides close exit callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert!(!effective.quit_when_all_windows_are_closed);
        assert_eq!(
            effective.window_close_confirmation,
            NativeWindowCloseConfirmation::NeverPrompt
        );
        assert_eq!(
            effective.exit_behavior,
            NativeExitBehavior::CloseOnCleanExit
        );
        assert_eq!(effective.clean_exit_codes, vec![130, 143]);
        assert_eq!(
            effective.exit_behavior_messaging,
            NativeExitBehaviorMessaging::Terse
        );
        assert_eq!(
            effective.skip_close_confirmation_for_processes_named,
            vec!["top".to_owned(), "htop".to_owned()]
        );
        assert_eq!(
            app.right_status,
            "quit=false close=NeverPrompt exit=CloseOnCleanExit clean=143 msg=Terse skip=htop"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_window_layout_fields() {
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
              local overrides = {
                window_decorations = 'INTEGRATED_BUTTONS|RESIZE',
                initial_cols = 100,
                initial_rows = 30,
                adjust_window_size_when_changing_font_size = false,
                selection_word_boundary = ' :',
                integrated_title_buttons = { 'Close', 'Hide' },
                integrated_title_button_alignment = 'Left',
                integrated_title_button_color = '#010203',
                integrated_title_button_style = 'Gnome',
              }
              window:set_config_overrides(overrides)
              window:set_right_status(
                'decor=' .. tostring(window:effective_config().window_decorations)
                  .. ' cols=' .. tostring(window:effective_config().initial_cols)
                  .. ' rows=' .. tostring(window:effective_config().initial_rows)
                  .. ' adjust=' .. tostring(window:effective_config().adjust_window_size_when_changing_font_size)
                  .. ' boundary=' .. tostring(window:effective_config().selection_word_boundary)
                  .. ' buttons=' .. tostring(window:effective_config().integrated_title_buttons[1])
                  .. '/' .. tostring(window:effective_config().integrated_title_buttons[2])
                  .. ' button-align=' .. tostring(window:effective_config().integrated_title_button_alignment)
                  .. ' button-color=' .. tostring(window:effective_config().integrated_title_button_color)
                  .. ' button-style=' .. tostring(window:effective_config().integrated_title_button_style)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides window layout callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.window_decorations,
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
        assert_eq!(effective.initial_cols, 100);
        assert_eq!(effective.initial_rows, 30);
        assert!(!effective.adjust_window_size_when_changing_font_size);
        assert_eq!(effective.selection_word_boundary, " :");
        assert_eq!(
            effective.integrated_title_buttons,
            vec![
                NativeIntegratedTitleButton::Close,
                NativeIntegratedTitleButton::Hide,
            ]
        );
        assert_eq!(
            effective.integrated_title_button_alignment,
            NativeIntegratedTitleButtonAlignment::Left
        );
        assert_eq!(
            effective.integrated_title_button_color,
            NativeIntegratedTitleButtonColor::Color(Color::Rgb(1, 2, 3))
        );
        assert_eq!(
            effective.integrated_title_button_style,
            NativeIntegratedTitleButtonStyle::Gnome
        );
        assert_eq!(
            app.right_status,
            "decor=RESIZE|INTEGRATED_BUTTONS cols=100 rows=30 adjust=false boundary= : buttons=Close/Hide button-align=Left button-color=#010203 button-style=Gnome"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_window_frame() {
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
                window_frame = {
                  active_titlebar_bg = '#010203',
                  inactive_titlebar_fg = '#040506',
                  border_top_height = 4,
                  border_bottom_height = '1.5cell',
                },
              })
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides window_frame callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(&effective.window_frame, &effective.window_frame_appearance);
        assert_eq!(
            effective.window_frame_appearance.active_titlebar_bg,
            Some(Color::Rgb(1, 2, 3))
        );
        assert_eq!(
            effective.window_frame_appearance.inactive_titlebar_fg,
            Some(Color::Rgb(4, 5, 6))
        );
        assert_eq!(
            effective.window_frame_appearance.border_top_height,
            Some(NativeWindowPaddingDimension::Pixels(4))
        );
        assert_eq!(
            effective.window_frame_appearance.border_bottom_height,
            Some(NativeWindowPaddingDimension::CellFractionPerMille(1500))
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_window_layout_table_fields() {
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
                window_padding = {
                  left = 8,
                  right = 16,
                  top = '1cell',
                  bottom = '2pt',
                },
                window_content_alignment = {
                  horizontal = 'Center',
                  vertical = 'Bottom',
                },
              })
              local config = window:effective_config()
              window:set_right_status(
                'padding=' .. tostring(config.window_padding.left)
                  .. '/' .. tostring(config.window_padding.right)
                  .. '/' .. tostring(config.window_padding.top)
                  .. '/' .. tostring(config.window_padding.bottom)
                  .. ' align=' .. tostring(config.window_content_alignment.horizontal)
                  .. '/' .. tostring(config.window_content_alignment.vertical)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides window layout table callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.window_padding,
            NativeWindowPadding {
                left: NativeWindowPaddingDimension::Pixels(8),
                right: NativeWindowPaddingDimension::Pixels(16),
                top: NativeWindowPaddingDimension::CellFractionPerMille(1_000),
                bottom: NativeWindowPaddingDimension::Points(2),
            }
        );
        assert_eq!(
            effective.window_content_alignment,
            NativeWindowContentAlignment {
                horizontal: NativeHorizontalContentAlignment::Center,
                vertical: NativeVerticalContentAlignment::Bottom,
            }
        );
        assert_eq!(
            app.right_status,
            "padding=8px/16px/1cell/2pt align=Center/Bottom"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_cursor_decoration_fields() {
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
                bold_brightens_ansi_colors = 'BrightOnly',
                default_cursor_style = 'BlinkingBar',
                cursor_thickness = '25%',
                underline_thickness = '2px',
                underline_position = '-2px',
                strikethrough_position = '0.5cell',
                force_reverse_video_cursor = true,
                reverse_video_cursor_min_contrast = 3.25,
                text_min_contrast_ratio = 4.5,
              })
              local config = window:effective_config()
              window:set_right_status(
                'bold=' .. tostring(config.bold_brightens_ansi_colors)
                  .. ' cursor=' .. tostring(config.default_cursor_style)
                  .. ' thickness=' .. tostring(config.cursor_thickness)
                  .. ' underline=' .. tostring(config.underline_thickness)
                  .. '/' .. tostring(config.underline_position)
                  .. ' strike=' .. tostring(config.strikethrough_position)
                  .. ' reverse=' .. tostring(config.force_reverse_video_cursor)
                  .. ' contrast=' .. tostring(config.reverse_video_cursor_min_contrast)
                  .. ' text=' .. tostring(config.text_min_contrast_ratio)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides cursor decoration callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.bold_brightens_ansi_colors,
            NativeBoldBrightensAnsiColors::BrightOnly
        );
        assert_eq!(
            effective.default_cursor_style,
            NativeCursorStyle::BlinkingBar
        );
        assert_eq!(
            effective.cursor_thickness,
            Some(NativeCursorThickness::Percent(25))
        );
        assert_eq!(
            effective.underline_thickness,
            Some(NativeUnderlineThickness::Pixels(2))
        );
        assert_eq!(
            effective.underline_position,
            Some(NativeUnderlinePosition::Pixels(-2))
        );
        assert_eq!(
            effective.strikethrough_position,
            Some(NativeStrikethroughPosition::CellFractionPerMille(500))
        );
        assert!(effective.force_reverse_video_cursor);
        assert_eq!(
            effective.reverse_video_cursor_min_contrast,
            NativeContrastRatio::from_centi(325)
        );
        assert_eq!(
            effective.text_min_contrast_ratio,
            Some(NativeTextMinContrastRatio::from_centi(450))
        );
        assert_eq!(
            app.right_status,
            "bold=BrightOnly cursor=BlinkingBar thickness=25% underline=2px/-2px strike=0.5cell reverse=true contrast=3.25 text=4.5"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_selector_fields() {
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
              local overrides = {
                command_palette_rows = 12,
                command_palette_font_size = 15.5,
                char_select_font_size = 16.25,
                pane_select_font_size = 36.5,
                launcher_alphabet = '12',
                quick_select_alphabet = 'xy',
                quick_select_patterns = { 'ticket-[0-9]+', 'BUG-[0-9]+' },
                disable_default_quick_select_patterns = true,
                quick_select_remove_styling = true,
              }
              window:set_config_overrides(overrides)
              window:set_right_status(
                'rows=' .. tostring(window:effective_config().command_palette_rows)
                  .. ' command-font=' .. tostring(window:effective_config().command_palette_font_size)
                  .. ' char-font=' .. tostring(window:effective_config().char_select_font_size)
                  .. ' pane-font=' .. tostring(window:effective_config().pane_select_font_size)
                  .. ' launcher=' .. tostring(window:effective_config().launcher_alphabet)
                  .. ' quick=' .. tostring(window:effective_config().quick_select_alphabet)
                  .. ' pattern=' .. tostring(window:effective_config().quick_select_patterns[2])
                  .. ' disable=' .. tostring(window:effective_config().disable_default_quick_select_patterns)
                  .. ' styling=' .. tostring(window:effective_config().quick_select_remove_styling)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides selector callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(effective.command_palette_rows, Some(12));
        assert_eq!(
            effective.command_palette_font_size,
            NativeFontSize::from_millipoints(15_500)
        );
        assert_eq!(
            effective.char_select_font_size,
            NativeFontSize::from_millipoints(16_250)
        );
        assert_eq!(
            effective.pane_select_font_size,
            NativeFontSize::from_millipoints(36_500)
        );
        assert_eq!(effective.launcher_alphabet, "12");
        assert_eq!(effective.quick_select_alphabet, "xy");
        assert_eq!(
            effective.quick_select_patterns,
            vec!["ticket-[0-9]+".to_owned(), "BUG-[0-9]+".to_owned()]
        );
        assert!(effective.disable_default_quick_select_patterns);
        assert!(effective.quick_select_remove_styling);
        assert_eq!(
            app.right_status,
            "rows=12 command-font=15.5 char-font=16.25 pane-font=36.5 launcher=12 quick=xy pattern=BUG-[0-9]+ disable=true styling=true"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_overlay_fonts() {
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
                command_palette_font = wezterm.font_with_fallback {
                  { family = 'Iosevka Term', weight = 'Bold' },
                  'Noto Color Emoji',
                },
                char_select_font = wezterm.font {
                  family = 'Fira Code',
                  italic = true,
                },
                pane_select_font = wezterm.font 'JetBrains Mono',
              })
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides overlay fonts callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.command_palette_font,
            Some(super::NativeFontConfig {
                families: vec!["Iosevka Term".to_owned(), "Noto Color Emoji".to_owned()],
                attributes: NativeFontAttributes {
                    weight: Some("Bold".to_owned()),
                    stretch: None,
                    style: None,
                    harfbuzz_features: Vec::new(),
                    assume_emoji_presentation: None,
                    freetype_load_target: None,
                    freetype_render_target: None,
                    freetype_load_flags: None,
                }
            })
        );
        assert_eq!(
            effective.char_select_font,
            Some(super::NativeFontConfig {
                families: vec!["Fira Code".to_owned()],
                attributes: NativeFontAttributes {
                    weight: None,
                    stretch: None,
                    style: Some("Italic".to_owned()),
                    harfbuzz_features: Vec::new(),
                    assume_emoji_presentation: None,
                    freetype_load_target: None,
                    freetype_render_target: None,
                    freetype_load_flags: None,
                }
            })
        );
        assert_eq!(
            effective.pane_select_font,
            Some(super::NativeFontConfig {
                families: vec!["JetBrains Mono".to_owned()],
                attributes: NativeFontAttributes::default(),
            })
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_hyperlink_rules() {
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
                hyperlink_rules = {
                  {
                    regex = [[\bTICKET-(\d+)\b]],
                    format = 'https://tickets.example/$1',
                    highlight = 1,
                  },
                },
              })
              local rule = window:effective_config().hyperlink_rules[1]
              window:set_right_status(
                'regex=' .. tostring(rule.regex)
                  .. ' format=' .. tostring(rule.format)
                  .. ' highlight=' .. tostring(rule.highlight)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides hyperlink_rules callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.hyperlink_rules,
            vec![NativeHyperlinkRule {
                regex: r"\bTICKET-(\d+)\b".to_owned(),
                format: "https://tickets.example/$1".to_owned(),
                highlight: 1,
            }]
        );
        assert_eq!(
            app.right_status,
            r"regex=\bTICKET-(\d+)\b format=https://tickets.example/$1 highlight=1"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_update_status_set_config_overrides_overlay_color_fields() {
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
                command_palette_bg_color = '#010203',
                command_palette_fg_color = '#040506',
                char_select_bg_color = '#070809',
                char_select_fg_color = '#0a0b0c',
                pane_select_bg_color = '#0d0e0f',
                pane_select_fg_color = '#101112',
              })
              local config = window:effective_config()
              window:set_right_status(
                'palette=' .. tostring(config.command_palette_bg_color)
                  .. '/' .. tostring(config.command_palette_fg_color)
                  .. ' char=' .. tostring(config.char_select_bg_color)
                  .. '/' .. tostring(config.char_select_fg_color)
                  .. ' pane=' .. tostring(config.pane_select_bg_color)
                  .. '/' .. tostring(config.pane_select_fg_color)
              )
            end)
            "#,
        )
        .expect("expected WezTerm set_config_overrides overlay color callback");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.command_palette_bg_color,
            Some(Color::Rgb(1, 2, 3))
        );
        assert_eq!(
            effective.command_palette_fg_color,
            Some(Color::Rgb(4, 5, 6))
        );
        assert_eq!(effective.char_select_bg_color, Some(Color::Rgb(7, 8, 9)));
        assert_eq!(effective.char_select_fg_color, Some(Color::Rgb(10, 11, 12)));
        assert_eq!(effective.pane_select_bg_color, Some(Color::Rgb(13, 14, 15)));
        assert_eq!(effective.pane_select_fg_color, Some(Color::Rgb(16, 17, 18)));
        assert_eq!(
            app.right_status,
            "palette=#010203/#040506 char=#070809/#0a0b0c pane=#0d0e0f/#101112"
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
                NativeWindowConfigReloaded {
                    window_id: rterm_core::WindowId::new(1),
                    pane: active_pane,
                },
            ]
        );
    }

    #[test]
    fn window_app_parses_wezterm_update_status_effective_config_default_workspace_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_workspace = 'ops'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('workspace=' .. window:effective_config().default_workspace)
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config default_workspace status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "workspace=ops");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_effective_config_interval_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.status_update_interval = 250

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('interval=' .. window:effective_config().status_update_interval)
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config status_update_interval status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "interval=250");
    }

    #[test]
    fn window_app_parses_update_status_dpi_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.dpi = 144.0

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('dpi=' .. tostring(window:effective_config().dpi))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config dpi status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "dpi=144");
    }

    #[test]
    fn window_app_parses_update_status_dpi_by_screen_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.dpi_by_screen = {
              HDMI = 120.0,
            }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('screen-dpi=' .. tostring(window:effective_config().dpi_by_screen.HDMI))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config dpi_by_screen status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "screen-dpi=120");
    }

    #[test]
    fn window_app_parses_update_status_dpi_by_screen_bracket_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.dpi_by_screen = {
              ['HDMI-A-1'] = 125.0,
            }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('screen-dpi=' .. tostring(window:effective_config().dpi_by_screen['HDMI-A-1']))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config dpi_by_screen bracket status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "screen-dpi=125");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_effective_config_tab_max_width_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.tab_max_width = 32

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('tab=' .. window:effective_config().tab_max_width)
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config tab_max_width status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "tab=32");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_effective_config_max_fps_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.max_fps = 144

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('fps=' .. window:effective_config().max_fps)
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config max_fps status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "fps=144");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_effective_config_animation_fps_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.animation_fps = 24

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('anim=' .. window:effective_config().animation_fps)
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config animation_fps status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "anim=24");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_effective_config_front_end_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.front_end = 'WebGpu'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('front=' .. window:effective_config().front_end)
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config front_end status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "front=WebGpu");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_effective_config_webgpu_power_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.webgpu_power_preference = 'HighPerformance'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('power=' .. window:effective_config().webgpu_power_preference)
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config webgpu_power_preference status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "power=HighPerformance");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_effective_config_webgpu_fallback_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.webgpu_force_fallback_adapter = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('fallback=' .. tostring(window:effective_config().webgpu_force_fallback_adapter))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config webgpu_force_fallback_adapter status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "fallback=true");
    }

    #[test]
    fn window_app_parses_update_status_webgpu_preferred_adapter_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.webgpu_preferred_adapter = {
              backend = 'Vulkan',
              device = 29730,
              device_type = 'DiscreteGpu',
              driver = 'radv',
              driver_info = 'Mesa 22.3.4',
              name = 'AMD Radeon Pro W6400',
              vendor = 4098,
            }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status(
                'adapter=' ..
                tostring(window:effective_config().webgpu_preferred_adapter.backend) .. '/' ..
                tostring(window:effective_config().webgpu_preferred_adapter.device) .. '/' ..
                tostring(window:effective_config().webgpu_preferred_adapter.device_type) .. '/' ..
                tostring(window:effective_config().webgpu_preferred_adapter.driver) .. '/' ..
                tostring(window:effective_config().webgpu_preferred_adapter.driver_info) .. '/' ..
                tostring(window:effective_config().webgpu_preferred_adapter.name) .. '/' ..
                tostring(window:effective_config().webgpu_preferred_adapter.vendor)
              )
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config webgpu_preferred_adapter status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(
            app.right_status,
            "adapter=Vulkan/29730/DiscreteGpu/radv/Mesa 22.3.4/AMD Radeon Pro W6400/4098"
        );
    }

    #[test]
    fn window_app_parses_wezterm_update_status_effective_config_prefer_egl_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.prefer_egl = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('egl=' .. tostring(window:effective_config().prefer_egl))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config prefer_egl status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "egl=false");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_effective_config_enable_wayland_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.enable_wayland = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('wayland=' .. tostring(window:effective_config().enable_wayland))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config enable_wayland status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "wayland=false");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_effective_config_zwlr_output_manager_status_setter()
    {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.enable_zwlr_output_manager = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('zwlr=' .. tostring(window:effective_config().enable_zwlr_output_manager))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config zwlr output manager status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "zwlr=true");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_effective_config_use_box_model_render_status_setter()
    {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.use_box_model_render = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('box=' .. tostring(window:effective_config().use_box_model_render))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config use_box_model_render status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "box=true");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_effective_config_pixel_positioning_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.experimental_pixel_positioning = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('pixel=' .. tostring(window:effective_config().experimental_pixel_positioning))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config experimental_pixel_positioning status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "pixel=true");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_effective_config_shape_cache_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.shape_cache_size = 2048

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('shape=' .. tostring(window:effective_config().shape_cache_size))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config shape_cache_size status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "shape=2048");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_effective_config_line_state_cache_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.line_state_cache_size = 512

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('line-state=' .. tostring(window:effective_config().line_state_cache_size))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config line_state_cache_size status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "line-state=512");
    }

    #[test]
    fn window_app_parses_wezterm_update_status_effective_config_line_quad_cache_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.line_quad_cache_size = 768

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('line-quad=' .. tostring(window:effective_config().line_quad_cache_size))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config line_quad_cache_size status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "line-quad=768");
    }

    #[test]
    fn window_app_parses_update_status_line_to_ele_shape_cache_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.line_to_ele_shape_cache_size = 1536

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('line-to-ele=' .. tostring(window:effective_config().line_to_ele_shape_cache_size))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config line_to_ele_shape_cache_size status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "line-to-ele=1536");
    }

    #[test]
    fn window_app_parses_update_status_cell_width_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.cell_width = 1.25

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('cell-width=' .. tostring(window:effective_config().cell_width))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config cell_width status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "cell-width=1.25");
    }

    #[test]
    fn window_app_parses_update_status_cell_widths_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.cell_widths = {
              { first = 0x2606, last = 0x2606, width = 1 },
              { first = 0xe000, last = 0xf8ff, width = 2 },
            }

            wezterm.on('update-status', function(window, pane)
              local width = window:effective_config().cell_widths[2]
              window:set_right_status(
                'first=' .. tostring(width.first) ..
                ' last=' .. tostring(width.last) ..
                ' width=' .. tostring(width.width)
              )
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config cell_widths status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "first=57344 last=63743 width=2");
    }

    #[test]
    fn window_app_parses_update_status_line_height_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.line_height = 1.5

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('line-height=' .. tostring(window:effective_config().line_height))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config line_height status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "line-height=1.5");
    }

    #[test]
    fn window_app_parses_update_status_font_antialias_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font_antialias = 'Subpixel'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('font-aa=' .. tostring(window:effective_config().font_antialias))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config font_antialias status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "font-aa=Subpixel");
    }

    #[test]
    fn window_app_parses_update_status_font_hinting_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font_hinting = 'VerticalSubpixel'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('hinting=' .. tostring(window:effective_config().font_hinting))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config font_hinting status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "hinting=VerticalSubpixel");
    }

    #[test]
    fn window_app_parses_update_status_harfbuzz_features_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.harfbuzz_features = { 'liga=0', 'calt=0' }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('hb=' .. tostring(window:effective_config().harfbuzz_features[2]))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config harfbuzz_features status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "hb=calt=0");
    }

    #[test]
    fn window_app_parses_update_status_font_dirs_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font_dirs = { 'fonts', 'vendor/fonts' }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('font-dir=' .. tostring(window:effective_config().font_dirs[2]))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config font_dirs status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "font-dir=vendor/fonts");
    }

    #[test]
    fn window_app_parses_update_status_font_locator_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font_locator = 'ConfigDirsOnly'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('font-locator=' .. tostring(window:effective_config().font_locator))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config font_locator status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "font-locator=ConfigDirsOnly");
    }

    #[test]
    fn window_app_parses_update_status_use_cap_height_fallback_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.use_cap_height_to_scale_fallback_fonts = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('cap-height=' .. tostring(window:effective_config().use_cap_height_to_scale_fallback_fonts))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config use_cap_height_to_scale_fallback_fonts status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "cap-height=true");
    }

    #[test]
    fn window_app_parses_update_status_sort_fallback_fonts_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.sort_fallback_fonts_by_coverage = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('sort-fallback=' .. tostring(window:effective_config().sort_fallback_fonts_by_coverage))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config sort_fallback_fonts_by_coverage status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "sort-fallback=true");
    }

    #[test]
    fn window_app_parses_update_status_search_font_dirs_fallback_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.search_font_dirs_for_fallback = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('search-font-dirs=' .. tostring(window:effective_config().search_font_dirs_for_fallback))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config search_font_dirs_for_fallback status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "search-font-dirs=true");
    }

    #[test]
    fn window_app_parses_update_status_freetype_load_target_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.freetype_load_target = 'Light'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('ft-load=' .. tostring(window:effective_config().freetype_load_target))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config freetype_load_target status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "ft-load=Light");
    }

    #[test]
    fn window_app_parses_update_status_freetype_render_target_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.freetype_render_target = 'HorizontalLcd'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('ft-render=' .. tostring(window:effective_config().freetype_render_target))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config freetype_render_target status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "ft-render=HorizontalLcd");
    }

    #[test]
    fn window_app_parses_update_status_freetype_load_flags_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.freetype_load_flags = 'NO_HINTING|MONOCHROME'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('ft-flags=' .. tostring(window:effective_config().freetype_load_flags))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config freetype_load_flags status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "ft-flags=NO_HINTING|MONOCHROME");
    }

    #[test]
    fn window_app_parses_update_status_freetype_interpreter_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.freetype_interpreter_version = 38

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('ft-interpreter=' .. tostring(window:effective_config().freetype_interpreter_version))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config freetype_interpreter_version status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "ft-interpreter=38");
    }

    #[test]
    fn window_app_parses_update_status_freetype_pcf_long_family_names_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.freetype_pcf_long_family_names = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('pcf-long=' .. tostring(window:effective_config().freetype_pcf_long_family_names))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config freetype_pcf_long_family_names status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "pcf-long=true");
    }

    #[test]
    fn window_app_parses_update_status_bold_brightens_ansi_colors_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.bold_brightens_ansi_colors = 'BrightOnly'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('bold-bright=' .. tostring(window:effective_config().bold_brightens_ansi_colors))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config bold_brightens_ansi_colors status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "bold-bright=BrightOnly");
    }

    #[test]
    fn window_app_parses_update_status_font_rasterizer_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font_rasterizer = 'Harfbuzz'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('rasterizer=' .. tostring(window:effective_config().font_rasterizer))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config font_rasterizer status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "rasterizer=Harfbuzz");
    }

    #[test]
    fn window_app_parses_update_status_font_colr_rasterizer_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font_colr_rasterizer = 'FreeType'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('colr=' .. tostring(window:effective_config().font_colr_rasterizer))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config font_colr_rasterizer status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "colr=FreeType");
    }

    #[test]
    fn window_app_parses_update_status_font_shaper_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font_shaper = 'Harfbuzz'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('shaper=' .. tostring(window:effective_config().font_shaper))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config font_shaper status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "shaper=Harfbuzz");
    }

    #[test]
    fn window_app_parses_update_status_square_glyph_overflow_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.allow_square_glyphs_to_overflow_width = 'Always'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('overflow=' .. tostring(window:effective_config().allow_square_glyphs_to_overflow_width))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config allow_square_glyphs_to_overflow_width status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "overflow=Always");
    }

    #[test]
    fn window_app_parses_update_status_display_pixel_geometry_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.display_pixel_geometry = 'BGR'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('geometry=' .. tostring(window:effective_config().display_pixel_geometry))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config display_pixel_geometry status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "geometry=BGR");
    }

    #[test]
    fn window_app_parses_update_status_text_background_opacity_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.text_background_opacity = 0.4

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('text-bg-opacity=' .. tostring(window:effective_config().text_background_opacity))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config text_background_opacity status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "text-bg-opacity=0.4");
    }

    #[test]
    fn window_app_parses_update_status_window_background_opacity_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_background_opacity = 0.5

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('window-bg-opacity=' .. tostring(window:effective_config().window_background_opacity))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config window_background_opacity status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "window-bg-opacity=0.5");
    }

    #[test]
    fn window_app_parses_update_status_foreground_text_hsb_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.foreground_text_hsb = {
              hue = 0.5,
              saturation = 1.25,
              brightness = 0.75,
            }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status(
                'fg-hsb=' ..
                tostring(window:effective_config().foreground_text_hsb.hue) .. ',' ..
                tostring(window:effective_config().foreground_text_hsb.saturation) .. ',' ..
                tostring(window:effective_config().foreground_text_hsb.brightness)
              )
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config foreground_text_hsb status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "fg-hsb=0.5,1.25,0.75");
    }

    #[test]
    fn window_app_parses_update_status_inactive_pane_hsb_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.inactive_pane_hsb = {
              hue = 0.8,
              saturation = 0.9,
              brightness = 1.1,
            }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status(
                'inactive-hsb=' ..
                tostring(window:effective_config().inactive_pane_hsb.hue) .. ',' ..
                tostring(window:effective_config().inactive_pane_hsb.saturation) .. ',' ..
                tostring(window:effective_config().inactive_pane_hsb.brightness)
              )
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config inactive_pane_hsb status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "inactive-hsb=0.8,0.9,1.1");
    }

    #[test]
    fn window_app_parses_update_status_glyph_cache_image_cache_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.glyph_cache_image_cache_size = 128

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('glyph-cache=' .. tostring(window:effective_config().glyph_cache_image_cache_size))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config glyph_cache_image_cache_size status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "glyph-cache=128");
    }

    #[test]
    fn window_app_parses_update_status_cursor_blink_rate_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.cursor_blink_rate = 375

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('cursor-rate=' .. tostring(window:effective_config().cursor_blink_rate))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config cursor_blink_rate status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "cursor-rate=375");
    }

    #[test]
    fn window_app_parses_update_status_cursor_blink_ease_in_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.cursor_blink_ease_in = 'EaseIn'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('cursor-ease-in=' .. tostring(window:effective_config().cursor_blink_ease_in))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config cursor_blink_ease_in status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "cursor-ease-in=EaseIn");
    }

    #[test]
    fn window_app_parses_update_status_cursor_blink_ease_out_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.cursor_blink_ease_out = 'EaseOut'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('cursor-ease-out=' .. tostring(window:effective_config().cursor_blink_ease_out))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config cursor_blink_ease_out status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "cursor-ease-out=EaseOut");
    }

    #[test]
    fn window_app_parses_update_status_text_blink_rate_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.text_blink_rate = 600

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('text-rate=' .. tostring(window:effective_config().text_blink_rate))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config text_blink_rate status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "text-rate=600");
    }

    #[test]
    fn window_app_parses_update_status_text_blink_rate_rapid_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.text_blink_rate_rapid = 150

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('rapid-rate=' .. tostring(window:effective_config().text_blink_rate_rapid))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config text_blink_rate_rapid status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "rapid-rate=150");
    }

    #[test]
    fn window_app_parses_update_status_text_blink_ease_in_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.text_blink_ease_in = 'EaseIn'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('text-ease-in=' .. tostring(window:effective_config().text_blink_ease_in))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config text_blink_ease_in status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "text-ease-in=EaseIn");
    }

    #[test]
    fn window_app_parses_update_status_text_blink_ease_out_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.text_blink_ease_out = 'EaseOut'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('text-ease-out=' .. tostring(window:effective_config().text_blink_ease_out))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config text_blink_ease_out status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "text-ease-out=EaseOut");
    }

    #[test]
    fn window_app_parses_update_status_text_blink_rapid_ease_in_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.text_blink_rapid_ease_in = 'EaseInOut'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('rapid-ease-in=' .. tostring(window:effective_config().text_blink_rapid_ease_in))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config text_blink_rapid_ease_in status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "rapid-ease-in=EaseInOut");
    }

    #[test]
    fn window_app_parses_update_status_text_blink_rapid_ease_out_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.text_blink_rapid_ease_out = 'Constant'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('rapid-ease-out=' .. tostring(window:effective_config().text_blink_rapid_ease_out))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config text_blink_rapid_ease_out status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "rapid-ease-out=Constant");
    }

    #[test]
    fn window_app_parses_update_status_cursor_thickness_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.cursor_thickness = '25%'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('cursor-thickness=' .. tostring(window:effective_config().cursor_thickness))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config cursor_thickness status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "cursor-thickness=25%");
    }

    #[test]
    fn window_app_parses_update_status_underline_thickness_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.underline_thickness = '2px'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('underline-thickness=' .. tostring(window:effective_config().underline_thickness))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config underline_thickness status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "underline-thickness=2px");
    }

    #[test]
    fn window_app_parses_update_status_underline_position_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.underline_position = '-2px'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('underline-position=' .. tostring(window:effective_config().underline_position))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config underline_position status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "underline-position=-2px");
    }

    #[test]
    fn window_app_parses_update_status_strikethrough_position_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.strikethrough_position = '0.5cell'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('strike-position=' .. tostring(window:effective_config().strikethrough_position))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config strikethrough_position status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "strike-position=0.5cell");
    }

    #[test]
    fn window_app_parses_update_status_hide_mouse_cursor_when_typing_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.hide_mouse_cursor_when_typing = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('hide-mouse=' .. tostring(window:effective_config().hide_mouse_cursor_when_typing))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config hide_mouse_cursor_when_typing status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "hide-mouse=false");
    }

    #[test]
    fn window_app_parses_update_status_periodic_stat_logging_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.periodic_stat_logging = 15

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('stats=' .. tostring(window:effective_config().periodic_stat_logging))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config periodic_stat_logging status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "stats=15");
    }

    #[test]
    fn window_app_parses_update_status_default_mux_server_domain_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_mux_server_domain = 'mux-main'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('mux-domain=' .. tostring(window:effective_config().default_mux_server_domain))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config default_mux_server_domain status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "mux-domain=mux-main");
    }

    #[test]
    fn window_app_parses_update_status_default_prog_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = { 'nu', '--login' }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('prog=' .. tostring(window:effective_config().default_prog[1]))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config default_prog status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog=nu");
    }

    #[test]
    fn window_app_parses_update_status_effective_config_bracket_field_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = { 'nu', '--login' }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('prog=' .. tostring(window:effective_config()['default_prog'][1]))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config bracket field status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog=nu");
    }

    #[test]
    fn window_app_parses_update_status_effective_config_alias_static_bracket_key_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = { 'nu', '--login' }

            wezterm.on('update-status', function(window, pane)
              local effective = window:effective_config()
              local field = 'default_prog'
              window:set_right_status('prog=' .. tostring(effective[field][1]))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config alias static bracket key status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog=nu");
    }

    #[test]
    fn window_app_parses_update_status_effective_config_alias_top_level_static_bracket_status() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local field = 'default_prog'
            local prog_index = 1

            config.default_prog = { 'nu', '--login' }

            wezterm.on('update-status', function(window, pane)
              local effective = window:effective_config()
              window:set_right_status('prog=' .. tostring(effective[field][prog_index]))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config alias top-level static bracket status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog=nu");
    }

    #[test]
    fn window_app_parses_update_status_effective_config_direct_static_bracket_key_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = { 'nu', '--login' }

            wezterm.on('update-status', function(window, pane)
              local field = 'default_prog'
              window:set_right_status('prog=' .. tostring(window:effective_config()[field][1]))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config direct static bracket key status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog=nu");
    }

    #[test]
    fn window_app_parses_update_status_effective_config_static_array_index_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = { 'nu', '--login' }

            wezterm.on('update-status', function(window, pane)
              local prog_index = 1
              window:set_right_status('prog=' .. tostring(window:effective_config().default_prog[prog_index]))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config static array index status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog=nu");
    }

    #[test]
    fn window_app_parses_update_status_effective_config_top_level_static_array_index_status_setter()
    {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local prog_index = 1

            config.default_prog = { 'nu', '--login' }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('prog=' .. tostring(window:effective_config().default_prog[prog_index]))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config top-level static array index status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "prog=nu");
    }

    #[test]
    fn window_app_parses_update_status_default_gui_startup_args_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_gui_startup_args = { 'connect', 'prod' }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('startup=' .. tostring(window:effective_config().default_gui_startup_args[2]))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config default_gui_startup_args status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "startup=prod");
    }

    #[test]
    fn window_app_parses_update_status_default_cwd_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_cwd = 'C:/Project Dir'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('cwd=' .. tostring(window:effective_config().default_cwd))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config default_cwd status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "cwd=C:/Project Dir");
    }

    #[test]
    fn window_app_parses_update_status_default_domain_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_domain = 'ssh-prod'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('domain=' .. tostring(window:effective_config().default_domain))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config default_domain status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "domain=ssh-prod");
    }

    #[test]
    fn window_app_parses_update_status_prefer_to_spawn_tabs_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.prefer_to_spawn_tabs = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('prefer-tabs=' .. tostring(window:effective_config().prefer_to_spawn_tabs))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config prefer_to_spawn_tabs status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "prefer-tabs=true");
    }

    #[test]
    fn window_app_parses_update_status_ssh_backend_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.ssh_backend = 'Ssh2'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('ssh-backend=' .. tostring(window:effective_config().ssh_backend))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config ssh_backend status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "ssh-backend=Ssh2");
    }

    #[test]
    fn window_app_parses_update_status_ratelimit_mux_line_prefetches_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.ratelimit_mux_line_prefetches_per_second = 12

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('mux-prefetch=' .. tostring(window:effective_config().ratelimit_mux_line_prefetches_per_second))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config ratelimit mux prefetch status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "mux-prefetch=12");
    }

    #[test]
    fn window_app_parses_update_status_mux_output_parser_buffer_size_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.mux_output_parser_buffer_size = 4096

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('mux-buffer=' .. tostring(window:effective_config().mux_output_parser_buffer_size))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config mux parser buffer status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "mux-buffer=4096");
    }

    #[test]
    fn window_app_parses_update_status_mux_output_parser_coalesce_delay_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.mux_output_parser_coalesce_delay_ms = 7

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('mux-coalesce=' .. tostring(window:effective_config().mux_output_parser_coalesce_delay_ms))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config mux parser coalesce status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "mux-coalesce=7");
    }

    #[test]
    fn window_app_parses_update_status_ulimit_nofile_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.ulimit_nofile = 4096

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('nofile=' .. tostring(window:effective_config().ulimit_nofile))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config ulimit_nofile status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "nofile=4096");
    }

    #[test]
    fn window_app_parses_update_status_ulimit_nproc_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.ulimit_nproc = 8192

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('nproc=' .. tostring(window:effective_config().ulimit_nproc))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config ulimit_nproc status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "nproc=8192");
    }

    #[test]
    fn window_app_parses_update_status_mux_env_remove_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.mux_env_remove = { 'REMOVE_ME', 'REMOVE_TOO' }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('mux-env-remove=' .. tostring(window:effective_config().mux_env_remove[1]))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config mux_env_remove status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "mux-env-remove=REMOVE_ME");
    }

    #[test]
    fn window_app_parses_update_status_set_environment_variables_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.set_environment_variables = {
              PROJECT_MODE = 'dev',
            }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('env=' .. tostring(window:effective_config().set_environment_variables.PROJECT_MODE))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config set_environment_variables status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "env=dev");
    }

    #[test]
    fn window_app_parses_update_status_set_environment_variables_bracket_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.set_environment_variables = {
              PROJECT_MODE = 'dev',
            }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('env=' .. tostring(window:effective_config().set_environment_variables['PROJECT_MODE']))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config set_environment_variables bracket status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "env=dev");
    }

    #[test]
    fn window_app_parses_update_status_set_environment_variables_alias_static_bracket_status() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local env_key = 'PROJECT_MODE'

            config.set_environment_variables = {
              PROJECT_MODE = 'dev',
            }

            wezterm.on('update-status', function(window, pane)
              local env = window:effective_config().set_environment_variables
              window:set_right_status('env=' .. tostring(env[env_key]))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config set_environment_variables alias static bracket status");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "env=dev");
    }

    #[test]
    fn window_app_parses_update_status_scroll_to_bottom_on_input_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.scroll_to_bottom_on_input = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('scroll-input=' .. tostring(window:effective_config().scroll_to_bottom_on_input))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config scroll_to_bottom_on_input status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "scroll-input=false");
    }

    #[test]
    fn window_app_parses_update_status_use_ime_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.use_ime = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('ime=' .. tostring(window:effective_config().use_ime))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config use_ime status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "ime=false");
    }

    #[test]
    fn window_app_parses_update_status_xim_im_name_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.xim_im_name = 'fcitx'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('xim=' .. tostring(window:effective_config().xim_im_name))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config xim_im_name status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "xim=fcitx");
    }

    #[test]
    fn window_app_parses_update_status_ime_preedit_rendering_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.ime_preedit_rendering = 'System'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('preedit=' .. tostring(window:effective_config().ime_preedit_rendering))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config ime_preedit_rendering status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "preedit=System");
    }

    #[test]
    fn window_app_parses_update_status_macos_forward_to_ime_modifier_mask_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.macos_forward_to_ime_modifier_mask = 'SHIFT|CTRL'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('ime-mask=' .. tostring(window:effective_config().macos_forward_to_ime_modifier_mask))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config macos_forward_to_ime_modifier_mask status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "ime-mask=CTRL|SHIFT");
    }

    #[test]
    fn window_app_parses_update_status_notification_handling_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.notification_handling = 'SuppressFromFocusedWindow'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('notify=' .. tostring(window:effective_config().notification_handling))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config notification_handling status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "notify=SuppressFromFocusedWindow");
    }

    #[test]
    fn window_app_parses_update_status_color_scheme_dirs_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.color_scheme_dirs = { 'schemes', '/opt/wezterm/colors' }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('scheme-dir=' .. tostring(window:effective_config().color_scheme_dirs[2]))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config color_scheme_dirs status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "scheme-dir=/opt/wezterm/colors");
    }

    #[test]
    fn window_app_parses_update_status_color_scheme_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.color_scheme = 'Project Scheme'
            config.color_schemes = {
              ['Project Scheme'] = {
                foreground = '#101112',
                background = '#131415',
              },
            }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('scheme=' .. tostring(window:effective_config().color_scheme))
            end)

            return config
            "##,
        )
        .expect("expected WezTerm effective_config color_scheme status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "scheme=Project Scheme");
    }

    #[test]
    fn window_app_parses_update_status_effective_config_foreground_background_colors_status_setter()
    {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              foreground = '#010203',
              background = '#040506',
            }

            wezterm.on('update-status', function(window, pane)
              local config = window:effective_config()
              window:set_right_status(
                'fg=' .. tostring(config.foreground_color) ..
                ' bg=' .. tostring(config.background_color)
              )
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config foreground/background color status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "fg=#010203 bg=#040506");
    }

    #[test]
    fn window_app_parses_update_status_effective_config_resolved_palette_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              foreground = '#010203',
              background = '#040506',
              cursor_bg = '#070809',
            }

            wezterm.on('update-status', function(window, pane)
              local config = window:effective_config()
              window:set_right_status(
                'palette-fg=' .. tostring(config.resolved_palette.foreground) ..
                ' palette-bg=' .. tostring(config.resolved_palette.background) ..
                ' cursor=' .. tostring(config.resolved_palette.cursor_bg)
              )
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config resolved_palette status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(
            app.right_status,
            "palette-fg=#010203 palette-bg=#040506 cursor=#070809"
        );
    }

    #[test]
    fn window_app_parses_update_status_effective_config_resolved_palette_optional_color_status_setter()
     {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              cursor_fg = '#010203',
              cursor_border = '#040506',
              selection_fg = '#070809',
              selection_bg = '#0a0b0c',
              compose_cursor = '#0d0e0f',
              visual_bell = '#101112',
            }

            wezterm.on('update-status', function(window, pane)
              local palette = window:effective_config().resolved_palette
              window:set_right_status(
                'cursor-fg=' .. tostring(palette.cursor_fg) ..
                ' cursor-border=' .. tostring(palette.cursor_border) ..
                ' selection-fg=' .. tostring(palette.selection_fg) ..
                ' selection-bg=' .. tostring(palette.selection_bg) ..
                ' compose=' .. tostring(palette.compose_cursor) ..
                ' bell=' .. tostring(palette.visual_bell)
              )
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config resolved_palette optional colors status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(
            app.right_status,
            concat!(
                "cursor-fg=#010203 cursor-border=#040506 ",
                "selection-fg=#070809 selection-bg=#0a0b0c ",
                "compose=#0d0e0f bell=#101112"
            )
        );
    }

    #[test]
    fn window_app_parses_update_status_resolved_palette_indexed_color_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              ansi = {
                '#010203', '#040506', '#070809', '#0a0b0c',
                '#0d0e0f', '#101112', '#131415', '#161718',
              },
              brights = {
                '#191a1b', '#1c1d1e', '#1f2021', '#222324',
                '#252627', '#28292a', '#2b2c2d', '#2e2f30',
              },
              indexed = {
                [136] = '#313233',
              },
            }

            wezterm.on('update-status', function(window, pane)
              local palette = window:effective_config().resolved_palette
              window:set_right_status(
                'ansi=' .. tostring(palette.ansi[2]) ..
                ' bright=' .. tostring(palette.brights[3]) ..
                ' indexed=' .. tostring(palette.indexed[136])
              )
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config resolved_palette indexed colors status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(
            app.right_status,
            "ansi=#040506 bright=#1f2021 indexed=#313233"
        );
    }

    #[test]
    fn window_app_parses_update_status_use_dead_keys_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.use_dead_keys = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('dead-keys=' .. tostring(window:effective_config().use_dead_keys))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config use_dead_keys status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "dead-keys=false");
    }

    #[test]
    fn window_app_parses_update_status_audible_bell_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.audible_bell = 'Disabled'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('audible=' .. tostring(window:effective_config().audible_bell))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config audible_bell status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "audible=Disabled");
    }

    #[test]
    fn window_app_parses_update_status_visual_bell_target_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.visual_bell = {
              target = 'CursorColor',
            }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('bell-target=' .. tostring(window:effective_config().visual_bell.target))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config visual_bell.target status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "bell-target=CursorColor");
    }

    #[test]
    fn window_app_parses_update_status_visual_bell_bracket_field_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.visual_bell = {
              target = 'CursorColor',
            }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('bell-target=' .. tostring(window:effective_config().visual_bell['target']))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config visual_bell bracket target status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "bell-target=CursorColor");
    }

    #[test]
    fn window_app_parses_update_status_effective_config_nested_static_bracket_key_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.visual_bell = {
              target = 'CursorColor',
            }

            wezterm.on('update-status', function(window, pane)
              local bell_field = 'visual_bell'
              local target_field = 'target'
              window:set_right_status('bell-target=' .. tostring(window:effective_config()[bell_field][target_field]))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config nested static bracket key status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "bell-target=CursorColor");
    }

    #[test]
    fn window_app_parses_update_status_visual_bell_alias_top_level_static_bracket_status() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local target_field = 'target'

            config.visual_bell = {
              target = 'CursorColor',
            }

            wezterm.on('update-status', function(window, pane)
              local bell = window:effective_config().visual_bell
              window:set_right_status('bell-target=' .. tostring(bell[target_field]))
            end)

            return config
            "#,
        )
        .expect(
            "expected WezTerm effective_config visual_bell alias top-level static bracket status",
        );
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "bell-target=CursorColor");
    }

    #[test]
    fn window_app_parses_update_status_visual_bell_fields_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.visual_bell = {
              fade_in_duration_ms = 25,
              fade_out_duration_ms = 175,
              fade_in_function = 'EaseIn',
              fade_out_function = 'EaseOut',
              target = 'CursorColor',
            }

            wezterm.on('update-status', function(window, pane)
              local bell = window:effective_config().visual_bell
              window:set_right_status(
                'in=' .. tostring(bell.fade_in_duration_ms) ..
                ' out=' .. tostring(bell.fade_out_duration_ms) ..
                ' in-fn=' .. tostring(bell.fade_in_function) ..
                ' out-fn=' .. tostring(bell.fade_out_function) ..
                ' target=' .. tostring(bell.target)
              )
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config visual_bell fields status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(
            app.right_status,
            "in=25 out=175 in-fn=EaseIn out-fn=EaseOut target=CursorColor"
        );
    }

    #[test]
    fn window_app_parses_update_status_launch_menu_label_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.launch_menu = {
              {
                label = 'System Monitor',
                args = { 'top' },
              },
            }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('launch=' .. tostring(window:effective_config().launch_menu[1].label))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config launch_menu label status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "launch=System Monitor");
    }

    #[test]
    fn window_app_parses_update_status_launch_menu_command_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.launch_menu = {
              {
                label = 'System Monitor',
                args = { 'top', '-H' },
                cwd = '/tmp/project',
              },
            }

            wezterm.on('update-status', function(window, pane)
              local item = window:effective_config().launch_menu[1]
              window:set_right_status(
                'program=' .. tostring(item.args[1]) ..
                ' arg=' .. tostring(item.args[2]) ..
                ' cwd=' .. tostring(item.cwd)
              )
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config launch_menu command status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "program=top arg=-H cwd=/tmp/project");
    }

    #[test]
    fn window_app_parses_update_status_launch_menu_environment_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.launch_menu = {
              {
                label = 'Project Shell',
                args = { 'nu' },
                set_environment_variables = {
                  PROJECT_MODE = 'dev',
                  FEATURE_FLAG = 'on',
                },
              },
            }

            wezterm.on('update-status', function(window, pane)
              local env = window:effective_config().launch_menu[1].set_environment_variables
              window:set_right_status(
                'mode=' .. tostring(env.PROJECT_MODE) ..
                ' flag=' .. tostring(env['FEATURE_FLAG'])
              )
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config launch_menu environment status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "mode=dev flag=on");
    }

    #[test]
    fn window_app_parses_update_status_launch_menu_env_alias_static_index_status() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.launch_menu = {
              {
                label = 'Monitor',
                args = { 'top' },
                set_environment_variables = {
                  PROJECT_MODE = 'dev',
                },
              },
            }

            wezterm.on('update-status', function(window, pane)
              local item_index = 1
              local env_key = 'PROJECT_MODE'
              local env = window:effective_config().launch_menu[item_index].set_environment_variables
              window:set_right_status('env=' .. tostring(env[env_key]))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config launch_menu env alias static index status");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "env=dev");
    }

    #[test]
    fn window_app_parses_update_status_launch_menu_domain_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.launch_menu = {
              {
                label = 'Local Shell',
                args = { 'nu' },
                domain = 'local',
              },
            }

            wezterm.on('update-status', function(window, pane)
              local item = window:effective_config().launch_menu[1]
              window:set_right_status('domain=' .. tostring(item.domain))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config launch_menu domain status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "domain=local");
    }

    #[test]
    fn window_app_parses_update_status_launch_menu_default_program_status_setter() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.launch_menu = {
              {
                label = 'Project Shell',
                cwd = 'C:/Project Dir',
                set_environment_variables = {
                  PROJECT_MODE = 'dev',
                },
              },
            }

            wezterm.on('update-status', function(window, pane)
              local item = window:effective_config().launch_menu[1]
              window:set_right_status(
                'program=' .. tostring(item.args[1]) ..
                ' arg=' .. tostring(item.args[2]) ..
                ' cwd=' .. tostring(item.cwd) ..
                ' mode=' .. tostring(item.set_environment_variables.PROJECT_MODE)
              )
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config launch_menu default program status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(
            app.right_status,
            "program=powershell arg=-NoProfile cwd=C:/Project Dir mode=dev"
        );
    }

    #[test]
    fn window_app_parses_update_status_automatically_reload_config_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.automatically_reload_config = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('reload=' .. tostring(window:effective_config().automatically_reload_config))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config automatically_reload_config status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "reload=false");
    }

    #[test]
    fn window_app_parses_update_status_check_for_updates_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.check_for_updates = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('checks=' .. tostring(window:effective_config().check_for_updates))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config check_for_updates status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "checks=false");
    }

    #[test]
    fn window_app_parses_update_status_show_update_window_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.show_update_window = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('update-window=' .. tostring(window:effective_config().show_update_window))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config show_update_window status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "update-window=true");
    }

    #[test]
    fn window_app_parses_update_status_check_for_updates_interval_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.check_for_updates_interval_seconds = 43200

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('interval=' .. tostring(window:effective_config().check_for_updates_interval_seconds))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config check_for_updates_interval_seconds status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "interval=43200");
    }

    #[test]
    fn window_app_parses_update_status_enable_csi_u_key_encoding_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.enable_csi_u_key_encoding = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('csi-u=' .. tostring(window:effective_config().enable_csi_u_key_encoding))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config enable_csi_u_key_encoding status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "csi-u=true");
    }

    #[test]
    fn window_app_parses_update_status_enable_kitty_keyboard_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.enable_kitty_keyboard = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('kitty-keyboard=' .. tostring(window:effective_config().enable_kitty_keyboard))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config enable_kitty_keyboard status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "kitty-keyboard=true");
    }

    #[test]
    fn window_app_parses_update_status_enable_title_reporting_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.enable_title_reporting = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('title-reporting=' .. tostring(window:effective_config().enable_title_reporting))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config enable_title_reporting status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "title-reporting=true");
    }

    #[test]
    fn window_app_parses_update_status_enable_checksum_rectangular_area_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.enable_checksum_rectangular_area = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('checksum-rect=' .. tostring(window:effective_config().enable_checksum_rectangular_area))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config enable_checksum_rectangular_area status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "checksum-rect=true");
    }

    #[test]
    fn window_app_parses_update_status_enable_kitty_graphics_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.enable_kitty_graphics = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('kitty-graphics=' .. tostring(window:effective_config().enable_kitty_graphics))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config enable_kitty_graphics status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "kitty-graphics=false");
    }

    #[test]
    fn window_app_parses_update_status_allow_download_protocols_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.allow_download_protocols = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('downloads=' .. tostring(window:effective_config().allow_download_protocols))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config allow_download_protocols status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "downloads=false");
    }

    #[test]
    fn window_app_parses_update_status_xcursor_theme_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.xcursor_theme = 'Adwaita'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('xcursor=' .. window:effective_config().xcursor_theme)
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config xcursor_theme status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "xcursor=Adwaita");
    }

    #[test]
    fn window_app_parses_update_status_xcursor_size_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.xcursor_size = 24

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('xcursor-size=' .. tostring(window:effective_config().xcursor_size))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config xcursor_size status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "xcursor-size=24");
    }

    #[test]
    fn window_app_parses_update_status_palette_max_key_assigments_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.palette_max_key_assigments_for_action = 3

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('palette-max=' .. tostring(window:effective_config().palette_max_key_assigments_for_action))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config palette max key assigments status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "palette-max=3");
    }

    #[test]
    fn window_app_parses_update_status_allow_win32_input_mode_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.allow_win32_input_mode = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('win32-input=' .. tostring(window:effective_config().allow_win32_input_mode))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config allow_win32_input_mode status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "win32-input=false");
    }

    #[test]
    fn window_app_parses_update_status_treat_left_ctrlalt_as_altgr_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.treat_left_ctrlalt_as_altgr = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('altgr=' .. tostring(window:effective_config().treat_left_ctrlalt_as_altgr))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config treat_left_ctrlalt_as_altgr status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "altgr=true");
    }

    #[test]
    fn window_app_parses_update_status_send_composed_left_alt_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.send_composed_key_when_left_alt_is_pressed = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('left-alt-compose=' .. tostring(window:effective_config().send_composed_key_when_left_alt_is_pressed))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config left Alt composed-key status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "left-alt-compose=true");
    }

    #[test]
    fn window_app_parses_update_status_send_composed_right_alt_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.send_composed_key_when_right_alt_is_pressed = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('right-alt-compose=' .. tostring(window:effective_config().send_composed_key_when_right_alt_is_pressed))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config right Alt composed-key status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "right-alt-compose=false");
    }

    #[test]
    fn window_app_parses_update_status_east_asian_ambiguous_width_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.treat_east_asian_ambiguous_width_as_wide = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('wide=' .. tostring(window:effective_config().treat_east_asian_ambiguous_width_as_wide))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config east Asian ambiguous-width status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "wide=true");
    }

    #[test]
    fn window_app_parses_update_status_normalize_output_nfc_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.normalize_output_to_unicode_nfc = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('nfc=' .. tostring(window:effective_config().normalize_output_to_unicode_nfc))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config normalize-output status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "nfc=true");
    }

    #[test]
    fn window_app_parses_update_status_unicode_version_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.unicode_version = 14

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('unicode=' .. tostring(window:effective_config().unicode_version))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config unicode_version status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "unicode=14");
    }

    #[test]
    fn window_app_parses_update_status_window_close_confirmation_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_close_confirmation = 'NeverPrompt'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('close=' .. tostring(window:effective_config().window_close_confirmation))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config window_close_confirmation status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "close=NeverPrompt");
    }

