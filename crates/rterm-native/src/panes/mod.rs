use std::collections::HashMap;

use rterm_domain::PaneId;
use rterm_runtime::{
    EffectSequenceCursor, PaneToken, RuntimeProgress, RuntimeRevision, TerminalStateSummary,
};

use crate::{
    PaneSplitDirection, RuntimePortEffect, SpawnEffect, WindowEffect, WindowPortEffect,
    WindowState, controller::request_redraw,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneCommand {
    Spawn,
    Split {
        source: PaneId,
        direction: PaneSplitDirection,
    },
    Activate(PaneId),
    Close(PaneId),
    Restart(PaneId),
}

/// Pane ownership changes emitted by the runtime composition root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneLifecycleIntent {
    Opened(PaneToken),
    Activated(PaneToken),
    Closed(PaneToken),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneState {
    pub token: PaneToken,
    pub revision: Option<RuntimeRevision>,
    pub snapshot: Option<TerminalStateSummary>,
    pub title: Option<String>,
    pub working_directory: Option<String>,
    pub badge_format: Option<String>,
    pub progress: RuntimeProgress,
    pub user_vars: HashMap<String, String>,
    pub restarting: bool,
    pub closing: bool,
    pub effect_sequence: EffectSequenceCursor,
}

impl PaneState {
    #[must_use]
    pub fn new(token: PaneToken) -> Self {
        Self {
            token,
            revision: None,
            snapshot: None,
            title: None,
            working_directory: None,
            badge_format: None,
            progress: RuntimeProgress::None,
            user_vars: HashMap::new(),
            restarting: false,
            closing: false,
            effect_sequence: EffectSequenceCursor::default(),
        }
    }
}

pub(crate) fn reduce_command(
    state: &mut WindowState,
    command: PaneCommand,
    effects: &mut Vec<WindowEffect>,
) {
    match command {
        PaneCommand::Spawn => effects.push(WindowEffect::Spawn(SpawnEffect::Pane)),
        PaneCommand::Split { source, direction } => {
            if let Some(pane) = state.panes.get(&source).filter(|pane| !pane.closing) {
                effects.push(WindowEffect::Spawn(SpawnEffect::SplitPane {
                    source: pane.token,
                    direction,
                }));
            }
        }
        PaneCommand::Activate(pane_id) => {
            let Some(token) = state.panes.get(&pane_id).map(|pane| pane.token) else {
                return;
            };
            reduce_lifecycle(state, PaneLifecycleIntent::Activated(token), effects);
        }
        PaneCommand::Close(pane_id) => {
            let Some(pane) = state.panes.get_mut(&pane_id) else {
                return;
            };
            if pane.closing {
                return;
            }
            pane.closing = true;
            effects.push(WindowEffect::Runtime(RuntimePortEffect::BeginClose {
                pane: pane.token,
            }));
        }
        PaneCommand::Restart(pane_id) => {
            if let Some(pane) = state.panes.get_mut(&pane_id) {
                pane.restarting = true;
                effects.push(WindowEffect::Runtime(RuntimePortEffect::Restart {
                    pane: pane.token,
                }));
            }
        }
    }
}

pub(crate) fn reduce_lifecycle(
    state: &mut WindowState,
    intent: PaneLifecycleIntent,
    effects: &mut Vec<WindowEffect>,
) {
    match intent {
        PaneLifecycleIntent::Opened(token) => {
            if state
                .panes
                .get(&token.pane())
                .is_some_and(|pane| pane.token.generation() >= token.generation())
            {
                return;
            }
            if !state.pane_order.contains(&token.pane()) {
                state.pane_order.push(token.pane());
            }
            state.panes.insert(token.pane(), PaneState::new(token));
            if state.active_pane.is_none() {
                state.active_pane = Some(token.pane());
            }
        }
        PaneLifecycleIntent::Activated(token) => {
            if state.panes.get(&token.pane()).map(|pane| pane.token) != Some(token)
                || state.active_pane == Some(token.pane())
            {
                return;
            }
            state.active_pane = Some(token.pane());
            request_redraw(state, effects);
        }
        PaneLifecycleIntent::Closed(token) => {
            if state.panes.get(&token.pane()).map(|pane| pane.token) == Some(token) {
                state.panes.remove(&token.pane());
                let closed_position = state
                    .pane_order
                    .iter()
                    .position(|pane| *pane == token.pane());
                if let Some(position) = closed_position {
                    state.pane_order.remove(position);
                    if state.active_pane == Some(token.pane()) {
                        state.active_pane = state
                            .pane_order
                            .get(position)
                            .or_else(|| state.pane_order.last())
                            .copied();
                        if !state.lifecycle.closing {
                            request_redraw(state, effects);
                        }
                    }
                }
                if state.lifecycle.closing && state.panes.is_empty() {
                    effects.push(WindowEffect::Window(WindowPortEffect::CloseNow));
                }
            }
        }
    }
}
