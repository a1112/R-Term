impl NativeWindowApp {
    fn has_active_text_blink_animation(&self) -> bool {
        let regular_blink_active = !self.text_blink_rate.is_zero();
        let rapid_blink_active = !self.text_blink_rate_rapid.is_zero();
        if !regular_blink_active && !rapid_blink_active {
            return false;
        }

        self.pane_snapshots().any(|snapshot| {
            snapshot_has_active_text_blink(snapshot, regular_blink_active, rapid_blink_active)
        })
    }

    fn has_active_visual_bell_at(&self, now: Instant) -> bool {
        if !self.visual_bell.is_enabled() {
            return false;
        }

        self.visual_bell_started_at.values().any(|started| {
            let Some(elapsed) = now.checked_duration_since(*started) else {
                return true;
            };
            visual_bell_intensity(self.visual_bell, elapsed).is_some()
        })
    }

    fn expire_visual_bells_if_due(&mut self, now: Instant) -> bool {
        if self.visual_bell_started_at.is_empty() {
            return false;
        }

        if !self.visual_bell.is_enabled() {
            self.visual_bell_started_at.clear();
            self.frame_needs_full_repaint = true;
            return true;
        }

        let visual_bell = self.visual_bell;
        let before = self.visual_bell_started_at.len();
        self.visual_bell_started_at.retain(|_, started| {
            let Some(elapsed) = now.checked_duration_since(*started) else {
                return true;
            };
            visual_bell_intensity(visual_bell, elapsed).is_some()
        });

        if self.visual_bell_started_at.len() == before {
            return false;
        }

        self.frame_needs_full_repaint = true;
        true
    }

    fn has_active_inline_image_animation(&self) -> bool {
        self.pane_snapshots().any(|snapshot| {
            snapshot
                .inline_images()
                .iter()
                .any(inline_image_may_animate)
        })
    }

    fn pane_snapshots(&self) -> impl Iterator<Item = &TerminalRenderSnapshot> {
        std::iter::once(&self.snapshot)
            .chain(self.pane_runtimes.values().map(|runtime| &runtime.snapshot))
    }

    fn handle_pane_pty_output(
        &mut self,
        pane_id: rterm_core::PaneId,
        bytes: &[u8],
    ) -> io::Result<()> {
        if pane_id == self.app_shell.active_pane_id() {
            return self.handle_active_pane_output(bytes);
        }

        let Some(mut runtime) = self.pane_runtimes.remove(&pane_id) else {
            return Ok(());
        };

        let result = self.handle_inactive_pane_output(pane_id, &mut runtime, bytes);
        self.pane_runtimes.insert(pane_id, runtime);
        result
    }

    fn handle_active_pane_output(&mut self, bytes: &[u8]) -> io::Result<()> {
        let started = Instant::now();
        self.metrics.record_pty_chunk(bytes);
        self.metrics.record_active_pty_content(bytes);
        let previous_dimensions = self.runtime.terminal().stable_dimensions();
        let mut buffers = std::mem::take(&mut self.runtime.storage.buffers);
        let delta = self.runtime.inner.feed_into(bytes, &mut buffers);
        let result = self.apply_active_pane_delta(delta, previous_dimensions, Some(started));
        self.runtime.storage.buffers = buffers;
        result
    }

    fn apply_active_pane_delta(
        &mut self,
        delta: rterm_runtime::RuntimeDelta<'_>,
        previous_dimensions: rterm_terminal::TerminalStableDimensions,
        started: Option<Instant>,
    ) -> io::Result<()> {
        let dimensions = self.runtime.terminal().stable_dimensions();
        if delta.screen_identity_changed()
            || dimensions.domain != previous_dimensions.domain
            || dimensions.viewport_rows != previous_dimensions.viewport_rows
        {
            self.retire_active_terminal_identity_state();
        }
        self.reconcile_active_terminal_mutation();
        self.write_session_log(delta.visible_bytes())?;
        for message in delta.diagnostics() {
            #[cfg(feature = "functional-test-observer")]
            crate::functional_observer::record_effect("diagnostic");
            self.record_unknown_escape_sequence_warning(self.app_shell.active_pane_id(), message);
        }
        for response in delta.responses() {
            #[cfg(feature = "functional-test-observer")]
            crate::functional_observer::record_effect("transport_write");
            self.write_pty_bytes(response)?;
        }
        for (_, contents) in delta.clipboard_writes() {
            #[cfg(feature = "functional-test-observer")]
            crate::functional_observer::record_effect("clipboard_write");
            if self.osc52_policy.allows_write() {
                self.write_clipboard_text(contents);
            }
        }
        for selection in delta.clipboard_reads() {
            #[cfg(feature = "functional-test-observer")]
            crate::functional_observer::record_effect("clipboard_read");
            if self.osc52_policy.allows_query() {
                self.answer_clipboard_query(selection)?;
            }
        }
        for (title, body) in delta.notifications() {
            #[cfg(feature = "functional-test-observer")]
            crate::functional_observer::record_effect("notification");
            let notification = TerminalNotification {
                title: title.map(str::to_owned),
                body: body.to_owned(),
            };
            self.dispatch_notification(self.app_shell.active_pane_id(), &notification);
        }
        let metadata = delta.metadata();
        if metadata.working_directory().is_some()
            || (started.is_some() && self.runtime.terminal().current_working_dir().is_none())
        {
            self.sync_active_pane_current_working_dir_from_runtime();
        }
        if metadata.user_vars().next().is_some() {
            self.sync_active_pane_user_vars_from_runtime();
        }
        if metadata.badge_format().is_some() {
            self.sync_active_pane_badge_format_from_runtime();
        }
        if metadata.progress().is_some() {
            self.sync_active_pane_progress_from_runtime();
        }
        if metadata.title().is_some() {
            self.sync_window_title_from_runtime();
        }
        self.metrics.record_damage(delta.damage());
        self.refresh_snapshot_after_terminal_damage(delta.damage());
        self.record_pane_bells(self.app_shell.active_pane_id(), delta.bell_count());
        #[cfg(feature = "functional-test-observer")]
        if delta.bell_count() > 0 {
            crate::functional_observer::record_effect("bell");
        }
        self.metrics.record_bells(delta.bell_count());
        self.dispatch_bells(self.app_shell.active_pane_id(), delta.bell_count());
        let snapshot_is_empty = self.snapshot.iter_cells().next().is_none();
        self.metrics.record_first_rendered_cell(snapshot_is_empty);
        if let Some(started) = started {
            self.metrics.record_pty_chunk_process(started.elapsed());
        }
        Ok(())
    }

    fn finish_active_pane_output(&mut self) -> io::Result<()> {
        let previous_dimensions = self.runtime.terminal().stable_dimensions();
        let mut buffers = std::mem::take(&mut self.runtime.storage.buffers);
        let delta = self.runtime.inner.finish_into(&mut buffers);
        let result = self.apply_active_pane_delta(delta, previous_dimensions, None);
        self.runtime.storage.buffers = buffers;
        result
    }

    fn retire_active_terminal_identity_state(&mut self) {
        self.active_ui.retire_terminal_identity();
        self.selection = None;
        self.selecting = false;
        self.active_mouse_button = None;
        self.last_left_click = None;
        self.last_mouse_assignment_click = None;
    }

    fn reconcile_active_terminal_mutation(&mut self) {
        self.interaction_state
            .active_ui
            .reconcile_terminal_mutation(self.runtime.terminal());
        self.update_selection_projection();
    }

    fn reconcile_active_terminal_resize(&mut self, preserve_ordinary_selection: bool) {
        self.interaction_state
            .active_ui
            .reconcile_terminal_resize(self.runtime.terminal(), preserve_ordinary_selection);
        self.update_selection_projection();
    }

    fn reconcile_active_ui_after_main_screen_reflow(&mut self) {
        self.interaction_state
            .active_ui
            .reconcile_after_main_screen_reflow(self.runtime.terminal());
        self.update_selection_projection();
    }

    fn stable_source_cell_for_viewport_cell(&self, cell: SelectionCell) -> SelectionSourceCell {
        let terminal = self.runtime.terminal();
        let dimensions = terminal.stable_dimensions();
        let viewport_top = self
            .active_ui
            .stable_viewport
            .active_top(terminal)
            .unwrap_or(dimensions.physical_top);
        SelectionSourceCell {
            domain: dimensions.domain,
            row: viewport_top
                .saturating_add(StableRowIndex::try_from(cell.row).unwrap_or(StableRowIndex::MAX)),
            column: usize::from(cell.column),
        }
    }

    fn set_ordinary_selection(&mut self, selection: StableOrdinarySelection) {
        self.active_ui.ordinary_selection = Some(selection);
        self.update_selection_projection();
    }

    fn clear_ordinary_selection(&mut self) {
        self.active_ui.ordinary_selection = None;
        if self.active_ui.quick_select().is_none()
            && self.active_ui.retained_search().is_none()
            && self.active_ui.copy_mode().is_none()
        {
            self.selection = None;
        }
    }

    fn invalidate_active_ordinary_selection_for_presentation(&mut self) {
        let transient_overlay_active = self.active_ui.overlay_active();
        if !transient_overlay_active
            && ordinary_selection_is_invalidated_by_visible_dirty_rows(
                self.runtime.terminal(),
                self.active_ui
                    .stable_viewport
                    .active_top(self.runtime.terminal()),
                self.active_ui.ordinary_selection,
            )
        {
            self.clear_ordinary_selection();
        }
    }

    fn update_selection_projection(&mut self) -> bool {
        let terminal = self.runtime.terminal();
        let dimensions = terminal.stable_dimensions();
        let viewport_top = self
            .active_ui
            .stable_viewport
            .active_top(terminal)
            .unwrap_or(dimensions.physical_top);
        let size = terminal.grid().size();
        let transient_active = self.active_ui.overlay_active();
        self.selection = if let Some(quick_select) = self.active_ui.quick_select() {
            quick_select.current_match().and_then(|matched| {
                matched.viewport_selection_for_top(dimensions.domain, viewport_top, size)
            })
        } else {
            match self.active_ui.copy_search_mode() {
                Some(WindowCopySearchMode::Search) => self
                    .active_ui
                    .search()
                    .and_then(|search| search.current)
                    .and_then(|matched| {
                        matched.viewport_selection_for_top(dimensions.domain, viewport_top, size)
                    }),
                Some(WindowCopySearchMode::Copy) => self
                    .active_ui
                    .copy_mode()
                    .and_then(|copy_mode| {
                        copy_mode_source_selection(
                            copy_mode,
                            terminal,
                            &self.selection_word_boundary,
                        )
                    })
                    .and_then(|selection| {
                        selection.viewport_selection(dimensions.domain, viewport_top, size)
                    })
                    .or_else(|| {
                        self.active_ui
                            .retained_search()
                            .and_then(|search| search.current)
                            .and_then(|matched| {
                                matched.viewport_selection_for_top(
                                    dimensions.domain,
                                    viewport_top,
                                    size,
                                )
                            })
                    }),
                None => self.active_ui.ordinary_selection.and_then(|selection| {
                    selection.viewport_selection(dimensions.domain, viewport_top, size)
                }),
            }
        };
        transient_active
    }

    #[cfg(test)]
    fn update_transient_selection_projection(&mut self) -> bool {
        self.update_selection_projection()
    }

    fn handle_inactive_pane_output(
        &mut self,
        pane_id: rterm_core::PaneId,
        runtime: &mut PaneRuntime,
        bytes: &[u8],
    ) -> io::Result<()> {
        let started = Instant::now();
        self.metrics.record_pty_chunk(bytes);
        let previous_dimensions = runtime.runtime.terminal().stable_dimensions();
        let mut buffers = std::mem::take(&mut runtime.runtime.storage.buffers);
        let delta = runtime.runtime.inner.feed_into(bytes, &mut buffers);
        let result = self.apply_inactive_pane_delta(
            pane_id,
            runtime,
            delta,
            previous_dimensions,
            Some(!bytes.is_empty()),
            Some(started),
        );
        runtime.runtime.storage.buffers = buffers;
        result
    }

    fn apply_inactive_pane_delta(
        &mut self,
        pane_id: rterm_core::PaneId,
        runtime: &mut PaneRuntime,
        delta: rterm_runtime::RuntimeDelta<'_>,
        previous_dimensions: rterm_terminal::TerminalStableDimensions,
        has_unseen_output: Option<bool>,
        started: Option<Instant>,
    ) -> io::Result<()> {
        let dimensions = runtime.runtime.terminal().stable_dimensions();
        if delta.screen_identity_changed()
            || dimensions.domain != previous_dimensions.domain
            || dimensions.viewport_rows != previous_dimensions.viewport_rows
        {
            runtime.ui.retire_terminal_identity();
        }
        runtime
            .ui
            .reconcile_terminal_mutation(runtime.runtime.terminal());
        if delta.snapshot_changed() {
            runtime.snapshot =
                terminal_runtime_snapshot(&runtime.runtime, runtime.ui.stable_viewport);
        }
        self.apply_inactive_pane_host_effects(pane_id, runtime, &delta)?;
        if let Some(has_unseen_output) = has_unseen_output {
            self.sync_pane_has_unseen_output_from_value(pane_id, has_unseen_output);
        }
        let metadata = delta.metadata();
        if (metadata.working_directory().is_some()
            || (started.is_some() && runtime.runtime.terminal().current_working_dir().is_none()))
            && let PaneRuntimeCwdUpdate::Resolved(cwd) = pane_runtime_current_working_dir_if_due(
                &mut runtime.runtime,
                runtime.session_process_id,
                Instant::now(),
            )
        {
            self.sync_pane_current_working_dir_from_value(pane_id, cwd);
        }
        if metadata.user_vars().next().is_some() {
            self.sync_pane_user_vars_from_pairs(
                pane_id,
                runtime
                    .runtime
                    .terminal()
                    .user_vars()
                    .iter()
                    .map(|(name, value)| (name.clone(), value.clone()))
                    .collect(),
            );
        }
        if metadata.badge_format().is_some() {
            self.sync_pane_badge_format_from_value(
                pane_id,
                runtime.runtime.terminal().badge_format().map(str::to_owned),
            );
        }
        if metadata.progress().is_some() {
            self.sync_pane_progress_from_value(pane_id, runtime.runtime.progress());
        }
        self.record_pane_bells(pane_id, delta.bell_count());
        #[cfg(feature = "functional-test-observer")]
        if delta.bell_count() > 0 {
            crate::functional_observer::record_effect("bell");
        }
        self.metrics.record_bells(delta.bell_count());
        self.dispatch_bells(pane_id, delta.bell_count());
        let snapshot_is_empty = self.snapshot.iter_cells().next().is_none();
        self.metrics.record_first_rendered_cell(snapshot_is_empty);
        if let Some(started) = started {
            self.metrics.record_pty_chunk_process(started.elapsed());
        }
        Ok(())
    }

    fn apply_inactive_pane_host_effects(
        &mut self,
        pane_id: rterm_core::PaneId,
        runtime: &mut PaneRuntime,
        delta: &rterm_runtime::RuntimeDelta<'_>,
    ) -> io::Result<()> {
        for message in delta.diagnostics() {
            #[cfg(feature = "functional-test-observer")]
            crate::functional_observer::record_effect("diagnostic");
            self.record_unknown_escape_sequence_warning(pane_id, message);
        }
        for response in delta.responses() {
            #[cfg(feature = "functional-test-observer")]
            crate::functional_observer::record_effect("transport_write");
            if let Some(writer) = runtime.writer.as_mut() {
                let response_started = Instant::now();
                writer.write_all(response)?;
                writer.flush()?;
                self.metrics
                    .record_input_write(response.len(), response_started.elapsed());
            }
        }
        for (_, contents) in delta.clipboard_writes() {
            #[cfg(feature = "functional-test-observer")]
            crate::functional_observer::record_effect("clipboard_write");
            if self.osc52_policy.allows_write() {
                self.write_clipboard_text(contents);
            }
        }
        for selection in delta.clipboard_reads() {
            #[cfg(feature = "functional-test-observer")]
            crate::functional_observer::record_effect("clipboard_read");
            if self.osc52_policy.allows_query()
                && let Some(text) = self.read_clipboard_text()
            {
                let response = encode_osc52_clipboard_response(selection, &text);
                if let Some(writer) = runtime.writer.as_mut() {
                    let response_started = Instant::now();
                    writer.write_all(&response)?;
                    writer.flush()?;
                    self.metrics
                        .record_input_write(response.len(), response_started.elapsed());
                }
            }
        }
        for (title, body) in delta.notifications() {
            #[cfg(feature = "functional-test-observer")]
            crate::functional_observer::record_effect("notification");
            let notification = TerminalNotification {
                title: title.map(str::to_owned),
                body: body.to_owned(),
            };
            self.dispatch_notification(pane_id, &notification);
        }
        Ok(())
    }

    fn finish_inactive_pane_output(
        &mut self,
        pane_id: rterm_core::PaneId,
        runtime: &mut PaneRuntime,
    ) -> io::Result<()> {
        let previous_dimensions = runtime.runtime.terminal().stable_dimensions();
        let mut buffers = std::mem::take(&mut runtime.runtime.storage.buffers);
        let delta = runtime.runtime.inner.finish_into(&mut buffers);
        let result = self.apply_inactive_pane_delta(
            pane_id,
            runtime,
            delta,
            previous_dimensions,
            None,
            None,
        );
        runtime.runtime.storage.buffers = buffers;
        result
    }

    fn record_unknown_escape_sequence_warning(
        &mut self,
        pane_id: rterm_core::PaneId,
        sequence: &str,
    ) {
        if !self.log_unknown_escape_sequences {
            return;
        }
        let warning = format!(
            "WARN unknown escape sequence from pane {}: {sequence}",
            pane_id.get()
        );
        eprintln!("{warning}");
        self.unknown_escape_sequence_warnings.push(warning);
    }

    #[cfg(test)]
    fn handle_pty_output(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.handle_active_pane_output(bytes)
    }

    fn write_session_log(&mut self, bytes: &[u8]) -> io::Result<()> {
        if bytes.is_empty() {
            return Ok(());
        }
        let Some(log) = self.session_log.as_mut() else {
            return Ok(());
        };

        log.write_all(bytes)?;
        log.flush()
    }

    fn refresh_snapshot(&mut self) {
        self.rebuild_snapshot();
        self.metrics.record_snapshot_rebuild();
        self.frame_needs_full_repaint = true;
        self.pending_frame_damage.clear();
    }

    fn refresh_snapshot_after_terminal_resize(&mut self, preserve_ordinary_selection: bool) {
        self.rebuild_snapshot_after_terminal_resize(preserve_ordinary_selection);
        self.metrics.record_snapshot_rebuild();
        self.frame_needs_full_repaint = true;
        self.pending_frame_damage.clear();
    }

    fn rebuild_snapshot(&mut self) {
        self.rebuild_snapshot_after_terminal_resize(false);
    }

    fn rebuild_snapshot_after_terminal_resize(&mut self, preserve_ordinary_selection: bool) {
        self.interaction_state
            .active_ui
            .stable_viewport
            .clamp_main(self.runtime.terminal());
        if !preserve_ordinary_selection {
            self.invalidate_active_ordinary_selection_for_presentation();
        }
        self.active_ui
            .refresh_search_match_cache(self.runtime.terminal());
        self.update_selection_projection();
        self.snapshot = terminal_runtime_snapshot(&self.runtime, self.active_ui.stable_viewport);
    }

    fn refresh_snapshot_after_terminal_damage(&mut self, damage: &[DamageRegion]) {
        self.interaction_state
            .active_ui
            .stable_viewport
            .clamp_main(self.runtime.terminal());
        if self.can_update_snapshot_from_damage() {
            let cursor_color = self.runtime.cursor_color_override();
            let cursor_color_changed = self.snapshot.cursor_color() != cursor_color;
            self.snapshot
                .update_from_terminal_damage(self.runtime.terminal(), damage);
            self.snapshot.set_cursor_color(cursor_color);
            if cursor_color_changed {
                self.frame_needs_full_repaint = true;
            }
            self.pending_frame_damage
                .extend(damage.iter().copied().filter(|region| !region.is_empty()));
            self.metrics.record_snapshot_damage_update();
            return;
        }

        self.refresh_snapshot();
    }

    fn can_update_snapshot_from_damage(&self) -> bool {
        self.current_scrollback_offset() == 0
            && self.selection.is_none()
            && self.active_ui.retained_search().is_none()
            && self.active_ui.copy_mode().is_none()
            && self.pane_select.is_none()
    }

    fn scroll_viewport_lines(&mut self, lines: isize) {
        let history_len = self.runtime.terminal().scrollback().len();
        let current_offset = self.current_scrollback_offset();
        let next_offset = if lines.is_positive() {
            current_offset
                .saturating_add(lines.unsigned_abs())
                .min(history_len)
        } else {
            current_offset.saturating_sub(lines.unsigned_abs())
        };

        if next_offset == current_offset {
            return;
        }

        self.interaction_state
            .active_ui
            .stable_viewport
            .set_scrollback_offset(self.runtime.terminal(), next_offset);
        self.update_selection_projection();
        self.pane_select = None;
        self.refresh_snapshot();
        self.apply_window_title();
    }

    fn scroll_to_prompt(&mut self, amount: isize) {
        let prompt_rows = self.runtime.terminal().stable_semantic_prompt_rows();
        if prompt_rows.is_empty() || amount == 0 {
            return;
        }

        let viewport_top = self
            .current_stable_viewport_top()
            .unwrap_or(self.runtime.terminal().stable_dimensions().physical_top);
        let index = match prompt_rows.binary_search(&viewport_top) {
            Ok(index) | Err(index) => index,
        };
        let target_index = if amount.is_negative() {
            index.saturating_sub(amount.unsigned_abs())
        } else {
            index.saturating_add(usize::try_from(amount).unwrap_or(usize::MAX))
        };
        let Some(prompt_row) = prompt_rows.get(target_index).copied() else {
            return;
        };

        self.set_stable_viewport_top(Some(prompt_row));
    }

    fn set_scrollback_offset(&mut self, offset: usize) {
        let next_offset = offset.min(self.runtime.terminal().scrollback().len());
        if next_offset == self.current_scrollback_offset() {
            return;
        }

        self.interaction_state
            .active_ui
            .stable_viewport
            .set_scrollback_offset(self.runtime.terminal(), next_offset);
        self.update_selection_projection();
        self.pane_select = None;
        self.refresh_snapshot();
        self.apply_window_title();
    }

    fn set_stable_viewport_top(&mut self, top: Option<StableRowIndex>) {
        if self.runtime.terminal().stable_dimensions().domain != TerminalScreenDomain::Main {
            return;
        }
        let next_top = PaneStableViewport::normalized_main_top(self.runtime.terminal(), top);
        if next_top == self.current_stable_viewport_top() {
            return;
        }
        self.active_ui.stable_viewport.main_top = next_top;
        self.update_selection_projection();
        self.pane_select = None;
        self.refresh_snapshot();
        self.apply_window_title();
    }

    fn current_stable_viewport_top(&self) -> Option<StableRowIndex> {
        self.active_ui
            .stable_viewport
            .active_top(self.runtime.terminal())
    }

    fn current_scrollback_offset(&self) -> usize {
        self.active_ui
            .stable_viewport
            .scrollback_offset(self.runtime.terminal())
    }

    fn current_viewport_stable_top(&self) -> StableRowIndex {
        self.current_stable_viewport_top()
            .unwrap_or(self.runtime.terminal().stable_dimensions().physical_top)
    }

    #[cfg(test)]
    fn set_scrollback_offset_for_test(&mut self, offset: usize) {
        let terminal = self.runtime.terminal();
        self.interaction_state
            .active_ui
            .stable_viewport
            .set_scrollback_offset(terminal, offset);
    }

    fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) -> bool {
        let lines = scrollback_lines_from_mouse_delta(delta);
        if lines == 0 {
            return false;
        }

        self.scroll_viewport_lines(lines);
        true
    }

    fn pane_runtime_ref(&self, pane_id: rterm_core::PaneId) -> Option<&TerminalRuntime> {
        if pane_id == self.app_shell.active_pane_id() {
            return Some(&self.runtime);
        }
        self.pane_runtimes
            .get(&pane_id)
            .map(|runtime| &runtime.runtime)
    }

    fn pane_ui_ref(&self, pane_id: rterm_core::PaneId) -> Option<&PaneUiState> {
        if pane_id == self.app_shell.active_pane_id() {
            return Some(&self.active_ui);
        }
        self.pane_runtimes.get(&pane_id).map(|runtime| &runtime.ui)
    }

    fn set_pane_scrollback_offset(&mut self, pane_id: rterm_core::PaneId, offset: usize) -> bool {
        if pane_id == self.app_shell.active_pane_id() {
            let before = self.current_scrollback_offset();
            self.set_scrollback_offset(offset);
            return self.current_scrollback_offset() != before;
        }

        let Some(runtime) = self.pane_runtimes.get_mut(&pane_id) else {
            return false;
        };
        let before = runtime
            .ui
            .stable_viewport
            .scrollback_offset(runtime.runtime.terminal());
        runtime
            .ui
            .stable_viewport
            .set_scrollback_offset(runtime.runtime.terminal(), offset);
        let changed = runtime
            .ui
            .stable_viewport
            .scrollback_offset(runtime.runtime.terminal())
            != before;
        if changed {
            self.refresh_wheel_target_owner(pane_id);
        }
        changed
    }

    fn scroll_pane_viewport_lines(&mut self, pane_id: rterm_core::PaneId, lines: isize) -> bool {
        let Some(runtime) = self.pane_runtime_ref(pane_id) else {
            return false;
        };
        let Some(ui) = self.pane_ui_ref(pane_id) else {
            return false;
        };
        let history_len = runtime.terminal().scrollback().len();
        let current_offset = ui.stable_viewport.scrollback_offset(runtime.terminal());
        let next_offset = if lines.is_positive() {
            current_offset
                .saturating_add(lines.unsigned_abs())
                .min(history_len)
        } else {
            current_offset.saturating_sub(lines.unsigned_abs())
        };
        self.set_pane_scrollback_offset(pane_id, next_offset)
    }

    fn refresh_wheel_target_owner(&mut self, pane_id: rterm_core::PaneId) {
        if pane_id == self.app_shell.active_pane_id() {
            self.update_selection_projection();
            self.pane_select = None;
            self.refresh_snapshot();
            self.apply_window_title();
            return;
        }

        let Some(runtime) = self.pane_runtimes.get_mut(&pane_id) else {
            return;
        };
        runtime
            .ui
            .stable_viewport
            .clamp_main(runtime.runtime.terminal());
        runtime
            .ui
            .refresh_search_match_cache(runtime.runtime.terminal());
        runtime.snapshot = terminal_runtime_snapshot(&runtime.runtime, runtime.ui.stable_viewport);
        self.metrics.record_snapshot_rebuild();
        self.frame_needs_full_repaint = true;
        self.pending_frame_damage.clear();
    }

    fn handle_window_mouse_wheel(&mut self, delta: MouseScrollDelta) -> io::Result<bool> {
        if self.pane_inspection_input_barrier_active() {
            return Ok(true);
        }
        let previous_delta = self.current_mouse_wheel_delta.replace(delta);
        let result = self.handle_window_mouse_wheel_with_current_delta(delta);
        self.current_mouse_wheel_delta = previous_delta;
        result
    }

    fn handle_window_mouse_wheel_with_current_delta(
        &mut self,
        delta: MouseScrollDelta,
    ) -> io::Result<bool> {
        if self.mouse_position_is_in_tab_bar() {
            return Ok(self.handle_tab_bar_mouse_wheel(delta));
        }

        let Some(hit) = self.wheel_hit_target_at_mouse_position() else {
            return Ok(false);
        };
        let target = match hit {
            WheelHitTarget::ActiveScrollbar { pane_id } => {
                debug_assert_eq!(pane_id, self.app_shell.active_pane_id());
                return Ok(self.handle_mouse_wheel(delta));
            }
            WheelHitTarget::PaneSurface(target) => target,
        };
        let Some(target_runtime) = self.pane_runtime_ref(target.pane_id) else {
            return Ok(false);
        };
        let mode = target_runtime.mouse_input_mode();
        let alternate_screen_active = target_runtime.terminal().alternate_screen_active();
        let bypass_mouse_reporting = mode.reporting_enabled()
            && !self.bypass_mouse_reporting_modifiers.is_empty()
            && self
                .modifiers
                .contains(self.bypass_mouse_reporting_modifiers);
        let mouse_reporting_for_assignment = mode.reporting_enabled() && !bypass_mouse_reporting;
        if mouse_reporting_for_assignment {
            self.set_pane_scrollback_offset(target.pane_id, 0);
        }
        let assignment_modifiers = if bypass_mouse_reporting {
            self.modifiers - self.bypass_mouse_reporting_modifiers
        } else {
            self.modifiers
        };
        let original_modifiers = std::mem::replace(&mut self.modifiers, assignment_modifiers);
        let assignment = native_mouse_assignment_wheel_button_from_delta(delta).map_or(
            WheelAssignmentMatch::None,
            |button| {
                self.wheel_assignment_match(
                    NativeMouseAssignmentEventKind::Down,
                    button,
                    1,
                    mouse_reporting_for_assignment,
                    alternate_screen_active,
                )
            },
        );
        let assignment_result = match &assignment {
            WheelAssignmentMatch::Command(command) => {
                Some(self.apply_wheel_command_for_target(target, command.clone()))
            }
            WheelAssignmentMatch::DisableDefault | WheelAssignmentMatch::None => None,
        };
        self.modifiers = original_modifiers;
        match assignment {
            WheelAssignmentMatch::DisableDefault => return Ok(false),
            WheelAssignmentMatch::Command(_) => {
                let outcome = assignment_result
                    .expect("wheel command match must produce a dispatcher result")?;
                match outcome {
                    WheelCommandOutcome::Consumed => return Ok(true),
                }
            }
            WheelAssignmentMatch::None => {}
        }

        if mode.reporting_enabled() && !bypass_mouse_reporting {
            let Some(kind) = window_mouse_wheel_kind(delta) else {
                return Ok(false);
            };
            if let Some(bytes) =
                self.encode_wheel_mouse_event_for_target(target, kind, mode, self.modifiers)
            {
                self.write_pty_bytes_to_pane_for_wheel(target.pane_id, &bytes)?;
                return Ok(true);
            }
            return Ok(false);
        }

        if self.disable_default_mouse_bindings {
            return Ok(false);
        }

        if alternate_screen_active {
            return self.handle_alternate_buffer_mouse_wheel_for_target(target, delta);
        }

        let lines = scrollback_lines_from_mouse_delta(delta);
        if lines == 0 {
            return Ok(false);
        }
        self.scroll_pane_viewport_lines(target.pane_id, lines);
        Ok(true)
    }

    fn wheel_assignment_match(
        &self,
        kind: NativeMouseAssignmentEventKind,
        button: NativeMouseAssignmentButton,
        streak: u8,
        mouse_reporting: bool,
        alternate_screen_active: bool,
    ) -> WheelAssignmentMatch {
        let Some(command) = self
            .mouse_assignments
            .iter()
            .find(|assignment| {
                assignment.event.kind == kind
                    && assignment.event.button == button
                    && assignment.event.streak == streak
                    && assignment.modifiers == self.modifiers
                    && assignment.mouse_reporting == mouse_reporting
                    && assignment.alt_screen.matches(alternate_screen_active)
            })
            .map(|assignment| assignment.command.clone())
        else {
            return WheelAssignmentMatch::None;
        };
        if command == WindowCommand::DisableDefaultAssignment {
            WheelAssignmentMatch::DisableDefault
        } else {
            WheelAssignmentMatch::Command(command)
        }
    }

    fn apply_wheel_command_for_target(
        &mut self,
        target: WheelTarget,
        command: WindowCommand,
    ) -> io::Result<WheelCommandOutcome> {
        match command.wheel_command_class() {
            WheelCommandClass::Composite => {
                let WindowCommand::Multiple(commands) = command else {
                    unreachable!("composite wheel command classification must be exhaustive");
                };
                for command in commands {
                    self.apply_wheel_command_for_target(target, command)?;
                }
            }
            WheelCommandClass::Viewport => self.apply_wheel_viewport_command(target, &command),
            WheelCommandClass::Writer => self.apply_wheel_writer_command(target, command)?,
            WheelCommandClass::PaneUi => self.apply_wheel_pane_ui_command(target, command)?,
            WheelCommandClass::PaneAction => self.apply_wheel_pane_action(target, command)?,
            WheelCommandClass::ContextualUi => {
                self.apply_wheel_contextual_ui_command(target, command)?;
            }
            WheelCommandClass::ExplicitFocusOrCreation => {
                self.apply_wheel_explicit_command(target, command)?;
            }
            WheelCommandClass::Global => {
                let original = command.clone();
                self.command_palette_apply_command(command)
                    .map_err(|error| wheel_action_io_error(&original, error))?;
            }
            WheelCommandClass::Nop => {}
            WheelCommandClass::DisableDefault => {
                debug_assert!(
                    false,
                    "DisableDefaultAssignment must not reach wheel dispatcher"
                );
            }
        }
        Ok(WheelCommandOutcome::Consumed)
    }

    fn apply_wheel_contextual_ui_command(
        &mut self,
        target: WheelTarget,
        command: WindowCommand,
    ) -> io::Result<()> {
        let pane_id = target.pane_id;
        if self.pane_runtime_ref(pane_id).is_none() {
            return Err(wheel_action_io_error(
                &command,
                AppShellError::InvalidPane(pane_id),
            ));
        }
        match command {
            WindowCommand::EmitEvent(event) => {
                self.emit_event_for_target(target, event);
            }
            WindowCommand::OpenUri(uri) => {
                self.open_uri_for_target(target, &uri);
            }
            WindowCommand::CharSelect => {
                self.enter_char_select_mode();
                self.deferred_wheel_context = Some(target);
            }
            WindowCommand::CharSelectArgs(options) => {
                self.enter_char_select_mode_with_options(options);
                self.deferred_wheel_context = Some(target);
            }
            WindowCommand::PromptInputLine(options) => {
                self.enter_prompt_input_line_mode(options);
                self.deferred_wheel_context = Some(target);
            }
            WindowCommand::InputSelector(options) => {
                self.enter_input_selector_mode(options);
                self.deferred_wheel_context = Some(target);
            }
            WindowCommand::Confirmation(options) => {
                self.enter_confirmation_mode(options);
                self.deferred_wheel_context = Some(target);
            }
            WindowCommand::ActivateCommandPalette => {
                self.enter_command_palette_mode_for_pane(pane_id);
                self.deferred_wheel_context = Some(target);
            }
            WindowCommand::ShowLauncher => {
                self.enter_launcher_mode();
                self.deferred_wheel_context = Some(target);
            }
            WindowCommand::ShowLauncherArgs(args) => {
                self.enter_launcher_mode_with_args(args);
                self.deferred_wheel_context = Some(target);
            }
            WindowCommand::EnterPaneSwap => {
                self.enter_pane_select_mode_with_mode(WindowPaneSelectMode::SwapWithActive);
                self.deferred_wheel_context = Some(target);
            }
            WindowCommand::EnterPaneSwapKeepFocus => {
                self.enter_pane_select_mode_with_mode(
                    WindowPaneSelectMode::SwapWithActiveKeepFocus,
                );
                self.deferred_wheel_context = Some(target);
            }
            WindowCommand::PaneSelect(options) => {
                debug_assert!(matches!(
                    options.mode,
                    WindowPaneSelectMode::SwapWithActive
                        | WindowPaneSelectMode::SwapWithActiveKeepFocus
                ));
                self.enter_pane_select_mode_with_action(options);
                self.deferred_wheel_context = Some(target);
            }
            _ => unreachable!("contextual UI wheel command classification must be exhaustive"),
        }
        Ok(())
    }

    fn apply_command_for_target_context(
        &mut self,
        target: WheelTarget,
        command: WindowCommand,
    ) -> io::Result<()> {
        let pane_id = target.pane_id;
        if self.pane_runtime_ref(pane_id).is_none() {
            return Err(wheel_action_io_error(
                &command,
                AppShellError::InvalidPane(pane_id),
            ));
        }
        self.apply_wheel_command_for_target(target, command)?;
        Ok(())
    }

    fn apply_wheel_viewport_command(&mut self, target: WheelTarget, command: &WindowCommand) {
        let pane_id = target.pane_id;
        match command {
            WindowCommand::ScrollToTop => {
                let history = self
                    .pane_runtime_ref(pane_id)
                    .map_or(0, |runtime| runtime.terminal().scrollback().len());
                self.set_pane_scrollback_offset(pane_id, history);
            }
            WindowCommand::ScrollToBottom => {
                self.set_pane_scrollback_offset(pane_id, 0);
            }
            WindowCommand::ScrollByPage(amount) => {
                let rows = self.pane_runtime_ref(pane_id).map_or(0, |runtime| {
                    isize::try_from(i32::from(runtime.terminal().grid().size().rows))
                        .unwrap_or(isize::MAX)
                });
                self.scroll_pane_viewport_lines(pane_id, amount.viewport_lines(rows));
            }
            WindowCommand::ScrollByLine(amount) => {
                self.scroll_pane_viewport_lines(pane_id, amount.saturating_neg());
            }
            WindowCommand::ScrollByCurrentEventWheelDelta => {
                if let Some(delta) = self.current_mouse_wheel_delta {
                    self.scroll_pane_viewport_lines(
                        pane_id,
                        scrollback_lines_from_mouse_delta(delta),
                    );
                }
            }
            WindowCommand::ScrollPageUp => {
                let rows = self.pane_runtime_ref(pane_id).map_or(0, |runtime| {
                    isize::try_from(i32::from(runtime.terminal().grid().size().rows))
                        .unwrap_or(isize::MAX)
                });
                self.scroll_pane_viewport_lines(pane_id, rows);
            }
            WindowCommand::ScrollPageDown => {
                let rows = self.pane_runtime_ref(pane_id).map_or(0, |runtime| {
                    isize::try_from(i32::from(runtime.terminal().grid().size().rows))
                        .unwrap_or(isize::MAX)
                });
                self.scroll_pane_viewport_lines(pane_id, -rows);
            }
            WindowCommand::ScrollLineUp => {
                self.scroll_pane_viewport_lines(pane_id, 1);
            }
            WindowCommand::ScrollLineDown => {
                self.scroll_pane_viewport_lines(pane_id, -1);
            }
            WindowCommand::ScrollToPrompt(amount) => {
                self.scroll_wheel_target_to_prompt(pane_id, *amount);
            }
            WindowCommand::ScrollToPreviousPrompt => {
                self.scroll_wheel_target_to_prompt(pane_id, -1);
            }
            WindowCommand::ScrollToNextPrompt => {
                self.scroll_wheel_target_to_prompt(pane_id, 1);
            }
            _ => unreachable!("viewport wheel command classification must be exhaustive"),
        }
    }

    fn scroll_wheel_target_to_prompt(&mut self, pane_id: rterm_core::PaneId, amount: isize) {
        let Some(runtime) = self.pane_runtime_ref(pane_id) else {
            return;
        };
        let prompts = runtime.terminal().stable_semantic_prompt_rows();
        if prompts.is_empty() || amount == 0 {
            return;
        }
        let dimensions = runtime.terminal().stable_dimensions();
        let top = self
            .pane_ui_ref(pane_id)
            .and_then(|ui| ui.stable_viewport.active_top(runtime.terminal()))
            .unwrap_or(dimensions.physical_top);
        let index = match prompts.binary_search(&top) {
            Ok(index) | Err(index) => index,
        };
        let target_index = if amount.is_negative() {
            index.saturating_sub(amount.unsigned_abs())
        } else {
            index.saturating_add(usize::try_from(amount).unwrap_or(usize::MAX))
        };
        let Some(prompt) = prompts.get(target_index).copied() else {
            return;
        };
        let offset = dimensions
            .physical_top
            .saturating_sub(prompt)
            .try_into()
            .unwrap_or(usize::MAX);
        self.set_pane_scrollback_offset(pane_id, offset);
    }

    fn apply_wheel_writer_command(
        &mut self,
        target: WheelTarget,
        command: WindowCommand,
    ) -> io::Result<()> {
        let pane_id = target.pane_id;
        match command {
            WindowCommand::SendString(value) => {
                self.write_pty_bytes_to_pane_for_wheel(pane_id, value.as_bytes())?;
            }
            WindowCommand::SendPaste(value) => {
                let bytes = encode_window_paste(
                    &value,
                    self.pane_bracketed_paste(pane_id),
                    self.canonicalize_pasted_newlines,
                );
                self.write_pty_bytes_to_pane_for_wheel(pane_id, &bytes)?;
            }
            WindowCommand::SendKey(send_key) => self.send_key_to_pane(pane_id, &send_key)?,
            WindowCommand::PasteFromClipboard
            | WindowCommand::Paste
            | WindowCommand::PasteFrom(WindowPasteSource::Clipboard) => {
                let text = self.read_clipboard_text();
                self.paste_wheel_text_to_pane(pane_id, text.as_deref())?;
            }
            WindowCommand::PasteFromPrimarySelection
            | WindowCommand::PastePrimarySelection
            | WindowCommand::PasteFrom(WindowPasteSource::PrimarySelection) => {
                let text = self.read_primary_selection_text();
                self.paste_wheel_text_to_pane(pane_id, text.as_deref())?;
            }
            _ => unreachable!("writer wheel command classification must be exhaustive"),
        }
        Ok(())
    }

    fn paste_wheel_text_to_pane(
        &mut self,
        pane_id: rterm_core::PaneId,
        text: Option<&str>,
    ) -> io::Result<()> {
        let Some(text) = text.filter(|text| !text.is_empty()) else {
            return Ok(());
        };
        let bytes = encode_window_paste(
            text,
            self.pane_bracketed_paste(pane_id),
            self.canonicalize_pasted_newlines,
        );
        self.write_pty_bytes_to_pane_for_wheel(pane_id, &bytes)
    }

    fn wheel_target_source_cell(&self, target: WheelTarget) -> Option<SelectionSourceCell> {
        let runtime = self.pane_runtime_ref(target.pane_id)?;
        let ui = self.pane_ui_ref(target.pane_id)?;
        let size = runtime.terminal().grid().size();
        if target.cell.row >= size.rows || target.cell.column >= size.columns {
            return None;
        }
        let dimensions = runtime.terminal().stable_dimensions();
        let top = ui
            .stable_viewport
            .active_top(runtime.terminal())
            .unwrap_or(dimensions.physical_top);
        Some(SelectionSourceCell {
            domain: dimensions.domain,
            row: top.saturating_add(
                StableRowIndex::try_from(target.cell.row).unwrap_or(StableRowIndex::MAX),
            ),
            column: usize::from(target.cell.column),
        })
    }

    fn set_wheel_target_selection(
        &mut self,
        pane_id: rterm_core::PaneId,
        selection: Option<StableOrdinarySelection>,
    ) {
        if pane_id == self.app_shell.active_pane_id() {
            self.active_ui.ordinary_selection = selection;
        } else if let Some(runtime) = self.pane_runtimes.get_mut(&pane_id) {
            runtime.ui.ordinary_selection = selection;
        }
        self.refresh_wheel_target_owner(pane_id);
    }

    fn wheel_selection_for_mode(
        &self,
        target: WheelTarget,
        mode: WindowMouseSelectionMode,
    ) -> Option<WindowSourceSelection> {
        let cell = self.wheel_target_source_cell(target)?;
        let terminal = self.pane_runtime_ref(target.pane_id)?.terminal();
        match mode {
            WindowMouseSelectionMode::Cell => Some(WindowSourceSelection::new(cell, cell)),
            WindowMouseSelectionMode::Word => {
                copy_mode_word_source_selection(terminal, cell, &self.selection_word_boundary)
            }
            WindowMouseSelectionMode::Line => {
                let columns = terminal.grid().size().columns;
                Some(WindowSourceSelection::new(
                    SelectionSourceCell { column: 0, ..cell },
                    SelectionSourceCell {
                        column: usize::from(columns.saturating_sub(1)),
                        ..cell
                    },
                ))
            }
            WindowMouseSelectionMode::Block => Some(WindowSourceSelection::rectangular(cell, cell)),
            WindowMouseSelectionMode::SemanticZone => {
                copy_mode_semantic_zone_source_selection(terminal, cell)
            }
        }
    }

    fn select_wheel_target_text(&mut self, target: WheelTarget, mode: WindowMouseSelectionMode) {
        let Some(selection) = self.wheel_selection_for_mode(target, mode) else {
            return;
        };
        let Some(sequence) = self
            .pane_runtime_ref(target.pane_id)
            .map(|runtime| runtime.terminal().current_seqno())
        else {
            return;
        };
        let stable = if selection.rectangular {
            StableOrdinarySelection::rectangular(selection.anchor, selection.focus, sequence)
        } else {
            StableOrdinarySelection::new(selection.anchor, selection.focus, sequence)
        };
        if target.pane_id == self.app_shell.active_pane_id() {
            self.active_ui.exit_overlay();
            self.selecting = false;
            self.last_left_click = None;
        } else if let Some(runtime) = self.pane_runtimes.get_mut(&target.pane_id) {
            runtime.ui.exit_overlay();
        }
        self.set_wheel_target_selection(target.pane_id, Some(stable));
    }

    fn extend_wheel_target_selection(
        &mut self,
        target: WheelTarget,
        mode: WindowMouseSelectionMode,
    ) {
        let Some(current) = self
            .pane_ui_ref(target.pane_id)
            .and_then(|ui| ui.ordinary_selection)
        else {
            return;
        };
        let Some(target_selection) = self.wheel_selection_for_mode(target, mode) else {
            return;
        };
        let Some(sequence) = self
            .pane_runtime_ref(target.pane_id)
            .map(|runtime| runtime.terminal().current_seqno())
        else {
            return;
        };
        let selection = if mode == WindowMouseSelectionMode::Block {
            StableOrdinarySelection::rectangular(current.anchor, target_selection.focus, sequence)
        } else {
            StableOrdinarySelection::new(
                current.anchor,
                stable_selection_focus_for_extension(current, target_selection),
                sequence,
            )
        };
        if target.pane_id == self.app_shell.active_pane_id() {
            self.active_ui.exit_overlay();
            self.selecting = false;
            self.last_left_click = None;
        } else if let Some(runtime) = self.pane_runtimes.get_mut(&target.pane_id) {
            runtime.ui.exit_overlay();
        }
        self.set_wheel_target_selection(target.pane_id, Some(selection));
    }

    fn wheel_target_selected_text(&self, pane_id: rterm_core::PaneId) -> Option<String> {
        let selection = self.pane_ui_ref(pane_id)?.ordinary_selection?;
        let text = selection.text_from_terminal(self.pane_runtime_ref(pane_id)?.terminal())?;
        (!text.is_empty()).then_some(text)
    }

    fn initial_copy_mode_for_pane(&self, pane_id: rterm_core::PaneId) -> Option<WindowCopyMode> {
        let terminal = self.pane_runtime_ref(pane_id)?.terminal();
        let size = terminal.grid().size();
        let (row, column) = terminal.cursor();
        let dimensions = terminal.stable_dimensions();
        let row = if size.rows == 0 {
            0
        } else {
            row.min(size.rows.saturating_sub(1))
        };
        let column = if size.columns == 0 {
            0
        } else {
            column.min(size.columns.saturating_sub(1))
        };
        Some(WindowCopyMode {
            cursor: SelectionCell { row, column },
            source_cursor: SelectionSourceCell {
                domain: dimensions.domain,
                row: dimensions
                    .physical_top
                    .saturating_add(StableRowIndex::try_from(row).unwrap_or(StableRowIndex::MAX)),
                column: usize::from(column),
            },
            pending_jump: None,
            last_jump: None,
            search_direction: None,
            selection_mode: WindowCopySelectionMode::None,
            anchor: None,
            source_anchor: None,
        })
    }

    fn enter_wheel_target_copy_mode(&mut self, pane_id: rterm_core::PaneId) {
        let Some(initial) = self.initial_copy_mode_for_pane(pane_id) else {
            return;
        };
        if pane_id == self.app_shell.active_pane_id() {
            self.active_ui.enter_copy_mode(initial);
        } else if let Some(runtime) = self.pane_runtimes.get_mut(&pane_id) {
            runtime.ui.enter_copy_mode(initial);
        }
        self.refresh_wheel_target_owner(pane_id);
    }

    fn apply_wheel_copy_mode_assignment(
        &mut self,
        pane_id: rterm_core::PaneId,
        assignment: WindowCopyModeAssignment,
    ) -> Result<(), AppShellError> {
        if pane_id == self.app_shell.active_pane_id() {
            self.perform_copy_mode_assignment(assignment);
            return Ok(());
        }
        let Some(runtime) = self.pane_runtimes.get_mut(&pane_id) else {
            return Err(AppShellError::InvalidPane(pane_id));
        };
        let handled = Self::perform_copy_mode_assignment_for_owner(
            runtime.runtime.terminal(),
            &mut runtime.ui,
            assignment,
        );
        if handled {
            self.refresh_wheel_target_owner(pane_id);
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
    )]
    fn perform_copy_mode_assignment_for_owner(
        terminal: &Terminal,
        ui: &mut PaneUiState,
        assignment: WindowCopyModeAssignment,
    ) -> bool {
        let (had_copy_mode, had_search) = Self::copy_mode_overlay_presence(ui);
        let before = (
            ui.copy_search_mode(),
            ui.retained_copy_mode().cloned(),
            ui.retained_search().cloned(),
            ui.stable_viewport,
        );
        if !Self::perform_copy_mode_meta_assignment(terminal, ui, assignment, had_copy_mode) {
            match assignment {
            WindowCopyModeAssignment::StartJump { forward, prev_char } => {
                if let Some(copy_mode) = ui.copy_mode_mut() {
                    copy_mode.pending_jump = Some(WindowCopyPendingJump { forward, prev_char });
                }
            }
            WindowCopyModeAssignment::JumpAgain | WindowCopyModeAssignment::JumpReverse => {
                if let Some(mut jump) = ui.copy_mode().and_then(|copy_mode| copy_mode.last_jump) {
                    if assignment == WindowCopyModeAssignment::JumpReverse {
                        jump.forward = !jump.forward;
                    }
                    if let Some(cursor) = ui.copy_mode().map(|mode| mode.source_cursor)
                        && let Some(target) = copy_mode_jump_target(terminal, cursor, jump, true)
                    {
                        Self::set_copy_mode_owner_source(terminal, ui, target, None);
                    }
                }
            }
            WindowCopyModeAssignment::MoveBackwardSemanticZone
            | WindowCopyModeAssignment::MoveForwardSemanticZone
            | WindowCopyModeAssignment::MoveSemanticZoneOfType { .. } => {
                let (delta, semantic_type) = match assignment {
                    WindowCopyModeAssignment::MoveBackwardSemanticZone => (-1, None),
                    WindowCopyModeAssignment::MoveForwardSemanticZone => (1, None),
                    WindowCopyModeAssignment::MoveSemanticZoneOfType {
                        delta,
                        semantic_type,
                    } => (delta, Some(semantic_type)),
                    _ => unreachable!(),
                };
                Self::move_copy_mode_owner_semantic_zone(terminal, ui, delta, semantic_type);
            }
            WindowCopyModeAssignment::MoveBackwardWord
            | WindowCopyModeAssignment::MoveForwardWord
            | WindowCopyModeAssignment::MoveForwardWordEnd => {
                let movement = match assignment {
                    WindowCopyModeAssignment::MoveBackwardWord => WindowCopyWordMovement::Backward,
                    WindowCopyModeAssignment::MoveForwardWord => WindowCopyWordMovement::Forward,
                    WindowCopyModeAssignment::MoveForwardWordEnd => WindowCopyWordMovement::End,
                    _ => unreachable!(),
                };
                if let Some(cursor) = ui.copy_mode().map(|mode| mode.source_cursor)
                    && let Some(target) = copy_mode_word_target(terminal, cursor, movement)
                {
                    Self::set_copy_mode_owner_source(terminal, ui, target, None);
                }
            }
            WindowCopyModeAssignment::MoveDown
            | WindowCopyModeAssignment::MoveLeft
            | WindowCopyModeAssignment::MoveRight
            | WindowCopyModeAssignment::MoveUp => {
                let (rows, columns) = match assignment {
                    WindowCopyModeAssignment::MoveDown => (1, 0),
                    WindowCopyModeAssignment::MoveLeft => (0, -1),
                    WindowCopyModeAssignment::MoveRight => (0, 1),
                    WindowCopyModeAssignment::MoveUp => (-1, 0),
                    _ => unreachable!(),
                };
                Self::move_copy_mode_owner_cursor(terminal, ui, rows, columns);
            }
            WindowCopyModeAssignment::MoveToEndOfLineContent
            | WindowCopyModeAssignment::MoveToStartOfLineContent => {
                if let Some(cursor) = ui.copy_mode().map(|mode| mode.source_cursor)
                    && let Some((start, end)) =
                        copy_mode_line_content_bounds(terminal, cursor.domain, cursor.row)
                {
                    let column = if assignment == WindowCopyModeAssignment::MoveToEndOfLineContent {
                        end
                    } else {
                        start
                    };
                    Self::set_copy_mode_owner_source(
                        terminal,
                        ui,
                        SelectionSourceCell { column, ..cursor },
                        None,
                    );
                }
            }
            WindowCopyModeAssignment::MoveToScrollbackBottom
            | WindowCopyModeAssignment::MoveToScrollbackTop => {
                let retained = terminal.retained_stable_range();
                if retained.start < retained.end
                    && let Some(cursor) = ui.copy_mode().map(|mode| mode.source_cursor)
                {
                    let row = if assignment == WindowCopyModeAssignment::MoveToScrollbackTop {
                        retained.start
                    } else {
                        retained.end.saturating_sub(1)
                    };
                    Self::set_copy_mode_owner_source(
                        terminal,
                        ui,
                        SelectionSourceCell { row, ..cursor },
                        None,
                    );
                }
            }
            WindowCopyModeAssignment::MoveToSelectionOtherEnd
            | WindowCopyModeAssignment::MoveToSelectionOtherEndHoriz => {
                if let Some((cursor, anchor)) = ui.copy_mode().and_then(|mode| {
                    mode.source_anchor
                        .map(|anchor| (mode.source_cursor, anchor))
                }) {
                    let (next_cursor, next_anchor) =
                        if assignment == WindowCopyModeAssignment::MoveToSelectionOtherEnd {
                            (anchor, cursor)
                        } else {
                            (
                                SelectionSourceCell {
                                    column: anchor.column,
                                    ..cursor
                                },
                                SelectionSourceCell {
                                    column: cursor.column,
                                    ..anchor
                                },
                            )
                        };
                    Self::set_copy_mode_owner_source(terminal, ui, next_cursor, Some(next_anchor));
                }
            }
            WindowCopyModeAssignment::MoveToStartOfLine => {
                if let Some(cursor) = ui.copy_mode().map(|mode| mode.source_cursor) {
                    Self::set_copy_mode_owner_source(
                        terminal,
                        ui,
                        SelectionSourceCell {
                            column: 0,
                            ..cursor
                        },
                        None,
                    );
                }
            }
            WindowCopyModeAssignment::MoveToStartOfNextLine => {
                Self::move_copy_mode_owner_cursor(terminal, ui, 1, 0);
                if let Some(cursor) = ui.copy_mode().map(|mode| mode.source_cursor) {
                    Self::set_copy_mode_owner_source(
                        terminal,
                        ui,
                        SelectionSourceCell {
                            column: 0,
                            ..cursor
                        },
                        None,
                    );
                }
            }
            WindowCopyModeAssignment::MoveToViewportBottom
            | WindowCopyModeAssignment::MoveToViewportMiddle
            | WindowCopyModeAssignment::MoveToViewportTop => {
                let size = terminal.grid().size();
                let top = ui
                    .stable_viewport
                    .active_top(terminal)
                    .unwrap_or(terminal.stable_dimensions().physical_top);
                let offset = match assignment {
                    WindowCopyModeAssignment::MoveToViewportTop => 0,
                    WindowCopyModeAssignment::MoveToViewportMiddle => size.rows / 2,
                    WindowCopyModeAssignment::MoveToViewportBottom => size.rows.saturating_sub(1),
                    _ => unreachable!(),
                };
                if let Some(cursor) = ui.copy_mode().map(|mode| mode.source_cursor) {
                    Self::set_copy_mode_owner_source(
                        terminal,
                        ui,
                        SelectionSourceCell {
                            row: top.saturating_add(
                                StableRowIndex::try_from(offset).unwrap_or(StableRowIndex::MAX),
                            ),
                            column: 0,
                            ..cursor
                        },
                        None,
                    );
                }
            }
            WindowCopyModeAssignment::MoveByPage(amount) => {
                let page = isize::try_from(terminal.grid().size().rows).unwrap_or(0);
                Self::move_copy_mode_owner_cursor(terminal, ui, -amount.viewport_lines(page), 0);
            }
            WindowCopyModeAssignment::PageDown | WindowCopyModeAssignment::PageUp => {
                let page = isize::try_from(terminal.grid().size().rows).unwrap_or(0);
                let delta = if assignment == WindowCopyModeAssignment::PageDown {
                    page
                } else {
                    -page
                };
                Self::move_copy_mode_owner_cursor(terminal, ui, delta, 0);
            }
            WindowCopyModeAssignment::NextMatch
            | WindowCopyModeAssignment::NextMatchPage
            | WindowCopyModeAssignment::PriorMatch
            | WindowCopyModeAssignment::PriorMatchPage => {
                let direction = if matches!(
                    assignment,
                    WindowCopyModeAssignment::NextMatch | WindowCopyModeAssignment::NextMatchPage
                ) {
                    SearchDirection::Next
                } else {
                    SearchDirection::Previous
                };
                if let Some((query, current)) = ui
                    .retained_search()
                    .map(|search| (search.query.clone(), search.current))
                    && !query.is_empty()
                {
                    ui.refresh_search_match_cache(terminal);
                    let size = terminal.grid().size();
                    let viewport_top = ui
                        .stable_viewport
                        .active_top(terminal)
                        .unwrap_or(terminal.stable_dimensions().physical_top);
                    let retained = terminal.retained_stable_range();
                    let page = matches!(
                        assignment,
                        WindowCopyModeAssignment::NextMatchPage
                            | WindowCopyModeAssignment::PriorMatchPage
                    );
                    let found = ui.cached_search_matches(terminal).and_then(|matches| {
                        if page && size.rows != 0 {
                            find_window_search_page_match(
                                &matches,
                                retained,
                                viewport_top,
                                usize::from(size.rows),
                                direction,
                            )
                            .or_else(|| find_window_search_match(&matches, current, direction))
                        } else {
                            find_window_search_match(&matches, current, direction)
                        }
                    });
                    if let Some(found) = found {
                        ui.set_search_current(Some(found));
                        Self::apply_search_match_to_pane_ui(terminal, ui, found, false);
                    }
                }
            }
            WindowCopyModeAssignment::SetSelectionMode(mode) => {
                if let Some(copy_mode) = ui.copy_mode_mut() {
                    copy_mode.selection_mode = mode;
                    match mode {
                        WindowCopySelectionMode::None
                        | WindowCopySelectionMode::Word
                        | WindowCopySelectionMode::Line
                        | WindowCopySelectionMode::SemanticZone => {
                            copy_mode.anchor = None;
                            copy_mode.source_anchor = None;
                        }
                        WindowCopySelectionMode::Cell | WindowCopySelectionMode::Block => {
                            copy_mode.anchor = Some(copy_mode.cursor);
                            copy_mode.source_anchor = Some(copy_mode.source_cursor);
                        }
                    }
                }
            }
            WindowCopyModeAssignment::AcceptPattern
            | WindowCopyModeAssignment::Close
            | WindowCopyModeAssignment::ClearPattern
            | WindowCopyModeAssignment::ClearSelectionMode
            | WindowCopyModeAssignment::CycleMatchType
            | WindowCopyModeAssignment::EditPattern => unreachable!(),
            }
        }
        let changed = before
            != (
                ui.copy_search_mode(),
                ui.retained_copy_mode().cloned(),
                ui.retained_search().cloned(),
                ui.stable_viewport,
            );
        changed
            || matches!(
                assignment,
                WindowCopyModeAssignment::AcceptPattern
                    | WindowCopyModeAssignment::ClearPattern
                    | WindowCopyModeAssignment::EditPattern
            ) && had_search
            || matches!(
                assignment,
                WindowCopyModeAssignment::ClearSelectionMode
                    | WindowCopyModeAssignment::StartJump { .. }
                    | WindowCopyModeAssignment::SetSelectionMode(_)
            ) && had_copy_mode
    }

    fn perform_copy_mode_meta_assignment(
        terminal: &Terminal,
        ui: &mut PaneUiState,
        assignment: WindowCopyModeAssignment,
        had_copy_mode: bool,
    ) -> bool {
        match assignment {
            WindowCopyModeAssignment::Close => {
                if had_copy_mode {
                    ui.stable_viewport = PaneStableViewport::default();
                    ui.exit_overlay();
                }
            }
            WindowCopyModeAssignment::AcceptPattern => {
                ui.set_search_editing(false);
            }
            WindowCopyModeAssignment::ClearPattern => {
                let match_type = ui
                    .retained_search()
                    .map_or(WindowSearchMatchType::CaseSensitive, |search| {
                        search.match_type
                    });
                ui.replace_search_pattern(String::new(), match_type);
            }
            WindowCopyModeAssignment::ClearSelectionMode => {
                if let Some(copy_mode) = ui.copy_mode_mut() {
                    copy_mode.selection_mode = WindowCopySelectionMode::None;
                    copy_mode.anchor = None;
                    copy_mode.source_anchor = None;
                }
            }
            WindowCopyModeAssignment::CycleMatchType => {
                if let Some(search) = ui.retained_search() {
                    let query = search.query.clone();
                    ui.replace_search_pattern(query.clone(), search.match_type.next());
                    if !query.is_empty() {
                        ui.refresh_search_match_cache(terminal);
                        let found = ui.cached_search_matches(terminal).and_then(|matches| {
                            find_window_search_match(&matches, None, SearchDirection::Next)
                        });
                        ui.set_search_current(found);
                        if let Some(found) = found {
                            let preserve_copy_state = ui.copy_search_mode()
                                == Some(WindowCopySearchMode::Search)
                                && ui.retained_copy_mode().is_some();
                            Self::apply_search_match_to_pane_ui(
                                terminal,
                                ui,
                                found,
                                preserve_copy_state,
                            );
                        }
                    }
                }
            }
            WindowCopyModeAssignment::EditPattern => {
                ui.set_search_editing(true);
            }
            _ => return false,
        }
        true
    }

    fn copy_mode_overlay_presence(ui: &PaneUiState) -> (bool, bool) {
        (ui.copy_mode().is_some(), ui.retained_search().is_some())
    }
}

impl NativeWindowApp {
    fn set_copy_mode_owner_source(
        terminal: &Terminal,
        ui: &mut PaneUiState,
        source_cursor: SelectionSourceCell,
        requested_anchor: Option<SelectionSourceCell>,
    ) -> bool {
        let size = terminal.grid().size();
        if size.rows == 0 || size.columns == 0 {
            return false;
        }
        let dimensions = terminal.stable_dimensions();
        if source_cursor.domain != dimensions.domain {
            return false;
        }
        let history_len = terminal.scrollback().len();
        let current_offset = ui
            .stable_viewport
            .scrollback_offset(terminal)
            .min(history_len);
        let Some(source_history_row) = terminal.stable_row_to_history_index(source_cursor.row)
        else {
            return false;
        };
        let (target_offset, target_viewport_top, target) =
            if dimensions.domain == TerminalScreenDomain::Alternate {
                let Some(target) = copy_mode_cell_for_source_position(
                    source_history_row,
                    source_cursor.column,
                    0,
                    size,
                ) else {
                    return false;
                };
                (0, 0, target)
            } else {
                let Some((target_offset, target)) = copy_mode_viewport_cell_for_source_position(
                    source_history_row,
                    source_cursor.column,
                    current_offset,
                    history_len,
                    size,
                ) else {
                    return false;
                };
                (
                    target_offset,
                    copy_mode_viewport_top(history_len, target_offset),
                    target,
                )
            };
        let source_anchor = requested_anchor
            .or_else(|| ui.copy_mode().and_then(|copy_mode| copy_mode.source_anchor));
        ui.stable_viewport
            .set_scrollback_offset(terminal, target_offset);
        if let Some(copy_mode) = ui.copy_mode_mut() {
            copy_mode.cursor = target;
            copy_mode.source_cursor = source_cursor;
            copy_mode.source_anchor = source_anchor;
            copy_mode.anchor = source_anchor.and_then(|anchor| {
                let source_row = terminal.stable_row_to_history_index(anchor.row)?;
                copy_mode_cell_for_source_position(
                    source_row,
                    anchor.column,
                    target_viewport_top,
                    size,
                )
            });
            return true;
        }
        false
    }

    fn move_copy_mode_owner_cursor(
        terminal: &Terminal,
        ui: &mut PaneUiState,
        row_delta: isize,
        column_delta: isize,
    ) -> bool {
        let Some(cursor) = ui.copy_mode().map(|mode| mode.source_cursor) else {
            return false;
        };
        let retained = terminal.retained_stable_range();
        if retained.start >= retained.end {
            return false;
        }
        let row = cursor
            .row
            .saturating_add(StableRowIndex::try_from(row_delta).unwrap_or_else(|_| {
                if row_delta.is_negative() {
                    StableRowIndex::MIN
                } else {
                    StableRowIndex::MAX
                }
            }))
            .clamp(retained.start, retained.end.saturating_sub(1));
        let max_column = usize::from(terminal.grid().size().columns.saturating_sub(1));
        let column = if column_delta.is_negative() {
            cursor.column.saturating_sub(column_delta.unsigned_abs())
        } else {
            cursor
                .column
                .saturating_add(column_delta.unsigned_abs())
                .min(max_column)
        };
        Self::set_copy_mode_owner_source(
            terminal,
            ui,
            SelectionSourceCell {
                row,
                column,
                ..cursor
            },
            None,
        )
    }

    fn move_copy_mode_owner_semantic_zone(
        terminal: &Terminal,
        ui: &mut PaneUiState,
        delta: isize,
        semantic_type: Option<SemanticType>,
    ) -> bool {
        if delta == 0 {
            return false;
        }
        let Some(cursor) = ui.copy_mode().map(|mode| mode.source_cursor) else {
            return false;
        };
        let zones = terminal.stable_semantic_zones();
        let mut index = match zones.binary_search_by(|zone| match zone.start_y.cmp(&cursor.row) {
            std::cmp::Ordering::Equal => zone.start_x.cmp(&cursor.column),
            ordering => ordering,
        }) {
            Ok(index) | Err(index) => index,
        };
        let step = if delta > 0 { 1 } else { -1 };
        let mut remaining = delta;
        while remaining != 0 {
            index = if step > 0 {
                let Some(next) = index.checked_add(1) else {
                    return false;
                };
                next
            } else {
                let Some(previous) = index.checked_sub(1) else {
                    return false;
                };
                previous
            };
            let Some(zone) = zones.get(index) else {
                return false;
            };
            if semantic_type.is_some_and(|kind| zone.semantic_type != kind) {
                continue;
            }
            remaining -= step;
            if remaining == 0 {
                return Self::set_copy_mode_owner_source(
                    terminal,
                    ui,
                    SelectionSourceCell {
                        domain: cursor.domain,
                        row: zone.start_y,
                        column: zone.start_x,
                    },
                    None,
                );
            }
        }
        false
    }

    fn enter_wheel_target_search(
        &mut self,
        pane_id: rterm_core::PaneId,
        query: Option<WindowSearchCommandQuery>,
    ) {
        if pane_id == self.app_shell.active_pane_id() {
            if let Some(query) = query.as_ref() {
                self.enter_search_mode_with_query(query);
            } else {
                self.enter_search_mode();
            }
            return;
        }
        let Some(initial) = self.initial_copy_mode_for_pane(pane_id) else {
            return;
        };
        let selected_query = self
            .wheel_target_selected_text(pane_id)
            .map(|text| single_line_search_query_from_selection(&text))
            .filter(|query| !query.is_empty());
        let requested = match query {
            Some(WindowSearchCommandQuery::Pattern {
                pattern,
                match_type,
            }) => WindowSearch {
                query: pattern,
                match_type,
                ..WindowSearch::default()
            },
            Some(WindowSearchCommandQuery::CurrentSelectionOrEmptyString) | None => WindowSearch {
                query: selected_query.unwrap_or_default(),
                ..WindowSearch::default()
            },
        };
        if let Some(runtime) = self.pane_runtimes.get_mut(&pane_id) {
            runtime.ui.enter_search(initial, requested);
            let direction = runtime
                .ui
                .retained_copy_mode()
                .and_then(|copy_mode| copy_mode.search_direction)
                .unwrap_or(SearchDirection::Previous);
            runtime
                .ui
                .refresh_search_match_cache(runtime.runtime.terminal());
            let found = runtime
                .ui
                .cached_search_matches(runtime.runtime.terminal())
                .and_then(|matches| find_window_search_match(&matches, None, direction));
            runtime.ui.set_search_current(found);
            if let Some(found) = found {
                Self::apply_search_match_to_pane_ui(
                    runtime.runtime.terminal(),
                    &mut runtime.ui,
                    found,
                    true,
                );
            }
        }
        self.refresh_wheel_target_owner(pane_id);
    }

    fn apply_search_match_to_pane_ui(
        terminal: &Terminal,
        ui: &mut PaneUiState,
        search_match: WindowSearchMatch,
        preserve_copy_state: bool,
    ) -> bool {
        let Some((offset, selection)) = search_match.viewport_selection(terminal) else {
            ui.set_search_current(None);
            return false;
        };
        ui.stable_viewport.set_scrollback_offset(terminal, offset);
        if !preserve_copy_state && let Some(copy_mode) = ui.retained_copy_mode_mut() {
            copy_mode.cursor = selection.anchor;
            copy_mode.source_cursor = SelectionSourceCell {
                domain: search_match.domain,
                row: search_match.source_row,
                column: usize::from(search_match.start_column),
            };
            copy_mode.anchor = None;
            copy_mode.source_anchor = None;
        }
        true
    }

    fn enter_wheel_target_quick_select(
        &mut self,
        pane_id: rterm_core::PaneId,
        options: WindowQuickSelectOptions,
    ) {
        let scope_lines = options
            .scope_lines
            .unwrap_or(DEFAULT_QUICK_SELECT_SCOPE_LINES);
        let alphabet = options
            .alphabet
            .unwrap_or_else(|| self.quick_select_alphabet.clone());
        let owned_patterns = options.patterns.map_or_else(
            || {
                let mut patterns = Vec::new();
                if !self.disable_default_quick_select_patterns {
                    patterns.extend(
                        QUICK_SELECT_PATTERNS
                            .iter()
                            .map(|pattern| (pattern.regex.to_owned(), pattern.capture)),
                    );
                }
                patterns.extend(
                    self.quick_select_patterns
                        .iter()
                        .cloned()
                        .map(|pattern| (pattern, None)),
                );
                patterns
            },
            |patterns| {
                patterns
                    .into_iter()
                    .map(|pattern| (pattern, None))
                    .collect()
            },
        );
        let patterns = owned_patterns
            .iter()
            .map(|(regex, capture)| WindowQuickSelectPatternRef {
                regex,
                capture: *capture,
            })
            .collect::<Vec<_>>();
        let Some(runtime) = self.pane_runtime_ref(pane_id) else {
            return;
        };
        let offset = self.pane_ui_ref(pane_id).map_or(0, |ui| {
            ui.stable_viewport.scrollback_offset(runtime.terminal())
        });
        let (row_start, row_end) =
            quick_select_source_row_scope(runtime.terminal(), offset, scope_lines);
        let matches = find_window_quick_select_matches_with_patterns(
            runtime.terminal(),
            &patterns,
            row_start,
            row_end,
        );
        let labels = quick_select_labels_for_alphabet_by_match(&alphabet, matches.len());
        let quick_select = WindowQuickSelect {
            current: 0,
            matches,
            labels,
            input: String::new(),
            reflow_config: Some(WindowQuickSelectReflowConfig {
                alphabet,
                patterns: patterns
                    .iter()
                    .map(|pattern| (pattern.regex.to_owned(), pattern.capture))
                    .collect(),
                scope_lines,
            }),
            action_label: options.label,
            action: options.action.unwrap_or_default(),
            skip_action_on_paste: options.skip_action_on_paste,
        };
        if pane_id == self.app_shell.active_pane_id() {
            self.active_ui.enter_quick_select(quick_select);
        } else if let Some(runtime) = self.pane_runtimes.get_mut(&pane_id) {
            runtime.ui.enter_quick_select(quick_select);
        }
        self.refresh_wheel_target_owner(pane_id);
    }

    fn copy_wheel_target_selection(
        &mut self,
        pane_id: rterm_core::PaneId,
        destination: WindowCopyDestination,
    ) {
        if let Some(text) = self.wheel_target_selected_text(pane_id) {
            self.write_text_to_copy_destination(&text, destination);
        }
    }

    fn wheel_target_hyperlink(&self, target: WheelTarget) -> Option<Arc<str>> {
        let snapshot = self.pane_snapshot(target.pane_id)?;
        snapshot
            .iter_cells()
            .find(|cell| cell.row == target.cell.row && cell.column == target.cell.column)
            .and_then(|cell| cell.hyperlink.clone())
            .or_else(|| self.hyperlink_rule_at_cell(snapshot, target.cell.row, target.cell.column))
    }

    fn open_wheel_target_link(&mut self, target: WheelTarget) {
        let Some(uri) = self.wheel_target_hyperlink(target) else {
            return;
        };
        let event = NativeWindowOpenUri {
            window_id: self.app_window_id,
            pane: target.pane_id,
            uri: uri.to_string(),
        };
        if self.dispatch_open_uri_in_context(&event, Some(target)) {
            (self.hyperlink_opener)(&uri);
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
    )]
    fn apply_wheel_pane_ui_command(
        &mut self,
        target: WheelTarget,
        command: WindowCommand,
    ) -> io::Result<()> {
        match command {
            WindowCommand::ActivateCopyMode | WindowCommand::EnterCopyMode => {
                self.enter_wheel_target_copy_mode(target.pane_id);
            }
            WindowCommand::CopyMode(assignment) => self
                .apply_wheel_copy_mode_assignment(target.pane_id, assignment)
                .map_err(|error| {
                    wheel_action_io_error(&WindowCommand::CopyMode(assignment), error)
                })?,
            WindowCommand::Search(query) => {
                self.enter_wheel_target_search(target.pane_id, Some(query));
            }
            WindowCommand::EnterSearch => {
                self.enter_wheel_target_search(target.pane_id, None);
            }
            WindowCommand::QuickSelect(options) | WindowCommand::QuickSelectArgs(options) => {
                self.enter_wheel_target_quick_select(target.pane_id, options);
            }
            WindowCommand::EnterQuickSelect => {
                self.enter_wheel_target_quick_select(
                    target.pane_id,
                    WindowQuickSelectOptions::default(),
                );
            }
            WindowCommand::ClearSelection => {
                self.set_wheel_target_selection(target.pane_id, None);
            }
            WindowCommand::CopyToClipboard | WindowCommand::Copy => {
                self.copy_wheel_target_selection(target.pane_id, WindowCopyDestination::Clipboard);
            }
            WindowCommand::CopyToPrimarySelection => self.copy_wheel_target_selection(
                target.pane_id,
                WindowCopyDestination::PrimarySelection,
            ),
            WindowCommand::CopyToClipboardAndPrimarySelection => {
                self.copy_wheel_target_selection(
                    target.pane_id,
                    WindowCopyDestination::ClipboardAndPrimarySelection,
                );
            }
            WindowCommand::CopyTo(destination) => {
                self.copy_wheel_target_selection(target.pane_id, destination);
            }
            WindowCommand::CompleteSelection => {
                self.complete_wheel_target_selection_to(
                    target.pane_id,
                    WindowCopyDestination::ClipboardAndPrimarySelection,
                );
            }
            WindowCommand::CompleteSelectionTo(destination) => {
                self.complete_wheel_target_selection_to(target.pane_id, destination);
            }
            WindowCommand::SelectTextAtMouseCursorCell => {
                self.select_wheel_target_text(target, WindowMouseSelectionMode::Cell);
            }
            WindowCommand::SelectTextAtMouseCursorWord => {
                self.select_wheel_target_text(target, WindowMouseSelectionMode::Word);
            }
            WindowCommand::SelectTextAtMouseCursorLine => {
                self.select_wheel_target_text(target, WindowMouseSelectionMode::Line);
            }
            WindowCommand::SelectTextAtMouseCursorBlock => {
                self.select_wheel_target_text(target, WindowMouseSelectionMode::Block);
            }
            WindowCommand::SelectTextAtMouseCursorSemanticZone => {
                self.select_wheel_target_text(target, WindowMouseSelectionMode::SemanticZone);
            }
            WindowCommand::SelectTextAtMouseCursor(mode) => {
                self.select_wheel_target_text(target, mode);
            }
            WindowCommand::ExtendSelectionToMouseCursorCell => {
                self.extend_wheel_target_selection(target, WindowMouseSelectionMode::Cell);
            }
            WindowCommand::ExtendSelectionToMouseCursorWord => {
                self.extend_wheel_target_selection(target, WindowMouseSelectionMode::Word);
            }
            WindowCommand::ExtendSelectionToMouseCursorLine => {
                self.extend_wheel_target_selection(target, WindowMouseSelectionMode::Line);
            }
            WindowCommand::ExtendSelectionToMouseCursorBlock => {
                self.extend_wheel_target_selection(target, WindowMouseSelectionMode::Block);
            }
            WindowCommand::ExtendSelectionToMouseCursorSemanticZone => {
                self.extend_wheel_target_selection(target, WindowMouseSelectionMode::SemanticZone);
            }
            WindowCommand::ExtendSelectionToMouseCursor(mode) => {
                self.extend_wheel_target_selection(target, mode);
            }
            WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursor
            | WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(_) => {
                let destination = match command {
                    WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursor => {
                        WindowCopyDestination::ClipboardAndPrimarySelection
                    }
                    WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(destination) => {
                        destination
                    }
                    _ => unreachable!(),
                };
                if target.pane_id == self.app_shell.active_pane_id() && self.selecting {
                    self.complete_selection_to(destination);
                } else {
                    self.open_wheel_target_link(target);
                }
            }
            WindowCommand::OpenLinkAtMouseCursor => self.open_wheel_target_link(target),
            _ => unreachable!("pane UI wheel command classification must be exhaustive"),
        }
        Ok(())
    }

    fn complete_wheel_target_selection_to(
        &mut self,
        pane_id: rterm_core::PaneId,
        destination: WindowCopyDestination,
    ) -> bool {
        if pane_id == self.app_shell.active_pane_id() {
            return self.complete_selection_to(destination);
        }
        let single_cell = self
            .pane_ui_ref(pane_id)
            .and_then(|ui| ui.ordinary_selection)
            .is_some_and(StableOrdinarySelection::is_single_cell);
        if single_cell {
            self.set_wheel_target_selection(pane_id, None);
            return false;
        }
        let Some(text) = self.wheel_target_selected_text(pane_id) else {
            return false;
        };
        self.write_text_to_copy_destination(&text, destination)
    }

    fn apply_wheel_pane_action(
        &mut self,
        target: WheelTarget,
        command: WindowCommand,
    ) -> io::Result<()> {
        let pane = target.pane_id;
        let result = match command {
            WindowCommand::CloseCurrentPane { confirm: false } | WindowCommand::ClosePane => {
                self.dispatch_app_action(AppAction::ClosePane { pane })
            }
            WindowCommand::CloseCurrentPane { confirm: true } => {
                self.request_close_confirmation_or_close(WindowCloseTarget::Pane(pane));
                Ok(())
            }
            WindowCommand::ResetTerminal => return self.handle_pane_pty_output(pane, b"\x1bc"),
            WindowCommand::RestartPane => {
                return self
                    .restart_pane_runtime(pane)
                    .map_err(|error| io::Error::other(error.to_string()));
            }
            WindowCommand::InspectPane => {
                self.request_pane_inspection(pane);
                return Ok(());
            }
            WindowCommand::ClearScrollback(WindowClearScrollbackMode::ScrollbackOnly) => {
                if pane == self.app_shell.active_pane_id() {
                    self.active_ui.stable_viewport = PaneStableViewport::default();
                } else if let Some(runtime) = self.pane_runtimes.get_mut(&pane) {
                    runtime.ui.stable_viewport = PaneStableViewport::default();
                }
                return self.handle_pane_pty_output(pane, b"\x1b[3J");
            }
            WindowCommand::ClearScrollback(WindowClearScrollbackMode::ScrollbackAndViewport)
            | WindowCommand::ClearScrollbackAndViewport => {
                self.clear_wheel_target_scrollback_and_viewport(pane)
                    .map_err(|error| wheel_action_io_error(&command, error))?;
                return Ok(());
            }
            WindowCommand::AdjustPaneSize { direction, amount } => {
                self.dispatch_app_action(AppAction::ResizePane {
                    pane,
                    direction,
                    amount,
                })
            }
            WindowCommand::ResizePaneLeft => self.dispatch_app_action(AppAction::ResizePane {
                pane,
                direction: ResizeDirection::Left,
                amount: 1,
            }),
            WindowCommand::ResizePaneRight => self.dispatch_app_action(AppAction::ResizePane {
                pane,
                direction: ResizeDirection::Right,
                amount: 1,
            }),
            WindowCommand::ResizePaneUp => self.dispatch_app_action(AppAction::ResizePane {
                pane,
                direction: ResizeDirection::Up,
                amount: 1,
            }),
            WindowCommand::ResizePaneDown => self.dispatch_app_action(AppAction::ResizePane {
                pane,
                direction: ResizeDirection::Down,
                amount: 1,
            }),
            WindowCommand::TogglePaneZoom | WindowCommand::TogglePaneZoomState => {
                self.dispatch_app_action(AppAction::TogglePaneZoom { pane })
            }
            WindowCommand::SetPaneZoomState(zoomed) => {
                self.dispatch_app_action(AppAction::SetPaneZoomState { pane, zoomed })
            }
            WindowCommand::ZoomPane => {
                self.dispatch_app_action(AppAction::SetPaneZoomState { pane, zoomed: true })
            }
            WindowCommand::UnzoomPane => self.dispatch_app_action(AppAction::SetPaneZoomState {
                pane,
                zoomed: false,
            }),
            other => {
                let original = other.clone();
                return self
                    .command_palette_apply_command(other)
                    .map_err(|error| wheel_action_io_error(&original, error));
            }
        };
        result.map_err(|error| wheel_action_io_error(&command, error))
    }

    fn clear_wheel_target_scrollback_and_viewport(
        &mut self,
        pane_id: rterm_core::PaneId,
    ) -> Result<(), AppShellError> {
        if pane_id == self.app_shell.active_pane_id() {
            self.clear_scrollback_and_viewport();
            return Ok(());
        }
        let Some(runtime) = self.pane_runtimes.get_mut(&pane_id) else {
            return Err(AppShellError::InvalidPane(pane_id));
        };
        runtime.ui.stable_viewport = PaneStableViewport::default();
        runtime.ui.retire_terminal_identity();
        let damage = runtime.runtime.erase_scrollback_and_viewport();
        runtime.reconcile_terminal_mutation();
        self.metrics.record_damage(&damage);
        self.metrics.record_snapshot_rebuild();
        self.frame_needs_full_repaint = true;
        self.pending_frame_damage.clear();
        Ok(())
    }

    fn wheel_direction_destination(
        &self,
        pane: rterm_core::PaneId,
        direction: rterm_core::app_shell::PaneDirection,
    ) -> Result<rterm_core::PaneId, AppShellError> {
        let mut shell = self.app_shell.clone();
        shell.apply_action(AppAction::ActivatePane { pane })?;
        shell.apply_action(AppAction::ActivatePaneDirection { direction })?;
        Ok(shell.active_pane_id())
    }

    fn wheel_reference_launch(&self, pane_id: rterm_core::PaneId) -> Option<PaneLaunch> {
        self.app_shell
            .workspaces()
            .iter()
            .flat_map(rterm_core::app_shell::Workspace::tabs)
            .flat_map(rterm_core::app_shell::Tab::panes)
            .find(|pane| pane.id() == pane_id)
            .map(|pane| pane.launch().for_child_pane())
    }

    fn default_pane_launch_with_options_for_wheel(
        &self,
        pane_id: rterm_core::PaneId,
        options: WindowSpawnCommandQueryOptions,
    ) -> Result<PaneLaunch, AppShellError> {
        if let Some(domain) = &options.domain
            && !domain.is_supported_local_domain(&self.default_domain)
        {
            return Err(AppShellError::UnsupportedAction);
        }
        let mut launch = self.default_prog_launch_for_wheel(pane_id)?;
        if let Some(cwd) = options.cwd {
            launch = launch.with_cwd(cwd);
        }
        Ok(launch.with_environment(options.environment))
    }

    fn default_prog_launch_for_wheel(
        &self,
        pane_id: rterm_core::PaneId,
    ) -> Result<PaneLaunch, AppShellError> {
        let reference_launch = self
            .wheel_reference_launch(pane_id)
            .ok_or(AppShellError::InvalidPane(pane_id))?;
        if matches!(reference_launch.domain(), PaneLaunchDomain::Ssh(_)) {
            return Ok(reference_launch);
        }
        let Some((program, args)) = self
            .default_prog
            .as_ref()
            .and_then(|value| value.split_first())
        else {
            return Ok(reference_launch);
        };
        if program.is_empty() {
            return Ok(reference_launch);
        }
        let mut launch = PaneLaunch::local(program.clone()).with_args(args.iter().cloned());
        if let Some(cwd) = self.pane_launch_current_working_dir(pane_id) {
            launch = launch.with_cwd(cwd.to_owned());
        }
        Ok(launch)
    }

    fn switch_to_workspace_launch_for_wheel(
        &self,
        pane_id: rterm_core::PaneId,
        command: Option<WindowSpawnCommandQuery>,
        command_options: Option<WindowSpawnCommandQueryOptions>,
    ) -> Result<PaneLaunch, AppShellError> {
        match (command, command_options) {
            (Some(command), _) => self.supported_pane_launch(command),
            (None, Some(options)) => {
                self.default_pane_launch_with_options_for_wheel(pane_id, options)
            }
            (None, None) => self.default_prog_launch_for_wheel(pane_id),
        }
    }

    fn wheel_split_pane_app_action(
        &self,
        pane_id: rterm_core::PaneId,
        split_pane: WindowSplitPaneOptions,
    ) -> Result<AppAction, AppShellError> {
        if let Some(domain) = &split_pane.domain
            && !domain.is_supported_local_domain(&self.default_domain)
        {
            return Err(AppShellError::UnsupportedAction);
        }
        let source_cells = self
            .pane_render_rect(pane_id)
            .map(|rect| match split_pane.direction {
                SplitDirection::Left | SplitDirection::Right => rect.columns,
                SplitDirection::Up | SplitDirection::Down => rect.rows,
            })
            .unwrap_or_default();
        let source_size_delta = split_pane
            .size
            .map(|size| split_pane_source_size_delta(source_cells, size))
            .unwrap_or_default();
        let launch = match split_pane.command {
            Some(mut command) => {
                if command.domain.is_none() {
                    command.domain = split_pane.domain;
                }
                Some(self.supported_pane_launch(command)?)
            }
            None => match split_pane.command_options {
                Some(options) => {
                    Some(self.default_pane_launch_with_options_for_wheel(pane_id, options)?)
                }
                None => Some(self.default_prog_launch_for_wheel(pane_id)?),
            },
        };
        if split_pane.top_level {
            Ok(AppAction::SplitTopLevelPaneWithSize {
                direction: split_pane.direction,
                launch,
                source_size_delta,
            })
        } else {
            Ok(AppAction::SplitPaneWithSize {
                pane: pane_id,
                direction: split_pane.direction,
                launch,
                source_size_delta,
            })
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
    )]
    fn apply_wheel_explicit_command(
        &mut self,
        target: WheelTarget,
        command: WindowCommand,
    ) -> io::Result<()> {
        let pane = target.pane_id;
        let direction = match command {
            WindowCommand::ActivatePaneLeft => Some(rterm_core::app_shell::PaneDirection::Left),
            WindowCommand::ActivatePaneRight => Some(rterm_core::app_shell::PaneDirection::Right),
            WindowCommand::ActivatePaneUp => Some(rterm_core::app_shell::PaneDirection::Up),
            WindowCommand::ActivatePaneDown => Some(rterm_core::app_shell::PaneDirection::Down),
            WindowCommand::ActivatePaneDirection(direction) => Some(direction),
            WindowCommand::NextPane => Some(rterm_core::app_shell::PaneDirection::Next),
            WindowCommand::PreviousPane => Some(rterm_core::app_shell::PaneDirection::Previous),
            _ => None,
        };
        if let Some(direction) = direction {
            let destination = self
                .wheel_direction_destination(pane, direction)
                .map_err(|error| wheel_action_io_error(&command, error))?;
            return self
                .dispatch_app_action(AppAction::ActivatePane { pane: destination })
                .map_err(|error| wheel_action_io_error(&command, error));
        }

        let action = match command {
            WindowCommand::SplitRight | WindowCommand::SplitHorizontal => {
                Some(AppAction::SplitPane {
                    pane,
                    direction: SplitDirection::Right,
                    launch: Some(
                        self.default_prog_launch_for_wheel(pane)
                            .map_err(|error| wheel_action_io_error(&command, error))?,
                    ),
                })
            }
            WindowCommand::SplitDown | WindowCommand::SplitVertical => Some(AppAction::SplitPane {
                pane,
                direction: SplitDirection::Down,
                launch: Some(
                    self.default_prog_launch_for_wheel(pane)
                        .map_err(|error| wheel_action_io_error(&command, error))?,
                ),
            }),
            _ => None,
        };
        if let Some(action) = action {
            return self
                .dispatch_app_action(action)
                .map_err(|error| wheel_action_io_error(&command, error));
        }

        if let WindowCommand::SplitPane(options) = &command {
            let original = command.clone();
            let action = self
                .wheel_split_pane_app_action(pane, options.clone())
                .map_err(|error| wheel_action_io_error(&original, error))?;
            return self
                .dispatch_app_action(action)
                .map_err(|error| wheel_action_io_error(&original, error));
        }

        if matches!(command, WindowCommand::NewTab | WindowCommand::SpawnTab(_)) {
            if let WindowCommand::SpawnTab(domain) = &command
                && !domain.is_supported_local_domain(&self.default_domain)
            {
                return Err(wheel_action_io_error(
                    &command,
                    AppShellError::UnsupportedAction,
                ));
            }
            let action = AppAction::NewTab {
                launch: Some(
                    self.default_prog_launch_for_wheel(pane)
                        .map_err(|error| wheel_action_io_error(&command, error))?,
                ),
            };
            return self
                .dispatch_app_action(action)
                .map_err(|error| wheel_action_io_error(&command, error));
        }

        if let WindowCommand::SpawnCommandInNewTab(spawn) = command {
            let original = WindowCommand::SpawnCommandInNewTab(spawn.clone());
            let launch = self
                .supported_pane_launch(spawn)
                .map_err(|error| wheel_action_io_error(&original, error))?;
            return self
                .dispatch_app_action(AppAction::NewTab {
                    launch: Some(launch),
                })
                .map_err(|error| wheel_action_io_error(&original, error));
        }

        if let WindowCommand::SpawnCommandOptionsInNewTab(options) = command {
            let original = WindowCommand::SpawnCommandOptionsInNewTab(options.clone());
            let launch = self
                .default_pane_launch_with_options_for_wheel(pane, options)
                .map_err(|error| wheel_action_io_error(&original, error))?;
            return self
                .dispatch_app_action(AppAction::NewTab {
                    launch: Some(launch),
                })
                .map_err(|error| wheel_action_io_error(&original, error));
        }

        if command == WindowCommand::SpawnWindow {
            let launch = self
                .default_prog_launch_for_wheel(pane)
                .map(Some)
                .map_err(|error| wheel_action_io_error(&command, error))?;
            return self
                .dispatch_spawn_window_or_preferred_tab(launch, None)
                .map_err(|error| wheel_action_io_error(&command, error));
        }

        if let WindowCommand::SpawnCommandOptionsInNewWindow(options) = command {
            let original = WindowCommand::SpawnCommandOptionsInNewWindow(options.clone());
            let position = options.window_position.clone();
            let launch = self
                .default_pane_launch_with_options_for_wheel(pane, options)
                .map_err(|error| wheel_action_io_error(&original, error))?;
            return self
                .dispatch_spawn_window_or_preferred_tab(Some(launch), position)
                .map_err(|error| wheel_action_io_error(&original, error));
        }

        if let WindowCommand::SpawnCommandInNewWindow(spawn) = command {
            let original = WindowCommand::SpawnCommandInNewWindow(spawn.clone());
            let position = spawn.window_position.clone();
            let launch = self
                .supported_pane_launch(spawn)
                .map_err(|error| wheel_action_io_error(&original, error))?;
            return self
                .dispatch_spawn_window_or_preferred_tab(Some(launch), position)
                .map_err(|error| wheel_action_io_error(&original, error));
        }

        let workspace_original = command.clone();
        let workspace_action =
            match command.clone() {
                WindowCommand::NewWorkspace => Some(AppAction::NewWorkspace {
                    name: format!("workspace-{}", self.app_shell.workspaces().len() + 1),
                    launch: Some(self.default_prog_launch_for_wheel(pane).map_err(|error| {
                        wheel_action_io_error(&WindowCommand::NewWorkspace, error)
                    })?),
                }),
                WindowCommand::SwitchToWorkspace => Some(AppAction::SwitchToWorkspace {
                    name: None,
                    launch: Some(self.default_prog_launch_for_wheel(pane).map_err(|error| {
                        wheel_action_io_error(&WindowCommand::SwitchToWorkspace, error)
                    })?),
                }),
                WindowCommand::SwitchToWorkspaceArgs(args) => {
                    let original = WindowCommand::SwitchToWorkspaceArgs(args.clone());
                    Some(AppAction::SwitchToWorkspace {
                        name: args.name,
                        launch: Some(
                            self.switch_to_workspace_launch_for_wheel(
                                pane,
                                args.command,
                                args.command_options,
                            )
                            .map_err(|error| wheel_action_io_error(&original, error))?,
                        ),
                    })
                }
                WindowCommand::SwitchToWorkspaceName(name) => Some(AppAction::SwitchToWorkspace {
                    name: Some(name),
                    launch: Some(self.default_prog_launch_for_wheel(pane).map_err(|error| {
                        wheel_action_io_error(
                            &WindowCommand::SwitchToWorkspaceName(String::new()),
                            error,
                        )
                    })?),
                }),
                _ => None,
            };
        if let Some(action) = workspace_action {
            return self
                .dispatch_app_action(action)
                .map_err(|error| wheel_action_io_error(&workspace_original, error));
        }

        let original = command.clone();
        self.command_palette_apply_command(command)
            .map_err(|error| wheel_action_io_error(&original, error))
    }

    #[expect(
        clippy::unused_self,
        reason = "method shape is retained for compatibility call-site consistency"
    )]
    fn encode_wheel_mouse_event_for_target(
        &self,
        target: WheelTarget,
        kind: WindowMouseEventKind,
        mode: MouseInputMode,
        modifiers: ModifiersState,
    ) -> Option<Vec<u8>> {
        let event = WindowMouseEvent {
            kind,
            column: target.cell.column,
            row: target.cell.row,
            modifiers,
        };
        match mode.protocol() {
            MouseProtocolMode::SgrPixels => {
                let (x_pixels, y_pixels) =
                    mouse_report_pixel_coordinate(target.pixel_position.x)
                        .zip(mouse_report_pixel_coordinate(target.pixel_position.y))?;
                encode_window_mouse_event_with_pixels(event, x_pixels, y_pixels, mode)
            }
            _ => encode_window_mouse_event(event, mode),
        }
    }

    fn handle_alternate_buffer_mouse_wheel_for_target(
        &mut self,
        target: WheelTarget,
        delta: MouseScrollDelta,
    ) -> io::Result<bool> {
        let lines = scrollback_lines_from_mouse_delta(delta);
        let key = match lines.cmp(&0) {
            std::cmp::Ordering::Greater => NamedKey::ArrowUp,
            std::cmp::Ordering::Less => NamedKey::ArrowDown,
            std::cmp::Ordering::Equal => return Ok(false),
        };
        let repeats = lines
            .unsigned_abs()
            .saturating_mul(self.alternate_buffer_wheel_scroll_speed);
        if repeats == 0 {
            return Ok(false);
        }
        let Some(runtime) = self.pane_runtime_ref(target.pane_id) else {
            return Ok(false);
        };
        let physical_key = match key {
            NamedKey::ArrowUp => PhysicalKey::Code(WinitKeyCode::ArrowUp),
            NamedKey::ArrowDown => PhysicalKey::Code(WinitKeyCode::ArrowDown),
            _ => PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
        };
        let kitty_flags = runtime.kitty_keyboard_flags()
            | (u16::from(self.enable_csi_u_key_encoding) * KITTY_KEYBOARD_DISAMBIGUATE);
        let bytes = encode_window_key_with_kitty_event(
            &Key::Named(key),
            physical_key,
            None,
            ModifiersState::empty(),
            runtime.application_cursor_keys(),
            runtime.application_keypad(),
            kitty_flags,
            runtime.modify_other_keys(),
            KittyKeyEventKind::Press,
        );
        for _ in 0..repeats {
            self.write_pty_bytes_to_pane_for_wheel(target.pane_id, &bytes)?;
        }
        Ok(true)
    }

    fn handle_tab_bar_mouse_wheel(&mut self, delta: MouseScrollDelta) -> bool {
        let lines = scrollback_lines_from_mouse_delta(delta);
        if lines == 0 {
            return false;
        }
        match self.tab_bar_wheel_behavior {
            NativeTabBarWheelBehavior::Disabled => false,
            NativeTabBarWheelBehavior::Switch => {
                let offset = if lines > 0 { -1 } else { 1 };
                if let Err(error) =
                    self.dispatch_app_action(AppAction::ActivateTabRelative { offset })
                {
                    eprintln!("tab bar wheel activation failed: {error:?}");
                    return false;
                }
                true
            }
            NativeTabBarWheelBehavior::Scroll => {
                let tab_count = self.app_shell.active_workspace().tabs().len();
                let next = if lines > 0 {
                    self.tab_bar_scroll_position.saturating_sub(1)
                } else {
                    self.tab_bar_scroll_position
                        .saturating_add(1)
                        .min(tab_count.saturating_sub(1))
                };
                if next != self.tab_bar_scroll_position {
                    self.tab_bar_scroll_position = next;
                    self.rendered_tab_bar_layout.replace(None);
                    self.frame_needs_full_repaint = true;
                }
                true
            }
        }
    }

    fn mouse_position_is_in_tab_bar(&self) -> bool {
        let Some(position) = self.mouse_pixel_position else {
            return false;
        };
        let content_left = f64::from(self.frame_content_pixel_left());
        let content_right = f64::from(
            self.frame_content_pixel_left()
                .saturating_add(self.frame_content_placement().width),
        );
        position.x.is_finite()
            && position.y.is_finite()
            && position.x >= content_left
            && position.x < content_right
            && position.y >= f64::from(self.tab_bar_pixel_top())
            && position.y
                < f64::from(
                    self.tab_bar_pixel_top()
                        .saturating_add(self.tab_bar_pixel_height()),
                )
    }

    fn encode_window_mouse_event_for_position(
        &self,
        event: WindowMouseEvent,
        mode: MouseInputMode,
        position: Option<PhysicalPosition<f64>>,
    ) -> Option<Vec<u8>> {
        position
            .and_then(|position| self.mouse_report_pixels_for_position(position))
            .and_then(|(x_pixels, y_pixels)| {
                encode_window_mouse_event_with_pixels(event, x_pixels, y_pixels, mode)
            })
            .or_else(|| encode_window_mouse_event(event, mode))
    }

    fn mouse_report_pixels_for_position(
        &self,
        position: PhysicalPosition<f64>,
    ) -> Option<(u16, u16)> {
        let x = position.x - f64::from(self.frame_content_pixel_left());
        let terminal_y = position.y - f64::from(self.terminal_pixel_top());
        Some((
            mouse_report_pixel_coordinate(x)?,
            mouse_report_pixel_coordinate(terminal_y)?,
        ))
    }

    #[expect(
        clippy::too_many_lines,
        reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
    )]
    fn handle_mouse_input(&mut self, state: ElementState, button: MouseButton) -> io::Result<bool> {
        if self.pane_inspection_input_barrier_active() {
            if self.handle_pending_ui_left_release(state, button) {
                return Ok(true);
            }
            self.mark_ui_left_press_consumed(state, button);
            return Ok(true);
        }
        let kind = match state {
            ElementState::Pressed => WindowMouseEventKind::Down(button),
            ElementState::Released => WindowMouseEventKind::Up(button),
        };
        let released_active_mouse_button =
            state == ElementState::Released && self.active_mouse_button == Some(button);
        self.update_active_mouse_button(state, button);
        if released_active_mouse_button {
            self.mouse_click_may_focus_window = false;
        }

        if button == MouseButton::Left {
            match state {
                ElementState::Pressed => {
                    self.tab_bar_drag = None;
                }
                ElementState::Released if self.handle_tab_bar_drag_release() => {
                    return Ok(true);
                }
                ElementState::Released => {}
            }
        }

        if self.handle_pending_ui_left_release(state, button) {
            return Ok(true);
        }

        let mouse_click_focuses_window = state == ElementState::Pressed
            && (!self.window_focused || self.mouse_click_may_focus_window);
        if mouse_click_focuses_window {
            self.mouse_click_may_focus_window = false;
            if self.swallow_mouse_click_on_window_focus {
                return Ok(true);
            }
        }

        if self.pane_select.is_some() {
            self.exit_pane_select_mode();
            self.mark_ui_left_press_consumed(state, button);
            return Ok(true);
        }

        if self.handle_input_selector_mouse_input(state, button) {
            self.mark_ui_left_press_consumed(state, button);
            return Ok(true);
        }

        if self.handle_workspace_chrome_mouse_input(state, button) {
            return Ok(true);
        }

        if self.handle_pane_close_button_mouse_input(state, button) {
            return Ok(true);
        }

        if self.higher_level_ui_blocks_pane_surface_mouse() {
            self.mark_ui_left_press_consumed(state, button);
            return Ok(true);
        }

        if self.handle_tab_bar_mouse_input(state, button) {
            self.mark_ui_left_press_consumed(state, button);
            return Ok(true);
        }

        let mode = self.runtime.mouse_input_mode();
        let alternate_screen_active = self.runtime.terminal().alternate_screen_active();
        let bypass_mouse_reporting = mode.reporting_enabled()
            && !self.bypass_mouse_reporting_modifiers.is_empty()
            && self
                .modifiers
                .contains(self.bypass_mouse_reporting_modifiers);
        let mouse_reporting_for_assignment = mode.reporting_enabled() && !bypass_mouse_reporting;
        let assignment_kind = match state {
            ElementState::Pressed => NativeMouseAssignmentEventKind::Down,
            ElementState::Released => NativeMouseAssignmentEventKind::Up,
        };
        let assignment_button = NativeMouseAssignmentButton::Mouse(button);
        let assignment_streak = self.mouse_assignment_streak(
            state,
            button,
            mouse_reporting_for_assignment,
            alternate_screen_active,
        );
        if self.handle_user_mouse_assignment(
            assignment_kind,
            assignment_button,
            assignment_streak,
            mouse_reporting_for_assignment,
            alternate_screen_active,
        ) {
            return Ok(true);
        }

        let default_mouse_bindings_enabled = !self.disable_default_mouse_bindings
            && (!mode.reporting_enabled() || bypass_mouse_reporting)
            && !self.user_mouse_assignment_overrides_default_for_button(
                assignment_button,
                assignment_streak,
                mouse_reporting_for_assignment,
                alternate_screen_active,
            );
        if default_mouse_bindings_enabled
            && state == ElementState::Pressed
            && window_start_drag_mouse_binding(button, self.modifiers)
        {
            return Ok(false);
        }

        if default_mouse_bindings_enabled
            && state == ElementState::Pressed
            && button == MouseButton::Middle
            && self.modifiers.is_empty()
        {
            return self.handle_window_primary_selection_paste();
        }

        if self.handle_split_resize_mouse_input(state, button) {
            return Ok(true);
        }

        if self.handle_scrollbar_mouse_input(state, button) {
            return Ok(true);
        }

        let active_pane_before_click = self.app_shell.active_pane_id();
        let mouse_cell = if state == ElementState::Pressed {
            self.focus_pane_for_mouse_position()
        } else {
            self.mouse_cell_for_active_pane()
        };
        let pane_focus_changed_by_click = state == ElementState::Pressed
            && active_pane_before_click != self.app_shell.active_pane_id();
        if pane_focus_changed_by_click && self.swallow_mouse_click_on_pane_focus {
            return Ok(true);
        }
        if pane_focus_changed_by_click && self.active_ui.overlay_active() {
            return Ok(true);
        }

        if !mode.reporting_enabled() || bypass_mouse_reporting {
            let saved_modifiers = self.modifiers;
            if bypass_mouse_reporting {
                let bypass_modifiers = self.bypass_mouse_reporting_modifiers;
                self.modifiers.remove(bypass_modifiers);
            }
            let handled = default_mouse_bindings_enabled
                && (self.handle_hyperlink_mouse_input(state, button)
                    || self.handle_selection_mouse_input(state, button));
            self.modifiers = saved_modifiers;
            return Ok(handled);
        }

        let Some(mouse_cell @ PaneMouseCell { column, row, .. }) = mouse_cell else {
            return Ok(false);
        };

        let event = WindowMouseEvent {
            kind,
            column,
            row,
            modifiers: self.modifiers,
        };
        let Some(bytes) =
            self.encode_window_mouse_event_for_position(event, mode, self.mouse_pixel_position)
        else {
            return Ok(false);
        };

        self.record_reported_iterm_mouse_info(mouse_cell, kind, self.modifiers);
        self.write_pty_bytes(&bytes)?;
        Ok(true)
    }

    fn handle_pane_close_button_mouse_input(
        &mut self,
        state: ElementState,
        button: MouseButton,
    ) -> bool {
        if button != MouseButton::Left {
            return false;
        }

        match state {
            ElementState::Pressed => {
                self.ui_left_release_pending = false;
                self.pressed_pane_close_button = None;
                let Some(pane) = self.pane_close_button_at_mouse_position() else {
                    return false;
                };

                self.ui_left_release_pending = true;
                self.pressed_pane_close_button = Some(pane);
                self.clear_ordinary_selection();
                self.selecting = false;
                self.last_left_click = None;
                self.request_close_confirmation_or_close(WindowCloseTarget::Pane(pane));
                true
            }
            ElementState::Released => {
                let handled = self.pressed_pane_close_button.take().is_some();
                if handled {
                    self.ui_left_release_pending = false;
                }
                handled
            }
        }
    }

    fn handle_pending_ui_left_release(&mut self, state: ElementState, button: MouseButton) -> bool {
        if button != MouseButton::Left {
            return false;
        }

        match state {
            ElementState::Pressed => {
                self.ui_left_release_pending = false;
                self.pressed_pane_close_button = None;
                false
            }
            ElementState::Released if self.ui_left_release_pending => {
                self.ui_left_release_pending = false;
                self.pressed_pane_close_button = None;
                true
            }
            ElementState::Released => false,
        }
    }

    fn mark_ui_left_press_consumed(&mut self, state: ElementState, button: MouseButton) {
        if state == ElementState::Pressed && button == MouseButton::Left {
            self.ui_left_release_pending = true;
        }
    }

    fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) -> io::Result<bool> {
        self.set_mouse_cursor_visible(true);
        self.mouse_pixel_position = Some(position);
        let next_position = self.window_mouse_cell(position);
        let mouse_cell_changed = self.mouse_position != next_position;
        self.mouse_position = next_position;
        if self.pane_inspection_input_barrier_active() {
            return Ok(true);
        }
        self.update_split_resize_cursor_icon();

        if self.update_tab_bar_drag_from_mouse_position() {
            return Ok(true);
        }
        if self.ui_left_release_pending {
            return Ok(true);
        }

        if self.scrollbar_dragging {
            return Ok(self.scroll_to_scrollbar_position(position));
        }

        if self.split_resize_dragging.is_some() {
            return Ok(self.resize_split_to_mouse_position());
        }

        let mode = self.runtime.mouse_input_mode();
        let alternate_screen_active = self.runtime.terminal().alternate_screen_active();
        let bypass_mouse_reporting = mode.reporting_enabled()
            && !self.bypass_mouse_reporting_modifiers.is_empty()
            && self
                .modifiers
                .contains(self.bypass_mouse_reporting_modifiers);
        let mouse_reporting_for_assignment = mode.reporting_enabled() && !bypass_mouse_reporting;
        if mouse_cell_changed && let Some(button) = self.active_mouse_button {
            let assignment_streak = self.active_mouse_assignment_streak(
                button,
                mouse_reporting_for_assignment,
                alternate_screen_active,
            );
            if self.handle_user_mouse_assignment(
                NativeMouseAssignmentEventKind::Drag,
                NativeMouseAssignmentButton::Mouse(button),
                assignment_streak,
                mouse_reporting_for_assignment,
                alternate_screen_active,
            ) {
                return Ok(true);
            }

            let default_mouse_bindings_enabled = !self.disable_default_mouse_bindings
                && (!mode.reporting_enabled() || bypass_mouse_reporting)
                && !self.user_mouse_assignment_overrides_default_for_button(
                    NativeMouseAssignmentButton::Mouse(button),
                    assignment_streak,
                    mouse_reporting_for_assignment,
                    alternate_screen_active,
                );
            if default_mouse_bindings_enabled
                && window_start_drag_mouse_binding(button, self.modifiers)
            {
                self.start_window_drag();
                return Ok(true);
            }
        }

        if !mouse_cell_changed {
            return Ok(false);
        }

        if self.pane_focus_follows_mouse && self.active_mouse_button.is_none() {
            let _ = self.focus_pane_for_mouse_position();
        }

        if !mode.reporting_enabled() || bypass_mouse_reporting {
            let saved_modifiers = self.modifiers;
            if bypass_mouse_reporting {
                let bypass_modifiers = self.bypass_mouse_reporting_modifiers;
                self.modifiers.remove(bypass_modifiers);
            }
            let handled = self.update_selection_from_mouse_position();
            self.modifiers = saved_modifiers;
            return Ok(handled);
        }

        let Some(mouse_cell @ PaneMouseCell { column, row, .. }) =
            self.mouse_cell_for_active_pane()
        else {
            return Ok(false);
        };
        let kind = match self.active_mouse_button {
            Some(button) => WindowMouseEventKind::Drag(button),
            None => WindowMouseEventKind::Moved,
        };

        let event = WindowMouseEvent {
            kind,
            column,
            row,
            modifiers: self.modifiers,
        };
        let Some(bytes) = self.encode_window_mouse_event_for_position(event, mode, Some(position))
        else {
            return Ok(false);
        };

        self.record_reported_iterm_mouse_info(mouse_cell, kind, self.modifiers);
        self.write_pty_bytes(&bytes)?;
        Ok(true)
    }
}

impl NativeWindowApp {
    fn workspace_chrome_enabled(&self) -> bool {
        self.window.is_some() && self.workspace_chrome_frame_rows() != 0
    }

    fn workspace_chrome_frame_rows(&self) -> u16 {
        workspace_chrome::frame_rows_for_layout(
            self.modern_tab_bar_uses_compact_labels(),
            self.tab_bar_is_visible() && !self.tab_bar_at_bottom,
        )
    }

    fn workspace_chrome_right_reserved_columns(&self) -> u16 {
        let right_buttons = if self.integrated_title_buttons_are_right_aligned() {
            self.integrated_title_buttons_tab_bar_width()
        } else {
            0
        };
        let status = tab_bar_ansi_visible_width(&self.right_status);
        u16::try_from(right_buttons.saturating_add(status)).unwrap_or(u16::MAX)
    }

    fn workspace_chrome_palette(&self) -> NativeResolvedPalette {
        let mut palette = self.native_resolved_palette();
        let explicit_palette = self.config_overrides.background_color.is_some()
            || self.config_overrides.colors.is_some()
            || self.config_overrides.color_scheme.is_some();
        if self.workspace_chrome_enabled() && !explicit_palette {
            palette.background = workspace_chrome::GRAPHITE_TERMINAL_BACKGROUND;
            palette.tab_bar_background = Some(workspace_chrome::GRAPHITE_TITLEBAR_COLOR);
        }
        palette
    }

    fn workspace_chrome_frame_cell_at_mouse(&self) -> Option<(u16, u16)> {
        let position = self.mouse_pixel_position?;
        if !position.x.is_finite() || !position.y.is_finite() {
            return None;
        }
        let x = position.x - f64::from(self.frame_content_pixel_left());
        let y = position.y - f64::from(self.frame_content_pixel_top());
        if x < 0.0 || y < 0.0 {
            return None;
        }
        Some((
            pixel_axis_to_cell(x, self.cell_width())?,
            pixel_axis_to_cell(y, self.cell_height())?,
        ))
    }

    fn workspace_chrome_action_at_mouse(&self) -> Option<workspace_chrome::WorkspaceChromeAction> {
        if !self.workspace_chrome_enabled() || self.tab_bar_at_bottom {
            return None;
        }
        let (column, row) = self.workspace_chrome_frame_cell_at_mouse()?;
        if row != self.tab_bar_frame_row() {
            return None;
        }
        workspace_chrome::action_at_column(
            column,
            self.window_terminal_size_for_layout().columns,
            self.workspace_chrome_right_reserved_columns(),
        )
    }

    fn workspace_chrome_pane_menu_at_mouse(&self) -> Option<rterm_core::PaneId> {
        if !self.workspace_chrome_enabled() || self.tab_bar_at_bottom {
            return None;
        }
        let (column, row) = self.workspace_chrome_frame_cell_at_mouse()?;
        self.pane_render_layout()
            .panes
            .into_iter()
            .find_map(|rect| {
                let menu_column = rect.column.saturating_add(rect.columns.saturating_sub(3));
                (row == rect.row.saturating_sub(1)
                    && column >= menu_column
                    && column < rect.column.saturating_add(rect.columns.saturating_sub(1)))
                .then_some(rect.pane_id)
            })
    }

    fn workspace_chrome_pane_close_at_mouse(&self) -> Option<rterm_core::PaneId> {
        if !self.workspace_chrome_enabled() || self.tab_bar_at_bottom {
            return None;
        }
        let (column, row) = self.workspace_chrome_frame_cell_at_mouse()?;
        self.pane_render_layout()
            .panes
            .into_iter()
            .find_map(|rect| {
                let close_column = rect.column.saturating_add(rect.columns.saturating_sub(1));
                (row == rect.row.saturating_sub(1) && column == close_column)
                    .then_some(rect.pane_id)
            })
    }

    fn handle_workspace_chrome_mouse_input(
        &mut self,
        state: ElementState,
        button: MouseButton,
    ) -> bool {
        if !self.workspace_chrome_enabled() {
            return false;
        }
        if button == MouseButton::Right {
            if state == ElementState::Pressed
                && let Some(action) = self.workspace_chrome_action_at_mouse()
            {
                match action {
                    workspace_chrome::WorkspaceChromeAction::Split => {
                        self.enter_workspace_split_menu();
                    }
                    workspace_chrome::WorkspaceChromeAction::Settings => {
                        self.enter_workspace_settings_menu();
                    }
                    workspace_chrome::WorkspaceChromeAction::Search => {}
                }
            }
            return self.workspace_chrome_action_at_mouse().is_some();
        }
        if button != MouseButton::Left || state == ElementState::Released {
            return false;
        }
        if let Some(action) = self.workspace_chrome_action_at_mouse() {
            self.mark_ui_left_press_consumed(state, button);
            match action {
                workspace_chrome::WorkspaceChromeAction::Search => self.enter_search_mode(),
                workspace_chrome::WorkspaceChromeAction::Split => {
                    self.workspace_chrome_split_active_pane();
                }
                workspace_chrome::WorkspaceChromeAction::Settings => {
                    self.enter_workspace_settings_menu();
                }
            }
            return true;
        }
        if let Some(pane) = self.workspace_chrome_pane_close_at_mouse() {
            self.mark_ui_left_press_consumed(state, button);
            self.request_close_confirmation_or_close(WindowCloseTarget::Pane(pane));
            return true;
        }
        if let Some(pane) = self.workspace_chrome_pane_menu_at_mouse() {
            self.mark_ui_left_press_consumed(state, button);
            self.enter_workspace_pane_menu(pane);
            return true;
        }
        false
    }

    fn workspace_chrome_split_active_pane(&mut self) {
        let direction = if self.app_shell.active_tab().panes().len() <= 1 {
            SplitDirection::Right
        } else {
            SplitDirection::Down
        };
        if let Err(error) = self.dispatch_app_action(AppAction::SplitPane {
            pane: self.app_shell.active_pane_id(),
            direction,
            launch: None,
        }) {
            eprintln!("workspace split action failed: {error:?}");
        }
    }

    fn refresh_workspace_command_palette(&mut self) {
        self.frame_needs_full_repaint = true;
        self.apply_window_title();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn enter_workspace_settings_menu(&mut self) {
        self.enter_command_palette_mode();
        if let Some(palette) = self.command_palette.as_mut() {
            palette.context_title = Some("Settings".to_owned());
            palette.context_entries = Some(
                [
                    WindowCommand::IncreaseFontSize,
                    WindowCommand::DecreaseFontSize,
                    WindowCommand::ResetFontSize,
                    WindowCommand::ResetFontAndWindowSize,
                    WindowCommand::ReloadConfiguration,
                    WindowCommand::ToggleFullScreen,
                ]
                .into_iter()
                .map(WindowCommandPaletteEntry::BuiltIn)
                .collect(),
            );
        }
        self.refresh_workspace_command_palette();
    }

    fn enter_workspace_split_menu(&mut self) {
        self.enter_command_palette_mode();
        if let Some(palette) = self.command_palette.as_mut() {
            palette.context_title = Some("Split Pane".to_owned());
            palette.context_entries = Some(
                [WindowCommand::SplitRight, WindowCommand::SplitDown]
                    .into_iter()
                    .map(WindowCommandPaletteEntry::BuiltIn)
                    .collect(),
            );
        }
        self.refresh_workspace_command_palette();
    }

    fn enter_workspace_pane_menu(&mut self, pane: rterm_core::PaneId) {
        self.enter_command_palette_mode_for_pane(pane);
        if let Some(palette) = self.command_palette.as_mut() {
            palette.context_title = Some("Pane Actions".to_owned());
            palette.context_entries = Some(
                [
                    WindowCommand::InspectPane,
                    WindowCommand::SplitRight,
                    WindowCommand::SplitDown,
                    WindowCommand::ClosePane,
                ]
                .into_iter()
                .map(WindowCommandPaletteEntry::BuiltIn)
                .collect(),
            );
        }
        self.refresh_workspace_command_palette();
    }

    fn workspace_chrome_action_cells(&self) -> Vec<RenderCell> {
        if !self.workspace_chrome_enabled() || self.tab_bar_at_bottom {
            return Vec::new();
        }
        let hover = self.workspace_chrome_action_at_mouse();
        workspace_chrome::action_cells(
            self.tab_bar_frame_row(),
            self.window_terminal_size_for_layout().columns,
            self.workspace_chrome_right_reserved_columns(),
            hover,
        )
    }

    fn workspace_chrome_pane_header_cells(&self, layout: &PaneRenderLayout) -> Vec<RenderCell> {
        if !self.workspace_chrome_enabled() || self.tab_bar_at_bottom {
            return Vec::new();
        }
        let active_pane = self.app_shell.active_pane_id();
        let tab = self.app_shell.active_tab();
        layout
            .panes
            .iter()
            .filter_map(|rect| {
                let pane = tab.panes().iter().find(|pane| pane.id() == rect.pane_id)?;
                let shell = pane_launch_display_program(pane.launch());
                let title = self
                    .pane_title(rect.pane_id)
                    .unwrap_or_else(|| shell.to_owned());
                let cwd = self
                    .pane_launch_current_working_dir(rect.pane_id)
                    .unwrap_or("");
                let progress = pane_progress_label(pane.progress());
                let spec = workspace_chrome::PaneHeaderSpec {
                    row: rect.row.saturating_sub(1),
                    column: rect.column,
                    columns: rect.columns,
                    title: &title,
                    shell,
                    cwd,
                    progress: &progress,
                    active: rect.pane_id == active_pane,
                    attention: pane.has_unseen_output(),
                };
                Some(workspace_chrome::pane_header_cells(&spec))
            })
            .flatten()
            .collect()
    }

    fn workspace_chrome_status_cells(&self) -> Vec<RenderCell> {
        if !self.workspace_chrome_enabled() || self.tab_bar_at_bottom {
            return self.search_overlay_cells();
        }
        let size = self.window_terminal_size_for_layout();
        let active_pane_id = self.app_shell.active_pane_id();
        let (terminal_columns, terminal_rows) = self
            .pane_render_layout()
            .panes
            .into_iter()
            .find(|rect| rect.pane_id == active_pane_id)
            .map_or((size.columns, size.rows), |rect| (rect.columns, rect.rows));
        let pane = self.app_shell.active_pane();
        let renderer = match self.renderer_mode {
            RendererMode::Cpu => "CPU",
            RendererMode::Gpu => "GPU",
            RendererMode::Auto => {
                if self.presentation_owner == PresentationOwner::GpuActive {
                    "GPU"
                } else {
                    "CPU"
                }
            }
        };
        let mut cells = workspace_chrome::status_cells(
            self.terminal_frame_row_offset().saturating_add(size.rows),
            size.columns,
            pane_launch_domain_name(pane.launch()),
            pane_launch_display_program(pane.launch()),
            terminal_columns,
            terminal_rows,
            renderer,
        );
        cells.extend(self.search_overlay_cells());
        cells
    }
}

impl NativeWindowApp {
    fn handle_cursor_left(&mut self) {
        self.mouse_pixel_position = None;
        self.mouse_position = None;
        self.set_mouse_cursor_visible(true);
        self.set_mouse_cursor_icon(CursorIcon::Default);
    }

    fn set_mouse_cursor_visible(&mut self, visible: bool) {
        if self.mouse_cursor_visible == visible {
            return;
        }

        self.mouse_cursor_visible = visible;
        if let Some(window) = &self.window {
            window.set_cursor_visible(visible);
        }
    }

    fn set_mouse_cursor_icon(&mut self, icon: CursorIcon) {
        if self.mouse_cursor_icon == icon {
            return;
        }

        self.mouse_cursor_icon = icon;
        if let Some(window) = &self.window {
            window.set_cursor(icon);
        }
    }

    fn update_split_resize_cursor_icon(&mut self) {
        let drag = self
            .split_resize_dragging
            .or_else(|| self.split_resize_drag_at_mouse_position());
        let icon = drag.map_or(CursorIcon::Default, |drag| {
            split_resize_cursor_icon(drag.direction)
        });
        self.set_mouse_cursor_icon(icon);
    }

    fn hide_mouse_cursor_for_typing_if_needed(&mut self) {
        if self.hide_mouse_cursor_when_typing && self.mouse_pixel_position.is_some() {
            self.set_mouse_cursor_visible(false);
        }
    }

    fn handle_split_resize_mouse_input(
        &mut self,
        state: ElementState,
        button: MouseButton,
    ) -> bool {
        if button != MouseButton::Left {
            return false;
        }

        match state {
            ElementState::Pressed => {
                let Some(drag) = self.split_resize_drag_at_mouse_position() else {
                    return false;
                };

                self.clear_ordinary_selection();
                self.selecting = false;
                self.split_resize_dragging = Some(drag);
                self.update_split_resize_cursor_icon();
                true
            }
            ElementState::Released if self.split_resize_dragging.is_some() => {
                self.split_resize_dragging = None;
                self.update_split_resize_cursor_icon();
                true
            }
            ElementState::Released => false,
        }
    }

    fn resize_split_to_mouse_position(&mut self) -> bool {
        let Some(drag) = self.split_resize_dragging else {
            return false;
        };
        let Some((column, row)) = self.mouse_position else {
            return false;
        };

        let (delta, direction) = match drag.direction {
            SplitDirection::Left | SplitDirection::Right => {
                let delta = i32::from(column) - i32::from(drag.last_column);
                let direction = if delta > 0 {
                    ResizeDirection::Right
                } else {
                    ResizeDirection::Left
                };
                (delta, direction)
            }
            SplitDirection::Up | SplitDirection::Down => {
                let delta = i32::from(row) - i32::from(drag.last_row);
                let direction = if delta > 0 {
                    ResizeDirection::Down
                } else {
                    ResizeDirection::Up
                };
                (delta, direction)
            }
        };

        let amount = delta.unsigned_abs();
        if amount == 0 {
            return false;
        }
        let amount = u16::try_from(amount).unwrap_or(u16::MAX);
        if let Err(error) = self.dispatch_app_action(AppAction::ResizePane {
            pane: drag.pane_id,
            direction,
            amount,
        }) {
            eprintln!("split resize drag failed: {error:?}");
            return false;
        }

        if let Some(drag) = self.split_resize_dragging.as_mut() {
            drag.last_column = column;
            drag.last_row = row;
        }
        true
    }

    fn split_resize_drag_at_mouse_position(&self) -> Option<PaneSplitResizeDrag> {
        let (column, row) = self.mouse_position?;
        let render_row = row.checked_add(self.terminal_frame_row_offset())?;
        self.pane_render_layout()
            .separators
            .into_iter()
            .find_map(|separator| split_resize_drag(separator, render_row, column))
            .map(|mut drag| {
                // split_resize_drag receives render-space rows for hit testing,
                // while subsequent motion is reported in raw terminal space.
                // Keep the drag baseline in that same raw space so the frame
                // chrome offset is not charged to the first movement.
                drag.last_row = row;
                drag
            })
    }

    fn handle_scrollbar_mouse_input(&mut self, state: ElementState, button: MouseButton) -> bool {
        if button != MouseButton::Left {
            return false;
        }

        match state {
            ElementState::Pressed => {
                let Some(position) = self.mouse_pixel_position else {
                    return false;
                };
                if !self.scrollbar_hit_test(position) {
                    return false;
                }

                self.scrollbar_dragging = true;
                self.scroll_to_scrollbar_position(position)
            }
            ElementState::Released if self.scrollbar_dragging => {
                self.scrollbar_dragging = false;
                true
            }
            ElementState::Released => false,
        }
    }

    fn scrollbar_hit_test(&self, position: PhysicalPosition<f64>) -> bool {
        let placement = self.frame_content_placement();
        if self.scrollback_scrollbar().is_none()
            || placement.width < SCROLLBAR_WIDTH
            || self.frame_height == 0
            || !position.x.is_finite()
            || !position.y.is_finite()
            || position.x < 0.0
            || position.y < f64::from(self.terminal_pixel_top())
            || position.y >= f64::from(self.terminal_pixel_bottom())
            || position.y >= f64::from(self.frame_height)
        {
            return false;
        }

        let track_left = f64::from(
            placement
                .x
                .saturating_add(placement.width.saturating_sub(SCROLLBAR_WIDTH)),
        );
        let track_right = f64::from(placement.x.saturating_add(placement.width));
        position.x >= track_left && position.x < track_right
    }

    fn scroll_to_scrollbar_position(&mut self, position: PhysicalPosition<f64>) -> bool {
        let Some(offset) = self.scrollbar_offset_from_pixel_y(position.y) else {
            return false;
        };

        self.selecting = false;
        self.last_left_click = None;

        let old_offset = self.current_scrollback_offset();
        let had_overlay = self.selection.is_some()
            || self.active_ui.retained_search().is_some()
            || self.active_ui.quick_select().is_some()
            || self.pane_select.is_some()
            || self.prompt_input_line.is_some()
            || self.input_selector.is_some()
            || self.active_ui.copy_mode().is_some();
        self.interaction_state
            .active_ui
            .stable_viewport
            .set_scrollback_offset(self.runtime.terminal(), offset);
        self.update_selection_projection();
        self.pane_select = None;
        self.prompt_input_line = None;
        self.input_selector = None;
        self.confirmation = None;

        if self.current_scrollback_offset() != old_offset || had_overlay {
            self.refresh_snapshot();
            self.apply_window_title();
        }

        true
    }

    fn scrollbar_offset_from_pixel_y(&self, y: f64) -> Option<usize> {
        if !y.is_finite() || self.frame_height == 0 {
            return None;
        }

        let y = y - f64::from(self.terminal_pixel_top());
        if y < 0.0 {
            return None;
        }
        let content_height = self.terminal_content_pixel_height();
        let y = y.clamp(0.0, f64::from(content_height.saturating_sub(1)));
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let y = y.floor() as u32;
        let geometry = RenderGeometry::new(
            self.frame_width,
            content_height,
            self.cell_width(),
            self.cell_height(),
        );
        Some(self.scrollback_scrollbar()?.offset_from_pixel_y_with_dpi(
            y,
            geometry,
            self.window_dpi,
        ))
    }

    fn render_geometry(&self) -> RenderGeometry {
        RenderGeometry::new(
            self.frame_width,
            self.frame_height,
            self.cell_width(),
            self.cell_height(),
        )
    }

    fn frame_render_geometry(
        &self,
        geometry: RenderGeometry,
        placement: NativeFrameContentPlacement,
    ) -> RenderGeometry {
        let geometry =
            geometry.with_content_rect(placement.x, placement.y, placement.width, placement.height);
        let has_outer_margin = placement.x > 0
            && placement.y > 0
            && placement.x.saturating_add(placement.width) < geometry.target_width
            && placement.y.saturating_add(placement.height) < geometry.target_height;
        let modern_chrome = self.modern_tab_bar_brand && has_outer_margin;
        let geometry = if modern_chrome {
            geometry.with_frame_border(DEFAULT_WINDOW_CHROME_BORDER_RGBA)
        } else {
            geometry
        };
        if modern_chrome && self.tab_bar_is_visible() && !self.tab_bar_at_bottom {
            geometry.with_frame_separator(
                placement
                    .y
                    .saturating_add(self.cell_height())
                    .saturating_sub(1),
                DEFAULT_TAB_BAR_SEPARATOR_RGBA,
            )
        } else {
            geometry
        }
    }
}

impl NativeWindowApp {
    fn frame_content_placement(&self) -> NativeFrameContentPlacement {
        let terminal = self.window_terminal_size_for_layout();
        let terminal_width = u32::from(terminal.columns).saturating_mul(self.cell_width());
        let terminal_height = u32::from(terminal.rows).saturating_mul(self.cell_height());
        let padding = window_padding_pixels_for_terminal_size(
            self.window_padding,
            terminal_width,
            terminal_height,
            self.cell_width(),
            self.cell_height(),
            self.window_dpi,
        );
        let available_width = self.frame_width.saturating_sub(padding.horizontal());
        let available_height = self.frame_height.saturating_sub(padding.vertical());
        if !self.window_content_alignment_is_configured() {
            return NativeFrameContentPlacement {
                x: padding.left.min(self.frame_width),
                y: padding.top.min(self.frame_height),
                width: available_width,
                height: available_height,
            };
        }

        let content_width = terminal_width.min(available_width);
        let content_height = u32::from(
            terminal
                .rows
                .saturating_add(self.tab_bar_rows())
                .saturating_add(self.workspace_chrome_frame_rows()),
        )
        .saturating_mul(self.cell_height())
        .min(available_height);
        let horizontal_gap = available_width.saturating_sub(content_width);
        let vertical_gap = available_height.saturating_sub(content_height);
        NativeFrameContentPlacement {
            x: padding.left.saturating_add(
                self.window_content_alignment
                    .horizontal
                    .offset(horizontal_gap),
            ),
            y: padding
                .top
                .saturating_add(self.window_content_alignment.vertical.offset(vertical_gap)),
            width: content_width,
            height: content_height,
        }
    }

    fn window_content_alignment_is_configured(&self) -> bool {
        self.config_overrides.window_content_alignment.is_some()
    }

    fn frame_content_pixel_left(&self) -> u32 {
        self.frame_content_placement().x
    }

    fn frame_content_pixel_top(&self) -> u32 {
        self.frame_content_placement().y
    }

    fn terminal_content_pixel_height(&self) -> u32 {
        if !self.window_content_alignment_is_configured() {
            return self
                .frame_content_placement()
                .height
                .saturating_sub(self.tab_bar_pixel_height())
                .saturating_sub(
                    u32::from(self.workspace_chrome_frame_rows()) * self.cell_height(),
                );
        }

        u32::from(self.window_terminal_size_for_layout().rows).saturating_mul(self.cell_height())
    }

    fn terminal_frame_row_offset(&self) -> u16 {
        let tab_bar_rows = if self.tab_bar_is_visible() && !self.tab_bar_at_bottom {
            TAB_BAR_ROWS
        } else {
            0
        };
        let chrome_header_rows = u16::from(self.workspace_chrome_enabled());
        tab_bar_rows.saturating_add(chrome_header_rows)
    }

    fn tab_bar_rows(&self) -> u16 {
        if self.tab_bar_is_visible() {
            TAB_BAR_ROWS
        } else {
            0
        }
    }

    fn tab_bar_pixel_height(&self) -> u32 {
        u32::from(self.tab_bar_rows()) * self.cell_height()
    }

    fn tab_bar_frame_row(&self) -> u16 {
        if self.tab_bar_is_visible() && self.tab_bar_at_bottom {
            self.window_terminal_size_for_layout().rows
        } else {
            0
        }
    }

    fn tab_bar_pixel_top(&self) -> u32 {
        self.frame_content_pixel_top()
            .saturating_add(u32::from(self.tab_bar_frame_row()) * self.cell_height())
    }

    fn terminal_pixel_top(&self) -> u32 {
        self.frame_content_pixel_top()
            .saturating_add(u32::from(self.terminal_frame_row_offset()) * self.cell_height())
    }

    fn terminal_pixel_bottom(&self) -> u32 {
        self.terminal_pixel_top()
            .saturating_add(self.terminal_content_pixel_height())
    }

    fn tab_bar_is_visible(&self) -> bool {
        self.enable_tab_bar
            && !(self.hide_tab_bar_if_only_one_tab
                && self.app_shell.active_workspace().tabs().len() == 1)
    }

    fn window_mouse_cell(&self, position: PhysicalPosition<f64>) -> Option<(u16, u16)> {
        let x = position.x - f64::from(self.frame_content_pixel_left());
        if x < 0.0 {
            return None;
        }
        let terminal_width = u32::from(self.window_terminal_size_for_layout().columns)
            .saturating_mul(self.cell_width());
        if x >= f64::from(terminal_width) {
            return None;
        }
        let terminal_y = position.y - f64::from(self.terminal_pixel_top());
        if terminal_y < 0.0 {
            return None;
        }
        let terminal_height = u32::from(self.window_terminal_size_for_layout().rows)
            .saturating_mul(self.cell_height());
        if terminal_y >= f64::from(terminal_height) {
            return None;
        }

        Some((
            pixel_axis_to_cell(x, self.cell_width())?,
            pixel_axis_to_cell(terminal_y, self.cell_height())?,
        ))
    }

    fn suppress_terminal_cursor_for_overlay(
        &self,
        mut snapshot: TerminalRenderSnapshot,
    ) -> TerminalRenderSnapshot {
        // The GPU cursor is a separate graph primitive.  Its opacity state is
        // intentionally blink-aware, so a non-blinking cursor can still leak
        // through a workspace overlay even when the renderer opacity is zero.
        // Remove only the presentation cursor; the terminal runtime keeps its
        // cursor position and visibility unchanged for restoration after Esc.
        let search_editor_active = self.active_ui.search().is_some_and(|search| search.editing);
        if self.higher_level_ui_suppresses_pane_overlay() || search_editor_active {
            snapshot.cursor = None;
        }
        snapshot
    }

    #[expect(
        clippy::too_many_lines,
        reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
    )]
    fn render_snapshot(&self) -> TerminalRenderSnapshot {
        let layout = self.pane_render_layout();
        let palette = self.workspace_chrome_palette();
        let suppress_pane_overlay = self.higher_level_ui_suppresses_pane_overlay();
        let pane_close_button_cells = self.pane_close_button_cells(&layout);
        if layout.panes.len() <= 1 {
            let rect = layout.panes.first().copied().unwrap_or_else(|| {
                let size = self.window_terminal_size_for_layout();
                PaneRenderRect {
                    pane_id: self.app_shell.active_pane_id(),
                    row: self.terminal_frame_row_offset(),
                    column: 0,
                    rows: size.rows,
                    columns: size.columns,
                }
            });
            let pane_base = self.snapshot.clone();
            let snapshot = pane_presentation_snapshot(
                pane_base,
                self.runtime.terminal(),
                &self.active_ui,
                rect,
                &palette,
                &self.selection_word_boundary,
                suppress_pane_overlay,
                self.quick_select_remove_styling,
                self.foreground_text_hsb,
                self.text_background_opacity,
                self.window_background_opacity,
                None,
                self.text_min_contrast_ratio,
                self.bold_brightens_ansi_colors,
            );
            let snapshot =
                self.apply_compose_cursor_to_snapshot(self.app_shell.active_pane_id(), snapshot);
            let snapshot = self.apply_visual_bell_to_snapshot(
                self.app_shell.active_pane_id(),
                snapshot,
                rect.rows,
                rect.columns,
            );
            let snapshot = self.hyperlink_rules_snapshot(snapshot);
            let viewport_rows = workspace_chrome::viewport_rows_for_chrome(
                rect.rows,
                self.workspace_chrome_enabled(),
            );
            return self.suppress_terminal_cursor_for_overlay(
                snapshot
                    .with_viewport(rect.row, rect.column, viewport_rows, rect.columns)
                    .with_overlay_cells(self.workspace_chrome_pane_header_cells(&layout))
                    .with_overlay_cells(pane_close_button_cells)
                    .with_overlay_cells(self.pane_badge_cells(&layout))
                    .with_overlay_cells(self.ssh_connection_overlay_cells(&layout))
                    .with_overlay_cells(self.pane_inspection_cells(&layout))
                    .with_overlay_cells(self.pane_select_cells(&layout))
                    .with_overlay_cells(self.ime_preedit_cells(&layout))
                    .with_overlay_cells(self.window_frame_border_cells())
                    .with_overlay_cells(self.tab_bar_cells())
                    .with_overlay_cells(self.workspace_chrome_action_cells())
                    .with_overlay_cells(self.workspace_chrome_status_cells())
                    .with_overlay_cells(self.tab_navigator_cells())
                    .with_overlay_cells(self.command_palette_cells())
                    .with_overlay_cells(self.input_selector_cells())
                    .with_overlay_cells(self.char_select_cells())
                    .with_overlay_cells(self.debug_overlay_cells()),
            );
        }

        let active_pane = self.app_shell.active_pane_id();
        let mut pane_rects = layout.panes.clone();
        pane_rects.sort_by_key(|rect| rect.pane_id != active_pane);

        let mut snapshot: Option<TerminalRenderSnapshot> = None;
        for rect in pane_rects {
            let (base, terminal, ui) = if rect.pane_id == active_pane {
                (&self.snapshot, self.runtime.terminal(), &self.active_ui)
            } else {
                let Some(runtime) = self.pane_runtimes.get(&rect.pane_id) else {
                    continue;
                };
                (&runtime.snapshot, runtime.runtime.terminal(), &runtime.ui)
            };
            let pane_base = base.clone();
            let mut pane_snapshot = pane_presentation_snapshot(
                pane_base,
                terminal,
                ui,
                rect,
                &palette,
                &self.selection_word_boundary,
                suppress_pane_overlay && rect.pane_id == active_pane,
                self.quick_select_remove_styling,
                self.foreground_text_hsb,
                self.text_background_opacity,
                self.window_background_opacity,
                (rect.pane_id != active_pane).then_some(self.inactive_pane_hsb),
                self.text_min_contrast_ratio,
                self.bold_brightens_ansi_colors,
            );
            pane_snapshot = self.apply_compose_cursor_to_snapshot(rect.pane_id, pane_snapshot);
            pane_snapshot = self.apply_visual_bell_to_snapshot(
                rect.pane_id,
                pane_snapshot,
                rect.rows,
                rect.columns,
            );
            pane_snapshot = self.hyperlink_rules_snapshot(pane_snapshot);
            let viewport_rows = workspace_chrome::viewport_rows_for_chrome(
                rect.rows,
                self.workspace_chrome_enabled(),
            );
            pane_snapshot =
                pane_snapshot.with_viewport(rect.row, rect.column, viewport_rows, rect.columns);
            snapshot = Some(match snapshot {
                Some(current) => current.with_overlay_snapshot(pane_snapshot),
                None => pane_snapshot,
            });
        }

        let snapshot = snapshot
            .unwrap_or_else(|| {
                let snapshot =
                    foreground_text_hsb_snapshot(self.snapshot.clone(), self.foreground_text_hsb);
                let snapshot =
                    text_background_opacity_snapshot(snapshot, self.text_background_opacity);
                let snapshot = window_background_opacity_snapshot(
                    snapshot,
                    self.window_background_opacity,
                    self.background_color,
                );
                let snapshot = text_min_contrast_snapshot(
                    snapshot,
                    self.text_min_contrast_ratio,
                    color_to_rgba(palette.foreground, DEFAULT_RENDER_FOREGROUND_RGBA),
                    color_to_rgba(palette.background, DEFAULT_RENDER_BACKGROUND_RGBA),
                    self.bold_brightens_ansi_colors,
                    self.ansi_palette,
                    self.indexed_palette,
                );
                let snapshot = self
                    .apply_compose_cursor_to_snapshot(self.app_shell.active_pane_id(), snapshot);
                self.hyperlink_rules_snapshot(snapshot)
                    .with_row_offset(self.terminal_frame_row_offset())
            })
            .with_overlay_cells(self.pane_separator_cells(&layout))
            .with_overlay_cells(self.workspace_chrome_pane_header_cells(&layout))
            .with_overlay_cells(pane_close_button_cells)
            .with_overlay_cells(self.pane_badge_cells(&layout))
            .with_overlay_cells(self.ssh_connection_overlay_cells(&layout))
            .with_overlay_cells(self.pane_inspection_cells(&layout))
            .with_overlay_cells(self.pane_select_cells(&layout))
            .with_overlay_cells(self.ime_preedit_cells(&layout))
            .with_overlay_cells(self.window_frame_border_cells())
            .with_overlay_cells(self.tab_bar_cells())
            .with_overlay_cells(self.workspace_chrome_action_cells())
            .with_overlay_cells(self.workspace_chrome_status_cells())
            .with_overlay_cells(self.tab_navigator_cells())
            .with_overlay_cells(self.command_palette_cells())
            .with_overlay_cells(self.input_selector_cells())
            .with_overlay_cells(self.char_select_cells())
            .with_overlay_cells(self.debug_overlay_cells());
        self.suppress_terminal_cursor_for_overlay(snapshot)
    }

    fn apply_compose_cursor_to_snapshot(
        &self,
        pane_id: rterm_core::PaneId,
        snapshot: TerminalRenderSnapshot,
    ) -> TerminalRenderSnapshot {
        if pane_id != self.app_shell.active_pane_id() {
            return snapshot;
        }

        let builtin_preedit_active = self.use_ime
            && self.ime_preedit_rendering == NativeImePreeditRendering::Builtin
            && self
                .ime_preedit
                .as_deref()
                .is_some_and(|preedit| !preedit.is_empty());
        let leader_active = self.leader_active_since.is_some();
        let dead_key_active = self.dead_key_active;

        if !builtin_preedit_active && !leader_active && !dead_key_active {
            return snapshot;
        }

        match self.compose_cursor_color {
            Some(color) => snapshot.with_cursor_color(Some(color)),
            None => snapshot,
        }
    }

    fn apply_visual_bell_to_snapshot(
        &self,
        pane_id: rterm_core::PaneId,
        snapshot: TerminalRenderSnapshot,
        rows: u16,
        columns: u16,
    ) -> TerminalRenderSnapshot {
        let Some(intensity) = self.visual_bell_intensity_for_pane(pane_id) else {
            return snapshot;
        };

        match self.visual_bell.target {
            NativeVisualBellTarget::BackgroundColor => {
                let color = visual_bell_color_from_snapshot(
                    &snapshot,
                    self.visual_bell_color,
                    self.foreground_color,
                );
                let background_rgba =
                    color_to_rgba(self.background_color, DEFAULT_RENDER_BACKGROUND_RGBA);
                let background_cells = visual_bell_background_cells(
                    &snapshot,
                    rows,
                    columns,
                    color,
                    intensity,
                    background_rgba,
                );
                snapshot
                    .with_cells_mapped(|mut cell| {
                        let color = self.visual_bell_color.unwrap_or(match cell.foreground {
                            Color::Default => self.foreground_color,
                            color => color,
                        });
                        cell.background = blend_visual_bell_color(
                            cell.background,
                            color,
                            background_rgba,
                            intensity,
                        );
                        cell
                    })
                    .with_overlay_cells(background_cells)
            }
            NativeVisualBellTarget::CursorColor => {
                let color = visual_bell_color_from_snapshot(
                    &snapshot,
                    self.visual_bell_color,
                    self.foreground_color,
                );
                let base_color =
                    visual_bell_cursor_base_color(&snapshot, self.force_reverse_video_cursor);
                let cursor_color = blend_visual_bell_color(
                    base_color,
                    color,
                    DEFAULT_RENDER_FOREGROUND_RGBA,
                    intensity,
                );
                snapshot.with_cursor_color(Some(cursor_color))
            }
        }
    }

    fn visual_bell_intensity_for_pane(&self, pane_id: rterm_core::PaneId) -> Option<f64> {
        if !self.visual_bell.is_enabled() {
            return None;
        }

        let started_at = self.visual_bell_started_at.get(&pane_id)?;
        #[cfg(test)]
        let elapsed = self
            .visual_bell_test_now
            .unwrap_or_else(Instant::now)
            .saturating_duration_since(*started_at);
        #[cfg(not(test))]
        let elapsed = started_at.elapsed();
        visual_bell_intensity(self.visual_bell, elapsed)
    }

    fn has_visible_split_layout(&self) -> bool {
        self.app_shell.active_tab().panes().len() > 1
    }

    fn focus_pane_for_mouse_position(&mut self) -> Option<PaneMouseCell> {
        let mouse_cell = self.pane_cell_at_mouse_position()?;
        if mouse_cell.pane_id != self.app_shell.active_pane_id()
            && let Err(error) = self.dispatch_app_action(AppAction::ActivatePane {
                pane: mouse_cell.pane_id,
            })
        {
            eprintln!("app shell pane focus error: {error:?}");
            return None;
        }

        Some(mouse_cell)
    }

    fn mouse_cell_for_active_pane(&self) -> Option<PaneMouseCell> {
        let mouse_cell = self.pane_cell_at_mouse_position()?;
        (mouse_cell.pane_id == self.app_shell.active_pane_id()).then_some(mouse_cell)
    }

    fn pane_cell_at_mouse_position(&self) -> Option<PaneMouseCell> {
        let (column, row) = self.mouse_position?;
        let render_row = row.checked_add(self.terminal_frame_row_offset())?;
        self.pane_render_layout()
            .panes
            .into_iter()
            .find_map(|rect| pane_mouse_cell(rect, render_row, column))
    }

    fn pane_close_button_at_mouse_position(&self) -> Option<rterm_core::PaneId> {
        let (column, row) = self.mouse_position?;
        let render_row = row.checked_add(self.terminal_frame_row_offset())?;
        let layout = self.pane_render_layout();
        self.pane_close_button_targets(&layout)
            .into_iter()
            .find_map(|(pane, button_row, button_column)| {
                (button_row == render_row && button_column == column).then_some(pane)
            })
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn wheel_hit_target_at_mouse_position(&self) -> Option<WheelHitTarget> {
        let position = self.mouse_pixel_position?;
        if self.scrollbar_hit_test(position) {
            return Some(WheelHitTarget::ActiveScrollbar {
                pane_id: self.app_shell.active_pane_id(),
            });
        }

        self.pane_render_layout()
            .panes
            .into_iter()
            .find_map(|rect| self.wheel_target_for_rect(rect, position))
            .map(WheelHitTarget::PaneSurface)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn wheel_target_for_rect(
        &self,
        rect: PaneRenderRect,
        position: PhysicalPosition<f64>,
    ) -> Option<WheelTarget> {
        let pane_terminal_row = rect.row.checked_sub(self.terminal_frame_row_offset())?;
        let x = position.x
            - f64::from(self.frame_content_pixel_left())
            - f64::from(rect.column) * f64::from(self.cell_width());
        let y = position.y
            - f64::from(self.terminal_pixel_top())
            - f64::from(pane_terminal_row) * f64::from(self.cell_height());
        let width = f64::from(u32::from(rect.columns).saturating_mul(self.cell_width()));
        let height = f64::from(u32::from(rect.rows).saturating_mul(self.cell_height()));
        if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 || x >= width || y >= height {
            return None;
        }

        Some(WheelTarget {
            pane_id: rect.pane_id,
            rect,
            cell: PaneMouseCell {
                pane_id: rect.pane_id,
                row: pixel_axis_to_cell(y, self.cell_height())?,
                column: pixel_axis_to_cell(x, self.cell_width())?,
            },
            pixel_position: PhysicalPosition::new(x, y),
        })
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn pane_render_rect(&self, pane_id: rterm_core::PaneId) -> Option<PaneRenderRect> {
        self.pane_render_layout()
            .panes
            .into_iter()
            .find(|rect| rect.pane_id == pane_id)
    }

    fn pane_snapshot(&self, pane_id: rterm_core::PaneId) -> Option<&TerminalRenderSnapshot> {
        if pane_id == self.app_shell.active_pane_id() {
            return Some(&self.snapshot);
        }

        self.pane_runtimes
            .get(&pane_id)
            .map(|runtime| &runtime.snapshot)
    }

    fn pane_mouse_reporting_mode(&self, pane_id: rterm_core::PaneId) -> MouseReportingMode {
        if pane_id == self.app_shell.active_pane_id() {
            return self.runtime.mouse_input_mode().reporting();
        }

        self.pane_runtimes
            .get(&pane_id)
            .map(|runtime| runtime.runtime.mouse_input_mode().reporting())
            .unwrap_or_default()
    }

    fn pane_iterm_mouse_reporting_mode(&self, pane_id: rterm_core::PaneId) -> i16 {
        iterm_mouse_reporting_mode_value(self.pane_mouse_reporting_mode(pane_id))
    }

    fn pane_application_keypad(&self, pane_id: rterm_core::PaneId) -> bool {
        if pane_id == self.app_shell.active_pane_id() {
            return self.runtime.application_keypad();
        }

        self.pane_runtimes
            .get(&pane_id)
            .is_some_and(|runtime| runtime.runtime.application_keypad())
    }

    fn record_pane_bells(&mut self, pane_id: rterm_core::PaneId, bells: u64) {
        if bells == 0 {
            return;
        }

        let count = self.pane_bell_counts.entry(pane_id).or_default();
        *count = count.saturating_add(bells);
    }

    fn pane_bell_count(&self, pane_id: rterm_core::PaneId) -> u64 {
        self.pane_bell_counts.get(&pane_id).copied().unwrap_or(0)
    }

    fn pane_mouse_info(&self, pane_id: rterm_core::PaneId) -> Option<ItermMouseInfo> {
        self.last_mouse_info.filter(|info| info.pane_id == pane_id)
    }

    fn pane_render_layout(&self) -> PaneRenderLayout {
        self.pane_render_layout_for_tab(self.app_shell.active_tab())
    }

    fn padded_terminal_render_rect_for_size(
        &self,
        pane_id: rterm_core::PaneId,
        size: rterm_core::TerminalSize,
    ) -> PaneRenderRect {
        // Padding is a physical frame margin, not terminal grid space. Keep
        // the PTY dimensions intact so a visual inset never hides rows/cols.
        PaneRenderRect {
            pane_id,
            row: self.terminal_frame_row_offset(),
            column: 0,
            rows: size.rows,
            columns: size.columns,
        }
    }

    fn pane_render_layout_for_tab(&self, tab: &rterm_core::app_shell::Tab) -> PaneRenderLayout {
        let size = self.window_terminal_size_for_layout();
        self.pane_render_layout_for_tab_at_size(tab, size)
    }

    fn pane_render_layout_for_tab_at_size(
        &self,
        tab: &rterm_core::app_shell::Tab,
        size: rterm_core::TerminalSize,
    ) -> PaneRenderLayout {
        let panes = tab
            .panes()
            .iter()
            .map(|pane| match pane.split() {
                Some(split) => rterm_native::PaneLayoutPane::split(
                    pane.id(),
                    rterm_native::PaneSplitSpec::new(
                        split.source_pane,
                        native_pane_split_direction(split.direction),
                        split.source_size_delta,
                    ),
                ),
                None => rterm_native::PaneLayoutPane::root(pane.id()),
            })
            .collect();
        let native = rterm_native::build_pane_layout(&rterm_native::PaneLayoutSpec::new(
            size,
            self.terminal_frame_row_offset(),
            panes,
            tab.zoomed_pane_id(),
        ));
        PaneRenderLayout {
            panes: native
                .panes
                .into_iter()
                .map(|pane| PaneRenderRect {
                    pane_id: pane.pane,
                    row: pane.rect.row,
                    column: pane.rect.column,
                    rows: pane.rect.rows,
                    columns: pane.rect.columns,
                })
                .collect(),
            separators: native
                .separators
                .into_iter()
                .map(|separator| PaneSeparator {
                    row: separator.rect.row,
                    column: separator.rect.column,
                    rows: separator.rect.rows,
                    columns: separator.rect.columns,
                    direction: app_pane_split_direction(separator.direction),
                    source_pane: separator.source_pane,
                    new_pane: separator.new_pane,
                })
                .collect(),
        }
    }

    fn split_pane_source_size_delta_for_active_pane(
        &self,
        direction: SplitDirection,
        size: WindowSplitPaneSize,
    ) -> i16 {
        let active_pane = self.app_shell.active_pane_id();
        let Some(rect) = self
            .pane_render_layout()
            .panes
            .into_iter()
            .find(|rect| rect.pane_id == active_pane)
        else {
            return 0;
        };
        let total_cells = match direction {
            SplitDirection::Left | SplitDirection::Right => rect.columns,
            SplitDirection::Up | SplitDirection::Down => rect.rows,
        };
        split_pane_source_size_delta(total_cells, size)
    }

    fn pane_close_button_cells(&self, layout: &PaneRenderLayout) -> Vec<RenderCell> {
        if self.workspace_chrome_enabled() {
            return Vec::new();
        }
        self.pane_close_button_targets(layout)
            .into_iter()
            .map(|(_, row, column)| {
                ui_render_cell(
                    row,
                    column,
                    PANE_CLOSE_BUTTON_GLYPH,
                    PANE_CLOSE_BUTTON_FOREGROUND,
                    PANE_CLOSE_BUTTON_BACKGROUND,
                    true,
                )
            })
            .collect()
    }

    fn request_pane_inspection(&mut self, pane_id: rterm_core::PaneId) {
        self.request_pane_inspection_from(pane_id, PaneInspectionRequestSource::Direct);
    }

    fn request_pane_inspection_from(
        &mut self,
        pane_id: rterm_core::PaneId,
        source: PaneInspectionRequestSource,
    ) -> bool {
        let layout = self.pane_render_layout();
        if self.pane_inspection == Some(pane_id) {
            return true;
        }
        if self.debug_overlay_active
            || self.char_select.is_some()
            || self.pane_select.is_some()
            || self.tab_navigator.is_some()
            || self.prompt_input_line.is_some()
            || self.input_selector.is_some()
            || self.confirmation.is_some()
            || self.close_confirmation.is_some()
            || (self.command_palette.is_some()
                && source != PaneInspectionRequestSource::CommandPaletteExecute)
            || layout.panes.iter().any(|rect| {
                self.pane_ui_ref(rect.pane_id)
                    .is_some_and(PaneUiState::overlay_active)
            })
            || !layout.panes.iter().any(|rect| rect.pane_id == pane_id)
            || !self.pane_runtime_owner_exists(pane_id)
        {
            return false;
        }

        let consume_left_release = self.active_mouse_button == Some(MouseButton::Left);
        self.clear_ordinary_selection_for_pane(pane_id);
        self.end_pointer_modes_for_pane_change();
        self.tab_bar_drag = None;
        self.current_mouse_wheel_delta = None;
        self.last_mouse_info = None;
        self.deferred_wheel_context = None;
        self.pressed_pane_close_button = None;
        self.ui_left_release_pending |= consume_left_release;
        self.ime_preedit = None;
        self.dead_key_active = false;
        self.dead_key_text = None;
        self.pane_inspection = Some(pane_id);
        self.ui_key_release_pending = None;
        self.frame_needs_full_repaint = true;
        true
    }

    fn cancel_pane_inspection(&mut self) {
        if self.pane_inspection.take().is_some() {
            self.frame_needs_full_repaint = true;
        }
    }

    fn clear_pane_inspection_if_invalid(&mut self) {
        let Some(pane_id) = self.pane_inspection else {
            return;
        };
        if !self.pane_runtime_owner_exists(pane_id)
            || !self
                .pane_render_layout()
                .panes
                .iter()
                .any(|rect| rect.pane_id == pane_id)
        {
            self.cancel_pane_inspection();
        }
    }

    fn pane_runtime_owner_exists(&self, pane_id: rterm_core::PaneId) -> bool {
        pane_id == self.app_shell.active_pane_id() || self.pane_runtimes.contains_key(&pane_id)
    }

    fn pane_inspection_close_key(key: &Key) -> Option<UiKeyRelease> {
        match key {
            Key::Named(NamedKey::Escape) => Some(UiKeyRelease::Escape),
            Key::Named(NamedKey::Enter) => Some(UiKeyRelease::Enter),
            Key::Character(value) if value.as_str() == "\r" => Some(UiKeyRelease::Enter),
            _ => None,
        }
    }

    fn handle_pane_inspection_key_event(&mut self, key: &Key, state: ElementState) -> bool {
        if let Some(pending) = self.ui_key_release_pending {
            let (pending_key, full_barrier) = match pending {
                UiKeyReleasePending::FullBarrier(key) => (key, true),
                UiKeyReleasePending::MatchingReleaseOnly(key) => (key, false),
            };
            if !full_barrier
                && self.window_focused
                && state == ElementState::Pressed
                && Self::pane_inspection_close_key(key) == Some(pending_key)
            {
                self.ui_key_release_pending = None;
                return false;
            }
            if state == ElementState::Released
                && Self::pane_inspection_close_key(key) == Some(pending_key)
            {
                self.ui_key_release_pending = None;
                return true;
            }
            if !full_barrier && state == ElementState::Released && !self.window_focused {
                return true;
            }
            return full_barrier;
        }
        if self.pane_inspection.is_none() {
            return false;
        }
        if state == ElementState::Pressed
            && let Some(close_key) = Self::pane_inspection_close_key(key)
        {
            self.pane_inspection = None;
            self.ui_key_release_pending = Some(UiKeyReleasePending::FullBarrier(close_key));
            self.frame_needs_full_repaint = true;
        }
        true
    }

    fn pane_inspection_input_barrier_active(&self) -> bool {
        self.pane_inspection.is_some()
            || matches!(
                self.ui_key_release_pending,
                Some(UiKeyReleasePending::FullBarrier(_))
            )
    }

    fn pane_inspection_cells(&self, layout: &PaneRenderLayout) -> Vec<RenderCell> {
        let Some(pane_id) = self.pane_inspection else {
            return Vec::new();
        };
        let Some(rect) = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == pane_id)
            .copied()
        else {
            return Vec::new();
        };
        let Some(lines) = self.pane_inspection_lines(pane_id) else {
            return Vec::new();
        };
        pane_inspection_cells_for_rect(&lines, rect)
    }

    fn pane_close_button_targets(
        &self,
        layout: &PaneRenderLayout,
    ) -> Vec<(rterm_core::PaneId, u16, u16)> {
        if layout.panes.len() <= 1 || self.higher_level_ui_blocks_pane_surface_mouse() {
            return Vec::new();
        }

        let mut occupied = self
            .pane_badge_cells(layout)
            .into_iter()
            .chain(self.ime_preedit_cells(layout))
            .chain(self.window_frame_border_cells())
            .map(|cell| (cell.row, cell.column))
            .collect::<HashSet<_>>();
        let palette = self.native_resolved_palette();
        let active_pane = self.app_shell.active_pane_id();
        for rect in &layout.panes {
            let (terminal, ui) = if rect.pane_id == active_pane {
                (self.runtime.terminal(), &self.active_ui)
            } else {
                let Some(runtime) = self.pane_runtimes.get(&rect.pane_id) else {
                    continue;
                };
                (runtime.runtime.terminal(), &runtime.ui)
            };
            let Some(quick_select) = ui.quick_select() else {
                continue;
            };
            occupied.extend(
                quick_select_cells_for_pane(
                    terminal,
                    ui.stable_viewport,
                    quick_select,
                    *rect,
                    &palette,
                )
                .into_iter()
                .filter_map(|cell| {
                    Some((
                        rect.row.checked_add(cell.row)?,
                        rect.column.checked_add(cell.column)?,
                    ))
                }),
            );
        }

        layout
            .panes
            .iter()
            .filter_map(|rect| {
                let (row, column) = pane_close_button_position(*rect)?;
                (!occupied.contains(&(row, column))).then_some((rect.pane_id, row, column))
            })
            .collect()
    }

    fn pane_separator_cells(&self, layout: &PaneRenderLayout) -> Vec<RenderCell> {
        let active_pane = self.app_shell.active_pane_id();
        let compact_default =
            self.workspace_chrome_enabled() && self.config_overrides.split_color.is_none();
        let mut cells = Vec::new();
        for separator in &layout.separators {
            let active = separator.source_pane == active_pane || separator.new_pane == active_pane;
            let foreground = self.split_color.unwrap_or(if compact_default {
                workspace_chrome::GRAPHITE_SEPARATOR
            } else if active {
                DEFAULT_SPLIT_ACTIVE_COLOR
            } else {
                DEFAULT_SPLIT_INACTIVE_COLOR
            });
            let background = if compact_default {
                workspace_chrome::GRAPHITE_TERMINAL_BACKGROUND
            } else {
                DEFAULT_SPLIT_BACKGROUND_COLOR
            };
            let ch = if separator.columns == 1 {
                if compact_default { '│' } else { '|' }
            } else if compact_default {
                '─'
            } else {
                '-'
            };
            for row in separator.row..separator.row.saturating_add(separator.rows) {
                for column in separator.column..separator.column.saturating_add(separator.columns) {
                    let mut cell = ui_render_cell(
                        row,
                        column,
                        ch,
                        foreground,
                        background,
                        active && !compact_default,
                    );
                    if separator.columns > 1 && !compact_default {
                        cell.underline = true;
                        cell.underline_style = UnderlineStyle::Single;
                        cell.underline_color = foreground;
                    }
                    cells.push(cell);
                }
            }
        }
        cells
    }

    fn ime_preedit_cells(&self, layout: &PaneRenderLayout) -> Vec<RenderCell> {
        if !self.use_ime || self.ime_preedit_rendering != NativeImePreeditRendering::Builtin {
            return Vec::new();
        }
        let Some(preedit) = self.ime_preedit.as_deref().filter(|text| !text.is_empty()) else {
            return Vec::new();
        };
        let active_pane = self.app_shell.active_pane_id();
        let Some(rect) = layout
            .panes
            .iter()
            .find(|rect| rect.pane_id == active_pane)
            .copied()
        else {
            return Vec::new();
        };
        if rect.rows == 0 || rect.columns == 0 {
            return Vec::new();
        }

        let (cursor_row, cursor_column) = self.runtime.terminal().cursor();
        let row = rect
            .row
            .saturating_add(cursor_row.min(rect.rows.saturating_sub(1)));
        let start_column = rect
            .column
            .saturating_add(cursor_column.min(rect.columns.saturating_sub(1)));
        let end_column = rect.column.saturating_add(rect.columns);

        let mut cells = Vec::new();
        let mut column = start_column;
        for grapheme in preedit.graphemes(true) {
            let columns = UnicodeWidthStr::width(grapheme).max(1);
            let columns = u16::try_from(columns).unwrap_or(u16::MAX);
            if column >= end_column || column.saturating_add(columns) > end_column {
                break;
            }
            let mut leader = ui_render_cell(
                row,
                column,
                grapheme.chars().next().unwrap_or(' '),
                Color::Default,
                Color::Default,
                false,
            );
            leader.text = Arc::<str>::from(grapheme);
            leader.columns = u8::try_from(columns).unwrap_or(u8::MAX);
            leader.underline = true;
            leader.underline_style = UnderlineStyle::Single;
            cells.push(leader);
            for continuation_offset in 1..columns {
                let mut continuation = ui_render_cell(
                    row,
                    column.saturating_add(continuation_offset),
                    ' ',
                    Color::Default,
                    Color::Default,
                    false,
                );
                continuation.text = Arc::from("");
                continuation.columns = 0;
                continuation.continuation = true;
                continuation.underline = true;
                continuation.underline_style = UnderlineStyle::Single;
                cells.push(continuation);
            }
            column = column.saturating_add(columns);
        }
        cells
    }

    fn pane_select_cells(&self, layout: &PaneRenderLayout) -> Vec<RenderCell> {
        let Some(pane_select) = self.pane_select.as_ref() else {
            return Vec::new();
        };

        let foreground = self
            .pane_select_fg_color
            .unwrap_or(DEFAULT_PANE_SELECT_FG_COLOR);
        let background = self
            .pane_select_bg_color
            .unwrap_or(DEFAULT_PANE_SELECT_BG_COLOR);
        let mut cells = Vec::new();
        for label in &pane_select.labels {
            let Some(rect) = layout
                .panes
                .iter()
                .find(|rect| rect.pane_id == label.pane_id)
                .copied()
            else {
                continue;
            };
            let display_label = if pane_select.show_pane_ids {
                format!("{}:{}", label.label, label.pane_id.get())
            } else {
                label.label.clone()
            };
            let label_width = u16::try_from(display_label.chars().count()).unwrap_or(u16::MAX);
            if label_width == 0 || rect.rows == 0 || rect.columns == 0 {
                continue;
            }

            let row = rect.row.saturating_add(rect.rows / 2);
            let column = rect
                .column
                .saturating_add((rect.columns / 2).saturating_sub(label_width / 2));
            for (offset, ch) in display_label.chars().enumerate() {
                let offset = u16::try_from(offset).unwrap_or(u16::MAX);
                let column = column.saturating_add(offset);
                if column >= rect.column.saturating_add(rect.columns) {
                    break;
                }
                cells.push(ui_render_cell(
                    row, column, ch, foreground, background, true,
                ));
            }
        }

        cells
    }

    fn pane_badge_cells(&self, layout: &PaneRenderLayout) -> Vec<RenderCell> {
        let mut cells = Vec::new();
        let active_tab = self.app_shell.active_tab();
        let tab_context = self.tab_badge_context(active_tab, layout);
        let host_context = local_badge_host_context();
        for rect in &layout.panes {
            let Some(pane) = active_tab
                .panes()
                .iter()
                .find(|pane| pane.id() == rect.pane_id)
            else {
                continue;
            };
            let Some(badge) = self.pane_badge_text(pane, rect, &tab_context, &host_context) else {
                continue;
            };

            Self::push_pane_badge_cells(&mut cells, rect, &badge);
        }

        cells
    }

    fn pane_badge_text(
        &self,
        pane: &rterm_core::app_shell::Pane,
        rect: &PaneRenderRect,
        tab_context: &TabBadgeContext<'_>,
        host_context: &BadgeHostContext,
    ) -> Option<String> {
        let badge_format = pane
            .badge_format()
            .map(str::trim)
            .filter(|badge| !badge.is_empty())?;
        if rect.rows == 0 || rect.columns == 0 {
            return None;
        }

        let session_name = self.pane_title(pane.id());
        let session_id = pane.id().get();
        let session_termid =
            iterm_session_termid(tab_context.window_id, tab_context.tab_id, session_id);
        let session_process_id = self.pane_process_id(pane.id());
        let session_tty_name = self.pane_tty_name(pane.id());
        let session_job_name = pane_launch_display_program(pane.launch());
        let session_command_line = pane_launch_command_line(pane.launch());
        let session_last_command = self.pane_last_command(pane.id());
        let profile_name = pane_launch_profile_name(pane.launch());
        let terminal_icon_name = self.pane_terminal_icon_title(pane.id());
        let terminal_window_name = self.pane_terminal_window_title(pane.id());
        let session_mouse_reporting_mode = self.pane_iterm_mouse_reporting_mode(pane.id());
        let session_application_keypad = self.pane_application_keypad(pane.id());
        let session_bell_count = self.pane_bell_count(pane.id());
        let session_selection = self.badge_session_selection(pane.id());
        let window_title_override =
            badge_window_title_override(&self.window_title, tab_context.title.as_deref());
        let badge = interpolate_badge_format(
            badge_format,
            &BadgeInterpolationContext {
                user_vars: pane.user_vars(),
                session_id,
                session_termid: &session_termid,
                session_process_id,
                session_tty_name,
                session_name: session_name.as_deref(),
                session_job_name,
                session_command_line: &session_command_line,
                session_last_command: session_last_command.as_deref(),
                session_home_directory: host_context.home_directory.as_deref(),
                profile_name,
                session_username: host_context.username.as_deref(),
                session_hostname: host_context.hostname.as_deref(),
                session_shell: host_context.shell.as_deref(),
                session_uname: &host_context.uname,
                session_path: pane.launch().cwd(),
                terminal_icon_name: terminal_icon_name.as_deref(),
                terminal_window_name: terminal_window_name.as_deref(),
                iterm2_pid: tab_context.iterm2_pid,
                iterm2_localhost_name: host_context.hostname.as_deref(),
                iterm2_effective_theme: ITERM2_EFFECTIVE_THEME,
                window_id: tab_context.window_id,
                window_style: tab_context.window_style,
                window_frame: self.window_frame,
                window_is_hotkey_window: false,
                window_title_override,
                tab_id: tab_context.tab_id,
                tab_current_session_id: tab_context.current_session_id,
                tab_current_session_process_id: tab_context.current_session_process_id,
                tab_current_session_tty_name: tab_context.current_session_tty_name.as_deref(),
                tab_current_session_name: tab_context.current_session_name.as_deref(),
                tab_current_session_job_name: tab_context.current_session_job_name,
                tab_current_session_command_line: tab_context
                    .current_session_command_line
                    .as_deref(),
                tab_current_session_last_command: tab_context
                    .current_session_last_command
                    .as_deref(),
                tab_current_session_home_directory: host_context.home_directory.as_deref(),
                tab_current_session_username: host_context.username.as_deref(),
                tab_current_session_hostname: host_context.hostname.as_deref(),
                tab_current_session_shell: host_context.shell.as_deref(),
                tab_current_session_uname: &host_context.uname,
                tab_current_session_terminal_icon_name: tab_context
                    .current_session_terminal_icon_name
                    .as_deref(),
                tab_current_session_terminal_window_name: tab_context
                    .current_session_terminal_window_name
                    .as_deref(),
                tab_current_session_path: tab_context.current_session_path,
                tab_current_session_profile_name: tab_context.current_session_profile_name,
                tab_current_session_mouse_reporting_mode: tab_context
                    .current_session_mouse_reporting_mode,
                tab_current_session_mouse_info: tab_context.current_session_mouse_info,
                tab_current_session_application_keypad: tab_context
                    .current_session_application_keypad,
                tab_current_session_bell_count: tab_context.current_session_bell_count,
                tab_current_session_columns: tab_context.current_session_columns,
                tab_current_session_rows: tab_context.current_session_rows,
                tab_current_session_selection: tab_context.current_session_selection.as_deref(),
                tab_title: tab_context.title.as_deref(),
                tab_title_override: tab_context.title_override,
                session_selection: session_selection.as_deref(),
                session_mouse_reporting_mode,
                session_mouse_info: self.pane_mouse_info(pane.id()),
                session_application_keypad,
                session_bell_count,
                session_columns: rect.columns,
                session_rows: rect.rows,
            },
        );
        trimmed_badge_text(&badge)
    }

    fn tab_badge_context<'a>(
        &self,
        active_tab: &'a rterm_core::app_shell::Tab,
        layout: &PaneRenderLayout,
    ) -> TabBadgeContext<'a> {
        let active_pane = active_tab.active_pane_id();
        let current_session_launch = tab_current_session_launch(active_tab);
        let active_rect = layout.panes.iter().find(|rect| rect.pane_id == active_pane);
        TabBadgeContext {
            iterm2_pid: std::process::id(),
            window_id: self.app_window_id.get(),
            window_style: iterm_window_style_value(self.full_screen),
            tab_id: active_tab.id().get(),
            current_session_id: active_pane.get(),
            current_session_name: self.pane_title(active_pane),
            current_session_process_id: self.pane_process_id(active_pane),
            current_session_tty_name: self.pane_tty_name(active_pane).map(str::to_owned),
            current_session_job_name: current_session_launch.map(pane_launch_display_program),
            current_session_command_line: current_session_launch.map(pane_launch_command_line),
            current_session_last_command: self.pane_last_command(active_pane),
            current_session_terminal_icon_name: self.pane_terminal_icon_title(active_pane),
            current_session_terminal_window_name: self.pane_terminal_window_title(active_pane),
            current_session_path: tab_current_session_path(active_tab),
            current_session_profile_name: tab_current_session_profile_name(active_tab),
            current_session_mouse_reporting_mode: self.pane_iterm_mouse_reporting_mode(active_pane),
            current_session_mouse_info: self.pane_mouse_info(active_pane),
            current_session_application_keypad: self.pane_application_keypad(active_pane),
            current_session_bell_count: self.pane_bell_count(active_pane),
            current_session_columns: active_rect.map_or(0, |rect| rect.columns),
            current_session_rows: active_rect.map_or(0, |rect| rect.rows),
            current_session_selection: self.badge_session_selection(active_pane),
            title: self.tab_title_for_tab(active_tab),
            title_override: tab_title_override(active_tab),
        }
    }

    fn badge_session_selection(&self, pane_id: rterm_core::PaneId) -> Option<String> {
        (pane_id == self.app_shell.active_pane_id())
            .then(|| self.selected_text())
            .flatten()
    }

    fn push_pane_badge_cells(cells: &mut Vec<RenderCell>, rect: &PaneRenderRect, badge: &str) {
        let chars: Vec<char> = format!(" {badge} ").chars().collect();
        let width = u16::try_from(chars.len()).unwrap_or(u16::MAX);
        let start_column = rect
            .column
            .saturating_add(rect.columns.saturating_sub(width));
        for (offset, ch) in chars.into_iter().enumerate() {
            let offset = u16::try_from(offset).unwrap_or(u16::MAX);
            let column = start_column.saturating_add(offset);
            if column >= rect.column.saturating_add(rect.columns) {
                break;
            }
            cells.push(ui_render_cell(
                rect.row,
                column,
                ch,
                DEFAULT_UI_SURFACE_FOREGROUND,
                DEFAULT_UI_SURFACE_BACKGROUND,
                true,
            ));
        }
    }

    #[expect(
        clippy::similar_names,
        reason = "singular and plural names mirror distinct compatibility API parameters"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
    )]
    fn command_palette_cells(&self) -> Vec<RenderCell> {
        let Some(palette) = self.command_palette.as_ref() else {
            return Vec::new();
        };
        let entries = self.command_palette_filtered_entries();
        if entries.is_empty() {
            return Vec::new();
        }

        let size = self.window_terminal_size_for_layout();
        let columns = size.columns;
        let visible_rows = self
            .command_palette_visible_row_count()
            .min(entries.len())
            .min(usize::from(size.rows));
        if visible_rows == 0 || columns == 0 {
            return Vec::new();
        }

        let selected = palette.selected.min(entries.len().saturating_sub(1));
        let start = selected.saturating_add(1).saturating_sub(visible_rows);
        let first_row = if self.tab_bar_is_visible() && !self.tab_bar_at_bottom {
            TAB_BAR_ROWS
        } else {
            0
        };
        let mut cells = Vec::with_capacity(visible_rows.saturating_mul(usize::from(columns)));

        let launcher_labels = palette.launcher_args.as_ref().and_then(|args| {
            (palette.query.is_empty() && !args.flags.fuzzy && !palette.launcher_fuzzy_filter).then(
                || {
                    quick_select_labels_for_alphabet(
                        args.alphabet.as_deref().unwrap_or(&self.launcher_alphabet),
                        entries.len(),
                    )
                },
            )
        });

        for (visible_index, entry) in entries.iter().skip(start).take(visible_rows).enumerate() {
            let row = first_row.saturating_add(u16::try_from(visible_index).unwrap_or(u16::MAX));
            let entry_index = start + visible_index;
            let is_selected = start + visible_index == selected;
            let foreground = if is_selected {
                DEFAULT_UI_ACCENT_FOREGROUND
            } else {
                self.command_palette_fg_color
                    .unwrap_or(DEFAULT_COMMAND_PALETTE_FG_COLOR)
            };
            let background = if is_selected {
                DEFAULT_UI_ACCENT_BACKGROUND
            } else {
                self.command_palette_bg_color
                    .unwrap_or(DEFAULT_COMMAND_PALETTE_BG_COLOR)
            };

            let row_start = cells.len();
            for column in 0..columns {
                cells.push(ui_render_cell(
                    row,
                    column,
                    ' ',
                    foreground,
                    background,
                    is_selected,
                ));
            }

            let label = entry.display_label(is_selected, self.ui_key_cap_rendering);
            let text_column = if let Some(shortcut_label) = launcher_labels
                .as_ref()
                .and_then(|labels| labels.get(entry_index))
                .filter(|label| !label.is_empty())
            {
                let shortcut_fg = self.launcher_label_fg.map_or(
                    DEFAULT_UI_ACCENT_FOREGROUND,
                    native_color_spec_to_render_color,
                );
                let shortcut_bg = self.launcher_label_bg.map_or(
                    DEFAULT_UI_ACCENT_BACKGROUND,
                    native_color_spec_to_render_color,
                );
                let mut column = 0usize;
                for ch in shortcut_label.chars().take(usize::from(columns)) {
                    if let Some(cell) = cells.get_mut(row_start + column) {
                        *cell = ui_render_cell(
                            row,
                            u16::try_from(column).unwrap_or(u16::MAX),
                            ch,
                            shortcut_fg,
                            shortcut_bg,
                            false,
                        );
                    }
                    column = column.saturating_add(1);
                }
                column.saturating_add(1)
            } else {
                0
            };
            for (offset, ch) in label
                .chars()
                .take(usize::from(columns).saturating_sub(text_column))
                .enumerate()
            {
                let column = text_column.saturating_add(offset);
                if let Some(cell) = cells.get_mut(row_start + column) {
                    cell.ch = ch;
                    cell.text = ch.to_string().into();
                    cell.columns = 1;
                    cell.continuation = false;
                }
            }
        }

        cells
    }

    #[expect(
        clippy::similar_names,
        reason = "singular and plural names mirror distinct compatibility API parameters"
    )]
    fn input_selector_cells(&self) -> Vec<RenderCell> {
        let Some(input_selector) = self.input_selector.as_ref() else {
            return Vec::new();
        };
        let choices = Self::input_selector_filtered_choices(input_selector);
        if choices.is_empty() {
            return Vec::new();
        }

        let size = self.window_terminal_size_for_layout();
        if size.rows == 0 || size.columns == 0 {
            return Vec::new();
        }

        let first_row = if self.tab_bar_is_visible() && !self.tab_bar_at_bottom {
            TAB_BAR_ROWS
        } else {
            0
        };
        let visible_rows = choices.len().min(usize::from(size.rows));
        let selected = input_selector.selected.min(choices.len().saturating_sub(1));
        let start = selected.saturating_add(1).saturating_sub(visible_rows);
        let shortcut_labels = (!input_selector.fuzzy && input_selector.query.is_empty())
            .then(|| quick_select_labels_for_alphabet(&input_selector.alphabet, choices.len()));
        let shortcut_fg = self.input_selector_label_fg.map_or(
            DEFAULT_UI_ACCENT_FOREGROUND,
            native_color_spec_to_render_color,
        );
        let shortcut_bg = self.input_selector_label_bg.map_or(
            DEFAULT_UI_ACCENT_BACKGROUND,
            native_color_spec_to_render_color,
        );
        let mut cells = Vec::with_capacity(visible_rows.saturating_mul(usize::from(size.columns)));

        for (visible_index, choice) in choices.iter().skip(start).take(visible_rows).enumerate() {
            let row = first_row.saturating_add(u16::try_from(visible_index).unwrap_or(u16::MAX));
            let choice_index = start + visible_index;
            let is_selected = choice_index == selected;
            let foreground = if is_selected {
                DEFAULT_UI_ACCENT_FOREGROUND
            } else {
                self.command_palette_fg_color
                    .unwrap_or(DEFAULT_COMMAND_PALETTE_FG_COLOR)
            };
            let background = if is_selected {
                DEFAULT_UI_ACCENT_BACKGROUND
            } else {
                self.command_palette_bg_color
                    .unwrap_or(DEFAULT_COMMAND_PALETTE_BG_COLOR)
            };

            let row_start = cells.len();
            for column in 0..size.columns {
                cells.push(ui_render_cell(
                    row,
                    column,
                    ' ',
                    foreground,
                    background,
                    is_selected,
                ));
            }

            let text_column = if let Some(shortcut_label) = shortcut_labels
                .as_ref()
                .and_then(|labels| labels.get(choice_index))
                .filter(|label| !label.is_empty())
            {
                let mut column = 0usize;
                for ch in shortcut_label.chars().take(usize::from(size.columns)) {
                    if let Some(cell) = cells.get_mut(row_start + column) {
                        *cell = ui_render_cell(
                            row,
                            u16::try_from(column).unwrap_or(u16::MAX),
                            ch,
                            shortcut_fg,
                            shortcut_bg,
                            false,
                        );
                    }
                    column = column.saturating_add(1);
                }
                column.saturating_add(1)
            } else {
                0
            };
            let label = format!("{} {}", if is_selected { '>' } else { ' ' }, choice.label);
            for (offset, ch) in label
                .chars()
                .take(usize::from(size.columns).saturating_sub(text_column))
                .enumerate()
            {
                let column = text_column.saturating_add(offset);
                if let Some(cell) = cells.get_mut(row_start + column) {
                    cell.ch = ch;
                    cell.text = ch.to_string().into();
                    cell.columns = 1;
                    cell.continuation = false;
                }
            }
        }

        cells
    }

    fn char_select_cells(&self) -> Vec<RenderCell> {
        let Some(char_select) = self.char_select.as_ref() else {
            return Vec::new();
        };
        if char_select.matches.is_empty() {
            return Vec::new();
        }

        let size = self.window_terminal_size_for_layout();
        if size.rows == 0 || size.columns == 0 {
            return Vec::new();
        }

        let first_row = if self.tab_bar_is_visible() && !self.tab_bar_at_bottom {
            TAB_BAR_ROWS
        } else {
            0
        };
        let visible_rows = char_select
            .matches
            .len()
            .min(usize::from(size.rows))
            .min(CHAR_SELECT_VISIBLE_ROWS);
        let mut cells = Vec::with_capacity(visible_rows.saturating_mul(usize::from(size.columns)));

        let selected = char_select
            .selected
            .min(char_select.matches.len().saturating_sub(1));
        let start = selected.saturating_add(1).saturating_sub(visible_rows);

        for (visible_index, candidate) in char_select
            .matches
            .iter()
            .skip(start)
            .take(visible_rows)
            .enumerate()
        {
            let row = first_row.saturating_add(u16::try_from(visible_index).unwrap_or(u16::MAX));
            let is_selected = start + visible_index == selected;
            let foreground = if is_selected {
                DEFAULT_UI_ACCENT_FOREGROUND
            } else {
                self.char_select_fg_color
                    .unwrap_or(DEFAULT_CHAR_SELECT_FG_COLOR)
            };
            let background = if is_selected {
                DEFAULT_UI_ACCENT_BACKGROUND
            } else {
                self.char_select_bg_color
                    .unwrap_or(DEFAULT_CHAR_SELECT_BG_COLOR)
            };
            let row_start = cells.len();
            for column in 0..size.columns {
                cells.push(ui_render_cell(
                    row,
                    column,
                    ' ',
                    foreground,
                    background,
                    is_selected,
                ));
            }

            let label = candidate.display_label(is_selected);
            for (column, ch) in label.chars().take(usize::from(size.columns)).enumerate() {
                if let Some(cell) = cells.get_mut(row_start + column) {
                    cell.ch = ch;
                    cell.text = ch.to_string().into();
                    cell.columns = 1;
                    cell.continuation = false;
                }
            }
        }

        cells
    }

    fn debug_overlay_cells(&self) -> Vec<RenderCell> {
        if !self.debug_overlay_active {
            return Vec::new();
        }

        let size = self.window_terminal_size_for_layout();
        if size.rows == 0 || size.columns == 0 {
            return Vec::new();
        }

        let first_row = if self.tab_bar_is_visible() && !self.tab_bar_at_bottom {
            TAB_BAR_ROWS
        } else {
            0
        };
        let max_rows = usize::from(size.rows);
        let lines = self.debug_overlay_lines();
        let visible_rows = lines.len().min(max_rows);
        let mut cells = Vec::with_capacity(visible_rows.saturating_mul(usize::from(size.columns)));

        for (offset, line) in lines.into_iter().take(visible_rows).enumerate() {
            let row = first_row.saturating_add(u16::try_from(offset).unwrap_or(u16::MAX));
            let row_start = cells.len();
            for column in 0..size.columns {
                cells.push(ui_render_cell(
                    row,
                    column,
                    ' ',
                    DEFAULT_UI_SURFACE_FOREGROUND,
                    DEFAULT_COMMAND_PALETTE_BG_COLOR,
                    true,
                ));
            }
            for (column, ch) in line.chars().take(usize::from(size.columns)).enumerate() {
                if let Some(cell) = cells.get_mut(row_start + column) {
                    cell.ch = ch;
                    cell.text = ch.to_string().into();
                    cell.columns = 1;
                    cell.continuation = false;
                }
            }
        }

        cells
    }

    fn debug_overlay_lines(&self) -> Vec<String> {
        let workspace = self.app_shell.active_workspace();
        let active_tab = self.app_shell.active_tab();
        let key_table = self
            .key_table_stack
            .last()
            .map_or("<none>", |activation| activation.name.as_str());
        let mut lines = vec![
            format!(
                "Debug Overlay window={} tab={} pane={} workspace={}",
                self.app_window_id.get(),
                self.app_shell.active_tab_id().get(),
                self.app_shell.active_pane_id().get(),
                workspace.name()
            ),
            format!(
                "tabs={} panes={} scrollback={} bracketed_paste={}",
                workspace.tabs().len(),
                active_tab.panes().len(),
                self.runtime.terminal().scrollback().len(),
                self.runtime.bracketed_paste()
            ),
            format!(
                "fullscreen={} font_scale={:.2} key_table={}",
                self.full_screen, self.font_size_scale, key_table
            ),
        ];
        let recent_logs = self.debug_overlay_recent_log_lines();
        if !recent_logs.is_empty() {
            lines.push("Recent Logs:".to_owned());
            lines.extend(recent_logs);
        }
        lines
    }

    fn debug_overlay_recent_log_lines(&self) -> Vec<String> {
        let mut logs = Vec::new();
        logs.extend(self.unknown_escape_sequence_warnings.iter().cloned());
        logs.extend(self.missing_glyph_warnings.iter().cloned());
        logs.extend(self.debug_key_event_logs.iter().cloned());
        let skipped = logs.len().saturating_sub(DEBUG_OVERLAY_MAX_LOG_LINES);
        logs.into_iter().skip(skipped).collect()
    }

    fn tab_navigator_cells(&self) -> Vec<RenderCell> {
        let Some(tab_navigator) = self.tab_navigator.as_ref() else {
            return Vec::new();
        };
        if tab_navigator.tabs.is_empty() {
            return Vec::new();
        }

        let size = self.window_terminal_size_for_layout();
        let columns = size.columns;
        let rows = size.rows;
        let visible_rows = tab_navigator.tabs.len().min(usize::from(rows));
        if visible_rows == 0 || columns == 0 {
            return Vec::new();
        }

        let selected = tab_navigator
            .selected
            .min(tab_navigator.tabs.len().saturating_sub(1));
        let start = selected.saturating_add(1).saturating_sub(visible_rows);
        let first_row = if self.tab_bar_is_visible() && !self.tab_bar_at_bottom {
            TAB_BAR_ROWS
        } else {
            0
        };
        let mut cells = Vec::with_capacity(visible_rows.saturating_mul(usize::from(columns)));

        for (visible_index, entry) in tab_navigator
            .tabs
            .iter()
            .skip(start)
            .take(visible_rows)
            .enumerate()
        {
            let row = first_row.saturating_add(u16::try_from(visible_index).unwrap_or(u16::MAX));
            let is_selected = start + visible_index == selected;
            let foreground = if is_selected {
                DEFAULT_UI_ACCENT_FOREGROUND
            } else {
                DEFAULT_UI_SUBDUED_FOREGROUND
            };
            let background = if is_selected {
                DEFAULT_UI_ACCENT_BACKGROUND
            } else {
                DEFAULT_COMMAND_PALETTE_BG_COLOR
            };

            let row_start = cells.len();
            for column in 0..columns {
                cells.push(ui_render_cell(
                    row,
                    column,
                    ' ',
                    foreground,
                    background,
                    is_selected,
                ));
            }

            let active_marker = if entry.tab_id == self.app_shell.active_tab_id() {
                '*'
            } else {
                ' '
            };
            let label = format!(
                "{}{} {}",
                if is_selected { '>' } else { ' ' },
                active_marker,
                entry.title
            );
            for (column, ch) in label.chars().take(usize::from(columns)).enumerate() {
                if let Some(cell) = cells.get_mut(row_start + column) {
                    cell.ch = ch;
                    cell.text = ch.to_string().into();
                    cell.columns = 1;
                    cell.continuation = false;
                }
            }
        }

        cells
    }

    fn command_palette_visible_row_count(&self) -> usize {
        self.command_palette_rows.unwrap_or_else(|| {
            usize::from(self.window_terminal_size_for_layout().rows)
                .saturating_sub(2)
                .clamp(1, 14)
        })
    }

    fn tab_bar_cells(&self) -> Vec<RenderCell> {
        if !self.tab_bar_is_visible() {
            self.rendered_tab_bar_layout.replace(None);
            return Vec::new();
        }
        if self.use_fancy_tab_bar {
            self.tab_bar_cells_fancy()
        } else {
            self.tab_bar_cells_retro()
        }
    }
}

impl NativeWindowApp {
    fn tab_bar_cells_retro(&self) -> Vec<RenderCell> {
        self.tab_bar_cells_fancy()
    }

    // WezTerm-style proportional tab bar rendering for `use_fancy_tab_bar = true`.
    #[expect(
        clippy::too_many_lines,
        reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
    )]
    fn tab_bar_cells_fancy(&self) -> Vec<RenderCell> {
        if !self.tab_bar_is_visible() {
            self.rendered_tab_bar_layout.replace(None);
            return Vec::new();
        }

        let columns = self.window_terminal_size_for_layout().columns;
        let background = self.window_frame_title_bar_background_color();
        let tab_bar_foreground = self.window_frame_title_bar_foreground_color();
        let mut cells = (0..columns)
            .map(|column| tab_bar_render_cell(column, ' ', tab_bar_foreground, background, false))
            .collect::<Vec<_>>();

        let hover_column = self.tab_bar_hover_column();
        let mut column =
            u16::try_from(self.macos_native_integrated_title_button_spacer_width()).unwrap_or(0);
        if self.integrated_title_buttons_are_left_aligned() {
            let hovered_button = hover_column
                .and_then(|column| self.integrated_title_button_for_tab_bar_column(column));
            for button in &self.tab_bar_integrated_title_buttons() {
                let hovered = hovered_button == Some(*button);
                write_tab_bar_format_items(
                    &mut cells,
                    &mut column,
                    &self.integrated_title_button_tab_bar_items(*button, hovered),
                    self.integrated_title_button_segment_style(background, hovered),
                );
            }
        }
        if self.modern_tab_bar_uses_compact_labels() {
            // Keep the first tab visually separated from the native traffic
            // lights/window edge without reserving a product-name region.
            column = column.saturating_add(MODERN_TAB_BAR_SIDE_INSET_COLUMNS);
        }
        write_tab_bar_segment(
            &mut cells,
            &mut column,
            &self.tab_bar_workspace_label(),
            if self.modern_tab_bar_uses_compact_labels() {
                Color::Rgb(0x84, 0x92, 0xa6)
            } else {
                tab_bar_foreground
            },
            background,
            !self.modern_tab_bar_uses_compact_labels(),
        );
        if !self.left_status.is_empty() {
            write_tab_bar_ansi_segment(
                &mut cells,
                &mut column,
                &format!("{} ", self.left_status),
                tab_bar_segment_style(tab_bar_foreground, background, false),
            );
        }

        let mut visible_layout = self.build_tab_bar_visible_layout(hover_column);
        let generation = self.rendered_tab_bar_generation.get().wrapping_add(1);
        self.rendered_tab_bar_generation.set(generation);
        visible_layout.generation = generation;
        visible_layout.generation = generation;
        self.paint_fancy_tab_items(
            &mut cells,
            &visible_layout,
            columns,
            hover_column,
            tab_bar_foreground,
            background,
        );
        self.paint_fancy_new_tab(
            &mut cells,
            &visible_layout,
            columns,
            hover_column,
            tab_bar_foreground,
            background,
        );
        let right_integrated_title_buttons_items =
            if self.integrated_title_buttons_are_right_aligned() {
                self.integrated_title_buttons_tab_bar_items(hover_column)
            } else {
                Vec::new()
            };
        let right_integrated_title_buttons_width =
            native_format_items_visible_width(&right_integrated_title_buttons_items);
        if !self.right_status.is_empty() {
            write_right_aligned_tab_bar_segment_with_reserved(
                &mut cells,
                &self.right_status,
                tab_bar_segment_style(tab_bar_foreground, background, false),
                right_integrated_title_buttons_width,
            );
        }
        if right_integrated_title_buttons_width > 0 {
            let mut button_column = columns.saturating_sub(
                u16::try_from(right_integrated_title_buttons_width).unwrap_or(columns),
            );
            let hovered_button = hover_column
                .and_then(|column| self.integrated_title_button_for_tab_bar_column(column));
            for button in &self.tab_bar_integrated_title_buttons() {
                let hovered = hovered_button == Some(*button);
                let style = self.integrated_title_button_segment_style(background, hovered);
                write_tab_bar_format_items(
                    &mut cells,
                    &mut button_column,
                    &self.integrated_title_button_tab_bar_items(*button, hovered),
                    style,
                );
            }
        }

        let row = self.tab_bar_frame_row();
        for cell in &mut cells {
            cell.row = row;
        }

        self.rendered_tab_bar_layout.replace(Some(visible_layout));
        cells
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the tab painter retains ordered compatibility layout decisions"
    )]
    fn paint_fancy_tab_items(
        &self,
        cells: &mut [RenderCell],
        visible_layout: &TabBarVisibleLayout,
        columns: u16,
        hover_column: Option<u16>,
        tab_bar_foreground: Color,
        background: Color,
    ) {
        let mut column;
        if self.show_tabs_in_tab_bar {
            for tab in &visible_layout.tabs {
                let active = tab.active;
                let hovered_style = tab.hovered && !active;

                let defaults = if active {
                    DEFAULT_TAB_BAR_ACTIVE_TAB_COLORS
                } else if hovered_style {
                    DEFAULT_TAB_BAR_INACTIVE_TAB_HOVER_COLORS
                } else {
                    DEFAULT_TAB_BAR_INACTIVE_TAB_COLORS
                };
                let default_foreground = defaults.fg_color.unwrap_or(tab_bar_foreground);
                let default_background = defaults.bg_color.unwrap_or(background);
                let item_colors = if active {
                    self.tab_bar_active_tab_colors
                } else if hovered_style {
                    self.tab_bar_inactive_tab_hover_colors
                } else {
                    self.tab_bar_inactive_tab_colors
                };
                let style = tab_bar_item_segment_style(
                    item_colors,
                    default_foreground,
                    default_background,
                    active,
                );
                let style = if active
                    && self.modern_tab_bar_uses_compact_labels()
                    && self.tab_bar_active_tab_colors == DEFAULT_TAB_BAR_ACTIVE_TAB_COLORS
                    && matches!(&tab.title, NativeTabTitle::Text(_))
                {
                    // A thin cyan underline is the only fixed active-tab
                    // affordance in the compact chrome.  It stays in the
                    // cell style so the GPU/CPU renderers share the same
                    // output and custom tab-bar colors remain compatible.
                    TabBarSegmentStyle {
                        underline_color: DEFAULT_MODERN_ACTIVE_TAB_UNDERLINE,
                        underline_style: UnderlineStyle::Single,
                        ..style
                    }
                } else {
                    style
                };
                column = tab.start_column;
                write_tab_bar_format_items_if_configured(
                    &mut cells[..usize::from(tab.left_edge_end_column.min(columns))],
                    &mut column,
                    tab.left_edge.as_deref(),
                    style,
                );
                write_tab_bar_ansi_segment(
                    &mut cells[..usize::from(tab.prefix_end_column.min(columns))],
                    &mut column,
                    &tab.label.prefix,
                    style,
                );
                let _ = write_tab_bar_title_with_max_width(
                    &mut cells[..usize::from(tab.title_end_column.min(columns))],
                    &mut column,
                    &tab.title,
                    style,
                    usize::MAX,
                );
                write_tab_bar_ansi_segment(
                    &mut cells[..usize::from(tab.suffix_end_column.min(columns))],
                    &mut column,
                    &tab.label.suffix,
                    style,
                );
                write_tab_bar_format_items_if_configured(
                    &mut cells[..usize::from(tab.end_column.min(columns))],
                    &mut column,
                    tab.right_edge.as_deref(),
                    style,
                );

                // Give the default active tab a visual breathing cell without
                // changing its interactive bounds.  Keeping this paint-only
                // margin preserves WezTerm-compatible mouse columns and the
                // existing tab/new-tab hit targets while making the active
                // tile read as a distinct surface in the modern chrome.
                if active && self.tab_bar_style.is_empty() {
                    if tab.start_column > 0
                        && let Some(cell) = cells.get_mut(usize::from(tab.start_column - 1))
                    {
                        *cell = tab_bar_styled_render_cell(tab.start_column - 1, ' ', style);
                    }

                    // Clip one cell at each edge of the default active tile.
                    // The compact prefix and trailing suffix are both blank,
                    // so this creates a rounded/pill-like silhouette without
                    // covering text or changing the tab hit rectangle.
                    if tab.end_column.saturating_sub(tab.start_column) >= 3 {
                        let corner_style = tab_bar_render_cell(
                            tab.start_column,
                            ' ',
                            tab_bar_foreground,
                            background,
                            false,
                        );
                        if let Some(cell) = cells.get_mut(usize::from(tab.start_column)) {
                            *cell = corner_style;
                        }
                        if let Some(cell) = cells.get_mut(usize::from(tab.end_column - 1)) {
                            *cell = tab_bar_render_cell(
                                tab.end_column - 1,
                                ' ',
                                tab_bar_foreground,
                                background,
                                false,
                            );
                        }
                    }
                }

                // Keep the default close affordance quiet at rest, but make
                // the destructive action unmistakable on hover.  This is a
                // paint-only override: explicit tab colors or tab-bar format
                // items retain their WezTerm-defined appearance and the
                // close hit column remains unchanged.
                let close_hovered = self.modern_tab_bar_uses_compact_labels()
                    && hover_column == tab.close_column
                    && ((active
                        && self.tab_bar_active_tab_colors == DEFAULT_TAB_BAR_ACTIVE_TAB_COLORS)
                        || (!active
                            && self.tab_bar_inactive_tab_hover_colors
                                == DEFAULT_TAB_BAR_INACTIVE_TAB_HOVER_COLORS));
                if close_hovered
                    && let Some(close_column) = tab.close_column
                    && let Some(cell) = cells.get_mut(usize::from(close_column))
                {
                    *cell = tab_bar_render_cell(
                        close_column,
                        '×',
                        DEFAULT_MODERN_TAB_CLOSE_HOVER_FOREGROUND,
                        DEFAULT_MODERN_TAB_CLOSE_HOVER_BACKGROUND,
                        false,
                    );
                }
            }
        }
        if let Some(overflow_column) = visible_layout.leading_overflow_column
            && let Some(cell) = cells.get_mut(usize::from(overflow_column))
        {
            let style = tab_bar_item_segment_style(
                self.tab_bar_inactive_tab_colors,
                tab_bar_foreground,
                background,
                true,
            );
            *cell = tab_bar_styled_render_cell(overflow_column, '‹', style);
        }
        if let Some(overflow_column) = visible_layout.overflow_column
            && let Some(cell) = cells.get_mut(usize::from(overflow_column))
        {
            let style = tab_bar_item_segment_style(
                self.tab_bar_inactive_tab_colors,
                tab_bar_foreground,
                background,
                true,
            );
            *cell = tab_bar_styled_render_cell(overflow_column, '…', style);
        }
    }

    fn paint_fancy_new_tab(
        &self,
        cells: &mut [RenderCell],
        visible_layout: &TabBarVisibleLayout,
        columns: u16,
        hover_column: Option<u16>,
        tab_bar_foreground: Color,
        background: Color,
    ) {
        let mut column;
        if let (Some(new_tab_start), Some(new_tab_end)) = (
            visible_layout.new_tab_start_column,
            visible_layout.new_tab_end_column,
        ) {
            column = new_tab_start;
            let new_tab_hovered = hover_column.is_some_and(|hover_column| {
                hover_column >= new_tab_start && hover_column < new_tab_end
            });
            let new_tab_colors = if new_tab_hovered {
                self.tab_bar_new_tab_hover_colors
            } else {
                self.tab_bar_new_tab_colors
            };
            let new_tab_defaults = if new_tab_hovered {
                DEFAULT_TAB_BAR_NEW_TAB_HOVER_COLORS
            } else {
                DEFAULT_TAB_BAR_NEW_TAB_COLORS
            };
            let (left_edge, right_edge) = if new_tab_hovered {
                (
                    self.tab_bar_style
                        .new_tab_hover_left
                        .as_deref()
                        .or(self.tab_bar_style.new_tab_left.as_deref()),
                    self.tab_bar_style
                        .new_tab_hover_right
                        .as_deref()
                        .or(self.tab_bar_style.new_tab_right.as_deref()),
                )
            } else {
                (
                    self.tab_bar_style.new_tab_left.as_deref(),
                    self.tab_bar_style.new_tab_right.as_deref(),
                )
            };
            let mut style = tab_bar_item_segment_style(
                new_tab_colors,
                new_tab_defaults.fg_color.unwrap_or(tab_bar_foreground),
                new_tab_defaults.bg_color.unwrap_or(background),
                true,
            );
            if self.modern_tab_bar_uses_compact_labels()
                && !new_tab_hovered
                && self.tab_bar_new_tab_colors == DEFAULT_TAB_BAR_NEW_TAB_COLORS
            {
                // Keep the default '+' in the same high-emphasis tier as the
                // title controls while leaving explicit WezTerm colors intact.
                style.foreground = DEFAULT_MODERN_WINDOW_BUTTON_FOREGROUND_COLOR;
            }
            let visible_cells = &mut cells[..usize::from(new_tab_end.min(columns))];
            if self.tab_bar_style.new_tab.is_some()
                || (new_tab_hovered && self.tab_bar_style.new_tab_hover.is_some())
            {
                write_tab_bar_format_items(
                    visible_cells,
                    &mut column,
                    &self.new_tab_button_tab_bar_items(new_tab_hovered),
                    style,
                );
            } else {
                write_tab_bar_format_items_if_configured(
                    visible_cells,
                    &mut column,
                    left_edge,
                    style,
                );
                write_tab_bar_ansi_segment(
                    visible_cells,
                    &mut column,
                    self.modern_tab_bar_new_tab_label(),
                    style,
                );
                write_tab_bar_format_items_if_configured(
                    visible_cells,
                    &mut column,
                    right_edge,
                    style,
                );
            }
        }
        if self.modern_tab_bar_uses_compact_labels()
            && let Some(new_tab_end) = visible_layout.new_tab_end_column
            && let Some(cell) = cells.get_mut(usize::from(new_tab_end))
            && cell.ch == ' '
        {
            // Keep the chevron outside the interactive '+' segment so the
            // default new-tab hit target remains byte-for-byte compatible.
            *cell = tab_bar_render_cell(
                new_tab_end,
                '▾',
                DEFAULT_MODERN_NEW_TAB_CHEVRON_FOREGROUND,
                background,
                false,
            );
        }
    }

    fn window_frame_title_bar_background_color(&self) -> Color {
        let default = if self.workspace_chrome_enabled()
            && self.config_overrides.tab_bar_background_color.is_none()
        {
            workspace_chrome::GRAPHITE_TITLEBAR_COLOR
        } else {
            self.tab_bar_background_color
                .unwrap_or(DEFAULT_TAB_BAR_BACKGROUND_COLOR)
        };
        if self.window_focused {
            self.window_frame_appearance
                .active_titlebar_bg
                .unwrap_or(default)
        } else {
            self.window_frame_appearance
                .inactive_titlebar_bg
                .unwrap_or(default)
        }
    }

    fn window_frame_title_bar_foreground_color(&self) -> Color {
        let default = DEFAULT_FOREGROUND_COLOR;
        if self.window_focused {
            self.window_frame_appearance
                .active_titlebar_fg
                .unwrap_or(default)
        } else {
            self.window_frame_appearance
                .inactive_titlebar_fg
                .unwrap_or(default)
        }
    }

    fn window_frame_title_bar_border_bottom_color(&self) -> Color {
        let fallback = self.window_frame_title_bar_background_color();
        let active = self.window_frame_appearance.active_titlebar_border_bottom;
        let inactive = self.window_frame_appearance.inactive_titlebar_border_bottom;
        if self.window_focused {
            active.unwrap_or(fallback)
        } else {
            inactive.unwrap_or(fallback)
        }
    }

    fn window_frame_border_color(
        &self,
        active: Option<Color>,
        inactive: Option<Color>,
        fallback: Color,
    ) -> Color {
        if self.window_focused {
            active.unwrap_or(fallback)
        } else {
            inactive.unwrap_or(fallback)
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
    )]
    fn window_frame_border_cells(&self) -> Vec<RenderCell> {
        let size = self.window_terminal_size_for_layout();
        if size.rows == 0 || size.columns == 0 {
            return Vec::new();
        }

        let columns = size.columns;
        let rows = size.rows;
        let cell_width = self.cell_width();
        let cell_height = self.cell_height();
        let terminal_width = u32::from(columns).saturating_mul(cell_width);
        let terminal_height = u32::from(rows).saturating_mul(cell_height);

        let left_width = self
            .window_frame_appearance
            .border_left_width
            .map_or(0, |width| {
                padding_dimension_to_cells(width, cell_width, terminal_width, self.window_dpi)
            })
            .min(columns);
        let right_width = self
            .window_frame_appearance
            .border_right_width
            .map_or(0, |width| {
                padding_dimension_to_cells(width, cell_width, terminal_width, self.window_dpi)
            })
            .min(columns);
        let top_height = self
            .window_frame_appearance
            .border_top_height
            .map_or(0, |height| {
                padding_dimension_to_cells(height, cell_height, terminal_height, self.window_dpi)
            })
            .min(rows);
        let bottom_height = self
            .window_frame_appearance
            .border_bottom_height
            .map_or(0, |height| {
                padding_dimension_to_cells(height, cell_height, terminal_height, self.window_dpi)
            })
            .min(rows);

        if left_width == 0 && right_width == 0 && top_height == 0 && bottom_height == 0 {
            return Vec::new();
        }

        let content_row_start = self.terminal_frame_row_offset();
        let content_row_end = content_row_start.saturating_add(rows);
        let border_background = self.window_frame_title_bar_background_color();
        let border_default_foreground = self.window_frame_title_bar_foreground_color();
        let left_color = self.window_frame_border_color(
            self.window_frame_appearance.border_left_color,
            self.window_frame_appearance.border_left_color,
            border_background,
        );
        let right_color = self.window_frame_border_color(
            self.window_frame_appearance.border_right_color,
            self.window_frame_appearance.border_right_color,
            border_background,
        );
        let top_color = self.window_frame_border_color(
            self.window_frame_appearance.border_top_color,
            self.window_frame_appearance.border_top_color,
            border_background,
        );
        let bottom_color = self.window_frame_border_color(
            self.window_frame_appearance.border_bottom_color,
            self.window_frame_appearance.border_bottom_color,
            self.window_frame_title_bar_border_bottom_color(),
        );

        let mut cells = Vec::new();
        let top_rows = (0..top_height).map(|offset| content_row_start.saturating_add(offset));
        let bottom_start = content_row_start
            .saturating_add(rows.saturating_sub(bottom_height))
            .min(content_row_end);
        let bottom_rows = bottom_start..content_row_end;

        if top_height > 0 {
            for row in top_rows {
                for column in 0..columns {
                    cells.push(ui_render_cell(row, column, ' ', top_color, top_color, true));
                }
            }
        }

        if bottom_height > 0 && !bottom_rows.is_empty() {
            for row in bottom_rows {
                for column in 0..columns {
                    cells.push(ui_render_cell(
                        row,
                        column,
                        ' ',
                        border_default_foreground,
                        bottom_color,
                        true,
                    ));
                }
            }
        }

        if left_width > 0 {
            let left_end = left_width.min(columns);
            for row in content_row_start..content_row_end {
                for column in 0..left_end {
                    cells.push(ui_render_cell(
                        row, column, ' ', left_color, left_color, true,
                    ));
                }
            }
        }

        if right_width > 0 {
            let right_start = columns.saturating_sub(right_width.min(columns));
            for row in content_row_start..content_row_end {
                for column in right_start..columns {
                    cells.push(ui_render_cell(
                        row,
                        column,
                        ' ',
                        right_color,
                        right_color,
                        true,
                    ));
                }
            }
        }

        cells
    }

    #[expect(
        clippy::too_many_lines,
        reason = "tab-bar hit-testing keeps mouse actions and drag initialization together"
    )]
    fn handle_tab_bar_mouse_input(&mut self, state: ElementState, button: MouseButton) -> bool {
        if state != ElementState::Pressed
            || !matches!(
                button,
                MouseButton::Left | MouseButton::Right | MouseButton::Middle
            )
        {
            return false;
        }

        let Some(column) = self.tab_bar_column_at_mouse_position() else {
            return false;
        };
        let rendered_drag_source = self.rendered_tab_bar_body_target_for_column(column);
        if let Some(integrated_button) = self.integrated_title_button_for_tab_bar_column(column) {
            if button != MouseButton::Left {
                return false;
            }
            self.dispatch_integrated_title_button(integrated_button);
            return true;
        }
        if self.new_tab_button_for_tab_bar_column(column) {
            let event = NativeWindowNewTabButtonClick {
                window_id: self.app_window_id,
                pane: self.app_shell.active_pane_id(),
                button,
                default_action: Self::new_tab_button_default_action(button),
            };
            if !self.dispatch_new_tab_button_click(&event) {
                return true;
            }
            // Preserve the event contract for existing `new-tab-button-click`
            // handlers: their supplied default action still creates a tab.
            // With no Lua event handler, the browser-style button instead
            // opens the session launcher.
            let default_action = if self.lua_new_tab_button_click.is_some() {
                event.default_action
            } else {
                Some(WindowCommand::ShowLauncher)
            };
            let Some(default_action) = default_action else {
                return true;
            };
            if let Err(error) = self.command_palette_apply_command(default_action) {
                eprintln!("tab bar new tab failed: {error:?}");
                return false;
            }
            return true;
        }

        if button == MouseButton::Left {
            let (is_leading_overflow, is_trailing_overflow) = {
                let layout = self.current_tab_bar_visible_layout(Some(column));
                (
                    layout.leading_overflow_column == Some(column),
                    layout.overflow_column == Some(column),
                )
            };
            if is_leading_overflow {
                self.tab_bar_scroll_position = self.tab_bar_scroll_position.saturating_sub(1);
                self.rendered_tab_bar_layout.replace(None);
                self.frame_needs_full_repaint = true;
                return true;
            }
            if is_trailing_overflow {
                let tab_count = self.app_shell.active_workspace().tabs().len();
                self.tab_bar_scroll_position = self
                    .tab_bar_scroll_position
                    .saturating_add(1)
                    .min(tab_count.saturating_sub(1));
                self.rendered_tab_bar_layout.replace(None);
                self.frame_needs_full_repaint = true;
                return true;
            }
        }

        if button == MouseButton::Middle {
            let Some(tab) = self.tab_for_tab_bar_column(column) else {
                return false;
            };
            if let Err(error) =
                self.dispatch_close_tab_action(tab, self.switch_to_last_active_tab_when_closing_tab)
            {
                eprintln!("tab bar middle-click close failed: {error:?}");
                return false;
            }
            return true;
        }

        if button == MouseButton::Right {
            let Some(tab) = self.tab_for_tab_bar_column(column) else {
                return false;
            };
            if let Err(error) = self.enter_tab_context_menu(tab) {
                eprintln!("tab context menu failed: {error:?}");
                return false;
            }
            return true;
        }

        if button != MouseButton::Left {
            return false;
        }

        if let Some(tab) = self.close_tab_for_tab_bar_column(column) {
            if let Err(error) = self.dispatch_app_action(AppAction::CloseTab {
                tab,
                switch_to_last_active: self.switch_to_last_active_tab_when_closing_tab,
            }) {
                eprintln!("tab bar close failed: {error:?}");
                return false;
            }
            return true;
        }

        let Some(tab) = self.tab_for_tab_bar_column(column) else {
            // Unified/borderless chrome still needs an ergonomic title-bar
            // affordance. Empty tab-bar cells are safe to use for dragging;
            // tab, new-tab, and title-button cells were handled above.
            if self.tab_bar_provides_window_drag_region() {
                self.start_window_drag();
                return true;
            }
            return false;
        };

        if let Err(error) = self.dispatch_app_action(AppAction::ActivateTab { tab }) {
            eprintln!("tab bar activation failed: {error:?}");
            return false;
        }
        if rendered_drag_source == Some(tab) {
            self.active_mouse_button = Some(MouseButton::Left);
            self.tab_bar_drag = Some(TabBarDrag {
                source_tab_id: tab,
                pressed_pixel_x: self.mouse_pixel_position.map_or(0.0, |position| position.x),
                moved: false,
            });
        }
        self.clear_ordinary_selection();
        self.selecting = false;
        self.last_left_click = None;

        true
    }

    fn handle_tab_bar_drag_release(&mut self) -> bool {
        let Some(drag) = self.tab_bar_drag.take() else {
            return false;
        };
        self.ui_left_release_pending = false;

        if !drag.moved || self.app_shell.active_tab_id() != drag.source_tab_id {
            return true;
        }
        let Some(column) = self.tab_bar_column_at_mouse_position() else {
            return true;
        };
        let Some(target_tab_id) = self.rendered_tab_bar_body_target_for_column(column) else {
            return true;
        };
        if target_tab_id == drag.source_tab_id {
            return true;
        }
        let tabs = self.app_shell.active_workspace().tabs();
        let Some(source_position) = tabs.iter().position(|tab| tab.id() == drag.source_tab_id)
        else {
            return true;
        };
        let Some(target_position) = tabs.iter().position(|tab| tab.id() == target_tab_id) else {
            return true;
        };
        if source_position == target_position {
            return true;
        }

        if let Err(error) = self.dispatch_app_action(AppAction::MoveTab {
            index: target_position,
        }) {
            eprintln!("tab bar reorder failed: {error:?}");
        }
        true
    }

    fn update_tab_bar_drag_from_mouse_position(&mut self) -> bool {
        let Some(mut drag) = self.tab_bar_drag else {
            return false;
        };
        if self.mouse_pixel_position.is_some_and(|position| {
            (position.x - drag.pressed_pixel_x).abs() >= 6.0 * self.window_dpi_scale()
        }) {
            drag.moved = true;
            self.tab_bar_drag = Some(drag);
        }
        true
    }

    fn tab_bar_column_at_mouse_position(&self) -> Option<u16> {
        let position = self.mouse_pixel_position?;
        let content_left = f64::from(self.frame_content_pixel_left());
        let content_right = f64::from(
            self.frame_content_pixel_left()
                .saturating_add(self.frame_content_placement().width),
        );
        if !position.x.is_finite()
            || !position.y.is_finite()
            || position.x < content_left
            || position.x >= content_right
            || position.y < f64::from(self.tab_bar_pixel_top())
            || position.y
                >= f64::from(
                    self.tab_bar_pixel_top()
                        .saturating_add(self.tab_bar_pixel_height()),
                )
        {
            return None;
        }

        pixel_axis_to_cell(position.x - content_left, self.cell_width())
    }

    fn tab_for_tab_bar_column(&self, column: u16) -> Option<rterm_core::TabId> {
        if !self.show_tabs_in_tab_bar {
            return None;
        }
        self.tab_bar_tab_target_for_column(column)
            .map(|(_, tab_id)| tab_id)
    }

    fn tab_bar_tab_target_for_column(&self, column: u16) -> Option<(usize, rterm_core::TabId)> {
        self.current_tab_bar_visible_layout(Some(column))
            .tabs
            .iter()
            .find(|tab| column >= tab.start_column && column < tab.end_column)
            .map(|tab| (tab.position, tab.tab_id))
    }

    fn rendered_tab_bar_body_target_for_column(&self, column: u16) -> Option<rterm_core::TabId> {
        if !self.show_tabs_in_tab_bar {
            return None;
        }
        let layout = self.rendered_tab_bar_layout.borrow();
        let layout = layout.as_ref()?;
        if layout.generation == 0 {
            return None;
        }
        layout
            .tabs
            .iter()
            .find(|tab| {
                column >= tab.start_column
                    && column < tab.end_column
                    && tab.close_column != Some(column)
            })
            .map(|tab| tab.tab_id)
    }

    fn new_tab_button_for_tab_bar_column(&self, column: u16) -> bool {
        if !self.show_new_tab_button_in_tab_bar {
            return false;
        }
        let layout = self.current_tab_bar_visible_layout(Some(column));
        let Some(start) = layout.new_tab_start_column else {
            return false;
        };
        let Some(end) = layout.new_tab_end_column else {
            return false;
        };
        column >= start && column < end
    }

    fn new_tab_button_default_action(button: MouseButton) -> Option<WindowCommand> {
        match button {
            MouseButton::Left | MouseButton::Right => Some(WindowCommand::NewTab),
            _ => None,
        }
    }

    fn new_tab_button_tab_bar_items(&self, hovered: bool) -> Vec<NativeFormatItem> {
        if hovered
            && let Some(items) = self
                .tab_bar_style
                .new_tab_hover
                .as_ref()
                .or(self.tab_bar_style.new_tab.as_ref())
        {
            return items.clone();
        }
        if let Some(items) = self.tab_bar_style.new_tab.as_ref() {
            return items.clone();
        }

        let mut items = Vec::new();
        let (left_edge, right_edge) = if hovered {
            (
                self.tab_bar_style
                    .new_tab_hover_left
                    .as_deref()
                    .or(self.tab_bar_style.new_tab_left.as_deref()),
                self.tab_bar_style
                    .new_tab_hover_right
                    .as_deref()
                    .or(self.tab_bar_style.new_tab_right.as_deref()),
            )
        } else {
            (
                self.tab_bar_style.new_tab_left.as_deref(),
                self.tab_bar_style.new_tab_right.as_deref(),
            )
        };
        if let Some(left_edge) = left_edge {
            items.extend_from_slice(left_edge);
        }
        items.push(NativeFormatItem::Text(
            self.modern_tab_bar_new_tab_label().to_owned(),
        ));
        if let Some(right_edge) = right_edge {
            items.extend_from_slice(right_edge);
        }
        items
    }

    fn new_tab_button_tab_bar_width(&self) -> usize {
        native_format_items_visible_width(&self.new_tab_button_tab_bar_items(false)).max(
            native_format_items_visible_width(&self.new_tab_button_tab_bar_items(true)),
        )
    }

    fn integrated_title_buttons_are_visible(&self) -> bool {
        self.window_decorations.integrated_buttons
            && !self.integrated_title_buttons.is_empty()
            && (self.integrated_title_button_style != NativeIntegratedTitleButtonStyle::MacOsNative
                || self.macos_self_drawn_window_controls())
    }

    fn tab_bar_integrated_title_buttons(&self) -> Vec<NativeIntegratedTitleButton> {
        let default_order = [
            NativeIntegratedTitleButton::Hide,
            NativeIntegratedTitleButton::Maximize,
            NativeIntegratedTitleButton::Close,
        ];
        if self.macos_self_drawn_window_controls()
            && self.config_overrides.integrated_title_buttons.is_none()
            && self.integrated_title_buttons.as_slice() == default_order
        {
            return vec![
                NativeIntegratedTitleButton::Close,
                NativeIntegratedTitleButton::Hide,
                NativeIntegratedTitleButton::Maximize,
            ];
        }
        self.integrated_title_buttons.clone()
    }

    fn macos_self_drawn_window_controls(&self) -> bool {
        cfg!(target_os = "macos")
            && self.modern_tab_bar_uses_compact_labels()
            && self.window_decorations.title
            && self.window_decorations.integrated_buttons
            && !self.tab_bar_at_bottom
    }

    fn macos_native_integrated_title_button_spacer_width(&self) -> usize {
        if self.window_decorations.integrated_buttons
            && self.window_decorations.title
            && self.integrated_title_button_style == NativeIntegratedTitleButtonStyle::MacOsNative
            && !self.macos_self_drawn_window_controls()
            && !self.tab_bar_at_bottom
        {
            10
        } else {
            0
        }
    }

    fn integrated_title_buttons_are_left_aligned(&self) -> bool {
        self.integrated_title_buttons_are_visible()
            && (self.macos_self_drawn_window_controls()
                || self.integrated_title_button_alignment
                    == NativeIntegratedTitleButtonAlignment::Left)
    }

    fn integrated_title_buttons_are_right_aligned(&self) -> bool {
        self.integrated_title_buttons_are_visible()
            && !self.macos_self_drawn_window_controls()
            && self.integrated_title_button_alignment == NativeIntegratedTitleButtonAlignment::Right
    }

    fn tab_bar_provides_window_drag_region(&self) -> bool {
        self.integrated_title_buttons_are_visible()
            || (cfg!(target_os = "macos")
                && self.window_decorations.title
                && self.window_decorations.integrated_buttons
                && self.integrated_title_button_style
                    == NativeIntegratedTitleButtonStyle::MacOsNative
                && !self.macos_self_drawn_window_controls()
                && !self.tab_bar_at_bottom)
    }

    fn integrated_title_buttons_tab_bar_items(
        &self,
        hover_column: Option<u16>,
    ) -> Vec<NativeFormatItem> {
        let hovered_button =
            hover_column.and_then(|column| self.integrated_title_button_for_tab_bar_column(column));
        let mut items = Vec::new();
        for button in &self.tab_bar_integrated_title_buttons() {
            items.extend(
                self.integrated_title_button_tab_bar_items(
                    *button,
                    hovered_button == Some(*button),
                ),
            );
        }
        items
    }

    fn integrated_title_button_tab_bar_items(
        &self,
        button: NativeIntegratedTitleButton,
        hovered: bool,
    ) -> Vec<NativeFormatItem> {
        let (normal, hover) = match button {
            NativeIntegratedTitleButton::Hide => (
                self.tab_bar_style.window_hide.as_ref(),
                self.tab_bar_style.window_hide_hover.as_ref(),
            ),
            NativeIntegratedTitleButton::Maximize => (
                self.tab_bar_style.window_maximize.as_ref(),
                self.tab_bar_style.window_maximize_hover.as_ref(),
            ),
            NativeIntegratedTitleButton::Close => (
                self.tab_bar_style.window_close.as_ref(),
                self.tab_bar_style.window_close_hover.as_ref(),
            ),
        };
        if hovered { hover.or(normal) } else { normal }
            .cloned()
            .unwrap_or_else(|| {
                if self.macos_self_drawn_window_controls() {
                    let color = match button {
                        NativeIntegratedTitleButton::Close => Color::Rgb(0xff, 0x5f, 0x57),
                        NativeIntegratedTitleButton::Hide => Color::Rgb(0xfe, 0xbc, 0x2e),
                        NativeIntegratedTitleButton::Maximize => Color::Rgb(0x28, 0xc8, 0x40),
                    };
                    return vec![
                        NativeFormatItem::Foreground(color),
                        NativeFormatItem::Text(" ● ".to_owned()),
                        NativeFormatItem::ResetAttributes,
                    ];
                }
                let label = if self.modern_tab_bar_uses_compact_labels() {
                    match button {
                        NativeIntegratedTitleButton::Hide => "  —  ",
                        NativeIntegratedTitleButton::Maximize => "  □  ",
                        NativeIntegratedTitleButton::Close => "  ×  ",
                    }
                } else {
                    integrated_title_button_default_tab_bar_label(button)
                };
                vec![NativeFormatItem::Text(label.to_owned())]
            })
    }

    fn integrated_title_buttons_tab_bar_width(&self) -> usize {
        if self.integrated_title_buttons_are_visible() {
            self.tab_bar_integrated_title_buttons()
                .iter()
                .map(|button| self.integrated_title_button_tab_bar_width(*button))
                .sum()
        } else {
            0
        }
    }

    fn integrated_title_button_tab_bar_width(&self, button: NativeIntegratedTitleButton) -> usize {
        let normal = native_format_items_visible_width(
            &self.integrated_title_button_tab_bar_items(button, false),
        );
        let hover = native_format_items_visible_width(
            &self.integrated_title_button_tab_bar_items(button, true),
        );
        normal.max(hover)
    }

    fn integrated_title_button_segment_style(
        &self,
        background: Color,
        hovered: bool,
    ) -> TabBarSegmentStyle {
        let configured_button_background = if hovered {
            self.window_frame_appearance.button_hover_bg
        } else {
            self.window_frame_appearance.button_bg
        };
        let button_background = configured_button_background.unwrap_or_else(|| {
            if hovered
                && self.modern_tab_bar_uses_compact_labels()
                && self.window_frame_appearance.button_bg.is_none()
            {
                DEFAULT_MODERN_WINDOW_BUTTON_HOVER_BACKGROUND
            } else {
                background
            }
        });
        let button_foreground = if hovered {
            self.window_frame_appearance.button_hover_fg
        } else {
            self.window_frame_appearance.button_fg
        };
        let foreground = match button_foreground {
            Some(foreground) => foreground,
            None => match self.integrated_title_button_color {
                NativeIntegratedTitleButtonColor::Auto => {
                    if self.modern_tab_bar_uses_compact_labels() {
                        DEFAULT_MODERN_WINDOW_BUTTON_FOREGROUND_COLOR
                    } else {
                        DEFAULT_FOREGROUND_COLOR
                    }
                }
                NativeIntegratedTitleButtonColor::Color(color) => color,
            },
        };
        tab_bar_segment_style(foreground, button_background, true)
    }

    fn integrated_title_button_for_tab_bar_column(
        &self,
        column: u16,
    ) -> Option<NativeIntegratedTitleButton> {
        if !self.integrated_title_buttons_are_visible() {
            return None;
        }

        let label_width = u16::try_from(self.integrated_title_buttons_tab_bar_width()).ok()?;
        let start = if self.integrated_title_buttons_are_left_aligned() {
            0
        } else {
            self.runtime
                .terminal()
                .grid()
                .size()
                .columns
                .saturating_sub(label_width)
        };
        if column < start || column >= start.saturating_add(label_width) {
            return None;
        }

        let mut cursor = start;
        for button in &self.tab_bar_integrated_title_buttons() {
            let width = u16::try_from(self.integrated_title_button_tab_bar_width(*button)).ok()?;
            let end = cursor.saturating_add(width);
            if column >= cursor && column < end {
                return Some(*button);
            }
            cursor = end;
        }

        None
    }

    fn dispatch_integrated_title_button(&mut self, button: NativeIntegratedTitleButton) {
        match button {
            NativeIntegratedTitleButton::Hide => self.hide_window(),
            NativeIntegratedTitleButton::Maximize => self.toggle_window_maximized(),
            NativeIntegratedTitleButton::Close => self.handle_window_close_requested(),
        }
    }

    fn tab_bar_hover_column(&self) -> Option<u16> {
        let position = self.mouse_pixel_position?;
        if !position.x.is_finite()
            || !position.y.is_finite()
            || position.x < f64::from(self.frame_content_pixel_left())
            || position.y < f64::from(self.tab_bar_pixel_top())
            || position.y
                >= f64::from(
                    self.tab_bar_pixel_top()
                        .saturating_add(self.tab_bar_pixel_height()),
                )
        {
            return None;
        }

        pixel_axis_to_cell(
            position.x - f64::from(self.frame_content_pixel_left()),
            self.cell_width(),
        )
    }

    fn close_tab_for_tab_bar_column(&self, column: u16) -> Option<rterm_core::TabId> {
        if !self.show_tabs_in_tab_bar {
            return None;
        }
        self.current_tab_bar_visible_layout(Some(column))
            .tabs
            .iter()
            .find(|tab| tab.close_column == Some(column))
            .map(|tab| tab.tab_id)
    }

    fn tab_bar_left_prefix_width(&self) -> Option<u16> {
        let mut width =
            u16::try_from(self.macos_native_integrated_title_button_spacer_width()).ok()?;
        width = width.checked_add(if self.integrated_title_buttons_are_left_aligned() {
            u16::try_from(self.integrated_title_buttons_tab_bar_width()).ok()?
        } else {
            0
        })?;
        if self.modern_tab_bar_uses_compact_labels() {
            width = width.checked_add(MODERN_TAB_BAR_SIDE_INSET_COLUMNS)?;
        }
        let workspace_label = self.tab_bar_workspace_label();
        width = width.checked_add(
            u16::try_from(tab_bar_ansi_visible_width(&workspace_label)).ok()?,
        )?;
        if !self.left_status.is_empty() {
            let left_status_width =
                u16::try_from(tab_bar_ansi_visible_width(&self.left_status) + 1).ok()?;
            width = width.checked_add(left_status_width)?;
        }
        Some(width)
    }

    fn tab_bar_workspace_label(&self) -> String {
        if self.modern_tab_bar_uses_compact_labels()
            && self.app_shell.active_workspace().name() == DEFAULT_WORKSPACE_NAME
        {
            return String::new();
        }
        format!(" ws:{} ", self.app_shell.active_workspace().name())
    }

    fn modern_tab_bar_new_tab_label(&self) -> &'static str {
        if self.modern_tab_bar_uses_compact_labels() {
            " +  "
        } else {
            tab_bar_new_tab_label()
        }
    }

    fn modern_tab_bar_uses_compact_labels(&self) -> bool {
        self.modern_tab_bar_brand && self.tab_bar_style.is_empty()
    }

    fn tab_bar_label_options(&self) -> TabBarTabLabelOptions {
        TabBarTabLabelOptions {
            show_tab_index: self.show_tab_index_in_tab_bar,
            zero_based_tab_index: self.tab_and_split_indices_are_zero_based,
            show_close_button: self.show_close_tab_button_in_tabs,
        }
    }

    fn current_tab_bar_visible_layout(
        &self,
        fallback_hover_column: Option<u16>,
    ) -> Ref<'_, TabBarVisibleLayout> {
        if self.rendered_tab_bar_layout.borrow().is_none() {
            let layout = self.build_tab_bar_visible_layout(fallback_hover_column);
            self.rendered_tab_bar_layout.replace(Some(layout));
        }
        Ref::map(self.rendered_tab_bar_layout.borrow(), |layout| {
            layout
                .as_ref()
                .expect("tab bar layout must be initialized before it is borrowed")
        })
    }

    #[expect(
        clippy::too_many_lines,
        reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
    )]
    fn build_tab_bar_visible_layout(&self, hover_column: Option<u16>) -> TabBarVisibleLayout {
        let columns = self.window_terminal_size_for_layout().columns;
        let left_prefix_width = self.tab_bar_left_prefix_width().unwrap_or(0);
        let right_status_width =
            u16::try_from(tab_bar_ansi_visible_width(&self.right_status)).unwrap_or(u16::MAX);
        let right_button_width = if self.integrated_title_buttons_are_right_aligned() {
            u16::try_from(self.integrated_title_buttons_tab_bar_width()).unwrap_or(u16::MAX)
        } else {
            0
        };
        let interactive_end = columns
            .saturating_sub(right_button_width)
            .saturating_sub(right_status_width)
            .saturating_sub(if self.workspace_chrome_enabled() {
                workspace_chrome::ACTION_RESERVE_COLUMNS
            } else {
                0
            });
        let requested_new_tab_width = if self.show_new_tab_button_in_tab_bar {
            u16::try_from(self.new_tab_button_tab_bar_width()).unwrap_or(u16::MAX)
        } else {
            0
        };
        let tabs_need_room =
            self.show_tabs_in_tab_bar && !self.app_shell.active_workspace().tabs().is_empty();
        let new_tab_width = if !tabs_need_room
            || interactive_end.saturating_sub(left_prefix_width) > requested_new_tab_width
        {
            requested_new_tab_width
        } else {
            0
        };
        let tab_area_end = interactive_end.saturating_sub(new_tab_width);
        let tab_width_max = if self.use_fancy_tab_bar {
            self.tab_title_second_pass_max_width_with_new_tab_width(usize::from(new_tab_width))
                .max(1)
        } else {
            usize::MAX
        };
        let title_context = self
            .show_tabs_in_tab_bar
            .then(|| self.tab_bar_title_context());
        let active_tab_id = self.app_shell.active_tab_id();
        let mut cursor = left_prefix_width;
        let mut tabs = Vec::new();

        if self.show_tabs_in_tab_bar {
            for (position, tab) in self.app_shell.active_workspace().tabs().iter().enumerate() {
                let active = tab.id() == active_tab_id;
                let first_pass_title = self.formatted_tab_title_for_tab_first_pass_with_context(
                    position,
                    tab,
                    title_context
                        .as_ref()
                        .expect("tab title context must exist when tabs are rendered"),
                );
                let title_text = first_pass_title.as_ref().map(NativeTabTitle::plain_text);
                let first_pass_title_width = first_pass_title
                    .as_ref()
                    .map_or(0, native_tab_title_visible_width);
                let mut label = tab_bar_tab_label_segments(
                    position,
                    tab.id(),
                    tab.panes().len(),
                    active,
                    title_text.as_deref(),
                    Self::tab_progress_for_tab(tab),
                    self.tab_bar_label_options(),
                );
                if self.modern_tab_bar_uses_compact_labels() {
                    // The modern title bar uses a compact terminal glyph in
                    // each tab and keeps the session title as the only text.
                    // This removes diagnostic indices/pane counts and the
                    // application-name badge while retaining the existing
                    // close hit target and formatter compatibility path.
                    " [>_] ".clone_into(&mut label.prefix);
                    label.suffix = if self.show_close_tab_button_in_tabs {
                        "  ×  ".to_owned()
                    } else {
                        "  ".to_owned()
                    };
                }
                let allocated_title_width = if first_pass_title_width == 0 {
                    tab_width_max.max(1)
                } else {
                    first_pass_title_width.min(tab_width_max)
                };
                let (normal_left_edge, normal_right_edge) = self.tab_bar_tab_edges(active, false);
                let normal_width = normal_left_edge
                    .as_deref()
                    .map_or(0, native_format_items_visible_width)
                    .saturating_add(tab_bar_ansi_visible_width(&label.prefix))
                    .saturating_add(allocated_title_width)
                    .saturating_add(tab_bar_ansi_visible_width(&label.suffix))
                    .saturating_add(
                        normal_right_edge
                            .as_deref()
                            .map_or(0, native_format_items_visible_width),
                    );
                let provisional_end =
                    cursor.saturating_add(u16::try_from(normal_width).unwrap_or(u16::MAX));
                let hovered = hover_column.is_some_and(|hover_column| {
                    hover_column >= cursor
                        && hover_column < provisional_end
                        && hover_column < tab_area_end
                });
                let (left_edge, right_edge) = if hovered {
                    self.tab_bar_tab_edges(active, true)
                } else {
                    (normal_left_edge, normal_right_edge)
                };
                let title = self
                    .formatted_tab_title_for_tab_with_max_width_and_context(
                        position,
                        tab,
                        hovered,
                        tab_width_max,
                        title_context
                            .as_ref()
                            .expect("tab title context must exist when tabs are rendered"),
                    )
                    .or(first_pass_title)
                    .unwrap_or_else(|| NativeTabTitle::Text(label.title.clone()));

                let start_column = cursor;
                let left_edge_end_column = cursor.saturating_add(
                    u16::try_from(
                        left_edge
                            .as_deref()
                            .map_or(0, native_format_items_visible_width),
                    )
                    .unwrap_or(u16::MAX),
                );
                let prefix_end_column = left_edge_end_column.saturating_add(
                    u16::try_from(tab_bar_ansi_visible_width(&label.prefix)).unwrap_or(u16::MAX),
                );
                let title_end_column = prefix_end_column.saturating_add(
                    u16::try_from(native_tab_title_visible_width(&title)).unwrap_or(u16::MAX),
                );
                let suffix_end_column = title_end_column.saturating_add(
                    u16::try_from(tab_bar_ansi_visible_width(&label.suffix)).unwrap_or(u16::MAX),
                );
                let end_column = suffix_end_column.saturating_add(
                    u16::try_from(
                        right_edge
                            .as_deref()
                            .map_or(0, native_format_items_visible_width),
                    )
                    .unwrap_or(u16::MAX),
                );
                let close_column = if self.show_close_tab_button_in_tabs {
                    let prefix_text = tab_bar_ansi_plain_text(&label.prefix);
                    if let Some(offset) = prefix_text.chars().position(|ch| ch == 'x' || ch == '×')
                    {
                        Some(
                            left_edge_end_column
                                .saturating_add(u16::try_from(offset).unwrap_or(u16::MAX)),
                        )
                    } else {
                        tab_bar_ansi_plain_text(&label.suffix)
                            .chars()
                            .position(|ch| ch == 'x' || ch == '×')
                            .map(|offset| {
                                title_end_column
                                    .saturating_add(u16::try_from(offset).unwrap_or(u16::MAX))
                            })
                    }
                } else {
                    None
                };

                tabs.push(TabBarVisibleTabLayout {
                    position,
                    tab_id: tab.id(),
                    active,
                    hovered,
                    start_column,
                    end_column,
                    left_edge_end_column,
                    prefix_end_column,
                    title_end_column,
                    suffix_end_column,
                    left_edge,
                    label,
                    title,
                    right_edge,
                    close_column,
                });
                cursor = end_column;
            }
        }

        let tabs_were_clipped = tabs_need_room && cursor > tab_area_end;
        let mut leading_overflow_column = None;
        let active_position = tabs.iter().position(|tab| tab.active).unwrap_or_default();
        let scroll_position = self.tab_bar_scroll_position.min(active_position);
        if scroll_position > 0 {
            leading_overflow_column =
                (tab_area_end > left_prefix_width).then_some(left_prefix_width);
            let mut relocated_cursor =
                left_prefix_width.saturating_add(u16::from(leading_overflow_column.is_some()));
            tabs = tabs
                .into_iter()
                .skip(scroll_position)
                .map(|mut tab| {
                    tab.reposition(relocated_cursor);
                    relocated_cursor = tab.end_column;
                    tab.hovered = hover_column.is_some_and(|column| {
                        column >= tab.start_column && column < tab.end_column
                    });
                    tab
                })
                .collect();
        }
        let overflow_column = if tabs_were_clipped {
            if tab_area_end > left_prefix_width {
                Some(tab_area_end.saturating_sub(1))
            } else if interactive_end > 0 {
                Some(interactive_end.saturating_sub(1))
            } else {
                None
            }
        } else {
            None
        };
        let visible_tab_end = overflow_column.unwrap_or(tab_area_end);
        let active_tab_was_outside_view = tabs_were_clipped
            && tabs
                .iter()
                .find(|tab| tab.active)
                .is_some_and(|tab| tab.start_column >= visible_tab_end);
        if active_tab_was_outside_view
            && let Some(active_position) = tabs.iter().position(|tab| tab.active)
        {
            // Move the viewport to the active tab instead of leaving keyboard-
            // selected or newly-created tabs hidden beyond the right edge.
            // A leading chevron communicates that earlier tabs still exist.
            leading_overflow_column =
                (tab_area_end > left_prefix_width).then_some(left_prefix_width);
            let mut relocated_cursor =
                left_prefix_width.saturating_add(u16::from(leading_overflow_column.is_some()));
            tabs = tabs
                .into_iter()
                .skip(active_position)
                .map(|mut tab| {
                    tab.reposition(relocated_cursor);
                    relocated_cursor = tab.end_column;
                    tab.hovered = hover_column.is_some_and(|column| {
                        column >= tab.start_column && column < tab.end_column
                    });
                    tab
                })
                .collect();
        }
        tabs.retain(|tab| tab.start_column < visible_tab_end);
        for tab in &mut tabs {
            tab.end_column = tab.end_column.min(visible_tab_end);
            tab.left_edge_end_column = tab.left_edge_end_column.min(tab.end_column);
            tab.prefix_end_column = tab.prefix_end_column.min(tab.end_column);
            tab.title_end_column = tab.title_end_column.min(tab.end_column);
            tab.suffix_end_column = tab.suffix_end_column.min(tab.end_column);
            if tab
                .close_column
                .is_some_and(|column| column >= tab.end_column)
            {
                tab.close_column = None;
            }
        }

        let tabs_end = tabs.last().map_or(left_prefix_width, |tab| tab.end_column);
        let new_tab_start_column = if self.show_new_tab_button_in_tab_bar
            && new_tab_width > 0
            && interactive_end >= left_prefix_width.saturating_add(new_tab_width)
        {
            Some(if overflow_column.is_some() {
                tab_area_end
            } else {
                tabs_end
            })
        } else {
            None
        };
        let new_tab_end_column = new_tab_start_column
            .map(|start| start.saturating_add(new_tab_width).min(interactive_end));

        TabBarVisibleLayout {
            tabs,
            leading_overflow_column,
            overflow_column,
            new_tab_start_column,
            new_tab_end_column,
            generation: 0,
        }
    }

    fn tab_bar_tab_edges(
        &self,
        active: bool,
        hovered: bool,
    ) -> (Option<Vec<NativeFormatItem>>, Option<Vec<NativeFormatItem>>) {
        if active {
            (
                self.tab_bar_style.active_tab_left.clone(),
                self.tab_bar_style.active_tab_right.clone(),
            )
        } else if hovered {
            (
                self.tab_bar_style
                    .inactive_tab_hover_left
                    .clone()
                    .or_else(|| self.tab_bar_style.inactive_tab_left.clone()),
                self.tab_bar_style
                    .inactive_tab_hover_right
                    .clone()
                    .or_else(|| self.tab_bar_style.inactive_tab_right.clone()),
            )
        } else {
            (
                self.tab_bar_style.inactive_tab_left.clone(),
                self.tab_bar_style.inactive_tab_right.clone(),
            )
        }
    }

    fn tab_title_second_pass_max_width_with_new_tab_width(&self, new_tab_width: usize) -> usize {
        if !self.show_tabs_in_tab_bar {
            return 0;
        }

        let tab_count = self.app_shell.active_workspace().tabs().len();
        if tab_count == 0 {
            return self.tab_max_width;
        }

        let columns = usize::from(self.window_terminal_size_for_layout().columns);
        let left_prefix_width = usize::from(self.tab_bar_left_prefix_width().unwrap_or(0));
        let right_status_width = tab_bar_ansi_visible_width(&self.right_status);
        let right_integrated_title_buttons_width =
            if self.integrated_title_buttons_are_right_aligned() {
                self.integrated_title_buttons_tab_bar_width()
            } else {
                0
            };
        let fixed_tab_width = self
            .app_shell
            .active_workspace()
            .tabs()
            .iter()
            .enumerate()
            .map(|(position, tab)| {
                let label = tab_bar_tab_label_segments(
                    position,
                    tab.id(),
                    tab.panes().len(),
                    tab.id() == self.app_shell.active_tab_id(),
                    Some(""),
                    Self::tab_progress_for_tab(tab),
                    self.tab_bar_label_options(),
                );
                label.prefix.chars().count() + label.suffix.chars().count()
            })
            .sum::<usize>();

        let available_title_width = columns.saturating_sub(
            left_prefix_width
                .saturating_add(new_tab_width)
                .saturating_add(right_status_width)
                .saturating_add(right_integrated_title_buttons_width)
                .saturating_add(fixed_tab_width),
        );
        let per_tab_width = available_title_width / tab_count;
        self.tab_max_width
            .min(per_tab_width.max(self.tab_min_width))
    }

    fn tab_progress_for_tab(tab: &rterm_core::app_shell::Tab) -> PaneProgress {
        tab.panes()
            .iter()
            .find(|pane| pane.id() == tab.active_pane_id())
            .map_or(PaneProgress::None, rterm_core::app_shell::Pane::progress)
    }

    fn tab_bar_title_context(&self) -> TabBarTitleContext {
        TabBarTitleContext {
            config: self.native_effective_config(),
            tabs: self.native_window_tab_information(),
            active_pane_info: self.native_pane_information_for_tab(self.app_shell.active_tab()),
        }
    }

    fn formatted_tab_title_for_tab_first_pass_with_context(
        &self,
        position: usize,
        tab: &rterm_core::app_shell::Tab,
        context: &TabBarTitleContext,
    ) -> Option<NativeTabTitle> {
        let default_title = self.tab_bar_title_source_for_tab(tab);
        let tab_bar_default_title =
            self.tab_bar_default_title_for_tab(tab, default_title.as_deref());
        let tab_title = tab.title().map(str::to_owned);
        let tab_info = self.native_tab_information(position, tab, tab_title.clone());
        let title_format = |hover, max_width| NativeTabTitleFormat {
            default_title: default_title.clone(),
            tab: tab.id(),
            active_pane: tab.active_pane_id(),
            tab_index: position,
            tab_count: self.app_shell.active_workspace().tabs().len(),
            pane_count: context.active_pane_info.len(),
            is_active: tab.id() == self.app_shell.active_tab_id(),
            is_last_active: Some(tab.id()) == self.app_shell.last_active_tab_id(),
            hover,
            max_width,
            config: context.config.clone(),
            window_id: self.app_window_id,
            window_title: self.window_title.clone(),
            tab_title: tab_title.clone(),
            active_pane_info: tab_info.active_pane.clone(),
            tabs: context.tabs.clone(),
            panes: context.active_pane_info.clone(),
            tab_info: tab_info.clone(),
        };

        let first_pass = title_format(false, self.tab_max_width);
        let lua_tab_title = self
            .lua_tab_title
            .as_ref()
            .and_then(|title| title.resolve(&first_pass));

        (self.tab_title_formatter)(&first_pass)
            .or(lua_tab_title)
            .or_else(|| tab_bar_default_title.map(NativeTabTitle::Text))
    }

    fn formatted_tab_title_for_tab_with_max_width_and_context(
        &self,
        position: usize,
        tab: &rterm_core::app_shell::Tab,
        hover: bool,
        max_width: usize,
        context: &TabBarTitleContext,
    ) -> Option<NativeTabTitle> {
        let default_title = self.tab_bar_title_source_for_tab(tab);
        let tab_bar_default_title =
            self.tab_bar_default_title_for_tab(tab, default_title.as_deref());
        let tab_title = tab.title().map(str::to_owned);
        let tab_info = self.native_tab_information(position, tab, tab_title.clone());
        let title_format = |hover, max_width| NativeTabTitleFormat {
            default_title: default_title.clone(),
            tab: tab.id(),
            active_pane: tab.active_pane_id(),
            tab_index: position,
            tab_count: self.app_shell.active_workspace().tabs().len(),
            pane_count: context.active_pane_info.len(),
            is_active: tab.id() == self.app_shell.active_tab_id(),
            is_last_active: Some(tab.id()) == self.app_shell.last_active_tab_id(),
            hover,
            max_width,
            config: context.config.clone(),
            window_id: self.app_window_id,
            window_title: self.window_title.clone(),
            tab_title: tab_title.clone(),
            active_pane_info: tab_info.active_pane.clone(),
            tabs: context.tabs.clone(),
            panes: context.active_pane_info.clone(),
            tab_info: tab_info.clone(),
        };

        let second_pass = title_format(hover, max_width);
        let lua_tab_title = self
            .lua_tab_title
            .as_ref()
            .and_then(|title| title.resolve(&second_pass));

        (self.tab_title_formatter)(&second_pass)
            .or(lua_tab_title)
            .or_else(|| tab_bar_default_title.map(NativeTabTitle::Text))
    }

    fn tab_bar_title_source_for_tab(&self, tab: &rterm_core::app_shell::Tab) -> Option<String> {
        if let Some(title) = self.tab_title_for_tab(tab) {
            return Some(title);
        }
        if !self.modern_tab_bar_uses_compact_labels() {
            return None;
        }

        tab.panes()
            .iter()
            .find(|pane| pane.id() == tab.active_pane_id())
            .map(|pane| compact_terminal_tab_title(pane_launch_display_program(pane.launch())))
            .filter(|title| !title.is_empty())
    }

    fn tab_bar_default_title_for_tab(
        &self,
        tab: &rterm_core::app_shell::Tab,
        title: Option<&str>,
    ) -> Option<String> {
        let title = title?;
        if tab.title().is_some() || !self.modern_tab_bar_uses_compact_labels() {
            return Some(if tab.title().is_some() {
                title.to_owned()
            } else {
                compact_terminal_tab_title(title)
            });
        }

        // Keep the shell/program title in the tab.  The title bar itself has
        // no product-name region, so a default shell remains identifiable
        // without adding application branding to the chrome.
        let compact_title = compact_terminal_tab_title(title);
        Some(compact_title)
    }
}
