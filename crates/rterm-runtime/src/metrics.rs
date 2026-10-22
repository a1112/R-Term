use std::time::Duration;

/// Per-batch measurements emitted by a pane worker.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeBatchMetrics {
    /// Number of transport bytes consumed by the batch.
    pub transport_bytes: usize,
    /// Number of transport reads coalesced before publication.
    pub coalesced_reads: u32,
    /// Time spent advancing terminal state.
    pub parse_duration: Duration,
    /// Time spent creating the optional immutable snapshot.
    pub snapshot_duration: Duration,
}
