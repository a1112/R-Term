use std::{
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use rterm_render_cpu::{
    DamageRegion, PixelRenderer, RenderBackgroundGradient, RenderBackgroundGradientBlend,
    RenderBackgroundGradientHsb, RenderBackgroundGradientInterpolation,
    RenderBackgroundGradientOrientation, RenderBackgroundImage, RenderBackgroundImageAttachment,
    RenderBackgroundImageDimension, RenderBackgroundImageHorizontalAlign,
    RenderBackgroundImageLength, RenderBackgroundImageRepeat, RenderBackgroundImageVerticalAlign,
    RenderBackgroundLayer, RenderGeometry, ScrollbackScrollbar, TerminalRenderSnapshot,
};
use rterm_render_wgpu::GpuFramePlanner;
use rterm_render_wgpu::gpu::{
    DEFAULT_GPU_INSTANCE_BYTE_BUDGET, GpuContext, GpuContextOptions, GpuImage, GpuLayer,
    GpuLayerRenderer, GpuQuad, ImageProtocol, PixelRect, RenderGraph, SignedPixelRect,
};
use rterm_terminal::Terminal;
use rterm_types::TerminalSize;

static GPU_TEST_LOCK: Mutex<()> = Mutex::new(());

fn gpu_test_guard() -> MutexGuard<'static, ()> {
    GPU_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> [u8; 4] {
    [red, green, blue, alpha]
}

fn assert_rgba_close(actual: &[u8], expected: &[u8], tolerance: u8) {
    assert_eq!(actual.len(), expected.len());
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            actual.abs_diff(expected) <= tolerance,
            "channel {index}: GPU={actual}, CPU={expected}"
        );
    }
}

mod lazy_image_pipeline {
    use super::*;

    fn image_snapshot_graph() -> RenderGraph {
        const RED_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
        let mut terminal = Terminal::new(TerminalSize::new(1, 1));
        terminal
            .feed(format!("\x1b]1337;File=inline=1;width=2px;height=2px:{RED_PNG}\x07").as_bytes());
        let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
        let mut graph = RenderGraph::new(2, 2);
        graph.push_snapshot_images(&snapshot, RenderGeometry::new(2, 2, 2, 2), 0, None);
        assert_eq!(graph.planned_image_draw_count(), 1);
        graph
    }

    #[test]
    fn image_pipeline_materializes_once_on_first_image() {
        let _gpu = gpu_test_guard();
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless adapter");
        let mut renderer = GpuLayerRenderer::new_headless(&context, 4096).expect("layer renderer");

        let initial = renderer.initialization_resources();
        assert_eq!(initial.pipeline_count, 1);
        assert_eq!(initial.pipeline_layout_count, 1);
        assert_eq!(initial.image_texture_bytes, 0);

        renderer
            .render_headless_rgba8(&RenderGraph::new(2, 2), Duration::from_secs(5))
            .expect("empty frame");
        assert_eq!(renderer.initialization_resources(), initial);

        let image = image_snapshot_graph();
        renderer
            .render_headless_rgba8(&image, Duration::from_secs(5))
            .expect("first image frame");
        let materialized = renderer.initialization_resources();
        assert_eq!(materialized.pipeline_count, 2);
        assert_eq!(materialized.pipeline_layout_count, 2);
        assert!(materialized.image_texture_bytes > 0);
        let texture_metrics = renderer.texture_cache_metrics();

        renderer
            .render_headless_rgba8(&image, Duration::from_secs(5))
            .expect("repeated image frame");
        assert_eq!(renderer.initialization_resources(), materialized);
        let repeated_texture_metrics = renderer.texture_cache_metrics();
        assert_eq!(repeated_texture_metrics.entries, texture_metrics.entries);
        assert_eq!(
            repeated_texture_metrics.retained_bytes,
            texture_metrics.retained_bytes
        );
        assert_eq!(repeated_texture_metrics.uploads, texture_metrics.uploads);
        assert_eq!(
            repeated_texture_metrics.evictions,
            texture_metrics.evictions
        );

        let mut constrained =
            GpuLayerRenderer::new_with_budgets(&context, wgpu::TextureFormat::Rgba8Unorm, 4096, 8)
                .expect("constrained layer renderer");
        let before_failure = constrained.initialization_resources();
        let error: rterm_render_wgpu::gpu::GpuLayerError = constrained
            .upload(&image)
            .expect_err("the 2x2 image must exceed the retained-byte budget");
        assert!(error.to_string().contains("budget"));
        assert_eq!(constrained.initialization_resources(), before_failure);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn upload_reports_image_pipeline_creation_failure_without_committing_resources() {
        let _gpu = gpu_test_guard();
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless adapter");
        let mut renderer = GpuLayerRenderer::new_headless(&context, 4096).expect("layer renderer");
        let image = image_snapshot_graph();
        let before = renderer.initialization_resources();

        renderer.inject_image_pipeline_creation_failure_for_test();
        let error: rterm_render_wgpu::gpu::GpuLayerError = renderer
            .upload(&image)
            .expect_err("injected image pipeline creation must fail");
        assert!(error.to_string().contains("image pipeline"));
        assert_eq!(renderer.initialization_resources(), before);
        assert_eq!(renderer.texture_cache_metrics().entries, 0);

        renderer.upload(&image).expect("one-shot failure can retry");
        assert_eq!(renderer.initialization_resources().pipeline_count, 2);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn headless_reports_image_pipeline_creation_failure_without_committing_resources() {
        let _gpu = gpu_test_guard();
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless adapter");
        let mut renderer = GpuLayerRenderer::new_headless(&context, 4096).expect("layer renderer");
        let image = image_snapshot_graph();
        let before = renderer.initialization_resources();

        renderer.inject_image_pipeline_creation_failure_for_test();
        let error: rterm_render_wgpu::gpu::GpuLayerError = renderer
            .render_headless_rgba8(&image, Duration::from_secs(5))
            .expect_err("injected image pipeline creation must fail");
        assert!(error.to_string().contains("image pipeline"));
        assert_eq!(renderer.initialization_resources(), before);
        assert_eq!(renderer.texture_cache_metrics().entries, 0);

        renderer
            .render_headless_rgba8(&image, Duration::from_secs(5))
            .expect("one-shot failure can retry");
        assert_eq!(renderer.initialization_resources().pipeline_count, 2);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn device_loss_during_first_image_creation_keeps_the_recovery_error_kind() {
        let _gpu = gpu_test_guard();
        let mut context =
            pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
                .expect("headless adapter");
        let mut renderer = GpuLayerRenderer::new_headless(&context, 4096).expect("layer renderer");
        let image = image_snapshot_graph();

        context.inject_device_loss_during_direct_upload_for_test();
        renderer.inject_image_pipeline_creation_failure_for_test();
        let error = context
            .render_graph(&mut renderer, &image, || {})
            .expect_err("device loss must outrank the image pipeline validation error");

        assert_eq!(
            error.kind(),
            rterm_render_wgpu::gpu::GpuContextErrorKind::DeviceLost
        );
        assert_eq!(context.metrics().device_losses, 1);
        assert_eq!(renderer.initialization_resources().pipeline_count, 1);
        assert_eq!(renderer.texture_cache_metrics().entries, 0);
    }
}

fn red_green_blue_vertical_png_bytes() -> &'static [u8] {
    &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x08, 0x06, 0x00, 0x00, 0x00, 0x52,
        0xdd, 0x65, 0x82, 0x00, 0x00, 0x00, 0x14, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8,
        0xcf, 0xc0, 0x00, 0x46, 0xff, 0x19, 0x18, 0x18, 0xfe, 0xff, 0x07, 0x00, 0x29, 0xe5, 0x05,
        0xfb, 0x48, 0xb8, 0xae, 0x8a, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42,
        0x60, 0x82,
    ]
}

fn red_green_gif_bytes() -> &'static [u8] {
    &[
        0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x01, 0x00, 0x01, 0x00, 0x81, 0x00, 0x00, 0xff, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x21, 0xff, 0x0b, 0x4e, 0x45,
        0x54, 0x53, 0x43, 0x41, 0x50, 0x45, 0x32, 0x2e, 0x30, 0x03, 0x01, 0x00, 0x00, 0x00, 0x21,
        0xf9, 0x04, 0x08, 0x0a, 0x00, 0x00, 0x00, 0x2c, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01,
        0x00, 0x00, 0x08, 0x04, 0x00, 0x01, 0x04, 0x04, 0x00, 0x21, 0xf9, 0x04, 0x08, 0x0a, 0x00,
        0x00, 0x00, 0x2c, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x81, 0x00, 0xff, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x04, 0x00, 0x01, 0x04, 0x04,
        0x00, 0x3b,
    ]
}

#[test]
fn render_graph_orders_every_terminal_layer_independently_of_insertion_order() {
    let mut graph = RenderGraph::new(8, 8);
    for layer in [
        GpuLayer::Selection,
        GpuLayer::Overlay,
        GpuLayer::TabBar,
        GpuLayer::Cursor,
        GpuLayer::Strikethrough,
        GpuLayer::Underline,
        GpuLayer::PositiveImage,
        GpuLayer::Glyph,
        GpuLayer::NegativeImage,
        GpuLayer::UltraNegativeImage,
        GpuLayer::CellBackground,
        GpuLayer::PaneBackground,
    ] {
        graph.push_quad(GpuQuad::new(
            layer,
            PixelRect::new(0, 0, 1, 1),
            rgba(1, 2, 3, 255),
        ));
    }

    assert_eq!(
        graph.ordered_layers(),
        &[
            GpuLayer::PaneBackground,
            GpuLayer::UltraNegativeImage,
            GpuLayer::CellBackground,
            GpuLayer::NegativeImage,
            GpuLayer::Glyph,
            GpuLayer::PositiveImage,
            GpuLayer::Underline,
            GpuLayer::Strikethrough,
            GpuLayer::Cursor,
            GpuLayer::TabBar,
            GpuLayer::Overlay,
            GpuLayer::Selection,
        ]
    );
    assert_eq!(graph.ordered_content_layers(), graph.ordered_layers());
}

#[test]
fn kitty_iterm_and_sixel_images_share_clipping_and_stable_z_ordering() {
    let mut graph = RenderGraph::new(8, 8);
    let clip = PixelRect::new(2, 1, 4, 5);
    for (protocol, z_index, color) in [
        (ImageProtocol::Kitty, 3, rgba(1, 0, 0, 255)),
        (ImageProtocol::Iterm, -2, rgba(2, 0, 0, 255)),
        (ImageProtocol::Sixel, -2, rgba(3, 0, 0, 255)),
    ] {
        graph.push_image(
            GpuImage::new(protocol, z_index, PixelRect::new(0, 0, 8, 8), color).with_clip(clip),
        );
    }

    let images = graph.ordered_images();
    assert_eq!(
        images
            .iter()
            .map(|image| (image.protocol(), image.z_index(), image.rect()))
            .collect::<Vec<_>>(),
        vec![
            (ImageProtocol::Iterm, -2, clip),
            (ImageProtocol::Sixel, -2, clip),
            (ImageProtocol::Kitty, 3, clip),
        ]
    );
    assert_eq!(images[0].layer(), GpuLayer::NegativeImage);
    assert_eq!(images[2].layer(), GpuLayer::PositiveImage);
}

#[test]
fn extreme_negative_kitty_image_precedes_cell_background() {
    let mut graph = RenderGraph::new(2, 2);
    graph.push_quad(GpuQuad::new(
        GpuLayer::CellBackground,
        PixelRect::new(0, 0, 2, 2),
        rgba(1, 2, 3, 255),
    ));
    graph.push_image(GpuImage::new(
        ImageProtocol::Kitty,
        i32::MIN / 2 - 1,
        PixelRect::new(0, 0, 2, 2),
        rgba(4, 5, 6, 255),
    ));

    assert_eq!(
        graph.ordered_content_layers(),
        vec![GpuLayer::UltraNegativeImage, GpuLayer::CellBackground]
    );
}

#[test]
fn same_z_kitty_ids_use_cpu_tie_break_and_missing_ids_retain_insertion_order() {
    let mut graph = RenderGraph::new(2, 2);
    graph.push_image(
        GpuImage::new(
            ImageProtocol::Kitty,
            -1,
            PixelRect::new(0, 0, 1, 1),
            rgba(20, 0, 0, 255),
        )
        .with_kitty_id(20),
    );
    graph.push_image(
        GpuImage::new(
            ImageProtocol::Kitty,
            -1,
            PixelRect::new(0, 0, 1, 1),
            rgba(10, 0, 0, 255),
        )
        .with_kitty_id(10),
    );
    graph.push_image(GpuImage::new(
        ImageProtocol::Iterm,
        -1,
        PixelRect::new(0, 0, 1, 1),
        rgba(30, 0, 0, 255),
    ));

    assert_eq!(
        graph
            .ordered_images()
            .iter()
            .map(|image| image.color()[0])
            .collect::<Vec<_>>(),
        vec![10, 20, 30]
    );
}

#[test]
fn ordered_images_is_total_for_some_missing_some_id_permutations() {
    let permutations = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    for permutation in permutations {
        let mut graph = RenderGraph::new(1, 1);
        for index in permutation {
            let (id, red) = match index {
                0 => (Some(100), 100),
                1 => (None, 200),
                _ => (Some(1), 1),
            };
            let image = GpuImage::new(
                ImageProtocol::Kitty,
                0,
                PixelRect::new(0, 0, 1, 1),
                rgba(red, 0, 0, 255),
            );
            graph.push_image(id.map_or(image, |id| image.with_kitty_id(id)));
        }
        assert_eq!(
            graph
                .ordered_images()
                .iter()
                .map(|image| image.color()[0])
                .collect::<Vec<_>>(),
            vec![1, 100, 200]
        );
    }
}

#[test]
fn signed_half_open_clipping_discards_negative_pixels_before_encoding() {
    let image = GpuImage::new_signed(
        ImageProtocol::Sixel,
        0,
        SignedPixelRect::new(-2, -1, 5, 4),
        rgba(1, 2, 3, 255),
    )
    .with_signed_clip(SignedPixelRect::new(0, 0, 4, 3));

    assert_eq!(image.rect(), PixelRect::new(0, 0, 3, 3));
}

#[test]
fn persistent_instance_uploads_only_the_changed_aligned_range_and_honors_budget() {
    let _gpu = gpu_test_guard();
    let mut context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 256)
        .expect("bounded renderer");
    let mut graph = RenderGraph::new(4, 4);
    graph.push_quad(GpuQuad::new(
        GpuLayer::PaneBackground,
        PixelRect::new(0, 0, 4, 4),
        rgba(10, 20, 30, 255),
    ));
    graph.push_quad(GpuQuad::new(
        GpuLayer::Cursor,
        PixelRect::new(1, 1, 1, 1),
        rgba(40, 50, 60, 255),
    ));

    renderer.upload(&graph).expect("first upload");
    let first = renderer.upload_metrics();
    assert!(first.bytes_written > 0);
    assert!(first.capacity_bytes <= 256);

    renderer.upload(&graph).expect("unchanged upload");
    assert_eq!(renderer.upload_metrics().bytes_written, 0);

    graph.replace_quad(
        1,
        GpuQuad::new(
            GpuLayer::Cursor,
            PixelRect::new(1, 1, 1, 1),
            rgba(70, 80, 90, 255),
        ),
    );
    renderer.upload(&graph).expect("dirty upload");
    let dirty = renderer.upload_metrics();
    assert!(dirty.bytes_written > 0);
    assert!(dirty.bytes_written < first.bytes_written);
    assert_eq!(dirty.dirty_offset % wgpu::COPY_BUFFER_ALIGNMENT, 0);
    assert_eq!(dirty.bytes_written as u64 % wgpu::COPY_BUFFER_ALIGNMENT, 0);

    let mut oversized = RenderGraph::new(4, 4);
    for _ in 0..32 {
        oversized.push_quad(GpuQuad::new(
            GpuLayer::CellBackground,
            PixelRect::new(0, 0, 1, 1),
            rgba(0, 0, 0, 255),
        ));
    }
    let error = renderer
        .upload(&oversized)
        .expect_err("instance budget must be enforced");
    assert!(error.to_string().contains("budget"));

    context
        .run_headless_submission_probe(Duration::from_secs(5))
        .expect("writes leave device usable");
}

#[test]
fn headless_gpu_readback_matches_cpu_layering_invariants_with_tolerance() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer =
        GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096).expect("renderer");
    let mut graph = RenderGraph::new(4, 4);
    graph.push_quad(GpuQuad::new(
        GpuLayer::PaneBackground,
        PixelRect::new(0, 0, 4, 4),
        rgba(180, 0, 0, 255),
    ));
    graph.push_image(GpuImage::new(
        ImageProtocol::Kitty,
        -1,
        PixelRect::new(0, 0, 3, 3),
        rgba(0, 160, 0, 255),
    ));
    graph.push_quad(GpuQuad::new(
        GpuLayer::Glyph,
        PixelRect::new(1, 1, 1, 1),
        rgba(0, 0, 0, 0),
    ));
    graph.push_image(GpuImage::new(
        ImageProtocol::Sixel,
        1,
        PixelRect::new(1, 1, 3, 3),
        rgba(0, 0, 140, 255),
    ));
    graph.push_quad(GpuQuad::new(
        GpuLayer::Selection,
        PixelRect::new(2, 2, 1, 1),
        rgba(230, 230, 230, 255),
    ));

    let actual = renderer
        .render_headless_rgba8(&graph, Duration::from_secs(5))
        .expect("real wgpu render and readback");
    let expected = [
        rgba(0, 160, 0, 255),
        rgba(0, 160, 0, 255),
        rgba(0, 160, 0, 255),
        rgba(180, 0, 0, 255),
        rgba(0, 160, 0, 255),
        rgba(0, 0, 140, 255),
        rgba(0, 0, 140, 255),
        rgba(0, 0, 140, 255),
        rgba(0, 160, 0, 255),
        rgba(0, 0, 140, 255),
        rgba(230, 230, 230, 255),
        rgba(0, 0, 140, 255),
        rgba(180, 0, 0, 255),
        rgba(0, 0, 140, 255),
        rgba(0, 0, 140, 255),
        rgba(0, 0, 140, 255),
    ]
    .concat();
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            actual.abs_diff(expected) <= 2,
            "channel {index}: GPU={actual}, CPU reference={expected}"
        );
    }
}

#[test]
fn renderer_owned_gpu_planner_composites_configured_background_layers() {
    let _gpu = gpu_test_guard();
    let terminal = Terminal::new(TerminalSize::new(1, 1));
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let geometry = RenderGeometry::new(4, 4, 1, 1);
    let mut planner = GpuFramePlanner::default();
    planner.set_default_background(rgba(10, 20, 30, 255));
    planner
        .set_default_background_layers(vec![RenderBackgroundLayer::Color(rgba(90, 80, 70, 255))]);

    let graph = planner.prepare_gpu_frame(&snapshot, geometry, None, 0);
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer =
        GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096).expect("renderer");
    let actual = renderer
        .render_headless_rgba8(&graph, Duration::from_secs(5))
        .expect("renderer-owned GPU plan readback");

    assert_eq!(
        &actual[60..64],
        &rgba(90, 80, 70, 255),
        "configured background layers must be part of the direct GPU plan"
    );
}

#[test]
fn renderer_owned_gpu_planner_rounds_frame_border_corners() {
    let _gpu = gpu_test_guard();
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[?25l");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let geometry = RenderGeometry::new(8, 8, 1, 1).with_frame_border(rgba(200, 210, 220, 255));
    let mut planner = GpuFramePlanner::default();
    let background = rgba(10, 20, 30, 255);
    planner.set_default_background(background);
    let graph = planner.prepare_gpu_frame(&snapshot, geometry, None, 0);
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer =
        GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096).expect("renderer");
    let actual = renderer
        .render_headless_rgba8(&graph, Duration::from_secs(5))
        .expect("rounded frame border readback");
    let pixel = |x: usize, y: usize| {
        let start = (y * 8 + x) * 4;
        &actual[start..start + 4]
    };

    assert_eq!(pixel(0, 0), background);
    assert_eq!(pixel(1, 0), rgba(200, 210, 220, 255));
    assert_eq!(pixel(0, 1), rgba(200, 210, 220, 255));
    assert_eq!(pixel(2, 2), background);
}

#[test]
fn renderer_owned_gpu_planner_preserves_gradient_stops_and_reuses_the_texture() {
    let _gpu = gpu_test_guard();
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[?25l");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let geometry = RenderGeometry::new(5, 3, 1, 1);
    let mut planner = GpuFramePlanner::default();
    planner.set_default_background(rgba(5, 7, 9, 255));
    planner.set_default_background_gradient(Some(RenderBackgroundGradient {
        orientation: RenderBackgroundGradientOrientation::Horizontal,
        interpolation: RenderBackgroundGradientInterpolation::Linear,
        blend: RenderBackgroundGradientBlend::Rgb,
        noise: Some(0),
        segment: None,
        preset: None,
        opacity_alpha: 192,
        blend_with_default_background: true,
        hsb: RenderBackgroundGradientHsb::IDENTITY,
        colors: vec![rgba(255, 0, 0, 255), rgba(0, 0, 255, 255)],
    }));
    let graph = planner.prepare_gpu_frame(&snapshot, geometry, None, 0);
    let mut expected = vec![0; 5 * 3 * 4];
    planner.render(&snapshot, &mut expected, 5, 3, 1, 1);

    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer =
        GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096).expect("renderer");
    let actual = renderer
        .render_headless_rgba8(&graph, Duration::from_secs(5))
        .expect("gradient readback");
    assert_rgba_close(&actual, &expected, 2);
    assert_ne!(&actual[0..4], &actual[16..20], "gradient stops collapsed");
    assert_eq!(planner.gpu_background_plan_updates(), 1);
    let uploads = renderer.texture_cache_metrics().uploads;
    let materializations = renderer.texture_cache_metrics().materializations;

    planner.set_animation_elapsed_ms(1);
    let same = planner.prepare_gpu_frame(&snapshot, geometry, None, 0);
    renderer
        .render_headless_rgba8(&same, Duration::from_secs(5))
        .expect("cached gradient readback");
    assert_eq!(planner.gpu_background_plan_updates(), 1);
    assert_eq!(renderer.texture_cache_metrics().uploads, uploads);
    assert_eq!(
        renderer.texture_cache_metrics().materializations,
        materializations
    );

    let resized = planner.prepare_gpu_frame(&snapshot, RenderGeometry::new(6, 3, 1, 1), None, 0);
    renderer
        .render_headless_rgba8(&resized, Duration::from_secs(5))
        .expect("resized gradient readback");
    assert_eq!(planner.gpu_background_plan_updates(), 2);
    assert_eq!(renderer.texture_cache_metrics().uploads, uploads + 1);
}

#[test]
fn renderer_owned_gpu_background_cache_tracks_selected_animation_frame_not_elapsed_clock() {
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[?25l");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let geometry = RenderGeometry::new(2, 2, 2, 2);
    let mut planner = GpuFramePlanner::default();
    planner.set_default_background_image(Some(RenderBackgroundImage {
        data: red_green_gif_bytes().to_vec(),
        opacity_alpha: u8::MAX,
        hsb: RenderBackgroundGradientHsb::IDENTITY,
        animation_speed_millis: 1_000,
        attachment: RenderBackgroundImageAttachment::Fixed,
        width: RenderBackgroundImageDimension::Cover,
        height: RenderBackgroundImageDimension::Cover,
        repeat_x: RenderBackgroundImageRepeat::Repeat,
        repeat_y: RenderBackgroundImageRepeat::Repeat,
        horizontal_align: RenderBackgroundImageHorizontalAlign::Left,
        vertical_align: RenderBackgroundImageVerticalAlign::Top,
        horizontal_offset: RenderBackgroundImageLength::Pixels(0),
        vertical_offset: RenderBackgroundImageLength::Pixels(0),
        repeat_x_size: None,
        repeat_y_size: None,
    }));
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer =
        GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096).expect("renderer");

    planner.set_animation_elapsed_ms(1);
    let first = planner.prepare_gpu_frame(&snapshot, geometry, None, 0);
    renderer
        .render_headless_rgba8(&first, Duration::from_secs(5))
        .expect("first animation bucket");
    let uploads = renderer.texture_cache_metrics().uploads;

    planner.set_animation_elapsed_ms(99);
    let same_bucket = planner.prepare_gpu_frame(&snapshot, geometry, None, 0);
    renderer
        .render_headless_rgba8(&same_bucket, Duration::from_secs(5))
        .expect("same animation bucket");
    assert_eq!(planner.gpu_background_plan_updates(), 1);
    assert_eq!(renderer.texture_cache_metrics().uploads, uploads);

    planner.set_animation_elapsed_ms(100);
    let next_bucket = planner.prepare_gpu_frame(&snapshot, geometry, None, 0);
    renderer
        .render_headless_rgba8(&next_bucket, Duration::from_secs(5))
        .expect("next animation bucket");
    assert_eq!(planner.gpu_background_plan_updates(), 2);
    assert_eq!(renderer.texture_cache_metrics().uploads, uploads + 1);
}

#[test]
fn renderer_owned_gpu_background_rejects_oversized_raster_before_allocation() {
    let snapshot = TerminalRenderSnapshot::from_terminal(&Terminal::new(TerminalSize::new(1, 1)));
    let mut planner = GpuFramePlanner::default();
    planner.set_default_background_gradient(Some(RenderBackgroundGradient {
        orientation: RenderBackgroundGradientOrientation::Horizontal,
        interpolation: RenderBackgroundGradientInterpolation::Linear,
        blend: RenderBackgroundGradientBlend::Rgb,
        noise: Some(0),
        segment: None,
        preset: None,
        opacity_alpha: u8::MAX,
        blend_with_default_background: false,
        hsb: RenderBackgroundGradientHsb::IDENTITY,
        colors: vec![rgba(255, 0, 0, 255), rgba(0, 0, 255, 255)],
    }));

    let graph =
        planner.prepare_gpu_frame(&snapshot, RenderGeometry::new(4097, 4097, 1, 1), None, 0);

    assert_eq!(planner.gpu_background_plan_updates(), 0);
    assert_eq!(planner.gpu_background_plan_budget_rejections(), 1);
    assert_eq!(graph.planned_image_draw_count(), 0);
}

#[test]
fn renderer_owned_gpu_planner_preserves_background_image_layout_opacity_and_clip() {
    let _gpu = gpu_test_guard();
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b[?25l");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let geometry = RenderGeometry::new(8, 6, 1, 1).with_content_rect(1, 1, 6, 4);
    let background = rgba(10, 20, 30, 255);
    let mut planner = GpuFramePlanner::default();
    planner.set_default_background(background);
    planner.set_default_background_image(Some(RenderBackgroundImage {
        data: red_green_blue_vertical_png_bytes().to_vec(),
        opacity_alpha: 128,
        hsb: RenderBackgroundGradientHsb::IDENTITY,
        animation_speed_millis: 1_000,
        attachment: RenderBackgroundImageAttachment::Fixed,
        width: RenderBackgroundImageDimension::Pixels(2),
        height: RenderBackgroundImageDimension::Pixels(3),
        repeat_x: RenderBackgroundImageRepeat::Repeat,
        repeat_y: RenderBackgroundImageRepeat::NoRepeat,
        horizontal_align: RenderBackgroundImageHorizontalAlign::Right,
        vertical_align: RenderBackgroundImageVerticalAlign::Bottom,
        horizontal_offset: RenderBackgroundImageLength::Pixels(-1),
        vertical_offset: RenderBackgroundImageLength::Pixels(0),
        repeat_x_size: Some(RenderBackgroundImageLength::Pixels(3)),
        repeat_y_size: None,
    }));
    let graph = planner.prepare_gpu_frame(&snapshot, geometry, None, 0);
    let mut content = vec![0; 6 * 4 * 4];
    planner.render(&snapshot, &mut content, 6, 4, 1, 1);
    let mut expected = vec![0; 8 * 6 * 4];
    for pixel in expected.chunks_exact_mut(4) {
        pixel.copy_from_slice(&background);
    }
    for y in 0..4_usize {
        let source = y * 6 * 4;
        let destination = ((y + 1) * 8 + 1) * 4;
        expected[destination..destination + 6 * 4]
            .copy_from_slice(&content[source..source + 6 * 4]);
    }

    let gpu_context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&gpu_context, wgpu::TextureFormat::Rgba8Unorm, 4096)
        .expect("renderer");
    let actual = renderer
        .render_headless_rgba8(&graph, Duration::from_secs(5))
        .expect("background image readback");
    assert_rgba_close(&actual, &expected, 2);
    assert_eq!(
        &actual[0..4],
        &background,
        "padding must retain window color"
    );
    assert_ne!(
        &actual[((4 * 8 + 5) * 4)..((4 * 8 + 5) * 4 + 4)],
        &background,
        "aligned repeated image was not sampled"
    );
}

#[test]
fn renderer_owned_gpu_planner_emits_effective_cell_backgrounds() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[48;2;3;4;5mX");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let graph = GpuFramePlanner::default().prepare_gpu_frame(
        &snapshot,
        RenderGeometry::new(4, 2, 2, 2),
        None,
        0,
    );

    assert!(
        graph
            .ordered_content_layers()
            .contains(&GpuLayer::CellBackground),
        "effective backgrounds baked into snapshot cells must reach the direct GPU graph"
    );
}

#[test]
fn renderer_owned_gpu_planner_reuses_snapshot_image_z_order() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 2));
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=91,f=24,s=2,v=2,c=2,r=2;/wAAAP8AAAD/////\x1b\\");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let graph = GpuFramePlanner::default().prepare_gpu_frame(
        &snapshot,
        RenderGeometry::new(4, 4, 2, 2),
        None,
        0,
    );

    assert_eq!(
        graph.planned_image_draw_count(),
        4,
        "the renderer-owned planner must preserve the authoritative fragmented image plan"
    );
    assert_eq!(
        graph.planned_image_destinations(),
        vec![
            PixelRect::new(0, 0, 2, 2),
            PixelRect::new(2, 0, 2, 2),
            PixelRect::new(0, 2, 2, 2),
            PixelRect::new(2, 2, 2, 2),
        ]
    );
}

#[test]
fn renderer_owned_gpu_planner_emits_decorations_and_cursor() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 1));
    terminal.feed(b"\x1b[4mX");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let graph = GpuFramePlanner::default().prepare_gpu_frame(
        &snapshot,
        RenderGeometry::new(8, 4, 4, 4),
        None,
        0,
    );
    let layers = graph.ordered_content_layers();

    assert!(
        layers.contains(&GpuLayer::Underline),
        "text decorations must be explicit GPU graph layers"
    );
    assert!(
        layers.contains(&GpuLayer::Cursor),
        "the terminal cursor must be an explicit GPU graph layer"
    );
}

#[test]
fn renderer_owned_gpu_planner_omits_cursor_when_snapshot_hides_it() {
    let terminal = Terminal::new(TerminalSize::new(2, 1));
    let mut snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    assert!(snapshot.cursor().is_some(), "fixture must contain a cursor");
    snapshot.cursor = None;

    let graph = GpuFramePlanner::default().prepare_gpu_frame(
        &snapshot,
        RenderGeometry::new(8, 4, 4, 4),
        None,
        0,
    );

    assert!(
        !graph.ordered_content_layers().contains(&GpuLayer::Cursor),
        "a snapshot-level cursor suppression must reach the real GPU display list"
    );
}

#[test]
fn renderer_owned_gpu_planner_clips_scrollbar_below_protected_ui_rows() {
    let _gpu = gpu_test_guard();
    let terminal = Terminal::new(TerminalSize::new(1, 1));
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let geometry = RenderGeometry::new(8, 8, 2, 2);
    let mut planner = GpuFramePlanner::default();
    let pane = rgba(12, 34, 56, 255);
    planner.set_default_background(pane);
    let graph =
        planner.prepare_gpu_frame(&snapshot, geometry, ScrollbackScrollbar::new(100, 4, 0), 2);
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer =
        GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096).expect("renderer");
    let actual = renderer
        .render_headless_rgba8(&graph, Duration::from_secs(5))
        .expect("scrollbar GPU plan readback");
    let pixel = |x: usize, y: usize| &actual[(y * 8 + x) * 4..(y * 8 + x + 1) * 4];

    assert_eq!(
        pixel(7, 0),
        pane,
        "the scrollbar must not paint over protected UI rows"
    );
    assert_ne!(
        pixel(7, 7),
        pane,
        "the scrollbar must remain visible below protected UI rows"
    );
}

#[test]
fn renderer_owned_gpu_planner_moves_and_clips_every_layer_to_content_placement() {
    let _gpu = gpu_test_guard();
    let mut terminal = Terminal::new(TerminalSize::new(2, 2));
    terminal.feed(b"\x1b[48;2;90;80;70mX");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let geometry = RenderGeometry::new(8, 8, 2, 2).with_content_rect(2, 2, 4, 4);
    let graph = GpuFramePlanner::default().prepare_gpu_frame(&snapshot, geometry, None, 0);
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer =
        GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096).expect("renderer");
    let actual = renderer
        .render_headless_rgba8(&graph, Duration::from_secs(5))
        .expect("placed graph readback");
    let pixel = |x: usize, y: usize| &actual[(y * 8 + x) * 4..(y * 8 + x + 1) * 4];

    assert_eq!(pixel(2, 2), rgba(90, 80, 70, 255));
    assert_ne!(
        pixel(0, 0),
        rgba(90, 80, 70, 255),
        "content layers must not remain at the surface origin"
    );
    assert_ne!(
        pixel(6, 2),
        rgba(90, 80, 70, 255),
        "content layers must be clipped at the placement right edge"
    );
}

#[test]
fn snapshot_fragment_plan_suppresses_parent_and_recomputes_cell_geometry() {
    let mut terminal = Terminal::new(TerminalSize::new(2, 2));
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=77,f=24,s=2,v=2,c=2,r=2;/wAAAP8AAAD/////\x1b\\");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

    let mut one_pixel_cells = RenderGraph::new(2, 2);
    one_pixel_cells.push_snapshot_images(&snapshot, RenderGeometry::new(2, 2, 1, 1), 0, None);
    assert_eq!(
        one_pixel_cells.planned_image_draw_count(),
        4,
        "the fragmented parent must not also emit a whole-image draw"
    );

    let mut two_pixel_cells = RenderGraph::new(4, 4);
    two_pixel_cells.push_snapshot_images(&snapshot, RenderGeometry::new(4, 4, 2, 2), 0, None);
    assert_eq!(two_pixel_cells.planned_image_draw_count(), 4);
    assert_eq!(
        two_pixel_cells.planned_image_destinations(),
        vec![
            PixelRect::new(0, 0, 2, 2),
            PixelRect::new(2, 0, 2, 2),
            PixelRect::new(0, 2, 2, 2),
            PixelRect::new(2, 2, 2, 2),
        ]
    );
}

#[test]
fn decoded_fragment_textures_have_no_seams_and_are_reused_by_bounded_cache() {
    let _gpu = gpu_test_guard();
    let mut terminal = Terminal::new(TerminalSize::new(2, 2));
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=78,f=24,s=2,v=2,c=2,r=2;/wAAAP8AAAD/////\x1b\\");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let mut graph = RenderGraph::new(2, 2);
    graph.push_snapshot_images(&snapshot, RenderGeometry::new(2, 2, 1, 1), 0, None);

    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer =
        GpuLayerRenderer::new_with_budgets(&context, wgpu::TextureFormat::Rgba8Unorm, 4096, 32)
            .expect("renderer");
    let actual = renderer
        .render_headless_rgba8(&graph, Duration::from_secs(5))
        .expect("decoded texture readback");
    assert_eq!(
        actual,
        [
            rgba(255, 0, 0, 255),
            rgba(0, 255, 0, 255),
            rgba(0, 0, 255, 255),
            rgba(255, 255, 255, 255),
        ]
        .concat()
    );
    let first = renderer.texture_cache_metrics();
    assert_eq!(first.retained_bytes, 32);
    assert_eq!(first.uploads, 4);

    renderer
        .render_headless_rgba8(&graph, Duration::from_secs(5))
        .expect("cached decoded texture readback");
    assert_eq!(renderer.texture_cache_metrics().uploads, first.uploads);
}

#[test]
fn texture_cache_evicts_lru_entries_without_exceeding_its_byte_budget() {
    fn one_pixel_graph(id: u32, payload: &str) -> RenderGraph {
        let mut terminal = Terminal::new(TerminalSize::new(1, 1));
        terminal.feed(
            format!("\x1b_Ga=T,C=1,q=1,i={id},f=24,s=1,v=1,c=1,r=1;{payload}\x1b\\").as_bytes(),
        );
        let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
        let mut graph = RenderGraph::new(1, 1);
        graph.push_snapshot_images(&snapshot, RenderGeometry::new(1, 1, 1, 1), 0, None);
        graph
    }

    let _gpu = gpu_test_guard();
    let red = one_pixel_graph(81, "/wAA");
    let green = one_pixel_graph(82, "AP8A");
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer =
        GpuLayerRenderer::new_with_budgets(&context, wgpu::TextureFormat::Rgba8Unorm, 64, 8)
            .expect("one-pixel cache");

    for graph in [&red, &green, &red] {
        renderer
            .render_headless_rgba8(graph, Duration::from_secs(5))
            .expect("bounded cache frame");
        assert!(renderer.texture_cache_metrics().retained_bytes <= 8);
    }
    let metrics = renderer.texture_cache_metrics();
    assert_eq!(metrics.entries, 1);
    assert_eq!(metrics.uploads, 3);
    assert_eq!(metrics.evictions, 2);
}

#[test]
fn persistent_capacity_grows_geometrically_reuses_allocation_and_never_draws_stale_tail() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer =
        GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 256).expect("renderer");
    let mut one = RenderGraph::new(2, 1);
    one.push_quad(GpuQuad::new(
        GpuLayer::PaneBackground,
        PixelRect::new(0, 0, 2, 1),
        rgba(0, 200, 0, 255),
    ));
    renderer.upload(&one).expect("one instance");
    let initial_capacity = renderer.upload_metrics().capacity_bytes;

    let mut three = RenderGraph::new(2, 1);
    for (layer, color) in [
        (GpuLayer::PaneBackground, rgba(200, 0, 0, 255)),
        (GpuLayer::Cursor, rgba(0, 0, 200, 255)),
        (GpuLayer::Selection, rgba(200, 200, 200, 255)),
    ] {
        three.push_quad(GpuQuad::new(layer, PixelRect::new(1, 0, 1, 1), color));
    }
    renderer.upload(&three).expect("grow");
    let grown_capacity = renderer.upload_metrics().capacity_bytes;
    assert!(grown_capacity > initial_capacity);
    assert!(grown_capacity.is_power_of_two());

    let actual = renderer
        .render_headless_rgba8(&one, Duration::from_secs(5))
        .expect("shrunken active range");
    assert_eq!(
        actual,
        [rgba(0, 200, 0, 255), rgba(0, 200, 0, 255)].concat(),
        "instances beyond the current active count must not survive a shrink"
    );
    assert_eq!(renderer.upload_metrics().capacity_bytes, grown_capacity);
    assert!(!renderer.upload_metrics().reallocated);
}

#[test]
fn default_instance_budget_accepts_maximized_three_pane_workspace_graph() {
    const MAX_WINDOW_WIDTH: u32 = 3456;
    const MAX_WINDOW_HEIGHT: u32 = 2010;
    const PANE_COUNT: u32 = 3;
    const GRID_COLUMNS: u32 = 32;
    const GRID_ROWS: u32 = 32;

    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new_headless(&context, DEFAULT_GPU_INSTANCE_BYTE_BUDGET)
        .expect("default bounded instance budget");
    let mut graph = RenderGraph::new(MAX_WINDOW_WIDTH, MAX_WINDOW_HEIGHT);
    let pane_width = MAX_WINDOW_WIDTH / PANE_COUNT;
    for pane in 0..PANE_COUNT {
        let pane_x = pane.saturating_mul(pane_width);
        graph.push_quad(GpuQuad::new(
            GpuLayer::PaneBackground,
            PixelRect::new(pane_x, 0, pane_width, MAX_WINDOW_HEIGHT),
            rgba(12, 16, 22, 255),
        ));
        for row in 0..GRID_ROWS {
            for column in 0..GRID_COLUMNS {
                graph.push_quad(GpuQuad::new(
                    GpuLayer::CellBackground,
                    PixelRect::new(pane_x + column, row, 1, 1),
                    rgba(24, 30, 38, 255),
                ));
            }
        }
    }
    let expected_nodes = usize::try_from(PANE_COUNT)
        .unwrap_or(usize::MAX)
        .saturating_mul(
            1 + usize::try_from(GRID_COLUMNS.saturating_mul(GRID_ROWS)).unwrap_or(usize::MAX),
        );
    let expected_bytes = expected_nodes.saturating_mul(std::mem::size_of::<[f32; 8]>());
    assert!(expected_bytes > 64 * 1024);

    renderer.upload(&graph).expect("maximized three-pane graph");
    assert_eq!(renderer.upload_metrics().bytes_written, expected_bytes);
    assert!(renderer.upload_metrics().capacity_bytes >= expected_bytes);
}

#[test]
fn instance_budget_is_clamped_to_the_device_max_buffer_size() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, usize::MAX)
        .expect("device-clamped budget");
    assert!(renderer.instance_budget_bytes() as u64 <= context.device().limits().max_buffer_size);
}

#[test]
fn inline_image_alpha_replaces_cpu_style_instead_of_blending() {
    fn render_pixel(payload: &str) -> [u8; 4] {
        let mut terminal = Terminal::new(TerminalSize::new(1, 1));
        terminal.feed(format!("\x1b_Ga=T,q=1,i=90,f=32,s=1,v=1;{payload}\x1b\\").as_bytes());
        let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
        let mut graph = RenderGraph::new(1, 1);
        graph.push_quad(GpuQuad::new(
            GpuLayer::PaneBackground,
            PixelRect::new(0, 0, 1, 1),
            rgba(200, 0, 0, 255),
        ));
        graph.push_snapshot_images(&snapshot, RenderGeometry::new(1, 1, 1, 1), 0, None);
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless adapter");
        let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 256)
            .expect("renderer");
        let bytes = renderer
            .render_headless_rgba8(&graph, Duration::from_secs(5))
            .expect("readback");
        bytes.try_into().expect("one RGBA pixel")
    }

    let _gpu = gpu_test_guard();
    assert_eq!(
        render_pixel("AAD/gA=="),
        rgba(0, 0, 255, 128),
        "nonzero image alpha is authoritative, not source-over blended"
    );
    assert_eq!(
        render_pixel("AAD/AA=="),
        rgba(200, 0, 0, 255),
        "fully transparent image pixels leave the lower layer unchanged"
    );
}

#[test]
fn same_z_fragment_group_follows_whole_group_in_both_insertion_directions() {
    const RED_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
    let _gpu = gpu_test_guard();
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(b"\x1b_Ga=T,q=1,i=91,f=24,s=1,v=1;AP8A\x1b\\");
    terminal.feed(format!("\x1b]1337;File=inline=1;width=1px;height=1px:{RED_PNG}\x07").as_bytes());
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let mut graph = RenderGraph::new(1, 1);
    graph.push_snapshot_images(&snapshot, RenderGeometry::new(1, 1, 1, 1), 0, None);
    assert_eq!(graph.planned_image_draw_count(), 2);

    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer =
        GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 256).expect("renderer");
    assert_eq!(
        renderer
            .render_headless_rgba8(&graph, Duration::from_secs(5),)
            .expect("same-z readback"),
        rgba(0, 255, 0, 255),
        "the Kitty placement fragment group must follow the iTerm whole group"
    );

    let mut reversed = Terminal::new(TerminalSize::new(1, 1));
    reversed.feed(format!("\x1b]1337;File=inline=1;width=1px;height=1px:{RED_PNG}\x07").as_bytes());
    reversed.feed(b"\x1b[H");
    reversed.feed(b"\x1b_Ga=T,q=1,i=91,f=24,s=1,v=1;AP8A\x1b\\");
    let reversed_snapshot = TerminalRenderSnapshot::from_terminal(&reversed);
    let mut reversed_graph = RenderGraph::new(1, 1);
    reversed_graph.push_snapshot_images(
        &reversed_snapshot,
        RenderGeometry::new(1, 1, 1, 1),
        0,
        None,
    );
    assert_eq!(
        renderer
            .render_headless_rgba8(&reversed_graph, Duration::from_secs(5))
            .expect("reverse insertion readback"),
        rgba(0, 255, 0, 255),
        "whole/fragment grouping must not depend on snapshot insertion"
    );
}

#[test]
fn mixed_whole_and_fragment_images_match_cpu_full_and_damage_on_real_gpu() {
    const RED_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
    let _gpu = gpu_test_guard();
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(format!("\x1b]1337;File=inline=1;width=1px;height=1px:{RED_PNG}\x07").as_bytes());
    terminal.feed(b"\x1b[H");
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=1,f=24,s=1,v=1,c=1,r=1;AAD/\x1b\\");
    terminal.feed(b"\x1b[?25l");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let geometry = RenderGeometry::new(1, 1, 1, 1);

    let mut cpu_renderer = PixelRenderer::new();
    cpu_renderer.set_default_background(rgba(8, 9, 10, 255));
    cpu_renderer.set_cursor_opacity(0.0);
    let mut cpu_full = vec![0; 4];
    cpu_renderer.render(&snapshot, &mut cpu_full, 1, 1, 1, 1);
    let mut cpu_damage = rgba(99, 98, 97, 255).to_vec();
    cpu_renderer.render_damage(
        &snapshot,
        &[DamageRegion::new(0, 0, 1, 1)],
        &mut cpu_damage,
        geometry,
    );
    assert_eq!(cpu_damage, cpu_full);

    let mut graph = RenderGraph::new(1, 1);
    graph.push_quad(GpuQuad::new(
        GpuLayer::PaneBackground,
        PixelRect::new(0, 0, 1, 1),
        rgba(8, 9, 10, 255),
    ));
    graph.push_snapshot_images(&snapshot, geometry, 0, None);
    assert_eq!(graph.planned_image_draw_count(), 2);
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut gpu_renderer =
        GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 256).expect("renderer");
    let gpu = gpu_renderer
        .render_headless_rgba8(&graph, Duration::from_secs(5))
        .expect("mixed whole/fragment real GPU readback");

    assert_eq!(cpu_full, rgba(0, 0, 255, 255));
    assert_eq!(gpu, cpu_full);
}

#[test]
fn mixed_texture_and_legacy_image_nodes_have_one_real_gpu_order() {
    const RED_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
    let _gpu = gpu_test_guard();
    let geometry = RenderGeometry::new(1, 1, 1, 1);
    let mut fragment_terminal = Terminal::new(TerminalSize::new(1, 1));
    fragment_terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=1,f=24,s=1,v=1,c=1,r=1;AAD/\x1b\\");
    let fragment_snapshot = TerminalRenderSnapshot::from_terminal(&fragment_terminal);
    let mut whole_terminal = Terminal::new(TerminalSize::new(1, 1));
    whole_terminal
        .feed(format!("\x1b]1337;File=inline=1;width=1px;height=1px:{RED_PNG}\x07").as_bytes());
    let whole_snapshot = TerminalRenderSnapshot::from_terminal(&whole_terminal);

    let mut graph = RenderGraph::new(1, 1);
    graph.push_snapshot_images(&fragment_snapshot, geometry, 0, None);
    graph.push_image(GpuImage::new(
        ImageProtocol::Iterm,
        0,
        PixelRect::new(0, 0, 1, 1),
        rgba(0, 255, 0, 255),
    ));
    graph.push_snapshot_images(&whole_snapshot, geometry, 0, None);

    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer =
        GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 256).expect("renderer");
    assert_eq!(
        renderer
            .render_headless_rgba8(&graph, Duration::from_secs(5))
            .expect("mixed GraphNode readback"),
        rgba(0, 0, 255, 255),
        "the fragment group must follow both legacy and texture whole-image nodes"
    );
}

#[test]
fn gpu_image_materialization_budget_errors_before_mutating_renderer_state() {
    const RED_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
    let _gpu = gpu_test_guard();
    let mut terminal = Terminal::new(TerminalSize::new(1, 1));
    terminal.feed(format!("\x1b]1337;File=inline=1;width=2px;height=2px:{RED_PNG}\x07").as_bytes());
    terminal.feed(b"\x1b[H");
    terminal.feed(b"\x1b_Ga=T,C=1,q=1,i=1,f=24,s=1,v=1,c=1,r=1;AAD/\x1b\\");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let mut graph = RenderGraph::new(2, 2);
    graph.push_snapshot_images(&snapshot, RenderGeometry::new(2, 2, 2, 2), 0, None);
    assert_eq!(graph.planned_image_draw_count(), 2);

    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer =
        GpuLayerRenderer::new_with_budgets(&context, wgpu::TextureFormat::Rgba8Unorm, 256, 40)
            .expect("renderer");

    let mut repeated_terminal = Terminal::new(TerminalSize::new(1, 1));
    repeated_terminal
        .feed(format!("\x1b]1337;File=inline=1;width=2px;height=2px:{RED_PNG}\x07").as_bytes());
    let repeated_snapshot = TerminalRenderSnapshot::from_terminal(&repeated_terminal);
    let mut repeated = RenderGraph::new(2, 2);
    repeated.push_snapshot_images(&repeated_snapshot, RenderGeometry::new(2, 2, 2, 2), 0, None);
    repeated.push_snapshot_images(&repeated_snapshot, RenderGeometry::new(2, 2, 2, 2), 0, None);
    renderer
        .upload(&repeated)
        .expect("two exact placements share one 32-byte retained texture");
    assert_eq!(renderer.texture_cache_metrics().entries, 1);
    assert_eq!(renderer.texture_cache_metrics().retained_bytes, 32);

    let upload_before = renderer.upload_metrics();
    let cache_before = renderer.texture_cache_metrics();
    let error = renderer
        .upload(&graph)
        .expect_err("two 2x2 draws require 64 retained bytes");
    assert!(error.to_string().contains("budget"));
    assert_eq!(renderer.upload_metrics(), upload_before);
    assert_eq!(renderer.texture_cache_metrics(), cache_before);
}

#[test]
fn every_adjacent_layer_pair_is_submitted_in_canonical_order() {
    fn push_layer(graph: &mut RenderGraph, layer: GpuLayer, rect: PixelRect, color: [u8; 4]) {
        match layer {
            GpuLayer::UltraNegativeImage => graph.push_image(GpuImage::new(
                ImageProtocol::Kitty,
                i32::MIN / 2 - 1,
                rect,
                color,
            )),
            GpuLayer::NegativeImage => {
                graph.push_image(GpuImage::new(ImageProtocol::Kitty, -1, rect, color));
            }
            GpuLayer::PositiveImage => {
                graph.push_image(GpuImage::new(ImageProtocol::Kitty, 0, rect, color));
            }
            _ => graph.push_quad(GpuQuad::new(layer, rect, color)),
        }
    }

    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer =
        GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 4096).expect("renderer");
    for pair in GpuLayer::canonical_order().windows(2) {
        let lower = pair[0];
        let upper = pair[1];
        let mut graph = RenderGraph::new(2, 1);
        // Reverse insertion proves the explicit graph order is authoritative.
        push_layer(
            &mut graph,
            upper,
            PixelRect::new(1, 0, 1, 1),
            rgba(0, 0, 220, 255),
        );
        push_layer(
            &mut graph,
            lower,
            PixelRect::new(0, 0, 2, 1),
            rgba(220, 0, 0, 255),
        );
        assert_eq!(graph.ordered_content_layers(), vec![lower, upper]);
        let actual = renderer
            .render_headless_rgba8(&graph, Duration::from_secs(5))
            .expect("adjacent layer readback");
        let lower_pixel = rgba(220, 0, 0, 255);
        let upper_pixel = rgba(0, 0, 220, 255);
        assert_eq!(
            actual,
            [lower_pixel, upper_pixel].concat(),
            "incorrect GPU ordering for {lower:?} -> {upper:?}"
        );
    }
}

#[test]
fn renderer_rejects_a_foreign_device_and_queue_before_mutating_upload_state() {
    let _gpu = gpu_test_guard();
    let context_a = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("context A");
    let mut context_b = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("context B");
    let mut renderer = GpuLayerRenderer::new(&context_a, wgpu::TextureFormat::Rgba8Unorm, 256)
        .expect("renderer A");
    let mut graph = RenderGraph::new(1, 1);
    graph.push_quad(GpuQuad::new(
        GpuLayer::PaneBackground,
        PixelRect::new(0, 0, 1, 1),
        rgba(1, 2, 3, 255),
    ));
    let before = renderer.upload_metrics();
    let error = renderer
        .upload_from(&context_b, &graph)
        .expect_err("foreign context must be rejected");
    assert!(error.to_string().contains("different GPU context"));
    assert_eq!(renderer.upload_metrics(), before);
    context_b
        .run_headless_submission_probe(Duration::from_secs(5))
        .expect("rejection must not poison the foreign device");
}

#[test]
fn headless_readback_rejects_device_and_host_resource_limits_before_gpu_creation() {
    let _gpu = gpu_test_guard();
    let mut context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer =
        GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 256).expect("renderer");
    let max_dimension = context.device().limits().max_texture_dimension_2d;
    let oversized = RenderGraph::new(max_dimension.saturating_add(1), 1);
    assert!(
        renderer
            .render_headless_rgba8(&oversized, Duration::from_secs(5),)
            .expect_err("max dimension must be validated")
            .to_string()
            .contains("limit")
    );
    let budget_square = RenderGraph::new(max_dimension, max_dimension);
    assert!(
        renderer
            .render_headless_rgba8(&budget_square, Duration::from_secs(5),)
            .expect_err("host readback budget must be validated")
            .to_string()
            .contains("budget")
    );
    context
        .run_headless_submission_probe(Duration::from_secs(5))
        .expect("preflight rejection must not create an uncaptured GPU fault");
}

#[test]
fn non_power_of_two_instance_budget_accepts_legal_active_bytes_at_boundary() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 100)
        .expect("100-byte renderer");
    let mut graph = RenderGraph::new(1, 1);
    for _ in 0..3 {
        graph.push_quad(GpuQuad::new(
            GpuLayer::CellBackground,
            PixelRect::new(0, 0, 1, 1),
            rgba(0, 0, 0, 255),
        ));
    }
    renderer
        .upload(&graph)
        .expect("96 active bytes fit a 100-byte budget");
    graph.push_quad(GpuQuad::new(
        GpuLayer::CellBackground,
        PixelRect::new(0, 0, 1, 1),
        rgba(0, 0, 0, 255),
    ));
    assert!(
        renderer
            .upload(&graph)
            .expect_err("128 active bytes exceed a 100-byte budget")
            .to_string()
            .contains("budget")
    );
}

#[test]
fn instance_budget_counts_visible_glyph_blocks_but_not_clipped_draws() {
    let _gpu = gpu_test_guard();
    let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let mut renderer =
        GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 96).expect("renderer");

    let mut glyphs = RenderGraph::new(1, 1);
    for _ in 0..3 {
        glyphs.push_quad(GpuQuad::new(
            GpuLayer::Glyph,
            PixelRect::new(0, 0, 1, 1),
            rgba(1, 2, 3, 255),
        ));
    }
    renderer
        .upload(&glyphs)
        .expect("Task 17 custom block glyphs consume bounded instances");
    assert_eq!(renderer.upload_metrics().bytes_written, 96);

    let mut offscreen = RenderGraph::new(1, 1);
    for _ in 0..3 {
        offscreen.push_quad(GpuQuad::new(
            GpuLayer::CellBackground,
            PixelRect::new(10, 10, 1, 1),
            rgba(1, 2, 3, 255),
        ));
    }
    renderer
        .upload(&offscreen)
        .expect("clipped nodes consume no instance budget");
    assert_eq!(renderer.upload_metrics().bytes_written, 0);
}
