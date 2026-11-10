//! Part 3: higher-order wrappers. `retry` and `timed` take their time source
//! and sleep hook as injected values.

use crate::Clock;
use std::time::Duration;
use thiserror::Error;

/// Errors that a retry loop may recover from.
pub trait Retryable {
    /// `true` if another attempt could plausibly succeed.
    fn is_retryable(&self) -> bool;
}

/// Invalid retry policy.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PolicyError {
    /// `attempts` must count the first call, so it must be at least 1.
    #[error("retry policy needs at least one attempt")]
    ZeroAttempts,
}

/// How many times to call an operation and how long to wait between failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    attempts: u32,
    base_backoff: Duration,
}

impl RetryPolicy {
    /// Creates a policy. `attempts` includes the first call.
    pub fn new(attempts: u32, base_backoff: Duration) -> Result<Self, PolicyError> {
        if attempts == 0 {
            return Err(PolicyError::ZeroAttempts);
        }
        Ok(Self {
            attempts,
            base_backoff,
        })
    }

    /// Total number of calls allowed, including the first.
    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    /// Delay after the 1-based `failed_attempt` fails: `base * 2^(failed_attempt - 1)`.
    ///
    /// Saturates at `Duration::MAX` rather than overflowing.
    pub fn backoff_after(&self, failed_attempt: u32) -> Duration {
        let mut delay = self.base_backoff;
        if delay.is_zero() {
            return delay;
        }
        // A non-zero delay overflows after roughly 100 doublings, so this loop is bounded.
        for _ in 1..failed_attempt {
            match delay.checked_mul(2) {
                Some(next) => delay = next,
                None => return Duration::MAX,
            }
        }
        delay
    }
}

/// Calls `op` with the 1-based attempt number until it succeeds, the error is
/// not retryable, or the attempts run out.
///
/// Between failed attempts, calls `sleep(policy.backoff_after(attempt))`. This
/// happens even when the delay is zero. It never happens after a success or
/// after the final failure. The last error is returned unchanged.
pub fn retry<T, E, F, S>(policy: &RetryPolicy, mut op: F, mut sleep: S) -> Result<T, E>
where
    E: Retryable,
    F: FnMut(u32) -> Result<T, E>,
    S: FnMut(Duration),
{
    let mut attempt = 1;
    loop {
        match op(attempt) {
            Ok(value) => return Ok(value),
            Err(err) if err.is_retryable() && attempt < policy.attempts() => {
                sleep(policy.backoff_after(attempt));
                attempt += 1;
            }
            Err(err) => return Err(err),
        }
    }
}

/// Reports the elapsed clock time to the sink when dropped, so panics are measured too.
struct Report<'a, C: Clock, S: FnMut(Duration)> {
    clock: &'a C,
    sink: &'a mut S,
    started: Duration,
}

impl<C: Clock, S: FnMut(Duration)> Drop for Report<'_, C, S> {
    fn drop(&mut self) {
        (self.sink)(self.clock.now().saturating_sub(self.started));
    }
}

/// Wraps `f` so that every call reports its elapsed clock time to `sink`.
///
/// The report happens even if `f` panics, because the measurement is taken
/// by a `Drop` guard. The result (or the panic) passes through unchanged.
pub fn timed<A, R, C, S, F>(clock: C, mut sink: S, mut f: F) -> impl FnMut(A) -> R
where
    C: Clock,
    S: FnMut(Duration),
    F: FnMut(A) -> R,
{
    move |arg: A| -> R {
        let _report = Report {
            clock: &clock,
            sink: &mut sink,
            started: clock.now(),
        };
        f(arg)
    }
}
