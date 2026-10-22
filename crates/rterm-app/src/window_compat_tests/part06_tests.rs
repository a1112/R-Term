    #[test]
    fn window_app_parses_update_status_native_macos_fullscreen_mode_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.native_macos_fullscreen_mode = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('macos-fullscreen=' .. tostring(window:effective_config().native_macos_fullscreen_mode))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config native_macos_fullscreen_mode status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "macos-fullscreen=true");
    }

    #[test]
    fn window_app_parses_update_status_macos_fullscreen_notch_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.macos_fullscreen_extend_behind_notch = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('notch=' .. tostring(window:effective_config().macos_fullscreen_extend_behind_notch))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config macos_fullscreen_extend_behind_notch status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "notch=true");
    }

    #[test]
    fn window_app_parses_update_status_selection_word_boundary_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.selection_word_boundary = ' <>[]{}'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('boundary=' .. tostring(window:effective_config().selection_word_boundary))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config selection_word_boundary status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "boundary= <>[]{}");
    }

    #[test]
    fn window_app_parses_update_status_enq_answerback_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.enq_answerback = 'rterm'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('enq=' .. tostring(window:effective_config().enq_answerback))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config enq_answerback status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "enq=rterm");
    }

    #[test]
    fn window_app_parses_update_status_term_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.term = 'wezterm-direct'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('term=' .. tostring(window:effective_config().term))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config term status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "term=wezterm-direct");
    }

    #[test]
    fn window_app_parses_update_status_initial_cols_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.initial_cols = 100

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('cols=' .. tostring(window:effective_config().initial_cols))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config initial_cols status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "cols=100");
    }

    #[test]
    fn window_app_parses_update_status_initial_rows_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.initial_rows = 30

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('rows=' .. tostring(window:effective_config().initial_rows))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config initial_rows status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "rows=30");
    }

    #[test]
    fn window_app_parses_update_status_scrollback_lines_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.scrollback_lines = 12345

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('scrollback=' .. tostring(window:effective_config().scrollback_lines))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config scrollback_lines status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "scrollback=12345");
    }

    #[test]
    fn window_app_parses_update_status_switch_to_last_active_tab_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.switch_to_last_active_tab_when_closing_tab = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('last-tab=' .. tostring(window:effective_config().switch_to_last_active_tab_when_closing_tab))
            end)

            return config
            "#,
        )
        .expect(
            "expected WezTerm effective_config switch_to_last_active_tab_when_closing_tab status setter",
        );
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "last-tab=true");
    }

    #[test]
    fn window_app_parses_update_status_exit_behavior_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.exit_behavior = 'CloseOnCleanExit'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('exit=' .. tostring(window:effective_config().exit_behavior))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config exit_behavior status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "exit=CloseOnCleanExit");
    }

    #[test]
    fn window_app_parses_update_status_exit_behavior_messaging_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.exit_behavior_messaging = 'Brief'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('exit-msg=' .. tostring(window:effective_config().exit_behavior_messaging))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config exit_behavior_messaging status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "exit-msg=Brief");
    }

    #[test]
    fn window_app_parses_update_status_clean_exit_codes_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.clean_exit_codes = { 130, 143 }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('clean-exit=' .. tostring(window:effective_config().clean_exit_codes[2]))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config clean_exit_codes status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "clean-exit=143");
    }

    #[test]
    fn window_app_parses_update_status_adjust_window_size_when_changing_font_size_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.adjust_window_size_when_changing_font_size = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('adjust=' .. tostring(window:effective_config().adjust_window_size_when_changing_font_size))
            end)

            return config
            "#,
        )
        .expect(
            "expected WezTerm effective_config adjust_window_size_when_changing_font_size status setter",
        );
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "adjust=false");
    }

    #[test]
    fn window_app_parses_update_status_tiling_desktop_environment_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.tiling_desktop_environments = { 'X11 i3', 'Wayland Sway' }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('tiling=' .. tostring(window:effective_config().tiling_desktop_environments[1]))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config tiling_desktop_environments status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "tiling=X11 i3");
    }

    #[test]
    fn window_app_parses_update_status_use_resize_increments_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.use_resize_increments = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('resize-increments=' .. tostring(window:effective_config().use_resize_increments))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config use_resize_increments status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "resize-increments=true");
    }

    #[test]
    fn window_app_parses_update_status_alternate_buffer_wheel_scroll_speed_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.alternate_buffer_wheel_scroll_speed = 2

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('alt-wheel=' .. tostring(window:effective_config().alternate_buffer_wheel_scroll_speed))
            end)

            return config
            "#,
        )
        .expect(
            "expected WezTerm effective_config alternate_buffer_wheel_scroll_speed status setter",
        );
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "alt-wheel=2");
    }

    #[test]
    fn window_app_parses_update_status_ignore_svg_fonts_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.ignore_svg_fonts = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('ignore-svg=' .. tostring(window:effective_config().ignore_svg_fonts))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config ignore_svg_fonts status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "ignore-svg=true");
    }

    #[test]
    fn window_app_parses_update_status_bidi_enabled_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.bidi_enabled = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('bidi=' .. tostring(window:effective_config().bidi_enabled))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config bidi_enabled status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "bidi=true");
    }

    #[test]
    fn window_app_parses_update_status_bidi_direction_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.bidi_direction = 'AutoRightToLeft'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('bidi-dir=' .. tostring(window:effective_config().bidi_direction))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config bidi_direction status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "bidi-dir=AutoRightToLeft");
    }

    #[test]
    fn window_app_parses_update_status_skip_close_confirmation_process_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.skip_close_confirmation_for_processes_named = { 'top', 'cmd.exe' }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('skip-close=' .. window:effective_config().skip_close_confirmation_for_processes_named[1])
            end)

            return config
            "#,
        )
        .expect(
            "expected WezTerm effective_config skip_close_confirmation_for_processes_named status setter",
        );
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "skip-close=top");
    }

    #[test]
    fn window_app_parses_update_status_enable_tab_bar_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.enable_tab_bar = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('tabbar=' .. tostring(window:effective_config().enable_tab_bar))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config enable_tab_bar status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "tabbar=false");
    }

    #[test]
    fn window_app_parses_update_status_use_fancy_tab_bar_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.use_fancy_tab_bar = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('fancy-tab=' .. tostring(window:effective_config().use_fancy_tab_bar))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config use_fancy_tab_bar status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "fancy-tab=false");
    }

    #[test]
    fn window_app_parses_update_status_tab_bar_at_bottom_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.tab_bar_at_bottom = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('tab-bottom=' .. tostring(window:effective_config().tab_bar_at_bottom))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config tab_bar_at_bottom status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "tab-bottom=true");
    }

    #[test]
    fn window_app_parses_update_status_mouse_wheel_scrolls_tabs_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.mouse_wheel_scrolls_tabs = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('wheel-tabs=' .. tostring(window:effective_config().mouse_wheel_scrolls_tabs))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config mouse_wheel_scrolls_tabs status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "wheel-tabs=false");
    }

    #[test]
    fn window_app_parses_update_status_warn_missing_glyphs_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.warn_about_missing_glyphs = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('glyph-warn=' .. tostring(window:effective_config().warn_about_missing_glyphs))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config warn_about_missing_glyphs status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "glyph-warn=false");
    }

    #[test]
    fn window_app_parses_update_status_pane_focus_follows_mouse_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.pane_focus_follows_mouse = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('focus-follows=' .. tostring(window:effective_config().pane_focus_follows_mouse))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config pane_focus_follows_mouse status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "focus-follows=true");
    }

    #[test]
    fn window_app_parses_update_status_swallow_pane_focus_click_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.swallow_mouse_click_on_pane_focus = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('swallow-pane=' .. tostring(window:effective_config().swallow_mouse_click_on_pane_focus))
            end)

            return config
            "#,
        )
        .expect(
            "expected WezTerm effective_config swallow_mouse_click_on_pane_focus status setter",
        );
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "swallow-pane=true");
    }

    #[test]
    fn window_app_parses_update_status_swallow_window_focus_click_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.swallow_mouse_click_on_window_focus = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('swallow-window=' .. tostring(window:effective_config().swallow_mouse_click_on_window_focus))
            end)

            return config
            "#,
        )
        .expect(
            "expected WezTerm effective_config swallow_mouse_click_on_window_focus status setter",
        );
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "swallow-window=true");
    }

    #[test]
    fn window_app_parses_update_status_bypass_mouse_reporting_modifiers_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.bypass_mouse_reporting_modifiers = 'ALT|SHIFT'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('bypass=' .. tostring(window:effective_config().bypass_mouse_reporting_modifiers))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config bypass_mouse_reporting_modifiers status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "bypass=SHIFT|ALT");
    }

    #[test]
    fn window_app_parses_update_status_unzoom_on_switch_pane_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.unzoom_on_switch_pane = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('unzoom=' .. tostring(window:effective_config().unzoom_on_switch_pane))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config unzoom_on_switch_pane status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "unzoom=false");
    }

    #[test]
    fn window_app_parses_update_status_quit_when_all_windows_closed_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.quit_when_all_windows_are_closed = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('quit-all=' .. tostring(window:effective_config().quit_when_all_windows_are_closed))
            end)

            return config
            "#,
        )
        .expect(
            "expected WezTerm effective_config quit_when_all_windows_are_closed status setter",
        );
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "quit-all=false");
    }

    #[test]
    fn window_app_parses_update_status_default_cursor_style_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_cursor_style = 'BlinkingBar'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('cursor=' .. tostring(window:effective_config().default_cursor_style))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config default_cursor_style status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "cursor=BlinkingBar");
    }

    #[test]
    fn window_app_parses_update_status_force_reverse_video_cursor_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.force_reverse_video_cursor = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('reverse-cursor=' .. tostring(window:effective_config().force_reverse_video_cursor))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config force_reverse_video_cursor status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "reverse-cursor=true");
    }

    #[test]
    fn window_app_parses_update_status_reverse_video_cursor_min_contrast_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.reverse_video_cursor_min_contrast = 3.25

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('reverse-contrast=' .. tostring(window:effective_config().reverse_video_cursor_min_contrast))
            end)

            return config
            "#,
        )
        .expect(
            "expected WezTerm effective_config reverse_video_cursor_min_contrast status setter",
        );
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "reverse-contrast=3.25");
    }

    #[test]
    fn window_app_parses_update_status_text_min_contrast_ratio_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.text_min_contrast_ratio = 4.5

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('text-contrast=' .. tostring(window:effective_config().text_min_contrast_ratio))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config text_min_contrast_ratio status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "text-contrast=4.5");
    }

    #[test]
    fn window_app_parses_update_status_command_palette_rows_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.command_palette_rows = 3

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('palette-rows=' .. tostring(window:effective_config().command_palette_rows))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config command_palette_rows status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "palette-rows=3");
    }

    #[test]
    fn window_app_parses_update_status_command_palette_font_size_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.command_palette_font_size = 15.5

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('palette-font=' .. tostring(window:effective_config().command_palette_font_size))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config command_palette_font_size status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "palette-font=15.5");
    }

    #[test]
    fn window_app_parses_update_status_overlay_color_status_setters() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.command_palette_bg_color = '#010203'
            config.command_palette_fg_color = '#040506'
            config.char_select_bg_color = '#070809'
            config.char_select_fg_color = '#0a0b0c'
            config.pane_select_bg_color = '#0d0e0f'
            config.pane_select_fg_color = '#101112'

            wezterm.on('update-status', function(window, pane)
              local config = window:effective_config()
              window:set_right_status(
                'palette=' .. tostring(config.command_palette_bg_color) ..
                '/' .. tostring(config.command_palette_fg_color) ..
                ' char=' .. tostring(config.char_select_bg_color) ..
                '/' .. tostring(config.char_select_fg_color) ..
                ' pane=' .. tostring(config.pane_select_bg_color) ..
                '/' .. tostring(config.pane_select_fg_color)
              )
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config overlay color status setters");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(
            app.right_status,
            "palette=#010203/#040506 char=#070809/#0a0b0c pane=#0d0e0f/#101112"
        );
    }

    #[test]
    fn window_app_parses_update_status_char_select_font_size_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.char_select_font_size = 16.25

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('char-font=' .. tostring(window:effective_config().char_select_font_size))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config char_select_font_size status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "char-font=16.25");
    }

    #[test]
    fn window_app_parses_update_status_pane_select_font_size_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.pane_select_font_size = 36.5

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('pane-font=' .. tostring(window:effective_config().pane_select_font_size))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config pane_select_font_size status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "pane-font=36.5");
    }

    #[test]
    fn window_app_parses_update_status_launcher_alphabet_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.launcher_alphabet = 'ab'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('launcher-alpha=' .. tostring(window:effective_config().launcher_alphabet))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config launcher_alphabet status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "launcher-alpha=ab");
    }

    #[test]
    fn window_app_parses_update_status_canonicalize_pasted_newlines_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.canonicalize_pasted_newlines = 'CarriageReturnAndLineFeed'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('paste-newlines=' .. tostring(window:effective_config().canonicalize_pasted_newlines))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config canonicalize_pasted_newlines status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "paste-newlines=CarriageReturnAndLineFeed");
    }

    #[test]
    fn window_app_parses_update_status_quote_dropped_files_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.quote_dropped_files = 'Posix'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('quote-drop=' .. tostring(window:effective_config().quote_dropped_files))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config quote_dropped_files status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "quote-drop=Posix");
    }

    #[test]
    fn window_app_parses_update_status_disable_default_key_bindings_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.disable_default_key_bindings = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('disable-keys=' .. tostring(window:effective_config().disable_default_key_bindings))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config disable_default_key_bindings status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "disable-keys=true");
    }

    #[test]
    fn window_app_parses_update_status_disable_default_mouse_bindings_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.disable_default_mouse_bindings = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('disable-mouse=' .. tostring(window:effective_config().disable_default_mouse_bindings))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config disable_default_mouse_bindings status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "disable-mouse=true");
    }

    #[test]
    fn window_app_parses_update_status_debug_key_events_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.debug_key_events = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('debug-keys=' .. tostring(window:effective_config().debug_key_events))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config debug_key_events status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "debug-keys=true");
    }

    #[test]
    fn window_app_parses_update_status_key_map_preference_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.key_map_preference = 'Physical'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('key-map=' .. tostring(window:effective_config().key_map_preference))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config key_map_preference status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "key-map=Physical");
    }

    #[test]
    fn window_app_parses_update_status_ui_key_cap_rendering_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.ui_key_cap_rendering = 'Emacs'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('key-caps=' .. tostring(window:effective_config().ui_key_cap_rendering))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config ui_key_cap_rendering status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "key-caps=Emacs");
    }

    #[test]
    fn window_app_parses_update_status_swap_backspace_and_delete_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.swap_backspace_and_delete = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('swap-bs-del=' .. tostring(window:effective_config().swap_backspace_and_delete))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config swap_backspace_and_delete status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "swap-bs-del=true");
    }

    #[test]
    fn window_app_parses_update_status_log_unknown_escape_sequences_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.log_unknown_escape_sequences = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('unknown-esc=' .. tostring(window:effective_config().log_unknown_escape_sequences))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config log_unknown_escape_sequences status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "unknown-esc=true");
    }

    #[test]
    fn window_app_parses_update_status_default_ssh_auth_sock_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_ssh_auth_sock = '/tmp/wezterm-agent.sock'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('ssh-auth=' .. tostring(window:effective_config().default_ssh_auth_sock))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config default_ssh_auth_sock status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "ssh-auth=/tmp/wezterm-agent.sock");
    }

    #[test]
    fn window_app_parses_update_status_mux_enable_ssh_agent_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.mux_enable_ssh_agent = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('mux-agent=' .. tostring(window:effective_config().mux_enable_ssh_agent))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config mux_enable_ssh_agent status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "mux-agent=false");
    }

    #[test]
    fn window_app_parses_update_status_detect_password_input_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.detect_password_input = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('detect-password=' .. tostring(window:effective_config().detect_password_input))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config detect_password_input status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "detect-password=false");
    }

    #[test]
    fn window_app_parses_update_status_quick_select_alphabet_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.quick_select_alphabet = 'ab'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('quick-alpha=' .. tostring(window:effective_config().quick_select_alphabet))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config quick_select_alphabet status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "quick-alpha=ab");
    }

    #[test]
    fn window_app_parses_update_status_quick_select_patterns_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.quick_select_patterns = { 'ticket-[0-9]+', 'bug-[A-Z]+' }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('quick-pattern=' .. tostring(window:effective_config().quick_select_patterns[2]))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config quick_select_patterns status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "quick-pattern=bug-[A-Z]+");
    }

    #[test]
    fn window_app_parses_update_status_disable_default_quick_select_patterns_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.disable_default_quick_select_patterns = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('quick-defaults=' .. tostring(window:effective_config().disable_default_quick_select_patterns))
            end)

            return config
            "#,
        )
        .expect(
            "expected WezTerm effective_config disable_default_quick_select_patterns status setter",
        );
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "quick-defaults=true");
    }

    #[test]
    fn window_app_parses_update_status_quick_select_remove_styling_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.quick_select_remove_styling = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('quick-style=' .. tostring(window:effective_config().quick_select_remove_styling))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config quick_select_remove_styling status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "quick-style=true");
    }

    #[test]
    fn window_app_parses_update_status_show_close_tab_button_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.show_close_tab_button_in_tabs = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('tab-close=' .. tostring(window:effective_config().show_close_tab_button_in_tabs))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config show_close_tab_button_in_tabs status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "tab-close=false");
    }

    #[test]
    fn window_app_parses_update_status_show_new_tab_button_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.show_new_tab_button_in_tab_bar = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('new-tab=' .. tostring(window:effective_config().show_new_tab_button_in_tab_bar))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config show_new_tab_button_in_tab_bar status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "new-tab=false");
    }

    #[test]
    fn window_app_parses_update_status_show_tab_index_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.show_tab_index_in_tab_bar = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('tab-index=' .. tostring(window:effective_config().show_tab_index_in_tab_bar))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config show_tab_index_in_tab_bar status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "tab-index=false");
    }

    #[test]
    fn window_app_parses_update_status_show_tabs_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.show_tabs_in_tab_bar = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('tabs=' .. tostring(window:effective_config().show_tabs_in_tab_bar))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config show_tabs_in_tab_bar status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "tabs=false");
    }

    #[test]
    fn window_app_parses_update_status_zero_based_tab_index_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.tab_and_split_indices_are_zero_based = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('zero-based=' .. tostring(window:effective_config().tab_and_split_indices_are_zero_based))
            end)

            return config
            "#,
        )
        .expect(
            "expected WezTerm effective_config tab_and_split_indices_are_zero_based status setter",
        );
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "zero-based=true");
    }

    #[test]
    fn window_app_parses_update_status_hide_tab_bar_if_only_one_tab_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.hide_tab_bar_if_only_one_tab = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('hide-tab=' .. tostring(window:effective_config().hide_tab_bar_if_only_one_tab))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config hide_tab_bar_if_only_one_tab status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "hide-tab=true");
    }

    #[test]
    fn window_app_parses_update_status_enable_scroll_bar_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.enable_scroll_bar = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('scrollbar=' .. tostring(window:effective_config().enable_scroll_bar))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config enable_scroll_bar status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "scrollbar=true");
    }

    #[test]
    fn window_app_parses_update_status_min_scroll_bar_height_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.min_scroll_bar_height = '2cell'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('scrollbar-min=' .. tostring(window:effective_config().min_scroll_bar_height))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config min_scroll_bar_height status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "scrollbar-min=2cell");
    }

    #[test]
    fn window_app_parses_update_status_custom_block_glyphs_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.custom_block_glyphs = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('blocks=' .. tostring(window:effective_config().custom_block_glyphs))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config custom_block_glyphs status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "blocks=false");
    }

    #[test]
    fn window_app_parses_update_status_anti_alias_custom_block_glyphs_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.anti_alias_custom_block_glyphs = false

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('aa-blocks=' .. tostring(window:effective_config().anti_alias_custom_block_glyphs))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config anti_alias_custom_block_glyphs status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "aa-blocks=false");
    }

    #[test]
    fn window_app_parses_update_status_window_padding_left_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_padding = {
              left = 8,
              right = 16,
              top = '1cell',
              bottom = '2pt',
            }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('padding-left=' .. tostring(window:effective_config().window_padding.left))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config window_padding.left status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "padding-left=8px");
    }

    #[test]
    fn window_app_parses_update_status_window_padding_right_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_padding = {
              left = 8,
              right = 16,
              top = '1cell',
              bottom = '2pt',
            }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('padding-right=' .. tostring(window:effective_config().window_padding.right))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config window_padding.right status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "padding-right=16px");
    }

    #[test]
    fn window_app_parses_update_status_window_padding_top_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_padding = {
              left = 8,
              right = 16,
              top = '1cell',
              bottom = '2pt',
            }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('padding-top=' .. tostring(window:effective_config().window_padding.top))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config window_padding.top status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "padding-top=1cell");
    }

    #[test]
    fn window_app_parses_update_status_window_padding_bottom_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_padding = {
              left = 8,
              right = 16,
              top = '1cell',
              bottom = '2pt',
            }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('padding-bottom=' .. tostring(window:effective_config().window_padding.bottom))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config window_padding.bottom status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "padding-bottom=2pt");
    }

    #[test]
    fn window_app_parses_update_status_window_content_alignment_horizontal_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_content_alignment = {
              horizontal = 'Right',
              vertical = 'Bottom',
            }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('align-h=' .. tostring(window:effective_config().window_content_alignment.horizontal))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config window_content_alignment.horizontal status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "align-h=Right");
    }

    #[test]
    fn window_app_parses_update_status_window_content_alignment_vertical_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_content_alignment = {
              horizontal = 'Right',
              vertical = 'Bottom',
            }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('align-v=' .. tostring(window:effective_config().window_content_alignment.vertical))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config window_content_alignment.vertical status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "align-v=Bottom");
    }

    #[test]
    fn window_app_parses_update_status_kde_window_background_blur_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.kde_window_background_blur = true

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('kde-blur=' .. tostring(window:effective_config().kde_window_background_blur))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config kde_window_background_blur status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "kde-blur=true");
    }

    #[test]
    fn window_app_parses_update_status_macos_window_background_blur_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.macos_window_background_blur = 20

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('macos-blur=' .. tostring(window:effective_config().macos_window_background_blur))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config macos_window_background_blur status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "macos-blur=20");
    }

    #[test]
    fn window_app_parses_update_status_win32_system_backdrop_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.win32_system_backdrop = 'Mica'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('backdrop=' .. tostring(window:effective_config().win32_system_backdrop))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config win32_system_backdrop status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "backdrop=Mica");
    }

    #[test]
    fn window_app_parses_update_status_win32_acrylic_accent_color_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.win32_acrylic_accent_color = '#112233'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('accent=' .. tostring(window:effective_config().win32_acrylic_accent_color))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config win32_acrylic_accent_color status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "accent=#112233");
    }

    #[test]
    fn window_app_parses_update_status_window_decorations_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_decorations = 'INTEGRATED_BUTTONS|RESIZE'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('decorations=' .. tostring(window:effective_config().window_decorations))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config window_decorations status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "decorations=RESIZE|INTEGRATED_BUTTONS");
    }

    #[test]
    fn window_app_parses_update_status_integrated_title_button_alignment_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.integrated_title_button_alignment = 'Left'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('button-align=' .. tostring(window:effective_config().integrated_title_button_alignment))
            end)

            return config
            "#,
        )
        .expect(
            "expected WezTerm effective_config integrated_title_button_alignment status setter",
        );
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "button-align=Left");
    }

    #[test]
    fn window_app_parses_update_status_integrated_title_buttons_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.integrated_title_buttons = { 'Close', 'Hide', 'Maximize' }

            wezterm.on('update-status', function(window, pane)
              window:set_right_status(
                'buttons=' .. tostring(window:effective_config().integrated_title_buttons[1]) ..
                '/' .. tostring(window:effective_config().integrated_title_buttons[2]) ..
                '/' .. tostring(window:effective_config().integrated_title_buttons[3])
              )
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config integrated_title_buttons status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "buttons=Close/Hide/Maximize");
    }

    #[test]
    fn window_app_parses_update_status_integrated_title_button_color_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.integrated_title_button_color = '#010203'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('button-color=' .. tostring(window:effective_config().integrated_title_button_color))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config integrated_title_button_color status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "button-color=#010203");
    }

    #[test]
    fn window_app_parses_update_status_integrated_title_button_style_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.integrated_title_button_style = 'Gnome'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status('button-style=' .. tostring(window:effective_config().integrated_title_button_style))
            end)

            return config
            "#,
        )
        .expect("expected WezTerm effective_config integrated_title_button_style status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "button-style=Gnome");
    }

    #[test]
    fn window_app_parses_documented_wezterm_update_status_keyboard_modifiers_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            local config = wezterm.config_builder()

            config.debug_key_events = true
            wezterm.on('update-status', function(window, pane)
              local mods, leds = window:keyboard_modifiers()
              window:set_right_status('mods=' .. mods .. ' leds=' .. leds)
            end)

            return config
            "#,
        )
        .expect("expected documented WezTerm keyboard modifiers status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "mods= leds=");

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.dispatch_update_status();
        assert_eq!(app.right_status, "mods=CTRL|SHIFT leds=");
    }

    #[test]
    fn window_app_parses_documented_wezterm_update_right_status_composition_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-right-status', function(window, pane)
              local compose = window:composition_status()
              if compose then
                compose = 'COMPOSING: ' .. compose
              end
              window:set_right_status(compose or '')
            end)
            "#,
        )
        .expect("expected documented WezTerm composition status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();
        assert_eq!(app.right_status, "");

        app.handle_ime_preedit("kan");
        app.dispatch_update_status();
        assert_eq!(app.right_status, "COMPOSING: kan");

        app.handle_ime_preedit("");
        app.handle_keyboard_input_event(
            &Key::Dead(Some('^')),
            PhysicalKey::Code(WinitKeyCode::Quote),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "COMPOSING: ^");

        app.handle_keyboard_input_event(
            &Key::Character("e".into()),
            PhysicalKey::Code(WinitKeyCode::KeyE),
            Some("ê"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        app.dispatch_update_status();
        assert_eq!(app.right_status, "");
    }

    #[test]
    fn window_app_parses_static_wezterm_update_status_event_wezterm_format_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status(wezterm.format({
                { Text = 'RIGHT' },
                { Text = '-FORMAT' },
              }))
            end)
            "#,
        )
        .expect("expected static WezTerm update-status event wezterm.format status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.ends_with("RIGHT-FORMAT"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_update_status_event_format_alias_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local fmt = wezterm.format

            wezterm.on('update-status', function(window, pane)
              window:set_right_status(fmt({
                { Text = 'RIGHT' },
                { Text = '-ALIAS' },
              }))
            end)
            "#,
        )
        .expect("expected static WezTerm update-status event format alias status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.ends_with("RIGHT-ALIAS"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_update_status_event_format_alias_static_key_module() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wt = require 'wezterm'
            local format_key = 'format'
            local fmt = wt[format_key]

            wt.on('update-status', function(window, pane)
              window:set_right_status(fmt({
                { Text = 'RIGHT' },
                { Text = '-STATIC-KEY-FORMAT' },
              }))
            end)
            "#,
        )
        .expect(
            "expected static WezTerm update-status event static-key format alias status setter",
        );
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.ends_with("RIGHT-STATIC-KEY-FORMAT"),
            "tab bar was {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_update_status_event_styled_wezterm_format_status_setter() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              window:set_right_status(wezterm.format({
                { Foreground = { Color = '#010203' } },
                { Background = { Color = '#040506' } },
                { Attribute = { Intensity = 'Bold' } },
                { Attribute = { Italic = true } },
                { Attribute = { Underline = 'Curly' } },
                { Text = 'RIGHT-FORMAT' },
                'ResetAttributes',
                { Text = 'PLAIN' },
              }))
            end)
            "##,
        )
        .expect("expected static WezTerm update-status event styled wezterm.format status setter");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let start_column = tab_bar
            .find("RIGHT-FORMATPLAIN")
            .expect("styled format status should render without escape bytes");
        let styled_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(start_column).unwrap()).unwrap();
        let plain_cell = snapshot_cell(
            &snapshot,
            0,
            u16::try_from(start_column + "RIGHT-FORMAT".len()).unwrap(),
        )
        .unwrap();

        assert_eq!(styled_cell.ch, 'R');
        assert_eq!(styled_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_eq!(styled_cell.background, rterm_terminal::Color::Rgb(4, 5, 6));
        assert!(styled_cell.bold);
        assert!(styled_cell.italic);
        assert_eq!(
            styled_cell.underline_style,
            rterm_terminal::UnderlineStyle::Curly
        );
        assert_eq!(plain_cell.ch, 'P');
        assert_ne!(plain_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_ne!(plain_cell.background, rterm_terminal::Color::Rgb(4, 5, 6));
        assert!(!plain_cell.bold);
        assert!(!plain_cell.italic);
        assert_eq!(
            plain_cell.underline_style,
            rterm_terminal::UnderlineStyle::None
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_update_status_event_local_format_status_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local right = wezterm.format({
                { Foreground = { Color = '#010203' } },
                { Text = 'RIGHT' },
                'ResetAttributes',
                { Text = '-LOCAL' },
              })
              window:set_right_status(right)
            end)
            "##,
        )
        .expect("expected static WezTerm update-status event local format status variable");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let start_column = tab_bar
            .find("RIGHT-LOCAL")
            .expect("local format status variable should render without escape bytes");
        let styled_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(start_column).unwrap()).unwrap();
        let plain_cell = snapshot_cell(
            &snapshot,
            0,
            u16::try_from(start_column + "RIGHT".len()).unwrap(),
        )
        .unwrap();

        assert_eq!(styled_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_ne!(plain_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
    }

    #[test]
    fn window_app_parses_static_wezterm_update_status_event_top_level_format_status_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local right = wezterm.format({
              { Foreground = { Color = '#010203' } },
              { Text = 'RIGHT' },
              'ResetAttributes',
              { Text = '-TOP' },
            })

            wezterm.on('update-status', function(window, pane)
              window:set_right_status(right)
            end)
            "##,
        )
        .expect("expected static WezTerm update-status event top-level format status variable");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let start_column = tab_bar
            .find("RIGHT-TOP")
            .expect("top-level format status variable should render without escape bytes");
        let styled_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(start_column).unwrap()).unwrap();
        let plain_cell = snapshot_cell(
            &snapshot,
            0,
            u16::try_from(start_column + "RIGHT".len()).unwrap(),
        )
        .unwrap();

        assert_eq!(styled_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_ne!(plain_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
    }

    #[test]
    fn window_app_parses_static_wezterm_update_status_event_local_format_items_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local items = {
                { Foreground = { Color = '#010203' } },
                { Text = 'RIGHT' },
                'ResetAttributes',
                { Text = '-ITEMS' },
              }
              window:set_right_status(wezterm.format(items))
            end)
            "##,
        )
        .expect("expected static WezTerm update-status event local format items variable");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let start_column = tab_bar
            .find("RIGHT-ITEMS")
            .expect("local format items status variable should render without escape bytes");
        let styled_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(start_column).unwrap()).unwrap();
        let plain_cell = snapshot_cell(
            &snapshot,
            0,
            u16::try_from(start_column + "RIGHT".len()).unwrap(),
        )
        .unwrap();

        assert_eq!(styled_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_ne!(plain_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
    }

    #[test]
    fn window_app_parses_static_wezterm_update_status_event_local_format_items_table_insert() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local elements = {}
              table.insert(elements, { Foreground = { Color = '#010203' } })
              table.insert(elements, { Text = 'RIGHT' })
              table.insert(elements, 'ResetAttributes')
              table.insert(elements, { Text = '-INSERT' })
              window:set_right_status(wezterm.format(elements))
            end)
            "##,
        )
        .expect("expected static WezTerm update-status local format item table.insert config");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let start_column = tab_bar
            .find("RIGHT-INSERT")
            .expect("local format item table.insert status should render without escape bytes");
        let styled_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(start_column).unwrap()).unwrap();
        let plain_cell = snapshot_cell(
            &snapshot,
            0,
            u16::try_from(start_column + "RIGHT".len()).unwrap(),
        )
        .unwrap();

        assert_eq!(styled_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_ne!(plain_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
    }

    #[test]
    fn window_app_parses_static_wezterm_update_status_event_local_format_items_table_insert_string_variable()
     {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local reset = 'ResetAttributes'
              local elements = {}
              table.insert(elements, { Foreground = { Color = '#010203' } })
              table.insert(elements, { Text = 'RIGHT' })
              table.insert(elements, reset)
              table.insert(elements, { Text = '-INSERT-VAR' })
              window:set_right_status(wezterm.format(elements))
            end)
            "##,
        )
        .expect("expected static WezTerm update-status local format item table.insert string variable config");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let start_column = tab_bar
            .find("RIGHT-INSERT-VAR")
            .expect("local format item table.insert string variable status should render without escape bytes");
        let styled_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(start_column).unwrap()).unwrap();
        let plain_cell = snapshot_cell(
            &snapshot,
            0,
            u16::try_from(start_column + "RIGHT".len()).unwrap(),
        )
        .unwrap();

        assert_eq!(styled_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_ne!(plain_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
    }

    #[test]
    fn window_app_parses_static_wezterm_update_status_event_local_format_items_table_insert_top_level_string_variable()
     {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local reset = 'ResetAttributes'

            wezterm.on('update-status', function(window, pane)
              local elements = {}
              table.insert(elements, { Foreground = { Color = '#010203' } })
              table.insert(elements, { Text = 'RIGHT' })
              table.insert(elements, reset)
              table.insert(elements, { Text = '-INSERT-TOP-VAR' })
              window:set_right_status(wezterm.format(elements))
            end)
            "##,
        )
        .expect("expected static WezTerm update-status local format item table.insert top-level string variable config");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let start_column = tab_bar
            .find("RIGHT-INSERT-TOP-VAR")
            .expect("local format item table.insert top-level string variable status should render without escape bytes");
        let styled_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(start_column).unwrap()).unwrap();
        let plain_cell = snapshot_cell(
            &snapshot,
            0,
            u16::try_from(start_column + "RIGHT".len()).unwrap(),
        )
        .unwrap();

        assert_eq!(styled_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_ne!(plain_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
    }

    #[test]
    fn window_app_parses_static_wezterm_update_status_event_local_format_items_table_insert_top_level_string_alias()
     {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local reset = 'ResetAttributes'

            wezterm.on('update-status', function(window, pane)
              local item = reset
              local elements = {}
              table.insert(elements, { Foreground = { Color = '#010203' } })
              table.insert(elements, { Text = 'RIGHT' })
              table.insert(elements, item)
              table.insert(elements, { Text = '-INSERT-STRING-ALIAS' })
              window:set_right_status(wezterm.format(elements))
            end)
            "##,
        )
        .expect("expected static WezTerm update-status local format item table.insert top-level string alias config");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let start_column = tab_bar
            .find("RIGHT-INSERT-STRING-ALIAS")
            .expect("local format item table.insert top-level string alias status should render without escape bytes");
        let styled_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(start_column).unwrap()).unwrap();
        let plain_cell = snapshot_cell(
            &snapshot,
            0,
            u16::try_from(start_column + "RIGHT".len()).unwrap(),
        )
        .unwrap();

        assert_eq!(styled_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_ne!(plain_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
    }

    #[test]
    fn window_app_parses_static_wezterm_update_status_event_local_format_items_table_insert_top_level_table_variable()
     {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local accent = { Foreground = { Color = '#010203' } }

            wezterm.on('update-status', function(window, pane)
              local elements = {}
              table.insert(elements, accent)
              table.insert(elements, { Text = 'RIGHT' })
              table.insert(elements, 'ResetAttributes')
              table.insert(elements, { Text = '-INSERT-TABLE-VAR' })
              window:set_right_status(wezterm.format(elements))
            end)
            "##,
        )
        .expect("expected static WezTerm update-status local format item table.insert top-level table variable config");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let start_column = tab_bar
            .find("RIGHT-INSERT-TABLE-VAR")
            .expect("local format item table.insert top-level table variable status should render without escape bytes");
        let styled_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(start_column).unwrap()).unwrap();
        let plain_cell = snapshot_cell(
            &snapshot,
            0,
            u16::try_from(start_column + "RIGHT".len()).unwrap(),
        )
        .unwrap();

        assert_eq!(styled_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_ne!(plain_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
    }

    #[test]
    fn window_app_parses_static_wezterm_update_status_event_local_format_items_table_insert_top_level_table_alias()
     {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local accent = { Foreground = { Color = '#010203' } }

            wezterm.on('update-status', function(window, pane)
              local style = accent
              local elements = {}
              table.insert(elements, style)
              table.insert(elements, { Text = 'RIGHT' })
              table.insert(elements, 'ResetAttributes')
              table.insert(elements, { Text = '-INSERT-ALIAS' })
              window:set_right_status(wezterm.format(elements))
            end)
            "##,
        )
        .expect("expected static WezTerm update-status local format item table.insert top-level table alias config");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let start_column = tab_bar
            .find("RIGHT-INSERT-ALIAS")
            .expect("local format item table.insert top-level table alias status should render without escape bytes");
        let styled_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(start_column).unwrap()).unwrap();
        let plain_cell = snapshot_cell(
            &snapshot,
            0,
            u16::try_from(start_column + "RIGHT".len()).unwrap(),
        )
        .unwrap();

        assert_eq!(styled_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_ne!(plain_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
    }

    #[test]
    fn lua_format_items_table_variable_length_append_preserves_reset_attributes() {
        let source = r##"
        local elements = {}
        elements[#elements + 1] = { Text = 'RIGHT' }
        elements[#elements + 1] = 'ResetAttributes'
        elements[#elements + 1] = { Text = '-APPEND' }
        window:set_right_status(wezterm.format(elements))
        "##;
        let max_start = source
            .find("window:set_right_status")
            .expect("status setter should be in source");

        let table = super::lua_format_items_table_variable_with_insert_appends_before_offset(
            source, None, "elements", max_start,
        )
        .expect("expected static format item table length appends");
        let items = super::native_format_items_from_lua_format_items_table_query(&table)
            .expect("expected rebuilt format item table");

        assert_eq!(items.len(), 3, "rebuilt table was {table:?}");
        assert!(matches!(items[1], super::NativeFormatItem::ResetAttributes));
    }

    #[test]
    fn window_app_parses_static_wezterm_update_status_event_local_format_items_length_append() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local elements = {}
              elements[#elements + 1] = { Foreground = { Color = '#010203' } }
              elements[#elements + 1] = { Text = 'RIGHT' }
              elements[#elements + 1] = 'ResetAttributes'
              elements[#elements + 1] = { Text = '-APPEND' }
              window:set_right_status(wezterm.format(elements))
            end)
            "##,
        )
        .expect("expected static WezTerm update-status local format item length append config");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let start_column = tab_bar
            .find("RIGHT-APPEND")
            .expect("local format item length append status should render without escape bytes");
        let styled_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(start_column).unwrap()).unwrap();
        let plain_cell = snapshot_cell(
            &snapshot,
            0,
            u16::try_from(start_column + "RIGHT".len()).unwrap(),
        )
        .unwrap();

        assert_eq!(styled_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_ne!(plain_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
    }

    #[test]
    fn window_app_parses_static_wezterm_update_status_event_local_format_items_string_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'

            wezterm.on('update-status', function(window, pane)
              local reset = 'ResetAttributes'
              local elements = {}
              elements[#elements + 1] = { Foreground = { Color = '#010203' } }
              elements[#elements + 1] = { Text = 'RIGHT' }
              elements[#elements + 1] = reset
              elements[#elements + 1] = { Text = '-VAR' }
              window:set_right_status(wezterm.format(elements))
            end)
            "##,
        )
        .expect("expected static WezTerm update-status local format item string variable config");
        app.set_config_overrides(overrides);

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let start_column = tab_bar
            .find("RIGHT-VAR")
            .expect("local format item string variable status should render without escape bytes");
        let styled_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(start_column).unwrap()).unwrap();
        let plain_cell = snapshot_cell(
            &snapshot,
            0,
            u16::try_from(start_column + "RIGHT".len()).unwrap(),
        )
        .unwrap();

        assert_eq!(styled_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_ne!(plain_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
    }

    #[test]
    fn window_app_update_right_status_handler_sets_right_status_text() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let mut app = NativeWindowApp::new(None);
        app.update_right_status_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            Some("LEGACY-RIGHT".to_owned())
        });

        app.dispatch_update_status();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("LEGACY-RIGHT"),
            "tab bar was {tab_bar:?}"
        );
        assert_eq!(
            seen.lock().unwrap().as_slice(),
            [NativeWindowStatusUpdateEvent {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1)
            }]
        );
    }

    #[test]
    fn window_app_set_status_methods_update_tab_bar_status_text() {
        let mut app = NativeWindowApp::new(None);

        app.set_left_status("LEFT".to_owned());
        app.set_right_status("RIGHT".to_owned());

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("ws:default LEFT"),
            "tab bar was {tab_bar:?}"
        );
        assert!(tab_bar.contains("RIGHT"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_status_text_applies_sgr_styles() {
        let mut app = NativeWindowApp::new(None);
        app.left_status = "\x1b[3;4mLEFT\x1b[0m".to_owned();
        app.right_status = "\x1b[1;3;4mRIGHT\x1b[0m".to_owned();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let left_column = tab_bar
            .find("LEFT")
            .expect("left status text should render without escape bytes");
        let right_column = tab_bar
            .find("RIGHT")
            .expect("right status text should render without escape bytes");
        let left_cell = snapshot_cell(&snapshot, 0, u16::try_from(left_column).unwrap()).unwrap();
        let right_cell = snapshot_cell(&snapshot, 0, u16::try_from(right_column).unwrap()).unwrap();

        assert_eq!(left_cell.ch, 'L');
        assert!(left_cell.italic);
        assert_eq!(
            left_cell.underline_style,
            rterm_terminal::UnderlineStyle::Single
        );
        assert!(!left_cell.bold);
        assert_eq!(right_cell.ch, 'R');
        assert!(right_cell.italic);
        assert_eq!(
            right_cell.underline_style,
            rterm_terminal::UnderlineStyle::Single
        );
        assert!(right_cell.bold);
    }

    #[test]
    fn window_app_status_text_applies_sgr_truecolor() {
        let mut app = NativeWindowApp::new(None);
        app.left_status = "\x1b[38;2;1;2;3;48;2;4;5;6mCOLOR\x1b[0m".to_owned();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let color_column = tab_bar
            .find("COLOR")
            .expect("color status text should render without escape bytes");
        let color_cell = snapshot_cell(&snapshot, 0, u16::try_from(color_column).unwrap()).unwrap();

        assert_eq!(color_cell.ch, 'C');
        assert_eq!(color_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_eq!(color_cell.background, rterm_terminal::Color::Rgb(4, 5, 6));
    }

    #[test]
    fn window_app_applies_wezterm_tab_bar_background_color_to_blank_cells() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              tab_bar = {
                background = '#010203',
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm tab_bar background config");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let blank_cell = snapshot_cell(&snapshot, 0, TERMINAL_COLUMNS - 1)
            .expect("expected trailing tab bar cell");

        assert_eq!(blank_cell.ch, ' ');
        assert_eq!(blank_cell.background, rterm_terminal::Color::Rgb(1, 2, 3));
    }

    #[test]
    fn window_app_applies_wezterm_tab_bar_item_colors() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              tab_bar = {
                background = '#202122',
                active_tab = {
                  bg_color = '#010203',
                  fg_color = '#040506',
                },
                inactive_tab = {
                  bg_color = '#070809',
                  fg_color = '#0a0b0c',
                },
                new_tab = {
                  bg_color = '#0d0e0f',
                  fg_color = '#101112',
                },
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm tab_bar item color config");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let inactive_column = tab_bar
            .find("1:1")
            .expect("inactive tab label should be visible");
        let active_column = tab_bar
            .find("2:2*")
            .expect("active tab label should be visible");
        let new_tab_column = tab_bar.find('+').expect("new-tab button should be visible");
        let inactive_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(inactive_column).unwrap()).unwrap();
        let active_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(active_column).unwrap()).unwrap();
        let new_tab_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(new_tab_column).unwrap()).unwrap();

        assert_eq!(active_cell.foreground, rterm_terminal::Color::Rgb(4, 5, 6));
        assert_eq!(active_cell.background, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_eq!(
            inactive_cell.foreground,
            rterm_terminal::Color::Rgb(10, 11, 12)
        );
        assert_eq!(inactive_cell.background, rterm_terminal::Color::Rgb(7, 8, 9));
        assert_eq!(
            new_tab_cell.foreground,
            rterm_terminal::Color::Rgb(16, 17, 18)
        );
        assert_eq!(
            new_tab_cell.background,
            rterm_terminal::Color::Rgb(13, 14, 15)
        );
    }

    #[test]
    fn window_app_parses_wezterm_tab_bar_inactive_tab_edge_color() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              tab_bar = {
                inactive_tab_edge = '#1a1b1c',
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm tab_bar inactive_tab_edge config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config()
                .tab_bar_inactive_tab_edge_color,
            Some(Color::Rgb(26, 27, 28))
        );
    }

    #[test]
    fn window_app_parses_wezterm_tab_bar_inactive_tab_edge_mutation() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local colors = {}

            colors.tab_bar = {}
            colors.tab_bar.inactive_tab_edge = '#1d1e1f'
            config.colors = colors

            return config
            "##,
        )
        .expect("expected WezTerm tab_bar inactive_tab_edge mutation config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config()
                .tab_bar_inactive_tab_edge_color,
            Some(Color::Rgb(29, 30, 31))
        );
    }

    #[test]
    fn window_app_parses_static_key_tab_bar_color_field_mutations() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local colors_field = 'colors'
            local active_tab_field = 'active_tab'
            local new_tab_field = 'new_tab'

            config[colors_field] = {}
            config[colors_field].tab_bar = {}
            config[colors_field].tab_bar.background = '#202122'
            config[colors_field].tab_bar[active_tab_field] = {}
            config[colors_field].tab_bar[active_tab_field].bg_color = '#010203'
            config[colors_field].tab_bar[active_tab_field].fg_color = '#040506'
            config[colors_field].tab_bar.inactive_tab = {}
            config[colors_field].tab_bar.inactive_tab.bg_color = '#070809'
            config[colors_field].tab_bar.inactive_tab.fg_color = '#0a0b0c'
            config[colors_field].tab_bar[new_tab_field] = {}
            config[colors_field].tab_bar[new_tab_field].bg_color = '#0d0e0f'
            config[colors_field].tab_bar[new_tab_field].fg_color = '#101112'

            return config
            "##,
        )
        .expect("expected WezTerm static field-name tab_bar color config");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let effective = app.native_effective_config();
        assert_eq!(
            effective.tab_bar_background_color,
            Some(rterm_terminal::Color::Rgb(32, 33, 34))
        );
        assert_eq!(
            effective.tab_bar_active_tab_colors.bg_color,
            Some(rterm_terminal::Color::Rgb(1, 2, 3))
        );
        assert_eq!(
            effective.tab_bar_active_tab_colors.fg_color,
            Some(rterm_terminal::Color::Rgb(4, 5, 6))
        );
        assert_eq!(
            effective.tab_bar_inactive_tab_colors.bg_color,
            Some(rterm_terminal::Color::Rgb(7, 8, 9))
        );
        assert_eq!(
            effective.tab_bar_inactive_tab_colors.fg_color,
            Some(rterm_terminal::Color::Rgb(10, 11, 12))
        );
        assert_eq!(
            effective.tab_bar_new_tab_colors.bg_color,
            Some(rterm_terminal::Color::Rgb(13, 14, 15))
        );
        assert_eq!(
            effective.tab_bar_new_tab_colors.fg_color,
            Some(rterm_terminal::Color::Rgb(16, 17, 18))
        );

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let active_column = tab_bar
            .find("2:2*")
            .expect("active tab label should be visible");
        let active_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(active_column).unwrap()).unwrap();
        let blank_cell = snapshot_cell(&snapshot, 0, TERMINAL_COLUMNS - 1)
            .expect("expected trailing tab bar cell");

        assert_eq!(active_cell.foreground, rterm_terminal::Color::Rgb(4, 5, 6));
        assert_eq!(active_cell.background, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_eq!(blank_cell.background, rterm_terminal::Color::Rgb(32, 33, 34));
    }

    #[test]
    fn window_app_parses_wezterm_tab_bar_item_static_field_names() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local bg_field = 'bg_color'
            local fg_field = 'fg_color'
            local intensity_field = 'intensity'

            config.colors = {
              tab_bar = {
                active_tab = {
                  [bg_field] = '#010203',
                  [fg_field] = '#040506',
                  [intensity_field] = 'Bold',
                },
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm tab_bar item static field-name config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.tab_bar_active_tab_colors.bg_color,
            Some(rterm_terminal::Color::Rgb(1, 2, 3))
        );
        assert_eq!(
            effective.tab_bar_active_tab_colors.fg_color,
            Some(rterm_terminal::Color::Rgb(4, 5, 6))
        );
        assert_eq!(
            effective.tab_bar_active_tab_colors.intensity,
            Some(NativeFormatIntensity::Bold)
        );
    }

    #[test]
    fn window_app_parses_wezterm_tab_bar_static_table_keys() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local tab_bar_field = 'tab_bar'
            local background_field = 'background'
            local active_tab_field = 'active_tab'

            config.colors = {
              [tab_bar_field] = {
                [background_field] = '#202122',
                [active_tab_field] = {
                  bg_color = '#010203',
                  fg_color = '#040506',
                },
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm tab_bar static table-key config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.tab_bar_background_color,
            Some(rterm_terminal::Color::Rgb(32, 33, 34))
        );
        assert_eq!(
            effective.tab_bar_active_tab_colors.bg_color,
            Some(rterm_terminal::Color::Rgb(1, 2, 3))
        );
        assert_eq!(
            effective.tab_bar_active_tab_colors.fg_color,
            Some(rterm_terminal::Color::Rgb(4, 5, 6))
        );
    }

    #[test]
    fn window_app_applies_wezterm_tab_bar_item_styles() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              tab_bar = {
                active_tab = {
                  intensity = 'Normal',
                  underline = 'Single',
                  italic = true,
                  strikethrough = true,
                },
                inactive_tab = {
                  intensity = 'Half',
                  underline = 'Double',
                  italic = true,
                  strikethrough = true,
                },
                new_tab = {
                  intensity = 'Half',
                  underline = 'Single',
                  italic = true,
                  strikethrough = true,
                },
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm tab_bar item style config");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let inactive_column = tab_bar
            .find("1:1")
            .expect("inactive tab label should be visible");
        let active_column = tab_bar
            .find("2:2*")
            .expect("active tab label should be visible");
        let new_tab_column = tab_bar.find('+').expect("new-tab button should be visible");
        let inactive_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(inactive_column).unwrap()).unwrap();
        let active_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(active_column).unwrap()).unwrap();
        let new_tab_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(new_tab_column).unwrap()).unwrap();

        assert!(!active_cell.bold);
        assert!(!active_cell.faint);
        assert!(active_cell.italic);
        assert!(active_cell.strikethrough);
        assert_eq!(
            active_cell.underline_style,
            rterm_terminal::UnderlineStyle::Single
        );
        assert!(!inactive_cell.bold);
        assert!(inactive_cell.faint);
        assert!(inactive_cell.italic);
        assert!(inactive_cell.strikethrough);
        assert_eq!(
            inactive_cell.underline_style,
            rterm_terminal::UnderlineStyle::Double
        );
        assert!(!new_tab_cell.bold);
        assert!(new_tab_cell.faint);
        assert!(new_tab_cell.italic);
        assert!(new_tab_cell.strikethrough);
        assert_eq!(
            new_tab_cell.underline_style,
            rterm_terminal::UnderlineStyle::Single
        );
    }

    #[test]
    fn window_app_applies_wezterm_tab_bar_item_extended_underline_styles() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              tab_bar = {
                active_tab = {
                  underline = 'Curly',
                },
                inactive_tab = {
                  underline = 'Dotted',
                },
                new_tab = {
                  underline = 'Dashed',
                },
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm tab_bar item extended underline style config");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let inactive_column = tab_bar
            .find("1:1")
            .expect("inactive tab label should be visible");
        let active_column = tab_bar
            .find("2:2*")
            .expect("active tab label should be visible");
        let new_tab_column = tab_bar.find('+').expect("new-tab button should be visible");
        let inactive_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(inactive_column).unwrap()).unwrap();
        let active_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(active_column).unwrap()).unwrap();
        let new_tab_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(new_tab_column).unwrap()).unwrap();

        assert_eq!(
            active_cell.underline_style,
            rterm_terminal::UnderlineStyle::Curly
        );
        assert_eq!(
            inactive_cell.underline_style,
            rterm_terminal::UnderlineStyle::Dotted
        );
        assert_eq!(
            new_tab_cell.underline_style,
            rterm_terminal::UnderlineStyle::Dashed
        );
    }

    #[test]
    fn window_app_parses_wezterm_tab_bar_item_style_static_values() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local active_intensity = 'Bold'
            local active_underline = 'Curly'
            local active_italic = true
            local active_strikethrough = true
            local inactive_intensity = 'Half'
            local inactive_underline = 'Dotted'
            local inactive_italic = true
            local inactive_strikethrough = true
            local new_intensity = 'Normal'
            local new_underline = 'Dashed'
            local new_italic = true
            local new_strikethrough = true

            config.colors = {
              tab_bar = {
                active_tab = {
                  intensity = active_intensity,
                  underline = active_underline,
                  italic = active_italic,
                  strikethrough = active_strikethrough,
                },
                inactive_tab = {
                  intensity = inactive_intensity,
                  underline = inactive_underline,
                  italic = inactive_italic,
                  strikethrough = inactive_strikethrough,
                },
                new_tab = {
                  intensity = new_intensity,
                  underline = new_underline,
                  italic = new_italic,
                  strikethrough = new_strikethrough,
                },
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm tab_bar item static style value config");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let inactive_column = tab_bar
            .find("1:1")
            .expect("inactive tab label should be visible");
        let active_column = tab_bar
            .find("2:2*")
            .expect("active tab label should be visible");
        let new_tab_column = tab_bar.find('+').expect("new-tab button should be visible");
        let inactive_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(inactive_column).unwrap()).unwrap();
        let active_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(active_column).unwrap()).unwrap();
        let new_tab_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(new_tab_column).unwrap()).unwrap();

        assert!(active_cell.bold);
        assert!(!active_cell.faint);
        assert_eq!(
            active_cell.underline_style,
            rterm_terminal::UnderlineStyle::Curly
        );
        assert!(active_cell.italic);
        assert!(active_cell.strikethrough);

        assert!(!inactive_cell.bold);
        assert!(inactive_cell.faint);
        assert_eq!(
            inactive_cell.underline_style,
            rterm_terminal::UnderlineStyle::Dotted
        );
        assert!(inactive_cell.italic);
        assert!(inactive_cell.strikethrough);

        assert!(!new_tab_cell.bold);
        assert!(!new_tab_cell.faint);
        assert_eq!(
            new_tab_cell.underline_style,
            rterm_terminal::UnderlineStyle::Dashed
        );
        assert!(new_tab_cell.italic);
        assert!(new_tab_cell.strikethrough);
    }

    #[test]
    fn window_app_applies_wezterm_tab_bar_style_edges() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.tab_bar_style = {
              active_tab_left = wezterm.format({ { Text = '[' } }),
              active_tab_right = wezterm.format({ { Text = ']' } }),
              inactive_tab_left = wezterm.format({ { Text = '<' } }),
              inactive_tab_right = wezterm.format({ { Text = '>' } }),
              new_tab_left = wezterm.format({ { Text = '{' } }),
              new_tab_right = wezterm.format({ { Text = '}' } }),
            }

            return config
            "##,
        )
        .expect("expected WezTerm tab_bar_style config");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);

        assert!(
            tab_bar.contains("< 1:1 panes:1 x >"),
            "inactive tab should be wrapped by configured tab_bar_style edges: {tab_bar:?}"
        );
        assert!(
            tab_bar.contains("[ 2:2* panes:1 x ]"),
            "active tab should be wrapped by configured tab_bar_style edges: {tab_bar:?}"
        );
        assert!(
            tab_bar.contains("{ + }"),
            "new-tab button should be wrapped by configured tab_bar_style edges: {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_tab_bar_style_static_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local retro_edges = {
              active_tab_left = wezterm.format({ { Text = '[' } }),
              active_tab_right = wezterm.format({ { Text = ']' } }),
              inactive_tab_left = wezterm.format({ { Text = '<' } }),
              inactive_tab_right = wezterm.format({ { Text = '>' } }),
              new_tab_left = wezterm.format({ { Text = '{' } }),
              new_tab_right = wezterm.format({ { Text = '}' } }),
            }

            config.tab_bar_style = retro_edges

            return config
            "##,
        )
        .expect("expected WezTerm tab_bar_style static variable config");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);

        assert!(
            tab_bar.contains("< 1:1 panes:1 x >"),
            "inactive tab should be wrapped by configured tab_bar_style static variable edges: {tab_bar:?}"
        );
        assert!(
            tab_bar.contains("[ 2:2* panes:1 x ]"),
            "active tab should be wrapped by configured tab_bar_style static variable edges: {tab_bar:?}"
        );
        assert!(
            tab_bar.contains("{ + }"),
            "new-tab button should be wrapped by configured tab_bar_style static variable edges: {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_parses_wezterm_tab_bar_style_static_field_names() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local active_left = 'active_tab_left'
            local active_right = 'active_tab_right'
            local inactive_left = 'inactive_tab_left'
            local inactive_right = 'inactive_tab_right'
            local new_left = 'new_tab_left'
            local new_right = 'new_tab_right'

            config.tab_bar_style = {
              [active_left] = wezterm.format({ { Text = '[' } }),
              [active_right] = wezterm.format({ { Text = ']' } }),
              [inactive_left] = wezterm.format({ { Text = '<' } }),
              [inactive_right] = wezterm.format({ { Text = '>' } }),
              [new_left] = wezterm.format({ { Text = '{' } }),
              [new_right] = wezterm.format({ { Text = '}' } }),
            }

            return config
            "##,
        )
        .expect("expected WezTerm tab_bar_style static field-name config");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);

        assert!(
            tab_bar.contains("< 1:1 panes:1 x >"),
            "inactive tab should be wrapped by configured tab_bar_style static field-name edges: {tab_bar:?}"
        );
        assert!(
            tab_bar.contains("[ 2:2* panes:1 x ]"),
            "active tab should be wrapped by configured tab_bar_style static field-name edges: {tab_bar:?}"
        );
        assert!(
            tab_bar.contains("{ + }"),
            "new-tab button should be wrapped by configured tab_bar_style static field-name edges: {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_parses_wezterm_tab_bar_style_static_format_item_field_names() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local text_field = 'Text'
            local foreground_field = 'Foreground'
            local color_field = 'Color'

            config.tab_bar_style = {
              active_tab_left = wezterm.format({
                { [foreground_field] = { [color_field] = '#010203' } },
                { [text_field] = '[' },
              }),
              active_tab_right = wezterm.format({ { [text_field] = ']' } }),
              inactive_tab_left = wezterm.format({ { [text_field] = '<' } }),
              inactive_tab_right = wezterm.format({ { [text_field] = '>' } }),
              new_tab_left = wezterm.format({ { [text_field] = '{' } }),
              new_tab_right = wezterm.format({ { [text_field] = '}' } }),
            }

            return config
            "##,
        )
        .expect("expected WezTerm tab_bar_style static format-item field-name config");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let active_column = tab_bar
            .find("[ 2:2*")
            .expect("active tab should use static format item text field");
        let active_left_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(active_column).unwrap()).unwrap();

        assert!(
            tab_bar.contains("< 1:1 panes:1 x >"),
            "inactive tab should use static format item text fields: {tab_bar:?}"
        );
        assert!(
            tab_bar.contains("{ + }"),
            "new-tab button should use static format item text fields: {tab_bar:?}"
        );
        assert_eq!(
            active_left_cell.foreground,
            rterm_terminal::Color::Rgb(1, 2, 3)
        );
    }

    #[test]
    fn window_app_parses_wezterm_tab_bar_style_static_format_item_values() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local active_color = '#010203'
            local active_weight = 'Bold'
            local active_underline = 'Single'
            local active_left = '['
            local active_right = ']'
            local inactive_left = '<'
            local inactive_right = '>'
            local new_left = '{'
            local new_right = '}'

            config.tab_bar_style = {
              active_tab_left = wezterm.format({
                { Foreground = { Color = active_color } },
                { Attribute = { Intensity = active_weight } },
                { Attribute = { Underline = active_underline } },
                { Text = active_left },
              }),
              active_tab_right = wezterm.format({ { Text = active_right } }),
              inactive_tab_left = wezterm.format({ { Text = inactive_left } }),
              inactive_tab_right = wezterm.format({ { Text = inactive_right } }),
              new_tab_left = wezterm.format({ { Text = new_left } }),
              new_tab_right = wezterm.format({ { Text = new_right } }),
            }

            return config
            "##,
        )
        .expect("expected WezTerm tab_bar_style static format-item value config");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let active_column = tab_bar
            .find("[ 2:2*")
            .expect("active tab should use static format item text value");
        let active_left_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(active_column).unwrap()).unwrap();

        assert!(
            tab_bar.contains("< 1:1 panes:1 x >"),
            "inactive tab should use static format item text values: {tab_bar:?}"
        );
        assert!(
            tab_bar.contains("{ + }"),
            "new-tab button should use static format item text values: {tab_bar:?}"
        );
        assert_eq!(
            active_left_cell.foreground,
            rterm_terminal::Color::Rgb(1, 2, 3)
        );
        assert!(active_left_cell.bold);
        assert_eq!(
            active_left_cell.underline_style,
            rterm_terminal::UnderlineStyle::Single
        );
    }

    #[test]
    fn window_app_parses_wezterm_tab_bar_style_static_reset_attribute_item() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local reset = 'ResetAttributes'

            config.tab_bar_style = {
              active_tab_left = wezterm.format({
                { Foreground = { Color = '#010203' } },
                reset,
                { Text = '[' },
              }),
              active_tab_right = wezterm.format({ { Text = ']' } }),
              inactive_tab_left = wezterm.format({ { Text = '<' } }),
              inactive_tab_right = wezterm.format({ { Text = '>' } }),
              new_tab_left = wezterm.format({ { Text = '{' } }),
              new_tab_right = wezterm.format({ { Text = '}' } }),
            }

            return config
            "##,
        )
        .expect("expected WezTerm tab_bar_style static reset attribute item config");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let active_column = tab_bar
            .find("[ 2:2*")
            .expect("active tab should use static ResetAttributes item");
        let active_left_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(active_column).unwrap()).unwrap();

        assert!(
            tab_bar.contains("< 1:1 panes:1 x >"),
            "inactive tab should render around active reset test: {tab_bar:?}"
        );
        assert_ne!(
            active_left_cell.foreground,
            rterm_terminal::Color::Rgb(1, 2, 3)
        );
    }

    #[test]
    fn window_app_parses_wezterm_tab_bar_style_static_format_item_table_variables() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local active_left_item = { Text = '[' }
            local active_right_item = { Text = ']' }
            local inactive_left_item = { Text = '<' }
            local inactive_right_item = { Text = '>' }
            local new_left_item = { Text = '{' }
            local new_right_item = { Text = '}' }

            config.tab_bar_style = {
              active_tab_left = wezterm.format({ active_left_item }),
              active_tab_right = wezterm.format({ active_right_item }),
              inactive_tab_left = wezterm.format({ inactive_left_item }),
              inactive_tab_right = wezterm.format({ inactive_right_item }),
              new_tab_left = wezterm.format({ new_left_item }),
              new_tab_right = wezterm.format({ new_right_item }),
            }

            return config
            "##,
        )
        .expect("expected WezTerm tab_bar_style static format item table variable config");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);

        assert!(
            tab_bar.contains("< 1:1 panes:1 x >"),
            "inactive tab should use static format item table variables: {tab_bar:?}"
        );
        assert!(
            tab_bar.contains("[ 2:2* panes:1 x ]"),
            "active tab should use static format item table variables: {tab_bar:?}"
        );
        assert!(
            tab_bar.contains("{ + }"),
            "new-tab button should use static format item table variables: {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_parses_static_key_tab_bar_style_field_assignments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local style_field = 'tab_bar_style'

            config[style_field] = {}
            config[style_field].active_tab_left = wezterm.format({ { Text = '[' } })
            config[style_field].active_tab_right = wezterm.format({ { Text = ']' } })
            config[style_field].inactive_tab_left = wezterm.format({ { Text = '<' } })
            config[style_field].inactive_tab_right = wezterm.format({ { Text = '>' } })
            config[style_field].new_tab_left = wezterm.format({ { Text = '{' } })
            config[style_field]['new_tab_right'] = wezterm.format({ { Text = '}' } })

            return config
            "##,
        )
        .expect("expected WezTerm static field-name tab_bar_style config");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);

        assert!(
            tab_bar.contains("< 1:1 panes:1 x >"),
            "inactive tab should be wrapped by configured tab_bar_style static field-name edges: {tab_bar:?}"
        );
        assert!(
            tab_bar.contains("[ 2:2* panes:1 x ]"),
            "active tab should be wrapped by configured tab_bar_style static field-name edges: {tab_bar:?}"
        );
        assert!(
            tab_bar.contains("{ + }"),
            "new-tab button should be wrapped by configured tab_bar_style static field-name edges: {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_parses_static_key_tab_bar_style_window_button_assignments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local style_field = 'tab_bar_style'

            config.window_decorations = 'INTEGRATED_BUTTONS|RESIZE'
            config.integrated_title_button_style = 'Windows'
            config.integrated_title_button_alignment = 'Left'
            config.integrated_title_buttons = { 'Hide', 'Maximize', 'Close' }
            config[style_field] = {}
            config[style_field].window_hide = wezterm.format({ { Text = ' h ' } })
            config[style_field].window_maximize = wezterm.format({ { Text = ' m ' } })
            config[style_field]['window_close'] = wezterm.format({ { Text = ' c ' } })

            return config
            "##,
        )
        .expect("expected static-key tab_bar_style window button config");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);

        assert!(
            tab_bar.starts_with(" h  m  c  ws:default"),
            "window buttons should use static-key tab_bar_style labels: {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_applies_wezterm_tab_bar_style_new_tab_button_labels() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.tab_bar_style = {
              new_tab = wezterm.format({ { Text = ' add ' } }),
              new_tab_hover = wezterm.format({ { Text = ' hover-add ' } }),
            }

            return config
            "##,
        )
        .expect("expected WezTerm tab_bar_style new_tab config");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let add_column = tab_bar.find(" add ").expect("new-tab label should render");
        assert!(!tab_bar.contains(" + "), "tab bar was {tab_bar:?}");

        let x = u32::try_from(add_column + 1).unwrap_or(0) * CELL_WIDTH;
        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains(" hover-add "),
            "hover new-tab label should render: {tab_bar:?}"
        );

        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
        assert_eq!(
            app.command_palette
                .as_ref()
                .expect("new-tab button should open the launcher")
                .title(),
            "Launcher"
        );
    }

    #[test]
    fn window_app_applies_wezterm_tab_bar_hover_item_colors() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              tab_bar = {
                inactive_tab = {
                  bg_color = '#010203',
                  fg_color = '#040506',
                },
                inactive_tab_hover = {
                  bg_color = '#070809',
                  fg_color = '#0a0b0c',
                  intensity = 'Bold',
                  underline = 'Double',
                  italic = true,
                  strikethrough = true,
                },
                new_tab = {
                  bg_color = '#0d0e0f',
                  fg_color = '#101112',
                },
                new_tab_hover = {
                  bg_color = '#131415',
                  fg_color = '#161718',
                  intensity = 'Normal',
                  underline = 'Single',
                  italic = true,
                  strikethrough = true,
                },
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm tab_bar hover item color config");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let first_tab_column = rendered_tab_body_columns(&mut app)
            .iter()
            .find(|(tab, _)| *tab == rterm_core::TabId::new(1))
            .map(|(_, column)| *column)
            .expect("inactive tab body should be laid out");
        move_mouse_to_tab_bar_column(&mut app, first_tab_column);
        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let inactive_column = tab_bar
            .find("1:1")
            .expect("inactive tab label should be visible");
        let inactive_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(inactive_column).unwrap()).unwrap();

        assert_eq!(
            inactive_cell.foreground,
            rterm_terminal::Color::Rgb(10, 11, 12)
        );
        assert_eq!(inactive_cell.background, rterm_terminal::Color::Rgb(7, 8, 9));
        assert!(inactive_cell.bold);
        assert!(!inactive_cell.faint);
        assert!(inactive_cell.italic);
        assert!(inactive_cell.strikethrough);
        assert_eq!(
            inactive_cell.underline_style,
            rterm_terminal::UnderlineStyle::Double
        );

        move_mouse_to_rendered_new_tab_button(&mut app);
        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let new_tab_column = u16::try_from(
            tab_bar
                .find(" + ")
                .expect("new-tab button should be visible")
                + 1,
        )
        .unwrap();
        let new_tab_cell = snapshot_cell(&snapshot, 0, new_tab_column).unwrap();

        assert_eq!(
            new_tab_cell.foreground,
            rterm_terminal::Color::Rgb(22, 23, 24)
        );
        assert_eq!(
            new_tab_cell.background,
            rterm_terminal::Color::Rgb(19, 20, 21)
        );
        assert!(!new_tab_cell.bold);
        assert!(!new_tab_cell.faint);
        assert!(new_tab_cell.italic);
        assert!(new_tab_cell.strikethrough);
        assert_eq!(
            new_tab_cell.underline_style,
            rterm_terminal::UnderlineStyle::Single
        );
    }

    #[test]
    fn window_app_status_text_applies_sgr_indexed_colors() {
        let mut app = NativeWindowApp::new(None);
        app.left_status = "\x1b[31;104mANSI\x1b[0m".to_owned();
        app.right_status = "\x1b[38;5;196;48;5;27mIDX\x1b[0m".to_owned();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let ansi_column = tab_bar
            .find("ANSI")
            .expect("ansi status text should render without escape bytes");
        let indexed_column = tab_bar
            .find("IDX")
            .expect("indexed status text should render without escape bytes");
        let ansi_cell = snapshot_cell(&snapshot, 0, u16::try_from(ansi_column).unwrap()).unwrap();
        let indexed_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(indexed_column).unwrap()).unwrap();

        assert_eq!(ansi_cell.ch, 'A');
        assert_eq!(ansi_cell.foreground, rterm_terminal::Color::Indexed(1));
        assert_eq!(ansi_cell.background, rterm_terminal::Color::Indexed(12));
        assert_eq!(indexed_cell.ch, 'I');
        assert_eq!(indexed_cell.foreground, rterm_terminal::Color::Indexed(196));
        assert_eq!(indexed_cell.background, rterm_terminal::Color::Indexed(27));
    }

    #[test]
    fn window_app_status_text_applies_sgr_underline_color() {
        let mut app = NativeWindowApp::new(None);
        app.left_status = "\x1b[4;58:2::9:10:11mU\x1b[59mD\x1b[0m".to_owned();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let underline_column = tab_bar
            .find("UD")
            .expect("underline color status text should render without escape bytes");
        let underline_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(underline_column).unwrap()).unwrap();
        let default_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(underline_column + 1).unwrap()).unwrap();

        assert_eq!(underline_cell.ch, 'U');
        assert_eq!(
            underline_cell.underline_style,
            rterm_terminal::UnderlineStyle::Single
        );
        assert_eq!(
            underline_cell.underline_color,
            rterm_terminal::Color::Rgb(9, 10, 11)
        );
        assert_eq!(default_cell.ch, 'D');
        assert_eq!(default_cell.underline_color, rterm_terminal::Color::Default);
    }

    #[test]
    fn window_app_status_text_applies_sgr_underline_styles() {
        let mut app = NativeWindowApp::new(None);
        app.left_status = "\x1b[4:3mC\x1b[4:4mD\x1b[4:5mA\x1b[24mN\x1b[0m".to_owned();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let start_column = tab_bar
            .find("CDAN")
            .expect("underline style status text should render without escape bytes");
        let curly_cell = snapshot_cell(&snapshot, 0, u16::try_from(start_column).unwrap()).unwrap();
        let dotted_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(start_column + 1).unwrap()).unwrap();
        let dashed_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(start_column + 2).unwrap()).unwrap();
        let none_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(start_column + 3).unwrap()).unwrap();

        assert_eq!(
            curly_cell.underline_style,
            rterm_terminal::UnderlineStyle::Curly
        );
        assert!(!curly_cell.italic);
        assert_eq!(
            dotted_cell.underline_style,
            rterm_terminal::UnderlineStyle::Dotted
        );
        assert_eq!(
            dashed_cell.underline_style,
            rterm_terminal::UnderlineStyle::Dashed
        );
        assert_eq!(
            none_cell.underline_style,
            rterm_terminal::UnderlineStyle::None
        );
    }

    #[test]
    fn window_app_status_text_applies_additional_sgr_presentation_attributes() {
        let mut app = NativeWindowApp::new(None);
        app.left_status = "\x1b[5;7;8;9;53mSTYLE\x1b[25;27;28;29;55mPLAIN\x1b[0m".to_owned();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let start_column = tab_bar
            .find("STYLEPLAIN")
            .expect("presentation status text should render without escape bytes");
        let styled_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(start_column).unwrap()).unwrap();
        let plain_cell = snapshot_cell(
            &snapshot,
            0,
            u16::try_from(start_column + "STYLE".len()).unwrap(),
        )
        .unwrap();

        assert_eq!(styled_cell.ch, 'S');
        assert!(styled_cell.blink);
        assert!(styled_cell.inverse);
        assert!(styled_cell.conceal);
        assert!(styled_cell.strikethrough);
        assert!(styled_cell.overline);
        assert_eq!(plain_cell.ch, 'P');
        assert!(!plain_cell.blink);
        assert!(!plain_cell.inverse);
        assert!(!plain_cell.conceal);
        assert!(!plain_cell.strikethrough);
        assert!(!plain_cell.overline);
    }

    #[test]
    fn window_app_status_text_preserves_rapid_blink_attribute() {
        let mut app = NativeWindowApp::new(None);
        app.left_status = "\x1b[6mFAST\x1b[25mPLAIN\x1b[0m".to_owned();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let start_column = tab_bar
            .find("FASTPLAIN")
            .expect("status text should render without escape bytes");
        let rapid_cell = snapshot_cell(&snapshot, 0, u16::try_from(start_column).unwrap()).unwrap();
        let plain_cell = snapshot_cell(
            &snapshot,
            0,
            u16::try_from(start_column + "FAST".len()).unwrap(),
        )
        .unwrap();

        assert!(rapid_cell.blink);
        assert!(rapid_cell.rapid_blink);
        assert!(!plain_cell.blink);
        assert!(!plain_cell.rapid_blink);
    }

    #[test]
    fn window_app_update_status_respects_status_update_interval() {
        let calls = Arc::new(Mutex::new(0));
        let recorded = Arc::clone(&calls);
        let mut app = NativeWindowApp::new(None);
        app.status_update_interval = Duration::from_millis(1_000);
        app.update_status_handler = Box::new(move |_event| {
            *recorded.lock().unwrap() += 1;
            NativeWindowStatusUpdate {
                left_status: None,
                right_status: None,
            }
        });

        let started = Instant::now();

        assert!(app.dispatch_update_status_if_due(started));
        assert!(!app.dispatch_update_status_if_due(started + Duration::from_millis(999)));
        assert_eq!(*calls.lock().unwrap(), 1);

        assert!(app.dispatch_update_status_if_due(started + Duration::from_millis(1_000)));
        assert_eq!(*calls.lock().unwrap(), 2);
    }

    #[test]
    fn window_app_cursor_blink_rate_zero_keeps_blinking_cursor_visible() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            cursor_blink_rate_ms: Some(0),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[?12h").unwrap();
        assert!(!app.update_cursor_blink_phase_if_due(Instant::now() + Duration::from_secs(10)));

        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.render_framebuffer(&mut frame);

        assert_eq!(
            frame_pixel_at(
                &frame,
                FRAME_WIDTH as usize,
                0,
                usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize
            ),
            [229, 229, 229, 255]
        );
    }

    #[test]
    fn window_app_cursor_blink_rate_toggles_phase_when_due() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            cursor_blink_rate_ms: Some(250),
            cursor_blink_ease_out: Some(NativeEasingFunction::Constant),
            cursor_blink_ease_in: Some(NativeEasingFunction::Constant),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[?12h").unwrap();
        let started = Instant::now();

        assert!(!app.update_cursor_blink_phase_if_due(started));
        assert!(!app.update_cursor_blink_phase_if_due(started + Duration::from_millis(249)));
        assert!(app.update_cursor_blink_phase_if_due(started + Duration::from_millis(250)));
        assert!(app.frame_needs_full_repaint);

        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.render_framebuffer(&mut frame);

        assert_ne!(
            frame_pixel_at(
                &frame,
                FRAME_WIDTH as usize,
                0,
                usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize
            ),
            [229, 229, 229, 255]
        );
    }

    #[test]
    fn window_app_cursor_blink_linear_easing_updates_cursor_opacity() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            cursor_blink_rate_ms: Some(500),
            cursor_blink_ease_out: Some(NativeEasingFunction::Linear),
            cursor_blink_ease_in: Some(NativeEasingFunction::Linear),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[?12h").unwrap();
        let started = Instant::now();

        assert!(!app.update_cursor_blink_phase_if_due(started));
        assert!(app.update_cursor_blink_phase_if_due(started + Duration::from_millis(250)));

        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.render_framebuffer(&mut frame);

        assert_eq!(
            frame_pixel_at(
                &frame,
                FRAME_WIDTH as usize,
                0,
                usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize
            ),
            [120, 120, 120, 255]
        );
    }

    #[test]
    fn window_app_text_blink_linear_easing_updates_text_opacity() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            text_blink_rate_ms: Some(500),
            text_blink_ease_out: Some(NativeEasingFunction::Linear),
            text_blink_ease_in: Some(NativeEasingFunction::Linear),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[5;38;2;255;0;0;48;2;3;4;5mA")
            .unwrap();
        let started = Instant::now();

        assert!(!app.update_text_blink_phase_if_due(started));
        assert!(app.update_text_blink_phase_if_due(started + Duration::from_millis(250)));

        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.render_framebuffer(&mut frame);

        assert!(count_frame_pixels(&frame, [128, 2, 2, 255]) > 0);
        assert_eq!(count_frame_pixels(&frame, [255, 0, 0, 255]), 0);
    }

    #[test]
    fn window_app_rapid_text_blink_uses_rapid_rate() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            text_blink_rate_ms: Some(1_000),
            text_blink_rate_rapid_ms: Some(250),
            text_blink_ease_out: Some(NativeEasingFunction::Constant),
            text_blink_ease_in: Some(NativeEasingFunction::Constant),
            text_blink_rapid_ease_out: Some(NativeEasingFunction::Constant),
            text_blink_rapid_ease_in: Some(NativeEasingFunction::Constant),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[6;38;2;255;0;0;48;2;3;4;5mA")
            .unwrap();
        let started = Instant::now();

        assert!(!app.update_text_blink_phase_if_due(started));
        assert!(app.update_text_blink_phase_if_due(started + Duration::from_millis(250)));

        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.render_framebuffer(&mut frame);

        assert_eq!(count_frame_pixels(&frame, [255, 0, 0, 255]), 0);
    }

    #[test]
    fn window_app_default_cursor_style_override_updates_runtime_default() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            default_cursor_style: Some(NativeCursorStyle::BlinkingBar),
            ..NativeConfigSnapshot::default()
        });

        assert_eq!(
            app.native_effective_config().default_cursor_style,
            NativeCursorStyle::BlinkingBar
        );
        assert_eq!(app.runtime.terminal().cursor_shape(), CursorShape::Bar);
        assert!(app.runtime.terminal().cursor_blinking());
        let cursor = app
            .render_snapshot()
            .cursor()
            .expect("expected visible cursor");
        assert_eq!(cursor.shape, CursorShape::Bar);
        assert!(cursor.blinking);

        app.handle_pty_output(b"\x1b[2 q\x1b[0 q").unwrap();

        assert_eq!(app.runtime.terminal().cursor_shape(), CursorShape::Bar);
        assert!(app.runtime.terminal().cursor_blinking());
    }

    #[test]
    fn window_app_cursor_thickness_override_updates_renderer() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            default_cursor_style: Some(NativeCursorStyle::SteadyBar),
            cursor_thickness: Some(NativeCursorThickness::Pixels(3)),
            ..NativeConfigSnapshot::default()
        });

        assert_eq!(
            app.native_effective_config().cursor_thickness,
            Some(NativeCursorThickness::Pixels(3))
        );

        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.render_framebuffer(&mut frame);
        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;

        assert_eq!(
            frame_pixel_at(&frame, FRAME_WIDTH as usize, 2, terminal_origin_y),
            [229, 229, 229, 255]
        );
        assert_eq!(
            frame_pixel_at(&frame, FRAME_WIDTH as usize, 3, terminal_origin_y),
            [12, 12, 12, 255]
        );
    }

    #[test]
    fn window_app_cursor_thickness_percent_override_updates_renderer() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            default_cursor_style: Some(NativeCursorStyle::SteadyUnderline),
            cursor_thickness: Some(NativeCursorThickness::Percent(200)),
            ..NativeConfigSnapshot::default()
        });

        assert_eq!(
            app.native_effective_config().cursor_thickness,
            Some(NativeCursorThickness::Percent(200))
        );

        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.render_framebuffer(&mut frame);
        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;

        assert_eq!(
            frame_pixel_at(&frame, FRAME_WIDTH as usize, 0, terminal_origin_y + 12),
            [229, 229, 229, 255]
        );
        assert_eq!(
            frame_pixel_at(&frame, FRAME_WIDTH as usize, 0, terminal_origin_y + 11),
            [12, 12, 12, 255]
        );
    }

    #[test]
    fn window_app_cursor_thickness_points_override_updates_renderer() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            default_cursor_style: Some(NativeCursorStyle::SteadyBar),
            cursor_thickness: Some(NativeCursorThickness::Points(2)),
            ..NativeConfigSnapshot::default()
        });

        assert_eq!(
            app.native_effective_config().cursor_thickness,
            Some(NativeCursorThickness::Points(2))
        );

        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.render_framebuffer(&mut frame);
        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;

        assert_eq!(
            frame_pixel_at(&frame, FRAME_WIDTH as usize, 2, terminal_origin_y),
            [229, 229, 229, 255]
        );
        assert_eq!(
            frame_pixel_at(&frame, FRAME_WIDTH as usize, 3, terminal_origin_y),
            [12, 12, 12, 255]
        );
    }

    #[test]
    fn window_app_cursor_thickness_points_scale_with_window_dpi() {
        let mut app = NativeWindowApp::new(None);
        app.apply_window_scale_factor(1.5);

        app.set_config_overrides(native_config_snapshot! {
            default_cursor_style: Some(NativeCursorStyle::SteadyBar),
            cursor_thickness: Some(NativeCursorThickness::Points(2)),
            ..NativeConfigSnapshot::default()
        });

        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.render_framebuffer(&mut frame);
        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;

        assert_eq!(
            frame_pixel_at(&frame, FRAME_WIDTH as usize, 3, terminal_origin_y),
            [229, 229, 229, 255]
        );
        assert_eq!(
            frame_pixel_at(&frame, FRAME_WIDTH as usize, 4, terminal_origin_y),
            [12, 12, 12, 255]
        );
    }

    #[test]
    fn window_app_underline_thickness_override_updates_renderer() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            underline_thickness: Some(NativeUnderlineThickness::Pixels(3)),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[4;38;2;255;0;0m ").unwrap();

        assert_eq!(
            app.native_effective_config().underline_thickness,
            Some(NativeUnderlineThickness::Pixels(3))
        );

        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.render_framebuffer(&mut frame);
        let red_rows = (0..FRAME_HEIGHT as usize)
            .filter(|row| frame_pixel_at(&frame, FRAME_WIDTH as usize, 0, *row) == [255, 0, 0, 255])
            .collect::<Vec<_>>();
        assert_eq!(red_rows.len(), 3);
        assert!(red_rows.windows(2).all(|rows| rows[1] == rows[0] + 1));
    }

    #[test]
    fn window_app_underline_position_override_updates_renderer() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            underline_position: Some(NativeUnderlinePosition::Pixels(-4)),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[4;38;2;255;0;0m ").unwrap();

        assert_eq!(
            app.native_effective_config().underline_position,
            Some(NativeUnderlinePosition::Pixels(-4))
        );

        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.render_framebuffer(&mut frame);
        let red_rows = (0..FRAME_HEIGHT as usize)
            .filter(|row| frame_pixel_at(&frame, FRAME_WIDTH as usize, 0, *row) == [255, 0, 0, 255])
            .collect::<Vec<_>>();
        assert!(!red_rows.is_empty());
        assert!(red_rows.iter().all(|row| *row >= usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize));
    }

    #[test]
    fn window_app_strikethrough_position_override_updates_renderer() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            strikethrough_position: Some(NativeStrikethroughPosition::CellFractionPerMille(250)),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[9;38;2;255;0;0m ").unwrap();

        assert_eq!(
            app.native_effective_config().strikethrough_position,
            Some(NativeStrikethroughPosition::CellFractionPerMille(250))
        );

        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.render_framebuffer(&mut frame);
        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;

        assert_eq!(
            frame_pixel_at(&frame, FRAME_WIDTH as usize, 0, terminal_origin_y + 4),
            [255, 0, 0, 255]
        );
        assert_eq!(
            frame_pixel_at(&frame, FRAME_WIDTH as usize, 0, terminal_origin_y + 6),
            [12, 12, 12, 255]
        );
    }

    #[test]
    fn window_app_bold_brightens_ansi_colors_override_updates_renderer() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            bold_brightens_ansi_colors: Some(NativeBoldBrightensAnsiColors::No),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[1;31mA").unwrap();

        assert_eq!(
            app.native_effective_config().bold_brightens_ansi_colors,
            NativeBoldBrightensAnsiColors::No
        );

        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.render_framebuffer(&mut frame);

        assert!(count_frame_pixels(&frame, [205, 49, 49, 255]) > 0);
        assert_eq!(count_frame_pixels(&frame, [241, 76, 76, 255]), 0);
    }

    #[test]
    fn window_app_force_reverse_video_cursor_override_updates_renderer() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            force_reverse_video_cursor: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[38;2;255;255;255;48;2;0;0;0mA\x1b[1;1H")
            .unwrap();

        assert!(app.native_effective_config().force_reverse_video_cursor);

        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.render_framebuffer(&mut frame);
        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;

        assert_eq!(
            frame_pixel_at(&frame, FRAME_WIDTH as usize, 0, terminal_origin_y),
            [255, 255, 255, 255]
        );
    }

    #[test]
    fn window_app_cursor_color_escape_overrides_force_reverse_video_cursor() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            force_reverse_video_cursor: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b]12;#00ff00\x07\x1b[38;2;255;0;0;48;2;0;0;255mA\x1b[1;1H")
            .unwrap();

        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.render_framebuffer(&mut frame);
        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;

        assert_eq!(
            frame_pixel_at(&frame, FRAME_WIDTH as usize, 0, terminal_origin_y),
            [0, 255, 0, 255]
        );
    }

    #[test]
    fn window_app_cursor_color_reset_restores_force_reverse_video_cursor() {
        let mut app = NativeWindowApp::new(None);

        app.set_config_overrides(native_config_snapshot! {
            force_reverse_video_cursor: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(
            b"\x1b]12;#00ff00\x07\x1b]112\x07\x1b[38;2;255;255;255;48;2;0;0;0mA\x1b[1;1H",
        )
        .unwrap();

        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.render_framebuffer(&mut frame);
        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;

        assert_eq!(
            frame_pixel_at(&frame, FRAME_WIDTH as usize, 0, terminal_origin_y),
            [255, 255, 255, 255]
        );
    }

    #[test]
    fn window_app_right_status_clips_from_left_when_too_wide() {
        let mut app = NativeWindowApp::new(None);
        app.right_status = format!(
            "{}VISIBLE-RIGHT-EDGE",
            "x".repeat(usize::from(TERMINAL_COLUMNS))
        );

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);

        assert!(
            tab_bar.contains("VISIBLE-RIGHT-EDGE"),
            "tab bar was {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_tab_bar_uses_active_pane_title() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("PowerShell"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_tab_bar_uses_inactive_tab_active_pane_title() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("PowerShell"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_tab_bar_prefers_explicit_tab_title() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "build".to_owned(),
        })
        .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("build"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("PowerShell"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_tab_title_formatter_can_override_default_tab_title() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();
        app.tab_title_formatter = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            Some(NativeTabTitle::Text(format!(
                "tab:{} pane:{} title:{}",
                event.tab.get(),
                event.active_pane.get(),
                event.default_title.as_deref().unwrap_or("<none>")
            )))
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("tab:1 pane:1 title:PowerShell"),
            "tab bar was {tab_bar:?}"
        );

        let events = seen.lock().unwrap();
        assert!(!events.is_empty());
        assert_eq!(events[0].default_title.as_deref(), Some("PowerShell"));
        assert_eq!(events[0].tab, rterm_core::TabId::new(1));
        assert_eq!(events[0].active_pane, rterm_core::PaneId::new(1));
        assert_eq!(events[0].tab_index, 0);
        assert_eq!(events[0].tab_count, 1);
        assert_eq!(events[0].pane_count, 1);
        assert!(events[0].is_active);
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_event_string_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return 'STATIC LUA TAB'
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title event string return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("STATIC LUA TAB"),
            "tab bar was {tab_bar:?}"
        );
        assert!(!tab_bar.contains("PowerShell"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_uses_first_static_wezterm_format_tab_title_handler() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return 'FIRST LUA TAB'
            end)

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return 'SECOND LUA TAB'
            end)
            "#,
        )
        .expect("expected first static WezTerm format-tab-title handler");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("FIRST LUA TAB"), "tab bar was {tab_bar:?}");
        assert!(
            !tab_bar.contains("SECOND LUA TAB"),
            "tab bar was {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_event_name_variable() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local event_name = 'format-tab-title'

            wezterm.on(event_name, function(tab, tabs, panes, config, hover, max_width)
              return 'STATIC LUA TAB'
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title event-name variable");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("STATIC LUA TAB"),
            "tab bar was {tab_bar:?}"
        );
        assert!(!tab_bar.contains("PowerShell"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_event_name_concat() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local event_prefix = 'format-'
            local event_kind = 'tab-title'

            wezterm.on(event_prefix .. event_kind, function(tab, tabs, panes, config, hover, max_width)
              return 'STATIC LUA TAB'
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title event-name concat");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("STATIC LUA TAB"),
            "tab bar was {tab_bar:?}"
        );
        assert!(!tab_bar.contains("PowerShell"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_event_string_variable_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local static_title = 'STATIC LUA TAB'
              return static_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title event string variable return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("STATIC LUA TAB"),
            "tab bar was {tab_bar:?}"
        );
        assert!(!tab_bar.contains("PowerShell"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_event_top_level_string_variable_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local static_title = 'STATIC LUA TAB'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return static_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title event top-level string variable return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("STATIC LUA TAB"),
            "tab bar was {tab_bar:?}"
        );
        assert!(!tab_bar.contains("PowerShell"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_dynamic_variable_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PaneShell\x07").unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "explicit".to_owned(),
        })
        .unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local title = tab.active_pane.title
              return title
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title dynamic variable return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("PaneShell"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("explicit"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_helper_variable_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PaneShell\x07").unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "explicit".to_owned(),
        })
        .unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            function tab_title(tab_info)
              return tab_info.active_pane.title
            end

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local title = tab_title(tab)
              return title
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title helper variable return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("PaneShell"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("explicit"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_helper_text_item_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PaneShell\x07").unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "explicit".to_owned(),
        })
        .unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            function tab_title(tab_info)
              return tab_info.active_pane.title
            end

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local title = tab_title(tab)
              return {
                { Text = ' ' .. title .. ' ' },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title helper Text item return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains(" PaneShell "), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("explicit"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_helper_prefers_explicit_title() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PaneShell\x07").unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            function tab_title(tab_info)
              local title = tab_info.tab_title
              if title and #title > 0 then
                return title
              end
              return tab_info.active_pane.title
            end

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local title = tab_title(tab)
              return {
                { Text = ' ' .. title .. ' ' },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title helper explicit-title fallback");
        app.set_config_overrides(overrides);
        let active_tab = app.active_tab_id();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: active_tab,
            title: "explicit".to_owned(),
        })
        .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains(" explicit "), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("PaneShell"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn lua_parses_format_tab_title_helper_explicit_title_fallback_parts() {
        let body = r#"
              local title = tab_info.tab_title
              if title and #title > 0 then
                return title
              end
              return tab_info.active_pane.title
            "#;

        let parts = super::lua_tab_title_return_text_parts_from_function_body(
            body, "tab_info", "tabs", "panes", None, 0,
        )
        .expect("expected helper fallback title parts");

        assert_eq!(
            parts,
            vec![super::NativeLuaTabTitleTextPart::ActiveTabTitleOrActivePaneTitle]
        );
    }

    #[test]
    fn lua_parses_wezterm_format_tab_title_helper_explicit_title_fallback_config() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            function tab_title(tab_info)
              local title = tab_info.tab_title
              if title and #title > 0 then
                return title
              end
              return tab_info.active_pane.title
            end

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local title = tab_title(tab)
              return {
                { Text = ' ' .. title .. ' ' },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title helper explicit-title fallback");

        let parsed = format!("{:?}", overrides.lua_tab_title);
        assert!(
            parsed.contains("ActiveTabTitleOrActivePaneTitle"),
            "parsed lua tab title was {parsed}"
        );
    }

    #[test]
    fn lua_parses_wezterm_format_tab_title_active_and_last_active_branches() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            function tab_title(tab_info)
              local title = tab_info.tab_title
              if title and #title > 0 then
                return title
              end
              return tab_info.active_pane.title
            end

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local title = tab_title(tab)
              if tab.is_active then
                return {
                  { Background = { Color = 'blue' } },
                  { Text = ' ' .. title .. ' ' },
                }
              end
              if tab.is_last_active then
                return {
                  { Background = { Color = 'green' } },
                  { Text = ' ' .. title .. '*' },
                }
              end
              return title
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title active/last-active branches");

        let parsed = format!("{:?}", overrides.lua_tab_title);
        assert!(
            parsed.contains("IsActive"),
            "parsed lua tab title was {parsed}"
        );
        assert!(
            parsed.contains("IsLastActive"),
            "parsed lua tab title was {parsed}"
        );
        assert!(
            parsed.contains("ActiveTabTitleOrActivePaneTitle"),
            "parsed lua tab title was {parsed}"
        );
    }

    #[test]
    fn lua_parses_wezterm_format_tab_title_hover_branch() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              if hover then
                return {
                  { Text = 'hover:' .. tab.tab_title },
                }
              end
              return {
                { Text = 'plain:' .. tab.tab_title },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title hover branch");

        let parsed = format!("{:?}", overrides.lua_tab_title);
        assert!(
            parsed.contains("IsHover"),
            "parsed lua tab title was {parsed}"
        );
    }

    #[test]
    fn lua_parses_wezterm_format_tab_title_elseif_hover_branch() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              if tab.is_active then
                return {
                  { Text = 'active:' .. tab.tab_title },
                }
              elseif hover then
                return {
                  { Text = 'hover:' .. tab.tab_title },
                }
              end
              return {
                { Text = 'plain:' .. tab.tab_title },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title elseif hover branch");

        let parsed = format!("{:?}", overrides.lua_tab_title);
        assert!(
            parsed.contains("IsActive"),
            "parsed lua tab title was {parsed}"
        );
        assert!(
            parsed.contains("IsHover"),
            "parsed lua tab title was {parsed}"
        );
    }

    #[test]
    fn lua_parses_wezterm_format_tab_title_conditional_color_assignments() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local background = '#010203'
              local foreground = '#040506'
              if tab.is_active then
                background = '#070809'
                foreground = '#0a0b0c'
              elseif hover then
                background = '#0d0e0f'
                foreground = '#101112'
              end
              return {
                { Background = { Color = background } },
                { Foreground = { Color = foreground } },
                { Text = tab.tab_title },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title conditional color assignments");

        let parsed = format!("{:?}", overrides.lua_tab_title);
        assert!(
            parsed.contains("IsActive"),
            "parsed lua tab title was {parsed}"
        );
        assert!(
            parsed.contains("IsHover"),
            "parsed lua tab title was {parsed}"
        );
        assert!(
            parsed.contains("Rgb(7, 8, 9)"),
            "parsed lua tab title was {parsed}"
        );
        assert!(
            parsed.contains("Rgb(13, 14, 15)"),
            "parsed lua tab title was {parsed}"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_conditional_color_assignments() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local background = '#010203'
              local foreground = '#040506'
              if tab.is_active then
                background = '#070809'
                foreground = '#0a0b0c'
              elseif hover then
                background = '#0d0e0f'
                foreground = '#101112'
              end
              return {
                { Background = { Color = background } },
                { Foreground = { Color = foreground } },
                { Text = tab.tab_title },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title conditional color assignments");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "first".to_owned(),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(2),
            title: "second".to_owned(),
        })
        .unwrap();

        let first_tab_column = rendered_tab_body_columns(&mut app)
            .iter()
            .find(|(tab, _)| *tab == rterm_core::TabId::new(1))
            .map(|(_, column)| *column)
            .expect("inactive tab body should be laid out");
        move_mouse_to_tab_bar_column(&mut app, first_tab_column);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let first_column = tab_bar
            .find("first")
            .expect("hover tab title should render");
        let second_column = tab_bar
            .find("second")
            .expect("active tab title should render");
        let first_cell = snapshot_cell(&snapshot, 0, u16::try_from(first_column).unwrap()).unwrap();
        let second_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(second_column).unwrap()).unwrap();

        assert_eq!(first_cell.background, rterm_terminal::Color::Rgb(13, 14, 15));
        assert_eq!(first_cell.foreground, rterm_terminal::Color::Rgb(16, 17, 18));
        assert_eq!(second_cell.background, rterm_terminal::Color::Rgb(7, 8, 9));
        assert_eq!(
            second_cell.foreground,
            rterm_terminal::Color::Rgb(10, 11, 12)
        );
    }

    #[test]
    fn lua_parses_wezterm_format_tab_title_elaborate_shared_assignments() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local SOLID_LEFT_ARROW = wezterm.nerdfonts.pl_right_hard_divider
            local SOLID_RIGHT_ARROW = wezterm.nerdfonts.pl_left_hard_divider

            function tab_title(tab_info)
              local title = tab_info.tab_title
              if title and #title > 0 then
                return title
              end
              return tab_info.active_pane.title
            end

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local edge_background = '#0b0022'
              local background = '#1b1032'
              local foreground = '#808080'

              if tab.is_active then
                background = '#2b2042'
                foreground = '#c0c0c0'
              elseif hover then
                background = '#3b3052'
                foreground = '#909090'
              end

              local edge_foreground = background
              local title = tab_title(tab)
              title = wezterm.truncate_right(title, max_width - 2)

              return {
                { Background = { Color = edge_background } },
                { Foreground = { Color = edge_foreground } },
                { Text = SOLID_LEFT_ARROW },
                { Background = { Color = background } },
                { Foreground = { Color = foreground } },
                { Text = title },
                { Background = { Color = edge_background } },
                { Foreground = { Color = edge_foreground } },
                { Text = SOLID_RIGHT_ARROW },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title elaborate shared assignments");

        let parsed = format!("{:?}", overrides.lua_tab_title);
        assert!(
            parsed.contains("IsActive"),
            "parsed lua tab title was {parsed}"
        );
        assert!(
            parsed.contains("IsHover"),
            "parsed lua tab title was {parsed}"
        );
        assert!(
            parsed.contains("TruncateRight"),
            "parsed lua tab title was {parsed}"
        );
        assert!(
            parsed.contains("Rgb(43, 32, 66)"),
            "parsed lua tab title was {parsed}"
        );
        assert!(
            parsed.contains("Rgb(59, 48, 82)"),
            "parsed lua tab title was {parsed}"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_elaborate_shared_assignments() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local SOLID_LEFT_ARROW = wezterm.nerdfonts.pl_right_hard_divider
            local SOLID_RIGHT_ARROW = wezterm.nerdfonts.pl_left_hard_divider

            function tab_title(tab_info)
              local title = tab_info.tab_title
              if title and #title > 0 then
                return title
              end
              return tab_info.active_pane.title
            end

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local edge_background = '#0b0022'
              local background = '#1b1032'
              local foreground = '#808080'

              if tab.is_active then
                background = '#2b2042'
                foreground = '#c0c0c0'
              elseif hover then
                background = '#3b3052'
                foreground = '#909090'
              end

              local edge_foreground = background
              local title = tab_title(tab)
              title = wezterm.truncate_right(title, max_width - 2)

              return {
                { Background = { Color = edge_background } },
                { Foreground = { Color = edge_foreground } },
                { Text = SOLID_LEFT_ARROW },
                { Background = { Color = background } },
                { Foreground = { Color = foreground } },
                { Text = title },
                { Background = { Color = edge_background } },
                { Foreground = { Color = edge_foreground } },
                { Text = SOLID_RIGHT_ARROW },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title elaborate shared assignments");
        app.set_config_overrides(overrides);
        app.runtime.resize(rterm_core::TerminalSize::new(120, 2));
        app.frame_width = 120 * CELL_WIDTH;
        app.tab_max_width = 32;
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "first-title".to_owned(),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(2),
            title: "second-title".to_owned(),
        })
        .unwrap();

        let first_tab_column = rendered_tab_body_columns(&mut app)
            .iter()
            .find(|(tab, _)| *tab == rterm_core::TabId::new(1))
            .map(|(_, column)| *column)
            .expect("inactive tab body should be laid out");
        move_mouse_to_tab_bar_column(&mut app, first_tab_column);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let first_column = tab_bar
            .find("first-title")
            .expect("hover tab title should render");
        let second_column = tab_bar
            .find("second-title")
            .expect("active tab title should render");
        let first_cell = snapshot_cell(&snapshot, 0, u16::try_from(first_column).unwrap()).unwrap();
        let second_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(second_column).unwrap()).unwrap();

        assert_eq!(first_cell.background, rterm_terminal::Color::Rgb(59, 48, 82));
        assert_eq!(
            first_cell.foreground,
            rterm_terminal::Color::Rgb(144, 144, 144)
        );
        assert_eq!(
            second_cell.background,
            rterm_terminal::Color::Rgb(43, 32, 66)
        );
        assert_eq!(
            second_cell.foreground,
            rterm_terminal::Color::Rgb(192, 192, 192)
        );
    }

    #[test]
    fn lua_splits_if_elseif_tab_title_branches() {
        let statement = r#"
              if tab.is_active then
                return {
                  { Text = 'active:' .. tab.tab_title },
                }
              elseif hover then
                return {
                  { Text = 'hover:' .. tab.tab_title },
                }
              end
              return {
                { Text = 'plain:' .. tab.tab_title },
              }
            "#;

        let (branches, rest_after_if) =
            super::lua_static_if_condition_and_body_branches_from_statement(statement)
                .expect("expected if/elseif branches");

        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0].0, "tab.is_active");
        assert_eq!(branches[1].0, "hover");
        assert!(branches[0].1.contains("active:"));
        assert!(branches[1].1.contains("hover:"));
        assert!(rest_after_if.contains("plain:"));

        let starts =
            super::lua_top_level_statement_start_indices_before_offset(statement, statement.len())
                .expect("expected top-level statement starts");
        let top_level_lines = starts
            .iter()
            .filter_map(|start| statement.get(*start..))
            .filter_map(|statement| statement.lines().next())
            .map(str::trim)
            .collect::<Vec<_>>();
        assert!(
            top_level_lines
                .iter()
                .any(|line| line.starts_with("return")),
            "top-level starts were {top_level_lines:?}"
        );

        let active = super::lua_static_tab_title_first_return_from_nested_body(
            statement,
            branches[0].1,
            "tab",
            "tabs",
            "panes",
            None,
        )
        .expect("expected active branch title");
        let hover = super::lua_static_tab_title_first_return_from_nested_body(
            statement,
            branches[1].1,
            "tab",
            "tabs",
            "panes",
            None,
        )
        .expect("expected hover branch title");
        let fallback = super::lua_static_tab_title_first_return_from_nested_body(
            statement,
            rest_after_if,
            "tab",
            "tabs",
            "panes",
            None,
        )
        .expect("expected fallback title");
        assert!(format!("{active:?}").contains("active:"));
        assert!(format!("{hover:?}").contains("hover:"));
        assert!(format!("{fallback:?}").contains("plain:"));
        let parsed = super::lua_static_tab_title_conditional_return_from_function_body(
            statement,
            "tab",
            "tabs",
            "panes",
            Some("hover"),
            None,
        )
        .expect("expected conditional title");
        let parsed = format!("{parsed:?}");
        assert!(parsed.contains("IsActive"), "parsed was {parsed}");
        assert!(parsed.contains("IsHover"), "parsed was {parsed}");
    }

    #[expect(
        clippy::similar_names,
        reason = "singular and plural names mirror distinct compatibility API parameters"
    )]
    #[test]
    fn lua_extracts_function_body_with_elseif_branch() {
        let callback = r#"
            function(tab, tabs, panes, config, hover, max_width)
              if tab.is_active then
                return {
                  { Text = 'active:' .. tab.tab_title },
                }
              elseif hover then
                return {
                  { Text = 'hover:' .. tab.tab_title },
                }
              end
              return {
                { Text = 'plain:' .. tab.tab_title },
              }
            end
            "#;

        let (body, tab_param, tabs_param, panes_param, hover_param) =
            super::lua_anonymous_function_body_and_format_tab_title_params_from_query(callback)
                .expect("expected function body");

        assert_eq!(tab_param, "tab");
        assert_eq!(tabs_param, "tabs");
        assert_eq!(panes_param, "panes");
        assert_eq!(hover_param, Some("hover"));
        assert!(body.contains("elseif hover"));
        assert!(body.contains("plain:"));
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_elseif_hover_branch() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              if tab.is_active then
                return {
                  { Text = 'active:' .. tab.tab_title },
                }
              elseif hover then
                return {
                  { Text = 'hover:' .. tab.tab_title },
                }
              end
              return {
                { Text = 'plain:' .. tab.tab_title },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title elseif hover branch");
        app.set_config_overrides(overrides);
        app.runtime.resize(rterm_core::TerminalSize::new(120, 2));
        app.frame_width = 120 * CELL_WIDTH;
        app.tab_max_width = 32;
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "first".to_owned(),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(2),
            title: "second".to_owned(),
        })
        .unwrap();

        let first_tab_column = rendered_tab_body_columns(&mut app)
            .iter()
            .find(|(tab, _)| *tab == rterm_core::TabId::new(1))
            .map(|(_, column)| *column)
            .expect("inactive tab body should be laid out");
        move_mouse_to_tab_bar_column(&mut app, first_tab_column);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("hover:first"), "tab bar was {tab_bar:?}");
        assert!(tab_bar.contains("active:second"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("plain:first"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn lua_parses_wezterm_format_tab_title_one_param_callback() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab)
              return tab.tab_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title one-param callback");

        let parsed = format!("{:?}", overrides.lua_tab_title);
        assert!(
            parsed.contains("ActiveTabTitle"),
            "parsed lua tab title was {parsed}"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_hover_branch() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              if hover then
                return {
                  { Text = 'hover:' .. tab.tab_title },
                }
              end
              return {
                { Text = 'plain:' .. tab.tab_title },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title hover branch");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "first".to_owned(),
        })
        .unwrap();

        let first_tab_column = rendered_tab_body_columns(&mut app)
            .iter()
            .find(|(tab, _)| *tab == rterm_core::TabId::new(1))
            .map(|(_, column)| *column)
            .expect("inactive tab body should be laid out");
        move_mouse_to_tab_bar_column(&mut app, first_tab_column);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("hover:first"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("plain:first"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn lua_parses_wezterm_format_tab_title_truncate_right_title() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            function tab_title(tab_info)
              local title = tab_info.tab_title
              if title and #title > 0 then
                return title
              end
              return tab_info.active_pane.title
            end

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local title = tab_title(tab)
              title = wezterm.truncate_right(title, max_width - 2)
              return {
                { Text = '<' .. title .. '>' },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title truncate_right title");

        let parsed = format!("{:?}", overrides.lua_tab_title);
        assert!(
            parsed.contains("TruncateRight"),
            "parsed lua tab title was {parsed}"
        );
        assert!(
            parsed.contains("max_width_offset: 2"),
            "parsed lua tab title was {parsed}"
        );
        assert!(
            parsed.contains("ActiveTabTitleOrActivePaneTitle"),
            "parsed lua tab title was {parsed}"
        );
    }

    #[test]
    fn lua_parses_tab_title_truncate_right_text_parts() {
        let body = r#"
              local title = tab_title(tab)
              title = wezterm.truncate_right(title, max_width - 2)
              return {
                { Text = '<' .. title .. '>' },
              }
            "#;
        let value = body
            .find("return")
            .and_then(|index| body.get(index..))
            .and_then(super::lua_static_return_expression_from_statement)
            .expect("expected return expression");
        let static_source = super::LuaStaticSource {
            source: body,
            max_start: body.find("return").unwrap(),
        };
        let outer_source = r#"
            function tab_title(tab_info)
              local title = tab_info.tab_title
              if title and #title > 0 then
                return title
              end
              return tab_info.active_pane.title
            end
            "#;
        let outer_static_source = super::LuaStaticSource {
            source: outer_source,
            max_start: outer_source.len(),
        };
        let assignment_value =
            super::lua_static_expression_variable_assignment_before_offset_from_query(
                body,
                "title",
                static_source.max_start,
            )
            .expect("expected title assignment before return");
        assert!(
            assignment_value.starts_with("wezterm.truncate_right"),
            "assignment value was {assignment_value:?}"
        );

        let title_parts = super::lua_tab_title_text_parts_from_expression(
            "title",
            "tab",
            "tabs",
            "panes",
            Some(static_source),
            Some(outer_static_source),
        )
        .expect("expected title segment parts");
        assert!(
            format!("{title_parts:?}").contains("TruncateRight"),
            "title segment parts were {title_parts:?}"
        );

        let parts = super::lua_tab_title_text_parts_from_expression(
            "'<' .. title .. '>'",
            "tab",
            "tabs",
            "panes",
            Some(static_source),
            Some(outer_static_source),
        )
        .expect("expected truncate_right title parts");

        assert_eq!(
            parts,
            vec![
                super::NativeLuaTabTitleTextPart::Static("<".to_owned()),
                super::NativeLuaTabTitleTextPart::TruncateRight {
                    parts: vec![super::NativeLuaTabTitleTextPart::ActiveTabTitleOrActivePaneTitle],
                    max_width_offset: 2
                },
                super::NativeLuaTabTitleTextPart::Static(">".to_owned())
            ]
        );
        assert!(value.starts_with('{'));
    }

    #[test]
    fn lua_parses_tab_title_truncate_right_assignment_parts() {
        let body = r#"
              local title = tab_title(tab)
              title = wezterm.truncate_right(title, max_width - 2)
              return title
            "#;
        let assignment_value =
            super::lua_static_expression_variable_assignment_before_offset_from_query(
                body,
                "title",
                body.find("return").unwrap(),
            )
            .expect("expected title assignment");
        let outer_source = r#"
            function tab_title(tab_info)
              local title = tab_info.tab_title
              if title and #title > 0 then
                return title
              end
              return tab_info.active_pane.title
            end
            "#;

        let parts = super::lua_tab_title_text_parts_from_expression(
            assignment_value,
            "tab",
            "tabs",
            "panes",
            Some(super::LuaStaticSource {
                source: body,
                max_start: body.find("return").unwrap(),
            }),
            Some(super::LuaStaticSource {
                source: outer_source,
                max_start: outer_source.len(),
            }),
        )
        .expect("expected truncate_right assignment parts");

        assert_eq!(
            parts,
            vec![super::NativeLuaTabTitleTextPart::TruncateRight {
                parts: vec![super::NativeLuaTabTitleTextPart::ActiveTabTitleOrActivePaneTitle],
                max_width_offset: 2
            }]
        );
    }

    #[test]
    fn lua_parses_tab_title_truncate_right_callback_body() {
        let body = r#"
              local title = tab_title(tab)
              title = wezterm.truncate_right(title, max_width - 2)
              return {
                { Text = '<' .. title .. '>' },
              }
            "#;
        let outer_source = r#"
            function tab_title(tab_info)
              local title = tab_info.tab_title
              if title and #title > 0 then
                return title
              end
              return tab_info.active_pane.title
            end
            "#;

        let parsed = super::lua_static_tab_title_return_from_function_body(
            body,
            "tab",
            "tabs",
            "panes",
            None,
            Some(super::LuaStaticSource {
                source: outer_source,
                max_start: outer_source.len(),
            }),
        )
        .expect("expected truncate_right callback body");

        let debug = format!("{parsed:?}");
        assert!(debug.contains("TruncateRight"), "parsed was {debug}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_truncate_right_title() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            function tab_title(tab_info)
              local title = tab_info.tab_title
              if title and #title > 0 then
                return title
              end
              return tab_info.active_pane.title
            end

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local title = tab_title(tab)
              title = wezterm.truncate_right(title, max_width - 2)
              return {
                { Text = '<' .. title .. '>' },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title truncate_right title");
        app.set_config_overrides(overrides);
        let active_tab = app.active_tab_id();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: active_tab,
            title: "abcdefghijklmnopqr".to_owned(),
        })
        .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("<abcdefghijklmn>"),
            "tab bar was {tab_bar:?}"
        );
        assert!(
            !tab_bar.contains("<abcdefghijklmnopqr>"),
            "tab bar was {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_truncate_right_static_key_module() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local wt = require 'wezterm'
            local truncate_key = 'truncate_right'

            function tab_title(tab_info)
              local title = tab_info.tab_title
              if title and #title > 0 then
                return title
              end
              return tab_info.active_pane.title
            end

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local title = tab_title(tab)
              title = wt[truncate_key](title, max_width - 2)
              return {
                { Text = '<' .. title .. '>' },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title truncate_right static-key module");
        app.set_config_overrides(overrides);
        let active_tab = app.active_tab_id();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: active_tab,
            title: "abcdefghijklmnopqr".to_owned(),
        })
        .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("<abcdefghijklmn>"),
            "tab bar was {tab_bar:?}"
        );
        assert!(
            !tab_bar.contains("<abcdefghijklmnopqr>"),
            "tab bar was {tab_bar:?}"
        );
    }

    #[test]
    fn lua_parses_wezterm_format_tab_title_nerdfont_dividers() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local SOLID_LEFT_ARROW = wezterm.nerdfonts.pl_right_hard_divider
            local SOLID_RIGHT_ARROW = wezterm.nerdfonts.pl_left_hard_divider

            function tab_title(tab_info)
              local title = tab_info.tab_title
              if title and #title > 0 then
                return title
              end
              return tab_info.active_pane.title
            end

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local title = tab_title(tab)
              title = wezterm.truncate_right(title, max_width - 2)
              return {
                { Text = SOLID_LEFT_ARROW },
                { Text = title },
                { Text = SOLID_RIGHT_ARROW },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title nerdfont dividers");

        let parsed = format!("{:?}", overrides.lua_tab_title);
        let solid_left_arrow = char::from_u32(0xe0b2).unwrap().to_string();
        let solid_right_arrow = char::from_u32(0xe0b0).unwrap().to_string();
        assert!(
            parsed.contains("TruncateRight"),
            "parsed lua tab title was {parsed}"
        );
        assert!(
            parsed.contains(&solid_left_arrow) || parsed.contains("\\u{e0b2}"),
            "parsed lua tab title was {parsed}"
        );
        assert!(
            parsed.contains(&solid_right_arrow) || parsed.contains("\\u{e0b0}"),
            "parsed lua tab title was {parsed}"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_nerdfont_dividers() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local SOLID_LEFT_ARROW = wezterm.nerdfonts.pl_right_hard_divider
            local SOLID_RIGHT_ARROW = wezterm.nerdfonts.pl_left_hard_divider

            function tab_title(tab_info)
              local title = tab_info.tab_title
              if title and #title > 0 then
                return title
              end
              return tab_info.active_pane.title
            end

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local title = tab_title(tab)
              title = wezterm.truncate_right(title, max_width - 2)
              return {
                { Text = SOLID_LEFT_ARROW },
                { Text = title },
                { Text = SOLID_RIGHT_ARROW },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title nerdfont dividers");
        app.set_config_overrides(overrides);
        let active_tab = app.active_tab_id();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: active_tab,
            title: "abcdefghijklmnopqr".to_owned(),
        })
        .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let expected = format!(
            "{}abcdefghijklmn{}",
            char::from_u32(0xe0b2).unwrap(),
            char::from_u32(0xe0b0).unwrap()
        );
        assert!(tab_bar.contains(&expected), "tab bar was {tab_bar:?}");
        assert!(
            !tab_bar.contains("abcdefghijklmnopqr"),
            "tab bar was {tab_bar:?}"
        );
    }
