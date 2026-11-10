//! Part 7: monotonic stopwatch with an injected clock.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use ex17_time_jiff::TimeError;
use ex17_time_jiff::stopwatch::{MonotonicClock, Stopwatch};

/// A fake monotonic clock whose reading is advanced explicitly by the test.
#[derive(Clone)]
struct FakeMono {
    base: Instant,
    offset: Rc<Cell<Duration>>,
}

impl FakeMono {
    fn new() -> Self {
        Self { base: Instant::now(), offset: Rc::new(Cell::new(Duration::ZERO)) }
    }

    fn advance(&self, by: Duration) {
        self.offset.set(self.offset.get() + by);
    }
}

impl MonotonicClock for FakeMono {
    fn now(&self) -> Instant {
        self.base + self.offset.get()
    }
}

#[test]
fn part7_elapsed_is_zero_before_first_start() {
    let clock = FakeMono::new();
    let sw = Stopwatch::with_clock(clock.clone());
    clock.advance(Duration::from_secs(5));
    assert_eq!(sw.elapsed(), Duration::ZERO, "a stopwatch that never started reads zero");
}

#[test]
fn part7_start_twice_and_stop_without_start_are_invalid_state() {
    let clock = FakeMono::new();
    let mut sw = Stopwatch::with_clock(clock);
    assert!(matches!(sw.stop(), Err(TimeError::InvalidState(_))), "stop before start");
    sw.start().expect("first start succeeds");
    assert!(matches!(sw.start(), Err(TimeError::InvalidState(_))), "second start while running");
}

#[test]
fn part7_live_elapsed_then_frozen_on_stop() {
    let clock = FakeMono::new();
    let mut sw = Stopwatch::with_clock(clock.clone());
    sw.start().unwrap();
    clock.advance(Duration::from_millis(250));
    assert_eq!(sw.elapsed(), Duration::from_millis(250), "live reading while running");
    clock.advance(Duration::from_millis(1250));
    assert_eq!(sw.stop().unwrap(), Duration::from_millis(1500), "stop returns the total");
    clock.advance(Duration::from_secs(10));
    assert_eq!(sw.elapsed(), Duration::from_millis(1500), "elapsed is frozen after stop");
}

#[test]
fn part7_restart_measures_a_fresh_window() {
    let clock = FakeMono::new();
    let mut sw = Stopwatch::with_clock(clock.clone());
    sw.start().unwrap();
    clock.advance(Duration::from_secs(1));
    assert_eq!(sw.stop().unwrap(), Duration::from_secs(1));
    sw.start().expect("restart after stop is allowed");
    clock.advance(Duration::from_secs(2));
    assert_eq!(sw.stop().unwrap(), Duration::from_secs(2), "the restart window excludes the first run");
}

#[test]
fn part7_default_stopwatch_uses_the_system_monotonic_clock() {
    let mut sw = Stopwatch::new();
    sw.start().unwrap();
    let d = sw.stop().unwrap();
    assert!(d < Duration::from_secs(60), "system stopwatch measured {d:?}, expected a short span");
}
