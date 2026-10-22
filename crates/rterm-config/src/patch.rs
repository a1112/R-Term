use std::sync::Arc;

use crate::EffectiveConfig;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Patch<T> {
    #[default]
    Inherit,
    Clear,
    Set(T),
}

impl<T: Clone> Patch<T> {
    fn apply_to(&self, value: &mut T, schema_default: &T) {
        match self {
            Self::Inherit => {}
            Self::Clear => value.clone_from(schema_default),
            Self::Set(replacement) => value.clone_from(replacement),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfigPatch {
    pub font: FontConfigPatch,
    pub terminal: TerminalConfigPatch,
    pub input: InputConfigPatch,
    pub window: WindowConfigPatch,
    pub render: RenderConfigPatch,
    pub domain: DomainConfigPatch,
    pub lifecycle: LifecycleConfigPatch,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FontConfigPatch {
    pub family: Patch<String>,
    pub size_milli_points: Patch<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TerminalConfigPatch {
    pub scrollback_lines: Patch<usize>,
    pub term: Patch<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InputConfigPatch {
    pub copy_on_select: Patch<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WindowConfigPatch {
    pub title: Patch<String>,
    pub integrated_titlebar: Patch<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenderConfigPatch {
    pub max_fps: Patch<u16>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DomainConfigPatch {
    pub default_domain: Patch<Option<String>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LifecycleConfigPatch {
    pub reload_on_change: Patch<bool>,
}

pub fn resolve_layers<'a>(
    layers: impl IntoIterator<Item = &'a ConfigPatch>,
) -> Arc<EffectiveConfig> {
    let defaults = EffectiveConfig::default();
    let mut resolved = defaults.clone();
    for layer in layers {
        layer.apply_to(&mut resolved, &defaults);
    }
    Arc::new(resolved)
}

impl ConfigPatch {
    fn apply_to(&self, config: &mut EffectiveConfig, defaults: &EffectiveConfig) {
        let font = Arc::make_mut(&mut config.font);
        self.font
            .family
            .apply_to(&mut font.family, &defaults.font.family);
        self.font.size_milli_points.apply_to(
            &mut font.size_milli_points,
            &defaults.font.size_milli_points,
        );
        let terminal = Arc::make_mut(&mut config.terminal);
        self.terminal.scrollback_lines.apply_to(
            &mut terminal.scrollback_lines,
            &defaults.terminal.scrollback_lines,
        );
        self.terminal
            .term
            .apply_to(&mut terminal.term, &defaults.terminal.term);
        let input = Arc::make_mut(&mut config.input);
        self.input
            .copy_on_select
            .apply_to(&mut input.copy_on_select, &defaults.input.copy_on_select);
        let window = Arc::make_mut(&mut config.window);
        self.window
            .title
            .apply_to(&mut window.title, &defaults.window.title);
        self.window.integrated_titlebar.apply_to(
            &mut window.integrated_titlebar,
            &defaults.window.integrated_titlebar,
        );
        let render = Arc::make_mut(&mut config.render);
        self.render
            .max_fps
            .apply_to(&mut render.max_fps, &defaults.render.max_fps);
        let domain = Arc::make_mut(&mut config.domain);
        self.domain
            .default_domain
            .apply_to(&mut domain.default_domain, &defaults.domain.default_domain);
        let lifecycle = Arc::make_mut(&mut config.lifecycle);
        self.lifecycle.reload_on_change.apply_to(
            &mut lifecycle.reload_on_change,
            &defaults.lifecycle.reload_on_change,
        );
    }
}
