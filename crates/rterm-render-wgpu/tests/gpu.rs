use rterm_render_wgpu::gpu::{
    GpuContext, GpuContextOptions, SurfaceFault, SurfaceRecovery, SurfaceRecoveryState,
};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    time::Duration,
};

#[test]
fn surface_fault_policy_reconfigures_lost_and_outdated_surfaces() {
    assert_eq!(
        SurfaceRecovery::for_fault(SurfaceFault::Outdated),
        SurfaceRecovery::ReconfigureAndRetry
    );
    assert_eq!(
        SurfaceRecovery::for_fault(SurfaceFault::Lost),
        SurfaceRecovery::RecreateAndRetry
    );
    assert_eq!(
        SurfaceRecovery::for_fault(SurfaceFault::Timeout),
        SurfaceRecovery::SkipFrame
    );
    assert_eq!(
        SurfaceRecovery::for_fault(SurfaceFault::Occluded),
        SurfaceRecovery::SkipFrame
    );
    assert_eq!(
        SurfaceRecovery::for_fault(SurfaceFault::Validation),
        SurfaceRecovery::Report
    );
}

#[test]
fn surface_recovery_state_allows_only_one_reconfigure_or_recreate_retry() {
    let mut outdated = SurfaceRecoveryState::new();
    assert_eq!(
        outdated.action(SurfaceFault::Outdated),
        SurfaceRecovery::ReconfigureAndRetry
    );
    assert_eq!(
        outdated.action(SurfaceFault::Outdated),
        SurfaceRecovery::Report
    );

    let mut lost = SurfaceRecoveryState::new();
    assert_eq!(
        lost.action(SurfaceFault::Lost),
        SurfaceRecovery::RecreateAndRetry
    );
    assert_eq!(lost.action(SurfaceFault::Lost), SurfaceRecovery::Report);
}

#[test]
fn headless_context_reports_the_selected_adapter() {
    let mut context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("the native backend or its software fallback must provide a headless adapter");
    let metrics = context.metrics();

    assert!(!metrics.backend.is_empty());
    assert!(!metrics.adapter_name.is_empty());
    assert!(!metrics.adapter_type.is_empty());
    assert_eq!(
        metrics.software_adapter,
        matches!(metrics.adapter_type.as_str(), "cpu")
    );
    assert!(metrics.surface_format.is_none());
    assert!(metrics.present_mode.is_none());
    assert!(metrics.surface_width.is_none());
    assert!(metrics.surface_height.is_none());
    assert_eq!(metrics.rendered_frames, 0);
    assert_eq!(metrics.presented_frames, 0);

    context
        .run_headless_submission_probe(Duration::from_secs(5))
        .expect("tiny write, submit, and bounded poll must complete");
}

#[test]
fn uncaptured_validation_is_reported_without_panicking_or_counting_a_frame() {
    let mut context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
        .expect("headless adapter");
    let invalid_size = context
        .device()
        .limits()
        .max_buffer_size
        .checked_add(1)
        .expect("native max buffer size leaves room for an invalid probe");

    let creation = catch_unwind(AssertUnwindSafe(|| {
        context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("rterm-invalid-buffer-probe"),
            size: invalid_size,
            usage: wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }));
    assert!(
        creation.is_ok(),
        "registered uncaptured error handler must replace wgpu's default panic"
    );
    context
        .device()
        .poll(wgpu::PollType::Poll)
        .expect("validation probe poll");

    let error = context
        .run_headless_submission_probe(Duration::from_secs(5))
        .expect_err("pending uncaptured validation must stop later GPU work");
    assert_eq!(
        error.kind(),
        rterm_render_wgpu::gpu::GpuContextErrorKind::Validation
    );
    assert_eq!(context.metrics().rendered_frames, 0);
    assert_eq!(context.metrics().presented_frames, 0);
    assert_eq!(context.metrics().uncaptured_errors, 1);
}

mod attribution_stage {
    use rterm_fonts::{FontCatalog, FontConfig, FontSource, RasterCacheConfig};
    use rterm_render_wgpu::gpu::{
        GpuContext, GpuContextOptions, GpuInitializationResourceSnapshot, GpuInitializationStage,
        GpuLayerRenderer, GpuTextConfig,
    };

    fn valid_snapshot(stage: GpuInitializationStage) -> GpuInitializationResourceSnapshot {
        let mut snapshot = GpuInitializationResourceSnapshot {
            instance_count: 1,
            surface_count: 1,
            ..GpuInitializationResourceSnapshot::default()
        };
        if stage >= GpuInitializationStage::AdapterDevice {
            snapshot.adapter_count = 1;
            snapshot.device_count = 1;
            snapshot.queue_count = 1;
            snapshot.backend = Some("vulkan".to_owned());
            snapshot.adapter_name = Some("test-adapter".to_owned());
        }
        if stage >= GpuInitializationStage::ConfiguredSurfaceClear {
            snapshot.surface_configure_count = 1;
            snapshot.surface_acquire_count = 1;
            snapshot.clear_present_count = 1;
        }
        if stage >= GpuInitializationStage::LayerPipelines {
            snapshot.pipeline_count = 1;
            snapshot.pipeline_layout_count = 1;
            snapshot.materialized_buffer_count = 1;
            snapshot.total_allocated_buffer_bytes = 8;
        }
        snapshot
    }

    #[test]
    fn stage_resource_matrix_is_fail_closed() {
        for stage in GpuInitializationStage::ORDERED {
            valid_snapshot(stage)
                .validate_at(stage)
                .unwrap_or_else(|violations| panic!("valid {stage:?} snapshot: {violations:?}"));
        }

        let mut later_work = valid_snapshot(GpuInitializationStage::InstanceSurface);
        later_work.adapter_count = 1;
        assert!(
            later_work
                .validate_at(GpuInitializationStage::InstanceSurface)
                .expect_err("an adapter before AdapterDevice must fail closed")
                .iter()
                .any(|violation| violation.contains("adapter_count"))
        );

        let mut fabricated_identity = valid_snapshot(GpuInitializationStage::InstanceSurface);
        fabricated_identity.backend = Some("dx12".to_owned());
        assert!(
            fabricated_identity
                .validate_at(GpuInitializationStage::InstanceSurface)
                .expect_err("backend identity before AdapterDevice must fail closed")
                .iter()
                .any(|violation| violation.contains("backend"))
        );

        let mut absent_identity = valid_snapshot(GpuInitializationStage::AdapterDevice);
        absent_identity.adapter_name = None;
        assert!(
            absent_identity
                .validate_at(GpuInitializationStage::AdapterDevice)
                .expect_err("adapter identity is required from AdapterDevice onward")
                .iter()
                .any(|violation| violation.contains("adapter_name"))
        );
    }

    #[test]
    fn production_composition_is_unchanged() {
        assert_eq!(
            GpuInitializationStage::ORDERED,
            [
                GpuInitializationStage::InstanceSurface,
                GpuInitializationStage::AdapterDevice,
                GpuInitializationStage::ConfiguredSurfaceClear,
                GpuInitializationStage::LayerPipelines,
            ]
        );

        // The compatibility entry point remains callable while its implementation
        // becomes the production composition of the four explicit stages.
        std::hint::black_box(GpuContext::finish_windowed);
    }

    #[test]
    fn text_owner_counts_only_base_materialization_for_each_enable() {
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless attribution adapter");
        let mut renderer =
            GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8UnormSrgb, 64 * 1024)
                .expect("layer renderer");
        let enable = |renderer: &mut GpuLayerRenderer| {
            let source = FontSource::new(
                "R-Term Stage 7 Fixture",
                include_bytes!("../../../tests/fixtures/fonts/NotoSans-Latin.fixture.ttf").to_vec(),
            );
            let catalog = FontCatalog::from_sources("en-US", [source]).expect("fixture catalog");
            renderer
                .enable_text(
                    catalog,
                    FontConfig::new("R-Term Stage 7 Fixture"),
                    GpuTextConfig::new(4 * 1024 * 1024, RasterCacheConfig::new(4 * 1024 * 1024)),
                )
                .expect("enable fixture text");
        };

        enable(&mut renderer);
        let fixture = renderer.initialization_resources();
        assert_eq!(fixture.base_text_renderer_materialization_count, 1);
        assert_eq!(fixture.cursor_text_renderer_materialization_count, 0);

        enable(&mut renderer);
        let full = renderer.initialization_resources();
        assert_eq!(full.base_text_renderer_materialization_count, 2);
        assert_eq!(full.cursor_text_renderer_materialization_count, 0);
    }

    #[test]
    fn clear_present_bookkeeping_precedes_post_present_fault_checks_and_is_single_shot() {
        let source = include_str!("../src/gpu/context.rs");
        let function = source
            .split("pub fn present_clear_once")
            .nth(1)
            .expect("present_clear_once source")
            .split("/// Reconfigures the swap chain")
            .next()
            .expect("bounded present_clear_once source");
        assert!(
            source.contains("suboptimal initialization clear frame"),
            "suboptimal acquisition needs an explicit fail-closed attribution contract"
        );
        assert!(
            function.contains("run_initialization_clear_transaction"),
            "present_clear_once must execute the tested acquire/present/commit state machine"
        );
        let transaction = source
            .split("fn run_initialization_clear_transaction")
            .nth(1)
            .expect("initialization clear transaction")
            .split("fn ensure_initialization_clear_available")
            .next()
            .expect("bounded initialization clear transaction");
        for phase in ["acquire", "submit_and_present", "commit", "post_present"] {
            assert!(
                transaction.contains(phase),
                "clear transaction must expose {phase} to the cfg(test) driver seam"
            );
        }
        let present = transaction
            .find("driver.submit_and_present(frame)?")
            .expect("present side effect");
        let committed = transaction
            .find("driver.commit(suboptimal)?")
            .expect("single-shot accounting commit");
        let post_present_fault_check = transaction
            .find("driver.post_present()?")
            .expect("post-present fault check");
        assert!(
            present < committed && committed < post_present_fault_check,
            "the irreversible present must be committed before a later fault can permit retry"
        );
        let actual_driver = source
            .split("impl InitializationClearTransactionDriver for GpuContext")
            .nth(1)
            .expect("real GpuContext clear driver")
            .split("fn commit_initialization_clear_present")
            .next()
            .expect("bounded real clear driver");
        assert!(
            actual_driver.contains("self.queue.present(surface_texture);")
                && actual_driver.contains("commit_initialization_clear_present("),
            "the production clear path must bind real presentation and accounting to the transaction"
        );
    }
}
