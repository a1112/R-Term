use std::error::Error;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use rterm_terminal::Terminal;
use rterm_types::{DamageRegion, TerminalSize};

use crate::{
    DrainCompletion, LatestSlot, MailboxItem, MailboxLimits, MailboxMetrics, MailboxReceiver,
    MailboxSender, PaneMetadataDelta, PaneToken, PublishAction, RuntimeBatchMetrics, RuntimeEffect,
    RuntimeEffectKind, RuntimeRevision, TryRecvError, TrySendError, bounded_mailbox,
    latest::LatestSlotMetrics,
};

/// Compact renderer-neutral identity of the terminal state behind a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalStateSummary {
    /// Current terminal grid size.
    pub size: TerminalSize,
    /// Cursor row and column.
    pub cursor: (u16, u16),
    /// Retained scrollback row count.
    pub scrollback_rows: usize,
    /// Terminal mutation sequence.
    pub sequence: u64,
    /// Stable digest of visible cell text and row boundaries.
    pub visible_digest: u64,
}

impl TerminalStateSummary {
    /// Captures a compact summary without cloning terminal cells.
    #[must_use]
    pub fn capture(runtime: &crate::TerminalRuntime) -> Self {
        Self::capture_terminal(runtime.terminal())
    }

    /// Captures a compact summary of an immutable terminal snapshot.
    #[must_use]
    pub fn capture_terminal(terminal: &Terminal) -> Self {
        let grid = terminal.grid();
        let size = grid.size();
        let mut digest = 0xcbf2_9ce4_8422_2325u64;
        for row in 0..size.rows {
            if let Some(row) = grid.row(row) {
                for cell in row.cells() {
                    for byte in cell.text().as_bytes() {
                        digest ^= u64::from(*byte);
                        digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
                    }
                    digest ^= u64::from(cell.columns());
                    digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
            digest ^= 0xff;
            digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Self {
            size,
            cursor: terminal.cursor(),
            scrollback_rows: terminal.scrollback().len(),
            sequence: u64::try_from(terminal.current_seqno()).unwrap_or(u64::MAX),
            visible_digest: digest,
        }
    }
}

/// Byte, item, and latency boundaries for one pane parse batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchPolicy {
    bytes: usize,
    items: usize,
    latency: Duration,
}

impl BatchPolicy {
    /// Creates a policy with nonzero byte, item, and latency limits.
    ///
    /// # Errors
    ///
    /// Returns the first zero limit in byte, item, latency order.
    pub const fn try_new(
        max_bytes: usize,
        max_items: usize,
        max_latency: Duration,
    ) -> Result<Self, BatchPolicyError> {
        if max_bytes == 0 {
            return Err(BatchPolicyError::ZeroBytes);
        }
        if max_items == 0 {
            return Err(BatchPolicyError::ZeroItems);
        }
        if max_latency.is_zero() {
            return Err(BatchPolicyError::ZeroLatency);
        }
        Ok(Self {
            bytes: max_bytes,
            items: max_items,
            latency: max_latency,
        })
    }

    /// Maximum bytes admitted to a normal batch.
    #[must_use]
    pub const fn max_bytes(self) -> usize {
        self.bytes
    }

    /// Maximum source items admitted to one batch.
    #[must_use]
    pub const fn max_items(self) -> usize {
        self.items
    }

    /// Maximum elapsed monotonic time from the first admitted item.
    #[must_use]
    pub const fn max_latency(self) -> Duration {
        self.latency
    }
}

impl Default for BatchPolicy {
    fn default() -> Self {
        Self::try_new(128 * 1024, 16, Duration::from_millis(3))
            .expect("default batch policy limits are nonzero")
    }
}

/// Invalid batch policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchPolicyError {
    /// Byte limit was zero.
    ZeroBytes,
    /// Item limit was zero.
    ZeroItems,
    /// Latency limit was zero.
    ZeroLatency,
}

impl fmt::Display for BatchPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroBytes => formatter.write_str("batch byte limit must be nonzero"),
            Self::ZeroItems => formatter.write_str("batch item limit must be nonzero"),
            Self::ZeroLatency => formatter.write_str("batch latency limit must be nonzero"),
        }
    }
}

impl Error for BatchPolicyError {}

/// Result of attempting to append one source item to a batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchAdmission {
    /// The item was admitted and more work may fit.
    Accepted,
    /// The item was admitted and reached a byte or item boundary exactly.
    AcceptedAndFull,
    /// The item belongs to the next batch.
    Rejected,
}

/// Mutable accounting window for one parse batch.
#[derive(Debug, Clone, Copy)]
pub struct BatchWindow {
    policy: BatchPolicy,
    started_at: Option<Instant>,
    bytes: usize,
    items: usize,
}

impl BatchWindow {
    /// Creates an empty accounting window.
    #[must_use]
    pub const fn new(policy: BatchPolicy) -> Self {
        Self {
            policy,
            started_at: None,
            bytes: 0,
            items: 0,
        }
    }

    /// Attempts to admit one item at `now` without mutating on rejection.
    pub fn try_push(&mut self, now: Instant, item_bytes: usize) -> BatchAdmission {
        if let Some(started_at) = self.started_at {
            if now.saturating_duration_since(started_at) >= self.policy.latency {
                return BatchAdmission::Rejected;
            }
            let Some(next_bytes) = self.bytes.checked_add(item_bytes) else {
                return BatchAdmission::Rejected;
            };
            if next_bytes > self.policy.bytes || self.items >= self.policy.items {
                return BatchAdmission::Rejected;
            }
            self.bytes = next_bytes;
            self.items += 1;
        } else {
            self.started_at = Some(now);
            self.bytes = item_bytes;
            self.items = 1;
        }
        if self.bytes >= self.policy.bytes || self.items >= self.policy.items {
            BatchAdmission::AcceptedAndFull
        } else {
            BatchAdmission::Accepted
        }
    }

    /// Clears accounting for the next batch.
    pub fn reset(&mut self) {
        self.started_at = None;
        self.bytes = 0;
        self.items = 0;
    }

    /// Bytes admitted to the current batch.
    #[must_use]
    pub const fn bytes(&self) -> usize {
        self.bytes
    }

    /// Source items admitted to the current batch.
    #[must_use]
    pub const fn items(&self) -> usize {
        self.items
    }
}

/// Replaceable presentation state produced by one pane parse batch.
#[derive(Debug, Clone)]
pub struct PresentationFrame {
    /// Pane generation that produced this frame.
    pub pane: PaneToken,
    /// Strictly increasing pane revision.
    pub revision: RuntimeRevision,
    /// Immutable renderer-neutral terminal snapshot for this revision.
    pub snapshot: Arc<Terminal>,
    /// Terminal state represented by this frame.
    pub state: TerminalStateSummary,
    /// Whether terminal state changed.
    pub snapshot_changed: bool,
    /// Replacement requires repainting from the latest full snapshot.
    pub full_repaint: bool,
    /// Partial damage when no older frame was replaced.
    pub damage: Vec<DamageRegion>,
    /// Final-only metadata changes not yet consumed by the host.
    pub metadata: PaneMetadataDelta,
    /// Measurements for the batch that produced the latest frame.
    pub metrics: RuntimeBatchMetrics,
}

impl crate::CoalesceLatest for PresentationFrame {
    fn coalesce_replaced(&mut self, replaced: Self) {
        self.snapshot_changed |= replaced.snapshot_changed;
        self.full_repaint = true;
        self.damage.clear();
        merge_metadata(&mut self.metadata, replaced.metadata);
    }
}

fn merge_metadata(current: &mut PaneMetadataDelta, replaced: PaneMetadataDelta) {
    if current.title.is_none() {
        current.title = replaced.title;
    }
    if current.working_directory.is_none() {
        current.working_directory = replaced.working_directory;
    }
    if current.badge_format.is_none() {
        current.badge_format = replaced.badge_format;
    }
    if current.progress.is_none() {
        current.progress = replaced.progress;
    }
    let current_names = current
        .user_vars
        .iter()
        .map(|change| change.name.as_str())
        .collect::<Vec<_>>();
    let mut user_vars = replaced
        .user_vars
        .into_iter()
        .filter(|change| !current_names.contains(&change.name.as_str()))
        .collect::<Vec<_>>();
    user_vars.append(&mut current.user_vars);
    current.user_vars = user_vars;
}

/// One lossless host effect tied to its pane revision and per-pane sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedEffect {
    /// Pane generation that produced the effect.
    pub pane: PaneToken,
    /// Pane revision that contained the effect.
    pub revision: RuntimeRevision,
    /// Strictly ordered effect payload.
    pub effect: RuntimeEffect,
}

impl MailboxItem for PublishedEffect {
    fn retained_bytes(&self) -> usize {
        match self.effect.kind() {
            RuntimeEffectKind::TransportWrite(bytes)
            | RuntimeEffectKind::HostStream(bytes)
            | RuntimeEffectKind::VisibleOutput(bytes) => bytes.capacity(),
            RuntimeEffectKind::ModeChange(_) | RuntimeEffectKind::Bell { .. } => 0,
            RuntimeEffectKind::ClipboardWrite {
                selection,
                contents,
            } => selection.as_ref().map_or(0, String::capacity) + contents.capacity(),
            RuntimeEffectKind::ClipboardRead { selection } => selection.capacity(),
            RuntimeEffectKind::Notification { title, body } => {
                title.as_ref().map_or(0, String::capacity) + body.capacity()
            }
            RuntimeEffectKind::Diagnostic { message } => message.capacity(),
        }
    }
}

/// Work returned to a host for one pane wake.
#[derive(Debug, Clone)]
pub struct PaneDrain {
    /// Latest replaceable presentation frame, if one was pending.
    pub frame: Option<PresentationFrame>,
    /// Lossless effects in source order.
    pub effects: Vec<PublishedEffect>,
    /// Whether the host must schedule exactly one continuation turn.
    pub continuation: bool,
}

/// Cumulative publication and bounded-effect metrics for one pane generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanePublicationMetrics {
    /// Number of parse/resize/finalization batches published.
    pub batches: u64,
    /// Total source bytes parsed.
    pub source_bytes: u64,
    /// Total reader items coalesced.
    pub source_items: u64,
    /// Largest batch in bytes.
    pub max_batch_bytes: usize,
    /// Largest batch in reader items.
    pub max_batch_items: usize,
    /// Total parse duration.
    pub parse_duration: Duration,
    /// Latest-slot wake and replacement counters.
    pub latest: LatestSlotMetrics,
    /// Lossless effect mailbox accounting.
    pub effects: MailboxMetrics,
}

#[derive(Debug, Clone, Copy, Default)]
struct PublicationCounters {
    batches: u64,
    source_bytes: u64,
    source_items: u64,
    max_batch_bytes: usize,
    max_batch_items: usize,
    parse_duration: Duration,
}

pub(crate) struct PanePublication {
    latest: LatestSlot<PresentationFrame>,
    effect_sender: MailboxSender<PublishedEffect>,
    effect_receiver: Mutex<MailboxReceiver<PublishedEffect>>,
    counters: Mutex<PublicationCounters>,
}

impl PanePublication {
    pub(crate) fn new(effect_limits: MailboxLimits) -> Self {
        let (effect_sender, effect_receiver) = bounded_mailbox(effect_limits);
        Self {
            latest: LatestSlot::new(),
            effect_sender,
            effect_receiver: Mutex::new(effect_receiver),
            counters: Mutex::new(PublicationCounters::default()),
        }
    }

    pub(crate) fn publish(
        &self,
        frame: PresentationFrame,
        effects: impl IntoIterator<Item = PublishedEffect>,
        mut wake: impl FnMut() -> Result<(), ()>,
    ) -> Result<(), ()> {
        let mut frame = Some(frame);
        for effect in effects {
            match self.effect_sender.try_send(effect) {
                Ok(()) => {
                    if frame.is_none() {
                        self.wake_if_needed(self.latest.signal_work(), &mut wake)?;
                    }
                }
                Err(TrySendError::Full { item, .. }) => {
                    if let Some(frame) = frame.take() {
                        self.publish_frame(frame, &mut wake)?;
                    }
                    self.effect_sender.send(item).map_err(|_| ())?;
                    self.wake_if_needed(self.latest.signal_work(), &mut wake)?;
                }
                Err(TrySendError::Closed(_) | TrySendError::Oversize { .. }) => return Err(()),
            }
        }
        if let Some(frame) = frame {
            self.publish_frame(frame, &mut wake)?;
        }
        Ok(())
    }

    fn publish_frame(
        &self,
        frame: PresentationFrame,
        wake: &mut impl FnMut() -> Result<(), ()>,
    ) -> Result<(), ()> {
        let metrics = frame.metrics;
        {
            let mut counters = self.counters.lock().unwrap_or_else(PoisonError::into_inner);
            counters.batches = counters.batches.saturating_add(1);
            counters.source_bytes = counters
                .source_bytes
                .saturating_add(metrics.transport_bytes as u64);
            counters.source_items = counters
                .source_items
                .saturating_add(u64::from(metrics.coalesced_reads));
            counters.max_batch_bytes = counters.max_batch_bytes.max(metrics.transport_bytes);
            counters.max_batch_items = counters
                .max_batch_items
                .max(metrics.coalesced_reads as usize);
            counters.parse_duration = counters
                .parse_duration
                .saturating_add(metrics.parse_duration);
        }
        let action = self.latest.publish(frame);
        self.wake_if_needed(action, wake)
    }

    pub(crate) fn drain(&self, max_effects: usize) -> PaneDrain {
        let mut effects = Vec::new();
        let mut receiver = self
            .effect_receiver
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        while effects.len() < max_effects {
            match receiver.try_recv() {
                Ok(effect) => effects.push(effect),
                Err(TryRecvError::Empty | TryRecvError::Closed) => break,
            }
        }
        let work_remains = receiver.metrics().queued_items > 0;
        drop(receiver);
        // A latest frame may carry a newer revision than lossless effects
        // still waiting in the bounded mailbox. Keep it in the replaceable
        // slot until those effects are drained so hosts never observe the
        // future frame first and reject the older effects as stale.
        let frame = (!work_remains).then(|| self.latest.take()).flatten();
        let continuation = self.latest.complete_wake(work_remains) == DrainCompletion::Continue;
        PaneDrain {
            frame,
            effects,
            continuation,
        }
    }

    pub(crate) fn metrics(&self) -> PanePublicationMetrics {
        let counters = *self.counters.lock().unwrap_or_else(PoisonError::into_inner);
        PanePublicationMetrics {
            batches: counters.batches,
            source_bytes: counters.source_bytes,
            source_items: counters.source_items,
            max_batch_bytes: counters.max_batch_bytes,
            max_batch_items: counters.max_batch_items,
            parse_duration: counters.parse_duration,
            latest: self.latest.metrics(),
            effects: self.effect_sender.metrics(),
        }
    }

    pub(crate) fn has_work(&self) -> bool {
        self.latest.has_value() || self.effect_sender.metrics().queued_items > 0
    }

    fn wake_if_needed(
        &self,
        action: PublishAction,
        wake: &mut impl FnMut() -> Result<(), ()>,
    ) -> Result<(), ()> {
        if action == PublishAction::Wake && wake().is_err() {
            let _ = self.effect_sender.close();
            return Err(());
        }
        Ok(())
    }
}
