use std::sync::Arc;

use super::{
    LEGACY_COLOR_SCHEME_CURSOR_BG_COLOR, NativeConfigSnapshot, NativeResolvedPalette,
    NativeWindowConfigPatch, NativeWindowConfigPatchValues, native_split_ansi_palette,
    native_tab_bar_item_colors_with_overrides,
};

pub(super) fn native_resolved_palette_with_overrides(
    base: &NativeResolvedPalette,
    overrides: &NativeConfigSnapshot,
) -> NativeResolvedPalette {
    let (ansi, brights) = overrides
        .ansi_palette
        .map_or((base.ansi, base.brights), |palette| {
            native_split_ansi_palette(palette)
        });

    NativeResolvedPalette {
        foreground: overrides.foreground_color.unwrap_or(base.foreground),
        background: overrides.background_color.unwrap_or(base.background),
        cursor_fg: overrides.cursor_fg_color.or(base.cursor_fg),
        cursor_bg: overrides.cursor_bg_color.unwrap_or(base.cursor_bg),
        cursor_border: overrides.cursor_border_color.or(base.cursor_border),
        selection_fg: overrides.selection_fg_color.or(base.selection_fg),
        selection_bg: overrides.selection_bg_color.or(base.selection_bg),
        ansi,
        brights,
        indexed: overrides.indexed_palette.unwrap_or(base.indexed),
        tab_bar_background: overrides
            .tab_bar_background_color
            .or(base.tab_bar_background),
        tab_bar_inactive_tab_edge: overrides
            .tab_bar_inactive_tab_edge_color
            .or(base.tab_bar_inactive_tab_edge),
        tab_bar_active_tab: native_tab_bar_item_colors_with_overrides(
            base.tab_bar_active_tab,
            overrides.tab_bar_active_tab_colors,
        ),
        tab_bar_inactive_tab: native_tab_bar_item_colors_with_overrides(
            base.tab_bar_inactive_tab,
            overrides.tab_bar_inactive_tab_colors,
        ),
        tab_bar_inactive_tab_hover: native_tab_bar_item_colors_with_overrides(
            base.tab_bar_inactive_tab_hover,
            overrides.tab_bar_inactive_tab_hover_colors,
        ),
        tab_bar_new_tab: native_tab_bar_item_colors_with_overrides(
            base.tab_bar_new_tab,
            overrides.tab_bar_new_tab_colors,
        ),
        tab_bar_new_tab_hover: native_tab_bar_item_colors_with_overrides(
            base.tab_bar_new_tab_hover,
            overrides.tab_bar_new_tab_hover_colors,
        ),
        scrollbar_thumb: overrides.scrollbar_thumb_color.or(base.scrollbar_thumb),
        split: overrides.split_color.or(base.split),
        visual_bell: overrides.visual_bell_color.or(base.visual_bell),
        compose_cursor: overrides.compose_cursor_color.or(base.compose_cursor),
        copy_mode_active_highlight_fg: overrides
            .copy_mode_active_highlight_fg
            .or(base.copy_mode_active_highlight_fg),
        copy_mode_active_highlight_bg: overrides
            .copy_mode_active_highlight_bg
            .or(base.copy_mode_active_highlight_bg),
        copy_mode_inactive_highlight_fg: overrides
            .copy_mode_inactive_highlight_fg
            .or(base.copy_mode_inactive_highlight_fg),
        copy_mode_inactive_highlight_bg: overrides
            .copy_mode_inactive_highlight_bg
            .or(base.copy_mode_inactive_highlight_bg),
        quick_select_label_fg: overrides
            .quick_select_label_fg
            .or(base.quick_select_label_fg),
        quick_select_label_bg: overrides
            .quick_select_label_bg
            .or(base.quick_select_label_bg),
        quick_select_match_fg: overrides
            .quick_select_match_fg
            .or(base.quick_select_match_fg),
        quick_select_match_bg: overrides
            .quick_select_match_bg
            .or(base.quick_select_match_bg),
        input_selector_label_fg: overrides
            .input_selector_label_fg
            .or(base.input_selector_label_fg),
        input_selector_label_bg: overrides
            .input_selector_label_bg
            .or(base.input_selector_label_bg),
        launcher_label_fg: overrides.launcher_label_fg.or(base.launcher_label_fg),
        launcher_label_bg: overrides.launcher_label_bg.or(base.launcher_label_bg),
    }
}

pub(super) fn native_resolved_palette_from_overrides(
    overrides: &NativeConfigSnapshot,
) -> NativeResolvedPalette {
    let base = NativeResolvedPalette {
        cursor_bg: LEGACY_COLOR_SCHEME_CURSOR_BG_COLOR,
        ..NativeResolvedPalette::default()
    };
    native_resolved_palette_with_overrides(&base, overrides)
}

impl NativeConfigSnapshot {
    pub(super) fn with_refreshed_effective(mut self) -> Self {
        self.refresh_effective_config();
        self
    }

    pub(super) fn finish_parsing(mut self, parsed: bool) -> Option<Self> {
        parsed.then(|| {
            self.refresh_effective_config();
            self
        })
    }

    pub(super) fn refresh_effective_config(&mut self) {
        let mut effective = rterm_config::EffectiveConfig::default();
        {
            let font = Arc::make_mut(&mut effective.font);
            if let Some(family) = self.font.as_ref().filter(|family| !family.is_empty()) {
                family.clone_into(&mut font.family);
            }
            if let Some(size) = self.font_size {
                font.size_milli_points = size.millipoints;
            }
        }
        {
            let terminal = Arc::make_mut(&mut effective.terminal);
            if let Some(scrollback_lines) = self.scrollback_lines {
                terminal.scrollback_lines = scrollback_lines;
            }
            if let Some(term) = self.term.as_ref().filter(|term| !term.is_empty()) {
                term.clone_into(&mut terminal.term);
            }
        }
        if let Some(max_fps) = self.max_fps.and_then(|fps| u16::try_from(fps).ok())
            && max_fps > 0
        {
            Arc::make_mut(&mut effective.render).max_fps = max_fps;
        }
        if let Some(default_domain) = self
            .default_domain
            .as_ref()
            .filter(|domain| !domain.is_empty())
        {
            Arc::make_mut(&mut effective.domain).default_domain = Some(default_domain.clone());
        }
        if let Some(reload_on_change) = self.automatically_reload_config {
            Arc::make_mut(&mut effective.lifecycle).reload_on_change = reload_on_change;
        }
        self.effective = Arc::new(effective);
    }

    pub(super) fn effective_config(&self) -> Arc<rterm_config::EffectiveConfig> {
        Arc::clone(&self.effective)
    }
}

impl NativeWindowConfigPatch {
    pub(super) fn from_values(values: NativeWindowConfigPatchValues) -> Self {
        Self(Box::new(values))
    }
}

impl Default for NativeWindowConfigPatch {
    fn default() -> Self {
        Self::from_values(NativeWindowConfigPatchValues::default())
    }
}

impl std::ops::Deref for NativeWindowConfigPatch {
    type Target = NativeWindowConfigPatchValues;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for NativeWindowConfigPatch {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
