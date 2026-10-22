use std::io::{self, Write as _};

use rterm_terminal::{
    Cell, CellWidthOverride, Color, CursorShape, CursorStyle, Terminal, TerminalResizeOutcome,
    UnderlineStyle, VerticalAlign,
};
use rterm_types::{DamageRegion, TerminalSize};

use crate::{
    RuntimeProgress,
    delta::{RuntimeBuffers, RuntimeDelta, TerminalSnapshotRef},
    modes::{MouseInputMode, TerminalModeTracker, framed_control_may_change_modes},
    queries::{
        ClipboardCommand, DecrqcraRequest as SharedDecrqcraRequest, FixedQuery, KeyModifierOptions,
        KittyKeyboardMode, NotificationCommand, OscColorKind as SharedOscColorKind,
        OscColorRequest as SharedOscColorRequest, PrivateModeSequence, ProgressCommand,
        QueryScanStorageCounters, ScannedSegmentRef, SemanticControl, StringTerminator,
        TerminalQueryScanner, WindowReportRequest,
        XtSmGraphicsRequest as SharedXtSmGraphicsRequest,
    },
    query_dcs::{
        DcsTerminator, DecrqssKind as SharedDecrqssKind, DecrqssRequest as SharedDecrqssRequest,
        MAX_XTGETTCAP_RESPONSE_BYTES, XtGetTcapRequest as SharedXtGetTcapRequest,
    },
    visible_output::TerminalVisibleOutputFilter,
};

const DEFAULT_TERMINAL_NAME: &str = "xterm-256color";

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent compatibility flags represent valid combinations"
)]
pub struct TerminalRuntime {
    terminal: Terminal,
    output_filter: TerminalOutputFilter,
    visible_output_filter: TerminalVisibleOutputFilter,
    mode_tracker: TerminalModeTracker,
    enable_kitty_keyboard: bool,
    enable_checksum_rectangular_area: bool,
    enable_title_reporting: bool,
    enq_answerback: String,
    allow_win32_input_mode: bool,
    clipboard_texts: Vec<String>,
    clipboard_queries: Vec<String>,
    notifications: Vec<TerminalNotification>,
    progress: TerminalProgress,
    published_progress: TerminalProgress,
    metadata_source_entries_inspected: u64,
    capture_host_stream: bool,
    synchronized_console_output: Vec<u8>,
    #[cfg(test)]
    fixture_trace_id: u64,
    #[cfg(test)]
    fixture_trace_buffers: RuntimeBuffers,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalNotification {
    pub title: Option<String>,
    pub body: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TerminalProgress {
    #[default]
    None,
    Percentage(u8),
    Error(u8),
    Indeterminate,
}

pub struct TerminalRuntimeOutput {
    pub responses: Vec<Vec<u8>>,
    pub display: Vec<u8>,
    pub damage: Vec<DamageRegion>,
    pub bells: u64,
    pub unknown_escape_sequences: Vec<String>,
    pub screen_identity_changed: bool,
}

impl TerminalRuntime {
    #[must_use]
    pub fn new(size: TerminalSize) -> Self {
        let runtime = Self {
            terminal: Terminal::new(size),
            output_filter: TerminalOutputFilter::new(size),
            visible_output_filter: TerminalVisibleOutputFilter::default(),
            mode_tracker: TerminalModeTracker::default(),
            enable_kitty_keyboard: false,
            enable_checksum_rectangular_area: false,
            enable_title_reporting: false,
            enq_answerback: String::new(),
            allow_win32_input_mode: true,
            clipboard_texts: Vec::new(),
            clipboard_queries: Vec::new(),
            notifications: Vec::new(),
            progress: TerminalProgress::None,
            published_progress: TerminalProgress::None,
            metadata_source_entries_inspected: 0,
            capture_host_stream: false,
            synchronized_console_output: Vec::new(),
            #[cfg(test)]
            fixture_trace_id: 0,
            #[cfg(test)]
            fixture_trace_buffers: RuntimeBuffers::default(),
        };
        #[cfg(test)]
        let mut runtime = runtime;
        #[cfg(test)]
        {
            runtime.fixture_trace_id =
                terminal_transcript_tests::trace_runtime_construct(&runtime, size);
        }
        runtime
    }

    /// Creates a fresh parser/runtime owner around an existing presentation.
    ///
    /// This is used when a transport must be restarted under a new owner while
    /// preserving the pane's terminal grid and scrollback. Parser/filter state
    /// starts fresh because the replacement transport is a new session.
    #[must_use]
    pub fn from_terminal(terminal: Terminal) -> Self {
        let size = terminal.grid().size();
        let mut runtime = Self::new(size);
        runtime.terminal = terminal;
        runtime
    }

    #[must_use]
    pub fn new_with_query_scan_work(size: TerminalSize) -> Self {
        let mut runtime = Self::new(size);
        runtime.output_filter.query_scanner = TerminalQueryScanner::new_with_work_counter();
        runtime
    }

    pub fn set_terminal_name(&mut self, terminal_name: impl Into<String>) {
        self.output_filter.set_terminal_name(terminal_name);
    }

    /// Compatibility helper for tests and embedders that only need responses.
    ///
    /// # Panics
    ///
    /// Panics if formatting an internally generated response into its in-memory byte buffer
    /// reports an I/O error. The current `Vec<u8>` writer is infallible; allocation failure
    /// follows normal `Vec` behavior.
    pub fn feed_pty_output(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        self.feed_pty_output_with_display(bytes).responses
    }

    /// Progresses the terminal and returns an owned compatibility transcript.
    ///
    /// # Panics
    ///
    /// Panics if formatting an internally generated response into its in-memory byte buffer
    /// reports an I/O error. The current `Vec<u8>` writer is infallible; allocation failure
    /// follows normal `Vec` behavior.
    pub fn feed_pty_output_with_display(&mut self, bytes: &[u8]) -> TerminalRuntimeOutput {
        #[cfg(test)]
        if self.fixture_trace_id != 0 {
            return terminal_transcript_tests::trace_feed_into_legacy_projection(self, bytes);
        }
        let screen_identity_generation = self.terminal.screen_identity_generation();
        let output = self.output_filter.process(bytes);

        let mut responses = Vec::new();
        let mut display_bytes = Vec::new();
        let mut damage = Vec::new();
        let mut bells = 0_u64;
        let mut unknown_escape_sequences = Vec::new();
        for event in output.events {
            match event {
                FilteredOutputEvent::Display {
                    bytes: display,
                    all_lines_changed,
                    track_modes,
                    ..
                } => {
                    if track_modes {
                        self.mode_tracker.process_without_emitting(&display);
                    }
                    if all_lines_changed {
                        self.terminal.feed_with_all_lines_changed(&display);
                    } else {
                        self.terminal.feed(&display);
                    }
                    for sequence in self.terminal.take_unknown_escape_sequences() {
                        unknown_escape_sequences.push(sequence.sequence);
                    }
                    for response in self.terminal.take_kitty_graphics_responses() {
                        responses.push(response);
                    }
                    let display_bells = self.terminal.take_bell_count();
                    bells = bells.saturating_add(display_bells);
                    display_bytes.extend(self.visible_output_filter.process(&display));
                    if self.mode_tracker.synchronized_output() {
                        continue;
                    }
                    damage.extend(self.terminal.take_damage());
                }
                FilteredOutputEvent::Response(response) => {
                    if self.should_emit_response(&response) {
                        let response = self.output_filter.response_bytes(
                            response,
                            &self.terminal,
                            &self.mode_tracker,
                        );
                        responses.push(response);
                    }
                }
                FilteredOutputEvent::ResponseBytes(bytes) => {
                    responses.push(bytes);
                }
                FilteredOutputEvent::Enq => {
                    if !self.enq_answerback.is_empty() {
                        responses.push(self.enq_answerback.as_bytes().to_vec());
                    }
                }
                FilteredOutputEvent::SynchronizedOutputMode(sequence) => {
                    let enabled = sequence.enabled;
                    self.mode_tracker
                        .apply_private_mode_sequence(&sequence, |_| {});
                    if !enabled {
                        damage.extend(self.terminal.take_damage());
                    }
                }
                FilteredOutputEvent::KittyKeyboardMode(sequence) => {
                    if self.enable_kitty_keyboard {
                        self.mode_tracker
                            .apply_kitty_keyboard_sequence(sequence, |_| {});
                    }
                }
                FilteredOutputEvent::KeyModifierOptions(sequence) => {
                    self.mode_tracker
                        .apply_key_modifier_options_sequence(sequence, |_| {});
                }
                FilteredOutputEvent::Clipboard(command) => match command {
                    ClipboardCommand::Write { contents, .. } => {
                        self.clipboard_texts.push(contents);
                    }
                    ClipboardCommand::Query(selection) => {
                        self.clipboard_queries.push(selection);
                    }
                },
                FilteredOutputEvent::Notification(command) => {
                    self.apply_legacy_notification(command);
                }
            }
        }
        self.finish_legacy_metadata_boundary();

        TerminalRuntimeOutput {
            responses,
            display: display_bytes,
            damage,
            bells,
            unknown_escape_sequences,
            screen_identity_changed: self.terminal.screen_identity_generation()
                != screen_identity_generation,
        }
    }

    fn apply_legacy_notification(&mut self, command: NotificationCommand) {
        match command {
            NotificationCommand::Notify { title, body } => {
                self.notifications
                    .push(TerminalNotification { title, body });
            }
            NotificationCommand::Progress(progress) => {
                self.progress = terminal_progress_from_command(progress);
            }
            NotificationCommand::Ignored => {}
        }
    }

    fn finish_legacy_metadata_boundary(&mut self) {
        self.terminal.clear_pending_metadata_changes();
        self.published_progress = self.progress;
    }

    /// Progresses the terminal into reusable caller-owned storage.
    ///
    /// # Panics
    ///
    /// Panics if formatting an internally generated response into the in-memory byte arena
    /// reports an I/O error. The current `Vec<u8>` writer is infallible; allocation failure
    /// follows normal `Vec` behavior.
    pub fn feed_into<'buffers>(
        &mut self,
        bytes: &[u8],
        buffers: &'buffers mut RuntimeBuffers,
    ) -> RuntimeDelta<'buffers> {
        let capacities = buffers.begin_feed();
        let Self {
            terminal,
            output_filter,
            visible_output_filter,
            mode_tracker,
            enable_kitty_keyboard,
            enable_checksum_rectangular_area,
            enable_title_reporting,
            enq_answerback,
            progress: runtime_progress_state,
            published_progress,
            metadata_source_entries_inspected,
            capture_host_stream,
            synchronized_console_output,
            ..
        } = self;
        let screen_identity_generation = terminal.screen_identity_generation();
        let response_size = output_filter.size;
        let (bell_count, progress_candidate) = {
            let mut context = FeedIntoContext {
                terminal,
                visible_output_filter,
                mode_tracker,
                buffers,
                response_size,
                response_options: RuntimeResponseOptions {
                    enable_kitty_keyboard: *enable_kitty_keyboard,
                    enable_checksum_rectangular_area: *enable_checksum_rectangular_area,
                    enable_title_reporting: *enable_title_reporting,
                },
                enq_answerback,
                runtime_progress_state,
                capture_host_stream: *capture_host_stream,
                synchronized_console_output,
                bell_count: 0,
                progress_candidate: false,
            };
            output_filter.for_each_event(bytes, |event| context.feed_event_into(event));
            (context.bell_count, context.progress_candidate)
        };

        let metadata_changed = publish_pending_metadata(
            terminal,
            buffers,
            *runtime_progress_state,
            published_progress,
            metadata_source_entries_inspected,
            progress_candidate,
        );
        let screen_identity_changed =
            terminal.screen_identity_generation() != screen_identity_generation;
        let snapshot_changed = buffers.has_damage() || screen_identity_changed || metadata_changed;
        buffers.finish_feed(capacities);
        RuntimeDelta::new(
            buffers,
            bell_count,
            snapshot_changed,
            screen_identity_changed,
        )
    }

    /// Enables ordered console writes and mode changes in runtime effects.
    ///
    /// The capture is opt-in so renderer-only consumers do not copy the terminal display stream.
    pub fn set_capture_host_stream(&mut self, enabled: bool) {
        self.capture_host_stream = enabled;
        if !enabled {
            self.synchronized_console_output.clear();
        }
    }

    /// Discards an incomplete trailing control and releases synchronized console output.
    pub fn finish_into<'buffers>(
        &mut self,
        buffers: &'buffers mut RuntimeBuffers,
    ) -> RuntimeDelta<'buffers> {
        let capacities = buffers.begin_feed();
        self.output_filter.query_scanner.discard_incomplete();
        let capture_host_stream = self.capture_host_stream;
        self.mode_tracker.finish(|change| {
            if capture_host_stream {
                buffers.push_mode_change(change);
            }
        });
        if self.capture_host_stream {
            flush_synchronized_console_output(buffers, &mut self.synchronized_console_output);
        }
        self.terminal.drain_damage_into(buffers.damage_mut());
        let snapshot_changed = buffers.has_damage();
        buffers.finish_feed(capacities);
        RuntimeDelta::new(buffers, 0, snapshot_changed, false)
    }

    /// Returns the current renderer-independent terminal state.
    #[must_use]
    pub const fn snapshot(&self) -> TerminalSnapshotRef<'_> {
        TerminalSnapshotRef::new(&self.terminal)
    }

    fn should_emit_response(&self, response: &TerminalResponse) -> bool {
        match response {
            TerminalResponse::KittyKeyboardFlags => self.enable_kitty_keyboard,
            TerminalResponse::ChecksumRectangularArea(_) => self.enable_checksum_rectangular_area,
            TerminalResponse::WindowTitle => self.enable_title_reporting,
            _ => true,
        }
    }

    pub fn take_clipboard_texts(&mut self) -> Vec<String> {
        std::mem::take(&mut self.clipboard_texts)
    }

    pub fn take_clipboard_queries(&mut self) -> Vec<String> {
        std::mem::take(&mut self.clipboard_queries)
    }

    pub fn take_notifications(&mut self) -> Vec<TerminalNotification> {
        std::mem::take(&mut self.notifications)
    }

    #[must_use]
    pub const fn progress(&self) -> TerminalProgress {
        self.progress
    }

    pub fn resize(&mut self, size: TerminalSize) -> TerminalResizeOutcome {
        let outcome = self.terminal.resize(size);
        self.output_filter.resize(size);
        outcome
    }

    /// Resizes the terminal and drains its damage into caller-owned storage.
    pub fn resize_into<'buffers>(
        &mut self,
        size: TerminalSize,
        buffers: &'buffers mut RuntimeBuffers,
    ) -> (TerminalResizeOutcome, RuntimeDelta<'buffers>) {
        let capacities = buffers.begin_feed();
        let outcome = self.terminal.resize(size);
        self.output_filter.resize(size);
        self.terminal.drain_damage_into(buffers.damage_mut());
        let snapshot_changed = buffers.has_damage();
        buffers.finish_feed(capacities);
        (
            outcome,
            RuntimeDelta::new(buffers, 0, snapshot_changed, false),
        )
    }

    pub fn set_scrollback_limit(&mut self, limit: usize) {
        self.terminal.set_scrollback_limit(limit);
    }

    pub fn mark_all_lines_changed(&mut self) {
        self.terminal.mark_all_lines_changed();
    }

    pub fn set_default_cursor_style(&mut self, default_cursor_style: CursorStyle) {
        self.terminal.set_default_cursor_style(default_cursor_style);
    }

    #[must_use]
    pub fn cursor_color_override(&self) -> Option<Color> {
        self.output_filter.cursor_color_override()
    }

    pub fn erase_scrollback_and_viewport(&mut self) -> Vec<DamageRegion> {
        self.terminal.erase_scrollback_and_viewport();
        self.terminal.take_damage()
    }

    #[must_use]
    pub fn terminal(&self) -> &Terminal {
        &self.terminal
    }

    /// Applies a worker-owned mode transition to a presentation mirror.
    pub fn install_presentation_mode_change(&mut self, change: crate::TerminalModeChange) {
        self.mode_tracker.install_change(change);
    }

    /// Cumulative bytes inspected by the streaming query scanner.
    ///
    /// The counter saturates at `u64::MAX` and is disabled for normal runtimes.
    #[must_use]
    pub const fn inspected_query_bytes(&self) -> u64 {
        self.output_filter.query_scanner.inspected_bytes()
    }

    #[must_use]
    pub const fn query_scan_storage_counters(&self) -> QueryScanStorageCounters {
        self.output_filter.query_scanner.storage_counters()
    }

    pub fn reset_query_scan_storage_counters(&mut self) {
        self.output_filter.query_scanner.reset_storage_counters();
    }

    #[must_use]
    pub const fn metadata_source_entries_inspected(&self) -> u64 {
        self.metadata_source_entries_inspected
    }

    #[must_use]
    pub fn application_cursor_keys(&self) -> bool {
        self.mode_tracker.application_cursor_keys()
    }

    #[must_use]
    pub fn focus_reporting(&self) -> bool {
        self.mode_tracker.focus_reporting()
    }

    #[must_use]
    pub fn bracketed_paste(&self) -> bool {
        self.mode_tracker.bracketed_paste()
    }

    #[cfg(test)]
    #[must_use]
    pub fn synchronized_output(&self) -> bool {
        self.mode_tracker.synchronized_output()
    }

    #[must_use]
    pub fn application_keypad(&self) -> bool {
        self.mode_tracker.application_keypad()
    }

    #[must_use]
    pub fn kitty_keyboard_flags(&self) -> u16 {
        if self.enable_kitty_keyboard {
            self.mode_tracker.kitty_keyboard_flags()
        } else {
            0
        }
    }

    #[must_use]
    pub fn modify_other_keys(&self) -> u8 {
        self.mode_tracker.modify_other_keys()
    }

    #[must_use]
    pub fn win32_input_mode(&self) -> bool {
        self.allow_win32_input_mode && self.mode_tracker.win32_input_mode()
    }

    #[must_use]
    pub fn mouse_input_mode(&self) -> MouseInputMode {
        self.mode_tracker.mouse_input_mode()
    }

    pub fn set_enable_kitty_keyboard(&mut self, enabled: bool) {
        self.enable_kitty_keyboard = enabled;
        if !enabled {
            self.mode_tracker.clear_kitty_keyboard_flags();
        }
    }

    pub fn set_enable_kitty_graphics(&mut self, enabled: bool) {
        self.terminal.set_enable_kitty_graphics(enabled);
    }

    pub fn set_enable_checksum_rectangular_area(&mut self, enabled: bool) {
        self.enable_checksum_rectangular_area = enabled;
    }

    pub fn set_enable_title_reporting(&mut self, enabled: bool) {
        self.enable_title_reporting = enabled;
    }

    pub fn set_enq_answerback(&mut self, answerback: impl Into<String>) {
        self.enq_answerback = answerback.into();
    }

    pub fn set_allow_win32_input_mode(&mut self, allowed: bool) {
        self.allow_win32_input_mode = allowed;
        self.mode_tracker.set_allow_win32_input_mode(allowed);
    }

    pub fn set_treat_east_asian_ambiguous_width_as_wide(&mut self, enabled: bool) {
        self.terminal
            .set_treat_east_asian_ambiguous_width_as_wide(enabled);
    }

    pub fn set_normalize_output_to_unicode_nfc(&mut self, enabled: bool) {
        self.terminal.set_normalize_output_to_unicode_nfc(enabled);
    }

    pub fn set_unicode_version(&mut self, version: u32) {
        self.terminal.set_unicode_version(version);
    }

    pub fn set_cell_width_overrides(&mut self, overrides: Vec<CellWidthOverride>) {
        self.terminal.set_cell_width_overrides(overrides);
    }
}

#[cfg(test)]
impl Drop for TerminalRuntime {
    fn drop(&mut self) {
        terminal_transcript_tests::trace_runtime_drop(self);
    }
}

const fn runtime_progress(progress: TerminalProgress) -> RuntimeProgress {
    match progress {
        TerminalProgress::None => RuntimeProgress::None,
        TerminalProgress::Percentage(value) => RuntimeProgress::Percentage(value),
        TerminalProgress::Error(value) => RuntimeProgress::Error(value),
        TerminalProgress::Indeterminate => RuntimeProgress::Indeterminate,
    }
}

const fn terminal_progress_from_command(progress: ProgressCommand) -> TerminalProgress {
    match progress {
        ProgressCommand::None => TerminalProgress::None,
        ProgressCommand::Percentage(value) => TerminalProgress::Percentage(value),
        ProgressCommand::Error(value) => TerminalProgress::Error(value),
        ProgressCommand::Indeterminate => TerminalProgress::Indeterminate,
    }
}

const fn should_emit_runtime_response(
    response: &TerminalResponse,
    enable_kitty_keyboard: bool,
    enable_checksum_rectangular_area: bool,
    enable_title_reporting: bool,
) -> bool {
    match response {
        TerminalResponse::KittyKeyboardFlags => enable_kitty_keyboard,
        TerminalResponse::ChecksumRectangularArea(_) => enable_checksum_rectangular_area,
        TerminalResponse::WindowTitle => enable_title_reporting,
        _ => true,
    }
}

struct FeedIntoContext<'runtime> {
    terminal: &'runtime mut Terminal,
    visible_output_filter: &'runtime mut TerminalVisibleOutputFilter,
    mode_tracker: &'runtime mut TerminalModeTracker,
    buffers: &'runtime mut RuntimeBuffers,
    response_size: TerminalSize,
    response_options: RuntimeResponseOptions,
    enq_answerback: &'runtime str,
    runtime_progress_state: &'runtime mut TerminalProgress,
    capture_host_stream: bool,
    synchronized_console_output: &'runtime mut Vec<u8>,
    bell_count: u64,
    progress_candidate: bool,
}

#[derive(Clone, Copy)]
struct RuntimeResponseOptions {
    enable_kitty_keyboard: bool,
    enable_checksum_rectangular_area: bool,
    enable_title_reporting: bool,
}

impl FeedIntoContext<'_> {
    fn feed_event_into(&mut self, event: FilteredOutputEventRef<'_>) {
        match event {
            FilteredOutputEventRef::Display {
                bytes,
                all_lines_changed,
                track_modes,
                console_write,
            } => self.feed_display_into(
                bytes,
                all_lines_changed,
                track_modes,
                self.capture_host_stream && console_write,
            ),
            FilteredOutputEventRef::Response(response) => {
                if should_emit_runtime_response(
                    &response,
                    self.response_options.enable_kitty_keyboard,
                    self.response_options.enable_checksum_rectangular_area,
                    self.response_options.enable_title_reporting,
                ) {
                    emit_response_into(
                        self.buffers,
                        response,
                        self.response_size,
                        self.terminal,
                        self.mode_tracker,
                    );
                }
            }
            FilteredOutputEventRef::OscColorResponse {
                color_state,
                response,
            } => emit_osc_color_response_into(self.buffers, color_state, response),
            FilteredOutputEventRef::Enq => self.feed_enq(),
            FilteredOutputEventRef::SynchronizedOutputMode(sequence) => {
                self.feed_synchronized_output_mode(&sequence);
            }
            FilteredOutputEventRef::KittyKeyboardMode(sequence) => {
                if self.response_options.enable_kitty_keyboard {
                    if self.capture_host_stream {
                        self.mode_tracker
                            .apply_kitty_keyboard_sequence(sequence, |change| {
                                self.buffers.push_mode_change(change);
                            });
                    } else {
                        self.mode_tracker
                            .apply_kitty_keyboard_sequence(sequence, |_| {});
                    }
                }
            }
            FilteredOutputEventRef::KeyModifierOptions(sequence) => {
                if self.capture_host_stream {
                    self.mode_tracker
                        .apply_key_modifier_options_sequence(sequence, |change| {
                            self.buffers.push_mode_change(change);
                        });
                } else {
                    self.mode_tracker
                        .apply_key_modifier_options_sequence(sequence, |_| {});
                }
            }
            FilteredOutputEventRef::Clipboard(command) => {
                apply_clipboard_into(command, self.buffers);
            }
            FilteredOutputEventRef::Notification(command) => {
                self.progress_candidate |=
                    apply_notification_into(command, self.buffers, self.runtime_progress_state);
            }
        }
    }

    #[inline]
    fn feed_display_into(
        &mut self,
        display: &[u8],
        all_lines_changed: bool,
        track_modes: bool,
        capture_console: bool,
    ) {
        let was_synchronized = self.mode_tracker.synchronized_output();
        if capture_console {
            capture_console_bytes(
                self.buffers,
                self.synchronized_console_output,
                display,
                was_synchronized,
            );
        }
        if track_modes {
            if capture_console {
                self.mode_tracker
                    .process(display, |change| self.buffers.push_mode_change(change));
            } else {
                self.mode_tracker.process_without_emitting(display);
            }
        }
        if capture_console && was_synchronized && !self.mode_tracker.synchronized_output() {
            flush_synchronized_console_output(self.buffers, self.synchronized_console_output);
        }
        if all_lines_changed {
            self.terminal.feed_with_all_lines_changed(display);
        } else {
            self.terminal.feed(display);
        }
        for sequence in self.terminal.take_unknown_escape_sequences() {
            self.buffers.push_diagnostic(&sequence.sequence);
        }
        for response in self.terminal.take_kitty_graphics_responses() {
            self.buffers.push_transport_write(&response);
        }
        let display_bells = self.terminal.take_bell_count();
        self.bell_count = self.bell_count.saturating_add(display_bells);
        self.buffers.push_bell(display_bells);
        self.visible_output_filter
            .process_into(display, self.buffers.visible_mut());
        if !self.mode_tracker.synchronized_output() {
            self.terminal.drain_damage_into(self.buffers.damage_mut());
        }
    }

    fn feed_enq(&mut self) {
        if self.capture_host_stream {
            capture_console_bytes(
                self.buffers,
                self.synchronized_console_output,
                b"\x05",
                self.mode_tracker.synchronized_output(),
            );
        }
        if !self.enq_answerback.is_empty() {
            self.buffers
                .push_transport_write(self.enq_answerback.as_bytes());
        }
    }

    fn feed_synchronized_output_mode(&mut self, sequence: &PrivateModeSequence) {
        let enabled = sequence.enabled;
        apply_private_mode_into(
            self.mode_tracker,
            sequence,
            self.buffers,
            self.capture_host_stream,
        );
        if !enabled {
            if self.capture_host_stream {
                flush_synchronized_console_output(self.buffers, self.synchronized_console_output);
            }
            self.terminal.drain_damage_into(self.buffers.damage_mut());
        }
    }
}

fn capture_console_bytes(
    buffers: &mut RuntimeBuffers,
    synchronized_console_output: &mut Vec<u8>,
    bytes: &[u8],
    synchronized: bool,
) {
    if synchronized {
        synchronized_console_output.extend_from_slice(bytes);
    } else {
        buffers.push_console_write(bytes);
    }
}

fn flush_synchronized_console_output(
    buffers: &mut RuntimeBuffers,
    synchronized_console_output: &mut Vec<u8>,
) {
    buffers.push_console_write(synchronized_console_output);
    synchronized_console_output.clear();
}

fn apply_private_mode_into(
    mode_tracker: &mut TerminalModeTracker,
    sequence: &PrivateModeSequence,
    buffers: &mut RuntimeBuffers,
    capture_host_stream: bool,
) {
    if capture_host_stream {
        mode_tracker.apply_private_mode_sequence(sequence, |change| {
            buffers.push_mode_change(change);
        });
    } else {
        mode_tracker.apply_private_mode_sequence(sequence, |_| {});
    }
}

#[inline]
fn emit_response_into(
    buffers: &mut RuntimeBuffers,
    response: TerminalResponse,
    size: TerminalSize,
    terminal: &Terminal,
    mode_tracker: &TerminalModeTracker,
) {
    buffers
        .try_push_transport_write_with(|arena| {
            response.write_into(size, terminal, mode_tracker, arena)
        })
        .expect("writing to a byte arena cannot fail");
}

#[inline]
fn emit_osc_color_response_into(
    buffers: &mut RuntimeBuffers,
    color_state: &TerminalColorState,
    response: OscColorResponse,
) {
    buffers
        .try_push_transport_write_with(|arena| color_state.write_response(response, arena))
        .expect("writing to a byte arena cannot fail");
}

fn apply_notification_into(
    command: NotificationCommand,
    buffers: &mut RuntimeBuffers,
    runtime_progress_state: &mut TerminalProgress,
) -> bool {
    match command {
        NotificationCommand::Notify { title, body } => {
            buffers.push_notification(title.as_deref(), &body);
            false
        }
        NotificationCommand::Progress(progress) => {
            *runtime_progress_state = terminal_progress_from_command(progress);
            true
        }
        NotificationCommand::Ignored => false,
    }
}

fn apply_clipboard_into(command: ClipboardCommand, buffers: &mut RuntimeBuffers) {
    match command {
        ClipboardCommand::Write {
            selection,
            contents,
        } => buffers.push_clipboard_write(selection.as_deref(), &contents),
        ClipboardCommand::Query(selection) => buffers.push_clipboard_read(&selection),
    }
}

fn publish_pending_metadata(
    terminal: &mut Terminal,
    buffers: &mut RuntimeBuffers,
    runtime_progress_state: TerminalProgress,
    published_progress: &mut TerminalProgress,
    metadata_source_entries_inspected: &mut u64,
    progress_candidate: bool,
) -> bool {
    let mut metadata_changed = false;
    let changes = terminal.pending_metadata_changes();
    if changes.title() {
        buffers.set_title(terminal.title());
        metadata_changed = true;
    }
    if changes.current_working_dir() {
        buffers.set_working_directory(terminal.current_working_dir());
        metadata_changed = true;
    }
    if changes.badge_format() {
        buffers.set_badge_format(terminal.badge_format());
        metadata_changed = true;
    }
    for name in changes.user_vars() {
        *metadata_source_entries_inspected = metadata_source_entries_inspected.saturating_add(1);
        let current = terminal.user_vars().get(name);
        buffers.push_user_var(name, current.map(String::as_str));
        metadata_changed = true;
    }
    terminal.clear_pending_metadata_changes();
    if progress_candidate && *published_progress != runtime_progress_state {
        buffers.set_progress(runtime_progress(runtime_progress_state));
        *published_progress = runtime_progress_state;
        metadata_changed = true;
    }
    metadata_changed
}

struct TerminalOutputFilter {
    query_scanner: TerminalQueryScanner,
    size: TerminalSize,
    terminal_name: String,
    color_state: TerminalColorState,
    #[cfg(test)]
    fixture_trace_id: u64,
}

struct FilteredOutput {
    events: Vec<FilteredOutputEvent>,
}

enum FilteredOutputEvent {
    Display {
        bytes: Vec<u8>,
        all_lines_changed: bool,
        track_modes: bool,
    },
    Response(TerminalResponse),
    ResponseBytes(Vec<u8>),
    Enq,
    SynchronizedOutputMode(PrivateModeSequence),
    KittyKeyboardMode(KittyKeyboardMode),
    KeyModifierOptions(KeyModifierOptions),
    Clipboard(ClipboardCommand),
    Notification(NotificationCommand),
}

enum FilteredOutputEventRef<'a> {
    Display {
        bytes: &'a [u8],
        all_lines_changed: bool,
        track_modes: bool,
        console_write: bool,
    },
    Response(TerminalResponse),
    OscColorResponse {
        color_state: &'a TerminalColorState,
        response: OscColorResponse,
    },
    Enq,
    SynchronizedOutputMode(PrivateModeSequence),
    KittyKeyboardMode(KittyKeyboardMode),
    KeyModifierOptions(KeyModifierOptions),
    Clipboard(ClipboardCommand),
    Notification(NotificationCommand),
}

impl FilteredOutputEventRef<'_> {
    fn into_owned(self) -> FilteredOutputEvent {
        match self {
            Self::Display {
                bytes,
                all_lines_changed,
                track_modes,
                ..
            } => FilteredOutputEvent::Display {
                bytes: bytes.to_vec(),
                all_lines_changed,
                track_modes,
            },
            Self::Response(response) => FilteredOutputEvent::Response(response),
            Self::OscColorResponse {
                color_state,
                response,
            } => FilteredOutputEvent::ResponseBytes(color_state.response(response)),
            Self::Enq => FilteredOutputEvent::Enq,
            Self::SynchronizedOutputMode(sequence) => {
                FilteredOutputEvent::SynchronizedOutputMode(sequence)
            }
            Self::KittyKeyboardMode(sequence) => FilteredOutputEvent::KittyKeyboardMode(sequence),
            Self::KeyModifierOptions(sequence) => FilteredOutputEvent::KeyModifierOptions(sequence),
            Self::Clipboard(command) => FilteredOutputEvent::Clipboard(command),
            Self::Notification(command) => FilteredOutputEvent::Notification(command),
        }
    }
}

impl TerminalOutputFilter {
    const CELL_HEIGHT_PIXELS: u16 = 16;
    const CELL_WIDTH_PIXELS: u16 = 8;
    const PRIMARY_DEVICE_ATTRIBUTES: &'static [u8] = b"\x1b[?65;4;6;18;22;52c";
    const SECONDARY_DEVICE_ATTRIBUTES: &'static [u8] = b"\x1b[>1;277;0c";
    const TERTIARY_DEVICE_ATTRIBUTES: &'static [u8] = b"\x1bP!|00000000\x1b\\";
    const TERMINAL_PARAMETERS_0: &'static [u8] = b"\x1b[2;1;1;128;128;1;0x";
    const TERMINAL_PARAMETERS_1: &'static [u8] = b"\x1b[3;1;1;128;128;1;0x";
    fn new(size: TerminalSize) -> Self {
        let filter = Self {
            query_scanner: TerminalQueryScanner::new(),
            size,
            terminal_name: DEFAULT_TERMINAL_NAME.to_owned(),
            color_state: TerminalColorState::default(),
            #[cfg(test)]
            fixture_trace_id: 0,
        };
        #[cfg(test)]
        let mut filter = filter;
        #[cfg(test)]
        {
            filter.fixture_trace_id =
                terminal_transcript_tests::trace_filter_construct(&filter, size);
        }
        filter
    }

    fn set_terminal_name(&mut self, terminal_name: impl Into<String>) {
        self.terminal_name = terminal_name.into();
    }

    fn resize(&mut self, size: TerminalSize) {
        self.size = size;
    }

    fn process(&mut self, bytes: &[u8]) -> FilteredOutput {
        #[cfg(test)]
        let pre_state = (self.fixture_trace_id != 0)
            .then(|| terminal_transcript_tests::trace_filter_process_state(self));
        let mut events = Vec::new();
        self.for_each_event(bytes, |event| events.push(event.into_owned()));
        let output = FilteredOutput { events };
        #[cfg(test)]
        if let Some(pre_state) = pre_state {
            terminal_transcript_tests::trace_filter_process(self, bytes, &output, &pre_state);
        }
        output
    }

    fn for_each_event<F>(&mut self, bytes: &[u8], mut callback: F)
    where
        F: for<'event> FnMut(FilteredOutputEventRef<'event>),
    {
        #[cfg(test)]
        let fixture_trace_id = self.fixture_trace_id;
        let Self {
            query_scanner,
            size,
            terminal_name,
            color_state,
            #[cfg(test)]
                fixture_trace_id: _,
        } = self;
        let size = *size;
        let mut callback = |event: FilteredOutputEventRef<'_>| {
            #[cfg(test)]
            terminal_transcript_tests::trace_filter_event(fixture_trace_id, &event);
            callback(event);
        };
        query_scanner.for_each_segment(bytes, |segment| match segment {
            ScannedSegmentRef::Bytes(display) => {
                Self::emit_display(&mut callback, display, false, false, true);
            }
            ScannedSegmentRef::Control {
                bytes, semantic, ..
            } => Self::emit_control_event(
                &mut callback,
                bytes,
                semantic,
                size,
                terminal_name,
                color_state,
            ),
        });
    }

    fn emit_control_event<F>(
        callback: &mut F,
        bytes: &[u8],
        semantic: SemanticControl,
        size: TerminalSize,
        terminal_name: &str,
        color_state: &mut TerminalColorState,
    ) where
        F: for<'event> FnMut(FilteredOutputEventRef<'event>),
    {
        if let SemanticControl::OscColor(query) = &semantic {
            callback(FilteredOutputEventRef::OscColorResponse {
                color_state,
                response: Self::osc_color_response(query.clone()),
            });
            return;
        }
        if let Some(response) = Self::response_for_control(&semantic, size, terminal_name) {
            callback(FilteredOutputEventRef::Response(response));
            return;
        }
        Self::emit_unanswered_control(callback, bytes, semantic, color_state);
    }

    fn response_for_control(
        semantic: &SemanticControl,
        size: TerminalSize,
        terminal_name: &str,
    ) -> Option<TerminalResponse> {
        match semantic {
            SemanticControl::Fixed(query) => Some(Self::fixed_response(*query)),
            SemanticControl::WindowReport(query) => Self::window_response(*query),
            SemanticControl::PrivateModeStatus(mode) => {
                Some(TerminalResponse::PrivateModeStatus(*mode))
            }
            SemanticControl::AnsiModeStatus(mode) => Some(TerminalResponse::AnsiModeStatus(*mode)),
            SemanticControl::ItermReportCellSize => Some(TerminalResponse::ItermReportCellSize),
            SemanticControl::Decrqcra(request) => Some(TerminalResponse::ChecksumRectangularArea(
                Self::decrqcra_request(*request),
            )),
            SemanticControl::Decrqss(request) => {
                Some(TerminalResponse::Decrqss(Self::decrqss_response(*request)))
            }
            SemanticControl::XtGetTcap(request) => Some(TerminalResponse::XtGetTcap(
                Self::xtgettcap_response(size, terminal_name, request),
            )),
            SemanticControl::XtSmGraphics(request) => Some(TerminalResponse::XtSmGraphics(
                Self::xtsmgraphics_request(*request),
            )),
            SemanticControl::KittyKeyboardFlags => Some(TerminalResponse::KittyKeyboardFlags),
            SemanticControl::KeyModifierOptionsQuery(resource) => {
                Some(TerminalResponse::KeyModifierOptions(*resource))
            }
            _ => None,
        }
    }

    fn emit_unanswered_control<F>(
        callback: &mut F,
        bytes: &[u8],
        semantic: SemanticControl,
        color_state: &mut TerminalColorState,
    ) where
        F: for<'event> FnMut(FilteredOutputEventRef<'event>),
    {
        match semantic {
            SemanticControl::Enq => callback(FilteredOutputEventRef::Enq),
            SemanticControl::SynchronizedOutputMode(sequence) => {
                callback(FilteredOutputEventRef::SynchronizedOutputMode(sequence));
            }
            SemanticControl::KittyKeyboardMode(sequence) => {
                callback(FilteredOutputEventRef::KittyKeyboardMode(sequence));
            }
            SemanticControl::KeyModifierOptionsSequence(sequence) => {
                callback(FilteredOutputEventRef::KeyModifierOptions(sequence));
            }
            SemanticControl::Osc52(command) => {
                callback(FilteredOutputEventRef::Clipboard(command));
            }
            SemanticControl::Notification(command) => {
                callback(FilteredOutputEventRef::Notification(command));
            }
            SemanticControl::OscColor(_)
            | SemanticControl::StandaloneSt
            | SemanticControl::Cancelled
            | SemanticControl::Ignored
            | SemanticControl::WindowReport(_) => {}
            SemanticControl::Osc8Hyperlink | SemanticControl::DeviceAttributesResponse => {
                Self::emit_display_with_color_state(callback, bytes, color_state, false);
            }
            _ => Self::emit_display_with_color_state(callback, bytes, color_state, true),
        }
    }

    fn emit_display_with_color_state<F>(
        callback: &mut F,
        bytes: &[u8],
        color_state: &mut TerminalColorState,
        console_write: bool,
    ) where
        F: for<'event> FnMut(FilteredOutputEventRef<'event>),
    {
        let all_lines_changed = color_state.process_control(bytes);
        Self::emit_display(
            callback,
            bytes,
            all_lines_changed,
            framed_control_may_change_modes(bytes),
            console_write,
        );
    }

    fn emit_display<'display, F>(
        callback: &mut F,
        display: &'display [u8],
        all_lines_changed: bool,
        track_modes: bool,
        console_write: bool,
    ) where
        F: for<'event> FnMut(FilteredOutputEventRef<'event>),
    {
        if display.is_empty() {
            return;
        }
        callback(FilteredOutputEventRef::Display {
            bytes: display,
            all_lines_changed,
            track_modes,
            console_write,
        });
    }

    fn fixed_response(query: FixedQuery) -> TerminalResponse {
        match query {
            FixedQuery::CursorPosition => TerminalResponse::CursorPosition { private: false },
            FixedQuery::PrimaryDeviceAttributes => {
                TerminalResponse::Static(Self::PRIMARY_DEVICE_ATTRIBUTES)
            }
            FixedQuery::SecondaryDeviceAttributes => {
                TerminalResponse::Static(Self::SECONDARY_DEVICE_ATTRIBUTES)
            }
            FixedQuery::TertiaryDeviceAttributes => {
                TerminalResponse::Static(Self::TERTIARY_DEVICE_ATTRIBUTES)
            }
            FixedQuery::TerminalParameters0 => {
                TerminalResponse::Static(Self::TERMINAL_PARAMETERS_0)
            }
            FixedQuery::TerminalParameters1 => {
                TerminalResponse::Static(Self::TERMINAL_PARAMETERS_1)
            }
            FixedQuery::XtVersion => TerminalResponse::XtVersion,
            FixedQuery::OperatingStatus => TerminalResponse::Static(b"\x1b[0n"),
            FixedQuery::WindowPixelSize => TerminalResponse::WindowPixelSize,
            FixedQuery::CharacterCellSize => TerminalResponse::CharacterCellSize,
            FixedQuery::TextAreaSize => TerminalResponse::TextAreaSize,
        }
    }

    fn window_response(query: WindowReportRequest) -> Option<TerminalResponse> {
        match query {
            WindowReportRequest::WindowPixelSize => Some(TerminalResponse::WindowPixelSize),
            WindowReportRequest::CharacterCellSize => Some(TerminalResponse::CharacterCellSize),
            WindowReportRequest::TextAreaSize => Some(TerminalResponse::TextAreaSize),
            WindowReportRequest::WindowTitle => Some(TerminalResponse::WindowTitle),
            WindowReportRequest::Ignored => None,
        }
    }

    fn osc_color_response(query: SharedOscColorRequest) -> OscColorResponse {
        OscColorResponse {
            kinds: query
                .kinds
                .into_iter()
                .map(|kind| match kind {
                    SharedOscColorKind::DefaultForeground => OscColorKind::DefaultForeground,
                    SharedOscColorKind::DefaultBackground => OscColorKind::DefaultBackground,
                    SharedOscColorKind::Cursor => OscColorKind::Cursor,
                    SharedOscColorKind::Palette(index) => OscColorKind::Palette(index),
                })
                .collect(),
            terminator: match query.terminator {
                StringTerminator::Bel => OscResponseTerminator::Bel,
                StringTerminator::St => OscResponseTerminator::St,
                StringTerminator::C1St => OscResponseTerminator::C1St,
            },
        }
    }

    fn decrqcra_request(request: SharedDecrqcraRequest) -> DecrqcraRequest {
        DecrqcraRequest {
            request_id: request.request_id,
            top: request.top,
            left: request.left,
            bottom: request.bottom,
            right: request.right,
        }
    }

    fn xtsmgraphics_request(request: SharedXtSmGraphicsRequest) -> XtSmGraphicsRequest {
        XtSmGraphicsRequest {
            item: request.item,
            action: request.action,
        }
    }

    fn decrqss_response(request: SharedDecrqssRequest) -> DecrqssResponse {
        DecrqssResponse {
            kind: match request.kind {
                SharedDecrqssKind::Sgr => Some(DecrqssKind::Sgr),
                SharedDecrqssKind::CursorShape => Some(DecrqssKind::CursorShape),
                SharedDecrqssKind::ScrollRegion => Some(DecrqssKind::ScrollRegion),
                SharedDecrqssKind::ConformanceLevel => Some(DecrqssKind::ConformanceLevel),
                SharedDecrqssKind::LeftRightMargins => Some(DecrqssKind::LeftRightMargins),
                SharedDecrqssKind::Unknown => None,
            },
            terminator: match request.terminator {
                DcsTerminator::SevenBit => OscResponseTerminator::St,
                DcsTerminator::EightBit => OscResponseTerminator::C1St,
            },
        }
    }

    fn xtgettcap_response(
        size: TerminalSize,
        terminal_name: &str,
        request: &SharedXtGetTcapRequest,
    ) -> XtGetTcapResponse {
        let entries = request
            .names
            .iter()
            .map(|requested| {
                let name = requested.decoded.as_deref().unwrap_or(&requested.encoded);
                let name = String::from_utf8_lossy(name).into_owned().into_bytes();
                XtGetTcapEntry {
                    name_hex: encode_ascii_hex(&name),
                    value_hex: xtgettcap_value_hex(&name, size, terminal_name),
                }
            })
            .collect();
        XtGetTcapResponse { entries }
    }

    fn response_bytes(
        &self,
        response: TerminalResponse,
        terminal: &Terminal,
        modes: &TerminalModeTracker,
    ) -> Vec<u8> {
        response.response_bytes(self.size, terminal, modes)
    }

    fn cursor_color_override(&self) -> Option<Color> {
        self.color_state
            .cursor_override()
            .map(DynamicColor::to_color)
    }
}

#[cfg(test)]
impl Drop for TerminalOutputFilter {
    fn drop(&mut self) {
        terminal_transcript_tests::trace_filter_drop(self);
    }
}

#[derive(Clone)]
enum TerminalResponse {
    Static(&'static [u8]),
    CursorPosition { private: bool },
    WindowPixelSize,
    CharacterCellSize,
    TextAreaSize,
    WindowTitle,
    PrivateModeStatus(u16),
    AnsiModeStatus(u16),
    ItermReportCellSize,
    ChecksumRectangularArea(DecrqcraRequest),
    Decrqss(DecrqssResponse),
    XtGetTcap(XtGetTcapResponse),
    XtSmGraphics(XtSmGraphicsRequest),
    XtVersion,
    KittyKeyboardFlags,
    KeyModifierOptions(u16),
}

impl TerminalResponse {
    fn response_bytes(
        self,
        size: TerminalSize,
        terminal: &Terminal,
        modes: &TerminalModeTracker,
    ) -> Vec<u8> {
        let mut response = Vec::new();
        self.write_into(size, terminal, modes, &mut response)
            .expect("writing to a byte arena cannot fail");
        response
    }

    fn write_into(
        self,
        size: TerminalSize,
        terminal: &Terminal,
        modes: &TerminalModeTracker,
        response: &mut Vec<u8>,
    ) -> io::Result<()> {
        match self {
            TerminalResponse::Static(bytes) => response.extend_from_slice(bytes),
            TerminalResponse::CursorPosition { private } => {
                let (row, column) = terminal.cursor();
                if private {
                    write!(
                        response,
                        "\x1b[?{};{}R",
                        row.saturating_add(1),
                        column.saturating_add(1)
                    )?;
                } else {
                    write!(
                        response,
                        "\x1b[{};{}R",
                        row.saturating_add(1),
                        column.saturating_add(1)
                    )?;
                }
            }
            TerminalResponse::WindowPixelSize => write!(
                response,
                "\x1b[4;{};{}t",
                u32::from(size.rows) * u32::from(TerminalOutputFilter::CELL_HEIGHT_PIXELS),
                u32::from(size.columns) * u32::from(TerminalOutputFilter::CELL_WIDTH_PIXELS)
            )?,
            TerminalResponse::CharacterCellSize => write!(
                response,
                "\x1b[6;{};{}t",
                TerminalOutputFilter::CELL_HEIGHT_PIXELS,
                TerminalOutputFilter::CELL_WIDTH_PIXELS
            )?,
            TerminalResponse::TextAreaSize => {
                write!(response, "\x1b[8;{};{}t", size.rows, size.columns)?;
            }
            TerminalResponse::WindowTitle => {
                let title = terminal
                    .window_title()
                    .or_else(|| terminal.title())
                    .unwrap_or("");
                write!(response, "\x1b]l{title}\x1b\\")?;
            }
            TerminalResponse::PrivateModeStatus(mode) => {
                write!(
                    response,
                    "\x1b[?{};{}$y",
                    mode,
                    modes.private_mode_report_value(mode)
                )?;
            }
            TerminalResponse::AnsiModeStatus(mode) => {
                write!(
                    response,
                    "\x1b[{};{}$y",
                    mode,
                    modes.ansi_mode_report_value(mode)
                )?;
            }
            TerminalResponse::ItermReportCellSize => write!(
                response,
                "\x1b]1337;ReportCellSize={:.1};{:.1}\x1b\\",
                f32::from(TerminalOutputFilter::CELL_HEIGHT_PIXELS),
                f32::from(TerminalOutputFilter::CELL_WIDTH_PIXELS)
            )?,
            TerminalResponse::ChecksumRectangularArea(request) => {
                let checksum = terminal.checksum_rectangle(
                    request.left,
                    request.top,
                    request.right,
                    request.bottom,
                );
                write!(
                    response,
                    "\x1bP{}!~{:04x}\x1b\\",
                    request.request_id, checksum
                )?;
            }
            TerminalResponse::Decrqss(query) => query.write_into(terminal, response),
            TerminalResponse::XtGetTcap(query) => query.write_into(response),
            TerminalResponse::XtSmGraphics(request) => request.write_into(size, response)?,
            TerminalResponse::XtVersion => {
                write!(
                    response,
                    "\x1bP>|R-Term {}\x1b\\",
                    env!("CARGO_PKG_VERSION")
                )?;
            }
            TerminalResponse::KittyKeyboardFlags => {
                write!(response, "\x1b[?{}u", modes.kitty_keyboard_flags())?;
            }
            TerminalResponse::KeyModifierOptions(resource) => {
                let value = if resource == 4 {
                    modes.modify_other_keys()
                } else {
                    0
                };
                write!(response, "\x1b[>{resource};{value}m")?;
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
struct DecrqcraRequest {
    request_id: i64,
    top: u16,
    left: u16,
    bottom: u16,
    right: u16,
}

const UTF8_C1_OSC: &[u8] = b"\xc2\x9d";
const UTF8_C1_ST: &[u8] = b"\xc2\x9c";

#[derive(Clone)]
struct DecrqssResponse {
    kind: Option<DecrqssKind>,
    terminator: OscResponseTerminator,
}

#[derive(Clone, Copy)]
enum DecrqssKind {
    Sgr,
    CursorShape,
    ScrollRegion,
    ConformanceLevel,
    LeftRightMargins,
}

impl DecrqssResponse {
    fn write_into(&self, terminal: &Terminal, response: &mut Vec<u8>) {
        if let Some(kind) = self.kind {
            response.extend_from_slice(b"\x1bP1$r");
            match kind {
                DecrqssKind::Sgr => append_sgr_state(terminal.active_style(), response),
                DecrqssKind::CursorShape => {
                    append_cursor_shape_state(terminal.cursor_shape(), response);
                }
                DecrqssKind::ScrollRegion => {
                    append_scroll_region_state(terminal.scroll_region(), response);
                }
                DecrqssKind::ConformanceLevel => response.extend_from_slice(b"61;1\"p"),
                DecrqssKind::LeftRightMargins => {
                    append_left_right_margin_state(terminal.left_right_margins(), response);
                }
            }
        } else {
            response.extend_from_slice(b"\x1bP0$r");
        }
        response.extend_from_slice(self.terminator.bytes());
    }
}

fn append_sgr_state(style: &Cell, bytes: &mut Vec<u8>) {
    let mut params = Vec::new();
    if style.bold {
        params.push("1".to_owned());
    }
    if style.faint {
        params.push("2".to_owned());
    }
    if style.italic {
        params.push("3".to_owned());
    }
    append_underline_style_sgr(style, &mut params);
    if style.blink {
        params.push("5".to_owned());
    }
    if style.inverse {
        params.push("7".to_owned());
    }
    if style.conceal {
        params.push("8".to_owned());
    }
    if style.strikethrough {
        params.push("9".to_owned());
    }
    if style.double_underline {
        params.push("21".to_owned());
    }
    if style.overline {
        params.push("53".to_owned());
    }
    match style.vertical_align {
        VerticalAlign::Baseline => {}
        VerticalAlign::Superscript => params.push("73".to_owned()),
        VerticalAlign::Subscript => params.push("74".to_owned()),
    }
    append_color_sgr(58, style.underline_color, &mut params);
    append_color_sgr(38, style.foreground, &mut params);
    append_color_sgr(48, style.background, &mut params);

    if params.is_empty() {
        bytes.push(b'0');
    } else {
        bytes.extend_from_slice(params.join(";").as_bytes());
    }
    bytes.push(b'm');
}

fn append_underline_style_sgr(style: &Cell, params: &mut Vec<String>) {
    match style.underline_style {
        UnderlineStyle::None if style.double_underline => params.push("21".to_owned()),
        UnderlineStyle::None if style.underline => params.push("4".to_owned()),
        UnderlineStyle::None => {}
        UnderlineStyle::Single => params.push("4".to_owned()),
        UnderlineStyle::Double => params.push("21".to_owned()),
        UnderlineStyle::Curly => params.push("4:3".to_owned()),
        UnderlineStyle::Dotted => params.push("4:4".to_owned()),
        UnderlineStyle::Dashed => params.push("4:5".to_owned()),
    }
}

fn append_color_sgr(prefix: u8, color: Color, params: &mut Vec<String>) {
    match color {
        Color::Default => {}
        Color::Indexed(index) => {
            params.push(prefix.to_string());
            params.push("5".to_owned());
            params.push(index.to_string());
        }
        Color::Rgb(red, green, blue) => {
            params.push(prefix.to_string());
            params.push("2".to_owned());
            params.push(red.to_string());
            params.push(green.to_string());
            params.push(blue.to_string());
        }
        Color::Rgba(red, green, blue, alpha) => {
            params.push(prefix.to_string());
            params.push("6".to_owned());
            params.push(red.to_string());
            params.push(green.to_string());
            params.push(blue.to_string());
            params.push(alpha.to_string());
        }
    }
}

fn append_cursor_shape_state(shape: CursorShape, bytes: &mut Vec<u8>) {
    let value = match shape {
        CursorShape::Block => 2,
        CursorShape::Underline => 3,
        CursorShape::Bar => 5,
    };
    bytes.extend_from_slice(value.to_string().as_bytes());
    bytes.extend_from_slice(b" q");
}

fn append_scroll_region_state((top, bottom): (u16, u16), bytes: &mut Vec<u8>) {
    bytes.extend_from_slice(top.saturating_add(1).to_string().as_bytes());
    bytes.push(b';');
    bytes.extend_from_slice(bottom.saturating_add(1).to_string().as_bytes());
    bytes.push(b'r');
}

fn append_left_right_margin_state((left, right): (u16, u16), bytes: &mut Vec<u8>) {
    bytes.extend_from_slice(left.saturating_add(1).to_string().as_bytes());
    bytes.push(b';');
    bytes.extend_from_slice(right.saturating_add(1).to_string().as_bytes());
    bytes.push(b's');
}

#[derive(Clone)]
struct XtGetTcapResponse {
    entries: Vec<XtGetTcapEntry>,
}

#[derive(Clone)]
struct XtGetTcapEntry {
    name_hex: Vec<u8>,
    value_hex: Option<Vec<u8>>,
}

impl XtGetTcapResponse {
    fn write_into(&self, response: &mut Vec<u8>) {
        let response_start = response.len();
        if self.entries.is_empty() {
            response.extend_from_slice(b"\x1bP0+r\x1b\\");
            return;
        }

        for entry in &self.entries {
            let entry_start = response.len();
            if let Some(value_hex) = &entry.value_hex {
                response.extend_from_slice(b"\x1bP1+r");
                extend_ascii_hex_uppercase(response, &entry.name_hex);
                response.push(b'=');
                extend_ascii_hex_uppercase(response, value_hex);
            } else {
                response.extend_from_slice(b"\x1bP0+r");
                extend_ascii_hex_uppercase(response, &entry.name_hex);
            }
            response.extend_from_slice(b"\x1b\\");
            if response.len().saturating_sub(response_start) > MAX_XTGETTCAP_RESPONSE_BYTES {
                response.truncate(entry_start);
                break;
            }
        }
    }
}

#[allow(clippy::match_same_arms, clippy::too_many_lines)]
fn xtgettcap_value_hex(name: &[u8], size: TerminalSize, terminal_name: &str) -> Option<Vec<u8>> {
    match name {
        b"Co" | b"colors" => Some(b"323536".to_vec()),
        b"TN" | b"name" => Some(encode_ascii_hex(terminal_name.as_bytes())),
        b"RGB" => Some(b"382f382f38".to_vec()),
        b"Tc" => Some(b"31".to_vec()),
        b"am" => Some(b"31".to_vec()),
        b"bce" => Some(b"31".to_vec()),
        b"ccc" => Some(b"31".to_vec()),
        b"hs" => Some(b"31".to_vec()),
        b"km" => Some(b"31".to_vec()),
        b"mc5i" => Some(b"31".to_vec()),
        b"mir" => Some(b"31".to_vec()),
        b"msgr" => Some(b"31".to_vec()),
        b"npc" => Some(b"31".to_vec()),
        b"Su" => Some(b"31".to_vec()),
        b"xenl" => Some(b"31".to_vec()),
        b"Ms" => Some(b"1b5d35323b25703125733b257032257307".to_vec()),
        b"dsl" => Some(encode_ascii_hex(b"\x1b]2;\x1b\\")),
        b"fsl" => Some(encode_ascii_hex(b"\x1b\\")),
        b"tsl" => Some(encode_ascii_hex(b"\x1b]0;")),
        b"initc" => Some(encode_ascii_hex(
            b"\x1b]4;%p1%d;rgb:%p2%{255}%*%{1000}%/%2.2X/%p3%{255}%*%{1000}%/%2.2X/%p4%{255}%*%{1000}%/%2.2X\x1b\\",
        )),
        b"Smulx" => Some(b"1b5b343a25703125646d".to_vec()),
        b"Setulc" => Some(
            b"1b5b35383a323a3a257031257b36353533367d252f25643a257031257b3235367d252f257b3235357d252625643a257031257b3235357d25262564253b6d"
                .to_vec(),
        ),
        b"Cr" => Some(encode_ascii_hex(b"\x1b]112\x07")),
        b"Cs" => Some(encode_ascii_hex(b"\x1b]12;%p1%s\x07")),
        b"Se" => Some(encode_ascii_hex(b"\x1b[2 q")),
        b"Ss" => Some(encode_ascii_hex(b"\x1b[%p1%d q")),
        b"Sync" => Some(encode_ascii_hex(b"\x1b[?2026%?%p1%{1}%-%tl%eh%;")),
        b"sitm" => Some(b"1b5b336d".to_vec()),
        b"ritm" => Some(b"1b5b32336d".to_vec()),
        b"Smol" => Some(encode_ascii_hex(b"\x1b[53m")),
        b"smxx" => Some(encode_ascii_hex(b"\x1b[9m")),
        b"rmxx" => Some(encode_ascii_hex(b"\x1b[29m")),
        b"flash" => Some(encode_ascii_hex(b"\x1b[?5h$<100/>\x1b[?5l")),
        b"op" => Some(encode_ascii_hex(b"\x1b[39;49m")),
        b"oc" => Some(encode_ascii_hex(b"\x1b]104\x07")),
        b"bel" => Some(encode_ascii_hex(b"\x07")),
        b"cr" => Some(encode_ascii_hex(b"\r")),
        b"ind" => Some(encode_ascii_hex(b"\n")),
        b"ri" => Some(encode_ascii_hex(b"\x1bM")),
        b"sc" => Some(encode_ascii_hex(b"\x1b7")),
        b"rc" => Some(encode_ascii_hex(b"\x1b8")),
        b"u6" => Some(encode_ascii_hex(b"\x1b[%i%d;%dR")),
        b"u7" => Some(encode_ascii_hex(b"\x1b[6n")),
        b"u8" => Some(encode_ascii_hex(b"\x1b[?%[;0123456789]c")),
        b"u9" => Some(encode_ascii_hex(b"\x1b[c")),
        b"clear" => Some(encode_ascii_hex(b"\x1b[H\x1b[2J")),
        b"cup" => Some(encode_ascii_hex(b"\x1b[%i%p1%d;%p2%dH")),
        b"home" => Some(encode_ascii_hex(b"\x1b[H")),
        b"el" => Some(encode_ascii_hex(b"\x1b[K")),
        b"ed" => Some(encode_ascii_hex(b"\x1b[J")),
        b"el1" => Some(encode_ascii_hex(b"\x1b[1K")),
        b"dch" => Some(encode_ascii_hex(b"\x1b[%p1%dP")),
        b"dch1" => Some(encode_ascii_hex(b"\x1b[P")),
        b"ich" => Some(encode_ascii_hex(b"\x1b[%p1%d@")),
        b"ich1" => Some(encode_ascii_hex(b"\x1b[@")),
        b"il" => Some(encode_ascii_hex(b"\x1b[%p1%dL")),
        b"il1" => Some(encode_ascii_hex(b"\x1b[L")),
        b"dl" => Some(encode_ascii_hex(b"\x1b[%p1%dM")),
        b"dl1" => Some(encode_ascii_hex(b"\x1b[M")),
        b"cuu" => Some(encode_ascii_hex(b"\x1b[%p1%dA")),
        b"cuu1" => Some(encode_ascii_hex(b"\x1b[A")),
        b"cud" => Some(encode_ascii_hex(b"\x1b[%p1%dB")),
        b"cud1" => Some(encode_ascii_hex(b"\n")),
        b"cub" => Some(encode_ascii_hex(b"\x1b[%p1%dD")),
        b"cub1" => Some(encode_ascii_hex(b"\x08")),
        b"cuf" => Some(encode_ascii_hex(b"\x1b[%p1%dC")),
        b"cuf1" => Some(encode_ascii_hex(b"\x1b[C")),
        b"hpa" => Some(encode_ascii_hex(b"\x1b[%i%p1%dG")),
        b"vpa" => Some(encode_ascii_hex(b"\x1b[%i%p1%dd")),
        b"cbt" => Some(encode_ascii_hex(b"\x1b[Z")),
        b"ht" => Some(encode_ascii_hex(b"\t")),
        b"hts" => Some(encode_ascii_hex(b"\x1bH")),
        b"tbc" => Some(encode_ascii_hex(b"\x1b[3g")),
        b"ech" => Some(encode_ascii_hex(b"\x1b[%p1%dX")),
        b"rep" => Some(encode_ascii_hex(b"%p1%c\x1b[%p2%{1}%-%db")),
        b"csr" => Some(encode_ascii_hex(b"\x1b[%i%p1%d;%p2%dr")),
        b"indn" => Some(encode_ascii_hex(b"\x1b[%p1%dS")),
        b"rin" => Some(encode_ascii_hex(b"\x1b[%p1%dT")),
        b"kmous" => Some(encode_ascii_hex(b"\x1b[<")),
        b"XM" => Some(encode_ascii_hex(
            b"\x1b[?1006;1000%?%p1%{1}%=%th%el%;",
        )),
        b"xm" => Some(encode_ascii_hex(
            b"\x1b[<%i%p3%d;%p1%d;%p2%d;%?%p4%tM%em%;",
        )),
        b"civis" => Some(encode_ascii_hex(b"\x1b[?25l")),
        b"cnorm" => Some(encode_ascii_hex(b"\x1b[?12l\x1b[?25h")),
        b"cvvis" => Some(encode_ascii_hex(b"\x1b[?12;25h")),
        b"smcup" => Some(encode_ascii_hex(b"\x1b[?1049h\x1b[22;0;0t")),
        b"rmcup" => Some(encode_ascii_hex(b"\x1b[?1049l\x1b[23;0;0t")),
        b"is2" | b"rs2" => Some(encode_ascii_hex(b"\x1b[!p\x1b[?3;4l\x1b[4l\x1b>")),
        b"rs1" => Some(encode_ascii_hex(b"\x1bc\x1b]104\x07")),
        b"smir" => Some(encode_ascii_hex(b"\x1b[4h")),
        b"rmir" => Some(encode_ascii_hex(b"\x1b[4l")),
        b"smam" => Some(encode_ascii_hex(b"\x1b[?7h")),
        b"rmam" => Some(encode_ascii_hex(b"\x1b[?7l")),
        b"smm" => Some(encode_ascii_hex(b"\x1b[?1034h")),
        b"rmm" => Some(encode_ascii_hex(b"\x1b[?1034l")),
        b"mc0" => Some(encode_ascii_hex(b"\x1b[i")),
        b"mc4" => Some(encode_ascii_hex(b"\x1b[4i")),
        b"mc5" => Some(encode_ascii_hex(b"\x1b[5i")),
        b"meml" => Some(encode_ascii_hex(b"\x1bl")),
        b"memu" => Some(encode_ascii_hex(b"\x1bm")),
        b"smkx" => Some(encode_ascii_hex(b"\x1b[?1h\x1b=")),
        b"rmkx" => Some(encode_ascii_hex(b"\x1b[?1l\x1b>")),
        b"sgr0" => Some(encode_ascii_hex(b"\x1b(B\x1b[m")),
        b"sgr" => Some(encode_ascii_hex(
            b"%?%p9%t\x1b(0%e\x1b(B%;\x1b[0%?%p6%t;1%;%?%p5%t;2%;%?%p2%t;4%;%?%p1%p3%|%t;7%;%?%p4%t;5%;%?%p7%t;8%;m",
        )),
        b"bold" => Some(encode_ascii_hex(b"\x1b[1m")),
        b"dim" => Some(encode_ascii_hex(b"\x1b[2m")),
        b"blink" => Some(encode_ascii_hex(b"\x1b[5m")),
        b"rev" => Some(encode_ascii_hex(b"\x1b[7m")),
        b"smso" => Some(encode_ascii_hex(b"\x1b[7m")),
        b"rmso" => Some(encode_ascii_hex(b"\x1b[27m")),
        b"invis" => Some(encode_ascii_hex(b"\x1b[8m")),
        b"smul" => Some(encode_ascii_hex(b"\x1b[4m")),
        b"rmul" => Some(encode_ascii_hex(b"\x1b[24m")),
        b"setaf" => Some(encode_ascii_hex(
            b"\x1b[%?%p1%{8}%<%t3%p1%d%e%p1%{16}%<%t9%p1%{8}%-%d%e38;5;%p1%d%;m",
        )),
        b"setab" => Some(encode_ascii_hex(
            b"\x1b[%?%p1%{8}%<%t4%p1%d%e%p1%{16}%<%t10%p1%{8}%-%d%e48;5;%p1%d%;m",
        )),
        b"kcuu1" => Some(encode_ascii_hex(b"\x1bOA")),
        b"kcud1" => Some(encode_ascii_hex(b"\x1bOB")),
        b"kcuf1" => Some(encode_ascii_hex(b"\x1bOC")),
        b"kcub1" => Some(encode_ascii_hex(b"\x1bOD")),
        b"kb2" => Some(encode_ascii_hex(b"\x1bOE")),
        b"kbs" => Some(encode_ascii_hex(b"\x7f")),
        b"kcbt" => Some(encode_ascii_hex(b"\x1b[Z")),
        b"khome" => Some(encode_ascii_hex(b"\x1bOH")),
        b"kend" => Some(encode_ascii_hex(b"\x1bOF")),
        b"kich1" => Some(encode_ascii_hex(b"\x1b[2~")),
        b"kdch1" => Some(encode_ascii_hex(b"\x1b[3~")),
        b"kpp" => Some(encode_ascii_hex(b"\x1b[5~")),
        b"knp" => Some(encode_ascii_hex(b"\x1b[6~")),
        b"kHOM" => Some(encode_ascii_hex(b"\x1b[1;2H")),
        b"kEND" => Some(encode_ascii_hex(b"\x1b[1;2F")),
        b"kIC" => Some(encode_ascii_hex(b"\x1b[2;2~")),
        b"kDC" => Some(encode_ascii_hex(b"\x1b[3;2~")),
        b"kPRV" => Some(encode_ascii_hex(b"\x1b[5;2~")),
        b"kNXT" => Some(encode_ascii_hex(b"\x1b[6;2~")),
        b"kLFT" => Some(encode_ascii_hex(b"\x1b[1;2D")),
        b"kRIT" => Some(encode_ascii_hex(b"\x1b[1;2C")),
        b"kri" => Some(encode_ascii_hex(b"\x1b[1;2A")),
        b"kind" => Some(encode_ascii_hex(b"\x1b[1;2B")),
        b"kent" => Some(encode_ascii_hex(b"\x1bOM")),
        b"kf1" => Some(encode_ascii_hex(b"\x1bOP")),
        b"kf2" => Some(encode_ascii_hex(b"\x1bOQ")),
        b"kf3" => Some(encode_ascii_hex(b"\x1bOR")),
        b"kf4" => Some(encode_ascii_hex(b"\x1bOS")),
        b"kf5" => Some(encode_ascii_hex(b"\x1b[15~")),
        b"kf6" => Some(encode_ascii_hex(b"\x1b[17~")),
        b"kf7" => Some(encode_ascii_hex(b"\x1b[18~")),
        b"kf8" => Some(encode_ascii_hex(b"\x1b[19~")),
        b"kf9" => Some(encode_ascii_hex(b"\x1b[20~")),
        b"kf10" => Some(encode_ascii_hex(b"\x1b[21~")),
        b"kf11" => Some(encode_ascii_hex(b"\x1b[23~")),
        b"kf12" => Some(encode_ascii_hex(b"\x1b[24~")),
        name if name.starts_with(b"kf") => xtgettcap_modified_function_key_hex(name),
        b"enacs" => Some(encode_ascii_hex(b"\x1b)0")),
        b"smacs" => Some(encode_ascii_hex(b"\x1b(0")),
        b"rmacs" => Some(encode_ascii_hex(b"\x1b(B")),
        b"acsc" => Some(encode_ascii_hex(
            b"``aaffggiijjkkllmmnnooppqqrrssttuuvvwwxxyyzz{{||}}~~",
        )),
        b"co" | b"cols" => Some(decimal_value_hex(size.columns)),
        b"li" | b"lines" => Some(decimal_value_hex(size.rows)),
        b"it" => Some(decimal_value_hex(8)),
        b"pairs" => Some(decimal_value_hex(0x7fff)),
        _ => None,
    }
}

fn xtgettcap_modified_function_key_hex(name: &[u8]) -> Option<Vec<u8>> {
    let number = parse_ascii_decimal_u8(name.strip_prefix(b"kf")?)?;
    let (function_key, modifier) = match number {
        13..=24 => (number - 12, 2),
        25..=36 => (number - 24, 5),
        37..=48 => (number - 36, 6),
        49..=60 => (number - 48, 3),
        61..=63 => (number - 60, 4),
        _ => return None,
    };

    let sequence = match function_key {
        1 => format!("\x1b[1;{modifier}P"),
        2 => format!("\x1b[1;{modifier}Q"),
        3 => format!("\x1b[1;{modifier}R"),
        4 => format!("\x1b[1;{modifier}S"),
        5 => format!("\x1b[15;{modifier}~"),
        6 => format!("\x1b[17;{modifier}~"),
        7 => format!("\x1b[18;{modifier}~"),
        8 => format!("\x1b[19;{modifier}~"),
        9 => format!("\x1b[20;{modifier}~"),
        10 => format!("\x1b[21;{modifier}~"),
        11 => format!("\x1b[23;{modifier}~"),
        12 => format!("\x1b[24;{modifier}~"),
        _ => return None,
    };

    Some(encode_ascii_hex(sequence.as_bytes()))
}

fn parse_ascii_decimal_u8(bytes: &[u8]) -> Option<u8> {
    if bytes.is_empty() {
        return None;
    }

    let mut value = 0u8;
    for &byte in bytes {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add(byte - b'0')?;
    }
    Some(value)
}

fn decimal_value_hex(value: u16) -> Vec<u8> {
    encode_ascii_hex(value.to_string().as_bytes())
}

fn encode_ascii_hex(bytes: &[u8]) -> Vec<u8> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = Vec::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[usize::from(byte >> 4)]);
        encoded.push(HEX[usize::from(byte & 0x0f)]);
    }
    encoded
}

fn extend_ascii_hex_uppercase(output: &mut Vec<u8>, hex: &[u8]) {
    output.extend(hex.iter().map(u8::to_ascii_uppercase));
}

#[derive(Clone, Copy)]
struct XtSmGraphicsRequest {
    item: u64,
    action: u64,
}

impl XtSmGraphicsRequest {
    const ACTION_READ_ATTRIBUTE: u64 = 1;
    const ACTION_RESET_TO_DEFAULT: u64 = 2;
    const ACTION_READ_MAXIMUM_ALLOWED_VALUE: u64 = 4;
    const ITEM_NUMBER_OF_COLOR_REGISTERS: u64 = 1;
    const ITEM_SIXEL_GRAPHICS_GEOMETRY: u64 = 2;
    const ITEM_REGIS_GRAPHICS_GEOMETRY: u64 = 3;
    const STATUS_SUCCESS: u64 = 0;
    const STATUS_INVALID_ITEM: u64 = 1;
    const STATUS_INVALID_ACTION: u64 = 2;

    fn write_into(self, size: TerminalSize, response: &mut Vec<u8>) -> io::Result<()> {
        let (status, values) = self.status_and_values(size);
        write!(response, "\x1b[?{};{}", self.item, status)?;
        for value in values {
            write!(response, ";{value}")?;
        }
        response.push(b'S');
        Ok(())
    }

    fn status_and_values(self, size: TerminalSize) -> (u64, Vec<u32>) {
        if !matches!(
            self.item,
            Self::ITEM_NUMBER_OF_COLOR_REGISTERS
                | Self::ITEM_SIXEL_GRAPHICS_GEOMETRY
                | Self::ITEM_REGIS_GRAPHICS_GEOMETRY
        ) {
            return (Self::STATUS_INVALID_ITEM, Vec::new());
        }

        match self.action {
            Self::ACTION_READ_ATTRIBUTE | Self::ACTION_READ_MAXIMUM_ALLOWED_VALUE => {
                (Self::STATUS_SUCCESS, self.values(size))
            }
            Self::ACTION_RESET_TO_DEFAULT => (Self::STATUS_SUCCESS, Vec::new()),
            _ => (Self::STATUS_INVALID_ACTION, Vec::new()),
        }
    }

    fn values(self, size: TerminalSize) -> Vec<u32> {
        match self.item {
            Self::ITEM_NUMBER_OF_COLOR_REGISTERS => vec![65_536],
            Self::ITEM_SIXEL_GRAPHICS_GEOMETRY | Self::ITEM_REGIS_GRAPHICS_GEOMETRY => vec![
                u32::from(size.columns) * u32::from(TerminalOutputFilter::CELL_WIDTH_PIXELS),
                u32::from(size.rows) * u32::from(TerminalOutputFilter::CELL_HEIGHT_PIXELS),
            ],
            _ => Vec::new(),
        }
    }
}

#[derive(Clone)]
struct OscColorResponse {
    kinds: Vec<OscColorKind>,
    terminator: OscResponseTerminator,
}

#[derive(Clone, Copy)]
enum OscColorKind {
    DefaultForeground,
    DefaultBackground,
    Cursor,
    Palette(u8),
}

#[derive(Clone, Copy)]
enum OscResponseTerminator {
    Bel,
    St,
    C1St,
}

fn parse_u8_decimal(bytes: &[u8]) -> Option<u8> {
    if bytes.is_empty() {
        return None;
    }
    let mut value = 0u16;
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value
            .saturating_mul(10)
            .saturating_add(u16::from(*byte - b'0'));
    }
    u8::try_from(value).ok()
}

struct TerminalColorState {
    foreground: DynamicColor,
    background: DynamicColor,
    cursor_override: Option<DynamicColor>,
    palette_overrides: Vec<(u8, [u8; 3])>,
}

impl Default for TerminalColorState {
    fn default() -> Self {
        Self {
            foreground: DynamicColor::rgb8(DEFAULT_FOREGROUND),
            background: DynamicColor::rgb8(DEFAULT_BACKGROUND),
            cursor_override: None,
            palette_overrides: Vec::new(),
        }
    }
}

impl TerminalColorState {
    fn process_control(&mut self, bytes: &[u8]) -> bool {
        let Some(content) = complete_osc_content(bytes) else {
            return false;
        };
        parse_osc_color_change(content).is_some_and(|change| self.apply(change))
    }

    fn response(&self, query: OscColorResponse) -> Vec<u8> {
        let mut response = Vec::new();
        self.write_response(query, &mut response)
            .expect("writing to a byte arena cannot fail");
        response
    }

    fn write_response(&self, query: OscColorResponse, response: &mut Vec<u8>) -> io::Result<()> {
        for kind in query.kinds {
            let color = match kind {
                OscColorKind::DefaultForeground => {
                    response.extend_from_slice(b"\x1b]10;");
                    self.foreground
                }
                OscColorKind::DefaultBackground => {
                    response.extend_from_slice(b"\x1b]11;");
                    self.background
                }
                OscColorKind::Cursor => {
                    response.extend_from_slice(b"\x1b]12;");
                    self.cursor_color()
                }
                OscColorKind::Palette(index) => {
                    write!(response, "\x1b]4;{index};")?;
                    DynamicColor::rgb8(self.palette_color(index))
                }
            };
            write_color_response(response, color)?;
            response.extend_from_slice(query.terminator.bytes());
        }
        Ok(())
    }

    fn apply(&mut self, change: OscColorChange) -> bool {
        match change {
            OscColorChange::DefaultForeground(color) => {
                let changed = self.foreground != color;
                self.foreground = color;
                changed
            }
            OscColorChange::DefaultBackground(color) => {
                let changed = self.background != color;
                self.background = color;
                changed
            }
            OscColorChange::Cursor(color) => {
                let before = self.cursor_color();
                self.cursor_override = Some(color);
                before != self.cursor_color()
            }
            OscColorChange::ResetDefaultForeground => {
                let before = self.foreground;
                self.foreground = DynamicColor::rgb8(DEFAULT_FOREGROUND);
                before != self.foreground
            }
            OscColorChange::ResetDefaultBackground => {
                let before = self.background;
                self.background = DynamicColor::rgb8(DEFAULT_BACKGROUND);
                before != self.background
            }
            OscColorChange::ResetCursor => {
                let before = self.cursor_color();
                self.cursor_override = None;
                before != self.cursor_color()
            }
            OscColorChange::ResetPalette(indices) => {
                let changed = self.palette_overrides.iter().any(|(index, color)| {
                    indices.contains(index) && *color != indexed_color(*index)
                });
                self.palette_overrides
                    .retain(|(index, _)| !indices.contains(index));
                changed
            }
            OscColorChange::ResetPaletteAll => {
                let changed = self
                    .palette_overrides
                    .iter()
                    .any(|(index, color)| *color != indexed_color(*index));
                self.palette_overrides.clear();
                changed
            }
            OscColorChange::Palette(changes) => {
                let mut before = Vec::new();
                for (index, _) in &changes {
                    if before
                        .iter()
                        .all(|(existing_index, _)| existing_index != index)
                    {
                        before.push((*index, self.palette_color(*index)));
                    }
                }
                for (index, color) in changes {
                    if let Some((_, existing)) = self
                        .palette_overrides
                        .iter_mut()
                        .find(|(palette_index, _)| *palette_index == index)
                    {
                        *existing = color;
                    } else {
                        self.palette_overrides.push((index, color));
                    }
                }
                before
                    .into_iter()
                    .any(|(index, color)| self.palette_color(index) != color)
            }
        }
    }

    fn palette_color(&self, index: u8) -> [u8; 3] {
        self.palette_overrides
            .iter()
            .find_map(|(palette_index, color)| (*palette_index == index).then_some(*color))
            .unwrap_or_else(|| indexed_color(index))
    }

    fn cursor_color(&self) -> DynamicColor {
        self.cursor_override
            .unwrap_or_else(|| DynamicColor::rgb8(DEFAULT_CURSOR))
    }

    fn cursor_override(&self) -> Option<DynamicColor> {
        self.cursor_override
    }
}

fn complete_osc_content(bytes: &[u8]) -> Option<&[u8]> {
    let body = bytes
        .strip_prefix(b"\x1b]")
        .or_else(|| bytes.strip_prefix(b"\x9d"))
        .or_else(|| bytes.strip_prefix(UTF8_C1_OSC))?;
    body.strip_suffix(b"\x07")
        .or_else(|| body.strip_suffix(b"\x1b\\"))
        .or_else(|| body.strip_suffix(b"\x9c"))
        .or_else(|| body.strip_suffix(UTF8_C1_ST))
}

#[derive(Clone)]
enum OscColorChange {
    DefaultForeground(DynamicColor),
    DefaultBackground(DynamicColor),
    Cursor(DynamicColor),
    ResetDefaultForeground,
    ResetDefaultBackground,
    ResetCursor,
    ResetPalette(Vec<u8>),
    ResetPaletteAll,
    Palette(Vec<(u8, [u8; 3])>),
}

fn parse_osc_color_change(content: &[u8]) -> Option<OscColorChange> {
    if let Some(color) = content.strip_prefix(b"10;").and_then(parse_color_spec) {
        return Some(OscColorChange::DefaultForeground(color));
    }
    if let Some(color) = content.strip_prefix(b"11;").and_then(parse_color_spec) {
        return Some(OscColorChange::DefaultBackground(color));
    }
    if let Some(color) = content.strip_prefix(b"12;").and_then(parse_color_spec) {
        return Some(OscColorChange::Cursor(color));
    }
    if matches!(content, b"110" | b"110;") {
        return Some(OscColorChange::ResetDefaultForeground);
    }
    if matches!(content, b"111" | b"111;") {
        return Some(OscColorChange::ResetDefaultBackground);
    }
    if matches!(content, b"112" | b"112;") {
        return Some(OscColorChange::ResetCursor);
    }
    if let Some(change) = parse_palette_reset_change(content) {
        return Some(change);
    }
    parse_palette_color_change(content)
}

fn parse_palette_reset_change(content: &[u8]) -> Option<OscColorChange> {
    if matches!(content, b"104" | b"104;") {
        return Some(OscColorChange::ResetPaletteAll);
    }
    let rest = content.strip_prefix(b"104;")?;
    let mut indices = Vec::new();
    for index in rest.split(|byte| *byte == b';') {
        indices.push(parse_u8_decimal(index)?);
    }
    (!indices.is_empty()).then_some(OscColorChange::ResetPalette(indices))
}

fn parse_palette_color_change(content: &[u8]) -> Option<OscColorChange> {
    let rest = content.strip_prefix(b"4;")?;
    let mut changes = Vec::new();
    let mut parts = rest.split(|byte| *byte == b';');

    while let Some(index) = parts.next() {
        let color = parts.next()?;
        changes.push((parse_u8_decimal(index)?, parse_color_spec(color)?.to_rgb8()));
    }

    (!changes.is_empty()).then_some(OscColorChange::Palette(changes))
}

fn parse_color_spec(value: &[u8]) -> Option<DynamicColor> {
    if let Some(hex) = value.strip_prefix(b"#") {
        return parse_hex_color_spec(hex);
    }
    if let Some(rest) = value.strip_prefix(b"rgba:") {
        return parse_slash_rgba_color_spec(rest);
    }
    if value.starts_with(b"rgba(") {
        return parse_function_rgba_color_spec(value);
    }

    let rest = value.strip_prefix(b"rgb:")?;
    let mut components = rest.split(|byte| *byte == b'/');
    let red = parse_rgb_component(components.next()?)?;
    let green = parse_rgb_component(components.next()?)?;
    let blue = parse_rgb_component(components.next()?)?;
    components
        .next()
        .is_none()
        .then_some(DynamicColor::rgb(red, green, blue))
}

fn parse_hex_color_spec(hex: &[u8]) -> Option<DynamicColor> {
    match hex.len() {
        3 => Some(DynamicColor::rgb8([
            parse_hex_digit(hex[0])? * 17,
            parse_hex_digit(hex[1])? * 17,
            parse_hex_digit(hex[2])? * 17,
        ])),
        6 => Some(DynamicColor::rgb8([
            parse_hex_byte(&hex[0..2])?,
            parse_hex_byte(&hex[2..4])?,
            parse_hex_byte(&hex[4..6])?,
        ])),
        _ => None,
    }
}

fn parse_slash_rgba_color_spec(value: &[u8]) -> Option<DynamicColor> {
    let mut components = value.split(|byte| *byte == b'/');
    let red = parse_hex_component16(components.next()?)?;
    let green = parse_hex_component16(components.next()?)?;
    let blue = parse_hex_component16(components.next()?)?;
    let alpha = parse_hex_component16(components.next()?)?;
    components
        .next()
        .is_none()
        .then_some(DynamicColor::rgba(red, green, blue, alpha))
}

fn parse_function_rgba_color_spec(value: &[u8]) -> Option<DynamicColor> {
    let inner = value.strip_prefix(b"rgba(")?.strip_suffix(b")")?;
    let mut components = inner.split(|byte| *byte == b',');
    let red = parse_u8_decimal(components.next()?.trim_ascii())?;
    let green = parse_u8_decimal(components.next()?.trim_ascii())?;
    let blue = parse_u8_decimal(components.next()?.trim_ascii())?;
    let alpha = parse_alpha_float_component(components.next()?.trim_ascii())?;
    components
        .next()
        .is_none()
        .then_some(DynamicColor::rgba8(red, green, blue, alpha))
}

fn parse_rgb_component(component: &[u8]) -> Option<u16> {
    match component.len() {
        1 => parse_hex_digit(component[0]).map(|value| u16::from(value) * 0x1111),
        2 => parse_hex_byte(component).map(DynamicColor::expand_byte),
        3 | 4 => parse_hex_component16(component),
        _ => None,
    }
}

fn parse_hex_component16(component: &[u8]) -> Option<u16> {
    match component.len() {
        1 => parse_hex_digit(component[0]).map(|value| u16::from(value) * 0x1111),
        2 => parse_hex_byte(component).map(DynamicColor::expand_byte),
        3 => Some(
            parse_hex_digit(component[0]).map(u16::from)? * 0x1000
                + parse_hex_digit(component[1]).map(u16::from)? * 0x100
                + parse_hex_digit(component[2]).map(u16::from)? * 0x10,
        ),
        4 => Some(
            parse_hex_digit(component[0]).map(u16::from)? * 0x1000
                + parse_hex_digit(component[1]).map(u16::from)? * 0x100
                + parse_hex_digit(component[2]).map(u16::from)? * 0x10
                + parse_hex_digit(component[3]).map(u16::from)?,
        ),
        _ => None,
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn parse_alpha_float_component(component: &[u8]) -> Option<u16> {
    let text = std::str::from_utf8(component).ok()?;
    let value = text.parse::<f32>().ok()?;
    if !(0.0..=1.0).contains(&value) {
        return None;
    }
    Some((value * f32::from(u16::MAX)).round() as u16)
}

fn parse_hex_byte(bytes: &[u8]) -> Option<u8> {
    Some(parse_hex_digit(bytes[0])? * 16 + parse_hex_digit(bytes[1])?)
}

fn parse_hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

impl OscResponseTerminator {
    const fn bytes(self) -> &'static [u8] {
        match self {
            Self::Bel => b"\x07",
            Self::St => b"\x1b\\",
            Self::C1St => b"\x9c",
        }
    }
}

const DEFAULT_FOREGROUND: [u8; 3] = [229, 229, 229];
const DEFAULT_BACKGROUND: [u8; 3] = [12, 12, 12];
const DEFAULT_CURSOR: [u8; 3] = DEFAULT_FOREGROUND;

#[derive(Clone, Copy, PartialEq, Eq)]
struct DynamicColor {
    red: u16,
    green: u16,
    blue: u16,
    alpha: Option<u16>,
}

impl DynamicColor {
    const fn rgb8(color: [u8; 3]) -> Self {
        Self::rgb(
            color[0] as u16 * 0x101,
            color[1] as u16 * 0x101,
            color[2] as u16 * 0x101,
        )
    }

    const fn rgb(red: u16, green: u16, blue: u16) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: None,
        }
    }

    const fn rgba(red: u16, green: u16, blue: u16, alpha: u16) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: Some(alpha),
        }
    }

    const fn rgba8(red: u8, green: u8, blue: u8, alpha: u16) -> Self {
        Self::rgba(
            Self::expand_byte(red),
            Self::expand_byte(green),
            Self::expand_byte(blue),
            alpha,
        )
    }

    const fn expand_byte(value: u8) -> u16 {
        value as u16 * 0x101
    }

    const fn to_rgb8(self) -> [u8; 3] {
        [
            (self.red >> 8) as u8,
            (self.green >> 8) as u8,
            (self.blue >> 8) as u8,
        ]
    }

    const fn to_color(self) -> Color {
        let red = (self.red >> 8) as u8;
        let green = (self.green >> 8) as u8;
        let blue = (self.blue >> 8) as u8;
        match self.alpha {
            Some(alpha) => Color::Rgba(red, green, blue, (alpha >> 8) as u8),
            None => Color::Rgb(red, green, blue),
        }
    }
}

fn write_color_response(response: &mut Vec<u8>, color: DynamicColor) -> io::Result<()> {
    match color.alpha {
        Some(alpha) => write!(
            response,
            "rgba:{:04x}/{:04x}/{:04x}/{:04x}",
            color.red, color.green, color.blue, alpha
        )?,
        None => write!(
            response,
            "rgb:{:04x}/{:04x}/{:04x}",
            color.red, color.green, color.blue
        )?,
    }
    Ok(())
}

fn indexed_color(index: u8) -> [u8; 3] {
    const ANSI: [[u8; 3]; 16] = [
        [0, 0, 0],
        [205, 49, 49],
        [13, 188, 121],
        [229, 229, 16],
        [36, 114, 200],
        [188, 63, 188],
        [17, 168, 205],
        [229, 229, 229],
        [102, 102, 102],
        [241, 76, 76],
        [35, 209, 139],
        [245, 245, 67],
        [59, 142, 234],
        [214, 112, 214],
        [41, 184, 219],
        [255, 255, 255],
    ];

    if let Some(color) = ANSI.get(usize::from(index)) {
        return *color;
    }

    if (16..=231).contains(&index) {
        let cube_index = index - 16;
        return [
            xterm_color_cube_intensity(cube_index / 36),
            xterm_color_cube_intensity((cube_index / 6) % 6),
            xterm_color_cube_intensity(cube_index % 6),
        ];
    }

    let level = 8 + (index - 232) * 10;
    [level, level, level]
}

const fn xterm_color_cube_intensity(value: u8) -> u8 {
    if value == 0 { 0 } else { 55 + value * 40 }
}

#[cfg(test)]
#[path = "terminal_transcript_tests.rs"]
mod terminal_transcript_tests;

#[cfg(test)]
mod tests {
    mod task10_registry {
        include!("terminal_task10_registry.rs");
    }

    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use rterm_types::TerminalSize;

    use crate::{
        delta::{RuntimeBuffers, RuntimeDelta},
        modes::{MouseInputMode, MouseProtocolMode, MouseReportingMode, TerminalModeTracker},
        queries::TerminalQueryScanner,
    };

    use super::{
        DecrqcraRequest, DecrqssKind, DecrqssResponse, FilteredOutputEvent, FilteredOutputEventRef,
        OscResponseTerminator, TerminalNotification, TerminalOutputFilter, TerminalProgress,
        TerminalResponse, TerminalRuntime, XtGetTcapEntry, XtGetTcapResponse, XtSmGraphicsRequest,
    };

    #[test]
    fn normal_runtime_keeps_query_scan_counter_disabled() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(10, 2));
        runtime.feed_pty_output(b"plain output\x1b[6n");

        assert_eq!(runtime.inspected_query_bytes(), 0);
    }

    #[test]
    fn measured_runtime_counts_query_matcher_work() {
        let mut runtime = TerminalRuntime::new_with_query_scan_work(TerminalSize::new(10, 2));
        runtime.feed_pty_output(b"plain output\x1b[6n");

        assert!(runtime.inspected_query_bytes() > 0);
    }

    #[test]
    fn every_terminal_response_variant_matches_legacy_bytes_in_direct_arena() {
        let size = TerminalSize::new(20, 4);
        let mut terminal = rterm_terminal::Terminal::new(size);
        terminal.feed(b"cell\x1b]0;direct title\x07");
        let modes = TerminalModeTracker::default();
        let responses = vec![
            TerminalResponse::Static(b"static"),
            TerminalResponse::CursorPosition { private: false },
            TerminalResponse::CursorPosition { private: true },
            TerminalResponse::WindowPixelSize,
            TerminalResponse::CharacterCellSize,
            TerminalResponse::TextAreaSize,
            TerminalResponse::WindowTitle,
            TerminalResponse::PrivateModeStatus(1),
            TerminalResponse::AnsiModeStatus(4),
            TerminalResponse::ItermReportCellSize,
            TerminalResponse::ChecksumRectangularArea(DecrqcraRequest {
                request_id: 7,
                top: 0,
                left: 0,
                bottom: 1,
                right: 4,
            }),
            TerminalResponse::Decrqss(DecrqssResponse {
                kind: Some(DecrqssKind::Sgr),
                terminator: OscResponseTerminator::Bel,
            }),
            TerminalResponse::XtGetTcap(XtGetTcapResponse {
                entries: vec![XtGetTcapEntry {
                    name_hex: b"544e".to_vec(),
                    value_hex: Some(b"72737368".to_vec()),
                }],
            }),
            TerminalResponse::XtSmGraphics(XtSmGraphicsRequest { item: 1, action: 1 }),
            TerminalResponse::XtVersion,
            TerminalResponse::KittyKeyboardFlags,
            TerminalResponse::KeyModifierOptions(4),
        ];
        let expected = responses
            .iter()
            .cloned()
            .map(|response| response.response_bytes(size, &terminal, &modes))
            .collect::<Vec<_>>();
        let mut buffers = RuntimeBuffers::with_capacity(1024);

        for response in responses {
            buffers
                .try_push_transport_write_with(|arena| {
                    response.write_into(size, &terminal, &modes, arena)
                })
                .unwrap();
        }

        let delta = RuntimeDelta::new(&buffers, 0, false, false);
        assert_eq!(
            delta.responses().map(<[u8]>::to_vec).collect::<Vec<_>>(),
            expected
        );
        assert_eq!(buffers.response_commits(), 17);
        assert_eq!(buffers.response_payload_copies(), 0);
        assert_eq!(buffers.owned_response_materializations(), 0);
    }

    #[test]
    fn gui_filter_passes_malformed_modes_and_fail_closes_reserved_clipboard() {
        let malformed = b"\x1b[?2026;badh\x1b[?2026;;l\x1b[>badu\x1b[=1;4u\x1b[>badm";
        let mut filter = TerminalOutputFilter::new(TerminalSize::new(20, 2));
        let output = filter.process(malformed);
        let displayed = output
            .events
            .into_iter()
            .flat_map(|event| match event {
                FilteredOutputEvent::Display { bytes, .. } => bytes,
                _ => Vec::new(),
            })
            .collect::<Vec<_>>();
        assert_eq!(displayed, malformed);

        let reserved = filter.process(
            b"\x1b]052;c;not-base64!\x07\x9d00052;c;not-base64!\x9c\xc2\x9d052;c;not-base64!\xc2\x9c\x1b]001337;Copy=;not-base64!\x07",
        );
        assert!(reserved.events.is_empty());
    }

    #[test]
    fn color_state_display_scanning_does_not_add_query_scan_work() {
        let bytes = b"\x1b]10;#123456\x07";
        let mut scanner = TerminalQueryScanner::new_with_work_counter();
        let _ = scanner.process(bytes);
        let scanner_work = scanner.inspected_bytes();

        let mut runtime = TerminalRuntime::new_with_query_scan_work(TerminalSize::new(10, 2));
        runtime.feed_pty_output(bytes);

        assert_eq!(runtime.inspected_query_bytes(), scanner_work);
    }

    #[test]
    fn framed_display_events_only_request_mode_scans_for_mode_controls() {
        let mut filter = TerminalOutputFilter::new(TerminalSize::new(20, 2));
        let mut displays = Vec::new();
        filter.for_each_event(
            b"plain\x1b[31mred\x1b[?1htail\x1b]0;title\x07done\x1b[0m",
            |event| {
                if let FilteredOutputEventRef::Display {
                    bytes, track_modes, ..
                } = event
                {
                    displays.push((bytes.to_vec(), track_modes));
                }
            },
        );

        assert_eq!(
            displays,
            vec![
                (b"plain".to_vec(), false),
                (b"\x1b[31m".to_vec(), false),
                (b"red".to_vec(), false),
                (b"\x1b[?1h".to_vec(), true),
                (b"tail".to_vec(), false),
                (b"\x1b]0;title\x07".to_vec(), false),
                (b"done".to_vec(), false),
                (b"\x1b[0m".to_vec(), false),
            ]
        );
    }

    #[test]
    fn terminal_runtime_reports_active_main_reflow_separately_from_alternate_resize() {
        let mut main = TerminalRuntime::new(TerminalSize::new(8, 2));
        main.feed_pty_output(b"abcdefgh");

        assert_eq!(
            main.resize(TerminalSize::new(6, 2)),
            rterm_terminal::TerminalResizeOutcome::MainScreenReflowed
        );

        let mut alternate = TerminalRuntime::new(TerminalSize::new(8, 2));
        alternate.feed_pty_output(b"\x1b[?1049halt");

        assert_eq!(
            alternate.resize(TerminalSize::new(6, 2)),
            rterm_terminal::TerminalResizeOutcome::AlternateScreenResized
        );
    }

    #[test]
    fn feeds_plain_pty_output_into_terminal_grid() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(10, 2));

        let responses = runtime.feed_pty_output(b"abc");

        assert!(responses.is_empty());
        assert_eq!(
            runtime.terminal().grid().get(0, 0).unwrap().primary_char(),
            'a'
        );
        assert_eq!(
            runtime.terminal().grid().get(0, 1).unwrap().primary_char(),
            'b'
        );
        assert_eq!(
            runtime.terminal().grid().get(0, 2).unwrap().primary_char(),
            'c'
        );
    }

    pub(super) fn replay_task10_fixture(test_name: &str) -> bool {
        task10_registry::replay(test_name)
    }

    #[test]
    fn terminal_runtime_reports_same_chunk_screen_identity_roundtrip() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(10, 2));

        let output = runtime.feed_pty_output_with_display(b"\x1b[?1049halt\x1b[?1049l");

        assert!(output.screen_identity_changed);
        assert_eq!(
            runtime.terminal().stable_dimensions().domain,
            rterm_terminal::TerminalScreenDomain::Main
        );
    }

    #[test]
    fn terminal_runtime_ignores_noop_screen_identity_sets() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(10, 2));

        let output = runtime.feed_pty_output_with_display(b"\x1b[?1049l\x1b[?1049l");

        assert!(!output.screen_identity_changed);
    }

    #[test]
    fn terminal_runtime_reports_reset_and_full_erase_identity_mutations() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(10, 2));

        let reset = runtime.feed_pty_output_with_display(b"\x1bc");
        let erase = runtime.feed_pty_output_with_display(b"\x1b[2J");
        let non_identity = runtime.feed_pty_output_with_display(b"\x1b[J\x1b[1J\x1b[?2J\x1b[3J");

        assert!(reset.screen_identity_changed);
        assert!(erase.screen_identity_changed);
        assert!(!non_identity.screen_identity_changed);
    }

    #[test]
    fn feeds_emoji_prefixed_pty_output_into_terminal_grid() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(24, 2));

        let output = runtime.feed_pty_output_with_display("👍 Process".as_bytes());

        assert!(output.responses.is_empty());
        assert!(
            terminal_text(&runtime).contains("Process"),
            "display={:?} text={:?}",
            String::from_utf8_lossy(&output.display),
            terminal_text(&runtime)
        );
    }

    #[test]
    fn does_not_match_raw_c1_inside_utf8_text() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(24, 2));

        let output = runtime.feed_pty_output_with_display(b"before \xc3\x9b6n after");

        assert!(output.responses.is_empty());
        assert_eq!(output.display, b"before \xc3\x9b6n after");
        assert!(terminal_text(&runtime).contains("before Û6n after"));
    }

    #[test]
    fn does_not_retain_raw_c1_prefix_inside_utf8_text() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(24, 2));

        let output = runtime.feed_pty_output_with_display(b"before \xc3\x9b");

        assert!(output.responses.is_empty());
        assert_eq!(output.display, b"before \xc3\x9b");
        assert!(terminal_text(&runtime).contains("before Û"));
    }

    #[test]
    fn reports_bell_events_without_display_bytes() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(10, 2));

        let output = runtime.feed_pty_output_with_display(b"ab\x07cd\x07");

        assert!(output.responses.is_empty());
        assert_eq!(output.bells, 2);
        assert_eq!(output.display, b"abcd");
        assert_eq!(terminal_text(&runtime), "abcd                ");
    }

    #[test]
    fn reports_damage_regions_from_terminal_feed() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(10, 2));

        let output = runtime.feed_pty_output_with_display(b"abc");

        assert_eq!(
            output.damage,
            vec![rterm_types::DamageRegion::new(0, 0, 3, 1)]
        );
    }

    #[test]
    fn omits_osc_title_from_display_bytes() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let output = runtime.feed_pty_output_with_display(b"before\x1b]0;ops\x07after");

        assert!(output.responses.is_empty());
        assert_eq!(runtime.terminal().title(), Some("ops"));
        assert_eq!(output.display, b"beforeafter");
        assert!(terminal_text(&runtime).contains("beforeafter"));
    }

    #[test]
    fn omits_split_osc_title_from_display_bytes() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let first = runtime.feed_pty_output_with_display(b"before\x1b]0;op");
        let second = runtime.feed_pty_output_with_display(b"s\x07after");

        assert_eq!(first.display, b"before");
        assert_eq!(second.display, b"after");
        assert_eq!(runtime.terminal().title(), Some("ops"));
        assert!(terminal_text(&runtime).contains("beforeafter"));
    }

    #[test]
    fn consumes_wezterm_no_response_window_title_stack_controls_without_side_effects() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let output = runtime.feed_pty_output_with_display(
            b"before\x1b]0;main\x07\x1b[22;0t middle\x1b]0;alternate\x07\x1b[23;2tafter",
        );

        assert!(output.responses.is_empty());
        assert_eq!(output.display, b"before middleafter");
        assert_eq!(runtime.terminal().title(), Some("alternate"));
        assert!(terminal_text(&runtime).contains("before middleafter"));
    }

    #[test]
    fn malformed_window_title_stack_controls_do_not_fall_through_to_terminal_parser() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let output = runtime.feed_pty_output_with_display(
            b"before\x1b]0;main\x07\x1b[22;?t middle\x1b]0;alternate\x07\x1b[23;?tafter",
        );

        assert!(output.responses.is_empty());
        assert_eq!(output.display, b"before middleafter");
        assert_eq!(runtime.terminal().title(), Some("alternate"));
        assert!(terminal_text(&runtime).contains("before middleafter"));
    }

    #[test]
    fn tracks_c1_osc8_hyperlinks_without_displaying_control_bytes() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let output =
            runtime.feed_pty_output_with_display(b"a\x9d8;;https://example.com\x9cbc\x9d8;;\x9cd");

        assert!(output.responses.is_empty());
        assert_eq!(output.display, b"abcd");
        assert_eq!(
            runtime
                .terminal()
                .grid()
                .get(0, 1)
                .unwrap()
                .hyperlink
                .as_deref(),
            Some("https://example.com")
        );
        assert_eq!(
            runtime
                .terminal()
                .grid()
                .get(0, 2)
                .unwrap()
                .hyperlink
                .as_deref(),
            Some("https://example.com")
        );
        assert_eq!(runtime.terminal().grid().get(0, 0).unwrap().hyperlink, None);
        assert_eq!(runtime.terminal().grid().get(0, 3).unwrap().hyperlink, None);
    }

    #[test]
    fn omits_st_controls_without_displaying_control_bytes() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let first = runtime.feed_pty_output_with_display(b"ab\x1b\\cd\x9cef");
        let second = runtime.feed_pty_output_with_display("gh\u{9c}ij".as_bytes());
        let third = runtime.feed_pty_output_with_display(b"kl\x1b");
        let fourth = runtime.feed_pty_output_with_display(b"\\mn");

        assert!(first.responses.is_empty());
        assert_eq!(first.display, b"abcdef");
        assert!(second.responses.is_empty());
        assert_eq!(second.display, b"ghij");
        assert!(third.responses.is_empty());
        assert_eq!(third.display, b"kl");
        assert!(fourth.responses.is_empty());
        assert_eq!(fourth.display, b"mn");
        assert!(terminal_text(&runtime).contains("abcdefghijklmn"));
    }

    #[test]
    fn resynchronizes_queries_after_escape_inside_osc_control_strings() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let output = runtime.feed_pty_output_with_display(b"before\x1b]0;title \x1b[6n\x07after");

        assert_eq!(output.responses, vec![b"\x1b[1;7R".to_vec()]);
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn resynchronizes_split_queries_after_escape_inside_osc_control_strings() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let first = runtime.feed_pty_output_with_display(b"before\x1b]0;title \x1b[");
        let second = runtime.feed_pty_output_with_display(b"6n\x07after");

        assert!(first.responses.is_empty());
        assert_eq!(first.display, b"before");
        assert_eq!(second.responses, vec![b"\x1b[1;7R".to_vec()]);
        assert_eq!(second.display, b"after");
    }

    #[test]
    fn resynchronizes_split_queries_after_escape_inside_dcs_control_strings() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let first = runtime.feed_pty_output_with_display(b"before\x1bPpayload \x1b[");
        let second = runtime.feed_pty_output_with_display(b"6n\x1b\\after");

        assert!(first.responses.is_empty());
        assert_eq!(first.display, b"before");
        assert_eq!(second.responses, vec![b"\x1b[1;7R".to_vec()]);
        assert_eq!(second.display, b"after");
    }

    #[test]
    fn answers_cursor_position_query_without_feeding_it_to_terminal() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let first = runtime.feed_pty_output(b"before\x1b[");
        let second = runtime.feed_pty_output(b"6nafter");

        assert!(first.is_empty());
        assert_eq!(second, vec![b"\x1b[1;7R".to_vec()]);

        let text = terminal_text(&runtime);
        assert!(text.contains("beforeafter"));
        assert!(!text.contains("[6n"));
    }

    #[test]
    fn answers_cursor_position_query_with_current_cursor() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let responses = runtime.feed_pty_output(b"abc\x1b[6n");

        assert_eq!(responses, vec![b"\x1b[1;4R".to_vec()]);
        assert!(terminal_text(&runtime).contains("abc"));
    }

    #[test]
    fn answers_c1_cursor_position_query_without_feeding_it_to_terminal() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let responses = runtime.feed_pty_output(b"abc\x9b6n");

        assert_eq!(responses, vec![b"\x1b[1;4R".to_vec()]);
        assert!(terminal_text(&runtime).contains("abc"));
        assert!(!terminal_text(&runtime).contains("6n"));
    }

    #[test]
    fn consumes_private_cursor_position_queries_like_wezterm() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
        let csi = '\u{9b}';
        let mut input = b"abc\x1b[?6n def".to_vec();
        input.extend_from_slice(b"\x9b?6n ghi");
        input.extend_from_slice(format!("{csi}?6n").as_bytes());

        let responses = runtime.feed_pty_output(&input);

        assert!(responses.is_empty());
        let text = terminal_text(&runtime);
        assert!(text.contains("abc def ghi"));
        assert!(!text.contains("?6n"));
    }

    #[test]
    fn answers_device_and_status_queries_without_feeding_them_to_terminal() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let responses = runtime
            .feed_pty_output(b"a\x1b[c b\x1b[0c c\x1b[>c d\x1b[>0c e\x1b[=c f\x1b[=0c g\x1b[5n h");

        assert_eq!(
            responses,
            vec![
                b"\x1b[?65;4;6;18;22;52c".to_vec(),
                b"\x1b[?65;4;6;18;22;52c".to_vec(),
                b"\x1b[>1;277;0c".to_vec(),
                b"\x1b[>1;277;0c".to_vec(),
                b"\x1bP!|00000000\x1b\\".to_vec(),
                b"\x1bP!|00000000\x1b\\".to_vec(),
                b"\x1b[0n".to_vec()
            ]
        );

        let text = terminal_text(&runtime);
        assert!(text.contains("a b c d e f g h"));
        assert!(!text.contains("[0c"));
        assert!(!text.contains("[>c"));
        assert!(!text.contains("[>0c"));
        assert!(!text.contains("[=c"));
        assert!(!text.contains("[=0c"));
        assert!(!text.contains("[5n"));
    }

    #[test]
    fn answers_c1_device_and_status_queries_without_feeding_them_to_terminal() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let responses =
            runtime.feed_pty_output(b"a\x9bc b\x9b0c c\x9b>c d\x9b>0c e\x9b=c f\x9b=0c g\x9b5n h");

        assert_eq!(
            responses,
            vec![
                b"\x1b[?65;4;6;18;22;52c".to_vec(),
                b"\x1b[?65;4;6;18;22;52c".to_vec(),
                b"\x1b[>1;277;0c".to_vec(),
                b"\x1b[>1;277;0c".to_vec(),
                b"\x1bP!|00000000\x1b\\".to_vec(),
                b"\x1bP!|00000000\x1b\\".to_vec(),
                b"\x1b[0n".to_vec()
            ]
        );
        let text = terminal_text(&runtime);
        assert!(text.contains("a b c d e f g h"));
        assert!(!text.contains("0c"));
        assert!(!text.contains("=c"));
    }

    #[test]
    fn answers_utf8_c1_device_and_status_queries_without_feeding_them_to_terminal() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
        let csi = '\u{9b}';
        let input = format!("a{csi}c b{csi}0c c{csi}>c d{csi}>0c e{csi}=c f{csi}=0c g{csi}5n h");

        let responses = runtime.feed_pty_output(input.as_bytes());

        assert_eq!(
            responses,
            vec![
                b"\x1b[?65;4;6;18;22;52c".to_vec(),
                b"\x1b[?65;4;6;18;22;52c".to_vec(),
                b"\x1b[>1;277;0c".to_vec(),
                b"\x1b[>1;277;0c".to_vec(),
                b"\x1bP!|00000000\x1b\\".to_vec(),
                b"\x1bP!|00000000\x1b\\".to_vec(),
                b"\x1b[0n".to_vec()
            ]
        );
        let text = terminal_text(&runtime);
        assert!(text.contains("a b c d e f g h"));
        assert!(!text.contains("0c"));
        assert!(!text.contains("=c"));
    }

    #[test]
    fn answers_terminal_parameter_queries_without_feeding_them_to_terminal() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let responses = runtime.feed_pty_output(b"a\x1b[x b\x1b[0x c\x1b[1x d");

        assert_eq!(
            responses,
            vec![
                b"\x1b[2;1;1;128;128;1;0x".to_vec(),
                b"\x1b[2;1;1;128;128;1;0x".to_vec(),
                b"\x1b[3;1;1;128;128;1;0x".to_vec()
            ]
        );

        let text = terminal_text(&runtime);
        assert!(text.contains("a b c d"));
        assert!(!text.contains("[x"));
        assert!(!text.contains("[0x"));
        assert!(!text.contains("[1x"));
    }

    #[test]
    fn answers_c1_terminal_parameter_queries_without_feeding_them_to_terminal() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let responses = runtime.feed_pty_output(b"a\x9bx b\x9b0x c\x9b1x d");

        assert_eq!(
            responses,
            vec![
                b"\x1b[2;1;1;128;128;1;0x".to_vec(),
                b"\x1b[2;1;1;128;128;1;0x".to_vec(),
                b"\x1b[3;1;1;128;128;1;0x".to_vec()
            ]
        );

        let text = terminal_text(&runtime);
        assert!(text.contains("a b c d"));
        assert!(!text.contains("0x"));
        assert!(!text.contains("1x"));
    }

    #[test]
    fn answers_utf8_c1_terminal_parameter_queries_without_feeding_them_to_terminal() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
        let csi = '\u{9b}';
        let input = format!("a{csi}x b{csi}0x c{csi}1x d");

        let responses = runtime.feed_pty_output(input.as_bytes());

        assert_eq!(
            responses,
            vec![
                b"\x1b[2;1;1;128;128;1;0x".to_vec(),
                b"\x1b[2;1;1;128;128;1;0x".to_vec(),
                b"\x1b[3;1;1;128;128;1;0x".to_vec()
            ]
        );

        let text = terminal_text(&runtime);
        assert!(text.contains("a b c d"));
        assert!(!text.contains("0x"));
        assert!(!text.contains("1x"));
    }

    #[test]
    fn answers_xtsmgraphics_queries_without_feeding_them_to_terminal() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));

        let responses = runtime.feed_pty_output(
            b"a\x1b[?1;1S b\x1b[?1;4S c\x1b[?2;1S d\x1b[?2;4S e\x1b[?3;1S f\x1b[?3;4S g\x1b[?2;2S h\x1b[?9;1S i\x1b[?1;3;10S j",
        );

        assert_eq!(
            responses,
            vec![
                b"\x1b[?1;0;65536S".to_vec(),
                b"\x1b[?1;0;65536S".to_vec(),
                b"\x1b[?2;0;1056;688S".to_vec(),
                b"\x1b[?2;0;1056;688S".to_vec(),
                b"\x1b[?3;0;1056;688S".to_vec(),
                b"\x1b[?3;0;1056;688S".to_vec(),
                b"\x1b[?2;0S".to_vec(),
                b"\x1b[?9;1S".to_vec(),
                b"\x1b[?1;2S".to_vec()
            ]
        );

        let text = terminal_text(&runtime);
        assert!(text.contains("a b c d e f g h i j"));
        assert!(!text.contains("?1;1S"));
        assert!(!text.contains("?2;4S"));
        assert!(!text.contains("?9;1S"));
    }

    #[test]
    fn answers_xtsmgraphics_large_numeric_parameters_like_wezterm() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));

        let responses =
            runtime.feed_pty_output(b"a\x1b[?70000;1S b\x1b[?1;70000S c\x1b[?1;1;70000S d");

        assert_eq!(
            responses,
            vec![
                b"\x1b[?70000;1S".to_vec(),
                b"\x1b[?1;2S".to_vec(),
                b"\x1b[?1;0;65536S".to_vec()
            ]
        );

        let text = terminal_text(&runtime);
        assert!(text.contains("a b c d"));
        assert!(!text.contains("70000"));
    }

    #[test]
    fn answers_c1_xtsmgraphics_queries_without_feeding_them_to_terminal() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));

        let responses = runtime.feed_pty_output(b"a\x9b?1;1S b\x9b?2;4S c\x9b?9;1S d");

        assert_eq!(
            responses,
            vec![
                b"\x1b[?1;0;65536S".to_vec(),
                b"\x1b[?2;0;1056;688S".to_vec(),
                b"\x1b[?9;1S".to_vec()
            ]
        );

        let text = terminal_text(&runtime);
        assert!(text.contains("a b c d"));
        assert!(!text.contains("?1;1S"));
    }

    #[test]
    fn answers_utf8_c1_xtsmgraphics_queries_without_feeding_them_to_terminal() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));
        let csi = '\u{9b}';
        let input = format!("a{csi}?1;1S b{csi}?2;4S c{csi}?9;1S d");

        let responses = runtime.feed_pty_output(input.as_bytes());

        assert_eq!(
            responses,
            vec![
                b"\x1b[?1;0;65536S".to_vec(),
                b"\x1b[?2;0;1056;688S".to_vec(),
                b"\x1b[?9;1S".to_vec()
            ]
        );

        let text = terminal_text(&runtime);
        assert!(text.contains("a b c d"));
        assert!(!text.contains("?1;1S"));
    }

    #[test]
    fn answers_text_area_size_query() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));

        let responses = runtime.feed_pty_output(b"before\x1b[18tafter");

        assert_eq!(responses, vec![b"\x1b[8;43;132t".to_vec()]);

        let text = terminal_text(&runtime);
        assert!(text.contains("beforeafter"));
        assert!(!text.contains("[18t"));
    }

    #[test]
    fn answers_window_pixel_size_query() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));

        let responses = runtime.feed_pty_output(b"before\x1b[14tafter");

        assert_eq!(responses, vec![b"\x1b[4;688;1056t".to_vec()]);

        let text = terminal_text(&runtime);
        assert!(text.contains("beforeafter"));
        assert!(!text.contains("[14t"));
    }

    #[test]
    fn consumes_window_position_query_without_response() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));

        let responses = runtime.feed_pty_output(b"before\x1b[13tafter");

        assert!(responses.is_empty());

        let text = terminal_text(&runtime);
        assert!(text.contains("beforeafter"));
        assert!(!text.contains("[13t"));
    }

    #[test]
    fn consumes_screen_pixel_size_query_without_response() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));

        let responses = runtime.feed_pty_output(b"before\x1b[15tafter");

        assert!(responses.is_empty());

        let text = terminal_text(&runtime);
        assert!(text.contains("beforeafter"));
        assert!(!text.contains("[15t"));
    }

    #[test]
    fn answers_character_cell_size_query() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));

        let responses = runtime.feed_pty_output(b"before\x1b[16tafter");

        assert_eq!(responses, vec![b"\x1b[6;16;8t".to_vec()]);

        let text = terminal_text(&runtime);
        assert!(text.contains("beforeafter"));
        assert!(!text.contains("[16t"));
    }

    #[test]
    fn answers_wezterm_window_reports_with_empty_and_extra_parameters() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));

        let responses =
            runtime.feed_pty_output(b"before\x1b[14;t middle\x1b[16;0;99t after\x1b[18;1t");

        assert_eq!(
            responses,
            vec![
                b"\x1b[4;688;1056t".to_vec(),
                b"\x1b[6;16;8t".to_vec(),
                b"\x1b[8;43;132t".to_vec()
            ]
        );

        let text = terminal_text(&runtime);
        assert!(text.contains("before middle after"));
        assert!(!text.contains("[14;"));
        assert!(!text.contains("[16;"));
        assert!(!text.contains("[18;"));
    }

    #[test]
    fn answers_iterm_report_cell_size_query() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));

        let responses = runtime.feed_pty_output(b"before\x1b]1337;ReportCellSize\x07after");

        assert_eq!(
            responses,
            vec![b"\x1b]1337;ReportCellSize=16.0;8.0\x1b\\".to_vec()]
        );

        let text = terminal_text(&runtime);
        assert!(text.contains("beforeafter"));
        assert!(!text.contains("ReportCellSize"));
    }

    #[test]
    fn consumes_screen_size_query_without_response() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));

        let responses = runtime.feed_pty_output(b"before\x1b[19tafter");

        assert!(responses.is_empty());

        let text = terminal_text(&runtime);
        assert!(text.contains("beforeafter"));
        assert!(!text.contains("[19t"));
    }

    #[test]
    fn consumes_window_state_query_without_response() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));

        let responses = runtime.feed_pty_output(b"before\x1b[11tafter");

        assert!(responses.is_empty());

        let text = terminal_text(&runtime);
        assert!(text.contains("beforeafter"));
        assert!(!text.contains("[11t"));
    }

    #[test]
    fn consumes_window_title_queries_without_response_by_default() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));

        let responses =
            runtime.feed_pty_output(b"\x1b]0;ops\x07before\x1b[20t middle\x1b[21tafter");

        assert!(responses.is_empty());

        let text = terminal_text(&runtime);
        assert!(text.contains("before middleafter"));
        assert!(!text.contains("[20t"));
        assert!(!text.contains("[21t"));
    }

    #[test]
    fn answers_enq_with_configured_answerback() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
        runtime.set_enq_answerback("rssh");

        let output = runtime.feed_pty_output_with_display(b"before\x05after");

        assert_eq!(output.responses, vec![b"rssh".to_vec()]);
        assert_eq!(output.display, b"beforeafter");
        assert!(terminal_text(&runtime).contains("beforeafter"));
    }

    #[test]
    fn ignores_enq_inside_osc_control_string() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
        runtime.set_enq_answerback("rssh");

        let first = runtime.feed_pty_output_with_display(b"before\x1b]0;op\x05");
        let second = runtime.feed_pty_output_with_display(b"s\x07after\x05");

        assert!(first.responses.is_empty());
        assert_eq!(second.responses, vec![b"rssh".to_vec()]);
        assert_eq!(first.display, b"before");
        assert_eq!(second.display, b"after");
    }

    #[test]
    fn answers_window_title_query_when_title_reporting_enabled() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));
        runtime.set_enable_title_reporting(true);

        let responses = runtime.feed_pty_output(b"\x1b]0;ops\x07before\x1b[21tafter");

        assert_eq!(responses, vec![b"\x1b]lops\x1b\\".to_vec()]);
        let text = terminal_text(&runtime);
        assert!(text.contains("beforeafter"));
        assert!(!text.contains("[21t"));
    }

    #[test]
    fn consumes_decrqcra_without_response_by_default() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(10, 2));

        let responses = runtime.feed_pty_output(b"ABC\x1b[7;1;1;1;1;3*yDEF");

        assert!(responses.is_empty());
        let text = terminal_text(&runtime);
        assert!(text.contains("ABCDEF"));
        assert!(!text.contains("[7;"));
    }

    #[test]
    fn answers_decrqcra_when_checksum_rectangular_area_enabled() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(10, 2));
        runtime.set_enable_checksum_rectangular_area(true);

        let responses = runtime.feed_pty_output(b"ABC\x1b[7;1;1;1;1;3*y");

        assert_eq!(responses, vec![b"\x1bP7!~00c6\x1b\\".to_vec()]);
        assert!(terminal_text(&runtime).starts_with("ABC"));
    }

    #[test]
    fn answers_c1_terminal_size_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));

        let text_area = runtime.feed_pty_output(b"\x9b18t");
        let screen = runtime.feed_pty_output(b"\x9b19t");

        assert_eq!(text_area, vec![b"\x1b[8;43;132t".to_vec()]);
        assert!(screen.is_empty());
    }

    #[test]
    fn answers_c1_window_pixel_and_cell_size_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));

        let window_pixels = runtime.feed_pty_output(b"\x9b14t");
        let cell_pixels = runtime.feed_pty_output(b"\x9b16t");

        assert_eq!(window_pixels, vec![b"\x1b[4;688;1056t".to_vec()]);
        assert_eq!(cell_pixels, vec![b"\x1b[6;16;8t".to_vec()]);
    }

    #[test]
    fn consumes_c1_window_position_and_screen_pixel_size_queries_without_response() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));

        let window_position = runtime.feed_pty_output(b"\x9b13t");
        let screen_pixels = runtime.feed_pty_output(b"\x9b15t");

        assert!(window_position.is_empty());
        assert!(screen_pixels.is_empty());
    }

    #[test]
    fn consumes_c1_window_state_and_title_queries_without_response_by_default() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));

        runtime.feed_pty_output(b"\x1b]0;ops\x07");
        let state = runtime.feed_pty_output(b"\x9b11t");
        let icon = runtime.feed_pty_output(b"\x9b20t");
        let title = runtime.feed_pty_output(b"\x9b21t");

        assert!(state.is_empty());
        assert!(icon.is_empty());
        assert!(title.is_empty());
    }

    #[test]
    fn answers_private_mode_status_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let responses = runtime
            .feed_pty_output(b"before\x1b[?1h\x1b[?1$p middle\x1b[?1004$p after\x1b[?9999$p");

        assert_eq!(
            responses,
            vec![
                b"\x1b[?1;1$y".to_vec(),
                b"\x1b[?1004;2$y".to_vec(),
                b"\x1b[?9999;0$y".to_vec()
            ]
        );

        let text = terminal_text(&runtime);
        assert!(text.contains("before middle after"));
        assert!(!text.contains("$p"));
    }

    #[test]
    fn answers_display_private_mode_status_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let responses = runtime.feed_pty_output(
            b"\x1b[?1034$p \x1b[?1034h\x1b[?1034$p\x1b[?1034l\x1b[?1034$p \
              \x1b[?12$p \x1b[?12h\x1b[?12$p\x1b[?12l\x1b[?12$p \
              \x1b[?7$p \x1b[?7l\x1b[?7$p \
              \x1b[?25$p \x1b[?25l\x1b[?25$p \
              \x1b[?45$p \x1b[?45h\x1b[?45$p\x1b[?45l\x1b[?45$p \
              \x1b[?6$p \x1b[?6h\x1b[?6$p \
              \x1b[?80$p \x1b[?80h\x1b[?80$p\x1b[?80l\x1b[?80$p \
              \x1b[?8452$p \x1b[?8452h\x1b[?8452$p\x1b[?8452l\x1b[?8452$p \
              \x1b[?47$p \x1b[?47h\x1b[?47$p\x1b[?47l\x1b[?47$p \
              \x1b[?1048$p \x1b[?1048h\x1b[?1048$p\x1b[?1048l\x1b[?1048$p \
              \x1b[?1047$p \x1b[?1047h\x1b[?1047$p\x1b[?1047l\x1b[?1047$p \
              \x1b[?1049$p \x1b[?1049h\x1b[?1049$p\x1b[?1049l\x1b[?1049$p",
        );

        assert_eq!(
            responses,
            vec![
                b"\x1b[?1034;2$y".to_vec(),
                b"\x1b[?1034;1$y".to_vec(),
                b"\x1b[?1034;2$y".to_vec(),
                b"\x1b[?12;2$y".to_vec(),
                b"\x1b[?12;1$y".to_vec(),
                b"\x1b[?12;2$y".to_vec(),
                b"\x1b[?7;1$y".to_vec(),
                b"\x1b[?7;2$y".to_vec(),
                b"\x1b[?25;1$y".to_vec(),
                b"\x1b[?25;2$y".to_vec(),
                b"\x1b[?45;2$y".to_vec(),
                b"\x1b[?45;1$y".to_vec(),
                b"\x1b[?45;2$y".to_vec(),
                b"\x1b[?6;2$y".to_vec(),
                b"\x1b[?6;1$y".to_vec(),
                b"\x1b[?80;2$y".to_vec(),
                b"\x1b[?80;1$y".to_vec(),
                b"\x1b[?80;2$y".to_vec(),
                b"\x1b[?8452;2$y".to_vec(),
                b"\x1b[?8452;1$y".to_vec(),
                b"\x1b[?8452;2$y".to_vec(),
                b"\x1b[?47;0$y".to_vec(),
                b"\x1b[?47;0$y".to_vec(),
                b"\x1b[?47;0$y".to_vec(),
                b"\x1b[?1048;0$y".to_vec(),
                b"\x1b[?1048;0$y".to_vec(),
                b"\x1b[?1048;0$y".to_vec(),
                b"\x1b[?1047;0$y".to_vec(),
                b"\x1b[?1047;0$y".to_vec(),
                b"\x1b[?1047;0$y".to_vec(),
                b"\x1b[?1049;0$y".to_vec(),
                b"\x1b[?1049;0$y".to_vec(),
                b"\x1b[?1049;0$y".to_vec(),
            ]
        );
    }

    #[test]
    fn reports_wezterm_unknown_alternate_screen_private_mode_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let responses = runtime.feed_pty_output(
            b"\x1b[?47$p\x1b[?47h\x1b[?47$p\
              \x1b[?1047$p\x1b[?1047h\x1b[?1047$p\
              \x1b[?1048$p\x1b[?1048h\x1b[?1048$p\
              \x1b[?1049$p\x1b[?1049h\x1b[?1049$p",
        );

        assert_eq!(
            responses,
            vec![
                b"\x1b[?47;0$y".to_vec(),
                b"\x1b[?47;0$y".to_vec(),
                b"\x1b[?1047;0$y".to_vec(),
                b"\x1b[?1047;0$y".to_vec(),
                b"\x1b[?1048;0$y".to_vec(),
                b"\x1b[?1048;0$y".to_vec(),
                b"\x1b[?1049;0$y".to_vec(),
                b"\x1b[?1049;0$y".to_vec(),
            ]
        );
    }

    #[test]
    fn answers_declrmm_mode_status_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let responses =
            runtime.feed_pty_output(b"\x1b[?69$p\x1b[?69h\x1b[?69$p\x1b[?69l\x1b[?69$p");

        assert_eq!(
            responses,
            vec![
                b"\x1b[?69;2$y".to_vec(),
                b"\x1b[?69;1$y".to_vec(),
                b"\x1b[?69;2$y".to_vec(),
            ]
        );
    }

    #[test]
    fn answers_wezterm_private_mode_reports() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let responses = runtime.feed_pty_output(
            b"\x1b[?3$p \x1b[?2027$p \x1b[?2027l\x1b[?2027$p \
              \x1b[?1070$p \x1b[?1070h\x1b[?1070$p\x1b[?1070l\x1b[?1070$p",
        );

        assert_eq!(
            responses,
            vec![
                b"\x1b[?3;2$y".to_vec(),
                b"\x1b[?2027;3$y".to_vec(),
                b"\x1b[?2027;3$y".to_vec(),
                b"\x1b[?1070;2$y".to_vec(),
                b"\x1b[?1070;1$y".to_vec(),
                b"\x1b[?1070;2$y".to_vec(),
            ]
        );
    }

    #[test]
    fn answers_dec_ansi_mode_status_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let responses = runtime.feed_pty_output(b"\x1b[?2$p\x1b[?2h\x1b[?2$p\x1b[?2l\x1b[?2$p");

        assert_eq!(
            responses,
            vec![
                b"\x1b[?2;2$y".to_vec(),
                b"\x1b[?2;1$y".to_vec(),
                b"\x1b[?2;2$y".to_vec(),
            ]
        );
    }

    #[test]
    fn answers_screen_reverse_mode_status_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output =
            runtime.feed_pty_output_with_display(b"\x1b[?5$p\x1b[?5h\x1b[?5$p\x1b[?5l\x1b[?5$p");

        assert_eq!(
            output.responses,
            vec![
                b"\x1b[?5;2$y".to_vec(),
                b"\x1b[?5;1$y".to_vec(),
                b"\x1b[?5;2$y".to_vec(),
            ]
        );
        assert!(output.display.is_empty());
    }

    #[test]
    fn answers_private_mode_status_defaults_after_terminal_reset() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let responses = runtime.feed_pty_output(
            b"\x1b[?1;6;25;47;1048;1049;1000;1006;1004;2004;2026h\x1b[?7l\x1b=\x1bc\
              \x1b[?1$p\x1b[?6$p\x1b[?7$p\x1b[?25$p\x1b[?47$p\x1b[?1048$p\
              \x1b[?1049$p\x1b[?1000$p\x1b[?1006$p\x1b[?1004$p\x1b[?2004$p\x1b[?2026$p",
        );

        assert_eq!(
            responses,
            vec![
                b"\x1b[?1;2$y".to_vec(),
                b"\x1b[?6;2$y".to_vec(),
                b"\x1b[?7;1$y".to_vec(),
                b"\x1b[?25;1$y".to_vec(),
                b"\x1b[?47;0$y".to_vec(),
                b"\x1b[?1048;0$y".to_vec(),
                b"\x1b[?1049;0$y".to_vec(),
                b"\x1b[?1000;2$y".to_vec(),
                b"\x1b[?1006;2$y".to_vec(),
                b"\x1b[?1004;2$y".to_vec(),
                b"\x1b[?2004;2$y".to_vec(),
                b"\x1b[?2026;2$y".to_vec(),
            ]
        );
    }

    #[test]
    fn answers_ansi_mode_status_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output = runtime.feed_pty_output_with_display(
            b"before\x1b[4$p \x1b[4h\x1b[4$p \x1b[4l\x1b[4$p \x1b[999$p",
        );

        assert_eq!(
            output.responses,
            vec![
                b"\x1b[4;2$y".to_vec(),
                b"\x1b[4;1$y".to_vec(),
                b"\x1b[4;2$y".to_vec(),
                b"\x1b[999;0$y".to_vec(),
            ]
        );
        assert_eq!(output.display, b"before   ");
        assert!(!terminal_text(&runtime).contains("$p"));
    }

    #[test]
    fn answers_automatic_newline_mode_status_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output = runtime
            .feed_pty_output_with_display(b"before\x1b[20$p \x1b[20h\x1b[20$p \x1b[20l\x1b[20$p");

        assert_eq!(
            output.responses,
            vec![
                b"\x1b[20;2$y".to_vec(),
                b"\x1b[20;1$y".to_vec(),
                b"\x1b[20;2$y".to_vec(),
            ]
        );
        assert_eq!(output.display, b"before  ");
        assert!(!terminal_text(&runtime).contains("$p"));
    }

    #[test]
    fn answers_bidirectional_support_mode_status_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output =
            runtime.feed_pty_output_with_display(b"before\x1b[8$p \x1b[8h\x1b[8$p \x1b[8l\x1b[8$p");

        assert_eq!(
            output.responses,
            vec![
                b"\x1b[8;2$y".to_vec(),
                b"\x1b[8;1$y".to_vec(),
                b"\x1b[8;2$y".to_vec(),
            ]
        );
        assert_eq!(output.display, b"before  ");
        assert!(!terminal_text(&runtime).contains("$p"));
    }

    #[test]
    fn answers_mode_status_after_soft_terminal_reset() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let responses =
            runtime.feed_pty_output(b"\x1b[?6h\x1b[4h\x1b[?6$p\x1b[4$p\x1b[!p\x1b[?6$p\x1b[4$p");

        assert_eq!(
            responses,
            vec![
                b"\x1b[?6;1$y".to_vec(),
                b"\x1b[4;1$y".to_vec(),
                b"\x1b[?6;2$y".to_vec(),
                b"\x1b[4;2$y".to_vec(),
            ]
        );
    }

    #[test]
    fn answers_bidirectional_support_mode_status_after_soft_terminal_reset() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let responses = runtime.feed_pty_output(b"\x1b[8h\x1b[8$p\x1b[!p\x1b[8$p");

        assert_eq!(
            responses,
            vec![b"\x1b[8;1$y".to_vec(), b"\x1b[8;2$y".to_vec()]
        );
    }

    #[test]
    fn answers_auto_wrap_mode_status_after_soft_terminal_reset() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let responses = runtime.feed_pty_output(b"\x1b[?7l\x1b[?7$p\x1b[!p\x1b[?7$p");

        assert_eq!(
            responses,
            vec![b"\x1b[?7;2$y".to_vec(), b"\x1b[?7;1$y".to_vec()]
        );
    }

    #[test]
    fn answers_cursor_visibility_status_after_soft_terminal_reset() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let responses = runtime.feed_pty_output(b"\x1b[?25l\x1b[?25$p\x1b[!p\x1b[?25$p");

        assert_eq!(
            responses,
            vec![b"\x1b[?25;2$y".to_vec(), b"\x1b[?25;1$y".to_vec()]
        );
    }

    #[test]
    fn answers_reverse_wrap_mode_status_after_soft_terminal_reset() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let responses = runtime.feed_pty_output(b"\x1b[?45h\x1b[?45$p\x1b[!p\x1b[?45$p");

        assert_eq!(
            responses,
            vec![b"\x1b[?45;1$y".to_vec(), b"\x1b[?45;2$y".to_vec()]
        );
    }

    #[test]
    fn answers_screen_reverse_mode_status_after_soft_terminal_reset() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let responses = runtime.feed_pty_output(b"\x1b[?5h\x1b[?5$p\x1b[!p\x1b[?5$p");

        assert_eq!(
            responses,
            vec![b"\x1b[?5;1$y".to_vec(), b"\x1b[?5;2$y".to_vec()]
        );
    }

    #[test]
    fn restores_wezterm_input_modes_after_soft_terminal_reset() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        runtime.feed_pty_output(b"\x1b[?1h\x1b=\x1b[>4;2m");
        assert!(runtime.application_cursor_keys());
        assert!(runtime.application_keypad());
        assert_eq!(runtime.modify_other_keys(), 2);

        runtime.feed_pty_output(b"\x1b[!p");

        assert!(!runtime.application_cursor_keys());
        assert!(!runtime.application_keypad());
        assert_eq!(runtime.modify_other_keys(), 0);
        assert_eq!(
            runtime.feed_pty_output(b"\x1b[?1$p\x1b[?4m"),
            vec![b"\x1b[?1;2$y".to_vec(), b"\x1b[>4;0m".to_vec()]
        );
    }

    #[test]
    fn answers_c1_mode_status_after_soft_terminal_reset() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let responses = runtime.feed_pty_output(b"\x9b?6h\x9b4h\x9b!p\x9b?6$p\x9b4$p");

        assert_eq!(
            responses,
            vec![b"\x1b[?6;2$y".to_vec(), b"\x1b[4;2$y".to_vec()]
        );
    }

    #[test]
    fn answers_c1_cursor_blink_mode_status_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let responses = runtime.feed_pty_output(b"\x9b?12$p\x9b?12h\x9b?12$p");

        assert_eq!(
            responses,
            vec![b"\x1b[?12;2$y".to_vec(), b"\x1b[?12;1$y".to_vec()]
        );
    }

    #[test]
    fn answers_c1_meta_key_mode_status_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let responses = runtime.feed_pty_output(b"\x9b?1034$p\x9b?1034h\x9b?1034$p");

        assert_eq!(
            responses,
            vec![b"\x1b[?1034;2$y".to_vec(), b"\x1b[?1034;1$y".to_vec()]
        );
    }

    #[test]
    fn answers_c1_ansi_mode_status_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        runtime.feed_pty_output(b"\x9b4h");
        let responses = runtime.feed_pty_output(b"\x9b4$p");

        assert_eq!(responses, vec![b"\x1b[4;1$y".to_vec()]);
    }

    #[test]
    fn answers_c1_automatic_newline_mode_status_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        runtime.feed_pty_output(b"\x9b20h");
        let responses = runtime.feed_pty_output(b"\x9b20$p");

        assert_eq!(responses, vec![b"\x1b[20;1$y".to_vec()]);
    }

    #[test]
    fn answers_c1_bidirectional_support_mode_status_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        runtime.feed_pty_output(b"\x9b8h");
        let responses = runtime.feed_pty_output(b"\x9b8$p");

        assert_eq!(responses, vec![b"\x1b[8;1$y".to_vec()]);
    }

    #[test]
    fn answers_osc_color_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output = runtime.feed_pty_output_with_display(
            b"before\x1b]10;?\x07 middle\x1b]11;?\x1b\\ after\x1b]4;1;?\x07done",
        );

        assert_eq!(
            output.responses,
            vec![
                b"\x1b]10;rgb:e5e5/e5e5/e5e5\x07".to_vec(),
                b"\x1b]11;rgb:0c0c/0c0c/0c0c\x1b\\".to_vec(),
                b"\x1b]4;1;rgb:cdcd/3131/3131\x07".to_vec()
            ]
        );
        assert_eq!(output.display, b"before middle afterdone");

        let text = terminal_text(&runtime);
        assert!(text.contains("before middle afterdone"));
        assert!(!text.contains("10;?"));
        assert!(!text.contains("11;?"));
        assert!(!text.contains("4;1;?"));
    }

    #[test]
    fn answers_c1_osc_color_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let responses = runtime.feed_pty_output(b"\x9d4;196;?\x9c");

        assert_eq!(
            responses,
            vec![b"\x1b]4;196;rgb:ffff/0000/0000\x9c".to_vec()]
        );
    }

    #[test]
    fn answers_utf8_c1_osc_color_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output = runtime.feed_pty_output_with_display("\u{9d}4;196;?\u{9c}".as_bytes());

        assert_eq!(
            output.responses,
            vec![b"\x1b]4;196;rgb:ffff/0000/0000\x9c".to_vec()]
        );
        assert!(output.display.is_empty());
    }

    #[test]
    fn answers_cursor_color_queries_after_changes_and_reset() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output = runtime.feed_pty_output_with_display(
            b"before\x1b]12;rgb:aa/bb/cc\x07 middle\x1b]12;?\x07 after\x1b]112\x07 reset\x1b]12;?\x1b\\done",
        );

        assert_eq!(
            output.responses,
            vec![
                b"\x1b]12;rgb:aaaa/bbbb/cccc\x07".to_vec(),
                b"\x1b]12;rgb:e5e5/e5e5/e5e5\x1b\\".to_vec(),
            ]
        );
        assert_eq!(output.display, b"before middle after resetdone");
    }

    #[test]
    fn answers_c1_cursor_color_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output = runtime.feed_pty_output_with_display(b"\x9d12;rgb:01/02/03\x9c\x9d12;?\x9c");

        assert_eq!(
            output.responses,
            vec![b"\x1b]12;rgb:0101/0202/0303\x9c".to_vec()]
        );
        assert!(output.display.is_empty());
    }

    #[test]
    fn answers_osc_color_queries_after_color_changes() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output = runtime.feed_pty_output_with_display(
            b"before\x1b]10;rgb:11/22/33\x07 middle\x1b]10;?\x07 after\x1b]4;1;rgb:01/02/03\x1b\\ done\x1b]4;1;?\x1b\\",
        );

        assert_eq!(
            output.responses,
            vec![
                b"\x1b]10;rgb:1111/2222/3333\x07".to_vec(),
                b"\x1b]4;1;rgb:0101/0202/0303\x1b\\".to_vec()
            ]
        );
        assert_eq!(output.display, b"before middle after done");
    }

    #[test]
    fn applies_hex_osc_color_changes() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output = runtime.feed_pty_output_with_display(
            b"\x1b]10;#112233\x07\x1b]4;2;#445566\x07\x1b]10;?\x07\x1b]4;2;?\x07",
        );

        assert_eq!(
            output.responses,
            vec![
                b"\x1b]10;rgb:1111/2222/3333\x07".to_vec(),
                b"\x1b]4;2;rgb:4444/5555/6666\x07".to_vec(),
            ]
        );
        assert!(output.display.is_empty());
    }

    #[test]
    fn applies_rgba_osc_dynamic_color_changes() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output = runtime.feed_pty_output_with_display(
            b"\x1b]10;rgba(127,127,127,0.4)\x07\
              \x1b]11;rgba:efff/ecff/f4ff/d000\x1b\\\
              \x1b]12;rgba(1,2,3,1)\x07\
              \x1b]10;?\x07\x1b]11;?\x1b\\\x1b]12;?\x07",
        );

        assert_eq!(
            output.responses,
            vec![
                b"\x1b]10;rgba:7f7f/7f7f/7f7f/6666\x07".to_vec(),
                b"\x1b]11;rgba:efff/ecff/f4ff/d000\x1b\\".to_vec(),
                b"\x1b]12;rgba:0101/0202/0303/ffff\x07".to_vec(),
            ]
        );
        assert!(output.display.is_empty());
    }

    #[test]
    fn applies_multiple_palette_color_changes_from_one_osc4_sequence() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output = runtime.feed_pty_output_with_display(
            b"\x1b]4;1;rgb:01/02/03;2;rgb:04/05/06\x07\
              \x1b]4;1;?\x07\x1b]4;2;?\x07",
        );

        assert_eq!(
            output.responses,
            vec![
                b"\x1b]4;1;rgb:0101/0202/0303\x07".to_vec(),
                b"\x1b]4;2;rgb:0404/0505/0606\x07".to_vec(),
            ]
        );
        assert!(output.display.is_empty());
    }

    #[test]
    fn terminal_runtime_osc_palette_change_marks_active_domain_rows_changed() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(8, 2));
        runtime.feed_pty_output(b"one\r\ntwo");
        let before = runtime.terminal().current_seqno();
        let visible = runtime.terminal().viewport_stable_range(None);

        runtime.feed_pty_output(b"\x1b]4;1;rgb:01/02/03\x07");

        assert_eq!(
            runtime
                .terminal()
                .changed_stable_rows_since(visible.clone(), before),
            visible.collect::<Vec<_>>()
        );
    }

    #[test]
    fn terminal_runtime_osc_palette_change_advances_sequence_once() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(8, 2));
        runtime.feed_pty_output(b"one\r\ntwo");
        let before = runtime.terminal().current_seqno();

        runtime.feed_pty_output(b"\x1b]4;1;rgb:01/02/03\x07");

        assert_eq!(
            runtime.terminal().current_seqno(),
            before.checked_add(1).unwrap()
        );
    }

    #[test]
    fn terminal_runtime_palette_query_and_noop_do_not_mark_lines_changed() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(8, 2));
        runtime.feed_pty_output(b"one\r\ntwo");
        runtime.feed_pty_output(b"\x1b]4;1;rgb:01/02/03\x07");
        let before = runtime.terminal().current_seqno();
        let visible = runtime.terminal().viewport_stable_range(None);

        runtime.feed_pty_output(b"\x1b]4;1;?\x07");
        runtime.feed_pty_output(b"\x1b]4;1;rgb:01/02/03\x07");

        assert_eq!(
            runtime.terminal().current_seqno(),
            before.checked_add(1).unwrap()
        );
        assert!(
            runtime
                .terminal()
                .changed_stable_rows_since(visible, before)
                .is_empty()
        );
    }

    #[test]
    fn terminal_runtime_palette_default_override_and_reset_are_effective_noops() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(8, 2));
        runtime.feed_pty_output(b"one\r\ntwo");
        let rows = runtime.terminal().viewport_stable_range(None);

        let before_set = runtime.terminal().current_seqno();
        runtime.feed_pty_output(b"\x1b]4;1;rgb:cd/31/31\x07");
        assert_eq!(
            runtime.terminal().current_seqno(),
            before_set.checked_add(1).unwrap()
        );
        assert!(
            runtime
                .terminal()
                .changed_stable_rows_since(rows.clone(), before_set)
                .is_empty()
        );

        let before_reset = runtime.terminal().current_seqno();
        runtime.feed_pty_output(b"\x1b]104;1\x07");
        assert_eq!(
            runtime.terminal().current_seqno(),
            before_reset.checked_add(1).unwrap()
        );
        assert!(
            runtime
                .terminal()
                .changed_stable_rows_since(rows, before_reset)
                .is_empty()
        );
    }

    #[test]
    fn terminal_runtime_cursor_default_override_and_reset_are_effective_noops() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(8, 2));
        runtime.feed_pty_output(b"one\r\ntwo");
        let rows = runtime.terminal().viewport_stable_range(None);

        let before_set = runtime.terminal().current_seqno();
        runtime.feed_pty_output(b"\x1b]12;rgb:e5/e5/e5\x07");
        assert_eq!(
            runtime.terminal().current_seqno(),
            before_set.checked_add(1).unwrap()
        );
        assert!(
            runtime
                .terminal()
                .changed_stable_rows_since(rows.clone(), before_set)
                .is_empty()
        );

        let before_reset = runtime.terminal().current_seqno();
        runtime.feed_pty_output(b"\x1b]112\x07");
        assert_eq!(
            runtime.terminal().current_seqno(),
            before_reset.checked_add(1).unwrap()
        );
        assert!(
            runtime
                .terminal()
                .changed_stable_rows_since(rows, before_reset)
                .is_empty()
        );
    }

    #[test]
    fn answers_multiple_palette_color_queries_from_one_osc4_sequence() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output = runtime.feed_pty_output_with_display(
            b"\x1b]4;1;rgb:01/02/03;2;rgb:04/05/06\x07\
              \x1b]4;1;?;2;?\x07",
        );

        assert_eq!(
            output.responses,
            vec![b"\x1b]4;1;rgb:0101/0202/0303\x07\x1b]4;2;rgb:0404/0505/0606\x07".to_vec()]
        );
        assert!(output.display.is_empty());
    }

    #[test]
    fn resets_dynamic_and_palette_colors() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output = runtime.feed_pty_output_with_display(
            b"before\
              \x1b]10;rgb:11/22/33\x07\x1b]11;rgb:44/55/66\x07\
              \x1b]4;1;rgb:01/02/03\x07\
              \x1b]10;?\x07\x1b]11;?\x07\x1b]4;1;?\x07\
              \x1b]110\x07\x1b]111\x07\x1b]104;1\x07\
              \x1b]10;?\x07\x1b]11;?\x07\x1b]4;1;?\x07after",
        );

        assert_eq!(
            output.responses,
            vec![
                b"\x1b]10;rgb:1111/2222/3333\x07".to_vec(),
                b"\x1b]11;rgb:4444/5555/6666\x07".to_vec(),
                b"\x1b]4;1;rgb:0101/0202/0303\x07".to_vec(),
                b"\x1b]10;rgb:e5e5/e5e5/e5e5\x07".to_vec(),
                b"\x1b]11;rgb:0c0c/0c0c/0c0c\x07".to_vec(),
                b"\x1b]4;1;rgb:cdcd/3131/3131\x07".to_vec(),
            ]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn resets_all_palette_colors() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output = runtime.feed_pty_output_with_display(
            b"\x1b]4;1;rgb:01/02/03\x07\x1b]4;2;rgb:04/05/06\x07\
              \x1b]104\x07\x1b]4;1;?\x07\x1b]4;2;?\x07",
        );

        assert_eq!(
            output.responses,
            vec![
                b"\x1b]4;1;rgb:cdcd/3131/3131\x07".to_vec(),
                b"\x1b]4;2;rgb:0d0d/bcbc/7979\x07".to_vec(),
            ]
        );
        assert!(output.display.is_empty());
    }

    #[test]
    fn resets_multiple_palette_colors() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output = runtime.feed_pty_output_with_display(
            b"\x1b]4;1;rgb:01/02/03\x07\x1b]4;2;rgb:04/05/06\x07\x1b]4;3;rgb:07/08/09\x07\
              \x1b]104;1;2\x07\x1b]4;1;?\x07\x1b]4;2;?\x07\x1b]4;3;?\x07",
        );

        assert_eq!(
            output.responses,
            vec![
                b"\x1b]4;1;rgb:cdcd/3131/3131\x07".to_vec(),
                b"\x1b]4;2;rgb:0d0d/bcbc/7979\x07".to_vec(),
                b"\x1b]4;3;rgb:0707/0808/0909\x07".to_vec(),
            ]
        );
        assert!(output.display.is_empty());
    }

    #[test]
    fn resynchronizes_osc_color_changes_after_escape_inside_dcs_control_strings() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output = runtime.feed_pty_output_with_display(
            b"\x1bPpayload \x1b]10;rgb:11/22/33\x1b\\ after\x1b]10;?\x07",
        );

        assert_eq!(
            output.responses,
            vec![b"\x1b]10;rgb:1111/2222/3333\x07".to_vec()]
        );
        assert_eq!(output.display, b" after");
    }

    #[test]
    fn resynchronizes_split_osc_color_changes_after_escape_inside_dcs_control_strings() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let first = runtime.feed_pty_output_with_display(b"\x1bPpayload ");
        let second =
            runtime.feed_pty_output_with_display(b"\x1b]10;rgb:11/22/33\x1b\\ after\x1b]10;?\x07");

        assert!(first.responses.is_empty());
        assert!(first.display.is_empty());
        assert_eq!(
            second.responses,
            vec![b"\x1b]10;rgb:1111/2222/3333\x07".to_vec()]
        );
        assert_eq!(second.display, b" after");
    }

    #[test]
    fn answers_xtgettcap_queries_without_feeding_them_to_terminal() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output = runtime.feed_pty_output_with_display(
            b"before\x1bP+q436f\x1b\\ middle\x90+q544e;524742;6e616d65\x9c after\x1bP+q666f6f\x1b\\done",
        );

        assert_eq!(
            output.responses,
            vec![
                b"\x1bP1+r436F=323536\x1b\\".to_vec(),
                b"\x1bP1+r544E=787465726D2D323536636F6C6F72\x1b\\\x1bP1+r524742=382F382F38\x1b\\\x1bP1+r6E616D65=787465726D2D323536636F6C6F72\x1b\\".to_vec(),
                b"\x1bP0+r666F6F\x1b\\".to_vec(),
            ]
        );
        assert_eq!(output.display, b"before middle afterdone");

        let text = terminal_text(&runtime);
        assert!(text.contains("before middle afterdone"));
        assert!(!text.contains("+q"));
    }

    #[test]
    fn answers_xtgettcap_invalid_hex_names_like_wezterm() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let first = runtime.feed_pty_output_with_display(b"before\x1bP+qZZ;544e");
        let second = runtime.feed_pty_output_with_display(b";5\x1b\\after");

        assert!(first.responses.is_empty());
        assert_eq!(first.display, b"before");
        assert_eq!(
            second.responses,
            vec![
                b"\x1bP0+r5A5A\x1b\\\x1bP1+r544E=787465726D2D323536636F6C6F72\x1b\\\x1bP0+r35\x1b\\"
                    .to_vec()
            ]
        );
        assert_eq!(second.display, b"after");
    }

    #[test]
    fn answers_xtgettcap_non_utf8_hex_names_like_wezterm() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output = runtime.feed_pty_output_with_display(b"before\x1bP+qff;436f\x1b\\after");

        assert_eq!(
            output.responses,
            vec![b"\x1bP0+rEFBFBD\x1b\\\x1bP1+r436F=323536\x1b\\".to_vec()]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_utf8_c1_dcs_xtgettcap_queries_without_feeding_them_to_terminal() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let dcs = '\u{90}';
        let st = '\u{9c}';
        let input = format!("before{dcs}+q436f{st}after");

        let output = runtime.feed_pty_output_with_display(input.as_bytes());

        assert_eq!(
            output.responses,
            vec![b"\x1bP1+r436F=323536\x1b\\".to_vec()]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_size_queries_from_current_terminal_size() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));

        let output = runtime.feed_pty_output_with_display(b"before\x1bP+q636f;6c69\x1b\\after");

        assert_eq!(
            output.responses,
            vec![b"\x1bP1+r636F=313332\x1b\\\x1bP1+r6C69=3433\x1b\\".to_vec()]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_official_numeric_capability_names() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(132, 43));
        let query = xtgettcap_query(&[
            b"cols".as_slice(),
            b"lines".as_slice(),
            b"it".as_slice(),
            b"pairs".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        let output = runtime.feed_pty_output_with_display(&input);

        assert_eq!(
            output.responses,
            vec![xtgettcap_response(&[
                (b"cols".as_slice(), b"132".as_slice()),
                (b"lines".as_slice(), b"43".as_slice()),
                (b"it".as_slice(), b"8".as_slice()),
                (b"pairs".as_slice(), b"32767".as_slice()),
            ])]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_modern_style_and_color_capabilities() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let query = xtgettcap_query(&[
            b"Tc".as_slice(),
            b"Smulx".as_slice(),
            b"Setulc".as_slice(),
            b"sitm".as_slice(),
            b"ritm".as_slice(),
            b"Smol".as_slice(),
            b"smxx".as_slice(),
            b"rmxx".as_slice(),
            b"op".as_slice(),
            b"oc".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        let output = runtime.feed_pty_output_with_display(&input);

        assert_eq!(
            output.responses,
            vec![xtgettcap_response(&[
                (b"Tc".as_slice(), b"1".as_slice()),
                (b"Smulx".as_slice(), b"\x1b[4:%p1%dm".as_slice()),
                (
                    b"Setulc".as_slice(),
                    b"\x1b[58:2::%p1%{65536}%/%d:%p1%{256}%/%{255}%&%d:%p1%{255}%&%d%;m".as_slice()
                ),
                (b"sitm".as_slice(), b"\x1b[3m".as_slice()),
                (b"ritm".as_slice(), b"\x1b[23m".as_slice()),
                (b"Smol".as_slice(), b"\x1b[53m".as_slice()),
                (b"smxx".as_slice(), b"\x1b[9m".as_slice()),
                (b"rmxx".as_slice(), b"\x1b[29m".as_slice()),
                (b"op".as_slice(), b"\x1b[39;49m".as_slice()),
                (b"oc".as_slice(), b"\x1b]104\x07".as_slice()),
            ])]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_wezterm_official_boolean_capabilities() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let query = xtgettcap_query(&[
            b"am".as_slice(),
            b"bce".as_slice(),
            b"ccc".as_slice(),
            b"hs".as_slice(),
            b"mc5i".as_slice(),
            b"mir".as_slice(),
            b"msgr".as_slice(),
            b"npc".as_slice(),
            b"Su".as_slice(),
            b"xenl".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        let output = runtime.feed_pty_output_with_display(&input);

        assert_eq!(
            output.responses,
            vec![xtgettcap_response(&[
                (b"am".as_slice(), b"1".as_slice()),
                (b"bce".as_slice(), b"1".as_slice()),
                (b"ccc".as_slice(), b"1".as_slice()),
                (b"hs".as_slice(), b"1".as_slice()),
                (b"mc5i".as_slice(), b"1".as_slice()),
                (b"mir".as_slice(), b"1".as_slice()),
                (b"msgr".as_slice(), b"1".as_slice()),
                (b"npc".as_slice(), b"1".as_slice()),
                (b"Su".as_slice(), b"1".as_slice()),
                (b"xenl".as_slice(), b"1".as_slice()),
            ])]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_wezterm_official_printer_memory_and_reset_templates() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let query = xtgettcap_query(&[
            b"flash".as_slice(),
            b"mc0".as_slice(),
            b"mc4".as_slice(),
            b"mc5".as_slice(),
            b"meml".as_slice(),
            b"memu".as_slice(),
            b"rs1".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        let output = runtime.feed_pty_output_with_display(&input);

        assert_eq!(
            output.responses,
            vec![xtgettcap_response(&[
                (b"flash".as_slice(), b"\x1b[?5h$<100/>\x1b[?5l".as_slice()),
                (b"mc0".as_slice(), b"\x1b[i".as_slice()),
                (b"mc4".as_slice(), b"\x1b[4i".as_slice()),
                (b"mc5".as_slice(), b"\x1b[5i".as_slice()),
                (b"meml".as_slice(), b"\x1bl".as_slice()),
                (b"memu".as_slice(), b"\x1bm".as_slice()),
                (b"rs1".as_slice(), b"\x1bc\x1b]104\x07".as_slice()),
            ])]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_wezterm_title_and_palette_templates() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let query = xtgettcap_query(&[
            b"dsl".as_slice(),
            b"fsl".as_slice(),
            b"tsl".as_slice(),
            b"initc".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        let output = runtime.feed_pty_output_with_display(&input);

        assert_eq!(
            output.responses,
            vec![xtgettcap_response(&[
                (b"dsl".as_slice(), b"\x1b]2;\x1b\\".as_slice()),
                (b"fsl".as_slice(), b"\x1b\\".as_slice()),
                (b"tsl".as_slice(), b"\x1b]0;".as_slice()),
                (
                    b"initc".as_slice(),
                    b"\x1b]4;%p1%d;rgb:%p2%{255}%*%{1000}%/%2.2X/%p3%{255}%*%{1000}%/%2.2X/%p4%{255}%*%{1000}%/%2.2X\x1b\\".as_slice()
                ),
            ])]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_tmux_cursor_capabilities() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let query = xtgettcap_query(&[
            b"Cr".as_slice(),
            b"Cs".as_slice(),
            b"Se".as_slice(),
            b"Ss".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        let output = runtime.feed_pty_output_with_display(&input);

        assert_eq!(
            output.responses,
            vec![xtgettcap_response(&[
                (b"Cr".as_slice(), b"\x1b]112\x07".as_slice()),
                (b"Cs".as_slice(), b"\x1b]12;%p1%s\x07".as_slice()),
                (b"Se".as_slice(), b"\x1b[2 q".as_slice()),
                (b"Ss".as_slice(), b"\x1b[%p1%d q".as_slice()),
            ])]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_synchronized_output_capability() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let query = xtgettcap_query(&[b"Sync".as_slice()]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        let output = runtime.feed_pty_output_with_display(&input);

        assert_eq!(
            output.responses,
            vec![xtgettcap_response(&[(
                b"Sync".as_slice(),
                b"\x1b[?2026%?%p1%{1}%-%tl%eh%;".as_slice()
            )])]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_mouse_capabilities() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let query = xtgettcap_query(&[b"kmous".as_slice(), b"XM".as_slice(), b"xm".as_slice()]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        let output = runtime.feed_pty_output_with_display(&input);

        assert_eq!(
            output.responses,
            vec![xtgettcap_response(&[
                (b"kmous".as_slice(), b"\x1b[<".as_slice()),
                (
                    b"XM".as_slice(),
                    b"\x1b[?1006;1000%?%p1%{1}%=%th%el%;".as_slice()
                ),
                (
                    b"xm".as_slice(),
                    b"\x1b[<%i%p3%d;%p1%d;%p2%d;%?%p4%tM%em%;".as_slice()
                ),
            ])]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_foundational_terminal_capabilities() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let query = xtgettcap_query(&[
            b"clear".as_slice(),
            b"cup".as_slice(),
            b"home".as_slice(),
            b"civis".as_slice(),
            b"cnorm".as_slice(),
            b"cvvis".as_slice(),
            b"smcup".as_slice(),
            b"rmcup".as_slice(),
            b"sgr0".as_slice(),
            b"sgr".as_slice(),
            b"bold".as_slice(),
            b"dim".as_slice(),
            b"blink".as_slice(),
            b"rev".as_slice(),
            b"smso".as_slice(),
            b"rmso".as_slice(),
            b"invis".as_slice(),
            b"smul".as_slice(),
            b"rmul".as_slice(),
            b"setaf".as_slice(),
            b"setab".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        let output = runtime.feed_pty_output_with_display(&input);

        assert_eq!(
            output.responses,
            vec![xtgettcap_response(&[
                (b"clear".as_slice(), b"\x1b[H\x1b[2J".as_slice()),
                (b"cup".as_slice(), b"\x1b[%i%p1%d;%p2%dH".as_slice()),
                (b"home".as_slice(), b"\x1b[H".as_slice()),
                (b"civis".as_slice(), b"\x1b[?25l".as_slice()),
                (b"cnorm".as_slice(), b"\x1b[?12l\x1b[?25h".as_slice()),
                (b"cvvis".as_slice(), b"\x1b[?12;25h".as_slice()),
                (b"smcup".as_slice(), b"\x1b[?1049h\x1b[22;0;0t".as_slice()),
                (b"rmcup".as_slice(), b"\x1b[?1049l\x1b[23;0;0t".as_slice()),
                (b"sgr0".as_slice(), b"\x1b(B\x1b[m".as_slice()),
                (
                    b"sgr".as_slice(),
                    b"%?%p9%t\x1b(0%e\x1b(B%;\x1b[0%?%p6%t;1%;%?%p5%t;2%;%?%p2%t;4%;%?%p1%p3%|%t;7%;%?%p4%t;5%;%?%p7%t;8%;m".as_slice()
                ),
                (b"bold".as_slice(), b"\x1b[1m".as_slice()),
                (b"dim".as_slice(), b"\x1b[2m".as_slice()),
                (b"blink".as_slice(), b"\x1b[5m".as_slice()),
                (b"rev".as_slice(), b"\x1b[7m".as_slice()),
                (b"smso".as_slice(), b"\x1b[7m".as_slice()),
                (b"rmso".as_slice(), b"\x1b[27m".as_slice()),
                (b"invis".as_slice(), b"\x1b[8m".as_slice()),
                (b"smul".as_slice(), b"\x1b[4m".as_slice()),
                (b"rmul".as_slice(), b"\x1b[24m".as_slice()),
                (
                    b"setaf".as_slice(),
                    b"\x1b[%?%p1%{8}%<%t3%p1%d%e%p1%{16}%<%t9%p1%{8}%-%d%e38;5;%p1%d%;m".as_slice()
                ),
                (
                    b"setab".as_slice(),
                    b"\x1b[%?%p1%{8}%<%t4%p1%d%e%p1%{16}%<%t10%p1%{8}%-%d%e48;5;%p1%d%;m".as_slice()
                ),
            ])]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_common_control_capabilities() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let query = xtgettcap_query(&[
            b"el".as_slice(),
            b"ed".as_slice(),
            b"el1".as_slice(),
            b"dch1".as_slice(),
            b"ich1".as_slice(),
            b"il1".as_slice(),
            b"dl1".as_slice(),
            b"cuu".as_slice(),
            b"cud".as_slice(),
            b"cub".as_slice(),
            b"cuf".as_slice(),
            b"hpa".as_slice(),
            b"vpa".as_slice(),
            b"cbt".as_slice(),
            b"ht".as_slice(),
            b"hts".as_slice(),
            b"tbc".as_slice(),
            b"ech".as_slice(),
            b"rep".as_slice(),
            b"csr".as_slice(),
            b"indn".as_slice(),
            b"rin".as_slice(),
            b"smir".as_slice(),
            b"rmir".as_slice(),
            b"smam".as_slice(),
            b"rmam".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        let output = runtime.feed_pty_output_with_display(&input);

        assert_eq!(
            output.responses,
            vec![xtgettcap_response(&[
                (b"el".as_slice(), b"\x1b[K".as_slice()),
                (b"ed".as_slice(), b"\x1b[J".as_slice()),
                (b"el1".as_slice(), b"\x1b[1K".as_slice()),
                (b"dch1".as_slice(), b"\x1b[P".as_slice()),
                (b"ich1".as_slice(), b"\x1b[@".as_slice()),
                (b"il1".as_slice(), b"\x1b[L".as_slice()),
                (b"dl1".as_slice(), b"\x1b[M".as_slice()),
                (b"cuu".as_slice(), b"\x1b[%p1%dA".as_slice()),
                (b"cud".as_slice(), b"\x1b[%p1%dB".as_slice()),
                (b"cub".as_slice(), b"\x1b[%p1%dD".as_slice()),
                (b"cuf".as_slice(), b"\x1b[%p1%dC".as_slice()),
                (b"hpa".as_slice(), b"\x1b[%i%p1%dG".as_slice()),
                (b"vpa".as_slice(), b"\x1b[%i%p1%dd".as_slice()),
                (b"cbt".as_slice(), b"\x1b[Z".as_slice()),
                (b"ht".as_slice(), b"\t".as_slice()),
                (b"hts".as_slice(), b"\x1bH".as_slice()),
                (b"tbc".as_slice(), b"\x1b[3g".as_slice()),
                (b"ech".as_slice(), b"\x1b[%p1%dX".as_slice()),
                (b"rep".as_slice(), b"%p1%c\x1b[%p2%{1}%-%db".as_slice()),
                (b"csr".as_slice(), b"\x1b[%i%p1%d;%p2%dr".as_slice()),
                (b"indn".as_slice(), b"\x1b[%p1%dS".as_slice()),
                (b"rin".as_slice(), b"\x1b[%p1%dT".as_slice()),
                (b"smir".as_slice(), b"\x1b[4h".as_slice()),
                (b"rmir".as_slice(), b"\x1b[4l".as_slice()),
                (b"smam".as_slice(), b"\x1b[?7h".as_slice()),
                (b"rmam".as_slice(), b"\x1b[?7l".as_slice()),
            ])]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_common_key_capabilities() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let query = xtgettcap_query(&[
            b"kcuu1".as_slice(),
            b"kcud1".as_slice(),
            b"kcuf1".as_slice(),
            b"kcub1".as_slice(),
            b"kb2".as_slice(),
            b"kbs".as_slice(),
            b"kcbt".as_slice(),
            b"khome".as_slice(),
            b"kend".as_slice(),
            b"kich1".as_slice(),
            b"kdch1".as_slice(),
            b"kpp".as_slice(),
            b"knp".as_slice(),
            b"kHOM".as_slice(),
            b"kEND".as_slice(),
            b"kIC".as_slice(),
            b"kDC".as_slice(),
            b"kPRV".as_slice(),
            b"kNXT".as_slice(),
            b"kLFT".as_slice(),
            b"kRIT".as_slice(),
            b"kri".as_slice(),
            b"kind".as_slice(),
            b"kent".as_slice(),
            b"kf1".as_slice(),
            b"kf2".as_slice(),
            b"kf3".as_slice(),
            b"kf4".as_slice(),
            b"kf5".as_slice(),
            b"kf6".as_slice(),
            b"kf7".as_slice(),
            b"kf8".as_slice(),
            b"kf9".as_slice(),
            b"kf10".as_slice(),
            b"kf11".as_slice(),
            b"kf12".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        let output = runtime.feed_pty_output_with_display(&input);

        assert_eq!(
            output.responses,
            vec![xtgettcap_response(&[
                (b"kcuu1".as_slice(), b"\x1bOA".as_slice()),
                (b"kcud1".as_slice(), b"\x1bOB".as_slice()),
                (b"kcuf1".as_slice(), b"\x1bOC".as_slice()),
                (b"kcub1".as_slice(), b"\x1bOD".as_slice()),
                (b"kb2".as_slice(), b"\x1bOE".as_slice()),
                (b"kbs".as_slice(), b"\x7f".as_slice()),
                (b"kcbt".as_slice(), b"\x1b[Z".as_slice()),
                (b"khome".as_slice(), b"\x1bOH".as_slice()),
                (b"kend".as_slice(), b"\x1bOF".as_slice()),
                (b"kich1".as_slice(), b"\x1b[2~".as_slice()),
                (b"kdch1".as_slice(), b"\x1b[3~".as_slice()),
                (b"kpp".as_slice(), b"\x1b[5~".as_slice()),
                (b"knp".as_slice(), b"\x1b[6~".as_slice()),
                (b"kHOM".as_slice(), b"\x1b[1;2H".as_slice()),
                (b"kEND".as_slice(), b"\x1b[1;2F".as_slice()),
                (b"kIC".as_slice(), b"\x1b[2;2~".as_slice()),
                (b"kDC".as_slice(), b"\x1b[3;2~".as_slice()),
                (b"kPRV".as_slice(), b"\x1b[5;2~".as_slice()),
                (b"kNXT".as_slice(), b"\x1b[6;2~".as_slice()),
                (b"kLFT".as_slice(), b"\x1b[1;2D".as_slice()),
                (b"kRIT".as_slice(), b"\x1b[1;2C".as_slice()),
                (b"kri".as_slice(), b"\x1b[1;2A".as_slice()),
                (b"kind".as_slice(), b"\x1b[1;2B".as_slice()),
                (b"kent".as_slice(), b"\x1bOM".as_slice()),
                (b"kf1".as_slice(), b"\x1bOP".as_slice()),
                (b"kf2".as_slice(), b"\x1bOQ".as_slice()),
                (b"kf3".as_slice(), b"\x1bOR".as_slice()),
                (b"kf4".as_slice(), b"\x1bOS".as_slice()),
                (b"kf5".as_slice(), b"\x1b[15~".as_slice()),
                (b"kf6".as_slice(), b"\x1b[17~".as_slice()),
                (b"kf7".as_slice(), b"\x1b[18~".as_slice()),
                (b"kf8".as_slice(), b"\x1b[19~".as_slice()),
                (b"kf9".as_slice(), b"\x1b[20~".as_slice()),
                (b"kf10".as_slice(), b"\x1b[21~".as_slice()),
                (b"kf11".as_slice(), b"\x1b[23~".as_slice()),
                (b"kf12".as_slice(), b"\x1b[24~".as_slice()),
            ])]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_wezterm_keypad_transmit_capabilities() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let query = xtgettcap_query(&[b"smkx".as_slice(), b"rmkx".as_slice()]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        let output = runtime.feed_pty_output_with_display(&input);

        assert_eq!(
            output.responses,
            vec![xtgettcap_response(&[
                (b"smkx".as_slice(), b"\x1b[?1h\x1b=".as_slice()),
                (b"rmkx".as_slice(), b"\x1b[?1l\x1b>".as_slice()),
            ])]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_modified_function_key_capabilities() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let entries: &[(&[u8], &[u8])] = &[
            (b"kf13".as_slice(), b"\x1b[1;2P".as_slice()),
            (b"kf14".as_slice(), b"\x1b[1;2Q".as_slice()),
            (b"kf15".as_slice(), b"\x1b[1;2R".as_slice()),
            (b"kf16".as_slice(), b"\x1b[1;2S".as_slice()),
            (b"kf17".as_slice(), b"\x1b[15;2~".as_slice()),
            (b"kf18".as_slice(), b"\x1b[17;2~".as_slice()),
            (b"kf19".as_slice(), b"\x1b[18;2~".as_slice()),
            (b"kf20".as_slice(), b"\x1b[19;2~".as_slice()),
            (b"kf21".as_slice(), b"\x1b[20;2~".as_slice()),
            (b"kf22".as_slice(), b"\x1b[21;2~".as_slice()),
            (b"kf23".as_slice(), b"\x1b[23;2~".as_slice()),
            (b"kf24".as_slice(), b"\x1b[24;2~".as_slice()),
            (b"kf25".as_slice(), b"\x1b[1;5P".as_slice()),
            (b"kf26".as_slice(), b"\x1b[1;5Q".as_slice()),
            (b"kf27".as_slice(), b"\x1b[1;5R".as_slice()),
            (b"kf28".as_slice(), b"\x1b[1;5S".as_slice()),
            (b"kf29".as_slice(), b"\x1b[15;5~".as_slice()),
            (b"kf30".as_slice(), b"\x1b[17;5~".as_slice()),
            (b"kf31".as_slice(), b"\x1b[18;5~".as_slice()),
            (b"kf32".as_slice(), b"\x1b[19;5~".as_slice()),
            (b"kf33".as_slice(), b"\x1b[20;5~".as_slice()),
            (b"kf34".as_slice(), b"\x1b[21;5~".as_slice()),
            (b"kf35".as_slice(), b"\x1b[23;5~".as_slice()),
            (b"kf36".as_slice(), b"\x1b[24;5~".as_slice()),
            (b"kf37".as_slice(), b"\x1b[1;6P".as_slice()),
            (b"kf38".as_slice(), b"\x1b[1;6Q".as_slice()),
            (b"kf39".as_slice(), b"\x1b[1;6R".as_slice()),
            (b"kf40".as_slice(), b"\x1b[1;6S".as_slice()),
            (b"kf41".as_slice(), b"\x1b[15;6~".as_slice()),
            (b"kf42".as_slice(), b"\x1b[17;6~".as_slice()),
            (b"kf43".as_slice(), b"\x1b[18;6~".as_slice()),
            (b"kf44".as_slice(), b"\x1b[19;6~".as_slice()),
            (b"kf45".as_slice(), b"\x1b[20;6~".as_slice()),
            (b"kf46".as_slice(), b"\x1b[21;6~".as_slice()),
            (b"kf47".as_slice(), b"\x1b[23;6~".as_slice()),
            (b"kf48".as_slice(), b"\x1b[24;6~".as_slice()),
            (b"kf49".as_slice(), b"\x1b[1;3P".as_slice()),
            (b"kf50".as_slice(), b"\x1b[1;3Q".as_slice()),
            (b"kf51".as_slice(), b"\x1b[1;3R".as_slice()),
            (b"kf52".as_slice(), b"\x1b[1;3S".as_slice()),
            (b"kf53".as_slice(), b"\x1b[15;3~".as_slice()),
            (b"kf54".as_slice(), b"\x1b[17;3~".as_slice()),
            (b"kf55".as_slice(), b"\x1b[18;3~".as_slice()),
            (b"kf56".as_slice(), b"\x1b[19;3~".as_slice()),
            (b"kf57".as_slice(), b"\x1b[20;3~".as_slice()),
            (b"kf58".as_slice(), b"\x1b[21;3~".as_slice()),
            (b"kf59".as_slice(), b"\x1b[23;3~".as_slice()),
            (b"kf60".as_slice(), b"\x1b[24;3~".as_slice()),
            (b"kf61".as_slice(), b"\x1b[1;4P".as_slice()),
            (b"kf62".as_slice(), b"\x1b[1;4Q".as_slice()),
            (b"kf63".as_slice(), b"\x1b[1;4R".as_slice()),
        ];
        let names: Vec<&[u8]> = entries.iter().map(|(name, _)| *name).collect();
        let query = xtgettcap_query(&names);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        let output = runtime.feed_pty_output_with_display(&input);

        assert_eq!(output.responses, vec![xtgettcap_response(entries)]);
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_acs_capabilities() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let query = xtgettcap_query(&[
            b"enacs".as_slice(),
            b"smacs".as_slice(),
            b"rmacs".as_slice(),
            b"acsc".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        let output = runtime.feed_pty_output_with_display(&input);

        assert_eq!(
            output.responses,
            vec![xtgettcap_response(&[
                (b"enacs".as_slice(), b"\x1b)0".as_slice()),
                (b"smacs".as_slice(), b"\x1b(0".as_slice()),
                (b"rmacs".as_slice(), b"\x1b(B".as_slice()),
                (
                    b"acsc".as_slice(),
                    b"``aaffggiijjkkllmmnnooppqqrrssttuuvvwwxxyyzz{{||}}~~".as_slice()
                ),
            ])]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_wezterm_control_sequence_capabilities() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let query = xtgettcap_query(&[
            b"bel".as_slice(),
            b"cr".as_slice(),
            b"ind".as_slice(),
            b"ri".as_slice(),
            b"sc".as_slice(),
            b"rc".as_slice(),
            b"cuu1".as_slice(),
            b"cud1".as_slice(),
            b"cuf1".as_slice(),
            b"cub1".as_slice(),
            b"dch".as_slice(),
            b"ich".as_slice(),
            b"dl".as_slice(),
            b"il".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        let output = runtime.feed_pty_output_with_display(&input);

        assert_eq!(
            output.responses,
            vec![xtgettcap_response(&[
                (b"bel".as_slice(), b"\x07".as_slice()),
                (b"cr".as_slice(), b"\r".as_slice()),
                (b"ind".as_slice(), b"\n".as_slice()),
                (b"ri".as_slice(), b"\x1bM".as_slice()),
                (b"sc".as_slice(), b"\x1b7".as_slice()),
                (b"rc".as_slice(), b"\x1b8".as_slice()),
                (b"cuu1".as_slice(), b"\x1b[A".as_slice()),
                (b"cud1".as_slice(), b"\n".as_slice()),
                (b"cuf1".as_slice(), b"\x1b[C".as_slice()),
                (b"cub1".as_slice(), b"\x08".as_slice()),
                (b"dch".as_slice(), b"\x1b[%p1%dP".as_slice()),
                (b"ich".as_slice(), b"\x1b[%p1%d@".as_slice()),
                (b"dl".as_slice(), b"\x1b[%p1%dM".as_slice()),
                (b"il".as_slice(), b"\x1b[%p1%dL".as_slice()),
            ])]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_wezterm_meta_key_capabilities() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let query = xtgettcap_query(&[b"km".as_slice(), b"smm".as_slice(), b"rmm".as_slice()]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        let output = runtime.feed_pty_output_with_display(&input);

        assert_eq!(
            output.responses,
            vec![xtgettcap_response(&[
                (b"km".as_slice(), b"1".as_slice()),
                (b"smm".as_slice(), b"\x1b[?1034h".as_slice()),
                (b"rmm".as_slice(), b"\x1b[?1034l".as_slice()),
            ])]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_wezterm_reset_templates() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let query = xtgettcap_query(&[b"is2".as_slice(), b"rs2".as_slice()]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        let output = runtime.feed_pty_output_with_display(&input);

        assert_eq!(
            output.responses,
            vec![xtgettcap_response(&[
                (
                    b"is2".as_slice(),
                    b"\x1b[!p\x1b[?3;4l\x1b[4l\x1b>".as_slice()
                ),
                (
                    b"rs2".as_slice(),
                    b"\x1b[!p\x1b[?3;4l\x1b[4l\x1b>".as_slice()
                ),
            ])]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtgettcap_wezterm_query_templates() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let query = xtgettcap_query(&[
            b"u6".as_slice(),
            b"u7".as_slice(),
            b"u8".as_slice(),
            b"u9".as_slice(),
        ]);
        let mut input = b"before".to_vec();
        input.extend_from_slice(&query);
        input.extend_from_slice(b"after");

        let output = runtime.feed_pty_output_with_display(&input);

        assert_eq!(
            output.responses,
            vec![xtgettcap_response(&[
                (b"u6".as_slice(), b"\x1b[%i%d;%dR".as_slice()),
                (b"u7".as_slice(), b"\x1b[6n".as_slice()),
                (b"u8".as_slice(), b"\x1b[?%[;0123456789]c".as_slice()),
                (b"u9".as_slice(), b"\x1b[c".as_slice()),
            ])]
        );
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_decrqss_state_queries_without_feeding_them_to_terminal() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output = runtime.feed_pty_output_with_display(
            b"before\x1b[1;2;4:3;5;8;9;53;73;58;5;34;38;6;4;5;6;7;48;2;1;2;3m\x1bP$qm\x1b\\ middle\x1b[5 q\x90$q q\x9c after\x1b[2;5r\x1bP$qr\x1b\\done",
        );

        assert_eq!(
            output.responses,
            vec![
                b"\x1bP1$r1;2;4:3;5;8;9;53;73;58;5;34;38;6;4;5;6;7;48;2;1;2;3m\x1b\\".to_vec(),
                b"\x1bP1$r5 q\x9c".to_vec(),
                b"\x1bP1$r2;5r\x1b\\".to_vec(),
            ]
        );
        assert_eq!(output.display, b"before middle afterdone");
        assert!(!String::from_utf8_lossy(&output.display).contains("$q"));
    }

    #[test]
    fn answers_wezterm_decrqss_conformance_and_left_right_margin_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output =
            runtime.feed_pty_output_with_display(b"before\x1bP$q\"p\x1b\\ middle\x90$qs\x9c after");

        assert_eq!(
            output.responses,
            vec![
                b"\x1bP1$r61;1\"p\x1b\\".to_vec(),
                b"\x1bP1$r1;80s\x9c".to_vec(),
            ]
        );
        assert_eq!(output.display, b"before middle after");
    }

    #[test]
    fn answers_utf8_c1_dcs_decrqss_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let dcs = '\u{90}';
        let st = '\u{9c}';
        let input = format!("before{dcs}$q\"p{st} middle{dcs}$qs{st} after");

        let output = runtime.feed_pty_output_with_display(input.as_bytes());

        assert_eq!(
            output.responses,
            vec![
                b"\x1bP1$r61;1\"p\x9c".to_vec(),
                b"\x1bP1$r1;80s\x9c".to_vec(),
            ]
        );
        assert_eq!(output.display, b"before middle after");
    }

    #[test]
    fn answers_split_utf8_c1_dcs_decrqss_query() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let first = runtime.feed_pty_output_with_display(b"before\xc2");
        let second = runtime.feed_pty_output_with_display(b"\x90$q\"p\xc2");
        let third = runtime.feed_pty_output_with_display(b"\x9cafter");

        assert!(first.responses.is_empty());
        assert_eq!(first.display, b"before");
        assert!(second.responses.is_empty());
        assert!(second.display.is_empty());
        assert_eq!(third.responses, vec![b"\x1bP1$r61;1\"p\x9c".to_vec()]);
        assert_eq!(third.display, b"after");
    }

    #[test]
    fn resynchronizes_queries_after_escape_inside_utf8_c1_dcs_payload() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let dcs = '\u{90}';
        let st = '\u{9c}';
        let input = format!("before{dcs}payload \x1b[6n{st}after");

        let output = runtime.feed_pty_output_with_display(input.as_bytes());

        assert_eq!(output.responses, vec![b"\x1b[1;7R".to_vec()]);
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_split_wezterm_decrqss_conformance_and_left_right_margin_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let first = runtime.feed_pty_output_with_display(b"before\x1bP$q\"");
        let second = runtime.feed_pty_output_with_display(b"p\x1b\\ middle\x90$q");
        let third = runtime.feed_pty_output_with_display(b"s\x9c after");

        assert!(first.responses.is_empty());
        assert_eq!(first.display, b"before");
        assert_eq!(second.responses, vec![b"\x1bP1$r61;1\"p\x1b\\".to_vec()]);
        assert_eq!(second.display, b" middle");
        assert_eq!(third.responses, vec![b"\x1bP1$r1;80s\x9c".to_vec()]);
        assert_eq!(third.display, b" after");
    }

    #[test]
    fn answers_decrqss_left_right_margin_query_from_declrmm_state() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output =
            runtime.feed_pty_output_with_display(b"before\x1b[?69h\x1b[3;6s\x1bP$qs\x1b\\after");

        assert_eq!(output.responses, vec![b"\x1bP1$r3;6s\x1b\\".to_vec()]);
        assert_eq!(output.display, b"beforeafter");
    }

    #[test]
    fn answers_xtversion_queries_without_feeding_them_to_terminal() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let output =
            runtime.feed_pty_output_with_display(b"before\x1b[>q middle\x1b[>0q after\x9b>q done");

        assert_eq!(
            output.responses,
            vec![
                b"\x1bP>|R-Term 0.1.0\x1b\\".to_vec(),
                b"\x1bP>|R-Term 0.1.0\x1b\\".to_vec(),
                b"\x1bP>|R-Term 0.1.0\x1b\\".to_vec(),
            ]
        );
        assert_eq!(output.display, b"before middle after done");
    }

    #[test]
    fn answers_c1_private_mode_status_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        runtime.feed_pty_output(b"\x9b?1000;1006h\x1b[?2004h\x1b[?2026h");
        let normal_mouse = runtime.feed_pty_output(b"\x9b?1000$p");
        let sgr_mouse = runtime.feed_pty_output(b"\x9b?1006$p");
        let bracketed_paste = runtime.feed_pty_output(b"\x9b?2004$p");
        let synchronized_output = runtime.feed_pty_output(b"\x1b[?2026$p");

        assert_eq!(normal_mouse, vec![b"\x1b[?1000;1$y".to_vec()]);
        assert_eq!(sgr_mouse, vec![b"\x1b[?1006;1$y".to_vec()]);
        assert_eq!(bracketed_paste, vec![b"\x1b[?2004;1$y".to_vec()]);
        assert_eq!(synchronized_output, vec![b"\x1b[?2026;1$y".to_vec()]);
    }

    #[test]
    fn answers_c1_wezterm_private_mode_reports() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        let responses = runtime.feed_pty_output(b"\x9b?2027$p\x9b?1070h\x9b?1070$p");

        assert_eq!(
            responses,
            vec![b"\x1b[?2027;3$y".to_vec(), b"\x1b[?1070;1$y".to_vec()]
        );
    }

    #[test]
    fn answers_c1_dec_ansi_mode_status_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));

        runtime.feed_pty_output(b"\x9b?2h");
        let responses = runtime.feed_pty_output(b"\x9b?2$p");

        assert_eq!(responses, vec![b"\x1b[?2;1$y".to_vec()]);
    }

    #[test]
    fn answers_utf8_c1_private_mode_status_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let csi = '\u{9b}';

        let initial = runtime.feed_pty_output(format!("{csi}?45$p").as_bytes());
        runtime.feed_pty_output(format!("{csi}?45h").as_bytes());
        let enabled = runtime.feed_pty_output(format!("{csi}?45$p").as_bytes());

        assert_eq!(initial, vec![b"\x1b[?45;2$y".to_vec()]);
        assert_eq!(enabled, vec![b"\x1b[?45;1$y".to_vec()]);
    }

    #[test]
    fn tracks_application_cursor_key_mode_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        assert!(!runtime.application_cursor_keys());

        runtime.feed_pty_output(b"\x1b[?1h");
        assert!(runtime.application_cursor_keys());

        runtime.feed_pty_output(b"\x1b[?1l");
        assert!(!runtime.application_cursor_keys());
    }

    #[test]
    fn tracks_split_application_cursor_key_mode_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x1b[?");
        assert!(!runtime.application_cursor_keys());

        runtime.feed_pty_output(b"1h");
        assert!(runtime.application_cursor_keys());
    }

    #[test]
    fn tracks_focus_reporting_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        assert!(!runtime.focus_reporting());

        runtime.feed_pty_output(b"\x1b[?1004h");
        assert!(runtime.focus_reporting());

        runtime.feed_pty_output(b"\x1b[?1004l");
        assert!(!runtime.focus_reporting());
    }

    #[test]
    fn tracks_bracketed_paste_mode_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        assert!(!runtime.bracketed_paste());

        runtime.feed_pty_output(b"\x1b[?2004h");
        assert!(runtime.bracketed_paste());

        runtime.feed_pty_output(b"\x1b[?2004l");
        assert!(!runtime.bracketed_paste());
    }

    #[test]
    fn tracks_synchronized_output_mode_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        assert!(!runtime.synchronized_output());
        assert_eq!(
            runtime.feed_pty_output(b"\x1b[?2026$p"),
            vec![b"\x1b[?2026;2$y".to_vec()]
        );

        runtime.feed_pty_output(b"\x1b[?2026h");
        assert!(runtime.synchronized_output());
        assert_eq!(
            runtime.feed_pty_output(b"\x1b[?2026$p"),
            vec![b"\x1b[?2026;1$y".to_vec()]
        );

        runtime.feed_pty_output(b"\x1b[?2026l");
        assert!(!runtime.synchronized_output());
        assert_eq!(
            runtime.feed_pty_output(b"\x1b[?2026$p"),
            vec![b"\x1b[?2026;2$y".to_vec()]
        );
    }

    #[test]
    fn delays_synchronized_output_damage_until_mode_resets() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let first = runtime.feed_pty_output_with_display(b"before\x1b[?2026hmid");

        assert_eq!(first.display, b"beforemid");
        assert_eq!(
            first.damage,
            vec![rterm_types::DamageRegion::new(0, 0, 6, 1)]
        );
        assert!(runtime.synchronized_output());
        assert!(terminal_text(&runtime).contains("beforemid"));

        let buffered = runtime.feed_pty_output_with_display(b"after\x1b[?2026$p");

        assert_eq!(buffered.display, b"after");
        assert!(buffered.damage.is_empty());
        assert_eq!(buffered.responses, vec![b"\x1b[?2026;1$y".to_vec()]);
        assert!(terminal_text(&runtime).contains("beforemidafter"));

        let flushed = runtime.feed_pty_output_with_display(b"\x1b[?2026l done");

        assert_eq!(flushed.display, b" done");
        assert!(!flushed.damage.is_empty());
        assert!(!runtime.synchronized_output());
        assert!(terminal_text(&runtime).contains("beforemidafter done"));
    }

    #[test]
    fn resynchronizes_private_input_modes_after_escape_inside_control_strings() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x1b]0;title \x1b[?1004h\x07\x1bPpayload \x1b[?2004h\x1b\\");

        assert!(runtime.focus_reporting());
        assert!(runtime.bracketed_paste());
        assert_eq!(
            runtime.feed_pty_output(b"\x1b[?1004$p\x1b[?2004$p"),
            vec![b"\x1b[?1004;1$y".to_vec(), b"\x1b[?2004;1$y".to_vec()]
        );
    }

    #[test]
    fn extracts_osc52_clipboard_text_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x1b]52;c;Y29weQ==\x07");

        assert_eq!(runtime.take_clipboard_texts(), vec!["copy".to_owned()]);
        assert!(runtime.take_clipboard_texts().is_empty());
    }

    #[test]
    fn extracts_iterm_copy_clipboard_text_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x1b]1337;Copy=;Y29weQ==\x07");

        assert_eq!(runtime.take_clipboard_texts(), vec!["copy".to_owned()]);
        assert!(runtime.take_clipboard_texts().is_empty());
    }

    #[test]
    fn extracts_c1_iterm_copy_clipboard_text_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x9d1337;Copy=;Y29weQ==\x9c");

        assert_eq!(runtime.take_clipboard_texts(), vec!["copy".to_owned()]);
        assert!(runtime.take_clipboard_texts().is_empty());
    }

    #[test]
    fn extracts_utf8_c1_iterm_copy_clipboard_text_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output("\u{9d}1337;Copy=;Y29weQ==\u{9c}".as_bytes());

        assert_eq!(runtime.take_clipboard_texts(), vec!["copy".to_owned()]);
        assert!(runtime.take_clipboard_texts().is_empty());
    }

    #[test]
    fn extracts_wezterm_osc9_and_osc777_notifications_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let output = runtime.feed_pty_output_with_display(
            b"before\x1b]9;build done\x07 middle\x9d777;notify;Build;failed\x9c after",
        );

        assert_eq!(output.display, b"before middle after");
        assert_eq!(
            runtime.take_notifications(),
            vec![
                TerminalNotification {
                    title: None,
                    body: "build done".to_owned(),
                },
                TerminalNotification {
                    title: Some("Build".to_owned()),
                    body: "failed".to_owned(),
                },
            ]
        );
        assert!(runtime.take_notifications().is_empty());
    }

    #[test]
    fn extracts_utf8_c1_osc9_and_osc777_notifications_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let output = runtime.feed_pty_output_with_display(
            "before\u{9d}9;build done\u{9c} middle\u{9d}777;notify;Build;failed\u{9c} after"
                .as_bytes(),
        );

        assert_eq!(output.display, b"before middle after");
        assert_eq!(
            runtime.take_notifications(),
            vec![
                TerminalNotification {
                    title: None,
                    body: "build done".to_owned(),
                },
                TerminalNotification {
                    title: Some("Build".to_owned()),
                    body: "failed".to_owned(),
                },
            ]
        );
        assert!(runtime.take_notifications().is_empty());
    }

    #[test]
    fn tracks_conemu_progress_osc9_without_notification() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let output = runtime
            .feed_pty_output_with_display(b"before\x1b]9;4;1;42\x07 middle\x9d9;4;2\x9c after");

        assert_eq!(output.display, b"before middle after");
        assert_eq!(runtime.progress(), TerminalProgress::Error(0));
        assert!(runtime.take_notifications().is_empty());

        runtime.feed_pty_output(b"\x1b]9;4;3\x07");
        assert_eq!(runtime.progress(), TerminalProgress::Indeterminate);

        runtime.feed_pty_output(b"\x1b]9;4;0\x07");
        assert_eq!(runtime.progress(), TerminalProgress::None);
    }

    #[test]
    fn tracks_utf8_c1_conemu_progress_osc9_without_notification() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let output = runtime.feed_pty_output_with_display(
            "before\u{9d}9;4;1;42\u{9c} middle\u{9d}9;4;2\u{9c} after".as_bytes(),
        );

        assert_eq!(output.display, b"before middle after");
        assert_eq!(runtime.progress(), TerminalProgress::Error(0));
        assert!(runtime.take_notifications().is_empty());
    }

    #[test]
    fn extracts_c1_osc52_clipboard_text_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x9d52;c;Y29weQ==\x9c");

        assert_eq!(runtime.take_clipboard_texts(), vec!["copy".to_owned()]);
        assert!(runtime.take_clipboard_texts().is_empty());
    }

    #[test]
    fn extracts_utf8_c1_osc52_clipboard_text_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output("\u{9d}52;c;Y29weQ==\u{9c}".as_bytes());

        assert_eq!(runtime.take_clipboard_texts(), vec!["copy".to_owned()]);
        assert!(runtime.take_clipboard_texts().is_empty());
    }

    #[test]
    fn resynchronizes_osc52_clipboard_text_after_escape_inside_osc_control_strings() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x1b]0;title \x1b]52;c;Y29weQ==\x07");

        assert_eq!(runtime.take_clipboard_texts(), vec!["copy".to_owned()]);
        assert!(runtime.take_clipboard_queries().is_empty());
    }

    #[test]
    fn resynchronizes_osc52_clipboard_text_after_escape_inside_st_control_strings() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x1bPpayload \x1b]52;c;Y29weQ==\x1b\\");

        assert_eq!(runtime.take_clipboard_texts(), vec!["copy".to_owned()]);
        assert!(runtime.take_clipboard_queries().is_empty());
    }

    #[test]
    fn resynchronizes_split_osc52_clipboard_text_after_escape_inside_st_control_strings() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x1bPpayload ");
        runtime.feed_pty_output(b"\x1b]52;c;Y29weQ==\x1b\\");

        assert_eq!(runtime.take_clipboard_texts(), vec!["copy".to_owned()]);
        assert!(runtime.take_clipboard_queries().is_empty());
    }

    #[test]
    fn cancelled_osc_clipboard_and_notification_have_no_side_effects() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x1b]52;c;Y29weQ==\x18\x07\x1b]9;should not notify\x1a\x07");

        assert!(runtime.take_clipboard_texts().is_empty());
        assert!(runtime.take_clipboard_queries().is_empty());
        assert!(runtime.take_notifications().is_empty());
        assert_eq!(runtime.progress(), TerminalProgress::None);
    }

    #[test]
    fn extracts_split_osc52_clipboard_text_with_st_terminator() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x1b]52;c;Y2");
        assert!(runtime.take_clipboard_texts().is_empty());

        runtime.feed_pty_output(b"9weQ==\x1b\\");

        assert_eq!(runtime.take_clipboard_texts(), vec!["copy".to_owned()]);
    }

    #[test]
    fn extracts_split_c1_osc52_clipboard_text() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x9d52;c;Y2");
        assert!(runtime.take_clipboard_texts().is_empty());

        runtime.feed_pty_output(b"9weQ==\x9c");

        assert_eq!(runtime.take_clipboard_texts(), vec!["copy".to_owned()]);
    }

    #[test]
    fn extracts_osc52_clipboard_queries_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x1b]52;c;?\x07");

        assert_eq!(runtime.take_clipboard_queries(), vec!["c".to_owned()]);
        assert!(runtime.take_clipboard_queries().is_empty());
    }

    #[test]
    fn extracts_c1_osc52_clipboard_queries_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x9d52;c;?\x9c");

        assert_eq!(runtime.take_clipboard_queries(), vec!["c".to_owned()]);
        assert!(runtime.take_clipboard_queries().is_empty());
    }

    #[test]
    fn tracks_combined_private_input_modes_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x1b[?1;1004;2004h");

        assert!(runtime.application_cursor_keys());
        assert!(runtime.focus_reporting());
        assert!(runtime.bracketed_paste());
    }

    #[test]
    fn tracks_c1_private_input_modes_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x9b?1;1004;2004h");

        assert!(runtime.application_cursor_keys());
        assert!(runtime.focus_reporting());
        assert!(runtime.bracketed_paste());
    }

    #[test]
    fn tracks_mouse_reporting_modes_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        assert_eq!(runtime.mouse_input_mode(), MouseInputMode::default());

        runtime.feed_pty_output(b"\x1b[?1000;1006h");
        assert_eq!(
            runtime.mouse_input_mode(),
            MouseInputMode::new(MouseReportingMode::Normal, MouseProtocolMode::Sgr)
        );

        runtime.feed_pty_output(b"\x1b[?1002h");
        assert_eq!(
            runtime.mouse_input_mode(),
            MouseInputMode::new(MouseReportingMode::ButtonEvent, MouseProtocolMode::Sgr)
        );

        runtime.feed_pty_output(b"\x1b[?1006l");
        assert_eq!(
            runtime.mouse_input_mode(),
            MouseInputMode::new(MouseReportingMode::ButtonEvent, MouseProtocolMode::X10)
        );

        runtime.feed_pty_output(b"\x1b[?1002;1000l");
        assert_eq!(
            runtime.mouse_input_mode(),
            MouseInputMode::new(MouseReportingMode::None, MouseProtocolMode::X10)
        );
    }

    #[test]
    fn resets_extended_mouse_protocols_to_x10_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x1b[?1000;1005;1015;1006;1016h");
        assert_eq!(
            runtime.mouse_input_mode(),
            MouseInputMode::new(MouseReportingMode::Normal, MouseProtocolMode::SgrPixels)
        );

        runtime.feed_pty_output(b"\x1b[?1016l");
        assert_eq!(
            runtime.mouse_input_mode(),
            MouseInputMode::new(MouseReportingMode::Normal, MouseProtocolMode::X10)
        );

        runtime.feed_pty_output(b"\x1b[?1006h");
        assert_eq!(
            runtime.mouse_input_mode(),
            MouseInputMode::new(MouseReportingMode::Normal, MouseProtocolMode::Sgr)
        );

        runtime.feed_pty_output(b"\x1b[?1006l");
        assert_eq!(
            runtime.mouse_input_mode(),
            MouseInputMode::new(MouseReportingMode::Normal, MouseProtocolMode::X10)
        );
    }

    #[test]
    fn answers_extended_mouse_protocol_status_queries() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x1b[?1005;1015;1016h");

        assert_eq!(
            runtime.feed_pty_output(b"\x1b[?1005$p\x1b[?1015$p\x1b[?1016$p"),
            vec![
                b"\x1b[?1005;2$y".to_vec(),
                b"\x1b[?1015;2$y".to_vec(),
                b"\x1b[?1016;1$y".to_vec(),
            ]
        );

        runtime.feed_pty_output(b"\x1b[?1005;1015;1016l");

        assert_eq!(
            runtime.feed_pty_output(b"\x1b[?1005$p\x1b[?1015$p\x1b[?1016$p"),
            vec![
                b"\x1b[?1005;2$y".to_vec(),
                b"\x1b[?1015;2$y".to_vec(),
                b"\x1b[?1016;2$y".to_vec(),
            ]
        );
    }

    #[test]
    fn tracks_application_keypad_mode_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        assert!(!runtime.application_keypad());

        runtime.feed_pty_output(b"\x1b=");
        assert!(runtime.application_keypad());

        runtime.feed_pty_output(b"\x1b>");
        assert!(!runtime.application_keypad());
    }

    #[test]
    fn tracks_split_application_keypad_mode_from_pty_output() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x1b");
        assert!(!runtime.application_keypad());

        runtime.feed_pty_output(b"=");
        assert!(runtime.application_keypad());
    }

    #[test]
    fn answers_kitty_keyboard_protocol_flags_queries_and_tracks_push_pop() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
        runtime.set_enable_kitty_keyboard(true);

        let output = runtime.feed_pty_output_with_display(b"before\x1b[?u");

        assert_eq!(output.display, b"before");
        assert_eq!(output.responses, vec![b"\x1b[?0u".to_vec()]);

        runtime.feed_pty_output(b"\x1b[>1u");
        assert_eq!(
            runtime.feed_pty_output(b"\x1b[?u"),
            vec![b"\x1b[?1u".to_vec()]
        );

        runtime.feed_pty_output(b"\x1b[>9u");
        assert_eq!(
            runtime.feed_pty_output(b"\x1b[?u"),
            vec![b"\x1b[?9u".to_vec()]
        );

        runtime.feed_pty_output(b"\x1b[<u");
        assert_eq!(
            runtime.feed_pty_output(b"\x1b[?u"),
            vec![b"\x1b[?1u".to_vec()]
        );

        runtime.feed_pty_output(b"\x1b[<1u");
        assert_eq!(
            runtime.feed_pty_output(b"\x1b[?u"),
            vec![b"\x1b[?0u".to_vec()]
        );

        let text = terminal_text(&runtime);
        assert!(text.contains("before"));
        assert!(!text.contains("[?u"));
        assert!(!text.contains("[>"));
        assert!(!text.contains("[<"));
    }

    #[test]
    fn answers_kitty_keyboard_protocol_flags_queries_and_tracks_set_reset() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
        runtime.set_enable_kitty_keyboard(true);

        runtime.feed_pty_output(b"\x1b[=1u");
        assert_eq!(
            runtime.feed_pty_output(b"\x1b[?u"),
            vec![b"\x1b[?1u".to_vec()]
        );

        runtime.feed_pty_output(b"\x1b[=8;2u");
        assert_eq!(
            runtime.feed_pty_output(b"\x1b[?u"),
            vec![b"\x1b[?9u".to_vec()]
        );

        runtime.feed_pty_output(b"\x1b[=1;3u");
        assert_eq!(
            runtime.feed_pty_output(b"\x1b[?u"),
            vec![b"\x1b[?8u".to_vec()]
        );

        let text = terminal_text(&runtime);
        assert!(!text.contains("[="));
    }

    #[test]
    fn ignores_kitty_keyboard_protocol_when_disabled() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        runtime.feed_pty_output(b"\x1b[=1u");
        let output = runtime.feed_pty_output_with_display(b"\x1b[?u");

        assert!(output.responses.is_empty());
        assert_eq!(runtime.kitty_keyboard_flags(), 0);
    }

    #[test]
    fn tracks_win32_input_mode_when_allowed() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        assert!(!runtime.win32_input_mode());

        runtime.feed_pty_output(b"\x1b[?9001h");
        assert!(runtime.win32_input_mode());

        runtime.feed_pty_output(b"\x1b[?9001l");
        assert!(!runtime.win32_input_mode());
    }

    #[test]
    fn ignores_win32_input_mode_when_disallowed() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
        runtime.set_allow_win32_input_mode(false);

        runtime.feed_pty_output(b"\x1b[?9001h");

        assert!(!runtime.win32_input_mode());
    }

    #[test]
    fn answers_kitty_graphics_query_for_supported_direct_rgb_payload() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let output = runtime
            .feed_pty_output_with_display(b"before\x1b_Ga=q,i=31,t=d,f=24,s=1,v=1;/wAA\x1b\\after");

        assert_eq!(output.display, b"beforeafter");
        assert_eq!(output.responses, vec![b"\x1b_Gi=31;OK\x1b\\".to_vec()]);
        assert!(runtime.terminal().inline_images().is_empty());
        assert!(!terminal_text(&runtime).contains("_G"));
    }

    #[test]
    fn answers_kitty_graphics_query_for_supported_file_rgb_payload() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));
        let file = KittyTestFile::new(&[255, 0, 0]);
        let encoded_path = STANDARD.encode(file.path.as_os_str().to_string_lossy().as_bytes());
        let query =
            format!("before\x1b_Ga=q,i=32,t=f,f=24,s=1,v=1,c=1,r=1;{encoded_path}\x1b\\after");

        let output = runtime.feed_pty_output_with_display(query.as_bytes());

        assert_eq!(output.display, b"beforeafter");
        assert_eq!(output.responses, vec![b"\x1b_Gi=32;OK\x1b\\".to_vec()]);
        assert!(runtime.terminal().inline_images().is_empty());
        assert!(!terminal_text(&runtime).contains("_G"));
    }

    #[test]
    fn answers_kitty_graphics_placement_query_for_missing_image() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let output = runtime.feed_pty_output_with_display(b"before\x1b_Ga=p,i=404,p=2\x1b\\after");

        assert_eq!(output.display, b"beforeafter");
        assert_eq!(
            output.responses,
            vec![b"\x1b_Gi=404,p=2;ENOENT:No image with id 404\x1b\\".to_vec()]
        );
        assert!(runtime.terminal().inline_images().is_empty());
        assert!(!terminal_text(&runtime).contains("_G"));
    }

    #[test]
    fn answers_kitty_graphics_placement_query_for_existing_image() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let store =
            runtime.feed_pty_output_with_display(b"\x1b_Ga=t,i=7,f=24,s=1,v=1,c=1,r=1;/wAA\x1b\\");
        assert_eq!(store.responses, vec![b"\x1b_Gi=7;OK\x1b\\".to_vec()]);

        let output = runtime.feed_pty_output_with_display(b"before\x1b_Ga=p,i=7,p=2\x1b\\after");

        assert_eq!(output.display, b"beforeafter");
        assert_eq!(output.responses, vec![b"\x1b_Gi=7,p=2;OK\x1b\\".to_vec()]);
        assert_eq!(runtime.terminal().inline_images().len(), 1);
        assert!(!terminal_text(&runtime).contains("_G"));
    }

    #[test]
    fn answers_modify_other_keys_queries_and_tracks_set_reset() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let output = runtime.feed_pty_output_with_display(b"before\x1b[?4m");

        assert_eq!(output.display, b"before");
        assert_eq!(output.responses, vec![b"\x1b[>4;0m".to_vec()]);

        runtime.feed_pty_output(b"\x1b[>4;2m");
        assert_eq!(runtime.modify_other_keys(), 2);
        assert_eq!(
            runtime.feed_pty_output(b"\x1b[?4m"),
            vec![b"\x1b[>4;2m".to_vec()]
        );

        let text = terminal_text(&runtime);
        assert!(text.contains("before"));
        assert!(!text.contains("[>4"));
        assert!(!text.contains("[?4"));
    }

    #[test]
    fn resize_updates_terminal_grid_and_size_query_response() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(4, 2));
        runtime.feed_pty_output(b"abcd\r\nef");

        runtime.resize(TerminalSize::new(6, 3));
        let responses = runtime.feed_pty_output(b"\x1b[18t");

        assert_eq!(runtime.terminal().grid().size(), TerminalSize::new(6, 3));
        assert_eq!(responses, vec![b"\x1b[8;3;6t".to_vec()]);
    }

    #[test]
    fn answers_split_device_attribute_query() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(20, 2));

        let first = runtime.feed_pty_output(b"before\x1b[");
        let second = runtime.feed_pty_output(b">cafter");

        assert!(first.is_empty());
        assert_eq!(second, vec![b"\x1b[>1;277;0c".to_vec()]);

        let text = terminal_text(&runtime);
        assert!(text.contains("beforeafter"));
        assert!(!text.contains("[>c"));
    }

    fn terminal_text(runtime: &TerminalRuntime) -> String {
        let grid = runtime.terminal().grid();
        let size = grid.size();
        let mut text = String::new();

        for row in 0..size.rows {
            for column in 0..size.columns {
                text.push_str(grid.get(row, column).unwrap().text());
            }
        }

        text
    }

    fn xtgettcap_query(names: &[&[u8]]) -> Vec<u8> {
        let mut query = b"\x1bP+q".to_vec();
        for (index, name) in names.iter().enumerate() {
            if index > 0 {
                query.push(b';');
            }
            query.extend_from_slice(&super::encode_ascii_hex(name));
        }
        query.extend_from_slice(b"\x1b\\");
        query
    }

    fn xtgettcap_response(entries: &[(&[u8], &[u8])]) -> Vec<u8> {
        let mut response = Vec::new();
        for (name, value) in entries {
            response.extend_from_slice(b"\x1bP1+r");
            response.extend_from_slice(&encode_ascii_hex_upper(name));
            response.push(b'=');
            response.extend_from_slice(&encode_ascii_hex_upper(value));
            response.extend_from_slice(b"\x1b\\");
        }
        response
    }

    fn encode_ascii_hex_upper(bytes: &[u8]) -> Vec<u8> {
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        let mut encoded = Vec::with_capacity(bytes.len() * 2);
        for byte in bytes {
            encoded.push(HEX[usize::from(byte >> 4)]);
            encoded.push(HEX[usize::from(byte & 0x0f)]);
        }
        encoded
    }

    struct KittyTestFile {
        path: PathBuf,
    }

    impl KittyTestFile {
        fn new(data: &[u8]) -> Self {
            static NEXT_TEST_FILE_ID: AtomicUsize = AtomicUsize::new(0);

            let suffix = NEXT_TEST_FILE_ID.fetch_add(1, Ordering::Relaxed);
            let mut path = std::env::temp_dir();
            path.push(format!(
                "rterm-runtime-kitty-file-query-{}-{suffix}.rgb",
                std::process::id()
            ));
            fs::write(&path, data).expect("write runtime kitty query test image file");
            Self { path }
        }
    }

    impl Drop for KittyTestFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }
}
