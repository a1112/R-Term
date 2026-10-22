mod catalog;
mod checkpoint;
mod coverage;
mod evidence;
mod hermetic_network;
mod observer;
mod platform_input;
mod pty_driver;
mod runner;
mod scenario;
mod shard;
mod suite;
mod transport_driver;

pub use catalog::{BehaviorCatalogV1, validate_catalog};
pub use checkpoint::{CheckpointContext, CheckpointError, evaluate_checkpoint};
pub use coverage::{
    BehaviorCoverageReportV1, BehaviorEvidenceMapV1, CoverageInputs, ExecutableEvidenceSource,
    verify_behavior_coverage,
};
pub use evidence::{EvidenceEventV1, EvidenceWriter, ScenarioOutcome, ScenarioRunId};
pub use observer::{
    HostEffectObservationV1, ObserverClient, ObserverEndpoint, ObserverRequestV1,
    ObserverResponseV1, ObserverServer, ObserverSnapshotV1, ObserverState, ObserverStateError,
    ObserverToken, PaneObservationV1, RuntimeObservationV1, TerminalObservationV1,
    WindowObservationV1,
};
pub use platform_input::{DriverOperation, InputBackend, PlatformInputDriver, PlatformInputError};
pub use pty_driver::{PtyFixtureDriver, PtyFixtureError, PtyFixtureResult, TRANSPORT_WRITE_MARKER};
pub use runner::{EXIT_INFRASTRUCTURE_FAILED, run_cli};
pub use scenario::{
    ActionV1, BehaviorId, Capability, CheckpointV1, DeadlinesV1, EvidenceKind, KeyModifier,
    MouseButton, ScenarioV1, ScenarioValidationError, Surface, WindowControl,
};
pub use shard::{ShardAssignment, ShardAssignmentError, assign_lpt_shards};
pub use suite::{FunctionalSuite, SuiteLoadError};
pub use transport_driver::{
    SshJourneyResult, TransferJourneyResult, TransportJourneyError, run_ssh_loopback_journey,
    run_transfer_roundtrip_journey,
};
