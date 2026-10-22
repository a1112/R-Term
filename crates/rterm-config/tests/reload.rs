use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use rterm_config::{
    ConfigDiff, ConfigDiscoveryInputs, ConfigLifecycle, ConfigLifecycleEvent, ConfigSource,
    ConfigSourceErrorKind, EffectiveConfig, FixedWindowDebouncer, ResolvedConfigSource,
    SourceChange,
};

struct TestDir(PathBuf);

impl TestDir {
    fn new(label: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rterm-config-reload-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("create isolated reload fixture directory");
        Self(path)
    }

    fn join(&self, path: impl AsRef<Path>) -> PathBuf {
        self.0.join(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn discovery() -> ConfigDiscoveryInputs {
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

fn parse_config(source: &str) -> Result<EffectiveConfig, String> {
    let document = source
        .parse::<toml::Table>()
        .map_err(|error| error.to_string())?;
    let mut config = EffectiveConfig::default();
    if let Some(font) = document.get("font").and_then(toml::Value::as_table)
        && let Some(family) = font.get("family").and_then(toml::Value::as_str)
    {
        family.clone_into(&mut Arc::make_mut(&mut config.font).family);
    }
    if let Some(lifecycle) = document.get("lifecycle").and_then(toml::Value::as_table)
        && let Some(reload) = lifecycle
            .get("reload_on_change")
            .and_then(toml::Value::as_bool)
    {
        Arc::make_mut(&mut config.lifecycle).reload_on_change = reload;
    }
    Ok(config)
}

fn config_diff(before: &EffectiveConfig, after: &EffectiveConfig) -> ConfigDiff {
    ConfigDiff::between(before, after)
}

#[test]
fn missing_optional_file_falls_through_while_missing_required_file_is_rejected() {
    let root = TestDir::new("missing");
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();
    let defaults = EffectiveConfig::default();
    let mut optional = ConfigLifecycle::<EffectiveConfig, String>::new(
        ConfigDiscoveryInputs {
            home_dir: Some(home),
            ..discovery()
        },
        false,
        None,
        defaults.clone(),
    );

    let optional_attempt = optional.attempt_reload(parse_config);
    assert_eq!(optional_attempt.resolved, ResolvedConfigSource::Defaults);
    assert_eq!(optional_attempt.result.as_ref().unwrap(), &defaults);
    let ConfigLifecycleEvent::Applied { snapshot, .. } =
        optional.install_attempt(optional_attempt, config_diff)
    else {
        panic!("all optional sources missing must install resolved defaults");
    };
    assert_eq!(snapshot.generation, 1);
    assert_eq!(snapshot.source, None);

    let required_path = root.join("required.lua");
    let mut required = ConfigLifecycle::<EffectiveConfig, String>::new(
        discovery(),
        false,
        Some(required_path.clone()),
        EffectiveConfig::default(),
    );
    let required_attempt = required.attempt_reload(parse_config);
    assert_eq!(
        required_attempt.resolved,
        ResolvedConfigSource::File(ConfigSource {
            path: required_path.clone(),
            required: true,
        })
    );
    assert!(matches!(
        required_attempt.result.as_ref().unwrap_err().kind,
        ConfigSourceErrorKind::Io(std::io::ErrorKind::NotFound)
    ));
    let ConfigLifecycleEvent::Rejected { snapshot, .. } =
        required.install_attempt(required_attempt, config_diff)
    else {
        panic!("missing required source must reject the initial attempt");
    };
    assert_eq!(snapshot.generation, 0);
    assert_eq!(snapshot.source, None);
    assert_eq!(
        required.latest_selection(),
        &ResolvedConfigSource::File(ConfigSource {
            path: required_path,
            required: true,
        }),
        "failed selection remains observable so the app can watch it for recovery"
    );
}

#[test]
fn valid_reload_returns_new_snapshot_typed_diff_and_generation() {
    let root = TestDir::new("valid");
    let path = root.join("config.toml");
    fs::write(&path, "[font]\nfamily = 'first'\n").unwrap();
    let mut lifecycle = ConfigLifecycle::<EffectiveConfig, String>::new(
        discovery(),
        false,
        Some(path.clone()),
        EffectiveConfig::default(),
    );

    let first = lifecycle.install_attempt(lifecycle.attempt_reload(parse_config), config_diff);
    let ConfigLifecycleEvent::Applied { snapshot, diff } = first else {
        panic!("valid initial source must be applied");
    };
    assert_eq!(snapshot.generation, 1);
    assert_eq!(snapshot.source.as_ref(), Some(&path));
    assert_eq!(snapshot.config.font.family, "first");
    assert!(diff.font.unwrap().family);

    fs::write(&path, "[font]\nfamily = 'second'\n").unwrap();
    let second = lifecycle.install_attempt(lifecycle.attempt_reload(parse_config), config_diff);
    let ConfigLifecycleEvent::Applied { snapshot, diff } = second else {
        panic!("valid runtime source must be applied");
    };
    assert_eq!(snapshot.generation, 2);
    assert_eq!(snapshot.config.font.family, "second");
    assert!(diff.font.unwrap().family);
    assert!(diff.terminal.is_none());

    let unchanged = lifecycle.install_attempt(lifecycle.attempt_reload(parse_config), config_diff);
    let ConfigLifecycleEvent::Applied { snapshot, diff } = unchanged else {
        panic!("an unchanged but valid reload is still a successful reload");
    };
    assert_eq!(snapshot.generation, 3);
    assert!(diff.is_empty());
}

#[test]
fn single_install_api_keeps_generation_monotonic_across_repeated_successes() {
    let root = TestDir::new("monotonic-generation");
    let path = root.join("config.toml");
    fs::write(&path, "[font]\nfamily = 'stable'\n").unwrap();
    let mut lifecycle = ConfigLifecycle::<EffectiveConfig, String>::new(
        discovery(),
        false,
        Some(path),
        EffectiveConfig::default(),
    );

    let initial = lifecycle.install_attempt(lifecycle.attempt_reload(parse_config), config_diff);
    assert!(matches!(
        initial,
        ConfigLifecycleEvent::Applied {
            snapshot: rterm_config::ConfigSnapshot { generation: 1, .. },
            ..
        }
    ));

    let runtime = lifecycle.install_attempt(lifecycle.attempt_reload(parse_config), config_diff);
    assert!(matches!(
        runtime,
        ConfigLifecycleEvent::Applied {
            snapshot: rterm_config::ConfigSnapshot { generation: 2, .. },
            ..
        }
    ));

    let repeated_install =
        lifecycle.install_attempt(lifecycle.attempt_reload(parse_config), config_diff);
    let ConfigLifecycleEvent::Applied { snapshot, .. } = repeated_install else {
        panic!("a valid repeated install must still be applied");
    };
    assert_eq!(
        snapshot.generation, 3,
        "a public install operation must never move generation backward"
    );
}

#[test]
fn invalid_reload_retains_the_exact_last_known_good_arc_and_generation() {
    let root = TestDir::new("last-known-good");
    let path = root.join("config.toml");
    fs::write(&path, "[font]\nfamily = 'stable'\n").unwrap();
    let mut lifecycle = ConfigLifecycle::<EffectiveConfig, String>::new(
        discovery(),
        false,
        Some(path.clone()),
        EffectiveConfig::default(),
    );
    let initial = lifecycle.attempt_reload(parse_config);
    lifecycle.install_attempt(initial, config_diff);
    let last_known_good = lifecycle.snapshot();

    fs::write(&path, "[font\nfamily = 'broken'\n").unwrap();
    let invalid = lifecycle.attempt_reload(parse_config);
    let event = lifecycle.install_attempt(invalid, config_diff);
    let ConfigLifecycleEvent::Rejected {
        snapshot,
        diagnostic,
    } = event
    else {
        panic!("invalid runtime source must be rejected");
    };

    assert_eq!(snapshot.generation, last_known_good.generation);
    assert_eq!(snapshot.source, last_known_good.source);
    assert_eq!(snapshot.publication, last_known_good.publication);
    assert!(Arc::ptr_eq(&snapshot.config, &last_known_good.config));
    assert!(Arc::ptr_eq(
        &lifecycle.snapshot().config,
        &last_known_good.config
    ));
    assert_eq!(diagnostic.path, path);
    assert!(matches!(diagnostic.kind, ConfigSourceErrorKind::Strict(_)));
}

#[test]
fn fixed_window_debounce_coalesces_a_burst_without_extending_deadline() {
    let start = Instant::now();
    let window = Duration::from_millis(200);
    let mut debounce = FixedWindowDebouncer::new(window);

    let first_deadline = debounce.observe(SourceChange::Changed, start).unwrap();
    assert_eq!(first_deadline, start + window);
    assert_eq!(
        debounce
            .observe(SourceChange::Changed, start + Duration::from_millis(75))
            .unwrap(),
        first_deadline,
        "a fixed debounce window must not slide with every filesystem event"
    );
    assert!(
        debounce
            .observe(SourceChange::Ignored, start + Duration::from_millis(100))
            .is_none()
    );
    assert!(!debounce.take_ready(start + Duration::from_millis(199)));
    assert!(debounce.take_ready(first_deadline));
    assert!(!debounce.take_ready(first_deadline));
}

#[test]
fn debounce_duration_overflow_becomes_immediately_ready_without_panicking() {
    let now = Instant::now();
    let mut debounce = FixedWindowDebouncer::new(Duration::MAX);

    assert_eq!(debounce.observe(SourceChange::Changed, now), Some(now));
    assert!(debounce.take_ready(now));
}

#[test]
fn source_precedence_matches_explicit_environment_portable_home_and_xdg_order() {
    let root = TestDir::new("precedence");
    let explicit = root.join("explicit.toml");
    let environment = root.join("environment.toml");
    let portable = root.join("wezterm.lua");
    let home = root.join("home");
    let home_config = home.join(".wezterm.lua");
    let xdg = root.join("xdg");
    let xdg_config = xdg.join("wezterm/wezterm.lua");
    fs::create_dir_all(xdg_config.parent().unwrap()).unwrap();
    fs::create_dir_all(&home).unwrap();
    for (path, family) in [
        (&explicit, "explicit"),
        (&environment, "environment"),
        (&portable, "portable"),
        (&home_config, "home"),
        (&xdg_config, "xdg"),
    ] {
        fs::write(path, format!("[font]\nfamily = '{family}'\n")).unwrap();
    }
    let inputs = ConfigDiscoveryInputs {
        is_windows: true,
        current_exe: Some(root.join("rterm.exe")),
        home_dir: Some(home),
        xdg_config_home: Some(xdg),
        environment_config_file: Some(environment.clone()),
        ..discovery()
    };

    let explicit_lifecycle = ConfigLifecycle::<EffectiveConfig, String>::new(
        inputs.clone(),
        false,
        Some(explicit.clone()),
        EffectiveConfig::default(),
    );
    let attempt = explicit_lifecycle.attempt_reload(parse_config);
    assert_eq!(attempt.preferred, Some(explicit));
    assert_eq!(attempt.result.unwrap().font.family, "explicit");

    let environment_lifecycle = ConfigLifecycle::<EffectiveConfig, String>::new(
        inputs.clone(),
        false,
        None,
        EffectiveConfig::default(),
    );
    let attempt = environment_lifecycle.attempt_reload(parse_config);
    assert_eq!(attempt.preferred, Some(environment));
    assert_eq!(attempt.result.unwrap().font.family, "environment");

    let optional_inputs = ConfigDiscoveryInputs {
        environment_config_file: None,
        ..inputs
    };
    let portable_lifecycle = ConfigLifecycle::<EffectiveConfig, String>::new(
        optional_inputs.clone(),
        false,
        None,
        EffectiveConfig::default(),
    );
    assert_eq!(
        portable_lifecycle
            .attempt_reload(parse_config)
            .result
            .unwrap()
            .font
            .family,
        "portable"
    );
    fs::remove_file(portable).unwrap();
    let home_lifecycle = ConfigLifecycle::<EffectiveConfig, String>::new(
        optional_inputs,
        false,
        None,
        EffectiveConfig::default(),
    );
    assert_eq!(
        home_lifecycle
            .attempt_reload(parse_config)
            .result
            .unwrap()
            .font
            .family,
        "home"
    );
}

#[test]
fn skip_uses_cli_resolved_defaults_without_reading_any_source() {
    let root = TestDir::new("skip");
    let path = root.join("config.toml");
    fs::write(&path, "[font]\nfamily = 'file'\n").unwrap();
    let mut cli_resolved = EffectiveConfig::default();
    Arc::make_mut(&mut cli_resolved.font).family = "cli".to_owned();
    let lifecycle = ConfigLifecycle::<EffectiveConfig, String>::new(
        ConfigDiscoveryInputs {
            environment_config_file: Some(path),
            ..discovery()
        },
        true,
        None,
        cli_resolved,
    );

    let attempt = lifecycle.attempt_reload(|_| -> Result<EffectiveConfig, String> {
        panic!("disabled discovery must never invoke the source parser")
    });
    assert_eq!(attempt.resolved, ResolvedConfigSource::Disabled);
    assert_eq!(attempt.result.unwrap().font.family, "cli");
}

#[test]
fn utf8_and_parser_diagnostics_are_path_qualified() {
    let root = TestDir::new("diagnostics");
    let path = root.join("config.toml");
    fs::write(&path, [0xff, 0xfe]).unwrap();
    let lifecycle = ConfigLifecycle::<EffectiveConfig, String>::new(
        discovery(),
        false,
        Some(path.clone()),
        EffectiveConfig::default(),
    );

    let utf8 = lifecycle.attempt_reload(parse_config).result.unwrap_err();
    assert_eq!(utf8.path, path);
    assert!(matches!(utf8.kind, ConfigSourceErrorKind::InvalidUtf8));
    assert!(utf8.to_string().starts_with(&path.display().to_string()));

    fs::write(&path, "[font\nfamily = 'broken'\n").unwrap();
    let parser = lifecycle.attempt_reload(parse_config).result.unwrap_err();
    assert_eq!(parser.path, path);
    assert!(matches!(parser.kind, ConfigSourceErrorKind::Strict(_)));
    assert!(parser.to_string().starts_with(&path.display().to_string()));
}
