//! Local-only window integration. Remote launches fail before starting a worker.
use super::{
    ConnectionState, Error, EventLoopProxy, Instant, Key, NativeWindowApp, PaneRenderLayout,
    PaneRuntime, RenderCell, SshOptions, SshPaneLaunch, WindowUserEvent,
};

pub(super) struct DisabledSshPaneState;

// Keep the same extension hooks in local builds, without allocating transport state.
#[allow(clippy::unused_self)]
impl NativeWindowApp {
    pub(super) fn cancel_ssh_runtime(&mut self, _: rssh_core::PaneId) {}
    pub(super) fn retire_ssh_connection_state(&mut self, _: rssh_core::PaneId) {}
    pub(super) fn handle_ssh_state(&mut self, _: rssh_core::PaneId, _: ConnectionState) {}
    pub(super) fn resolve_secret_prompt_for_pane(
        &mut self,
        _: rssh_core::PaneId,
        _: Option<String>,
    ) {
    }
    pub(super) fn ssh_connection_state_for_pane(&self, _: rssh_core::PaneId) -> ConnectionState {
        ConnectionState::NotStarted
    }
    pub(super) fn take_ssh_pane_auxiliary_state(
        &mut self,
        _: rssh_core::PaneId,
    ) -> DisabledSshPaneState {
        DisabledSshPaneState
    }
    pub(super) fn install_ssh_pane_auxiliary_state(
        &mut self,
        _: rssh_core::PaneId,
        _: DisabledSshPaneState,
    ) {
    }
    pub(super) fn ssh_connection_overlay_cells(&self, _: &PaneRenderLayout) -> Vec<RenderCell> {
        Vec::new()
    }
    pub(super) fn handle_ssh_prompt_key_event(&mut self, _: &Key, _: Option<&str>) -> bool {
        false
    }
    pub(super) fn spawn_native_ssh_runtime(
        &mut self,
        _: rssh_core::PaneId,
        _: &SshPaneLaunch,
        _: u64,
        _: EventLoopProxy<WindowUserEvent>,
    ) -> Result<PaneRuntime, Box<dyn Error>> {
        Err(crate::feature_disabled("ssh"))
    }
}

pub(crate) fn run_ssh_gui(_: &SshOptions, _: Instant) -> Result<(), Box<dyn Error>> {
    Err(crate::feature_disabled("ssh"))
}
