//! Part 7: retry with capped exponential backoff, a circuit breaker, and a token-bucket rate limiter.
//! Time-dependent behaviour is tested through an injected `Clock` where the state machine matters,
//! and through paused tokio time where the sleeps themselves matter.

mod support;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use ex15_async_tokio::error::ConfigError;
use ex15_async_tokio::resilience::{
    CallError, CircuitBreaker, CircuitState, Clock, RetryPolicy, SystemClock, TokenBucket, retry,
};
use support::ManualClock;
use tokio::time::Instant;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

// ---------- RetryPolicy and retry ----------

#[tokio::test(start_paused = true)]
async fn part7_policy_rejects_zero_attempts() {
    assert!(matches!(
        RetryPolicy::new(0, ms(100), ms(500)),
        Err(ConfigError::ZeroAttempts)
    ));
}

#[tokio::test(start_paused = true)]
async fn part7_delay_doubles_then_caps_without_overflow() {
    let policy = RetryPolicy::new(10, ms(100), ms(500)).expect("valid policy");

    assert_eq!(policy.delay_before_retry(0), Duration::ZERO, "no delay before the first attempt");
    let delays: Vec<Duration> = (1..=5).map(|n| policy.delay_before_retry(n)).collect();
    assert_eq!(delays, vec![ms(100), ms(200), ms(400), ms(500), ms(500)]);

    assert_eq!(
        policy.delay_before_retry(u32::MAX),
        ms(500),
        "huge retry counts must saturate at max, not overflow"
    );
}

#[tokio::test(start_paused = true)]
async fn part7_retry_succeeds_after_transient_failures_with_backoff() {
    let policy = RetryPolicy::new(3, ms(100), ms(500)).expect("valid policy");
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();

    let started = Instant::now();
    let out = retry(
        &policy,
        move || {
            let n = counter.fetch_add(1, Ordering::SeqCst) + 1;
            async move {
                if n < 3 {
                    Err(format!("attempt {n} failed"))
                } else {
                    Ok(n)
                }
            }
        },
        |_: &String| true,
    )
    .await;

    assert_eq!(out, Ok(3));
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(
        started.elapsed(),
        ms(300),
        "sleeps of 100ms then 200ms separate the three attempts"
    );
}

#[tokio::test(start_paused = true)]
async fn part7_retry_exhaustion_returns_last_error_without_a_final_sleep() {
    let policy = RetryPolicy::new(2, ms(100), ms(500)).expect("valid policy");
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();

    let started = Instant::now();
    let out: Result<u32, String> = retry(
        &policy,
        move || {
            let n = counter.fetch_add(1, Ordering::SeqCst) + 1;
            async move { Err(format!("attempt {n} failed")) }
        },
        |_: &String| true,
    )
    .await;

    assert_eq!(out, Err("attempt 2 failed".to_string()), "the last error is returned");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        started.elapsed(),
        ms(100),
        "there is one sleep between two attempts and none after the last"
    );
}

#[tokio::test(start_paused = true)]
async fn part7_retry_stops_immediately_when_predicate_rejects_the_error() {
    let policy = RetryPolicy::new(5, ms(100), ms(500)).expect("valid policy");
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();

    let started = Instant::now();
    let out: Result<u32, String> = retry(
        &policy,
        move || {
            counter.fetch_add(1, Ordering::SeqCst);
            async { Err("fatal".to_string()) }
        },
        |e: &String| e != "fatal",
    )
    .await;

    assert_eq!(out, Err("fatal".to_string()));
    assert_eq!(calls.load(Ordering::SeqCst), 1, "a non-retryable error is not retried");
    assert_eq!(started.elapsed(), Duration::ZERO);
}

// ---------- CircuitBreaker ----------

#[tokio::test(start_paused = true)]
async fn part7_breaker_rejects_zero_threshold() {
    assert!(matches!(
        CircuitBreaker::new(0, secs(1), SystemClock),
        Err(ConfigError::ZeroFailureThreshold)
    ));
}

#[tokio::test(start_paused = true)]
async fn part7_breaker_opens_after_threshold_and_rejects_without_calling() {
    let breaker = CircuitBreaker::new(2, secs(30), ManualClock::default()).expect("valid breaker");

    for _ in 0..2 {
        let out = breaker
            .call(|| async { Err::<u32, &str>("db down") })
            .await;
        assert_eq!(out, Err(CallError::Inner("db down")), "inner errors are wrapped");
    }
    assert_eq!(breaker.state(), CircuitState::Open);

    let ran = Arc::new(AtomicBool::new(false));
    let flag = ran.clone();
    let out = breaker
        .call(move || async move {
            flag.store(true, Ordering::SeqCst);
            Ok::<u32, &str>(1)
        })
        .await;

    assert_eq!(out, Err(CallError::CircuitOpen));
    assert!(!ran.load(Ordering::SeqCst), "an open circuit must not run the operation");
}

#[tokio::test(start_paused = true)]
async fn part7_breaker_success_resets_the_failure_count() {
    let breaker = CircuitBreaker::new(2, secs(30), ManualClock::default()).expect("valid breaker");

    let _ = breaker.call(|| async { Err::<u32, &str>("e") }).await;
    assert_eq!(breaker.call(|| async { Ok::<u32, &str>(1) }).await, Ok(1));
    let _ = breaker.call(|| async { Err::<u32, &str>("e") }).await;

    assert_eq!(
        breaker.state(),
        CircuitState::Closed,
        "failures must be consecutive; a success in between resets the count"
    );
}

#[tokio::test(start_paused = true)]
async fn part7_breaker_turns_half_open_only_after_the_reset_timeout() {
    let clock = ManualClock::default();
    let breaker = CircuitBreaker::new(1, secs(10), clock.clone()).expect("valid breaker");
    let _ = breaker.call(|| async { Err::<u32, &str>("e") }).await;
    assert_eq!(breaker.state(), CircuitState::Open);

    clock.advance(secs(9));
    assert_eq!(breaker.state(), CircuitState::Open, "one second early is still open");

    clock.advance(secs(1));
    assert_eq!(breaker.state(), CircuitState::HalfOpen);
}

#[tokio::test(start_paused = true)]
async fn part7_breaker_half_open_success_closes_the_circuit() {
    let clock = ManualClock::default();
    let breaker = CircuitBreaker::new(1, secs(10), clock.clone()).expect("valid breaker");
    let _ = breaker.call(|| async { Err::<u32, &str>("e") }).await;
    clock.advance(secs(10));

    assert_eq!(breaker.call(|| async { Ok::<u32, &str>(7) }).await, Ok(7));
    assert_eq!(breaker.state(), CircuitState::Closed);
}

#[tokio::test(start_paused = true)]
async fn part7_breaker_half_open_failure_reopens_and_restarts_the_timer() {
    let clock = ManualClock::default();
    let breaker = CircuitBreaker::new(1, secs(10), clock.clone()).expect("valid breaker");
    let _ = breaker.call(|| async { Err::<u32, &str>("e") }).await;
    clock.advance(secs(10));

    let out = breaker.call(|| async { Err::<u32, &str>("still down") }).await;
    assert_eq!(out, Err(CallError::Inner("still down")));
    assert_eq!(breaker.state(), CircuitState::Open);

    clock.advance(secs(5));
    assert_eq!(
        breaker.call(|| async { Ok::<u32, &str>(1) }).await,
        Err(CallError::CircuitOpen),
        "the reset timer restarts from the failed trial"
    );
}

#[tokio::test(start_paused = true)]
async fn part7_breaker_admits_one_trial_at_a_time_in_half_open() {
    let clock = ManualClock::default();
    let breaker = Arc::new(CircuitBreaker::new(1, secs(10), clock.clone()).expect("valid breaker"));
    let _ = breaker.call(|| async { Err::<u32, &str>("e") }).await;
    clock.advance(secs(10));

    let trial = {
        let breaker = breaker.clone();
        tokio::spawn(async move {
            breaker
                .call(|| async {
                    tokio::time::sleep(secs(60)).await;
                    Ok::<u32, &str>(1)
                })
                .await
        })
    };
    tokio::time::sleep(ms(1)).await;

    let second = breaker.call(|| async { Ok::<u32, &str>(2) }).await;
    assert_eq!(
        second,
        Err(CallError::CircuitOpen),
        "while a trial is in flight, other calls are rejected"
    );

    trial.abort();
    let _ = trial.await;
}

#[tokio::test(start_paused = true)]
async fn part7_breaker_cancelled_trial_releases_the_probe_slot() {
    let clock = ManualClock::default();
    let breaker = CircuitBreaker::new(1, secs(10), clock.clone()).expect("valid breaker");
    let _ = breaker.call(|| async { Err::<u32, &str>("e") }).await;
    clock.advance(secs(10));

    let abandoned = tokio::time::timeout(
        ms(50),
        breaker.call(|| async {
            tokio::time::sleep(secs(60)).await;
            Ok::<u32, &str>(1)
        }),
    )
    .await;
    assert!(abandoned.is_err(), "the trial was abandoned by its caller");
    assert_eq!(
        breaker.state(),
        CircuitState::HalfOpen,
        "an abandoned trial is neither a success nor a failure"
    );

    assert_eq!(breaker.call(|| async { Ok::<u32, &str>(3) }).await, Ok(3));
    assert_eq!(breaker.state(), CircuitState::Closed);
}

// ---------- TokenBucket ----------

#[tokio::test(start_paused = true)]
async fn part7_bucket_rejects_invalid_parameters() {
    assert!(matches!(
        TokenBucket::new(0.0, 5.0, SystemClock),
        Err(ConfigError::InvalidRate)
    ));
    assert!(matches!(
        TokenBucket::new(1.0, f64::NAN, SystemClock),
        Err(ConfigError::InvalidCapacity)
    ));

    let bucket = TokenBucket::new(1.0, 5.0, SystemClock).expect("valid bucket");
    assert!(matches!(bucket.try_acquire(0.0), Err(ConfigError::InvalidTokens)));
    assert!(
        matches!(bucket.try_acquire(6.0), Err(ConfigError::InvalidTokens)),
        "a request larger than capacity can never be satisfied"
    );
}

#[tokio::test(start_paused = true)]
async fn part7_bucket_starts_full_and_refuses_without_going_negative() {
    let bucket = TokenBucket::new(1.0, 3.0, ManualClock::default()).expect("valid bucket");
    assert_eq!(bucket.available(), 3.0);

    assert_eq!(bucket.try_acquire(2.0), Ok(true));
    assert_eq!(
        bucket.try_acquire(2.0),
        Ok(false),
        "one token left; the refused request leaves the balance alone"
    );
    assert_eq!(bucket.available(), 1.0);
}

#[tokio::test(start_paused = true)]
async fn part7_bucket_refills_with_time_and_caps_at_capacity() {
    let clock = ManualClock::default();
    let bucket = TokenBucket::new(2.0, 4.0, clock.clone()).expect("valid bucket");
    assert_eq!(bucket.try_acquire(4.0), Ok(true));
    assert_eq!(bucket.available(), 0.0);

    clock.advance(secs(1));
    assert_eq!(bucket.available(), 2.0, "two tokens per second");

    clock.advance(secs(10));
    assert_eq!(bucket.available(), 4.0, "refill never exceeds capacity");
}

#[tokio::test(start_paused = true)]
async fn part7_bucket_acquire_waits_for_exactly_the_deficit() {
    let bucket = TokenBucket::new(2.0, 1.0, SystemClock).expect("valid bucket");

    let started = Instant::now();
    bucket.acquire(1.0).await.expect("the bucket starts full");
    assert!(started.elapsed() < ms(1), "a full bucket does not wait");

    bucket
        .acquire(1.0)
        .await
        .expect("waits for the refill, then succeeds");
    assert_eq!(
        started.elapsed(),
        ms(500),
        "one token at two tokens per second is a 500ms wait"
    );
}

#[tokio::test(start_paused = true)]
async fn part7_system_clock_follows_tokio_time() {
    let clock = SystemClock;
    let start = clock.now();
    tokio::time::sleep(secs(3)).await;
    assert_eq!(clock.now() - start, secs(3), "SystemClock must read tokio's (paused) clock");
}
