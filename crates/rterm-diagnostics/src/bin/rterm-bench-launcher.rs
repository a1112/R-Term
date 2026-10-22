use std::process::ExitCode;

use rterm_diagnostics::{LAUNCHER_USAGE, LauncherCliError, LauncherOptions, execute_launcher};

fn main() -> ExitCode {
    match LauncherOptions::parse(std::env::args()) {
        Err(LauncherCliError::HelpRequested) => {
            println!("{LAUNCHER_USAGE}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
        Ok(options) => {
            let execution = execute_launcher(&options);
            let serialized = if options.json {
                serde_json::to_string(&execution.result)
            } else {
                serde_json::to_string_pretty(&execution.result)
            };
            match serialized {
                Ok(json) => println!("{json}"),
                Err(error) => {
                    eprintln!("failed to serialize diagnostics result: {error}");
                    return ExitCode::from(2);
                }
            }
            if execution.success {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
    }
}
