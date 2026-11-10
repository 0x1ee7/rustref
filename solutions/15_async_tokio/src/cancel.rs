//! Part 2 — cancellation: a `watch`-based token, `select!` cancel-safety, cleanup on every
//! exit path, and shielded background work.

use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot, watch};

/// A cloneable, one-way cancellation flag built on a `watch` channel.
///
/// `cancel` is idempotent. Every clone observes the same state.
#[derive(Clone)]
pub struct CancelToken {
    tx: Arc<watch::Sender<bool>>,
}

impl CancelToken {
    /// Creates a token that is not yet cancelled.
    pub fn new() -> Self {
        let (tx, _rx) = watch::channel(false);
        Self { tx: Arc::new(tx) }
    }

    /// Sets the flag and wakes every waiter. Calling it again has no further effect.
    pub fn cancel(&self) {
        self.tx.send_replace(true);
    }

    /// Whether [`cancel`](Self::cancel) has been called on any clone.
    pub fn is_cancelled(&self) -> bool {
        *self.tx.borrow()
    }

    /// Completes once the token is cancelled, immediately if it already is.
    pub async fn cancelled(&self) {
        let mut rx = self.tx.subscribe();
        // The sender lives in `self`, so `wait_for` cannot fail while we hold a reference.
        let _ = rx.wait_for(|cancelled| *cancelled).await;
    }
}

impl Default for CancelToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Receives from `rx` until the token is cancelled or every sender is dropped.
///
/// The select is biased towards cancellation, so a cancelled token stops the loop before any
/// further message is taken. Messages left unread stay in the channel.
pub async fn collect_until_cancelled<T>(rx: &mut mpsc::Receiver<T>, token: &CancelToken) -> Vec<T> {
    let mut collected = Vec::new();
    loop {
        tokio::select! {
            biased;
            () = token.cancelled() => break,
            message = rx.recv() => match message {
                Some(message) => collected.push(message),
                None => break,
            },
        }
    }
    collected
}

/// Why [`run_with_cleanup`] did not return a value.
#[derive(Debug, PartialEq, Eq)]
pub enum RunError<E> {
    /// The body returned an error.
    Body(E),
    /// The body did not finish within the timeout.
    TimedOut,
}

/// Owns the cleanup closure until it is run. If the guard is dropped first (the caller was
/// cancelled while the body was pending), the cleanup is handed to a detached task.
struct CleanupGuard<C, CFut>
where
    C: FnOnce() -> CFut + Send + 'static,
    CFut: Future<Output = ()> + Send + 'static,
{
    cleanup: Option<C>,
}

impl<C, CFut> Drop for CleanupGuard<C, CFut>
where
    C: FnOnce() -> CFut + Send + 'static,
    CFut: Future<Output = ()> + Send + 'static,
{
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            tokio::spawn(cleanup());
        }
    }
}

/// Runs `body` under a timeout and guarantees `cleanup` runs exactly once.
///
/// `cleanup` is awaited in place on success, body error and timeout. If the caller's future is
/// dropped while the body is pending, Rust has no async drop, so the guard spawns the cleanup.
pub async fn run_with_cleanup<T, E, F, Fut, C, CFut>(
    body: F,
    cleanup: C,
    timeout: Duration,
) -> Result<T, RunError<E>>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    C: FnOnce() -> CFut + Send + 'static,
    CFut: Future<Output = ()> + Send + 'static,
{
    let mut guard = CleanupGuard {
        cleanup: Some(cleanup),
    };
    let result = match tokio::time::timeout(timeout, body()).await {
        Err(_elapsed) => Err(RunError::TimedOut),
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(RunError::Body(error)),
    };
    if let Some(cleanup) = guard.cleanup.take() {
        cleanup().await;
    }
    result
}

/// Runs `operation` as a detached task so that cancelling the caller does not stop it.
///
/// If the caller is still waiting, it receives the result. If the caller is gone and the
/// operation failed, the error is logged at `ERROR` with target `commit`.
///
/// # Panics
/// Propagates a panic in `operation` to the caller if it is still waiting.
pub async fn shielded_commit<T, E, F, Fut>(operation: F) -> Result<T, E>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: Display + Send + 'static,
{
    let (reply, waiting) = oneshot::channel();
    tokio::spawn(async move {
        let result = operation().await;
        // `send` hands the value back when the caller has gone; only then is a failure logged.
        if let Err(result) = reply.send(result)
            && let Err(error) = result
        {
            tracing::error!(target: "commit", error = %error, "shielded operation failed after its caller was dropped");
        }
    });
    waiting
        .await
        .expect("shielded operation panicked before replying")
}
