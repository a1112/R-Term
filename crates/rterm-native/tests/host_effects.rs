use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use rterm_config::EffectiveConfig;
use rterm_domain::{PaneId, WindowId};
use rterm_native::{
    ClipboardEffect, CommandIntent, ConfigDiff, HostEffectContext, HostError, HostPorts,
    NotificationEffect, PaneLifecycleIntent, PlatformEvent, PortError, PortErrorKind, PortKind,
    RendererEffect, RuntimeDrain, RuntimePortEffect, SpawnEffect, TimerId, TimerIntent, TurnBudget,
    UriEffect, WindowEffect, WindowIntent, WindowPortEffect, WindowState, WinitHost,
};
use rterm_runtime::{
    Clock, EffectSequence, PaneToken, PaneTokenAllocator, RuntimeBatch, RuntimeBatchMetrics,
    RuntimeRevision, TerminalStateSummary,
};
use rterm_types::{DamageRegion, TerminalSize};

#[derive(Debug, Clone)]
struct TestClock(Arc<Mutex<Instant>>);

impl Default for TestClock {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Instant::now())))
    }
}

impl TestClock {
    fn advance(&self, duration: Duration) {
        let mut now = self.0.lock().expect("clock lock");
        *now = now.checked_add(duration).unwrap_or(*now);
    }
}

impl Clock for TestClock {
    fn now(&self) -> Instant {
        *self.0.lock().expect("clock lock")
    }
}

fn token(pane: u64) -> PaneToken {
    PaneTokenAllocator::new()
        .issue(PaneId::new(pane))
        .expect("pane token")
}

#[derive(Debug, Default)]
struct FakePorts {
    calls: Vec<String>,
    drains: VecDeque<RuntimeDrain>,
    fail: Option<(PortKind, PortErrorKind)>,
    clock: Option<TestClock>,
}

impl FakePorts {
    fn record(&mut self, port: PortKind, call: String) -> Result<(), PortError> {
        self.calls.push(call);
        if let Some(clock) = &self.clock {
            clock.advance(Duration::from_millis(10));
        }
        if self.fail == Some((port, PortErrorKind::Backpressure)) {
            return Err(PortError::new(
                PortErrorKind::Backpressure,
                "bounded destination is full",
            ));
        }
        if self.fail == Some((port, PortErrorKind::Unavailable)) {
            return Err(PortError::new(
                PortErrorKind::Unavailable,
                "port unavailable",
            ));
        }
        Ok(())
    }
}

impl HostPorts for FakePorts {
    fn runtime(&mut self, effect: &RuntimePortEffect) -> Result<(), PortError> {
        self.record(PortKind::Runtime, format!("runtime:{effect:?}"))
    }

    fn window(&mut self, effect: &WindowPortEffect) -> Result<(), PortError> {
        self.record(PortKind::Window, format!("window:{effect:?}"))
    }

    fn renderer(&mut self, effect: &RendererEffect) -> Result<(), PortError> {
        self.record(PortKind::Renderer, format!("renderer:{effect:?}"))
    }

    fn clipboard(&mut self, effect: &ClipboardEffect) -> Result<(), PortError> {
        self.record(PortKind::Clipboard, format!("clipboard:{effect:?}"))
    }

    fn uri(&mut self, effect: &UriEffect) -> Result<(), PortError> {
        self.record(PortKind::Uri, format!("uri:{effect:?}"))
    }

    fn notification(&mut self, effect: &NotificationEffect) -> Result<(), PortError> {
        self.record(PortKind::Notification, format!("notification:{effect:?}"))
    }

    fn persistence(&mut self, effect: &rterm_native::PersistenceEffect) -> Result<(), PortError> {
        self.record(PortKind::Persistence, format!("persistence:{effect:?}"))
    }

    fn spawn(&mut self, effect: &SpawnEffect) -> Result<(), PortError> {
        self.record(PortKind::Spawn, format!("spawn:{effect:?}"))
    }

    fn drain_runtime(
        &mut self,
        pane: PaneToken,
        max_batches: usize,
    ) -> Result<RuntimeDrain, PortError> {
        self.calls.push(format!("drain:{pane:?}:max={max_batches}"));
        Ok(self.drains.pop_front().unwrap_or_default())
    }

    fn schedule_runtime_continuation(
        &mut self,
        window: WindowId,
        pane: PaneToken,
    ) -> Result<(), PortError> {
        self.record(
            PortKind::Platform,
            format!("continue:window={window:?}:pane={pane:?}"),
        )
    }
}

#[test]
fn host_routes_every_typed_effect_in_exact_order_including_recovery_and_new_window() {
    let pane = token(7);
    let context = HostEffectContext {
        pane,
        revision: RuntimeRevision::FIRST,
        sequence: EffectSequence::FIRST,
    };
    let effects = vec![
        WindowEffect::Window(WindowPortEffect::SetFocused(true)),
        WindowEffect::Renderer(RendererEffect::ResizeSurface(TerminalSize::new(100, 30))),
        WindowEffect::Runtime(RuntimePortEffect::SubmitInput {
            pane,
            bytes: b"input".to_vec(),
        }),
        WindowEffect::Clipboard(ClipboardEffect::Read {
            context,
            selection: "c".to_owned(),
        }),
        WindowEffect::Uri(UriEffect::Open("https://example.test".to_owned())),
        WindowEffect::Notification(NotificationEffect::Show {
            context,
            title: Some("title".to_owned()),
            body: "body".to_owned(),
        }),
        WindowEffect::Persistence(rterm_native::PersistenceEffect::Save),
        WindowEffect::Renderer(RendererEffect::RecoverDevice),
        WindowEffect::Spawn(SpawnEffect::Window),
        WindowEffect::Window(WindowPortEffect::CloseNow),
    ];
    let mut host = WinitHost::new(
        WindowId::new(3),
        WindowState::default(),
        FakePorts::default(),
        TurnBudget::new(4, Duration::from_millis(5)),
    );

    let turn = host.execute_effects(&effects).expect("effects");
    assert_eq!(turn.effects_executed, effects.len());
    assert_eq!(host.ports().calls.len(), effects.len());
    assert!(host.ports().calls[0].starts_with("window:"));
    assert!(host.ports().calls[1].starts_with("renderer:"));
    assert!(host.ports().calls[2].starts_with("runtime:"));
    assert!(host.ports().calls[8].contains("Window"));
}

#[test]
fn typed_port_failure_reports_index_kind_and_stops_later_effects() {
    let ports = FakePorts {
        fail: Some((PortKind::Runtime, PortErrorKind::Backpressure)),
        ..FakePorts::default()
    };
    let mut host = WinitHost::new(
        WindowId::new(5),
        WindowState::default(),
        ports,
        TurnBudget::new(1, Duration::from_millis(5)),
    );
    let pane = token(5);
    let effects = [
        WindowEffect::Window(WindowPortEffect::RequestRedraw),
        WindowEffect::Runtime(RuntimePortEffect::SubmitInput {
            pane,
            bytes: vec![1],
        }),
        WindowEffect::Window(WindowPortEffect::CloseNow),
    ];

    let error = host.execute_effects(&effects).expect_err("backpressure");
    assert_eq!(
        error,
        HostError::new(
            1,
            PortKind::Runtime,
            PortError::new(PortErrorKind::Backpressure, "bounded destination is full")
        )
    );
    assert_eq!(host.ports().calls.len(), 2);
}

#[test]
fn platform_deadline_redraw_close_and_commands_flow_through_the_reducer() {
    let pane = token(13);
    let mut state = WindowState::default();
    state
        .panes
        .insert(pane.pane(), rterm_native::PaneState::new(pane));
    let mut host = WinitHost::new(
        WindowId::new(9),
        state,
        FakePorts::default(),
        TurnBudget::new(2, Duration::from_millis(5)),
    );

    host.handle(PlatformEvent::Focused(true)).expect("focus");
    host.handle(PlatformEvent::Resized(TerminalSize::new(132, 43)))
        .expect("resize");
    host.handle(PlatformEvent::Command(CommandIntent::SetTitle(
        "native".to_owned(),
    )))
    .expect("title");
    host.handle(PlatformEvent::Command(CommandIntent::SpawnWindow))
        .expect("new window");
    host.handle(PlatformEvent::Timer(TimerIntent::Arm {
        timer: TimerId::new(4),
        epoch: 8,
    }))
    .expect("arm");
    host.handle(PlatformEvent::Timer(TimerIntent::Fired {
        timer: TimerId::new(4),
        epoch: 8,
    }))
    .expect("deadline");
    host.handle(PlatformEvent::RedrawRequested).expect("redraw");
    host.handle(PlatformEvent::CloseRequested).expect("close");

    assert!(host.state().platform.focused);
    assert!(host.state().lifecycle.closing);
    assert!(
        host.ports()
            .calls
            .iter()
            .any(|call| call.contains("ResizeSurface"))
    );
    assert!(
        host.ports()
            .calls
            .iter()
            .any(|call| call.contains("Present"))
    );
    assert!(
        host.ports()
            .calls
            .iter()
            .any(|call| call.contains("BeginClose"))
    );
    assert!(host.ports().calls.iter().any(|call| call == "spawn:Window"));
}

#[test]
fn runtime_wake_obeys_turn_budget_and_schedules_only_one_continuation() {
    let pane = token(21);
    let mut ports = FakePorts::default();
    ports.drains.push_back(RuntimeDrain {
        intents: vec![
            WindowIntent::Config(ConfigDiff::new(
                1,
                Arc::new(EffectiveConfig::default()),
                Some("one".to_owned()),
            )),
            WindowIntent::Config(ConfigDiff::new(
                2,
                Arc::new(EffectiveConfig::default()),
                Some("two".to_owned()),
            )),
        ],
        continuation: true,
    });
    ports.drains.push_back(RuntimeDrain {
        intents: vec![WindowIntent::Config(ConfigDiff::new(
            3,
            Arc::new(EffectiveConfig::default()),
            Some("three".to_owned()),
        ))],
        continuation: false,
    });
    let mut state = WindowState::default();
    state
        .panes
        .insert(pane.pane(), rterm_native::PaneState::new(pane));
    let mut host = WinitHost::new(
        WindowId::new(12),
        state,
        ports,
        TurnBudget::new(2, Duration::from_millis(5)),
    );

    let first = host
        .handle(PlatformEvent::RuntimeWake { pane })
        .expect("first wake");
    assert_eq!(first.runtime_intents, 2);
    assert!(first.continuation_scheduled);
    assert_eq!(host.state().config.revision, 2);
    let call_count = host.ports().calls.len();

    let duplicate = host
        .handle(PlatformEvent::RuntimeWake { pane })
        .expect("duplicate wake");
    assert_eq!(duplicate.runtime_intents, 0);
    assert_eq!(host.ports().calls.len(), call_count);

    let continuation = host
        .handle(PlatformEvent::RuntimeContinuation { pane })
        .expect("continuation");
    assert_eq!(continuation.runtime_intents, 1);
    assert!(!continuation.continuation_scheduled);
    assert_eq!(host.state().config.revision, 3);
    assert_eq!(
        host.ports()
            .calls
            .iter()
            .filter(|call| call.starts_with("continue:"))
            .count(),
        1
    );
}

#[test]
fn rejected_continuation_wake_rolls_back_pending_state_for_retry() {
    let pane = token(25);
    let mut ports = FakePorts {
        fail: Some((PortKind::Platform, PortErrorKind::Backpressure)),
        ..FakePorts::default()
    };
    ports.drains.push_back(RuntimeDrain {
        intents: Vec::new(),
        continuation: true,
    });
    ports.drains.push_back(RuntimeDrain {
        intents: Vec::new(),
        continuation: true,
    });
    let mut state = WindowState::default();
    state
        .panes
        .insert(pane.pane(), rterm_native::PaneState::new(pane));
    let mut host = WinitHost::new(
        WindowId::new(13),
        state,
        ports,
        TurnBudget::new(1, Duration::from_millis(5)),
    );

    assert!(host.handle(PlatformEvent::RuntimeWake { pane }).is_err());
    assert!(host.handle(PlatformEvent::RuntimeWake { pane }).is_err());
    assert_eq!(
        host.ports()
            .calls
            .iter()
            .filter(|call| call.starts_with("continue:"))
            .count(),
        2
    );
}

#[test]
fn elapsed_turn_budget_retains_unprocessed_intents_for_one_continuation() {
    let pane = token(29);
    let clock = TestClock::default();
    let mut ports = FakePorts {
        clock: Some(clock.clone()),
        ..FakePorts::default()
    };
    ports.drains.push_back(RuntimeDrain {
        intents: vec![
            WindowIntent::Config(ConfigDiff::new(
                1,
                Arc::new(EffectiveConfig::default()),
                Some("first".to_owned()),
            )),
            WindowIntent::Config(ConfigDiff::new(
                2,
                Arc::new(EffectiveConfig::default()),
                Some("second".to_owned()),
            )),
        ],
        continuation: false,
    });
    let mut state = WindowState::default();
    state
        .panes
        .insert(pane.pane(), rterm_native::PaneState::new(pane));
    let mut host = WinitHost::with_clock(
        WindowId::new(15),
        state,
        ports,
        TurnBudget::new(8, Duration::from_millis(5)),
        clock,
    );

    let first = host
        .handle(PlatformEvent::RuntimeWake { pane })
        .expect("first turn");
    assert_eq!(first.runtime_intents, 1);
    assert!(first.continuation_scheduled);
    assert_eq!(host.state().config.revision, 1);

    let second = host
        .handle(PlatformEvent::RuntimeContinuation { pane })
        .expect("continuation");
    assert_eq!(second.runtime_intents, 1);
    assert!(!second.continuation_scheduled);
    assert_eq!(host.state().config.revision, 2);
}

#[test]
fn stale_generation_wake_never_reaches_the_runtime_drain_port() {
    let stale = token(37);
    let current = token(37);
    let mut ports = FakePorts::default();
    ports.drains.push_back(RuntimeDrain {
        intents: vec![WindowIntent::Config(ConfigDiff::new(
            99,
            Arc::new(EffectiveConfig::default()),
            Some("stale".to_owned()),
        ))],
        continuation: false,
    });
    let mut window_state = WindowState::default();
    window_state
        .panes
        .insert(current.pane(), rterm_native::PaneState::new(current));
    let mut host = WinitHost::new(
        WindowId::new(16),
        window_state,
        ports,
        TurnBudget::new(4, Duration::from_millis(5)),
    );

    let turn = host
        .handle(PlatformEvent::RuntimeWake { pane: stale })
        .expect("stale wake is ignored");

    assert_eq!(turn, rterm_native::HostTurn::default());
    assert!(host.ports().calls.is_empty());
    assert_eq!(host.state().config.revision, 0);
}

#[test]
fn token_only_runtime_event_can_deliver_a_real_batch_without_transport_bytes() {
    let pane = token(31);
    let batch = RuntimeBatch {
        pane,
        revision: RuntimeRevision::FIRST,
        snapshot: Some(Arc::new(TerminalStateSummary {
            size: TerminalSize::new(80, 24),
            cursor: (0, 0),
            scrollback_rows: 0,
            sequence: 1,
            visible_digest: 2,
        })),
        damage: vec![DamageRegion::new(0, 0, 1, 1)],
        metadata: rterm_runtime::PaneMetadataDelta::default(),
        effects: Vec::new(),
        metrics: RuntimeBatchMetrics::default(),
    };
    let mut ports = FakePorts::default();
    ports.drains.push_back(RuntimeDrain {
        intents: vec![WindowIntent::RuntimeBatch(batch)],
        continuation: false,
    });
    let mut host = WinitHost::new(
        WindowId::new(14),
        WindowState::default(),
        ports,
        TurnBudget::new(8, Duration::from_millis(5)),
    );
    host.handle(PlatformEvent::PaneLifecycle(PaneLifecycleIntent::Opened(
        pane,
    )))
    .expect("open");

    host.handle(PlatformEvent::RuntimeWake { pane })
        .expect("runtime wake");

    assert_eq!(
        host.state().panes[&pane.pane()].revision,
        Some(RuntimeRevision::FIRST)
    );
    assert!(
        host.ports()
            .calls
            .iter()
            .any(|call| call.contains("ApplyPane"))
    );
}
