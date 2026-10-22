#[cfg(feature = "functional-test-observer")]
impl NativeWindowApp {
    fn functional_observer_snapshot(&self) -> rterm_functional_tests::ObserverSnapshotV1 {
        let terminal = self.runtime.terminal();
        let (cursor_row, cursor_column) = terminal.cursor();
        let layout = self.pane_render_layout();
        let active_tab_id = self.app_shell.active_tab_id().get();
        let active_pane_id = self.app_shell.active_pane_id().get();
        let panes = functional_observer_panes(layout, active_tab_id, active_pane_id);
        let modes = BTreeMap::from([
            (
                "application_cursor_keys".to_owned(),
                self.runtime.application_cursor_keys(),
            ),
            (
                "application_keypad".to_owned(),
                self.runtime.application_keypad(),
            ),
            (
                "bracketed_paste".to_owned(),
                self.runtime.bracketed_paste(),
            ),
            ("focus_reporting".to_owned(), self.runtime.focus_reporting()),
            (
                "mouse_reporting".to_owned(),
                self.runtime.mouse_input_mode().reporting().is_enabled(),
            ),
            ("win32_input".to_owned(), self.runtime.win32_input_mode()),
        ]);
        let live_runtime_threads = self
            .runtime
            .worker()
            .map_or(0, WindowPaneRuntime::live_thread_count_for_metrics);
        let legacy_threads = usize::from(self.reader_thread.is_some())
            .saturating_add(usize::from(self.writer_thread.is_some()));
        let pane_threads = self
            .pane_runtimes
            .values()
            .map(|runtime| {
                usize::from(runtime.reader_thread.is_some())
                    .saturating_add(usize::from(runtime.writer_thread.is_some()))
            })
            .sum::<usize>();
        let worker_count = live_runtime_threads
            .saturating_add(legacy_threads)
            .saturating_add(pane_threads);
        let child_process_count = usize::from(self.session_process_id.is_some()).saturating_add(
            self.pane_runtimes
                .values()
                .filter(|runtime| runtime.session_process_id.is_some())
                .count(),
        );
        let transport_state = if worker_count > 0 || child_process_count > 0 {
            "connected"
        } else if self.rendered_frames == 0 {
            "starting"
        } else {
            "closed"
        };

        rterm_functional_tests::ObserverSnapshotV1 {
            schema: 1,
            revision: 0,
            config_generation: 0,
            config_diagnostic_present: false,
            terminal: rterm_functional_tests::TerminalObservationV1 {
                text: functional_observer_terminal_text(terminal),
                cursor_row: u32::from(cursor_row),
                cursor_column: u32::from(cursor_column),
                modes,
            },
            window: rterm_functional_tests::WindowObservationV1 {
                width: self.window_frame.width,
                height: self.window_frame.height,
                active_tab_id: Some(active_tab_id),
                active_pane_id: Some(active_pane_id),
                overlay: functional_observer_overlay(self),
                panes,
            },
            runtime: rterm_functional_tests::RuntimeObservationV1 {
                transport_state: transport_state.to_owned(),
                effects: Vec::new(),
                render_digest: Some(format!(
                    "sha256:{}",
                    functional_observer_hex(
                        &rterm_render_cpu::terminal_first_row_pixel_digest(&self.snapshot)
                    )
                )),
                worker_count: u32::try_from(worker_count).unwrap_or(u32::MAX),
                listener_count: 0,
                child_process_count: u32::try_from(child_process_count).unwrap_or(u32::MAX),
            },
        }
    }
}

#[cfg(feature = "functional-test-observer")]
fn functional_observer_panes(
    layout: PaneRenderLayout,
    active_tab_id: u64,
    active_pane_id: u64,
) -> Vec<rterm_functional_tests::PaneObservationV1> {
    layout
        .panes
        .into_iter()
        .map(|pane| rterm_functional_tests::PaneObservationV1 {
            tab_id: active_tab_id,
            pane_id: pane.pane_id.get(),
            active: pane.pane_id.get() == active_pane_id,
            row: u32::from(pane.row),
            column: u32::from(pane.column),
            rows: u32::from(pane.rows),
            columns: u32::from(pane.columns),
        })
        .collect()
}

#[cfg(feature = "functional-test-observer")]
fn functional_observer_terminal_text(terminal: &Terminal) -> String {
    let grid = terminal.grid();
    let size = grid.size();
    let mut output = String::new();
    for row in 0..size.rows {
        let row_start = output.len();
        for column in 0..size.columns {
            let Some(cell) = grid.get(row, column) else {
                continue;
            };
            if cell.is_continuation() {
                continue;
            }
            output.push_str(cell.text());
        }
        while output.len() > row_start && output.ends_with(' ') {
            output.pop();
        }
        if row + 1 < size.rows {
            output.push('\n');
        }
    }
    output
}

#[cfg(feature = "functional-test-observer")]
fn functional_observer_overlay(app: &NativeWindowApp) -> Option<String> {
    if app.command_palette.is_some() {
        Some("command_palette".to_owned())
    } else if app.pane_select.is_some() {
        Some("pane_select".to_owned())
    } else if app.char_select.is_some() {
        Some("char_select".to_owned())
    } else if app.confirmation.is_some() {
        Some("confirmation".to_owned())
    } else if app.input_selector.is_some() {
        Some("input_selector".to_owned())
    } else if app.prompt_input_line.is_some() {
        Some("prompt_input_line".to_owned())
    } else {
        None
    }
}

#[cfg(feature = "functional-test-observer")]
fn functional_observer_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut encoded = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

#[cfg(all(test, feature = "functional-test-observer"))]
mod functional_observer_tests {
    use super::*;

    #[test]
    fn snapshot_projects_terminal_layout_modes_and_lifecycle_without_credentials() {
        let mut app = NativeWindowApp::new(None);
        app.handle_pty_output(b"visible\x1b[?2004h").unwrap();

        let snapshot = app.functional_observer_snapshot();

        assert!(snapshot.terminal.text.starts_with("visible"));
        assert_eq!(snapshot.terminal.cursor_column, 7);
        assert_eq!(snapshot.terminal.modes.get("bracketed_paste"), Some(&true));
        assert_eq!(snapshot.window.active_pane_id, Some(1));
        assert_eq!(snapshot.window.panes.len(), 1);
        assert!(snapshot.runtime.render_digest.as_deref().is_some_and(|value| {
            value.starts_with("sha256:") && value.len() == "sha256:".len() + 64
        }));
        let encoded = serde_json::to_string(&snapshot).unwrap();
        for forbidden in ["password", "private_key", "environment", "token"] {
            assert!(!encoded.contains(forbidden), "leaked {forbidden}: {encoded}");
        }
    }
}
