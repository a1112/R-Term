use std::sync::{Mutex, MutexGuard, PoisonError};

/// Values that preserve required state when an unconsumed frame is replaced.
pub trait CoalesceLatest: Send + 'static {
    /// Merges state required from the replaced older value into `self`.
    fn coalesce_replaced(&mut self, replaced: Self);
}

/// Whether publishing requires a host wake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishAction {
    /// The publisher transitioned from idle to ready and must wake the host.
    Wake,
    /// A wake is already pending; the latest value was coalesced in place.
    Coalesced,
}

/// Host action after completing one wake turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrainCompletion {
    /// No frame or lossless work remains; the wake token returned to idle.
    Idle,
    /// Work remains and exactly one continuation wake must be scheduled.
    Continue,
}

/// Monotonic counters for one replaceable latest-value slot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LatestSlotMetrics {
    /// Total published values.
    pub publications: u64,
    /// Values overwritten before the host consumed them.
    pub replaced_frames: u64,
    /// Idle-to-ready wake transitions.
    pub wakes: u64,
    /// Distinct continuation wake requests.
    pub continuations: u64,
}

struct State<T> {
    latest: Option<T>,
    wake_pending: bool,
    continuation_scheduled: bool,
    metrics: LatestSlotMetrics,
}

/// Mutex-backed one-slot latest publication with lost-wake protection.
pub struct LatestSlot<T> {
    state: Mutex<State<T>>,
}

impl<T> LatestSlot<T> {
    /// Creates an empty idle slot.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: Mutex::new(State {
                latest: None,
                wake_pending: false,
                continuation_scheduled: false,
                metrics: LatestSlotMetrics {
                    publications: 0,
                    replaced_frames: 0,
                    wakes: 0,
                    continuations: 0,
                },
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, State<T>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Removes the current latest value while retaining the active wake token.
    pub fn take(&self) -> Option<T> {
        self.lock().latest.take()
    }

    /// Reports whether a latest value remains unconsumed.
    #[must_use]
    pub fn has_value(&self) -> bool {
        self.lock().latest.is_some()
    }

    /// Marks non-replaceable work ready and reports whether the host must wake.
    pub fn signal_work(&self) -> PublishAction {
        let mut state = self.lock();
        signal_work(&mut state)
    }

    /// Completes one host turn and resolves the continuation/idle transition.
    pub fn complete_wake(&self, lossless_work_remains: bool) -> DrainCompletion {
        let mut state = self.lock();
        if state.latest.is_some() || lossless_work_remains {
            state.wake_pending = true;
            if !state.continuation_scheduled {
                state.continuation_scheduled = true;
                state.metrics.continuations = state.metrics.continuations.saturating_add(1);
            }
            DrainCompletion::Continue
        } else {
            state.wake_pending = false;
            state.continuation_scheduled = false;
            DrainCompletion::Idle
        }
    }

    /// Returns a consistent metrics snapshot.
    #[must_use]
    pub fn metrics(&self) -> LatestSlotMetrics {
        self.lock().metrics
    }
}

impl<T: CoalesceLatest> LatestSlot<T> {
    /// Publishes a value and reports whether the host needs a new wake.
    pub fn publish(&self, mut value: T) -> PublishAction {
        let mut state = self.lock();
        state.metrics.publications = state.metrics.publications.saturating_add(1);
        if let Some(replaced) = state.latest.take() {
            value.coalesce_replaced(replaced);
            state.metrics.replaced_frames = state.metrics.replaced_frames.saturating_add(1);
        }
        state.latest = Some(value);
        signal_work(&mut state)
    }
}

fn signal_work<T>(state: &mut State<T>) -> PublishAction {
    if state.wake_pending {
        PublishAction::Coalesced
    } else {
        state.wake_pending = true;
        state.continuation_scheduled = false;
        state.metrics.wakes = state.metrics.wakes.saturating_add(1);
        PublishAction::Wake
    }
}

impl<T> Default for LatestSlot<T> {
    fn default() -> Self {
        Self::new()
    }
}
