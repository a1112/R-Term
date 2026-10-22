use rterm_functional_tests::{
    ActionV1, BehaviorCatalogV1, Capability, CheckpointV1, EvidenceKind, ScenarioV1,
    ScenarioValidationError, Surface, WindowControl, assign_lpt_shards, validate_catalog,
};

const COMPLETE_SCENARIO: &str = r#"
schema = 1
id = "window.local.interaction"
behavior_ids = ["BHV-WINDOW-INPUT", "BHV-LIFECYCLE-CLEANUP"]
surface = "native_window"
capabilities = ["real_os_keyboard", "real_os_pointer", "system_clipboard"]
fixture = "terminal_probe"
estimated_cost_ms = 4200
required_evidence = ["event_log", "final_snapshot", "process_tree", "screenshot_on_failure"]

[deadlines]
action_ms = 12000
startup_ms = 25000
cleanup_ms = 8000
scenario_ms = 90000

[[actions]]
type = "type_text"
text = "R-Term 终端"

[[actions]]
type = "key"
key = "Enter"
modifiers = ["ctrl"]

[[actions]]
type = "mouse_click"
x = 12
y = 8
button = "left"

[[actions]]
type = "mouse_drag"
from_x = 2
from_y = 3
to_x = 40
to_y = 9
button = "left"

[[actions]]
type = "mouse_wheel"
delta_x = 0
delta_y = -3

[[actions]]
type = "clipboard_paste"
text = "paste-value"

[[actions]]
type = "resize_window"
width = 1024
height = 640

[[actions]]
type = "focus_window"

[[actions]]
type = "pty_input"
bytes_hex = "1b5b41"

[[actions]]
type = "fixture_disconnect"
fixture = "ssh"

[[actions]]
type = "fixture_reconnect"
fixture = "ssh"

[[actions]]
type = "finish"

[[checkpoints]]
type = "terminal_contains"
text = "R-Term"

[[checkpoints]]
type = "cursor"
row = 3
column = 7

[[checkpoints]]
type = "terminal_mode"
name = "bracketed_paste"
enabled = true

[[checkpoints]]
type = "pane"
tab_id = 1
pane_id = 2
active = true

[[checkpoints]]
type = "overlay"
name = "command_palette"
visible = false

[[checkpoints]]
type = "transport"
state = "closed"

[[checkpoints]]
type = "host_effect"
kind = "clipboard_write"
sequence = 3

[[checkpoints]]
type = "window_geometry"
width = 1024
height = 640

[[checkpoints]]
type = "file_sha256"
path = "session.log"
sha256 = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"

[[checkpoints]]
type = "network_bytes"
fixture = "echo"
bytes_hex = "70696e67"

[[checkpoints]]
type = "exit_status"
code = 0

[[checkpoints]]
type = "resources_zero"

[[checkpoints]]
type = "render_probe"
region = "terminal_content"
digest = "sha256:abcdef"

"#;

#[test]
fn scenario_v1_parses_the_closed_action_and_checkpoint_surface() {
    let scenario = ScenarioV1::from_toml(COMPLETE_SCENARIO).expect("complete scenario");

    assert_eq!(scenario.schema, 1);
    assert_eq!(scenario.surface, Surface::NativeWindow);
    assert!(scenario.capabilities.contains(&Capability::RealOsKeyboard));
    assert_eq!(scenario.actions.len(), 12);
    assert!(matches!(scenario.actions[0], ActionV1::TypeText { .. }));
    assert!(matches!(scenario.actions[11], ActionV1::Finish));
    assert_eq!(scenario.checkpoints.len(), 13);
    assert!(matches!(
        scenario.checkpoints[12],
        CheckpointV1::RenderProbe { .. }
    ));
    assert!(scenario.required_evidence.contains(&EvidenceKind::EventLog));
}

#[test]
fn scenario_defaults_match_the_functional_test_budget() {
    let scenario = ScenarioV1::from_toml(
        r#"
schema = 1
id = "cli.version"
behavior_ids = ["BHV-CLI-VERSION"]
surface = "console"
fixture = "none"
estimated_cost_ms = 50
actions = [{ type = "finish" }]
checkpoints = [{ type = "exit_status", code = 0 }]
required_evidence = ["event_log"]
"#,
    )
    .unwrap();

    assert_eq!(scenario.deadlines.action_ms, 15_000);
    assert_eq!(scenario.deadlines.startup_ms, 30_000);
    assert_eq!(scenario.deadlines.cleanup_ms, 10_000);
    assert_eq!(scenario.deadlines.scenario_ms, 120_000);
}

#[test]
fn scenario_deadlines_cannot_exceed_the_functional_test_budget() {
    for (field, value) in [
        ("action_ms", 15_001),
        ("startup_ms", 30_001),
        ("cleanup_ms", 10_001),
        ("scenario_ms", 120_001),
    ] {
        let document = format!(
            r#"
schema = 1
id = "cli.deadline"
behavior_ids = ["BHV-CLI-VERSION"]
surface = "console"
fixture = "none"
estimated_cost_ms = 50
deadlines = {{ {field} = {value} }}
actions = [{{ type = "finish" }}]
checkpoints = [{{ type = "exit_status", code = 0 }}]
required_evidence = ["event_log"]
"#
        );
        let error = ScenarioV1::from_toml(&document).unwrap_err();
        assert!(error.to_string().contains(field), "{error}");
    }
}

#[test]
fn arbitrary_scripts_and_unknown_fields_are_not_part_of_the_schema() {
    let error = ScenarioV1::from_toml(
        r#"
schema = 1
id = "cli.forbidden"
behavior_ids = ["BHV-CLI-VERSION"]
surface = "console"
fixture = "none"
estimated_cost_ms = 1
actions = [{ type = "script", command = "sleep 30" }]
checkpoints = []
required_evidence = []
"#,
    )
    .unwrap_err();

    assert!(error.to_string().contains("unknown variant `script`"));

    let error = ScenarioV1::from_toml(&COMPLETE_SCENARIO.replace(
        "estimated_cost_ms = 4200",
        "estimated_cost_ms = 4200\nretry_count = 1",
    ))
    .unwrap_err();
    assert!(error.to_string().contains("unknown field `retry_count`"));
}

#[test]
fn every_closed_action_has_an_approved_executable_driver_path() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scenarios = std::fs::read_dir(root.join("functional-tests/scenarios"))
        .unwrap()
        .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
        .collect::<String>();
    for action in [
        "type_text",
        "key",
        "mouse_click",
        "mouse_drag",
        "mouse_wheel",
        "clipboard_paste",
        "resize_window",
        "window_control",
        "focus_window",
        "pty_input",
        "fixture_disconnect",
        "fixture_reconnect",
        "finish",
    ] {
        assert!(
            scenarios.contains(&format!("type = \"{action}\"")),
            "closed action {action} has no approved executable scenario"
        );
    }
    let runner =
        std::fs::read_to_string(root.join("crates/rterm-functional-tests/src/runner.rs")).unwrap();
    assert!(runner.contains("execute_pty_disconnect_reconnect_scenario"));
    assert!(runner.contains("ActionV1::FixtureDisconnect"));
    assert!(runner.contains("ActionV1::FixtureReconnect"));
}

#[test]
fn tauri_window_scenario_closes_after_window_state_transitions() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source =
        std::fs::read_to_string(root.join("functional-tests/scenarios/tauri.local-pty.toml"))
            .unwrap();
    let scenario = ScenarioV1::from_toml(&source).unwrap();
    let last_restore = scenario
        .actions
        .iter()
        .rposition(|action| {
            matches!(
                action,
                ActionV1::WindowControl { operation }
                    if *operation == WindowControl::Restore
            )
        })
        .expect("Tauri scenario exercises window restore");
    assert!(
        matches!(
            scenario.actions.get(last_restore + 1),
            Some(ActionV1::WindowControl {
                operation: WindowControl::Close
            })
        ),
        "the Tauri journey must close the restored window without relying on PTY input focus"
    );
}

#[test]
fn scenario_validation_rejects_unstable_ids_and_invalid_hex() {
    let mut scenario = ScenarioV1::from_toml(COMPLETE_SCENARIO).unwrap();
    scenario.id = "Not Stable".to_owned();
    assert_eq!(
        scenario.validate().unwrap_err(),
        ScenarioValidationError::InvalidScenarioId("Not Stable".to_owned())
    );

    scenario.id = "window.local.interaction".to_owned();
    scenario.actions.push(ActionV1::PtyInput {
        bytes_hex: "xyz".to_owned(),
    });
    assert!(matches!(
        scenario.validate(),
        Err(ScenarioValidationError::InvalidHex { .. })
    ));
}

#[test]
fn lpt_shards_are_deterministic_balanced_and_complete() {
    let costs = [
        ("scenario-a", 10_000),
        ("scenario-b", 9_000),
        ("scenario-c", 8_000),
        ("scenario-d", 7_000),
        ("scenario-e", 6_000),
    ];

    let first = assign_lpt_shards(costs, 2).unwrap();
    let second = assign_lpt_shards(costs.into_iter().rev(), 2).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        first[0].scenario_ids,
        ["scenario-a", "scenario-d", "scenario-e"]
    );
    assert_eq!(first[1].scenario_ids, ["scenario-b", "scenario-c"]);
    assert_eq!(first[0].estimated_cost_ms, 23_000);
    assert_eq!(first[1].estimated_cost_ms, 17_000);
}

#[test]
fn catalog_validation_rejects_unknown_references_and_defers_orphans_to_executable_coverage() {
    let catalog = BehaviorCatalogV1::from_toml(
        r#"
schema = 1

[[behaviors]]
id = "BHV-CLI-VERSION"
subsystem = "startup"
summary = "version is machine readable"
surfaces = ["console"]

[[behaviors]]
id = "BHV-ORPHAN"
subsystem = "startup"
summary = "must have evidence"
surfaces = ["console"]
"#,
    )
    .unwrap();
    let scenario = ScenarioV1::from_toml(
        r#"
schema = 1
id = "cli.version"
behavior_ids = ["BHV-CLI-VERSION", "BHV-UNKNOWN"]
surface = "console"
fixture = "none"
estimated_cost_ms = 1
actions = [{ type = "finish" }]
checkpoints = [{ type = "exit_status", code = 0 }]
required_evidence = ["event_log"]
"#,
    )
    .unwrap();

    let errors = validate_catalog(&catalog, &[scenario]).unwrap_err();
    assert!(errors.iter().any(|error| error.contains("BHV-UNKNOWN")));
    assert!(!errors.iter().any(|error| error.contains("BHV-ORPHAN")));
}

#[test]
fn catalog_validation_rejects_behaviors_outside_the_scenario_surface() {
    let catalog = BehaviorCatalogV1::from_toml(
        r#"
schema = 1

[[behaviors]]
id = "BHV-CONSOLE-ONLY"
subsystem = "startup"
summary = "console-only behavior"
surfaces = ["console"]
"#,
    )
    .unwrap();
    let scenario = ScenarioV1::from_toml(
        r#"
schema = 1
id = "host.surface.mismatch"
behavior_ids = ["BHV-CONSOLE-ONLY"]
surface = "host_terminal"
fixture = "none"
estimated_cost_ms = 1
actions = [{ type = "finish" }]
checkpoints = [{ type = "exit_status", code = 0 }]
required_evidence = ["event_log"]
"#,
    )
    .unwrap();

    let errors = validate_catalog(&catalog, &[scenario]).unwrap_err();
    assert!(errors.iter().any(|error| {
        error.contains("host.surface.mismatch")
            && error.contains("BHV-CONSOLE-ONLY")
            && error.contains("host_terminal")
    }));
}

#[test]
fn scenario_rejects_duplicate_behavior_and_evidence_declarations() {
    let duplicate_behavior = ScenarioV1::from_toml(
        r#"
schema = 1
id = "duplicate.behavior"
behavior_ids = ["BHV-ONE", "BHV-ONE"]
surface = "console"
fixture = "none"
estimated_cost_ms = 1
actions = [{ type = "finish" }]
checkpoints = []
required_evidence = ["event_log"]
"#,
    )
    .unwrap_err();
    assert!(
        duplicate_behavior
            .to_string()
            .contains("behavior IDs must be unique")
    );

    let duplicate_evidence = ScenarioV1::from_toml(
        r#"
schema = 1
id = "duplicate.evidence"
behavior_ids = ["BHV-ONE"]
surface = "console"
fixture = "none"
estimated_cost_ms = 1
actions = [{ type = "finish" }]
checkpoints = []
required_evidence = ["event_log", "event_log"]
"#,
    )
    .unwrap_err();
    assert!(
        duplicate_evidence
            .to_string()
            .contains("evidence kinds must be unique")
    );
}
