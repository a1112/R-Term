use std::io::{self, Write};

use super::{
    LocalControlEvent, LocalMetricsCounters, Osc52Policy, PtySize, RuntimeBuffers, RuntimeDelta,
    RuntimeEffectRef, SharedTerminalRuntime, SharedTerminalSize, TerminalModeChange,
    TerminalVisibleOutputFilter, encode_osc52_clipboard_response, terminal_size_from_pty,
};

pub(super) fn local_control_event_from_mode_change(
    change: TerminalModeChange,
) -> Option<LocalControlEvent> {
    match change {
        TerminalModeChange::ApplicationCursorKeys(enabled) => {
            Some(LocalControlEvent::SetApplicationCursorKeys(enabled))
        }
        TerminalModeChange::ApplicationKeypad(enabled) => {
            Some(LocalControlEvent::SetApplicationKeypad(enabled))
        }
        TerminalModeChange::BracketedPaste(enabled) => {
            Some(LocalControlEvent::SetBracketedPaste(enabled))
        }
        TerminalModeChange::Mouse(mode) => Some(LocalControlEvent::SetMouseReporting(mode)),
        TerminalModeChange::Focus(enabled) => Some(LocalControlEvent::SetFocusReporting(enabled)),
        TerminalModeChange::KittyKeyboardFlags(flags) => {
            Some(LocalControlEvent::SetKittyKeyboardFlags(flags))
        }
        TerminalModeChange::ModifyOtherKeys(mode) => {
            Some(LocalControlEvent::SetModifyOtherKeys(mode))
        }
        TerminalModeChange::Win32InputMode(enabled) => {
            Some(LocalControlEvent::SetWin32InputMode(enabled))
        }
        TerminalModeChange::SynchronizedOutput(_) => None,
    }
}

/// Local host adapter whose runtime state advances before fallible host effects run.
///
/// A host I/O error is terminal for `copy_pty_output`: the runtime has consumed the
/// complete input batch, while only the successful host-effect prefix was committed.
/// Keep that lifecycle explicit by rejecting feed or finish attempts after an error.
pub(super) struct LocalTerminalRuntime {
    runtime: SharedTerminalRuntime,
    buffers: RuntimeBuffers,
    size: SharedTerminalSize,
    applied_size: PtySize,
    osc52_policy: Osc52Policy,
    host_io_failed: bool,
}

impl LocalTerminalRuntime {
    pub(super) fn new(
        size: SharedTerminalSize,
        terminal_name: String,
        osc52_policy: Osc52Policy,
    ) -> Self {
        let applied_size = size.snapshot();
        let mut runtime = SharedTerminalRuntime::new(terminal_size_from_pty(applied_size));
        runtime.set_terminal_name(terminal_name);
        runtime.set_enable_kitty_keyboard(true);
        runtime.set_capture_host_stream(true);
        Self {
            runtime,
            buffers: RuntimeBuffers::with_capacity(16 * 1024),
            size,
            applied_size,
            osc52_policy,
            host_io_failed: false,
        }
    }

    pub(super) fn write_with_clipboard(
        &mut self,
        bytes: &[u8],
        output: &mut dyn Write,
        respond: impl FnMut(&[u8]) -> io::Result<()>,
        write_clipboard: impl FnMut(&str) -> bool,
        read_clipboard: impl FnMut() -> Option<String>,
        mode_change: impl FnMut(TerminalModeChange),
    ) -> io::Result<()> {
        self.ensure_host_available()?;
        self.sync_size();
        let mut buffers = std::mem::take(&mut self.buffers);
        let delta = self.runtime.feed_into(bytes, &mut buffers);
        let result = apply_local_runtime_delta(
            delta,
            output,
            respond,
            write_clipboard,
            read_clipboard,
            self.osc52_policy,
            mode_change,
        );
        self.buffers = buffers;
        if result.is_err() {
            self.host_io_failed = true;
        }
        result
    }

    pub(super) fn finish(
        &mut self,
        output: &mut dyn Write,
        respond: impl FnMut(&[u8]) -> io::Result<()>,
        write_clipboard: impl FnMut(&str) -> bool,
        read_clipboard: impl FnMut() -> Option<String>,
        _mode_change: impl FnMut(TerminalModeChange),
    ) -> io::Result<()> {
        self.ensure_host_available()?;
        let mut buffers = std::mem::take(&mut self.buffers);
        let delta = self.runtime.finish_into(&mut buffers);
        let result = apply_local_runtime_delta(
            delta,
            output,
            respond,
            write_clipboard,
            read_clipboard,
            self.osc52_policy,
            |_| {},
        );
        self.buffers = buffers;
        if result.is_err() {
            self.host_io_failed = true;
        }
        result
    }

    fn ensure_host_available(&self) -> io::Result<()> {
        if self.host_io_failed {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "local terminal runtime cannot be reused after a host I/O error",
            ));
        }
        Ok(())
    }

    fn sync_size(&mut self) {
        let size = self.size.snapshot();
        if size != self.applied_size {
            self.runtime.resize(terminal_size_from_pty(size));
            self.applied_size = size;
        }
    }
}

fn apply_local_runtime_delta(
    delta: RuntimeDelta<'_>,
    output: &mut dyn Write,
    mut respond: impl FnMut(&[u8]) -> io::Result<()>,
    mut write_clipboard: impl FnMut(&str) -> bool,
    mut read_clipboard: impl FnMut() -> Option<String>,
    osc52_policy: Osc52Policy,
    mut mode_change: impl FnMut(TerminalModeChange),
) -> io::Result<()> {
    for effect in delta.effects() {
        match effect {
            RuntimeEffectRef::ConsoleWrite(bytes) => output.write_all(bytes)?,
            RuntimeEffectRef::TransportWrite(bytes) => respond(bytes)?,
            RuntimeEffectRef::ClipboardWrite { contents, .. } => {
                if osc52_policy.allows_write() {
                    let _ = write_clipboard(contents);
                }
            }
            RuntimeEffectRef::ClipboardRead { selection } => {
                if osc52_policy.allows_query()
                    && let Some(text) = read_clipboard()
                {
                    let response = encode_osc52_clipboard_response(selection, &text);
                    respond(&response)?;
                }
            }
            RuntimeEffectRef::ModeChange(_)
            | RuntimeEffectRef::Bell { .. }
            | RuntimeEffectRef::Notification { .. }
            | RuntimeEffectRef::Diagnostic { .. } => {}
        }
    }
    for change in delta.mode_changes() {
        mode_change(change);
    }
    Ok(())
}

pub(super) struct SessionLogWriter<'screen, 'log> {
    screen: &'screen mut dyn Write,
    log: Option<&'log mut dyn Write>,
    log_filter: TerminalVisibleOutputFilter,
    metrics: LocalMetricsCounters,
}

impl<'screen, 'log> SessionLogWriter<'screen, 'log> {
    pub(super) fn new(
        screen: &'screen mut dyn Write,
        log: Option<&'log mut dyn Write>,
        metrics: LocalMetricsCounters,
    ) -> Self {
        Self {
            screen,
            log,
            log_filter: TerminalVisibleOutputFilter::default(),
            metrics,
        }
    }
}

impl Write for SessionLogWriter<'_, '_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let count = self.screen.write(buffer)?;
        if count > 0 {
            self.metrics.add_terminal_output(count as u64);
            if let Some(log) = self.log.as_mut() {
                log.write_all(&self.log_filter.process(&buffer[..count]))?;
            }
        }
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.screen.flush()?;
        if let Some(log) = self.log.as_mut() {
            log.flush()?;
        }
        Ok(())
    }
}
