use std::{
    collections::{HashMap, HashSet},
    io::{self, Write},
    sync::{Arc, Mutex},
    time::Instant,
};

use rterm_diagnostics::{
    ConnectionState, MARKER_PREFIX, MarkerKind, MarkerRecord, RendererKind, Scenario, SchemaVersion,
};

#[derive(Clone)]
pub(crate) struct DiagnosticMarkerHandle {
    state: Arc<Mutex<DiagnosticMarkerState>>,
}

struct DiagnosticMarkerState {
    run_id: String,
    scenario: Scenario,
    pid: u32,
    process_started_at: Instant,
    emitted: HashSet<MarkerKind>,
    last_renderer: Option<RendererKind>,
    last_connection_state: Option<ConnectionState>,
}

impl DiagnosticMarkerHandle {
    pub(crate) fn new(run_id: String, scenario: Scenario, process_started_at: Instant) -> Self {
        Self {
            state: Arc::new(Mutex::new(DiagnosticMarkerState {
                run_id,
                scenario,
                pid: std::process::id(),
                process_started_at,
                emitted: HashSet::new(),
                last_renderer: None,
                last_connection_state: None,
            })),
        }
    }

    pub(crate) fn emit(
        &self,
        kind: MarkerKind,
        renderer: Option<RendererKind>,
        connection_state: Option<ConnectionState>,
    ) -> io::Result<bool> {
        self.emit_with_extra(kind, renderer, connection_state, HashMap::new())
    }

    pub(crate) fn emit_with_extra(
        &self,
        kind: MarkerKind,
        renderer: Option<RendererKind>,
        connection_state: Option<ConnectionState>,
        extra: HashMap<String, serde_json::Value>,
    ) -> io::Result<bool> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !state.emitted.insert(kind) {
            return Ok(false);
        }
        if renderer.is_some() {
            state.last_renderer = renderer;
        }
        if connection_state.is_some() {
            state.last_connection_state = connection_state;
        }
        let renderer = if kind == MarkerKind::ProcessExited {
            renderer.or(state.last_renderer)
        } else {
            renderer
        };
        let connection_state = if kind == MarkerKind::ProcessExited {
            connection_state.or(state.last_connection_state)
        } else {
            connection_state
        };
        let record = MarkerRecord {
            schema: SchemaVersion::V2,
            run_id: state.run_id.clone(),
            pid: state.pid,
            scenario: state.scenario,
            kind,
            elapsed_ms: u64::try_from(state.process_started_at.elapsed().as_millis())
                .unwrap_or(u64::MAX),
            renderer,
            connection_state,
            extra,
        };
        let mut stdout = io::stdout().lock();
        write!(stdout, "{MARKER_PREFIX}")?;
        serde_json::to_writer(&mut stdout, &record)?;
        writeln!(stdout)?;
        stdout.flush()?;
        Ok(true)
    }
}
