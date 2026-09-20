use std::error::Error;

use rssh_pty::PtyBackend;
use serde::Serialize;

use crate::cli::VersionOptions;

#[derive(Debug, PartialEq, Eq, Serialize)]
struct VersionReport {
    name: &'static str,
    version: &'static str,
    target: String,
    console: bool,
    pty_backend: &'static str,
    native_ssh_backend: &'static str,
}

pub fn print_version(options: &VersionOptions) -> Result<(), Box<dyn Error>> {
    if options.json {
        println!("{}", version_report_json()?);
        return Ok(());
    }

    for line in version_text_lines() {
        println!("{line}");
    }

    Ok(())
}

pub fn version_report_json() -> Result<String, Box<dyn Error>> {
    Ok(serde_json::to_string(&version_report())?)
}

pub fn version_text_lines() -> Vec<String> {
    let report = version_report();

    vec![
        format!("{} version={}", report.name, report.version),
        format!("target={}", report.target),
        format!("console={}", report.console),
        format!("pty_backend={}", report.pty_backend),
        format!("native_ssh_backend={}", report.native_ssh_backend),
    ]
}

fn version_report() -> VersionReport {
    VersionReport {
        name: "rterm",
        version: env!("CARGO_PKG_VERSION"),
        target: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        console: true,
        pty_backend: pty_backend_name(PtyBackend::current_platform()),
        native_ssh_backend: if cfg!(feature = "ssh") {
            "russh"
        } else {
            "disabled"
        },
    }
}

const fn pty_backend_name(backend: PtyBackend) -> &'static str {
    match backend {
        PtyBackend::WindowsConpty => "windows-conpty",
        PtyBackend::UnixPty => "unix-pty",
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_report_json_includes_console_release_identity() {
        let json = super::version_report_json().unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["name"], "rterm");
        assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["console"], true);
        let expected_pty_backend = if cfg!(windows) {
            "windows-conpty"
        } else {
            "unix-pty"
        };

        assert_eq!(value["pty_backend"], expected_pty_backend);
        assert_eq!(
            value["native_ssh_backend"],
            if cfg!(feature = "ssh") {
                "russh"
            } else {
                "disabled"
            }
        );
        assert!(!value["target"].as_str().unwrap().is_empty());
    }

    #[test]
    fn version_text_includes_version_and_backends() {
        let lines = super::version_text_lines();

        assert!(lines.iter().any(|line| line.contains("rterm")));
        assert!(lines.iter().any(|line| line.contains("version=")));
        assert!(lines.iter().any(|line| line.contains("pty_backend=")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains(if cfg!(feature = "ssh") {
                    "native_ssh_backend=russh"
                } else {
                    "native_ssh_backend=disabled"
                }))
        );
    }
}
