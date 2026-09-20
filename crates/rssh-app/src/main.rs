#![cfg_attr(
    test,
    expect(
        clippy::large_stack_arrays,
        reason = "the generated binary test harness aggregates a large compatibility test inventory"
    )
)]
#![cfg_attr(
    not(feature = "developer-full"),
    allow(
        dead_code,
        reason = "reduced production feature sets intentionally compile out command entrypoints"
    )
)]

#[cfg(feature = "diagnostic-tools")]
mod bench;
mod cli;
mod config_lifecycle;
mod diagnostic_markers;
mod diagnostics;
#[cfg(feature = "functional-test-observer")]
mod functional_observer;
mod local;
mod platform;
mod platform_fonts;
mod profiles;
#[cfg(any(test, feature = "rterm-legacy-0-1"))]
mod rterm_compat;
#[cfg(feature = "rterm-legacy-0-1")]
mod rterm_compat_gpu;
mod runtime_composition;
#[cfg(feature = "transfer-tools")]
mod scp;
#[cfg(feature = "diagnostic-tools")]
mod self_test;
#[cfg(feature = "transfer-tools")]
mod sftp;
#[cfg(feature = "ssh")]
mod ssh;
#[allow(
    dead_code,
    reason = "Task 6 wires the cfg-gated Task 5 controller; production uses only its scheduling gate"
)]
mod stage7_attribution;
mod startup_metrics;
mod terminal_input;
mod terminal_modes;
mod terminal_queries;
mod terminal_query_dcs;
mod terminal_runtime;
mod version;
mod visible_output;
// This compatibility aggregate is intentionally generated/monolithic and is
// too large for rustfmt's current allocation strategy. Keep the rest of the
// crate under the workspace-wide formatter without attempting to reformat it.
#[rustfmt::skip]
mod window;
mod window_bootstrap;
mod window_gpu;

use std::{env, process::ExitCode, time::Instant};

use cli::{AppCommand, SshOptions};
use rssh_pty::PtyExitStatus;

fn main() -> ExitCode {
    let process_started_at = Instant::now();
    match run(process_started_at) {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(process_started_at: Instant) -> Result<ExitCode, Box<dyn std::error::Error>> {
    #[cfg(feature = "functional-test-observer")]
    functional_observer::initialize_from_environment()?;
    run_command(
        cli::parse_args(env::args()).map_err(io_error)?,
        process_started_at,
    )
}

fn run_command(
    command: AppCommand,
    process_started_at: Instant,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    run_command_with_gui(command, process_started_at, &mut |options, started_at| {
        window::run_ssh_gui(options, started_at)
    })
}

// These hooks are consumed by SSH GUI dispatch when the extension is enabled.
#[cfg_attr(not(feature = "ssh"), allow(clippy::only_used_in_recursion))]
fn run_command_with_gui<F>(
    command: AppCommand,
    process_started_at: Instant,
    gui_runner: &mut F,
) -> Result<ExitCode, Box<dyn std::error::Error>>
where
    F: FnMut(&SshOptions, Instant) -> Result<(), Box<dyn std::error::Error>>,
{
    #[cfg(feature = "rterm-legacy-0-1")]
    if matches!(
        command,
        AppCommand::Bench(_)
            | AppCommand::Doctor(_)
            | AppCommand::DiagnosticGui(_)
            | AppCommand::SelfTest(_)
    ) {
        return Err(rterm_compat::unsupported_diagnostics());
    }
    match command {
        #[cfg(feature = "diagnostic-tools")]
        AppCommand::Bench(options) => {
            bench::print_bench(&options)?;
            Ok(ExitCode::SUCCESS)
        }
        #[cfg(feature = "diagnostic-tools")]
        AppCommand::Doctor(options) => {
            diagnostics::print_doctor(&options)?;
            Ok(ExitCode::SUCCESS)
        }
        #[cfg(feature = "diagnostic-tools")]
        AppCommand::DiagnosticGui(options) => {
            if options.attribution_stage.is_some() {
                window::run_attribution_diagnostic_gui(&options, process_started_at)?;
            } else {
                window::run_diagnostic_gui(&options, process_started_at)?;
            }
            Ok(ExitCode::SUCCESS)
        }
        AppCommand::Local(options) => local::run(&options).map(|status| pty_exit_code(&status)),
        AppCommand::Profile(options) => run_command_with_gui(
            profiles::load_command(&options)?,
            process_started_at,
            gui_runner,
        ),
        AppCommand::ProfileCheck(options) => {
            profiles::print_profile_check(&options)?;
            Ok(ExitCode::SUCCESS)
        }
        AppCommand::ProfileInit(options) => {
            profiles::print_profile_init(&options)?;
            Ok(ExitCode::SUCCESS)
        }
        AppCommand::ProfileList(options) => {
            profiles::print_profile_list(&options)?;
            Ok(ExitCode::SUCCESS)
        }
        AppCommand::ProfileShow(options) => {
            profiles::print_profile_show(&options)?;
            Ok(ExitCode::SUCCESS)
        }
        #[cfg(feature = "transfer-tools")]
        AppCommand::Scp(options) => scp::run(&options).map(|status| pty_exit_code(&status)),
        #[cfg(feature = "diagnostic-tools")]
        AppCommand::SelfTest(options) => {
            self_test::print_self_test(&options)?;
            Ok(ExitCode::SUCCESS)
        }
        #[cfg(feature = "transfer-tools")]
        AppCommand::Sftp(options) => sftp::run(&options).map(|status| pty_exit_code(&status)),
        #[cfg(feature = "ssh")]
        AppCommand::Ssh(options) if options.gui => {
            gui_runner(&options, process_started_at)?;
            Ok(ExitCode::SUCCESS)
        }
        #[cfg(feature = "ssh")]
        AppCommand::Ssh(options) => ssh::run(&options).map(|status| pty_exit_code(&status)),
        #[cfg(not(feature = "ssh"))]
        AppCommand::Ssh(_) => Err(feature_disabled("ssh")),
        AppCommand::Version(options) => {
            version::print_version(&options)?;
            Ok(ExitCode::SUCCESS)
        }
        AppCommand::Window(options) => {
            let composition = runtime_composition::RuntimeComposition::new();
            window::run(&options, composition)?;
            Ok(ExitCode::SUCCESS)
        }
        AppCommand::Help => {
            print!("{}", cli::help_text());
            Ok(ExitCode::SUCCESS)
        }
        #[cfg(not(feature = "diagnostic-tools"))]
        AppCommand::Bench(_)
        | AppCommand::Doctor(_)
        | AppCommand::DiagnosticGui(_)
        | AppCommand::SelfTest(_) => Err(feature_disabled("diagnostic-tools")),
        #[cfg(not(feature = "transfer-tools"))]
        AppCommand::Scp(_) | AppCommand::Sftp(_) => Err(feature_disabled("transfer-tools")),
    }
}

#[cfg(any(
    not(feature = "ssh"),
    not(feature = "diagnostic-tools"),
    not(feature = "transfer-tools")
))]
fn feature_disabled(feature: &str) -> Box<dyn std::error::Error> {
    Box::new(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        format!("this rterm build does not include the '{feature}' feature"),
    ))
}

fn pty_exit_code(status: &PtyExitStatus) -> ExitCode {
    ExitCode::from(pty_status_code(status))
}

fn pty_status_code(status: &PtyExitStatus) -> u8 {
    if status.success() {
        return 0;
    }

    u8::try_from(status.exit_code()).unwrap_or(1)
}

fn io_error(message: String) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;
    #[cfg(feature = "ssh")]
    use std::{fs, time::Duration};

    use rssh_pty::PtyExitStatus;

    use super::{pty_status_code, run_command_with_gui};

    #[test]
    fn maps_pty_success_to_process_success() {
        assert_eq!(pty_status_code(&PtyExitStatus::from_exit_code(0)), 0);
    }

    #[test]
    fn maps_pty_failure_to_process_exit_code() {
        assert_eq!(pty_status_code(&PtyExitStatus::from_exit_code(7)), 7);
    }

    #[test]
    fn maps_large_pty_failure_to_generic_process_failure() {
        assert_eq!(pty_status_code(&PtyExitStatus::from_exit_code(300)), 1);
    }

    #[cfg(feature = "rterm-legacy-0-1")]
    #[test]
    fn legacy_profile_rejects_diagnostics_before_starting_services() {
        for args in [
            vec!["rssh", "doctor"],
            vec!["rssh", "bench"],
            vec!["rssh", "self-test"],
            vec![
                "rssh",
                "diagnostic-gui",
                "--run-id",
                "legacy-reject",
                "--scenario",
                "empty-window",
                "--hold-ms",
                "1",
            ],
        ] {
            let command = crate::cli::parse_args(args).unwrap();
            let error = run_command_with_gui(command, Instant::now(), &mut |_, _| {
                panic!("unsupported diagnostics must not open a GUI")
            })
            .expect_err("frozen profile cannot produce diagnostic evidence");
            assert!(error.to_string().contains("legacy-0.1"));
            assert_eq!(
                error.downcast_ref::<std::io::Error>().unwrap().kind(),
                std::io::ErrorKind::Unsupported
            );
        }
    }

    #[cfg(not(feature = "diagnostic-tools"))]
    #[test]
    fn reduced_gui_build_reports_disabled_diagnostic_entrypoints() {
        let command = crate::cli::parse_args(["rssh", "bench"]).expect("parse bench command");
        let error = run_command_with_gui(command, Instant::now(), &mut |_, _| Ok(()))
            .expect_err("reduced GUI must reject diagnostic commands");

        assert!(error.to_string().contains("diagnostic-tools"));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn gui_dispatch_preserves_the_process_start_instant() {
        let command = crate::cli::parse_args([
            "rssh",
            "ssh",
            "--gui",
            "--host",
            "example.test",
            "--user",
            "alice",
        ])
        .expect("SSH GUI arguments should parse");
        let process_started_at = Instant::now()
            .checked_sub(Duration::from_millis(25))
            .expect("test instant should support a small subtraction");
        let mut observed = None;

        let result = run_command_with_gui(command, process_started_at, &mut |_, started_at| {
            observed = Some(started_at);
            Ok(())
        });

        assert!(result.is_ok());
        assert_eq!(observed, Some(process_started_at));
    }

    #[cfg(feature = "ssh")]
    #[test]
    fn gui_profile_dispatch_preserves_the_process_start_instant() {
        let mut file = std::env::temp_dir();
        file.push(format!("rssh-main-gui-profile-{}.toml", std::process::id()));
        fs::write(
            &file,
            r#"
[profiles.gui]
kind = "ssh"
target = "example.test"
gui = true
auth = "agent"
"#,
        )
        .expect("test profile should be written");
        let command = crate::cli::AppCommand::Profile(crate::cli::ProfileOptions {
            name: "gui".to_owned(),
            file: file.clone(),
        });
        let process_started_at = Instant::now()
            .checked_sub(Duration::from_millis(25))
            .expect("test instant should support a small subtraction");
        let mut observed = None;

        let result = run_command_with_gui(command, process_started_at, &mut |_, started_at| {
            observed = Some(started_at);
            Ok(())
        });
        let _ = fs::remove_file(file);

        assert!(result.is_ok());
        assert_eq!(observed, Some(process_started_at));
    }
}
