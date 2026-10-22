    #[test]
    fn window_app_parses_wezterm_lua_config_leader_direct_field_assignments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.leader = {}
            config.leader.key = 'a'
            config.leader.mods = 'CTRL'
            config.leader.timeout_milliseconds = 1000

            config.keys = {
              {
                key = '|',
                mods = 'LEADER|SHIFT',
                action = act.ShowDebugOverlay,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm leader direct field config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL;
        app.handle_keyboard_input_event(
            &Key::Character("a".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            Some("a"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(!app.debug_overlay_active_for_test());

        app.modifiers = ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character("|".into()),
            PhysicalKey::Code(WinitKeyCode::Backslash),
            Some("|"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(app.debug_overlay_active_for_test());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_leader_static_variable_post_assignment_fields() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local user_leader = {}

            config.leader = user_leader
            user_leader.key = 'a'
            user_leader.mods = 'CTRL'
            user_leader.timeout_milliseconds = 1000

            config.keys = {
              {
                key = '|',
                mods = 'LEADER|SHIFT',
                action = act.ShowDebugOverlay,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm leader post-assignment field config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL;
        app.handle_keyboard_input_event(
            &Key::Character("a".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            Some("a"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(!app.debug_overlay_active_for_test());

        app.modifiers = ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character("|".into()),
            PhysicalKey::Code(WinitKeyCode::Backslash),
            Some("|"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(app.debug_overlay_active_for_test());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_leader_static_field_variables() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local leader_key = 'a'
            local leader_mods = 'CTRL'
            local leader_timeout = 1000

            config.leader = {
              key = leader_key,
              mods = leader_mods,
              timeout_milliseconds = leader_timeout,
            }

            config.keys = {
              {
                key = '|',
                mods = 'LEADER|SHIFT',
                action = act.ShowDebugOverlay,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm leader static field variable config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL;
        app.handle_keyboard_input_event(
            &Key::Character("a".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            Some("a"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(!app.debug_overlay_active_for_test());

        app.modifiers = ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character("|".into()),
            PhysicalKey::Code(WinitKeyCode::Backslash),
            Some("|"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(app.debug_overlay_active_for_test());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_leader_static_field_name_variables() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local key_field = 'key'
            local mods_field = 'mods'
            local timeout_field = 'timeout_milliseconds'

            config.leader = {
              [key_field] = 'a',
              [mods_field] = 'CTRL',
              [timeout_field] = 1000,
            }

            config.keys = {
              {
                key = '|',
                mods = 'LEADER|SHIFT',
                action = act.ShowDebugOverlay,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm leader static field-name variable config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL;
        app.handle_keyboard_input_event(
            &Key::Character("a".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            Some("a"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(!app.debug_overlay_active_for_test());

        app.modifiers = ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character("|".into()),
            PhysicalKey::Code(WinitKeyCode::Backslash),
            Some("|"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(app.debug_overlay_active_for_test());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_leader_config_long_bracket_key_query() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.leader = { [[=[key]=]] = 'a', [[=[mods]=]] = 'CTRL', [[=[timeout_milliseconds]=]] = 1000 }

            config.keys = {
              {
                key = '|',
                mods = 'LEADER|SHIFT',
                action = act.ShowDebugOverlay,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm leader config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL;
        app.handle_keyboard_input_event(
            &Key::Character("a".into()),
            PhysicalKey::Code(WinitKeyCode::KeyA),
            Some("a"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();
        assert!(!app.debug_overlay_active_for_test());

        app.modifiers = ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character("|".into()),
            PhysicalKey::Code(WinitKeyCode::Backslash),
            Some("|"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert!(app.debug_overlay_active_for_test());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_default_launch_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = { 'nu', '--login' }
            config.default_cwd = 'C:/Project Dir'
            config.term = 'wezterm'
            config.set_environment_variables = {
              PROJECT_MODE = 'dev',
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("C:/Project Dir")));
        assert_eq!(command.env_value("TERM"), Some("wezterm"));
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_default_prog_static_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local shell = { 'nu', '--login' }

            config.default_prog = shell
            config.default_cwd = 'C:/Project Dir'

            return config
            "#,
        )
        .expect("expected WezTerm default_prog variable config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("C:/Project Dir")));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_default_prog_table_insert() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = {}
            table.insert(config.default_prog, 'nu')
            table.insert(config.default_prog, '--login')
            config.default_cwd = 'C:/Project Dir'

            return config
            "#,
        )
        .expect("expected WezTerm default_prog table insert config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("C:/Project Dir")));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_default_prog_static_variable_post_assignment_inserts() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local shell = {}

            config.default_prog = shell
            table.insert(shell, 'nu')
            table.insert(shell, '--login')
            config.default_cwd = 'C:/Project Dir'

            return config
            "#,
        )
        .expect("expected WezTerm default_prog post-assignment insert config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("C:/Project Dir")));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_environment_static_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local env = {
              PROJECT_MODE = 'dev',
              FEATURE_FLAG = 'on',
            }

            config.default_prog = { 'nu', '--login' }
            config.set_environment_variables = env

            return config
            "#,
        )
        .expect("expected WezTerm environment variable table config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
        assert_eq!(command.env_value("FEATURE_FLAG"), Some("on"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_environment_static_field_name() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local env_key = 'PROJECT_MODE'
            local env_value = 'dev'

            config.default_prog = { 'nu', '--login' }
            config.set_environment_variables = {
              [env_key] = env_value,
            }

            return config
            "#,
        )
        .expect("expected WezTerm environment static field-name config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_environment_static_variable_field_mutations() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local env = {}
            env.PROJECT_MODE = 'dev'
            env['FEATURE_FLAG'] = 'on'

            config.default_prog = { 'nu', '--login' }
            config.set_environment_variables = env

            return config
            "#,
        )
        .expect("expected WezTerm environment variable table mutation config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
        assert_eq!(command.env_value("FEATURE_FLAG"), Some("on"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_environment_static_variable_initializer_post_mutations()
    {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local env = {}
            local config = {
              default_prog = { 'nu', '--login' },
              set_environment_variables = env,
            }
            env.PROJECT_MODE = 'dev'
            env['FEATURE_FLAG'] = 'on'

            return config
            "#,
        )
        .expect("expected WezTerm environment initializer post-mutation config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
        assert_eq!(command.env_value("FEATURE_FLAG"), Some("on"));
    }

    #[test]
    fn window_app_parses_static_initializer_set_environment_variables_key() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local env_field = 'set_environment_variables'
            local config = {
              default_prog = { 'nu', '--login' },
              [env_field] = {
                PROJECT_MODE = 'dev',
                FEATURE_FLAG = 'on',
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm static field-name initializer environment config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
        assert_eq!(command.env_value("FEATURE_FLAG"), Some("on"));
    }

    #[test]
    fn window_app_parses_static_key_environment_table_assignment() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local env_field = 'set_environment_variables'
            local config = {}

            config.default_prog = { 'nu', '--login' }
            config[env_field] = {
              PROJECT_MODE = 'dev',
              FEATURE_FLAG = 'on',
            }

            return config
            "#,
        )
        .expect("expected WezTerm static field-name environment table config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
        assert_eq!(command.env_value("FEATURE_FLAG"), Some("on"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_environment_static_variable_post_assignment_mutations()
    {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local env = {}

            config.default_prog = { 'nu', '--login' }
            config.set_environment_variables = env
            env.PROJECT_MODE = 'dev'
            env['FEATURE_FLAG'] = 'on'

            return config
            "#,
        )
        .expect("expected WezTerm environment variable post-assignment mutation config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
        assert_eq!(command.env_value("FEATURE_FLAG"), Some("on"));
    }

    #[test]
    fn window_app_parses_static_environment_variable_field_name_mutation() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local env = {}
            local entry_field = 'PROJECT_MODE'
            local env_value = 'dev'

            config.default_prog = { 'nu', '--login' }
            config.set_environment_variables = env
            env[entry_field] = env_value

            return config
            "#,
        )
        .expect("expected WezTerm static environment variable field-name mutation config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
    }

    #[test]
    fn window_app_stops_following_reassigned_wezterm_lua_environment_static_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local env = {}

            config.default_prog = { 'nu', '--login' }
            config.set_environment_variables = env
            env.PROJECT_MODE = 'dev'
            env = {}
            env['FEATURE_FLAG'] = 'on'

            return config
            "#,
        )
        .expect("expected WezTerm environment variable reassignment config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
        assert_eq!(command.env_value("FEATURE_FLAG"), None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_environment_field_mutations() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = { 'nu', '--login' }
            config.set_environment_variables = {}
            config.set_environment_variables.PROJECT_MODE = 'dev'
            config.set_environment_variables.FEATURE_FLAG = 'on'

            return config
            "#,
        )
        .expect("expected WezTerm environment variable mutation config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
        assert_eq!(command.env_value("FEATURE_FLAG"), Some("on"));
    }

    #[test]
    fn window_app_parses_static_key_environment_field_mutations() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local env_field = 'set_environment_variables'
            local config = {}

            config.default_prog = { 'nu', '--login' }
            config[env_field] = {}
            config[env_field].PROJECT_MODE = 'dev'
            config[env_field]['FEATURE_FLAG'] = 'on'

            return config
            "#,
        )
        .expect("expected WezTerm static field-name environment mutation config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
        assert_eq!(command.env_value("FEATURE_FLAG"), Some("on"));
    }

    #[test]
    fn window_app_parses_static_environment_field_name_mutation() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local env_field = 'set_environment_variables'
            local entry_field = 'PROJECT_MODE'
            local env_value = 'dev'
            local config = {}

            config.default_prog = { 'nu', '--login' }
            config[env_field] = {}
            config[env_field][entry_field] = env_value

            return config
            "#,
        )
        .expect("expected WezTerm static environment field-name mutation config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
    }

    #[test]
    fn window_app_uses_later_wezterm_lua_config_field_assignments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = { 'bad-shell', '--bad' }
            config.default_cwd = 'C:/Bad Dir'
            config.default_prog = { 'nu', '--login' }
            config.default_cwd = 'C:/Project Dir'

            return config
            "#,
        )
        .expect("expected later WezTerm launch config assignments");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("C:/Project Dir")));
    }

    #[test]
    fn window_app_uses_later_wezterm_lua_config_table_assignment_after_fields() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = { 'bad-shell', '--bad' }
            config.default_cwd = 'C:/Bad Dir'
            config = {
              default_prog = { 'nu', '--login' },
              default_cwd = 'C:/Project Dir',
            }

            return config
            "#,
        )
        .expect("expected later WezTerm config table assignment");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("C:/Project Dir")));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_static_return_table_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            return {
              default_prog = { 'nu', '--login' },
              default_cwd = 'C:/Project Dir',
            }
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("C:/Project Dir")));
    }

    #[test]
    fn window_app_parses_static_return_table_set_environment_variables_key() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local env_field = 'set_environment_variables'

            return {
              default_prog = { 'nu', '--login' },
              [env_field] = {
                PROJECT_MODE = 'dev',
                FEATURE_FLAG = 'on',
              },
            }
            "#,
        )
        .expect("expected WezTerm static field-name return table environment config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
        assert_eq!(command.env_value("FEATURE_FLAG"), Some("on"));
    }

    #[test]
    fn window_app_uses_later_wezterm_lua_return_table_duplicate_fields() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            return {
              default_prog = { 'bad-shell', '--bad' },
              default_cwd = 'C:/Bad Dir',
              default_prog = { 'nu', '--login' },
              default_cwd = 'C:/Project Dir',
            }
            "#,
        )
        .expect("expected duplicate WezTerm return-table launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("C:/Project Dir")));
    }

    #[test]
    fn window_app_uses_wezterm_lua_return_table_launch_after_config_assignment() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = { 'bad-shell', '--bad' }
            config.default_cwd = 'C:/Bad Dir'

            return {
              default_prog = { 'nu', '--login' },
              default_cwd = 'C:/Project Dir',
            }
            "#,
        )
        .expect("expected WezTerm return-table launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("C:/Project Dir")));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_local_config_table_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {
              default_prog = { 'nu', '--login' },
              default_cwd = 'C:/Project Dir',
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("C:/Project Dir")));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_top_level_config_table_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            config = {
              default_prog = { 'nu', '--login' },
              default_cwd = 'C:/Project Dir',
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("C:/Project Dir")));
    }

    #[test]
    fn window_app_parses_wezterm_lua_returned_config_variable_launch_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local cfg = {}

            cfg.default_prog = { 'nu', '--login' }
            cfg.default_cwd = 'C:/Project Dir'
            cfg.term = 'wezterm'

            return cfg
            "#,
        )
        .expect("expected returned WezTerm config variable launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("C:/Project Dir")));
        assert_eq!(command.env_value("TERM"), Some("wezterm"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_static_return_table_after_helper_return() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            local function ignored()
              return config
            end

            return {
              default_prog = { 'nu', '--login' },
            }
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);
    }

    #[test]
    fn window_app_ignores_wezterm_lua_config_helper_return_tables() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'

            local function ignored()
              return {
                default_prog = { 'bad-shell', '--bad' },
                default_cwd = 'C:/Bad Dir',
              }
            end

            local config = {}
            config.term = 'wezterm'
            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert_eq!(app.default_prog, None);
        assert_eq!(app.default_cwd, None);
        assert_eq!(app.term, "wezterm");
    }

    #[test]
    fn window_app_ignores_wezterm_lua_config_helper_config_assignments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            local function ignored()
              config.default_prog = { 'bad-shell', '--bad' }
              config['default_cwd'] = 'C:/Bad Dir'
            end

            config.term = 'wezterm'
            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert_eq!(app.default_prog, None);
        assert_eq!(app.default_cwd, None);
        assert_eq!(app.term, "wezterm");
    }

    #[test]
    fn window_app_ignores_wezterm_lua_config_helper_static_table_variable_assignments() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action

            local user_keys = {}
            local function ignored()
              user_keys = {
                {
                  key = 'H',
                  mods = 'CTRL|SHIFT',
                  action = act.SendString 'bad-helper-binding',
                },
              }
            end

            local config = {}
            config.keys = user_keys
            return config
            "#,
        )
        .expect("expected WezTerm empty user keys config");

        assert_eq!(overrides.key_assignments, Some(Vec::new()));
    }

    #[test]
    fn window_app_ignores_wezterm_lua_config_bare_local_fields() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local default_prog = { 'bad-shell', '--bad' }
            local default_cwd = 'C:/Bad Dir'
            local config = {}
            config.term = 'wezterm'

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert_eq!(app.default_prog, None);
        assert_eq!(app.default_cwd, None);
        assert_eq!(app.term, "wezterm");
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_top_level_bracket_key_query() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config[ [[default_prog]] ] = { 'nu', '--login' }
            config['default_cwd'] = 'C:/Project Dir'
            config["term"] = 'wezterm'
            config[ [=[set_environment_variables]=] ] = {
              PROJECT_MODE = 'dev',
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("C:/Project Dir")));
        assert_eq!(command.env_value("TERM"), Some("wezterm"));
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_bracket_key_comments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config[
              -- launch command key
              'default_prog' -- close bracket after comment
            ] = { 'nu', '--login' }

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_assignment_comments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog -- launch command
              = { 'nu', '--login' }
            config.default_cwd -- launch cwd
              = 'C:/Project Dir'

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("C:/Project Dir")));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_assignment_value_prefix_comments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog =
              -- launch command
              { 'nu', '--login' }
            config.default_cwd =
              --[=[ launch cwd ]=]
              'C:/Project Dir'

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("C:/Project Dir")));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_bracket_assignment_value_prefix_comments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config[ [[default_prog]] ] =
              -- launch command
              { 'nu', '--login' }
            config[ [[default_cwd]] ] =
              --[=[ launch cwd ]=]
              'C:/Project Dir'

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("C:/Project Dir")));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_table_long_bracket_string_with_brace() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = { 'nu', '--login' }
            config.set_environment_variables = {
              PROJECT_MODE = [=[dev}ops]=],
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev}ops"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_table_comments_with_brace() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = {
              -- ignored } line comment
              'nu',
              --[=[
              ignored } block comment
              ]=]
              '--login',
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_table_line_comments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = { 'nu', '--login' }
            config.set_environment_variables = {
              -- ignored, comment separator
              PROJECT_MODE = 'dev',
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_table_block_comments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = { 'nu', '--login' }
            config.set_environment_variables = {
              --[=[
              ignored, comment separator
              ]=]
              PROJECT_MODE = 'dev',
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_table_assignment_comments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = { 'nu', '--login' }
            config.set_environment_variables = {
              PROJECT_MODE --[[ environment key ]] = 'dev',
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_table_bracket_key_comments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = { 'nu', '--login' }
            config.set_environment_variables = {
              [
                -- environment key
                'PROJECT_MODE' -- close bracket after comment
              ] = 'dev',
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_table_numeric_index_comments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = {
              [
                -- program index
                1 -- close bracket after comment
              ] = 'nu',
              [
                -- argument index
                2 -- close bracket after comment
              ] = '--login',
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_table_value_prefix_comments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = { 'nu', '--login' }
            config.set_environment_variables = {
              PROJECT_MODE =
                -- environment value
                'dev',
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_indexed_table_value_prefix_comments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = {
              [1] =
                -- program value
                'nu',
              [2] =
                -- argument value
                '--login',
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_table_value_comments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_prog = { 'nu', '--login' }
            config.set_environment_variables = {
              PROJECT_MODE = 'dev' -- environment value
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);

        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.env_value("PROJECT_MODE"), Some("dev"));
    }

    #[test]
    fn window_app_ignores_wezterm_lua_config_long_block_comments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            --[[
            config.default_prog = { 'bad-shell', '--bad' }
            config.default_cwd = 'C:/Ignored'
            ]]

            config.default_prog = { 'nu', '--login' }
            config.default_cwd = 'C:/Project Dir'

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);
        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("C:/Project Dir")));
    }

    #[test]
    fn window_app_ignores_wezterm_lua_config_long_bracket_strings() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            local ignored = [=[
            config.default_prog = { 'bad-shell', '--bad' }
            config.default_cwd = 'C:/Ignored'
            ]=]

            config.default_prog = { 'nu', '--login' }
            config.default_cwd = 'C:/Project Dir'

            return config
            "#,
        )
        .expect("expected WezTerm launch config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::NewTab));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(launch.program(), "nu");
        assert_eq!(launch.args(), ["--login"]);
        let command = pty_command_from_pane_launch_with_environment(
            launch,
            &app.term,
            &app.set_environment_variables,
            app.default_cwd.as_deref(),
        );
        assert_eq!(command.cwd(), Some(Path::new("C:/Project Dir")));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_paste_and_input_overrides() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.clipboard_reader = Box::new(|| Some("paste\ntext".to_owned()));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.canonicalize_pasted_newlines = 'CarriageReturnAndLineFeed'
            config.quote_dropped_files = 'Posix'
            config.disable_default_key_bindings = true
            config.disable_default_mouse_bindings = true
            config.hide_mouse_cursor_when_typing = false

            return config
            "#,
        )
        .expect("expected WezTerm paste/input config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.canonicalize_pasted_newlines,
            NativeCanonicalizePastedNewlines::CarriageReturnAndLineFeed
        );
        assert_eq!(
            effective.quote_dropped_files,
            NativeQuoteDroppedFiles::Posix
        );
        assert!(effective.disable_default_key_bindings);
        assert!(effective.disable_default_mouse_bindings);
        assert!(!effective.hide_mouse_cursor_when_typing);

        app.command_palette_execute(WindowCommand::PasteFrom(WindowPasteSource::Clipboard));
        assert_eq!(written.lock().unwrap().as_slice(), b"paste\r\ntext");
        written.lock().unwrap().clear();

        app.handle_dropped_file_path(std::path::Path::new("hello ($world)"))
            .unwrap();
        assert_eq!(written.lock().unwrap().as_slice(), b"\"hello (\\$world)\"");
    }

    #[test]
    fn window_app_reports_default_wezterm_password_input_detection_config() {
        let app = NativeWindowApp::new(None);
        let effective = app.native_effective_config();

        assert_eq!(
            effective.detect_password_input,
            DEFAULT_DETECT_PASSWORD_INPUT
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_password_input_detection_override() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.detect_password_input = false

            return config
            "#,
        )
        .expect("expected WezTerm password-input config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert!(!effective.detect_password_input);
    }

    #[test]
    fn window_app_reports_default_wezterm_gui_input_misc_config() {
        let app = NativeWindowApp::new(None);
        let effective = app.native_effective_config();

        assert!(effective.allow_download_protocols);
        assert_eq!(effective.xcursor_theme, None);
        assert_eq!(effective.xcursor_size, None);
        assert_eq!(effective.palette_max_key_assigments_for_action, 1);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_gui_input_misc_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.allow_download_protocols = false
            config.xcursor_theme = 'Adwaita'
            config.xcursor_size = 24
            config.palette_max_key_assigments_for_action = 3

            return config
            "#,
        )
        .expect("expected WezTerm GUI/input misc config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert!(!effective.allow_download_protocols);
        assert_eq!(effective.xcursor_theme, Some("Adwaita".to_owned()));
        assert_eq!(effective.xcursor_size, Some(24));
        assert_eq!(effective.palette_max_key_assigments_for_action, 3);
    }

    #[test]
    fn window_app_reports_default_wezterm_bidi_config() {
        let app = NativeWindowApp::new(None);
        let effective = app.native_effective_config();

        assert!(!effective.bidi_enabled);
        assert_eq!(effective.bidi_direction, NativeBidiDirection::LeftToRight);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_bidi_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.bidi_enabled = true
            config.bidi_direction = 'AutoRightToLeft'

            return config
            "#,
        )
        .expect("expected WezTerm bidi config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert!(effective.bidi_enabled);
        assert_eq!(
            effective.bidi_direction,
            NativeBidiDirection::AutoRightToLeft
        );
    }

    #[test]
    fn window_app_reports_default_wezterm_mux_diagnostic_config() {
        let app = NativeWindowApp::new(None);
        let effective = app.native_effective_config();

        assert_eq!(effective.default_mux_server_domain, None);
        assert_eq!(effective.ratelimit_mux_line_prefetches_per_second, 50);
        assert_eq!(effective.mux_output_parser_buffer_size, 128 * 1024);
        assert_eq!(effective.mux_output_parser_coalesce_delay_ms, 3);
        assert_eq!(effective.periodic_stat_logging, 0);
        assert_eq!(effective.ulimit_nofile, 2048);
        assert_eq!(effective.ulimit_nproc, 2048);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_mux_diagnostic_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.default_mux_server_domain = 'mux-main'
            config.ratelimit_mux_line_prefetches_per_second = 12
            config.mux_output_parser_buffer_size = 4096
            config.mux_output_parser_coalesce_delay_ms = 7
            config.periodic_stat_logging = 15
            config.ulimit_nofile = 4096
            config.ulimit_nproc = 8192

            return config
            "#,
        )
        .expect("expected WezTerm mux diagnostic config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.default_mux_server_domain,
            Some("mux-main".to_owned())
        );
        assert_eq!(effective.ratelimit_mux_line_prefetches_per_second, 12);
        assert_eq!(effective.mux_output_parser_buffer_size, 4096);
        assert_eq!(effective.mux_output_parser_coalesce_delay_ms, 7);
        assert_eq!(effective.periodic_stat_logging, 15);
        assert_eq!(effective.ulimit_nofile, 4096);
        assert_eq!(effective.ulimit_nproc, 8192);
    }

    #[test]
    fn window_app_reports_default_wezterm_startup_ssh_environment_config() {
        let app = NativeWindowApp::new(None);
        let effective = app.native_effective_config();

        assert_eq!(effective.default_gui_startup_args, vec!["start".to_owned()]);
        assert_eq!(effective.ssh_backend, NativeSshBackend::LibSsh);
        assert_eq!(
            effective.tiling_desktop_environments,
            vec![
                "X11 LG3D".to_owned(),
                "X11 Qtile".to_owned(),
                "X11 awesome".to_owned(),
                "X11 bspwm".to_owned(),
                "X11 dwm".to_owned(),
                "X11 i3".to_owned(),
                "X11 xmonad".to_owned(),
            ]
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_startup_ssh_environment_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.default_gui_startup_args = { 'connect', 'prod' }
            config.ssh_backend = 'Ssh2'
            config.tiling_desktop_environments = { 'X11 i3', 'Wayland Sway' }

            return config
            "#,
        )
        .expect("expected WezTerm startup SSH environment config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.default_gui_startup_args,
            vec!["connect".to_owned(), "prod".to_owned()]
        );
        assert_eq!(effective.ssh_backend, NativeSshBackend::Ssh2);
        assert_eq!(
            effective.tiling_desktop_environments,
            vec!["X11 i3".to_owned(), "Wayland Sway".to_owned()]
        );
    }

    #[test]
    fn window_app_reports_default_wezterm_daemon_options_config() {
        let app = NativeWindowApp::new(None);
        let effective = app.native_effective_config();

        assert_eq!(effective.daemon_options, NativeDaemonOptions::default());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_daemon_options_override() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.daemon_options = {
                pid_file = 'run/wezterm.pid',
                stdout = 'logs/wezterm.out',
                stderr = 'logs/wezterm.err',
            }

            return config
            "#,
        )
        .expect("expected WezTerm daemon options config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().daemon_options,
            NativeDaemonOptions {
                pid_file: Some("run/wezterm.pid".to_owned()),
                stdout: Some("logs/wezterm.out".to_owned()),
                stderr: Some("logs/wezterm.err".to_owned()),
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_daemon_options_static_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local pid_path = 'run/static.pid'
            local daemon = {
                pid_file = pid_path,
                stdout = 'logs/static.out',
                stderr = 'logs/static.err',
            }

            config.daemon_options = daemon

            return config
            "#,
        )
        .expect("expected WezTerm daemon options static-variable config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().daemon_options,
            NativeDaemonOptions {
                pid_file: Some("run/static.pid".to_owned()),
                stdout: Some("logs/static.out".to_owned()),
                stderr: Some("logs/static.err".to_owned()),
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_keyboard_protocol_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.key_map_preference = 'Physical'
            config.swap_backspace_and_delete = true
            config.enable_kitty_graphics = false
            config.enable_checksum_rectangular_area = true
            config.enable_title_reporting = true
            config.enable_csi_u_key_encoding = true
            config.enable_kitty_keyboard = true
            config.allow_win32_input_mode = false
            config.treat_left_ctrlalt_as_altgr = true
            config.send_composed_key_when_left_alt_is_pressed = true
            config.send_composed_key_when_right_alt_is_pressed = false

            return config
            "#,
        )
        .expect("expected WezTerm keyboard protocol config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.key_map_preference,
            NativeKeyMapPreference::Physical
        );
        assert!(effective.swap_backspace_and_delete);
        assert!(!effective.enable_kitty_graphics);
        assert!(effective.enable_checksum_rectangular_area);
        assert!(effective.enable_title_reporting);
        assert!(effective.enable_csi_u_key_encoding);
        assert!(effective.enable_kitty_keyboard);
        assert!(!effective.allow_win32_input_mode);
        assert!(effective.treat_left_ctrlalt_as_altgr);
        assert!(effective.send_composed_key_when_left_alt_is_pressed);
        assert!(!effective.send_composed_key_when_right_alt_is_pressed);
    }

    #[test]
    fn window_app_reports_default_wezterm_altgr_config() {
        let app = NativeWindowApp::new(None);
        let effective = app.native_effective_config();

        assert_eq!(
            effective.treat_left_ctrlalt_as_altgr,
            DEFAULT_TREAT_LEFT_CTRLALT_AS_ALTGR
        );
        assert!(!effective.send_composed_key_when_left_alt_is_pressed);
        assert!(effective.send_composed_key_when_right_alt_is_pressed);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_east_asian_ambiguous_width() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.treat_east_asian_ambiguous_width_as_wide = true

            return config
            "#,
        )
        .expect("expected WezTerm ambiguous-width config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert!(effective.treat_east_asian_ambiguous_width_as_wide);

        app.runtime.feed_pty_output("☆x".as_bytes());
        assert_eq!(app.runtime.terminal().cursor(), (0, 3));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_cell_widths() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.treat_east_asian_ambiguous_width_as_wide = true
            config.cell_widths = {
                { first = 0x2606, last = 0x2606, width = 1 },
                { first = 0xe000, last = 0xf8ff, width = 2 },
            }

            return config
            "#,
        )
        .expect("expected WezTerm cell width config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.cell_widths,
            vec![
                NativeCellWidthOverride::new(0x2606, 0x2606, 1),
                NativeCellWidthOverride::new(0xe000, 0xf8ff, 2),
            ]
        );

        app.runtime.feed_pty_output("☆x".as_bytes());
        assert_eq!(app.runtime.terminal().cursor(), (0, 2));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_cell_widths_static_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local widths = {
                { first = 0x2606, last = 0x2606, width = 1 },
            }

            config.treat_east_asian_ambiguous_width_as_wide = true
            config.cell_widths = widths

            return config
            "#,
        )
        .expect("expected WezTerm cell width variable config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.cell_widths,
            vec![NativeCellWidthOverride::new(0x2606, 0x2606, 1)]
        );

        app.runtime.feed_pty_output("☆x".as_bytes());
        assert_eq!(app.runtime.terminal().cursor(), (0, 2));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_cell_widths_static_field_variables() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local star_first = 0x2606
            local star_last = 0x2606
            local star_width = 1

            config.treat_east_asian_ambiguous_width_as_wide = true
            config.cell_widths = {
                { first = star_first, last = star_last, width = star_width },
            }

            return config
            "#,
        )
        .expect("expected WezTerm cell width static field variable config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.cell_widths,
            vec![NativeCellWidthOverride::new(0x2606, 0x2606, 1)]
        );

        app.runtime.feed_pty_output("☆x".as_bytes());
        assert_eq!(app.runtime.terminal().cursor(), (0, 2));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_cell_widths_static_entry_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local star = { first = 0x2606, last = 0x2606, width = 1 }

            config.treat_east_asian_ambiguous_width_as_wide = true
            config.cell_widths = { star }

            return config
            "#,
        )
        .expect("expected WezTerm cell width static entry variable config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.cell_widths,
            vec![NativeCellWidthOverride::new(0x2606, 0x2606, 1)]
        );

        app.runtime.feed_pty_output("☆x".as_bytes());
        assert_eq!(app.runtime.terminal().cursor(), (0, 2));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_cell_widths_static_field_name_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local widths_field = 'cell_widths'

            config.treat_east_asian_ambiguous_width_as_wide = true
            config[widths_field] = {
                { first = 0x2606, last = 0x2606, width = 1 },
            }

            return config
            "#,
        )
        .expect("expected WezTerm cell width static field-name variable config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.cell_widths,
            vec![NativeCellWidthOverride::new(0x2606, 0x2606, 1)]
        );

        app.runtime.feed_pty_output("☆x".as_bytes());
        assert_eq!(app.runtime.terminal().cursor(), (0, 2));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_cell_widths_static_field_name_variable_table_insert() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local widths_field = 'cell_widths'

            config.treat_east_asian_ambiguous_width_as_wide = true
            config[widths_field] = {}
            table.insert(config[widths_field], { first = 0x2606, last = 0x2606, width = 1 })

            return config
            "#,
        )
        .expect("expected WezTerm cell width static field-name table.insert config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.cell_widths,
            vec![NativeCellWidthOverride::new(0x2606, 0x2606, 1)]
        );

        app.runtime.feed_pty_output("☆x".as_bytes());
        assert_eq!(app.runtime.terminal().cursor(), (0, 2));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_cell_widths_static_field_name_length_append() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local widths_field = 'cell_widths'

            config.treat_east_asian_ambiguous_width_as_wide = true
            config[widths_field] = {}
            config[widths_field][#config[widths_field] + 1] = {
                first = 0x2606,
                last = 0x2606,
                width = 1,
            }

            return config
            "#,
        )
        .expect("expected WezTerm cell width static field-name length append config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.cell_widths,
            vec![NativeCellWidthOverride::new(0x2606, 0x2606, 1)]
        );

        app.runtime.feed_pty_output("☆x".as_bytes());
        assert_eq!(app.runtime.terminal().cursor(), (0, 2));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_cell_widths_static_field_name_in_table_constructor() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local widths_field = 'cell_widths'
            local config = {
                treat_east_asian_ambiguous_width_as_wide = true,
                [widths_field] = {
                    { first = 0x2606, last = 0x2606, width = 1 },
                },
            }

            return config
            "#,
        )
        .expect("expected WezTerm cell width static field-name constructor config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert!(effective.treat_east_asian_ambiguous_width_as_wide);
        assert_eq!(
            effective.cell_widths,
            vec![NativeCellWidthOverride::new(0x2606, 0x2606, 1)]
        );

        app.runtime.feed_pty_output("☆x".as_bytes());
        assert_eq!(app.runtime.terminal().cursor(), (0, 2));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_cell_widths_static_field_name_in_return_table() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local widths_field = 'cell_widths'

            return {
                treat_east_asian_ambiguous_width_as_wide = true,
                [widths_field] = {
                    { first = 0x2606, last = 0x2606, width = 1 },
                },
            }
            "#,
        )
        .expect("expected WezTerm cell width static field-name return table config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert!(effective.treat_east_asian_ambiguous_width_as_wide);
        assert_eq!(
            effective.cell_widths,
            vec![NativeCellWidthOverride::new(0x2606, 0x2606, 1)]
        );

        app.runtime.feed_pty_output("☆x".as_bytes());
        assert_eq!(app.runtime.terminal().cursor(), (0, 2));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_cell_widths_table_insert() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.treat_east_asian_ambiguous_width_as_wide = true
            config.cell_widths = {}
            table.insert(config.cell_widths, { first = 0x2606, last = 0x2606, width = 1 })

            return config
            "#,
        )
        .expect("expected WezTerm cell width table insert config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.cell_widths,
            vec![NativeCellWidthOverride::new(0x2606, 0x2606, 1)]
        );

        app.runtime.feed_pty_output("☆x".as_bytes());
        assert_eq!(app.runtime.terminal().cursor(), (0, 2));
    }

    #[test]
    fn window_app_normalizes_output_to_unicode_nfc_when_configured() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.normalize_output_to_unicode_nfc = true

            return config
            "#,
        )
        .expect("expected WezTerm Unicode normalization config");
        app.set_config_overrides(overrides);

        app.handle_pty_output("e\u{0301}x".as_bytes()).unwrap();

        let snapshot = app.render_snapshot();
        let first_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_row.starts_with("éx"),
            "expected NFC-normalized terminal output, got {first_row:?}"
        );
        assert_eq!(app.runtime.terminal().cursor(), (0, 2));
    }

    #[test]
    fn window_app_normalizes_output_to_unicode_nfc_across_pty_chunks_when_configured() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.normalize_output_to_unicode_nfc = true

            return config
            "#,
        )
        .expect("expected WezTerm Unicode normalization config");
        app.set_config_overrides(overrides);

        app.handle_pty_output("e".as_bytes()).unwrap();
        app.handle_pty_output("\u{0301}x".as_bytes()).unwrap();

        let snapshot = app.render_snapshot();
        let first_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, TERMINAL_COLUMNS);
        assert!(
            first_row.starts_with("éx"),
            "expected NFC-normalized terminal output across PTY chunks, got {first_row:?}"
        );
        assert_eq!(app.runtime.terminal().cursor(), (0, 2));
    }

    #[test]
    fn window_app_reports_default_wezterm_east_asian_ambiguous_width_config() {
        let app = NativeWindowApp::new(None);
        let effective = app.native_effective_config();

        assert_eq!(
            effective.treat_east_asian_ambiguous_width_as_wide,
            DEFAULT_TREAT_EAST_ASIAN_AMBIGUOUS_WIDTH_AS_WIDE
        );
    }

    #[test]
    fn window_app_reports_default_wezterm_ime_config() {
        let app = NativeWindowApp::new(None);
        let effective = app.native_effective_config();

        assert_eq!(effective.use_ime, DEFAULT_USE_IME);
        assert_eq!(effective.use_dead_keys, DEFAULT_USE_DEAD_KEYS);
        assert_eq!(
            effective.ime_preedit_rendering,
            NativeImePreeditRendering::Builtin
        );
        assert_eq!(effective.xim_im_name, None);
        assert_eq!(
            effective.macos_forward_to_ime_modifier_mask,
            ModifiersState::SHIFT
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_ime_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.use_ime = false
            config.use_dead_keys = false
            config.ime_preedit_rendering = 'System'
            config.xim_im_name = 'fcitx'
            config.macos_forward_to_ime_modifier_mask = 'SHIFT|CTRL'

            return config
            "#,
        )
        .expect("expected WezTerm IME config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert!(!effective.use_ime);
        assert!(!effective.use_dead_keys);
        assert_eq!(
            effective.ime_preedit_rendering,
            NativeImePreeditRendering::System
        );
        assert_eq!(effective.xim_im_name.as_deref(), Some("fcitx"));
        assert_eq!(
            effective.macos_forward_to_ime_modifier_mask,
            ModifiersState::SHIFT | ModifiersState::CONTROL
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_static_field_name_return_table_ime_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local ime_field = 'use_ime'
            local dead_keys_field = 'use_dead_keys'
            local preedit_field = 'ime_preedit_rendering'
            local xim_field = 'xim_im_name'
            local forward_field = 'macos_forward_to_ime_modifier_mask'

            return {
                [ime_field] = false,
                [dead_keys_field] = false,
                [preedit_field] = 'System',
                [xim_field] = 'fcitx',
                [forward_field] = 'SHIFT|CTRL',
            }
            "#,
        )
        .expect("expected WezTerm static field-name return table IME config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert!(!effective.use_ime);
        assert!(!effective.use_dead_keys);
        assert_eq!(
            effective.ime_preedit_rendering,
            NativeImePreeditRendering::System
        );
        assert_eq!(effective.xim_im_name.as_deref(), Some("fcitx"));
        assert_eq!(
            effective.macos_forward_to_ime_modifier_mask,
            ModifiersState::SHIFT | ModifiersState::CONTROL
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_ui_key_cap_rendering() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.ui_key_cap_rendering = 'Emacs'

            return config
            "#,
        )
        .expect("expected WezTerm ui_key_cap_rendering config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().ui_key_cap_rendering,
            NativeUiKeyCapRendering::Emacs
        );
    }

    #[test]
    fn window_app_renders_key_assignments_with_configured_ui_key_caps() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.resize(rterm_core::TerminalSize::new(48, 8));
        app.set_config_overrides(native_config_snapshot! {
            ui_key_cap_rendering: Some(NativeUiKeyCapRendering::Emacs),
            ..NativeConfigSnapshot::default()
        });

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::key_assignments(),
                title: Some("Pick Key".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));

        let snapshot = app.render_snapshot();
        let first_row = snapshot_row_text(&snapshot, TAB_BAR_ROWS, 48);
        assert!(
            first_row.contains("> C-S-T: New Tab"),
            "expected Emacs key cap rendering in first key-assignment row: {first_row:?}"
        );
    }

    #[test]
    fn window_app_delivered_shift_text_reaches_pty_with_ime_enabled() {
        for text in [
            "(", ")", "（", "）", "!", "@", "#", "$", "%", "^", "&", "*",
            "_", "+", "{", "}", "|", ":", "\"", "<", ">", "?", "A", "，", "。", "！", "？",
        ] {
            let written = Arc::new(Mutex::new(Vec::new()));
            let mut app = NativeWindowApp::new(None);
            app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
            app.modifiers = ModifiersState::SHIFT;
            app.handle_keyboard_input_event(
                &Key::Character(text.into()),
                PhysicalKey::Code(WinitKeyCode::Digit9),
                Some(text),
                ElementState::Pressed,
                KittyKeyEventKind::Press,
            )
            .unwrap();
            assert_eq!(
                written.lock().unwrap().as_slice(),
                text.as_bytes(),
                "lost shifted text {text:?}",
            );
        }
    }

    #[test]
    fn window_app_ime_commits_fullwidth_punctuation_once() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.modifiers = ModifiersState::SHIFT;
        app.handle_ime_preedit("取消");
        app.handle_ime_preedit("");
        assert!(written.lock().unwrap().is_empty());
        assert!(app.ime_preedit.is_none());
        app.handle_ime_preedit("（");
        assert!(written.lock().unwrap().is_empty());
        let text = "()（）!@#$%^&*，。！？";
        app.handle_ime_commit(text).unwrap();
        assert_eq!(written.lock().unwrap().as_slice(), text.as_bytes());
        assert!(app.ime_preedit.is_none());
    }

    #[test]
    fn window_app_writes_ime_commit_text_when_ime_is_enabled() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.handle_ime_commit("かな").unwrap();

        assert_eq!(written.lock().unwrap().as_slice(), "かな".as_bytes());
    }

    #[test]
    fn window_app_ignores_ime_commit_text_when_ime_is_disabled() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            use_ime: Some(false),
            ..NativeConfigSnapshot::default()
        });
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.handle_ime_commit("かな").unwrap();

        assert!(written.lock().unwrap().is_empty());
    }

    #[test]
    fn window_app_renders_builtin_ime_preedit_at_cursor() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.feed_pty_output(b"ab");
        app.snapshot = TerminalRenderSnapshot::from_terminal(app.runtime.terminal());

        app.handle_ime_preedit("kan");

        let snapshot = app.render_snapshot();
        assert_eq!(snapshot_row_text(&snapshot, TAB_BAR_ROWS, 6), "abkan ");
    }

    #[test]
    fn window_app_ime_preedit_preserves_graphemes_and_authoritative_cell_spans() {
        let mut app = NativeWindowApp::new(None);
        app.handle_ime_preedit("中e\u{301}👨‍👩‍👧‍👦");

        let snapshot = app.render_snapshot();
        let row = TAB_BAR_ROWS;
        let leaders = snapshot
            .cells()
            .iter()
            .filter(|cell| cell.row == row && !cell.continuation)
            .collect::<Vec<_>>();
        assert_eq!(
            leaders
                .iter()
                .map(|cell| (cell.text.as_ref(), cell.column, cell.columns))
                .collect::<Vec<_>>(),
            [("中", 0, 2), ("e\u{301}", 2, 1), ("👨‍👩‍👧‍👦", 3, 2)]
        );
        assert_eq!(
            snapshot
                .cells()
                .iter()
                .filter(|cell| cell.row == row && cell.continuation)
                .map(|cell| cell.column)
                .collect::<Vec<_>>(),
            [1, 4]
        );
    }

    #[test]
    fn window_app_uses_wezterm_compose_cursor_color_for_builtin_ime_preedit() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              compose_cursor = '#010203',
            }

            return config
            "##,
        )
        .expect("expected WezTerm colors.compose_cursor config");
        app.set_config_overrides(overrides);

        app.handle_ime_preedit("kan");

        let snapshot = app.render_snapshot();
        assert_eq!(snapshot.cursor_color(), Some(Color::Rgb(1, 2, 3)));
    }

    #[test]
    fn window_app_does_not_render_system_ime_preedit() {
        let mut app = NativeWindowApp::new(None);
        app.runtime.feed_pty_output(b"ab");
        app.snapshot = TerminalRenderSnapshot::from_terminal(app.runtime.terminal());
        app.set_config_overrides(native_config_snapshot! {
            ime_preedit_rendering: Some(NativeImePreeditRendering::System),
            ..NativeConfigSnapshot::default()
        });

        app.handle_ime_preedit("kan");

        let snapshot = app.render_snapshot();
        assert_eq!(snapshot_row_text(&snapshot, TAB_BAR_ROWS, 6), "ab    ");
    }

    #[test]
    fn window_app_clears_ime_preedit_on_commit() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.runtime.feed_pty_output(b"ab");
        app.snapshot = TerminalRenderSnapshot::from_terminal(app.runtime.terminal());
        app.handle_ime_preedit("kan");

        app.handle_ime_commit("か").unwrap();

        let snapshot = app.render_snapshot();
        assert_eq!(snapshot_row_text(&snapshot, TAB_BAR_ROWS, 6), "ab    ");
        assert_eq!(written.lock().unwrap().as_slice(), "か".as_bytes());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_mouse_focus_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.pane_focus_follows_mouse = true
            config.swallow_mouse_click_on_pane_focus = true
            config.swallow_mouse_click_on_window_focus = true
            config.bypass_mouse_reporting_modifiers = 'ALT|SHIFT'

            return config
            "#,
        )
        .expect("expected WezTerm mouse/focus config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert!(effective.pane_focus_follows_mouse);
        assert!(effective.swallow_mouse_click_on_pane_focus);
        assert!(effective.swallow_mouse_click_on_window_focus);
        assert_eq!(
            effective.bypass_mouse_reporting_modifiers,
            ModifiersState::ALT | ModifiersState::SHIFT
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_tab_bar_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.tab_max_width = 32
            config.enable_tab_bar = false
            config.hide_tab_bar_if_only_one_tab = true
            config.unzoom_on_switch_pane = false
            config.tab_bar_at_bottom = true
            config.tab_and_split_indices_are_zero_based = true
            config.mouse_wheel_scrolls_tabs = false
            config.switch_to_last_active_tab_when_closing_tab = true
            config.quit_when_all_windows_are_closed = false
            config.show_close_tab_button_in_tabs = false
            config.show_new_tab_button_in_tab_bar = false
            config.show_tab_index_in_tab_bar = false
            config.show_tabs_in_tab_bar = false

            return config
            "#,
        )
        .expect("expected WezTerm tab bar config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.tab_max_width, 32);
        assert!(!effective.enable_tab_bar);
        assert!(effective.hide_tab_bar_if_only_one_tab);
        assert!(!effective.unzoom_on_switch_pane);
        assert!(effective.tab_bar_at_bottom);
        assert!(effective.tab_and_split_indices_are_zero_based);
        assert!(!effective.mouse_wheel_scrolls_tabs);
        assert!(effective.switch_to_last_active_tab_when_closing_tab);
        assert!(!effective.quit_when_all_windows_are_closed);
        assert!(!effective.show_close_tab_button_in_tabs);
        assert!(!effective.show_new_tab_button_in_tab_bar);
        assert!(!effective.show_tab_index_in_tab_bar);
        assert!(!effective.show_tabs_in_tab_bar);
        assert!(!app.tab_bar_is_visible());
    }

    #[test]
    fn window_app_reports_default_wezterm_fancy_tab_bar_config() {
        let app = NativeWindowApp::new(None);

        assert!(app.native_effective_config().use_fancy_tab_bar);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_fancy_tab_bar_override() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.use_fancy_tab_bar = false

            return config
            "#,
        )
        .expect("expected WezTerm fancy tab bar config");
        app.set_config_overrides(overrides);

        assert!(!app.native_effective_config().use_fancy_tab_bar);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_window_content_alignment() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_content_alignment = {
              horizontal = 'Center',
              vertical = 'Bottom',
            }

            return config
            "#,
        )
        .expect("expected WezTerm window content alignment config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().window_content_alignment,
            NativeWindowContentAlignment {
                horizontal: NativeHorizontalContentAlignment::Center,
                vertical: NativeVerticalContentAlignment::Bottom,
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_window_content_alignment_static_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local project_alignment = {
              horizontal = 'Center',
              vertical = 'Bottom',
            }

            config.term = 'xterm-256color'
            config.window_content_alignment = project_alignment

            return config
            "#,
        )
        .expect("expected WezTerm window content alignment static variable config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().window_content_alignment,
            NativeWindowContentAlignment {
                horizontal: NativeHorizontalContentAlignment::Center,
                vertical: NativeVerticalContentAlignment::Bottom,
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_window_content_alignment_static_field_variables() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local horizontal_alignment = 'Right'
            local vertical_alignment = 'Center'

            config.window_content_alignment = {
              horizontal = horizontal_alignment,
              vertical = vertical_alignment,
            }

            return config
            "#,
        )
        .expect("expected WezTerm window content alignment static field variable config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().window_content_alignment,
            NativeWindowContentAlignment {
                horizontal: NativeHorizontalContentAlignment::Right,
                vertical: NativeVerticalContentAlignment::Center,
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_window_content_alignment_static_field_names() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local horizontal_field = 'horizontal'
            local vertical_field = 'vertical'

            config.window_content_alignment = {
              [horizontal_field] = 'Right',
              [vertical_field] = 'Bottom',
            }

            return config
            "#,
        )
        .expect("expected WezTerm window content alignment static field-name config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().window_content_alignment,
            NativeWindowContentAlignment {
                horizontal: NativeHorizontalContentAlignment::Right,
                vertical: NativeVerticalContentAlignment::Bottom,
            }
        );
    }

    #[test]
    fn window_app_parses_static_key_window_content_alignment_field_assignments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local alignment_field = 'window_content_alignment'
            local vertical_alignment = 'Bottom'

            config[alignment_field] = {}
            config[alignment_field].horizontal = 'Right'
            config[alignment_field]['vertical'] = vertical_alignment

            return config
            "#,
        )
        .expect("expected WezTerm static field-name window content alignment config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().window_content_alignment,
            NativeWindowContentAlignment {
                horizontal: NativeHorizontalContentAlignment::Right,
                vertical: NativeVerticalContentAlignment::Bottom,
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_diagnostics_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.automatically_reload_config = false
            config.use_resize_increments = true
            config.debug_key_events = true
            config.log_unknown_escape_sequences = true
            config.warn_about_missing_glyphs = false

            return config
            "#,
        )
        .expect("expected WezTerm diagnostics config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert!(!effective.automatically_reload_config);
        assert!(effective.use_resize_increments);
        assert!(effective.debug_key_events);
        assert!(effective.log_unknown_escape_sequences);
        assert!(!effective.warn_about_missing_glyphs);

        app.handle_keyboard_input_event(
            &Key::Character("x".into()),
            PhysicalKey::Code(WinitKeyCode::KeyX),
            Some("x"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        let logs = app.debug_key_event_logs_for_test();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].contains("INFO key_event"));
        assert!(logs[0].contains("key: Character(\"x\")"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_diagnostics_static_bool_variables() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local reload_config = false
            local resize_increments = true
            local debug_keys = true
            local unknown_escapes = true
            local missing_glyphs = false

            config.automatically_reload_config = reload_config
            config.use_resize_increments = resize_increments
            config.debug_key_events = debug_keys
            config.log_unknown_escape_sequences = unknown_escapes
            config.warn_about_missing_glyphs = missing_glyphs

            return config
            "#,
        )
        .expect("expected WezTerm diagnostics static bool variable config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert!(!effective.automatically_reload_config);
        assert!(effective.use_resize_increments);
        assert!(effective.debug_key_events);
        assert!(effective.log_unknown_escape_sequences);
        assert!(!effective.warn_about_missing_glyphs);
    }

    #[test]
    fn window_app_reports_default_wezterm_update_check_config() {
        let app = NativeWindowApp::new(None);
        let effective = app.native_effective_config();

        assert!(effective.check_for_updates);
        assert_eq!(effective.check_for_updates_interval_seconds, 86_400);
        assert!(!effective.show_update_window);
        assert!(!effective.native_macos_fullscreen_mode);
        assert!(!effective.macos_fullscreen_extend_behind_notch);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_update_check_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.check_for_updates = false
            config.check_for_updates_interval_seconds = 43200
            config.show_update_window = true
            config.macos_fullscreen_extend_behind_notch = true

            return config
            "#,
        )
        .expect("expected WezTerm update check config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert!(!effective.check_for_updates);
        assert_eq!(effective.check_for_updates_interval_seconds, 43_200);
        assert!(effective.show_update_window);
        assert!(effective.macos_fullscreen_extend_behind_notch);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_native_macos_fullscreen_mode() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.native_macos_fullscreen_mode = true

            return config
            "#,
        )
        .expect("expected WezTerm native_macos_fullscreen_mode config");
        app.set_config_overrides(overrides);

        assert!(app.native_effective_config().native_macos_fullscreen_mode);
    }

    #[test]
    fn window_app_reports_default_wezterm_frame_rate_config() {
        let app = NativeWindowApp::new(None);
        let effective = app.native_effective_config();

        assert_eq!(effective.max_fps, 60);
        assert_eq!(effective.animation_fps, 10);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_frame_rate_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.max_fps = 144
            config.animation_fps = 24

            return config
            "#,
        )
        .expect("expected WezTerm frame-rate config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.max_fps, 144);
        assert_eq!(effective.animation_fps, 24);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_frame_rate_static_number_variables() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local redraw_rate = 144
            local animation_rate = 24

            config.max_fps = redraw_rate
            config.animation_fps = animation_rate

            return config
            "#,
        )
        .expect("expected WezTerm frame-rate static number variable config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.max_fps, 144);
        assert_eq!(effective.animation_fps, 24);
    }

    #[test]
    fn window_app_limits_redraw_requests_to_configured_max_fps() {
        let mut app = NativeWindowApp::new(None);
        let start = Instant::now();

        assert!(app.should_request_redraw_at(start));
        assert!(!app.should_request_redraw_at(start + Duration::from_millis(15)));
        assert!(app.should_request_redraw_at(start + Duration::from_millis(17)));

        app.set_config_overrides(native_config_snapshot! {
            max_fps: Some(144),
            ..NativeConfigSnapshot::default()
        });
        let next = start + Duration::from_millis(20);

        assert!(app.should_request_redraw_at(next));
        assert!(!app.should_request_redraw_at(next + Duration::from_millis(6)));
        assert!(app.should_request_redraw_at(next + Duration::from_millis(7)));
    }

    #[test]
    fn window_app_requests_throttled_redraws_until_the_frame_limit_is_reached() {
        let mut app = NativeWindowApp::new(Some(10));
        assert!(app.frame_limit_redraw_pending());

        app.rendered_frames = 9;
        assert!(app.frame_limit_redraw_pending());

        app.rendered_frames = 10;
        assert!(!app.frame_limit_redraw_pending());
        assert!(!NativeWindowApp::new(None).frame_limit_redraw_pending());
    }

    #[test]
    fn window_app_limits_animation_redraw_requests_to_configured_animation_fps() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            max_fps: Some(240),
            ..NativeConfigSnapshot::default()
        });
        let start = Instant::now();
        app.visual_bell = NativeVisualBell {
            fade_out_duration_ms: 1_000,
            ..NativeVisualBell::default()
        };
        let active_pane_id = app.app_shell.active_pane_id();
        app.visual_bell_started_at
            .insert(active_pane_id, start);

        assert!(app.should_request_animation_redraw_at(start));
        assert!(!app.should_request_animation_redraw_at(start + Duration::from_millis(99)));
        assert!(app.should_request_animation_redraw_at(start + Duration::from_millis(100)));

        app.set_config_overrides(native_config_snapshot! {
            animation_fps: Some(24),
            max_fps: Some(240),
            ..NativeConfigSnapshot::default()
        });
        let next = start + Duration::from_millis(150);
        app.visual_bell = NativeVisualBell {
            fade_out_duration_ms: 1_000,
            ..NativeVisualBell::default()
        };
        let active_pane_id = app.app_shell.active_pane_id();
        app.visual_bell_started_at
            .insert(active_pane_id, next);

        assert!(app.should_request_animation_redraw_at(next));
        assert!(!app.should_request_animation_redraw_at(next + Duration::from_millis(41)));
        assert!(app.should_request_animation_redraw_at(next + Duration::from_millis(42)));
    }

    #[test]
    fn window_app_caps_animation_redraw_requests_by_max_fps() {
        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(native_config_snapshot! {
            animation_fps: Some(240),
            max_fps: Some(60),
            ..NativeConfigSnapshot::default()
        });
        let start = Instant::now();
        app.visual_bell = NativeVisualBell {
            fade_out_duration_ms: 1_000,
            ..NativeVisualBell::default()
        };
        let active_pane_id = app.app_shell.active_pane_id();
        app.visual_bell_started_at
            .insert(active_pane_id, start);

        assert!(app.should_request_animation_redraw_at(start));
        assert!(!app.should_request_animation_redraw_at(start + Duration::from_millis(5)));
        assert!(!app.should_request_animation_redraw_at(start + Duration::from_millis(16)));
        assert!(app.should_request_animation_redraw_at(start + Duration::from_millis(17)));
    }

    #[test]
    fn window_app_reports_default_wezterm_render_backend_config() {
        let app = NativeWindowApp::new(None);
        let effective = app.native_effective_config();

        assert_eq!(effective.front_end, NativeRenderFrontEnd::OpenGl);
        assert_eq!(
            effective.webgpu_power_preference,
            NativeWebGpuPowerPreference::LowPower
        );
        assert!(!effective.webgpu_force_fallback_adapter);
        assert_eq!(effective.webgpu_preferred_adapter, None);
        assert!(effective.prefer_egl);
        assert!(effective.enable_wayland);
        assert!(!effective.enable_zwlr_output_manager);
        assert!(!effective.use_box_model_render);
        assert!(!effective.experimental_pixel_positioning);
    }

    #[test]
    fn window_app_reports_default_wezterm_render_cache_config() {
        let app = NativeWindowApp::new(None);
        let effective = app.native_effective_config();

        assert_eq!(effective.shape_cache_size, 1_024);
        assert_eq!(effective.line_state_cache_size, 1_024);
        assert_eq!(effective.line_quad_cache_size, 1_024);
        assert_eq!(effective.line_to_ele_shape_cache_size, 1_024);
        assert_eq!(effective.glyph_cache_image_cache_size, 256);
    }

    #[test]
    fn window_app_reports_default_wezterm_platform_backdrop_config() {
        let effective = NativeWindowApp::new(None).native_effective_config();

        assert!(!effective.kde_window_background_blur);
        assert_eq!(effective.macos_window_background_blur, 0);
        assert_eq!(
            effective.win32_system_backdrop,
            NativeWin32SystemBackdrop::Auto
        );
        assert_eq!(effective.win32_acrylic_accent_color, None);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_platform_backdrop_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.kde_window_background_blur = true
            config.macos_window_background_blur = 20
            config.win32_system_backdrop = 'Mica'
            config.win32_acrylic_accent_color = '#112233'

            return config
            "#,
        )
        .expect("expected WezTerm platform backdrop config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
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
    }

    #[test]
    fn window_app_parses_wezterm_color_parse_static_alias_for_lua_win32_acrylic_accent_color() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local parse_color = wezterm.color.parse

            config.win32_acrylic_accent_color = parse_color('rgba(17,34,51,0.5)')

            return config
            "##,
        )
        .expect("expected WezTerm color.parse Win32 acrylic accent color config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().win32_acrylic_accent_color,
            Some(Color::Rgb(17, 34, 51))
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_render_backend_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.front_end = 'WebGpu'
            config.webgpu_power_preference = 'HighPerformance'
            config.webgpu_force_fallback_adapter = true
            config.webgpu_preferred_adapter = {
              backend = 'Vulkan',
              device = 29730,
              device_type = 'DiscreteGpu',
              driver = 'radv',
              driver_info = 'Mesa 22.3.4',
              name = 'AMD Radeon Pro W6400 (RADV NAVI24)',
              vendor = 4098,
            }
            config.prefer_egl = false
            config.enable_wayland = false
            config.enable_zwlr_output_manager = true
            config.use_box_model_render = true
            config.experimental_pixel_positioning = true

            return config
            "#,
        )
        .expect("expected WezTerm render-backend config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.front_end, NativeRenderFrontEnd::WebGpu);
        assert_eq!(
            effective.webgpu_power_preference,
            NativeWebGpuPowerPreference::HighPerformance
        );
        assert!(effective.webgpu_force_fallback_adapter);
        assert_eq!(
            effective.webgpu_preferred_adapter,
            Some(NativeWebGpuPreferredAdapter {
                backend: Some("Vulkan".to_owned()),
                device: Some(29_730),
                device_type: Some("DiscreteGpu".to_owned()),
                driver: Some("radv".to_owned()),
                driver_info: Some("Mesa 22.3.4".to_owned()),
                name: Some("AMD Radeon Pro W6400 (RADV NAVI24)".to_owned()),
                vendor: Some(4_098),
            })
        );
        assert!(!effective.prefer_egl);
        assert!(!effective.enable_wayland);
        assert!(effective.enable_zwlr_output_manager);
        assert!(effective.use_box_model_render);
        assert!(effective.experimental_pixel_positioning);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_render_cache_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.shape_cache_size = 2048
            config.line_state_cache_size = 512
            config.line_quad_cache_size = 768
            config.line_to_ele_shape_cache_size = 1536
            config.glyph_cache_image_cache_size = 128

            return config
            "#,
        )
        .expect("expected WezTerm render cache config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.shape_cache_size, 2_048);
        assert_eq!(effective.line_state_cache_size, 512);
        assert_eq!(effective.line_quad_cache_size, 768);
        assert_eq!(effective.line_to_ele_shape_cache_size, 1_536);
        assert_eq!(effective.glyph_cache_image_cache_size, 128);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_webgpu_preferred_adapter_static_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local preferred_adapter = {
              backend = 'Vulkan',
              device = 29730,
              device_type = 'DiscreteGpu',
              driver = 'radv',
              driver_info = 'Mesa 22.3.4',
              name = 'AMD Radeon Pro W6400 (RADV NAVI24)',
              vendor = 4098,
            }

            config.term = 'xterm-256color'
            config.webgpu_preferred_adapter = preferred_adapter

            return config
            "#,
        )
        .expect("expected WezTerm WebGPU preferred adapter static variable config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().webgpu_preferred_adapter,
            Some(NativeWebGpuPreferredAdapter {
                backend: Some("Vulkan".to_owned()),
                device: Some(29_730),
                device_type: Some("DiscreteGpu".to_owned()),
                driver: Some("radv".to_owned()),
                driver_info: Some("Mesa 22.3.4".to_owned()),
                name: Some("AMD Radeon Pro W6400 (RADV NAVI24)".to_owned()),
                vendor: Some(4_098),
            })
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_webgpu_preferred_adapter_static_field_variables() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local adapter_backend = 'Vulkan'
            local adapter_device = 29730
            local adapter_device_type = 'DiscreteGpu'
            local adapter_driver = 'radv'
            local adapter_driver_info = 'Mesa 22.3.4'
            local adapter_name = 'AMD Radeon Pro W6400 (RADV NAVI24)'
            local adapter_vendor = 4098

            config.webgpu_preferred_adapter = {
              backend = adapter_backend,
              device = adapter_device,
              device_type = adapter_device_type,
              driver = adapter_driver,
              driver_info = adapter_driver_info,
              name = adapter_name,
              vendor = adapter_vendor,
            }

            return config
            "#,
        )
        .expect("expected WezTerm WebGPU preferred adapter static field variable config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().webgpu_preferred_adapter,
            Some(NativeWebGpuPreferredAdapter {
                backend: Some("Vulkan".to_owned()),
                device: Some(29_730),
                device_type: Some("DiscreteGpu".to_owned()),
                driver: Some("radv".to_owned()),
                driver_info: Some("Mesa 22.3.4".to_owned()),
                name: Some("AMD Radeon Pro W6400 (RADV NAVI24)".to_owned()),
                vendor: Some(4_098),
            })
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_webgpu_preferred_adapter_static_field_names() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local backend_field = 'backend'
            local device_field = 'device'
            local device_type_field = 'device_type'
            local driver_field = 'driver'
            local driver_info_field = 'driver_info'
            local name_field = 'name'
            local vendor_field = 'vendor'
            local adapter_device = 29730
            local adapter_vendor = 4098

            config.webgpu_preferred_adapter = {
              [backend_field] = 'Vulkan',
              [device_field] = adapter_device,
              [device_type_field] = 'DiscreteGpu',
              [driver_field] = 'radv',
              [driver_info_field] = 'Mesa 22.3.4',
              [name_field] = 'AMD Radeon Pro W6400 (RADV NAVI24)',
              [vendor_field] = adapter_vendor,
            }

            return config
            "#,
        )
        .expect("expected WezTerm WebGPU preferred adapter static field-name config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().webgpu_preferred_adapter,
            Some(NativeWebGpuPreferredAdapter {
                backend: Some("Vulkan".to_owned()),
                device: Some(29_730),
                device_type: Some("DiscreteGpu".to_owned()),
                driver: Some("radv".to_owned()),
                driver_info: Some("Mesa 22.3.4".to_owned()),
                name: Some("AMD Radeon Pro W6400 (RADV NAVI24)".to_owned()),
                vendor: Some(4_098),
            })
        );
    }

    #[test]
    fn window_app_parses_static_key_webgpu_preferred_adapter_field_assignments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local adapter_field = 'webgpu_preferred_adapter'
            local adapter_device = 29730
            local adapter_driver_info = 'Mesa 22.3.4'
            local adapter_name = 'AMD Radeon Pro W6400 (RADV NAVI24)'
            local adapter_vendor = 4098

            config.front_end = 'WebGpu'
            config[adapter_field] = {}
            config[adapter_field].backend = 'Vulkan'
            config[adapter_field].device = adapter_device
            config[adapter_field].device_type = 'DiscreteGpu'
            config[adapter_field].driver = 'radv'
            config[adapter_field].driver_info = adapter_driver_info
            config[adapter_field].name = adapter_name
            config[adapter_field]['vendor'] = adapter_vendor

            return config
            "#,
        )
        .expect("expected WezTerm static field-name WebGPU preferred adapter config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.front_end, NativeRenderFrontEnd::WebGpu);
        assert_eq!(
            effective.webgpu_preferred_adapter,
            Some(NativeWebGpuPreferredAdapter {
                backend: Some("Vulkan".to_owned()),
                device: Some(29_730),
                device_type: Some("DiscreteGpu".to_owned()),
                driver: Some("radv".to_owned()),
                driver_info: Some("Mesa 22.3.4".to_owned()),
                name: Some("AMD Radeon Pro W6400 (RADV NAVI24)".to_owned()),
                vendor: Some(4_098),
            })
        );
    }

    #[test]
    fn window_app_reports_default_wezterm_overlay_config() {
        let app = NativeWindowApp::new(None);
        let effective = app.native_effective_config();

        assert_eq!(
            effective.command_palette_font_size,
            NativeFontSize::from_millipoints(14_000)
        );
        assert_eq!(
            effective.char_select_font_size,
            NativeFontSize::from_millipoints(18_000)
        );
        assert_eq!(
            effective.pane_select_font_size,
            NativeFontSize::from_millipoints(36_000)
        );
        assert_eq!(
            effective.command_palette_fg_color,
            Some(Color::Rgb(0xd8, 0xe2, 0xf0))
        );
        assert_eq!(
            effective.command_palette_bg_color,
            Some(Color::Rgb(0x10, 0x18, 0x27))
        );
        assert_eq!(
            effective.char_select_fg_color,
            Some(Color::Rgb(0xd8, 0xe2, 0xf0))
        );
        assert_eq!(
            effective.char_select_bg_color,
            Some(Color::Rgb(0x10, 0x18, 0x27))
        );
        assert_eq!(
            effective.pane_select_fg_color,
            Some(Color::Rgb(0xd8, 0xe2, 0xf0))
        );
        assert_eq!(
            effective.pane_select_bg_color,
            Some(Color::Rgba(0x0b, 0x12, 0x20, 0xe6))
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_overlay_font_sizes() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.command_palette_font_size = 15.5
            config.char_select_font_size = 16.25
            config.pane_select_font_size = 36.5

            return config
            "#,
        )
        .expect("expected WezTerm overlay font-size config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
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
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_pane_select_overlay_colors() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.pane_select_bg_color = 'rgba(17,34,51,0.5)'
            config.pane_select_fg_color = '#445566'

            return config
            "#,
        )
        .expect("expected WezTerm pane-select overlay color config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.pane_select_bg_color,
            Some(Color::Rgba(0x11, 0x22, 0x33, 127))
        );
        assert_eq!(
            effective.pane_select_fg_color,
            Some(Color::Rgb(0x44, 0x55, 0x66))
        );
    }

    #[test]
    fn window_app_parses_wezterm_color_parse_static_alias_for_lua_selector_overlay_colors() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local parse_color = wezterm.color.parse

            config.command_palette_bg_color = parse_color('#010203')
            config.command_palette_fg_color = parse_color('#040506')
            config.char_select_bg_color = parse_color('#070809')
            config.char_select_fg_color = parse_color('#0a0b0c')
            config.pane_select_bg_color = parse_color('rgba(13,14,15,0.5)')
            config.pane_select_fg_color = parse_color('#101112')

            return config
            "##,
        )
        .expect("expected WezTerm color.parse selector overlay color config");
        app.set_config_overrides(overrides);

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
        assert_eq!(
            effective.pane_select_bg_color,
            Some(Color::Rgba(13, 14, 15, 127))
        );
        assert_eq!(effective.pane_select_fg_color, Some(Color::Rgb(16, 17, 18)));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_overlay_fonts() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.command_palette_font = wezterm.font_with_fallback {
              { family = 'Iosevka Term', weight = 'Bold' },
              'Noto Color Emoji',
            }
            config.char_select_font = wezterm.font {
              family = 'Fira Code',
              italic = true,
            }
            config.pane_select_font = wezterm.font 'JetBrains Mono'

            return config
            "#,
        )
        .expect("expected WezTerm overlay font config");
        app.set_config_overrides(overrides);

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
    }

    #[test]
    fn window_app_defaults_wezterm_lua_config_overlay_fonts_to_window_frame_font() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_frame = {
              font = wezterm.font 'Roboto Mono',
            }

            return config
            "#,
        )
        .expect("expected WezTerm window_frame font config");
        app.set_config_overrides(overrides);

        let expected = Some(super::NativeFontConfig {
            families: vec!["Roboto Mono".to_owned()],
            attributes: NativeFontAttributes::default(),
        });
        let effective = app.native_effective_config();
        assert_eq!(effective.command_palette_font, expected);
        assert_eq!(effective.char_select_font, expected);
        assert_eq!(effective.pane_select_font, expected);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_palette_and_quick_select_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.status_update_interval = 250
            config.command_palette_rows = 3
            config.launcher_alphabet = 'ab'
            config.quick_select_alphabet = 'xy'
            config.quick_select_patterns = { 'ticket-[0-9]+', 'bug-[A-Z]+' }
            config.disable_default_quick_select_patterns = true
            config.quick_select_remove_styling = true
            config.selection_word_boundary = ' :'

            return config
            "#,
        )
        .expect("expected WezTerm palette/quick-select config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.status_update_interval, 250);
        assert_eq!(effective.status_update_interval_ms, 250);
        assert_eq!(effective.command_palette_rows, Some(3));
        assert_eq!(effective.launcher_alphabet, "ab");
        assert_eq!(effective.quick_select_alphabet, "xy");
        assert_eq!(
            effective.quick_select_patterns,
            vec!["ticket-[0-9]+".to_owned(), "bug-[A-Z]+".to_owned()]
        );
        assert!(effective.disable_default_quick_select_patterns);
        assert!(effective.quick_select_remove_styling);
        assert_eq!(effective.selection_word_boundary, " :");
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_quick_select_patterns_static_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local patterns = { 'ticket-[0-9]+', 'bug-[A-Z]+' }

            config.quick_select_patterns = patterns
            config.disable_default_quick_select_patterns = true

            return config
            "#,
        )
        .expect("expected WezTerm quick-select patterns table variable config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.quick_select_patterns,
            vec!["ticket-[0-9]+".to_owned(), "bug-[A-Z]+".to_owned()]
        );
        assert!(effective.disable_default_quick_select_patterns);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_quick_select_patterns_table_insert() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.quick_select_patterns = {}
            table.insert(config.quick_select_patterns, 'ticket-[0-9]+')
            table.insert(config.quick_select_patterns, 'bug-[A-Z]+')
            config.disable_default_quick_select_patterns = true

            return config
            "#,
        )
        .expect("expected WezTerm quick-select patterns table insert config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.quick_select_patterns,
            vec!["ticket-[0-9]+".to_owned(), "bug-[A-Z]+".to_owned()]
        );
        assert!(effective.disable_default_quick_select_patterns);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_hyperlink_rules() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local ticket_regex = [[\bT(\d+)\b]]
            local ticket_format = 'https://tickets.example/$1'
            local digit_capture = 1
            local email_rule = {
              regex = [[\bops@[\w-]+(\.[\w-]+)+\b]],
              format = 'mailto:$0',
            }

            config.hyperlink_rules = {
              {
                regex = ticket_regex,
                format = ticket_format,
                highlight = digit_capture,
              },
            }
            table.insert(config.hyperlink_rules, email_rule)

            return config
            "#,
        )
        .expect("expected WezTerm hyperlink_rules config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.hyperlink_rules,
            vec![
                NativeHyperlinkRule {
                    regex: r"\bT(\d+)\b".to_owned(),
                    format: "https://tickets.example/$1".to_owned(),
                    highlight: 1,
                },
                NativeHyperlinkRule {
                    regex: r"\bops@[\w-]+(\.[\w-]+)+\b".to_owned(),
                    format: "mailto:$0".to_owned(),
                    highlight: 0,
                },
            ]
        );
    }

    #[test]
    fn window_app_extends_default_hyperlink_rules_from_wezterm_lua_config() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.hyperlink_rules = wezterm.default_hyperlink_rules()
            table.insert(config.hyperlink_rules, {
              regex = [[\b[tT](\d+)\b]],
              format = 'https://tickets.example/$1',
              highlight = 1,
            })

            return config
            "#,
        )
        .expect("expected WezTerm default hyperlink_rules extension config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        let defaults = default_hyperlink_rules();
        assert_eq!(effective.hyperlink_rules.len(), defaults.len() + 1);
        assert!(
            defaults
                .iter()
                .all(|rule| effective.hyperlink_rules.contains(rule))
        );
        assert!(effective.hyperlink_rules.iter().any(|rule| {
            rule.regex == r"\b[tT](\d+)\b"
                && rule.format == "https://tickets.example/$1"
                && rule.highlight == 1
        }));
    }

    #[test]
    fn window_app_accepts_explicit_default_hyperlink_rules_from_wezterm_lua_config() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.hyperlink_rules = wezterm.default_hyperlink_rules()

            return config
            "#,
        )
        .expect("expected explicit WezTerm default hyperlink_rules config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.hyperlink_rules, default_hyperlink_rules());
    }

    #[test]
    fn window_app_accepts_config_initializer_default_hyperlink_rules_from_wezterm_lua_config() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {
              hyperlink_rules = wezterm.default_hyperlink_rules(),
            }

            return config
            "#,
        )
        .expect("expected config initializer WezTerm default hyperlink_rules config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.hyperlink_rules, default_hyperlink_rules());
    }

    #[test]
    fn window_app_accepts_returned_static_default_hyperlink_rules_from_wezterm_lua_config() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local rules = wezterm.default_hyperlink_rules()

            return {
              hyperlink_rules = rules,
            }
            "#,
        )
        .expect("expected returned static WezTerm default hyperlink_rules config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.hyperlink_rules, default_hyperlink_rules());
    }

    #[test]
    fn window_app_accepts_returned_config_static_default_hyperlink_rules_from_wezterm_lua_config() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local rules = wezterm.default_hyperlink_rules()
            local cfg = {
              hyperlink_rules = rules,
            }

            return cfg
            "#,
        )
        .expect("expected returned config static WezTerm default hyperlink_rules config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.hyperlink_rules, default_hyperlink_rules());
    }

    #[test]
    fn window_app_extends_static_default_hyperlink_rules_from_wezterm_lua_config() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local rules = wezterm.default_hyperlink_rules()

            config.hyperlink_rules = rules
            table.insert(config.hyperlink_rules, {
              regex = [[\bBUG-(\d+)\b]],
              format = 'https://bugs.example/$1',
              highlight = 1,
            })

            return config
            "#,
        )
        .expect("expected WezTerm static default hyperlink_rules extension config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        let defaults = default_hyperlink_rules();
        assert_eq!(effective.hyperlink_rules.len(), defaults.len() + 1);
        assert_eq!(
            &effective.hyperlink_rules[..defaults.len()],
            defaults.as_slice()
        );
        assert_eq!(
            effective.hyperlink_rules.last(),
            Some(&NativeHyperlinkRule {
                regex: r"\bBUG-(\d+)\b".to_owned(),
                format: "https://bugs.example/$1".to_owned(),
                highlight: 1,
            })
        );
    }

    #[test]
    fn window_app_extends_returned_static_default_hyperlink_rules_from_wezterm_lua_config() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local rules = wezterm.default_hyperlink_rules()
            table.insert(rules, {
              regex = [[\bINC-(\d+)\b]],
              format = 'https://incidents.example/$1',
              highlight = 1,
            })

            return {
              hyperlink_rules = rules,
            }
            "#,
        )
        .expect("expected WezTerm returned static default hyperlink_rules extension config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        let defaults = default_hyperlink_rules();
        assert_eq!(effective.hyperlink_rules.len(), defaults.len() + 1);
        assert_eq!(
            &effective.hyperlink_rules[..defaults.len()],
            defaults.as_slice()
        );
        assert_eq!(
            effective.hyperlink_rules.last(),
            Some(&NativeHyperlinkRule {
                regex: r"\bINC-(\d+)\b".to_owned(),
                format: "https://incidents.example/$1".to_owned(),
                highlight: 1,
            })
        );
    }

    #[test]
    fn window_app_extends_returned_config_static_default_hyperlink_rules_from_wezterm_lua_config() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local rules = wezterm.default_hyperlink_rules()
            table.insert(rules, {
              regex = [[\bCASE-(\d+)\b]],
              format = 'https://cases.example/$1',
              highlight = 1,
            })
            local cfg = {
              hyperlink_rules = rules,
            }

            return cfg
            "#,
        )
        .expect("expected WezTerm returned config static default hyperlink_rules extension config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        let defaults = default_hyperlink_rules();
        assert_eq!(effective.hyperlink_rules.len(), defaults.len() + 1);
        assert_eq!(
            &effective.hyperlink_rules[..defaults.len()],
            defaults.as_slice()
        );
        assert_eq!(
            effective.hyperlink_rules.last(),
            Some(&NativeHyperlinkRule {
                regex: r"\bCASE-(\d+)\b".to_owned(),
                format: "https://cases.example/$1".to_owned(),
                highlight: 1,
            })
        );
    }

    #[test]
    fn window_app_preserves_positioned_default_hyperlink_rule_insert_from_wezterm_lua_config() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local rules = wezterm.default_hyperlink_rules()
            table.insert(rules, 1, {
              regex = [[\bPR-(\d+)\b]],
              format = 'https://reviews.example/$1',
              highlight = 1,
            })

            return {
              hyperlink_rules = rules,
            }
            "#,
        )
        .expect("expected WezTerm positioned default hyperlink_rules extension config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        let defaults = default_hyperlink_rules();
        assert_eq!(effective.hyperlink_rules.len(), defaults.len() + 1);
        assert_eq!(
            effective.hyperlink_rules.first(),
            Some(&NativeHyperlinkRule {
                regex: r"\bPR-(\d+)\b".to_owned(),
                format: "https://reviews.example/$1".to_owned(),
                highlight: 1,
            })
        );
        assert_eq!(&effective.hyperlink_rules[1..], defaults.as_slice());
    }

    #[test]
    fn window_app_preserves_positioned_config_default_hyperlink_rule_insert() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.hyperlink_rules = wezterm.default_hyperlink_rules()
            table.insert(config.hyperlink_rules, 1, {
              regex = [[\bOPS-(\d+)\b]],
              format = 'https://ops.example/$1',
              highlight = 1,
            })

            return config
            "#,
        )
        .expect("expected WezTerm positioned config default hyperlink_rules extension config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        let defaults = default_hyperlink_rules();
        assert_eq!(effective.hyperlink_rules.len(), defaults.len() + 1);
        assert_eq!(
            effective.hyperlink_rules.first(),
            Some(&NativeHyperlinkRule {
                regex: r"\bOPS-(\d+)\b".to_owned(),
                format: "https://ops.example/$1".to_owned(),
                highlight: 1,
            })
        );
        assert_eq!(&effective.hyperlink_rules[1..], defaults.as_slice());
    }

    #[test]
    fn window_app_replaces_indexed_config_default_hyperlink_rule() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.hyperlink_rules = wezterm.default_hyperlink_rules()
            config.hyperlink_rules[1] = {
              regex = [[\bRUN-(\d+)\b]],
              format = 'https://runs.example/$1',
              highlight = 1,
            }

            return config
            "#,
        )
        .expect("expected WezTerm indexed config default hyperlink_rules config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        let mut expected = default_hyperlink_rules();
        expected[0] = NativeHyperlinkRule {
            regex: r"\bRUN-(\d+)\b".to_owned(),
            format: "https://runs.example/$1".to_owned(),
            highlight: 1,
        };
        assert_eq!(effective.hyperlink_rules, expected);
    }

    #[test]
    fn window_app_mutates_indexed_config_default_hyperlink_rule_field() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.hyperlink_rules = wezterm.default_hyperlink_rules()
            config.hyperlink_rules[1].format = 'https://config-wrapped.example/$1'

            return config
            "#,
        )
        .expect("expected WezTerm indexed config default hyperlink_rules field mutation");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        let mut expected = default_hyperlink_rules();
        expected[0].format = "https://config-wrapped.example/$1".to_owned();
        assert_eq!(effective.hyperlink_rules, expected);
    }

    #[test]
    fn window_app_replaces_indexed_static_default_hyperlink_rule() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local rules = wezterm.default_hyperlink_rules()

            rules[1] = {
              regex = [[\bDEPLOY-(\d+)\b]],
              format = 'https://deploys.example/$1',
              highlight = 1,
            }

            return {
              hyperlink_rules = rules,
            }
            "#,
        )
        .expect("expected WezTerm indexed static default hyperlink_rules config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        let mut expected = default_hyperlink_rules();
        expected[0] = NativeHyperlinkRule {
            regex: r"\bDEPLOY-(\d+)\b".to_owned(),
            format: "https://deploys.example/$1".to_owned(),
            highlight: 1,
        };
        assert_eq!(effective.hyperlink_rules, expected);
    }

    #[test]
    fn window_app_mutates_indexed_static_default_hyperlink_rule_field() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local rules = wezterm.default_hyperlink_rules()

            rules[1].format = 'https://wrapped.example/$1'

            return {
              hyperlink_rules = rules,
            }
            "#,
        )
        .expect("expected WezTerm indexed static default hyperlink_rules field mutation");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        let mut expected = default_hyperlink_rules();
        expected[0].format = "https://wrapped.example/$1".to_owned();
        assert_eq!(effective.hyperlink_rules, expected);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_copy_mode_and_quick_select_colors() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              copy_mode_active_highlight_bg = { Color = '#010203' },
              copy_mode_active_highlight_fg = { AnsiColor = 'Black' },
              copy_mode_inactive_highlight_bg = { Color = 'peru' },
              copy_mode_inactive_highlight_fg = { AnsiColor = 'White' },
              quick_select_label_bg = { Color = '#040506' },
              quick_select_label_fg = { Color = 'silver' },
              quick_select_match_bg = { AnsiColor = 'Navy' },
              quick_select_match_fg = { Color = '#070809' },
              input_selector_label_bg = { AnsiColor = 'Black' },
              input_selector_label_fg = { Color = '#0a0b0c' },
              launcher_label_bg = { AnsiColor = 'White' },
              launcher_label_fg = { Color = '#0d0e0f' },
            }

            return config
            "#,
        )
        .expect("expected WezTerm copy-mode/quick-select color config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.copy_mode_active_highlight_bg,
            Some(NativeColorSpec::Color(Color::Rgb(1, 2, 3)))
        );
        assert_eq!(
            effective.copy_mode_active_highlight_fg,
            Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Black))
        );
        assert_eq!(
            effective.copy_mode_inactive_highlight_bg,
            Some(NativeColorSpec::Color(Color::Rgb(205, 133, 63)))
        );
        assert_eq!(
            effective.copy_mode_inactive_highlight_fg,
            Some(NativeColorSpec::AnsiColor(NativeAnsiColor::White))
        );
        assert_eq!(
            effective.quick_select_label_bg,
            Some(NativeColorSpec::Color(Color::Rgb(4, 5, 6)))
        );
        assert_eq!(
            effective.quick_select_label_fg,
            Some(NativeColorSpec::Color(Color::Rgb(192, 192, 192)))
        );
        assert_eq!(
            effective.quick_select_match_bg,
            Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Navy))
        );
        assert_eq!(
            effective.quick_select_match_fg,
            Some(NativeColorSpec::Color(Color::Rgb(7, 8, 9)))
        );
        assert_eq!(
            effective.input_selector_label_bg,
            Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Black))
        );
        assert_eq!(
            effective.input_selector_label_fg,
            Some(NativeColorSpec::Color(Color::Rgb(10, 11, 12)))
        );
        assert_eq!(
            effective.launcher_label_bg,
            Some(NativeColorSpec::AnsiColor(NativeAnsiColor::White))
        );
        assert_eq!(
            effective.launcher_label_fg,
            Some(NativeColorSpec::Color(Color::Rgb(13, 14, 15)))
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_color_spec_static_key_mutations() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local colors = {}
            local active_bg = 'copy_mode_active_highlight_bg'
            local active_fg = 'copy_mode_active_highlight_fg'
            local quick_match_bg = 'quick_select_match_bg'
            local input_label_fg = 'input_selector_label_fg'
            local launcher_label_bg = 'launcher_label_bg'

            colors[active_bg] = { Color = '#111213' }
            colors[active_fg] = { AnsiColor = 'Black' }
            colors[quick_match_bg] = { AnsiColor = 'Navy' }
            colors[input_label_fg] = { Color = '#141516' }
            colors[launcher_label_bg] = { Color = '#171819' }
            config.colors = colors

            return config
            "#,
        )
        .expect("expected WezTerm color spec static-key mutation config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.copy_mode_active_highlight_bg,
            Some(NativeColorSpec::Color(Color::Rgb(17, 18, 19)))
        );
        assert_eq!(
            effective.copy_mode_active_highlight_fg,
            Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Black))
        );
        assert_eq!(
            effective.quick_select_match_bg,
            Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Navy))
        );
        assert_eq!(
            effective.input_selector_label_fg,
            Some(NativeColorSpec::Color(Color::Rgb(20, 21, 22)))
        );
        assert_eq!(
            effective.launcher_label_bg,
            Some(NativeColorSpec::Color(Color::Rgb(23, 24, 25)))
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_color_spec_nested_mutations() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local parse_color = wezterm.color.parse

            config.colors = {}
            config.colors.copy_mode_active_highlight_bg = {}
            config.colors.copy_mode_active_highlight_bg.Color = parse_color('#010203')
            config.colors.copy_mode_active_highlight_fg = {}
            config.colors.copy_mode_active_highlight_fg.AnsiColor = 'Black'
            config.colors.quick_select_match_bg = {}
            config.colors.quick_select_match_bg.AnsiColor = 'Navy'
            config.colors.quick_select_match_fg = {}
            config.colors.quick_select_match_fg.Color = parse_color('#040506')

            return config
            "##,
        )
        .expect("expected WezTerm nested color spec mutation config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.copy_mode_active_highlight_bg,
            Some(NativeColorSpec::Color(Color::Rgb(1, 2, 3)))
        );
        assert_eq!(
            effective.copy_mode_active_highlight_fg,
            Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Black))
        );
        assert_eq!(
            effective.quick_select_match_bg,
            Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Navy))
        );
        assert_eq!(
            effective.quick_select_match_fg,
            Some(NativeColorSpec::Color(Color::Rgb(4, 5, 6)))
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_color_spec_variable_nested_mutations() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local parse_color = wezterm.color.parse
            local colors = {}
            local active_bg = 'copy_mode_active_highlight_bg'
            local quick_match_fg = 'quick_select_match_fg'

            colors[active_bg] = {}
            colors[active_bg].Color = parse_color('#010203')
            colors.copy_mode_active_highlight_fg = {}
            colors.copy_mode_active_highlight_fg.AnsiColor = 'Black'
            colors.quick_select_match_bg = {}
            colors.quick_select_match_bg.AnsiColor = 'Navy'
            colors[quick_match_fg] = {}
            colors[quick_match_fg].Color = parse_color('#040506')
            config.colors = colors

            return config
            "##,
        )
        .expect("expected WezTerm variable nested color spec mutation config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.copy_mode_active_highlight_bg,
            Some(NativeColorSpec::Color(Color::Rgb(1, 2, 3)))
        );
        assert_eq!(
            effective.copy_mode_active_highlight_fg,
            Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Black))
        );
        assert_eq!(
            effective.quick_select_match_bg,
            Some(NativeColorSpec::AnsiColor(NativeAnsiColor::Navy))
        );
        assert_eq!(
            effective.quick_select_match_fg,
            Some(NativeColorSpec::Color(Color::Rgb(4, 5, 6)))
        );
    }

    #[test]
    fn window_app_ignores_unsupported_lua_config_colors_nested_mutations() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              foreground = '#010203',
            }
            config.colors.unhandled_nested_color.Color = not_a_static_color()

            return config
            "##,
        )
        .expect("expected unsupported nested colors mutation to be ignored");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().foreground_color,
            Color::Rgb(1, 2, 3)
        );
    }

    #[test]
    fn window_app_ignores_unsupported_lua_colors_variable_nested_mutations() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local colors = {
              foreground = '#010203',
            }

            colors.unhandled_nested_color.Color = not_a_static_color()
            config.colors = colors

            return config
            "##,
        )
        .expect("expected unsupported nested colors variable mutation to be ignored");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().foreground_color,
            Color::Rgb(1, 2, 3)
        );
    }

    #[test]
    fn window_app_reports_default_wezterm_font_rasterizer_config() {
        let app = NativeWindowApp::new(None);
        let effective = app.native_effective_config();

        assert_eq!(effective.font_antialias, NativeFontAntialias::Greyscale);
        assert_eq!(effective.font_hinting, NativeFontHinting::Full);
        assert_eq!(effective.font_rasterizer, NativeFontRasterizer::FreeType);
        assert_eq!(
            effective.font_colr_rasterizer,
            NativeFontRasterizer::Harfbuzz
        );
        assert_eq!(effective.font_shaper, NativeFontShaper::Harfbuzz);
        assert!(!effective.ignore_svg_fonts);
        assert!(!effective.sort_fallback_fonts_by_coverage);
        assert!(!effective.search_font_dirs_for_fallback);
        assert!(effective.custom_block_glyphs);
        assert!(effective.anti_alias_custom_block_glyphs);
        assert_eq!(
            effective.allow_square_glyphs_to_overflow_width,
            NativeSquareGlyphOverflow::WhenFollowedBySpace
        );
        assert_eq!(effective.freetype_load_target, NativeFreetypeTarget::Normal);
        assert_eq!(
            effective.freetype_render_target,
            NativeFreetypeTarget::Normal
        );
        assert_eq!(
            effective.freetype_load_flags,
            NativeFreetypeLoadFlags::DEFAULT
        );
        assert_eq!(effective.freetype_interpreter_version, None);
        assert_eq!(
            effective.display_pixel_geometry,
            NativeDisplayPixelGeometry::Rgb
        );

        let mut high_dpi_app = NativeWindowApp::new(None);
        high_dpi_app.window_dpi = 144;
        let effective = high_dpi_app.native_effective_config();

        assert_eq!(
            effective.freetype_load_flags,
            NativeFreetypeLoadFlags::NO_HINTING
        );
        assert!(!effective.freetype_pcf_long_family_names);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_fallback_rasterizer_flags() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.font_colr_rasterizer = 'FreeType'
            config.ignore_svg_fonts = true
            config.sort_fallback_fonts_by_coverage = true
            config.search_font_dirs_for_fallback = true

            return config
            "#,
        )
        .expect("expected WezTerm font fallback config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.font_colr_rasterizer,
            NativeFontRasterizer::FreeType
        );
        assert!(effective.ignore_svg_fonts);
        assert!(effective.sort_fallback_fonts_by_coverage);
        assert!(effective.search_font_dirs_for_fallback);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_family() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font = wezterm.font 'JetBrains Mono'

            return config
            "#,
        )
        .expect("expected WezTerm font config");
        app.set_config_overrides(overrides);

        let effective = format!("{:?}", app.native_effective_config());
        assert!(
            effective.contains("font: Some(\"JetBrains Mono\")"),
            "effective config should expose WezTerm's configured font family: {effective:?}"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_dotted_comment_call() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font = wezterm. -- primary font helper
              font('JetBrains Mono', {
                weight = 'Bold',
                style = 'Italic',
              })

            return config
            "#,
        )
        .expect("expected WezTerm font dotted comment config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font.as_deref(), Some("JetBrains Mono"));
        assert_eq!(
            effective.font_attributes,
            NativeFontAttributes {
                weight: Some("Bold".to_owned()),
                stretch: None,
                style: Some("Italic".to_owned()),
                harfbuzz_features: Vec::new(),
                assume_emoji_presentation: None,
                freetype_load_target: None,
                freetype_render_target: None,
                freetype_load_flags: None,
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_static_alias() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local font = wezterm.font

            config.font = font('JetBrains Mono', {
              weight = 'Bold',
              style = 'Italic',
            })

            return config
            "#,
        )
        .expect("expected WezTerm font alias config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font.as_deref(), Some("JetBrains Mono"));
        assert_eq!(
            effective.font_attributes,
            NativeFontAttributes {
                weight: Some("Bold".to_owned()),
                stretch: None,
                style: Some("Italic".to_owned()),
                harfbuzz_features: Vec::new(),
                assume_emoji_presentation: None,
                freetype_load_target: None,
                freetype_render_target: None,
                freetype_load_flags: None,
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_static_alias_static_key_module() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wt = require 'wezterm'
            local config = {}
            local font_key = 'font'
            local font = wt[font_key]

            config.font = font('JetBrains Mono', {
              weight = 'Bold',
              style = 'Italic',
            })

            return config
            "#,
        )
        .expect("expected WezTerm font static-key alias config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font.as_deref(), Some("JetBrains Mono"));
        assert_eq!(
            effective.font_attributes,
            NativeFontAttributes {
                weight: Some("Bold".to_owned()),
                stretch: None,
                style: Some("Italic".to_owned()),
                harfbuzz_features: Vec::new(),
                assume_emoji_presentation: None,
                freetype_load_target: None,
                freetype_render_target: None,
                freetype_load_flags: None,
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_direct_static_key_module() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wt = require 'wezterm'
            local config = {}
            local font_key = 'font'

            config.font = wt[font_key]('JetBrains Mono', {
              weight = 'Bold',
              style = 'Italic',
            })

            return config
            "#,
        )
        .expect("expected WezTerm font direct static-key module config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font.as_deref(), Some("JetBrains Mono"));
        assert_eq!(
            effective.font_attributes,
            NativeFontAttributes {
                weight: Some("Bold".to_owned()),
                stretch: None,
                style: Some("Italic".to_owned()),
                harfbuzz_features: Vec::new(),
                assume_emoji_presentation: None,
                freetype_load_target: None,
                freetype_render_target: None,
                freetype_load_flags: None,
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_static_alias_comment_call() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local font = wezterm.font

            config.font = font -- primary
              ('JetBrains Mono', {
                weight = 'Bold',
                style = 'Italic',
              })

            return config
            "#,
        )
        .expect("expected WezTerm font alias comment config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font.as_deref(), Some("JetBrains Mono"));
        assert_eq!(
            effective.font_attributes,
            NativeFontAttributes {
                weight: Some("Bold".to_owned()),
                stretch: None,
                style: Some("Italic".to_owned()),
                harfbuzz_features: Vec::new(),
                assume_emoji_presentation: None,
                freetype_load_target: None,
                freetype_render_target: None,
                freetype_load_flags: None,
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_static_alias_dotted_comment() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local font = wezterm -- font helper
              .font

            config.font = font('JetBrains Mono', {
              weight = 'Bold',
              style = 'Italic',
            })

            return config
            "#,
        )
        .expect("expected WezTerm font alias dotted-comment config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font.as_deref(), Some("JetBrains Mono"));
        assert_eq!(
            effective.font_attributes,
            NativeFontAttributes {
                weight: Some("Bold".to_owned()),
                stretch: None,
                style: Some("Italic".to_owned()),
                harfbuzz_features: Vec::new(),
                assume_emoji_presentation: None,
                freetype_load_target: None,
                freetype_render_target: None,
                freetype_load_flags: None,
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_static_value() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local app_font = wezterm.font('JetBrains Mono', {
              weight = 'DemiBold',
              style = 'Italic',
            })

            config.font = app_font

            return config
            "#,
        )
        .expect("expected WezTerm static font value config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font.as_deref(), Some("JetBrains Mono"));
        assert_eq!(
            effective.font_attributes,
            NativeFontAttributes {
                weight: Some("DemiBold".to_owned()),
                stretch: None,
                style: Some("Italic".to_owned()),
                harfbuzz_features: Vec::new(),
                assume_emoji_presentation: None,
                freetype_load_target: None,
                freetype_render_target: None,
                freetype_load_flags: None,
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_attributes() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font = wezterm.font('Iosevka Term', {
              weight = 'Bold',
              stretch = 'Expanded',
              style = 'Italic',
            })

            return config
            "#,
        )
        .expect("expected WezTerm font attributes config");
        app.set_config_overrides(overrides);

        let effective = format!("{:?}", app.native_effective_config());
        assert!(
            effective.contains("font: Some(\"Iosevka Term\")"),
            "effective config should expose WezTerm's configured font family: {effective:?}"
        );
        assert!(
            effective.contains(
                "font_attributes: NativeFontAttributes { weight: Some(\"Bold\"), stretch: Some(\"Expanded\"), style: Some(\"Italic\"), harfbuzz_features: [], assume_emoji_presentation: None, freetype_load_target: None, freetype_render_target: None, freetype_load_flags: None }"
            ),
            "effective config should expose WezTerm's configured font attributes: {effective:?}"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_legacy_bold_italic_attributes() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font = wezterm.font('Iosevka Term', {
              bold = true,
              italic = true,
            })

            return config
            "#,
        )
        .expect("expected WezTerm legacy font attributes config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font.as_deref(), Some("Iosevka Term"));
        assert_eq!(
            effective.font_attributes,
            NativeFontAttributes {
                weight: Some("Bold".to_owned()),
                stretch: None,
                style: Some("Italic".to_owned()),
                harfbuzz_features: Vec::new(),
                assume_emoji_presentation: None,
                freetype_load_target: None,
                freetype_render_target: None,
                freetype_load_flags: None,
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_table_attributes() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font = wezterm.font {
              weight = 'DemiBold',
              family = 'Iosevka Term',
              stretch = 'Condensed',
              style = 'Oblique',
            }

            return config
            "#,
        )
        .expect("expected expanded WezTerm font config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font.as_deref(), Some("Iosevka Term"));
        assert_eq!(
            effective.font_attributes,
            NativeFontAttributes {
                weight: Some("DemiBold".to_owned()),
                stretch: Some("Condensed".to_owned()),
                style: Some("Oblique".to_owned()),
                harfbuzz_features: Vec::new(),
                assume_emoji_presentation: None,
                freetype_load_target: None,
                freetype_render_target: None,
                freetype_load_flags: None,
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_harfbuzz_features_attribute() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font = wezterm.font {
              family = 'Iosevka Term',
              harfbuzz_features = { 'liga=0', 'calt=0' },
            }

            return config
            "#,
        )
        .expect("expected WezTerm font harfbuzz features config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font.as_deref(), Some("Iosevka Term"));
        assert_eq!(
            effective.font_attributes.harfbuzz_features,
            vec!["liga=0", "calt=0"]
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_emoji_presentation_attribute() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font = wezterm.font {
              family = 'Noto Color Emoji',
              assume_emoji_presentation = true,
            }

            return config
            "#,
        )
        .expect("expected WezTerm font emoji presentation config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font.as_deref(), Some("Noto Color Emoji"));
        assert_eq!(
            effective.font_attributes.assume_emoji_presentation,
            Some(true)
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_freetype_attributes() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font = wezterm.font {
              family = 'Iosevka Term',
              freetype_load_target = 'Light',
              freetype_render_target = 'HorizontalLcd',
              freetype_load_flags = 'NO_HINTING|MONOCHROME',
            }

            return config
            "#,
        )
        .expect("expected WezTerm font freetype attributes config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font.as_deref(), Some("Iosevka Term"));
        assert_eq!(
            effective.font_attributes.freetype_load_target,
            Some(NativeFreetypeTarget::Light)
        );
        assert_eq!(
            effective.font_attributes.freetype_render_target,
            Some(NativeFreetypeTarget::HorizontalLcd)
        );
        assert_eq!(
            effective.font_attributes.freetype_load_flags,
            Some(NativeFreetypeLoadFlags::NO_HINTING.union(NativeFreetypeLoadFlags::MONOCHROME))
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_attributes_with_unmodeled_fields() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font = wezterm.font('Iosevka Term', {
              weight = 'Bold',
              synthesize_styled_fonts = false,
            })

            return config
            "#,
        )
        .expect("expected WezTerm font attributes config");
        app.set_config_overrides(overrides);

        let effective = format!("{:?}", app.native_effective_config());
        assert!(
            effective.contains("weight: Some(\"Bold\")"),
            "effective config should keep recognized font attributes while ignoring unmodeled fields: {effective:?}"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_with_fallback_families() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font = wezterm.font_with_fallback { 'JetBrains Mono', 'Noto Color Emoji' }

            return config
            "#,
        )
        .expect("expected WezTerm font fallback config");
        app.set_config_overrides(overrides);

        let effective = format!("{:?}", app.native_effective_config());
        assert!(
            effective.contains("font: Some(\"JetBrains Mono\")"),
            "effective config should expose the primary WezTerm font family: {effective:?}"
        );
        assert!(
            effective.contains("font_fallbacks: [\"Noto Color Emoji\"]"),
            "effective config should expose WezTerm fallback font families: {effective:?}"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_with_fallback_dotted_comment() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font = wezterm. -- fallback helper
              font_with_fallback { 'JetBrains Mono', 'Noto Color Emoji' }

            return config
            "#,
        )
        .expect("expected WezTerm font_with_fallback dotted comment config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font.as_deref(), Some("JetBrains Mono"));
        assert_eq!(effective.font_fallbacks, vec!["Noto Color Emoji"]);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_with_fallback_static_alias() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local font_with_fallback = wezterm.font_with_fallback

            config.font = font_with_fallback {
              'JetBrains Mono',
              'Noto Color Emoji',
            }

            return config
            "#,
        )
        .expect("expected WezTerm font_with_fallback alias config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font.as_deref(), Some("JetBrains Mono"));
        assert_eq!(effective.font_fallbacks, vec!["Noto Color Emoji"]);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_with_fallback_static_alias_static_key_module() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wt = require 'wezterm'
            local config = {}
            local fallback_key = 'font_with_fallback'
            local font_with_fallback = wt[fallback_key]

            config.font = font_with_fallback {
              'JetBrains Mono',
              'Noto Color Emoji',
            }

            return config
            "#,
        )
        .expect("expected WezTerm font_with_fallback static-key alias config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font.as_deref(), Some("JetBrains Mono"));
        assert_eq!(effective.font_fallbacks, vec!["Noto Color Emoji"]);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_with_fallback_direct_static_key_module() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wt = require 'wezterm'
            local config = {}
            local fallback_key = 'font_with_fallback'

            config.font = wt[fallback_key] {
              'JetBrains Mono',
              'Noto Color Emoji',
            }

            return config
            "#,
        )
        .expect("expected WezTerm font_with_fallback direct static-key module config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font.as_deref(), Some("JetBrains Mono"));
        assert_eq!(effective.font_fallbacks, vec!["Noto Color Emoji"]);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_with_fallback_family_tables() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font = wezterm.font_with_fallback {
              { family = 'JetBrains Mono', weight = 'Medium' },
              { family = 'Terminus', weight = 'Bold' },
              'Noto Color Emoji',
            }

            return config
            "#,
        )
        .expect("expected WezTerm font fallback config");
        app.set_config_overrides(overrides);

        let effective = format!("{:?}", app.native_effective_config());
        assert!(
            effective.contains("font: Some(\"JetBrains Mono\")"),
            "effective config should expose the primary expanded WezTerm font family: {effective:?}"
        );
        assert!(
            effective.contains("font_fallbacks: [\"Terminus\", \"Noto Color Emoji\"]"),
            "effective config should expose expanded WezTerm fallback font families: {effective:?}"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_with_fallback_attributes() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font = wezterm.font_with_fallback(
              { 'JetBrains Mono', 'Noto Color Emoji' },
              {
                weight = 'DemiBold',
                stretch = 'Condensed',
                style = 'Italic',
              }
            )

            return config
            "#,
        )
        .expect("expected WezTerm font fallback attributes config");
        app.set_config_overrides(overrides);

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
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_rules() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font_rules = {
              {
                italic = true,
                intensity = 'Bold',
                font = wezterm.font { family = 'Victor Mono', weight = 'Bold' },
              },
              {
                italic = true,
                intensity = 'Half',
                font = wezterm.font_with_fallback { 'Terminus', 'Noto Color Emoji' },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm font_rules config");
        app.set_config_overrides(overrides);

        let effective = format!("{:?}", app.native_effective_config());
        assert!(
            effective.contains("font_rules: ["),
            "effective config should expose WezTerm font_rules: {effective:?}"
        );
        assert!(
            effective.contains("italic: Some(true)")
                && effective.contains("intensity: Some(Bold)")
                && effective.contains("font: Some(\"Victor Mono\")"),
            "effective config should expose the bold italic font rule: {effective:?}"
        );
        assert!(
            effective.contains("intensity: Some(Half)")
                && effective.contains("font: Some(\"Terminus\")")
                && effective.contains("font_fallbacks: [\"Noto Color Emoji\"]"),
            "effective config should expose the fallback font rule: {effective:?}"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_rule_static_font_value() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local rule_font = wezterm.font {
              family = 'Victor Mono',
              weight = 'Bold',
            }

            config.font_rules = {
              {
                italic = true,
                intensity = 'Bold',
                font = rule_font,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm font rule static font value config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font_rules.len(), 1);
        assert_eq!(effective.font_rules[0].font.as_deref(), Some("Victor Mono"));
        assert_eq!(
            effective.font_rules[0].font_attributes.weight.as_deref(),
            Some("Bold")
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_rule_insert_static_font_value() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local rule_font = wezterm.font {
              family = 'Victor Mono',
              weight = 'Bold',
            }

            config.font_rules = {}
            table.insert(config.font_rules, {
              italic = true,
              intensity = 'Bold',
              font = rule_font,
            })

            return config
            "#,
        )
        .expect("expected WezTerm inserted font rule static font value config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font_rules.len(), 1);
        assert_eq!(effective.font_rules[0].font.as_deref(), Some("Victor Mono"));
        assert_eq!(
            effective.font_rules[0].font_attributes.weight.as_deref(),
            Some("Bold")
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_rule_insert_static_matcher_fields() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local rule_italic = true
            local rule_intensity = 'Bold'
            local rule_underline = 'Single'
            local rule_blink = 'Slow'
            local rule_reverse = true
            local rule_strikethrough = true
            local rule_invisible = false

            config.font_rules = {}
            table.insert(config.font_rules, {
              italic = rule_italic,
              intensity = rule_intensity,
              underline = rule_underline,
              blink = rule_blink,
              reverse = rule_reverse,
              strikethrough = rule_strikethrough,
              invisible = rule_invisible,
              font = wezterm.font { family = 'Victor Mono' },
            })

            return config
            "#,
        )
        .expect("expected WezTerm inserted font rule static matcher fields config");
        app.set_config_overrides(overrides);

        let rule = &app.native_effective_config().font_rules[0];
        assert_eq!(rule.italic, Some(true));
        assert_eq!(rule.intensity, Some(NativeFormatIntensity::Bold));
        assert_eq!(rule.underline, Some(NativeFormatUnderline::Single));
        assert_eq!(rule.blink, Some(NativeFontRuleBlink::Slow));
        assert_eq!(rule.reverse, Some(true));
        assert_eq!(rule.strikethrough, Some(true));
        assert_eq!(rule.invisible, Some(false));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_rule_static_field_names() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local italic_field = 'italic'
            local intensity_field = 'intensity'
            local underline_field = 'underline'
            local font_field = 'font'

            config.font_rules = {
              {
                [italic_field] = true,
                [intensity_field] = 'Bold',
                [underline_field] = 'Single',
                [font_field] = wezterm.font { family = 'Victor Mono' },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm font rule static field-name config");
        app.set_config_overrides(overrides);

        let rule = &app.native_effective_config().font_rules[0];
        assert_eq!(rule.italic, Some(true));
        assert_eq!(rule.intensity, Some(NativeFormatIntensity::Bold));
        assert_eq!(rule.underline, Some(NativeFormatUnderline::Single));
        assert_eq!(rule.font.as_deref(), Some("Victor Mono"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_rule_font_attributes() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font_rules = {
              {
                italic = true,
                intensity = 'Bold',
                font = wezterm.font {
                  family = 'Victor Mono',
                  weight = 'Bold',
                  stretch = 'Condensed',
                  style = 'Italic',
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm font_rule font attributes config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font_rules[0].font.as_deref(), Some("Victor Mono"));
        assert_eq!(
            effective.font_rules[0].font_attributes,
            NativeFontAttributes {
                weight: Some("Bold".to_owned()),
                stretch: Some("Condensed".to_owned()),
                style: Some("Italic".to_owned()),
                harfbuzz_features: Vec::new(),
                assume_emoji_presentation: None,
                freetype_load_target: None,
                freetype_render_target: None,
                freetype_load_flags: None,
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_rule_matchers() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font_rules = {
              {
                underline = 'Curly',
                blink = 'Rapid',
                reverse = true,
                strikethrough = true,
                invisible = true,
                font = wezterm.font 'Victor Mono',
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm font_rules matcher config");
        app.set_config_overrides(overrides);

        let effective = format!("{:?}", app.native_effective_config());
        assert!(
            effective.contains("underline: Some(Curly)")
                && effective.contains("blink: Some(Rapid)")
                && effective.contains("reverse: Some(true)")
                && effective.contains("strikethrough: Some(true)")
                && effective.contains("invisible: Some(true)"),
            "effective config should expose WezTerm font rule matchers: {effective:?}"
        );
    }

    #[test]
    fn window_app_reports_default_wezterm_fallback_font_scaling_config() {
        let app = NativeWindowApp::new(None);
        let effective = format!("{:?}", app.native_effective_config());

        assert!(
            effective.contains("use_cap_height_to_scale_fallback_fonts: false"),
            "effective config should expose WezTerm's use_cap_height_to_scale_fallback_fonts default: {effective:?}"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_fallback_font_scaling() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.use_cap_height_to_scale_fallback_fonts = true

            return config
            "#,
        )
        .expect("expected WezTerm fallback font scaling config");
        app.set_config_overrides(overrides);

        assert!(
            app.native_effective_config()
                .use_cap_height_to_scale_fallback_fonts
        );
    }

    #[test]
    fn window_app_reports_default_wezterm_unicode_version_config() {
        let app = NativeWindowApp::new(None);
        let effective = format!("{:?}", app.native_effective_config());

        assert!(
            effective.contains("unicode_version: 9"),
            "effective config should expose WezTerm's unicode_version default: {effective:?}"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_unicode_version() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.unicode_version = 14

            return config
            "#,
        )
        .expect("expected WezTerm unicode_version config");
        app.set_config_overrides(overrides);

        assert_eq!(app.native_effective_config().unicode_version, 14);
        assert_eq!(app.runtime.terminal().unicode_version(), 14);
    }

    #[test]
    fn window_app_parses_static_initializer_unicode_version_key() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local unicode_field = 'unicode_version'
            local config = {
              use_ime = false,
              [unicode_field] = 14,
            }

            return config
            "#,
        )
        .expect("expected WezTerm static field-name initializer unicode version config");
        app.set_config_overrides(overrides);

        assert_eq!(app.native_effective_config().unicode_version, 14);
        assert_eq!(app.runtime.terminal().unicode_version(), 14);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_dpi_override() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.dpi = 144.0

            return config
            "#,
        )
        .expect("expected WezTerm dpi config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(app.window_dpi, 144);
        assert_eq!(
            effective.freetype_load_flags,
            NativeFreetypeLoadFlags::NO_HINTING
        );
    }
