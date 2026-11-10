//! Part 7: an injectable wall clock.

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
        todo!()
    }
}

/// A clock frozen at one instant.
#[derive(Debug, Clone, Copy)]
pub struct FixedClock {
    _now: Timestamp,
}

impl FixedClock {
    /// A clock that always reports `now`.
    pub fn new(_now: Timestamp) -> Self {
        todo!()
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        todo!()
    }
}
