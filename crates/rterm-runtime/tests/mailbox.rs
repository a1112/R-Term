use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc, Barrier,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::sync_channel,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use rterm_runtime::{
    MailboxItem, MailboxLimits, MailboxLimitsError, RecvError, RecvTimeoutError, SendError,
    TryRecvError, TrySendError, bounded_mailbox,
};

#[derive(Debug, PartialEq, Eq)]
struct Item {
    id: usize,
    bytes: usize,
}

impl Item {
    const fn new(id: usize, bytes: usize) -> Self {
        Self { id, bytes }
    }
}

impl MailboxItem for Item {
    fn retained_bytes(&self) -> usize {
        self.bytes
    }
}

fn limits(items: usize, bytes: usize) -> MailboxLimits {
    MailboxLimits::try_new(items, bytes).expect("positive mailbox limits")
}

fn wait_until(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !predicate() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for mailbox state"
        );
        thread::yield_now();
    }
}

fn join_with_watchdog<T>(worker: JoinHandle<T>) -> T {
    wait_until(|| worker.is_finished());
    worker.join().expect("mailbox worker thread")
}

#[derive(Debug)]
struct MutableCostItem {
    id: usize,
    bytes: Arc<AtomicUsize>,
}

impl MailboxItem for MutableCostItem {
    fn retained_bytes(&self) -> usize {
        self.bytes.load(Ordering::Acquire)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct FallibleCostItem {
    id: usize,
    bytes: usize,
    panic_in_cost: bool,
}

impl MailboxItem for FallibleCostItem {
    fn retained_bytes(&self) -> usize {
        assert!(!self.panic_in_cost, "intentional retained-byte panic");
        self.bytes
    }
}

#[derive(Debug)]
struct BlockingCostItem {
    id: usize,
    entered: Arc<AtomicBool>,
    release: Arc<AtomicBool>,
}

impl MailboxItem for BlockingCostItem {
    fn retained_bytes(&self) -> usize {
        self.entered.store(true, Ordering::Release);
        while !self.release.load(Ordering::Acquire) {
            thread::yield_now();
        }
        1
    }
}

#[derive(Debug)]
struct BlockingDropItem {
    drop_started: Arc<AtomicBool>,
    release_drop: Arc<AtomicBool>,
}

#[derive(Debug)]
struct PanicDropItem;

impl MailboxItem for PanicDropItem {
    fn retained_bytes(&self) -> usize {
        1
    }
}

impl Drop for PanicDropItem {
    fn drop(&mut self) {
        panic!("intentional queued-item drop panic");
    }
}

impl MailboxItem for BlockingDropItem {
    fn retained_bytes(&self) -> usize {
        1
    }
}

impl Drop for BlockingDropItem {
    fn drop(&mut self) {
        self.drop_started.store(true, Ordering::Release);
        while !self.release_drop.load(Ordering::Acquire) {
            thread::yield_now();
        }
    }
}

#[test]
fn limits_reject_zero_and_expose_both_budgets() {
    assert_eq!(
        MailboxLimits::try_new(0, 0),
        Err(MailboxLimitsError::ZeroItems),
        "item validation has deterministic priority"
    );
    assert_eq!(
        MailboxLimits::try_new(0, 4),
        Err(MailboxLimitsError::ZeroItems)
    );
    assert_eq!(
        MailboxLimits::try_new(4, 0),
        Err(MailboxLimitsError::ZeroBytes)
    );
    let limits = limits(3, 17);
    assert_eq!(limits.max_items(), 3);
    assert_eq!(limits.max_bytes(), 17);
}

#[test]
fn item_and_byte_budgets_are_independently_enforced() {
    let (items_tx, mut items_rx) = bounded_mailbox(limits(2, 100));
    items_tx.try_send(Item::new(1, 1)).expect("first item");
    items_tx.try_send(Item::new(2, 1)).expect("second item");
    assert_eq!(
        items_tx.try_send(Item::new(3, 1)),
        Err(TrySendError::Full {
            item: Item::new(3, 1),
            item_bytes: 1,
        })
    );
    assert_eq!(items_rx.try_recv().expect("queued item").id, 1);

    let (bytes_tx, _bytes_rx) = bounded_mailbox(limits(4, 5));
    bytes_tx.try_send(Item::new(1, 4)).expect("four bytes");
    assert_eq!(
        bytes_tx.try_send(Item::new(2, 2)),
        Err(TrySendError::Full {
            item: Item::new(2, 2),
            item_bytes: 2,
        })
    );

    let item_metrics = items_tx.metrics();
    assert_eq!(item_metrics.high_water_items, 2);
    assert_eq!(item_metrics.high_water_bytes, 2);
    let byte_metrics = bytes_tx.metrics();
    assert_eq!(byte_metrics.high_water_items, 1);
    assert_eq!(byte_metrics.high_water_bytes, 4);
}

#[test]
fn fifo_and_pop_release_the_original_reservation() {
    let (tx, mut rx) = bounded_mailbox(limits(3, 6));
    for id in 0..3 {
        tx.try_send(Item::new(id, 2)).expect("within both budgets");
    }
    assert_eq!(tx.metrics().queued_bytes, 6);
    assert_eq!(rx.try_recv().expect("first").id, 0);
    assert_eq!(tx.metrics().queued_bytes, 4);
    tx.try_send(Item::new(3, 2)).expect("released reservation");
    assert_eq!(rx.try_recv().expect("second").id, 1);
    assert_eq!(rx.try_recv().expect("third").id, 2);
    assert_eq!(rx.try_recv().expect("fourth").id, 3);
    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
}

#[test]
fn blocked_sender_is_observable_and_pop_unblocks_it() {
    let (tx, mut rx) = bounded_mailbox(limits(1, 1));
    tx.try_send(Item::new(1, 1)).expect("fill mailbox");
    let blocked_tx = tx.clone();
    let worker = thread::spawn(move || blocked_tx.send(Item::new(2, 1)));

    wait_until(|| tx.metrics().waiting_producers == 1);
    assert_eq!(rx.try_recv().expect("release capacity").id, 1);
    join_with_watchdog(worker).expect("sender resumes");
    assert_eq!(rx.try_recv().expect("blocked item").id, 2);

    let metrics = tx.metrics();
    assert_eq!(metrics.blocked_sends, 1);
    assert_eq!(metrics.waiting_producers, 0);
    assert!(metrics.send_blocked_duration > Duration::ZERO);
}

#[test]
fn close_wakes_blocked_sender_and_returns_its_item() {
    let (tx, mut rx) = bounded_mailbox(limits(1, 1));
    tx.try_send(Item::new(1, 1)).expect("fill mailbox");
    let blocked_tx = tx.clone();
    let worker = thread::spawn(move || blocked_tx.send(Item::new(2, 1)));

    wait_until(|| tx.metrics().waiting_producers == 1);
    assert!(tx.close());
    assert_eq!(
        join_with_watchdog(worker),
        Err(SendError::Closed(Item::new(2, 1)))
    );
    assert_eq!(rx.recv().expect("closed mailbox still drains").id, 1);
    assert_eq!(rx.recv(), Err(RecvError::Closed));
}

#[test]
fn close_wakes_an_empty_receiver() {
    let (tx, mut rx) = bounded_mailbox::<Item>(limits(1, 1));
    let worker = thread::spawn(move || rx.recv());

    wait_until(|| tx.metrics().consumer_waiting);
    assert!(tx.close());
    assert_eq!(join_with_watchdog(worker), Err(RecvError::Closed));
}

#[test]
fn timed_receive_distinguishes_timeout_delivery_and_close() {
    let (tx, mut rx) = bounded_mailbox::<Item>(limits(1, 1));

    assert_eq!(
        rx.recv_timeout(Duration::from_millis(1)),
        Err(RecvTimeoutError::Timeout)
    );
    tx.try_send(Item::new(7, 1)).expect("timed receive item");
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1))
            .expect("timed receive delivers")
            .id,
        7
    );
    assert!(tx.close());
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1)),
        Err(RecvTimeoutError::Closed)
    );
}

#[test]
fn controlled_multi_producer_pushes_remain_fifo_and_accounted() {
    const PRODUCERS: usize = 8;
    let (tx, mut rx) = bounded_mailbox(limits(PRODUCERS, PRODUCERS));
    let gate = Arc::new(AtomicUsize::new(0));
    let start = Arc::new(Barrier::new(PRODUCERS));
    let workers = (0..PRODUCERS)
        .map(|id| {
            let tx = tx.clone();
            let gate = Arc::clone(&gate);
            let start = Arc::clone(&start);
            thread::spawn(move || {
                start.wait();
                while gate.load(Ordering::Acquire) != id {
                    thread::yield_now();
                }
                tx.try_send(Item::new(id, 1)).expect("producer capacity");
                gate.fetch_add(1, Ordering::Release);
            })
        })
        .collect::<Vec<_>>();
    for worker in workers {
        join_with_watchdog(worker);
    }

    let observed = (0..PRODUCERS)
        .map(|_| rx.try_recv().expect("one item per producer").id)
        .collect::<Vec<_>>();
    assert_eq!(observed, (0..PRODUCERS).collect::<Vec<_>>());
    let metrics = tx.metrics();
    assert_eq!(metrics.enqueued_items, PRODUCERS as u64);
    assert_eq!(metrics.enqueued_bytes, PRODUCERS as u64);
    assert_eq!(metrics.dequeued_items, PRODUCERS as u64);
    assert_eq!(metrics.dequeued_bytes, PRODUCERS as u64);
}

#[test]
fn metrics_exactly_report_acceptance_rejections_and_drain() {
    let (tx, mut rx) = bounded_mailbox(limits(2, 5));
    tx.try_send(Item::new(1, 2)).expect("first item");
    tx.try_send(Item::new(2, 3)).expect("second item");
    assert!(matches!(
        tx.try_send(Item::new(3, 1)),
        Err(TrySendError::Full { .. })
    ));
    assert!(matches!(
        tx.try_send(Item::new(4, 6)),
        Err(TrySendError::Oversize { .. })
    ));
    assert_eq!(rx.try_recv().expect("first item").id, 1);
    assert!(tx.close());
    assert!(matches!(
        tx.try_send(Item::new(5, 1)),
        Err(TrySendError::Closed(_))
    ));
    assert_eq!(rx.try_recv().expect("second item").id, 2);

    let metrics = tx.metrics();
    assert_eq!(metrics.limits, limits(2, 5));
    assert_eq!(metrics.queued_items, 0);
    assert_eq!(metrics.queued_bytes, 0);
    assert_eq!(metrics.high_water_items, 2);
    assert_eq!(metrics.high_water_bytes, 5);
    assert_eq!(metrics.enqueued_items, 2);
    assert_eq!(metrics.enqueued_bytes, 5);
    assert_eq!(metrics.dequeued_items, 2);
    assert_eq!(metrics.dequeued_bytes, 5);
    assert_eq!(metrics.discarded_items, 0);
    assert_eq!(metrics.discarded_bytes, 0);
    assert_eq!(metrics.full_rejections, 1);
    assert_eq!(metrics.oversize_rejections, 1);
    assert_eq!(metrics.closed_rejections, 1);
    assert_eq!(metrics.blocked_sends, 0);
    assert_eq!(metrics.send_blocked_duration, Duration::ZERO);
    assert_eq!(metrics.waiting_producers, 0);
    assert!(!metrics.consumer_waiting);
}

#[test]
fn oversize_zero_byte_and_error_priority_are_explicit() {
    let (tx, _rx) = bounded_mailbox(limits(1, 2));
    tx.try_send(Item::new(1, 0))
        .expect("zero-byte item still occupies one slot");
    assert_eq!(
        tx.try_send(Item::new(2, 0)),
        Err(TrySendError::Full {
            item: Item::new(2, 0),
            item_bytes: 0,
        })
    );
    assert_eq!(
        tx.try_send(Item::new(3, 3)),
        Err(TrySendError::Oversize {
            item: Item::new(3, 3),
            item_bytes: 3,
            max_bytes: 2,
        }),
        "oversize has priority over temporary fullness"
    );
    assert_eq!(
        tx.send(Item::new(4, 3)),
        Err(SendError::Oversize {
            item: Item::new(4, 3),
            item_bytes: 3,
            max_bytes: 2,
        })
    );
    assert_eq!(tx.metrics().blocked_sends, 0);

    assert!(tx.close());
    assert_eq!(
        tx.try_send(Item::new(5, usize::MAX)),
        Err(TrySendError::Closed(Item::new(5, usize::MAX))),
        "closed has priority over oversize"
    );
    assert_eq!(
        tx.send(Item::new(6, usize::MAX)),
        Err(SendError::Closed(Item::new(6, usize::MAX)))
    );
}

#[test]
fn reservation_is_fixed_at_admission_and_checked_add_never_wraps() {
    let cost = Arc::new(AtomicUsize::new(4));
    let (tx, mut rx) = bounded_mailbox(limits(3, usize::MAX));
    tx.try_send(MutableCostItem {
        id: 1,
        bytes: Arc::clone(&cost),
    })
    .expect("initial mutable-cost item");
    cost.store(usize::MAX, Ordering::Release);
    assert_eq!(rx.try_recv().expect("mutable-cost item").id, 1);
    assert_eq!(tx.metrics().dequeued_bytes, 4);
    assert_eq!(tx.metrics().queued_bytes, 0);

    let (overflow_tx, mut overflow_rx) = bounded_mailbox(limits(3, usize::MAX));
    overflow_tx
        .try_send(Item::new(1, usize::MAX - 1))
        .expect("near-maximum reservation");
    assert_eq!(
        overflow_tx.try_send(Item::new(2, 2)),
        Err(TrySendError::Full {
            item: Item::new(2, 2),
            item_bytes: 2,
        }),
        "checked-add overflow must be treated as full"
    );
    assert_eq!(overflow_rx.try_recv().expect("release huge item").id, 1);
    overflow_tx
        .try_send(Item::new(2, 2))
        .expect("item fits after exact release");
}

#[test]
fn close_is_idempotent_and_last_sender_closes_after_fifo_drain() {
    let (tx, mut rx) = bounded_mailbox(limits(2, 2));
    let last = tx.clone();
    drop(tx);
    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
    last.try_send(Item::new(1, 1))
        .expect("remaining sender is live");
    drop(last);
    assert_eq!(rx.recv().expect("last-sender queue drain").id, 1);
    assert_eq!(rx.recv(), Err(RecvError::Closed));

    let (tx, rx) = bounded_mailbox::<Item>(limits(1, 1));
    assert!(tx.close());
    assert!(!tx.close());
    assert!(!rx.close());
    assert!(tx.is_closed());
    assert!(rx.is_closed());
    assert!(
        tx.clone().is_closed(),
        "cloning must never reopen a mailbox"
    );
}

#[test]
fn receiver_drop_discards_queue_and_wakes_blocked_senders() {
    let (tx, rx) = bounded_mailbox(limits(1, 1));
    tx.try_send(Item::new(1, 1)).expect("fill mailbox");
    let blocked_tx = tx.clone();
    let worker = thread::spawn(move || blocked_tx.send(Item::new(2, 1)));
    wait_until(|| tx.metrics().waiting_producers == 1);

    drop(rx);
    assert_eq!(
        join_with_watchdog(worker),
        Err(SendError::Closed(Item::new(2, 1)))
    );
    let metrics = tx.metrics();
    assert_eq!(metrics.queued_items, 0);
    assert_eq!(metrics.queued_bytes, 0);
    assert_eq!(metrics.discarded_items, 1);
    assert_eq!(metrics.discarded_bytes, 1);
    assert_eq!(metrics.closed_rejections, 1);
    assert!(tx.is_closed());
}

#[test]
fn retained_byte_panics_and_slow_costs_never_poison_or_hold_the_mutex() {
    let (tx, mut rx) = bounded_mailbox(limits(2, 2));
    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        let _ = tx.try_send(FallibleCostItem {
            id: 1,
            bytes: 1,
            panic_in_cost: true,
        });
    }));
    assert!(panic_result.is_err());
    tx.try_send(FallibleCostItem {
        id: 2,
        bytes: 1,
        panic_in_cost: false,
    })
    .expect("mailbox remains usable after cost panic");
    assert_eq!(rx.try_recv().expect("post-panic item").id, 2);

    let (slow_tx, _slow_rx) = bounded_mailbox(limits(2, 2));
    let entered = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let worker_tx = slow_tx.clone();
    let worker_entered = Arc::clone(&entered);
    let worker_release = Arc::clone(&release);
    let (result_tx, result_rx) = sync_channel(1);
    let worker = thread::spawn(move || {
        result_tx
            .send(worker_tx.try_send(BlockingCostItem {
                id: 3,
                entered: worker_entered,
                release: worker_release,
            }))
            .expect("publish slow-cost result");
    });
    wait_until(|| entered.load(Ordering::Acquire));
    assert_eq!(
        slow_tx.metrics().queued_items,
        0,
        "cost runs before locking"
    );
    assert!(slow_tx.close());
    release.store(true, Ordering::Release);
    match result_rx.recv().expect("slow-cost result") {
        Err(TrySendError::Closed(item)) => assert_eq!(item.id, 3),
        other => panic!("expected closed slow-cost item, got {other:?}"),
    }
    join_with_watchdog(worker);
}

#[test]
fn receiver_drop_releases_state_before_running_item_destructors() {
    let drop_started = Arc::new(AtomicBool::new(false));
    let release_drop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = bounded_mailbox(limits(1, 1));
    tx.try_send(BlockingDropItem {
        drop_started: Arc::clone(&drop_started),
        release_drop: Arc::clone(&release_drop),
    })
    .expect("queued blocking destructor");
    let drop_worker = thread::spawn(move || drop(rx));
    wait_until(|| drop_started.load(Ordering::Acquire));

    let observer_tx = tx.clone();
    let observer = thread::spawn(move || observer_tx.metrics());
    let metrics = join_with_watchdog(observer);
    assert_eq!(metrics.queued_items, 0);
    assert_eq!(metrics.discarded_items, 1);
    assert!(tx.is_closed());

    release_drop.store(true, Ordering::Release);
    join_with_watchdog(drop_worker);
}

#[test]
fn panicking_item_destructor_does_not_poison_released_mailbox_state() {
    let (tx, rx) = bounded_mailbox(limits(1, 1));
    tx.try_send(PanicDropItem)
        .expect("queued panic-on-drop item");
    let result = catch_unwind(AssertUnwindSafe(|| drop(rx)));
    assert!(result.is_err());
    let metrics = tx.metrics();
    assert_eq!(metrics.queued_items, 0);
    assert_eq!(metrics.queued_bytes, 0);
    assert_eq!(metrics.discarded_items, 1);
    assert!(tx.is_closed());
}

#[test]
fn concurrent_blocking_stress_preserves_every_id_and_both_high_water_limits() {
    const PRODUCERS: usize = 6;
    const PER_PRODUCER: usize = 64;
    let total = PRODUCERS * PER_PRODUCER;
    let configured = limits(5, 12);
    let (tx, mut rx) = bounded_mailbox(configured);
    let workers = (0..PRODUCERS)
        .map(|producer| {
            let tx = tx.clone();
            thread::spawn(move || {
                for sequence in 0..PER_PRODUCER {
                    tx.send(Item::new(
                        producer * PER_PRODUCER + sequence,
                        sequence % 3 + 1,
                    ))
                    .expect("stress item admitted");
                }
            })
        })
        .collect::<Vec<_>>();

    let mut seen = vec![false; total];
    let mut next_by_producer = [0; PRODUCERS];
    for _ in 0..total {
        let item = rx.recv().expect("stress item");
        assert!(!seen[item.id], "duplicate item {}", item.id);
        seen[item.id] = true;
        let producer = item.id / PER_PRODUCER;
        let sequence = item.id % PER_PRODUCER;
        assert_eq!(sequence, next_by_producer[producer]);
        next_by_producer[producer] += 1;
        let metrics = rx.metrics();
        assert!(metrics.queued_items <= configured.max_items());
        assert!(metrics.queued_bytes <= configured.max_bytes());
    }
    for worker in workers {
        join_with_watchdog(worker);
    }
    assert!(seen.into_iter().all(|value| value));
    let metrics = tx.metrics();
    assert!(metrics.high_water_items <= configured.max_items());
    assert!(metrics.high_water_bytes <= configured.max_bytes());
    assert_eq!(metrics.enqueued_items, total as u64);
    assert_eq!(metrics.dequeued_items, total as u64);
    drop(tx);
    assert_eq!(rx.recv(), Err(RecvError::Closed));
}
