use std::{
    collections::BTreeMap,
    env, io,
    path::Path,
    sync::{Mutex, OnceLock},
    thread,
};

use rterm_functional_tests::{
    HostEffectObservationV1, ObserverServer, ObserverSnapshotV1, ObserverState, ObserverToken,
    RuntimeObservationV1, TerminalObservationV1, WindowObservationV1,
};

pub(crate) const ENDPOINT_ENV: &str = "RTERM_FUNCTIONAL_OBSERVER_ENDPOINT";
pub(crate) const TOKEN_ENV: &str = "RTERM_FUNCTIONAL_OBSERVER_TOKEN";

struct ObserverRuntime {
    state: ObserverState,
    effects: Vec<HostEffectObservationV1>,
    next_effect_sequence: u64,
    config_generation: u64,
    config_diagnostic_present: bool,
}

static OBSERVER: OnceLock<Mutex<ObserverRuntime>> = OnceLock::new();

pub(crate) fn initialize_from_environment() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = env::var_os(ENDPOINT_ENV);
    let token = env::var(TOKEN_ENV).ok();
    match (endpoint, token) {
        (None, None) => return Ok(()),
        (Some(_), None) | (None, Some(_)) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "functional observer endpoint and token must be configured together",
            )
            .into());
        }
        (Some(endpoint), Some(token)) => {
            let token = ObserverToken::from_child_process(&token)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
            let state = ObserverState::new(empty_snapshot())?;
            let mut server = ObserverServer::bind(Path::new(&endpoint), token, state.clone())?;
            thread::Builder::new()
                .name("rterm-functional-observer".to_owned())
                .spawn(move || {
                    if let Err(error) = server.serve_one() {
                        eprintln!("functional observer error: {error}");
                    }
                })?;
            OBSERVER
                .set(Mutex::new(ObserverRuntime {
                    state,
                    effects: Vec::new(),
                    next_effect_sequence: 1,
                    config_generation: 0,
                    config_diagnostic_present: false,
                }))
                .map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        "functional observer was initialized twice",
                    )
                })?;
        }
    }
    Ok(())
}

pub(crate) fn publish(mut candidate: ObserverSnapshotV1) {
    let Some(observer) = OBSERVER.get() else {
        return;
    };
    let observer = observer
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let current = observer.state.snapshot();
    candidate.runtime.effects.clone_from(&observer.effects);
    candidate.config_generation = observer.config_generation;
    candidate.config_diagnostic_present = observer.config_diagnostic_present;
    candidate.revision = current.revision;
    if candidate == current {
        return;
    }
    candidate.revision = current.revision.saturating_add(1);
    let _ = observer.state.publish(candidate);
}

pub(crate) fn record_config_lifecycle(generation: u64, diagnostic_present: bool) {
    let Some(observer) = OBSERVER.get() else {
        return;
    };
    let mut observer = observer
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    observer.config_generation = generation;
    observer.config_diagnostic_present = diagnostic_present;
}

pub(crate) fn wait_until_current_revision_delivered(timeout: std::time::Duration) -> bool {
    let Some(observer) = OBSERVER.get() else {
        return true;
    };
    let (state, revision) = {
        let observer = observer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let state = observer.state.clone();
        let revision = state.snapshot().revision;
        (state, revision)
    };
    state.wait_until_delivered(revision, timeout)
}

pub(crate) fn record_effect(kind: &'static str) {
    let Some(observer) = OBSERVER.get() else {
        return;
    };
    let mut observer = observer
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let sequence = observer.next_effect_sequence;
    observer.next_effect_sequence = observer.next_effect_sequence.saturating_add(1);
    observer.effects.push(HostEffectObservationV1 {
        sequence,
        kind: kind.to_owned(),
    });
}

fn empty_snapshot() -> ObserverSnapshotV1 {
    ObserverSnapshotV1 {
        schema: 1,
        revision: 0,
        config_generation: 0,
        config_diagnostic_present: false,
        terminal: TerminalObservationV1 {
            text: String::new(),
            cursor_row: 0,
            cursor_column: 0,
            modes: BTreeMap::new(),
        },
        window: WindowObservationV1 {
            width: 0,
            height: 0,
            active_tab_id: None,
            active_pane_id: None,
            overlay: None,
            panes: Vec::new(),
        },
        runtime: RuntimeObservationV1 {
            transport_state: "starting".to_owned(),
            effects: Vec::new(),
            render_digest: None,
            worker_count: 0,
            listener_count: 0,
            child_process_count: 0,
        },
    }
}
