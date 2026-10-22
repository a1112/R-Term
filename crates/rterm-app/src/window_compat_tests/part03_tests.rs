    #[test]
    fn window_app_loads_wezterm_lua_builtin_solarized_light_color_scheme_aliases() {
        for color_scheme in [
            "Builtin Solarized Light",
            "SolarizedLight (Gogh)",
            "iTerm2 Solarized Light",
        ] {
            let mut app = NativeWindowApp::new(None);
            let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
                r##"
                local config = {{}}

                config.color_scheme = '{color_scheme}'

                return config
                "##
            ))
            .expect("expected WezTerm built-in Solarized Light color_scheme config");
            app.set_config_overrides(overrides);

            let effective = app.native_effective_config();
            assert_eq!(effective.color_scheme.as_deref(), Some(color_scheme));
            assert_eq!(effective.foreground_color, Color::Rgb(101, 123, 131));
            assert_eq!(effective.background_color, Color::Rgb(253, 246, 227));
            assert_eq!(effective.cursor_bg_color, Color::Rgb(101, 123, 131));
            assert_eq!(effective.cursor_fg_color, Some(Color::Rgb(238, 232, 213)));
            let ansi = effective.ansi_palette.expect("expected ANSI palette");
            assert_eq!(ansi[0], Color::Rgb(7, 54, 66));
            assert_eq!(ansi[1], Color::Rgb(220, 50, 47));
            assert_eq!(ansi[8], Color::Rgb(0, 43, 54));
            assert_eq!(ansi[15], Color::Rgb(253, 246, 227));
        }
    }

    #[test]
    fn window_app_loads_wezterm_lua_builtin_tango_color_schemes() {
        let cases = [
            (
                "Builtin Tango Dark",
                Color::Rgb(255, 255, 255),
                Color::Rgb(0, 0, 0),
                Color::Rgb(255, 255, 255),
                Some(Color::Rgb(0, 0, 0)),
            ),
            (
                "Builtin Tango Light",
                Color::Rgb(0, 0, 0),
                Color::Rgb(255, 255, 255),
                Color::Rgb(0, 0, 0),
                Some(Color::Rgb(255, 255, 255)),
            ),
        ];

        for (color_scheme, foreground, background, cursor_bg, cursor_fg) in cases {
            let mut app = NativeWindowApp::new(None);
            let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
                r##"
                local config = {{}}

                config.color_scheme = '{color_scheme}'

                return config
                "##
            ))
            .expect("expected WezTerm built-in Tango color_scheme config");
            app.set_config_overrides(overrides);

            let effective = app.native_effective_config();
            assert_eq!(effective.color_scheme.as_deref(), Some(color_scheme));
            assert_eq!(effective.foreground_color, foreground);
            assert_eq!(effective.background_color, background);
            assert_eq!(effective.cursor_bg_color, cursor_bg);
            assert_eq!(effective.cursor_fg_color, cursor_fg);
            let ansi = effective.ansi_palette.expect("expected ANSI palette");
            assert_eq!(ansi[1], Color::Rgb(204, 0, 0));
            assert_eq!(ansi[8], Color::Rgb(85, 87, 83));
            assert_eq!(ansi[15], Color::Rgb(238, 238, 236));
        }
    }

    #[test]
    fn window_app_loads_wezterm_lua_builtin_color_scheme_aliases_from_wezterm_upstream() {
        for color_scheme in [
            "AyuDark (Gogh)",
            "AyuLight (Gogh)",
            "AyuMirage (Gogh)",
            "BelafonteDay (Gogh)",
            "BelafonteNight (Gogh)",
            "CloneofUbuntu (Gogh)",
            "CobaltNeon (Gogh)",
            "DarkPastel (Gogh)",
            "DeHydration (Gogh)",
            "EspressoLibre (Gogh)",
            "EverforestDark (Gogh)",
            "EverforestLight (Gogh)",
            "FairyFloss (Gogh)",
            "FairyFlossDark (Gogh)",
            "FlatRemix (Gogh)",
            "FrontendDelight (Gogh)",
            "FrontendFunForrest (Gogh)",
            "FrontendGalaxy (Gogh)",
            "GeoHot (Gogh)",
            "Miu (Gogh)",
        ] {
            assert!(builtin_color_scheme_toml(color_scheme).is_some());
        }
    }

    #[test]
    #[ignore = "requires the optional refs/wezterm reference checkout"]
    fn builtin_color_scheme_lookup_covers_pinned_wezterm_names_and_aliases() {
        let data_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../refs/wezterm/docs/colorschemes/data.json");
        let data = std::fs::read_to_string(&data_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", data_path.display()));
        let schemes: serde_json::Value = serde_json::from_str(&data)
            .expect("pinned WezTerm color scheme data must be valid JSON");
        let mut missing = Vec::new();

        for scheme in schemes
            .as_array()
            .expect("pinned WezTerm color scheme data must be an array")
        {
            let metadata = &scheme["metadata"];
            let name = metadata["name"]
                .as_str()
                .expect("pinned WezTerm color scheme must have a name");
            if builtin_color_scheme_toml(name).is_none() {
                missing.push(name.to_owned());
            }

            for alias in metadata["aliases"]
                .as_array()
                .expect("pinned WezTerm color scheme aliases must be an array")
            {
                let alias = alias
                    .as_str()
                    .expect("pinned WezTerm color scheme alias must be a string");
                if builtin_color_scheme_toml(alias).is_none() {
                    missing.push(alias.to_owned());
                }
            }
        }

        assert!(
            missing.is_empty(),
            "missing built-in color scheme names or aliases: {missing:?}"
        );
    }

    #[test]
    #[ignore = "requires the optional refs/wezterm reference checkout"]
    fn builtin_color_scheme_lookup_matches_all_pinned_wezterm_palette_data() {
        let data_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../refs/wezterm/docs/colorschemes/data.json");
        let data = std::fs::read_to_string(&data_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", data_path.display()));
        let schemes: serde_json::Value = serde_json::from_str(&data)
            .expect("pinned WezTerm color scheme data must be valid JSON");
        let schemes = schemes
            .as_array()
            .expect("pinned WezTerm color scheme data must be an array");
        let mut effective_palettes = BTreeMap::new();

        for scheme in schemes {
            let metadata = &scheme["metadata"];
            let canonical_name = metadata["name"]
                .as_str()
                .expect("pinned WezTerm color scheme must have a name");
            let aliases = metadata["aliases"]
                .as_array()
                .expect("pinned WezTerm color scheme aliases must be an array");

            for name in std::iter::once(canonical_name).chain(
                aliases
                    .iter()
                    .map(|alias| alias.as_str().expect("palette alias must be a string")),
            ) {
                // WezTerm builds COLOR_SCHEMES in this same order: each canonical
                // name and its aliases are inserted, so later canonical collisions
                // replace an earlier alias with the same spelling.
                effective_palettes.insert(name, (&scheme["colors"], canonical_name));
            }
        }

        let mut mismatched = Vec::new();
        for (name, (expected_colors, canonical_name)) in &effective_palettes {
            let toml_source = builtin_color_scheme_toml(name)
                .unwrap_or_else(|| panic!("missing pinned palette name {name:?}"));
            let toml_value = toml::from_str::<toml::Value>(toml_source)
                .unwrap_or_else(|error| panic!("invalid TOML for {name:?}: {error}"));
            let actual_colors = serde_json::to_value(
                toml_value
                    .get("colors")
                    .unwrap_or_else(|| panic!("missing colors table for {name:?}")),
            )
            .expect("TOML colors must serialize as JSON");

            if actual_colors != **expected_colors {
                mismatched.push(format!("{name:?} => {canonical_name:?}"));
            }
        }

        assert_eq!(schemes.len(), 1001, "unexpected pinned canonical count");
        assert_eq!(
            effective_palettes.len(),
            1113,
            "unexpected effective canonical-plus-alias count"
        );
        assert!(
            mismatched.is_empty(),
            "palette data mismatches: {}",
            mismatched.join(", ")
        );
    }

    #[test]
    fn window_app_retains_unknown_wezterm_lua_color_scheme_name() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local config = {}

            config.color_scheme = 'Unknown Project Scheme'

            return config
            "##,
        )
        .expect("expected unknown WezTerm color_scheme name to remain a valid config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.color_scheme.as_deref(),
            Some("Unknown Project Scheme")
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_color_scheme_dirs() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.color_scheme_dirs = { 'schemes', '/opt/wezterm/colors' }

            return config
            "##,
        )
        .expect("expected WezTerm color_scheme_dirs config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().color_scheme_dirs,
            vec!["schemes".to_owned(), "/opt/wezterm/colors".to_owned()]
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_color_scheme_dirs_static_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local config = {}
            local project_scheme_dirs = { 'schemes', '/opt/wezterm/colors' }

            config.term = 'xterm-256color'
            config.color_scheme_dirs = project_scheme_dirs

            return config
            "##,
        )
        .expect("expected WezTerm color_scheme_dirs static variable config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().color_scheme_dirs,
            vec!["schemes".to_owned(), "/opt/wezterm/colors".to_owned()]
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_color_scheme_dirs_table_insert() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local config = {}

            config.color_scheme_dirs = {}
            table.insert(config.color_scheme_dirs, 'schemes')
            table.insert(config.color_scheme_dirs, '/opt/wezterm/colors')

            return config
            "##,
        )
        .expect("expected WezTerm color_scheme_dirs table insert config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().color_scheme_dirs,
            vec!["schemes".to_owned(), "/opt/wezterm/colors".to_owned()]
        );
    }

    #[test]
    fn window_app_loads_wezterm_lua_color_scheme_from_configured_toml_dir() {
        static NEXT_COLOR_SCHEME_DIR_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_dir = std::env::temp_dir();
        scheme_dir.push(format!(
            "rterm-color-scheme-dir-{}-{}",
            std::process::id(),
            NEXT_COLOR_SCHEME_DIR_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&scheme_dir);
        std::fs::create_dir_all(&scheme_dir).expect("expected temp color scheme dir");
        std::fs::write(
            scheme_dir.join("project.toml"),
            r##"
            [metadata]
            name = "Project Scheme"
            origin_url = "https://example.invalid/project"

            [colors]
            foreground = "#010203"
            background = "#040506"
            cursor_bg = "#070809"
            cursor_border = "#0a0b0c"
            cursor_fg = "#0d0e0f"
            selection_bg = "#101112"
            selection_fg = "#131415"
            ansi = [
              "#000001",
              "#000002",
              "#000003",
              "#000004",
              "#000005",
              "#000006",
              "#000007",
              "#000008",
            ]
            brights = [
              "#000009",
              "#00000a",
              "#00000b",
              "#00000c",
              "#00000d",
              "#00000e",
              "#00000f",
              "#000010",
            ]
            indexed = { 136 = "#202122" }
            "##,
        )
        .expect("expected temp TOML color scheme");
        let scheme_dir = scheme_dir.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}

            config.color_scheme = 'Project Scheme'
            config.color_scheme_dirs = {{ '{scheme_dir}' }}

            return config
            "##
        ))
        .expect("expected WezTerm external TOML color scheme config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.color_scheme_dirs, vec![scheme_dir.clone()]);
        assert_eq!(effective.foreground_color, Color::Rgb(1, 2, 3));
        assert_eq!(effective.background_color, Color::Rgb(4, 5, 6));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(7, 8, 9));
        assert_eq!(effective.cursor_border_color, Some(Color::Rgb(10, 11, 12)));
        assert_eq!(effective.cursor_fg_color, Some(Color::Rgb(13, 14, 15)));
        assert_eq!(effective.selection_bg_color, Some(Color::Rgb(16, 17, 18)));
        assert_eq!(
            effective.selection_fg_color,
            Some(Some(Color::Rgb(19, 20, 21)))
        );
        assert_eq!(
            effective.ansi_palette.expect("expected ANSI palette")[1],
            Color::Rgb(0, 0, 2)
        );
        assert_eq!(
            effective.ansi_palette.expect("expected ANSI palette")[9],
            Color::Rgb(0, 0, 10)
        );
        assert_eq!(
            effective.indexed_palette.expect("expected indexed palette")[136],
            Some(Color::Rgb(32, 33, 34))
        );
        let _ = std::fs::remove_dir_all(scheme_dir);
    }

    #[cfg(windows)]
    #[test]
    fn window_app_loads_wezterm_lua_color_scheme_from_default_windows_colors_dir() {
        static NEXT_DEFAULT_COLOR_SCHEME_DIR_ID: AtomicUsize = AtomicUsize::new(0);

        let scheme_name = format!(
            "RTERM Default Windows Scheme {}-{}",
            std::process::id(),
            NEXT_DEFAULT_COLOR_SCHEME_DIR_ID.fetch_add(1, Ordering::Relaxed)
        );
        let mut scheme_dir = std::env::current_exe()
            .expect("expected current test executable path")
            .parent()
            .expect("expected test executable directory")
            .to_path_buf();
        scheme_dir.push("colors");
        std::fs::create_dir_all(&scheme_dir).expect("expected default colors dir");
        let scheme_file = scheme_dir.join(format!(
            "rterm-default-windows-scheme-{}-{}.toml",
            std::process::id(),
            NEXT_DEFAULT_COLOR_SCHEME_DIR_ID.load(Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            format!(
                r##"
                [metadata]
                name = "{scheme_name}"

                [colors]
                foreground = "#313233"
                background = "#343536"
                cursor_bg = "#373839"
                "##
            ),
        )
        .expect("expected default colors-dir TOML color scheme");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}

            config.color_scheme = '{scheme_name}'

            return config
            "##
        ))
        .expect("expected WezTerm default colors-dir color scheme config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(49, 50, 51));
        assert_eq!(effective.background_color, Color::Rgb(52, 53, 54));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(55, 56, 57));
        let _ = std::fs::remove_file(scheme_file);
        let _ = std::fs::remove_dir(scheme_dir);
    }

    #[test]
    fn static_load_scheme_path_expressions_preserve_assignment_time_values() {
        let source = r#"
            dir = '/first'
            name = 'scheme'
            path = dir .. '/' .. name
            dir = '/second'
            path = path .. '.toml'
            wezterm.color.load_scheme(path)
        "#;
        let call_start = source
            .find("wezterm.color.load_scheme(path)")
            .expect("expected load_scheme call marker");

        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                source, "path", call_start,
            ),
            Some("/first/scheme.toml".to_owned())
        );
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                source,
                "dir .. '/direct.toml'",
                call_start,
            ),
            Some("/second/direct.toml".to_owned())
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_reject_dynamic_multiline_concatenation() {
        let dynamic_multiline_concat = r#"
            path = '/safe'
              .. compute_path()
            wezterm.color.load_scheme(path)
        "#;
        let call_start = dynamic_multiline_concat
            .find("wezterm.color.load_scheme(path)")
            .expect("expected dynamic multiline-concat call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                dynamic_multiline_concat,
                "path",
                call_start,
            ),
            None
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_accept_static_multiline_concatenation() {
        let static_multiline_concat = r#"
            path = '/a'
              .. '/b'
            wezterm.color.load_scheme(path)
        "#;
        let call_start = static_multiline_concat
            .find("wezterm.color.load_scheme(path)")
            .expect("expected static multiline-concat call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                static_multiline_concat,
                "path",
                call_start,
            ),
            Some("/a/b".to_owned())
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_reject_newline_before_assignment_operator() {
        let newline_before_assignment = r#"
            path = '/old'
            path
              = compute_path()
            wezterm.color.load_scheme(path)
        "#;
        let call_start = newline_before_assignment
            .find("wezterm.color.load_scheme(path)")
            .expect("expected newline-before-assignment call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                newline_before_assignment,
                "path",
                call_start,
            ),
            None
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_reject_newline_after_local_shadowing() {
        let newline_after_local = r#"
            path = '/old'
            local
              path
            wezterm.color.load_scheme(path)
        "#;
        let call_start = newline_after_local
            .find("wezterm.color.load_scheme(path)")
            .expect("expected newline-after-local call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                newline_after_local,
                "path",
                call_start,
            ),
            None
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_accept_wezterm_config_dir() {
        let expected = std::env::var("WEZTERM_CONFIG_DIR")
            .ok()
            .map(|config_dir| format!("{}/scheme.toml", config_dir.replace('\\', "/")));
        let call_start = "wezterm.color.load_scheme(wezterm.config_dir .. '/scheme.toml')"
            .find("wezterm.color.load_scheme(wezterm.config_dir .. '/scheme.toml')")
            .expect("expected call marker");

        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                "",
                "wezterm.config_dir .. '/scheme.toml'",
                call_start,
            ),
            expected
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_accept_wezterm_alias_config_dir() {
        let source = "
            local wt = require 'wezterm'
            local _scheme = wt.config_dir .. '/scheme.toml'
            wezterm.color.load_scheme(_scheme)
        ";
        let expected = std::env::var("WEZTERM_CONFIG_DIR")
            .ok()
            .map(|config_dir| format!("{}/scheme.toml", config_dir.replace('\\', "/")));
        let call_start = source
            .find("wezterm.color.load_scheme(_scheme)")
            .expect("expected call marker");

        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                source,
                "wt.config_dir .. '/scheme.toml'",
                call_start,
            ),
            expected
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_accept_wezterm_alias_config_dir_double_quotes() {
        let source = "
            local wt = require \"wezterm\"
            local _scheme = wt.config_dir .. '/scheme.toml'
            wezterm.color.load_scheme(_scheme)
        ";
        let expected = std::env::var("WEZTERM_CONFIG_DIR")
            .ok()
            .map(|config_dir| format!("{}/scheme.toml", config_dir.replace('\\', "/")));
        let call_start = source
            .find("wezterm.color.load_scheme(_scheme)")
            .expect("expected call marker");

        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                source,
                "wt.config_dir .. '/scheme.toml'",
                call_start,
            ),
            expected
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_accept_wezterm_alias_config_dir_parenthesized_require_call()
     {
        let source = "
            local wt = require('wezterm')
            local _scheme = wt.config_dir .. '/scheme.toml'
            wezterm.color.load_scheme(_scheme)
        ";
        let expected = std::env::var("WEZTERM_CONFIG_DIR")
            .ok()
            .map(|config_dir| format!("{}/scheme.toml", config_dir.replace('\\', "/")));
        let call_start = source
            .find("wezterm.color.load_scheme(_scheme)")
            .expect("expected call marker");

        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                source,
                "wt.config_dir .. '/scheme.toml'",
                call_start,
            ),
            expected
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_accept_wezterm_alias_config_dir_aliased_binding() {
        let source = "
            local wt0 = require 'wezterm'
            local wt = wt0
            local _scheme = wt.config_dir .. '/scheme.toml'
            wezterm.color.load_scheme(_scheme)
        ";
        let expected = std::env::var("WEZTERM_CONFIG_DIR")
            .ok()
            .map(|config_dir| format!("{}/scheme.toml", config_dir.replace('\\', "/")));
        let call_start = source
            .find("wezterm.color.load_scheme(_scheme)")
            .expect("expected call marker");

        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                source,
                "wt.config_dir .. '/scheme.toml'",
                call_start,
            ),
            expected
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_reflect_wezterm_config_dir_environment() {
        let call_start = "wezterm.color.load_scheme(wezterm.config_dir .. '/scheme.toml')"
            .find("wezterm.color.load_scheme(wezterm.config_dir .. '/scheme.toml')")
            .expect("expected call marker");
        let expected = std::env::var("WEZTERM_CONFIG_DIR")
            .ok()
            .map(|config_dir| format!("{}/scheme.toml", config_dir.replace('\\', "/")));

        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                "",
                "wezterm.config_dir .. '/scheme.toml'",
                call_start,
            ),
            expected
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_reject_other_multiline_binary_continuations() {
        let dynamic_binary_continuation = r#"
            path = '/safe'
              or compute_path()
            wezterm.color.load_scheme(path)
        "#;
        let call_start = dynamic_binary_continuation
            .find("wezterm.color.load_scheme(path)")
            .expect("expected dynamic binary-continuation call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                dynamic_binary_continuation,
                "path",
                call_start,
            ),
            None
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_reject_multiline_index_continuations() {
        let dynamic_index_continuation = r#"
            base = '/safe'
            path = base
              [dynamic_key]
            wezterm.color.load_scheme(path)
        "#;
        let call_start = dynamic_index_continuation
            .find("wezterm.color.load_scheme(path)")
            .expect("expected dynamic index-continuation call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                dynamic_index_continuation,
                "path",
                call_start,
            ),
            None
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_accept_concat_operator_before_newline() {
        let concat_before_newline = r#"
            dir = '/a'
            name = '/b'
            path = dir ..
              name
            wezterm.color.load_scheme(path)
        "#;
        let call_start = concat_before_newline
            .find("wezterm.color.load_scheme(path)")
            .expect("expected concat-before-newline call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                concat_before_newline,
                "path",
                call_start,
            ),
            Some("/a/b".to_owned())
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_preserve_binding_before_lua_label() {
        let label_after_binding = r#"
            path = '/safe'
            ::keep::
            wezterm.color.load_scheme(path)
        "#;
        let call_start = label_after_binding
            .find("wezterm.color.load_scheme(path)")
            .expect("expected label-after-binding call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                label_after_binding,
                "path",
                call_start,
            ),
            Some("/safe".to_owned())
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_preserve_binding_after_lua_label() {
        let binding_after_label = r#"
            path = '/old'
            ::keep::
            path = '/new'
            wezterm.color.load_scheme(path)
        "#;
        let call_start = binding_after_label
            .find("wezterm.color.load_scheme(path)")
            .expect("expected binding-after-label call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                binding_after_label,
                "path",
                call_start,
            ),
            Some("/new".to_owned())
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_reject_dynamic_shadowing_and_cycles() {
        let dynamically_shadowed = r#"
            path = '/static.toml'
            path = compute_path()
            wezterm.color.load_scheme(path)
        "#;
        let call_start = dynamically_shadowed
            .find("wezterm.color.load_scheme(path)")
            .expect("expected dynamic-shadowing call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                dynamically_shadowed,
                "path",
                call_start,
            ),
            None
        );

        let locally_shadowed = r#"
            path = '/static.toml'
            local path
            wezterm.color.load_scheme(path)
        "#;
        let call_start = locally_shadowed
            .find("wezterm.color.load_scheme(path)")
            .expect("expected local-shadowing call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                locally_shadowed,
                "path",
                call_start,
            ),
            None
        );

        let cyclic = r#"
            first = second
            second = first
            wezterm.color.load_scheme(first)
        "#;
        let call_start = cyclic
            .find("wezterm.color.load_scheme(first)")
            .expect("expected cyclic call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                cyclic, "first", call_start,
            ),
            None
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_reject_local_named_function_shadowing() {
        let local_function_shadowing = r#"
            path = '/old'
            local function path() end
            wezterm.color.load_scheme(path)
        "#;
        let call_start = local_function_shadowing
            .find("wezterm.color.load_scheme(path)")
            .expect("expected local-function-shadowing call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                local_function_shadowing,
                "path",
                call_start,
            ),
            None
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_reject_global_named_function_shadowing() {
        let global_function_shadowing = r#"
            path = '/old'
            function path() end
            wezterm.color.load_scheme(path)
        "#;
        let call_start = global_function_shadowing
            .find("wezterm.color.load_scheme(path)")
            .expect("expected global-function-shadowing call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                global_function_shadowing,
                "path",
                call_start,
            ),
            None
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_preserve_named_field_function_declarations() {
        let dotted_function = r#"
            path = '/safe'
            function path.field() end
            wezterm.color.load_scheme(path)
        "#;
        let call_start = dotted_function
            .find("wezterm.color.load_scheme(path)")
            .expect("expected dotted-function call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                dotted_function,
                "path",
                call_start,
            ),
            Some("/safe".to_owned())
        );

        let method_function = r#"
            path = '/safe'
            function path:method() end
            wezterm.color.load_scheme(path)
        "#;
        let call_start = method_function
            .find("wezterm.color.load_scheme(path)")
            .expect("expected method-function call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                method_function,
                "path",
                call_start,
            ),
            Some("/safe".to_owned())
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_reject_multi_target_direct_bindings() {
        let multi_target_binding = r#"
            path = '/static.toml'
            other.foo, path = first(), second()
            wezterm.color.load_scheme(path)
        "#;
        let call_start = multi_target_binding
            .find("wezterm.color.load_scheme(path)")
            .expect("expected multi-target binding call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                multi_target_binding,
                "path",
                call_start,
            ),
            None
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_reject_comparison_rhs_as_dynamic_binding() {
        let comparison_rhs = r#"
            path = '/static.toml'
            path = enabled == true
            wezterm.color.load_scheme(path)
        "#;
        let call_start = comparison_rhs
            .find("wezterm.color.load_scheme(path)")
            .expect("expected comparison-RHS call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                comparison_rhs,
                "path",
                call_start,
            ),
            None
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_accept_statement_separator_after_binding() {
        let semicolon_terminated = r#"
            path = '/static.toml';
            wezterm.color.load_scheme(path)
        "#;
        let call_start = semicolon_terminated
            .find("wezterm.color.load_scheme(path)")
            .expect("expected semicolon-terminated binding call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                semicolon_terminated,
                "path",
                call_start,
            ),
            Some("/static.toml".to_owned())
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_reject_dynamic_block_comment_continuations() {
        let dynamic_continuation = r#"
            path = '/safe' --[[gap]] .. compute_path()
            wezterm.color.load_scheme(path)
        "#;
        let call_start = dynamic_continuation
            .find("wezterm.color.load_scheme(path)")
            .expect("expected dynamic block-comment continuation call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                dynamic_continuation,
                "path",
                call_start,
            ),
            None
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_reject_comparisons_after_block_comments() {
        let dynamic_comparison = r#"
            path = '/safe' --[[gap]] == compute_path()
            wezterm.color.load_scheme(path)
        "#;
        let call_start = dynamic_comparison
            .find("wezterm.color.load_scheme(path)")
            .expect("expected block-comment comparison call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                dynamic_comparison,
                "path",
                call_start,
            ),
            None
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_accept_trailing_comments() {
        let trailing_comment = r#"
            path = '/safe' -- trailing
            wezterm.color.load_scheme(path)
        "#;
        let call_start = trailing_comment
            .find("wezterm.color.load_scheme(path)")
            .expect("expected trailing-comment call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                trailing_comment,
                "path",
                call_start,
            ),
            Some("/safe".to_owned())
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_accept_trailing_block_comments() {
        let trailing_block_comment = r#"
            path = '/safe' --[[trailing]]
            wezterm.color.load_scheme(path)
        "#;
        let call_start = trailing_block_comment
            .find("wezterm.color.load_scheme(path)")
            .expect("expected trailing block-comment call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                trailing_block_comment,
                "path",
                call_start,
            ),
            Some("/safe".to_owned())
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_preserve_multi_target_field_mutations() {
        let multi_target_field_mutation = r#"
            path = '/static.toml'
            other, path.foo = first(), second()
            wezterm.color.load_scheme(path)
        "#;
        let call_start = multi_target_field_mutation
            .find("wezterm.color.load_scheme(path)")
            .expect("expected multi-target field-mutation call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                multi_target_field_mutation,
                "path",
                call_start,
            ),
            Some("/static.toml".to_owned())
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_preserve_commented_field_mutations() {
        let commented_field_mutation = r#"
            path = '/static.toml'
            path --[[gap]] .foo = compute()
            wezterm.color.load_scheme(path)
        "#;
        let call_start = commented_field_mutation
            .find("wezterm.color.load_scheme(path)")
            .expect("expected commented field-mutation call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                commented_field_mutation,
                "path",
                call_start,
            ),
            Some("/static.toml".to_owned())
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_accept_comments_around_concat_operator() {
        let commented_concat = r#"
            path = '/a' --[[left]] .. --[[right]] '/b'
            wezterm.color.load_scheme(path)
        "#;
        let call_start = commented_concat
            .find("wezterm.color.load_scheme(path)")
            .expect("expected commented-concat call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                commented_concat,
                "path",
                call_start,
            ),
            Some("/a/b".to_owned())
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_reject_unicode_comment_prefixed_dynamic_binding() {
        let comment_prefixed_binding = r#"
            path = '/old'
            other = 1
            --[[中文]] path = compute_path()
            wezterm.color.load_scheme(path)
        "#;
        let call_start = comment_prefixed_binding
            .find("wezterm.color.load_scheme(path)")
            .expect("expected Unicode-comment-prefixed binding call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                comment_prefixed_binding,
                "path",
                call_start,
            ),
            None
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_accept_line_comment_concat_continuations() {
        let line_comment_concat = r#"
            path = '/a' --left
              .. --right
              '/b'
            wezterm.color.load_scheme(path)
        "#;
        let call_start = line_comment_concat
            .find("wezterm.color.load_scheme(path)")
            .expect("expected line-comment concat call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                line_comment_concat,
                "path",
                call_start,
            ),
            Some("/a/b".to_owned())
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_reject_line_comment_dynamic_continuations() {
        let line_comment_dynamic = r#"
            path = '/safe' --gap
              == compute_path()
            wezterm.color.load_scheme(path)
        "#;
        let call_start = line_comment_dynamic
            .find("wezterm.color.load_scheme(path)")
            .expect("expected line-comment dynamic call marker");
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                line_comment_dynamic,
                "path",
                call_start,
            ),
            None
        );
    }

    #[test]
    fn static_load_scheme_path_expressions_preserve_comment_text_inside_string_literals() {
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                "",
                "'/a--[[quoted]]/b'",
                0,
            ),
            Some("/a--[[quoted]]/b".to_owned())
        );
        assert_eq!(
            super::lua_static_load_scheme_path_expression_value_from_query(
                "",
                "[=[/a--[[long]]/b]=]",
                0,
            ),
            Some("/a--[[long]]/b".to_owned())
        );
    }

    #[test]
    fn default_colors_call_resolver_accepts_static_zero_argument_forms() {
        let cases = [
            (
                "canonical call",
                r#"
                    config.colors = wezterm.color.get_default_colors()
                    local after = true
                "#,
                "wezterm.color.get_default_colors()",
            ),
            (
                "explicit wezterm binding",
                r#"
                    local wezterm = require 'wezterm'
                    config.colors = wezterm.color.get_default_colors()
                "#,
                "wezterm.color.get_default_colors()",
            ),
            (
                "direct require receiver",
                "config.colors = require('wezterm').color.get_default_colors()",
                "require('wezterm').color.get_default_colors()",
            ),
            (
                "parenthesized require receiver",
                "config.colors = (require 'wezterm').color.get_default_colors()",
                "(require 'wezterm').color.get_default_colors()",
            ),
            (
                "const module alias",
                r#"
                    local wt <const> = require 'wezterm'
                    config.colors = wt.color.get_default_colors()
                "#,
                "wt.color.get_default_colors()",
            ),
            (
                "module identity alias chain",
                r#"
                    local wt = require 'wezterm'
                    local module_alias = wt
                    config.colors = module_alias.color.get_default_colors()
                "#,
                "module_alias.color.get_default_colors()",
            ),
            (
                "static field keys",
                r#"
                    local wt = require 'wezterm'
                    local color_key = 'color'
                    local default_key = 'get_default_colors'
                    config.colors = wt[color_key][default_key]()
                "#,
                "wt[color_key][default_key]()",
            ),
            (
                "function alias",
                r#"
                    local get_defaults = wezterm.color.get_default_colors
                    config.colors = get_defaults()
                "#,
                "get_defaults()",
            ),
            (
                "direct require function alias",
                r#"
                    local get_defaults = require('wezterm').color.get_default_colors
                    config.colors = get_defaults()
                "#,
                "get_defaults()",
            ),
            (
                "color namespace alias",
                r#"
                    local color = wezterm.color
                    config.colors = color.get_default_colors()
                "#,
                "color.get_default_colors()",
            ),
            (
                "parenthesized color namespace",
                "config.colors = (wezterm.color).get_default_colors()",
                "(wezterm.color).get_default_colors()",
            ),
            (
                "parenthesized require color namespace",
                "config.colors = (require('wezterm').color).get_default_colors()",
                "(require('wezterm').color).get_default_colors()",
            ),
            (
                "parenthesized color namespace alias",
                r#"
                    local color = wezterm.color
                    config.colors = (color).get_default_colors()
                "#,
                "(color).get_default_colors()",
            ),
            (
                "call-site function binding",
                r#"
                    local get_defaults = wezterm.color.get_default_colors
                    local palette = get_defaults()
                    get_defaults = choose_palette
                    config.colors = palette
                "#,
                "get_defaults()",
            ),
            (
                "function capture survives namespace rebind",
                r#"
                    local color = wezterm.color
                    local get_defaults = color.get_default_colors
                    color = choose_color_namespace()
                    config.colors = get_defaults()
                "#,
                "get_defaults()",
            ),
            (
                "comments-only arguments",
                "config.colors = wezterm.color.get_default_colors(--[[ no arguments ]])",
                "wezterm.color.get_default_colors(--[[ no arguments ]])",
            ),
        ];

        for (label, source, marker) in cases {
            let query_start = source
                .find(marker)
                .expect("expected default-colors call marker");
            assert_eq!(
                super::lua_wezterm_default_colors_from_query_with_static_source(
                    source,
                    &source[query_start..],
                ),
                Some(()),
                "case was {label:?}: {source:?}"
            );
        }

        for (label, source) in [
            ("end of input", "wezterm.color.get_default_colors()"),
            (
                "semicolon boundary",
                "wezterm.color.get_default_colors(); next_call()",
            ),
            (
                "table comma boundary",
                "wezterm.color.get_default_colors(), next_field = true",
            ),
            (
                "table close boundary",
                "wezterm.color.get_default_colors() }",
            ),
            (
                "newline statement boundary",
                "wezterm.color.get_default_colors()\nnext_call()",
            ),
            (
                "label statement boundary",
                "wezterm.color.get_default_colors() ::next:: next_call()",
            ),
        ] {
            assert_eq!(
                super::lua_wezterm_default_colors_from_query_with_static_source(source, source),
                Some(()),
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn default_colors_call_resolver_rejects_dynamic_or_nonexact_forms() {
        let direct_cases = [
            ("one argument", "wezterm.color.get_default_colors(nil)"),
            (
                "multiple arguments",
                "wezterm.color.get_default_colors(nil, false)",
            ),
            ("method call", "wezterm.color:get_default_colors()"),
            (
                "missing close parenthesis",
                "wezterm.color.get_default_colors(",
            ),
            ("wrong function", "wezterm.color.get_default_colors_extra()"),
        ];
        for (label, source) in direct_cases {
            assert_eq!(
                super::lua_wezterm_default_colors_from_query_with_static_source(source, source),
                None,
                "case was {label:?}: {source:?}"
            );
        }

        for (label, tail) in [
            ("field access", ".background"),
            ("index access", "['background']"),
            ("second call", "()"),
            ("method call tail", ":clone()"),
            ("concatenation", " .. suffix"),
            ("arithmetic", " + suffix"),
            ("comparison", " == suffix"),
            ("logical and", " and suffix"),
            ("logical or", " or {}"),
            ("block-comment field", " --[[gap]] .background"),
            ("line-comment index", " -- gap\n ['background']"),
        ] {
            let source = format!("wezterm.color.get_default_colors(){tail}");
            assert_eq!(
                super::lua_wezterm_default_colors_from_query_with_static_source(&source, &source),
                None,
                "case was {label:?}: {source:?}"
            );
        }

        for (label, source, marker) in [
            (
                "dynamic getter key",
                r#"
                    local key = choose_key()
                    config.colors = wezterm.color[key]()
                "#,
                "wezterm.color[key]()",
            ),
            (
                "function alias rebound before call",
                r#"
                    local get_defaults = wezterm.color.get_default_colors
                    get_defaults = choose_palette
                    config.colors = get_defaults()
                "#,
                "get_defaults()",
            ),
            (
                "module alias rebound before call",
                r#"
                    local wt = require 'wezterm'
                    wt = choose_module()
                    config.colors = wt.color.get_default_colors()
                "#,
                "wt.color.get_default_colors()",
            ),
            (
                "module alias rebound before return-table call",
                r#"
                    local wt = require 'wezterm'
                    wt = choose_module()
                    return {
                      colors = wt.color.get_default_colors(),
                    }
                "#,
                "wt.color.get_default_colors()",
            ),
            (
                "color namespace rebound before call",
                r#"
                    local color = wezterm.color
                    color = choose_color_namespace()
                    config.colors = color.get_default_colors()
                "#,
                "color.get_default_colors()",
            ),
            (
                "shadowed require",
                r#"
                    local require = choose_loader()
                    config.colors = require('wezterm').color.get_default_colors()
                "#,
                "require('wezterm').color.get_default_colors()",
            ),
            (
                "shadowed wezterm",
                r#"
                    local wezterm = choose_module()
                    config.colors = wezterm.color.get_default_colors()
                "#,
                "wezterm.color.get_default_colors()",
            ),
            (
                "direct getter field replacement",
                r#"
                    wezterm.color.get_default_colors = choose_palette
                    config.colors = wezterm.color.get_default_colors()
                "#,
                "wezterm.color.get_default_colors()",
            ),
            (
                "module alias color field replacement",
                r#"
                    local wt = require 'wezterm'
                    wt.color = choose_color_namespace()
                    config.colors = wt.color.get_default_colors()
                "#,
                "wt.color.get_default_colors()",
            ),
            (
                "module identity alias color field replacement",
                r#"
                    local wt = require 'wezterm'
                    local module_alias = wt
                    module_alias.color = choose_color_namespace()
                    config.colors = wt.color.get_default_colors()
                "#,
                "wt.color.get_default_colors()",
            ),
            (
                "color namespace getter replacement",
                r#"
                    local color = wezterm.color
                    color.get_default_colors = choose_palette
                    config.colors = color.get_default_colors()
                "#,
                "color.get_default_colors()",
            ),
            (
                "static-key getter replacement",
                r#"
                    local color = wezterm.color
                    local getter = 'get_default_colors'
                    color[getter] = choose_palette
                    config.colors = color.get_default_colors()
                "#,
                "color.get_default_colors()",
            ),
            (
                "dynamic module field replacement",
                r#"
                    local wt = require 'wezterm'
                    local field = choose_field()
                    wt[field] = choose_value()
                    config.colors = wt.color.get_default_colors()
                "#,
                "wt.color.get_default_colors()",
            ),
            (
                "dynamic color namespace field replacement",
                r#"
                    local color = wezterm.color
                    local field = choose_field()
                    color[field] = choose_value()
                    config.colors = color.get_default_colors()
                "#,
                "color.get_default_colors()",
            ),
            (
                "named getter replacement",
                r#"
                    function wezterm.color.get_default_colors()
                      return {}
                    end
                    config.colors = wezterm.color.get_default_colors()
                "#,
                "wezterm.color.get_default_colors()",
            ),
            (
                "named method getter replacement",
                r#"
                    local color = wezterm.color
                    function color:get_default_colors()
                      return {}
                    end
                    config.colors = color.get_default_colors()
                "#,
                "color.get_default_colors()",
            ),
        ] {
            let query_start = source
                .rfind(marker)
                .expect("expected rejected default-colors marker");
            assert_eq!(
                super::lua_wezterm_default_colors_from_query_with_static_source(
                    source,
                    &source[query_start..],
                ),
                None,
                "case was {label:?}: {source:?}"
            );
        }
    }

    fn assert_wezterm_default_colors_config_overrides(overrides: &NativeConfigSnapshot) {
        let expected = super::native_wezterm_default_colors_palette();
        let expected_ansi = std::array::from_fn(|index| {
            if index < 8 {
                expected.ansi[index]
            } else {
                expected.brights[index - 8]
            }
        });

        assert_eq!(overrides.foreground_color, Some(expected.foreground));
        assert_eq!(overrides.background_color, Some(expected.background));
        assert_eq!(overrides.cursor_fg_color, expected.cursor_fg);
        assert_eq!(overrides.cursor_bg_color, Some(expected.cursor_bg));
        assert_eq!(overrides.cursor_border_color, expected.cursor_border);
        assert_eq!(overrides.selection_fg_color, Some(None));
        assert_eq!(overrides.selection_bg_color, expected.selection_bg);
        assert_eq!(overrides.scrollbar_thumb_color, expected.scrollbar_thumb);
        assert_eq!(overrides.split_color, expected.split);
        assert_eq!(overrides.ansi_palette, Some(expected_ansi));

        let indexed = overrides
            .indexed_palette
            .as_ref()
            .expect("expected all WezTerm indexed colors");
        assert_eq!(indexed, &expected.indexed);
        assert!(indexed[..16].iter().all(Option::is_none));
        assert_eq!(
            indexed[16..].iter().filter(|color| color.is_some()).count(),
            240
        );

        assert_eq!(overrides.compose_cursor_color, None);
        assert_eq!(overrides.visual_bell_color, None);
        assert_eq!(overrides.tab_bar_background_color, None);
        assert_eq!(overrides.tab_bar_inactive_tab_edge_color, None);
        assert_eq!(
            overrides.tab_bar_active_tab_colors,
            NativeTabBarItemColors::default()
        );
        assert_eq!(overrides.copy_mode_active_highlight_fg, None);
        assert_eq!(overrides.copy_mode_active_highlight_bg, None);
        assert_eq!(overrides.quick_select_label_fg, None);
        assert_eq!(overrides.quick_select_label_bg, None);

        let colors = overrides
            .colors
            .as_ref()
            .expect("expected config.colors source palette");
        assert_eq!(colors.foreground, Some(expected.foreground));
        assert_eq!(colors.background, Some(expected.background));
        assert_eq!(colors.cursor_fg, expected.cursor_fg);
        assert_eq!(colors.cursor_bg, Some(expected.cursor_bg));
        assert_eq!(colors.cursor_border, expected.cursor_border);
        assert_eq!(colors.selection_fg, Some(None));
        assert_eq!(colors.selection_bg, expected.selection_bg);
        assert_eq!(colors.ansi, Some(expected.ansi));
        assert_eq!(colors.brights, Some(expected.brights));
        assert_eq!(colors.indexed, expected.indexed);
        assert_eq!(
            colors.indexed[16..]
                .iter()
                .filter(|color| color.is_some())
                .count(),
            240
        );
        assert_eq!(colors.scrollbar_thumb, expected.scrollbar_thumb);
        assert_eq!(colors.split, expected.split);
        assert_eq!(colors.compose_cursor, None);
        assert_eq!(colors.visual_bell, None);
        assert_eq!(colors.tab_bar_background, None);
        assert_eq!(colors.tab_bar_inactive_tab_edge, None);
        assert_eq!(colors.tab_bar_active_tab, NativeTabBarItemColors::default());
        assert_eq!(colors.copy_mode_active_highlight_fg, None);
        assert_eq!(colors.copy_mode_active_highlight_bg, None);
        assert_eq!(colors.quick_select_label_fg, None);
        assert_eq!(colors.quick_select_label_bg, None);
    }

    #[test]
    fn window_app_loads_wezterm_default_colors_directly() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
                local wezterm = require 'wezterm'
                local config = wezterm.config_builder()
                config.colors = wezterm.color.get_default_colors()
                return config
            "#,
        )
        .expect("expected direct WezTerm default colors config");

        assert_wezterm_default_colors_config_overrides(&overrides);
    }

    #[test]
    fn window_app_loads_wezterm_default_colors_through_static_aliases() {
        let cases = [
            (
                "module alias in return table",
                r#"
                    local wt = require 'wezterm'
                    return {
                        colors = wt.color.get_default_colors(),
                    }
                "#,
            ),
            (
                "color namespace alias",
                r#"
                    local wezterm = require 'wezterm'
                    local config = wezterm.config_builder()
                    local color = wezterm.color
                    config.colors = color.get_default_colors()
                    return config
                "#,
            ),
            (
                "function alias",
                r#"
                    local wezterm = require 'wezterm'
                    local config = wezterm.config_builder()
                    local get_defaults = wezterm.color.get_default_colors
                    config.colors = get_defaults()
                    return config
                "#,
            ),
            (
                "static key aliases",
                r#"
                    local wezterm = require 'wezterm'
                    local config = wezterm.config_builder()
                    local color_key = 'color'
                    local getter_key = 'get_default_colors'
                    config.colors = wezterm[color_key][getter_key]()
                    return config
                "#,
            ),
        ];

        for (label, source) in cases {
            let overrides = super::native_config_overrides_from_wezterm_lua_config(source)
                .unwrap_or_else(|| panic!("expected static alias config for {label}"));
            assert_wezterm_default_colors_config_overrides(&overrides);
        }
    }

    #[test]
    fn window_app_reduces_wezterm_default_color_mutations_in_source_order() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
                local wezterm = require 'wezterm'
                local config = {}
                local colors = wezterm.color.get_default_colors()

                colors.background = '#010203'
                colors.background = '#040506'

                colors.indexed[16] = '#070809'
                colors.indexed = {
                  [17] = '#0a0b0c',
                }
                colors.indexed[18] = '#0d0e0f'

                colors.ansi[1] = '#101112'
                colors.ansi = {
                  '#131415', '#161718', '#191a1b', '#1c1d1e',
                  '#1f2021', '#222324', '#252627', '#28292a',
                }
                colors.ansi[2] = '#2b2c2d'

                config.colors = colors
                return config
            "##,
        )
        .expect("expected ordered WezTerm default-color mutations");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.background_color, Color::Rgb(4, 5, 6));

        let indexed = effective
            .indexed_palette
            .expect("expected replaced indexed palette");
        assert_eq!(indexed[16], None);
        assert_eq!(indexed[17], Some(Color::Rgb(10, 11, 12)));
        assert_eq!(indexed[18], Some(Color::Rgb(13, 14, 15)));
        assert_eq!(indexed[19], None);

        let ansi = effective.ansi_palette.expect("expected ANSI palette");
        assert_eq!(ansi[0], Color::Rgb(19, 20, 21));
        assert_eq!(ansi[1], Color::Rgb(43, 44, 45));
        assert_eq!(ansi[2], Color::Rgb(25, 26, 27));
    }

    #[test]
    fn window_app_resets_wezterm_default_color_mutations_on_fresh_binding() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
                local wezterm = require 'wezterm'
                local config = {}
                local colors = wezterm.color.get_default_colors()

                colors.background = '#010203'
                colors.indexed = {}
                colors.ansi = {
                  '#101112', '#131415', '#161718', '#191a1b',
                  '#1c1d1e', '#1f2021', '#222324', '#252627',
                }

                colors = wezterm.color.get_default_colors()
                colors.cursor_bg = '#040506'
                colors.indexed[16] = '#070809'

                config.colors = colors
                return config
            "##,
        )
        .expect("expected fresh WezTerm default-color replacement");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.background_color, Color::Rgb(0, 0, 0));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(4, 5, 6));

        let ansi = effective.ansi_palette.expect("expected ANSI palette");
        assert_eq!(ansi[0], Color::Rgb(0, 0, 0));
        assert_eq!(ansi[1], Color::Rgb(0xcc, 0x55, 0x55));

        let indexed = effective.indexed_palette.expect("expected indexed palette");
        assert_eq!(indexed[16], Some(Color::Rgb(7, 8, 9)));
        assert_eq!(indexed[17], Some(Color::Rgb(0, 0, 0x5f)));
        assert_eq!(indexed[255], Some(Color::Rgb(0xee, 0xee, 0xee)));
    }

    #[test]
    fn window_app_uses_latest_wezterm_default_color_binding_before_reference() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
                local wezterm = require 'wezterm'
                local config = {}
                local colors = {
                  foreground = '#010203',
                  background = '#040506',
                }

                colors = wezterm.color.get_default_colors()
                colors.foreground = '#070809'
                config.colors = colors

                colors = wezterm.color.get_default_colors()
                colors.foreground = '#0a0b0c'
                return config
            "##,
        )
        .expect("expected latest default-color binding before config.colors");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(7, 8, 9));
        assert_eq!(effective.background_color, Color::Rgb(0, 0, 0));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(0x52, 0xad, 0x70));
    }

    #[test]
    fn window_app_rejects_dynamic_wezterm_default_color_identity() {
        for (label, statement) in [
            (
                "palette alias escape",
                "local alias = colors\nalias.background = '#010203'",
            ),
            (
                "dynamic indexed key",
                "local slot = choose_index()\ncolors.indexed[slot] = '#040506'",
            ),
        ] {
            let source = format!(
                r##"
                    local wezterm = require 'wezterm'
                    local config = {{}}
                    local colors = wezterm.color.get_default_colors()

                    {statement}
                    config.colors = colors
                    return config
                "##,
            );

            assert!(
                super::native_config_overrides_from_wezterm_lua_config(&source).is_none(),
                "{label} must fail closed"
            );
        }
    }

    #[test]
    fn window_app_uses_wezterm_default_colors_in_inline_custom_color_scheme() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
                local wezterm = require 'wezterm'
                local config = {}
                config.color_schemes = {
                  ['Default Copy'] = wezterm.color.get_default_colors(),
                }
                config.color_scheme = 'Default Copy'
                return config
            "#,
        )
        .expect("expected inline custom scheme from WezTerm default colors");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        let scheme = effective
            .color_schemes
            .get("Default Copy")
            .expect("expected inline default-copy scheme");
        assert_eq!(scheme, &super::native_wezterm_default_colors_palette());
        assert_eq!(effective.resolved_palette, *scheme);
    }

    #[test]
    fn window_app_uses_wezterm_default_colors_in_direct_custom_scheme_assignment() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
                local wt = require 'wezterm'
                local config = wt.config_builder()
                local color = wt.color
                local get_defaults = color.get_default_colors
                config.color_schemes['Default Copy'] = get_defaults()
                config.color_scheme = 'Default Copy'
                return config
            "#,
        )
        .expect("expected direct custom scheme from aliased WezTerm default colors");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        let scheme = effective
            .color_schemes
            .get("Default Copy")
            .expect("expected directly assigned default-copy scheme");
        assert_eq!(scheme, &super::native_wezterm_default_colors_palette());
        assert_eq!(scheme.selection_fg, Some(None));
        assert_eq!(
            scheme.indexed[16..]
                .iter()
                .filter(|color| color.is_some())
                .count(),
            240
        );
        assert_eq!(effective.resolved_palette, *scheme);
    }

    #[test]
    fn window_app_mutates_wezterm_default_colors_in_custom_color_scheme() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
                local wezterm = require 'wezterm'
                local config = {}
                local get_defaults = wezterm.color.get_default_colors
                local scheme = get_defaults()
                scheme.indexed[249] = '#010203'

                config.color_schemes = {
                  ['Default Copy'] = scheme,
                }
                config.color_schemes['Default Copy'].selection_bg = 'rgba(4,5,6,0.5)'
                config.color_scheme = 'Default Copy'
                return config
            "##,
        )
        .expect("expected mutated custom scheme from WezTerm default colors");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        let scheme = effective
            .color_schemes
            .get("Default Copy")
            .expect("expected mutated default-copy scheme");
        let mut expected = super::native_wezterm_default_colors_palette();
        expected.indexed[249] = Some(Color::Rgb(1, 2, 3));
        expected.selection_bg = Some(Color::Rgba(4, 5, 6, 127));
        assert_eq!(scheme, &expected);
        assert_eq!(scheme.selection_fg, Some(None));
        assert_eq!(scheme.indexed[16], Some(Color::Rgb(0, 0, 0)));
        assert_eq!(scheme.indexed[255], Some(Color::Rgb(238, 238, 238)));
        assert_eq!(
            scheme.indexed[16..]
                .iter()
                .filter(|color| color.is_some())
                .count(),
            240
        );
        assert_eq!(effective.resolved_palette, *scheme);
    }

    #[test]
    fn builtin_scheme_lookup_resolver_accepts_supported_forms_at_original_lookup_offset() {
        let cases = [
            (
                "canonical modern call",
                r#"
                    local scheme_name = 'Gruvbox Light'
                    config.colors = wezterm.color.get_builtin_schemes()[scheme_name]
                    scheme_name = 'Tokyo Night'
                "#,
                "wezterm.color.get_builtin_schemes()[scheme_name]",
            ),
            (
                "explicit wezterm require binding",
                r#"
                    local wezterm = require 'wezterm'
                    local scheme_name = 'Gruvbox Light'
                    config.colors = wezterm.color.get_builtin_schemes()[scheme_name]
                    scheme_name = 'Tokyo Night'
                "#,
                "wezterm.color.get_builtin_schemes()[scheme_name]",
            ),
            (
                "legacy call",
                r#"
                    local scheme_name = 'Gruvbox Light'
                    config.colors = wezterm.get_builtin_color_schemes()[scheme_name]
                    scheme_name = 'Tokyo Night'
                "#,
                "wezterm.get_builtin_color_schemes()[scheme_name]",
            ),
            (
                "direct require receiver",
                r#"
                    local scheme_name = 'Gruvbox Light'
                    config.colors = require('wezterm').color.get_builtin_schemes()[scheme_name]
                    scheme_name = 'Tokyo Night'
                "#,
                "require('wezterm').color.get_builtin_schemes()[scheme_name]",
            ),
            (
                "parenthesized require receiver",
                r#"
                    local scheme_name = 'Gruvbox Light'
                    config.colors = (require 'wezterm').color.get_builtin_schemes()[scheme_name]
                    scheme_name = 'Tokyo Night'
                "#,
                "(require 'wezterm').color.get_builtin_schemes()[scheme_name]",
            ),
            (
                "module alias call",
                r#"
                    local wt = require 'wezterm'
                    local scheme_name = 'Gruvbox Light'
                    config.colors = wt.color.get_builtin_schemes()[scheme_name]
                    scheme_name = 'Tokyo Night'
                "#,
                "wt.color.get_builtin_schemes()[scheme_name]",
            ),
            (
                "parenthesized require module alias call",
                r#"
                    local wt = (require 'wezterm')
                    local scheme_name = 'Gruvbox Light'
                    config.colors = wt.color.get_builtin_schemes()[scheme_name]
                    scheme_name = 'Tokyo Night'
                "#,
                "wt.color.get_builtin_schemes()[scheme_name]",
            ),
            (
                "static field keys",
                r#"
                    local wt = require 'wezterm'
                    local color_key = 'color'
                    local getter_key = 'get_builtin_schemes'
                    local scheme_name = 'Gruvbox Light'
                    config.colors = wt[color_key][getter_key]()[scheme_name]
                    scheme_name = 'Tokyo Night'
                "#,
                "wt[color_key][getter_key]()[scheme_name]",
            ),
            (
                "legacy static field key",
                r#"
                    local wt = require 'wezterm'
                    local getter_key = 'get_builtin_color_schemes'
                    local scheme_name = 'Gruvbox Light'
                    config.colors = wt[getter_key]()[scheme_name]
                    scheme_name = 'Tokyo Night'
                "#,
                "wt[getter_key]()[scheme_name]",
            ),
            (
                "modern function alias",
                r#"
                    local wt = require 'wezterm'
                    local get_schemes = wt.color.get_builtin_schemes
                    local scheme_name = 'Gruvbox Light'
                    config.colors = get_schemes()[scheme_name]
                    scheme_name = 'Tokyo Night'
                "#,
                "get_schemes()[scheme_name]",
            ),
            (
                "legacy function alias",
                r#"
                    local get_schemes = wezterm.get_builtin_color_schemes
                    local scheme_name = 'Gruvbox Light'
                    config.colors = get_schemes()[scheme_name]
                    scheme_name = 'Tokyo Night'
                "#,
                "get_schemes()[scheme_name]",
            ),
            (
                "direct require function alias",
                r#"
                    local get_schemes = require('wezterm').color.get_builtin_schemes
                    local scheme_name = 'Gruvbox Light'
                    config.colors = get_schemes()[scheme_name]
                    scheme_name = 'Tokyo Night'
                "#,
                "get_schemes()[scheme_name]",
            ),
            (
                "parenthesized require function alias",
                r#"
                    local get_schemes = (require 'wezterm').color.get_builtin_schemes
                    local scheme_name = 'Gruvbox Light'
                    config.colors = get_schemes()[scheme_name]
                    scheme_name = 'Tokyo Night'
                "#,
                "get_schemes()[scheme_name]",
            ),
            (
                "long bracket literal key",
                r#"
                    config.colors = wezterm.color.get_builtin_schemes()[[=[Gruvbox Light]=]]
                    local scheme_name = 'Tokyo Night'
                "#,
                "wezterm.color.get_builtin_schemes()[[=[Gruvbox Light]=]]",
            ),
        ];

        for (label, source, marker) in cases {
            let query_start = source
                .find(marker)
                .expect("expected built-in scheme lookup marker");
            assert_eq!(
                super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                    source,
                    &source[query_start..],
                )
                .as_deref(),
                Some("Gruvbox Light"),
                "case was {label:?}"
            );
        }
    }

    #[test]
    fn builtin_scheme_lookup_resolver_bounds_receiver_recursion() {
        let excessive_depth = super::LUA_STATIC_LOAD_SCHEME_PATH_MAX_DEPTH + 2;

        let mut self_captures = String::from("local wezterm = require 'wezterm'\n");
        for _ in 0..excessive_depth {
            self_captures.push_str("local wezterm = wezterm\n");
        }
        let marker = "wezterm.color.get_builtin_schemes()['Gruvbox Light']";
        self_captures.push_str("config.colors = ");
        self_captures.push_str(marker);
        let query_start = self_captures
            .rfind(marker)
            .expect("expected excessive self-capture lookup marker");
        assert_eq!(
            super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                &self_captures,
                &self_captures[query_start..],
            ),
            None,
            "excessive reserved receiver self-captures must fail closed"
        );

        let parenthesized_receiver = format!(
            "{}require('wezterm'){}",
            "(".repeat(excessive_depth),
            ")".repeat(excessive_depth)
        );
        let parenthesized =
            format!("{parenthesized_receiver}.color.get_builtin_schemes()['Gruvbox Light']");
        assert_eq!(
            super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                &parenthesized,
                &parenthesized,
            ),
            None,
            "excessive parenthesized receivers must fail closed"
        );

        for (label, source, marker) in [
            (
                "one self-capture",
                r#"
                    local wezterm = require 'wezterm'
                    local wezterm = wezterm
                    config.colors = wezterm.color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "wezterm.color.get_builtin_schemes()['Gruvbox Light']",
            ),
            (
                "one parenthesized require receiver",
                "(require('wezterm')).color.get_builtin_schemes()['Gruvbox Light']",
                "(require('wezterm')).color.get_builtin_schemes()['Gruvbox Light']",
            ),
        ] {
            let query_start = source
                .rfind(marker)
                .expect("expected normal-depth receiver lookup marker");
            assert_eq!(
                super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                    source,
                    &source[query_start..],
                )
                .as_deref(),
                Some("Gruvbox Light"),
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn builtin_scheme_lookup_resolver_rejects_invalid_or_unprovable_lookups() {
        let cases = [
            (
                "one argument",
                "wezterm.color.get_builtin_schemes(true)['Gruvbox Light']",
                "wezterm.color.get_builtin_schemes(true)['Gruvbox Light']",
            ),
            (
                "multiple arguments",
                "wezterm.color.get_builtin_schemes(nil, false)['Gruvbox Light']",
                "wezterm.color.get_builtin_schemes(nil, false)['Gruvbox Light']",
            ),
            (
                "missing close parenthesis",
                "wezterm.color.get_builtin_schemes(['Gruvbox Light']",
                "wezterm.color.get_builtin_schemes(['Gruvbox Light']",
            ),
            (
                "missing close bracket",
                "wezterm.color.get_builtin_schemes()['Gruvbox Light'",
                "wezterm.color.get_builtin_schemes()['Gruvbox Light'",
            ),
            (
                "missing key lookup",
                "wezterm.color.get_builtin_schemes()",
                "wezterm.color.get_builtin_schemes()",
            ),
            (
                "unknown built-in name",
                "wezterm.color.get_builtin_schemes()['Definitely Not Built In']",
                "wezterm.color.get_builtin_schemes()['Definitely Not Built In']",
            ),
            (
                "wrong-case modern function name",
                "wezterm.color.Get_builtin_schemes()['Gruvbox Light']",
                "wezterm.color.Get_builtin_schemes()['Gruvbox Light']",
            ),
            (
                "dynamic inline key",
                "wezterm.color.get_builtin_schemes()[choose_scheme()]",
                "wezterm.color.get_builtin_schemes()[choose_scheme()]",
            ),
            (
                "dynamically shadowed key",
                r#"
                    local scheme_name = 'Gruvbox Light'
                    scheme_name = choose_scheme()
                    config.colors = wezterm.color.get_builtin_schemes()[scheme_name]
                "#,
                "wezterm.color.get_builtin_schemes()[scheme_name]",
            ),
            (
                "dynamically shadowed static getter key",
                r#"
                    local getter_key = 'get_builtin_schemes'
                    getter_key = choose_getter_key()
                    config.colors = wezterm.color[getter_key]()['Gruvbox Light']
                "#,
                "wezterm.color[getter_key]()['Gruvbox Light']",
            ),
            (
                "whole map",
                "local schemes = wezterm.color.get_builtin_schemes()",
                "wezterm.color.get_builtin_schemes()",
            ),
        ];

        for (label, source, marker) in cases {
            let query_start = source
                .find(marker)
                .expect("expected rejected built-in lookup marker");
            assert_eq!(
                super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                    source,
                    &source[query_start..],
                ),
                None,
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn builtin_scheme_lookup_resolver_rejects_expression_continuation_tails() {
        for (label, tail) in [
            ("second key lookup", "['Gruvbox Light']"),
            ("field access", ".background"),
            ("parenthesized call", "()"),
            ("method call", ":clone()"),
            ("concat operator", " .. suffix"),
            ("arithmetic operator", " + suffix"),
            ("comparison operator", " == suffix"),
            ("logical operator", " and suffix"),
            ("block-comment field access", " --[[gap]] .background"),
            ("line-comment index access", " -- gap\n ['Gruvbox Light']"),
            ("line-comment call", " -- gap\n ()"),
            ("line-comment operator", " -- gap\n .. suffix"),
        ] {
            let source = format!("wezterm.color.get_builtin_schemes()['Gruvbox Light']{tail}");
            assert_eq!(
                super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                    &source, &source,
                ),
                None,
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn builtin_scheme_lookup_resolver_normalizers_preserve_continuation_tails() {
        let cases = [
            (
                "module receiver field continuation",
                r#"
                    local wt = require 'wezterm'
                    config.colors = wt.color.get_builtin_schemes()['Gruvbox Light'].background
                "#,
                "wt.color.get_builtin_schemes()['Gruvbox Light'].background",
            ),
            (
                "legacy static-key call continuation",
                r#"
                    local wt = require 'wezterm'
                    local getter_key = 'get_builtin_color_schemes'
                    config.colors = wt[getter_key]()['Gruvbox Light']()
                "#,
                "wt[getter_key]()['Gruvbox Light']()",
            ),
            (
                "function alias hidden index continuation",
                r#"
                    local get_schemes = wezterm.color.get_builtin_schemes
                    config.colors = get_schemes()['Gruvbox Light'] -- gap
                      ['Gruvbox Light']
                "#,
                "get_schemes()['Gruvbox Light']",
            ),
        ];

        for (label, source, marker) in cases {
            let query_start = source
                .find(marker)
                .expect("expected normalized continuation lookup marker");
            assert_eq!(
                super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                    source,
                    &source[query_start..],
                ),
                None,
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn builtin_scheme_lookup_resolver_accepts_value_end_tails() {
        for (label, source) in [
            (
                "end of input",
                "wezterm.color.get_builtin_schemes()['Gruvbox Light']",
            ),
            (
                "comments-only empty argument list",
                "wezterm.color.get_builtin_schemes(--[[ no arguments ]])['Gruvbox Light']",
            ),
            (
                "semicolon terminator",
                "wezterm.color.get_builtin_schemes()['Gruvbox Light']; next_call()",
            ),
            (
                "table comma terminator",
                "wezterm.color.get_builtin_schemes()['Gruvbox Light'], next_field = true",
            ),
            (
                "table close terminator",
                "wezterm.color.get_builtin_schemes()['Gruvbox Light'] }",
            ),
            (
                "newline statement boundary",
                "wezterm.color.get_builtin_schemes()['Gruvbox Light']\nnext_call()",
            ),
            (
                "label statement boundary",
                "wezterm.color.get_builtin_schemes()['Gruvbox Light'] ::next:: next_call()",
            ),
        ] {
            assert_eq!(
                super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                    source, source,
                )
                .as_deref(),
                Some("Gruvbox Light"),
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn builtin_scheme_lookup_resolver_invalidates_rebound_function_aliases() {
        for (label, source) in [
            (
                "dynamic reassignment",
                r#"
                    local get_schemes = wezterm.color.get_builtin_schemes
                    get_schemes = choose_getter()
                    config.colors = get_schemes()['Gruvbox Light']
                "#,
            ),
            (
                "named function redeclaration",
                r#"
                    local get_schemes = wezterm.color.get_builtin_schemes
                    function get_schemes()
                      return {}
                    end
                    config.colors = get_schemes()['Gruvbox Light']
                "#,
            ),
        ] {
            let marker = "get_schemes()['Gruvbox Light']";
            let query_start = source
                .rfind(marker)
                .expect("expected rebound function alias lookup marker");
            assert_eq!(
                super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                    source,
                    &source[query_start..],
                ),
                None,
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn builtin_scheme_lookup_resolver_invalidates_rebound_module_aliases() {
        let cases = [
            (
                "local declaration shadow",
                r#"
                    local wt = require 'wezterm'
                    local wt
                    config.colors = wt.color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "wt.color.get_builtin_schemes()['Gruvbox Light']",
            ),
            (
                "named function shadow",
                r#"
                    local wt = require 'wezterm'
                    function wt()
                      return {}
                    end
                    config.colors = wt.color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "wt.color.get_builtin_schemes()['Gruvbox Light']",
            ),
            (
                "multi-target dynamic shadow",
                r#"
                    local wt = require 'wezterm'
                    wt, other = choose_module(), nil
                    config.colors = wt.color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "wt.color.get_builtin_schemes()['Gruvbox Light']",
            ),
            (
                "stale module used by function alias",
                r#"
                    local wt = require 'wezterm'
                    local wt
                    local get_schemes = wt.color.get_builtin_schemes
                    config.colors = get_schemes()['Gruvbox Light']
                "#,
                "get_schemes()['Gruvbox Light']",
            ),
        ];

        for (label, source, marker) in cases {
            let query_start = source
                .rfind(marker)
                .expect("expected rebound module alias lookup marker");
            assert_eq!(
                super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                    source,
                    &source[query_start..],
                ),
                None,
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn builtin_scheme_lookup_resolver_rejects_shadowed_direct_wezterm_receiver() {
        for (label, source) in [
            (
                "dynamic assignment",
                r#"
                    local wezterm = choose_module()
                    config.colors = wezterm.color.get_builtin_schemes()['Gruvbox Light']
                "#,
            ),
            (
                "declaration-only shadow",
                r#"
                    local wezterm
                    config.colors = wezterm.color.get_builtin_schemes()['Gruvbox Light']
                "#,
            ),
            (
                "named function shadow",
                r#"
                    function wezterm()
                      return {}
                    end
                    config.colors = wezterm.color.get_builtin_schemes()['Gruvbox Light']
                "#,
            ),
            (
                "multi-target shadow",
                r#"
                    wezterm, other = choose_module(), nil
                    config.colors = wezterm.color.get_builtin_schemes()['Gruvbox Light']
                "#,
            ),
        ] {
            let marker = "wezterm.color.get_builtin_schemes()['Gruvbox Light']";
            let query_start = source
                .rfind(marker)
                .expect("expected shadowed direct wezterm lookup marker");
            assert_eq!(
                super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                    source,
                    &source[query_start..],
                ),
                None,
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn builtin_scheme_lookup_resolver_rejects_shadowed_require_receiver() {
        let cases = [
            (
                "direct require dynamic shadow",
                r#"
                    local require = choose_loader()
                    config.colors = require('wezterm').color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "require('wezterm').color.get_builtin_schemes()['Gruvbox Light']",
            ),
            (
                "direct require declaration-only shadow",
                r#"
                    local require
                    config.colors = require('wezterm').color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "require('wezterm').color.get_builtin_schemes()['Gruvbox Light']",
            ),
            (
                "direct require named function shadow",
                r#"
                    function require()
                      return {}
                    end
                    config.colors = require('wezterm').color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "require('wezterm').color.get_builtin_schemes()['Gruvbox Light']",
            ),
            (
                "direct require multi-target shadow",
                r#"
                    require, other = choose_loader(), nil
                    config.colors = require('wezterm').color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "require('wezterm').color.get_builtin_schemes()['Gruvbox Light']",
            ),
            (
                "module alias derived from shadowed require",
                r#"
                    local require <const> = choose_loader()
                    local wt = require 'wezterm'
                    config.colors = wt.color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "wt.color.get_builtin_schemes()['Gruvbox Light']",
            ),
            (
                "function alias derived from shadowed require",
                r#"
                    local require = choose_loader()
                    local get_schemes = require('wezterm').color.get_builtin_schemes
                    config.colors = get_schemes()['Gruvbox Light']
                "#,
                "get_schemes()['Gruvbox Light']",
            ),
        ];

        for (label, source, marker) in cases {
            let query_start = source
                .rfind(marker)
                .expect("expected shadowed require lookup marker");
            assert_eq!(
                super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                    source,
                    &source[query_start..],
                ),
                None,
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn builtin_scheme_lookup_resolver_honors_lua_local_attributes() {
        let resolved = r#"
            local scheme_name = 'Gruvbox Light'
            local scheme_name <const> = 'Tokyo Night'
            config.colors = wezterm.color.get_builtin_schemes()[scheme_name]
        "#;
        let resolved_marker = "wezterm.color.get_builtin_schemes()[scheme_name]";
        let resolved_start = resolved
            .rfind(resolved_marker)
            .expect("expected attributed scheme-name lookup marker");
        assert_eq!(
            super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                resolved,
                &resolved[resolved_start..],
            )
            .as_deref(),
            Some("Tokyo Night")
        );

        for (label, source, marker) in [
            (
                "declaration-only key shadow",
                r#"
                    local scheme_name = 'Gruvbox Light'
                    local scheme_name <const>
                    config.colors = wezterm.color.get_builtin_schemes()[scheme_name]
                "#,
                "wezterm.color.get_builtin_schemes()[scheme_name]",
            ),
            (
                "dynamic module alias shadow",
                r#"
                    local wt = require 'wezterm'
                    local wt <const> = choose_module()
                    config.colors = wt.color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "wt.color.get_builtin_schemes()['Gruvbox Light']",
            ),
            (
                "dynamic function alias shadow",
                r#"
                    local get_schemes = wezterm.color.get_builtin_schemes
                    local get_schemes <close> = choose_getter()
                    config.colors = get_schemes()['Gruvbox Light']
                "#,
                "get_schemes()['Gruvbox Light']",
            ),
        ] {
            let query_start = source
                .rfind(marker)
                .expect("expected attributed shadow lookup marker");
            assert_eq!(
                super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                    source,
                    &source[query_start..],
                ),
                None,
                "case was {label:?}: {source:?}"
            );
        }

        for (label, source, marker) in [
            (
                "const module alias",
                r#"
                    local wt <const> = require 'wezterm'
                    config.colors = wt.color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "wt.color.get_builtin_schemes()['Gruvbox Light']",
            ),
            (
                "const function alias",
                r#"
                    local get_schemes <const> = wezterm.color.get_builtin_schemes
                    config.colors = get_schemes()['Gruvbox Light']
                "#,
                "get_schemes()['Gruvbox Light']",
            ),
        ] {
            let query_start = source
                .rfind(marker)
                .expect("expected valid attributed alias lookup marker");
            assert_eq!(
                super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                    source,
                    &source[query_start..],
                )
                .as_deref(),
                Some("Gruvbox Light"),
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn builtin_scheme_lookup_resolver_honors_bindings_after_same_line_labels() {
        let cases = [
            (
                "direct wezterm shadow",
                r#"
                    ::reload:: local wezterm = choose_module()
                    config.colors = wezterm.color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "wezterm.color.get_builtin_schemes()['Gruvbox Light']",
                None,
            ),
            (
                "require shadow after commented label",
                r#"
                    ::reload:: --[[gap]] local require = choose_loader()
                    config.colors = require('wezterm').color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "require('wezterm').color.get_builtin_schemes()['Gruvbox Light']",
                None,
            ),
            (
                "named function alias shadow",
                r#"
                    local get_schemes = wezterm.color.get_builtin_schemes
                    ::reload:: function get_schemes() return {} end
                    config.colors = get_schemes()['Gruvbox Light']
                "#,
                "get_schemes()['Gruvbox Light']",
                None,
            ),
            (
                "attributed key replacement",
                r#"
                    local scheme_name = 'Gruvbox Light'
                    ::reload:: local scheme_name <const> = 'Tokyo Night'
                    config.colors = wezterm.color.get_builtin_schemes()[scheme_name]
                "#,
                "wezterm.color.get_builtin_schemes()[scheme_name]",
                Some("Tokyo Night"),
            ),
            (
                "attributed declaration-only shadow",
                r#"
                    local scheme_name = 'Gruvbox Light'
                    ::reload:: local scheme_name <const>
                    config.colors = wezterm.color.get_builtin_schemes()[scheme_name]
                "#,
                "wezterm.color.get_builtin_schemes()[scheme_name]",
                None,
            ),
            (
                "multiple labels before module shadow",
                r#"
                    local wt = require 'wezterm'
                    ::first:: ::second:: local wt = choose_module()
                    config.colors = wt.color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "wt.color.get_builtin_schemes()['Gruvbox Light']",
                None,
            ),
            (
                "mid-slice direct wezterm shadow",
                r#"
                    local marker = true ::reload:: local wezterm = choose_module()
                    config.colors = wezterm.color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "wezterm.color.get_builtin_schemes()['Gruvbox Light']",
                None,
            ),
            (
                "mid-slice require shadow",
                r#"
                    local marker = true ::reload:: local require = choose_loader()
                    config.colors = require('wezterm').color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "require('wezterm').color.get_builtin_schemes()['Gruvbox Light']",
                None,
            ),
            (
                "mid-slice attributed key replacement",
                r#"
                    local scheme_name = 'Gruvbox Light'
                    local marker = true ::reload:: local scheme_name <const> = 'Tokyo Night'
                    config.colors = wezterm.color.get_builtin_schemes()[scheme_name]
                "#,
                "wezterm.color.get_builtin_schemes()[scheme_name]",
                Some("Tokyo Night"),
            ),
            (
                "mid-slice named function alias shadow",
                r#"
                    local get_schemes = wezterm.color.get_builtin_schemes
                    local marker = true ::reload:: function get_schemes() return {} end
                    config.colors = get_schemes()['Gruvbox Light']
                "#,
                "get_schemes()['Gruvbox Light']",
                None,
            ),
            (
                "mid-slice consecutive labels before module shadow",
                r#"
                    local wt = require 'wezterm'
                    local marker = true ::first:: ::second:: local wt = choose_module()
                    config.colors = wt.color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "wt.color.get_builtin_schemes()['Gruvbox Light']",
                None,
            ),
            (
                "spaced commented multiline label require shadow",
                r#"
                    local marker = true :: --[[label gap]]
                      reload --[[closing gap]]
                    :: local require = choose_loader()
                    config.colors = require('wezterm').color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "require('wezterm').color.get_builtin_schemes()['Gruvbox Light']",
                None,
            ),
        ];

        for (label, source, marker, expected) in cases {
            let query_start = source
                .rfind(marker)
                .expect("expected same-line label lookup marker");
            assert_eq!(
                super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                    source,
                    &source[query_start..],
                )
                .as_deref(),
                expected,
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn builtin_scheme_lookup_resolver_rejects_non_function_block_rebindings() {
        let cases = [
            (
                "conditional scheme key rebind",
                r#"
                    local scheme_name = 'Gruvbox Light'
                    if choose_dynamically then
                      scheme_name = choose_scheme()
                    end
                    config.colors = wezterm.color.get_builtin_schemes()[scheme_name]
                "#,
                "wezterm.color.get_builtin_schemes()[scheme_name]",
            ),
            (
                "do-block static getter key rebind",
                r#"
                    local getter_key = 'get_builtin_schemes'
                    do
                      getter_key = choose_getter_key()
                    end
                    config.colors = wezterm.color[getter_key]()['Gruvbox Light']
                "#,
                "wezterm.color[getter_key]()['Gruvbox Light']",
            ),
            (
                "conditional function alias rebind",
                r#"
                    local get_schemes = wezterm.color.get_builtin_schemes
                    if choose_dynamically then
                      get_schemes = choose_getter()
                    end
                    config.colors = get_schemes()['Gruvbox Light']
                "#,
                "get_schemes()['Gruvbox Light']",
            ),
            (
                "do-block module alias rebind",
                r#"
                    local wt = require 'wezterm'
                    do
                      wt = choose_module()
                    end
                    config.colors = wt.color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "wt.color.get_builtin_schemes()['Gruvbox Light']",
            ),
            (
                "conditional named function write",
                r#"
                    local scheme_name = 'Gruvbox Light'
                    if choose_dynamically then
                      function scheme_name()
                        return 'Tokyo Night'
                      end
                    end
                    config.colors = wezterm.color.get_builtin_schemes()[scheme_name]
                "#,
                "wezterm.color.get_builtin_schemes()[scheme_name]",
            ),
        ];

        for (label, source, marker) in cases {
            let query_start = source
                .rfind(marker)
                .expect("expected non-function block rebind lookup marker");
            assert_eq!(
                super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                    source,
                    &source[query_start..],
                ),
                None,
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn builtin_scheme_lookup_resolver_characterizes_non_function_block_boundary() {
        for (label, source, expected) in [
            (
                "same-name function body write",
                r#"
                    local scheme_name = 'Gruvbox Light'
                    function mutate_later()
                      scheme_name = choose_scheme()
                    end
                    config.colors = wezterm.color.get_builtin_schemes()[scheme_name]
                "#,
                Some("Gruvbox Light"),
            ),
            (
                "unrelated conditional block",
                r#"
                    local scheme_name = 'Gruvbox Light'
                    if inspect_only then
                      unrelated = choose_unrelated()
                    end
                    config.colors = wezterm.color.get_builtin_schemes()[scheme_name]
                "#,
                None,
            ),
            (
                "same-name conditional read",
                r#"
                    local scheme_name = 'Gruvbox Light'
                    if inspect_only then
                      print(scheme_name)
                    end
                    config.colors = wezterm.color.get_builtin_schemes()[scheme_name]
                "#,
                None,
            ),
        ] {
            let marker = "wezterm.color.get_builtin_schemes()[scheme_name]";
            let query_start = source
                .rfind(marker)
                .expect("expected function-body guard lookup marker");
            assert_eq!(
                super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                    source,
                    &source[query_start..],
                )
                .as_deref(),
                expected,
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn builtin_scheme_lookup_resolver_rejects_continued_module_alias_bindings() {
        let cases = [
            (
                "direct module alias block-comment continuation",
                r#"
                    local wt = require 'wezterm' --[[gap]] .other
                    config.colors = wt.color.get_builtin_schemes()['Gruvbox Light']
                "#,
                "wt.color.get_builtin_schemes()['Gruvbox Light']",
            ),
            (
                "continued module used by function alias",
                r#"
                    local wt = require 'wezterm' -- gap
                      .other
                    local get_schemes = wt.color.get_builtin_schemes
                    config.colors = get_schemes()['Gruvbox Light']
                "#,
                "get_schemes()['Gruvbox Light']",
            ),
        ];

        for (label, source, marker) in cases {
            let query_start = source
                .rfind(marker)
                .expect("expected continued module alias lookup marker");
            assert_eq!(
                super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                    source,
                    &source[query_start..],
                ),
                None,
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn builtin_scheme_lookup_resolver_rejects_continued_function_alias_bindings() {
        for (label, source) in [
            (
                "block-comment accessor continuation",
                r#"
                    local get_schemes = wezterm.color.get_builtin_schemes --[[gap]] .other
                    config.colors = get_schemes()['Gruvbox Light']
                "#,
            ),
            (
                "line-comment accessor continuation",
                r#"
                    local get_schemes = wezterm.color.get_builtin_schemes -- gap
                      .other
                    config.colors = get_schemes()['Gruvbox Light']
                "#,
            ),
        ] {
            let marker = "get_schemes()['Gruvbox Light']";
            let query_start = source
                .rfind(marker)
                .expect("expected continued function alias lookup marker");
            assert_eq!(
                super::lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
                    source,
                    &source[query_start..],
                ),
                None,
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn builtin_scheme_lookup_resolver_preserves_load_scheme_tail_regressions() {
        for (source, expected) in [
            (
                "wezterm.color.load_scheme('/label.toml') ::next:: next_call()",
                Some("/label.toml"),
            ),
            (
                "wezterm.color.load_scheme('/next.toml')\nnext_call()",
                Some("/next.toml"),
            ),
            (
                "wezterm.color.load_scheme('/continued.toml') -- gap\n .colors",
                None,
            ),
        ] {
            assert_eq!(
                super::lua_wezterm_color_load_scheme_path_from_query_with_static_source(
                    source, source,
                )
                .as_deref(),
                expected,
                "source was {source:?}"
            );
        }
    }

    #[test]
    fn load_scheme_call_resolver_honors_lua_local_attributes() {
        for (label, source, expected) in [
            (
                "const path replacement",
                r#"
                    local path = '/old.toml'
                    local path <const> = '/const.toml'
                    config.colors = wezterm.color.load_scheme(path)
                "#,
                Some("/const.toml"),
            ),
            (
                "labelled const path replacement",
                r#"
                    local path = '/old.toml'
                    ::reload:: local path <const> = '/labelled.toml'
                    config.colors = wezterm.color.load_scheme(path)
                "#,
                Some("/labelled.toml"),
            ),
            (
                "mid-slice labelled const path replacement",
                r#"
                    local path = '/old.toml'
                    local marker = true ::reload:: local path <const> = '/mid-label.toml'
                    config.colors = wezterm.color.load_scheme(path)
                "#,
                Some("/mid-label.toml"),
            ),
            (
                "close path dynamic shadow",
                r#"
                    local path = '/old.toml'
                    local path <close> = choose_closeable()
                    config.colors = wezterm.color.load_scheme(path)
                "#,
                None,
            ),
            (
                "declaration-only attributed shadow",
                r#"
                    local path = '/old.toml'
                    local path <const>
                    config.colors = wezterm.color.load_scheme(path)
                "#,
                None,
            ),
        ] {
            let marker = "wezterm.color.load_scheme(path)";
            let query_start = source
                .rfind(marker)
                .expect("expected attributed load_scheme marker");
            assert_eq!(
                super::lua_wezterm_color_load_scheme_path_from_query_with_static_source(
                    source,
                    &source[query_start..],
                )
                .as_deref(),
                expected,
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn load_scheme_call_resolver_accepts_static_path_expressions_at_original_call_offset() {
        let cases = [
            (
                "canonical direct call",
                r#"
                    local dir = '/canonical'
                    local file = 'direct.toml'
                    local path = dir .. '/' .. file
                    config.colors = wezterm.color.load_scheme(path)
                    path = '/after/canonical.toml'
                "#,
                "wezterm.color.load_scheme(path)",
                "/canonical/direct.toml",
            ),
            (
                "module alias call",
                r#"
                    local wt = wezterm
                    local dir = '/module'
                    local file = 'alias.toml'
                    local path = dir .. '/' .. file
                    config.colors = wt.color.load_scheme(path)
                    path = '/after/module.toml'
                "#,
                "wt.color.load_scheme(path)",
                "/module/alias.toml",
            ),
            (
                "direct require call",
                r#"
                    local dir = '/require'
                    local file = 'direct.toml'
                    local path = dir .. '/' .. file
                    config.colors = require('wezterm').color.load_scheme(path)
                    path = '/after/require.toml'
                "#,
                "require('wezterm').color.load_scheme(path)",
                "/require/direct.toml",
            ),
            (
                "static key call",
                r#"
                    local wt = require 'wezterm'
                    local color_key = 'color'
                    local loader_key = 'load_scheme'
                    local dir = '/static-key'
                    local file = 'call.toml'
                    local path = dir .. '/' .. file
                    config.colors = wt[color_key][loader_key](path)
                    path = '/after/static-key.toml'
                "#,
                "wt[color_key][loader_key](path)",
                "/static-key/call.toml",
            ),
            (
                "function alias call",
                r#"
                    local wt = require 'wezterm'
                    local load_scheme = wt.color.load_scheme
                    local dir = '/function'
                    local file = 'alias.toml'
                    local path = dir .. '/' .. file
                    config.colors = load_scheme(path)
                    path = '/after/function.toml'
                "#,
                "load_scheme(path)",
                "/function/alias.toml",
            ),
        ];

        for (label, source, marker, expected) in cases {
            let query_start = source
                .find(marker)
                .expect("expected load_scheme call marker");
            assert_eq!(
                super::lua_wezterm_color_load_scheme_path_from_query_with_static_source(
                    source,
                    &source[query_start..],
                )
                .as_deref(),
                Some(expected),
                "case was {label:?}"
            );
        }
    }

    #[test]
    fn load_scheme_call_resolver_rejects_invalid_argument_shapes() {
        for (label, call) in [
            ("zero arguments", "wezterm.color.load_scheme()"),
            (
                "multiple arguments",
                "wezterm.color.load_scheme(path, '/two.toml')",
            ),
            (
                "missing close parenthesis",
                "wezterm.color.load_scheme('/one.toml'",
            ),
            (
                "no-parentheses identifier",
                "wezterm.color.load_scheme path",
            ),
        ] {
            let source = format!("local path = '/one.toml'; config.colors = {call}");
            let query_start = source
                .find("wezterm.color.load_scheme")
                .expect("expected load_scheme call marker");
            assert_eq!(
                super::lua_wezterm_color_load_scheme_path_from_query_with_static_source(
                    &source,
                    &source[query_start..],
                ),
                None,
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn load_scheme_call_resolver_rejects_expression_continuation_tails() {
        for (label, tail) in [
            ("field access", ").colors"),
            ("index access", ")[1]"),
            ("parenthesized call", ")('/next.toml')"),
            ("method call", "):next()"),
            ("quoted call sugar", ") '/next.toml'"),
            ("long-bracket call sugar", ") [=[/next.toml]=]"),
            ("table call sugar", ") {}"),
            ("concat", ") .. suffix"),
            ("addition", ") + suffix"),
            ("subtraction", ") - suffix"),
            ("multiplication", ") * suffix"),
            ("division", ") / suffix"),
            ("floor division", ") // suffix"),
            ("modulo", ") % suffix"),
            ("power", ") ^ suffix"),
            ("bit and", ") & suffix"),
            ("bit or", ") | suffix"),
            ("bit xor", ") ~ suffix"),
            ("shift left", ") << suffix"),
            ("shift right", ") >> suffix"),
            ("equality", ") == suffix"),
            ("inequality", ") ~= suffix"),
            ("less than", ") < suffix"),
            ("greater than", ") > suffix"),
            ("less than or equal", ") <= suffix"),
            ("greater than or equal", ") >= suffix"),
            ("logical and", ") and suffix"),
            ("logical or", ") or suffix"),
            ("incomplete label", ") ::next"),
            ("block-comment field access", ") --[[gap]] .colors"),
            ("line-comment index access", ") -- gap\n [1]"),
            (
                "block-comment parenthesized call",
                ") --[[gap]]\n ('/next.toml')",
            ),
            ("line-comment concat", ") -- gap\n .. suffix"),
        ] {
            let source = format!("wezterm.color.load_scheme('/one.toml'{tail}");
            assert_eq!(
                super::lua_wezterm_color_load_scheme_path_from_query_with_static_source(
                    &source, &source,
                ),
                None,
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn load_scheme_call_resolver_accepts_literal_sugar_and_value_end_tails() {
        for (label, source, expected) in [
            (
                "quoted no-parentheses sugar",
                "wezterm.color.load_scheme '/quoted.toml'",
                "/quoted.toml",
            ),
            (
                "long-bracket no-parentheses sugar",
                "wezterm.color.load_scheme [=[/long-bracket.toml]=]",
                "/long-bracket.toml",
            ),
            (
                "end of input",
                "wezterm.color.load_scheme('/eof.toml')",
                "/eof.toml",
            ),
            (
                "semicolon terminator",
                "wezterm.color.load_scheme('/semicolon.toml'); next_call()",
                "/semicolon.toml",
            ),
            (
                "table comma terminator",
                "wezterm.color.load_scheme('/comma.toml'), next_field = true",
                "/comma.toml",
            ),
            (
                "table close terminator",
                "wezterm.color.load_scheme('/close.toml') }",
                "/close.toml",
            ),
            (
                "newline statement boundary",
                "wezterm.color.load_scheme('/newline.toml')\nnext_call()",
                "/newline.toml",
            ),
            (
                "line-comment end of input",
                "wezterm.color.load_scheme('/line-comment.toml') -- trailing",
                "/line-comment.toml",
            ),
            (
                "line-comment statement boundary",
                "wezterm.color.load_scheme('/line-comment-next.toml') -- trailing\nnext_call()",
                "/line-comment-next.toml",
            ),
            (
                "block-comment statement boundary",
                "wezterm.color.load_scheme('/block-comment.toml') --[[trailing]]\nnext_call()",
                "/block-comment.toml",
            ),
            (
                "block-comment end of input",
                "wezterm.color.load_scheme('/block-comment-eof.toml') --[[trailing]]",
                "/block-comment-eof.toml",
            ),
            (
                "label statement boundary",
                "wezterm.color.load_scheme('/label.toml') ::next:: next_call()",
                "/label.toml",
            ),
        ] {
            assert_eq!(
                super::lua_wezterm_color_load_scheme_path_from_query_with_static_source(
                    source, source,
                )
                .as_deref(),
                Some(expected),
                "case was {label:?}: {source:?}"
            );
        }
    }

    #[test]
    fn lua_config_load_scheme_colors_assignment_from_query_resolves_variable_path() {
        let source = r#"
            local dir = '/legacy'
            local file = 'config-colors.toml'
            local path = dir .. '/' .. file
            config.colors = wezterm.color.load_scheme(path)
            path = '/after/legacy.toml'
        "#;

        let assignment = super::lua_config_load_scheme_colors_assignment_from_query(source)
            .expect("expected config.colors load_scheme assignment");
        assert_eq!(assignment.path, "/legacy/config-colors.toml");
        assert!(assignment.variable.is_none());
    }

    #[test]
    fn window_app_applies_wezterm_lua_builtin_scheme_lookup_to_config_colors() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local config = {}

            config.colors = wezterm.color.get_builtin_schemes()['Gruvbox Light']

            return config
            "##,
        )
        .expect("expected WezTerm built-in scheme assignment config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(40, 40, 40));
        assert_eq!(effective.background_color, Color::Rgb(251, 241, 199));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(40, 40, 40));
        let ansi = effective.ansi_palette.expect("expected ANSI palette");
        assert_eq!(ansi[0], Color::Rgb(251, 241, 199));
        assert_eq!(ansi[1], Color::Rgb(157, 0, 6));
        assert_eq!(ansi[8], Color::Rgb(157, 131, 116));
        assert_eq!(ansi[15], Color::Rgb(124, 111, 100));
    }

    #[test]
    fn lua_builtin_color_scheme_assignment_resolves_whole_map_palette_binding() {
        let source = r#"
            local schemes = wezterm.color.get_builtin_schemes()
            local scheme = schemes['Gruvbox Light']
            scheme.background = '#010203'
            config.colors = scheme
        "#;
        let statement = source
            .get(
                source
                    .find("local scheme =")
                    .expect("expected palette binding")..,
            )
            .expect("expected palette binding query");

        assert_eq!(
            super::lua_builtin_color_scheme_assignment_from_query(source, statement, "scheme")
                .as_deref(),
            Some("Gruvbox Light")
        );
    }

    #[test]
    fn window_app_applies_wezterm_lua_builtin_scheme_variable_mutations_to_config_colors() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local config = {}
            local scheme_name = 'Gruvbox Light'

            local scheme = wezterm.color.get_builtin_schemes()[scheme_name]
            scheme_name = 'Builtin Dark'
            scheme = wezterm.color.get_builtin_schemes()[scheme_name]
            scheme.background = '#010203'

            config.colors = scheme
            scheme = wezterm.color.get_builtin_schemes()['Gruvbox Light']
            "##,
        )
        .expect("expected WezTerm built-in scheme variable mutation config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(187, 187, 187));
        assert_eq!(effective.background_color, Color::Rgb(1, 2, 3));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(187, 187, 187));
        assert_eq!(effective.cursor_fg_color, Some(Color::Rgb(255, 255, 255)));
    }

    #[test]
    fn window_app_applies_wezterm_lua_builtin_scheme_lookup_through_aliases_to_config_colors() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local config = {}
            local wt <const> = require 'wezterm'
            local get_schemes = wt.color.get_builtin_schemes
            local scheme = get_schemes()['Gruvbox Light']

            config.colors = scheme
            "##,
        )
        .expect("expected WezTerm built-in scheme alias config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(40, 40, 40));
        assert_eq!(effective.background_color, Color::Rgb(251, 241, 199));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(40, 40, 40));
    }

    #[test]
    fn window_app_loads_wezterm_lua_colors_from_load_scheme_toml_file() {
        static NEXT_LOAD_SCHEME_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Loaded Scheme"

            [colors]
            foreground = "#212223"
            background = "#242526"
            ansi = [
              "#000001",
              "#000002",
              "#000003",
              "#000004",
              "#000005",
              "#000006",
              "#000007",
              "#000008",
            ]
            brights = [
              "#000009",
              "#00000a",
              "#00000b",
              "#00000c",
              "#00000d",
              "#00000e",
              "#00000f",
              "#000010",
            ]
            "##,
        )
        .expect("expected temp load_scheme TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}
            local colors, metadata = wezterm.color.load_scheme('{scheme_file_query}')

            config.colors = colors

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme colors config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(33, 34, 35));
        assert_eq!(effective.background_color, Color::Rgb(36, 37, 38));
        assert_eq!(
            effective.ansi_palette.expect("expected ANSI palette")[1],
            Color::Rgb(0, 0, 2)
        );
        assert_eq!(
            effective.ansi_palette.expect("expected ANSI palette")[9],
            Color::Rgb(0, 0, 10)
        );
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_loads_wezterm_lua_colors_from_load_scheme_dotted_comment_call() {
        static NEXT_LOAD_SCHEME_DOTTED_COMMENT_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-dotted-comment-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_DOTTED_COMMENT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Dotted Comment Loaded Scheme"

            [colors]
            foreground = "#414243"
            background = "#444546"
            cursor_bg = "#474849"
            "##,
        )
        .expect("expected temp load_scheme dotted comment TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}
            local colors, metadata = wezterm.color -- loader namespace
              .load_scheme('{scheme_file_query}')

            config.colors = colors

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme dotted comment colors config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(65, 66, 67));
        assert_eq!(effective.background_color, Color::Rgb(68, 69, 70));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(71, 72, 73));
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_loads_wezterm_lua_colors_from_load_scheme_direct_require_call() {
        static NEXT_LOAD_SCHEME_DIRECT_REQUIRE_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-direct-require-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_DIRECT_REQUIRE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Direct Require Loaded Scheme"

            [colors]
            foreground = "#a1a2a3"
            background = "#a4a5a6"
            cursor_bg = "#a7a8a9"
            "##,
        )
        .expect("expected temp direct require load_scheme TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local config = {{}}
            local colors, metadata = require('wezterm').color.load_scheme('{scheme_file_query}')

            config.colors = colors

            return config
            "##
        ))
        .expect("expected WezTerm direct require load_scheme colors config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(161, 162, 163));
        assert_eq!(effective.background_color, Color::Rgb(164, 165, 166));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(167, 168, 169));
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_loads_wezterm_lua_colors_from_load_scheme_parenthesized_require_call() {
        static NEXT_LOAD_SCHEME_PAREN_REQUIRE_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-paren-require-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_PAREN_REQUIRE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Parenthesized Require Loaded Scheme"

            [colors]
            foreground = "#b1b2b3"
            background = "#b4b5b6"
            cursor_bg = "#b7b8b9"
            "##,
        )
        .expect("expected temp parenthesized require load_scheme TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local config = {{}}
            local colors, metadata = (require('wezterm')).color.load_scheme('{scheme_file_query}')

            config.colors = colors

            return config
            "##
        ))
        .expect("expected WezTerm parenthesized require load_scheme colors config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(177, 178, 179));
        assert_eq!(effective.background_color, Color::Rgb(180, 181, 182));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(183, 184, 185));
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_loads_wezterm_lua_colors_from_load_scheme_direct_static_key_module() {
        static NEXT_LOAD_SCHEME_DIRECT_STATIC_KEY_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-direct-static-key-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_DIRECT_STATIC_KEY_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Direct Static Key Loaded Scheme"

            [colors]
            foreground = "#919293"
            background = "#949596"
            cursor_bg = "#979899"
            "##,
        )
        .expect("expected temp direct static-key load_scheme TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wt = require 'wezterm'
            local config = {{}}
            local color_key = 'color'
            local load_key = 'load_scheme'
            local colors, metadata = wt[color_key][load_key]('{scheme_file_query}')

            config.colors = colors

            return config
            "##
        ))
        .expect("expected WezTerm direct static-key load_scheme colors config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(145, 146, 147));
        assert_eq!(effective.background_color, Color::Rgb(148, 149, 150));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(151, 152, 153));
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_loads_wezterm_lua_colors_from_load_scheme_alias() {
        static NEXT_LOAD_SCHEME_ALIAS_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-alias-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_ALIAS_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Alias Loaded Scheme"

            [colors]
            foreground = "#313233"
            background = "#343536"
            cursor_bg = "#373839"
            "##,
        )
        .expect("expected temp load_scheme alias TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}
            local load_scheme = wezterm.color.load_scheme

            config.colors = load_scheme('{scheme_file_query}')

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme alias colors config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(49, 50, 51));
        assert_eq!(effective.background_color, Color::Rgb(52, 53, 54));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(55, 56, 57));
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_loads_wezterm_lua_colors_from_load_scheme_alias_static_key_module() {
        static NEXT_LOAD_SCHEME_ALIAS_STATIC_KEY_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-alias-static-key-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_ALIAS_STATIC_KEY_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Alias Static Key Loaded Scheme"

            [colors]
            foreground = "#818283"
            background = "#848586"
            cursor_bg = "#878889"
            "##,
        )
        .expect("expected temp load_scheme alias static-key TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wt = require 'wezterm'
            local config = {{}}
            local color_key = 'color'
            local load_key = 'load_scheme'
            local load_scheme = wt[color_key][load_key]

            config.colors = load_scheme('{scheme_file_query}')

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme static-key alias colors config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(129, 130, 131));
        assert_eq!(effective.background_color, Color::Rgb(132, 133, 134));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(135, 136, 137));
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_loads_wezterm_lua_colors_from_load_scheme_alias_comment_call() {
        static NEXT_LOAD_SCHEME_ALIAS_COMMENT_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-alias-comment-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_ALIAS_COMMENT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Alias Comment Loaded Scheme"

            [colors]
            foreground = "#616263"
            background = "#646566"
            cursor_bg = "#676869"
            "##,
        )
        .expect("expected temp load_scheme alias comment TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}
            local load_scheme = wezterm.color.load_scheme

            config.colors = load_scheme -- palette
              ('{scheme_file_query}')

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme alias comment colors config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(97, 98, 99));
        assert_eq!(effective.background_color, Color::Rgb(100, 101, 102));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(103, 104, 105));
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_loads_wezterm_lua_colors_from_load_scheme_alias_dotted_comment() {
        static NEXT_LOAD_SCHEME_ALIAS_DOTTED_COMMENT_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-alias-dotted-comment-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_ALIAS_DOTTED_COMMENT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Alias Dotted Comment Loaded Scheme"

            [colors]
            foreground = "#717273"
            background = "#747576"
            cursor_bg = "#777879"
            "##,
        )
        .expect("expected temp load_scheme alias dotted comment TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}
            local load_scheme = wezterm.color -- loader namespace
              .load_scheme

            config.colors = load_scheme('{scheme_file_query}')

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme alias dotted-comment colors config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(113, 114, 115));
        assert_eq!(effective.background_color, Color::Rgb(116, 117, 118));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(119, 120, 121));
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_loads_wezterm_lua_colors_from_load_scheme_alias_variable() {
        static NEXT_LOAD_SCHEME_ALIAS_VARIABLE_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-alias-variable-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_ALIAS_VARIABLE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Alias Variable Loaded Scheme"

            [colors]
            foreground = "#515253"
            background = "#545556"
            cursor_bg = "#575859"
            "##,
        )
        .expect("expected temp load_scheme alias variable TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}
            local load_scheme = wezterm.color.load_scheme
            local colors, metadata = load_scheme('{scheme_file_query}')

            config.colors = colors

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme alias variable colors config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(81, 82, 83));
        assert_eq!(effective.background_color, Color::Rgb(84, 85, 86));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(87, 88, 89));
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_loads_wezterm_lua_colors_from_nonlocal_load_scheme_assignment() {
        static NEXT_NONLOCAL_LOAD_SCHEME_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-nonlocal-load-scheme-{}-{}.toml",
            std::process::id(),
            NEXT_NONLOCAL_LOAD_SCHEME_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Nonlocal Loaded Scheme"

            [colors]
            foreground = "#414243"
            background = "#444546"
            cursor_bg = "#474849"
            "##,
        )
        .expect("expected temp nonlocal load_scheme TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}
            colors, metadata = wezterm.color.load_scheme('{scheme_file_query}')

            config.colors = colors

            return config
            "##
        ))
        .expect("expected WezTerm nonlocal load_scheme colors config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(65, 66, 67));
        assert_eq!(effective.background_color, Color::Rgb(68, 69, 70));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(71, 72, 73));
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_uses_latest_wezterm_lua_load_scheme_variable_assignment_before_config_colors() {
        static NEXT_LOAD_SCHEME_REASSIGN_ID: AtomicUsize = AtomicUsize::new(0);

        let scheme_id = NEXT_LOAD_SCHEME_REASSIGN_ID.fetch_add(1, Ordering::Relaxed);
        let mut first_scheme_file = std::env::temp_dir();
        first_scheme_file.push(format!(
            "rterm-load-scheme-reassign-first-{}-{scheme_id}.toml",
            std::process::id()
        ));
        let mut second_scheme_file = std::env::temp_dir();
        second_scheme_file.push(format!(
            "rterm-load-scheme-reassign-second-{}-{scheme_id}.toml",
            std::process::id()
        ));
        let mut third_scheme_file = std::env::temp_dir();
        third_scheme_file.push(format!(
            "rterm-load-scheme-reassign-third-{}-{scheme_id}.toml",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&first_scheme_file);
        let _ = std::fs::remove_file(&second_scheme_file);
        let _ = std::fs::remove_file(&third_scheme_file);
        std::fs::write(
            &first_scheme_file,
            r##"
            [colors]
            foreground = "#010203"
            background = "#040506"
            "##,
        )
        .expect("expected first temp load_scheme reassignment TOML color scheme");
        std::fs::write(
            &second_scheme_file,
            r##"
            [colors]
            foreground = "#111213"
            background = "#141516"
            "##,
        )
        .expect("expected second temp load_scheme reassignment TOML color scheme");
        std::fs::write(
            &third_scheme_file,
            r##"
            [colors]
            foreground = "#212223"
            background = "#242526"
            "##,
        )
        .expect("expected third temp load_scheme reassignment TOML color scheme");
        let first_scheme_file_query = first_scheme_file.to_string_lossy().replace('\\', "/");
        let second_scheme_file_query = second_scheme_file.to_string_lossy().replace('\\', "/");
        let third_scheme_file_query = third_scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}

            local colors = wezterm.color.load_scheme('{first_scheme_file_query}')
            colors = wezterm.color.load_scheme('{second_scheme_file_query}')
            config.colors = colors
            colors = wezterm.color.load_scheme('{third_scheme_file_query}')

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme variable reassignment config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(17, 18, 19));
        assert_eq!(effective.background_color, Color::Rgb(20, 21, 22));
        let _ = std::fs::remove_file(first_scheme_file);
        let _ = std::fs::remove_file(second_scheme_file);
        let _ = std::fs::remove_file(third_scheme_file);
    }

    #[test]
    fn window_app_loads_wezterm_lua_colors_from_load_scheme_path_binding_at_call_time() {
        static NEXT_LOAD_SCHEME_PATH_BINDING_ID: AtomicUsize = AtomicUsize::new(0);

        let scheme_id = NEXT_LOAD_SCHEME_PATH_BINDING_ID.fetch_add(1, Ordering::Relaxed);
        let mut first_scheme_file = std::env::temp_dir();
        first_scheme_file.push(format!(
            "rterm-load-scheme-path-binding-first-{}-{scheme_id}.toml",
            std::process::id()
        ));
        let mut second_scheme_file = std::env::temp_dir();
        second_scheme_file.push(format!(
            "rterm-load-scheme-path-binding-second-{}-{scheme_id}.toml",
            std::process::id()
        ));
        let mut third_scheme_file = std::env::temp_dir();
        third_scheme_file.push(format!(
            "rterm-load-scheme-path-binding-third-{}-{scheme_id}.toml",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&first_scheme_file);
        let _ = std::fs::remove_file(&second_scheme_file);
        let _ = std::fs::remove_file(&third_scheme_file);
        std::fs::write(
            &first_scheme_file,
            r##"
            [colors]
            foreground = "#313233"
            background = "#343536"
            "##,
        )
        .expect("expected first temp load_scheme path-binding TOML color scheme");
        std::fs::write(
            &second_scheme_file,
            r##"
            [colors]
            foreground = "#414243"
            background = "#444546"
            "##,
        )
        .expect("expected second temp load_scheme path-binding TOML color scheme");
        std::fs::write(
            &third_scheme_file,
            r##"
            [colors]
            foreground = "#515253"
            background = "#545556"
            "##,
        )
        .expect("expected third temp load_scheme path-binding TOML color scheme");
        let first_scheme_file_query = first_scheme_file.to_string_lossy().replace('\\', "/");
        let second_scheme_file_query = second_scheme_file.to_string_lossy().replace('\\', "/");
        let third_scheme_file_query = third_scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}
            local first_path = '{first_scheme_file_query}'
            local second_path = '{second_scheme_file_query}'
            local third_path = '{third_scheme_file_query}'
            local scheme_path = first_path
            scheme_path = second_path
            local colors = wezterm.color.load_scheme(scheme_path)
            scheme_path = third_path
            config.colors = colors

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme path-binding config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(65, 66, 67));
        assert_eq!(effective.background_color, Color::Rgb(68, 69, 70));
        let _ = std::fs::remove_file(first_scheme_file);
        let _ = std::fs::remove_file(second_scheme_file);
        let _ = std::fs::remove_file(third_scheme_file);
    }

    #[test]
    fn window_app_ignores_wezterm_lua_config_helper_load_scheme_variable_assignments() {
        static NEXT_LOAD_SCHEME_HELPER_ID: AtomicUsize = AtomicUsize::new(0);

        let scheme_id = NEXT_LOAD_SCHEME_HELPER_ID.fetch_add(1, Ordering::Relaxed);
        let mut top_level_scheme_file = std::env::temp_dir();
        top_level_scheme_file.push(format!(
            "rterm-load-scheme-helper-top-level-{}-{scheme_id}.toml",
            std::process::id()
        ));
        let mut helper_scheme_file = std::env::temp_dir();
        helper_scheme_file.push(format!(
            "rterm-load-scheme-helper-inner-{}-{scheme_id}.toml",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&top_level_scheme_file);
        let _ = std::fs::remove_file(&helper_scheme_file);
        std::fs::write(
            &top_level_scheme_file,
            r##"
            [colors]
            foreground = "#313233"
            background = "#343536"
            "##,
        )
        .expect("expected top-level temp load_scheme TOML color scheme");
        std::fs::write(
            &helper_scheme_file,
            r##"
            [colors]
            foreground = "#010203"
            background = "#040506"
            "##,
        )
        .expect("expected helper temp load_scheme TOML color scheme");
        let top_level_scheme_file_query =
            top_level_scheme_file.to_string_lossy().replace('\\', "/");
        let helper_scheme_file_query = helper_scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}

            local colors = wezterm.color.load_scheme('{top_level_scheme_file_query}')
            local function ignored()
              colors = wezterm.color.load_scheme('{helper_scheme_file_query}')
            end

            config.colors = colors

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme helper assignment config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(49, 50, 51));
        assert_eq!(effective.background_color, Color::Rgb(52, 53, 54));
        let _ = std::fs::remove_file(top_level_scheme_file);
        let _ = std::fs::remove_file(helper_scheme_file);
    }

    #[test]
    fn window_app_ignores_wezterm_lua_config_helper_load_scheme_color_mutations() {
        static NEXT_LOAD_SCHEME_HELPER_MUTATION_ID: AtomicUsize = AtomicUsize::new(0);

        let scheme_id = NEXT_LOAD_SCHEME_HELPER_MUTATION_ID.fetch_add(1, Ordering::Relaxed);
        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-helper-mutation-{}-{scheme_id}.toml",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [colors]
            foreground = "#313233"
            background = "#343536"
            cursor_bg = "#373839"
            "##,
        )
        .expect("expected temp load_scheme helper mutation TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}
            local colors, metadata = wezterm.color.load_scheme('{scheme_file_query}')

            local function ignored()
              colors.background = '#010203'
              colors.cursor_bg = '#040506'
            end

            config.colors = colors

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme helper mutation config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.background_color, Color::Rgb(52, 53, 54));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(55, 56, 57));
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_applies_wezterm_lua_load_scheme_static_color_mutations() {
        static NEXT_LOAD_SCHEME_MUTATION_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-mutation-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_MUTATION_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Loaded Scheme"

            [colors]
            foreground = "#212223"
            background = "#242526"
            cursor_bg = "#272829"
            "##,
        )
        .expect("expected temp load_scheme mutation TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}
            local colors, metadata = wezterm.color.load_scheme('{scheme_file_query}')

            colors.background = '#2a2b2c'
            colors.cursor_bg = '#2d2e2f'
            config.colors = colors

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme mutated colors config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(33, 34, 35));
        assert_eq!(effective.background_color, Color::Rgb(42, 43, 44));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(45, 46, 47));
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_uses_later_wezterm_lua_load_scheme_colors_assignment_after_table() {
        static NEXT_LOAD_SCHEME_ORDER_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-order-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_ORDER_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Loaded Scheme"

            [colors]
            foreground = "#212223"
            background = "#242526"
            cursor_bg = "#272829"
            "##,
        )
        .expect("expected temp load_scheme order TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}
            local colors, metadata = wezterm.color.load_scheme('{scheme_file_query}')

            config.colors = {{
              foreground = '#010203',
              background = '#040506',
              cursor_bg = '#070809',
            }}
            colors.background = '#2a2b2c'
            colors.cursor_bg = '#2d2e2f'
            config.colors = colors

            return config
            "##
        ))
        .expect("expected later WezTerm load_scheme colors assignment config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(33, 34, 35));
        assert_eq!(effective.background_color, Color::Rgb(42, 43, 44));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(45, 46, 47));
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_applies_wezterm_lua_load_scheme_multiline_palette_mutations() {
        static NEXT_LOAD_SCHEME_PALETTE_MUTATION_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-palette-mutation-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_PALETTE_MUTATION_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Loaded Scheme"

            [colors]
            ansi = [
              "#000001",
              "#000002",
              "#000003",
              "#000004",
              "#000005",
              "#000006",
              "#000007",
              "#000008",
            ]
            brights = [
              "#000009",
              "#00000a",
              "#00000b",
              "#00000c",
              "#00000d",
              "#00000e",
              "#00000f",
              "#000010",
            ]
            "##,
        )
        .expect("expected temp load_scheme palette mutation TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}
            local colors, metadata = wezterm.color.load_scheme('{scheme_file_query}')

            colors.ansi = {{
              '#010203',
              '#040506',
              '#070809',
              '#0a0b0c',
              '#0d0e0f',
              '#101112',
              '#131415',
              '#161718',
            }}
            colors.brights = {{
              '#191a1b',
              '#1c1d1e',
              '#1f2021',
              '#222324',
              '#252627',
              '#28292a',
              '#2b2c2d',
              '#2e2f30',
            }}
            config.colors = colors

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme mutated palette config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.ansi_palette.expect("expected ANSI palette")[0],
            Color::Rgb(1, 2, 3)
        );
        assert_eq!(
            effective.ansi_palette.expect("expected ANSI palette")[7],
            Color::Rgb(22, 23, 24)
        );
        assert_eq!(
            effective.ansi_palette.expect("expected ANSI palette")[8],
            Color::Rgb(25, 26, 27)
        );
        assert_eq!(
            effective.ansi_palette.expect("expected ANSI palette")[15],
            Color::Rgb(46, 47, 48)
        );
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_applies_wezterm_lua_load_scheme_bracket_color_mutations() {
        static NEXT_LOAD_SCHEME_BRACKET_MUTATION_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-bracket-mutation-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_BRACKET_MUTATION_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Loaded Scheme"

            [colors]
            foreground = "#313233"
            background = "#343536"
            cursor_bg = "#373839"
            cursor_border = "#3a3b3c"
            "##,
        )
        .expect("expected temp load_scheme bracket mutation TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}
            local colors, metadata = wezterm.color.load_scheme('{scheme_file_query}')

            colors['background'] = '#3d3e3f'
            colors["cursor_bg"] = '#404142'
            colors[[[cursor_border]]] = '#434445'
            config.colors = colors

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme bracket-mutated colors config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(49, 50, 51));
        assert_eq!(effective.background_color, Color::Rgb(61, 62, 63));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(64, 65, 66));
        assert_eq!(effective.cursor_border_color, Some(Color::Rgb(67, 68, 69)));
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_applies_wezterm_lua_load_scheme_indexed_palette_mutations() {
        static NEXT_LOAD_SCHEME_INDEXED_MUTATION_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-indexed-mutation-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_INDEXED_MUTATION_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Loaded Scheme"

            [colors.indexed]
            136 = "#010203"
            "##,
        )
        .expect("expected temp load_scheme indexed mutation TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}
            local colors, metadata = wezterm.color.load_scheme('{scheme_file_query}')

            colors.indexed[136] = '#464748'
            colors.indexed[137] = '#494a4b'
            config.colors = colors

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme indexed palette mutation config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        let indexed = effective.indexed_palette.expect("expected indexed palette");
        assert_eq!(indexed[136], Some(Color::Rgb(70, 71, 72)));
        assert_eq!(indexed[137], Some(Color::Rgb(73, 74, 75)));
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_merges_wezterm_lua_load_scheme_indexed_palette_table_and_slot_mutations() {
        static NEXT_LOAD_SCHEME_INDEXED_MERGE_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-indexed-merge-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_INDEXED_MERGE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Loaded Scheme"

            [colors]
            foreground = "#010203"
            "##,
        )
        .expect("expected temp load_scheme indexed merge TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}
            local colors, metadata = wezterm.color.load_scheme('{scheme_file_query}')

            colors.indexed = {{
              [136] = '#4c4d4e',
            }}
            colors.indexed[137] = '#4f5051'
            config.colors = colors

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme indexed palette merged mutation config");
        app.set_config_overrides(overrides);

        let indexed = app
            .native_effective_config()
            .indexed_palette
            .expect("expected indexed palette");
        assert_eq!(indexed[136], Some(Color::Rgb(76, 77, 78)));
        assert_eq!(indexed[137], Some(Color::Rgb(79, 80, 81)));
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_applies_wezterm_lua_load_scheme_ansi_palette_slot_mutations() {
        static NEXT_LOAD_SCHEME_ANSI_SLOT_MUTATION_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-ansi-slot-mutation-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_ANSI_SLOT_MUTATION_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Loaded Scheme"

            [colors]
            ansi = [
              "#000001",
              "#000002",
              "#000003",
              "#000004",
              "#000005",
              "#000006",
              "#000007",
              "#000008",
            ]
            brights = [
              "#000009",
              "#00000a",
              "#00000b",
              "#00000c",
              "#00000d",
              "#00000e",
              "#00000f",
              "#000010",
            ]
            "##,
        )
        .expect("expected temp load_scheme ANSI slot mutation TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}
            local colors, metadata = wezterm.color.load_scheme('{scheme_file_query}')

            colors.ansi[2] = '#525354'
            colors.brights[8] = '#555657'
            config.colors = colors

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme ANSI slot mutation config");
        app.set_config_overrides(overrides);

        let palette = app
            .native_effective_config()
            .ansi_palette
            .expect("expected ANSI palette");
        assert_eq!(palette[0], Color::Rgb(0, 0, 1));
        assert_eq!(palette[1], Color::Rgb(82, 83, 84));
        assert_eq!(palette[8], Color::Rgb(0, 0, 9));
        assert_eq!(palette[15], Color::Rgb(85, 86, 87));
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_applies_wezterm_lua_load_scheme_ansi_palette_mutations_in_order() {
        static NEXT_LOAD_SCHEME_ANSI_ORDER_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-ansi-order-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_ANSI_ORDER_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Loaded Scheme"

            [colors]
            foreground = "#010203"
            "##,
        )
        .expect("expected temp load_scheme ANSI order TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}
            local colors, metadata = wezterm.color.load_scheme('{scheme_file_query}')

            colors.ansi[2] = '#58595a'
            colors.ansi = {{
              '#5b5c5d',
              '#5e5f60',
              '#616263',
              '#646566',
              '#676869',
              '#6a6b6c',
              '#6d6e6f',
              '#707172',
            }}
            config.colors = colors

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme ANSI mutation order config");
        app.set_config_overrides(overrides);

        let palette = app
            .native_effective_config()
            .ansi_palette
            .expect("expected ANSI palette");
        assert_eq!(palette[0], Color::Rgb(91, 92, 93));
        assert_eq!(palette[1], Color::Rgb(94, 95, 96));
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_applies_wezterm_lua_load_scheme_tab_bar_nested_mutations() {
        static NEXT_LOAD_SCHEME_TAB_BAR_MUTATION_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-tab-bar-mutation-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_TAB_BAR_MUTATION_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Loaded Scheme"

            [colors.tab_bar]
            background = "#010203"

            [colors.tab_bar.active_tab]
            fg_color = "#040506"
            bg_color = "#070809"
            intensity = "Bold"
            "##,
        )
        .expect("expected temp load_scheme tab-bar mutation TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}
            local colors, metadata = wezterm.color.load_scheme('{scheme_file_query}')

            colors.tab_bar.background = '#101112'
            colors.tab_bar.active_tab.bg_color = '#131415'
            config.colors = colors

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme tab-bar nested mutation config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.tab_bar_background_color,
            Some(Color::Rgb(16, 17, 18))
        );
        assert_eq!(
            effective.tab_bar_active_tab_colors.fg_color,
            Some(Color::Rgb(4, 5, 6))
        );
        assert_eq!(
            effective.tab_bar_active_tab_colors.bg_color,
            Some(Color::Rgb(19, 20, 21))
        );
        assert_eq!(
            effective.tab_bar_active_tab_colors.intensity,
            Some(NativeFormatIntensity::Bold)
        );
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_applies_wezterm_lua_load_scheme_tab_bar_inactive_tab_edge() {
        static NEXT_LOAD_SCHEME_TAB_BAR_EDGE_ID: AtomicUsize = AtomicUsize::new(0);

        let mut scheme_file = std::env::temp_dir();
        scheme_file.push(format!(
            "rterm-load-scheme-tab-bar-edge-{}-{}.toml",
            std::process::id(),
            NEXT_LOAD_SCHEME_TAB_BAR_EDGE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&scheme_file);
        std::fs::write(
            &scheme_file,
            r##"
            [metadata]
            name = "Loaded Scheme"

            [colors]
            foreground = "#010203"

            [colors.tab_bar]
            inactive_tab_edge = "#202122"
            "##,
        )
        .expect("expected temp load_scheme tab-bar inactive edge TOML color scheme");
        let scheme_file_query = scheme_file.to_string_lossy().replace('\\', "/");

        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(&format!(
            r##"
            local wezterm = require 'wezterm'
            local config = {{}}

            config.colors = wezterm.color.load_scheme('{scheme_file_query}')

            return config
            "##
        ))
        .expect("expected WezTerm load_scheme tab_bar inactive_tab_edge config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(1, 2, 3));
        assert_eq!(
            effective.tab_bar_inactive_tab_edge_color,
            Some(Color::Rgb(32, 33, 34))
        );
        let _ = std::fs::remove_file(scheme_file);
    }

    #[test]
    fn window_app_wezterm_lua_config_colors_override_custom_color_scheme() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.color_scheme = 'Project Scheme'
            config.color_schemes = {
              ['Project Scheme'] = {
                foreground = '#010203',
                background = '#040506',
              },
            }
            config.colors = {
              background = '#070809',
            }

            return config
            "##,
        )
        .expect("expected WezTerm color_scheme plus colors override config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(1, 2, 3));
        assert_eq!(effective.background_color, Color::Rgb(7, 8, 9));
    }

    #[test]
    fn window_app_uses_wezterm_lua_return_table_colors_after_config_assignment() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              foreground = '#010203',
              background = '#040506',
              cursor_bg = '#070809',
            }

            return {
              colors = {
                foreground = '#101112',
                background = '#131415',
                cursor_bg = '#161718',
              },
            }
            "##,
        )
        .expect("expected WezTerm return table colors config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(16, 17, 18));
        assert_eq!(effective.background_color, Color::Rgb(19, 20, 21));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(22, 23, 24));
    }

    #[test]
    fn window_app_uses_wezterm_lua_returned_config_variable_colors_after_config_assignment() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local cfg = {}

            config.colors = {
              foreground = '#010203',
              background = '#040506',
              cursor_bg = '#070809',
            }

            cfg.colors = {
              foreground = '#101112',
              background = '#131415',
              cursor_bg = '#161718',
            }

            return cfg
            "##,
        )
        .expect("expected returned WezTerm config variable colors config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.foreground_color, Color::Rgb(16, 17, 18));
        assert_eq!(effective.background_color, Color::Rgb(19, 20, 21));
        assert_eq!(effective.cursor_bg_color, Color::Rgb(22, 23, 24));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_ansi_palette_for_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              foreground = '#eeeeee',
              ansi = {
                '#000000',
                '#010203',
                '#030405',
                '#050607',
                '#070809',
                '#090a0b',
                '#0b0c0d',
                '#0d0e0f',
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm colors.ansi config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b[31mA").unwrap();
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        let uses_configured_ansi_red =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [1, 2, 3, 255])
            });
        assert!(
            uses_configured_ansi_red,
            "SGR 31 did not use colors.ansi red"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_brights_palette_for_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              foreground = '#eeeeee',
              brights = {
                '#101112',
                '#111213',
                '#121314',
                '#131415',
                '#141516',
                '#151617',
                '#161718',
                '#171819',
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm colors.brights config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b[91mA").unwrap();
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        let uses_configured_bright_red =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize).any(|x| {
                    frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [17, 18, 19, 255]
                })
            });
        assert!(
            uses_configured_bright_red,
            "SGR 91 did not use colors.brights red"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_indexed_palette_for_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              foreground = '#eeeeee',
              indexed = {
                [136] = '#010203',
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm colors.indexed config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"\x1b[38;5;136mA").unwrap();
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        let uses_configured_indexed_color =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [1, 2, 3, 255])
            });
        assert!(
            uses_configured_indexed_color,
            "SGR 38;5;136 did not use colors.indexed palette entry"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_selection_colors_for_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              selection_fg = '#010203',
              selection_bg = '#040506',
            }

            return config
            "##,
        )
        .expect("expected WezTerm selection color config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"A").unwrap();
        set_ordinary_viewport_selection_for_test(
            &mut app,
            WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 0 },
            ),
        );
        app.refresh_snapshot();
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        let selected_cell_uses_configured_background =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [4, 5, 6, 255])
            });
        let selected_cell_uses_configured_foreground =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [1, 2, 3, 255])
            });
        assert!(
            selected_cell_uses_configured_background,
            "selection did not use colors.selection_bg"
        );
        assert!(
            selected_cell_uses_configured_foreground,
            "selection did not use colors.selection_fg"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_rgb_function_colors_for_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              foreground = 'rgb(1,2,3)',
              background = 'rgb(4,5,6)',
              selection_fg = 'rgb(7,8,9)',
              selection_bg = 'rgb(10,11,12)',
            }

            return config
            "##,
        )
        .expect("expected WezTerm rgb function color config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"AB").unwrap();
        set_ordinary_viewport_selection_for_test(
            &mut app,
            WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 0 },
            ),
        );
        app.refresh_snapshot();
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        let selected_cell_uses_rgb_background =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize).any(|x| {
                    frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [10, 11, 12, 255]
                })
            });
        let selected_cell_uses_rgb_foreground =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [7, 8, 9, 255])
            });
        let plain_cell_uses_rgb_background =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (CELL_WIDTH as usize..(CELL_WIDTH * 2) as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [4, 5, 6, 255])
            });
        let plain_cell_uses_rgb_foreground =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (CELL_WIDTH as usize..(CELL_WIDTH * 2) as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [1, 2, 3, 255])
            });
        assert!(
            selected_cell_uses_rgb_background,
            "selection_bg rgb() did not render"
        );
        assert!(
            selected_cell_uses_rgb_foreground,
            "selection_fg rgb() did not render"
        );
        assert!(
            plain_cell_uses_rgb_background,
            "background rgb() did not render"
        );
        assert!(
            plain_cell_uses_rgb_foreground,
            "foreground rgb() did not render"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_css_rgb_space_percent_colors_for_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              foreground = 'rgb(100% 0% 0%)',
              background = 'rgb(0 255 0 / 100%)',
              selection_fg = 'rgb(0% 0% 100%)',
              selection_bg = 'rgb(50% 50% 50% / 50%)',
            }

            return config
            "##,
        )
        .expect("expected WezTerm CSS rgb color config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"AB").unwrap();
        set_ordinary_viewport_selection_for_test(
            &mut app,
            WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 0 },
            ),
        );
        app.refresh_snapshot();
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        let selected_cell_uses_blended_percent_background =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize).any(|x| {
                    frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [63, 191, 63, 255]
                })
            });
        let selected_cell_uses_percent_foreground =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [0, 0, 255, 255])
            });
        let plain_cell_uses_space_rgb_background =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (CELL_WIDTH as usize..(CELL_WIDTH * 2) as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [0, 255, 0, 255])
            });
        let plain_cell_uses_percent_foreground =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (CELL_WIDTH as usize..(CELL_WIDTH * 2) as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [255, 0, 0, 255])
            });
        assert!(
            selected_cell_uses_blended_percent_background,
            "selection_bg CSS rgb percentage alpha did not blend over current background"
        );
        assert!(
            selected_cell_uses_percent_foreground,
            "selection_fg CSS rgb percentages did not render"
        );
        assert!(
            plain_cell_uses_space_rgb_background,
            "background CSS rgb space/slash syntax did not render"
        );
        assert!(
            plain_cell_uses_percent_foreground,
            "foreground CSS rgb percentages did not render"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_css_rgba_space_percent_selection_colors() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local config = {}

            config.colors = {
              selection_bg = 'rgba(26.666668% 27.843138% 35.294117% 50%)',
              selection_fg = 'rgba(0% 0% 0% 0%)',
            }

            return config
            "##,
        )
        .expect("expected WezTerm CSS rgba space percentage color config");

        assert_eq!(
            overrides.selection_bg_color,
            Some(Color::Rgba(68, 71, 89, 127))
        );
        assert_eq!(overrides.selection_fg_color, Some(None));
    }

    #[test]
    fn window_app_ignores_wezterm_non_selection_color_alpha_for_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              foreground = 'rgba(255,0,0,0.5)',
              background = 'rgba(0,255,0,0.5)',
            }

            return config
            "##,
        )
        .expect("expected WezTerm non-selection rgba config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"A").unwrap();
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        let cell_uses_opaque_background =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [0, 255, 0, 255])
            });
        let cell_uses_opaque_foreground =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [255, 0, 0, 255])
            });
        assert!(
            cell_uses_opaque_background,
            "non-selection background alpha was not ignored"
        );
        assert!(
            cell_uses_opaque_foreground,
            "non-selection foreground alpha was not ignored"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_hsl_colors_for_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              foreground = 'hsl:0 100 50',
              background = 'hsl(120,100%,50%)',
              selection_fg = 'hsl(-240 100% 50%)',
              selection_bg = 'hsl(240deg 100% 50%)',
            }

            return config
            "##,
        )
        .expect("expected WezTerm HSL color config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"AB").unwrap();
        set_ordinary_viewport_selection_for_test(
            &mut app,
            WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 0 },
            ),
        );
        app.refresh_snapshot();
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        let selected_cell_uses_hsl_background =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [0, 0, 255, 255])
            });
        let selected_cell_uses_hsl_foreground =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [0, 255, 0, 255])
            });
        let plain_cell_uses_hsl_background =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (CELL_WIDTH as usize..(CELL_WIDTH * 2) as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [0, 255, 0, 255])
            });
        let plain_cell_uses_hsl_foreground =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (CELL_WIDTH as usize..(CELL_WIDTH * 2) as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [255, 0, 0, 255])
            });
        assert!(
            selected_cell_uses_hsl_background,
            "selection_bg HSL color did not render"
        );
        assert!(
            selected_cell_uses_hsl_foreground,
            "selection_fg HSL color did not render"
        );
        assert!(
            plain_cell_uses_hsl_background,
            "background HSL color did not render"
        );
        assert!(
            plain_cell_uses_hsl_foreground,
            "foreground HSL color did not render"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_hsla_colors_for_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              foreground = 'hsla(0,100%,50%,25%)',
              background = 'hsla(120,100%,50%,25%)',
              selection_fg = 'hsla(120,100%,50%,100%)',
              selection_bg = 'hsla(240,100%,50%,50%)',
            }

            return config
            "##,
        )
        .expect("expected WezTerm HSLA color config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"AB").unwrap();
        set_ordinary_viewport_selection_for_test(
            &mut app,
            WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 0 },
            ),
        );
        app.refresh_snapshot();
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        let selected_cell_uses_hsla_background =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize).any(|x| {
                    frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [0, 128, 127, 255]
                })
            });
        let selected_cell_uses_hsla_foreground =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [0, 255, 0, 255])
            });
        let plain_cell_uses_opaque_hsla_background =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (CELL_WIDTH as usize..(CELL_WIDTH * 2) as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [0, 255, 0, 255])
            });
        let plain_cell_uses_opaque_hsla_foreground =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (CELL_WIDTH as usize..(CELL_WIDTH * 2) as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [255, 0, 0, 255])
            });
        assert!(
            selected_cell_uses_hsla_background,
            "selection_bg HSLA alpha did not blend over the current background"
        );
        assert!(
            selected_cell_uses_hsla_foreground,
            "selection_fg HSLA color did not render"
        );
        assert!(
            plain_cell_uses_opaque_hsla_background,
            "non-selection HSLA background alpha was not ignored"
        );
        assert!(
            plain_cell_uses_opaque_hsla_foreground,
            "non-selection HSLA foreground alpha was not ignored"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_hsv_colors_for_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              foreground = 'hsv(0,100%,100%)',
              background = 'hsv(120,100%,100%)',
              selection_fg = 'hsv(120deg 100% 100%)',
              selection_bg = 'hsv(240deg 100% 100% / 50%)',
            }

            return config
            "##,
        )
        .expect("expected WezTerm HSV color config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"AB").unwrap();
        set_ordinary_viewport_selection_for_test(
            &mut app,
            WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 0 },
            ),
        );
        app.refresh_snapshot();
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        let selected_cell_uses_hsv_background =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize).any(|x| {
                    frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [0, 128, 127, 255]
                })
            });
        let selected_cell_uses_hsv_foreground =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [0, 255, 0, 255])
            });
        let plain_cell_uses_hsv_background =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (CELL_WIDTH as usize..(CELL_WIDTH * 2) as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [0, 255, 0, 255])
            });
        let plain_cell_uses_hsv_foreground =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (CELL_WIDTH as usize..(CELL_WIDTH * 2) as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [255, 0, 0, 255])
            });
        assert!(
            selected_cell_uses_hsv_background,
            "selection_bg HSV alpha did not blend over the current background"
        );
        assert!(
            selected_cell_uses_hsv_foreground,
            "selection_fg HSV color did not render"
        );
        assert!(
            plain_cell_uses_hsv_background,
            "background HSV color did not render"
        );
        assert!(
            plain_cell_uses_hsv_foreground,
            "foreground HSV color did not render"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_hwb_colors_for_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              foreground = 'hwb(0 0% 0% / 25%)',
              background = 'hwb(120 0% 0% / 25%)',
              selection_fg = 'hwb(480deg 0% 0%)',
              selection_bg = 'hwb(240deg 0% 0% / 50%)',
            }

            return config
            "##,
        )
        .expect("expected WezTerm HWB color config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"AB").unwrap();
        set_ordinary_viewport_selection_for_test(
            &mut app,
            WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 0 },
            ),
        );
        app.refresh_snapshot();
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        let selected_cell_uses_hwb_background =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize).any(|x| {
                    frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [0, 128, 127, 255]
                })
            });
        let selected_cell_uses_hwb_foreground =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [0, 255, 0, 255])
            });
        let plain_cell_uses_opaque_hwb_background =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (CELL_WIDTH as usize..(CELL_WIDTH * 2) as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [0, 255, 0, 255])
            });
        let plain_cell_uses_opaque_hwb_foreground =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (CELL_WIDTH as usize..(CELL_WIDTH * 2) as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [255, 0, 0, 255])
            });
        assert!(
            selected_cell_uses_hwb_background,
            "selection_bg HWB alpha did not blend over the current background"
        );
        assert!(
            selected_cell_uses_hwb_foreground,
            "selection_fg HWB color did not render"
        );
        assert!(
            plain_cell_uses_opaque_hwb_background,
            "non-selection HWB background alpha was not ignored"
        );
        assert!(
            plain_cell_uses_opaque_hwb_foreground,
            "non-selection HWB foreground alpha was not ignored"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_named_colors_for_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              foreground = 'red',
              background = 'lime',
              selection_fg = 'silver',
              selection_bg = 'navy',
            }

            return config
            "##,
        )
        .expect("expected WezTerm named color config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"AB").unwrap();
        set_ordinary_viewport_selection_for_test(
            &mut app,
            WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 0 },
            ),
        );
        app.refresh_snapshot();
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        let selected_cell_uses_named_background =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [0, 0, 128, 255])
            });
        let selected_cell_uses_named_foreground =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize).any(|x| {
                    frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [192, 192, 192, 255]
                })
            });
        let plain_cell_uses_named_background =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (CELL_WIDTH as usize..(CELL_WIDTH * 2) as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [0, 255, 0, 255])
            });
        let plain_cell_uses_named_foreground =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (CELL_WIDTH as usize..(CELL_WIDTH * 2) as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [255, 0, 0, 255])
            });
        assert!(
            selected_cell_uses_named_background,
            "selection_bg named color did not render"
        );
        assert!(
            selected_cell_uses_named_foreground,
            "selection_fg named color did not render"
        );
        assert!(
            plain_cell_uses_named_background,
            "background named color did not render"
        );
        assert!(
            plain_cell_uses_named_foreground,
            "foreground named color did not render"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_selection_fg_none_for_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              foreground = '#090a0b',
              selection_fg = 'none',
              selection_bg = '#040506',
            }

            return config
            "##,
        )
        .expect("expected WezTerm selection_fg none config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"A").unwrap();
        set_ordinary_viewport_selection_for_test(
            &mut app,
            WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 0 },
            ),
        );
        app.refresh_snapshot();
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        let selected_cell_uses_configured_background =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [4, 5, 6, 255])
            });
        let selected_cell_uses_current_foreground =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [9, 10, 11, 255])
            });
        assert!(
            selected_cell_uses_configured_background,
            "selection did not use colors.selection_bg"
        );
        assert!(
            selected_cell_uses_current_foreground,
            "selection_fg none did not preserve the current text foreground"
        );
    }

    #[test]
    fn window_app_blends_wezterm_lua_selection_bg_alpha_for_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              background = '#0a141e',
              selection_fg = 'none',
              selection_bg = 'rgba(110,120,130,0.5)',
            }

            return config
            "##,
        )
        .expect("expected WezTerm selection_bg alpha config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"A").unwrap();
        set_ordinary_viewport_selection_for_test(
            &mut app,
            WindowSelection::new(
                SelectionCell { row: 0, column: 0 },
                SelectionCell { row: 0, column: 0 },
            ),
        );
        app.refresh_snapshot();
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        let selected_cell_uses_blended_background =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize).any(|x| {
                    frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [59, 69, 79, 255]
                })
            });
        assert!(
            selected_cell_uses_blended_background,
            "selection_bg alpha did not blend over the current cell background"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_cursor_bg_for_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              cursor_bg = '#070809',
            }

            return config
            "##,
        )
        .expect("expected WezTerm colors.cursor_bg config");
        app.set_config_overrides(overrides);
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        assert_eq!(
            frame_pixel_at(&frame, FRAME_WIDTH as usize, 0, terminal_origin_y),
            [7, 8, 9, 255]
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_cursor_fg_for_framebuffer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.colors = {
              cursor_bg = '#070809',
              cursor_fg = '#010203',
            }

            return config
            "##,
        )
        .expect("expected WezTerm colors.cursor_fg config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"A\r").unwrap();
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        let cursor_cell_has_configured_text =
            (terminal_origin_y..terminal_origin_y + CELL_HEIGHT as usize).any(|y| {
                (0..CELL_WIDTH as usize)
                    .any(|x| frame_pixel_at(&frame, FRAME_WIDTH as usize, x, y) == [1, 2, 3, 255])
            });
        assert!(
            cursor_cell_has_configured_text,
            "cursor text did not use colors.cursor_fg"
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_cursor_border_for_line_cursor() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_cursor_style = 'SteadyUnderline'
            config.colors = {
              cursor_bg = '#070809',
              cursor_border = '#010203',
            }

            return config
            "##,
        )
        .expect("expected WezTerm colors.cursor_border config");
        app.set_config_overrides(overrides);
        let mut frame = vec![0; usize::try_from(FRAME_WIDTH * FRAME_HEIGHT * 4).unwrap()];

        assert_eq!(app.render_framebuffer(&mut frame), FrameRenderMode::Full);

        let terminal_origin_y = usize::from(TAB_BAR_ROWS) * CELL_HEIGHT as usize;
        assert_eq!(
            frame_pixel_at(
                &frame,
                FRAME_WIDTH as usize,
                0,
                terminal_origin_y + CELL_HEIGHT as usize - 1
            ),
            [1, 2, 3, 255]
        );
    }

