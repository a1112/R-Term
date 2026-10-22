    #[test]
    fn lua_parses_wezterm_format_tab_title_truncate_left_title() {
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
              title = wezterm.truncate_left(title, max_width - 2)
              return {
                { Text = '<' .. title .. '>' },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title truncate_left title");

        let parsed = format!("{:?}", overrides.lua_tab_title);
        assert!(
            parsed.contains("TruncateLeft"),
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
    fn window_app_parses_static_wezterm_format_tab_title_nerdfont_static_key_module() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local wt = require 'wezterm'
            local nerd_key = 'nerdfonts'
            local left_key = 'pl_right_hard_divider'
            local right_key = 'pl_left_hard_divider'
            local SOLID_LEFT_ARROW = wt[nerd_key][left_key]
            local SOLID_RIGHT_ARROW = wt[nerd_key][right_key]

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
        .expect("expected static WezTerm format-tab-title nerdfont static-key module");
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

    #[test]
    fn lua_parses_wezterm_format_tab_title_truncate_left_static_key_module() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local wt = require 'wezterm'
            local truncate_key = 'truncate_left'

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
        .expect("expected static WezTerm format-tab-title truncate_left static-key module");

        let parsed = format!("{:?}", overrides.lua_tab_title);
        assert!(
            parsed.contains("TruncateLeft"),
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
    fn window_app_parses_static_wezterm_format_tab_title_truncate_left_title() {
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
              title = wezterm.truncate_left(title, max_width - 2)
              return {
                { Text = '<' .. title .. '>' },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title truncate_left title");
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
            tab_bar.contains("<efghijklmnopqr>"),
            "tab bar was {tab_bar:?}"
        );
        assert!(
            !tab_bar.contains("<abcdefghijklmnopqr>"),
            "tab bar was {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_active_and_last_active_branches() {
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
        app.set_config_overrides(overrides);

        let first_tab = app.active_tab_id();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: first_tab,
            title: "first".to_owned(),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        let second_tab = app.active_tab_id();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: second_tab,
            title: "second".to_owned(),
        })
        .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let first_column = tab_bar
            .find(" first*")
            .expect("last-active formatted Lua tab title should render in the tab bar");
        let second_column = tab_bar
            .find(" second ")
            .expect("active formatted Lua tab title should render in the tab bar");
        let first_cell = snapshot_cell(&snapshot, 0, u16::try_from(first_column).unwrap()).unwrap();
        let second_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(second_column).unwrap()).unwrap();

        assert_eq!(first_cell.background, rterm_terminal::Color::Rgb(0, 128, 0));
        assert_eq!(second_cell.background, rterm_terminal::Color::Rgb(0, 0, 255));
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_event_string_concat_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local prefix = 'STATIC '
              local subject = 'LUA '
              local suffix = 'TAB'
              return prefix .. subject .. suffix
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title event string concat return");
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
    fn window_app_parses_static_wezterm_format_tab_title_event_top_level_string_concat_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local prefix = 'STATIC '
            local subject = 'LUA '
            local suffix = 'TAB'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return prefix .. subject .. suffix
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title event top-level string concat return");
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
    fn window_app_parses_static_wezterm_format_tab_title_event_format_item_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return {
                { Foreground = { Color = '#010203' } },
                { Background = { Color = '#040506' } },
                { Text = 'STATIC LUA FORMAT' },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title event format item return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let title_column = tab_bar
            .find("STATIC LUA FORMAT")
            .expect("formatted Lua title should render in the tab bar");
        let title_cell = snapshot_cell(&snapshot, 0, u16::try_from(title_column).unwrap()).unwrap();

        assert_eq!(title_cell.ch, 'S');
        assert_eq!(title_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_eq!(title_cell.background, rterm_terminal::Color::Rgb(4, 5, 6));
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_dynamic_text_format_item_return() {
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
              return {
                { Foreground = { Color = '#010203' } },
                { Background = { Color = '#040506' } },
                { Text = ' ' .. tab.active_pane.title .. ' ' },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title dynamic Text item return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let title_column = tab_bar
            .find(" PaneShell ")
            .expect("formatted dynamic Lua title should render in the tab bar");
        let title_cell = snapshot_cell(&snapshot, 0, u16::try_from(title_column).unwrap()).unwrap();

        assert_eq!(title_cell.ch, ' ');
        assert_eq!(title_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_eq!(title_cell.background, rterm_terminal::Color::Rgb(4, 5, 6));
        assert!(!tab_bar.contains("explicit"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_dynamic_text_variable_return() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("foreground-proc"));
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
              local pane = tab.active_pane
              local title = pane.foreground_process_name .. ' ' .. pane.pane_id
              return {
                { Foreground = { Color = '#010203' } },
                { Background = { Color = '#040506' } },
                { Text = ' ' .. title .. ' ' },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title dynamic Text variable return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let title_column = tab_bar
            .find(" foreground-proc 1 ")
            .expect("formatted dynamic Lua variable title should render in the tab bar");
        let title_cell = snapshot_cell(&snapshot, 0, u16::try_from(title_column).unwrap()).unwrap();

        assert_eq!(title_cell.ch, ' ');
        assert_eq!(title_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_eq!(title_cell.background, rterm_terminal::Color::Rgb(4, 5, 6));
        assert!(!tab_bar.contains("PaneShell"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("explicit"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_event_format_alias_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local fmt = wezterm.format

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return fmt({
                { Foreground = { Color = '#010203' } },
                { Background = { Color = '#040506' } },
                { Text = 'STATIC LUA ALIAS FORMAT' },
              })
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title event format alias return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let title_column = tab_bar
            .find("STATIC LUA ALIAS FORMAT")
            .expect("formatted Lua alias title should render in the tab bar");
        let title_cell = snapshot_cell(&snapshot, 0, u16::try_from(title_column).unwrap()).unwrap();

        assert_eq!(title_cell.ch, 'S');
        assert_eq!(title_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_eq!(title_cell.background, rterm_terminal::Color::Rgb(4, 5, 6));
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_event_format_alias_dotted_comment() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local fmt = wezterm -- formatter helper
              .format

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return fmt({
                { Foreground = { Color = '#070809' } },
                { Background = { Color = '#0a0b0c' } },
                { Text = 'STATIC LUA DOTTED ALIAS FORMAT' },
              })
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title dotted-comment format alias return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let title_column = tab_bar
            .find("STATIC LUA DOTTED ALIAS FORMAT")
            .expect("formatted Lua dotted-comment alias title should render in the tab bar");
        let title_cell = snapshot_cell(&snapshot, 0, u16::try_from(title_column).unwrap()).unwrap();

        assert_eq!(title_cell.ch, 'S');
        assert_eq!(title_cell.foreground, rterm_terminal::Color::Rgb(7, 8, 9));
        assert_eq!(title_cell.background, rterm_terminal::Color::Rgb(10, 11, 12));
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_event_format_item_variable_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local items = {
                { Foreground = { Color = '#010203' } },
                { Background = { Color = '#040506' } },
                { Text = 'STATIC LUA VAR FORMAT' },
              }
              return items
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title event format item variable return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let title_column = tab_bar
            .find("STATIC LUA VAR FORMAT")
            .expect("formatted Lua variable title should render in the tab bar");
        let title_cell = snapshot_cell(&snapshot, 0, u16::try_from(title_column).unwrap()).unwrap();

        assert_eq!(title_cell.ch, 'S');
        assert_eq!(title_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_eq!(title_cell.background, rterm_terminal::Color::Rgb(4, 5, 6));
    }

    #[test]
    fn window_app_parses_format_tab_title_top_level_format_item_variable_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local items = {
              { Foreground = { Color = '#010203' } },
              { Background = { Color = '#040506' } },
              { Text = 'STATIC LUA TOP FORMAT' },
            }

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return items
            end)
            "#,
        )
        .expect(
            "expected static WezTerm format-tab-title event top-level format item variable return",
        );
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let title_column = tab_bar
            .find("STATIC LUA TOP FORMAT")
            .expect("top-level formatted Lua variable title should render in the tab bar");
        let title_cell = snapshot_cell(&snapshot, 0, u16::try_from(title_column).unwrap()).unwrap();

        assert_eq!(title_cell.ch, 'S');
        assert_eq!(title_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_eq!(title_cell.background, rterm_terminal::Color::Rgb(4, 5, 6));
    }

    #[test]
    fn window_app_parses_format_tab_title_top_level_wezterm_format_result_variable_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local title = wezterm.format({
              { Foreground = { Color = '#010203' } },
              { Background = { Color = '#040506' } },
              { Text = 'STATIC LUA TOP FORMAT RESULT' },
            })

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return title
            end)
            "#,
        )
        .expect(
            "expected static WezTerm format-tab-title event top-level format result variable return",
        );
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let title_column = tab_bar
            .find("STATIC LUA TOP FORMAT RESULT")
            .expect("top-level formatted Lua result variable title should render in the tab bar");
        let title_cell = snapshot_cell(&snapshot, 0, u16::try_from(title_column).unwrap()).unwrap();

        assert_eq!(title_cell.ch, 'S');
        assert_eq!(title_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_eq!(title_cell.background, rterm_terminal::Color::Rgb(4, 5, 6));
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_active_pane_title_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "explicit".to_owned(),
        })
        .unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return tab.active_pane.title
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title active pane title return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("PowerShell"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("explicit"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn lua_parses_static_wezterm_format_tab_title_tab_title_return() {
        let title = super::lua_static_wezterm_tab_title_return_event_from_query(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return tab.tab_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title tab title return");

        assert_eq!(format!("{title:?}"), "ActiveTabTitle");
    }

    #[test]
    fn lua_parses_static_wezterm_format_tab_title_tab_id_return() {
        let title = super::lua_static_wezterm_tab_title_return_event_from_query(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return tab.tab_id
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title tab id return");

        assert_eq!(format!("{title:?}"), "TabId");
    }

    #[test]
    fn lua_parses_static_wezterm_format_tab_title_tab_count_return() {
        let title = super::lua_static_wezterm_tab_title_return_event_from_query(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return #tabs
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title tab count return");

        assert_eq!(format!("{title:?}"), "TabCount");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_window_title_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PaneShell\x07").unwrap();
        app.window_title = "Project Window".to_owned();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "explicit".to_owned(),
        })
        .unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return tab.window_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title window title return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("Project Window"),
            "tab bar was {tab_bar:?}"
        );
        assert!(!tab_bar.contains("PaneShell"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("explicit"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_active_pane_domain_name_return() {
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
              return tab.active_pane.domain_name
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title active pane domain name return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("local"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("PaneShell"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("explicit"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_active_pane_foreground_process_return() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("foreground-proc"));
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
              return tab.active_pane.foreground_process_name
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title active pane foreground process return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("foreground-proc"),
            "tab bar was {tab_bar:?}"
        );
        assert!(!tab_bar.contains("PaneShell"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("explicit"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_dynamic_concat_return() {
        let mut app =
            NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("foreground-proc"));
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
              return tab.active_pane.foreground_process_name .. ':' .. tab.active_pane.pane_id
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title dynamic concat return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("foreground-proc:1"),
            "tab bar was {tab_bar:?}"
        );
        assert!(!tab_bar.contains("PaneShell"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("explicit"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_active_pane_user_var_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return 'prog=' .. tab.active_pane.user_vars.WEZTERM_PROG
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title active pane user var return");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM_PROG=cHNo\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("prog=psh"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_active_pane_alias_user_var_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local pane = tab.active_pane
              return 'prog=' .. pane.user_vars['WEZTERM-PROG']
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title active pane alias user var return");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM-PROG=cHNo\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("prog=psh"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_local_user_vars_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local pane = tab.active_pane
              local vars = pane.user_vars
              return 'prog=' .. vars['WEZTERM-PROG']
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title local user vars return");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM-PROG=cHNo\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("prog=psh"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_local_user_var_condition() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local pane = tab.active_pane
              local vars = pane.user_vars
              if vars['WEZTERM-PROG'] ~= nil then
                return 'prog=' .. vars['WEZTERM-PROG']
              end

              return 'prog=none'
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title local user var condition");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]1337;SetUserVar=WEZTERM-PROG=cHNo\x07")
            .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("prog=psh"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("prog=none"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_missing_user_var_condition() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local pane = tab.active_pane
              local vars = pane.user_vars
              if vars['WEZTERM-PROG'] == nil then
                return 'prog=none'
              end

              return 'prog=' .. vars['WEZTERM-PROG']
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title missing user var condition");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("prog=none"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_active_pane_progress_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return 'pct=' .. tab.active_pane.progress.Percentage
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title active pane progress return");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]9;4;1;42\x07").unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("pct=42"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_active_pane_alias_progress_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local pane = tab.active_pane
              return 'pct=' .. pane.progress.Percentage
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title active pane alias progress return");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]9;4;1;42\x07").unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("pct=42"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_active_pane_alias_progress_percentage_condition()
     {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local pane = tab.active_pane
              if pane.progress.Percentage ~= nil then
                return 'pct=' .. pane.progress.Percentage
              end

              return 'progress=idle'
            end)
            "#,
        )
        .expect(
            "expected static WezTerm format-tab-title active pane alias progress percentage condition",
        );
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]9;4;1;42\x07").unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("pct=42"), "tab bar was {tab_bar:?}");
        assert!(
            !tab_bar.contains("progress=idle"),
            "tab bar was {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_local_progress_percentage_condition() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local pane = tab.active_pane
              local progress = pane.progress
              if progress.Percentage ~= nil then
                return 'pct=' .. progress.Percentage
              end

              return 'progress=idle'
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title local progress percentage condition");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]9;4;1;42\x07").unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("pct=42"), "tab bar was {tab_bar:?}");
        assert!(
            !tab_bar.contains("progress=idle"),
            "tab bar was {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_missing_progress_percentage_condition() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local pane = tab.active_pane
              local progress = pane.progress
              if progress.Percentage == nil then
                return 'progress=idle'
              end

              return 'pct=' .. progress.Percentage
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title missing progress percentage condition");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("progress=idle"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_active_pane_progress_error_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return 'err=' .. tab.active_pane.progress.Error
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title active pane progress error return");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]9;4;2;7\x07").unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("err=7"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_active_pane_indeterminate_progress_condition()
     {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              if tab.active_pane.progress == 'Indeterminate' then
                return 'progress=indeterminate'
              end

              return 'progress=idle'
            end)
            "#,
        )
        .expect(
            "expected static WezTerm format-tab-title active pane indeterminate progress condition",
        );
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]9;4;3;0\x07").unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("progress=indeterminate"),
            "tab bar was {tab_bar:?}"
        );
        assert!(
            !tab_bar.contains("progress=idle"),
            "tab bar was {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_active_pane_alias_indeterminate_progress_condition()
     {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local pane = tab.active_pane
              if pane.progress == 'Indeterminate' then
                return 'progress=indeterminate'
              end

              return 'progress=idle'
            end)
            "#,
        )
        .expect(
            "expected static WezTerm format-tab-title active pane alias indeterminate progress condition",
        );
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]9;4;3;0\x07").unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("progress=indeterminate"),
            "tab bar was {tab_bar:?}"
        );
        assert!(
            !tab_bar.contains("progress=idle"),
            "tab bar was {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_local_progress_indeterminate_condition() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local pane = tab.active_pane
              local progress = pane.progress
              if progress == 'Indeterminate' then
                return 'progress=indeterminate'
              end

              return 'progress=idle'
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title local progress indeterminate condition");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b]9;4;3;0\x07").unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(
            tab_bar.contains("progress=indeterminate"),
            "tab bar was {tab_bar:?}"
        );
        assert!(
            !tab_bar.contains("progress=idle"),
            "tab bar was {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_tab_id_index_concat_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;First\x07").unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.handle_pty_output(b"\x1b]2;Second\x07").unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return 'tab:' .. tab.tab_id .. '/' .. tab.tab_index
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title tab id/index concat return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("tab:1/0"), "tab bar was {tab_bar:?}");
        assert!(tab_bar.contains("tab:2/1"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("First"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("Second"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_tab_pane_count_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;First\x07").unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.handle_pty_output(b"\x1b]2;Second\x07").unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return 'counts:' .. #tabs .. '/' .. #panes
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title tab/pane count return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("counts:2/1"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("First"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("Second"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_string_format_tab_index_count_return() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;First\x07").unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.handle_pty_output(b"\x1b]2;Second\x07").unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return string.format('%d/%d', tab.tab_index + 1, #tabs)
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title string.format return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("1/2"), "tab bar was {tab_bar:?}");
        assert!(tab_bar.contains("2/2"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("First"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("Second"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_string_format_text_item() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "First".to_owned(),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(2),
            title: "Second".to_owned(),
        })
        .unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              return {
                { Text = string.format('[%d/%d] ', tab.tab_index + 1, #tabs) .. tab.tab_title },
              }
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title string.format Text item");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("[1/2] First"), "tab bar was {tab_bar:?}");
        assert!(tab_bar.contains("[2/2] Second"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn lua_parses_static_wezterm_format_tab_title_tab_count_condition() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              if #tabs > 1 then
                return 'many:' .. #tabs
              end
              return 'one:' .. tab.tab_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title tab count condition");

        let parsed = format!("{:?}", overrides.lua_tab_title);
        assert!(
            parsed.contains("TabCountGreaterThan(1)"),
            "parsed lua tab title was {parsed}"
        );
        assert!(
            parsed.contains("TabCount"),
            "parsed lua tab title was {parsed}"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_tab_count_condition() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "First".to_owned(),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(2),
            title: "Second".to_owned(),
        })
        .unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              if #tabs > 1 then
                return 'many:' .. #tabs
              end
              return 'one:' .. tab.tab_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title tab count condition");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("many:2"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("one:First"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("one:Second"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_else_return_condition() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              if #tabs > 1 then
                return 'many:' .. #tabs
              else
                return 'one:' .. tab.tab_title
              end
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title else return condition");
        let parsed = format!("{:?}", overrides.lua_tab_title);
        assert!(parsed.contains("Conditional"), "parsed was {parsed}");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "Solo".to_owned(),
        })
        .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("one:Solo"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("many:"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_else_assignment_condition() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local title = tab.tab_title

              if #tabs > 1 then
                title = 'many:' .. #tabs
              else
                title = 'one:' .. tab.tab_title
              end

              return title
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title else assignment condition");
        let parsed = format!("{:?}", overrides.lua_tab_title);
        assert!(parsed.contains("Conditional"), "parsed was {parsed}");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "Solo".to_owned(),
        })
        .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("one:Solo"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("many:"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_self_referential_condition() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local title = tab.tab_title

              if #tabs > 0 then
                title = '[' .. #tabs .. '] ' .. title
              end

              return title
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title self-referential condition");
        app.set_config_overrides(overrides);
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "Solo".to_owned(),
        })
        .unwrap();

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("[1] Solo"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("] ["), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_pane_count_condition() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: app.active_pane_id(),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              if #panes > 1 then
                return 'lua-panes:' .. #panes
              end
              return 'lua-single:' .. tab.tab_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title pane count condition");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("lua-panes:2"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("lua-single:"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn lua_parses_static_wezterm_format_tab_title_zoomed_condition() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              if tab.active_pane.is_zoomed then
                return 'zoomed:' .. tab.tab_title
              end
              return 'plain:' .. tab.tab_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title zoomed condition");

        let parsed = format!("{:?}", overrides.lua_tab_title);
        assert!(
            parsed.contains("ActivePaneIsZoomed"),
            "parsed lua tab title was {parsed}"
        );
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_zoomed_condition() {
        let mut app = NativeWindowApp::new(None);
        let active_tab = app.active_tab_id();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: active_tab,
            title: "Main".to_owned(),
        })
        .unwrap();
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

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              if tab.active_pane.is_zoomed then
                return 'zoomed:' .. tab.tab_title
              end
              return 'plain:' .. tab.tab_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title zoomed condition");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("zoomed:Main"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("plain:Main"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_active_pane_alias_zoomed_condition() {
        let mut app = NativeWindowApp::new(None);
        let active_tab = app.active_tab_id();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: active_tab,
            title: "Main".to_owned(),
        })
        .unwrap();
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

            wezterm.on('format-tab-title', function(tab, tabs, panes, config, hover, max_width)
              local pane = tab.active_pane
              if pane.is_zoomed then
                return 'zoomed:' .. tab.tab_title
              end
              return 'plain:' .. tab.tab_title
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title active pane alias zoomed condition");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("zoomed:Main"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("plain:Main"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_active_pane_cwd_return() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("foreground-proc").with_cwd("/tmp/project"),
        );
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
              return tab.active_pane.current_working_dir
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title active pane cwd return");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("/tmp/project"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("PaneShell"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("explicit"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_parses_static_wezterm_format_tab_title_active_pane_tty_name_return() {
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
              return tab.active_pane.tty_name
            end)
            "#,
        )
        .expect("expected static WezTerm format-tab-title active pane tty name return");
        app.set_config_overrides(overrides);
        app.session_tty_name = Some("/dev/pts/9".to_owned());

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("/dev/pts/9"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("PaneShell"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("explicit"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_tab_title_formatter_receives_tab_and_pane_information_snapshot() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"\x1b]2;PowerShell\x07").unwrap();
        app.dispatch_app_action(AppAction::SetTabTitle {
            tab: rterm_core::TabId::new(1),
            title: "build".to_owned(),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SetPaneUserVar {
            pane: rterm_core::PaneId::new(1),
            name: "branch".to_owned(),
            value: "main".to_owned(),
        })
        .unwrap();
        app.handle_pty_output(b"\x1b]7;file://host/home/ops\x07")
            .unwrap();
        app.dispatch_app_action(AppAction::SetPaneProgress {
            pane: rterm_core::PaneId::new(1),
            progress: rterm_core::app_shell::PaneProgress::Percentage(42),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.tab_title_formatter = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            None
        });

        let _ = app.render_snapshot();

        let events = seen.lock().unwrap();
        let event = events
            .first()
            .expect("expected format-tab-title event to be dispatched");
        let debug = format!("{event:?}");
        assert!(debug.contains("window_id: WindowId(1)"), "{debug}");
        assert!(debug.contains("window_title:"), "{debug}");
        assert!(debug.contains("tab_title: Some(\"build\")"), "{debug}");
        assert!(debug.contains("active_pane_info:"), "{debug}");
        assert!(debug.contains("tabs:"), "{debug}");
        assert!(debug.contains("tab_id: TabId(1)"), "{debug}");
        assert!(debug.contains("tab_id: TabId(2)"), "{debug}");
        assert!(debug.contains("panes:"), "{debug}");
        assert!(debug.contains("pane_id: PaneId(1)"), "{debug}");
        assert!(debug.contains("pane_index: 0"), "{debug}");
        assert!(debug.contains("is_active: true"), "{debug}");
        assert!(debug.contains("title: Some(\"PowerShell\")"), "{debug}");
        assert!(
            debug.contains("foreground_process_name: \"")
                && !debug.contains("foreground_process_name: \"\""),
            "{debug}"
        );
        assert!(
            debug.contains("current_working_dir: Some(\"file://host/home/ops\")"),
            "{debug}"
        );
        assert!(debug.contains("has_unseen_output: false"), "{debug}");
        assert!(debug.contains("domain_name: \"local\""), "{debug}");
        assert!(debug.contains("tty_name:"), "{debug}");
        assert!(debug.contains("\"branch\": \"main\""), "{debug}");
        assert!(debug.contains("progress: Percentage(42)"), "{debug}");
    }

    #[test]
    fn window_app_tab_title_formatter_receives_runtime_tty_name() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let mut app = NativeWindowApp::new(None);
        app.session_tty_name = Some("/dev/pts/9".to_owned());
        app.tab_title_formatter = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            None
        });

        let _ = app.render_snapshot();

        let events = seen.lock().unwrap();
        let event = events
            .first()
            .expect("expected format-tab-title event to be dispatched");
        let debug = format!("{event:?}");
        assert!(debug.contains("tty_name: Some(\"/dev/pts/9\")"), "{debug}");
    }

    #[test]
    fn window_app_tab_title_formatter_receives_active_tab_panes_for_each_tab() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        let active_pane = app.app_shell.active_pane_id();
        assert_eq!(active_pane, rterm_core::PaneId::new(3));
        app.tab_title_formatter = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            None
        });

        let _ = app.render_snapshot();

        let events = seen.lock().unwrap();
        let inactive_tab_event = events
            .iter()
            .find(|event| event.tab == rterm_core::TabId::new(1))
            .expect("expected inactive tab title format event");
        let pane_ids = inactive_tab_event
            .panes
            .iter()
            .map(|pane| pane.pane_id)
            .collect::<Vec<_>>();
        assert_eq!(pane_ids, vec![active_pane]);
        assert_eq!(inactive_tab_event.pane_count, 1);
    }

    #[test]
    fn window_app_tab_title_formatter_marks_unseen_inactive_pane_output() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pane_pty_output(rterm_core::PaneId::new(1), b"background")
            .unwrap();
        app.tab_title_formatter = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            None
        });

        let _ = app.render_snapshot();

        let events = seen.lock().unwrap();
        let event = events
            .first()
            .expect("expected format-tab-title event to be dispatched");
        let inactive_pane = event
            .panes
            .iter()
            .find(|pane| pane.pane_id == rterm_core::PaneId::new(1))
            .expect("expected inactive pane information");
        let active_pane = event
            .panes
            .iter()
            .find(|pane| pane.pane_id == rterm_core::PaneId::new(2))
            .expect("expected active pane information");
        assert!(inactive_pane.has_unseen_output);
        assert!(!active_pane.has_unseen_output);
        drop(events);

        seen.lock().unwrap().clear();
        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rterm_core::PaneId::new(1),
        })
        .unwrap();
        let _ = app.render_snapshot();

        let events = seen.lock().unwrap();
        let event = events
            .first()
            .expect("expected format-tab-title event to be dispatched");
        let focused_pane = event
            .panes
            .iter()
            .find(|pane| pane.pane_id == rterm_core::PaneId::new(1))
            .expect("expected focused pane information");
        assert!(!focused_pane.has_unseen_output);
    }

    #[test]
    fn window_app_tab_title_formatter_applies_format_item_colors() {
        let mut app = NativeWindowApp::new(None);
        app.tab_title_formatter = Box::new(|_| {
            Some(NativeTabTitle::Format(vec![
                NativeFormatItem::Foreground(rterm_terminal::Color::Rgb(1, 2, 3)),
                NativeFormatItem::Background(rterm_terminal::Color::Rgb(4, 5, 6)),
                NativeFormatItem::Text("styled".to_owned()),
            ]))
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let title_column = tab_bar
            .find("styled")
            .expect("formatted title should render in the tab bar");
        let title_cell = snapshot_cell(&snapshot, 0, u16::try_from(title_column).unwrap()).unwrap();

        assert_eq!(title_cell.ch, 's');
        assert_eq!(title_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_eq!(title_cell.background, rterm_terminal::Color::Rgb(4, 5, 6));
    }

    #[test]
    fn window_app_tab_title_formatter_text_items_apply_sgr_escapes() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.tab_title_formatter = Box::new(|_| {
            Some(NativeTabTitle::Format(vec![NativeFormatItem::Text(
                "\x1b[38;2;1;2;3;48;2;4;5;6;4:3mESC\x1b[0mBASE".to_owned(),
            )]))
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let styled_byte_offset = tab_bar
            .find("ESC")
            .expect("escaped title text should render without escape bytes");
        let base_byte_offset = tab_bar
            .find("BASE")
            .expect("reset title text should render without escape bytes");
        let styled_column = unicode_width::UnicodeWidthStr::width(&tab_bar[..styled_byte_offset]);
        let base_column = unicode_width::UnicodeWidthStr::width(&tab_bar[..base_byte_offset]);
        let styled_cell =
            snapshot_cell(&snapshot, 0, u16::try_from(styled_column).unwrap()).unwrap();
        let base_cell = snapshot_cell(&snapshot, 0, u16::try_from(base_column).unwrap()).unwrap();

        assert_eq!(styled_cell.ch, 'E');
        assert_eq!(styled_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_eq!(styled_cell.background, rterm_terminal::Color::Rgb(4, 5, 6));
        assert_eq!(
            styled_cell.underline_style,
            rterm_terminal::UnderlineStyle::Curly
        );
        assert_eq!(base_cell.ch, 'B');
        assert_eq!(
            base_cell.foreground,
            rterm_terminal::Color::Rgb(0xf8, 0xfa, 0xfc)
        );
        assert_eq!(
            base_cell.background,
            rterm_terminal::Color::Rgb(0x1b, 0x2b, 0x44)
        );
        assert_eq!(
            base_cell.underline_style,
            rterm_terminal::UnderlineStyle::None
        );
    }

    #[test]
    fn window_app_tab_title_formatter_text_items_strip_sgr_for_layout() {
        let mut app = NativeWindowApp::new(None);
        app.tab_title_formatter = Box::new(|_| {
            Some(NativeTabTitle::Format(vec![NativeFormatItem::Text(
                "\x1b[31mLAYOUT\x1b[0m".to_owned(),
            )]))
        });

        let visible_tab_bar = snapshot_row_text(&app.render_snapshot(), 0, TERMINAL_COLUMNS);
        assert!(
            visible_tab_bar.contains("LAYOUT"),
            "tab bar was {visible_tab_bar:?}"
        );
        assert!(
            !visible_tab_bar.contains('\x1b'),
            "rendered layout should omit SGR escapes: {visible_tab_bar:?}"
        );
        let plus_column = visible_tab_bar
            .find(" + ")
            .expect("new tab button should render after visible title");
        let title_end = visible_tab_bar.find("LAYOUT").unwrap() + "LAYOUT".len();

        assert!(plus_column >= title_end, "tab bar was {visible_tab_bar:?}");
    }

    #[test]
    fn window_app_tab_title_formatter_reset_attributes_restores_tab_style() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.tab_title_formatter = Box::new(|_| {
            Some(NativeTabTitle::Format(vec![
                NativeFormatItem::Foreground(rterm_terminal::Color::Rgb(1, 2, 3)),
                NativeFormatItem::Background(rterm_terminal::Color::Rgb(4, 5, 6)),
                NativeFormatItem::Text("hot".to_owned()),
                NativeFormatItem::ResetAttributes,
                NativeFormatItem::Text("plain".to_owned()),
            ]))
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let hot_byte_offset = tab_bar
            .find("hot")
            .expect("styled title should render in the tab bar");
        let plain_byte_offset = tab_bar
            .find("plain")
            .expect("reset title should render in the tab bar");
        let hot_column = unicode_width::UnicodeWidthStr::width(&tab_bar[..hot_byte_offset]);
        let plain_column = unicode_width::UnicodeWidthStr::width(&tab_bar[..plain_byte_offset]);
        let hot_cell = snapshot_cell(&snapshot, 0, u16::try_from(hot_column).unwrap()).unwrap();
        let plain_cell = snapshot_cell(&snapshot, 0, u16::try_from(plain_column).unwrap()).unwrap();

        assert_eq!(hot_cell.foreground, rterm_terminal::Color::Rgb(1, 2, 3));
        assert_eq!(hot_cell.background, rterm_terminal::Color::Rgb(4, 5, 6));
        assert_eq!(
            plain_cell.foreground,
            rterm_terminal::Color::Rgb(0xf8, 0xfa, 0xfc)
        );
        assert_eq!(
            plain_cell.background,
            rterm_terminal::Color::Rgb(0x1b, 0x2b, 0x44)
        );
    }

    #[test]
    fn window_app_tab_title_formatter_applies_intensity_attributes() {
        let mut app = NativeWindowApp::new(None);
        app.tab_title_formatter = Box::new(|_| {
            Some(NativeTabTitle::Format(vec![
                NativeFormatItem::Attribute(NativeFormatAttribute::Intensity(
                    NativeFormatIntensity::Normal,
                )),
                NativeFormatItem::Text("normal".to_owned()),
                NativeFormatItem::Attribute(NativeFormatAttribute::Intensity(
                    NativeFormatIntensity::Bold,
                )),
                NativeFormatItem::Text("bold".to_owned()),
                NativeFormatItem::Attribute(NativeFormatAttribute::Intensity(
                    NativeFormatIntensity::Normal,
                )),
                NativeFormatItem::Text("plain".to_owned()),
            ]))
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let normal_column = tab_bar
            .find("normal")
            .expect("normal title segment should render in the tab bar");
        let bold_column = tab_bar
            .find("bold")
            .expect("bold title segment should render in the tab bar");
        let plain_column = tab_bar
            .find("plain")
            .expect("plain title segment should render in the tab bar");

        assert!(
            !snapshot_cell(&snapshot, 0, u16::try_from(normal_column).unwrap())
                .unwrap()
                .bold
        );
        assert!(
            snapshot_cell(&snapshot, 0, u16::try_from(bold_column).unwrap())
                .unwrap()
                .bold
        );
        assert!(
            !snapshot_cell(&snapshot, 0, u16::try_from(plain_column).unwrap())
                .unwrap()
                .bold
        );
    }

    #[test]
    fn window_app_tab_title_formatter_applies_half_intensity_as_faint() {
        let mut app = NativeWindowApp::new(None);
        app.tab_title_formatter = Box::new(|_| {
            Some(NativeTabTitle::Format(vec![
                NativeFormatItem::Attribute(NativeFormatAttribute::Intensity(
                    NativeFormatIntensity::Half,
                )),
                NativeFormatItem::Text("faint".to_owned()),
                NativeFormatItem::Attribute(NativeFormatAttribute::Intensity(
                    NativeFormatIntensity::Normal,
                )),
                NativeFormatItem::Text("normal".to_owned()),
            ]))
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let faint_column = tab_bar
            .find("faint")
            .expect("half-intensity title segment should render in the tab bar");
        let normal_column = tab_bar
            .find("normal")
            .expect("normal-intensity title segment should render in the tab bar");

        assert!(
            snapshot_cell(&snapshot, 0, u16::try_from(faint_column).unwrap())
                .unwrap()
                .faint
        );
        assert!(
            !snapshot_cell(&snapshot, 0, u16::try_from(normal_column).unwrap())
                .unwrap()
                .faint
        );
    }

    #[test]
    fn window_app_tab_title_formatter_applies_italic_attributes() {
        let mut app = NativeWindowApp::new(None);
        app.tab_title_formatter = Box::new(|_| {
            Some(NativeTabTitle::Format(vec![
                NativeFormatItem::Attribute(NativeFormatAttribute::Italic(true)),
                NativeFormatItem::Text("slant".to_owned()),
                NativeFormatItem::Attribute(NativeFormatAttribute::Italic(false)),
                NativeFormatItem::Text("upright".to_owned()),
            ]))
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let slant_column = tab_bar
            .find("slant")
            .expect("italic title segment should render in the tab bar");
        let upright_column = tab_bar
            .find("upright")
            .expect("upright title segment should render in the tab bar");

        assert!(
            snapshot_cell(&snapshot, 0, u16::try_from(slant_column).unwrap())
                .unwrap()
                .italic
        );
        assert!(
            !snapshot_cell(&snapshot, 0, u16::try_from(upright_column).unwrap())
                .unwrap()
                .italic
        );
    }

    #[test]
    fn window_app_tab_title_formatter_applies_underline_attributes() {
        let mut app = NativeWindowApp::new(None);
        app.tab_title_formatter = Box::new(|_| {
            Some(NativeTabTitle::Format(vec![
                NativeFormatItem::Attribute(NativeFormatAttribute::Underline(
                    NativeFormatUnderline::Single,
                )),
                NativeFormatItem::Text("single".to_owned()),
                NativeFormatItem::Attribute(NativeFormatAttribute::Underline(
                    NativeFormatUnderline::Double,
                )),
                NativeFormatItem::Text("double".to_owned()),
                NativeFormatItem::Attribute(NativeFormatAttribute::Underline(
                    NativeFormatUnderline::Curly,
                )),
                NativeFormatItem::Text("curly".to_owned()),
                NativeFormatItem::Attribute(NativeFormatAttribute::Underline(
                    NativeFormatUnderline::Dotted,
                )),
                NativeFormatItem::Text("dotted".to_owned()),
                NativeFormatItem::Attribute(NativeFormatAttribute::Underline(
                    NativeFormatUnderline::Dashed,
                )),
                NativeFormatItem::Text("dashed".to_owned()),
                NativeFormatItem::Attribute(NativeFormatAttribute::Underline(
                    NativeFormatUnderline::None,
                )),
                NativeFormatItem::Text("plain".to_owned()),
            ]))
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        for (text, style) in [
            ("single", rterm_terminal::UnderlineStyle::Single),
            ("double", rterm_terminal::UnderlineStyle::Double),
            ("curly", rterm_terminal::UnderlineStyle::Curly),
            ("dotted", rterm_terminal::UnderlineStyle::Dotted),
            ("dashed", rterm_terminal::UnderlineStyle::Dashed),
            ("plain", rterm_terminal::UnderlineStyle::None),
        ] {
            let column = tab_bar
                .find(text)
                .unwrap_or_else(|| panic!("{text} title segment should render in the tab bar"));
            assert_eq!(
                snapshot_cell(&snapshot, 0, u16::try_from(column).unwrap())
                    .unwrap()
                    .underline_style,
                style
            );
        }
    }

    #[test]
    fn window_app_tab_title_formatter_marks_last_active_tab() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        app.tab_title_formatter = Box::new(move |event| {
            recorded
                .lock()
                .unwrap()
                .push((event.tab, event.is_active, event.is_last_active));
            None
        });

        app.render_snapshot();

        let events = seen.lock().unwrap();
        assert!(events.contains(&(rterm_core::TabId::new(1), true, false)));
        assert!(events.contains(&(rterm_core::TabId::new(2), false, true)));
    }

    #[test]
    fn window_app_tab_title_formatter_receives_default_max_width() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let mut app = NativeWindowApp::new(None);
        app.tab_title_formatter = Box::new(move |event| {
            recorded.lock().unwrap().push(event.max_width);
            None
        });

        app.render_snapshot();

        assert_eq!(seen.lock().unwrap().as_slice(), [16, 16]);
    }

    #[test]
    fn window_app_tab_title_formatter_receives_effective_config_snapshot() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let mut app = NativeWindowApp::new(None);
        app.tab_max_width = 24;
        app.status_update_interval = Duration::from_millis(750);
        app.tab_title_formatter = Box::new(move |event| {
            recorded.lock().unwrap().push(event.config.clone());
            None
        });

        app.render_snapshot();

        let events = seen.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|config| config.tab_max_width == 24));
        assert!(
            events
                .iter()
                .all(|config| config.status_update_interval_ms == 750)
        );
        assert!(events.iter().all(|config| config.mouse_wheel_scrolls_tabs));
        assert!(
            events
                .iter()
                .all(|config| config.show_close_tab_button_in_tabs)
        );
        assert!(
            events
                .iter()
                .all(|config| config.show_new_tab_button_in_tab_bar)
        );
        assert!(events.iter().all(|config| config.show_tab_index_in_tab_bar));
        assert!(events.iter().all(|config| config.show_tabs_in_tab_bar));
    }

    #[test]
    fn window_app_tab_title_formatter_second_pass_max_width_reflects_available_space() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(70, 2));
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.tab_title_formatter = Box::new(move |event| {
            recorded
                .lock()
                .unwrap()
                .push((event.tab, event.hover, event.max_width));
            None
        });

        app.render_snapshot();

        let events = seen.lock().unwrap();
        assert_eq!(events.len(), 6);
        let first_pass_widths = events
            .iter()
            .step_by(2)
            .map(|(_, _, max_width)| *max_width)
            .collect::<Vec<_>>();
        let second_pass_widths = events
            .iter()
            .skip(1)
            .step_by(2)
            .map(|(_, _, max_width)| *max_width)
            .collect::<Vec<_>>();
        assert_eq!(first_pass_widths, vec![16, 16, 16]);
        assert!(
            second_pass_widths.iter().all(|max_width| *max_width < 16),
            "second pass widths were {second_pass_widths:?}"
        );
    }

    #[test]
    fn window_app_tab_title_formatter_runs_wezterm_style_two_passes() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let mut app = NativeWindowApp::new(None);
        let first_tab_column = app.tab_bar_workspace_label().chars().count() + 1;
        let x = u32::try_from(first_tab_column).unwrap_or(0) * CELL_WIDTH;
        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        app.tab_title_formatter = Box::new(move |event| {
            recorded
                .lock()
                .unwrap()
                .push((event.tab, event.hover, event.max_width));
            None
        });

        app.render_snapshot();

        assert_eq!(
            seen.lock().unwrap().as_slice(),
            [
                (rterm_core::TabId::new(1), false, 16),
                (rterm_core::TabId::new(1), true, 16)
            ]
        );
    }

    #[test]
    fn window_app_tab_title_formatter_uses_unbounded_width_for_non_fancy_tab_bar() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let mut app = NativeWindowApp::new(None);
        let tab_max_width = app.tab_max_width;
        app.set_config_overrides(native_config_snapshot! {
            use_fancy_tab_bar: Some(false),
            ..NativeConfigSnapshot::default()
        });
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.tab_title_formatter = Box::new(move |event| {
            recorded
                .lock()
                .unwrap()
                .push((event.tab, event.hover, event.max_width));
            None
        });

        app.render_snapshot();

        let events = seen.lock().unwrap();
        assert!(
            events.len() >= 4,
            "expected two-pass events for both tabs; saw {events:?}"
        );
        let first_pass_widths = events
            .iter()
            .step_by(2)
            .map(|(_, _, max_width)| *max_width)
            .collect::<Vec<_>>();
        let second_pass_widths = events
            .iter()
            .skip(1)
            .step_by(2)
            .map(|(_, _, max_width)| *max_width)
            .collect::<Vec<_>>();

        assert!(
            first_pass_widths
                .iter()
                .all(|max_width| *max_width == tab_max_width),
            "first pass widths were {first_pass_widths:?}"
        );
        assert!(
            second_pass_widths
                .iter()
                .all(|max_width| *max_width == usize::MAX),
            "second pass widths were {second_pass_widths:?}"
        );
    }

    #[test]
    fn window_app_tab_title_formatter_marks_hovered_tab() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        let first_tab_column = app.tab_bar_workspace_label().chars().count() + 1;
        let x = u32::try_from(first_tab_column).unwrap_or(0) * CELL_WIDTH;
        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        app.tab_title_formatter = Box::new(move |event| {
            recorded.lock().unwrap().push((event.tab, event.hover));
            None
        });

        app.render_snapshot();

        let events = seen.lock().unwrap();
        assert!(events.contains(&(rterm_core::TabId::new(1), true)));
        assert!(events.contains(&(rterm_core::TabId::new(2), false)));
    }

    #[test]
    fn window_app_clicking_tab_bar_activates_tab() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));

        let first_tab_column = app.tab_bar_workspace_label().chars().count() + 1;
        let x = u32::try_from(first_tab_column).unwrap_or(0) * CELL_WIDTH;
        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
    }

    #[test]
    fn window_app_clicking_tab_bar_accounts_for_left_status_width() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.left_status = "LEFT".to_owned();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));

        let first_tab_label = tab_bar_tab_label(
            0,
            rterm_core::TabId::new(1),
            1,
            false,
            None,
            rterm_core::app_shell::PaneProgress::None,
        );
        let first_tab_column = app.tab_bar_workspace_label().chars().count()
            + "LEFT ".chars().count()
            + first_tab_label.chars().count()
            - 1;
        let x = u32::try_from(first_tab_column).unwrap_or(0) * CELL_WIDTH;
        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
    }

    #[test]
    fn tab_bar_renders_overflow_indicator_for_clipped_tabs() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(32, 2));
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let tab_bar = snapshot_row_text(&app.render_snapshot(), 0, 32);

        assert!(
            tab_bar.contains('…'),
            "clipped tabs should render an overflow indicator: {tab_bar:?}"
        );
    }

    #[test]
    fn tab_bar_overflow_indicator_does_not_target_tabs() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(32, 2));
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();

        let tab_bar = snapshot_row_text(&app.render_snapshot(), 0, 32);
        let overflow_column = tab_bar
            .find('…')
            .expect("clipped tabs should render an overflow indicator");
        let overflow_column = u16::try_from(overflow_column).unwrap();

        assert_eq!(app.tab_for_tab_bar_column(overflow_column), None);
        assert_eq!(app.close_tab_for_tab_bar_column(overflow_column), None);
        assert_eq!(app.tab_for_tab_bar_column(45), None);
        assert_eq!(app.close_tab_for_tab_bar_column(45), None);
    }

    #[test]
    fn tab_bar_render_and_hit_testing_share_formatted_segment_layout() {
        let mut app = NativeWindowApp::new(None);
        app.left_status = "\x1b[32mLEFT\x1b[0m".to_owned();
        app.tab_bar_style.active_tab_left = Some(vec![NativeFormatItem::Text("<".to_owned())]);
        app.tab_title_formatter = Box::new(|_| {
            Some(NativeTabTitle::Format(vec![NativeFormatItem::Text(
                "\x1b[34mFMT\x1b[0m".to_owned(),
            )]))
        });

        let tab_bar = snapshot_row_text(&app.render_snapshot(), 0, TERMINAL_COLUMNS);
        let title_column = u16::try_from(
            tab_bar
                .find("FMT")
                .expect("formatted title should render in the tab bar"),
        )
        .unwrap();
        let close_column = u16::try_from(
            tab_bar
                .find(" x ")
                .expect("formatted tab should retain its close segment")
                + 1,
        )
        .unwrap();

        assert_eq!(
            app.tab_for_tab_bar_column(title_column),
            Some(rterm_core::TabId::new(1))
        );
        assert_eq!(
            app.close_tab_for_tab_bar_column(close_column),
            Some(rterm_core::TabId::new(1))
        );
    }

    #[test]
    fn tab_bar_without_overflow_keeps_existing_targets_and_new_tab_button() {
        let app = NativeWindowApp::new(None);

        let tab_bar = snapshot_row_text(&app.render_snapshot(), 0, TERMINAL_COLUMNS);
        let tab_column = u16::try_from(tab_bar.find("panes:1").unwrap()).unwrap();
        let new_tab_column = u16::try_from(tab_bar.find(" + ").unwrap() + 1).unwrap();

        assert!(!tab_bar.contains('…'), "tab bar was {tab_bar:?}");
        assert_eq!(
            app.tab_for_tab_bar_column(tab_column),
            Some(rterm_core::TabId::new(1))
        );
        assert!(app.new_tab_button_for_tab_bar_column(new_tab_column));
    }

    #[test]
    fn tab_bar_hit_testing_reuses_and_refreshes_the_rendered_layout() {
        let calls = Arc::new(AtomicUsize::new(0));
        let title = Arc::new(Mutex::new("FIRST".to_owned()));
        let recorded_calls = Arc::clone(&calls);
        let recorded_title = Arc::clone(&title);
        let mut app = NativeWindowApp::new(None);
        app.tab_title_formatter = Box::new(move |_| {
            recorded_calls.fetch_add(1, Ordering::Relaxed);
            Some(NativeTabTitle::Text(recorded_title.lock().unwrap().clone()))
        });

        let first_tab_bar = snapshot_row_text(&app.render_snapshot(), 0, TERMINAL_COLUMNS);
        let title_column = u16::try_from(first_tab_bar.find("FIRST").unwrap()).unwrap();
        let close_column =
            u16::try_from(first_tab_bar.find(" x ").expect("close segment") + 1).unwrap();
        let new_tab_column =
            u16::try_from(first_tab_bar.find(" + ").expect("new tab button") + 1).unwrap();
        let calls_after_render = calls.load(Ordering::Relaxed);

        assert_eq!(
            app.tab_for_tab_bar_column(title_column),
            Some(rterm_core::TabId::new(1))
        );
        assert_eq!(
            app.close_tab_for_tab_bar_column(close_column),
            Some(rterm_core::TabId::new(1))
        );
        assert!(app.new_tab_button_for_tab_bar_column(new_tab_column));
        assert_eq!(
            calls.load(Ordering::Relaxed),
            calls_after_render,
            "hit testing must consume the rendered ledger without rerunning formatters"
        );

        *title.lock().unwrap() = "SECOND".to_owned();
        let second_tab_bar = snapshot_row_text(&app.render_snapshot(), 0, TERMINAL_COLUMNS);
        assert!(
            second_tab_bar.contains("SECOND"),
            "tab bar was {second_tab_bar:?}"
        );
        assert_eq!(
            calls.load(Ordering::Relaxed),
            calls_after_render + 2,
            "the next render must atomically refresh the two-pass ledger"
        );
    }

    #[test]
    fn tab_bar_extreme_narrow_width_prioritizes_overflow_over_new_tab() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(15, 2));

        let tab_bar = snapshot_row_text(&app.render_snapshot(), 0, 15);
        let overflow_column =
            u16::try_from(tab_bar.find('…').expect("overflow indicator")).unwrap();

        assert!(!tab_bar.contains(" + "), "tab bar was {tab_bar:?}");
        assert_eq!(app.tab_for_tab_bar_column(overflow_column), None);
        assert_eq!(app.close_tab_for_tab_bar_column(overflow_column), None);
        assert!(!app.new_tab_button_for_tab_bar_column(overflow_column));
    }

    #[test]
    fn tab_bar_overflow_never_overwrites_reserved_right_status() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(18, 2));
        app.right_status = "RIGHT".to_owned();

        let tab_bar = snapshot_row_text(&app.render_snapshot(), 0, 18);
        let overflow_column =
            u16::try_from(tab_bar.find('…').expect("overflow indicator")).unwrap();

        assert!(tab_bar.ends_with("RIGHT"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains(" + "), "tab bar was {tab_bar:?}");
        assert_eq!(app.tab_for_tab_bar_column(overflow_column), None);
        assert_eq!(app.close_tab_for_tab_bar_column(overflow_column), None);
    }

    fn rendered_tab_body_columns(app: &mut NativeWindowApp) -> Vec<(rterm_core::TabId, u16)> {
        app.render_snapshot();
        app.rendered_tab_bar_layout
            .borrow()
            .as_ref()
            .expect("rendered tab bar layout")
            .tabs
            .iter()
            .map(|tab| {
                let column = (tab.start_column..tab.end_column)
                    .find(|column| tab.close_column != Some(*column))
                    .expect("visible tab body column");
                (tab.tab_id, column)
            })
            .collect()
    }

    fn move_mouse_to_tab_bar_column(app: &mut NativeWindowApp, column: u16) {
        let x = app
            .frame_content_pixel_left()
            .saturating_add(u32::from(column) * app.cell_width())
            .saturating_add(app.cell_width() / 2);
        let y = app
            .tab_bar_pixel_top()
            .saturating_add(app.tab_bar_pixel_height() / 2);
        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), f64::from(y)))
            .unwrap();
    }

    fn move_mouse_to_rendered_new_tab_button(app: &mut NativeWindowApp) {
        app.render_snapshot();
        let column = app
            .rendered_tab_bar_layout
            .borrow()
            .as_ref()
            .and_then(|layout| layout.new_tab_start_column)
            .expect("rendered new-tab button layout");
        move_mouse_to_tab_bar_column(app, column);
    }

    fn active_workspace_tab_ids(app: &NativeWindowApp) -> Vec<rterm_core::TabId> {
        app.app_shell
            .active_workspace()
            .tabs()
            .iter()
            .map(rterm_core::app_shell::Tab::id)
            .collect()
    }

    #[test]
    fn tab_bar_overflow_keeps_newly_activated_tab_visible() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(32, 2));
        for _ in 0..4 {
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
        }
        let active_tab = app.active_tab_id();

        let tab_bar = snapshot_row_text(&app.render_snapshot(), 0, 32);
        let layout = app.rendered_tab_bar_layout.borrow();
        let layout = layout.as_ref().expect("rendered tab bar layout");

        assert!(
            layout.tabs.iter().any(|tab| tab.tab_id == active_tab),
            "active tab {active_tab:?} must remain visible in {tab_bar:?}"
        );
        assert!(layout.leading_overflow_column.is_some());
        assert!(tab_bar.contains('‹'), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn middle_click_closes_visible_tab_without_activating_it() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        let active_tab = app.active_tab_id();
        let columns = rendered_tab_body_columns(&mut app);
        let (inactive_tab, inactive_column) = columns
            .iter()
            .find(|(tab, _)| *tab != active_tab)
            .copied()
            .expect("inactive tab body");

        move_mouse_to_tab_bar_column(&mut app, inactive_column);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Middle)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), active_tab);
        assert!(!active_workspace_tab_ids(&app).contains(&inactive_tab));
    }

    #[test]
    fn dragging_tab_bar_reorders_tabs() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        let columns = rendered_tab_body_columns(&mut app);
        let source_column = columns
            .iter()
            .find(|(tab, _)| *tab == rterm_core::TabId::new(1))
            .unwrap()
            .1;
        move_mouse_to_tab_bar_column(&mut app, source_column);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        let redrawn_columns = rendered_tab_body_columns(&mut app);
        let redrawn_target_column = redrawn_columns
            .iter()
            .find(|(tab, _)| *tab == rterm_core::TabId::new(3))
            .unwrap()
            .1;
        assert_ne!(
            app.rendered_tab_bar_layout
                .borrow()
                .as_ref()
                .unwrap()
                .generation,
            0
        );
        move_mouse_to_tab_bar_column(&mut app, redrawn_target_column);
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(
            active_workspace_tab_ids(&app),
            vec![
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(3),
                rterm_core::TabId::new(1),
            ]
        );
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        let refreshed_columns = rendered_tab_body_columns(&mut app);
        assert_eq!(
            refreshed_columns
                .iter()
                .map(|(tab, _)| *tab)
                .collect::<Vec<_>>(),
            active_workspace_tab_ids(&app),
            "the next render must refresh the ledger in the new order"
        );
    }

    #[test]
    fn dragging_tab_bar_to_same_target_is_a_no_op() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        let before = active_workspace_tab_ids(&app);
        let source_column = rendered_tab_body_columns(&mut app)[0].1;

        move_mouse_to_tab_bar_column(&mut app, source_column);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(active_workspace_tab_ids(&app), before);
    }

    #[test]
    fn tab_drag_revalidates_source_and_target_ids_after_tab_order_changes() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        for _ in 0..3 {
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
        }
        let columns = rendered_tab_body_columns(&mut app);
        let source_column = columns
            .iter()
            .find(|(tab, _)| *tab == rterm_core::TabId::new(1))
            .unwrap()
            .1;
        let target_column = columns
            .iter()
            .find(|(tab, _)| *tab == rterm_core::TabId::new(3))
            .unwrap()
            .1;

        move_mouse_to_tab_bar_column(&mut app, source_column);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        app.dispatch_app_action(AppAction::MoveTab { index: 2 })
            .unwrap();
        assert_eq!(
            active_workspace_tab_ids(&app),
            vec![
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(3),
                rterm_core::TabId::new(1),
                rterm_core::TabId::new(4),
            ]
        );

        move_mouse_to_tab_bar_column(&mut app, target_column);
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(
            active_workspace_tab_ids(&app),
            vec![
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(1),
                rterm_core::TabId::new(3),
                rterm_core::TabId::new(4),
            ]
        );
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
    }

    #[test]
    fn tab_drag_does_not_start_before_the_first_rendered_ledger() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        let before = active_workspace_tab_ids(&app);
        let first_tab_column =
            u16::try_from(app.tab_bar_workspace_label().chars().count() + 1).unwrap();

        move_mouse_to_tab_bar_column(&mut app, first_tab_column);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        let target_column = app
            .rendered_tab_bar_layout
            .borrow()
            .as_ref()
            .expect("ordinary click fallback may populate click hit testing")
            .tabs
            .last()
            .map(|tab| tab.start_column)
            .unwrap();
        move_mouse_to_tab_bar_column(&mut app, target_column);
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(active_workspace_tab_ids(&app), before);

        let fallback_columns = app
            .rendered_tab_bar_layout
            .borrow()
            .as_ref()
            .unwrap()
            .tabs
            .iter()
            .map(|tab| {
                (
                    tab.tab_id,
                    (tab.start_column..tab.end_column)
                        .find(|column| tab.close_column != Some(*column))
                        .unwrap(),
                )
            })
            .collect::<Vec<_>>();
        move_mouse_to_tab_bar_column(&mut app, fallback_columns[0].1);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert!(
            app.tab_bar_drag.is_none(),
            "a cached fallback ledger must not arm drag state"
        );
        move_mouse_to_tab_bar_column(&mut app, fallback_columns[2].1);
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );
        assert_eq!(
            active_workspace_tab_ids(&app),
            before,
            "a cached fallback ledger must never become a drag source"
        );
    }

    #[test]
    fn dragging_tab_bar_reorder_ignores_hidden_overflow_target() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.runtime.resize(rterm_core::TerminalSize::new(32, 2));
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        let before = active_workspace_tab_ids(&app);
        let columns = rendered_tab_body_columns(&mut app);
        let source_column = columns[0].1;
        let overflow_column = app
            .rendered_tab_bar_layout
            .borrow()
            .as_ref()
            .and_then(|layout| layout.overflow_column)
            .expect("narrow tab bar overflow column");

        move_mouse_to_tab_bar_column(&mut app, source_column);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        move_mouse_to_tab_bar_column(&mut app, overflow_column);
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap(),
            "the paired tab UI release must remain consumed"
        );

        assert_eq!(active_workspace_tab_ids(&app), before);
    }

    #[test]
    fn dragging_active_tab_preserves_tab_identity_and_runtime_owners() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        let active_tab = app.active_tab_id();
        let active_pane = app.active_pane_id();
        let inactive_runtime_owners = app.pane_runtimes.keys().copied().collect::<HashSet<_>>();
        let columns = rendered_tab_body_columns(&mut app);
        let source_column = columns
            .iter()
            .find(|(tab, _)| *tab == active_tab)
            .unwrap()
            .1;
        let target_column = columns[0].1;

        move_mouse_to_tab_bar_column(&mut app, source_column);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        move_mouse_to_tab_bar_column(&mut app, target_column);
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), active_tab);
        assert_eq!(app.active_pane_id(), active_pane);
        assert_eq!(
            app.pane_runtimes.keys().copied().collect::<HashSet<_>>(),
            inactive_runtime_owners
        );
    }

    #[test]
    fn tab_drag_hit_testing_does_not_rerun_stateful_formatter() {
        let calls = Arc::new(AtomicUsize::new(0));
        let recorded_calls = Arc::clone(&calls);
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.tab_title_formatter = Box::new(move |_| {
            recorded_calls.fetch_add(1, Ordering::Relaxed);
            Some(NativeTabTitle::Text("STATEFUL".to_owned()))
        });
        let columns = rendered_tab_body_columns(&mut app);
        let calls_after_render = calls.load(Ordering::Relaxed);

        move_mouse_to_tab_bar_column(&mut app, columns[0].1);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        move_mouse_to_tab_bar_column(&mut app, columns[2].1);
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(
            calls.load(Ordering::Relaxed),
            calls_after_render,
            "press, drag, and release must only use the last rendered ledger"
        );
    }

    #[test]
    fn tab_drag_consumes_full_mouse_sequence_and_focus_loss_cancels_safely() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            mouse_assignments: Some(
                [
                    NativeMouseAssignmentEventKind::Down,
                    NativeMouseAssignmentEventKind::Drag,
                    NativeMouseAssignmentEventKind::Up,
                ]
                .into_iter()
                .map(|kind| NativeUserMouseAssignment {
                    event: NativeMouseAssignmentEvent {
                        kind,
                        button: NativeMouseAssignmentButton::Mouse(MouseButton::Left),
                        streak: 1,
                    },
                    modifiers: ModifiersState::empty(),
                    mouse_reporting: true,
                    alt_screen: NativeMouseAssignmentAltScreen::Any,
                    command: WindowCommand::StartWindowDrag,
                })
                .collect(),
            ),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"\x1b[?1000;1006h").unwrap();
        let columns = rendered_tab_body_columns(&mut app);

        move_mouse_to_tab_bar_column(&mut app, columns[0].1);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        move_mouse_to_tab_bar_column(&mut app, columns[2].1);
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );
        assert!(!app.window_drag_requested_for_test());
        assert!(written.lock().unwrap().is_empty());
        assert!(app.selection.is_none());
        assert!(!app.selecting);

        let before_focus_loss = active_workspace_tab_ids(&app);
        let columns = rendered_tab_body_columns(&mut app);
        move_mouse_to_tab_bar_column(&mut app, columns[0].1);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert!(!app.handle_focus_changed(true).unwrap());
        assert!(app.handle_focus_changed(false).unwrap());
        move_mouse_to_tab_bar_column(&mut app, columns[2].1);
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(active_workspace_tab_ids(&app), before_focus_loss);
        assert!(!app.window_drag_requested_for_test());
        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn tab_drag_cancels_if_the_source_disappears_from_the_rendered_ledger() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        let columns = rendered_tab_body_columns(&mut app);
        let source = columns[0].0;

        move_mouse_to_tab_bar_column(&mut app, columns[0].1);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        app.dispatch_app_action(AppAction::CloseTab {
            tab: source,
            switch_to_last_active: false,
        })
        .unwrap();
        let remaining_columns = rendered_tab_body_columns(&mut app);
        let before_release = active_workspace_tab_ids(&app);
        move_mouse_to_tab_bar_column(&mut app, remaining_columns[0].1);
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(active_workspace_tab_ids(&app), before_release);
    }

    #[test]
    fn tab_drag_cancels_if_the_target_disappears_from_the_workspace() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        let columns = rendered_tab_body_columns(&mut app);
        let source = app.active_tab_id();
        let source_column = columns.iter().find(|(tab, _)| *tab == source).unwrap().1;
        let (target, target_column) = columns
            .iter()
            .find(|(tab, _)| *tab != source)
            .copied()
            .unwrap();

        move_mouse_to_tab_bar_column(&mut app, source_column);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        app.dispatch_app_action(AppAction::CloseTab {
            tab: target,
            switch_to_last_active: false,
        })
        .unwrap();
        let before_release = active_workspace_tab_ids(&app);
        move_mouse_to_tab_bar_column(&mut app, target_column);
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(active_workspace_tab_ids(&app), before_release);
        assert_eq!(app.active_tab_id(), source);
    }

    #[test]
    fn tab_drag_cancels_if_another_tab_becomes_active() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        let columns = rendered_tab_body_columns(&mut app);
        let source = rterm_core::TabId::new(1);
        let source_column = columns.iter().find(|(tab, _)| *tab == source).unwrap().1;
        let target_column = columns
            .iter()
            .find(|(tab, _)| *tab == rterm_core::TabId::new(3))
            .unwrap()
            .1;

        move_mouse_to_tab_bar_column(&mut app, source_column);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(2),
        })
        .unwrap();
        let before_release = active_workspace_tab_ids(&app);
        move_mouse_to_tab_bar_column(&mut app, target_column);
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(active_workspace_tab_ids(&app), before_release);
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
    }

    #[test]
    fn tab_drag_revalidates_ids_when_a_real_render_replaces_the_pressed_ledger() {
        let title = Arc::new(Mutex::new("BEFORE".to_owned()));
        let formatted_title = Arc::clone(&title);
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.runtime.resize(rterm_core::TerminalSize::new(200, 2));
        app.frame_width = 200 * CELL_WIDTH;
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.tab_title_formatter = Box::new(move |_| {
            Some(NativeTabTitle::Text(
                formatted_title.lock().unwrap().clone(),
            ))
        });
        let pressed_columns = rendered_tab_body_columns(&mut app);
        let source_column = pressed_columns
            .iter()
            .find(|(tab, _)| *tab == app.active_tab_id())
            .unwrap()
            .1;
        move_mouse_to_tab_bar_column(&mut app, source_column);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        *title.lock().unwrap() = "AFTER-REFRESH".to_owned();
        app.left_status = "REFRESHED".to_owned();
        let refreshed_columns = rendered_tab_body_columns(&mut app);
        let refreshed_target_column = refreshed_columns
            .iter()
            .find(|(tab, _)| *tab != app.active_tab_id())
            .unwrap()
            .1;
        move_mouse_to_tab_bar_column(&mut app, refreshed_target_column);
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(
            active_workspace_tab_ids(&app),
            vec![
                rterm_core::TabId::new(3),
                rterm_core::TabId::new(1),
                rterm_core::TabId::new(2),
            ]
        );
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(3));
        let next = snapshot_row_text(&app.render_snapshot(), 0, TERMINAL_COLUMNS);
        assert!(next.contains("AFTER-REFRESH"), "tab bar was {next:?}");
        assert_eq!(
            app.rendered_tab_bar_layout
                .borrow()
                .as_ref()
                .unwrap()
                .tabs
                .iter()
                .map(|tab| tab.tab_id)
                .collect::<Vec<_>>(),
            active_workspace_tab_ids(&app)
        );
    }

    #[test]
    fn tab_drag_focus_loss_pending_release_blocks_terminal_mouse_move() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            mouse_assignments: Some(vec![NativeUserMouseAssignment {
                event: NativeMouseAssignmentEvent {
                    kind: NativeMouseAssignmentEventKind::Drag,
                    button: NativeMouseAssignmentButton::Mouse(MouseButton::Left),
                    streak: 1,
                },
                modifiers: ModifiersState::empty(),
                mouse_reporting: false,
                alt_screen: NativeMouseAssignmentAltScreen::Any,
                command: WindowCommand::SendString("drag-leaked".to_owned()),
            }]),
            ..NativeConfigSnapshot::default()
        });
        let columns = rendered_tab_body_columns(&mut app);
        let source_column = columns
            .iter()
            .find(|(tab, _)| *tab == app.active_tab_id())
            .unwrap()
            .1;

        move_mouse_to_tab_bar_column(&mut app, source_column);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert!(!app.handle_focus_changed(true).unwrap());
        assert!(app.handle_focus_changed(false).unwrap());
        assert!(app.handle_focus_changed(true).unwrap());
        assert_eq!(app.active_mouse_button, Some(MouseButton::Left));
        assert!(
            app.handle_cursor_moved(PhysicalPosition::new(
                f64::from(app.cell_width()),
                f64::from(app.terminal_pixel_top() + app.cell_height()),
            ))
            .unwrap()
        );

        assert!(written.lock().unwrap().is_empty());
        assert!(app.selection.is_none());
        assert!(!app.selecting);
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );
        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn tab_drag_release_cancels_on_blank_new_close_and_right_status_cells() {
        #[derive(Clone, Copy)]
        enum InvalidDropTarget {
            Blank,
            NewTab,
            Close,
            RightStatus,
        }

        for target in [
            InvalidDropTarget::Blank,
            InvalidDropTarget::NewTab,
            InvalidDropTarget::Close,
            InvalidDropTarget::RightStatus,
        ] {
            let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
            app.runtime.resize(rterm_core::TerminalSize::new(120, 2));
            app.frame_width = 120 * CELL_WIDTH;
            app.right_status = "RIGHT".to_owned();
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
            app.dispatch_app_action(AppAction::NewTab { launch: None })
                .unwrap();
            let columns = rendered_tab_body_columns(&mut app);
            let source_column = columns
                .iter()
                .find(|(tab, _)| *tab == app.active_tab_id())
                .unwrap()
                .1;
            let (new_tab_start, new_tab_end, close_column) = {
                let layout = app.rendered_tab_bar_layout.borrow();
                let layout = layout.as_ref().unwrap();
                (
                    layout.new_tab_start_column.unwrap(),
                    layout.new_tab_end_column.unwrap(),
                    layout.tabs[0].close_column.unwrap(),
                )
            };
            let right_status_start = 120_u16 - 5;
            let target_column = match target {
                InvalidDropTarget::Blank => {
                    assert!(new_tab_end < right_status_start);
                    new_tab_end
                }
                InvalidDropTarget::NewTab => new_tab_start,
                InvalidDropTarget::Close => close_column,
                InvalidDropTarget::RightStatus => right_status_start,
            };
            let before = active_workspace_tab_ids(&app);

            move_mouse_to_tab_bar_column(&mut app, source_column);
            assert!(
                app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                    .unwrap()
            );
            move_mouse_to_tab_bar_column(&mut app, target_column);
            assert!(
                app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                    .unwrap()
            );

            assert_eq!(active_workspace_tab_ids(&app), before);
        }
    }

    #[test]
    fn tab_drag_focus_loss_latch_clears_on_next_press() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.set_config_overrides(native_config_snapshot! {
            swallow_mouse_click_on_window_focus: Some(false),
            ..NativeConfigSnapshot::default()
        });
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        let columns = rendered_tab_body_columns(&mut app);
        let first_column = columns[0].1;

        move_mouse_to_tab_bar_column(&mut app, first_column);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert!(!app.handle_focus_changed(true).unwrap());
        assert!(app.handle_focus_changed(false).unwrap());
        assert!(app.handle_focus_changed(true).unwrap());
        let columns = rendered_tab_body_columns(&mut app);
        let first_column = columns[0].1;
        let last_column = columns[2].1;
        move_mouse_to_tab_bar_column(&mut app, first_column);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        move_mouse_to_tab_bar_column(&mut app, last_column);
        assert!(
            app.handle_mouse_input(ElementState::Released, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(
            active_workspace_tab_ids(&app),
            vec![
                rterm_core::TabId::new(2),
                rterm_core::TabId::new(3),
                rterm_core::TabId::new(1),
            ]
        );
    }

    #[test]
    fn window_app_clicking_tab_bar_close_marker_closes_tab() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));

        let first_tab_width = tab_bar_tab_label(
            0,
            rterm_core::TabId::new(1),
            1,
            false,
            None,
            rterm_core::app_shell::PaneProgress::None,
        )
        .chars()
        .count();
        let second_tab_label = tab_bar_tab_label(
            1,
            rterm_core::TabId::new(2),
            1,
            true,
            None,
            rterm_core::app_shell::PaneProgress::None,
        );
        let second_tab_start = app.tab_bar_workspace_label().chars().count() + first_tab_width;
        let close_offset = second_tab_label
            .chars()
            .position(|character| character == 'x')
            .expect("tab label should expose close marker");
        let x = u32::try_from(second_tab_start + close_offset).unwrap_or(0) * CELL_WIDTH;

        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_tab_bar_close_marker_can_switch_to_last_active_tab() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(3),
        })
        .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            switch_to_last_active_tab_when_closing_tab: Some(true),
            ..NativeConfigSnapshot::default()
        });

        let _ = rendered_tab_body_columns(&mut app);
        let close_column = app.rendered_tab_bar_layout.borrow().as_ref().unwrap()
            .tabs.iter().find(|tab| tab.tab_id == rterm_core::TabId::new(3))
            .and_then(|tab| tab.close_column).expect("third tab close marker");
        move_mouse_to_tab_bar_column(&mut app, close_column);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert!(!app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_clicking_last_tab_bar_close_marker_requests_window_close() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;

        let tab_label = tab_bar_tab_label(
            0,
            rterm_core::TabId::new(1),
            1,
            true,
            None,
            rterm_core::app_shell::PaneProgress::None,
        );
        let close_offset = tab_label
            .chars()
            .position(|character| character == 'x')
            .expect("tab label should expose close marker");
        let close_column = app.tab_bar_workspace_label().chars().count() + close_offset;
        let x = u32::try_from(close_column).unwrap_or(0) * CELL_WIDTH;

        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert!(app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_clicking_tab_bar_new_tab_button_opens_session_launcher() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains(tab_bar_new_tab_label()));

        move_mouse_to_rendered_new_tab_button(&mut app);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
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
    fn window_app_can_hide_tab_bar_new_tab_button() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.set_config_overrides(native_config_snapshot! {
            show_new_tab_button_in_tab_bar: Some(false),
            window_decorations: Some(NativeWindowDecorations {
                title: false,
                resize: false,
                integrated_buttons: false,
                macos_force_disable_shadow: false,
                macos_force_enable_shadow: false,
                macos_force_square_corners: false,
                macos_use_background_color_as_titlebar_color: false,
            }),
            ..NativeConfigSnapshot::default()
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(!tab_bar.contains(" + "), "tab bar was {tab_bar:?}");

        let hidden_new_tab_column = app
            .rendered_tab_bar_layout
            .borrow()
            .as_ref()
            .and_then(|layout| layout.tabs.last())
            .map(|tab| tab.end_column)
            .expect("tab layout should expose the blank column after the tab");
        move_mouse_to_tab_bar_column(&mut app, hidden_new_tab_column);
        assert!(
            !app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
    }

    #[test]
    fn window_app_can_hide_tab_labels_in_tab_bar() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            show_tabs_in_tab_bar: Some(false),
            window_decorations: Some(NativeWindowDecorations {
                title: false,
                resize: false,
                integrated_buttons: false,
                macos_force_disable_shadow: false,
                macos_force_enable_shadow: false,
                macos_force_square_corners: false,
                macos_use_background_color_as_titlebar_color: false,
            }),
            ..NativeConfigSnapshot::default()
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("ws:default"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("panes"), "tab bar was {tab_bar:?}");
        assert!(tab_bar.contains(" + "), "tab bar was {tab_bar:?}");

        let (hidden_tab_column, blank_column) = {
            let rendered_layout = app.rendered_tab_bar_layout.borrow();
            let layout = rendered_layout
                .as_ref()
                .expect("rendered tab bar layout");
            let new_tab_start = layout
                .new_tab_start_column
                .expect("new-tab button layout");
            let new_tab_end = layout.new_tab_end_column.expect("new-tab button end");
            (new_tab_start, new_tab_end)
        };
        assert_eq!(app.tab_for_tab_bar_column(hidden_tab_column), None);
        assert_eq!(app.close_tab_for_tab_bar_column(hidden_tab_column), None);

        move_mouse_to_tab_bar_column(&mut app, blank_column);
        assert!(
            !app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);

        move_mouse_to_rendered_new_tab_button(&mut app);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
        assert_eq!(
            app.command_palette
                .as_ref()
                .expect("new-tab button should open the launcher")
                .title(),
            "Launcher"
        );
    }

    #[test]
    fn window_app_can_hide_tab_index_in_tab_bar() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            show_tab_index_in_tab_bar: Some(false),
            ..NativeConfigSnapshot::default()
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(!tab_bar.contains("1:"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("2:"), "tab bar was {tab_bar:?}");
        assert!(tab_bar.contains("panes:1"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_can_show_zero_based_tab_indices_in_tab_bar() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            tab_and_split_indices_are_zero_based: Some(true),
            ..NativeConfigSnapshot::default()
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(tab_bar.contains("0:1 panes:1"), "tab bar was {tab_bar:?}");
        assert!(tab_bar.contains("1:2* panes:1"), "tab bar was {tab_bar:?}");
        assert!(!tab_bar.contains("2:2*"), "tab bar was {tab_bar:?}");
    }

    #[test]
    fn window_app_can_hide_tab_close_buttons_in_tab_bar() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::NewTab { launch: None })
            .unwrap();
        app.set_config_overrides(native_config_snapshot! {
            show_close_tab_button_in_tabs: Some(false),
            ..NativeConfigSnapshot::default()
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        assert!(!tab_bar.contains(" x "), "tab bar was {tab_bar:?}");
        assert!(tab_bar.contains("panes:1"), "tab bar was {tab_bar:?}");

        let closable_label = tab_bar_tab_label(
            0,
            rterm_core::TabId::new(1),
            1,
            false,
            None,
            rterm_core::app_shell::PaneProgress::None,
        );
        let close_offset = closable_label.find('x').expect("expected close marker");
        let close_column = app.tab_bar_workspace_label().chars().count() + close_offset;
        assert_eq!(
            app.close_tab_for_tab_bar_column(u16::try_from(close_column).unwrap_or(0)),
            None
        );
    }

    #[test]
    fn window_app_renders_integrated_title_buttons_left_aligned_in_configured_order() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            window_decorations: Some(NativeWindowDecorations {
                title: false,
                resize: true,
                integrated_buttons: true,
                macos_force_disable_shadow: false,
                macos_force_enable_shadow: false,
                macos_force_square_corners: false,
                macos_use_background_color_as_titlebar_color: false,
            }),
            integrated_title_buttons: Some(vec![
                NativeIntegratedTitleButton::Close,
                NativeIntegratedTitleButton::Hide,
            ]),
            integrated_title_button_alignment: Some(NativeIntegratedTitleButtonAlignment::Left),
            integrated_title_button_style: Some(NativeIntegratedTitleButtonStyle::Windows),
            ..NativeConfigSnapshot::default()
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);

        assert!(
            tab_bar.starts_with(" ×  —  ws:default"),
            "tab bar was {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_renders_right_aligned_integrated_title_buttons_after_status() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            window_decorations: Some(NativeWindowDecorations {
                title: false,
                resize: true,
                integrated_buttons: true,
                macos_force_disable_shadow: false,
                macos_force_enable_shadow: false,
                macos_force_square_corners: false,
                macos_use_background_color_as_titlebar_color: false,
            }),
            integrated_title_buttons: Some(vec![
                NativeIntegratedTitleButton::Hide,
                NativeIntegratedTitleButton::Maximize,
                NativeIntegratedTitleButton::Close,
            ]),
            integrated_title_button_style: Some(NativeIntegratedTitleButtonStyle::Windows),
            ..NativeConfigSnapshot::default()
        });
        app.set_right_status("READY".to_owned());

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);

        assert!(tab_bar.ends_with(" —  □  × "), "tab bar was {tab_bar:?}");
        assert!(
            tab_bar.contains("READY —  □  × "),
            "tab bar was {tab_bar:?}"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn window_app_macos_native_integrated_title_buttons_reserve_top_retro_space() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.set_config_overrides(native_config_snapshot! {
            window_decorations: Some(NativeWindowDecorations {
                title: true,
                resize: true,
                integrated_buttons: true,
                macos_force_disable_shadow: false,
                macos_force_enable_shadow: false,
                macos_force_square_corners: false,
                macos_use_background_color_as_titlebar_color: false,
            }),
            integrated_title_button_style: Some(NativeIntegratedTitleButtonStyle::MacOsNative),
            integrated_title_button_alignment: Some(NativeIntegratedTitleButtonAlignment::Left),
            use_fancy_tab_bar: Some(false),
            tab_bar_at_bottom: Some(false),
            ..NativeConfigSnapshot::default()
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);

        assert_eq!(&tab_bar[..10], "          ");
        assert!(
            tab_bar[10..].starts_with(" ws:default"),
            "tab bar was {tab_bar:?}"
        );
        assert_eq!(app.integrated_title_button_for_tab_bar_column(1), None);

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH),
            f64::from(app.tab_bar_pixel_top()) + 1.0,
        ))
        .unwrap();
        assert!(
            !app.handle_mouse_input(ElementState::Pressed, MouseButton::Right)
                .unwrap()
        );
        assert!(!app.window_hide_requested_for_test());
    }

    #[test]
    fn window_app_macos_native_integrated_title_buttons_skip_top_retro_space_when_fancy() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.set_config_overrides(native_config_snapshot! {
            window_decorations: Some(NativeWindowDecorations {
                title: false,
                resize: true,
                integrated_buttons: true,
                macos_force_disable_shadow: false,
                macos_force_enable_shadow: false,
                macos_force_square_corners: false,
                macos_use_background_color_as_titlebar_color: false,
            }),
            integrated_title_button_style: Some(NativeIntegratedTitleButtonStyle::MacOsNative),
            integrated_title_button_alignment: Some(NativeIntegratedTitleButtonAlignment::Left),
            use_fancy_tab_bar: Some(true),
            tab_bar_at_bottom: Some(false),
            ..NativeConfigSnapshot::default()
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);

        assert!(
            tab_bar.starts_with(" ws:default"),
            "tab bar was {tab_bar:?}"
        );
        assert_eq!(app.integrated_title_button_for_tab_bar_column(1), None);

        app.handle_cursor_moved(PhysicalPosition::new(f64::from(CELL_WIDTH), 0.0))
            .unwrap();
        assert!(
            !app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert!(!app.window_hide_requested_for_test());
    }

    #[test]
    fn window_app_macos_native_integrated_title_buttons_defaults_to_fancy_and_skips_top_retro_space()
     {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.set_config_overrides(native_config_snapshot! {
            window_decorations: Some(NativeWindowDecorations {
                title: false,
                resize: true,
                integrated_buttons: true,
                macos_force_disable_shadow: false,
                macos_force_enable_shadow: false,
                macos_force_square_corners: false,
                macos_use_background_color_as_titlebar_color: false,
            }),
            integrated_title_button_style: Some(NativeIntegratedTitleButtonStyle::MacOsNative),
            integrated_title_button_alignment: Some(NativeIntegratedTitleButtonAlignment::Left),
            tab_bar_at_bottom: Some(false),
            ..NativeConfigSnapshot::default()
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);

        assert!(
            tab_bar.starts_with(" ws:default"),
            "tab bar was {tab_bar:?}"
        );

        app.handle_cursor_moved(PhysicalPosition::new(f64::from(CELL_WIDTH), 0.0))
            .unwrap();
        assert!(
            !app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert!(!app.window_hide_requested_for_test());
    }

    #[test]
    fn window_app_applies_wezterm_tab_bar_style_integrated_title_button_labels() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_decorations = "INTEGRATED_BUTTONS|RESIZE"
            config.integrated_title_button_style = "Windows"
            config.integrated_title_button_alignment = "Left"
            config.integrated_title_buttons = { "Hide", "Maximize", "Close" }
            config.tab_bar_style = {
              window_hide = wezterm.format({ { Text = ' h ' } }),
              window_maximize = wezterm.format({ { Text = ' m ' } }),
              window_close = wezterm.format({ { Text = ' c ' } }),
            }

            return config
            "##,
        )
        .expect("expected WezTerm integrated title button style config");
        app.set_config_overrides(overrides);

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);

        assert!(
            tab_bar.starts_with(" h  m  c  ws:default"),
            "tab bar was {tab_bar:?}"
        );
    }

    #[test]
    fn window_app_applies_window_frame_button_colors_to_integrated_title_buttons() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            window_decorations: Some(NativeWindowDecorations {
                title: false,
                resize: true,
                integrated_buttons: true,
                macos_force_disable_shadow: false,
                macos_force_enable_shadow: false,
                macos_force_square_corners: false,
                macos_use_background_color_as_titlebar_color: false,
            }),
            integrated_title_buttons: Some(vec![NativeIntegratedTitleButton::Close]),
            integrated_title_button_style: Some(NativeIntegratedTitleButtonStyle::Windows),
            window_frame_appearance: Some(NativeWindowFrameAppearance {
                button_fg: Some(Color::Rgb(23, 24, 25)),
                button_bg: Some(Color::Rgb(45, 46, 47)),
                button_hover_fg: Some(Color::Rgb(67, 68, 69)),
                button_hover_bg: Some(Color::Rgb(89, 90, 91)),
                ..NativeWindowFrameAppearance::default()
            }),
            ..NativeConfigSnapshot::default()
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let close_column = tab_bar
            .chars()
            .position(|character| character == '×')
            .expect("close button should render");
        let close_cell = snapshot_cell(&snapshot, 0, u16::try_from(close_column).unwrap_or(0))
            .expect("expected close button cell");

        assert_eq!(close_cell.foreground, Color::Rgb(23, 24, 25));
        assert_eq!(close_cell.background, Color::Rgb(45, 46, 47));

        let x = u32::try_from(close_column).unwrap_or(0) * CELL_WIDTH;
        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        let hovered_snapshot = app.render_snapshot();
        let hovered_cell = snapshot_cell(
            &hovered_snapshot,
            0,
            u16::try_from(close_column).unwrap_or(0),
        )
        .expect("expected hovered close button cell");

        assert_eq!(hovered_cell.foreground, Color::Rgb(67, 68, 69));
        assert_eq!(hovered_cell.background, Color::Rgb(89, 90, 91));
    }

    #[test]
    fn window_app_applies_window_frame_titlebar_and_borders_to_render_snapshot() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.runtime.resize(rterm_core::TerminalSize::new(24, 4));
        app.set_config_overrides(native_config_snapshot! {
            hide_tab_bar_if_only_one_tab: Some(false),
            show_tabs_in_tab_bar: Some(false),
            window_frame_appearance: Some(NativeWindowFrameAppearance {
                active_titlebar_bg: Some(Color::Rgb(10, 20, 30)),
                inactive_titlebar_bg: Some(Color::Rgb(40, 50, 60)),
                active_titlebar_fg: Some(Color::Rgb(70, 80, 90)),
                inactive_titlebar_fg: Some(Color::Rgb(100, 110, 120)),
                inactive_titlebar_border_bottom: Some(Color::Rgb(30, 30, 30)),
                active_titlebar_border_bottom: Some(Color::Rgb(31, 32, 33)),
                border_left_width: Some(NativeWindowPaddingDimension::CellFractionPerMille(1000)),
                border_right_width: Some(NativeWindowPaddingDimension::CellFractionPerMille(1000)),
                border_bottom_height: Some(NativeWindowPaddingDimension::CellFractionPerMille(
                    1000,
                )),
                border_left_color: Some(Color::Rgb(11, 21, 31)),
                border_right_color: Some(Color::Rgb(12, 22, 32)),
                ..NativeWindowFrameAppearance::default()
            }),
            ..NativeConfigSnapshot::default()
        });

        assert!(!app.handle_focus_changed(true).unwrap());
        let active_snapshot = app.render_snapshot();
        let title_bar_sample_column = app
            .runtime
            .terminal()
            .grid()
            .size()
            .columns
            .saturating_sub(1);
        let active_tab_label = snapshot_row_text(&active_snapshot, 0, TERMINAL_COLUMNS)
            .find("ws:")
            .expect("expected tab label");
        let active_title_bar_cell = snapshot_cell(
            &active_snapshot,
            0,
            u16::try_from(active_tab_label).unwrap_or(0),
        )
        .expect("expected active title cell");
        assert_eq!(active_title_bar_cell.foreground, Color::Rgb(70, 80, 90));
        assert_eq!(
            snapshot_cell(&active_snapshot, 0, title_bar_sample_column,)
                .expect("expected theme title bar sample cell")
                .background,
            Color::Rgb(10, 20, 30)
        );
        assert_eq!(
            snapshot_cell(&active_snapshot, 1, 0)
                .expect("expected left border cell")
                .background,
            Color::Rgb(11, 21, 31)
        );
        assert_eq!(
            snapshot_cell(&active_snapshot, 1, 23)
                .expect("expected right border cell")
                .background,
            Color::Rgb(12, 22, 32)
        );
        let active_bottom_row = app
            .runtime
            .terminal()
            .grid()
            .size()
            .rows
            .saturating_add(app.terminal_frame_row_offset())
            .saturating_sub(1);
        assert_eq!(
            snapshot_cell(&active_snapshot, active_bottom_row, 1)
                .expect("expected bottom border cell")
                .background,
            Color::Rgb(31, 32, 33)
        );

        assert!(app.handle_focus_changed(false).unwrap());
        let inactive_snapshot = app.render_snapshot();
        let inactive_tab_label = snapshot_row_text(&inactive_snapshot, 0, TERMINAL_COLUMNS)
            .find("ws:")
            .expect("expected tab label");
        let inactive_title_bar_cell = snapshot_cell(
            &inactive_snapshot,
            0,
            u16::try_from(inactive_tab_label).unwrap_or(0),
        )
        .expect("expected inactive title cell");
        assert_eq!(
            inactive_title_bar_cell.foreground,
            Color::Rgb(100, 110, 120)
        );
        assert_eq!(
            snapshot_cell(&inactive_snapshot, 0, title_bar_sample_column,)
                .expect("expected theme title bar sample cell")
                .background,
            Color::Rgb(40, 50, 60)
        );
        assert_eq!(
            snapshot_cell(&inactive_snapshot, active_bottom_row, 1)
                .expect("expected bottom border cell")
                .background,
            Color::Rgb(30, 30, 30)
        );
    }

    #[test]
    fn window_app_integrated_title_button_clicks_dispatch_window_actions() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.set_config_overrides(native_config_snapshot! {
            window_decorations: Some(NativeWindowDecorations {
                title: false,
                resize: true,
                integrated_buttons: true,
                macos_force_disable_shadow: false,
                macos_force_enable_shadow: false,
                macos_force_square_corners: false,
                macos_use_background_color_as_titlebar_color: false,
            }),
            integrated_title_buttons: Some(vec![
                NativeIntegratedTitleButton::Hide,
                NativeIntegratedTitleButton::Maximize,
                NativeIntegratedTitleButton::Close,
            ]),
            integrated_title_button_alignment: Some(NativeIntegratedTitleButtonAlignment::Left),
            integrated_title_button_style: Some(NativeIntegratedTitleButtonStyle::Windows),
            window_close_confirmation: Some(NativeWindowCloseConfirmation::NeverPrompt),
            ..NativeConfigSnapshot::default()
        });

        let snapshot = app.render_snapshot();
        let tab_bar = snapshot_row_text(&snapshot, 0, TERMINAL_COLUMNS);
        let hide_column = tab_bar
            .chars()
            .position(|character| character == '—')
            .expect("hide button should render");
        let maximize_column = tab_bar
            .chars()
            .position(|character| character == '□')
            .expect("maximize button should render");
        let close_column = tab_bar
            .chars()
            .position(|character| character == '×')
            .expect("close button should render");

        let x = u32::try_from(hide_column).unwrap_or(0) * CELL_WIDTH;
        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert!(app.window_hide_requested_for_test());

        let x = u32::try_from(maximize_column).unwrap_or(0) * CELL_WIDTH;
        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert!(app.window_maximized_for_test());

        let x = u32::try_from(close_column).unwrap_or(0) * CELL_WIDTH;
        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );
        assert!(app.window_close_requested_for_test());
    }

    #[test]
    fn window_app_new_tab_button_click_handler_can_suppress_default_action() {
        let clicks = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&clicks);
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.new_tab_button_click_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            false
        });

        let tab_width = tab_bar_tab_label(
            0,
            rterm_core::TabId::new(1),
            1,
            true,
            None,
            rterm_core::app_shell::PaneProgress::None,
        )
        .chars()
        .count();
        let new_tab_column = app.tab_bar_workspace_label().chars().count() + tab_width + 1;
        let x = u32::try_from(new_tab_column).unwrap_or(0) * CELL_WIDTH;

        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(
            clicks.lock().unwrap().as_slice(),
            [NativeWindowNewTabButtonClick {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                button: MouseButton::Left,
                default_action: Some(WindowCommand::NewTab),
            }]
        );
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_new_tab_button_click_handler_receives_right_click_with_launcher_default_action() {
        let clicks = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&clicks);
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.new_tab_button_click_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        let tab_width = tab_bar_tab_label(
            0,
            rterm_core::TabId::new(1),
            1,
            true,
            None,
            rterm_core::app_shell::PaneProgress::None,
        )
        .chars()
        .count();
        let new_tab_column = app.tab_bar_workspace_label().chars().count() + tab_width + 1;
        let x = u32::try_from(new_tab_column).unwrap_or(0) * CELL_WIDTH;

        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Right)
                .unwrap()
        );

        assert_eq!(
            clicks.lock().unwrap().as_slice(),
            [NativeWindowNewTabButtonClick {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                button: MouseButton::Right,
                default_action: Some(WindowCommand::NewTab),
            }]
        );
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
        let palette = app
            .command_palette
            .as_ref()
            .expect("right-clicking the new-tab button should open launcher");
        assert_eq!(palette.title(), "Launcher");
    }

    #[test]
    fn window_app_parses_static_wezterm_new_tab_button_click_false_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('new-tab-button-click', function(window, pane, button, default_action)
              return false
            end)
            "#,
        )
        .expect("expected static WezTerm new-tab-button-click false return");
        app.set_config_overrides(overrides);

        let tab_width = tab_bar_tab_label(
            0,
            rterm_core::TabId::new(1),
            1,
            true,
            None,
            rterm_core::app_shell::PaneProgress::None,
        )
        .chars()
        .count();
        let new_tab_column = app.tab_bar_workspace_label().chars().count() + tab_width + 1;
        let x = u32::try_from(new_tab_column).unwrap_or(0) * CELL_WIDTH;

        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
    }

    #[test]
    fn window_app_parses_static_wezterm_new_tab_button_click_false_return_without_params() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('new-tab-button-click', function()
              return false
            end)
            "#,
        )
        .expect("expected static WezTerm new-tab-button-click false return without params");
        app.set_config_overrides(overrides);

        let tab_width = tab_bar_tab_label(
            0,
            rterm_core::TabId::new(1),
            1,
            true,
            None,
            rterm_core::app_shell::PaneProgress::None,
        )
        .chars()
        .count();
        let new_tab_column = app.tab_bar_workspace_label().chars().count() + tab_width + 1;
        let x = u32::try_from(new_tab_column).unwrap_or(0) * CELL_WIDTH;

        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
    }

    #[test]
    fn window_app_parses_static_wezterm_new_tab_button_click_left_button_condition() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('new-tab-button-click', function(window, pane, button, default_action)
              if button == 'Left' then
                return false
              else
                return true
              end
            end)
            "#,
        )
        .expect("expected static WezTerm new-tab-button-click left-button condition");
        app.set_config_overrides(overrides);

        let tab_width = tab_bar_tab_label(
            0,
            rterm_core::TabId::new(1),
            1,
            true,
            None,
            rterm_core::app_shell::PaneProgress::None,
        )
        .chars()
        .count();
        let new_tab_column = app.tab_bar_workspace_label().chars().count() + tab_width + 1;
        let x = u32::try_from(new_tab_column).unwrap_or(0) * CELL_WIDTH;

        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
    }

    #[test]
    fn window_app_parses_static_wezterm_new_tab_button_click_button_not_equal_condition() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('new-tab-button-click', function(window, pane, button, default_action)
              if button ~= 'Left' then
                return false
              else
                return true
              end
            end)
            "#,
        )
        .expect("expected static WezTerm new-tab-button-click button-not-equal condition");
        app.set_config_overrides(overrides);
        move_mouse_to_rendered_new_tab_button(&mut app);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
    }

    #[test]
    fn window_app_parses_static_wezterm_new_tab_button_click_elseif_button_condition() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('new-tab-button-click', function(window, pane, button, default_action)
              if button == 'Left' then
                return true
              elseif button == 'Right' then
                return false
              else
                return true
              end
            end)
            "#,
        )
        .expect("expected static WezTerm new-tab-button-click elseif button condition");
        app.set_config_overrides(overrides);

        let tab_width = tab_bar_tab_label(
            0,
            rterm_core::TabId::new(1),
            1,
            true,
            None,
            rterm_core::app_shell::PaneProgress::None,
        )
        .chars()
        .count();
        let new_tab_column = app.tab_bar_workspace_label().chars().count() + tab_width + 1;
        let x = u32::try_from(new_tab_column).unwrap_or(0) * CELL_WIDTH;

        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Right)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_parses_static_wezterm_new_tab_button_click_button_alias_condition() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('new-tab-button-click', function(window, pane, button, default_action)
              local clicked = button
              if clicked == 'Right' then
                return false
              else
                return true
              end
            end)
            "#,
        )
        .expect("expected static WezTerm new-tab-button-click button alias condition");
        app.set_config_overrides(overrides);

        let tab_width = tab_bar_tab_label(
            0,
            rterm_core::TabId::new(1),
            1,
            true,
            None,
            rterm_core::app_shell::PaneProgress::None,
        )
        .chars()
        .count();
        let new_tab_column = app.tab_bar_workspace_label().chars().count() + tab_width + 1;
        let x = u32::try_from(new_tab_column).unwrap_or(0) * CELL_WIDTH;

        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Right)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_parses_static_wezterm_new_tab_button_click_nested_button_alias_condition() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('new-tab-button-click', function(window, pane, button, default_action)
              local clicked = button
              local pressed = clicked
              if pressed == 'Right' then
                return false
              else
                return true
              end
            end)
            "#,
        )
        .expect("expected static WezTerm new-tab-button-click nested button alias condition");
        app.set_config_overrides(overrides);

        let tab_width = tab_bar_tab_label(
            0,
            rterm_core::TabId::new(1),
            1,
            true,
            None,
            rterm_core::app_shell::PaneProgress::None,
        )
        .chars()
        .count();
        let new_tab_column = app.tab_bar_workspace_label().chars().count() + tab_width + 1;
        let x = u32::try_from(new_tab_column).unwrap_or(0) * CELL_WIDTH;

        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Right)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_parses_static_wezterm_new_tab_button_click_button_string_variable_condition() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('new-tab-button-click', function(window, pane, button, default_action)
              local right_button = 'Right'
              if button == right_button then
                return false
              else
                return true
              end
            end)
            "#,
        )
        .expect("expected static WezTerm new-tab-button-click button string variable condition");
        app.set_config_overrides(overrides);

        let tab_width = tab_bar_tab_label(
            0,
            rterm_core::TabId::new(1),
            1,
            true,
            None,
            rterm_core::app_shell::PaneProgress::None,
        )
        .chars()
        .count();
        let new_tab_column = app.tab_bar_workspace_label().chars().count() + tab_width + 1;
        let x = u32::try_from(new_tab_column).unwrap_or(0) * CELL_WIDTH;

        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Right)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_parses_static_wezterm_new_tab_button_click_direct_button_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('new-tab-button-click', function(window, pane, button, default_action)
              return button ~= 'Right'
            end)
            "#,
        )
        .expect("expected static WezTerm new-tab-button-click direct button return");
        app.set_config_overrides(overrides);

        let tab_width = tab_bar_tab_label(
            0,
            rterm_core::TabId::new(1),
            1,
            true,
            None,
            rterm_core::app_shell::PaneProgress::None,
        )
        .chars()
        .count();
        let new_tab_column = app.tab_bar_workspace_label().chars().count() + tab_width + 1;
        let x = u32::try_from(new_tab_column).unwrap_or(0) * CELL_WIDTH;

        app.handle_cursor_moved(PhysicalPosition::new(f64::from(x), 0.0))
            .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Right)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(1));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 1);
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_parses_documented_new_tab_button_click_manual_default_action() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on(
              'new-tab-button-click',
              function(window, pane, button, default_action)
                wezterm.log_info('new-tab', window, pane, button, default_action)
                if default_action then
                  window:perform_action(default_action, pane)
                end
                return false
              end
            )
            "#,
        )
        .expect("expected documented static WezTerm new-tab-button-click default action");
        app.set_config_overrides(overrides);
        move_mouse_to_rendered_new_tab_button(&mut app);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
    }

    #[test]
    fn window_app_parses_new_tab_button_click_non_nil_default_action_guard() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('new-tab-button-click', function(window, pane, button, default_action)
              if default_action ~= nil then
                window:perform_action(default_action, pane)
              end
              return false
            end)
            "#,
        )
        .expect("expected static WezTerm new-tab-button-click non-nil default action guard");
        app.set_config_overrides(overrides);
        move_mouse_to_rendered_new_tab_button(&mut app);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
    }

    #[test]
    fn window_app_parses_new_tab_button_click_nil_guard_before_default_action() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('new-tab-button-click', function(window, pane, button, default_action)
              if default_action == nil then
                return false
              end
              window:perform_action(default_action, pane)
              return false
            end)
            "#,
        )
        .expect("expected static WezTerm new-tab-button-click nil guard before default action");
        app.set_config_overrides(overrides);
        move_mouse_to_rendered_new_tab_button(&mut app);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
    }

    #[test]
    fn window_app_parses_new_tab_button_click_reversed_non_nil_default_action_guard() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('new-tab-button-click', function(window, pane, button, default_action)
              if nil ~= default_action then
                window:perform_action(default_action, pane)
              end
              return false
            end)
            "#,
        )
        .expect(
            "expected static WezTerm new-tab-button-click reversed non-nil default action guard",
        );
        app.set_config_overrides(overrides);
        move_mouse_to_rendered_new_tab_button(&mut app);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
    }

    #[test]
    fn window_app_parses_new_tab_button_click_default_action_alias() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('new-tab-button-click', function(window, pane, button, default_action)
              local action = default_action
              if action ~= nil then
                window:perform_action(action, pane)
              end
              return false
            end)
            "#,
        )
        .expect("expected static WezTerm new-tab-button-click default action alias");
        app.set_config_overrides(overrides);
        move_mouse_to_rendered_new_tab_button(&mut app);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
    }

    #[test]
    fn window_app_parses_new_tab_button_click_pane_alias_default_action() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('new-tab-button-click', function(window, pane, button, default_action)
              local target = pane
              if default_action then
                window:perform_action(default_action, target)
              end
              return false
            end)
            "#,
        )
        .expect("expected static WezTerm new-tab-button-click pane alias default action");
        app.set_config_overrides(overrides);
        move_mouse_to_rendered_new_tab_button(&mut app);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
    }

    #[test]
    fn window_app_parses_new_tab_button_click_window_alias_default_action() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('new-tab-button-click', function(window, pane, button, default_action)
              local win = window
              if default_action then
                win:perform_action(default_action, pane)
              end
              return false
            end)
            "#,
        )
        .expect("expected static WezTerm new-tab-button-click window alias default action");
        app.set_config_overrides(overrides);
        move_mouse_to_rendered_new_tab_button(&mut app);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
    }

    #[test]
    fn window_app_parses_new_tab_button_click_dot_perform_action_default_action() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            wezterm.on('new-tab-button-click', function(window, pane, button, default_action)
              if default_action then
                window.perform_action(window, default_action, pane)
              end
              return false
            end)
            "#,
        )
        .expect("expected static WezTerm new-tab-button-click dot perform_action default action");
        app.set_config_overrides(overrides);
        move_mouse_to_rendered_new_tab_button(&mut app);
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(app.app_shell.active_workspace().tabs().len(), 2);
    }

    #[test]
    fn window_app_renders_right_split_panes_with_separator() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"left").unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"right").unwrap();

        let snapshot = app.render_snapshot();

        assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS, 0), Some('l'));
        assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS, 39), Some('|'));
        assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS, 40), Some('r'));
    }

    #[test]
    fn window_app_uses_modern_default_split_separator_surface() {
        let mut app = NativeWindowApp::new(None);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        let snapshot = app.render_snapshot();
        let separator = snapshot_cell(&snapshot, TAB_BAR_ROWS, 39)
            .expect("expected default split separator");

        assert_eq!(separator.foreground, Color::Rgb(0x38, 0xbd, 0xf8));
        assert_eq!(separator.background, Color::Rgb(0x10, 0x18, 0x27));
    }

    #[test]
    fn pane_inspection_uses_modern_surface_defaults() {
        let cells = super::pane_inspection_cells_for_rect(
            &["inspect".to_owned()],
            super::PaneRenderRect {
                pane_id: rterm_core::PaneId::new(1),
                row: 2,
                column: 3,
                rows: 1,
                columns: 8,
            },
        );
        let first = cells.first().expect("expected pane inspection cell");

        assert_eq!(first.foreground, Color::Rgb(0xd8, 0xe2, 0xf0));
        assert_eq!(first.background, Color::Rgb(0x1b, 0x2b, 0x44));
    }

    #[test]
    fn window_app_applies_wezterm_split_color_to_separator() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              split = '#010203',
            }

            return config
            "##,
        )
        .expect("expected WezTerm colors.split config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"left").unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"right").unwrap();

        let snapshot = app.render_snapshot();
        let separator = snapshot_cell(&snapshot, TAB_BAR_ROWS, 39).expect("split separator");

        assert_eq!(separator.ch, '|');
        assert_eq!(separator.foreground, Color::Rgb(1, 2, 3));
        assert_eq!(separator.background, Color::Rgb(0x10, 0x18, 0x27));
    }

    #[test]
    fn window_app_renders_down_split_panes_with_separator() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"top").unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Down,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"bottom").unwrap();

        let snapshot = app.render_snapshot();

        assert_eq!(snapshot_char(&snapshot, TAB_BAR_ROWS, 0), Some('t'));
        assert_eq!(snapshot_char(&snapshot, 12, 0), Some('-'));
        assert_eq!(snapshot_char(&snapshot, 13, 0), Some('b'));
    }

    #[test]
    fn window_app_applies_underline_thickness_to_down_split_separator() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            underline_thickness: Some(NativeUnderlineThickness::Pixels(3)),
            split_color: Some(Color::Rgb(1, 2, 3)),
            ..NativeConfigSnapshot::default()
        });
        app.handle_pty_output(b"top").unwrap();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Down,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"bottom").unwrap();

        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];
        app.render_framebuffer(&mut frame);
        let split_rows = (0..FRAME_HEIGHT as usize)
            .filter(|row| frame_pixel_at(&frame, FRAME_WIDTH as usize, 0, *row) == [1, 2, 3, 255])
            .collect::<Vec<_>>();
        assert!(split_rows.len() >= 3);
        assert!(split_rows
            .windows(3)
            .any(|rows| rows[1] == rows[0] + 1 && rows[2] == rows[1] + 1));
    }

    #[test]
    fn window_app_clicking_split_pane_focuses_that_pane() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
    }

    fn app_with_two_selected_panes_for_test() -> NativeWindowApp {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(12, 2));
        app.handle_pty_output(b"left").unwrap();
        set_ordinary_viewport_selection_for_test(
            &mut app,
            WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 3 },
            ),
        );
        app.refresh_snapshot();

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert!(
            app.pane_runtimes
                .get(&rterm_core::PaneId::new(1))
                .is_some_and(|runtime| runtime.ui.ordinary_selection.is_some())
        );

        app.handle_pty_output(b"right").unwrap();
        set_ordinary_viewport_selection_for_test(
            &mut app,
            WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 4 },
            ),
        );
        app.refresh_snapshot();
        assert_eq!(app.selected_text().as_deref(), Some("right"));
        app
    }

    #[test]
    fn window_app_clear_selection_only_clears_active_pane_selection() {
        let mut app = app_with_two_selected_panes_for_test();

        app.command_palette_apply_command(WindowCommand::ClearSelection)
            .unwrap();

        assert!(app.selection.is_none());
        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rterm_core::PaneId::new(1),
        })
        .unwrap();
        assert_eq!(app.selected_text().as_deref(), Some("left"));
    }

    #[test]
    fn window_app_copy_selection_only_copies_active_pane_selection() {
        let clipboard = Arc::new(Mutex::new(Vec::new()));
        let recorded_clipboard = Arc::clone(&clipboard);
        let mut app = app_with_two_selected_panes_for_test();
        app.clipboard_writer = Box::new(move |text: &str| {
            recorded_clipboard.lock().unwrap().push(text.to_owned());
            true
        });

        app.command_palette_apply_command(WindowCommand::CopyToClipboard)
            .unwrap();
        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rterm_core::PaneId::new(1),
        })
        .unwrap();
        app.command_palette_apply_command(WindowCommand::CopyToClipboard)
            .unwrap();

        assert_eq!(clipboard.lock().unwrap().as_slice(), ["right", "left"]);
    }

    #[test]
    fn window_app_closing_active_selected_pane_restores_surviving_selection() {
        let mut app = app_with_two_selected_panes_for_test();

        app.dispatch_app_action(AppAction::ClosePane {
            pane: rterm_core::PaneId::new(2),
        })
        .unwrap();

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
        assert_eq!(app.selected_text().as_deref(), Some("left"));
        assert!(!app.pane_runtimes.contains_key(&rterm_core::PaneId::new(2)));
    }

    #[test]
    fn window_app_closing_inactive_selected_pane_keeps_active_selection() {
        let mut app = app_with_two_selected_panes_for_test();

        app.dispatch_app_action(AppAction::ClosePane {
            pane: rterm_core::PaneId::new(1),
        })
        .unwrap();

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(app.selected_text().as_deref(), Some("right"));
        assert!(!app.pane_runtimes.contains_key(&rterm_core::PaneId::new(1)));
    }

    #[test]
    fn window_app_new_split_pane_starts_without_source_selection() {
        let mut app = app_with_two_selected_panes_for_test();

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(2),
            direction: rterm_core::app_shell::SplitDirection::Down,
            launch: None,
        })
        .unwrap();

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(3));
        assert!(app.selection.is_none());
        assert!(
            app.pane_runtimes
                .get(&rterm_core::PaneId::new(2))
                .is_some_and(|runtime| runtime.ui.ordinary_selection.is_some())
        );
    }

    #[test]
    fn window_app_move_selected_pane_to_new_tab_preserves_selection() {
        let mut app = app_with_two_selected_panes_for_test();

        app.dispatch_app_action(AppAction::MovePaneToNewTab {
            pane: rterm_core::PaneId::new(2),
        })
        .unwrap();

        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert_eq!(app.selected_text().as_deref(), Some("right"));
    }

    #[test]
    fn window_app_move_selected_pane_to_new_window_drops_gui_selection_only() {
        let mut app = app_with_two_selected_panes_for_test();
        app.runtime.resize(rterm_core::TerminalSize::new(12, 2));
        app.handle_pty_output(b"\rone\r\ntwo\r\nthree\r\nfour")
            .unwrap();
        app.scroll_viewport_lines(1);
        assert_eq!(app.current_scrollback_offset(), 1);
        assert!(!app.runtime.terminal().scrollback().is_empty());
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 2 },
        );
        app.refresh_snapshot();

        app.dispatch_app_action(AppAction::MovePaneToNewWindow {
            pane: rterm_core::PaneId::new(2),
        })
        .unwrap();
        assert!(
            app.pane_runtimes
                .get(&rterm_core::PaneId::new(2))
                .is_some_and(|runtime| runtime.ui.ordinary_selection.is_some())
        );
        let detached = app
            .take_next_pending_window_app()
            .expect("pane should materialize as a detached window");

        assert!(detached.selection.is_none());
        assert_eq!(detached.current_scrollback_offset(), 1);
        assert!(!detached.runtime.terminal().scrollback().is_empty());
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
        assert_eq!(app.selected_text().as_deref(), Some("left"));
    }

    #[test]
    fn window_app_restores_independent_selection_for_each_pane() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"left").unwrap();
        set_ordinary_viewport_selection_for_test(
            &mut app,
            WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 3 },
            ),
        );
        app.refresh_snapshot();

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(2));
        assert!(app.selection.is_none());

        app.handle_pty_output(b"right").unwrap();
        set_ordinary_viewport_selection_for_test(
            &mut app,
            WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 4 },
            ),
        );
        app.refresh_snapshot();

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
    fn window_app_switching_panes_ends_drag_but_preserves_source_selection() {
        let mut app = NativeWindowApp::new(None);
        let selection = WindowSelection::new(
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 1 },
        );
        set_ordinary_viewport_selection_for_test(&mut app, selection);
        app.selecting = true;

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        assert!(!app.selecting);
        assert!(
            app.pane_runtimes
                .get(&rterm_core::PaneId::new(1))
                .is_some_and(|runtime| runtime.ui.ordinary_selection.is_some())
        );
    }

    #[test]
    fn window_app_pane_switch_does_not_persist_copy_mode_selection_as_ordinary_selection() {
        let mut app = NativeWindowApp::new(None);
        app.enter_copy_mode();
        app.selection = Some(WindowSelection::new(
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 1 },
        ));

        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: rterm_core::app_shell::SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        assert!(copy_mode_for_test(&app).is_none());
        assert!(search_for_test(&app).is_none());
        assert!(quick_select_for_test(&app).is_none());
        assert!(
            app.pane_runtimes
                .get(&rterm_core::PaneId::new(1))
                .and_then(|runtime| runtime.ui.ordinary_selection)
                .is_none()
        );
    }

    #[derive(Debug, Clone, Copy)]
    enum PaneOverlayLifecycleClass {
        Search,
        Copy,
        Quick,
    }

    fn pane_overlay_lifecycle_copy_selection_mode(tag: &str) -> super::WindowCopySelectionMode {
        if tag.ends_with("-a") {
            super::WindowCopySelectionMode::Word
        } else {
            super::WindowCopySelectionMode::Line
        }
    }

    fn install_distinct_pane_overlay_for_lifecycle_test(
        app: &mut NativeWindowApp,
        class: PaneOverlayLifecycleClass,
        tag: &str,
        row: u16,
        column: u16,
    ) -> String {
        let source_row = app.current_viewport_stable_top().saturating_add(
            StableRowIndex::try_from(row)
                .expect("u16 row fits StableRowIndex on supported targets"),
        );
        let search_match = WindowSearchMatch {
            domain: TerminalScreenDomain::Main,
            source_row,
            start_column: column,
            end_source_row: source_row,
            end_column: column.saturating_add(2),
        };
        let mut copy_mode =
            pane_overlay_copy_mode(row, column, pane_overlay_lifecycle_copy_selection_mode(tag));
        copy_mode.source_cursor.row = source_row;
        copy_mode.source_anchor = copy_mode.source_anchor.map(|anchor| SelectionSourceCell {
            row: source_row.saturating_add(1),
            ..anchor
        });
        match class {
            PaneOverlayLifecycleClass::Search => {
                app.active_ui.enter_search(
                    copy_mode,
                    pane_overlay_search(
                        tag,
                        WindowSearchMatchType::CaseInsensitive,
                        Some(search_match),
                        false,
                    ),
                );
                assert!(app.active_ui.set_search_current(Some(search_match)));
            }
            PaneOverlayLifecycleClass::Copy => {
                app.active_ui.enter_search(
                    copy_mode,
                    pane_overlay_search(
                        tag,
                        WindowSearchMatchType::Regex,
                        Some(search_match),
                        false,
                    ),
                );
                assert!(app.active_ui.set_search_current(Some(search_match)));
                let initial_copy_mode = app.initial_copy_mode();
                app.active_ui.enter_copy_mode(initial_copy_mode);
            }
            PaneOverlayLifecycleClass::Quick => {
                let mut first_match = pane_overlay_match(column);
                first_match.source_row = source_row;
                first_match.end_source_row = source_row;
                let mut second_match = first_match;
                second_match.start_column = column.saturating_add(3);
                second_match.end_column = column.saturating_add(5);
                app.active_ui.enter_quick_select(WindowQuickSelect {
                    current: 1,
                    matches: vec![first_match, second_match],
                    labels: vec![format!("{tag}-a"), format!("{tag}-b")],
                    input: tag.to_owned(),
                    reflow_config: None,
                    action_label: Some(format!("action-{tag}")),
                    action: WindowQuickSelectAction::SendString(format!("send-{tag}")),
                    skip_action_on_paste: row % 2 == 0,
                });
            }
        }
        app.apply_window_title();
        let effective_title = app.effective_window_title();
        match class {
            PaneOverlayLifecycleClass::Search => {
                assert!(effective_title.contains(&format!("Search: {tag}")));
            }
            PaneOverlayLifecycleClass::Copy => {
                let status = match pane_overlay_lifecycle_copy_selection_mode(tag) {
                    super::WindowCopySelectionMode::Word => "Copy Mode: Word",
                    super::WindowCopySelectionMode::Line => "Copy Mode: Line",
                    _ => unreachable!("lifecycle fixture only uses word and line copy modes"),
                };
                assert!(effective_title.contains(status));
            }
            PaneOverlayLifecycleClass::Quick => {
                assert!(effective_title.contains(&format!("Quick Select action-{tag}: \"{tag}\"")));
            }
        }
        effective_title
    }

    fn assert_distinct_pane_overlay_for_lifecycle_test(
        app: &NativeWindowApp,
        class: PaneOverlayLifecycleClass,
        tag: &str,
        row: u16,
        column: u16,
        expected_title: &str,
    ) {
        let source_row = app.current_viewport_stable_top().saturating_add(
            StableRowIndex::try_from(row)
                .expect("u16 row fits StableRowIndex on supported targets"),
        );
        assert_eq!(
            app.effective_window_title(),
            expected_title,
            "{class:?} owner title"
        );
        match class {
            PaneOverlayLifecycleClass::Search => {
                assert_eq!(
                    copy_search_mode_for_test(app),
                    Some(super::WindowCopySearchMode::Search)
                );
                let search = search_for_test(app).expect("search overlay");
                assert_eq!(search.query, tag);
                assert_eq!(search.match_type, WindowSearchMatchType::CaseInsensitive);
                assert_eq!(
                    search.current,
                    Some(WindowSearchMatch {
                        domain: TerminalScreenDomain::Main,
                        source_row,
                        start_column: column,
                        end_source_row: source_row,
                        end_column: column.saturating_add(2),
                    })
                );
                assert!(search.editing);
            }
            PaneOverlayLifecycleClass::Copy => {
                assert_eq!(
                    copy_search_mode_for_test(app),
                    Some(super::WindowCopySearchMode::Copy)
                );
                let search = search_for_test(app).expect("retained copy search");
                assert_eq!(search.query, tag);
                assert_eq!(search.match_type, WindowSearchMatchType::Regex);
                assert_eq!(
                    search.current,
                    Some(WindowSearchMatch {
                        domain: TerminalScreenDomain::Main,
                        source_row,
                        start_column: column,
                        end_source_row: source_row,
                        end_column: column.saturating_add(2),
                    })
                );
                assert!(!search.editing);
                let copy = copy_mode_for_test(app).expect("copy overlay");
                assert_eq!(copy.cursor, SelectionCell { row, column });
                assert_eq!(copy.source_cursor.row, source_row);
                assert_eq!(copy.source_cursor.column, usize::from(column));
                assert_eq!(
                    copy.source_anchor.map(|anchor| anchor.row),
                    Some(source_row + 1)
                );
                assert_eq!(
                    copy.selection_mode,
                    pane_overlay_lifecycle_copy_selection_mode(tag)
                );
            }
            PaneOverlayLifecycleClass::Quick => {
                let quick = quick_select_for_test(app).expect("quick-select overlay");
                assert_eq!(quick.current, 1);
                assert_eq!(quick.input, tag);
                assert_eq!(quick.labels, [format!("{tag}-a"), format!("{tag}-b")]);
                assert_eq!(
                    quick.action_label.as_deref(),
                    Some(&*format!("action-{tag}"))
                );
                assert_eq!(
                    quick.action,
                    WindowQuickSelectAction::SendString(format!("send-{tag}"))
                );
                assert_eq!(quick.skip_action_on_paste, row % 2 == 0);
                assert_eq!(quick.matches.len(), 2);
                assert_eq!(
                    quick.matches[1],
                    WindowSearchMatch {
                        domain: TerminalScreenDomain::Main,
                        source_row,
                        start_column: column.saturating_add(3),
                        end_source_row: source_row,
                        end_column: column.saturating_add(5),
                    }
                );
            }
        }
    }

    fn assert_pane_overlay_class_for_lifecycle_test(
        app: &NativeWindowApp,
        class: PaneOverlayLifecycleClass,
    ) {
        match class {
            PaneOverlayLifecycleClass::Search => {
                assert_eq!(
                    copy_search_mode_for_test(app),
                    Some(super::WindowCopySearchMode::Search)
                );
                assert!(search_for_test(app).is_some());
            }
            PaneOverlayLifecycleClass::Copy => {
                assert_eq!(
                    copy_search_mode_for_test(app),
                    Some(super::WindowCopySearchMode::Copy)
                );
                assert!(copy_mode_for_test(app).is_some());
            }
            PaneOverlayLifecycleClass::Quick => {
                assert!(quick_select_for_test(app).is_some());
            }
        }
    }

    fn assert_pane_overlay_tag_for_lifecycle_test(
        app: &NativeWindowApp,
        class: PaneOverlayLifecycleClass,
        tag: &str,
    ) {
        assert_pane_overlay_class_for_lifecycle_test(app, class);
        match class {
            PaneOverlayLifecycleClass::Search | PaneOverlayLifecycleClass::Copy => {
                assert_eq!(
                    search_for_test(app).map(|search| search.query.as_str()),
                    Some(tag)
                );
            }
            PaneOverlayLifecycleClass::Quick => {
                assert_eq!(
                    quick_select_for_test(app).map(|quick| quick.input.as_str()),
                    Some(tag)
                );
            }
        }
    }

    fn install_line_copy_overlay_for_lifecycle_test(app: &mut NativeWindowApp) {
        app.enter_copy_mode();
        assert!(app.set_copy_mode_selection_mode(super::WindowCopySelectionMode::Line));
        assert!(app.selected_text().is_some());
    }

    #[test]
    fn window_app_copy_mode_focus_fallback_preserves_source_overlay() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"source").unwrap();
        install_line_copy_overlay_for_lifecycle_test(&mut app);

        assert!(app.handle_copy_mode_key(
            &Key::Character("t".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));

        app.dispatch_app_action(AppAction::ActivateTab {
            tab: rterm_core::TabId::new(1),
        })
        .unwrap();
        assert_eq!(
            copy_search_mode_for_test(&app),
            Some(super::WindowCopySearchMode::Copy)
        );
        assert_eq!(app.selected_text().as_deref(), Some("source"));
    }

    #[test]
    fn window_app_click_focus_does_not_clear_source_or_target_overlay() {
        let mut app = NativeWindowApp::new(None);
        app.window_focused = true;
        install_distinct_pane_overlay_for_lifecycle_test(
            &mut app,
            PaneOverlayLifecycleClass::Search,
            "click-target",
            0,
            0,
        );
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        install_distinct_pane_overlay_for_lifecycle_test(
            &mut app,
            PaneOverlayLifecycleClass::Quick,
            "click-source",
            0,
            0,
        );

        app.handle_cursor_moved(PhysicalPosition::new(
            f64::from(CELL_WIDTH),
            f64::from(tab_bar_pixel_height()),
        ))
        .unwrap();
        assert!(
            app.handle_mouse_input(ElementState::Pressed, MouseButton::Left)
                .unwrap()
        );

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
        assert_eq!(
            search_for_test(&app).map(|search| search.query.as_str()),
            Some("click-target")
        );
        assert!(
            app.pane_runtimes
                .get(&rterm_core::PaneId::new(2))
                .and_then(|runtime| runtime.ui.quick_select())
                .is_some_and(|quick| quick.input == "click-source")
        );
    }

    #[test]
    fn window_app_active_input_mutates_only_active_overlay() {
        let mut app = NativeWindowApp::new(None);
        install_distinct_pane_overlay_for_lifecycle_test(
            &mut app,
            PaneOverlayLifecycleClass::Search,
            "inactive",
            0,
            0,
        );
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        install_distinct_pane_overlay_for_lifecycle_test(
            &mut app,
            PaneOverlayLifecycleClass::Search,
            "active",
            0,
            0,
        );

        assert!(app.handle_search_key(&Key::Character("x".into()), ModifiersState::empty()));
        assert_eq!(
            search_for_test(&app).map(|search| search.query.as_str()),
            Some("activex")
        );
        assert_eq!(
            app.pane_runtimes
                .get(&rterm_core::PaneId::new(1))
                .and_then(|runtime| runtime.ui.search())
                .map(|search| search.query.as_str()),
            Some("inactive")
        );
    }

    #[test]
    fn window_app_copy_and_selection_actions_read_only_active_pane_overlay() {
        let clipboard = Arc::new(Mutex::new(Vec::new()));
        let recorded_clipboard = Arc::clone(&clipboard);
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(8, 1));
        app.handle_pty_output(b"left").unwrap();
        install_line_copy_overlay_for_lifecycle_test(&mut app);
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.runtime.resize(rterm_core::TerminalSize::new(8, 1));
        app.handle_pty_output(b"right").unwrap();
        install_line_copy_overlay_for_lifecycle_test(&mut app);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.clipboard_writer = Box::new(move |text| {
            recorded_clipboard.lock().unwrap().push(text.to_owned());
            true
        });

        app.command_palette_apply_command(WindowCommand::CopyToClipboard)
            .unwrap();
        set_app_quick_select_for_test(
            &mut app,
            WindowQuickSelect {
                matches: vec![WindowSearchMatch {
                    end_column: 5,
                    ..pane_overlay_match(0)
                }],
                labels: vec!["a".to_owned()],
                action: WindowQuickSelectAction::SendSelectedText,
                ..WindowQuickSelect::default()
            },
        );
        app.accept_quick_select_match(false);
        set_app_quick_select_for_test(
            &mut app,
            WindowQuickSelect {
                matches: vec![WindowSearchMatch {
                    end_column: 5,
                    ..pane_overlay_match(0)
                }],
                labels: vec!["a".to_owned()],
                action: WindowQuickSelectAction::PasteSelectedText,
                ..WindowQuickSelect::default()
            },
        );
        app.accept_quick_select_match(false);
        app.command_palette_apply_command(WindowCommand::ClearSelection)
            .unwrap();

        assert_eq!(clipboard.lock().unwrap().as_slice(), ["right"]);
        assert_eq!(written.lock().unwrap().as_slice(), b"rightright");
        assert!(app.selected_text().is_none());
        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rterm_core::PaneId::new(1),
        })
        .unwrap();
        assert_eq!(app.selected_text().as_deref(), Some("left"));
    }

    #[test]
    fn window_app_quick_nested_focus_action_clears_only_source_owner() {
        let mut app = NativeWindowApp::new(None);
        install_distinct_pane_overlay_for_lifecycle_test(
            &mut app,
            PaneOverlayLifecycleClass::Search,
            "nested-target",
            0,
            0,
        );
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.handle_pty_output(b"source").unwrap();
        set_app_quick_select_for_test(
            &mut app,
            WindowQuickSelect {
                matches: vec![pane_overlay_match(0)],
                labels: vec!["a".to_owned()],
                action: WindowQuickSelectAction::Multiple(vec![
                    WindowCommand::ActivatePane1,
                    WindowCommand::Nop,
                ]),
                ..WindowQuickSelect::default()
            },
        );
        app.update_selection_projection();

        assert!(
            app.handle_quick_select_logical_key(
                &Key::Character("a".into()),
                ModifiersState::empty()
            )
        );
        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
        assert_eq!(
            search_for_test(&app).map(|search| search.query.as_str()),
            Some("nested-target")
        );
        assert!(
            app.pane_runtimes
                .get(&rterm_core::PaneId::new(2))
                .is_some_and(|runtime| !runtime.ui.overlay_active())
        );
    }

    #[test]
    fn window_app_quick_multiple_binds_pane_sensitive_actions_to_source_owner() {
        let source_written = Arc::new(Mutex::new(Vec::new()));
        let target_written = Arc::new(Mutex::new(Vec::new()));
        let clipboard = Arc::new(Mutex::new(Vec::new()));
        let recorded_clipboard = Arc::clone(&clipboard);
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"target").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 2 },
        );
        let target_ordinary = ordinary_selection_for_test(&app).expect("target ordinary");
        install_distinct_pane_overlay_for_lifecycle_test(
            &mut app,
            PaneOverlayLifecycleClass::Search,
            "target-search",
            0,
            0,
        );
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&target_written))));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();

        app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
        app.handle_pty_output(b"source").unwrap();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 1 },
            SelectionCell { row: 0, column: 3 },
        );
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&source_written))));
        app.clipboard_writer = Box::new(move |text| {
            recorded_clipboard.lock().unwrap().push(text.to_owned());
            true
        });
        set_app_quick_select_for_test(
            &mut app,
            WindowQuickSelect {
                matches: vec![WindowSearchMatch {
                    end_column: 6,
                    ..pane_overlay_match(0)
                }],
                labels: vec!["a".to_owned()],
                action: WindowQuickSelectAction::Multiple(vec![
                    WindowCommand::ActivatePane1,
                    WindowCommand::Multiple(vec![
                        WindowCommand::SendString("source-bound".to_owned()),
                        WindowCommand::Copy,
                    ]),
                    WindowCommand::ClearSelection,
                ]),
                ..WindowQuickSelect::default()
            },
        );
        app.update_selection_projection();

        app.accept_quick_select_match(false);

        assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
        assert_eq!(source_written.lock().unwrap().as_slice(), b"source-bound");
        assert!(target_written.lock().unwrap().is_empty());
        assert_eq!(clipboard.lock().unwrap().as_slice(), ["source"]);
        assert_eq!(ordinary_selection_for_test(&app), Some(target_ordinary));
        assert_eq!(
            search_for_test(&app).map(|search| search.query.as_str()),
            Some("target-search")
        );
        assert!(
            app.pane_runtimes
                .get(&rterm_core::PaneId::new(2))
                .is_some_and(|runtime| runtime.ui.ordinary_selection.is_none())
        );
        assert!(
            app.pane_runtimes
                .get(&rterm_core::PaneId::new(2))
                .is_some_and(|runtime| !runtime.ui.overlay_active())
        );
    }

    #[test]
    fn window_app_quick_multiple_complete_or_open_link_copies_captured_source_text() {
        for command in [
            WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursor,
            WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(
                WindowCopyDestination::Clipboard,
            ),
        ] {
            let clipboard = Arc::new(Mutex::new(Vec::new()));
            let recorded_clipboard = Arc::clone(&clipboard);
            let mut app = NativeWindowApp::new(None);
            app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
            app.handle_pty_output(b"target").unwrap();
            set_ordinary_viewport_range_for_test(
                &mut app,
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 2 },
            );
            let target_ordinary = ordinary_selection_for_test(&app).expect("target ordinary");
            app.dispatch_app_action(AppAction::SplitPane {
                pane: rterm_core::PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
            app.runtime.resize(rterm_core::TerminalSize::new(16, 1));
            app.handle_pty_output(b"source").unwrap();
            app.clipboard_writer = Box::new(move |text| {
                recorded_clipboard.lock().unwrap().push(text.to_owned());
                true
            });
            set_app_quick_select_for_test(
                &mut app,
                WindowQuickSelect {
                    matches: vec![WindowSearchMatch {
                        end_column: 6,
                        ..pane_overlay_match(0)
                    }],
                    labels: vec!["a".to_owned()],
                    action: WindowQuickSelectAction::Multiple(vec![
                        WindowCommand::ActivatePane1,
                        command,
                    ]),
                    ..WindowQuickSelect::default()
                },
            );
            app.update_selection_projection();

            app.accept_quick_select_match(false);

            assert_eq!(app.active_pane_id(), rterm_core::PaneId::new(1));
            assert_eq!(clipboard.lock().unwrap().as_slice(), ["source"]);
            assert_eq!(ordinary_selection_for_test(&app), Some(target_ordinary));
        }
    }

    #[test]
    fn window_app_quick_source_bound_inactive_write_rebuilds_bottom_viewport() {
        let source_written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(12, 2));
        app.handle_pty_output(b"one\r\ntwo\r\nthree\r\nfour")
            .unwrap();
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&source_written))));
        app.dispatch_app_action(AppAction::SplitPane {
            pane: rterm_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
        app.dispatch_app_action(AppAction::ActivatePane {
            pane: rterm_core::PaneId::new(1),
        })
        .unwrap();
        app.set_scrollback_offset_for_test(0);
        app.refresh_snapshot();
        set_ordinary_viewport_range_for_test(
            &mut app,
            SelectionCell { row: 0, column: 0 },
            SelectionCell { row: 0, column: 0 },
        );
        let ordinary = ordinary_selection_for_test(&app).expect("source ordinary selection");
        let expected_bottom_rows = [
            snapshot_row_text(&app.snapshot, 0, 12),
            snapshot_row_text(&app.snapshot, 1, 12),
        ];
        app.set_scrollback_offset_for_test(1);
        app.refresh_snapshot();
        let scrolled_rows = [
            snapshot_row_text(&app.snapshot, 0, 12),
            snapshot_row_text(&app.snapshot, 1, 12),
        ];
        assert_ne!(scrolled_rows, expected_bottom_rows);
        set_app_quick_select_for_test(
            &mut app,
            WindowQuickSelect {
                matches: vec![pane_overlay_match(0)],
                labels: vec!["a".to_owned()],
                action: WindowQuickSelectAction::Multiple(vec![
                    WindowCommand::ActivatePane2,
                    WindowCommand::SendString("write".to_owned()),
                ]),
                ..WindowQuickSelect::default()
            },
        );
        app.update_selection_projection();

        app.accept_quick_select_match(false);

        assert_eq!(source_written.lock().unwrap().as_slice(), b"write");
        let source = app
            .pane_runtimes
            .get(&rterm_core::PaneId::new(1))
            .expect("inactive source owner");
        assert_eq!(
            source.ui.stable_viewport,
            super::PaneStableViewport::default()
        );
        assert_eq!(
            [
                snapshot_row_text(&source.snapshot, 0, 12),
                snapshot_row_text(&source.snapshot, 1, 12),
            ],
            expected_bottom_rows
        );
        assert_eq!(source.ui.ordinary_selection, Some(ordinary));
        let projected = super::pane_overlay_viewport_selection(
            source.runtime.terminal(),
            &source.ui,
            &app.selection_word_boundary,
        )
        .expect("ordinary selection projects at bottom");
        assert!(
            projected.contains(0, 0, source.runtime.terminal().grid().size()),
            "ordinary overlay stays aligned with rebuilt bottom viewport"
        );
        assert!(app.frame_needs_full_repaint);
    }
