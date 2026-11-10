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
#[allow(dead_code)] // stub: remove once the fields are read
pub struct RetryPolicy {
    attempts: u32,
    base_backoff: Duration,
}

impl RetryPolicy {
    /// Creates a policy. `attempts` includes the first call.
    pub fn new(_attempts: u32, _base_backoff: Duration) -> Result<Self, PolicyError> {
        todo!()
    }

    /// Total number of calls allowed, including the first.
    pub fn attempts(&self) -> u32 {
        todo!()
    }

    /// Delay after the 1-based `failed_attempt` fails: `base * 2^(failed_attempt - 1)`.
    ///
    /// Saturates at `Duration::MAX` rather than overflowing.
    pub fn backoff_after(&self, _failed_attempt: u32) -> Duration {
        todo!()
    }
}

/// Calls `op` with the 1-based attempt number until it succeeds, the error is
/// not retryable, or the attempts run out.
///
/// Between failed attempts, calls `sleep(policy.backoff_after(attempt))`. This
/// happens even when the delay is zero. It never happens after a success or
/// after the final failure. The last error is returned unchanged.
pub fn retry<T, E, F, S>(_policy: &RetryPolicy, _op: F, _sleep: S) -> Result<T, E>
where
    E: Retryable,
    F: FnMut(u32) -> Result<T, E>,
    S: FnMut(Duration),
{
    todo!()
}

/// Wraps `f` so that every call reports its elapsed clock time to `sink`.
///
/// The report happens even if `f` panics, because the measurement is taken
/// by a `Drop` guard. The result (or the panic) passes through unchanged.
pub fn timed<A, R, C, S, F>(_clock: C, _sink: S, _f: F) -> impl FnMut(A) -> R
where
    C: Clock,
    S: FnMut(Duration),
    F: FnMut(A) -> R,
{
    |_arg: A| -> R { todo!() }
}
