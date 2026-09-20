#[derive(Debug, Clone, PartialEq, Eq)]
struct ProductGuiProbe {
    run_id: String,
    scenario: DiagnosticScenario,
    hold_duration: Duration,
}

#[cfg(feature = "ssh")]
fn install_product_gui_probe(
    app: &mut NativeWindowApp,
    options: &SshOptions,
    process_started_at: Instant,
) -> io::Result<Option<DiagnosticMarkerHandle>> {
    let probe = match std::env::var(PRODUCT_GUI_PROBE_ENV) {
        Ok(value) => Some(
            parse_product_gui_probe(&value)
                .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?,
        ),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{PRODUCT_GUI_PROBE_ENV} must contain UTF-8 JSON"),
            ));
        }
    };
    let Some(probe) = probe else {
        return Ok(None);
    };
    if options.benchmark_startup {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the product GUI residence probe cannot use --benchmark-startup",
        ));
    }

    let markers =
        DiagnosticMarkerHandle::new(probe.run_id, probe.scenario, process_started_at);
    if !markers.emit(DiagnosticMarkerKind::ProcessStarted, None, None)? {
        return Err(io::Error::other(
            "product probe process_started marker was not unique",
        ));
    }
    let pending_secret = if probe.scenario == DiagnosticScenario::Ssh1 {
        Some(std::env::var("RSSH_DIAGNOSTIC_SSH_SECRET").map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "SSH1 product GUI probe requires its isolated secret channel",
            )
        })?)
    } else {
        None
    };
    app.set_product_gui_probe(
        markers.clone(),
        probe.scenario,
        probe.hold_duration,
        pending_secret,
    );
    Ok(Some(markers))
}

fn parse_product_gui_probe(value: &str) -> Result<ProductGuiProbe, String> {
    let value: serde_json::Value = serde_json::from_str(value)
        .map_err(|error| format!("invalid {PRODUCT_GUI_PROBE_ENV} JSON: {error}"))?;
    let fields = value
        .as_object()
        .ok_or_else(|| format!("{PRODUCT_GUI_PROBE_ENV} must be a JSON object"))?;
    let expected = ["schema", "run_id", "scenario", "hold_ms"];
    if fields.len() != expected.len() || !expected.iter().all(|field| fields.contains_key(*field))
    {
        return Err(format!(
            "{PRODUCT_GUI_PROBE_ENV} must contain only schema, run_id, scenario, and hold_ms"
        ));
    }
    if fields.get("schema").and_then(serde_json::Value::as_str)
        != Some(PRODUCT_GUI_PROBE_SCHEMA)
    {
        return Err(format!("{PRODUCT_GUI_PROBE_ENV} schema is unsupported"));
    }
    let run_id = fields
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("{PRODUCT_GUI_PROBE_ENV} run_id must be a string"))?;
    let mut run_id_characters = run_id.chars();
    if run_id.len() > 128
        || !run_id_characters
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric())
        || !run_id_characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
    {
        return Err(format!(
            "{PRODUCT_GUI_PROBE_ENV} run_id must be a stable ASCII identifier"
        ));
    }
    let scenario = match fields
        .get("scenario")
        .and_then(serde_json::Value::as_str)
    {
        Some("empty-window") => DiagnosticScenario::EmptyWindow,
        Some("ssh1") => DiagnosticScenario::Ssh1,
        _ => {
            return Err(format!(
                "{PRODUCT_GUI_PROBE_ENV} scenario must be empty-window or ssh1"
            ));
        }
    };
    let hold_ms = fields
        .get("hold_ms")
        .and_then(serde_json::Value::as_u64)
        .filter(|hold_ms| (5_000..=300_000).contains(hold_ms))
        .ok_or_else(|| format!("{PRODUCT_GUI_PROBE_ENV} hold_ms must be from 5000 to 300000"))?;
    Ok(ProductGuiProbe {
        run_id: run_id.to_owned(),
        scenario,
        hold_duration: Duration::from_millis(hold_ms),
    })
}

/// Runs a private GUI scenario for the cross-platform diagnostics launcher.
/// The empty-window scenario deliberately never starts a PTY or SSH transport.
pub fn run_diagnostic_gui(
    options: &DiagnosticGuiOptions,
    process_started_at: Instant,
) -> Result<(), Box<dyn Error>> {
    let markers = DiagnosticMarkerHandle::new(
        options.run_id.clone(),
        options.scenario,
        process_started_at,
    );
    markers.emit(DiagnosticMarkerKind::ProcessStarted, None, None)?;

    let mut app = NativeWindowApp::new_with_workspace_class_position_and_osc52_policy(
        None,
        Osc52Policy::Off,
        PtyCommand::default_shell(),
        None,
        None,
        None,
    );
    let pending_secret = if options.scenario == DiagnosticScenario::Ssh1 {
        let host = options.ssh_host.ok_or("ssh1 diagnostic is missing --ssh-host")?;
        let port = options.ssh_port.ok_or("ssh1 diagnostic is missing --ssh-port")?;
        let user = options
            .ssh_user
            .as_deref()
            .ok_or("ssh1 diagnostic is missing --ssh-user")?;
        let authority = match host {
            std::net::IpAddr::V4(address) => address.to_string(),
            std::net::IpAddr::V6(address) => format!("[{address}]"),
        };
        app.set_initial_pane_launch(PaneLaunch::ssh(SshPaneLaunch::new(
            format!("{user}@{authority}:{port}"),
            SshAuthDescription::PasswordPrompt,
            SshKnownHostsPolicy::AcceptUnknown,
        )));
        Some(std::env::var("RSSH_DIAGNOSTIC_SSH_SECRET").map_err(|_| {
            "ssh1 diagnostic requires RSSH_DIAGNOSTIC_SSH_SECRET on its isolated environment channel"
        })?)
    } else {
        None
    };
    configure_diagnostic_gui_initial_size(&mut app, options.columns, options.rows);
    app.metrics.startup_trace = StartupTrace::from_process_started_at(process_started_at);
    app.set_renderer_mode(options.renderer);
    app.set_diagnostic_gpu_backend(options.gpu_backend);
    app.set_diagnostic_gui(
        markers.clone(),
        options.scenario,
        Duration::from_millis(options.hold_ms),
        pending_secret,
        options.font_mode,
        options.font_specimen,
    );
    if let Some(path) = &options.log {
        app.session_log = Some(Box::new(File::create(path)?) as Box<dyn Write + Send>);
    }

    let event_loop = EventLoop::<WindowUserEvent>::with_user_event().build()?;
    let event_proxy = event_loop.create_proxy();
    app.event_proxy = Some(event_proxy.clone());
    spawn_diagnostic_stdin_shutdown_listener(event_proxy)?;
    let cli = validate_cli_config_overrides(&[])?;
    let lifecycle = Box::new(NativeConfigLifecycle::new(
        ConfigDiscoveryInputs::capture_current_process(),
        false,
        None,
        cli,
    ));
    let mut manager = NativeWindowManager::new(app)
        .with_config_lifecycle(lifecycle)
        .with_deferred_config();
    event_loop.run_app(&mut manager)?;
    manager.shutdown_runtime_owners();
    manager.reap_retired_apps();
    markers.emit(DiagnosticMarkerKind::ProcessExited, None, None)?;
    Ok(())
}

fn configure_diagnostic_gui_initial_size(app: &mut NativeWindowApp, columns: u16, rows: u16) {
    let size = TerminalSize::new(columns, rows);
    app.initial_cols = columns;
    app.initial_rows = rows;
    *app.runtime = TerminalRuntime::new(size);
    app.snapshot = terminal_runtime_snapshot(&app.runtime, PaneStableViewport::default());
    let frame_size = app.initial_frame_size();
    app.frame_width = frame_size.width;
    app.frame_height = frame_size.height;
    app.window_frame.set_size(frame_size);
}

fn spawn_diagnostic_stdin_shutdown_listener(
    event_proxy: EventLoopProxy<WindowUserEvent>,
) -> io::Result<()> {
    thread::Builder::new()
        .name("rssh-diagnostic-stdin".to_owned())
        .spawn(move || {
            let mut line = String::new();
            if io::stdin().read_line(&mut line).is_ok() && !line.is_empty() {
                let _ = event_proxy.send_event(WindowUserEvent::DiagnosticShutdownRequested);
            }
        })
        .map(drop)
}


#[cfg(feature = "diagnostic-tools")]
pub(crate) fn run_attribution_diagnostic_gui(
    options: &DiagnosticGuiOptions,
    process_started_at: Instant,
) -> Result<(), Box<dyn Error>> {
    let stage = options
        .attribution_stage
        .ok_or_else(|| io::Error::other("attribution diagnostic is missing its typed stage"))?;
    let markers = DiagnosticMarkerHandle::new(
        options.run_id.clone(),
        options.scenario,
        process_started_at,
    );
    if !markers.emit(DiagnosticMarkerKind::ProcessStarted, None, None)? {
        return Err(io::Error::other("process_started marker was not unique").into());
    }
    // The stdin listener sends at most one terminal decision.
    let (shutdown_sender, shutdown_receiver) = mpsc::sync_channel(1);
    thread::Builder::new()
        .name("rssh-attribution-stdin".to_owned())
        .spawn(move || {
            let mut line = String::new();
            match io::stdin().read_line(&mut line) {
                Ok(0) => {}
                Ok(_) if line.trim() == "shutdown" => {
                    let _ = shutdown_sender.send(Ok(()));
                }
                Ok(_) => {
                    let _ = shutdown_sender.send(Err(
                        "attribution diagnostic stdin accepted only 'shutdown'".to_owned(),
                    ));
                }
                Err(error) => {
                    let _ = shutdown_sender.send(Err(format!(
                        "read attribution diagnostic stdin: {error}"
                    )));
                }
            }
        })?;

    let owner_result = crate::window_gpu::run_stage7_native_attribution(
        stage,
        options.gpu_backend,
        Duration::from_millis(options.hold_ms).max(Duration::from_secs(5)),
        markers.clone(),
        shutdown_receiver,
    );
    owner_result?;
    let exited = markers.emit(
        DiagnosticMarkerKind::ProcessExited,
        None,
        Some(DiagnosticConnectionState::NotStarted),
    );
    if !exited? {
        return Err(io::Error::other("process_exited marker was not unique").into());
    }
    Ok(())
}

fn gpu_ready_extra(metrics: &GpuPresentationMetrics) -> HashMap<String, serde_json::Value> {
    HashMap::from([
        (
            "gpu_backend".to_owned(),
            serde_json::json!(metrics.backend),
        ),
        (
            "gpu_adapter_name".to_owned(),
            serde_json::json!(metrics.adapter_name),
        ),
        (
            "gpu_adapter_vendor_id".to_owned(),
            serde_json::json!(metrics.adapter_vendor_id),
        ),
        (
            "gpu_adapter_device_id".to_owned(),
            serde_json::json!(metrics.adapter_device_id),
        ),
        (
            "gpu_adapter_type".to_owned(),
            serde_json::json!(metrics.adapter_type),
        ),
    ])
}

const FONT_PROOF_GPU_READY_FOLLOWUPS: [DiagnosticMarkerKind; 2] = [
    DiagnosticMarkerKind::FontOwnershipReady,
    DiagnosticMarkerKind::ScenarioReady,
];

const fn diagnostic_first_present_is_scenario_ready(
    font_mode: Option<rssh_diagnostics::DiagnosticFontMode>,
    scenario: DiagnosticScenario,
    scenario_ready_requires_gpu: bool,
) -> bool {
    !scenario_ready_requires_gpu
        && font_mode.is_none()
        && matches!(scenario, DiagnosticScenario::EmptyWindow)
}

const fn diagnostic_ssh1_present_is_scenario_ready(
    scenario: DiagnosticScenario,
    scenario_ready_requires_gpu: bool,
    connection_state: ConnectionState,
    renderer: DiagnosticRendererKind,
) -> bool {
    matches!(scenario, DiagnosticScenario::Ssh1)
        && matches!(connection_state, ConnectionState::Connected)
        && (!scenario_ready_requires_gpu || matches!(renderer, DiagnosticRendererKind::Gpu))
}

fn validate_font_resource_marker_value(value: &serde_json::Value) -> Result<(), String> {
    match value {
        serde_json::Value::Object(fields) => {
            for (key, value) in fields {
                let normalized = key.to_ascii_lowercase();
                if normalized.contains("path") || normalized.starts_with("env_") {
                    return Err(format!(
                        "font resource marker contains forbidden raw host key '{key}'"
                    ));
                }
                validate_font_resource_marker_value(value)?;
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                validate_font_resource_marker_value(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn gpu_ready_extra_with_font_resources(
    gpu: &WindowGpu,
) -> Result<HashMap<String, serde_json::Value>, String> {
    let mut extra = gpu_ready_extra(gpu.metrics());
    if let Some(resources) = gpu.diagnostic_font_resources() {
        let value = serde_json::to_value(resources)
            .map_err(|error| format!("serialize font resource summary: {error}"))?;
        validate_font_resource_marker_value(&value)?;
        extra.insert("font_resources".to_owned(), value);
    }
    Ok(extra)
}

impl NativeWindowApp {
    fn set_diagnostic_gpu_backend(
        &mut self,
        backend: Option<rssh_diagnostics::DiagnosticGpuBackend>,
    ) {
        self.startup.diagnostic_gpu_backend = backend;
    }

    fn is_benchmark_startup(&self) -> bool {
        matches!(self.startup.mode, NativeStartupMode::Benchmark)
    }

    fn diagnostic_gui(&self) -> Option<&NativeDiagnosticGuiState> {
        match &self.startup.mode {
            NativeStartupMode::Diagnostic(diagnostic) => Some(diagnostic),
            NativeStartupMode::Normal | NativeStartupMode::Benchmark => None,
        }
    }

    fn diagnostic_gui_mut(&mut self) -> Option<&mut NativeDiagnosticGuiState> {
        match &mut self.startup.mode {
            NativeStartupMode::Diagnostic(diagnostic) => Some(diagnostic),
            NativeStartupMode::Normal | NativeStartupMode::Benchmark => None,
        }
    }

    fn has_diagnostic_gui(&self) -> bool {
        self.diagnostic_gui().is_some()
    }

    fn suppresses_transport_start(&self) -> bool {
        self.diagnostic_gui()
            .is_some_and(|diagnostic| diagnostic.scenario == DiagnosticScenario::EmptyWindow)
    }

    fn set_diagnostic_gui(
        &mut self,
        markers: DiagnosticMarkerHandle,
        scenario: DiagnosticScenario,
        hold_duration: Duration,
        pending_secret: Option<String>,
        font_mode: Option<rssh_diagnostics::DiagnosticFontMode>,
        font_specimen: Option<rssh_diagnostics::DiagnosticFontSpecimen>,
    ) {
        self.set_diagnostic_gui_state(
            markers,
            scenario,
            hold_duration,
            pending_secret,
            font_mode,
            font_specimen,
            false,
        );
    }

    fn set_product_gui_probe(
        &mut self,
        markers: DiagnosticMarkerHandle,
        scenario: DiagnosticScenario,
        hold_duration: Duration,
        pending_secret: Option<String>,
    ) {
        self.set_diagnostic_gui_state(
            markers,
            scenario,
            hold_duration,
            pending_secret,
            None,
            None,
            true,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn set_diagnostic_gui_state(
        &mut self,
        markers: DiagnosticMarkerHandle,
        scenario: DiagnosticScenario,
        hold_duration: Duration,
        pending_secret: Option<String>,
        font_mode: Option<rssh_diagnostics::DiagnosticFontMode>,
        font_specimen: Option<rssh_diagnostics::DiagnosticFontSpecimen>,
        scenario_ready_requires_gpu: bool,
    ) {
        self.startup.mode = NativeStartupMode::Diagnostic(NativeDiagnosticGuiState {
            markers,
            scenario,
            hold_duration,
            hold_deadline: None,
            absolute_deadline: Instant::now()
                + Duration::from_secs(10).saturating_add(hold_duration),
            pending_secret,
            secret_prompt_presented: false,
            font_mode,
            font_specimen,
            scenario_ready_requires_gpu,
        });
    }

    fn diagnostic_font_options(
        &self,
    ) -> (
        Option<rssh_diagnostics::DiagnosticFontMode>,
        Option<rssh_diagnostics::DiagnosticFontSpecimen>,
    ) {
        self.diagnostic_gui()
            .map_or((None, None), |diagnostic| {
                (diagnostic.font_mode, diagnostic.font_specimen)
            })
    }

    fn diagnostic_scale_override_enabled(&self) -> bool {
        self.is_benchmark_startup() || self.has_diagnostic_gui()
    }

    fn with_diagnostic_font_specimen(
        &self,
        snapshot: TerminalRenderSnapshot,
    ) -> TerminalRenderSnapshot {
        let Some(specimen) = self
            .diagnostic_gui()
            .and_then(|diagnostic| diagnostic.font_specimen)
        else {
            return snapshot;
        };
        let size = self.runtime.terminal().grid().size();
        let snapshot = TerminalRenderSnapshot::from_grid(&rssh_terminal::TerminalGrid::new(size));
        let row = self.terminal_frame_row_offset();
        let mut column = 0_u16;
        let mut cells = Vec::new();
        for grapheme in crate::window_gpu::diagnostic_font_specimen_text(specimen).graphemes(true) {
            let columns = UnicodeWidthStr::width(grapheme).max(1);
            let mut leader = RenderCell::new(row, column, grapheme);
            leader.columns = u8::try_from(columns).unwrap_or(u8::MAX);
            cells.push(leader);
            for continuation_offset in 1..columns {
                let mut continuation = RenderCell::new(
                    row,
                    column.saturating_add(u16::try_from(continuation_offset).unwrap_or(u16::MAX)),
                    "",
                );
                continuation.columns = 0;
                continuation.continuation = true;
                cells.push(continuation);
            }
            column = column.saturating_add(u16::try_from(columns).unwrap_or(u16::MAX));
        }
        snapshot.with_overlay_cells(cells)
    }

    fn diagnostic_hold_deadline(&self) -> Option<Instant> {
        self.diagnostic_gui().map(|diagnostic| {
            diagnostic
                .hold_deadline
                .unwrap_or(diagnostic.absolute_deadline)
                .min(diagnostic.absolute_deadline)
        })
    }

    fn emit_diagnostic_marker(
        &self,
        kind: DiagnosticMarkerKind,
        renderer: Option<DiagnosticRendererKind>,
        connection_state: Option<DiagnosticConnectionState>,
    ) {
        if let Some(diagnostic) = self.diagnostic_gui()
            && let Err(error) = diagnostic.markers.emit(kind, renderer, connection_state)
        {
            eprintln!("failed to emit diagnostic marker {kind:?}: {error}");
        }
    }

    fn mark_diagnostic_window_created(&self) {
        self.emit_diagnostic_marker(DiagnosticMarkerKind::WindowCreated, None, None);
    }

    fn mark_diagnostic_config_ready(&self) {
        self.emit_diagnostic_marker(DiagnosticMarkerKind::ConfigReady, None, None);
    }

    fn mark_diagnostic_transport_started(&self) {
        if self
            .diagnostic_gui()
            .is_some_and(|diagnostic| diagnostic.scenario == DiagnosticScenario::Ssh1)
        {
            self.emit_diagnostic_marker(
                DiagnosticMarkerKind::TransportStarted,
                None,
                Some(DiagnosticConnectionState::Pending),
            );
        }
    }

    fn mark_transport_start_requested(&mut self) {
        self.transport_start_requested = true;
        self.mark_diagnostic_transport_started();
    }

    fn advance_ssh1_diagnostic_after_present(
        &mut self,
        renderer: DiagnosticRendererKind,
        snapshot: &TerminalRenderSnapshot,
    ) {
        if !self
            .diagnostic_gui()
            .is_some_and(|diagnostic| diagnostic.scenario == DiagnosticScenario::Ssh1)
        {
            return;
        }
        let pane_id = self.app_shell.active_pane_id();
        let state = self.ssh_connection_state_for_pane(pane_id);
        if state == ConnectionState::AwaitingSecret {
            let secret = self.diagnostic_gui_mut().and_then(|diagnostic| {
                diagnostic.secret_prompt_presented = true;
                diagnostic.pending_secret.take()
            });
            if secret.is_some() {
                self.resolve_secret_prompt_for_pane(pane_id, secret);
            }
            return;
        }
        let scenario_ready_requires_gpu = self
            .diagnostic_gui()
            .is_some_and(|diagnostic| diagnostic.scenario_ready_requires_gpu);
        if !diagnostic_ssh1_present_is_scenario_ready(
            DiagnosticScenario::Ssh1,
            scenario_ready_requires_gpu,
            state,
            renderer,
        ) {
            return;
        }

        let visible_cell_count = visible_snapshot_cell_count(snapshot);
        let mut extra = HashMap::new();
        extra.insert(
            "visible_connection_state".to_owned(),
            serde_json::json!("connected"),
        );
        extra.insert(
            "visible_cell_count".to_owned(),
            serde_json::json!(visible_cell_count),
        );
        let Some(diagnostic) = self.diagnostic_gui_mut() else {
            return;
        };
        extra.insert(
            "secret_prompt_presented".to_owned(),
            serde_json::json!(diagnostic.secret_prompt_presented),
        );
        if let Err(error) = diagnostic.markers.emit(
            DiagnosticMarkerKind::TransportReady,
            Some(renderer),
            Some(DiagnosticConnectionState::Connected),
        ) {
            eprintln!("failed to emit diagnostic transport readiness: {error}");
        }
        if diagnostic.hold_deadline.is_none() {
            if let Err(error) = diagnostic.markers.emit_with_extra(
                DiagnosticMarkerKind::ScenarioReady,
                Some(renderer),
                Some(DiagnosticConnectionState::Connected),
                extra,
            ) {
                eprintln!("failed to emit diagnostic scenario readiness: {error}");
            }
            diagnostic.hold_deadline = Some(Instant::now() + diagnostic.hold_duration);
        }
    }

    fn mark_diagnostic_first_present(
        &mut self,
        renderer: DiagnosticRendererKind,
        visible_cell_count: usize,
    ) {
        if let Some(diagnostic) = self.diagnostic_gui() {
            let mut extra = HashMap::new();
            extra.insert(
                "visible_cell_count".to_owned(),
                serde_json::json!(visible_cell_count),
            );
            if let Err(error) = diagnostic.markers.emit_with_extra(
                DiagnosticMarkerKind::FirstPresent,
                Some(renderer),
                Some(diagnostic_connection_state(
                    self.ssh_connection_state_for_pane(self.app_shell.active_pane_id()),
                )),
                extra,
            ) {
                eprintln!("failed to emit diagnostic first present: {error}");
            }
        }
        if renderer == DiagnosticRendererKind::Gpu {
            self.mark_diagnostic_gpu_ready();
        }
        if let Some(diagnostic) = self.diagnostic_gui_mut()
            && diagnostic_first_present_is_scenario_ready(
                diagnostic.font_mode,
                diagnostic.scenario,
                diagnostic.scenario_ready_requires_gpu,
            )
            && diagnostic.hold_deadline.is_none()
        {
            if let Err(error) = diagnostic.markers.emit(
                DiagnosticMarkerKind::ScenarioReady,
                Some(renderer),
                Some(DiagnosticConnectionState::NotStarted),
            ) {
                eprintln!("failed to emit diagnostic scenario readiness: {error}");
            }
            diagnostic.hold_deadline = Some(Instant::now() + diagnostic.hold_duration);
        }
    }

    fn mark_diagnostic_gpu_ready(&mut self) {
        let Some((
            markers,
            scenario,
            font_mode,
            hold_deadline,
            hold_duration,
            scenario_ready_requires_gpu,
        )) = self.diagnostic_gui().map(|diagnostic| {
            (
                diagnostic.markers.clone(),
                diagnostic.scenario,
                diagnostic.font_mode,
                diagnostic.hold_deadline,
                diagnostic.hold_duration,
                diagnostic.scenario_ready_requires_gpu,
            )
        })
        else {
            return;
        };
        let extra = match self.gpu_owners.active.as_ref() {
            Some(gpu) => match gpu_ready_extra_with_font_resources(gpu) {
                Ok(extra) => extra,
                Err(error) => {
                    eprintln!("failed to build diagnostic GPU resource marker: {error}");
                    return;
                }
            },
            None => HashMap::new(),
        };
        if let Err(error) = markers.emit_with_extra(
            DiagnosticMarkerKind::GpuReady,
            Some(DiagnosticRendererKind::Gpu),
            None,
            extra,
        ) {
            eprintln!("failed to emit diagnostic GPU readiness: {error}");
        }
        if scenario != DiagnosticScenario::EmptyWindow || hold_deadline.is_some() {
            return;
        }
        let followups: &[DiagnosticMarkerKind] = if font_mode.is_some() {
            &FONT_PROOF_GPU_READY_FOLLOWUPS
        } else if scenario_ready_requires_gpu {
            &[DiagnosticMarkerKind::ScenarioReady]
        } else {
            return;
        };
        for &kind in followups {
            if let Err(error) = markers.emit(
                kind,
                Some(DiagnosticRendererKind::Gpu),
                Some(DiagnosticConnectionState::NotStarted),
            ) {
                eprintln!("failed to emit diagnostic font proof readiness: {error}");
            }
        }
        if let Some(diagnostic) = self.diagnostic_gui_mut() {
            diagnostic.hold_deadline = Some(Instant::now() + hold_duration);
        }
    }

    fn record_first_present(
        &mut self,
        renderer: RendererKind,
        snapshot: &TerminalRenderSnapshot,
    ) {
        let diagnostic_renderer = match renderer {
            RendererKind::Cpu => {
                self.metrics.record_first_present(RendererKind::Cpu);
                DiagnosticRendererKind::Cpu
            }
            RendererKind::Gpu => {
                self.metrics.record_first_present(RendererKind::Gpu);
                DiagnosticRendererKind::Gpu
            }
        };
        self.mark_diagnostic_first_present(
            diagnostic_renderer,
            visible_snapshot_cell_count(snapshot),
        );
        self.metrics
            .record_first_frame_private_bytes(current_process_private_bytes());
    }

    fn frame_limit_redraw_pending(&self) -> bool {
        if test_ssh_gui_frame_limit().is_some()
            && self.presentation_owner == PresentationOwner::GpuInitializing
            && self.gpu_owners.active.is_none()
        {
            return false;
        }
        self.frame_limit.is_some_and(|limit| {
            let target = if self.metrics.pty_linkage_enabled
                && !self.metrics.terminal_linkage_nonce_found
            {
                limit.saturating_sub(1)
            } else {
                limit
            };
            self.rendered_frames < target
        })
    }

    fn frame_limit_refresh_pending(&self) -> bool {
        self.frame_limit_redraw_pending() || self.final_linkage_frame_is_reserved()
    }

    fn final_linkage_frame_is_reserved(&self) -> bool {
        self.metrics.pty_linkage_enabled
            && !self.metrics.terminal_linkage_nonce_found
            && self
                .frame_limit
                .is_some_and(|limit| self.rendered_frames.saturating_add(1) >= limit)
    }

    fn frame_limit_reached(&self) -> bool {
        self.frame_limit
            .is_some_and(|limit| self.rendered_frames >= limit)
    }

    fn frame_limit_probe_ready(&self) -> bool {
        self.frame_limit_reached()
            && (!self.metrics.pty_linkage_enabled
                || self.metrics.terminal_linkage_nonce_found)
    }

    fn frame_limit_probe_pending(&self) -> bool {
        self.frame_limit.is_some() && !self.frame_limit_probe_ready()
    }

    fn frame_limit_redraw_deadline(&self, now: Instant) -> Option<Instant> {
        self.frame_limit_refresh_pending().then(|| {
            self.last_redraw_request_at
                .map_or(now, |last| last + self.redraw_request_interval())
        })
    }
}

#[cfg(test)]
mod font_mode_tests {
    use super::*;
    use rssh_diagnostics::{DiagnosticFontMode, DiagnosticFontSpecimen};

    #[test]
    fn diagnostic_font_mode_is_stored_as_typed_app_state() {
        let mut app = NativeWindowApp::new(None);
        app.set_diagnostic_gui(
            DiagnosticMarkerHandle::new(
                "font-proof".to_owned(),
                DiagnosticScenario::EmptyWindow,
                Instant::now(),
            ),
            DiagnosticScenario::EmptyWindow,
            Duration::from_millis(250),
            None,
            Some(DiagnosticFontMode::Lazy),
            Some(DiagnosticFontSpecimen::Emoji),
        );

        assert_eq!(
            app.diagnostic_font_options(),
            (
                Some(DiagnosticFontMode::Lazy),
                Some(DiagnosticFontSpecimen::Emoji)
            )
        );
    }

    #[test]
    fn diagnostic_font_marker_rejects_raw_path_keys_recursively() {
        let unsafe_value = serde_json::json!({
            "retained_source_bytes": 1,
            "nested": {"source_path": r"C:\\Windows\\Fonts\\private.ttf"}
        });
        assert!(validate_font_resource_marker_value(&unsafe_value).is_err());

        let safe_value = serde_json::json!({
            "retained_source_bytes": 1,
            "catalog_fingerprint_sha256": "a".repeat(64),
            "nested": [{"generation": 2}]
        });
        validate_font_resource_marker_value(&safe_value).expect("safe resource summary");
    }

    #[test]
    fn diagnostic_font_specimen_is_injected_into_the_gpu_snapshot() {
        for (specimen, expected) in [
            (DiagnosticFontSpecimen::Ascii, "R-SSH Stage 7"),
            (DiagnosticFontSpecimen::Cjk, "中文"),
            (DiagnosticFontSpecimen::Emoji, "😀"),
        ] {
            let mut app = NativeWindowApp::new(None);
            app.set_diagnostic_gui(
                DiagnosticMarkerHandle::new(
                    "font-proof-snapshot".to_owned(),
                    DiagnosticScenario::EmptyWindow,
                    Instant::now(),
                ),
                DiagnosticScenario::EmptyWindow,
                Duration::from_millis(250),
                None,
                Some(DiagnosticFontMode::Lazy),
                Some(specimen),
            );

            let production_snapshot = app.render_snapshot().with_overlay_cells([
                RenderCell::new(0, 40, "PRODUCTION UI MUST NOT REACH FONT PROOF"),
            ]);
            let snapshot = app.with_diagnostic_font_specimen(production_snapshot);
            let rendered = snapshot
                .cells()
                .iter()
                .filter(|cell| !cell.continuation)
                .map(|cell| cell.text.as_ref())
                .collect::<String>();
            assert!(
                rendered.contains(expected),
                "{specimen:?} specimen must reach the actual GPU snapshot: {rendered:?}"
            );
            assert!(
                !rendered.contains("PRODUCTION UI"),
                "font proof must use a controlled specimen-only snapshot: {rendered:?}"
            );
        }
    }

    #[test]
    fn font_proof_gpu_readiness_followups_are_owner_then_scenario() {
        assert_eq!(
            FONT_PROOF_GPU_READY_FOLLOWUPS,
            [
                DiagnosticMarkerKind::FontOwnershipReady,
                DiagnosticMarkerKind::ScenarioReady,
            ]
        );
        assert!(!diagnostic_first_present_is_scenario_ready(
            Some(DiagnosticFontMode::CurrentCopied),
            DiagnosticScenario::EmptyWindow,
            false,
        ));
        assert!(diagnostic_first_present_is_scenario_ready(
            None,
            DiagnosticScenario::EmptyWindow,
            false,
        ));
        assert!(!diagnostic_first_present_is_scenario_ready(
            None,
            DiagnosticScenario::EmptyWindow,
            true,
        ));
    }

    #[test]
    fn product_gui_probe_requires_connected_gpu_frame_for_ssh1() {
        assert!(!diagnostic_ssh1_present_is_scenario_ready(
            DiagnosticScenario::Ssh1,
            true,
            ConnectionState::Connected,
            DiagnosticRendererKind::Cpu,
        ));
        assert!(diagnostic_ssh1_present_is_scenario_ready(
            DiagnosticScenario::Ssh1,
            true,
            ConnectionState::Connected,
            DiagnosticRendererKind::Gpu,
        ));
        assert!(!diagnostic_ssh1_present_is_scenario_ready(
            DiagnosticScenario::Ssh1,
            true,
            ConnectionState::Connecting,
            DiagnosticRendererKind::Gpu,
        ));
        assert!(diagnostic_ssh1_present_is_scenario_ready(
            DiagnosticScenario::Ssh1,
            false,
            ConnectionState::Connected,
            DiagnosticRendererKind::Cpu,
        ));
    }
}
