use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ConnectionState, DiagnosticAttributionStage, DiagnosticGpuBackend,
    ProjectOwnedResourceMetricsV1, ProjectOwnedResourceSchemaVersion, RendererKind, Scenario,
    SchemaVersion, StartupMilestones,
};

pub const MARKER_PREFIX: &str = "rterm_diagnostic ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerIdentity {
    pub run_id: String,
    pub pid: u32,
    pub scenario: Scenario,
}

impl MarkerIdentity {
    #[must_use]
    pub fn new(run_id: impl Into<String>, pid: u32, scenario: Scenario) -> Self {
        Self {
            run_id: run_id.into(),
            pid,
            scenario,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkerKind {
    ProcessStarted,
    WindowCreated,
    FirstPresent,
    ConfigReady,
    TransportStarted,
    TransportReady,
    GpuReady,
    FontOwnershipReady,
    AttributionStageReady,
    ScenarioReady,
    SamplingStarted,
    SamplingFinished,
    ProcessExited,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarkerRecord {
    pub schema: SchemaVersion,
    pub run_id: String,
    pub pid: u32,
    pub scenario: Scenario,
    pub kind: MarkerKind,
    pub elapsed_ms: u64,
    #[serde(default)]
    pub renderer: Option<RendererKind>,
    #[serde(default)]
    pub connection_state: Option<ConnectionState>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MarkerDisposition {
    Ignored,
    Accepted(MarkerRecord),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectedMarkers {
    pub milestones: StartupMilestones,
    pub first_renderer: Option<RendererKind>,
    pub final_renderer: Option<RendererKind>,
    pub connection_state: Option<ConnectionState>,
    pub gpu_backend: Option<DiagnosticGpuBackend>,
    pub gpu_adapter_name: Option<String>,
    pub gpu_adapter_vendor_id: Option<u32>,
    pub gpu_adapter_device_id: Option<u32>,
    pub gpu_adapter_type: Option<String>,
    pub final_attribution_stage: Option<DiagnosticAttributionStage>,
    pub resource_summary_schema: Option<ProjectOwnedResourceSchemaVersion>,
    pub resource_summary: Option<ProjectOwnedResourceMetricsV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkerError {
    Malformed {
        message: String,
    },
    IdentityMismatch {
        field: &'static str,
        expected: String,
        observed: String,
    },
    DecreasingElapsed {
        previous_ms: u64,
        observed_ms: u64,
    },
    Duplicate(MarkerKind),
    AttributionProtocol(String),
}

impl Display for MarkerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed { message } => write!(formatter, "malformed marker: {message}"),
            Self::IdentityMismatch {
                field,
                expected,
                observed,
            } => write!(
                formatter,
                "marker {field} mismatch: expected {expected}, observed {observed}"
            ),
            Self::DecreasingElapsed {
                previous_ms,
                observed_ms,
            } => write!(
                formatter,
                "marker elapsed time decreased from {previous_ms} ms to {observed_ms} ms"
            ),
            Self::Duplicate(kind) => write!(formatter, "duplicate marker {kind:?}"),
            Self::AttributionProtocol(message) => {
                write!(
                    formatter,
                    "attribution marker protocol violation: {message}"
                )
            }
        }
    }
}

impl Error for MarkerError {}

#[derive(Debug)]
pub struct MarkerCollector {
    identity: MarkerIdentity,
    seen: HashSet<MarkerKind>,
    last_elapsed_ms: Option<u64>,
    trace: CollectedMarkers,
    requested_attribution_stage: Option<DiagnosticAttributionStage>,
}

impl MarkerCollector {
    #[must_use]
    pub fn new(identity: MarkerIdentity) -> Self {
        Self::new_with_attribution(identity, None)
    }

    #[must_use]
    pub fn new_attribution(
        identity: MarkerIdentity,
        requested_stage: DiagnosticAttributionStage,
    ) -> Self {
        Self::new_with_attribution(identity, Some(requested_stage))
    }

    fn new_with_attribution(
        identity: MarkerIdentity,
        requested_attribution_stage: Option<DiagnosticAttributionStage>,
    ) -> Self {
        Self {
            identity,
            seen: HashSet::new(),
            last_elapsed_ms: None,
            trace: CollectedMarkers {
                milestones: StartupMilestones::default(),
                first_renderer: None,
                final_renderer: None,
                connection_state: None,
                gpu_backend: None,
                gpu_adapter_name: None,
                gpu_adapter_vendor_id: None,
                gpu_adapter_device_id: None,
                gpu_adapter_type: None,
                final_attribution_stage: None,
                resource_summary_schema: None,
                resource_summary: None,
            },
            requested_attribution_stage,
        }
    }

    /// Parses and validates one application output line.
    ///
    /// # Errors
    ///
    /// Returns an error when a prefixed line is malformed, belongs to a different
    /// run/process/scenario, moves elapsed time backwards, or repeats a marker kind.
    pub fn push_line(&mut self, line: &str) -> Result<MarkerDisposition, MarkerError> {
        let Some(payload) = line.strip_prefix(MARKER_PREFIX) else {
            return Ok(MarkerDisposition::Ignored);
        };
        let record: MarkerRecord =
            serde_json::from_str(payload).map_err(|error| MarkerError::Malformed {
                message: error.to_string(),
            })?;
        self.validate_identity(&record)?;
        if let Some(previous_ms) = self.last_elapsed_ms
            && record.elapsed_ms < previous_ms
        {
            return Err(MarkerError::DecreasingElapsed {
                previous_ms,
                observed_ms: record.elapsed_ms,
            });
        }
        if self.seen.contains(&record.kind) {
            return Err(MarkerError::Duplicate(record.kind));
        }

        self.validate_attribution_protocol(&record)?;

        self.apply(&record);
        self.seen.insert(record.kind);
        self.last_elapsed_ms = Some(record.elapsed_ms);
        Ok(MarkerDisposition::Accepted(record))
    }

    #[must_use]
    pub const fn trace(&self) -> &CollectedMarkers {
        &self.trace
    }

    fn validate_identity(&self, record: &MarkerRecord) -> Result<(), MarkerError> {
        if record.run_id != self.identity.run_id {
            return Err(identity_mismatch(
                "run_id",
                self.identity.run_id.clone(),
                record.run_id.clone(),
            ));
        }
        if record.pid != self.identity.pid {
            return Err(identity_mismatch(
                "pid",
                self.identity.pid.to_string(),
                record.pid.to_string(),
            ));
        }
        if record.scenario != self.identity.scenario {
            return Err(identity_mismatch(
                "scenario",
                format!("{:?}", self.identity.scenario),
                format!("{:?}", record.scenario),
            ));
        }
        Ok(())
    }

    fn validate_attribution_protocol(&self, record: &MarkerRecord) -> Result<(), MarkerError> {
        let Some(requested_stage) = self.requested_attribution_stage else {
            if record.kind == MarkerKind::AttributionStageReady {
                return Err(MarkerError::AttributionProtocol(
                    "unexpected attribution_stage_ready without a requested stage".to_owned(),
                ));
            }
            return Ok(());
        };
        if self.seen.contains(&MarkerKind::ProcessExited) {
            return Err(MarkerError::AttributionProtocol(
                "marker observed after process_exited".to_owned(),
            ));
        }
        if self.seen.contains(&MarkerKind::AttributionStageReady)
            && record.kind != MarkerKind::ProcessExited
        {
            return Err(MarkerError::AttributionProtocol(format!(
                "later marker {:?} followed attribution_stage_ready",
                record.kind
            )));
        }
        match record.kind {
            MarkerKind::ProcessStarted if self.seen.is_empty() => Ok(()),
            MarkerKind::WindowCreated
                if self.seen.contains(&MarkerKind::ProcessStarted)
                    && !self.seen.contains(&MarkerKind::FirstPresent) =>
            {
                Ok(())
            }
            MarkerKind::FirstPresent
                if self.seen.contains(&MarkerKind::WindowCreated)
                    && !self.seen.contains(&MarkerKind::AttributionStageReady) =>
            {
                Ok(())
            }
            MarkerKind::AttributionStageReady if self.seen.contains(&MarkerKind::FirstPresent) => {
                let payload = attribution_ready_payload(record)?;
                if payload.requested_stage != requested_stage
                    || payload.final_stage != requested_stage
                {
                    return Err(MarkerError::AttributionProtocol(format!(
                        "requested/final stage must both equal {}",
                        requested_stage.as_str()
                    )));
                }
                if payload.resource_summary_schema != ProjectOwnedResourceSchemaVersion::V1 {
                    return Err(MarkerError::AttributionProtocol(
                        "resource summary schema is not v1".to_owned(),
                    ));
                }
                payload
                    .resource_summary
                    .validate_at(requested_stage)
                    .map_err(|violations| {
                        MarkerError::AttributionProtocol(violations.join("; "))
                    })?;
                payload.validate_adapter_identity(requested_stage)
            }
            MarkerKind::ProcessExited if self.seen.contains(&MarkerKind::AttributionStageReady) => {
                Ok(())
            }
            _ => Err(MarkerError::AttributionProtocol(format!(
                "out-of-order or forbidden marker {:?} for {}",
                record.kind,
                requested_stage.as_str()
            ))),
        }
    }

    fn apply(&mut self, record: &MarkerRecord) {
        match record.kind {
            MarkerKind::ProcessStarted => {
                self.trace.milestones.process_started_ms = record.elapsed_ms;
            }
            MarkerKind::WindowCreated => {
                self.trace.milestones.window_created_ms = Some(record.elapsed_ms);
            }
            MarkerKind::FirstPresent => {
                self.trace.milestones.first_present_ms = Some(record.elapsed_ms);
                self.trace.first_renderer = record.renderer;
                if record.renderer.is_some() {
                    self.trace.final_renderer = record.renderer;
                }
            }
            MarkerKind::ConfigReady => {
                self.trace.milestones.config_ready_ms = Some(record.elapsed_ms);
            }
            MarkerKind::TransportStarted => {
                self.trace.milestones.transport_started_ms = Some(record.elapsed_ms);
            }
            MarkerKind::TransportReady => {
                self.trace.milestones.transport_ready_ms = Some(record.elapsed_ms);
            }
            MarkerKind::GpuReady => {
                self.trace.milestones.gpu_ready_ms = Some(record.elapsed_ms);
                self.trace.final_renderer = record.renderer.or(Some(RendererKind::Gpu));
                self.trace.gpu_backend = record
                    .extra
                    .get("gpu_backend")
                    .and_then(Value::as_str)
                    .and_then(|backend| backend.to_ascii_lowercase().parse().ok());
                self.trace.gpu_adapter_name = record
                    .extra
                    .get("gpu_adapter_name")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                self.trace.gpu_adapter_vendor_id =
                    marker_u32(&record.extra, "gpu_adapter_vendor_id");
                self.trace.gpu_adapter_device_id =
                    marker_u32(&record.extra, "gpu_adapter_device_id");
                self.trace.gpu_adapter_type = record
                    .extra
                    .get("gpu_adapter_type")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
            }
            MarkerKind::FontOwnershipReady => {
                self.trace.milestones.font_ownership_ready_ms = Some(record.elapsed_ms);
            }
            MarkerKind::AttributionStageReady => {
                if let Ok(payload) = attribution_ready_payload(record) {
                    self.trace.final_renderer = record.renderer;
                    self.trace.gpu_backend = payload.resource_summary.backend;
                    self.trace
                        .gpu_adapter_name
                        .clone_from(&payload.resource_summary.adapter_name);
                    self.trace.gpu_adapter_vendor_id = payload.gpu_adapter_vendor_id;
                    self.trace.gpu_adapter_device_id = payload.gpu_adapter_device_id;
                    self.trace.gpu_adapter_type = payload.gpu_adapter_type;
                    self.trace.final_attribution_stage = Some(payload.final_stage);
                    self.trace.resource_summary_schema = Some(payload.resource_summary_schema);
                    self.trace.resource_summary = Some(payload.resource_summary);
                }
            }
            MarkerKind::ScenarioReady => {
                self.trace.milestones.scenario_ready_ms = Some(record.elapsed_ms);
            }
            MarkerKind::SamplingStarted => {
                self.trace.milestones.sampling_started_ms = Some(record.elapsed_ms);
            }
            MarkerKind::SamplingFinished => {
                self.trace.milestones.sampling_finished_ms = Some(record.elapsed_ms);
            }
            MarkerKind::ProcessExited => {
                self.trace.milestones.process_exited_ms = Some(record.elapsed_ms);
            }
        }
        if record.connection_state.is_some() {
            self.trace.connection_state = record.connection_state;
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AttributionStageReadyPayload {
    requested_stage: DiagnosticAttributionStage,
    final_stage: DiagnosticAttributionStage,
    resource_summary_schema: ProjectOwnedResourceSchemaVersion,
    resource_summary: ProjectOwnedResourceMetricsV1,
    #[serde(default)]
    gpu_adapter_vendor_id: Option<u32>,
    #[serde(default)]
    gpu_adapter_device_id: Option<u32>,
    #[serde(default)]
    gpu_adapter_type: Option<String>,
}

impl AttributionStageReadyPayload {
    fn validate_adapter_identity(
        &self,
        stage: DiagnosticAttributionStage,
    ) -> Result<(), MarkerError> {
        let identity_present = self.gpu_adapter_vendor_id.is_some()
            || self.gpu_adapter_device_id.is_some()
            || self.gpu_adapter_type.is_some();
        if stage < DiagnosticAttributionStage::AdapterDevice {
            return if identity_present {
                Err(MarkerError::AttributionProtocol(
                    "GPU adapter identity is forbidden before adapter-device".to_owned(),
                ))
            } else {
                Ok(())
            };
        }
        let adapter_type = self.gpu_adapter_type.as_deref().ok_or_else(|| {
            MarkerError::AttributionProtocol(
                "complete GPU adapter identity is required from adapter-device".to_owned(),
            )
        })?;
        if self.gpu_adapter_vendor_id.is_none()
            || self.gpu_adapter_device_id.is_none()
            || !matches!(
                adapter_type,
                "other" | "integrated-gpu" | "discrete-gpu" | "virtual-gpu" | "cpu"
            )
        {
            return Err(MarkerError::AttributionProtocol(
                "GPU adapter identity is incomplete or differs from resource_summary".to_owned(),
            ));
        }
        Ok(())
    }
}

fn attribution_ready_payload(
    record: &MarkerRecord,
) -> Result<AttributionStageReadyPayload, MarkerError> {
    let object = record.extra.clone().into_iter().collect();
    serde_json::from_value(Value::Object(object)).map_err(|error| {
        MarkerError::AttributionProtocol(format!(
            "invalid attribution_stage_ready payload: {error}"
        ))
    })
}

fn marker_u32(extra: &HashMap<String, Value>, key: &str) -> Option<u32> {
    extra
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

fn identity_mismatch(field: &'static str, expected: String, observed: String) -> MarkerError {
    MarkerError::IdentityMismatch {
        field,
        expected,
        observed,
    }
}

#[cfg(test)]
mod font_ownership_tests {
    use super::*;

    #[test]
    fn font_ownership_ready_is_a_singleton_marker_with_a_typed_milestone() {
        let mut collector =
            MarkerCollector::new(MarkerIdentity::new("font-ready", 42, Scenario::EmptyWindow));
        let line = format!(
            "{MARKER_PREFIX}{}",
            r#"{"schema":"rterm.diagnostics/v2","run_id":"font-ready","pid":42,"scenario":"empty_window","kind":"font_ownership_ready","elapsed_ms":125,"renderer":"gpu"}"#
        );

        let disposition = collector.push_line(&line).expect("font ownership marker");
        assert!(matches!(
            disposition,
            MarkerDisposition::Accepted(record)
                if record.kind == MarkerKind::FontOwnershipReady
        ));
        assert_eq!(
            collector.trace().milestones.font_ownership_ready_ms,
            Some(125)
        );
        assert_eq!(
            collector.push_line(&line).unwrap_err(),
            MarkerError::Duplicate(MarkerKind::FontOwnershipReady)
        );
    }
}
