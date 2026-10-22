use super::*;

fn snapshot_row_text(
    snapshot: &rterm_render_core::TerminalRenderSnapshot,
    row: u16,
    columns: u16,
) -> String {
    (0..columns)
        .map(|column| {
            snapshot
                .cells()
                .iter()
                .find(|cell| cell.row == row && cell.column == column)
                .map_or(' ', |cell| cell.ch)
        })
        .collect()
}

#[test]
fn window_manager_declared_owner_wins_over_relocation_route_on_pane_id_collision() {
    let mut declared = NativeWindowApp::new(None);
    declared.handle_pty_output(b"declared").unwrap();
    let mut collision = NativeWindowApp::new(None);
    collision.app_window_id = rterm_core::WindowId::new(2);
    collision.handle_pty_output(b"collision").unwrap();
    let mut manager = NativeWindowManager::new_for_test(declared);
    manager.pending_apps.push(Box::new(collision));
    manager.pane_event_routes.insert(
        (rterm_core::WindowId::new(1), rterm_core::PaneId::new(1)),
        PaneEventRoute {
            window_id: rterm_core::WindowId::new(2),
            pane_id: rterm_core::PaneId::new(1),
        },
    );

    assert_eq!(
        manager.dispatch_user_event_to_owner(WindowUserEvent::Output {
            window_id: rterm_core::WindowId::new(1),
            pane_id: rterm_core::PaneId::new(1),
            runtime_generation: 0,
            bytes: b"-owner".to_vec(),
        }),
        Some(false)
    );
    assert_eq!(
        manager
            .startup_app
            .as_ref()
            .map(|app| snapshot_row_text(&app.snapshot, 0, 14)),
        Some("declared-owner".to_owned())
    );
    assert_eq!(
        manager
            .pending_apps
            .first()
            .map(|app| snapshot_row_text(&app.snapshot, 0, 9)),
        Some("collision".to_owned())
    );
}

#[test]
fn window_manager_routes_runtime_hub_wakes_without_a_retired_pane_identity() {
    let mut app = NativeWindowApp::new(None);
    app.app_window_id = rterm_core::WindowId::new(23);
    let mut manager = NativeWindowManager::new_for_test(app);

    assert_eq!(
        manager.dispatch_user_event_to_owner(WindowUserEvent::RuntimeWakeWindow {
            window_id: rterm_core::WindowId::new(23),
        }),
        Some(false)
    );
}
