use std::{
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};

use rterm_runtime::testing::{
    ExitAction, ReadAction, ScriptedTransport, VirtualClock, WriteAction,
};
use rterm_runtime::{
    BatchPolicy, MailboxLimits, PaneNotice, PaneWorkerConfig, RuntimeEffectKind, RuntimeHub,
    SessionExit, SubmitResult, TerminalRuntime,
};
use rterm_types::PaneId;
use rterm_types::TerminalSize;

fn scripted_session(
    reads: impl IntoIterator<Item = ReadAction>,
    writes: impl IntoIterator<Item = WriteAction>,
) -> (
    ScriptedTransport,
    rterm_runtime::testing::ScriptedSessionDriver,
) {
    ScriptedTransport::new(reads, writes, [ExitAction::Pending])
}

fn recv_matching_notice(
    hub: &mut RuntimeHub<VirtualClock>,
    mut predicate: impl FnMut(&PaneNotice) -> bool,
) -> PaneNotice {
    loop {
        let notice = hub.recv_notice().expect("pane notice");
        if predicate(&notice) {
            return notice;
        }
    }
}

fn recv_ready(hub: &mut RuntimeHub<VirtualClock>, token: rterm_runtime::PaneToken) {
    assert_eq!(
        recv_matching_notice(
            hub,
            |notice| matches!(notice, PaneNotice::Ready(pane) if *pane == token)
        ),
        PaneNotice::Ready(token)
    );
}

fn recv_wake(hub: &mut RuntimeHub<VirtualClock>, token: rterm_runtime::PaneToken) {
    assert_eq!(
        recv_matching_notice(
            hub,
            |notice| matches!(notice, PaneNotice::Wake(pane) if *pane == token)
        ),
        PaneNotice::Wake(token)
    );
}

fn wait_for_wakes(wakes: &(Mutex<usize>, Condvar), expected: usize) {
    let (count, changed) = wakes;
    let observed = count.lock().expect("wake count lock");
    let (observed, timeout) = changed
        .wait_timeout_while(observed, Duration::from_secs(1), |count| *count < expected)
        .expect("wake count wait");
    assert!(
        !timeout.timed_out(),
        "observed {observed} of {expected} wakes"
    );
    assert_eq!(*observed, expected);
}

#[test]
fn pane_notices_invoke_the_host_waker_without_idle_polling() {
    let wakes = Arc::new((Mutex::new(0_usize), Condvar::new()));
    let wake_counter = Arc::clone(&wakes);
    let clock = VirtualClock::new(Instant::now());
    let mut hub = RuntimeHub::new_with_notice_waker(
        clock,
        Arc::new(move || {
            let (count, changed) = &*wake_counter;
            *count.lock().expect("wake count lock") += 1;
            changed.notify_all();
        }),
    );
    let (transport, driver) = scripted_session(
        [ReadAction::Block, ReadAction::Block],
        [WriteAction::accept(usize::MAX)],
    );
    let handle = hub
        .open(PaneId::new(6), transport, PaneWorkerConfig::default())
        .expect("open pane");
    let token = handle.token();

    recv_ready(&mut hub, token);
    wait_for_wakes(&wakes, 1);
    driver.push_read(ReadAction::bytes(b"wake"));
    recv_wake(&mut hub, token);
    wait_for_wakes(&wakes, 3);
    hub.shutdown();
}

#[test]
fn fair_drain_does_not_let_a_hot_pane_starve_a_ready_quiet_pane() {
    let clock = VirtualClock::new(Instant::now());
    let mut hub = RuntimeHub::new(clock);
    let (hot_transport, hot_driver) = scripted_session(
        [ReadAction::Block, ReadAction::Block, ReadAction::Block],
        [WriteAction::accept(usize::MAX)],
    );
    let (quiet_transport, quiet_driver) = scripted_session(
        [ReadAction::Block, ReadAction::Block],
        [WriteAction::accept(usize::MAX)],
    );
    let hot = hub
        .open(PaneId::new(61), hot_transport, PaneWorkerConfig::default())
        .expect("open hot pane");
    let quiet = hub
        .open(
            PaneId::new(62),
            quiet_transport,
            PaneWorkerConfig::default(),
        )
        .expect("open quiet pane");
    assert!(matches!(hub.recv_notice(), Ok(PaneNotice::Ready(_))));
    assert!(matches!(hub.recv_notice(), Ok(PaneNotice::Ready(_))));

    hot_driver.push_read(ReadAction::bytes(b"hot-1"));
    quiet_driver.push_read(ReadAction::bytes(b"quiet"));
    let mut wakes = Vec::new();
    while wakes.len() < 2 {
        if let PaneNotice::Wake(token) = hub.recv_notice().expect("publication notice") {
            wakes.push(token);
        }
    }
    assert!(wakes.contains(&hot.token()));
    assert!(wakes.contains(&quiet.token()));

    let first = hub.drain_ready_fair(1, usize::MAX);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].0, hot.token());

    hot_driver.push_read(ReadAction::bytes(b"hot-2"));
    assert_eq!(hub.recv_notice(), Ok(PaneNotice::Wake(hot.token())));
    let second = hub.drain_ready_fair(1, usize::MAX);
    assert_eq!(second.len(), 1);
    assert_eq!(
        second[0].0,
        quiet.token(),
        "the hot pane must not jump ahead of already-ready quiet work"
    );
    hub.shutdown();
}

#[test]
fn worker_is_ready_and_serializes_user_input_terminal_replies_and_resize() {
    let clock = VirtualClock::new(Instant::now());
    let mut hub = RuntimeHub::new(clock);
    let (transport, driver) = scripted_session(
        [ReadAction::Block, ReadAction::Block],
        [WriteAction::accept(usize::MAX); 4],
    );
    let handle = hub
        .open(PaneId::new(7), transport, PaneWorkerConfig::default())
        .expect("open pane");
    let token = handle.token();

    recv_ready(&mut hub, token);
    assert_eq!(
        handle.submit_input(b"user".to_vec()),
        SubmitResult::Accepted
    );
    driver.wait_until_accepted_write_len(4);
    driver.push_read(ReadAction::bytes(b"\x1b[5n"));
    driver.wait_until_accepted_write_len(8);
    assert_eq!(driver.accepted_writes(), b"user\x1b[0n");
    recv_wake(&mut hub, token);
    let output = hub.drain_pane(token, usize::MAX).expect("output drain");
    assert!(!output.frame.expect("output frame").snapshot_changed);

    let size = TerminalSize::new(132, 43);
    assert_eq!(handle.resize(size), SubmitResult::Accepted);
    driver.wait_until_control_call_count(1);
    assert_eq!(driver.control_log().resizes, vec![size]);
    recv_wake(&mut hub, token);
    let resize = hub.drain_pane(token, usize::MAX).expect("resize drain");
    assert!(resize.frame.expect("resize frame").snapshot_changed);

    assert!(hub.begin_close(token, Duration::from_secs(1)));
    let closed = hub.recv_notice().expect("closed notice");
    assert!(
        matches!(
            closed,
            PaneNotice::Closed { pane, .. } if pane == token
        ),
        "unexpected close notice: {closed:?}"
    );
    hub.shutdown();
    assert_eq!(hub.live_thread_count(), 0);
}

#[test]
fn worker_starts_from_the_configured_terminal_runtime() {
    let clock = VirtualClock::new(Instant::now());
    let mut hub = RuntimeHub::new(clock);
    let (transport, driver) = scripted_session(
        [ReadAction::Block, ReadAction::Block],
        [WriteAction::accept(usize::MAX)],
    );
    let config = PaneWorkerConfig::default();
    let mut runtime = TerminalRuntime::new(config.size);
    runtime.set_enq_answerback("configured-worker");
    let handle = hub
        .open_with_runtime(PaneId::new(8), transport, config, runtime)
        .expect("open configured pane");
    let token = handle.token();
    recv_ready(&mut hub, token);

    driver.push_read(ReadAction::bytes(b"\x05"));
    driver.wait_until_accepted_write_len("configured-worker".len());
    assert_eq!(driver.accepted_writes(), b"configured-worker");
    hub.shutdown();
}

#[test]
fn explicitly_enabled_host_stream_is_published_losslessly_and_in_order() {
    let clock = VirtualClock::new(Instant::now());
    let mut hub = RuntimeHub::new(clock);
    let (transport, driver) = scripted_session(
        [ReadAction::Block, ReadAction::Block],
        [WriteAction::accept(usize::MAX)],
    );
    let config = PaneWorkerConfig {
        capture_host_stream: true,
        ..PaneWorkerConfig::default()
    };
    let handle = hub
        .open(PaneId::new(9), transport, config)
        .expect("open capture pane");
    let token = handle.token();
    recv_ready(&mut hub, token);

    driver.push_read(ReadAction::bytes(b"\x1b[?1hhost-stream-marker"));
    recv_wake(&mut hub, token);
    let drain = hub.drain_pane(token, usize::MAX).expect("drain");
    let streams = drain
        .effects
        .iter()
        .filter_map(|effect| match effect.effect.kind() {
            RuntimeEffectKind::HostStream(bytes) => Some(bytes.as_slice()),
            _ => None,
        })
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    assert!(drain.effects.iter().any(|effect| matches!(
        effect.effect.kind(),
        RuntimeEffectKind::ModeChange(rterm_runtime::TerminalModeChange::ApplicationCursorKeys(
            true
        ))
    )));

    assert_eq!(streams, b"\x1b[?1hhost-stream-marker");
    hub.shutdown();
}

#[test]
fn explicitly_enabled_visible_output_is_published_without_terminal_controls() {
    let clock = VirtualClock::new(Instant::now());
    let mut hub = RuntimeHub::new(clock);
    let (transport, driver) = scripted_session(
        [ReadAction::Block, ReadAction::Block],
        [WriteAction::accept(usize::MAX)],
    );
    let config = PaneWorkerConfig {
        capture_visible_output: true,
        ..PaneWorkerConfig::default()
    };
    let handle = hub
        .open(PaneId::new(10), transport, config)
        .expect("open visible-output pane");
    let token = handle.token();
    recv_ready(&mut hub, token);

    driver.push_read(ReadAction::bytes(b"A\x1b]0;hidden-title\x07B"));
    recv_wake(&mut hub, token);
    let drain = hub.drain_pane(token, usize::MAX).expect("drain");
    let visible = drain
        .effects
        .iter()
        .filter_map(|effect| match effect.effect.kind() {
            RuntimeEffectKind::VisibleOutput(bytes) => Some(bytes.as_slice()),
            _ => None,
        })
        .flatten()
        .copied()
        .collect::<Vec<_>>();

    assert_eq!(visible, b"AB");
    assert!(
        !drain
            .effects
            .iter()
            .any(|effect| matches!(effect.effect.kind(), RuntimeEffectKind::HostStream(_)))
    );
    hub.shutdown();
}

#[test]
fn restart_issues_a_new_generation_rejects_old_handle_and_filters_stale_notices() {
    let clock = VirtualClock::new(Instant::now());
    let mut hub = RuntimeHub::new(clock);
    let (first_transport, _) =
        scripted_session([ReadAction::Block], [WriteAction::accept(usize::MAX); 2]);
    let first = hub
        .open(
            PaneId::new(11),
            first_transport,
            PaneWorkerConfig::default(),
        )
        .expect("first generation");
    assert_eq!(
        hub.recv_notice().expect("first ready"),
        PaneNotice::Ready(first.token())
    );

    let (second_transport, _) =
        scripted_session([ReadAction::Block], [WriteAction::accept(usize::MAX); 2]);
    let second = hub
        .restart(
            PaneId::new(11),
            second_transport,
            PaneWorkerConfig::default(),
        )
        .expect("replacement generation");

    assert_ne!(first.token().generation(), second.token().generation());
    assert_eq!(first.submit_input(b"stale".to_vec()), SubmitResult::Closed);
    assert_eq!(
        hub.recv_notice().expect("replacement ready"),
        PaneNotice::Ready(second.token())
    );
    hub.shutdown();
    assert_eq!(hub.live_thread_count(), 0);
}

#[test]
fn restart_with_runtime_installs_the_configured_replacement_generation() {
    let clock = VirtualClock::new(Instant::now());
    let mut hub = RuntimeHub::new(clock);
    let (first_transport, _) =
        scripted_session([ReadAction::Block], [WriteAction::accept(usize::MAX)]);
    let first = hub
        .open(
            PaneId::new(12),
            first_transport,
            PaneWorkerConfig::default(),
        )
        .expect("first generation");
    assert_eq!(
        hub.recv_notice().expect("first ready"),
        PaneNotice::Ready(first.token())
    );

    let (second_transport, second_driver) = scripted_session(
        [ReadAction::Block, ReadAction::Block],
        [WriteAction::accept(usize::MAX)],
    );
    let config = PaneWorkerConfig::default();
    let mut runtime = TerminalRuntime::new(config.size);
    runtime.set_enq_answerback("restarted-runtime");
    let second = hub
        .restart_with_runtime(PaneId::new(12), second_transport, config, runtime)
        .expect("configured replacement generation");

    assert_ne!(first.token().generation(), second.token().generation());
    assert_eq!(first.submit_input(b"stale".to_vec()), SubmitResult::Closed);
    assert_eq!(
        hub.recv_notice().expect("replacement ready"),
        PaneNotice::Ready(second.token())
    );
    second_driver.push_read(ReadAction::bytes(b"\x05"));
    second_driver.wait_until_accepted_write_len("restarted-runtime".len());
    assert_eq!(second_driver.accepted_writes(), b"restarted-runtime");
    hub.shutdown();
}

#[test]
fn expired_close_interrupts_blocked_writer_and_hands_join_to_reaper() {
    let clock = VirtualClock::new(Instant::now());
    let mut hub = RuntimeHub::new(clock.clone());
    let (transport, driver) = scripted_session([ReadAction::Block], [WriteAction::Block]);
    let handle = hub
        .open(PaneId::new(19), transport, PaneWorkerConfig::default())
        .expect("open blocked pane");
    let token = handle.token();
    recv_ready(&mut hub, token);
    assert_eq!(
        handle.submit_input(b"blocked".to_vec()),
        SubmitResult::Accepted
    );
    driver.wait_until_writer_blocked();

    assert!(hub.begin_close(token, Duration::from_millis(10)));
    clock.advance(Duration::from_millis(9)).expect("advance");
    assert_eq!(hub.reap_expired(), 0);
    clock.advance(Duration::from_millis(1)).expect("deadline");
    assert_eq!(hub.reap_expired(), 1);
    assert!(driver.interrupt_calls() >= 1);

    hub.shutdown();
    assert_eq!(hub.live_thread_count(), 0);
    assert_eq!(driver.interrupt_calls(), 2);
    assert!(driver.accepted_writes().is_empty());
}

#[test]
fn close_publishes_synchronized_final_damage_and_effects_before_closed() {
    let clock = VirtualClock::new(Instant::now());
    let mut hub = RuntimeHub::new(clock);
    let (transport, driver) = scripted_session(
        [ReadAction::Block, ReadAction::Block],
        [WriteAction::accept(usize::MAX); 2],
    );
    let handle = hub
        .open(PaneId::new(23), transport, PaneWorkerConfig::default())
        .expect("open pane");
    let token = handle.token();
    assert_eq!(hub.recv_notice().expect("ready"), PaneNotice::Ready(token));

    driver.push_read(ReadAction::bytes(b"\x1b[?2026hheld\x07"));
    recv_wake(&mut hub, token);
    let held = hub.drain_pane(token, usize::MAX).expect("held drain");
    assert!(!held.frame.expect("held frame").snapshot_changed);
    assert!(matches!(
        held.effects.as_slice(),
        [effect] if matches!(effect.effect.kind(), RuntimeEffectKind::Bell { .. })
    ));

    assert!(hub.begin_close(token, Duration::from_secs(1)));
    recv_wake(&mut hub, token);
    let final_drain = hub.drain_pane(token, usize::MAX).expect("final drain");
    let final_frame = final_drain.frame.expect("final frame");
    assert!(final_frame.snapshot_changed);
    assert!(!final_frame.damage.is_empty());
    assert!(matches!(
        hub.recv_notice().expect("closed"),
        PaneNotice::Closed { pane, .. } if pane == token
    ));
    hub.shutdown();
    assert_eq!(hub.live_thread_count(), 0);
}

#[test]
fn observed_exit_drains_reader_output_before_closed_notice() {
    let clock = VirtualClock::new(Instant::now());
    let mut hub = RuntimeHub::new(clock);
    let exit = SessionExit {
        status: Some(0),
        signal: None,
    };
    let (transport, driver) = ScriptedTransport::new(
        [ReadAction::Block],
        [WriteAction::accept(usize::MAX)],
        [ExitAction::Exited(exit.clone())],
    );
    let config = PaneWorkerConfig {
        capture_host_stream: true,
        ..PaneWorkerConfig::default()
    };
    let handle = hub
        .open(PaneId::new(24), transport, config)
        .expect("open pane");
    let token = handle.token();
    assert_eq!(hub.recv_notice().expect("ready"), PaneNotice::Ready(token));
    driver.wait_until_reader_blocked();
    driver.wait_until_control_call_count(2);

    let marker = b"output-published-after-exit-observed";
    driver.push_reads([ReadAction::bytes(marker), ReadAction::Eof]);

    assert!(matches!(
        hub.recv_notice().expect("first byte before close"),
        PaneNotice::FirstPtyByte { pane, .. } if pane == token
    ));
    recv_wake(&mut hub, token);
    let drain = hub
        .drain_pane(token, usize::MAX)
        .expect("final output drain");
    let host_stream = drain
        .effects
        .iter()
        .filter_map(|effect| match effect.effect.kind() {
            RuntimeEffectKind::HostStream(bytes) => Some(bytes.as_slice()),
            _ => None,
        })
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(host_stream, marker);
    assert!(drain.frame.expect("final output frame").snapshot_changed);
    assert_eq!(
        hub.recv_notice().expect("closed after final output"),
        PaneNotice::Closed {
            pane: token,
            exit: Some(exit),
        }
    );
    hub.shutdown();
}

#[test]
fn queued_output_batches_once_wakes_once_replaces_frames_and_preserves_effects() {
    let clock = VirtualClock::new(Instant::now());
    let mut hub = RuntimeHub::new(clock);
    let mut reads = vec![ReadAction::bytes(b"\x07"); 32];
    reads.push(ReadAction::Block);
    let (transport, driver) = scripted_session(
        [ReadAction::Block],
        [WriteAction::Block, WriteAction::accept(usize::MAX)],
    );
    let config = PaneWorkerConfig {
        batch_policy: BatchPolicy::try_new(16, 16, Duration::from_millis(3)).expect("batch policy"),
        effect_limits: MailboxLimits::try_new(64, 4096).expect("effect limits"),
        ..PaneWorkerConfig::default()
    };
    let handle = hub
        .open(PaneId::new(29), transport, config)
        .expect("open pane");
    let token = handle.token();
    recv_ready(&mut hub, token);

    assert_eq!(
        handle.submit_input(b"gate".to_vec()),
        SubmitResult::Accepted
    );
    driver.wait_until_writer_blocked();
    driver.push_reads(reads);
    driver.wait_until_reader_blocked();
    assert_eq!(
        handle.resize(TerminalSize::new(81, 24)),
        SubmitResult::Accepted
    );
    driver.push_write(WriteAction::accept(usize::MAX));
    driver.wait_until_control_call_count(1);
    assert_eq!(
        handle.submit_input(b"barrier".to_vec()),
        SubmitResult::Accepted
    );
    driver.wait_until_accepted_write_len(11);

    recv_wake(&mut hub, token);
    while let Ok(notice) = hub.try_recv_notice() {
        assert_ne!(
            notice,
            PaneNotice::Wake(token),
            "publication wake duplicated"
        );
    }
    let drain = hub.drain_pane(token, usize::MAX).expect("drain");
    let frame = drain.frame.expect("latest frame");
    assert_eq!(frame.revision.get(), 3);
    assert!(frame.full_repaint);
    assert_eq!(frame.snapshot.grid().size(), TerminalSize::new(81, 24));
    assert_eq!(
        rterm_runtime::TerminalStateSummary::capture_terminal(&frame.snapshot),
        frame.state
    );
    assert_eq!(
        drain
            .effects
            .iter()
            .map(|effect| match effect.effect.kind() {
                RuntimeEffectKind::Bell { count } => count.get(),
                other => panic!("unexpected effect {other:?}"),
            })
            .sum::<u64>(),
        32
    );
    assert!(
        drain
            .effects
            .windows(2)
            .all(|pair| pair[0].effect.sequence() < pair[1].effect.sequence())
    );

    let metrics = hub.publication_metrics(token).expect("metrics");
    assert_eq!(metrics.batches, 3);
    assert_eq!(metrics.source_items, 32);
    assert_eq!(metrics.max_batch_items, 16);
    assert_eq!(metrics.latest.wakes, 1);
    assert_eq!(metrics.latest.replaced_frames, 2);
    hub.shutdown();
}

#[test]
fn effect_backpressure_wakes_before_the_worker_waits_for_mailbox_space() {
    let clock = VirtualClock::new(Instant::now());
    let mut hub = RuntimeHub::new(clock);
    let payload =
        b"\x1b]777;notify;test;one\x07\x1b]777;notify;test;two\x07\x1b]777;notify;test;three\x07";
    let (transport, _) = scripted_session(
        [ReadAction::bytes(payload), ReadAction::Block],
        [WriteAction::accept(usize::MAX)],
    );
    let config = PaneWorkerConfig {
        effect_limits: MailboxLimits::try_new(1, 512).expect("single-effect limit"),
        ..PaneWorkerConfig::default()
    };
    let handle = hub
        .open(PaneId::new(31), transport, config)
        .expect("open pane");
    let token = handle.token();
    recv_ready(&mut hub, token);

    let mut bodies = Vec::new();
    while bodies.len() < 3 {
        recv_wake(&mut hub, token);
        let mut continuation = true;
        while continuation {
            let drain = hub.drain_pane(token, 1).expect("single effect drain");
            for effect in drain.effects {
                match effect.effect.kind() {
                    RuntimeEffectKind::Notification { body, .. } => bodies.push(body.clone()),
                    other => panic!("unexpected effect {other:?}"),
                }
            }
            continuation = drain.continuation;
        }
    }
    assert_eq!(bodies, ["one", "two", "three"]);
    assert_eq!(
        hub.publication_metrics(token)
            .expect("publication metrics")
            .effects
            .high_water_items,
        1
    );
    hub.shutdown();
}
