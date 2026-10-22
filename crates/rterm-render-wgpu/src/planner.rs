//! CPU-side display-list planning for the WGPU backend.

use std::{
    cell::{Cell, RefCell},
    ops::{Deref, DerefMut},
    sync::Arc,
};

use rterm_render_cpu::{
    DecodedImage, PixelRenderer, RenderBackgroundGradient, RenderBackgroundImage,
    RenderBackgroundImageAttachment, RenderBackgroundLayer, RenderBoldBrightensAnsiColors,
    RenderCell, RenderCursor, RenderCursorThickness, RenderGeometry, RenderIndexedPalette,
    RenderStrikethroughPosition, RenderUnderlinePosition, RenderUnderlineThickness,
    SCROLLBAR_THUMB_COLOR, SCROLLBAR_TRACK_COLOR, SCROLLBAR_WIDTH, ScrollbackScrollbar,
    TerminalRenderSnapshot, color_to_rgba_with_palette, configured_cursor_border, cursor_colors,
    cursor_rect, cursor_shape_default_color, effective_cell_colors, effective_underline_style,
    scrollbar_thumb_rect, strikethrough_position_px, text_foreground_alpha, underline_position_px,
    underline_thickness_px,
};
use rterm_terminal::{CursorShape, UnderlineStyle};

use crate::gpu;

#[derive(Debug, Clone, PartialEq, Eq)]
struct GpuBackgroundPlanKey {
    config_generation: u64,
    width: u32,
    height: u32,
    cell_width: u32,
    cell_height: u32,
    scrollback_offset: Option<usize>,
    animation_frames: Vec<usize>,
}

#[derive(Debug, Clone)]
struct CachedGpuBackgroundPlan {
    key: GpuBackgroundPlanKey,
    decoded: Arc<DecodedImage>,
    texture: gpu::TextureIdentity,
}

#[derive(Debug)]
pub struct GpuFramePlanner {
    renderer: PixelRenderer,
    cached: RefCell<Option<CachedGpuBackgroundPlan>>,
    updates: Cell<u64>,
    budget_rejections: Cell<u64>,
}

impl GpuFramePlanner {
    #[must_use]
    pub fn new(renderer: PixelRenderer) -> Self {
        Self {
            renderer,
            cached: RefCell::new(None),
            updates: Cell::new(0),
            budget_rejections: Cell::new(0),
        }
    }

    /// Returns the CPU renderer used for shared renderer configuration and
    /// software fallback drawing.
    pub fn cpu_mut(&mut self) -> &mut PixelRenderer {
        &mut self.renderer
    }

    pub fn set_default_foreground(&mut self, foreground: [u8; 4]) {
        self.renderer.set_default_foreground(foreground);
    }

    pub fn set_default_background(&mut self, background: [u8; 4]) {
        self.renderer.set_default_background(background);
    }

    pub fn set_default_background_gradient(&mut self, gradient: Option<RenderBackgroundGradient>) {
        self.renderer.set_default_background_gradient(gradient);
    }

    pub fn set_default_background_images(&mut self, images: Vec<RenderBackgroundImage>) {
        self.renderer.set_default_background_images(images);
    }

    pub fn set_default_background_layers(&mut self, layers: Vec<RenderBackgroundLayer>) {
        self.renderer.set_default_background_layers(layers);
    }

    pub fn set_ansi_palette(&mut self, palette: Option<[[u8; 4]; 16]>) {
        self.renderer.set_ansi_palette(palette);
    }

    pub fn set_indexed_palette(&mut self, palette: Option<RenderIndexedPalette>) {
        self.renderer.set_indexed_palette(palette);
    }

    pub fn set_default_cursor_color(&mut self, color: [u8; 4]) {
        self.renderer.set_default_cursor_color(color);
    }

    pub fn set_default_cursor_border(&mut self, color: Option<[u8; 4]>) {
        self.renderer.set_default_cursor_border(color);
    }

    pub fn set_default_cursor_foreground(&mut self, color: Option<[u8; 4]>) {
        self.renderer.set_default_cursor_foreground(color);
    }

    pub fn set_text_blink_opacity(&mut self, opacity: f32) {
        self.renderer.set_text_blink_opacity(opacity);
    }

    pub fn set_rapid_text_blink_opacity(&mut self, opacity: f32) {
        self.renderer.set_rapid_text_blink_opacity(opacity);
    }

    pub fn set_bold_brightens_ansi_colors(&mut self, value: RenderBoldBrightensAnsiColors) {
        self.renderer.set_bold_brightens_ansi_colors(value);
    }

    pub fn set_cursor_thickness(&mut self, value: Option<RenderCursorThickness>) {
        self.renderer.set_cursor_thickness(value);
    }

    pub fn set_underline_thickness(&mut self, value: Option<RenderUnderlineThickness>) {
        self.renderer.set_underline_thickness(value);
    }

    pub fn set_underline_position(&mut self, value: Option<RenderUnderlinePosition>) {
        self.renderer.set_underline_position(value);
    }

    pub fn set_strikethrough_position(&mut self, value: Option<RenderStrikethroughPosition>) {
        self.renderer.set_strikethrough_position(value);
    }

    pub fn set_force_reverse_video_cursor(&mut self, value: bool) {
        self.renderer.set_force_reverse_video_cursor(value);
    }

    pub fn set_reverse_video_cursor_min_contrast(&mut self, value: Option<f64>) {
        self.renderer.set_reverse_video_cursor_min_contrast(value);
    }

    pub fn set_cursor_opacity(&mut self, opacity: f32) {
        self.renderer.set_cursor_opacity(opacity);
    }

    pub fn set_window_dpi(&mut self, dpi: u32) {
        self.renderer.set_window_dpi(dpi);
    }

    pub fn set_animation_elapsed_ms(&mut self, elapsed_ms: u64) {
        self.renderer.set_animation_elapsed_ms(elapsed_ms);
    }

    #[must_use]
    pub fn gpu_background_plan_updates(&self) -> u64 {
        self.updates.get()
    }

    #[must_use]
    pub fn gpu_background_plan_budget_rejections(&self) -> u64 {
        self.budget_rejections.get()
    }

    fn prepared_gpu_background(
        &self,
        snapshot: &TerminalRenderSnapshot,
        geometry: RenderGeometry,
    ) -> Option<CachedGpuBackgroundPlan> {
        let state = self.renderer.state();
        let requires_raster = if state.default_background_layers.is_empty() {
            state.default_background_gradient.is_some()
                || !state.default_background_images.is_empty()
        } else {
            state.default_background_layers.iter().any(|layer| {
                matches!(
                    layer,
                    RenderBackgroundLayer::Gradient(_) | RenderBackgroundLayer::Image(_)
                )
            })
        };
        if !requires_raster || geometry.content_width == 0 || geometry.content_height == 0 {
            return None;
        }
        let images = if state.default_background_layers.is_empty() {
            state.default_background_images.iter().collect::<Vec<_>>()
        } else {
            state
                .default_background_layers
                .iter()
                .filter_map(|layer| match layer {
                    RenderBackgroundLayer::Image(image) => Some(image),
                    RenderBackgroundLayer::Color(_) | RenderBackgroundLayer::Gradient(_) => None,
                })
                .collect::<Vec<_>>()
        };
        let key = GpuBackgroundPlanKey {
            config_generation: state.render_config_generation,
            width: geometry.content_width,
            height: geometry.content_height,
            cell_width: geometry.cell_width,
            cell_height: geometry.cell_height,
            scrollback_offset: images
                .iter()
                .any(|image| image.attachment != RenderBackgroundImageAttachment::Fixed)
                .then(|| snapshot.scrollback_offset()),
            animation_frames: self.renderer.background_animation_frames(),
        };
        if let Some(cached) = self
            .cached
            .borrow()
            .as_ref()
            .filter(|cached| cached.key == key)
        {
            return Some(cached.clone());
        }
        let byte_len = usize::try_from(geometry.content_width)
            .ok()?
            .checked_mul(usize::try_from(geometry.content_height).ok()?)?
            .checked_mul(4)?;
        if byte_len
            .checked_mul(2)
            .is_none_or(|retained| retained > gpu::DEFAULT_GPU_IMAGE_BYTE_BUDGET)
        {
            self.budget_rejections
                .set(self.budget_rejections.get().saturating_add(1));
            return None;
        }
        let decoded = Arc::new(self.renderer.render_background_rgba(snapshot, geometry)?);
        let prepared = CachedGpuBackgroundPlan {
            key,
            texture: gpu::TextureIdentity::from_rgba(
                decoded.width,
                decoded.height,
                Arc::clone(&decoded.pixels),
            ),
            decoded,
        };
        self.cached.replace(Some(prepared.clone()));
        self.updates.set(self.updates.get().saturating_add(1));
        Some(prepared)
    }
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn prepare_gpu_frame(
        &self,
        snapshot: &TerminalRenderSnapshot,
        geometry: RenderGeometry,
        scrollbar: Option<ScrollbackScrollbar>,
        protected_ui_rows: u16,
    ) -> gpu::RenderGraph {
        let state = self.renderer.state();
        let mut graph = gpu::RenderGraph::new(geometry.target_width, geometry.target_height);
        let viewport = gpu::PixelRect::new(
            geometry.content_x,
            geometry.content_y,
            geometry.content_width,
            geometry.content_height,
        );
        let frame_rect = gpu::PixelRect::new(0, 0, geometry.target_width, geometry.target_height);
        if let Some(border) = geometry.frame_border_color {
            // Fill the target first, then draw the one-pixel frame with its
            // corner pixels omitted. This gives the modern chrome a subtle
            // rounded silhouette without changing the content viewport.
            graph.push_quad(gpu::GpuQuad::new(
                gpu::GpuLayer::PaneBackground,
                frame_rect,
                state.default_background,
            ));
            if geometry.target_width > 2 && geometry.target_height > 2 {
                graph.push_quad(gpu::GpuQuad::new(
                    gpu::GpuLayer::PaneBackground,
                    gpu::PixelRect::new(
                        1,
                        1,
                        geometry.target_width.saturating_sub(2),
                        geometry.target_height.saturating_sub(2),
                    ),
                    state.default_background,
                ));
                graph.push_quad(gpu::GpuQuad::new(
                    gpu::GpuLayer::PaneBackground,
                    gpu::PixelRect::new(1, 0, geometry.target_width.saturating_sub(2), 1),
                    border,
                ));
                graph.push_quad(gpu::GpuQuad::new(
                    gpu::GpuLayer::PaneBackground,
                    gpu::PixelRect::new(
                        1,
                        geometry.target_height.saturating_sub(1),
                        geometry.target_width.saturating_sub(2),
                        1,
                    ),
                    border,
                ));
                graph.push_quad(gpu::GpuQuad::new(
                    gpu::GpuLayer::PaneBackground,
                    gpu::PixelRect::new(0, 1, 1, geometry.target_height.saturating_sub(2)),
                    border,
                ));
                graph.push_quad(gpu::GpuQuad::new(
                    gpu::GpuLayer::PaneBackground,
                    gpu::PixelRect::new(
                        geometry.target_width.saturating_sub(1),
                        1,
                        1,
                        geometry.target_height.saturating_sub(2),
                    ),
                    border,
                ));
            } else {
                // There is no room for a rounded corner on a degenerate
                // target; preserve the historical full-frame border there.
                graph.push_quad(gpu::GpuQuad::new(
                    gpu::GpuLayer::PaneBackground,
                    frame_rect,
                    border,
                ));
            }
        } else {
            graph.push_quad(gpu::GpuQuad::new(
                gpu::GpuLayer::PaneBackground,
                frame_rect,
                state.default_background,
            ));
        }
        if let Some(background) = self.prepared_gpu_background(snapshot, geometry) {
            graph.push_background_texture(background.decoded, background.texture, viewport);
        } else {
            for layer in state.default_background_layers {
                if let RenderBackgroundLayer::Color(color) = layer {
                    graph.push_quad(gpu::GpuQuad::new(
                        gpu::GpuLayer::PaneBackground,
                        viewport,
                        *color,
                    ));
                }
            }
        }
        graph.push_snapshot_images(
            snapshot,
            geometry,
            state.animation_frame,
            state.animation_elapsed_ms,
        );
        for cell in snapshot.iter_cells() {
            let (foreground, background) = effective_cell_colors(
                cell,
                state.bold_brightens_ansi_colors,
                state.default_foreground,
                state.default_background,
                state.ansi_palette,
                state.indexed_palette,
            );
            let cell_rect = gpu::PixelRect::new(
                geometry
                    .content_x
                    .saturating_add(u32::from(cell.column).saturating_mul(geometry.cell_width)),
                geometry
                    .content_y
                    .saturating_add(u32::from(cell.row).saturating_mul(geometry.cell_height)),
                geometry.cell_width,
                geometry.cell_height,
            );
            let Some(cell_rect) = cell_rect.intersection(viewport) else {
                continue;
            };
            if background != state.default_background {
                graph.push_quad(gpu::GpuQuad::new(
                    gpu::GpuLayer::CellBackground,
                    cell_rect,
                    background,
                ));
            }
            push_gpu_text_decorations(
                &mut graph,
                cell,
                cell_rect,
                foreground,
                color_to_rgba_with_palette(
                    cell.underline_color,
                    foreground,
                    state.ansi_palette,
                    state.indexed_palette,
                ),
                text_foreground_alpha(
                    cell,
                    state.text_blink_opacity_alpha,
                    state.rapid_text_blink_opacity_alpha,
                ),
                state.underline_thickness,
                state.underline_position,
                state.strikethrough_position,
                state.window_dpi,
            );
        }
        if let Some(cursor) = snapshot.cursor() {
            let colors = cursor_colors(
                snapshot,
                cursor,
                state.force_reverse_video_cursor,
                state.reverse_video_cursor_min_contrast,
                state.bold_brightens_ansi_colors,
                state.default_foreground,
                state.default_background,
                state.ansi_palette,
                state.indexed_palette,
                cursor_shape_default_color(
                    cursor,
                    state.default_cursor_color,
                    state.default_cursor_border,
                ),
                state.default_cursor_foreground,
            );
            push_gpu_cursor(
                &mut graph,
                cursor,
                geometry,
                CursorRenderStyle {
                    blink_visible: state.blink_visible,
                    opacity_alpha: state.cursor_opacity_alpha,
                    thickness: state.cursor_thickness,
                    window_dpi: state.window_dpi,
                    color: colors.color,
                    border: configured_cursor_border(
                        snapshot,
                        state.force_reverse_video_cursor,
                        state.default_cursor_border,
                    ),
                },
            );
        }
        if let Some(scrollbar) = scrollbar {
            push_gpu_scrollbar(
                &mut graph,
                scrollbar,
                geometry,
                protected_ui_rows,
                state.window_dpi,
            );
        }
        if let Some((y, color)) = geometry.frame_separator
            && y < geometry.target_height
            && geometry.target_width > 2
        {
            graph.push_quad(gpu::GpuQuad::new(
                gpu::GpuLayer::TabBar,
                gpu::PixelRect::new(1, y, geometry.target_width.saturating_sub(2), 1),
                color,
            ));
        }
        graph
    }
}

impl Default for GpuFramePlanner {
    fn default() -> Self {
        Self::new(PixelRenderer::new())
    }
}

impl Deref for GpuFramePlanner {
    type Target = PixelRenderer;

    fn deref(&self) -> &Self::Target {
        &self.renderer
    }
}

impl DerefMut for GpuFramePlanner {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.cached.get_mut().take();
        &mut self.renderer
    }
}

#[derive(Clone, Copy)]
struct CursorRenderStyle {
    blink_visible: bool,
    opacity_alpha: u8,
    thickness: Option<RenderCursorThickness>,
    window_dpi: u32,
    color: [u8; 4],
    border: Option<[u8; 4]>,
}

#[expect(
    clippy::too_many_arguments,
    reason = "GPU decorations consume the same authoritative style inputs as the CPU oracle"
)]
fn push_gpu_text_decorations(
    graph: &mut gpu::RenderGraph,
    cell: &RenderCell,
    cell_rect: gpu::PixelRect,
    foreground: [u8; 4],
    underline_color: [u8; 4],
    foreground_alpha: u8,
    underline_thickness: Option<RenderUnderlineThickness>,
    underline_position: Option<RenderUnderlinePosition>,
    strikethrough_position: Option<RenderStrikethroughPosition>,
    window_dpi: u32,
) {
    if cell.conceal || foreground_alpha == 0 {
        return;
    }
    let with_alpha = |mut color: [u8; 4]| {
        color[3] = gpu_modulate_alpha(color[3], foreground_alpha);
        color
    };
    let underline_height =
        underline_thickness_px(underline_thickness, cell_rect.height, window_dpi);
    let lower_y = cell_rect.y.saturating_add(underline_position_px(
        underline_position,
        cell_rect.height,
        underline_height,
        window_dpi,
    ));
    let lower = gpu::PixelRect::new(cell_rect.x, lower_y, cell_rect.width, underline_height);
    let underline_color = with_alpha(underline_color);
    match effective_underline_style(cell) {
        UnderlineStyle::None => {}
        UnderlineStyle::Single => graph.push_quad(gpu::GpuQuad::new(
            gpu::GpuLayer::Underline,
            lower,
            underline_color,
        )),
        UnderlineStyle::Double => {
            graph.push_quad(gpu::GpuQuad::new(
                gpu::GpuLayer::Underline,
                lower,
                underline_color,
            ));
            graph.push_quad(gpu::GpuQuad::new(
                gpu::GpuLayer::Underline,
                gpu::PixelRect::new(
                    lower.x,
                    lower.y.saturating_sub(underline_height.saturating_mul(2)),
                    lower.width,
                    lower.height,
                ),
                underline_color,
            ));
        }
        UnderlineStyle::Curly => {
            for offset in 0..lower.width {
                let wave = (offset / underline_height.max(1)) % 2;
                graph.push_quad(gpu::GpuQuad::new(
                    gpu::GpuLayer::Underline,
                    gpu::PixelRect::new(
                        lower.x.saturating_add(offset),
                        lower.y.saturating_sub(wave),
                        1,
                        lower.height,
                    ),
                    underline_color,
                ));
            }
        }
        UnderlineStyle::Dotted => push_gpu_patterned_line(graph, lower, underline_color, 1, 1),
        UnderlineStyle::Dashed => push_gpu_patterned_line(
            graph,
            lower,
            underline_color,
            underline_height.saturating_mul(3).max(3),
            underline_height.saturating_mul(2).max(2),
        ),
    }
    if cell.overline {
        graph.push_quad(gpu::GpuQuad::new(
            gpu::GpuLayer::Underline,
            gpu::PixelRect::new(cell_rect.x, cell_rect.y, cell_rect.width, underline_height),
            with_alpha(foreground),
        ));
    }
    if cell.strikethrough {
        let strike_y = cell_rect.y.saturating_add(strikethrough_position_px(
            strikethrough_position,
            cell_rect.height,
            underline_height,
            window_dpi,
        ));
        graph.push_quad(gpu::GpuQuad::new(
            gpu::GpuLayer::Strikethrough,
            gpu::PixelRect::new(cell_rect.x, strike_y, cell_rect.width, underline_height),
            with_alpha(foreground),
        ));
    }
}

fn push_gpu_patterned_line(
    graph: &mut gpu::RenderGraph,
    rect: gpu::PixelRect,
    color: [u8; 4],
    segment: u32,
    gap: u32,
) {
    let step = segment.saturating_add(gap).max(1);
    let mut x = 0;
    while x < rect.width {
        graph.push_quad(gpu::GpuQuad::new(
            gpu::GpuLayer::Underline,
            gpu::PixelRect::new(
                rect.x.saturating_add(x),
                rect.y,
                segment.min(rect.width.saturating_sub(x)),
                rect.height,
            ),
            color,
        ));
        x = x.saturating_add(step);
    }
}

fn push_gpu_cursor(
    graph: &mut gpu::RenderGraph,
    cursor: RenderCursor,
    geometry: RenderGeometry,
    style: CursorRenderStyle,
) {
    if cursor.blinking && !style.blink_visible {
        return;
    }
    let rect = cursor_rect(
        cursor.shape,
        geometry
            .content_x
            .saturating_add(u32::from(cursor.column).saturating_mul(geometry.cell_width)),
        geometry
            .content_y
            .saturating_add(u32::from(cursor.row).saturating_mul(geometry.cell_height)),
        geometry.cell_width,
        geometry.cell_height,
        style.thickness,
        style.window_dpi,
    );
    let alpha = if cursor.blinking {
        style.opacity_alpha
    } else {
        u8::MAX
    };
    let mut color = style.color;
    color[3] = gpu_modulate_alpha(color[3], alpha);
    let Some(rect) = gpu::PixelRect::new(rect.x, rect.y, rect.width, rect.height).intersection(
        gpu::PixelRect::new(
            geometry.content_x,
            geometry.content_y,
            geometry.content_width,
            geometry.content_height,
        ),
    ) else {
        return;
    };
    graph.push_quad(gpu::GpuQuad::new(gpu::GpuLayer::Cursor, rect, color));
    if cursor.shape == CursorShape::Block
        && let Some(mut border) = style.border
    {
        border[3] = gpu_modulate_alpha(border[3], alpha);
        let right = rect.x.saturating_add(rect.width.saturating_sub(1));
        let bottom = rect.y.saturating_add(rect.height.saturating_sub(1));
        for edge in [
            gpu::PixelRect::new(rect.x, rect.y, rect.width, 1),
            gpu::PixelRect::new(rect.x, bottom, rect.width, 1),
            gpu::PixelRect::new(rect.x, rect.y, 1, rect.height),
            gpu::PixelRect::new(right, rect.y, 1, rect.height),
        ] {
            graph.push_quad(gpu::GpuQuad::new(gpu::GpuLayer::Cursor, edge, border));
        }
    }
}

fn push_gpu_scrollbar(
    graph: &mut gpu::RenderGraph,
    scrollbar: ScrollbackScrollbar,
    geometry: RenderGeometry,
    protected_ui_rows: u16,
    window_dpi: u32,
) {
    if geometry.content_width == 0 || geometry.content_height == 0 {
        return;
    }
    let protected_height = u32::from(protected_ui_rows)
        .saturating_mul(geometry.cell_height)
        .min(geometry.content_height);
    let clip = gpu::PixelRect::new(
        geometry.content_x,
        geometry.content_y.saturating_add(protected_height),
        geometry.content_width,
        geometry.content_height.saturating_sub(protected_height),
    );
    let track_width = SCROLLBAR_WIDTH.min(geometry.content_width);
    let track = gpu::PixelRect::new(
        geometry
            .content_x
            .saturating_add(geometry.content_width.saturating_sub(track_width)),
        geometry.content_y,
        track_width,
        geometry.content_height,
    );
    if let Some(track) = track.intersection(clip) {
        graph.push_quad(gpu::GpuQuad::new(
            gpu::GpuLayer::Overlay,
            track,
            SCROLLBAR_TRACK_COLOR,
        ));
    }
    let content_geometry = RenderGeometry::new(
        geometry.content_width,
        geometry.content_height,
        geometry.cell_width,
        geometry.cell_height,
    );
    let thumb = scrollbar_thumb_rect(scrollbar, content_geometry, track_width, window_dpi);
    let thumb = gpu::PixelRect::new(
        geometry.content_x.saturating_add(thumb.x),
        geometry.content_y.saturating_add(thumb.y),
        thumb.width,
        thumb.height,
    );
    if let Some(thumb) = thumb.intersection(clip) {
        graph.push_quad(gpu::GpuQuad::new(
            gpu::GpuLayer::Overlay,
            thumb,
            scrollbar.thumb_color.unwrap_or(SCROLLBAR_THUMB_COLOR),
        ));
    }
}

fn gpu_modulate_alpha(left: u8, right: u8) -> u8 {
    u8::try_from((u16::from(left) * u16::from(right)) / 255).unwrap_or(u8::MAX)
}
