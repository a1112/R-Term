//! Terminal-oriented row shaping and logical/visual mapping.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::ops::Range;

use cosmic_text::{
    Attrs, Buffer, Family, FeatureTag, FontFeatures, Metrics, Shaping, Stretch, Style, Weight, Wrap,
};
use sha2::{Digest, Sha256};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::cache::{BoundedCache, CacheMetrics};
use crate::catalog::{FontCatalog, FontId, is_default_ignorable};
use crate::config::{FontConfig, FontStretch, FontStyle};
use crate::diagnostics::{DiagnosticKind, Diagnostics, FontDiagnostic};
use crate::raster::RasterFlags;

/// Logical terminal cell range.
pub type CellSpan = Range<usize>;

/// Logical grapheme-cluster range.
pub type ClusterSpan = Range<usize>;

/// Primary-face metrics resolved for terminal grid geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerminalFontMetrics {
    /// Shaping font size in logical pixels.
    pub font_size: f32,
    /// Width of one terminal cell after the configured multiplier.
    pub cell_width: f32,
    /// Full row height after the configured multiplier.
    pub line_height: f32,
    /// Distance from row top to the alphabetic baseline.
    pub baseline: f32,
    /// Scaled primary-face ascender.
    pub ascent: f32,
    /// Positive scaled primary-face descender.
    pub descent: f32,
}

/// One terminal-owned grapheme and its authoritative logical cell span.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TerminalCluster {
    /// One complete extended grapheme cluster.
    pub text: String,
    /// Cell geometry established by the terminal model.
    pub cell_span: CellSpan,
    /// Renderer-defined shaping boundary. Adjacent clusters with different
    /// values cannot form one ligature, while the paragraph still shares one
    /// Unicode bidi analysis.
    pub shape_boundary: usize,
    /// Optional per-cluster font weight selected by terminal cell attributes.
    pub weight: Option<u16>,
    /// Optional per-cluster font style selected by terminal cell attributes.
    pub style: Option<FontStyle>,
}

impl TerminalCluster {
    /// Creates a terminal cluster with an explicit cell span.
    #[must_use]
    pub fn new(text: impl Into<String>, cell_span: CellSpan) -> Self {
        Self {
            text: text.into(),
            cell_span,
            shape_boundary: 0,
            weight: None,
            style: None,
        }
    }

    /// Creates a cluster from its starting cell and width.
    #[must_use]
    pub fn with_columns(text: impl Into<String>, start: usize, columns: usize) -> Self {
        Self::new(text, start..start.saturating_add(columns))
    }

    /// Prevents shaping across a renderer style/color/cursor boundary without
    /// splitting the paragraph or restarting bidi analysis.
    ///
    /// The current conservative backend disables standard/contextual
    /// ligatures for the complete row when any adjacent boundary differs.
    /// This preserves one UBA paragraph; per-span feature precision can be
    /// added later without changing this API.
    #[must_use]
    pub const fn with_shape_boundary(mut self, shape_boundary: usize) -> Self {
        self.shape_boundary = shape_boundary;
        self
    }

    #[must_use]
    pub const fn with_weight(mut self, weight: u16) -> Self {
        self.weight = Some(weight);
        self
    }

    #[must_use]
    pub const fn with_style(mut self, style: FontStyle) -> Self {
        self.style = Some(style);
        self
    }
}

/// Input error that would otherwise make the shaping backend panic or mis-map cells.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShapeError {
    /// The isolated catalog contains no usable face.
    NoUsableFont,
    /// A font size, line height, or cell width is zero, non-finite, or negative.
    InvalidMetrics,
    /// A cluster input is empty or contains more than one extended grapheme.
    InvalidCluster,
    /// Authoritative cell spans are empty, overlap, or are not contiguous.
    InvalidCellSpan,
    /// Terminal rows cannot contain CR or LF.
    EmbeddedLineBreak,
}

impl fmt::Display for ShapeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NoUsableFont => "font catalog contains no usable face",
            Self::InvalidMetrics => "font metrics must be finite and greater than zero",
            Self::InvalidCluster => "each input must contain one complete grapheme cluster",
            Self::InvalidCellSpan => "terminal cell spans must be non-empty and contiguous",
            Self::EmbeddedLineBreak => "a terminal row cannot contain CR or LF",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ShapeError {}

/// A glyph positioned for a terminal renderer.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapedGlyph {
    /// Catalog font face used by the shaping engine.
    pub font_id: FontId,
    /// Glyph identifier within the face.
    pub glyph_id: u16,
    /// Backend font size required to reproduce this glyph's raster key.
    pub raster_font_size: f32,
    /// Backend font weight required to reproduce this glyph's raster key.
    pub raster_weight: u16,
    /// Backend raster flags required to reproduce this glyph's raster key.
    pub raster_flags: RasterFlags,
    /// UTF-8 source range covered by the shaping cluster.
    pub byte_range: Range<usize>,
    /// Logical grapheme clusters covered by this glyph.
    pub cluster_range: ClusterSpan,
    /// Logical terminal cells covered by this glyph.
    pub cell_span: CellSpan,
    /// Index in renderer draw order.
    pub visual_order: usize,
    /// Terminal-grid x position.
    pub x: f32,
    /// Baseline-relative y position from the shaping engine.
    pub y: f32,
    /// Terminal-grid width; configured cell geometry wins over glyph advance.
    pub width: f32,
    /// Original proportional x position, retained for diagnostics.
    pub shaping_x: f32,
    /// Original proportional hitbox width, retained for diagnostics.
    pub shaping_width: f32,
    /// Outline x offset reported by the shaping engine.
    pub x_offset: f32,
    /// Outline y offset reported by the shaping engine.
    pub y_offset: f32,
    /// Unicode bidi embedding level.
    pub bidi_level: u8,
    /// Whether the selected face has color tables.
    pub is_color: bool,
    /// Whether this glyph represents an uncovered cluster.
    pub is_tofu: bool,
}

/// One logical extended grapheme cluster in a shaped row.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapedCluster {
    /// UTF-8 source range.
    pub byte_range: Range<usize>,
    /// Logical terminal cell span.
    pub cell_span: CellSpan,
    /// Font selected for the complete cluster.
    pub font_id: FontId,
    /// Canonical selected family.
    pub font_family: String,
    /// Glyph slice touching this cluster.
    pub glyph_range: Range<usize>,
    /// Stable logical cluster index.
    pub logical_index: usize,
    /// Index of this cluster in visual order.
    pub visual_index: usize,
    /// Unicode bidi embedding level.
    pub bidi_level: u8,
    /// Whether no configured font covers the complete cluster.
    pub is_tofu: bool,
}

/// Fully shaped terminal row.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapedRow {
    /// Original logical UTF-8 row.
    pub text: String,
    /// Renderer-ready glyphs in visual order.
    pub glyphs: Vec<ShapedGlyph>,
    /// Grapheme clusters in logical order.
    pub clusters: Vec<ShapedCluster>,
    /// Logical cluster indexes in renderer visual order.
    pub visual_clusters: Vec<usize>,
    /// Total logical terminal cells.
    pub cell_count: usize,
    /// Number of physical layout lines (always one for a terminal row).
    pub layout_line_count: usize,
    /// Catalog generation used for shaping.
    pub catalog_generation: u64,
    /// Primary-face geometry used by the row.
    pub metrics: TerminalFontMetrics,
    /// Instrumented steps in the linear byte/cluster/cell index-building passes.
    ///
    /// This excludes backend shaping, visual sorting, and the cost of hash-table operations.
    pub linear_index_steps: usize,
    /// Deduplicated diagnostics known when this row was shaped.
    pub diagnostics: Vec<FontDiagnostic>,
}

/// Basic cache counters exposed for metrics and invalidation tests.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShapeCacheStats {
    /// Reuses of the most recently shaped row.
    pub hits: u64,
    /// Actual shaping operations.
    pub misses: u64,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ShapeCacheKey {
    catalog_incarnation: u64,
    catalog_generation: u64,
    catalog_fingerprint: [u8; 32],
    request_fingerprint: [u8; 32],
}

#[derive(Clone, Debug)]
struct FaceCandidate {
    font_id: FontId,
    font_family: String,
    is_color: bool,
}

#[derive(Clone, Debug)]
struct ClusterPlan {
    byte_range: Range<usize>,
    cell_span: Range<usize>,
    font_id: FontId,
    font_family: String,
    is_color: bool,
    is_tofu: bool,
    fallback_candidates: Vec<FaceCandidate>,
    shape_boundary: usize,
    weight: u16,
    style: FontStyle,
}

struct LayoutOutput {
    glyphs: Vec<ShapedGlyph>,
    line_count: usize,
    linear_index_steps: usize,
}

#[derive(Clone, Copy, Debug)]
struct CollapsedLayout {
    shaping_x: f32,
    y: f32,
    x_offset: f32,
    y_offset: f32,
    bidi_level: u8,
}

/// Shapes logical terminal rows using an isolated [`FontCatalog`].
pub struct TerminalShaper {
    config: FontConfig,
    diagnostics: Diagnostics,
    cache: BoundedCache<ShapeCacheKey, ShapedRow>,
    cache_scope: Option<(u64, u64, [u8; 32])>,
    stats: ShapeCacheStats,
}

impl TerminalShaper {
    const DEFAULT_CACHE_BUDGET: usize = 8 * 1024 * 1024;

    /// Creates a shaper for one effective terminal font configuration.
    #[must_use]
    pub fn new(config: FontConfig) -> Self {
        Self::with_cache_budget(config, Self::DEFAULT_CACHE_BUDGET)
    }

    /// Creates a shaper with an explicit maximum number of retained cache bytes.
    #[must_use]
    pub fn with_cache_budget(config: FontConfig, budget_bytes: usize) -> Self {
        Self {
            config,
            diagnostics: Diagnostics::default(),
            cache: BoundedCache::new(budget_bytes),
            cache_scope: None,
            stats: ShapeCacheStats::default(),
        }
    }

    /// Replaces the effective configuration and invalidates the row cache.
    pub fn set_config(&mut self, config: FontConfig) {
        if self.config != config {
            self.config = config;
            self.cache.invalidate();
        }
    }

    /// Current cache counters.
    #[must_use]
    pub const fn cache_stats(&self) -> ShapeCacheStats {
        self.stats
    }

    /// Detailed bounded-cache instrumentation.
    #[must_use]
    pub const fn cache_metrics(&self) -> CacheMetrics {
        self.cache.metrics()
    }

    /// Changes the retained-byte budget, evicting least-recently-used rows if needed.
    pub fn set_cache_budget(&mut self, budget_bytes: usize) {
        self.cache.set_budget(budget_bytes);
    }

    /// Shapes one unwrapped terminal row.
    ///
    /// # Errors
    ///
    /// Returns [`ShapeError`] when metrics, row contents, or the catalog are invalid.
    pub fn shape_row(
        &mut self,
        catalog: &mut FontCatalog,
        text: &str,
    ) -> Result<ShapedRow, ShapeError> {
        if text.contains(['\r', '\n']) {
            return Err(ShapeError::EmbeddedLineBreak);
        }
        let mut cell = 0;
        let clusters: Vec<_> = text
            .graphemes(true)
            .map(|cluster| {
                let width = UnicodeWidthStr::width(cluster);
                let width = if width == 0 && !cluster.is_empty() {
                    1
                } else {
                    width
                };
                let input = TerminalCluster::with_columns(cluster, cell, width);
                cell += width;
                input
            })
            .collect();
        self.shape_clusters(catalog, &clusters)
    }

    /// Shapes terminal-owned clusters using their authoritative cell geometry.
    ///
    /// # Errors
    ///
    /// Returns [`ShapeError`] before entering the shaping backend when the
    /// catalog, metrics, clusters, or cell spans are invalid.
    pub fn shape_clusters(
        &mut self,
        catalog: &mut FontCatalog,
        clusters: &[TerminalCluster],
    ) -> Result<ShapedRow, ShapeError> {
        self.validate(catalog, clusters)?;
        let scope = (
            catalog.incarnation(),
            catalog.generation(),
            catalog.fingerprint(),
        );
        if self.cache_scope != Some(scope) {
            if self.cache_scope.is_some() {
                self.cache.invalidate();
            }
            self.cache_scope = Some(scope);
        }
        let key = ShapeCacheKey {
            catalog_incarnation: catalog.incarnation(),
            catalog_generation: catalog.generation(),
            catalog_fingerprint: catalog.fingerprint(),
            request_fingerprint: shape_request_fingerprint(clusters, &self.config),
        };
        if let Some(row) = self.cache.get(&key) {
            self.stats.hits = self.stats.hits.saturating_add(1);
            return Ok(row);
        }
        self.stats.misses = self.stats.misses.saturating_add(1);
        self.diagnostics.begin_row();

        let text: String = clusters
            .iter()
            .map(|cluster| cluster.text.as_str())
            .collect();
        let plans = self.plan_clusters(catalog, clusters);
        let row = self.shape_plans(catalog, text, plans)?;
        let entry_bytes = estimate_shape_entry_bytes(&row);
        if self.cache.can_retain(entry_bytes) {
            self.cache.insert(key, row.clone(), entry_bytes);
        } else {
            self.cache.record_oversize_bypass();
        }
        Ok(row)
    }

    fn validate(
        &self,
        catalog: &FontCatalog,
        clusters: &[TerminalCluster],
    ) -> Result<(), ShapeError> {
        if !self.config.metrics_are_valid() {
            return Err(ShapeError::InvalidMetrics);
        }
        if catalog.face_count() == 0 {
            return Err(ShapeError::NoUsableFont);
        }
        let mut expected_cell = 0;
        for cluster in clusters {
            if cluster.text.contains(['\r', '\n']) {
                return Err(ShapeError::EmbeddedLineBreak);
            }
            if cluster.text.graphemes(true).count() != 1 {
                return Err(ShapeError::InvalidCluster);
            }
            if cluster.cell_span.start != expected_cell
                || cluster.cell_span.end <= cluster.cell_span.start
            {
                return Err(ShapeError::InvalidCellSpan);
            }
            expected_cell = cluster.cell_span.end;
        }
        Ok(())
    }

    fn plan_clusters(
        &mut self,
        catalog: &FontCatalog,
        clusters: &[TerminalCluster],
    ) -> Vec<ClusterPlan> {
        let mut plans = Vec::new();
        let mut byte = 0;
        for input in clusters {
            let cluster = input.text.as_str();
            let byte_range = byte..byte + cluster.len();
            byte = byte_range.end;

            let config = &self.config;
            let weight = input.weight.unwrap_or(config.weight);
            let style = input.style.unwrap_or(config.style);
            let diagnostics = &mut self.diagnostics;
            let mut candidates: Vec<_> = config
                .families()
                .filter_map(|family| {
                    Self::face_candidate(
                        config,
                        diagnostics,
                        catalog,
                        family,
                        cluster,
                        weight,
                        style,
                    )
                })
                .collect();
            let selected = (!candidates.is_empty()).then(|| candidates.remove(0));

            let (font_id, font_family, is_color, is_tofu) = if let Some(candidate) = selected {
                (
                    candidate.font_id,
                    candidate.font_family,
                    candidate.is_color,
                    false,
                )
            } else {
                self.diagnostics.record(FontDiagnostic {
                    kind: DiagnosticKind::MissingCluster,
                    family: None,
                    cluster: Some(cluster.to_owned()),
                    catalog_generation: catalog.generation(),
                });
                let tofu = self
                    .config
                    .families()
                    .find_map(|family| {
                        catalog.record_for_family(family, weight, style, self.config.stretch)
                    })
                    .or_else(|| catalog.first_record())
                    .map_or(
                        (catalog.missing_font_id(), "<missing>".to_owned(), false),
                        |record| {
                            (
                                catalog.font_id(record.id),
                                record.family.clone(),
                                record.is_color,
                            )
                        },
                    );
                self.diagnostics.record(FontDiagnostic {
                    kind: DiagnosticKind::VisibleTofu,
                    family: Some(tofu.1.clone()),
                    cluster: Some(cluster.to_owned()),
                    catalog_generation: catalog.generation(),
                });
                (tofu.0, tofu.1, tofu.2, true)
            };

            plans.push(ClusterPlan {
                byte_range,
                cell_span: input.cell_span.clone(),
                font_id,
                font_family,
                is_color,
                is_tofu,
                fallback_candidates: candidates,
                shape_boundary: input.shape_boundary,
                weight,
                style,
            });
        }
        plans
    }

    fn face_candidate(
        config: &FontConfig,
        diagnostics: &mut Diagnostics,
        catalog: &FontCatalog,
        family: &str,
        cluster: &str,
        weight: u16,
        style: FontStyle,
    ) -> Option<FaceCandidate> {
        let Some(record) = catalog.record_for_family(family, weight, style, config.stretch) else {
            diagnostics.record(FontDiagnostic {
                kind: DiagnosticKind::MissingFamily,
                family: Some(family.to_owned()),
                cluster: None,
                catalog_generation: catalog.generation(),
            });
            return None;
        };
        if catalog.face_metrics(record, config.font_size).is_none() {
            diagnostics.record(FontDiagnostic {
                kind: DiagnosticKind::CorruptFont,
                family: Some(family.to_owned()),
                cluster: None,
                catalog_generation: catalog.generation(),
            });
            return None;
        }
        catalog
            .supports_cluster(record, cluster)
            .then_some(FaceCandidate {
                font_id: catalog.font_id(record.id),
                font_family: record.family.clone(),
                is_color: record.is_color,
            })
    }

    fn shape_plans(
        &mut self,
        catalog: &mut FontCatalog,
        text: String,
        mut plans: Vec<ClusterPlan>,
    ) -> Result<ShapedRow, ShapeError> {
        let metrics = self.resolve_metrics(catalog)?;
        let cell_count = plans.last().map_or(0, |plan| plan.cell_span.end);
        if plans.is_empty() {
            return Ok(ShapedRow {
                text,
                glyphs: Vec::new(),
                clusters: Vec::new(),
                visual_clusters: Vec::new(),
                cell_count: 0,
                layout_line_count: 1,
                catalog_generation: catalog.generation(),
                metrics,
                linear_index_steps: 0,
                diagnostics: self.diagnostics.snapshot(),
            });
        }

        let layout = self.layout_glyphs(catalog, &text, &mut plans, metrics);
        Ok(self.finish_row(catalog, text, plans, layout, cell_count, metrics))
    }

    fn resolve_metrics(&self, catalog: &FontCatalog) -> Result<TerminalFontMetrics, ShapeError> {
        let face = self
            .config
            .families()
            .filter_map(|family| {
                catalog.record_for_family(
                    family,
                    self.config.weight,
                    self.config.style,
                    self.config.stretch,
                )
            })
            .find_map(|record| catalog.face_metrics(record, self.config.font_size))
            .or_else(|| {
                catalog
                    .first_record()
                    .and_then(|record| catalog.face_metrics(record, self.config.font_size))
            })
            .ok_or(ShapeError::InvalidMetrics)?;
        let natural_line_height =
            (face.ascent + face.descent + face.line_gap).max(self.config.font_size);
        let line_height = natural_line_height * self.config.line_height;
        let baseline = face.ascent + (line_height - natural_line_height) / 2.0;
        let metrics = TerminalFontMetrics {
            font_size: self.config.font_size,
            cell_width: face.cell_width * self.config.cell_width,
            line_height,
            baseline,
            ascent: face.ascent,
            descent: face.descent,
        };
        if !metrics.cell_width.is_finite()
            || metrics.cell_width <= 0.0
            || !metrics.line_height.is_finite()
            || metrics.line_height <= 0.0
            || !metrics.baseline.is_finite()
            || metrics.baseline <= 0.0
        {
            return Err(ShapeError::InvalidMetrics);
        }
        Ok(metrics)
    }

    fn layout_glyphs(
        &mut self,
        catalog: &mut FontCatalog,
        text: &str,
        plans: &mut [ClusterPlan],
        metrics: TerminalFontMetrics,
    ) -> LayoutOutput {
        let mut linear_index_steps = 0;
        let mut byte_to_cluster = vec![0; text.len()];
        for (logical_index, plan) in plans.iter().enumerate() {
            for slot in &mut byte_to_cluster[plan.byte_range.clone()] {
                *slot = logical_index;
                linear_index_steps += 1;
            }
        }

        loop {
            let buffer = self.build_buffer(catalog, text, plans, metrics);
            let rejected = rejected_cluster_plans(catalog, text, plans, &buffer, &byte_to_cluster);
            let mut retry = false;
            for logical_index in rejected {
                let plan = &mut plans[logical_index];
                if let Some(candidate) = plan.fallback_candidates.first().cloned() {
                    plan.fallback_candidates.remove(0);
                    plan.font_id = candidate.font_id;
                    plan.font_family = candidate.font_family;
                    plan.is_color = candidate.is_color;
                    retry = true;
                } else {
                    mark_plan_tofu(&mut self.diagnostics, plan, text, catalog.generation());
                }
            }
            if retry {
                continue;
            }

            let (glyphs, line_count, glyph_steps) = self.collect_backend_glyphs(
                catalog,
                text,
                plans,
                &buffer,
                &byte_to_cluster,
                metrics,
            );
            return LayoutOutput {
                glyphs,
                line_count,
                linear_index_steps: linear_index_steps + glyph_steps,
            };
        }
    }

    fn build_buffer(
        &self,
        catalog: &mut FontCatalog,
        text: &str,
        plans: &[ClusterPlan],
        metrics: TerminalFontMetrics,
    ) -> Buffer {
        let mut features = FontFeatures::new();
        for (tag, value) in &self.config.features {
            features.set(FeatureTag::new(tag), *value);
        }
        let has_shape_boundaries = plans
            .windows(2)
            .any(|pair| pair[0].shape_boundary != pair[1].shape_boundary);
        if !self.config.ligatures || has_shape_boundaries {
            features.disable(FeatureTag::STANDARD_LIGATURES);
            features.disable(FeatureTag::CONTEXTUAL_LIGATURES);
        }

        let stretch = match self.config.stretch {
            FontStretch::Condensed => Stretch::Condensed,
            FontStretch::Normal => Stretch::Normal,
            FontStretch::Expanded => Stretch::Expanded,
        };
        let families: Vec<_> = plans.iter().map(|plan| plan.font_family.clone()).collect();
        let spans: Vec<_> = plans
            .iter()
            .zip(&families)
            .map(|(plan, family)| {
                let style = match plan.style {
                    FontStyle::Normal => Style::Normal,
                    FontStyle::Italic => Style::Italic,
                    FontStyle::Oblique => Style::Oblique,
                };
                let attrs = Attrs::new()
                    .family(Family::Name(family))
                    .weight(Weight(plan.weight))
                    .style(style)
                    .stretch(stretch)
                    .font_features(features.clone())
                    .metadata(plan.shape_boundary);
                (&text[plan.byte_range.clone()], attrs)
            })
            .collect();
        let default_attrs = Attrs::new()
            .weight(Weight(self.config.weight))
            .style(match self.config.style {
                FontStyle::Normal => Style::Normal,
                FontStyle::Italic => Style::Italic,
                FontStyle::Oblique => Style::Oblique,
            })
            .stretch(stretch)
            .font_features(features);
        let mut buffer = Buffer::new_empty(Metrics::new(metrics.font_size, metrics.line_height));
        buffer.set_wrap(Wrap::None);
        buffer.set_size(None, None);
        buffer.set_rich_text(spans, &default_attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(catalog.font_system_mut(), false);
        buffer
    }

    fn collect_backend_glyphs(
        &mut self,
        catalog: &FontCatalog,
        text: &str,
        plans: &mut [ClusterPlan],
        buffer: &Buffer,
        byte_to_cluster: &[usize],
        metrics: TerminalFontMetrics,
    ) -> (Vec<ShapedGlyph>, usize, usize) {
        let mut linear_index_steps = 0;
        let mut glyphs = Vec::new();
        let mut plan_has_glyph = vec![false; plans.len()];
        let mut collapsed_layouts = vec![None; plans.len()];
        let mut layout_line_count = 0;
        for run in buffer.layout_runs() {
            layout_line_count += 1;
            for glyph in run.glyphs {
                let cluster_range =
                    indexed_cluster_range(byte_to_cluster, glyph.start..glyph.end, plans.len());
                if plans[cluster_range.clone()].iter().any(|plan| plan.is_tofu) {
                    record_collapsed_layout(
                        &mut collapsed_layouts,
                        cluster_range,
                        CollapsedLayout {
                            shaping_x: glyph.x,
                            y: glyph.y,
                            x_offset: glyph.x_offset,
                            y_offset: glyph.y_offset,
                            bidi_level: glyph.level.number(),
                        },
                    );
                    continue;
                }
                let cell_span = cells_for_clusters(plans, cluster_range.clone());
                linear_index_steps += 1;
                let planned_id = plans[cluster_range.start].font_id;
                let actual_id = catalog.font_id(glyph.font_id);
                let same_planned_face = plans[cluster_range.clone()]
                    .iter()
                    .all(|plan| plan.font_id == planned_id);
                linear_index_steps += cluster_range.end - cluster_range.start;
                let actual_is_valid =
                    same_planned_face && actual_id == planned_id && glyph.glyph_id != 0;
                if !actual_is_valid {
                    record_collapsed_layout(
                        &mut collapsed_layouts,
                        cluster_range.clone(),
                        CollapsedLayout {
                            shaping_x: glyph.x,
                            y: glyph.y,
                            x_offset: glyph.x_offset,
                            y_offset: glyph.y_offset,
                            bidi_level: glyph.level.number(),
                        },
                    );
                    for plan in &mut plans[cluster_range.clone()] {
                        mark_plan_tofu(&mut self.diagnostics, plan, text, catalog.generation());
                    }
                    continue;
                }
                for logical_index in cluster_range.clone() {
                    plan_has_glyph[logical_index] = true;
                }
                let is_color = plans[cluster_range.clone()]
                    .iter()
                    .all(|plan| plan.is_color);
                glyphs.push(shaped_backend_glyph(
                    glyph,
                    actual_id,
                    cluster_range,
                    cell_span,
                    glyphs.len(),
                    is_color,
                ));
            }
        }
        for (logical_index, plan) in plans.iter_mut().enumerate() {
            let cluster = &text[plan.byte_range.clone()];
            let has_visible_scalar = has_visible_scalar(cluster);
            if plan.is_tofu || (!plan_has_glyph[logical_index] && has_visible_scalar) {
                if !plan.is_tofu {
                    mark_plan_tofu(&mut self.diagnostics, plan, text, catalog.generation());
                }
                let visual_order = glyphs.len();
                glyphs.push(synthetic_tofu_glyph(
                    plan,
                    logical_index,
                    collapsed_layouts[logical_index],
                    metrics,
                    plan.weight,
                    visual_order,
                ));
                linear_index_steps += 1;
            }
        }
        (glyphs, layout_line_count, linear_index_steps)
    }

    fn finish_row(
        &self,
        catalog: &FontCatalog,
        text: String,
        plans: Vec<ClusterPlan>,
        layout: LayoutOutput,
        cell_count: usize,
        metrics: TerminalFontMetrics,
    ) -> ShapedRow {
        let mut glyphs = layout.glyphs;
        glyphs.sort_by(|left, right| {
            left.shaping_x
                .total_cmp(&right.shaping_x)
                .then_with(|| left.visual_order.cmp(&right.visual_order))
        });
        let visual_clusters = visual_cluster_order(&glyphs, plans.len());
        let mut visual_indexes = vec![0; plans.len()];
        let mut visual_x = vec![0.0; plans.len()];
        let mut linear_index_steps = layout.linear_index_steps;
        let mut next_x = 0.0;
        for (visual_index, logical_index) in visual_clusters.iter().copied().enumerate() {
            visual_indexes[logical_index] = visual_index;
            visual_x[logical_index] = next_x;
            let cells = plans[logical_index].cell_span.end - plans[logical_index].cell_span.start;
            next_x += cell_pixels(cells, metrics.cell_width);
            linear_index_steps += 1;
        }

        let mut group_bounds: HashMap<(usize, usize), (f32, f32)> = HashMap::new();
        for glyph in &glyphs {
            let key = (glyph.cluster_range.start, glyph.cluster_range.end);
            let bounds = group_bounds
                .entry(key)
                .or_insert((glyph.shaping_x, glyph.shaping_x + glyph.shaping_width));
            bounds.0 = bounds.0.min(glyph.shaping_x);
            bounds.1 = bounds.1.max(glyph.shaping_x + glyph.shaping_width);
        }
        for (visual_order, glyph) in glyphs.iter_mut().enumerate() {
            glyph.visual_order = visual_order;
            let target_x = glyph
                .cluster_range
                .clone()
                .map(|logical_index| visual_x[logical_index])
                .min_by(f32::total_cmp)
                .unwrap_or(0.0);
            let target_width = cell_pixels(
                glyph.cell_span.end - glyph.cell_span.start,
                metrics.cell_width,
            );
            let bounds = group_bounds
                .get(&(glyph.cluster_range.start, glyph.cluster_range.end))
                .copied()
                .unwrap_or((glyph.shaping_x, glyph.shaping_x + glyph.shaping_width));
            let shaping_width = bounds.1 - bounds.0;
            let scale = if shaping_width.is_finite() && shaping_width > 0.0 {
                target_width / shaping_width
            } else {
                1.0
            };
            glyph.x = target_x + (glyph.shaping_x - bounds.0) * scale;
            glyph.width = glyph.shaping_width * scale;
            glyph.x_offset *= scale;
        }

        let mut first_glyph = vec![None; plans.len()];
        let mut last_glyph = vec![None; plans.len()];
        for (glyph_index, glyph) in glyphs.iter().enumerate() {
            for logical_index in glyph.cluster_range.clone() {
                first_glyph[logical_index].get_or_insert(glyph_index);
                last_glyph[logical_index] = Some(glyph_index);
                linear_index_steps += 1;
            }
        }
        let clusters = plans
            .into_iter()
            .enumerate()
            .map(|(logical_index, plan)| {
                let glyph_range = first_glyph[logical_index]
                    .zip(last_glyph[logical_index])
                    .map_or(0..0, |(first, last)| first..last.saturating_add(1));
                let bidi_level =
                    first_glyph[logical_index].map_or(0, |index| glyphs[index].bidi_level);
                ShapedCluster {
                    byte_range: plan.byte_range,
                    cell_span: plan.cell_span,
                    font_id: plan.font_id,
                    font_family: plan.font_family,
                    glyph_range,
                    logical_index,
                    visual_index: visual_indexes[logical_index],
                    bidi_level,
                    is_tofu: plan.is_tofu,
                }
            })
            .collect();

        ShapedRow {
            text,
            glyphs,
            clusters,
            visual_clusters,
            cell_count,
            layout_line_count: layout.line_count,
            catalog_generation: catalog.generation(),
            metrics,
            linear_index_steps,
            diagnostics: self.diagnostics.snapshot(),
        }
    }
}

fn shaped_backend_glyph(
    glyph: &cosmic_text::LayoutGlyph,
    font_id: FontId,
    cluster_range: Range<usize>,
    cell_span: Range<usize>,
    visual_order: usize,
    is_color: bool,
) -> ShapedGlyph {
    ShapedGlyph {
        font_id,
        glyph_id: glyph.glyph_id,
        raster_font_size: glyph.font_size,
        raster_weight: glyph.font_weight.0,
        raster_flags: RasterFlags::from_cosmic(glyph.cache_key_flags),
        byte_range: glyph.start..glyph.end,
        cluster_range,
        cell_span,
        visual_order,
        x: glyph.x,
        y: glyph.y,
        width: glyph.w,
        shaping_x: glyph.x,
        shaping_width: glyph.w,
        x_offset: glyph.x_offset,
        y_offset: glyph.y_offset,
        bidi_level: glyph.level.number(),
        is_color,
        is_tofu: false,
    }
}

fn synthetic_tofu_glyph(
    plan: &ClusterPlan,
    logical_index: usize,
    collapsed: Option<CollapsedLayout>,
    metrics: TerminalFontMetrics,
    weight: u16,
    visual_order: usize,
) -> ShapedGlyph {
    let logical_x = cell_pixels(plan.cell_span.start, metrics.cell_width);
    let shaping_x = collapsed.map_or(logical_x, |layout| layout.shaping_x);
    let shaping_width = cell_pixels(
        plan.cell_span.end - plan.cell_span.start,
        metrics.cell_width,
    );
    ShapedGlyph {
        font_id: plan.font_id,
        glyph_id: 0,
        raster_font_size: metrics.font_size,
        raster_weight: weight,
        raster_flags: RasterFlags::default(),
        byte_range: plan.byte_range.clone(),
        cluster_range: logical_index..logical_index.saturating_add(1),
        cell_span: plan.cell_span.clone(),
        visual_order,
        x: shaping_x,
        y: collapsed.map_or(0.0, |layout| layout.y),
        width: shaping_width,
        shaping_x,
        shaping_width,
        x_offset: collapsed.map_or(0.0, |layout| layout.x_offset),
        y_offset: collapsed.map_or(0.0, |layout| layout.y_offset),
        bidi_level: collapsed.map_or(0, |layout| layout.bidi_level),
        is_color: plan.is_color,
        is_tofu: true,
    }
}

fn record_collapsed_layout(
    layouts: &mut [Option<CollapsedLayout>],
    clusters: Range<usize>,
    candidate: CollapsedLayout,
) {
    for logical_index in clusters {
        let slot = &mut layouts[logical_index];
        if slot.is_none_or(|current| candidate.shaping_x < current.shaping_x) {
            *slot = Some(candidate);
        }
    }
}

fn rejected_cluster_plans(
    catalog: &FontCatalog,
    text: &str,
    plans: &[ClusterPlan],
    buffer: &Buffer,
    byte_to_cluster: &[usize],
) -> Vec<usize> {
    let strict: Vec<_> = plans
        .iter()
        .map(|plan| requires_single_glyph_sequence(&text[plan.byte_range.clone()], plan.is_color))
        .collect();
    let mut glyph_counts = vec![0_usize; plans.len()];
    let mut valid = vec![true; plans.len()];
    for run in buffer.layout_runs() {
        for glyph in run.glyphs {
            let cluster_range =
                indexed_cluster_range(byte_to_cluster, glyph.start..glyph.end, plans.len());
            let same_planned_face = plans[cluster_range.clone()]
                .iter()
                .all(|plan| plan.font_id == plans[cluster_range.start].font_id);
            for logical_index in cluster_range.clone() {
                glyph_counts[logical_index] += 1;
                valid[logical_index] &= same_planned_face
                    && catalog.font_id(glyph.font_id) == plans[logical_index].font_id
                    && glyph.glyph_id != 0;
                if strict[logical_index] {
                    valid[logical_index] &= cluster_range.start == logical_index
                        && cluster_range.end == logical_index + 1
                        && glyph.start <= plans[logical_index].byte_range.start
                        && glyph.end >= plans[logical_index].byte_range.end;
                }
            }
        }
    }

    plans
        .iter()
        .enumerate()
        .filter_map(|(logical_index, plan)| {
            if plan.is_tofu {
                return None;
            }
            let cluster = &text[plan.byte_range.clone()];
            let has_visible_scalar = has_visible_scalar(cluster);
            let wrong_glyph_count = if strict[logical_index] {
                glyph_counts[logical_index] != 1
            } else {
                glyph_counts[logical_index] == 0 && has_visible_scalar
            };
            (wrong_glyph_count || !valid[logical_index]).then_some(logical_index)
        })
        .collect()
}

fn has_visible_scalar(cluster: &str) -> bool {
    cluster
        .chars()
        .any(|character| !is_default_ignorable(character) && !character.is_whitespace())
}

fn requires_single_glyph_sequence(cluster: &str, is_color: bool) -> bool {
    let regional_indicators = cluster
        .chars()
        .filter(|character| matches!(character, '\u{1f1e6}'..='\u{1f1ff}'))
        .count();
    let has_skin_tone = cluster
        .chars()
        .any(|character| matches!(character, '\u{1f3fb}'..='\u{1f3ff}'));
    let has_keycap = cluster.contains('\u{20e3}');
    let has_vs16 = cluster.contains('\u{fe0f}');
    let has_zwj = cluster.contains('\u{200d}');
    let has_emoji_base = cluster.chars().any(is_emoji_base);
    has_skin_tone
        || has_keycap
        || regional_indicators >= 2
        || (has_vs16 && (has_emoji_base || is_color))
        || (has_zwj && has_emoji_base)
}

fn is_emoji_base(character: char) -> bool {
    matches!(
        character,
        '\u{00a9}'
            | '\u{00ae}'
            | '\u{203c}'..='\u{3299}'
            | '\u{1f000}'..='\u{1faff}'
    )
}

fn mark_plan_tofu(
    diagnostics: &mut Diagnostics,
    plan: &mut ClusterPlan,
    text: &str,
    catalog_generation: u64,
) {
    if !plan.is_tofu {
        let cluster = &text[plan.byte_range.clone()];
        diagnostics.record(FontDiagnostic {
            kind: DiagnosticKind::MissingCluster,
            family: Some(plan.font_family.clone()),
            cluster: Some(cluster.to_owned()),
            catalog_generation,
        });
        diagnostics.record(FontDiagnostic {
            kind: DiagnosticKind::VisibleTofu,
            family: Some(plan.font_family.clone()),
            cluster: Some(cluster.to_owned()),
            catalog_generation,
        });
    }
    plan.is_tofu = true;
}

fn estimate_shape_entry_bytes(row: &ShapedRow) -> usize {
    let mut bytes = std::mem::size_of::<ShapeCacheKey>()
        .saturating_add(std::mem::size_of::<ShapedRow>())
        .saturating_add(std::mem::size_of::<(u64, ShapeCacheKey)>())
        .saturating_add(std::mem::size_of::<usize>() * 4);
    bytes = bytes
        .saturating_add(row.text.capacity())
        .saturating_add(
            row.glyphs
                .capacity()
                .saturating_mul(std::mem::size_of::<ShapedGlyph>()),
        )
        .saturating_add(
            row.clusters
                .capacity()
                .saturating_mul(std::mem::size_of::<ShapedCluster>()),
        )
        .saturating_add(
            row.visual_clusters
                .capacity()
                .saturating_mul(std::mem::size_of::<usize>()),
        );
    for cluster in &row.clusters {
        bytes = bytes.saturating_add(cluster.font_family.capacity());
    }
    for diagnostic in &row.diagnostics {
        bytes = bytes
            .saturating_add(std::mem::size_of::<FontDiagnostic>())
            .saturating_add(diagnostic.family.as_ref().map_or(0, String::capacity))
            .saturating_add(diagnostic.cluster.as_ref().map_or(0, String::capacity));
    }
    bytes
}

fn shape_request_fingerprint(clusters: &[TerminalCluster], config: &FontConfig) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(b"rterm-shape-cache-v1");
    digest.update(clusters.len().to_le_bytes());
    for cluster in clusters {
        digest.update(cluster.text.len().to_le_bytes());
        digest.update(cluster.text.as_bytes());
        digest.update(cluster.cell_span.start.to_le_bytes());
        digest.update(cluster.cell_span.end.to_le_bytes());
        digest.update(cluster.shape_boundary.to_le_bytes());
        digest.update(cluster.weight.unwrap_or_default().to_le_bytes());
        digest.update([cluster.style.map_or(u8::MAX, |style| style as u8)]);
    }
    digest.update(config.primary.len().to_le_bytes());
    digest.update(config.primary.as_bytes());
    digest.update(config.fallbacks.len().to_le_bytes());
    for fallback in &config.fallbacks {
        digest.update(fallback.len().to_le_bytes());
        digest.update(fallback.as_bytes());
    }
    digest.update(config.font_size.to_bits().to_le_bytes());
    digest.update(config.line_height.to_bits().to_le_bytes());
    digest.update(config.cell_width.to_bits().to_le_bytes());
    digest.update(config.weight.to_le_bytes());
    digest.update([config.style as u8, config.stretch as u8]);
    digest.update([u8::from(config.ligatures), config.bidi as u8]);
    digest.update(config.features.len().to_le_bytes());
    for (tag, value) in &config.features {
        digest.update(tag);
        digest.update(value.to_le_bytes());
    }
    digest.finalize().into()
}

fn indexed_cluster_range(
    byte_to_cluster: &[usize],
    bytes: Range<usize>,
    cluster_count: usize,
) -> Range<usize> {
    if cluster_count == 0 {
        return 0..0;
    }
    let first = byte_to_cluster
        .get(bytes.start)
        .copied()
        .unwrap_or(cluster_count - 1);
    let last_byte = bytes.end.saturating_sub(1);
    let last = byte_to_cluster.get(last_byte).copied().unwrap_or(first);
    first.min(last)..first.max(last).saturating_add(1)
}

#[allow(clippy::cast_precision_loss)]
fn cell_pixels(cells: usize, cell_width: f32) -> f32 {
    cells as f32 * cell_width
}

fn cells_for_clusters(plans: &[ClusterPlan], clusters: Range<usize>) -> Range<usize> {
    let Some(first) = plans.get(clusters.start) else {
        return 0..0;
    };
    let last = &plans[clusters.end - 1];
    first.cell_span.start..last.cell_span.end
}

fn visual_cluster_order(glyphs: &[ShapedGlyph], cluster_count: usize) -> Vec<usize> {
    let mut result = Vec::with_capacity(cluster_count);
    let mut seen = HashSet::new();
    for glyph in glyphs {
        let logical: Box<dyn Iterator<Item = usize>> = if glyph.bidi_level % 2 == 0 {
            Box::new(glyph.cluster_range.clone())
        } else {
            Box::new(glyph.cluster_range.clone().rev())
        };
        for cluster in logical {
            if seen.insert(cluster) {
                result.push(cluster);
            }
        }
    }
    for cluster in 0..cluster_count {
        if seen.insert(cluster) {
            result.push(cluster);
        }
    }
    result
}
