use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    io,
    path::Path,
    process::Command,
};

/// An automatically removed temporary home directory and its child environment.
pub struct TempHome {
    directory: tempfile::TempDir,
    environment: BTreeMap<OsString, OsString>,
}

impl TempHome {
    /// Creates a fresh isolated home directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the operating system cannot create the directory.
    pub fn new() -> io::Result<Self> {
        let directory = tempfile::Builder::new()
            .prefix("rterm-e2e-home-")
            .tempdir()?;
        let value = directory.path().as_os_str().to_os_string();
        let environment = [
            (OsString::from("HOME"), value.clone()),
            (OsString::from("USERPROFILE"), value),
        ]
        .into_iter()
        .collect();
        Ok(Self {
            directory,
            environment,
        })
    }

    /// Returns the temporary home path.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    /// Returns the environment settings to apply to a child command.
    #[must_use]
    pub fn environment(&self) -> &BTreeMap<OsString, OsString> {
        &self.environment
    }

    /// Applies the isolated home environment to `command`.
    pub fn apply_to(&self, command: &mut Command) {
        command.envs(&self.environment);
    }

    /// Returns one configured environment value by its platform string key.
    #[must_use]
    pub fn environment_value(&self, key: &OsStr) -> Option<&OsStr> {
        self.environment.get(key).map(OsString::as_os_str)
    }
}
