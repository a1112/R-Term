use std::{collections::BTreeSet, error::Error, fmt};

use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioV1 {
    pub schema: u16,
    pub id: String,
    pub behavior_ids: Vec<BehaviorId>,
    pub surface: Surface,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    pub fixture: String,
    pub estimated_cost_ms: u64,
    #[serde(default)]
    pub deadlines: DeadlinesV1,
    pub actions: Vec<ActionV1>,
    pub checkpoints: Vec<CheckpointV1>,
    pub required_evidence: Vec<EvidenceKind>,
}

impl ScenarioV1 {
    /// Parses and validates a version-one scenario document.
    ///
    /// # Errors
    ///
    /// Returns a parse or validation error for an invalid scenario.
    pub fn from_toml(contents: &str) -> Result<Self, ScenarioParseError> {
        let scenario: Self = toml::from_str(contents).map_err(ScenarioParseError::Toml)?;
        scenario
            .validate()
            .map_err(ScenarioParseError::Validation)?;
        Ok(scenario)
    }

    /// Validates closed actions, checkpoints, identities, evidence, and deadlines.
    ///
    /// # Errors
    ///
    /// Returns the first structural scenario violation.
    pub fn validate(&self) -> Result<(), ScenarioValidationError> {
        if self.schema != SCHEMA_VERSION {
            return Err(ScenarioValidationError::UnsupportedSchema(self.schema));
        }
        if !is_stable_scenario_id(&self.id) {
            return Err(ScenarioValidationError::InvalidScenarioId(self.id.clone()));
        }
        if self.behavior_ids.is_empty() {
            return Err(ScenarioValidationError::MissingBehaviorId);
        }
        if self.behavior_ids.iter().collect::<BTreeSet<_>>().len() != self.behavior_ids.len() {
            return Err(ScenarioValidationError::DuplicateBehaviorId);
        }
        if self.required_evidence.iter().collect::<BTreeSet<_>>().len()
            != self.required_evidence.len()
        {
            return Err(ScenarioValidationError::DuplicateEvidenceKind);
        }
        if self.actions.is_empty() {
            return Err(ScenarioValidationError::MissingAction);
        }
        validate_deadlines(&self.deadlines)?;
        for action in &self.actions {
            if let ActionV1::PtyInput { bytes_hex } = action {
                validate_hex("pty_input.bytes_hex", bytes_hex)?;
            }
        }
        for checkpoint in &self.checkpoints {
            validate_checkpoint(checkpoint)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct BehaviorId(pub String);

impl AsRef<str> for BehaviorId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    Console,
    HostTerminal,
    NativeWindow,
    Web,
    Tauri,
    Package,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    RealStdinPty,
    RealOsKeyboard,
    RealOsPointer,
    SystemClipboard,
    X11,
    Wayland,
    MacosAccessibility,
    BrowserChromium,
    BrowserFirefox,
    BrowserWebkit,
    SystemOpenssh,
    NativeSsh,
    GpuReadback,
    RealHostTerminal,
    ProductionObserverIsolation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActionV1 {
    TypeText {
        text: String,
    },
    Key {
        key: String,
        #[serde(default)]
        modifiers: Vec<KeyModifier>,
    },
    MouseClick {
        x: i32,
        y: i32,
        button: MouseButton,
    },
    MouseDrag {
        from_x: i32,
        from_y: i32,
        to_x: i32,
        to_y: i32,
        button: MouseButton,
    },
    MouseWheel {
        delta_x: i32,
        delta_y: i32,
    },
    ClipboardPaste {
        text: String,
    },
    ResizeWindow {
        width: u32,
        height: u32,
    },
    WindowControl {
        operation: WindowControl,
    },
    FocusWindow,
    PtyInput {
        bytes_hex: String,
    },
    FixtureDisconnect {
        fixture: String,
    },
    FixtureReconnect {
        fixture: String,
    },
    Finish,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyModifier {
    Shift,
    Ctrl,
    Alt,
    Super,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowControl {
    Minimize,
    Maximize,
    Restore,
    Close,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum CheckpointV1 {
    TerminalContains {
        text: String,
    },
    Cursor {
        row: u32,
        column: u32,
    },
    TerminalMode {
        name: String,
        enabled: bool,
    },
    Pane {
        tab_id: u64,
        pane_id: u64,
        active: bool,
    },
    Overlay {
        name: String,
        visible: bool,
    },
    Transport {
        state: String,
    },
    ConfigGeneration {
        generation: u64,
    },
    ConfigDiagnostic {
        present: bool,
    },
    HostEffect {
        kind: String,
        sequence: u64,
    },
    WindowGeometry {
        width: u32,
        height: u32,
    },
    FileSha256 {
        path: String,
        sha256: String,
    },
    NetworkBytes {
        fixture: String,
        bytes_hex: String,
    },
    ExitStatus {
        code: i32,
    },
    ResourcesZero,
    RenderProbe {
        region: String,
        digest: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    EventLog,
    Stdout,
    Stderr,
    FinalSnapshot,
    ServerTrace,
    ProcessTree,
    CompositorLog,
    ScreenshotOnFailure,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DeadlinesV1 {
    pub action_ms: u64,
    pub startup_ms: u64,
    pub cleanup_ms: u64,
    pub scenario_ms: u64,
}

impl Default for DeadlinesV1 {
    fn default() -> Self {
        Self {
            action_ms: 15_000,
            startup_ms: 30_000,
            cleanup_ms: 10_000,
            scenario_ms: 120_000,
        }
    }
}

#[derive(Debug)]
pub enum ScenarioParseError {
    Toml(toml::de::Error),
    Validation(ScenarioValidationError),
}

impl fmt::Display for ScenarioParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Toml(source) => source.fmt(formatter),
            Self::Validation(source) => source.fmt(formatter),
        }
    }
}

impl Error for ScenarioParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Toml(source) => Some(source),
            Self::Validation(source) => Some(source),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScenarioValidationError {
    UnsupportedSchema(u16),
    InvalidScenarioId(String),
    MissingBehaviorId,
    DuplicateBehaviorId,
    DuplicateEvidenceKind,
    MissingAction,
    InvalidDeadline(&'static str),
    DeadlineExceedsBudget {
        field: &'static str,
        value: u64,
        maximum: u64,
    },
    InvalidHex {
        field: &'static str,
        value: String,
    },
    InvalidSha256(String),
}

impl fmt::Display for ScenarioValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema(version) => {
                write!(formatter, "unsupported scenario schema {version}")
            }
            Self::InvalidScenarioId(id) => write!(formatter, "invalid stable scenario id `{id}`"),
            Self::MissingBehaviorId => {
                formatter.write_str("scenario must reference at least one behavior")
            }
            Self::DuplicateBehaviorId => {
                formatter.write_str("scenario behavior IDs must be unique")
            }
            Self::DuplicateEvidenceKind => {
                formatter.write_str("required evidence kinds must be unique")
            }
            Self::MissingAction => formatter.write_str("scenario must contain at least one action"),
            Self::InvalidDeadline(field) => {
                write!(formatter, "deadline `{field}` must be non-zero")
            }
            Self::DeadlineExceedsBudget {
                field,
                value,
                maximum,
            } => write!(
                formatter,
                "deadline `{field}` value {value} exceeds functional budget {maximum}"
            ),
            Self::InvalidHex { field, value } => write!(
                formatter,
                "invalid hexadecimal value for `{field}`: `{value}`"
            ),
            Self::InvalidSha256(value) => write!(formatter, "invalid SHA-256 value `{value}`"),
        }
    }
}

impl Error for ScenarioValidationError {}

fn validate_deadlines(deadlines: &DeadlinesV1) -> Result<(), ScenarioValidationError> {
    for (name, value, maximum) in [
        ("action_ms", deadlines.action_ms, 15_000),
        ("startup_ms", deadlines.startup_ms, 30_000),
        ("cleanup_ms", deadlines.cleanup_ms, 10_000),
        ("scenario_ms", deadlines.scenario_ms, 120_000),
    ] {
        if value == 0 {
            return Err(ScenarioValidationError::InvalidDeadline(name));
        }
        if value > maximum {
            return Err(ScenarioValidationError::DeadlineExceedsBudget {
                field: name,
                value,
                maximum,
            });
        }
    }
    Ok(())
}

fn validate_checkpoint(checkpoint: &CheckpointV1) -> Result<(), ScenarioValidationError> {
    match checkpoint {
        CheckpointV1::NetworkBytes { bytes_hex, .. } => {
            validate_hex("network_bytes.bytes_hex", bytes_hex)
        }
        CheckpointV1::FileSha256 { sha256, .. }
            if sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) =>
        {
            Err(ScenarioValidationError::InvalidSha256(sha256.clone()))
        }
        _ => Ok(()),
    }
}

fn validate_hex(field: &'static str, value: &str) -> Result<(), ScenarioValidationError> {
    if value.len() % 2 == 0 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(ScenarioValidationError::InvalidHex {
            field,
            value: value.to_owned(),
        })
    }
}

fn is_stable_scenario_id(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
}
