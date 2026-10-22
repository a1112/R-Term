use std::sync::Arc;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EffectiveConfig {
    pub font: Arc<FontConfig>,
    pub terminal: Arc<TerminalConfig>,
    pub input: Arc<InputConfig>,
    pub window: Arc<WindowConfig>,
    pub render: Arc<RenderConfig>,
    pub domain: Arc<DomainConfig>,
    pub lifecycle: Arc<LifecycleConfig>,
}

impl EffectiveConfig {
    /// Reuses immutable domain subtrees that are equal to the previous
    /// snapshot, preserving allocation identity across reloads.
    pub fn reuse_equal_subtrees_from(&mut self, previous: &Self) {
        reuse_if_equal(&mut self.font, &previous.font);
        reuse_if_equal(&mut self.terminal, &previous.terminal);
        reuse_if_equal(&mut self.input, &previous.input);
        reuse_if_equal(&mut self.window, &previous.window);
        reuse_if_equal(&mut self.render, &previous.render);
        reuse_if_equal(&mut self.domain, &previous.domain);
        reuse_if_equal(&mut self.lifecycle, &previous.lifecycle);
    }
}

fn reuse_if_equal<T: PartialEq>(candidate: &mut Arc<T>, previous: &Arc<T>) {
    if candidate == previous {
        candidate.clone_from(previous);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontConfig {
    pub family: String,
    pub size_milli_points: u32,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "monospace".to_owned(),
            size_milli_points: 12_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalConfig {
    pub scrollback_lines: usize,
    pub term: String,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            scrollback_lines: 3_500,
            term: "xterm-256color".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InputConfig {
    pub copy_on_select: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowConfig {
    pub title: String,
    pub integrated_titlebar: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "R-Term".to_owned(),
            integrated_titlebar: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderConfig {
    pub max_fps: u16,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self { max_fps: 60 }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DomainConfig {
    pub default_domain: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleConfig {
    pub reload_on_change: bool,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            reload_on_change: true,
        }
    }
}
