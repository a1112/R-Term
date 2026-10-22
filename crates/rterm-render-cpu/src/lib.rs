use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use font8x8::{BASIC_FONTS, UnicodeFonts};
#[cfg(feature = "image-gif")]
use image::AnimationDecoder;
use rterm_terminal::{Color, CursorShape, InlineImageFormat, UnderlineStyle, VerticalAlign};
#[cfg(feature = "image-gif")]
use std::io::Cursor;

#[doc(hidden)]
pub mod text;

pub use text::{
    CpuTextRenderReport, CpuTextRenderer, RenderedClusterBounds, TextBackend, TextPixelBounds,
};

pub use rterm_render_core::{
    AttachmentViewportClip, DEFAULT_DPI, DamageRegion, KITTY_NON_DEFAULT_BACKGROUND_Z_CUTOFF,
    RenderCell, RenderCellColorRole, RenderCursor, RenderGeometry, RenderIndexedPalette,
    RenderInlineImage, RenderInlineImageFragment, RenderRowSnapshot, RenderStyle,
    RuntimeInlineImageFragment, SCROLLBAR_THUMB_COLOR, SCROLLBAR_TRACK_COLOR, SCROLLBAR_WIDTH,
    SnapshotCacheConfig, SnapshotCacheMetrics, TerminalContentDigest, TerminalRenderSnapshot,
    TerminalSnapshotCache, terminal_bytes_content_digest, terminal_snapshot_content_digest,
};

/// Renders a stable bitmap probe for the first terminal row and hashes its
/// exact RGBA bytes with SHA-256.
///
/// The probe deliberately uses the bundled 8x16 bitmap path and a fixed
/// 16-cell region so it is independent of host fonts, DPI, window chrome, and
/// compositor color management. Real GPU readback is verified separately by
/// the renderer's headless wgpu contracts.
#[must_use]
pub fn terminal_first_row_pixel_digest(snapshot: &TerminalRenderSnapshot) -> TerminalContentDigest {
    const CELL_WIDTH: u32 = 8;
    const CELL_HEIGHT: u32 = 16;
    const COLUMNS: u32 = 16;
    const WIDTH: u32 = CELL_WIDTH * COLUMNS;
    let mut pixels = vec![0; usize::try_from(WIDTH * CELL_HEIGHT * 4).unwrap_or(0)];
    PixelRenderer::new().render(
        snapshot,
        &mut pixels,
        WIDTH,
        CELL_HEIGHT,
        CELL_WIDTH,
        CELL_HEIGHT,
    );
    terminal_bytes_content_digest(&pixels)
}
const DEFAULT_ANSI_PALETTE: [[u8; 4]; 16] = [
    [0, 0, 0, 255],
    [205, 49, 49, 255],
    [13, 188, 121, 255],
    [229, 229, 16, 255],
    [36, 114, 200, 255],
    [188, 63, 188, 255],
    [17, 168, 205, 255],
    [229, 229, 229, 255],
    [102, 102, 102, 255],
    [241, 76, 76, 255],
    [35, 209, 139, 255],
    [245, 245, 67, 255],
    [59, 142, 234, 255],
    [214, 112, 214, 255],
    [41, 184, 219, 255],
    [255, 255, 255, 255],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollbackScrollbar {
    pub scrollback_lines: usize,
    pub viewport_rows: u16,
    pub scrollback_offset: usize,
    pub min_thumb_height: Option<RenderScrollbarThumbSize>,
    pub thumb_color: Option<[u8; 4]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderScrollbarThumbSize {
    Pixels(u32),
    Points(u32),
    CellFractionPerMille(u32),
    Percent(u32),
}

impl ScrollbackScrollbar {
    #[must_use]
    pub fn new(
        scrollback_lines: usize,
        viewport_rows: u16,
        scrollback_offset: usize,
    ) -> Option<Self> {
        if scrollback_lines == 0 || viewport_rows == 0 {
            return None;
        }

        Some(Self {
            scrollback_lines,
            viewport_rows,
            scrollback_offset: scrollback_offset.min(scrollback_lines),
            min_thumb_height: None,
            thumb_color: None,
        })
    }

    #[must_use]
    pub const fn with_min_thumb_height_px(mut self, min_thumb_height_px: u32) -> Self {
        self.min_thumb_height = Some(RenderScrollbarThumbSize::Pixels(min_thumb_height_px));
        self
    }

    #[must_use]
    pub const fn with_min_thumb_height(
        mut self,
        min_thumb_height: RenderScrollbarThumbSize,
    ) -> Self {
        self.min_thumb_height = Some(min_thumb_height);
        self
    }

    #[must_use]
    pub const fn with_min_thumb_height_points(mut self, points: u32) -> Self {
        self.min_thumb_height = Some(RenderScrollbarThumbSize::Points(points));
        self
    }

    #[must_use]
    pub const fn with_min_thumb_height_cell_fraction_per_mille(mut self, per_mille: u32) -> Self {
        self.min_thumb_height = Some(RenderScrollbarThumbSize::CellFractionPerMille(per_mille));
        self
    }

    #[must_use]
    pub const fn with_min_thumb_height_percent(mut self, percent: u32) -> Self {
        self.min_thumb_height = Some(RenderScrollbarThumbSize::Percent(percent));
        self
    }

    #[must_use]
    pub const fn with_thumb_color(mut self, thumb_color: [u8; 4]) -> Self {
        self.thumb_color = Some(thumb_color);
        self
    }

    #[must_use]
    pub fn offset_from_pixel_y(self, y: u32, geometry: RenderGeometry) -> usize {
        self.offset_from_pixel_y_with_dpi(y, geometry, DEFAULT_DPI)
    }

    #[must_use]
    pub fn offset_from_pixel_y_with_dpi(
        self,
        y: u32,
        geometry: RenderGeometry,
        window_dpi: u32,
    ) -> usize {
        if geometry.target_height == 0 {
            return self.scrollback_offset;
        }

        let thumb_height = scrollbar_thumb_height(self, geometry, window_dpi);
        let travel = geometry.target_height.saturating_sub(thumb_height);
        if travel == 0 {
            return 0;
        }

        let y = y.min(geometry.target_height.saturating_sub(1));
        let live_distance = u64::from(y)
            .saturating_mul(self.scrollback_lines as u64)
            .saturating_add(u64::from(travel / 2))
            / u64::from(travel);
        let live_distance = usize::try_from(live_distance).unwrap_or(self.scrollback_lines);
        self.scrollback_lines
            .saturating_sub(live_distance.min(self.scrollback_lines))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixelRenderer {
    blink_visible: bool,
    cursor_opacity_alpha: u8,
    text_blink_opacity_alpha: u8,
    rapid_text_blink_opacity_alpha: u8,
    bold_brightens_ansi_colors: RenderBoldBrightensAnsiColors,
    ansi_palette: Option<[[u8; 4]; 16]>,
    indexed_palette: Option<RenderIndexedPalette>,
    cursor_thickness: Option<RenderCursorThickness>,
    underline_thickness: Option<RenderUnderlineThickness>,
    underline_position: Option<RenderUnderlinePosition>,
    strikethrough_position: Option<RenderStrikethroughPosition>,
    force_reverse_video_cursor: bool,
    reverse_video_cursor_min_contrast: Option<u16>,
    default_foreground: [u8; 4],
    default_background: [u8; 4],
    default_background_gradient: Option<RenderBackgroundGradient>,
    default_background_images: Vec<RenderBackgroundImage>,
    default_background_layers: Vec<RenderBackgroundLayer>,
    render_config_generation: u64,
    default_cursor_color: [u8; 4],
    default_cursor_border: Option<[u8; 4]>,
    default_cursor_foreground: Option<[u8; 4]>,
    window_dpi: u32,
    animation_frame: usize,
    animation_elapsed_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RenderBoldBrightensAnsiColors {
    No,
    #[default]
    BrightAndBold,
    BrightOnly,
}

/// Complete color and text-opacity state shared by CPU and GPU glyph painters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextPaintConfig {
    pub default_foreground: [u8; 4],
    pub default_background: [u8; 4],
    pub bold_brightens_ansi_colors: RenderBoldBrightensAnsiColors,
    pub ansi_palette: Option<[[u8; 4]; 16]>,
    pub indexed_palette: Option<RenderIndexedPalette>,
    pub text_blink_opacity_alpha: u8,
    pub rapid_text_blink_opacity_alpha: u8,
}

/// Renderer-neutral read-only state consumed by the WGPU display-list planner.
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct PixelRendererState<'a> {
    pub blink_visible: bool,
    pub cursor_opacity_alpha: u8,
    pub text_blink_opacity_alpha: u8,
    pub rapid_text_blink_opacity_alpha: u8,
    pub bold_brightens_ansi_colors: RenderBoldBrightensAnsiColors,
    pub ansi_palette: Option<&'a [[u8; 4]; 16]>,
    pub indexed_palette: Option<&'a RenderIndexedPalette>,
    pub cursor_thickness: Option<RenderCursorThickness>,
    pub underline_thickness: Option<RenderUnderlineThickness>,
    pub underline_position: Option<RenderUnderlinePosition>,
    pub strikethrough_position: Option<RenderStrikethroughPosition>,
    pub force_reverse_video_cursor: bool,
    pub reverse_video_cursor_min_contrast: Option<u16>,
    pub default_foreground: [u8; 4],
    pub default_background: [u8; 4],
    pub default_background_gradient: Option<&'a RenderBackgroundGradient>,
    pub default_background_images: &'a [RenderBackgroundImage],
    pub default_background_layers: &'a [RenderBackgroundLayer],
    pub default_cursor_color: [u8; 4],
    pub default_cursor_border: Option<[u8; 4]>,
    pub default_cursor_foreground: Option<[u8; 4]>,
    pub window_dpi: u32,
    pub animation_frame: usize,
    pub animation_elapsed_ms: Option<u64>,
    pub render_config_generation: u64,
}

impl Default for TextPaintConfig {
    fn default() -> Self {
        Self {
            default_foreground: default_foreground(),
            default_background: default_background(),
            bold_brightens_ansi_colors: RenderBoldBrightensAnsiColors::BrightAndBold,
            ansi_palette: None,
            indexed_palette: None,
            text_blink_opacity_alpha: u8::MAX,
            rapid_text_blink_opacity_alpha: u8::MAX,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderCursorThickness {
    Pixels(u32),
    Points(u32),
    Percent(u32),
    CellFractionPerMille(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderUnderlineThickness {
    Pixels(u32),
    Points(u32),
    Percent(u32),
    CellFractionPerMille(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderUnderlinePosition {
    Pixels(i32),
    Points(i32),
    Percent(i32),
    CellFractionPerMille(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderStrikethroughPosition {
    Pixels(u32),
    Points(u32),
    Percent(u32),
    CellFractionPerMille(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackgroundGradientOrientation {
    Horizontal,
    Vertical,
    Linear {
        angle_millidegrees: i32,
    },
    Radial {
        cx_millis: u32,
        cy_millis: u32,
        radius_millis: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackgroundGradientInterpolation {
    Linear,
    Basis,
    CatmullRom,
}

impl RenderBackgroundGradientInterpolation {
    const fn to_colorgrad(self) -> colorgrad::Interpolation {
        match self {
            Self::Linear => colorgrad::Interpolation::Linear,
            Self::Basis => colorgrad::Interpolation::Basis,
            Self::CatmullRom => colorgrad::Interpolation::CatmullRom,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackgroundGradientBlend {
    Rgb,
    LinearRgb,
    Hsv,
    Oklab,
}

impl RenderBackgroundGradientBlend {
    const fn to_colorgrad(self) -> colorgrad::BlendMode {
        match self {
            Self::Rgb => colorgrad::BlendMode::Rgb,
            Self::LinearRgb => colorgrad::BlendMode::LinearRgb,
            Self::Hsv => colorgrad::BlendMode::Hsv,
            Self::Oklab => colorgrad::BlendMode::Oklab,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderBackgroundGradientSegment {
    pub size: usize,
    pub smoothness_millis: u32,
}

impl RenderBackgroundGradientSegment {
    fn smoothness(self) -> f64 {
        f64::from(self.smoothness_millis) / 1_000.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackgroundGradientPreset {
    Blues,
    BrBg,
    BuGn,
    BuPu,
    Cividis,
    Cool,
    CubeHelixDefault,
    GnBu,
    Greens,
    Greys,
    Inferno,
    Magma,
    OrRd,
    Oranges,
    PiYg,
    Plasma,
    PrGn,
    PuBu,
    PuBuGn,
    PuOr,
    PuRd,
    Purples,
    Rainbow,
    RdBu,
    RdGy,
    RdPu,
    RdYlBu,
    RdYlGn,
    Reds,
    Sinebow,
    Spectral,
    Turbo,
    Viridis,
    Warm,
    YlGn,
    YlGnBu,
    YlOrBr,
    YlOrRd,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderBackgroundGradient {
    pub orientation: RenderBackgroundGradientOrientation,
    pub interpolation: RenderBackgroundGradientInterpolation,
    pub blend: RenderBackgroundGradientBlend,
    pub noise: Option<usize>,
    pub segment: Option<RenderBackgroundGradientSegment>,
    pub preset: Option<RenderBackgroundGradientPreset>,
    pub opacity_alpha: u8,
    pub blend_with_default_background: bool,
    pub hsb: RenderBackgroundGradientHsb,
    pub colors: Vec<[u8; 4]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderBackgroundImage {
    pub data: Vec<u8>,
    pub opacity_alpha: u8,
    pub hsb: RenderBackgroundGradientHsb,
    pub animation_speed_millis: u32,
    pub attachment: RenderBackgroundImageAttachment,
    pub width: RenderBackgroundImageDimension,
    pub height: RenderBackgroundImageDimension,
    pub repeat_x: RenderBackgroundImageRepeat,
    pub repeat_y: RenderBackgroundImageRepeat,
    pub horizontal_align: RenderBackgroundImageHorizontalAlign,
    pub vertical_align: RenderBackgroundImageVerticalAlign,
    pub horizontal_offset: RenderBackgroundImageLength,
    pub vertical_offset: RenderBackgroundImageLength,
    pub repeat_x_size: Option<RenderBackgroundImageLength>,
    pub repeat_y_size: Option<RenderBackgroundImageLength>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackgroundImageAttachment {
    Fixed,
    Scroll,
    Parallax { factor_millis: i32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderBackgroundLayer {
    Color([u8; 4]),
    Gradient(RenderBackgroundGradient),
    Image(RenderBackgroundImage),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackgroundImageDimension {
    Cover,
    Contain,
    Pixels(u32),
    Percent(u32),
    Cells(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackgroundImageLength {
    Pixels(i32),
    Percent(i32),
    Cells(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackgroundImageRepeat {
    Repeat,
    Mirror,
    NoRepeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackgroundImageHorizontalAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackgroundImageVerticalAlign {
    Top,
    Middle,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderBackgroundGradientHsb {
    pub hue: u16,
    pub saturation: u16,
    pub brightness: u16,
}

impl RenderBackgroundGradientHsb {
    pub const IDENTITY: Self = Self {
        hue: 1_000,
        saturation: 1_000,
        brightness: 1_000,
    };

    const fn is_identity(self) -> bool {
        self.hue == Self::IDENTITY.hue
            && self.saturation == Self::IDENTITY.saturation
            && self.brightness == Self::IDENTITY.brightness
    }
}

#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "sampling positions are normalized ratios whose materialized vector bounds make integer precision loss unobservable"
)]
pub fn background_gradient_color_strings(
    gradient: &RenderBackgroundGradient,
    count: usize,
) -> Option<Vec<String>> {
    let sampler = BackgroundGradientSampler::from_gradient(gradient);
    if sampler.is_empty() {
        return None;
    }

    Some(
        (0..count)
            .map(|index| {
                let position = if count <= 1 {
                    0.0
                } else {
                    index as f64 / (count - 1) as f64
                };
                let color = sampler.color_at(position);
                format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
            })
            .collect(),
    )
}

impl PixelRenderer {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            blink_visible: true,
            cursor_opacity_alpha: u8::MAX,
            text_blink_opacity_alpha: u8::MAX,
            rapid_text_blink_opacity_alpha: u8::MAX,
            bold_brightens_ansi_colors: RenderBoldBrightensAnsiColors::BrightAndBold,
            ansi_palette: None,
            indexed_palette: None,
            cursor_thickness: None,
            underline_thickness: None,
            underline_position: None,
            strikethrough_position: None,
            force_reverse_video_cursor: false,
            reverse_video_cursor_min_contrast: None,
            default_foreground: default_foreground(),
            default_background: default_background(),
            default_background_gradient: None,
            default_background_images: Vec::new(),
            default_background_layers: Vec::new(),
            render_config_generation: 0,
            default_cursor_color: default_foreground(),
            default_cursor_border: None,
            default_cursor_foreground: None,
            window_dpi: DEFAULT_DPI,
            animation_frame: 0,
            animation_elapsed_ms: None,
        }
    }

    #[must_use]
    pub const fn with_blink_visible(blink_visible: bool) -> Self {
        Self {
            blink_visible,
            cursor_opacity_alpha: if blink_visible { u8::MAX } else { 0 },
            text_blink_opacity_alpha: if blink_visible { u8::MAX } else { 0 },
            rapid_text_blink_opacity_alpha: if blink_visible { u8::MAX } else { 0 },
            bold_brightens_ansi_colors: RenderBoldBrightensAnsiColors::BrightAndBold,
            ansi_palette: None,
            indexed_palette: None,
            cursor_thickness: None,
            underline_thickness: None,
            underline_position: None,
            strikethrough_position: None,
            force_reverse_video_cursor: false,
            reverse_video_cursor_min_contrast: None,
            default_foreground: default_foreground(),
            default_background: default_background(),
            default_background_gradient: None,
            default_background_images: Vec::new(),
            default_background_layers: Vec::new(),
            render_config_generation: 0,
            default_cursor_color: default_foreground(),
            default_cursor_border: None,
            default_cursor_foreground: None,
            window_dpi: DEFAULT_DPI,
            animation_frame: 0,
            animation_elapsed_ms: None,
        }
    }

    #[must_use]
    pub fn with_cursor_opacity(opacity: f32) -> Self {
        let mut renderer = Self::new();
        renderer.set_cursor_opacity(opacity);
        renderer
    }

    pub fn set_cursor_opacity(&mut self, opacity: f32) {
        let alpha = opacity_alpha(opacity);
        self.blink_visible = alpha > 0;
        self.cursor_opacity_alpha = alpha;
    }

    #[must_use]
    pub fn with_text_blink_opacity(opacity: f32) -> Self {
        let mut renderer = Self::new();
        renderer.set_text_blink_opacity(opacity);
        renderer
    }

    pub fn set_text_blink_opacity(&mut self, opacity: f32) {
        self.text_blink_opacity_alpha = opacity_alpha(opacity);
    }

    #[must_use]
    pub fn with_rapid_text_blink_opacity(opacity: f32) -> Self {
        let mut renderer = Self::new();
        renderer.set_rapid_text_blink_opacity(opacity);
        renderer
    }

    pub fn set_rapid_text_blink_opacity(&mut self, opacity: f32) {
        self.rapid_text_blink_opacity_alpha = opacity_alpha(opacity);
    }

    #[must_use]
    pub fn with_bold_brightens_ansi_colors(
        bold_brightens_ansi_colors: RenderBoldBrightensAnsiColors,
    ) -> Self {
        let mut renderer = Self::new();
        renderer.set_bold_brightens_ansi_colors(bold_brightens_ansi_colors);
        renderer
    }

    pub fn set_bold_brightens_ansi_colors(
        &mut self,
        bold_brightens_ansi_colors: RenderBoldBrightensAnsiColors,
    ) {
        self.bold_brightens_ansi_colors = bold_brightens_ansi_colors;
    }

    pub fn set_ansi_palette(&mut self, ansi_palette: Option<[[u8; 4]; 16]>) {
        self.ansi_palette = ansi_palette;
    }

    pub fn set_indexed_palette(&mut self, indexed_palette: Option<RenderIndexedPalette>) {
        self.indexed_palette = indexed_palette;
    }

    #[must_use]
    pub fn text_paint_config(&self) -> TextPaintConfig {
        TextPaintConfig {
            default_foreground: self.default_foreground,
            default_background: self.default_background,
            bold_brightens_ansi_colors: self.bold_brightens_ansi_colors,
            ansi_palette: self.ansi_palette,
            indexed_palette: self.indexed_palette,
            text_blink_opacity_alpha: self.text_blink_opacity_alpha,
            rapid_text_blink_opacity_alpha: self.rapid_text_blink_opacity_alpha,
        }
    }

    #[must_use]
    pub fn with_cursor_thickness_px(cursor_thickness_px: u32) -> Self {
        let mut renderer = Self::new();
        renderer.set_cursor_thickness(Some(RenderCursorThickness::Pixels(cursor_thickness_px)));
        renderer
    }

    pub fn set_cursor_thickness_px(&mut self, cursor_thickness_px: Option<u32>) {
        self.set_cursor_thickness(cursor_thickness_px.map(RenderCursorThickness::Pixels));
    }

    #[must_use]
    pub fn with_cursor_thickness_points(points: u32) -> Self {
        let mut renderer = Self::new();
        renderer.set_cursor_thickness(Some(RenderCursorThickness::Points(points)));
        renderer
    }

    #[must_use]
    pub fn with_cursor_thickness_percent(percent: u32) -> Self {
        let mut renderer = Self::new();
        renderer.set_cursor_thickness(Some(RenderCursorThickness::Percent(percent)));
        renderer
    }

    #[must_use]
    pub fn with_cursor_thickness_cell_fraction_per_mille(per_mille: u32) -> Self {
        let mut renderer = Self::new();
        renderer.set_cursor_thickness(Some(RenderCursorThickness::CellFractionPerMille(per_mille)));
        renderer
    }

    pub fn set_cursor_thickness(&mut self, cursor_thickness: Option<RenderCursorThickness>) {
        self.cursor_thickness = cursor_thickness;
    }

    #[must_use]
    pub fn with_underline_thickness_px(underline_thickness_px: u32) -> Self {
        let mut renderer = Self::new();
        renderer.set_underline_thickness(Some(RenderUnderlineThickness::Pixels(
            underline_thickness_px,
        )));
        renderer
    }

    #[must_use]
    pub fn with_underline_thickness_points(points: u32) -> Self {
        let mut renderer = Self::new();
        renderer.set_underline_thickness(Some(RenderUnderlineThickness::Points(points)));
        renderer
    }

    #[must_use]
    pub fn with_underline_thickness_percent(percent: u32) -> Self {
        let mut renderer = Self::new();
        renderer.set_underline_thickness(Some(RenderUnderlineThickness::Percent(percent)));
        renderer
    }

    #[must_use]
    pub fn with_underline_thickness_cell_fraction_per_mille(per_mille: u32) -> Self {
        let mut renderer = Self::new();
        renderer.set_underline_thickness(Some(RenderUnderlineThickness::CellFractionPerMille(
            per_mille,
        )));
        renderer
    }

    pub fn set_underline_thickness(
        &mut self,
        underline_thickness: Option<RenderUnderlineThickness>,
    ) {
        self.underline_thickness = underline_thickness;
    }

    #[must_use]
    pub fn with_underline_position_px(underline_position_px: i32) -> Self {
        let mut renderer = Self::new();
        renderer
            .set_underline_position(Some(RenderUnderlinePosition::Pixels(underline_position_px)));
        renderer
    }

    #[must_use]
    pub fn with_underline_position_points(points: i32) -> Self {
        let mut renderer = Self::new();
        renderer.set_underline_position(Some(RenderUnderlinePosition::Points(points)));
        renderer
    }

    #[must_use]
    pub fn with_underline_position_percent(percent: i32) -> Self {
        let mut renderer = Self::new();
        renderer.set_underline_position(Some(RenderUnderlinePosition::Percent(percent)));
        renderer
    }

    #[must_use]
    pub fn with_underline_position_cell_fraction_per_mille(per_mille: i32) -> Self {
        let mut renderer = Self::new();
        renderer.set_underline_position(Some(RenderUnderlinePosition::CellFractionPerMille(
            per_mille,
        )));
        renderer
    }

    pub fn set_underline_position(&mut self, underline_position: Option<RenderUnderlinePosition>) {
        self.underline_position = underline_position;
    }

    #[must_use]
    pub fn with_strikethrough_position_px(strikethrough_position_px: u32) -> Self {
        let mut renderer = Self::new();
        renderer.set_strikethrough_position(Some(RenderStrikethroughPosition::Pixels(
            strikethrough_position_px,
        )));
        renderer
    }

    #[must_use]
    pub fn with_strikethrough_position_points(points: u32) -> Self {
        let mut renderer = Self::new();
        renderer.set_strikethrough_position(Some(RenderStrikethroughPosition::Points(points)));
        renderer
    }

    #[must_use]
    pub fn with_strikethrough_position_percent(percent: u32) -> Self {
        let mut renderer = Self::new();
        renderer.set_strikethrough_position(Some(RenderStrikethroughPosition::Percent(percent)));
        renderer
    }

    #[must_use]
    pub fn with_strikethrough_position_cell_fraction_per_mille(per_mille: u32) -> Self {
        let mut renderer = Self::new();
        renderer.set_strikethrough_position(Some(
            RenderStrikethroughPosition::CellFractionPerMille(per_mille),
        ));
        renderer
    }

    pub fn set_strikethrough_position(
        &mut self,
        strikethrough_position: Option<RenderStrikethroughPosition>,
    ) {
        self.strikethrough_position = strikethrough_position;
    }

    pub fn set_window_dpi(&mut self, dpi: u32) {
        self.window_dpi = dpi.max(1);
    }

    pub fn set_default_background(&mut self, background: [u8; 4]) {
        if self.default_background != background {
            self.default_background = background;
            self.bump_render_config_generation();
        }
    }

    pub fn set_default_background_gradient(&mut self, gradient: Option<RenderBackgroundGradient>) {
        let gradient =
            gradient.filter(|gradient| gradient.preset.is_some() || !gradient.colors.is_empty());
        if self.default_background_gradient != gradient {
            self.default_background_gradient = gradient;
            self.bump_render_config_generation();
        }
    }

    pub fn set_default_background_image(&mut self, image: Option<RenderBackgroundImage>) {
        self.set_default_background_images(image.into_iter().collect());
    }

    pub fn set_default_background_images(&mut self, images: Vec<RenderBackgroundImage>) {
        let images = images
            .into_iter()
            .filter(|image| !image.data.is_empty())
            .collect();
        if self.default_background_images != images {
            self.default_background_images = images;
            self.bump_render_config_generation();
        }
    }

    pub fn set_default_background_layers(&mut self, layers: Vec<RenderBackgroundLayer>) {
        let layers = layers
            .into_iter()
            .filter(|layer| match layer {
                RenderBackgroundLayer::Color(color) => color[3] != 0,
                RenderBackgroundLayer::Gradient(gradient) => {
                    gradient.preset.is_some() || !gradient.colors.is_empty()
                }
                RenderBackgroundLayer::Image(image) => !image.data.is_empty(),
            })
            .collect();
        if self.default_background_layers != layers {
            self.default_background_layers = layers;
            self.bump_render_config_generation();
        }
    }

    fn bump_render_config_generation(&mut self) {
        self.render_config_generation = self.render_config_generation.saturating_add(1);
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn render_config_generation(&self) -> u64 {
        self.render_config_generation
    }

    #[doc(hidden)]
    #[must_use]
    pub fn state(&self) -> PixelRendererState<'_> {
        PixelRendererState {
            blink_visible: self.blink_visible,
            cursor_opacity_alpha: self.cursor_opacity_alpha,
            text_blink_opacity_alpha: self.text_blink_opacity_alpha,
            rapid_text_blink_opacity_alpha: self.rapid_text_blink_opacity_alpha,
            bold_brightens_ansi_colors: self.bold_brightens_ansi_colors,
            ansi_palette: self.ansi_palette.as_ref(),
            indexed_palette: self.indexed_palette.as_ref(),
            cursor_thickness: self.cursor_thickness,
            underline_thickness: self.underline_thickness,
            underline_position: self.underline_position,
            strikethrough_position: self.strikethrough_position,
            force_reverse_video_cursor: self.force_reverse_video_cursor,
            reverse_video_cursor_min_contrast: self.reverse_video_cursor_min_contrast,
            default_foreground: self.default_foreground,
            default_background: self.default_background,
            default_background_gradient: self.default_background_gradient.as_ref(),
            default_background_images: &self.default_background_images,
            default_background_layers: &self.default_background_layers,
            default_cursor_color: self.default_cursor_color,
            default_cursor_border: self.default_cursor_border,
            default_cursor_foreground: self.default_cursor_foreground,
            window_dpi: self.window_dpi,
            animation_frame: self.animation_frame,
            animation_elapsed_ms: self.animation_elapsed_ms,
            render_config_generation: self.render_config_generation,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn background_animation_frames(&self) -> Vec<usize> {
        self.background_images()
            .into_iter()
            .filter_map(|image| gif_frame_delays_ms(&image.data).map(|delays| (image, delays)))
            .map(|(image, delays)| {
                self.animation_elapsed_ms.map_or_else(
                    || self.animation_frame % delays.len().max(1),
                    |elapsed_ms| {
                        animation_frame_index_for_delays(
                            &delays,
                            background_image_animation_elapsed_ms(
                                elapsed_ms,
                                image.animation_speed_millis,
                            ),
                        )
                    },
                )
            })
            .collect()
    }

    #[doc(hidden)]
    #[must_use]
    pub fn render_background_rgba(
        &self,
        snapshot: &TerminalRenderSnapshot,
        geometry: RenderGeometry,
    ) -> Option<DecodedImage> {
        let requires_raster = if self.default_background_layers.is_empty() {
            self.default_background_gradient.is_some() || !self.default_background_images.is_empty()
        } else {
            self.default_background_layers.iter().any(|layer| {
                matches!(
                    layer,
                    RenderBackgroundLayer::Gradient(_) | RenderBackgroundLayer::Image(_)
                )
            })
        };
        if !requires_raster || geometry.content_width == 0 || geometry.content_height == 0 {
            return None;
        }
        let byte_len = usize::try_from(geometry.content_width)
            .ok()?
            .checked_mul(usize::try_from(geometry.content_height).ok()?)?
            .checked_mul(4)?;
        let mut pixels = Vec::new();
        pixels.try_reserve_exact(byte_len).ok()?;
        pixels.resize(byte_len, 0);
        let mut surface = Surface {
            target: &mut pixels,
            width: geometry.content_width,
            height: geometry.content_height,
        };
        let background_rect = Rect {
            x: 0,
            y: 0,
            width: geometry.content_width,
            height: geometry.content_height,
        };
        if self.default_background_layers.is_empty() {
            fill_default_background(
                &mut surface,
                self.default_background,
                self.default_background_gradient.as_ref(),
            );
            render_background_images(
                &mut surface,
                &self.default_background_images,
                background_rect,
                snapshot.scrollback_offset(),
                self.animation_frame,
                self.animation_elapsed_ms,
                geometry.cell_width,
                geometry.cell_height,
            );
        } else {
            surface.fill(self.default_background);
            render_background_layers(
                &mut surface,
                &self.default_background_layers,
                background_rect,
                snapshot.scrollback_offset(),
                self.animation_frame,
                self.animation_elapsed_ms,
                geometry.cell_width,
                geometry.cell_height,
            );
        }
        Some(DecodedImage {
            width: geometry.content_width,
            height: geometry.content_height,
            pixels: pixels.into(),
        })
    }

    fn background_images(&self) -> Vec<&RenderBackgroundImage> {
        if self.default_background_layers.is_empty() {
            self.default_background_images.iter().collect()
        } else {
            self.default_background_layers
                .iter()
                .filter_map(|layer| match layer {
                    RenderBackgroundLayer::Image(image) => Some(image),
                    RenderBackgroundLayer::Color(_) | RenderBackgroundLayer::Gradient(_) => None,
                })
                .collect()
        }
    }

    pub fn set_default_foreground(&mut self, foreground: [u8; 4]) {
        self.default_foreground = foreground;
    }

    pub fn set_default_cursor_color(&mut self, color: [u8; 4]) {
        self.default_cursor_color = color;
    }

    pub fn set_default_cursor_border(&mut self, color: Option<[u8; 4]>) {
        self.default_cursor_border = color;
    }

    pub fn set_default_cursor_foreground(&mut self, color: Option<[u8; 4]>) {
        self.default_cursor_foreground = color;
    }

    #[must_use]
    pub const fn with_force_reverse_video_cursor(force_reverse_video_cursor: bool) -> Self {
        Self {
            blink_visible: true,
            cursor_opacity_alpha: u8::MAX,
            text_blink_opacity_alpha: u8::MAX,
            rapid_text_blink_opacity_alpha: u8::MAX,
            bold_brightens_ansi_colors: RenderBoldBrightensAnsiColors::BrightAndBold,
            ansi_palette: None,
            indexed_palette: None,
            cursor_thickness: None,
            underline_thickness: None,
            underline_position: None,
            strikethrough_position: None,
            force_reverse_video_cursor,
            reverse_video_cursor_min_contrast: None,
            default_foreground: default_foreground(),
            default_background: default_background(),
            default_background_gradient: None,
            default_background_images: Vec::new(),
            default_background_layers: Vec::new(),
            render_config_generation: 0,
            default_cursor_color: default_foreground(),
            default_cursor_border: None,
            default_cursor_foreground: None,
            window_dpi: DEFAULT_DPI,
            animation_frame: 0,
            animation_elapsed_ms: None,
        }
    }

    pub fn set_force_reverse_video_cursor(&mut self, force_reverse_video_cursor: bool) {
        self.force_reverse_video_cursor = force_reverse_video_cursor;
    }

    #[must_use]
    pub fn with_reverse_video_cursor_min_contrast(mut self, min_contrast: f64) -> Self {
        self.set_reverse_video_cursor_min_contrast(Some(min_contrast));
        self
    }

    pub fn set_reverse_video_cursor_min_contrast(&mut self, min_contrast: Option<f64>) {
        self.reverse_video_cursor_min_contrast = min_contrast.and_then(contrast_ratio_to_centi);
    }

    #[must_use]
    pub const fn with_animation_frame(animation_frame: usize) -> Self {
        Self {
            blink_visible: true,
            cursor_opacity_alpha: u8::MAX,
            text_blink_opacity_alpha: u8::MAX,
            rapid_text_blink_opacity_alpha: u8::MAX,
            bold_brightens_ansi_colors: RenderBoldBrightensAnsiColors::BrightAndBold,
            ansi_palette: None,
            indexed_palette: None,
            cursor_thickness: None,
            underline_thickness: None,
            underline_position: None,
            strikethrough_position: None,
            force_reverse_video_cursor: false,
            reverse_video_cursor_min_contrast: None,
            default_foreground: default_foreground(),
            default_background: default_background(),
            default_background_gradient: None,
            default_background_images: Vec::new(),
            default_background_layers: Vec::new(),
            render_config_generation: 0,
            default_cursor_color: default_foreground(),
            default_cursor_border: None,
            default_cursor_foreground: None,
            window_dpi: DEFAULT_DPI,
            animation_frame,
            animation_elapsed_ms: None,
        }
    }

    #[must_use]
    pub const fn with_animation_elapsed_ms(animation_elapsed_ms: u64) -> Self {
        Self {
            blink_visible: true,
            cursor_opacity_alpha: u8::MAX,
            text_blink_opacity_alpha: u8::MAX,
            rapid_text_blink_opacity_alpha: u8::MAX,
            bold_brightens_ansi_colors: RenderBoldBrightensAnsiColors::BrightAndBold,
            ansi_palette: None,
            indexed_palette: None,
            cursor_thickness: None,
            underline_thickness: None,
            underline_position: None,
            strikethrough_position: None,
            force_reverse_video_cursor: false,
            reverse_video_cursor_min_contrast: None,
            default_foreground: default_foreground(),
            default_background: default_background(),
            default_background_gradient: None,
            default_background_images: Vec::new(),
            default_background_layers: Vec::new(),
            render_config_generation: 0,
            default_cursor_color: default_foreground(),
            default_cursor_border: None,
            default_cursor_foreground: None,
            window_dpi: DEFAULT_DPI,
            animation_frame: 0,
            animation_elapsed_ms: Some(animation_elapsed_ms),
        }
    }

    /// Reports the backend used by the compatibility `render` entry point.
    #[must_use]
    pub const fn text_backend(&self) -> TextBackend {
        TextBackend::BitmapEmergency
    }

    /// Renders with an explicit isolated shaping/raster owner.
    pub fn render_shaped(
        &self,
        text_renderer: &mut CpuTextRenderer,
        snapshot: &TerminalRenderSnapshot,
        target: &mut [u8],
        geometry: RenderGeometry,
    ) {
        text::render_full(self, text_renderer, snapshot, target, geometry);
    }

    /// Repaints damaged rows with an explicit isolated shaping/raster owner.
    pub fn render_damage_shaped(
        &self,
        text_renderer: &mut CpuTextRenderer,
        snapshot: &TerminalRenderSnapshot,
        damage: &[DamageRegion],
        target: &mut [u8],
        geometry: RenderGeometry,
    ) {
        text::render_damage(self, text_renderer, snapshot, damage, target, geometry);
    }

    pub fn set_animation_elapsed_ms(&mut self, animation_elapsed_ms: u64) {
        self.animation_elapsed_ms = Some(animation_elapsed_ms);
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the complete-frame render pipeline stays in draw order so layer ordering remains explicit"
    )]
    pub fn render(
        &self,
        snapshot: &TerminalRenderSnapshot,
        target: &mut [u8],
        target_width: u32,
        target_height: u32,
        cell_width: u32,
        cell_height: u32,
    ) {
        if target_width == 0 || target_height == 0 || cell_width == 0 || cell_height == 0 {
            return;
        }

        let mut surface = Surface {
            target,
            width: target_width,
            height: target_height,
        };

        let background_rect = Rect {
            x: 0,
            y: 0,
            width: target_width,
            height: target_height,
        };
        if self.default_background_layers.is_empty() {
            fill_default_background(
                &mut surface,
                self.default_background,
                self.default_background_gradient.as_ref(),
            );
            render_background_images(
                &mut surface,
                &self.default_background_images,
                background_rect,
                snapshot.scrollback_offset(),
                self.animation_frame,
                self.animation_elapsed_ms,
                cell_width,
                cell_height,
            );
        } else {
            surface.fill(self.default_background);
            render_background_layers(
                &mut surface,
                &self.default_background_layers,
                background_rect,
                snapshot.scrollback_offset(),
                self.animation_frame,
                self.animation_elapsed_ms,
                cell_width,
                cell_height,
            );
        }

        render_snapshot_inline_images_in_z_order(
            &mut surface,
            snapshot,
            ImageDrawLayer::UltraNegative,
            cell_width,
            cell_height,
            self.animation_frame,
            self.animation_elapsed_ms,
        );

        for cell in snapshot.iter_cells() {
            render_cell_background(
                &mut surface,
                cell,
                cell_width,
                cell_height,
                self.bold_brightens_ansi_colors,
                self.default_foreground,
                self.default_background,
                self.ansi_palette.as_ref(),
                self.indexed_palette.as_ref(),
            );
        }

        render_snapshot_inline_images_in_z_order(
            &mut surface,
            snapshot,
            ImageDrawLayer::Negative,
            cell_width,
            cell_height,
            self.animation_frame,
            self.animation_elapsed_ms,
        );

        for cell in snapshot.iter_cells() {
            render_cell_foreground(
                &mut surface,
                cell,
                cell_width,
                cell_height,
                self.text_blink_opacity_alpha,
                self.rapid_text_blink_opacity_alpha,
                self.underline_thickness,
                self.underline_position,
                self.strikethrough_position,
                self.window_dpi,
                self.bold_brightens_ansi_colors,
                self.default_foreground,
                self.default_background,
                self.ansi_palette.as_ref(),
                self.indexed_palette.as_ref(),
            );
        }

        render_snapshot_inline_images_in_z_order(
            &mut surface,
            snapshot,
            ImageDrawLayer::Positive,
            cell_width,
            cell_height,
            self.animation_frame,
            self.animation_elapsed_ms,
        );

        if let Some(cursor) = snapshot.cursor() {
            let cursor_cell = snapshot
                .iter_cells()
                .find(|cell| cell.row == cursor.row && cell.column == cursor.column);
            let cursor_colors = cursor_colors(
                snapshot,
                cursor,
                self.force_reverse_video_cursor,
                self.reverse_video_cursor_min_contrast,
                self.bold_brightens_ansi_colors,
                self.default_foreground,
                self.default_background,
                self.ansi_palette.as_ref(),
                self.indexed_palette.as_ref(),
                cursor_shape_default_color(
                    cursor,
                    self.default_cursor_color,
                    self.default_cursor_border,
                ),
                self.default_cursor_foreground,
            );
            render_cursor(
                &mut surface,
                cursor,
                cursor_cell,
                cell_width,
                cell_height,
                CursorRenderStyle {
                    blink_visible: self.blink_visible,
                    opacity_alpha: self.cursor_opacity_alpha,
                    thickness: self.cursor_thickness,
                    window_dpi: self.window_dpi,
                    color: cursor_colors.color,
                    foreground: cursor_colors.foreground,
                    border: configured_cursor_border(
                        snapshot,
                        self.force_reverse_video_cursor,
                        self.default_cursor_border,
                    ),
                },
            );
        }
    }

    pub fn render_scrollbar(
        &self,
        scrollbar: ScrollbackScrollbar,
        target: &mut [u8],
        geometry: RenderGeometry,
    ) {
        if geometry.target_width == 0 || geometry.target_height == 0 {
            return;
        }

        let mut surface = Surface {
            target,
            width: geometry.target_width,
            height: geometry.target_height,
        };
        let track_width = SCROLLBAR_WIDTH.min(geometry.target_width);
        let track = Rect {
            x: geometry.target_width.saturating_sub(track_width),
            y: 0,
            width: track_width,
            height: geometry.target_height,
        };
        surface.fill_rect(track, SCROLLBAR_TRACK_COLOR);
        surface.fill_rect(
            scrollbar_thumb_rect(scrollbar, geometry, track_width, self.window_dpi),
            scrollbar.thumb_color.unwrap_or(SCROLLBAR_THUMB_COLOR),
        );
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the damage pipeline stays aligned with complete-frame draw order so parity remains reviewable"
    )]
    pub fn render_damage(
        &self,
        snapshot: &TerminalRenderSnapshot,
        damage: &[DamageRegion],
        target: &mut [u8],
        geometry: RenderGeometry,
    ) {
        if geometry.target_width == 0
            || geometry.target_height == 0
            || geometry.cell_width == 0
            || geometry.cell_height == 0
            || damage.is_empty()
        {
            return;
        }

        let mut surface = Surface {
            target,
            width: geometry.target_width,
            height: geometry.target_height,
        };

        for region in damage.iter().copied().filter(|region| !region.is_empty()) {
            let rect = damage_rect(region, geometry.cell_width, geometry.cell_height);
            if self.default_background_layers.is_empty() {
                fill_default_background_rect(
                    &mut surface,
                    rect,
                    self.default_background,
                    self.default_background_gradient.as_ref(),
                );
                render_background_images(
                    &mut surface,
                    &self.default_background_images,
                    rect,
                    snapshot.scrollback_offset(),
                    self.animation_frame,
                    self.animation_elapsed_ms,
                    geometry.cell_width,
                    geometry.cell_height,
                );
            } else {
                surface.fill_rect(rect, self.default_background);
                render_background_layers(
                    &mut surface,
                    &self.default_background_layers,
                    rect,
                    snapshot.scrollback_offset(),
                    self.animation_frame,
                    self.animation_elapsed_ms,
                    geometry.cell_width,
                    geometry.cell_height,
                );
            }
        }

        let damaged_cells = snapshot
            .iter_cells()
            .filter(|cell| damage_contains_cell(damage, cell.row, cell.column))
            .collect::<Vec<_>>();

        render_damaged_snapshot_inline_images_in_z_order(
            &mut surface,
            snapshot,
            ImageDrawLayer::UltraNegative,
            damage,
            geometry,
            self.animation_frame,
            self.animation_elapsed_ms,
        );

        for cell in &damaged_cells {
            render_cell_background(
                &mut surface,
                cell,
                geometry.cell_width,
                geometry.cell_height,
                self.bold_brightens_ansi_colors,
                self.default_foreground,
                self.default_background,
                self.ansi_palette.as_ref(),
                self.indexed_palette.as_ref(),
            );
        }

        render_damaged_snapshot_inline_images_in_z_order(
            &mut surface,
            snapshot,
            ImageDrawLayer::Negative,
            damage,
            geometry,
            self.animation_frame,
            self.animation_elapsed_ms,
        );

        for cell in &damaged_cells {
            render_cell_foreground(
                &mut surface,
                cell,
                geometry.cell_width,
                geometry.cell_height,
                self.text_blink_opacity_alpha,
                self.rapid_text_blink_opacity_alpha,
                self.underline_thickness,
                self.underline_position,
                self.strikethrough_position,
                self.window_dpi,
                self.bold_brightens_ansi_colors,
                self.default_foreground,
                self.default_background,
                self.ansi_palette.as_ref(),
                self.indexed_palette.as_ref(),
            );
        }

        render_damaged_snapshot_inline_images_in_z_order(
            &mut surface,
            snapshot,
            ImageDrawLayer::Positive,
            damage,
            geometry,
            self.animation_frame,
            self.animation_elapsed_ms,
        );

        if let Some(cursor) = snapshot
            .cursor()
            .filter(|cursor| damage_contains_cell(damage, cursor.row, cursor.column))
        {
            let cursor_cell = snapshot
                .iter_cells()
                .find(|cell| cell.row == cursor.row && cell.column == cursor.column);
            let cursor_colors = cursor_colors(
                snapshot,
                cursor,
                self.force_reverse_video_cursor,
                self.reverse_video_cursor_min_contrast,
                self.bold_brightens_ansi_colors,
                self.default_foreground,
                self.default_background,
                self.ansi_palette.as_ref(),
                self.indexed_palette.as_ref(),
                cursor_shape_default_color(
                    cursor,
                    self.default_cursor_color,
                    self.default_cursor_border,
                ),
                self.default_cursor_foreground,
            );
            render_cursor(
                &mut surface,
                cursor,
                cursor_cell,
                geometry.cell_width,
                geometry.cell_height,
                CursorRenderStyle {
                    blink_visible: self.blink_visible,
                    opacity_alpha: self.cursor_opacity_alpha,
                    thickness: self.cursor_thickness,
                    window_dpi: self.window_dpi,
                    color: cursor_colors.color,
                    foreground: cursor_colors.foreground,
                    border: configured_cursor_border(
                        snapshot,
                        self.force_reverse_video_cursor,
                        self.default_cursor_border,
                    ),
                },
            );
        }
    }
}

#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Arc<[u8]>,
}

/// Backend-neutral, fully normalized inline-image draw.
///
/// The CPU and GPU backends derive these draws from the same private snapshot
/// metadata so fragmented parents, cell attachments, viewport transforms, and
/// animation frame selection cannot diverge.
#[derive(Debug, Clone)]
pub struct ImageDrawPlan {
    pub destination_x: u32,
    pub destination_y: u32,
    pub width: u32,
    pub height: u32,
    pub decoded: Arc<DecodedImage>,
    pub sample_source_x: u32,
    pub sample_source_y: u32,
    pub sample_target_x: u32,
    pub sample_target_y: u32,
    pub sample_source_width: u32,
    pub sample_source_height: u32,
    pub sample_destination_width: u32,
    pub sample_destination_height: u32,
    pub z_index: i32,
    pub kitty_image_id: Option<u32>,
    pub parent_index: usize,
    pub fragment_index: usize,
    pub tie_policy: ImageTiePolicy,
    pub stable_order: usize,
}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub enum ImageTiePolicy {
    Whole,
    Fragment,
}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub enum ImageDrawLayer {
    UltraNegative,
    Negative,
    Positive,
}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
struct ImageDrawOrderKey {
    layer: ImageDrawLayer,
    kind: ImageTiePolicy,
    z_index: i32,
    id_group: u8,
    kitty_image_id: u32,
    parent_index: usize,
    fragment_index: usize,
    stable_order: usize,
}

#[must_use]
pub fn compare_image_draw_plans(left: &ImageDrawPlan, right: &ImageDrawPlan) -> Ordering {
    image_draw_order_key(left).cmp(&image_draw_order_key(right))
}

fn image_draw_order_key(plan: &ImageDrawPlan) -> ImageDrawOrderKey {
    // Layer precedes kind so extreme-negative and ordinary-negative images
    // retain their fixed relationship to cell backgrounds and glyphs.
    // Within one layer, whole draws precede fragments exactly as the CPU
    // renderer historically grouped them. Whole images with protocol IDs sort
    // first by ID; missing IDs then preserve snapshot insertion order.
    let (id_group, kitty_image_id, parent_index, fragment_index) = match plan.tie_policy {
        ImageTiePolicy::Whole => (
            u8::from(plan.kitty_image_id.is_none()),
            plan.kitty_image_id.unwrap_or_default(),
            0,
            0,
        ),
        ImageTiePolicy::Fragment => (
            u8::from(plan.kitty_image_id.is_some()),
            plan.kitty_image_id.unwrap_or_default(),
            plan.parent_index,
            plan.fragment_index,
        ),
    };
    ImageDrawOrderKey {
        layer: image_draw_layer(plan.z_index),
        kind: plan.tie_policy,
        z_index: plan.z_index,
        id_group,
        kitty_image_id,
        parent_index,
        fragment_index,
        stable_order: plan.stable_order,
    }
}

const fn image_draw_layer(z_index: i32) -> ImageDrawLayer {
    if z_index < KITTY_NON_DEFAULT_BACKGROUND_Z_CUTOFF {
        ImageDrawLayer::UltraNegative
    } else if z_index < 0 {
        ImageDrawLayer::Negative
    } else {
        ImageDrawLayer::Positive
    }
}

const fn inline_image_pixel_is_drawn(pixel: [u8; 4]) -> bool {
    pixel[3] != 0
}

impl Default for PixelRenderer {
    fn default() -> Self {
        Self::new()
    }
}

struct Surface<'a> {
    target: &'a mut [u8],
    width: u32,
    height: u32,
}

#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy)]
struct CursorRenderStyle {
    blink_visible: bool,
    opacity_alpha: u8,
    thickness: Option<RenderCursorThickness>,
    window_dpi: u32,
    color: [u8; 4],
    foreground: Option<[u8; 4]>,
    border: Option<[u8; 4]>,
}

fn fill_default_background(
    surface: &mut Surface<'_>,
    color: [u8; 4],
    gradient: Option<&RenderBackgroundGradient>,
) {
    if let Some(gradient) = gradient {
        if gradient.blend_with_default_background {
            surface.fill(color);
        }
        fill_default_background_gradient(
            surface,
            Rect {
                x: 0,
                y: 0,
                width: surface.width,
                height: surface.height,
            },
            gradient,
        );
    } else {
        surface.fill(color);
    }
}

fn fill_default_background_rect(
    surface: &mut Surface<'_>,
    rect: Rect,
    color: [u8; 4],
    gradient: Option<&RenderBackgroundGradient>,
) {
    if let Some(gradient) = gradient {
        if gradient.blend_with_default_background {
            surface.fill_rect(rect, color);
        }
        fill_default_background_gradient(surface, rect, gradient);
    } else {
        surface.fill_rect(rect, color);
    }
}

fn fill_default_background_gradient(
    surface: &mut Surface<'_>,
    rect: Rect,
    gradient: &RenderBackgroundGradient,
) {
    let max_y = rect.y.saturating_add(rect.height).min(surface.height);
    let max_x = rect.x.saturating_add(rect.width).min(surface.width);
    if max_y <= rect.y || max_x <= rect.x {
        return;
    }

    let sampler = BackgroundGradientSampler::from_gradient(gradient);
    let noise_amount = background_gradient_noise_amount(gradient);
    for row in rect.y..max_y {
        for column in rect.x..max_x {
            let position = background_gradient_position_at(
                gradient,
                column,
                row,
                surface.width,
                surface.height,
                noise_amount,
            );
            let color =
                background_gradient_color_with_hsb(sampler.color_at(position), gradient.hsb);
            let color = background_gradient_color_with_opacity(color, gradient.opacity_alpha);
            let index = ((row * surface.width + column) * 4) as usize;
            if let Some(pixel) = surface.target.get_mut(index..index + 4) {
                if gradient.blend_with_default_background {
                    let background = [pixel[0], pixel[1], pixel[2], pixel[3]];
                    pixel.copy_from_slice(&source_over_rgba(background, color));
                } else {
                    pixel.copy_from_slice(&color);
                }
            }
        }
    }
}

fn source_over_rgba(background: [u8; 4], foreground: [u8; 4]) -> [u8; 4] {
    let foreground_alpha = u32::from(foreground[3]);
    let background_alpha = u32::from(background[3]);
    let inverse_alpha = u32::from(u8::MAX) - foreground_alpha;
    let alpha =
        foreground_alpha + background_alpha.saturating_mul(inverse_alpha) / u32::from(u8::MAX);
    if alpha == 0 {
        return [0, 0, 0, 0];
    }

    let channel = |index: usize| {
        let foreground_weight = u32::from(foreground[index]).saturating_mul(foreground_alpha);
        let background_weight = u32::from(background[index])
            .saturating_mul(background_alpha)
            .saturating_mul(inverse_alpha)
            / u32::from(u8::MAX);
        let value = (foreground_weight + background_weight) / alpha;
        u8::try_from(value).unwrap_or(u8::MAX)
    };

    [
        channel(0),
        channel(1),
        channel(2),
        u8::try_from(alpha.min(u32::from(u8::MAX))).unwrap_or(u8::MAX),
    ]
}

enum BackgroundGradientSampler {
    Empty,
    Single([u8; 4]),
    Gradient(colorgrad::Gradient),
}

impl BackgroundGradientSampler {
    fn from_gradient(gradient: &RenderBackgroundGradient) -> Self {
        if let Some(preset) = gradient.preset {
            return Self::Gradient(segment_colorgrad_gradient(
                colorgrad_gradient_for_preset(preset),
                gradient.segment,
            ));
        }

        match gradient.colors.as_slice() {
            [] => Self::Empty,
            [color] => Self::Single(*color),
            colors => {
                let colors = colors
                    .iter()
                    .copied()
                    .map(colorgrad_color_from_rgba)
                    .collect::<Vec<_>>();
                colorgrad::CustomGradient::new()
                    .colors(&colors)
                    .interpolation(gradient.interpolation.to_colorgrad())
                    .mode(gradient.blend.to_colorgrad())
                    .build()
                    .map(|base| segment_colorgrad_gradient(base, gradient.segment))
                    .map_or(Self::Empty, Self::Gradient)
            }
        }
    }

    const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    fn color_at(&self, position: f64) -> [u8; 4] {
        let position = position.clamp(0.0, 1.0);
        match self {
            Self::Empty => default_background(),
            Self::Single(color) => *color,
            Self::Gradient(gradient) => gradient.at(position).to_rgba8(),
        }
    }
}

fn background_gradient_color_with_hsb(
    mut color: [u8; 4],
    hsb: RenderBackgroundGradientHsb,
) -> [u8; 4] {
    if !hsb.is_identity() {
        let [red, green, blue] = transform_rgb_hsb(color[0], color[1], color[2], hsb);
        color[0] = red;
        color[1] = green;
        color[2] = blue;
    }
    color
}

fn background_gradient_color_with_opacity(mut color: [u8; 4], opacity_alpha: u8) -> [u8; 4] {
    if opacity_alpha != u8::MAX {
        color[3] =
            u8::try_from((u16::from(color[3]) * u16::from(opacity_alpha)) / u16::from(u8::MAX))
                .expect("scaled alpha remains within u8");
    }
    color
}

fn transform_rgb_hsb(red: u8, green: u8, blue: u8, hsb: RenderBackgroundGradientHsb) -> [u8; 3] {
    let red_channel = red;
    let green_channel = green;
    let max_channel = red.max(green).max(blue);
    let red = f64::from(red) / 255.0;
    let green = f64::from(green) / 255.0;
    let blue = f64::from(blue) / 255.0;

    let max = red.max(green).max(blue);
    let min = red.min(green).min(blue);
    let delta = max - min;
    let hue = if delta <= f64::EPSILON {
        0.0
    } else if max_channel == red_channel {
        60.0 * ((green - blue) / delta).rem_euclid(6.0)
    } else if max_channel == green_channel {
        60.0 * (((blue - red) / delta) + 2.0)
    } else {
        60.0 * (((red - green) / delta) + 4.0)
    };
    let saturation = if max <= f64::EPSILON {
        0.0
    } else {
        delta / max
    };
    let value = max;

    let hue = (hue * (f64::from(hsb.hue) / 1_000.0)).rem_euclid(360.0);
    let saturation = (saturation * (f64::from(hsb.saturation) / 1_000.0)).clamp(0.0, 1.0);
    let value = (value * (f64::from(hsb.brightness) / 1_000.0)).clamp(0.0, 1.0);

    hsv_to_rgb(hue, saturation, value)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn hsv_to_rgb(hue: f64, saturation: f64, value: f64) -> [u8; 3] {
    let chroma = value * saturation;
    let hue_sector = hue / 60.0;
    let x = chroma * (1.0 - (hue_sector.rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = if hue_sector < 1.0 {
        (chroma, x, 0.0)
    } else if hue_sector < 2.0 {
        (x, chroma, 0.0)
    } else if hue_sector < 3.0 {
        (0.0, chroma, x)
    } else if hue_sector < 4.0 {
        (0.0, x, chroma)
    } else if hue_sector < 5.0 {
        (x, 0.0, chroma)
    } else {
        (chroma, 0.0, x)
    };
    let m = value - chroma;

    [
        round_rgb_component(red + m),
        round_rgb_component(green + m),
        round_rgb_component(blue + m),
    ]
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn round_rgb_component(component: f64) -> u8 {
    (component.mul_add(255.0, 1e-9)).round().clamp(0.0, 255.0) as u8
}

fn segment_colorgrad_gradient(
    gradient: colorgrad::Gradient,
    segment: Option<RenderBackgroundGradientSegment>,
) -> colorgrad::Gradient {
    match segment {
        Some(segment) => gradient.sharp(segment.size, segment.smoothness()),
        None => gradient,
    }
}

fn colorgrad_color_from_rgba(color: [u8; 4]) -> colorgrad::Color {
    colorgrad::Color::new(
        f64::from(color[0]) / 255.0,
        f64::from(color[1]) / 255.0,
        f64::from(color[2]) / 255.0,
        f64::from(color[3]) / 255.0,
    )
}

fn background_gradient_position_at(
    gradient: &RenderBackgroundGradient,
    column: u32,
    row: u32,
    width: u32,
    height: u32,
    noise_amount: usize,
) -> f64 {
    match gradient.orientation {
        RenderBackgroundGradientOrientation::Horizontal => {
            gradient_axis_position_with_noise(column, width, column, row, noise_amount)
        }
        RenderBackgroundGradientOrientation::Vertical => {
            1.0 - gradient_axis_position_with_noise(row, height, column, row, noise_amount)
        }
        RenderBackgroundGradientOrientation::Linear { angle_millidegrees } => {
            linear_gradient_axis_position(
                column,
                row,
                width,
                height,
                angle_millidegrees,
                noise_amount,
            )
        }
        RenderBackgroundGradientOrientation::Radial {
            cx_millis,
            cy_millis,
            radius_millis,
        } => radial_gradient_axis_position(
            column,
            row,
            width,
            height,
            cx_millis,
            cy_millis,
            radius_millis,
            noise_amount,
        ),
    }
}

fn gradient_axis_position(value: u32, extent: u32) -> f64 {
    if extent <= 1 {
        return 0.0;
    }

    f64::from(value.min(extent - 1)) / f64::from(extent - 1)
}

fn gradient_axis_position_with_noise(
    value: u32,
    extent: u32,
    column: u32,
    row: u32,
    noise_amount: usize,
) -> f64 {
    if extent <= 1 {
        return 0.0;
    }

    let noise = background_gradient_noise_offset(column, row, noise_amount);
    (f64::from(value.min(extent - 1)) + noise) / f64::from(extent - 1)
}

fn linear_gradient_axis_position(
    column: u32,
    row: u32,
    width: u32,
    height: u32,
    angle_millidegrees: i32,
    noise_amount: usize,
) -> f64 {
    let x = gradient_axis_position(column, width);
    let y = gradient_axis_position(row, height);
    let radians = (f64::from(angle_millidegrees) / 1_000.0).to_radians();
    let axis_x = radians.cos();
    let axis_y = -radians.sin();
    let pixel_noise = background_gradient_noise_offset(column, row, noise_amount);
    let noise = if width <= 1 {
        0.0
    } else {
        pixel_noise / f64::from(width - 1)
    };
    let projection = x.mul_add(axis_x, y * axis_y) + noise;
    let corners = [0.0, axis_x, axis_y, axis_x + axis_y];
    let min = corners.iter().copied().fold(f64::INFINITY, f64::min);
    let max = corners.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    if (max - min).abs() <= f64::EPSILON {
        return 0.0;
    }

    (projection - min) / (max - min)
}

#[expect(
    clippy::too_many_arguments,
    reason = "radial sampling inputs form one cohesive coordinate and configuration tuple"
)]
fn radial_gradient_axis_position(
    column: u32,
    row: u32,
    width: u32,
    height: u32,
    horizontal_center_millis: u32,
    vertical_center_millis: u32,
    radius_millis: u32,
    noise_amount: usize,
) -> f64 {
    if radius_millis == 0 {
        return 0.0;
    }

    if noise_amount > 0 {
        return radial_gradient_axis_position_with_noise(
            column,
            row,
            width,
            height,
            horizontal_center_millis,
            vertical_center_millis,
            radius_millis,
            noise_amount,
        );
    }

    let x = gradient_axis_position(column, width);
    let y = gradient_axis_position(row, height);
    let horizontal_center = f64::from(horizontal_center_millis) / 1_000.0;
    let vertical_center = f64::from(vertical_center_millis) / 1_000.0;
    let radius = f64::from(radius_millis) / 1_000.0;
    let dx = x - horizontal_center;
    let dy = y - vertical_center;

    dx.hypot(dy) / radius
}

#[expect(
    clippy::cast_precision_loss,
    clippy::too_many_arguments,
    reason = "the noisy radial path uses the same bounded coordinate tuple and normalized precision"
)]
fn radial_gradient_axis_position_with_noise(
    column: u32,
    row: u32,
    width: u32,
    height: u32,
    horizontal_center_millis: u32,
    vertical_center_millis: u32,
    radius_millis: u32,
    noise_amount: usize,
) -> f64 {
    let width = width.max(1);
    let height = height.max(1);
    let radius = (f64::from(width) * f64::from(radius_millis) / 1_000.0).max(f64::EPSILON);
    let horizontal_center = f64::from(width) * f64::from(horizontal_center_millis) / 1_000.0;
    let vertical_center = f64::from(height) * f64::from(vertical_center_millis) / 1_000.0;
    let x = f64::from(column.min(width - 1));
    let y = f64::from(row.min(height - 1));
    let noise_limit = noise_amount as f64;
    let nx = if (horizontal_center - x).abs() < noise_limit {
        0.0
    } else {
        background_gradient_noise_offset(column, row, noise_amount)
    };
    let ny = if (vertical_center - y).abs() < noise_limit {
        0.0
    } else {
        background_gradient_noise_offset(row, column, noise_amount)
    };
    let value = nx + (x - horizontal_center).powi(2) + (ny + y - vertical_center).powi(2);

    if value <= 0.0 {
        0.0
    } else {
        value.sqrt() / radius
    }
}

fn background_gradient_noise_amount(gradient: &RenderBackgroundGradient) -> usize {
    gradient.noise.unwrap_or(match gradient.orientation {
        RenderBackgroundGradientOrientation::Radial { .. } => 16,
        RenderBackgroundGradientOrientation::Horizontal
        | RenderBackgroundGradientOrientation::Vertical
        | RenderBackgroundGradientOrientation::Linear { .. } => 64,
    })
}

#[expect(
    clippy::cast_precision_loss,
    reason = "hash-derived visual noise is deliberately projected into f64 pixel space"
)]
fn background_gradient_noise_offset(column: u32, row: u32, noise_amount: usize) -> f64 {
    if noise_amount == 0 {
        return 0.0;
    }

    let amount = u64::try_from(noise_amount).unwrap_or(u64::MAX);
    let hash =
        splitmix64(u64::from(column) ^ u64::from(row).rotate_left(21) ^ 0x9e37_79b9_7f4a_7c15);
    -((hash % amount) as f64)
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[allow(clippy::too_many_lines)]
fn colorgrad_gradient_for_preset(preset: RenderBackgroundGradientPreset) -> colorgrad::Gradient {
    match preset {
        RenderBackgroundGradientPreset::Blues => colorgrad::blues(),
        RenderBackgroundGradientPreset::BrBg => colorgrad::br_bg(),
        RenderBackgroundGradientPreset::BuGn => colorgrad::bu_gn(),
        RenderBackgroundGradientPreset::BuPu => colorgrad::bu_pu(),
        RenderBackgroundGradientPreset::Cividis => colorgrad::cividis(),
        RenderBackgroundGradientPreset::Cool => colorgrad::cool(),
        RenderBackgroundGradientPreset::CubeHelixDefault => colorgrad::cubehelix_default(),
        RenderBackgroundGradientPreset::GnBu => colorgrad::gn_bu(),
        RenderBackgroundGradientPreset::Greens => colorgrad::greens(),
        RenderBackgroundGradientPreset::Greys => colorgrad::greys(),
        RenderBackgroundGradientPreset::Inferno => colorgrad::inferno(),
        RenderBackgroundGradientPreset::Magma => colorgrad::magma(),
        RenderBackgroundGradientPreset::OrRd => colorgrad::or_rd(),
        RenderBackgroundGradientPreset::Oranges => colorgrad::oranges(),
        RenderBackgroundGradientPreset::PiYg => colorgrad::pi_yg(),
        RenderBackgroundGradientPreset::Plasma => colorgrad::plasma(),
        RenderBackgroundGradientPreset::PrGn => colorgrad::pr_gn(),
        RenderBackgroundGradientPreset::PuBu => colorgrad::pu_bu(),
        RenderBackgroundGradientPreset::PuBuGn => colorgrad::pu_bu_gn(),
        RenderBackgroundGradientPreset::PuOr => colorgrad::pu_or(),
        RenderBackgroundGradientPreset::PuRd => colorgrad::pu_rd(),
        RenderBackgroundGradientPreset::Purples => colorgrad::purples(),
        RenderBackgroundGradientPreset::Rainbow => colorgrad::rainbow(),
        RenderBackgroundGradientPreset::RdBu => colorgrad::rd_bu(),
        RenderBackgroundGradientPreset::RdGy => colorgrad::rd_gy(),
        RenderBackgroundGradientPreset::RdPu => colorgrad::rd_pu(),
        RenderBackgroundGradientPreset::RdYlBu => colorgrad::rd_yl_bu(),
        RenderBackgroundGradientPreset::RdYlGn => colorgrad::rd_yl_gn(),
        RenderBackgroundGradientPreset::Reds => colorgrad::reds(),
        RenderBackgroundGradientPreset::Sinebow => colorgrad::sinebow(),
        RenderBackgroundGradientPreset::Spectral => colorgrad::spectral(),
        RenderBackgroundGradientPreset::Turbo => colorgrad::turbo(),
        RenderBackgroundGradientPreset::Viridis => colorgrad::viridis(),
        RenderBackgroundGradientPreset::Warm => colorgrad::warm(),
        RenderBackgroundGradientPreset::YlGn => colorgrad::yl_gn(),
        RenderBackgroundGradientPreset::YlGnBu => colorgrad::yl_gn_bu(),
        RenderBackgroundGradientPreset::YlOrBr => colorgrad::yl_or_br(),
        RenderBackgroundGradientPreset::YlOrRd => colorgrad::yl_or_rd(),
    }
}

impl Surface<'_> {
    fn fill(&mut self, color: [u8; 4]) {
        let pixel_count =
            usize::try_from(u64::from(self.width).saturating_mul(u64::from(self.height)))
                .unwrap_or(usize::MAX);
        let (pixels, _) = self.target.as_chunks_mut::<4>();
        let fill_len = pixel_count.min(pixels.len());
        pixels[..fill_len].fill(color);
    }

    fn fill_rect(&mut self, rect: Rect, color: [u8; 4]) {
        let max_y = rect.y.saturating_add(rect.height).min(self.height);
        let max_x = rect.x.saturating_add(rect.width).min(self.width);

        for row in rect.y..max_y {
            if let Some(range) = self.clipped_row_byte_range(row, rect.x, max_x) {
                let (pixels, remainder) = self.target[range].as_chunks_mut::<4>();
                debug_assert!(remainder.is_empty());
                pixels.fill(color);
            }
        }
    }

    fn fill_rect_alpha(&mut self, rect: Rect, color: [u8; 4], alpha: u8) {
        if alpha == u8::MAX {
            self.fill_rect(rect, color);
            return;
        }
        if alpha == 0 {
            return;
        }

        let max_y = rect.y.saturating_add(rect.height).min(self.height);
        let max_x = rect.x.saturating_add(rect.width).min(self.width);
        let alpha = u16::from(alpha);
        let inverse_alpha = u16::from(u8::MAX).saturating_sub(alpha);

        for row in rect.y..max_y {
            if let Some(range) = self.clipped_row_byte_range(row, rect.x, max_x) {
                let (pixels, remainder) = self.target[range].as_chunks_mut::<4>();
                debug_assert!(remainder.is_empty());
                for pixel in pixels {
                    pixel[0] = blend_channel(color[0], pixel[0], alpha, inverse_alpha);
                    pixel[1] = blend_channel(color[1], pixel[1], alpha, inverse_alpha);
                    pixel[2] = blend_channel(color[2], pixel[2], alpha, inverse_alpha);
                    pixel[3] = u8::MAX;
                }
            }
        }
    }

    fn clipped_row_byte_range(
        &self,
        row: u32,
        start_x: u32,
        end_x: u32,
    ) -> Option<std::ops::Range<usize>> {
        if row >= self.height || start_x >= end_x || start_x >= self.width {
            return None;
        }
        let end_x = end_x.min(self.width);
        let row_start = u64::from(row).saturating_mul(u64::from(self.width));
        let start = row_start
            .saturating_add(u64::from(start_x))
            .saturating_mul(4);
        let end = row_start.saturating_add(u64::from(end_x)).saturating_mul(4);
        let start = usize::try_from(start).ok()?;
        let complete_target_len = self.target.len() - self.target.len() % 4;
        if start >= complete_target_len {
            return None;
        }
        let end = usize::try_from(end)
            .unwrap_or(usize::MAX)
            .min(complete_target_len);
        (start < end).then_some(start..end)
    }

    fn try_fill_basic_glyph_8x16(
        &mut self,
        glyph: [u8; 8],
        origin_x: u32,
        origin_y: u32,
        color: [u8; 4],
    ) -> bool {
        if origin_x
            .checked_add(8)
            .is_none_or(|right| right > self.width)
            || origin_y
                .checked_add(16)
                .is_none_or(|bottom| bottom > self.height)
        {
            return false;
        }
        let required_len = u64::from(self.width)
            .checked_mul(u64::from(self.height))
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(|bytes| usize::try_from(bytes).ok());
        if required_len.is_none_or(|required_len| self.target.len() < required_len) {
            return false;
        }

        for (glyph_y, row_bits) in glyph.iter().copied().enumerate() {
            if row_bits == 0 {
                continue;
            }
            let glyph_y = u32::try_from(glyph_y).unwrap_or(0);
            let draw_y = origin_y + glyph_y * 2;
            for row in draw_y..draw_y + 2 {
                let row_start = usize::try_from(
                    (u64::from(row) * u64::from(self.width) + u64::from(origin_x)) * 4,
                )
                .expect("the complete framebuffer length was validated above");
                let (pixels, remainder) =
                    self.target[row_start..row_start + 8 * 4].as_chunks_mut::<4>();
                debug_assert!(remainder.is_empty());
                for (glyph_x, pixel) in pixels.iter_mut().enumerate() {
                    if row_bits & (1 << glyph_x) != 0 {
                        *pixel = color;
                    }
                }
            }
        }
        true
    }

    fn stroke_rect(&mut self, rect: Rect, color: [u8; 4], alpha: u8) {
        if rect.width == 0 || rect.height == 0 || alpha == 0 {
            return;
        }
        if rect.width == 1 || rect.height == 1 {
            self.fill_rect_alpha(rect, color, alpha);
            return;
        }

        self.fill_rect_alpha(Rect { height: 1, ..rect }, color, alpha);
        if rect.height > 1 {
            self.fill_rect_alpha(
                Rect {
                    y: rect.y + rect.height - 1,
                    height: 1,
                    ..rect
                },
                color,
                alpha,
            );
        }
        if rect.width > 1 {
            self.fill_rect_alpha(Rect { width: 1, ..rect }, color, alpha);
            self.fill_rect_alpha(
                Rect {
                    x: rect.x + rect.width - 1,
                    width: 1,
                    ..rect
                },
                color,
                alpha,
            );
        }
    }

    fn put_pixel(&mut self, x: u32, y: u32, color: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }

        let index = ((y * self.width + x) * 4) as usize;
        if let Some(pixel) = self.target.get_mut(index..index + 4) {
            pixel.copy_from_slice(&color);
        }
    }
}

fn blend_channel(foreground: u8, background: u8, alpha: u16, inverse_alpha: u16) -> u8 {
    let blended = u16::from(foreground)
        .saturating_mul(alpha)
        .saturating_add(u16::from(background).saturating_mul(inverse_alpha))
        / u16::from(u8::MAX);
    u8::try_from(blended).unwrap_or(u8::MAX)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn opacity_alpha(opacity: f32) -> u8 {
    (opacity.clamp(0.0, 1.0) * f32::from(u8::MAX)) as u8
}

fn resolve_runtime_inline_image_attachment_source(
    fragment: &RenderInlineImageFragment,
    image: &RenderInlineImage,
    decoded_width: u32,
    decoded_height: u32,
) -> Option<RenderInlineImageFragment> {
    if !fragment.cell_attachment || fragment.sampling_source_width != 0 {
        return Some(fragment.clone());
    }
    let source_x = image.source_x.unwrap_or(0);
    let source_y = image.source_y.unwrap_or(0);
    let source_width = image
        .source_width
        .unwrap_or(decoded_width.checked_sub(source_x)?)
        .min(decoded_width.checked_sub(source_x)?);
    let source_height = image
        .source_height
        .unwrap_or(decoded_height.checked_sub(source_y)?)
        .min(decoded_height.checked_sub(source_y)?);
    if source_width == 0
        || source_height == 0
        || fragment.source_destination_width == 0
        || fragment.source_destination_height == 0
    {
        return None;
    }

    let source_destination_right = fragment
        .source_destination_x
        .checked_add(fragment.destination_width)?;
    let source_destination_bottom = fragment
        .source_destination_y
        .checked_add(fragment.destination_height)?;
    let mut resolved = fragment.clone();
    resolved.source_x = source_x.checked_add(
        fragment.source_destination_x.saturating_mul(source_width)
            / fragment.source_destination_width,
    )?;
    resolved.source_y = source_y.checked_add(
        fragment.source_destination_y.saturating_mul(source_height)
            / fragment.source_destination_height,
    )?;
    let source_right = source_x.checked_add(
        source_destination_right
            .saturating_mul(source_width)
            .saturating_add(fragment.source_destination_width - 1)
            / fragment.source_destination_width,
    )?;
    let source_bottom = source_y.checked_add(
        source_destination_bottom
            .saturating_mul(source_height)
            .saturating_add(fragment.source_destination_height - 1)
            / fragment.source_destination_height,
    )?;
    resolved.source_width = source_right.checked_sub(resolved.source_x)?;
    resolved.source_height = source_bottom.checked_sub(resolved.source_y)?;
    resolved.sampling_source_x = source_x;
    resolved.sampling_source_y = source_y;
    resolved.sampling_source_width = source_width;
    resolved.sampling_source_height = source_height;
    Some(resolved)
}

#[must_use]
pub fn gpu_image_draw_plan(
    snapshot: &TerminalRenderSnapshot,
    geometry: RenderGeometry,
    animation_frame: usize,
    animation_elapsed_ms: Option<u64>,
) -> Vec<ImageDrawPlan> {
    image_draw_plan(
        snapshot,
        geometry,
        animation_frame,
        animation_elapsed_ms,
        None,
    )
}

fn image_draw_plan(
    snapshot: &TerminalRenderSnapshot,
    geometry: RenderGeometry,
    animation_frame: usize,
    animation_elapsed_ms: Option<u64>,
    selected_layer: Option<ImageDrawLayer>,
) -> Vec<ImageDrawPlan> {
    build_image_draw_plan(
        snapshot,
        geometry,
        animation_frame,
        animation_elapsed_ms,
        selected_layer,
    )
    .0
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ImageDrawPlanMetrics {
    decode_count: usize,
    unique_decoded_bytes: usize,
}

#[expect(
    clippy::too_many_lines,
    reason = "whole images and attachment fragments share one immutable normalization pass"
)]
fn build_image_draw_plan(
    snapshot: &TerminalRenderSnapshot,
    geometry: RenderGeometry,
    animation_frame: usize,
    animation_elapsed_ms: Option<u64>,
    selected_layer: Option<ImageDrawLayer>,
) -> (Vec<ImageDrawPlan>, ImageDrawPlanMetrics) {
    let fragments =
        runtime_inline_image_fragments(snapshot, geometry.cell_width, geometry.cell_height);
    let fragmented_parents = fragments
        .iter()
        .map(|fragment| fragment.fragment.parent_image_index)
        .collect::<HashSet<_>>();
    let mut plan = Vec::new();
    let mut decoded_parents = HashMap::<usize, Option<Arc<DecodedImage>>>::new();
    let mut metrics = ImageDrawPlanMetrics::default();

    for (parent_index, image) in snapshot.inline_images.iter().enumerate() {
        if fragmented_parents.contains(&parent_index)
            || snapshot
                .empty_inline_image_attachment_parents
                .contains(&parent_index)
            || selected_layer.is_some_and(|layer| layer != image_draw_layer(image_z_index(image)))
        {
            continue;
        }
        let Some(decoded) = cached_decoded_image(
            &mut decoded_parents,
            &mut metrics,
            parent_index,
            image,
            animation_frame,
            animation_elapsed_ms,
        ) else {
            continue;
        };
        let destination = inline_image_rect(image, geometry.cell_width, geometry.cell_height);
        let Some(source) = inline_image_source_rect(image, decoded.width, decoded.height) else {
            continue;
        };
        if let Some(draw) = plan_image_draw(
            decoded,
            i64::from(destination.x),
            i64::from(destination.y),
            destination.width,
            destination.height,
            None,
            geometry,
            source.x,
            source.y,
            0,
            0,
            source.width,
            source.height,
            destination.width,
            destination.height,
            image,
            parent_index,
            0,
            ImageTiePolicy::Whole,
            parent_index,
        ) {
            plan.push(draw);
        }
    }

    for (fragment_index, runtime) in fragments.iter().enumerate() {
        let parent_index = runtime.fragment.parent_image_index;
        let Some(image) = snapshot.inline_images.get(parent_index) else {
            continue;
        };
        if selected_layer.is_some_and(|layer| layer != image_draw_layer(image_z_index(image))) {
            continue;
        }
        let Some(decoded) = cached_decoded_image(
            &mut decoded_parents,
            &mut metrics,
            parent_index,
            image,
            animation_frame,
            animation_elapsed_ms,
        ) else {
            continue;
        };
        let Some(fragment) = resolve_runtime_inline_image_attachment_source(
            &runtime.fragment,
            image,
            decoded.width,
            decoded.height,
        ) else {
            continue;
        };
        if fragment.destination_width == 0
            || fragment.destination_height == 0
            || fragment.sampling_source_width == 0
            || fragment.sampling_source_height == 0
            || fragment.source_destination_width == 0
            || fragment.source_destination_height == 0
        {
            continue;
        }
        let origin_x = (i64::from(fragment.column) + runtime.column_offset)
            .saturating_mul(i64::from(geometry.cell_width))
            .saturating_add(i64::from(fragment.destination_x));
        let origin_y = (i64::from(fragment.row) + runtime.row_offset)
            .saturating_mul(i64::from(geometry.cell_height))
            .saturating_add(i64::from(fragment.destination_y));
        let clip = runtime.clip.map(|clip| {
            (
                clip.left.saturating_mul(i64::from(geometry.cell_width)),
                clip.top.saturating_mul(i64::from(geometry.cell_height)),
                clip.right.saturating_mul(i64::from(geometry.cell_width)),
                clip.bottom.saturating_mul(i64::from(geometry.cell_height)),
            )
        });
        if let Some(draw) = plan_image_draw(
            decoded,
            origin_x,
            origin_y,
            fragment.destination_width,
            fragment.destination_height,
            clip,
            geometry,
            fragment.sampling_source_x,
            fragment.sampling_source_y,
            fragment.source_destination_x,
            fragment.source_destination_y,
            fragment.sampling_source_width,
            fragment.sampling_source_height,
            fragment.source_destination_width,
            fragment.source_destination_height,
            image,
            parent_index,
            fragment_index,
            ImageTiePolicy::Fragment,
            fragment_index,
        ) {
            plan.push(draw);
        }
    }
    plan.sort_by(compare_image_draw_plans);
    (plan, metrics)
}

fn cached_decoded_image(
    cache: &mut HashMap<usize, Option<Arc<DecodedImage>>>,
    metrics: &mut ImageDrawPlanMetrics,
    parent_index: usize,
    image: &RenderInlineImage,
    animation_frame: usize,
    animation_elapsed_ms: Option<u64>,
) -> Option<Arc<DecodedImage>> {
    cache
        .entry(parent_index)
        .or_insert_with(|| {
            metrics.decode_count = metrics.decode_count.saturating_add(1);
            decode_inline_image(image, animation_frame, animation_elapsed_ms).map(|decoded| {
                metrics.unique_decoded_bytes = metrics
                    .unique_decoded_bytes
                    .checked_add(decoded.pixels.len())
                    .expect("live decoded image allocations cannot exceed host address space");
                Arc::new(decoded)
            })
        })
        .clone()
}

#[expect(
    clippy::too_many_arguments,
    reason = "the immutable draw plan keeps placement, clipping, sampling, and stable ordering explicit"
)]
fn plan_image_draw(
    decoded: Arc<DecodedImage>,
    local_origin_x: i64,
    local_origin_y: i64,
    destination_width: u32,
    destination_height: u32,
    clip: Option<(i64, i64, i64, i64)>,
    geometry: RenderGeometry,
    sample_source_x: u32,
    sample_source_y: u32,
    sample_target_x: u32,
    sample_target_y: u32,
    sample_source_width: u32,
    sample_source_height: u32,
    sample_destination_width: u32,
    sample_destination_height: u32,
    image: &RenderInlineImage,
    parent_index: usize,
    fragment_index: usize,
    tie_policy: ImageTiePolicy,
    stable_order: usize,
) -> Option<ImageDrawPlan> {
    if destination_width == 0
        || destination_height == 0
        || sample_source_width == 0
        || sample_source_height == 0
        || sample_destination_width == 0
        || sample_destination_height == 0
    {
        return None;
    }
    let origin_x = local_origin_x.saturating_add(i64::from(geometry.content_x));
    let origin_y = local_origin_y.saturating_add(i64::from(geometry.content_y));
    let destination_right = origin_x.saturating_add(i64::from(destination_width));
    let destination_bottom = origin_y.saturating_add(i64::from(destination_height));
    let content_right = geometry.content_x.saturating_add(geometry.content_width);
    let content_bottom = geometry.content_y.saturating_add(geometry.content_height);
    let mut left = origin_x.max(i64::from(geometry.content_x));
    let mut top = origin_y.max(i64::from(geometry.content_y));
    let mut right = destination_right.min(i64::from(content_right));
    let mut bottom = destination_bottom.min(i64::from(content_bottom));
    if let Some((clip_left, clip_top, clip_right, clip_bottom)) = clip {
        left = left.max(clip_left.saturating_add(i64::from(geometry.content_x)));
        top = top.max(clip_top.saturating_add(i64::from(geometry.content_y)));
        right = right.min(clip_right.saturating_add(i64::from(geometry.content_x)));
        bottom = bottom.min(clip_bottom.saturating_add(i64::from(geometry.content_y)));
    }
    if right <= left || bottom <= top {
        return None;
    }
    let x = u32::try_from(left).ok()?;
    let y = u32::try_from(top).ok()?;
    let width = u32::try_from(right - left).ok()?;
    let height = u32::try_from(bottom - top).ok()?;
    let clipped_target_x = u32::try_from(left.saturating_sub(origin_x)).ok()?;
    let clipped_target_y = u32::try_from(top.saturating_sub(origin_y)).ok()?;
    Some(ImageDrawPlan {
        destination_x: x,
        destination_y: y,
        width,
        height,
        decoded,
        sample_source_x,
        sample_source_y,
        sample_target_x: sample_target_x.checked_add(clipped_target_x)?,
        sample_target_y: sample_target_y.checked_add(clipped_target_y)?,
        sample_source_width,
        sample_source_height,
        sample_destination_width,
        sample_destination_height,
        z_index: image_z_index(image),
        kitty_image_id: image.kitty_image_id,
        parent_index,
        fragment_index,
        tie_policy,
        stable_order,
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "background rendering consumes one cohesive viewport and animation geometry tuple"
)]
fn render_background_images(
    surface: &mut Surface<'_>,
    images: &[RenderBackgroundImage],
    rect: Rect,
    scrollback_offset: usize,
    animation_frame: usize,
    animation_elapsed_ms: Option<u64>,
    cell_width: u32,
    cell_height: u32,
) {
    for image in images {
        render_background_image(
            surface,
            image,
            rect,
            scrollback_offset,
            animation_frame,
            animation_elapsed_ms,
            cell_width,
            cell_height,
        );
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "layer rendering consumes the same cohesive viewport and animation geometry tuple"
)]
fn render_background_layers(
    surface: &mut Surface<'_>,
    layers: &[RenderBackgroundLayer],
    rect: Rect,
    scrollback_offset: usize,
    animation_frame: usize,
    animation_elapsed_ms: Option<u64>,
    cell_width: u32,
    cell_height: u32,
) {
    for layer in layers {
        match layer {
            RenderBackgroundLayer::Color(color) => {
                render_background_color_layer(surface, rect, *color);
            }
            RenderBackgroundLayer::Gradient(gradient) => {
                render_background_gradient_layer(surface, rect, gradient);
            }
            RenderBackgroundLayer::Image(image) => {
                render_background_image(
                    surface,
                    image,
                    rect,
                    scrollback_offset,
                    animation_frame,
                    animation_elapsed_ms,
                    cell_width,
                    cell_height,
                );
            }
        }
    }
}

fn render_background_color_layer(surface: &mut Surface<'_>, rect: Rect, color: [u8; 4]) {
    let max_y = rect.y.saturating_add(rect.height).min(surface.height);
    let max_x = rect.x.saturating_add(rect.width).min(surface.width);
    if color[3] == 0 || max_y <= rect.y || max_x <= rect.x {
        return;
    }

    for row in rect.y..max_y {
        for column in rect.x..max_x {
            let index = ((row * surface.width + column) * 4) as usize;
            if let Some(pixel) = surface.target.get_mut(index..index + 4) {
                let background = [pixel[0], pixel[1], pixel[2], pixel[3]];
                pixel.copy_from_slice(&source_over_rgba(background, color));
            }
        }
    }
}

fn render_background_gradient_layer(
    surface: &mut Surface<'_>,
    rect: Rect,
    gradient: &RenderBackgroundGradient,
) {
    let max_y = rect.y.saturating_add(rect.height).min(surface.height);
    let max_x = rect.x.saturating_add(rect.width).min(surface.width);
    if max_y <= rect.y || max_x <= rect.x {
        return;
    }

    let sampler = BackgroundGradientSampler::from_gradient(gradient);
    let noise_amount = background_gradient_noise_amount(gradient);
    for row in rect.y..max_y {
        for column in rect.x..max_x {
            let position = background_gradient_position_at(
                gradient,
                column,
                row,
                surface.width,
                surface.height,
                noise_amount,
            );
            let color =
                background_gradient_color_with_hsb(sampler.color_at(position), gradient.hsb);
            let color = background_gradient_color_with_opacity(color, gradient.opacity_alpha);
            if color[3] == 0 {
                continue;
            }
            let index = ((row * surface.width + column) * 4) as usize;
            if let Some(pixel) = surface.target.get_mut(index..index + 4) {
                let background = [pixel[0], pixel[1], pixel[2], pixel[3]];
                pixel.copy_from_slice(&source_over_rgba(background, color));
            }
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "one image render consumes the complete viewport and animation geometry tuple"
)]
fn render_background_image(
    surface: &mut Surface<'_>,
    image: &RenderBackgroundImage,
    rect: Rect,
    scrollback_offset: usize,
    animation_frame: usize,
    animation_elapsed_ms: Option<u64>,
    cell_width: u32,
    cell_height: u32,
) {
    let animation_elapsed_ms = animation_elapsed_ms.map(|elapsed_ms| {
        background_image_animation_elapsed_ms(elapsed_ms, image.animation_speed_millis)
    });
    let Some(decoded) = decode_image_rgba(&image.data, animation_frame, animation_elapsed_ms)
    else {
        return;
    };
    if decoded.width == 0 || decoded.height == 0 {
        return;
    }
    let max_y = rect.y.saturating_add(rect.height).min(surface.height);
    let max_x = rect.x.saturating_add(rect.width).min(surface.width);
    if max_y <= rect.y || max_x <= rect.x || surface.width == 0 || surface.height == 0 {
        return;
    }

    let Some(layout) = background_image_layout(
        image,
        &decoded,
        surface.width,
        surface.height,
        cell_width,
        cell_height,
    ) else {
        return;
    };
    let attachment_scroll_y =
        background_image_attachment_scroll_pixels(image.attachment, scrollback_offset, cell_height);

    for target_y in rect.y..max_y {
        let Some(layout_y) = background_image_axis_coordinate(
            i64::from(target_y) - layout.origin_y + attachment_scroll_y,
            layout.height,
            layout.repeat_height,
            image.repeat_y,
        ) else {
            continue;
        };
        let source_y = layout_y.saturating_mul(decoded.height) / layout.height;
        for target_x in rect.x..max_x {
            let Some(layout_x) = background_image_axis_coordinate(
                i64::from(target_x) - layout.origin_x,
                layout.width,
                layout.repeat_width,
                image.repeat_x,
            ) else {
                continue;
            };
            let source_x = layout_x.saturating_mul(decoded.width) / layout.width;
            if let Some(mut pixel) = rgba_pixel(&decoded, source_x, source_y) {
                pixel = background_gradient_color_with_hsb(pixel, image.hsb);
                pixel[3] = u8::try_from(
                    u16::from(pixel[3]).saturating_mul(u16::from(image.opacity_alpha))
                        / u16::from(u8::MAX),
                )
                .unwrap_or(u8::MAX);
                if pixel[3] == 0 {
                    continue;
                }
                let index = ((target_y * surface.width + target_x) * 4) as usize;
                if let Some(background) = surface.target.get_mut(index..index + 4) {
                    let background_pixel =
                        [background[0], background[1], background[2], background[3]];
                    background.copy_from_slice(&source_over_rgba(background_pixel, pixel));
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct BackgroundImageLayout {
    origin_x: i64,
    origin_y: i64,
    width: u32,
    height: u32,
    repeat_width: u32,
    repeat_height: u32,
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "positive finite cover and contain scales are rounded to pixels with saturating float conversion"
)]
fn background_image_layout(
    image: &RenderBackgroundImage,
    decoded: &DecodedImage,
    surface_width: u32,
    surface_height: u32,
    cell_width: u32,
    cell_height: u32,
) -> Option<BackgroundImageLayout> {
    let explicit_width = background_image_dimension_pixels(image.width, surface_width, cell_width);
    let explicit_height =
        background_image_dimension_pixels(image.height, surface_height, cell_height);
    let (width, height) = match (explicit_width, explicit_height) {
        (Some(width), Some(height)) => (width, height),
        (Some(width), None) => (
            width,
            scale_preserving_aspect(width, decoded.height, decoded.width)?,
        ),
        (None, Some(height)) => (
            scale_preserving_aspect(height, decoded.width, decoded.height)?,
            height,
        ),
        (None, None) => {
            let width_scale = f64::from(surface_width) / f64::from(decoded.width);
            let height_scale = f64::from(surface_height) / f64::from(decoded.height);
            let scale = if image.width == RenderBackgroundImageDimension::Contain
                && image.height == RenderBackgroundImageDimension::Contain
            {
                width_scale.min(height_scale)
            } else {
                width_scale.max(height_scale)
            };
            if !scale.is_finite() || scale <= 0.0 {
                return None;
            }
            (
                (f64::from(decoded.width) * scale).ceil() as u32,
                (f64::from(decoded.height) * scale).ceil() as u32,
            )
        }
    };
    if width == 0 || height == 0 {
        return None;
    }
    let horizontal_offset =
        background_image_length_pixels(image.horizontal_offset, surface_width, cell_width);
    let vertical_offset =
        background_image_length_pixels(image.vertical_offset, surface_height, cell_height);
    let repeat_width = image
        .repeat_x_size
        .map(|length| background_image_length_pixels(length, surface_width, cell_width))
        .and_then(positive_i64_to_u32)
        .unwrap_or(width);
    let repeat_height = image
        .repeat_y_size
        .map(|length| background_image_length_pixels(length, surface_height, cell_height))
        .and_then(positive_i64_to_u32)
        .unwrap_or(height);
    if repeat_width == 0 || repeat_height == 0 {
        return None;
    }

    Some(BackgroundImageLayout {
        origin_x: background_image_horizontal_align_offset(
            surface_width,
            width,
            image.horizontal_align,
        ) + horizontal_offset,
        origin_y: background_image_vertical_align_offset(
            surface_height,
            height,
            image.vertical_align,
        ) + vertical_offset,
        width,
        height,
        repeat_width,
        repeat_height,
    })
}

fn background_image_animation_elapsed_ms(elapsed_ms: u64, speed_millis: u32) -> u64 {
    elapsed_ms.saturating_mul(u64::from(speed_millis)) / 1_000
}

fn background_image_attachment_scroll_pixels(
    attachment: RenderBackgroundImageAttachment,
    scrollback_offset: usize,
    cell_height: u32,
) -> i64 {
    let scroll_pixels = (scrollback_offset as i128).saturating_mul(i128::from(cell_height));
    let offset = match attachment {
        RenderBackgroundImageAttachment::Fixed => 0,
        RenderBackgroundImageAttachment::Scroll => scroll_pixels,
        RenderBackgroundImageAttachment::Parallax { factor_millis } => {
            scroll_pixels.saturating_mul(i128::from(factor_millis)) / 1_000
        }
    };
    i64::try_from(offset.clamp(i128::from(i64::MIN), i128::from(i64::MAX)))
        .expect("clamped background offset must fit i64")
}

fn background_image_dimension_pixels(
    dimension: RenderBackgroundImageDimension,
    viewport_size: u32,
    cell_size: u32,
) -> Option<u32> {
    match dimension {
        RenderBackgroundImageDimension::Cover | RenderBackgroundImageDimension::Contain => None,
        RenderBackgroundImageDimension::Pixels(pixels) => Some(pixels),
        RenderBackgroundImageDimension::Percent(basis_points) => {
            Some(scale_basis_points(viewport_size, basis_points))
        }
        RenderBackgroundImageDimension::Cells(cells) => cells.checked_mul(cell_size),
    }
}

fn background_image_length_pixels(
    length: RenderBackgroundImageLength,
    viewport_size: u32,
    cell_size: u32,
) -> i64 {
    match length {
        RenderBackgroundImageLength::Pixels(pixels) => i64::from(pixels),
        RenderBackgroundImageLength::Percent(basis_points) => {
            i64::from(viewport_size).saturating_mul(i64::from(basis_points)) / 10_000
        }
        RenderBackgroundImageLength::Cells(cells) => {
            i64::from(cell_size).saturating_mul(i64::from(cells))
        }
    }
}

fn scale_basis_points(value: u32, basis_points: u32) -> u32 {
    let scaled = u64::from(value).saturating_mul(u64::from(basis_points)) / 10_000;
    u32::try_from(scaled).unwrap_or(u32::MAX)
}

fn scale_preserving_aspect(size: u32, numerator: u32, denominator: u32) -> Option<u32> {
    if denominator == 0 {
        return None;
    }
    let scaled = u64::from(size)
        .saturating_mul(u64::from(numerator))
        .saturating_add(u64::from(denominator) - 1)
        / u64::from(denominator);
    Some(u32::try_from(scaled.max(1)).unwrap_or(u32::MAX))
}

fn positive_i64_to_u32(value: i64) -> Option<u32> {
    if value <= 0 {
        return None;
    }
    u32::try_from(value).ok()
}

fn background_image_horizontal_align_offset(
    surface_width: u32,
    image_width: u32,
    align: RenderBackgroundImageHorizontalAlign,
) -> i64 {
    match align {
        RenderBackgroundImageHorizontalAlign::Left => 0,
        RenderBackgroundImageHorizontalAlign::Center => {
            (i64::from(surface_width) - i64::from(image_width)) / 2
        }
        RenderBackgroundImageHorizontalAlign::Right => {
            i64::from(surface_width) - i64::from(image_width)
        }
    }
}

fn background_image_vertical_align_offset(
    surface_height: u32,
    image_height: u32,
    align: RenderBackgroundImageVerticalAlign,
) -> i64 {
    match align {
        RenderBackgroundImageVerticalAlign::Top => 0,
        RenderBackgroundImageVerticalAlign::Middle => {
            (i64::from(surface_height) - i64::from(image_height)) / 2
        }
        RenderBackgroundImageVerticalAlign::Bottom => {
            i64::from(surface_height) - i64::from(image_height)
        }
    }
}

fn background_image_axis_coordinate(
    relative_coordinate: i64,
    image_size: u32,
    repeat_size: u32,
    repeat: RenderBackgroundImageRepeat,
) -> Option<u32> {
    let image_size = i64::from(image_size);
    let repeat_size = i64::from(repeat_size);
    match repeat {
        RenderBackgroundImageRepeat::NoRepeat => {
            if (0..image_size).contains(&relative_coordinate) {
                u32::try_from(relative_coordinate).ok()
            } else {
                None
            }
        }
        RenderBackgroundImageRepeat::Repeat | RenderBackgroundImageRepeat::Mirror => {
            let coordinate = relative_coordinate.rem_euclid(repeat_size);
            if coordinate >= image_size {
                return None;
            }
            if repeat == RenderBackgroundImageRepeat::Mirror
                && relative_coordinate.div_euclid(repeat_size).rem_euclid(2) != 0
            {
                u32::try_from(image_size - coordinate - 1).ok()
            } else {
                u32::try_from(coordinate).ok()
            }
        }
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "runtime fragment normalization keeps checked parent and geometry bookkeeping in one auditable pass"
)]
fn runtime_inline_image_fragments(
    snapshot: &TerminalRenderSnapshot,
    cell_width: u32,
    cell_height: u32,
) -> Vec<RuntimeInlineImageFragment> {
    let mut fragments = Vec::new();
    for (parent_image_index, image) in snapshot.inline_images.iter().enumerate() {
        if snapshot
            .empty_inline_image_attachment_parents
            .contains(&parent_image_index)
        {
            continue;
        }
        let authored_fragments = snapshot
            .inline_image_fragments
            .iter()
            .filter(|fragment| fragment.parent_image_index == parent_image_index)
            .collect::<Vec<_>>();
        let (origin_column, origin_row) = snapshot
            .inline_image_parent_origins
            .get(parent_image_index)
            .copied()
            .unwrap_or((i64::from(image.column), i64::from(image.row)));
        let has_cell_attachments = authored_fragments
            .iter()
            .any(|fragment| fragment.cell_attachment);
        if has_cell_attachments {
            fragments.extend(authored_fragments.into_iter().filter_map(|attachment| {
                attachment.cell_attachment.then(|| {
                    let fragment = render_inline_image_attachment_for_geometry(
                        parent_image_index,
                        image,
                        attachment,
                        cell_width,
                        cell_height,
                    )?;
                    let (row_offset, column_offset) = snapshot
                        .inline_image_attachment_viewport_offsets
                        .get(&(
                            parent_image_index,
                            attachment.source_row,
                            attachment.source_column,
                        ))
                        .copied()
                        .unwrap_or((0, 0));
                    let clip = snapshot
                        .inline_image_attachment_viewport_clips
                        .get(&(
                            parent_image_index,
                            attachment.source_row,
                            attachment.source_column,
                        ))
                        .copied();
                    Some(RuntimeInlineImageFragment {
                        fragment,
                        row_offset,
                        column_offset,
                        clip,
                    })
                })?
            }));
            continue;
        }
        let Some(runtime_fragments) = render_inline_image_fragments_for_geometry(
            parent_image_index,
            image,
            origin_column,
            origin_row,
            cell_width,
            cell_height,
        ) else {
            continue;
        };
        let has_transform = authored_fragments.iter().any(|fragment| {
            i64::from(fragment.row) != fragment.source_row
                || i64::from(fragment.column) != fragment.source_column
        });
        if !has_transform {
            fragments.extend(runtime_fragments.into_iter().map(|fragment| {
                RuntimeInlineImageFragment {
                    fragment,
                    row_offset: 0,
                    column_offset: 0,
                    clip: None,
                }
            }));
            continue;
        }
        for attachment in authored_fragments {
            let Some(mut fragment) = runtime_fragments
                .iter()
                .find(|candidate| {
                    candidate.source_row == attachment.source_row
                        && candidate.source_column == attachment.source_column
                })
                .cloned()
            else {
                continue;
            };
            fragment.row = attachment.row;
            fragment.column = attachment.column;
            fragment.source_row = attachment.source_row;
            fragment.source_column = attachment.source_column;
            fragments.push(RuntimeInlineImageFragment {
                fragment,
                row_offset: 0,
                column_offset: 0,
                clip: None,
            });
        }
    }
    fragments
}

fn render_inline_image_attachment_for_geometry(
    parent_image_index: usize,
    image: &RenderInlineImage,
    attachment: &RenderInlineImageFragment,
    cell_width: u32,
    cell_height: u32,
) -> Option<RenderInlineImageFragment> {
    let columns = image.width.as_deref()?.parse::<u32>().ok()?.max(1);
    let rows = image.height.as_deref()?.parse::<u32>().ok()?.max(1);
    let source_column = u32::try_from(attachment.source_column).ok()?;
    let source_row = u32::try_from(attachment.source_row).ok()?;
    if source_column >= columns || source_row >= rows || cell_width == 0 || cell_height == 0 {
        return None;
    }

    let destination_width = columns.checked_mul(cell_width)?;
    let destination_height = rows.checked_mul(cell_height)?;
    let source_destination_x = source_column.checked_mul(cell_width)?;
    let source_destination_y = source_row.checked_mul(cell_height)?;
    if image.pixel_width.is_none() || image.pixel_height.is_none() {
        return Some(RenderInlineImageFragment {
            parent_image_index,
            cell_attachment: true,
            row: attachment.row,
            column: attachment.column,
            source_row: attachment.source_row,
            source_column: attachment.source_column,
            destination_x: image.target_x.unwrap_or(0),
            destination_y: image.target_y.unwrap_or(0),
            destination_width: cell_width,
            destination_height: cell_height,
            source_x: 0,
            source_y: 0,
            source_width: 0,
            source_height: 0,
            sampling_source_x: 0,
            sampling_source_y: 0,
            sampling_source_width: 0,
            sampling_source_height: 0,
            source_destination_x,
            source_destination_y,
            source_destination_width: destination_width,
            source_destination_height: destination_height,
        });
    }
    let pixel_width = image.pixel_width?;
    let pixel_height = image.pixel_height?;
    let source_x = image.source_x.unwrap_or(0);
    let source_y = image.source_y.unwrap_or(0);
    let source_width = image
        .source_width
        .unwrap_or(pixel_width.checked_sub(source_x)?)
        .min(pixel_width.checked_sub(source_x)?);
    let source_height = image
        .source_height
        .unwrap_or(pixel_height.checked_sub(source_y)?)
        .min(pixel_height.checked_sub(source_y)?);
    if source_column >= columns || source_row >= rows || source_width == 0 || source_height == 0 {
        return None;
    }
    let source_destination_right = source_destination_x.checked_add(cell_width)?;
    let source_destination_bottom = source_destination_y.checked_add(cell_height)?;
    let fragment_source_x = source_x
        .checked_add(source_destination_x.saturating_mul(source_width) / destination_width)?;
    let fragment_source_y = source_y
        .checked_add(source_destination_y.saturating_mul(source_height) / destination_height)?;
    let fragment_source_right = source_x.checked_add(
        source_destination_right
            .saturating_mul(source_width)
            .saturating_add(destination_width - 1)
            / destination_width,
    )?;
    let fragment_source_bottom = source_y.checked_add(
        source_destination_bottom
            .saturating_mul(source_height)
            .saturating_add(destination_height - 1)
            / destination_height,
    )?;

    Some(RenderInlineImageFragment {
        parent_image_index,
        cell_attachment: true,
        row: attachment.row,
        column: attachment.column,
        source_row: attachment.source_row,
        source_column: attachment.source_column,
        destination_x: image.target_x.unwrap_or(0),
        destination_y: image.target_y.unwrap_or(0),
        destination_width: cell_width,
        destination_height: cell_height,
        source_x: fragment_source_x,
        source_y: fragment_source_y,
        source_width: fragment_source_right.checked_sub(fragment_source_x)?,
        source_height: fragment_source_bottom.checked_sub(fragment_source_y)?,
        sampling_source_x: source_x,
        sampling_source_y: source_y,
        sampling_source_width: source_width,
        sampling_source_height: source_height,
        source_destination_x,
        source_destination_y,
        source_destination_width: destination_width,
        source_destination_height: destination_height,
    })
}

fn render_snapshot_inline_images_in_z_order(
    surface: &mut Surface<'_>,
    snapshot: &TerminalRenderSnapshot,
    layer: ImageDrawLayer,
    cell_width: u32,
    cell_height: u32,
    animation_frame: usize,
    animation_elapsed_ms: Option<u64>,
) {
    let geometry = RenderGeometry::new(surface.width, surface.height, cell_width, cell_height);
    for draw in image_draw_plan(
        snapshot,
        geometry,
        animation_frame,
        animation_elapsed_ms,
        Some(layer),
    ) {
        render_image_draw_plan(surface, &draw, None);
    }
}

fn render_damaged_snapshot_inline_images_in_z_order(
    surface: &mut Surface<'_>,
    snapshot: &TerminalRenderSnapshot,
    layer: ImageDrawLayer,
    damage: &[DamageRegion],
    geometry: RenderGeometry,
    animation_frame: usize,
    animation_elapsed_ms: Option<u64>,
) {
    let damage_rects = damage
        .iter()
        .copied()
        .filter(|region| !region.is_empty())
        .map(|region| damage_rect(region, geometry.cell_width, geometry.cell_height))
        .collect::<Vec<_>>();
    for draw in image_draw_plan(
        snapshot,
        geometry,
        animation_frame,
        animation_elapsed_ms,
        Some(layer),
    ) {
        render_image_draw_plan(surface, &draw, Some(&damage_rects));
    }
}

#[must_use]
pub fn image_draw_pixel(draw: &ImageDrawPlan, output_x: u32, output_y: u32) -> [u8; 4] {
    let source_x = draw.sample_source_x.saturating_add(
        draw.sample_target_x
            .saturating_add(output_x)
            .saturating_mul(draw.sample_source_width)
            / draw.sample_destination_width,
    );
    let source_y = draw.sample_source_y.saturating_add(
        draw.sample_target_y
            .saturating_add(output_y)
            .saturating_mul(draw.sample_source_height)
            / draw.sample_destination_height,
    );
    rgba_pixel(&draw.decoded, source_x, source_y).unwrap_or([0; 4])
}

fn render_image_draw_plan(
    surface: &mut Surface<'_>,
    draw: &ImageDrawPlan,
    damage: Option<&[Rect]>,
) -> usize {
    let draw_rect = Rect {
        x: draw.destination_x,
        y: draw.destination_y,
        width: draw.width,
        height: draw.height,
    };
    let mut sampled = 0;
    for_each_image_draw_span(draw_rect, damage, |y, start_x, end_x| {
        for x in start_x..end_x {
            let pixel = image_draw_pixel(
                draw,
                x.saturating_sub(draw.destination_x),
                y.saturating_sub(draw.destination_y),
            );
            sampled += 1;
            if inline_image_pixel_is_drawn(pixel) {
                surface.put_pixel(x, y, pixel);
            }
        }
    });
    sampled
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ImageDrawSpan {
    y: u32,
    start_x: u32,
    end_x: u32,
}

fn for_each_image_draw_span(
    draw_rect: Rect,
    damage: Option<&[Rect]>,
    mut visit: impl FnMut(u32, u32, u32),
) {
    let Some(damage) = damage else {
        let end_x = draw_rect.x.saturating_add(draw_rect.width);
        for y in draw_rect.y..draw_rect.y.saturating_add(draw_rect.height) {
            visit(y, draw_rect.x, end_x);
        }
        return;
    };

    let mut spans = Vec::new();
    for rect in damage
        .iter()
        .filter_map(|damage| rect_intersection(draw_rect, *damage))
    {
        let end_x = rect.x.saturating_add(rect.width);
        for y in rect.y..rect.y.saturating_add(rect.height) {
            spans.push(ImageDrawSpan {
                y,
                start_x: rect.x,
                end_x,
            });
        }
    }
    spans.sort_unstable();

    let mut current = None::<ImageDrawSpan>;
    for span in spans {
        match current.as_mut() {
            Some(active) if active.y == span.y && span.start_x <= active.end_x => {
                active.end_x = active.end_x.max(span.end_x);
            }
            Some(active) => {
                visit(active.y, active.start_x, active.end_x);
                *active = span;
            }
            None => current = Some(span),
        }
    }
    if let Some(span) = current {
        visit(span.y, span.start_x, span.end_x);
    }
}

fn rect_intersection(left: Rect, right: Rect) -> Option<Rect> {
    let x = left.x.max(right.x);
    let y = left.y.max(right.y);
    let right_edge = left
        .x
        .saturating_add(left.width)
        .min(right.x.saturating_add(right.width));
    let bottom_edge = left
        .y
        .saturating_add(left.height)
        .min(right.y.saturating_add(right.height));
    (right_edge > x && bottom_edge > y).then(|| Rect {
        x,
        y,
        width: right_edge - x,
        height: bottom_edge - y,
    })
}

fn image_z_index(image: &RenderInlineImage) -> i32 {
    image.kitty_z_index.unwrap_or(0)
}

fn decode_inline_image(
    image: &RenderInlineImage,
    animation_frame: usize,
    animation_elapsed_ms: Option<u64>,
) -> Option<DecodedImage> {
    match image.image_format {
        InlineImageFormat::Encoded => {
            decode_image_rgba(&image.data, animation_frame, animation_elapsed_ms)
        }
        InlineImageFormat::Rgb => {
            decode_raw_rgb(&image.data, image.pixel_width?, image.pixel_height?)
        }
        InlineImageFormat::Rgba => {
            decode_raw_rgba(&image.data, image.pixel_width?, image.pixel_height?)
        }
    }
}

fn decode_image_rgba(
    data: &[u8],
    animation_frame: usize,
    animation_elapsed_ms: Option<u64>,
) -> Option<DecodedImage> {
    #[cfg(not(feature = "image-gif"))]
    let _ = (animation_frame, animation_elapsed_ms);
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        #[cfg(feature = "image-gif")]
        return decode_gif_frame_rgba(data, animation_frame, animation_elapsed_ms);
        #[cfg(not(feature = "image-gif"))]
        return None;
    }

    let decoded = image::load_from_memory(data);
    #[cfg(feature = "image-legacy")]
    let decoded =
        decoded.or_else(|_| image::load_from_memory_with_format(data, image::ImageFormat::Tga));
    let image = decoded.ok()?.to_rgba8();
    let width = image.width();
    let height = image.height();

    Some(DecodedImage {
        width,
        height,
        pixels: image.into_raw().into(),
    })
}

#[cfg(feature = "image-gif")]
fn decode_gif_frame_rgba(
    data: &[u8],
    animation_frame: usize,
    animation_elapsed_ms: Option<u64>,
) -> Option<DecodedImage> {
    let decoder = image::codecs::gif::GifDecoder::new(Cursor::new(data)).ok()?;
    let frames = decoder.into_frames().collect_frames().ok()?;
    let frame_count = frames.len();
    if frame_count == 0 {
        return None;
    }

    let frame_index = animation_elapsed_ms.map_or(animation_frame % frame_count, |elapsed_ms| {
        gif_frame_index_for_elapsed_ms(&frames, elapsed_ms)
    });
    let image = frames.into_iter().nth(frame_index)?.into_buffer();
    let width = image.width();
    let height = image.height();

    Some(DecodedImage {
        width,
        height,
        pixels: image.into_raw().into(),
    })
}

#[cfg(feature = "image-gif")]
fn gif_frame_index_for_elapsed_ms(frames: &[image::Frame], elapsed_ms: u64) -> usize {
    let delays = frames.iter().map(gif_frame_delay_ms).collect::<Vec<_>>();
    animation_frame_index_for_delays(&delays, elapsed_ms)
}

#[cfg(feature = "image-gif")]
fn gif_frame_delays_ms(data: &[u8]) -> Option<Vec<u64>> {
    if !data.starts_with(b"GIF87a") && !data.starts_with(b"GIF89a") {
        return None;
    }
    let decoder = image::codecs::gif::GifDecoder::new(Cursor::new(data)).ok()?;
    let frames = decoder.into_frames().collect_frames().ok()?;
    (!frames.is_empty()).then(|| frames.iter().map(gif_frame_delay_ms).collect())
}

#[cfg(not(feature = "image-gif"))]
fn gif_frame_delays_ms(_data: &[u8]) -> Option<Vec<u64>> {
    None
}

fn animation_frame_index_for_delays(delays: &[u64], elapsed_ms: u64) -> usize {
    let total_duration_ms = delays
        .iter()
        .fold(0_u64, |total, delay| total.saturating_add(*delay));
    if total_duration_ms == 0 {
        return 0;
    }

    let elapsed_ms = elapsed_ms % total_duration_ms;
    let mut frame_start_ms = 0_u64;
    for (index, delay) in delays.iter().enumerate() {
        frame_start_ms = frame_start_ms.saturating_add(*delay);
        if elapsed_ms < frame_start_ms {
            return index;
        }
    }

    0
}

#[cfg(feature = "image-gif")]
fn gif_frame_delay_ms(frame: &image::Frame) -> u64 {
    let (numerator, denominator) = frame.delay().numer_denom_ms();
    if denominator == 0 {
        return 0;
    }

    u64::from(numerator) / u64::from(denominator)
}

fn decode_raw_rgb(data: &[u8], width: u32, height: u32) -> Option<DecodedImage> {
    validate_raw_image_len(data.len(), width, height, 3)?;

    let mut pixels = Vec::with_capacity(
        usize::try_from(width)
            .ok()?
            .checked_mul(usize::try_from(height).ok()?)?
            .checked_mul(4)?,
    );
    for rgb in data.chunks_exact(3) {
        pixels.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
    }

    Some(DecodedImage {
        width,
        height,
        pixels: pixels.into(),
    })
}

fn decode_raw_rgba(data: &[u8], width: u32, height: u32) -> Option<DecodedImage> {
    validate_raw_image_len(data.len(), width, height, 4)?;

    Some(DecodedImage {
        width,
        height,
        pixels: Arc::from(data),
    })
}

fn validate_raw_image_len(
    len: usize,
    width: u32,
    height: u32,
    bytes_per_pixel: usize,
) -> Option<()> {
    let expected_len = usize::try_from(width)
        .ok()?
        .checked_mul(usize::try_from(height).ok()?)?
        .checked_mul(bytes_per_pixel)?;
    (len == expected_len).then_some(())
}

fn rgba_pixel(image: &DecodedImage, x: u32, y: u32) -> Option<[u8; 4]> {
    if x >= image.width || y >= image.height {
        return None;
    }

    let index = usize::try_from((y * image.width + x) * 4).ok()?;
    let pixel = image.pixels.get(index..index + 4)?;
    Some([pixel[0], pixel[1], pixel[2], pixel[3]])
}

fn inline_image_rect(image: &RenderInlineImage, cell_width: u32, cell_height: u32) -> Rect {
    Rect {
        x: u32::from(image.column)
            .saturating_mul(cell_width)
            .saturating_add(image.target_x.unwrap_or(0)),
        y: u32::from(image.row)
            .saturating_mul(cell_height)
            .saturating_add(image.target_y.unwrap_or(0)),
        width: inline_image_axis_pixels(image.width.as_deref(), cell_width),
        height: inline_image_axis_pixels(image.height.as_deref(), cell_height),
    }
}

fn inline_image_source_rect(
    image: &RenderInlineImage,
    decoded_width: u32,
    decoded_height: u32,
) -> Option<Rect> {
    if decoded_width == 0 || decoded_height == 0 {
        return None;
    }

    let x = image.source_x.unwrap_or(0);
    let y = image.source_y.unwrap_or(0);
    if x >= decoded_width || y >= decoded_height {
        return None;
    }

    let available_width = decoded_width - x;
    let available_height = decoded_height - y;
    let width = image
        .source_width
        .unwrap_or(available_width)
        .min(available_width);
    let height = image
        .source_height
        .unwrap_or(available_height)
        .min(available_height);

    (width > 0 && height > 0).then_some(Rect {
        x,
        y,
        width,
        height,
    })
}

fn inline_image_axis_pixels(value: Option<&str>, cell_pixels: u32) -> u32 {
    let Some(value) = value else {
        return cell_pixels;
    };
    if let Some(pixels) = value.strip_suffix("px").and_then(parse_positive_u32) {
        return pixels;
    }
    value
        .parse::<u32>()
        .ok()
        .filter(|value| *value > 0)
        .map_or(cell_pixels, |cells| cells.saturating_mul(cell_pixels))
}

#[expect(
    clippy::too_many_lines,
    reason = "checked image slicing stays together so every overflow exit remains visible"
)]
fn render_inline_image_fragments_for_geometry(
    parent_image_index: usize,
    image: &RenderInlineImage,
    origin_column: i64,
    origin_row: i64,
    cell_width: u32,
    cell_height: u32,
) -> Option<Vec<RenderInlineImageFragment>> {
    let pixel_width = image.pixel_width?;
    let pixel_height = image.pixel_height?;
    let source_x = image.source_x.unwrap_or(0);
    let source_y = image.source_y.unwrap_or(0);
    let available_width = pixel_width.checked_sub(source_x)?;
    let available_height = pixel_height.checked_sub(source_y)?;
    let source_width = image
        .source_width
        .unwrap_or(available_width)
        .min(available_width);
    let source_height = image
        .source_height
        .unwrap_or(available_height)
        .min(available_height);
    if source_width == 0 || source_height == 0 || cell_width == 0 || cell_height == 0 {
        return None;
    }

    let destination_width = inline_image_axis_pixels(image.width.as_deref(), cell_width);
    let destination_height = inline_image_axis_pixels(image.height.as_deref(), cell_height);
    let destination_left = origin_column
        .checked_mul(i64::from(cell_width))?
        .checked_add(i64::from(image.target_x.unwrap_or(0)))?;
    let destination_top = origin_row
        .checked_mul(i64::from(cell_height))?
        .checked_add(i64::from(image.target_y.unwrap_or(0)))?;
    let destination_right = destination_left.checked_add(i64::from(destination_width))?;
    let destination_bottom = destination_top.checked_add(i64::from(destination_height))?;
    let first_column = destination_left.div_euclid(i64::from(cell_width));
    let first_row = destination_top.div_euclid(i64::from(cell_height));
    let last_column = destination_right
        .checked_sub(1)?
        .div_euclid(i64::from(cell_width));
    let last_row = destination_bottom
        .checked_sub(1)?
        .div_euclid(i64::from(cell_height));
    let fragment_count = (last_column.checked_sub(first_column)? + 1)
        .checked_mul(last_row.checked_sub(first_row)? + 1)?;
    if !(1..=1_000_000).contains(&fragment_count) {
        return None;
    }

    let mut fragments = Vec::with_capacity(usize::try_from(fragment_count).ok()?);
    for row in first_row..=last_row {
        if !(0..=i64::from(u16::MAX)).contains(&row) {
            continue;
        }
        let cell_top = row.checked_mul(i64::from(cell_height))?;
        let fragment_top = destination_top.max(cell_top);
        let fragment_bottom = destination_bottom.min(cell_top + i64::from(cell_height));
        for column in first_column..=last_column {
            if !(0..=i64::from(u16::MAX)).contains(&column) {
                continue;
            }
            let cell_left = column.checked_mul(i64::from(cell_width))?;
            let fragment_left = destination_left.max(cell_left);
            let fragment_right = destination_right.min(cell_left + i64::from(cell_width));
            let source_destination_x = u32::try_from(fragment_left - destination_left).ok()?;
            let source_destination_y = u32::try_from(fragment_top - destination_top).ok()?;
            let source_destination_width = destination_width;
            let source_destination_height = destination_height;
            let source_destination_right = source_destination_x
                .checked_add(u32::try_from(fragment_right - fragment_left).ok()?)?;
            let source_destination_bottom = source_destination_y
                .checked_add(u32::try_from(fragment_bottom - fragment_top).ok()?)?;
            let fragment_source_x = source_x.checked_add(
                source_destination_x.saturating_mul(source_width) / destination_width,
            )?;
            let fragment_source_y = source_y.checked_add(
                source_destination_y.saturating_mul(source_height) / destination_height,
            )?;
            let fragment_source_right = source_x.checked_add(
                source_destination_right
                    .saturating_mul(source_width)
                    .saturating_add(destination_width - 1)
                    / destination_width,
            )?;
            let fragment_source_bottom = source_y.checked_add(
                source_destination_bottom
                    .saturating_mul(source_height)
                    .saturating_add(destination_height - 1)
                    / destination_height,
            )?;
            fragments.push(RenderInlineImageFragment {
                parent_image_index,
                cell_attachment: false,
                row: u16::try_from(row).ok()?,
                column: u16::try_from(column).ok()?,
                source_row: row,
                source_column: column,
                destination_x: u32::try_from(fragment_left - cell_left).ok()?,
                destination_y: u32::try_from(fragment_top - cell_top).ok()?,
                destination_width: u32::try_from(fragment_right - fragment_left).ok()?,
                destination_height: u32::try_from(fragment_bottom - fragment_top).ok()?,
                source_x: fragment_source_x,
                source_y: fragment_source_y,
                source_width: fragment_source_right.checked_sub(fragment_source_x)?,
                source_height: fragment_source_bottom.checked_sub(fragment_source_y)?,
                sampling_source_x: source_x,
                sampling_source_y: source_y,
                sampling_source_width: source_width,
                sampling_source_height: source_height,
                source_destination_x,
                source_destination_y,
                source_destination_width,
                source_destination_height,
            });
        }
    }
    Some(fragments)
}

fn parse_positive_u32(value: &str) -> Option<u32> {
    value.parse::<u32>().ok().filter(|value| *value > 0)
}

#[expect(
    clippy::too_many_arguments,
    reason = "cell rendering consumes one cohesive palette and geometry context"
)]
fn render_cell_background(
    surface: &mut Surface<'_>,
    cell: &RenderCell,
    cell_width: u32,
    cell_height: u32,
    bold_brightens_ansi_colors: RenderBoldBrightensAnsiColors,
    default_foreground: [u8; 4],
    default_background: [u8; 4],
    ansi_palette: Option<&[[u8; 4]; 16]>,
    indexed_palette: Option<&RenderIndexedPalette>,
) {
    let origin_x = u32::from(cell.column).saturating_mul(cell_width);
    let origin_y = u32::from(cell.row).saturating_mul(cell_height);
    let (_, background) = effective_cell_colors(
        cell,
        bold_brightens_ansi_colors,
        default_foreground,
        default_background,
        ansi_palette,
        indexed_palette,
    );
    if background == default_background {
        return;
    }

    let rect = Rect {
        x: origin_x,
        y: origin_y,
        width: cell_width,
        height: cell_height,
    };
    if background[3] == u8::MAX {
        surface.fill_rect(rect, background);
    } else {
        surface.fill_rect_alpha(rect, background, background[3]);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "foreground rendering consumes one cohesive text, palette, and geometry context"
)]
fn render_cell_foreground(
    surface: &mut Surface<'_>,
    cell: &RenderCell,
    cell_width: u32,
    cell_height: u32,
    text_blink_opacity_alpha: u8,
    rapid_text_blink_opacity_alpha: u8,
    underline_thickness: Option<RenderUnderlineThickness>,
    underline_position: Option<RenderUnderlinePosition>,
    strikethrough_position: Option<RenderStrikethroughPosition>,
    window_dpi: u32,
    bold_brightens_ansi_colors: RenderBoldBrightensAnsiColors,
    default_foreground: [u8; 4],
    default_background: [u8; 4],
    ansi_palette: Option<&[[u8; 4]; 16]>,
    indexed_palette: Option<&RenderIndexedPalette>,
) {
    let origin_x = u32::from(cell.column).saturating_mul(cell_width);
    let origin_y = u32::from(cell.row).saturating_mul(cell_height);
    let (foreground, _) = effective_cell_colors(
        cell,
        bold_brightens_ansi_colors,
        default_foreground,
        default_background,
        ansi_palette,
        indexed_palette,
    );
    let foreground_alpha = text_foreground_alpha(
        cell,
        text_blink_opacity_alpha,
        rapid_text_blink_opacity_alpha,
    );

    if cell.conceal || foreground_alpha == 0 {
        return;
    }

    let Some(glyph) = BASIC_FONTS.get(cell.ch) else {
        return;
    };

    let draws_bold = cell_draws_bold(cell, bold_brightens_ansi_colors);
    render_basic_glyph(
        surface,
        BasicGlyphRender {
            glyph,
            origin_x,
            origin_y,
            cell_width,
            cell_height,
            foreground,
            foreground_alpha,
            italic: cell.italic,
            draws_bold,
            vertical_align: cell.vertical_align,
        },
    );

    render_text_decorations(
        surface,
        cell,
        Rect {
            x: origin_x,
            y: origin_y,
            width: cell_width,
            height: cell_height,
        },
        foreground,
        color_to_rgba_with_palette(
            cell.underline_color,
            foreground,
            ansi_palette,
            indexed_palette,
        ),
        foreground_alpha,
        underline_thickness,
        underline_position,
        strikethrough_position,
        window_dpi,
    );
}

#[derive(Clone, Copy)]
struct BasicGlyphRender {
    glyph: [u8; 8],
    origin_x: u32,
    origin_y: u32,
    cell_width: u32,
    cell_height: u32,
    foreground: [u8; 4],
    foreground_alpha: u8,
    italic: bool,
    draws_bold: bool,
    vertical_align: VerticalAlign,
}

fn render_basic_glyph(surface: &mut Surface<'_>, render: BasicGlyphRender) {
    let BasicGlyphRender {
        glyph,
        origin_x,
        origin_y,
        cell_width,
        cell_height,
        foreground,
        foreground_alpha,
        italic,
        draws_bold,
        vertical_align,
    } = render;
    let scale_x = cell_width.max(8) / 8;
    let scale_y = cell_height.max(8) / 8;
    let rendered_fast = foreground_alpha == u8::MAX
        && cell_width == 8
        && cell_height == 16
        && !italic
        && !draws_bold
        && vertical_align == VerticalAlign::Baseline
        && surface.try_fill_basic_glyph_8x16(glyph, origin_x, origin_y, foreground);
    if rendered_fast {
        return;
    }

    for (glyph_y, row_bits) in glyph.iter().enumerate() {
        if *row_bits == 0 {
            continue;
        }
        let row_offset = italic_row_offset(glyph_y, scale_x, italic);
        let Some(draw_y) = vertical_aligned_y(
            origin_y,
            cell_height,
            u32::try_from(glyph_y).unwrap_or(0) * scale_y,
            vertical_align,
        ) else {
            continue;
        };
        if foreground_alpha == u8::MAX {
            for_each_opaque_glyph_row_run(
                *row_bits,
                origin_x,
                cell_width,
                scale_x,
                row_offset,
                draws_bold,
                |draw_x, width| {
                    surface.fill_rect(
                        Rect {
                            x: draw_x,
                            y: draw_y,
                            width,
                            height: scale_y,
                        },
                        foreground,
                    );
                },
            );
            continue;
        }
        render_translucent_glyph_row(
            surface,
            TranslucentGlyphRow {
                row_bits: *row_bits,
                origin_x,
                draw_y,
                cell_width,
                scale_x,
                scale_y,
                row_offset,
                foreground,
                foreground_alpha,
                draws_bold,
            },
        );
    }
}

#[derive(Clone, Copy)]
struct TranslucentGlyphRow {
    row_bits: u8,
    origin_x: u32,
    draw_y: u32,
    cell_width: u32,
    scale_x: u32,
    scale_y: u32,
    row_offset: u32,
    foreground: [u8; 4],
    foreground_alpha: u8,
    draws_bold: bool,
}

fn render_translucent_glyph_row(surface: &mut Surface<'_>, row: TranslucentGlyphRow) {
    let TranslucentGlyphRow {
        row_bits,
        origin_x,
        draw_y,
        cell_width,
        scale_x,
        scale_y,
        row_offset,
        foreground,
        foreground_alpha,
        draws_bold,
    } = row;
    for glyph_x in 0..8 {
        if row_bits & (1 << glyph_x) == 0 {
            continue;
        }
        let draw_x = origin_x + glyph_x * scale_x + row_offset;
        let Some(width) = clipped_cell_width(draw_x, origin_x, cell_width, scale_x) else {
            continue;
        };
        surface.fill_rect_alpha(
            Rect {
                x: draw_x,
                y: draw_y,
                width,
                height: scale_y,
            },
            foreground,
            foreground_alpha,
        );
        let bold_x = draw_x.saturating_add(scale_x);
        if draws_bold && bold_x < origin_x.saturating_add(cell_width) {
            surface.fill_rect_alpha(
                Rect {
                    x: bold_x,
                    y: draw_y,
                    width: scale_x,
                    height: scale_y,
                },
                foreground,
                foreground_alpha,
            );
        }
    }
}

#[doc(hidden)]
#[must_use]
pub fn text_foreground_alpha(
    cell: &RenderCell,
    text_blink_opacity_alpha: u8,
    rapid_text_blink_opacity_alpha: u8,
) -> u8 {
    if !cell.blink {
        return u8::MAX;
    }

    if cell.rapid_blink {
        rapid_text_blink_opacity_alpha
    } else {
        text_blink_opacity_alpha
    }
}

#[doc(hidden)]
#[must_use]
pub fn effective_cell_colors(
    cell: &RenderCell,
    bold_brightens_ansi_colors: RenderBoldBrightensAnsiColors,
    default_foreground: [u8; 4],
    default_background: [u8; 4],
    ansi_palette: Option<&[[u8; 4]; 16]>,
    indexed_palette: Option<&RenderIndexedPalette>,
) -> ([u8; 4], [u8; 4]) {
    let foreground = color_to_rgba_with_palette(
        effective_cell_foreground(cell, bold_brightens_ansi_colors),
        default_foreground,
        ansi_palette,
        indexed_palette,
    );
    let background = color_to_rgba_with_palette(
        cell.background,
        default_background,
        ansi_palette,
        indexed_palette,
    );
    let (foreground, background) = if cell.inverse {
        (background, foreground)
    } else {
        (foreground, background)
    };
    let foreground = if cell.faint {
        dim_foreground(foreground)
    } else {
        foreground
    };

    (foreground, background)
}

fn effective_cell_foreground(
    cell: &RenderCell,
    bold_brightens_ansi_colors: RenderBoldBrightensAnsiColors,
) -> Color {
    let Color::Indexed(index @ 0..=7) = cell.foreground else {
        return cell.foreground;
    };

    if cell.bold && bold_brightens_ansi_colors != RenderBoldBrightensAnsiColors::No {
        Color::Indexed(index.saturating_add(8))
    } else {
        cell.foreground
    }
}

fn cell_draws_bold(
    cell: &RenderCell,
    bold_brightens_ansi_colors: RenderBoldBrightensAnsiColors,
) -> bool {
    if !cell.bold {
        return false;
    }

    if bold_brightens_ansi_colors != RenderBoldBrightensAnsiColors::BrightOnly {
        return true;
    }

    !matches!(cell.foreground, Color::Indexed(0..=7))
}

fn italic_row_offset(glyph_y: usize, scale_x: u32, italic: bool) -> u32 {
    if italic {
        u32::try_from(7usize.saturating_sub(glyph_y)).unwrap_or(0) / 3 * scale_x
    } else {
        0
    }
}

fn vertical_aligned_y(
    origin_y: u32,
    cell_height: u32,
    glyph_y: u32,
    vertical_align: VerticalAlign,
) -> Option<u32> {
    let offset = (cell_height / 4).max(1);
    match vertical_align {
        VerticalAlign::Baseline => Some(origin_y.saturating_add(glyph_y)),
        VerticalAlign::Superscript => {
            let y = glyph_y.saturating_sub(offset);
            Some(origin_y.saturating_add(y))
        }
        VerticalAlign::Subscript => {
            let y = glyph_y.saturating_add(offset);
            (y < cell_height).then_some(origin_y.saturating_add(y))
        }
    }
}

fn clipped_cell_width(draw_x: u32, origin_x: u32, cell_width: u32, width: u32) -> Option<u32> {
    let cell_right = origin_x.saturating_add(cell_width);
    if draw_x >= cell_right {
        None
    } else {
        Some(width.min(cell_right - draw_x))
    }
}

fn for_each_opaque_glyph_row_run(
    row_bits: u8,
    origin_x: u32,
    cell_width: u32,
    scale_x: u32,
    row_offset: u32,
    bold: bool,
    mut callback: impl FnMut(u32, u32),
) {
    if !bold {
        let mut coverage = u16::from(row_bits);
        while coverage != 0 {
            let run_start = coverage.trailing_zeros();
            let run_len = (coverage >> run_start).trailing_ones();
            let run_mask =
                u16::try_from(((1_u32 << run_len) - 1).checked_shl(run_start).unwrap_or(0))
                    .unwrap_or(u16::MAX);
            coverage &= !run_mask;
            let draw_x = origin_x
                .saturating_add(run_start.saturating_mul(scale_x))
                .saturating_add(row_offset);
            let run_width = run_len.saturating_mul(scale_x);
            if let Some(width) = clipped_cell_width(draw_x, origin_x, cell_width, run_width) {
                callback(draw_x, width);
            }
        }
        return;
    }

    let mut run = None::<(u32, u32)>;
    {
        let mut include_interval = |start: u32, width: u32| {
            if width == 0 {
                return;
            }
            let end = start.saturating_add(width);
            match run {
                Some((run_start, run_end)) if start <= run_end => {
                    run = Some((run_start, run_end.max(end)));
                }
                Some((run_start, run_end)) => {
                    callback(run_start, run_end.saturating_sub(run_start));
                    run = Some((start, end));
                }
                None => run = Some((start, end)),
            }
        };
        for glyph_x in 0_u32..8 {
            if row_bits & (1 << glyph_x) == 0 {
                continue;
            }
            let draw_x = origin_x
                .saturating_add(glyph_x.saturating_mul(scale_x))
                .saturating_add(row_offset);
            if let Some(width) = clipped_cell_width(draw_x, origin_x, cell_width, scale_x) {
                include_interval(draw_x, width);
            }
            let bold_x = draw_x.saturating_add(scale_x);
            if bold && bold_x < origin_x.saturating_add(cell_width) {
                include_interval(bold_x, scale_x);
            }
        }
    }
    if let Some((run_start, run_end)) = run {
        callback(run_start, run_end.saturating_sub(run_start));
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "decoration rendering shares the complete font metric and color context"
)]
fn render_text_decorations(
    surface: &mut Surface<'_>,
    cell: &RenderCell,
    cell_rect: Rect,
    foreground: [u8; 4],
    underline_color: [u8; 4],
    foreground_alpha: u8,
    underline_thickness: Option<RenderUnderlineThickness>,
    underline_position: Option<RenderUnderlinePosition>,
    strikethrough_position: Option<RenderStrikethroughPosition>,
    window_dpi: u32,
) {
    render_underline_style(
        surface,
        cell,
        cell_rect,
        underline_color,
        foreground_alpha,
        underline_thickness,
        underline_position,
        window_dpi,
    );

    if cell.overline {
        let overline_height = (cell_rect.height / 8).max(1);
        surface.fill_rect_alpha(
            Rect {
                x: cell_rect.x,
                y: cell_rect.y,
                width: cell_rect.width,
                height: overline_height,
            },
            foreground,
            foreground_alpha,
        );
    }

    if cell.strikethrough {
        let strike_height = (cell_rect.height / 8).max(1);
        let strike_y = cell_rect.y.saturating_add(strikethrough_position_px(
            strikethrough_position,
            cell_rect.height,
            strike_height,
            window_dpi,
        ));
        surface.fill_rect_alpha(
            Rect {
                x: cell_rect.x,
                y: strike_y,
                width: cell_rect.width,
                height: strike_height,
            },
            foreground,
            foreground_alpha,
        );
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "underline rendering shares the complete font metric and color context"
)]
fn render_underline_style(
    surface: &mut Surface<'_>,
    cell: &RenderCell,
    cell_rect: Rect,
    underline_color: [u8; 4],
    foreground_alpha: u8,
    underline_thickness: Option<RenderUnderlineThickness>,
    underline_position: Option<RenderUnderlinePosition>,
    window_dpi: u32,
) {
    let style = effective_underline_style(cell);
    if style == UnderlineStyle::None {
        return;
    }

    let underline_height =
        underline_thickness_px(underline_thickness, cell_rect.height, window_dpi);
    let lower_y = cell_rect.y.saturating_add(underline_position_px(
        underline_position,
        cell_rect.height,
        underline_height,
        window_dpi,
    ));
    let lower_rect = Rect {
        x: cell_rect.x,
        y: lower_y,
        width: cell_rect.width,
        height: underline_height,
    };

    match style {
        UnderlineStyle::None => {}
        UnderlineStyle::Single => {
            surface.fill_rect_alpha(lower_rect, underline_color, foreground_alpha);
        }
        UnderlineStyle::Double => {
            surface.fill_rect_alpha(lower_rect, underline_color, foreground_alpha);
            surface.fill_rect_alpha(
                Rect {
                    y: lower_y.saturating_sub(underline_height.saturating_mul(2)),
                    ..lower_rect
                },
                underline_color,
                foreground_alpha,
            );
        }
        UnderlineStyle::Curly => {
            render_curly_underline(surface, lower_rect, underline_color, foreground_alpha);
        }
        UnderlineStyle::Dotted => {
            render_patterned_underline(
                surface,
                lower_rect,
                underline_color,
                foreground_alpha,
                1,
                1,
            );
        }
        UnderlineStyle::Dashed => {
            render_patterned_underline(
                surface,
                lower_rect,
                underline_color,
                foreground_alpha,
                3,
                2,
            );
        }
    }
}

#[doc(hidden)]
#[must_use]
pub fn effective_underline_style(cell: &RenderCell) -> UnderlineStyle {
    match cell.underline_style {
        UnderlineStyle::None if cell.double_underline => UnderlineStyle::Double,
        UnderlineStyle::None if cell.underline => UnderlineStyle::Single,
        style => style,
    }
}

fn render_patterned_underline(
    surface: &mut Surface<'_>,
    rect: Rect,
    color: [u8; 4],
    alpha: u8,
    stroke_width: u32,
    gap_width: u32,
) {
    let cycle = stroke_width.saturating_add(gap_width).max(1);
    let mut offset = 0;
    while offset < rect.width {
        let segment_width = stroke_width.min(rect.width - offset);
        surface.fill_rect_alpha(
            Rect {
                x: rect.x + offset,
                width: segment_width,
                ..rect
            },
            color,
            alpha,
        );
        offset = offset.saturating_add(cycle);
    }
}

fn render_curly_underline(surface: &mut Surface<'_>, rect: Rect, color: [u8; 4], alpha: u8) {
    let upper_y = rect.y.saturating_sub(rect.height);
    for offset in 0..rect.width {
        let y = if (offset / 2) % 2 == 0 {
            upper_y
        } else {
            rect.y
        };
        surface.fill_rect_alpha(
            Rect {
                x: rect.x + offset,
                y,
                width: 1,
                height: rect.height,
            },
            color,
            alpha,
        );
    }
}

fn render_cursor(
    surface: &mut Surface<'_>,
    cursor: RenderCursor,
    cell: Option<&RenderCell>,
    cell_width: u32,
    cell_height: u32,
    style: CursorRenderStyle,
) {
    if cursor.blinking && !style.blink_visible {
        return;
    }

    let origin_x = u32::from(cursor.column).saturating_mul(cell_width);
    let origin_y = u32::from(cursor.row).saturating_mul(cell_height);
    let rect = cursor_rect(
        cursor.shape,
        origin_x,
        origin_y,
        cell_width,
        cell_height,
        style.thickness,
        style.window_dpi,
    );
    let cursor_alpha = if cursor.blinking {
        style.opacity_alpha
    } else {
        u8::MAX
    };
    if cursor_alpha < u8::MAX {
        surface.fill_rect_alpha(rect, style.color, cursor_alpha);
    } else {
        surface.fill_rect(rect, style.color);
    }

    if cursor.shape == CursorShape::Block
        && let Some(border) = style.border
    {
        surface.stroke_rect(rect, border, cursor_alpha);
    }

    if cursor.shape == CursorShape::Block
        && let (Some(cell), Some(foreground)) = (cell, style.foreground)
    {
        render_cursor_cell_foreground(surface, cell, cell_width, cell_height, foreground);
    }
}

fn render_cursor_cell_foreground(
    surface: &mut Surface<'_>,
    cell: &RenderCell,
    cell_width: u32,
    cell_height: u32,
    foreground: [u8; 4],
) {
    if cell.conceal {
        return;
    }
    let Some(glyph) = BASIC_FONTS.get(cell.ch) else {
        return;
    };

    let origin_x = u32::from(cell.column).saturating_mul(cell_width);
    let origin_y = u32::from(cell.row).saturating_mul(cell_height);
    let scale_x = cell_width.max(8) / 8;
    let scale_y = cell_height.max(8) / 8;

    for (glyph_y, row_bits) in glyph.iter().enumerate() {
        let row_offset = italic_row_offset(glyph_y, scale_x, cell.italic);
        for glyph_x in 0..8 {
            if row_bits & (1 << glyph_x) == 0 {
                continue;
            }

            let draw_x = origin_x + glyph_x * scale_x + row_offset;
            let Some(draw_y) = vertical_aligned_y(
                origin_y,
                cell_height,
                u32::try_from(glyph_y).unwrap_or(0) * scale_y,
                cell.vertical_align,
            ) else {
                continue;
            };
            let Some(width) = clipped_cell_width(draw_x, origin_x, cell_width, scale_x) else {
                continue;
            };
            surface.fill_rect(
                Rect {
                    x: draw_x,
                    y: draw_y,
                    width,
                    height: scale_y,
                },
                foreground,
            );
        }
    }
}

#[derive(Clone, Copy)]
pub struct CursorColors {
    pub color: [u8; 4],
    pub foreground: Option<[u8; 4]>,
}

#[expect(
    clippy::too_many_arguments,
    reason = "cursor color resolution depends on the complete renderer palette context"
)]
#[doc(hidden)]
#[must_use]
pub fn cursor_colors(
    snapshot: &TerminalRenderSnapshot,
    cursor: RenderCursor,
    force_reverse_video_cursor: bool,
    reverse_video_cursor_min_contrast: Option<u16>,
    bold_brightens_ansi_colors: RenderBoldBrightensAnsiColors,
    default_foreground: [u8; 4],
    default_background: [u8; 4],
    ansi_palette: Option<&[[u8; 4]; 16]>,
    indexed_palette: Option<&RenderIndexedPalette>,
    default_cursor_color: [u8; 4],
    default_cursor_foreground: Option<[u8; 4]>,
) -> CursorColors {
    if let Some(color) = snapshot.cursor_color() {
        return CursorColors {
            color: color_to_rgba_with_palette(
                color,
                default_foreground,
                ansi_palette,
                indexed_palette,
            ),
            foreground: None,
        };
    }

    if !force_reverse_video_cursor {
        return CursorColors {
            color: default_cursor_color,
            foreground: default_cursor_foreground,
        };
    }

    let Some(cell) = snapshot
        .iter_cells()
        .find(|cell| cell.row == cursor.row && cell.column == cursor.column)
    else {
        return CursorColors {
            color: default_foreground,
            foreground: None,
        };
    };
    let (reverse_background, reverse_foreground) = effective_cell_colors(
        cell,
        bold_brightens_ansi_colors,
        default_foreground,
        default_background,
        ansi_palette,
        indexed_palette,
    );

    if reverse_video_cursor_min_contrast.is_some_and(|minimum| {
        contrast_ratio(reverse_foreground, reverse_background) < f64::from(minimum) / 100.0
    }) {
        CursorColors {
            color: default_cursor_color,
            foreground: default_cursor_foreground,
        }
    } else {
        CursorColors {
            color: reverse_background,
            foreground: None,
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn contrast_ratio_to_centi(value: f64) -> Option<u16> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    Some((value * 100.0).round().min(f64::from(u16::MAX)) as u16)
}

fn contrast_ratio(foreground: [u8; 4], background: [u8; 4]) -> f64 {
    let foreground_luminance = relative_luminance(foreground);
    let background_luminance = relative_luminance(background);
    let light = foreground_luminance.max(background_luminance);
    let dark = foreground_luminance.min(background_luminance);
    (light + 0.05) / (dark + 0.05)
}

fn relative_luminance(color: [u8; 4]) -> f64 {
    let red = linear_srgb_component(color[0]);
    let green = linear_srgb_component(color[1]);
    let blue = linear_srgb_component(color[2]);
    0.2126 * red + 0.7152 * green + 0.0722 * blue
}

fn linear_srgb_component(channel: u8) -> f64 {
    let value = f64::from(channel) / 255.0;
    if value <= 0.03928 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[doc(hidden)]
#[must_use]
pub fn cursor_shape_default_color(
    cursor: RenderCursor,
    default_cursor_color: [u8; 4],
    default_cursor_border: Option<[u8; 4]>,
) -> [u8; 4] {
    if cursor.shape == CursorShape::Block {
        default_cursor_color
    } else {
        default_cursor_border.unwrap_or(default_cursor_color)
    }
}

#[doc(hidden)]
#[must_use]
pub fn configured_cursor_border(
    snapshot: &TerminalRenderSnapshot,
    force_reverse_video_cursor: bool,
    default_cursor_border: Option<[u8; 4]>,
) -> Option<[u8; 4]> {
    if force_reverse_video_cursor || snapshot.cursor_color().is_some() {
        None
    } else {
        default_cursor_border
    }
}

fn damage_rect(region: DamageRegion, cell_width: u32, cell_height: u32) -> Rect {
    Rect {
        x: u32::from(region.x).saturating_mul(cell_width),
        y: u32::from(region.y).saturating_mul(cell_height),
        width: u32::from(region.width).saturating_mul(cell_width),
        height: u32::from(region.height).saturating_mul(cell_height),
    }
}

fn damage_contains_cell(damage: &[DamageRegion], row: u16, column: u16) -> bool {
    damage.iter().copied().any(|region| {
        !region.is_empty()
            && row >= region.y
            && row < region.y.saturating_add(region.height)
            && column >= region.x
            && column < region.x.saturating_add(region.width)
    })
}

#[doc(hidden)]
#[must_use]
pub fn scrollbar_thumb_rect(
    scrollbar: ScrollbackScrollbar,
    geometry: RenderGeometry,
    track_width: u32,
    window_dpi: u32,
) -> Rect {
    let thumb_height = scrollbar_thumb_height(scrollbar, geometry, window_dpi);
    let travel = geometry.target_height.saturating_sub(thumb_height);
    let scrollback_lines = scrollbar.scrollback_lines as u64;
    let live_distance = scrollbar
        .scrollback_lines
        .saturating_sub(scrollbar.scrollback_offset) as u64;
    let thumb_y = if scrollback_lines == 0 {
        0
    } else {
        u32::try_from(u64::from(travel).saturating_mul(live_distance) / scrollback_lines)
            .unwrap_or(travel)
    };

    Rect {
        x: geometry.target_width.saturating_sub(track_width),
        y: thumb_y.min(travel),
        width: track_width,
        height: thumb_height,
    }
}

fn scrollbar_thumb_height(
    scrollbar: ScrollbackScrollbar,
    geometry: RenderGeometry,
    window_dpi: u32,
) -> u32 {
    let viewport_rows = u64::from(scrollbar.viewport_rows);
    let total_rows = viewport_rows.saturating_add(scrollbar.scrollback_lines as u64);
    let target_height = geometry.target_height;
    let target_height_u64 = u64::from(target_height);
    let proportional_height = if total_rows == 0 {
        target_height_u64
    } else {
        target_height_u64.saturating_mul(viewport_rows) / total_rows
    };
    let min_thumb_height =
        scrollbar_min_thumb_height(scrollbar.min_thumb_height, geometry, window_dpi).max(1);

    u32::try_from(proportional_height)
        .unwrap_or(target_height)
        .max(min_thumb_height)
        .min(target_height)
}

fn scrollbar_min_thumb_height(
    min_thumb_height: Option<RenderScrollbarThumbSize>,
    geometry: RenderGeometry,
    window_dpi: u32,
) -> u32 {
    match min_thumb_height {
        Some(RenderScrollbarThumbSize::Pixels(pixels)) => pixels,
        Some(RenderScrollbarThumbSize::Points(points)) => points_to_pixels(points, window_dpi),
        Some(RenderScrollbarThumbSize::CellFractionPerMille(per_mille)) => {
            geometry.cell_height.saturating_mul(per_mille) / 1_000
        }
        Some(RenderScrollbarThumbSize::Percent(percent)) => {
            geometry.target_height.saturating_mul(percent) / 100
        }
        None => geometry.cell_height.div_ceil(2),
    }
}

#[must_use]
pub fn color_to_rgba(color: Color, default: [u8; 4]) -> [u8; 4] {
    color_to_rgba_with_palette(color, default, None, None)
}

#[doc(hidden)]
#[must_use]
pub fn color_to_rgba_with_palette(
    color: Color,
    default: [u8; 4],
    ansi_palette: Option<&[[u8; 4]; 16]>,
    indexed_palette: Option<&RenderIndexedPalette>,
) -> [u8; 4] {
    match color {
        Color::Default => default,
        Color::Indexed(index) => indexed_color(index, ansi_palette, indexed_palette),
        Color::Rgb(red, green, blue) => [red, green, blue, 255],
        Color::Rgba(red, green, blue, alpha) => [red, green, blue, alpha],
    }
}

fn dim_foreground(color: [u8; 4]) -> [u8; 4] {
    [color[0] / 2, color[1] / 2, color[2] / 2, color[3]]
}

fn indexed_color(
    index: u8,
    ansi_palette: Option<&[[u8; 4]; 16]>,
    indexed_palette: Option<&RenderIndexedPalette>,
) -> [u8; 4] {
    let ansi_palette = ansi_palette.unwrap_or(&DEFAULT_ANSI_PALETTE);
    if let Some(color) = ansi_palette.get(usize::from(index)) {
        return *color;
    }

    if let Some(color) = indexed_palette
        .and_then(|palette| palette.get(usize::from(index)))
        .copied()
        .flatten()
    {
        return color;
    }

    if (16..=231).contains(&index) {
        let cube_index = index - 16;
        let red = xterm_color_cube_intensity(cube_index / 36);
        let green = xterm_color_cube_intensity((cube_index / 6) % 6);
        let blue = xterm_color_cube_intensity(cube_index % 6);
        return [red, green, blue, 255];
    }

    let level = 8 + (index - 232) * 10;
    [level, level, level, 255]
}

const fn xterm_color_cube_intensity(value: u8) -> u8 {
    if value == 0 { 0 } else { 55 + value * 40 }
}

const fn default_foreground() -> [u8; 4] {
    [229, 229, 229, 255]
}

const fn default_background() -> [u8; 4] {
    [12, 12, 12, 255]
}

#[doc(hidden)]
#[must_use]
pub fn cursor_rect(
    shape: CursorShape,
    origin_x: u32,
    origin_y: u32,
    cell_width: u32,
    cell_height: u32,
    cursor_thickness: Option<RenderCursorThickness>,
    window_dpi: u32,
) -> Rect {
    match shape {
        CursorShape::Block => Rect {
            x: origin_x,
            y: origin_y,
            width: cell_width,
            height: cell_height,
        },
        CursorShape::Underline => {
            let height =
                cursor_thickness_px(cursor_thickness, cell_height, cell_height, window_dpi);
            Rect {
                x: origin_x,
                y: origin_y + cell_height.saturating_sub(height),
                width: cell_width,
                height,
            }
        }
        CursorShape::Bar => Rect {
            x: origin_x,
            y: origin_y,
            width: cursor_thickness_px(cursor_thickness, cell_width, cell_height, window_dpi),
            height: cell_height,
        },
    }
}

fn cursor_thickness_px(
    cursor_thickness: Option<RenderCursorThickness>,
    max_thickness: u32,
    cell_height: u32,
    window_dpi: u32,
) -> u32 {
    let default_thickness = (cell_height / 6).max(1);
    let thickness = match cursor_thickness {
        Some(RenderCursorThickness::Pixels(pixels)) => pixels,
        Some(RenderCursorThickness::Points(points)) => points_to_pixels(points, window_dpi),
        Some(RenderCursorThickness::Percent(percent)) => {
            default_thickness.saturating_mul(percent) / 100
        }
        Some(RenderCursorThickness::CellFractionPerMille(per_mille)) => {
            cell_height.saturating_mul(per_mille) / 1_000
        }
        None => default_thickness,
    };

    thickness.max(1).min(max_thickness)
}

#[doc(hidden)]
#[must_use]
pub fn underline_thickness_px(
    underline_thickness: Option<RenderUnderlineThickness>,
    cell_height: u32,
    window_dpi: u32,
) -> u32 {
    let default_thickness = (cell_height / 8).max(1);
    let thickness = match underline_thickness {
        Some(RenderUnderlineThickness::Pixels(pixels)) => pixels,
        Some(RenderUnderlineThickness::Points(points)) => points_to_pixels(points, window_dpi),
        Some(RenderUnderlineThickness::Percent(percent)) => {
            default_thickness.saturating_mul(percent) / 100
        }
        Some(RenderUnderlineThickness::CellFractionPerMille(per_mille)) => {
            cell_height.saturating_mul(per_mille) / 1_000
        }
        None => default_thickness,
    };

    thickness.max(1).min(cell_height)
}

#[doc(hidden)]
#[must_use]
pub fn underline_position_px(
    underline_position: Option<RenderUnderlinePosition>,
    cell_height: u32,
    underline_height: u32,
    window_dpi: u32,
) -> u32 {
    let default_position = cell_height.saturating_sub(underline_height);
    let offset = match underline_position {
        Some(RenderUnderlinePosition::Pixels(pixels)) => pixels,
        Some(RenderUnderlinePosition::Points(points)) => {
            signed_points_to_pixels(points, window_dpi)
        }
        Some(RenderUnderlinePosition::Percent(percent)) => {
            signed_scaled(default_position, percent.saturating_sub(100), 100)
        }
        Some(RenderUnderlinePosition::CellFractionPerMille(per_mille)) => {
            signed_scaled(cell_height, per_mille, 1_000)
        }
        None => return default_position,
    };

    shifted_line_position(default_position, offset, cell_height, underline_height)
}

#[doc(hidden)]
#[must_use]
pub fn strikethrough_position_px(
    strikethrough_position: Option<RenderStrikethroughPosition>,
    cell_height: u32,
    strike_height: u32,
    window_dpi: u32,
) -> u32 {
    let default_position = cell_height
        .saturating_div(2)
        .saturating_sub(strike_height.saturating_div(2));
    let position = match strikethrough_position {
        Some(RenderStrikethroughPosition::Pixels(pixels)) => pixels,
        Some(RenderStrikethroughPosition::Points(points)) => points_to_pixels(points, window_dpi),
        Some(RenderStrikethroughPosition::Percent(percent)) => {
            default_position.saturating_mul(percent) / 100
        }
        Some(RenderStrikethroughPosition::CellFractionPerMille(per_mille)) => {
            cell_height.saturating_mul(per_mille) / 1_000
        }
        None => default_position,
    };

    position.min(cell_height.saturating_sub(strike_height.min(cell_height)))
}

fn shifted_line_position(
    default_position: u32,
    offset: i32,
    cell_height: u32,
    line_height: u32,
) -> u32 {
    let max_position = i64::from(cell_height.saturating_sub(line_height.min(cell_height)));
    let position = i64::from(default_position).saturating_add(i64::from(offset));
    u32::try_from(position.clamp(0, max_position)).unwrap_or(0)
}

fn signed_scaled(value: u32, factor: i32, divisor: i32) -> i32 {
    let scaled = i64::from(value).saturating_mul(i64::from(factor)) / i64::from(divisor.max(1));
    i32::try_from(scaled).unwrap_or(if scaled.is_negative() {
        i32::MIN
    } else {
        i32::MAX
    })
}

fn signed_points_to_pixels(points: i32, dpi: u32) -> i32 {
    let numerator = i64::from(points).saturating_mul(i64::from(dpi));
    let rounded = if numerator.is_negative() {
        numerator.saturating_sub(36) / 72
    } else {
        numerator.saturating_add(36) / 72
    };
    i32::try_from(rounded).unwrap_or(if rounded.is_negative() {
        i32::MIN
    } else {
        i32::MAX
    })
}

fn points_to_pixels(points: u32, dpi: u32) -> u32 {
    let numerator = u64::from(points).saturating_mul(u64::from(dpi));
    u32::try_from((numerator + 36) / 72).unwrap_or(u32::MAX)
}

#[cfg(test)]
#[path = "renderer_tests.rs"]
mod tests;
