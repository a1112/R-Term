use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rterm_core::TerminalSize;
use rterm_runtime::RuntimeBuffers;
use rterm_terminal::Terminal;

pub(crate) use rterm_runtime::terminal::{TerminalNotification, TerminalProgress};

const PROCESS_CWD_PROBE_INTERVAL: Duration = Duration::from_secs(1);

pub(crate) struct TerminalRuntime {
    pub(crate) inner: rterm_runtime::TerminalRuntime,
    pub(crate) storage: TerminalRuntimeStorage,
    presentation_snapshot: Option<Arc<Terminal>>,
}

pub(crate) struct TerminalRuntimeStorage {
    pub(crate) buffers: RuntimeBuffers,
    process_cwd_probe: ProcessCwdProbeState,
}

#[derive(Default)]
struct ProcessCwdProbeState {
    process_id: Option<u32>,
    next_probe_at: Option<Instant>,
}

impl TerminalRuntime {
    pub(crate) fn new(size: TerminalSize) -> Self {
        Self {
            inner: rterm_runtime::TerminalRuntime::new(size),
            storage: TerminalRuntimeStorage::new(),
            presentation_snapshot: None,
        }
    }

    pub(crate) fn new_with_query_scan_work(size: TerminalSize) -> Self {
        Self {
            inner: rterm_runtime::TerminalRuntime::new_with_query_scan_work(size),
            storage: TerminalRuntimeStorage::new(),
            presentation_snapshot: None,
        }
    }

    pub(crate) fn from_terminal(terminal: Terminal) -> Self {
        Self {
            inner: rterm_runtime::TerminalRuntime::from_terminal(terminal),
            storage: TerminalRuntimeStorage::new(),
            presentation_snapshot: None,
        }
    }

    pub(crate) fn terminal(&self) -> &Terminal {
        self.presentation_snapshot
            .as_deref()
            .unwrap_or_else(|| self.inner.terminal())
    }

    pub(crate) fn install_presentation_snapshot(&mut self, terminal: Arc<Terminal>) {
        self.presentation_snapshot = Some(terminal);
    }

    pub(crate) fn should_probe_process_cwd(&mut self, process_id: u32, now: Instant) -> bool {
        let probe = &mut self.storage.process_cwd_probe;
        if probe.process_id != Some(process_id) {
            probe.process_id = Some(process_id);
            probe.next_probe_at = now.checked_add(PROCESS_CWD_PROBE_INTERVAL);
            return true;
        }

        if probe.next_probe_at.is_none_or(|deadline| now >= deadline) {
            probe.next_probe_at = now.checked_add(PROCESS_CWD_PROBE_INTERVAL);
            return true;
        }

        false
    }

    pub(crate) fn complete_process_cwd_probe(&mut self, process_id: u32, now: Instant) {
        let probe = &mut self.storage.process_cwd_probe;
        if probe.process_id == Some(process_id) {
            probe.next_probe_at = now.checked_add(PROCESS_CWD_PROBE_INTERVAL);
        }
    }

    pub(crate) fn reset_process_cwd_probe(&mut self) {
        self.storage.process_cwd_probe = ProcessCwdProbeState::default();
    }
}

impl TerminalRuntimeStorage {
    fn new() -> Self {
        Self {
            buffers: RuntimeBuffers::with_capacity(8192),
            process_cwd_probe: ProcessCwdProbeState::default(),
        }
    }
}

impl Deref for TerminalRuntime {
    type Target = rterm_runtime::TerminalRuntime;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for TerminalRuntime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_cwd_probe_is_throttled_and_invalidated_by_process_id() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let now = Instant::now();

        assert!(runtime.should_probe_process_cwd(10, now));
        assert!(!runtime.should_probe_process_cwd(10, now));
        assert!(runtime.should_probe_process_cwd(11, now));
        assert!(runtime.should_probe_process_cwd(11, now + PROCESS_CWD_PROBE_INTERVAL));

        runtime.reset_process_cwd_probe();
        assert!(runtime.should_probe_process_cwd(11, now));
    }

    #[test]
    fn process_cwd_probe_interval_starts_after_the_probe_completes() {
        let mut runtime = TerminalRuntime::new(TerminalSize::new(80, 24));
        let started = Instant::now();
        let completed = started + PROCESS_CWD_PROBE_INTERVAL + Duration::from_millis(1);

        assert!(runtime.should_probe_process_cwd(10, started));
        runtime.complete_process_cwd_probe(10, completed);

        assert!(!runtime.should_probe_process_cwd(10, completed));
        assert!(runtime.should_probe_process_cwd(10, completed + PROCESS_CWD_PROBE_INTERVAL));
    }

    #[test]
    fn presentation_snapshot_reuses_the_worker_terminal_arc() {
        let size = TerminalSize::new(80, 24);
        let worker = rterm_runtime::TerminalRuntime::new(size);
        let snapshot = std::sync::Arc::new(worker.terminal().clone());
        let mut presentation = TerminalRuntime::new(size);

        presentation.install_presentation_snapshot(std::sync::Arc::clone(&snapshot));

        assert!(std::ptr::eq(
            presentation.terminal(),
            std::sync::Arc::as_ptr(&snapshot)
        ));
    }
}
