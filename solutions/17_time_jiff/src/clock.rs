//! Injectable wall-clock source, so "now" can be fixed in tests.

use jiff::Timestamp;

/// Source of "now". Production code uses [`SystemClock`]; tests inject
/// [`FixedClock`].
pub trait Clock {
    /// The current instant.
    fn now(&self) -> Timestamp;
}

/// The operating system's wall clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }
}

/// A clock frozen at one instant.
#[derive(Debug, Clone, Copy)]
pub struct FixedClock {
    now: Timestamp,
}

impl FixedClock {
    /// A clock that always reports `now`.
    pub fn new(now: Timestamp) -> Self {
        Self { now }
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        self.now
    }
}
