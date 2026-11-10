//! Part 7: monotonic stopwatch with an injected clock.

use std::time::{Duration, Instant};

use crate::TimeError;

/// Source of monotonic readings. Tests inject a fake; production uses
/// [`SystemMonotonic`].
pub trait MonotonicClock {
    /// The current monotonic reading.
    fn now(&self) -> Instant;
}

/// The operating system's monotonic clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemMonotonic;

impl MonotonicClock for SystemMonotonic {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[derive(Debug, Clone, Copy)]
enum State {
    Idle,
    Running(Instant),
    Stopped(Duration),
}

/// Measures elapsed time with a monotonic clock.
///
/// Fields are private. Each `start` begins a fresh measurement window.
#[derive(Debug)]
pub struct Stopwatch<C: MonotonicClock = SystemMonotonic> {
    clock: C,
    state: State,
}

impl Stopwatch<SystemMonotonic> {
    /// A stopwatch on the system monotonic clock.
    pub fn new() -> Self {
        Self::with_clock(SystemMonotonic)
    }
}

impl Default for Stopwatch<SystemMonotonic> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: MonotonicClock> Stopwatch<C> {
    /// A stopwatch that reads `clock`.
    pub fn with_clock(clock: C) -> Self {
        Self { clock, state: State::Idle }
    }

    /// Begin a measurement window.
    ///
    /// # Errors
    ///
    /// [`TimeError::InvalidState`] if already running.
    pub fn start(&mut self) -> Result<(), TimeError> {
        match self.state {
            State::Running(_) => Err(TimeError::InvalidState("stopwatch is already running")),
            State::Idle | State::Stopped(_) => {
                self.state = State::Running(self.clock.now());
                Ok(())
            }
        }
    }

    /// End the window and return its duration. Further reads are frozen.
    ///
    /// # Errors
    ///
    /// [`TimeError::InvalidState`] if not running.
    pub fn stop(&mut self) -> Result<Duration, TimeError> {
        match self.state {
            State::Running(started) => {
                let total = self.clock.now().duration_since(started);
                self.state = State::Stopped(total);
                Ok(total)
            }
            State::Idle | State::Stopped(_) => {
                Err(TimeError::InvalidState("stopwatch is not running"))
            }
        }
    }

    /// Time measured so far: live while running, frozen after `stop`.
    /// [`Duration::ZERO`] before the first `start`.
    pub fn elapsed(&self) -> Duration {
        match self.state {
            State::Idle => Duration::ZERO,
            State::Running(started) => self.clock.now().duration_since(started),
            State::Stopped(total) => total,
        }
    }
}
