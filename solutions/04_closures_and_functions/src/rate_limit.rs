//! Part 6: a sliding-window rate limiter with interior mutability.
//!
//! The limiter is used through `&self`, so one instance can be shared across
//! threads (for example in an `Arc`) and still enforce a single budget.

use crate::Clock;
use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};
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
pub struct RateLimiter<C> {
    max_calls: usize,
    period: Duration,
    clock: C,
    window: Mutex<VecDeque<Duration>>,
}

impl<C: Clock> RateLimiter<C> {
    /// Validates the configuration and creates an empty limiter.
    pub fn new(max_calls: usize, period: Duration, clock: C) -> Result<Self, ConfigError> {
        if max_calls == 0 {
            return Err(ConfigError::ZeroCalls);
        }
        if period.is_zero() {
            return Err(ConfigError::ZeroPeriod);
        }
        Ok(Self {
            max_calls,
            period,
            clock,
            window: Mutex::new(VecDeque::new()),
        })
    }

    /// Records a call if the budget allows it, or reports when to retry.
    pub fn try_acquire(&self) -> Result<(), RateLimited> {
        let now = self.clock.now();
        // The window holds plain timestamps, so a panic elsewhere cannot leave it
        // inconsistent. Recover from poisoning rather than refusing all callers.
        let mut window = self.window.lock().unwrap_or_else(PoisonError::into_inner);

        while let Some(&oldest) = window.front() {
            if now.saturating_sub(oldest) < self.period {
                break;
            }
            window.pop_front();
        }

        if window.len() < self.max_calls {
            window.push_back(now);
            return Ok(());
        }
        let oldest = *window.front().expect("a full window is never empty");
        Err(RateLimited {
            retry_after: (oldest + self.period).saturating_sub(now),
        })
    }
}

/// Gates `f` through `limiter`. Each call to the returned closure first takes a
/// permit, then runs `f` only if the permit was granted.
pub fn limited<'a, C, A, R, F>(limiter: &'a RateLimiter<C>, f: F) -> impl Fn(A) -> Result<R, RateLimited> + 'a
where
    C: Clock,
    F: Fn(A) -> R + 'a,
{
    move |arg: A| -> Result<R, RateLimited> {
        limiter.try_acquire()?;
        Ok(f(arg))
    }
}
