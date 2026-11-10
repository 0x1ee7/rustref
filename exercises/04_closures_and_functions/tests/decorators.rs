//! Part 3: higher-order retry and timing wrappers with injected clock and sleep.

mod common;

use common::ManualClock;
use ex04_closures_and_functions::decorators::*;
use ex04_closures_and_functions::{Clock, SystemClock};
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
enum OpError {
    Transient(u32),
    Fatal,
}

impl Retryable for OpError {
    fn is_retryable(&self) -> bool {
        matches!(self, OpError::Transient(_))
    }
}

#[test]
fn part3_policy_rejects_zero_attempts() {
    assert_eq!(RetryPolicy::new(0, Duration::ZERO).err(), Some(PolicyError::ZeroAttempts));
}

#[test]
fn part3_backoff_doubles_after_each_failed_attempt() {
    let policy = RetryPolicy::new(5, Duration::from_millis(100)).unwrap();
    assert_eq!(policy.attempts(), 5);
    assert_eq!(policy.backoff_after(1), Duration::from_millis(100));
    assert_eq!(policy.backoff_after(2), Duration::from_millis(200));
    assert_eq!(policy.backoff_after(3), Duration::from_millis(400));
}

#[test]
fn part3_backoff_saturates_instead_of_overflowing() {
    let policy = RetryPolicy::new(100, Duration::from_millis(100)).unwrap();
    assert_eq!(policy.backoff_after(200), Duration::MAX, "2^199 saturates rather than panicking");
}

#[test]
fn part3_retry_success_on_first_attempt_never_sleeps() {
    let policy = RetryPolicy::new(3, Duration::from_millis(10)).unwrap();
    let mut sleeps = Vec::new();
    let result: Result<&str, OpError> = retry(&policy, |_| Ok("done"), |d| sleeps.push(d));
    assert_eq!(result, Ok("done"));
    assert!(sleeps.is_empty(), "no failures, no sleeps");
}

#[test]
fn part3_retry_passes_one_based_attempt_numbers() {
    let policy = RetryPolicy::new(5, Duration::ZERO).unwrap();
    let mut seen = Vec::new();
    let result = retry(
        &policy,
        |attempt| {
            seen.push(attempt);
            if attempt < 3 {
                Err(OpError::Transient(attempt))
            } else {
                Ok(attempt)
            }
        },
        |_| {},
    );
    assert_eq!(result, Ok(3));
    assert_eq!(seen, vec![1, 2, 3]);
}

#[test]
fn part3_retry_sleeps_between_attempts_but_not_after_the_last() {
    let policy = RetryPolicy::new(3, Duration::from_millis(10)).unwrap();
    let mut calls = 0;
    let mut sleeps = Vec::new();
    let result: Result<(), OpError> = retry(
        &policy,
        |_| {
            calls += 1;
            Err(OpError::Transient(calls))
        },
        |d| sleeps.push(d),
    );
    assert_eq!(result, Err(OpError::Transient(3)), "the last error propagates unchanged");
    assert_eq!(calls, 3);
    assert_eq!(
        sleeps,
        vec![Duration::from_millis(10), Duration::from_millis(20)],
        "sleep only between attempts, doubling each time"
    );
}

#[test]
fn part3_retry_gives_up_immediately_on_non_retryable_error() {
    let policy = RetryPolicy::new(5, Duration::from_millis(10)).unwrap();
    let mut calls = 0;
    let mut sleeps = Vec::new();
    let result: Result<(), OpError> = retry(
        &policy,
        |_| {
            calls += 1;
            Err(OpError::Fatal)
        },
        |d| sleeps.push(d),
    );
    assert_eq!(result, Err(OpError::Fatal));
    assert_eq!(calls, 1, "fatal errors are not retried");
    assert!(sleeps.is_empty());
}

#[test]
fn part3_timed_reports_elapsed_clock_time_per_call() {
    let clock = ManualClock::default();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink_seen = Arc::clone(&seen);
    let inner = clock.clone();
    let mut doubled = timed(
        clock.clone(),
        move |d| sink_seen.lock().unwrap().push(d),
        move |x: u32| {
            inner.advance(Duration::from_millis(25));
            x * 2
        },
    );
    assert_eq!(doubled(3), 6, "the wrapper returns the wrapped result");
    assert_eq!(doubled(4), 8);
    assert_eq!(*seen.lock().unwrap(), vec![Duration::from_millis(25); 2]);
}

#[test]
fn part3_timed_still_reports_when_the_call_panics() {
    let clock = ManualClock::default();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink_seen = Arc::clone(&seen);
    let inner = clock.clone();
    let mut risky = timed(
        clock.clone(),
        move |d| sink_seen.lock().unwrap().push(d),
        move |(): ()| {
            inner.advance(Duration::from_millis(5));
            panic!("boom");
        },
    );
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| risky(())));
    assert!(outcome.is_err(), "the panic propagates to the caller");
    assert_eq!(
        *seen.lock().unwrap(),
        vec![Duration::from_millis(5)],
        "the drop-based report ran while unwinding"
    );
}

#[test]
fn part3_retry_outside_timed_records_each_attempt() {
    let clock = ManualClock::default();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink_seen = Arc::clone(&seen);
    let inner = clock.clone();
    let timed_op = timed(
        clock.clone(),
        move |d| sink_seen.lock().unwrap().push(d),
        move |attempt: u32| {
            inner.advance(Duration::from_millis(10 * u64::from(attempt)));
            if attempt < 3 {
                Err(OpError::Transient(attempt))
            } else {
                Ok(attempt)
            }
        },
    );
    let policy = RetryPolicy::new(3, Duration::from_millis(1)).unwrap();
    let mut sleeps = Vec::new();
    let result = retry(&policy, timed_op, |d| sleeps.push(d));
    assert_eq!(result, Ok(3));
    assert_eq!(
        *seen.lock().unwrap(),
        vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
        ],
        "one measurement per attempt"
    );
    assert_eq!(sleeps, vec![Duration::from_millis(1), Duration::from_millis(2)]);
}

#[test]
fn part3_timed_outside_retry_includes_backoff_sleeps() {
    let clock = ManualClock::default();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink_seen = Arc::clone(&seen);
    let sleep_clock = clock.clone();
    let policy = RetryPolicy::new(3, Duration::from_millis(10)).unwrap();
    let mut whole_call = timed(
        clock.clone(),
        move |d| sink_seen.lock().unwrap().push(d),
        move |(): ()| {
            let mut calls = 0;
            retry(
                &policy,
                |_| {
                    calls += 1;
                    if calls < 3 {
                        Err(OpError::Transient(calls))
                    } else {
                        Ok(calls)
                    }
                },
                |d| sleep_clock.advance(d),
            )
        },
    );
    assert_eq!(whole_call(()), Ok(3));
    assert_eq!(
        *seen.lock().unwrap(),
        vec![Duration::from_millis(30)],
        "the 10 ms and 20 ms backoffs count toward the call"
    );
}

#[test]
fn part3_system_clock_never_goes_backwards() {
    let clock = SystemClock::new();
    let first = clock.now();
    let second = clock.now();
    assert!(second >= first, "a monotonic clock must not decrease");
}
