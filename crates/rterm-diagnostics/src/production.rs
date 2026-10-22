use std::{
    collections::VecDeque,
    io::{BufRead, BufReader, Read, Write},
    net::SocketAddr,
    path::Path,
    process::{Child, ChildStdout, Command, ExitStatus, Stdio},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rterm_test_support::ssh::HermeticSshServer;

#[cfg(target_os = "windows")]
use crate::WindowsPrivateWorkingSetSampler;

use crate::{
    CollectedMarkers, ConnectionState, ConnectionSummary, DiagnosticFailure,
    DiagnosticFontResourceSummary, DiagnosticsResult, LauncherOptions, LauncherPhase,
    LauncherStateMachine, MarkerCollector, MarkerDisposition, MarkerIdentity, MarkerKind,
    MemoryMetric, MemorySample, MemorySampler, MemoryStatistics, MemorySummary, Platform,
    ProcessExitKind, ProcessSummary, Readiness, ReadinessStatus, RendererSummary, RunIdentity,
    SamplerError, Scenario, SchemaVersion, StartupMilestones, summarize_bytes,
};

const READINESS_TIMEOUT: Duration = Duration::from_secs(20);
const PIPE_POLL_INTERVAL: Duration = Duration::from_millis(10);
const OUTPUT_TAIL_LIMIT: usize = 64 * 1024;
const FIXTURE_USER: &str = "rterm-diagnostics";
const PRODUCT_GUI_PROBE_ENV: &str = "RTERM_STAGE7_PRODUCT_GUI_PROBE";

#[derive(Debug)]
pub struct LauncherExecution {
    pub result: DiagnosticsResult,
    pub success: bool,
}

/// Runs one fully owned GUI diagnostic child and returns its schema-v2 result.
///
/// Runtime failures are represented inside the returned JSON model so the launcher
/// binary can always emit one machine-readable object after argument validation.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "process, pipe, sampler, fixture, and cleanup ownership are kept in one auditable lifetime"
)]
pub fn execute_launcher(options: &LauncherOptions) -> LauncherExecution {
    let started = Instant::now();
    let run_id = unique_run_id(options.scenario);
    let run = run_identity(options, &run_id);
    let metric = native_metric();
    let fixture = match SshFixtureContext::start(options.scenario) {
        Ok(fixture) => fixture,
        Err(message) => {
            return failed_execution(
                run,
                options,
                metric,
                0,
                RunFailure::new("fixture_start_failed", "launch", message),
                ProcessExitKind::Natural,
                None,
            );
        }
    };
    let mut command = match diagnostic_command(options, &run_id, fixture.as_ref()) {
        Ok(command) => command,
        Err(message) => {
            let _ = stop_fixture(fixture, None);
            return failed_execution(
                run,
                options,
                metric,
                0,
                RunFailure::new("command_build_failed", "launch", message),
                ProcessExitKind::Natural,
                None,
            );
        }
    };
    let secret = fixture.as_ref().map(|fixture| fixture.secret.clone());
    let evidence = match StreamEvidence::from_environment(secret.clone()) {
        Ok(evidence) => evidence,
        Err(error) => {
            let _ = stop_fixture(fixture, None);
            return failed_execution(
                run,
                options,
                metric,
                0,
                RunFailure::from_io("evidence_setup_failed", "launch", error),
                ProcessExitKind::Natural,
                None,
            );
        }
    };
    let mut child = match command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_fixture(fixture, evidence.root.as_deref());
            return failed_execution(
                run,
                options,
                metric,
                0,
                RunFailure::from_io("child_spawn_failed", "launch", error),
                ProcessExitKind::Natural,
                None,
            );
        }
    };
    let pid = child.id();
    let stdout_tail = BoundedTail::new(OUTPUT_TAIL_LIMIT).with_capture(evidence.path("stdout.txt"));
    let stderr_tail = BoundedTail::new(OUTPUT_TAIL_LIMIT).with_capture(evidence.path("stderr.txt"));
    let (line_sender, line_receiver) = mpsc::channel();
    let (Some(child_stdout), Some(child_stderr)) = (child.stdout.take(), child.stderr.take())
    else {
        let status = force_reap(&mut child);
        let _ = stop_fixture(fixture, evidence.root.as_deref());
        return failed_execution(
            run,
            options,
            metric,
            pid,
            RunFailure::new(
                "pipe_setup_failed",
                "launch",
                "configured child pipes were not available",
            ),
            ProcessExitKind::Forced,
            status.as_ref().and_then(ExitStatus::code),
        );
    };
    let stdout_thread = spawn_stdout_drain(child_stdout, stdout_tail.clone(), line_sender);
    let stderr_thread = spawn_stderr_drain(child_stderr, stderr_tail.clone());

    let mut child_memory_source = match native_sampler(pid) {
        Ok(child_memory_source) => child_memory_source,
        Err(error) => {
            let status = force_reap(&mut child);
            join_pipe_threads(stdout_thread, stderr_thread);
            let _ = stop_fixture(fixture, evidence.root.as_deref());
            return failed_execution(
                run,
                options,
                metric,
                pid,
                failure_with_tails(
                    RunFailure::new("sampler_init_failed", "launch", error.to_string()),
                    &stdout_tail,
                    &stderr_tail,
                    secret.as_deref(),
                ),
                ProcessExitKind::Forced,
                status.as_ref().and_then(ExitStatus::code),
            );
        }
    };

    let marker_identity = MarkerIdentity::new(run_id, pid, options.scenario);
    let mut collector = match options.attribution_stage {
        Some(stage) => MarkerCollector::new_attribution(marker_identity, stage),
        None => MarkerCollector::new(marker_identity),
    };
    let mut font_resources = None;
    let mut state = LauncherStateMachine::new(options.configuration());
    let _ = state.child_started(pid);
    let mut protocol = ChildProtocol {
        lines: &line_receiver,
        collector: &mut collector,
        state: &mut state,
        font_resources: &mut font_resources,
    };
    let run_outcome = run_child(
        options,
        started,
        &mut child,
        &mut protocol,
        child_memory_source.as_mut(),
    );

    let (memory_samples, exit_kind, exit_status, teardown_ms) = match run_outcome {
        Ok(outcome) => outcome,
        Err(failure) => {
            let cleanup_started = Instant::now();
            let status = force_reap(&mut child);
            join_pipe_threads(stdout_thread, stderr_thread);
            let _ = drain_late_markers(&line_receiver, &mut collector);
            let fixture_failure = stop_fixture(fixture, evidence.root.as_deref()).err();
            let mut failure =
                failure_with_tails(failure, &stdout_tail, &stderr_tail, secret.as_deref());
            if let Some(fixture_failure) = fixture_failure {
                failure.message =
                    format!("{}; fixture teardown: {fixture_failure}", failure.message);
            }
            return failed_execution_with_trace(
                run,
                options,
                metric,
                pid,
                failure,
                ProcessExitKind::Forced,
                status.as_ref().and_then(ExitStatus::code),
                Some(duration_millis(cleanup_started.elapsed())),
                collector.trace().clone(),
            );
        }
    };

    join_pipe_threads(stdout_thread, stderr_thread);
    if let Err(mut failure) = drain_late_markers(&line_receiver, &mut collector) {
        if let Err(fixture_failure) = stop_fixture(fixture, evidence.root.as_deref()) {
            failure.message = format!("{}; fixture teardown: {fixture_failure}", failure.message);
        }
        return failed_execution_with_trace(
            run,
            options,
            metric,
            pid,
            failure_with_tails(failure, &stdout_tail, &stderr_tail, secret.as_deref()),
            exit_kind,
            exit_status.code(),
            Some(teardown_ms),
            collector.trace().clone(),
        );
    }
    if let Err(error) = stop_fixture(fixture, evidence.root.as_deref()) {
        return failed_execution_with_trace(
            run,
            options,
            metric,
            pid,
            failure_with_tails(
                RunFailure::new("fixture_teardown_failed", "reap", error),
                &stdout_tail,
                &stderr_tail,
                secret.as_deref(),
            ),
            exit_kind,
            exit_status.code(),
            Some(teardown_ms),
            collector.trace().clone(),
        );
    }

    match successful_result(
        run,
        options,
        metric,
        pid,
        memory_samples,
        exit_kind,
        exit_status.code(),
        teardown_ms,
        collector.trace().clone(),
        font_resources,
    ) {
        Ok(result) => LauncherExecution {
            result,
            success: true,
        },
        Err(failure) => failed_execution_with_trace(
            run_identity(options, "result-build-failure"),
            options,
            metric,
            pid,
            failure,
            exit_kind,
            exit_status.code(),
            Some(teardown_ms),
            collector.trace().clone(),
        ),
    }
}

struct ChildProtocol<'a> {
    lines: &'a mpsc::Receiver<String>,
    collector: &'a mut MarkerCollector,
    state: &'a mut LauncherStateMachine,
    font_resources: &'a mut Option<DiagnosticFontResourceSummary>,
}

#[expect(
    clippy::too_many_lines,
    reason = "the state-machine phases remain explicit and linear for deadline auditing"
)]
fn run_child(
    options: &LauncherOptions,
    started: Instant,
    child: &mut Child,
    protocol: &mut ChildProtocol<'_>,
    sampler: &mut dyn MemorySampler,
) -> Result<(Vec<MemorySample>, ProcessExitKind, ExitStatus, u64), RunFailure> {
    let readiness_deadline = Instant::now() + READINESS_TIMEOUT;
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| RunFailure::from_io("child_poll_failed", "await_markers", error))?
        {
            return Err(RunFailure::new(
                "child_exited_early",
                "await_scenario_ready",
                format!("child exited before readiness with {:?}", status.code()),
            ));
        }
        if Instant::now() >= readiness_deadline {
            return Err(RunFailure::new(
                "readiness_timeout",
                "await_scenario_ready",
                format!("scenario readiness exceeded {READINESS_TIMEOUT:?}"),
            ));
        }
        match protocol.lines.recv_timeout(PIPE_POLL_INTERVAL) {
            Ok(line) => {
                let ready = process_marker_line(
                    &line,
                    started,
                    protocol.collector,
                    Some(protocol.state),
                    protocol.font_resources,
                )?;
                if ready {
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => thread::yield_now(),
        }
    }

    let stabilization_deadline = protocol.state.next_deadline_ms().ok_or_else(|| {
        RunFailure::new("missing_deadline", "stabilize", "no stabilization deadline")
    })?;
    wait_until_elapsed(
        stabilization_deadline,
        started,
        child,
        protocol.lines,
        protocol.collector,
        protocol.state,
        protocol.font_resources,
    )?;
    protocol
        .state
        .advance_to(elapsed_ms(started))
        .map_err(state_failure)?;

    let sampling_started_ms = elapsed_ms(started);
    let mut memory_samples = Vec::with_capacity(usize::try_from(options.sample_count).unwrap_or(0));
    for sequence in 0..options.sample_count {
        let due = protocol.state.next_deadline_ms().ok_or_else(|| {
            RunFailure::new("missing_deadline", "sample", "sample deadline is missing")
        })?;
        wait_until_elapsed(
            due,
            started,
            child,
            protocol.lines,
            protocol.collector,
            protocol.state,
            protocol.font_resources,
        )?;
        let observed_ms = elapsed_ms(started);
        let bytes = sampler.sample().map_err(|error| {
            RunFailure::new("memory_sample_failed", "sample", error.to_string())
        })?;
        protocol
            .state
            .record_sample(observed_ms, bytes)
            .map_err(state_failure)?;
        memory_samples.push(MemorySample {
            sequence,
            elapsed_ms: observed_ms,
            bytes,
        });
    }
    let sampling_finished_ms = elapsed_ms(started);

    protocol
        .state
        .graceful_shutdown_requested(sampling_finished_ms)
        .map_err(state_failure)?;
    let teardown_started = Instant::now();
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(b"shutdown\n")
            .and_then(|()| stdin.flush())
            .map_err(|error| {
                RunFailure::from_io("shutdown_signal_failed", "request_shutdown", error)
            })?;
    }
    child.stdin.take();

    let shutdown_deadline = Instant::now() + options.shutdown_timeout;
    let (exit_kind, status) = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| RunFailure::from_io("child_poll_failed", "reap", error))?
        {
            break (ProcessExitKind::Requested, status);
        }
        drain_available_markers(
            protocol.lines,
            started,
            protocol.collector,
            Some(protocol.state),
            protocol.font_resources,
        )?;
        if Instant::now() >= shutdown_deadline {
            protocol
                .state
                .force_shutdown(elapsed_ms(started))
                .map_err(state_failure)?;
            child
                .kill()
                .map_err(|error| RunFailure::from_io("child_kill_failed", "reap", error))?;
            let status = child
                .wait()
                .map_err(|error| RunFailure::from_io("child_wait_failed", "reap", error))?;
            break (ProcessExitKind::Forced, status);
        }
        thread::sleep(PIPE_POLL_INTERVAL);
    };
    protocol
        .state
        .child_reaped(status.code(), elapsed_ms(started))
        .map_err(state_failure)?;
    let teardown_ms = duration_millis(teardown_started.elapsed());

    let trace = protocol.collector.trace();
    let mut milestones = trace.milestones.clone();
    milestones.sampling_started_ms = Some(sampling_started_ms);
    milestones.sampling_finished_ms = Some(sampling_finished_ms);
    // The launcher-owned boundaries are installed by `successful_result`; this
    // assignment keeps their values available without inventing wire markers.
    let _ = milestones;
    Ok((memory_samples, exit_kind, status, teardown_ms))
}

fn wait_until_elapsed(
    due_ms: u64,
    started: Instant,
    child: &mut Child,
    lines: &mpsc::Receiver<String>,
    collector: &mut MarkerCollector,
    state: &mut LauncherStateMachine,
    font_resources: &mut Option<DiagnosticFontResourceSummary>,
) -> Result<(), RunFailure> {
    loop {
        let now_ms = elapsed_ms(started);
        if now_ms >= due_ms {
            return Ok(());
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|error| RunFailure::from_io("child_poll_failed", "sample", error))?
        {
            return Err(RunFailure::new(
                "child_exited_early",
                "sample",
                format!("child exited during sampling with {:?}", status.code()),
            ));
        }
        let wait = Duration::from_millis(due_ms.saturating_sub(now_ms)).min(PIPE_POLL_INTERVAL);
        match lines.recv_timeout(wait) {
            Ok(line) => {
                process_marker_line(&line, started, collector, Some(state), font_resources)?;
            }
            Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {}
        }
    }
}

fn process_marker_line(
    line: &str,
    started: Instant,
    collector: &mut MarkerCollector,
    state: Option<&mut LauncherStateMachine>,
    font_resources: &mut Option<DiagnosticFontResourceSummary>,
) -> Result<bool, RunFailure> {
    match collector
        .push_line(line)
        .map_err(|error| RunFailure::new("marker_invalid", "await_markers", error.to_string()))?
    {
        MarkerDisposition::Ignored => Ok(false),
        MarkerDisposition::Accepted(record) => {
            if record.kind == MarkerKind::GpuReady
                && let Some(value) = record.extra.get("font_resources")
            {
                *font_resources = Some(serde_json::from_value(value.clone()).map_err(|error| {
                    RunFailure::new("font_resources_invalid", "await_markers", error.to_string())
                })?);
            }
            let readiness_marker = state.as_deref().map_or(
                MarkerKind::ScenarioReady,
                LauncherStateMachine::readiness_marker,
            );
            if let Some(state) = state {
                state
                    .observe_marker(record.kind, elapsed_ms(started))
                    .map_err(state_failure)?;
            }
            Ok(record.kind == readiness_marker)
        }
    }
}

fn drain_available_markers(
    lines: &mpsc::Receiver<String>,
    started: Instant,
    collector: &mut MarkerCollector,
    mut state: Option<&mut LauncherStateMachine>,
    font_resources: &mut Option<DiagnosticFontResourceSummary>,
) -> Result<(), RunFailure> {
    while let Ok(line) = lines.try_recv() {
        process_marker_line(
            &line,
            started,
            collector,
            state.as_deref_mut(),
            font_resources,
        )?;
    }
    Ok(())
}

fn drain_late_markers(
    lines: &mpsc::Receiver<String>,
    collector: &mut MarkerCollector,
) -> Result<(), RunFailure> {
    while let Ok(line) = lines.try_recv() {
        collector
            .push_line(&line)
            .map_err(|error| RunFailure::new("marker_invalid", "reap", error.to_string()))?;
    }
    Ok(())
}

#[expect(
    clippy::too_many_arguments,
    reason = "schema assembly names each independently verified lifecycle artifact"
)]
fn successful_result(
    run: RunIdentity,
    options: &LauncherOptions,
    metric: MemoryMetric,
    pid: u32,
    samples: Vec<MemorySample>,
    exit_kind: ProcessExitKind,
    exit_code: Option<i32>,
    teardown_ms: u64,
    trace: CollectedMarkers,
    font_resources: Option<DiagnosticFontResourceSummary>,
) -> Result<DiagnosticsResult, RunFailure> {
    let statistics = summarize_bytes(
        &samples
            .iter()
            .map(|sample| sample.bytes)
            .collect::<Vec<_>>(),
    )
    .map_err(|error| RunFailure::new("statistics_failed", "emit_result", error.to_string()))?;
    let mut milestones = trace.milestones;
    let first_sample = samples.first().map(|sample| sample.elapsed_ms);
    let last_sample = samples.last().map(|sample| sample.elapsed_ms);
    milestones.sampling_started_ms = first_sample;
    milestones.sampling_finished_ms = last_sample;
    let readiness_evidence = if options.attribution_stage.is_some() {
        "validated attribution_stage_ready marker from the requested owner"
    } else if options.font_mode.is_some() && options.font_specimen.is_some() {
        "validated font_ownership_ready marker after a complete GPU frame"
    } else {
        "validated scenario_ready marker after a presented frame"
    };
    let result = DiagnosticsResult {
        schema: SchemaVersion::V2,
        run,
        configuration: options.configuration(),
        milestones,
        readiness: Readiness {
            status: ReadinessStatus::Ready,
            evidence: vec![readiness_evidence.to_owned()],
        },
        renderer: RendererSummary {
            first: trace.first_renderer,
            final_renderer: trace.final_renderer,
            backend: trace.gpu_backend,
            adapter_name: trace.gpu_adapter_name,
            adapter_vendor_id: trace.gpu_adapter_vendor_id,
            adapter_device_id: trace.gpu_adapter_device_id,
            adapter_type: trace.gpu_adapter_type,
        },
        connection: ConnectionSummary {
            final_state: trace
                .connection_state
                .unwrap_or(ConnectionState::NotStarted),
        },
        memory: MemorySummary {
            metric,
            unit: "bytes".to_owned(),
            samples,
            statistics,
        },
        process: ProcessSummary {
            pid,
            exit_kind,
            exit_code,
            teardown_ms: Some(teardown_ms),
        },
        font_resources,
        final_attribution_stage: trace.final_attribution_stage,
        resource_summary_schema: trace.resource_summary_schema,
        resource_summary: trace.resource_summary,
        failures: Vec::new(),
    };
    result.validate().map_err(|error| {
        RunFailure::new("schema_validation_failed", "emit_result", error.to_string())
    })?;
    Ok(result)
}

fn failed_execution(
    run: RunIdentity,
    options: &LauncherOptions,
    metric: MemoryMetric,
    pid: u32,
    failure: RunFailure,
    exit_kind: ProcessExitKind,
    exit_code: Option<i32>,
) -> LauncherExecution {
    failed_execution_with_trace(
        run,
        options,
        metric,
        pid,
        failure,
        exit_kind,
        exit_code,
        None,
        empty_trace(),
    )
}

#[allow(clippy::too_many_arguments)]
fn failed_execution_with_trace(
    run: RunIdentity,
    options: &LauncherOptions,
    metric: MemoryMetric,
    pid: u32,
    failure: RunFailure,
    exit_kind: ProcessExitKind,
    exit_code: Option<i32>,
    teardown_ms: Option<u64>,
    trace: CollectedMarkers,
) -> LauncherExecution {
    LauncherExecution {
        result: DiagnosticsResult {
            schema: SchemaVersion::V2,
            run,
            configuration: options.configuration(),
            milestones: trace.milestones,
            readiness: Readiness {
                status: ReadinessStatus::Failed,
                evidence: vec![failure.message.clone()],
            },
            renderer: RendererSummary {
                first: trace.first_renderer,
                final_renderer: trace.final_renderer,
                backend: trace.gpu_backend,
                adapter_name: trace.gpu_adapter_name,
                adapter_vendor_id: trace.gpu_adapter_vendor_id,
                adapter_device_id: trace.gpu_adapter_device_id,
                adapter_type: trace.gpu_adapter_type,
            },
            connection: ConnectionSummary {
                final_state: trace
                    .connection_state
                    .unwrap_or(ConnectionState::NotStarted),
            },
            memory: MemorySummary {
                metric,
                unit: "bytes".to_owned(),
                samples: Vec::new(),
                statistics: empty_statistics(),
            },
            process: ProcessSummary {
                pid,
                exit_kind,
                exit_code,
                teardown_ms,
            },
            font_resources: None,
            final_attribution_stage: None,
            resource_summary_schema: None,
            resource_summary: None,
            failures: vec![failure.into_schema()],
        },
        success: false,
    }
}

fn diagnostic_command(
    options: &LauncherOptions,
    run_id: &str,
    fixture: Option<&SshFixtureContext>,
) -> Result<Command, String> {
    let mut command = Command::new(&options.app);
    if options.product_gui {
        let address = fixture.map(|fixture| fixture.server.address());
        let session_log = fixture.map(|fixture| fixture.session_log.as_path());
        command
            .args(product_gui_arguments(options, address, session_log)?)
            .env(
                PRODUCT_GUI_PROBE_ENV,
                product_gui_probe_descriptor(options, run_id),
            );
    } else {
        command.args(diagnostic_arguments(options, run_id));
    }
    command.env("RTERM_BENCHMARK_WINDOW_SCALE_FACTOR", "1");
    if let Some(fixture) = fixture {
        let address = fixture.server.address();
        if !options.product_gui {
            command.args([
                "--ssh-host",
                &address.ip().to_string(),
                "--ssh-port",
                &address.port().to_string(),
                "--ssh-user",
                FIXTURE_USER,
                "--log",
                fixture.session_log.to_string_lossy().as_ref(),
            ]);
        }
        fixture.server.temp_home().apply_to(&mut command);
        command
            .env("RTERM_DIAGNOSTIC_SSH_SECRET", &fixture.secret)
            .env("SSH_AUTH_SOCK", "rterm-diagnostics-invalid-agent");
    }
    Ok(command)
}

pub(crate) fn diagnostic_arguments(options: &LauncherOptions, run_id: &str) -> Vec<String> {
    let scenario = match options.scenario {
        Scenario::EmptyWindow => "empty-window",
        Scenario::Ssh1 => "ssh1",
    };
    let mut arguments = vec![
        "diagnostic-gui".to_owned(),
        "--run-id".to_owned(),
        run_id.to_owned(),
        "--scenario".to_owned(),
        scenario.to_owned(),
        "--hold-ms".to_owned(),
        product_gui_hold_ms(options).to_string(),
        "--renderer".to_owned(),
        options.renderer.to_string(),
    ];
    if let Some(backend) = options.gpu_backend {
        arguments.extend(["--gpu-backend".to_owned(), backend.to_string()]);
    }
    if let (Some(mode), Some(specimen)) = (options.font_mode, options.font_specimen) {
        arguments.extend([
            "--font-mode".to_owned(),
            mode.to_string(),
            "--font-specimen".to_owned(),
            specimen.to_string(),
        ]);
    }
    if let Some(stage) = options.attribution_stage {
        arguments.extend(["--attribution-stage".to_owned(), stage.as_str().to_owned()]);
    }
    arguments.extend([
        "--cols".to_owned(),
        options.columns.to_string(),
        "--rows".to_owned(),
        options.rows.to_string(),
    ]);
    arguments
}

fn product_gui_hold_ms(options: &LauncherOptions) -> u64 {
    duration_millis(
        options
            .stabilization
            .saturating_add(options.sample_interval.saturating_mul(options.sample_count))
            .saturating_add(options.shutdown_timeout)
            .saturating_add(Duration::from_secs(30)),
    )
}

fn product_gui_probe_descriptor(options: &LauncherOptions, run_id: &str) -> String {
    let scenario = match options.scenario {
        Scenario::EmptyWindow => "empty-window",
        Scenario::Ssh1 => "ssh1",
    };
    serde_json::json!({
        "schema": "rterm.stage7/product-gui-probe/v1",
        "run_id": run_id,
        "scenario": scenario,
        "hold_ms": product_gui_hold_ms(options),
    })
    .to_string()
}

fn product_gui_arguments(
    options: &LauncherOptions,
    fixture_address: Option<SocketAddr>,
    session_log: Option<&Path>,
) -> Result<Vec<String>, String> {
    let address = match options.scenario {
        Scenario::EmptyWindow => SocketAddr::from(([127, 0, 0, 1], 9)),
        Scenario::Ssh1 => fixture_address
            .ok_or_else(|| "SSH1 product GUI probe requires a loopback fixture".to_owned())?,
    };
    let mut arguments = vec![
        "ssh".to_owned(),
        "--gui".to_owned(),
        "--renderer".to_owned(),
        "auto".to_owned(),
        "--host".to_owned(),
        address.ip().to_string(),
        "--port".to_owned(),
        address.port().to_string(),
        "--user".to_owned(),
        FIXTURE_USER.to_owned(),
        "--password".to_owned(),
        "--accept-unknown-host-key".to_owned(),
        "--cols".to_owned(),
        options.columns.to_string(),
        "--rows".to_owned(),
        options.rows.to_string(),
    ];
    if let Some(session_log) = session_log {
        arguments.extend([
            "--log".to_owned(),
            session_log.to_string_lossy().into_owned(),
        ]);
    }
    Ok(arguments)
}

struct SshFixtureContext {
    server: HermeticSshServer,
    secret: String,
    session_log: std::path::PathBuf,
}

impl SshFixtureContext {
    fn start(scenario: Scenario) -> Result<Option<Self>, String> {
        if scenario == Scenario::EmptyWindow {
            return Ok(None);
        }
        let secret = format!("rterm-diagnostic-secret-{}", unique_nonce());
        let server = HermeticSshServer::builder()
            .password(FIXTURE_USER, &secret)
            .start(Duration::from_secs(5))
            .map_err(|error| error.to_string())?;
        let session_log = server.temp_home().path().join("ssh1-session.log");
        Ok(Some(Self {
            server,
            secret,
            session_log,
        }))
    }

    fn stop(self, evidence_root: Option<&Path>) -> Result<(), String> {
        let retained = if let Some(root) = evidence_root {
            // Copy before the fixture removes its private temporary home.
            std::fs::copy(&self.session_log, root.join("session.log"))
                .map(|_| ())
                .map_err(|error| format!("retain session evidence: {error}"))
        } else {
            Ok(())
        };
        let stopped = self
            .server
            .stop(Duration::from_secs(5))
            .map_err(|error| error.to_string());
        retained.and(stopped)
    }
}

fn stop_fixture(
    fixture: Option<SshFixtureContext>,
    evidence_root: Option<&Path>,
) -> Result<(), String> {
    fixture.map_or(Ok(()), |fixture| fixture.stop(evidence_root))
}

fn native_sampler(pid: u32) -> Result<Box<dyn MemorySampler>, SamplerError> {
    #[cfg(target_os = "windows")]
    {
        return WindowsPrivateWorkingSetSampler::new(pid)
            .map(|sampler| Box::new(sampler) as Box<dyn MemorySampler>);
    }
    #[cfg(target_os = "linux")]
    {
        return crate::LinuxPssSampler::new(pid)
            .map(|sampler| Box::new(sampler) as Box<dyn MemorySampler>);
    }
    #[cfg(target_os = "macos")]
    {
        return crate::MacosPhysFootprintSampler::new(pid)
            .map(|sampler| Box::new(sampler) as Box<dyn MemorySampler>);
    }
    #[allow(unreachable_code)]
    Err(SamplerError::Unsupported {
        metric: native_metric(),
        detail: "the launcher has no native sampler for this target".to_owned(),
    })
}

const fn native_metric() -> MemoryMetric {
    #[cfg(target_os = "windows")]
    {
        return MemoryMetric::WindowsPrivateWorkingSetBytes;
    }
    #[cfg(target_os = "linux")]
    {
        return MemoryMetric::LinuxPssBytes;
    }
    #[cfg(target_os = "macos")]
    {
        return MemoryMetric::MacosPhysFootprintBytes;
    }
    #[allow(unreachable_code)]
    MemoryMetric::LinuxPssBytes
}

const fn native_platform() -> Platform {
    #[cfg(target_os = "windows")]
    {
        return Platform::Windows;
    }
    #[cfg(target_os = "linux")]
    {
        return Platform::Linux;
    }
    #[cfg(target_os = "macos")]
    {
        return Platform::Macos;
    }
    #[allow(unreachable_code)]
    Platform::Linux
}

fn run_identity(options: &LauncherOptions, run_id: &str) -> RunIdentity {
    RunIdentity {
        id: run_id.to_owned(),
        scenario: options.scenario,
        platform: native_platform(),
        architecture: std::env::consts::ARCH.to_owned(),
        app_path: options.app.to_string_lossy().into_owned(),
        app_version: "unknown".to_owned(),
        started_at_utc: format!("unix:{}", unix_seconds()),
        launcher_version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

fn unique_run_id(scenario: Scenario) -> String {
    format!(
        "{}-{}-{}",
        match scenario {
            Scenario::EmptyWindow => "empty-window",
            Scenario::Ssh1 => "ssh1",
        },
        std::process::id(),
        unique_nonce()
    )
}

fn unique_nonce() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn elapsed_ms(started: Instant) -> u64 {
    duration_millis(started.elapsed())
}

fn duration_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "map_err transfers the typed failure into its schema-facing representation"
)]
fn state_failure(failure: crate::LauncherFailure) -> RunFailure {
    RunFailure::new(
        "launcher_state_failed",
        phase_name(failure.phase),
        failure.to_string(),
    )
}

const fn phase_name(phase: LauncherPhase) -> &'static str {
    match phase {
        LauncherPhase::Launch => "launch",
        LauncherPhase::AwaitMarkers => "await_markers",
        LauncherPhase::AwaitScenarioReady => "await_scenario_ready",
        LauncherPhase::Stabilize => "stabilize",
        LauncherPhase::Sample => "sample",
        LauncherPhase::RequestShutdown => "request_shutdown",
        LauncherPhase::Reap => "reap",
        LauncherPhase::EmitResult => "emit_result",
        LauncherPhase::Failed => "failed",
    }
}

fn force_reap(child: &mut Child) -> Option<ExitStatus> {
    if let Ok(Some(status)) = child.try_wait() {
        return Some(status);
    }
    let _ = child.kill();
    child.wait().ok()
}

#[derive(Clone)]
struct BoundedTail {
    bytes: Arc<Mutex<VecDeque<u8>>>,
    limit: usize,
    capture: Option<Arc<Mutex<std::fs::File>>>,
    capture_error: Option<std::path::PathBuf>,
}

impl BoundedTail {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Arc::new(Mutex::new(VecDeque::with_capacity(limit))),
            limit,
            capture: None,
            capture_error: None,
        }
    }

    fn with_capture(mut self, path: Option<std::path::PathBuf>) -> Self {
        self.capture_error = path
            .as_ref()
            .and_then(|path| path.parent())
            .map(|parent| parent.join("capture-error"));
        self.capture = path
            .and_then(|path| std::fs::File::create(path).ok())
            .map(|file| Arc::new(Mutex::new(file)));
        self
    }

    fn mark_capture_error(&self) {
        if let Some(path) = &self.capture_error {
            let _ = std::fs::write(path, b"stream capture failed");
        }
    }

    fn push(&self, bytes: &[u8]) {
        if let Some(file) = &self.capture {
            let mut file = file
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if file.write_all(bytes).is_err() {
                // A missing completion receipt makes evidence consumers fail closed.
                self.mark_capture_error();
            }
        }
        let mut tail = self
            .bytes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        tail.extend(bytes);
        let excess = tail.len().saturating_sub(self.limit);
        tail.drain(..excess);
    }

    fn text(&self) -> String {
        let tail = self
            .bytes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        String::from_utf8_lossy(&tail.iter().copied().collect::<Vec<_>>()).into_owned()
    }
}

/// Optional launcher-owned raw capture. The caller supplies a fresh directory;
/// no secret value or digest is serialized into the public scan receipt.
struct StreamEvidence {
    root: Option<std::path::PathBuf>,
    secret: Option<String>,
}

impl StreamEvidence {
    fn from_environment(secret: Option<String>) -> std::io::Result<Self> {
        Self::create(
            std::env::var_os("RTERM_DIAGNOSTIC_EVIDENCE_DIR").map(std::path::PathBuf::from),
            secret,
        )
    }

    fn create(root: Option<std::path::PathBuf>, secret: Option<String>) -> std::io::Result<Self> {
        if let Some(root) = &root {
            std::fs::create_dir(root)?;
        }
        Ok(Self { root, secret })
    }

    fn path(&self, name: &str) -> Option<std::path::PathBuf> {
        self.root.as_ref().map(|root| root.join(name))
    }
}

impl Drop for StreamEvidence {
    fn drop(&mut self) {
        let Some(root) = &self.root else { return };
        let mut complete = !root.join("capture-error").exists();
        let mut files = Vec::new();
        let mut names = vec!["stdout.txt", "stderr.txt"];
        if self.secret.is_some() {
            names.push("session.log");
        }
        for name in names {
            match std::fs::read(root.join(name)) {
                Ok(bytes) => {
                    let hits = self.secret.as_ref().map_or(0, |secret| {
                        bytes
                            .windows(secret.len())
                            .filter(|part| *part == secret.as_bytes())
                            .count()
                    });
                    complete &= hits == 0;
                    files.push(
                        serde_json::json!({"path": name, "bytes": bytes.len(), "hits": hits}),
                    );
                }
                Err(_) => complete = false,
            }
        }
        let receipt = serde_json::json!({
            "schema": "rterm.diagnostics/raw-capture/v1",
            "complete": complete,
            "actual_fixture_secret_checked": self.secret.is_some(),
            "coverage": ["stdout-including-markers", "stderr", "ssh-session-log-if-applicable"],
            "files": files,
        });
        if let Ok(bytes) = serde_json::to_vec_pretty(&receipt) {
            let _ = std::fs::write(root.join("scan.json"), bytes);
        }
    }
}

fn spawn_stdout_drain(
    stdout: ChildStdout,
    tail: BoundedTail,
    sender: mpsc::Sender<String>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut bytes = Vec::new();
        loop {
            bytes.clear();
            match reader.read_until(b'\n', &mut bytes) {
                Ok(0) => break,
                Err(_) => {
                    tail.mark_capture_error();
                    break;
                }
                Ok(_) => {
                    tail.push(&bytes);
                    let line = String::from_utf8_lossy(&bytes)
                        .trim_end_matches(['\r', '\n'])
                        .to_owned();
                    if sender.send(line).is_err() {
                        break;
                    }
                }
            }
        }
    })
}

fn spawn_stderr_drain(
    mut stderr: impl Read + Send + 'static,
    tail: BoundedTail,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        loop {
            match stderr.read(&mut buffer) {
                Ok(0) => break,
                Err(_) => {
                    tail.mark_capture_error();
                    break;
                }
                Ok(count) => tail.push(&buffer[..count]),
            }
        }
    })
}

fn join_pipe_threads(stdout: thread::JoinHandle<()>, stderr: thread::JoinHandle<()>) {
    let _ = stdout.join();
    let _ = stderr.join();
}

#[derive(Debug)]
struct RunFailure {
    code: String,
    phase: String,
    message: String,
    os_error_code: Option<i64>,
    context: Option<String>,
}

impl RunFailure {
    fn new(code: impl Into<String>, phase: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            phase: phase.into(),
            message: message.into(),
            os_error_code: None,
            context: None,
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "the source error is intentionally consumed at the error boundary"
    )]
    fn from_io(code: &'static str, phase: &'static str, error: std::io::Error) -> Self {
        let os_error_code = error.raw_os_error().map(i64::from);
        let mut failure = Self::new(code, phase, error.to_string());
        failure.os_error_code = os_error_code;
        failure
    }

    fn into_schema(self) -> DiagnosticFailure {
        DiagnosticFailure {
            code: self.code,
            phase: self.phase,
            message: self.message,
            os_error_code: self.os_error_code,
            recoverable: false,
            context: self.context,
        }
    }
}

fn failure_with_tails(
    mut failure: RunFailure,
    stdout: &BoundedTail,
    stderr: &BoundedTail,
    secret: Option<&str>,
) -> RunFailure {
    let mut context = format!(
        "stdout_tail={:?}; stderr_tail={:?}",
        stdout.text(),
        stderr.text()
    );
    if let Some(secret) = secret {
        context = context.replace(secret, "<redacted>");
    }
    failure.context = Some(context);
    failure
}

fn empty_trace() -> CollectedMarkers {
    CollectedMarkers {
        milestones: StartupMilestones::default(),
        first_renderer: None,
        final_renderer: None,
        connection_state: None,
        gpu_backend: None,
        gpu_adapter_name: None,
        gpu_adapter_vendor_id: None,
        gpu_adapter_device_id: None,
        gpu_adapter_type: None,
        final_attribution_stage: None,
        resource_summary_schema: None,
        resource_summary: None,
    }
}

const fn empty_statistics() -> MemoryStatistics {
    MemoryStatistics {
        count: 0,
        min: 0,
        max: 0,
        mean: 0,
        median: 0,
        p50: 0,
        p95: 0,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::{DiagnosticGpuBackend, DiagnosticRendererMode, MARKER_PREFIX};

    fn fixture_options() -> LauncherOptions {
        LauncherOptions {
            app: PathBuf::from("rterm-app"),
            scenario: Scenario::EmptyWindow,
            stabilization: Duration::from_millis(1),
            sample_interval: Duration::from_millis(1),
            sample_count: 1,
            shutdown_timeout: Duration::from_millis(1),
            columns: 80,
            rows: 24,
            renderer: DiagnosticRendererMode::Gpu,
            gpu_backend: Some(DiagnosticGpuBackend::Dx12),
            font_mode: None,
            font_specimen: None,
            attribution_stage: None,
            product_gui: false,
            json: true,
        }
    }

    fn gpu_identity_trace() -> CollectedMarkers {
        CollectedMarkers {
            milestones: StartupMilestones::default(),
            first_renderer: Some(crate::RendererKind::Cpu),
            final_renderer: Some(crate::RendererKind::Gpu),
            connection_state: Some(ConnectionState::NotStarted),
            gpu_backend: Some(DiagnosticGpuBackend::Dx12),
            gpu_adapter_name: Some("fixture-adapter".to_owned()),
            gpu_adapter_vendor_id: Some(0x10de),
            gpu_adapter_device_id: Some(0x2684),
            gpu_adapter_type: Some("discrete-gpu".to_owned()),
            final_attribution_stage: None,
            resource_summary_schema: None,
            resource_summary: None,
        }
    }

    fn assert_gpu_identity(renderer: &RendererSummary) {
        assert_eq!(renderer.backend, Some(DiagnosticGpuBackend::Dx12));
        assert_eq!(renderer.adapter_name.as_deref(), Some("fixture-adapter"));
        assert_eq!(renderer.adapter_vendor_id, Some(0x10de));
        assert_eq!(renderer.adapter_device_id, Some(0x2684));
        assert_eq!(renderer.adapter_type.as_deref(), Some("discrete-gpu"));
    }

    #[test]
    fn successful_result_copies_collected_gpu_identity() {
        let options = fixture_options();
        let result = successful_result(
            RunIdentity::fixture(Scenario::EmptyWindow, Platform::Windows),
            &options,
            MemoryMetric::WindowsPrivateWorkingSetBytes,
            42,
            vec![MemorySample {
                sequence: 0,
                elapsed_ms: 10,
                bytes: 1,
            }],
            ProcessExitKind::Requested,
            Some(0),
            1,
            gpu_identity_trace(),
            None,
        )
        .expect("valid successful fixture result");

        assert_gpu_identity(&result.renderer);
    }

    #[test]
    fn successful_font_proof_reports_owner_readiness_and_serializes_auto_renderer() {
        let mut options = fixture_options();
        options.renderer = DiagnosticRendererMode::Auto;
        options.gpu_backend = None;
        options.font_mode = Some(crate::DiagnosticFontMode::Lazy);
        options.font_specimen = Some(crate::DiagnosticFontSpecimen::Ascii);
        let result = successful_result(
            RunIdentity::fixture(Scenario::EmptyWindow, Platform::Windows),
            &options,
            MemoryMetric::WindowsPrivateWorkingSetBytes,
            42,
            vec![MemorySample {
                sequence: 0,
                elapsed_ms: 10,
                bytes: 1,
            }],
            ProcessExitKind::Requested,
            Some(0),
            1,
            gpu_identity_trace(),
            None,
        )
        .expect("valid font proof result");
        assert_eq!(
            result.readiness.evidence,
            ["validated font_ownership_ready marker after a complete GPU frame"]
        );
        let serialized = serde_json::to_value(result).expect("serialize real result");
        assert_eq!(serialized["configuration"]["requested_renderer"], "auto");
    }

    #[test]
    fn gpu_ready_marker_captures_font_resources_for_the_launcher_result() {
        let mut collector =
            MarkerCollector::new(MarkerIdentity::new("font-proof", 42, Scenario::EmptyWindow));
        let mut captured = None;
        let marker = format!(
            "{MARKER_PREFIX}{{\"schema\":\"rterm.diagnostics/v2\",\"run_id\":\"font-proof\",\"pid\":42,\"scenario\":\"empty_window\",\"kind\":\"gpu_ready\",\"elapsed_ms\":10,\"renderer\":\"gpu\",\"font_resources\":{{\"mode\":\"lazy\",\"specimen\":\"cjk\",\"retained_source_bytes\":64,\"indexed_source_count\":3,\"active_source_count\":2,\"initial_catalog_source_count\":1,\"catalog_builds\":2,\"generation\":2,\"recovery_retained_source_bytes\":64,\"recovery_generation\":2,\"activation_latency_micros\":9,\"tofu_count\":0,\"frame_catalog_generation\":2,\"frame_generation_consistent\":true,\"index_fingerprint_sha256\":\"{}\",\"catalog_fingerprint_sha256\":\"{}\",\"ordered_catalog_fingerprint_sha256\":\"{}\"}}}}",
            "1".repeat(64),
            "2".repeat(64),
            "3".repeat(64),
        );

        process_marker_line(&marker, Instant::now(), &mut collector, None, &mut captured)
            .expect("valid font resource marker");

        let captured = captured.expect("font resources captured");
        assert_eq!(captured.mode, crate::DiagnosticFontMode::Lazy);
        assert_eq!(captured.specimen, crate::DiagnosticFontSpecimen::Cjk);
        assert_eq!(captured.initial_catalog_source_count, 1);
        assert_eq!(captured.frame_generation_consistent, Some(true));
    }

    #[test]
    fn failed_result_copies_collected_gpu_identity() {
        let options = fixture_options();
        let execution = failed_execution_with_trace(
            RunIdentity::fixture(Scenario::EmptyWindow, Platform::Windows),
            &options,
            MemoryMetric::WindowsPrivateWorkingSetBytes,
            42,
            RunFailure::new("fixture_failure", "fixture", "fixture failure"),
            ProcessExitKind::Forced,
            Some(1),
            Some(1),
            gpu_identity_trace(),
        );

        assert_gpu_identity(&execution.result.renderer);
    }

    #[test]
    fn diagnostic_arguments_forward_requested_renderer_and_backend_exactly() {
        let options = LauncherOptions {
            app: PathBuf::from("rterm-app"),
            scenario: Scenario::EmptyWindow,
            stabilization: Duration::from_millis(5_000),
            sample_interval: Duration::from_millis(100),
            sample_count: 10,
            shutdown_timeout: Duration::from_millis(2_000),
            columns: 80,
            rows: 24,
            renderer: DiagnosticRendererMode::Gpu,
            gpu_backend: Some(DiagnosticGpuBackend::Dx12),
            font_mode: None,
            font_specimen: None,
            attribution_stage: None,
            product_gui: false,
            json: true,
        };

        assert_eq!(
            diagnostic_arguments(&options, "probe-run"),
            [
                "diagnostic-gui",
                "--run-id",
                "probe-run",
                "--scenario",
                "empty-window",
                "--hold-ms",
                "38000",
                "--renderer",
                "gpu",
                "--gpu-backend",
                "dx12",
                "--cols",
                "80",
                "--rows",
                "24",
            ]
            .map(str::to_owned)
        );
    }

    #[test]
    fn diagnostic_arguments_keep_auto_default_without_a_backend_override() {
        let options = LauncherOptions {
            app: PathBuf::from("rterm-app"),
            scenario: Scenario::EmptyWindow,
            stabilization: Duration::from_millis(5_000),
            sample_interval: Duration::from_millis(100),
            sample_count: 10,
            shutdown_timeout: Duration::from_millis(2_000),
            columns: 80,
            rows: 24,
            renderer: DiagnosticRendererMode::Auto,
            gpu_backend: None,
            font_mode: None,
            font_specimen: None,
            attribution_stage: None,
            product_gui: false,
            json: true,
        };

        let arguments = diagnostic_arguments(&options, "default-run");

        assert_eq!(
            arguments
                .windows(2)
                .find(|pair| pair[0] == "--renderer")
                .map(|pair| pair[1].as_str()),
            Some("auto")
        );
        assert!(!arguments.iter().any(|argument| argument == "--gpu-backend"));
    }

    #[test]
    fn product_gui_empty_window_uses_normal_ssh_entrypoint() {
        let options = LauncherOptions {
            stabilization: Duration::from_millis(5_000),
            sample_interval: Duration::from_millis(100),
            sample_count: 10,
            shutdown_timeout: Duration::from_millis(2_000),
            renderer: DiagnosticRendererMode::Auto,
            gpu_backend: None,
            product_gui: true,
            ..fixture_options()
        };

        let arguments =
            product_gui_arguments(&options, None, None).expect("empty product probe arguments");
        let descriptor = product_gui_probe_descriptor(&options, "product-empty");
        let descriptor: serde_json::Value =
            serde_json::from_str(&descriptor).expect("product probe descriptor JSON");

        assert_eq!(&arguments[..2], ["ssh", "--gui"]);
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--renderer", "auto"])
        );
        assert!(arguments.windows(2).any(|pair| pair == ["--port", "9"]));
        assert!(
            !arguments
                .iter()
                .any(|argument| argument == "diagnostic-gui")
        );
        assert_eq!(
            descriptor,
            serde_json::json!({
                "schema": "rterm.stage7/product-gui-probe/v1",
                "run_id": "product-empty",
                "scenario": "empty-window",
                "hold_ms": 38000_u64,
            })
        );
        assert!(!descriptor.to_string().contains("secret"));
    }

    #[test]
    fn product_gui_ssh1_binds_the_loopback_fixture() {
        let options = LauncherOptions {
            scenario: Scenario::Ssh1,
            renderer: DiagnosticRendererMode::Auto,
            gpu_backend: None,
            product_gui: true,
            ..fixture_options()
        };
        let log = PathBuf::from("fixture-session.log");
        let address = "127.0.0.1:2222".parse().expect("fixture address");

        let arguments = product_gui_arguments(&options, Some(address), Some(&log))
            .expect("SSH1 product probe arguments");

        for pair in [
            ["--host", "127.0.0.1"],
            ["--port", "2222"],
            ["--user", FIXTURE_USER],
            ["--log", "fixture-session.log"],
        ] {
            assert!(
                arguments.windows(2).any(|actual| actual == pair),
                "missing pair {pair:?} in {arguments:?}"
            );
        }
        assert!(arguments.iter().any(|argument| argument == "--password"));
        assert!(
            arguments
                .iter()
                .any(|argument| argument == "--accept-unknown-host-key")
        );
        assert!(
            !arguments
                .iter()
                .any(|argument| argument == "diagnostic-gui")
        );
    }

    #[test]
    fn attribution_stage_is_forwarded_and_captured_in_the_final_launcher_result() {
        use crate::{
            DiagnosticAttributionStage, ProjectOwnedResourceMetricsV1,
            ProjectOwnedResourceSchemaVersion,
        };

        let options = LauncherOptions {
            renderer: DiagnosticRendererMode::Cpu,
            gpu_backend: None,
            attribution_stage: Some(DiagnosticAttributionStage::CpuWindow),
            ..fixture_options()
        };
        let arguments = diagnostic_arguments(&options, "attribution-run");
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--attribution-stage", "cpu-window"]),
            "attribution stage was not forwarded: {arguments:?}"
        );

        let resources = ProjectOwnedResourceMetricsV1 {
            cpu_staging_bytes: 4,
            cpu_surface_count: 1,
            cpu_present_count: 1,
            ..ProjectOwnedResourceMetricsV1::default()
        };
        let mut trace = empty_trace();
        trace.final_attribution_stage = Some(DiagnosticAttributionStage::CpuWindow);
        trace.resource_summary_schema = Some(ProjectOwnedResourceSchemaVersion::V1);
        trace.resource_summary = Some(resources.clone());
        trace.first_renderer = Some(crate::RendererKind::Cpu);
        trace.final_renderer = Some(crate::RendererKind::Cpu);

        let result = successful_result(
            RunIdentity::fixture(Scenario::EmptyWindow, Platform::Windows),
            &options,
            MemoryMetric::WindowsPrivateWorkingSetBytes,
            42,
            vec![MemorySample {
                sequence: 0,
                elapsed_ms: 5_100,
                bytes: 1,
            }],
            ProcessExitKind::Requested,
            Some(0),
            1,
            trace,
            None,
        )
        .expect("complete attribution launcher result");
        assert_eq!(
            result.configuration.requested_attribution_stage,
            Some(DiagnosticAttributionStage::CpuWindow)
        );
        assert_eq!(
            result.final_attribution_stage,
            Some(DiagnosticAttributionStage::CpuWindow)
        );
        assert_eq!(
            result.resource_summary_schema,
            Some(ProjectOwnedResourceSchemaVersion::V1)
        );
        assert_eq!(result.resource_summary, Some(resources));
        assert_eq!(
            result.readiness.evidence,
            ["validated attribution_stage_ready marker from the requested owner"]
        );
    }

    #[test]
    fn attribution_stage_late_service_marker_is_not_discarded_during_final_drain() {
        use crate::{DiagnosticAttributionStage, ProjectOwnedResourceMetricsV1};

        let mut collector = MarkerCollector::new_attribution(
            MarkerIdentity::new("late-stage", 42, Scenario::EmptyWindow),
            DiagnosticAttributionStage::CpuWindow,
        );
        let identity = |kind: &str, elapsed_ms: u64| {
            format!(
                "{MARKER_PREFIX}{}",
                serde_json::json!({
                    "schema": "rterm.diagnostics/v2",
                    "run_id": "late-stage",
                    "pid": 42,
                    "scenario": "empty_window",
                    "kind": kind,
                    "elapsed_ms": elapsed_ms,
                })
            )
        };
        collector
            .push_line(&identity("process_started", 0))
            .unwrap();
        collector.push_line(&identity("window_created", 1)).unwrap();
        let first = identity("first_present", 2)
            .replace("\"elapsed_ms\":2", "\"elapsed_ms\":2,\"renderer\":\"cpu\"");
        collector.push_line(&first).unwrap();
        let resources = ProjectOwnedResourceMetricsV1 {
            cpu_staging_bytes: 4,
            cpu_surface_count: 1,
            cpu_present_count: 1,
            ..ProjectOwnedResourceMetricsV1::default()
        };
        let ready = format!(
            "{MARKER_PREFIX}{}",
            serde_json::json!({
                "schema": "rterm.diagnostics/v2",
                "run_id": "late-stage",
                "pid": 42,
                "scenario": "empty_window",
                "kind": "attribution_stage_ready",
                "elapsed_ms": 3,
                "renderer": "cpu",
                "requested_stage": "cpu-window",
                "final_stage": "cpu-window",
                "resource_summary_schema": "rterm.project-owned-resources/v1",
                "resource_summary": resources,
            })
        );
        collector.push_line(&ready).unwrap();

        let (sender, receiver) = mpsc::channel();
        sender.send(identity("transport_started", 4)).unwrap();
        drop(sender);
        let failure = drain_late_markers(&receiver, &mut collector)
            .expect_err("late transport activity must fail the completed attribution run");
        assert_eq!(failure.code, "marker_invalid");
    }
}

#[cfg(test)]
mod raw_capture_tests {
    use super::{BoundedTail, StreamEvidence, unique_nonce};

    #[test]
    fn reused_capture_directory_is_rejected_without_borrowing_old_evidence() {
        let root = std::env::temp_dir().join(format!("rterm-capture-{}", unique_nonce()));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("scan.json"), b"old receipt").unwrap();
        assert!(StreamEvidence::create(Some(root.clone()), None).is_err());
        assert_eq!(
            std::fs::read(root.join("scan.json")).unwrap(),
            b"old receipt"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn secret_scan_checks_full_stream_across_chunks_beyond_tail() {
        let root = std::env::temp_dir().join(format!("rterm-capture-{}", unique_nonce()));
        std::fs::create_dir(&root).unwrap();
        let evidence = StreamEvidence {
            root: Some(root.clone()),
            secret: Some("split-secret".to_owned()),
        };
        let stdout = BoundedTail::new(4).with_capture(evidence.path("stdout.txt"));
        let stderr = BoundedTail::new(4).with_capture(evidence.path("stderr.txt"));
        stdout.push(b"prefix split-");
        stdout.push(b"secret suffix beyond the bounded tail");
        stderr.push(b"safe stderr");
        std::fs::write(root.join("session.log"), b"safe session").unwrap();
        drop(stdout);
        drop(stderr);
        drop(evidence);
        let scan: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("scan.json")).unwrap()).unwrap();
        assert_eq!(scan["complete"], false);
        assert_eq!(scan["files"][0]["hits"], 1);
        assert!(!scan.to_string().contains("split-secret"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_session_capture_cannot_report_complete() {
        let root = std::env::temp_dir().join(format!("rterm-capture-{}", unique_nonce()));
        std::fs::create_dir(&root).unwrap();
        let evidence = StreamEvidence {
            root: Some(root.clone()),
            secret: Some("fixture-secret".to_owned()),
        };
        std::fs::write(root.join("stdout.txt"), b"safe").unwrap();
        std::fs::write(root.join("stderr.txt"), b"safe").unwrap();
        drop(evidence);
        let scan: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("scan.json")).unwrap()).unwrap();
        assert_eq!(scan["complete"], false);
        std::fs::remove_dir_all(root).unwrap();
    }
}
