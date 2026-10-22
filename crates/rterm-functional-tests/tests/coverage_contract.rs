use std::{fs, io::Cursor};

use rterm_functional_tests::{
    BehaviorEvidenceMapV1, CoverageInputs, EvidenceEventV1, EvidenceWriter, FunctionalSuite,
    ScenarioOutcome, ScenarioRunId, verify_behavior_coverage,
};

fn suite() -> tempfile::TempDir {
    let root = tempfile::TempDir::new().unwrap();
    fs::create_dir(root.path().join("scenarios")).unwrap();
    fs::write(
        root.path().join("behaviors.toml"),
        r#"schema = 1
[[behaviors]]
id = "BHV-CLI"
subsystem = "cli"
summary = "CLI behavior"
surfaces = ["console"]
[[behaviors]]
id = "BHV-WEB"
subsystem = "web"
summary = "Web behavior"
surfaces = ["web"]
[[behaviors]]
id = "BHV-PARSER"
subsystem = "terminal"
summary = "Protocol detail"
surfaces = ["console"]
[[behaviors]]
id = "BHV-TERM-JOURNEY"
subsystem = "terminal"
summary = "Terminal real entry journey"
surfaces = ["console"]
"#,
    )
    .unwrap();
    fs::write(
        root.path().join("scenarios/cli.toml"),
        r#"schema = 1
id = "cli.real"
behavior_ids = ["BHV-CLI", "BHV-TERM-JOURNEY"]
surface = "console"
fixture = "version"
estimated_cost_ms = 100
actions = [{ type = "finish" }]
checkpoints = [{ type = "exit_status", code = 0 }]
required_evidence = ["event_log"]
"#,
    )
    .unwrap();
    fs::write(
        root.path().join("scenarios/web.toml"),
        r#"schema = 1
id = "web.real"
behavior_ids = ["BHV-WEB"]
surface = "web"
fixture = "terminal_probe"
estimated_cost_ms = 100
actions = [{ type = "finish" }]
checkpoints = [{ type = "resources_zero" }]
required_evidence = ["event_log"]
"#,
    )
    .unwrap();
    fs::write(
        root.path().join("scenarios/parser.toml"),
        r#"schema = 1
id = "parser.detail"
behavior_ids = ["BHV-PARSER"]
surface = "console"
fixture = "frozen_trace"
estimated_cost_ms = 100
actions = [{ type = "finish" }]
checkpoints = [{ type = "resources_zero" }]
required_evidence = ["event_log"]
"#,
    )
    .unwrap();
    root
}

fn passed_scenario(id: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    let run_id = ScenarioRunId::new(id, "test-target", 0).unwrap();
    let mut writer = EvidenceWriter::new(&mut bytes, run_id);
    writer
        .record(EvidenceEventV1::scenario_started(0, []))
        .unwrap();
    writer
        .record(EvidenceEventV1::scenario_finished(
            1,
            ScenarioOutcome::Passed,
        ))
        .unwrap();
    drop(writer);
    bytes
}

fn passed_scenario_with_behaviors(id: &str, behaviors: &[&str]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let run_id = ScenarioRunId::new(id, "test-target", 0).unwrap();
    let mut writer = EvidenceWriter::new(&mut bytes, run_id);
    writer
        .record(EvidenceEventV1::scenario_started(0, []))
        .unwrap();
    for behavior in behaviors {
        writer
            .record(EvidenceEventV1::behavior_observed(0, behavior, "asserted"))
            .unwrap();
    }
    writer
        .record(EvidenceEventV1::scenario_finished(
            1,
            ScenarioOutcome::Passed,
        ))
        .unwrap();
    drop(writer);
    bytes
}

#[test]
fn coverage_requires_runtime_presence_and_success_not_just_declared_test_names() {
    let root = suite();
    let suite = FunctionalSuite::load(root.path()).unwrap();
    let map = BehaviorEvidenceMapV1::from_toml(
        r#"schema = 1
[[evidence]]
behavior_id = "BHV-PARSER"
source = "libtest"
identity = "parser::split_utf8"
"#,
    )
    .unwrap();

    let missing = verify_behavior_coverage(
        &suite,
        &map,
        CoverageInputs {
            scenario_ndjson: vec![Cursor::new(passed_scenario_with_behaviors(
                "cli.real",
                &["BHV-CLI", "BHV-TERM-JOURNEY"],
            ))],
            libtest_listings: vec![Cursor::new(b"other::test: test\n".as_slice())],
            playwright_reports: Vec::<Cursor<&[u8]>>::new(),
        },
    )
    .unwrap_err();
    assert!(missing.iter().any(|error| error.contains("BHV-WEB")));
    assert!(
        missing
            .iter()
            .any(|error| error.contains("parser::split_utf8") && error.contains("not executed"))
    );
}

#[test]
fn scenario_libtest_and_playwright_evidence_close_the_matrix_without_orphans() {
    let root = suite();
    let suite = FunctionalSuite::load(root.path()).unwrap();
    let map = BehaviorEvidenceMapV1::from_toml(
        r#"schema = 1
[[evidence]]
behavior_id = "BHV-PARSER"
source = "libtest"
identity = "parser::split_utf8"
[[evidence]]
behavior_id = "BHV-WEB"
source = "playwright"
identity = "terminal.spec.ts › redeems ticket"
"#,
    )
    .unwrap();
    let playwright = br#"{
      "suites":[{"title":"terminal.spec.ts","specs":[{"title":"redeems ticket","tests":[{"results":[{"status":"passed"}]}]}]}]
    }"#;

    let report = verify_behavior_coverage(
        &suite,
        &map,
        CoverageInputs {
            scenario_ndjson: vec![Cursor::new(passed_scenario_with_behaviors(
                "cli.real",
                &["BHV-CLI", "BHV-TERM-JOURNEY"],
            ))],
            libtest_listings: vec![Cursor::new(b"test parser::split_utf8 ... ok\n".as_slice())],
            playwright_reports: vec![Cursor::new(playwright.as_slice())],
        },
    )
    .unwrap();
    assert_eq!(report.behaviors_total, 4);
    assert_eq!(report.behaviors_covered, 4);
    assert_eq!(report.subsystems_with_e2e, vec!["cli", "terminal", "web"]);
}

#[test]
fn passed_scenario_without_runtime_behavior_observations_does_not_cover_declarations() {
    let root = suite();
    let suite = FunctionalSuite::load(root.path()).unwrap();
    let errors = verify_behavior_coverage(
        &suite,
        &BehaviorEvidenceMapV1::from_toml("schema = 1\nevidence = []\n").unwrap(),
        CoverageInputs {
            scenario_ndjson: vec![Cursor::new(passed_scenario("cli.real"))],
            libtest_listings: Vec::<Cursor<&[u8]>>::new(),
            playwright_reports: Vec::<Cursor<&[u8]>>::new(),
        },
    )
    .unwrap_err();
    assert!(errors.iter().any(|error| error.contains("BHV-CLI")));
    assert!(
        errors
            .iter()
            .any(|error| error.contains("runtime observation"))
    );
}

#[test]
fn failed_scenario_and_failed_browser_result_are_never_retried_or_counted() {
    let root = suite();
    let suite = FunctionalSuite::load(root.path()).unwrap();
    let map = BehaviorEvidenceMapV1::from_toml(
        r#"schema = 1
[[evidence]]
behavior_id = "BHV-WEB"
source = "playwright"
identity = "terminal.spec.ts › redeems ticket"
[[evidence]]
behavior_id = "BHV-PARSER"
source = "libtest"
identity = "parser::split_utf8"
"#,
    )
    .unwrap();
    let playwright = br#"{
      "suites":[{"title":"terminal.spec.ts","specs":[{"title":"redeems ticket","tests":[{"results":[{"status":"failed"},{"status":"passed"}]}]}]}]
    }"#;
    let errors = verify_behavior_coverage(
        &suite,
        &map,
        CoverageInputs {
            scenario_ndjson: vec![Cursor::new(passed_scenario("cli.real"))],
            libtest_listings: vec![Cursor::new(b"test parser::split_utf8 ... ok\n".as_slice())],
            playwright_reports: vec![Cursor::new(playwright.as_slice())],
        },
    )
    .unwrap_err();
    assert!(errors.iter().any(|error| error.contains("retry")));
}
