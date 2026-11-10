//! Part 7 — resilience primitives: retry with capped exponential backoff, a circuit breaker and
//! a token bucket. Time comes from an injected [`Clock`] so the state machines are testable.

use std::future::Future;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use thiserror::Error;
use tokio::time::Instant;

use crate::error::ConfigError;

/// A source of monotonic time. Implement it to control time in tests.
pub trait Clock: Send + Sync {
    /// The current instant on this clock.
    fn now(&self) -> Instant;
}

/// The real clock. Under `start_paused` it follows tokio's paused clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Retry configuration: attempts and capped exponential backoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    attempts: u32,
    base: Duration,
    max: Duration,
}

impl RetryPolicy {
    /// `attempts == 0` is `ConfigError::ZeroAttempts`.
    pub fn new(attempts: u32, base: Duration, max: Duration) -> Result<Self, ConfigError> {
        if attempts == 0 {
            return Err(ConfigError::ZeroAttempts);
        }
        Ok(Self {
            attempts,
            base,
            max,
        })
    }

    /// Delay before retry `retry` (1-based): `base * 2^(retry-1)`, capped at `max`.
    /// `retry == 0` has no delay. Overflow saturates at `max`.
    pub fn delay_before_retry(&self, retry: u32) -> Duration {
        if retry == 0 {
            return Duration::ZERO;
        }
        match 2u32
            .checked_pow(retry - 1)
            .and_then(|factor| self.base.checked_mul(factor))
        {
            Some(delay) => delay.min(self.max),
            None => self.max,
        }
    }
}

/// Runs `operation` up to `policy` attempts, sleeping between attempts.
///
/// Errors for which `retry_if` returns `false` are returned immediately. When attempts run out,
/// the last error is returned unchanged. No sleep follows the final attempt.
pub async fn retry<T, E, F, Fut, P>(
    policy: &RetryPolicy,
    mut operation: F,
    retry_if: P,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    P: Fn(&E) -> bool,
{
    let mut attempt = 1;
    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(error) => {
                if attempt >= policy.attempts || !retry_if(&error) {
                    return Err(error);
                }
                tokio::time::sleep(policy.delay_before_retry(attempt)).await;
                attempt += 1;
            }
        }
    }
}

/// Observable state of a [`CircuitBreaker`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Calls pass through; consecutive failures are counted.
    Closed,
    /// Calls are rejected without running.
    Open,
    /// One trial call at a time is admitted to probe the upstream.
    HalfOpen,
}

/// Why a guarded call did not produce a value.
#[derive(Debug, PartialEq, Eq, Error)]
pub enum CallError<E> {
    /// The breaker rejected the call.
    #[error("circuit is open")]
    CircuitOpen,
    /// The operation ran and failed.
    #[error("guarded call failed")]
    Inner(E),
}

/// Internal breaker state. `Copy` so transitions can be written as a single match.
#[derive(Debug, Clone, Copy)]
enum BreakerState {
    Closed { failures: u32 },
    Open { opened_at: Instant },
    HalfOpen { trial_in_flight: bool },
}

/// Stops calling a failing upstream for a while.
///
/// `Closed` counts consecutive failures. At `failure_threshold` it becomes `Open` and records the
/// clock time. Once `reset_timeout` has elapsed the state reads `HalfOpen`, and exactly one trial
/// call is admitted at a time.
pub struct CircuitBreaker<C: Clock = SystemClock> {
    failure_threshold: u32,
    reset_timeout: Duration,
    clock: C,
    state: Mutex<BreakerState>,
}

/// Releases the half-open trial slot if the trial future is dropped before it settles.
struct Trial<'a, C: Clock> {
    breaker: &'a CircuitBreaker<C>,
    is_trial: bool,
}

impl<C: Clock> Drop for Trial<'_, C> {
    fn drop(&mut self) {
        if self.is_trial {
            self.breaker.release_trial();
        }
    }
}

impl<C: Clock> CircuitBreaker<C> {
    /// `failure_threshold == 0` is `ConfigError::ZeroFailureThreshold`.
    pub fn new(failure_threshold: u32, reset_timeout: Duration, clock: C) -> Result<Self, ConfigError> {
        if failure_threshold == 0 {
            return Err(ConfigError::ZeroFailureThreshold);
        }
        Ok(Self {
            failure_threshold,
            reset_timeout,
            clock,
            state: Mutex::new(BreakerState::Closed { failures: 0 }),
        })
    }

    /// Current state. Reading it applies the `Open` to `HalfOpen` transition once the reset
    /// timeout has elapsed.
    pub fn state(&self) -> CircuitState {
        let mut state = self.lock();
        self.refresh(&mut state);
        match *state {
            BreakerState::Closed { .. } => CircuitState::Closed,
            BreakerState::Open { .. } => CircuitState::Open,
            BreakerState::HalfOpen { .. } => CircuitState::HalfOpen,
        }
    }

    /// Runs `operation` if the breaker admits it, and records the outcome.
    pub async fn call<T, E, F, Fut>(&self, operation: F) -> Result<T, CallError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        let Some(is_trial) = self.admit() else {
            return Err(CallError::CircuitOpen);
        };
        let _trial = Trial {
            breaker: self,
            is_trial,
        };
        let result = operation().await;
        self.settle(is_trial, result.is_ok());
        result.map_err(CallError::Inner)
    }

    fn lock(&self) -> MutexGuard<'_, BreakerState> {
        self.state.lock().expect("breaker state lock poisoned")
    }

    /// Applies the timeout-driven `Open` to `HalfOpen` transition.
    fn refresh(&self, state: &mut BreakerState) {
        if let BreakerState::Open { opened_at } = *state
            && self.clock.now().duration_since(opened_at) >= self.reset_timeout
        {
            *state = BreakerState::HalfOpen {
                trial_in_flight: false,
            };
        }
    }

    /// Returns `Some(is_trial)` when the call may run, `None` when it is rejected.
    fn admit(&self) -> Option<bool> {
        let mut state = self.lock();
        self.refresh(&mut state);
        match &mut *state {
            BreakerState::Closed { .. } => Some(false),
            BreakerState::Open { .. } => None,
            BreakerState::HalfOpen { trial_in_flight } => {
                if *trial_in_flight {
                    None
                } else {
                    *trial_in_flight = true;
                    Some(true)
                }
            }
        }
    }

    /// Records an outcome. Only the transitions that match the call's admission apply; a stale
    /// result from before the circuit opened is ignored.
    fn settle(&self, is_trial: bool, succeeded: bool) {
        let now = self.clock.now();
        let mut state = self.lock();
        *state = match (*state, is_trial, succeeded) {
            (BreakerState::Closed { .. }, false, true) => BreakerState::Closed { failures: 0 },
            (BreakerState::Closed { failures }, false, false) => {
                if failures + 1 >= self.failure_threshold {
                    BreakerState::Open { opened_at: now }
                } else {
                    BreakerState::Closed {
                        failures: failures + 1,
                    }
                }
            }
            (BreakerState::HalfOpen { .. }, true, true) => BreakerState::Closed { failures: 0 },
            (BreakerState::HalfOpen { .. }, true, false) => BreakerState::Open { opened_at: now },
            (other, _, _) => other,
        };
    }

    fn release_trial(&self) {
        let mut state = self.lock();
        if let BreakerState::HalfOpen { trial_in_flight } = &mut *state {
            *trial_in_flight = false;
        }
    }
}

/// Internal bucket state: the balance and the instant it was last brought up to date.
#[derive(Debug, Clone, Copy)]
struct BucketState {
    tokens: f64,
    updated: Instant,
}

/// Token bucket: `capacity` tokens, refilled continuously at `rate` tokens per second.
///
/// Refill is computed from the clock; no timers run in the background. `acquire` sleeps on
/// tokio's timer.
pub struct TokenBucket<C: Clock = SystemClock> {
    rate: f64,
    capacity: f64,
    clock: C,
    state: Mutex<BucketState>,
}

impl<C: Clock> TokenBucket<C> {
    /// Creates a full bucket. `rate` must be finite and positive; `capacity` must be finite and
    /// positive.
    pub fn new(rate: f64, capacity: f64, clock: C) -> Result<Self, ConfigError> {
        if !rate.is_finite() || rate <= 0.0 {
            return Err(ConfigError::InvalidRate);
        }
        if !capacity.is_finite() || capacity <= 0.0 {
            return Err(ConfigError::InvalidCapacity);
        }
        let updated = clock.now();
        Ok(Self {
            rate,
            capacity,
            clock,
            state: Mutex::new(BucketState {
                tokens: capacity,
                updated,
            }),
        })
    }

    /// Current balance, never negative.
    pub fn available(&self) -> f64 {
        let mut state = self.lock();
        self.refill(&mut state);
        state.tokens.max(0.0)
    }

    /// Takes `tokens` if available right now. Returns `Ok(false)` and leaves the balance alone
    /// otherwise.
    pub fn try_acquire(&self, tokens: f64) -> Result<bool, ConfigError> {
        self.check_tokens(tokens)?;
        let mut state = self.lock();
        self.refill(&mut state);
        if state.tokens >= tokens {
            state.tokens -= tokens;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Reserves `tokens` now and sleeps until the balance would have covered them.
    ///
    /// The reservation is made under the lock, so concurrent callers queue in order without
    /// holding the lock across the sleep.
    pub async fn acquire(&self, tokens: f64) -> Result<(), ConfigError> {
        self.check_tokens(tokens)?;
        let deficit = {
            let mut state = self.lock();
            self.refill(&mut state);
            state.tokens -= tokens;
            (-state.tokens).max(0.0)
        };
        if deficit > 0.0 {
            tokio::time::sleep(Duration::from_secs_f64(deficit / self.rate)).await;
        }
        Ok(())
    }

    fn lock(&self) -> MutexGuard<'_, BucketState> {
        self.state.lock().expect("bucket state lock poisoned")
    }

    fn check_tokens(&self, tokens: f64) -> Result<(), ConfigError> {
        if tokens.is_finite() && tokens > 0.0 && tokens <= self.capacity {
            Ok(())
        } else {
            Err(ConfigError::InvalidTokens)
        }
    }

    fn refill(&self, state: &mut BucketState) {
        let now = self.clock.now();
        let elapsed = now.duration_since(state.updated).as_secs_f64();
        state.tokens = (state.tokens + elapsed * self.rate).min(self.capacity);
        state.updated = now;
    }
}
