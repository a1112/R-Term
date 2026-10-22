use std::{
    fmt,
    num::NonZeroU64,
    sync::{Arc, atomic::AtomicU64, atomic::Ordering},
    time::Duration,
};

use rterm_types::DamageRegion;
use rterm_types::PaneId;

use crate::RuntimeBatchMetrics;

/// Identifies the monotonic counter that has reached its terminal value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceKind {
    /// The process-wide pane generation counter.
    PaneGeneration,
    /// A pane's published runtime revision.
    RuntimeRevision,
    /// The lossless runtime effect stream.
    EffectSequence,
}

/// Reports that a monotonic sequence cannot advance without wrapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceExhausted {
    kind: SequenceKind,
}

impl SequenceExhausted {
    const fn new(kind: SequenceKind) -> Self {
        Self { kind }
    }

    /// Returns the counter that was exhausted.
    #[must_use]
    pub const fn kind(self) -> SequenceKind {
        self.kind
    }
}

impl fmt::Display for SequenceExhausted {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?} is exhausted", self.kind)
    }
}

impl std::error::Error for SequenceExhausted {}

/// A nonzero incarnation number that distinguishes stale events for a pane ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PaneGeneration(NonZeroU64);

impl PaneGeneration {
    /// The first valid pane generation.
    pub const FIRST: Self = Self(NonZeroU64::MIN);
    /// The final generation that can be issued without wrapping.
    pub const MAX: Self = Self(NonZeroU64::MAX);

    /// Returns the primitive generation value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

static NEXT_PANE_GENERATION: AtomicU64 = AtomicU64::new(PaneGeneration::FIRST.get());

fn issue_pane_generation(counter: &AtomicU64) -> Result<PaneGeneration, SequenceExhausted> {
    let value = counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            if current == 0 {
                None
            } else {
                Some(current.checked_add(1).unwrap_or(0))
            }
        })
        .map_err(|_| SequenceExhausted::new(SequenceKind::PaneGeneration))?;
    NonZeroU64::new(value)
        .map(PaneGeneration)
        .ok_or_else(|| SequenceExhausted::new(SequenceKind::PaneGeneration))
}

/// A pane identity paired with the generation that owns its events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaneToken {
    pane: PaneId,
    generation: PaneGeneration,
}

impl PaneToken {
    /// Returns the stable logical pane ID.
    #[must_use]
    pub const fn pane(self) -> PaneId {
        self.pane
    }

    /// Returns the nonzero incarnation number for this pane.
    #[must_use]
    pub const fn generation(self) -> PaneGeneration {
        self.generation
    }
}

/// Issues process-wide generations monotonically across every allocator handle.
///
/// Every handle refers to one private process counter. The allocator is
/// intentionally not cloneable and exposes no way to seed or rewind it.
#[derive(Debug)]
pub struct PaneTokenAllocator {
    counter: &'static AtomicU64,
}

impl PaneTokenAllocator {
    /// Creates a handle to the process-wide pane generation authority.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            counter: &NEXT_PANE_GENERATION,
        }
    }

    /// Issues a token and permanently reserves its generation.
    ///
    /// # Errors
    ///
    /// Returns [`SequenceExhausted`] after generation [`PaneGeneration::MAX`]
    /// has been issued. The allocator never wraps or reuses a generation.
    pub fn issue(&mut self, pane: PaneId) -> Result<PaneToken, SequenceExhausted> {
        let generation = issue_pane_generation(self.counter)?;
        Ok(PaneToken { pane, generation })
    }
}

impl Default for PaneTokenAllocator {
    fn default() -> Self {
        Self::new()
    }
}

/// A strictly increasing revision of one pane generation's terminal state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RuntimeRevision(NonZeroU64);

impl RuntimeRevision {
    /// The first published runtime revision.
    pub const FIRST: Self = Self(NonZeroU64::MIN);
    /// The final revision that can be published without wrapping.
    pub const MAX: Self = Self(NonZeroU64::MAX);

    /// Returns the primitive revision value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    /// Advances to the next revision.
    ///
    /// # Errors
    ///
    /// Returns [`SequenceExhausted`] at [`RuntimeRevision::MAX`] instead of
    /// wrapping to an older revision.
    pub fn next(self) -> Result<Self, SequenceExhausted> {
        self.get()
            .checked_add(1)
            .and_then(NonZeroU64::new)
            .map(Self)
            .ok_or_else(|| SequenceExhausted::new(SequenceKind::RuntimeRevision))
    }
}

/// A position in the lossless, globally ordered runtime effect stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EffectSequence(NonZeroU64);

impl EffectSequence {
    /// The first emitted effect sequence.
    pub const FIRST: Self = Self(NonZeroU64::MIN);
    /// The final effect sequence that can be emitted without wrapping.
    pub const MAX: Self = Self(NonZeroU64::MAX);

    /// Returns the primitive sequence value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    /// Advances to the next effect position.
    ///
    /// Comparing this result with the next received effect detects gaps and
    /// duplicates, including at runtime-batch boundaries.
    ///
    /// # Errors
    ///
    /// Returns [`SequenceExhausted`] at [`EffectSequence::MAX`] instead of
    /// wrapping and making old and new effects indistinguishable.
    pub fn next(self) -> Result<Self, SequenceExhausted> {
        self.get()
            .checked_add(1)
            .and_then(NonZeroU64::new)
            .map(Self)
            .ok_or_else(|| SequenceExhausted::new(SequenceKind::EffectSequence))
    }
}

/// A typed discontinuity in the ordered runtime effect stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectSequenceError {
    /// One or more effect positions were skipped.
    Gap {
        /// Next sequence required by the cursor.
        expected: EffectSequence,
        /// Later sequence observed in the batch.
        observed: EffectSequence,
    },
    /// An already consumed or older effect position was observed.
    DuplicateOrOutOfOrder {
        /// Next sequence required by the cursor.
        expected: EffectSequence,
        /// Duplicate or older sequence observed in the batch.
        observed: EffectSequence,
    },
    /// The maximum effect position was already consumed.
    Exhausted {
        /// Sequence observed after exhaustion.
        observed: EffectSequence,
    },
}

impl fmt::Display for EffectSequenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gap { expected, observed } => write!(
                formatter,
                "effect sequence gap: expected {}, observed {}",
                expected.get(),
                observed.get()
            ),
            Self::DuplicateOrOutOfOrder { expected, observed } => write!(
                formatter,
                "duplicate or out-of-order effect: expected {}, observed {}",
                expected.get(),
                observed.get()
            ),
            Self::Exhausted { observed } => write!(
                formatter,
                "effect sequence exhausted before observed {}",
                observed.get()
            ),
        }
    }
}

impl std::error::Error for EffectSequenceError {}

/// Retains the next required effect sequence across runtime batches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectSequenceCursor {
    expected: Option<EffectSequence>,
}

impl EffectSequenceCursor {
    /// Creates a cursor that requires `expected` as its first effect.
    #[must_use]
    pub const fn new(expected: EffectSequence) -> Self {
        Self {
            expected: Some(expected),
        }
    }

    /// Returns the next required sequence, or `None` after accepting the maximum.
    #[must_use]
    pub const fn expected(&self) -> Option<EffectSequence> {
        self.expected
    }

    /// Validates effects in order and retains the next requirement for later batches.
    ///
    /// # Errors
    ///
    /// Returns [`EffectSequenceError::Gap`] when an effect is missing,
    /// [`EffectSequenceError::DuplicateOrOutOfOrder`] for an already consumed
    /// position, or [`EffectSequenceError::Exhausted`] for an effect after
    /// [`EffectSequence::MAX`]. If any effect is rejected, the cursor remains
    /// exactly as it was before the batch so the corrected whole batch can be
    /// retried.
    pub fn validate_batch<S>(
        &mut self,
        batch: &RuntimeBatch<S>,
    ) -> Result<(), EffectSequenceError> {
        let mut candidate = self.expected;
        for effect in &batch.effects {
            let observed = effect.sequence();
            let Some(expected) = candidate else {
                return Err(EffectSequenceError::Exhausted { observed });
            };
            if observed > expected {
                return Err(EffectSequenceError::Gap { expected, observed });
            }
            if observed < expected {
                return Err(EffectSequenceError::DuplicateOrOutOfOrder { expected, observed });
            }
            candidate = expected.next().ok();
        }
        self.expected = candidate;
        Ok(())
    }
}

impl Default for EffectSequenceCursor {
    fn default() -> Self {
        Self::new(EffectSequence::FIRST)
    }
}

/// Reports whether a command entered the runtime's bounded work queue.
#[must_use = "submission results must be handled so input is never silently dropped"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmitResult {
    /// The runtime accepted ownership of the command.
    Accepted,
    /// Capacity is currently unavailable; the caller should retry no sooner
    /// than the supplied duration.
    Backpressured {
        /// Minimum delay suggested by the runtime before another attempt.
        retry_after: Duration,
    },
    /// The pane no longer accepts commands.
    Closed,
}

/// A changed metadata value; absence of this enum means no change occurred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataChange<T> {
    /// Replace the previous value.
    Set(T),
    /// Remove the previous value.
    Clear,
}

/// Transport-neutral progress state reported by terminal integrations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RuntimeProgress {
    /// No progress is currently reported.
    #[default]
    None,
    /// Work completion percentage.
    Percentage(u8),
    /// Failed work completion percentage.
    Error(u8),
    /// Work is active but has no numeric completion value.
    Indeterminate,
}

/// One changed pane user variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserVarDelta {
    /// User-variable name.
    pub name: String,
    /// New value or explicit removal.
    pub value: MetadataChange<String>,
}

/// Metadata changes produced while parsing one runtime batch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaneMetadataDelta {
    /// Changed pane title, if any.
    pub title: Option<MetadataChange<String>>,
    /// Changed current working directory, if any.
    pub working_directory: Option<MetadataChange<String>>,
    /// Changed badge-format expression, if any.
    pub badge_format: Option<MetadataChange<String>>,
    /// Changed progress state, if any.
    pub progress: Option<MetadataChange<RuntimeProgress>>,
    /// Changed user variables in terminal-observation order.
    pub user_vars: Vec<UserVarDelta>,
}

/// A platform-neutral side effect produced by terminal progression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeEffectKind {
    /// Bytes that must be written back to the session transport.
    TransportWrite(Vec<u8>),
    /// Ordered terminal display bytes retained only when a host-stream
    /// diagnostic consumer was explicitly enabled for the pane.
    HostStream(Vec<u8>),
    /// Terminal-visible bytes suitable for an explicitly enabled session log.
    ///
    /// Unlike [`Self::HostStream`], this payload excludes terminal controls and
    /// is emitted only when the pane worker's session-log capture is enabled.
    VisibleOutput(Vec<u8>),
    /// Input-encoding or host-interaction mode transition.
    ModeChange(crate::TerminalModeChange),
    /// An audible or visual bell request.
    Bell {
        /// Number of bell occurrences coalesced into this ordered effect.
        count: NonZeroU64,
    },
    /// Replace the host clipboard contents.
    ClipboardWrite {
        /// OSC 52 selection token, or `None` for protocols without one.
        selection: Option<String>,
        /// Text requested by the terminal.
        contents: String,
    },
    /// Request the current contents of a host clipboard selection.
    ClipboardRead {
        /// OSC 52 selection token that must be preserved in the response.
        selection: String,
    },
    /// Dispatch a desktop-independent notification request.
    Notification {
        /// Optional notification heading.
        title: Option<String>,
        /// Notification body.
        body: String,
    },
    /// Surface a runtime diagnostic to the host.
    Diagnostic {
        /// Human-readable diagnostic detail.
        message: String,
    },
}

/// An ordered, lossless terminal side effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEffect {
    sequence: EffectSequence,
    kind: RuntimeEffectKind,
}

impl RuntimeEffect {
    /// Creates an effect at its assigned position within one [`PaneToken`] stream.
    #[must_use]
    pub const fn new(sequence: EffectSequence, kind: RuntimeEffectKind) -> Self {
        Self { sequence, kind }
    }

    /// Returns the position within the effect's [`PaneToken`] stream.
    #[must_use]
    pub const fn sequence(&self) -> EffectSequence {
        self.sequence
    }

    /// Returns the platform-neutral effect payload.
    #[must_use]
    pub const fn kind(&self) -> &RuntimeEffectKind {
        &self.kind
    }
}

/// One atomic publication from a pane worker to its host.
///
/// `S` is a renderer-owned immutable snapshot type. Keeping it generic lets
/// the runtime preserve snapshot ownership without depending on a renderer or
/// native window implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBatch<S> {
    /// Pane incarnation that produced this batch.
    pub pane: PaneToken,
    /// Strictly increasing terminal-state revision.
    pub revision: RuntimeRevision,
    /// New immutable presentation snapshot, when publication was requested.
    pub snapshot: Option<Arc<S>>,
    /// Normalized changed regions in terminal coordinates.
    pub damage: Vec<DamageRegion>,
    /// Metadata changed by this batch.
    pub metadata: PaneMetadataDelta,
    /// Lossless effects in their exact production order.
    pub effects: Vec<RuntimeEffect>,
    /// Timing and volume measurements for this batch.
    pub metrics: RuntimeBatchMetrics,
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{PaneGeneration, SequenceKind, issue_pane_generation};

    #[test]
    fn private_pane_generation_counter_issues_max_once_then_exhausts() {
        let counter = AtomicU64::new(u64::MAX);

        assert_eq!(
            issue_pane_generation(&counter).expect("MAX remains issuable"),
            PaneGeneration::MAX
        );
        assert_eq!(counter.load(Ordering::Relaxed), 0);
        assert_eq!(
            issue_pane_generation(&counter)
                .expect_err("zero sentinel must remain exhausted")
                .kind(),
            SequenceKind::PaneGeneration
        );
        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }
}
