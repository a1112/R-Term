use std::{collections::BTreeMap, error::Error, fmt, fs, path::Path};

use sha2::{Digest, Sha256};

use crate::{CheckpointV1, ObserverSnapshotV1};

pub struct CheckpointContext<'a> {
    pub snapshot: Option<&'a ObserverSnapshotV1>,
    pub stdout: &'a [u8],
    pub stderr: &'a [u8],
    pub exit_code: Option<i32>,
    pub resources_zero: bool,
    pub artifact_root: Option<&'a Path>,
    pub network_bytes: BTreeMap<&'a str, &'a [u8]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointError(String);

impl fmt::Display for CheckpointError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for CheckpointError {}

/// Evaluates one typed checkpoint against captured semantic evidence.
///
/// # Errors
///
/// Returns a diagnostic when evidence is unavailable or the predicate fails.
pub fn evaluate_checkpoint(
    checkpoint: &CheckpointV1,
    context: &CheckpointContext<'_>,
) -> Result<String, CheckpointError> {
    match checkpoint {
        CheckpointV1::TerminalContains { .. }
        | CheckpointV1::Cursor { .. }
        | CheckpointV1::TerminalMode { .. }
        | CheckpointV1::Pane { .. }
        | CheckpointV1::Overlay { .. } => evaluate_terminal_checkpoint(checkpoint, context),
        CheckpointV1::Transport { .. }
        | CheckpointV1::ConfigGeneration { .. }
        | CheckpointV1::ConfigDiagnostic { .. }
        | CheckpointV1::HostEffect { .. }
        | CheckpointV1::WindowGeometry { .. }
        | CheckpointV1::RenderProbe { .. } => evaluate_observer_checkpoint(checkpoint, context),
        CheckpointV1::FileSha256 { .. }
        | CheckpointV1::NetworkBytes { .. }
        | CheckpointV1::ExitStatus { .. }
        | CheckpointV1::ResourcesZero => evaluate_external_checkpoint(checkpoint, context),
    }
}

fn evaluate_terminal_checkpoint(
    checkpoint: &CheckpointV1,
    context: &CheckpointContext<'_>,
) -> Result<String, CheckpointError> {
    match checkpoint {
        CheckpointV1::TerminalContains { text } => {
            let observed = context.snapshot.map_or_else(
                || String::from_utf8_lossy(context.stdout),
                |snapshot| std::borrow::Cow::Borrowed(snapshot.terminal.text.as_str()),
            );
            require(
                observed.contains(text),
                format!("terminal text did not contain {text:?}"),
                format!("terminal_contains={text:?}"),
            )
        }
        CheckpointV1::Cursor { row, column } => {
            let snapshot = require_snapshot(context)?;
            require(
                snapshot.terminal.cursor_row == *row && snapshot.terminal.cursor_column == *column,
                format!(
                    "expected cursor {row},{column}; observed {},{}",
                    snapshot.terminal.cursor_row, snapshot.terminal.cursor_column
                ),
                format!("cursor={row},{column}"),
            )
        }
        CheckpointV1::TerminalMode { name, enabled } => {
            let snapshot = require_snapshot(context)?;
            let observed = snapshot.terminal.modes.get(name).copied();
            require(
                observed == Some(*enabled),
                format!("expected terminal mode {name}={enabled}; observed {observed:?}"),
                format!("terminal_mode={name}:{enabled}"),
            )
        }
        CheckpointV1::Pane {
            tab_id,
            pane_id,
            active,
        } => {
            let snapshot = require_snapshot(context)?;
            let found = snapshot.window.panes.iter().any(|pane| {
                pane.tab_id == *tab_id && pane.pane_id == *pane_id && pane.active == *active
            });
            require(
                found,
                format!("pane tab={tab_id} pane={pane_id} active={active} was absent"),
                format!("pane={tab_id}:{pane_id}:{active}"),
            )
        }
        CheckpointV1::Overlay { name, visible } => {
            let snapshot = require_snapshot(context)?;
            let observed = snapshot.window.overlay.as_deref() == Some(name);
            require(
                observed == *visible,
                format!(
                    "expected overlay {name} visible={visible}; observed {:?}",
                    snapshot.window.overlay
                ),
                format!("overlay={name}:{visible}"),
            )
        }
        _ => unreachable!("terminal checkpoint dispatcher is exhaustive"),
    }
}

fn evaluate_observer_checkpoint(
    checkpoint: &CheckpointV1,
    context: &CheckpointContext<'_>,
) -> Result<String, CheckpointError> {
    match checkpoint {
        CheckpointV1::Transport { state } => {
            let snapshot = require_snapshot(context)?;
            require(
                snapshot.runtime.transport_state == *state,
                format!(
                    "expected transport {state}; observed {}",
                    snapshot.runtime.transport_state
                ),
                format!("transport={state}"),
            )
        }
        CheckpointV1::ConfigGeneration { generation } => {
            let snapshot = require_snapshot(context)?;
            require(
                snapshot.config_generation == *generation,
                format!(
                    "expected config generation {generation}; observed {}",
                    snapshot.config_generation
                ),
                format!("config_generation={generation}"),
            )
        }
        CheckpointV1::ConfigDiagnostic { present } => {
            let snapshot = require_snapshot(context)?;
            require(
                snapshot.config_diagnostic_present == *present,
                format!(
                    "expected config diagnostic present={present}; observed {}",
                    snapshot.config_diagnostic_present
                ),
                format!("config_diagnostic_present={present}"),
            )
        }
        CheckpointV1::HostEffect { kind, sequence } => {
            let snapshot = require_snapshot(context)?;
            let found = snapshot
                .runtime
                .effects
                .iter()
                .any(|effect| effect.kind == *kind && effect.sequence == *sequence);
            require(
                found,
                format!("host effect {sequence}:{kind} was absent"),
                format!("host_effect={sequence}:{kind}"),
            )
        }
        CheckpointV1::WindowGeometry { width, height } => {
            let snapshot = require_snapshot(context)?;
            require(
                snapshot.window.width == *width && snapshot.window.height == *height,
                format!(
                    "expected window {width}x{height}; observed {}x{}",
                    snapshot.window.width, snapshot.window.height
                ),
                format!("window_geometry={width}x{height}"),
            )
        }
        CheckpointV1::RenderProbe { region, digest } => {
            let snapshot = require_snapshot(context)?;
            let observed = snapshot.runtime.render_digest.as_deref();
            require(
                observed == Some(digest.as_str()),
                format!("render probe {region:?} expected {digest}; observed {observed:?}"),
                format!("render_probe={region}:{digest}"),
            )
        }
        _ => unreachable!("observer checkpoint dispatcher is exhaustive"),
    }
}

fn evaluate_external_checkpoint(
    checkpoint: &CheckpointV1,
    context: &CheckpointContext<'_>,
) -> Result<String, CheckpointError> {
    match checkpoint {
        CheckpointV1::FileSha256 { path, sha256 } => {
            let root = context
                .artifact_root
                .ok_or_else(|| CheckpointError("artifact root is unavailable".to_owned()))?;
            let relative = Path::new(path);
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|component| matches!(component, std::path::Component::ParentDir))
            {
                return Err(CheckpointError(
                    "file checkpoint path must stay relative to the artifact root".to_owned(),
                ));
            }
            let contents = fs::read(root.join(relative))
                .map_err(|error| CheckpointError(format!("read `{path}`: {error}")))?;
            let observed = encode_hex(&Sha256::digest(contents));
            require(
                observed.eq_ignore_ascii_case(sha256),
                format!("expected SHA-256 {sha256}; observed {observed}"),
                format!("file_sha256={path}:{observed}"),
            )
        }
        CheckpointV1::NetworkBytes { fixture, bytes_hex } => {
            let observed = context.network_bytes.get(fixture.as_str()).ok_or_else(|| {
                CheckpointError(format!("network fixture {fixture:?} has no observation"))
            })?;
            let expected = decode_hex(bytes_hex)?;
            require(
                *observed == expected,
                format!(
                    "network fixture {fixture:?} bytes differ: observed {}",
                    encode_hex(observed)
                ),
                format!("network_bytes={fixture}:{bytes_hex}"),
            )
        }
        CheckpointV1::ExitStatus { code } => {
            let observed = context
                .exit_code
                .ok_or_else(|| CheckpointError("process exit status is unavailable".to_owned()))?;
            require(
                observed == *code,
                format!("expected exit status {code}; observed {observed}"),
                format!("exit_status={observed}"),
            )
        }
        CheckpointV1::ResourcesZero => require(
            context.resources_zero,
            "owned processes, workers, listeners, ports, or temporary files remain".to_owned(),
            "resources_zero=true".to_owned(),
        ),
        _ => unreachable!("external checkpoint dispatcher is exhaustive"),
    }
}

fn require_snapshot<'a>(
    context: &'a CheckpointContext<'a>,
) -> Result<&'a ObserverSnapshotV1, CheckpointError> {
    context
        .snapshot
        .ok_or_else(|| CheckpointError("observer snapshot is unavailable".to_owned()))
}

fn require(condition: bool, failure: String, success: String) -> Result<String, CheckpointError> {
    if condition {
        Ok(success)
    } else {
        Err(CheckpointError(failure))
    }
}

fn decode_hex(value: &str) -> Result<Vec<u8>, CheckpointError> {
    if value.len() % 2 != 0 {
        return Err(CheckpointError("hexadecimal byte count is odd".to_owned()));
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair)
                .map_err(|error| CheckpointError(format!("invalid hexadecimal: {error}")))?;
            u8::from_str_radix(pair, 16)
                .map_err(|error| CheckpointError(format!("invalid hexadecimal: {error}")))
        })
        .collect()
}

fn encode_hex(bytes: &[u8]) -> String {
    use fmt::Write as _;

    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}
