//! Deterministic runtime test support enabled by the `test-support` feature.

use std::error::Error;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::Clock;

mod transport;

pub use transport::{
    ControlCall, ControlLog, ExitAction, ReadAction, ScriptedSessionDriver, ScriptedTransport,
    WriteAction,
};

/// Error returned when virtual monotonic time cannot be represented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualClockAdvanceError {
    /// Adding the requested duration would overflow [`Instant`].
    Overflow,
}

impl fmt::Display for VirtualClockAdvanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow => formatter.write_str("virtual clock advance overflowed Instant"),
        }
    }
}

impl Error for VirtualClockAdvanceError {}

/// Cloneable deterministic monotonic clock advanced only by the test driver.
#[derive(Debug, Clone)]
pub struct VirtualClock {
    now: Arc<Mutex<Instant>>,
}

impl VirtualClock {
    /// Creates a virtual clock at `start`.
    #[must_use]
    pub fn new(start: Instant) -> Self {
        Self {
            now: Arc::new(Mutex::new(start)),
        }
    }

    /// Advances the shared clock without sleeping.
    ///
    /// # Errors
    ///
    /// Returns [`VirtualClockAdvanceError::Overflow`] without changing the
    /// clock if the resulting [`Instant`] cannot be represented.
    pub fn advance(&self, duration: Duration) -> Result<Instant, VirtualClockAdvanceError> {
        let mut now = self.now.lock().unwrap_or_else(PoisonError::into_inner);
        let advanced = now
            .checked_add(duration)
            .ok_or(VirtualClockAdvanceError::Overflow)?;
        *now = advanced;
        Ok(advanced)
    }
}

impl Clock for VirtualClock {
    fn now(&self) -> Instant {
        *self.now.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
