use rterm_domain::PaneId;
use rterm_native::{WindowState, accessibility::build_accessibility_snapshot, panes::PaneState};
use rterm_runtime::{PaneToken, PaneTokenAllocator, RuntimeProgress};

fn token(pane: u64) -> PaneToken {
    PaneTokenAllocator::new()
        .issue(PaneId::new(pane))
        .expect("pane generation")
}

#[test]
fn accessibility_snapshot_preserves_pane_order_active_owner_and_semantic_labels() {
    let mut state = WindowState::default();
    let first = token(61);
    let second = token(62);
    state.pane_order = vec![first.pane(), second.pane()];
    state.active_pane = Some(second.pane());
    let mut first_state = PaneState::new(first);
    first_state.title = Some("editor".to_owned());
    first_state.progress = RuntimeProgress::Percentage(35);
    state.panes.insert(first.pane(), first_state);
    state.panes.insert(second.pane(), PaneState::new(second));

    let snapshot = build_accessibility_snapshot(&state);

    assert_eq!(snapshot.panes.len(), 2);
    assert_eq!(snapshot.panes[0].pane, first);
    assert_eq!(snapshot.panes[0].label, "editor");
    assert_eq!(snapshot.panes[0].progress, RuntimeProgress::Percentage(35));
    assert!(!snapshot.panes[0].active);
    assert!(snapshot.panes[1].active);
    assert_eq!(snapshot.panes[1].label, "Pane 62");
}
