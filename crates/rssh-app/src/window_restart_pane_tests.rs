use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::Duration;

use super::*;

#[test]
fn window_app_recognizes_restart_pane_typed_action_query() {
    let mut app = NativeWindowApp::new(None);
    app.enter_command_palette_mode();
    app.command_palette_set_query("wezterm.action.RestartPane".to_owned());

    let labels = app
        .command_palette_filtered_commands()
        .iter()
        .map(WindowCommand::label)
        .collect::<Vec<_>>();

    assert_eq!(labels, ["Restart Pane"]);
}

#[test]
fn restart_pane_command_has_palette_metadata_and_records_frecency() {
    let mut app = NativeWindowApp::new(None);

    assert!(WINDOW_COMMANDS.contains(&WindowCommand::RestartPane));
    assert_eq!(WindowCommand::RestartPane.label(), "Restart Pane");
    assert_eq!(app.command_palette_frecency("Restart Pane").uses, 0);

    app.enter_command_palette_mode();
    assert!(app.command_palette_execute(WindowCommand::RestartPane));

    assert_eq!(app.command_palette_frecency("Restart Pane").uses, 1);
}

#[test]
fn restart_pane_does_not_replace_reload_configuration_shortcut() {
    let assignment = NATIVE_WINDOW_KEY_ASSIGNMENTS
        .iter()
        .find(|assignment| assignment.keys == "CTRL+SHIFT+R")
        .expect("default reload shortcut");

    assert_eq!(assignment.command, WindowCommand::ReloadConfiguration);
    assert_ne!(assignment.command, WindowCommand::RestartPane);
}

#[test]
fn window_app_restart_pane_retires_active_runtime_and_owner_state() {
    let mut app = NativeWindowApp::new(None);
    app.handle_pty_output(b"inactive").unwrap();
    let launch = PaneLaunch::local("restart-shell")
        .with_args(["--login"])
        .with_cwd("file://host/work")
        .with_environment([("RESTART_TEST", "preserved")]);
    app.dispatch_app_action(AppAction::SplitPane {
        pane: app.active_pane_id(),
        direction: SplitDirection::Right,
        launch: Some(launch.clone()),
    })
    .unwrap();
    app.handle_pty_output(b"active").unwrap();
    let active = app.active_pane_id();
    let inactive = rssh_core::PaneId::new(1);
    app.dispatch_app_action(AppAction::TogglePaneZoom { pane: active })
        .unwrap();

    let writer_dropped = Arc::new(AtomicUsize::new(0));
    app.writer = Some(Box::new(DropTrackingWriter(Arc::clone(&writer_dropped))));
    app.session_process_id = Some(41_101);
    app.session_tty_name = Some("old-tty".to_owned());
    app.reader_thread = Some(std::thread::spawn(|| {}));
    let initial_copy_mode = app.initial_copy_mode();
    app.active_ui.enter_copy_mode(initial_copy_mode);
    app.ime_preedit = Some("old-preedit".to_owned());
    app.dead_key_active = true;
    app.dead_key_text = Some("old-dead-key".to_owned());
    app.selecting = true;
    app.active_mouse_button = Some(MouseButton::Left);
    app.scrollbar_dragging = true;
    app.ui_left_release_pending = true;
    app.pressed_pane_close_button = Some(active);
    let inactive_snapshot = app.pane_snapshot(inactive).unwrap().clone();
    let inactive_process_id = app
        .pane_runtimes
        .get(&inactive)
        .and_then(|runtime| runtime.session_process_id);
    let pane_ids = app.app_shell.pane_ids();
    let launch_before = app.app_shell.active_pane().launch().clone();

    assert!(app.command_palette_execute(WindowCommand::RestartPane));

    assert_eq!(app.active_pane_id(), active);
    assert_eq!(app.app_shell.active_pane().launch(), &launch_before);
    assert_eq!(app.app_shell.pane_ids(), pane_ids);
    assert_eq!(app.app_shell.active_tab().zoomed_pane_id(), Some(active));
    assert_eq!(writer_dropped.load(Ordering::Relaxed), 1);
    assert!(app.session.is_none());
    assert!(app.session_process_id.is_none());
    assert!(app.session_tty_name.is_none());
    assert!(app.writer.is_none());
    assert!(app.reader_thread.is_none());
    assert_eq!(snapshot_char(&app.snapshot, 0, 0), None);
    assert!(app.active_ui.ordinary_selection.is_none());
    assert!(!pane_copy_overlay_rendering(&app.active_ui));
    assert!(app.selection.is_none());
    assert!(app.ime_preedit.is_none());
    assert!(!app.dead_key_active);
    assert!(app.dead_key_text.is_none());
    assert!(!app.selecting);
    assert!(app.active_mouse_button.is_none());
    assert!(!app.scrollbar_dragging);
    assert!(!app.ui_left_release_pending);
    assert!(app.pressed_pane_close_button.is_none());
    assert_eq!(app.pane_snapshot(inactive).unwrap(), &inactive_snapshot);
    assert_eq!(
        app.pane_runtimes
            .get(&inactive)
            .and_then(|runtime| runtime.session_process_id),
        inactive_process_id
    );
}

#[test]
fn window_app_restart_pane_installs_fresh_runtime_without_touching_other_owner() {
    let mut app = NativeWindowApp::new(None);
    app.handle_pty_output(b"inactive-old").unwrap();
    app.dispatch_app_action(AppAction::SplitPane {
        pane: app.active_pane_id(),
        direction: SplitDirection::Right,
        launch: Some(PaneLaunch::local("restart-success")),
    })
    .unwrap();
    app.handle_pty_output(b"active-old").unwrap();
    let active = app.active_pane_id();
    let inactive = rssh_core::PaneId::new(1);
    let inactive_snapshot = app.pane_snapshot(inactive).unwrap().clone();
    let old_writer_dropped = Arc::new(AtomicUsize::new(0));
    app.writer = Some(Box::new(DropTrackingWriter(Arc::clone(
        &old_writer_dropped,
    ))));
    app.session_process_id = Some(41_201);

    app.restart_pane_runtime_with(active, |app, _| {
        let mut runtime = app.new_inactive_pane_runtime();
        runtime.runtime.feed_pty_output_with_display(b"fresh");
        runtime
            .ui
            .reconcile_terminal_mutation(runtime.runtime.terminal());
        runtime.snapshot = terminal_runtime_snapshot(&runtime.runtime, runtime.ui.stable_viewport);
        runtime.session_process_id = Some(41_202);
        Ok::<_, Box<dyn std::error::Error>>(runtime)
    })
    .unwrap();

    assert_eq!(old_writer_dropped.load(Ordering::Relaxed), 1);
    assert_eq!(app.session_process_id, Some(41_202));
    assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('f'));
    assert_eq!(app.pane_snapshot(inactive).unwrap(), &inactive_snapshot);
    assert_eq!(app.active_pane_id(), active);
}

#[cfg(feature = "ssh")]
#[test]
fn ssh_retry_preserves_terminal_presentation_and_allocates_a_new_generation() {
    let mut app = NativeWindowApp::new(None);
    app.set_initial_pane_launch(PaneLaunch::ssh(SshPaneLaunch::new(
        "ops@example.test:2222",
        SshAuthDescription::PasswordPrompt,
        SshKnownHostsPolicy::Prompt,
    )));
    app.runtime.resize(rssh_core::TerminalSize::new(32, 2));
    app.handle_pty_output(b"remote output survives retry")
        .unwrap();
    app.active_runtime_generation = 44_001;
    app.active_runtime_transport = Some(PaneRuntimeTransportKind::NativeSsh);
    let pane = app.active_pane_id();
    let snapshot_before = app.snapshot.clone();
    let generation_before = app.active_runtime_generation;

    app.restart_pane_runtime_with(pane, |app, _| {
        let mut runtime = app.new_inactive_pane_runtime();
        runtime.runtime_generation = 44_002;
        Ok::<_, Box<dyn std::error::Error>>(runtime)
    })
    .unwrap();

    assert_eq!(app.snapshot, snapshot_before);
    assert_ne!(app.active_runtime_generation, generation_before);
    assert_eq!(app.active_runtime_generation, 44_002);
}

#[cfg(feature = "ssh")]
#[test]
fn window_manager_ignores_events_from_retired_pane_runtime_generation() {
    let mut app = NativeWindowApp::new(None);
    let pane_id = app.active_pane_id();
    let window_id = app.app_window_id_for_test();
    let retired_generation = app.active_runtime_generation;

    app.restart_pane_runtime_with(pane_id, |app, _| {
        let mut runtime = app.new_inactive_pane_runtime();
        runtime.runtime.feed_pty_output_with_display(b"fresh");
        runtime.snapshot = terminal_runtime_snapshot(&runtime.runtime, runtime.ui.stable_viewport);
        runtime.session_process_id = Some(41_302);
        Ok::<_, Box<dyn std::error::Error>>(runtime)
    })
    .unwrap();
    let active_generation = app.active_runtime_generation;
    assert_ne!(active_generation, retired_generation);
    let current_ssh_cancellation = Arc::new(AtomicBool::new(false));
    let (current_ssh_sender, _current_ssh_receiver) = std::sync::mpsc::channel();
    app.ssh_writer_senders
        .insert(pane_id, current_ssh_sender);
    app.ssh_writer_cancellations
        .insert(pane_id, Arc::clone(&current_ssh_cancellation));
    let mut manager = NativeWindowManager::new(app);

    assert_eq!(
        manager.dispatch_user_event_to_owner(WindowUserEvent::Output {
            window_id,
            pane_id,
            runtime_generation: retired_generation,
            bytes: b"stale".to_vec(),
        }),
        Some(false)
    );
    assert_eq!(
        manager.dispatch_user_event_to_owner(WindowUserEvent::Exited {
            window_id,
            pane_id,
            runtime_generation: retired_generation,
        }),
        Some(false)
    );
    assert_eq!(
        manager.dispatch_user_event_to_owner(WindowUserEvent::SshState {
            window_id,
            pane_id,
            runtime_generation: retired_generation,
            state: ConnectionState::Failed,
        }),
        Some(false)
    );

    let app = manager.startup_app.as_ref().unwrap();
    assert_eq!(app.active_runtime_generation, active_generation);
    assert_eq!(app.session_process_id, Some(41_302));
    assert_eq!(snapshot_char(&app.snapshot, 0, 0), Some('f'));
    assert!(app.ssh_writer_cancellations.contains_key(&pane_id));
    assert!(!current_ssh_cancellation.load(Ordering::Acquire));
}

#[test]
fn window_manager_ignores_retired_events_after_pane_owner_transfer() {
    let mut primary = NativeWindowApp::new(None);
    primary.runtime.resize(rssh_core::TerminalSize::new(16, 1));
    primary.handle_pty_output(b"relocated").unwrap();
    primary.active_runtime_generation = 41;
    primary
        .dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
    primary
        .dispatch_app_action(AppAction::MovePaneToNewWindow {
            pane: rssh_core::PaneId::new(1),
        })
        .unwrap();
    let mut manager = NativeWindowManager::new_for_test(primary);
    manager.collect_pending_window_apps_from_primary_for_test();

    assert_eq!(
        manager.dispatch_user_event_to_owner(WindowUserEvent::Output {
            window_id: rssh_core::WindowId::new(1),
            pane_id: rssh_core::PaneId::new(1),
            runtime_generation: 40,
            bytes: b"-stale".to_vec(),
        }),
        Some(false)
    );

    let pending_text = manager
        .pending_apps
        .first()
        .map(|app| snapshot_row_text(&app.snapshot, 0, 16));
    assert_eq!(
        pending_text.as_deref().map(str::trim_end),
        Some("relocated")
    );
}

#[test]
fn runtime_tokens_are_process_unique_across_window_apps() {
    let mut first = NativeWindowApp::new(None);
    let mut second = NativeWindowApp::new(None);

    let first_token = first.allocate_pane_runtime_generation();
    let second_token = second.allocate_pane_runtime_generation();

    assert_ne!(first_token, second_token);
    assert_ne!(first_token, 0);
    assert_ne!(second_token, 0);
}

#[test]
fn runtime_token_allocator_fails_fast_before_overflow_reuse() {
    let exhausted = AtomicU64::new(u64::MAX);

    let panic = std::panic::catch_unwind(|| allocate_pane_runtime_token_from(&exhausted));

    assert!(panic.is_err());
    assert_eq!(exhausted.load(Ordering::Relaxed), u64::MAX);
}

#[test]
fn manager_never_falls_back_to_same_pane_id_without_a_route() {
    let mut owner = NativeWindowApp::new(None);
    owner.handle_pty_output(b"RED").unwrap();
    owner.active_runtime_generation = 72;
    let mut manager = NativeWindowManager::new_for_test(owner);

    for event in [
        WindowUserEvent::Output {
            window_id: rssh_core::WindowId::new(99),
            pane_id: rssh_core::PaneId::new(1),
            runtime_generation: 71,
            bytes: b"-wrong".to_vec(),
        },
        WindowUserEvent::Exited {
            window_id: rssh_core::WindowId::new(99),
            pane_id: rssh_core::PaneId::new(1),
            runtime_generation: 71,
        },
        WindowUserEvent::ReadError {
            window_id: rssh_core::WindowId::new(99),
            pane_id: rssh_core::PaneId::new(1),
            runtime_generation: 71,
            error: "stale".to_owned(),
        },
    ] {
        assert_eq!(manager.dispatch_user_event_to_owner(event), None);
    }

    let owner = manager.startup_app.as_ref().unwrap();
    assert_eq!(snapshot_row_text(&owner.snapshot, 0, 3), "RED");
    assert_eq!(owner.active_runtime_generation, 72);
}

#[test]
fn closed_detach_route_never_delivers_old_events_to_another_detached_same_id_owner() {
    let mut first_source = NativeWindowApp::new(None);
    first_source
        .runtime
        .resize(rssh_core::TerminalSize::new(16, 1));
    first_source.handle_pty_output(b"first").unwrap();
    first_source.active_runtime_generation = 41;
    first_source
        .dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
    first_source
        .dispatch_app_action(AppAction::MovePaneToNewWindow {
            pane: rssh_core::PaneId::new(1),
        })
        .unwrap();
    let mut manager = NativeWindowManager::new_for_test(first_source);
    manager.collect_pending_window_apps_from_primary_for_test();

    drop(manager.startup_app.take());
    drop(manager.pending_apps.remove(0));
    manager.remove_pane_event_routes_for_window(rssh_core::WindowId::new(1));
    manager.remove_pane_event_routes_for_window(rssh_core::WindowId::new(2));

    let mut second_source = NativeWindowApp::new(None);
    second_source.app_window_id = rssh_core::WindowId::new(3);
    second_source
        .runtime
        .resize(rssh_core::TerminalSize::new(16, 1));
    second_source.handle_pty_output(b"RED").unwrap();
    second_source.active_runtime_generation = 41;
    second_source
        .dispatch_app_action(AppAction::SplitPane {
            pane: rssh_core::PaneId::new(1),
            direction: SplitDirection::Right,
            launch: None,
        })
        .unwrap();
    second_source
        .dispatch_app_action(AppAction::MovePaneToNewWindow {
            pane: rssh_core::PaneId::new(1),
        })
        .unwrap();
    let mut second_owner = second_source
        .take_next_pending_window_app()
        .expect("second detached owner");
    second_owner.app_window_id = rssh_core::WindowId::new(4);
    manager.pending_apps.push(second_owner);
    manager.pane_event_routes.insert(
        (rssh_core::WindowId::new(3), rssh_core::PaneId::new(1)),
        PaneEventRoute {
            window_id: rssh_core::WindowId::new(4),
            pane_id: rssh_core::PaneId::new(1),
        },
    );

    for event in [
        WindowUserEvent::Output {
            window_id: rssh_core::WindowId::new(1),
            pane_id: rssh_core::PaneId::new(1),
            runtime_generation: 41,
            bytes: b"-stale".to_vec(),
        },
        WindowUserEvent::Exited {
            window_id: rssh_core::WindowId::new(1),
            pane_id: rssh_core::PaneId::new(1),
            runtime_generation: 41,
        },
        WindowUserEvent::ReadError {
            window_id: rssh_core::WindowId::new(1),
            pane_id: rssh_core::PaneId::new(1),
            runtime_generation: 41,
            error: "stale".to_owned(),
        },
    ] {
        assert_eq!(manager.dispatch_user_event_to_owner(event), None);
    }
    assert_eq!(
        manager
            .pending_apps
            .first()
            .map(|app| snapshot_row_text(&app.snapshot, 0, 3)),
        Some("RED".to_owned())
    );

    assert_eq!(
        manager.dispatch_user_event_to_owner(WindowUserEvent::Output {
            window_id: rssh_core::WindowId::new(3),
            pane_id: rssh_core::PaneId::new(1),
            runtime_generation: 41,
            bytes: b"-current".to_vec(),
        }),
        Some(false)
    );
    assert_eq!(
        manager
            .pending_apps
            .first()
            .map(|app| snapshot_row_text(&app.snapshot, 0, 11)),
        Some("RED-current".to_owned())
    );
}

#[test]
fn restart_inactive_pane_replaces_only_target_runtime() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rssh_core::TerminalSize::new(16, 1));
    app.handle_pty_output(b"inactive-old").unwrap();
    app.dispatch_app_action(AppAction::SplitPane {
        pane: app.active_pane_id(),
        direction: SplitDirection::Right,
        launch: Some(PaneLaunch::local("active-shell")),
    })
    .unwrap();
    app.handle_pty_output(b"active-old").unwrap();
    let active = app.active_pane_id();
    let inactive = rssh_core::PaneId::new(1);
    let active_snapshot = app.snapshot.clone();
    let active_generation = app.active_runtime_generation;

    app.restart_pane_runtime_with(inactive, |app, pane_id| {
        assert_eq!(pane_id, inactive);
        let mut runtime = app.new_inactive_pane_runtime();
        runtime
            .runtime
            .feed_pty_output_with_display(b"inactive-fresh");
        runtime.snapshot = terminal_runtime_snapshot(&runtime.runtime, runtime.ui.stable_viewport);
        Ok::<_, Box<dyn std::error::Error>>(runtime)
    })
    .unwrap();

    assert_eq!(app.active_pane_id(), active);
    assert_eq!(app.snapshot, active_snapshot);
    assert_eq!(app.active_runtime_generation, active_generation);
    assert_eq!(
        app.pane_runtimes
            .get(&inactive)
            .map(|runtime| snapshot_row_text(&runtime.snapshot, 0, 14)),
        Some("inactive-fresh".to_owned())
    );
}

#[test]
fn restart_without_new_cwd_evidence_preserves_full_pane_launch_on_success_and_failure() {
    let launch = PaneLaunch::local("restart-shell")
        .with_args(["--login", "--noprofile"])
        .with_cwd("file://host/preserved")
        .with_environment([("KEEP_ONE", "yes"), ("KEEP_TWO", "also")]);

    for spawn_succeeds in [true, false] {
        let mut app = NativeWindowApp::new(None);
        app.app_shell = AppShell::new(launch.clone());
        app.session_process_id = None;
        let pane = app.active_pane_id();

        let result = app.restart_pane_runtime_with(pane, |app, _| {
            if spawn_succeeds {
                Ok::<_, Box<dyn std::error::Error>>(app.new_inactive_pane_runtime())
            } else {
                Err::<PaneRuntime, _>(Box::new(io::Error::other("spawn failed")))
            }
        });
        assert_eq!(result.is_ok(), spawn_succeeds);
        assert_eq!(app.app_shell.active_pane().launch(), &launch);
        assert_eq!(snapshot_char(&app.snapshot, 0, 0), None);
        assert_ne!(app.active_runtime_generation, 0);
    }
}

#[test]
fn restart_uses_explicit_runtime_cwd_evidence_without_changing_other_launch_fields() {
    let original = PaneLaunch::local("restart-shell")
        .with_args(["--login"])
        .with_cwd("file://host/original")
        .with_environment([("KEEP", "yes")]);
    let mut app = NativeWindowApp::new(None);
    app.app_shell = AppShell::new(original);
    app.runtime
        .feed_pty_output_with_display(b"\x1b]7;file://host/new-cwd\x07");
    let pane = app.active_pane_id();

    app.restart_pane_runtime_with(pane, |app, _| {
        Ok::<_, Box<dyn std::error::Error>>(app.new_inactive_pane_runtime())
    })
    .unwrap();

    let launch = app.app_shell.active_pane().launch();
    assert_eq!(launch.program(), "restart-shell");
    assert_eq!(launch.args(), ["--login"]);
    assert_eq!(launch.cwd(), Some("file://host/new-cwd"));
    assert_eq!(
        launch.environment().get("KEEP").map(String::as_str),
        Some("yes")
    );
}

#[test]
fn restart_clears_only_target_runtime_projection_and_resets_active_title() {
    let mut app = NativeWindowApp::new(None);
    app.dispatch_app_action(AppAction::SplitPane {
        pane: app.active_pane_id(),
        direction: SplitDirection::Right,
        launch: Some(PaneLaunch::local("active")),
    })
    .unwrap();
    let active = app.active_pane_id();
    let inactive = rssh_core::PaneId::new(1);
    for pane in [active, inactive] {
        app.dispatch_app_action(AppAction::SetPaneUserVar {
            pane,
            name: "RUNTIME".to_owned(),
            value: pane.get().to_string(),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SetPaneBadgeFormat {
            pane,
            badge_format: Some(format!("badge-{}", pane.get())),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SetPaneProgress {
            pane,
            progress: PaneProgress::Percentage(42),
        })
        .unwrap();
        app.dispatch_app_action(AppAction::SetPaneHasUnseenOutput {
            pane,
            has_unseen_output: true,
        })
        .unwrap();
    }
    app.window_title = "stale runtime title".to_owned();

    app.restart_pane_runtime_with(active, |app, _| {
        Ok::<_, Box<dyn std::error::Error>>(app.new_inactive_pane_runtime())
    })
    .unwrap();

    let target = app.app_shell.active_pane();
    assert!(target.user_vars().is_empty());
    assert_eq!(target.badge_format(), None);
    assert_eq!(target.progress(), PaneProgress::None);
    assert!(!target.has_unseen_output());
    assert_eq!(app.window_title, DEFAULT_WINDOW_TITLE);
    let sibling = app
        .app_shell
        .active_tab()
        .panes()
        .iter()
        .find(|pane| pane.id() == inactive)
        .unwrap();
    assert_eq!(
        sibling.user_vars().get("RUNTIME").map(String::as_str),
        Some("1")
    );
    assert_eq!(sibling.badge_format(), Some("badge-1"));
    assert_eq!(sibling.progress(), PaneProgress::Percentage(42));
    assert!(sibling.has_unseen_output());
}

#[test]
fn manager_ignores_stale_read_error_and_accepts_current_read_error() {
    let mut app = NativeWindowApp::new(None);
    let pane = app.active_pane_id();
    let window = app.app_window_id;
    app.active_runtime_generation = 91;
    let mut manager = NativeWindowManager::new_for_test(app);

    assert_eq!(
        manager.dispatch_user_event_to_owner(WindowUserEvent::ReadError {
            window_id: window,
            pane_id: pane,
            runtime_generation: 90,
            error: "stale".to_owned(),
        }),
        Some(false)
    );
    assert!(manager.startup_app.is_some());

    assert_eq!(
        manager.dispatch_user_event_to_owner(WindowUserEvent::ReadError {
            window_id: window,
            pane_id: pane,
            runtime_generation: 91,
            error: "current".to_owned(),
        }),
        Some(true)
    );
    assert!(manager.startup_app.is_none());
}

#[test]
fn wheel_restart_targets_inactive_pane_and_keeps_active_runtime_unchanged_on_spawn_failure() {
    let mut app = NativeWindowApp::new(None);
    app.runtime.resize(rssh_core::TerminalSize::new(16, 1));
    app.handle_pty_output(b"inactive-old").unwrap();
    app.dispatch_app_action(AppAction::SplitPane {
        pane: app.active_pane_id(),
        direction: SplitDirection::Right,
        launch: Some(PaneLaunch::local("active")),
    })
    .unwrap();
    app.handle_pty_output(b"active-old").unwrap();
    let active_snapshot = app.snapshot.clone();
    let active_generation = app.active_runtime_generation;
    let inactive = rssh_core::PaneId::new(1);
    let rect = app.pane_render_rect(inactive).unwrap();
    let target = WheelTarget {
        pane_id: inactive,
        rect,
        cell: PaneMouseCell {
            pane_id: inactive,
            row: 0,
            column: 0,
        },
        pixel_position: PhysicalPosition::new(1.0, 1.0),
    };

    let error = app
        .apply_wheel_pane_action(target, WindowCommand::RestartPane)
        .unwrap_err();

    assert!(error.to_string().contains("window event proxy"));
    assert_eq!(app.snapshot, active_snapshot);
    assert_eq!(app.active_runtime_generation, active_generation);
    let inactive_runtime = app.pane_runtimes.get(&inactive).unwrap();
    assert_eq!(snapshot_char(&inactive_runtime.snapshot, 0, 0), None);
    assert_ne!(inactive_runtime.runtime_generation, 0);
}

#[test]
fn pane_runtime_close_drops_writer_before_joining_reader() {
    let app = NativeWindowApp::new(None);
    let mut runtime = app.new_inactive_pane_runtime();
    let writer_dropped = Arc::new(AtomicUsize::new(0));
    let reader_saw_writer_drop = Arc::new(AtomicBool::new(false));
    runtime.writer = Some(Box::new(DropTrackingWriter(Arc::clone(&writer_dropped))));
    let observed_writer = Arc::clone(&writer_dropped);
    let observed_order = Arc::clone(&reader_saw_writer_drop);
    let (started_sender, started_receiver) = std::sync::mpsc::channel();
    runtime.reader_thread = Some(std::thread::spawn(move || {
        started_sender.send(()).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while std::time::Instant::now() < deadline {
            if observed_writer.load(Ordering::Acquire) == 1 {
                observed_order.store(true, Ordering::Release);
                return;
            }
            std::thread::yield_now();
        }
    }));
    started_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("reader close observer did not start");

    runtime.close();

    assert_eq!(writer_dropped.load(Ordering::Acquire), 1);
    assert!(reader_saw_writer_drop.load(Ordering::Acquire));
}

#[test]
fn real_pty_close_lifecycle_finishes_within_timeout() {
    let app = NativeWindowApp::new(None);
    let mut runtime = app.new_inactive_pane_runtime();
    let command = if cfg!(windows) {
        PtyCommand::new("cmd.exe").with_args(["/D", "/C", "ping -n 30 127.0.0.1 >NUL"])
    } else {
        PtyCommand::new("/bin/sh").with_args(["-lc", "sleep 30"])
    };
    let mut session = PtySession::spawn(&command, PtySize::try_new(80, 24).unwrap()).unwrap();
    let mut reader = session.take_reader().unwrap();
    runtime.writer = Some(Box::new(session.take_writer().unwrap()));
    runtime.session_process_id = session.process_id();
    assert!(runtime.session_process_id.is_some_and(|pid| pid > 0));
    runtime.session = Some(session);
    runtime.reader_thread = Some(std::thread::spawn(move || {
        let mut buffer = [0_u8; 512];
        while let Ok(count) = reader.read(&mut buffer) {
            if count == 0 {
                break;
            }
        }
    }));
    let (sender, receiver) = std::sync::mpsc::channel();

    let close_thread = std::thread::spawn(move || {
        runtime.close();
        let _ = sender.send((runtime.session.is_none(), runtime.reader_thread.is_none()));
    });

    let (session_reaped, reader_joined) = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("real PTY close lifecycle timed out");
    close_thread.join().expect("PTY close worker panicked");
    assert!(
        session_reaped,
        "PTY session was not reaped before close returned"
    );
    assert!(
        reader_joined,
        "PTY reader was not joined before close returned"
    );
}

fn assert_restart_applies_title_after_target_projection_reset(
    case_name: &str,
    restart_inactive: bool,
) {
    let mut app = NativeWindowApp::new(None);
    app.dispatch_app_action(AppAction::SplitPane {
        pane: app.active_pane_id(),
        direction: SplitDirection::Right,
        launch: Some(PaneLaunch::local("active")),
    })
    .unwrap();
    let target = if restart_inactive {
        rssh_core::PaneId::new(1)
    } else {
        app.active_pane_id()
    };
    app.dispatch_app_action(AppAction::SetPaneUserVar {
        pane: target,
        name: "STALE".to_owned(),
        value: "yes".to_owned(),
    })
    .unwrap();
    app.dispatch_app_action(AppAction::SetPaneBadgeFormat {
        pane: target,
        badge_format: Some("stale badge".to_owned()),
    })
    .unwrap();
    app.dispatch_app_action(AppAction::SetPaneProgress {
        pane: target,
        progress: PaneProgress::Percentage(87),
    })
    .unwrap();
    app.dispatch_app_action(AppAction::SetPaneHasUnseenOutput {
        pane: target,
        has_unseen_output: true,
    })
    .unwrap();
    let formatter_observations = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&formatter_observations);
    app.window_title_formatter = Box::new(move |event| {
        let pane = event
            .panes
            .iter()
            .find(|pane| pane.pane_id == target)
            .expect("target pane must be present in formatter event");
        let clean = pane.user_vars.is_empty()
            && pane.progress == PaneProgress::None
            && !pane.has_unseen_output;
        recorded.lock().unwrap().push(clean);
        Some(if clean { "clean" } else { "stale" }.to_owned())
    });
    app.clear_applied_window_titles_for_test();

    app.restart_pane_runtime_with(target, |app, _| {
        Ok::<_, Box<dyn std::error::Error>>(app.new_inactive_pane_runtime())
    })
    .unwrap();

    assert_eq!(
        formatter_observations.lock().unwrap().last().copied(),
        Some(true),
        "restart case {case_name} observed stale formatter state"
    );
    assert_eq!(
        app.applied_window_titles_for_test()
            .last()
            .map(String::as_str),
        Some("clean"),
        "restart case {case_name} applied the wrong title"
    );
    assert_eq!(
        app.pane_badge_format(target),
        None,
        "restart case {case_name} retained a stale badge"
    );
}

#[test]
fn restart_applies_title_after_target_projection_reset() {
    for (case_name, restart_inactive) in [("active", false), ("inactive", true)] {
        assert_restart_applies_title_after_target_projection_reset(case_name, restart_inactive);
    }
}

#[test]
fn pane_spawn_pty_and_terminal_runtime_use_same_target_dimensions() {
    let mut app = NativeWindowApp::new(None);
    app.dispatch_app_action(AppAction::SplitPane {
        pane: app.active_pane_id(),
        direction: SplitDirection::Right,
        launch: Some(PaneLaunch::local("active")),
    })
    .unwrap();
    let active = app.active_pane_id();
    let inactive = rssh_core::PaneId::new(1);
    app.runtime.resize(rssh_core::TerminalSize::new(91, 27));
    let inactive_runtime = app.pane_runtimes.get_mut(&inactive).unwrap();
    inactive_runtime
        .runtime
        .resize(rssh_core::TerminalSize::new(37, 9));

    for (pane, expected) in [
        (active, rssh_core::TerminalSize::new(91, 27)),
        (inactive, rssh_core::TerminalSize::new(37, 9)),
    ] {
        let (pty_size, terminal_runtime) = app.prepare_pane_spawn_runtime_for_test(pane).unwrap();
        assert_eq!(pty_size.columns(), expected.columns);
        assert_eq!(pty_size.rows(), expected.rows);
        assert_eq!(terminal_runtime.terminal().grid().size(), expected);
    }
}

fn snapshot_char(
    snapshot: &rterm_render_core::TerminalRenderSnapshot,
    row: u16,
    column: u16,
) -> Option<char> {
    snapshot
        .cells()
        .iter()
        .find(|cell| cell.row == row && cell.column == column)
        .map(|cell| cell.ch)
}

fn snapshot_row_text(
    snapshot: &rterm_render_core::TerminalRenderSnapshot,
    row: u16,
    columns: u16,
) -> String {
    (0..columns)
        .map(|column| snapshot_char(snapshot, row, column).unwrap_or(' '))
        .collect()
}

struct DropTrackingWriter(Arc<AtomicUsize>);

impl Write for DropTrackingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for DropTrackingWriter {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}
