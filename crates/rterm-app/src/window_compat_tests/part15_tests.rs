    #[test]
    fn window_app_reports_default_wezterm_dpi_by_screen_config() {
        let app = NativeWindowApp::new(None);

        assert!(app.native_effective_config().dpi_by_screen.is_empty());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_dpi_by_screen() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.dpi_by_screen = {
                ['Built-in Retina Display'] = 144.0,
                HDMI = 120.0,
            }

            return config
            "#,
        )
        .expect("expected WezTerm dpi_by_screen config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().dpi_by_screen,
            BTreeMap::from([
                ("Built-in Retina Display".to_owned(), 144),
                ("HDMI".to_owned(), 120),
            ])
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_dpi_by_screen_static_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local retina_dpi = 144.0
            local screen_dpi = {
                ['Built-in Retina Display'] = retina_dpi,
                HDMI = 96.0,
            }

            config.dpi_by_screen = screen_dpi

            return config
            "#,
        )
        .expect("expected WezTerm dpi_by_screen static-variable config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().dpi_by_screen,
            BTreeMap::from([
                ("Built-in Retina Display".to_owned(), 144),
                ("HDMI".to_owned(), 96),
            ])
        );
    }

    #[test]
    fn window_app_reports_default_wezterm_serial_ports_config() {
        let app = NativeWindowApp::new(None);

        assert!(app.native_effective_config().serial_ports.is_empty());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_serial_ports() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.serial_ports = {
              { name = 'dev-console', port = 'COM3', baud = 115200 },
              { name = 'usb-debug', port = '/dev/ttyUSB0' },
              { name = 'named-default' },
            }

            return config
            "#,
        )
        .expect("expected WezTerm serial_ports config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().serial_ports,
            vec![
                NativeSerialDomain {
                    name: "dev-console".to_owned(),
                    port: Some("COM3".to_owned()),
                    baud: Some(115_200),
                },
                NativeSerialDomain {
                    name: "usb-debug".to_owned(),
                    port: Some("/dev/ttyUSB0".to_owned()),
                    baud: None,
                },
                NativeSerialDomain {
                    name: "named-default".to_owned(),
                    port: None,
                    baud: None,
                },
            ]
        );
    }

    #[test]
    fn window_app_reports_default_wezterm_exec_domains_config() {
        let app = NativeWindowApp::new(None);

        assert!(app.native_effective_config().exec_domains.is_empty());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_exec_domains() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.exec_domains = {
              wezterm.exec_domain('build', function(cmd)
                return cmd
              end, 'Build Host'),
              wezterm.exec_domain('deploy', function(cmd)
                table.insert(cmd.args, '--deploy')
                return cmd
              end, function(domain)
                return 'Deploy ' .. domain
              end),
            }

            return config
            "#,
        )
        .expect("expected WezTerm exec_domains config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().exec_domains,
            vec![
                NativeExecDomain {
                    name: "build".to_owned(),
                    fixup_command: "exec-domain-build".to_owned(),
                    label: Some(NativeExecDomainLabel::Value("Build Host".to_owned())),
                },
                NativeExecDomain {
                    name: "deploy".to_owned(),
                    fixup_command: "exec-domain-deploy".to_owned(),
                    label: Some(NativeExecDomainLabel::Function(
                        "exec-domain-deploy-label".to_owned(),
                    )),
                },
            ]
        );
    }

    #[test]
    fn window_app_reports_default_wezterm_wsl_domains_config() {
        let app = NativeWindowApp::new(None);

        assert!(app.native_effective_config().wsl_domains.is_empty());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_wsl_domains() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local ubuntu_prog = { 'zsh', '-l' }

            config.wsl_domains = {
              {
                name = 'WSL:Ubuntu',
                distribution = 'Ubuntu',
                username = 'ops',
                default_cwd = '~',
                default_prog = ubuntu_prog,
              },
              {
                name = 'WSL:Debian',
                distribution = 'Debian',
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm wsl_domains config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().wsl_domains,
            vec![
                NativeWslDomain {
                    name: "WSL:Ubuntu".to_owned(),
                    distribution: Some("Ubuntu".to_owned()),
                    username: Some("ops".to_owned()),
                    default_cwd: Some("~".to_owned()),
                    default_prog: Some(vec!["zsh".to_owned(), "-l".to_owned()]),
                },
                NativeWslDomain {
                    name: "WSL:Debian".to_owned(),
                    distribution: Some("Debian".to_owned()),
                    username: None,
                    default_cwd: None,
                    default_prog: None,
                },
            ]
        );
    }

    #[test]
    fn window_app_reports_default_wezterm_unix_domains_config() {
        let app = NativeWindowApp::new(None);

        assert_eq!(
            app.native_effective_config().unix_domains,
            vec![NativeUnixDomain {
                name: "unix".to_owned(),
                socket_path: None,
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
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_unix_domains() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local serve = { 'wsl', '-e', 'wezterm-mux-server', '--daemonize' }

            config.unix_domains = {
              {
                name = 'wsl-mux',
                socket_path = '/tmp/wezterm.sock',
                connect_automatically = true,
                no_serve_automatically = true,
                serve_command = serve,
                proxy_command = { 'ssh', 'dev', 'wezterm', 'cli', 'proxy' },
                skip_permissions_check = true,
                local_echo_threshold_ms = 15,
                overlay_lag_indicator = true,
              },
              { name = 'local-alt' },
            }

            return config
            "#,
        )
        .expect("expected WezTerm unix_domains config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().unix_domains,
            vec![
                NativeUnixDomain {
                    name: "wsl-mux".to_owned(),
                    socket_path: Some("/tmp/wezterm.sock".to_owned()),
                    connect_automatically: true,
                    no_serve_automatically: true,
                    serve_command: Some(vec![
                        "wsl".to_owned(),
                        "-e".to_owned(),
                        "wezterm-mux-server".to_owned(),
                        "--daemonize".to_owned(),
                    ]),
                    proxy_command: Some(vec![
                        "ssh".to_owned(),
                        "dev".to_owned(),
                        "wezterm".to_owned(),
                        "cli".to_owned(),
                        "proxy".to_owned(),
                    ]),
                    skip_permissions_check: true,
                    read_timeout_ms: 60_000,
                    write_timeout_ms: 60_000,
                    local_echo_threshold_ms: Some(15),
                    overlay_lag_indicator: true,
                },
                NativeUnixDomain {
                    name: "local-alt".to_owned(),
                    socket_path: None,
                    connect_automatically: false,
                    no_serve_automatically: false,
                    serve_command: None,
                    proxy_command: None,
                    skip_permissions_check: false,
                    read_timeout_ms: 60_000,
                    write_timeout_ms: 60_000,
                    local_echo_threshold_ms: None,
                    overlay_lag_indicator: false,
                },
            ]
        );
    }

    #[test]
    fn window_app_reports_default_wezterm_ssh_domains_config() {
        let app = NativeWindowApp::new(None);

        assert!(app.native_effective_config().ssh_domains.is_empty());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_ssh_domains() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local opts = {
              compression = 'yes',
              proxycommand = 'ssh bastion -W %h:%p',
            }
            local prog = { 'zsh', '-l' }

            config.ssh_domains = {
              {
                name = 'prod',
                remote_address = 'prod.example.com:2222',
                no_agent_auth = true,
                username = 'deploy',
                connect_automatically = true,
                timeout = 45000,
                local_echo_threshold_ms = 25,
                overlay_lag_indicator = true,
                remote_wezterm_path = '/opt/wezterm/wezterm',
                override_proxy_command = 'wezterm cli proxy --stdio',
                ssh_backend = 'Ssh2',
                multiplexing = 'None',
                ssh_option = opts,
                default_prog = prog,
                assume_shell = 'Posix',
              },
              {
                name = 'mux',
                remote_address = 'mux.example.com',
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm ssh_domains config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().ssh_domains,
            vec![
                NativeSshDomain {
                    name: "prod".to_owned(),
                    remote_address: "prod.example.com:2222".to_owned(),
                    no_agent_auth: true,
                    username: Some("deploy".to_owned()),
                    connect_automatically: true,
                    timeout_ms: 45_000,
                    local_echo_threshold_ms: Some(25),
                    overlay_lag_indicator: true,
                    remote_wezterm_path: Some("/opt/wezterm/wezterm".to_owned()),
                    override_proxy_command: Some("wezterm cli proxy --stdio".to_owned()),
                    ssh_backend: Some(NativeSshBackend::Ssh2),
                    multiplexing: NativeSshMultiplexing::None,
                    ssh_option: BTreeMap::from([
                        ("compression".to_owned(), "yes".to_owned()),
                        ("proxycommand".to_owned(), "ssh bastion -W %h:%p".to_owned()),
                    ]),
                    default_prog: Some(vec!["zsh".to_owned(), "-l".to_owned()]),
                    assume_shell: NativeShellAssumption::Posix,
                },
                NativeSshDomain {
                    name: "mux".to_owned(),
                    remote_address: "mux.example.com".to_owned(),
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
                },
            ]
        );
    }

    #[test]
    fn window_app_reports_default_wezterm_tls_domains_config() {
        let app = NativeWindowApp::new(None);
        let effective = app.native_effective_config();

        assert!(effective.tls_servers.is_empty());
        assert!(effective.tls_clients.is_empty());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_tls_domains() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local roots = { '/etc/ssl/certs', '/opt/wezterm/ca.pem' }

            config.tls_servers = {
              {
                bind_address = '127.0.0.1:8080',
                pem_private_key = '/etc/wezterm/server.key',
                pem_cert = '/etc/wezterm/server.crt',
                pem_ca = '/etc/wezterm/ca.pem',
                pem_root_certs = roots,
              },
            }

            config.tls_clients = {
              {
                name = 'tls-prod',
                bootstrap_via_ssh = 'deploy@bastion.example.com:22',
                remote_address = 'prod.example.com:8443',
                pem_private_key = '/home/me/client.key',
                pem_cert = '/home/me/client.crt',
                pem_ca = '/home/me/ca.pem',
                pem_root_certs = roots,
                accept_invalid_hostnames = true,
                expected_cn = 'prod.internal',
                connect_automatically = true,
                read_timeout = 45000,
                write_timeout = 30000,
                local_echo_threshold_ms = 25,
                remote_wezterm_path = '/opt/wezterm/wezterm',
                overlay_lag_indicator = true,
              },
              {
                name = 'tls-minimal',
                remote_address = 'minimal.example.com:443',
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm TLS domain config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.tls_servers,
            vec![NativeTlsServerDomain {
                bind_address: "127.0.0.1:8080".to_owned(),
                pem_private_key: Some("/etc/wezterm/server.key".to_owned()),
                pem_cert: Some("/etc/wezterm/server.crt".to_owned()),
                pem_ca: Some("/etc/wezterm/ca.pem".to_owned()),
                pem_root_certs: vec![
                    "/etc/ssl/certs".to_owned(),
                    "/opt/wezterm/ca.pem".to_owned(),
                ],
            }]
        );
        assert_eq!(
            effective.tls_clients,
            vec![
                NativeTlsClientDomain {
                    name: "tls-prod".to_owned(),
                    bootstrap_via_ssh: Some("deploy@bastion.example.com:22".to_owned()),
                    remote_address: "prod.example.com:8443".to_owned(),
                    pem_private_key: Some("/home/me/client.key".to_owned()),
                    pem_cert: Some("/home/me/client.crt".to_owned()),
                    pem_ca: Some("/home/me/ca.pem".to_owned()),
                    pem_root_certs: vec![
                        "/etc/ssl/certs".to_owned(),
                        "/opt/wezterm/ca.pem".to_owned(),
                    ],
                    accept_invalid_hostnames: true,
                    expected_cn: Some("prod.internal".to_owned()),
                    connect_automatically: true,
                    read_timeout_ms: 45_000,
                    write_timeout_ms: 30_000,
                    local_echo_threshold_ms: Some(25),
                    remote_wezterm_path: Some("/opt/wezterm/wezterm".to_owned()),
                    overlay_lag_indicator: true,
                },
                NativeTlsClientDomain {
                    name: "tls-minimal".to_owned(),
                    bootstrap_via_ssh: None,
                    remote_address: "minimal.example.com:443".to_owned(),
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
                },
            ]
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn test_window_scale_factor_accepts_supported_windows_buckets_only() {
        assert_eq!(parse_test_window_scale_factor("1"), Some(1.0));
        assert_eq!(parse_test_window_scale_factor("1.25"), Some(1.25));
        assert_eq!(parse_test_window_scale_factor("1.5"), Some(1.5));
        for invalid in ["0", "0.49", "4.01", "nan", "not-a-scale"] {
            assert_eq!(
                parse_test_window_scale_factor(invalid),
                None,
                "invalid scale factor {invalid:?} must be ignored"
            );
        }
    }

    #[test]
    fn window_app_configured_dpi_overrides_detected_scale_factor() {
        let mut app = NativeWindowApp::new(None);
        app.apply_window_scale_factor(2.0);
        assert_eq!(app.window_dpi, 192);

        app.set_config_overrides(native_config_snapshot! {
            dpi: Some(144),
            ..NativeConfigSnapshot::default()
        });
        assert_eq!(app.window_dpi, 144);

        app.apply_window_scale_factor(1.0);
        assert_eq!(app.window_dpi, 144);

        app.set_config_overrides(NativeConfigSnapshot::default());
        assert_eq!(app.window_dpi, 96);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_freetype_target_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.freetype_load_target = 'Light'

            return config
            "#,
        )
        .expect("expected WezTerm freetype load target config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.freetype_load_target, NativeFreetypeTarget::Light);
        assert_eq!(
            effective.freetype_render_target,
            NativeFreetypeTarget::Light
        );

        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.freetype_load_target = 'Mono'
            config.freetype_render_target = 'HorizontalLcd'

            return config
            "#,
        )
        .expect("expected WezTerm freetype load/render target config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.freetype_load_target, NativeFreetypeTarget::Mono);
        assert_eq!(
            effective.freetype_render_target,
            NativeFreetypeTarget::HorizontalLcd
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_freetype_load_flags() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.freetype_load_flags = 'NO_HINTING|MONOCHROME'

            return config
            "#,
        )
        .expect("expected WezTerm freetype load flags config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.freetype_load_flags,
            NativeFreetypeLoadFlags::NO_HINTING.union(NativeFreetypeLoadFlags::MONOCHROME)
        );

        let mut high_dpi_app = NativeWindowApp::new(None);
        high_dpi_app.window_dpi = 144;
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.freetype_load_flags = 'DEFAULT'

            return config
            "#,
        )
        .expect("expected explicit WezTerm default freetype load flags config");
        high_dpi_app.set_config_overrides(overrides);

        let effective = high_dpi_app.native_effective_config();
        assert_eq!(
            effective.freetype_load_flags,
            NativeFreetypeLoadFlags::DEFAULT
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_freetype_pcf_long_family_names() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.freetype_pcf_long_family_names = true

            return config
            "#,
        )
        .expect("expected WezTerm FreeType PCF long-family-names config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert!(effective.freetype_pcf_long_family_names);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_freetype_interpreter_version() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.freetype_interpreter_version = 38

            return config
            "#,
        )
        .expect("expected WezTerm FreeType interpreter version config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.freetype_interpreter_version, Some(38));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_display_pixel_geometry() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.display_pixel_geometry = 'BGR'

            return config
            "#,
        )
        .expect("expected WezTerm display pixel geometry config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.display_pixel_geometry,
            NativeDisplayPixelGeometry::Bgr
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_shaper() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.font_shaper = 'Harfbuzz'

            return config
            "#,
        )
        .expect("expected WezTerm font shaper config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.font_shaper, NativeFontShaper::Harfbuzz);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_harfbuzz_features() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local features = { 'kern', 'liga=0' }
            table.insert(features, 'calt=0')

            config.harfbuzz_features = features

            return config
            "#,
        )
        .expect("expected WezTerm Harfbuzz features config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.harfbuzz_features,
            vec!["kern".to_owned(), "liga=0".to_owned(), "calt=0".to_owned()]
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_dirs_and_locator() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.font_dirs = { 'fonts', 'vendor/fonts' }
            config.font_locator = 'ConfigDirsOnly'

            return config
            "#,
        )
        .expect("expected WezTerm font dirs and locator config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.font_dirs,
            vec!["fonts".to_owned(), "vendor/fonts".to_owned()]
        );
        assert_eq!(
            effective.font_locator,
            Some(NativeFontLocator::ConfigDirsOnly)
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_dirs_static_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local dirs = { 'fonts', 'vendor/fonts' }

            config.font_dirs = dirs
            config.font_locator = 'ConfigDirsOnly'

            return config
            "#,
        )
        .expect("expected WezTerm font dirs table variable config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.font_dirs,
            vec!["fonts".to_owned(), "vendor/fonts".to_owned()]
        );
        assert_eq!(
            effective.font_locator,
            Some(NativeFontLocator::ConfigDirsOnly)
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_dirs_table_insert() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.font_dirs = {}
            table.insert(config.font_dirs, 'fonts')
            table.insert(config.font_dirs, 'vendor/fonts')
            config.font_locator = 'ConfigDirsOnly'

            return config
            "#,
        )
        .expect("expected WezTerm font dirs table insert config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.font_dirs,
            vec!["fonts".to_owned(), "vendor/fonts".to_owned()]
        );
        assert_eq!(
            effective.font_locator,
            Some(NativeFontLocator::ConfigDirsOnly)
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_custom_glyph_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}

            config.custom_block_glyphs = false
            config.anti_alias_custom_block_glyphs = false
            config.allow_square_glyphs_to_overflow_width = 'Always'

            return config
            "#,
        )
        .expect("expected WezTerm custom glyph config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert!(!effective.custom_block_glyphs);
        assert!(!effective.anti_alias_custom_block_glyphs);
        assert_eq!(
            effective.allow_square_glyphs_to_overflow_width,
            NativeSquareGlyphOverflow::Always
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_font_and_cursor_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.font_size = 13.5
            config.cell_width = 1.25
            config.line_height = 1.5
            config.font_antialias = 'Subpixel'
            config.font_hinting = 'VerticalSubpixel'
            config.font_rasterizer = 'FreeType'
            config.initial_cols = 100
            config.initial_rows = 30
            config.adjust_window_size_when_changing_font_size = false
            config.cursor_blink_rate = 375
            config.cursor_blink_ease_in = 'EaseIn'
            config.cursor_blink_ease_out = 'EaseOut'
            config.default_cursor_style = 'BlinkingBar'
            config.force_reverse_video_cursor = true
            config.reverse_video_cursor_min_contrast = 3.25

            return config
            "#,
        )
        .expect("expected WezTerm font/cursor config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.font_size,
            NativeFontSize::from_millipoints(13_500)
        );
        assert_eq!(effective.cell_width, NativeCellWidth::from_per_mille(1_250));
        assert_eq!(
            effective.line_height,
            NativeLineHeight::from_per_mille(1_500)
        );
        assert_eq!(effective.font_antialias, NativeFontAntialias::Subpixel);
        assert_eq!(effective.font_hinting, NativeFontHinting::VerticalSubpixel);
        assert_eq!(effective.font_rasterizer, NativeFontRasterizer::FreeType);
        assert_eq!(effective.initial_cols, 100);
        assert_eq!(effective.initial_rows, 30);
        assert!(!effective.adjust_window_size_when_changing_font_size);
        assert_eq!(effective.cursor_blink_rate, 375);
        assert_eq!(effective.cursor_blink_rate_ms, 375);
        assert_eq!(effective.cursor_blink_ease_in, NativeEasingFunction::EaseIn);
        assert_eq!(
            effective.cursor_blink_ease_out,
            NativeEasingFunction::EaseOut
        );
        assert_eq!(
            effective.default_cursor_style,
            NativeCursorStyle::BlinkingBar
        );
        assert!(effective.force_reverse_video_cursor);
        assert_eq!(
            effective.reverse_video_cursor_min_contrast,
            NativeContrastRatio::from_centi(325)
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_bell_and_notification_overrides() {
        let audible_bells = Arc::new(Mutex::new(Vec::new()));
        let recorded_audible = Arc::clone(&audible_bells);
        let mut app = NativeWindowApp::new(None);
        app.audible_bell_handler = Box::new(move |bell| {
            recorded_audible.lock().unwrap().push(*bell);
            true
        });
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.audible_bell = 'Disabled'
            config.visual_bell = {
              fade_in_duration_ms = 0,
              fade_out_duration_ms = 150,
              fade_in_function = 'EaseIn',
              fade_out_function = 'EaseOut',
              target = 'BackgroundColor',
            }
            config.colors = {
              visual_bell = '#010203',
            }
            config.notification_handling = 'SuppressFromFocusedWindow'

            return config
            "##,
        )
        .expect("expected WezTerm bell/notification config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.audible_bell, NativeAudibleBell::Disabled);
        assert_eq!(
            effective.visual_bell,
            NativeVisualBell {
                fade_in_duration_ms: 0,
                fade_out_duration_ms: 150,
                fade_in_function: NativeEasingFunction::EaseIn,
                fade_out_function: NativeEasingFunction::EaseOut,
                target: NativeVisualBellTarget::BackgroundColor,
            }
        );
        assert_eq!(effective.visual_bell_color, Some(Color::Rgb(1, 2, 3)));
        assert_eq!(
            effective.notification_handling,
            NativeNotificationHandling::SuppressFromFocusedWindow
        );

        app.handle_pty_output(b"\x1b[31mA\x07").unwrap();

        assert!(audible_bells.lock().unwrap().is_empty());
        let snapshot = app.render_snapshot();
        let cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("visible cell");
        assert_eq!(cell.background, Color::Rgb(1, 2, 3));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_visual_bell_static_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local config = {}
            local project_visual_bell = {
              fade_in_duration_ms = 10,
              fade_out_duration_ms = 200,
              fade_in_function = 'Linear',
              fade_out_function = 'EaseInOut',
              target = 'CursorColor',
            }

            config.term = 'xterm-256color'
            config.visual_bell = project_visual_bell

            return config
            "##,
        )
        .expect("expected WezTerm visual bell static variable config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().visual_bell,
            NativeVisualBell {
                fade_in_duration_ms: 10,
                fade_out_duration_ms: 200,
                fade_in_function: NativeEasingFunction::Linear,
                fade_out_function: NativeEasingFunction::EaseInOut,
                target: NativeVisualBellTarget::CursorColor,
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_visual_bell_static_field_variables() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local config = {}
            local fade_in_ms = 25
            local fade_out_ms = 175
            local fade_in_easing = 'Linear'
            local fade_out_easing = { CubicBezier = { 0.0, 0.0, 0.58, 1.0 } }
            local bell_target = 'CursorColor'

            config.visual_bell = {
              fade_in_duration_ms = fade_in_ms,
              fade_out_duration_ms = fade_out_ms,
              fade_in_function = fade_in_easing,
              fade_out_function = fade_out_easing,
              target = bell_target,
            }

            return config
            "##,
        )
        .expect("expected WezTerm visual bell static field variable config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().visual_bell,
            NativeVisualBell {
                fade_in_duration_ms: 25,
                fade_out_duration_ms: 175,
                fade_in_function: NativeEasingFunction::Linear,
                fade_out_function: NativeEasingFunction::CubicBezier(NativeCubicBezier {
                    x1_per_mille: 0,
                    y1_per_mille: 0,
                    x2_per_mille: 580,
                    y2_per_mille: 1_000,
                }),
                target: NativeVisualBellTarget::CursorColor,
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_visual_bell_static_field_names() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local config = {}
            local fade_in_duration_field = 'fade_in_duration_ms'
            local fade_out_duration_field = 'fade_out_duration_ms'
            local fade_in_function_field = 'fade_in_function'
            local fade_out_function_field = 'fade_out_function'
            local target_field = 'target'
            local fade_in_ms = 25
            local fade_out_ms = 175
            local fade_in_easing = 'Linear'
            local fade_out_easing = 'EaseOut'
            local bell_target = 'CursorColor'

            config.visual_bell = {
              [fade_in_duration_field] = fade_in_ms,
              [fade_out_duration_field] = fade_out_ms,
              [fade_in_function_field] = fade_in_easing,
              [fade_out_function_field] = fade_out_easing,
              [target_field] = bell_target,
            }

            return config
            "##,
        )
        .expect("expected WezTerm visual bell static field-name config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().visual_bell,
            NativeVisualBell {
                fade_in_duration_ms: 25,
                fade_out_duration_ms: 175,
                fade_in_function: NativeEasingFunction::Linear,
                fade_out_function: NativeEasingFunction::EaseOut,
                target: NativeVisualBellTarget::CursorColor,
            }
        );
    }

    #[test]
    fn window_app_parses_static_key_visual_bell_field_assignments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local config = {}
            local bell_field = 'visual_bell'
            local fade_out_function = 'EaseOut'

            config[bell_field] = {}
            config[bell_field].fade_in_duration_ms = 75
            config[bell_field].fade_out_duration_ms = 125
            config[bell_field].fade_in_function = 'EaseIn'
            config[bell_field]['fade_out_function'] = fade_out_function
            config[bell_field].target = 'CursorColor'

            return config
            "##,
        )
        .expect("expected WezTerm static field-name visual_bell config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().visual_bell,
            NativeVisualBell {
                fade_in_duration_ms: 75,
                fade_out_duration_ms: 125,
                fade_in_function: NativeEasingFunction::EaseIn,
                fade_out_function: NativeEasingFunction::EaseOut,
                target: NativeVisualBellTarget::CursorColor,
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_visual_bell_table_long_bracket_key_query() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.visual_bell = {
              [[=[fade_in_duration_ms]=]] = 0,
              [[=[fade_out_duration_ms]=]] = 150,
              [[=[fade_in_function]=]] = 'EaseIn',
              [[=[fade_out_function]=]] = { [[=[CubicBezier]=]] = { 0.0, 0.0, 0.58, 1.0 } },
              [[=[target]=]] = 'BackgroundColor',
            }
            config.colors = {
              [[=[visual_bell]=]] = '#010203',
            }

            return config
            "##,
        )
        .expect("expected WezTerm visual bell config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.visual_bell,
            NativeVisualBell {
                fade_in_duration_ms: 0,
                fade_out_duration_ms: 150,
                fade_in_function: NativeEasingFunction::EaseIn,
                fade_out_function: NativeEasingFunction::CubicBezier(NativeCubicBezier {
                    x1_per_mille: 0,
                    y1_per_mille: 0,
                    x2_per_mille: 580,
                    y2_per_mille: 1_000,
                }),
                target: NativeVisualBellTarget::BackgroundColor,
            }
        );
        assert_eq!(effective.visual_bell_color, Some(Color::Rgb(1, 2, 3)));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_render_color_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.foreground_text_hsb = {
              hue = 1.0,
              saturation = 1.0,
              brightness = 0.5,
            }
            config.inactive_pane_hsb = {
              hue = 1.0,
              saturation = 0.8,
              brightness = 0.7,
            }
            config.bold_brightens_ansi_colors = 'BrightOnly'
            config.text_background_opacity = 0.4

            return config
            "#,
        )
        .expect("expected WezTerm render color config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.foreground_text_hsb,
            NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(1.0),
                brightness: NativeHsbMultiplier::from_f32(0.5),
            }
        );
        assert_eq!(
            effective.inactive_pane_hsb,
            NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(0.8),
                brightness: NativeHsbMultiplier::from_f32(0.7),
            }
        );
        assert_eq!(
            effective.bold_brightens_ansi_colors,
            NativeBoldBrightensAnsiColors::BrightOnly
        );
        assert_eq!(
            effective.text_background_opacity,
            NativeTextBackgroundOpacity::from_f32(0.4)
        );

        app.handle_pty_output(b"\x1b[38;2;100;150;200;48;2;20;40;60mA\x1b[0m")
            .unwrap();

        let snapshot = app.render_snapshot();
        let cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("visible cell");
        assert_eq!(cell.foreground, Color::Rgb(50, 75, 100));
        assert_eq!(cell.background, Color::Rgba(20, 40, 60, 102));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_hsb_static_variables() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local foreground_hsb = {
              hue = 1.0,
              saturation = 1.0,
              brightness = 0.5,
            }
            local inactive_hsb = {
              hue = 1.0,
              saturation = 0.8,
              brightness = 0.7,
            }

            config.term = 'xterm-256color'
            config.foreground_text_hsb = foreground_hsb
            config.inactive_pane_hsb = inactive_hsb

            return config
            "#,
        )
        .expect("expected WezTerm HSB static variable config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.foreground_text_hsb,
            NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(1.0),
                brightness: NativeHsbMultiplier::from_f32(0.5),
            }
        );
        assert_eq!(
            effective.inactive_pane_hsb,
            NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(0.8),
                brightness: NativeHsbMultiplier::from_f32(0.7),
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_hsb_static_field_variables() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local foreground_hue = 1.0
            local foreground_saturation = 0.9
            local foreground_brightness = 0.6
            local inactive_hue = 1.0
            local inactive_saturation = 0.7
            local inactive_brightness = 0.5

            config.foreground_text_hsb = {
              hue = foreground_hue,
              saturation = foreground_saturation,
              brightness = foreground_brightness,
            }
            config.inactive_pane_hsb = {
              hue = inactive_hue,
              saturation = inactive_saturation,
              brightness = inactive_brightness,
            }

            return config
            "#,
        )
        .expect("expected WezTerm HSB static field variable config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.foreground_text_hsb,
            NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(0.9),
                brightness: NativeHsbMultiplier::from_f32(0.6),
            }
        );
        assert_eq!(
            effective.inactive_pane_hsb,
            NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(0.7),
                brightness: NativeHsbMultiplier::from_f32(0.5),
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_hsb_static_field_names() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local hue_field = 'hue'
            local saturation_field = 'saturation'
            local brightness_field = 'brightness'
            local foreground_saturation = 0.9
            local inactive_brightness = 0.5

            config.foreground_text_hsb = {
              [hue_field] = 1.0,
              [saturation_field] = foreground_saturation,
              [brightness_field] = 0.6,
            }
            config.inactive_pane_hsb = {
              [hue_field] = 1.0,
              [saturation_field] = 0.7,
              [brightness_field] = inactive_brightness,
            }

            return config
            "#,
        )
        .expect("expected WezTerm HSB static field-name config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.foreground_text_hsb,
            NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(0.9),
                brightness: NativeHsbMultiplier::from_f32(0.6),
            }
        );
        assert_eq!(
            effective.inactive_pane_hsb,
            NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(0.7),
                brightness: NativeHsbMultiplier::from_f32(0.5),
            }
        );
    }

    #[test]
    fn window_app_parses_static_key_hsb_field_assignments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local foreground_field = 'foreground_text_hsb'
            local inactive_field = 'inactive_pane_hsb'
            local foreground_brightness = 0.6
            local inactive_brightness = 0.5

            config[foreground_field] = {}
            config[foreground_field].hue = 1.0
            config[foreground_field].saturation = 0.9
            config[foreground_field]['brightness'] = foreground_brightness

            config[inactive_field] = {}
            config[inactive_field].hue = 1.0
            config[inactive_field].saturation = 0.7
            config[inactive_field]['brightness'] = inactive_brightness

            return config
            "#,
        )
        .expect("expected WezTerm static field-name HSB config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.foreground_text_hsb,
            NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(0.9),
                brightness: NativeHsbMultiplier::from_f32(0.6),
            }
        );
        assert_eq!(
            effective.inactive_pane_hsb,
            NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(0.7),
                brightness: NativeHsbMultiplier::from_f32(0.5),
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_window_background_opacity() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_background_opacity = 0.5

            return config
            "#,
        )
        .expect("expected WezTerm window background opacity config");
        app.set_config_overrides(overrides);
        app.handle_pty_output(b"A").unwrap();

        let snapshot = app.render_snapshot();
        let cell = snapshot_cell(&snapshot, TAB_BAR_ROWS, 0).expect("visible cell");

        assert_eq!(cell.background, Color::Rgba(12, 12, 12, 127));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_background_color_layer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.background = {
              {
                source = { Color = '#0a141e' },
                opacity = 0.5,
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm background color layer config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.background,
            vec![super::NativeWindowBackgroundVisualLayer::Color(
                Color::Rgba(10, 20, 30, 127)
            )]
        );
        assert_eq!(effective.background_color, Color::Rgba(10, 20, 30, 127));
    }

    #[test]
    fn window_app_parses_wezterm_color_parse_static_alias_for_lua_background_color_layer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local parse_color = wezterm.color.parse

            config.background = {
              {
                source = { Color = parse_color('#0a141e') },
                opacity = 0.5,
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm color.parse background color layer config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.background,
            vec![super::NativeWindowBackgroundVisualLayer::Color(
                Color::Rgba(10, 20, 30, 127)
            )]
        );
        assert_eq!(effective.background_color, Color::Rgba(10, 20, 30, 127));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_background_color_layers() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.background = {
              {
                source = { Color = '#000000' },
              },
              {
                source = { Color = '#ffffff' },
                opacity = 0.5,
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm background color layers config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().background_color,
            Color::Rgb(127, 127, 127)
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_background_layer_source_static_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local bg_source = { Color = '#0a141e' }

            config.background = {
              {
                source = bg_source,
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm background layer static source config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().background_color,
            Color::Rgb(10, 20, 30)
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_background_layer_static_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local layer = {
              source = { Color = '#0a141e' },
              opacity = 0.5,
            }

            config.background = {
              layer,
            }

            return config
            "##,
        )
        .expect("expected WezTerm background static layer config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().background_color,
            Color::Rgba(10, 20, 30, 127)
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_background_color_layer_hsb() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.background = {
              {
                source = { Color = '#204060' },
                hsb = { brightness = 0.5 },
                opacity = 0.5,
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm background color layer hsb config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().background_color,
            Color::Rgba(16, 32, 48, 127)
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_background_layer_hsb_static_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local dimmer = { brightness = 0.5 }

            config.background = {
              {
                source = { Color = '#204060' },
                hsb = dimmer,
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm background layer static hsb config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().background_color,
            Color::Rgb(16, 32, 48)
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_background_gradient_layer() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.background = {
              {
                source = {
                  Gradient = {
                    orientation = 'Vertical',
                    colors = { '#010203', '#111213' },
                  },
                },
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm background gradient layer config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().window_background_gradient,
            Some(NativeWindowBackgroundGradient {
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
            })
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_background_gradient_layer_opacity() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.background = {
              {
                source = {
                  Gradient = {
                    orientation = 'Vertical',
                    colors = { '#010203', '#111213' },
                  },
                },
                opacity = 0.5,
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm background gradient layer opacity config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().window_background_gradient,
            Some(NativeWindowBackgroundGradient {
                orientation: NativeWindowBackgroundGradientOrientation::Vertical,
                interpolation: NativeWindowBackgroundGradientInterpolation::Linear,
                blend: NativeWindowBackgroundGradientBlend::Rgb,
                noise: None,
                segment: None,
                preset: None,
                opacity_alpha: u8::MAX,
                blend_with_background_color: false,
                hsb: super::native_identity_hsb(),
                colors: vec![Color::Rgba(1, 2, 3, 127), Color::Rgba(17, 18, 19, 127)],
            })
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_background_gradient_layer_hsb() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}

            config.background = {
              {
                source = {
                  Gradient = {
                    orientation = 'Vertical',
                    colors = { '#204060', '#406080' },
                  },
                },
                hsb = { brightness = 0.5 },
              },
            }

            return config
            "##,
        )
        .expect("expected WezTerm background gradient layer hsb config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().window_background_gradient,
            Some(NativeWindowBackgroundGradient {
                orientation: NativeWindowBackgroundGradientOrientation::Vertical,
                interpolation: NativeWindowBackgroundGradientInterpolation::Linear,
                blend: NativeWindowBackgroundGradientBlend::Rgb,
                noise: None,
                segment: None,
                preset: None,
                opacity_alpha: u8::MAX,
                blend_with_background_color: false,
                hsb: super::native_identity_hsb(),
                colors: vec![Color::Rgb(16, 32, 48), Color::Rgb(32, 48, 64)],
            })
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_window_decorations() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_decorations = 'NONE'

            return config
            "#,
        )
        .expect("expected WezTerm window decorations config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().window_decorations,
            NativeWindowDecorations {
                title: false,
                resize: false,
                integrated_buttons: false,
                macos_force_disable_shadow: false,
                macos_force_enable_shadow: false,
                macos_force_square_corners: false,
                macos_use_background_color_as_titlebar_color: false,
            }
        );
        assert!(
            !app.native_effective_config()
                .window_decorations
                .winit_decorations_enabled()
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_window_frame_colors() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local border_top_height = 4
            local frame_font_size = 13.5

            config.window_frame = {
              inactive_titlebar_bg = '#010203',
              active_titlebar_bg = '#040506',
              inactive_titlebar_fg = '#070809',
              active_titlebar_fg = '#0a0b0c',
              inactive_titlebar_border_bottom = '#0d0e0f',
              active_titlebar_border_bottom = '#101112',
              button_fg = '#131415',
              button_bg = '#161718',
              button_hover_fg = '#191a1b',
              button_hover_bg = '#1c1d1e',
              border_left_width = '0.5cell',
              border_right_width = 2.5,
              border_top_height = border_top_height,
              border_bottom_height = '1.5cell',
              border_left_color = '#1f2021',
              border_right_color = '#222324',
              border_top_color = '#252627',
              border_bottom_color = '#28292a',
              font = wezterm.font 'Roboto',
              font_size = frame_font_size,
            }

            return config
            "#,
        )
        .expect("expected WezTerm window_frame color config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(&effective.window_frame, &effective.window_frame_appearance);
        let effective = effective.window_frame_appearance.clone();
        assert_eq!(effective.inactive_titlebar_bg, Some(Color::Rgb(1, 2, 3)));
        assert_eq!(effective.active_titlebar_bg, Some(Color::Rgb(4, 5, 6)));
        assert_eq!(effective.inactive_titlebar_fg, Some(Color::Rgb(7, 8, 9)));
        assert_eq!(effective.active_titlebar_fg, Some(Color::Rgb(10, 11, 12)));
        assert_eq!(
            effective.inactive_titlebar_border_bottom,
            Some(Color::Rgb(13, 14, 15))
        );
        assert_eq!(
            effective.active_titlebar_border_bottom,
            Some(Color::Rgb(16, 17, 18))
        );
        assert_eq!(effective.button_fg, Some(Color::Rgb(19, 20, 21)));
        assert_eq!(effective.button_bg, Some(Color::Rgb(22, 23, 24)));
        assert_eq!(effective.button_hover_fg, Some(Color::Rgb(25, 26, 27)));
        assert_eq!(effective.button_hover_bg, Some(Color::Rgb(28, 29, 30)));
        assert_eq!(
            effective.border_left_width,
            Some(NativeWindowPaddingDimension::CellFractionPerMille(500))
        );
        assert_eq!(
            effective.border_right_width,
            Some(NativeWindowPaddingDimension::Pixels(3))
        );
        assert_eq!(
            effective.border_top_height,
            Some(NativeWindowPaddingDimension::Pixels(4))
        );
        assert_eq!(
            effective.border_bottom_height,
            Some(NativeWindowPaddingDimension::CellFractionPerMille(1500))
        );
        assert_eq!(effective.border_left_color, Some(Color::Rgb(31, 32, 33)));
        assert_eq!(effective.border_right_color, Some(Color::Rgb(34, 35, 36)));
        assert_eq!(effective.border_top_color, Some(Color::Rgb(37, 38, 39)));
        assert_eq!(effective.border_bottom_color, Some(Color::Rgb(40, 41, 42)));
        assert_eq!(effective.font, Some("Roboto".to_owned()));
        assert_eq!(
            effective.font_size,
            Some(NativeFontSize::from_millipoints(13_500))
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_window_frame_static_field_names() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local bg_field = 'inactive_titlebar_bg'
            local width_field = 'border_left_width'
            local font_field = 'font'
            local font_size_field = 'font_size'
            local frame_font_size = 13.5

            config.window_frame = {
              [bg_field] = '#010203',
              [width_field] = '0.5cell',
              [font_field] = wezterm.font 'Roboto',
              [font_size_field] = frame_font_size,
            }

            return config
            "#,
        )
        .expect("expected WezTerm window_frame static field-name config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config().window_frame_appearance.clone();
        assert_eq!(effective.inactive_titlebar_bg, Some(Color::Rgb(1, 2, 3)));
        assert_eq!(
            effective.border_left_width,
            Some(NativeWindowPaddingDimension::CellFractionPerMille(500))
        );
        assert_eq!(effective.font, Some("Roboto".to_owned()));
        assert_eq!(
            effective.font_size,
            Some(NativeFontSize::from_millipoints(13_500))
        );
    }

    #[test]
    fn window_app_parses_wezterm_color_parse_static_alias_for_lua_window_frame_colors() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local parse_color = wezterm.color.parse

            config.window_frame = {
              inactive_titlebar_bg = parse_color('rgba(1,2,3,0.5)'),
              border_left_color = parse_color('#040506'),
            }

            return config
            "##,
        )
        .expect("expected WezTerm color.parse window_frame color config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config().window_frame_appearance.clone();
        assert_eq!(effective.inactive_titlebar_bg, Some(Color::Rgb(1, 2, 3)));
        assert_eq!(effective.border_left_color, Some(Color::Rgb(4, 5, 6)));
    }

    #[test]
    fn window_app_parses_wezterm_font_static_alias_for_lua_window_frame_font() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local frame_font = wezterm.font

            config.window_frame = {
              font = frame_font 'Roboto Mono',
            }

            return config
            "#,
        )
        .expect("expected WezTerm font static alias window_frame font config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().window_frame_appearance.font,
            Some("Roboto Mono".to_owned())
        );
    }

    #[test]
    fn window_app_parses_wezterm_font_static_alias_comment_for_lua_window_frame_font() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local frame_font = wezterm.font

            config.window_frame = {
              font = frame_font -- titlebar
                'Roboto Mono',
            }

            return config
            "#,
        )
        .expect("expected WezTerm font static alias comment window_frame font config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().window_frame_appearance.font,
            Some("Roboto Mono".to_owned())
        );
    }

    #[test]
    fn window_app_parses_wezterm_font_table_static_family_for_lua_window_frame_font() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local frame_family = 'Roboto Mono'

            config.window_frame = {
              font = wezterm.font {
                family = frame_family,
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm font table static family window_frame font config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().window_frame_appearance.font,
            Some("Roboto Mono".to_owned())
        );
    }

    #[test]
    fn window_app_parses_wezterm_font_static_value_for_lua_window_frame_font() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local frame_font = wezterm.font {
              family = 'Roboto Mono',
            }

            config.window_frame = {
              font = frame_font,
            }

            return config
            "#,
        )
        .expect("expected WezTerm static font value window_frame font config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().window_frame_appearance.font,
            Some("Roboto Mono".to_owned())
        );
    }

    #[test]
    fn window_app_reports_default_wezterm_window_frame_font() {
        let effective = NativeWindowApp::new(None)
            .native_effective_config()
            .window_frame_appearance.clone();
        let expected_font_size = if cfg!(target_os = "windows") {
            NativeFontSize::from_millipoints(10_000)
        } else {
            NativeFontSize::from_millipoints(12_000)
        };

        let expected_font = if cfg!(target_os = "windows") {
            "Cascadia Mono"
        } else if cfg!(target_os = "macos") {
            "Menlo"
        } else if cfg!(target_os = "linux") {
            "Noto Sans Mono"
        } else {
            "Cascadia Mono"
        };
        assert_eq!(effective.font, Some(expected_font.to_owned()));
        assert_eq!(effective.font_size, Some(expected_font_size));
    }

    #[test]
    fn window_app_reports_default_wezterm_integrated_title_button_config() {
        let effective = NativeWindowApp::new(None).native_effective_config();

        assert_eq!(
            effective.integrated_title_buttons,
            vec![
                NativeIntegratedTitleButton::Hide,
                NativeIntegratedTitleButton::Maximize,
                NativeIntegratedTitleButton::Close,
            ]
        );
        assert_eq!(
            effective.integrated_title_button_alignment,
            NativeIntegratedTitleButtonAlignment::Right
        );
        assert_eq!(
            effective.integrated_title_button_color,
            NativeIntegratedTitleButtonColor::Auto
        );
        #[cfg(target_os = "macos")]
        assert_eq!(
            effective.integrated_title_button_style,
            NativeIntegratedTitleButtonStyle::MacOsNative
        );
        #[cfg(not(target_os = "macos"))]
        assert_eq!(
            effective.integrated_title_button_style,
            NativeIntegratedTitleButtonStyle::Windows
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_integrated_title_button_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.integrated_title_buttons = { 'Close', 'Hide' }
            config.integrated_title_button_alignment = 'Left'
            config.integrated_title_button_color = '#010203'
            config.integrated_title_button_style = 'Gnome'

            return config
            "#,
        )
        .expect("expected WezTerm integrated title button config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
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
    }

    #[test]
    fn window_app_parses_wezterm_color_parse_static_alias_for_lua_integrated_title_button_color() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r##"
            local wezterm = require 'wezterm'
            local config = {}
            local parse_color = wezterm.color.parse

            config.integrated_title_button_color = parse_color('rgba(4,5,6,0.5)')

            return config
            "##,
        )
        .expect("expected WezTerm color.parse integrated title button color config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().integrated_title_button_color,
            NativeIntegratedTitleButtonColor::Color(Color::Rgb(4, 5, 6))
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_hsb_table_long_bracket_key_query() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.foreground_text_hsb = {
              [[=[hue]=]] = 1.0,
              [[=[saturation]=]] = 1.0,
              [[=[brightness]=]] = 0.5,
            }
            config.inactive_pane_hsb = {
              [[=[hue]=]] = 1.0,
              [[=[saturation]=]] = 0.8,
              [[=[brightness]=]] = 0.7,
            }

            return config
            "#,
        )
        .expect("expected WezTerm HSB config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.foreground_text_hsb,
            NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(1.0),
                brightness: NativeHsbMultiplier::from_f32(0.5),
            }
        );
        assert_eq!(
            effective.inactive_pane_hsb,
            NativeInactivePaneHsb {
                hue: NativeHsbMultiplier::from_f32(1.0),
                saturation: NativeHsbMultiplier::from_f32(0.8),
                brightness: NativeHsbMultiplier::from_f32(0.7),
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_text_blink_and_decoration_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.text_blink_rate = 600
            config.text_blink_rate_rapid = 150
            config.text_blink_ease_in = 'EaseIn'
            config.text_blink_ease_out = 'EaseOut'
            config.text_blink_rapid_ease_in = 'EaseInOut'
            config.text_blink_rapid_ease_out = 'Constant'
            config.cursor_thickness = '25%'
            config.underline_thickness = '2px'
            config.underline_position = '-2px'
            config.strikethrough_position = '0.5cell'

            return config
            "#,
        )
        .expect("expected WezTerm text blink/decoration config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
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
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_cubic_bezier_easing_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.cursor_blink_ease_in = { CubicBezier = { 0.1, 0.2, 0.3, 0.4 } }
            config.cursor_blink_ease_out = { CubicBezier = { 0.2, 0.3, 0.4, 0.5 } }
            config.text_blink_ease_in = { CubicBezier = { 0.3, 0.4, 0.5, 0.6 } }
            config.text_blink_ease_out = { CubicBezier = { 0.4, 0.5, 0.6, 0.7 } }
            config.text_blink_rapid_ease_in = { CubicBezier = { 0.5, 0.6, 0.7, 0.8 } }
            config.text_blink_rapid_ease_out = { CubicBezier = { 0.6, 0.7, 0.8, 0.9 } }
            config.visual_bell = {
              fade_out_duration_ms = 100,
              fade_out_function = { CubicBezier = { 0.0, 0.0, 0.58, 1.0 } },
            }

            return config
            "#,
        )
        .expect("expected WezTerm CubicBezier easing config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.cursor_blink_ease_in,
            NativeEasingFunction::CubicBezier(NativeCubicBezier {
                x1_per_mille: 100,
                y1_per_mille: 200,
                x2_per_mille: 300,
                y2_per_mille: 400,
            })
        );
        assert_eq!(
            effective.cursor_blink_ease_out,
            NativeEasingFunction::CubicBezier(NativeCubicBezier {
                x1_per_mille: 200,
                y1_per_mille: 300,
                x2_per_mille: 400,
                y2_per_mille: 500,
            })
        );
        assert_eq!(
            effective.text_blink_ease_in,
            NativeEasingFunction::CubicBezier(NativeCubicBezier {
                x1_per_mille: 300,
                y1_per_mille: 400,
                x2_per_mille: 500,
                y2_per_mille: 600,
            })
        );
        assert_eq!(
            effective.text_blink_ease_out,
            NativeEasingFunction::CubicBezier(NativeCubicBezier {
                x1_per_mille: 400,
                y1_per_mille: 500,
                x2_per_mille: 600,
                y2_per_mille: 700,
            })
        );
        assert_eq!(
            effective.text_blink_rapid_ease_in,
            NativeEasingFunction::CubicBezier(NativeCubicBezier {
                x1_per_mille: 500,
                y1_per_mille: 600,
                x2_per_mille: 700,
                y2_per_mille: 800,
            })
        );
        assert_eq!(
            effective.text_blink_rapid_ease_out,
            NativeEasingFunction::CubicBezier(NativeCubicBezier {
                x1_per_mille: 600,
                y1_per_mille: 700,
                x2_per_mille: 800,
                y2_per_mille: 900,
            })
        );
        assert_eq!(
            effective.visual_bell.fade_out_function,
            NativeEasingFunction::CubicBezier(NativeCubicBezier {
                x1_per_mille: 0,
                y1_per_mille: 0,
                x2_per_mille: 580,
                y2_per_mille: 1_000,
            })
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_cubic_bezier_easing_trailing_comma() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.cursor_blink_ease_in = { CubicBezier = { 0.1, 0.2, 0.3, 0.4 }, }

            return config
            "#,
        )
        .expect("expected WezTerm CubicBezier easing config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().cursor_blink_ease_in,
            NativeEasingFunction::CubicBezier(NativeCubicBezier {
                x1_per_mille: 100,
                y1_per_mille: 200,
                x2_per_mille: 300,
                y2_per_mille: 400,
            })
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_easing_static_variables() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local cursor_enter = 'EaseIn'
            local cursor_exit = { CubicBezier = { 0.2, 0.3, 0.4, 0.5 } }
            local text_enter = 'Linear'
            local text_exit = { CubicBezier = { 0.4, 0.5, 0.6, 0.7 } }

            config.cursor_blink_ease_in = cursor_enter
            config.cursor_blink_ease_out = cursor_exit
            config.text_blink_ease_in = text_enter
            config.text_blink_ease_out = text_exit

            return config
            "#,
        )
        .expect("expected WezTerm easing static variable config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.cursor_blink_ease_in, NativeEasingFunction::EaseIn);
        assert_eq!(
            effective.cursor_blink_ease_out,
            NativeEasingFunction::CubicBezier(NativeCubicBezier {
                x1_per_mille: 200,
                y1_per_mille: 300,
                x2_per_mille: 400,
                y2_per_mille: 500,
            })
        );
        assert_eq!(effective.text_blink_ease_in, NativeEasingFunction::Linear);
        assert_eq!(
            effective.text_blink_ease_out,
            NativeEasingFunction::CubicBezier(NativeCubicBezier {
                x1_per_mille: 400,
                y1_per_mille: 500,
                x2_per_mille: 600,
                y2_per_mille: 700,
            })
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_numeric_decoration_dimensions() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.cursor_thickness = 2.0
            config.underline_thickness = 3
            config.underline_position = -4
            config.strikethrough_position = 5.0

            return config
            "#,
        )
        .expect("expected WezTerm numeric decoration dimensions");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.cursor_thickness,
            Some(NativeCursorThickness::Pixels(2))
        );
        assert_eq!(
            effective.underline_thickness,
            Some(NativeUnderlineThickness::Pixels(3))
        );
        assert_eq!(
            effective.underline_position,
            Some(NativeUnderlinePosition::Pixels(-4))
        );
        assert_eq!(
            effective.strikethrough_position,
            Some(NativeStrikethroughPosition::Pixels(5))
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_numeric_decoration_dimension_static_variables() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local cursor_width = 2.0
            local underline_width = 3
            local underline_offset = -4
            local strike_offset = 5.0

            config.cursor_thickness = cursor_width
            config.underline_thickness = underline_width
            config.underline_position = underline_offset
            config.strikethrough_position = strike_offset

            return config
            "#,
        )
        .expect("expected WezTerm numeric decoration dimension static variables");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.cursor_thickness,
            Some(NativeCursorThickness::Pixels(2))
        );
        assert_eq!(
            effective.underline_thickness,
            Some(NativeUnderlineThickness::Pixels(3))
        );
        assert_eq!(
            effective.underline_position,
            Some(NativeUnderlinePosition::Pixels(-4))
        );
        assert_eq!(
            effective.strikethrough_position,
            Some(NativeStrikethroughPosition::Pixels(5))
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_close_confirmation_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_close_confirmation = 'NeverPrompt'
            config.skip_close_confirmation_for_processes_named = { 'top', 'cmd.exe' }

            return config
            "#,
        )
        .expect("expected WezTerm close confirmation config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.window_close_confirmation,
            NativeWindowCloseConfirmation::NeverPrompt
        );
        assert_eq!(
            effective.skip_close_confirmation_for_processes_named,
            ["top".to_owned(), "cmd.exe".to_owned()]
        );

        app.handle_window_close_requested();

        assert!(app.window_close_requested_for_test());
        assert!(app.close_confirmation.is_none());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_skip_close_confirmation_static_variable() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("top"));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local stateless_processes = { 'top', 'cmd.exe' }

            config.window_close_confirmation = 'AlwaysPrompt'
            config.skip_close_confirmation_for_processes_named = stateless_processes

            return config
            "#,
        )
        .expect("expected WezTerm skip close confirmation table variable config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.skip_close_confirmation_for_processes_named,
            ["top".to_owned(), "cmd.exe".to_owned()]
        );

        app.handle_window_close_requested();

        assert!(app.window_close_requested_for_test());
        assert!(app.close_confirmation.is_none());
    }

    #[test]
    fn window_app_parses_static_return_table_skip_close_confirmation_key() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("top"));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local skip_field = 'skip_close_confirmation_for_processes_named'

            return {
              window_close_confirmation = 'AlwaysPrompt',
              [skip_field] = { 'top', 'cmd.exe' },
            }
            "#,
        )
        .expect("expected WezTerm static field-name return table skip close config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.skip_close_confirmation_for_processes_named,
            ["top".to_owned(), "cmd.exe".to_owned()]
        );

        app.handle_window_close_requested();

        assert!(app.window_close_requested_for_test());
        assert!(app.close_confirmation.is_none());
    }

    #[test]
    fn window_app_parses_static_initializer_skip_close_confirmation_key() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("top"));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local skip_field = 'skip_close_confirmation_for_processes_named'
            local config = {
              window_close_confirmation = 'AlwaysPrompt',
              [skip_field] = { 'top', 'cmd.exe' },
            }

            return config
            "#,
        )
        .expect("expected WezTerm static field-name initializer skip close config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.skip_close_confirmation_for_processes_named,
            ["top".to_owned(), "cmd.exe".to_owned()]
        );

        app.handle_window_close_requested();

        assert!(app.window_close_requested_for_test());
        assert!(app.close_confirmation.is_none());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_skip_close_confirmation_table_insert() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("top"));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_close_confirmation = 'AlwaysPrompt'
            config.skip_close_confirmation_for_processes_named = {}
            table.insert(config.skip_close_confirmation_for_processes_named, 'top')
            table.insert(config.skip_close_confirmation_for_processes_named, 'cmd.exe')

            return config
            "#,
        )
        .expect("expected WezTerm skip close confirmation table insert config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.skip_close_confirmation_for_processes_named,
            ["top".to_owned(), "cmd.exe".to_owned()]
        );

        app.handle_window_close_requested();

        assert!(app.window_close_requested_for_test());
        assert!(app.close_confirmation.is_none());
    }

    #[test]
    fn window_app_parses_static_key_skip_close_confirmation_table_insert() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("top"));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local skip_field = 'skip_close_confirmation_for_processes_named'
            local config = {}

            config.window_close_confirmation = 'AlwaysPrompt'
            config[skip_field] = {}
            table.insert(config[skip_field], 'top')
            table.insert(config[skip_field], 'cmd.exe')

            return config
            "#,
        )
        .expect("expected WezTerm static field-name skip close insert config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.skip_close_confirmation_for_processes_named,
            ["top".to_owned(), "cmd.exe".to_owned()]
        );

        app.handle_window_close_requested();

        assert!(app.window_close_requested_for_test());
        assert!(app.close_confirmation.is_none());
    }

    #[test]
    fn window_app_parses_static_key_skip_close_confirmation_length_append() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("top"));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local skip_field = 'skip_close_confirmation_for_processes_named'
            local config = {}

            config.window_close_confirmation = 'AlwaysPrompt'
            config[skip_field] = {}
            config[skip_field][#config[skip_field] + 1] = 'top'
            config[skip_field][#config[skip_field] + 1] = 'cmd.exe'

            return config
            "#,
        )
        .expect("expected WezTerm static field-name skip close length append config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.skip_close_confirmation_for_processes_named,
            ["top".to_owned(), "cmd.exe".to_owned()]
        );

        app.handle_window_close_requested();

        assert!(app.window_close_requested_for_test());
        assert!(app.close_confirmation.is_none());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_default_workspace_and_domain() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.default_workspace = 'ops'
            config.default_domain = 'local'
            config.prefer_to_spawn_tabs = true

            return config
            "#,
        )
        .expect("expected WezTerm workspace/domain spawn preference config");
        app.set_config_overrides(overrides);

        assert_eq!(app.app_shell.active_workspace().name(), "ops");
        let effective = app.native_effective_config();
        assert_eq!(effective.default_workspace, "ops");
        assert_eq!(effective.default_domain, "local");
        assert!(effective.prefer_to_spawn_tabs);

        assert!(app.command_palette_execute(WindowCommand::SpawnTab(
            WindowSpawnTabDomain::DefaultDomain,
        )));
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_exit_behavior() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("tool"));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.exit_behavior = 'CloseOnCleanExit'
            config.clean_exit_codes = { 130, [2] = 143 }
            config.exit_behavior_messaging = 'Brief'

            return config
            "#,
        )
        .expect("expected WezTerm exit behavior config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(
            effective.exit_behavior,
            NativeExitBehavior::CloseOnCleanExit
        );
        assert_eq!(effective.clean_exit_codes, [130, 143]);
        assert_eq!(
            effective.exit_behavior_messaging,
            NativeExitBehaviorMessaging::Brief
        );

        let close_window = app.apply_pane_exit_behavior(
            rterm_core::PaneId::new(1),
            &PtyExitStatus::from_exit_code(143),
        );
        assert!(close_window);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_clean_exit_codes_static_variable() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("tool"));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local clean_codes = { 130, [2] = 143 }

            config.exit_behavior = 'CloseOnCleanExit'
            config.clean_exit_codes = clean_codes

            return config
            "#,
        )
        .expect("expected WezTerm clean exit codes table variable config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.clean_exit_codes, [130, 143]);

        let close_window = app.apply_pane_exit_behavior(
            rterm_core::PaneId::new(1),
            &PtyExitStatus::from_exit_code(143),
        );
        assert!(close_window);
    }

    #[test]
    fn window_app_parses_static_return_table_clean_exit_codes_key() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("tool"));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local codes_field = 'clean_exit_codes'

            return {
              exit_behavior = 'CloseOnCleanExit',
              [codes_field] = { 130, [2] = 143 },
            }
            "#,
        )
        .expect("expected WezTerm static field-name return table clean exit codes config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.clean_exit_codes, [130, 143]);

        let close_window = app.apply_pane_exit_behavior(
            rterm_core::PaneId::new(1),
            &PtyExitStatus::from_exit_code(143),
        );
        assert!(close_window);
    }

    #[test]
    fn window_app_parses_static_initializer_clean_exit_codes_key() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("tool"));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local codes_field = 'clean_exit_codes'
            local config = {
              exit_behavior = 'CloseOnCleanExit',
              [codes_field] = { 130, [2] = 143 },
            }

            return config
            "#,
        )
        .expect("expected WezTerm static field-name initializer clean exit codes config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.clean_exit_codes, [130, 143]);

        let close_window = app.apply_pane_exit_behavior(
            rterm_core::PaneId::new(1),
            &PtyExitStatus::from_exit_code(143),
        );
        assert!(close_window);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_clean_exit_codes_table_insert() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("tool"));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.exit_behavior = 'CloseOnCleanExit'
            config.clean_exit_codes = {}
            table.insert(config.clean_exit_codes, 130)
            table.insert(config.clean_exit_codes, 143)

            return config
            "#,
        )
        .expect("expected WezTerm clean exit codes table insert config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.clean_exit_codes, [130, 143]);

        let close_window = app.apply_pane_exit_behavior(
            rterm_core::PaneId::new(1),
            &PtyExitStatus::from_exit_code(143),
        );
        assert!(close_window);
    }

    #[test]
    fn window_app_parses_static_key_clean_exit_codes_table_insert() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("tool"));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local codes_field = 'clean_exit_codes'
            local config = {}

            config.exit_behavior = 'CloseOnCleanExit'
            config[codes_field] = {}
            table.insert(config[codes_field], 130)
            table.insert(config[codes_field], 143)

            return config
            "#,
        )
        .expect("expected WezTerm static field-name clean exit codes insert config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.clean_exit_codes, [130, 143]);

        let close_window = app.apply_pane_exit_behavior(
            rterm_core::PaneId::new(1),
            &PtyExitStatus::from_exit_code(143),
        );
        assert!(close_window);
    }

    #[test]
    fn window_app_parses_static_key_clean_exit_codes_length_append() {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("tool"));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local codes_field = 'clean_exit_codes'
            local config = {}

            config.exit_behavior = 'CloseOnCleanExit'
            config[codes_field] = {}
            config[codes_field][#config[codes_field] + 1] = 130
            config[codes_field][#config[codes_field] + 1] = 143

            return config
            "#,
        )
        .expect("expected WezTerm static field-name clean exit codes length append config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.clean_exit_codes, [130, 143]);

        let close_window = app.apply_pane_exit_behavior(
            rterm_core::PaneId::new(1),
            &PtyExitStatus::from_exit_code(143),
        );
        assert!(close_window);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_clean_exit_codes_static_variable_post_assignment_inserts()
     {
        let mut app = NativeWindowApp::new_with_command(None, rterm_pty::PtyCommand::new("tool"));
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local clean_codes = {}

            config.exit_behavior = 'CloseOnCleanExit'
            config.clean_exit_codes = clean_codes
            table.insert(clean_codes, 130)
            table.insert(clean_codes, 143)

            return config
            "#,
        )
        .expect("expected WezTerm clean exit codes post-assignment insert config");
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        assert_eq!(effective.clean_exit_codes, [130, 143]);

        let close_window = app.apply_pane_exit_behavior(
            rterm_core::PaneId::new(1),
            &PtyExitStatus::from_exit_code(143),
        );
        assert!(close_window);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_scrollback_overrides() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.scroll_to_bottom_on_input = false
            config.alternate_buffer_wheel_scroll_speed = 2
            config.scrollback_lines = 1
            config.enable_scroll_bar = true
            config.min_scroll_bar_height = '2cell'

            return config
            "#,
        )
        .expect("expected WezTerm scrollback config");
        app.set_config_overrides(overrides);
        app.runtime.resize(rterm_core::TerminalSize::new(4, 2));
        app.handle_pty_output(b"aa\r\nbb\r\ncc\r\ndd\r\nee")
            .unwrap();

        let effective = app.native_effective_config();
        assert!(!effective.scroll_to_bottom_on_input);
        assert_eq!(effective.alternate_buffer_wheel_scroll_speed, 2);
        assert_eq!(effective.scrollback_lines, 1);
        assert!(effective.enable_scroll_bar);
        assert_eq!(
            effective.min_scroll_bar_height,
            Some(NativeScrollBarHeight::CellFractionPerMille(2_000))
        );
        assert_eq!(app.runtime.terminal().scrollback().len(), 1);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_window_padding() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_padding = {
              left = 8,
              right = 16,
              top = 16,
              bottom = 32,
            }

            return config
            "#,
        )
        .expect("expected WezTerm window padding config");
        app.runtime.resize(rterm_core::TerminalSize::new(20, 6));
        app.refresh_snapshot();
        app.set_config_overrides(overrides);

        let effective = app.native_effective_config();
        let layout = app.pane_render_layout();
        let rect = layout.panes.first().expect("pane rect");

        assert_eq!(
            effective.window_padding,
            NativeWindowPadding {
                left: NativeWindowPaddingDimension::Pixels(8),
                right: NativeWindowPaddingDimension::Pixels(16),
                top: NativeWindowPaddingDimension::Pixels(16),
                bottom: NativeWindowPaddingDimension::Pixels(32),
            }
        );
        assert_eq!(rect.row, TAB_BAR_ROWS);
        assert_eq!(rect.column, 0);
        assert_eq!(rect.rows, 6);
        assert_eq!(rect.columns, 20);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_window_padding_static_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local project_padding = {
              left = 8,
              right = 16,
              top = 16,
              bottom = 32,
            }

            config.term = 'xterm-256color'
            config.window_padding = project_padding

            return config
            "#,
        )
        .expect("expected WezTerm window padding static variable config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().window_padding,
            NativeWindowPadding {
                left: NativeWindowPaddingDimension::Pixels(8),
                right: NativeWindowPaddingDimension::Pixels(16),
                top: NativeWindowPaddingDimension::Pixels(16),
                bottom: NativeWindowPaddingDimension::Pixels(32),
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_window_padding_static_field_variables() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local left_padding = 8
            local right_padding = 16
            local top_padding = '1cell'
            local bottom_padding = '2pt'

            config.window_padding = {
              left = left_padding,
              right = right_padding,
              top = top_padding,
              bottom = bottom_padding,
            }

            return config
            "#,
        )
        .expect("expected WezTerm window padding static field variable config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().window_padding,
            NativeWindowPadding {
                left: NativeWindowPaddingDimension::Pixels(8),
                right: NativeWindowPaddingDimension::Pixels(16),
                top: NativeWindowPaddingDimension::CellFractionPerMille(1_000),
                bottom: NativeWindowPaddingDimension::Points(2),
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_window_padding_static_field_names() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local left_field = 'left'
            local right_field = 'right'
            local top_field = 'top'
            local bottom_field = 'bottom'

            config.window_padding = {
              [left_field] = 8,
              [right_field] = 16,
              [top_field] = '1cell',
              [bottom_field] = '2pt',
            }

            return config
            "#,
        )
        .expect("expected WezTerm window padding static field-name config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().window_padding,
            NativeWindowPadding {
                left: NativeWindowPaddingDimension::Pixels(8),
                right: NativeWindowPaddingDimension::Pixels(16),
                top: NativeWindowPaddingDimension::CellFractionPerMille(1_000),
                bottom: NativeWindowPaddingDimension::Points(2),
            }
        );
    }

    #[test]
    fn window_app_parses_static_key_window_padding_field_assignments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local padding_field = 'window_padding'

            config[padding_field] = {}
            config[padding_field].left = 8
            config[padding_field].right = 16
            config[padding_field].top = '1cell'
            config[padding_field].bottom = '2pt'

            return config
            "#,
        )
        .expect("expected WezTerm static field-name window padding field config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().window_padding,
            NativeWindowPadding {
                left: NativeWindowPaddingDimension::Pixels(8),
                right: NativeWindowPaddingDimension::Pixels(16),
                top: NativeWindowPaddingDimension::CellFractionPerMille(1_000),
                bottom: NativeWindowPaddingDimension::Points(2),
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_window_padding_static_field_name_mutations() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local config = {}
            local left_field = 'left'
            local right_field = 'right'
            local top_field = 'top'
            local bottom_field = 'bottom'

            config.window_padding = {}
            config.window_padding[left_field] = 8
            config.window_padding[right_field] = 16
            config.window_padding[top_field] = '1cell'
            config.window_padding[bottom_field] = '2pt'

            return config
            "#,
        )
        .expect("expected WezTerm window padding static field-name mutation config");
        app.set_config_overrides(overrides);

        assert_eq!(
            app.native_effective_config().window_padding,
            NativeWindowPadding {
                left: NativeWindowPaddingDimension::Pixels(8),
                right: NativeWindowPaddingDimension::Pixels(16),
                top: NativeWindowPaddingDimension::CellFractionPerMille(1_000),
                bottom: NativeWindowPaddingDimension::Points(2),
            }
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_window_padding_cell_units() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.window_padding = {
              left = '1cell',
              right = '2cell',
              top = '1cell',
              bottom = '2cell',
            }

            return config
            "#,
        )
        .expect("expected WezTerm window padding config with cell units");
        app.runtime.resize(rterm_core::TerminalSize::new(20, 6));
        app.refresh_snapshot();
        app.set_config_overrides(overrides);

        let layout = app.pane_render_layout();
        let rect = layout.panes.first().expect("pane rect");

        assert_eq!(rect.row, TAB_BAR_ROWS);
        assert_eq!(rect.column, 0);
        assert_eq!(rect.rows, 6);
        assert_eq!(rect.columns, 20);
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_launch_menu_into_launcher_entries() {
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
                set_environment_variables = {
                  LAUNCH_MENU = '1',
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch_menu config");
        app.set_config_overrides(overrides);

        let effective_launch_menu = app.native_effective_config().launch_menu.clone();
        assert_eq!(effective_launch_menu.len(), 1);
        assert_eq!(
            effective_launch_menu[0].label.as_deref(),
            Some("System Monitor")
        );

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::launch_menu_items(),
                title: Some("Pick Launch".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));

        app.command_palette_set_query("monitor".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "System Monitor");
        assert!(app.command_palette_execute_entry(entries[0].clone()));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-H"]);
        assert_eq!(launch.cwd(), Some("/tmp/project"));
        assert_eq!(
            launch.environment().get("LAUNCH_MENU"),
            Some(&"1".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_launch_menu_static_field_variables() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local menu_label = 'System Monitor'
            local program = 'top'
            local flag = '-H'
            local project_cwd = '/tmp/project'
            local launch_menu_enabled = '1'

            config.launch_menu = {
              {
                label = menu_label,
                args = { program, flag },
                cwd = project_cwd,
                set_environment_variables = {
                  LAUNCH_MENU = launch_menu_enabled,
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch_menu static field variable config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::launch_menu_items(),
                title: Some("Pick Launch".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));

        app.command_palette_set_query("monitor".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "System Monitor");
        assert!(app.command_palette_execute_entry(entries[0].clone()));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-H"]);
        assert_eq!(launch.cwd(), Some("/tmp/project"));
        assert_eq!(
            launch.environment().get("LAUNCH_MENU"),
            Some(&"1".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_launch_menu_static_label_field_name_variable() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local label_field = 'label'
            local args_field = 'args'
            local cwd_field = 'cwd'

            config.launch_menu = {
              {
                [label_field] = 'Static Label Monitor',
                [args_field] = { 'top', '-H' },
                [cwd_field] = '/tmp/static-label',
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch_menu static label field-name config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::launch_menu_items(),
                title: Some("Pick Launch".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));

        app.command_palette_set_query("static label".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Static Label Monitor");
        assert!(app.command_palette_execute_entry(entries[0].clone()));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-H"]);
        assert_eq!(launch.cwd(), Some("/tmp/static-label"));
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_launch_menu_static_field_variable_item() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            local monitor = {}
            monitor.label = 'Field Variable Monitor'
            monitor.args = { 'top', '-H' }
            monitor.cwd = '/tmp/field-variable'

            config.launch_menu = { monitor }

            return config
            "#,
        )
        .expect("expected WezTerm launch_menu field-built item variable config");

        let launch_menu = overrides
            .launch_menu.clone().expect("expected launch_menu overrides");
        assert_eq!(launch_menu.len(), 1);
        assert_eq!(
            launch_menu[0].label.as_deref(),
            Some("Field Variable Monitor")
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_launch_menu_table_insert_entries() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.launch_menu = {}
            table.insert(config.launch_menu, {
              label = 'Inserted Monitor',
              args = { 'top', '-H' },
              cwd = '/tmp/inserted',
            })

            return config
            "#,
        )
        .expect("expected WezTerm launch_menu table.insert config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::launch_menu_items(),
                title: None,
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));

        app.command_palette_set_query("inserted".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Inserted Monitor");
        assert!(app.command_palette_execute_entry(entries[0].clone()));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-H"]);
        assert_eq!(launch.cwd(), Some("/tmp/inserted"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_launch_menu_table_insert_static_variable_entries() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            local monitor = {
              label = 'Variable Monitor',
              args = { 'top' },
            }

            config.launch_menu = {}
            table.insert(config.launch_menu, monitor)

            return config
            "#,
        )
        .expect("expected WezTerm launch_menu table.insert variable config");

        let launch_menu = overrides
            .launch_menu.clone().expect("expected launch_menu overrides");
        assert_eq!(launch_menu.len(), 1);
        assert_eq!(launch_menu[0].label.as_deref(), Some("Variable Monitor"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_launch_menu_static_variable_post_assignment_inserts() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}
            local menu = {}

            config.launch_menu = menu
            table.insert(menu, {
              label = 'Post Assignment Monitor',
              args = { 'top' },
            })

            return config
            "#,
        )
        .expect("expected WezTerm launch_menu post-assignment table.insert config");

        let launch_menu = overrides
            .launch_menu.clone().expect("expected launch_menu overrides");
        assert_eq!(launch_menu.len(), 1);
        assert_eq!(
            launch_menu[0].label.as_deref(),
            Some("Post Assignment Monitor")
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_launch_menu_bracket_key_table_insert_entries() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config['launch_menu'] = {}
            table.insert(config['launch_menu'], {
              label = 'Bracket Insert',
              args = { 'top' },
            })

            return config
            "#,
        )
        .expect("expected WezTerm launch_menu bracket-key table.insert config");

        let launch_menu = overrides
            .launch_menu.clone().expect("expected launch_menu overrides");
        assert_eq!(launch_menu.len(), 1);
        assert_eq!(launch_menu[0].label.as_deref(), Some("Bracket Insert"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_launch_menu_positioned_table_insert_entries() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.launch_menu = {
              {
                label = 'Existing Shell',
                args = { 'cmd' },
              },
            }
            table.insert(config.launch_menu, 1, {
              label = 'Inserted Monitor',
              args = { 'top' },
            })

            return config
            "#,
        )
        .expect("expected WezTerm positioned launch_menu table.insert config");

        let launch_menu = overrides
            .launch_menu.clone().expect("expected launch_menu overrides");
        assert_eq!(launch_menu.len(), 2);
        assert_eq!(launch_menu[0].label.as_deref(), Some("Inserted Monitor"));
        assert_eq!(launch_menu[1].label.as_deref(), Some("Existing Shell"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_positioned_launch_menu_variable_insert() {
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            local monitor = {
              label = 'Variable Monitor',
              args = { 'top' },
            }

            config.launch_menu = {
              {
                label = 'Existing Shell',
                args = { 'cmd' },
              },
            }
            table.insert(config.launch_menu, 1, monitor)

            return config
            "#,
        )
        .expect("expected WezTerm positioned launch_menu table.insert variable config");

        let launch_menu = overrides
            .launch_menu.clone().expect("expected launch_menu overrides");
        assert_eq!(launch_menu.len(), 2);
        assert_eq!(launch_menu[0].label.as_deref(), Some("Variable Monitor"));
        assert_eq!(launch_menu[1].label.as_deref(), Some("Existing Shell"));
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_indexed_launch_menu_value_prefix_comments() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.launch_menu = {
              [1] =
                -- launch menu item
                {
                  label = 'System Monitor',
                  args = { 'top', '-H' },
                  cwd = '/tmp/project',
                  set_environment_variables = {
                    LAUNCH_MENU = '1',
                  },
                },
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch_menu config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::launch_menu_items(),
                title: None,
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));

        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "System Monitor");
        assert!(app.command_palette_execute_entry(entries[0].clone()));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-H"]);
        assert_eq!(launch.cwd(), Some("/tmp/project"));
        assert_eq!(
            launch.environment().get("LAUNCH_MENU"),
            Some(&"1".to_owned())
        );
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_launch_menu_config_long_bracket_key_query() {
        let mut app = NativeWindowApp::new(None);
        let overrides = super::native_config_overrides_from_wezterm_lua_config(
            r#"
            local wezterm = require 'wezterm'
            local config = {}

            config.launch_menu = {
              {
                [[=[label]=]] = 'System Monitor',
                [[=[args]=]] = { 'top', '-H' },
                [[=[cwd]=]] = '/tmp/project',
                [[=[set_environment_variables]=]] = {
                  [[=[LAUNCH_MENU]=]] = '1',
                },
              },
            }

            return config
            "#,
        )
        .expect("expected WezTerm launch_menu config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::launch_menu_items(),
                title: Some("Pick Launch".to_owned()),
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));

        app.command_palette_set_query("monitor".to_owned());
        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "System Monitor");
        assert!(app.command_palette_execute_entry(entries[0].clone()));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "top");
        assert_eq!(launch.args(), ["-H"]);
        assert_eq!(launch.cwd(), Some("/tmp/project"));
        assert_eq!(
            launch.environment().get("LAUNCH_MENU"),
            Some(&"1".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_parses_wezterm_lua_config_launch_menu_default_program_entries() {
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

            return config
            "#,
        )
        .expect("expected WezTerm launch_menu config");
        app.set_config_overrides(overrides);

        assert!(app.command_palette_execute(WindowCommand::ShowLauncherArgs(
            WindowShowLauncherArgs {
                flags: WindowShowLauncherFlags::launch_menu_items(),
                title: None,
                alphabet: None,
                help_text: None,
                fuzzy_help_text: None,
            },
        )));

        let entries = app.command_palette_filtered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label(), "Project Shell");
        assert!(app.command_palette_execute_entry(entries[0].clone()));

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.active_tab_id(), rterm_core::TabId::new(2));
        assert_eq!(launch.program(), "powershell");
        assert_eq!(launch.args(), ["-NoProfile"]);
        assert_eq!(launch.cwd(), Some("C:/Project Dir"));
        assert_eq!(
            launch.environment().get("PROJECT_MODE"),
            Some(&"dev".to_owned())
        );
        assert!(app.command_palette.is_none());
    }

    #[test]
    fn window_app_confirmation_accepts_with_enter() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.confirmation_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::Confirmation(
            WindowConfirmationOptions {
                message: "Run deployment?".to_owned(),
                action: Box::new(WindowCommand::Nop),
                cancel: None,
            },
        )));

        assert!(app.command_palette.is_none());
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Run deployment? Enter/Y=yes Esc/N=no"
        );

        assert!(app.handle_confirmation_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()));

        assert!(app.confirmation.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeConfirmation {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                accepted: true,
            }]
        );
    }

    #[test]
    fn window_app_confirmation_accept_dispatches_nested_action() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        assert!(app.command_palette_execute(WindowCommand::Confirmation(
            WindowConfirmationOptions {
                message: "Send command?".to_owned(),
                action: Box::new(WindowCommand::SendString("yes\n".to_owned())),
                cancel: Some(Box::new(WindowCommand::SendString("no\n".to_owned()))),
            },
        )));

        assert!(app.handle_confirmation_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()));

        assert!(app.confirmation.is_none());
        assert_eq!(written.lock().unwrap().as_slice(), b"yes\n");
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "confirmation message Send command? action send string yes cancel send string no"
                .to_owned(),
        );
        let command = WindowCommand::Confirmation(WindowConfirmationOptions {
            message: "Send command?".to_owned(),
            action: Box::new(WindowCommand::SendString("yes".to_owned())),
            cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Send command? Enter/Y=yes Esc/N=no"
        );

        assert!(app.handle_confirmation_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()));
        assert!(app.confirmation.is_none());
        assert_eq!(written.lock().unwrap().as_slice(), b"yes");
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_query_with_quoted_message() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "confirmation message \"Send command?\" action send string yes cancel send string no"
                .to_owned(),
        );

        let command = WindowCommand::Confirmation(WindowConfirmationOptions {
            message: "Send command?".to_owned(),
            action: Box::new(WindowCommand::SendString("yes".to_owned())),
            cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Send command? Enter/Y=yes Esc/N=no"
        );
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_action_name_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "confirmationmessage \"Send command?\" action sendstring yes cancel sendstring no"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Confirmation(WindowConfirmationOptions {
                message: "Send command?".to_owned(),
                action: Box::new(WindowCommand::SendString("yes".to_owned())),
                cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_wezterm_action_table_call_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Confirmation { message = \"Send command?\", action = \"sendstring yes\", cancel = \"sendstring no\" }"
                .to_owned(),
        );
        let command = WindowCommand::Confirmation(WindowConfirmationOptions {
            message: "Send command?".to_owned(),
            action: Box::new(WindowCommand::SendString("yes".to_owned())),
            cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Send command? Enter/Y=yes Esc/N=no"
        );

        assert!(app.handle_confirmation_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()));
        assert!(app.confirmation.is_none());
        assert_eq!(written.lock().unwrap().as_slice(), b"yes");
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_wezterm_formatted_message_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Confirmation { message = wezterm.format { { Text = 'Send' }, { Text = ' command?' } }, action = \"sendstring yes\", cancel = \"sendstring no\" }"
                .to_owned(),
        );
        let command = WindowCommand::Confirmation(WindowConfirmationOptions {
            message: "Send command?".to_owned(),
            action: Box::new(WindowCommand::SendString("yes".to_owned())),
            cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Send command? Enter/Y=yes Esc/N=no"
        );

        assert!(app.handle_confirmation_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()));
        assert!(app.confirmation.is_none());
        assert_eq!(written.lock().unwrap().as_slice(), b"yes");
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_wezterm_action_default_message_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Confirmation { action = \"sendstring yes\" }".to_owned(),
        );
        let command = WindowCommand::Confirmation(WindowConfirmationOptions {
            message: " Really continue?".to_owned(),
            action: Box::new(WindowCommand::SendString("yes".to_owned())),
            cancel: None,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] -  Really continue? Enter/Y=yes Esc/N=no"
        );

        assert!(app.handle_confirmation_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()));
        assert!(app.confirmation.is_none());
        assert_eq!(written.lock().unwrap().as_slice(), b"yes");
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_wezterm_action_callback_fields_query() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.confirmation_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Confirmation { message = \"Run deployment?\", action = wezterm.action_callback(function(window, pane) end), cancel = wezterm.action_callback(function(window, pane) end) }"
                .to_owned(),
        );
        let command = WindowCommand::Confirmation(WindowConfirmationOptions {
            message: "Run deployment?".to_owned(),
            action: Box::new(WindowCommand::Nop),
            cancel: Some(Box::new(WindowCommand::Nop)),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Run deployment? Enter/Y=yes Esc/N=no"
        );

        assert!(app.handle_confirmation_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()));
        assert!(app.confirmation.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeConfirmation {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                accepted: true,
            }]
        );
    }

    #[test]
    fn window_app_confirmation_static_action_callback_performs_nested_action() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Confirmation { message = \"Send command?\", action = wezterm.action_callback(function(window, pane) window:perform_action(wezterm.action.SendString 'yes', pane) end) }"
                .to_owned(),
        );

        let [command] = app.command_palette_filtered_commands().try_into().unwrap();
        assert!(app.command_palette_execute(command));
        assert!(app.handle_confirmation_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()));

        assert_eq!(written.lock().unwrap().as_slice(), b"yes");
        assert!(app.confirmation.is_none());
    }

    #[test]
    fn window_app_confirmation_documented_action_callback_spawns_new_window() {
        let mut app = NativeWindowApp::new_with_command(
            None,
            rterm_pty::PtyCommand::new("powershell").with_args(["-NoProfile"]),
        );

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            r#"
act.Confirmation {
  message = 'Do you want to run htop in a new window?',
  action = wezterm.action_callback(function(window, pane)
    window:perform_action(
      act.SpawnCommandInNewWindow { args = { 'htop' } },
      pane
    )
  end),
}
"#
            .to_owned(),
        );

        let [command] = app.command_palette_filtered_commands().try_into().unwrap();
        assert!(app.command_palette_execute(command));
        assert!(app.handle_confirmation_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()));

        assert!(app.confirmation.is_none());
        assert_eq!(app.app_shell.pending_windows().len(), 1);
        let pending_window = app
            .app_shell
            .pending_windows()
            .first()
            .expect("spawn window should request a pending window");
        let launch = pending_window.tab().panes()[0].launch();
        assert_eq!(launch.program(), "htop");
        assert!(launch.args().is_empty());
    }

    #[test]
    fn window_app_confirmation_static_action_callback_sends_pane_text() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Confirmation { message = \"Send command?\", action = wezterm.action_callback(function(window, pane) pane:send_text('yes') end) }"
                .to_owned(),
        );

        let [command] = app.command_palette_filtered_commands().try_into().unwrap();
        assert!(app.command_palette_execute(command));
        assert!(app.handle_confirmation_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()));

        assert_eq!(written.lock().unwrap().as_slice(), b"yes");
        assert!(app.confirmation.is_none());
    }

    #[test]
    fn window_app_confirmation_static_cancel_callback_performs_nested_action() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Confirmation { message = \"Send command?\", action = \"sendstring yes\", cancel = wezterm.action_callback(function(window, pane) window:perform_action(wezterm.action.SendString 'no', pane) end) }"
                .to_owned(),
        );

        let [command] = app.command_palette_filtered_commands().try_into().unwrap();
        assert!(app.command_palette_execute(command));
        assert!(
            app.handle_confirmation_key(&Key::Named(NamedKey::Escape), ModifiersState::empty())
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"no");
        assert!(app.confirmation.is_none());
    }

    #[test]
    fn window_app_confirmation_static_cancel_callback_sends_pane_paste() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Confirmation { message = \"Send command?\", action = \"sendstring yes\", cancel = wezterm.action_callback(function(window, pane) pane:send_paste('no\\n') end) }"
                .to_owned(),
        );

        let [command] = app.command_palette_filtered_commands().try_into().unwrap();
        assert!(app.command_palette_execute(command));
        assert!(
            app.handle_confirmation_key(&Key::Named(NamedKey::Escape), ModifiersState::empty())
        );

        let expected = encode_window_paste("no\n", false, DEFAULT_CANONICALIZE_PASTED_NEWLINES);
        assert_eq!(written.lock().unwrap().as_slice(), expected.as_slice());
        assert!(app.confirmation.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_wezterm_action_table_long_bracket_key_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Confirmation { [[=[message]=]] = [[Send command?]], [[=[action]=]] = [[sendstring yes]], [[=[cancel]=]] = [[sendstring no]] }"
                .to_owned(),
        );
        let command = WindowCommand::Confirmation(WindowConfirmationOptions {
            message: "Send command?".to_owned(),
            action: Box::new(WindowCommand::SendString("yes".to_owned())),
            cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Send command? Enter/Y=yes Esc/N=no"
        );

        assert!(app.handle_confirmation_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()));
        assert!(app.confirmation.is_none());
        assert_eq!(written.lock().unwrap().as_slice(), b"yes");
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_wezterm_action_parenthesized_table_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Confirmation({ message = \"Send command?\", action = \"sendstring yes\", cancel = \"sendstring no\" })"
                .to_owned(),
        );
        let command = WindowCommand::Confirmation(WindowConfirmationOptions {
            message: "Send command?".to_owned(),
            action: Box::new(WindowCommand::SendString("yes".to_owned())),
            cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Send command? Enter/Y=yes Esc/N=no"
        );

        assert!(app.handle_confirmation_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()));
        assert!(app.confirmation.is_none());
        assert_eq!(written.lock().unwrap().as_slice(), b"yes");
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_wezterm_action_table_trailing_comma_query() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.Confirmation({ message = \"Send command?\", action = \"sendstring yes\", cancel = \"sendstring no\", })"
                .to_owned(),
        );
        let command = WindowCommand::Confirmation(WindowConfirmationOptions {
            message: "Send command?".to_owned(),
            action: Box::new(WindowCommand::SendString("yes".to_owned())),
            cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Send command? Enter/Y=yes Esc/N=no"
        );

        assert!(app.handle_confirmation_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()));
        assert!(app.confirmation.is_none());
        assert_eq!(written.lock().unwrap().as_slice(), b"yes");
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "confirmation=message Send? action send string yes cancel send string no".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Confirmation(WindowConfirmationOptions {
                message: "Send?".to_owned(),
                action: Box::new(WindowCommand::SendString("yes".to_owned())),
                cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_message_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "confirmationmessage=Send? action=sendstring yes cancel=sendstring no".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Confirmation(WindowConfirmationOptions {
                message: "Send?".to_owned(),
                action: Box::new(WindowCommand::SendString("yes".to_owned())),
                cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_equals_field_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "confirmation message=Send? action=send string yes cancel=send string no".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Confirmation(WindowConfirmationOptions {
                message: "Send?".to_owned(),
                action: Box::new(WindowCommand::SendString("yes".to_owned())),
                cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_action_name_equals_field_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "confirmationmessage Send? action=sendstring yes cancel=sendstring no".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Confirmation(WindowConfirmationOptions {
                message: "Send?".to_owned(),
                action: Box::new(WindowCommand::SendString("yes".to_owned())),
                cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_action_name_message_equals_field_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "confirmationmessage message=Send? action=sendstring yes cancel=sendstring no"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Confirmation(WindowConfirmationOptions {
                message: "Send?".to_owned(),
                action: Box::new(WindowCommand::SendString("yes".to_owned())),
                cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_action_name_action_before_message_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "confirmationmessage action sendstring yes message Send?".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Confirmation(WindowConfirmationOptions {
                message: "Send?".to_owned(),
                action: Box::new(WindowCommand::SendString("yes".to_owned())),
                cancel: None,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_mixed_case_action_name_message_field_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "ConfirmationMessage Message=Send? Action=sendstring yes Cancel=sendstring no"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Confirmation(WindowConfirmationOptions {
                message: "Send?".to_owned(),
                action: Box::new(WindowCommand::SendString("yes".to_owned())),
                cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_mixed_case_equals_field_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "confirmation Message=Send? Action=sendstring yes Cancel=sendstring no".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Confirmation(WindowConfirmationOptions {
                message: "Send?".to_owned(),
                action: Box::new(WindowCommand::SendString("yes".to_owned())),
                cancel: Some(Box::new(WindowCommand::SendString("no".to_owned()))),
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_query_with_field_words_in_message() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "confirmation message Confirm action before deploy action send string yes".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Confirmation(WindowConfirmationOptions {
                message: "Confirm action before deploy".to_owned(),
                action: Box::new(WindowCommand::SendString("yes".to_owned())),
                cancel: None,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_confirmation_query_with_action_like_words_in_message() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "confirmation message Review action send string sample action send string yes"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::Confirmation(WindowConfirmationOptions {
                message: "Review action send string sample".to_owned(),
                action: Box::new(WindowCommand::SendString("yes".to_owned())),
                cancel: None,
            })]
        );
    }

    #[test]
    fn window_app_confirmation_cancels_with_escape() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.confirmation_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        assert!(app.command_palette_execute(WindowCommand::Confirmation(
            WindowConfirmationOptions {
                message: "Delete profile?".to_owned(),
                action: Box::new(WindowCommand::Nop),
                cancel: None,
            },
        )));

        assert!(
            app.handle_confirmation_key(&Key::Named(NamedKey::Escape), ModifiersState::empty())
        );

        assert!(app.confirmation.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativeConfirmation {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                accepted: false,
            }]
        );
    }

    #[test]
    fn window_app_confirmation_cancel_dispatches_nested_cancel_action() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        assert!(app.command_palette_execute(WindowCommand::Confirmation(
            WindowConfirmationOptions {
                message: "Send command?".to_owned(),
                action: Box::new(WindowCommand::SendString("yes\n".to_owned())),
                cancel: Some(Box::new(WindowCommand::SendString("no\n".to_owned()))),
            },
        )));

        assert!(
            app.handle_confirmation_key(&Key::Named(NamedKey::Escape), ModifiersState::empty())
        );

        assert!(app.confirmation.is_none());
        assert_eq!(written.lock().unwrap().as_slice(), b"no\n");
    }

    #[test]
    fn window_app_prompt_input_line_submits_entered_text_to_native_handler() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.prompt_input_line_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        app.enter_command_palette_mode();
        assert!(app.command_palette_execute(WindowCommand::PromptInputLine(
            WindowPromptInputLineOptions {
                description: "Rename tab".to_owned(),
                prompt: Some("name: ".to_owned()),
                initial_value: Some("old".to_owned()),
                action: None,
            },
        )));

        assert!(app.command_palette.is_none());
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Rename tab: name: old"
        );

        assert!(app.handle_prompt_input_line_key(
            &Key::Named(NamedKey::Backspace),
            ModifiersState::empty()
        ));
        assert!(
            app.handle_prompt_input_line_key(&Key::Character("s".into()), ModifiersState::empty())
        );
        assert!(
            app.handle_prompt_input_line_key(&Key::Named(NamedKey::Enter), ModifiersState::empty())
        );

        assert!(app.prompt_input_line.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativePromptInputLine {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                line: Some("ols".to_owned()),
            }]
        );
    }

    #[test]
    fn window_app_dispatches_palette_prompt_input_line_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "prompt input line description Rename tab prompt name: initial_value old".to_owned(),
        );
        let command = WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
            description: "Rename tab".to_owned(),
            prompt: Some("name:".to_owned()),
            initial_value: Some("old".to_owned()),
            action: None,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));

        assert!(app.command_palette.is_none());
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Rename tab: name:old"
        );
    }

    #[test]
    fn window_app_dispatches_palette_prompt_input_line_query_with_field_words_in_description() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "prompt input line description Confirm prompt text prompt > initial_value ok"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::PromptInputLine(
                WindowPromptInputLineOptions {
                    description: "Confirm prompt text".to_owned(),
                    prompt: Some(">".to_owned()),
                    initial_value: Some("ok".to_owned()),
                    action: None,
                }
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_prompt_input_line_query_with_quoted_fields() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "prompt input line description \"Rename tab\" prompt \"name: \" initial_value \"old name\""
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::PromptInputLine(
                WindowPromptInputLineOptions {
                    description: "Rename tab".to_owned(),
                    prompt: Some("name: ".to_owned()),
                    initial_value: Some("old name".to_owned()),
                    action: None,
                }
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_prompt_input_line_action_name_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "promptinputline description \"Rename tab\" prompt \"name: \" initial_value \"old name\""
                .to_owned(),
        );

        let command = WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
            description: "Rename tab".to_owned(),
            prompt: Some("name: ".to_owned()),
            initial_value: Some("old name".to_owned()),
            action: None,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));

        assert!(app.command_palette.is_none());
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Rename tab: name: old name"
        );
    }

    #[test]
    fn window_app_dispatches_palette_prompt_input_line_wezterm_action_table_call_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.PromptInputLine { description = \"Rename tab\", prompt = \"name: \", initial_value = \"old name\" }"
                .to_owned(),
        );

        let command = WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
            description: "Rename tab".to_owned(),
            prompt: Some("name: ".to_owned()),
            initial_value: Some("old name".to_owned()),
            action: None,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));

        assert!(app.command_palette.is_none());
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Rename tab: name: old name"
        );
    }

    #[test]
    fn window_app_dispatches_palette_prompt_input_line_wezterm_formatted_text_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.PromptInputLine { description = wezterm.format { { Text = 'Rename' }, { Text = ' tab' } }, prompt = wezterm.format { { Text = 'name' }, { Text = ': ' } }, initial_value = \"old name\" }"
                .to_owned(),
        );

        let command = WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
            description: "Rename tab".to_owned(),
            prompt: Some("name: ".to_owned()),
            initial_value: Some("old name".to_owned()),
            action: None,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));

        assert!(app.command_palette.is_none());
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Rename tab: name: old name"
        );
    }

    #[test]
    fn window_app_dispatches_palette_prompt_input_line_wezterm_action_callback_field_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.PromptInputLine { description = \"Rename tab\", prompt = \"name: \", initial_value = \"old name\", action = wezterm.action_callback(function(window, pane, line) end) }"
                .to_owned(),
        );

        let command = WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
            description: "Rename tab".to_owned(),
            prompt: Some("name: ".to_owned()),
            initial_value: Some("old name".to_owned()),
            action: None,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));

        assert!(app.command_palette.is_none());
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Rename tab: name: old name"
        );
    }

    #[test]
    fn window_app_dispatches_palette_prompt_input_line_wezterm_nested_action_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.PromptInputLine { description = \"Confirm\", action = act.Hide }"
                .to_owned(),
        );

        let command = WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
            description: "Confirm".to_owned(),
            prompt: None,
            initial_value: None,
            action: Some(WindowPromptInputLineAction::Command(Box::new(
                WindowCommand::Hide,
            ))),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert!(!app.window_hide_requested_for_test());

        assert!(
            app.handle_prompt_input_line_key(&Key::Named(NamedKey::Enter), ModifiersState::empty())
        );

        assert!(app.window_hide_requested_for_test());
        assert!(app.prompt_input_line.is_none());
    }

    #[test]
    fn window_app_prompt_input_line_static_action_callback_can_rename_tab() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.PromptInputLine { description = \"Enter new name for tab\", action = wezterm.action_callback(function(window, pane, line) if line then window:active_tab():set_title(line) end end) }"
                .to_owned(),
        );

        let [command] = app.command_palette_filtered_commands().try_into().unwrap();
        assert!(app.command_palette_execute(command));
        assert!(app.handle_prompt_input_line_key(
            &Key::Character("build-prod".into()),
            ModifiersState::empty(),
        ));
        assert!(
            app.handle_prompt_input_line_key(&Key::Named(NamedKey::Enter), ModifiersState::empty())
        );

        assert_eq!(app.app_shell.active_tab().title(), Some("build-prod"));
        assert!(app.prompt_input_line.is_none());
    }

    #[test]
    fn window_app_prompt_input_line_static_action_callback_switches_workspace_from_line() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.PromptInputLine { description = \"Enter name for new workspace\", action = wezterm.action_callback(function(window, pane, line) if line then window:perform_action(wezterm.action.SwitchToWorkspace { name = line }, pane) end end) }"
                .to_owned(),
        );

        let [command] = app.command_palette_filtered_commands().try_into().unwrap();
        assert!(app.command_palette_execute(command));
        assert!(
            app.handle_prompt_input_line_key(
                &Key::Character("ops".into()),
                ModifiersState::empty()
            )
        );
        assert!(
            app.handle_prompt_input_line_key(&Key::Named(NamedKey::Enter), ModifiersState::empty())
        );

        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_eq!(app.app_shell.active_workspace().name(), "ops");
        assert!(app.prompt_input_line.is_none());
    }

    #[test]
    fn window_app_prompt_input_line_static_action_callback_sends_line_text() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.PromptInputLine { description = \"Send text\", action = wezterm.action_callback(function(window, pane, line) if line then pane:send_text(line) end end) }"
                .to_owned(),
        );

        let [command] = app.command_palette_filtered_commands().try_into().unwrap();
        assert!(app.command_palette_execute(command));
        assert!(app.handle_prompt_input_line_key(
            &Key::Character("deploy".into()),
            ModifiersState::empty()
        ));
        assert!(
            app.handle_prompt_input_line_key(&Key::Named(NamedKey::Enter), ModifiersState::empty())
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"deploy");
        assert!(app.prompt_input_line.is_none());
    }

    #[test]
    fn window_app_prompt_input_line_static_action_callback_pastes_line_text() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));
        app.handle_pty_output(b"\x1b[?2004h").unwrap();
        assert!(app.runtime.bracketed_paste());

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.PromptInputLine { description = \"Paste text\", action = wezterm.action_callback(function(window, pane, line) if line then pane:send_paste(line) end end) }"
                .to_owned(),
        );

        let [command] = app.command_palette_filtered_commands().try_into().unwrap();
        assert!(app.command_palette_execute(command));
        assert!(app.handle_prompt_input_line_key(
            &Key::Character("deploy".into()),
            ModifiersState::empty()
        ));
        assert!(
            app.handle_prompt_input_line_key(&Key::Named(NamedKey::Enter), ModifiersState::empty())
        );

        let expected = encode_window_paste(
            "deploy",
            app.runtime.bracketed_paste(),
            DEFAULT_CANONICALIZE_PASTED_NEWLINES,
        );
        assert_eq!(written.lock().unwrap().as_slice(), expected.as_slice());
        assert!(app.prompt_input_line.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_prompt_input_line_table_long_bracket_key_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.PromptInputLine { [[=[description]=]] = [[Rename tab]], [[=[prompt]=]] = [[name: ]], [[=[initial_value]=]] = [[old name]] }"
                .to_owned(),
        );

        let command = WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
            description: "Rename tab".to_owned(),
            prompt: Some("name: ".to_owned()),
            initial_value: Some("old name".to_owned()),
            action: None,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));

        assert!(app.command_palette.is_none());
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Rename tab: name: old name"
        );
    }

    #[test]
    fn window_app_dispatches_palette_prompt_input_line_wezterm_action_parenthesized_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.PromptInputLine({ description = \"Rename tab\", prompt = \"name: \", initial_value = \"old name\" })"
                .to_owned(),
        );

        let command = WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
            description: "Rename tab".to_owned(),
            prompt: Some("name: ".to_owned()),
            initial_value: Some("old name".to_owned()),
            action: None,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));

        assert!(app.command_palette.is_none());
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Rename tab: name: old name"
        );
    }

    #[test]
    fn window_app_dispatches_palette_prompt_input_line_wezterm_action_table_trailing_comma_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.PromptInputLine({ description = \"Rename tab\", prompt = \"name: \", initial_value = \"old name\", })"
                .to_owned(),
        );

        let command = WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
            description: "Rename tab".to_owned(),
            prompt: Some("name: ".to_owned()),
            initial_value: Some("old name".to_owned()),
            action: None,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));

        assert!(app.command_palette.is_none());
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Rename tab: name: old name"
        );
    }

    #[test]
    fn window_app_dispatches_palette_prompt_input_line_wezterm_action_alias_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "act.PromptInputLine { description = \"Rename tab\", prompt = \"name: \", initial_value = \"old name\" }"
                .to_owned(),
        );

        let command = WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
            description: "Rename tab".to_owned(),
            prompt: Some("name: ".to_owned()),
            initial_value: Some("old name".to_owned()),
            action: None,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));

        assert!(app.command_palette.is_none());
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Rename tab: name: old name"
        );
    }

    #[test]
    fn window_app_dispatches_palette_prompt_input_line_hyphenated_initial_value_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "promptinputline description \"Rename tab\" prompt \"name: \" initial-value \"old name\""
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::PromptInputLine(
                WindowPromptInputLineOptions {
                    description: "Rename tab".to_owned(),
                    prompt: Some("name: ".to_owned()),
                    initial_value: Some("old name".to_owned()),
                    action: None,
                }
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_prompt_input_line_equals_field_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "promptinputline description=Rename prompt=name: initial-value=old".to_owned(),
        );

        let command = WindowCommand::PromptInputLine(WindowPromptInputLineOptions {
            description: "Rename".to_owned(),
            prompt: Some("name:".to_owned()),
            initial_value: Some("old".to_owned()),
            action: None,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));

        assert!(app.command_palette.is_none());
        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Rename: name:old"
        );
    }

    #[test]
    fn window_app_dispatches_palette_prompt_input_line_structured_equals_field_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "prompt input line description=Rename prompt=name: initial_value=old".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::PromptInputLine(
                WindowPromptInputLineOptions {
                    description: "Rename".to_owned(),
                    prompt: Some("name:".to_owned()),
                    initial_value: Some("old".to_owned()),
                    action: None,
                }
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_prompt_input_line_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "prompt input line=description=Rename prompt=name: initial_value=old".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::PromptInputLine(
                WindowPromptInputLineOptions {
                    description: "Rename".to_owned(),
                    prompt: Some("name:".to_owned()),
                    initial_value: Some("old".to_owned()),
                    action: None,
                }
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_prompt_input_line_compact_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "promptinputline=description=Rename prompt=name: initial_value=old".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::PromptInputLine(
                WindowPromptInputLineOptions {
                    description: "Rename".to_owned(),
                    prompt: Some("name:".to_owned()),
                    initial_value: Some("old".to_owned()),
                    action: None,
                }
            )]
        );
    }

    #[test]
    fn window_app_dispatches_palette_prompt_input_line_mixed_case_equals_field_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "PromptInputLine Description=Rename Prompt=name: Initial_Value=old".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::PromptInputLine(
                WindowPromptInputLineOptions {
                    description: "Rename".to_owned(),
                    prompt: Some("name:".to_owned()),
                    initial_value: Some("old".to_owned()),
                    action: None,
                }
            )]
        );
    }

    #[test]
    fn window_app_prompt_input_line_cancel_emits_none_to_native_handler() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let mut app = NativeWindowApp::new(None);
        app.prompt_input_line_handler = Box::new(move |event| {
            recorded.lock().unwrap().push(event.clone());
            true
        });

        assert!(app.command_palette_execute(WindowCommand::PromptInputLine(
            WindowPromptInputLineOptions {
                description: "Workspace".to_owned(),
                prompt: None,
                initial_value: None,
                action: None,
            },
        )));

        assert_eq!(
            app.effective_window_title(),
            "R-Term [workspace:1 tab:1 pane:1] - Workspace: > "
        );
        assert!(
            app.handle_prompt_input_line_key(&Key::Character("c".into()), ModifiersState::CONTROL)
        );

        assert!(app.prompt_input_line.is_none());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [NativePromptInputLine {
                window_id: rterm_core::WindowId::new(1),
                pane: rterm_core::PaneId::new(1),
                line: None,
            }]
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_query_with_field_words_in_title() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "input selector title Pick choices carefully choices yes=Yes ; no=No description Choose:"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::InputSelector(WindowInputSelectorOptions {
                title: "Pick choices carefully".to_owned(),
                choices: vec![
                    WindowInputSelectorChoice {
                        label: "Yes".to_owned(),
                        id: Some("yes".to_owned()),
                    },
                    WindowInputSelectorChoice {
                        label: "No".to_owned(),
                        id: Some("no".to_owned()),
                    },
                ],
                alphabet: None,
                description: Some("Choose:".to_owned()),
                fuzzy_description: None,
                fuzzy: false,
                action: None,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_query_with_quoted_semicolon_choice_label() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "input selector title Pick choices first=\"Alpha ; Beta\" ; second=Gamma".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::InputSelector(WindowInputSelectorOptions {
                title: "Pick".to_owned(),
                choices: vec![
                    WindowInputSelectorChoice {
                        label: "Alpha ; Beta".to_owned(),
                        id: Some("first".to_owned()),
                    },
                    WindowInputSelectorChoice {
                        label: "Gamma".to_owned(),
                        id: Some("second".to_owned()),
                    },
                ],
                alphabet: None,
                description: None,
                fuzzy_description: None,
                fuzzy: false,
                action: None,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_query_with_compact_choice_separators() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("input selector title Pick choices yes=Yes;no=No".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::InputSelector(WindowInputSelectorOptions {
                title: "Pick".to_owned(),
                choices: vec![
                    WindowInputSelectorChoice {
                        label: "Yes".to_owned(),
                        id: Some("yes".to_owned()),
                    },
                    WindowInputSelectorChoice {
                        label: "No".to_owned(),
                        id: Some("no".to_owned()),
                    },
                ],
                alphabet: None,
                description: None,
                fuzzy_description: None,
                fuzzy: false,
                action: None,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_query_with_quoted_fields() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "input selector title \"Pick Reply\" choices decline=\"No thanks\" ; lgtm=LGTM alphabet \"ab\" description \"Choose one:\" fuzzy_description \"Filter replies:\" fuzzy true"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::InputSelector(WindowInputSelectorOptions {
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
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_action_name_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "inputselector title \"Pick Reply\" choices decline=\"No thanks\" ; lgtm=LGTM alphabet \"ab\" description \"Choose one:\" fuzzy_description \"Filter replies:\" fuzzy true"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::InputSelector(WindowInputSelectorOptions {
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
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_wezterm_action_table_call_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.InputSelector { title = \"Pick Reply\", choices = \"decline=No thanks ; lgtm=LGTM\", alphabet = \"ab\", description = \"Choose one:\", fuzzy_description = \"Filter replies:\", fuzzy = true }"
                .to_owned(),
        );

        let command = WindowCommand::InputSelector(WindowInputSelectorOptions {
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
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));

        assert!(app.command_palette.is_none());
        assert!(
            app.effective_window_title()
                .contains("Pick Reply: Filter replies:")
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_wezterm_action_table_long_bracket_key_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.InputSelector { [[=[title]=]] = [[Pick Reply]], [[=[choices]=]] = [[decline=No thanks ; lgtm=LGTM]], alphabet = \"ab\" }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::InputSelector(WindowInputSelectorOptions {
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
                description: None,
                fuzzy_description: None,
                fuzzy: false,
                action: None,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_wezterm_action_choices_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.InputSelector { title = \"Pick Reply\", choices = { { id = \"decline\", label = \"No thanks\" }, { label = \"LGTM\" } }, alphabet = \"ab\", description = \"Choose one:\" }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::InputSelector(WindowInputSelectorOptions {
                title: "Pick Reply".to_owned(),
                choices: vec![
                    WindowInputSelectorChoice {
                        label: "No thanks".to_owned(),
                        id: Some("decline".to_owned()),
                    },
                    WindowInputSelectorChoice {
                        label: "LGTM".to_owned(),
                        id: None,
                    },
                ],
                alphabet: Some("ab".to_owned()),
                description: Some("Choose one:".to_owned()),
                fuzzy_description: None,
                fuzzy: false,
                action: None,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_choice_table_long_bracket_key_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.InputSelector { title = [[Pick Reply]], choices = { { [[=[label]=]] = [[No thanks]], [[=[id]=]] = [[decline]] }, { [[=[label]=]] = [[LGTM]], [[=[id]=]] = [[lgtm]] } }, alphabet = [[ab]], fuzzy = true }"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::InputSelector(WindowInputSelectorOptions {
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
                description: None,
                fuzzy_description: None,
                fuzzy: true,
                action: None,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_wezterm_action_parenthesized_table_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.InputSelector({ title = \"Pick Reply\", choices = \"decline=No thanks ; lgtm=LGTM\", alphabet = \"ab\", description = \"Choose one:\", fuzzy_description = \"Filter replies:\", fuzzy = true })"
                .to_owned(),
        );

        let command = WindowCommand::InputSelector(WindowInputSelectorOptions {
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
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));

        assert!(app.command_palette.is_none());
        assert!(
            app.effective_window_title()
                .contains("Pick Reply: Filter replies:")
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_wezterm_action_table_trailing_comma_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.InputSelector({ title = \"Pick Reply\", choices = \"decline=No thanks ; lgtm=LGTM\", alphabet = \"ab\", description = \"Choose one:\", fuzzy_description = \"Filter replies:\", fuzzy = true, })"
                .to_owned(),
        );

        let command = WindowCommand::InputSelector(WindowInputSelectorOptions {
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
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));

        assert!(app.command_palette.is_none());
        assert!(
            app.effective_window_title()
                .contains("Pick Reply: Filter replies:")
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_wezterm_action_callback_field_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.InputSelector { title = \"Pick Reply\", choices = { { label = \"No thanks\", id = \"decline\" }, { label = \"LGTM\", id = \"lgtm\" } }, action = wezterm.action_callback(function(window, pane, id, label) end) }"
                .to_owned(),
        );

        let command = WindowCommand::InputSelector(WindowInputSelectorOptions {
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
            alphabet: None,
            description: None,
            fuzzy_description: None,
            fuzzy: false,
            action: None,
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));

        assert!(app.command_palette.is_none());
        assert!(app.effective_window_title().contains("Pick Reply:"));
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_wezterm_nested_action_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.InputSelector { title = \"Pick Reply\", choices = { { label = \"No thanks\", id = \"decline\" }, { label = \"LGTM\", id = \"lgtm\" } }, alphabet = \"ab\", action = act.Hide }"
                .to_owned(),
        );

        let command = WindowCommand::InputSelector(WindowInputSelectorOptions {
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
            description: None,
            fuzzy_description: None,
            fuzzy: false,
            action: Some(WindowInputSelectorAction::Command(Box::new(
                WindowCommand::Hide,
            ))),
        });
        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![command.clone()]
        );
        assert!(app.command_palette_execute(command));
        assert!(!app.window_hide_requested_for_test());

        assert!(
            app.handle_input_selector_key(&Key::Character("b".into()), ModifiersState::empty())
        );

        assert!(app.window_hide_requested_for_test());
        assert!(app.input_selector.is_none());
    }

    #[test]
    fn window_app_input_selector_static_action_callback_sends_choice_id() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.InputSelector { title = \"Pick Reply\", choices = { { label = \"No thanks\", id = \"Regretfully, I decline.\" }, { label = \"LGTM\", id = \"This sounds right.\" } }, alphabet = \"ab\", action = wezterm.action_callback(function(window, pane, id, label) if id then pane:send_text(id) end end) }"
                .to_owned(),
        );

        let [command] = app.command_palette_filtered_commands().try_into().unwrap();
        assert!(app.command_palette_execute(command));
        assert!(
            app.handle_input_selector_key(&Key::Character("b".into()), ModifiersState::empty())
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"This sounds right.");
        assert!(app.input_selector.is_none());
    }

    #[test]
    fn window_app_input_selector_static_action_callback_pastes_choice_id() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.InputSelector { title = \"Pick Reply\", choices = { { label = \"No thanks\", id = \"Regretfully, I decline.\" }, { label = \"LGTM\", id = \"hello\\nworld\" } }, alphabet = \"ab\", action = wezterm.action_callback(function(window, pane, id, label) if id then pane:send_paste(id) end end) }"
                .to_owned(),
        );

        let [command] = app.command_palette_filtered_commands().try_into().unwrap();
        assert!(app.command_palette_execute(command));
        assert!(
            app.handle_input_selector_key(&Key::Character("b".into()), ModifiersState::empty())
        );

        let expected =
            encode_window_paste("hello\nworld", false, DEFAULT_CANONICALIZE_PASTED_NEWLINES);
        assert_eq!(written.lock().unwrap().as_slice(), expected.as_slice());
        assert!(app.input_selector.is_none());
    }

    #[test]
    fn window_app_input_selector_static_action_callback_sends_choice_label_without_id() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.InputSelector { title = \"Pick Number\", choices = { { label = \"One\" }, { label = \"Two\" } }, alphabet = \"ab\", action = wezterm.action_callback(function(window, pane, id, label) if label then pane:send_text(label) end end) }"
                .to_owned(),
        );

        let [command] = app.command_palette_filtered_commands().try_into().unwrap();
        assert!(app.command_palette_execute(command));
        assert!(
            app.handle_input_selector_key(&Key::Character("b".into()), ModifiersState::empty())
        );

        assert_eq!(written.lock().unwrap().as_slice(), b"Two");
        assert!(app.input_selector.is_none());
    }

    #[test]
    fn window_app_input_selector_static_action_callback_pastes_choice_label_without_id() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut app = NativeWindowApp::new(None);
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&written))));

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.InputSelector { title = \"Pick Number\", choices = { { label = \"One\" }, { label = \"hello\\nworld\" } }, alphabet = \"ab\", action = wezterm.action_callback(function(window, pane, id, label) if label then pane:send_paste(label) end end) }"
                .to_owned(),
        );

        let [command] = app.command_palette_filtered_commands().try_into().unwrap();
        assert!(app.command_palette_execute(command));
        assert!(
            app.handle_input_selector_key(&Key::Character("b".into()), ModifiersState::empty())
        );

        let expected =
            encode_window_paste("hello\nworld", false, DEFAULT_CANONICALIZE_PASTED_NEWLINES);
        assert_eq!(written.lock().unwrap().as_slice(), expected.as_slice());
        assert!(app.input_selector.is_none());
    }

    #[test]
    fn window_app_input_selector_static_action_callback_switches_workspace_from_choice() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "wezterm.action.InputSelector { title = \"Choose Workspace\", choices = { { id = \"C:/Users/me\", label = \"Home\" }, { id = \"C:/Users/me/work\", label = \"Work\" } }, alphabet = \"ab\", action = wezterm.action_callback(function(inner_window, inner_pane, id, label) if not id and not label then wezterm.log_info 'cancelled' else inner_window:perform_action(act.SwitchToWorkspace { name = label, spawn = { label = 'Workspace: ' .. label, cwd = id } }, inner_pane) end end) }"
                .to_owned(),
        );

        let [command] = app.command_palette_filtered_commands().try_into().unwrap();
        assert!(app.command_palette_execute(command));
        assert!(
            app.handle_input_selector_key(&Key::Character("b".into()), ModifiersState::empty())
        );

        let launch = app.app_shell.active_pane().launch();
        assert_eq!(app.app_shell.workspaces().len(), 2);
        assert_eq!(app.app_shell.active_workspace().name(), "Work");
        assert_eq!(launch.cwd(), Some("C:/Users/me/work"));
        assert!(app.input_selector.is_none());
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_hyphenated_fuzzy_description_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "inputselector title Pick choices yes=Yes ; no=No fuzzy-description \"Filter replies:\" fuzzy true"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::InputSelector(WindowInputSelectorOptions {
                title: "Pick".to_owned(),
                choices: vec![
                    WindowInputSelectorChoice {
                        label: "Yes".to_owned(),
                        id: Some("yes".to_owned()),
                    },
                    WindowInputSelectorChoice {
                        label: "No".to_owned(),
                        id: Some("no".to_owned()),
                    },
                ],
                alphabet: None,
                description: None,
                fuzzy_description: Some("Filter replies:".to_owned()),
                fuzzy: true,
                action: None,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_fuzzy_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "input selector title Pick choices yes=Yes ; no=No fuzzy=true".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::InputSelector(WindowInputSelectorOptions {
                title: "Pick".to_owned(),
                choices: vec![
                    WindowInputSelectorChoice {
                        label: "Yes".to_owned(),
                        id: Some("yes".to_owned()),
                    },
                    WindowInputSelectorChoice {
                        label: "No".to_owned(),
                        id: Some("no".to_owned()),
                    },
                ],
                alphabet: None,
                description: None,
                fuzzy_description: None,
                fuzzy: true,
                action: None,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_action_name_fuzzy_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "inputselector title Pick choices yes=Yes ; no=No fuzzy=false".to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::InputSelector(WindowInputSelectorOptions {
                title: "Pick".to_owned(),
                choices: vec![
                    WindowInputSelectorChoice {
                        label: "Yes".to_owned(),
                        id: Some("yes".to_owned()),
                    },
                    WindowInputSelectorChoice {
                        label: "No".to_owned(),
                        id: Some("no".to_owned()),
                    },
                ],
                alphabet: None,
                description: None,
                fuzzy_description: None,
                fuzzy: false,
                action: None,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_equals_field_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "inputselector title=Pick choices=yes=Yes ; no=No alphabet=ab description=Choose fuzzy-description=Filter fuzzy=true"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::InputSelector(WindowInputSelectorOptions {
                title: "Pick".to_owned(),
                choices: vec![
                    WindowInputSelectorChoice {
                        label: "Yes".to_owned(),
                        id: Some("yes".to_owned()),
                    },
                    WindowInputSelectorChoice {
                        label: "No".to_owned(),
                        id: Some("no".to_owned()),
                    },
                ],
                alphabet: Some("ab".to_owned()),
                description: Some("Choose".to_owned()),
                fuzzy_description: Some("Filter".to_owned()),
                fuzzy: true,
                action: None,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_structured_equals_field_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query(
            "input selector title=Pick choices=yes=Yes ; no=No alphabet=ab description=Choose fuzzy_description=Filter fuzzy=false"
                .to_owned(),
        );

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::InputSelector(WindowInputSelectorOptions {
                title: "Pick".to_owned(),
                choices: vec![
                    WindowInputSelectorChoice {
                        label: "Yes".to_owned(),
                        id: Some("yes".to_owned()),
                    },
                    WindowInputSelectorChoice {
                        label: "No".to_owned(),
                        id: Some("no".to_owned()),
                    },
                ],
                alphabet: Some("ab".to_owned()),
                description: Some("Choose".to_owned()),
                fuzzy_description: Some("Filter".to_owned()),
                fuzzy: false,
                action: None,
            })]
        );
    }

    #[test]
    fn window_app_dispatches_palette_input_selector_equals_query() {
        let mut app = NativeWindowApp::new(None);

        app.enter_command_palette_mode();
        app.command_palette_set_query("input selector=title=Pick choices=yes=Yes".to_owned());

        assert_eq!(
            app.command_palette_filtered_commands(),
            vec![WindowCommand::InputSelector(WindowInputSelectorOptions {
                title: "Pick".to_owned(),
                choices: vec![WindowInputSelectorChoice {
                    label: "Yes".to_owned(),
                    id: Some("yes".to_owned()),
                }],
                alphabet: None,
                description: None,
                fuzzy_description: None,
                fuzzy: false,
                action: None,
            })]
        );
    }

