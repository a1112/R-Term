use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    error::Error,
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Duration,
};

use rssh_core::{DamageRegion, PaneId, TerminalSize, WindowId};
use rssh_native::{
    ClipboardEffect, HostPorts, NotificationEffect, PaneLifecycleIntent, PersistenceEffect,
    PlatformEvent, PortError, PortErrorKind, RendererEffect, RuntimeDrain, RuntimePortEffect,
    SpawnEffect, TurnBudget, UriEffect, WindowIntent, WindowPortEffect, WindowState, WinitHost,
    input::{PendingPaneCommand, PendingPaneCommandQueue},
};
use rssh_pty::{LocalPtyTransport, PtySession};
#[cfg(feature = "ssh")]
use rssh_ssh::LazyRusshRuntime;
use rssh_terminal::Terminal;
use rterm_runtime::{
    PaneHandle, PaneMetadataDelta, PaneNotice, PaneToken, PaneWorkerConfig, RuntimeBatch,
    RuntimeBatchMetrics, RuntimeEffect, RuntimeHub, RuntimeRevision, SessionExit, SessionTransport,
    SubmitResult, SystemClock,
};

use crate::terminal_runtime::TerminalRuntime;

type AdoptLocalSession = fn(
    PaneRuntimeRoute,
    PtySession,
    TerminalSize,
    rterm_runtime::TerminalRuntime,
    PaneCapturePolicy,
    Arc<dyn Fn() + Send + Sync>,
) -> Result<SpawnedLocalPane, Box<dyn Error>>;

#[derive(Debug, Clone)]
pub(crate) struct RuntimeComposition {
    adopt_local_session: AdoptLocalSession,
    #[cfg(feature = "ssh")]
    ssh_runtime: Arc<LazyRusshRuntime>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PaneCapturePolicy {
    pub(crate) host_stream: bool,
    pub(crate) visible_output: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PaneRuntimeRoute {
    pub(crate) window: WindowId,
    pub(crate) pane: PaneId,
}

impl RuntimeComposition {
    pub(crate) fn new() -> Self {
        Self {
            adopt_local_session: WindowPaneRuntime::adopt_local_session,
            #[cfg(feature = "ssh")]
            ssh_runtime: Arc::new(LazyRusshRuntime::new()),
        }
    }

    #[cfg(feature = "ssh")]
    pub(crate) fn ssh_runtime_owner(&self) -> Arc<LazyRusshRuntime> {
        Arc::clone(&self.ssh_runtime)
    }

    #[cfg(test)]
    #[cfg(feature = "ssh")]
    pub(crate) fn ssh_runtime_initialized(&self) -> bool {
        self.ssh_runtime.is_initialized()
    }

    #[cfg(test)]
    #[cfg(feature = "ssh")]
    pub(crate) fn ssh_runtime_handle(
        &self,
    ) -> Result<rssh_ssh::RusshRuntimeHandle, rssh_ssh::SshSessionError> {
        self.ssh_runtime.get_or_try_init()
    }

    pub(crate) fn adopt_local_session(
        self,
        route: PaneRuntimeRoute,
        session: PtySession,
        size: TerminalSize,
        terminal: rterm_runtime::TerminalRuntime,
        capture: PaneCapturePolicy,
        notice_waker: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<SpawnedLocalPane, Box<dyn Error>> {
        (self.adopt_local_session)(route, session, size, terminal, capture, notice_waker)
    }
}

#[derive(Debug, Clone)]
pub(crate) enum RuntimeHostEvent {
    Frame {
        pane: PaneToken,
        terminal: Arc<Terminal>,
        damage: Vec<DamageRegion>,
        metadata: PaneMetadataDelta,
        metrics: RuntimeBatchMetrics,
        full_repaint: bool,
    },
    Bell {
        pane: PaneToken,
        count: u64,
    },
    ClipboardWrite {
        pane: PaneToken,
        selection: Option<String>,
        contents: String,
    },
    ClipboardRead {
        pane: PaneToken,
        selection: String,
    },
    Notification {
        pane: PaneToken,
        title: Option<String>,
        body: String,
    },
    Diagnostic {
        pane: Option<PaneToken>,
        message: String,
    },
    HostStream {
        pane: PaneToken,
        bytes: Vec<u8>,
    },
    VisibleOutput {
        pane: PaneToken,
        bytes: Vec<u8>,
    },
    ModeChange {
        pane: PaneToken,
        change: rterm_runtime::TerminalModeChange,
    },
    InputWriteCompleted {
        byte_count: usize,
        elapsed: Duration,
    },
    FirstPtyByte {
        observed_at: std::time::Instant,
    },
    RequestRedraw,
    Closed {
        pane: PaneToken,
        exit: Option<SessionExit>,
    },
}

pub(crate) struct WindowPaneRuntime {
    host: WinitHost<RuntimePorts>,
    token: PaneToken,
    closed: bool,
    pending_commands: HashMap<PaneId, PendingPaneCommandQueue>,
    closing: HashSet<PaneToken>,
}

pub(crate) type SpawnedLocalPane = (WindowPaneRuntime, Option<u32>, Option<String>);
pub(crate) type AdoptedLocalPane = (PaneToken, Option<u32>, Option<String>);

pub(crate) struct ActiveWindowRuntime {
    composition: RuntimeComposition,
    presentation: TerminalRuntime,
    worker: Option<WindowPaneRuntime>,
}

impl ActiveWindowRuntime {
    pub(crate) fn new(presentation: TerminalRuntime) -> Self {
        Self {
            composition: RuntimeComposition::new(),
            presentation,
            worker: None,
        }
    }

    pub(crate) fn set_composition(&mut self, composition: RuntimeComposition) {
        self.composition = composition;
    }

    pub(crate) fn composition(&self) -> RuntimeComposition {
        self.composition.clone()
    }

    pub(crate) const fn worker(&self) -> Option<&WindowPaneRuntime> {
        self.worker.as_ref()
    }

    pub(crate) fn worker_mut(&mut self) -> Option<&mut WindowPaneRuntime> {
        self.worker.as_mut()
    }

    pub(crate) fn install_worker(&mut self, worker: Option<WindowPaneRuntime>) {
        self.worker = worker;
    }

    pub(crate) fn take_worker(&mut self) -> Option<WindowPaneRuntime> {
        self.worker.take()
    }
}

impl Deref for ActiveWindowRuntime {
    type Target = TerminalRuntime;

    fn deref(&self) -> &Self::Target {
        &self.presentation
    }
}

impl DerefMut for ActiveWindowRuntime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.presentation
    }
}

impl WindowPaneRuntime {
    fn adopt_local_session(
        route: PaneRuntimeRoute,
        session: PtySession,
        size: TerminalSize,
        terminal: rterm_runtime::TerminalRuntime,
        capture: PaneCapturePolicy,
        notice_waker: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<SpawnedLocalPane, Box<dyn Error>> {
        let process_id = session.process_id();
        let tty_name = session.tty_name();
        let transport = LocalPtyTransport::from_session(session)?;
        let runtime =
            Self::open_transport(route, transport, size, terminal, capture, notice_waker)?;
        Ok((runtime, process_id, tty_name))
    }

    pub(crate) fn open_transport<T: SessionTransport>(
        route: PaneRuntimeRoute,
        transport: T,
        size: TerminalSize,
        terminal: rterm_runtime::TerminalRuntime,
        capture: PaneCapturePolicy,
        notice_waker: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut hub = RuntimeHub::new_with_notice_waker(SystemClock, Arc::clone(&notice_waker));
        let mut config = PaneWorkerConfig {
            size,
            ..PaneWorkerConfig::default()
        };
        config.capture_host_stream = capture.host_stream;
        config.capture_visible_output = capture.visible_output;
        let handle = hub.open_with_runtime(route.pane, transport, config, terminal)?;
        let token = handle.token();
        let ports = RuntimePorts::new(hub, handle.clone(), notice_waker);
        let mut state = WindowState::default();
        state.presentation.size = size;
        let mut host = WinitHost::new(
            route.window,
            state,
            ports,
            TurnBudget::new(64, Duration::from_millis(2)),
        );
        host.handle(PlatformEvent::PaneLifecycle(PaneLifecycleIntent::Opened(
            token,
        )))?;
        Ok(Self {
            host,
            token,
            closed: false,
            pending_commands: HashMap::from([(route.pane, PendingPaneCommandQueue::new())]),
            closing: HashSet::new(),
        })
    }

    #[cfg(test)]
    pub(crate) const fn token(&self) -> PaneToken {
        self.token
    }

    pub(crate) fn add_transport<T: SessionTransport>(
        &mut self,
        pane: PaneId,
        transport: T,
        config: PaneWorkerConfig,
        runtime: rterm_runtime::TerminalRuntime,
    ) -> Result<PaneToken, Box<dyn Error>> {
        let replacing = self.host.state().panes.contains_key(&pane);
        let handle = if replacing {
            self.host
                .ports_mut()
                .hub
                .restart_with_runtime(pane, transport, config, runtime)?
        } else {
            self.host
                .ports_mut()
                .hub
                .open_with_runtime(pane, transport, config, runtime)?
        };
        let token = handle.token();
        self.host.ports_mut().handles.insert(pane, handle);
        self.pending_commands
            .insert(pane, PendingPaneCommandQueue::new());
        self.host
            .handle(PlatformEvent::PaneLifecycle(PaneLifecycleIntent::Opened(
                token,
            )))?;
        self.closing.retain(|closing| closing.pane() != pane);
        self.closed = false;
        Ok(token)
    }

    pub(crate) fn adopt_additional_local_session(
        &mut self,
        pane: PaneId,
        session: PtySession,
        size: TerminalSize,
        terminal: rterm_runtime::TerminalRuntime,
        capture: PaneCapturePolicy,
    ) -> Result<AdoptedLocalPane, Box<dyn Error>> {
        let process_id = session.process_id();
        let tty_name = session.tty_name();
        let transport = LocalPtyTransport::from_session(session)?;
        let config = PaneWorkerConfig {
            size,
            capture_host_stream: capture.host_stream,
            capture_visible_output: capture.visible_output,
            ..PaneWorkerConfig::default()
        };
        let token = self.add_transport(pane, transport, config, terminal)?;
        Ok((token, process_id, tty_name))
    }

    pub(crate) fn pane_tokens(&self) -> Vec<PaneToken> {
        self.host
            .state()
            .pane_order
            .iter()
            .filter_map(|pane| self.host.state().panes.get(pane).map(|state| state.token))
            .collect()
    }

    pub(crate) fn token_for_pane(&self, pane: PaneId) -> Option<PaneToken> {
        self.host.state().panes.get(&pane).map(|state| state.token)
    }

    pub(crate) fn contains_pane(&self, pane: PaneId) -> bool {
        self.token_for_pane(pane).is_some()
    }

    #[cfg(test)]
    fn active_token(&self) -> PaneToken {
        self.token
    }

    pub(crate) fn activate(&mut self, token: PaneToken) -> Result<(), Box<dyn Error>> {
        self.host.handle(PlatformEvent::PaneLifecycle(
            PaneLifecycleIntent::Activated(token),
        ))?;
        if self.host.state().active_pane == Some(token.pane()) {
            self.token = token;
        }
        Ok(())
    }

    pub(crate) fn activate_pane(&mut self, pane: PaneId) -> Result<(), Box<dyn Error>> {
        let token = self.token_for_pane(pane).ok_or_else(|| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("runtime V2 pane {} is not open", pane.get()),
            )) as Box<dyn Error>
        })?;
        self.activate(token)
    }

    pub(crate) fn submit_input_to_pane(
        &mut self,
        pane: PaneId,
        bytes: &[u8],
    ) -> std::io::Result<()> {
        let token = self
            .token_for_pane(pane)
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::BrokenPipe))?;
        let handle = self
            .host
            .ports()
            .handles
            .get(&pane)
            .filter(|handle| handle.token() == token)
            .cloned()
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::BrokenPipe))?;
        self.pending_commands
            .get_mut(&pane)
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::BrokenPipe))?
            .submit_input(bytes, move |command| submit_pane_command(&handle, command))
    }

    pub(crate) fn resize_all(&mut self, size: TerminalSize) -> std::io::Result<()> {
        let panes = self.pane_tokens();
        for token in panes {
            let handle = self
                .host
                .ports()
                .handles
                .get(&token.pane())
                .filter(|handle| handle.token() == token)
                .cloned()
                .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::BrokenPipe))?;
            self.pending_commands
                .get_mut(&token.pane())
                .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::BrokenPipe))?
                .submit_resize(size, move |command| submit_pane_command(&handle, command))?;
        }
        Ok(())
    }

    fn begin_close_pane(&mut self, token: PaneToken, grace: Duration) -> bool {
        if self.closing.contains(&token) || !self.host.ports_mut().hub.begin_close(token, grace) {
            return false;
        }
        self.closing.insert(token);
        true
    }

    pub(crate) fn begin_close_by_pane(&mut self, pane: PaneId, grace: Duration) -> bool {
        let Some(token) = self.host.state().panes.get(&pane).map(|state| state.token) else {
            return false;
        };
        self.begin_close_pane(token, grace)
    }

    pub(crate) fn retire_pane_transport(&mut self, pane: PaneId) -> Result<bool, Box<dyn Error>> {
        let Some(token) = self.token_for_pane(pane) else {
            return Ok(false);
        };
        let _ = self.host.ports_mut().hub.begin_close(token, Duration::ZERO);
        let _ = self.host.ports_mut().hub.reap_expired();
        self.host
            .handle(PlatformEvent::PaneLifecycle(PaneLifecycleIntent::Closed(
                token,
            )))?;
        let ports = self.host.ports_mut();
        ports.handles.remove(&pane);
        ports
            .pending_frames
            .retain(|(candidate, _), _| *candidate != token);
        ports.pending_runtime_batches.remove(&token);
        ports.continuations.retain(|candidate| *candidate != token);
        self.pending_commands.remove(&pane);
        self.closing.remove(&token);
        if self.token == token
            && let Some(active) = self
                .host
                .state()
                .active_pane
                .and_then(|active| self.token_for_pane(active))
        {
            self.token = active;
        }
        self.closed = self.host.state().panes.is_empty();
        Ok(true)
    }

    pub(crate) fn poll(&mut self) -> Result<Vec<RuntimeHostEvent>, Box<dyn Error>> {
        let pending_panes = self.pending_commands.keys().copied().collect::<Vec<_>>();
        for pane in pending_panes {
            let Some(handle) = self.host.ports().handles.get(&pane).cloned() else {
                continue;
            };
            if let Some(pending) = self.pending_commands.get_mut(&pane) {
                pending.flush(move |command| submit_pane_command(&handle, command))?;
            }
        }
        let continuations = self.host.ports_mut().take_continuations();
        for pane in continuations {
            self.host
                .handle(PlatformEvent::RuntimeContinuation { pane })?;
        }
        if self.host.ports().has_continuations() {
            return Ok(self.host.ports_mut().events.drain(..).collect());
        }

        while let Ok(notice) = self.host.ports_mut().hub.try_recv_notice() {
            match notice {
                PaneNotice::Ready(_) => {}
                PaneNotice::Wake(pane) => {
                    self.host.handle(PlatformEvent::RuntimeWake { pane })?;
                    if self.host.ports().has_continuations() {
                        break;
                    }
                }
                PaneNotice::InputWriteCompleted {
                    pane: _,
                    byte_count,
                    elapsed,
                } => {
                    self.host
                        .ports_mut()
                        .events
                        .push_back(RuntimeHostEvent::InputWriteCompleted {
                            byte_count,
                            elapsed,
                        });
                }
                PaneNotice::FirstPtyByte { observed_at, .. } => self
                    .host
                    .ports_mut()
                    .events
                    .push_back(RuntimeHostEvent::FirstPtyByte { observed_at }),
                PaneNotice::Closed { pane, exit } => {
                    self.host
                        .handle(PlatformEvent::PaneLifecycle(PaneLifecycleIntent::Closed(
                            pane,
                        )))?;
                    self.host.ports_mut().handles.remove(&pane.pane());
                    self.pending_commands.remove(&pane.pane());
                    self.closing.remove(&pane);
                    if self.token == pane
                        && let Some(active) = self.host.state().active_pane.and_then(|active| {
                            self.host
                                .state()
                                .panes
                                .get(&active)
                                .map(|state| state.token)
                        })
                    {
                        self.token = active;
                    }
                    self.closed = self.host.state().panes.is_empty();
                    self.host
                        .ports_mut()
                        .events
                        .push_back(RuntimeHostEvent::Closed { pane, exit });
                }
            }
        }
        Ok(self.host.ports_mut().events.drain(..).collect())
    }

    pub(crate) fn needs_poll(&self) -> bool {
        self.pending_commands
            .values()
            .any(|pending| !pending.is_empty())
            || !self.host.ports().continuations.is_empty()
    }

    pub(crate) fn live_thread_count_for_metrics(&self) -> usize {
        self.host.ports().hub.live_thread_count()
    }

    pub(crate) fn shutdown(&mut self) {
        self.host.ports_mut().hub.shutdown();
        self.closed = true;
    }
}

fn submit_pane_command(handle: &PaneHandle, command: PendingPaneCommand) -> SubmitResult {
    match command {
        PendingPaneCommand::Input(bytes) => handle.submit_input(bytes),
        PendingPaneCommand::Resize(size) => handle.resize(size),
    }
}

impl Drop for WindowPaneRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct PendingFrame {
    terminal: Arc<Terminal>,
    metadata: PaneMetadataDelta,
    metrics: RuntimeBatchMetrics,
    full_repaint: bool,
}

struct DrainBatch {
    frame: Option<rterm_runtime::PresentationFrame>,
    effects: Vec<RuntimeEffect>,
}

struct RuntimePorts {
    hub: RuntimeHub,
    handles: HashMap<PaneId, PaneHandle>,
    pending_frames: HashMap<(PaneToken, RuntimeRevision), PendingFrame>,
    pending_runtime_batches: HashMap<PaneToken, BTreeMap<RuntimeRevision, DrainBatch>>,
    events: VecDeque<RuntimeHostEvent>,
    continuations: VecDeque<PaneToken>,
    continuation_waker: Arc<dyn Fn() + Send + Sync>,
}

impl RuntimePorts {
    fn new(
        hub: RuntimeHub,
        handle: PaneHandle,
        continuation_waker: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        Self {
            hub,
            handles: HashMap::from([(handle.token().pane(), handle)]),
            pending_frames: HashMap::new(),
            pending_runtime_batches: HashMap::new(),
            events: VecDeque::new(),
            continuations: VecDeque::new(),
            continuation_waker,
        }
    }

    fn take_continuations(&mut self) -> Vec<PaneToken> {
        self.continuations.drain(..).collect()
    }

    fn has_continuations(&self) -> bool {
        !self.continuations.is_empty()
    }

    fn unavailable(message: impl Into<String>) -> PortError {
        PortError::new(PortErrorKind::Unavailable, message)
    }

    fn submit_result(result: SubmitResult) -> Result<(), PortError> {
        match result {
            SubmitResult::Accepted => Ok(()),
            SubmitResult::Backpressured { .. } => Err(PortError::new(
                PortErrorKind::Backpressure,
                "pane command mailbox is full",
            )),
            SubmitResult::Closed => Err(Self::unavailable("pane runtime is closed")),
        }
    }
}

impl HostPorts for RuntimePorts {
    fn runtime(&mut self, effect: &RuntimePortEffect) -> Result<(), PortError> {
        match effect {
            RuntimePortEffect::SubmitInput { pane, bytes } => {
                let handle = self
                    .handles
                    .get(&pane.pane())
                    .filter(|handle| handle.token() == *pane)
                    .ok_or_else(|| Self::unavailable("pane input target is stale"))?;
                Self::submit_result(handle.submit_input(bytes.clone()))
            }
            RuntimePortEffect::ResizePane { pane, size } => {
                let handle = self
                    .handles
                    .get(&pane.pane())
                    .filter(|handle| handle.token() == *pane)
                    .ok_or_else(|| Self::unavailable("pane resize target is stale"))?;
                Self::submit_result(handle.resize(*size))
            }
            RuntimePortEffect::BeginClose { pane } => {
                if self.hub.begin_close(*pane, Duration::from_millis(250)) {
                    Ok(())
                } else {
                    Err(Self::unavailable("pane close target is stale"))
                }
            }
            RuntimePortEffect::WriteTransport { .. } => Err(PortError::new(
                PortErrorKind::Rejected,
                "pane worker owns transport response writes",
            )),
            RuntimePortEffect::ObserveHostStream { context, bytes } => {
                self.events.push_back(RuntimeHostEvent::HostStream {
                    pane: context.pane,
                    bytes: bytes.clone(),
                });
                Ok(())
            }
            RuntimePortEffect::WriteSessionLog { context, bytes } => {
                self.events.push_back(RuntimeHostEvent::VisibleOutput {
                    pane: context.pane,
                    bytes: bytes.clone(),
                });
                Ok(())
            }
            RuntimePortEffect::ApplyModeChange { context, change } => {
                self.events.push_back(RuntimeHostEvent::ModeChange {
                    pane: context.pane,
                    change: *change,
                });
                Ok(())
            }
            RuntimePortEffect::Restart { .. } => Err(Self::unavailable(
                "runtime selector does not own pane restart",
            )),
        }
    }

    fn window(&mut self, effect: &WindowPortEffect) -> Result<(), PortError> {
        match effect {
            WindowPortEffect::RequestRedraw => {
                self.events.push_back(RuntimeHostEvent::RequestRedraw);
            }
            WindowPortEffect::ReportDiagnostic(message) => {
                self.events.push_back(RuntimeHostEvent::Diagnostic {
                    pane: None,
                    message: message.clone(),
                });
            }
            WindowPortEffect::RuntimeDiagnostic { context, message } => {
                self.events.push_back(RuntimeHostEvent::Diagnostic {
                    pane: Some(context.pane),
                    message: message.clone(),
                });
            }
            WindowPortEffect::SetFocused(_)
            | WindowPortEffect::SetTitle(_)
            | WindowPortEffect::CloseAfterRuntimes
            | WindowPortEffect::CloseNow => {}
        }
        Ok(())
    }

    fn renderer(&mut self, effect: &RendererEffect) -> Result<(), PortError> {
        match effect {
            RendererEffect::ApplyPane {
                pane,
                revision,
                damage,
                ..
            } => {
                let frame = self
                    .pending_frames
                    .remove(&(*pane, *revision))
                    .ok_or_else(|| Self::unavailable("missing full terminal presentation"))?;
                self.events.push_back(RuntimeHostEvent::Frame {
                    pane: *pane,
                    terminal: frame.terminal,
                    damage: damage.clone(),
                    metadata: frame.metadata,
                    metrics: frame.metrics,
                    full_repaint: frame.full_repaint,
                });
            }
            RendererEffect::Bell { context, count } => {
                self.events.push_back(RuntimeHostEvent::Bell {
                    pane: context.pane,
                    count: count.get(),
                });
            }
            RendererEffect::ResizeSurface(_)
            | RendererEffect::ApplyConfig { .. }
            | RendererEffect::RecoverDevice
            | RendererEffect::Present => {}
        }
        Ok(())
    }

    fn clipboard(&mut self, effect: &ClipboardEffect) -> Result<(), PortError> {
        match effect {
            ClipboardEffect::Write {
                context,
                selection,
                contents,
            } => {
                let pane = context
                    .as_ref()
                    .map(|context| context.pane)
                    .ok_or_else(|| Self::unavailable("runtime clipboard write lacks context"))?;
                self.events.push_back(RuntimeHostEvent::ClipboardWrite {
                    pane,
                    selection: selection.clone(),
                    contents: contents.clone(),
                });
            }
            ClipboardEffect::Read { context, selection } => {
                self.events.push_back(RuntimeHostEvent::ClipboardRead {
                    pane: context.pane,
                    selection: selection.clone(),
                });
            }
        }
        Ok(())
    }

    fn uri(&mut self, _effect: &UriEffect) -> Result<(), PortError> {
        Ok(())
    }

    fn notification(&mut self, effect: &NotificationEffect) -> Result<(), PortError> {
        let NotificationEffect::Show {
            context,
            title,
            body,
        } = effect;
        self.events.push_back(RuntimeHostEvent::Notification {
            pane: context.pane,
            title: title.clone(),
            body: body.clone(),
        });
        Ok(())
    }

    fn persistence(&mut self, _effect: &PersistenceEffect) -> Result<(), PortError> {
        Ok(())
    }

    fn spawn(&mut self, _effect: &SpawnEffect) -> Result<(), PortError> {
        Err(Self::unavailable(
            "runtime selector does not own pane spawning",
        ))
    }

    fn drain_runtime(
        &mut self,
        pane: PaneToken,
        max_batches: usize,
    ) -> Result<RuntimeDrain, PortError> {
        let drain = self
            .hub
            .drain_pane(pane, max_batches)
            .ok_or_else(|| Self::unavailable("pane drain target is stale"))?;
        let batches = self.pending_runtime_batches.entry(pane).or_default();
        for published in drain.effects {
            batches
                .entry(published.revision)
                .or_insert_with(|| DrainBatch {
                    frame: None,
                    effects: Vec::new(),
                })
                .effects
                .push(published.effect);
        }
        let mut ready_revisions = Vec::new();
        if let Some(frame) = drain.frame {
            let revision = frame.revision;
            batches
                .entry(revision)
                .or_insert_with(|| DrainBatch {
                    frame: None,
                    effects: Vec::new(),
                })
                .frame = Some(frame);
            ready_revisions.extend(
                batches
                    .keys()
                    .copied()
                    .take_while(|candidate| *candidate <= revision),
            );
        }
        let batches = ready_revisions
            .into_iter()
            .filter_map(|revision| {
                self.pending_runtime_batches
                    .get_mut(&pane)
                    .and_then(|batches| batches.remove(&revision))
                    .map(|batch| (revision, batch))
            })
            .collect::<Vec<_>>();
        if self
            .pending_runtime_batches
            .get(&pane)
            .is_some_and(BTreeMap::is_empty)
        {
            self.pending_runtime_batches.remove(&pane);
        }

        let intents = batches
            .into_iter()
            .map(|(revision, batch)| {
                let (snapshot, damage, metadata, metrics) = if let Some(frame) = batch.frame {
                    self.pending_frames.insert(
                        (pane, revision),
                        PendingFrame {
                            terminal: Arc::clone(&frame.snapshot),
                            metadata: frame.metadata.clone(),
                            metrics: frame.metrics,
                            full_repaint: frame.full_repaint,
                        },
                    );
                    (
                        Some(Arc::new(frame.state)),
                        frame.damage,
                        frame.metadata,
                        frame.metrics,
                    )
                } else {
                    (
                        None,
                        Vec::new(),
                        PaneMetadataDelta::default(),
                        RuntimeBatchMetrics::default(),
                    )
                };
                WindowIntent::RuntimeBatch(RuntimeBatch {
                    pane,
                    revision,
                    snapshot,
                    damage,
                    metadata,
                    effects: batch.effects,
                    metrics,
                })
            })
            .collect();
        Ok(RuntimeDrain {
            intents,
            continuation: drain.continuation,
        })
    }

    fn schedule_runtime_continuation(
        &mut self,
        _window: WindowId,
        pane: PaneToken,
    ) -> Result<(), PortError> {
        if !self.continuations.contains(&pane) {
            self.continuations.push_back(pane);
            (self.continuation_waker)();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use rssh_core::{PaneId, WindowId};
    use rterm_runtime::{
        PaneWorkerConfig, TerminalRuntime,
        testing::{ReadAction, ScriptedTransport, WriteAction},
    };

    #[cfg(feature = "ssh")]
    use super::RuntimeComposition;
    use super::{PaneCapturePolicy, PaneRuntimeRoute, TerminalSize, WindowPaneRuntime};

    #[test]
    #[cfg(feature = "ssh")]
    fn cloned_compositions_share_one_lazy_ssh_runtime_without_initializing_it() {
        let first = RuntimeComposition::new();
        let second = first.clone();

        assert!(!first.ssh_runtime_initialized());
        assert!(!second.ssh_runtime_initialized());

        let first_handle = first.ssh_runtime_handle().expect("initialize SSH runtime");
        let second_handle = second.ssh_runtime_handle().expect("reuse SSH runtime");

        assert!(first_handle.shares_runtime_with(&second_handle));
        assert!(first.ssh_runtime_initialized());
        assert!(second.ssh_runtime_initialized());
    }

    fn runtime_with_queued_output_batches(
        output_batches: usize,
        notice_waker: Arc<dyn Fn() + Send + Sync>,
    ) -> (
        WindowPaneRuntime,
        rterm_runtime::testing::ScriptedSessionDriver,
        PaneId,
    ) {
        let size = TerminalSize::new(80, 24);
        let (transport, driver) = ScriptedTransport::new(
            [ReadAction::Block],
            std::iter::repeat_n(WriteAction::accept(usize::MAX), output_batches),
            [],
        );
        let pane = PaneId::new(200);
        let mut runtime = WindowPaneRuntime::open_transport(
            PaneRuntimeRoute {
                window: WindowId::new(1),
                pane,
            },
            transport,
            size,
            TerminalRuntime::new(size),
            PaneCapturePolicy {
                host_stream: true,
                visible_output: false,
            },
            notice_waker,
        )
        .expect("local worker");
        driver.wait_until_reader_blocked();
        let token = runtime.token_for_pane(pane).expect("queued pane token");

        for byte in 0..output_batches {
            driver.push_reads([
                ReadAction::bytes(vec![b'a' + u8::try_from(byte % 26).unwrap()]),
                ReadAction::Block,
            ]);
            // Reader blocking only proves that bytes entered the worker inbox.
            // Wait for publication before feeding another byte so coalescing
            // cannot turn the intended 70 batches into fewer than one drain.
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            loop {
                let metrics = runtime
                    .host
                    .ports()
                    .hub
                    .publication_metrics(token)
                    .expect("live pane publication");
                if metrics.source_bytes == (byte + 1) as u64 {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "output batch {byte} was not published: {metrics:?}"
                );
                std::thread::yield_now();
            }
            runtime
                .submit_input_to_pane(pane, &[b'0' + u8::try_from(byte % 10).unwrap()])
                .expect("separator input");
            driver.wait_until_accepted_write_len(byte + 1);
            driver.wait_until_reader_blocked();
        }
        (runtime, driver, pane)
    }

    #[test]
    fn scheduled_runtime_continuation_wakes_the_window_without_another_transport_notice() {
        const OUTPUT_BATCHES: usize = 70;
        let wakes = Arc::new(AtomicUsize::new(0));
        let wake_counter = Arc::clone(&wakes);
        let (mut runtime, _driver, _pane) =
            runtime_with_queued_output_batches(OUTPUT_BATCHES, Arc::new(|| {}));
        // Count continuation requests only; a late transport notice must not
        // make this assertion pass when continuation scheduling is broken.
        runtime.host.ports_mut().continuation_waker = Arc::new(move || {
            wake_counter.fetch_add(1, Ordering::Relaxed);
        });

        runtime.poll().expect("poll first bounded runtime turn");

        assert!(runtime.host.ports().has_continuations());
        assert_eq!(
            wakes.load(Ordering::Relaxed),
            1,
            "the continuation must schedule another window turn"
        );
    }

    #[test]
    fn close_drains_every_published_runtime_batch_before_closed_event() {
        const OUTPUT_BATCHES: usize = 70;
        let (mut runtime, driver, pane) =
            runtime_with_queued_output_batches(OUTPUT_BATCHES, Arc::new(|| {}));
        let token = runtime.token_for_pane(pane).expect("queued pane token");
        driver.push_read(ReadAction::Eof);
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while runtime.live_thread_count_for_metrics() != 0 {
            assert!(
                std::time::Instant::now() < deadline,
                "worker did not publish close"
            );
            std::thread::yield_now();
        }
        let mut observed = Vec::new();
        let mut observed_per_turn = Vec::new();
        let mut closed = false;
        while !closed {
            let events = runtime.poll().expect("poll queued output and close");
            let before = observed.len();
            observed.extend(
                events
                    .iter()
                    .filter_map(|event| match event {
                        super::RuntimeHostEvent::HostStream { bytes, .. } => Some(bytes.as_slice()),
                        _ => None,
                    })
                    .flatten()
                    .copied(),
            );
            observed_per_turn.push(observed.len() - before);
            closed = matches!(events.last(), Some(super::RuntimeHostEvent::Closed { .. }));
        }
        assert_eq!(
            observed.len(),
            OUTPUT_BATCHES,
            "observed per turn: {observed_per_turn:?}; publication: {:?}",
            runtime.host.ports().hub.publication_metrics(token)
        );
    }

    #[test]
    fn one_runtime_revision_is_not_split_into_stale_host_batches() {
        let size = TerminalSize::new(80, 24);
        let payload = b"\x1b[?1000hX\x1b[?1000l".repeat(80);
        let (transport, driver) = ScriptedTransport::new(
            [ReadAction::bytes(payload.clone()), ReadAction::Block],
            [WriteAction::accept(usize::MAX)],
            [],
        );
        let pane = PaneId::new(201);
        let mut runtime = WindowPaneRuntime::open_transport(
            PaneRuntimeRoute {
                window: WindowId::new(1),
                pane,
            },
            transport,
            size,
            TerminalRuntime::new(size),
            PaneCapturePolicy {
                host_stream: true,
                visible_output: false,
            },
            Arc::new(|| {}),
        )
        .expect("local worker");
        let token = runtime.token_for_pane(pane).expect("pane token");
        driver.wait_until_reader_blocked();
        runtime
            .submit_input_to_pane(pane, b"barrier")
            .expect("barrier input");
        driver.wait_until_accepted_write_len("barrier".len());

        let mut observed = Vec::new();
        loop {
            observed.extend(
                runtime
                    .poll()
                    .expect("poll bounded runtime turn")
                    .into_iter()
                    .filter_map(|event| match event {
                        super::RuntimeHostEvent::HostStream { bytes, .. } => Some(bytes),
                        _ => None,
                    })
                    .flatten(),
            );
            if !runtime.needs_poll() {
                break;
            }
        }

        let metrics = runtime
            .host
            .ports()
            .hub
            .publication_metrics(token)
            .expect("publication metrics");
        assert!(
            metrics.effects.enqueued_items > 64,
            "fixture must exceed one bounded host turn: {metrics:?}"
        );
        assert_eq!(observed, payload);
    }

    #[test]
    fn window_runtime_owns_multiple_pane_workers_and_active_selection() {
        let size = TerminalSize::new(80, 24);
        let (first_transport, first_driver) =
            ScriptedTransport::new([ReadAction::Block], [WriteAction::accept(usize::MAX)], []);
        let mut runtime = WindowPaneRuntime::open_transport(
            PaneRuntimeRoute {
                window: WindowId::new(1),
                pane: PaneId::new(201),
            },
            first_transport,
            size,
            TerminalRuntime::new(size),
            PaneCapturePolicy {
                host_stream: false,
                visible_output: false,
            },
            Arc::new(|| {}),
        )
        .expect("open first pane");
        let first = runtime.token();
        let (second_transport, second_driver) =
            ScriptedTransport::new([ReadAction::Block], [WriteAction::accept(usize::MAX)], []);
        let second = runtime
            .add_transport(
                PaneId::new(202),
                second_transport,
                PaneWorkerConfig::default(),
                TerminalRuntime::new(size),
            )
            .expect("open second pane");

        assert_eq!(runtime.pane_tokens(), vec![first, second]);
        assert!(runtime.contains_pane(first.pane()));
        assert!(runtime.contains_pane(second.pane()));
        assert!(!runtime.contains_pane(PaneId::new(999)));
        runtime.activate(second).expect("activate second pane");
        assert_eq!(runtime.active_token(), second);
        runtime
            .submit_input_to_pane(first.pane(), b"first-only")
            .expect("target first pane input");
        first_driver.wait_until_accepted_write_len("first-only".len());
        assert_eq!(first_driver.accepted_writes(), b"first-only");
        assert!(second_driver.accepted_writes().is_empty());
        let resized = TerminalSize::new(120, 40);
        runtime.resize_all(resized).expect("resize all panes");
        first_driver.wait_until_control_call_count(1);
        second_driver.wait_until_control_call_count(1);
        assert_eq!(first_driver.control_log().resizes, vec![resized]);
        assert_eq!(second_driver.control_log().resizes, vec![resized]);
        assert!(runtime.begin_close_pane(first, std::time::Duration::ZERO));
        assert!(!runtime.begin_close_pane(first, std::time::Duration::ZERO));
        runtime.shutdown();
        assert_eq!(runtime.live_thread_count_for_metrics(), 0);
    }

    #[test]
    fn v2_worker_ownership_is_window_scoped_instead_of_pane_scoped() {
        let window_source = [
            include_str!("window.rs"),
            include_str!("window_parts/part10.rs"),
            include_str!("window_parts/part15.rs"),
        ]
        .join("\n");
        assert!(
            !window_source.contains("v2_runtime: Option<WindowPaneRuntime>"),
            "pane-local ownership would create one RuntimeHub per pane"
        );
        assert!(
            !window_source.contains("self.app_shell.pane_ids().len() == 1"),
            "V2 ownership must not silently fall back after the first pane"
        );
    }
}
