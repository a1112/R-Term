use std::collections::HashMap;
use std::fmt;
use std::io;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rterm_types::PaneId;

use crate::{
    Clock, MailboxLimits, MailboxReceiver, MailboxSender, PaneDrain, PaneNotice,
    PanePublicationMetrics, PaneToken, PaneTokenAllocator, PaneWorkerConfig, RecvError,
    SequenceExhausted, SessionTransport, SystemClock, TerminalRuntime, TryRecvError,
    batch::PanePublication,
    bounded_mailbox,
    pane::{ErasedInterrupt, PaneHandle, spawn_pane},
    shutdown::WorkerReaper,
};

const NOTICE_ITEMS: usize = 1024;
const NOTICE_BYTES: usize = 4 * 1024 * 1024;
const NOTICE_LIMITS: MailboxLimits = match MailboxLimits::try_new(NOTICE_ITEMS, NOTICE_BYTES) {
    Ok(limits) => limits,
    Err(_) => panic!("runtime notice limits must be nonzero"),
};

/// Failure to install a new pane generation in a runtime hub.
#[derive(Debug)]
pub enum OpenPaneError {
    /// A live generation already owns the logical pane ID.
    AlreadyOpen(PaneId),
    /// The process-wide pane generation sequence was exhausted.
    GenerationExhausted(SequenceExhausted),
    /// A worker or reader thread could not be spawned.
    Spawn(io::Error),
}

impl fmt::Display for OpenPaneError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyOpen(pane) => write!(formatter, "pane {} is already open", pane.get()),
            Self::GenerationExhausted(error) => error.fmt(formatter),
            Self::Spawn(error) => write!(formatter, "failed to spawn pane worker: {error}"),
        }
    }
}

impl std::error::Error for OpenPaneError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AlreadyOpen(_) => None,
            Self::GenerationExhausted(error) => Some(error),
            Self::Spawn(error) => Some(error),
        }
    }
}

struct PaneSlot {
    token: PaneToken,
    handle: PaneHandle,
    interrupt: ErasedInterrupt,
    publication: Arc<PanePublication>,
    join: Option<JoinHandle<()>>,
    close_deadline: Option<Instant>,
}

/// Owner of live pane generations, notices, deadlines, and worker joins.
pub struct RuntimeHub<C = SystemClock> {
    clock: C,
    allocator: PaneTokenAllocator,
    panes: HashMap<PaneId, PaneSlot>,
    pane_order: Vec<PaneId>,
    fair_drain_cursor: usize,
    notice_sender: MailboxSender<PaneNotice>,
    notice_receiver: MailboxReceiver<PaneNotice>,
    reaper: WorkerReaper,
    completed_metrics: HashMap<PaneToken, PanePublicationMetrics>,
    pending_closed: HashMap<PaneToken, PaneNotice>,
    live_threads: Arc<AtomicUsize>,
    notice_waker: Arc<dyn Fn() + Send + Sync>,
}

impl<C: fmt::Debug> fmt::Debug for RuntimeHub<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeHub")
            .field("clock", &self.clock)
            .field("live_panes", &self.panes.len())
            .field("live_threads", &self.live_threads.load(Ordering::Acquire))
            .finish_non_exhaustive()
    }
}

impl<C: Clock> RuntimeHub<C> {
    /// Creates an empty hub with a bounded lossless notice queue.
    #[must_use]
    pub fn new(clock: C) -> Self {
        Self::new_with_notice_waker(clock, Arc::new(|| {}))
    }

    /// Creates an empty hub with a host callback invoked for every new notice.
    #[must_use]
    pub fn new_with_notice_waker(clock: C, notice_waker: Arc<dyn Fn() + Send + Sync>) -> Self {
        let (notice_sender, notice_receiver) = bounded_mailbox(NOTICE_LIMITS);
        Self {
            clock,
            allocator: PaneTokenAllocator::new(),
            panes: HashMap::new(),
            pane_order: Vec::new(),
            fair_drain_cursor: 0,
            notice_sender,
            notice_receiver,
            reaper: WorkerReaper::default(),
            completed_metrics: HashMap::new(),
            pending_closed: HashMap::new(),
            live_threads: Arc::new(AtomicUsize::new(0)),
            notice_waker,
        }
    }

    /// Opens one logical pane with a fresh process-wide generation.
    ///
    /// # Errors
    ///
    /// Returns an error if the logical pane is already open, generation space
    /// is exhausted, or the worker threads cannot be created.
    pub fn open<T: SessionTransport>(
        &mut self,
        pane: PaneId,
        transport: T,
        config: PaneWorkerConfig,
    ) -> Result<PaneHandle, OpenPaneError> {
        let runtime = TerminalRuntime::new(config.size);
        self.open_with_runtime(pane, transport, config, runtime)
    }

    /// Opens one logical pane with a caller-configured terminal runtime.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::open`].
    pub fn open_with_runtime<T: SessionTransport>(
        &mut self,
        pane: PaneId,
        transport: T,
        mut config: PaneWorkerConfig,
        runtime: TerminalRuntime,
    ) -> Result<PaneHandle, OpenPaneError> {
        if self.panes.contains_key(&pane) {
            return Err(OpenPaneError::AlreadyOpen(pane));
        }
        let token = self
            .allocator
            .issue(pane)
            .map_err(OpenPaneError::GenerationExhausted)?;
        config.size = runtime.terminal().grid().size();
        let spawned = spawn_pane(
            token,
            transport,
            config,
            runtime,
            self.clock.clone(),
            self.notice_sender.clone(),
            Arc::clone(&self.notice_waker),
            &self.live_threads,
        )
        .map_err(OpenPaneError::Spawn)?;
        let handle = spawned.handle.clone();
        self.panes.insert(
            pane,
            PaneSlot {
                token,
                handle: spawned.handle,
                interrupt: spawned.interrupt,
                publication: spawned.publication,
                join: Some(spawned.join),
                close_deadline: None,
            },
        );
        if !self.pane_order.contains(&pane) {
            self.pane_order.push(pane);
        }
        Ok(handle)
    }

    /// Replaces a logical pane with a fresh generation and rejects old handles.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::open`] for the replacement session.
    pub fn restart<T: SessionTransport>(
        &mut self,
        pane: PaneId,
        transport: T,
        config: PaneWorkerConfig,
    ) -> Result<PaneHandle, OpenPaneError> {
        let runtime = TerminalRuntime::new(config.size);
        self.restart_with_runtime(pane, transport, config, runtime)
    }

    /// Replaces a logical pane with a caller-configured runtime and fresh generation.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::open_with_runtime`] for the replacement
    /// session.
    pub fn restart_with_runtime<T: SessionTransport>(
        &mut self,
        pane: PaneId,
        transport: T,
        config: PaneWorkerConfig,
        runtime: TerminalRuntime,
    ) -> Result<PaneHandle, OpenPaneError> {
        let order_position = self
            .pane_order
            .iter()
            .position(|candidate| *candidate == pane);
        if let Some(token) = self.panes.get(&pane).map(|slot| slot.token) {
            let _ = self.begin_close(token, Duration::ZERO);
            let _ = self.reap_expired();
        }
        let handle = self.open_with_runtime(pane, transport, config, runtime)?;
        if let Some(position) = order_position {
            self.reposition_pane(pane, position);
        }
        Ok(handle)
    }

    /// Begins orderly close and records the external-interrupt deadline.
    #[must_use]
    pub fn begin_close(&mut self, token: PaneToken, grace: Duration) -> bool {
        let Some(slot) = self.panes.get_mut(&token.pane()) else {
            return false;
        };
        if slot.token != token {
            return false;
        }
        slot.handle.request_close();
        let now = self.clock.now();
        slot.close_deadline = Some(now.checked_add(grace).unwrap_or(now));
        true
    }

    /// Interrupts expired sessions and transfers their joins to the reaper.
    #[must_use]
    pub fn reap_expired(&mut self) -> usize {
        let now = self.clock.now();
        let expired = self
            .panes
            .iter()
            .filter_map(|(pane, slot)| {
                slot.close_deadline
                    .filter(|deadline| *deadline <= now)
                    .map(|_| *pane)
            })
            .collect::<Vec<_>>();
        for pane in &expired {
            if let Some(mut slot) = self.panes.remove(pane) {
                slot.handle.stop_accepting();
                let _ = slot.interrupt.interrupt();
                self.completed_metrics
                    .insert(slot.token, slot.publication.metrics());
                if let Some(join) = slot.join.take() {
                    self.reaper.handoff(join);
                }
                self.remove_pane_order(*pane);
            }
        }
        self.reaper.reap_finished();
        expired.len()
    }

    /// Waits for the next notice from the current pane generation.
    ///
    /// # Errors
    ///
    /// Returns [`RecvError::Closed`] after hub shutdown and queue drain.
    pub fn recv_notice(&mut self) -> Result<PaneNotice, RecvError> {
        if let Some(notice) = self.take_drained_close() {
            return Ok(notice);
        }
        loop {
            let notice = self.notice_receiver.recv()?;
            if self.is_current(notice.pane()) {
                if matches!(notice, PaneNotice::Closed { .. }) && self.notice_has_work(&notice) {
                    let token = notice.pane();
                    self.pending_closed.insert(token, notice);
                    return Ok(PaneNotice::Wake(token));
                }
                self.finish_closed_slot(&notice);
                return Ok(notice);
            }
        }
    }

    /// Attempts to receive one notice, discarding stale generations.
    ///
    /// # Errors
    ///
    /// Returns empty or closed status from the bounded notice mailbox.
    pub fn try_recv_notice(&mut self) -> Result<PaneNotice, TryRecvError> {
        if let Some(notice) = self.take_drained_close() {
            return Ok(notice);
        }
        loop {
            let notice = self.notice_receiver.try_recv()?;
            if self.is_current(notice.pane()) {
                if matches!(notice, PaneNotice::Closed { .. }) && self.notice_has_work(&notice) {
                    let token = notice.pane();
                    self.pending_closed.insert(token, notice);
                    return Ok(PaneNotice::Wake(token));
                }
                self.finish_closed_slot(&notice);
                return Ok(notice);
            }
        }
    }

    /// Drains the latest frame and up to `max_effects` lossless effects.
    ///
    /// When work remains, the returned drain requests exactly one continuation
    /// turn. The host must schedule that turn without relying on a second queue
    /// insertion, which keeps continuation delivery independent of queue space.
    #[must_use]
    pub fn drain_pane(&self, token: PaneToken, max_effects: usize) -> Option<PaneDrain> {
        let slot = self.panes.get(&token.pane())?;
        if slot.token != token {
            return None;
        }
        Some(slot.publication.drain(max_effects))
    }

    /// Drains ready panes in stable round-robin order.
    ///
    /// A pane that continuously republishes cannot jump ahead of already-ready
    /// work from a later pane in the logical ownership order.
    #[must_use]
    pub fn drain_ready_fair(
        &mut self,
        max_panes: usize,
        max_effects_per_pane: usize,
    ) -> Vec<(PaneToken, PaneDrain)> {
        if max_panes == 0 || self.pane_order.is_empty() {
            return Vec::new();
        }
        let pane_count = self.pane_order.len();
        let start = self.fair_drain_cursor % pane_count;
        let mut drained = Vec::new();
        for offset in 0..pane_count {
            if drained.len() == max_panes {
                break;
            }
            let index = (start + offset) % pane_count;
            let pane = self.pane_order[index];
            let Some(slot) = self
                .panes
                .get(&pane)
                .filter(|slot| slot.publication.has_work())
            else {
                continue;
            };
            drained.push((slot.token, slot.publication.drain(max_effects_per_pane)));
            self.fair_drain_cursor = (index + 1) % pane_count;
        }
        drained
    }

    /// Returns publication, replacement, wake, and effect high-water metrics.
    #[must_use]
    pub fn publication_metrics(&self, token: PaneToken) -> Option<PanePublicationMetrics> {
        self.panes
            .get(&token.pane())
            .filter(|slot| slot.token == token)
            .map(|slot| slot.publication.metrics())
            .or_else(|| self.completed_metrics.get(&token).copied())
    }

    /// Returns the number of worker and blocking-reader threads still alive.
    #[must_use]
    pub fn live_thread_count(&self) -> usize {
        self.live_threads.load(Ordering::Acquire)
    }

    /// Interrupts and joins every pane generation and deadline reaper.
    pub fn shutdown(&mut self) {
        let _ = self.notice_receiver.close();
        for slot in self.panes.values_mut() {
            slot.handle.stop_accepting();
            let _ = slot.interrupt.interrupt();
        }
        for (_, mut slot) in self.panes.drain() {
            if let Some(join) = slot.join.take() {
                let _ = join.join();
            }
        }
        self.reaper.join_all();
    }

    fn is_current(&self, token: PaneToken) -> bool {
        self.panes
            .get(&token.pane())
            .is_some_and(|slot| slot.token == token)
    }

    fn finish_closed_slot(&mut self, notice: &PaneNotice) {
        if !matches!(notice, PaneNotice::Closed { .. }) {
            return;
        }
        let pane = notice.pane().pane();
        if let Some(mut slot) = self.panes.remove(&pane) {
            self.completed_metrics
                .insert(slot.token, slot.publication.metrics());
            if let Some(join) = slot.join.take() {
                self.reaper.handoff(join);
            }
            self.remove_pane_order(pane);
            self.reaper.reap_finished();
        }
    }

    fn notice_has_work(&self, notice: &PaneNotice) -> bool {
        self.panes
            .get(&notice.pane().pane())
            .filter(|slot| slot.token == notice.pane())
            .is_some_and(|slot| slot.publication.has_work())
    }

    fn take_drained_close(&mut self) -> Option<PaneNotice> {
        let token = self.pending_closed.keys().copied().find(|token| {
            self.panes
                .get(&token.pane())
                .filter(|slot| slot.token == *token)
                .is_none_or(|slot| !slot.publication.has_work())
        })?;
        let notice = self.pending_closed.remove(&token)?;
        self.finish_closed_slot(&notice);
        Some(notice)
    }

    fn remove_pane_order(&mut self, pane: PaneId) {
        let Some(position) = self
            .pane_order
            .iter()
            .position(|candidate| *candidate == pane)
        else {
            return;
        };
        self.pane_order.remove(position);
        if self.pane_order.is_empty() {
            self.fair_drain_cursor = 0;
        } else {
            if position < self.fair_drain_cursor {
                self.fair_drain_cursor -= 1;
            }
            self.fair_drain_cursor %= self.pane_order.len();
        }
    }

    fn reposition_pane(&mut self, pane: PaneId, position: usize) {
        self.remove_pane_order(pane);
        let position = position.min(self.pane_order.len());
        self.pane_order.insert(position, pane);
        self.fair_drain_cursor = self.fair_drain_cursor.min(self.pane_order.len() - 1);
    }
}

impl<C> Drop for RuntimeHub<C> {
    fn drop(&mut self) {
        let _ = self.notice_receiver.close();
        for slot in self.panes.values_mut() {
            slot.handle.stop_accepting();
            let _ = slot.interrupt.interrupt();
        }
        for (_, mut slot) in self.panes.drain() {
            if let Some(join) = slot.join.take() {
                let _ = join.join();
            }
        }
        self.reaper.join_all();
    }
}
