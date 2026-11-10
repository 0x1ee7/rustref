//! Part 5: a worker pool with per-job replies, graceful drain, and abort.

mod support;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use ex15_async_tokio::error::ConfigError;
use ex15_async_tokio::pool::{PoolError, Shutdown, WorkerPool};
use tokio::time::Instant;

const ONE_SEC: Duration = Duration::from_secs(1);

fn square_pool(workers: usize) -> WorkerPool<u32, u32, String> {
    WorkerPool::start(
        |n: u32| async move { Ok::<u32, String>(n * n) },
        workers,
    )
    .expect("valid worker count")
}

#[tokio::test(start_paused = true)]
async fn part5_zero_workers_is_rejected() {
    let result = WorkerPool::<u32, u32, String>::start(
        |n: u32| async move { Ok::<u32, String>(n) },
        0,
    );
    assert!(matches!(result, Err(ConfigError::ZeroWorkers)));
}

#[tokio::test(start_paused = true)]
async fn part5_submit_returns_the_handler_result_to_the_caller() {
    let pool = square_pool(2);

    assert_eq!(pool.submit(7).await, Ok(49));
    assert_eq!(pool.submit(0).await, Ok(0));
    assert_eq!(pool.processed(), 2);

    pool.close(Shutdown::Drain).await;
}

#[tokio::test(start_paused = true)]
async fn part5_handler_error_is_returned_only_to_its_submitter() {
    let pool = WorkerPool::start(
        |n: u32| async move {
            100u32
                .checked_div(n)
                .ok_or_else(|| "zero is not allowed".to_string())
        },
        2,
    )
    .expect("valid worker count");

    assert_eq!(
        pool.submit(0).await,
        Err(PoolError::Handler("zero is not allowed".to_string()))
    );
    assert_eq!(pool.submit(4).await, Ok(25), "the pool keeps serving after a failure");
    assert_eq!(pool.processed(), 1, "only successful jobs count as processed");

    pool.close(Shutdown::Drain).await;
}

#[tokio::test(start_paused = true)]
async fn part5_jobs_run_concurrently_across_workers() {
    let pool = Arc::new(
        WorkerPool::start(
            |_: u32| async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok::<u32, String>(0)
            },
            4,
        )
        .expect("valid worker count"),
    );

    let started = Instant::now();
    let mut handles = Vec::new();
    for n in 0..4u32 {
        let pool = pool.clone();
        handles.push(tokio::spawn(async move { pool.submit(n).await }));
    }
    for handle in handles {
        handle
            .await
            .expect("submitter task must not panic")
            .expect("job succeeds");
    }
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_millis(200),
        "four 100ms jobs on four workers took {elapsed:?}; they must overlap"
    );
    pool.close(Shutdown::Drain).await;
}

#[tokio::test(start_paused = true)]
async fn part5_drain_finishes_jobs_already_queued() {
    let pool = Arc::new(
        WorkerPool::start(
            |n: u32| async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Ok::<u32, String>(n)
            },
            1,
        )
        .expect("valid worker count"),
    );

    let mut handles = Vec::new();
    for n in 0..3u32 {
        let pool = pool.clone();
        handles.push(tokio::spawn(async move { pool.submit(n).await }));
    }
    // Let the submitters enqueue their jobs before closing.
    tokio::time::sleep(Duration::from_millis(1)).await;

    pool.close(Shutdown::Drain).await;

    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.expect("submitter task must not panic"));
    }
    assert_eq!(
        results,
        vec![Ok(0), Ok(1), Ok(2)],
        "drain completes queued work instead of failing it"
    );
    assert_eq!(pool.processed(), 3);
}

#[tokio::test(start_paused = true)]
async fn part5_abort_fails_pending_submitters_with_aborted() {
    let pool = Arc::new(
        WorkerPool::start(
            |_: u32| async {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok::<u32, String>(0)
            },
            1,
        )
        .expect("valid worker count"),
    );

    let mut handles = Vec::new();
    for n in 0..3u32 {
        let pool = pool.clone();
        handles.push(tokio::spawn(async move { pool.submit(n).await }));
    }
    tokio::time::sleep(Duration::from_millis(1)).await;

    tokio::time::timeout(ONE_SEC, pool.close(Shutdown::Abort))
        .await
        .expect("abort must not wait for the 60s job");

    for handle in handles {
        let result = handle.await.expect("submitter task must not panic");
        assert_eq!(
            result,
            Err(PoolError::Aborted),
            "in-flight and queued submitters all see Aborted"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn part5_submit_after_close_is_closed_and_close_is_idempotent() {
    let pool = square_pool(2);
    pool.close(Shutdown::Drain).await;

    assert_eq!(pool.submit(3).await, Err(PoolError::Closed));

    tokio::time::timeout(ONE_SEC, pool.close(Shutdown::Drain))
        .await
        .expect("a second close returns immediately");
}

#[tokio::test(start_paused = true)]
async fn part5_dropped_submitter_is_skipped_by_the_worker() {
    let runs = Arc::new(AtomicUsize::new(0));
    let counter = runs.clone();
    let pool = Arc::new(
        WorkerPool::start(
            move |_: u32| {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    Ok::<u32, String>(0)
                }
            },
            1,
        )
        .expect("valid worker count"),
    );

    // The first job occupies the only worker; the second waits in the queue.
    let busy = {
        let pool = pool.clone();
        tokio::spawn(async move { pool.submit(1).await })
    };
    tokio::time::sleep(Duration::from_millis(1)).await;
    let queued = {
        let pool = pool.clone();
        tokio::spawn(async move { pool.submit(2).await })
    };
    tokio::time::sleep(Duration::from_millis(1)).await;
    queued.abort();
    let _ = queued.await;

    busy.await
        .expect("submitter task must not panic")
        .expect("first job succeeds");
    pool.close(Shutdown::Drain).await;

    assert_eq!(
        runs.load(Ordering::SeqCst),
        1,
        "a job whose submitter gave up must not run"
    );
}
