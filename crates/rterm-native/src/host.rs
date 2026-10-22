use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::Duration,
};

use rterm_domain::WindowId;
use rterm_runtime::{Clock, PaneToken, SystemClock};
use rterm_types::TerminalSize;

use crate::{
    CommandIntent, ConfigDiff, HostPorts, PaneLifecycleIntent, PlatformIntent, PortError, PortKind,
    TimerIntent, WindowEffect, WindowIntent, WindowState, reduce,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformEvent {
    Focused(bool),
    Resized(TerminalSize),
    Command(CommandIntent),
    Config(ConfigDiff),
    Timer(TimerIntent),
    PaneLifecycle(PaneLifecycleIntent),
    RuntimeWake { pane: PaneToken },
    RuntimeContinuation { pane: PaneToken },
    RedrawRequested,
    CloseRequested,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeDrain {
    pub intents: Vec<WindowIntent>,
    pub continuation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnBudget {
    max_runtime_intents: usize,
    max_duration: Duration,
}

impl TurnBudget {
    #[must_use]
    pub const fn new(max_runtime_intents: usize, max_duration: Duration) -> Self {
        Self {
            max_runtime_intents: if max_runtime_intents == 0 {
                1
            } else {
                max_runtime_intents
            },
            max_duration,
        }
    }

    #[must_use]
    pub const fn max_runtime_intents(self) -> usize {
        self.max_runtime_intents
    }

    #[must_use]
    pub const fn max_duration(self) -> Duration {
        self.max_duration
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HostTurn {
    pub effects_executed: usize,
    pub runtime_intents: usize,
    pub continuation_scheduled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostError {
    effect_index: usize,
    port: PortKind,
    source: PortError,
}

impl HostError {
    #[must_use]
    pub const fn new(effect_index: usize, port: PortKind, source: PortError) -> Self {
        Self {
            effect_index,
            port,
            source,
        }
    }

    #[must_use]
    pub const fn effect_index(&self) -> usize {
        self.effect_index
    }

    #[must_use]
    pub const fn port(&self) -> PortKind {
        self.port
    }

    #[must_use]
    pub const fn source_error(&self) -> &PortError {
        &self.source
    }
}

impl std::fmt::Display for HostError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "host effect {} failed on {:?}: {}",
            self.effect_index, self.port, self.source
        )
    }
}

impl std::error::Error for HostError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Platform-neutral owner of reducer state and ordered typed-port dispatch.
pub struct WinitHost<P, C = SystemClock> {
    window: WindowId,
    state: WindowState,
    ports: P,
    clock: C,
    budget: TurnBudget,
    continuation_pending: HashSet<PaneToken>,
    pending_runtime_intents: HashMap<PaneToken, VecDeque<WindowIntent>>,
    source_continuation: HashSet<PaneToken>,
}

impl<P: HostPorts> WinitHost<P, SystemClock> {
    #[must_use]
    pub fn new(window: WindowId, state: WindowState, ports: P, budget: TurnBudget) -> Self {
        Self::with_clock(window, state, ports, budget, SystemClock)
    }
}

impl<P: HostPorts, C: Clock> WinitHost<P, C> {
    #[must_use]
    pub fn with_clock(
        window: WindowId,
        state: WindowState,
        ports: P,
        budget: TurnBudget,
        clock: C,
    ) -> Self {
        Self {
            window,
            state,
            ports,
            clock,
            budget,
            continuation_pending: HashSet::new(),
            pending_runtime_intents: HashMap::new(),
            source_continuation: HashSet::new(),
        }
    }

    #[must_use]
    pub const fn state(&self) -> &WindowState {
        &self.state
    }

    #[must_use]
    pub const fn ports(&self) -> &P {
        &self.ports
    }

    #[must_use]
    pub fn ports_mut(&mut self) -> &mut P {
        &mut self.ports
    }

    /// Handles one token-only platform observation or parsed domain event.
    ///
    /// # Errors
    ///
    /// Returns [`HostError`] when a typed port rejects a reducer effect,
    /// runtime drain, or continuation wake.
    pub fn handle(&mut self, event: PlatformEvent) -> Result<HostTurn, HostError> {
        match event {
            PlatformEvent::RuntimeWake { pane } => {
                if !self.is_current_runtime_token(pane) {
                    return Ok(HostTurn::default());
                }
                if self.continuation_pending.contains(&pane) {
                    Ok(HostTurn::default())
                } else {
                    self.drain_runtime_turn(pane)
                }
            }
            PlatformEvent::RuntimeContinuation { pane } => {
                if !self.is_current_runtime_token(pane) {
                    self.clear_runtime_turn_state(pane);
                    return Ok(HostTurn::default());
                }
                self.continuation_pending.remove(&pane);
                self.drain_runtime_turn(pane)
            }
            PlatformEvent::Focused(focused) => {
                self.dispatch_intent(WindowIntent::Platform(PlatformIntent::Focused(focused)))
            }
            PlatformEvent::Resized(size) => {
                self.dispatch_intent(WindowIntent::Platform(PlatformIntent::Resized(size)))
            }
            PlatformEvent::Command(command) => self.dispatch_intent(WindowIntent::Command(command)),
            PlatformEvent::Config(diff) => self.dispatch_intent(WindowIntent::Config(diff)),
            PlatformEvent::Timer(timer) => self.dispatch_intent(WindowIntent::Timer(timer)),
            PlatformEvent::PaneLifecycle(event) => self.dispatch_pane_lifecycle(event),
            PlatformEvent::RedrawRequested => self.dispatch_intent(WindowIntent::RedrawRequested),
            PlatformEvent::CloseRequested => self.dispatch_intent(WindowIntent::CloseRequested),
        }
    }

    /// Executes an already-reduced effect list in exact source order.
    ///
    /// # Errors
    ///
    /// Stops at the first rejected effect and returns its stable index and
    /// typed port classification.
    pub fn execute_effects(&mut self, effects: &[WindowEffect]) -> Result<HostTurn, HostError> {
        let mut turn = HostTurn::default();
        for (index, effect) in effects.iter().enumerate() {
            self.execute_effect(index, effect)?;
            turn.effects_executed += 1;
        }
        Ok(turn)
    }

    fn dispatch_intent(&mut self, intent: WindowIntent) -> Result<HostTurn, HostError> {
        let mut effects = Vec::new();
        reduce(&mut self.state, intent, &mut effects);
        self.execute_effects(&effects)
    }

    fn dispatch_pane_lifecycle(
        &mut self,
        event: PaneLifecycleIntent,
    ) -> Result<HostTurn, HostError> {
        let turn = self.dispatch_intent(WindowIntent::PaneLifecycle(event))?;
        let token = match event {
            PaneLifecycleIntent::Opened(token)
            | PaneLifecycleIntent::Activated(token)
            | PaneLifecycleIntent::Closed(token) => token,
        };
        let current = self.state.panes.get(&token.pane()).map(|pane| pane.token);
        self.continuation_pending
            .retain(|candidate| candidate.pane() != token.pane() || Some(*candidate) == current);
        self.pending_runtime_intents
            .retain(|candidate, _| candidate.pane() != token.pane() || Some(*candidate) == current);
        self.source_continuation
            .retain(|candidate| candidate.pane() != token.pane() || Some(*candidate) == current);
        Ok(turn)
    }

    fn is_current_runtime_token(&self, token: PaneToken) -> bool {
        self.state
            .panes
            .get(&token.pane())
            .is_some_and(|pane| pane.token == token)
    }

    fn clear_runtime_turn_state(&mut self, token: PaneToken) {
        self.continuation_pending.remove(&token);
        self.pending_runtime_intents.remove(&token);
        self.source_continuation.remove(&token);
    }

    fn drain_runtime_turn(&mut self, pane: PaneToken) -> Result<HostTurn, HostError> {
        let budget = self.budget.max_runtime_intents();
        if budget == 0 {
            return self.schedule_continuation(pane, HostTurn::default());
        }
        let queue = self.pending_runtime_intents.entry(pane).or_default();
        if queue.is_empty() {
            let drain = self
                .ports
                .drain_runtime(pane, budget)
                .map_err(|error| HostError::new(0, PortKind::Runtime, error))?;
            queue.extend(drain.intents);
            if drain.continuation {
                self.source_continuation.insert(pane);
            }
        }

        let mut turn = HostTurn::default();
        let started = self.clock.now();
        for _ in 0..budget {
            if turn.runtime_intents != 0
                && self.clock.now().saturating_duration_since(started) >= self.budget.max_duration()
            {
                break;
            }
            let Some(intent) = self
                .pending_runtime_intents
                .get_mut(&pane)
                .and_then(VecDeque::pop_front)
            else {
                break;
            };
            let intent_turn = self.dispatch_intent(intent)?;
            turn.effects_executed = turn
                .effects_executed
                .saturating_add(intent_turn.effects_executed);
            turn.runtime_intents = turn.runtime_intents.saturating_add(1);
        }
        let pending = self
            .pending_runtime_intents
            .get(&pane)
            .is_some_and(|queue| !queue.is_empty());
        if !pending {
            self.pending_runtime_intents.remove(&pane);
        }
        if pending || self.source_continuation.remove(&pane) {
            self.schedule_continuation(pane, turn)
        } else {
            Ok(turn)
        }
    }

    fn schedule_continuation(
        &mut self,
        pane: PaneToken,
        mut turn: HostTurn,
    ) -> Result<HostTurn, HostError> {
        if self.continuation_pending.insert(pane) {
            if let Err(error) = self.ports.schedule_runtime_continuation(self.window, pane) {
                self.continuation_pending.remove(&pane);
                return Err(HostError::new(
                    turn.effects_executed,
                    PortKind::Platform,
                    error,
                ));
            }
            turn.continuation_scheduled = true;
        }
        Ok(turn)
    }

    fn execute_effect(&mut self, index: usize, effect: &WindowEffect) -> Result<(), HostError> {
        let (port, result) = match effect {
            WindowEffect::Runtime(effect) => (PortKind::Runtime, self.ports.runtime(effect)),
            WindowEffect::Window(effect) => (PortKind::Window, self.ports.window(effect)),
            WindowEffect::Renderer(effect) => (PortKind::Renderer, self.ports.renderer(effect)),
            WindowEffect::Clipboard(effect) => (PortKind::Clipboard, self.ports.clipboard(effect)),
            WindowEffect::Uri(effect) => (PortKind::Uri, self.ports.uri(effect)),
            WindowEffect::Notification(effect) => {
                (PortKind::Notification, self.ports.notification(effect))
            }
            WindowEffect::Persistence(effect) => {
                (PortKind::Persistence, self.ports.persistence(effect))
            }
            WindowEffect::Spawn(effect) => (PortKind::Spawn, self.ports.spawn(effect)),
        };
        result.map_err(|error| HostError::new(index, port, error))
    }
}
