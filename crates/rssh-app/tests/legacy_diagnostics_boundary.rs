//! Run this target in a verified frozen consumer as well as the local selector.
#![cfg(feature = "rterm-legacy-0-1")]

use std::{process::Command, time::Duration};

use rssh_test_support::ChildGuard;

#[test]
fn legacy_diagnostic_commands_fail_without_metrics_or_startup_markers() {
    let cases: &[&[&str]] = &[
        &["bench"],
        &["doctor"],
        &["self-test"],
        &[
            "diagnostic-gui",
            "--run-id",
            "legacy-rejection",
            "--scenario",
            "empty-window",
            "--hold-ms",
            "1",
        ],
        &[
            "diagnostic-gui",
            "--run-id",
            "legacy-attribution",
            "--scenario",
            "empty-window",
            "--hold-ms",
            "1",
            "--renderer",
            "cpu",
            "--attribution-stage",
            "cpu-window",
        ],
        &[
            "diagnostic-gui",
            "--run-id",
            "legacy-font-proof",
            "--scenario",
            "empty-window",
            "--hold-ms",
            "1",
            "--renderer",
            "gpu",
            "--font-mode",
            "lazy",
            "--font-specimen",
            "cjk",
        ],
    ];
    for args in cases {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rterm"));
        command.args(*args);
        let output = ChildGuard::spawn(command, Duration::from_secs(15))
            .expect("bounded legacy diagnostic process")
            .wait()
            .expect("unsupported diagnostics must exit promptly");
        assert!(!output.status.success(), "{args:?} must fail");
        assert!(output.stdout.is_empty(), "{args:?} emitted a false report");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr).trim(),
            "error: R-Term legacy-0.1 diagnostic-tools are unsupported",
            "{args:?} must not emit startup markers or run diagnostic services"
        );
    }
}
