use std::{
    error::Error,
    fmt,
    io::{self, BufRead, BufReader, Read, Write},
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioRunId {
    pub scenario_id: String,
    pub target: String,
    pub attempt: u8,
}

impl ScenarioRunId {
    /// Creates a validated evidence-run identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the scenario or target identity is empty.
    pub fn new(scenario_id: &str, target: &str, attempt: u8) -> Result<Self, RunIdError> {
        if scenario_id.is_empty() || target.is_empty() {
            return Err(RunIdError);
        }
        Ok(Self {
            scenario_id: scenario_id.to_owned(),
            target: target.to_owned(),
            attempt,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunIdError;

impl fmt::Display for RunIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("scenario and target must be non-empty")
    }
}

impl Error for RunIdError {}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceEventV1 {
    pub schema: u16,
    pub sequence: u64,
    pub run_id: ScenarioRunId,
    pub monotonic_ms: u64,
    #[serde(flatten)]
    pub payload: EvidencePayloadV1,
}

impl EvidenceEventV1 {
    pub fn scenario_started<const N: usize>(monotonic_ms: u64, capabilities: [&str; N]) -> Self {
        Self::unbound(
            monotonic_ms,
            EvidencePayloadV1::ScenarioStarted {
                capabilities: capabilities.into_iter().map(str::to_owned).collect(),
            },
        )
    }

    #[must_use]
    pub fn scenario_started_with_capabilities(
        monotonic_ms: u64,
        capabilities: Vec<String>,
    ) -> Self {
        Self::unbound(
            monotonic_ms,
            EvidencePayloadV1::ScenarioStarted { capabilities },
        )
    }

    #[must_use]
    pub fn action_finished(
        monotonic_ms: u64,
        action_index: usize,
        action: &str,
        result: &str,
    ) -> Self {
        Self::unbound(
            monotonic_ms,
            EvidencePayloadV1::ActionFinished {
                action_index,
                action: action.to_owned(),
                result: result.to_owned(),
            },
        )
    }

    #[must_use]
    pub fn scenario_finished(monotonic_ms: u64, outcome: ScenarioOutcome) -> Self {
        Self::unbound(
            monotonic_ms,
            EvidencePayloadV1::ScenarioFinished { outcome },
        )
    }

    #[must_use]
    pub fn behavior_observed(monotonic_ms: u64, behavior_id: &str, evidence: &str) -> Self {
        Self::unbound(
            monotonic_ms,
            EvidencePayloadV1::BehaviorObserved {
                behavior_id: behavior_id.to_owned(),
                evidence: evidence.to_owned(),
            },
        )
    }

    #[must_use]
    pub fn checkpoint_finished(
        monotonic_ms: u64,
        checkpoint_index: usize,
        checkpoint: &str,
        passed: bool,
        detail: &str,
    ) -> Self {
        Self::unbound(
            monotonic_ms,
            EvidencePayloadV1::CheckpointFinished {
                checkpoint_index,
                checkpoint: checkpoint.to_owned(),
                passed,
                detail: detail.to_owned(),
            },
        )
    }

    /// Reads and validates a sequence of NDJSON evidence events.
    ///
    /// # Errors
    ///
    /// Returns an I/O or JSON error annotated with the failing line.
    pub fn read_ndjson(reader: impl Read) -> Result<Vec<Self>, EvidenceReadError> {
        BufReader::new(reader)
            .lines()
            .enumerate()
            .map(|(index, line)| {
                let line = line.map_err(EvidenceReadError::Io)?;
                serde_json::from_str(&line).map_err(|source| EvidenceReadError::Json {
                    line: index + 1,
                    source,
                })
            })
            .collect()
    }

    fn unbound(monotonic_ms: u64, payload: EvidencePayloadV1) -> Self {
        Self {
            schema: 1,
            sequence: 0,
            run_id: ScenarioRunId {
                scenario_id: String::new(),
                target: String::new(),
                attempt: 0,
            },
            monotonic_ms,
            payload,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum EvidencePayloadV1 {
    ScenarioStarted {
        capabilities: Vec<String>,
    },
    ActionFinished {
        action_index: usize,
        action: String,
        result: String,
    },
    CheckpointFinished {
        checkpoint_index: usize,
        checkpoint: String,
        passed: bool,
        detail: String,
    },
    BehaviorObserved {
        behavior_id: String,
        evidence: String,
    },
    ScenarioFinished {
        outcome: ScenarioOutcome,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioOutcome {
    Passed,
    Failed,
    InfrastructureFailed,
}

pub struct EvidenceWriter<W> {
    output: W,
    run_id: ScenarioRunId,
    next_sequence: u64,
    last_monotonic_ms: Option<u64>,
}

impl<W: Write> EvidenceWriter<W> {
    pub fn new(output: W, run_id: ScenarioRunId) -> Self {
        Self {
            output,
            run_id,
            next_sequence: 1,
            last_monotonic_ms: None,
        }
    }

    /// Appends one sequenced event to the evidence stream.
    ///
    /// # Errors
    ///
    /// Returns an error for decreasing monotonic time or output failures.
    pub fn record(&mut self, mut event: EvidenceEventV1) -> Result<(), EvidenceWriteError> {
        if self
            .last_monotonic_ms
            .is_some_and(|previous| event.monotonic_ms < previous)
        {
            return Err(EvidenceWriteError::DecreasingMonotonicTime);
        }
        event.schema = 1;
        event.sequence = self.next_sequence;
        event.run_id = self.run_id.clone();
        serde_json::to_writer(&mut self.output, &event).map_err(EvidenceWriteError::Json)?;
        self.output
            .write_all(b"\n")
            .map_err(EvidenceWriteError::Io)?;
        self.last_monotonic_ms = Some(event.monotonic_ms);
        self.next_sequence = self.next_sequence.saturating_add(1);
        Ok(())
    }
}

#[derive(Debug)]
pub enum EvidenceWriteError {
    DecreasingMonotonicTime,
    Json(serde_json::Error),
    Io(io::Error),
}

impl fmt::Display for EvidenceWriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DecreasingMonotonicTime => formatter.write_str("monotonic time decreased"),
            Self::Json(source) => write!(formatter, "serialize evidence: {source}"),
            Self::Io(source) => write!(formatter, "write evidence: {source}"),
        }
    }
}

impl Error for EvidenceWriteError {}

#[derive(Debug)]
pub enum EvidenceReadError {
    Io(io::Error),
    Json {
        line: usize,
        source: serde_json::Error,
    },
}

impl fmt::Display for EvidenceReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(source) => write!(formatter, "read evidence: {source}"),
            Self::Json { line, source } => {
                write!(formatter, "parse evidence line {line}: {source}")
            }
        }
    }
}

impl Error for EvidenceReadError {}
