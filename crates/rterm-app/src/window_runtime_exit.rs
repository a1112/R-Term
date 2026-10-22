use super::{NativeWindowApp, PtyExitStatus, report_pane_pty_cleanup};

impl NativeWindowApp {
    pub(super) fn finish_pane_runtime_after_exit(
        &mut self,
        pane_id: rterm_core::PaneId,
        runtime_generation: u64,
    ) -> Option<PtyExitStatus> {
        let observed_status = self
            .metrics
            .observed_pane_exit_statuses
            .remove(&(pane_id, runtime_generation));
        if pane_id == self.app_shell.active_pane_id() {
            return self
                .finish_active_runtime_after_exit()
                .or(observed_status);
        }

        self.cancel_ssh_runtime(pane_id);
        let mut runtime = self.pane_runtimes.remove(&pane_id)?;
        if let Err(error) = self.finish_inactive_pane_output(pane_id, &mut runtime) {
            eprintln!("inactive pane terminal finish failed: {error}");
        }
        let cleanup = runtime.finish_after_exit();
        report_pane_pty_cleanup("inactive pane exit cleanup", &cleanup);
        self.pane_runtimes.insert(pane_id, runtime);
        cleanup.status.or(observed_status)
    }
}
