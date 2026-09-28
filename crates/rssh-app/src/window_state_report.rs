use std::{collections::BTreeSet, fmt::Write as _};

use rssh_core::app_shell::{PaneLaunch, PaneLaunchDomain, SplitDirection};
use serde::{Deserialize, Serialize};

use crate::cli::WindowOptions;

use super::NativeWindowApp;

const WINDOW_STATE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowStateFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct WindowStateSnapshot {
    pub(super) schema_version: u32,
    pub(super) window_id: u64,
    pub(super) title: String,
    pub(super) terminal_dimensions: WindowStateDimensions,
    pub(super) active_workspace_id: u64,
    pub(super) workspaces: Vec<WindowStateWorkspace>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct WindowStateDimensions {
    columns: u16,
    rows: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct WindowStateWorkspace {
    pub(super) id: u64,
    pub(super) index: usize,
    pub(super) name: String,
    pub(super) active: bool,
    pub(super) tabs: Vec<WindowStateTab>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct WindowStateTab {
    pub(super) id: u64,
    pub(super) index: usize,
    pub(super) active: bool,
    pub(super) title: Option<String>,
    pub(super) zoomed_pane_id: Option<u64>,
    pub(super) panes: Vec<WindowStatePane>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct WindowStatePane {
    pub(super) id: u64,
    pub(super) index: usize,
    pub(super) active: bool,
    pub(super) title: Option<String>,
    pub(super) launch: WindowStateLaunch,
    pub(super) dimensions: WindowStateDimensions,
    pub(super) split: Option<WindowStateSplit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct WindowStateLaunch {
    domain: String,
    program: String,
    args: Vec<String>,
    cwd: Option<String>,
    environment_keys: Vec<String>,
    environment_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct WindowStateSplit {
    pub(super) source_pane_id: u64,
    pub(super) direction: String,
    pub(super) source_size_delta: i16,
}

impl WindowStateSnapshot {
    pub(super) fn capture(app: &NativeWindowApp) -> Self {
        let active_workspace_id = app.app_shell.active_workspace_id();
        let startup_dimensions = rssh_core::TerminalSize::new(app.initial_cols, app.initial_rows);
        let configured_environment_keys = app
            .pane_environment_variables()
            .into_keys()
            .collect::<Vec<_>>();
        let workspaces = app
            .app_shell
            .workspaces()
            .iter()
            .enumerate()
            .map(|(workspace_index, workspace)| {
                let workspace_active = workspace.id() == active_workspace_id;
                let active_tab_id = workspace.active_tab_id();
                let tabs = workspace
                    .tabs()
                    .iter()
                    .enumerate()
                    .map(|(tab_index, tab)| {
                        let tab_active = tab.id() == active_tab_id;
                        let active_pane_id = tab.active_pane_id();
                        let layout =
                            app.pane_render_layout_for_tab_at_size(tab, startup_dimensions);
                        let panes = tab
                            .panes()
                            .iter()
                            .enumerate()
                            .map(|(pane_index, pane)| {
                                let dimensions = layout
                                    .panes
                                    .iter()
                                    .find(|rect| rect.pane_id == pane.id())
                                    .map_or(startup_dimensions, |rect| {
                                        rssh_core::TerminalSize::new(rect.columns, rect.rows)
                                    });
                                WindowStatePane {
                                    id: pane.id().get(),
                                    index: pane_index,
                                    active: pane.id() == active_pane_id,
                                    title: app.pane_title(pane.id()),
                                    launch: WindowStateLaunch::capture(
                                        pane.launch(),
                                        app.default_cwd.as_deref(),
                                        &configured_environment_keys,
                                    ),
                                    dimensions: WindowStateDimensions {
                                        columns: dimensions.columns,
                                        rows: dimensions.rows,
                                    },
                                    split: pane.split().map(WindowStateSplit::capture),
                                }
                            })
                            .collect();
                        WindowStateTab {
                            id: tab.id().get(),
                            index: tab_index,
                            active: tab_active,
                            title: tab
                                .title()
                                .map(str::to_owned)
                                .or_else(|| app.tab_title_for_tab(tab)),
                            zoomed_pane_id: tab.zoomed_pane_id().map(rssh_core::PaneId::get),
                            panes,
                        }
                    })
                    .collect();
                WindowStateWorkspace {
                    id: workspace.id().get(),
                    index: workspace_index,
                    name: workspace.name().to_owned(),
                    active: workspace_active,
                    tabs,
                }
            })
            .collect();

        Self {
            schema_version: WINDOW_STATE_SCHEMA_VERSION,
            window_id: app.app_window_id.get(),
            title: app.window_title.clone(),
            terminal_dimensions: WindowStateDimensions {
                columns: startup_dimensions.columns,
                rows: startup_dimensions.rows,
            },
            active_workspace_id: active_workspace_id.get(),
            workspaces,
        }
    }

    fn text_report(&self) -> String {
        let mut report = String::new();
        let _ = writeln!(report, "R-SSH state schema_version={}", self.schema_version);
        let _ = writeln!(
            report,
            "window id={} title={:?} active_workspace_id={} dimensions={}x{}",
            self.window_id,
            self.title,
            self.active_workspace_id,
            self.terminal_dimensions.columns,
            self.terminal_dimensions.rows
        );
        for workspace in &self.workspaces {
            let _ = writeln!(
                report,
                "workspace[{}] id={} name={:?} active={}",
                workspace.index, workspace.id, workspace.name, workspace.active
            );
            for tab in &workspace.tabs {
                let _ = writeln!(
                    report,
                    "  tab[{}] id={} active={} title={} zoomed_pane_id={}",
                    tab.index,
                    tab.id,
                    tab.active,
                    optional_debug_string(tab.title.as_deref()),
                    optional_u64(tab.zoomed_pane_id)
                );
                for pane in &tab.panes {
                    let split = pane.split.as_ref().map_or_else(
                        || "none".to_owned(),
                        |split| {
                            format!(
                                "{}:source={}:delta={}",
                                split.direction, split.source_pane_id, split.source_size_delta
                            )
                        },
                    );
                    let _ = writeln!(
                        report,
                        "    pane[{}] id={} active={} title={} dimensions={}x{} \
                         launch.domain={} launch.program={:?} launch.args={:?} \
                         launch.cwd={} env_keys={:?} env_count={} split={}",
                        pane.index,
                        pane.id,
                        pane.active,
                        optional_debug_string(pane.title.as_deref()),
                        pane.dimensions.columns,
                        pane.dimensions.rows,
                        pane.launch.domain,
                        pane.launch.program,
                        pane.launch.args,
                        optional_debug_string(pane.launch.cwd.as_deref()),
                        pane.launch.environment_keys,
                        pane.launch.environment_count,
                        split
                    );
                }
            }
        }
        report
    }
}

impl WindowStateLaunch {
    fn capture(
        launch: &PaneLaunch,
        default_cwd: Option<&str>,
        configured_environment_keys: &[String],
    ) -> Self {
        let environment_keys = configured_environment_keys
            .iter()
            .cloned()
            .chain(launch.environment().keys().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let environment_count = environment_keys.len();
        let (domain, program, args, cwd) = match launch.domain() {
            PaneLaunchDomain::Local => (
                "local",
                launch.program().to_owned(),
                launch.args().to_vec(),
                launch.cwd().or(default_cwd).map(str::to_owned),
            ),
            PaneLaunchDomain::Ssh(ssh) => (
                "ssh",
                "ssh".to_owned(),
                ssh.remote_command().to_vec(),
                None,
            ),
        };
        Self {
            domain: domain.to_owned(),
            program,
            args,
            cwd,
            environment_keys,
            environment_count,
        }
    }
}

impl WindowStateSplit {
    fn capture(split: rssh_core::app_shell::PaneSplit) -> Self {
        Self {
            source_pane_id: split.source_pane.get(),
            direction: split_direction_name(split.direction).to_owned(),
            source_size_delta: split.source_size_delta,
        }
    }
}

fn split_direction_name(direction: SplitDirection) -> &'static str {
    match direction {
        SplitDirection::Left => "left",
        SplitDirection::Right => "right",
        SplitDirection::Up => "up",
        SplitDirection::Down => "down",
    }
}

fn optional_debug_string(value: Option<&str>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| format!("{value:?}"))
}

fn optional_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

pub(super) fn render_window_state(
    app: &NativeWindowApp,
    format: WindowStateFormat,
) -> Result<String, serde_json::Error> {
    let snapshot = WindowStateSnapshot::capture(app);
    match format {
        WindowStateFormat::Text => Ok(snapshot.text_report()),
        WindowStateFormat::Json => serde_json::to_string(&snapshot),
    }
}

pub(super) fn render_requested_window_state(
    options: &WindowOptions,
    app: &NativeWindowApp,
) -> Result<Option<(WindowStateFormat, String)>, serde_json::Error> {
    let format = if options.state_json {
        Some(WindowStateFormat::Json)
    } else if options.state {
        Some(WindowStateFormat::Text)
    } else {
        None
    };
    format
        .map(|format| render_window_state(app, format).map(|report| (format, report)))
        .transpose()
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};

    use crate::cli::WindowConfigOptions;
    use rssh_core::app_shell::{AppAction, PaneLaunch, SplitDirection};

    use super::super::*;
    use super::{
        WindowStateFormat, WindowStateSnapshot, render_requested_window_state, render_window_state,
    };

    fn startup_options(config: WindowConfigOptions) -> WindowOptions {
        WindowOptions {
            renderer: crate::cli::RendererMode::Auto,
            display_backend: crate::cli::DisplayBackend::Auto,
            config,
            frame_limit: None,
            workspace: None,
            window_class: None,
            position: None,
            osc52_policy: Osc52Policy::default(),
            metrics: false,
            metrics_json: false,
            state: true,
            state_json: false,
            command: rssh_pty::PtyCommand::default_shell(),
            log: None,
        }
    }

    fn isolated_discovery() -> ConfigDiscoveryInputs {
        ConfigDiscoveryInputs {
            is_windows: false,
            is_unix: false,
            current_exe: None,
            home_dir: None,
            xdg_config_home: None,
            xdg_config_dirs: Vec::new(),
            environment_config_file: None,
        }
    }

    fn populated_app() -> NativeWindowApp {
        let command = rssh_pty::PtyCommand::new("root-shell")
            .with_args(["--login"])
            .with_cwd("root-cwd")
            .with_env("VISIBLE_KEY", "top-secret-value");
        let mut app = NativeWindowApp::new_with_workspace(None, command, Some("primary"));
        let first = app.app_shell.active_pane_id();
        app.app_shell
            .apply_action(AppAction::SplitPaneWithSize {
                pane: first,
                direction: SplitDirection::Right,
                launch: Some(
                    PaneLaunch::local("worker")
                        .with_args(["--task"])
                        .with_cwd("worker-cwd")
                        .with_environment([
                            ("Z_SECRET", "never-print-z"),
                            ("A_VISIBLE", "never-print-a"),
                        ]),
                ),
                source_size_delta: 7,
            })
            .unwrap();
        let split = app.app_shell.active_pane_id();
        app.app_shell
            .apply_action(AppAction::SetPaneZoomState {
                pane: split,
                zoomed: true,
            })
            .unwrap();
        let first_tab = app.app_shell.active_tab_id();
        app.app_shell
            .apply_action(AppAction::SetTabTitle {
                tab: first_tab,
                title: "build".to_owned(),
            })
            .unwrap();
        app.app_shell
            .apply_action(AppAction::NewTab {
                launch: Some(PaneLaunch::local("logs")),
            })
            .unwrap();
        app.app_shell
            .apply_action(AppAction::NewWorkspace {
                name: "secondary".to_owned(),
                launch: Some(PaneLaunch::local("review")),
            })
            .unwrap();
        app
    }

    #[test]
    fn state_snapshot_covers_all_shell_entries_in_stable_order() {
        let app = populated_app();
        let snapshot = WindowStateSnapshot::capture(&app);

        assert_eq!(snapshot.schema_version, 1);
        assert_eq!(snapshot.workspaces.len(), 2);
        assert_eq!(
            snapshot
                .workspaces
                .iter()
                .map(|workspace| (workspace.index, workspace.id))
                .collect::<Vec<_>>(),
            [(0, 1), (1, 2)]
        );
        assert_eq!(snapshot.workspaces[0].name, "primary");
        assert_eq!(snapshot.workspaces[0].index, 0);
        assert!(!snapshot.workspaces[0].active);
        assert_eq!(snapshot.workspaces[0].tabs.len(), 2);
        assert_eq!(
            snapshot.workspaces[0]
                .tabs
                .iter()
                .map(|tab| (tab.index, tab.id, tab.active))
                .collect::<Vec<_>>(),
            [(0, 1, false), (1, 2, true)]
        );
        assert_eq!(
            snapshot.workspaces[0].tabs[0].title.as_deref(),
            Some("build")
        );
        assert_eq!(snapshot.workspaces[0].tabs[0].panes.len(), 2);
        assert_eq!(
            snapshot.workspaces[0].tabs[0]
                .panes
                .iter()
                .map(|pane| (pane.index, pane.id, pane.active))
                .collect::<Vec<_>>(),
            [(0, 1, false), (1, 2, true)]
        );
        assert_eq!(
            snapshot.workspaces[0].tabs[1]
                .panes
                .iter()
                .map(|pane| (pane.index, pane.id, pane.active))
                .collect::<Vec<_>>(),
            [(0, 3, true)]
        );
        assert_eq!(
            snapshot.workspaces[0].tabs[0].zoomed_pane_id,
            Some(snapshot.workspaces[0].tabs[0].panes[1].id)
        );
        let split = snapshot.workspaces[0].tabs[0].panes[1]
            .split
            .as_ref()
            .unwrap();
        assert_eq!(split.direction, "right");
        assert_eq!(split.source_size_delta, 7);
        assert_eq!(snapshot.workspaces[1].name, "secondary");
        assert!(snapshot.workspaces[1].active);
        assert_eq!(
            snapshot.workspaces[1]
                .tabs
                .iter()
                .map(|tab| (tab.index, tab.id, tab.active))
                .collect::<Vec<_>>(),
            [(0, 3, true)]
        );
        assert_eq!(
            snapshot.workspaces[1].tabs[0]
                .panes
                .iter()
                .map(|pane| (pane.index, pane.id, pane.active))
                .collect::<Vec<_>>(),
            [(0, 4, true)]
        );
    }

    #[test]
    fn state_json_round_trips_and_never_exposes_environment_values() {
        let app = populated_app();
        let report = render_window_state(&app, WindowStateFormat::Json).unwrap();
        let decoded: WindowStateSnapshot = serde_json::from_str(&report).unwrap();

        assert_eq!(decoded, WindowStateSnapshot::capture(&app));
        assert!(report.contains("\"environment_keys\":[\"A_VISIBLE\",\"Z_SECRET\"]"));
        assert!(report.contains("\"environment_count\":2"));
        assert!(!report.contains("never-print"));
        assert!(!report.contains("top-secret-value"));
    }

    #[test]
    fn state_text_is_readable_deterministic_and_uses_the_same_snapshot() {
        let app = populated_app();
        let first = render_window_state(&app, WindowStateFormat::Text).unwrap();
        let second = render_window_state(&app, WindowStateFormat::Text).unwrap();

        assert_eq!(first, second);
        assert!(first.starts_with("R-SSH state schema_version=1\n"));
        assert!(first.contains("workspace[0] id=1 name=\"primary\" active=false"));
        assert!(first.contains("tab[0] id=1 active=false title=\"build\""));
        assert!(first.contains("pane[1] id=2 active=true"));
        assert!(first.contains("split=right:source=1:delta=7"));
        assert!(first.contains("env_keys=[\"A_VISIBLE\", \"Z_SECRET\"] env_count=2"));
        assert!(!first.contains("never-print"));
    }

    #[test]
    fn configured_default_state_is_reportable_without_window_or_pty() {
        let mut options = startup_options(WindowConfigOptions {
            skip_config: true,
            config_file: None,
            config_overrides: Vec::new(),
        });
        let configured = configured_startup_app_for_test(&options, isolated_discovery()).unwrap();

        let requested = render_requested_window_state(&options, &configured.app)
            .unwrap()
            .expect("text state report requested");
        assert_eq!(requested.0, WindowStateFormat::Text);
        assert!(requested.1.starts_with("R-SSH state schema_version=1\n"));
        options.state = false;
        assert!(
            render_requested_window_state(&options, &configured.app)
                .unwrap()
                .is_none()
        );
        options.state_json = true;
        let requested = render_requested_window_state(&options, &configured.app)
            .unwrap()
            .expect("json state report requested");
        assert_eq!(requested.0, WindowStateFormat::Json);
        assert_eq!(
            serde_json::from_str::<WindowStateSnapshot>(&requested.1).unwrap(),
            WindowStateSnapshot::capture(&configured.app)
        );

        let configured = configured_startup_app_for_test(
            &startup_options(WindowConfigOptions {
                skip_config: true,
                config_file: None,
                config_overrides: Vec::new(),
            }),
            isolated_discovery(),
        )
        .unwrap();

        assert!(configured.app.window.is_none());
        assert!(configured.app.session.is_none());
        assert!(configured.app.writer.is_none());
        assert!(configured.app.reader_thread.is_none());
        assert!(configured.app.pane_runtimes.is_empty());

        let report = render_window_state(&configured.app, WindowStateFormat::Json).unwrap();
        let snapshot: WindowStateSnapshot = serde_json::from_str(&report).unwrap();
        assert_eq!(snapshot.workspaces.len(), 1);
        assert_eq!(snapshot.workspaces[0].name, "default");
        assert!(snapshot.workspaces[0].active);
        assert_eq!(snapshot.workspaces[0].tabs.len(), 1);
        assert_eq!(snapshot.workspaces[0].tabs[0].panes.len(), 1);
    }

    #[test]
    fn configured_state_uses_cli_default_workspace_prog_cwd_and_redacts_values() {
        let options = startup_options(WindowConfigOptions {
            skip_config: true,
            config_file: None,
            config_overrides: vec![
                ("default_workspace".to_owned(), "'reports'".to_owned()),
                (
                    "default_prog".to_owned(),
                    "{ 'configured-shell', '--login' }".to_owned(),
                ),
                ("default_cwd".to_owned(), "'configured-cwd'".to_owned()),
                (
                    "set_environment_variables".to_owned(),
                    "{ REPORT_SECRET = 'override-secret-value' }".to_owned(),
                ),
            ],
        });
        let configured = configured_startup_app_for_test(&options, isolated_discovery()).unwrap();

        let snapshot = WindowStateSnapshot::capture(&configured.app);
        let pane = &snapshot.workspaces[0].tabs[0].panes[0];
        assert_eq!(snapshot.workspaces[0].name, "reports");
        assert_eq!(pane.launch.program, "configured-shell");
        assert_eq!(pane.launch.args, ["--login"]);
        assert_eq!(pane.launch.cwd.as_deref(), Some("configured-cwd"));

        let report = render_window_state(&configured.app, WindowStateFormat::Json).unwrap();
        assert!(report.contains("REPORT_SECRET"));
        assert!(!report.contains("override-secret-value"));
        assert!(configured.app.window.is_none());
        assert!(configured.app.session.is_none());
    }

    #[test]
    fn configured_state_uses_file_default_workspace_prog_and_dimensions() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static NEXT_CONFIG: AtomicUsize = AtomicUsize::new(0);
        let config_dir = std::env::temp_dir().join(format!(
            "rssh-state-file-config-{}-{}",
            std::process::id(),
            NEXT_CONFIG.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&config_dir).unwrap();
        let config_file = config_dir.join("wezterm.lua");
        std::fs::write(
            &config_file,
            "return {
                default_workspace = 'file-space',
                default_prog = { 'file-shell', '--login' },
                initial_cols = 90,
                initial_rows = 28,
            }",
        )
        .unwrap();
        let configured = configured_startup_app_for_test(
            &startup_options(WindowConfigOptions {
                skip_config: false,
                config_file: Some(config_file),
                config_overrides: Vec::new(),
            }),
            isolated_discovery(),
        )
        .unwrap();

        let snapshot = WindowStateSnapshot::capture(&configured.app);
        assert_eq!(snapshot.workspaces[0].name, "file-space");
        assert_eq!(
            snapshot.workspaces[0].tabs[0].panes[0].launch.program,
            "file-shell"
        );
        assert_eq!(
            snapshot.workspaces[0].tabs[0].panes[0].launch.args,
            ["--login"]
        );
        assert_eq!(
            snapshot.terminal_dimensions,
            super::WindowStateDimensions {
                columns: 90,
                rows: 28
            }
        );
        assert!(configured.app.window.is_none());
        assert!(configured.app.session.is_none());
        std::fs::remove_dir_all(config_dir).unwrap();
    }

    #[test]
    fn configured_initial_grid_drives_json_text_and_nested_split_dimensions() {
        let mut configured = configured_startup_app_for_test(
            &startup_options(WindowConfigOptions {
                skip_config: true,
                config_file: None,
                config_overrides: vec![
                    ("initial_cols".to_owned(), "100".to_owned()),
                    ("initial_rows".to_owned(), "30".to_owned()),
                ],
            }),
            isolated_discovery(),
        )
        .unwrap();
        let root = configured.app.app_shell.active_pane_id();
        configured
            .app
            .app_shell
            .apply_action(AppAction::SplitPane {
                pane: root,
                direction: SplitDirection::Right,
                launch: Some(PaneLaunch::local("right")),
            })
            .unwrap();
        let right = configured.app.app_shell.active_pane_id();
        configured
            .app
            .app_shell
            .apply_action(AppAction::SplitPane {
                pane: right,
                direction: SplitDirection::Down,
                launch: Some(PaneLaunch::local("bottom-right")),
            })
            .unwrap();

        let snapshot = WindowStateSnapshot::capture(&configured.app);
        assert_eq!(
            snapshot.terminal_dimensions,
            super::WindowStateDimensions {
                columns: 100,
                rows: 30
            }
        );
        let panes = &snapshot.workspaces[0].tabs[0].panes;
        assert_eq!(
            panes
                .iter()
                .map(|pane| (pane.id, pane.dimensions.columns, pane.dimensions.rows))
                .collect::<Vec<_>>(),
            [(1, 49, 30), (2, 50, 14), (3, 50, 15)]
        );

        let json = render_window_state(&configured.app, WindowStateFormat::Json).unwrap();
        assert_eq!(
            serde_json::from_str::<WindowStateSnapshot>(&json).unwrap(),
            snapshot
        );
        let text = render_window_state(&configured.app, WindowStateFormat::Text).unwrap();
        assert!(text.contains("dimensions=100x30"));
        assert!(text.contains("pane[0] id=1 active=false title=null dimensions=49x30"));
        assert!(text.contains("pane[1] id=2 active=false title=null dimensions=50x14"));
        assert!(text.contains("pane[2] id=3 active=true title=null dimensions=50x15"));
        assert!(configured.app.window.is_none());
        assert!(configured.app.session.is_none());
    }

    #[derive(Default)]
    struct StateOutputWriter {
        bytes: Vec<u8>,
        writes: usize,
        flushes: usize,
        write_error: Option<io::ErrorKind>,
    }

    impl Write for StateOutputWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.writes += 1;
            if let Some(kind) = self.write_error {
                return Err(io::Error::new(kind, "injected write failure"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flushes += 1;
            Ok(())
        }
    }

    fn configured_report(format: WindowStateFormat, report: &str) -> ConfiguredWindowStateReport {
        ConfiguredWindowStateReport {
            diagnostic: None,
            format,
            report: report.to_owned(),
        }
    }

    #[test]
    fn state_stdout_writes_once_flushes_and_propagates_broken_pipe() {
        let mut json_writer = StateOutputWriter::default();
        write_configured_window_state_report(
            &configured_report(WindowStateFormat::Json, "{\"schema_version\":1}"),
            &mut json_writer,
        )
        .unwrap();
        assert_eq!(json_writer.writes, 1);
        assert_eq!(json_writer.flushes, 1);
        assert_eq!(json_writer.bytes, b"{\"schema_version\":1}\n");

        let mut text_writer = StateOutputWriter::default();
        write_configured_window_state_report(
            &configured_report(WindowStateFormat::Text, "R-SSH state\n"),
            &mut text_writer,
        )
        .unwrap();
        assert_eq!(text_writer.writes, 1);
        assert_eq!(text_writer.flushes, 1);
        assert_eq!(text_writer.bytes, b"R-SSH state\n");

        let mut broken = StateOutputWriter {
            write_error: Some(io::ErrorKind::BrokenPipe),
            ..StateOutputWriter::default()
        };
        let error = write_configured_window_state_report(
            &configured_report(WindowStateFormat::Json, "{}"),
            &mut broken,
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
        assert_eq!(broken.writes, 1);
        assert_eq!(broken.flushes, 0);
    }

    #[test]
    fn state_report_thread_seam_maps_spawn_panic_and_inner_errors() {
        let spawn = resolve_configured_window_state_report_thread(Err(io::Error::new(
            io::ErrorKind::OutOfMemory,
            "spawn failed",
        )))
        .unwrap_err();
        assert_eq!(spawn.kind(), io::ErrorKind::OutOfMemory);
        assert!(spawn.to_string().contains("failed to start state reporter"));

        let panic =
            resolve_configured_window_state_report_thread(Ok(Err(Box::new("panic")))).unwrap_err();
        assert_eq!(panic.kind(), io::ErrorKind::Other);
        assert!(panic.to_string().contains("state reporter panicked"));

        let inner =
            resolve_configured_window_state_report_thread(Ok(Ok(Err("inner failure".to_owned()))))
                .unwrap_err();
        assert_eq!(inner.kind(), io::ErrorKind::Other);
        assert!(inner.to_string().contains("inner failure"));
    }
}
