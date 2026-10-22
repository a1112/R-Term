use std::io::Cursor;

use rterm_functional_tests::{EvidenceEventV1, EvidenceWriter, ScenarioOutcome, ScenarioRunId};
use std::{fs, path::PathBuf};

#[test]
fn evidence_is_versioned_monotonic_ndjson_without_secrets() {
    let run_id = ScenarioRunId::new("window.local.interaction", "windows-x86_64", 0).unwrap();
    let mut output = Vec::new();
    let mut writer = EvidenceWriter::new(&mut output, run_id.clone());
    writer
        .record(EvidenceEventV1::scenario_started(100, ["real_os_keyboard"]))
        .unwrap();
    writer
        .record(EvidenceEventV1::action_finished(
            150,
            0,
            "type_text",
            "accepted",
        ))
        .unwrap();
    writer
        .record(EvidenceEventV1::scenario_finished(
            220,
            ScenarioOutcome::Passed,
        ))
        .unwrap();

    let events = EvidenceEventV1::read_ndjson(Cursor::new(&output)).unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].schema, 1);
    assert_eq!(events[0].sequence, 1);
    assert_eq!(events[2].sequence, 3);
    assert_eq!(events[0].run_id, run_id);
    assert!(!String::from_utf8(output).unwrap().contains("token"));
}

#[test]
fn evidence_writer_rejects_decreasing_monotonic_time() {
    let run_id = ScenarioRunId::new("cli.version", "windows-x86_64", 0).unwrap();
    let mut output = Vec::new();
    let mut writer = EvidenceWriter::new(&mut output, run_id);
    writer
        .record(EvidenceEventV1::scenario_started(20, []))
        .unwrap();

    let error = writer
        .record(EvidenceEventV1::scenario_finished(
            19,
            ScenarioOutcome::Failed,
        ))
        .unwrap_err();
    assert!(error.to_string().contains("monotonic"));
}

#[test]
fn runner_enforces_declared_artifacts_and_captures_failure_diagnostics_without_retries() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source =
        fs::read_to_string(root.join("crates/rterm-functional-tests/src/runner.rs")).unwrap();
    for contract in [
        "validate_required_evidence",
        "capture_failure_diagnostics",
        "finalize_failure_evidence",
        ".failure-screenshot.png",
        ".compositor.log",
        "ScreenshotOnFailure",
        "screencapture",
        "import",
        "CopyFromScreen",
    ] {
        assert!(
            source.contains(contract),
            "missing failure evidence contract {contract}"
        );
    }
    assert!(!source.contains("retry_scenario"));
}
