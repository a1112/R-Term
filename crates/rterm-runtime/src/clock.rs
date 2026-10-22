use std::time::Instant;

/// Monotonic time source used by runtime scheduling and deadline logic.
pub trait Clock: Clone + Send + Sync + 'static {
    /// Returns the current monotonic instant.
    fn now(&self) -> Instant;
}

/// Production monotonic clock backed by [`Instant::now`].
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}
