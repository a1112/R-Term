use rterm_config::EffectiveConfig;
use rterm_runtime::{RuntimeBatch, TerminalStateSummary};
use rterm_types::TerminalSize;
use std::sync::Arc;

use crate::{CommandIntent, panes::PaneLifecycleIntent};

/// Stable identifier for a controller-owned timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimerId(u64);

impl TimerId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Platform observations translated without platform-library types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformIntent {
    Focused(bool),
    Resized(TerminalSize),
}

/// Pure configuration changes relevant to the native presentation layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiff {
    pub revision: u64,
    pub effective: Arc<EffectiveConfig>,
    pub theme: Option<String>,
}

impl ConfigDiff {
    #[must_use]
    pub fn new(revision: u64, effective: Arc<EffectiveConfig>, theme: Option<String>) -> Self {
        Self {
            revision,
            effective,
            theme,
        }
    }
}

/// Timer lifecycle with an epoch that rejects stale callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerIntent {
    Arm { timer: TimerId, epoch: u64 },
    Fired { timer: TimerId, epoch: u64 },
    Cancel { timer: TimerId },
}

/// Every input accepted by the deterministic native reducer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowIntent {
    Platform(PlatformIntent),
    Command(CommandIntent),
    RuntimeBatch(RuntimeBatch<TerminalStateSummary>),
    Config(ConfigDiff),
    Timer(TimerIntent),
    PaneLifecycle(PaneLifecycleIntent),
    RedrawRequested,
    CloseRequested,
}
