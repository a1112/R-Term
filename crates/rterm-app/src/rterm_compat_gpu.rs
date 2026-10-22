use std::{error::Error, io};

use rterm_fonts::FontConfig;
use rterm_render_core::{RenderGeometry, TerminalRenderSnapshot};
use rterm_render_cpu::TextPaintConfig;
use rterm_render_wgpu::gpu::{GpuLayerRenderer, GpuTextConfig, GpuTextPrepareReport};

use crate::platform_fonts::{CatalogActivation, FontCatalogMode, PlatformFontRepository};

pub(crate) struct LegacyTextSettings {
    pub(crate) fonts: FontConfig,
    pub(crate) text: GpuTextConfig,
}

#[allow(
    clippy::too_many_arguments,
    reason = "one full-frame transaction binds repository, renderer and paint inputs"
)]
pub(crate) fn prepare_full_frame(
    repository: &mut PlatformFontRepository,
    renderer: &mut GpuLayerRenderer,
    settings: &LegacyTextSettings,
    snapshot: &TerminalRenderSnapshot,
    geometry: RenderGeometry,
    paint: &TextPaintConfig,
    dpi_scale: f32,
    zoom: f32,
) -> Result<GpuTextPrepareReport, Box<dyn Error>> {
    let result: Result<GpuTextPrepareReport, Box<dyn Error>> = (|| {
        repository.preflight_snapshot(
            snapshot,
            renderer
                .text_catalog_mut()
                .ok_or_else(|| io::Error::other("legacy GPU text is not enabled"))?,
        )?;
        reset_text(repository, renderer, settings)?;
        for attempt in 0..=1 {
            let report = renderer.prepare_text(snapshot, geometry, &[], paint, dpi_scale, zoom)?;
            if attempt == 0 && !report.missing_glyphs.is_empty() {
                let activation = repository.activate_missing_glyphs(
                    &report.missing_glyphs,
                    renderer
                        .text_catalog_mut()
                        .ok_or_else(|| io::Error::other("legacy GPU text is not enabled"))?,
                )?;
                if matches!(activation, CatalogActivation::CatalogExpanded { .. }) {
                    reset_text(repository, renderer, settings)?;
                    continue;
                }
            }
            return Ok(report);
        }
        unreachable!("one bounded legacy full-frame restart always returns")
    })();
    match result {
        Ok(report) => Ok(report),
        Err(error) => {
            // Never publish the frame after any error. If a device/allocator
            // failure also prevents cleanup, the caller must quarantine this
            // renderer; do not disguise that failure as a successful discard.
            reset_text(repository, renderer, settings).map_err(|cleanup| {
                io::Error::other(format!("legacy GPU preparation failed: {error}; partial-frame cleanup also failed: {cleanup}"))
            })?;
            Err(error)
        }
    }
}

fn reset_text(
    repository: &PlatformFontRepository,
    renderer: &mut GpuLayerRenderer,
    settings: &LegacyTextSettings,
) -> Result<(), Box<dyn Error>> {
    // Frozen GPU cache scope ignores catalog incarnation. A fresh text backend
    // discards all instance-scoped IDs and prepared rows, including same-epoch
    // replacements. This conservative path is only for the frozen profile.
    let catalog = repository.rebuild_catalog_from_active(FontCatalogMode::Lazy)?;
    renderer.enable_text(catalog, settings.fonts.clone(), settings.text)?;
    Ok(())
}

/// Release the exposed catalog without touching the lost device. Frozen R-Term
/// does not expose CPU cache retirement: retain its private caches and GPU
/// handles with the quarantined renderer until the existing safe shutdown path.
/// Never prepare or present this renderer again, including on rebuild failure.
pub(crate) fn retire_catalog_before_rebuild<T>(
    lost_renderer: &mut GpuLayerRenderer,
    rebuild: impl FnOnce(&mut GpuLayerRenderer) -> Result<T, Box<dyn Error>>,
) -> Result<T, Box<dyn Error>> {
    if let Some(catalog) = lost_renderer.text_catalog_mut() {
        *catalog = rterm_fonts::FontCatalog::new("retired");
    }
    rebuild(lost_renderer).map_err(|error| {
        io::Error::other(format!(
            "rebuild legacy GPU renderer after releasing catalog (private caches retained): {error}"
        ))
        .into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_fonts::FontCatalogMode;
    use rterm_core::TerminalSize;
    use rterm_fonts::RasterCacheConfig;
    use rterm_render_wgpu::gpu::{GpuContext, GpuContextOptions, RenderGraph};
    use rterm_terminal::Terminal;
    use std::time::Duration;

    fn settings() -> LegacyTextSettings {
        LegacyTextSettings {
            fonts: FontConfig::new("Noto Sans").with_fallbacks(["Noto Sans SC"]),
            text: GpuTextConfig::new(4 * 1024 * 1024, RasterCacheConfig::new(4 * 1024 * 1024)),
        }
    }

    fn renderer(context: &GpuContext, repository: &mut PlatformFontRepository) -> GpuLayerRenderer {
        let mut renderer =
            GpuLayerRenderer::new(context, wgpu::TextureFormat::Rgba8Unorm, 64 * 1024).unwrap();
        let config = settings();
        renderer
            .enable_text(
                repository.build_catalog(FontCatalogMode::Lazy).unwrap(),
                config.fonts,
                config.text,
            )
            .unwrap();
        renderer
    }

    #[test]
    fn legacy_gpu_full_frame_draws_pixels_and_rebuilds_every_visible_row() {
        let context =
            pollster::block_on(GpuContext::new_headless(GpuContextOptions::default())).unwrap();
        let mut repository = PlatformFontRepository::production_index_for_os("test");
        let mut renderer = renderer(&context, &mut repository);
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed(b"FIRST\r\nSECOND");
        let graph = RenderGraph::new(128, 48);
        let blank = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 64 * 1024)
            .unwrap()
            .render_headless_rgba8(&graph, Duration::from_secs(5))
            .unwrap();
        for _ in 0..2 {
            let report = prepare_full_frame(
                &mut repository,
                &mut renderer,
                &settings(),
                &TerminalRenderSnapshot::from_terminal(&terminal),
                RenderGeometry::new(128, 48, 16, 24),
                &TextPaintConfig::default(),
                1.0,
                1.0,
            )
            .unwrap();
            assert_eq!(report.prepared_rows, [0, 1]);
            assert!(report.missing_glyphs.is_empty());
            assert_ne!(
                renderer
                    .render_headless_rgba8(&graph, Duration::from_secs(5))
                    .unwrap(),
                blank
            );
        }
    }

    #[test]
    fn legacy_gpu_late_fallback_renders_real_cjk_in_the_same_full_frame() {
        let context =
            pollster::block_on(GpuContext::new_headless(GpuContextOptions::default())).unwrap();
        let mut repository = PlatformFontRepository::late_missing_fixture();
        let mut renderer = renderer(&context, &mut repository);
        let mut terminal = Terminal::new(TerminalSize::new(4, 1));
        terminal.feed("中文".as_bytes());
        let report = prepare_full_frame(
            &mut repository,
            &mut renderer,
            &settings(),
            &TerminalRenderSnapshot::from_terminal(&terminal),
            RenderGeometry::new(64, 24, 16, 24),
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .unwrap();
        assert!(report.missing_glyphs.is_empty());
        assert_eq!(renderer.text_catalog_mut().unwrap().generation(), 3);
        assert!(renderer.text_atlas_metrics().unwrap().entries > 0);
        let graph = RenderGraph::new(64, 24);
        let blank = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 64 * 1024)
            .unwrap()
            .render_headless_rgba8(&graph, Duration::from_secs(5))
            .unwrap();
        assert_ne!(
            renderer
                .render_headless_rgba8(&graph, Duration::from_secs(5))
                .unwrap(),
            blank
        );
    }

    #[test]
    fn legacy_gpu_invalid_late_font_clears_partial_glyphs_before_returning_error() {
        let context =
            pollster::block_on(GpuContext::new_headless(GpuContextOptions::default())).unwrap();
        let mut repository = PlatformFontRepository::invalid_late_missing_fixture();
        let mut renderer = renderer(&context, &mut repository);
        let mut terminal = Terminal::new(TerminalSize::new(4, 1));
        terminal.feed("中文".as_bytes());
        let graph = RenderGraph::new(64, 24);
        let blank = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 64 * 1024)
            .unwrap()
            .render_headless_rgba8(&graph, Duration::from_secs(5))
            .unwrap();
        for _ in 0..2 {
            let error = prepare_full_frame(
                &mut repository,
                &mut renderer,
                &settings(),
                &TerminalRenderSnapshot::from_terminal(&terminal),
                RenderGeometry::new(64, 24, 16, 24),
                &TextPaintConfig::default(),
                1.0,
                1.0,
            )
            .unwrap_err();
            assert_eq!(renderer.text_atlas_metrics().unwrap().entries, 0);
            assert_eq!(
                renderer
                    .render_headless_rgba8(&graph, Duration::from_secs(5))
                    .unwrap(),
                blank
            );
            assert!(error.to_string().contains("invalid font"));
        }
    }

    #[test]
    fn legacy_gpu_same_epoch_replacement_and_resize_rebuild_pixels() {
        let context =
            pollster::block_on(GpuContext::new_headless(GpuContextOptions::default())).unwrap();
        let mut repository = PlatformFontRepository::production_index_for_os("test");
        let mut renderer = renderer(&context, &mut repository);
        let mut terminal = Terminal::new(TerminalSize::new(4, 1));
        terminal.feed(b"TEXT");
        let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
        let paint = TextPaintConfig::default();
        let config = settings();
        let geometry = RenderGeometry::new(64, 24, 16, 24);
        prepare_full_frame(
            &mut repository,
            &mut renderer,
            &config,
            &snapshot,
            geometry,
            &paint,
            1.0,
            1.0,
        )
        .unwrap();
        let graph = RenderGraph::new(64, 24);
        let before = renderer
            .render_headless_rgba8(&graph, Duration::from_secs(5))
            .unwrap();
        let old = renderer.text_catalog_mut().unwrap();
        let identity = (old.incarnation(), old.generation());
        *old = repository
            .rebuild_catalog_from_active(FontCatalogMode::Lazy)
            .unwrap();
        assert_eq!(old.generation(), identity.1);
        assert_ne!(old.incarnation(), identity.0);
        prepare_full_frame(
            &mut repository,
            &mut renderer,
            &config,
            &snapshot,
            geometry,
            &paint,
            1.0,
            1.0,
        )
        .unwrap();
        assert_eq!(
            renderer
                .render_headless_rgba8(&graph, Duration::from_secs(5))
                .unwrap(),
            before
        );
        let report = prepare_full_frame(
            &mut repository,
            &mut renderer,
            &config,
            &snapshot,
            RenderGeometry::new(128, 48, 32, 48),
            &paint,
            2.0,
            1.0,
        )
        .unwrap();
        assert_eq!(report.prepared_rows, [0]);
        let resized = renderer
            .render_headless_rgba8(&RenderGraph::new(128, 48), Duration::from_secs(5))
            .unwrap();
        assert_eq!(resized.len(), 128 * 48 * 4);
        assert!(resized.chunks_exact(4).any(|pixel| pixel != &resized[..4]));
    }

    #[test]
    fn legacy_gpu_static_missing_glyphs_converge_across_bounded_frames() {
        let context =
            pollster::block_on(GpuContext::new_headless(GpuContextOptions::default())).unwrap();
        let mut repository = PlatformFontRepository::repeated_late_missing_fixture();
        let mut renderer = renderer(&context, &mut repository);
        let mut terminal = Terminal::new(TerminalSize::new(4, 1));
        terminal.feed("中文".as_bytes());
        let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
        let geometry = RenderGeometry::new(64, 24, 16, 24);
        let first = prepare_full_frame(
            &mut repository,
            &mut renderer,
            &settings(),
            &snapshot,
            geometry,
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .unwrap();
        assert!(!first.missing_glyphs.is_empty());
        assert_eq!(renderer.text_catalog_mut().unwrap().generation(), 3);
        assert!(repository.has_pending_fallbacks(&first.missing_glyphs));
        let second = prepare_full_frame(
            &mut repository,
            &mut renderer,
            &settings(),
            &snapshot,
            geometry,
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .unwrap();
        assert!(second.missing_glyphs.is_empty());
        assert_eq!(renderer.text_catalog_mut().unwrap().generation(), 4);
    }

    #[test]
    fn legacy_gpu_cleanup_failure_preserves_both_errors_and_cannot_report_a_frame() {
        let context =
            pollster::block_on(GpuContext::new_headless(GpuContextOptions::default())).unwrap();
        let mut repository = PlatformFontRepository::production_index_for_os("test");
        let mut renderer = renderer(&context, &mut repository);
        let terminal = Terminal::new(TerminalSize::new(4, 1));
        let mut invalid = settings();
        invalid.text.budget_bytes = 0;
        let error = prepare_full_frame(
            &mut repository,
            &mut renderer,
            &invalid,
            &TerminalRenderSnapshot::from_terminal(&terminal),
            RenderGeometry::new(64, 24, 16, 24),
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("legacy GPU preparation failed"));
        assert!(message.contains("partial-frame cleanup also failed"));
        assert_eq!(message.matches("glyph atlas budget").count(), 2);
    }

    #[test]
    fn legacy_gpu_recovery_releases_catalog_before_rebuilding_on_the_new_device() {
        let mut context =
            pollster::block_on(GpuContext::new_headless(GpuContextOptions::default())).unwrap();
        let mut repository = PlatformFontRepository::late_missing_fixture();
        let mut lost = renderer(&context, &mut repository);
        let mut terminal = Terminal::new(TerminalSize::new(4, 1));
        terminal.feed("中文".as_bytes());
        let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
        let geometry = RenderGeometry::new(64, 24, 16, 24);
        let graph = RenderGraph::new(64, 24);
        prepare_full_frame(
            &mut repository,
            &mut lost,
            &settings(),
            &snapshot,
            geometry,
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .unwrap();
        let before = lost
            .render_headless_rgba8(&graph, Duration::from_secs(5))
            .unwrap();
        let old_catalog = lost.text_catalog_mut().unwrap();
        let old_incarnation = old_catalog.incarnation();
        assert!(old_catalog.face_count() > 0);
        let old_entries = lost.text_atlas_metrics().unwrap().entries;
        assert!(old_entries > 0);
        let generation = context.generation();
        context.inject_device_loss_for_test();
        let fault = context
            .run_headless_submission_probe(Duration::from_secs(5))
            .unwrap_err();
        assert_eq!(
            fault.kind(),
            rterm_render_wgpu::gpu::GpuContextErrorKind::DeviceLost
        );
        pollster::block_on(context.recover_device()).unwrap();
        assert_ne!(context.generation(), generation);

        let mut replacement = retire_catalog_before_rebuild(&mut lost, |retired| {
            let catalog = retired.text_catalog_mut().unwrap();
            assert_eq!(
                catalog.face_count(),
                0,
                "catalog must be released before allocating replacement fonts"
            );
            assert_ne!(catalog.incarnation(), old_incarnation);
            // Old private caches and GPU handles stay quarantined, not falsely
            // reported as discarded or destroyed on the lost device.
            assert_eq!(retired.text_atlas_metrics().unwrap().entries, old_entries);
            let mut replacement =
                GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 64 * 1024)?;
            let config = settings();
            replacement.enable_text(
                repository.rebuild_catalog_from_active(FontCatalogMode::Lazy)?,
                config.fonts,
                config.text,
            )?;
            Ok(replacement)
        })
        .unwrap();
        let report = prepare_full_frame(
            &mut repository,
            &mut replacement,
            &settings(),
            &snapshot,
            geometry,
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .unwrap();
        assert!(report.missing_glyphs.is_empty());
        assert_eq!(
            replacement
                .render_headless_rgba8(&graph, Duration::from_secs(5))
                .unwrap(),
            before
        );
        assert_eq!(lost.text_catalog_mut().unwrap().face_count(), 0);
        // Preserve the app's ordering: both renderer owners outlive preparation,
        // and are released before their context at a safe shutdown boundary.
        drop(replacement);
        drop(lost);
        drop(context);
    }

    #[test]
    fn legacy_gpu_recovery_failure_keeps_retired_catalog_empty_and_preserves_error() {
        let context =
            pollster::block_on(GpuContext::new_headless(GpuContextOptions::default())).unwrap();
        let mut repository = PlatformFontRepository::production_index_for_os("test");
        let mut lost = renderer(&context, &mut repository);
        for _ in 0..2 {
            let error = retire_catalog_before_rebuild(&mut lost, |retired| {
                assert_eq!(retired.text_catalog_mut().unwrap().face_count(), 0);
                Err::<(), _>(io::Error::other("replacement unavailable").into())
            })
            .unwrap_err();
            assert!(error.to_string().contains("replacement unavailable"));
            assert!(error.to_string().contains("private caches retained"));
            assert_eq!(lost.text_catalog_mut().unwrap().face_count(), 0);
        }
    }
}
