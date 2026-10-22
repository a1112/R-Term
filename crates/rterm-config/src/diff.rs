use crate::EffectiveConfig;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConfigDiff {
    pub font: Option<FontConfigDiff>,
    pub terminal: Option<TerminalConfigDiff>,
    pub input: Option<InputConfigDiff>,
    pub window: Option<WindowConfigDiff>,
    pub render: Option<RenderConfigDiff>,
    pub domain: Option<DomainConfigDiff>,
    pub lifecycle: Option<LifecycleConfigDiff>,
}

impl ConfigDiff {
    #[must_use]
    pub fn between(before: &EffectiveConfig, after: &EffectiveConfig) -> Self {
        Self {
            font: FontConfigDiff::between(before, after),
            terminal: TerminalConfigDiff::between(before, after),
            input: InputConfigDiff::between(before, after),
            window: WindowConfigDiff::between(before, after),
            render: RenderConfigDiff::between(before, after),
            domain: DomainConfigDiff::between(before, after),
            lifecycle: LifecycleConfigDiff::between(before, after),
        }
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.font.is_none()
            && self.terminal.is_none()
            && self.input.is_none()
            && self.window.is_none()
            && self.render.is_none()
            && self.domain.is_none()
            && self.lifecycle.is_none()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontConfigDiff {
    pub family: bool,
    pub size_milli_points: bool,
}

impl FontConfigDiff {
    fn between(before: &EffectiveConfig, after: &EffectiveConfig) -> Option<Self> {
        present(Self {
            family: before.font.family != after.font.family,
            size_milli_points: before.font.size_milli_points != after.font.size_milli_points,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalConfigDiff {
    pub scrollback_lines: bool,
    pub term: bool,
}

impl TerminalConfigDiff {
    fn between(before: &EffectiveConfig, after: &EffectiveConfig) -> Option<Self> {
        present(Self {
            scrollback_lines: before.terminal.scrollback_lines != after.terminal.scrollback_lines,
            term: before.terminal.term != after.terminal.term,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputConfigDiff {
    pub copy_on_select: bool,
}

impl InputConfigDiff {
    fn between(before: &EffectiveConfig, after: &EffectiveConfig) -> Option<Self> {
        present(Self {
            copy_on_select: before.input.copy_on_select != after.input.copy_on_select,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowConfigDiff {
    pub title: bool,
    pub integrated_titlebar: bool,
}

impl WindowConfigDiff {
    fn between(before: &EffectiveConfig, after: &EffectiveConfig) -> Option<Self> {
        present(Self {
            title: before.window.title != after.window.title,
            integrated_titlebar: before.window.integrated_titlebar
                != after.window.integrated_titlebar,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderConfigDiff {
    pub max_fps: bool,
}

impl RenderConfigDiff {
    fn between(before: &EffectiveConfig, after: &EffectiveConfig) -> Option<Self> {
        present(Self {
            max_fps: before.render.max_fps != after.render.max_fps,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DomainConfigDiff {
    pub default_domain: bool,
}

impl DomainConfigDiff {
    fn between(before: &EffectiveConfig, after: &EffectiveConfig) -> Option<Self> {
        present(Self {
            default_domain: before.domain.default_domain != after.domain.default_domain,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LifecycleConfigDiff {
    pub reload_on_change: bool,
}

impl LifecycleConfigDiff {
    fn between(before: &EffectiveConfig, after: &EffectiveConfig) -> Option<Self> {
        present(Self {
            reload_on_change: before.lifecycle.reload_on_change != after.lifecycle.reload_on_change,
        })
    }
}

trait Changed {
    fn changed(self) -> bool;
}

impl Changed for FontConfigDiff {
    fn changed(self) -> bool {
        self.family || self.size_milli_points
    }
}

impl Changed for TerminalConfigDiff {
    fn changed(self) -> bool {
        self.scrollback_lines || self.term
    }
}

impl Changed for InputConfigDiff {
    fn changed(self) -> bool {
        self.copy_on_select
    }
}

impl Changed for WindowConfigDiff {
    fn changed(self) -> bool {
        self.title || self.integrated_titlebar
    }
}

impl Changed for RenderConfigDiff {
    fn changed(self) -> bool {
        self.max_fps
    }
}

impl Changed for DomainConfigDiff {
    fn changed(self) -> bool {
        self.default_domain
    }
}

impl Changed for LifecycleConfigDiff {
    fn changed(self) -> bool {
        self.reload_on_change
    }
}

fn present<T: Changed + Copy>(diff: T) -> Option<T> {
    diff.changed().then_some(diff)
}
