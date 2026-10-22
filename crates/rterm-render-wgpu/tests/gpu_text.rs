use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use rterm_fonts::{FontCatalog, FontConfig, FontSource, RasterCacheConfig};
use rterm_render_cpu::{
    CpuTextRenderer, DamageRegion, PixelRenderer, RenderBoldBrightensAnsiColors, RenderGeometry,
    TerminalRenderSnapshot, TextPaintConfig, terminal_snapshot_content_digest,
};
use rterm_render_wgpu::gpu::{
    GpuContext, GpuContextOptions, GpuImage, GpuLayer, GpuLayerRenderer, GpuQuad, GpuTextConfig,
    ImageProtocol, PixelRect, RenderGraph,
};
use rterm_terminal::{CursorShape, Terminal};
use rterm_types::TerminalSize;

static GPU_TEST_LOCK: Mutex<()> = Mutex::new(());

fn gpu_test_guard() -> MutexGuard<'static, ()> {
    GPU_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
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

fn snapshot(text: &str, columns: u16) -> TerminalRenderSnapshot {
    let mut terminal = Terminal::new(TerminalSize::new(columns, 1));
    terminal.feed(b"\x1b[?25l");
    terminal.feed(text.as_bytes());
    TerminalRenderSnapshot::from_terminal(&terminal)
}

#[test]
fn structured_terminal_digest_preserves_order_spaces_and_cell_positions() {
    let ab = snapshot("ab", 4);
    let ba = snapshot("ba", 4);
    let spaced = snapshot("a b", 4);
    let compact = snapshot("ab", 4);
    let misplaced = snapshot("a\x1b[2Cb", 4);

    assert_ne!(
        terminal_snapshot_content_digest(&ab),
        terminal_snapshot_content_digest(&ba),
        "text order must be part of the terminal digest"
    );
    assert_ne!(
        terminal_snapshot_content_digest(&spaced),
        terminal_snapshot_content_digest(&compact),
        "blank-cell layout must be part of the terminal digest"
    );
    assert_ne!(
        terminal_snapshot_content_digest(&compact),
        terminal_snapshot_content_digest(&misplaced),
        "cell coordinates must be part of the terminal digest"
    );
}

fn multiline_snapshot(text: &str, columns: u16, rows: u16) -> TerminalRenderSnapshot {
    let mut terminal = Terminal::new(TerminalSize::new(columns, rows));
    terminal.feed(b"\x1b[?25l");
    terminal.feed(text.as_bytes());
    TerminalRenderSnapshot::from_terminal(&terminal)
}

fn config(atlas_budget_bytes: usize) -> GpuTextConfig {
    GpuTextConfig::new(
        atlas_budget_bytes,
        RasterCacheConfig::new(atlas_budget_bytes),
    )
}

mod lazy_cursor_renderer {
    use super::*;

    #[test]
    fn cursor_renderer_materializes_once_on_first_cursor_foreground() {
        let _gpu = gpu_test_guard();
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless adapter");
        let mut renderer = GpuLayerRenderer::new_headless(&context, 4096).expect("layer renderer");
        renderer
            .enable_text(
                catalog(),
                font_config(),
                config(4 * 1024 * 1024).with_cursor_foreground([255, 0, 0, 255]),
            )
            .expect("enable GPU text");

        let after_enable = renderer.initialization_resources();
        assert_eq!(after_enable.base_text_renderer_materialization_count, 1);
        assert_eq!(after_enable.cursor_text_renderer_materialization_count, 0);

        let geometry = RenderGeometry::new(32, 24, 16, 24);
        renderer
            .prepare_text(
                &snapshot("A", 2),
                geometry,
                &[],
                &TextPaintConfig::default(),
                1.0,
                1.0,
            )
            .expect("ordinary hidden-cursor text");
        assert_eq!(
            renderer
                .initialization_resources()
                .cursor_text_renderer_materialization_count,
            0
        );

        let mut terminal = Terminal::new(TerminalSize::new(2, 1));
        terminal.feed(b"A\x1b[1D\x1b[2 q");
        let cursor_snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
        let report = renderer
            .prepare_text(
                &cursor_snapshot,
                geometry,
                &[],
                &TextPaintConfig::default(),
                1.0,
                1.0,
            )
            .expect("block cursor foreground");
        assert!(report.cursor_foreground_glyphs > 0);
        let first = renderer.initialization_resources();
        assert_eq!(first.cursor_text_renderer_materialization_count, 1);

        renderer
            .prepare_text(
                &cursor_snapshot,
                geometry,
                &[],
                &TextPaintConfig::default(),
                1.0,
                1.0,
            )
            .expect("repeated block cursor foreground");
        assert_eq!(renderer.initialization_resources(), first);
    }

    #[test]
    fn cursor_foreground_vertices_are_cleared_when_cursor_is_not_redrawn() {
        let _gpu = gpu_test_guard();
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless adapter");
        let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096)
            .expect("layer renderer");
        renderer
            .enable_text(
                catalog(),
                font_config(),
                config(4 * 1024 * 1024).with_cursor_foreground([255, 0, 0, 255]),
            )
            .expect("enable GPU text");

        let geometry = RenderGeometry::new(32, 24, 16, 24);
        let mut terminal = Terminal::new(TerminalSize::new(2, 1));
        terminal.feed(b"A\x1b[1D\x1b[2 q");
        renderer
            .prepare_text(
                &TerminalRenderSnapshot::from_terminal(&terminal),
                geometry,
                &[],
                &TextPaintConfig::default(),
                1.0,
                1.0,
            )
            .expect("prepare visible block cursor foreground");
        let visible = render_prepared(&mut renderer, geometry, [0, 0, 0, 255]);
        let is_red = |pixel: &[u8]| pixel[0] > 200 && pixel[1] < 50 && pixel[2] < 50;
        assert!(visible.chunks_exact(4).any(is_red));

        renderer
            .prepare_text(
                &snapshot("A", 2),
                geometry,
                &[],
                &TextPaintConfig::default(),
                1.0,
                1.0,
            )
            .expect("prepare hidden cursor frame");
        let hidden = render_prepared(&mut renderer, geometry, [0, 0, 0, 255]);
        assert!(
            !hidden.chunks_exact(4).any(is_red),
            "hidden cursor must not retain the previous cursor foreground vertices"
        );

        terminal.feed(b"\x1b[6 q");
        renderer
            .prepare_text(
                &TerminalRenderSnapshot::from_terminal(&terminal),
                geometry,
                &[],
                &TextPaintConfig::default(),
                1.0,
                1.0,
            )
            .expect("prepare bar cursor frame");
        let bar = render_prepared(&mut renderer, geometry, [0, 0, 0, 255]);
        assert!(
            !bar.chunks_exact(4).any(is_red),
            "non-block cursor must not retain the previous block-cursor foreground vertices"
        );
    }
}

fn render_gpu_text(
    snapshot: &TerminalRenderSnapshot,
    paint: &TextPaintConfig,
    geometry: RenderGeometry,
) -> Vec<u8> {
    render_gpu_text_with_quads(snapshot, paint, geometry, &[])
}

fn render_gpu_text_with_quads(
    snapshot: &TerminalRenderSnapshot,
    paint: &TextPaintConfig,
    geometry: RenderGeometry,
    quads: &[GpuQuad],
) -> Vec<u8> {
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 64 * 1024)
        .expect("layer renderer");
    renderer
        .enable_text(catalog(), font_config(), config(4 * 1024 * 1024))
        .expect("enable GPU text");
    renderer
        .prepare_text(snapshot, geometry, &[], paint, 1.0, 1.0)
        .expect("prepare GPU text");
    let mut graph = RenderGraph::new(geometry.target_width, geometry.target_height);
    graph.push_quad(GpuQuad::new(
        GpuLayer::PaneBackground,
        PixelRect::new(0, 0, geometry.target_width, geometry.target_height),
        paint.default_background,
    ));
    for quad in quads {
        graph.push_quad(*quad);
    }
    renderer
        .render_headless_rgba8(&graph, Duration::from_secs(5))
        .expect("render GPU text")
}

fn render_cpu_text(
    snapshot: &TerminalRenderSnapshot,
    renderer: &PixelRenderer,
    geometry: RenderGeometry,
) -> Vec<u8> {
    let mut text = CpuTextRenderer::new(
        catalog(),
        font_config(),
        RasterCacheConfig::new(4 * 1024 * 1024),
    );
    let mut pixels = vec![0; geometry.target_width as usize * geometry.target_height as usize * 4];
    renderer.render_shaped(&mut text, snapshot, &mut pixels, geometry);
    pixels
}

fn render_prepared(
    renderer: &mut GpuLayerRenderer,
    geometry: RenderGeometry,
    background: [u8; 4],
) -> Vec<u8> {
    let mut graph = RenderGraph::new(geometry.target_width, geometry.target_height);
    graph.push_quad(GpuQuad::new(
        GpuLayer::PaneBackground,
        PixelRect::new(0, 0, geometry.target_width, geometry.target_height),
        background,
    ));
    renderer
        .render_headless_rgba8(&graph, Duration::from_secs(5))
        .expect("render prepared GPU text")
}

fn rgb_energy(pixels: &[u8], background: [u8; 4]) -> u64 {
    pixels
        .chunks_exact(4)
        .map(|pixel| {
            u64::from((i16::from(pixel[0]) - i16::from(background[0])).unsigned_abs())
                + u64::from((i16::from(pixel[1]) - i16::from(background[1])).unsigned_abs())
                + u64::from((i16::from(pixel[2]) - i16::from(background[2])).unsigned_abs())
        })
        .sum()
}

fn cell_rgb_energy(pixels: &[u8], frame_width: usize, column: usize, background: [u8; 4]) -> u64 {
    (0..24)
        .map(|row| {
            let start = (row * frame_width + column * 16) * 4;
            rgb_energy(&pixels[start..start + 16 * 4], background)
        })
        .sum()
}

fn strongest_cell_pixel(
    pixels: &[u8],
    frame_width: usize,
    column: usize,
    background: [u8; 4],
) -> [u8; 4] {
    (0..24)
        .flat_map(|row| {
            let start = (row * frame_width + column * 16) * 4;
            pixels[start..start + 16 * 4].chunks_exact(4)
        })
        .max_by_key(|pixel| {
            pixel[..3]
                .iter()
                .zip(background)
                .map(|(channel, background)| {
                    (i16::from(*channel) - i16::from(background)).unsigned_abs()
                })
                .sum::<u16>()
        })
        .map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
        .expect("nonempty cell")
}

#[test]
fn gpu_paint_matches_cpu_for_faint_masks_color_emoji_and_blink_phases() {
    let _gpu = gpu_test_guard();
    let geometry = RenderGeometry::new(64, 24, 16, 24);
    for (normal, faint) in [("A", "\x1b[2mA"), ("😀", "\x1b[2m😀")] {
        let mut cpu = PixelRenderer::new();
        cpu.set_text_blink_opacity(0.0);
        cpu.set_rapid_text_blink_opacity(1.0);
        let paint = cpu.text_paint_config();
        let cpu_normal = rgb_energy(
            &render_cpu_text(&snapshot(normal, 4), &cpu, geometry),
            paint.default_background,
        );
        let cpu_faint = rgb_energy(
            &render_cpu_text(&snapshot(faint, 4), &cpu, geometry),
            paint.default_background,
        );
        let gpu_normal = rgb_energy(
            &render_gpu_text(&snapshot(normal, 4), &paint, geometry),
            paint.default_background,
        );
        let gpu_faint = rgb_energy(
            &render_gpu_text(&snapshot(faint, 4), &paint, geometry),
            paint.default_background,
        );
        assert!(cpu_faint < cpu_normal, "CPU faint did not dim {normal:?}");
        assert!(gpu_faint < gpu_normal, "GPU faint did not dim {normal:?}");
        assert!(
            cpu_faint.saturating_mul(100) > cpu_normal.saturating_mul(35)
                && cpu_faint.saturating_mul(100) < cpu_normal.saturating_mul(60)
                && gpu_faint.saturating_mul(100) > gpu_normal.saturating_mul(8)
                && gpu_faint.saturating_mul(100) < gpu_normal.saturating_mul(35),
            "CPU sRGB and glyphon Accurate linear faint ratios left their matching semantic budgets for {normal:?}: cpu={cpu_faint}/{cpu_normal} gpu={gpu_faint}/{gpu_normal}"
        );
    }

    let mut cpu = PixelRenderer::new();
    cpu.set_text_blink_opacity(0.0);
    cpu.set_rapid_text_blink_opacity(1.0);
    let paint = cpu.text_paint_config();
    let blinking = snapshot("\x1b[5mA\x1b[0m\x1b[6mB", 4);
    let cpu_pixels = render_cpu_text(&blinking, &cpu, geometry);
    let gpu_pixels = render_gpu_text(&blinking, &paint, geometry);
    for pixels in [&cpu_pixels, &gpu_pixels] {
        assert_eq!(
            cell_rgb_energy(pixels, 64, 0, paint.default_background),
            0,
            "normal blink phase must be hidden"
        );
        assert!(
            cell_rgb_energy(pixels, 64, 1, paint.default_background) > 0,
            "rapid blink phase must remain visible"
        );
    }
}

#[test]
fn gpu_paint_uses_custom_palette_inverse_and_bold_bright_policy() {
    let _gpu = gpu_test_guard();
    let mut cpu = PixelRenderer::with_bold_brightens_ansi_colors(RenderBoldBrightensAnsiColors::No);
    let mut palette = [[0, 0, 0, 255]; 16];
    palette[1] = [17, 31, 47, 255];
    palette[9] = [211, 223, 239, 255];
    cpu.set_ansi_palette(Some(palette));
    let paint = cpu.text_paint_config();
    let geometry = RenderGeometry::new(16, 24, 16, 24);
    let bold = snapshot("\x1b[1;31mA", 1);
    let cpu_bold = render_cpu_text(&bold, &cpu, geometry);
    let gpu_bold = render_gpu_text(&bold, &paint, geometry);
    for pixel in [
        strongest_cell_pixel(&cpu_bold, 16, 0, paint.default_background),
        strongest_cell_pixel(&gpu_bold, 16, 0, paint.default_background),
    ] {
        assert!(
            pixel[2] > pixel[1] && pixel[1] > pixel[0] && pixel[2] < 100,
            "bold policy No must use the configured dark ANSI primary, not bright index 9: {pixel:?}"
        );
    }

    let inverse = snapshot("\x1b[7;31mA", 1);
    let inverse_background = palette[1];
    let cpu_inverse = render_cpu_text(&inverse, &cpu, geometry);
    let gpu_inverse = render_gpu_text_with_quads(
        &inverse,
        &paint,
        geometry,
        &[GpuQuad::new(
            GpuLayer::CellBackground,
            PixelRect::new(0, 0, 16, 24),
            inverse_background,
        )],
    );
    for pixels in [&cpu_inverse, &gpu_inverse] {
        assert!(
            pixels.chunks_exact(4).any(|pixel| {
                pixel[0] < inverse_background[0]
                    && pixel[1] < inverse_background[1]
                    && pixel[2] < inverse_background[2]
            }),
            "inverse text must swap the configured foreground into the cell background and draw the default background as glyph foreground"
        );
    }
}

#[test]
fn paint_phase_change_rebuilds_only_blink_rows_after_local_damage() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 64 * 1024)
        .expect("layer renderer");
    renderer
        .enable_text(catalog(), font_config(), config(4 * 1024 * 1024))
        .expect("enable GPU text");
    let snapshot = multiline_snapshot("\x1b[5mA\x1b[25m\r\nB", 2, 2);
    let geometry = RenderGeometry::new(32, 48, 16, 24);
    renderer
        .prepare_text(
            &snapshot,
            geometry,
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("prepare visible blink phase");
    let hidden = TextPaintConfig {
        text_blink_opacity_alpha: 0,
        rapid_text_blink_opacity_alpha: 0,
        ..TextPaintConfig::default()
    };
    let report = renderer
        .prepare_text(
            &snapshot,
            geometry,
            &[DamageRegion::new(0, 0, 1, 1)],
            &hidden,
            1.0,
            1.0,
        )
        .expect("prepare hidden blink phase");
    assert_eq!(report.prepared_rows, vec![0]);
    let pixels = render_prepared(&mut renderer, geometry, hidden.default_background);
    let row_energy = |row: usize| {
        let start = row * geometry.target_width as usize * 4;
        rgb_energy(
            &pixels[start..start + geometry.target_width as usize * 4],
            hidden.default_background,
        )
    };
    assert_eq!(row_energy(0), 0, "hidden blink row must be repainted");
}

#[test]
fn prepared_gpu_text_uses_shaped_runs_and_mask_and_color_atlas_entries() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096)
        .expect("layer renderer");
    renderer
        .enable_text(catalog(), font_config(), config(4 * 1024 * 1024))
        .expect("enable GPU text");

    let report = renderer
        .prepare_text(
            &snapshot("office 中 مرحبا नमस्ते שלום 😀 █", 48),
            RenderGeometry::new(48 * 16, 24, 16, 24),
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("prepare shaped GPU text");

    assert!(report.shaped_glyphs > 0);
    assert!(report.mask_glyphs > 0);
    assert!(report.color_glyphs > 0);
    assert!(report.custom_block_glyphs > 0);
    assert!(report.retained_glyphs > 0);
    assert!(report.retained_mask_glyphs > 0);
    assert!(report.retained_color_glyphs > 0);
    assert!(report.retained_custom_block_glyphs > 0);
    assert_eq!(report.second_shape_calls, 0);
    renderer
        .upload(&RenderGraph::new(48 * 16, 24))
        .expect("upload generated block glyph quad");
    assert_eq!(renderer.upload_metrics().bytes_written, 32);
}

#[test]
fn atlas_is_bounded_and_repack_retries_at_most_once() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096)
        .expect("layer renderer");
    let error = renderer
        .enable_text(catalog(), font_config(), config(16 * 1024))
        .expect_err("budget below the two initial physical textures must fail");
    assert!(
        error
            .to_string()
            .contains("initial mask and color textures")
    );

    renderer
        .enable_text(catalog(), font_config(), config(340 * 1024))
        .expect("enable minimally bounded GPU text");

    let error = renderer
        .prepare_text(
            &snapshot(
                "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789中🧑‍💻",
                80,
            ),
            RenderGeometry::new(80 * 32, 48, 32, 48),
            &[],
            &TextPaintConfig::default(),
            2.0,
            1.5,
        )
        .expect_err("tiny atlas budget must fail in a controlled way");

    assert!(error.to_string().contains("glyph atlas budget"));
    let metrics = renderer.text_atlas_metrics().expect("text metrics");
    assert!(metrics.retained_bytes <= metrics.budget_bytes);
    assert_eq!(
        metrics.physical_texture_bytes,
        metrics.mask_dimension as usize * metrics.mask_dimension as usize
            + metrics.color_dimension as usize * metrics.color_dimension as usize * 4
    );
    assert!(metrics.repack_attempts <= 1);
}

#[test]
fn repeated_frame_reuses_payloads_and_physical_atlas_without_growth() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096)
        .expect("layer renderer");
    renderer
        .enable_text(catalog(), font_config(), config(4 * 1024 * 1024))
        .expect("enable GPU text");
    let snapshot = snapshot("office 中 😀", 16);
    let geometry = RenderGeometry::new(16 * 16, 24, 16, 24);

    renderer
        .prepare_text(
            &snapshot,
            geometry,
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("first prepare");
    let first = renderer.text_atlas_metrics().expect("first metrics");
    renderer
        .prepare_text(
            &snapshot,
            geometry,
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("repeat prepare");
    let repeated = renderer.text_atlas_metrics().expect("repeat metrics");

    assert_eq!(repeated.uploads, first.uploads);
    assert_eq!(
        repeated.physical_texture_bytes,
        first.physical_texture_bytes
    );
    assert_eq!(repeated.entries, first.entries);
    assert_eq!(repeated.repack_attempts, 0);
    assert_eq!(first.trim_calls, 1);
    assert_eq!(repeated.trim_calls, 2);
}

#[test]
fn scale_and_font_generation_rebuild_the_complete_atlas_scope() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096)
        .expect("layer renderer");
    renderer
        .enable_text(catalog(), font_config(), config(4 * 1024 * 1024))
        .expect("enable GPU text");
    let snapshot = snapshot("scope", 8);
    let geometry = RenderGeometry::new(8 * 16, 24, 16, 24);

    renderer
        .prepare_text(
            &snapshot,
            geometry,
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("initial prepare");
    let initial = renderer.text_atlas_metrics().expect("initial metrics");
    renderer
        .prepare_text(
            &snapshot,
            geometry,
            &[],
            &TextPaintConfig::default(),
            2.0,
            1.0,
        )
        .expect("DPI prepare");
    let dpi = renderer.text_atlas_metrics().expect("DPI metrics");
    assert_eq!(dpi.scope_generation, initial.scope_generation + 1);
    renderer
        .prepare_text(
            &snapshot,
            geometry,
            &[],
            &TextPaintConfig::default(),
            2.0,
            1.5,
        )
        .expect("zoom prepare");
    let zoom = renderer.text_atlas_metrics().expect("zoom metrics");
    assert_eq!(zoom.scope_generation, dpi.scope_generation + 1);
    renderer
        .prepare_text(
            &snapshot,
            geometry,
            &[],
            &TextPaintConfig::default(),
            2.0,
            1.5,
        )
        .expect("same zoom prepare");
    let same_zoom = renderer.text_atlas_metrics().expect("same zoom metrics");
    assert_eq!(same_zoom.scope_generation, zoom.scope_generation);

    renderer
        .text_catalog_mut()
        .expect("text catalog")
        .load_source(source("NotoSansHebrew.fixture.ttf"))
        .expect("advance catalog generation");
    renderer
        .prepare_text(
            &snapshot,
            geometry,
            &[],
            &TextPaintConfig::default(),
            2.0,
            1.5,
        )
        .expect("font generation prepare");
    let font = renderer.text_atlas_metrics().expect("font metrics");
    assert_eq!(font.scope_generation, same_zoom.scope_generation + 1);
}

#[test]
fn catalog_generation_expansion_restarts_one_whole_frame_without_mixed_rows() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096)
        .expect("layer renderer");
    renderer
        .enable_text(
            FontCatalog::from_sources("en-US", [source("NotoSans-Latin.fixture.ttf")])
                .expect("Latin catalog"),
            font_config(),
            config(4 * 1024 * 1024),
        )
        .expect("enable GPU text");
    renderer
        .prepare_text(
            &multiline_snapshot("first\r\nsecond", 8, 2),
            RenderGeometry::new(8 * 16, 2 * 24, 16, 24),
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("prime the prior catalog generation");
    let snapshot = multiline_snapshot("first\r\nאבג", 8, 2);
    let geometry = RenderGeometry::new(8 * 16, 2 * 24, 16, 24);
    let expected_generation = renderer.text_catalog_generation().expect("text catalog");

    renderer
        .text_catalog_mut()
        .expect("text catalog")
        .load_source(source("NotoSansHebrew.fixture.ttf"))
        .expect("late Hebrew activation");
    let expansion = renderer
        .prepare_text_for_catalog_generation(
            expected_generation,
            &snapshot,
            geometry,
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("generation check");
    assert_eq!(
        expansion.catalog_expansion(),
        Some((expected_generation, expected_generation + 1))
    );

    let restarted = renderer
        .prepare_text_for_catalog_generation(
            expected_generation + 1,
            &snapshot,
            geometry,
            &[DamageRegion::new(0, 1, 1, 1)],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("one whole-frame restart");
    let report = restarted
        .into_prepared()
        .expect("a stable retry must prepare the frame");
    assert_eq!(report.prepared_rows, vec![0, 1]);
    assert_eq!(report.catalog_generation, expected_generation + 1);
    assert!(report.missing_glyphs.is_empty());
}

#[test]
fn retired_renderers_release_all_cpu_font_state_without_draining_the_replacement() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let make_renderer = || {
        let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096)
            .expect("layer renderer");
        renderer
            .enable_text(
                FontCatalog::from_sources(
                    "en-US",
                    [
                        source("NotoSans-Latin.fixture.ttf"),
                        source("NotoSansSC-CJK.fixture.ttf"),
                    ],
                )
                .expect("active catalog"),
                font_config(),
                config(4 * 1024 * 1024),
            )
            .expect("enable GPU text");
        renderer
    };
    let snapshot = multiline_snapshot("ASCII\r\n中文", 8, 2);
    let geometry = RenderGeometry::new(8 * 16, 2 * 24, 16, 24);
    let mut current = make_renderer();
    current
        .prepare_text(
            &snapshot,
            geometry,
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("prime font caches");
    let populated = current.text_cpu_font_metrics().expect("text metrics");
    assert_eq!(populated.catalog.active_source_count, 2);
    assert!(populated.catalog.retained_source_bytes > 0);
    assert!(populated.shape_cache_bytes > 0);
    assert!(populated.raster_cache_bytes > 0);
    assert!(populated.row_cache_entries > 0);
    assert!(populated.payload_bytes > 0);

    let mut retired = Vec::new();
    for _ in 0..3 {
        let replacement = make_renderer();
        let active = replacement
            .text_cpu_font_metrics()
            .expect("replacement metrics");
        assert_eq!(active.catalog.active_source_count, 2);
        assert!(active.catalog.retained_source_bytes > 0);

        current.retire_text_cpu_font_state();
        let released = current.text_cpu_font_metrics().expect("retired metrics");
        assert_eq!(released.catalog.active_source_count, 0);
        assert_eq!(released.catalog.retained_source_bytes, 0);
        assert_eq!(released.shape_cache_bytes, 0);
        assert_eq!(released.raster_cache_bytes, 0);
        assert_eq!(released.row_cache_entries, 0);
        assert_eq!(released.payload_bytes, 0);
        retired.push(current);
        current = replacement;
        assert_eq!(
            retired
                .iter()
                .map(|renderer| {
                    renderer
                        .text_cpu_font_metrics()
                        .expect("retired text metrics")
                        .catalog
                        .retained_source_bytes
                })
                .sum::<usize>(),
            0,
            "repeated recovery must not accumulate retired font bytes"
        );
    }
    assert_eq!(
        current
            .text_cpu_font_metrics()
            .expect("current metrics")
            .catalog
            .active_source_count,
        2
    );
}

#[test]
fn alternating_atlas_working_sets_evict_history_without_repack_or_scope_rebuild() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 64 * 1024)
        .expect("layer renderer");
    renderer
        .enable_text(catalog(), font_config(), config(520 * 1024))
        .expect("enable bounded GPU text");
    let geometry = RenderGeometry::new(40 * 32, 48, 32, 48);
    let paint = TextPaintConfig::default();
    let a = snapshot("ABCDEFGHIJKLMNOPQRSTUVWXYZ01234", 40);
    let b = snapshot("abcdefghijklmnopqrstuvwxyz56789", 40);

    renderer
        .prepare_text(&a, geometry, &[], &paint, 2.0, 1.5)
        .expect("working set A fits");
    let first = renderer.text_atlas_metrics().expect("A metrics");
    renderer
        .prepare_text(&b, geometry, &[], &paint, 2.0, 1.5)
        .expect("working set B evicts A placements");
    let second = renderer.text_atlas_metrics().expect("B metrics");
    renderer
        .prepare_text(&a, geometry, &[], &paint, 2.0, 1.5)
        .expect("working set A can return after B");
    let third = renderer.text_atlas_metrics().expect("second A metrics");

    assert_eq!(first.scope_generation, second.scope_generation);
    assert_eq!(second.scope_generation, third.scope_generation);
    assert_eq!(first.repack_attempts, 0);
    assert_eq!(second.repack_attempts, 0);
    assert_eq!(third.repack_attempts, 0);
    assert_eq!(
        [first.trim_calls, second.trim_calls, third.trim_calls],
        [1, 2, 3]
    );
}

#[test]
fn damaged_cell_expands_to_full_shaped_run_and_clips_to_half_open_terminal_bounds() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096)
        .expect("layer renderer");
    renderer
        .enable_text(catalog(), font_config(), config(4 * 1024 * 1024))
        .expect("enable GPU text");

    let report = renderer
        .prepare_text(
            &snapshot("office中", 8),
            RenderGeometry::new(8 * 16, 24, 16, 24),
            &[DamageRegion::new(2, 0, 1, 1)],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("prepare damaged shaped run");

    assert_eq!(report.prepared_rows, vec![0]);
    assert_eq!(report.damage_bounds, vec![PixelRect::new(0, 0, 8 * 16, 24)]);
    assert!(
        report
            .glyph_bounds
            .iter()
            .all(|bounds| bounds.x + bounds.width <= 8 * 16 && bounds.y + bounds.height <= 24)
    );
}

#[test]
fn gpu_text_bounds_follow_nonzero_content_origin_and_clip() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096)
        .expect("layer renderer");
    renderer
        .enable_text(catalog(), font_config(), config(4 * 1024 * 1024))
        .expect("enable GPU text");
    let geometry = RenderGeometry::new(64, 32, 16, 24).with_content_rect(16, 4, 32, 24);

    let report = renderer
        .prepare_text(
            &snapshot("AB", 2),
            geometry,
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("prepare placed GPU text");

    assert!(!report.glyph_bounds.is_empty());
    assert!(report.glyph_bounds.iter().all(|bounds| {
        bounds.x >= 16
            && bounds.y >= 4
            && bounds.x.saturating_add(bounds.width) <= 48
            && bounds.y.saturating_add(bounds.height) <= 28
    }));
}

#[test]
fn local_row_damage_preserves_cached_glyphs_on_untouched_rows() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096)
        .expect("layer renderer");
    renderer
        .enable_text(catalog(), font_config(), config(4 * 1024 * 1024))
        .expect("enable GPU text");
    let snapshot = multiline_snapshot("A\r\nB", 4, 2);
    let geometry = RenderGeometry::new(64, 48, 16, 24);
    let first = renderer
        .prepare_text(
            &snapshot,
            geometry,
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("prepare complete frame");
    let report = renderer
        .prepare_text(
            &snapshot,
            geometry,
            &[DamageRegion::new(0, 0, 1, 1)],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("prepare only first damaged row");
    assert_eq!(report.prepared_rows, vec![0]);
    assert_eq!(report.retained_glyphs, first.retained_glyphs);
    assert_eq!(report.retained_mask_glyphs, first.retained_mask_glyphs);
    assert_eq!(report.retained_color_glyphs, first.retained_color_glyphs);
    assert_eq!(
        report.retained_custom_block_glyphs,
        first.retained_custom_block_glyphs
    );

    let mut graph = RenderGraph::new(64, 48);
    graph.push_quad(GpuQuad::new(
        GpuLayer::PaneBackground,
        PixelRect::new(0, 0, 64, 48),
        [0, 0, 0, 255],
    ));
    let pixels = renderer
        .render_headless_rgba8(&graph, Duration::from_secs(5))
        .expect("render cached rows");
    assert!(
        pixels[64 * 24 * 4..]
            .chunks_exact(4)
            .any(|pixel| pixel[0] > 0 || pixel[1] > 0 || pixel[2] > 0),
        "row 1 must remain in glyphon's complete prepared batch after row 0 damage"
    );
}

#[test]
fn unchanged_full_frame_reuses_prepared_rows() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096)
        .expect("layer renderer");
    renderer
        .enable_text(catalog(), font_config(), config(4 * 1024 * 1024))
        .expect("enable GPU text");
    let geometry = RenderGeometry::new(64, 48, 16, 24);
    let paint = TextPaintConfig::default();
    let snapshot = multiline_snapshot("A\r\nB", 4, 2);

    let first = renderer
        .prepare_text(&snapshot, geometry, &[], &paint, 1.0, 1.0)
        .expect("prepare initial full frame");
    assert_eq!(first.prepared_rows, vec![0, 1]);

    let second = renderer
        .prepare_text(&snapshot, geometry, &[], &paint, 1.0, 1.0)
        .expect("reuse unchanged full frame");
    assert!(
        second.prepared_rows.is_empty(),
        "unchanged full frames must retain shaped rows instead of reshaping every row"
    );
    assert_eq!(second.shaped_glyphs, 0);
    assert_eq!(second.mask_glyphs, 0);
    assert_eq!(second.color_glyphs, 0);
    assert_eq!(second.custom_block_glyphs, 0);
    assert_eq!(second.retained_glyphs, first.retained_glyphs);
    assert_eq!(second.retained_mask_glyphs, first.retained_mask_glyphs);
    assert_eq!(second.retained_color_glyphs, first.retained_color_glyphs);
    assert_eq!(
        second.retained_custom_block_glyphs,
        first.retained_custom_block_glyphs
    );
}

#[test]
fn full_frame_style_change_rebuilds_cached_rows() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096)
        .expect("layer renderer");
    renderer
        .enable_text(catalog(), font_config(), config(4 * 1024 * 1024))
        .expect("enable GPU text");
    let geometry = RenderGeometry::new(64, 48, 16, 24);
    let paint = TextPaintConfig::default();
    let plain = multiline_snapshot("A\r\nB", 4, 2);
    let styled = multiline_snapshot("\x1b[31mA\r\nB", 4, 2);

    renderer
        .prepare_text(&plain, geometry, &[], &paint, 1.0, 1.0)
        .expect("prepare initial full frame");
    let changed = renderer
        .prepare_text(&styled, geometry, &[], &paint, 1.0, 1.0)
        .expect("prepare changed style full frame");
    assert_eq!(
        changed.prepared_rows,
        vec![0, 1],
        "style changes must rebuild rows even when text and coordinates are unchanged"
    );
}

#[test]
fn stable_missing_glyph_full_frame_reuses_rows_and_report_metadata() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096)
        .expect("layer renderer");
    renderer
        .enable_text(catalog(), font_config(), config(4 * 1024 * 1024))
        .expect("enable GPU text");
    let geometry = RenderGeometry::new(64, 24, 16, 24);
    let paint = TextPaintConfig::default();
    let snapshot = multiline_snapshot("\u{10ffff}", 4, 1);

    let first = renderer
        .prepare_text(&snapshot, geometry, &[], &paint, 1.0, 1.0)
        .expect("prepare stable tofu frame");
    assert_eq!(first.missing_glyphs, ['\u{10ffff}']);
    let second = renderer
        .prepare_text(&snapshot, geometry, &[], &paint, 1.0, 1.0)
        .expect("reuse stable tofu frame");
    assert!(second.prepared_rows.is_empty());
    assert_eq!(second.missing_glyphs, first.missing_glyphs);
}

#[test]
fn text_blink_opacity_only_rebuilds_rows_with_blinking_cells() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096)
        .expect("layer renderer");
    renderer
        .enable_text(catalog(), font_config(), config(4 * 1024 * 1024))
        .expect("enable GPU text");
    let geometry = RenderGeometry::new(64, 24, 16, 24);
    let plain = multiline_snapshot("A", 4, 1);
    let blinking = multiline_snapshot("\x1b[5mA", 4, 1);
    let visible = TextPaintConfig::default();
    let hidden = TextPaintConfig {
        text_blink_opacity_alpha: 0,
        rapid_text_blink_opacity_alpha: 0,
        ..visible.clone()
    };

    renderer
        .prepare_text(&plain, geometry, &[], &visible, 1.0, 1.0)
        .expect("prepare plain frame");
    let plain_alpha_change = renderer
        .prepare_text(&plain, geometry, &[], &hidden, 1.0, 1.0)
        .expect("reuse plain frame after blink alpha change");
    assert!(plain_alpha_change.prepared_rows.is_empty());

    renderer
        .prepare_text(&blinking, geometry, &[], &visible, 1.0, 1.0)
        .expect("prepare blinking frame");
    let blinking_alpha_change = renderer
        .prepare_text(&blinking, geometry, &[], &hidden, 1.0, 1.0)
        .expect("rebuild blinking frame after blink alpha change");
    assert_eq!(blinking_alpha_change.prepared_rows, vec![0]);
}

#[test]
fn damage_render_matches_full_frame_after_terminal_scroll() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let geometry = RenderGeometry::new(8 * 16, 3 * 24, 16, 24);
    let paint = TextPaintConfig::default();
    let first = {
        let mut terminal = Terminal::new(TerminalSize::new(8, 3));
        terminal.feed(b"A\r\nB\r\nC");
        terminal.take_damage();
        let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
        terminal.feed(b"\r\nD");
        let damage = terminal.take_damage();
        (
            snapshot,
            damage,
            TerminalRenderSnapshot::from_terminal(&terminal),
        )
    };

    let mut damaged_renderer =
        GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 64 * 1024)
            .expect("layer renderer");
    damaged_renderer
        .enable_text(catalog(), font_config(), config(4 * 1024 * 1024))
        .expect("enable GPU text");
    damaged_renderer
        .prepare_text(&first.0, geometry, &[], &paint, 1.0, 1.0)
        .expect("prepare initial frame");
    let _ = render_prepared(&mut damaged_renderer, geometry, paint.default_background);
    damaged_renderer
        .prepare_text(&first.2, geometry, &first.1, &paint, 1.0, 1.0)
        .expect("prepare damaged frame");
    let damaged = render_prepared(&mut damaged_renderer, geometry, paint.default_background);

    let mut full_renderer =
        GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 64 * 1024)
            .expect("full layer renderer");
    full_renderer
        .enable_text(catalog(), font_config(), config(4 * 1024 * 1024))
        .expect("enable full GPU text");
    full_renderer
        .prepare_text(&first.2, geometry, &[], &paint, 1.0, 1.0)
        .expect("prepare full frame");
    let full = render_prepared(&mut full_renderer, geometry, paint.default_background);

    assert_eq!(damaged, full, "damage render diverged from full redraw");
}

#[test]
fn ordinary_glyph_overhang_survives_inside_a_run_but_not_across_a_hard_boundary() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096)
        .expect("layer renderer");
    renderer
        .enable_text(catalog(), font_config(), config(4 * 1024 * 1024))
        .expect("enable GPU text");
    let geometry = RenderGeometry::new(32, 24, 16, 24);

    let overhang = renderer
        .prepare_text(
            &snapshot("Aj", 2),
            geometry,
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("prepare unbroken run");
    assert!(
        overhang
            .glyph_bounds
            .iter()
            .any(|bounds| bounds.x < 16 && bounds.x + bounds.width > 16),
        "the second-cell j must retain its legal left overhang within one shaped run"
    );

    let hard_boundary = snapshot("\x1b[31mA\x1b[39mj", 2);
    let clipped = renderer
        .prepare_text(
            &hard_boundary,
            geometry,
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("prepare style-separated runs");
    assert!(
        clipped
            .glyph_bounds
            .iter()
            .all(|bounds| bounds.x >= 16 || bounds.x + bounds.width <= 16),
        "a style/hard boundary must prevent glyph coverage from leaking into the adjacent run: {:?}",
        clipped.glyph_bounds
    );
}

#[test]
fn block_cursor_foreground_redraw_is_clipped_to_one_visual_cell() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096)
        .expect("layer renderer");
    renderer
        .enable_text(
            catalog(),
            font_config(),
            config(4 * 1024 * 1024).with_cursor_foreground([255, 0, 0, 255]),
        )
        .expect("enable GPU text");
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed("中\x1b[2D\x1b[2 q".as_bytes());
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert_eq!(
        snapshot.cursor().expect("visible cursor").shape,
        CursorShape::Block
    );
    let geometry = RenderGeometry::new(32, 24, 16, 24);
    let report = renderer
        .prepare_text(
            &snapshot,
            geometry,
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("prepare cursor foreground");
    assert!(report.cursor_foreground_glyphs > 0);
    assert!(
        report
            .cursor_foreground_bounds
            .iter()
            .all(|bounds| bounds.x + bounds.width <= 16),
        "wide glyph cursor redraw must use the one-cell visual cursor clip: {:?}",
        report.cursor_foreground_bounds
    );

    let mut graph = RenderGraph::new(32, 24);
    graph.push_quad(GpuQuad::new(
        GpuLayer::PaneBackground,
        PixelRect::new(0, 0, 32, 24),
        [0, 0, 0, 255],
    ));
    graph.push_quad(GpuQuad::new(
        GpuLayer::Cursor,
        PixelRect::new(0, 0, 16, 24),
        [0, 0, 255, 255],
    ));
    let pixels = renderer
        .render_headless_rgba8(&graph, Duration::from_secs(5))
        .expect("render cursor redraw after cursor quad");
    let is_red = |pixel: &[u8]| pixel[0] > 200 && pixel[1] < 50 && pixel[2] < 50;
    assert!((0..24).any(|y| {
        pixels[(y * 32) * 4..(y * 32 + 16) * 4]
            .chunks_exact(4)
            .any(is_red)
    }));
    assert!(
        !(0..24).any(|y| pixels[(y * 32 + 16) * 4..(y * 32 + 32) * 4]
            .chunks_exact(4)
            .any(is_red)),
        "cursor foreground redraw must not leak into the second half of a wide glyph"
    );
}

#[test]
fn color_glyph_cursor_redraw_uses_canonical_pixels_when_cell_is_faint() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096)
        .expect("layer renderer");
    renderer
        .enable_text(
            catalog(),
            font_config(),
            config(4 * 1024 * 1024).with_cursor_foreground([255, 255, 255, 255]),
        )
        .expect("enable GPU text");
    let geometry = RenderGeometry::new(32, 24, 16, 24);
    let paint = TextPaintConfig::default();
    let mut render_cursor = |text: &str| {
        let mut terminal = Terminal::new(TerminalSize::new(2, 1));
        terminal.feed(text.as_bytes());
        terminal.feed(b"\x1b[2D\x1b[2 q");
        let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
        renderer
            .prepare_text(&snapshot, geometry, &[], &paint, 1.0, 1.0)
            .expect("prepare color glyph under block cursor");
        let mut graph = RenderGraph::new(32, 24);
        graph.push_quad(GpuQuad::new(
            GpuLayer::PaneBackground,
            PixelRect::new(0, 0, 32, 24),
            [0, 0, 0, 255],
        ));
        graph.push_quad(GpuQuad::new(
            GpuLayer::Cursor,
            PixelRect::new(0, 0, 16, 24),
            [0, 0, 255, 255],
        ));
        renderer
            .render_headless_rgba8(&graph, Duration::from_secs(5))
            .expect("render color glyph cursor redraw")
    };

    let normal = render_cursor("😀");
    let faint = render_cursor("\x1b[2m😀");
    let first_cell = |pixels: &[u8]| {
        (0..24)
            .flat_map(|row| pixels[(row * 32) * 4..(row * 32 + 16) * 4].iter().copied())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        first_cell(&faint),
        first_cell(&normal),
        "the cursor redraw must use canonical color glyph pixels, independent of faint paint"
    );
}

#[test]
fn real_gpu_renders_glyphs_at_the_reserved_layer_slot() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 64 * 1024)
        .expect("layer renderer");
    renderer
        .enable_text(catalog(), font_config(), config(4 * 1024 * 1024))
        .expect("enable GPU text");
    let geometry = RenderGeometry::new(64, 24, 16, 24);
    renderer
        .prepare_text(
            &snapshot("A", 4),
            geometry,
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("prepare glyph");

    let mut graph = RenderGraph::new(64, 24);
    graph.push_quad(GpuQuad::new(
        GpuLayer::PaneBackground,
        PixelRect::new(0, 0, 64, 24),
        [0, 0, 0, 255],
    ));
    let pixels = renderer
        .render_headless_rgba8(&graph, Duration::from_secs(5))
        .expect("render real GPU text");

    assert!(
        pixels
            .chunks_exact(4)
            .any(|pixel| pixel[0] > 0 || pixel[1] > 0 || pixel[2] > 0)
    );
}

#[test]
fn real_atlas_glyph_is_below_a_positive_image_without_a_glyph_quad() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 64 * 1024)
        .expect("layer renderer");
    renderer
        .enable_text(catalog(), font_config(), config(4 * 1024 * 1024))
        .expect("enable GPU text");
    let geometry = RenderGeometry::new(16, 24, 16, 24);
    renderer
        .prepare_text(
            &snapshot("A", 1),
            geometry,
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("prepare real atlas glyph");
    let mut graph = RenderGraph::new(16, 24);
    graph.push_quad(GpuQuad::new(
        GpuLayer::PaneBackground,
        PixelRect::new(0, 0, 16, 24),
        [0, 0, 0, 255],
    ));
    graph.push_image(GpuImage::new(
        ImageProtocol::Kitty,
        0,
        PixelRect::new(0, 0, 16, 24),
        [0, 0, 255, 255],
    ));
    let pixels = renderer
        .render_headless_rgba8(&graph, Duration::from_secs(5))
        .expect("render atlas glyph below positive image");
    assert!(
        pixels
            .chunks_exact(4)
            .all(|pixel| pixel == [0, 0, 255, 255]),
        "an opaque PositiveImage must cover the actual glyphon atlas draw"
    );
}
