use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize, ser::SerializeStruct};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaVersion {
    #[serde(rename = "rterm.diagnostics/v2")]
    V2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticRendererMode {
    Auto,
    Cpu,
    Gpu,
}

impl Default for DiagnosticRendererMode {
    fn default() -> Self {
        Self::Auto
    }
}

impl DiagnosticRendererMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Cpu => "cpu",
            Self::Gpu => "gpu",
        }
    }
}

impl Display for DiagnosticRendererMode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for DiagnosticRendererMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "auto" => Ok(Self::Auto),
            "cpu" => Ok(Self::Cpu),
            "gpu" => Ok(Self::Gpu),
            _ => Err(format!("unsupported diagnostic renderer mode: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticGpuBackend {
    Dx12,
    Vulkan,
    Gl,
}

impl DiagnosticGpuBackend {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dx12 => "dx12",
            Self::Vulkan => "vulkan",
            Self::Gl => "gl",
        }
    }
}

impl Display for DiagnosticGpuBackend {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for DiagnosticGpuBackend {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "dx12" => Ok(Self::Dx12),
            "vulkan" => Ok(Self::Vulkan),
            "gl" => Ok(Self::Gl),
            _ => Err(format!("unsupported diagnostic GPU backend: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticFontMode {
    #[serde(rename = "current")]
    CurrentCopied,
    #[serde(rename = "shared")]
    SharedAll,
    Lazy,
}

impl DiagnosticFontMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CurrentCopied => "current",
            Self::SharedAll => "shared",
            Self::Lazy => "lazy",
        }
    }
}

impl Display for DiagnosticFontMode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for DiagnosticFontMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "current" => Ok(Self::CurrentCopied),
            "shared" => Ok(Self::SharedAll),
            "lazy" => Ok(Self::Lazy),
            _ => Err(format!("unsupported diagnostic font mode: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticFontSpecimen {
    Ascii,
    Cjk,
    Emoji,
}

impl DiagnosticFontSpecimen {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ascii => "ascii",
            Self::Cjk => "cjk",
            Self::Emoji => "emoji",
        }
    }
}

impl Display for DiagnosticFontSpecimen {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for DiagnosticFontSpecimen {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ascii" => Ok(Self::Ascii),
            "cjk" => Ok(Self::Cjk),
            "emoji" => Ok(Self::Emoji),
            _ => Err(format!("unsupported diagnostic font specimen: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticAttributionStage {
    CpuWindow,
    InstanceSurface,
    AdapterDevice,
    ConfiguredSurfaceClear,
    LayerPipelines,
    FixtureFontText,
    PlatformFontIndex,
    FullFrame,
}

impl DiagnosticAttributionStage {
    pub const ORDERED: [Self; 8] = [
        Self::CpuWindow,
        Self::InstanceSurface,
        Self::AdapterDevice,
        Self::ConfiguredSurfaceClear,
        Self::LayerPipelines,
        Self::FixtureFontText,
        Self::PlatformFontIndex,
        Self::FullFrame,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CpuWindow => "cpu-window",
            Self::InstanceSurface => "instance-surface",
            Self::AdapterDevice => "adapter-device",
            Self::ConfiguredSurfaceClear => "configured-surface-clear",
            Self::LayerPipelines => "layer-pipelines",
            Self::FixtureFontText => "fixture-font-text",
            Self::PlatformFontIndex => "platform-font-index",
            Self::FullFrame => "full-frame",
        }
    }
}

impl Display for DiagnosticAttributionStage {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for DiagnosticAttributionStage {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "cpu-window" => Ok(Self::CpuWindow),
            "instance-surface" => Ok(Self::InstanceSurface),
            "adapter-device" => Ok(Self::AdapterDevice),
            "configured-surface-clear" => Ok(Self::ConfiguredSurfaceClear),
            "layer-pipelines" => Ok(Self::LayerPipelines),
            "fixture-font-text" => Ok(Self::FixtureFontText),
            "platform-font-index" => Ok(Self::PlatformFontIndex),
            "full-frame" => Ok(Self::FullFrame),
            _ => Err(format!("unsupported diagnostic attribution stage: {value}")),
        }
    }
}

#[cfg(test)]
mod font_mode_tests {
    use super::*;

    #[test]
    fn font_mode_and_specimen_have_stable_private_wire_values() {
        for (wire, mode) in [
            ("current", DiagnosticFontMode::CurrentCopied),
            ("shared", DiagnosticFontMode::SharedAll),
            ("lazy", DiagnosticFontMode::Lazy),
        ] {
            assert_eq!(wire.parse::<DiagnosticFontMode>(), Ok(mode));
            assert_eq!(mode.to_string(), wire);
            assert_eq!(serde_json::to_value(mode).unwrap(), wire);
        }
        for (wire, specimen) in [
            ("ascii", DiagnosticFontSpecimen::Ascii),
            ("cjk", DiagnosticFontSpecimen::Cjk),
            ("emoji", DiagnosticFontSpecimen::Emoji),
        ] {
            assert_eq!(wire.parse::<DiagnosticFontSpecimen>(), Ok(specimen));
            assert_eq!(specimen.to_string(), wire);
            assert_eq!(serde_json::to_value(specimen).unwrap(), wire);
        }
    }

    #[test]
    fn font_mode_fields_are_omitted_from_default_configuration_json() {
        let default_json = serde_json::to_value(RunConfiguration::default()).unwrap();
        assert!(default_json.get("requested_font_mode").is_none());
        assert!(default_json.get("requested_font_specimen").is_none());

        let configured = RunConfiguration {
            requested_font_mode: Some(DiagnosticFontMode::Lazy),
            requested_font_specimen: Some(DiagnosticFontSpecimen::Emoji),
            ..RunConfiguration::default()
        };
        let configured_json = serde_json::to_value(configured).unwrap();
        assert_eq!(configured_json["requested_renderer"], "auto");
        assert_eq!(configured_json["requested_font_mode"], "lazy");
        assert_eq!(configured_json["requested_font_specimen"], "emoji");
    }

    #[test]
    fn absent_font_ownership_milestone_preserves_the_default_wire_shape() {
        let mut milestones = StartupMilestones::default();
        let default_json = serde_json::to_value(&milestones).unwrap();
        assert!(default_json.get("font_ownership_ready_ms").is_none());

        milestones.font_ownership_ready_ms = Some(125);
        let ready_json = serde_json::to_value(milestones).unwrap();
        assert_eq!(ready_json["font_ownership_ready_ms"], 125);
    }

    #[test]
    fn font_resource_summary_is_optional_and_uses_irreversible_wire_identity() {
        let mut result = DiagnosticsResult::successful_fixture(
            RunIdentity::fixture(Scenario::EmptyWindow, Platform::Windows),
            MemoryMetric::WindowsPrivateWorkingSetBytes,
            RunConfiguration::default(),
        );
        assert!(
            serde_json::to_value(&result)
                .unwrap()
                .get("font_resources")
                .is_none(),
            "legacy diagnostics JSON must remain byte-shape compatible when no proof is requested"
        );

        result.font_resources = Some(DiagnosticFontResourceSummary {
            mode: DiagnosticFontMode::SharedAll,
            specimen: DiagnosticFontSpecimen::Ascii,
            retained_source_bytes: 64,
            indexed_source_count: 3,
            active_source_count: 2,
            initial_catalog_source_count: 2,
            catalog_builds: 2,
            generation: 2,
            recovery_retained_source_bytes: 64,
            recovery_generation: 2,
            activation_latency_micros: 9,
            tofu_count: 0,
            frame_catalog_generation: Some(2),
            frame_generation_consistent: Some(true),
            index_fingerprint_sha256: "1".repeat(64),
            catalog_fingerprint_sha256: "2".repeat(64),
            ordered_catalog_fingerprint_sha256: "3".repeat(64),
            font_inventory_fingerprint_sha256: Some("4".repeat(64)),
            font_index_policy_version: Some(1),
        });
        let value = serde_json::to_value(result).unwrap();
        assert_eq!(value["font_resources"]["mode"], "shared");
        assert_eq!(value["font_resources"]["specimen"], "ascii");
        assert_eq!(value["font_resources"]["initial_catalog_source_count"], 2);
        assert_eq!(value["font_resources"]["frame_generation_consistent"], true);
        assert_eq!(
            value["font_resources"]["font_inventory_fingerprint_sha256"],
            "4".repeat(64)
        );
        assert_eq!(value["font_resources"]["font_index_policy_version"], 1);
        assert!(
            !value["font_resources"]
                .to_string()
                .to_ascii_lowercase()
                .contains("path")
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scenario {
    EmptyWindow,
    Ssh1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Windows,
    Linux,
    Macos,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunIdentity {
    pub id: String,
    pub scenario: Scenario,
    pub platform: Platform,
    pub architecture: String,
    pub app_path: String,
    pub app_version: String,
    pub started_at_utc: String,
    pub launcher_version: String,
}

impl RunIdentity {
    #[must_use]
    pub fn fixture(scenario: Scenario, platform: Platform) -> Self {
        Self {
            id: "fixture-run".to_owned(),
            scenario,
            platform,
            architecture: "fixture-arch".to_owned(),
            app_path: "fixture-app".to_owned(),
            app_version: "0.0.0-fixture".to_owned(),
            started_at_utc: "1970-01-01T00:00:00Z".to_owned(),
            launcher_version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RunConfiguration {
    pub stabilization_ms: u64,
    pub sample_interval_ms: u64,
    pub sample_count: u32,
    pub columns: u16,
    pub rows: u16,
    pub scale_factor_milli: u16,
    #[serde(default)]
    pub requested_renderer: DiagnosticRendererMode,
    #[serde(default)]
    pub requested_gpu_backend: Option<DiagnosticGpuBackend>,
    #[serde(default)]
    pub requested_font_mode: Option<DiagnosticFontMode>,
    #[serde(default)]
    pub requested_font_specimen: Option<DiagnosticFontSpecimen>,
    #[serde(default)]
    pub requested_attribution_stage: Option<DiagnosticAttributionStage>,
}

impl Serialize for RunConfiguration {
    fn serialize<Serializer>(
        &self,
        serializer: Serializer,
    ) -> Result<Serializer::Ok, Serializer::Error>
    where
        Serializer: serde::Serializer,
    {
        let font_proof =
            self.requested_font_mode.is_some() && self.requested_font_specimen.is_some();
        let serialize_renderer = self.requested_renderer != DiagnosticRendererMode::Auto
            || self.requested_gpu_backend.is_some()
            || font_proof
            || self.requested_attribution_stage.is_some();
        let field_count = 6
            + usize::from(serialize_renderer)
            + usize::from(self.requested_gpu_backend.is_some())
            + usize::from(self.requested_font_mode.is_some())
            + usize::from(self.requested_font_specimen.is_some())
            + usize::from(self.requested_attribution_stage.is_some());
        let mut configuration = serializer.serialize_struct("RunConfiguration", field_count)?;
        configuration.serialize_field("stabilization_ms", &self.stabilization_ms)?;
        configuration.serialize_field("sample_interval_ms", &self.sample_interval_ms)?;
        configuration.serialize_field("sample_count", &self.sample_count)?;
        configuration.serialize_field("columns", &self.columns)?;
        configuration.serialize_field("rows", &self.rows)?;
        configuration.serialize_field("scale_factor_milli", &self.scale_factor_milli)?;
        if serialize_renderer {
            configuration.serialize_field("requested_renderer", &self.requested_renderer)?;
        }
        if let Some(backend) = self.requested_gpu_backend {
            configuration.serialize_field("requested_gpu_backend", &backend)?;
        }
        if let Some(mode) = self.requested_font_mode {
            configuration.serialize_field("requested_font_mode", &mode)?;
        }
        if let Some(specimen) = self.requested_font_specimen {
            configuration.serialize_field("requested_font_specimen", &specimen)?;
        }
        if let Some(stage) = self.requested_attribution_stage {
            configuration.serialize_field("requested_attribution_stage", &stage)?;
        }
        configuration.end()
    }
}

impl Default for RunConfiguration {
    fn default() -> Self {
        Self {
            stabilization_ms: 5_000,
            sample_interval_ms: 100,
            sample_count: 10,
            columns: 80,
            rows: 24,
            scale_factor_milli: 1_000,
            requested_renderer: DiagnosticRendererMode::Auto,
            requested_gpu_backend: None,
            requested_font_mode: None,
            requested_font_specimen: None,
            requested_attribution_stage: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartupMilestones {
    pub process_started_ms: u64,
    pub window_created_ms: Option<u64>,
    pub first_present_ms: Option<u64>,
    pub config_ready_ms: Option<u64>,
    pub transport_started_ms: Option<u64>,
    pub transport_ready_ms: Option<u64>,
    pub gpu_ready_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_ownership_ready_ms: Option<u64>,
    pub scenario_ready_ms: Option<u64>,
    pub sampling_started_ms: Option<u64>,
    pub sampling_finished_ms: Option<u64>,
    pub process_exited_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessStatus {
    Pending,
    Ready,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Readiness {
    pub status: ReadinessStatus,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RendererKind {
    Cpu,
    Gpu,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RendererSummary {
    pub first: Option<RendererKind>,
    #[serde(rename = "final")]
    pub final_renderer: Option<RendererKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<DiagnosticGpuBackend>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter_vendor_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter_device_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter_type: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    NotStarted,
    Pending,
    Connecting,
    AwaitingSecret,
    AwaitingHostKey,
    Connected,
    Disconnected,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionSummary {
    pub final_state: ConnectionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryMetric {
    WindowsPrivateWorkingSetBytes,
    LinuxPssBytes,
    MacosPhysFootprintBytes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemorySample {
    pub sequence: u32,
    pub elapsed_ms: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryStatistics {
    pub count: u32,
    pub min: u64,
    pub max: u64,
    pub mean: u64,
    pub median: u64,
    pub p50: u64,
    pub p95: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemorySummary {
    pub metric: MemoryMetric,
    pub unit: String,
    pub samples: Vec<MemorySample>,
    pub statistics: MemoryStatistics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessExitKind {
    Running,
    Natural,
    Requested,
    Forced,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessSummary {
    pub pid: u32,
    pub exit_kind: ProcessExitKind,
    pub exit_code: Option<i32>,
    pub teardown_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticFailure {
    pub code: String,
    pub phase: String,
    pub message: String,
    pub os_error_code: Option<i64>,
    pub recoverable: bool,
    pub context: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticFontResourceSummary {
    pub mode: DiagnosticFontMode,
    pub specimen: DiagnosticFontSpecimen,
    pub retained_source_bytes: usize,
    pub indexed_source_count: usize,
    pub active_source_count: usize,
    pub initial_catalog_source_count: usize,
    pub catalog_builds: u64,
    pub generation: u64,
    pub recovery_retained_source_bytes: usize,
    pub recovery_generation: u64,
    pub activation_latency_micros: u64,
    pub tofu_count: usize,
    pub frame_catalog_generation: Option<u64>,
    pub frame_generation_consistent: Option<bool>,
    pub index_fingerprint_sha256: String,
    pub catalog_fingerprint_sha256: String,
    pub ordered_catalog_fingerprint_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_inventory_fingerprint_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_index_policy_version: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectOwnedResourceSchemaVersion {
    #[serde(rename = "rterm.project-owned-resources/v1")]
    V1,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectOwnedResourceMetricsV1 {
    pub cpu_staging_bytes: u64,
    pub cpu_surface_count: u64,
    pub cpu_present_count: u64,
    pub instance_count: u64,
    pub surface_count: u64,
    pub adapter_count: u64,
    pub device_count: u64,
    pub queue_count: u64,
    pub surface_configure_count: u64,
    pub surface_acquire_count: u64,
    pub clear_present_count: u64,
    pub pipeline_count: u64,
    pub pipeline_layout_count: u64,
    pub materialized_buffer_count: u64,
    pub retained_font_bytes: u64,
    pub inactive_font_bytes: u64,
    pub indexed_font_count: u64,
    pub active_font_count: u64,
    pub catalog_builds: u64,
    pub catalog_generation: u64,
    pub glyph_atlas_bytes: u64,
    pub raster_cache_bytes: u64,
    pub image_texture_bytes: u64,
    pub snapshot_bytes: u64,
    pub instance_buffer_bytes: u64,
    pub upload_buffer_bytes: u64,
    pub total_allocated_buffer_bytes: u64,
    pub total_allocated_texture_bytes: u64,
    pub base_text_renderer_materialization_count: u64,
    pub cursor_text_renderer_materialization_count: u64,
    pub config_load_count: u64,
    pub config_watcher_count: u64,
    pub pty_start_count: u64,
    pub ssh_start_count: u64,
    pub post_ready_task_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<DiagnosticGpuBackend>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter_name: Option<String>,
}

impl ProjectOwnedResourceMetricsV1 {
    #[must_use]
    pub fn numeric_fields(&self) -> [(&'static str, u64); 35] {
        [
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
        ]
    }

    /// Validates the exact cumulative project-owned resource boundary.
    ///
    /// # Errors
    ///
    /// Returns every violated field invariant when a resource row includes a
    /// later-stage counter, omits a required counter, or has invalid ownership
    /// identity for the requested boundary.
    #[expect(
        clippy::too_many_lines,
        reason = "the closed v1 matrix validates every resource boundary explicitly"
    )]
    pub fn validate_at(&self, stage: DiagnosticAttributionStage) -> Result<(), Vec<String>> {
        let mut violations = Vec::new();
        let fields = self.numeric_fields();
        let allowed = attribution_allowed_nonzero_fields(stage);
        for (field, value) in fields {
            if value != 0 && !allowed.contains(&field) {
                violations.push(format!(
                    "{field} must remain zero at {}, got {value}",
                    stage.as_str()
                ));
            }
        }
        require_attribution_positive(&mut violations, "cpu_staging_bytes", self.cpu_staging_bytes);
        require_attribution_exact(
            &mut violations,
            "cpu_surface_count",
            self.cpu_surface_count,
            1,
        );
        require_attribution_exact(
            &mut violations,
            "cpu_present_count",
            self.cpu_present_count,
            1,
        );
        if stage >= DiagnosticAttributionStage::InstanceSurface {
            require_attribution_exact(&mut violations, "instance_count", self.instance_count, 1);
            require_attribution_exact(&mut violations, "surface_count", self.surface_count, 1);
        }
        if stage >= DiagnosticAttributionStage::AdapterDevice {
            require_attribution_exact(&mut violations, "adapter_count", self.adapter_count, 1);
            require_attribution_exact(&mut violations, "device_count", self.device_count, 1);
            require_attribution_exact(&mut violations, "queue_count", self.queue_count, 1);
            if self.backend.is_none() {
                violations.push("backend is required from adapter-device onward".to_owned());
            }
            if self.adapter_name.as_deref().is_none_or(str::is_empty) {
                violations.push("adapter_name is required from adapter-device onward".to_owned());
            }
        } else if self.backend.is_some() || self.adapter_name.is_some() {
            violations
                .push("backend and adapter_name must be absent before adapter-device".to_owned());
        }
        if stage >= DiagnosticAttributionStage::ConfiguredSurfaceClear {
            require_attribution_exact(
                &mut violations,
                "surface_configure_count",
                self.surface_configure_count,
                1,
            );
            require_attribution_exact(
                &mut violations,
                "clear_present_count",
                self.clear_present_count,
                1,
            );
            let expected_acquires = if stage >= DiagnosticAttributionStage::FullFrame {
                3
            } else if stage >= DiagnosticAttributionStage::FixtureFontText {
                2
            } else {
                1
            };
            require_attribution_exact(
                &mut violations,
                "surface_acquire_count",
                self.surface_acquire_count,
                expected_acquires,
            );
        }
        if stage >= DiagnosticAttributionStage::LayerPipelines {
            require_attribution_exact(&mut violations, "pipeline_count", self.pipeline_count, 1);
            require_attribution_exact(
                &mut violations,
                "pipeline_layout_count",
                self.pipeline_layout_count,
                1,
            );
            require_attribution_exact(
                &mut violations,
                "materialized_buffer_count",
                self.materialized_buffer_count,
                1,
            );
            require_attribution_exact(
                &mut violations,
                "instance_buffer_bytes",
                self.instance_buffer_bytes,
                0,
            );
            require_attribution_exact(
                &mut violations,
                "upload_buffer_bytes",
                self.upload_buffer_bytes,
                0,
            );
            require_attribution_exact(
                &mut violations,
                "total_allocated_buffer_bytes",
                self.total_allocated_buffer_bytes,
                8,
            );
        }
        if stage >= DiagnosticAttributionStage::FixtureFontText {
            for (field, value) in [
                ("retained_font_bytes", self.retained_font_bytes),
                ("active_font_count", self.active_font_count),
                ("catalog_builds", self.catalog_builds),
                ("catalog_generation", self.catalog_generation),
                ("glyph_atlas_bytes", self.glyph_atlas_bytes),
            ] {
                require_attribution_positive(&mut violations, field, value);
            }
            match self.glyph_atlas_bytes.checked_add(self.image_texture_bytes) {
                Some(expected) => require_attribution_exact(
                    &mut violations,
                    "total_allocated_texture_bytes",
                    self.total_allocated_texture_bytes,
                    expected,
                ),
                None => violations.push("project-owned texture byte total overflowed".to_owned()),
            }
            let expected_renderers = if stage >= DiagnosticAttributionStage::FullFrame {
                2
            } else {
                1
            };
            require_attribution_exact(
                &mut violations,
                "base_text_renderer_materialization_count",
                self.base_text_renderer_materialization_count,
                expected_renderers,
            );
            require_attribution_exact(
                &mut violations,
                "cursor_text_renderer_materialization_count",
                self.cursor_text_renderer_materialization_count,
                0,
            );
        }
        if stage >= DiagnosticAttributionStage::PlatformFontIndex {
            require_attribution_positive(
                &mut violations,
                "indexed_font_count",
                self.indexed_font_count,
            );
            require_attribution_exact(
                &mut violations,
                "inactive_font_bytes",
                self.inactive_font_bytes,
                0,
            );
        }
        if stage >= DiagnosticAttributionStage::FullFrame {
            require_attribution_exact(
                &mut violations,
                "image_texture_bytes",
                self.image_texture_bytes,
                0,
            );
            require_attribution_positive(&mut violations, "snapshot_bytes", self.snapshot_bytes);
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
}

fn require_attribution_exact(
    violations: &mut Vec<String>,
    field: &str,
    actual: u64,
    expected: u64,
) {
    if actual != expected {
        violations.push(format!("{field} must be {expected}, got {actual}"));
    }
}

fn require_attribution_positive(violations: &mut Vec<String>, field: &str, actual: u64) {
    if actual == 0 {
        violations.push(format!("{field} must be positive"));
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the closed v1 matrix explicitly names every resource permitted at each boundary"
)]
fn attribution_allowed_nonzero_fields(
    stage: DiagnosticAttributionStage,
) -> &'static [&'static str] {
    match stage {
        DiagnosticAttributionStage::CpuWindow => &[
            "cpu_staging_bytes",
            "cpu_surface_count",
            "cpu_present_count",
        ],
        DiagnosticAttributionStage::InstanceSurface => &[
            "cpu_staging_bytes",
            "cpu_surface_count",
            "cpu_present_count",
            "instance_count",
            "surface_count",
        ],
        DiagnosticAttributionStage::AdapterDevice => &[
            "cpu_staging_bytes",
            "cpu_surface_count",
            "cpu_present_count",
            "instance_count",
            "surface_count",
            "adapter_count",
            "device_count",
            "queue_count",
        ],
        DiagnosticAttributionStage::ConfiguredSurfaceClear => &[
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
        DiagnosticAttributionStage::LayerPipelines => &[
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
        DiagnosticAttributionStage::FixtureFontText => &[
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
        DiagnosticAttributionStage::PlatformFontIndex => &[
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
        DiagnosticAttributionStage::FullFrame => &[
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsResult {
    pub schema: SchemaVersion,
    pub run: RunIdentity,
    pub configuration: RunConfiguration,
    pub milestones: StartupMilestones,
    pub readiness: Readiness,
    pub renderer: RendererSummary,
    pub connection: ConnectionSummary,
    pub memory: MemorySummary,
    pub process: ProcessSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_resources: Option<DiagnosticFontResourceSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_attribution_stage: Option<DiagnosticAttributionStage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_summary_schema: Option<ProjectOwnedResourceSchemaVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_summary: Option<ProjectOwnedResourceMetricsV1>,
    pub failures: Vec<DiagnosticFailure>,
}

impl DiagnosticsResult {
    #[must_use]
    pub fn successful_fixture(
        run: RunIdentity,
        metric: MemoryMetric,
        configuration: RunConfiguration,
    ) -> Self {
        let sample_count = configuration.sample_count;
        let samples = (0..configuration.sample_count)
            .map(|sequence| MemorySample {
                sequence,
                elapsed_ms: u64::from(sequence) * configuration.sample_interval_ms,
                bytes: 1,
            })
            .collect();
        Self {
            schema: SchemaVersion::V2,
            run,
            configuration,
            milestones: StartupMilestones::default(),
            readiness: Readiness {
                status: ReadinessStatus::Ready,
                evidence: vec!["fixture".to_owned()],
            },
            renderer: RendererSummary {
                first: Some(RendererKind::Cpu),
                final_renderer: Some(RendererKind::Cpu),
                ..RendererSummary::default()
            },
            connection: ConnectionSummary {
                final_state: ConnectionState::NotStarted,
            },
            memory: MemorySummary {
                metric,
                unit: "bytes".to_owned(),
                samples,
                statistics: MemoryStatistics {
                    count: sample_count,
                    min: 1,
                    max: 1,
                    mean: 1,
                    median: 1,
                    p50: 1,
                    p95: 1,
                },
            },
            process: ProcessSummary {
                pid: 1,
                exit_kind: ProcessExitKind::Requested,
                exit_code: Some(0),
                teardown_ms: Some(0),
            },
            font_resources: None,
            final_attribution_stage: None,
            resource_summary_schema: None,
            resource_summary: None,
            failures: Vec::new(),
        }
    }

    /// Validates the cross-field invariants required by schema v2.
    ///
    /// # Errors
    ///
    /// Returns an error when a sampling parameter is zero, a successful run does not
    /// contain the configured number of samples, or the memory unit is not bytes.
    pub fn validate(&self) -> Result<(), SchemaValidationError> {
        if self.configuration.stabilization_ms == 0 {
            return Err(SchemaValidationError::ZeroConfiguration("stabilization_ms"));
        }
        if self.configuration.sample_interval_ms == 0 {
            return Err(SchemaValidationError::ZeroConfiguration(
                "sample_interval_ms",
            ));
        }
        if self.configuration.sample_count == 0 {
            return Err(SchemaValidationError::ZeroConfiguration("sample_count"));
        }
        let observed = u32::try_from(self.memory.samples.len()).unwrap_or(u32::MAX);
        if self.failures.is_empty() && observed != self.configuration.sample_count {
            return Err(SchemaValidationError::SampleCount {
                expected: self.configuration.sample_count,
                observed,
            });
        }
        if self.memory.unit != "bytes" {
            return Err(SchemaValidationError::MemoryUnit(self.memory.unit.clone()));
        }
        let evidence_present = self.final_attribution_stage.is_some()
            || self.resource_summary_schema.is_some()
            || self.resource_summary.is_some();
        match self.configuration.requested_attribution_stage {
            None if evidence_present => {
                return Err(SchemaValidationError::Attribution(
                    "unrequested attribution evidence is forbidden".to_owned(),
                ));
            }
            None => {}
            Some(_) if !self.failures.is_empty() && !evidence_present => {}
            Some(requested) => {
                let final_stage = self.final_attribution_stage.ok_or_else(|| {
                    SchemaValidationError::Attribution(
                        "final_attribution_stage is required".to_owned(),
                    )
                })?;
                if final_stage != requested {
                    return Err(SchemaValidationError::Attribution(format!(
                        "final attribution stage {} does not match requested {}",
                        final_stage.as_str(),
                        requested.as_str()
                    )));
                }
                if self.resource_summary_schema != Some(ProjectOwnedResourceSchemaVersion::V1) {
                    return Err(SchemaValidationError::Attribution(
                        "resource_summary_schema must be rterm.project-owned-resources/v1"
                            .to_owned(),
                    ));
                }
                let summary = self.resource_summary.as_ref().ok_or_else(|| {
                    SchemaValidationError::Attribution("resource_summary is required".to_owned())
                })?;
                summary.validate_at(final_stage).map_err(|violations| {
                    SchemaValidationError::Attribution(violations.join("; "))
                })?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaValidationError {
    ZeroConfiguration(&'static str),
    SampleCount { expected: u32, observed: u32 },
    MemoryUnit(String),
    Attribution(String),
}

impl Display for SchemaValidationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroConfiguration(field) => write!(formatter, "{field} must be positive"),
            Self::SampleCount { expected, observed } => {
                write!(
                    formatter,
                    "expected {expected} memory samples, observed {observed}"
                )
            }
            Self::MemoryUnit(unit) => {
                write!(formatter, "memory unit must be bytes, observed {unit}")
            }
            Self::Attribution(message) => formatter.write_str(message),
        }
    }
}

impl Error for SchemaValidationError {}
