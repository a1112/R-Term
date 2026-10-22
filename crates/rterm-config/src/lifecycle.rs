use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::source::{
    ConfigDiscoveryInputs, ConfigLoadAttempt, ConfigSourceError, ConfigSourceErrorKind,
    DerivedConfigEnvironment, ResolvedConfigSource, load_source,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigSnapshot<T> {
    pub source: Option<PathBuf>,
    pub config: Arc<T>,
    pub generation: u64,
    pub publication: DerivedConfigEnvironment,
}

#[derive(Debug, Clone)]
pub enum ConfigLifecycleEvent<T, D, E> {
    Applied {
        snapshot: ConfigSnapshot<T>,
        diff: D,
    },
    Rejected {
        snapshot: ConfigSnapshot<T>,
        diagnostic: ConfigSourceError<E>,
    },
}

#[derive(Clone)]
pub struct ConfigLifecycle<T, E> {
    inputs: ConfigDiscoveryInputs,
    skip: bool,
    explicit: Option<PathBuf>,
    defaults: T,
    snapshot: ConfigSnapshot<T>,
    latest_diagnostic: Option<ConfigSourceError<E>>,
    latest_selection: ResolvedConfigSource,
}

impl<T: Clone, E: Clone + std::fmt::Display> ConfigLifecycle<T, E> {
    #[must_use]
    pub fn new(
        inputs: ConfigDiscoveryInputs,
        skip: bool,
        explicit: Option<PathBuf>,
        defaults: T,
    ) -> Self {
        Self::new_with_initial(inputs, skip, explicit, defaults.clone(), defaults)
    }

    #[must_use]
    pub fn new_with_initial(
        inputs: ConfigDiscoveryInputs,
        skip: bool,
        explicit: Option<PathBuf>,
        initial: T,
        defaults: T,
    ) -> Self {
        Self {
            inputs,
            skip,
            explicit,
            snapshot: ConfigSnapshot {
                source: None,
                config: Arc::new(initial),
                generation: 0,
                publication: DerivedConfigEnvironment::default(),
            },
            defaults,
            latest_diagnostic: None,
            latest_selection: ResolvedConfigSource::Defaults,
        }
    }

    #[must_use]
    pub fn attempt_reload(&self, parser: impl Fn(&str) -> Result<T, E>) -> ConfigLoadAttempt<T, E> {
        if self.skip {
            return ConfigLoadAttempt {
                preferred: None,
                resolved: ResolvedConfigSource::Disabled,
                result: Ok(self.defaults.clone()),
                publication: DerivedConfigEnvironment::default(),
            };
        }

        for source in self.candidate_sources() {
            let path = source.path.clone();
            let result = load_source(&path, &parser);
            if !source.required
                && matches!(
                    result,
                    Err(ConfigSourceError {
                        kind: ConfigSourceErrorKind::Io(std::io::ErrorKind::NotFound),
                        ..
                    })
                )
            {
                continue;
            }
            let (result, publication) = match result {
                Ok(config) => match DerivedConfigEnvironment::for_file(&path) {
                    Ok(publication) => (Ok(config), publication),
                    Err(error) => (Err(error), DerivedConfigEnvironment::default()),
                },
                Err(error) => (Err(error), DerivedConfigEnvironment::default()),
            };
            return ConfigLoadAttempt {
                preferred: Some(path),
                resolved: ResolvedConfigSource::File(source),
                result,
                publication,
            };
        }

        ConfigLoadAttempt {
            preferred: None,
            resolved: ResolvedConfigSource::Defaults,
            result: Ok(self.defaults.clone()),
            publication: DerivedConfigEnvironment::default(),
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> ConfigSnapshot<T> {
        self.snapshot.clone()
    }

    #[must_use]
    pub const fn snapshot_ref(&self) -> &ConfigSnapshot<T> {
        &self.snapshot
    }

    #[must_use]
    pub const fn latest_diagnostic(&self) -> Option<&ConfigSourceError<E>> {
        self.latest_diagnostic.as_ref()
    }

    #[must_use]
    pub const fn latest_selection(&self) -> &ResolvedConfigSource {
        &self.latest_selection
    }

    #[must_use]
    pub const fn inputs(&self) -> &ConfigDiscoveryInputs {
        &self.inputs
    }

    #[must_use]
    pub fn candidate_sources(&self) -> Vec<crate::ConfigSource> {
        self.inputs.candidate_sources(self.explicit.as_deref())
    }

    /// Installs a reload attempt and advances the generation on success.
    ///
    /// # Panics
    ///
    /// Panics if a successful install would advance a configuration generation
    /// that has already reached [`u64::MAX`].
    pub fn install_attempt<D>(
        &mut self,
        attempt: ConfigLoadAttempt<T, E>,
        diff: impl FnOnce(&T, &T) -> D,
    ) -> ConfigLifecycleEvent<T, D, E> {
        self.latest_selection = attempt.resolved.clone();
        match attempt.result {
            Ok(config) => {
                let generation = self
                    .snapshot
                    .generation
                    .checked_add(1)
                    .expect("configuration generation overflowed");
                let diff = diff(&self.snapshot.config, &config);
                self.snapshot = ConfigSnapshot {
                    source: selected_path(&attempt.resolved),
                    config: Arc::new(config),
                    generation,
                    publication: attempt.publication,
                };
                self.latest_diagnostic = None;
                ConfigLifecycleEvent::Applied {
                    snapshot: self.snapshot.clone(),
                    diff,
                }
            }
            Err(diagnostic) => {
                self.latest_diagnostic = Some(diagnostic.clone());
                ConfigLifecycleEvent::Rejected {
                    snapshot: self.snapshot.clone(),
                    diagnostic,
                }
            }
        }
    }
}

fn selected_path(source: &ResolvedConfigSource) -> Option<PathBuf> {
    match source {
        ResolvedConfigSource::File(source) => Some(source.path.clone()),
        ResolvedConfigSource::Disabled | ResolvedConfigSource::Defaults => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceChange {
    Changed,
    Ignored,
}

#[derive(Debug, Clone)]
pub struct FixedWindowDebouncer {
    window: Duration,
    deadline: Option<Instant>,
}

impl FixedWindowDebouncer {
    #[must_use]
    pub const fn new(window: Duration) -> Self {
        Self {
            window,
            deadline: None,
        }
    }

    pub fn observe(&mut self, change: SourceChange, now: Instant) -> Option<Instant> {
        if change == SourceChange::Ignored {
            return None;
        }
        let window = self.window;
        let deadline = *self
            .deadline
            .get_or_insert_with(|| now.checked_add(window).unwrap_or(now));
        Some(deadline)
    }

    #[must_use]
    pub fn remaining(&self, now: Instant) -> Option<Duration> {
        self.deadline
            .map(|deadline| deadline.saturating_duration_since(now))
    }

    pub fn take_ready(&mut self, now: Instant) -> bool {
        if self.deadline.is_none_or(|deadline| now < deadline) {
            return false;
        }
        self.deadline = None;
        true
    }
}
