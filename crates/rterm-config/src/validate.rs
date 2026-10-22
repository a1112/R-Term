use std::sync::Arc;

use crate::{ConfigDiagnostic, ConfigDiff, EffectiveConfig};

const MAX_FONT_SIZE_MILLI_POINTS: u32 = 512_000;
const MAX_SCROLLBACK_LINES: usize = 10_000_000;
const MAX_FPS: u16 = 1_000;

#[derive(Debug)]
pub struct ConfigUpdate {
    pub snapshot: Arc<EffectiveConfig>,
    pub diff: ConfigDiff,
}

#[derive(Debug)]
pub struct ValidatedConfigStore {
    current: Arc<EffectiveConfig>,
}

impl ValidatedConfigStore {
    /// Creates a store from a valid initial snapshot.
    ///
    /// # Errors
    ///
    /// Returns all path-qualified validation diagnostics when `initial` is
    /// invalid.
    pub fn new(initial: EffectiveConfig) -> Result<Self, Vec<ConfigDiagnostic>> {
        validate(&initial)?;
        Ok(Self {
            current: Arc::new(initial),
        })
    }

    #[must_use]
    pub fn current(&self) -> Arc<EffectiveConfig> {
        Arc::clone(&self.current)
    }

    /// Validates and atomically replaces the current snapshot.
    ///
    /// # Errors
    ///
    /// Returns diagnostics without changing the last-known-good snapshot when
    /// `candidate` is invalid.
    pub fn replace(
        &mut self,
        candidate: Arc<EffectiveConfig>,
    ) -> Result<ConfigUpdate, Vec<ConfigDiagnostic>> {
        validate(&candidate)?;
        let diff = ConfigDiff::between(&self.current, &candidate);
        if diff.is_empty() {
            return Ok(ConfigUpdate {
                snapshot: Arc::clone(&self.current),
                diff,
            });
        }
        let mut candidate = Arc::unwrap_or_clone(candidate);
        candidate.reuse_equal_subtrees_from(&self.current);
        self.current = Arc::new(candidate);
        Ok(ConfigUpdate {
            snapshot: Arc::clone(&self.current),
            diff,
        })
    }
}

/// Validates a complete configuration snapshot.
///
/// # Errors
///
/// Returns every deterministic, path-qualified diagnostic found in `config`.
pub fn validate(config: &EffectiveConfig) -> Result<(), Vec<ConfigDiagnostic>> {
    let mut diagnostics = Vec::new();
    if config.font.family.trim().is_empty() {
        diagnostics.push(ConfigDiagnostic::error(
            "font.family",
            "empty",
            "font family must not be empty",
        ));
    }
    if !(1_000..=MAX_FONT_SIZE_MILLI_POINTS).contains(&config.font.size_milli_points) {
        diagnostics.push(ConfigDiagnostic::error(
            "font.size_milli_points",
            "out_of_range",
            format!("font size must be between 1000 and {MAX_FONT_SIZE_MILLI_POINTS} milli-points"),
        ));
    }
    if config.terminal.scrollback_lines > MAX_SCROLLBACK_LINES {
        diagnostics.push(ConfigDiagnostic::error(
            "terminal.scrollback_lines",
            "out_of_range",
            format!("scrollback lines must not exceed {MAX_SCROLLBACK_LINES}"),
        ));
    }
    validate_nonempty_without_nul("terminal.term", &config.terminal.term, &mut diagnostics);
    if config.window.title.contains('\0') {
        diagnostics.push(ConfigDiagnostic::error(
            "window.title",
            "contains_nul",
            "window title must not contain NUL",
        ));
    }
    if !(1..=MAX_FPS).contains(&config.render.max_fps) {
        diagnostics.push(ConfigDiagnostic::error(
            "render.max_fps",
            "out_of_range",
            format!("maximum frame rate must be between 1 and {MAX_FPS}"),
        ));
    }
    if let Some(domain) = &config.domain.default_domain {
        validate_nonempty_without_nul("domain.default_domain", domain, &mut diagnostics);
    }

    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

fn validate_nonempty_without_nul(
    path: &'static str,
    value: &str,
    diagnostics: &mut Vec<ConfigDiagnostic>,
) {
    if value.trim().is_empty() {
        diagnostics.push(ConfigDiagnostic::error(
            path,
            "empty",
            format!("{path} must not be empty"),
        ));
    } else if value.contains('\0') {
        diagnostics.push(ConfigDiagnostic::error(
            path,
            "contains_nul",
            format!("{path} must not contain NUL"),
        ));
    }
}
