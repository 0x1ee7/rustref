//! Part 7: a monotonic stopwatch with an injectable clock.

use std::time::{Duration, Instant};

use crate::TimeError;

/// Source of monotonic readings. Tests inject a fake; production uses
/// [`SystemMonotonic`].
pub trait MonotonicClock {
    /// The current monotonic reading.
    fn now(&self) -> Instant;
}

/// `std::time::Instant::now()`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemMonotonic;

impl MonotonicClock for SystemMonotonic {
    fn now(&self) -> Instant {
        todo!()
    }
}

/// Elapsed-time meter. See `README.md` Part 7 for the state rules.
///
/// Fields are private; add whatever state the implementation needs.
#[derive(Debug)]
pub struct Stopwatch<C: MonotonicClock = SystemMonotonic> {
    _clock: C,
}

impl Stopwatch<SystemMonotonic> {
    /// A stopwatch on the system monotonic clock.
    pub fn new() -> Self {
        todo!()
    }
}

impl<C: MonotonicClock> Stopwatch<C> {
    /// A stopwatch that reads `clock`.
    pub fn with_clock(_clock: C) -> Self {
        todo!()
    }

    /// Start timing.
    ///
    /// # Errors
    ///
    /// [`TimeError::InvalidState`] if already running.
    pub fn start(&mut self) -> Result<(), TimeError> {
        todo!()
    }

    /// Stop timing and return the elapsed time since `start`.
    ///
    /// # Errors
    ///
    /// [`TimeError::InvalidState`] if not running.
    pub fn stop(&mut self) -> Result<Duration, TimeError> {
        todo!()
    }

    /// Live reading while running; the last stopped value after `stop`;
    /// [`Duration::ZERO`] before the first `start`.
    pub fn elapsed(&self) -> Duration {
        todo!()
    }
}
