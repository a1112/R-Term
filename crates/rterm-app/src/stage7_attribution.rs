use std::{fmt, sync::Mutex};

#[cfg(any(test, feature = "diagnostic-tools"))]
use std::{collections::BTreeMap, time::Duration};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProductServiceEntry {
    DeferredConfig,
    ConfigWatcher,
    LocalPty,
    NativeSsh,
    PostReadyCoordinator,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[expect(
    clippy::struct_field_names,
    reason = "every audit field explicitly names the corresponding product-service start event"
)]
pub(crate) struct ProductServiceCounters {
    pub(crate) deferred_config_starts: u64,
    pub(crate) config_watcher_starts: u64,
    pub(crate) pty_starts: u64,
    pub(crate) ssh_starts: u64,
    pub(crate) post_ready_task_starts: u64,
}

impl ProductServiceCounters {
    fn record(&mut self, entry: ProductServiceEntry) {
        let counter = match entry {
            ProductServiceEntry::DeferredConfig => &mut self.deferred_config_starts,
            ProductServiceEntry::ConfigWatcher => &mut self.config_watcher_starts,
            ProductServiceEntry::LocalPty => &mut self.pty_starts,
            ProductServiceEntry::NativeSsh => &mut self.ssh_starts,
            ProductServiceEntry::PostReadyCoordinator => &mut self.post_ready_task_starts,
        };
        *counter = counter.saturating_add(1);
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct SchedulingAuditState {
    disabled: bool,
    counters: ProductServiceCounters,
}

static SCHEDULING_AUDIT_SESSION: Mutex<()> = Mutex::new(());
static SCHEDULING_AUDIT_STATE: Mutex<Option<SchedulingAuditState>> = Mutex::new(None);

/// Shared product-service scheduling gate. Ordinary production has no active
/// audit and is allowed without counters or other side effects.
pub(crate) fn audit_product_service_start(
    entry: ProductServiceEntry,
) -> Result<(), ProductServiceDisabled> {
    audit_product_service_start_in(&SCHEDULING_AUDIT_STATE, entry)
}

fn audit_product_service_start_in(
    audit: &Mutex<Option<SchedulingAuditState>>,
    entry: ProductServiceEntry,
) -> Result<(), ProductServiceDisabled> {
    let Ok(mut slot) = audit.lock() else {
        return Err(ProductServiceDisabled);
    };
    let Some(state) = slot.as_mut() else {
        return Ok(());
    };
    if state.disabled {
        return Err(ProductServiceDisabled);
    }
    state.counters.record(entry);
    Ok(())
}

#[cfg(any(test, feature = "diagnostic-tools"))]
pub(crate) struct AttributionSchedulingAuditGuard {
    _session: std::sync::MutexGuard<'static, ()>,
    previous: Option<SchedulingAuditState>,
}

#[cfg(any(test, feature = "diagnostic-tools"))]
impl AttributionSchedulingAuditGuard {
    pub(crate) fn disabled() -> Self {
        Self::install(SchedulingAuditState {
            disabled: true,
            counters: ProductServiceCounters::default(),
        })
    }

    #[cfg(test)]
    pub(crate) fn enabled_for_test() -> Self {
        Self::install(SchedulingAuditState {
            disabled: false,
            counters: ProductServiceCounters::default(),
        })
    }

    fn install(state: SchedulingAuditState) -> Self {
        let session = SCHEDULING_AUDIT_SESSION
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous = match SCHEDULING_AUDIT_STATE.lock() {
            Ok(mut slot) => slot.replace(state),
            Err(mut poisoned) => poisoned.get_mut().replace(state),
        };
        Self {
            _session: session,
            previous,
        }
    }

    #[expect(
        clippy::unused_self,
        reason = "the RAII handle scopes which installed audit is being queried"
    )]
    pub(crate) fn counters(&self) -> ProductServiceCounters {
        SCHEDULING_AUDIT_STATE.lock().map_or_else(
            |_| ProductServiceCounters::default(),
            |slot| {
                slot.as_ref()
                    .map_or_else(ProductServiceCounters::default, |state| state.counters)
            },
        )
    }
}

#[cfg(any(test, feature = "diagnostic-tools"))]
impl Drop for AttributionSchedulingAuditGuard {
    fn drop(&mut self) {
        match SCHEDULING_AUDIT_STATE.lock() {
            Ok(mut slot) => *slot = self.previous.take(),
            Err(mut poisoned) => **poisoned.get_mut() = self.previous.take(),
        }
    }
}

#[cfg(test)]
pub(crate) fn inactive_scheduling_audit_allows_for_test(entry: ProductServiceEntry) -> bool {
    let _session = SCHEDULING_AUDIT_SESSION
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    audit_product_service_start(entry).is_ok()
}

#[cfg(test)]
pub(crate) fn poisoned_scheduling_audit_fails_closed_for_test(entry: ProductServiceEntry) -> bool {
    let audit = std::sync::Arc::new(Mutex::new(None));
    let poison_target = std::sync::Arc::clone(&audit);
    let _ = std::thread::spawn(move || {
        let _locked = poison_target.lock().expect("local audit lock");
        panic!("poison local scheduling audit");
    })
    .join();
    audit_product_service_start_in(&audit, entry).is_err()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ProductServiceDisabled;

impl fmt::Display for ProductServiceDisabled {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Stage 7 attribution product services are disabled")
    }
}

impl std::error::Error for ProductServiceDisabled {}

/// Exact cumulative owner boundary held by the private Stage 7 diagnostic.
#[cfg(any(test, feature = "diagnostic-tools"))]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum GpuAttributionStage {
    CpuWindow,
    InstanceSurface,
    AdapterDevice,
    ConfiguredSurfaceClear,
    LayerPipelines,
    FixtureFontText,
    PlatformFontIndex,
    FullFrame,
}

#[cfg(any(test, feature = "diagnostic-tools"))]
impl GpuAttributionStage {
    pub(crate) const ORDERED: [Self; 8] = [
        Self::CpuWindow,
        Self::InstanceSurface,
        Self::AdapterDevice,
        Self::ConfiguredSurfaceClear,
        Self::LayerPipelines,
        Self::FixtureFontText,
        Self::PlatformFontIndex,
        Self::FullFrame,
    ];
}

/// Complete project-owned resource inventory. Driver memory is excluded.
#[cfg(any(test, feature = "diagnostic-tools"))]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ProjectOwnedResourceSnapshot {
    pub(crate) cpu_staging_bytes: u64,
    pub(crate) cpu_surface_count: u64,
    pub(crate) cpu_present_count: u64,
    pub(crate) instance_count: u64,
    pub(crate) surface_count: u64,
    pub(crate) adapter_count: u64,
    pub(crate) device_count: u64,
    pub(crate) queue_count: u64,
    pub(crate) surface_configure_count: u64,
    pub(crate) surface_acquire_count: u64,
    pub(crate) clear_present_count: u64,
    pub(crate) pipeline_count: u64,
    pub(crate) pipeline_layout_count: u64,
    pub(crate) materialized_buffer_count: u64,
    pub(crate) retained_font_bytes: u64,
    pub(crate) inactive_font_bytes: u64,
    pub(crate) indexed_font_count: u64,
    pub(crate) active_font_count: u64,
    pub(crate) catalog_builds: u64,
    pub(crate) catalog_generation: u64,
    pub(crate) glyph_atlas_bytes: u64,
    pub(crate) raster_cache_bytes: u64,
    pub(crate) image_texture_bytes: u64,
    /// `TerminalRenderSnapshot::project_owned_logical_bytes_v1`: explicit
    /// logical bytes only; allocator, Arc, and hash-control overhead excluded.
    pub(crate) snapshot_bytes: u64,
    pub(crate) instance_buffer_bytes: u64,
    pub(crate) upload_buffer_bytes: u64,
    pub(crate) total_allocated_buffer_bytes: u64,
    pub(crate) total_allocated_texture_bytes: u64,
    pub(crate) base_text_renderer_materialization_count: u64,
    pub(crate) cursor_text_renderer_materialization_count: u64,
    pub(crate) config_load_count: u64,
    pub(crate) config_watcher_count: u64,
    pub(crate) pty_start_count: u64,
    pub(crate) ssh_start_count: u64,
    pub(crate) post_ready_task_count: u64,
    pub(crate) backend: Option<String>,
    pub(crate) adapter_name: Option<String>,
}

#[cfg(any(test, feature = "diagnostic-tools"))]
const RESOURCE_FIELDS: [&str; 35] = [
    "cpu_staging_bytes",
    "cpu_surface_count",
    "cpu_present_count",
    "instance_count",
    "surface_count",
    "adapter_count",
    "device_count",
    "queue_count",
    "surface_configure_count",
    "surface_acquire_count",
    "clear_present_count",
    "pipeline_count",
    "pipeline_layout_count",
    "materialized_buffer_count",
    "retained_font_bytes",
    "inactive_font_bytes",
    "indexed_font_count",
    "active_font_count",
    "catalog_builds",
    "catalog_generation",
    "glyph_atlas_bytes",
    "raster_cache_bytes",
    "image_texture_bytes",
    "snapshot_bytes",
    "instance_buffer_bytes",
    "upload_buffer_bytes",
    "total_allocated_buffer_bytes",
    "total_allocated_texture_bytes",
    "base_text_renderer_materialization_count",
    "cursor_text_renderer_materialization_count",
    "config_load_count",
    "config_watcher_count",
    "pty_start_count",
    "ssh_start_count",
    "post_ready_task_count",
];

#[cfg(any(test, feature = "diagnostic-tools"))]
impl ProjectOwnedResourceSnapshot {
    pub(crate) fn validate_at(&self, stage: GpuAttributionStage) -> Result<(), Vec<String>> {
        validate_resource_fields(
            stage,
            &self.resource_fields(),
            self.backend.as_deref(),
            self.adapter_name.as_deref(),
        )
    }

    pub(crate) fn resource_fields(&self) -> BTreeMap<&'static str, u64> {
        BTreeMap::from([
            ("cpu_staging_bytes", self.cpu_staging_bytes),
            ("cpu_surface_count", self.cpu_surface_count),
            ("cpu_present_count", self.cpu_present_count),
            ("instance_count", self.instance_count),
            ("surface_count", self.surface_count),
            ("adapter_count", self.adapter_count),
            ("device_count", self.device_count),
            ("queue_count", self.queue_count),
            ("surface_configure_count", self.surface_configure_count),
            ("surface_acquire_count", self.surface_acquire_count),
            ("clear_present_count", self.clear_present_count),
            ("pipeline_count", self.pipeline_count),
            ("pipeline_layout_count", self.pipeline_layout_count),
            ("materialized_buffer_count", self.materialized_buffer_count),
            ("retained_font_bytes", self.retained_font_bytes),
            ("inactive_font_bytes", self.inactive_font_bytes),
            ("indexed_font_count", self.indexed_font_count),
            ("active_font_count", self.active_font_count),
            ("catalog_builds", self.catalog_builds),
            ("catalog_generation", self.catalog_generation),
            ("glyph_atlas_bytes", self.glyph_atlas_bytes),
            ("raster_cache_bytes", self.raster_cache_bytes),
            ("image_texture_bytes", self.image_texture_bytes),
            ("snapshot_bytes", self.snapshot_bytes),
            ("instance_buffer_bytes", self.instance_buffer_bytes),
            ("upload_buffer_bytes", self.upload_buffer_bytes),
            (
                "total_allocated_buffer_bytes",
                self.total_allocated_buffer_bytes,
            ),
            (
                "total_allocated_texture_bytes",
                self.total_allocated_texture_bytes,
            ),
            (
                "base_text_renderer_materialization_count",
                self.base_text_renderer_materialization_count,
            ),
            (
                "cursor_text_renderer_materialization_count",
                self.cursor_text_renderer_materialization_count,
            ),
            ("config_load_count", self.config_load_count),
            ("config_watcher_count", self.config_watcher_count),
            ("pty_start_count", self.pty_start_count),
            ("ssh_start_count", self.ssh_start_count),
            ("post_ready_task_count", self.post_ready_task_count),
        ])
    }

    #[cfg(test)]
    pub(crate) fn validate_explicit_fields_for_test(
        stage: GpuAttributionStage,
        fields: &BTreeMap<&str, u64>,
        backend: Option<&str>,
        adapter_name: Option<&str>,
    ) -> Result<(), Vec<String>> {
        validate_resource_fields(stage, fields, backend, adapter_name)
    }

    #[cfg(test)]
    pub(crate) fn exact_for_test_stage(stage: GpuAttributionStage) -> Self {
        let mut snapshot = Self {
            cpu_staging_bytes: 4,
            cpu_surface_count: 1,
            cpu_present_count: 1,
            ..Self::default()
        };
        if stage >= GpuAttributionStage::InstanceSurface {
            snapshot.instance_count = 1;
            snapshot.surface_count = 1;
        }
        if stage >= GpuAttributionStage::AdapterDevice {
            snapshot.adapter_count = 1;
            snapshot.device_count = 1;
            snapshot.queue_count = 1;
            snapshot.backend = Some("vulkan".to_owned());
            snapshot.adapter_name = Some("test-adapter".to_owned());
        }
        if stage >= GpuAttributionStage::ConfiguredSurfaceClear {
            snapshot.surface_configure_count = 1;
            snapshot.surface_acquire_count = if stage >= GpuAttributionStage::FullFrame {
                3
            } else if stage >= GpuAttributionStage::FixtureFontText {
                2
            } else {
                1
            };
            snapshot.clear_present_count = 1;
        }
        if stage >= GpuAttributionStage::LayerPipelines {
            snapshot.pipeline_count = 1;
            snapshot.pipeline_layout_count = 1;
            snapshot.materialized_buffer_count = 1;
            snapshot.total_allocated_buffer_bytes = 8;
        }
        if stage >= GpuAttributionStage::FixtureFontText {
            snapshot.retained_font_bytes = 1;
            snapshot.active_font_count = 1;
            snapshot.catalog_builds = 1;
            snapshot.catalog_generation = 1;
            snapshot.glyph_atlas_bytes = 1;
            snapshot.total_allocated_texture_bytes = 1;
            snapshot.base_text_renderer_materialization_count = 1;
        }
        if stage >= GpuAttributionStage::PlatformFontIndex {
            snapshot.indexed_font_count = 2;
        }
        if stage >= GpuAttributionStage::FullFrame {
            snapshot.snapshot_bytes = 1;
            snapshot.base_text_renderer_materialization_count = 2;
        }
        snapshot
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one exhaustive fail-closed matrix keeps every project-owned resource field auditable"
)]
#[cfg(any(test, feature = "diagnostic-tools"))]
fn validate_resource_fields(
    stage: GpuAttributionStage,
    fields: &BTreeMap<&str, u64>,
    backend: Option<&str>,
    adapter_name: Option<&str>,
) -> Result<(), Vec<String>> {
    let mut violations = Vec::new();
    for required in RESOURCE_FIELDS {
        if !fields.contains_key(required) {
            violations.push(format!("missing resource field {required}"));
        }
    }
    for field in fields.keys() {
        if !RESOURCE_FIELDS.contains(field) {
            violations.push(format!("unknown resource field {field}"));
        }
    }
    if !violations.is_empty() {
        return Err(violations);
    }

    let allowed = allowed_nonzero_fields(stage);
    for (field, value) in fields {
        if *value != 0 && !allowed.contains(field) {
            violations.push(format!(
                "{field} must remain zero at {stage:?}, got {value}"
            ));
        }
    }
    require_positive(&mut violations, fields, "cpu_staging_bytes");
    require_exact(&mut violations, fields, "cpu_surface_count", 1);
    require_exact(&mut violations, fields, "cpu_present_count", 1);
    if stage >= GpuAttributionStage::InstanceSurface {
        require_exact(&mut violations, fields, "instance_count", 1);
        require_exact(&mut violations, fields, "surface_count", 1);
    }
    if stage >= GpuAttributionStage::AdapterDevice {
        for field in ["adapter_count", "device_count", "queue_count"] {
            require_exact(&mut violations, fields, field, 1);
        }
        if backend.is_none_or(str::is_empty) {
            violations.push("backend is required from AdapterDevice onward".to_owned());
        }
        if adapter_name.is_none_or(str::is_empty) {
            violations.push("adapter_name is required from AdapterDevice onward".to_owned());
        }
    } else if backend.is_some() || adapter_name.is_some() {
        violations.push("backend and adapter_name must be absent before AdapterDevice".to_owned());
    }
    if stage >= GpuAttributionStage::ConfiguredSurfaceClear {
        require_exact(&mut violations, fields, "surface_configure_count", 1);
        require_exact(&mut violations, fields, "clear_present_count", 1);
        let acquisitions = if stage >= GpuAttributionStage::FullFrame {
            3
        } else if stage >= GpuAttributionStage::FixtureFontText {
            2
        } else {
            1
        };
        require_exact(
            &mut violations,
            fields,
            "surface_acquire_count",
            acquisitions,
        );
    }
    if stage >= GpuAttributionStage::LayerPipelines {
        require_exact(&mut violations, fields, "pipeline_count", 1);
        require_exact(&mut violations, fields, "pipeline_layout_count", 1);
        require_exact(&mut violations, fields, "materialized_buffer_count", 1);
        require_exact(&mut violations, fields, "instance_buffer_bytes", 0);
        require_exact(&mut violations, fields, "upload_buffer_bytes", 0);
        require_exact(&mut violations, fields, "total_allocated_buffer_bytes", 8);
    }
    if stage >= GpuAttributionStage::FixtureFontText {
        for field in [
            "retained_font_bytes",
            "active_font_count",
            "catalog_builds",
            "catalog_generation",
        ] {
            require_positive(&mut violations, fields, field);
        }
        require_positive(&mut violations, fields, "glyph_atlas_bytes");
        if let Some(texture_bytes) = fields
            .get("glyph_atlas_bytes")
            .copied()
            .unwrap_or_default()
            .checked_add(
                fields
                    .get("image_texture_bytes")
                    .copied()
                    .unwrap_or_default(),
            )
        {
            require_exact(
                &mut violations,
                fields,
                "total_allocated_texture_bytes",
                texture_bytes,
            );
        } else {
            violations.push("project-owned texture byte total overflowed".to_owned());
        }
        let text_renderer_count = if stage >= GpuAttributionStage::FullFrame {
            2
        } else {
            1
        };
        require_exact(
            &mut violations,
            fields,
            "base_text_renderer_materialization_count",
            text_renderer_count,
        );
        require_exact(
            &mut violations,
            fields,
            "cursor_text_renderer_materialization_count",
            0,
        );
    }
    if stage >= GpuAttributionStage::PlatformFontIndex {
        require_positive(&mut violations, fields, "indexed_font_count");
        require_exact(&mut violations, fields, "inactive_font_bytes", 0);
    }
    if stage >= GpuAttributionStage::FullFrame {
        require_exact(&mut violations, fields, "image_texture_bytes", 0);
        require_positive(&mut violations, fields, "snapshot_bytes");
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the fail-closed matrix spells out every permitted nonzero field at all eight boundaries"
)]
#[cfg(any(test, feature = "diagnostic-tools"))]
fn allowed_nonzero_fields(stage: GpuAttributionStage) -> &'static [&'static str] {
    match stage {
        GpuAttributionStage::CpuWindow => &[
            "cpu_staging_bytes",
            "cpu_surface_count",
            "cpu_present_count",
        ],
        GpuAttributionStage::InstanceSurface => &[
            "cpu_staging_bytes",
            "cpu_surface_count",
            "cpu_present_count",
            "instance_count",
            "surface_count",
        ],
        GpuAttributionStage::AdapterDevice => &[
            "cpu_staging_bytes",
            "cpu_surface_count",
            "cpu_present_count",
            "instance_count",
            "surface_count",
            "adapter_count",
            "device_count",
            "queue_count",
        ],
        GpuAttributionStage::ConfiguredSurfaceClear => &[
            "cpu_staging_bytes",
            "cpu_surface_count",
            "cpu_present_count",
            "instance_count",
            "surface_count",
            "adapter_count",
            "device_count",
            "queue_count",
            "surface_configure_count",
            "surface_acquire_count",
            "clear_present_count",
        ],
        GpuAttributionStage::LayerPipelines => &[
            "cpu_staging_bytes",
            "cpu_surface_count",
            "cpu_present_count",
            "instance_count",
            "surface_count",
            "adapter_count",
            "device_count",
            "queue_count",
            "surface_configure_count",
            "surface_acquire_count",
            "clear_present_count",
            "pipeline_count",
            "pipeline_layout_count",
            "materialized_buffer_count",
            "total_allocated_buffer_bytes",
        ],
        GpuAttributionStage::FixtureFontText => &[
            "cpu_staging_bytes",
            "cpu_surface_count",
            "cpu_present_count",
            "instance_count",
            "surface_count",
            "adapter_count",
            "device_count",
            "queue_count",
            "surface_configure_count",
            "surface_acquire_count",
            "clear_present_count",
            "pipeline_count",
            "pipeline_layout_count",
            "materialized_buffer_count",
            "retained_font_bytes",
            "active_font_count",
            "catalog_builds",
            "catalog_generation",
            "glyph_atlas_bytes",
            "raster_cache_bytes",
            "instance_buffer_bytes",
            "upload_buffer_bytes",
            "total_allocated_buffer_bytes",
            "total_allocated_texture_bytes",
            "base_text_renderer_materialization_count",
            "cursor_text_renderer_materialization_count",
        ],
        GpuAttributionStage::PlatformFontIndex => &[
            "cpu_staging_bytes",
            "cpu_surface_count",
            "cpu_present_count",
            "instance_count",
            "surface_count",
            "adapter_count",
            "device_count",
            "queue_count",
            "surface_configure_count",
            "surface_acquire_count",
            "clear_present_count",
            "pipeline_count",
            "pipeline_layout_count",
            "materialized_buffer_count",
            "retained_font_bytes",
            "indexed_font_count",
            "active_font_count",
            "catalog_builds",
            "catalog_generation",
            "glyph_atlas_bytes",
            "raster_cache_bytes",
            "instance_buffer_bytes",
            "upload_buffer_bytes",
            "total_allocated_buffer_bytes",
            "total_allocated_texture_bytes",
            "base_text_renderer_materialization_count",
            "cursor_text_renderer_materialization_count",
        ],
        GpuAttributionStage::FullFrame => &[
            "cpu_staging_bytes",
            "cpu_surface_count",
            "cpu_present_count",
            "instance_count",
            "surface_count",
            "adapter_count",
            "device_count",
            "queue_count",
            "surface_configure_count",
            "surface_acquire_count",
            "clear_present_count",
            "pipeline_count",
            "pipeline_layout_count",
            "materialized_buffer_count",
            "retained_font_bytes",
            "indexed_font_count",
            "active_font_count",
            "catalog_builds",
            "catalog_generation",
            "glyph_atlas_bytes",
            "raster_cache_bytes",
            "snapshot_bytes",
            "instance_buffer_bytes",
            "upload_buffer_bytes",
            "total_allocated_buffer_bytes",
            "total_allocated_texture_bytes",
            "base_text_renderer_materialization_count",
            "cursor_text_renderer_materialization_count",
        ],
    }
}

#[cfg(any(test, feature = "diagnostic-tools"))]
fn require_exact(
    violations: &mut Vec<String>,
    fields: &BTreeMap<&str, u64>,
    field: &str,
    expected: u64,
) {
    let actual = fields.get(field).copied().unwrap_or_default();
    if actual != expected {
        violations.push(format!("{field} must be {expected}, got {actual}"));
    }
}

#[cfg(any(test, feature = "diagnostic-tools"))]
fn require_positive(violations: &mut Vec<String>, fields: &BTreeMap<&str, u64>, field: &str) {
    if fields.get(field).copied().unwrap_or_default() == 0 {
        violations.push(format!("{field} must be positive"));
    }
}

#[cfg(any(test, feature = "diagnostic-tools"))]
pub(crate) trait AttributionStageRuntime {
    type Error;

    fn disable_product_services(&mut self);
    fn complete_stage(
        &mut self,
        stage: GpuAttributionStage,
    ) -> Result<ProjectOwnedResourceSnapshot, Self::Error>;
    fn hold(&mut self, duration: Duration);
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg(any(test, feature = "diagnostic-tools"))]
pub(crate) struct AttributionHoldReport {
    pub(crate) held_stage: GpuAttributionStage,
    pub(crate) resources: ProjectOwnedResourceSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(any(test, feature = "diagnostic-tools"))]
pub(crate) struct AttributionStageController {
    stop_stage: GpuAttributionStage,
}

#[cfg(any(test, feature = "diagnostic-tools"))]
impl AttributionStageController {
    pub(crate) const fn new(stop_stage: GpuAttributionStage) -> Self {
        Self { stop_stage }
    }

    pub(crate) fn run<R: AttributionStageRuntime>(
        self,
        runtime: &mut R,
    ) -> Result<AttributionHoldReport, AttributionControllerError<R::Error>> {
        let _scheduling_audit = AttributionSchedulingAuditGuard::disabled();
        runtime.disable_product_services();
        let mut resources = None;
        for stage in GpuAttributionStage::ORDERED {
            let current = runtime
                .complete_stage(stage)
                .map_err(AttributionControllerError::Owner)?;
            current
                .validate_at(stage)
                .map_err(AttributionControllerError::ResourceMatrix)?;
            resources = Some(current);
            if stage == self.stop_stage {
                break;
            }
        }
        runtime.hold(Duration::from_secs(5));
        Ok(AttributionHoldReport {
            held_stage: self.stop_stage,
            resources: resources.expect("CpuWindow is always completed"),
        })
    }
}

#[derive(Debug)]
#[cfg(any(test, feature = "diagnostic-tools"))]
pub(crate) enum AttributionControllerError<E> {
    Owner(E),
    ResourceMatrix(Vec<String>),
}

#[cfg(any(test, feature = "diagnostic-tools"))]
impl<E: fmt::Display> fmt::Display for AttributionControllerError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Owner(error) => write!(formatter, "attribution owner failed: {error}"),
            Self::ResourceMatrix(violations) => write!(
                formatter,
                "attribution resource matrix failed: {}",
                violations.join("; ")
            ),
        }
    }
}
