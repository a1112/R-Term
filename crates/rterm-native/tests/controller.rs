use std::num::NonZeroU64;
use std::sync::Arc;

use rterm_config::EffectiveConfig;
use rterm_domain::PaneId;
use rterm_native::{
    ClipboardEffect, CommandIntent, ConfigDiff, HostEffectContext, NotificationEffect, PaneCommand,
    PaneLifecycleIntent, PlatformIntent, RendererEffect, RuntimePortEffect, TimerId, TimerIntent,
    WindowEffect, WindowIntent, WindowPortEffect, WindowState, reduce,
};
use rterm_runtime::{
    EffectSequence, MetadataChange, PaneMetadataDelta, PaneToken, PaneTokenAllocator, RuntimeBatch,
    RuntimeBatchMetrics, RuntimeEffect, RuntimeEffectKind, RuntimeProgress, RuntimeRevision,
    TerminalStateSummary,
};
use rterm_types::{DamageRegion, TerminalSize};

fn token(pane: u64) -> PaneToken {
    PaneTokenAllocator::new()
        .issue(PaneId::new(pane))
        .expect("pane generation")
}

fn summary(sequence: u64) -> TerminalStateSummary {
    TerminalStateSummary {
        size: TerminalSize::new(80, 24),
        cursor: (2, 3),
        scrollback_rows: 4,
        sequence,
        visible_digest: 0x1234,
    }
}

#[test]
fn native_window_owns_the_exact_shared_effective_config_snapshot() {
    let mut state = WindowState::default();
    let snapshot = Arc::new(EffectiveConfig::default());

    let effects = apply(
        &mut state,
        WindowIntent::Config(ConfigDiff::new(
            1,
            Arc::clone(&snapshot),
            Some("dark".to_owned()),
        )),
    );

    assert!(Arc::ptr_eq(&state.config.effective, &snapshot));
    assert!(matches!(
        effects.as_slice(),
        [
            WindowEffect::Renderer(RendererEffect::ApplyConfig { revision: 1, .. }),
            WindowEffect::Window(WindowPortEffect::RequestRedraw)
        ]
    ));
}

#[test]
fn terminal_only_config_reload_does_not_notify_renderer_or_window() {
    let mut state = WindowState::default();
    let mut candidate = EffectiveConfig::default();
    Arc::make_mut(&mut candidate.terminal).term = "xterm-rterm".to_owned();

    let effects = apply(
        &mut state,
        WindowIntent::Config(ConfigDiff::new(1, Arc::new(candidate), None)),
    );

    assert!(effects.is_empty());
    assert_eq!(state.config.effective.terminal.term, "xterm-rterm");
}

fn batch(
    pane: PaneToken,
    revision: RuntimeRevision,
    metadata: PaneMetadataDelta,
    effects: Vec<RuntimeEffect>,
) -> RuntimeBatch<TerminalStateSummary> {
    RuntimeBatch {
        pane,
        revision,
        snapshot: Some(Arc::new(summary(revision.get()))),
        damage: vec![DamageRegion::new(1, 2, 3, 4)],
        metadata,
        effects,
        metrics: RuntimeBatchMetrics::default(),
    }
}

fn apply(state: &mut WindowState, intent: WindowIntent) -> Vec<WindowEffect> {
    let mut effects = Vec::new();
    reduce(state, intent, &mut effects);
    effects
}

#[test]
fn platform_intents_update_state_and_emit_only_typed_window_renderer_effects() {
    let mut state = WindowState::default();

    assert_eq!(
        apply(
            &mut state,
            WindowIntent::Platform(PlatformIntent::Focused(true))
        ),
        [WindowEffect::Window(WindowPortEffect::SetFocused(true))]
    );
    assert!(state.platform.focused);

    let size = TerminalSize::new(132, 43);
    assert_eq!(
        apply(
            &mut state,
            WindowIntent::Platform(PlatformIntent::Resized(size))
        ),
        [
            WindowEffect::Renderer(RendererEffect::ResizeSurface(size)),
            WindowEffect::Window(WindowPortEffect::RequestRedraw),
        ]
    );
    assert_eq!(state.presentation.size, size);
    assert!(state.presentation.redraw_pending);
}

#[test]
fn runtime_batch_updates_metadata_snapshot_and_preserves_ordered_host_effects() {
    let mut state = WindowState::default();
    let pane = token(11);
    apply(
        &mut state,
        WindowIntent::PaneLifecycle(PaneLifecycleIntent::Opened(pane)),
    );
    let metadata = PaneMetadataDelta {
        title: Some(MetadataChange::Set("build".to_owned())),
        progress: Some(MetadataChange::Set(RuntimeProgress::Percentage(42))),
        ..PaneMetadataDelta::default()
    };
    let effects = vec![
        RuntimeEffect::new(
            EffectSequence::FIRST,
            RuntimeEffectKind::ClipboardWrite {
                selection: Some("c".to_owned()),
                contents: "hello".to_owned(),
            },
        ),
        RuntimeEffect::new(
            EffectSequence::FIRST.next().unwrap(),
            RuntimeEffectKind::Notification {
                title: Some("done".to_owned()),
                body: "ok".to_owned(),
            },
        ),
    ];
    let context = |sequence| HostEffectContext {
        pane,
        revision: RuntimeRevision::FIRST,
        sequence,
    };

    let emitted = apply(
        &mut state,
        WindowIntent::RuntimeBatch(batch(pane, RuntimeRevision::FIRST, metadata, effects)),
    );

    assert_eq!(
        emitted,
        [
            WindowEffect::Clipboard(ClipboardEffect::Write {
                context: Some(context(EffectSequence::FIRST)),
                selection: Some("c".to_owned()),
                contents: "hello".to_owned(),
            }),
            WindowEffect::Notification(NotificationEffect::Show {
                context: context(EffectSequence::FIRST.next().unwrap()),
                title: Some("done".to_owned()),
                body: "ok".to_owned(),
            }),
            WindowEffect::Renderer(RendererEffect::ApplyPane {
                pane,
                revision: RuntimeRevision::FIRST,
                snapshot: summary(1),
                damage: vec![DamageRegion::new(1, 2, 3, 4)],
            }),
            WindowEffect::Window(WindowPortEffect::RequestRedraw),
        ]
    );
    let pane_state = state.panes.get(&PaneId::new(11)).unwrap();
    assert_eq!(pane_state.title.as_deref(), Some("build"));
    assert_eq!(pane_state.progress, RuntimeProgress::Percentage(42));
    assert_eq!(pane_state.revision, Some(RuntimeRevision::FIRST));
}

#[test]
fn visible_output_routes_to_the_session_log_port() {
    let pane = token(31);
    let mut state = WindowState::default();
    apply(
        &mut state,
        WindowIntent::PaneLifecycle(PaneLifecycleIntent::Opened(pane)),
    );
    let effects = apply(
        &mut state,
        WindowIntent::RuntimeBatch(batch(
            pane,
            RuntimeRevision::FIRST,
            PaneMetadataDelta::default(),
            vec![RuntimeEffect::new(
                EffectSequence::FIRST,
                RuntimeEffectKind::VisibleOutput(b"visible".to_vec()),
            )],
        )),
    );

    assert!(matches!(
        effects.first(),
        Some(WindowEffect::Runtime(RuntimePortEffect::WriteSessionLog { bytes, .. }))
            if bytes == b"visible"
    ));
}

#[test]
fn stale_generation_revision_and_effect_gap_are_atomic_noops() {
    let mut state = WindowState::default();
    let current = token(13);
    let stale_token = token(13);
    apply(
        &mut state,
        WindowIntent::PaneLifecycle(PaneLifecycleIntent::Opened(current)),
    );
    apply(
        &mut state,
        WindowIntent::RuntimeBatch(batch(
            current,
            RuntimeRevision::FIRST,
            PaneMetadataDelta::default(),
            vec![],
        )),
    );
    let before = state.clone();

    assert!(
        apply(
            &mut state,
            WindowIntent::RuntimeBatch(batch(
                stale_token,
                RuntimeRevision::FIRST.next().unwrap(),
                PaneMetadataDelta::default(),
                vec![],
            ))
        )
        .is_empty()
    );
    assert!(
        apply(
            &mut state,
            WindowIntent::RuntimeBatch(batch(
                current,
                RuntimeRevision::FIRST,
                PaneMetadataDelta::default(),
                vec![],
            ))
        )
        .is_empty()
    );
    let gap = RuntimeEffect::new(
        EffectSequence::FIRST.next().unwrap(),
        RuntimeEffectKind::Diagnostic {
            message: "gap".to_owned(),
        },
    );
    let rejected = apply(
        &mut state,
        WindowIntent::RuntimeBatch(batch(
            current,
            RuntimeRevision::FIRST.next().unwrap(),
            PaneMetadataDelta::default(),
            vec![gap],
        )),
    );
    assert!(matches!(
        rejected.as_slice(),
        [WindowEffect::Window(WindowPortEffect::ReportDiagnostic(message))]
            if message.contains("gap")
    ));
    assert_eq!(state, before);
}

#[test]
fn config_timer_redraw_restart_and_close_transitions_are_deterministic() {
    let mut state = WindowState::default();
    let pane = token(17);
    apply(
        &mut state,
        WindowIntent::PaneLifecycle(PaneLifecycleIntent::Opened(pane)),
    );

    let config_effects = apply(
        &mut state,
        WindowIntent::Config(ConfigDiff::new(
            2,
            Arc::new(EffectiveConfig::default()),
            Some("dark".to_owned()),
        )),
    );
    assert_eq!(
        config_effects,
        [
            WindowEffect::Renderer(RendererEffect::ApplyConfig {
                revision: 2,
                theme: Some("dark".to_owned()),
            }),
            WindowEffect::Window(WindowPortEffect::RequestRedraw),
        ]
    );
    assert!(
        apply(
            &mut state,
            WindowIntent::Config(ConfigDiff::new(
                1,
                Arc::new(EffectiveConfig::default()),
                Some("stale".to_owned()),
            ))
        )
        .is_empty()
    );

    assert_eq!(
        apply(&mut state, WindowIntent::RedrawRequested),
        [WindowEffect::Renderer(RendererEffect::Present)]
    );

    let timer = TimerId::new(9);
    apply(
        &mut state,
        WindowIntent::Timer(TimerIntent::Arm { timer, epoch: 3 }),
    );
    assert!(
        apply(
            &mut state,
            WindowIntent::Timer(TimerIntent::Fired { timer, epoch: 2 })
        )
        .is_empty()
    );
    assert_eq!(
        apply(
            &mut state,
            WindowIntent::Timer(TimerIntent::Fired { timer, epoch: 3 })
        ),
        [WindowEffect::Window(WindowPortEffect::RequestRedraw)]
    );
    assert_eq!(
        apply(&mut state, WindowIntent::RedrawRequested),
        [WindowEffect::Renderer(RendererEffect::Present)]
    );

    assert_eq!(
        apply(
            &mut state,
            WindowIntent::Command(CommandIntent::Pane(PaneCommand::Restart(PaneId::new(17))))
        ),
        [WindowEffect::Runtime(RuntimePortEffect::Restart { pane })]
    );
    assert_eq!(
        apply(&mut state, WindowIntent::CloseRequested),
        [
            WindowEffect::Runtime(RuntimePortEffect::BeginClose { pane }),
            WindowEffect::Window(WindowPortEffect::CloseAfterRuntimes),
        ]
    );
    assert!(state.lifecycle.closing);
    assert!(apply(&mut state, WindowIntent::CloseRequested).is_empty());
    assert!(
        apply(
            &mut state,
            WindowIntent::PaneLifecycle(PaneLifecycleIntent::Closed(token(17)))
        )
        .is_empty()
    );
    assert_eq!(
        apply(
            &mut state,
            WindowIntent::PaneLifecycle(PaneLifecycleIntent::Closed(pane))
        ),
        [WindowEffect::Window(WindowPortEffect::CloseNow)]
    );
}

#[test]
fn metadata_only_batch_redraws_once_and_stale_open_cannot_replace_the_owner() {
    let mut state = WindowState::default();
    let old = token(18);
    let current = token(18);
    apply(
        &mut state,
        WindowIntent::PaneLifecycle(PaneLifecycleIntent::Opened(current)),
    );
    apply(
        &mut state,
        WindowIntent::PaneLifecycle(PaneLifecycleIntent::Opened(old)),
    );
    assert_eq!(state.panes[&PaneId::new(18)].token, current);

    let metadata = PaneMetadataDelta {
        title: Some(MetadataChange::Set("metadata-only".to_owned())),
        ..PaneMetadataDelta::default()
    };
    let mut publication = batch(current, RuntimeRevision::FIRST, metadata, vec![]);
    publication.snapshot = None;
    publication.damage.clear();
    assert_eq!(
        apply(&mut state, WindowIntent::RuntimeBatch(publication)),
        [WindowEffect::Window(WindowPortEffect::RequestRedraw)]
    );
    assert_eq!(
        state.panes[&PaneId::new(18)].title.as_deref(),
        Some("metadata-only")
    );
}

#[test]
fn damage_without_snapshot_is_rejected_without_advancing_revision() {
    let mut state = WindowState::default();
    let pane = token(20);
    apply(
        &mut state,
        WindowIntent::PaneLifecycle(PaneLifecycleIntent::Opened(pane)),
    );
    let mut publication = batch(
        pane,
        RuntimeRevision::FIRST,
        PaneMetadataDelta::default(),
        vec![],
    );
    publication.snapshot = None;

    let effects = apply(&mut state, WindowIntent::RuntimeBatch(publication));

    assert!(matches!(
        effects.as_slice(),
        [WindowEffect::Window(WindowPortEffect::ReportDiagnostic(message))]
            if message.contains("damage without a snapshot")
    ));
    assert_eq!(state.panes[&PaneId::new(20)].revision, None);
}

#[test]
fn runtime_effect_ports_cover_transport_bell_clipboard_and_diagnostics() {
    let mut state = WindowState::default();
    let pane = token(19);
    apply(
        &mut state,
        WindowIntent::PaneLifecycle(PaneLifecycleIntent::Opened(pane)),
    );
    let sequences = [
        EffectSequence::FIRST,
        EffectSequence::FIRST.next().unwrap(),
        EffectSequence::FIRST.next().unwrap().next().unwrap(),
        EffectSequence::FIRST
            .next()
            .unwrap()
            .next()
            .unwrap()
            .next()
            .unwrap(),
        EffectSequence::FIRST
            .next()
            .unwrap()
            .next()
            .unwrap()
            .next()
            .unwrap()
            .next()
            .unwrap(),
    ];
    let effects = vec![
        RuntimeEffect::new(
            sequences[0],
            RuntimeEffectKind::TransportWrite(b"reply".to_vec()),
        ),
        RuntimeEffect::new(
            sequences[1],
            RuntimeEffectKind::HostStream(b"display".to_vec()),
        ),
        RuntimeEffect::new(
            sequences[2],
            RuntimeEffectKind::Bell {
                count: NonZeroU64::new(2).unwrap(),
            },
        ),
        RuntimeEffect::new(
            sequences[3],
            RuntimeEffectKind::ClipboardRead {
                selection: "p".to_owned(),
            },
        ),
        RuntimeEffect::new(
            sequences[4],
            RuntimeEffectKind::Diagnostic {
                message: "bad sequence".to_owned(),
            },
        ),
    ];

    let emitted = apply(
        &mut state,
        WindowIntent::RuntimeBatch(batch(
            pane,
            RuntimeRevision::FIRST,
            PaneMetadataDelta::default(),
            effects,
        )),
    );

    assert!(matches!(
        &emitted[0],
        WindowEffect::Runtime(RuntimePortEffect::WriteTransport { bytes, .. }) if bytes == b"reply"
    ));
    assert!(matches!(
        &emitted[1],
        WindowEffect::Runtime(RuntimePortEffect::ObserveHostStream { bytes, .. }) if bytes == b"display"
    ));
    assert!(matches!(
        &emitted[2],
        WindowEffect::Renderer(RendererEffect::Bell { count, .. }) if count.get() == 2
    ));
    assert!(matches!(
        &emitted[3],
        WindowEffect::Clipboard(ClipboardEffect::Read { selection, .. }) if selection == "p"
    ));
    assert!(matches!(
        &emitted[4],
        WindowEffect::Window(WindowPortEffect::RuntimeDiagnostic { message, .. })
            if message == "bad sequence"
    ));
}
