//! Persistent software text owner; the emergency bitmap is only a bootstrap path.
use std::error::Error;

use rssh_core::DamageRegion;
use rssh_fonts::{FontConfig, RasterCacheConfig};
use rterm_render_core::{RenderGeometry, TerminalRenderSnapshot};
use rterm_render_cpu::{CpuTextRenderer, PixelRenderer, ScrollbackScrollbar};

use crate::platform_fonts::{CatalogActivation, FontCatalogMode, PlatformFontRepository};

pub(crate) struct WindowCpuText {
    repository: PlatformFontRepository,
    text: CpuTextRenderer,
    config: FontConfig,
    dpi_scale: f32,
}

pub(crate) fn font_config(family: Option<&str>, size: f32) -> FontConfig {
    FontConfig::new(family.unwrap_or("Cascadia Mono"))
        .with_fallbacks([
            "Cascadia Mono",
            "Cascadia Code",
            "Source Code Pro",
            "Consolas",
            "Menlo",
            "Monaco",
            "Noto Sans Mono",
            "DejaVu Sans Mono",
            "Noto Sans",
            "Noto Sans SC",
            "Noto Sans JP",
            "Noto Sans CJK SC",
            "Noto Sans CJK JP",
            "Hiragino Sans GB",
            "Arial Unicode MS",
            "Apple SD Gothic Neo",
            "Noto Sans Arabic",
            "Noto Sans Devanagari",
            "Noto Sans Hebrew",
            "Noto Sans Symbols 2",
            "Noto Color Emoji",
            "Microsoft YaHei",
            "Meiryo",
            "Malgun Gothic",
            "Segoe UI",
            "Nirmala UI",
            "Segoe UI Emoji",
        ])
        .with_font_size(size)
}

/// Lazily activate real fonts required by visible text. Recreate only the shaping
/// caches on a configuration change; retain loaded font sources across frames.
pub(crate) fn prepare(
    owner: &mut Option<Box<WindowCpuText>>,
    snapshot: &TerminalRenderSnapshot,
    config: FontConfig,
    dpi_scale: f32,
) -> Result<bool, Box<dyn Error>> {
    let mut changed = false;
    if owner.is_none() {
        let mut repository = PlatformFontRepository::production_index();
        let catalog = repository.build_catalog(FontCatalogMode::Lazy)?;
        *owner = Some(Box::new(WindowCpuText {
            repository,
            text: CpuTextRenderer::new(catalog, config.clone(), RasterCacheConfig::default()),
            config: config.clone(),
            dpi_scale,
        }));
        changed = true;
    }
    let owner = owner.as_mut().expect("initialized CPU text owner");
    if owner.config != config {
        let catalog = owner
            .repository
            .rebuild_catalog_from_active(FontCatalogMode::Lazy)?;
        owner.text = CpuTextRenderer::new(catalog, config.clone(), RasterCacheConfig::default());
        owner.config = config;
        changed = true;
    }
    changed |= matches!(
        owner
            .repository
            .preflight_snapshot(snapshot, owner.text.text_catalog_mut())?,
        CatalogActivation::CatalogExpanded { .. }
    );
    changed |= owner.dpi_scale != dpi_scale;
    owner.dpi_scale = dpi_scale;
    owner.text.set_scale(dpi_scale, 1.0);
    Ok(changed)
}

/// Keeps framebuffer placement, damage, UI rows, and scrollbar composition common
/// to the transient bitmap bootstrap and persistent shaped software rendering.
pub(crate) struct CpuFramePainter<'a> {
    renderer: &'a PixelRenderer,
    text: Option<&'a mut WindowCpuText>,
}

impl<'a> CpuFramePainter<'a> {
    pub(crate) fn new(renderer: &'a PixelRenderer, text: Option<&'a mut WindowCpuText>) -> Self {
        Self { renderer, text }
    }

    pub(crate) fn render(
        &mut self,
        snapshot: &TerminalRenderSnapshot,
        frame: &mut [u8],
        width: u32,
        height: u32,
        cell_width: u32,
        cell_height: u32,
    ) {
        if let Some(owner) = self.text.as_mut() {
            self.renderer.render_shaped(
                &mut owner.text,
                snapshot,
                frame,
                RenderGeometry::new(width, height, cell_width, cell_height),
            );
        } else {
            self.renderer
                .render(snapshot, frame, width, height, cell_width, cell_height);
        }
    }

    pub(crate) fn render_damage(
        &mut self,
        snapshot: &TerminalRenderSnapshot,
        damage: &[DamageRegion],
        frame: &mut [u8],
        geometry: RenderGeometry,
    ) {
        if let Some(owner) = self.text.as_mut() {
            self.renderer
                .render_damage_shaped(&mut owner.text, snapshot, damage, frame, geometry);
        } else {
            self.renderer
                .render_damage(snapshot, damage, frame, geometry);
        }
    }

    pub(crate) fn render_scrollbar(
        &self,
        scrollbar: ScrollbackScrollbar,
        frame: &mut [u8],
        geometry: RenderGeometry,
    ) {
        self.renderer.render_scrollbar(scrollbar, frame, geometry);
    }
}
