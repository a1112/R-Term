use rterm_types::SessionId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Created,
    Connecting,
    Connected,
    Disconnected,
    Closed,
}

impl SessionState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Connecting => "connecting",
            Self::Connected => "connected",
            Self::Disconnected => "disconnected",
            Self::Closed => "closed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionTransitionError {
    from: SessionState,
    to: SessionState,
}

impl SessionTransitionError {
    #[must_use]
    pub const fn from(&self) -> SessionState {
        self.from
    }

    #[must_use]
    pub const fn to(&self) -> SessionState {
        self.to
    }
}

impl std::fmt::Display for SessionTransitionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "invalid session transition from {:?} to {:?}",
            self.from, self.to
        )
    }
}

impl std::error::Error for SessionTransitionError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionLifecycle {
    id: SessionId,
    state: SessionState,
}

impl SessionLifecycle {
    /// Creates a new lifecycle in the [`SessionState::Created`] state.
    #[must_use]
    pub const fn new(id: SessionId) -> Self {
        Self {
            id,
            state: SessionState::Created,
        }
    }

    #[must_use]
    pub const fn id(&self) -> SessionId {
        self.id
    }

    #[must_use]
    pub const fn state(&self) -> SessionState {
        self.state
    }

    /// Moves a created or disconnected session into the connecting state.
    ///
    /// # Errors
    ///
    /// Returns [`SessionTransitionError`] when the current state cannot start a
    /// connection attempt, including connected or closed sessions.
    pub fn start_connecting(&mut self) -> Result<(), SessionTransitionError> {
        self.transition(SessionState::Connecting)
    }

    /// Marks a connecting session as connected.
    ///
    /// # Errors
    ///
    /// Returns [`SessionTransitionError`] unless the current state is
    /// [`SessionState::Connecting`].
    pub fn mark_connected(&mut self) -> Result<(), SessionTransitionError> {
        self.transition(SessionState::Connected)
    }

    /// Marks a connecting or connected session as disconnected.
    ///
    /// # Errors
    ///
    /// Returns [`SessionTransitionError`] when the current state cannot become
    /// disconnected, including created or closed sessions.
    pub fn mark_disconnected(&mut self) -> Result<(), SessionTransitionError> {
        self.transition(SessionState::Disconnected)
    }

    /// Moves any non-closed session to the terminal closed state.
    ///
    /// # Errors
    ///
    /// Returns [`SessionTransitionError`] when the session is already closed.
    pub fn close(&mut self) -> Result<(), SessionTransitionError> {
        self.transition(SessionState::Closed)
    }

    fn transition(&mut self, next: SessionState) -> Result<(), SessionTransitionError> {
        if !valid_transition(self.state, next) {
            return Err(SessionTransitionError {
                from: self.state,
                to: next,
            });
        }

        self.state = next;
        Ok(())
    }
}

const fn valid_transition(from: SessionState, to: SessionState) -> bool {
    matches!(
        (from, to),
        (
            SessionState::Created | SessionState::Disconnected,
            SessionState::Connecting,
        ) | (
            SessionState::Created
                | SessionState::Connecting
                | SessionState::Connected
                | SessionState::Disconnected,
            SessionState::Closed,
        ) | (
            SessionState::Connecting,
            SessionState::Connected | SessionState::Disconnected,
        ) | (SessionState::Connected, SessionState::Disconnected)
    )
}

#[cfg(test)]
mod tests {
    use crate::{
        SessionId,
        session::{SessionLifecycle, SessionState},
    };

    #[test]
    fn session_lifecycle_starts_created() {
        let lifecycle = SessionLifecycle::new(SessionId::new(7));

        assert_eq!(lifecycle.id(), SessionId::new(7));
        assert_eq!(lifecycle.state(), SessionState::Created);
    }

    #[test]
    fn session_lifecycle_moves_through_runtime_states() {
        let mut lifecycle = SessionLifecycle::new(SessionId::new(7));

        lifecycle.start_connecting().unwrap();
        assert_eq!(lifecycle.state(), SessionState::Connecting);

        lifecycle.mark_connected().unwrap();
        assert_eq!(lifecycle.state(), SessionState::Connected);

        lifecycle.mark_disconnected().unwrap();
        assert_eq!(lifecycle.state(), SessionState::Disconnected);

        lifecycle.start_connecting().unwrap();
        assert_eq!(lifecycle.state(), SessionState::Connecting);

        lifecycle.close().unwrap();
        assert_eq!(lifecycle.state(), SessionState::Closed);
    }

    #[test]
    fn session_lifecycle_rejects_invalid_transitions() {
        let mut lifecycle = SessionLifecycle::new(SessionId::new(7));

        let error = lifecycle.mark_connected().unwrap_err();

        assert_eq!(error.from(), SessionState::Created);
        assert_eq!(error.to(), SessionState::Connected);
        assert_eq!(lifecycle.state(), SessionState::Created);
    }

    #[test]
    fn closed_session_rejects_new_work() {
        let mut lifecycle = SessionLifecycle::new(SessionId::new(7));

        lifecycle.close().unwrap();
        let error = lifecycle.start_connecting().unwrap_err();

        assert_eq!(error.from(), SessionState::Closed);
        assert_eq!(error.to(), SessionState::Connecting);
        assert_eq!(lifecycle.state(), SessionState::Closed);
    }

    #[test]
    fn session_state_names_are_stable_for_logs() {
        assert_eq!(SessionState::Created.as_str(), "created");
        assert_eq!(SessionState::Connecting.as_str(), "connecting");
        assert_eq!(SessionState::Connected.as_str(), "connected");
        assert_eq!(SessionState::Disconnected.as_str(), "disconnected");
        assert_eq!(SessionState::Closed.as_str(), "closed");
    }

    #[test]
    fn transition_errors_are_readable() {
        let mut lifecycle = SessionLifecycle::new(SessionId::new(7));

        let error = lifecycle.mark_connected().unwrap_err();

        assert_eq!(
            error.to_string(),
            "invalid session transition from Created to Connected"
        );
    }
}
