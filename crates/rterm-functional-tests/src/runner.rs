use std::{
    collections::BTreeSet,
    error::Error,
    fmt, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus},
    time::{Duration, Instant},
};

use serde::Serialize;

use crate::hermetic_network::{apply_loopback_only_environment, hermetic_app_command};
use crate::{
    ActionV1, Capability, CheckpointContext, CheckpointV1, EvidenceEventV1, EvidenceWriter,
    FunctionalSuite, ScenarioOutcome, ScenarioRunId, ScenarioV1, Surface, assign_lpt_shards,
    evaluate_checkpoint,
};
use rterm_test_support::{ChildGuard, ChildGuardError};

pub const EXIT_USAGE: i32 = 2;
pub const EXIT_INVALID_SUITE: i32 = 3;
pub const EXIT_INFRASTRUCTURE_FAILED: i32 = 4;
pub const EXIT_SCENARIO_FAILED: i32 = 5;
const FUNCTIONAL_X11_WINDOW_CLASS: &str = "rterm-functional";

pub fn run_cli(args: impl IntoIterator<Item = String>) -> i32 {
    match RunnerCommand::parse(args) {
        Ok(command) => match command.execute(&mut io::stdout(), &mut io::stderr()) {
            Ok(()) => 0,
            Err(error) => {
                let _ = writeln!(io::stderr(), "{error}");
                error.exit_code()
            }
        },
        Err(error) => {
            let _ = writeln!(io::stderr(), "{error}");
            EXIT_USAGE
        }
    }
}

#[derive(Debug)]
enum RunnerCommand {
    Validate {
        suite: PathBuf,
    },
    List {
        suite: PathBuf,
    },
    Shard {
        suite: PathBuf,
        count: usize,
    },
    RunShard {
        suite: PathBuf,
        count: usize,
        index: usize,
        surface: Surface,
        target: String,
        evidence: PathBuf,
        app: Option<PathBuf>,
        fixture_bin: Option<PathBuf>,
        capabilities: BTreeSet<Capability>,
    },
    Coverage {
        suite: PathBuf,
        map: PathBuf,
        evidence_root: PathBuf,
    },
    Run {
        suite: PathBuf,
        scenario: String,
        target: String,
        evidence: PathBuf,
        app: Option<PathBuf>,
        fixture_bin: Option<PathBuf>,
        capabilities: BTreeSet<Capability>,
    },
}

impl RunnerCommand {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, CliError> {
        let mut args = args.into_iter();
        let _program = args.next();
        let verb = args.next().ok_or(CliError::MissingCommand)?;
        let options = ParsedOptions::parse(args)?;
        match verb.as_str() {
            "validate" => Ok(Self::Validate {
                suite: options.required_path("--suite")?,
            }),
            "list" => Ok(Self::List {
                suite: options.required_path("--suite")?,
            }),
            "shard" => Ok(Self::Shard {
                suite: options.required_path("--suite")?,
                count: options.required_usize("--count")?,
            }),
            "run-shard" => Ok(Self::RunShard {
                suite: options.required_path("--suite")?,
                count: options.required_usize("--count")?,
                index: options.required_usize("--index")?,
                surface: parse_surface(&options.required("--surface")?)?,
                target: options.required("--target")?,
                evidence: options.required_path("--evidence")?,
                app: options.optional_path("--app"),
                fixture_bin: options.optional_path("--fixture-bin"),
                capabilities: options.capabilities,
            }),
            "coverage" => Ok(Self::Coverage {
                suite: options.required_path("--suite")?,
                map: options.required_path("--map")?,
                evidence_root: options.required_path("--evidence-root")?,
            }),
            "run" => Ok(Self::Run {
                suite: options.required_path("--suite")?,
                scenario: options.required("--scenario")?,
                target: options.required("--target")?,
                evidence: options.required_path("--evidence")?,
                app: options.optional_path("--app"),
                fixture_bin: options.optional_path("--fixture-bin"),
                capabilities: options.capabilities,
            }),
            _ => Err(CliError::UnknownCommand(verb)),
        }
    }

    fn execute(self, stdout: &mut impl Write, _stderr: &mut impl Write) -> Result<(), RunnerError> {
        match self {
            Self::Validate { suite } => {
                let suite = FunctionalSuite::load(suite)
                    .map_err(|error| RunnerError::Suite(Box::new(error)))?;
                serde_json::to_writer(
                    &mut *stdout,
                    &ValidationReport {
                        schema: 1,
                        scenarios: suite.scenarios.len(),
                        behaviors: suite.catalog.behaviors.len(),
                    },
                )
                .map_err(RunnerError::Json)?;
                writeln!(stdout).map_err(RunnerError::Io)
            }
            Self::List { suite } => {
                let suite = FunctionalSuite::load(suite)
                    .map_err(|error| RunnerError::Suite(Box::new(error)))?;
                for scenario in suite.scenarios {
                    serde_json::to_writer(
                        &mut *stdout,
                        &ScenarioSummary {
                            schema: 1,
                            id: &scenario.id,
                            surface: scenario.surface,
                            capabilities: &scenario.capabilities,
                            estimated_cost_ms: scenario.estimated_cost_ms,
                        },
                    )
                    .map_err(RunnerError::Json)?;
                    writeln!(stdout).map_err(RunnerError::Io)?;
                }
                Ok(())
            }
            Self::Shard { suite, count } => {
                let suite = FunctionalSuite::load(suite)
                    .map_err(|error| RunnerError::Suite(Box::new(error)))?;
                let shards = assign_lpt_shards(
                    suite
                        .scenarios
                        .iter()
                        .map(|scenario| (scenario.id.as_str(), scenario.estimated_cost_ms)),
                    count,
                )
                .map_err(|source| RunnerError::InvalidShard(source.to_string()))?;
                serde_json::to_writer(&mut *stdout, &ShardReport { schema: 1, shards })
                    .map_err(RunnerError::Json)?;
                writeln!(stdout).map_err(RunnerError::Io)
            }
            Self::RunShard {
                suite,
                count,
                index,
                surface,
                target,
                evidence,
                app,
                fixture_bin,
                capabilities,
            } => execute_lpt_shard(&LptShardRun {
                suite: &suite,
                count,
                index,
                surface,
                target: &target,
                evidence: &evidence,
                app: app.as_deref(),
                fixture_bin: fixture_bin.as_deref(),
                capabilities: &capabilities,
            }),
            Self::Coverage {
                suite,
                map,
                evidence_root,
            } => execute_coverage(&suite, &map, &evidence_root, stdout),
            Self::Run {
                suite,
                scenario,
                target,
                evidence,
                app,
                fixture_bin,
                capabilities,
            } => execute_scenario(
                &suite,
                &scenario,
                &target,
                &evidence,
                app.as_deref(),
                fixture_bin.as_deref(),
                &capabilities,
            ),
        }
    }
}

struct LptShardRun<'a> {
    suite: &'a Path,
    count: usize,
    index: usize,
    surface: Surface,
    target: &'a str,
    evidence: &'a Path,
    app: Option<&'a Path>,
    fixture_bin: Option<&'a Path>,
    capabilities: &'a BTreeSet<Capability>,
}

fn execute_lpt_shard(run: &LptShardRun<'_>) -> Result<(), RunnerError> {
    let suite =
        FunctionalSuite::load(run.suite).map_err(|error| RunnerError::Suite(Box::new(error)))?;
    let scenarios = suite
        .scenarios
        .iter()
        .filter(|scenario| scenario.surface == run.surface)
        .collect::<Vec<_>>();
    let shards = assign_lpt_shards(
        scenarios
            .iter()
            .map(|scenario| (scenario.id.as_str(), scenario.estimated_cost_ms)),
        run.count,
    )
    .map_err(|source| RunnerError::InvalidShard(source.to_string()))?;
    let shard = shards.get(run.index).ok_or_else(|| {
        RunnerError::InvalidShard(format!(
            "shard index {} is outside the configured count {}",
            run.index, run.count
        ))
    })?;
    for scenario_id in &shard.scenario_ids {
        execute_scenario(
            run.suite,
            scenario_id,
            run.target,
            run.evidence,
            run.app,
            run.fixture_bin,
            run.capabilities,
        )?;
    }
    Ok(())
}

fn execute_coverage(
    suite_path: &Path,
    map_path: &Path,
    evidence_root: &Path,
    stdout: &mut impl Write,
) -> Result<(), RunnerError> {
    let suite =
        FunctionalSuite::load(suite_path).map_err(|error| RunnerError::Suite(Box::new(error)))?;
    let map_contents = fs::read_to_string(map_path).map_err(RunnerError::Io)?;
    let map = crate::BehaviorEvidenceMapV1::from_toml(&map_contents).map_err(|source| {
        RunnerError::Coverage(format!("parse `{}`: {source}", map_path.display()))
    })?;
    let mut files = Vec::new();
    collect_evidence_files(evidence_root, &mut files)?;
    files.sort();
    let mut scenarios = Vec::new();
    let mut libtests = Vec::new();
    let mut playwright = Vec::new();
    for path in files {
        let bytes = fs::read(&path).map_err(RunnerError::Io)?;
        match path.file_name().and_then(|name| name.to_str()) {
            Some(name) if name.ends_with(".ndjson") => scenarios.push(io::Cursor::new(bytes)),
            Some(name) if name.ends_with(".libtest") => libtests.push(io::Cursor::new(bytes)),
            Some(name) if name.ends_with(".playwright.json") => {
                playwright.push(io::Cursor::new(bytes));
            }
            _ => {}
        }
    }
    let report = crate::verify_behavior_coverage(
        &suite,
        &map,
        crate::CoverageInputs {
            scenario_ndjson: scenarios,
            libtest_listings: libtests,
            playwright_reports: playwright,
        },
    )
    .map_err(|errors| RunnerError::Coverage(errors.join("; ")))?;
    serde_json::to_writer(&mut *stdout, &report).map_err(RunnerError::Json)?;
    writeln!(stdout).map_err(RunnerError::Io)
}

fn collect_evidence_files(
    path: &std::path::Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), RunnerError> {
    for entry in fs::read_dir(path).map_err(RunnerError::Io)? {
        let entry = entry.map_err(RunnerError::Io)?;
        let path = entry.path();
        if path.is_dir() {
            collect_evidence_files(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

fn execute_scenario(
    suite_path: &Path,
    scenario_id: &str,
    target: &str,
    evidence_dir: &Path,
    app: Option<&Path>,
    fixture_bin: Option<&Path>,
    available: &BTreeSet<Capability>,
) -> Result<(), RunnerError> {
    let app = app.map(absolute_from_current).transpose()?;
    let fixture_bin = fixture_bin.map(absolute_from_current).transpose()?;
    let suite =
        FunctionalSuite::load(suite_path).map_err(|error| RunnerError::Suite(Box::new(error)))?;
    let scenario = suite
        .scenario(scenario_id)
        .ok_or_else(|| RunnerError::UnknownScenario(scenario_id.to_owned()))?;
    fs::create_dir_all(evidence_dir).map_err(RunnerError::Io)?;
    let run_id = ScenarioRunId::new(scenario_id, target, 0)
        .map_err(|source| RunnerError::InvalidRunId(source.to_string()))?;
    let evidence_path = evidence_dir.join(format!("{scenario_id}.{target}.0.ndjson"));
    let file = fs::File::create(evidence_path).map_err(RunnerError::Io)?;
    let mut writer = EvidenceWriter::new(file, run_id);
    let started = Instant::now();
    let capability_names = scenario
        .capabilities
        .iter()
        .copied()
        .map(capability_name)
        .map(str::to_owned)
        .collect();
    writer
        .record(EvidenceEventV1::scenario_started_with_capabilities(
            elapsed_ms(started),
            capability_names,
        ))
        .map_err(|source| RunnerError::Evidence(source.to_string()))?;
    let missing: Vec<_> = scenario
        .capabilities
        .iter()
        .filter(|capability| !available.contains(capability))
        .copied()
        .map(capability_name)
        .collect();
    if !missing.is_empty() {
        finalize_failure_evidence(
            scenario,
            target,
            evidence_dir,
            &format!("missing required capabilities: {}", missing.join(", ")),
            None,
            None,
        );
        writer
            .record(EvidenceEventV1::scenario_finished(
                elapsed_ms(started),
                ScenarioOutcome::InfrastructureFailed,
            ))
            .map_err(|source| RunnerError::Evidence(source.to_string()))?;
        return Err(RunnerError::MissingCapabilities(missing.join(", ")));
    }
    let result = app
        .as_deref()
        .ok_or(RunnerError::MissingApplication)
        .and_then(|app| {
            dispatch_scenario(
                scenario,
                app,
                fixture_bin.as_deref(),
                target,
                evidence_dir,
                started,
                &mut writer,
            )
        })
        .and_then(|()| {
            scenario_timeout(scenario, started)?;
            validate_required_evidence(scenario, target, evidence_dir)?;
            scenario_timeout(scenario, started).map(|_| ())
        });
    match result {
        Ok(()) => {
            writer
                .record(EvidenceEventV1::scenario_finished(
                    elapsed_ms(started),
                    ScenarioOutcome::Passed,
                ))
                .map_err(|source| RunnerError::Evidence(source.to_string()))?;
            Ok(())
        }
        Err(error) => {
            let (stdout, stderr) = error.diagnostic_streams();
            finalize_failure_evidence(
                scenario,
                target,
                evidence_dir,
                &error.to_string(),
                stdout,
                stderr,
            );
            writer
                .record(EvidenceEventV1::scenario_finished(
                    elapsed_ms(started),
                    ScenarioOutcome::Failed,
                ))
                .map_err(|source| RunnerError::Evidence(source.to_string()))?;
            Err(error)
        }
    }
}

fn dispatch_scenario(
    scenario: &ScenarioV1,
    app: &Path,
    fixture_bin: Option<&Path>,
    target: &str,
    evidence_dir: &Path,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<(), RunnerError> {
    if scenario.fixture == "hermetic_ssh" {
        execute_ssh_scenario(scenario, app, target, evidence_dir, started, writer)
    } else if scenario.fixture == "hermetic_ssh_transfer" {
        execute_transfer_scenario(scenario, app, target, evidence_dir, started, writer)
    } else if scenario.fixture == "profile_lifecycle" {
        execute_profile_lifecycle_scenario(
            scenario,
            app,
            fixture_bin.ok_or(RunnerError::MissingFixtureApplication)?,
            target,
            evidence_dir,
            started,
            writer,
        )
    } else if scenario.fixture == "observer_disconnect" {
        execute_observer_disconnect_scenario(
            scenario,
            app,
            fixture_bin.ok_or(RunnerError::MissingFixtureApplication)?,
            target,
            evidence_dir,
            started,
            writer,
        )
    } else if scenario.fixture == "config_lifecycle" {
        execute_config_lifecycle_scenario(
            scenario,
            app,
            fixture_bin.ok_or(RunnerError::MissingFixtureApplication)?,
            target,
            evidence_dir,
            started,
            writer,
        )
    } else if scenario.fixture == "pty_disconnect_reconnect" {
        execute_pty_disconnect_reconnect_scenario(
            scenario,
            app,
            fixture_bin.ok_or(RunnerError::MissingFixtureApplication)?,
            target,
            evidence_dir,
            started,
            writer,
        )
    } else {
        match scenario.surface {
            Surface::Console if scenario.fixture == "terminal_probe" => execute_pty_scenario(
                scenario,
                app,
                fixture_bin.ok_or(RunnerError::MissingFixtureApplication)?,
                target,
                evidence_dir,
                started,
                writer,
            ),
            Surface::Console if scenario.fixture == "terminal_stress" => {
                execute_pty_stress_scenario(
                    scenario,
                    app,
                    fixture_bin.ok_or(RunnerError::MissingFixtureApplication)?,
                    target,
                    evidence_dir,
                    started,
                    writer,
                )
            }
            Surface::Console | Surface::Package => {
                execute_process_scenario(scenario, app, target, evidence_dir, started, writer)
            }
            Surface::HostTerminal => execute_host_terminal_scenario(
                scenario,
                app,
                fixture_bin.ok_or(RunnerError::MissingFixtureApplication)?,
                target,
                evidence_dir,
                started,
                writer,
            ),
            Surface::NativeWindow => execute_native_window_scenario(
                scenario,
                app,
                fixture_bin.ok_or(RunnerError::MissingFixtureApplication)?,
                target,
                evidence_dir,
                started,
                writer,
            ),
            Surface::Tauri => {
                execute_tauri_window_scenario(scenario, app, target, evidence_dir, started, writer)
            }
            surface @ Surface::Web => Err(RunnerError::NoDriver(surface)),
        }
    }
}

fn execute_pty_disconnect_reconnect_scenario(
    scenario: &ScenarioV1,
    app: &Path,
    fixture_bin: &Path,
    target: &str,
    evidence_dir: &Path,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<(), RunnerError> {
    let context = ScenarioRunContext {
        scenario,
        target,
        evidence_dir,
        started,
    };
    let mut driver = PtyReconnectDriver::start(context, app, fixture_bin)?;
    record_driver_behavior(
        scenario,
        started,
        writer,
        "BHV-CMD-LOCAL",
        "first PTY generation entered through the real local command",
    )?;

    for (index, action) in scenario.actions.iter().enumerate() {
        driver.apply(action, writer)?;
        writer
            .record(EvidenceEventV1::action_finished(
                elapsed_ms(started),
                index,
                action_name(action),
                "completed by the two-generation PTY driver",
            ))
            .map_err(|source| RunnerError::Evidence(source.to_string()))?;
        record_action_behavior(scenario, started, writer, action)?;
    }
    driver.finish(writer)
}

struct PtyReconnectDriver<'a> {
    context: ScenarioRunContext<'a>,
    app: &'a Path,
    fixture_bin: &'a Path,
    generation: Option<crate::PtyFixtureDriver>,
    disconnected: Option<crate::PtyFixtureResult>,
}

impl<'a> PtyReconnectDriver<'a> {
    fn start(
        context: ScenarioRunContext<'a>,
        app: &'a Path,
        fixture_bin: &'a Path,
    ) -> Result<Self, RunnerError> {
        let generation = crate::PtyFixtureDriver::spawn_with_args(
            app,
            Self::local_args(fixture_bin, "hold-open"),
            80,
            24,
            scenario_timeout(context.scenario, context.started)?,
        )
        .and_then(|driver| driver.wait_for_output(b"fixture-hold-open"))
        .map_err(|source| RunnerError::Driver(source.to_string()))?;
        Ok(Self {
            context,
            app,
            fixture_bin,
            generation: Some(generation),
            disconnected: None,
        })
    }

    fn local_args(fixture_bin: &Path, mode: &str) -> [String; 4] {
        [
            "local".to_owned(),
            "--".to_owned(),
            fixture_bin.to_string_lossy().into_owned(),
            mode.to_owned(),
        ]
    }

    fn apply(
        &mut self,
        action: &ActionV1,
        writer: &mut EvidenceWriter<fs::File>,
    ) -> Result<(), RunnerError> {
        match action {
            ActionV1::FixtureDisconnect { .. } => self.disconnect(writer),
            ActionV1::FixtureReconnect { .. } => self.reconnect(writer),
            ActionV1::PtyInput { bytes_hex } => self.write(bytes_hex),
            ActionV1::Finish => Ok(()),
            action => Err(RunnerError::Driver(format!(
                "disconnect/reconnect driver does not support action {action:?}"
            ))),
        }
    }

    fn disconnect(&mut self, writer: &mut EvidenceWriter<fs::File>) -> Result<(), RunnerError> {
        let result = self
            .generation
            .take()
            .ok_or_else(|| RunnerError::Scenario("fixture is already disconnected".to_owned()))?
            .cap_remaining_timeout(cleanup_timeout(
                self.context.scenario,
                self.context.started,
            )?)
            .disconnect()
            .map_err(|source| RunnerError::Driver(source.to_string()))?;
        if !result.resources_zero() {
            return Err(RunnerError::Scenario(
                "disconnected PTY generation retained resources".to_owned(),
            ));
        }
        self.disconnected = Some(result);
        record_driver_behavior(
            self.context.scenario,
            self.context.started,
            writer,
            "BHV-LIFECYCLE-DISCONNECTED",
            "first PTY generation was interrupted and fully reaped",
        )
    }

    fn reconnect(&mut self, writer: &mut EvidenceWriter<fs::File>) -> Result<(), RunnerError> {
        if self.generation.is_some() || self.disconnected.is_none() {
            return Err(RunnerError::Scenario(
                "fixture reconnect requires one completed disconnect".to_owned(),
            ));
        }
        self.generation = Some(
            crate::PtyFixtureDriver::spawn_with_args(
                self.app,
                Self::local_args(self.fixture_bin, "echo-query"),
                80,
                24,
                scenario_timeout(self.context.scenario, self.context.started)?,
            )
            .and_then(|driver| driver.wait_for_output(b"fixture-ready"))
            .map_err(|source| RunnerError::Driver(source.to_string()))?,
        );
        record_driver_behavior(
            self.context.scenario,
            self.context.started,
            writer,
            "BHV-LIFECYCLE-RECONNECTED",
            "second PTY generation reached its independent ready marker",
        )
    }

    fn write(&mut self, bytes_hex: &str) -> Result<(), RunnerError> {
        let bytes = decode_hex(bytes_hex).map_err(RunnerError::Driver)?;
        self.generation = Some(
            self.generation
                .take()
                .ok_or_else(|| {
                    RunnerError::Scenario("PTY input requires a connected fixture".to_owned())
                })?
                .write(&bytes)
                .map_err(|source| RunnerError::Driver(source.to_string()))?,
        );
        Ok(())
    }

    fn finish(mut self, writer: &mut EvidenceWriter<fs::File>) -> Result<(), RunnerError> {
        let reconnected = self
            .generation
            .take()
            .ok_or_else(|| {
                RunnerError::Scenario("scenario finished without a reconnected fixture".to_owned())
            })?
            .cap_remaining_timeout(cleanup_timeout(
                self.context.scenario,
                self.context.started,
            )?)
            .finish()
            .map_err(|source| RunnerError::Driver(source.to_string()))?;
        let disconnected = self.disconnected.ok_or_else(|| {
            RunnerError::Scenario("scenario never disconnected its first fixture".to_owned())
        })?;
        write_pty_reconnect_evidence(self.context, &disconnected, &reconnected)?;
        let projection = ChildOutputProjection {
            stdout: &reconnected.output,
            stderr: &[],
            exit_code: i32::try_from(reconnected.exit_code).unwrap_or(i32::MAX),
            resources_zero: disconnected.resources_zero() && reconnected.resources_zero(),
        };
        finish_projected_checkpoints(
            self.context.scenario,
            self.context.started,
            writer,
            &projection,
        )
    }
}

fn write_pty_reconnect_evidence(
    context: ScenarioRunContext<'_>,
    disconnected: &crate::PtyFixtureResult,
    reconnected: &crate::PtyFixtureResult,
) -> Result<(), RunnerError> {
    let stem = format!("{}.{}.0", context.scenario.id, context.target);
    let mut output = disconnected.output.clone();
    output.extend_from_slice(&reconnected.output);
    fs::write(context.evidence_dir.join(format!("{stem}.stdout")), &output)
        .map_err(RunnerError::Io)?;
    fs::write(context.evidence_dir.join(format!("{stem}.stderr")), b"").map_err(RunnerError::Io)?;
    fs::write(
        context
            .evidence_dir
            .join(format!("{stem}.final-snapshot.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": 1,
            "generations": 2,
            "terminal_text": String::from_utf8_lossy(&reconnected.output),
        }))
        .map_err(RunnerError::Json)?,
    )
    .map_err(RunnerError::Io)?;
    let resources_zero = disconnected.resources_zero() && reconnected.resources_zero();
    fs::write(
        context
            .evidence_dir
            .join(format!("{stem}.process-tree.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": 1,
            "subprocesses": 2,
            "reaped": resources_zero,
            "reader_joined": resources_zero,
            "master_closed": resources_zero,
            "remaining_owned_processes": u8::from(!resources_zero),
        }))
        .map_err(RunnerError::Json)?,
    )
    .map_err(RunnerError::Io)
}

fn execute_config_lifecycle_scenario(
    scenario: &ScenarioV1,
    app: &Path,
    fixture_bin: &Path,
    target: &str,
    evidence_dir: &Path,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<(), RunnerError> {
    let config_path = absolute_from_current(
        &evidence_dir.join(format!("{}.{}.wezterm.lua", scenario.id, target)),
    )?;
    write_config_fixture(
        &config_path,
        "return { automatically_reload_config = true, term = 'initial' }\n",
    )?;
    let mut command = hermetic_app_command(app);
    command.args([
        "--config-file",
        config_path.to_string_lossy().as_ref(),
        "window",
        "--class",
        FUNCTIONAL_X11_WINDOW_CLASS,
        "--",
        fixture_bin.to_string_lossy().as_ref(),
        "window-effects",
    ]);
    let endpoint = evidence_dir.join(format!("{}.{}.observer", scenario.id, target));
    let token = crate::ObserverToken::generate();
    command
        .env("RTERM_FUNCTIONAL_OBSERVER_ENDPOINT", endpoint.as_os_str())
        .env(
            "RTERM_FUNCTIONAL_OBSERVER_TOKEN",
            token.expose_for_child_process(),
        );
    let mut child = ChildGuard::spawn(command, scenario_timeout(scenario, started)?)
        .map_err(|source| RunnerError::Driver(format!("launch config window: {source}")))?;
    let process_id = child
        .process_id()
        .ok_or_else(|| RunnerError::Driver("config window has no process id".to_owned()))?;
    let mut client = connect_observer(&endpoint, &token, startup_timeout(scenario, started)?)?;
    let initial = wait_for_config_state(&mut client, 1, false, action_timeout(scenario, started)?)?;
    let initial = wait_for_native_fixture_ready(scenario, &mut client, initial, started)?;
    record_driver_behavior(
        scenario,
        started,
        writer,
        "BHV-CMD-WINDOW",
        "window command loaded an explicit configuration fixture",
    )?;
    record_driver_behavior(
        scenario,
        started,
        writer,
        "BHV-LIFECYCLE-STARTED",
        "configured window reached its authenticated observer",
    )?;
    record_driver_behavior(
        scenario,
        started,
        writer,
        "BHV-CONFIG-PRECEDENCE",
        "explicit --config-file was selected at generation 1",
    )?;

    let mut snapshot = exercise_config_reload(
        &config_path,
        &mut client,
        initial.revision,
        scenario,
        started,
        writer,
    )?;

    run_config_window_actions(
        ScenarioRunContext {
            scenario,
            target,
            evidence_dir,
            started,
        },
        process_id,
        &mut client,
        &mut snapshot,
        writer,
    )?;
    snapshot = wait_for_live_checkpoints(
        scenario,
        &mut client,
        snapshot,
        action_timeout(scenario, started)?,
    )?;
    child.cap_remaining_timeout(cleanup_timeout(scenario, started)?);
    let output = child
        .wait()
        .map_err(|source| RunnerError::child("wait for config window", source))?;
    finish_observed_window_evidence(
        scenario,
        target,
        evidence_dir,
        started,
        writer,
        &ObservedWindowResult {
            process_id,
            snapshot: &snapshot,
            stdout: &output.stdout,
            stderr: &output.stderr,
            exit_code: output.status.code(),
        },
    )
}

fn run_config_window_actions(
    context: ScenarioRunContext<'_>,
    process_id: u32,
    client: &mut crate::ObserverClient,
    snapshot: &mut crate::ObserverSnapshotV1,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<(), RunnerError> {
    let options = PlatformDriverOptions::x11_class(FUNCTIONAL_X11_WINDOW_CLASS);
    let platform = platform_driver(context.target, process_id, options)?;
    for (action_index, action) in context.scenario.actions.iter().enumerate() {
        if matches!(action, ActionV1::Finish) {
            writer
                .record(EvidenceEventV1::action_finished(
                    elapsed_ms(context.started),
                    action_index,
                    "finish",
                    "waiting_for_config_window_exit",
                ))
                .map_err(|source| RunnerError::Evidence(source.to_string()))?;
        } else {
            platform
                .execute(action, action_timeout(context.scenario, context.started)?)
                .map_err(|source| RunnerError::Driver(source.to_string()))?;
            if action_must_publish(action) {
                *snapshot = wait_for_observer_change(
                    client,
                    snapshot.revision,
                    action_timeout(context.scenario, context.started)?,
                )?;
            } else if let Ok(observed) = client.snapshot() {
                *snapshot = observed;
            }
            writer
                .record(EvidenceEventV1::action_finished(
                    elapsed_ms(context.started),
                    action_index,
                    action_name(action),
                    &format!("observer_revision={}", snapshot.revision),
                ))
                .map_err(|source| RunnerError::Evidence(source.to_string()))?;
        }
        record_action_behavior(context.scenario, context.started, writer, action)?;
    }
    Ok(())
}

fn exercise_config_reload(
    config_path: &Path,
    client: &mut crate::ObserverClient,
    initial_revision: u64,
    scenario: &ScenarioV1,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<crate::ObserverSnapshotV1, RunnerError> {
    write_config_fixture(
        config_path,
        "return { automatically_reload_config = true, term = 'valid-reload' }\n",
    )?;
    let valid = wait_for_config_state_after(
        client,
        initial_revision,
        2,
        false,
        action_timeout(scenario, started)?,
    )?;
    record_driver_behavior(
        scenario,
        started,
        writer,
        "BHV-CONFIG-HOT-RELOAD",
        "valid watcher reload advanced generation 1 to 2",
    )?;

    write_config_fixture(config_path, "return { term = }\n")?;
    let invalid = wait_for_config_state_after(
        client,
        valid.revision,
        2,
        true,
        action_timeout(scenario, started)?,
    )?;
    write_config_fixture(
        config_path,
        "return { automatically_reload_config = true, term = 'recovered' }\n",
    )?;
    let snapshot = wait_for_config_state_after(
        client,
        invalid.revision,
        3,
        false,
        action_timeout(scenario, started)?,
    )?;
    record_driver_behavior(
        scenario,
        started,
        writer,
        "BHV-CONFIG-LAST-KNOWN-GOOD",
        "invalid reload retained generation 2 and recovery advanced to 3",
    )?;

    Ok(snapshot)
}

fn write_config_fixture(path: &std::path::Path, contents: &str) -> Result<(), RunnerError> {
    fs::write(path, contents).map_err(RunnerError::Io)
}

struct ObservedWindowResult<'a> {
    process_id: u32,
    snapshot: &'a crate::ObserverSnapshotV1,
    stdout: &'a [u8],
    stderr: &'a [u8],
    exit_code: Option<i32>,
}

#[derive(Clone, Copy)]
struct ScenarioRunContext<'a> {
    scenario: &'a ScenarioV1,
    target: &'a str,
    evidence_dir: &'a Path,
    started: Instant,
}

fn finish_observed_window_evidence(
    scenario: &ScenarioV1,
    target: &str,
    evidence_dir: &std::path::Path,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
    result: &ObservedWindowResult<'_>,
) -> Result<(), RunnerError> {
    write_process_evidence(
        scenario,
        target,
        evidence_dir,
        &[result.process_id],
        result.stdout,
        result.stderr,
    )?;
    let stem = format!("{}.{target}.0", scenario.id);
    fs::write(
        evidence_dir.join(format!("{stem}.final-snapshot.json")),
        serde_json::to_vec_pretty(result.snapshot).map_err(RunnerError::Json)?,
    )
    .map_err(RunnerError::Io)?;
    if scenario
        .required_evidence
        .contains(&crate::EvidenceKind::ServerTrace)
    {
        fs::write(
            evidence_dir.join(format!("{stem}.server-trace.json")),
            serde_json::to_vec_pretty(&result.snapshot.runtime.effects)
                .map_err(RunnerError::Json)?,
        )
        .map_err(RunnerError::Io)?;
    }
    let context = CheckpointContext {
        snapshot: Some(result.snapshot),
        stdout: result.stdout,
        stderr: result.stderr,
        exit_code: result.exit_code,
        resources_zero: observed_resources_zero(result.snapshot),
        artifact_root: Some(evidence_dir),
        network_bytes: std::collections::BTreeMap::new(),
    };
    finish_checkpoints(scenario, started, writer, &context)
}

fn wait_for_config_state(
    client: &mut crate::ObserverClient,
    generation: u64,
    diagnostic_present: bool,
    timeout: Duration,
) -> Result<crate::ObserverSnapshotV1, RunnerError> {
    let snapshot = client.snapshot().map_err(|source| {
        RunnerError::Driver(format!("read config observer snapshot: {source}"))
    })?;
    wait_for_config_state_after(
        client,
        snapshot.revision.saturating_sub(1),
        generation,
        diagnostic_present,
        timeout,
    )
}

fn wait_for_config_state_after(
    client: &mut crate::ObserverClient,
    after_revision: u64,
    generation: u64,
    diagnostic_present: bool,
    timeout: Duration,
) -> Result<crate::ObserverSnapshotV1, RunnerError> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| RunnerError::Driver("config observer deadline overflow".to_owned()))?;
    let mut revision = after_revision;
    while Instant::now() < deadline {
        let snapshot = client.snapshot().map_err(|source| {
            RunnerError::Driver(format!("read config observer snapshot: {source}"))
        })?;
        revision = revision.max(snapshot.revision);
        if snapshot.config_generation == generation
            && snapshot.config_diagnostic_present == diagnostic_present
        {
            return Ok(snapshot);
        }
        if let Some(snapshot) = client
            .subscribe(revision)
            .map_err(|source| RunnerError::Driver(format!("subscribe config observer: {source}")))?
        {
            revision = snapshot.revision;
            if snapshot.config_generation == generation
                && snapshot.config_diagnostic_present == diagnostic_present
            {
                return Ok(snapshot);
            }
        }
    }
    Err(RunnerError::Scenario(format!(
        "config state did not become generation={generation} diagnostic={diagnostic_present}"
    )))
}

fn execute_observer_disconnect_scenario(
    scenario: &ScenarioV1,
    app: &Path,
    fixture_bin: &Path,
    target: &str,
    evidence_dir: &Path,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<(), RunnerError> {
    let endpoint = evidence_dir.join(format!("{}.{}.observer", scenario.id, target));
    let token = crate::ObserverToken::generate();
    let mut command = hermetic_app_command(app);
    command
        .args([
            "window",
            "--class",
            FUNCTIONAL_X11_WINDOW_CLASS,
            "--",
            fixture_bin.to_string_lossy().as_ref(),
            "window-effects",
        ])
        .env("RTERM_FUNCTIONAL_OBSERVER_ENDPOINT", endpoint.as_os_str())
        .env(
            "RTERM_FUNCTIONAL_OBSERVER_TOKEN",
            token.expose_for_child_process(),
        );
    let timeout = scenario_timeout(scenario, started)?;
    let mut child = ChildGuard::spawn(command, timeout).map_err(|source| {
        RunnerError::Driver(format!("launch observer disconnect fixture: {source}"))
    })?;
    let process_id = child
        .process_id()
        .ok_or_else(|| RunnerError::Driver("observer disconnect process has no id".to_owned()))?;
    let mut client = connect_observer(&endpoint, &token, startup_timeout(scenario, started)?)?;
    let initial = client
        .snapshot()
        .map_err(|source| RunnerError::Driver(format!("read disconnect snapshot: {source}")))?;
    let _initial = wait_for_native_fixture_ready(scenario, &mut client, initial, started)?;
    record_driver_behavior(
        scenario,
        started,
        writer,
        "BHV-CMD-WINDOW",
        "native window command reached its authenticated observer",
    )?;
    drop(client);
    record_driver_behavior(
        scenario,
        started,
        writer,
        "BHV-OBSERVER-DISCONNECT",
        "observer channel intentionally disconnected after authenticated read",
    )?;
    let options = PlatformDriverOptions::x11_class(FUNCTIONAL_X11_WINDOW_CLASS);
    let platform = platform_driver(target, process_id, options)?;
    for (action_index, action) in scenario.actions.iter().enumerate() {
        if matches!(action, ActionV1::Finish) {
            writer
                .record(EvidenceEventV1::action_finished(
                    elapsed_ms(started),
                    action_index,
                    "finish",
                    "waiting after observer disconnect",
                ))
                .map_err(|source| RunnerError::Evidence(source.to_string()))?;
        } else {
            platform
                .execute(action, action_timeout(scenario, started)?)
                .map_err(|source| RunnerError::Driver(source.to_string()))?;
            writer
                .record(EvidenceEventV1::action_finished(
                    elapsed_ms(started),
                    action_index,
                    action_name(action),
                    "completed after observer disconnect",
                ))
                .map_err(|source| RunnerError::Evidence(source.to_string()))?;
        }
        record_action_behavior(scenario, started, writer, action)?;
    }
    child.cap_remaining_timeout(cleanup_timeout(scenario, started)?);
    let output = child
        .wait()
        .map_err(|source| RunnerError::child("wait after observer disconnect", source))?;
    write_process_evidence(
        scenario,
        target,
        evidence_dir,
        &[process_id],
        &output.stdout,
        &output.stderr,
    )?;
    finish_projected_checkpoints(
        scenario,
        started,
        writer,
        &ChildOutputProjection {
            stdout: &output.stdout,
            stderr: &output.stderr,
            exit_code: output.status.code().unwrap_or(-1),
            resources_zero: true,
        },
    )
}

fn execute_host_terminal_scenario(
    scenario: &ScenarioV1,
    app: &Path,
    fixture_bin: &Path,
    target: &str,
    evidence_dir: &Path,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<(), RunnerError> {
    let mut launch =
        prepare_host_terminal_launch(scenario, app, fixture_bin, target, evidence_dir)?;
    let child = launch
        .command
        .spawn()
        .map_err(|source| RunnerError::Driver(format!("launch host terminal: {source}")))?;
    let child = HostTerminalChildGuard::new(child);
    let emulator_pid = child.process_id();
    let input_pid = if target.contains("macos") {
        macos_terminal_process_id(startup_timeout(scenario, started)?)?
    } else {
        emulator_pid
    };
    let window_title = target
        .contains("windows")
        .then_some(launch.host_title.as_str());
    let options = PlatformDriverOptions::window_title(window_title);
    let platform = platform_driver(target, input_pid, options)?;
    if target.contains("macos") {
        let setup = launch
            .command_arguments
            .iter()
            .map(|argument| shell_quote(argument))
            .collect::<Vec<_>>()
            .join(" ");
        platform
            .execute(
                &ActionV1::TypeText { text: setup },
                action_timeout(scenario, started)?,
            )
            .map_err(|source| RunnerError::Driver(source.to_string()))?;
        platform
            .execute(
                &ActionV1::Key {
                    key: "Enter".to_owned(),
                    modifiers: Vec::new(),
                },
                action_timeout(scenario, started)?,
            )
            .map_err(|source| RunnerError::Driver(source.to_string()))?;
    }
    record_driver_behavior(
        scenario,
        started,
        writer,
        "BHV-CMD-LOCAL",
        "local command started inside a real host terminal",
    )?;
    record_driver_behavior(
        scenario,
        started,
        writer,
        "BHV-LIFECYCLE-STARTED",
        "host terminal exposed an OS-input surface",
    )?;
    wait_for_host_terminal_readiness(&launch.marker, scenario, started)?;
    run_host_terminal_actions(scenario, &platform, started, writer)?;
    wait_for_marker(
        &launch.marker,
        "host-terminal-input-ok",
        action_timeout(scenario, started)?,
    )?;
    if target.contains("macos") {
        platform
            .execute(
                &ActionV1::WindowControl {
                    operation: crate::WindowControl::Close,
                },
                action_timeout(scenario, started)?,
            )
            .map_err(|source| RunnerError::Driver(source.to_string()))?;
    }
    let status = child.wait(cleanup_timeout(scenario, started)?)?;
    let marker_text = fs::read_to_string(&launch.marker)
        .map_err(|source| RunnerError::Driver(format!("read host terminal marker: {source}")))?;
    if marker_text != "host-terminal-input-ok" {
        return Err(RunnerError::Scenario(format!(
            "host terminal marker was {marker_text:?}"
        )));
    }
    record_driver_behavior(
        scenario,
        started,
        writer,
        "BHV-HOST-TERMINAL",
        "host-terminal-input-ok marker was produced only after emulator input",
    )?;
    write_process_evidence(scenario, target, evidence_dir, &[emulator_pid], &[], &[])?;
    finish_projected_checkpoints(
        scenario,
        started,
        writer,
        &ChildOutputProjection {
            stdout: &[],
            stderr: &[],
            exit_code: status.code().unwrap_or(-1),
            resources_zero: true,
        },
    )
}

fn wait_for_host_terminal_readiness(
    marker: &Path,
    scenario: &ScenarioV1,
    started: Instant,
) -> Result<(), RunnerError> {
    wait_for_marker(
        marker,
        "host-terminal-ready",
        startup_timeout(scenario, started)?,
    )
}

struct HostTerminalLaunch {
    command: Command,
    command_arguments: Vec<String>,
    marker: PathBuf,
    host_title: String,
}

fn prepare_host_terminal_launch(
    scenario: &ScenarioV1,
    app: &Path,
    fixture_bin: &Path,
    target: &str,
    evidence_dir: &Path,
) -> Result<HostTerminalLaunch, RunnerError> {
    let app = absolute_from_current(app)?;
    let fixture_bin = absolute_from_current(fixture_bin)?;
    let marker = absolute_from_current(
        &evidence_dir.join(format!("{}.{}.host-terminal-marker", scenario.id, target)),
    )?;
    let _ = fs::remove_file(&marker);
    let host_title = format!("RTERM-{:016x}", stable_path_hash(&marker));
    if target.contains("windows") {
        ensure_no_matching_windows_title(&host_title)?;
    }
    let emulator = std::env::var("RTERM_FUNCTIONAL_HOST_TERMINAL").map_err(|_| {
        RunnerError::Driver("RTERM_FUNCTIONAL_HOST_TERMINAL is required".to_owned())
    })?;
    let command_arguments = vec![
        app.to_string_lossy().into_owned(),
        "local".to_owned(),
        "--".to_owned(),
        fixture_bin.to_string_lossy().into_owned(),
        "host-terminal-probe".to_owned(),
        marker.to_string_lossy().into_owned(),
        host_title.clone(),
    ];
    let mut command = Command::new(&emulator);
    if target.contains("windows") {
        #[cfg(windows)]
        {
            command = Command::new("cmd.exe");
            command
                .args(["/d", "/c", "start", "", "/wait"])
                .args(&command_arguments);
        }
        #[cfg(not(windows))]
        command.args(&command_arguments);
    } else if target.contains("xterm") {
        command
            .args(["-T", "R-Term Functional Host Terminal", "-e"])
            .args(&command_arguments);
    } else if target.contains("foot") {
        command
            .args(["--title", "R-Term Functional Host Terminal"])
            .args(&command_arguments);
    } else if target.contains("macos") {
        command.args(["-n", "-W", "-a", "Terminal"]);
    } else {
        return Err(RunnerError::Driver(format!(
            "target {target:?} has no host terminal adapter"
        )));
    }
    apply_loopback_only_environment(&mut command);
    Ok(HostTerminalLaunch {
        command,
        command_arguments,
        marker,
        host_title,
    })
}

fn run_host_terminal_actions(
    scenario: &ScenarioV1,
    platform: &crate::PlatformInputDriver,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<(), RunnerError> {
    for (index, action) in scenario.actions.iter().enumerate() {
        let result = if matches!(action, ActionV1::Finish) {
            "waiting for host terminal marker and exit"
        } else {
            platform
                .execute(action, action_timeout(scenario, started)?)
                .map_err(|source| RunnerError::Driver(source.to_string()))?;
            "delivered through host terminal OS input"
        };
        writer
            .record(EvidenceEventV1::action_finished(
                elapsed_ms(started),
                index,
                action_name(action),
                result,
            ))
            .map_err(|source| RunnerError::Evidence(source.to_string()))?;
        record_action_behavior(scenario, started, writer, action)?;
    }
    Ok(())
}

struct HostTerminalChildGuard {
    child: Option<Child>,
    process_id: u32,
}

impl HostTerminalChildGuard {
    fn new(child: Child) -> Self {
        let process_id = child.id();
        Self {
            child: Some(child),
            process_id,
        }
    }

    fn process_id(&self) -> u32 {
        self.process_id
    }

    fn wait(mut self, timeout: Duration) -> Result<ExitStatus, RunnerError> {
        let status = wait_host_terminal_child(
            self.child
                .as_mut()
                .expect("host terminal guard owns a child until wait completes"),
            timeout,
        )?;
        self.child.take();
        Ok(status)
    }

    fn terminate_tree(&mut self) {
        let Some(child) = self.child.as_mut() else {
            return;
        };
        if matches!(child.try_wait(), Ok(Some(_))) {
            self.child.take();
            return;
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;

            let _ = Command::new("taskkill.exe")
                .args(["/PID", &self.process_id.to_string(), "/T", "/F"])
                .creation_flags(0x0800_0000)
                .status();
        }
        #[cfg(not(windows))]
        let _ = child.kill();
        let _ = child.wait();
        self.child.take();
    }
}

impl Drop for HostTerminalChildGuard {
    fn drop(&mut self) {
        self.terminate_tree();
    }
}

fn wait_host_terminal_child(
    child: &mut Child,
    timeout: Duration,
) -> Result<ExitStatus, RunnerError> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| RunnerError::Driver("host terminal deadline overflow".to_owned()))?;
    loop {
        if let Some(status) = child.try_wait().map_err(RunnerError::Io)? {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            return Err(RunnerError::Driver(format!(
                "host terminal exceeded {timeout:?}"
            )));
        }
        std::thread::yield_now();
    }
}

fn wait_for_marker(
    path: &std::path::Path,
    expected: &str,
    timeout: Duration,
) -> Result<(), RunnerError> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| RunnerError::Driver("host terminal marker deadline overflow".to_owned()))?;
    let mut last = None;
    while Instant::now() < deadline {
        match fs::read_to_string(path) {
            Ok(value) if value == expected => return Ok(()),
            Ok(value) => last = Some(value),
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) => return Err(RunnerError::Io(source)),
        }
        std::thread::park_timeout(Duration::from_millis(10));
    }
    Err(RunnerError::Scenario(format!(
        "host terminal marker did not become {expected:?}; last={last:?}"
    )))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn stable_path_hash(path: &std::path::Path) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    path.to_string_lossy().bytes().fold(OFFSET, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(PRIME)
    })
}

fn absolute_from_current(path: &std::path::Path) -> Result<PathBuf, RunnerError> {
    if path.is_absolute() {
        return Ok(path.to_owned());
    }
    std::env::current_dir()
        .map(|current| current.join(path))
        .map_err(RunnerError::Io)
}

fn ensure_no_matching_windows_title(fragment: &str) -> Result<(), RunnerError> {
    let escaped = fragment.replace('\'', "''");
    let script = format!(
        "$match = Get-Process | Where-Object {{ $_.MainWindowTitle -like '*{escaped}*' }} | Select-Object -First 1; if ($match) {{ $match.Id }}"
    );
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map_err(RunnerError::Io)?;
    if !output.status.success() {
        return Err(RunnerError::Driver(format!(
            "inspect Windows host terminal title: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    if output.stdout.iter().any(|byte| !byte.is_ascii_whitespace()) {
        return Err(RunnerError::Driver(format!(
            "a pre-existing host terminal title already contains {fragment:?}; refusing ambiguous OS input"
        )));
    }
    Ok(())
}

fn macos_terminal_process_id(timeout: Duration) -> Result<u32, RunnerError> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| RunnerError::Driver("macOS Terminal deadline overflow".to_owned()))?;
    while Instant::now() < deadline {
        if let Ok(output) = Command::new("pgrep")
            .args(["-n", "-x", "Terminal"])
            .output()
            && output.status.success()
            && let Some(pid) = String::from_utf8_lossy(&output.stdout)
                .lines()
                .find_map(|line| line.trim().parse().ok())
        {
            return Ok(pid);
        }
        std::thread::park_timeout(Duration::from_millis(10));
    }
    Err(RunnerError::Driver(
        "Terminal.app did not expose a process before the startup deadline".to_owned(),
    ))
}

fn execute_ssh_scenario(
    scenario: &ScenarioV1,
    app: &Path,
    target: &str,
    evidence_dir: &Path,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<(), RunnerError> {
    record_finish_only_actions(scenario, started, writer)?;
    let result = crate::run_ssh_loopback_journey(
        app,
        scenario_timeout(scenario, started)?,
        Duration::from_millis(scenario.deadlines.cleanup_ms),
    )
    .map_err(|source| RunnerError::Driver(source.to_string()))?;
    record_driver_behavior(
        scenario,
        started,
        writer,
        "BHV-CMD-SSH",
        "native and system SSH commands completed",
    )?;
    record_driver_behavior(
        scenario,
        started,
        writer,
        "BHV-SSH-LOOPBACK",
        "loopback shell and -L/-D/-R forwarding echoed exact bytes",
    )?;
    record_driver_behavior(
        scenario,
        started,
        writer,
        "BHV-LIFECYCLE-CONNECTED",
        "hermetic SSH server accepted authenticated sessions",
    )?;
    let stdout = format!("{}\n{}\n", result.native_stdout, result.system_stdout);
    write_transport_evidence(
        scenario,
        target,
        evidence_dir,
        stdout.as_bytes(),
        &result.server_trace,
        result.resources_zero,
    )?;
    let context = CheckpointContext {
        snapshot: None,
        stdout: stdout.as_bytes(),
        stderr: &[],
        exit_code: Some(0),
        resources_zero: result.resources_zero,
        artifact_root: Some(evidence_dir),
        network_bytes: std::collections::BTreeMap::new(),
    };
    finish_checkpoints(scenario, started, writer, &context)
}

fn execute_transfer_scenario(
    scenario: &ScenarioV1,
    app: &Path,
    target: &str,
    evidence_dir: &Path,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<(), RunnerError> {
    record_finish_only_actions(scenario, started, writer)?;
    let result = crate::run_transfer_roundtrip_journey(
        app,
        scenario_timeout(scenario, started)?,
        Duration::from_millis(scenario.deadlines.cleanup_ms),
    )
    .map_err(|source| RunnerError::Driver(source.to_string()))?;
    for (behavior, evidence) in [
        ("BHV-CMD-SFTP", "SFTP upload and download completed"),
        ("BHV-CMD-SCP", "SCP upload and download completed"),
        (
            "BHV-TRANSFER-ROUNDTRIP",
            "SFTP and SCP downloads matched the source SHA-256",
        ),
    ] {
        record_driver_behavior(scenario, started, writer, behavior, evidence)?;
    }
    fs::write(
        evidence_dir.join("sftp-download.bin"),
        &result.sftp_download,
    )
    .map_err(RunnerError::Io)?;
    fs::write(evidence_dir.join("scp-download.bin"), &result.scp_download)
        .map_err(RunnerError::Io)?;
    write_transport_evidence(
        scenario,
        target,
        evidence_dir,
        b"",
        &result.server_trace,
        result.resources_zero,
    )?;
    let context = CheckpointContext {
        snapshot: None,
        stdout: &[],
        stderr: &[],
        exit_code: Some(0),
        resources_zero: result.resources_zero,
        artifact_root: Some(evidence_dir),
        network_bytes: std::collections::BTreeMap::new(),
    };
    finish_checkpoints(scenario, started, writer, &context)
}

fn record_finish_only_actions(
    scenario: &ScenarioV1,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<(), RunnerError> {
    for (index, action) in scenario.actions.iter().enumerate() {
        if !matches!(action, ActionV1::Finish) {
            return Err(RunnerError::Driver(format!(
                "transport preset only accepts finish; observed {action:?}"
            )));
        }
        writer
            .record(EvidenceEventV1::action_finished(
                elapsed_ms(started),
                index,
                "finish",
                "transport_fixture_complete",
            ))
            .map_err(|source| RunnerError::Evidence(source.to_string()))?;
        record_action_behavior(scenario, started, writer, action)?;
    }
    Ok(())
}

fn write_transport_evidence(
    scenario: &ScenarioV1,
    target: &str,
    evidence_dir: &Path,
    stdout: &[u8],
    server_trace: &[String],
    resources_zero: bool,
) -> Result<(), RunnerError> {
    let stem = format!("{}.{target}.0", scenario.id);
    fs::write(evidence_dir.join(format!("{stem}.stdout")), stdout).map_err(RunnerError::Io)?;
    fs::write(evidence_dir.join(format!("{stem}.stderr")), b"").map_err(RunnerError::Io)?;
    fs::write(
        evidence_dir.join(format!("{stem}.server-trace.json")),
        serde_json::to_vec_pretty(server_trace).map_err(RunnerError::Json)?,
    )
    .map_err(RunnerError::Io)?;
    fs::write(
        evidence_dir.join(format!("{stem}.process-tree.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": 1,
            "root_process_id": null,
            "remaining_owned_processes": u8::from(!resources_zero),
            "fixture_tasks_zero": resources_zero,
        }))
        .map_err(RunnerError::Json)?,
    )
    .map_err(RunnerError::Io)
}

fn execute_native_window_scenario(
    scenario: &ScenarioV1,
    app: &Path,
    fixture_bin: &Path,
    target: &str,
    evidence_dir: &Path,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<(), RunnerError> {
    let (command, _scenario_config) = native_window_command(&scenario.fixture, app, fixture_bin)?;
    execute_observed_window_scenario(
        scenario,
        command,
        target,
        evidence_dir,
        started,
        writer,
        "native window",
    )
}

fn native_window_command(
    fixture: &str,
    app: &Path,
    fixture_bin: &Path,
) -> Result<(Command, Option<tempfile::NamedTempFile>), RunnerError> {
    let mut command = hermetic_app_command(app);
    let mut scenario_config = tempfile::Builder::new()
        .prefix("rterm-functional-window-")
        .suffix(".lua")
        .tempfile()
        .map_err(RunnerError::Io)?;
    let config_contents = if fixture == "forced_close" {
        b"return { window_close_confirmation = 'NeverPrompt' }\n".as_slice()
    } else {
        b"return {}\n".as_slice()
    };
    scenario_config
        .write_all(config_contents)
        .map_err(RunnerError::Io)?;
    scenario_config.flush().map_err(RunnerError::Io)?;
    command.args([
        "--config-file",
        scenario_config.path().to_string_lossy().as_ref(),
    ]);
    let fixture_mode = if fixture == "forced_close" {
        "hold-open"
    } else {
        "window-effects"
    };
    command.args([
        "window",
        "--class",
        FUNCTIONAL_X11_WINDOW_CLASS,
        "--osc52",
        "write",
        "--",
        fixture_bin.to_string_lossy().as_ref(),
        fixture_mode,
    ]);
    Ok((command, Some(scenario_config)))
}

fn execute_tauri_window_scenario(
    scenario: &ScenarioV1,
    app: &Path,
    target: &str,
    evidence_dir: &Path,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<(), RunnerError> {
    execute_observed_window_scenario(
        scenario,
        hermetic_app_command(app),
        target,
        evidence_dir,
        started,
        writer,
        "Tauri window",
    )
}

fn execute_observed_window_scenario(
    scenario: &ScenarioV1,
    mut command: Command,
    target: &str,
    evidence_dir: &Path,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
    description: &str,
) -> Result<(), RunnerError> {
    let endpoint = evidence_dir.join(format!("{}.{}.observer", scenario.id, target));
    let token = crate::ObserverToken::generate();
    command
        .env("RTERM_FUNCTIONAL_OBSERVER_ENDPOINT", endpoint.as_os_str())
        .env(
            "RTERM_FUNCTIONAL_OBSERVER_TOKEN",
            token.expose_for_child_process(),
        );
    let timeout = scenario_timeout(scenario, started)?;
    let mut child = ChildGuard::spawn(command, timeout)
        .map_err(|source| RunnerError::Driver(format!("launch {description}: {source}")))?;
    let process_id = child
        .process_id()
        .ok_or_else(|| RunnerError::Driver("native window has no process id".to_owned()))?;
    let mut client = connect_observer(&endpoint, &token, startup_timeout(scenario, started)?)?;
    let mut snapshot = client.snapshot().map_err(|source| {
        RunnerError::Driver(format!("read initial observer snapshot: {source}"))
    })?;
    if scenario.surface == Surface::NativeWindow {
        record_driver_behavior(
            scenario,
            started,
            writer,
            "BHV-CMD-WINDOW",
            "native window command reached its authenticated observer",
        )?;
        record_driver_behavior(
            scenario,
            started,
            writer,
            "BHV-WINDOW-INTERACTION",
            "native window opened a real PTY-backed pane",
        )?;
    } else if scenario.surface == Surface::Tauri {
        record_driver_behavior(
            scenario,
            started,
            writer,
            "BHV-TAURI-PTY",
            "Tauri window reached its embedded PTY observer",
        )?;
    }
    record_driver_behavior(
        scenario,
        started,
        writer,
        "BHV-LIFECYCLE-STARTED",
        "observer handshake completed for a live window",
    )?;
    if scenario.surface == Surface::NativeWindow {
        snapshot = match wait_for_native_fixture_ready(scenario, &mut client, snapshot, started) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return Err(with_window_child_diagnostics(&error, child, description));
            }
        };
    }
    let x11_class =
        (scenario.surface == Surface::NativeWindow).then_some(FUNCTIONAL_X11_WINDOW_CLASS);
    let options = observed_window_platform_options(scenario, target, x11_class)?;
    let platform = platform_driver(target, process_id, options)?;
    let _clipboard = ClipboardRestore::capture_if_needed(&scenario.actions, target)?;
    let context = ScenarioRunContext {
        scenario,
        target,
        evidence_dir,
        started,
    };
    run_observed_window_actions(
        context,
        process_id,
        &platform,
        &mut client,
        &mut snapshot,
        writer,
    )?;
    snapshot = wait_for_live_checkpoints(
        scenario,
        &mut client,
        snapshot,
        action_timeout(scenario, started)?,
    )?;
    child.cap_remaining_timeout(cleanup_timeout(scenario, started)?);
    let output = child
        .wait()
        .map_err(|source| RunnerError::child(format!("wait for {description}"), source))?;
    if scenario.fixture == "forced_close" {
        record_driver_behavior(
            scenario,
            started,
            writer,
            "BHV-FAULT-BLOCKED-IO",
            "platform close interrupted a child blocked on PTY input and the app exited",
        )?;
    }
    finish_window_run(context, process_id, &snapshot, &output, writer)
}

fn with_window_child_diagnostics(
    error: &RunnerError,
    child: ChildGuard,
    description: &str,
) -> RunnerError {
    match child.terminate() {
        Ok(output) => RunnerError::Driver(format!(
            "{error}; {description} child status={:?}; stdout={:?}; stderr={:?}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        )),
        Err(source) => RunnerError::child(
            format!("{error}; collect diagnostics from {description}"),
            source,
        ),
    }
}

fn run_observed_window_actions(
    context: ScenarioRunContext<'_>,
    process_id: u32,
    platform: &crate::PlatformInputDriver,
    client: &mut crate::ObserverClient,
    snapshot: &mut crate::ObserverSnapshotV1,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<(), RunnerError> {
    for (action_index, action) in context.scenario.actions.iter().enumerate() {
        if matches!(action, ActionV1::Finish) {
            writer
                .record(EvidenceEventV1::action_finished(
                    elapsed_ms(context.started),
                    action_index,
                    "finish",
                    "waiting_for_native_window_exit",
                ))
                .map_err(|source| RunnerError::Evidence(source.to_string()))?;
            record_action_behavior(context.scenario, context.started, writer, action)?;
            continue;
        }
        if let Err(source) =
            platform.execute(action, action_timeout(context.scenario, context.started)?)
        {
            capture_failure_diagnostics(
                context.scenario,
                context.target,
                context.evidence_dir,
                Some(process_id),
            );
            return Err(RunnerError::Driver(source.to_string()));
        }
        let prior_revision = snapshot.revision;
        if action_must_publish(action) {
            *snapshot = wait_for_observer_change(
                client,
                prior_revision,
                action_timeout(context.scenario, context.started)?,
            )?;
        } else if let Ok(observed) = client.snapshot() {
            *snapshot = observed;
        }
        let result = format!("observer_revision={}", snapshot.revision);
        writer
            .record(EvidenceEventV1::action_finished(
                elapsed_ms(context.started),
                action_index,
                action_name(action),
                &result,
            ))
            .map_err(|source| RunnerError::Evidence(source.to_string()))?;
        record_action_behavior(context.scenario, context.started, writer, action)?;
    }
    Ok(())
}

fn finish_window_run(
    context: ScenarioRunContext<'_>,
    process_id: u32,
    snapshot: &crate::ObserverSnapshotV1,
    output: &rterm_test_support::ChildOutput,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<(), RunnerError> {
    write_process_evidence(
        context.scenario,
        context.target,
        context.evidence_dir,
        &[process_id],
        &output.stdout,
        &output.stderr,
    )?;
    let stem = format!("{}.{}.0", context.scenario.id, context.target);
    fs::write(
        context
            .evidence_dir
            .join(format!("{stem}.final-snapshot.json")),
        serde_json::to_vec_pretty(snapshot).map_err(RunnerError::Json)?,
    )
    .map_err(RunnerError::Io)?;
    if context
        .scenario
        .required_evidence
        .contains(&crate::EvidenceKind::ServerTrace)
    {
        fs::write(
            context
                .evidence_dir
                .join(format!("{stem}.server-trace.json")),
            serde_json::to_vec_pretty(&snapshot.runtime.effects).map_err(RunnerError::Json)?,
        )
        .map_err(RunnerError::Io)?;
    }
    let checkpoint_context = CheckpointContext {
        snapshot: Some(snapshot),
        stdout: &output.stdout,
        stderr: &output.stderr,
        exit_code: output.status.code(),
        resources_zero: observed_resources_zero(snapshot),
        artifact_root: Some(context.evidence_dir),
        network_bytes: std::collections::BTreeMap::new(),
    };
    let result = finish_checkpoints(
        context.scenario,
        context.started,
        writer,
        &checkpoint_context,
    );
    if result.is_err() {
        capture_failure_diagnostics(
            context.scenario,
            context.target,
            context.evidence_dir,
            Some(process_id),
        );
    }
    result
}

struct ClipboardRestore {
    program: Option<String>,
    arguments: Vec<String>,
    environment: Vec<(String, String)>,
    previous_text: Option<String>,
}

impl ClipboardRestore {
    fn capture_if_needed(actions: &[ActionV1], target: &str) -> Result<Self, RunnerError> {
        if !actions
            .iter()
            .any(|action| matches!(action, ActionV1::ClipboardPaste { .. }))
        {
            return Ok(Self {
                program: None,
                arguments: Vec::new(),
                environment: Vec::new(),
                previous_text: None,
            });
        }
        let (program, read_arguments, write_arguments, environment) = if target.contains("windows")
        {
            (
                "powershell.exe".to_owned(),
                vec![
                    "-NoProfile".to_owned(),
                    "-NonInteractive".to_owned(),
                    "-Command".to_owned(),
                    "Get-Clipboard -Raw".to_owned(),
                ],
                vec![
                    "-NoProfile".to_owned(),
                    "-NonInteractive".to_owned(),
                    "-Command".to_owned(),
                    "Set-Clipboard -Value ([Console]::In.ReadToEnd())".to_owned(),
                ],
                Vec::new(),
            )
        } else if target.contains("macos") {
            ("pbpaste".to_owned(), Vec::new(), Vec::new(), Vec::new())
        } else {
            let display = std::env::var("DISPLAY").map_err(|_| {
                RunnerError::Driver("DISPLAY is absent for clipboard restore".to_owned())
            })?;
            (
                "xclip".to_owned(),
                vec![
                    "-selection".to_owned(),
                    "clipboard".to_owned(),
                    "-o".to_owned(),
                ],
                vec!["-selection".to_owned(), "clipboard".to_owned()],
                vec![("DISPLAY".to_owned(), display)],
            )
        };
        let output = Command::new(&program)
            .args(&read_arguments)
            .envs(environment.iter().cloned())
            .output()
            .map_err(|source| RunnerError::Driver(format!("read system clipboard: {source}")))?;
        let previous_text = output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).into_owned());
        let (program, arguments) = if target.contains("macos") {
            ("pbcopy".to_owned(), Vec::new())
        } else {
            (program, write_arguments)
        };
        Ok(Self {
            program: Some(program),
            arguments,
            environment,
            previous_text,
        })
    }
}

impl Drop for ClipboardRestore {
    fn drop(&mut self) {
        let (Some(program), Some(previous)) = (&self.program, self.previous_text.take()) else {
            return;
        };
        let Ok(mut child) = Command::new(program)
            .args(&self.arguments)
            .envs(self.environment.iter().cloned())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        else {
            return;
        };
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(previous.as_bytes());
        }
        let _ = child.wait();
    }
}

fn connect_observer(
    endpoint: &std::path::Path,
    token: &crate::ObserverToken,
    timeout: Duration,
) -> Result<crate::ObserverClient, RunnerError> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| RunnerError::Driver("observer startup deadline overflow".to_owned()))?;
    let mut last_error = None;
    while Instant::now() < deadline {
        match crate::ObserverClient::connect_path(endpoint) {
            Ok(mut client) => match client.hello(token.clone()) {
                Ok(()) => return Ok(client),
                Err(error) => last_error = Some(error),
            },
            Err(error) => last_error = Some(error),
        }
        std::thread::park_timeout(Duration::from_millis(10));
    }
    Err(RunnerError::Driver(format!(
        "observer did not become ready before the startup deadline: {}",
        last_error.map_or_else(
            || "no connection attempt completed".to_owned(),
            |error| error.to_string()
        )
    )))
}

#[derive(Clone, Copy, Default)]
struct PlatformDriverOptions<'a> {
    window_title: Option<&'a str>,
    window_handle: Option<u64>,
    windows_client_close_button: bool,
    x11_class: Option<&'a str>,
    xtest_paste_key: Option<&'a str>,
    xtest_wayland_clipboard: bool,
    xtest_web_close_point: Option<(i32, i32)>,
}

impl<'a> PlatformDriverOptions<'a> {
    const fn window_title(window_title: Option<&'a str>) -> Self {
        Self {
            window_title,
            window_handle: None,
            windows_client_close_button: false,
            x11_class: None,
            xtest_paste_key: None,
            xtest_wayland_clipboard: false,
            xtest_web_close_point: None,
        }
    }

    const fn x11_class(x11_class: &'a str) -> Self {
        Self {
            window_title: None,
            window_handle: None,
            windows_client_close_button: false,
            x11_class: Some(x11_class),
            xtest_paste_key: None,
            xtest_wayland_clipboard: false,
            xtest_web_close_point: None,
        }
    }
}

fn platform_driver(
    target: &str,
    process_id: u32,
    options: PlatformDriverOptions<'_>,
) -> Result<crate::PlatformInputDriver, RunnerError> {
    let backend = if target.contains("windows") {
        crate::InputBackend::WindowsSendInput
    } else if target.contains("wayland") {
        crate::InputBackend::WaylandWestonSeat
    } else if target.contains("x11") || target.contains("linux") {
        crate::InputBackend::X11Xtest
    } else if target.contains("macos") {
        crate::InputBackend::MacosCgEvent
    } else {
        return Err(RunnerError::Driver(format!(
            "target {target:?} has no platform input backend"
        )));
    };
    let mut environment: std::collections::BTreeMap<String, String> = std::env::vars().collect();
    environment.insert(
        "RTERM_FUNCTIONAL_APP_PID".to_owned(),
        process_id.to_string(),
    );
    if let Some(window_title) = options.window_title {
        environment.insert(
            "RTERM_FUNCTIONAL_WINDOWS_WINDOW_TITLE".to_owned(),
            window_title.to_owned(),
        );
    }
    if let Some(window_handle) = options.window_handle {
        environment.insert(
            "RTERM_FUNCTIONAL_WINDOWS_WINDOW_HANDLE".to_owned(),
            format!("hwnd:{window_handle}"),
        );
    }
    if options.windows_client_close_button {
        environment.insert(
            "RTERM_FUNCTIONAL_WINDOWS_CLIENT_CLOSE_BUTTON".to_owned(),
            "1".to_owned(),
        );
    }
    if let Some(xtest_paste_key) = options.xtest_paste_key {
        environment.insert(
            "RTERM_FUNCTIONAL_XTEST_PASTE_KEY".to_owned(),
            xtest_paste_key.to_owned(),
        );
    }
    if backend == crate::InputBackend::WaylandWestonSeat && options.x11_class.is_some() {
        environment.insert(
            "RTERM_FUNCTIONAL_XTEST_CLOSE_KEY".to_owned(),
            "ctrl+shift+w".to_owned(),
        );
        environment.insert(
            "RTERM_FUNCTIONAL_XTEST_CLOSE_CONFIRM_KEY".to_owned(),
            "Return".to_owned(),
        );
    }
    if options.xtest_wayland_clipboard {
        environment.insert(
            "RTERM_FUNCTIONAL_WAYLAND_CLIPBOARD".to_owned(),
            "1".to_owned(),
        );
    }
    if let Some((x, y)) = options.xtest_web_close_point {
        environment.insert(
            "RTERM_FUNCTIONAL_XTEST_WEB_CLOSE_POINT".to_owned(),
            format!("{x},{y}"),
        );
    }
    if backend == crate::InputBackend::X11Xtest {
        let window = discover_x11_window(process_id, options.x11_class, &environment)?;
        environment.insert("RTERM_FUNCTIONAL_X11_WINDOW".to_owned(), window);
    }
    crate::PlatformInputDriver::from_environment(backend, &environment)
        .map_err(|source| RunnerError::Driver(source.to_string()))
}

fn tauri_web_close_point(actions: &[ActionV1]) -> Result<(i32, i32), RunnerError> {
    let width = actions
        .iter()
        .rev()
        .find_map(|action| match action {
            ActionV1::ResizeWindow { width, .. } => Some(*width),
            _ => None,
        })
        .ok_or_else(|| {
            RunnerError::Driver("Tauri Wayland close requires a resize action".to_owned())
        })?;
    let x = i32::try_from(width)
        .ok()
        .and_then(|width| width.checked_sub(44))
        .filter(|x| *x > 0)
        .ok_or_else(|| RunnerError::Driver("Tauri Wayland close width is invalid".to_owned()))?;
    Ok((x, 35))
}

fn observed_window_platform_options<'a>(
    scenario: &ScenarioV1,
    target: &str,
    x11_class: Option<&'a str>,
) -> Result<PlatformDriverOptions<'a>, RunnerError> {
    let tauri_wayland = scenario.surface == Surface::Tauri && target.contains("wayland");
    Ok(PlatformDriverOptions {
        window_title: None,
        window_handle: None,
        windows_client_close_button: scenario.surface == Surface::Tauri
            && target.contains("windows"),
        x11_class,
        xtest_paste_key: tauri_wayland.then_some("ctrl+v"),
        xtest_wayland_clipboard: tauri_wayland,
        xtest_web_close_point: tauri_wayland
            .then(|| tauri_web_close_point(&scenario.actions))
            .transpose()?,
    })
}

fn discover_x11_window(
    process_id: u32,
    window_class: Option<&str>,
    environment: &std::collections::BTreeMap<String, String>,
) -> Result<String, RunnerError> {
    let program = environment
        .get("RTERM_FUNCTIONAL_XDOTOOL")
        .map_or("xdotool", String::as_str);
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(15))
        .ok_or_else(|| RunnerError::Driver("X11 discovery deadline overflow".to_owned()))?;
    let mut observed = 0;
    for _ in 0..100 {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        let mut selectors = vec![vec![
            "search".to_owned(),
            "--onlyvisible".to_owned(),
            "--pid".to_owned(),
            process_id.to_string(),
        ]];
        if let Some(window_class) = window_class {
            selectors.push(vec![
                "search".to_owned(),
                "--onlyvisible".to_owned(),
                "--class".to_owned(),
                format!("^{window_class}$"),
            ]);
        }
        let mut stdout = Vec::new();
        for selector in selectors {
            let mut command = Command::new(program);
            command.args(selector);
            if let Some(display) = environment.get("DISPLAY") {
                command.env("DISPLAY", display);
            }
            let output = ChildGuard::spawn(command, remaining.min(Duration::from_secs(2)))
                .map_err(|source| RunnerError::Driver(format!("discover X11 window: {source}")))?
                .wait()
                .map_err(|source| RunnerError::child("discover X11 window", source))?;
            if !output.stdout.is_empty() {
                stdout = output.stdout;
                break;
            }
        }
        let windows: Vec<_> = String::from_utf8_lossy(&stdout)
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(str::to_owned)
            .collect();
        observed = windows.len();
        if observed == 1 {
            return Ok(windows[0].clone());
        }
        if observed > 1 {
            break;
        }
        std::thread::park_timeout(Duration::from_millis(50));
    }
    Err(RunnerError::Driver(format!(
        "expected one visible X11 window for PID {process_id}; observed {observed}"
    )))
}

fn wait_for_terminal_text(
    client: &mut crate::ObserverClient,
    mut snapshot: crate::ObserverSnapshotV1,
    marker: &str,
    timeout: Duration,
) -> Result<crate::ObserverSnapshotV1, RunnerError> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| RunnerError::Driver("terminal readiness deadline overflow".to_owned()))?;
    loop {
        if snapshot.terminal.text.contains(marker) {
            return Ok(snapshot);
        }
        if Instant::now() >= deadline {
            return Err(RunnerError::Scenario(format!(
                "observer terminal did not contain readiness marker {marker:?}"
            )));
        }
        if let Some(updated) = client
            .subscribe(snapshot.revision)
            .map_err(|source| RunnerError::Driver(format!("subscribe to observer: {source}")))?
        {
            snapshot = updated;
        }
    }
}

fn wait_for_native_fixture_ready(
    scenario: &ScenarioV1,
    client: &mut crate::ObserverClient,
    snapshot: crate::ObserverSnapshotV1,
    started: Instant,
) -> Result<crate::ObserverSnapshotV1, RunnerError> {
    let marker = if scenario.fixture == "forced_close" {
        "fixture-hold-open"
    } else {
        "fixture-ready"
    };
    wait_for_terminal_text(
        client,
        snapshot,
        marker,
        startup_timeout(scenario, started)?,
    )
}

fn wait_for_observer_change(
    client: &mut crate::ObserverClient,
    revision: u64,
    timeout: Duration,
) -> Result<crate::ObserverSnapshotV1, RunnerError> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| RunnerError::Driver("observer action deadline overflow".to_owned()))?;
    while Instant::now() < deadline {
        if let Some(snapshot) = client
            .subscribe(revision)
            .map_err(|source| RunnerError::Driver(format!("subscribe to observer: {source}")))?
        {
            return Ok(snapshot);
        }
    }
    Err(RunnerError::Scenario(format!(
        "observer revision did not advance after {revision}"
    )))
}

fn wait_for_live_checkpoints(
    scenario: &ScenarioV1,
    client: &mut crate::ObserverClient,
    mut snapshot: crate::ObserverSnapshotV1,
    timeout: Duration,
) -> Result<crate::ObserverSnapshotV1, RunnerError> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| RunnerError::Driver("checkpoint deadline overflow".to_owned()))?;
    loop {
        let context = CheckpointContext {
            snapshot: Some(&snapshot),
            stdout: &[],
            stderr: &[],
            exit_code: None,
            resources_zero: observed_resources_zero(&snapshot),
            artifact_root: None,
            network_bytes: std::collections::BTreeMap::new(),
        };
        let all_ready = scenario
            .checkpoints
            .iter()
            .filter(|checkpoint| !matches!(checkpoint, CheckpointV1::ExitStatus { .. }))
            .all(|checkpoint| evaluate_checkpoint(checkpoint, &context).is_ok());
        if all_ready {
            return Ok(snapshot);
        }
        if Instant::now() >= deadline {
            return Ok(snapshot);
        }
        match client.subscribe(snapshot.revision) {
            Ok(Some(updated)) => snapshot = updated,
            Ok(None) => {}
            Err(source) if observer_closed_with_process(&source) => return Ok(snapshot),
            Err(source) => {
                return Err(RunnerError::Driver(format!(
                    "observer disconnected before checkpoints became true: {source}"
                )));
            }
        }
    }
}

fn observer_closed_with_process(error: &io::Error) -> bool {
    let expected_kind = matches!(
        error.kind(),
        io::ErrorKind::UnexpectedEof
            | io::ErrorKind::BrokenPipe
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::NotConnected
    );
    let message = error.to_string();
    expected_kind
        || error.raw_os_error() == Some(232)
        || message.contains("The pipe is being closed")
        || message.contains("(os error 32)")
        || message.contains("(os error 232)")
}

fn observed_resources_zero(snapshot: &crate::ObserverSnapshotV1) -> bool {
    snapshot.runtime.worker_count == 0
        && snapshot.runtime.listener_count == 0
        && snapshot.runtime.child_process_count == 0
}

fn action_must_publish(action: &ActionV1) -> bool {
    matches!(
        action,
        ActionV1::Key { .. } | ActionV1::ClipboardPaste { .. } | ActionV1::ResizeWindow { .. }
    )
}

fn action_name(action: &ActionV1) -> &'static str {
    match action {
        ActionV1::TypeText { .. } => "type_text",
        ActionV1::Key { .. } => "key",
        ActionV1::MouseClick { .. } => "mouse_click",
        ActionV1::MouseDrag { .. } => "mouse_drag",
        ActionV1::MouseWheel { .. } => "mouse_wheel",
        ActionV1::ClipboardPaste { .. } => "clipboard_paste",
        ActionV1::ResizeWindow { .. } => "resize_window",
        ActionV1::WindowControl { .. } => "window_control",
        ActionV1::FocusWindow => "focus_window",
        ActionV1::PtyInput { .. } => "pty_input",
        ActionV1::FixtureDisconnect { .. } => "fixture_disconnect",
        ActionV1::FixtureReconnect { .. } => "fixture_reconnect",
        ActionV1::Finish => "finish",
    }
}

fn execute_pty_scenario(
    scenario: &ScenarioV1,
    app: &Path,
    fixture_bin: &Path,
    target: &str,
    evidence_dir: &Path,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<(), RunnerError> {
    let arguments = [
        "local".to_owned(),
        "--".to_owned(),
        fixture_bin.to_string_lossy().into_owned(),
        "echo-query".to_owned(),
    ];
    let mut driver = crate::PtyFixtureDriver::spawn_with_args(
        app,
        arguments,
        80,
        24,
        scenario_timeout(scenario, started)?,
    )
    .and_then(|driver| driver.wait_for_output(b"fixture-ready"))
    .map_err(|source| RunnerError::Driver(source.to_string()))?;
    for (action_index, action) in scenario.actions.iter().enumerate() {
        match action {
            ActionV1::PtyInput { bytes_hex } => {
                let bytes = decode_hex(bytes_hex).map_err(RunnerError::Driver)?;
                driver = driver
                    .write(&bytes)
                    .map_err(|source| RunnerError::Driver(source.to_string()))?;
                writer
                    .record(EvidenceEventV1::action_finished(
                        elapsed_ms(started),
                        action_index,
                        "pty_input",
                        "accepted",
                    ))
                    .map_err(|source| RunnerError::Evidence(source.to_string()))?;
                record_action_behavior(scenario, started, writer, action)?;
            }
            ActionV1::Finish => {
                writer
                    .record(EvidenceEventV1::action_finished(
                        elapsed_ms(started),
                        action_index,
                        "finish",
                        "waiting_for_process_exit",
                    ))
                    .map_err(|source| RunnerError::Evidence(source.to_string()))?;
                record_action_behavior(scenario, started, writer, action)?;
            }
            action => {
                return Err(RunnerError::Driver(format!(
                    "PTY driver does not support action {action:?}"
                )));
            }
        }
    }
    let result = driver
        .cap_remaining_timeout(cleanup_timeout(scenario, started)?)
        .finish()
        .map_err(|source| RunnerError::Driver(source.to_string()))?;
    record_driver_behavior(
        scenario,
        started,
        writer,
        "BHV-CMD-LOCAL",
        "local command completed through a real PTY",
    )?;
    record_driver_behavior(
        scenario,
        started,
        writer,
        "BHV-CONSOLE-INTERACTION",
        "PTY round-tripped UTF-8 input and output",
    )?;
    record_driver_behavior(
        scenario,
        started,
        writer,
        "BHV-LIFECYCLE-STARTED",
        "PTY child started and accepted input",
    )?;
    if result.transport_write_observed() {
        record_driver_behavior(
            scenario,
            started,
            writer,
            "BHV-EFFECT-TRANSPORT-WRITE",
            "terminal query produced a PTY response",
        )?;
    }
    write_pty_evidence(
        ScenarioRunContext {
            scenario,
            target,
            evidence_dir,
            started,
        },
        &result,
    )?;
    let output = ChildOutputProjection {
        stdout: &result.output,
        stderr: &[],
        exit_code: i32::try_from(result.exit_code).unwrap_or(i32::MAX),
        resources_zero: result.resources_zero(),
    };
    finish_projected_checkpoints(scenario, started, writer, &output)
}

fn write_pty_evidence(
    context: ScenarioRunContext<'_>,
    result: &crate::PtyFixtureResult,
) -> Result<(), RunnerError> {
    let scenario = context.scenario;
    let target = context.target;
    let evidence_dir = context.evidence_dir;
    let stem = format!("{}.{target}.0", scenario.id);
    fs::write(evidence_dir.join(format!("{stem}.stdout")), &result.output)
        .map_err(RunnerError::Io)?;
    fs::write(evidence_dir.join(format!("{stem}.stderr")), b"").map_err(RunnerError::Io)?;
    fs::write(
        evidence_dir.join(format!("{stem}.final-snapshot.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": 1,
            "terminal_text": String::from_utf8_lossy(&result.output),
            "terminal_query_responses": result.terminal_query_responses,
        }))
        .map_err(RunnerError::Json)?,
    )
    .map_err(RunnerError::Io)?;
    fs::write(
        evidence_dir.join(format!("{stem}.process-tree.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": 1,
            "root_process_id": null,
            "reaped": result.child_process_reaped,
            "reader_joined": result.reader_joined,
            "master_closed": result.master_closed,
            "remaining_owned_processes": u8::from(!result.resources_zero()),
        }))
        .map_err(RunnerError::Json)?,
    )
    .map_err(RunnerError::Io)?;
    Ok(())
}

fn execute_pty_stress_scenario(
    scenario: &ScenarioV1,
    app: &Path,
    fixture_bin: &Path,
    target: &str,
    evidence_dir: &Path,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<(), RunnerError> {
    record_finish_only_actions(scenario, started, writer)?;
    let run = |arguments: Vec<String>, input: Option<&[u8]>| {
        let arguments = std::iter::once("local".to_owned())
            .chain(std::iter::once("--".to_owned()))
            .chain(std::iter::once(fixture_bin.to_string_lossy().into_owned()))
            .chain(arguments)
            .collect::<Vec<_>>();
        let driver = crate::PtyFixtureDriver::spawn_with_args(
            app,
            arguments,
            80,
            24,
            scenario_timeout(scenario, started)?,
        )
        .map_err(|source| RunnerError::Driver(source.to_string()))?;
        let driver = if let Some(input) = input {
            driver
                .write(input)
                .map_err(|source| RunnerError::Driver(source.to_string()))?
        } else {
            driver
        };
        driver
            .finish()
            .map_err(|source| RunnerError::Driver(source.to_string()))
    };
    let high_output = run(vec!["high-output".to_owned(), "1048576".to_owned()], None)?;
    let high_output_bytes = high_output
        .output
        .split(|byte| *byte != b'X')
        .map(<[u8]>::len)
        .sum::<usize>();
    let synchronized = run(vec!["synchronized-output".to_owned()], None)?;
    let synchronized_output_released = synchronized_output_was_released(&synchronized.output);
    let nonzero = run(vec!["exit-code".to_owned(), "37".to_owned()], None)?;
    let slow_read = run(
        vec!["slow-read".to_owned(), "1".to_owned(), "22".to_owned()],
        Some(b"functional-slow-read\r\n"),
    )?;
    let slow_read_completed =
        String::from_utf8_lossy(&slow_read.output).contains("functional-slow-read");
    let resources_zero = [&high_output, &synchronized, &nonzero, &slow_read]
        .into_iter()
        .all(crate::PtyFixtureResult::resources_zero);
    if high_output_bytes < 1_048_576
        || !synchronized_output_released
        || nonzero.exit_code != 37
        || !slow_read_completed
    {
        return Err(RunnerError::Scenario(format!(
            "stress journey mismatch: high_output={high_output_bytes}, sync={synchronized_output_released}, exit={}, slow_read={slow_read_completed}",
            nonzero.exit_code
        )));
    }
    for (behavior, evidence) in [
        (
            "BHV-CMD-LOCAL",
            "real local entry completed all stress fixtures",
        ),
        (
            "BHV-CONSOLE-STRESS",
            "1 MiB output, synchronized output, slow read, and exit 37 completed",
        ),
    ] {
        record_driver_behavior(scenario, started, writer, behavior, evidence)?;
    }
    write_pty_stress_evidence(
        ScenarioRunContext {
            scenario,
            target,
            evidence_dir,
            started,
        },
        &high_output,
        &PtyStressEvidence {
            high_output_bytes,
            synchronized_output_released,
            nonzero_exit_code: nonzero.exit_code,
            slow_read_completed,
            resources_zero,
        },
    )?;
    finish_projected_checkpoints(
        scenario,
        started,
        writer,
        &ChildOutputProjection {
            stdout: &high_output.output,
            stderr: &[],
            exit_code: 0,
            resources_zero,
        },
    )
}

struct PtyStressEvidence {
    high_output_bytes: usize,
    synchronized_output_released: bool,
    nonzero_exit_code: u32,
    slow_read_completed: bool,
    resources_zero: bool,
}

fn synchronized_output_was_released(output: &[u8]) -> bool {
    let Some(first) = output
        .windows(b"first".len())
        .position(|part| part == b"first")
    else {
        return false;
    };
    output[first + b"first".len()..]
        .windows(b"second".len())
        .any(|part| part == b"second")
}

fn write_pty_stress_evidence(
    context: ScenarioRunContext<'_>,
    high_output: &crate::PtyFixtureResult,
    result: &PtyStressEvidence,
) -> Result<(), RunnerError> {
    let scenario = context.scenario;
    let target = context.target;
    let evidence_dir = context.evidence_dir;
    let stem = format!("{}.{target}.0", scenario.id);
    fs::write(
        evidence_dir.join(format!("{stem}.stdout")),
        &high_output.output,
    )
    .map_err(RunnerError::Io)?;
    fs::write(evidence_dir.join(format!("{stem}.stderr")), b"").map_err(RunnerError::Io)?;
    fs::write(
        evidence_dir.join(format!("{stem}.final-snapshot.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": 1,
            "high_output_bytes": result.high_output_bytes,
            "synchronized_output_released": result.synchronized_output_released,
            "nonzero_exit_code": result.nonzero_exit_code,
            "slow_read_completed": result.slow_read_completed,
        }))
        .map_err(RunnerError::Json)?,
    )
    .map_err(RunnerError::Io)?;
    fs::write(
        evidence_dir.join(format!("{stem}.process-tree.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": 1,
            "subprocesses": 4,
            "reaped": result.resources_zero,
            "reader_joined": result.resources_zero,
            "master_closed": result.resources_zero,
            "remaining_owned_processes": u8::from(!result.resources_zero),
        }))
        .map_err(RunnerError::Json)?,
    )
    .map_err(RunnerError::Io)?;
    Ok(())
}

struct ChildOutputProjection<'a> {
    stdout: &'a [u8],
    stderr: &'a [u8],
    exit_code: i32,
    resources_zero: bool,
}

fn finish_projected_checkpoints(
    scenario: &ScenarioV1,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
    output: &ChildOutputProjection<'_>,
) -> Result<(), RunnerError> {
    let context = CheckpointContext {
        snapshot: None,
        stdout: output.stdout,
        stderr: output.stderr,
        exit_code: Some(output.exit_code),
        resources_zero: output.resources_zero,
        artifact_root: None,
        network_bytes: std::collections::BTreeMap::new(),
    };
    finish_checkpoints(scenario, started, writer, &context)
}

fn finish_checkpoints(
    scenario: &ScenarioV1,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
    context: &CheckpointContext<'_>,
) -> Result<(), RunnerError> {
    let mut failures = Vec::new();
    for (checkpoint_index, checkpoint) in scenario.checkpoints.iter().enumerate() {
        let result = evaluate_checkpoint(checkpoint, context).map_err(|error| error.to_string());
        let (passed, detail) = match result {
            Ok(detail) => (true, detail),
            Err(detail) => {
                failures.push(format!("checkpoint {checkpoint_index}: {detail}"));
                (false, detail)
            }
        };
        writer
            .record(EvidenceEventV1::checkpoint_finished(
                elapsed_ms(started),
                checkpoint_index,
                checkpoint_name(checkpoint),
                passed,
                &detail,
            ))
            .map_err(|source| RunnerError::Evidence(source.to_string()))?;
        if passed {
            record_checkpoint_behavior(scenario, started, writer, checkpoint, &detail)?;
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(RunnerError::Scenario(failures.join("; ")))
    }
}

fn record_action_behavior(
    scenario: &ScenarioV1,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
    action: &ActionV1,
) -> Result<(), RunnerError> {
    let behavior = match action {
        ActionV1::TypeText { .. } => "BHV-ACTION-TYPE-TEXT",
        ActionV1::Key { .. } => "BHV-ACTION-KEY",
        ActionV1::MouseClick { .. } => "BHV-ACTION-MOUSE-CLICK",
        ActionV1::MouseDrag { .. } => "BHV-ACTION-MOUSE-DRAG",
        ActionV1::MouseWheel { .. } => "BHV-ACTION-MOUSE-WHEEL",
        ActionV1::ClipboardPaste { .. } => "BHV-ACTION-CLIPBOARD-PASTE",
        ActionV1::ResizeWindow { .. } => "BHV-ACTION-RESIZE",
        ActionV1::WindowControl { .. } => "BHV-ACTION-WINDOW-CONTROL",
        ActionV1::FocusWindow => "BHV-ACTION-FOCUS",
        ActionV1::PtyInput { .. } => "BHV-ACTION-PTY-INPUT",
        ActionV1::FixtureDisconnect { .. } => "BHV-ACTION-FIXTURE-DISCONNECT",
        ActionV1::FixtureReconnect { .. } => "BHV-ACTION-FIXTURE-RECONNECT",
        ActionV1::Finish => "BHV-ACTION-FINISH",
    };
    record_driver_behavior(
        scenario,
        started,
        writer,
        behavior,
        &format!(
            "{} action completed through its real driver",
            action_name(action)
        ),
    )
}

fn record_checkpoint_behavior(
    scenario: &ScenarioV1,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
    checkpoint: &CheckpointV1,
    detail: &str,
) -> Result<(), RunnerError> {
    let behavior = match checkpoint {
        CheckpointV1::HostEffect { kind, .. } => effect_behavior(kind),
        CheckpointV1::Transport { state } => match state.as_str() {
            "connected" => Some("BHV-LIFECYCLE-CONNECTED"),
            "disconnected" | "closed" => Some("BHV-LIFECYCLE-DISCONNECTED"),
            "reconnected" => Some("BHV-LIFECYCLE-RECONNECTED"),
            "error" => Some("BHV-LIFECYCLE-ERROR"),
            _ => None,
        },
        CheckpointV1::ExitStatus { .. } => Some("BHV-LIFECYCLE-EXITED"),
        CheckpointV1::ResourcesZero => Some("BHV-LIFECYCLE-CLEANUP"),
        CheckpointV1::RenderProbe { .. } => Some("BHV-RENDER-STABLE-REGION"),
        _ => None,
    };
    if let Some(behavior) = behavior {
        record_driver_behavior(scenario, started, writer, behavior, detail)?;
    }
    Ok(())
}

fn effect_behavior(kind: &str) -> Option<&'static str> {
    match kind {
        "transport_write" => Some("BHV-EFFECT-TRANSPORT-WRITE"),
        "host_stream" => Some("BHV-EFFECT-HOST-STREAM"),
        "visible_output" => Some("BHV-EFFECT-VISIBLE-OUTPUT"),
        "mode_change" => Some("BHV-EFFECT-MODE-CHANGE"),
        "clipboard_write" => Some("BHV-EFFECT-CLIPBOARD-WRITE"),
        "clipboard_read" => Some("BHV-EFFECT-CLIPBOARD-READ"),
        "notification" => Some("BHV-EFFECT-NOTIFICATION"),
        "bell" => Some("BHV-EFFECT-BELL"),
        "diagnostic" => Some("BHV-EFFECT-DIAGNOSTIC"),
        _ => None,
    }
}

fn record_driver_behavior(
    _scenario: &ScenarioV1,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
    behavior: &str,
    evidence: &str,
) -> Result<(), RunnerError> {
    writer
        .record(EvidenceEventV1::behavior_observed(
            elapsed_ms(started),
            behavior,
            evidence,
        ))
        .map_err(|source| RunnerError::Evidence(source.to_string()))
}

fn process_driver_behaviors(scenario_id: &str) -> &'static [&'static str] {
    match scenario_id {
        "startup.version" => &["BHV-STARTUP-VERSION"],
        "startup.self-test" => &["BHV-STARTUP-SELF-TEST"],
        "startup.help" => &["BHV-CMD-HELP"],
        "startup.doctor" => &["BHV-CMD-DOCTOR"],
        "startup.bench" => &["BHV-CMD-BENCH"],
        "package.startup-smoke" => &[
            "BHV-STARTUP-VERSION",
            "BHV-STARTUP-SELF-TEST",
            "BHV-PACKAGE-OBSERVER-ABSENT",
        ],
        _ => &[],
    }
}

fn execute_process_scenario(
    scenario: &ScenarioV1,
    app: &Path,
    target: &str,
    evidence_dir: &Path,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<(), RunnerError> {
    let commands = process_commands(scenario)?;
    let mut process_ids = Vec::with_capacity(commands.len());
    let mut combined_stdout = Vec::new();
    let mut combined_stderr = Vec::new();
    let mut final_status = None;

    for (action_index, action) in scenario.actions.iter().enumerate() {
        match action {
            ActionV1::Finish => {
                writer
                    .record(EvidenceEventV1::action_finished(
                        elapsed_ms(started),
                        action_index,
                        "finish",
                        "waiting_for_process_exit",
                    ))
                    .map_err(|source| RunnerError::Evidence(source.to_string()))?;
                record_action_behavior(scenario, started, writer, action)?;
            }
            action => {
                return Err(RunnerError::Driver(format!(
                    "process driver does not support action {action:?}"
                )));
            }
        }
    }

    for arguments in commands {
        let mut command = hermetic_app_command(app);
        command.args(arguments);
        let child = ChildGuard::spawn(command, action_timeout(scenario, started)?)
            .map_err(|source| RunnerError::Driver(format!("launch `{}`: {source}", scenario.id)))?;
        process_ids.push(child.process_id().ok_or_else(|| {
            RunnerError::Driver("spawned child did not expose a process id".to_owned())
        })?);
        let output = child
            .wait()
            .map_err(|source| RunnerError::child(format!("wait for `{}`", scenario.id), source))?;
        final_status = output.status.code();
        combined_stdout.extend_from_slice(&output.stdout);
        combined_stderr.extend_from_slice(&output.stderr);
        if !output.status.success() {
            break;
        }
    }
    for behavior in process_driver_behaviors(&scenario.id) {
        record_driver_behavior(
            scenario,
            started,
            writer,
            behavior,
            "public process entry completed successfully",
        )?;
    }
    write_process_evidence(
        scenario,
        target,
        evidence_dir,
        &process_ids,
        &combined_stdout,
        &combined_stderr,
    )?;
    let context = CheckpointContext {
        snapshot: None,
        stdout: &combined_stdout,
        stderr: &combined_stderr,
        exit_code: final_status,
        resources_zero: true,
        artifact_root: Some(evidence_dir),
        network_bytes: std::collections::BTreeMap::new(),
    };
    finish_checkpoints(scenario, started, writer, &context)
}

fn execute_profile_lifecycle_scenario(
    scenario: &ScenarioV1,
    app: &Path,
    fixture_bin: &Path,
    target: &str,
    evidence_dir: &Path,
    started: Instant,
    writer: &mut EvidenceWriter<fs::File>,
) -> Result<(), RunnerError> {
    let profile_dir = tempfile::Builder::new()
        .prefix("rterm-functional-profile-")
        .tempdir_in(evidence_dir)
        .map_err(RunnerError::Io)?;
    let file = profile_dir.path().join("profiles.toml");
    let (combined_stdout, combined_stderr) =
        run_profile_lifecycle_commands(scenario, app, fixture_bin, &file, started)?;
    record_finish_only_actions(scenario, started, writer)?;
    for behavior in [
        "BHV-CMD-PROFILE-INIT",
        "BHV-CMD-PROFILE-CHECK",
        "BHV-CMD-PROFILE-LIST",
        "BHV-CMD-PROFILE-SHOW",
        "BHV-CMD-PROFILE-RUN",
    ] {
        record_driver_behavior(
            scenario,
            started,
            writer,
            behavior,
            "isolated profile lifecycle operation completed",
        )?;
    }
    write_profile_evidence(
        ScenarioRunContext {
            scenario,
            target,
            evidence_dir,
            started,
        },
        &combined_stdout,
        &combined_stderr,
    )?;
    let context = CheckpointContext {
        snapshot: None,
        stdout: &combined_stdout,
        stderr: &combined_stderr,
        exit_code: Some(0),
        resources_zero: true,
        artifact_root: Some(evidence_dir),
        network_bytes: std::collections::BTreeMap::new(),
    };
    finish_checkpoints(scenario, started, writer, &context)
}

fn run_profile_lifecycle_commands(
    scenario: &ScenarioV1,
    app: &Path,
    fixture_bin: &Path,
    file: &Path,
    started: Instant,
) -> Result<(Vec<u8>, Vec<u8>), RunnerError> {
    let operations = [
        vec!["profile", "--init", "--file"],
        vec!["profile", "--check", "--json", "--file"],
        vec!["profile", "--list", "--json", "--file"],
        vec!["profile", "--show", "local-smoke", "--json", "--file"],
    ];
    let mut combined_stdout = Vec::new();
    let mut combined_stderr = Vec::new();
    for arguments in operations {
        let mut command = hermetic_app_command(app);
        command.args(arguments).arg(file);
        let output = ChildGuard::spawn(command, action_timeout(scenario, started)?)
            .map_err(|source| RunnerError::Driver(format!("launch profile operation: {source}")))?
            .wait()
            .map_err(|source| RunnerError::child("wait profile operation", source))?;
        combined_stdout.extend_from_slice(&output.stdout);
        combined_stderr.extend_from_slice(&output.stderr);
        if !output.status.success() {
            return Err(RunnerError::Scenario(format!(
                "profile operation failed with {:?}: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            )));
        }
    }
    let fixture_program = fixture_bin.to_string_lossy().replace('\\', "\\\\");
    fs::write(
        file,
        format!(
            "[profiles.functional-smoke]\nkind = \"local\"\ncommand = [\"{fixture_program}\", \"exit-code\", \"0\"]\n"
        ),
    )
    .map_err(RunnerError::Io)?;
    for arguments in [
        vec!["profile", "--check", "--json", "--file"],
        vec!["profile", "--list", "--json", "--file"],
        vec!["profile", "--show", "functional-smoke", "--json", "--file"],
    ] {
        let mut command = hermetic_app_command(app);
        command.args(arguments).arg(file);
        let output = ChildGuard::spawn(command, action_timeout(scenario, started)?)
            .map_err(|source| {
                RunnerError::Driver(format!("launch portable profile validation: {source}"))
            })?
            .wait()
            .map_err(|source| RunnerError::child("wait portable profile validation", source))?;
        combined_stdout.extend_from_slice(&output.stdout);
        combined_stderr.extend_from_slice(&output.stderr);
        if !output.status.success() {
            return Err(RunnerError::Scenario(format!(
                "portable profile validation failed with {:?}: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            )));
        }
    }
    let mut command = hermetic_app_command(app);
    command
        .args(["profile", "functional-smoke", "--file"])
        .arg(file);
    let mut profile_run = ChildGuard::spawn(command, scenario_timeout(scenario, started)?)
        .map_err(|source| RunnerError::Driver(format!("launch profile run: {source}")))?;
    profile_run.cap_remaining_timeout(cleanup_timeout(scenario, started)?);
    let profile_run = profile_run
        .wait()
        .map_err(|source| RunnerError::child("wait profile run", source))?;
    if !profile_run.status.success() {
        return Err(RunnerError::Scenario(format!(
            "profile run exited with {:?}: {}",
            profile_run.status.code(),
            String::from_utf8_lossy(&profile_run.stderr)
        )));
    }
    combined_stdout.extend_from_slice(&profile_run.stdout);
    combined_stderr.extend_from_slice(&profile_run.stderr);
    Ok((combined_stdout, combined_stderr))
}

fn write_profile_evidence(
    context: ScenarioRunContext<'_>,
    combined_stdout: &[u8],
    combined_stderr: &[u8],
) -> Result<(), RunnerError> {
    let scenario = context.scenario;
    let target = context.target;
    let evidence_dir = context.evidence_dir;
    let stem = format!("{}.{target}.0", scenario.id);
    fs::write(evidence_dir.join(format!("{stem}.stdout")), combined_stdout)
        .map_err(RunnerError::Io)?;
    fs::write(evidence_dir.join(format!("{stem}.stderr")), combined_stderr)
        .map_err(RunnerError::Io)?;
    fs::write(
        evidence_dir.join(format!("{stem}.process-tree.json")),
        br#"{"schema":1,"reaped":true,"remaining_owned_processes":0}"#,
    )
    .map_err(RunnerError::Io)?;
    Ok(())
}

fn process_commands(scenario: &ScenarioV1) -> Result<Vec<Vec<&'static str>>, RunnerError> {
    if scenario.id == "package.startup-smoke" {
        Ok(vec![vec!["version", "--json"], vec!["self-test", "--json"]])
    } else if scenario.id.ends_with("bench") {
        Ok(vec![vec![
            "bench",
            "--json",
            "--workload",
            "plain-scroll",
            "--bytes",
            "65536",
            "--chunk-size",
            "8192",
            "--render-frames",
            "1",
            "--idle-ms",
            "1",
        ]])
    } else if scenario.id.ends_with("version") {
        Ok(vec![vec!["version", "--json"]])
    } else if scenario.id.ends_with("self-test") {
        Ok(vec![vec!["self-test", "--json"]])
    } else if scenario.id.ends_with("help") {
        Ok(vec![vec!["--help"]])
    } else if scenario.id.ends_with("doctor") {
        Ok(vec![vec!["doctor", "--json"]])
    } else {
        Err(RunnerError::NoDriver(scenario.surface))
    }
}

fn write_process_evidence(
    scenario: &ScenarioV1,
    target: &str,
    evidence_dir: &Path,
    process_ids: &[u32],
    stdout: &[u8],
    stderr: &[u8],
) -> Result<(), RunnerError> {
    let stem = format!("{}.{target}.0", scenario.id);
    fs::write(evidence_dir.join(format!("{stem}.stdout")), stdout).map_err(RunnerError::Io)?;
    fs::write(evidence_dir.join(format!("{stem}.stderr")), stderr).map_err(RunnerError::Io)?;
    let process_tree = serde_json::json!({
        "schema": 1,
        "root_process_ids": process_ids,
        "reaped": true,
        "remaining_owned_processes": 0,
    });
    fs::write(
        evidence_dir.join(format!("{stem}.process-tree.json")),
        serde_json::to_vec_pretty(&process_tree).map_err(RunnerError::Json)?,
    )
    .map_err(RunnerError::Io)
}

fn validate_required_evidence(
    scenario: &ScenarioV1,
    target: &str,
    evidence_dir: &std::path::Path,
) -> Result<(), RunnerError> {
    let stem = format!("{}.{target}.0", scenario.id);
    let mut missing = Vec::new();
    for kind in &scenario.required_evidence {
        let path = match kind {
            crate::EvidenceKind::EventLog => evidence_dir.join(format!("{stem}.ndjson")),
            crate::EvidenceKind::Stdout => evidence_dir.join(format!("{stem}.stdout")),
            crate::EvidenceKind::Stderr => evidence_dir.join(format!("{stem}.stderr")),
            crate::EvidenceKind::FinalSnapshot => {
                evidence_dir.join(format!("{stem}.final-snapshot.json"))
            }
            crate::EvidenceKind::ServerTrace => {
                evidence_dir.join(format!("{stem}.server-trace.json"))
            }
            crate::EvidenceKind::ProcessTree => {
                evidence_dir.join(format!("{stem}.process-tree.json"))
            }
            crate::EvidenceKind::CompositorLog | crate::EvidenceKind::ScreenshotOnFailure => {
                continue;
            }
        };
        if !path.is_file() {
            missing.push(path.display().to_string());
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(RunnerError::Evidence(format!(
            "required artifacts are absent: {}",
            missing.join(", ")
        )))
    }
}

fn finalize_failure_evidence(
    scenario: &ScenarioV1,
    target: &str,
    evidence_dir: &std::path::Path,
    failure: &str,
    stdout: Option<&[u8]>,
    stderr: Option<&[u8]>,
) {
    capture_failure_diagnostics(scenario, target, evidence_dir, None);
    let stem = format!("{}.{target}.0", scenario.id);
    write_if_missing(
        &evidence_dir.join(format!("{stem}.stdout")),
        stdout.unwrap_or_default(),
    );
    write_if_missing(
        &evidence_dir.join(format!("{stem}.stderr")),
        stderr.unwrap_or_default(),
    );
    write_json_if_missing(
        &evidence_dir.join(format!("{stem}.final-snapshot.json")),
        &serde_json::json!({
            "schema": 1,
            "available": false,
            "failure": failure,
        }),
    );
    write_json_if_missing(
        &evidence_dir.join(format!("{stem}.server-trace.json")),
        &serde_json::json!({
            "schema": 1,
            "available": false,
            "events": [],
            "failure": failure,
        }),
    );
    write_json_if_missing(
        &evidence_dir.join(format!("{stem}.process-tree.json")),
        &serde_json::json!({
            "schema": 1,
            "available": false,
            "root_process_ids": [],
            "reaped": false,
            "remaining_owned_processes": serde_json::Value::Null,
            "failure": failure,
        }),
    );
}

fn write_if_missing(path: &Path, contents: &[u8]) {
    if !path.is_file() {
        let _ = fs::write(path, contents);
    }
}

fn write_json_if_missing(path: &Path, value: &serde_json::Value) {
    if path.is_file() {
        return;
    }
    if let Ok(contents) = serde_json::to_vec_pretty(value) {
        let _ = fs::write(path, contents);
    }
}

fn capture_failure_diagnostics(
    scenario: &ScenarioV1,
    target: &str,
    evidence_dir: &std::path::Path,
    process_id: Option<u32>,
) {
    let stem = format!("{}.{target}.0", scenario.id);
    let screenshot = evidence_dir.join(format!("{stem}.failure-screenshot.png"));
    let compositor = evidence_dir.join(format!("{stem}.compositor.log"));
    let mut diagnostics = Vec::new();
    if target.contains("windows") {
        let script = format!(
            "$ErrorActionPreference='Stop'; Add-Type -AssemblyName System.Drawing; Add-Type -AssemblyName System.Windows.Forms; $b=[System.Windows.Forms.Screen]::PrimaryScreen.Bounds; $i=New-Object Drawing.Bitmap $b.Width,$b.Height; $g=[Drawing.Graphics]::FromImage($i); $g.CopyFromScreen($b.Location,[Drawing.Point]::Empty,$b.Size); $i.Save('{}',[Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $i.Dispose()",
            screenshot.display().to_string().replace('\'', "''")
        );
        diagnostics.push(run_failure_command(
            "powershell.exe",
            &["-NoProfile", "-NonInteractive", "-Command", &script],
        ));
    } else if target.contains("macos") {
        diagnostics.push(run_failure_command(
            "screencapture",
            &["-x", screenshot.to_string_lossy().as_ref()],
        ));
    } else {
        diagnostics.push(run_failure_command(
            "import",
            &["-window", "root", screenshot.to_string_lossy().as_ref()],
        ));
    }
    if let Ok(path) = std::env::var("RTERM_FUNCTIONAL_COMPOSITOR_LOG")
        && let Ok(bytes) = fs::read(path)
    {
        let _ = fs::write(&compositor, bytes);
    }
    if !compositor.is_file() {
        let payload = serde_json::json!({
            "target": target,
            "process_id": process_id,
            "capture_diagnostics": diagnostics,
            "display": std::env::var("DISPLAY").ok(),
            "wayland_display": std::env::var("WAYLAND_DISPLAY").ok(),
        });
        let _ = fs::write(
            compositor,
            serde_json::to_vec_pretty(&payload).unwrap_or_default(),
        );
    }
}

fn run_failure_command(program: &str, arguments: &[&str]) -> String {
    match Command::new(program).args(arguments).output() {
        Ok(output) => format!(
            "{program} exit={:?} stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ),
        Err(error) => format!("{program}: {error}"),
    }
}

fn checkpoint_name(checkpoint: &CheckpointV1) -> &'static str {
    match checkpoint {
        CheckpointV1::TerminalContains { .. } => "terminal_contains",
        CheckpointV1::Cursor { .. } => "cursor",
        CheckpointV1::TerminalMode { .. } => "terminal_mode",
        CheckpointV1::Pane { .. } => "pane",
        CheckpointV1::Overlay { .. } => "overlay",
        CheckpointV1::Transport { .. } => "transport",
        CheckpointV1::ConfigGeneration { .. } => "config_generation",
        CheckpointV1::ConfigDiagnostic { .. } => "config_diagnostic",
        CheckpointV1::HostEffect { .. } => "host_effect",
        CheckpointV1::WindowGeometry { .. } => "window_geometry",
        CheckpointV1::FileSha256 { .. } => "file_sha256",
        CheckpointV1::NetworkBytes { .. } => "network_bytes",
        CheckpointV1::ExitStatus { .. } => "exit_status",
        CheckpointV1::ResourcesZero => "resources_zero",
        CheckpointV1::RenderProbe { .. } => "render_probe",
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn phase_timeout(
    started: Instant,
    scenario_ms: u64,
    phase_ms: u64,
) -> Result<Duration, RunnerError> {
    let budget = Duration::from_millis(scenario_ms);
    let remaining = budget.checked_sub(started.elapsed()).ok_or_else(|| {
        RunnerError::Scenario(format!(
            "scenario exceeded its hard {scenario_ms}ms deadline"
        ))
    })?;
    if remaining.is_zero() {
        return Err(RunnerError::Scenario(format!(
            "scenario exceeded its hard {scenario_ms}ms deadline"
        )));
    }
    Ok(remaining.min(Duration::from_millis(phase_ms)))
}

fn scenario_timeout(scenario: &ScenarioV1, started: Instant) -> Result<Duration, RunnerError> {
    phase_timeout(
        started,
        scenario.deadlines.scenario_ms,
        scenario.deadlines.scenario_ms,
    )
}

fn startup_timeout(scenario: &ScenarioV1, started: Instant) -> Result<Duration, RunnerError> {
    phase_timeout(
        started,
        scenario.deadlines.scenario_ms,
        scenario.deadlines.startup_ms,
    )
}

fn action_timeout(scenario: &ScenarioV1, started: Instant) -> Result<Duration, RunnerError> {
    phase_timeout(
        started,
        scenario.deadlines.scenario_ms,
        scenario.deadlines.action_ms,
    )
}

fn cleanup_timeout(scenario: &ScenarioV1, started: Instant) -> Result<Duration, RunnerError> {
    phase_timeout(
        started,
        scenario.deadlines.scenario_ms,
        scenario.deadlines.cleanup_ms,
    )
}

fn capability_name(capability: Capability) -> &'static str {
    match capability {
        Capability::RealStdinPty => "real_stdin_pty",
        Capability::RealOsKeyboard => "real_os_keyboard",
        Capability::RealOsPointer => "real_os_pointer",
        Capability::SystemClipboard => "system_clipboard",
        Capability::X11 => "x11",
        Capability::Wayland => "wayland",
        Capability::MacosAccessibility => "macos_accessibility",
        Capability::BrowserChromium => "browser_chromium",
        Capability::BrowserFirefox => "browser_firefox",
        Capability::BrowserWebkit => "browser_webkit",
        Capability::SystemOpenssh => "system_openssh",
        Capability::NativeSsh => "native_ssh",
        Capability::GpuReadback => "gpu_readback",
        Capability::RealHostTerminal => "real_host_terminal",
        Capability::ProductionObserverIsolation => "production_observer_isolation",
    }
}

#[derive(Default)]
struct ParsedOptions {
    values: std::collections::BTreeMap<String, String>,
    capabilities: BTreeSet<Capability>,
}

impl ParsedOptions {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, CliError> {
        let mut parsed = Self::default();
        let mut args = args.into_iter();
        while let Some(name) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| CliError::MissingValue(name.clone()))?;
            if name == "--capability" {
                parsed.capabilities.insert(parse_capability(&value)?);
            } else if matches!(
                name.as_str(),
                "--suite"
                    | "--count"
                    | "--index"
                    | "--surface"
                    | "--scenario"
                    | "--target"
                    | "--evidence"
                    | "--app"
                    | "--fixture-bin"
                    | "--map"
                    | "--evidence-root"
            ) {
                if parsed.values.insert(name.clone(), value).is_some() {
                    return Err(CliError::DuplicateOption(name));
                }
            } else {
                return Err(CliError::UnknownOption(name));
            }
        }
        Ok(parsed)
    }

    fn required(&self, name: &'static str) -> Result<String, CliError> {
        self.values
            .get(name)
            .cloned()
            .ok_or(CliError::RequiredOption(name))
    }

    fn required_path(&self, name: &'static str) -> Result<PathBuf, CliError> {
        self.required(name).map(PathBuf::from)
    }

    fn optional_path(&self, name: &'static str) -> Option<PathBuf> {
        self.values.get(name).map(PathBuf::from)
    }

    fn required_usize(&self, name: &'static str) -> Result<usize, CliError> {
        let value = self.required(name)?;
        value
            .parse()
            .map_err(|_| CliError::InvalidNumber { name, value })
    }
}

fn parse_capability(value: &str) -> Result<Capability, CliError> {
    let quoted = serde_json::to_string(value).expect("serialize capability name");
    serde_json::from_str(&quoted).map_err(|_| CliError::InvalidCapability(value.to_owned()))
}

fn parse_surface(value: &str) -> Result<Surface, CliError> {
    let quoted = serde_json::to_string(value).expect("serialize surface name");
    serde_json::from_str(&quoted).map_err(|_| CliError::InvalidSurface(value.to_owned()))
}

#[derive(Serialize)]
struct ValidationReport {
    schema: u16,
    scenarios: usize,
    behaviors: usize,
}

#[derive(Serialize)]
struct ScenarioSummary<'a> {
    schema: u16,
    id: &'a str,
    surface: crate::Surface,
    capabilities: &'a [Capability],
    estimated_cost_ms: u64,
}

#[derive(Serialize)]
struct ShardReport {
    schema: u16,
    shards: Vec<crate::ShardAssignment>,
}

#[derive(Debug)]
enum CliError {
    MissingCommand,
    UnknownCommand(String),
    UnknownOption(String),
    MissingValue(String),
    DuplicateOption(String),
    RequiredOption(&'static str),
    InvalidNumber { name: &'static str, value: String },
    InvalidCapability(String),
    InvalidSurface(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand => {
                formatter.write_str("missing command: validate, list, shard, or run")
            }
            Self::UnknownCommand(value) => write!(formatter, "unknown command `{value}`"),
            Self::UnknownOption(value) => write!(formatter, "unknown option `{value}`"),
            Self::MissingValue(value) => write!(formatter, "missing value for `{value}`"),
            Self::DuplicateOption(value) => write!(formatter, "duplicate option `{value}`"),
            Self::RequiredOption(value) => {
                write!(formatter, "required option `{value}` is missing")
            }
            Self::InvalidNumber { name, value } => {
                write!(formatter, "invalid number `{value}` for `{name}`")
            }
            Self::InvalidCapability(value) => write!(formatter, "unknown capability `{value}`"),
            Self::InvalidSurface(value) => write!(formatter, "unknown surface `{value}`"),
        }
    }
}

#[derive(Debug)]
enum RunnerError {
    Suite(Box<crate::SuiteLoadError>),
    Json(serde_json::Error),
    Io(io::Error),
    InvalidShard(String),
    UnknownScenario(String),
    InvalidRunId(String),
    Evidence(String),
    MissingCapabilities(String),
    MissingApplication,
    MissingFixtureApplication,
    NoDriver(crate::Surface),
    Driver(String),
    Child {
        operation: String,
        source: ChildGuardError,
    },
    Scenario(String),
    Coverage(String),
}

impl RunnerError {
    fn child(operation: impl Into<String>, source: ChildGuardError) -> Self {
        Self::Child {
            operation: operation.into(),
            source,
        }
    }

    fn diagnostic_streams(&self) -> (Option<&[u8]>, Option<&[u8]>) {
        match self {
            Self::Child { source, .. } => (source.stdout(), source.stderr()),
            _ => (None, None),
        }
    }

    fn exit_code(&self) -> i32 {
        match self {
            Self::Suite(_) | Self::InvalidShard(_) | Self::UnknownScenario(_) => EXIT_INVALID_SUITE,
            Self::MissingCapabilities(_)
            | Self::MissingApplication
            | Self::MissingFixtureApplication
            | Self::Driver(_)
            | Self::Child { .. }
            | Self::Json(_)
            | Self::Io(_)
            | Self::InvalidRunId(_)
            | Self::Evidence(_) => EXIT_INFRASTRUCTURE_FAILED,
            Self::NoDriver(_) | Self::Scenario(_) | Self::Coverage(_) => EXIT_SCENARIO_FAILED,
        }
    }
}

impl fmt::Display for RunnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Suite(source) => source.fmt(formatter),
            Self::Json(source) => write!(formatter, "write JSON report: {source}"),
            Self::Io(source) => write!(formatter, "I/O failure: {source}"),
            Self::InvalidShard(source) | Self::InvalidRunId(source) => formatter.write_str(source),
            Self::UnknownScenario(id) => write!(formatter, "unknown scenario `{id}`"),
            Self::Evidence(source) => write!(formatter, "write evidence: {source}"),
            Self::MissingCapabilities(capabilities) => {
                write!(
                    formatter,
                    "required capabilities are unavailable: {capabilities}"
                )
            }
            Self::MissingApplication => {
                formatter.write_str("required option `--app` is missing for this driver")
            }
            Self::MissingFixtureApplication => {
                formatter.write_str("required option `--fixture-bin` is missing for this driver")
            }
            Self::NoDriver(surface) => write!(
                formatter,
                "no functional driver is registered for {surface:?}"
            ),
            Self::Driver(detail) => write!(formatter, "functional driver failed: {detail}"),
            Self::Child { operation, source } => {
                write!(formatter, "functional child {operation} failed: {source}")
            }
            Self::Scenario(detail) => write!(formatter, "functional scenario failed: {detail}"),
            Self::Coverage(detail) => write!(formatter, "functional coverage failed: {detail}"),
        }
    }
}

impl Error for RunnerError {}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).map_err(|error| error.to_string())?;
            u8::from_str_radix(pair, 16).map_err(|error| error.to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{
        io,
        path::Path,
        time::{Duration, Instant},
    };

    use super::{
        RunnerError, absolute_from_current, native_window_command, observer_closed_with_process,
        phase_timeout,
    };

    #[test]
    fn phase_timeout_is_capped_by_the_single_absolute_scenario_budget() {
        let started = Instant::now()
            .checked_sub(Duration::from_millis(80))
            .expect("representable test instant");
        let remaining = phase_timeout(started, 100, 75).expect("remaining budget");

        assert!(remaining <= Duration::from_millis(20));
        assert!(phase_timeout(started, 50, 75).is_err());
    }

    #[test]
    fn windows_pipe_closing_is_an_expected_observer_shutdown() {
        let error = io::Error::from_raw_os_error(232);
        let wrapped = io::Error::other("The pipe is being closed. (os error 232)");
        let wrapped_unix = io::Error::other("Broken pipe (os error 32)");

        assert!(observer_closed_with_process(&error));
        assert!(observer_closed_with_process(&wrapped));
        assert!(observer_closed_with_process(&wrapped_unix));
    }

    #[test]
    fn runner_child_paths_are_absolutized_before_an_application_changes_cwd() {
        let relative = Path::new("target/debug/rterm-functional-fixture.exe");
        let expected = std::env::current_dir()
            .expect("current directory")
            .join(relative);

        assert_eq!(
            absolute_from_current(relative).expect("absolute fixture path"),
            expected
        );
    }

    #[test]
    fn synchronized_output_requires_the_complete_payload_in_source_order() {
        assert!(super::synchronized_output_was_released(
            b"prefix-first\r\nsecond-suffix"
        ));
        assert!(super::synchronized_output_was_released(
            b"prefix-first\r\r\nsecond-suffix"
        ));
        assert!(!super::synchronized_output_was_released(b"first-only"));
        assert!(!super::synchronized_output_was_released(b"second\r\nfirst"));
    }

    #[test]
    fn forced_close_uses_an_isolated_never_prompt_configuration() {
        let (command, config) = native_window_command(
            "forced_close",
            Path::new("rterm-app"),
            Path::new("rterm-functional-fixture"),
        )
        .expect("forced-close command");
        let arguments = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let config = config.expect("forced close must isolate close confirmation policy");
        let config_path = config.path().to_string_lossy().into_owned();

        assert_eq!(arguments[0..2], ["--config-file", config_path.as_str()]);
        assert_eq!(arguments[2], "window");
        assert_eq!(
            std::fs::read_to_string(config.path()).expect("read isolated config"),
            "return { window_close_confirmation = 'NeverPrompt' }\n"
        );
    }

    #[test]
    fn every_native_window_uses_an_explicit_minimal_configuration() {
        let (command, config) = native_window_command(
            "terminal_probe",
            std::path::Path::new("rterm-app"),
            std::path::Path::new("fixture"),
        )
        .expect("build isolated native command");
        let config = config.expect("native test windows must not read user configuration");
        assert_eq!(
            std::fs::read_to_string(config.path()).expect("read isolated config"),
            "return {}\n"
        );
        let arguments: Vec<_> = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect();
        assert_eq!(arguments[0], "--config-file");
        assert_eq!(arguments[1], config.path().to_string_lossy());
    }

    #[test]
    fn child_failures_keep_bounded_redacted_streams_for_final_evidence() {
        let error = RunnerError::child(
            "test child",
            rterm_test_support::ChildGuardError::CleanupDeferred {
                operation: "test cleanup",
                source: io::Error::new(io::ErrorKind::TimedOut, "test timeout"),
                secondary: None,
                stdout: b"bounded stdout".to_vec(),
                stderr: b"bounded <redacted> stderr".to_vec(),
            },
        );

        assert_eq!(
            error.diagnostic_streams(),
            (
                Some(b"bounded stdout".as_slice()),
                Some(b"bounded <redacted> stderr".as_slice())
            )
        );
    }
}
