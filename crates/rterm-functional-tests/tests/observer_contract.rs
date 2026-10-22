use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Write},
    thread,
    time::Duration,
};

#[cfg(unix)]
use std::path::PathBuf;

use rterm_functional_tests::{
    HostEffectObservationV1, ObserverClient, ObserverEndpoint, ObserverRequestV1,
    ObserverResponseV1, ObserverServer, ObserverSnapshotV1, ObserverState, ObserverToken,
    PaneObservationV1, RuntimeObservationV1, TerminalObservationV1, WindowObservationV1,
};

fn snapshot(revision: u64, text: &str) -> ObserverSnapshotV1 {
    ObserverSnapshotV1 {
        schema: 1,
        revision,
        config_generation: 0,
        config_diagnostic_present: false,
        terminal: TerminalObservationV1 {
            text: text.to_owned(),
            cursor_row: 2,
            cursor_column: 4,
            modes: BTreeMap::from([("bracketed_paste".to_owned(), true)]),
        },
        window: WindowObservationV1 {
            width: 1024,
            height: 640,
            active_tab_id: Some(1),
            active_pane_id: Some(2),
            overlay: None,
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
                sequence: 1,
                kind: "bell".to_owned(),
            }],
            render_digest: Some("sha256:abcd".to_owned()),
            worker_count: 1,
            listener_count: 1,
            child_process_count: 1,
        },
    }
}

#[test]
fn protocol_has_only_hello_snapshot_and_subscribe_requests() {
    for request in [
        r#"{"schema":1,"request":"hello","token":"0000000000000000000000000000000000000000000000000000000000000000"}"#,
        r#"{"schema":1,"request":"snapshot"}"#,
        r#"{"schema":1,"request":"subscribe","after_revision":7}"#,
    ] {
        serde_json::from_str::<ObserverRequestV1>(request).unwrap();
    }

    for forbidden in [
        r#"{"schema":1,"request":"input","text":"secret"}"#,
        r#"{"schema":1,"request":"execute","command":"whoami"}"#,
        r#"{"schema":1,"request":"disconnect"}"#,
    ] {
        let error = serde_json::from_str::<ObserverRequestV1>(forbidden).unwrap_err();
        assert!(error.to_string().contains("unknown variant"));
    }
}

#[test]
fn observer_requires_a_256_bit_one_time_token_before_any_read() {
    let token = ObserverToken::generate();
    let encoded = token.expose_for_child_process();
    assert_eq!(encoded.len(), 64);
    assert_eq!(ObserverToken::from_child_process(&encoded).unwrap(), token);
    assert!(ObserverToken::from_child_process("short").is_err());
    let state = ObserverState::new(snapshot(1, "ready")).unwrap();
    let mut session = state.session(token.clone());

    assert!(matches!(
        session.handle(ObserverRequestV1::Snapshot),
        ObserverResponseV1::Unauthorized { .. }
    ));
    assert!(matches!(
        session.handle(ObserverRequestV1::hello(ObserverToken::generate())),
        ObserverResponseV1::Unauthorized { .. }
    ));
    assert!(matches!(
        session.handle(ObserverRequestV1::hello(token.clone())),
        ObserverResponseV1::Hello { schema: 1, .. }
    ));
    assert!(matches!(
        session.handle(ObserverRequestV1::hello(token)),
        ObserverResponseV1::ProtocolError { .. }
    ));
}

#[test]
fn snapshot_publication_is_revision_monotonic_and_subscribe_waits_for_change() {
    let state = ObserverState::new(snapshot(5, "first")).unwrap();
    assert!(state.publish(snapshot(5, "duplicate")).is_err());
    assert!(state.publish(snapshot(4, "backwards")).is_err());

    let waiting = state.clone();
    let handle = thread::spawn(move || {
        waiting
            .wait_after(5, Duration::from_secs(1))
            .expect("updated snapshot")
    });
    state.publish(snapshot(6, "second")).unwrap();
    let updated = handle.join().unwrap();
    assert_eq!(updated.revision, 6);
    assert_eq!(updated.terminal.text, "second");
}

#[test]
fn real_local_transport_round_trips_and_removes_its_endpoint() {
    let directory = tempfile::TempDir::new().unwrap();
    let endpoint = directory.path().join("observer.sock");
    let token = ObserverToken::generate();
    let state = ObserverState::new(snapshot(1, "transport-ready")).unwrap();
    let mut server = ObserverServer::bind(&endpoint, token.clone(), state).unwrap();
    let endpoint_name = server.endpoint().to_owned();
    let handle = thread::spawn(move || server.serve_one().unwrap());

    assert_eq!(
        ObserverEndpoint::from_requested_path(&endpoint).unwrap(),
        endpoint_name
    );
    let mut client = ObserverClient::connect_path(&endpoint).unwrap();
    client.hello(token).unwrap();
    let observed = client.snapshot().unwrap();
    assert_eq!(observed.terminal.text, "transport-ready");
    drop(client);
    handle.join().unwrap();

    #[cfg(unix)]
    assert!(!endpoint.exists(), "UDS path survived server drop");
}

#[test]
fn unix_observer_transport_uses_a_bounded_hashed_socket_path() {
    let source = include_str!("../src/observer.rs");
    assert!(source.contains("socket_path: PathBuf"));
    assert!(source.contains("fn unix_socket_path("));
    assert!(source.contains("endpoint_path_hash(requested_path)"));
    assert!(!source.contains(
        "endpoint\n            .requested_path\n            .as_path()\n            .to_fs_name::<GenericFilePath>()"
    ));
}

#[cfg(unix)]
#[test]
fn long_requested_path_round_trips_through_a_short_private_unix_socket() {
    use std::os::unix::fs::PermissionsExt;

    let requested = PathBuf::from("/an/intentionally/long/runner/checkout/root")
        .join("evidence/tauri-macos-accessibility")
        .join("tauri.local-pty.macos-accessibility.observer");
    let token = ObserverToken::generate();
    let state = ObserverState::new(snapshot(1, "long-path-ready")).unwrap();
    let mut server = ObserverServer::bind(&requested, token.clone(), state).unwrap();
    let transport_path = server.endpoint().transport_path().to_owned();
    assert!(transport_path.as_os_str().len() < 90);
    assert_ne!(transport_path, requested);
    assert_eq!(
        transport_path.metadata().unwrap().permissions().mode() & 0o777,
        0o600,
        "observer socket must remain owner-only"
    );
    assert_eq!(
        transport_path
            .parent()
            .unwrap()
            .metadata()
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    let handle = thread::spawn(move || server.serve_one().unwrap());
    let mut client = ObserverClient::connect_path(&requested).unwrap();
    client.hello(token).unwrap();
    assert_eq!(client.snapshot().unwrap().terminal.text, "long-path-ready");
    drop(client);
    handle.join().unwrap();
    assert!(!transport_path.exists());
}

#[test]
fn server_acknowledges_a_revision_only_after_writing_it_to_the_observer() {
    let directory = tempfile::TempDir::new().unwrap();
    let endpoint = directory.path().join("delivery.sock");
    let token = ObserverToken::generate();
    let state = ObserverState::new(snapshot(1, "delivery-ready")).unwrap();
    let delivered = state.clone();
    let mut server = ObserverServer::bind(&endpoint, token.clone(), state).unwrap();
    let handle = thread::spawn(move || server.serve_one().unwrap());

    assert!(!delivered.wait_until_delivered(1, Duration::from_millis(1)));
    let mut client = ObserverClient::connect_path(&endpoint).unwrap();
    client.hello(token).unwrap();
    assert!(delivered.wait_until_delivered(1, Duration::from_secs(1)));
    drop(client);
    handle.join().unwrap();
}

#[test]
fn wire_responses_do_not_accept_authentication_or_environment_secrets() {
    let mut encoded = Vec::new();
    serde_json::to_writer(
        &mut encoded,
        &ObserverResponseV1::Snapshot {
            schema: 1,
            snapshot: snapshot(1, "public fixture output"),
        },
    )
    .unwrap();
    encoded.write_all(b"\n").unwrap();
    let line = BufReader::new(encoded.as_slice())
        .lines()
        .next()
        .unwrap()
        .unwrap();
    assert!(!line.contains("password"));
    assert!(!line.contains("private_key"));
    assert!(!line.contains("environment"));
    assert!(!line.contains("token"));
}

#[test]
fn snapshot_exposes_only_non_sensitive_config_lifecycle_state() {
    let source = include_str!("../src/observer.rs");
    assert!(source.contains("pub config_generation: u64"));
    assert!(source.contains("pub config_diagnostic_present: bool"));
    assert!(!source.contains("pub config_path:"));
    assert!(!source.contains("pub config_contents:"));
}
