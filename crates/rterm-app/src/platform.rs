use std::{ffi::OsString, path::PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HostPlatform {
    Windows,
    Macos,
    Unix,
}

impl HostPlatform {
    const fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else {
            Self::Unix
        }
    }
}

#[derive(Debug, Default)]
struct StateDirectoryEnvironment {
    local_app_data: Option<OsString>,
    app_data: Option<OsString>,
    xdg_state_home: Option<OsString>,
    home: Option<OsString>,
}

impl StateDirectoryEnvironment {
    fn capture() -> Self {
        Self {
            local_app_data: std::env::var_os("LOCALAPPDATA"),
            app_data: std::env::var_os("APPDATA"),
            xdg_state_home: std::env::var_os("XDG_STATE_HOME"),
            home: std::env::var_os("HOME"),
        }
    }
}

/// Returns the platform-native directory for mutable, user-scoped R-Term state.
///
/// An explicit `XDG_STATE_HOME` remains an override on Unix and macOS so CLI
/// users and hermetic tests can relocate state. Finder-launched macOS apps do
/// not normally receive that variable, so their default follows the native
/// `~/Library/Application Support` convention.
pub(crate) fn state_dir() -> Option<PathBuf> {
    state_dir_from(
        HostPlatform::current(),
        &StateDirectoryEnvironment::capture(),
    )
}

fn state_dir_from(
    platform: HostPlatform,
    environment: &StateDirectoryEnvironment,
) -> Option<PathBuf> {
    match platform {
        HostPlatform::Windows => non_empty_path(environment.local_app_data.as_ref())
            .or_else(|| non_empty_path(environment.app_data.as_ref()))
            .map(|path| path.join("R-Term")),
        HostPlatform::Macos => non_empty_path(environment.xdg_state_home.as_ref())
            .map(|path| path.join("rterm"))
            .or_else(|| {
                non_empty_path(environment.home.as_ref()).map(|path| {
                    path.join("Library")
                        .join("Application Support")
                        .join("R-Term")
                })
            }),
        HostPlatform::Unix => non_empty_path(environment.xdg_state_home.as_ref())
            .or_else(|| {
                non_empty_path(environment.home.as_ref())
                    .map(|path| path.join(".local").join("state"))
            })
            .map(|path| path.join("rterm")),
    }
}

fn non_empty_path(value: Option<&OsString>) -> Option<PathBuf> {
    value.filter(|value| !value.is_empty()).map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::{HostPlatform, StateDirectoryEnvironment, state_dir_from};
    use std::{ffi::OsString, path::PathBuf};

    #[test]
    fn macos_uses_application_support_without_xdg_override() {
        let environment = StateDirectoryEnvironment {
            home: Some(OsString::from("/Users/alice")),
            ..StateDirectoryEnvironment::default()
        };

        assert_eq!(
            state_dir_from(HostPlatform::Macos, &environment),
            Some(PathBuf::from(
                "/Users/alice/Library/Application Support/R-Term"
            ))
        );
    }

    #[test]
    fn macos_honors_explicit_xdg_state_home() {
        let environment = StateDirectoryEnvironment {
            xdg_state_home: Some(OsString::from("/tmp/state")),
            home: Some(OsString::from("/Users/alice")),
            ..StateDirectoryEnvironment::default()
        };

        assert_eq!(
            state_dir_from(HostPlatform::Macos, &environment),
            Some(PathBuf::from("/tmp/state/rterm"))
        );
    }

    #[test]
    fn unix_keeps_xdg_compatible_default() {
        let environment = StateDirectoryEnvironment {
            home: Some(OsString::from("/home/alice")),
            ..StateDirectoryEnvironment::default()
        };

        assert_eq!(
            state_dir_from(HostPlatform::Unix, &environment),
            Some(PathBuf::from("/home/alice/.local/state/rterm"))
        );
    }

    #[test]
    fn windows_prefers_local_app_data() {
        let environment = StateDirectoryEnvironment {
            local_app_data: Some(OsString::from(r"C:\Users\alice\AppData\Local")),
            app_data: Some(OsString::from(r"C:\Users\alice\AppData\Roaming")),
            ..StateDirectoryEnvironment::default()
        };

        assert_eq!(
            state_dir_from(HostPlatform::Windows, &environment),
            Some(PathBuf::from(r"C:\Users\alice\AppData\Local").join("R-Term"))
        );
    }

    #[test]
    fn empty_environment_values_are_not_paths() {
        let environment = StateDirectoryEnvironment {
            xdg_state_home: Some(OsString::new()),
            home: Some(OsString::new()),
            ..StateDirectoryEnvironment::default()
        };

        assert_eq!(state_dir_from(HostPlatform::Macos, &environment), None);
    }
}
