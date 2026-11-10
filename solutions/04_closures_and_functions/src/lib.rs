//! Function toolkit for a platform team's Rust services: closures, callbacks,
//! decorators, memoisation, middleware, rate limiting and a command registry.

pub mod callbacks;
pub mod closures;
pub mod commands;
pub mod decorators;
pub mod memo;
pub mod middleware;
pub mod rate_limit;

use std::time::{Duration, Instant};

/// Monotonic time source, expressed as time elapsed since an arbitrary origin.
///
/// Time-dependent types take a `Clock` so tests can drive time by hand.
/// `Send + Sync` lets one clock be shared by many threads.
pub trait Clock: Send + Sync {
    /// Time elapsed since this clock's origin. Never decreases.
    fn now(&self) -> Duration;
}

/// The real monotonic clock. Its origin is the moment of construction.
#[derive(Debug, Clone, Copy)]
pub struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    /// Creates a clock whose origin is now.
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
}
