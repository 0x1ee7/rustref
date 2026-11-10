//! Part 6: rate limiting with interior mutability, shared across threads.

mod common;

use common::ManualClock;
use ex04_closures_and_functions::rate_limit::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn make_limiter(max_calls: usize, period_secs: u64) -> (RateLimiter<ManualClock>, ManualClock) {
    let clock = ManualClock::default();
    let limiter = RateLimiter::new(max_calls, Duration::from_secs(period_secs), clock.clone())
        .expect("valid config");
    (limiter, clock)
}

#[test]
fn part6_rejects_zero_calls_and_zero_period() {
    let clock = ManualClock::default();
    assert_eq!(
        RateLimiter::new(0, Duration::from_secs(1), clock.clone()).err(),
        Some(ConfigError::ZeroCalls)
    );
    assert_eq!(RateLimiter::new(1, Duration::ZERO, clock).err(), Some(ConfigError::ZeroPeriod));
}

#[test]
fn part6_allows_max_calls_then_rejects_with_retry_after() {
    let (limiter, _clock) = make_limiter(3, 10);
    for _ in 0..3 {
        assert_eq!(limiter.try_acquire(), Ok(()));
    }
    assert_eq!(
        limiter.try_acquire(),
        Err(RateLimited { retry_after: Duration::from_secs(10) }),
        "the oldest call at t=0 leaves the window at t=10"
    );
}

#[test]
fn part6_retry_after_counts_down_from_the_oldest_call() {
    let (limiter, clock) = make_limiter(2, 10);
    limiter.try_acquire().unwrap(); // t=0
    clock.advance(Duration::from_secs(4));
    limiter.try_acquire().unwrap(); // t=4
    clock.advance(Duration::from_secs(1)); // t=5
    assert_eq!(
        limiter.try_acquire(),
        Err(RateLimited { retry_after: Duration::from_secs(5) }),
        "the t=0 call expires at t=10"
    );
}

#[test]
fn part6_window_slides_as_calls_expire() {
    let (limiter, clock) = make_limiter(2, 10);
    limiter.try_acquire().unwrap();
    limiter.try_acquire().unwrap(); // both at t=0
    clock.advance(Duration::from_secs(9));
    assert!(limiter.try_acquire().is_err(), "both calls are still in the window at t=9");
    clock.advance(Duration::from_secs(1)); // t=10
    assert!(limiter.try_acquire().is_ok(), "both calls expired at t=10");
    assert!(limiter.try_acquire().is_ok());
    assert!(limiter.try_acquire().is_err());
}

#[test]
fn part6_rejected_calls_are_not_recorded() {
    let (limiter, clock) = make_limiter(1, 10);
    limiter.try_acquire().unwrap(); // t=0
    clock.advance(Duration::from_secs(5));
    let expected = Err(RateLimited { retry_after: Duration::from_secs(5) });
    assert_eq!(limiter.try_acquire(), expected);
    assert_eq!(limiter.try_acquire(), expected, "rejections must not extend the window");
    clock.advance(Duration::from_secs(5)); // t=10
    assert!(limiter.try_acquire().is_ok());
}

#[test]
fn part6_limited_gates_a_closure_through_the_limiter() {
    let (limiter, _clock) = make_limiter(2, 1);
    let call = limited(&limiter, |x: u32| x * 10);
    assert_eq!(call(1), Ok(10));
    assert_eq!(call(2), Ok(20));
    assert_eq!(call(3), Err(RateLimited { retry_after: Duration::from_secs(1) }));
    assert!(limiter.try_acquire().is_err(), "the wrapper spent the same budget");
}

#[test]
fn part6_concurrent_callers_share_one_budget() {
    let (limiter, _clock) = make_limiter(50, 60);
    let limiter = Arc::new(limiter);
    let granted = Arc::new(AtomicUsize::new(0));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let limiter = Arc::clone(&limiter);
            let granted = Arc::clone(&granted);
            thread::spawn(move || {
                for _ in 0..100 {
                    if limiter.try_acquire().is_ok() {
                        granted.fetch_add(1, Ordering::SeqCst);
                    }
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().expect("worker panicked");
    }
    assert_eq!(granted.load(Ordering::SeqCst), 50, "exactly max_calls succeed across all threads");
}

#[test]
fn part6_rate_limited_message_includes_retry_after() {
    let (limiter, clock) = make_limiter(1, 2);
    limiter.try_acquire().expect("first call allowed");
    clock.advance(Duration::from_millis(500));
    let err = limiter.try_acquire().expect_err("second call rejected");
    let message = err.to_string();
    assert!(message.contains("1.5s"), "message was {message:?}");
}
