//! GPU terminal glyph preparation backed by the authoritative `rterm-fonts`
//! shaping and raster caches.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

use glyphon::{
    Buffer, Cache, Color, ContentType, CustomGlyph, Metrics, RasterizedCustomGlyph, Resolution,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use rterm_fonts::{
    CatalogMemoryMetrics, FontCatalog, FontConfig, FontId, RasterCache, RasterCacheConfig,
    RasterContent, RasterFlags, RasterRequest, ShapedRow, TerminalShaper,
};

use rterm_render_cpu::{
    DamageRegion, RenderGeometry, TerminalContentDigest, TerminalRenderSnapshot, TextPaintConfig,
    effective_cell_colors, terminal_snapshot_content_digest, text::RowShapePlan,
    text::expand_damage_rows, text::row_shape_plan, text::vertical_align_baseline,
    text::visual_cell_starts, text_foreground_alpha,
};

use super::{GpuContextGeneration, GpuLayerError, PixelRect};

/// GPU text cache settings. The budget covers both canonical retained glyph
/// payloads and glyphon's physical mask/color texture allocations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuTextConfig {
    pub budget_bytes: usize,
    pub raster_cache: RasterCacheConfig,
    pub cursor_foreground: Option<[u8; 4]>,
    #[cfg(test)]
    identifier_ceiling_for_unit_tests: u32,
}

const MAX_CUSTOM_GLYPH_IDENTIFIER: u32 = u16::MAX as u32;
const GLYPH_IDENTIFIER_EXHAUSTED: &str =
    "glyph atlas identifier pool exhausted after 65535 custom glyph identifiers";

impl GpuTextConfig {
    #[must_use]
    pub const fn new(budget_bytes: usize, raster_cache: RasterCacheConfig) -> Self {
        Self {
            budget_bytes,
            raster_cache,
            cursor_foreground: None,
            #[cfg(test)]
            identifier_ceiling_for_unit_tests: MAX_CUSTOM_GLYPH_IDENTIFIER,
        }
    }

    /// Enables block-cursor foreground redraws. Callers should provide `None`
    /// while the cursor blink phase is hidden.
    #[must_use]
    pub const fn with_cursor_foreground(mut self, color: [u8; 4]) -> Self {
        self.cursor_foreground = Some(color);
        self
    }
}

/// Accounted retained state for the custom-glyph bridge.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GpuTextAtlasMetrics {
    pub budget_bytes: usize,
    pub retained_bytes: usize,
    pub payload_bytes: usize,
    pub physical_texture_bytes: usize,
    pub mask_dimension: u32,
    pub color_dimension: u32,
    pub entries: usize,
    pub scope_generation: u64,
    pub repack_attempts: u8,
    pub uploads: u64,
    pub trim_calls: u64,
}

/// CPU font allocations retained by a live or driver-retired text renderer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GpuTextCpuFontMetrics {
    pub catalog: CatalogMemoryMetrics,
    pub shape_cache_bytes: usize,
    pub raster_cache_bytes: usize,
    pub row_cache_entries: usize,
    pub payload_bytes: usize,
}

/// Diagnostics for one terminal text preparation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GpuTextPrepareReport {
    /// One catalog generation owns every shaped row in this report.
    pub catalog_generation: u64,
    pub prepared_rows: Vec<u16>,
    /// Glyph work performed during this preparation call.
    pub shaped_glyphs: usize,
    /// Glyphs retained in the currently drawable row cache, including reused rows.
    pub retained_glyphs: usize,
    /// Source codepoints whose shaped cluster is not covered by the configured
    /// font catalog and therefore rendered as a tofu glyph.
    pub missing_glyphs: Vec<char>,
    pub mask_glyphs: usize,
    pub color_glyphs: usize,
    pub custom_block_glyphs: usize,
    pub retained_mask_glyphs: usize,
    pub retained_color_glyphs: usize,
    pub retained_custom_block_glyphs: usize,
    pub cursor_foreground_glyphs: usize,
    pub subpixel_masks_converted: usize,
    pub second_shape_calls: usize,
    pub content_digest: TerminalContentDigest,
    pub glyph_bounds: Vec<PixelRect>,
    pub cursor_foreground_bounds: Vec<PixelRect>,
    pub damage_bounds: Vec<PixelRect>,
}

impl GpuTextPrepareReport {
    fn apply_retained_glyph_counts(&mut self, counts: (usize, usize, usize, usize)) {
        self.retained_glyphs = counts.0;
        self.retained_mask_glyphs = counts.1;
        self.retained_color_glyphs = counts.2;
        self.retained_custom_block_glyphs = counts.3;
    }
}

/// Result of preparing a frame against an app-owned catalog generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuTextCatalogStatus {
    Prepared(Box<GpuTextPrepareReport>),
    CatalogExpanded {
        previous_generation: u64,
        catalog_generation: u64,
    },
}

impl GpuTextCatalogStatus {
    #[must_use]
    pub const fn catalog_expansion(&self) -> Option<(u64, u64)> {
        match self {
            Self::CatalogExpanded {
                previous_generation,
                catalog_generation,
            } => Some((*previous_generation, *catalog_generation)),
            Self::Prepared(_) => None,
        }
    }

    #[must_use]
    pub fn into_prepared(self) -> Option<GpuTextPrepareReport> {
        match self {
            Self::Prepared(report) => Some(*report),
            Self::CatalogExpanded { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct BitmapIdentity {
    catalog_incarnation: u64,
    catalog_generation: u64,
    font_id: FontId,
    glyph_id: u16,
    font_size_bits: u32,
    weight: u16,
    flags: RasterFlags,
    width: u16,
    height: u16,
    content_type: u8,
    dpi_bits: u32,
    zoom_bits: u32,
    bytes: Arc<[u8]>,
}

#[derive(Clone, Debug)]
struct GlyphPayload {
    bytes: Arc<[u8]>,
    content_type: ContentType,
    width: u16,
    height: u16,
}

struct PreparedRasterPayload {
    bytes: Arc<[u8]>,
    content_type: ContentType,
    color: Option<Color>,
    canonical_cursor_bytes: Option<Arc<[u8]>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RetryableFailure {
    AtlasBudget,
    IdExhausted,
}

#[derive(Clone, Debug)]
struct PreparedGlyphArea {
    bounds: TextBounds,
    glyph: CustomGlyph,
}

#[derive(Clone, Debug, Default)]
struct PreparedGpuRow {
    areas: Vec<PreparedGlyphArea>,
    cursor_areas: Vec<PreparedGlyphArea>,
    blocks: Vec<super::GpuQuad>,
    shaped_glyphs: usize,
    mask_glyphs: usize,
    color_glyphs: usize,
    custom_block_glyphs: usize,
}

impl PreparedGpuRow {
    fn with_report_counts(report: &GpuTextPrepareReport) -> (Self, [usize; 4]) {
        (
            Self::default(),
            [
                report.shaped_glyphs,
                report.mask_glyphs,
                report.color_glyphs,
                report.custom_block_glyphs,
            ],
        )
    }

    fn apply_report_counts(&mut self, report: &GpuTextPrepareReport, before: [usize; 4]) {
        self.shaped_glyphs = report.shaped_glyphs.saturating_sub(before[0]);
        self.mask_glyphs = report.mask_glyphs.saturating_sub(before[1]);
        self.color_glyphs = report.color_glyphs.saturating_sub(before[2]);
        self.custom_block_glyphs = report.custom_block_glyphs.saturating_sub(before[3]);
    }
}

struct GlyphonState {
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
    cursor_renderer: Option<TextRenderer>,
    swash_cache: SwashCache,
    empty_buffer: Buffer,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct GpuTextOwnerMaterialization {
    pub(crate) base_renderer_count: u64,
    pub(crate) cursor_renderer_count: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TextPaintChange {
    None,
    BlinkRows { regular: bool, rapid: bool },
    AllRows,
}

impl GlyphonState {
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        budget_bytes: usize,
    ) -> Result<Self, GpuLayerError> {
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas =
            TextAtlas::with_max_bytes(device, queue, &cache, format, budget_bytes).ok_or_else(
                || {
                    GpuLayerError::message(format!(
                        "glyph atlas budget of {budget_bytes} bytes cannot hold the initial mask and color textures"
                    ))
                },
            )?;
        let renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);
        Ok(Self {
            viewport,
            atlas,
            renderer,
            cursor_renderer: None,
            swash_cache: SwashCache::new(),
            empty_buffer: Buffer::new_empty(Metrics::new(1.0, 1.0)),
        })
    }

    fn materialize_cursor_renderer(&mut self, device: &wgpu::Device) -> bool {
        if self.cursor_renderer.is_none() {
            self.cursor_renderer = Some(TextRenderer::new(
                &mut self.atlas,
                device,
                wgpu::MultisampleState::default(),
                None,
            ));
            true
        } else {
            false
        }
    }
}

pub(crate) struct GpuText {
    generation: GpuContextGeneration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    catalog: FontCatalog,
    shaper: TerminalShaper,
    raster: RasterCache,
    config: GpuTextConfig,
    glyphon: GlyphonState,
    owner_materializations: GpuTextOwnerMaterialization,
    identity_to_id: HashMap<BitmapIdentity, u16>,
    payloads: HashMap<u16, GlyphPayload>,
    next_id: u32,
    scope_catalog_generation: u64,
    dpi_scale: f32,
    zoom: f32,
    metrics: GpuTextAtlasMetrics,
    payload_retained_bytes: usize,
    report: GpuTextPrepareReport,
    block_quads: Vec<super::GpuQuad>,
    row_cache: BTreeMap<u16, PreparedGpuRow>,
    geometry: Option<RenderGeometry>,
    cursor_scope: Option<(u16, u16, [u8; 4])>,
    render_style_digest: u64,
    paint: Option<TextPaintConfig>,
    retryable_failure: Option<RetryableFailure>,
    force_full_rebuild_next: bool,
}

impl fmt::Debug for GpuText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GpuText")
            .field("generation", &self.generation)
            .field("format", &self.format)
            .field("catalog_generation", &self.catalog.generation())
            .field("metrics", &self.metrics)
            .field("report", &self.report)
            .finish_non_exhaustive()
    }
}

impl GpuText {
    pub(crate) fn new(
        generation: GpuContextGeneration,
        device: wgpu::Device,
        queue: wgpu::Queue,
        format: wgpu::TextureFormat,
        catalog: FontCatalog,
        font_config: FontConfig,
        config: GpuTextConfig,
    ) -> Result<Self, GpuLayerError> {
        if config.budget_bytes == 0 {
            return Err(GpuLayerError::message(
                "glyph atlas budget must be greater than zero",
            ));
        }
        let catalog_generation = catalog.generation();
        let atlas_dimension = initial_atlas_dimension(&device);
        let atlas_bytes = initial_atlas_bytes(&device);
        let glyphon = GlyphonState::new(&device, &queue, format, config.budget_bytes)?;
        Ok(Self {
            generation,
            glyphon,
            owner_materializations: GpuTextOwnerMaterialization {
                base_renderer_count: 1,
                cursor_renderer_count: 0,
            },
            device,
            queue,
            format,
            catalog,
            shaper: TerminalShaper::new(font_config),
            raster: RasterCache::new(config.raster_cache),
            config,
            identity_to_id: HashMap::new(),
            payloads: HashMap::new(),
            next_id: 1,
            scope_catalog_generation: catalog_generation,
            dpi_scale: config.raster_cache.dpi_scale,
            zoom: config.raster_cache.zoom,
            metrics: GpuTextAtlasMetrics {
                budget_bytes: config.budget_bytes,
                retained_bytes: atlas_bytes,
                physical_texture_bytes: atlas_bytes,
                mask_dimension: atlas_dimension,
                color_dimension: atlas_dimension,
                scope_generation: 1,
                ..GpuTextAtlasMetrics::default()
            },
            payload_retained_bytes: 0,
            report: GpuTextPrepareReport::default(),
            block_quads: Vec::new(),
            row_cache: BTreeMap::new(),
            geometry: None,
            cursor_scope: None,
            render_style_digest: 0,
            paint: None,
            retryable_failure: None,
            force_full_rebuild_next: false,
        })
    }

    pub(crate) fn owner_materialization(&self) -> GpuTextOwnerMaterialization {
        self.owner_materializations
    }

    pub(crate) const fn metrics(&self) -> GpuTextAtlasMetrics {
        self.metrics
    }

    pub(crate) fn catalog_mut(&mut self) -> &mut FontCatalog {
        &mut self.catalog
    }

    pub(crate) const fn catalog_generation(&self) -> u64 {
        self.catalog.generation()
    }

    pub(crate) fn cpu_font_metrics(&self) -> GpuTextCpuFontMetrics {
        GpuTextCpuFontMetrics {
            catalog: self.catalog.memory_metrics(),
            shape_cache_bytes: self.shaper.cache_metrics().current_bytes,
            raster_cache_bytes: self.raster.metrics().current_bytes,
            row_cache_entries: self.row_cache.len(),
            payload_bytes: self.payload_retained_bytes,
        }
    }

    /// Drops app-owned font bytes and CPU glyph caches while leaving driver
    /// GPU objects alive for the narrowly scoped deferred-destruction path.
    pub(crate) fn retire_cpu_font_state(&mut self) {
        self.catalog = FontCatalog::new("retired");
        self.shaper = TerminalShaper::with_cache_budget(FontConfig::new("retired"), 0);
        self.raster = RasterCache::new(RasterCacheConfig::new(0));
        self.glyphon.swash_cache = SwashCache::new();
        self.glyphon.empty_buffer = Buffer::new_empty(Metrics::new(1.0, 1.0));
        self.identity_to_id.clear();
        self.payloads.clear();
        self.row_cache.clear();
        self.block_quads.clear();
        self.geometry = None;
        self.cursor_scope = None;
        self.render_style_digest = 0;
        self.paint = None;
        self.retryable_failure = None;
        self.report = GpuTextPrepareReport::default();
        self.payload_retained_bytes = 0;
        self.metrics.payload_bytes = 0;
        self.metrics.retained_bytes = self.metrics.physical_texture_bytes;
        self.metrics.entries = 0;
        self.force_full_rebuild_next = true;
    }

    pub(crate) fn block_quads(&self) -> &[super::GpuQuad] {
        &self.block_quads
    }

    fn rebuild_scope(&mut self) -> Result<(), GpuLayerError> {
        self.glyphon = GlyphonState::new(
            &self.device,
            &self.queue,
            self.format,
            self.config.budget_bytes,
        )?;
        self.owner_materializations.base_renderer_count = self
            .owner_materializations
            .base_renderer_count
            .saturating_add(1);
        self.identity_to_id.clear();
        self.payloads.clear();
        self.row_cache.clear();
        self.block_quads.clear();
        self.geometry = None;
        self.cursor_scope = None;
        self.render_style_digest = 0;
        self.paint = None;
        self.retryable_failure = None;
        self.report = GpuTextPrepareReport::default();
        self.next_id = 1;
        let atlas = self.glyphon.atlas.metrics();
        self.payload_retained_bytes = 0;
        self.metrics.payload_bytes = 0;
        self.metrics.physical_texture_bytes = atlas.allocated_bytes;
        self.metrics.retained_bytes = atlas.allocated_bytes;
        self.metrics.mask_dimension = atlas.mask_dimension;
        self.metrics.color_dimension = atlas.color_dimension;
        self.metrics.entries = 0;
        self.metrics.scope_generation = self.metrics.scope_generation.saturating_add(1);
        Ok(())
    }

    fn ensure_scope(&mut self, dpi_scale: f32, zoom: f32) -> Result<(), GpuLayerError> {
        let changed = self.scope_catalog_generation != self.catalog.generation()
            || self.dpi_scale.to_bits() != dpi_scale.to_bits()
            || self.zoom.to_bits() != zoom.to_bits();
        if changed {
            self.rebuild_scope()?;
            self.scope_catalog_generation = self.catalog.generation();
            self.dpi_scale = dpi_scale;
            self.zoom = zoom;
        }
        self.raster.set_scale(dpi_scale, zoom);
        Ok(())
    }

    fn retained_cost(payload: &GlyphPayload) -> usize {
        payload
            .bytes
            .len()
            .saturating_add(std::mem::size_of::<BitmapIdentity>())
            .saturating_add(std::mem::size_of::<GlyphPayload>())
    }

    fn intern(
        &mut self,
        identity: BitmapIdentity,
        payload: GlyphPayload,
    ) -> Result<u16, GpuLayerError> {
        if let Some(id) = self.identity_to_id.get(&identity) {
            return Ok(*id);
        }
        let retained = Self::retained_cost(&payload);
        let next_payload = self.payload_retained_bytes.saturating_add(retained);
        let next_total = self
            .metrics
            .physical_texture_bytes
            .saturating_add(next_payload);
        if next_total > self.config.budget_bytes {
            self.retryable_failure = Some(RetryableFailure::AtlasBudget);
            return Err(GpuLayerError::message(format!(
                "glyph atlas budget of {} bytes cannot retain the prepared frame",
                self.config.budget_bytes
            )));
        }
        #[cfg(test)]
        let identifier_limit = self.config.identifier_ceiling_for_unit_tests;
        #[cfg(not(test))]
        let identifier_limit = MAX_CUSTOM_GLYPH_IDENTIFIER;
        if self.next_id > identifier_limit {
            self.retryable_failure = Some(RetryableFailure::IdExhausted);
            return Err(GpuLayerError::message(GLYPH_IDENTIFIER_EXHAUSTED));
        }
        let id = u16::try_from(self.next_id).map_err(|_| {
            self.retryable_failure = Some(RetryableFailure::IdExhausted);
            GpuLayerError::message(GLYPH_IDENTIFIER_EXHAUSTED)
        })?;
        self.next_id = self.next_id.saturating_add(1);
        self.identity_to_id.insert(identity, id);
        self.payloads.insert(id, payload);
        self.payload_retained_bytes = next_payload;
        self.metrics.payload_bytes = next_payload;
        self.metrics.retained_bytes = next_total;
        self.metrics.entries = self.payloads.len();
        self.metrics.uploads = self.metrics.uploads.saturating_add(1);
        Ok(id)
    }

    pub(crate) fn prepare(
        &mut self,
        snapshot: &TerminalRenderSnapshot,
        geometry: RenderGeometry,
        damage: &[DamageRegion],
        paint: &TextPaintConfig,
        dpi_scale: f32,
        zoom: f32,
    ) -> Result<GpuTextPrepareReport, GpuLayerError> {
        if !dpi_scale.is_finite() || dpi_scale <= 0.0 || !zoom.is_finite() || zoom <= 0.0 {
            return Err(GpuLayerError::message(
                "GPU text DPI scale and zoom must be finite and greater than zero",
            ));
        }
        if geometry.cell_width == 0
            || geometry.cell_height == 0
            || geometry.target_width == 0
            || geometry.target_height == 0
            || geometry.content_width == 0
            || geometry.content_height == 0
        {
            return Err(GpuLayerError::message("GPU text geometry must be nonzero"));
        }
        self.ensure_scope(dpi_scale, zoom)?;
        let paint_change = self
            .paint
            .as_ref()
            .map_or(TextPaintChange::AllRows, |previous| {
                text_paint_change(previous, paint, snapshot)
            });
        if self.force_full_rebuild_next {
            self.row_cache.clear();
            self.force_full_rebuild_next = false;
        }
        if paint_change == TextPaintChange::AllRows {
            self.row_cache.clear();
        }
        self.paint = Some(paint.clone());
        self.metrics.repack_attempts = 0;
        self.retryable_failure = None;
        self.glyphon.atlas.trim();
        self.metrics.trim_calls = self.metrics.trim_calls.saturating_add(1);

        match self.prepare_once(snapshot, geometry, damage, paint, paint_change) {
            Ok(report) => Ok(report),
            Err(error) if self.retryable_failure.is_some() => {
                self.metrics.repack_attempts = 1;
                self.rebuild_scope()?;
                self.metrics.repack_attempts = 1;
                self.paint = Some(paint.clone());
                self.retryable_failure = None;
                match self.prepare_once(snapshot, geometry, &[], paint, TextPaintChange::AllRows) {
                    Ok(report) => Ok(report),
                    Err(retry_error) => {
                        self.fail_closed_after_prepare_error()?;
                        Err(GpuLayerError::message(format!(
                            "{retry_error}; full-frame retry after {error} also failed"
                        )))
                    }
                }
            }
            Err(error) => {
                self.fail_closed_after_prepare_error()?;
                Err(error)
            }
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "generation joins the existing explicit whole-frame text preparation contract"
    )]
    pub(crate) fn prepare_for_catalog_generation(
        &mut self,
        expected_generation: u64,
        snapshot: &TerminalRenderSnapshot,
        geometry: RenderGeometry,
        damage: &[DamageRegion],
        paint: &TextPaintConfig,
        dpi_scale: f32,
        zoom: f32,
    ) -> Result<GpuTextCatalogStatus, GpuLayerError> {
        let before = self.catalog.generation();
        if before != expected_generation {
            return Ok(GpuTextCatalogStatus::CatalogExpanded {
                previous_generation: expected_generation,
                catalog_generation: before,
            });
        }
        let report = self.prepare(snapshot, geometry, damage, paint, dpi_scale, zoom)?;
        let after = self.catalog.generation();
        if after != expected_generation {
            self.fail_closed_after_prepare_error()?;
            return Ok(GpuTextCatalogStatus::CatalogExpanded {
                previous_generation: expected_generation,
                catalog_generation: after,
            });
        }
        Ok(GpuTextCatalogStatus::Prepared(Box::new(report)))
    }

    pub(crate) fn discard_prepared_frame(&mut self) -> Result<(), GpuLayerError> {
        self.fail_closed_after_prepare_error()
    }

    fn fail_closed_after_prepare_error(&mut self) -> Result<(), GpuLayerError> {
        self.rebuild_scope()?;
        self.force_full_rebuild_next = true;
        Ok(())
    }

    fn should_reuse_cached_rows(
        &self,
        damage: &[DamageRegion],
        content_digest: TerminalContentDigest,
        render_style_digest: u64,
    ) -> bool {
        damage.is_empty()
            && !self.row_cache.is_empty()
            && self.report.content_digest == content_digest
            && self.render_style_digest == render_style_digest
    }

    fn prepare_once(
        &mut self,
        snapshot: &TerminalRenderSnapshot,
        geometry: RenderGeometry,
        damage: &[DamageRegion],
        paint: &TextPaintConfig,
        paint_change: TextPaintChange,
    ) -> Result<GpuTextPrepareReport, GpuLayerError> {
        let columns_u32 = geometry.content_width / geometry.cell_width;
        let rows_u32 = geometry.content_height / geometry.cell_height;
        let columns = u16::try_from(columns_u32).unwrap_or(u16::MAX);
        let rows = u16::try_from(rows_u32).unwrap_or(u16::MAX);
        if self.geometry != Some(geometry) {
            self.row_cache.clear();
            self.geometry = Some(geometry);
        }
        let content_digest = terminal_snapshot_content_digest(snapshot);
        let render_style_digest = if damage.is_empty() {
            terminal_snapshot_render_style_digest(snapshot)
        } else {
            self.render_style_digest
        };
        let visible_rows = snapshot
            .iter_cells()
            .map(|cell| cell.row)
            .filter(|row| *row < rows)
            .collect::<BTreeSet<_>>();
        let damaged_rows = if damage.is_empty() {
            visible_rows.clone()
        } else {
            expand_damage_rows(damage, columns_u32, rows_u32)
                .into_iter()
                .map(|region| region.y)
                .collect::<BTreeSet<_>>()
        };
        let reuse_cached_rows =
            self.should_reuse_cached_rows(damage, content_digest, render_style_digest);
        let mut row_numbers = if damage.is_empty() {
            if reuse_cached_rows {
                BTreeSet::new()
            } else {
                self.row_cache.clear();
                visible_rows.clone()
            }
        } else if self.row_cache.is_empty() {
            visible_rows.clone()
        } else {
            damaged_rows.clone()
        };
        if let TextPaintChange::BlinkRows { regular, rapid } = paint_change {
            row_numbers.extend(blink_rows_requiring_rebuild(snapshot, regular, rapid));
        }
        let cursor_scope = snapshot
            .cursor()
            .filter(|cursor| cursor.shape == rterm_terminal::CursorShape::Block)
            .and_then(|cursor| {
                self.config
                    .cursor_foreground
                    .map(|foreground| (cursor.row, cursor.column, foreground))
            });
        if self.cursor_scope != cursor_scope {
            if let Some((row, _, _)) = self.cursor_scope
                && row < rows
            {
                row_numbers.insert(row);
            }
            if let Some((row, _, _)) = cursor_scope
                && row < rows
            {
                row_numbers.insert(row);
            }
            self.cursor_scope = cursor_scope;
        }
        self.row_cache
            .retain(|row, _| *row < rows && visible_rows.contains(row));
        let mut report = GpuTextPrepareReport {
            catalog_generation: self.catalog.generation(),
            prepared_rows: row_numbers.iter().copied().collect(),
            content_digest,
            missing_glyphs: if reuse_cached_rows {
                self.report.missing_glyphs.clone()
            } else {
                Vec::new()
            },
            damage_bounds: damaged_rows
                .iter()
                .map(|row| {
                    PixelRect::new(
                        geometry.content_x,
                        geometry
                            .content_y
                            .saturating_add(u32::from(*row).saturating_mul(geometry.cell_height)),
                        geometry.content_width,
                        geometry.cell_height,
                    )
                })
                .collect(),
            ..GpuTextPrepareReport::default()
        };
        self.prepare_rows(snapshot, geometry, columns, row_numbers, paint, &mut report)?;
        report.apply_retained_glyph_counts(self.retained_glyph_counts());
        self.prepare_atlas(geometry)?;
        self.render_style_digest = render_style_digest;
        self.report = report.clone();
        Ok(report)
    }

    #[expect(
        clippy::cast_precision_loss,
        clippy::too_many_lines,
        reason = "one row preparation pass keeps shaping, raster identity, clipping, and cursor redraw ordering explicit"
    )]
    fn prepare_rows(
        &mut self,
        snapshot: &TerminalRenderSnapshot,
        geometry: RenderGeometry,
        columns: u16,
        row_numbers: BTreeSet<u16>,
        paint: &TextPaintConfig,
        report: &mut GpuTextPrepareReport,
    ) -> Result<(), GpuLayerError> {
        for row in row_numbers {
            let plan = row_shape_plan(snapshot, row, columns);
            if plan.clusters.is_empty() {
                self.row_cache.insert(row, PreparedGpuRow::default());
                continue;
            }
            let (mut prepared_row, counters_before) = PreparedGpuRow::with_report_counts(report);
            let shaped = self
                .shaper
                .shape_clusters(&mut self.catalog, &plan.clusters)
                .map_err(|error| GpuLayerError::message(format!("shape GPU text row: {error}")))?;
            report.shaped_glyphs = report.shaped_glyphs.saturating_add(shaped.glyphs.len());
            for cluster in shaped.clusters.iter().filter(|cluster| cluster.is_tofu) {
                for codepoint in shaped.text[cluster.byte_range.clone()].chars() {
                    if !report.missing_glyphs.contains(&codepoint) {
                        report.missing_glyphs.push(codepoint);
                    }
                }
            }
            let scale_x = geometry.cell_width as f32 / shaped.metrics.cell_width;
            let visual_starts = visual_cell_starts(&shaped);
            let baseline = geometry.content_y as f32
                + f32::from(row) * geometry.cell_height as f32
                + shaped.metrics.baseline / shaped.metrics.line_height
                    * geometry.cell_height as f32;
            let cursor_redraw = snapshot
                .cursor()
                .filter(|cursor| {
                    cursor.row == row && cursor.shape == rterm_terminal::CursorShape::Block
                })
                .and_then(|cursor| {
                    let foreground = self.config.cursor_foreground?;
                    let cluster = shaped
                        .clusters
                        .iter()
                        .find(|cluster| cluster.cell_span.contains(&usize::from(cursor.column)))?;
                    let visual_cell = visual_starts[cluster.logical_index].saturating_add(
                        usize::from(cursor.column).saturating_sub(cluster.cell_span.start),
                    );
                    Some((cursor, visual_cell, foreground))
                });
            for glyph in &shaped.glyphs {
                let style = &plan.styles[glyph.cluster_range.start];
                if style.conceal {
                    continue;
                }
                let (foreground, _) = effective_cell_colors(
                    style,
                    paint.bold_brightens_ansi_colors,
                    paint.default_foreground,
                    paint.default_background,
                    paint.ansi_palette.as_ref(),
                    paint.indexed_palette.as_ref(),
                );
                let blink_alpha = text_foreground_alpha(
                    style,
                    paint.text_blink_opacity_alpha,
                    paint.rapid_text_blink_opacity_alpha,
                );
                let alpha = modulate_alpha(foreground[3], blink_alpha);
                if shaped
                    .text
                    .get(glyph.byte_range.clone())
                    .is_some_and(|text| text.chars().all(|character| character == '\u{2588}'))
                {
                    let logical_cluster = glyph.cluster_range.start;
                    let visual_start = visual_starts[logical_cluster];
                    let cell_width = glyph.cell_span.end.saturating_sub(glyph.cell_span.start);
                    prepared_row.blocks.push(super::GpuQuad::new(
                        super::GpuLayer::Glyph,
                        PixelRect::new(
                            geometry.content_x.saturating_add(
                                u32::try_from(visual_start)
                                    .unwrap_or(u32::MAX)
                                    .saturating_mul(geometry.cell_width),
                            ),
                            geometry.content_y.saturating_add(
                                u32::from(row).saturating_mul(geometry.cell_height),
                            ),
                            u32::try_from(cell_width)
                                .unwrap_or(u32::MAX)
                                .saturating_mul(geometry.cell_width),
                            geometry.cell_height,
                        ),
                        [foreground[0], foreground[1], foreground[2], alpha],
                    ));
                    report.custom_block_glyphs = report.custom_block_glyphs.saturating_add(1);
                    if let Some((cursor, visual_cell, cursor_foreground)) = cursor_redraw
                        && glyph.cell_span.contains(&usize::from(cursor.column))
                    {
                        let cursor_x = u32::try_from(visual_cell)
                            .unwrap_or(u32::MAX)
                            .saturating_mul(geometry.cell_width)
                            .saturating_add(geometry.content_x);
                        let cursor_rect = PixelRect::new(
                            cursor_x,
                            geometry.content_y.saturating_add(
                                u32::from(row).saturating_mul(geometry.cell_height),
                            ),
                            geometry.cell_width,
                            geometry.cell_height,
                        );
                        prepared_row.blocks.push(super::GpuQuad::new(
                            super::GpuLayer::Cursor,
                            cursor_rect,
                            cursor_foreground,
                        ));
                        report.cursor_foreground_glyphs =
                            report.cursor_foreground_glyphs.saturating_add(1);
                        report.cursor_foreground_bounds.push(cursor_rect);
                    }
                    continue;
                }
                let logical_x = geometry.content_x as f32 + glyph.x * scale_x;
                let aligned_baseline =
                    vertical_align_baseline(baseline, geometry.cell_height, style);
                let request = RasterRequest::for_shaped_glyph_at_physical_position(
                    &shaped,
                    glyph,
                    logical_x,
                    aligned_baseline,
                );
                let Some(positioned) = self.raster.rasterize_positioned(&mut self.catalog, request)
                else {
                    continue;
                };
                let raster = positioned.image;
                let width = u16::try_from(raster.width).map_err(|_| {
                    GpuLayerError::message("GPU glyph width exceeds the u16 atlas limit")
                })?;
                let height = u16::try_from(raster.height).map_err(|_| {
                    GpuLayerError::message("GPU glyph height exceeds the u16 atlas limit")
                })?;
                let PreparedRasterPayload {
                    bytes,
                    content_type,
                    color,
                    canonical_cursor_bytes,
                } = prepare_raster_payload(
                    &raster.content,
                    foreground,
                    alpha,
                    style.faint,
                    blink_alpha,
                    report,
                );
                let identity = BitmapIdentity {
                    catalog_incarnation: self.catalog.incarnation(),
                    catalog_generation: self.catalog.generation(),
                    font_id: glyph.font_id,
                    glyph_id: glyph.glyph_id,
                    font_size_bits: glyph.raster_font_size.to_bits(),
                    weight: glyph.raster_weight,
                    flags: glyph.raster_flags,
                    width,
                    height,
                    content_type: match content_type {
                        ContentType::Mask => 1,
                        ContentType::Color => 4,
                    },
                    dpi_bits: self.dpi_scale.to_bits(),
                    zoom_bits: self.zoom.to_bits(),
                    bytes: Arc::clone(&bytes),
                };
                let id = self.intern(
                    identity.clone(),
                    GlyphPayload {
                        bytes,
                        content_type,
                        width,
                        height,
                    },
                )?;
                let cursor_id = if cursor_redraw.is_some_and(|(cursor, _, _)| {
                    glyph.cell_span.contains(&usize::from(cursor.column))
                }) {
                    if let Some(canonical_bytes) = canonical_cursor_bytes {
                        let mut canonical_identity = identity;
                        canonical_identity.bytes = Arc::clone(&canonical_bytes);
                        self.intern(
                            canonical_identity,
                            GlyphPayload {
                                bytes: canonical_bytes,
                                content_type: ContentType::Color,
                                width,
                                height,
                            },
                        )?
                    } else {
                        id
                    }
                } else {
                    id
                };
                let left = i64::from(positioned.origin_x) + i64::from(raster.left);
                let top = i64::from(positioned.origin_y) - i64::from(raster.top);
                let (clip_x, clip_width) = shaped_run_clip(
                    &plan,
                    &shaped,
                    &visual_starts,
                    glyph.cluster_range.start,
                    geometry,
                );
                let clip_y = geometry
                    .content_y
                    .saturating_add(u32::from(row).saturating_mul(geometry.cell_height));
                let Some(bounds) = clipped_bounds(
                    left,
                    top,
                    u32::from(width),
                    u32::from(height),
                    clip_x,
                    clip_y,
                    clip_width,
                    geometry.cell_height,
                ) else {
                    continue;
                };
                report.glyph_bounds.push(bounds);
                let custom_glyph = CustomGlyph {
                    id,
                    left: left as f32,
                    top: top as f32,
                    width: f32::from(width),
                    height: f32::from(height),
                    color,
                    snap_to_physical_pixel: true,
                    metadata: glyph.visual_order,
                };
                if let Some((cursor, visual_cell, foreground)) = cursor_redraw
                    && glyph.cell_span.contains(&usize::from(cursor.column))
                    && !style.conceal
                {
                    let cursor_x = u32::try_from(visual_cell)
                        .unwrap_or(u32::MAX)
                        .saturating_mul(geometry.cell_width)
                        .saturating_add(geometry.content_x);
                    let cursor_bounds = clipped_bounds(
                        left,
                        top,
                        u32::from(width),
                        u32::from(height),
                        cursor_x,
                        clip_y,
                        geometry.cell_width,
                        geometry.cell_height,
                    );
                    if let Some(cursor_bounds) = cursor_bounds {
                        report.cursor_foreground_bounds.push(cursor_bounds);
                        report.cursor_foreground_glyphs =
                            report.cursor_foreground_glyphs.saturating_add(1);
                        let mut cursor_glyph = custom_glyph;
                        cursor_glyph.id = cursor_id;
                        if cursor_glyph.color.is_some() {
                            cursor_glyph.color = Some(Color::rgba(
                                foreground[0],
                                foreground[1],
                                foreground[2],
                                foreground[3],
                            ));
                        }
                        prepared_row.cursor_areas.push(PreparedGlyphArea {
                            bounds: TextBounds {
                                left: i32::try_from(cursor_x).unwrap_or(i32::MAX),
                                top: i32::try_from(clip_y).unwrap_or(i32::MAX),
                                right: i32::try_from(cursor_x.saturating_add(geometry.cell_width))
                                    .unwrap_or(i32::MAX),
                                bottom: i32::try_from(clip_y.saturating_add(geometry.cell_height))
                                    .unwrap_or(i32::MAX),
                            },
                            glyph: cursor_glyph,
                        });
                    }
                }
                prepared_row.areas.push(PreparedGlyphArea {
                    bounds: TextBounds {
                        left: i32::try_from(clip_x).unwrap_or(i32::MAX),
                        top: i32::try_from(clip_y).unwrap_or(i32::MAX),
                        right: i32::try_from(clip_x.saturating_add(clip_width)).unwrap_or(i32::MAX),
                        bottom: i32::try_from(clip_y.saturating_add(geometry.cell_height))
                            .unwrap_or(i32::MAX),
                    },
                    glyph: custom_glyph,
                });
            }
            prepared_row.apply_report_counts(report, counters_before);
            self.row_cache.insert(row, prepared_row);
        }
        Ok(())
    }

    fn retained_glyph_counts(&self) -> (usize, usize, usize, usize) {
        self.row_cache
            .values()
            .fold((0, 0, 0, 0), |mut counts, row| {
                counts.0 = counts.0.saturating_add(row.shaped_glyphs);
                counts.1 = counts.1.saturating_add(row.mask_glyphs);
                counts.2 = counts.2.saturating_add(row.color_glyphs);
                counts.3 = counts.3.saturating_add(row.custom_block_glyphs);
                counts
            })
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the main and cursor atlas preparations intentionally share one atomic budget and metrics commit"
    )]
    fn prepare_atlas(&mut self, geometry: RenderGeometry) -> Result<(), GpuLayerError> {
        self.glyphon.viewport.update(
            &self.queue,
            Resolution {
                width: geometry.target_width,
                height: geometry.target_height,
            },
        );
        let payloads = &self.payloads;
        let physical_budget = self
            .config
            .budget_bytes
            .saturating_sub(self.payload_retained_bytes);
        if !self.glyphon.atlas.set_max_bytes(physical_budget) {
            self.retryable_failure = Some(RetryableFailure::AtlasBudget);
            return Err(GpuLayerError::message(format!(
                "glyph atlas budget of {} bytes is below the current physical texture allocation",
                self.config.budget_bytes
            )));
        }
        {
            let areas = self.row_cache.values().flat_map(|row| {
                row.areas.iter().map(|area| TextArea {
                    buffer: &self.glyphon.empty_buffer,
                    left: 0.0,
                    top: 0.0,
                    scale: 1.0,
                    bounds: area.bounds,
                    default_color: Color::rgb(229, 229, 229),
                    custom_glyphs: std::slice::from_ref(&area.glyph),
                })
            });
            let result = self.glyphon.renderer.prepare_with_custom(
                &self.device,
                &self.queue,
                self.catalog.font_system_mut(),
                &mut self.glyphon.atlas,
                &self.glyphon.viewport,
                areas,
                &mut self.glyphon.swash_cache,
                |request| {
                    let payload = payloads.get(&request.id)?;
                    if payload.width != request.width || payload.height != request.height {
                        return None;
                    }
                    Some(RasterizedCustomGlyph {
                        data: payload.bytes.to_vec(),
                        content_type: payload.content_type,
                    })
                },
            );
            if let Err(error) = result {
                self.retryable_failure = Some(RetryableFailure::AtlasBudget);
                return Err(GpuLayerError::message(format!(
                    "glyph atlas budget exhausted while preparing physical textures: {error}"
                )));
            }
        }
        let has_cursor_foreground = self
            .row_cache
            .values()
            .any(|row| !row.cursor_areas.is_empty());
        if has_cursor_foreground && self.glyphon.materialize_cursor_renderer(&self.device) {
            self.owner_materializations.cursor_renderer_count = self
                .owner_materializations
                .cursor_renderer_count
                .saturating_add(1);
        }
        if let Some(cursor_renderer) = self.glyphon.cursor_renderer.as_mut() {
            let areas = self.row_cache.values().flat_map(|row| {
                row.cursor_areas.iter().map(|area| TextArea {
                    buffer: &self.glyphon.empty_buffer,
                    left: 0.0,
                    top: 0.0,
                    scale: 1.0,
                    bounds: area.bounds,
                    default_color: Color::rgb(229, 229, 229),
                    custom_glyphs: std::slice::from_ref(&area.glyph),
                })
            });
            let result = cursor_renderer.prepare_with_custom(
                &self.device,
                &self.queue,
                self.catalog.font_system_mut(),
                &mut self.glyphon.atlas,
                &self.glyphon.viewport,
                areas,
                &mut self.glyphon.swash_cache,
                |request| {
                    let payload = payloads.get(&request.id)?;
                    if payload.width != request.width || payload.height != request.height {
                        return None;
                    }
                    Some(RasterizedCustomGlyph {
                        data: payload.bytes.to_vec(),
                        content_type: payload.content_type,
                    })
                },
            );
            if let Err(error) = result {
                self.retryable_failure = Some(RetryableFailure::AtlasBudget);
                return Err(GpuLayerError::message(format!(
                    "glyph atlas budget exhausted while preparing cursor foreground: {error}"
                )));
            }
        }
        let atlas = self.glyphon.atlas.metrics();
        self.metrics.physical_texture_bytes = atlas.allocated_bytes;
        self.metrics.mask_dimension = atlas.mask_dimension;
        self.metrics.color_dimension = atlas.color_dimension;
        self.metrics.retained_bytes = atlas
            .allocated_bytes
            .saturating_add(self.payload_retained_bytes);
        self.block_quads = self
            .row_cache
            .values()
            .flat_map(|row| row.blocks.iter().copied())
            .collect();
        Ok(())
    }

    pub(crate) fn render<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
    ) -> Result<(), GpuLayerError> {
        self.glyphon
            .renderer
            .render(&self.glyphon.atlas, &self.glyphon.viewport, pass)
            .map_err(|error| GpuLayerError::message(format!("render GPU glyph atlas: {error}")))
    }

    pub(crate) fn render_cursor<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
    ) -> Result<(), GpuLayerError> {
        let Some(renderer) = self.glyphon.cursor_renderer.as_ref() else {
            return Ok(());
        };
        renderer
            .render(&self.glyphon.atlas, &self.glyphon.viewport, pass)
            .map_err(|error| {
                GpuLayerError::message(format!("render GPU cursor foreground atlas: {error}"))
            })
    }
}

fn initial_atlas_dimension(device: &wgpu::Device) -> u32 {
    256.min(device.limits().max_texture_dimension_2d)
}

fn initial_atlas_bytes(device: &wgpu::Device) -> usize {
    let dimension = initial_atlas_dimension(device) as usize;
    dimension.saturating_mul(dimension).saturating_mul(5)
}

fn terminal_snapshot_render_style_digest(snapshot: &TerminalRenderSnapshot) -> u64 {
    let mut digest = DefaultHasher::new();
    for cell in snapshot.iter_cells() {
        cell.style().hash(&mut digest);
        cell.hyperlink().hash(&mut digest);
    }
    digest.finish()
}

fn text_paint_change(
    previous: &TextPaintConfig,
    current: &TextPaintConfig,
    snapshot: &TerminalRenderSnapshot,
) -> TextPaintChange {
    if previous.default_foreground != current.default_foreground
        || previous.default_background != current.default_background
        || previous.bold_brightens_ansi_colors != current.bold_brightens_ansi_colors
        || previous.ansi_palette != current.ansi_palette
        || previous.indexed_palette != current.indexed_palette
    {
        return TextPaintChange::AllRows;
    }
    let regular_blink_changed =
        previous.text_blink_opacity_alpha != current.text_blink_opacity_alpha;
    let rapid_blink_changed =
        previous.rapid_text_blink_opacity_alpha != current.rapid_text_blink_opacity_alpha;
    if !regular_blink_changed && !rapid_blink_changed {
        return TextPaintChange::None;
    }
    if snapshot.iter_cells().any(|cell| {
        cell.style().blink
            && ((regular_blink_changed && !cell.style().rapid_blink)
                || (rapid_blink_changed && cell.style().rapid_blink))
    }) {
        TextPaintChange::BlinkRows {
            regular: regular_blink_changed,
            rapid: rapid_blink_changed,
        }
    } else {
        TextPaintChange::None
    }
}

fn blink_rows_requiring_rebuild(
    snapshot: &TerminalRenderSnapshot,
    regular: bool,
    rapid: bool,
) -> BTreeSet<u16> {
    snapshot
        .iter_cells()
        .filter_map(|cell| {
            let style = cell.style();
            let regular_row = regular && style.blink && !style.rapid_blink;
            let rapid_row = rapid && style.blink && style.rapid_blink;
            (regular_row || rapid_row).then_some(cell.row)
        })
        .collect()
}

fn modulate_alpha(left: u8, right: u8) -> u8 {
    u8::try_from((u16::from(left) * u16::from(right)) / 255).unwrap_or(u8::MAX)
}

fn prepare_raster_payload(
    content: &RasterContent,
    foreground: [u8; 4],
    alpha: u8,
    faint: bool,
    blink_alpha: u8,
    report: &mut GpuTextPrepareReport,
) -> PreparedRasterPayload {
    match content {
        RasterContent::Mask(bytes) => {
            report.mask_glyphs = report.mask_glyphs.saturating_add(1);
            PreparedRasterPayload {
                bytes: Arc::<[u8]>::from(bytes.clone()),
                content_type: ContentType::Mask,
                color: Some(Color::rgba(
                    foreground[0],
                    foreground[1],
                    foreground[2],
                    alpha,
                )),
                canonical_cursor_bytes: None,
            }
        }
        RasterContent::SubpixelMask(bytes) => {
            report.mask_glyphs = report.mask_glyphs.saturating_add(1);
            report.subpixel_masks_converted = report.subpixel_masks_converted.saturating_add(1);
            PreparedRasterPayload {
                bytes: Arc::<[u8]>::from(subpixel_to_grayscale(bytes)),
                content_type: ContentType::Mask,
                color: Some(Color::rgba(
                    foreground[0],
                    foreground[1],
                    foreground[2],
                    alpha,
                )),
                canonical_cursor_bytes: None,
            }
        }
        RasterContent::Rgba(bytes) => {
            report.color_glyphs = report.color_glyphs.saturating_add(1);
            let mut pixels = bytes.clone();
            if faint || blink_alpha != u8::MAX {
                for pixel in pixels.chunks_exact_mut(4) {
                    if faint {
                        pixel[0] /= 2;
                        pixel[1] /= 2;
                        pixel[2] /= 2;
                    }
                    pixel[3] = modulate_alpha(pixel[3], blink_alpha);
                }
            }
            PreparedRasterPayload {
                bytes: Arc::<[u8]>::from(pixels),
                content_type: ContentType::Color,
                color: None,
                canonical_cursor_bytes: Some(Arc::<[u8]>::from(bytes.clone())),
            }
        }
    }
}

fn subpixel_to_grayscale(bytes: &[u8]) -> Vec<u8> {
    bytes
        .chunks_exact(4)
        .map(|pixel| pixel[0].max(pixel[1]).max(pixel[2]))
        .collect()
}

fn shaped_run_clip(
    plan: &RowShapePlan,
    shaped: &ShapedRow,
    visual_starts: &[usize],
    logical_index: usize,
    geometry: RenderGeometry,
) -> (u32, u32) {
    if plan.run_count <= 1 {
        return (geometry.content_x, geometry.content_width);
    }
    let Some(cluster) = shaped.clusters.get(logical_index) else {
        return (geometry.content_x, geometry.content_width);
    };
    let Some(input) = plan.clusters.get(logical_index) else {
        return (geometry.content_x, geometry.content_width);
    };
    let boundary = input.shape_boundary;
    let mut visual_start = cluster.visual_index;
    while visual_start > 0 {
        let previous = shaped.visual_clusters[visual_start - 1];
        if plan.clusters[previous].shape_boundary != boundary {
            break;
        }
        visual_start -= 1;
    }
    let mut visual_end = cluster.visual_index + 1;
    while visual_end < shaped.visual_clusters.len() {
        let next = shaped.visual_clusters[visual_end];
        if plan.clusters[next].shape_boundary != boundary {
            break;
        }
        visual_end += 1;
    }
    let first_logical = shaped.visual_clusters[visual_start];
    let last_logical = shaped.visual_clusters[visual_end - 1];
    let first_cell = visual_starts[first_logical];
    let last_cluster = &shaped.clusters[last_logical];
    let last_cell = visual_starts[last_logical].saturating_add(
        last_cluster
            .cell_span
            .end
            .saturating_sub(last_cluster.cell_span.start),
    );
    let x = u32::try_from(first_cell)
        .unwrap_or(u32::MAX)
        .saturating_mul(geometry.cell_width)
        .min(geometry.content_width)
        .saturating_add(geometry.content_x);
    let right = u32::try_from(last_cell)
        .unwrap_or(u32::MAX)
        .saturating_mul(geometry.cell_width)
        .min(geometry.content_width)
        .saturating_add(geometry.content_x);
    (x, right.saturating_sub(x))
}

#[expect(
    clippy::too_many_arguments,
    reason = "signed glyph placement and half-open clip geometry are intentionally explicit"
)]
fn clipped_bounds(
    x: i64,
    y: i64,
    width: u32,
    height: u32,
    clip_x: u32,
    clip_y: u32,
    clip_width: u32,
    clip_height: u32,
) -> Option<PixelRect> {
    let left = x.max(i64::from(clip_x));
    let top = y.max(i64::from(clip_y));
    let right = x
        .saturating_add(i64::from(width))
        .min(i64::from(clip_x.saturating_add(clip_width)));
    let bottom = y
        .saturating_add(i64::from(height))
        .min(i64::from(clip_y.saturating_add(clip_height)));
    if right <= left || bottom <= top {
        return None;
    }
    Some(PixelRect::new(
        u32::try_from(left).ok()?,
        u32::try_from(top).ok()?,
        u32::try_from(right - left).ok()?,
        u32::try_from(bottom - top).ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use rterm_fonts::{FontCatalog, FontConfig, FontSource, RasterCacheConfig};
    use rterm_terminal::Terminal;
    use rterm_types::TerminalSize;

    use crate::gpu::{GpuContext, GpuContextOptions, GpuLayerRenderer};
    use rterm_render_cpu::{DamageRegion, RenderGeometry, TerminalRenderSnapshot, TextPaintConfig};

    use super::{GpuTextConfig, subpixel_to_grayscale};

    #[test]
    fn subpixel_mask_is_compacted_to_one_grayscale_byte_per_pixel() {
        let rgba_subpixel = [1, 7, 3, 255, 19, 11, 13, 0];
        let grayscale = subpixel_to_grayscale(&rgba_subpixel);
        assert_eq!(grayscale, [7, 19]);
        assert_eq!(grayscale.len(), rgba_subpixel.len() / 4);
    }

    #[test]
    fn prepare_report_tracks_real_tofu_without_flagging_configured_cjk_fallback() {
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless adapter");
        let geometry = RenderGeometry::new(32, 48, 16, 24);
        let paint = TextPaintConfig::default();
        let mut renderer =
            GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 64 * 1024)
                .expect("renderer");
        renderer
            .enable_text(catalog(), font_config(), test_config(256))
            .expect("enable text");

        let fallback_report = renderer
            .prepare_text(
                &multiline_snapshot("中文", 4, 1),
                geometry,
                &[],
                &paint,
                1.0,
                1.0,
            )
            .expect("shape configured CJK fallback");
        assert!(
            fallback_report.missing_glyphs.is_empty(),
            "configured CJK fallback must not be reported as tofu: {:?}",
            fallback_report.missing_glyphs
        );

        let tofu_report = renderer
            .prepare_text(
                &multiline_snapshot("\u{10ffff}", 4, 1),
                geometry,
                &[],
                &paint,
                1.0,
                1.0,
            )
            .expect("shape uncovered scalar");
        assert_eq!(tofu_report.missing_glyphs, ['\u{10ffff}']);
    }

    #[test]
    fn id_exhaustion_retries_full_frame_without_aliasing_and_failure_is_atomic() {
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless adapter");
        let geometry = RenderGeometry::new(32, 48, 16, 24);
        let paint = TextPaintConfig::default();

        let mut recoverable =
            GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 64 * 1024)
                .expect("recoverable renderer");
        recoverable
            .enable_text(catalog(), font_config(), test_config(2))
            .expect("enable low identifier seam");
        recoverable
            .prepare_text(
                &multiline_snapshot("A\r\nB", 2, 2),
                geometry,
                &[],
                &paint,
                1.0,
                1.0,
            )
            .expect("fill two simulated identifiers");
        let recovered = recoverable
            .prepare_text(
                &multiline_snapshot("A\r\nC", 2, 2),
                geometry,
                &[DamageRegion::new(0, 1, 1, 1)],
                &paint,
                1.0,
                1.0,
            )
            .expect("ID exhaustion rebuilds and retries the complete frame");
        let recovered_metrics = recoverable.text_atlas_metrics().expect("recovered metrics");
        assert_eq!(recovered.prepared_rows, vec![0, 1]);
        assert_eq!(recovered_metrics.entries, 2);
        assert_eq!(recovered_metrics.repack_attempts, 1);

        let mut atomic =
            GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 64 * 1024)
                .expect("atomic renderer");
        atomic
            .enable_text(catalog(), font_config(), test_config(1))
            .expect("enable one-identifier seam");
        let error = atomic
            .prepare_text(
                &multiline_snapshot("A\r\nB", 2, 2),
                geometry,
                &[],
                &paint,
                1.0,
                1.0,
            )
            .expect_err("both full-frame attempts must exhaust one identifier");
        assert!(error.to_string().contains("identifier pool exhausted"));
        let failed = atomic.text_atlas_metrics().expect("failed metrics");
        assert_eq!(failed.entries, 0);
        assert_eq!(failed.payload_bytes, 0);
        assert_eq!(failed.repack_attempts, 1);

        let restored = atomic
            .prepare_text(
                &multiline_snapshot("A\r\nA", 2, 2),
                geometry,
                &[DamageRegion::new(0, 0, 1, 1)],
                &paint,
                1.0,
                1.0,
            )
            .expect("small damage after failure must rebuild every visible row");
        assert_eq!(restored.prepared_rows, vec![0, 1]);
        assert_eq!(
            atomic
                .text_atlas_metrics()
                .expect("restored metrics")
                .entries,
            1
        );
    }

    fn test_config(identifier_limit: u32) -> GpuTextConfig {
        let mut config =
            GpuTextConfig::new(4 * 1024 * 1024, RasterCacheConfig::new(4 * 1024 * 1024));
        config.identifier_ceiling_for_unit_tests = identifier_limit;
        config
    }

    fn fixture_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/fonts")
    }

    fn source(name: &str) -> FontSource {
        FontSource::new(
            name,
            fs::read(fixture_dir().join(name)).expect("read deterministic font fixture"),
        )
    }

    fn catalog() -> FontCatalog {
        FontCatalog::from_sources(
            "en-US",
            [
                "NotoSans-Latin.fixture.ttf",
                "NotoSansSC-CJK.fixture.ttf",
                "NotoSansArabic.fixture.ttf",
                "NotoSansDevanagari.fixture.ttf",
                "NotoSansHebrew.fixture.ttf",
                "NotoColorEmoji.fixture.ttf",
            ]
            .into_iter()
            .map(source),
        )
        .expect("load isolated fixture catalog")
    }

    fn font_config() -> FontConfig {
        FontConfig::new("Noto Sans")
            .with_fallbacks([
                "Noto Sans SC",
                "Noto Sans Arabic",
                "Noto Sans Devanagari",
                "Noto Sans Hebrew",
                "Noto Color Emoji",
            ])
            .with_font_size(16.0)
            .with_line_height(1.0)
            .with_cell_width(1.0)
    }

    fn multiline_snapshot(text: &str, columns: u16, rows: u16) -> TerminalRenderSnapshot {
        let mut terminal = Terminal::new(TerminalSize::new(columns, rows));
        terminal.feed(b"\x1b[?25l");
        terminal.feed(text.as_bytes());
        TerminalRenderSnapshot::from_terminal(&terminal)
    }
}
