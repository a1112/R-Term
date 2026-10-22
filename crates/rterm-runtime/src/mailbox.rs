use std::{
    collections::VecDeque,
    fmt, mem,
    sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError},
    time::{Duration, Instant},
};

/// Reports the owned payload retained while an item is queued.
///
/// Implementations should include retained allocation (for example a
/// `Vec`'s capacity), not only the currently initialized byte length.
pub trait MailboxItem {
    /// Returns the byte reservation required while this item is queued.
    fn retained_bytes(&self) -> usize;
}

/// Item and retained-byte limits for a bounded mailbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MailboxLimits {
    max_items: usize,
    max_bytes: usize,
}

impl MailboxLimits {
    /// Creates nonzero item and byte limits.
    ///
    /// # Errors
    ///
    /// Returns [`MailboxLimitsError`] when either limit is zero. Item-limit
    /// validation has priority when both are zero.
    pub const fn try_new(max_items: usize, max_bytes: usize) -> Result<Self, MailboxLimitsError> {
        if max_items == 0 {
            return Err(MailboxLimitsError::ZeroItems);
        }
        if max_bytes == 0 {
            return Err(MailboxLimitsError::ZeroBytes);
        }
        Ok(Self {
            max_items,
            max_bytes,
        })
    }

    /// Returns the maximum number of queued items.
    #[must_use]
    pub const fn max_items(self) -> usize {
        self.max_items
    }

    /// Returns the maximum retained payload bytes.
    #[must_use]
    pub const fn max_bytes(self) -> usize {
        self.max_bytes
    }
}

/// Invalid bounded-mailbox limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailboxLimitsError {
    /// The item limit was zero.
    ZeroItems,
    /// The retained-byte limit was zero.
    ZeroBytes,
}

impl fmt::Display for MailboxLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroItems => formatter.write_str("mailbox item limit must be nonzero"),
            Self::ZeroBytes => formatter.write_str("mailbox byte limit must be nonzero"),
        }
    }
}

impl std::error::Error for MailboxLimitsError {}

/// A nonblocking send failure that preserves ownership of the item.
#[derive(Debug, PartialEq, Eq)]
pub enum TrySendError<T> {
    /// The mailbox no longer accepts items.
    Closed(T),
    /// The item can never fit within the configured byte limit.
    Oversize {
        /// Rejected item.
        item: T,
        /// Reservation requested by the item.
        item_bytes: usize,
        /// Configured maximum retained bytes.
        max_bytes: usize,
    },
    /// The item could fit later, but capacity is currently unavailable.
    Full {
        /// Rejected item.
        item: T,
        /// Reservation requested by the item.
        item_bytes: usize,
    },
}

impl<T> fmt::Display for TrySendError<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed(_) => formatter.write_str("mailbox is closed"),
            Self::Oversize {
                item_bytes,
                max_bytes,
                ..
            } => write!(
                formatter,
                "mailbox item requires {item_bytes} bytes but the limit is {max_bytes}"
            ),
            Self::Full { .. } => formatter.write_str("mailbox is full"),
        }
    }
}

impl<T: fmt::Debug> std::error::Error for TrySendError<T> {}

/// A blocking send failure that preserves ownership of the item.
#[derive(Debug, PartialEq, Eq)]
pub enum SendError<T> {
    /// The mailbox closed before accepting the item.
    Closed(T),
    /// The item can never fit within the configured byte limit.
    Oversize {
        /// Rejected item.
        item: T,
        /// Reservation requested by the item.
        item_bytes: usize,
        /// Configured maximum retained bytes.
        max_bytes: usize,
    },
}

impl<T> fmt::Display for SendError<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed(_) => formatter.write_str("mailbox is closed"),
            Self::Oversize {
                item_bytes,
                max_bytes,
                ..
            } => write!(
                formatter,
                "mailbox item requires {item_bytes} bytes but the limit is {max_bytes}"
            ),
        }
    }
}

impl<T: fmt::Debug> std::error::Error for SendError<T> {}

/// A nonblocking receive failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryRecvError {
    /// The mailbox is open but currently empty.
    Empty,
    /// The mailbox is closed and fully drained.
    Closed,
}

impl fmt::Display for TryRecvError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("mailbox is empty"),
            Self::Closed => formatter.write_str("mailbox is closed"),
        }
    }
}

impl std::error::Error for TryRecvError {}

/// A blocking receive failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvError {
    /// The mailbox is closed and fully drained.
    Closed,
}

impl fmt::Display for RecvError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("mailbox is closed")
    }
}

impl std::error::Error for RecvError {}

/// A timed blocking receive failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvTimeoutError {
    /// The timeout elapsed while the mailbox remained open and empty.
    Timeout,
    /// The mailbox is closed and fully drained.
    Closed,
}

impl fmt::Display for RecvTimeoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => formatter.write_str("mailbox receive timed out"),
            Self::Closed => formatter.write_str("mailbox is closed"),
        }
    }
}

impl std::error::Error for RecvTimeoutError {}

/// A point-in-time mailbox occupancy and cumulative accounting snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MailboxMetrics {
    /// Configured limits.
    pub limits: MailboxLimits,
    /// Currently queued items.
    pub queued_items: usize,
    /// Currently reserved bytes.
    pub queued_bytes: usize,
    /// Largest observed queued item count.
    pub high_water_items: usize,
    /// Largest observed queued byte reservation.
    pub high_water_bytes: usize,
    /// Successfully enqueued items.
    pub enqueued_items: u64,
    /// Successfully enqueued retained bytes.
    pub enqueued_bytes: u64,
    /// Items delivered to the receiver.
    pub dequeued_items: u64,
    /// Bytes released by successful receives.
    pub dequeued_bytes: u64,
    /// Items discarded when the receiver was dropped.
    pub discarded_items: u64,
    /// Bytes discarded when the receiver was dropped.
    pub discarded_bytes: u64,
    /// Nonblocking sends rejected for temporary capacity exhaustion.
    pub full_rejections: u64,
    /// Sends rejected because the item can never fit.
    pub oversize_rejections: u64,
    /// Sends rejected after closure.
    pub closed_rejections: u64,
    /// Blocking send calls that actually waited for capacity.
    pub blocked_sends: u64,
    /// Sum of producer time spent waiting for capacity.
    pub send_blocked_duration: Duration,
    /// Producers currently waiting for capacity.
    pub waiting_producers: usize,
    /// Whether the single consumer is waiting for an item.
    pub consumer_waiting: bool,
}

#[derive(Debug)]
struct Entry<T> {
    value: T,
    reserved_bytes: usize,
}

#[derive(Debug, Default)]
struct Counters {
    high_water_items: usize,
    high_water_bytes: usize,
    enqueued_items: u64,
    enqueued_bytes: u64,
    dequeued_items: u64,
    dequeued_bytes: u64,
    discarded_items: u64,
    discarded_bytes: u64,
    full_rejections: u64,
    oversize_rejections: u64,
    closed_rejections: u64,
    blocked_sends: u64,
    send_blocked_duration: Duration,
}

#[derive(Debug)]
struct State<T> {
    queue: VecDeque<Entry<T>>,
    queued_bytes: usize,
    accepting: bool,
    sender_count: usize,
    consumer_alive: bool,
    waiting_producers: usize,
    consumer_waiting: bool,
    counters: Counters,
}

struct Shared<T> {
    limits: MailboxLimits,
    state: Mutex<State<T>>,
    not_empty: Condvar,
    space_available: Condvar,
}

impl<T> Shared<T> {
    fn lock(&self) -> MutexGuard<'_, State<T>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn close(&self) -> bool {
        let changed = {
            let mut state = self.lock();
            let changed = state.accepting;
            state.accepting = false;
            changed
        };
        if changed {
            self.not_empty.notify_all();
            self.space_available.notify_all();
        }
        changed
    }

    fn is_closed(&self) -> bool {
        !self.lock().accepting
    }

    fn metrics(&self) -> MailboxMetrics {
        let state = self.lock();
        MailboxMetrics {
            limits: self.limits,
            queued_items: state.queue.len(),
            queued_bytes: state.queued_bytes,
            high_water_items: state.counters.high_water_items,
            high_water_bytes: state.counters.high_water_bytes,
            enqueued_items: state.counters.enqueued_items,
            enqueued_bytes: state.counters.enqueued_bytes,
            dequeued_items: state.counters.dequeued_items,
            dequeued_bytes: state.counters.dequeued_bytes,
            discarded_items: state.counters.discarded_items,
            discarded_bytes: state.counters.discarded_bytes,
            full_rejections: state.counters.full_rejections,
            oversize_rejections: state.counters.oversize_rejections,
            closed_rejections: state.counters.closed_rejections,
            blocked_sends: state.counters.blocked_sends,
            send_blocked_duration: state.counters.send_blocked_duration,
            waiting_producers: state.waiting_producers,
            consumer_waiting: state.consumer_waiting,
        }
    }
}

/// Cloneable producer endpoint of a byte-budgeted bounded mailbox.
pub struct MailboxSender<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Clone for MailboxSender<T> {
    fn clone(&self) -> Self {
        {
            let mut state = self.shared.lock();
            state.sender_count = state.sender_count.saturating_add(1);
        }
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<T> MailboxSender<T> {
    /// Stops all producers from admitting new items.
    ///
    /// Already queued items remain available to the receiver.
    #[must_use]
    pub fn close(&self) -> bool {
        self.shared.close()
    }

    /// Reports whether new admissions have stopped.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.shared.is_closed()
    }

    /// Returns a consistent accounting snapshot.
    #[must_use]
    pub fn metrics(&self) -> MailboxMetrics {
        self.shared.metrics()
    }
}

impl<T: MailboxItem> MailboxSender<T> {
    /// Attempts to enqueue without waiting.
    ///
    /// # Errors
    ///
    /// Returns a typed error containing the original item when the mailbox is
    /// closed, the item is oversized, or capacity is temporarily full.
    pub fn try_send(&self, item: T) -> Result<(), TrySendError<T>> {
        let item_bytes = item.retained_bytes();
        let mut state = self.shared.lock();
        if !state.accepting || !state.consumer_alive {
            state.counters.closed_rejections = state.counters.closed_rejections.saturating_add(1);
            return Err(TrySendError::Closed(item));
        }
        if item_bytes > self.shared.limits.max_bytes {
            state.counters.oversize_rejections =
                state.counters.oversize_rejections.saturating_add(1);
            return Err(TrySendError::Oversize {
                item,
                item_bytes,
                max_bytes: self.shared.limits.max_bytes,
            });
        }
        let Some(next_bytes) = admission_bytes(&state, self.shared.limits, item_bytes) else {
            state.counters.full_rejections = state.counters.full_rejections.saturating_add(1);
            return Err(TrySendError::Full { item, item_bytes });
        };
        enqueue(&mut state, item, item_bytes, next_bytes);
        drop(state);
        self.shared.not_empty.notify_one();
        Ok(())
    }

    /// Waits until the item is admitted or the mailbox closes.
    ///
    /// # Errors
    ///
    /// Returns a typed error containing the original item when the mailbox is
    /// closed or the item can never fit within the byte limit.
    pub fn send(&self, item: T) -> Result<(), SendError<T>> {
        let item_bytes = item.retained_bytes();
        let mut state = self.shared.lock();
        if !state.accepting || !state.consumer_alive {
            state.counters.closed_rejections = state.counters.closed_rejections.saturating_add(1);
            return Err(SendError::Closed(item));
        }
        if item_bytes > self.shared.limits.max_bytes {
            state.counters.oversize_rejections =
                state.counters.oversize_rejections.saturating_add(1);
            return Err(SendError::Oversize {
                item,
                item_bytes,
                max_bytes: self.shared.limits.max_bytes,
            });
        }

        let mut blocked_since = None;
        loop {
            if !state.accepting || !state.consumer_alive {
                finish_blocked_send(&mut state, blocked_since);
                state.counters.closed_rejections =
                    state.counters.closed_rejections.saturating_add(1);
                return Err(SendError::Closed(item));
            }
            if let Some(next_bytes) = admission_bytes(&state, self.shared.limits, item_bytes) {
                finish_blocked_send(&mut state, blocked_since);
                enqueue(&mut state, item, item_bytes, next_bytes);
                drop(state);
                self.shared.not_empty.notify_one();
                return Ok(());
            }
            if blocked_since.is_none() {
                blocked_since = Some(Instant::now());
                state.waiting_producers = state.waiting_producers.saturating_add(1);
                state.counters.blocked_sends = state.counters.blocked_sends.saturating_add(1);
            }
            state = self
                .shared
                .space_available
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }
}

impl<T> Drop for MailboxSender<T> {
    fn drop(&mut self) {
        let closed = {
            let mut state = self.shared.lock();
            state.sender_count = state.sender_count.saturating_sub(1);
            if state.sender_count == 0 && state.accepting {
                state.accepting = false;
                true
            } else {
                false
            }
        };
        if closed {
            self.shared.not_empty.notify_all();
            self.shared.space_available.notify_all();
        }
    }
}

/// Single-consumer endpoint of a byte-budgeted bounded mailbox.
///
/// This type is intentionally not cloneable; receive operations also require
/// mutable access to make the single-consumer contract explicit.
pub struct MailboxReceiver<T> {
    shared: Arc<Shared<T>>,
}

impl<T> MailboxReceiver<T> {
    /// Stops all producers from admitting new items while preserving the
    /// already queued drain.
    #[must_use]
    pub fn close(&self) -> bool {
        self.shared.close()
    }

    /// Reports whether new admissions have stopped.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.shared.is_closed()
    }

    /// Returns a consistent accounting snapshot.
    #[must_use]
    pub fn metrics(&self) -> MailboxMetrics {
        self.shared.metrics()
    }

    /// Attempts to receive without waiting.
    ///
    /// # Errors
    ///
    /// Returns [`TryRecvError::Empty`] while the mailbox can still receive
    /// future items, and [`TryRecvError::Closed`] after closure and drain.
    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        let mut state = self.shared.lock();
        if let Some(entry) = dequeue(&mut state) {
            drop(state);
            self.shared.space_available.notify_all();
            return Ok(entry.value);
        }
        if state.accepting {
            Err(TryRecvError::Empty)
        } else {
            Err(TryRecvError::Closed)
        }
    }

    /// Waits for the next item, draining queued items after closure.
    ///
    /// # Errors
    ///
    /// Returns [`RecvError::Closed`] once the closed mailbox is empty.
    pub fn recv(&mut self) -> Result<T, RecvError> {
        let mut state = self.shared.lock();
        while state.queue.is_empty() && state.accepting {
            state.consumer_waiting = true;
            state = self
                .shared
                .not_empty
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
        state.consumer_waiting = false;
        if let Some(entry) = dequeue(&mut state) {
            drop(state);
            self.shared.space_available.notify_all();
            Ok(entry.value)
        } else {
            Err(RecvError::Closed)
        }
    }

    /// Waits up to `timeout` for the next item, draining queued items after closure.
    ///
    /// # Errors
    ///
    /// Returns [`RecvTimeoutError::Timeout`] if the mailbox stays open and empty,
    /// or [`RecvTimeoutError::Closed`] once a closed mailbox is fully drained.
    pub fn recv_timeout(&mut self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        let Some(deadline) = Instant::now().checked_add(timeout) else {
            return self
                .recv()
                .map_err(|RecvError::Closed| RecvTimeoutError::Closed);
        };
        let mut state = self.shared.lock();
        loop {
            if let Some(entry) = dequeue(&mut state) {
                state.consumer_waiting = false;
                drop(state);
                self.shared.space_available.notify_all();
                return Ok(entry.value);
            }
            if !state.accepting {
                state.consumer_waiting = false;
                return Err(RecvTimeoutError::Closed);
            }
            let now = Instant::now();
            if now >= deadline {
                state.consumer_waiting = false;
                return Err(RecvTimeoutError::Timeout);
            }
            state.consumer_waiting = true;
            let (next, wait) = self
                .shared
                .not_empty
                .wait_timeout(state, deadline.saturating_duration_since(now))
                .unwrap_or_else(PoisonError::into_inner);
            state = next;
            if wait.timed_out() && state.queue.is_empty() && state.accepting {
                state.consumer_waiting = false;
                return Err(RecvTimeoutError::Timeout);
            }
        }
    }
}

impl<T> Drop for MailboxReceiver<T> {
    fn drop(&mut self) {
        let discarded = {
            let mut state = self.shared.lock();
            state.consumer_alive = false;
            state.accepting = false;
            state.consumer_waiting = false;
            let discarded_items = usize_to_u64(state.queue.len());
            let discarded_bytes = usize_to_u64(state.queued_bytes);
            state.counters.discarded_items = state
                .counters
                .discarded_items
                .saturating_add(discarded_items);
            state.counters.discarded_bytes = state
                .counters
                .discarded_bytes
                .saturating_add(discarded_bytes);
            state.queued_bytes = 0;
            mem::take(&mut state.queue)
        };
        self.shared.not_empty.notify_all();
        self.shared.space_available.notify_all();
        drop(discarded);
    }
}

/// Creates one byte-budgeted bounded mailbox.
#[must_use]
pub fn bounded_mailbox<T: MailboxItem>(
    limits: MailboxLimits,
) -> (MailboxSender<T>, MailboxReceiver<T>) {
    let shared = Arc::new(Shared {
        limits,
        state: Mutex::new(State {
            queue: VecDeque::new(),
            queued_bytes: 0,
            accepting: true,
            sender_count: 1,
            consumer_alive: true,
            waiting_producers: 0,
            consumer_waiting: false,
            counters: Counters::default(),
        }),
        not_empty: Condvar::new(),
        space_available: Condvar::new(),
    });
    (
        MailboxSender {
            shared: Arc::clone(&shared),
        },
        MailboxReceiver { shared },
    )
}

fn admission_bytes<T>(state: &State<T>, limits: MailboxLimits, item_bytes: usize) -> Option<usize> {
    if state.queue.len() >= limits.max_items {
        return None;
    }
    state
        .queued_bytes
        .checked_add(item_bytes)
        .filter(|next| *next <= limits.max_bytes)
}

fn enqueue<T>(state: &mut State<T>, item: T, item_bytes: usize, next_bytes: usize) {
    state.queue.push_back(Entry {
        value: item,
        reserved_bytes: item_bytes,
    });
    state.queued_bytes = next_bytes;
    state.counters.high_water_items = state.counters.high_water_items.max(state.queue.len());
    state.counters.high_water_bytes = state.counters.high_water_bytes.max(next_bytes);
    state.counters.enqueued_items = state.counters.enqueued_items.saturating_add(1);
    state.counters.enqueued_bytes = state
        .counters
        .enqueued_bytes
        .saturating_add(usize_to_u64(item_bytes));
}

fn dequeue<T>(state: &mut State<T>) -> Option<Entry<T>> {
    let entry = state.queue.pop_front()?;
    state.queued_bytes = state.queued_bytes.saturating_sub(entry.reserved_bytes);
    state.counters.dequeued_items = state.counters.dequeued_items.saturating_add(1);
    state.counters.dequeued_bytes = state
        .counters
        .dequeued_bytes
        .saturating_add(usize_to_u64(entry.reserved_bytes));
    Some(entry)
}

fn finish_blocked_send<T>(state: &mut State<T>, blocked_since: Option<Instant>) {
    if let Some(blocked_since) = blocked_since {
        state.waiting_producers = state.waiting_producers.saturating_sub(1);
        state.counters.send_blocked_duration = state
            .counters
            .send_blocked_duration
            .saturating_add(blocked_since.elapsed());
    }
}

fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
