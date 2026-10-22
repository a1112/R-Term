use std::collections::HashMap;
use std::sync::Arc;

use crate::{PaneState, TimerId};
use rterm_config::EffectiveConfig;
use rterm_domain::PaneId;
use rterm_types::TerminalSize;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LifecycleState {
    pub closing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlatformState {
    pub focused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationState {
    pub size: TerminalSize,
    pub redraw_pending: bool,
}

impl Default for PresentationState {
    fn default() -> Self {
        Self {
            size: TerminalSize::new(80, 24),
            redraw_pending: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfigState {
    pub revision: u64,
    pub effective: Arc<EffectiveConfig>,
    pub theme: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TimerState {
    pub epochs: HashMap<TimerId, u64>,
}

/// Pure native state grouped by ownership domain.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowState {
    pub lifecycle: LifecycleState,
    pub platform: PlatformState,
    pub presentation: PresentationState,
    pub config: ConfigState,
    pub timers: TimerState,
    pub panes: HashMap<PaneId, PaneState>,
    pub pane_order: Vec<PaneId>,
    pub active_pane: Option<PaneId>,
}
