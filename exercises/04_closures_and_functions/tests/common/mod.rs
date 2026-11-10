#![allow(dead_code)]

use ex04_closures_and_functions::Clock;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A clock that only moves when the test tells it to. Clones share one timeline.
#[derive(Clone, Default)]
pub struct ManualClock {
    now: Arc<Mutex<Duration>>,
}

impl ManualClock {
    /// Moves the clock forward by `by`.
    pub fn advance(&self, by: Duration) {
        *self.now.lock().unwrap() += by;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Duration {
        *self.now.lock().unwrap()
    }
}
