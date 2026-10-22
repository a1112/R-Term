use rterm_domain::PaneId;
use rterm_native::{
    CommandIntent, PaneSplitDirection, RuntimePortEffect, SpawnEffect, WindowEffect, WindowIntent,
    WindowPortEffect, WindowState,
    panes::{PaneCommand, PaneLifecycleIntent},
    reduce,
};
use rterm_runtime::{PaneToken, PaneTokenAllocator};

fn token(pane: u64) -> PaneToken {
    PaneTokenAllocator::new()
        .issue(PaneId::new(pane))
        .expect("pane generation")
}

fn apply(state: &mut WindowState, intent: WindowIntent) -> Vec<WindowEffect> {
    let mut effects = Vec::new();
    reduce(state, intent, &mut effects);
    effects
}

#[test]
fn controller_owns_generation_safe_pane_activation_and_close_fallback() {
    let mut state = WindowState::default();
    let first = token(41);
    let second = token(42);
    let stale_second = token(42);

    assert!(
        apply(
            &mut state,
            WindowIntent::PaneLifecycle(PaneLifecycleIntent::Opened(first))
        )
        .is_empty()
    );
    apply(
        &mut state,
        WindowIntent::PaneLifecycle(PaneLifecycleIntent::Opened(second)),
    );
    assert_eq!(state.pane_order, [first.pane(), second.pane()]);
    assert_eq!(state.active_pane, Some(first.pane()));

    assert_eq!(
        apply(
            &mut state,
            WindowIntent::PaneLifecycle(PaneLifecycleIntent::Activated(second))
        ),
        [WindowEffect::Window(WindowPortEffect::RequestRedraw)]
    );
    assert_eq!(state.active_pane, Some(second.pane()));
    apply(&mut state, WindowIntent::RedrawRequested);

    assert!(
        apply(
            &mut state,
            WindowIntent::PaneLifecycle(PaneLifecycleIntent::Closed(stale_second))
        )
        .is_empty()
    );
    assert_eq!(state.active_pane, Some(second.pane()));
    assert_eq!(state.pane_order, [first.pane(), second.pane()]);

    assert_eq!(
        apply(
            &mut state,
            WindowIntent::PaneLifecycle(PaneLifecycleIntent::Closed(second))
        ),
        [WindowEffect::Window(WindowPortEffect::RequestRedraw)]
    );
    assert_eq!(state.active_pane, Some(first.pane()));
    assert_eq!(state.pane_order, [first.pane()]);
}

#[test]
fn pane_commands_route_split_activation_and_idempotent_close_by_current_generation() {
    let mut state = WindowState::default();
    let first = token(51);
    let second = token(52);
    apply(
        &mut state,
        WindowIntent::PaneLifecycle(PaneLifecycleIntent::Opened(first)),
    );
    apply(
        &mut state,
        WindowIntent::PaneLifecycle(PaneLifecycleIntent::Opened(second)),
    );

    assert_eq!(
        apply(
            &mut state,
            WindowIntent::Command(CommandIntent::Pane(PaneCommand::Split {
                source: first.pane(),
                direction: PaneSplitDirection::Right,
            }))
        ),
        [WindowEffect::Spawn(SpawnEffect::SplitPane {
            source: first,
            direction: PaneSplitDirection::Right,
        })]
    );
    assert_eq!(
        apply(
            &mut state,
            WindowIntent::Command(CommandIntent::Pane(PaneCommand::Activate(second.pane())))
        ),
        [WindowEffect::Window(WindowPortEffect::RequestRedraw)]
    );
    assert_eq!(state.active_pane, Some(second.pane()));
    apply(&mut state, WindowIntent::RedrawRequested);

    assert_eq!(
        apply(
            &mut state,
            WindowIntent::Command(CommandIntent::Pane(PaneCommand::Close(second.pane())))
        ),
        [WindowEffect::Runtime(RuntimePortEffect::BeginClose {
            pane: second,
        })]
    );
    assert!(state.panes[&second.pane()].closing);
    assert!(
        apply(
            &mut state,
            WindowIntent::Command(CommandIntent::Pane(PaneCommand::Close(second.pane())))
        )
        .is_empty(),
        "a pending close must not be submitted twice"
    );
    assert!(
        apply(
            &mut state,
            WindowIntent::Command(CommandIntent::Pane(PaneCommand::Activate(PaneId::new(999))))
        )
        .is_empty()
    );
}
