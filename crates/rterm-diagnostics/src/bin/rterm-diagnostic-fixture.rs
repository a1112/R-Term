use std::{
    collections::HashMap,
    io::{self, BufRead, Write},
    thread,
    time::{Duration, Instant},
};

use rterm_diagnostics::{
    ConnectionState, MARKER_PREFIX, MarkerKind, MarkerRecord, RendererKind, Scenario, SchemaVersion,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (run_id, scenario) = parse_identity()?;
    let started = Instant::now();
    emit(&run_id, scenario, MarkerKind::ProcessStarted, started, None)?;
    match std::env::var("RTERM_DIAGNOSTIC_FIXTURE_MODE").as_deref() {
        Ok("early-exit") => {
            thread::sleep(Duration::from_millis(25));
            std::process::exit(7);
        }
        Ok("ignore-shutdown") => {
            emit_ready(&run_id, scenario, started)?;
            thread::sleep(Duration::from_secs(30));
        }
        _ => {
            emit_ready(&run_id, scenario, started)?;
            let mut line = String::new();
            io::stdin().lock().read_line(&mut line)?;
        }
    }
    emit(&run_id, scenario, MarkerKind::ProcessExited, started, None)?;
    Ok(())
}

fn parse_identity() -> Result<(String, Scenario), Box<dyn std::error::Error>> {
    let arguments = std::env::args().collect::<Vec<_>>();
    let value_after = |name: &str| {
        arguments
            .iter()
            .position(|argument| argument == name)
            .and_then(|index| arguments.get(index + 1))
            .cloned()
    };
    let run_id = value_after("--run-id").ok_or("fixture requires --run-id")?;
    let scenario = match value_after("--scenario").as_deref() {
        Some("empty-window") => Scenario::EmptyWindow,
        Some("ssh1") => Scenario::Ssh1,
        _ => return Err("fixture requires a valid --scenario".into()),
    };
    Ok((run_id, scenario))
}

fn emit_ready(
    run_id: &str,
    scenario: Scenario,
    started: Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    for kind in [
        MarkerKind::WindowCreated,
        MarkerKind::FirstPresent,
        MarkerKind::ConfigReady,
        MarkerKind::ScenarioReady,
    ] {
        emit(run_id, scenario, kind, started, Some(RendererKind::Cpu))?;
    }
    Ok(())
}

fn emit(
    run_id: &str,
    scenario: Scenario,
    kind: MarkerKind,
    started: Instant,
    renderer: Option<RendererKind>,
) -> Result<(), Box<dyn std::error::Error>> {
    let connection_state = Some(match scenario {
        Scenario::EmptyWindow => ConnectionState::NotStarted,
        Scenario::Ssh1 => ConnectionState::Connected,
    });
    let record = MarkerRecord {
        schema: SchemaVersion::V2,
        run_id: run_id.to_owned(),
        pid: std::process::id(),
        scenario,
        kind,
        elapsed_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        renderer,
        connection_state,
        extra: HashMap::new(),
    };
    let mut stdout = io::stdout().lock();
    write!(stdout, "{MARKER_PREFIX}")?;
    serde_json::to_writer(&mut stdout, &record)?;
    writeln!(stdout)?;
    stdout.flush()?;
    Ok(())
}
