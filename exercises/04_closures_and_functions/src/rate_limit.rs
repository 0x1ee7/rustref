//! Part 6: a sliding-window rate limiter with interior mutability.
//!
//! The limiter is used through `&self`, so one instance can be shared across
//! threads (for example in an `Arc`) and still enforce a single budget.

use crate::Clock;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;
use thiserror::Error;

/// Invalid limiter configuration.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// At least one call must be allowed per window.
    #[error("max_calls must be at least 1")]
    ZeroCalls,
    /// A zero-length window would allow unlimited calls.
    #[error("period must be greater than zero")]
    ZeroPeriod,
}

/// A call was rejected. `retry_after` is when the budget next opens up.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("rate limit exceeded; retry after {retry_after:?}")]
pub struct RateLimited {
    /// Time until the oldest recorded call leaves the window.
    pub retry_after: Duration,
}

/// Allows at most `max_calls` calls in any window of length `period`.
///
/// A call at time `now` is allowed when fewer than `max_calls` recorded calls
/// have `now - t < period`. Rejected calls are not recorded.
#[allow(dead_code)] // stub: remove once the fields are used
pub struct RateLimiter<C> {
    max_calls: usize,
    period: Duration,
    clock: C,
    window: Mutex<VecDeque<Duration>>,
}

impl<C: Clock> RateLimiter<C> {
    /// Validates the configuration and creates an empty limiter.
    pub fn new(_max_calls: usize, _period: Duration, _clock: C) -> Result<Self, ConfigError> {
        todo!()
    }

    /// Records a call if the budget allows it, or reports when to retry.
    pub fn try_acquire(&self) -> Result<(), RateLimited> {
        todo!()
    }
}

/// Gates `f` through `limiter`. Each call to the returned closure first takes a
/// permit, then runs `f` only if the permit was granted.
pub fn limited<'a, C, A, R, F>(
    _limiter: &'a RateLimiter<C>,
    _f: F,
) -> impl Fn(A) -> Result<R, RateLimited> + 'a
where
    C: Clock,
    F: Fn(A) -> R + 'a,
{
    |_arg: A| -> Result<R, RateLimited> { todo!() }
}
