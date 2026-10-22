    #[test]
    fn window_app_parses_static_wezterm_format_window_title_function_param_comment() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, -- active tab
              pane, tabs, panes, config)
              return 'PARAM COMMENT TITLE'
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title param comment");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "PARAM COMMENT TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_named_callback() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            local function title_callback(tab, pane, tabs, panes, config)
              return 'NAMED CALLBACK TITLE'
            end

            wezterm.on('format-window-title', title_callback)
            "#,
        )
        .expect("expected static WezTerm format-window-title named callback");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "NAMED CALLBACK TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_function_value_callback() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            local title_callback = function(tab, pane, tabs, panes, config)
              return 'FUNCTION VALUE CALLBACK TITLE'
            end

            wezterm.on('format-window-title', title_callback)
            "#,
        )
        .expect("expected static WezTerm format-window-title function value callback");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.effective_window_title(),
            "FUNCTION VALUE CALLBACK TITLE"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_parenthesized_callback() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            local title_callback = function(tab, pane, tabs, panes, config)
              return 'PARENTHESIZED CALLBACK TITLE'
            end

            wezterm.on('format-window-title', (title_callback))
            "#,
        )
        .expect("expected static WezTerm format-window-title parenthesized callback");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "PARENTHESIZED CALLBACK TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_nested_parenthesized_callback() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            local title_callback = function(tab, pane, tabs, panes, config)
              return 'NESTED PARENTHESIZED CALLBACK TITLE'
            end

            wezterm.on('format-window-title', ((title_callback)))
            "#,
        )
        .expect("expected static WezTerm format-window-title nested parenthesized callback");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.effective_window_title(),
            "NESTED PARENTHESIZED CALLBACK TITLE"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_table_field_callback() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local callbacks = {}

            callbacks.title = function(tab, pane, tabs, panes, config)
              return 'TABLE FIELD CALLBACK TITLE'
            end

            wezterm.on('format-window-title', callbacks.title)
            "#,
        )
        .expect("expected static WezTerm format-window-title table field callback");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "TABLE FIELD CALLBACK TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_table_initializer_callback() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            local callbacks = {
              title = function(tab, pane, tabs, panes, config)
                return 'TABLE INITIALIZER CALLBACK TITLE'
              end,
            }

            wezterm.on('format-window-title', callbacks.title)
            "#,
        )
        .expect("expected static WezTerm format-window-title table initializer callback");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.effective_window_title(),
            "TABLE INITIALIZER CALLBACK TITLE"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_nested_table_field_callback() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            local callbacks = {
              window = {
                title = function(tab, pane, tabs, panes, config)
                  return 'NESTED TABLE FIELD CALLBACK TITLE'
                end,
              },
            }

            wezterm.on('format-window-title', callbacks.window.title)
            "#,
        )
        .expect("expected static WezTerm format-window-title nested table-field callback");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.effective_window_title(),
            "NESTED TABLE FIELD CALLBACK TITLE"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_nested_table_field_assignment_callback()
    {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            local callbacks = {}
            callbacks.window = {}
            callbacks.window.title = function(tab, pane, tabs, panes, config)
              return 'NESTED TABLE FIELD ASSIGNMENT CALLBACK TITLE'
            end

            wezterm.on('format-window-title', callbacks.window.title)
            "#,
        )
        .expect(
            "expected static WezTerm format-window-title nested table-field assignment callback",
        );
        app.set_config_overrides(overrides);

        assert_eq!(
            app.effective_window_title(),
            "NESTED TABLE FIELD ASSIGNMENT CALLBACK TITLE"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_nested_table_named_function_callback() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            local callbacks = {}
            callbacks.window = {}
            function callbacks.window.title(tab, pane, tabs, panes, config)
              return 'NESTED TABLE NAMED FUNCTION CALLBACK TITLE'
            end

            wezterm.on('format-window-title', callbacks.window.title)
            "#,
        )
        .expect("expected static WezTerm format-window-title nested table named function callback");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.effective_window_title(),
            "NESTED TABLE NAMED FUNCTION CALLBACK TITLE"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_table_field_callback_alias() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            local callbacks = {
              window = {
                title = function(tab, pane, tabs, panes, config)
                  return 'TABLE FIELD CALLBACK ALIAS TITLE'
                end,
              },
            }
            local title_callback = callbacks.window.title

            wezterm.on('format-window-title', title_callback)
            "#,
        )
        .expect("expected static WezTerm format-window-title table-field callback alias");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.effective_window_title(),
            "TABLE FIELD CALLBACK ALIAS TITLE"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_nested_table_variable_callback_alias() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            local callbacks = {}
            local window_callbacks = {}
            window_callbacks.title = function(tab, pane, tabs, panes, config)
              return 'NESTED TABLE VARIABLE CALLBACK ALIAS TITLE'
            end
            callbacks.window = window_callbacks

            wezterm.on('format-window-title', callbacks.window.title)
            "#,
        )
        .expect("expected static WezTerm format-window-title nested table-variable callback alias");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.effective_window_title(),
            "NESTED TABLE VARIABLE CALLBACK ALIAS TITLE"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_table_field_callback_value_alias() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            local title_callback = function(tab, pane, tabs, panes, config)
              return 'TABLE FIELD CALLBACK VALUE ALIAS TITLE'
            end
            local callbacks = {}
            callbacks.window = {}
            callbacks.window.title = title_callback

            wezterm.on('format-window-title', callbacks.window.title)
            "#,
        )
        .expect("expected static WezTerm format-window-title table-field callback value alias");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.effective_window_title(),
            "TABLE FIELD CALLBACK VALUE ALIAS TITLE"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_event_name_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local event_name = 'format-window-title'

            wezterm.on(event_name, function(tab, pane, tabs, panes, config)
              return 'STATIC LUA TITLE'
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title event-name variable");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "STATIC LUA TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_event_name_concat() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local event_prefix = 'format-'
            local event_kind = 'window-title'

            wezterm.on(event_prefix .. event_kind, function(tab, pane, tabs, panes, config)
              return 'STATIC LUA TITLE'
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title event-name concat");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "STATIC LUA TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_module_alias_format_window_title_event() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wt = require 'wezterm'

            wt.on('format-window-title', function(tab, pane, tabs, panes, config)
              return 'MODULE ALIAS LUA TITLE'
            end)
            "#,
        )
        .expect("expected static WezTerm module alias format-window-title event");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "MODULE ALIAS LUA TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_require_call_format_window_title_event() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            require('wezterm').on('format-window-title', function(tab, pane, tabs, panes, config)
              return 'REQUIRE CALL LUA TITLE'
            end)
            "#,
        )
        .expect("expected static WezTerm require-call format-window-title event");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "REQUIRE CALL LUA TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_bare_require_call_format_window_title_event() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            require 'wezterm'.on('format-window-title', function(tab, pane, tabs, panes, config)
              return 'BARE REQUIRE CALL LUA TITLE'
            end)
            "#,
        )
        .expect("expected static WezTerm bare require-call format-window-title event");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "BARE REQUIRE CALL LUA TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_dotted_comment_format_window_title_event() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm -- event helper
              .on('format-window-title', function(tab, pane, tabs, panes, config)
                return 'DOTTED COMMENT LUA TITLE'
              end)
            "#,
        )
        .expect("expected static WezTerm dotted-comment format-window-title event");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "DOTTED COMMENT LUA TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_module_alias_dotted_comment_format_window_title_event() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wt = require 'wezterm'

            wt -- event helper
              .on('format-window-title', function(tab, pane, tabs, panes, config)
                return 'MODULE ALIAS DOTTED COMMENT LUA TITLE'
              end)
            "#,
        )
        .expect("expected static WezTerm module alias dotted-comment format-window-title event");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.effective_window_title(),
            "MODULE ALIAS DOTTED COMMENT LUA TITLE"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_bracket_on_format_window_title_event() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm['on']('format-window-title', function(tab, pane, tabs, panes, config)
              return 'BRACKET ON LUA TITLE'
            end)
            "#,
        )
        .expect("expected static WezTerm bracket on format-window-title event");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "BRACKET ON LUA TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_event_string_variable_concat_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local prefix = 'STATIC '
              local subject = 'LUA '
              local suffix = 'TITLE'
              return prefix .. subject .. suffix
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title string concat return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "STATIC LUA TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_event_top_level_string_concat_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local prefix = 'STATIC '
            local subject = 'LUA '
            local suffix = 'TITLE'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return prefix .. subject .. suffix
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title top-level string concat return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "STATIC LUA TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_event_top_level_string_variable_return()
    {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local static_title = 'STATIC LUA TITLE'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return static_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title top-level string variable return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "STATIC LUA TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_event_string_variable_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local static_title = 'STATIC LUA TITLE'
              return static_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title string variable return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "STATIC LUA TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_on_alias_format_window_title_event() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local on = wezterm.on

            on('format-window-title', function(tab, pane, tabs, panes, config)
              return 'ALIAS LUA TITLE'
            end)
            "#,
        )
        .expect("expected static WezTerm on alias format-window-title event string return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "ALIAS LUA TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_require_on_alias_format_window_title_event() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local on = require('wezterm').on

            on('format-window-title', function(tab, pane, tabs, panes, config)
              return 'REQUIRE ALIAS LUA TITLE'
            end)
            "#,
        )
        .expect("expected static WezTerm require on-alias format-window-title event");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "REQUIRE ALIAS LUA TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_module_on_alias_format_window_title_event() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wt = require 'wezterm'
            local on = wt.on

            on('format-window-title', function(tab, pane, tabs, panes, config)
              return 'MODULE ON ALIAS LUA TITLE'
            end)
            "#,
        )
        .expect("expected static WezTerm module on-alias format-window-title event");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "MODULE ON ALIAS LUA TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_bracket_on_alias_format_window_title_event() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local on = wezterm['on']

            on('format-window-title', function(tab, pane, tabs, panes, config)
              return 'BRACKET ALIAS LUA TITLE'
            end)
            "#,
        )
        .expect("expected static WezTerm bracket on alias format-window-title event");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "BRACKET ALIAS LUA TITLE");
    }

    #[test]
    fn window_app_parses_static_wezterm_on_alias_dotted_comment_format_window_title_event() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local on = wezterm -- event helper
              .on

            on('format-window-title', function(tab, pane, tabs, panes, config)
              return 'ALIAS DOTTED COMMENT LUA TITLE'
            end)
            "#,
        )
        .expect("expected static WezTerm on alias dotted-comment format-window-title event");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.effective_window_title(),
            "ALIAS DOTTED COMMENT LUA TITLE"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_pane_title_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Pane Title\x07").unwrap();
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return pane.title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title pane title return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "Pane Title");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_tostring_pane_title_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Pane Title\x07").unwrap();
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return tostring(pane.title)
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title tostring pane title return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "Pane Title");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_local_pane_title_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Pane Title\x07").unwrap();
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local title = pane.title
              return title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title local pane title return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "Pane Title");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_helper_explicit_title_fallback() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Pane Title\x07").unwrap();
        app.window_title = "Window Fallback".to_owned();
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

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local title = tab_title(tab)
              return title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title helper explicit-title fallback");
        app.set_config_overrides(overrides);
        assert_eq!(app.effective_window_title(), "Pane Title");

        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "Explicit Tab".to_owned(),
        })
        .unwrap();

        assert_eq!(app.effective_window_title(), "Explicit Tab");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_inline_explicit_title_fallback() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Pane Title\x07").unwrap();
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local title = tab.tab_title
              if title and #title > 0 then
                return title
              end
              return tab.active_pane.title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title inline explicit-title fallback");
        app.set_config_overrides(overrides);
        assert_eq!(app.effective_window_title(), "Pane Title");

        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "Explicit Tab".to_owned(),
        })
        .unwrap();

        assert_eq!(app.effective_window_title(), "Explicit Tab");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_inline_else_explicit_title_fallback() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Pane Title\x07").unwrap();
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local title = tab.tab_title
              if title and #title > 0 then
                return title
              else
                return tab.active_pane.title
              end
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title inline else explicit-title fallback");
        app.set_config_overrides(overrides);
        assert_eq!(app.effective_window_title(), "Pane Title");

        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "Explicit Tab".to_owned(),
        })
        .unwrap();

        assert_eq!(app.effective_window_title(), "Explicit Tab");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_active_pane_alias_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Pane Title\x07").unwrap();
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local active = tab.active_pane
              return active.title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title active pane alias return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "Pane Title");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_active_pane_alias_metadata_return() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("foreground-proc").with_cwd("/tmp/project"),
        );
        app.handle_pty_output(b"\x1b]2;Pane Title\x07").unwrap();
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local active = tab.active_pane
              return active.domain_name .. ':' .. active.foreground_process_name .. ':' .. active.current_working_dir .. ':' .. active.tty_name
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title active pane alias metadata return");
        app.set_config_overrides(overrides);
        app.session_tty_name = Some("/dev/pts/9".to_owned());

        assert_eq!(
            app.effective_window_title(),
            "local:foreground-proc:/tmp/project:/dev/pts/9"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_pane_param_alias_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Pane Title\x07").unwrap();
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local active = pane
              return active.title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title pane param alias return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "Pane Title");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_pane_param_metadata_return() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("foreground-proc").with_cwd("/tmp/project"),
        );
        app.handle_pty_output(b"\x1b]2;Pane Title\x07").unwrap();
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return pane.domain_name .. ':' .. pane.foreground_process_name .. ':' .. pane.current_working_dir .. ':' .. pane.tty_name
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title pane param metadata return");
        app.set_config_overrides(overrides);
        app.session_tty_name = Some("/dev/pts/9".to_owned());

        assert_eq!(
            app.effective_window_title(),
            "local:foreground-proc:/tmp/project:/dev/pts/9"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_pane_id_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Pane Title\x07").unwrap();
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return pane.pane_id .. ':' .. tab.active_pane.pane_id
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title pane id return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "1:1");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_active_pane_domain_name_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Pane Title\x07").unwrap();
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return tab.active_pane.domain_name
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title active pane domain name return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "local");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_active_pane_foreground_process_return()
    {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("foreground-proc"));
        app.handle_pty_output(b"\x1b]2;Pane Title\x07").unwrap();
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return tab.active_pane.foreground_process_name
            end)
            "#,
        )
        .expect(
            "expected static WezTerm format-window-title active pane foreground process return",
        );
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "foreground-proc");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_active_pane_cwd_return() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("foreground-proc").with_cwd("/tmp/project"),
        );
        app.handle_pty_output(b"\x1b]2;Pane Title\x07").unwrap();
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return tab.active_pane.current_working_dir
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title active pane cwd return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "/tmp/project");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_active_pane_tty_name_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Pane Title\x07").unwrap();
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return tab.active_pane.tty_name
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title active pane tty name return");
        app.set_config_overrides(overrides);
        app.session_tty_name = Some("/dev/pts/9".to_owned());

        assert_eq!(app.effective_window_title(), "/dev/pts/9");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_active_pane_user_var_return() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return 'prog=' .. tab.active_pane.user_vars.WEZTERM_PROG
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title active pane user var return");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_PROG=cHNo\x07")
            .unwrap();

        assert_eq!(app.effective_window_title(), "prog=psh");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_active_pane_alias_user_var_return() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local active = tab.active_pane
              return 'prog=' .. active.user_vars['WEZTERM-PROG']
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title active pane alias user var return");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM-PROG=cHNo\x07")
            .unwrap();

        assert_eq!(app.effective_window_title(), "prog=psh");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_local_user_vars_return() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local vars = pane.user_vars
              return 'prog=' .. vars['WEZTERM-PROG']
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title local user vars return");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM-PROG=cHNo\x07")
            .unwrap();

        assert_eq!(app.effective_window_title(), "prog=psh");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_user_var_condition() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              if pane.user_vars['WEZTERM-PROG'] ~= nil then
                return 'prog=' .. pane.user_vars['WEZTERM-PROG']
              end

              return 'prog=none'
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title user var condition");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM-PROG=cHNo\x07")
            .unwrap();

        assert_eq!(app.effective_window_title(), "prog=psh");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_missing_user_var_condition() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local vars = tab.active_pane.user_vars
              if vars.WEZTERM_PROG == nil then
                return 'prog=none'
              end

              return 'prog=' .. vars.WEZTERM_PROG
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title missing user var condition");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "prog=none");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_active_pane_progress_return() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return 'pct=' .. tab.active_pane.progress.Percentage
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title active pane progress return");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::SetPaneProgress {
            pane: rterm_core::PaneId::new(1),
            progress: rterm_core::app_shell::PaneProgress::Percentage(42),
        })
        .unwrap();

        assert_eq!(app.effective_window_title(), "pct=42");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_active_pane_progress_error_return() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return 'err=' .. pane.progress.Error
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title active pane progress error return");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::SetPaneProgress {
            pane: rterm_core::PaneId::new(1),
            progress: rterm_core::app_shell::PaneProgress::Error(7),
        })
        .unwrap();

        assert_eq!(app.effective_window_title(), "err=7");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_local_progress_return() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local progress = pane.progress
              return 'pct=' .. progress.Percentage
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title local progress return");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::SetPaneProgress {
            pane: rterm_core::PaneId::new(1),
            progress: rterm_core::app_shell::PaneProgress::Percentage(42),
        })
        .unwrap();

        assert_eq!(app.effective_window_title(), "pct=42");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_progress_percentage_condition() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              if pane.progress.Percentage ~= nil then
                return 'pct=' .. pane.progress.Percentage
              end

              return 'pct=none'
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title progress percentage condition");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::SetPaneProgress {
            pane: rterm_core::PaneId::new(1),
            progress: rterm_core::app_shell::PaneProgress::Percentage(42),
        })
        .unwrap();

        assert_eq!(app.effective_window_title(), "pct=42");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_missing_progress_percentage_condition()
    {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              if tab.active_pane.progress.Percentage == nil then
                return 'pct=none'
              end

              return 'pct=' .. tab.active_pane.progress.Percentage
            end)
            "#,
        )
        .expect(
            "expected static WezTerm format-window-title missing progress percentage condition",
        );
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "pct=none");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_indeterminate_progress_condition() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              if tab.active_pane.progress == 'Indeterminate' then
                return 'busy'
              end

              return 'idle'
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title indeterminate progress condition");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::SetPaneProgress {
            pane: rterm_core::PaneId::new(1),
            progress: rterm_core::app_shell::PaneProgress::Indeterminate,
        })
        .unwrap();

        assert_eq!(app.effective_window_title(), "busy");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_local_indeterminate_progress_condition()
    {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local progress = pane.progress
              if progress == 'Indeterminate' then
                return 'busy'
              end

              return 'idle'
            end)
            "#,
        )
        .expect(
            "expected static WezTerm format-window-title local indeterminate progress condition",
        );
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::SetPaneProgress {
            pane: rterm_core::PaneId::new(1),
            progress: rterm_core::app_shell::PaneProgress::Indeterminate,
        })
        .unwrap();

        assert_eq!(app.effective_window_title(), "busy");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_two_param_pane_title_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Pane Title\x07").unwrap();
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane)
              return pane.title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title two-param pane title return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "Pane Title");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_tab_title_return() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return tab.tab_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title tab title return");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "Explicit Tab".to_owned(),
        })
        .unwrap();

        assert_eq!(app.effective_window_title(), "Explicit Tab");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_tab_window_title_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Pane Title\x07").unwrap();
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return tab.window_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title tab window title return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "Window Fallback");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_tab_id_return() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return tab.tab_id .. ':' .. tab.window_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title tab id return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "1:Window Fallback");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_tab_index_and_count_return() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return tab.tab_index .. ':' .. #tabs
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title tab index/count return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "0:1");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_tab_index_offset_return() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return tab.tab_index + 1 .. '/' .. #tabs
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title tab index offset return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "1/1");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_pane_count_return() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return #panes .. ':' .. #tabs
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title pane count return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "1:1");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_pane_count_condition() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local prefix = ''

              if #panes > 0 then
                prefix = '[' .. #panes .. '] '
              end

              return prefix .. tab.window_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title pane count condition");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "[1] Window Fallback");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_pane_zoomed_condition() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.active_pane_id(),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SetPaneZoomState {
            pane: app.active_pane_id(),
            zoomed: true,
        })
        .unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              if pane.is_zoomed then
                return 'zoomed:' .. tab.window_title
              end

              return 'plain:' .. tab.window_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title pane zoomed condition");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "zoomed:Window Fallback");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_pane_alias_zoomed_condition() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.active_pane_id(),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SetPaneZoomState {
            pane: app.active_pane_id(),
            zoomed: true,
        })
        .unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local active = pane
              if active.is_zoomed then
                return 'zoomed:' .. tab.window_title
              end

              return 'plain:' .. tab.window_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title pane alias zoomed condition");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "zoomed:Window Fallback");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_else_condition() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local prefix = ''

              if #panes > 1 then
                prefix = '[many] '
              else
                prefix = '[one] '
              end

              return prefix .. tab.window_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title else condition");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "[one] Window Fallback");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_elseif_condition() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local prefix = ''

              if #panes > 1 then
                prefix = '[many] '
              elseif #tabs > 0 then
                prefix = '[tabs] '
              else
                prefix = '[none] '
              end

              return prefix .. tab.window_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title elseif condition");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "[tabs] Window Fallback");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_self_referential_condition() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local title = tab.window_title

              if #panes > 0 then
                title = '[' .. #panes .. '] ' .. title
              end

              return title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title self-referential condition");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "[1] Window Fallback");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_self_referential_else_condition() {
        let mut app = NativeWindowApp::new(None);
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local title = tab.window_title

              if #panes > 1 then
                title = '[many] ' .. title
              else
                title = '[one] ' .. title
              end

              return title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title self-referential else condition");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "[one] Window Fallback");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_tab_active_pane_title_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;Pane Title\x07").unwrap();
        app.window_title = "Window Fallback".to_owned();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              return tab.active_pane.title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title tab active pane title return");
        app.set_config_overrides(overrides);

        assert_eq!(app.effective_window_title(), "Pane Title");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_window_title_documented_default_example() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-window-title', function(tab, pane, tabs, panes, config)
              local zoomed = ''
              if tab.active_pane.is_zoomed then
                zoomed = '[Z] '
              end

              local index = ''
              if #tabs > 1 then
                index = string.format('[%d/%d] ', tab.tab_index + 1, #tabs)
              end

              return zoomed .. index .. tab.active_pane.title
            end)
            "#,
        )
        .expect("expected static WezTerm format-window-title documented default example");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]2;First Pane\x07").unwrap();
        assert_eq!(app.effective_window_title(), "First Pane");

        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.handle_pty_output(b"\x1b]2;Second Pane\x07").unwrap();
        assert_eq!(app.effective_window_title(), "[2/2] Second Pane");

        app.dispatch_app_action(AppAction::SetPaneZoomState {
            pane: app.active_pane_id(),
            zoomed: true,
        })
        .unwrap();
        assert_eq!(app.effective_window_title(), "[Z] [2/2] Second Pane");
    }

    #[test]
    fn window_title_formatter_receives_effective_config_snapshot() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.tab_max_width = 28;
        app.status_update_interval = Duration::from_millis(1_250);
        let expected = app.native_effective_config();
        app.window_title_formatter = Box::new(move |event| {
            recorded.lock().unwrap().push(event.config.clone());
            None
        });

        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1]"
        );
        assert_eq!(seen.lock().unwrap().as_slice(), [expected]);
    }
    #[test]
    fn window_title_formatter_receives_tab_and_pane_information_snapshots() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "build".to_owned(),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.window_title_formatter = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            None
        });

        assert_eq!(
            app.effective_window_title(),
            "PowerShell [workspace:1 tab:2 pane:2]"
        );

        let events = seen.lock().unwrap();
        let event = events
            .first()
            .expect("expected format-window-title event to be dispatched");
        let debug = format!("{event:?}");
        assert!(debug.contains("active_tab_info:"), "{debug}");
        assert!(debug.contains("active_pane_info:"), "{debug}");
        assert!(debug.contains("tabs:"), "{debug}");
        assert!(debug.contains("panes:"), "{debug}");
        assert!(debug.contains("tab_id: TabId(1)"), "{debug}");
        assert!(debug.contains("tab_id: TabId(2)"), "{debug}");
        assert!(debug.contains("pane_id: PaneId(2)"), "{debug}");
        assert!(debug.contains("tab_title: Some(\"build\")"), "{debug}");
    }

    #[test]
    fn window_title_formatter_receives_active_key_table_snapshot() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let mut app = NativeWindowApp::new(None);
        app.window_title_formatter = Box::new(move |event| {
            recorded
                .lock()
                .unwrap()
                .push(event.active_key_table.clone());
            None
        });

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
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - KeyTable: resize_pane"
        );

        assert!(app.command_palette_execute(WindowCommand::ClearKeyTableStack));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1]"
        );

        let mut events = seen.lock().unwrap().clone();
        events.dedup();
        assert_eq!(events.as_slice(), [Some("resize_pane".to_owned()), None]);
    }

    #[test]
    fn window_app_logs_visible_pty_output() {
        let logged = Arc::new(Mutex::new(Vec::new()));
        let mut app =
            NativeWindowApp::new_with_session_log(None, SharedWriter(Arc::clone(&logged)));

        app.handle_pty_output(b"before\x1b[6nafter").unwrap();

        assert_eq!(logged.lock().unwrap().as_slice(), b"beforeafter");
    }

    #[test]
    fn window_app_omits_title_sequence_from_session_log() {
        let logged = Arc::new(Mutex::new(Vec::new()));
        let mut app =
            NativeWindowApp::new_with_session_log(None, SharedWriter(Arc::clone(&logged)));

        app.handle_pty_output(b"before\x1b]0;ops\x07after").unwrap();

        assert_eq!(app.window_title, "ops");
        assert_eq!(logged.lock().unwrap().as_slice(), b"beforeafter");
    }

    #[test]
    fn window_app_logs_unknown_escape_sequences_when_configured() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            log_unknown_escape_sequences: Some(true),
            ..NativeConfigSnapshot::default()
        });

        app.handle_pty_output(b"before\x1bzafter").unwrap();

        assert_eq!(
            app.unknown_escape_sequence_warnings_for_test(),
            ["WARN unknown escape sequence from pane 1: ESC z"]
        );
    }

    #[test]
    fn window_app_logs_unknown_csi_sequences_without_polluting_session_log() {
        let logged = Arc::new(Mutex::new(Vec::new()));
        let mut app =
            NativeWindowApp::new_with_session_log(None, SharedWriter(Arc::clone(&logged)));
        app.set_config_overrides(native_config_snapshot! {
            log_unknown_escape_sequences: Some(true),
            ..NativeConfigSnapshot::default()
        });

        app.handle_pty_output(b"before\x1b[?999zafter").unwrap();

        assert_eq!(
            app.unknown_escape_sequence_warnings_for_test(),
            ["WARN unknown escape sequence from pane 1: CSI ?999z"]
        );
        assert_eq!(logged.lock().unwrap().as_slice(), b"beforeafter");
    }

    #[test]
    fn window_app_suppresses_unknown_escape_sequence_logs_by_default() {
        let mut app = NativeWindowApp::new(None);

        app.handle_pty_output(b"before\x1bzafter").unwrap();

        assert!(app.unknown_escape_sequence_warnings_for_test().is_empty());
    }

    #[test]
    fn window_app_scrolls_snapshot_to_scrollback_lines() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));

        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('c'));

        app.scroll_viewport_lines(1);

        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('a'));
        assert_eq!(snapshot_char(&app.snapshot, 1, 0), Some('c'));
        assert!(app.snapshot.cursor().is_none());

        app.scroll_viewport_lines(-1);

        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('c'));
        assert!(app.snapshot.cursor().is_some());
    }

    #[test]
    fn window_app_scrolls_to_bottom_on_input_by_default() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.scroll_viewport_lines(1);

        app.write_pty_bytes(b"x").unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"x");
        assert_eq!(app.current_scrollback_offset(), 0);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('c'));
    }

    #[test]
    fn window_app_can_preserve_scrollback_viewport_on_input() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.set_config_overrides(native_config_snapshot! {
            scroll_to_bottom_on_input: Some(false),
            ..NativeConfigSnapshot::default()
        });
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.scroll_viewport_lines(1);

        app.write_pty_bytes(b"x").unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), b"x");
        assert_eq!(app.current_scrollback_offset(), 1);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('a'));
    }

    #[test]
    fn window_app_clamps_scrollback_viewport_to_available_history() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();

        app.scroll_viewport_lines(99);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('a'));

        app.scroll_viewport_lines(-99);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('c'));
    }

    #[test]
    fn window_app_mouse_wheel_scrolls_scrollback_viewport() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();

        assert!(app.handle_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0)));

        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('a'));

        assert!(app.handle_mouse_wheel(MouseScrollDelta::LineDelta(0.0, -1.0)));

        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('c'));
    }

    #[test]
    fn window_app_wheel_updates_stable_viewport_top() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc").unwrap();
        let physical_top = app.runtime.terminal().stable_dimensions().physical_top;

        assert!(app.handle_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0)));

        assert_eq!(
            app.current_stable_viewport_top(),
            physical_top.checked_sub(1)
        );
    }

    #[test]
    fn window_app_page_scroll_updates_stable_viewport_top() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();

        app.command_palette_execute(WindowCommand::ScrollPageUp);

        assert_eq!(
            app.current_stable_viewport_top(),
            app.runtime
                .terminal()
                .stable_dimensions()
                .physical_top
                .checked_sub(2)
        );
    }

    #[test]
    fn window_app_scrollbar_drag_updates_stable_viewport_top() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            enable_scroll_bar: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();

        assert!(app.scroll_to_scrollbar_position(PhysicalPosition::new(
            f64::from(FRAME_WIDTH - 1),
            f64::from(tab_bar_pixel_height()),
        )));

        assert_eq!(
            app.current_stable_viewport_top(),
            Some(app.runtime.terminal().stable_dimensions().scrollback_top)
        );
    }

    #[test]
    fn window_app_scroll_to_prompt_updates_stable_viewport_top() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(
            b"\x1b]133;A\x07> one\r\nout1\r\n\x1b]133;A\x07> two\r\nout2\r\n\x1b]133;A\x07> three\r\nlive",
        )
        .unwrap();
        let prompt_rows = app.runtime.terminal().stable_semantic_prompt_rows();

        app.scroll_to_prompt(-2);

        assert_eq!(app.current_stable_viewport_top(), Some(prompt_rows[0]));
    }

    #[test]
    fn window_app_scrolled_back_viewport_stays_on_same_stable_top_after_output() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc").unwrap();
        app.scroll_viewport_lines(1);
        let stable_top = app.current_stable_viewport_top();

        app.handle_pty_output(b"\r\ndd").unwrap();

        assert_eq!(app.current_stable_viewport_top(), stable_top);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "aa  ");
    }

    #[test]
    fn window_app_pty_output_preserves_ordinary_selection_without_transient_controller() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"ordinary").unwrap();
        let selection = WindowSelection::new(
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 3 },
        );
        set_ordinary_viewport_selection_for_test(&mut app, selection);
        assert!(copy_mode_for_test(&app).is_none());
        assert!(search_for_test(&app).is_none());
        assert!(quick_select_for_test(&app).is_none());

        app.handle_pty_output(b"!").unwrap();

        assert!(ordinary_selection_for_test(&app).is_some());
        assert_eq!(app.selected_text().as_deref(), Some("ordi"));
    }

    fn ordinary_source_cell_for_viewport(
        app: &NativeWindowApp,
        row: u16,
        column: usize,
    ) -> SelectionSourceCell {
        SelectionSourceCell {
            domain: app.runtime.terminal().stable_dimensions().domain,
            row: app
                .current_viewport_stable_top()
                .saturating_add(StableRowIndex::try_from(row).unwrap()),
            column,
        }
    }

    fn set_ordinary_stable_selection_for_test(
        app: &mut NativeWindowApp,
        anchor: SelectionSourceCell,
        focus: SelectionSourceCell,
        rectangular: bool,
    ) {
        let sequence = app.runtime.terminal().current_seqno();
        set_ordinary_selection_for_test(
            app,
            Some(StableOrdinarySelection {
                anchor,
                focus,
                rectangular,
                sequence,
            }),
        );
        app.update_selection_projection();
        app.refresh_snapshot();
    }

    fn set_ordinary_viewport_selection_for_test(
        app: &mut NativeWindowApp,
        selection: WindowSelection,
    ) {
        let anchor = ordinary_source_cell_for_viewport(
            app,
            selection.anchor.row,
            usize::from(selection.anchor.column),
        );
        let focus = ordinary_source_cell_for_viewport(
            app,
            selection.focus.row,
            usize::from(selection.focus.column),
        );
        set_ordinary_stable_selection_for_test(app, anchor, focus, selection.rectangular);
    }

    fn set_ordinary_viewport_range_for_test(
        app: &mut NativeWindowApp,
        anchor: SelectionCell,
        focus: SelectionCell,
    ) {
        set_ordinary_viewport_selection_for_test(app, WindowSelection::new(anchor, focus));
    }

    #[test]
    fn window_app_visible_dirty_selected_row_clears_ordinary_selection_on_paint() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"selected\r\nother").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 3 },
        );

        app.handle_pty_output(b"\x1b[1;1HX").unwrap();

        assert!(ordinary_selection_for_test(&app).is_none());
    }

    #[test]
    fn window_app_visible_dirty_unselected_row_preserves_ordinary_selection() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"selected\r\nother").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 3 },
        );

        app.handle_pty_output(b"\x1b[2;1HX").unwrap();

        assert!(ordinary_selection_for_test(&app).is_some());
    }

    #[test]
    fn window_app_offscreen_dirty_selected_row_waits_until_visible_paint() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"old-0\r\nold-1\r\nold-2\r\nselected\r\nbottom")
            .unwrap();
        let selected = ordinary_source_cell_for_viewport(&app, 0, 0);
        set_ordinary_stable_selection_for_test(
            &mut app,
            selected,
            SelectionSourceCell {
                column: 3,
                ..selected
            },
            false,
        );
        app.scroll_viewport_lines(99);
        assert!(app.selection.is_none());

        app.handle_pty_output(b"\x1b[1;1HX").unwrap();
        assert!(ordinary_selection_for_test(&app).is_some());

        app.set_scrollback_offset(0);
        assert!(ordinary_selection_for_test(&app).is_none());
    }

    #[test]
    fn window_app_full_screen_scroll_preserves_unchanged_selected_row() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"selected\r\nother").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 3 },
        );

        app.handle_pty_output(b"\r\nnew").unwrap();

        assert!(ordinary_selection_for_test(&app).is_some());
        assert_eq!(app.selected_text().as_deref(), Some("sele"));
    }

    #[test]
    fn window_app_inactive_visible_dirty_selected_row_clears_only_that_pane_selection() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"left\r\npane").unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"right\r\npane").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 2 },
        );
        let inactive = app
            .pane_runtimes
            .get_mut(&rterm_core::PaneId::new(1))
            .expect("inactive pane runtime");
        let dimensions = inactive.runtime.terminal().stable_dimensions();
        inactive.ui.ordinary_selection = Some(StableOrdinarySelection {
            anchor: SelectionSourceCell {
                domain: dimensions.domain,
                row: dimensions.physical_top,
                column: 0,
            },
            focus: SelectionSourceCell {
                domain: dimensions.domain,
                row: dimensions.physical_top,
                column: 2,
            },
            rectangular: false,
            sequence: inactive.runtime.terminal().current_seqno(),
        });
        inactive.reconcile_terminal_mutation();

        app.handle_pane_pty_output(rterm_core::PaneId::new(1), b"\x1b[1;1HX")
            .unwrap();

        assert!(ordinary_selection_for_test(&app).is_some());
        assert!(
            app.pane_runtimes
                .get(&rterm_core::PaneId::new(1))
                .is_some_and(|runtime| runtime.ui.ordinary_selection.is_none())
        );
    }

    #[test]
    fn window_app_inactive_dirty_unselected_row_preserves_selection() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"left\r\npane").unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        let inactive = app
            .pane_runtimes
            .get_mut(&rterm_core::PaneId::new(1))
            .expect("inactive pane runtime");
        let dimensions = inactive.runtime.terminal().stable_dimensions();
        inactive.ui.ordinary_selection = Some(StableOrdinarySelection {
            anchor: SelectionSourceCell {
                domain: dimensions.domain,
                row: dimensions.physical_top,
                column: 0,
            },
            focus: SelectionSourceCell {
                domain: dimensions.domain,
                row: dimensions.physical_top,
                column: 2,
            },
            rectangular: false,
            sequence: inactive.runtime.terminal().current_seqno(),
        });
        inactive.reconcile_terminal_mutation();

        app.handle_pane_pty_output(rterm_core::PaneId::new(1), b"\x1b[2;1HX")
            .unwrap();

        assert!(
            app.pane_runtimes
                .get(&rterm_core::PaneId::new(1))
                .is_some_and(|runtime| runtime.ui.ordinary_selection.is_some())
        );
    }

    #[test]
    fn window_app_inactive_output_preserves_retained_copy_search_coordinates() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"row-zero\r\nrow-one").unwrap();
        app.enter_copy_mode();
        assert!(app.set_copy_mode_cursor(1, 0));
        assert!(app.set_copy_mode_selection_mode(super::WindowCopySelectionMode::Cell));
        assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
        assert!(app.update_search_query("row"));
        assert!(app.perform_copy_mode_assignment(super::WindowCopyModeAssignment::AcceptPattern));
        let owner = app.active_pane_id();
        let source_cursor = active_copy_mode_for_test(&app).source_cursor;
        let source_anchor = active_copy_mode_for_test(&app).source_anchor;
        assert_eq!(active_copy_mode_for_test(&app).cursor.row, 1);
        assert_eq!(
            active_copy_mode_for_test(&app).anchor.map(|cell| cell.row),
            Some(1)
        );

        app.dispatch_app_action(AppAction::SplitPane {
            pane: owner,
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pane_pty_output(owner, b"\r\nnext").unwrap();

        let inactive = app.pane_runtimes.get(&owner).expect("inactive owner");
        let copy = inactive
            .ui
            .retained_copy_mode()
            .expect("retained Copy-search state");
        assert_eq!(copy.source_cursor, source_cursor);
        assert_eq!(copy.source_anchor, source_anchor);
        assert_eq!(copy.cursor, SelectionCell { row: 0, column: 0 });
        assert_eq!(copy.anchor, Some(SelectionCell { row: 0, column: 0 }));
        assert_eq!(
            inactive.ui.copy_search_mode(),
            Some(super::WindowCopySearchMode::Copy)
        );
        assert_eq!(
            inactive
                .ui
                .retained_search()
                .map(|search| search.query.as_str()),
            Some("row")
        );

        let mut offscreen = NativeWindowApp::new(None);
        offscreen.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        offscreen.handle_pty_output(b"source\r\nvisible").unwrap();
        offscreen.enter_copy_mode();
        assert!(offscreen.set_copy_mode_cursor(0, 0));
        let owner = offscreen.active_pane_id();
        let retained_but_offscreen = active_copy_mode_for_test(&offscreen).source_cursor;
        offscreen
            .dispatch_app_action(AppAction::SplitPane {
                pane: owner,
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        offscreen
            .handle_pane_pty_output(owner, b"\r\nnew-1\r\nnew-2")
            .unwrap();
        let inactive = offscreen
            .pane_runtimes
            .get(&owner)
            .expect("inactive offscreen owner");
        assert!(
            inactive
                .runtime
                .terminal()
                .retained_stable_range()
                .contains(&retained_but_offscreen.row),
            "source row remains in history, but no longer has a viewport projection"
        );
        let copy = inactive
            .ui
            .retained_copy_mode()
            .expect("retained offscreen Copy owner");
        assert_eq!(copy.source_cursor, retained_but_offscreen);
        assert!(
            inactive.ui.overlay_active(),
            "retained stable cursor must keep the CopySearch controller alive"
        );
        assert_eq!(
            inactive
                .ui
                .stable_viewport
                .active_top(inactive.runtime.terminal()),
            Some(retained_but_offscreen.row),
            "owner viewport must move back to the retained Copy cursor"
        );
        assert_eq!(copy.cursor, SelectionCell { row: 0, column: 0 });

        let mut offscreen_anchor = NativeWindowApp::new(None);
        offscreen_anchor
            .runtime
            .resize(rterm_core::TerminalSize::new(8, 2));
        offscreen_anchor
            .handle_pty_output(b"anchor-0\r\nline-1\r\nline-2\r\nline-3\r\ncursor-4\r\nlive")
            .unwrap();
        offscreen_anchor.enter_copy_mode();
        assert!(offscreen_anchor.move_copy_mode_to_scrollback_top());
        assert!(
            offscreen_anchor.set_copy_mode_selection_mode(super::WindowCopySelectionMode::Cell)
        );
        assert!(offscreen_anchor.move_copy_mode_cursor_by_lines(4));
        let owner = offscreen_anchor.active_pane_id();
        let before = active_copy_mode_for_test(&offscreen_anchor);
        let source_cursor = before.source_cursor;
        let source_anchor = before.source_anchor.expect("offscreen source anchor");
        assert!(before.anchor.is_none(), "anchor begins offscreen");
        offscreen_anchor
            .dispatch_app_action(AppAction::SplitPane {
                pane: owner,
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        offscreen_anchor
            .handle_pane_pty_output(owner, b"\r\nmutation")
            .unwrap();
        let inactive = offscreen_anchor
            .pane_runtimes
            .get(&owner)
            .expect("inactive offscreen-anchor owner");
        let copy = inactive
            .ui
            .retained_copy_mode()
            .expect("offscreen anchor must not retire Copy");
        assert_eq!(copy.source_cursor, source_cursor);
        assert_eq!(copy.source_anchor, Some(source_anchor));
        assert!(copy.anchor.is_none(), "local anchor remains offscreen");
        assert!(
            inactive
                .runtime
                .terminal()
                .retained_stable_range()
                .contains(&source_anchor.row)
        );
    }

    #[test]
    fn window_app_inactive_prune_retires_only_unretained_copy_search_owner() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(12, 2));
        app.runtime.set_scrollback_limit(2);
        app.handle_pty_output(b"copy-old\r\nneedle\r\nlive")
            .unwrap();
        app.enter_copy_mode();
        assert!(app.move_copy_mode_to_scrollback_top());
        assert!(app.handle_copy_mode_key(&Key::Character("/".into()), ModifiersState::empty()));
        assert!(app.update_search_query("needle"));
        assert!(app.perform_copy_mode_assignment(super::WindowCopyModeAssignment::AcceptPattern));
        let owner = app.active_pane_id();
        let stale_cursor = active_copy_mode_for_test(&app).source_cursor;

        app.dispatch_app_action(AppAction::SplitPane {
            pane: owner,
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.enter_search_mode();
        assert!(!app.update_search_query("active-owner-query"));
        app.handle_pane_pty_output(owner, b"\r\nnew-1\r\nnew-2")
            .unwrap();

        let inactive = app.pane_runtimes.get(&owner).expect("inactive owner");
        assert!(
            !inactive
                .runtime
                .terminal()
                .retained_stable_range()
                .contains(&stale_cursor.row),
            "test setup must prune the inactive Copy cursor"
        );
        assert!(!inactive.ui.overlay_active());
        assert!(inactive.ui.retained_copy_mode().is_none());
        assert!(inactive.ui.retained_search().is_none());
        assert_eq!(
            search_for_test(&app).map(|search| search.query.as_str()),
            Some("active-owner-query"),
            "identity retirement must be owner-local"
        );

        let mut anchor = NativeWindowApp::new(None);
        anchor.runtime.resize(rterm_core::TerminalSize::new(12, 2));
        anchor.runtime.set_scrollback_limit(2);
        anchor
            .handle_pty_output(b"anchor-old\r\ncursor-keep\r\nlive")
            .unwrap();
        anchor.enter_copy_mode();
        assert!(anchor.move_copy_mode_to_scrollback_top());
        assert!(anchor.set_copy_mode_selection_mode(super::WindowCopySelectionMode::Cell));
        assert!(anchor.move_copy_mode_cursor_by_lines(1));
        let owner = anchor.active_pane_id();
        let copy = active_copy_mode_for_test(&anchor);
        let source_cursor = copy.source_cursor;
        let source_anchor = copy.source_anchor.expect("Copy anchor");
        anchor
            .dispatch_app_action(AppAction::SplitPane {
                pane: owner,
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        anchor
            .handle_pane_pty_output(owner, b"\r\nnew-1\r\nnew-2")
            .unwrap();
        let inactive = anchor
            .pane_runtimes
            .get(&owner)
            .expect("inactive anchor owner");
        assert!(
            inactive
                .runtime
                .terminal()
                .retained_stable_range()
                .contains(&source_cursor.row)
        );
        assert!(
            !inactive
                .runtime
                .terminal()
                .retained_stable_range()
                .contains(&source_anchor.row)
        );
        assert!(
            !inactive.ui.overlay_active(),
            "anchor-only prune must retire the full CopySearch controller"
        );
    }

    #[test]
    fn window_app_inactive_prune_clears_search_current_without_dropping_query() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(12, 2));
        app.runtime.set_scrollback_limit(2);
        app.handle_pty_output(b"needle-old\r\nkeep\r\nlive")
            .unwrap();
        app.enter_search_mode();
        assert!(app.update_search_query("needle-old"));
        let owner = app.active_pane_id();
        let stale_match = active_search_for_test(&app).current.expect("search match");

        app.dispatch_app_action(AppAction::SplitPane {
            pane: owner,
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pane_pty_output(owner, b"\r\nnew-1\r\nnew-2")
            .unwrap();

        let inactive = app.pane_runtimes.get(&owner).expect("inactive owner");
        assert!(!stale_match.is_retained(inactive.runtime.terminal()));
        let search = inactive
            .ui
            .retained_search()
            .expect("Search remains active");
        assert_eq!(search.query, "needle-old");
        assert_eq!(search.current, None);
        assert!(search.editing);
        assert_eq!(
            inactive.ui.copy_search_mode(),
            Some(super::WindowCopySearchMode::Search)
        );
    }

    #[test]
    fn window_app_inactive_quick_prune_keeps_match_identity_or_retires_overlay() {
        fn quick_fixture(
            current: usize,
        ) -> (
            NativeWindowApp,
            rterm_core::PaneId,
            WindowSearchMatch,
            String,
            WindowSearchMatch,
        ) {
            let mut app = NativeWindowApp::new(None);
            app.runtime.resize(rterm_core::TerminalSize::new(32, 2));
            app.runtime.set_scrollback_limit(2);
            app.handle_pty_output(b"https://old.test\r\nhttps://keep.test\r\nlive")
                .unwrap();
            app.enter_quick_select_mode();
            let quick = active_quick_select_for_test(&app);
            assert_eq!(quick.matches.len(), 2);
            let old = quick.matches[0];
            let keep = quick.matches[1];
            let keep_label = quick.labels[1].clone();
            app.active_ui
                .quick_select_mut()
                .expect("quick-select state")
                .current = current;
            app.update_transient_selection_projection();
            let owner = app.active_pane_id();
            app.dispatch_app_action(AppAction::SplitPane {
                pane: owner,
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
            (app, owner, keep, keep_label, old)
        }

        let (mut survives, owner, keep, keep_label, old) = quick_fixture(1);
        survives
            .handle_pane_pty_output(owner, b"\r\nnew-1\r\nnew-2")
            .unwrap();
        let inactive = survives
            .pane_runtimes
            .get(&owner)
            .expect("inactive Quick owner");
        assert!(!old.is_retained(inactive.runtime.terminal()));
        assert!(keep.is_retained(inactive.runtime.terminal()));
        let quick = inactive.ui.quick_select().expect("surviving Quick overlay");
        assert_eq!(quick.matches, [keep]);
        assert_eq!(quick.labels, [keep_label]);
        assert_eq!(quick.current, 0);
        assert_eq!(quick.current_match(), Some(keep));

        let (mut loses_current, owner, keep, _, old) = quick_fixture(0);
        loses_current
            .handle_pane_pty_output(owner, b"\r\nnew-1\r\nnew-2")
            .unwrap();
        let inactive = loses_current
            .pane_runtimes
            .get(&owner)
            .expect("inactive Quick owner");
        assert!(!old.is_retained(inactive.runtime.terminal()));
        assert!(keep.is_retained(inactive.runtime.terminal()));
        assert!(
            inactive.ui.quick_select().is_none(),
            "current loss must retire instead of retargeting to the survivor"
        );

        let (mut becomes_empty, owner, _, _, _) = quick_fixture(0);
        becomes_empty
            .handle_pane_pty_output(owner, b"\r\nnew-1\r\nnew-2\r\nnew-3")
            .unwrap();
        let inactive = becomes_empty
            .pane_runtimes
            .get(&owner)
            .expect("inactive Quick owner");
        assert!(
            inactive.ui.quick_select().is_none(),
            "empty retained result set must retire Quick Select"
        );

        let (mut malformed, owner, _, _, _) = quick_fixture(0);
        malformed
            .pane_runtimes
            .get_mut(&owner)
            .expect("inactive malformed Quick owner")
            .ui
            .quick_select_mut()
            .expect("Quick overlay")
            .labels
            .pop();
        malformed
            .handle_pane_pty_output(owner, b"\x1b[2;1HX")
            .unwrap();
        assert!(
            malformed
                .pane_runtimes
                .get(&owner)
                .is_some_and(|runtime| runtime.ui.quick_select().is_none()),
            "parallel-array invariant violation must retire rather than silently zip-truncate"
        );

        let (mut invalid_current, owner, _, _, _) = quick_fixture(0);
        invalid_current
            .pane_runtimes
            .get_mut(&owner)
            .expect("inactive invalid-current Quick owner")
            .ui
            .quick_select_mut()
            .expect("Quick overlay")
            .current = usize::MAX;
        invalid_current
            .handle_pane_pty_output(owner, b"\x1b[2;1HX")
            .unwrap();
        assert!(
            invalid_current
                .pane_runtimes
                .get(&owner)
                .is_some_and(|runtime| runtime.ui.quick_select().is_none()),
            "invalid current index must retire rather than select a survivor"
        );
    }

    #[test]
    fn window_app_ed3_preserves_unchanged_visible_selection() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"selected\r\nother").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 3 },
        );

        app.clear_scrollback();

        assert!(ordinary_selection_for_test(&app).is_some());
        assert_eq!(app.selected_text().as_deref(), Some("sele"));
    }

    #[test]
    fn window_app_config_palette_change_marks_all_pane_active_domains_changed() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"left\r\npane").unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"right\r\npane").unwrap();
        let active_before = app.runtime.terminal().current_seqno();
        let active_rows = app.runtime.terminal().retained_stable_range();
        let inactive = app
            .pane_runtimes
            .get(&rterm_core::PaneId::new(1))
            .expect("inactive pane runtime");
        let inactive_before = inactive.runtime.terminal().current_seqno();
        let inactive_rows = inactive.runtime.terminal().retained_stable_range();

        app.set_config_overrides(native_config_snapshot! {
            foreground_color: Some(Color::Rgb(1, 2, 3)),
            ..NativeConfigSnapshot::default()
        });

        assert_eq!(
            app.runtime
                .terminal()
                .changed_stable_rows_since(active_rows.clone(), active_before),
            active_rows.collect::<Vec<_>>()
        );
        let inactive = app
            .pane_runtimes
            .get(&rterm_core::PaneId::new(1))
            .expect("inactive pane runtime");
        assert_eq!(
            inactive
                .runtime
                .terminal()
                .changed_stable_rows_since(inactive_rows.clone(), inactive_before),
            inactive_rows.collect::<Vec<_>>()
        );
    }

    #[test]
    fn window_app_window_decoration_palette_changes_preserve_terminal_selection_and_seqno() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"selected\r\nother").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 3 },
        );
        let ordinary = ordinary_selection_for_test(&app);
        let rows = app.runtime.terminal().retained_stable_range();
        let before = app.runtime.terminal().current_seqno();
        let selection_background = Color::Rgb(1, 2, 3);

        app.set_config_overrides(native_config_snapshot! {
            selection_bg_color: Some(selection_background),
            tab_bar_background_color: Some(Color::Rgb(4, 5, 6)),
            scrollbar_thumb_color: Some(Color::Rgb(7, 8, 9)),
            ..NativeConfigSnapshot::default()
        });

        assert_eq!(app.runtime.terminal().current_seqno(), before);
        assert!(
            app.runtime
                .terminal()
                .changed_stable_rows_since(rows, before)
                .is_empty()
        );
        assert_eq!(ordinary_selection_for_test(&app), ordinary);
        assert_eq!(app.selection_bg_color, Some(selection_background));
        assert_eq!(
            rendered_active_pane_cell(&app, 0, 0).map(|cell| cell.background),
            Some(selection_background)
        );
        let palette = app.native_resolved_palette();
        assert_eq!(palette.tab_bar_background, Some(Color::Rgb(4, 5, 6)));
        assert_eq!(palette.scrollbar_thumb, Some(Color::Rgb(7, 8, 9)));
    }

    #[derive(Debug, Clone, Copy)]
    enum PaneSwitchOverlay {
        Search,
        Copy,
        Quick,
    }

    fn assert_dirty_ordinary_selection_is_deferred_across_pane_switch(overlay: PaneSwitchOverlay) {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 2));
        app.handle_pty_output(b"https://selected.test\r\nother")
            .unwrap();
        let selection_background = Color::Rgb(255, 0, 255);
        app.selection_bg_color = Some(selection_background);
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 3 },
        );
        assert_eq!(
            rendered_active_pane_cell(&app, 0, 0).map(|cell| cell.background),
            Some(selection_background),
            "{overlay:?} test requires an ordinary highlight before overlay entry"
        );

        match overlay {
            PaneSwitchOverlay::Search => app.enter_search_mode(),
            PaneSwitchOverlay::Copy => app.enter_copy_mode(),
            PaneSwitchOverlay::Quick => app.enter_quick_select_mode(),
        }
        app.handle_pty_output(b"\x1b[1;1HX").unwrap();
        assert!(
            ordinary_selection_for_test(&app).is_some(),
            "{overlay:?} should defer invalidation while active"
        );

        let original_pane = app.active_pane_id();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: original_pane,
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        let inactive = app
            .pane_runtimes
            .get(&original_pane)
            .expect("original pane should become inactive");
        assert!(
            inactive.ui.ordinary_selection.is_some(),
            "{overlay:?} must defer dirty ordinary selection while its overlay is saved"
        );
        assert!(inactive.ui.overlay_active(), "{overlay:?}");

        let original_rect = app
            .pane_render_layout()
            .panes
            .into_iter()
            .find(|rect| rect.pane_id == original_pane)
            .expect("original pane render rect");
        let terminal = inactive.runtime.terminal();
        let overlay_selection = super::pane_overlay_source_selection(
            terminal,
            &inactive.ui,
            &app.selection_word_boundary,
        )
        .and_then(|selection| {
            selection.viewport_selection(
                terminal.stable_dimensions().domain,
                super::pane_viewport_top(terminal, &inactive.ui),
                terminal.grid().size(),
            )
        });
        let snapshot = app.render_snapshot();
        if let Some(overlay_selection) = overlay_selection {
            let inside = (0..original_rect.rows)
                .flat_map(|row| (0..original_rect.columns).map(move |column| (row, column)))
                .find(|(row, column)| {
                    overlay_selection.contains(*row, *column, terminal.grid().size())
                })
                .expect("owner overlay must intersect its visible pane rect");
            let outside = (0..original_rect.rows)
                .flat_map(|row| (0..original_rect.columns).map(move |column| (row, column)))
                .find(|(row, column)| {
                    !overlay_selection.contains(*row, *column, terminal.grid().size())
                })
                .expect("fixture must include a visible cell outside the owner overlay");
            let overlay_cell = snapshot_cell(
                &snapshot,
                original_rect.row.saturating_add(inside.0),
                original_rect.column.saturating_add(inside.1),
            )
            .expect("inactive owner overlay cell");
            let outside_cell = snapshot_cell(
                &snapshot,
                original_rect.row.saturating_add(outside.0),
                original_rect.column.saturating_add(outside.1),
            )
            .expect("inactive owner non-overlay cell");
            assert_ne!(
                overlay_cell.background, outside_cell.background,
                "{overlay:?} must render the saved owner overlay instead of promoting deferred ordinary state"
            );
        } else {
            let former_ordinary_cell =
                snapshot_cell(&snapshot, original_rect.row, original_rect.column)
                    .expect("former ordinary selection cell");
            let outside_cell = snapshot_cell(
                &snapshot,
                original_rect.row.saturating_add(1),
                original_rect.column,
            )
            .expect("inactive owner non-overlay cell");
            assert_eq!(
                former_ordinary_cell.background, outside_cell.background,
                "{overlay:?} without a projection must suppress deferred ordinary presentation"
            );
        }
    }

    #[test]
    fn window_app_pane_switch_defers_search_copy_and_quick_dirty_ordinary_selection() {
        for overlay in [
            PaneSwitchOverlay::Search,
            PaneSwitchOverlay::Copy,
            PaneSwitchOverlay::Quick,
        ] {
            assert_dirty_ordinary_selection_is_deferred_across_pane_switch(overlay);
        }
    }

    #[test]
    fn window_app_search_overlay_defers_real_dirty_selection_invalidation() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 2));
        app.handle_pty_output(b"selected\r\nother").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 3 },
        );
        let ordinary = ordinary_selection_for_test(&app);
        app.enter_search_mode();
        assert!(search_for_test(&app).is_some());
        assert_app_search_mode(&app);
        assert!(quick_select_for_test(&app).is_none());

        app.handle_pty_output(b"\x1b[1;1HX").unwrap();
        assert_eq!(ordinary_selection_for_test(&app), ordinary);

        app.exit_search_mode();
        app.refresh_snapshot();
        assert!(ordinary_selection_for_test(&app).is_none());
    }

    #[test]
    fn window_app_copy_mode_overlay_defers_real_dirty_selection_invalidation() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 2));
        app.handle_pty_output(b"selected\r\nother").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 3 },
        );
        let ordinary = ordinary_selection_for_test(&app);
        app.enter_copy_mode();
        assert!(copy_mode_for_test(&app).is_some());
        assert!(search_for_test(&app).is_none());
        assert!(quick_select_for_test(&app).is_none());

        app.handle_pty_output(b"\x1b[1;1HX").unwrap();
        assert_eq!(ordinary_selection_for_test(&app), ordinary);

        app.exit_copy_mode();
        assert!(ordinary_selection_for_test(&app).is_none());
    }

    #[test]
    fn window_app_quick_select_overlay_defers_real_dirty_selection_invalidation() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(32, 2));
        app.handle_pty_output(b"https://selected.test\r\nother")
            .unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 3 },
        );
        let ordinary = ordinary_selection_for_test(&app);
        app.enter_quick_select_mode();
        assert!(quick_select_for_test(&app).is_some());
        assert!(search_for_test(&app).is_none());
        assert!(copy_mode_for_test(&app).is_none());

        app.handle_pty_output(b"\x1b[1;1HX").unwrap();
        assert_eq!(ordinary_selection_for_test(&app), ordinary);

        app.exit_quick_select_mode();
        assert!(ordinary_selection_for_test(&app).is_none());
    }

    #[test]
    fn window_app_presentation_selection_is_never_promoted_to_ordinary_storage() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"ordinary").unwrap();
        app.selection = Some(WindowSelection::new(
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 3 },
        ));

        app.refresh_snapshot();
        app.handle_pty_output(b"!").unwrap();

        assert!(ordinary_selection_for_test(&app).is_none());
        assert!(app.selected_text().is_none());
    }

    #[test]
    fn window_source_selection_same_row_width_shrink_does_not_retarget_right_edge() {
        let source = WindowSourceSelection::new(
            SelectionSourceCell {
                domain: TerminalScreenDomain::Main,
                row: 0,
                column: 6,
            },
            SelectionSourceCell {
                domain: TerminalScreenDomain::Main,
                row: 0,
                column: 7,
            },
        );

        assert!(
            source
                .viewport_selection(
                    TerminalScreenDomain::Main,
                    0,
                    rterm_core::TerminalSize::new(8, 1),
                )
                .is_some()
        );
        assert_eq!(
            source.viewport_selection(
                TerminalScreenDomain::Main,
                0,
                rterm_core::TerminalSize::new(7, 1),
            ),
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 6 },
                SelectionCell { row: 0, column: 6 },
            ))
        );
        assert_eq!(
            source.viewport_selection(
                TerminalScreenDomain::Main,
                0,
                rterm_core::TerminalSize::new(4, 1),
            ),
            None
        );
    }

    #[test]
    fn window_source_rectangular_selection_width_shrink_does_not_retarget_right_edge() {
        let source = WindowSourceSelection::rectangular(
            SelectionSourceCell {
                domain: TerminalScreenDomain::Main,
                row: 0,
                column: 6,
            },
            SelectionSourceCell {
                domain: TerminalScreenDomain::Main,
                row: 1,
                column: 7,
            },
        );

        assert!(
            source
                .viewport_selection(
                    TerminalScreenDomain::Main,
                    0,
                    rterm_core::TerminalSize::new(8, 2),
                )
                .is_some()
        );
        assert_eq!(
            source.viewport_selection(
                TerminalScreenDomain::Main,
                0,
                rterm_core::TerminalSize::new(7, 2),
            ),
            Some(WindowSelection::rectangular(
                SelectionCell { row: 0, column: 6 },
                SelectionCell { row: 1, column: 6 },
            ))
        );
        assert_eq!(
            source.viewport_selection(
                TerminalScreenDomain::Main,
                0,
                rterm_core::TerminalSize::new(4, 2),
            ),
            None
        );
    }

    #[test]
    fn window_app_search_current_selection_prefers_ordinary_over_transient_projection() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(24, 1));
        app.handle_pty_output(b"ordinary transient").unwrap();
        let dimensions = app.runtime.terminal().stable_dimensions();
        let anchor = SelectionSourceCell {
            domain: dimensions.domain,
            row: dimensions.physical_top,
            column: 0,
        };
        set_ordinary_stable_selection_for_test(
            &mut app,
            anchor,
            SelectionSourceCell {
                column: 7,
                ..anchor
            },
            false,
        );
        set_app_quick_select_for_test(
            &mut app,
            WindowQuickSelect {
                matches: vec![WindowSearchMatch {
                    domain: dimensions.domain,
                    source_row: dimensions.physical_top,
                    start_column: 9,
                    end_source_row: dimensions.physical_top,
                    end_column: 17,
                }],
                labels: vec!["a".to_owned()],
                ..WindowQuickSelect::default()
            },
        );
        app.update_selection_projection();
        assert_eq!(app.selected_text().as_deref(), Some("transient"));

        app.enter_search_mode_with_query(&WindowSearchCommandQuery::CurrentSelectionOrEmptyString);

        assert_eq!(
            search_for_test(&app).map(|search| search.query.as_str()),
            Some("ordinary")
        );
    }

    #[test]
    fn window_app_ordinary_selection_survives_scrolling_out_and_back() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc").unwrap();
        let selected = ordinary_source_cell_for_viewport(&app, 1, 0);
        set_ordinary_stable_selection_for_test(
            &mut app,
            selected,
            SelectionSourceCell {
                column: 1,
                ..selected
            },
            false,
        );

        app.scroll_viewport_lines(1);
        assert!(app.selection.is_none());
        assert_eq!(app.selected_text().as_deref(), Some("cc"));

        app.scroll_viewport_lines(-1);
        assert!(app.selection.is_some());
        assert_eq!(app.selected_text().as_deref(), Some("cc"));
    }

    #[test]
    fn window_app_wheel_keeps_ordinary_selection_while_viewport_moves() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc").unwrap();
        let selected = ordinary_source_cell_for_viewport(&app, 0, 0);
        set_ordinary_stable_selection_for_test(
            &mut app,
            selected,
            SelectionSourceCell {
                column: 1,
                ..selected
            },
            false,
        );
        let ordinary = ordinary_selection_for_test(&app);

        assert!(app.handle_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0)));

        assert_eq!(ordinary_selection_for_test(&app), ordinary);
        assert_eq!(app.selected_text().as_deref(), Some("bb"));
    }

    #[test]
    fn window_app_page_scroll_keeps_ordinary_selection_while_viewport_moves() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        let selected = ordinary_source_cell_for_viewport(&app, 1, 0);
        set_ordinary_stable_selection_for_test(
            &mut app,
            selected,
            SelectionSourceCell {
                column: 1,
                ..selected
            },
            false,
        );
        let ordinary = ordinary_selection_for_test(&app);

        app.command_palette_execute(WindowCommand::ScrollPageUp);

        assert_eq!(ordinary_selection_for_test(&app), ordinary);
        assert_eq!(app.selected_text().as_deref(), Some("ee"));
    }

    #[test]
    fn window_app_scrollbar_drag_keeps_ordinary_selection_while_viewport_moves() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            enable_scroll_bar: Some(true),
            ..NativeConfigSnapshot::default()
        });
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        let selected = ordinary_source_cell_for_viewport(&app, 1, 0);
        set_ordinary_stable_selection_for_test(
            &mut app,
            selected,
            SelectionSourceCell {
                column: 1,
                ..selected
            },
            false,
        );
        let ordinary = ordinary_selection_for_test(&app);

        assert!(app.scroll_to_scrollbar_position(PhysicalPosition::new(
            f64::from(FRAME_WIDTH - 1),
            f64::from(tab_bar_pixel_height()),
        )));

        assert_eq!(ordinary_selection_for_test(&app), ordinary);
        assert_eq!(app.selected_text().as_deref(), Some("ee"));
    }

    #[test]
    fn window_app_ordinary_selection_copies_offscreen_stable_text() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc").unwrap();
        let dimensions = app.runtime.terminal().stable_dimensions();
        let selected = SelectionSourceCell {
            domain: dimensions.domain,
            row: dimensions.scrollback_top,
            column: 0,
        };
        set_ordinary_stable_selection_for_test(
            &mut app,
            selected,
            SelectionSourceCell {
                column: 1,
                ..selected
            },
            false,
        );

        assert!(app.selection.is_none());
        assert_eq!(app.selected_text().as_deref(), Some("aa"));
    }

    #[test]
    fn window_app_ordinary_selection_survives_full_screen_scroll_into_history() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb").unwrap();
        let selected = ordinary_source_cell_for_viewport(&app, 0, 0);
        set_ordinary_stable_selection_for_test(
            &mut app,
            selected,
            SelectionSourceCell {
                column: 1,
                ..selected
            },
            false,
        );

        app.handle_pty_output(b"\r\ncc\r\ndd").unwrap();

        assert_eq!(app.selected_text().as_deref(), Some("aa"));
        assert!(ordinary_selection_for_test(&app).is_some());
    }

    #[test]
    fn window_app_ordinary_selection_partial_prune_copies_only_surviving_rows() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.runtime.set_scrollback_limit(3);
        app.handle_pty_output(b"aa\r\nbb\r\ncc").unwrap();
        let retained = app.runtime.terminal().retained_stable_range();
        let anchor = SelectionSourceCell {
            domain: app.runtime.terminal().stable_dimensions().domain,
            row: retained.start,
            column: 1,
        };
        let focus = SelectionSourceCell {
            row: retained.end - 1,
            column: 1,
            ..anchor
        };
        set_ordinary_stable_selection_for_test(&mut app, anchor, focus, false);

        app.runtime.set_scrollback_limit(1);
        app.handle_pty_output(b"\r\ndd").unwrap();

        assert_eq!(app.selected_text().as_deref(), Some("bb\ncc"));
    }

    #[test]
    fn window_app_ordinary_rectangular_selection_keeps_columns_after_prune() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.runtime.set_scrollback_limit(3);
        app.handle_pty_output(b"abcd\r\nefgh\r\nijkl").unwrap();
        let retained = app.runtime.terminal().retained_stable_range();
        let anchor = SelectionSourceCell {
            domain: app.runtime.terminal().stable_dimensions().domain,
            row: retained.start,
            column: 1,
        };
        let focus = SelectionSourceCell {
            row: retained.end - 1,
            column: 2,
            ..anchor
        };
        set_ordinary_stable_selection_for_test(&mut app, anchor, focus, true);

        app.runtime.set_scrollback_limit(1);
        app.handle_pty_output(b"\r\nmnop").unwrap();

        assert_eq!(app.selected_text().as_deref(), Some("fg\njk"));
    }

    #[test]
    fn window_app_ordinary_soft_wrap_selection_uses_stable_rows() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"abcdef").unwrap();
        let anchor = ordinary_source_cell_for_viewport(&app, 0, 2);
        let focus = ordinary_source_cell_for_viewport(&app, 1, 1);
        set_ordinary_stable_selection_for_test(&mut app, anchor, focus, false);

        app.scroll_viewport_lines(1);

        assert_eq!(app.selected_text().as_deref(), Some("cdef"));
    }

    #[test]
    fn window_app_fully_pruned_selection_never_copies_new_oldest_row() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.runtime.set_scrollback_limit(1);
        app.handle_pty_output(b"aa\r\nbb\r\ncc").unwrap();
        let dimensions = app.runtime.terminal().stable_dimensions();
        let selected = SelectionSourceCell {
            domain: dimensions.domain,
            row: dimensions.scrollback_top,
            column: 0,
        };
        set_ordinary_stable_selection_for_test(
            &mut app,
            selected,
            SelectionSourceCell {
                column: 1,
                ..selected
            },
            false,
        );

        app.handle_pty_output(b"\r\ndd\r\nee").unwrap();

        assert!(app.selected_text().is_none());
        assert_ne!(
            app.runtime
                .terminal()
                .text_from_stable_selection(StableSelectionRange {
                    start: StableSelectionCoordinate {
                        domain: dimensions.domain,
                        row: app.runtime.terminal().retained_stable_range().start,
                        column: 0,
                    },
                    end: StableSelectionCoordinate {
                        domain: dimensions.domain,
                        row: app.runtime.terminal().retained_stable_range().start,
                        column: 1,
                    },
                    rectangular: false,
                })
                .as_deref(),
            Some("aa")
        );
    }

    #[test]
    fn window_app_multi_click_cache_uses_stable_rows() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"oldword\r\nnewword\r\nlastword")
            .unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            0.0,
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        app.handle_mouse_input(ElementState::Released, MouseButton::Left)
            .unwrap();
        app.scroll_viewport_lines(1);

        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert!(app.selecting);
        assert!(
            ordinary_selection_for_test(&app).is_some_and(StableOrdinarySelection::is_single_cell)
        );
    }

    #[test]
    fn window_app_pane_switch_resets_multi_click_cache_for_same_stable_cell() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.handle_pty_output(b"same").unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            1.0,
            f64::from(tab_bar_pixel_height()) + 1.0,
        ))
        .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        app.handle_mouse_input(ElementState::Released, MouseButton::Left)
            .unwrap();
        let first_click = app
            .last_left_click
            .expect("first pane click should be cached");

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"same").unwrap();
        let active_pane = app.active_pane_id();
        let active_rect = app
            .pane_render_layout()
            .panes
            .into_iter()
            .find(|pane| pane.pane_id == active_pane)
            .expect("active pane render rect");
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(u32::from(active_rect.column) * CELL_WIDTH) + 1.0,
            f64::from(u32::from(active_rect.row) * CELL_HEIGHT) + 1.0,
        ))
        .unwrap();
        let second_cell = app
            .selection_source_cell_from_mouse_position()
            .expect("second pane source cell");
        assert_eq!(second_cell.domain, first_click.cell.domain);
        assert_eq!(second_cell.row, first_click.cell.row);

        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert!(app.selecting);
        assert!(
            ordinary_selection_for_test(&app).is_some_and(StableOrdinarySelection::is_single_cell)
        );
        assert_eq!(app.last_left_click.map(|click| click.count), Some(1));
    }

    #[test]
    fn window_app_focus_switch_restores_each_stable_selection() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"left\r\nhistory\r\nlive").unwrap();
        let left = SelectionSourceCell {
            domain: app.runtime.terminal().stable_dimensions().domain,
            row: app.runtime.terminal().retained_stable_range().start,
            column: 0,
        };
        set_ordinary_stable_selection_for_test(
            &mut app,
            left,
            SelectionSourceCell { column: 3, ..left },
            false,
        );

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"right").unwrap();
        let right = ordinary_source_cell_for_viewport(&app, 0, 0);
        set_ordinary_stable_selection_for_test(
            &mut app,
            right,
            SelectionSourceCell { column: 4, ..right },
            false,
        );

        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rterm_core::PaneId::new(1),
        })
        .unwrap();
        assert_eq!(app.selected_text().as_deref(), Some("left"));
        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rterm_core::PaneId::new(2),
        })
        .unwrap();
        assert_eq!(app.selected_text().as_deref(), Some("right"));
    }

    #[test]
    fn window_app_new_split_starts_without_stable_selection() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"source").unwrap();
        let selected = ordinary_source_cell_for_viewport(&app, 0, 0);
        set_ordinary_stable_selection_for_test(
            &mut app,
            selected,
            SelectionSourceCell {
                column: 5,
                ..selected
            },
            false,
        );

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        assert!(ordinary_selection_for_test(&app).is_none());
        assert!(
            app.pane_runtimes
                .get(&rterm_core::PaneId::new(1))
                .is_some_and(|runtime| runtime.ui.ordinary_selection.is_some())
        );
    }

    #[test]
    fn window_app_close_removes_only_closed_stable_selection() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"left").unwrap();
        let left = ordinary_source_cell_for_viewport(&app, 0, 0);
        set_ordinary_stable_selection_for_test(
            &mut app,
            left,
            SelectionSourceCell { column: 3, ..left },
            false,
        );
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"right").unwrap();
        let right = ordinary_source_cell_for_viewport(&app, 0, 0);
        set_ordinary_stable_selection_for_test(
            &mut app,
            right,
            SelectionSourceCell { column: 4, ..right },
            false,
        );

        app.dispatch_app_action(AppAction::ClosePane {
            pane: rterm_core::PaneId::new(2),
        })
        .unwrap();

        assert_eq!(app.selected_text().as_deref(), Some("left"));
        assert!(!app.pane_runtimes.contains_key(&rterm_core::PaneId::new(2)));
    }

    #[test]
    fn window_app_move_to_new_tab_preserves_stable_selection_and_viewport() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc").unwrap();
        app.scroll_viewport_lines(1);
        let selected = ordinary_source_cell_for_viewport(&app, 0, 0);
        set_ordinary_stable_selection_for_test(
            &mut app,
            selected,
            SelectionSourceCell {
                column: 1,
                ..selected
            },
            false,
        );
        let top = app.current_stable_viewport_top();
        let ordinary = ordinary_selection_for_test(&app);

        app.dispatch_app_action(AppAction::MovePaneToNewTab {
            pane: rterm_core::PaneId::new(2),
        })
        .unwrap();

        assert_eq!(app.current_stable_viewport_top(), top);
        assert_eq!(ordinary_selection_for_test(&app), ordinary);
        assert_eq!(app.selected_text().as_deref(), Some("aa"));
    }

    #[test]
    fn window_app_move_to_new_window_clears_gui_selection_but_preserves_stable_viewport() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc").unwrap();
        app.scroll_viewport_lines(1);
        let selected = ordinary_source_cell_for_viewport(&app, 0, 0);
        set_ordinary_stable_selection_for_test(
            &mut app,
            selected,
            SelectionSourceCell {
                column: 1,
                ..selected
            },
            false,
        );

        app.dispatch_app_action(AppAction::MovePaneToNewWindow {
            pane: rterm_core::PaneId::new(2),
        })
        .unwrap();
        let detached = app
            .take_next_pending_window_app()
            .expect("pane should materialize as a detached window");

        assert!(detached.active_ui.ordinary_selection.is_none());
        assert!(detached.selection.is_none());
        assert_eq!(detached.current_scrollback_offset(), 1);
        assert!(!detached.runtime.terminal().scrollback().is_empty());
    }

    #[test]
    fn window_app_transient_match_never_becomes_ordinary_stable_selection() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 1));
        app.handle_pty_output(b"transient").unwrap();
        let dimensions = app.runtime.terminal().stable_dimensions();
        set_app_quick_select_for_test(
            &mut app,
            WindowQuickSelect {
                matches: vec![WindowSearchMatch {
                    domain: dimensions.domain,
                    source_row: dimensions.physical_top,
                    start_column: 0,
                    end_source_row: dimensions.physical_top,
                    end_column: 8,
                }],
                labels: vec!["a".to_owned()],
                ..WindowQuickSelect::default()
            },
        );
        app.update_selection_projection();
        assert!(app.selection.is_some());
        assert!(ordinary_selection_for_test(&app).is_none());

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        assert!(
            app.pane_runtimes
                .get(&rterm_core::PaneId::new(1))
                .is_some_and(|runtime| runtime.ui.ordinary_selection.is_none())
        );
    }

    #[test]
    fn window_app_main_viewport_restores_after_alternate_screen() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc").unwrap();
        app.scroll_viewport_lines(1);
        let main_top = app.current_stable_viewport_top();

        app.handle_pty_output(b"\x1b[?1049halt").unwrap();
        assert_eq!(app.current_stable_viewport_top(), None);

        app.handle_pty_output(b"\x1b[?1049l").unwrap();
        assert_eq!(app.current_stable_viewport_top(), main_top);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 4), "aa  ");
    }

    #[test]
    fn window_app_active_height_change_retires_selection_before_copy() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"selected\r\nother").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 3 },
        );

        app.handle_window_resize(PhysicalSize::new(96, 80)).unwrap();

        assert!(ordinary_selection_for_test(&app).is_none());
        assert!(app.selected_text().is_none());
    }

    #[test]
    fn window_app_inactive_height_change_retires_selection_before_focus() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"left\r\npane").unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        let inactive = app
            .pane_runtimes
            .get_mut(&rterm_core::PaneId::new(1))
            .expect("inactive pane runtime");
        let dimensions = inactive.runtime.terminal().stable_dimensions();
        inactive.ui.ordinary_selection = Some(StableOrdinarySelection::new(
            SelectionSourceCell {
                domain: dimensions.domain,
                row: dimensions.physical_top,
                column: 0,
            },
            SelectionSourceCell {
                domain: dimensions.domain,
                row: dimensions.physical_top,
                column: 2,
            },
            inactive.runtime.terminal().current_seqno(),
        ));

        app.handle_window_resize(PhysicalSize::new(96, 80)).unwrap();

        assert!(
            app.pane_runtimes
                .get(&rterm_core::PaneId::new(1))
                .is_some_and(|runtime| runtime.ui.ordinary_selection.is_none())
        );
    }

    #[test]
    fn window_app_inactive_screen_or_height_change_retires_only_owner_ui_state() {
        struct FailingWriter;

        impl Write for FailingWriter {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("intentional inactive writer failure"))
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let mut domain = NativeWindowApp::new(None);
        domain.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        domain.enter_search_mode();
        assert!(!domain.update_search_query("inactive-domain-owner"));
        let owner = domain.active_pane_id();
        domain
            .dispatch_app_action(AppAction::SplitPane {
                pane: owner,
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        domain.enter_search_mode();
        assert!(!domain.update_search_query("active-domain-owner"));
        {
            let inactive = domain
                .pane_runtimes
                .get_mut(&owner)
                .expect("inactive domain owner");
            inactive.runtime.set_enq_answerback("answer");
            inactive.writer = Some(Box::new(FailingWriter));
        }

        assert!(
            domain
                .handle_pane_pty_output(owner, b"\x1b[?1049h\x05")
                .is_err(),
            "fixture must exit through a fallible inactive response write"
        );

        let inactive = domain
            .pane_runtimes
            .get(&owner)
            .expect("inactive domain owner after error");
        assert_eq!(
            inactive.runtime.terminal().stable_dimensions().domain,
            TerminalScreenDomain::Alternate
        );
        assert!(
            !inactive.ui.overlay_active(),
            "identity retirement must precede fallible writer/callback paths"
        );
        assert_eq!(
            search_for_test(&domain).map(|search| search.query.as_str()),
            Some("active-domain-owner"),
            "inactive failure must not retire the active owner's UI"
        );

        let mut height = NativeWindowApp::new(None);
        height.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        height.enter_search_mode();
        assert!(!height.update_search_query("inactive-height-owner"));
        let owner = height.active_pane_id();
        height
            .dispatch_app_action(AppAction::SplitPane {
                pane: owner,
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        height.runtime.resize(rterm_core::TerminalSize::new(12, 4));
        height.enter_search_mode();
        assert!(!height.update_search_query("active-height-owner"));
        height
            .pane_runtimes
            .get_mut(&owner)
            .expect("inactive height owner")
            .runtime
            .resize(rterm_core::TerminalSize::new(12, 2));

        height
            .handle_window_resize(PhysicalSize::new(96, 98))
            .unwrap();

        assert!(
            height
                .pane_runtimes
                .get(&owner)
                .is_some_and(|runtime| !runtime.ui.overlay_active()),
            "only the inactive runtime changes height in this fixture"
        );
        assert_eq!(
            search_for_test(&height).map(|search| search.query.as_str()),
            Some("active-height-owner"),
            "same-height active owner must survive another pane's identity change"
        );
    }

    #[test]
    fn window_app_active_screen_or_height_change_retires_owner_ui_state() {
        let mut domain = NativeWindowApp::new(None);
        domain.enter_search_mode();
        assert!(!domain.update_search_query("active-domain"));
        domain.handle_pty_output(b"\x1b[?1049h").unwrap();
        assert!(!overlay_active_for_test(&domain));

        let mut roundtrip = NativeWindowApp::new(None);
        roundtrip.enter_search_mode();
        assert!(!roundtrip.update_search_query("same-chunk-roundtrip"));
        roundtrip
            .handle_pty_output(b"\x1b[?1049halt\x1b[?1049l")
            .unwrap();
        assert_eq!(
            roundtrip.runtime.terminal().stable_dimensions().domain,
            TerminalScreenDomain::Main
        );
        assert!(
            !overlay_active_for_test(&roundtrip),
            "same-chunk main-to-alt-to-main must still retire the original identity"
        );

        let mut height = NativeWindowApp::new(None);
        height.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        height.enter_search_mode();
        assert!(!height.update_search_query("active-height"));
        height
            .handle_window_resize(PhysicalSize::new(64, 80))
            .unwrap();
        assert!(!overlay_active_for_test(&height));

        let mut reset = NativeWindowApp::new(None);
        reset.handle_pty_output(b"before-reset").unwrap();
        reset.enter_search_mode();
        assert!(!reset.update_search_query("active-reset"));
        reset.reset_terminal().unwrap();
        assert!(!overlay_active_for_test(&reset));
        assert!(reset.selection.is_none());
    }

    #[test]
    fn window_app_width_only_resize_preserves_active_and_inactive_overlays() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 4));
        app.enter_search_mode();
        assert!(!app.update_search_query("inactive-width-owner"));
        let owner = app.active_pane_id();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: owner,
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.runtime.resize(rterm_core::TerminalSize::new(8, 4));
        app.enter_search_mode();
        assert!(!app.update_search_query("active-width-owner"));

        app.handle_window_resize(PhysicalSize::new(96, 98)).unwrap();

        assert_eq!(
            app.runtime.terminal().grid().size(),
            rterm_core::TerminalSize::new(10, 4)
        );
        assert_eq!(
            search_for_test(&app).map(|search| search.query.as_str()),
            Some("active-width-owner")
        );
        assert_eq!(
            app.pane_runtimes
                .get(&owner)
                .and_then(|runtime| runtime.ui.retained_search())
                .map(|search| search.query.as_str()),
            Some("inactive-width-owner")
        );
    }

    #[test]
    fn window_app_main_reflow_clears_active_ordinary_selection_and_resets_viewport() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 4));
        app.handle_pty_output(b"row-0\r\nrow-1\r\nselected\r\nrow-3\r\nlive")
            .unwrap();
        app.scroll_viewport_lines(99);
        assert!(app.current_scrollback_offset() > 0);
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 3 },
        );

        app.handle_window_resize(PhysicalSize::new(48, 80)).unwrap();

        assert_eq!(
            app.runtime.terminal().grid().size(),
            rterm_core::TerminalSize::new(5, 3)
        );
        assert!(ordinary_selection_for_test(&app).is_none());
        assert!(app.selection.is_none());
        assert_eq!(app.current_scrollback_offset(), 0);
    }

    #[test]
    fn window_app_main_reflow_clears_inactive_ordinary_selection_and_resets_viewport() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 4));
        app.handle_pty_output(b"row-0\r\nrow-1\r\nselected\r\nrow-3\r\nlive")
            .unwrap();
        let inactive_pane = app.active_pane_id();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: inactive_pane,
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        let inactive = app
            .pane_runtimes
            .get_mut(&inactive_pane)
            .expect("inactive pane runtime");
        inactive
            .ui
            .stable_viewport
            .set_scrollback_offset(inactive.runtime.terminal(), 99);
        assert!(inactive.ui.stable_viewport.main_top.is_some());
        let dimensions = inactive.runtime.terminal().stable_dimensions();
        inactive.ui.ordinary_selection = Some(StableOrdinarySelection::new(
            SelectionSourceCell {
                domain: dimensions.domain,
                row: dimensions.scrollback_top,
                column: 0,
            },
            SelectionSourceCell {
                domain: dimensions.domain,
                row: dimensions.scrollback_top,
                column: 3,
            },
            inactive.runtime.terminal().current_seqno(),
        ));

        app.handle_window_resize(PhysicalSize::new(48, 80)).unwrap();

        let inactive = app
            .pane_runtimes
            .get(&inactive_pane)
            .expect("inactive pane runtime after resize");
        assert!(inactive.ui.ordinary_selection.is_none());
        assert_eq!(inactive.ui.stable_viewport.main_top, None);
    }

    #[test]
    fn window_app_alternate_physical_resize_keeps_ordinary_selection_state() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 4));
        app.handle_pty_output(b"\x1b[?1049halt-row").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 2 },
        );

        app.handle_window_resize(PhysicalSize::new(48, 98)).unwrap();

        assert_eq!(
            app.runtime.terminal().stable_dimensions().domain,
            TerminalScreenDomain::Alternate
        );
        assert!(ordinary_selection_for_test(&app).is_some());
    }

    #[test]
    fn window_app_main_reflow_rebuilds_active_copy_search_and_quick_overlays() {
        let mut copy = NativeWindowApp::new(None);
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&copied);
        copy.clipboard_writer = Box::new(move |text: &str| {
            recorded.lock().unwrap().push(text.to_owned());
            true
        });
        copy.runtime.resize(rterm_core::TerminalSize::new(12, 2));
        copy.handle_pty_output(b"0123456789AB").unwrap();
        copy.enter_copy_mode();
        assert!(copy.set_copy_mode_cursor(0, 10));
        assert!(copy.set_copy_mode_selection_mode(super::WindowCopySelectionMode::Cell));
        assert!(copy.move_copy_mode_cursor(0, 1));
        assert_eq!(copy.selected_text().as_deref(), Some("AB"));

        copy.handle_window_resize(PhysicalSize::new(48, 54))
            .unwrap();

        let copy_mode = active_copy_mode_for_test(&copy);
        let dimensions = copy.runtime.terminal().stable_dimensions();
        assert_eq!(copy_mode.cursor, SelectionCell { row: 0, column: 0 });
        assert_eq!(
            copy_mode.source_cursor,
            SelectionSourceCell {
                domain: TerminalScreenDomain::Main,
                row: dimensions.physical_top,
                column: 0,
            }
        );
        assert!(copy_mode.anchor.is_none());
        assert!(copy_mode.source_anchor.is_none());
        assert_ne!(copy.selected_text().as_deref(), Some("AB"));
        copy.command_palette_apply_command(WindowCommand::Copy)
            .unwrap();
        assert!(
            !copied.lock().unwrap().iter().any(|text| text == "AB"),
            "Copy action must not return text selected from the pre-reflow physical cells"
        );

        let mut search = NativeWindowApp::new(None);
        search.runtime.resize(rterm_core::TerminalSize::new(12, 2));
        search.handle_pty_output(b"needle-012345").unwrap();
        search.enter_search_mode();
        assert!(search.update_search_query("needle"));
        let prior_match = active_search_for_test(&search)
            .current
            .expect("search initializes a current match");

        search
            .handle_window_resize(PhysicalSize::new(48, 54))
            .unwrap();

        let rebuilt_search = active_search_for_test(&search);
        assert_eq!(rebuilt_search.query, "needle");
        assert!(rebuilt_search.editing);
        assert_eq!(rebuilt_search.current, None);
        let rebuilt_matches = search
            .active_ui
            .cached_search_matches(search.runtime.terminal())
            .expect("reflow must rebuild the retained search results");
        assert!(!rebuilt_matches.is_empty());
        assert!(
            rebuilt_matches
                .iter()
                .all(|matched| matched.is_retained(search.runtime.terminal()))
        );
        assert!(!rebuilt_matches.contains(&prior_match));

        let mut quick = NativeWindowApp::new(None);
        quick.runtime.resize(rterm_core::TerminalSize::new(24, 2));
        quick
            .handle_pty_output(b"https://example.test/needle")
            .unwrap();
        quick.enter_quick_select_mode();
        let prior_matches = active_quick_select_for_test(&quick).matches.clone();
        assert!(!prior_matches.is_empty());

        quick
            .handle_window_resize(PhysicalSize::new(96, 54))
            .unwrap();

        let rebuilt_quick = active_quick_select_for_test(&quick);
        assert!(!rebuilt_quick.matches.is_empty());
        assert_eq!(rebuilt_quick.matches.len(), rebuilt_quick.labels.len());
        assert!(
            rebuilt_quick
                .matches
                .iter()
                .all(|matched| matched.is_retained(quick.runtime.terminal()))
        );
        assert!(
            rebuilt_quick
                .matches
                .iter()
                .all(|matched| !prior_matches.contains(matched))
        );
    }

    #[test]
    fn window_app_main_reflow_rebuilds_inactive_overlay_owner_and_skips_alternate() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(12, 2));
        app.handle_pty_output(b"needle-012345").unwrap();
        app.enter_search_mode();
        assert!(app.update_search_query("needle"));
        let inactive_pane = app.active_pane_id();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: inactive_pane,
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.handle_window_resize(PhysicalSize::new(48, 54)).unwrap();

        let inactive = app
            .pane_runtimes
            .get(&inactive_pane)
            .expect("inactive search owner");
        let inactive_search = inactive.ui.retained_search().expect("search remains open");
        assert_eq!(inactive_search.query, "needle");
        assert!(inactive_search.editing);
        assert_eq!(inactive_search.current, None);
        let inactive_matches = inactive
            .ui
            .cached_search_matches(inactive.runtime.terminal())
            .expect("inactive search must be rebuilt");
        assert!(!inactive_matches.is_empty());
        assert!(
            inactive_matches
                .iter()
                .all(|matched| matched.is_retained(inactive.runtime.terminal()))
        );

        let mut inactive_copy = NativeWindowApp::new(None);
        inactive_copy
            .runtime
            .resize(rterm_core::TerminalSize::new(12, 2));
        inactive_copy.handle_pty_output(b"0123456789AB").unwrap();
        inactive_copy.enter_copy_mode();
        assert!(inactive_copy.set_copy_mode_cursor(0, 10));
        assert!(inactive_copy.set_copy_mode_selection_mode(super::WindowCopySelectionMode::Cell));
        assert!(inactive_copy.move_copy_mode_cursor(0, 1));
        let inactive_copy_pane = inactive_copy.active_pane_id();
        inactive_copy
            .dispatch_app_action(AppAction::SplitPane {
                pane: inactive_copy_pane,
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();

        inactive_copy
            .handle_window_resize(PhysicalSize::new(48, 54))
            .unwrap();

        let inactive_copy_runtime = inactive_copy
            .pane_runtimes
            .get(&inactive_copy_pane)
            .expect("inactive Copy owner");
        let inactive_copy_mode = inactive_copy_runtime
            .ui
            .retained_copy_mode()
            .expect("Copy mode remains open");
        let inactive_copy_dimensions = inactive_copy_runtime.runtime.terminal().stable_dimensions();
        assert_eq!(
            inactive_copy_mode.cursor,
            SelectionCell { row: 0, column: 0 }
        );
        assert_eq!(
            inactive_copy_mode.source_cursor,
            SelectionSourceCell {
                domain: TerminalScreenDomain::Main,
                row: inactive_copy_dimensions.physical_top,
                column: 0,
            }
        );
        assert!(inactive_copy_mode.anchor.is_none());
        assert!(inactive_copy_mode.source_anchor.is_none());

        let mut inactive_quick = NativeWindowApp::new(None);
        inactive_quick
            .runtime
            .resize(rterm_core::TerminalSize::new(24, 2));
        inactive_quick
            .handle_pty_output(b"https://example.test/needle")
            .unwrap();
        inactive_quick.enter_quick_select_mode();
        let prior_inactive_quick_matches = active_quick_select_for_test(&inactive_quick)
            .matches
            .clone();
        let inactive_quick_pane = inactive_quick.active_pane_id();
        inactive_quick
            .dispatch_app_action(AppAction::SplitPane {
                pane: inactive_quick_pane,
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();

        inactive_quick
            .handle_window_resize(PhysicalSize::new(96, 54))
            .unwrap();

        let inactive_quick_runtime = inactive_quick
            .pane_runtimes
            .get(&inactive_quick_pane)
            .expect("inactive Quick owner");
        let rebuilt_inactive_quick = inactive_quick_runtime
            .ui
            .quick_select()
            .expect("Quick mode remains open");
        assert!(!rebuilt_inactive_quick.matches.is_empty());
        assert_eq!(
            rebuilt_inactive_quick.matches.len(),
            rebuilt_inactive_quick.labels.len()
        );
        assert!(
            rebuilt_inactive_quick
                .matches
                .iter()
                .all(|matched| matched.is_retained(inactive_quick_runtime.runtime.terminal()))
        );
        assert!(
            rebuilt_inactive_quick
                .matches
                .iter()
                .all(|matched| !prior_inactive_quick_matches.contains(matched))
        );

        let mut alternate = NativeWindowApp::new(None);
        alternate
            .runtime
            .resize(rterm_core::TerminalSize::new(12, 2));
        alternate
            .handle_pty_output(b"\x1b[?1049hneedle-alt")
            .unwrap();
        alternate.enter_search_mode();
        assert!(alternate.update_search_query("needle"));
        let prior_current = active_search_for_test(&alternate).current;

        alternate
            .handle_window_resize(PhysicalSize::new(48, 54))
            .unwrap();

        assert_eq!(
            alternate.runtime.terminal().stable_dimensions().domain,
            TerminalScreenDomain::Alternate
        );
        assert_eq!(active_search_for_test(&alternate).current, prior_current);
    }

    #[test]
    fn window_app_width_only_shrink_preserves_active_and_inactive_copy_owners() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(12, 4));
        app.handle_pty_output(b"0123456789AB").unwrap();
        app.enter_copy_mode();
        assert!(app.set_copy_mode_cursor(0, 10));
        assert!(app.set_copy_mode_selection_mode(super::WindowCopySelectionMode::Cell));
        assert!(app.move_copy_mode_cursor(0, 1));
        let inactive_owner = app.active_pane_id();
        assert_eq!(app.selected_text().as_deref(), Some("AB"));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: inactive_owner,
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.handle_pty_output(b"0123456789AB").unwrap();
        app.enter_copy_mode();
        assert!(app.set_copy_mode_cursor(0, 9));
        assert!(app.set_copy_mode_selection_mode(super::WindowCopySelectionMode::Cell));
        assert!(app.move_copy_mode_cursor(0, 2));
        assert_eq!(app.selected_text().as_deref(), Some("9AB"));

        app.handle_window_resize(PhysicalSize::new(48, 98)).unwrap();

        assert_eq!(
            app.runtime.terminal().grid().size(),
            rterm_core::TerminalSize::new(5, 4)
        );
        assert!(
            copy_mode_for_test(&app).is_some(),
            "Task 3 preserves the active Copy controller; Task 4 rebuilds its derived state"
        );

        let inactive = app
            .pane_runtimes
            .get(&inactive_owner)
            .expect("inactive Copy owner");
        assert!(
            inactive.ui.retained_copy_mode().is_some(),
            "Task 3 preserves the inactive Copy controller; Task 4 rebuilds its derived state"
        );

        app.handle_window_resize(PhysicalSize::new(96, 98)).unwrap();

        assert_eq!(
            app.runtime.terminal().grid().size(),
            rterm_core::TerminalSize::new(10, 4)
        );
        assert!(copy_mode_for_test(&app).is_some());

        let inactive = app
            .pane_runtimes
            .get(&inactive_owner)
            .expect("inactive Copy owner after expansion");
        assert!(inactive.ui.retained_copy_mode().is_some());
    }

    #[test]
    fn window_app_copy_projection_handles_zero_width_without_panicking() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"copy").unwrap();
        app.enter_copy_mode();
        assert!(app.set_copy_mode_cursor(1, 3));

        app.runtime.resize(rterm_core::TerminalSize::new(0, 2));
        app.reconcile_active_terminal_mutation();

        assert!(
            !overlay_active_for_test(&app),
            "direct runtime reflow must not present stale Copy coordinates"
        );

        app.runtime.resize(rterm_core::TerminalSize::new(0, 0));
        app.reconcile_active_terminal_mutation();
        assert!(
            !overlay_active_for_test(&app),
            "zero rows retain no stable Copy cursor identity"
        );
    }

    #[test]
    fn window_app_alternate_copy_projection_keeps_retained_cursor_identity() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"\x1b[?1049halt-0\r\nalt-1").unwrap();
        app.enter_copy_mode();
        assert!(app.set_copy_mode_cursor(1, 2));
        let source_cursor = active_copy_mode_for_test(&app).source_cursor;

        app.handle_pty_output(b"\x1b[1;1HX").unwrap();

        let copy = active_copy_mode_for_test(&app);
        assert_eq!(copy.source_cursor, source_cursor);
        assert_eq!(copy.cursor, SelectionCell { row: 1, column: 2 });
        assert_eq!(
            app.active_ui
                .stable_viewport
                .active_top(app.runtime.terminal()),
            None,
            "alternate screen must not synthesize a main viewport"
        );
    }

    #[test]
    fn window_app_search_mode_drives_active_and_inactive_viewports_before_copy_resumes() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(12, 2));
        app.set_config_overrides(native_config_snapshot! {
            copy_mode_active_highlight_bg: Some(NativeColorSpec::Color(Color::Rgb(1, 2, 3))),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"copy-row\r\nmid-1\r\nmid-2\r\nmid-3\r\nneedle-row\r\nlive")
            .unwrap();
        app.enter_copy_mode();
        assert!(app.move_copy_mode_to_scrollback_top());
        assert!(app.set_copy_mode_cursor(0, 7));
        assert!(app.set_copy_mode_selection_mode(super::WindowCopySelectionMode::Cell));
        let copy_before = active_copy_mode_for_test(&app);
        let copy_source_cursor = copy_before.source_cursor;
        let copy_source_anchor = copy_before.source_anchor;
        let copy_selection_mode = copy_before.selection_mode;
        assert_eq!(copy_before.pending_jump, None);
        assert_eq!(copy_before.last_jump, None);
        assert_eq!(copy_before.search_direction, None);

        app.enter_search_mode();
        assert!(app.update_search_query("live"));
        let search_match = active_search_for_test(&app)
            .current
            .expect("retained search match");
        assert!(
            search_match
                .source_row
                .saturating_sub(copy_source_cursor.row)
                >= 3,
            "fixture keeps hidden Copy cursor and Search current on separate pages"
        );
        let search_top = app.current_viewport_stable_top();
        let expected_search_selection = search_match
            .viewport_selection_for_top(
                app.runtime.terminal().stable_dimensions().domain,
                search_top,
                app.runtime.terminal().grid().size(),
            )
            .expect("Search current projects in the search-driven viewport");
        assert_eq!(app.selection, Some(expected_search_selection));
        let copy = app
            .active_ui
            .retained_copy_mode()
            .expect("Search retains Copy state");
        assert_eq!(copy.source_cursor, copy_source_cursor);
        assert_eq!(copy.source_anchor, copy_source_anchor);
        assert_eq!(copy.selection_mode, copy_selection_mode);

        app.handle_pty_output(b"\x1b[2;12H!").unwrap();

        assert_eq!(
            app.current_viewport_stable_top(),
            search_top,
            "active mutation must keep Search current as viewport driver"
        );
        assert_eq!(
            active_search_for_test(&app).current,
            Some(search_match),
            "retained Search current survives active mutation"
        );
        assert_eq!(app.selection, Some(expected_search_selection));
        let copy = app
            .active_ui
            .retained_copy_mode()
            .expect("hidden Copy state survives active mutation");
        assert_eq!(copy.source_cursor, copy_source_cursor);
        assert_eq!(copy.source_anchor, copy_source_anchor);
        assert_eq!(copy.selection_mode, copy_selection_mode);
        assert_eq!(copy.pending_jump, None);
        assert_eq!(copy.last_jump, None);
        assert_eq!(copy.search_direction, None);

        let owner = app.active_pane_id();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: owner,
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pane_pty_output(owner, b"\x1b[2;11H?").unwrap();

        let inactive = app
            .pane_runtimes
            .get(&owner)
            .expect("inactive Search owner");
        let inactive_top = inactive
            .ui
            .stable_viewport
            .active_top(inactive.runtime.terminal())
            .unwrap_or(inactive.runtime.terminal().stable_dimensions().physical_top);
        assert_eq!(
            inactive_top, search_top,
            "inactive mutation must keep Search current as viewport driver"
        );
        let search = inactive
            .ui
            .search()
            .expect("inactive Search remains active");
        assert_eq!(search.current, Some(search_match));
        assert!(
            search_match
                .viewport_selection_for_top(
                    inactive.runtime.terminal().stable_dimensions().domain,
                    inactive_top,
                    inactive.runtime.terminal().grid().size(),
                )
                .is_some(),
            "inactive Search highlight remains projected"
        );
        let copy = inactive
            .ui
            .retained_copy_mode()
            .expect("inactive Search retains hidden Copy state");
        assert_eq!(copy.source_cursor, copy_source_cursor);
        assert_eq!(copy.source_anchor, copy_source_anchor);
        assert_eq!(copy.selection_mode, copy_selection_mode);

        app.dispatch_app_action(AppAction::ActivatePane { pane: owner })
            .unwrap();
        assert!(app.set_search_pattern_editing(false));

        assert_eq!(
            copy_search_mode_for_test(&app),
            Some(super::WindowCopySearchMode::Copy)
        );
        let copy_top = app.current_viewport_stable_top();
        let copy = active_copy_mode_for_test(&app);
        assert_eq!(copy.source_cursor, copy_source_cursor);
        assert_eq!(copy.source_anchor, copy_source_anchor);
        assert_eq!(copy.selection_mode, copy_selection_mode);
        let copy_cursor = copy.cursor;
        let expected_copy_selection = super::copy_mode_source_selection(
            copy,
            app.runtime.terminal(),
            &app.selection_word_boundary,
        )
        .and_then(|selection| {
            selection.viewport_selection(
                app.runtime.terminal().stable_dimensions().domain,
                copy_top,
                app.runtime.terminal().grid().size(),
            )
        })
        .expect("Copy source selection projects after AcceptPattern");
        assert_eq!(app.selection, Some(expected_copy_selection));
        assert_ne!(
            app.selection,
            Some(expected_search_selection),
            "active selection projection must switch away from Search immediately"
        );
        assert_eq!(
            copy_cursor,
            SelectionCell {
                row: u16::try_from(copy_source_cursor.row - copy_top).expect("visible Copy row"),
                column: u16::try_from(copy_source_cursor.column).expect("visible Copy column"),
            },
            "Copy cursor is reprojected immediately on AcceptPattern"
        );
        assert!(
            copy_source_cursor.row >= copy_top
                && copy_source_cursor.row
                    < copy_top
                        + StableRowIndex::try_from(app.runtime.terminal().grid().size().rows)
                            .unwrap(),
            "Copy cursor immediately resumes viewport ownership"
        );
        assert_eq!(
            app.active_ui
                .retained_search()
                .and_then(|search| search.current),
            Some(search_match),
            "AcceptPattern retains Search current for Copy/Search transitions"
        );

        app.dispatch_app_action(AppAction::SetPaneZoomState {
            pane: owner,
            zoomed: true,
        })
        .unwrap();
        assert!(
            pane_rect_for_test(&app, owner).columns > copy_cursor.column,
            "zoom must make the retained Copy column visible before presentation assertions"
        );
        assert_eq!(
            rendered_active_pane_cell(&app, copy_cursor.row, copy_cursor.column)
                .expect("Copy highlight cell after zoom")
                .background,
            Color::Rgb(1, 2, 3),
            "snapshot highlight must follow the resumed Copy selection"
        );
        assert_ne!(
            rendered_active_pane_cell(&app, expected_search_selection.anchor.row, 0)
                .expect("former Search highlight cell")
                .background,
            Color::Rgb(1, 2, 3),
            "snapshot must not retain the old Search highlight projection"
        );
    }

    #[test]
    fn window_app_raw_pty_reset_and_destructive_erase_retire_owner_ui_state() {
        for bytes in [b"\x1bc".as_slice(), b"\x1b[2J".as_slice()] {
            let mut active = NativeWindowApp::new(None);
            active.handle_pty_output(b"active-owner").unwrap();
            active.enter_search_mode();
            assert!(!active.update_search_query("raw-active"));
            active.handle_pty_output(bytes).unwrap();
            assert!(
                !overlay_active_for_test(&active),
                "raw active mutation {bytes:?}"
            );

            let mut inactive = NativeWindowApp::new(None);
            inactive.handle_pty_output(b"inactive-owner").unwrap();
            inactive.enter_search_mode();
            assert!(!inactive.update_search_query("raw-inactive"));
            let owner = inactive.active_pane_id();
            inactive
                .dispatch_app_action(AppAction::SplitPane {
                    pane: owner,
                    direction: SplitDirection::Right,
                    launch: None,
                })
                .unwrap();
            inactive.handle_pane_pty_output(owner, bytes).unwrap();
            assert!(
                inactive
                    .pane_runtimes
                    .get(&owner)
                    .is_some_and(|runtime| !runtime.ui.overlay_active()),
                "raw inactive mutation {bytes:?}"
            );
        }
    }

    #[test]
    fn window_app_forced_overlay_retirement_rechecks_deferred_ordinary_dirty_rows() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(32, 2));
        app.runtime.set_scrollback_limit(2);
        app.handle_pty_output(b"https://selected.test\r\nkeep\r\nlive")
            .unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 4 },
        );
        app.scroll_viewport_lines(1);
        assert_eq!(
            app.current_stable_viewport_top(),
            Some(
                app.runtime
                    .terminal()
                    .stable_dimensions()
                    .physical_top
                    .saturating_sub(1)
            )
        );
        app.enter_quick_select_mode();
        assert!(ordinary_selection_for_test(&app).is_some());
        assert!(overlay_active_for_test(&app));

        app.handle_pty_output(b"\x1b[1;1HX\r\nnew-1\r\nnew-2\r\nnew-3")
            .unwrap();

        assert!(
            !overlay_active_for_test(&app),
            "current Quick match must be retired by pruning"
        );
        assert!(
            ordinary_selection_for_test(&app).is_none(),
            "forced overlay exit must immediately re-evaluate accumulated ordinary dirty rows"
        );
        assert!(app.selection.is_none());
    }

    #[test]
    fn window_app_main_to_alt_retires_selection_before_projection() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"selected\r\nother").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 3 },
        );

        app.handle_pty_output(b"\x1b[?1049h").unwrap();

        assert!(ordinary_selection_for_test(&app).is_none());
        assert!(app.selection.is_none());
    }

    #[test]
    fn window_app_alt_to_main_does_not_revive_selection() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"selected\r\nother").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 3 },
        );

        app.handle_pty_output(b"\x1b[?1049h").unwrap();
        app.handle_pty_output(b"\x1b[?1049l").unwrap();

        assert!(ordinary_selection_for_test(&app).is_none());
        assert!(app.selection.is_none());
    }

    #[test]
    fn window_app_screen_switch_retires_transient_and_multiclick_state() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 2));
        app.handle_pty_output(b"https://test\r\nother").unwrap();
        app.enter_copy_mode();
        set_app_search_for_test(&mut app, WindowSearch::default());
        set_app_quick_select_for_test(&mut app, WindowQuickSelect::default());
        app.selecting = true;
        app.last_left_click = Some(WindowClick {
            cell: ordinary_source_cell_for_viewport(&app, 0, 0),
            time: Instant::now(),
            count: 2,
        });
        app.last_mouse_assignment_click = Some(WindowMouseAssignmentClick {
            button: MouseButton::Left,
            modifiers: ModifiersState::empty(),
            mouse_reporting: false,
            alternate_screen_active: false,
            time: Instant::now(),
            count: 2,
        });

        app.handle_pty_output(b"\x1b[?1049h").unwrap();

        assert!(search_for_test(&app).is_none());
        assert!(copy_mode_for_test(&app).is_none());
        assert!(quick_select_for_test(&app).is_none());
        assert!(!app.selecting);
        assert!(app.last_left_click.is_none());
        assert!(app.last_mouse_assignment_click.is_none());
    }

    #[test]
    fn window_app_main_viewport_restores_after_alt_selection_retirement() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd").unwrap();
        app.scroll_viewport_lines(2);
        let main_top = app.current_stable_viewport_top();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 1 },
        );

        app.handle_pty_output(b"\x1b[?1049h").unwrap();
        app.handle_pty_output(b"\x1b[?1049l").unwrap();

        assert_eq!(app.current_stable_viewport_top(), main_top);
        assert!(ordinary_selection_for_test(&app).is_none());
    }

    #[test]
    fn window_app_same_chunk_main_alt_main_retires_active_identity_state() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 2));
        app.handle_pty_output(b"old\r\nhttps://selected.test\r\nbottom")
            .unwrap();
        app.scroll_viewport_lines(1);
        let main_top = app.current_stable_viewport_top();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 3 },
        );
        app.enter_copy_mode();
        set_app_search_for_test(&mut app, WindowSearch::default());
        set_app_quick_select_for_test(&mut app, WindowQuickSelect::default());
        app.selecting = true;
        app.last_left_click = Some(WindowClick {
            cell: ordinary_source_cell_for_viewport(&app, 0, 0),
            time: Instant::now(),
            count: 2,
        });
        app.last_mouse_assignment_click = Some(WindowMouseAssignmentClick {
            button: MouseButton::Left,
            modifiers: ModifiersState::empty(),
            mouse_reporting: false,
            alternate_screen_active: false,
            time: Instant::now(),
            count: 2,
        });

        app.handle_pty_output(b"\x1b[?1049halt\x1b[?1049l").unwrap();

        assert_eq!(app.current_stable_viewport_top(), main_top);
        assert!(ordinary_selection_for_test(&app).is_none());
        assert!(app.selection.is_none());
        assert!(search_for_test(&app).is_none());
        assert!(copy_mode_for_test(&app).is_none());
        assert!(quick_select_for_test(&app).is_none());
        assert!(!app.selecting);
        assert!(app.last_left_click.is_none());
        assert!(app.last_mouse_assignment_click.is_none());
        assert!(app.selected_text().is_none());
    }

    #[test]
    fn window_app_same_chunk_main_alt_main_retires_inactive_identity_before_focus() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 2));
        app.handle_pty_output(b"old\r\nselected\r\nbottom").unwrap();
        app.scroll_viewport_lines(1);
        let inactive_top = app.current_stable_viewport_top();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 3 },
        );
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.handle_pane_pty_output(rterm_core::PaneId::new(1), b"\x1b[?1049halt\x1b[?1049l")
            .unwrap();

        let inactive = app
            .pane_runtimes
            .get(&rterm_core::PaneId::new(1))
            .expect("inactive pane runtime");
        assert_eq!(
            inactive
                .ui
                .stable_viewport
                .active_top(inactive.runtime.terminal()),
            inactive_top
        );
        assert!(inactive.ui.ordinary_selection.is_none());

        app.dispatch_app_action(AppAction::ActivatePaneByIndex { index: 0 })
            .unwrap();
        assert_eq!(app.current_stable_viewport_top(), inactive_top);
        assert!(ordinary_selection_for_test(&app).is_none());
        assert!(app.selection.is_none());
    }

    #[test]
    fn window_app_lua_pane_dimensions_use_stable_scrollback_top() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.runtime.set_scrollback_limit(1);
        app.handle_pty_output(b"zero\r\none\r\ntwo\r\nthree\r\nfour")
            .unwrap();
        let dimensions = app.runtime.terminal().stable_dimensions();
        assert!(dimensions.scrollback_top > 0);

        assert_eq!(
            app.lua_pane_dimensions_field_text(NativeLuaPaneDimensionsField::ScrollbackTop),
            dimensions.scrollback_top.to_string()
        );
    }

    #[test]
    fn window_app_lua_pane_dimensions_use_stable_physical_top() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.runtime.set_scrollback_limit(1);
        app.handle_pty_output(b"zero\r\none\r\ntwo\r\nthree\r\nfour")
            .unwrap();
        let dimensions = app.runtime.terminal().stable_dimensions();
        assert_ne!(
            dimensions.physical_top,
            StableRowIndex::try_from(app.runtime.terminal().scrollback().len()).unwrap()
        );

        assert_eq!(
            app.lua_pane_dimensions_field_text(NativeLuaPaneDimensionsField::PhysicalTop),
            dimensions.physical_top.to_string()
        );
    }

    #[test]
    fn window_app_lua_pane_cursor_y_uses_stable_row() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.runtime.set_scrollback_limit(1);
        app.handle_pty_output(b"zero\r\none\r\ntwo\r\nthree\r\nfour")
            .unwrap();
        let dimensions = app.runtime.terminal().stable_dimensions();
        let (row, _) = app.runtime.terminal().cursor();
        let stable_cursor_y = dimensions
            .physical_top
            .saturating_add(StableRowIndex::try_from(row).unwrap());

        assert_eq!(
            app.lua_pane_cursor_position_field_text(NativeLuaPaneCursorPositionField::Y),
            stable_cursor_y.to_string()
        );
    }

    #[test]
    fn window_app_prune_clamps_stable_viewport() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd").unwrap();
        app.scroll_viewport_lines(2);

        app.runtime.set_scrollback_limit(1);
        app.refresh_snapshot();

        assert_eq!(
            app.current_stable_viewport_top(),
            Some(app.runtime.terminal().stable_dimensions().scrollback_top)
        );
    }

    #[test]
    fn window_app_active_and_inactive_stable_viewports_are_independent() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 2));
        app.handle_pty_output(b"left-1\r\nleft-2\r\nleft-3\r\nleft-4\r\nleft-5\r\nleft-6")
            .unwrap();
        let left_physical_top = app.runtime.terminal().stable_dimensions().physical_top;
        app.scroll_viewport_lines(2);
        let left_top = app.current_stable_viewport_top();
        assert_eq!(left_top, Some(left_physical_top.saturating_sub(2)));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"right-1\r\nright-2\r\nright-3\r\nright-4")
            .unwrap();
        let right_physical_top = app.runtime.terminal().stable_dimensions().physical_top;
        app.scroll_viewport_lines(1);
        let right_top = app.current_stable_viewport_top();
        assert_eq!(right_top, Some(right_physical_top.saturating_sub(1)));
        assert_ne!(left_top, right_top);

        app.dispatch_app_action(AppAction::ActivatePaneByIndex { index: 0 })
            .unwrap();
        assert_eq!(app.current_stable_viewport_top(), left_top);

        app.dispatch_app_action(AppAction::ActivatePaneByIndex { index: 1 })
            .unwrap();
        assert_eq!(app.current_stable_viewport_top(), right_top);
    }

    #[test]
    fn window_app_copy_mode_cursor_survives_history_growth() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc").unwrap();
        app.enter_copy_mode();
        assert!(app.move_copy_mode_to_scrollback_top());
        let source_cursor = active_copy_mode_for_test(&app).source_cursor;

        app.handle_pty_output(b"\r\ndd\r\nee").unwrap();

        let retained_cursor = active_copy_mode_for_test(&app).source_cursor;
        assert_eq!(retained_cursor, source_cursor);
        assert_eq!(
            retained_cursor.domain,
            rterm_terminal::TerminalScreenDomain::Main
        );
    }

    #[test]
    fn window_app_search_matches_do_not_retarget_after_prune() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 2));
        app.runtime.set_scrollback_limit(2);
        app.handle_pty_output(b"needle-old\r\nkeep\r\nlive")
            .unwrap();
        app.enter_search_mode();
        assert!(app.update_search_query("needle-old"));
        let matched = active_search_for_test(&app).current.unwrap();
        assert_eq!(matched.domain, rterm_terminal::TerminalScreenDomain::Main);

        app.handle_pty_output(b"\r\nnew-1\r\nnew-2\r\nnew-3")
            .unwrap();

        assert!(search_for_test(&app).is_some_and(|search| search.current.is_none()));
        assert_ne!(
            app.runtime
                .terminal()
                .history_index_to_stable_row(0)
                .unwrap(),
            matched.source_row
        );
    }

    #[test]
    fn window_app_search_projection_tracks_stable_viewport() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 3));
        app.handle_pty_output(b"foo 0\r\nfoo 1\r\nfoo 2\r\nfoo 3\r\nfoo 4")
            .unwrap();
        assert!(app.update_search_query("foo"));
        assert!(app.step_search(SearchDirection::Previous));
        let matched = active_search_for_test(&app).current;
        assert_eq!(app.selection.unwrap().anchor.row, 1);

        app.scroll_viewport_lines(1);

        assert_eq!(active_search_for_test(&app).current, matched);
        assert_eq!(app.selection.unwrap().anchor.row, 2);
    }

    #[test]
    fn window_app_search_multiline_match_clips_across_stable_viewport_edges() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 3));
        app.handle_pty_output(b"row-0\r\nrow-1\r\nrow-2\r\nrow-3\r\nrow-4\r\nrow-5")
            .unwrap();
        let dimensions = app.runtime.terminal().stable_dimensions();
        let viewport_top = app.current_viewport_stable_top();
        let matched = WindowSearchMatch {
            domain: dimensions.domain,
            source_row: viewport_top.saturating_sub(1),
            start_column: 4,
            end_source_row: viewport_top.saturating_add(3),
            end_column: 2,
        };
        set_app_search_for_test(
            &mut app,
            WindowSearch {
                current: Some(matched),
                ..WindowSearch::default()
            },
        );

        app.update_transient_selection_projection();

        let selection = app.selection.expect("projected search selection");
        assert_eq!(selection.anchor, SelectionCell { row: 0, column: 0 });
        assert_eq!(selection.focus, SelectionCell { row: 2, column: 7 });
        assert!(selection.anchor.row <= selection.focus.row);
    }

    #[test]
    fn window_app_quick_select_multiline_match_clips_across_stable_viewport_edges() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 3));
        app.handle_pty_output(b"row-0\r\nrow-1\r\nrow-2\r\nrow-3\r\nrow-4\r\nrow-5")
            .unwrap();
        let dimensions = app.runtime.terminal().stable_dimensions();
        let viewport_top = app.current_viewport_stable_top();
        let matched = WindowSearchMatch {
            domain: dimensions.domain,
            source_row: viewport_top.saturating_sub(1),
            start_column: 5,
            end_source_row: viewport_top.saturating_add(3),
            end_column: 1,
        };
        set_app_quick_select_for_test(
            &mut app,
            WindowQuickSelect {
                matches: vec![matched],
                labels: vec!["a".to_owned()],
                ..WindowQuickSelect::default()
            },
        );

        app.update_transient_selection_projection();

        let selection = app.selection.expect("projected quick-select selection");
        assert_eq!(selection.anchor, SelectionCell { row: 0, column: 0 });
        assert_eq!(selection.focus, SelectionCell { row: 2, column: 7 });
        assert!(selection.anchor.row <= selection.focus.row);
    }

    #[test]
    fn window_app_quick_select_matches_do_not_retarget_after_prune() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(32, 2));
        app.runtime.set_scrollback_limit(2);
        app.handle_pty_output(b"https://old.test\r\nkeep\r\nlive")
            .unwrap();
        app.enter_quick_select_mode();
        let matched = active_quick_select_for_test(&app).matches[0];
        assert_eq!(matched.domain, rterm_terminal::TerminalScreenDomain::Main);

        app.handle_pty_output(b"\r\nnew-1\r\nnew-2\r\nnew-3")
            .unwrap();

        assert!(quick_select_for_test(&app).is_none());
        assert!(app.selection.is_none());
        assert_ne!(
            app.runtime
                .terminal()
                .history_index_to_stable_row(0)
                .unwrap(),
            matched.source_row
        );
    }

    #[test]
    fn window_app_quick_select_current_prune_does_not_retarget_surviving_match() {
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_copy = Arc::clone(&copied);
        let mut app = NativeWindowApp::new(None);
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_copy.lock().unwrap().push(text.to_owned());
            true
        });
        app.runtime.resize(rterm_core::TerminalSize::new(32, 2));
        app.runtime.set_scrollback_limit(3);
        app.handle_pty_output(b"https://old.test\r\nhttps://later.test\r\nkeep\r\nlive")
            .unwrap();
        app.enter_quick_select_mode();
        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 2);
        let pruned = quick_select.matches[0];
        let survivor = quick_select.matches[1];
        let survivor_label = quick_select.labels[1].clone();

        app.handle_pty_output(b"\r\nnew-1\r\nnew-2").unwrap();

        assert!(!pruned.is_retained(app.runtime.terminal()));
        assert!(survivor.is_retained(app.runtime.terminal()));
        assert!(quick_select_for_test(&app).is_none());
        assert!(app.selection.is_none());
        assert!(!app.handle_quick_select_logical_key(
            &Key::Character(survivor_label.into()),
            ModifiersState::empty()
        ));
        assert!(copied.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_quick_select_accept_uses_stable_match_text_after_viewport_moves() {
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_copy = Arc::clone(&copied);
        let mut app = NativeWindowApp::new(None);
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_copy.lock().unwrap().push(text.to_owned());
            true
        });
        app.runtime.resize(rterm_core::TerminalSize::new(32, 2));
        app.handle_pty_output(b"https://stable.test\r\nline-1\r\nline-2\r\nline-3")
            .unwrap();
        app.enter_quick_select_mode();
        assert!(
            quick_select_for_test(&app)
                .and_then(WindowQuickSelect::current_match)
                .is_some()
        );

        app.set_scrollback_offset(0);
        assert!(app.selection.is_none());
        app.accept_quick_select_match(false);
        app.exit_quick_select_mode();

        assert_eq!(
            copied.lock().unwrap().as_slice(),
            ["https://stable.test".to_owned()]
        );
    }

    #[test]
    fn window_app_search_uses_only_active_alternate_screen_domain() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(32, 3));
        app.handle_pty_output(b"main-only-needle\r\nmain-1\r\nmain-2\r\nmain-3\r\nmain-4")
            .unwrap();
        app.handle_pty_output(b"\x1b[?1049h\x1b[2J\x1b[Halt-only-needle\r\nalt-1\r\nalt-2")
            .unwrap();
        assert_eq!(
            app.runtime.terminal().stable_dimensions().domain,
            rterm_terminal::TerminalScreenDomain::Alternate
        );

        app.enter_search_mode();
        assert!(!app.update_search_query("main-only-needle"));
        assert!(search_for_test(&app).is_some_and(|search| search.current.is_none()));

        assert!(app.update_search_query("alt-only-needle"));
        let matched = active_search_for_test(&app).current.unwrap();
        assert_eq!(
            matched.domain,
            rterm_terminal::TerminalScreenDomain::Alternate
        );
        assert_eq!(matched.source_row, 0);
        assert_eq!(app.selected_text().as_deref(), Some("alt-only-needle"));
    }

    #[test]
    fn window_app_quick_select_uses_and_copies_only_active_alternate_screen_domain() {
        let copied = Arc::new(Mutex::new(Vec::new()));
        let recorded_copy = Arc::clone(&copied);
        let mut app = NativeWindowApp::new(None);
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_copy.lock().unwrap().push(text.to_owned());
            true
        });
        app.runtime.resize(rterm_core::TerminalSize::new(40, 3));
        app.handle_pty_output(b"https://main-only.test\r\nmain-1\r\nmain-2\r\nmain-3\r\nmain-4")
            .unwrap();
        app.handle_pty_output(b"\x1b[?1049h\x1b[2J\x1b[Hhttps://alt-only.test\r\nalt-1\r\nalt-2")
            .unwrap();

        app.enter_quick_select_mode();

        let quick_select = active_quick_select_for_test(&app);
        assert_eq!(quick_select.matches.len(), 1);
        assert_eq!(
            quick_select.matches[0].domain,
            rterm_terminal::TerminalScreenDomain::Alternate
        );
        assert_eq!(quick_select.matches[0].source_row, 0);
        let label = quick_select.labels[0].clone();
        assert!(app.handle_quick_select_logical_key(
            &Key::Character(label.into()),
            ModifiersState::empty()
        ));
        assert_eq!(
            copied.lock().unwrap().as_slice(),
            ["https://alt-only.test".to_owned()]
        );
    }

    #[test]
    fn window_copy_mode_moves_and_selects_across_alternate_rows_with_main_history() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 3));
        app.handle_pty_output(b"main-0\r\nmain-1\r\nmain-2\r\nmain-3\r\nmain-4")
            .unwrap();
        assert!(app.runtime.terminal().scrollback().len() >= 2);
        app.handle_pty_output(b"\x1b[?1049h\x1b[2J\x1b[Halt-0\r\nalt-1\r\nalt-2")
            .unwrap();

        app.enter_copy_mode();
        assert!(app.set_copy_mode_cursor(0, 0));
        assert!(app.handle_copy_mode_key(&Key::Character("v".into()), ModifiersState::empty()));
        assert!(app.handle_copy_mode_key(&Key::Character("j".into()), ModifiersState::empty()));
        assert!(app.handle_copy_mode_key(&Key::Character("j".into()), ModifiersState::empty()));

        let copy_mode = active_copy_mode_for_test(&app);
        assert_eq!(copy_mode.cursor, SelectionCell { row: 2, column: 0 });
        assert_eq!(copy_mode.anchor, Some(SelectionCell { row: 0, column: 0 }));
        assert_eq!(
            copy_mode.source_cursor,
            SelectionSourceCell {
                domain: rterm_terminal::TerminalScreenDomain::Alternate,
                row: 2,
                column: 0,
            }
        );
        assert_eq!(
            copy_mode.source_anchor,
            Some(SelectionSourceCell {
                domain: rterm_terminal::TerminalScreenDomain::Alternate,
                row: 0,
                column: 0,
            })
        );
        assert_eq!(app.current_scrollback_offset(), 0);
        assert_eq!(
            app.selection,
            Some(WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 2, column: 0 },
            ))
        );
        assert_eq!(app.selected_text().as_deref(), Some("alt-0\nalt-1\na"));
    }

    #[test]
    fn window_app_config_scrollback_limit_reconciles_active_search_and_snapshot() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(20, 2));
        app.handle_pty_output(b"needle-old\r\nkeep-1\r\nkeep-2\r\nlive")
            .unwrap();
        app.enter_search_mode();
        assert!(app.update_search_query("needle-old"));
        let matched = active_search_for_test(&app).current.unwrap();
        assert!(matched.is_retained(app.runtime.terminal()));
        assert_eq!(
            snapshot_row_text(&app.snapshot, 0, 20),
            "needle-old          "
        );

        app.set_config_overrides(native_config_snapshot! {
            scrollback_lines: Some(1),
            ..NativeConfigSnapshot::default()
        });

        assert!(!matched.is_retained(app.runtime.terminal()));
        assert!(search_for_test(&app).is_some_and(|search| search.current.is_none()));
        assert!(app.selection.is_none());
        assert_eq!(
            snapshot_row_text(&app.snapshot, 0, 20),
            "keep-1              "
        );
    }

    #[test]
    fn window_app_config_scrollback_limit_reconciles_active_copy_mode_and_snapshot() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(20, 2));
        app.handle_pty_output(b"copy-old\r\nkeep-1\r\nkeep-2\r\nlive")
            .unwrap();
        app.enter_copy_mode();
        assert!(app.move_copy_mode_to_scrollback_top());
        let source_cursor = active_copy_mode_for_test(&app).source_cursor;
        assert_eq!(
            snapshot_row_text(&app.snapshot, 0, 20),
            "copy-old            "
        );

        app.set_config_overrides(native_config_snapshot! {
            scrollback_lines: Some(1),
            ..NativeConfigSnapshot::default()
        });

        assert!(
            !app.runtime
                .terminal()
                .retained_stable_range()
                .contains(&source_cursor.row)
        );
        assert!(copy_mode_for_test(&app).is_none());
        assert!(app.selection.is_none());
        assert_eq!(
            snapshot_row_text(&app.snapshot, 0, 20),
            "keep-1              "
        );
    }

    #[test]
    fn window_app_config_scrollback_limit_reconciles_active_quick_select_and_snapshot() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(32, 2));
        app.handle_pty_output(b"https://old.test\r\nkeep-1\r\nkeep-2\r\nlive")
            .unwrap();
        app.enter_quick_select_mode();
        let matched = app
            .active_ui
            .quick_select()
            .and_then(WindowQuickSelect::current_match)
            .expect("quick-select match");
        assert!(matched.is_retained(app.runtime.terminal()));
        assert_eq!(
            snapshot_row_text(&app.snapshot, 0, 32),
            "https://old.test                "
        );

        app.set_config_overrides(native_config_snapshot! {
            scrollback_lines: Some(1),
            ..NativeConfigSnapshot::default()
        });

        assert!(!matched.is_retained(app.runtime.terminal()));
        assert!(quick_select_for_test(&app).is_none());
        assert!(app.selection.is_none());
        assert_eq!(
            snapshot_row_text(&app.snapshot, 0, 32),
            "keep-1                          "
        );
    }

    #[test]
    fn window_app_config_scrollback_limit_rebuilds_asymmetric_inactive_pane_snapshot() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 2));
        let left_pane = app.app_shell.active_pane_id();
        app.handle_pty_output(b"left-old\r\nleft-1\r\nleft-2\r\nleft-3\r\nleft-live")
            .unwrap();
        app.scroll_viewport_lines(3);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 16), "left-old        ");
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"right-old\r\nright-1\r\nright-live")
            .unwrap();
        app.scroll_viewport_lines(1);
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 16), "right-old       ");

        app.set_config_overrides(native_config_snapshot! {
            scrollback_lines: Some(1),
            ..NativeConfigSnapshot::default()
        });

        let inactive = app.pane_runtimes.get(&left_pane).expect("left runtime");
        assert_eq!(inactive.runtime.terminal().scrollback().len(), 1);
        assert_eq!(
            inactive
                .ui
                .stable_viewport
                .active_top(inactive.runtime.terminal()),
            Some(
                inactive
                    .runtime
                    .terminal()
                    .stable_dimensions()
                    .scrollback_top
            )
        );
        assert_eq!(
            snapshot_row_text(&inactive.snapshot, 0, 16),
            "left-2          "
        );
        assert_eq!(snapshot_row_text(&app.snapshot, 0, 16), "right-old       ");
    }

    #[test]
    fn window_app_runtime_scrollback_limit_reconciles_active_and_inactive_overlays() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(32, 2));
        app.handle_pty_output(b"copy-old\r\nleft-1\r\nleft-2\r\nleft-live")
            .unwrap();
        app.enter_copy_mode();
        assert!(app.move_copy_mode_to_scrollback_top());
        let inactive_owner = app.active_pane_id();
        let inactive_source = active_copy_mode_for_test(&app).source_cursor;
        app.dispatch_app_action(AppAction::SplitPane {
            pane: inactive_owner,
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"https://active-old.test\r\nright-1\r\nright-2\r\nright-live")
            .unwrap();
        app.enter_quick_select_mode();
        let active_match = active_quick_select_for_test(&app)
            .current_match()
            .expect("active Quick match");

        app.set_config_overrides(native_config_snapshot! {
            scrollback_lines: Some(1),
            ..NativeConfigSnapshot::default()
        });

        assert!(!active_match.is_retained(app.runtime.terminal()));
        assert!(quick_select_for_test(&app).is_none());
        assert!(app.selection.is_none());
        let inactive = app
            .pane_runtimes
            .get(&inactive_owner)
            .expect("inactive Copy owner");
        assert!(
            !inactive
                .runtime
                .terminal()
                .retained_stable_range()
                .contains(&inactive_source.row)
        );
        assert!(!inactive.ui.overlay_active());
        assert_eq!(inactive.runtime.terminal().scrollback().len(), 1);
        assert_eq!(app.runtime.terminal().scrollback().len(), 1);
    }

    #[test]
    fn window_app_disable_default_mouse_bindings_suppresses_default_wheel_scrollback() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.set_config_overrides(native_config_snapshot! {
            disable_default_mouse_bindings: Some(true),
            ..NativeConfigSnapshot::default()
        });
        let active = app.active_pane_id();
        move_wheel_to_pane_cell_for_test(&mut app, active, 0, 0, 1.0, 1.0);

        assert!(
            !app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert_eq!(app.current_scrollback_offset(), 0);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('c'));
    }

    #[test]
    fn window_app_mouse_wheel_sends_default_arrow_keys_in_alternate_screen() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.handle_pty_output(b"\x1b[?1049h").unwrap();
        let active = app.active_pane_id();
        move_wheel_to_pane_cell_for_test(&mut app, active, 0, 0, 1.0, 1.0);

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b[A\x1b[A\x1b[A");
        assert_eq!(app.current_scrollback_offset(), 0);
    }

    #[test]
    fn window_app_mouse_wheel_keeps_mouse_reporting_in_alternate_screen() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.handle_pty_output(b"\x1b[?1049h\x1b[?1000;1006h")
            .unwrap();
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b[<64;2;1M");
    }

    #[test]
    fn window_app_mouse_wheel_honors_mouse_reporting_bypass_modifier() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.handle_pty_output(b"\x1b[?1000;1006h").unwrap();
        app.modifiers = ModifiersState::SHIFT;
        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0))
                .unwrap()
        );

        assert!(written.lock().unwrap().is_empty());
        assert_eq!(app.current_scrollback_offset(), 1);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('a'));
    }

    #[test]
    fn window_app_mouse_wheel_honors_configured_alternate_buffer_scroll_speed() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.set_config_overrides(native_config_snapshot! {
            alternate_buffer_wheel_scroll_speed: Some(1),
            ..NativeConfigSnapshot::default()
        });
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"\x1b[?1049h").unwrap();
        let active = app.active_pane_id();
        move_wheel_to_pane_cell_for_test(&mut app, active, 0, 0, 1.0, 1.0);

        assert!(
            app.handle_window_mouse_wheel(MouseScrollDelta::LineDelta(0.0, -1.0))
                .unwrap()
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"\x1b[B");
        assert_eq!(
            app.native_effective_config()
                .alternate_buffer_wheel_scroll_speed,
            1
        );
    }

    #[test]
    fn window_app_scroll_by_current_event_wheel_delta_requires_current_wheel_event() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();

        assert!(
            app.command_palette_apply_command(WindowCommand::ScrollByCurrentEventWheelDelta)
                .is_ok()
        );

        assert_eq!(app.current_scrollback_offset(), 0);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('c'));
    }

    #[test]
    fn window_app_scroll_by_current_event_wheel_delta_uses_vertical_wheel_delta() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"ab\r\ncd\r\nef").unwrap();
        app.current_mouse_wheel_delta = Some(MouseScrollDelta::LineDelta(0.0, 1.0));

        assert!(
            app.command_palette_apply_command(WindowCommand::ScrollByCurrentEventWheelDelta)
                .is_ok()
        );

        assert_eq!(app.current_scrollback_offset(), 1);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('a'));

        app.current_mouse_wheel_delta = Some(MouseScrollDelta::LineDelta(1.0, 0.0));
        assert!(
            app.command_palette_apply_command(WindowCommand::ScrollByCurrentEventWheelDelta)
                .is_ok()
        );

        assert_eq!(app.current_scrollback_offset(), 1);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('a'));
    }

    #[test]
    fn window_app_shift_page_keys_scroll_scrollback_viewport() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();

        assert!(
            app.handle_scrollback_shortcut(&Key::Named(NamedKey::PageUp), ModifiersState::SHIFT)
        );

        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('b'));

        assert!(
            app.handle_scrollback_shortcut(&Key::Named(NamedKey::PageDown), ModifiersState::SHIFT)
        );

        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('d'));
    }

    #[test]
    fn window_app_keyboard_input_routes_shift_page_keys_to_scrollback_viewport() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();
        app.modifiers = ModifiersState::SHIFT;

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::PageUp),
            PhysicalKey::Code(WinitKeyCode::PageUp),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('b'));

        app.handle_keyboard_input_event(
            &Key::Named(NamedKey::PageDown),
            PhysicalKey::Code(WinitKeyCode::PageDown),
            None,
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('d'));
    }

    #[test]
    fn window_app_hides_mouse_cursor_when_typing_after_cursor_enters_window() {
        let mut app = NativeWindowApp::new(None);

        app.handle_cursor_moved(PhysicalPosition::new(8.0, 8.0))
            .unwrap();
        assert!(app.mouse_cursor_visible);

        app.handle_keyboard_input_event(
            &Key::Character("a".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            Some("a"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(!app.mouse_cursor_visible);
    }

    #[test]
    fn window_app_mouse_motion_restores_cursor_after_typing_hide() {
        let mut app = NativeWindowApp::new(None);
        app.handle_cursor_moved(PhysicalPosition::new(8.0, 8.0))
            .unwrap();
        app.handle_keyboard_input_event(
            &Key::Character("a".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            Some("a"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(!app.mouse_cursor_visible);

        app.handle_cursor_moved(PhysicalPosition::new(9.0, 8.0))
            .unwrap();

        assert!(app.mouse_cursor_visible);
    }

