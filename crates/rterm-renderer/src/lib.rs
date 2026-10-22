//! One-stage compatibility facade for the split R-Term renderer packages.

pub use rterm_render_core as core;
pub use rterm_render_core::{
    DamageRegion, RenderCell, RenderCellColorRole, RenderCursor, RenderGeometry,
    RenderIndexedPalette, RenderInlineImage, RenderInlineImageFragment, SCROLLBAR_THUMB_COLOR,
    SCROLLBAR_TRACK_COLOR, SCROLLBAR_WIDTH, TerminalContentDigest, TerminalRenderSnapshot,
    terminal_bytes_content_digest, terminal_snapshot_content_digest,
};
pub use rterm_render_cpu as cpu;
pub use rterm_render_cpu::{
    CpuTextRenderReport, CpuTextRenderer, DecodedImage, ImageDrawLayer, ImageDrawPlan,
    ImageTiePolicy, PixelRenderer, RenderBackgroundGradient, RenderBackgroundGradientBlend,
    RenderBackgroundGradientHsb, RenderBackgroundGradientInterpolation,
    RenderBackgroundGradientOrientation, RenderBackgroundGradientPreset,
    RenderBackgroundGradientSegment, RenderBackgroundImage, RenderBackgroundImageAttachment,
    RenderBackgroundImageDimension, RenderBackgroundImageHorizontalAlign,
    RenderBackgroundImageLength, RenderBackgroundImageRepeat, RenderBackgroundImageVerticalAlign,
    RenderBackgroundLayer, RenderBoldBrightensAnsiColors, RenderCursorThickness,
    RenderScrollbarThumbSize, RenderStrikethroughPosition, RenderUnderlinePosition,
    RenderUnderlineThickness, RenderedClusterBounds, ScrollbackScrollbar, TextBackend,
    TextPaintConfig, TextPixelBounds, background_gradient_color_strings, color_to_rgba,
    compare_image_draw_plans, terminal_first_row_pixel_digest,
};
pub use rterm_render_wgpu::{GpuFramePlanner, gpu};
