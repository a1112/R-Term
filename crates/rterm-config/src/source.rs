use std::{
    collections::BTreeMap,
    ffi::OsString,
    fmt, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiscoveryInputs {
    pub is_windows: bool,
    pub is_unix: bool,
    pub current_exe: Option<PathBuf>,
    pub home_dir: Option<PathBuf>,
    pub xdg_config_home: Option<PathBuf>,
    pub xdg_config_dirs: Vec<PathBuf>,
    pub environment_config_file: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigEnvironmentSnapshot {
    pub home: Option<OsString>,
    pub user_profile: Option<OsString>,
    pub home_drive: Option<OsString>,
    pub home_path: Option<OsString>,
    pub xdg_config_home: Option<OsString>,
    pub xdg_config_dirs: Option<OsString>,
    pub wezterm_config_file: Option<OsString>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSource {
    pub path: PathBuf,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedConfigSource {
    Disabled,
    Defaults,
    File(ConfigSource),
}

#[derive(Debug, Clone)]
pub enum ConfigSourceErrorKind<E> {
    Io(std::io::ErrorKind),
    InvalidUtf8,
    NonUnicodePath,
    Strict(E),
}

#[derive(Debug, Clone)]
pub struct ConfigSourceError<E> {
    pub path: PathBuf,
    pub kind: ConfigSourceErrorKind<E>,
    pub detail: String,
}

impl<E: fmt::Display> fmt::Display for ConfigSourceError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: ", self.path.display())?;
        match &self.kind {
            ConfigSourceErrorKind::Io(kind) => {
                let kind = match kind {
                    std::io::ErrorKind::NotFound => "not found".to_owned(),
                    kind => kind.to_string(),
                };
                write!(formatter, "I/O error: {kind}: {}", self.detail)
            }
            ConfigSourceErrorKind::InvalidUtf8 => formatter.write_str("invalid UTF-8"),
            ConfigSourceErrorKind::NonUnicodePath => {
                formatter.write_str("config source path cannot be published losslessly")
            }
            ConfigSourceErrorKind::Strict(error) => error.fmt(formatter),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for ConfigSourceError<E> {}

#[derive(Debug, Clone)]
pub struct ConfigLoadAttempt<T, E> {
    pub preferred: Option<PathBuf>,
    pub resolved: ResolvedConfigSource,
    pub result: Result<T, ConfigSourceError<E>>,
    pub publication: DerivedConfigEnvironment,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DerivedConfigEnvironment {
    variables: BTreeMap<String, String>,
}

impl DerivedConfigEnvironment {
    #[must_use]
    pub fn variables(&self) -> &BTreeMap<String, String> {
        &self.variables
    }

    pub(crate) fn for_file<E>(path: &Path) -> Result<Self, ConfigSourceError<E>> {
        fn non_unicode_path<E>(path: &Path) -> ConfigSourceError<E> {
            ConfigSourceError {
                path: path.to_path_buf(),
                kind: ConfigSourceErrorKind::NonUnicodePath,
                detail: "config source path is not valid Unicode".to_owned(),
            }
        }

        let file = path.to_str().ok_or_else(|| non_unicode_path(path))?;
        let mut variables = BTreeMap::new();
        variables.insert("WEZTERM_CONFIG_FILE".to_owned(), file.to_owned());
        if let Some(parent) = path.parent() {
            let parent = parent.to_str().ok_or_else(|| non_unicode_path(path))?;
            variables.insert("WEZTERM_CONFIG_DIR".to_owned(), parent.to_owned());
        }
        Ok(Self { variables })
    }
}

impl ConfigDiscoveryInputs {
    #[must_use]
    pub fn capture_current_process() -> Self {
        Self::from_environment_snapshot(
            cfg!(windows),
            cfg!(unix),
            std::env::current_exe().ok(),
            ConfigEnvironmentSnapshot {
                home: std::env::var_os("HOME"),
                user_profile: std::env::var_os("USERPROFILE"),
                home_drive: std::env::var_os("HOMEDRIVE"),
                home_path: std::env::var_os("HOMEPATH"),
                xdg_config_home: std::env::var_os("XDG_CONFIG_HOME"),
                xdg_config_dirs: std::env::var_os("XDG_CONFIG_DIRS"),
                wezterm_config_file: std::env::var_os("WEZTERM_CONFIG_FILE"),
            },
        )
    }

    #[must_use]
    pub fn from_environment_snapshot(
        is_windows: bool,
        is_unix: bool,
        current_exe: Option<PathBuf>,
        environment: ConfigEnvironmentSnapshot,
    ) -> Self {
        fn non_empty_path(value: Option<OsString>) -> Option<PathBuf> {
            value.filter(|value| !value.is_empty()).map(PathBuf::from)
        }

        let home_dir = if is_windows {
            non_empty_path(environment.user_profile)
                .or_else(|| {
                    let drive = environment.home_drive.filter(|value| !value.is_empty())?;
                    let path = environment.home_path.filter(|value| !value.is_empty())?;
                    let mut home = drive;
                    home.push(path);
                    Some(PathBuf::from(home))
                })
                .or_else(|| non_empty_path(environment.home))
        } else {
            non_empty_path(environment.home)
        };
        let xdg_config_dirs = environment
            .xdg_config_dirs
            .map(|value| std::env::split_paths(&value).collect())
            .unwrap_or_default();
        Self {
            is_windows,
            is_unix,
            current_exe,
            home_dir,
            xdg_config_home: environment.xdg_config_home.map(PathBuf::from),
            xdg_config_dirs,
            environment_config_file: environment.wezterm_config_file.map(PathBuf::from),
        }
    }

    #[must_use]
    pub fn candidate_sources(&self, explicit: Option<&Path>) -> Vec<ConfigSource> {
        let mut candidates = Vec::new();
        if let Some(path) = explicit {
            candidates.push(ConfigSource {
                path: path.to_path_buf(),
                required: true,
            });
            return candidates;
        }
        if let Some(path) = &self.environment_config_file {
            candidates.push(ConfigSource {
                path: path.clone(),
                required: true,
            });
            return candidates;
        }
        if self.is_windows
            && let Some(path) = self
                .current_exe
                .as_deref()
                .and_then(Path::parent)
                .map(|parent| parent.join("wezterm.lua"))
        {
            candidates.push(ConfigSource {
                path,
                required: false,
            });
        }
        if let Some(path) = self.home_dir.as_ref().map(|home| home.join(".wezterm.lua")) {
            candidates.push(ConfigSource {
                path,
                required: false,
            });
        }
        if let Some(path) = self
            .xdg_config_home
            .as_ref()
            .map(|dir| dir.join("wezterm").join("wezterm.lua"))
        {
            candidates.push(ConfigSource {
                path,
                required: false,
            });
        }
        if self.xdg_config_home.is_none()
            && let Some(path) = self
                .home_dir
                .as_ref()
                .map(|home| home.join(".config").join("wezterm").join("wezterm.lua"))
        {
            candidates.push(ConfigSource {
                path,
                required: false,
            });
        }
        if self.is_unix {
            candidates.extend(self.xdg_config_dirs.iter().map(|dir| ConfigSource {
                path: dir.join("wezterm").join("wezterm.lua"),
                required: false,
            }));
        }

        candidates
    }
}

pub(crate) fn load_source<T, E: fmt::Display>(
    path: &Path,
    parser: &impl Fn(&str) -> Result<T, E>,
) -> Result<T, ConfigSourceError<E>> {
    let bytes = fs::read(path).map_err(|error| ConfigSourceError {
        path: path.to_path_buf(),
        kind: ConfigSourceErrorKind::Io(error.kind()),
        detail: error.to_string(),
    })?;
    let source = std::str::from_utf8(&bytes).map_err(|_| ConfigSourceError {
        path: path.to_path_buf(),
        kind: ConfigSourceErrorKind::InvalidUtf8,
        detail: "input is not valid UTF-8".to_owned(),
    })?;
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    parser(source).map_err(|error| {
        let detail = error.to_string();
        ConfigSourceError {
            path: path.to_path_buf(),
            kind: ConfigSourceErrorKind::Strict(error),
            detail,
        }
    })
}
