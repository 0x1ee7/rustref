//! Part 3: bounded concurrency with a semaphore, and blocking file IO off the runtime.

mod support;

use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use ex15_async_tokio::bounded::{BoundedFetcher, ReadFilesError, read_files};
use ex15_async_tokio::error::ConfigError;

fn fetcher(limit: usize) -> BoundedFetcher {
    BoundedFetcher::new(limit).expect("valid limit")
}

/// Counts live fetches: incremented on creation, decremented when the guard is dropped.
struct Live(Arc<AtomicUsize>);

impl Live {
    fn enter(counter: &Arc<AtomicUsize>) -> Self {
        counter.fetch_add(1, Ordering::SeqCst);
        Self(counter.clone())
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

#[tokio::test(start_paused = true)]
async fn part3_zero_limit_is_rejected() {
    assert!(matches!(BoundedFetcher::new(0), Err(ConfigError::ZeroLimit)));
}

#[tokio::test(start_paused = true)]
async fn part3_run_preserves_input_order_even_when_completion_order_differs() {
    let results = fetcher(4)
        .run((0..8u64).collect::<Vec<_>>(), |n: u64| async move {
            // Later items finish first.
            tokio::time::sleep(Duration::from_millis(80 - n * 10)).await;
            Ok::<u64, String>(n * n)
        })
        .await
        .expect("every fetch succeeds");

    assert_eq!(results, (0..8u64).map(|n| n * n).collect::<Vec<_>>());
}

#[tokio::test(start_paused = true)]
async fn part3_peak_in_flight_equals_the_limit() {
    let f = fetcher(3);
    let results = f
        .run((0..12u32).collect::<Vec<_>>(), |_: u32| async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok::<u32, String>(0)
        })
        .await
        .expect("every fetch succeeds");

    assert_eq!(results.len(), 12);
    assert_eq!(
        f.peak_in_flight(),
        3,
        "12 items under a limit of 3 must saturate the limit and never exceed it"
    );
}

#[tokio::test(start_paused = true)]
async fn part3_limit_of_one_runs_strictly_in_sequence() {
    let f = fetcher(1);
    f.run((0..5u32).collect::<Vec<_>>(), |_: u32| async {
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok::<u32, String>(0)
    })
    .await
    .expect("every fetch succeeds");

    assert_eq!(f.peak_in_flight(), 1);
}

#[tokio::test(start_paused = true)]
async fn part3_failure_returns_first_error_after_siblings_have_stopped() {
    let live = Arc::new(AtomicUsize::new(0));
    let tracker = live.clone();

    let err = fetcher(5)
        .run((0..5usize).collect::<Vec<_>>(), move |n: usize| {
            let guard = Live::enter(&tracker);
            async move {
                let _guard = guard;
                if n == 2 {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    return Err::<usize, String>("bad".to_string());
                }
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok(n)
            }
        })
        .await
        .expect_err("item 2 fails");

    assert_eq!(err.index, 2, "the error must carry the failing item's index");
    assert_eq!(err.error, "bad");
    assert_eq!(
        live.load(Ordering::SeqCst),
        0,
        "every sibling must be cancelled and finished before run returns"
    );
}

#[tokio::test(start_paused = true)]
async fn part3_limit_is_shared_by_concurrent_runs() {
    let f = fetcher(2);
    let (a, b) = tokio::join!(
        f.run((0..4u32).collect::<Vec<_>>(), |n: u32| async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok::<u32, String>(n)
        }),
        f.run((10..14u32).collect::<Vec<_>>(), |n: u32| async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok::<u32, String>(n)
        }),
    );

    assert_eq!(a.expect("first run succeeds"), vec![0, 1, 2, 3]);
    assert_eq!(b.expect("second run succeeds"), vec![10, 11, 12, 13]);
    assert_eq!(f.peak_in_flight(), 2, "two runs on one fetcher share one limit");
}

#[tokio::test(start_paused = true)]
async fn part3_empty_input_is_ok_and_empty() {
    let results = fetcher(2)
        .run(Vec::<u32>::new(), |n: u32| async move { Ok::<u32, String>(n) })
        .await
        .expect("empty input succeeds");
    assert!(results.is_empty());
}

#[tokio::test]
async fn part3_read_files_returns_contents_in_input_order() {
    let dir = tempfile::tempdir().expect("temp dir");
    let paths: Vec<PathBuf> = (0..4)
        .map(|i| {
            let path = dir.path().join(format!("f{i}.txt"));
            std::fs::write(&path, format!("contents {i}")).expect("write fixture");
            path
        })
        .collect();

    let files = read_files(paths.clone(), 2)
        .await
        .expect("every file exists");

    assert_eq!(files.len(), 4);
    for (i, (path, bytes)) in files.iter().enumerate() {
        assert_eq!(path, &paths[i], "paths must keep input order");
        assert_eq!(bytes.as_slice(), format!("contents {i}").as_bytes());
    }

    let none = read_files(Vec::new(), 2).await.expect("empty input succeeds");
    assert!(none.is_empty());
}

#[tokio::test]
async fn part3_read_files_reports_the_index_of_a_missing_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = dir.path().join("a.txt");
    let c = dir.path().join("c.txt");
    std::fs::write(&a, "a").expect("write fixture");
    std::fs::write(&c, "c").expect("write fixture");
    let missing = dir.path().join("missing.txt");

    let err = read_files(vec![a, missing, c], 2)
        .await
        .expect_err("the middle file does not exist");

    match err {
        ReadFilesError::Io { index, error } => {
            assert_eq!(index, 1, "the error must name the failing position");
            assert_eq!(error.kind(), ErrorKind::NotFound);
        }
        other => panic!("expected an Io error, got {other:?}"),
    }
}

#[tokio::test]
async fn part3_read_files_zero_limit_is_rejected_before_any_io() {
    let err = read_files(vec![PathBuf::from("never-read.txt")], 0)
        .await
        .expect_err("limit zero is invalid");

    assert!(matches!(
        err,
        ReadFilesError::Config(ConfigError::ZeroLimit)
    ));
}
