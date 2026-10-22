use super::{
    ActiveV2Close, AppAction, Error, NativeSshCommand, NativeWindowApp, PhysicalSize,
    PtyExitStatus, PtySize, RuntimeHostEvent, TerminalNotification, TerminalResizeOutcome,
    TerminalSize, terminal_progress_from_runtime, terminal_size_from_window_pixels_with_padding,
};

impl NativeWindowApp {
    pub(super) fn app_action_changes_pane_layout(action: &AppAction) -> bool {
        match action {
            AppAction::Multiple { actions } => {
                actions.iter().any(Self::app_action_changes_pane_layout)
            }
            _ => matches!(
                action,
                AppAction::SplitPane { .. }
                    | AppAction::SplitPaneWithSize { .. }
                    | AppAction::SplitTopLevelPane { .. }
                    | AppAction::SplitTopLevelPaneWithSize { .. }
                    | AppAction::ResizePane { .. }
                    | AppAction::SetPaneZoomState { .. }
                    | AppAction::TogglePaneZoom { .. }
            ),
        }
    }

    pub(super) fn poll_active_v2_runtime(&mut self) -> Result<Option<bool>, Box<dyn Error>> {
        let active_pane = self.app_shell.active_pane_id();
        let Some(runtime) = self.runtime.worker_mut() else {
            return Ok(None);
        };
        let active_token = runtime.token_for_pane(active_pane);
        let events = runtime.poll()?;
        let mut closed = ActiveV2Close::Open;
        for event in events {
            self.apply_active_v2_event(active_token, event, &mut closed)?;
        }

        let ActiveV2Close::Closed { pane, exit } = closed else {
            return Ok(None);
        };
        let has_remaining_panes = self
            .runtime
            .worker()
            .is_some_and(|runtime| !runtime.pane_tokens().is_empty());
        if !has_remaining_panes && let Some(mut runtime) = self.runtime.take_worker() {
            runtime.shutdown();
        }
        self.session_process_id = None;
        self.session_tty_name = None;
        self.active_runtime_generation = 0;
        let status = exit
            .and_then(|exit| exit.status)
            .map(PtyExitStatus::from_exit_code);
        let close_window = self.apply_pane_exit_behavior_after_exit(pane.pane(), status);
        if has_remaining_panes {
            let active = self.app_shell.active_pane_id();
            if let Some(runtime) = self.runtime.worker_mut()
                && runtime
                    .pane_tokens()
                    .iter()
                    .any(|token| token.pane() == active)
            {
                runtime.activate_pane(active)?;
            }
        }
        Ok(Some(
            self.defer_automatic_close_for_frame_limit(close_window),
        ))
    }

    fn apply_active_v2_event(
        &mut self,
        active_token: Option<rterm_runtime::PaneToken>,
        event: RuntimeHostEvent,
        closed: &mut ActiveV2Close,
    ) -> Result<(), Box<dyn Error>> {
        match event {
            RuntimeHostEvent::Frame {
                pane,
                terminal,
                damage,
                metadata,
                metrics,
                full_repaint,
                ..
            } if Some(pane) == active_token => {
                self.apply_active_v2_frame(terminal, &damage, metadata, metrics, full_repaint);
            }
            RuntimeHostEvent::Frame {
                pane,
                terminal,
                damage,
                metadata,
                metrics,
                ..
            } => {
                self.apply_inactive_v2_frame(pane, terminal, &damage, &metadata, metrics);
            }
            RuntimeHostEvent::HostStream { pane, bytes } if Some(pane) == active_token => {
                #[cfg(feature = "functional-test-observer")]
                crate::functional_observer::record_effect("host_stream");
                self.metrics.record_pty_chunk(&bytes);
                self.metrics.record_active_pty_content(&bytes);
            }
            RuntimeHostEvent::VisibleOutput { pane, bytes } if Some(pane) == active_token => {
                #[cfg(feature = "functional-test-observer")]
                crate::functional_observer::record_effect("visible_output");
                self.write_session_log(&bytes)?;
            }
            RuntimeHostEvent::ModeChange { pane, change } if Some(pane) == active_token => {
                #[cfg(feature = "functional-test-observer")]
                crate::functional_observer::record_effect("mode_change");
                self.runtime.inner.install_presentation_mode_change(change);
            }
            RuntimeHostEvent::InputWriteCompleted {
                byte_count,
                elapsed,
            } => self.handle_pane_input_write_completed(byte_count, elapsed),
            RuntimeHostEvent::FirstPtyByte { observed_at } => {
                self.metrics.record_first_pty_byte_at(observed_at);
            }
            RuntimeHostEvent::Bell { pane, count } if Some(pane) == active_token => {
                #[cfg(feature = "functional-test-observer")]
                crate::functional_observer::record_effect("bell");
                self.record_pane_bells(pane.pane(), count);
                self.metrics.record_bells(count);
                self.dispatch_bells(pane.pane(), count);
            }
            RuntimeHostEvent::ClipboardWrite {
                pane,
                selection,
                contents,
            } if Some(pane) == active_token => {
                self.apply_v2_clipboard_write(selection.as_deref(), &contents);
            }
            RuntimeHostEvent::ClipboardRead { pane, selection } if Some(pane) == active_token => {
                self.apply_v2_clipboard_read(&selection)?;
            }
            RuntimeHostEvent::Notification { pane, title, body } if Some(pane) == active_token => {
                #[cfg(feature = "functional-test-observer")]
                crate::functional_observer::record_effect("notification");
                self.dispatch_notification(pane.pane(), &TerminalNotification { title, body });
            }
            RuntimeHostEvent::Diagnostic { pane, message } => {
                #[cfg(feature = "functional-test-observer")]
                crate::functional_observer::record_effect("diagnostic");
                self.record_unknown_escape_sequence_warning(
                    pane.map_or(
                        self.app_shell.active_pane_id(),
                        rterm_runtime::PaneToken::pane,
                    ),
                    &message,
                );
            }
            RuntimeHostEvent::RequestRedraw => {
                self.request_v2_redraw();
            }
            RuntimeHostEvent::Closed { pane, exit } if Some(pane) == active_token => {
                *closed = ActiveV2Close::Closed { pane, exit };
            }
            RuntimeHostEvent::Closed { pane, exit } => {
                let status = exit
                    .and_then(|exit| exit.status)
                    .map(PtyExitStatus::from_exit_code);
                let _ = self.apply_pane_exit_behavior_after_exit(pane.pane(), status);
            }
            RuntimeHostEvent::HostStream { .. }
            | RuntimeHostEvent::VisibleOutput { .. }
            | RuntimeHostEvent::ModeChange { .. }
            | RuntimeHostEvent::Bell { .. }
            | RuntimeHostEvent::ClipboardWrite { .. }
            | RuntimeHostEvent::ClipboardRead { .. }
            | RuntimeHostEvent::Notification { .. } => {}
        }
        Ok(())
    }

    fn apply_v2_clipboard_write(&mut self, selection: Option<&str>, contents: &str) {
        #[cfg(feature = "functional-test-observer")]
        crate::functional_observer::record_effect("clipboard_write");
        if self.allows_v2_clipboard_write(selection) {
            self.write_clipboard_text(contents);
        }
    }

    fn request_v2_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn apply_v2_clipboard_read(&mut self, selection: &str) -> Result<(), Box<dyn Error>> {
        #[cfg(feature = "functional-test-observer")]
        crate::functional_observer::record_effect("clipboard_read");
        if self.osc52_policy.allows_query() {
            self.answer_clipboard_query(selection)?;
        }
        Ok(())
    }

    fn apply_active_v2_frame(
        &mut self,
        terminal: std::sync::Arc<rterm_terminal::Terminal>,
        damage: &[rterm_core::DamageRegion],
        metadata: rterm_runtime::PaneMetadataDelta,
        metrics: rterm_runtime::RuntimeBatchMetrics,
        full_repaint: bool,
    ) {
        let previous_dimensions = self.runtime.terminal().stable_dimensions();
        self.runtime.install_presentation_snapshot(terminal);
        let dimensions = self.runtime.terminal().stable_dimensions();
        if dimensions.domain != previous_dimensions.domain
            || dimensions.viewport_rows != previous_dimensions.viewport_rows
        {
            self.retire_active_terminal_identity_state();
        }
        self.reconcile_active_terminal_mutation();
        self.apply_v2_metadata(metadata);
        self.metrics.record_damage(damage);
        if full_repaint {
            self.refresh_snapshot();
        } else {
            self.refresh_snapshot_after_terminal_damage(damage);
        }
        self.metrics.record_pty_chunk_process(
            metrics
                .parse_duration
                .saturating_add(metrics.snapshot_duration),
        );
        let snapshot_is_empty = self.snapshot.iter_cells().next().is_none();
        self.metrics.record_first_rendered_cell(snapshot_is_empty);
    }

    fn apply_inactive_v2_frame(
        &mut self,
        pane: rterm_runtime::PaneToken,
        terminal: std::sync::Arc<rterm_terminal::Terminal>,
        damage: &[rterm_core::DamageRegion],
        metadata: &rterm_runtime::PaneMetadataDelta,
        metrics: rterm_runtime::RuntimeBatchMetrics,
    ) {
        let pane_id = pane.pane();
        let Some(runtime) = self.pane_runtimes.get_mut(&pane_id) else {
            return;
        };
        let previous_dimensions = runtime.runtime.terminal().stable_dimensions();
        runtime.runtime.install_presentation_snapshot(terminal);
        let dimensions = runtime.runtime.terminal().stable_dimensions();
        if dimensions.domain != previous_dimensions.domain
            || dimensions.viewport_rows != previous_dimensions.viewport_rows
        {
            runtime.ui.retire_terminal_identity();
        }
        runtime.reconcile_terminal_mutation();
        let cwd = metadata
            .working_directory
            .is_some()
            .then(|| {
                runtime
                    .runtime
                    .terminal()
                    .current_working_dir()
                    .map(str::to_owned)
            })
            .flatten();
        let user_vars = (!metadata.user_vars.is_empty()).then(|| {
            runtime
                .runtime
                .terminal()
                .user_vars()
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect::<Vec<_>>()
        });
        let badge_format = metadata
            .badge_format
            .is_some()
            .then(|| runtime.runtime.terminal().badge_format().map(str::to_owned));
        let progress = metadata
            .progress
            .is_some()
            .then(|| runtime.runtime.progress());
        self.sync_pane_has_unseen_output_from_value(pane_id, true);
        if metadata.working_directory.is_some() {
            self.sync_pane_current_working_dir_from_value(pane_id, cwd);
        }
        if let Some(user_vars) = user_vars {
            self.sync_pane_user_vars_from_pairs(pane_id, user_vars);
        }
        if let Some(badge_format) = badge_format {
            self.sync_pane_badge_format_from_value(pane_id, badge_format);
        }
        if let Some(progress) = progress {
            self.sync_pane_progress_from_value(pane_id, progress);
        }
        self.metrics.record_damage(damage);
        self.metrics.record_pty_chunk_process(
            metrics
                .parse_duration
                .saturating_add(metrics.snapshot_duration),
        );
    }

    fn allows_v2_clipboard_write(&self, _selection: Option<&str>) -> bool {
        self.osc52_policy.allows_write()
    }

    fn apply_v2_metadata(&mut self, metadata: rterm_runtime::PaneMetadataDelta) {
        if metadata.working_directory.is_some() {
            self.sync_active_pane_current_working_dir_from_runtime();
        }
        if !metadata.user_vars.is_empty() {
            self.sync_active_pane_user_vars_from_runtime();
        }
        if metadata.badge_format.is_some() {
            self.sync_active_pane_badge_format_from_runtime();
        }
        if let Some(progress) = metadata.progress {
            let progress = match progress {
                rterm_runtime::MetadataChange::Set(progress) => progress,
                rterm_runtime::MetadataChange::Clear => rterm_runtime::RuntimeProgress::None,
            };
            self.sync_pane_progress_from_value(
                self.app_shell.active_pane_id(),
                terminal_progress_from_runtime(progress),
            );
        }
        if metadata.title.is_some() {
            self.sync_window_title_from_runtime();
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn handle_window_resize(
        &mut self,
        size: PhysicalSize<u32>,
    ) -> Result<(), Box<dyn Error>> {
        self.resize_presentation_surface(size)?;
        if self.window.is_some() {
            self.refresh_window_frame_from_window();
        } else {
            self.window_frame.set_size(size);
        }

        let chrome_height =
            u32::from(self.workspace_chrome_frame_rows()).saturating_mul(self.cell_height());
        let terminal_size = terminal_size_from_window_pixels_with_padding(
            size.width,
            size.height.saturating_sub(chrome_height),
            self.cell_width(),
            self.cell_height(),
            self.window_padding,
            self.window_dpi,
        );
        let old_terminal_size = self.window_terminal_size_for_layout();
        let split_resize = self
            .app_shell
            .active_tab()
            .panes()
            .first()
            .map(rterm_core::app_shell::Pane::id)
            .map(|root_pane_id| {
                let old_root =
                    self.padded_terminal_render_rect_for_size(root_pane_id, old_terminal_size);
                let new_root =
                    self.padded_terminal_render_rect_for_size(root_pane_id, terminal_size);
                (
                    old_root.columns,
                    old_root.rows,
                    new_root.columns,
                    new_root.rows,
                )
            });
        self.frame_width = size.width;
        self.frame_height = size.height;

        if let Some((old_columns, old_rows, new_columns, new_rows)) = split_resize {
            self.app_shell.preserve_split_layout_for_resize(
                old_columns,
                old_rows,
                new_columns,
                new_rows,
            );
        }
        let pane_sizes = self.pane_terminal_sizes_for_layout(terminal_size);
        let active_resize_outcome =
            self.resize_terminal_runtimes_for_layout_with_sizes(terminal_size, &pane_sizes)?;
        self.refresh_snapshot_after_terminal_resize(
            active_resize_outcome == TerminalResizeOutcome::AlternateScreenResized,
        );
        if !self.uses_compat_full_window_runtime_model() {
            self.resize_legacy_transports_for_layout(&pane_sizes)?;
        }
        let resize = self.native_window_resize_event(size.width, size.height, terminal_size);
        self.dispatch_resize(&resize);
        Ok(())
    }

    pub(super) fn resize_terminal_runtimes(
        &mut self,
        terminal_size: TerminalSize,
    ) -> Result<TerminalResizeOutcome, Box<dyn Error>> {
        let active_height_changed =
            self.runtime.terminal().grid().size().rows != terminal_size.rows;
        let active_pane = self.app_shell.active_pane_id();
        let active_is_local_v2 = self
            .runtime
            .worker()
            .is_some_and(|runtime| runtime.contains_pane(active_pane));
        if let Some(runtime) = self.runtime.worker_mut() {
            runtime.resize_all(terminal_size)?;
        }
        let active_resize_outcome = if active_is_local_v2 {
            TerminalResizeOutcome::Unchanged
        } else {
            self.runtime.resize(terminal_size)
        };
        if active_height_changed {
            self.retire_active_terminal_identity_state();
        }
        if active_resize_outcome == TerminalResizeOutcome::MainScreenReflowed {
            self.reconcile_active_ui_after_main_screen_reflow();
        } else {
            self.reconcile_active_terminal_resize(
                active_resize_outcome == TerminalResizeOutcome::AlternateScreenResized,
            );
        }
        self.resize_inactive_terminal_runtimes(terminal_size);
        Ok(active_resize_outcome)
    }

    pub(super) fn sync_pane_runtimes_to_layout(&mut self) -> Result<(), Box<dyn Error>> {
        let terminal_size = self.window_terminal_size_for_layout();
        let pane_sizes = self.pane_terminal_sizes_for_layout(terminal_size);
        let active_resize_outcome =
            self.resize_terminal_runtimes_for_layout_with_sizes(terminal_size, &pane_sizes)?;
        if !self.uses_compat_full_window_runtime_model() {
            self.resize_legacy_transports_for_layout(&pane_sizes)?;
        }
        self.refresh_snapshot_after_terminal_resize(
            active_resize_outcome == TerminalResizeOutcome::AlternateScreenResized,
        );
        Ok(())
    }

    pub(super) fn sync_pane_runtimes_to_layout_logged(&mut self) {
        if let Err(error) = self.sync_pane_runtimes_to_layout() {
            eprintln!("pane runtime resize after shell action failed: {error}");
        }
    }

    pub(super) fn window_terminal_size_for_layout(&self) -> TerminalSize {
        if self.uses_compat_full_window_runtime_model() {
            return self.runtime.terminal().grid().size();
        }
        let chrome_height =
            u32::from(self.workspace_chrome_frame_rows()).saturating_mul(self.cell_height());
        terminal_size_from_window_pixels_with_padding(
            self.frame_width,
            self.frame_height.saturating_sub(chrome_height),
            self.cell_width(),
            self.cell_height(),
            self.window_padding,
            self.window_dpi,
        )
    }

    fn pane_terminal_sizes_for_layout(
        &self,
        terminal_size: TerminalSize,
    ) -> Vec<(rterm_core::PaneId, TerminalSize)> {
        self.pane_render_layout_for_tab_at_size(self.app_shell.active_tab(), terminal_size)
            .panes
            .into_iter()
            .map(|rect| (rect.pane_id, TerminalSize::new(rect.columns, rect.rows)))
            .collect()
    }

    fn resize_terminal_runtimes_for_layout_with_sizes(
        &mut self,
        terminal_size: TerminalSize,
        pane_sizes: &[(rterm_core::PaneId, TerminalSize)],
    ) -> Result<TerminalResizeOutcome, Box<dyn Error>> {
        if self.uses_compat_full_window_runtime_model() {
            let active_needs_resize = self.runtime.terminal().grid().size() != terminal_size;
            let inactive_needs_resize = self
                .pane_runtimes
                .values()
                .any(|runtime| runtime.runtime.terminal().grid().size() != terminal_size);
            if !active_needs_resize && !inactive_needs_resize {
                return Ok(TerminalResizeOutcome::Unchanged);
            }
            return self.resize_terminal_runtimes(terminal_size);
        }
        let active_pane = self.app_shell.active_pane_id();
        let active_size = pane_sizes
            .iter()
            .find(|(pane, _)| *pane == active_pane)
            .map_or(terminal_size, |(_, size)| *size);
        let active_height_changed = self.runtime.terminal().grid().size().rows != active_size.rows;
        let active_is_local_v2 = self
            .runtime
            .worker()
            .is_some_and(|runtime| runtime.contains_pane(active_pane));
        if let Some(runtime) = self.runtime.worker_mut() {
            runtime.resize_panes(pane_sizes.iter().copied())?;
        }
        let active_resize_outcome = if active_is_local_v2 {
            TerminalResizeOutcome::Unchanged
        } else {
            self.runtime.resize(active_size)
        };
        if active_height_changed {
            self.retire_active_terminal_identity_state();
        }
        if active_resize_outcome == TerminalResizeOutcome::MainScreenReflowed {
            self.reconcile_active_ui_after_main_screen_reflow();
        } else {
            self.reconcile_active_terminal_resize(
                active_resize_outcome == TerminalResizeOutcome::AlternateScreenResized,
            );
        }
        self.resize_inactive_terminal_runtimes_for_layout(pane_sizes, terminal_size);
        Ok(active_resize_outcome)
    }

    #[cfg(test)]
    fn has_live_pane_runtime_owner(&self) -> bool {
        self.runtime.worker().is_some()
            || self.active_runtime_transport.is_some()
            || self
                .pane_runtimes
                .values()
                .any(|runtime| runtime.transport.is_some())
    }

    #[cfg_attr(
        not(test),
        expect(
            clippy::unused_self,
            reason = "only headless compatibility fixtures use the synthetic model"
        )
    )]
    fn uses_compat_full_window_runtime_model(&self) -> bool {
        #[cfg(test)]
        {
            self.legacy_test_geometry
                && self.window.is_none()
                && !self.has_live_pane_runtime_owner()
        }
        #[cfg(not(test))]
        {
            false
        }
    }

    fn resize_inactive_terminal_runtimes_for_layout(
        &mut self,
        pane_sizes: &[(rterm_core::PaneId, TerminalSize)],
        fallback_size: TerminalSize,
    ) {
        for (pane_id, runtime) in &mut self.pane_runtimes {
            let size = pane_sizes
                .iter()
                .find(|(pane, _)| pane == pane_id)
                .map_or(fallback_size, |(_, size)| *size);
            let height_changed = runtime.runtime.terminal().grid().size().rows != size.rows;
            let resize_outcome = runtime.runtime.resize(size);
            if height_changed {
                runtime.ui.retire_terminal_identity();
            }
            if resize_outcome == TerminalResizeOutcome::MainScreenReflowed {
                runtime.reconcile_after_main_screen_reflow();
            } else {
                runtime.reconcile_terminal_resize(
                    resize_outcome == TerminalResizeOutcome::AlternateScreenResized,
                );
            }
        }
    }

    fn resize_legacy_transports_for_layout(
        &mut self,
        pane_sizes: &[(rterm_core::PaneId, TerminalSize)],
    ) -> Result<(), Box<dyn Error>> {
        let active_pane = self.app_shell.active_pane_id();
        for (pane_id, size) in pane_sizes {
            let pty_size = PtySize::try_new(size.columns, size.rows)?;
            if *pane_id == active_pane {
                if let Some(session) = self.session.as_mut() {
                    session.resize(pty_size)?;
                }
            } else if let Some(runtime) = self.pane_runtimes.get_mut(pane_id)
                && let Some(session) = runtime.session.as_mut()
            {
                session.resize(pty_size)?;
            }
            if let Some(sender) = self.ssh_writer_senders.get(pane_id) {
                let _ = sender.send(NativeSshCommand::Resize(*size));
            }
        }
        Ok(())
    }

    fn resize_inactive_terminal_runtimes(&mut self, terminal_size: TerminalSize) {
        for runtime in self.pane_runtimes.values_mut() {
            let height_changed =
                runtime.runtime.terminal().grid().size().rows != terminal_size.rows;
            let resize_outcome = runtime.runtime.resize(terminal_size);
            if height_changed {
                runtime.ui.retire_terminal_identity();
            }
            if resize_outcome == TerminalResizeOutcome::MainScreenReflowed {
                runtime.reconcile_after_main_screen_reflow();
            } else {
                runtime.reconcile_terminal_resize(
                    resize_outcome == TerminalResizeOutcome::AlternateScreenResized,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{self, Write},
        sync::{Arc, Mutex},
    };

    use rterm_runtime::testing::{
        ReadAction, ScriptedSessionDriver, ScriptedTransport, WriteAction,
    };

    use super::super::*;

    struct SharedWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn mixed_transport_app_with_local_worker() -> (
        NativeWindowApp,
        ScriptedSessionDriver,
        rterm_core::PaneId,
        rterm_core::PaneId,
    ) {
        let mut app = NativeWindowApp::new(None);
        let local_pane = app.active_pane_id();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: local_pane,
            direction: SplitDirection::Right,
            launch: Some(PaneLaunch::ssh(SshPaneLaunch::new(
                "ops@example.test:2222",
                SshAuthDescription::PasswordPrompt,
                SshKnownHostsPolicy::Prompt,
            ))),
        })
        .unwrap();
        let ssh_pane = app.active_pane_id();
        app.pane_runtimes.get_mut(&local_pane).unwrap().transport =
            Some(PaneRuntimeTransportKind::LocalPty);
        app.active_runtime_transport = Some(PaneRuntimeTransportKind::NativeSsh);
        let size = app.runtime.terminal().grid().size();
        let (local_transport, local_driver) =
            ScriptedTransport::new([ReadAction::Block], [WriteAction::accept(usize::MAX)], []);
        let worker = WindowPaneRuntime::open_transport(
            PaneRuntimeRoute {
                window: app.app_window_id,
                pane: local_pane,
            },
            local_transport,
            size,
            rterm_runtime::TerminalRuntime::new(size),
            PaneCapturePolicy {
                host_stream: false,
                visible_output: false,
            },
            Arc::new(|| {}),
        )
        .expect("local worker");
        app.runtime.install_worker(Some(worker));
        (app, local_driver, local_pane, ssh_pane)
    }

    #[test]
    fn mixed_local_and_ssh_input_is_routed_by_pane_id_without_cross_talk() {
        let mut app = NativeWindowApp::new(None);
        let local_pane = app.active_pane_id();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: local_pane,
            direction: SplitDirection::Right,
            launch: Some(PaneLaunch::ssh(SshPaneLaunch::new(
                "ops@example.test:2222",
                SshAuthDescription::PasswordPrompt,
                SshKnownHostsPolicy::Prompt,
            ))),
        })
        .unwrap();
        let ssh_pane = app.active_pane_id();
        app.pane_runtimes.get_mut(&local_pane).unwrap().transport =
            Some(PaneRuntimeTransportKind::LocalPty);
        app.active_runtime_transport = Some(PaneRuntimeTransportKind::NativeSsh);
        let ssh_bytes = Arc::new(Mutex::new(Vec::new()));
        app.writer = Some(Box::new(SharedWriter(Arc::clone(&ssh_bytes))));

        let size = app.runtime.terminal().grid().size();
        let (local_transport, local_driver) =
            ScriptedTransport::new([ReadAction::Block], [WriteAction::accept(usize::MAX)], []);
        let worker = WindowPaneRuntime::open_transport(
            PaneRuntimeRoute {
                window: app.app_window_id,
                pane: local_pane,
            },
            local_transport,
            size,
            rterm_runtime::TerminalRuntime::new(size),
            PaneCapturePolicy {
                host_stream: false,
                visible_output: false,
            },
            Arc::new(|| {}),
        )
        .expect("local worker");
        app.runtime.install_worker(Some(worker));

        app.write_pty_bytes(b"ssh-only").unwrap();
        assert_eq!(&*ssh_bytes.lock().unwrap(), b"ssh-only");
        assert!(local_driver.accepted_writes().is_empty());

        app.write_pty_bytes_to_pane(local_pane, b"local-only")
            .unwrap();
        local_driver.wait_until_accepted_write_len("local-only".len());
        assert_eq!(local_driver.accepted_writes(), b"local-only");
        assert_eq!(app.active_pane_id(), ssh_pane);
        assert_eq!(&*ssh_bytes.lock().unwrap(), b"ssh-only");
    }

    #[test]
    fn mixed_transport_resize_updates_ssh_presentation_and_local_worker() {
        let (mut app, local_driver, local_pane, ssh_pane) = mixed_transport_app_with_local_worker();
        let resized = TerminalSize::new(111, 37);

        let outcome = app
            .resize_terminal_runtimes(resized)
            .expect("resize mixed transports");

        local_driver.wait_until_control_call_count(1);
        assert_eq!(local_driver.control_log().resizes, vec![resized]);
        assert_eq!(app.active_pane_id(), ssh_pane);
        assert_eq!(app.runtime.terminal().grid().size(), resized);
        assert_eq!(
            app.pane_runtimes
                .get(&local_pane)
                .unwrap()
                .runtime
                .terminal()
                .grid()
                .size(),
            resized
        );
        assert_ne!(outcome, TerminalResizeOutcome::Unchanged);
    }

    #[test]
    fn split_layout_resize_sends_each_local_worker_pane_dimensions() {
        let (mut app, local_driver, local_pane, _ssh_pane) =
            mixed_transport_app_with_local_worker();
        let rect = app
            .pane_render_rect(local_pane)
            .expect("local split pane has a render rectangle");
        let expected = TerminalSize::new(rect.columns, rect.rows);

        app.sync_pane_runtimes_to_layout()
            .expect("resize split pane runtime to its render rectangle");
        local_driver.wait_until_control_call_count(1);

        assert_eq!(local_driver.control_log().resizes, vec![expected]);
        assert_eq!(
            app.pane_runtimes
                .get(&local_pane)
                .expect("local pane runtime")
                .runtime
                .terminal()
                .grid()
                .size(),
            expected
        );
    }

    #[test]
    fn compat_full_window_runtime_model_does_not_follow_synthetic_split_rect() {
        let mut app = NativeWindowApp::new(None);
        let root_pane = app.active_pane_id();
        let full_window_size = app.runtime.terminal().grid().size();

        app.dispatch_app_action(AppAction::SplitPane {
            pane: root_pane,
            direction: SplitDirection::Right,
            launch: None,
        })
        .expect("split synthetic compatibility pane");
        app.sync_pane_runtimes_to_layout()
            .expect("sync synthetic compatibility panes");

        assert_eq!(app.window_terminal_size_for_layout(), full_window_size);
        assert_eq!(app.runtime.terminal().grid().size(), full_window_size);
        assert_eq!(
            app.pane_runtimes
                .get(&root_pane)
                .expect("inactive synthetic pane runtime")
                .runtime
                .terminal()
                .grid()
                .size(),
            full_window_size
        );
    }

    #[test]
    fn stopping_active_ssh_keeps_the_inactive_local_worker_alive() {
        let (mut app, _local_driver, local_pane, ssh_pane) =
            mixed_transport_app_with_local_worker();

        app.stop_active_runtime();

        assert_eq!(app.active_pane_id(), ssh_pane);
        assert!(
            app.runtime
                .worker()
                .is_some_and(|runtime| runtime.contains_pane(local_pane))
        );
    }

    #[test]
    fn manager_shutdown_joins_window_scoped_local_workers_before_metrics() {
        let (app, local_driver, _local_pane, _ssh_pane) = mixed_transport_app_with_local_worker();
        local_driver.wait_until_reader_blocked();
        assert!(
            app.runtime
                .worker()
                .expect("installed local worker")
                .live_thread_count_for_metrics()
                > 0
        );
        let mut manager = NativeWindowManager::new_for_test(app);

        manager.shutdown_runtime_owners();

        let app = manager.primary_app_mut_for_test();
        assert!(app.runtime.worker().is_none());
        assert_eq!(app.metrics_snapshot().runtime_live_threads, 0);
    }

    #[test]
    fn final_window_close_joins_all_window_scoped_local_workers_before_retirement() {
        let (app, local_driver, _local_pane, _ssh_pane) = mixed_transport_app_with_local_worker();
        local_driver.wait_until_reader_blocked();
        let mut manager = NativeWindowManager::new_for_test(app);

        manager
            .finalize_app_close_at_location(ManagedWindowAppLocation::Startup)
            .expect("startup app remains manager-owned until retirement");

        let retired = manager
            .retired_apps
            .last()
            .expect("closed app is retained until the event-loop boundary");
        assert!(retired.runtime.worker().is_none());
        assert_eq!(retired.metrics_snapshot().runtime_live_threads, 0);
    }

    #[test]
    fn local_v2_frame_stays_inactive_while_an_ssh_pane_is_active() {
        let mut app = NativeWindowApp::new(None);
        app.set_initial_pane_launch(PaneLaunch::ssh(SshPaneLaunch::new(
            "ops@example.test:2222",
            SshAuthDescription::PasswordPrompt,
            SshKnownHostsPolicy::Prompt,
        )));
        app.handle_pty_output(b"ssh-visible").unwrap();
        let ssh_pane = app.active_pane_id();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: ssh_pane,
            direction: SplitDirection::Right,
            launch: Some(PaneLaunch::local("local-shell")),
        })
        .unwrap();
        let local_pane = app.active_pane_id();
        app.dispatch_app_action(AppAction::ActivatePane { pane: ssh_pane })
            .unwrap();
        let ssh_snapshot = app.snapshot.clone();

        let mut allocator = rterm_runtime::PaneTokenAllocator::new();
        let local_token = allocator.issue(local_pane).expect("local token");
        let size = app
            .pane_runtimes
            .get(&local_pane)
            .unwrap()
            .runtime
            .terminal()
            .grid()
            .size();
        let mut terminal = Terminal::new(size);
        terminal.feed(b"\x1b]2;local-v2-title\x07local-v2-body");
        let mut closed = ActiveV2Close::Open;

        app.apply_active_v2_event(
            None,
            RuntimeHostEvent::Frame {
                pane: local_token,
                terminal: Arc::new(terminal),
                damage: vec![rterm_core::DamageRegion::new(0, 0, 1, 1)],
                metadata: rterm_runtime::PaneMetadataDelta {
                    title: Some(rterm_runtime::MetadataChange::Set(
                        "local-v2-title".to_owned(),
                    )),
                    ..rterm_runtime::PaneMetadataDelta::default()
                },
                metrics: rterm_runtime::RuntimeBatchMetrics::default(),
                full_repaint: false,
            },
            &mut closed,
        )
        .expect("apply local V2 frame");

        assert_eq!(app.active_pane_id(), ssh_pane);
        assert_eq!(app.snapshot, ssh_snapshot);
        assert_eq!(
            app.pane_runtimes
                .get(&local_pane)
                .unwrap()
                .runtime
                .terminal()
                .window_title(),
            Some("local-v2-title")
        );
    }

    #[test]
    fn inactive_v2_frame_updates_its_owned_presentation_and_metadata() {
        let mut app = NativeWindowApp::new(None);
        let inactive = app.active_pane_id();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: inactive,
            direction: SplitDirection::Right,
            launch: Some(PaneLaunch::local("active")),
        })
        .unwrap();
        let active = app.active_pane_id();
        let mut allocator = rterm_runtime::PaneTokenAllocator::new();
        let inactive_token = allocator.issue(inactive).expect("inactive token");
        let active_token = allocator.issue(active).expect("active token");
        let size = app
            .pane_runtimes
            .get(&inactive)
            .unwrap()
            .runtime
            .terminal()
            .grid()
            .size();
        let mut terminal = Terminal::new(size);
        terminal.feed(b"\x1b]2;inactive-v2-title\x07inactive-v2-body");
        let mut closed = ActiveV2Close::Open;

        app.apply_active_v2_event(
            Some(active_token),
            RuntimeHostEvent::Frame {
                pane: inactive_token,
                terminal: Arc::new(terminal),
                damage: vec![rterm_core::DamageRegion::new(0, 0, 1, 1)],
                metadata: rterm_runtime::PaneMetadataDelta {
                    title: Some(rterm_runtime::MetadataChange::Set(
                        "inactive-v2-title".to_owned(),
                    )),
                    ..rterm_runtime::PaneMetadataDelta::default()
                },
                metrics: rterm_runtime::RuntimeBatchMetrics::default(),
                full_repaint: false,
            },
            &mut closed,
        )
        .expect("apply inactive V2 frame");

        let inactive_runtime = app.pane_runtimes.get(&inactive).unwrap();
        assert_eq!(
            inactive_runtime.runtime.terminal().window_title(),
            Some("inactive-v2-title")
        );
        assert_eq!(app.active_pane_id(), active);
        assert!(matches!(closed, ActiveV2Close::Open));
    }

    #[test]
    fn inactive_v2_close_does_not_close_the_active_runtime_owner() {
        let mut app = NativeWindowApp::new(None);
        let inactive = app.active_pane_id();
        app.dispatch_app_action(AppAction::SplitPane {
            pane: inactive,
            direction: SplitDirection::Right,
            launch: Some(PaneLaunch::local("active")),
        })
        .unwrap();
        let active = app.active_pane_id();
        let mut allocator = rterm_runtime::PaneTokenAllocator::new();
        let inactive_token = allocator.issue(inactive).expect("inactive token");
        let active_token = allocator.issue(active).expect("active token");
        let mut closed = ActiveV2Close::Open;

        app.apply_active_v2_event(
            Some(active_token),
            RuntimeHostEvent::Closed {
                pane: inactive_token,
                exit: Some(rterm_runtime::SessionExit {
                    status: Some(0),
                    signal: None,
                }),
            },
            &mut closed,
        )
        .expect("apply inactive close");

        assert_eq!(app.active_pane_id(), active);
        assert_eq!(app.app_shell.pane_ids(), vec![active]);
        assert!(matches!(closed, ActiveV2Close::Open));
    }
}
