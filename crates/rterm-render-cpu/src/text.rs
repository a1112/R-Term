//! CPU reference text rendering driven by the isolated shaping and raster stack.

use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::ops::Range;

use rssh_fonts::{
    FontCatalog, FontConfig, RasterCache, RasterCacheConfig, RasterContent, RasterRequest,
    ShapedRow, TerminalCluster, TerminalShaper,
};
use rssh_terminal::VerticalAlign;

use super::{
    CursorRenderStyle, DamageRegion, ImageDrawLayer, PixelRenderer, Rect, RenderCell,
    RenderGeometry, Surface, TerminalRenderSnapshot, configured_cursor_border, cursor_colors,
    cursor_shape_default_color, effective_cell_colors, fill_default_background,
    render_background_images, render_background_layers, render_cell_background, render_cursor,
    render_snapshot_inline_images_in_z_order, render_text_decorations, source_over_rgba,
    text_foreground_alpha,
};

/// Active text backend for a [`PixelRenderer`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextBackend {
    /// Compatibility path used until an explicit font catalog is installed.
    BitmapEmergency,
    /// Isolated shaping, fallback, rasterization, and CPU alpha composition.
    Shaped,
}

/// Pixel rectangle reported for a shaped cluster.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TextPixelBounds {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Mapping retained by the reference renderer for tests, damage, and diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedClusterBounds {
    pub row: u16,
    pub cell_span: Range<usize>,
    pub pixel_bounds: TextPixelBounds,
}

/// Summary of the most recently completed shaped frame.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CpuTextRenderReport {
    pub shaped_rows: usize,
    pub shape_runs: usize,
    pub shaped_glyphs: usize,
    pub rasterized_glyphs: usize,
    pub color_glyphs: usize,
    pub fallback_glyphs: usize,
    pub bold_glyphs: usize,
    pub italic_glyphs: usize,
    pub cluster_bounds: Vec<RenderedClusterBounds>,
    pub expanded_damage: Vec<DamageRegion>,
}

/// Single owner of the catalog, shaper, rasterizer, and their bounded caches.
///
/// Construction is intentionally explicit. This type never consults fonts
/// installed on the host and never embeds repository fixtures into production.
pub struct CpuTextRenderer {
    catalog: FontCatalog,
    shaper: TerminalShaper,
    raster: RasterCache,
    last_report: Option<CpuTextRenderReport>,
}

impl fmt::Debug for CpuTextRenderer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CpuTextRenderer")
            .field("catalog_generation", &self.catalog.generation())
            .field("shape_cache", &self.shaper.cache_metrics())
            .field("raster_cache", &self.raster.metrics())
            .field("last_report", &self.last_report)
            .finish_non_exhaustive()
    }
}

impl CpuTextRenderer {
    #[must_use]
    pub fn new(catalog: FontCatalog, config: FontConfig, raster_config: RasterCacheConfig) -> Self {
        Self {
            catalog,
            shaper: TerminalShaper::new(config),
            raster: RasterCache::new(raster_config),
            last_report: None,
        }
    }

    #[must_use]
    pub const fn last_report(&self) -> Option<&CpuTextRenderReport> {
        self.last_report.as_ref()
    }

    /// Reports the explicit text path owned by this renderer.
    #[must_use]
    pub const fn text_backend(&self) -> TextBackend {
        TextBackend::Shaped
    }

    /// Extends the catalog through the application's lazy font repository.
    /// Shape and raster caches are scoped by the catalog generation.
    pub fn text_catalog_mut(&mut self) -> &mut FontCatalog {
        &mut self.catalog
    }

    /// Changes the raster DPI/zoom scope and invalidates stale cached images.
    pub fn set_scale(&mut self, dpi_scale: f32, zoom: f32) {
        self.raster.set_scale(dpi_scale, zoom);
    }
}

pub struct RowShapePlan {
    pub clusters: Vec<TerminalCluster>,
    pub styles: Vec<RenderCell>,
    pub run_count: usize,
}

pub(super) fn render_full(
    renderer: &PixelRenderer,
    text: &mut CpuTextRenderer,
    snapshot: &TerminalRenderSnapshot,
    target: &mut [u8],
    geometry: RenderGeometry,
) {
    if invalid_geometry(geometry) {
        return;
    }
    let mut surface = Surface {
        target,
        width: geometry.target_width,
        height: geometry.target_height,
    };
    let visual_columns = snapshot_visual_columns(text, snapshot, geometry);
    render_base_layers(renderer, snapshot, &visual_columns, &mut surface, geometry);
    let (mut report, _footprints) =
        render_shaped_foreground(renderer, text, snapshot, &mut surface, geometry);
    render_top_layers(renderer, text, snapshot, &mut surface, geometry);
    report.expanded_damage.clear();
    text.last_report = Some(report);
}

pub(super) fn render_damage(
    renderer: &PixelRenderer,
    text: &mut CpuTextRenderer,
    snapshot: &TerminalRenderSnapshot,
    damage: &[DamageRegion],
    target: &mut [u8],
    geometry: RenderGeometry,
) {
    if invalid_geometry(geometry) || damage.is_empty() {
        return;
    }

    let columns = geometry.target_width / geometry.cell_width;
    let rows = geometry.target_height / geometry.cell_height;
    let expanded = expand_damage_rows(damage, columns, rows);

    // The CPU implementation is the correctness oracle. Compose a complete
    // isolated frame to preserve every layer's ordering, then publish only
    // the conservatively expanded full-width rows. This makes the write set
    // match `expanded_damage` without retaining a previous frame.
    let mut composed = vec![0; target.len()];
    render_full(renderer, text, snapshot, &mut composed, geometry);
    let Ok(row_stride) = usize::try_from(u64::from(geometry.target_width).saturating_mul(4)) else {
        return;
    };
    for region in &expanded {
        let start_y = u64::from(region.y).saturating_mul(u64::from(geometry.cell_height));
        let end_y = start_y
            .saturating_add(
                u64::from(region.height).saturating_mul(u64::from(geometry.cell_height)),
            )
            .min(u64::from(geometry.target_height));
        for pixel_y in start_y..end_y {
            let Ok(start) = usize::try_from(pixel_y.saturating_mul(row_stride as u64)) else {
                continue;
            };
            let end = start
                .saturating_add(row_stride)
                .min(target.len())
                .min(composed.len());
            if start < end {
                target[start..end].copy_from_slice(&composed[start..end]);
            }
        }
    }
    if let Some(report) = text.last_report.as_mut() {
        report.expanded_damage = expanded;
    }
}

fn invalid_geometry(geometry: RenderGeometry) -> bool {
    geometry.target_width == 0
        || geometry.target_height == 0
        || geometry.cell_width == 0
        || geometry.cell_height == 0
}

fn render_base_layers(
    renderer: &PixelRenderer,
    snapshot: &TerminalRenderSnapshot,
    visual_columns: &HashMap<(u16, u16), u16>,
    surface: &mut Surface<'_>,
    geometry: RenderGeometry,
) {
    let background_rect = Rect {
        x: 0,
        y: 0,
        width: geometry.target_width,
        height: geometry.target_height,
    };
    if renderer.default_background_layers.is_empty() {
        fill_default_background(
            surface,
            renderer.default_background,
            renderer.default_background_gradient.as_ref(),
        );
        render_background_images(
            surface,
            &renderer.default_background_images,
            background_rect,
            snapshot.scrollback_offset(),
            renderer.animation_frame,
            renderer.animation_elapsed_ms,
            geometry.cell_width,
            geometry.cell_height,
        );
    } else {
        surface.fill(renderer.default_background);
        render_background_layers(
            surface,
            &renderer.default_background_layers,
            background_rect,
            snapshot.scrollback_offset(),
            renderer.animation_frame,
            renderer.animation_elapsed_ms,
            geometry.cell_width,
            geometry.cell_height,
        );
    }
    render_snapshot_inline_images_in_z_order(
        surface,
        snapshot,
        ImageDrawLayer::UltraNegative,
        geometry.cell_width,
        geometry.cell_height,
        renderer.animation_frame,
        renderer.animation_elapsed_ms,
    );
    for cell in snapshot.iter_cells() {
        let mut projected = cell.clone();
        projected.column = visual_columns
            .get(&(cell.row, cell.column))
            .copied()
            .unwrap_or(cell.column);
        render_cell_background(
            surface,
            &projected,
            geometry.cell_width,
            geometry.cell_height,
            renderer.bold_brightens_ansi_colors,
            renderer.default_foreground,
            renderer.default_background,
            renderer.ansi_palette.as_ref(),
            renderer.indexed_palette.as_ref(),
        );
    }
    render_snapshot_inline_images_in_z_order(
        surface,
        snapshot,
        ImageDrawLayer::Negative,
        geometry.cell_width,
        geometry.cell_height,
        renderer.animation_frame,
        renderer.animation_elapsed_ms,
    );
}

fn snapshot_visual_columns(
    text: &mut CpuTextRenderer,
    snapshot: &TerminalRenderSnapshot,
    geometry: RenderGeometry,
) -> HashMap<(u16, u16), u16> {
    let columns = u16::try_from(geometry.target_width / geometry.cell_width).unwrap_or(u16::MAX);
    let rows = u16::try_from(geometry.target_height / geometry.cell_height).unwrap_or(u16::MAX);
    let mut result = HashMap::new();
    for row in snapshot
        .iter_cells()
        .filter(|cell| cell.row < rows && !cell.continuation)
        .map(|cell| cell.row)
        .collect::<BTreeSet<_>>()
    {
        let plan = row_shape_plan(snapshot, row, columns);
        let Ok(shaped) = text
            .shaper
            .shape_clusters(&mut text.catalog, &plan.clusters)
        else {
            continue;
        };
        let starts = visual_cell_starts(&shaped);
        for cluster in &shaped.clusters {
            let width = cluster
                .cell_span
                .end
                .saturating_sub(cluster.cell_span.start);
            for offset in 0..width {
                let logical = cluster.cell_span.start.saturating_add(offset);
                let visual = starts[cluster.logical_index].saturating_add(offset);
                result.insert(
                    (row, u16::try_from(logical).unwrap_or(u16::MAX)),
                    u16::try_from(visual).unwrap_or(u16::MAX),
                );
            }
        }
    }
    result
}

fn render_top_layers(
    renderer: &PixelRenderer,
    text: &mut CpuTextRenderer,
    snapshot: &TerminalRenderSnapshot,
    surface: &mut Surface<'_>,
    geometry: RenderGeometry,
) {
    render_snapshot_inline_images_in_z_order(
        surface,
        snapshot,
        ImageDrawLayer::Positive,
        geometry.cell_width,
        geometry.cell_height,
        renderer.animation_frame,
        renderer.animation_elapsed_ms,
    );

    if let Some(cursor) = snapshot.cursor() {
        let cursor_shape = shape_cursor_row(text, snapshot, cursor, geometry);
        let visual_cursor = cursor_shape
            .as_ref()
            .map_or(cursor, |shape| super::RenderCursor {
                column: u16::try_from(shape.visual_column).unwrap_or(u16::MAX),
                ..cursor
            });
        let cursor_cell = snapshot
            .iter_cells()
            .find(|cell| cell.row == cursor.row && cell.column == cursor.column);
        let colors = cursor_colors(
            snapshot,
            cursor,
            renderer.force_reverse_video_cursor,
            renderer.reverse_video_cursor_min_contrast,
            renderer.bold_brightens_ansi_colors,
            renderer.default_foreground,
            renderer.default_background,
            renderer.ansi_palette.as_ref(),
            renderer.indexed_palette.as_ref(),
            cursor_shape_default_color(
                cursor,
                renderer.default_cursor_color,
                renderer.default_cursor_border,
            ),
            renderer.default_cursor_foreground,
        );
        render_cursor(
            surface,
            visual_cursor,
            cursor_cell,
            geometry.cell_width,
            geometry.cell_height,
            CursorRenderStyle {
                blink_visible: renderer.blink_visible,
                opacity_alpha: renderer.cursor_opacity_alpha,
                thickness: renderer.cursor_thickness,
                window_dpi: renderer.window_dpi,
                color: colors.color,
                // Redrawn below from the same shaped/rasterized glyph.
                foreground: None,
                border: configured_cursor_border(
                    snapshot,
                    renderer.force_reverse_video_cursor,
                    renderer.default_cursor_border,
                ),
            },
        );
        let redraw_shaped_foreground = cursor.shape == rssh_terminal::CursorShape::Block
            && (!cursor.blinking || renderer.blink_visible)
            && cursor_cell.is_none_or(|cell| !cell.conceal);
        if redraw_shaped_foreground
            && let (Some(foreground), Some(shape)) = (colors.foreground, cursor_shape)
        {
            redraw_cursor_glyph(
                text,
                surface,
                geometry,
                cursor,
                visual_cursor,
                foreground,
                &shape,
            );
        }
    }
}

struct CursorShape {
    row: ShapedRow,
    visual_column: usize,
}

fn shape_cursor_row(
    text: &mut CpuTextRenderer,
    snapshot: &TerminalRenderSnapshot,
    cursor: super::RenderCursor,
    geometry: RenderGeometry,
) -> Option<CursorShape> {
    let columns = u16::try_from(geometry.target_width / geometry.cell_width).unwrap_or(u16::MAX);
    let plan = row_shape_plan(snapshot, cursor.row, columns);
    let row = text
        .shaper
        .shape_clusters(&mut text.catalog, &plan.clusters)
        .ok()?;
    let visual_starts = visual_cell_starts(&row);
    let logical = row
        .clusters
        .iter()
        .position(|cluster| cluster.cell_span.contains(&usize::from(cursor.column)))?;
    let logical_offset =
        usize::from(cursor.column).saturating_sub(row.clusters[logical].cell_span.start);
    Some(CursorShape {
        visual_column: visual_starts[logical].saturating_add(logical_offset),
        row,
    })
}

#[expect(
    clippy::cast_precision_loss,
    reason = "terminal rows/cell dimensions are bounded by u16 viewport geometry"
)]
fn redraw_cursor_glyph(
    text: &mut CpuTextRenderer,
    surface: &mut Surface<'_>,
    geometry: RenderGeometry,
    logical_cursor: super::RenderCursor,
    visual_cursor: super::RenderCursor,
    foreground: [u8; 4],
    shape: &CursorShape,
) {
    let scale_x = geometry.cell_width as f32 / shape.row.metrics.cell_width;
    let baseline = u32::from(logical_cursor.row) as f32 * geometry.cell_height as f32
        + shape.row.metrics.baseline / shape.row.metrics.line_height * geometry.cell_height as f32;
    for glyph in shape.row.glyphs.iter().filter(|glyph| {
        glyph
            .cell_span
            .contains(&usize::from(logical_cursor.column))
    }) {
        let request = RasterRequest::for_shaped_glyph_at_physical_position(
            &shape.row,
            glyph,
            glyph.x * scale_x,
            baseline,
        );
        let Some(positioned) = text.raster.rasterize_positioned(&mut text.catalog, request) else {
            continue;
        };
        draw_raster(
            surface,
            &positioned.image,
            i64::from(positioned.origin_x) + i64::from(positioned.image.left),
            i64::from(positioned.origin_y) - i64::from(positioned.image.top),
            Rect {
                x: u32::from(visual_cursor.column).saturating_mul(geometry.cell_width),
                y: u32::from(visual_cursor.row).saturating_mul(geometry.cell_height),
                width: geometry.cell_width,
                height: geometry.cell_height,
            },
            foreground,
            u8::MAX,
            u8::MAX,
            false,
        );
    }
}

fn render_shaped_foreground(
    renderer: &PixelRenderer,
    text: &mut CpuTextRenderer,
    snapshot: &TerminalRenderSnapshot,
    surface: &mut Surface<'_>,
    geometry: RenderGeometry,
) -> (CpuTextRenderReport, HashMap<u16, Vec<Range<u16>>>) {
    let columns = u16::try_from(geometry.target_width / geometry.cell_width).unwrap_or(u16::MAX);
    let rows = u16::try_from(geometry.target_height / geometry.cell_height).unwrap_or(u16::MAX);
    let row_numbers = snapshot
        .iter_cells()
        .filter(|cell| cell.row < rows && !cell.continuation)
        .map(|cell| cell.row)
        .collect::<BTreeSet<_>>();
    let mut report = CpuTextRenderReport::default();
    let mut footprints: HashMap<u16, Vec<Range<u16>>> = HashMap::new();

    for row in row_numbers {
        let plan = row_shape_plan(snapshot, row, columns);
        if plan.clusters.is_empty() {
            continue;
        }
        report.shaped_rows += 1;
        report.shape_runs += plan.run_count;
        let Ok(shaped) = text
            .shaper
            .shape_clusters(&mut text.catalog, &plan.clusters)
        else {
            continue;
        };
        report.shaped_glyphs += shaped.glyphs.len();
        let visual_starts = visual_cell_starts(&shaped);
        append_cluster_bounds(&mut report, row, &shaped, &visual_starts, geometry);
        draw_shaped_row(
            renderer,
            text,
            surface,
            geometry,
            row,
            &plan,
            &shaped,
            &visual_starts,
            &mut report,
            &mut footprints,
        );
    }
    (report, footprints)
}

fn append_cluster_bounds(
    report: &mut CpuTextRenderReport,
    row: u16,
    shaped: &ShapedRow,
    visual_starts: &[usize],
    geometry: RenderGeometry,
) {
    for cluster in &shaped.clusters {
        let start = cluster.cell_span.start;
        let end = cluster.cell_span.end;
        let visual_start = visual_starts[cluster.logical_index];
        report.cluster_bounds.push(RenderedClusterBounds {
            row,
            cell_span: start..end,
            pixel_bounds: TextPixelBounds {
                x: u32::try_from(visual_start)
                    .unwrap_or(u32::MAX)
                    .saturating_mul(geometry.cell_width),
                y: u32::from(row).saturating_mul(geometry.cell_height),
                width: u32::try_from(end.saturating_sub(start))
                    .unwrap_or(u32::MAX)
                    .saturating_mul(geometry.cell_width),
                height: geometry.cell_height,
            },
        });
    }
}

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    reason = "the reference text oracle keeps shaping, style, placement, and reporting context explicit"
)]
fn draw_shaped_row(
    renderer: &PixelRenderer,
    text: &mut CpuTextRenderer,
    surface: &mut Surface<'_>,
    geometry: RenderGeometry,
    row: u16,
    plan: &RowShapePlan,
    shaped: &ShapedRow,
    visual_starts: &[usize],
    report: &mut CpuTextRenderReport,
    footprints: &mut HashMap<u16, Vec<Range<u16>>>,
) {
    let scale_x = geometry.cell_width as f32 / shaped.metrics.cell_width;
    let baseline = u32::from(row) as f32 * geometry.cell_height as f32
        + shaped.metrics.baseline / shaped.metrics.line_height * geometry.cell_height as f32;
    for glyph in &shaped.glyphs {
        let style = &plan.styles[glyph.cluster_range.start];
        report.bold_glyphs += usize::from(style.bold);
        report.italic_glyphs += usize::from(style.italic);
        let (foreground, _) = effective_cell_colors(
            style,
            renderer.bold_brightens_ansi_colors,
            renderer.default_foreground,
            renderer.default_background,
            renderer.ansi_palette.as_ref(),
            renderer.indexed_palette.as_ref(),
        );
        let foreground_alpha = text_foreground_alpha(
            style,
            renderer.text_blink_opacity_alpha,
            renderer.rapid_text_blink_opacity_alpha,
        );
        if !style.conceal && foreground_alpha != 0 {
            let logical_x = glyph.x * scale_x;
            let aligned_baseline = vertical_align_baseline(baseline, geometry.cell_height, style);
            let request = RasterRequest::for_shaped_glyph_at_physical_position(
                shaped,
                glyph,
                logical_x,
                aligned_baseline,
            );
            let Some(positioned) = text.raster.rasterize_positioned(&mut text.catalog, request)
            else {
                continue;
            };
            let raster = positioned.image;
            report.rasterized_glyphs += 1;
            if matches!(raster.content, RasterContent::Rgba(_)) {
                report.color_glyphs += 1;
            }
            if raster.fallback.is_some() {
                report.fallback_glyphs += 1;
            }
            let clip = Rect {
                x: 0,
                y: u32::from(row).saturating_mul(geometry.cell_height),
                width: geometry.target_width,
                height: geometry.cell_height,
            };
            draw_raster(
                surface,
                &raster,
                i64::from(positioned.origin_x) + i64::from(raster.left),
                i64::from(positioned.origin_y) - i64::from(raster.top),
                clip,
                foreground,
                foreground_alpha,
                foreground_alpha,
                style.faint,
            );
            footprints.entry(row).or_default().push(
                u16::try_from(glyph.cell_span.start).unwrap_or(u16::MAX)
                    ..u16::try_from(glyph.cell_span.end).unwrap_or(u16::MAX),
            );
        }
    }

    for cluster in &shaped.clusters {
        let style = &plan.styles[cluster.logical_index];
        if style.conceal {
            continue;
        }
        let (foreground, _) = effective_cell_colors(
            style,
            renderer.bold_brightens_ansi_colors,
            renderer.default_foreground,
            renderer.default_background,
            renderer.ansi_palette.as_ref(),
            renderer.indexed_palette.as_ref(),
        );
        let foreground_alpha = text_foreground_alpha(
            style,
            renderer.text_blink_opacity_alpha,
            renderer.rapid_text_blink_opacity_alpha,
        );
        let visual_start = visual_starts[cluster.logical_index];
        let width = cluster
            .cell_span
            .end
            .saturating_sub(cluster.cell_span.start);
        let rect = Rect {
            x: u32::try_from(visual_start)
                .unwrap_or(u32::MAX)
                .saturating_mul(geometry.cell_width),
            y: u32::from(row).saturating_mul(geometry.cell_height),
            width: u32::try_from(width)
                .unwrap_or(u32::MAX)
                .saturating_mul(geometry.cell_width),
            height: geometry.cell_height,
        };
        render_text_decorations(
            surface,
            style,
            rect,
            foreground,
            super::color_to_rgba_with_palette(
                style.underline_color,
                foreground,
                renderer.ansi_palette.as_ref(),
                renderer.indexed_palette.as_ref(),
            ),
            foreground_alpha,
            renderer.underline_thickness,
            renderer.underline_position,
            renderer.strikethrough_position,
            renderer.window_dpi,
        );
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "raster composition requires explicit placement, clipping, color, and opacity state"
)]
fn draw_raster(
    surface: &mut Surface<'_>,
    raster: &rssh_fonts::RasterizedGlyph,
    origin_x: i64,
    origin_y: i64,
    clip: Rect,
    foreground: [u8; 4],
    opacity: u8,
    rgba_opacity: u8,
    rgba_faint: bool,
) {
    for y in 0..raster.height {
        for x in 0..raster.width {
            let draw_x = origin_x.saturating_add(i64::from(x));
            let draw_y = origin_y.saturating_add(i64::from(y));
            if draw_x < i64::from(clip.x)
                || draw_y < i64::from(clip.y)
                || draw_x >= i64::from(clip.x.saturating_add(clip.width))
                || draw_y >= i64::from(clip.y.saturating_add(clip.height))
                || draw_x < 0
                || draw_y < 0
            {
                continue;
            }
            let (Ok(draw_x), Ok(draw_y)) = (u32::try_from(draw_x), u32::try_from(draw_y)) else {
                continue;
            };
            let pixel_index = (y as usize)
                .saturating_mul(raster.width as usize)
                .saturating_add(x as usize);
            match &raster.content {
                RasterContent::Mask(mask) => {
                    let alpha = combine_alpha(mask[pixel_index], opacity, foreground[3]);
                    blend_pixel(surface, draw_x, draw_y, foreground, alpha);
                }
                RasterContent::SubpixelMask(mask) => {
                    let offset = pixel_index.saturating_mul(4);
                    blend_subpixel_pixel(
                        surface,
                        draw_x,
                        draw_y,
                        foreground,
                        [mask[offset], mask[offset + 1], mask[offset + 2]],
                        opacity,
                    );
                }
                RasterContent::Rgba(pixels) => {
                    let offset = pixel_index.saturating_mul(4);
                    let mut color = [
                        pixels[offset],
                        pixels[offset + 1],
                        pixels[offset + 2],
                        pixels[offset + 3],
                    ];
                    if rgba_faint {
                        color[0] /= 2;
                        color[1] /= 2;
                        color[2] /= 2;
                    }
                    color[3] = combine_alpha(color[3], rgba_opacity, u8::MAX);
                    blend_rgba_pixel(surface, draw_x, draw_y, color);
                }
            }
        }
    }
}

fn blend_subpixel_pixel(
    surface: &mut Surface<'_>,
    x: u32,
    y: u32,
    foreground: [u8; 4],
    coverage: [u8; 3],
    opacity: u8,
) {
    if x >= surface.width || y >= surface.height {
        return;
    }
    let index = ((y * surface.width + x) * 4) as usize;
    let Some(pixel) = surface.target.get_mut(index..index + 4) else {
        return;
    };
    for channel in 0..3 {
        let alpha = combine_alpha(coverage[channel], opacity, foreground[3]);
        pixel[channel] = super::blend_channel(
            foreground[channel],
            pixel[channel],
            u16::from(alpha),
            u16::from(u8::MAX - alpha),
        );
    }
    pixel[3] = u8::MAX;
}

fn combine_alpha(coverage: u8, opacity: u8, color_alpha: u8) -> u8 {
    let value = u32::from(coverage)
        .saturating_mul(u32::from(opacity))
        .saturating_mul(u32::from(color_alpha))
        / (u32::from(u8::MAX) * u32::from(u8::MAX));
    u8::try_from(value).unwrap_or(u8::MAX)
}

fn blend_pixel(surface: &mut Surface<'_>, x: u32, y: u32, color: [u8; 4], alpha: u8) {
    if alpha == 0 || x >= surface.width || y >= surface.height {
        return;
    }
    let index = ((y * surface.width + x) * 4) as usize;
    let Some(pixel) = surface.target.get_mut(index..index + 4) else {
        return;
    };
    let background = [pixel[0], pixel[1], pixel[2], pixel[3]];
    let foreground = [color[0], color[1], color[2], alpha];
    pixel.copy_from_slice(&source_over_rgba(background, foreground));
}

fn blend_rgba_pixel(surface: &mut Surface<'_>, x: u32, y: u32, foreground: [u8; 4]) {
    if foreground[3] == 0 || x >= surface.width || y >= surface.height {
        return;
    }
    let index = ((y * surface.width + x) * 4) as usize;
    let Some(pixel) = surface.target.get_mut(index..index + 4) else {
        return;
    };
    let background = [pixel[0], pixel[1], pixel[2], pixel[3]];
    pixel.copy_from_slice(&source_over_rgba(background, foreground));
}

#[must_use]
pub fn row_shape_plan(snapshot: &TerminalRenderSnapshot, row: u16, columns: u16) -> RowShapePlan {
    let mut clusters = snapshot.terminal_clusters_for_row(row, columns);
    let cells = snapshot
        .iter_cells()
        .filter(|cell| cell.row == row)
        .map(|cell| (usize::from(cell.column), cell))
        .collect::<HashMap<_, _>>();
    let last_rendered_cell = cells
        .values()
        .filter(|cell| !cell.continuation)
        .map(|cell| usize::from(cell.column).saturating_add(usize::from(cell.columns.max(1))))
        .max()
        .unwrap_or(0);
    clusters.retain(|cluster| cluster.cell_span.start < last_rendered_cell);
    let mut styles = Vec::with_capacity(clusters.len());
    let mut previous_style: Option<RenderCell> = None;
    let mut previous_cursor = false;
    let mut boundary = 0_usize;
    let cursor = snapshot.cursor();
    for cluster in &mut clusters {
        let style = cells
            .get(&cluster.cell_span.start)
            .copied()
            .cloned()
            .unwrap_or_else(|| blank_cell(row, cluster.cell_span.start));
        let cursor_here = cursor.is_some_and(|cursor| {
            cursor.row == row && cluster.cell_span.contains(&usize::from(cursor.column))
        });
        if previous_style.as_ref().is_some_and(|previous| {
            !same_shape_run_style(previous, &style) || cursor_here != previous_cursor
        }) {
            boundary = boundary.saturating_add(1);
        }
        cluster.shape_boundary = boundary;
        if style.bold {
            cluster.weight = Some(700);
        }
        if style.italic {
            cluster.style = Some(rssh_fonts::FontStyle::Italic);
        }
        previous_style = Some(style.clone());
        previous_cursor = cursor_here;
        styles.push(style);
    }
    RowShapePlan {
        clusters,
        styles,
        run_count: boundary.saturating_add(1),
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "cell height is bounded by physical viewport dimensions"
)]
#[must_use]
pub fn vertical_align_baseline(baseline: f32, cell_height: u32, style: &RenderCell) -> f32 {
    let offset = (cell_height / 4).max(1) as f32;
    match style.vertical_align {
        VerticalAlign::Baseline => baseline,
        VerticalAlign::Superscript => baseline - offset,
        VerticalAlign::Subscript => baseline + offset,
    }
}

fn blank_cell(row: u16, column: usize) -> RenderCell {
    RenderCell::new(row, u16::try_from(column).unwrap_or(u16::MAX), " ")
}

fn same_shape_run_style(left: &RenderCell, right: &RenderCell) -> bool {
    left.foreground == right.foreground
        && left.background == right.background
        && left.underline_color == right.underline_color
        && left.underline_style == right.underline_style
        && left.bold == right.bold
        && left.faint == right.faint
        && left.italic == right.italic
        && left.blink == right.blink
        && left.rapid_blink == right.rapid_blink
        && left.underline == right.underline
        && left.double_underline == right.double_underline
        && left.conceal == right.conceal
        && left.strikethrough == right.strikethrough
        && left.overline == right.overline
        && left.vertical_align == right.vertical_align
        && left.inverse == right.inverse
}

#[must_use]
pub fn visual_cell_starts(shaped: &ShapedRow) -> Vec<usize> {
    let mut starts = vec![0; shaped.clusters.len()];
    let mut next = 0_usize;
    for logical in &shaped.visual_clusters {
        starts[*logical] = next;
        let cluster = &shaped.clusters[*logical];
        next = next.saturating_add(
            cluster
                .cell_span
                .end
                .saturating_sub(cluster.cell_span.start),
        );
    }
    starts
}

#[must_use]
pub fn expand_damage_rows(damage: &[DamageRegion], columns: u32, rows: u32) -> Vec<DamageRegion> {
    let columns = u16::try_from(columns).unwrap_or(u16::MAX);
    let rows = u16::try_from(rows).unwrap_or(u16::MAX);
    let mut expanded = Vec::new();
    for region in damage.iter().copied().filter(|region| !region.is_empty()) {
        let row_end = region.y.saturating_add(region.height).min(rows);
        for row in region.y.min(rows)..row_end {
            if columns > 0 {
                expanded.push(DamageRegion::new(0, row, columns, 1));
            }
        }
    }
    expanded
}
