//! Transport-neutral terminal runtime contracts.

mod api;
mod batch;
mod clock;
pub mod delta;
mod hub;
mod latest;
mod mailbox;
mod metrics;
pub mod modes;
mod pane;
pub mod queries;
pub mod query_dcs;
mod shutdown;
pub mod terminal;

#[cfg(any(test, feature = "test-support"))]
pub mod testing;

#[cfg(test)]
#[path = "fixture_trace.rs"]
pub(crate) mod fixture_trace;
#[cfg(test)]
#[path = "frozen_trace_pack.rs"]
pub(crate) mod frozen_trace_pack;
#[cfg(test)]
#[path = "test_body_digest.rs"]
pub(crate) mod test_body_digest;
mod transport;
pub mod visible_output;

pub use api::{
    EffectSequence, EffectSequenceCursor, EffectSequenceError, MetadataChange, PaneGeneration,
    PaneMetadataDelta, PaneToken, PaneTokenAllocator, RuntimeBatch, RuntimeEffect,
    RuntimeEffectKind, RuntimeProgress, RuntimeRevision, SequenceExhausted, SequenceKind,
    SubmitResult, UserVarDelta,
};
pub use batch::{
    BatchAdmission, BatchPolicy, BatchPolicyError, BatchWindow, PaneDrain, PanePublicationMetrics,
    PresentationFrame, PublishedEffect, TerminalStateSummary,
};
pub use clock::{Clock, SystemClock};
pub use delta::{
    MetadataChangeRef, RuntimeBufferCapacities, RuntimeBuffers, RuntimeDelta, RuntimeEffectRef,
    RuntimeMetadataDeltaRef, TerminalSnapshotRef,
};
pub use hub::{OpenPaneError, RuntimeHub};
pub use latest::{CoalesceLatest, DrainCompletion, LatestSlot, LatestSlotMetrics, PublishAction};
pub use mailbox::{
    MailboxItem, MailboxLimits, MailboxLimitsError, MailboxMetrics, MailboxReceiver, MailboxSender,
    RecvError, RecvTimeoutError, SendError, TryRecvError, TrySendError, bounded_mailbox,
};
pub use metrics::RuntimeBatchMetrics;
pub use modes::{MouseInputMode, MouseProtocolMode, MouseReportingMode, TerminalModeChange};
pub use pane::{PaneHandle, PaneNotice, PaneWorkerConfig};
pub use terminal::TerminalRuntime;
pub use transport::{
    SessionControl, SessionExit, SessionExitSignal, SessionInterrupt, SessionParts,
    SessionTransport,
};
