use std::thread;
use std::time::{Duration, Instant};

use rterm_runtime::testing::{
    ExitAction, ReadAction, ScriptedTransport, VirtualClock, WriteAction,
};
use rterm_runtime::{
    BatchPolicy, EffectSequence, MailboxLimits, PaneNotice, PaneToken, PaneWorkerConfig,
    RuntimeBuffers, RuntimeEffectKind, RuntimeHub, SubmitResult, TerminalRuntime,
    TerminalStateSummary,
};
use rterm_types::PaneId;

const CHUNK_BYTES: usize = 8 * 1024;
const CHUNKS: usize = 8 * 1024;
const TOTAL_BYTES: usize = CHUNK_BYTES * CHUNKS;

fn burst_chunk() -> Vec<u8> {
    let mut chunk = vec![b'\r'; CHUNK_BYTES];
    chunk[0] = b'\x07';
    chunk
}

fn drain_burst_wake(
    hub: &RuntimeHub<VirtualClock>,
    token: PaneToken,
    bell_count: &mut u64,
    sequences: &mut Vec<EffectSequence>,
    last_state: &mut Option<(u64, TerminalStateSummary)>,
) -> u64 {
    let mut turns = 0u64;
    let mut continuation = true;
    while continuation {
        turns = turns.saturating_add(1);
        let drain = hub.drain_pane(token, 256).expect("burst drain");
        if let Some(frame) = drain.frame
            && let Some(previous) = last_state.replace((frame.revision.get(), frame.state))
        {
            assert_eq!(
                TerminalStateSummary::capture_terminal(&frame.snapshot),
                frame.state
            );
            assert!(frame.revision.get() > previous.0);
        }
        for effect in drain.effects {
            sequences.push(effect.effect.sequence());
            match effect.effect.kind() {
                RuntimeEffectKind::Bell { count } => {
                    *bell_count = bell_count.saturating_add(count.get());
                }
                other => panic!("unexpected burst effect {other:?}"),
            }
        }
        continuation = drain.continuation;
    }
    turns
}

#[test]
fn sixty_four_mebibyte_burst_stays_bounded_coalesced_and_lossless() {
    let clock = VirtualClock::new(Instant::now());
    let mut hub = RuntimeHub::new(clock);
    let (transport, driver) = ScriptedTransport::new(
        [ReadAction::Block],
        [WriteAction::Block],
        [ExitAction::Pending],
    );
    let config = PaneWorkerConfig {
        inbox_limits: MailboxLimits::try_new(512, 4 * 1024 * 1024).expect("inbox limits"),
        effect_limits: MailboxLimits::try_new(1024, 1024 * 1024).expect("effect limits"),
        batch_policy: BatchPolicy::try_new(16 * CHUNK_BYTES, 16, Duration::from_millis(3))
            .expect("batch policy"),
        ..PaneWorkerConfig::default()
    };
    let handle = hub
        .open(PaneId::new(64), transport, config)
        .expect("open burst pane");
    let token = handle.token();
    assert_eq!(hub.recv_notice().expect("ready"), PaneNotice::Ready(token));
    assert_eq!(
        handle.submit_input(b"gate".to_vec()),
        SubmitResult::Accepted
    );
    driver.wait_until_writer_blocked();

    let chunk = burst_chunk();
    let mut script = (0..CHUNKS)
        .map(|_| ReadAction::Bytes(chunk.clone()))
        .collect::<Vec<_>>();
    script.push(ReadAction::Eof);
    driver.push_reads(script);
    for _ in 0..100_000 {
        if handle.inbox_metrics().queued_items >= 128 {
            break;
        }
        thread::yield_now();
    }
    assert!(handle.inbox_metrics().queued_items >= 128);
    driver.push_write(WriteAction::accept(usize::MAX));

    let mut wakes = 0u64;
    let mut bell_count = 0u64;
    let mut sequences = Vec::new();
    let mut last_state = None;
    loop {
        match hub.recv_notice().expect("burst notice") {
            PaneNotice::Ready(_) => panic!("ready must be emitted once"),
            PaneNotice::Wake(pane) => {
                assert_eq!(pane, token);
                wakes = wakes.saturating_add(drain_burst_wake(
                    &hub,
                    token,
                    &mut bell_count,
                    &mut sequences,
                    &mut last_state,
                ));
            }
            PaneNotice::Closed { pane, .. } => {
                assert_eq!(pane, token);
                break;
            }
            PaneNotice::InputWriteCompleted { .. } | PaneNotice::FirstPtyByte { .. } => {}
        }
    }

    let metrics = hub.publication_metrics(token).expect("completed metrics");
    assert_eq!(metrics.source_bytes, TOTAL_BYTES as u64);
    assert_eq!(metrics.source_items, CHUNKS as u64);
    assert!(metrics.max_batch_items <= 16);
    assert!(metrics.max_batch_bytes <= 16 * CHUNK_BYTES);
    assert!(metrics.source_items / wakes >= 16);
    assert!(metrics.latest.wakes > 0);
    assert!(metrics.source_items / metrics.latest.wakes >= 16);
    assert_eq!(metrics.latest.publications, metrics.batches);
    assert_eq!(bell_count, CHUNKS as u64);
    assert!(sequences.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(metrics.effects.high_water_items <= metrics.effects.limits.max_items());
    assert!(metrics.effects.high_water_bytes <= metrics.effects.limits.max_bytes());
    assert_eq!(metrics.effects.queued_items, 0);
    assert_eq!(metrics.effects.queued_bytes, 0);
    let inbox = handle.inbox_metrics();
    assert!(inbox.high_water_items <= inbox.limits.max_items());
    assert!(inbox.high_water_bytes <= inbox.limits.max_bytes());

    let mut reference = TerminalRuntime::new(config.size);
    let mut reference_buffers = RuntimeBuffers::with_capacity(CHUNK_BYTES);
    for _ in 0..CHUNKS {
        let _ = reference.feed_into(&chunk, &mut reference_buffers);
    }
    let expected = TerminalStateSummary::capture(&reference);
    let actual = last_state.expect("latest state").1;
    assert_eq!(actual.size, expected.size);
    assert_eq!(actual.cursor, expected.cursor);
    assert_eq!(actual.scrollback_rows, expected.scrollback_rows);
    assert_eq!(actual.visible_digest, expected.visible_digest);

    hub.shutdown();
    assert_eq!(hub.live_thread_count(), 0);
}
