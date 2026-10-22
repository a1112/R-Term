use std::{cell::RefCell, error::Error, io, sync::Arc};

#[cfg(feature = "diagnostic-tools")]
#[cfg(not(feature = "rterm-legacy-0-1"))]
use std::{collections::HashMap, sync::mpsc};

#[cfg(any(test, feature = "diagnostic-tools"))]
use std::time::Duration;

#[cfg(feature = "diagnostic-tools")]
#[cfg(not(feature = "rterm-legacy-0-1"))]
use std::time::Instant;

#[cfg(feature = "diagnostic-tools")]
#[cfg(not(feature = "rterm-legacy-0-1"))]
use rterm_diagnostics::{
    ConnectionState as DiagnosticConnectionState, DiagnosticAttributionStage,
    MarkerKind as DiagnosticMarkerKind, ProjectOwnedResourceMetricsV1,
    ProjectOwnedResourceSchemaVersion, RendererKind as DiagnosticRendererKind,
};
use rterm_diagnostics::{DiagnosticFontResourceSummary, DiagnosticGpuBackend};
use rterm_render_core::{DamageRegion, RenderGeometry, TerminalRenderSnapshot};
use rterm_render_cpu::TextPaintConfig;
use rterm_render_wgpu::gpu::{
    DEFAULT_GPU_INSTANCE_BYTE_BUDGET, GpuContext, GpuContextError, GpuContextErrorKind,
    GpuContextOptions, GpuFrameStatus, GpuLayerRenderer, GpuPresentationMetrics, GpuTextConfig,
    GpuTextPrepareReport, RenderGraph, WindowedGpuContextBootstrap,
    should_abandon_recovered_window_surface,
};
#[cfg(any(test, feature = "diagnostic-tools"))]
#[cfg(not(feature = "rterm-legacy-0-1"))]
use rterm_render_wgpu::gpu::{GpuInitializationResourceSnapshot, WindowedGpuDevice};
use winit::{dpi::PhysicalSize, event_loop::OwnedDisplayHandle, window::Window};

#[cfg(not(feature = "rterm-legacy-0-1"))]
use crate::platform_fonts::CatalogActivation;
use crate::platform_fonts::{
    FontCatalogMode, PlatformFontRepository, production_font_catalog_mode,
};
#[cfg(feature = "diagnostic-tools")]
#[cfg(not(feature = "rterm-legacy-0-1"))]
use crate::{
    diagnostic_markers::DiagnosticMarkerHandle, stage7_attribution::AttributionStageController,
};
#[cfg(any(test, feature = "diagnostic-tools"))]
#[cfg(not(feature = "rterm-legacy-0-1"))]
use crate::{
    stage7_attribution::{
        AttributionStageRuntime, GpuAttributionStage, ProductServiceEntry,
        ProjectOwnedResourceSnapshot, audit_product_service_start,
    },
    window_bootstrap::WindowBootstrapSurface,
};

/// App-owned direct terminal renderer for the native wgpu surface.
pub(crate) struct WindowGpu {
    context: Option<GpuContext>,
    renderer: Option<GpuLayerRenderer>,
    retired_renderers: Vec<GpuLayerRenderer>,
    font_repository: PlatformFontRepository,
    font_catalog_mode: FontCatalogMode,
    recovery: DeviceRecoveryCoordinator,
    report: Option<GpuTextPrepareReport>,
    rendered_frames: u64,
    replaced_device: bool,
    final_metrics: Option<GpuPresentationMetrics>,
    diagnostic_font_resources: Option<DiagnosticFontResourceSummary>,
    #[cfg(test)]
    abandonment_workaround_adapter_match_override: Option<bool>,
    #[cfg(test)]
    current_adapter_metrics_override: Option<GpuPresentationMetrics>,
    #[cfg(test)]
    abandonment_workaround_os_override: Option<&'static str>,
    #[cfg(debug_assertions)]
    test_device_loss_injected: bool,
}

pub(crate) struct PreparedWindowGpu {
    context: WindowedGpuContextBootstrap,
    diagnostic_font_mode: Option<rterm_diagnostics::DiagnosticFontMode>,
    diagnostic_font_specimen: Option<rterm_diagnostics::DiagnosticFontSpecimen>,
}

#[cfg(any(test, feature = "diagnostic-tools"))]
#[cfg(not(feature = "rterm-legacy-0-1"))]
pub(crate) struct Stage7FullFrameInputs {
    snapshot: TerminalRenderSnapshot,
    geometry: RenderGeometry,
    graph: RenderGraph,
    paint: TextPaintConfig,
}

#[cfg(any(test, feature = "diagnostic-tools"))]
#[cfg(not(feature = "rterm-legacy-0-1"))]
impl Stage7FullFrameInputs {
    fn for_diagnostic_empty_window(spec: Stage7DiagnosticFrameSpec) -> Self {
        #[cfg(test)]
        STAGE7_FULL_FRAME_INPUTS_CREATED.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let terminal = rterm_terminal::Terminal::new(rterm_core::TerminalSize::new(20, 4));
        Self {
            snapshot: TerminalRenderSnapshot::from_terminal(&terminal),
            geometry: RenderGeometry::new(spec.width, spec.height, 16, 24),
            graph: RenderGraph::new(spec.width, spec.height),
            paint: TextPaintConfig::default(),
        }
    }
}

#[cfg(test)]
#[cfg(not(feature = "rterm-legacy-0-1"))]
impl Drop for Stage7FullFrameInputs {
    fn drop(&mut self) {
        STAGE7_FULL_FRAME_INPUTS_DROPPED.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
#[cfg(not(feature = "rterm-legacy-0-1"))]
static STAGE7_FULL_FRAME_INPUTS_CREATED: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
#[cfg(test)]
#[cfg(not(feature = "rterm-legacy-0-1"))]
static STAGE7_FULL_FRAME_INPUTS_DROPPED: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(any(test, feature = "diagnostic-tools"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(not(feature = "rterm-legacy-0-1"))]
pub(crate) struct Stage7DiagnosticFrameSpec {
    width: u32,
    height: u32,
}

#[cfg(any(test, feature = "diagnostic-tools"))]
#[cfg(not(feature = "rterm-legacy-0-1"))]
impl Stage7DiagnosticFrameSpec {
    const fn for_surface(size: PhysicalSize<u32>) -> Self {
        Self {
            width: size.width,
            height: size.height,
        }
    }
}

/// Real owner composition used by the private Stage 7 stop-stage diagnostic.
/// No WGPU object is created until the controller enters `InstanceSurface`.
#[allow(
    dead_code,
    reason = "Task 6 wires the private diagnostic launcher to this Task 5 owner"
)]
#[cfg(any(test, feature = "diagnostic-tools"))]
#[cfg(not(feature = "rterm-legacy-0-1"))]
pub(crate) struct Stage7WindowAttributionRuntime<'a> {
    cpu_surface: &'a mut WindowBootstrapSurface,
    cpu_rgba: Vec<u8>,
    display: OwnedDisplayHandle,
    window: Arc<Window>,
    surface_size: PhysicalSize<u32>,
    high_performance: bool,
    force_fallback_adapter: bool,
    diagnostic_backend: Option<DiagnosticGpuBackend>,
    full_frame_spec: Stage7DiagnosticFrameSpec,
    full_frame_inputs: Option<Stage7FullFrameInputs>,
    prepared: Option<WindowedGpuContextBootstrap>,
    device: Option<WindowedGpuDevice>,
    context: Option<GpuContext>,
    renderer: Option<GpuLayerRenderer>,
    font_repository: Option<PlatformFontRepository>,
    gpu_identity: Option<GpuPresentationMetrics>,
    resources: ProjectOwnedResourceSnapshot,
    #[cfg(feature = "diagnostic-tools")]
    diagnostic_hold: Option<Stage7DiagnosticHold>,
    #[cfg(feature = "diagnostic-tools")]
    diagnostic_hold_error: Option<String>,
}

#[cfg(feature = "diagnostic-tools")]
#[cfg(not(feature = "rterm-legacy-0-1"))]
struct Stage7DiagnosticHold {
    requested_stage: DiagnosticAttributionStage,
    duration: Duration,
    markers: DiagnosticMarkerHandle,
    shutdown: mpsc::Receiver<Result<(), String>>,
}

#[allow(
    dead_code,
    reason = "Task 6 wires the private diagnostic launcher to this Task 5 owner"
)]
#[cfg(any(test, feature = "diagnostic-tools"))]
#[cfg(not(feature = "rterm-legacy-0-1"))]
impl<'a> Stage7WindowAttributionRuntime<'a> {
    #[expect(
        clippy::too_many_arguments,
        reason = "the private owner binds one real CPU surface, GPU surface, and diagnostic frame"
    )]
    pub(crate) fn new(
        cpu_surface: &'a mut WindowBootstrapSurface,
        cpu_rgba: Vec<u8>,
        display: OwnedDisplayHandle,
        window: Arc<Window>,
        surface_size: PhysicalSize<u32>,
        high_performance: bool,
        force_fallback_adapter: bool,
        diagnostic_backend: Option<DiagnosticGpuBackend>,
    ) -> Self {
        Self {
            cpu_surface,
            cpu_rgba,
            display,
            window,
            surface_size,
            high_performance,
            force_fallback_adapter,
            diagnostic_backend,
            full_frame_spec: Stage7DiagnosticFrameSpec::for_surface(surface_size),
            full_frame_inputs: None,
            prepared: None,
            device: None,
            context: None,
            renderer: None,
            font_repository: None,
            gpu_identity: None,
            resources: ProjectOwnedResourceSnapshot::default(),
            #[cfg(feature = "diagnostic-tools")]
            diagnostic_hold: None,
            #[cfg(feature = "diagnostic-tools")]
            diagnostic_hold_error: None,
        }
    }

    #[cfg(feature = "diagnostic-tools")]
    fn with_diagnostic_hold(mut self, hold: Stage7DiagnosticHold) -> Self {
        self.diagnostic_hold = Some(hold);
        self
    }

    #[cfg(feature = "diagnostic-tools")]
    fn take_diagnostic_hold_error(&mut self) -> Option<String> {
        self.diagnostic_hold_error.take()
    }

    #[cfg(feature = "diagnostic-tools")]
    fn shutdown_after_native_window_close(&mut self) -> bool {
        let (backend, vendor_id) = self.gpu_identity.as_ref().map_or(("", 0), |identity| {
            (identity.backend.as_str(), identity.adapter_vendor_id)
        });
        finalize_stage7_native_gpu_owners(
            std::env::consts::OS,
            backend,
            vendor_id,
            &mut self.renderer,
            &mut self.device,
            &mut self.context,
        )
    }

    #[cfg(feature = "diagnostic-tools")]
    fn hold_diagnostic(&mut self) {
        let Some(hold) = self.diagnostic_hold.take() else {
            return;
        };
        let result = (|| -> Result<(), Box<dyn Error>> {
            let summary = diagnostic_resource_summary(&self.resources)?;
            summary
                .validate_at(hold.requested_stage)
                .map_err(|violations| {
                    io::Error::other(format!(
                        "attribution resource summary failed at {}: {}",
                        hold.requested_stage,
                        violations.join("; ")
                    ))
                })?;
            let renderer =
                if hold.requested_stage >= DiagnosticAttributionStage::ConfiguredSurfaceClear {
                    DiagnosticRendererKind::Gpu
                } else {
                    DiagnosticRendererKind::Cpu
                };
            let mut extra = HashMap::from([
                (
                    "requested_stage".to_owned(),
                    serde_json::to_value(hold.requested_stage)?,
                ),
                (
                    "final_stage".to_owned(),
                    serde_json::to_value(hold.requested_stage)?,
                ),
                (
                    "resource_summary_schema".to_owned(),
                    serde_json::to_value(ProjectOwnedResourceSchemaVersion::V1)?,
                ),
                (
                    "resource_summary".to_owned(),
                    serde_json::to_value(&summary)?,
                ),
            ]);
            if hold.requested_stage >= DiagnosticAttributionStage::AdapterDevice {
                let identity = self.gpu_identity.as_ref().ok_or_else(|| {
                    io::Error::other("adapter identity owner is unavailable at adapter-device")
                })?;
                extra.extend(stage7_attribution_identity_extra(identity));
            }
            if !hold.markers.emit_with_extra(
                DiagnosticMarkerKind::AttributionStageReady,
                Some(renderer),
                Some(DiagnosticConnectionState::NotStarted),
                extra,
            )? {
                return Err(
                    io::Error::other("attribution_stage_ready marker was not unique").into(),
                );
            }

            let deadline = Instant::now()
                .checked_add(hold.duration)
                .ok_or_else(|| io::Error::other("attribution hold deadline overflow"))?;
            loop {
                let now = Instant::now();
                if now >= deadline {
                    break;
                }
                let remaining = deadline.saturating_duration_since(now);
                match hold
                    .shutdown
                    .recv_timeout(remaining.min(Duration::from_millis(100)))
                {
                    Ok(Ok(())) => break,
                    Ok(Err(message)) => return Err(io::Error::other(message).into()),
                    Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {
                        // EOF is not an implicit shutdown. Keep the owner live for the
                        // bounded absolute hold so an absent launcher cannot shorten it.
                    }
                }
            }
            Self::audit_disabled_product_service_entries()?;
            Ok(())
        })();
        if let Err(error) = result {
            self.diagnostic_hold_error = Some(error.to_string());
        }
    }

    fn audit_disabled_product_service_entries() -> Result<(), Box<dyn Error>> {
        let attempts = [
            audit_product_service_start(ProductServiceEntry::DeferredConfig),
            audit_product_service_start(ProductServiceEntry::ConfigWatcher),
            audit_product_service_start(ProductServiceEntry::LocalPty),
            audit_product_service_start(ProductServiceEntry::NativeSsh),
            audit_product_service_start(ProductServiceEntry::PostReadyCoordinator),
        ];
        if attempts.iter().any(Result::is_ok) {
            return Err(
                io::Error::other("a Stage 7 product service entry remained enabled").into(),
            );
        }
        Ok(())
    }

    fn complete_cpu_window(&mut self) -> Result<(), Box<dyn Error>> {
        self.cpu_surface.present_rgba(&self.cpu_rgba)?;
        let size = self.cpu_surface.size();
        let bytes = usize::try_from(size.width)
            .ok()
            .and_then(|width| {
                usize::try_from(size.height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| io::Error::other("CPU bootstrap byte size overflow"))?;
        if self.cpu_rgba.len() != bytes {
            return Err(io::Error::other("CPU bootstrap owner retained a mismatched frame").into());
        }
        self.resources.cpu_staging_bytes = u64::try_from(self.cpu_rgba.capacity())
            .map_err(|_| io::Error::other("CPU bootstrap retained capacity exceeds u64"))?;
        self.resources.cpu_surface_count = 1;
        self.resources.cpu_present_count = 1;
        #[cfg(feature = "diagnostic-tools")]
        if let Some(hold) = self.diagnostic_hold.as_ref()
            && !hold.markers.emit(
                DiagnosticMarkerKind::FirstPresent,
                Some(DiagnosticRendererKind::Cpu),
                Some(DiagnosticConnectionState::NotStarted),
            )?
        {
            return Err(io::Error::other("first_present marker was not unique").into());
        }
        Ok(())
    }

    fn complete_instance_surface(&mut self) -> Result<(), Box<dyn Error>> {
        let options = gpu_context_options(
            self.high_performance,
            self.force_fallback_adapter,
            self.diagnostic_backend,
        )?;
        let prepared = GpuContext::prepare_windowed(
            self.display.clone(),
            Arc::clone(&self.window),
            self.surface_size.width,
            self.surface_size.height,
            options,
        )?;
        merge_gpu_resources(&mut self.resources, &prepared.initialization_resources());
        self.prepared = Some(prepared);
        Ok(())
    }

    fn complete_adapter_device(&mut self) -> Result<(), Box<dyn Error>> {
        let prepared = self
            .prepared
            .take()
            .ok_or_else(|| io::Error::other("InstanceSurface owner is unavailable"))?;
        let device = pollster::block_on(prepared.select_device())?;
        merge_gpu_resources(&mut self.resources, device.initialization_resources());
        self.gpu_identity = Some(device.metrics().clone());
        self.device = Some(device);
        Ok(())
    }

    fn complete_configured_surface_clear(&mut self) -> Result<(), Box<dyn Error>> {
        let device = self
            .device
            .take()
            .ok_or_else(|| io::Error::other("AdapterDevice owner is unavailable"))?;
        let mut context = device.configure_surface()?;
        if context.present_clear_once()? != GpuFrameStatus::Presented {
            return Err(
                io::Error::other("configured Stage 7 clear frame was not presented").into(),
            );
        }
        merge_gpu_resources(&mut self.resources, context.initialization_resources());
        self.context = Some(context);
        Ok(())
    }

    fn complete_layer_pipelines(&mut self) -> Result<(), Box<dyn Error>> {
        let context = self
            .context
            .as_ref()
            .ok_or_else(|| io::Error::other("ConfiguredSurfaceClear owner is unavailable"))?;
        let format = context
            .surface_format()
            .ok_or_else(|| io::Error::other("configured attribution surface has no format"))?;
        let renderer = GpuLayerRenderer::new(context, format, DEFAULT_GPU_INSTANCE_BYTE_BUDGET)?;
        self.renderer = Some(renderer);
        self.sync_live_gpu_resources();
        Ok(())
    }

    fn complete_fixture_font_text(&mut self) -> Result<(), Box<dyn Error>> {
        let catalog = stage7_fixture_font_catalog()?;
        let metrics = catalog.memory_metrics();
        let catalog_generation = catalog.generation();
        let mut terminal = rterm_terminal::Terminal::new(rterm_core::TerminalSize::new(20, 1));
        terminal.feed(b"R-Term Stage 7");
        let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
        let geometry =
            RenderGeometry::new(self.surface_size.width, self.surface_size.height, 16, 24);
        let renderer = self
            .renderer
            .as_mut()
            .ok_or_else(|| io::Error::other("LayerPipelines owner is unavailable"))?;
        renderer.enable_text(catalog, stage7_fixture_font_config(), stage7_text_config())?;
        renderer.prepare_text(
            &snapshot,
            geometry,
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )?;
        let graph = RenderGraph::new(self.surface_size.width, self.surface_size.height);
        let context = self
            .context
            .as_mut()
            .ok_or_else(|| io::Error::other("ConfiguredSurfaceClear owner is unavailable"))?;
        if context.render_graph(renderer, &graph, || {})? != GpuFrameStatus::Presented {
            return Err(io::Error::other("fixture GPU text frame was not presented").into());
        }
        self.resources.retained_font_bytes =
            project_owned_u64(metrics.retained_source_bytes, "fixture retained font bytes")?;
        self.resources.active_font_count =
            project_owned_u64(metrics.active_source_count, "fixture active font count")?;
        self.resources.catalog_builds = metrics.catalog_builds;
        self.resources.catalog_generation = catalog_generation;
        self.sync_live_gpu_resources();
        Ok(())
    }

    fn complete_platform_font_index(&mut self) -> Result<(), Box<dyn Error>> {
        let repository = PlatformFontRepository::production_index();
        let diagnostics = repository.diagnostics()?;
        self.resources.indexed_font_count =
            project_owned_u64(diagnostics.indexed_source_count, "indexed font count")?;
        self.resources.inactive_font_bytes =
            project_owned_u64(diagnostics.retained_source_bytes, "inactive font bytes")?;
        self.font_repository = Some(repository);
        Ok(())
    }

    fn complete_full_frame(&mut self) -> Result<(), Box<dyn Error>> {
        let inputs = Stage7FullFrameInputs::for_diagnostic_empty_window(self.full_frame_spec);
        let repository = self
            .font_repository
            .as_mut()
            .ok_or_else(|| io::Error::other("PlatformFontIndex owner is unavailable"))?;
        let catalog = repository.build_catalog(FontCatalogMode::Lazy)?;
        let renderer = self
            .renderer
            .as_mut()
            .ok_or_else(|| io::Error::other("FixtureFontText owner is unavailable"))?;
        renderer.enable_text(
            catalog,
            bundled_emergency_font_config(),
            stage7_text_config(),
        )?;
        prepare_gpu_text_frame(
            repository,
            renderer,
            &inputs.snapshot,
            inputs.geometry,
            &[],
            &inputs.paint,
            1.0,
            1.0,
        )?;
        let context = self
            .context
            .as_mut()
            .ok_or_else(|| io::Error::other("ConfiguredSurfaceClear owner is unavailable"))?;
        if context.render_graph(renderer, &inputs.graph, || {})? != GpuFrameStatus::Presented {
            return Err(io::Error::other("full Stage 7 GPU frame was not presented").into());
        }
        let diagnostics = repository.diagnostics()?;
        self.resources.retained_font_bytes =
            project_owned_u64(diagnostics.retained_source_bytes, "retained font bytes")?;
        self.resources.indexed_font_count =
            project_owned_u64(diagnostics.indexed_source_count, "indexed font count")?;
        self.resources.active_font_count =
            project_owned_u64(diagnostics.active_source_count, "active font count")?;
        self.resources.catalog_builds = diagnostics.catalog_builds;
        self.resources.catalog_generation = diagnostics.generation;
        self.resources.snapshot_bytes = u64::try_from(
            inputs
                .snapshot
                .project_owned_logical_bytes_v1()
                .map_err(|error| io::Error::other(error.to_string()))?,
        )
        .map_err(|_| io::Error::other("snapshot logical bytes exceed u64"))?;
        self.sync_live_gpu_resources();
        self.full_frame_inputs = Some(inputs);
        Ok(())
    }

    fn sync_live_gpu_resources(&mut self) {
        let mut current = GpuInitializationResourceSnapshot::default();
        if let Some(context) = self.context.as_ref() {
            current = current.merged(context.initialization_resources());
        }
        if let Some(renderer) = self.renderer.as_ref() {
            current = current.merged(&renderer.initialization_resources());
        }
        merge_gpu_resources(&mut self.resources, &current);
    }
}

#[cfg(feature = "diagnostic-tools")]
#[cfg(not(feature = "rterm-legacy-0-1"))]
fn stage7_attribution_identity_extra(
    metrics: &GpuPresentationMetrics,
) -> HashMap<String, serde_json::Value> {
    HashMap::from([
        (
            "gpu_adapter_vendor_id".to_owned(),
            serde_json::json!(metrics.adapter_vendor_id),
        ),
        (
            "gpu_adapter_device_id".to_owned(),
            serde_json::json!(metrics.adapter_device_id),
        ),
        (
            "gpu_adapter_type".to_owned(),
            serde_json::json!(metrics.adapter_type),
        ),
    ])
}

#[cfg(any(test, feature = "diagnostic-tools"))]
#[cfg(not(feature = "rterm-legacy-0-1"))]
impl AttributionStageRuntime for Stage7WindowAttributionRuntime<'_> {
    type Error = Box<dyn Error>;

    fn disable_product_services(&mut self) {
        // The controller owns the shared RAII guard before completing a stage.
    }

    fn complete_stage(
        &mut self,
        stage: GpuAttributionStage,
    ) -> Result<ProjectOwnedResourceSnapshot, Self::Error> {
        Self::audit_disabled_product_service_entries()?;
        match stage {
            GpuAttributionStage::CpuWindow => self.complete_cpu_window()?,
            GpuAttributionStage::InstanceSurface => self.complete_instance_surface()?,
            GpuAttributionStage::AdapterDevice => self.complete_adapter_device()?,
            GpuAttributionStage::ConfiguredSurfaceClear => {
                self.complete_configured_surface_clear()?;
            }
            GpuAttributionStage::LayerPipelines => self.complete_layer_pipelines()?,
            GpuAttributionStage::FixtureFontText => self.complete_fixture_font_text()?,
            GpuAttributionStage::PlatformFontIndex => self.complete_platform_font_index()?,
            GpuAttributionStage::FullFrame => self.complete_full_frame()?,
        }
        Self::audit_disabled_product_service_entries()?;
        Ok(self.resources.clone())
    }

    fn hold(&mut self, duration: Duration) {
        #[cfg(feature = "diagnostic-tools")]
        if self.diagnostic_hold.is_some() {
            self.hold_diagnostic();
            return;
        }
        std::thread::sleep(duration);
        Self::audit_disabled_product_service_entries()
            .expect("product services remain disabled after the attribution hold");
    }
}

#[cfg(not(feature = "rterm-legacy-0-1"))]
#[cfg(any(test, feature = "diagnostic-tools"))]
fn stage7_fixture_font_catalog() -> Result<rterm_fonts::FontCatalog, Box<dyn Error>> {
    let source = rterm_fonts::FontSource::new(
        "R-Term Stage 7 Fixture",
        include_bytes!("../../../tests/fixtures/fonts/NotoSans-Latin.fixture.ttf").to_vec(),
    );
    Ok(rterm_fonts::FontCatalog::from_sources("en-US", [source])?)
}

#[cfg(not(feature = "rterm-legacy-0-1"))]
#[cfg(any(test, feature = "diagnostic-tools"))]
fn stage7_fixture_font_config() -> rterm_fonts::FontConfig {
    rterm_fonts::FontConfig::new("R-Term Stage 7 Fixture")
}

#[cfg(not(feature = "rterm-legacy-0-1"))]
#[cfg(any(test, feature = "diagnostic-tools"))]
fn stage7_text_config() -> GpuTextConfig {
    use rterm_fonts::RasterCacheConfig;

    GpuTextConfig::new(4 * 1024 * 1024, RasterCacheConfig::new(4 * 1024 * 1024))
}

#[cfg(any(test, feature = "diagnostic-tools"))]
#[cfg(not(feature = "rterm-legacy-0-1"))]
fn project_owned_u64(value: usize, label: &str) -> Result<u64, Box<dyn Error>> {
    u64::try_from(value).map_err(|_| io::Error::other(format!("{label} exceeds u64")).into())
}

#[cfg(feature = "diagnostic-tools")]
#[cfg(not(feature = "rterm-legacy-0-1"))]
fn diagnostic_resource_summary(
    source: &ProjectOwnedResourceSnapshot,
) -> Result<ProjectOwnedResourceMetricsV1, Box<dyn Error>> {
    let backend = source
        .backend
        .as_deref()
        .map(str::to_ascii_lowercase)
        .map(|backend| backend.parse::<DiagnosticGpuBackend>())
        .transpose()
        .map_err(io::Error::other)?;
    Ok(ProjectOwnedResourceMetricsV1 {
        cpu_staging_bytes: source.cpu_staging_bytes,
        cpu_surface_count: source.cpu_surface_count,
        cpu_present_count: source.cpu_present_count,
        instance_count: source.instance_count,
        surface_count: source.surface_count,
        adapter_count: source.adapter_count,
        device_count: source.device_count,
        queue_count: source.queue_count,
        surface_configure_count: source.surface_configure_count,
        surface_acquire_count: source.surface_acquire_count,
        clear_present_count: source.clear_present_count,
        pipeline_count: source.pipeline_count,
        pipeline_layout_count: source.pipeline_layout_count,
        materialized_buffer_count: source.materialized_buffer_count,
        retained_font_bytes: source.retained_font_bytes,
        inactive_font_bytes: source.inactive_font_bytes,
        indexed_font_count: source.indexed_font_count,
        active_font_count: source.active_font_count,
        catalog_builds: source.catalog_builds,
        catalog_generation: source.catalog_generation,
        glyph_atlas_bytes: source.glyph_atlas_bytes,
        raster_cache_bytes: source.raster_cache_bytes,
        image_texture_bytes: source.image_texture_bytes,
        snapshot_bytes: source.snapshot_bytes,
        instance_buffer_bytes: source.instance_buffer_bytes,
        upload_buffer_bytes: source.upload_buffer_bytes,
        total_allocated_buffer_bytes: source.total_allocated_buffer_bytes,
        total_allocated_texture_bytes: source.total_allocated_texture_bytes,
        base_text_renderer_materialization_count: source.base_text_renderer_materialization_count,
        cursor_text_renderer_materialization_count: source
            .cursor_text_renderer_materialization_count,
        config_load_count: source.config_load_count,
        config_watcher_count: source.config_watcher_count,
        pty_start_count: source.pty_start_count,
        ssh_start_count: source.ssh_start_count,
        post_ready_task_count: source.post_ready_task_count,
        backend,
        adapter_name: source.adapter_name.clone(),
    })
}

#[cfg(any(test, feature = "diagnostic-tools"))]
#[cfg(not(feature = "rterm-legacy-0-1"))]
fn merge_gpu_resources(
    destination: &mut ProjectOwnedResourceSnapshot,
    source: &GpuInitializationResourceSnapshot,
) {
    destination.instance_count = source.instance_count;
    destination.surface_count = source.surface_count;
    destination.adapter_count = source.adapter_count;
    destination.device_count = source.device_count;
    destination.queue_count = source.queue_count;
    destination.surface_configure_count = source.surface_configure_count;
    destination.surface_acquire_count = source.surface_acquire_count;
    destination.clear_present_count = source.clear_present_count;
    destination.pipeline_count = source.pipeline_count;
    destination.pipeline_layout_count = source.pipeline_layout_count;
    destination.materialized_buffer_count = source.materialized_buffer_count;
    destination.instance_buffer_bytes = source.instance_buffer_bytes;
    destination.upload_buffer_bytes = source.upload_buffer_bytes;
    destination.total_allocated_buffer_bytes = source.total_allocated_buffer_bytes;
    destination.total_allocated_texture_bytes = source.total_allocated_texture_bytes;
    destination.glyph_atlas_bytes = source.glyph_atlas_bytes;
    destination.raster_cache_bytes = source.raster_cache_bytes;
    destination.image_texture_bytes = source.image_texture_bytes;
    destination.base_text_renderer_materialization_count =
        source.base_text_renderer_materialization_count;
    destination.cursor_text_renderer_materialization_count =
        source.cursor_text_renderer_materialization_count;
    destination.backend.clone_from(&source.backend);
    destination.adapter_name.clone_from(&source.adapter_name);
}

#[cfg(feature = "diagnostic-tools")]
#[cfg(not(feature = "rterm-legacy-0-1"))]
const fn diagnostic_attribution_stage(stage: DiagnosticAttributionStage) -> GpuAttributionStage {
    match stage {
        DiagnosticAttributionStage::CpuWindow => GpuAttributionStage::CpuWindow,
        DiagnosticAttributionStage::InstanceSurface => GpuAttributionStage::InstanceSurface,
        DiagnosticAttributionStage::AdapterDevice => GpuAttributionStage::AdapterDevice,
        DiagnosticAttributionStage::ConfiguredSurfaceClear => {
            GpuAttributionStage::ConfiguredSurfaceClear
        }
        DiagnosticAttributionStage::LayerPipelines => GpuAttributionStage::LayerPipelines,
        DiagnosticAttributionStage::FixtureFontText => GpuAttributionStage::FixtureFontText,
        DiagnosticAttributionStage::PlatformFontIndex => GpuAttributionStage::PlatformFontIndex,
        DiagnosticAttributionStage::FullFrame => GpuAttributionStage::FullFrame,
    }
}

/// Runs the private Stage 7 attribution owner without constructing the normal
/// window manager or scheduling any product service.
#[cfg(feature = "diagnostic-tools")]
#[cfg(not(feature = "rterm-legacy-0-1"))]
#[expect(
    clippy::too_many_lines,
    reason = "the private native owner keeps window, CPU surface, GPU stages, and teardown in one auditable scope"
)]
pub(crate) fn run_stage7_native_attribution(
    requested_stage: DiagnosticAttributionStage,
    diagnostic_backend: Option<DiagnosticGpuBackend>,
    hold_duration: Duration,
    markers: DiagnosticMarkerHandle,
    shutdown: mpsc::Receiver<Result<(), String>>,
) -> Result<(), Box<dyn Error>> {
    use winit::{
        application::ApplicationHandler,
        event::WindowEvent,
        event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
        window::{WindowAttributes, WindowId},
    };

    struct NativeAttributionApp {
        requested_stage: DiagnosticAttributionStage,
        stop_stage: GpuAttributionStage,
        diagnostic_backend: Option<DiagnosticGpuBackend>,
        hold_duration: Duration,
        markers: DiagnosticMarkerHandle,
        shutdown: Option<mpsc::Receiver<Result<(), String>>>,
        result: Option<Result<(), String>>,
    }

    impl ApplicationHandler for NativeAttributionApp {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if self.result.is_some() {
                event_loop.exit();
                return;
            }
            let result = (|| -> Result<(), Box<dyn Error>> {
                let size = PhysicalSize::new(320, 96);
                let window = Arc::new(
                    event_loop.create_window(
                        WindowAttributes::default()
                            .with_title("R-Term Stage 7 attribution")
                            .with_visible(false)
                            .with_inner_size(size),
                    )?,
                );
                let actual_size = window.request_inner_size(size).unwrap_or(size);
                if !self.markers.emit(
                    DiagnosticMarkerKind::WindowCreated,
                    None,
                    Some(DiagnosticConnectionState::NotStarted),
                )? {
                    return Err(io::Error::other("window_created marker was not unique").into());
                }
                let frame_len = usize::try_from(actual_size.width)
                    .ok()
                    .and_then(|width| {
                        usize::try_from(actual_size.height)
                            .ok()
                            .and_then(|height| width.checked_mul(height))
                    })
                    .and_then(|pixels| pixels.checked_mul(4))
                    .ok_or_else(|| io::Error::other("native attribution frame overflow"))?;
                let cpu_rgba = {
                    let mut terminal =
                        rterm_terminal::Terminal::new(rterm_core::TerminalSize::new(1, 1));
                    terminal.feed(b"R");
                    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
                    let mut rgba = vec![0_u8; frame_len];
                    rterm_render_cpu::PixelRenderer::new().render(
                        &snapshot,
                        &mut rgba,
                        actual_size.width,
                        actual_size.height,
                        16,
                        24,
                    );
                    rgba
                };
                let mut cpu_surface =
                    WindowBootstrapSurface::new(event_loop, Arc::clone(&window), actual_size)?;
                let shutdown = self.shutdown.take().ok_or_else(|| {
                    io::Error::other("attribution shutdown receiver was already consumed")
                })?;
                let mut runtime = Stage7WindowAttributionRuntime::new(
                    &mut cpu_surface,
                    cpu_rgba,
                    event_loop.owned_display_handle(),
                    window,
                    actual_size,
                    false,
                    false,
                    self.diagnostic_backend,
                )
                .with_diagnostic_hold(Stage7DiagnosticHold {
                    requested_stage: self.requested_stage,
                    duration: self.hold_duration,
                    markers: self.markers.clone(),
                    shutdown,
                });
                let report_result = AttributionStageController::new(self.stop_stage)
                    .run(&mut runtime)
                    .map_err(|error| io::Error::other(error.to_string()));
                let hold_error = runtime.take_diagnostic_hold_error();
                runtime.shutdown_after_native_window_close();
                let report = report_result?;
                if report.held_stage != self.stop_stage {
                    return Err(
                        io::Error::other("attribution controller held the wrong stage").into(),
                    );
                }
                if let Some(error) = hold_error {
                    return Err(io::Error::other(error).into());
                }
                Ok(())
            })();
            self.result = Some(result.map_err(|error| error.to_string()));
            event_loop.exit();
        }

        fn window_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _window_id: WindowId,
            _event: WindowEvent,
        ) {
        }
    }

    let event_loop = EventLoop::builder()
        .build()
        .map_err(|error| io::Error::other(format!("create attribution event loop: {error}")))?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = NativeAttributionApp {
        requested_stage,
        stop_stage: diagnostic_attribution_stage(requested_stage),
        diagnostic_backend,
        hold_duration,
        markers,
        shutdown: Some(shutdown),
        result: None,
    };
    event_loop
        .run_app(&mut app)
        .map_err(|error| io::Error::other(format!("run attribution event loop: {error}")))?;
    app.result
        .take()
        .ok_or_else(|| io::Error::other("attribution event loop returned no result"))?
        .map_err(|error| io::Error::other(error).into())
}

#[cfg(all(feature = "diagnostic-tools", feature = "rterm-legacy-0-1"))]
pub(crate) fn run_stage7_native_attribution(
    _requested_stage: rterm_diagnostics::DiagnosticAttributionStage,
    _diagnostic_backend: Option<DiagnosticGpuBackend>,
    _hold_duration: Duration,
    _markers: crate::diagnostic_markers::DiagnosticMarkerHandle,
    _shutdown: std::sync::mpsc::Receiver<Result<(), String>>,
) -> Result<(), Box<dyn Error>> {
    Err(crate::rterm_compat::unsupported_diagnostics())
}

#[cfg(all(test, target_os = "windows"))]
#[cfg(not(feature = "rterm-legacy-0-1"))]
#[expect(
    clippy::too_many_lines,
    reason = "one native event-loop fixture keeps the real eight-stage owner and production compatibility teardown auditable"
)]
pub(crate) fn run_stage7_native_attribution_for_test(
    stop_stage: GpuAttributionStage,
) -> Result<ProjectOwnedResourceSnapshot, Box<dyn Error>> {
    use winit::{
        application::ApplicationHandler,
        event::WindowEvent,
        event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
        platform::windows::EventLoopBuilderExtWindows,
        window::{WindowAttributes, WindowId},
    };

    use crate::stage7_attribution::AttributionStageController;

    struct NativeAttributionApp {
        stop_stage: GpuAttributionStage,
        result: Option<Result<ProjectOwnedResourceSnapshot, String>>,
    }

    impl ApplicationHandler for NativeAttributionApp {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            let result = (|| -> Result<ProjectOwnedResourceSnapshot, Box<dyn Error>> {
                let size = PhysicalSize::new(320, 96);
                let attributes = WindowAttributes::default()
                    .with_title("R-Term Stage 7 attribution test")
                    .with_visible(false)
                    .with_inner_size(size);
                let window = Arc::new(event_loop.create_window(attributes)?);
                let actual_size = window.request_inner_size(size).unwrap_or(size);
                let frame_len = usize::try_from(actual_size.width)
                    .ok()
                    .and_then(|width| {
                        usize::try_from(actual_size.height)
                            .ok()
                            .and_then(|height| width.checked_mul(height))
                    })
                    .and_then(|pixels| pixels.checked_mul(4))
                    .ok_or_else(|| io::Error::other("native attribution frame overflow"))?;
                let cpu_rgba = {
                    let mut terminal =
                        rterm_terminal::Terminal::new(rterm_core::TerminalSize::new(1, 1));
                    terminal.feed(b"R");
                    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
                    let mut rgba = vec![0_u8; frame_len];
                    rterm_render_cpu::PixelRenderer::new().render(
                        &snapshot,
                        &mut rgba,
                        actual_size.width,
                        actual_size.height,
                        16,
                        24,
                    );
                    rgba
                };
                let mut cpu_surface =
                    WindowBootstrapSurface::new(event_loop, Arc::clone(&window), actual_size)?;
                STAGE7_FULL_FRAME_INPUTS_CREATED.store(0, std::sync::atomic::Ordering::SeqCst);
                STAGE7_FULL_FRAME_INPUTS_DROPPED.store(0, std::sync::atomic::Ordering::SeqCst);
                let mut runtime = Stage7WindowAttributionRuntime::new(
                    &mut cpu_surface,
                    cpu_rgba,
                    event_loop.owned_display_handle(),
                    window,
                    actual_size,
                    false,
                    false,
                    None,
                );
                assert_eq!(
                    STAGE7_FULL_FRAME_INPUTS_CREATED.load(std::sync::atomic::Ordering::SeqCst),
                    0,
                    "no FullFrame owner may exist before the controller reaches FullFrame"
                );
                let report = AttributionStageController::new(self.stop_stage)
                    .run(&mut runtime)
                    .map_err(|error| io::Error::other(error.to_string()))?;
                let resources = report.resources;
                let expected_full_frame_owners =
                    usize::from(self.stop_stage == GpuAttributionStage::FullFrame);
                assert_eq!(
                    STAGE7_FULL_FRAME_INPUTS_CREATED.load(std::sync::atomic::Ordering::SeqCst),
                    expected_full_frame_owners,
                    "the FullFrame input owner is created only at its stage"
                );
                assert_eq!(
                    STAGE7_FULL_FRAME_INPUTS_DROPPED.load(std::sync::atomic::Ordering::SeqCst),
                    0,
                    "the FullFrame owner must remain live throughout the controller hold"
                );
                drop(runtime);
                assert_eq!(
                    STAGE7_FULL_FRAME_INPUTS_DROPPED.load(std::sync::atomic::Ordering::SeqCst),
                    expected_full_frame_owners,
                    "dropping the runtime must release the retained FullFrame owner"
                );
                drop(cpu_surface);

                if self.stop_stage == GpuAttributionStage::FullFrame {
                    let production_window = Arc::new(
                        event_loop.create_window(
                            WindowAttributes::default()
                                .with_title("R-Term production composition test")
                                .with_visible(false)
                                .with_inner_size(actual_size),
                        )?,
                    );
                    let mut production = pollster::block_on(WindowGpu::new(
                        event_loop.owned_display_handle(),
                        Arc::clone(&production_window),
                        actual_size,
                        false,
                        false,
                    ))?;
                    if production.metrics().presented_frames != 0
                        || production.metrics().surface_reconfigurations != 1
                    {
                        return Err(io::Error::other(
                            "production one-call composition changed its pre-present boundary",
                        )
                        .into());
                    }
                    let terminal =
                        rterm_terminal::Terminal::new(rterm_core::TerminalSize::new(20, 4));
                    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
                    let status = production.present(
                        production_window.as_ref(),
                        &snapshot,
                        RenderGeometry::new(actual_size.width, actual_size.height, 16, 24),
                        &[],
                        &TextPaintConfig::default(),
                        &RenderGraph::new(actual_size.width, actual_size.height),
                        rterm_native::RenderMode::Full,
                        1.0,
                    )?;
                    if status != GpuFrameStatus::Presented
                        || production.metrics().presented_frames != 1
                    {
                        return Err(io::Error::other(
                            "production one-call composition did not present one normal frame",
                        )
                        .into());
                    }
                    production.shutdown_after_native_window_close();
                    drop(production);
                }
                Ok(resources)
            })();
            self.result = Some(result.map_err(|error| error.to_string()));
            event_loop.exit();
        }

        fn window_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _window_id: WindowId,
            _event: WindowEvent,
        ) {
        }
    }

    let event_loop = EventLoop::builder()
        .with_any_thread(true)
        .build()
        .map_err(|error| {
            io::Error::other(format!("create native attribution event loop: {error}"))
        })?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = NativeAttributionApp {
        stop_stage,
        result: None,
    };
    event_loop
        .run_app(&mut app)
        .map_err(|error| io::Error::other(format!("run native attribution event loop: {error}")))?;
    app.result
        .take()
        .ok_or_else(|| io::Error::other("native attribution event loop returned no result"))?
        .map_err(|error| io::Error::other(error).into())
}

struct WindowGpuFrame<'a> {
    window: &'a Window,
    snapshot: &'a TerminalRenderSnapshot,
    geometry: RenderGeometry,
    damage: &'a [DamageRegion],
    paint: &'a TextPaintConfig,
    graph: &'a RenderGraph,
    render_mode: rterm_native::RenderMode,
    dpi_scale: f32,
}

#[cfg(any(test, not(feature = "rterm-legacy-0-1")))]
pub(crate) enum CatalogFrameAttempt<T> {
    Prepared(T),
    Expanded(u64),
}

fn font_fallback_redraw_needed(
    repository: &PlatformFontRepository,
    report: &GpuTextPrepareReport,
) -> bool {
    repository.has_pending_fallbacks(&report.missing_glyphs)
}

#[cfg(any(test, not(feature = "rterm-legacy-0-1")))]
pub(crate) fn prepare_catalog_frame_with_one_restart<T>(
    mut expected_generation: u64,
    damage: &[DamageRegion],
    mut prepare: impl FnMut(
        u64,
        &[DamageRegion],
        bool,
    ) -> Result<CatalogFrameAttempt<T>, Box<dyn Error>>,
) -> Result<T, Box<dyn Error>> {
    let mut frame_damage = damage;
    for attempt in 0..=1 {
        match prepare(expected_generation, frame_damage, attempt == 0)? {
            CatalogFrameAttempt::Prepared(report) => return Ok(report),
            CatalogFrameAttempt::Expanded(catalog_generation) if attempt == 0 => {
                expected_generation = catalog_generation;
                frame_damage = &[];
            }
            CatalogFrameAttempt::Expanded(_) => {
                return Err(io::Error::other(
                    "GPU font catalog expanded twice during one presented frame",
                )
                .into());
            }
        }
    }
    unreachable!("bounded catalog frame loop always returns")
}

#[expect(
    clippy::too_many_arguments,
    reason = "one app-owned transaction couples font preflight to one complete GPU text frame"
)]
#[cfg(not(feature = "rterm-legacy-0-1"))]
fn prepare_gpu_text_frame(
    font_repository: &mut PlatformFontRepository,
    renderer: &mut GpuLayerRenderer,
    snapshot: &TerminalRenderSnapshot,
    geometry: RenderGeometry,
    damage: &[DamageRegion],
    paint: &TextPaintConfig,
    dpi_scale: f32,
    zoom: f32,
) -> Result<GpuTextPrepareReport, Box<dyn Error>> {
    font_repository.preflight_snapshot(
        snapshot,
        renderer
            .text_catalog_mut()
            .expect("window GPU text catalog is enabled"),
    )?;
    let expected_generation = renderer
        .text_catalog_generation()
        .expect("window GPU text catalog is enabled");
    prepare_catalog_frame_with_one_restart(
        expected_generation,
        damage,
        |expected_generation, frame_damage, can_expand| {
            let prepared = renderer.prepare_text_for_catalog_generation(
                expected_generation,
                snapshot,
                geometry,
                frame_damage,
                paint,
                dpi_scale,
                zoom,
            )?;
            let expansion = prepared.catalog_expansion();
            if let Some(report) = prepared.into_prepared() {
                if can_expand && !report.missing_glyphs.is_empty() {
                    let activation = match font_repository.activate_missing_glyphs(
                        &report.missing_glyphs,
                        renderer
                            .text_catalog_mut()
                            .expect("window GPU text catalog is enabled"),
                    ) {
                        Ok(activation) => activation,
                        Err(activation_error) => {
                            if let Err(discard_error) = renderer.discard_prepared_text_frame() {
                                return Err(io::Error::other(format!(
                                    "late font activation failed: {activation_error}; discarding the partial GPU text frame also failed: {discard_error}"
                                ))
                                .into());
                            }
                            return Err(io::Error::other(format!(
                                "late font activation failed after discarding the partial GPU text frame: {activation_error}"
                            ))
                            .into());
                        }
                    };
                    if let CatalogActivation::CatalogExpanded {
                        catalog_generation, ..
                    } = activation
                    {
                        renderer.discard_prepared_text_frame()?;
                        return Ok(CatalogFrameAttempt::Expanded(catalog_generation));
                    }
                }
                return Ok(CatalogFrameAttempt::Prepared(report));
            }
            let Some((_, catalog_generation)) = expansion else {
                return Err(io::Error::other("GPU text returned no frame outcome").into());
            };
            Ok(CatalogFrameAttempt::Expanded(catalog_generation))
        },
    )
}

#[cfg(feature = "rterm-legacy-0-1")]
#[expect(
    clippy::too_many_arguments,
    reason = "the legacy adapter preserves the window frame call boundary"
)]
fn prepare_gpu_text_frame(
    font_repository: &mut PlatformFontRepository,
    renderer: &mut GpuLayerRenderer,
    snapshot: &TerminalRenderSnapshot,
    geometry: RenderGeometry,
    _damage: &[DamageRegion],
    paint: &TextPaintConfig,
    dpi_scale: f32,
    zoom: f32,
) -> Result<GpuTextPrepareReport, Box<dyn Error>> {
    crate::rterm_compat_gpu::prepare_full_frame(
        font_repository,
        renderer,
        &crate::rterm_compat_gpu::LegacyTextSettings {
            fonts: bundled_emergency_font_config(),
            text: GpuTextConfig::new(
                4 * 1024 * 1024,
                rterm_fonts::RasterCacheConfig::new(4 * 1024 * 1024),
            ),
        },
        snapshot,
        geometry,
        paint,
        dpi_scale,
        zoom,
    )
}

#[cfg(not(feature = "rterm-legacy-0-1"))]
fn retire_lost_renderer_before_rebuild<T>(
    lost_renderer: &mut GpuLayerRenderer,
    rebuild: impl FnOnce(&GpuLayerRenderer) -> Result<T, Box<dyn Error>>,
) -> Result<T, Box<dyn Error>> {
    lost_renderer.retire_text_cpu_font_state();
    rebuild(lost_renderer).map_err(|error| {
        io::Error::other(format!(
            "rebuild GPU renderer after retiring lost CPU font state: {error}"
        ))
        .into()
    })
}

#[cfg(feature = "rterm-legacy-0-1")]
fn retire_lost_renderer_before_rebuild<T>(
    lost_renderer: &mut GpuLayerRenderer,
    rebuild: impl FnOnce(&GpuLayerRenderer) -> Result<T, Box<dyn Error>>,
) -> Result<T, Box<dyn Error>> {
    crate::rterm_compat_gpu::retire_catalog_before_rebuild(lost_renderer, |retired| {
        rebuild(retired)
    })
}

#[derive(Debug, Default)]
struct DeviceRecoveryCoordinator {
    pending: bool,
}

impl DeviceRecoveryCoordinator {
    #[cfg(test)]
    const fn pending(&self) -> bool {
        self.pending
    }

    fn cancel_pending(&mut self) -> bool {
        std::mem::take(&mut self.pending)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "one typed recovery transaction owns present, rebuild, classification, and metric callbacks"
    )]
    fn present<F, T, E>(
        &mut self,
        frame: &F,
        mut present: impl FnMut(&F) -> Result<T, E>,
        mut rebuild: impl FnMut() -> Result<(), E>,
        is_device_lost: impl Fn(&E) -> bool,
        is_presented: impl Fn(&T) -> bool,
        mut commit_recovery: impl FnMut(),
        mut record_recovery_failure: impl FnMut(),
    ) -> Result<T, E> {
        if self.pending {
            return match present(frame) {
                Ok(outcome) => {
                    if is_presented(&outcome) {
                        self.pending = false;
                        commit_recovery();
                    }
                    Ok(outcome)
                }
                Err(error) => {
                    self.pending = false;
                    record_recovery_failure();
                    Err(error)
                }
            };
        }

        match present(frame) {
            Err(error) if is_device_lost(&error) => {
                rebuild()?;
                match present(frame) {
                    Ok(outcome) => {
                        if is_presented(&outcome) {
                            commit_recovery();
                        } else {
                            self.pending = true;
                        }
                        Ok(outcome)
                    }
                    Err(error) => {
                        record_recovery_failure();
                        Err(error)
                    }
                }
            }
            outcome => outcome,
        }
    }
}

#[cfg(test)]
fn retry_device_lost_once<F, T, E>(
    frame: &F,
    present: impl FnMut(&F) -> Result<T, E>,
    rebuild: impl FnMut() -> Result<(), E>,
    is_device_lost: impl Fn(&E) -> bool,
    mut commit_recovery: impl FnMut(&T),
    record_retry_failure: impl FnMut(),
) -> Result<T, E> {
    DeviceRecoveryCoordinator::default()
        .present(
            frame,
            present,
            rebuild,
            is_device_lost,
            |_| true,
            || {},
            record_retry_failure,
        )
        .inspect(|outcome| commit_recovery(outcome))
}

fn should_apply_current_adapter_abandonment_workaround(
    os: &str,
    current: &GpuPresentationMetrics,
    shutdown_intent: bool,
    replaced_device: bool,
) -> bool {
    should_abandon_recovered_window_surface(
        os,
        &current.backend,
        current.adapter_vendor_id,
        shutdown_intent,
        replaced_device,
    )
}

fn should_abandon_current_adapter_after_native_close(
    os: &str,
    backend: &str,
    vendor_id: u32,
) -> bool {
    os == "windows" && backend.eq_ignore_ascii_case("vulkan") && vendor_id == 0x10de
}

#[cfg(any(test, not(feature = "rterm-legacy-0-1")))]
fn finalize_stage7_native_gpu_owners<Renderer, Device, Context>(
    os: &str,
    backend: &str,
    vendor_id: u32,
    renderer: &mut Option<Renderer>,
    device: &mut Option<Device>,
    context: &mut Option<Context>,
) -> bool {
    let abandon = should_abandon_current_adapter_after_native_close(os, backend, vendor_id);
    if abandon {
        if let Some(renderer) = renderer.take() {
            std::mem::forget(renderer);
        }
        if let Some(context) = context.take() {
            std::mem::forget(context);
        }
        if let Some(device) = device.take() {
            std::mem::forget(device);
        }
    } else {
        drop(renderer.take());
        drop(context.take());
        drop(device.take());
    }
    abandon
}

fn gpu_context_options(
    high_performance: bool,
    force_fallback_adapter: bool,
    diagnostic_backend: Option<DiagnosticGpuBackend>,
) -> Result<GpuContextOptions, GpuContextError> {
    let options = GpuContextOptions::default()
        .with_high_performance(high_performance)
        .with_force_fallback_adapter(force_fallback_adapter);
    diagnostic_backend.map_or(Ok(options), |backend| {
        options.with_only_backend_name(backend.as_str())
    })
}

#[cfg(feature = "diagnostic-tools")]
#[cfg(not(feature = "rterm-legacy-0-1"))]
const fn diagnostic_font_catalog_mode(
    mode: Option<rterm_diagnostics::DiagnosticFontMode>,
) -> FontCatalogMode {
    match mode {
        None => production_font_catalog_mode(),
        Some(rterm_diagnostics::DiagnosticFontMode::CurrentCopied) => {
            FontCatalogMode::CurrentCopied
        }
        Some(rterm_diagnostics::DiagnosticFontMode::SharedAll) => FontCatalogMode::SharedAll,
        Some(rterm_diagnostics::DiagnosticFontMode::Lazy) => FontCatalogMode::Lazy,
    }
}

#[cfg(any(not(feature = "diagnostic-tools"), feature = "rterm-legacy-0-1"))]
const fn diagnostic_font_catalog_mode(
    _mode: Option<rterm_diagnostics::DiagnosticFontMode>,
) -> FontCatalogMode {
    production_font_catalog_mode()
}

pub(crate) const fn diagnostic_font_specimen_text(
    specimen: rterm_diagnostics::DiagnosticFontSpecimen,
) -> &'static str {
    match specimen {
        rterm_diagnostics::DiagnosticFontSpecimen::Ascii => "R-Term Stage 7",
        rterm_diagnostics::DiagnosticFontSpecimen::Cjk => "中文",
        rterm_diagnostics::DiagnosticFontSpecimen::Emoji => "😀",
    }
}

#[cfg(not(feature = "rterm-legacy-0-1"))]
fn sha256_hex(digest: [u8; 32]) -> String {
    use std::fmt::Write as _;

    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(feature = "diagnostic-tools")]
#[cfg(not(feature = "rterm-legacy-0-1"))]
fn prepare_diagnostic_font_catalog(
    mut repository: PlatformFontRepository,
    mode: Option<rterm_diagnostics::DiagnosticFontMode>,
    specimen: Option<rterm_diagnostics::DiagnosticFontSpecimen>,
) -> Result<
    (
        PlatformFontRepository,
        rterm_fonts::FontCatalog,
        Option<DiagnosticFontResourceSummary>,
    ),
    Box<dyn Error>,
> {
    if mode.is_some() != specimen.is_some() {
        return Err(io::Error::other(
            "diagnostic font mode and specimen must be provided together",
        )
        .into());
    }
    let catalog_mode = diagnostic_font_catalog_mode(mode);
    let mut catalog = repository.build_catalog(catalog_mode)?;
    let Some((mode, specimen)) = mode.zip(specimen) else {
        return Ok((repository, catalog, None));
    };

    let activation_started = Instant::now();
    repository.preflight_text(diagnostic_font_specimen_text(specimen), &mut catalog)?;
    let activation_latency_micros =
        u64::try_from(activation_started.elapsed().as_micros()).unwrap_or(u64::MAX);
    let recovered = repository.rebuild_catalog_from_active(catalog_mode)?;
    let diagnostics = repository.diagnostics()?;
    let catalog_metrics = catalog.memory_metrics();
    let recovery_metrics = recovered.memory_metrics();
    let exposes_full_inventory = matches!(
        mode,
        rterm_diagnostics::DiagnosticFontMode::CurrentCopied
            | rterm_diagnostics::DiagnosticFontMode::SharedAll
    );
    let summary = DiagnosticFontResourceSummary {
        mode,
        specimen,
        retained_source_bytes: catalog_metrics.retained_source_bytes,
        indexed_source_count: diagnostics.indexed_source_count,
        active_source_count: diagnostics.active_source_count,
        initial_catalog_source_count: diagnostics.initial_catalog_source_count,
        catalog_builds: diagnostics.catalog_builds,
        generation: diagnostics.generation,
        recovery_retained_source_bytes: recovery_metrics.retained_source_bytes,
        recovery_generation: recovered.generation(),
        activation_latency_micros,
        // The proof marker is not emitted until a complete GPU present has
        // replaced this placeholder with evidence from that exact frame.
        tofu_count: 0,
        frame_catalog_generation: None,
        frame_generation_consistent: None,
        index_fingerprint_sha256: sha256_hex(diagnostics.index_fingerprint),
        catalog_fingerprint_sha256: sha256_hex(diagnostics.catalog_fingerprint),
        ordered_catalog_fingerprint_sha256: sha256_hex(catalog.diagnostic_ordered_fingerprint()),
        font_inventory_fingerprint_sha256: exposes_full_inventory
            .then(|| sha256_hex(diagnostics.inventory_fingerprint)),
        font_index_policy_version: exposes_full_inventory.then_some(diagnostics.policy_version),
    };
    Ok((repository, catalog, Some(summary)))
}

#[cfg(not(feature = "rterm-legacy-0-1"))]
fn record_diagnostic_font_frame_evidence(
    summary: &mut DiagnosticFontResourceSummary,
    frame_catalog_generation: u64,
    missing_glyphs: &[char],
) {
    summary.frame_catalog_generation = Some(frame_catalog_generation);
    summary.frame_generation_consistent = Some(frame_catalog_generation == summary.generation);
    let specimen = diagnostic_font_specimen_text(summary.specimen);
    summary.tofu_count = missing_glyphs
        .iter()
        .filter(|codepoint| {
            specimen.contains(**codepoint)
                && !codepoint.is_whitespace()
                && **codepoint != '\u{fe0f}'
        })
        .count();
}

#[cfg(feature = "diagnostic-tools")]
#[cfg(not(feature = "rterm-legacy-0-1"))]
fn finalize_diagnostic_font_resource_summary(
    repository: &PlatformFontRepository,
    catalog_mode: FontCatalogMode,
    renderer: &mut GpuLayerRenderer,
    report: &GpuTextPrepareReport,
    summary: &mut DiagnosticFontResourceSummary,
) -> Result<(), Box<dyn Error>> {
    let diagnostics = repository.diagnostics()?;
    let catalog = renderer
        .text_catalog_mut()
        .ok_or_else(|| io::Error::other("window GPU text catalog is not enabled"))?;
    let catalog_metrics = catalog.memory_metrics();
    let catalog_generation = catalog.generation();
    let ordered_catalog_fingerprint = catalog.diagnostic_ordered_fingerprint();
    if diagnostics.active_source_count != catalog_metrics.active_source_count
        || diagnostics.catalog_builds != catalog_metrics.catalog_builds
        || diagnostics.generation != catalog_generation
    {
        return Err(io::Error::other(
            "platform font repository and presented GPU catalog epochs diverged",
        )
        .into());
    }
    let recovered = repository.rebuild_catalog_from_active(catalog_mode)?;
    let recovery_metrics = recovered.memory_metrics();

    summary.retained_source_bytes = catalog_metrics.retained_source_bytes;
    summary.indexed_source_count = diagnostics.indexed_source_count;
    summary.active_source_count = catalog_metrics.active_source_count;
    summary.initial_catalog_source_count = diagnostics.initial_catalog_source_count;
    summary.catalog_builds = catalog_metrics.catalog_builds;
    summary.generation = catalog_generation;
    summary.recovery_retained_source_bytes = recovery_metrics.retained_source_bytes;
    summary.recovery_generation = recovered.generation();
    summary.index_fingerprint_sha256 = sha256_hex(diagnostics.index_fingerprint);
    summary.catalog_fingerprint_sha256 = sha256_hex(diagnostics.catalog_fingerprint);
    summary.ordered_catalog_fingerprint_sha256 = sha256_hex(ordered_catalog_fingerprint);
    let exposes_full_inventory = matches!(
        catalog_mode,
        FontCatalogMode::CurrentCopied | FontCatalogMode::SharedAll
    );
    summary.font_inventory_fingerprint_sha256 =
        exposes_full_inventory.then(|| sha256_hex(diagnostics.inventory_fingerprint));
    summary.font_index_policy_version =
        exposes_full_inventory.then_some(diagnostics.policy_version);
    record_diagnostic_font_frame_evidence(
        summary,
        report.catalog_generation,
        &report.missing_glyphs,
    );
    Ok(())
}

#[cfg(feature = "diagnostic-tools")]
#[cfg(not(feature = "rterm-legacy-0-1"))]
fn finalize_diagnostic_font_resource_summary_once(
    summary: &mut Option<DiagnosticFontResourceSummary>,
    finalize: impl FnOnce(&mut DiagnosticFontResourceSummary) -> Result<(), Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let Some(summary) = summary.as_mut() else {
        return Ok(());
    };
    if summary.frame_catalog_generation.is_some() {
        return Ok(());
    }
    finalize(summary)
}

#[cfg(any(not(feature = "diagnostic-tools"), feature = "rterm-legacy-0-1"))]
fn prepare_diagnostic_font_catalog(
    mut repository: PlatformFontRepository,
    mode: Option<rterm_diagnostics::DiagnosticFontMode>,
    specimen: Option<rterm_diagnostics::DiagnosticFontSpecimen>,
) -> Result<
    (
        PlatformFontRepository,
        rterm_fonts::FontCatalog,
        Option<DiagnosticFontResourceSummary>,
    ),
    Box<dyn Error>,
> {
    if mode.is_some() || specimen.is_some() {
        #[cfg(feature = "rterm-legacy-0-1")]
        return Err(crate::rterm_compat::unsupported_diagnostics());
        #[cfg(not(feature = "rterm-legacy-0-1"))]
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "font proof requires diagnostic-tools",
        )
        .into());
    }
    let catalog = repository.build_catalog(production_font_catalog_mode())?;
    Ok((repository, catalog, None))
}

impl WindowGpu {
    pub(crate) async fn new(
        display: OwnedDisplayHandle,
        window: Arc<Window>,
        surface_size: PhysicalSize<u32>,
        high_performance: bool,
        force_fallback_adapter: bool,
    ) -> Result<Self, Box<dyn Error>> {
        Self::new_with_diagnostic_backend(
            display,
            window,
            surface_size,
            high_performance,
            force_fallback_adapter,
            None,
        )
        .await
    }

    pub(crate) async fn new_with_diagnostic_backend(
        display: OwnedDisplayHandle,
        window: Arc<Window>,
        surface_size: PhysicalSize<u32>,
        high_performance: bool,
        force_fallback_adapter: bool,
        diagnostic_backend: Option<DiagnosticGpuBackend>,
    ) -> Result<Self, Box<dyn Error>> {
        Self::new_with_diagnostic_options(
            display,
            window,
            surface_size,
            high_performance,
            force_fallback_adapter,
            diagnostic_backend,
            None,
            None,
        )
        .await
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "private diagnostics forward independent backend, font mode, and specimen selections"
    )]
    pub(crate) async fn new_with_diagnostic_options(
        display: OwnedDisplayHandle,
        window: Arc<Window>,
        surface_size: PhysicalSize<u32>,
        high_performance: bool,
        force_fallback_adapter: bool,
        diagnostic_backend: Option<DiagnosticGpuBackend>,
        diagnostic_font_mode: Option<rterm_diagnostics::DiagnosticFontMode>,
        diagnostic_font_specimen: Option<rterm_diagnostics::DiagnosticFontSpecimen>,
    ) -> Result<Self, Box<dyn Error>> {
        let prepared = Self::prepare_with_diagnostic_options(
            display,
            window,
            surface_size,
            high_performance,
            force_fallback_adapter,
            diagnostic_backend,
            diagnostic_font_mode,
            diagnostic_font_specimen,
        )?;
        Self::finish_prepared(prepared).await
    }

    pub(crate) fn prepare(
        display: OwnedDisplayHandle,
        window: Arc<Window>,
        surface_size: PhysicalSize<u32>,
        high_performance: bool,
        force_fallback_adapter: bool,
    ) -> Result<PreparedWindowGpu, Box<dyn Error>> {
        Self::prepare_with_diagnostic_backend(
            display,
            window,
            surface_size,
            high_performance,
            force_fallback_adapter,
            None,
        )
    }

    pub(crate) fn prepare_with_diagnostic_backend(
        display: OwnedDisplayHandle,
        window: Arc<Window>,
        surface_size: PhysicalSize<u32>,
        high_performance: bool,
        force_fallback_adapter: bool,
        diagnostic_backend: Option<DiagnosticGpuBackend>,
    ) -> Result<PreparedWindowGpu, Box<dyn Error>> {
        Self::prepare_with_diagnostic_options(
            display,
            window,
            surface_size,
            high_performance,
            force_fallback_adapter,
            diagnostic_backend,
            None,
            None,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "private diagnostics forward independent backend, font mode, and specimen selections"
    )]
    pub(crate) fn prepare_with_diagnostic_options(
        display: OwnedDisplayHandle,
        window: Arc<Window>,
        surface_size: PhysicalSize<u32>,
        high_performance: bool,
        force_fallback_adapter: bool,
        diagnostic_backend: Option<DiagnosticGpuBackend>,
        diagnostic_font_mode: Option<rterm_diagnostics::DiagnosticFontMode>,
        diagnostic_font_specimen: Option<rterm_diagnostics::DiagnosticFontSpecimen>,
    ) -> Result<PreparedWindowGpu, Box<dyn Error>> {
        let options =
            gpu_context_options(high_performance, force_fallback_adapter, diagnostic_backend)?;
        let context = GpuContext::prepare_windowed(
            display,
            window,
            surface_size.width,
            surface_size.height,
            options,
        )?;
        Ok(PreparedWindowGpu {
            context,
            diagnostic_font_mode,
            diagnostic_font_specimen,
        })
    }

    pub(crate) async fn finish_prepared(
        prepared: PreparedWindowGpu,
    ) -> Result<Self, Box<dyn Error>> {
        let font_catalog_mode = diagnostic_font_catalog_mode(prepared.diagnostic_font_mode);
        let context = GpuContext::finish_windowed(prepared.context).await?;
        let (font_repository, catalog, diagnostic_font_resources) =
            prepare_diagnostic_font_catalog(
                PlatformFontRepository::production_index(),
                prepared.diagnostic_font_mode,
                prepared.diagnostic_font_specimen,
            )?;
        let renderer = bundled_emergency_text_backend(&context, catalog)?;
        Ok(Self {
            context: Some(context),
            renderer: Some(renderer),
            retired_renderers: Vec::new(),
            font_repository,
            font_catalog_mode,
            recovery: DeviceRecoveryCoordinator::default(),
            report: None,
            rendered_frames: 0,
            replaced_device: false,
            final_metrics: None,
            diagnostic_font_resources,
            #[cfg(test)]
            abandonment_workaround_adapter_match_override: None,
            #[cfg(test)]
            current_adapter_metrics_override: None,
            #[cfg(test)]
            abandonment_workaround_os_override: None,
            #[cfg(debug_assertions)]
            test_device_loss_injected: false,
        })
    }

    pub(crate) fn resize_surface(&mut self, size: PhysicalSize<u32>) -> Result<(), Box<dyn Error>> {
        self.context_mut().resize_surface(size.width, size.height)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn present(
        &mut self,
        window: &Window,
        snapshot: &TerminalRenderSnapshot,
        geometry: RenderGeometry,
        damage: &[DamageRegion],
        paint: &TextPaintConfig,
        graph: &RenderGraph,
        render_mode: rterm_native::RenderMode,
        dpi_scale: f32,
    ) -> Result<GpuFrameStatus, Box<dyn Error>> {
        #[cfg(debug_assertions)]
        if !self.test_device_loss_injected
            && std::env::var_os("RTERM_TEST_GPU_DEVICE_LOSS").is_some()
        {
            self.context_mut().inject_device_loss_for_test();
            self.test_device_loss_injected = true;
        }
        // A bounded late activation can leave another candidate for a static
        // row. Repaint all rows on its explicit continuation, even without input
        // damage or cursor animation.
        let render_mode = if self
            .report
            .as_ref()
            .is_some_and(|report| font_fallback_redraw_needed(&self.font_repository, report))
        {
            rterm_native::RenderMode::Full
        } else {
            render_mode
        };
        let frame = WindowGpuFrame {
            window,
            snapshot,
            geometry,
            damage,
            paint,
            graph,
            render_mode,
            dpi_scale,
        };
        let mut recovery = std::mem::take(&mut self.recovery);
        let state = RefCell::new(self);
        let outcome = recovery.present(
            &frame,
            |borrowed| state.borrow_mut().present_once(borrowed),
            || {
                state
                    .borrow_mut()
                    .execute_host_renderer_effect(&rterm_native::RendererEffect::RecoverDevice)
            },
            |error| {
                error
                    .as_ref()
                    .downcast_ref::<GpuContextError>()
                    .is_some_and(|error| error.kind() == GpuContextErrorKind::DeviceLost)
            },
            |outcome| outcome.0 == GpuFrameStatus::Presented,
            || {
                state
                    .borrow_mut()
                    .context_mut()
                    .commit_windowed_device_recovery();
            },
            || {
                state
                    .borrow_mut()
                    .context_mut()
                    .record_device_recovery_failure();
            },
        );
        let this = state.into_inner();
        this.recovery = recovery;
        let (status, report) = outcome?;
        if status == GpuFrameStatus::Presented {
            #[cfg(all(feature = "diagnostic-tools", not(feature = "rterm-legacy-0-1")))]
            finalize_diagnostic_font_resource_summary_once(
                &mut this.diagnostic_font_resources,
                |resources| {
                    finalize_diagnostic_font_resource_summary(
                        &this.font_repository,
                        this.font_catalog_mode,
                        this.renderer
                            .as_mut()
                            .expect("window GPU renderer is available before shutdown"),
                        &report,
                        resources,
                    )
                },
            )?;
            #[cfg(any(not(feature = "diagnostic-tools"), feature = "rterm-legacy-0-1"))]
            debug_assert!(this.diagnostic_font_resources.is_none());
            if font_fallback_redraw_needed(&this.font_repository, &report) {
                window.request_redraw();
            }
            this.report = Some(report);
            this.rendered_frames = this.rendered_frames.saturating_add(1);
        }
        Ok(status)
    }

    fn present_once(
        &mut self,
        frame: &WindowGpuFrame<'_>,
    ) -> Result<(GpuFrameStatus, GpuTextPrepareReport), Box<dyn Error>> {
        let damage = presentation_damage(frame.render_mode, frame.damage);
        let renderer = self
            .renderer
            .as_mut()
            .expect("window GPU renderer is available before shutdown");
        let report = prepare_gpu_text_frame(
            &mut self.font_repository,
            renderer,
            frame.snapshot,
            frame.geometry,
            damage,
            frame.paint,
            frame.dpi_scale,
            1.0,
        )?;
        let status = self
            .context
            .as_mut()
            .expect("window GPU context is available before shutdown")
            .render_graph(
                self.renderer
                    .as_mut()
                    .expect("window GPU renderer is available before shutdown"),
                frame.graph,
                || {
                    frame.window.pre_present_notify();
                },
            )?;
        Ok((status, report))
    }

    fn rebuild_device_and_layers(&mut self) -> Result<(), Box<dyn Error>> {
        pollster::block_on(self.context_mut().recover_device())?;
        self.replaced_device = true;
        let font_catalog_mode = self.font_catalog_mode;
        let font_repository = &self.font_repository;
        let context = self
            .context
            .as_ref()
            .expect("window GPU context is available before shutdown");
        let lost_renderer = self
            .renderer
            .as_mut()
            .expect("lost renderer is available before recovery");
        let renderer = match retire_lost_renderer_before_rebuild(lost_renderer, |_| {
            let catalog = font_repository.rebuild_catalog_from_active(font_catalog_mode)?;
            bundled_emergency_text_backend(context, catalog)
        }) {
            Ok(renderer) => renderer,
            Err(error) => {
                self.context_mut().record_device_recovery_failure();
                return Err(error);
            }
        };
        let lost_renderer = self
            .renderer
            .replace(renderer)
            .expect("lost renderer is retained until shutdown");
        self.retired_renderers.push(lost_renderer);
        self.report = None;
        Ok(())
    }

    fn execute_host_renderer_effect(
        &mut self,
        effect: &rterm_native::RendererEffect,
    ) -> Result<(), Box<dyn Error>> {
        match effect {
            rterm_native::RendererEffect::RecoverDevice => self.rebuild_device_and_layers(),
            _ => Err(io::Error::other("renderer effect requires the frame adapter").into()),
        }
    }

    /// Applies the narrowly-scoped NVIDIA Vulkan window-close workaround.
    ///
    /// The manager calls this before removing every window. Unrecovered or
    /// non-matching windows keep their resources for normal Drop.
    pub(crate) fn shutdown_for_window_close(&mut self) -> bool {
        if self.recovery.cancel_pending()
            && let Some(context) = self.context.as_mut()
        {
            context.record_device_recovery_failure();
        }
        if !self.should_apply_abandonment_workaround() {
            return false;
        }
        let Some(mut context) = self.context.take() else {
            return false;
        };
        if self.replaced_device {
            context.record_abandoned_lost_surface();
        }
        self.final_metrics = Some(context.metrics().clone());
        if let Some(renderer) = self.renderer.take() {
            std::mem::forget(renderer);
        }
        for renderer in self.retired_renderers.drain(..) {
            std::mem::forget(renderer);
        }
        std::mem::forget(context);
        true
    }

    /// Avoids a validation-layer crash after the final native close request
    /// on the affected Windows NVIDIA Vulkan stack.
    pub(crate) fn shutdown_after_native_window_close(&mut self) -> bool {
        #[cfg(test)]
        let os = self
            .abandonment_workaround_os_override
            .unwrap_or(std::env::consts::OS);
        #[cfg(not(test))]
        let os = std::env::consts::OS;
        let should_abandon = self.context.as_ref().is_some_and(|context| {
            #[cfg(test)]
            let metrics = self
                .current_adapter_metrics_override
                .as_ref()
                .unwrap_or_else(|| context.metrics());
            #[cfg(not(test))]
            let metrics = context.metrics();
            should_abandon_current_adapter_after_native_close(
                os,
                &metrics.backend,
                metrics.adapter_vendor_id,
            )
        });
        if !should_abandon {
            return self.shutdown_for_window_close();
        }
        if self.recovery.cancel_pending()
            && let Some(context) = self.context.as_mut()
        {
            context.record_device_recovery_failure();
        }
        let Some(mut context) = self.context.take() else {
            return false;
        };
        if self.replaced_device {
            context.record_abandoned_lost_surface();
        }
        self.final_metrics = Some(context.metrics().clone());
        if let Some(renderer) = self.renderer.take() {
            std::mem::forget(renderer);
        }
        for renderer in self.retired_renderers.drain(..) {
            std::mem::forget(renderer);
        }
        std::mem::forget(context);
        eprintln!(
            "preserving Windows NVIDIA Vulkan GPU resources after native window close to avoid an unsafe driver shutdown"
        );
        true
    }

    #[cfg(test)]
    pub(crate) fn for_manager_close_test(
        abandonment_workaround_adapter_match: bool,
        replaced_device: bool,
    ) -> Self {
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("manager close test context");
        let mut current = GpuPresentationMetrics::uninitialized();
        current.backend = "Vulkan".to_owned();
        current.adapter_vendor_id = if abandonment_workaround_adapter_match {
            0x10de
        } else {
            0x1002
        };
        Self {
            context: Some(context),
            renderer: None,
            retired_renderers: Vec::new(),
            font_repository: PlatformFontRepository::production_index_for_os("test"),
            font_catalog_mode: production_font_catalog_mode(),
            recovery: DeviceRecoveryCoordinator::default(),
            report: None,
            rendered_frames: 0,
            replaced_device,
            final_metrics: None,
            diagnostic_font_resources: None,
            abandonment_workaround_adapter_match_override: Some(
                abandonment_workaround_adapter_match,
            ),
            current_adapter_metrics_override: Some(current),
            abandonment_workaround_os_override: Some("windows"),
            #[cfg(debug_assertions)]
            test_device_loss_injected: false,
        }
    }

    pub(crate) fn metrics(&self) -> &GpuPresentationMetrics {
        if let Some(context) = self.context.as_ref() {
            context.metrics()
        } else {
            self.final_metrics
                .as_ref()
                .expect("final GPU metrics are retained after window-close abandonment")
        }
    }

    pub(crate) fn diagnostic_font_resources(&self) -> Option<&DiagnosticFontResourceSummary> {
        self.diagnostic_font_resources.as_ref()
    }

    pub(crate) fn direct_text_metrics(&self) -> Option<(&GpuTextPrepareReport, u64)> {
        self.report
            .as_ref()
            .map(|report| (report, self.rendered_frames))
    }

    fn context_mut(&mut self) -> &mut GpuContext {
        self.context
            .as_mut()
            .expect("window GPU context is available before shutdown")
    }

    fn should_apply_abandonment_workaround(&self) -> bool {
        let Some(context) = self.context.as_ref() else {
            return false;
        };
        #[cfg(test)]
        if let Some(workaround_match) = self.abandonment_workaround_adapter_match_override {
            return workaround_match && self.replaced_device;
        }
        #[cfg(test)]
        let current = self
            .current_adapter_metrics_override
            .as_ref()
            .unwrap_or_else(|| context.metrics());
        #[cfg(not(test))]
        let current = context.metrics();
        #[cfg(test)]
        let os = self
            .abandonment_workaround_os_override
            .unwrap_or(std::env::consts::OS);
        #[cfg(not(test))]
        let os = std::env::consts::OS;
        should_apply_current_adapter_abandonment_workaround(os, current, true, self.replaced_device)
    }
}

fn presentation_damage(mode: rterm_native::RenderMode, damage: &[DamageRegion]) -> &[DamageRegion] {
    match mode {
        rterm_native::RenderMode::Full => &[],
        rterm_native::RenderMode::Damage => damage,
    }
}

impl Drop for WindowGpu {
    fn drop(&mut self) {
        if self.recovery.cancel_pending()
            && let Some(context) = self.context.as_mut()
        {
            context.record_device_recovery_failure();
        }
        release_renderers_before_context(
            &mut self.renderer,
            &mut self.retired_renderers,
            &mut self.context,
        );
    }
}

fn release_renderers_before_context<Renderer, Context>(
    renderer: &mut Option<Renderer>,
    retired_renderers: &mut Vec<Renderer>,
    context: &mut Option<Context>,
) {
    drop(renderer.take());
    retired_renderers.clear();
    drop(context.take());
}

fn bundled_emergency_text_backend(
    context: &GpuContext,
    catalog: rterm_fonts::FontCatalog,
) -> Result<GpuLayerRenderer, Box<dyn Error>> {
    use rterm_fonts::RasterCacheConfig;

    let font_config = bundled_emergency_font_config();
    let format = context
        .surface_format()
        .ok_or_else(|| io::Error::other("direct text fixture requires a surface format"))?;
    let mut renderer = GpuLayerRenderer::new(context, format, DEFAULT_GPU_INSTANCE_BYTE_BUDGET)?;
    renderer.enable_text(
        catalog,
        font_config,
        GpuTextConfig::new(4 * 1024 * 1024, RasterCacheConfig::new(4 * 1024 * 1024)),
    )?;
    Ok(renderer)
}

fn bundled_emergency_font_config() -> rterm_fonts::FontConfig {
    rterm_fonts::FontConfig::new("Cascadia Mono")
        .with_fallbacks([
            "Cascadia Code",
            "Source Code Pro",
            "Consolas",
            "Menlo",
            "Monaco",
            "Noto Sans Mono",
            "DejaVu Sans Mono",
            "Noto Sans",
            "Noto Sans SC",
            "Noto Sans JP",
            "Hiragino Sans GB",
            "Arial Unicode MS",
            "Apple SD Gothic Neo",
            "Noto Sans Arabic",
            "Noto Sans Devanagari",
            "Noto Sans Hebrew",
            "Noto Sans Symbols 2",
            "Noto Color Emoji",
            "Microsoft YaHei",
            "Meiryo",
            "Malgun Gothic",
            "Segoe UI",
            "Nirmala UI",
            "Segoe UI Emoji",
        ])
        .with_font_size(17.0)
}

/// Owns live and quarantined devices together until the window close policy runs.
#[derive(Default)]
pub(crate) struct WindowGpuOwners {
    pub(crate) active: Option<Box<WindowGpu>>,
    #[allow(
        clippy::vec_box,
        reason = "retain boxed GPU owners without moving teardown state"
    )]
    pub(crate) quarantined: Vec<Box<WindowGpu>>,
}

impl WindowGpuOwners {
    pub(crate) fn quarantine_active(&mut self) {
        // Failed recovery can retain driver objects that are unsafe to drop here.
        // Remove them from present/resize without changing their final-close policy.
        if let Some(gpu) = self.active.take() {
            self.quarantined.push(gpu);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use rterm_core::TerminalSize;

    #[cfg(feature = "rterm-legacy-0-1")]
    #[test]
    fn legacy_diagnostic_font_options_reject_proof_but_allow_production_catalog() {
        use rterm_diagnostics::{DiagnosticFontMode, DiagnosticFontSpecimen};
        let (_, catalog, summary) = prepare_diagnostic_font_catalog(
            PlatformFontRepository::production_index_for_os("test"),
            None,
            None,
        )
        .expect("real production catalog");
        assert!(catalog.face_count() > 0);
        assert!(summary.is_none());
        for (mode, specimen) in [
            (Some(DiagnosticFontMode::Lazy), None),
            (Some(DiagnosticFontMode::SharedAll), None),
            (Some(DiagnosticFontMode::CurrentCopied), None),
            (None, Some(DiagnosticFontSpecimen::Cjk)),
        ] {
            let error = prepare_diagnostic_font_catalog(
                PlatformFontRepository::production_index_for_os("test"),
                mode,
                specimen,
            )
            .err()
            .expect("legacy proof is unavailable");
            assert_eq!(
                error
                    .downcast_ref::<io::Error>()
                    .expect("typed error")
                    .kind(),
                io::ErrorKind::Unsupported
            );
        }
    }

    fn test_headless_renderer(context: &GpuContext) -> Result<GpuLayerRenderer, Box<dyn Error>> {
        #[cfg(not(feature = "rterm-legacy-0-1"))]
        let renderer = GpuLayerRenderer::new_headless(context, 64 * 1024)?;
        #[cfg(feature = "rterm-legacy-0-1")]
        let renderer = GpuLayerRenderer::new(context, wgpu::TextureFormat::Rgba8Unorm, 64 * 1024)?;
        Ok(renderer)
    }
    use rterm_diagnostics::DiagnosticGpuBackend;
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    use rterm_fonts::TerminalShaper;
    use rterm_render_core::terminal_snapshot_content_digest;
    use rterm_terminal::Terminal;
    use winit::event_loop::OwnedDisplayHandle;

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn shape_with_production_font_repository(
        text: &str,
        config: rterm_fonts::FontConfig,
    ) -> rterm_fonts::ShapedRow {
        let mut repository = PlatformFontRepository::production_index();
        let mut catalog = repository
            .build_catalog(production_font_catalog_mode())
            .expect("production font catalog");
        repository
            .preflight_text(text, &mut catalog)
            .expect("preflight production font coverage");
        let mut shaper = TerminalShaper::new(config);
        let mut row = shaper
            .shape_row(&mut catalog, text)
            .expect("shape production font sample");
        let missing = row
            .clusters
            .iter()
            .filter(|cluster| cluster.is_tofu)
            .flat_map(|cluster| row.text[cluster.byte_range.clone()].chars())
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            repository
                .activate_missing_glyphs(&missing, &mut catalog)
                .expect("activate one late production fallback batch");
            row = shaper
                .shape_row(&mut catalog, text)
                .expect("reshape after one late production fallback batch");
        }
        assert_eq!(row.catalog_generation, catalog.generation());
        row
    }

    fn construct_window_gpu_from_owned_display(
        display: OwnedDisplayHandle,
        window: Arc<Window>,
        size: PhysicalSize<u32>,
    ) {
        drop(WindowGpu::new(display, window, size, false, false));
    }

    fn prepare_window_gpu_from_owned_display(
        display: OwnedDisplayHandle,
        window: Arc<Window>,
        size: PhysicalSize<u32>,
    ) {
        drop(WindowGpu::prepare(display, window, size, false, false));
    }

    fn prepare_window_gpu_with_diagnostic_backend_from_owned_display(
        display: OwnedDisplayHandle,
        window: Arc<Window>,
        size: PhysicalSize<u32>,
        backend: Option<DiagnosticGpuBackend>,
    ) {
        drop(WindowGpu::prepare_with_diagnostic_backend(
            display, window, size, false, false, backend,
        ));
    }

    fn prepare_window_gpu_with_diagnostic_font_options_from_owned_display(
        display: OwnedDisplayHandle,
        window: Arc<Window>,
        size: PhysicalSize<u32>,
        backend: Option<DiagnosticGpuBackend>,
        font_mode: Option<rterm_diagnostics::DiagnosticFontMode>,
        font_specimen: Option<rterm_diagnostics::DiagnosticFontSpecimen>,
    ) {
        drop(WindowGpu::prepare_with_diagnostic_options(
            display,
            window,
            size,
            false,
            false,
            backend,
            font_mode,
            font_specimen,
        ));
    }

    fn finish_prepared_window_gpu(prepared: PreparedWindowGpu) {
        drop(WindowGpu::finish_prepared(prepared));
    }

    #[test]
    fn window_gpu_constructor_accepts_owned_display_handle() {
        std::hint::black_box(
            construct_window_gpu_from_owned_display
                as fn(OwnedDisplayHandle, Arc<Window>, PhysicalSize<u32>),
        );
    }

    #[test]
    fn window_gpu_surface_preparation_and_worker_initialization_are_split() {
        fn assert_send<T: Send>() {}

        assert_send::<PreparedWindowGpu>();
        std::hint::black_box(
            prepare_window_gpu_from_owned_display
                as fn(OwnedDisplayHandle, Arc<Window>, PhysicalSize<u32>),
        );
        std::hint::black_box(finish_prepared_window_gpu as fn(PreparedWindowGpu));
    }

    #[test]
    fn diagnostic_gpu_backend_prepare_accepts_explicit_selection_without_running_hardware() {
        std::hint::black_box(
            prepare_window_gpu_with_diagnostic_backend_from_owned_display
                as fn(
                    OwnedDisplayHandle,
                    Arc<Window>,
                    PhysicalSize<u32>,
                    Option<DiagnosticGpuBackend>,
                ),
        );
    }

    #[test]
    fn diagnostic_gpu_backend_builds_hardware_free_context_options() {
        let native_default = GpuContextOptions::default();
        let unselected = gpu_context_options(true, true, None).expect("native options");
        assert_eq!(unselected.backends, native_default.backends);
        assert_eq!(
            unselected.power_preference,
            native_default.with_high_performance(true).power_preference
        );
        assert!(unselected.force_fallback_adapter);

        for backend in [
            DiagnosticGpuBackend::Dx12,
            DiagnosticGpuBackend::Vulkan,
            DiagnosticGpuBackend::Gl,
        ] {
            let selected = gpu_context_options(false, false, Some(backend))
                .expect("supported diagnostic backend");
            let expected = native_default
                .with_only_backend_name(backend.as_str())
                .expect("known backend");
            assert_eq!(selected.backends, expected.backends);
            assert_eq!(selected.backends.bits().count_ones(), 1);
            assert_eq!(selected.power_preference, native_default.power_preference);
            assert!(!selected.force_fallback_adapter);
        }
    }

    #[cfg(feature = "diagnostic-tools")]
    #[test]
    #[cfg(not(feature = "rterm-legacy-0-1"))]
    fn diagnostic_font_mode_maps_exactly_without_changing_production_default() {
        use rterm_diagnostics::DiagnosticFontMode;

        assert_eq!(
            diagnostic_font_catalog_mode(None),
            production_font_catalog_mode()
        );
        assert_eq!(
            diagnostic_font_catalog_mode(Some(DiagnosticFontMode::CurrentCopied)),
            FontCatalogMode::CurrentCopied
        );
        assert_eq!(
            diagnostic_font_catalog_mode(Some(DiagnosticFontMode::SharedAll)),
            FontCatalogMode::SharedAll
        );
        assert_eq!(
            diagnostic_font_catalog_mode(Some(DiagnosticFontMode::Lazy)),
            FontCatalogMode::Lazy
        );
    }

    #[cfg(feature = "diagnostic-tools")]
    #[test]
    #[cfg(not(feature = "rterm-legacy-0-1"))]
    fn stage7_attribution_ready_marker_owns_complete_adapter_identity() {
        let mut metrics = GpuPresentationMetrics::uninitialized();
        metrics.backend = "Vulkan".to_owned();
        metrics.adapter_name = "fixture-adapter".to_owned();
        metrics.adapter_vendor_id = 4318;
        metrics.adapter_device_id = 9860;
        metrics.adapter_type = "discrete-gpu".to_owned();

        let extra = stage7_attribution_identity_extra(&metrics);
        let mut keys = extra.keys().map(String::as_str).collect::<Vec<_>>();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "gpu_adapter_device_id",
                "gpu_adapter_type",
                "gpu_adapter_vendor_id",
            ]
        );
        assert_eq!(extra["gpu_adapter_vendor_id"], 4318);
        assert_eq!(extra["gpu_adapter_device_id"], 9860);
        assert_eq!(extra["gpu_adapter_type"], "discrete-gpu");
    }

    #[test]
    fn diagnostic_font_mode_prepare_accepts_typed_options_without_running_hardware() {
        std::hint::black_box(
            prepare_window_gpu_with_diagnostic_font_options_from_owned_display
                as fn(
                    OwnedDisplayHandle,
                    Arc<Window>,
                    PhysicalSize<u32>,
                    Option<DiagnosticGpuBackend>,
                    Option<rterm_diagnostics::DiagnosticFontMode>,
                    Option<rterm_diagnostics::DiagnosticFontSpecimen>,
                ),
        );
    }

    #[cfg(feature = "diagnostic-tools")]
    #[test]
    #[cfg(not(feature = "rterm-legacy-0-1"))]
    fn diagnostic_font_specimens_activate_one_bounded_batch_without_tofu() {
        use rterm_diagnostics::{DiagnosticFontMode, DiagnosticFontSpecimen};

        for specimen in [DiagnosticFontSpecimen::Cjk, DiagnosticFontSpecimen::Emoji] {
            let repository = PlatformFontRepository::production_index_for_os("test");
            let (_repository, _catalog, summary) = prepare_diagnostic_font_catalog(
                repository,
                Some(DiagnosticFontMode::Lazy),
                Some(specimen),
            )
            .expect("prepare bounded specimen");
            let summary = summary.expect("diagnostic resource summary");
            assert_eq!(summary.mode, DiagnosticFontMode::Lazy);
            assert_eq!(summary.specimen, specimen);
            assert_eq!(summary.catalog_builds, 2);
            assert_eq!(summary.generation, 2);
            assert_eq!(summary.tofu_count, 0, "specimen {specimen:?}");
            assert_eq!(
                summary.recovery_retained_source_bytes,
                summary.retained_source_bytes
            );
            assert_eq!(summary.recovery_generation, summary.generation);
        }
        assert_eq!(
            diagnostic_font_specimen_text(DiagnosticFontSpecimen::Emoji),
            "😀"
        );
    }

    #[cfg(feature = "diagnostic-tools")]
    #[test]
    #[cfg(not(feature = "rterm-legacy-0-1"))]
    fn diagnostic_font_resource_summary_serializes_irreversible_identity_without_paths() {
        use rterm_diagnostics::{DiagnosticFontMode, DiagnosticFontSpecimen};

        let repository = PlatformFontRepository::production_index_for_os("test");
        let (_, _, summary) = prepare_diagnostic_font_catalog(
            repository,
            Some(DiagnosticFontMode::SharedAll),
            Some(DiagnosticFontSpecimen::Ascii),
        )
        .expect("prepare shared proof catalog");
        let value = serde_json::to_value(summary.expect("resource summary")).unwrap();
        let rendered = value.to_string();

        assert_eq!(value["mode"], "shared");
        assert_eq!(value["specimen"], "ascii");
        for key in ["index_fingerprint_sha256", "catalog_fingerprint_sha256"] {
            assert_eq!(value[key].as_str().expect("digest").len(), 64);
        }
        assert!(!rendered.contains(r"C:\Windows\Fonts"));
        assert!(!rendered.to_ascii_lowercase().contains("path"));
    }

    #[cfg(feature = "diagnostic-tools")]
    #[test]
    #[cfg(not(feature = "rterm-legacy-0-1"))]
    fn diagnostic_font_frame_evidence_is_derived_from_the_presented_specimen_frame() {
        use rterm_diagnostics::{DiagnosticFontMode, DiagnosticFontSpecimen};

        let repository = PlatformFontRepository::production_index_for_os("test");
        let (_, _, summary) = prepare_diagnostic_font_catalog(
            repository,
            Some(DiagnosticFontMode::Lazy),
            Some(DiagnosticFontSpecimen::Cjk),
        )
        .expect("prepare summary");
        let mut summary = summary.expect("font summary");

        let generation = summary.generation;
        record_diagnostic_font_frame_evidence(&mut summary, generation, &['中']);
        assert_eq!(summary.frame_catalog_generation, Some(summary.generation));
        assert_eq!(summary.frame_generation_consistent, Some(true));
        assert_eq!(summary.tofu_count, 1);
        record_diagnostic_font_frame_evidence(&mut summary, generation, &[]);
        assert_eq!(summary.tofu_count, 0);
        let mixed_generation = summary.generation.saturating_add(1);
        record_diagnostic_font_frame_evidence(&mut summary, mixed_generation, &[]);
        assert_eq!(summary.frame_generation_consistent, Some(false));
    }

    #[cfg(feature = "diagnostic-tools")]
    #[test]
    #[cfg(not(feature = "rterm-legacy-0-1"))]
    fn diagnostic_font_summary_finalizes_only_the_first_presented_proof_frame() {
        use std::cell::Cell;

        use rterm_diagnostics::{DiagnosticFontMode, DiagnosticFontSpecimen};

        let (_, _, summary) = prepare_diagnostic_font_catalog(
            PlatformFontRepository::production_index_for_os("test"),
            Some(DiagnosticFontMode::Lazy),
            Some(DiagnosticFontSpecimen::Ascii),
        )
        .expect("prepare diagnostic catalog");
        let mut summary = summary;
        let finalizations = Cell::new(0_u32);

        for generation in [1_u64, 2] {
            finalize_diagnostic_font_resource_summary_once(&mut summary, |resources| {
                finalizations.set(finalizations.get() + 1);
                resources.frame_catalog_generation = Some(generation);
                Ok(())
            })
            .expect("finalize proof summary");
        }

        assert_eq!(finalizations.get(), 1);
        assert_eq!(
            summary
                .as_ref()
                .and_then(|resources| resources.frame_catalog_generation),
            Some(1)
        );
    }

    #[cfg(feature = "diagnostic-tools")]
    #[test]
    #[cfg(not(feature = "rterm-legacy-0-1"))]
    fn diagnostic_font_summary_is_rederived_from_the_actual_presented_catalog_epoch() {
        use rterm_diagnostics::{DiagnosticFontMode, DiagnosticFontSpecimen};

        let mode = DiagnosticFontMode::Lazy;
        let specimen = DiagnosticFontSpecimen::Emoji;
        let (mut repository, catalog, summary) = prepare_diagnostic_font_catalog(
            PlatformFontRepository::production_index_for_os("test"),
            Some(mode),
            Some(specimen),
        )
        .expect("prepare diagnostic catalog");
        let mut summary = summary.expect("diagnostic resource summary");
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless adapter");
        let mut renderer =
            GpuLayerRenderer::new_headless(&context, 64 * 1024).expect("GPU layer renderer");
        renderer
            .enable_text(
                catalog,
                bundled_emergency_font_config(),
                GpuTextConfig::new(
                    4 * 1024 * 1024,
                    rterm_fonts::RasterCacheConfig::new(4 * 1024 * 1024),
                ),
            )
            .expect("GPU text");
        let mut terminal = Terminal::new(TerminalSize::new(16, 1));
        terminal.feed(diagnostic_font_specimen_text(specimen).as_bytes());
        let report = prepare_gpu_text_frame(
            &mut repository,
            &mut renderer,
            &TerminalRenderSnapshot::from_terminal(&terminal),
            RenderGeometry::new(16 * 16, 24, 16, 24),
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("prepare actual specimen frame");

        summary.retained_source_bytes = 0;
        summary.active_source_count = 0;
        summary.catalog_builds = 0;
        summary.generation = 0;
        summary.recovery_retained_source_bytes = 0;
        summary.recovery_generation = 0;
        summary.catalog_fingerprint_sha256.clear();
        summary.ordered_catalog_fingerprint_sha256.clear();
        finalize_diagnostic_font_resource_summary(
            &repository,
            FontCatalogMode::Lazy,
            &mut renderer,
            &report,
            &mut summary,
        )
        .expect("derive final evidence from the presented catalog epoch");

        let diagnostics = repository.diagnostics().expect("modern font diagnostics");
        let catalog = renderer.text_catalog_mut().expect("actual catalog");
        let metrics = catalog.memory_metrics();
        assert_eq!(summary.retained_source_bytes, metrics.retained_source_bytes);
        assert_eq!(summary.active_source_count, metrics.active_source_count);
        assert_eq!(summary.initial_catalog_source_count, 1);
        assert_eq!(summary.catalog_builds, metrics.catalog_builds);
        assert_eq!(summary.generation, catalog.generation());
        assert_eq!(
            summary.indexed_source_count,
            diagnostics.indexed_source_count
        );
        assert_eq!(summary.frame_catalog_generation, Some(catalog.generation()));
        assert_eq!(summary.frame_generation_consistent, Some(true));
        assert_eq!(summary.tofu_count, 0);
        assert_eq!(
            summary.recovery_retained_source_bytes,
            summary.retained_source_bytes
        );
        assert_eq!(summary.recovery_generation, summary.generation);
        assert_eq!(summary.catalog_fingerprint_sha256.len(), 64);
        assert_eq!(summary.ordered_catalog_fingerprint_sha256.len(), 64);
    }

    #[test]
    fn native_presentation_mode_is_the_only_gpu_damage_selector() {
        let damage = [DamageRegion::new(1, 2, 3, 4)];

        assert!(presentation_damage(rterm_native::RenderMode::Full, &damage).is_empty());
        assert_eq!(
            presentation_damage(rterm_native::RenderMode::Damage, &damage),
            damage
        );
    }

    #[test]
    fn production_fonts_render_covered_scripts_in_one_generation_without_tofu() {
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless adapter");
        let mut repository = PlatformFontRepository::production_index_for_os("test");
        let catalog = repository
            .build_catalog(production_font_catalog_mode())
            .expect("production lazy catalog");
        let mut renderer = test_headless_renderer(&context).expect("GPU layer renderer");
        renderer
            .enable_text(
                catalog,
                bundled_emergency_font_config(),
                GpuTextConfig::new(
                    4 * 1024 * 1024,
                    rterm_fonts::RasterCacheConfig::new(4 * 1024 * 1024),
                ),
            )
            .expect("GPU text");
        let mut terminal = Terminal::new(TerminalSize::new(96, 1));
        terminal.feed("ASCII 中文 😀 سلام क्षि אבג".as_bytes());

        let report = prepare_gpu_text_frame(
            &mut repository,
            &mut renderer,
            &TerminalRenderSnapshot::from_terminal(&terminal),
            RenderGeometry::new(96 * 16, 24, 16, 24),
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("prepare covered production scripts");

        assert!(report.missing_glyphs.is_empty());
        assert_eq!(report.prepared_rows, [0]);
        #[cfg(not(feature = "rterm-legacy-0-1"))]
        {
            assert_eq!(
                report.catalog_generation,
                repository
                    .diagnostics()
                    .expect("modern font diagnostics")
                    .generation
            );
            assert_eq!(
                repository
                    .diagnostics()
                    .expect("modern font diagnostics")
                    .active_source_count,
                6
            );
        }
        #[cfg(feature = "rterm-legacy-0-1")]
        {
            assert_eq!(
                renderer
                    .text_catalog_mut()
                    .expect("live catalog")
                    .face_count(),
                6
            );
            assert!(repository.diagnostics().is_err());
        }
    }

    #[test]
    fn production_fonts_keep_one_generation_per_frame_and_allow_at_most_one_restart() {
        assert_eq!(production_font_catalog_mode(), FontCatalogMode::Lazy);
        let damage = [DamageRegion::new(1, 2, 3, 4)];
        let attempts = RefCell::new(Vec::new());
        let prepared = prepare_catalog_frame_with_one_restart(
            1,
            &damage,
            |generation, frame_damage, can_expand| {
                attempts
                    .borrow_mut()
                    .push((generation, frame_damage.to_vec(), can_expand));
                if can_expand {
                    Ok(CatalogFrameAttempt::Expanded(2))
                } else {
                    Ok(CatalogFrameAttempt::Prepared(generation))
                }
            },
        )
        .expect("one whole-frame restart");
        assert_eq!(prepared, 2);
        assert_eq!(
            attempts.into_inner(),
            vec![(1, damage.to_vec(), true), (2, Vec::new(), false)]
        );

        let attempt_count = Cell::new(0_u8);
        let error = prepare_catalog_frame_with_one_restart(
            1,
            &damage,
            |generation, _frame_damage, _can_expand| {
                attempt_count.set(attempt_count.get().saturating_add(1));
                Ok::<_, Box<dyn Error>>(CatalogFrameAttempt::<()>::Expanded(
                    generation.saturating_add(1),
                ))
            },
        )
        .expect_err("a second expansion in one frame must fail closed");
        assert_eq!(attempt_count.get(), 2);
        assert_eq!(
            error.to_string(),
            "GPU font catalog expanded twice during one presented frame"
        );
    }

    #[test]
    fn gpu_text_frame_late_missing_activates_next_source_and_restarts_without_tofu() {
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless adapter");
        let mut repository = PlatformFontRepository::late_missing_fixture();
        let catalog = repository
            .build_catalog(FontCatalogMode::Lazy)
            .expect("primary catalog");
        let mut renderer = test_headless_renderer(&context).expect("GPU layer renderer");
        renderer
            .enable_text(
                catalog,
                bundled_emergency_font_config(),
                GpuTextConfig::new(
                    4 * 1024 * 1024,
                    rterm_fonts::RasterCacheConfig::new(4 * 1024 * 1024),
                ),
            )
            .expect("GPU text");
        let geometry = RenderGeometry::new(8 * 16, 2 * 24, 16, 24);
        let paint = TextPaintConfig::default();

        let mut priming_terminal = Terminal::new(TerminalSize::new(8, 2));
        priming_terminal.feed(b"ASCII\r\nrow");
        prepare_gpu_text_frame(
            &mut repository,
            &mut renderer,
            &TerminalRenderSnapshot::from_terminal(&priming_terminal),
            geometry,
            &[],
            &paint,
            1.0,
            1.0,
        )
        .expect("prime generation one rows");

        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed("ASCII\r\n中文".as_bytes());
        let report = prepare_gpu_text_frame(
            &mut repository,
            &mut renderer,
            &TerminalRenderSnapshot::from_terminal(&terminal),
            geometry,
            &[DamageRegion::new(0, 1, 2, 1)],
            &paint,
            1.0,
            1.0,
        )
        .expect("late missing source expansion and one full-frame retry");

        #[cfg(not(feature = "rterm-legacy-0-1"))]
        assert_eq!(report.catalog_generation, 3);
        #[cfg(feature = "rterm-legacy-0-1")]
        assert_eq!(
            renderer
                .text_catalog_mut()
                .expect("live catalog")
                .generation(),
            3
        );
        assert_eq!(report.prepared_rows, [0, 1]);
        assert!(report.missing_glyphs.is_empty());
        #[cfg(not(feature = "rterm-legacy-0-1"))]
        {
            let diagnostics = repository.diagnostics().expect("modern font diagnostics");
            assert_eq!(diagnostics.active_source_count, 3);
            assert_eq!(diagnostics.generation, 3);
        }
        #[cfg(feature = "rterm-legacy-0-1")]
        assert_eq!(
            renderer
                .text_catalog_mut()
                .expect("live catalog")
                .face_count(),
            3
        );
    }

    #[test]
    fn gpu_text_frame_static_missing_fallback_requests_a_full_followup_until_exhausted() {
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless adapter");
        let mut repository = PlatformFontRepository::repeated_late_missing_fixture();
        let catalog = repository
            .build_catalog(FontCatalogMode::Lazy)
            .expect("primary catalog");
        let mut renderer = test_headless_renderer(&context).expect("GPU layer renderer");
        renderer
            .enable_text(
                catalog,
                bundled_emergency_font_config(),
                GpuTextConfig::new(
                    4 * 1024 * 1024,
                    rterm_fonts::RasterCacheConfig::new(4 * 1024 * 1024),
                ),
            )
            .expect("GPU text");
        let mut terminal = Terminal::new(TerminalSize::new(8, 2));
        terminal.feed("中文\r\nASCII".as_bytes());
        let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
        let geometry = RenderGeometry::new(8 * 16, 2 * 24, 16, 24);
        let paint = TextPaintConfig::default();
        let first = prepare_gpu_text_frame(
            &mut repository,
            &mut renderer,
            &snapshot,
            geometry,
            &[],
            &paint,
            1.0,
            1.0,
        )
        .expect("one bounded restart leaves another candidate");
        #[cfg(not(feature = "rterm-legacy-0-1"))]
        assert_eq!(first.catalog_generation, 3);
        #[cfg(feature = "rterm-legacy-0-1")]
        assert_eq!(
            renderer
                .text_catalog_mut()
                .expect("live catalog")
                .generation(),
            3
        );
        assert!(!first.missing_glyphs.is_empty());
        assert!(font_fallback_redraw_needed(&repository, &first));
        let followup = prepare_gpu_text_frame(
            &mut repository,
            &mut renderer,
            &snapshot,
            geometry,
            &[],
            &paint,
            1.0,
            1.0,
        )
        .expect("explicit full redraw without input or animation");
        #[cfg(not(feature = "rterm-legacy-0-1"))]
        assert_eq!(followup.catalog_generation, 4);
        #[cfg(feature = "rterm-legacy-0-1")]
        assert_eq!(
            renderer
                .text_catalog_mut()
                .expect("live catalog")
                .generation(),
            4
        );
        assert!(followup.missing_glyphs.is_empty());
        assert_eq!(followup.prepared_rows, [0, 1]);
        assert!(!font_fallback_redraw_needed(&repository, &followup));
        // Even a report that still contains classified CJK misses cannot
        // schedule forever once every CJK candidate has been consumed.
        assert!(!font_fallback_redraw_needed(&repository, &first));

        terminal.feed("\r\n\u{10ffff}".as_bytes());
        let missing = prepare_gpu_text_frame(
            &mut repository,
            &mut renderer,
            &TerminalRenderSnapshot::from_terminal(&terminal),
            geometry,
            &[],
            &paint,
            1.0,
            1.0,
        )
        .expect("unsupported glyph remains stable");
        assert!(!missing.missing_glyphs.is_empty());
        assert!(!font_fallback_redraw_needed(&repository, &missing));
    }

    #[test]
    fn gpu_text_frame_second_late_missing_stays_tofu_without_a_second_expansion() {
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless adapter");
        let mut repository = PlatformFontRepository::repeated_late_missing_fixture();
        let catalog = repository
            .build_catalog(FontCatalogMode::Lazy)
            .expect("primary catalog");
        let mut renderer = test_headless_renderer(&context).expect("GPU layer renderer");
        renderer
            .enable_text(
                catalog,
                bundled_emergency_font_config(),
                GpuTextConfig::new(
                    4 * 1024 * 1024,
                    rterm_fonts::RasterCacheConfig::new(4 * 1024 * 1024),
                ),
            )
            .expect("GPU text");
        let mut terminal = Terminal::new(TerminalSize::new(4, 1));
        terminal.feed("中文".as_bytes());

        let report = prepare_gpu_text_frame(
            &mut repository,
            &mut renderer,
            &TerminalRenderSnapshot::from_terminal(&terminal),
            RenderGeometry::new(4 * 16, 24, 16, 24),
            &[],
            &TextPaintConfig::default(),
            1.0,
            1.0,
        )
        .expect("one late expansion followed by stable tofu");

        #[cfg(not(feature = "rterm-legacy-0-1"))]
        assert_eq!(report.catalog_generation, 3);
        #[cfg(feature = "rterm-legacy-0-1")]
        assert_eq!(
            renderer
                .text_catalog_mut()
                .expect("live catalog")
                .generation(),
            3
        );
        assert_eq!(report.missing_glyphs, ['中', '文']);
        #[cfg(not(feature = "rterm-legacy-0-1"))]
        {
            let diagnostics = repository.diagnostics().expect("modern font diagnostics");
            assert_eq!(diagnostics.active_source_count, 3);
            assert_eq!(diagnostics.generation, 3);
        }
        #[cfg(feature = "rterm-legacy-0-1")]
        assert_eq!(
            renderer
                .text_catalog_mut()
                .expect("live catalog")
                .face_count(),
            3
        );
    }

    #[test]
    #[cfg(not(feature = "rterm-legacy-0-1"))]
    fn gpu_text_frame_invalid_late_fallback_discards_partial_state_before_error() {
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless adapter");
        let mut repository = PlatformFontRepository::invalid_late_missing_fixture();
        let catalog = repository
            .build_catalog(FontCatalogMode::Lazy)
            .expect("primary catalog");
        let mut renderer =
            GpuLayerRenderer::new_headless(&context, 64 * 1024).expect("GPU layer renderer");
        renderer
            .enable_text(
                catalog,
                bundled_emergency_font_config(),
                GpuTextConfig::new(
                    4 * 1024 * 1024,
                    rterm_fonts::RasterCacheConfig::new(4 * 1024 * 1024),
                ),
            )
            .expect("GPU text");
        let mut terminal = Terminal::new(TerminalSize::new(4, 1));
        terminal.feed("中文".as_bytes());
        let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);

        for _ in 0..2 {
            let error = prepare_gpu_text_frame(
                &mut repository,
                &mut renderer,
                &snapshot,
                RenderGeometry::new(4 * 16, 24, 16, 24),
                &[],
                &TextPaintConfig::default(),
                1.0,
                1.0,
            )
            .expect_err("readable invalid late fallback must fail");
            let cpu = renderer.text_cpu_font_metrics().expect("font metrics");
            assert_eq!(cpu.row_cache_entries, 0);
            assert_eq!(cpu.payload_bytes, 0);
            let atlas = renderer.text_atlas_metrics().expect("atlas metrics");
            assert_eq!(atlas.entries, 0);
            assert_eq!(atlas.payload_bytes, 0);
            assert!(error.to_string().contains("late font activation failed"));
        }
    }

    #[test]
    #[cfg(not(feature = "rterm-legacy-0-1"))]
    fn recovery_retires_real_cpu_font_state_before_building_the_replacement() {
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless adapter");
        let mut repository = PlatformFontRepository::late_missing_fixture();
        let catalog = repository
            .build_catalog(production_font_catalog_mode())
            .expect("active production catalog");
        let mut lost = GpuLayerRenderer::new_headless(&context, 64 * 1024).expect("lost renderer");
        lost.enable_text(
            catalog,
            bundled_emergency_font_config(),
            GpuTextConfig::new(
                4 * 1024 * 1024,
                rterm_fonts::RasterCacheConfig::new(4 * 1024 * 1024),
            ),
        )
        .expect("lost text state");
        let old_bytes = lost
            .text_cpu_font_metrics()
            .expect("lost font metrics")
            .catalog
            .retained_source_bytes;
        assert!(old_bytes > 0);
        let peak = Cell::new(old_bytes);

        let replacement = retire_lost_renderer_before_rebuild(&mut lost, |retired| {
            let retired_metrics = retired.text_cpu_font_metrics().expect("retired metrics");
            assert_eq!(retired_metrics.catalog.retained_source_bytes, 0);
            let recovered_catalog = repository
                .rebuild_catalog_from_active(production_font_catalog_mode())
                .expect("recovered active catalog");
            let mut replacement =
                GpuLayerRenderer::new_headless(&context, 64 * 1024).expect("replacement renderer");
            replacement
                .enable_text(
                    recovered_catalog,
                    bundled_emergency_font_config(),
                    GpuTextConfig::new(
                        4 * 1024 * 1024,
                        rterm_fonts::RasterCacheConfig::new(4 * 1024 * 1024),
                    ),
                )
                .expect("replacement text state");
            let replacement_bytes = replacement
                .text_cpu_font_metrics()
                .expect("replacement metrics")
                .catalog
                .retained_source_bytes;
            peak.set(peak.get().max(replacement_bytes));
            Ok::<_, Box<dyn Error>>(replacement)
        })
        .expect("retire then rebuild");

        assert_eq!(
            lost.text_cpu_font_metrics()
                .expect("retired metrics")
                .catalog
                .retained_source_bytes,
            0
        );
        assert!(
            replacement
                .text_cpu_font_metrics()
                .expect("replacement metrics")
                .catalog
                .retained_source_bytes
                > 0
        );
        assert_eq!(peak.get(), old_bytes, "old and new bytes must not overlap");

        let mut failed = replacement;
        let error = retire_lost_renderer_before_rebuild(&mut failed, |retired| {
            assert_eq!(
                retired
                    .text_cpu_font_metrics()
                    .expect("failed retired metrics")
                    .catalog
                    .retained_source_bytes,
                0
            );
            Err::<GpuLayerRenderer, _>(io::Error::other("injected replacement failure").into())
        })
        .expect_err("replacement failure must remain fail closed");
        assert!(error.to_string().contains("injected replacement failure"));
        assert_eq!(
            failed
                .text_cpu_font_metrics()
                .expect("failed renderer metrics")
                .catalog
                .active_source_count,
            0
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn emergency_font_catalog_covers_reviewed_cli_ui_fixtures() {
        let row = shape_with_production_font_repository(
            "中文 سلام क्षि אבג 😀 ×▾—□…",
            bundled_emergency_font_config(),
        );

        assert!(
            row.clusters.iter().all(|cluster| !cluster.is_tofu),
            "emergency GPU font catalog must not render reviewed UI fixtures as tofu: {:?}",
            row.clusters
                .iter()
                .filter(|cluster| cluster.is_tofu)
                .map(|cluster| &row.text[cluster.byte_range.clone()])
                .collect::<Vec<_>>()
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn emergency_font_catalog_covers_mac_cjk_fallback() {
        let row = shape_with_production_font_repository(
            "中文显示测试 日本語繁體字",
            bundled_emergency_font_config(),
        );

        assert!(
            row.clusters.iter().all(|cluster| !cluster.is_tofu),
            "macOS system CJK fallback must not render Chinese or Japanese as tofu: {:?}",
            row.clusters
                .iter()
                .filter(|cluster| cluster.is_tofu)
                .map(|cluster| &row.text[cluster.byte_range.clone()])
                .collect::<Vec<_>>()
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn modern_default_font_prefers_cascadia_mono_and_preserves_terminal_cells() {
        let config = bundled_emergency_font_config();
        assert_eq!(config.primary(), "Cascadia Mono");

        let row = shape_with_production_font_repository("R-Term 你好 😀", config);

        assert!((row.metrics.font_size - 17.0).abs() < f32::EPSILON);
        assert_eq!(
            row.clusters
                .iter()
                .map(|cluster| cluster.cell_span.clone())
                .collect::<Vec<_>>(),
            vec![
                0..1,
                1..2,
                2..3,
                3..4,
                4..5,
                5..6,
                6..8,
                8..10,
                10..11,
                11..13,
            ],
            "shaping must preserve the terminal's logical cell spans"
        );

        for cluster in &row.clusters {
            let text = &row.text[cluster.byte_range.clone()];
            if text
                .chars()
                .all(|character| character.is_ascii() && character != ' ')
            {
                assert_eq!(
                    cluster.font_family, "Cascadia Mono",
                    "Latin terminal glyphs should use the preferred monospace face"
                );
            }
            if text.chars().any(|character| "你好".contains(character)) {
                assert!(
                    !cluster.is_tofu,
                    "CJK fallback must resolve a visible glyph"
                );
            }
        }
    }

    #[test]
    fn native_app_manifest_has_no_pixels_runtime_dependency() {
        let manifest: toml::Value =
            toml::from_str(include_str!("../Cargo.toml")).expect("parse rterm-app manifest");
        assert!(
            manifest["dependencies"].get("pixels").is_none(),
            "the promoted native renderer must not depend on the Pixels compatibility frontend"
        );
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum TestPresentError {
        DeviceLost,
        Validation,
    }

    #[test]
    fn window_gpu_releases_every_renderer_before_its_context() {
        #[derive(Debug)]
        struct TrackedDrop {
            label: &'static str,
            drops: Rc<RefCell<Vec<&'static str>>>,
        }

        impl Drop for TrackedDrop {
            fn drop(&mut self) {
                self.drops.borrow_mut().push(self.label);
            }
        }

        let drops = Rc::new(RefCell::new(Vec::new()));
        let mut renderer = Some(TrackedDrop {
            label: "active-renderer",
            drops: Rc::clone(&drops),
        });
        let mut retired_renderers = vec![TrackedDrop {
            label: "retired-renderer",
            drops: Rc::clone(&drops),
        }];
        let mut context = Some(TrackedDrop {
            label: "context",
            drops: Rc::clone(&drops),
        });

        release_renderers_before_context(&mut renderer, &mut retired_renderers, &mut context);

        assert_eq!(
            drops.borrow().as_slice(),
            ["active-renderer", "retired-renderer", "context"]
        );
        assert!(renderer.is_none());
        assert!(retired_renderers.is_empty());
        assert!(context.is_none());
    }

    #[test]
    fn stage7_native_close_retains_only_windows_nvidia_vulkan_gpu_owners() {
        #[derive(Debug)]
        struct TrackedDrop {
            label: &'static str,
            drops: Rc<RefCell<Vec<&'static str>>>,
        }

        impl Drop for TrackedDrop {
            fn drop(&mut self) {
                self.drops.borrow_mut().push(self.label);
            }
        }

        let retained_drops = Rc::new(RefCell::new(Vec::new()));
        let mut retained_renderer = Some(TrackedDrop {
            label: "renderer",
            drops: Rc::clone(&retained_drops),
        });
        let mut retained_device = Some(TrackedDrop {
            label: "device",
            drops: Rc::clone(&retained_drops),
        });
        let mut retained_context = Some(TrackedDrop {
            label: "context",
            drops: Rc::clone(&retained_drops),
        });

        assert!(finalize_stage7_native_gpu_owners(
            "windows",
            "Vulkan",
            0x10de,
            &mut retained_renderer,
            &mut retained_device,
            &mut retained_context,
        ));
        assert!(retained_drops.borrow().is_empty());
        assert!(retained_renderer.is_none());
        assert!(retained_device.is_none());
        assert!(retained_context.is_none());

        let released_drops = Rc::new(RefCell::new(Vec::new()));
        let mut released_renderer = Some(TrackedDrop {
            label: "renderer",
            drops: Rc::clone(&released_drops),
        });
        let mut released_device = Some(TrackedDrop {
            label: "device",
            drops: Rc::clone(&released_drops),
        });
        let mut released_context = Some(TrackedDrop {
            label: "context",
            drops: Rc::clone(&released_drops),
        });

        assert!(!finalize_stage7_native_gpu_owners(
            "windows",
            "Dx12",
            0x10de,
            &mut released_renderer,
            &mut released_device,
            &mut released_context,
        ));
        assert_eq!(
            released_drops.borrow().as_slice(),
            ["renderer", "context", "device"]
        );
        assert!(released_renderer.is_none());
        assert!(released_device.is_none());
        assert!(released_context.is_none());
    }

    #[test]
    fn abandonment_workaround_requires_proven_adapter_and_explicit_final_shutdown() {
        assert!(should_abandon_recovered_window_surface(
            "windows", "Vulkan", 0x10de, true, true
        ));
        assert!(should_abandon_recovered_window_surface(
            "windows", "vulkan", 0x10de, true, true
        ));
        for (os, backend, vendor, shutdown, replaced) in [
            ("linux", "Vulkan", 0x10de, true, true),
            ("windows", "Dx12", 0x10de, true, true),
            ("windows", "Vulkan", 0x1002, true, true),
            ("windows", "Vulkan", 0x10de, false, true),
            ("windows", "Vulkan", 0x10de, true, false),
        ] {
            assert!(
                !should_abandon_recovered_window_surface(os, backend, vendor, shutdown, replaced),
                "{os}/{backend}/{vendor:#x} shutdown={shutdown} replaced={replaced}"
            );
        }
    }

    #[test]
    fn native_close_abandons_only_windows_nvidia_vulkan_devices() {
        assert!(should_abandon_current_adapter_after_native_close(
            "windows", "Vulkan", 0x10de
        ));
        for (os, backend, vendor) in [
            ("linux", "Vulkan", 0x10de),
            ("windows", "Dx12", 0x10de),
            ("windows", "Vulkan", 0x1002),
        ] {
            assert!(!should_abandon_current_adapter_after_native_close(
                os, backend, vendor
            ));
        }
    }

    #[test]
    fn abandonment_eligibility_tracks_the_recovered_current_adapter() {
        let mut current = GpuPresentationMetrics::uninitialized();
        current.backend = "Vulkan".to_owned();
        current.adapter_vendor_id = 0x10de;
        assert!(should_apply_current_adapter_abandonment_workaround(
            "windows", &current, true, true
        ));

        current.adapter_vendor_id = 0x1002;
        assert!(
            !should_apply_current_adapter_abandonment_workaround("windows", &current, true, true),
            "initial NVIDIA must not remain cached after recovery selects non-NVIDIA"
        );

        let mut current = GpuPresentationMetrics::uninitialized();
        current.backend = "Vulkan".to_owned();
        current.adapter_vendor_id = 0x1002;
        assert!(!should_apply_current_adapter_abandonment_workaround(
            "windows", &current, true, true
        ));

        current.adapter_vendor_id = 0x10de;
        assert!(
            should_apply_current_adapter_abandonment_workaround("windows", &current, true, true),
            "initial non-NVIDIA must not suppress a recovered NVIDIA adapter"
        );
    }

    #[test]
    fn window_gpu_records_abandonment_only_when_exit_shutdown_actually_forgets_resources() {
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless context");
        let mut gpu = WindowGpu {
            context: Some(context),
            renderer: None,
            retired_renderers: Vec::new(),
            font_repository: PlatformFontRepository::production_index_for_os("test"),
            font_catalog_mode: production_font_catalog_mode(),
            recovery: DeviceRecoveryCoordinator::default(),
            report: None,
            rendered_frames: 0,
            replaced_device: true,
            final_metrics: None,
            diagnostic_font_resources: None,
            abandonment_workaround_adapter_match_override: Some(true),
            current_adapter_metrics_override: None,
            abandonment_workaround_os_override: None,
            #[cfg(debug_assertions)]
            test_device_loss_injected: false,
        };
        assert_eq!(gpu.metrics().abandoned_lost_surfaces, 0);
        gpu.recovery.pending = true;

        assert!(gpu.shutdown_for_window_close());
        assert_eq!(gpu.metrics().abandoned_lost_surfaces, 1);
        assert_eq!(gpu.metrics().device_recoveries, 0);
        assert_eq!(gpu.metrics().device_recovery_failures, 1);
        assert!(!gpu.shutdown_for_window_close());
        assert_eq!(gpu.metrics().abandoned_lost_surfaces, 1);

        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("second headless context");
        let mut ordinary_close = WindowGpu {
            context: Some(context),
            renderer: None,
            retired_renderers: Vec::new(),
            font_repository: PlatformFontRepository::production_index_for_os("test"),
            font_catalog_mode: production_font_catalog_mode(),
            recovery: DeviceRecoveryCoordinator::default(),
            report: None,
            rendered_frames: 0,
            replaced_device: true,
            final_metrics: None,
            diagnostic_font_resources: None,
            abandonment_workaround_adapter_match_override: Some(false),
            current_adapter_metrics_override: None,
            abandonment_workaround_os_override: None,
            #[cfg(debug_assertions)]
            test_device_loss_injected: false,
        };
        assert!(!ordinary_close.shutdown_for_window_close());
        assert_eq!(ordinary_close.metrics().abandoned_lost_surfaces, 0);
        assert!(ordinary_close.context.is_some());
        assert!(ordinary_close.final_metrics.is_none());
    }

    #[test]
    fn current_adapter_shutdown_path_is_idempotent_after_context_is_taken() {
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless context");
        let mut recovered_metrics = GpuPresentationMetrics::uninitialized();
        recovered_metrics.backend = "Vulkan".to_owned();
        recovered_metrics.adapter_vendor_id = 0x10de;
        let mut gpu = WindowGpu {
            context: Some(context),
            renderer: None,
            retired_renderers: Vec::new(),
            font_repository: PlatformFontRepository::production_index_for_os("test"),
            font_catalog_mode: production_font_catalog_mode(),
            recovery: DeviceRecoveryCoordinator::default(),
            report: None,
            rendered_frames: 0,
            replaced_device: true,
            final_metrics: None,
            diagnostic_font_resources: None,
            abandonment_workaround_adapter_match_override: None,
            current_adapter_metrics_override: Some(recovered_metrics),
            abandonment_workaround_os_override: Some("windows"),
            #[cfg(debug_assertions)]
            test_device_loss_injected: false,
        };

        assert!(gpu.shutdown_for_window_close());
        assert!(!gpu.shutdown_for_window_close());
        assert_eq!(gpu.metrics().abandoned_lost_surfaces, 1);
    }

    #[test]
    fn skipped_recovery_stays_pending_until_a_later_presented_frame_commits_once() {
        let frame = ();
        let attempts = Cell::new(0);
        let rebuilds = Cell::new(0);
        let recoveries = Cell::new(0);
        let failures = Cell::new(0);
        let mut coordinator = DeviceRecoveryCoordinator::default();
        let outcomes = RefCell::new(
            [
                Err(TestPresentError::DeviceLost),
                Ok(GpuFrameStatus::Skipped),
                Ok(GpuFrameStatus::Presented),
            ]
            .into_iter(),
        );

        let first = coordinator
            .present(
                &frame,
                |()| {
                    attempts.set(attempts.get() + 1);
                    outcomes.borrow_mut().next().expect("scripted outcome")
                },
                || {
                    rebuilds.set(rebuilds.get() + 1);
                    Ok(())
                },
                |error| *error == TestPresentError::DeviceLost,
                |status| *status == GpuFrameStatus::Presented,
                || recoveries.set(recoveries.get() + 1),
                || failures.set(failures.get() + 1),
            )
            .expect("skipped retry");
        assert_eq!(first, GpuFrameStatus::Skipped);
        assert!(coordinator.pending());
        assert_eq!(recoveries.get(), 0);
        assert_eq!(failures.get(), 0);

        let second = coordinator
            .present(
                &frame,
                |()| {
                    attempts.set(attempts.get() + 1);
                    outcomes.borrow_mut().next().expect("scripted outcome")
                },
                || {
                    rebuilds.set(rebuilds.get() + 1);
                    Ok(())
                },
                |error| *error == TestPresentError::DeviceLost,
                |status| *status == GpuFrameStatus::Presented,
                || recoveries.set(recoveries.get() + 1),
                || failures.set(failures.get() + 1),
            )
            .expect("later presented frame");
        assert_eq!(second, GpuFrameStatus::Presented);
        assert!(!coordinator.pending());
        assert_eq!(attempts.get(), 3);
        assert_eq!(rebuilds.get(), 1);
        assert_eq!(recoveries.get(), 1);
        assert_eq!(failures.get(), 0);
    }

    #[test]
    fn pending_recovery_fails_once_without_starting_a_second_rebuild() {
        let frame = ();
        let rebuilds = Cell::new(0);
        let recoveries = Cell::new(0);
        let failures = Cell::new(0);
        let mut coordinator = DeviceRecoveryCoordinator::default();
        let outcomes = RefCell::new(
            [
                Err(TestPresentError::DeviceLost),
                Ok(GpuFrameStatus::Skipped),
                Err(TestPresentError::DeviceLost),
            ]
            .into_iter(),
        );

        coordinator
            .present(
                &frame,
                |()| outcomes.borrow_mut().next().expect("scripted outcome"),
                || {
                    rebuilds.set(rebuilds.get() + 1);
                    Ok(())
                },
                |error| *error == TestPresentError::DeviceLost,
                |status| *status == GpuFrameStatus::Presented,
                || recoveries.set(recoveries.get() + 1),
                || failures.set(failures.get() + 1),
            )
            .expect("skipped retry");
        let failed = coordinator.present(
            &frame,
            |()| outcomes.borrow_mut().next().expect("scripted outcome"),
            || {
                rebuilds.set(rebuilds.get() + 1);
                Ok(())
            },
            |error| *error == TestPresentError::DeviceLost,
            |status| *status == GpuFrameStatus::Presented,
            || recoveries.set(recoveries.get() + 1),
            || failures.set(failures.get() + 1),
        );

        assert_eq!(failed, Err(TestPresentError::DeviceLost));
        assert!(!coordinator.pending());
        assert_eq!(rebuilds.get(), 1);
        assert_eq!(recoveries.get(), 0);
        assert_eq!(failures.get(), 1);
    }

    #[test]
    fn device_loss_recovery_rebuilds_once_and_retries_the_same_snapshot() {
        let mut terminal = Terminal::new(TerminalSize::new(2, 1));
        terminal.feed(b"ok");
        let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
        let expected_digest = terminal_snapshot_content_digest(&snapshot);
        let expected_identity = std::ptr::from_ref(&snapshot);
        let attempts = Cell::new(0_u32);
        let rebuilds = Cell::new(0_u32);
        let recoveries = Cell::new(0_u32);
        let retry_failures = Cell::new(0_u32);
        let observed = RefCell::new(Vec::new());

        let status = retry_device_lost_once(
            &snapshot,
            |borrowed| {
                observed.borrow_mut().push((
                    std::ptr::from_ref(borrowed),
                    terminal_snapshot_content_digest(borrowed),
                ));
                let attempt = attempts.get();
                attempts.set(attempt + 1);
                if attempt == 0 {
                    Err(TestPresentError::DeviceLost)
                } else {
                    Ok(GpuFrameStatus::Presented)
                }
            },
            || {
                rebuilds.set(rebuilds.get() + 1);
                Ok(())
            },
            |error| *error == TestPresentError::DeviceLost,
            |status| {
                if *status == GpuFrameStatus::Presented {
                    recoveries.set(recoveries.get() + 1);
                }
            },
            || retry_failures.set(retry_failures.get() + 1),
        )
        .expect("one device-loss retry");

        assert_eq!(status, GpuFrameStatus::Presented);
        assert_eq!(attempts.get(), 2);
        assert_eq!(rebuilds.get(), 1);
        assert_eq!(recoveries.get(), 1);
        assert_eq!(retry_failures.get(), 0);
        assert_eq!(
            observed.into_inner(),
            vec![
                (expected_identity, expected_digest),
                (expected_identity, expected_digest),
            ]
        );
    }

    // Lazy-resource accounting is a modern capability; legacy recovery has real
    // device/readback coverage in rterm_compat_gpu::tests.
    #[cfg(not(feature = "rterm-legacy-0-1"))]
    mod device_loss {
        use super::*;

        fn cursor_snapshot() -> TerminalRenderSnapshot {
            let mut terminal = Terminal::new(TerminalSize::new(2, 1));
            terminal.feed(b"A\x1b[1D\x1b[2 q");
            TerminalRenderSnapshot::from_terminal(&terminal)
        }

        fn image_graph() -> RenderGraph {
            const RED_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
            let mut terminal = Terminal::new(TerminalSize::new(1, 1));
            terminal.feed(
                format!("\x1b]1337;File=inline=1;width=2px;height=2px:{RED_PNG}\x07").as_bytes(),
            );
            let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
            let mut graph = RenderGraph::new(2, 2);
            graph.push_snapshot_images(&snapshot, RenderGeometry::new(2, 2, 2, 2), 0, None);
            graph
        }

        fn enable_fixture_text(renderer: &mut GpuLayerRenderer) {
            renderer
                .enable_text(
                    stage7_fixture_font_catalog().expect("fixture catalog"),
                    stage7_fixture_font_config(),
                    stage7_text_config().with_cursor_foreground([255, 0, 0, 255]),
                )
                .expect("fixture GPU text");
        }

        fn materialize_lazy_resources(renderer: &mut GpuLayerRenderer) {
            renderer
                .prepare_text(
                    &cursor_snapshot(),
                    RenderGeometry::new(32, 24, 16, 24),
                    &[],
                    &TextPaintConfig::default(),
                    1.0,
                    1.0,
                )
                .expect("cursor foreground");
            renderer.upload(&image_graph()).expect("image pipeline");
        }

        #[test]
        fn recovery_does_not_duplicate_lazy_resources() {
            let context =
                pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
                    .expect("headless adapter");
            let mut lost =
                GpuLayerRenderer::new_headless(&context, 64 * 1024).expect("lost renderer");
            enable_fixture_text(&mut lost);
            materialize_lazy_resources(&mut lost);
            let lost_resources = lost.initialization_resources();
            assert_eq!(lost_resources.pipeline_count, 2);
            assert_eq!(lost_resources.base_text_renderer_materialization_count, 1);
            assert_eq!(lost_resources.cursor_text_renderer_materialization_count, 1);

            let mut replacement = retire_lost_renderer_before_rebuild(&mut lost, |retired| {
                assert_eq!(
                    retired
                        .text_cpu_font_metrics()
                        .expect("retired font metrics")
                        .catalog
                        .retained_source_bytes,
                    0
                );
                let mut replacement = GpuLayerRenderer::new_headless(&context, 64 * 1024)
                    .expect("replacement renderer");
                enable_fixture_text(&mut replacement);
                let initial = replacement.initialization_resources();
                assert_eq!(initial.pipeline_count, 1);
                assert_eq!(initial.base_text_renderer_materialization_count, 1);
                assert_eq!(initial.cursor_text_renderer_materialization_count, 0);
                Ok::<_, Box<dyn Error>>(replacement)
            })
            .expect("recover renderer");

            materialize_lazy_resources(&mut replacement);
            let recovered = replacement.initialization_resources();
            assert_eq!(recovered.pipeline_count, 2);
            assert_eq!(recovered.base_text_renderer_materialization_count, 1);
            assert_eq!(recovered.cursor_text_renderer_materialization_count, 1);
            materialize_lazy_resources(&mut replacement);
            assert_eq!(replacement.initialization_resources(), recovered);
        }
    }

    #[test]
    fn device_loss_recovery_never_attempts_a_third_present_or_swallows_other_faults() {
        let snapshot =
            TerminalRenderSnapshot::from_terminal(&Terminal::new(TerminalSize::new(1, 1)));
        let attempts = Cell::new(0_u32);
        let rebuilds = Cell::new(0_u32);
        let recoveries = Cell::new(0_u32);
        let recovery_failures = Cell::new(0_u32);
        let repeated = retry_device_lost_once(
            &snapshot,
            |_| {
                attempts.set(attempts.get() + 1);
                Err::<GpuFrameStatus, _>(TestPresentError::DeviceLost)
            },
            || {
                rebuilds.set(rebuilds.get() + 1);
                Ok(())
            },
            |error| *error == TestPresentError::DeviceLost,
            |_| recoveries.set(recoveries.get() + 1),
            || recovery_failures.set(recovery_failures.get() + 1),
        );
        assert_eq!(repeated, Err(TestPresentError::DeviceLost));
        assert_eq!(attempts.get(), 2);
        assert_eq!(rebuilds.get(), 1);
        assert_eq!(recoveries.get(), 0);
        assert_eq!(recovery_failures.get(), 1);

        attempts.set(0);
        rebuilds.set(0);
        recovery_failures.set(0);
        let validation = retry_device_lost_once(
            &snapshot,
            |_| {
                attempts.set(attempts.get() + 1);
                Err::<GpuFrameStatus, _>(TestPresentError::Validation)
            },
            || {
                rebuilds.set(rebuilds.get() + 1);
                Ok(())
            },
            |error| *error == TestPresentError::DeviceLost,
            |_| recoveries.set(recoveries.get() + 1),
            || recovery_failures.set(recovery_failures.get() + 1),
        );
        assert_eq!(validation, Err(TestPresentError::Validation));
        assert_eq!(attempts.get(), 1);
        assert_eq!(rebuilds.get(), 0);
        assert_eq!(recoveries.get(), 0);
        assert_eq!(recovery_failures.get(), 0);

        attempts.set(0);
        rebuilds.set(0);
        recovery_failures.set(0);
        let layer_rebuild_failure = retry_device_lost_once(
            &snapshot,
            |_| {
                attempts.set(attempts.get() + 1);
                Err::<GpuFrameStatus, _>(TestPresentError::DeviceLost)
            },
            || {
                rebuilds.set(rebuilds.get() + 1);
                recovery_failures.set(recovery_failures.get() + 1);
                Err(TestPresentError::Validation)
            },
            |error| *error == TestPresentError::DeviceLost,
            |_| recoveries.set(recoveries.get() + 1),
            || recovery_failures.set(recovery_failures.get() + 1),
        );
        assert_eq!(layer_rebuild_failure, Err(TestPresentError::Validation));
        assert_eq!(attempts.get(), 1);
        assert_eq!(rebuilds.get(), 1);
        assert_eq!(recoveries.get(), 0);
        assert_eq!(recovery_failures.get(), 1);
    }
}
