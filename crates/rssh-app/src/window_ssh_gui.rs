use super::{
    Arc, ConnectionState, DEFAULT_UI_SURFACE_BACKGROUND, DEFAULT_UI_SURFACE_FOREGROUND, Duration,
    Error, EventLoopProxy, HostKeyChallenge, HostKeyDecision, HostKeyVerifier, Instant, Key,
    NamedKey, NativeSshCommand, NativeSshWriter, NativeWindowApp, Ordering, PaneLaunchDomain,
    PaneRenderLayout, PaneRuntime, PaneRuntimeTransportKind, PaneStableViewport, PaneUiState,
    RenderCell, RusshChannelOpener, SecretPrompt, SecretPromptKind, SecretProvider,
    SshChannelConnector, SshConnectionPhase, SshKnownHostsPolicy, SshPaneAuxiliaryState,
    SshPaneLaunch, SshSecretPromptState, SshShellConnector, SshShellWriter, WindowUserEvent, Write,
    connection_metric_name, mpsc, russh_host_key_policy, ssh_known_hosts_path,
    ssh_request_from_pane_launch, terminal_runtime_snapshot, thread, ui_render_cell,
};
use rssh_ssh::HostKeyStatus;

fn native_ssh_command_channel() -> (
    mpsc::Sender<NativeSshCommand>,
    mpsc::Receiver<NativeSshCommand>,
) {
    mpsc::channel()
}

#[cfg(test)]
fn attach_native_ssh_writer(
    remote_writer: &mut Option<Box<dyn SshShellWriter>>,
    pending_resize: &mut Option<rssh_core::TerminalSize>,
    writer: Box<dyn SshShellWriter>,
) -> Result<bool, rssh_ssh::SshSessionError> {
    let cancellation = std::sync::atomic::AtomicBool::new(false);
    attach_native_ssh_writer_cancellable(
        remote_writer,
        pending_resize,
        writer,
        &cancellation,
    )
}

fn attach_native_ssh_writer_cancellable(
    remote_writer: &mut Option<Box<dyn SshShellWriter>>,
    pending_resize: &mut Option<rssh_core::TerminalSize>,
    mut writer: Box<dyn SshShellWriter>,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<bool, rssh_ssh::SshSessionError> {
    if cancelled.load(Ordering::Acquire) {
        return Ok(false);
    }
    if let Some(size) = pending_resize.take() {
        match writer.resize_cancellable(size.into(), cancelled) {
            Ok(Some(())) => {}
            Ok(None) => return Ok(false),
            Err(error) => {
                *pending_resize = Some(size);
                return Err(error);
            }
        }
    }
    *remote_writer = Some(writer);
    Ok(true)
}

#[cfg(test)]
fn resize_native_ssh_writer(
    remote_writer: &mut Option<Box<dyn SshShellWriter>>,
    pending_resize: &mut Option<rssh_core::TerminalSize>,
    size: rssh_core::TerminalSize,
) -> Result<bool, rssh_ssh::SshSessionError> {
    let cancellation = std::sync::atomic::AtomicBool::new(false);
    resize_native_ssh_writer_cancellable(remote_writer, pending_resize, size, &cancellation)
}

fn resize_native_ssh_writer_cancellable(
    remote_writer: &mut Option<Box<dyn SshShellWriter>>,
    pending_resize: &mut Option<rssh_core::TerminalSize>,
    size: rssh_core::TerminalSize,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<bool, rssh_ssh::SshSessionError> {
    if cancelled.load(Ordering::Acquire) {
        return Ok(false);
    }
    if let Some(writer) = remote_writer.as_mut() {
        Ok(writer.resize_cancellable(size.into(), cancelled)?.is_some())
    } else {
        *pending_resize = Some(size);
        Ok(true)
    }
}

fn cancel_native_ssh_writer(
    remote_writer: &mut Option<Box<dyn SshShellWriter>>,
    writer_cancellation: &std::sync::atomic::AtomicBool,
    connection_cancellation: &rssh_ssh::RusshConnectionCancellation,
) {
    writer_cancellation.store(true, Ordering::Release);
    connection_cancellation.cancel();
    if let Some(mut writer) = remote_writer.take() {
        let _ = writer.close();
    }
}

const fn connection_state_for_phase(phase: SshConnectionPhase) -> ConnectionState {
    match phase {
        SshConnectionPhase::Connecting
        | SshConnectionPhase::Authenticating
        | SshConnectionPhase::Opening => ConnectionState::Connecting,
        SshConnectionPhase::Connected => ConnectionState::Connected,
    }
}

fn report_native_ssh_connector_failure(
    event_proxy: &EventLoopProxy<WindowUserEvent>,
    command_sender: &mpsc::Sender<NativeSshCommand>,
    window_id: rssh_core::WindowId,
    pane_id: rssh_core::PaneId,
    runtime_generation: u64,
    message: &str,
) {
    let _ = event_proxy.send_event(WindowUserEvent::SshState {
        window_id,
        pane_id,
        runtime_generation,
        state: ConnectionState::Failed,
    });
    let _ = command_sender.send(NativeSshCommand::Cancel);
    eprintln!("{message}");
}

fn configure_native_ssh_host_key_verification(
    mut opener: RusshChannelOpener,
    policy: SshKnownHostsPolicy,
    known_hosts_path: Option<std::path::PathBuf>,
    prompt_proxy: EventLoopProxy<WindowUserEvent>,
    window_id: rssh_core::WindowId,
    pane_id: rssh_core::PaneId,
    runtime_generation: u64,
) -> RusshChannelOpener {
    if let Some(path) = known_hosts_path {
        opener = opener.with_known_hosts_path(path);
    }
    if policy == SshKnownHostsPolicy::Prompt {
        let verifier = HostKeyVerifier::new(move |challenge: HostKeyChallenge| {
            let (decision_sender, decision_receiver) = mpsc::sync_channel(1);
            let _ = prompt_proxy.send_event(WindowUserEvent::HostKeyPrompt {
                window_id,
                pane_id,
                runtime_generation,
                challenge,
                decision: decision_sender,
            });
            async move {
                decision_receiver
                    .recv_timeout(Duration::from_secs(120))
                    .unwrap_or(HostKeyDecision::Cancel)
            }
        });
        opener = opener.with_host_key_verifier_handle(verifier);
    }
    opener
}

impl NativeWindowApp {
    fn refresh_ssh_overlay(&mut self) {
        self.frame_needs_full_repaint = true;
        self.pending_frame_damage.clear();
        self.apply_window_title();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    pub(super) fn ssh_connection_state_for_pane(
        &self,
        pane_id: rssh_core::PaneId,
    ) -> ConnectionState {
        self.ssh_connection_states
            .get(&pane_id)
            .copied()
            .unwrap_or(ConnectionState::Pending)
    }

    fn mark_active_ssh_metrics_state(
        &mut self,
        pane_id: rssh_core::PaneId,
        state: ConnectionState,
    ) {
        if pane_id != self.app_shell.active_pane_id() {
            return;
        }
        if !matches!(
            state,
            ConnectionState::NotStarted | ConnectionState::Pending
        ) {
            self.metrics.mark_ssh_started();
        }
        self.metrics.mark_connection_state(state);
        if state == ConnectionState::Connected {
            self.metrics.mark_ssh_connected();
        }
    }

    pub(super) fn take_ssh_pane_auxiliary_state(
        &mut self,
        pane_id: rssh_core::PaneId,
    ) -> SshPaneAuxiliaryState {
        SshPaneAuxiliaryState {
            writer_sender: self.ssh_writer_senders.remove(&pane_id),
            writer_cancellation: self.ssh_writer_cancellations.remove(&pane_id),
            connection_cancellation: self.ssh_connection_cancellations.remove(&pane_id),
            connection_state: self.ssh_connection_states.remove(&pane_id),
            host_key_prompt: self.ssh_host_key_prompts.remove(&pane_id),
            secret_prompt: self.ssh_secret_prompts.remove(&pane_id),
        }
    }

    pub(super) fn install_ssh_pane_auxiliary_state(
        &mut self,
        pane_id: rssh_core::PaneId,
        auxiliary: SshPaneAuxiliaryState,
    ) {
        if let Some(sender) = auxiliary.writer_sender {
            self.ssh_writer_senders.insert(pane_id, sender);
        }
        if let Some(cancellation) = auxiliary.writer_cancellation {
            self.ssh_writer_cancellations
                .insert(pane_id, cancellation);
        }
        if let Some(cancellation) = auxiliary.connection_cancellation {
            self.ssh_connection_cancellations
                .insert(pane_id, cancellation);
        }
        if let Some(state) = auxiliary.connection_state {
            self.ssh_connection_states.insert(pane_id, state);
            self.mark_active_ssh_metrics_state(pane_id, state);
        }
        if let Some(prompt) = auxiliary.host_key_prompt {
            self.ssh_host_key_prompts.insert(pane_id, prompt);
        }
        if let Some(prompt) = auxiliary.secret_prompt {
            self.ssh_secret_prompts.insert(pane_id, prompt);
        }
        self.refresh_ssh_overlay();
    }

    fn ssh_pane_launch(&self, pane_id: rssh_core::PaneId) -> Option<&SshPaneLaunch> {
        self.app_shell
            .workspaces()
            .iter()
            .flat_map(rssh_core::app_shell::Workspace::tabs)
            .flat_map(rssh_core::app_shell::Tab::panes)
            .find(|pane| pane.id() == pane_id)
            .and_then(|pane| match pane.launch().domain() {
                PaneLaunchDomain::Ssh(launch) => Some(launch),
                PaneLaunchDomain::Local => None,
            })
    }

    fn ssh_connection_overlay_lines(&self, pane_id: rssh_core::PaneId) -> Vec<String> {
        let Some(launch) = self.ssh_pane_launch(pane_id) else {
            return Vec::new();
        };
        let state = self.ssh_connection_state_for_pane(pane_id);
        let mut lines = vec![format!(
            "SSH {} [{}]",
            launch.target(),
            connection_metric_name(state)
        )];

        if let Some(prompt) = self.ssh_secret_prompts.get(&pane_id) {
            let label = match prompt.prompt.kind {
                SecretPromptKind::Password => "Password",
                SecretPromptKind::PrivateKeyPassphrase => "Private-key passphrase",
            };
            let masked = "*".repeat(prompt.input.chars().count());
            lines.push(format!(
                "{label}: {masked} (masked) [Enter] submit [Esc] cancel"
            ));
        } else if let Some((challenge, _)) = self.ssh_host_key_prompts.get(&pane_id) {
            let known_hosts_path = challenge
                .known_hosts_path
                .as_ref()
                .map_or_else(|| "<default>".to_owned(), |path| path.display().to_string());
            match challenge.status {
                HostKeyStatus::Changed => {
                    lines.push(format!(
                        "BLOCKED: HOST KEY CHANGED {}:{}",
                        challenge.host, challenge.port
                    ));
                    lines.push(format!("{} {}", challenge.algorithm, challenge.fingerprint));
                    lines.push(format!("known_hosts: {known_hosts_path}"));
                    lines.push("[Esc] cancel".to_owned());
                }
                HostKeyStatus::Unknown | HostKeyStatus::Known => {
                    lines.push(format!(
                        "UNKNOWN HOST KEY {}:{}",
                        challenge.host, challenge.port
                    ));
                    lines.push(format!("{} {}", challenge.algorithm, challenge.fingerprint));
                    lines.push(format!("known_hosts: {known_hosts_path}"));
                    lines.push("[1] accept once  [2] accept and store  [Esc] cancel".to_owned());
                }
            }
        }
        lines
    }

    pub(super) fn ssh_connection_overlay_cells(
        &self,
        layout: &PaneRenderLayout,
    ) -> Vec<RenderCell> {
        let mut cells = Vec::new();
        for rect in &layout.panes {
            let lines = self.ssh_connection_overlay_lines(rect.pane_id);
            for (line_index, line) in lines.into_iter().take(usize::from(rect.rows)).enumerate() {
                let row = rect
                    .row
                    .saturating_add(u16::try_from(line_index).unwrap_or(u16::MAX));
                let row_start = cells.len();
                for column_offset in 0..rect.columns {
                    cells.push(ui_render_cell(
                        row,
                        rect.column.saturating_add(column_offset),
                        ' ',
                        DEFAULT_UI_SURFACE_FOREGROUND,
                        DEFAULT_UI_SURFACE_BACKGROUND,
                        true,
                    ));
                }
                for (column_offset, ch) in line.chars().take(usize::from(rect.columns)).enumerate()
                {
                    if let Some(cell) = cells.get_mut(row_start.saturating_add(column_offset)) {
                        cell.ch = ch;
                        cell.text = ch.to_string().into();
                    }
                }
            }
        }
        cells
    }

    /// Starts a native SSH channel without creating a local PTY.  The
    /// terminal runtime is deliberately the same one used by local panes;
    /// only the transport and lifecycle workers differ.  Secrets are
    /// represented by prompt auth descriptors and never enter this method's
    /// launch metadata, events, or terminal grid.
    #[allow(clippy::needless_pass_by_value, clippy::too_many_lines)]
    pub(super) fn spawn_native_ssh_runtime(
        &mut self,
        pane_id: rssh_core::PaneId,
        launch: &SshPaneLaunch,
        runtime_generation: u64,
        event_proxy: EventLoopProxy<WindowUserEvent>,
    ) -> Result<PaneRuntime, Box<dyn Error>> {
        let (pty_size, runtime) = self.prepare_pane_spawn_runtime(pane_id)?;
        let request_launch = launch.clone();
        let app_window_id = self.app_window_id;
        let ssh_runtime = self.runtime.composition().ssh_runtime_owner();
        let connected = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let writer_cancellation = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let connection_cancellation = rssh_ssh::RusshConnectionCancellation::new();
        let (command_sender, command_receiver) = native_ssh_command_channel();
        self.ssh_writer_senders
            .insert(pane_id, command_sender.clone());
        self.ssh_writer_cancellations
            .insert(pane_id, Arc::clone(&writer_cancellation));
        self.ssh_connection_cancellations
            .insert(pane_id, connection_cancellation.clone());
        let _ = event_proxy.send_event(WindowUserEvent::SshState {
            window_id: app_window_id,
            pane_id,
            runtime_generation,
            state: ConnectionState::Pending,
        });

        // Serialize input off the event thread; NativeSshWriter drops data while connection is pending.
        let writer_event_proxy = event_proxy.clone();
        let writer_connected = Arc::clone(&connected);
        let writer_operation_cancellation = Arc::clone(&writer_cancellation);
        let writer_connection_cancellation = connection_cancellation.clone();
        let writer_thread = thread::Builder::new()
            .name(format!("rssh-ssh-writer-{}", pane_id.get()))
            .spawn(move || {
                let mut remote_writer: Option<Box<dyn SshShellWriter>> = None;
                let mut pending_resize = None;
                while let Ok(command) = command_receiver.recv() {
                    if writer_operation_cancellation.load(Ordering::Acquire) {
                        break;
                    }
                    match command {
                        NativeSshCommand::Attach(writer) => {
                            match attach_native_ssh_writer_cancellable(
                                &mut remote_writer,
                                &mut pending_resize,
                                writer,
                                writer_operation_cancellation.as_ref(),
                            ) {
                                Ok(true) => {}
                                Ok(false) => break,
                                Err(error) => {
                                    writer_connected.store(false, Ordering::Release);
                                    let _ = writer_event_proxy.send_event(
                                        WindowUserEvent::WriteError {
                                            window_id: app_window_id,
                                            pane_id,
                                            runtime_generation,
                                            error: format!("SSH PTY resize failed: {error}"),
                                        },
                                    );
                                    break;
                                }
                            }
                        }
                        NativeSshCommand::Data(bytes) => {
                            let Some(writer) = remote_writer.as_mut() else {
                                continue;
                            };
                            let started = Instant::now();
                            match writer.write_cancellable(
                                &bytes,
                                writer_operation_cancellation.as_ref(),
                            ) {
                                Ok(Some(byte_count)) => {
                                    let _ = writer_event_proxy.send_event(
                                        WindowUserEvent::WriteCompleted {
                                            window_id: app_window_id,
                                            pane_id,
                                            runtime_generation,
                                            byte_count,
                                            elapsed: started.elapsed(),
                                        },
                                    );
                                }
                                Ok(None) => break,
                                Err(error) => {
                                    writer_connected.store(false, Ordering::Release);
                                    let _ = writer_event_proxy.send_event(
                                        WindowUserEvent::WriteError {
                                            window_id: app_window_id,
                                            pane_id,
                                            runtime_generation,
                                            error: error.to_string(),
                                        },
                                    );
                                    break;
                                }
                            }
                        }
                        NativeSshCommand::Resize(size) => {
                            match resize_native_ssh_writer_cancellable(
                                &mut remote_writer,
                                &mut pending_resize,
                                size,
                                writer_operation_cancellation.as_ref(),
                            ) {
                                Ok(true) => {}
                                Ok(false) => break,
                                Err(error) => {
                                    writer_connected.store(false, Ordering::Release);
                                    let _ = writer_event_proxy.send_event(
                                        WindowUserEvent::WriteError {
                                            window_id: app_window_id,
                                            pane_id,
                                            runtime_generation,
                                            error: format!("SSH PTY resize failed: {error}"),
                                        },
                                    );
                                    break;
                                }
                            }
                        }
                        NativeSshCommand::Cancel => {
                            writer_connected.store(false, Ordering::Release);
                            cancel_native_ssh_writer(
                                &mut remote_writer,
                                writer_operation_cancellation.as_ref(),
                                &writer_connection_cancellation,
                            );
                            break;
                        }
                    }
                }
                writer_connected.store(false, Ordering::Release);
                cancel_native_ssh_writer(
                    &mut remote_writer,
                    writer_operation_cancellation.as_ref(),
                    &writer_connection_cancellation,
                );
            })?;

        let connector_event_proxy = event_proxy.clone();
        let connector_connected = Arc::clone(&connected);
        let connector_command_sender = command_sender.clone();
        let connector_connection_cancellation = connection_cancellation.clone();
        let policy = launch.known_hosts_policy();
        let known_hosts_path = ssh_known_hosts_path();
        let connector_thread = thread::Builder::new()
            .name(format!("rssh-ssh-connector-{}", pane_id.get()))
            .spawn(move || {
                let request = match ssh_request_from_pane_launch(&request_launch, pty_size) {
                    Ok(request) => request,
                    Err(error) => {
                        report_native_ssh_connector_failure(
                            &connector_event_proxy,
                            &connector_command_sender,
                            app_window_id,
                            pane_id,
                            runtime_generation,
                            &format!("SSH target resolution failed: {error}"),
                        );
                        return;
                    }
                };
                let runtime_handle = match ssh_runtime.get_or_try_init() {
                    Ok(runtime_handle) => runtime_handle,
                    Err(error) => {
                        report_native_ssh_connector_failure(
                            &connector_event_proxy,
                            &connector_command_sender,
                            app_window_id,
                            pane_id,
                            runtime_generation,
                            &format!("SSH async runtime initialization failed: {error}"),
                        );
                        return;
                    }
                };
                let phase_proxy = connector_event_proxy.clone();
                let phase_reporter = move |phase: SshConnectionPhase| {
                    let state = connection_state_for_phase(phase);
                    let _ = phase_proxy.send_event(WindowUserEvent::SshState {
                        window_id: app_window_id,
                        pane_id,
                        runtime_generation,
                        state,
                    });
                };

                let opener = RusshChannelOpener::default()
                    .with_runtime_handle(runtime_handle)
                    .with_host_key_policy(russh_host_key_policy(policy))
                    .with_phase_reporter(phase_reporter)
                    .with_connection_cancellation(connector_connection_cancellation.clone());
                let mut opener = configure_native_ssh_host_key_verification(
                    opener,
                    policy,
                    known_hosts_path,
                    connector_event_proxy.clone(),
                    app_window_id,
                    pane_id,
                    runtime_generation,
                );

                let secret_proxy = connector_event_proxy.clone();
                let secret_provider = SecretProvider::new(move |prompt: SecretPrompt| {
                    let (response_sender, response_receiver) = mpsc::sync_channel(1);
                    let _ = secret_proxy.send_event(WindowUserEvent::SecretPrompt {
                        window_id: app_window_id,
                        pane_id,
                        runtime_generation,
                        prompt,
                        response: response_sender,
                    });
                    async move {
                        response_receiver
                            .recv_timeout(Duration::from_secs(120))
                            .ok()
                            .flatten()
                    }
                });
                opener = opener.with_secret_provider_handle(secret_provider);

                let mut connector = SshChannelConnector::new(opener);
                let session = match connector.connect(request) {
                    Ok(session) => session,
                    Err(error) => {
                        if connector_connection_cancellation.is_cancelled() {
                            return;
                        }
                        let _ = connector_event_proxy.send_event(WindowUserEvent::SshState {
                            window_id: app_window_id,
                            pane_id,
                            runtime_generation,
                            state: ConnectionState::Failed,
                        });
                        eprintln!("SSH connection failed: {error}");
                        return;
                    }
                };
                let (mut reader, writer) = session.into_read_writer();
                if connector_connection_cancellation.is_cancelled() {
                    return;
                }
                if connector_command_sender
                    .send(NativeSshCommand::Attach(writer))
                    .is_err()
                {
                    return;
                }
                connector_connected.store(true, Ordering::Release);
                let _ = connector_event_proxy.send_event(WindowUserEvent::SshState {
                    window_id: app_window_id,
                    pane_id,
                    runtime_generation,
                    state: ConnectionState::Connected,
                });

                let mut buffer = [0_u8; 8192];
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => {
                            connector_connected.store(false, Ordering::Release);
                            let _ = connector_event_proxy.send_event(WindowUserEvent::SshState {
                                window_id: app_window_id,
                                pane_id,
                                runtime_generation,
                                state: ConnectionState::Disconnected,
                            });
                            break;
                        }
                        Ok(count) => {
                            if connector_event_proxy
                                .send_event(WindowUserEvent::Output {
                                    window_id: app_window_id,
                                    pane_id,
                                    runtime_generation,
                                    bytes: buffer[..count].to_vec(),
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(error) => {
                            connector_connected.store(false, Ordering::Release);
                            let _ = connector_event_proxy.send_event(WindowUserEvent::SshState {
                                window_id: app_window_id,
                                pane_id,
                                runtime_generation,
                                state: ConnectionState::Failed,
                            });
                            eprintln!("SSH read failed: {error}");
                            break;
                        }
                    }
                }
                let _ = connector_command_sender.send(NativeSshCommand::Cancel);
            })?;

        let writer: Box<dyn Write + Send> = Box::new(NativeSshWriter {
            sender: command_sender,
            connected,
        });
        let snapshot = terminal_runtime_snapshot(&runtime, PaneStableViewport::default());
        Ok(PaneRuntime {
            runtime,
            transport: Some(PaneRuntimeTransportKind::NativeSsh),
            session: None,
            session_process_id: None,
            session_tty_name: None,
            writer: Some(writer),
            reader_thread: Some(connector_thread),
            writer_thread: Some(writer_thread),
            runtime_generation,
            snapshot,
            ui: PaneUiState::default(),
        })
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn handle_ssh_state(&mut self, pane_id: rssh_core::PaneId, state: ConnectionState) {
        if state == ConnectionState::Connected {
            if self
                .ssh_writer_cancellations
                .get(&pane_id)
                .is_some_and(|cancelled| cancelled.load(Ordering::Acquire))
            {
                return;
            }
            self.ssh_connection_cancellations.remove(&pane_id);
        } else if matches!(
            state,
            ConnectionState::Disconnected | ConnectionState::Failed
        ) {
            self.cancel_ssh_runtime(pane_id);
            self.ssh_writer_cancellations.remove(&pane_id);
        }
        self.ssh_connection_states.insert(pane_id, state);
        if matches!(
            state,
            ConnectionState::Disconnected | ConnectionState::Failed
        ) {
            self.resolve_host_key_prompt_for_pane(pane_id, HostKeyDecision::Cancel);
            self.resolve_secret_prompt_for_pane(pane_id, None);
        }
        self.mark_active_ssh_metrics_state(pane_id, state);
        self.refresh_ssh_overlay();
    }

    pub(super) fn handle_host_key_prompt(
        &mut self,
        pane_id: rssh_core::PaneId,
        challenge: HostKeyChallenge,
        decision: mpsc::SyncSender<HostKeyDecision>,
    ) {
        self.ssh_host_key_prompts
            .insert(pane_id, (challenge, decision));
        self.ssh_connection_states
            .insert(pane_id, ConnectionState::AwaitingHostKey);
        self.mark_active_ssh_metrics_state(pane_id, ConnectionState::AwaitingHostKey);
        self.refresh_ssh_overlay();
    }

    fn resolve_host_key_prompt(&mut self, decision: HostKeyDecision) {
        self.resolve_host_key_prompt_for_pane(self.app_shell.active_pane_id(), decision);
    }

    pub(super) fn resolve_host_key_prompt_for_pane(
        &mut self,
        pane_id: rssh_core::PaneId,
        decision: HostKeyDecision,
    ) {
        if let Some((_, sender)) = self.ssh_host_key_prompts.remove(&pane_id) {
            let _ = sender.send(decision);
            self.refresh_ssh_overlay();
        }
    }

    pub(super) fn handle_secret_prompt(
        &mut self,
        pane_id: rssh_core::PaneId,
        prompt: SecretPrompt,
        response: mpsc::SyncSender<Option<String>>,
    ) {
        self.ssh_secret_prompts.insert(
            pane_id,
            SshSecretPromptState {
                prompt,
                response,
                input: String::new(),
            },
        );
        self.ssh_connection_states
            .insert(pane_id, ConnectionState::AwaitingSecret);
        self.mark_active_ssh_metrics_state(pane_id, ConnectionState::AwaitingSecret);
        self.refresh_ssh_overlay();
    }

    fn resolve_secret_prompt(&mut self, value: Option<String>) {
        if let Some(mut prompt) = self
            .ssh_secret_prompts
            .remove(&self.app_shell.active_pane_id())
        {
            let _ = prompt.response.send(value);
            prompt.input.clear();
        }
        self.refresh_ssh_overlay();
    }

    pub(super) fn resolve_secret_prompt_for_pane(
        &mut self,
        pane_id: rssh_core::PaneId,
        value: Option<String>,
    ) {
        if let Some(mut prompt) = self.ssh_secret_prompts.remove(&pane_id) {
            let _ = prompt.response.send(value);
            prompt.input.clear();
            self.refresh_ssh_overlay();
        }
    }

    pub(super) fn retire_ssh_connection_state(&mut self, pane_id: rssh_core::PaneId) {
        self.resolve_host_key_prompt_for_pane(pane_id, HostKeyDecision::Cancel);
        self.resolve_secret_prompt_for_pane(pane_id, None);
        self.ssh_writer_cancellations.remove(&pane_id);
        self.ssh_connection_states.remove(&pane_id);
        self.refresh_ssh_overlay();
    }

    fn handle_secret_prompt_key(&mut self, logical_key: &Key, text: Option<&str>) -> bool {
        let pane_id = self.app_shell.active_pane_id();
        if !self.ssh_secret_prompts.contains_key(&pane_id) {
            return false;
        }
        match logical_key {
            Key::Named(NamedKey::Enter) => {
                let value = self
                    .ssh_secret_prompts
                    .get(&pane_id)
                    .map(|prompt| prompt.input.clone());
                self.resolve_secret_prompt(value);
                true
            }
            Key::Named(NamedKey::Escape) => {
                self.resolve_secret_prompt(None);
                true
            }
            Key::Named(NamedKey::Backspace) => {
                if let Some(prompt) = self.ssh_secret_prompts.get_mut(&pane_id) {
                    prompt.input.pop();
                }
                self.refresh_ssh_overlay();
                true
            }
            _ => {
                if let Some(text) = text
                    && let Some(prompt) = self.ssh_secret_prompts.get_mut(&pane_id)
                {
                    prompt
                        .input
                        .extend(text.chars().filter(|character| !character.is_control()));
                }
                self.refresh_ssh_overlay();
                true
            }
        }
    }

    pub(super) fn handle_ssh_prompt_key_event(
        &mut self,
        logical_key: &Key,
        text: Option<&str>,
    ) -> bool {
        if self
            .ssh_host_key_prompts
            .contains_key(&self.app_shell.active_pane_id())
        {
            let status = self
                .ssh_host_key_prompts
                .get(&self.app_shell.active_pane_id())
                .map(|(challenge, _)| challenge.status);
            let decision = match logical_key.as_ref() {
                Key::Character("1") if status == Some(HostKeyStatus::Unknown) => {
                    Some(HostKeyDecision::AcceptOnce)
                }
                Key::Character("2") if status == Some(HostKeyStatus::Unknown) => {
                    Some(HostKeyDecision::AcceptAndStore)
                }
                Key::Named(NamedKey::Escape) => Some(HostKeyDecision::Cancel),
                _ => None,
            };
            if let Some(decision) = decision {
                self.resolve_host_key_prompt(decision);
            }
            return true;
        }

        self.handle_secret_prompt_key(logical_key, text)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{self, Write},
        sync::{Arc, Mutex, mpsc},
    };

    use rssh_core::{
        TerminalSize,
        app_shell::{
            AppAction, PaneLaunch, SplitDirection, SshAuthDescription, SshKnownHostsPolicy,
            SshPaneLaunch,
        },
    };
    use rssh_ssh::{
        HostKeyChallenge, HostKeyDecision, HostKeyStatus, SecretPrompt, SshConnectionPhase,
        SshSessionError, SshShellWriter,
    };
    use winit::keyboard::{Key, NamedKey};

    use super::super::NativeWindowApp;
    use super::{
        attach_native_ssh_writer, cancel_native_ssh_writer, connection_state_for_phase,
        native_ssh_command_channel, resize_native_ssh_writer,
    };

    struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedBuffer {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct RecordingWriter {
        resizes: Arc<Mutex<Vec<TerminalSize>>>,
    }

    fn ssh_test_launch(target: &str) -> PaneLaunch {
        PaneLaunch::ssh(SshPaneLaunch::new(
            target,
            SshAuthDescription::PasswordPrompt,
            SshKnownHostsPolicy::Prompt,
        ))
    }

    fn snapshot_text(snapshot: &rterm_render_core::TerminalRenderSnapshot) -> String {
        let max_row = snapshot
            .cells()
            .iter()
            .map(|cell| cell.row)
            .max()
            .unwrap_or(0);
        let max_column = snapshot
            .cells()
            .iter()
            .map(|cell| cell.column)
            .max()
            .unwrap_or(0);
        (0..=max_row)
            .flat_map(|row| {
                (0..=max_column)
                    .map(move |column| {
                        snapshot
                            .cells()
                            .iter()
                            .find(|cell| cell.row == row && cell.column == column)
                            .map_or(' ', |cell| cell.ch)
                    })
                    .chain(std::iter::once('\n'))
            })
            .collect()
    }

    #[test]
    fn native_ssh_request_resolution_occurs_inside_the_connector_worker() {
        let source = include_str!("window_ssh_gui.rs");
        let body = source
            .split_once("pub(super) fn spawn_native_ssh_runtime")
            .expect("spawn function")
            .1
            .split_once("pub(super) fn handle_ssh_state")
            .expect("spawn function end")
            .0;
        let connector_worker = body
            .find(".name(format!(\"rssh-ssh-connector-")
            .expect("connector worker");
        let request_resolution = body
            .find("ssh_request_from_pane_launch")
            .expect("request resolution");

        assert!(
            request_resolution > connector_worker,
            "OpenSSH alias resolution must not execute on the event thread"
        );
    }

    #[test]
    fn native_ssh_runtime_initializes_inside_the_connector_worker() {
        let source = include_str!("window_ssh_gui.rs");
        let body = source
            .split_once("pub(super) fn spawn_native_ssh_runtime")
            .expect("spawn function")
            .1
            .split_once("pub(super) fn handle_ssh_state")
            .expect("spawn function end")
            .0;
        let connector_worker = body
            .find(".name(format!(\"rssh-ssh-connector-")
            .expect("connector worker");
        let initialization = body
            .find("ssh_runtime.get_or_try_init()")
            .expect("lazy runtime initialization");

        assert!(
            initialization > connector_worker,
            "Tokio runtime construction must stay off the event and pre-first-present threads"
        );
    }

    impl SshShellWriter for RecordingWriter {
        fn write(&mut self, bytes: &[u8]) -> Result<usize, SshSessionError> {
            Ok(bytes.len())
        }

        fn resize(&mut self, size: rssh_ssh::SshTerminalSize) -> Result<(), SshSessionError> {
            self.resizes.lock().unwrap().push(size.into());
            Ok(())
        }

        fn keepalive(&mut self) -> Result<(), SshSessionError> {
            Ok(())
        }

        fn finish_input(&mut self) -> Result<(), SshSessionError> {
            Ok(())
        }

        fn close(&mut self) -> Result<(), SshSessionError> {
            Ok(())
        }
    }

    #[test]
    fn native_ssh_writer_applies_only_latest_resize_queued_before_attach() {
        let first = TerminalSize::new(80, 24);
        let latest = TerminalSize::new(132, 43);
        let mut writer = None;
        let mut pending_resize = None;

        resize_native_ssh_writer(&mut writer, &mut pending_resize, first).unwrap();
        resize_native_ssh_writer(&mut writer, &mut pending_resize, latest).unwrap();

        let observed_resizes = Arc::new(Mutex::new(Vec::new()));
        attach_native_ssh_writer(
            &mut writer,
            &mut pending_resize,
            Box::new(RecordingWriter {
                resizes: Arc::clone(&observed_resizes),
            }),
        )
        .unwrap();

        assert_eq!(*observed_resizes.lock().unwrap(), vec![latest]);
        assert_eq!(pending_resize, None);
    }

    #[test]
    fn native_ssh_writer_cancel_stops_a_connection_attempt_before_attach() {
        let cancellation = rssh_ssh::RusshConnectionCancellation::new();
        let writer_cancellation = std::sync::atomic::AtomicBool::new(false);
        let mut writer = None;

        cancel_native_ssh_writer(&mut writer, &writer_cancellation, &cancellation);

        assert!(writer_cancellation.load(std::sync::atomic::Ordering::Acquire));
        assert!(cancellation.is_cancelled());
    }

    #[test]
    fn cancelling_a_local_pane_during_shutdown_does_not_request_an_ssh_repaint() {
        let mut app = NativeWindowApp::new(None);
        let pane_id = app.active_pane_id();
        app.frame_needs_full_repaint = false;

        app.cancel_ssh_runtime(pane_id);

        assert!(!app.frame_needs_full_repaint);
    }

    #[test]
    fn native_ssh_gui_command_queue_never_blocks_and_preserves_more_than_old_capacity() {
        let (sender, receiver) = native_ssh_command_channel();
        let connected = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let (done_sender, done_receiver) = mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            let mut writer = super::NativeSshWriter { sender, connected };
            for value in 0_u16..=255 {
                writer.write_all(&value.to_le_bytes()).unwrap();
            }
            done_sender.send(()).unwrap();
        });

        done_receiver
            .recv_timeout(std::time::Duration::from_millis(250))
            .expect("GUI input blocked after exceeding the old 64-command capacity");
        worker.join().unwrap();

        let payloads = receiver
            .try_iter()
            .map(|command| match command {
                super::NativeSshCommand::Data(bytes) => bytes,
                _ => panic!("unexpected non-data command"),
            })
            .collect::<Vec<_>>();
        assert_eq!(payloads.len(), 256);
        for (value, bytes) in payloads.into_iter().enumerate() {
            assert_eq!(bytes, u16::try_from(value).unwrap().to_le_bytes());
        }
    }

    #[test]
    fn native_ssh_gui_still_drops_terminal_input_before_connection() {
        let (sender, receiver) = native_ssh_command_channel();
        let mut writer = super::NativeSshWriter {
            sender,
            connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        assert_eq!(writer.write(b"must-not-be-cached").unwrap(), 18);
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn cancelling_ssh_runtime_sets_out_of_band_flags_before_queued_cancel() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        let pane_id = app.app_shell.active_pane_id();
        let writer_cancellation = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let connection_cancellation = rssh_ssh::RusshConnectionCancellation::new();
        let (sender, receiver) = native_ssh_command_channel();
        app.ssh_writer_senders.insert(pane_id, sender);
        app.ssh_writer_cancellations
            .insert(pane_id, Arc::clone(&writer_cancellation));
        app.ssh_connection_cancellations
            .insert(pane_id, connection_cancellation.clone());

        app.cancel_ssh_runtime(pane_id);

        assert!(writer_cancellation.load(std::sync::atomic::Ordering::Acquire));
        assert!(connection_cancellation.is_cancelled());
        assert!(matches!(
            receiver.try_recv().unwrap(),
            super::NativeSshCommand::Cancel
        ));
    }

    #[test]
    fn cancelled_ssh_generation_ignores_a_late_connected_event() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        let pane_id = app.app_shell.active_pane_id();
        let writer_cancellation = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let connection_cancellation = rssh_ssh::RusshConnectionCancellation::new();
        let (sender, _receiver) = native_ssh_command_channel();
        app.ssh_writer_senders.insert(pane_id, sender);
        app.ssh_writer_cancellations
            .insert(pane_id, Arc::clone(&writer_cancellation));
        app.ssh_connection_cancellations
            .insert(pane_id, connection_cancellation);
        app.handle_ssh_state(pane_id, super::ConnectionState::Connecting);

        app.cancel_ssh_runtime(pane_id);
        app.handle_ssh_state(pane_id, super::ConnectionState::Connected);

        assert_eq!(
            app.ssh_connection_state_for_pane(pane_id),
            super::ConnectionState::Connecting
        );
        assert!(writer_cancellation.load(std::sync::atomic::Ordering::Acquire));

        app.handle_ssh_state(pane_id, super::ConnectionState::Failed);
        assert!(!app.ssh_writer_cancellations.contains_key(&pane_id));
    }

    #[test]
    fn ssh_cancellation_handles_follow_auxiliary_state_across_window_remap() {
        let mut source = NativeWindowApp::new_with_visual_defaults(None);
        let source_pane = source.app_shell.active_pane_id();
        let target_pane = rssh_core::PaneId::new(source_pane.get().saturating_add(41));
        let writer_cancellation = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let connection_cancellation = rssh_ssh::RusshConnectionCancellation::new();
        let (sender, receiver) = native_ssh_command_channel();
        source.ssh_writer_senders.insert(source_pane, sender);
        source
            .ssh_writer_cancellations
            .insert(source_pane, Arc::clone(&writer_cancellation));
        source
            .ssh_connection_cancellations
            .insert(source_pane, connection_cancellation.clone());

        let auxiliary = source.take_ssh_pane_auxiliary_state(source_pane);
        let mut target = NativeWindowApp::new_with_visual_defaults(None);
        target.install_ssh_pane_auxiliary_state(target_pane, auxiliary);
        target.cancel_ssh_runtime(target_pane);

        assert!(!source.ssh_writer_senders.contains_key(&source_pane));
        assert!(!source.ssh_writer_cancellations.contains_key(&source_pane));
        assert!(
            !source
                .ssh_connection_cancellations
                .contains_key(&source_pane)
        );
        assert!(writer_cancellation.load(std::sync::atomic::Ordering::Acquire));
        assert!(connection_cancellation.is_cancelled());
        assert!(matches!(
            receiver.try_recv().unwrap(),
            super::NativeSshCommand::Cancel
        ));
    }

    #[test]
    fn ssh_state_cleanup_keeps_established_writer_cancellation_generation_safe() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        let pane_id = app.app_shell.active_pane_id();
        let writer_cancellation = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let connection_cancellation = rssh_ssh::RusshConnectionCancellation::new();
        let (sender, _receiver) = native_ssh_command_channel();
        app.ssh_writer_senders.insert(pane_id, sender);
        app.ssh_writer_cancellations
            .insert(pane_id, Arc::clone(&writer_cancellation));
        app.ssh_connection_cancellations
            .insert(pane_id, connection_cancellation);

        app.handle_ssh_state(pane_id, super::ConnectionState::Connected);
        assert!(app.ssh_writer_cancellations.contains_key(&pane_id));
        assert!(!app.ssh_connection_cancellations.contains_key(&pane_id));
        assert!(!writer_cancellation.load(std::sync::atomic::Ordering::Acquire));

        app.handle_ssh_state(pane_id, super::ConnectionState::Disconnected);
        assert!(!app.ssh_writer_cancellations.contains_key(&pane_id));
        assert!(writer_cancellation.load(std::sync::atomic::Ordering::Acquire));
    }

    #[test]
    fn ssh_authenticating_phase_stays_connecting_until_a_secret_prompt_arrives() {
        assert_eq!(
            connection_state_for_phase(SshConnectionPhase::Authenticating),
            super::ConnectionState::Connecting
        );

        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        let pane_id = app.app_shell.active_pane_id();
        app.handle_ssh_state(
            pane_id,
            connection_state_for_phase(SshConnectionPhase::Authenticating),
        );
        assert_eq!(
            app.ssh_connection_state_for_pane(pane_id),
            super::ConnectionState::Connecting
        );

        let (response_sender, _response_receiver) = mpsc::sync_channel(1);
        app.handle_secret_prompt(pane_id, SecretPrompt::password("ops"), response_sender);
        assert_eq!(
            app.ssh_connection_state_for_pane(pane_id),
            super::ConnectionState::AwaitingSecret
        );
    }

    #[test]
    fn awaiting_secret_state_remains_final_after_the_ssh_timer_starts() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.set_initial_pane_launch(ssh_test_launch("metrics-secret.example"));
        let pane_id = app.app_shell.active_pane_id();

        app.handle_ssh_state(pane_id, super::ConnectionState::AwaitingSecret);

        assert_eq!(
            app.ssh_connection_state_for_pane(pane_id),
            super::ConnectionState::AwaitingSecret
        );
        assert_eq!(
            app.metrics.connection_state(),
            super::ConnectionState::AwaitingSecret
        );
    }

    #[test]
    fn ssh_initial_frame_renders_target_and_pending_state_in_the_shared_snapshot() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.set_initial_pane_launch(ssh_test_launch("ops@pending.example:2222"));

        let rendered = snapshot_text(&app.render_snapshot());

        assert!(rendered.contains("SSH ops@pending.example:2222 [pending]"));
        assert_eq!(
            app.metrics.connection_state(),
            super::ConnectionState::Pending
        );
    }

    #[test]
    fn ssh_connection_state_and_title_follow_the_active_pane_without_overwriting_metrics() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.set_initial_pane_launch(ssh_test_launch("first.example"));
        let first = app.app_shell.active_pane_id();
        app.app_shell
            .apply_action(AppAction::SplitPane {
                pane: first,
                direction: SplitDirection::Right,
                launch: Some(ssh_test_launch("second.example")),
            })
            .unwrap();
        let second = app.app_shell.active_pane_id();
        assert_ne!(first, second);

        let metrics_before_inactive_event = app.metrics.connection_state();
        app.handle_ssh_state(first, super::ConnectionState::Failed);
        assert_eq!(
            app.metrics.connection_state(),
            metrics_before_inactive_event,
            "an inactive SSH pane must not overwrite startup metrics"
        );
        app.handle_ssh_state(second, super::ConnectionState::Connected);
        assert_eq!(
            app.metrics.connection_state(),
            super::ConnectionState::Connected
        );
        assert!(app.effective_window_title().contains("second.example"));
        assert!(app.effective_window_title().contains("[connected]"));

        app.app_shell
            .apply_action(AppAction::ActivatePane { pane: first })
            .unwrap();
        assert!(app.effective_window_title().contains("first.example"));
        assert!(app.effective_window_title().contains("[failed]"));

        let rendered = snapshot_text(&app.render_snapshot());
        assert!(rendered.contains("SSH first.example [failed]"));
        assert!(rendered.contains("SSH second.example [connected]"));
    }

    #[test]
    fn inactive_ssh_prompts_do_not_overwrite_startup_metrics() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.set_initial_pane_launch(ssh_test_launch("inactive-prompts.example"));
        let inactive = app.app_shell.active_pane_id();
        app.app_shell
            .apply_action(AppAction::SplitPane {
                pane: inactive,
                direction: SplitDirection::Right,
                launch: Some(ssh_test_launch("active.example")),
            })
            .unwrap();
        let metrics_before = app.metrics.connection_state();

        let (decision_sender, decision_receiver) = mpsc::sync_channel(1);
        app.handle_host_key_prompt(
            inactive,
            HostKeyChallenge::new(
                "inactive-prompts.example",
                22,
                "ssh-ed25519",
                "SHA256:inactive",
                HostKeyStatus::Unknown,
            ),
            decision_sender,
        );
        assert_eq!(app.metrics.connection_state(), metrics_before);
        app.resolve_host_key_prompt_for_pane(inactive, HostKeyDecision::Cancel);
        assert_eq!(decision_receiver.recv().unwrap(), HostKeyDecision::Cancel);

        let (response_sender, response_receiver) = mpsc::sync_channel(1);
        app.handle_secret_prompt(inactive, SecretPrompt::password("ops"), response_sender);
        assert_eq!(app.metrics.connection_state(), metrics_before);
        app.resolve_secret_prompt_for_pane(inactive, None);
        assert_eq!(response_receiver.recv().unwrap(), None);
    }

    #[test]
    fn ssh_overlay_redraws_for_every_connection_state() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.set_initial_pane_launch(ssh_test_launch("states.example"));
        let pane_id = app.app_shell.active_pane_id();

        for (state, name) in [
            (super::ConnectionState::Pending, "pending"),
            (super::ConnectionState::Connecting, "connecting"),
            (super::ConnectionState::AwaitingSecret, "awaiting_secret"),
            (super::ConnectionState::AwaitingHostKey, "awaiting_host_key"),
            (super::ConnectionState::Connected, "connected"),
            (super::ConnectionState::Disconnected, "disconnected"),
            (super::ConnectionState::Failed, "failed"),
        ] {
            app.frame_needs_full_repaint = false;
            app.handle_ssh_state(pane_id, state);
            assert!(app.frame_needs_full_repaint, "state {name} needs redraw");
            assert!(
                snapshot_text(&app.render_snapshot())
                    .contains(&format!("SSH states.example [{name}]")),
                "state {name} must be visible"
            );
        }
    }

    #[test]
    fn unknown_host_key_overlay_shows_fingerprint_path_and_all_safe_decisions() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.set_initial_pane_launch(ssh_test_launch("unknown.example"));
        let pane_id = app.app_shell.active_pane_id();
        let (decision_sender, decision_receiver) = mpsc::sync_channel(1);
        app.handle_host_key_prompt(
            pane_id,
            HostKeyChallenge::new(
                "unknown.example",
                2222,
                "ssh-ed25519",
                "SHA256:unknown-fingerprint",
                HostKeyStatus::Unknown,
            )
            .with_known_hosts_path("C:/Users/test/.ssh/known_hosts"),
            decision_sender,
        );

        let rendered = snapshot_text(&app.render_snapshot());
        assert!(rendered.contains("UNKNOWN HOST KEY unknown.example:2222"));
        assert!(rendered.contains("ssh-ed25519 SHA256:unknown-fingerprint"));
        assert!(rendered.contains("C:/Users/test/.ssh/known_hosts"));
        assert!(rendered.contains("[1] accept once"));
        assert!(rendered.contains("[2] accept and store"));
        assert!(rendered.contains("[Esc] cancel"));

        assert!(app.handle_ssh_prompt_key_event(&Key::Character("1".into()), None));
        assert_eq!(
            decision_receiver.recv().unwrap(),
            HostKeyDecision::AcceptOnce
        );
    }

    #[test]
    fn changed_host_key_is_visibly_blocked_and_only_escape_can_cancel() {
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.set_initial_pane_launch(ssh_test_launch("changed.example"));
        let pane_id = app.app_shell.active_pane_id();
        let (decision_sender, decision_receiver) = mpsc::sync_channel(1);
        app.handle_host_key_prompt(
            pane_id,
            HostKeyChallenge::new(
                "changed.example",
                22,
                "ssh-ed25519",
                "SHA256:changed-fingerprint",
                HostKeyStatus::Changed,
            )
            .with_known_hosts_path("C:/Users/test/.ssh/known_hosts"),
            decision_sender,
        );

        let rendered = snapshot_text(&app.render_snapshot());
        let title = app.effective_window_title();
        for surface in [&rendered, &title] {
            assert!(surface.contains("BLOCKED"));
            assert!(surface.contains("SHA256:changed-fingerprint"));
            assert!(surface.contains("C:/Users/test/.ssh/known_hosts"));
            assert!(!surface.contains("[1]"));
            assert!(!surface.contains("[2]"));
            assert!(surface.contains("[Esc]"));
        }

        assert!(app.handle_ssh_prompt_key_event(&Key::Character("1".into()), None));
        assert!(decision_receiver.try_recv().is_err());
        assert!(app.ssh_host_key_prompts.contains_key(&pane_id));

        assert!(app.handle_ssh_prompt_key_event(&Key::Named(NamedKey::Escape), None));
        assert_eq!(decision_receiver.recv().unwrap(), HostKeyDecision::Cancel);
        assert!(!app.ssh_host_key_prompts.contains_key(&pane_id));
    }

    fn assert_secret_prompt_is_masked_and_isolated(prompt: SecretPrompt, secret: &str) {
        let session_log = Arc::new(Mutex::new(Vec::new()));
        let mut app =
            NativeWindowApp::new_with_session_log(None, SharedBuffer(Arc::clone(&session_log)));
        app.set_initial_pane_launch(ssh_test_launch("secret.example"));
        let pane_id = app.app_shell.active_pane_id();
        let snapshot_before = app.snapshot.clone();
        let grid_before = format!("{:?}", app.runtime.terminal().grid());
        let (response_sender, response_receiver) = mpsc::sync_channel(1);

        app.handle_secret_prompt(pane_id, prompt, response_sender);
        let masked_title = app.effective_window_title();
        assert!(masked_title.contains("(masked)"));
        assert!(!masked_title.contains(secret));

        app.frame_needs_full_repaint = false;
        assert!(app.handle_ssh_prompt_key_event(&Key::Character("input".into()), Some(secret),));
        assert!(app.frame_needs_full_repaint, "masked input needs redraw");

        assert_eq!(app.effective_window_title(), masked_title);
        assert_eq!(app.snapshot, snapshot_before);
        assert_eq!(format!("{:?}", app.runtime.terminal().grid()), grid_before);
        assert!(
            response_receiver.try_recv().is_err(),
            "prompt input must not be forwarded before explicit submission"
        );

        let rendered_snapshot = app.render_snapshot();
        let rendered_text = snapshot_text(&rendered_snapshot);
        assert!(rendered_text.contains("(masked)"));
        assert!(rendered_text.contains(&"*".repeat(secret.chars().count())));
        let visible_snapshot = format!("{rendered_snapshot:?}");
        let startup_metrics_stdout = app.metrics_json_report().unwrap();
        let logged = session_log.lock().unwrap().clone();
        for (surface, contents) in [
            ("window title", masked_title.as_bytes()),
            ("visible snapshot", visible_snapshot.as_bytes()),
            (
                "startup metrics JSON/stdout",
                startup_metrics_stdout.as_bytes(),
            ),
            ("session log", logged.as_slice()),
        ] {
            assert!(
                !contents
                    .windows(secret.len())
                    .any(|bytes| bytes == secret.as_bytes()),
                "SSH secret leaked into {surface}"
            );
        }

        assert!(app.handle_ssh_prompt_key_event(&Key::Named(NamedKey::Enter), None));
        assert_eq!(
            response_receiver.recv().unwrap(),
            Some(secret.to_owned()),
            "the secret may leave the GUI only through the dedicated prompt response"
        );
        assert!(!app.ssh_secret_prompts.contains_key(&pane_id));
        assert!(!snapshot_text(&app.render_snapshot()).contains(secret));
    }

    #[test]
    fn ssh_password_prompt_never_enters_terminal_or_observable_gui_surfaces() {
        assert_secret_prompt_is_masked_and_isolated(
            SecretPrompt::password("ops"),
            "password-surface-leak-sentinel",
        );
    }

    #[test]
    fn ssh_private_key_passphrase_prompt_never_enters_terminal_or_observable_gui_surfaces() {
        assert_secret_prompt_is_masked_and_isolated(
            SecretPrompt::private_key_passphrase("ops"),
            "passphrase-surface-leak-sentinel",
        );
    }

    #[test]
    fn cancelling_a_secret_prompt_drops_its_input_without_leaking_it() {
        let sentinel = "cancelled-secret-surface-leak-sentinel";
        let session_log = Arc::new(Mutex::new(Vec::new()));
        let mut app =
            NativeWindowApp::new_with_session_log(None, SharedBuffer(Arc::clone(&session_log)));
        app.set_initial_pane_launch(ssh_test_launch("cancel-secret.example"));
        let pane_id = app.app_shell.active_pane_id();
        let (response_sender, response_receiver) = mpsc::sync_channel(1);
        app.handle_secret_prompt(pane_id, SecretPrompt::password("ops"), response_sender);
        assert!(app.handle_ssh_prompt_key_event(&Key::Character("input".into()), Some(sentinel),));

        assert!(app.handle_ssh_prompt_key_event(&Key::Named(NamedKey::Escape), None));
        assert_eq!(response_receiver.recv().unwrap(), None);
        assert!(!app.ssh_secret_prompts.contains_key(&pane_id));

        let observable = format!(
            "{}\n{:?}\n{}\n{:?}",
            app.effective_window_title(),
            app.render_snapshot(),
            app.metrics_json_report().unwrap(),
            session_log.lock().unwrap().as_slice(),
        );
        assert!(!observable.contains(sentinel));
    }

    #[test]
    fn retiring_an_ssh_pane_clears_its_state_and_prompt_buffer() {
        let sentinel = "retired-pane-secret-leak-sentinel";
        let mut app = NativeWindowApp::new_with_visual_defaults(None);
        app.set_initial_pane_launch(ssh_test_launch("retired.example"));
        let pane_id = app.app_shell.active_pane_id();
        let (response_sender, response_receiver) = mpsc::sync_channel(1);
        app.handle_secret_prompt(pane_id, SecretPrompt::password("ops"), response_sender);
        assert!(app.handle_ssh_prompt_key_event(&Key::Character("input".into()), Some(sentinel),));

        app.retire_ssh_connection_state(pane_id);

        assert_eq!(response_receiver.recv().unwrap(), None);
        assert!(!app.ssh_connection_states.contains_key(&pane_id));
        assert!(!app.ssh_secret_prompts.contains_key(&pane_id));
        assert!(
            !format!(
                "{}\n{:?}\n{}",
                app.effective_window_title(),
                app.render_snapshot(),
                app.metrics_json_report().unwrap(),
            )
            .contains(sentinel)
        );
    }
}
