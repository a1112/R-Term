use std::{collections::BTreeMap, fs};

use rterm_functional_tests::{
    CheckpointContext, CheckpointV1, HostEffectObservationV1, ObserverSnapshotV1,
    PaneObservationV1, RuntimeObservationV1, TerminalObservationV1, WindowObservationV1,
    evaluate_checkpoint,
};

fn snapshot() -> ObserverSnapshotV1 {
    ObserverSnapshotV1 {
        schema: 1,
        revision: 4,
        config_generation: 0,
        config_diagnostic_present: false,
        terminal: TerminalObservationV1 {
            text: "hello functional world".to_owned(),
            cursor_row: 2,
            cursor_column: 7,
            modes: BTreeMap::from([("bracketed_paste".to_owned(), true)]),
        },
        window: WindowObservationV1 {
            width: 1024,
            height: 640,
            active_tab_id: Some(1),
            active_pane_id: Some(2),
            overlay: Some("command_palette".to_owned()),
            panes: vec![PaneObservationV1 {
                tab_id: 1,
                pane_id: 2,
                active: true,
                row: 0,
                column: 0,
                rows: 24,
                columns: 80,
            }],
        },
        runtime: RuntimeObservationV1 {
            transport_state: "connected".to_owned(),
            effects: vec![HostEffectObservationV1 {
                sequence: 3,
                kind: "clipboard_write".to_owned(),
            }],
            render_digest: Some("sha256:abcd".to_owned()),
            worker_count: 0,
            listener_count: 0,
            child_process_count: 0,
        },
    }
}

#[test]
fn typed_checkpoint_evaluator_covers_semantic_window_runtime_and_resources() {
    let observed = snapshot();
    let context = CheckpointContext {
        snapshot: Some(&observed),
        stdout: b"hello from stdout",
        stderr: b"",
        exit_code: Some(7),
        resources_zero: true,
        artifact_root: None,
        network_bytes: BTreeMap::from([("echo", b"ping".as_slice())]),
    };
    for checkpoint in [
        CheckpointV1::TerminalContains {
            text: "functional".to_owned(),
        },
        CheckpointV1::Cursor { row: 2, column: 7 },
        CheckpointV1::TerminalMode {
            name: "bracketed_paste".to_owned(),
            enabled: true,
        },
        CheckpointV1::Pane {
            tab_id: 1,
            pane_id: 2,
            active: true,
        },
        CheckpointV1::Overlay {
            name: "command_palette".to_owned(),
            visible: true,
        },
        CheckpointV1::Transport {
            state: "connected".to_owned(),
        },
        CheckpointV1::HostEffect {
            kind: "clipboard_write".to_owned(),
            sequence: 3,
        },
        CheckpointV1::WindowGeometry {
            width: 1024,
            height: 640,
        },
        CheckpointV1::NetworkBytes {
            fixture: "echo".to_owned(),
            bytes_hex: "70696e67".to_owned(),
        },
        CheckpointV1::ExitStatus { code: 7 },
        CheckpointV1::ResourcesZero,
        CheckpointV1::RenderProbe {
            region: "terminal_content".to_owned(),
            digest: "sha256:abcd".to_owned(),
        },
    ] {
        evaluate_checkpoint(&checkpoint, &context).unwrap();
    }
}

#[test]
fn file_hash_is_rooted_and_traversal_fails_closed() {
    let directory = tempfile::TempDir::new().unwrap();
    fs::write(directory.path().join("payload.bin"), b"payload").unwrap();
    let context = CheckpointContext {
        snapshot: None,
        stdout: b"",
        stderr: b"",
        exit_code: None,
        resources_zero: false,
        artifact_root: Some(directory.path()),
        network_bytes: BTreeMap::new(),
    };
    evaluate_checkpoint(
        &CheckpointV1::FileSha256 {
            path: "payload.bin".to_owned(),
            sha256: "239f59ed55e737c77147cf55ad0c1b030b6d7ee748a7426952f9b852d5a935e5".to_owned(),
        },
        &context,
    )
    .unwrap();
    let error = evaluate_checkpoint(
        &CheckpointV1::FileSha256 {
            path: "../secret".to_owned(),
            sha256: "239f59ed55e737c77147cf55ad0c1b030b6d7ee748a7426952f9b852d5a935e5".to_owned(),
        },
        &context,
    )
    .unwrap_err();
    assert!(error.to_string().contains("relative"));
}

#[test]
fn mismatches_return_actionable_typed_diagnostics() {
    let observed = snapshot();
    let context = CheckpointContext {
        snapshot: Some(&observed),
        stdout: b"",
        stderr: b"",
        exit_code: Some(0),
        resources_zero: true,
        artifact_root: None,
        network_bytes: BTreeMap::new(),
    };
    let error = evaluate_checkpoint(
        &CheckpointV1::WindowGeometry {
            width: 1,
            height: 2,
        },
        &context,
    )
    .unwrap_err();
    assert!(error.to_string().contains("1024x640"));
}
