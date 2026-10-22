use std::{
    borrow::Cow,
    error::Error,
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use super::{
    GpuInitializationResourceSnapshot, GpuPresentationMetrics, SurfaceFault, SurfaceRecovery,
    SurfaceRecoveryState,
};

pub const DEFAULT_CPU_FRAME_BYTE_BUDGET: usize = 256 * 1024 * 1024;
static NEXT_GPU_CONTEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GpuContextGeneration(u64);

fn next_gpu_context_generation() -> GpuContextGeneration {
    GpuContextGeneration(NEXT_GPU_CONTEXT_GENERATION.fetch_add(1, Ordering::Relaxed))
}

const COMPATIBILITY_SHADER: &str = r"
@group(0) @binding(0)
var frame_texture: texture_2d<f32>;
@group(0) @binding(1)
var frame_sampler: sampler;
struct Layout {
    transform: vec4<f32>,
};
@group(0) @binding(2)
var<uniform> layout_uniform_data: Layout;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let positions = array(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let uvs = array(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    var output: VertexOutput;
    output.position = vec4<f32>(
        positions[index] * layout_uniform_data.transform.xy + layout_uniform_data.transform.zw,
        0.0,
        1.0,
    );
    output.uv = uvs[index];
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(frame_texture, frame_sampler, input.uv);
}
";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RgbaFrameLayout {
    pub bytes_per_row: u32,
    pub byte_len: usize,
}

impl RgbaFrameLayout {
    /// Validates an RGBA8 framebuffer against GPU and CPU resource limits.
    ///
    /// # Errors
    ///
    /// Returns a resource-limit error for zero dimensions, arithmetic
    /// overflow, dimensions beyond `max_texture_dimension_2d`, or a byte size
    /// beyond `cpu_byte_budget`.
    pub fn new(
        width: u32,
        height: u32,
        max_texture_dimension_2d: u32,
        cpu_byte_budget: usize,
    ) -> Result<Self, GpuContextError> {
        if width == 0 || height == 0 {
            return Err(GpuContextError::with_kind(
                GpuContextErrorKind::ResourceLimit,
                "RGBA framebuffer dimensions must be nonzero".to_owned(),
            ));
        }
        if width > max_texture_dimension_2d || height > max_texture_dimension_2d {
            return Err(GpuContextError::with_kind(
                GpuContextErrorKind::ResourceLimit,
                format!(
                    "RGBA framebuffer {width}x{height} exceeds max texture dimension {max_texture_dimension_2d}"
                ),
            ));
        }
        let bytes_per_row = width.checked_mul(4).ok_or_else(|| {
            GpuContextError::with_kind(
                GpuContextErrorKind::ResourceLimit,
                "RGBA framebuffer row pitch overflow".to_owned(),
            )
        })?;
        let byte_len_u64 = u64::from(bytes_per_row)
            .checked_mul(u64::from(height))
            .ok_or_else(|| {
                GpuContextError::with_kind(
                    GpuContextErrorKind::ResourceLimit,
                    "RGBA framebuffer byte length overflow".to_owned(),
                )
            })?;
        let byte_len = usize::try_from(byte_len_u64).map_err(|_| {
            GpuContextError::with_kind(
                GpuContextErrorKind::ResourceLimit,
                "RGBA framebuffer does not fit the host address space".to_owned(),
            )
        })?;
        if byte_len > cpu_byte_budget {
            return Err(GpuContextError::with_kind(
                GpuContextErrorKind::ResourceLimit,
                format!(
                    "RGBA framebuffer requires {byte_len} bytes, exceeding the {cpu_byte_budget}-byte CPU budget"
                ),
            ));
        }
        Ok(Self {
            bytes_per_row,
            byte_len,
        })
    }
}

#[derive(Clone, Debug)]
enum GpuRuntimeFault {
    OutOfMemory(String),
    Validation(String),
    Internal(String),
    DeviceLost {
        reason: wgpu::DeviceLostReason,
        message: String,
    },
}

#[derive(Debug, Default)]
struct GpuFaultMonitor {
    pending: Mutex<Option<GpuRuntimeFault>>,
    uncaptured_errors: AtomicU64,
    device_losses: AtomicU64,
}

impl GpuFaultMonitor {
    fn record(&self, fault: GpuRuntimeFault) {
        match &fault {
            GpuRuntimeFault::DeviceLost { .. } => {
                self.device_losses.fetch_add(1, Ordering::Relaxed);
            }
            _ => {
                self.uncaptured_errors.fetch_add(1, Ordering::Relaxed);
            }
        }
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if matches!(fault, GpuRuntimeFault::DeviceLost { .. })
            || !matches!(pending.as_ref(), Some(GpuRuntimeFault::DeviceLost { .. }))
                && pending.is_none()
        {
            *pending = Some(fault);
        }
    }

    fn take(&self) -> Option<GpuRuntimeFault> {
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }
}

fn install_device_fault_handlers(device: &wgpu::Device, monitor: &Arc<GpuFaultMonitor>) {
    let uncaptured_monitor = Arc::clone(monitor);
    device.on_uncaptured_error(Arc::new(move |error| {
        let fault = match error {
            wgpu::Error::OutOfMemory { .. } => GpuRuntimeFault::OutOfMemory(error.to_string()),
            wgpu::Error::Validation { description, .. } => GpuRuntimeFault::Validation(description),
            wgpu::Error::Internal { description, .. } => GpuRuntimeFault::Internal(description),
        };
        uncaptured_monitor.record(fault);
    }));

    let lost_monitor = Arc::clone(monitor);
    device.set_device_lost_callback(move |reason, message| {
        lost_monitor.record(GpuRuntimeFault::DeviceLost { reason, message });
    });
}

fn take_runtime_fault(
    monitor: &GpuFaultMonitor,
    metrics: &mut GpuPresentationMetrics,
    stage: &str,
) -> Result<(), GpuContextError> {
    metrics.uncaptured_errors = monitor.uncaptured_errors.load(Ordering::Relaxed);
    metrics.device_losses = monitor.device_losses.load(Ordering::Relaxed);
    let Some(fault) = monitor.take() else {
        return Ok(());
    };
    match fault {
        GpuRuntimeFault::OutOfMemory(message) => Err(GpuContextError::with_kind(
            GpuContextErrorKind::OutOfMemory,
            format!("GPU out of memory {stage}: {message}"),
        )),
        GpuRuntimeFault::Validation(message) => Err(GpuContextError::with_kind(
            GpuContextErrorKind::Validation,
            format!("GPU validation error {stage}: {message}"),
        )),
        GpuRuntimeFault::Internal(message) => Err(GpuContextError::with_kind(
            GpuContextErrorKind::Internal,
            format!("internal GPU error {stage}: {message}"),
        )),
        GpuRuntimeFault::DeviceLost { reason, message } => Err(GpuContextError::with_kind(
            GpuContextErrorKind::DeviceLost,
            format!("GPU device lost {stage}: {reason:?}: {message}"),
        )),
    }
}

/// Adapter selection controls shared by headless tests and native surfaces.
#[derive(Clone, Copy, Debug)]
pub struct GpuContextOptions {
    pub backends: wgpu::Backends,
    pub power_preference: wgpu::PowerPreference,
    pub force_fallback_adapter: bool,
}

impl Default for GpuContextOptions {
    fn default() -> Self {
        Self {
            backends: native_backends(),
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: false,
        }
    }
}

impl GpuContextOptions {
    #[must_use]
    pub const fn with_high_performance(mut self, high_performance: bool) -> Self {
        self.power_preference = if high_performance {
            wgpu::PowerPreference::HighPerformance
        } else {
            wgpu::PowerPreference::LowPower
        };
        self
    }

    #[must_use]
    pub const fn with_force_fallback_adapter(mut self, force: bool) -> Self {
        self.force_fallback_adapter = force;
        self
    }

    /// Restricts recovery to the backend selected by the original adapter.
    ///
    /// # Errors
    ///
    /// Returns an error for an unrecognized backend instead of probing a
    /// different backend during a device-loss transaction.
    pub fn with_only_backend_name(mut self, backend: &str) -> Result<Self, GpuContextError> {
        self.backends = if backend.eq_ignore_ascii_case("vulkan") {
            wgpu::Backends::VULKAN
        } else if backend.eq_ignore_ascii_case("dx12") {
            wgpu::Backends::DX12
        } else if backend.eq_ignore_ascii_case("metal") {
            wgpu::Backends::METAL
        } else if backend.eq_ignore_ascii_case("gl") {
            wgpu::Backends::GL
        } else if backend.eq_ignore_ascii_case("browserwebgpu")
            || backend.eq_ignore_ascii_case("webgpu")
        {
            wgpu::Backends::BROWSER_WEBGPU
        } else {
            return Err(GpuContextError::new(
                "select recovery backend",
                format!("unsupported original GPU backend {backend:?}"),
            ));
        };
        Ok(self)
    }
}

/// Owns the instance, selected adapter, logical device, and submission queue.
pub struct GpuContext {
    generation: GpuContextGeneration,
    options: GpuContextOptions,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_window: Option<Arc<dyn wgpu::WindowHandle>>,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    compatibility_pipeline: Option<CompatibilityPipeline>,
    runtime_faults: Arc<GpuFaultMonitor>,
    suspended: bool,
    metrics: GpuPresentationMetrics,
    initialization_resources: GpuInitializationResourceSnapshot,
    #[cfg(any(test, debug_assertions))]
    direct_upload_device_loss_injections: u64,
    #[cfg(test)]
    recovery_failure_injections: u64,
    #[cfg(test)]
    recovery_post_validation_failure_injections: u64,
}

/// Event-thread-created instance and surface awaiting adapter/device setup.
///
/// The bootstrap is `Send`, so platforms that restrict window-handle access
/// to the event-loop thread can create the surface there and move the
/// expensive asynchronous initialization work to a worker.
pub struct WindowedGpuContextBootstrap {
    options: GpuContextOptions,
    instance: wgpu::Instance,
    surface_window: Arc<dyn wgpu::WindowHandle>,
    surface: wgpu::Surface<'static>,
    width: u32,
    height: u32,
}

/// Prepared surface with exactly one selected adapter, device, and queue.
///
/// The surface is deliberately not configured until `configure_surface` is
/// called, making the adapter/device ownership boundary independently holdable.
pub struct WindowedGpuDevice {
    context: GpuContext,
    width: u32,
    height: u32,
}

impl WindowedGpuContextBootstrap {
    #[must_use]
    pub fn initialization_resources(&self) -> GpuInitializationResourceSnapshot {
        GpuInitializationResourceSnapshot {
            instance_count: 1,
            surface_count: 1,
            ..GpuInitializationResourceSnapshot::default()
        }
    }

    /// Selects exactly one adapter, device, and queue without configuring or
    /// acquiring the retained surface.
    ///
    /// # Errors
    ///
    /// Returns an error when adapter selection or device creation fails.
    pub async fn select_device(self) -> Result<WindowedGpuDevice, GpuContextError> {
        let Self {
            options,
            instance,
            surface_window,
            surface,
            width,
            height,
        } = self;
        let adapter = request_adapter(&instance, Some(&surface), options).await?;
        let info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("rterm-native-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults()
                    .using_resolution(adapter.limits()),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|error| GpuContextError::new("request device", error))?;
        let runtime_faults = Arc::new(GpuFaultMonitor::default());
        install_device_fault_handlers(&device, &runtime_faults);
        let initialization_resources = GpuInitializationResourceSnapshot {
            instance_count: 1,
            surface_count: 1,
            adapter_count: 1,
            device_count: 1,
            queue_count: 1,
            backend: Some(info.backend.to_string()),
            adapter_name: Some(info.name.clone()),
            ..GpuInitializationResourceSnapshot::default()
        };

        Ok(WindowedGpuDevice {
            context: GpuContext {
                generation: next_gpu_context_generation(),
                options,
                instance,
                adapter,
                device,
                queue,
                surface_window: Some(surface_window),
                surface: Some(surface),
                surface_config: None,
                compatibility_pipeline: None,
                runtime_faults,
                suspended: width == 0 || height == 0,
                metrics: GpuPresentationMetrics::from_adapter(&info),
                initialization_resources,
                #[cfg(any(test, debug_assertions))]
                direct_upload_device_loss_injections: 0,
                #[cfg(test)]
                recovery_failure_injections: 0,
                #[cfg(test)]
                recovery_post_validation_failure_injections: 0,
            },
            width,
            height,
        })
    }
}

impl WindowedGpuDevice {
    #[must_use]
    pub const fn initialization_resources(&self) -> &GpuInitializationResourceSnapshot {
        &self.context.initialization_resources
    }

    #[must_use]
    pub const fn metrics(&self) -> &GpuPresentationMetrics {
        &self.context.metrics
    }

    /// Configures the retained surface without acquiring or presenting it.
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported surface dimensions or capabilities.
    pub fn configure_surface(mut self) -> Result<GpuContext, GpuContextError> {
        if !self.context.suspended {
            self.context.configure_surface(self.width, self.height)?;
        }
        Ok(self.context)
    }
}

impl fmt::Debug for GpuContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GpuContext")
            .field("metrics", &self.metrics)
            .finish_non_exhaustive()
    }
}

impl GpuContext {
    /// Selects a native adapter and device without requiring a display surface.
    ///
    /// # Errors
    ///
    /// Returns an error when no native or software adapter can be selected, or
    /// when the selected adapter cannot create the required device.
    pub async fn new_headless(options: GpuContextOptions) -> Result<Self, GpuContextError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: options.backends,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = request_adapter(&instance, None, options).await?;
        let info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("rterm-native-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults()
                    .using_resolution(adapter.limits()),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|error| GpuContextError::new("request device", error))?;
        let runtime_faults = Arc::new(GpuFaultMonitor::default());
        install_device_fault_handlers(&device, &runtime_faults);

        Ok(Self {
            generation: next_gpu_context_generation(),
            options,
            instance,
            adapter,
            device,
            queue,
            surface_window: None,
            surface: None,
            surface_config: None,
            compatibility_pipeline: None,
            runtime_faults,
            suspended: false,
            metrics: GpuPresentationMetrics::from_adapter(&info),
            initialization_resources: GpuInitializationResourceSnapshot {
                instance_count: 1,
                adapter_count: 1,
                device_count: 1,
                queue_count: 1,
                backend: Some(info.backend.to_string()),
                adapter_name: Some(info.name.clone()),
                ..GpuInitializationResourceSnapshot::default()
            },
            #[cfg(any(test, debug_assertions))]
            direct_upload_device_loss_injections: 0,
            #[cfg(test)]
            recovery_failure_injections: 0,
            #[cfg(test)]
            recovery_post_validation_failure_injections: 0,
        })
    }

    /// Creates a direct presentation surface from the same display and window
    /// handles owned by the active winit event loop.
    ///
    /// # Errors
    ///
    /// Returns an error when surface creation, adapter selection, device
    /// creation, or initial surface configuration fails.
    pub async fn new_windowed<D, W>(
        display: D,
        window: Arc<W>,
        width: u32,
        height: u32,
        options: GpuContextOptions,
    ) -> Result<Self, GpuContextError>
    where
        D: raw_window_handle::HasDisplayHandle + fmt::Debug + Send + Sync + 'static,
        W: wgpu::WindowHandle + 'static,
    {
        let bootstrap = Self::prepare_windowed(display, window, width, height, options)?;
        Self::finish_windowed(bootstrap).await
    }

    /// Creates the instance and native surface while window handles are
    /// available on the platform event thread.
    ///
    /// # Errors
    ///
    /// Returns an error when the native presentation surface cannot be
    /// created from the supplied display and window.
    pub fn prepare_windowed<D, W>(
        display: D,
        window: Arc<W>,
        width: u32,
        height: u32,
        options: GpuContextOptions,
    ) -> Result<WindowedGpuContextBootstrap, GpuContextError>
    where
        D: raw_window_handle::HasDisplayHandle + fmt::Debug + Send + Sync + 'static,
        W: wgpu::WindowHandle + 'static,
    {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: options.backends,
            ..wgpu::InstanceDescriptor::new_with_display_handle(Box::new(display))
        });
        let surface_window: Arc<dyn wgpu::WindowHandle> = window;
        let surface = create_surface(&instance, Arc::clone(&surface_window))?;
        Ok(WindowedGpuContextBootstrap {
            options,
            instance,
            surface_window,
            surface,
            width,
            height,
        })
    }

    /// Completes adapter and device initialization for a prepared surface.
    ///
    /// # Errors
    ///
    /// Returns an error when adapter selection, device creation, or initial
    /// surface configuration fails.
    pub async fn finish_windowed(
        bootstrap: WindowedGpuContextBootstrap,
    ) -> Result<Self, GpuContextError> {
        bootstrap.select_device().await?.configure_surface()
    }

    #[must_use]
    pub const fn metrics(&self) -> &GpuPresentationMetrics {
        &self.metrics
    }

    #[must_use]
    pub const fn initialization_resources(&self) -> &GpuInitializationResourceSnapshot {
        &self.initialization_resources
    }

    #[must_use]
    pub const fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    #[must_use]
    pub const fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    #[must_use]
    pub const fn device(&self) -> &wgpu::Device {
        &self.device
    }

    #[must_use]
    pub const fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    #[must_use]
    pub const fn generation(&self) -> GpuContextGeneration {
        self.generation
    }

    /// Atomically replaces a lost logical device while retaining the instance,
    /// presentation surface, configuration options, and cumulative metrics.
    ///
    /// This prepares a recovery candidate but deliberately does not count a
    /// successful recovery. The window owner commits that metric only after it
    /// rebuilds every device-owned layer and presents the retried frame.
    ///
    /// # Errors
    ///
    /// Returns an initialization error without replacing the current context
    /// when adapter or device recreation fails.
    #[allow(clippy::too_many_lines)]
    pub async fn recover_device(&mut self) -> Result<(), GpuContextError> {
        #[cfg(test)]
        if self.recovery_failure_injections != 0 {
            self.recovery_failure_injections = self.recovery_failure_injections.saturating_sub(1);
            self.metrics.device_recovery_failures =
                self.metrics.device_recovery_failures.saturating_add(1);
            return Err(GpuContextError::new(
                "recover device",
                "injected candidate creation failure",
            ));
        }

        let recovery_options = self.options.with_only_backend_name(&self.metrics.backend)?;
        let recovered = async {
            let adapter =
                request_adapter(&self.instance, self.surface.as_ref(), recovery_options).await?;
            let info = adapter.get_info();
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("rterm-native-recovered-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits()),
                    experimental_features: wgpu::ExperimentalFeatures::disabled(),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                    trace: wgpu::Trace::Off,
                })
                .await
                .map_err(|error| GpuContextError::new("recover device", error))?;
            let runtime_faults = Arc::new(GpuFaultMonitor::default());
            runtime_faults
                .uncaptured_errors
                .store(self.metrics.uncaptured_errors, Ordering::Relaxed);
            runtime_faults
                .device_losses
                .store(self.metrics.device_losses, Ordering::Relaxed);
            install_device_fault_handlers(&device, &runtime_faults);
            let surface_config = match (self.surface.as_ref(), self.surface_config.as_ref()) {
                (Some(surface), Some(previous)) => Some(recovery_surface_config(
                    surface, &adapter, &device, previous,
                )?),
                (None | Some(_), None) => None,
                (None, Some(_)) => {
                    return Err(GpuContextError::message(
                        "configured context has no presentation surface".into(),
                    ));
                }
            };
            let compatibility_pipeline = if self.compatibility_pipeline.is_some() {
                surface_config.as_ref().map(|config| {
                    CompatibilityPipeline::new(&device, config.format, config.width, config.height)
                })
            } else {
                None
            };
            let mut recovery_metrics = GpuPresentationMetrics::uninitialized();
            take_runtime_fault(&runtime_faults, &mut recovery_metrics, "during recovery")?;
            Ok::<_, GpuContextError>((
                adapter,
                info,
                device,
                queue,
                runtime_faults,
                compatibility_pipeline,
                surface_config,
            ))
        }
        .await;

        let (adapter, info, device, queue, runtime_faults, compatibility_pipeline, surface_config) =
            match recovered {
                Ok(recovered) => recovered,
                Err(error) => {
                    self.metrics.device_recovery_failures =
                        self.metrics.device_recovery_failures.saturating_add(1);
                    return Err(error);
                }
            };

        #[cfg(test)]
        if self.recovery_post_validation_failure_injections != 0 {
            self.recovery_post_validation_failure_injections = self
                .recovery_post_validation_failure_injections
                .saturating_sub(1);
            self.metrics.device_recovery_failures =
                self.metrics.device_recovery_failures.saturating_add(1);
            return Err(GpuContextError::new(
                "recover device",
                "injected candidate post-validation failure",
            ));
        }

        let previous = self.metrics.clone();
        let mut metrics = GpuPresentationMetrics::from_adapter(&info);
        metrics.surface_format = surface_config
            .as_ref()
            .map(|config| format!("{:?}", config.format))
            .or(previous.surface_format);
        metrics.present_mode = surface_config
            .as_ref()
            .map(|config| format!("{:?}", config.present_mode))
            .or(previous.present_mode);
        metrics.surface_width = surface_config
            .as_ref()
            .map(|config| config.width)
            .or(previous.surface_width);
        metrics.surface_height = surface_config
            .as_ref()
            .map(|config| config.height)
            .or(previous.surface_height);
        metrics.rendered_frames = previous.rendered_frames;
        metrics.presented_frames = previous.presented_frames;
        metrics.surface_reconfigurations = previous.surface_reconfigurations;
        metrics.surface_recreations = previous.surface_recreations;
        metrics.surface_timeouts = previous.surface_timeouts;
        metrics.surface_occlusions = previous.surface_occlusions;
        metrics.surface_validation_errors = previous.surface_validation_errors;
        metrics.compatibility_frame_uploads = previous.compatibility_frame_uploads;
        metrics.uncaptured_errors = previous.uncaptured_errors;
        metrics.device_losses = previous.device_losses;
        metrics.device_recoveries = previous.device_recoveries;
        metrics.device_recovery_failures = previous.device_recovery_failures;
        metrics.abandoned_lost_surfaces = previous.abandoned_lost_surfaces;

        self.generation = next_gpu_context_generation();
        self.adapter = adapter;
        self.device = device;
        self.queue = queue;
        self.runtime_faults = runtime_faults;
        self.compatibility_pipeline = compatibility_pipeline;
        self.surface_config = surface_config;
        self.metrics = metrics;
        if let (Some(surface), Some(config)) = (self.surface.as_ref(), self.surface_config.as_ref())
        {
            // `Surface::configure` is the non-rollback recovery commit
            // boundary. Every operation that can return `Result::Err`
            // completed before self was replaced. A backend panic remains a
            // panic; it must not masquerade as an atomic, recoverable error
            // after either self or the live surface may have changed.
            self.metrics.surface_reconfigurations =
                self.metrics.surface_reconfigurations.saturating_add(1);
            surface.configure(&self.device, config);
            self.suspended = false;
        }
        Ok(())
    }

    /// Commits one recovery after the replacement device, device-owned layers,
    /// and retried presentation have all succeeded as one window transaction.
    pub fn commit_windowed_device_recovery(&mut self) {
        self.metrics.device_recoveries = self.metrics.device_recoveries.saturating_add(1);
    }

    /// Counts one recovery transaction that failed after candidate creation.
    pub fn record_device_recovery_failure(&mut self) {
        self.metrics.device_recovery_failures =
            self.metrics.device_recovery_failures.saturating_add(1);
    }

    /// Records the actual process-exit abandonment of one recovered window
    /// surface after the app has decided to `forget` its driver-owned bundle.
    pub fn record_abandoned_lost_surface(&mut self) {
        self.metrics.abandoned_lost_surfaces =
            self.metrics.abandoned_lost_surfaces.saturating_add(1);
    }

    #[cfg(any(test, debug_assertions))]
    #[doc(hidden)]
    pub fn inject_device_loss_for_test(&self) {
        self.runtime_faults.record(GpuRuntimeFault::DeviceLost {
            reason: wgpu::DeviceLostReason::Unknown,
            message: "injected device loss".to_owned(),
        });
    }

    #[cfg(any(test, debug_assertions))]
    #[doc(hidden)]
    pub fn inject_device_loss_during_direct_upload_for_test(&mut self) {
        self.direct_upload_device_loss_injections =
            self.direct_upload_device_loss_injections.saturating_add(1);
    }

    #[cfg(test)]
    fn inject_recovery_failures_for_test(&mut self, count: u64) {
        self.recovery_failure_injections = count;
    }

    #[cfg(test)]
    fn inject_recovery_post_validation_failures_for_test(&mut self, count: u64) {
        self.recovery_post_validation_failure_injections = count;
    }

    #[must_use]
    pub fn max_texture_dimension_2d(&self) -> u32 {
        self.device.limits().max_texture_dimension_2d
    }

    /// Configured native surface format, when this is a windowed context.
    #[must_use]
    pub fn surface_format(&self) -> Option<wgpu::TextureFormat> {
        self.surface_config.as_ref().map(|config| config.format)
    }

    /// Validates a compatibility framebuffer against the selected device and
    /// the process-wide CPU framebuffer budget.
    ///
    /// # Errors
    ///
    /// Returns a resource-limit error for unsupported or excessive geometry.
    pub fn rgba_frame_layout(
        &self,
        width: u32,
        height: u32,
    ) -> Result<RgbaFrameLayout, GpuContextError> {
        RgbaFrameLayout::new(
            width,
            height,
            self.max_texture_dimension_2d(),
            DEFAULT_CPU_FRAME_BYTE_BUDGET,
        )
    }

    fn check_runtime_faults(&mut self, stage: &str) -> Result<(), GpuContextError> {
        take_runtime_fault(&self.runtime_faults, &mut self.metrics, stage)
    }

    /// Executes a tiny native submission and polls it with a caller-owned deadline.
    ///
    /// # Errors
    ///
    /// Returns an error when device polling fails or the submission does not
    /// complete before `timeout`.
    pub fn run_headless_submission_probe(
        &mut self,
        timeout: Duration,
    ) -> Result<(), GpuContextError> {
        self.check_runtime_faults("before headless submission probe")?;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rterm-headless-probe"),
            size: 4,
            usage: wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.check_runtime_faults("after headless buffer creation")?;
        self.queue.write_buffer(&buffer, 0, &[1, 2, 3, 4]);
        self.check_runtime_faults("after headless buffer upload")?;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("rterm-headless-probe-submit"),
            });
        encoder.clear_buffer(&buffer, 0, None);
        self.queue.submit([encoder.finish()]);
        self.check_runtime_faults("after headless submission")?;

        let deadline = Instant::now()
            .checked_add(timeout)
            .unwrap_or_else(Instant::now);
        loop {
            let status = self
                .device
                .poll(wgpu::PollType::Poll)
                .map_err(|error| GpuContextError::new("poll headless submission", error))?;
            self.check_runtime_faults("while polling headless submission")?;
            if status.is_queue_empty() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(GpuContextError::message(format!(
                    "headless submission did not complete within {timeout:?}"
                )));
            }
            std::thread::yield_now();
        }
    }

    /// Acquires, clears, submits, and presents exactly one configured surface
    /// frame without materializing layer or text pipelines.
    ///
    /// # Errors
    ///
    /// Returns an error when the surface is not configured, acquisition fails,
    /// or a device fault is reported during the one-frame transaction.
    pub fn present_clear_once(&mut self) -> Result<GpuFrameStatus, GpuContextError> {
        if self.suspended {
            return Ok(GpuFrameStatus::Skipped);
        }
        run_initialization_clear_transaction(self)
    }

    /// Reconfigures the swap chain, or suspends acquisition for a zero-sized window.
    ///
    /// # Errors
    ///
    /// Returns an error if a nonzero surface exposes no usable configuration.
    pub fn resize_surface(&mut self, width: u32, height: u32) -> Result<(), GpuContextError> {
        self.suspended = width == 0 || height == 0;
        if self.suspended {
            return Ok(());
        }
        self.configure_surface(width, height)
    }

    /// Uploads the compatibility CPU framebuffer and presents it through the
    /// directly owned wgpu surface.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid framebuffer geometry, unrecoverable surface
    /// acquisition failures, or missing presentation state.
    pub fn render_rgba(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        pre_present: impl FnOnce(),
    ) -> Result<GpuFrameStatus, GpuContextError> {
        if self.suspended {
            return Ok(GpuFrameStatus::Skipped);
        }
        self.check_runtime_faults("before frame upload")?;
        let layout = self.rgba_frame_layout(width, height)?;
        validate_frame(rgba, layout)?;
        self.ensure_compatibility_pipeline()?;
        self.ensure_upload_frame(width, height)?;
        {
            let upload = self
                .compatibility_pipeline
                .as_ref()
                .and_then(|pipeline| pipeline.upload.as_ref())
                .ok_or_else(|| {
                    GpuContextError::message("upload texture is not configured".into())
                })?;
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &upload.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(layout.bytes_per_row),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
        self.metrics.compatibility_frame_uploads =
            self.metrics.compatibility_frame_uploads.saturating_add(1);
        self.check_runtime_faults("after frame upload")?;

        let Some((surface_texture, suboptimal)) =
            self.acquire_surface_texture(&mut SurfaceRecoveryState::new())?
        else {
            return Ok(GpuFrameStatus::Skipped);
        };
        let pipeline = self
            .compatibility_pipeline
            .as_ref()
            .ok_or_else(|| GpuContextError::message("surface pipeline is not configured".into()))?;
        let upload = pipeline
            .upload
            .as_ref()
            .ok_or_else(|| GpuContextError::message("upload texture is not configured".into()))?;
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("rterm-compatibility-frame"),
            });
        {
            let color_attachments = [Some(wgpu::RenderPassColorAttachment {
                view: &surface_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })];
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("rterm-compatibility-present"),
                color_attachments: &color_attachments,
                ..wgpu::RenderPassDescriptor::default()
            });
            let (x, y, width, height) = pipeline.clip_rect;
            render_pass.set_scissor_rect(x, y, width, height);
            render_pass.set_pipeline(&pipeline.pipeline);
            render_pass.set_bind_group(0, &upload.bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
        self.check_runtime_faults("before frame submission")?;
        self.queue.submit([encoder.finish()]);
        self.check_runtime_faults("after frame submission")?;
        self.metrics.rendered_frames = self.metrics.rendered_frames.saturating_add(1);
        self.check_runtime_faults("before frame presentation")?;
        pre_present();
        self.queue.present(surface_texture);
        self.check_runtime_faults("after frame presentation")?;
        self.metrics.presented_frames = self.metrics.presented_frames.saturating_add(1);
        if suboptimal {
            let (width, height) = self.configured_size()?;
            self.configure_surface(width, height)?;
        }
        Ok(GpuFrameStatus::Presented)
    }

    /// Presents a prepared terminal render graph directly to the native
    /// swapchain surface.
    ///
    /// # Errors
    ///
    /// Returns an error for a foreign renderer, graph preparation failure,
    /// unrecoverable surface acquisition failure, or device fault.
    pub fn render_graph(
        &mut self,
        renderer: &mut super::GpuLayerRenderer,
        graph: &super::RenderGraph,
        pre_present: impl FnOnce(),
    ) -> Result<GpuFrameStatus, GpuContextError> {
        if self.suspended {
            return Ok(GpuFrameStatus::Skipped);
        }
        self.check_runtime_faults("before direct frame upload")?;
        #[cfg(any(test, debug_assertions))]
        if self.direct_upload_device_loss_injections != 0 {
            self.direct_upload_device_loss_injections =
                self.direct_upload_device_loss_injections.saturating_sub(1);
            self.runtime_faults.record(GpuRuntimeFault::DeviceLost {
                reason: wgpu::DeviceLostReason::Unknown,
                message: "injected device loss during direct frame upload".to_owned(),
            });
        }
        if let Err(error) = renderer.upload_from(self, graph) {
            self.check_runtime_faults("during direct frame upload")?;
            return Err(GpuContextError::message(error.to_string()));
        }
        self.check_runtime_faults("after direct frame upload")?;
        let Some((surface_texture, suboptimal)) =
            self.acquire_surface_texture(&mut SurfaceRecoveryState::new())?
        else {
            return Ok(GpuFrameStatus::Skipped);
        };
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("rterm-direct-terminal-frame"),
            });
        if let Err(error) = renderer.encode_render_pass(&mut encoder, &surface_view) {
            self.check_runtime_faults("during direct frame encoding")?;
            return Err(GpuContextError::message(error.to_string()));
        }
        self.check_runtime_faults("before direct frame submission")?;
        self.queue.submit([encoder.finish()]);
        self.check_runtime_faults("after direct frame submission")?;
        self.metrics.rendered_frames = self.metrics.rendered_frames.saturating_add(1);
        pre_present();
        self.queue.present(surface_texture);
        self.check_runtime_faults("after direct frame presentation")?;
        self.metrics.presented_frames = self.metrics.presented_frames.saturating_add(1);
        if suboptimal {
            let (width, height) = self.configured_size()?;
            self.configure_surface(width, height)?;
        }
        Ok(GpuFrameStatus::Presented)
    }

    fn configure_surface(&mut self, width: u32, height: u32) -> Result<(), GpuContextError> {
        if width == 0 || height == 0 {
            self.suspended = true;
            return Ok(());
        }
        let max_dimension = self.max_texture_dimension_2d();
        if width > max_dimension || height > max_dimension {
            return Err(GpuContextError::with_kind(
                GpuContextErrorKind::ResourceLimit,
                format!("surface {width}x{height} exceeds max texture dimension {max_dimension}"),
            ));
        }
        let surface = self.surface.as_ref().ok_or_else(|| {
            GpuContextError::message("context has no presentation surface".into())
        })?;
        let capabilities = surface.get_capabilities(&self.adapter);
        let format = Self::preferred_surface_format(&capabilities.formats)
            .ok_or_else(|| GpuContextError::message("surface exposes no formats".into()))?;
        #[cfg(debug_assertions)]
        let test_present_mode = std::env::var("RTERM_TEST_PRESENT_MODE").ok();
        #[cfg(not(debug_assertions))]
        let test_present_mode: Option<&str> = None;
        let present_mode =
            preferred_present_mode(&capabilities.present_modes, test_present_mode.as_deref())
                .ok_or_else(|| {
                    GpuContextError::message("surface exposes no present modes".into())
                })?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width,
            height,
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
        };
        surface.configure(&self.device, &config);
        if self
            .surface_config
            .as_ref()
            .is_none_or(|previous| previous.format != config.format)
        {
            if self.compatibility_pipeline.is_some() {
                self.compatibility_pipeline = Some(CompatibilityPipeline::new(
                    &self.device,
                    config.format,
                    width,
                    height,
                ));
            }
        } else if let Some(pipeline) = self.compatibility_pipeline.as_mut() {
            pipeline.set_surface_size(&self.queue, width, height)?;
        }
        self.check_runtime_faults("after surface configuration")?;
        self.metrics.surface_format = Some(format!("{format:?}"));
        self.metrics.present_mode = Some(format!("{present_mode:?}"));
        self.metrics.surface_width = Some(width);
        self.metrics.surface_height = Some(height);
        self.metrics.surface_reconfigurations =
            self.metrics.surface_reconfigurations.saturating_add(1);
        self.initialization_resources.surface_configure_count = self
            .initialization_resources
            .surface_configure_count
            .saturating_add(1);
        self.surface_config = Some(config);
        self.suspended = false;
        Ok(())
    }

    fn preferred_surface_format(formats: &[wgpu::TextureFormat]) -> Option<wgpu::TextureFormat> {
        // Terminal colors are already screen-space RGBA bytes. A linear
        // surface keeps the GPU path from applying an extra sRGB transfer.
        formats
            .iter()
            .copied()
            .find(|format| !format.is_srgb())
            .or_else(|| formats.iter().copied().find(wgpu::TextureFormat::is_srgb))
            .or_else(|| formats.first().copied())
    }

    fn recreate_surface(&mut self) -> Result<(), GpuContextError> {
        let window = self
            .surface_window
            .clone()
            .ok_or_else(|| GpuContextError::message("surface window is unavailable".into()))?;
        self.surface = Some(create_surface(&self.instance, window)?);
        self.metrics.surface_recreations = self.metrics.surface_recreations.saturating_add(1);
        let (width, height) = self.configured_size()?;
        self.configure_surface(width, height)
    }

    fn configured_size(&self) -> Result<(u32, u32), GpuContextError> {
        self.surface_config
            .as_ref()
            .map(|config| (config.width, config.height))
            .ok_or_else(|| GpuContextError::message("surface is not configured".into()))
    }

    fn acquire_surface_texture(
        &mut self,
        recovery: &mut SurfaceRecoveryState,
    ) -> Result<Option<(wgpu::SurfaceTexture, bool)>, GpuContextError> {
        self.check_runtime_faults("before surface acquisition")?;
        let acquisition = self
            .surface
            .as_ref()
            .ok_or_else(|| GpuContextError::message("context has no presentation surface".into()))?
            .get_current_texture();
        self.check_runtime_faults("after surface acquisition")?;
        match acquisition {
            wgpu::CurrentSurfaceTexture::Success(texture) => {
                self.initialization_resources.surface_acquire_count = self
                    .initialization_resources
                    .surface_acquire_count
                    .saturating_add(1);
                Ok(Some((texture, false)))
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                self.initialization_resources.surface_acquire_count = self
                    .initialization_resources
                    .surface_acquire_count
                    .saturating_add(1);
                Ok(Some((texture, true)))
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                self.metrics.surface_timeouts = self.metrics.surface_timeouts.saturating_add(1);
                Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                self.metrics.surface_occlusions = self.metrics.surface_occlusions.saturating_add(1);
                Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Outdated
                if recovery.action(SurfaceFault::Outdated)
                    == SurfaceRecovery::ReconfigureAndRetry =>
            {
                let (width, height) = self.configured_size()?;
                self.configure_surface(width, height)?;
                self.acquire_surface_texture(recovery)
            }
            wgpu::CurrentSurfaceTexture::Lost
                if recovery.action(SurfaceFault::Lost) == SurfaceRecovery::RecreateAndRetry =>
            {
                self.recreate_surface()?;
                self.acquire_surface_texture(recovery)
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                self.metrics.surface_validation_errors =
                    self.metrics.surface_validation_errors.saturating_add(1);
                Err(GpuContextError::message(
                    "surface acquisition validation error".into(),
                ))
            }
            wgpu::CurrentSurfaceTexture::Outdated => Err(GpuContextError::message(
                "surface remained outdated after one reconfiguration".into(),
            )),
            wgpu::CurrentSurfaceTexture::Lost => Err(GpuContextError::message(
                "surface remained lost after one recreation".into(),
            )),
        }
    }

    fn ensure_upload_frame(&mut self, width: u32, height: u32) -> Result<(), GpuContextError> {
        self.ensure_compatibility_pipeline()?;
        let pipeline = self
            .compatibility_pipeline
            .as_mut()
            .expect("configured surface must own a compatibility pipeline");
        if pipeline
            .upload
            .as_ref()
            .is_some_and(|upload| upload.width == width && upload.height == height)
        {
            return Ok(());
        }
        pipeline.upload = Some(UploadFrame::new(
            &self.device,
            &pipeline.bind_group_layout,
            &pipeline.sampler,
            &pipeline.layout_uniform,
            self.surface_config
                .as_ref()
                .map_or(wgpu::TextureFormat::Rgba8Unorm, |config| config.format),
            width,
            height,
        ));
        pipeline.set_frame_size(&self.queue, width, height)?;
        self.check_runtime_faults("after compatibility texture creation")
    }

    fn ensure_compatibility_pipeline(&mut self) -> Result<(), GpuContextError> {
        if self.compatibility_pipeline.is_none() {
            let config = self
                .surface_config
                .as_ref()
                .ok_or_else(|| GpuContextError::message("surface is not configured".into()))?;
            self.compatibility_pipeline = Some(CompatibilityPipeline::new(
                &self.device,
                config.format,
                config.width,
                config.height,
            ));
        }
        Ok(())
    }
}

fn preferred_present_mode(
    available: &[wgpu::PresentMode],
    test_override: Option<&str>,
) -> Option<wgpu::PresentMode> {
    #[cfg(not(debug_assertions))]
    let _ = test_override;

    #[cfg(debug_assertions)]
    if test_override.is_some_and(|value| value.eq_ignore_ascii_case("immediate"))
        && available.contains(&wgpu::PresentMode::Immediate)
    {
        return Some(wgpu::PresentMode::Immediate);
    }

    available
        .contains(&wgpu::PresentMode::Fifo)
        .then_some(wgpu::PresentMode::Fifo)
        .or_else(|| available.first().copied())
}

fn create_surface(
    instance: &wgpu::Instance,
    window: Arc<dyn wgpu::WindowHandle>,
) -> Result<wgpu::Surface<'static>, GpuContextError> {
    instance
        .create_surface(wgpu::SurfaceTarget::from_window_without_display(window))
        .map_err(|error| GpuContextError::new("create surface", error))
}

fn recovery_surface_config(
    surface: &wgpu::Surface<'static>,
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    previous: &wgpu::SurfaceConfiguration,
) -> Result<wgpu::SurfaceConfiguration, GpuContextError> {
    let max_dimension = device.limits().max_texture_dimension_2d;
    if previous.width > max_dimension || previous.height > max_dimension {
        return Err(GpuContextError::with_kind(
            GpuContextErrorKind::ResourceLimit,
            format!(
                "recovered surface {}x{} exceeds max texture dimension {max_dimension}",
                previous.width, previous.height
            ),
        ));
    }
    let capabilities = surface.get_capabilities(adapter);
    if !capabilities
        .usages
        .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
    {
        return Err(GpuContextError::message(
            "recovered surface does not support render attachments".into(),
        ));
    }
    let format = capabilities
        .formats
        .contains(&previous.format)
        .then_some(previous.format)
        .filter(|format| !format.is_srgb())
        .or_else(|| GpuContext::preferred_surface_format(&capabilities.formats))
        .ok_or_else(|| GpuContextError::message("recovered surface exposes no formats".into()))?;
    let present_mode = capabilities
        .present_modes
        .contains(&previous.present_mode)
        .then_some(previous.present_mode)
        .or_else(|| {
            capabilities
                .present_modes
                .contains(&wgpu::PresentMode::Fifo)
                .then_some(wgpu::PresentMode::Fifo)
        })
        .or_else(|| capabilities.present_modes.first().copied())
        .ok_or_else(|| {
            GpuContextError::message("recovered surface exposes no present modes".into())
        })?;
    let alpha_mode = capabilities
        .alpha_modes
        .contains(&previous.alpha_mode)
        .then_some(previous.alpha_mode)
        .or_else(|| {
            capabilities
                .alpha_modes
                .contains(&wgpu::CompositeAlphaMode::Auto)
                .then_some(wgpu::CompositeAlphaMode::Auto)
        })
        .or_else(|| capabilities.alpha_modes.first().copied())
        .ok_or_else(|| {
            GpuContextError::message("recovered surface exposes no alpha modes".into())
        })?;
    Ok(wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        color_space: previous.color_space,
        width: previous.width,
        height: previous.height,
        present_mode,
        desired_maximum_frame_latency: previous.desired_maximum_frame_latency,
        alpha_mode,
        view_formats: Vec::new(),
    })
}

fn validate_frame(rgba: &[u8], layout: RgbaFrameLayout) -> Result<(), GpuContextError> {
    if rgba.len() != layout.byte_len {
        return Err(GpuContextError::with_kind(
            GpuContextErrorKind::InvalidFrame,
            format!(
                "compatibility framebuffer length mismatch: expected {}, got {}",
                layout.byte_len,
                rgba.len()
            ),
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct CompatibilityLayout {
    clip_rect: (u32, u32, u32, u32),
    transform: [f32; 4],
}

impl CompatibilityLayout {
    #[allow(clippy::cast_precision_loss)] // Device texture limits stay below f32's exact integer range.
    fn new(
        frame_width: u32,
        frame_height: u32,
        surface_width: u32,
        surface_height: u32,
    ) -> Result<Self, GpuContextError> {
        if frame_width == 0 || frame_height == 0 || surface_width == 0 || surface_height == 0 {
            return Err(GpuContextError::with_kind(
                GpuContextErrorKind::InvalidFrame,
                "compatibility layout dimensions must be nonzero".to_owned(),
            ));
        }
        let integer_scale = (surface_width / frame_width)
            .min(surface_height / frame_height)
            .max(1);
        let scaled_width = frame_width.checked_mul(integer_scale).ok_or_else(|| {
            GpuContextError::with_kind(
                GpuContextErrorKind::ResourceLimit,
                "compatibility layout width overflow".to_owned(),
            )
        })?;
        let scaled_height = frame_height.checked_mul(integer_scale).ok_or_else(|| {
            GpuContextError::with_kind(
                GpuContextErrorKind::ResourceLimit,
                "compatibility layout height overflow".to_owned(),
            )
        })?;
        let clip_width = scaled_width.min(surface_width);
        let clip_height = scaled_height.min(surface_height);
        let clip_x = surface_width.saturating_sub(clip_width) / 2;
        let clip_y = surface_height.saturating_sub(clip_height) / 2;
        let surface_width_f32 = surface_width as f32;
        let surface_height_f32 = surface_height as f32;
        let translation_x = if surface_width % 2 == 0 {
            0.0
        } else {
            0.5 / surface_width_f32
        };
        let translation_y = if surface_height % 2 == 0 {
            0.0
        } else {
            0.5 / surface_height_f32
        };
        Ok(Self {
            clip_rect: (clip_x, clip_y, clip_width, clip_height),
            transform: [
                scaled_width as f32 / surface_width_f32,
                scaled_height as f32 / surface_height_f32,
                translation_x,
                translation_y,
            ],
        })
    }
}

struct CompatibilityPipeline {
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    layout_uniform: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    upload: Option<UploadFrame>,
    surface_size: (u32, u32),
    frame_size: Option<(u32, u32)>,
    clip_rect: (u32, u32, u32, u32),
}

impl CompatibilityPipeline {
    fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        surface_width: u32,
        surface_height: u32,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rterm-compatibility-bindings"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(16),
                    },
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("rterm-compatibility-sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });
        let layout_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rterm-compatibility-layout"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rterm-compatibility-shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(COMPATIBILITY_SHADER)),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rterm-compatibility-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let targets = [Some(wgpu::ColorTargetState {
            format: surface_format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rterm-compatibility-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &targets,
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            bind_group_layout,
            sampler,
            layout_uniform,
            pipeline,
            upload: None,
            surface_size: (surface_width, surface_height),
            frame_size: None,
            clip_rect: (0, 0, surface_width, surface_height),
        }
    }

    fn set_surface_size(
        &mut self,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
    ) -> Result<(), GpuContextError> {
        self.surface_size = (width, height);
        self.update_layout(queue)
    }

    fn set_frame_size(
        &mut self,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
    ) -> Result<(), GpuContextError> {
        self.frame_size = Some((width, height));
        self.update_layout(queue)
    }

    fn update_layout(&mut self, queue: &wgpu::Queue) -> Result<(), GpuContextError> {
        let Some((frame_width, frame_height)) = self.frame_size else {
            return Ok(());
        };
        let layout = CompatibilityLayout::new(
            frame_width,
            frame_height,
            self.surface_size.0,
            self.surface_size.1,
        )?;
        let mut bytes = [0_u8; 16];
        for (destination, value) in bytes.chunks_exact_mut(4).zip(layout.transform) {
            destination.copy_from_slice(&value.to_ne_bytes());
        }
        queue.write_buffer(&self.layout_uniform, 0, &bytes);
        self.clip_rect = layout.clip_rect;
        Ok(())
    }
}

struct UploadFrame {
    width: u32,
    height: u32,
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

impl UploadFrame {
    fn new(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        layout_uniform: &wgpu::Buffer,
        surface_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rterm-compatibility-framebuffer"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: if surface_format.is_srgb() {
                wgpu::TextureFormat::Rgba8UnormSrgb
            } else {
                wgpu::TextureFormat::Rgba8Unorm
            },
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rterm-compatibility-frame-bind-group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: layout_uniform.as_entire_binding(),
                },
            ],
        });
        Self {
            width,
            height,
            texture,
            bind_group,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuFrameStatus {
    Presented,
    Skipped,
}

async fn request_adapter(
    instance: &wgpu::Instance,
    compatible_surface: Option<&wgpu::Surface<'_>>,
    options: GpuContextOptions,
) -> Result<wgpu::Adapter, GpuContextError> {
    let primary = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: options.power_preference,
            force_fallback_adapter: options.force_fallback_adapter,
            compatible_surface,
            apply_limit_buckets: false,
        })
        .await;
    match primary {
        Ok(adapter) => Ok(adapter),
        Err(primary_error) if !options.force_fallback_adapter => instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: options.power_preference,
                force_fallback_adapter: true,
                compatible_surface,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|fallback_error| {
                GpuContextError::message(format!(
                    "request adapter failed: primary={primary_error}; software fallback={fallback_error}"
                ))
            }),
        Err(error) => Err(GpuContextError::new("request fallback adapter", error)),
    }
}

const fn native_backends() -> wgpu::Backends {
    #[cfg(target_os = "windows")]
    {
        wgpu::Backends::DX12
            .union(wgpu::Backends::VULKAN)
            .union(wgpu::Backends::GL)
    }
    #[cfg(target_os = "macos")]
    {
        wgpu::Backends::METAL
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        wgpu::Backends::VULKAN.union(wgpu::Backends::GL)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    {
        wgpu::Backends::all()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuContextErrorKind {
    Initialization,
    Surface,
    InvalidFrame,
    ResourceLimit,
    Validation,
    OutOfMemory,
    Internal,
    DeviceLost,
}

#[derive(Debug)]
pub struct GpuContextError {
    kind: GpuContextErrorKind,
    message: String,
}

impl GpuContextError {
    fn new(context: &str, error: impl fmt::Display) -> Self {
        Self::with_kind(
            GpuContextErrorKind::Initialization,
            format!("{context}: {error}"),
        )
    }

    fn message(message: String) -> Self {
        Self::with_kind(GpuContextErrorKind::Surface, message)
    }

    fn with_kind(kind: GpuContextErrorKind, message: String) -> Self {
        Self { kind, message }
    }

    #[must_use]
    pub const fn kind(&self) -> GpuContextErrorKind {
        self.kind
    }
}

impl fmt::Display for GpuContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for GpuContextError {}

trait InitializationClearTransactionDriver {
    type Frame;

    fn ensure_available(&self) -> Result<(), GpuContextError>;
    fn acquire(&mut self) -> Result<Option<(Self::Frame, bool)>, GpuContextError>;
    fn submit_and_present(&mut self, frame: Self::Frame) -> Result<(), GpuContextError>;
    fn commit(&mut self, suboptimal: bool) -> Result<(), GpuContextError>;
    fn post_present(&mut self) -> Result<(), GpuContextError>;
}

fn run_initialization_clear_transaction<D: InitializationClearTransactionDriver>(
    driver: &mut D,
) -> Result<GpuFrameStatus, GpuContextError> {
    driver.ensure_available()?;
    let Some((frame, suboptimal)) = driver.acquire()? else {
        return Ok(GpuFrameStatus::Skipped);
    };
    driver.submit_and_present(frame)?;
    driver.commit(suboptimal)?;
    driver.post_present()?;
    Ok(GpuFrameStatus::Presented)
}

impl InitializationClearTransactionDriver for GpuContext {
    type Frame = wgpu::SurfaceTexture;

    fn ensure_available(&self) -> Result<(), GpuContextError> {
        ensure_initialization_clear_available(&self.initialization_resources)
    }

    fn acquire(&mut self) -> Result<Option<(Self::Frame, bool)>, GpuContextError> {
        self.acquire_surface_texture(&mut SurfaceRecoveryState::new())
    }

    fn submit_and_present(&mut self, surface_texture: Self::Frame) -> Result<(), GpuContextError> {
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("rterm-attribution-clear-frame"),
            });
        {
            let color_attachments = [Some(wgpu::RenderPassColorAttachment {
                view: &surface_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })];
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("rterm-attribution-clear-present"),
                color_attachments: &color_attachments,
                ..wgpu::RenderPassDescriptor::default()
            });
        }
        self.check_runtime_faults("before attribution clear submission")?;
        self.queue.submit([encoder.finish()]);
        self.check_runtime_faults("after attribution clear submission")?;
        self.metrics.rendered_frames = self.metrics.rendered_frames.saturating_add(1);
        self.queue.present(surface_texture);
        Ok(())
    }

    fn commit(&mut self, suboptimal: bool) -> Result<(), GpuContextError> {
        commit_initialization_clear_present(
            &mut self.metrics,
            &mut self.initialization_resources,
            suboptimal,
        )
    }

    fn post_present(&mut self) -> Result<(), GpuContextError> {
        self.check_runtime_faults("after attribution clear presentation")
    }
}

fn commit_initialization_clear_present(
    metrics: &mut GpuPresentationMetrics,
    resources: &mut GpuInitializationResourceSnapshot,
    suboptimal: bool,
) -> Result<(), GpuContextError> {
    metrics.presented_frames = metrics.presented_frames.saturating_add(1);
    resources.clear_present_count = 1;
    if suboptimal {
        return Err(GpuContextError::message(
            "suboptimal initialization clear frame was presented; retry is forbidden".to_owned(),
        ));
    }
    Ok(())
}

fn ensure_initialization_clear_available(
    resources: &GpuInitializationResourceSnapshot,
) -> Result<(), GpuContextError> {
    if resources.clear_present_count != 0 {
        return Err(GpuContextError::message(
            "initialization clear frame was already presented".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use super::*;

    fn finish_prepared_windowed_context(bootstrap: WindowedGpuContextBootstrap) {
        drop(GpuContext::finish_windowed(bootstrap));
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum FakeClearFault {
        None,
        Timeout,
        PostPresent,
    }

    struct FakeClearDriver {
        metrics: GpuPresentationMetrics,
        resources: GpuInitializationResourceSnapshot,
        fault: FakeClearFault,
        suboptimal: bool,
        acquire_calls: u64,
        present_calls: u64,
        configure_calls: u64,
    }

    impl FakeClearDriver {
        fn new(fault: FakeClearFault, suboptimal: bool) -> Self {
            Self {
                metrics: GpuPresentationMetrics::uninitialized(),
                resources: GpuInitializationResourceSnapshot::default(),
                fault,
                suboptimal,
                acquire_calls: 0,
                present_calls: 0,
                configure_calls: 0,
            }
        }
    }

    impl InitializationClearTransactionDriver for FakeClearDriver {
        type Frame = ();

        fn ensure_available(&self) -> Result<(), GpuContextError> {
            ensure_initialization_clear_available(&self.resources)
        }

        fn acquire(&mut self) -> Result<Option<(Self::Frame, bool)>, GpuContextError> {
            self.acquire_calls += 1;
            if self.fault == FakeClearFault::Timeout {
                return Err(GpuContextError::message(
                    "injected initialization clear timeout".to_owned(),
                ));
            }
            Ok(Some(((), self.suboptimal)))
        }

        fn submit_and_present(&mut self, (): Self::Frame) -> Result<(), GpuContextError> {
            self.metrics.rendered_frames += 1;
            self.present_calls += 1;
            Ok(())
        }

        fn commit(&mut self, suboptimal: bool) -> Result<(), GpuContextError> {
            commit_initialization_clear_present(&mut self.metrics, &mut self.resources, suboptimal)
        }

        fn post_present(&mut self) -> Result<(), GpuContextError> {
            if self.fault == FakeClearFault::PostPresent {
                return Err(GpuContextError::message(
                    "injected post-present fault".to_owned(),
                ));
            }
            Ok(())
        }
    }

    #[test]
    fn prepared_windowed_context_is_sendable_to_the_device_initialization_worker() {
        fn assert_send<T: Send>() {}
        fn assert_finish_signature(_: fn(WindowedGpuContextBootstrap)) {}

        assert_send::<WindowedGpuContextBootstrap>();
        assert_finish_signature(finish_prepared_windowed_context);
    }

    #[test]
    fn initialization_clear_timeout_does_not_commit_a_present() {
        let mut driver = FakeClearDriver::new(FakeClearFault::Timeout, false);
        let error = run_initialization_clear_transaction(&mut driver)
            .expect_err("timeout must fail before presentation");
        assert!(error.to_string().contains("timeout"));
        assert_eq!(driver.acquire_calls, 1);
        assert_eq!(driver.present_calls, 0);
        assert_eq!(driver.configure_calls, 0);
        assert_eq!(driver.metrics.rendered_frames, 0);
        assert_eq!(driver.metrics.presented_frames, 0);
        assert_eq!(driver.resources.clear_present_count, 0);
    }

    #[test]
    fn initialization_clear_present_is_committed_once_before_later_faults() {
        let mut driver = FakeClearDriver::new(FakeClearFault::PostPresent, false);
        let later_fault = run_initialization_clear_transaction(&mut driver)
            .expect_err("post-present fault must fail after committing the side effect");
        assert_eq!(later_fault.kind(), GpuContextErrorKind::Surface);
        assert_eq!(driver.acquire_calls, 1);
        assert_eq!(driver.present_calls, 1);
        assert_eq!(driver.configure_calls, 0);
        assert_eq!(driver.metrics.rendered_frames, 1);
        assert_eq!(driver.metrics.presented_frames, 1);
        assert_eq!(driver.resources.clear_present_count, 1);
        assert!(
            run_initialization_clear_transaction(&mut driver).is_err(),
            "retry must stay forbidden"
        );
        assert_eq!(
            driver.acquire_calls, 1,
            "second call must fail before acquire"
        );
        assert_eq!(
            driver.present_calls, 1,
            "second call must not present again"
        );
    }

    #[test]
    fn suboptimal_initialization_clear_is_committed_and_fails_closed() {
        let mut driver = FakeClearDriver::new(FakeClearFault::None, true);
        let error = run_initialization_clear_transaction(&mut driver)
            .expect_err("suboptimal clear must fail closed after present");

        assert!(
            error
                .to_string()
                .contains("suboptimal initialization clear frame")
        );
        assert_eq!(driver.acquire_calls, 1);
        assert_eq!(driver.present_calls, 1);
        assert_eq!(driver.configure_calls, 0);
        assert_eq!(driver.metrics.presented_frames, 1);
        assert_eq!(driver.resources.clear_present_count, 1);
        assert!(run_initialization_clear_transaction(&mut driver).is_err());
        assert_eq!(driver.acquire_calls, 1);
        assert_eq!(driver.present_calls, 1);
    }

    #[test]
    fn successful_initialization_clear_rejects_a_second_call_without_side_effects() {
        let mut driver = FakeClearDriver::new(FakeClearFault::None, false);
        assert_eq!(
            run_initialization_clear_transaction(&mut driver).expect("first clear"),
            GpuFrameStatus::Presented
        );
        assert_eq!(driver.acquire_calls, 1);
        assert_eq!(driver.present_calls, 1);
        assert_eq!(driver.configure_calls, 0);
        assert_eq!(driver.metrics.presented_frames, 1);
        assert_eq!(driver.resources.clear_present_count, 1);

        assert!(run_initialization_clear_transaction(&mut driver).is_err());
        assert_eq!(driver.acquire_calls, 1);
        assert_eq!(driver.present_calls, 1);
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() <= f32::EPSILON);
    }

    #[test]
    fn preferred_surface_format_preserves_terminal_srgb_values() {
        assert_eq!(
            GpuContext::preferred_surface_format(&[
                wgpu::TextureFormat::Bgra8UnormSrgb,
                wgpu::TextureFormat::Bgra8Unorm,
            ]),
            Some(wgpu::TextureFormat::Bgra8Unorm)
        );
        assert_eq!(
            GpuContext::preferred_surface_format(&[wgpu::TextureFormat::Rgba8UnormSrgb]),
            Some(wgpu::TextureFormat::Rgba8UnormSrgb)
        );
        assert_eq!(GpuContext::preferred_surface_format(&[]), None);
    }

    #[test]
    fn preferred_present_mode_uses_immediate_only_for_the_explicit_test_override() {
        let modes = [wgpu::PresentMode::Fifo, wgpu::PresentMode::Immediate];
        assert_eq!(
            preferred_present_mode(&modes, Some("immediate")),
            Some(wgpu::PresentMode::Immediate)
        );
        assert_eq!(
            preferred_present_mode(&modes, Some("fifo")),
            Some(wgpu::PresentMode::Fifo)
        );
        assert_eq!(
            preferred_present_mode(&[wgpu::PresentMode::Fifo], Some("immediate")),
            Some(wgpu::PresentMode::Fifo)
        );
    }

    #[test]
    fn compatibility_layout_matches_pixels_integer_scaling_and_letterbox() {
        let odd_surface = CompatibilityLayout::new(640, 400, 641, 401).expect("valid odd surface");
        assert_eq!(odd_surface.clip_rect, (0, 0, 640, 400));
        assert_close(odd_surface.transform[0], 640.0 / 641.0);
        assert_close(odd_surface.transform[1], 400.0 / 401.0);

        let letterboxed = CompatibilityLayout::new(640, 400, 960, 600).expect("valid letterbox");
        assert_eq!(letterboxed.clip_rect, (160, 100, 640, 400));
        assert_close(letterboxed.transform[0], 640.0 / 960.0);
        assert_close(letterboxed.transform[1], 400.0 / 600.0);

        let doubled =
            CompatibilityLayout::new(640, 400, 1_280, 800).expect("valid integer upscale");
        assert_eq!(doubled.clip_rect, (0, 0, 1_280, 800));
        assert_close(doubled.transform[0], 1.0);
        assert_close(doubled.transform[1], 1.0);
    }

    #[test]
    fn compatibility_layout_centers_and_clips_frames_larger_than_the_surface() {
        let cropped = CompatibilityLayout::new(640, 400, 320, 200).expect("valid crop");
        assert_eq!(cropped.clip_rect, (0, 0, 320, 200));
        assert!(cropped.transform[0] > 1.0);
        assert!(cropped.transform[1] > 1.0);
    }

    #[test]
    fn rgba_layout_accepts_4k_and_rejects_all_resource_limit_edges_without_panicking() {
        let four_k = RgbaFrameLayout::new(3_840, 2_160, 8_192, DEFAULT_CPU_FRAME_BYTE_BUDGET)
            .expect("4K frame fits the native texture and CPU budget");
        assert_eq!(four_k.bytes_per_row, 15_360);
        assert_eq!(four_k.byte_len, 33_177_600);

        let failures = catch_unwind(AssertUnwindSafe(|| {
            [
                RgbaFrameLayout::new(0, 1, 8_192, DEFAULT_CPU_FRAME_BYTE_BUDGET),
                RgbaFrameLayout::new(8_193, 1, 8_192, DEFAULT_CPU_FRAME_BYTE_BUDGET),
                RgbaFrameLayout::new(u32::MAX, 1, u32::MAX, DEFAULT_CPU_FRAME_BYTE_BUDGET),
                RgbaFrameLayout::new(8_192, 8_192, 8_192, 32 * 1024 * 1024),
            ]
        }))
        .expect("invalid resource requests must return errors, not panic");
        assert!(failures.into_iter().all(|result| result.is_err()));
    }

    #[test]
    fn injected_validation_and_device_loss_are_structured_and_do_not_count_frames() {
        for (fault, expected_uncaptured, expected_losses) in [
            (
                GpuRuntimeFault::Validation("injected validation".to_owned()),
                1,
                0,
            ),
            (
                GpuRuntimeFault::DeviceLost {
                    reason: wgpu::DeviceLostReason::Unknown,
                    message: "injected device loss".to_owned(),
                },
                0,
                1,
            ),
        ] {
            let monitor = GpuFaultMonitor::default();
            let mut metrics = GpuPresentationMetrics::uninitialized();
            monitor.record(fault);
            let result = catch_unwind(AssertUnwindSafe(|| {
                take_runtime_fault(&monitor, &mut metrics, "before upload")
            }))
            .expect("fault observation must return an error instead of panicking");
            let error = result.expect_err("injected fault must stop presentation");

            assert!(matches!(
                error.kind(),
                GpuContextErrorKind::Validation | GpuContextErrorKind::DeviceLost
            ));
            assert_eq!(metrics.rendered_frames, 0);
            assert_eq!(metrics.presented_frames, 0);
            assert_eq!(metrics.uncaptured_errors, expected_uncaptured);
            assert_eq!(metrics.device_losses, expected_losses);
        }
    }

    #[test]
    fn device_loss_keeps_prior_uncaptured_fault_metrics_while_taking_precedence() {
        let monitor = GpuFaultMonitor::default();
        let mut metrics = GpuPresentationMetrics::uninitialized();
        monitor.record(GpuRuntimeFault::Validation(
            "validation before loss".to_owned(),
        ));
        monitor.record(GpuRuntimeFault::DeviceLost {
            reason: wgpu::DeviceLostReason::Unknown,
            message: "device removed".to_owned(),
        });

        let error = take_runtime_fault(&monitor, &mut metrics, "after submit")
            .expect_err("device loss must stop the frame");
        assert_eq!(error.kind(), GpuContextErrorKind::DeviceLost);
        assert_eq!(metrics.uncaptured_errors, 1);
        assert_eq!(metrics.device_losses, 1);
        assert_eq!(metrics.rendered_frames, 0);
        assert_eq!(metrics.presented_frames, 0);
    }

    #[test]
    fn injected_device_loss_recovers_generation_metrics_and_submission_health() {
        let mut context =
            pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
                .expect("headless context");
        context
            .run_headless_submission_probe(Duration::from_secs(5))
            .expect("baseline submission");
        let generation = context.generation();
        let rendered_frames = context.metrics().rendered_frames;
        context.inject_device_loss_for_test();
        let error = context
            .run_headless_submission_probe(Duration::from_secs(5))
            .expect_err("injected device loss must be typed");
        assert_eq!(error.kind(), GpuContextErrorKind::DeviceLost);

        pollster::block_on(context.recover_device()).expect("recover logical device");

        assert_ne!(context.generation(), generation);
        assert_eq!(context.metrics().device_recoveries, 0);
        assert_eq!(context.metrics().abandoned_lost_surfaces, 0);
        context.commit_windowed_device_recovery();
        assert_eq!(context.metrics().device_recoveries, 1);
        assert_eq!(context.metrics().abandoned_lost_surfaces, 0);
        assert_eq!(context.metrics().device_recovery_failures, 0);
        assert_eq!(context.metrics().device_losses, 1);
        assert!(context.metrics().rendered_frames >= rendered_frames);
        context
            .run_headless_submission_probe(Duration::from_secs(5))
            .expect("recovered submission");
    }

    #[test]
    fn failed_device_recovery_is_atomic_and_counted() {
        let mut context =
            pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
                .expect("headless context");
        let generation = context.generation();
        let device = context.device().clone();
        context.inject_recovery_failures_for_test(2);

        for expected_failures in 1..=2 {
            pollster::block_on(context.recover_device())
                .expect_err("injected candidate creation failure");
            assert_eq!(context.generation(), generation);
            assert_eq!(context.device(), &device);
            assert_eq!(
                context.metrics().device_recovery_failures,
                expected_failures
            );
            assert_eq!(context.metrics().device_recoveries, 0);
        }
    }

    #[test]
    fn post_validation_recovery_failure_does_not_commit_candidate_or_surface_state() {
        let mut context =
            pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
                .expect("headless context");
        let generation = context.generation();
        let device = context.device().clone();
        let surface_config = context.surface_config.clone();
        let metrics = context.metrics().clone();
        context.inject_recovery_post_validation_failures_for_test(1);

        pollster::block_on(context.recover_device())
            .expect_err("post-validation candidate failure must remain atomic");

        assert_eq!(context.generation(), generation);
        assert_eq!(context.device(), &device);
        assert_eq!(context.surface_config, surface_config);
        assert_eq!(
            context.metrics().surface_reconfigurations,
            metrics.surface_reconfigurations
        );
        assert_eq!(context.metrics().device_recoveries, 0);
        assert_eq!(context.metrics().abandoned_lost_surfaces, 0);
        assert_eq!(
            context.metrics().device_recovery_failures,
            metrics.device_recovery_failures + 1
        );
    }

    #[test]
    fn recovery_backend_mapping_selects_exactly_one_original_backend() {
        for (backend, expected) in [
            ("Vulkan", wgpu::Backends::VULKAN),
            ("Dx12", wgpu::Backends::DX12),
            ("Metal", wgpu::Backends::METAL),
            ("Gl", wgpu::Backends::GL),
            ("BrowserWebGpu", wgpu::Backends::BROWSER_WEBGPU),
        ] {
            assert_eq!(
                GpuContextOptions::default()
                    .with_only_backend_name(backend)
                    .expect("known backend")
                    .backends,
                expected,
                "{backend} recovery must not probe a different backend"
            );
        }
        assert!(
            GpuContextOptions::default()
                .with_only_backend_name("unknown")
                .is_err(),
            "unknown recovery backends must not silently fall back to all backends"
        );
    }
}
