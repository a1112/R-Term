    #[test]
    fn window_app_parses_wezterm_lua_config_key_table_insert_static_field_name_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local key_field = 'key'
            local mods_field = 'mods'
            local action_field = 'action'

            local binding = {}
            binding[key_field] = 'K'
            binding[mods_field] = 'CTRL|SHIFT'
            binding[action_field] = act.SendString 'from-insert-static-field-name-variable'

            config.keys = {}
            table.insert(config.keys, binding)

            return config
            "#,
        )
        .expect("expected WezTerm table.insert static field-name key config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+K".to_owned(),
                command: WindowCommand::SendString(
                    "from-insert-static-field-name-variable".to_owned()
                ),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_table_static_field_variable_item() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local binding = {}
            binding.key = 'K'
            binding.mods = 'CTRL|SHIFT'
            binding.action = act.SendString 'from-table-field-variable-item'

            config.keys = { binding }

            return config
            "#,
        )
        .expect("expected WezTerm key table field-built item variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+K".to_owned(),
                command: WindowCommand::SendString("from-table-field-variable-item".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_index_assignment() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {}
            config.keys[1] = {
              key = 'K',
              mods = 'CTRL|SHIFT',
              action = act.SendString 'from-config-index',
            }

            return config
            "#,
        )
        .expect("expected WezTerm indexed key config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+K".to_owned(),
                command: WindowCommand::SendString("from-config-index".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_index_static_field_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local binding = {}
            binding.key = 'K'
            binding.mods = 'CTRL|SHIFT'
            binding.action = act.SendString 'from-index-field-variable'

            config.keys = {}
            config.keys[1] = binding

            return config
            "#,
        )
        .expect("expected WezTerm indexed field-built key config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+K".to_owned(),
                command: WindowCommand::SendString("from-index-field-variable".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_index_field_assignments() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {}
            config.keys[1] = {}
            config.keys[1].key = 'K'
            config.keys[1].mods = 'CTRL|SHIFT'
            config.keys[1].action = act.SendString 'from-config-index-fields'

            return config
            "#,
        )
        .expect("expected WezTerm indexed key field config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+K".to_owned(),
                command: WindowCommand::SendString("from-config-index-fields".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_static_key_key_index_field_assignments() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local keys_field = 'keys'
            local config = {}

            config[keys_field] = {}
            config[keys_field][1] = {}
            config[keys_field][1].key = 'K'
            config[keys_field][1].mods = 'CTRL|SHIFT'
            config[keys_field][1].action = act.SendString 'from-static-key-index-fields'

            return config
            "#,
        )
        .expect("expected WezTerm static field-name indexed key field config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+K".to_owned(),
                command: WindowCommand::SendString("from-static-key-index-fields".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_static_key_index_static_field_name_assignments() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local keys_field = 'keys'
            local key_field = 'key'
            local mods_field = 'mods'
            local action_field = 'action'
            local config = {}

            config[keys_field] = {}
            config[keys_field][1] = {}
            config[keys_field][1][key_field] = 'K'
            config[keys_field][1][mods_field] = 'CTRL|SHIFT'
            config[keys_field][1][action_field] = act.SendString 'from-static-key-index-static-fields'

            return config
            "#,
        )
        .expect("expected WezTerm static field-name indexed key static field config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+K".to_owned(),
                command: WindowCommand::SendString(
                    "from-static-key-index-static-fields".to_owned()
                ),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_length_append_assignment() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {}
            config.keys[#config.keys + 1] = {
              key = 'K',
              mods = 'CTRL|SHIFT',
              action = act.SendString 'from-config-length-append',
            }

            return config
            "#,
        )
        .expect("expected WezTerm length-append key config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+K".to_owned(),
                command: WindowCommand::SendString("from-config-length-append".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_static_variable_assignment() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local user_keys = {
              {
                key = 'H',
                mods = 'CTRL|SHIFT',
                action = act.SendString 'from-variable',
              },
            }

            config.keys = user_keys

            return config
            "#,
        )
        .expect("expected WezTerm static variable keys config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+H".to_owned(),
                command: WindowCommand::SendString("from-variable".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_static_variable_table_insert() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local user_keys = {}
            table.insert(user_keys, {
              key = 'H',
              mods = 'CTRL|SHIFT',
              action = act.SendString 'from-variable-insert',
            })

            config.keys = user_keys

            return config
            "#,
        )
        .expect("expected WezTerm static variable table.insert keys config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+H".to_owned(),
                command: WindowCommand::SendString("from-variable-insert".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_static_variable_index_assignment() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local user_keys = {}
            user_keys[1] = {
              key = 'H',
              mods = 'CTRL|SHIFT',
              action = act.SendString 'from-variable-index',
            }

            config.keys = user_keys

            return config
            "#,
        )
        .expect("expected WezTerm static variable indexed keys config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+H".to_owned(),
                command: WindowCommand::SendString("from-variable-index".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_static_variable_index_field_assignments() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local user_keys = {}
            user_keys[1] = {}
            user_keys[1].key = 'H'
            user_keys[1].mods = 'CTRL|SHIFT'
            user_keys[1].action = act.SendString 'from-variable-index-fields'

            config.keys = user_keys

            return config
            "#,
        )
        .expect("expected WezTerm static variable indexed field keys config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+H".to_owned(),
                command: WindowCommand::SendString("from-variable-index-fields".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_key_static_variable_index_static_field_name_assignments() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local key_field = 'key'
            local mods_field = 'mods'
            local action_field = 'action'

            local user_keys = {}
            user_keys[1] = {}
            user_keys[1][key_field] = 'H'
            user_keys[1][mods_field] = 'CTRL|SHIFT'
            user_keys[1][action_field] = act.SendString 'from-variable-index-static-fields'

            config.keys = user_keys

            return config
            "#,
        )
        .expect("expected WezTerm static variable indexed static field keys config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+H".to_owned(),
                command: WindowCommand::SendString("from-variable-index-static-fields".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_key_static_variable_post_assignment_index_fields() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local user_keys = {}
            config.keys = user_keys
            user_keys[1] = {}
            user_keys[1].key = 'H'
            user_keys[1].mods = 'CTRL|SHIFT'
            user_keys[1].action = act.SendString 'from-post-variable-index-fields'

            return config
            "#,
        )
        .expect("expected WezTerm post-assignment static variable indexed field keys config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+H".to_owned(),
                command: WindowCommand::SendString("from-post-variable-index-fields".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_static_variable_length_append_assignment() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            local user_keys = {}
            user_keys[#user_keys + 1] = {
              key = 'H',
              mods = 'CTRL|SHIFT',
              action = act.SendString 'from-variable-length-append',
            }

            config.keys = user_keys

            return config
            "#,
        )
        .expect("expected WezTerm static variable length-append keys config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+H".to_owned(),
                command: WindowCommand::SendString("from-variable-length-append".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_static_field_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local copy_key = 'H'
            local copy_mods = 'CTRL|SHIFT'

            config.keys = {
              {
                key = copy_key,
                mods = copy_mods,
                action = act.SendString 'from-field-variable',
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm key static field variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+H".to_owned(),
                command: WindowCommand::SendString("from-field-variable".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_static_field_name_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local key_field = 'key'
            local mods_field = 'mods'
            local action_field = 'action'

            config.keys = {
              {
                [key_field] = 'H',
                [mods_field] = 'CTRL|SHIFT',
                [action_field] = act.SendString 'from-static-field-name',
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm key static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+H".to_owned(),
                command: WindowCommand::SendString("from-static-field-name".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_static_action_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local send_line = act.SendString 'from-action-variable'

            config.keys = {
              {
                key = 'A',
                mods = 'CTRL|SHIFT',
                action = send_line,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm key static action variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+A".to_owned(),
                command: WindowCommand::SendString("from-action-variable".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_action_function_argument_comment() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'C',
                mods = 'CTRL|SHIFT',
                action = act.SendString('from-commented-call' -- payload
                ),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm key action function argument comment config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+C".to_owned(),
                command: WindowCommand::SendString("from-commented-call".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_action_string_call_comment() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'S',
                mods = 'CTRL|SHIFT',
                action = act.SendString -- payload
                  'from-commented-string-call',
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm key action string call comment config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+S".to_owned(),
                command: WindowCommand::SendString("from-commented-string-call".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_spawn_command_static_field_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local spawn_args = { 'top', '-d', '1' }
            local spawn_cwd = 'C:/Project Dir'
            local spawn_domain = 'local'
            local spawn_mode = 'dev'
            local spawn_position = 'main:42,84'

            config.keys = {
              {
                key = 'T',
                mods = 'CTRL|ALT',
                action = act.SpawnCommandInNewTab {
                  args = spawn_args,
                  cwd = spawn_cwd,
                  domain = spawn_domain,
                  set_environment_variables = { MODE = spawn_mode },
                },
              },
              {
                key = 'W',
                mods = 'CTRL|ALT',
                action = act.SpawnCommandInNewWindow {
                  args = spawn_args,
                  cwd = spawn_cwd,
                  domain = spawn_domain,
                  position = spawn_position,
                  set_environment_variables = { MODE = spawn_mode },
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SpawnCommand static field variable config");

        let command = WindowSpawnCommandQuery {
            label: None,
            program: "top".to_owned(),
            args: vec!["-d".to_owned(), "1".to_owned()],
            cwd: Some("C:/Project Dir".to_owned()),
            environment: BTreeMap::from([("MODE".to_owned(), "dev".to_owned())]),
            domain: Some(WindowSpawnTabDomain::DomainName("local".to_owned())),
            window_position: None,
        };

        assert_eq!(
            overrides.key_assignments,
            Some(vec![
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+T".to_owned(),
                    command: WindowCommand::SpawnCommandInNewTab(command.clone()),
                },
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+W".to_owned(),
                    command: WindowCommand::SpawnCommandInNewWindow(WindowSpawnCommandQuery {
                        label: None,
                        window_position: Some(crate::cli::WindowPosition {
                            origin: crate::cli::WindowPositionOrigin::Main,
                            x: 42,
                            y: 84,
                        }),
                        ..command
                    }),
                },
            ])
        );
    }

    #[test]
    fn window_app_show_launcher_key_assignments_use_spawn_command_label() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'T',
                mods = 'CTRL|ALT',
                action = act.SpawnCommandInNewTab {
                  label = 'System Monitor',
                  args = { 'top', '-d', '1' },
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SpawnCommand label config");

        let mut app = NativeWindowApp::new(None);
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::key_assignments(),
                title: Some("Pick Key".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));
        app.command_palette_set_query("system monitor".to_owned());

        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "CTRL|ALT+T: System Monitor");
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_spawn_command_static_environment_field_name() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local env_key = 'MODE'
            local spawn_mode = 'dev'

            config.keys = {
              {
                key = 'T',
                mods = 'CTRL|ALT',
                action = act.SpawnCommandInNewTab {
                  args = { 'top', '-d', '1' },
                  set_environment_variables = { [env_key] = spawn_mode },
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SpawnCommand static environment field-name config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+T".to_owned(),
                command: WindowCommand::SpawnCommandInNewTab(WindowSpawnCommandQuery {
                    label: None,
                    program: "top".to_owned(),
                    args: vec!["-d".to_owned(), "1".to_owned()],
                    cwd: None,
                    environment: BTreeMap::from([("MODE".to_owned(), "dev".to_owned())]),
                    domain: None,
                    window_position: None,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_spawn_command_static_table_variable_calls() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local spawn_args = { 'top', '-d', '1' }
            local spawn_cwd = 'C:/Project Dir'
            local spawn_domain = 'local'
            local spawn_mode = 'dev'
            local spawn_position = 'main:42,84'
            local spawn_opts = {
              args = spawn_args,
              cwd = spawn_cwd,
              domain = spawn_domain,
              set_environment_variables = { MODE = spawn_mode },
            }
            local window_opts = {
              args = spawn_args,
              cwd = spawn_cwd,
              domain = spawn_domain,
              position = spawn_position,
              set_environment_variables = { MODE = spawn_mode },
            }

            config.keys = {
              {
                key = 'T',
                mods = 'CTRL|ALT',
                action = act.SpawnCommandInNewTab(spawn_opts),
              },
              {
                key = 'W',
                mods = 'CTRL|ALT',
                action = act.SpawnCommandInNewWindow(window_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SpawnCommand static table variable call config");

        let command = WindowSpawnCommandQuery {
            label: None,
            program: "top".to_owned(),
            args: vec!["-d".to_owned(), "1".to_owned()],
            cwd: Some("C:/Project Dir".to_owned()),
            environment: BTreeMap::from([("MODE".to_owned(), "dev".to_owned())]),
            domain: Some(WindowSpawnTabDomain::DomainName("local".to_owned())),
            window_position: None,
        };

        assert_eq!(
            overrides.key_assignments,
            Some(vec![
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+T".to_owned(),
                    command: WindowCommand::SpawnCommandInNewTab(command.clone()),
                },
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+W".to_owned(),
                    command: WindowCommand::SpawnCommandInNewWindow(WindowSpawnCommandQuery {
                        label: None,
                        window_position: Some(crate::cli::WindowPosition {
                            origin: crate::cli::WindowPositionOrigin::Main,
                            x: 42,
                            y: 84,
                        }),
                        ..command
                    }),
                },
            ])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_spawn_command_static_option_table_variable_calls() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local spawn_cwd = 'C:/Project Dir'
            local spawn_domain = 'local'
            local spawn_mode = 'dev'
            local spawn_position = 'main:42,84'
            local spawn_opts = {
              cwd = spawn_cwd,
              domain = spawn_domain,
              set_environment_variables = { MODE = spawn_mode },
            }
            local window_opts = {
              cwd = spawn_cwd,
              domain = spawn_domain,
              position = spawn_position,
              set_environment_variables = { MODE = spawn_mode },
            }

            config.keys = {
              {
                key = 'T',
                mods = 'CTRL|ALT',
                action = act.SpawnCommandInNewTab(spawn_opts),
              },
              {
                key = 'W',
                mods = 'CTRL|ALT',
                action = act.SpawnCommandInNewWindow(window_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SpawnCommand static option table variable call config");

        let options = super::WindowSpawnCommandQueryOptions {
            cwd: Some("C:/Project Dir".to_owned()),
            environment: BTreeMap::from([("MODE".to_owned(), "dev".to_owned())]),
            domain: Some(WindowSpawnTabDomain::DomainName("local".to_owned())),
            window_position: None,
        };

        assert_eq!(
            overrides.key_assignments,
            Some(vec![
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+T".to_owned(),
                    command: WindowCommand::SpawnCommandOptionsInNewTab(options.clone()),
                },
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+W".to_owned(),
                    command: WindowCommand::SpawnCommandOptionsInNewWindow(
                        super::WindowSpawnCommandQueryOptions {
                            window_position: Some(crate::cli::WindowPosition {
                                origin: crate::cli::WindowPositionOrigin::Main,
                                x: 42,
                                y: 84,
                            }),
                            ..options
                        },
                    ),
                },
            ])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_spawn_tab_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local domain = 'CurrentPaneDomain'

            config.keys = {
              {
                key = 'T',
                mods = 'CTRL|ALT',
                action = act.SpawnTab(domain),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SpawnTab static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+T".to_owned(),
                command: WindowCommand::SpawnTab(WindowSpawnTabDomain::CurrentPaneDomain),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_spawn_tab_domain_name_static_field_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local domain_name = 'local'

            config.keys = {
              {
                key = 'D',
                mods = 'CTRL|ALT',
                action = act.SpawnTab { DomainName = domain_name },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SpawnTab DomainName static field variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+D".to_owned(),
                command: WindowCommand::SpawnTab(WindowSpawnTabDomain::DomainName(
                    "local".to_owned(),
                )),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_spawn_tab_domain_id_static_field_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local domain_id = 7

            config.keys = {
              {
                key = 'D',
                mods = 'CTRL|ALT',
                action = act.SpawnTab { DomainId = domain_id },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SpawnTab DomainId static field variable config");

        let keys = overrides
            .key_assignments.clone().expect("expected parsed key assignments");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].keys, "CTRL|ALT+D");
        assert_eq!(format!("{:?}", keys[0].command), "SpawnTab(DomainId(7))");
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_spawn_tab_static_field_name_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local domain_field = 'DomainName'
            local domain_name = 'local'

            config.keys = {
              {
                key = 'D',
                mods = 'CTRL|SHIFT',
                action = act.SpawnTab { [domain_field] = domain_name },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SpawnTab static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+D".to_owned(),
                command: WindowCommand::SpawnTab(WindowSpawnTabDomain::DomainName(
                    "local".to_owned(),
                )),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_spawn_tab_domain_name_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local domain_name = 'local'
            local tab_domain = {
              DomainName = domain_name,
            }

            config.keys = {
              {
                key = 'D',
                mods = 'CTRL|ALT',
                action = act.SpawnTab(tab_domain),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SpawnTab DomainName static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+D".to_owned(),
                command: WindowCommand::SpawnTab(WindowSpawnTabDomain::DomainName(
                    "local".to_owned(),
                )),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_attach_domain_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local domain = 'devhost'

            config.keys = {
              {
                key = 'A',
                mods = 'CTRL|ALT',
                action = act.AttachDomain(domain),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm AttachDomain static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+A".to_owned(),
                command: WindowCommand::AttachDomain("devhost".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_attach_domain_table_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local domain = 'devhost'

            config.keys = {
              {
                key = 'A',
                mods = 'CTRL|ALT',
                action = act.AttachDomain { DomainName = domain },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm AttachDomain static table config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+A".to_owned(),
                command: WindowCommand::AttachDomain("devhost".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_attach_domain_table_wrapper() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local domain = 'devhost'

            config.keys = {
              {
                key = 'A',
                mods = 'CTRL|ALT',
                action = act { AttachDomain = domain },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm AttachDomain table-wrapper config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+A".to_owned(),
                command: WindowCommand::AttachDomain("devhost".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_attach_domain_table_wrapper_default_domain_aliases() {
        for default_domain in [
            "default",
            "default domain",
            "default-domain",
            "default_domain",
        ] {
            let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
                r#"
                    local wezterm = require 'wezterm'
                    local act = wezterm.action
                    local config = {default_domain}

                    config.keys = {{
                      {{
                        key = 'A',
                        mods = 'CTRL|ALT',
                        action = act {{ AttachDomain = {{ DomainName = '{default_domain}' }} }},
                      }},
                    }}

                    return config
                    "#
            ))
            .expect("expected WezTerm AttachDomain table-wrapper default-domain config");

            assert_eq!(
                overrides.key_assignments,
                Some(vec![NativeUserKeyAssignment {
                    keys: "CTRL|ALT+A".to_owned(),
                    command: WindowCommand::AttachDomain(default_domain.to_owned()),
                }])
            );
        }
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_attach_domain_domainid_table_wrapper() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local domain_opts = {
              DomainId = 7,
            }

            config.keys = {
              {
                key = 'A',
                mods = 'CTRL|ALT',
                action = act { AttachDomain = domain_opts },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm AttachDomain table-wrapper DomainId config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+A".to_owned(),
                command: WindowCommand::AttachDomain("domainid:7".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_attach_domain_domainid_table_field() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'A',
                mods = 'CTRL|ALT',
                action = act.AttachDomain { DomainId = 7 },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm AttachDomain DomainId static table config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+A".to_owned(),
                command: WindowCommand::AttachDomain("domainid:7".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_detach_domain_table_wrapper() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local domain = 'devhost'
            local detach_domain = {
              DomainName = domain,
            }

            config.keys = {
              {
                key = 'D',
                mods = 'CTRL|ALT',
                action = act { DetachDomain = detach_domain },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm DetachDomain table-wrapper config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+D".to_owned(),
                command: WindowCommand::DetachDomain(WindowDomainSelector::DomainName(
                    "devhost".to_owned(),
                )),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_detach_domain_default_domain_selector() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'D',
                mods = 'CTRL|ALT',
                action = act.DetachDomain 'DefaultDomain',
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm DetachDomain DefaultDomain config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+D".to_owned(),
                command: WindowCommand::DetachDomain(WindowDomainSelector::DefaultDomain),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_detach_domain_table_wrapper_default_domain_aliases() {
        for default_domain in [
            "default",
            "default domain",
            "default-domain",
            "default_domain",
        ] {
            let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
                r#"
                local wezterm = require 'wezterm'
                local act = wezterm.action
                local config = {default_domain}

                config.keys = {{
                  {{
                    key = 'D',
                    mods = 'CTRL|ALT',
                    action = act {{ DetachDomain = {{ DomainName = '{default_domain}' }} }},
                  }},
                }}

                return config
                "#
            ))
            .expect("expected WezTerm DetachDomain table-wrapper default-domain config");

            assert_eq!(
                overrides.key_assignments,
                Some(vec![NativeUserKeyAssignment {
                    keys: "CTRL|ALT+D".to_owned(),
                    command: WindowCommand::DetachDomain(WindowDomainSelector::DomainName(
                        default_domain.to_owned()
                    )),
                }])
            );
        }
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_detach_domain_static_field_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local domain = 'devhost'

            config.keys = {
              {
                key = 'D',
                mods = 'CTRL|ALT',
                action = act.DetachDomain { DomainName = domain },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm DetachDomain static field variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+D".to_owned(),
                command: WindowCommand::DetachDomain(WindowDomainSelector::DomainName(
                    "devhost".to_owned(),
                )),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_detach_domain_id_static_field_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local domain_id = 7

            config.keys = {
              {
                key = 'D',
                mods = 'CTRL|ALT',
                action = act.DetachDomain { DomainId = domain_id },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm DetachDomain DomainId static field variable config");

        let keys = overrides
            .key_assignments.clone().expect("expected parsed key assignments");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].keys, "CTRL|ALT+D");
        assert_eq!(
            format!("{:?}", keys[0].command),
            "DetachDomain(DomainId(7))"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_detach_domain_static_field_name_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local domain_field = 'DomainName'
            local domain = 'devhost'

            config.keys = {
              {
                key = 'D',
                mods = 'CTRL|SHIFT',
                action = act.DetachDomain { [domain_field] = domain },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm DetachDomain static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+D".to_owned(),
                command: WindowCommand::DetachDomain(WindowDomainSelector::DomainName(
                    "devhost".to_owned(),
                )),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_detach_domain_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local domain = 'devhost'
            local detach_opts = {
              DomainName = domain,
            }

            config.keys = {
              {
                key = 'D',
                mods = 'CTRL|ALT',
                action = act.DetachDomain(detach_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm DetachDomain static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+D".to_owned(),
                command: WindowCommand::DetachDomain(WindowDomainSelector::DomainName(
                    "devhost".to_owned(),
                )),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_search_static_field_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local pattern = 'ticket-[0-9]+'

            config.keys = {
              {
                key = 'F',
                mods = 'CTRL|ALT',
                action = act.Search { Regex = pattern },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm Search static field variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+F".to_owned(),
                command: WindowCommand::Search(WindowSearchCommandQuery::Pattern {
                    pattern: "ticket-[0-9]+".to_owned(),
                    match_type: WindowSearchMatchType::Regex,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_search_static_field_name_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local search_field = 'Regex'
            local pattern = 'ticket-[0-9]+'

            config.keys = {
              {
                key = 'F',
                mods = 'CTRL|SHIFT',
                action = act.Search { [search_field] = pattern },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm Search static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+F".to_owned(),
                command: WindowCommand::Search(WindowSearchCommandQuery::Pattern {
                    pattern: "ticket-[0-9]+".to_owned(),
                    match_type: WindowSearchMatchType::Regex,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_search_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local pattern = 'ticket-[0-9]+'
            local search_opts = {
              Regex = pattern,
            }

            config.keys = {
              {
                key = 'F',
                mods = 'CTRL|ALT',
                action = act.Search(search_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm Search static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+F".to_owned(),
                command: WindowCommand::Search(WindowSearchCommandQuery::Pattern {
                    pattern: "ticket-[0-9]+".to_owned(),
                    match_type: WindowSearchMatchType::Regex,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_search_current_selection_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local selection_mode = 'CurrentSelectionOrEmptyString'

            config.keys = {
              {
                key = 'G',
                mods = 'CTRL|ALT',
                action = act.Search(selection_mode),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm Search current selection static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+G".to_owned(),
                command: WindowCommand::Search(
                    WindowSearchCommandQuery::CurrentSelectionOrEmptyString,
                ),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_split_pane_static_field_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local split_direction = 'Left'
            local split_domain = 'CurrentPaneDomain'
            local split_cells = 20
            local split_top_level = true
            local launch_args = { 'top', '-d', '1' }
            local launch_cwd = 'C:/Project Dir'
            local launch_domain = 'local'
            local launch_mode = 'dev'

            config.keys = {
              {
                key = 'S',
                mods = 'CTRL|ALT',
                action = act.SplitPane {
                  direction = split_direction,
                  domain = split_domain,
                  size = { Cells = split_cells },
                  top_level = split_top_level,
                  command = {
                    args = launch_args,
                    cwd = launch_cwd,
                    domain = launch_domain,
                    set_environment_variables = { MODE = launch_mode },
                  },
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SplitPane static field variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+S".to_owned(),
                command: WindowCommand::SplitPane(WindowSplitPaneOptions {
                    direction: rterm_core::app_shell::SplitDirection::Left,
                    domain: Some(WindowSpawnTabDomain::CurrentPaneDomain),
                    command: Some(WindowSpawnCommandQuery {
                        label: None,
                        program: "top".to_owned(),
                        args: vec!["-d".to_owned(), "1".to_owned()],
                        cwd: Some("C:/Project Dir".to_owned()),
                        environment: BTreeMap::from([("MODE".to_owned(), "dev".to_owned())]),
                        domain: Some(WindowSpawnTabDomain::DomainName("local".to_owned())),
                        window_position: None,
                    }),
                    command_options: None,
                    size: Some(WindowSplitPaneSize::Cells(20)),
                    top_level: true,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_split_pane_static_field_name_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local direction_field = 'direction'
            local args_field = 'args'
            local cwd_field = 'cwd'
            local size_field = 'size'
            local cells_field = 'Cells'
            local top_level_field = 'top_level'
            local split_direction = 'Right'
            local launch_args = { 'top', '-d', '1' }
            local launch_cwd = 'C:/Project Dir'
            local split_cells = 20

            config.keys = {
              {
                key = 'S',
                mods = 'CTRL|SHIFT',
                action = act.SplitPane {
                  [direction_field] = split_direction,
                  [args_field] = launch_args,
                  [cwd_field] = launch_cwd,
                  [size_field] = { [cells_field] = split_cells },
                  [top_level_field] = true,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SplitPane static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+S".to_owned(),
                command: WindowCommand::SplitPane(WindowSplitPaneOptions {
                    direction: rterm_core::app_shell::SplitDirection::Right,
                    domain: None,
                    command: Some(WindowSpawnCommandQuery {
                        label: None,
                        program: "top".to_owned(),
                        args: vec!["-d".to_owned(), "1".to_owned()],
                        cwd: Some("C:/Project Dir".to_owned()),
                        environment: BTreeMap::new(),
                        domain: None,
                        window_position: None,
                    }),
                    command_options: None,
                    size: Some(WindowSplitPaneSize::Cells(20)),
                    top_level: true,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_split_pane_static_table_variable_calls() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local split_direction = 'Left'
            local split_domain = 'CurrentPaneDomain'
            local split_cells = 20
            local split_top_level = true
            local launch_args = { 'top', '-d', '1' }
            local launch_cwd = 'C:/Project Dir'
            local launch_domain = 'local'
            local launch_mode = 'dev'
            local vertical_percent = 35
            local split_opts = {
              direction = split_direction,
              domain = split_domain,
              size = { Cells = split_cells },
              top_level = split_top_level,
              command = {
                args = launch_args,
                cwd = launch_cwd,
                domain = launch_domain,
                set_environment_variables = { MODE = launch_mode },
              },
            }
            local vertical_opts = {
              size = { Percent = vertical_percent },
            }

            config.keys = {
              {
                key = 'S',
                mods = 'CTRL|ALT',
                action = act.SplitPane(split_opts),
              },
              {
                key = 'V',
                mods = 'CTRL|ALT',
                action = act.SplitVertical(vertical_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SplitPane static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+S".to_owned(),
                    command: WindowCommand::SplitPane(WindowSplitPaneOptions {
                        direction: rterm_core::app_shell::SplitDirection::Left,
                        domain: Some(WindowSpawnTabDomain::CurrentPaneDomain),
                        command: Some(WindowSpawnCommandQuery {
                            label: None,
                            program: "top".to_owned(),
                            args: vec!["-d".to_owned(), "1".to_owned()],
                            cwd: Some("C:/Project Dir".to_owned()),
                            environment: BTreeMap::from([("MODE".to_owned(), "dev".to_owned())]),
                            domain: Some(WindowSpawnTabDomain::DomainName("local".to_owned())),
                            window_position: None,
                        }),
                        command_options: None,
                        size: Some(WindowSplitPaneSize::Cells(20)),
                        top_level: true,
                    }),
                },
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+V".to_owned(),
                    command: WindowCommand::SplitPane(WindowSplitPaneOptions {
                        direction: rterm_core::app_shell::SplitDirection::Down,
                        domain: None,
                        command: None,
                        command_options: None,
                        size: Some(WindowSplitPaneSize::Percent(35)),
                        top_level: false,
                    }),
                },
            ])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_close_current_static_field_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local pane_confirm = false
            local tab_confirm = true

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|ALT',
                action = act.CloseCurrentPane { confirm = pane_confirm },
              },
              {
                key = 'T',
                mods = 'CTRL|ALT',
                action = act.CloseCurrentTab { confirm = tab_confirm },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm CloseCurrent static field variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+P".to_owned(),
                    command: WindowCommand::CloseCurrentPane { confirm: false },
                },
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+T".to_owned(),
                    command: WindowCommand::CloseCurrentTab { confirm: true },
                },
            ])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_close_current_static_field_name_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local confirm_field = 'confirm'
            local pane_confirm = false
            local tab_confirm = true

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|ALT',
                action = act.CloseCurrentPane { [confirm_field] = pane_confirm },
              },
              {
                key = 'T',
                mods = 'CTRL|ALT',
                action = act.CloseCurrentTab { [confirm_field] = tab_confirm },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm CloseCurrent static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+P".to_owned(),
                    command: WindowCommand::CloseCurrentPane { confirm: false },
                },
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+T".to_owned(),
                    command: WindowCommand::CloseCurrentTab { confirm: true },
                },
            ])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_close_current_static_table_variable_calls() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local pane_confirm = false
            local tab_confirm = true
            local pane_opts = {
              confirm = pane_confirm,
            }
            local tab_opts = {
              confirm = tab_confirm,
            }

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|ALT',
                action = act.CloseCurrentPane(pane_opts),
              },
              {
                key = 'T',
                mods = 'CTRL|ALT',
                action = act.CloseCurrentTab(tab_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm CloseCurrent static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+P".to_owned(),
                    command: WindowCommand::CloseCurrentPane { confirm: false },
                },
                NativeUserKeyAssignment {
                    keys: "CTRL|ALT+T".to_owned(),
                    command: WindowCommand::CloseCurrentTab { confirm: true },
                },
            ])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_clear_scrollback_static_field_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local scroll_mode = 'ScrollbackAndViewport'

            config.keys = {
              {
                key = 'K',
                mods = 'CTRL|ALT',
                action = act.ClearScrollback { mode = scroll_mode },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ClearScrollback static field variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+K".to_owned(),
                command: WindowCommand::ClearScrollback(
                    WindowClearScrollbackMode::ScrollbackAndViewport,
                ),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_clear_scrollback_static_field_name_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local mode_field = 'mode'
            local scroll_mode = 'ScrollbackAndViewport'

            config.keys = {
              {
                key = 'K',
                mods = 'CTRL|ALT',
                action = act.ClearScrollback {
                  [mode_field] = scroll_mode,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ClearScrollback static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+K".to_owned(),
                command: WindowCommand::ClearScrollback(
                    WindowClearScrollbackMode::ScrollbackAndViewport,
                ),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_clear_scrollback_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local scroll_mode = 'ScrollbackAndViewport'
            local clear_opts = {
              mode = scroll_mode,
            }

            config.keys = {
              {
                key = 'K',
                mods = 'CTRL|ALT',
                action = act.ClearScrollback(clear_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ClearScrollback static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+K".to_owned(),
                command: WindowCommand::ClearScrollback(
                    WindowClearScrollbackMode::ScrollbackAndViewport,
                ),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_adjust_pane_size_static_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local resize_direction = 'Left'
            local resize_amount = 4

            config.keys = {
              {
                key = 'LeftArrow',
                mods = 'CTRL|SHIFT|ALT',
                action = act.AdjustPaneSize { resize_direction, resize_amount },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm AdjustPaneSize static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT|ALT+LeftArrow".to_owned(),
                command: WindowCommand::AdjustPaneSize {
                    direction: ResizeDirection::Left,
                    amount: 4,
                },
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_adjust_pane_size_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local resize_direction = 'Left'
            local resize_amount = 4
            local resize_opts = { resize_direction, resize_amount }

            config.keys = {
              {
                key = 'LeftArrow',
                mods = 'CTRL|SHIFT|ALT',
                action = act.AdjustPaneSize(resize_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm AdjustPaneSize static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT|ALT+LeftArrow".to_owned(),
                command: WindowCommand::AdjustPaneSize {
                    direction: ResizeDirection::Left,
                    amount: 4,
                },
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_set_pane_zoom_state_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local pane_zoomed = true

            config.keys = {
              {
                key = 'Z',
                mods = 'CTRL|SHIFT',
                action = act.SetPaneZoomState(pane_zoomed),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SetPaneZoomState static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+Z".to_owned(),
                command: WindowCommand::SetPaneZoomState(true),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_scroll_by_line_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local scroll_lines = -2

            config.keys = {
              {
                key = 'UpArrow',
                mods = 'SHIFT',
                action = act.ScrollByLine(scroll_lines),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ScrollByLine static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "SHIFT+UpArrow".to_owned(),
                command: WindowCommand::ScrollByLine(-2),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_scroll_to_prompt_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local prompt_delta = -1

            config.keys = {
              {
                key = 'P',
                mods = 'SHIFT',
                action = act.ScrollToPrompt(prompt_delta),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ScrollToPrompt static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "SHIFT+P".to_owned(),
                command: WindowCommand::ScrollToPrompt(-1),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_scroll_by_page_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local page_delta = -0.5

            config.keys = {
              {
                key = 'PageUp',
                mods = 'SHIFT',
                action = act.ScrollByPage(page_delta),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ScrollByPage static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "SHIFT+PageUp".to_owned(),
                command: WindowCommand::ScrollByPage(WindowScrollByPageAmount::from_per_mille(
                    -500,
                )),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_activate_tab_relative_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local tab_offset = -1

            config.keys = {
              {
                key = '[',
                mods = 'SUPER|SHIFT',
                action = act.ActivateTabRelative(tab_offset),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ActivateTabRelative static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "SUPER|SHIFT+[".to_owned(),
                command: WindowCommand::ActivateTabRelative(-1),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_activate_tab_relative_no_wrap_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local tab_offset = 1

            config.keys = {
              {
                key = ']',
                mods = 'SUPER|SHIFT',
                action = act.ActivateTabRelativeNoWrap(tab_offset),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ActivateTabRelativeNoWrap static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "SUPER|SHIFT+]".to_owned(),
                command: WindowCommand::ActivateTabRelativeNoWrap(1),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_activate_tab_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local tab_index = -1

            config.keys = {
              {
                key = '9',
                mods = 'SUPER',
                action = act.ActivateTab(tab_index),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ActivateTab static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "SUPER+9".to_owned(),
                command: WindowCommand::ActivateTab(-1),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_move_tab_relative_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local tab_offset = -1

            config.keys = {
              {
                key = 'LeftArrow',
                mods = 'SUPER|SHIFT',
                action = act.MoveTabRelative(tab_offset),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm MoveTabRelative static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "SUPER|SHIFT+LeftArrow".to_owned(),
                command: WindowCommand::MoveTabRelative(-1),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_move_tab_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local tab_index = 2

            config.keys = {
              {
                key = '2',
                mods = 'SUPER|SHIFT',
                action = act.MoveTab(tab_index),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm MoveTab static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "SUPER|SHIFT+2".to_owned(),
                command: WindowCommand::MoveTab(2),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_activate_pane_by_index_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local pane_index = 2

            config.keys = {
              {
                key = '1',
                mods = 'CTRL|SHIFT',
                action = act.ActivatePaneByIndex(pane_index),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ActivatePaneByIndex static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+1".to_owned(),
                command: WindowCommand::ActivatePaneByIndex(2),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_activate_pane_direction_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local pane_direction = 'Left'

            config.keys = {
              {
                key = 'LeftArrow',
                mods = 'CTRL|SHIFT',
                action = act.ActivatePaneDirection(pane_direction),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ActivatePaneDirection static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+LeftArrow".to_owned(),
                command: WindowCommand::ActivatePaneDirection(
                    rterm_core::app_shell::PaneDirection::Left,
                ),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_rotate_panes_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local rotation_direction = 'Clockwise'

            config.keys = {
              {
                key = 'R',
                mods = 'CTRL|SHIFT',
                action = act.RotatePanes(rotation_direction),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm RotatePanes static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+R".to_owned(),
                command: WindowCommand::RotatePanes(
                    rterm_core::app_shell::PaneRotationDirection::Clockwise,
                ),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_copy_to_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local destination = 'PrimarySelection'

            config.keys = {
              {
                key = 'C',
                mods = 'CTRL|SHIFT',
                action = act.CopyTo(destination),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm CopyTo static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+C".to_owned(),
                command: WindowCommand::CopyTo(WindowCopyDestination::PrimarySelection),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_copy_text_to_static_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local copied_text = 'literal text'
            local destination = 'ClipboardAndPrimarySelection'

            config.keys = {
              {
                key = 'C',
                mods = 'CTRL|SHIFT',
                action = act.CopyTextTo {
                  text = copied_text,
                  destination = destination,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm CopyTextTo static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+C".to_owned(),
                command: WindowCommand::CopyTextTo {
                    text: "literal text".to_owned(),
                    destination: WindowCopyDestination::ClipboardAndPrimarySelection,
                },
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_copy_text_to_static_field_name_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local text_field = 'text'
            local destination_field = 'destination'
            local copied_text = 'literal text'
            local destination = 'ClipboardAndPrimarySelection'

            config.keys = {
              {
                key = 'C',
                mods = 'CTRL|SHIFT',
                action = act.CopyTextTo {
                  [text_field] = copied_text,
                  [destination_field] = destination,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm CopyTextTo static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+C".to_owned(),
                command: WindowCommand::CopyTextTo {
                    text: "literal text".to_owned(),
                    destination: WindowCopyDestination::ClipboardAndPrimarySelection,
                },
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_open_uri_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local docs = 'https://example.com/docs'

            config.keys = {
              {
                key = 'O',
                mods = 'CTRL|SHIFT',
                action = act.OpenUri(docs),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm OpenUri static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+O".to_owned(),
                command: WindowCommand::OpenUri("https://example.com/docs".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_paste_from_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local source = 'PrimarySelection'

            config.keys = {
              {
                key = 'V',
                mods = 'CTRL|SHIFT',
                action = act.PasteFrom(source),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm PasteFrom static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+V".to_owned(),
                command: WindowCommand::PasteFrom(WindowPasteSource::PrimarySelection),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_complete_selection_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local destination = 'PrimarySelection'

            config.keys = {
              {
                key = 'Y',
                mods = 'CTRL|SHIFT',
                action = act.CompleteSelection(destination),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm CompleteSelection static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+Y".to_owned(),
                command: WindowCommand::CompleteSelectionTo(
                    WindowCopyDestination::PrimarySelection,
                ),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_complete_selection_or_open_link_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local destination = 'PrimarySelection'

            config.keys = {
              {
                key = 'O',
                mods = 'CTRL|SHIFT',
                action = act.CompleteSelectionOrOpenLinkAtMouseCursor(destination),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm CompleteSelectionOrOpenLinkAtMouseCursor static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+O".to_owned(),
                command: WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(
                    WindowCopyDestination::PrimarySelection,
                ),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_activate_window_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local index = 2

            config.keys = {
              {
                key = 'W',
                mods = 'CTRL|SHIFT',
                action = act.ActivateWindow(index),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ActivateWindow static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+W".to_owned(),
                command: WindowCommand::ActivateWindow(2),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_activate_window_relative_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local offset = -1

            config.keys = {
              {
                key = 'N',
                mods = 'CTRL|SHIFT',
                action = act.ActivateWindowRelative(offset),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ActivateWindowRelative static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+N".to_owned(),
                command: WindowCommand::ActivateWindowRelative(-1),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_activate_window_relative_no_wrap_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local offset = -2

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|SHIFT',
                action = act.ActivateWindowRelativeNoWrap(offset),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ActivateWindowRelativeNoWrap static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+P".to_owned(),
                command: WindowCommand::ActivateWindowRelativeNoWrap(-2),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_set_window_level_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local level = 'AlwaysOnTop'

            config.keys = {
              {
                key = 'T',
                mods = 'CTRL|SHIFT',
                action = act.SetWindowLevel(level),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SetWindowLevel static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+T".to_owned(),
                command: WindowCommand::SetWindowLevel(NativeWindowLevel::AlwaysOnTop),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_select_text_at_mouse_cursor_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local mode = 'SemanticZone'

            config.keys = {
              {
                key = 'S',
                mods = 'CTRL|SHIFT',
                action = act.SelectTextAtMouseCursor(mode),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SelectTextAtMouseCursor static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+S".to_owned(),
                command: WindowCommand::SelectTextAtMouseCursor(
                    WindowMouseSelectionMode::SemanticZone,
                ),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_extend_selection_to_mouse_cursor_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local mode = 'Block'

            config.keys = {
              {
                key = 'B',
                mods = 'CTRL|SHIFT',
                action = act.ExtendSelectionToMouseCursor(mode),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ExtendSelectionToMouseCursor static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+B".to_owned(),
                command: WindowCommand::ExtendSelectionToMouseCursor(
                    WindowMouseSelectionMode::Block,
                ),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_send_string_static_field_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local payload = 'from-send-string-variable'

            config.keys = {
              {
                key = 'S',
                mods = 'CTRL|SHIFT',
                action = act.SendString { string = payload },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SendString static string field variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+S".to_owned(),
                command: WindowCommand::SendString("from-send-string-variable".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_send_string_static_field_name_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local string_field = 'string'
            local payload = 'from-send-string-field-name-variable'

            config.keys = {
              {
                key = 'S',
                mods = 'CTRL|SHIFT',
                action = act.SendString {
                  [string_field] = payload,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SendString static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+S".to_owned(),
                command: WindowCommand::SendString(
                    "from-send-string-field-name-variable".to_owned(),
                ),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_send_string_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local send_opts = {
              string = 'from-send-string-table-variable',
            }

            config.keys = {
              {
                key = 'S',
                mods = 'CTRL|SHIFT',
                action = act.SendString(send_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SendString static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+S".to_owned(),
                command: WindowCommand::SendString("from-send-string-table-variable".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_send_key_static_field_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local key_name = 'LeftArrow'
            local key_mods = 'ALT'

            config.keys = {
              {
                key = 'K',
                mods = 'CTRL|SHIFT',
                action = act.SendKey { key = key_name, mods = key_mods },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SendKey static field variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+K".to_owned(),
                command: WindowCommand::SendKey(WindowSendKey {
                    key: Key::Named(NamedKey::ArrowLeft),
                    modifiers: ModifiersState::ALT,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_send_key_static_field_name_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local key_field = 'key'
            local mods_field = 'mods'
            local key_name = 'LeftArrow'
            local key_mods = 'ALT'

            config.keys = {
              {
                key = 'K',
                mods = 'CTRL|SHIFT',
                action = act.SendKey {
                  [key_field] = key_name,
                  [mods_field] = key_mods,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SendKey static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+K".to_owned(),
                command: WindowCommand::SendKey(WindowSendKey {
                    key: Key::Named(NamedKey::ArrowLeft),
                    modifiers: ModifiersState::ALT,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_send_key_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local send_key_opts = {
              key = 'LeftArrow',
              mods = 'ALT',
            }

            config.keys = {
              {
                key = 'K',
                mods = 'CTRL|SHIFT',
                action = act.SendKey(send_key_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SendKey static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+K".to_owned(),
                command: WindowCommand::SendKey(WindowSendKey {
                    key: Key::Named(NamedKey::ArrowLeft),
                    modifiers: ModifiersState::ALT,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_prompt_input_line_static_field_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local prompt_description = 'Rename tab'
            local prompt_label = 'name: '
            local prompt_initial = 'old name'

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|SHIFT',
                action = act.PromptInputLine {
                  description = prompt_description,
                  prompt = prompt_label,
                  initial_value = prompt_initial,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm PromptInputLine static field variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+P".to_owned(),
                command: WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
                    description: "Rename tab".to_owned(),
                    prompt: Some("name: ".to_owned()),
                    initial_value: Some("old name".to_owned()),
                    action: None,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_prompt_input_line_static_field_name_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local description_field = 'description'
            local prompt_field = 'prompt'
            local initial_field = 'initial_value'
            local action_field = 'action'

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|SHIFT',
                action = act.PromptInputLine {
                  [description_field] = 'Rename tab',
                  [prompt_field] = 'name: ',
                  [initial_field] = 'old name',
                  [action_field] = wezterm.action_callback(function(window, pane, line) end),
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm PromptInputLine static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+P".to_owned(),
                command: WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
                    description: "Rename tab".to_owned(),
                    prompt: Some("name: ".to_owned()),
                    initial_value: Some("old name".to_owned()),
                    action: None,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_prompt_input_line_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local prompt_opts = {
              description = 'Rename tab',
              prompt = 'name: ',
              initial_value = 'old name',
            }

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|SHIFT',
                action = act.PromptInputLine(prompt_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm PromptInputLine static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+P".to_owned(),
                command: WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
                    description: "Rename tab".to_owned(),
                    prompt: Some("name: ".to_owned()),
                    initial_value: Some("old name".to_owned()),
                    action: None,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_prompt_input_line_static_action_callback_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local prompt_action = wezterm.action_callback(function(window, pane, line) end)

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|SHIFT',
                action = act.PromptInputLine {
                  description = 'Rename tab',
                  prompt = 'name: ',
                  initial_value = 'old name',
                  action = prompt_action,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm PromptInputLine static action callback variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+P".to_owned(),
                command: WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
                    description: "Rename tab".to_owned(),
                    prompt: Some("name: ".to_owned()),
                    initial_value: Some("old name".to_owned()),
                    action: None,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_prompt_input_line_static_rename_tab_callback_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local prompt_action = wezterm.action_callback(function(window, pane, line)
              if line then
                window:active_tab():set_title(line)
              end
            end)

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|SHIFT',
                action = act.PromptInputLine {
                  description = 'Rename tab',
                  prompt = 'name: ',
                  initial_value = 'old name',
                  action = prompt_action,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm PromptInputLine static rename-tab callback variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+P".to_owned(),
                command: WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
                    description: "Rename tab".to_owned(),
                    prompt: Some("name: ".to_owned()),
                    initial_value: Some("old name".to_owned()),
                    action: Some(WindowPromptInputLineAction::RenameActiveTab),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_prompt_input_line_nested_static_action() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local prompt_action = act.Nop

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|SHIFT',
                action = act.PromptInputLine {
                  description = 'Confirm',
                  action = prompt_action,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm PromptInputLine nested static action config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+P".to_owned(),
                command: WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
                    description: "Confirm".to_owned(),
                    prompt: None,
                    initial_value: None,
                    action: Some(WindowPromptInputLineAction::Command(Box::new(
                        WindowCommand::Nop,
                    ))),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_prompt_input_line_static_format_alias_fields() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local fmt = wezterm.format
            local config = {}
            local prompt_label = fmt { { Text = 'name' }, { Text = ': ' } }

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|SHIFT',
                action = act.PromptInputLine {
                  description = fmt { { Text = 'Rename' }, { Text = ' tab' } },
                  prompt = prompt_label,
                  initial_value = 'old name',
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm PromptInputLine static format alias field config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+P".to_owned(),
                command: WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
                    description: "Rename tab".to_owned(),
                    prompt: Some("name: ".to_owned()),
                    initial_value: Some("old name".to_owned()),
                    action: None,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_prompt_input_line_format_alias_with_comment_before_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local fmt = wezterm.format
            local config = {}

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|SHIFT',
                action = act.PromptInputLine {
                  description = fmt -- title
                    { { Text = 'Rename' }, { Text = ' tab' } },
                  prompt = fmt -- prompt
                    ({ { Text = 'name: ' } }),
                  initial_value = 'old name',
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm PromptInputLine static format alias comment table-call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+P".to_owned(),
                command: WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
                    description: "Rename tab".to_owned(),
                    prompt: Some("name: ".to_owned()),
                    initial_value: Some("old name".to_owned()),
                    action: None,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_prompt_input_line_static_format_text_values() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local fmt = wezterm.format
            local config = {}
            local rename = 'Rename'
            local tab = ' tab'
            local prompt_name = 'name'
            local prompt_suffix = ': '

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|SHIFT',
                action = act.PromptInputLine {
                  description = fmt { { Text = rename }, { Text = tab } },
                  prompt = fmt { { Text = prompt_name }, { Text = prompt_suffix } },
                  initial_value = 'old name',
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm PromptInputLine static format text value config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+P".to_owned(),
                command: WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
                    description: "Rename tab".to_owned(),
                    prompt: Some("name: ".to_owned()),
                    initial_value: Some("old name".to_owned()),
                    action: None,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_input_selector_static_string_field_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local selector_title = 'Pick Reply'
            local selector_choices = 'decline=No thanks ; lgtm=LGTM'
            local selector_alphabet = 'ab'
            local selector_description = 'Choose one:'
            local selector_fuzzy_description = 'Filter replies:'

            config.keys = {
              {
                key = 'I',
                mods = 'CTRL|SHIFT',
                action = act.InputSelector {
                  title = selector_title,
                  choices = selector_choices,
                  alphabet = selector_alphabet,
                  description = selector_description,
                  fuzzy_description = selector_fuzzy_description,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm InputSelector static string field variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+I".to_owned(),
                command: WindowCommand::InputSelector(WindowInputSelectorOptions {
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
                    description: Some("Choose one:".to_owned()),
                    fuzzy_description: Some("Filter replies:".to_owned()),
                    fuzzy: false,
                    action: None,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_input_selector_static_field_name_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local title_field = 'title'
            local choices_field = 'choices'
            local alphabet_field = 'alphabet'
            local description_field = 'description'
            local fuzzy_description_field = 'fuzzy_description'
            local fuzzy_field = 'fuzzy'
            local action_field = 'action'
            local id_field = 'id'
            local label_field = 'label'
            local selector_fuzzy = true
            local selector_action = wezterm.action_callback(function(window, pane, id, label) end)

            config.keys = {
              {
                key = 'I',
                mods = 'CTRL|SHIFT',
                action = act.InputSelector {
                  [title_field] = 'Pick Reply',
                  [choices_field] = {
                    { [id_field] = 'decline', [label_field] = 'No thanks' },
                    { [id_field] = 'lgtm', [label_field] = 'LGTM' },
                  },
                  [alphabet_field] = 'ab',
                  [description_field] = 'Choose one:',
                  [fuzzy_description_field] = 'Filter replies:',
                  [fuzzy_field] = selector_fuzzy,
                  [action_field] = selector_action,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm InputSelector static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+I".to_owned(),
                command: WindowCommand::InputSelector(WindowInputSelectorOptions {
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
                    description: Some("Choose one:".to_owned()),
                    fuzzy_description: Some("Filter replies:".to_owned()),
                    fuzzy: true,
                    action: None,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_input_selector_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local input_opts = {
              title = 'Pick Reply',
              choices = 'decline=No thanks ; lgtm=LGTM',
              alphabet = 'ab',
              description = 'Choose one:',
              fuzzy_description = 'Filter replies:',
              fuzzy = true,
            }

            config.keys = {
              {
                key = 'I',
                mods = 'CTRL|SHIFT',
                action = act.InputSelector(input_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm InputSelector static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+I".to_owned(),
                command: WindowCommand::InputSelector(WindowInputSelectorOptions {
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
                    description: Some("Choose one:".to_owned()),
                    fuzzy_description: Some("Filter replies:".to_owned()),
                    fuzzy: true,
                    action: None,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_input_selector_static_action_callback_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local selector_action = wezterm.action_callback(function(window, pane, id, label) end)

            config.keys = {
              {
                key = 'I',
                mods = 'CTRL|SHIFT',
                action = act.InputSelector {
                  title = 'Pick Reply',
                  choices = 'decline=No thanks ; lgtm=LGTM',
                  action = selector_action,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm InputSelector static action callback variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+I".to_owned(),
                command: WindowCommand::InputSelector(WindowInputSelectorOptions {
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
                    ..WindowInputSelectorOptions::default()
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_input_selector_nested_static_action() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local selector_action = act.Nop

            config.keys = {
              {
                key = 'I',
                mods = 'CTRL|SHIFT',
                action = act.InputSelector {
                  title = 'Pick Reply',
                  choices = 'decline=No thanks ; lgtm=LGTM',
                  action = selector_action,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm InputSelector nested static action config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+I".to_owned(),
                command: WindowCommand::InputSelector(WindowInputSelectorOptions {
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
                    action: Some(WindowInputSelectorAction::Command(Box::new(
                        WindowCommand::Nop,
                    ))),
                    ..WindowInputSelectorOptions::default()
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_static_action_callback_aliases() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local cb = wezterm.action_callback
            local config = {}
            local prompt_action = cb(function(window, pane, line) end)

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|SHIFT',
                action = act.PromptInputLine {
                  description = 'Rename tab',
                  action = prompt_action,
                },
              },
              {
                key = 'I',
                mods = 'CTRL|SHIFT',
                action = act.InputSelector {
                  title = 'Pick Reply',
                  choices = 'decline=No thanks ; lgtm=LGTM',
                  action = cb(function(window, pane, id, label) end),
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm action_callback alias config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![
                NativeUserKeyAssignment {
                    keys: "CTRL|SHIFT+P".to_owned(),
                    command: WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
                        description: "Rename tab".to_owned(),
                        ..WindowPromptInputLineOptions::default()
                    }),
                },
                NativeUserKeyAssignment {
                    keys: "CTRL|SHIFT+I".to_owned(),
                    command: WindowCommand::InputSelector(WindowInputSelectorOptions {
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
                        ..WindowInputSelectorOptions::default()
                    }),
                },
            ])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_static_action_callback_alias_comment_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local cb = wezterm.action_callback
            local config = {}

            config.keys = {
              {
                key = 'Q',
                mods = 'CTRL|SHIFT',
                action = act.QuickSelectArgs {
                  pattern = 'ticket-[0-9]+',
                  action = cb -- selected ticket
                    (function(window, pane)
                      window:perform_action(act.CopyTo 'Clipboard', pane)
                    end),
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm action_callback alias comment-call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+Q".to_owned(),
                command: WindowCommand::QuickSelectArgs(WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    action: Some(WindowQuickSelectAction::CopyTo(
                        WindowCopyDestination::Clipboard
                    )),
                    ..WindowQuickSelectOptions::default()
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_static_action_callback_alias_dotted_comment() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local cb = wezterm -- callback helper
              .action_callback
            local config = {}

            config.keys = {
              {
                key = 'Q',
                mods = 'CTRL|SHIFT',
                action = act.QuickSelectArgs {
                  pattern = 'ticket-[0-9]+',
                  action = cb(function(window, pane)
                    window:perform_action(act.CopyTo 'Clipboard', pane)
                  end),
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm action_callback dotted-comment alias config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+Q".to_owned(),
                command: WindowCommand::QuickSelectArgs(WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    action: Some(WindowQuickSelectAction::CopyTo(
                        WindowCopyDestination::Clipboard
                    )),
                    ..WindowQuickSelectOptions::default()
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_static_action_callback_alias_static_key_module() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wt = require 'wezterm'
            local act = wt.action
            local callback_key = 'action_callback'
            local cb = wt[callback_key]
            local config = {}

            config.keys = {
              {
                key = 'K',
                mods = 'CTRL|SHIFT',
                action = cb(function(window, pane)
                  window:perform_action(act.SendString 'from-static-key-callback', pane)
                end),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm action_callback static-key module alias config");
        app.set_config_overrides(overrides);

        app.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
        app.handle_keyboard_input_event(
            &Key::Character("k".into()),
            PhysicalKey::Code(WinitKeyCode::KeyK),
            Some("k"),
            ElementState::Pressed,
            KittyKeyEventKind::Press,
        )
        .unwrap();

        assert_eq!(
            written.lock().unwrap().as_slice(),
            b"from-static-key-callback"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_input_selector_static_choices_table_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local decline_id = 'decline'
            local decline_label = 'No thanks'
            local choices = {
              { id = decline_id, label = decline_label },
              { id = 'lgtm', label = wezterm.format { { Text = 'LGTM' } } },
            }

            config.keys = {
              {
                key = 'I',
                mods = 'CTRL|SHIFT',
                action = act.InputSelector {
                  title = 'Pick Reply',
                  choices = choices,
                  alphabet = 'ab',
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm InputSelector static choices table variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+I".to_owned(),
                command: WindowCommand::InputSelector(WindowInputSelectorOptions {
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
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_input_selector_static_format_choice_label_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local lgtm_label = wezterm.format { { Text = 'LGTM' } }

            config.keys = {
              {
                key = 'I',
                mods = 'CTRL|SHIFT',
                action = act.InputSelector {
                  title = 'Pick Reply',
                  choices = {
                    { id = 'decline', label = 'No thanks' },
                    { id = 'lgtm', label = lgtm_label },
                  },
                  alphabet = 'ab',
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm InputSelector static format choice label variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+I".to_owned(),
                command: WindowCommand::InputSelector(WindowInputSelectorOptions {
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
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_input_selector_static_format_alias_label_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local fmt = wezterm.format
            local config = {}
            local lgtm_label = fmt { { Text = 'LGTM' } }

            config.keys = {
              {
                key = 'I',
                mods = 'CTRL|SHIFT',
                action = act.InputSelector {
                  title = 'Pick Reply',
                  choices = {
                    { id = 'decline', label = 'No thanks' },
                    { id = 'lgtm', label = lgtm_label },
                  },
                  alphabet = 'ab',
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm InputSelector static format alias label variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+I".to_owned(),
                command: WindowCommand::InputSelector(WindowInputSelectorOptions {
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
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_input_selector_static_fuzzy_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local selector_fuzzy = true

            config.keys = {
              {
                key = 'F',
                mods = 'CTRL|SHIFT',
                action = act.InputSelector {
                  title = 'Pick Reply',
                  choices = 'decline=No thanks ; lgtm=LGTM',
                  fuzzy = selector_fuzzy,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm InputSelector static fuzzy variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+F".to_owned(),
                command: WindowCommand::InputSelector(WindowInputSelectorOptions {
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
                    fuzzy: true,
                    ..WindowInputSelectorOptions::default()
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_confirmation_static_field_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local confirm_message = 'Send command?'
            local accept_action = act.SendString 'yes'
            local cancel_action = act.SendString 'no'

            config.keys = {
              {
                key = 'C',
                mods = 'CTRL|SHIFT',
                action = act.Confirmation {
                  message = confirm_message,
                  action = accept_action,
                  cancel = cancel_action,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm Confirmation static field variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+C".to_owned(),
                command: WindowCommand::Confirmation(WindowConfirmationOptions {
                    message: "Send command?".to_owned(),
                    action: Box::new(WindowCommand::SendString("yes".to_owned())),
                    cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_confirmation_static_field_name_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local message_field = 'message'
            local action_field = 'action'
            local cancel_field = 'cancel'
            local accept_action = act.SendString 'yes'
            local cancel_action = act.SendString 'no'

            config.keys = {
              {
                key = 'C',
                mods = 'CTRL|SHIFT',
                action = act.Confirmation {
                  [message_field] = 'Send command?',
                  [action_field] = accept_action,
                  [cancel_field] = cancel_action,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm Confirmation static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+C".to_owned(),
                command: WindowCommand::Confirmation(WindowConfirmationOptions {
                    message: "Send command?".to_owned(),
                    action: Box::new(WindowCommand::SendString("yes".to_owned())),
                    cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_confirmation_static_action_alias_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local action = wezterm.action
            local config = {}
            local confirm_message = 'Send command?'
            local accept_action = action.SendString 'yes'
            local cancel_action = action["SendString"]('no')

            config.keys = {
              {
                key = 'C',
                mods = 'CTRL|SHIFT',
                action = act.Confirmation {
                  message = confirm_message,
                  action = accept_action,
                  cancel = cancel_action,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm Confirmation static action alias variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+C".to_owned(),
                command: WindowCommand::Confirmation(WindowConfirmationOptions {
                    message: "Send command?".to_owned(),
                    action: Box::new(WindowCommand::SendString("yes".to_owned())),
                    cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_confirmation_static_format_message_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local confirm_message = wezterm.format { { Text = 'Send' }, { Text = ' command?' } }

            config.keys = {
              {
                key = 'C',
                mods = 'CTRL|SHIFT',
                action = act.Confirmation {
                  message = confirm_message,
                  action = act.SendString 'yes',
                  cancel = act.SendString 'no',
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm Confirmation static format message variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+C".to_owned(),
                command: WindowCommand::Confirmation(WindowConfirmationOptions {
                    message: "Send command?".to_owned(),
                    action: Box::new(WindowCommand::SendString("yes".to_owned())),
                    cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_confirmation_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local confirm_opts = {
              message = 'Send command?',
              action = act.SendString 'yes',
              cancel = act.SendString 'no',
            }

            config.keys = {
              {
                key = 'C',
                mods = 'CTRL|SHIFT',
                action = act.Confirmation(confirm_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm Confirmation static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+C".to_owned(),
                command: WindowCommand::Confirmation(WindowConfirmationOptions {
                    message: "Send command?".to_owned(),
                    action: Box::new(WindowCommand::SendString("yes".to_owned())),
                    cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_quick_select_static_non_string_fields() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local skip_paste = true
            local line_count = 2

            config.keys = {
              {
                key = 'Q',
                mods = 'CTRL|SHIFT',
                action = act.QuickSelectArgs {
                  action = 'open-uri',
                  skip_action_on_paste = skip_paste,
                  scope_lines = line_count,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm QuickSelectArgs static non-string field config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+Q".to_owned(),
                command: WindowCommand::QuickSelectArgs(WindowQuickSelectOptions {
                    action: Some(WindowQuickSelectAction::OpenUri),
                    skip_action_on_paste: true,
                    scope_lines: Some(2),
                    ..WindowQuickSelectOptions::default()
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_quick_select_static_string_fields() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local ticket_pattern = 'ticket-[0-9]+'
            local label_text = 'Open ticket'
            local selector_alphabet = '12'
            local selected_action = 'open-uri'

            config.keys = {
              {
                key = 'Q',
                mods = 'CTRL|ALT',
                action = act.QuickSelectArgs {
                  pattern = ticket_pattern,
                  label = label_text,
                  alphabet = selector_alphabet,
                  action = selected_action,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm QuickSelectArgs static string field config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+Q".to_owned(),
                command: WindowCommand::QuickSelectArgs(WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    label: Some("Open ticket".to_owned()),
                    alphabet: Some("12".to_owned()),
                    action: Some(WindowQuickSelectAction::OpenUri),
                    ..WindowQuickSelectOptions::default()
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_quick_select_static_field_name_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local pattern_field = 'pattern'
            local alphabet_field = 'alphabet'
            local label_field = 'label'
            local action_field = 'action'
            local skip_field = 'skip_action_on_paste'
            local scope_field = 'scope_lines'

            config.keys = {
              {
                key = 'Q',
                mods = 'CTRL|ALT',
                action = act.QuickSelectArgs {
                  [pattern_field] = 'ticket-[0-9]+',
                  [alphabet_field] = '12',
                  [label_field] = 'Open ticket',
                  [action_field] = 'open-uri',
                  [skip_field] = true,
                  [scope_field] = 2,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm QuickSelectArgs static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+Q".to_owned(),
                command: WindowCommand::QuickSelectArgs(WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    alphabet: Some("12".to_owned()),
                    label: Some("Open ticket".to_owned()),
                    action: Some(WindowQuickSelectAction::OpenUri),
                    skip_action_on_paste: true,
                    scope_lines: Some(2),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_quick_select_static_patterns_table_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local ticket_pattern = 'ticket-[0-9]+'
            local patterns = { ticket_pattern, 'bug-[A-Z]+' }

            config.keys = {
              {
                key = 'Q',
                mods = 'CTRL|ALT',
                action = act.QuickSelectArgs {
                  patterns = patterns,
                  alphabet = '12',
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm QuickSelectArgs static patterns table variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+Q".to_owned(),
                command: WindowCommand::QuickSelectArgs(WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned(), "bug-[A-Z]+".to_owned()]),
                    alphabet: Some("12".to_owned()),
                    ..WindowQuickSelectOptions::default()
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_quick_select_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local quick_opts = {
              pattern = 'ticket-[0-9]+',
              alphabet = '12',
              label = 'Open ticket',
              action = 'open-uri',
              skip_action_on_paste = true,
              scope_lines = 2,
            }

            config.keys = {
              {
                key = 'Q',
                mods = 'CTRL|ALT',
                action = act.QuickSelectArgs(quick_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm QuickSelectArgs static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+Q".to_owned(),
                command: WindowCommand::QuickSelectArgs(WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    alphabet: Some("12".to_owned()),
                    label: Some("Open ticket".to_owned()),
                    action: Some(WindowQuickSelectAction::OpenUri),
                    skip_action_on_paste: true,
                    scope_lines: Some(2),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_pane_select_static_field_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local pane_mode = 'SwapWithActive'
            local show_ids = true
            local pane_alphabet = '12'

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|ALT',
                action = act.PaneSelect {
                  mode = pane_mode,
                  show_pane_ids = show_ids,
                  alphabet = pane_alphabet,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm PaneSelect static field variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+P".to_owned(),
                command: WindowCommand::PaneSelect(WindowPaneSelectOptions {
                    mode: WindowPaneSelectMode::SwapWithActive,
                    show_pane_ids: true,
                    alphabet: Some("12".to_owned()),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_pane_select_static_field_name_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local mode_field = 'mode'
            local show_ids_field = 'show_pane_ids'
            local alphabet_field = 'alphabet'

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|ALT',
                action = act.PaneSelect {
                  [mode_field] = 'SwapWithActive',
                  [show_ids_field] = true,
                  [alphabet_field] = '12',
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm PaneSelect static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+P".to_owned(),
                command: WindowCommand::PaneSelect(WindowPaneSelectOptions {
                    mode: WindowPaneSelectMode::SwapWithActive,
                    show_pane_ids: true,
                    alphabet: Some("12".to_owned()),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_pane_select_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local pane_opts = {
              mode = 'SwapWithActive',
              show_pane_ids = true,
              alphabet = '12',
            }

            config.keys = {
              {
                key = 'P',
                mods = 'CTRL|ALT',
                action = act.PaneSelect(pane_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm PaneSelect static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+P".to_owned(),
                command: WindowCommand::PaneSelect(WindowPaneSelectOptions {
                    mode: WindowPaneSelectMode::SwapWithActive,
                    show_pane_ids: true,
                    alphabet: Some("12".to_owned()),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_char_select_static_field_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local should_copy = false
            local copy_target = 'PrimarySelection'
            local select_group = 'PeopleAndBody'

            config.keys = {
              {
                key = 'U',
                mods = 'CTRL|ALT',
                action = act.CharSelect {
                  copy_on_select = should_copy,
                  copy_to = copy_target,
                  group = select_group,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm CharSelect static field variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+U".to_owned(),
                command: WindowCommand::CharSelectArgs(WindowCharSelectOptions {
                    copy_on_select: false,
                    copy_to: WindowCopyDestination::PrimarySelection,
                    group: Some("PeopleAndBody".to_owned()),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_char_select_static_field_name_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local copy_on_select_field = 'copy_on_select'
            local copy_to_field = 'copy_to'
            local group_field = 'group'

            config.keys = {
              {
                key = 'U',
                mods = 'CTRL|ALT',
                action = act.CharSelect {
                  [copy_on_select_field] = false,
                  [copy_to_field] = 'PrimarySelection',
                  [group_field] = 'PeopleAndBody',
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm CharSelect static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+U".to_owned(),
                command: WindowCommand::CharSelectArgs(WindowCharSelectOptions {
                    copy_on_select: false,
                    copy_to: WindowCopyDestination::PrimarySelection,
                    group: Some("PeopleAndBody".to_owned()),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_char_select_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local char_opts = {
              copy_on_select = false,
              copy_to = 'PrimarySelection',
              group = 'PeopleAndBody',
            }

            config.keys = {
              {
                key = 'U',
                mods = 'CTRL|ALT',
                action = act.CharSelect(char_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm CharSelect static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+U".to_owned(),
                command: WindowCommand::CharSelectArgs(WindowCharSelectOptions {
                    copy_on_select: false,
                    copy_to: WindowCopyDestination::PrimarySelection,
                    group: Some("PeopleAndBody".to_owned()),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_emit_event_static_name_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local event_name = 'session-ready'

            config.keys = {
              {
                key = 'E',
                mods = 'CTRL|SHIFT',
                action = act.EmitEvent { name = event_name },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm EmitEvent static name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+E".to_owned(),
                command: WindowCommand::EmitEvent(WindowEmitEvent {
                    name: "session-ready".to_owned(),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_emit_event_static_field_name_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local name_field = 'name'
            local event_name = 'session-ready'

            config.keys = {
              {
                key = 'E',
                mods = 'CTRL|SHIFT',
                action = act.EmitEvent {
                  [name_field] = event_name,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm EmitEvent static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+E".to_owned(),
                command: WindowCommand::EmitEvent(WindowEmitEvent {
                    name: "session-ready".to_owned(),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_emit_event_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local event_opts = {
              name = 'session-ready',
            }

            config.keys = {
              {
                key = 'E',
                mods = 'CTRL|SHIFT',
                action = act.EmitEvent(event_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm EmitEvent static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+E".to_owned(),
                command: WindowCommand::EmitEvent(WindowEmitEvent {
                    name: "session-ready".to_owned(),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_activate_key_table_static_field_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local table_name = 'resize_pane'
            local table_timeout = 1000
            local keep_active = false
            local block_fallback = true

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable {
                  name = table_name,
                  timeout_milliseconds = table_timeout,
                  one_shot = keep_active,
                  prevent_fallback = block_fallback,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ActivateKeyTable static field variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+Space".to_owned(),
                command: WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
                    name: "resize_pane".to_owned(),
                    timeout_milliseconds: Some(1000),
                    one_shot: false,
                    replace_current: false,
                    until_unknown: false,
                    prevent_fallback: true,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_activate_key_table_static_field_name_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local name_field = 'name'
            local timeout_field = 'timeout_milliseconds'
            local one_shot_field = 'one_shot'
            local replace_field = 'replace_current'
            local until_field = 'until_unknown'
            local prevent_field = 'prevent_fallback'

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|ALT',
                action = act.ActivateKeyTable {
                  [name_field] = 'resize_pane',
                  [timeout_field] = 1000,
                  [one_shot_field] = false,
                  [replace_field] = true,
                  [until_field] = true,
                  [prevent_field] = true,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ActivateKeyTable static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+Space".to_owned(),
                command: WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
                    name: "resize_pane".to_owned(),
                    timeout_milliseconds: Some(1000),
                    one_shot: false,
                    replace_current: true,
                    until_unknown: true,
                    prevent_fallback: true,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_activate_key_table_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local key_table_opts = {
              name = 'resize_pane',
              timeout_milliseconds = 1000,
              one_shot = false,
              replace_current = true,
              until_unknown = true,
              prevent_fallback = true,
            }

            config.keys = {
              {
                key = 'Space',
                mods = 'CTRL|SHIFT',
                action = act.ActivateKeyTable(key_table_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ActivateKeyTable static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+Space".to_owned(),
                command: WindowCommand::ActivateKeyTable(WindowActivateKeyTable {
                    name: "resize_pane".to_owned(),
                    timeout_milliseconds: Some(1000),
                    one_shot: false,
                    replace_current: true,
                    until_unknown: true,
                    prevent_fallback: true,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_multiple_static_action_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local send_alpha = act.SendString 'alpha'

            config.keys = {
              {
                key = 'M',
                mods = 'CTRL|SHIFT',
                action = act.Multiple {
                  send_alpha,
                  act.SendString 'beta',
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm key multiple static action variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+M".to_owned(),
                command: WindowCommand::Multiple(vec![
                    WindowCommand::SendString("alpha".to_owned()),
                    WindowCommand::SendString("beta".to_owned()),
                ]),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_multiple_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local send_alpha = act.SendString 'alpha'
            local actions = {
              send_alpha,
              act.SendString 'beta',
            }

            config.keys = {
              {
                key = 'M',
                mods = 'CTRL|SHIFT',
                action = act.Multiple(actions),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm key multiple static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+M".to_owned(),
                command: WindowCommand::Multiple(vec![
                    WindowCommand::SendString("alpha".to_owned()),
                    WindowCommand::SendString("beta".to_owned()),
                ]),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_quick_select_static_action_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local quick_copy = act.CopyTo 'Clipboard'

            config.keys = {
              {
                key = 'Q',
                mods = 'CTRL|SHIFT',
                action = act.QuickSelectArgs {
                  pattern = 'ticket-[0-9]+',
                  action = quick_copy,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm quick select static action variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+Q".to_owned(),
                command: WindowCommand::QuickSelectArgs(WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    action: Some(WindowQuickSelectAction::CopyTo(
                        WindowCopyDestination::Clipboard
                    )),
                    ..WindowQuickSelectOptions::default()
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_quick_select_static_action_alias() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local action = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'Q',
                mods = 'CTRL|SHIFT',
                action = act.QuickSelectArgs {
                  pattern = 'ticket-[0-9]+',
                  action = action["CopyTo"]('PrimarySelection'),
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm quick select static action alias config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+Q".to_owned(),
                command: WindowCommand::QuickSelectArgs(WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    action: Some(WindowQuickSelectAction::CopyTo(
                        WindowCopyDestination::PrimarySelection
                    )),
                    ..WindowQuickSelectOptions::default()
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_quick_select_static_action_alias_comment_table_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local action = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'Q',
                mods = 'CTRL|SHIFT',
                action = act.QuickSelectArgs {
                  pattern = 'ticket-[0-9]+',
                  action = action -- nested action
                    { CopyTo = 'PrimarySelection' },
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm quick select static action alias comment table-call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+Q".to_owned(),
                command: WindowCommand::QuickSelectArgs(WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    action: Some(WindowQuickSelectAction::CopyTo(
                        WindowCopyDestination::PrimarySelection
                    )),
                    ..WindowQuickSelectOptions::default()
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_quick_select_static_action_alias_comment_dot_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local action = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'Q',
                mods = 'CTRL|SHIFT',
                action = act.QuickSelectArgs {
                  pattern = 'ticket-[0-9]+',
                  action = action -- nested action
                    .CopyTo('PrimarySelection'),
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm quick select static action alias comment dot-call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+Q".to_owned(),
                command: WindowCommand::QuickSelectArgs(WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    action: Some(WindowQuickSelectAction::CopyTo(
                        WindowCopyDestination::PrimarySelection
                    )),
                    ..WindowQuickSelectOptions::default()
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_quick_select_static_action_alias_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local action = wezterm.action
            local config = {}
            local quick_copy = action.CopyTo 'PrimarySelection'

            config.keys = {
              {
                key = 'Q',
                mods = 'CTRL|SHIFT',
                action = act.QuickSelectArgs {
                  pattern = 'ticket-[0-9]+',
                  action = quick_copy,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm quick select static action alias variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+Q".to_owned(),
                command: WindowCommand::QuickSelectArgs(WindowQuickSelectOptions {
                    patterns: Some(vec!["ticket-[0-9]+".to_owned()]),
                    action: Some(WindowQuickSelectAction::CopyTo(
                        WindowCopyDestination::PrimarySelection
                    )),
                    ..WindowQuickSelectOptions::default()
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_return_key_static_variable_assignment() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action

            local user_keys = {
              {
                key = 'R',
                mods = 'CTRL|SHIFT',
                action = act.SendString 'from-return-variable',
              },
            }

            return {
              keys = user_keys,
            }
            "#,
        )
        .expect("expected WezTerm return-table static variable keys config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+R".to_owned(),
                command: WindowCommand::SendString("from-return-variable".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_key_show_launcher_args_table_action() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.keys = {
              {
                key = 'L',
                mods = 'CTRL|SHIFT',
                action = act.ShowLauncherArgs {
                  flags = 'TABS|WORKSPACES',
                  title = 'Jump',
                  alphabet = 'ab',
                  help_text = 'Pick',
                  fuzzy_help_text = 'Filter',
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ShowLauncherArgs key config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+L".to_owned(),
                command: WindowCommand::ShowLauncherArgs(WindowShowLauncherArgs {
                    flags: WindowShowLauncherFlags {
                        tabs: true,
                        workspaces: true,
                        ..WindowShowLauncherFlags::default()
                    },
                    title: Some("Jump".to_owned()),
                    alphabet: Some("ab".to_owned()),
                    help_text: Some("Pick".to_owned()),
                    fuzzy_help_text: Some("Filter".to_owned()),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_show_launcher_args_static_field_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local launcher_flags = 'TABS|WORKSPACES'
            local launcher_title = 'Jump'
            local launcher_alphabet = 'ab'
            local launcher_help = 'Pick'
            local launcher_fuzzy_help = 'Filter'

            config.keys = {
              {
                key = 'L',
                mods = 'CTRL|ALT',
                action = act.ShowLauncherArgs {
                  flags = launcher_flags,
                  title = launcher_title,
                  alphabet = launcher_alphabet,
                  help_text = launcher_help,
                  fuzzy_help_text = launcher_fuzzy_help,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ShowLauncherArgs static field variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+L".to_owned(),
                command: WindowCommand::ShowLauncherArgs(WindowShowLauncherArgs {
                    flags: WindowShowLauncherFlags {
                        tabs: true,
                        workspaces: true,
                        ..WindowShowLauncherFlags::default()
                    },
                    title: Some("Jump".to_owned()),
                    alphabet: Some("ab".to_owned()),
                    help_text: Some("Pick".to_owned()),
                    fuzzy_help_text: Some("Filter".to_owned()),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_show_launcher_args_static_field_name_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local flags_field = 'flags'
            local title_field = 'title'
            local alphabet_field = 'alphabet'
            local help_field = 'help_text'
            local fuzzy_help_field = 'fuzzy_help_text'

            config.keys = {
              {
                key = 'L',
                mods = 'CTRL|ALT',
                action = act.ShowLauncherArgs {
                  [flags_field] = 'TABS|WORKSPACES',
                  [title_field] = 'Jump',
                  [alphabet_field] = 'ab',
                  [help_field] = 'Pick',
                  [fuzzy_help_field] = 'Filter',
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ShowLauncherArgs static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+L".to_owned(),
                command: WindowCommand::ShowLauncherArgs(WindowShowLauncherArgs {
                    flags: WindowShowLauncherFlags {
                        tabs: true,
                        workspaces: true,
                        ..WindowShowLauncherFlags::default()
                    },
                    title: Some("Jump".to_owned()),
                    alphabet: Some("ab".to_owned()),
                    help_text: Some("Pick".to_owned()),
                    fuzzy_help_text: Some("Filter".to_owned()),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_show_launcher_args_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local launcher_args = {
              flags = 'TABS|WORKSPACES',
              title = 'Jump',
              alphabet = 'ab',
              help_text = 'Pick',
              fuzzy_help_text = 'Filter',
            }

            config.keys = {
              {
                key = 'L',
                mods = 'CTRL|ALT',
                action = act.ShowLauncherArgs(launcher_args),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ShowLauncherArgs static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+L".to_owned(),
                command: WindowCommand::ShowLauncherArgs(WindowShowLauncherArgs {
                    flags: WindowShowLauncherFlags {
                        tabs: true,
                        workspaces: true,
                        ..WindowShowLauncherFlags::default()
                    },
                    title: Some("Jump".to_owned()),
                    alphabet: Some("ab".to_owned()),
                    help_text: Some("Pick".to_owned()),
                    fuzzy_help_text: Some("Filter".to_owned()),
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_switch_workspace_relative_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local offset = -1

            config.keys = {
              {
                key = 'W',
                mods = 'CTRL|ALT',
                action = act.SwitchWorkspaceRelative(offset),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SwitchWorkspaceRelative static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+W".to_owned(),
                command: WindowCommand::SwitchWorkspaceRelative(-1),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_rename_tab_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local title = 'build prod'

            config.keys = {
              {
                key = 'T',
                mods = 'CTRL|ALT',
                action = act.RenameTab(title),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm RenameTab static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+T".to_owned(),
                command: WindowCommand::RenameTabTo("build prod".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_rename_workspace_static_variable() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local name = 'deploy west'

            config.keys = {
              {
                key = 'R',
                mods = 'CTRL|ALT',
                action = act.RenameWorkspace(name),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm RenameWorkspace static variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+R".to_owned(),
                command: WindowCommand::RenameWorkspaceTo("deploy west".to_owned()),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_switch_to_workspace_static_field_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local workspace_name = 'monitoring'
            local spawn_args = { 'top', '-d', '1' }
            local spawn_cwd = 'C:/Mon'

            config.keys = {
              {
                key = 'W',
                mods = 'CTRL|ALT',
                action = act.SwitchToWorkspace {
                  name = workspace_name,
                  spawn = {
                    args = spawn_args,
                    cwd = spawn_cwd,
                  },
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SwitchToWorkspace static field variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+W".to_owned(),
                command: WindowCommand::SwitchToWorkspaceArgs(WindowSwitchToWorkspaceOptions {
                    name: Some("monitoring".to_owned()),
                    command: Some(WindowSpawnCommandQuery {
                        label: None,
                        program: "top".to_owned(),
                        args: vec!["-d".to_owned(), "1".to_owned()],
                        cwd: Some("C:/Mon".to_owned()),
                        environment: BTreeMap::new(),
                        domain: None,
                        window_position: None,
                    }),
                    command_options: None,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_switch_to_workspace_static_field_name_variables() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local name_field = 'name'
            local spawn_field = 'spawn'
            local workspace_name = 'monitoring'
            local spawn_args = { 'top', '-d', '1' }
            local spawn_cwd = 'C:/Mon'

            config.keys = {
              {
                key = 'W',
                mods = 'CTRL|SHIFT',
                action = act.SwitchToWorkspace {
                  [name_field] = workspace_name,
                  [spawn_field] = {
                    args = spawn_args,
                    cwd = spawn_cwd,
                  },
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SwitchToWorkspace static field-name variable config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|SHIFT+W".to_owned(),
                command: WindowCommand::SwitchToWorkspaceArgs(WindowSwitchToWorkspaceOptions {
                    name: Some("monitoring".to_owned()),
                    command: Some(WindowSpawnCommandQuery {
                        label: None,
                        program: "top".to_owned(),
                        args: vec!["-d".to_owned(), "1".to_owned()],
                        cwd: Some("C:/Mon".to_owned()),
                        environment: BTreeMap::new(),
                        domain: None,
                        window_position: None,
                    }),
                    command_options: None,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_switch_to_workspace_static_table_variable_call() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local workspace_name = 'monitoring'
            local spawn_args = { 'top', '-d', '1' }
            local spawn_cwd = 'C:/Mon'
            local spawn_mode = 'watch'
            local workspace_opts = {
              name = workspace_name,
              spawn = {
                args = spawn_args,
                cwd = spawn_cwd,
                set_environment_variables = { MODE = spawn_mode },
              },
            }

            config.keys = {
              {
                key = 'W',
                mods = 'CTRL|ALT',
                action = act.SwitchToWorkspace(workspace_opts),
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm SwitchToWorkspace static table variable call config");

        assert_eq!(
            overrides.key_assignments,
            Some(vec![NativeUserKeyAssignment {
                keys: "CTRL|ALT+W".to_owned(),
                command: WindowCommand::SwitchToWorkspaceArgs(WindowSwitchToWorkspaceOptions {
                    name: Some("monitoring".to_owned()),
                    command: Some(WindowSpawnCommandQuery {
                        label: None,
                        program: "top".to_owned(),
                        args: vec!["-d".to_owned(), "1".to_owned()],
                        cwd: Some("C:/Mon".to_owned()),
                        environment: BTreeMap::from([("MODE".to_owned(), "watch".to_owned())]),
                        domain: None,
                        window_position: None,
                    }),
                    command_options: None,
                }),
            }])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_leader_into_runtime_assignments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}

            config.leader = { key = 'a', mods = 'CTRL', timeout_milliseconds = 1000 }

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
    fn window_app_parses_wezterm_lua_config_leader_static_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local act = wezterm.action
            local config = {}
            local user_leader = { key = 'a', mods = 'CTRL', timeout_milliseconds = 1000 }

            config.leader = user_leader

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
        .expect("expected WezTerm leader static variable config");
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

