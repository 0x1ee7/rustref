//! Part 2: cancellation tokens, cancel-safe receiving, cleanup on every exit path, and shielded
//! background work.

mod support;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use ex15_async_tokio::cancel::{CancelToken, RunError, collect_until_cancelled, run_with_cleanup, shielded_commit};
use support::{Captured, capture_errors};
use tokio::sync::mpsc;

const ONE_SEC: Duration = Duration::from_secs(1);

#[tokio::test(start_paused = true)]
async fn part2_token_clones_share_one_flag() {
    let token = CancelToken::new();
    assert!(!token.is_cancelled(), "a new token starts uncancelled");
    assert!(!CancelToken::default().is_cancelled(), "default is uncancelled too");

    let other = token.clone();
    other.cancel();
    assert!(token.is_cancelled(), "a cancel through any clone is visible to all");

    other.cancel();
    assert!(token.is_cancelled(), "cancel is idempotent");
}

#[tokio::test(start_paused = true)]
async fn part2_cancelled_completes_at_once_when_already_set() {
    let token = CancelToken::new();
    token.cancel();

    tokio::time::timeout(ONE_SEC, token.cancelled())
        .await
        .expect("an already-cancelled token must complete immediately");
}

#[tokio::test(start_paused = true)]
async fn part2_cancelled_wakes_a_pending_waiter() {
    let token = CancelToken::new();
    let waiter = {
        let token = token.clone();
        tokio::spawn(async move { token.cancelled().await })
    };

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(!waiter.is_finished(), "the waiter must block until cancellation");

    token.cancel();
    tokio::time::timeout(ONE_SEC, waiter)
        .await
        .expect("cancel must wake the waiter")
        .expect("waiter task must not panic");
}

#[tokio::test(start_paused = true)]
async fn part2_collect_returns_everything_once_senders_are_gone() {
    let (tx, mut rx) = mpsc::channel(8);
    for n in 1..=3 {
        tx.send(n).await.expect("receiver is alive");
    }
    drop(tx);

    let token = CancelToken::new();
    let got = collect_until_cancelled(&mut rx, &token).await;

    assert_eq!(got, vec![1, 2, 3], "messages come back in order");
    assert!(!token.is_cancelled(), "channel closure is not cancellation");
}

#[tokio::test(start_paused = true)]
async fn part2_collect_stops_at_cancel_and_leaves_unread_messages() {
    let (tx, mut rx) = mpsc::channel(8);
    for n in 1..=5 {
        tx.send(n).await.expect("receiver is alive");
    }
    let token = CancelToken::new();
    token.cancel();

    let got = collect_until_cancelled(&mut rx, &token).await;

    assert!(
        got.is_empty(),
        "a token cancelled before the call must win over ready messages (biased select)"
    );
    assert_eq!(rx.recv().await, Some(1), "unread messages stay in the channel");
    drop(tx);
}

#[tokio::test(start_paused = true)]
async fn part2_collect_is_cancel_safe_and_keeps_the_channel_usable() {
    let (tx, mut rx) = mpsc::channel(8);
    tx.send(1).await.expect("receiver is alive");
    tx.send(2).await.expect("receiver is alive");

    let token = CancelToken::new();
    let canceller = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        canceller.cancel();
    });

    let got = tokio::time::timeout(ONE_SEC, collect_until_cancelled(&mut rx, &token))
        .await
        .expect("collect must return once cancelled, even while idle");
    assert_eq!(got, vec![1, 2]);

    tx.send(9).await.expect("cancelling must not close the channel");
    assert_eq!(rx.recv().await, Some(9));
}

#[tokio::test(start_paused = true)]
async fn part2_run_success_awaits_cleanup_before_returning() {
    let cleaned = Arc::new(AtomicUsize::new(0));
    let counter = cleaned.clone();

    let out = run_with_cleanup(
        || async { Ok::<u32, String>(7) },
        move || async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            counter.fetch_add(1, Ordering::SeqCst);
        },
        ONE_SEC,
    )
    .await;

    assert_eq!(out, Ok(7));
    assert_eq!(cleaned.load(Ordering::SeqCst), 1, "cleanup must be awaited in place on success");
}

#[tokio::test(start_paused = true)]
async fn part2_run_body_error_still_runs_cleanup() {
    let cleaned = Arc::new(AtomicUsize::new(0));
    let counter = cleaned.clone();

    let out = run_with_cleanup(
        || async { Err::<u32, String>("bad".to_string()) },
        move || async move {
            counter.fetch_add(1, Ordering::SeqCst);
        },
        ONE_SEC,
    )
    .await;

    assert_eq!(out, Err(RunError::Body("bad".to_string())));
    assert_eq!(cleaned.load(Ordering::SeqCst), 1, "cleanup runs once after a body error");
}

#[tokio::test(start_paused = true)]
async fn part2_run_timeout_reports_timed_out_and_runs_cleanup() {
    let cleaned = Arc::new(AtomicUsize::new(0));
    let counter = cleaned.clone();

    let out = run_with_cleanup(
        || async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            Ok::<u32, String>(1)
        },
        move || async move {
            counter.fetch_add(1, Ordering::SeqCst);
        },
        Duration::from_millis(100),
    )
    .await;

    assert_eq!(out, Err(RunError::TimedOut));
    assert_eq!(cleaned.load(Ordering::SeqCst), 1, "cleanup runs once after a timeout");
}

#[tokio::test(start_paused = true)]
async fn part2_run_caller_cancellation_hands_cleanup_to_a_detached_task() {
    let cleaned = Arc::new(AtomicBool::new(false));
    let flag = cleaned.clone();

    let outer = tokio::time::timeout(
        Duration::from_millis(50),
        run_with_cleanup(
            || async {
                tokio::time::sleep(Duration::from_secs(10)).await;
                Ok::<u32, String>(1)
            },
            move || async move { flag.store(true, Ordering::SeqCst) },
            Duration::from_secs(60),
        ),
    )
    .await;

    assert!(outer.is_err(), "the caller's own timeout must fire first");
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(
        cleaned.load(Ordering::SeqCst),
        "cleanup must still run after the caller is cancelled"
    );
}

#[tokio::test(start_paused = true)]
async fn part2_shielded_commit_finishes_after_caller_is_cancelled() {
    let done = Arc::new(AtomicBool::new(false));
    let flag = done.clone();

    let outer = tokio::time::timeout(
        Duration::from_millis(10),
        shielded_commit(move || async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            flag.store(true, Ordering::SeqCst);
            Ok::<u32, String>(5)
        }),
    )
    .await;

    assert!(outer.is_err(), "the caller gave up before the commit finished");
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        done.load(Ordering::SeqCst),
        "the commit must keep running after its caller is gone"
    );
}

#[tokio::test(start_paused = true)]
async fn part2_shielded_commit_logs_error_when_caller_is_gone() {
    let logs = Captured::default();
    let _guard = capture_errors(logs.clone());

    let outer = tokio::time::timeout(
        Duration::from_millis(10),
        shielded_commit(|| async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Err::<u32, String>("disk gone".to_string())
        }),
    )
    .await;

    assert!(outer.is_err());
    tokio::time::sleep(Duration::from_millis(100)).await;
    let text = logs.text();
    assert!(text.contains("disk gone"), "the failure must be logged; got {text:?}");
    assert!(
        text.contains("commit"),
        "the log must use the target `commit`; got {text:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn part2_shielded_commit_returns_results_and_stays_silent_for_waiting_callers() {
    let logs = Captured::default();
    let _guard = capture_errors(logs.clone());

    let ok = shielded_commit(|| async { Ok::<u32, String>(5) }).await;
    assert_eq!(ok, Ok(5));

    let err = shielded_commit(|| async { Err::<u32, String>("rejected".to_string()) }).await;
    assert_eq!(err, Err("rejected".to_string()));

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(
        logs.text().is_empty(),
        "errors returned to a waiting caller are not logged; got {:?}",
        logs.text()
    );
}
