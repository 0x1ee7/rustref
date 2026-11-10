//! Part 1: concurrent fan-out with per-call timeouts and all-or-nothing fan-out.

mod support;

use std::error::Error as _;
use std::fmt::{self, Display};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use ex15_async_tokio::error::ConfigError;
use ex15_async_tokio::fanout::{Outcome, SourceResult, aggregate, fetch_all_strict};
use tokio::time::Instant;

type Step = Pin<Box<dyn Future<Output = Result<u32, String>> + Send>>;
type NoSource = fn() -> Step;

/// Scripted behaviour for a fake upstream.
enum Behaviour {
    Ok(u32),
    Fail(&'static str),
    Sleep { ms: u64, value: u32 },
    /// Never completes; sets the flag when the future is dropped.
    Hang(Arc<AtomicBool>),
}

struct DropFlag(Arc<AtomicBool>);

impl Drop for DropFlag {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

fn step(behaviour: Behaviour) -> Step {
    match behaviour {
        Behaviour::Ok(value) => Box::pin(async move { Ok(value) }),
        Behaviour::Fail(message) => Box::pin(async move { Err(message.to_string()) }),
        Behaviour::Sleep { ms, value } => Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(ms)).await;
            Ok(value)
        }),
        Behaviour::Hang(flag) => Box::pin(async move {
            let _guard = DropFlag(flag);
            std::future::pending::<Result<u32, String>>().await
        }),
    }
}

fn source(name: &str, behaviour: Behaviour) -> (String, impl FnOnce() -> Step + Send + 'static) {
    (name.to_string(), move || step(behaviour))
}

#[tokio::test(start_paused = true)]
async fn part1_aggregate_returns_results_in_source_order() {
    let sources = vec![
        source("slow", Behaviour::Sleep { ms: 300, value: 1 }),
        source("fast", Behaviour::Sleep { ms: 10, value: 2 }),
        source("mid", Behaviour::Sleep { ms: 100, value: 3 }),
    ];

    let results = aggregate(sources, Duration::from_secs(1))
        .await
        .expect("valid configuration");

    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(
        names,
        ["slow", "fast", "mid"],
        "results must follow registration order, not completion order"
    );
    assert_eq!(results[1].outcome, Outcome::Ok(2));
}

#[tokio::test(start_paused = true)]
async fn part1_aggregate_reports_ok_failed_and_timed_out_per_source() {
    let sources = vec![
        source("ok", Behaviour::Ok(1)),
        source("broken", Behaviour::Fail("boom")),
        source(
            "hung",
            Behaviour::Sleep {
                ms: 5_000,
                value: 9,
            },
        ),
    ];

    let results = aggregate(sources, Duration::from_millis(50))
        .await
        .expect("valid configuration");

    assert_eq!(
        results,
        vec![
            SourceResult {
                name: "ok".into(),
                outcome: Outcome::Ok(1)
            },
            SourceResult {
                name: "broken".into(),
                outcome: Outcome::Failed("boom".into())
            },
            SourceResult {
                name: "hung".into(),
                outcome: Outcome::TimedOut
            },
        ]
    );
}

#[tokio::test(start_paused = true)]
async fn part1_aggregate_source_timeout_error_is_failed_not_timed_out() {
    let sources = vec![source("upstream", Behaviour::Fail("upstream timeout"))];

    let results = aggregate(sources, Duration::from_secs(1))
        .await
        .expect("valid configuration");

    assert_eq!(
        results[0].outcome,
        Outcome::Failed("upstream timeout".into()),
        "an error returned by the source is Failed; only our own timer produces TimedOut"
    );
}

#[tokio::test(start_paused = true)]
async fn part1_aggregate_runs_sources_concurrently() {
    let sources = vec![
        source("a", Behaviour::Sleep { ms: 100, value: 1 }),
        source("b", Behaviour::Sleep { ms: 100, value: 2 }),
        source("c", Behaviour::Sleep { ms: 100, value: 3 }),
    ];

    let started = Instant::now();
    let results = aggregate(sources, Duration::from_secs(1))
        .await
        .expect("valid configuration");
    let elapsed = started.elapsed();

    assert_eq!(results.len(), 3);
    assert!(
        elapsed < Duration::from_millis(200),
        "three 100ms sources took {elapsed:?}; they must overlap, not run in sequence"
    );
}

#[tokio::test(start_paused = true)]
async fn part1_aggregate_zero_timeout_is_config_error_and_calls_nothing() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let sources = vec![(
        "probe".to_string(),
        move || -> Step {
            counter.fetch_add(1, Ordering::SeqCst);
            step(Behaviour::Ok(1))
        },
    )];

    let result = aggregate(sources, Duration::ZERO).await;

    assert_eq!(result.err(), Some(ConfigError::ZeroTimeout));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "no factory may run when the configuration is rejected"
    );
}

#[tokio::test(start_paused = true)]
async fn part1_aggregate_dropping_caller_aborts_in_flight_sources() {
    let dropped = Arc::new(AtomicBool::new(false));
    let sources = vec![source("hung", Behaviour::Hang(dropped.clone()))];

    let outer = tokio::time::timeout(
        Duration::from_millis(50),
        aggregate(sources, Duration::from_secs(60)),
    )
    .await;

    assert!(outer.is_err(), "the outer timeout must fire first");
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(
        dropped.load(Ordering::SeqCst),
        "dropping the caller must abort every in-flight source"
    );
}

#[tokio::test(start_paused = true)]
async fn part1_strict_returns_values_in_source_order() {
    let sources = vec![
        source("a", Behaviour::Sleep { ms: 50, value: 1 }),
        source("b", Behaviour::Ok(2)),
        source("c", Behaviour::Sleep { ms: 10, value: 3 }),
    ];

    let values = fetch_all_strict(sources)
        .await
        .expect("every source succeeds");
    assert_eq!(
        values,
        vec![
            ("a".to_string(), 1),
            ("b".to_string(), 2),
            ("c".to_string(), 3)
        ]
    );

    let empty: Vec<(String, NoSource)> = Vec::new();
    let none = fetch_all_strict(empty).await.expect("empty input succeeds");
    assert!(none.is_empty(), "empty input must yield an empty vec");
}

#[tokio::test(start_paused = true)]
async fn part1_strict_first_failure_names_the_source() {
    let sources = vec![
        source("ok", Behaviour::Sleep { ms: 1_000, value: 1 }),
        source("db", Behaviour::Fail("disk full")),
        source("cache", Behaviour::Sleep { ms: 1_000, value: 3 }),
    ];

    let err = fetch_all_strict(sources)
        .await
        .expect_err("one source fails");

    assert_eq!(err.name, "db");
    assert_eq!(err.error, "disk full");
}

#[tokio::test(start_paused = true)]
async fn part1_strict_aborts_siblings_before_returning() {
    let sibling_dropped = Arc::new(AtomicBool::new(false));
    let sources = vec![
        source("hung", Behaviour::Hang(sibling_dropped.clone())),
        source("bad", Behaviour::Fail("nope")),
    ];

    let err = fetch_all_strict(sources)
        .await
        .expect_err("second source fails");

    assert_eq!(err.name, "bad");
    assert!(
        sibling_dropped.load(Ordering::SeqCst),
        "the sibling must be cancelled and dropped before the error is returned"
    );
}

#[derive(Debug)]
struct Upstream(&'static str);

impl Display for Upstream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for Upstream {}

#[tokio::test(start_paused = true)]
async fn part1_strict_error_exposes_source_chain() {
    type Boxed = Pin<Box<dyn Future<Output = Result<u32, Upstream>> + Send>>;
    let sources = vec![(
        "db".to_string(),
        || -> Boxed { Box::pin(async { Err(Upstream("connection reset")) }) },
    )];

    let err = fetch_all_strict(sources)
        .await
        .expect_err("the only source fails");

    assert_eq!(err.name, "db");
    assert!(err.to_string().contains("db"), "Display should name the source");
    let cause = err.source().expect("StrictError exposes the source error");
    assert_eq!(cause.to_string(), "connection reset");
}
