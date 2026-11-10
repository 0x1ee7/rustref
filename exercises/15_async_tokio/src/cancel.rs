//! Part 2 — cancellation: a `watch`-based token, `select!` cancel-safety, cleanup on every
//! exit path, and shielded background work.

use std::fmt::Display;
use std::future::Future;
use std::time::Duration;

use tokio::sync::{mpsc, watch};

/// A cloneable, one-way cancellation flag built on a `watch` channel.
///
/// `cancel` is idempotent. Every clone observes the same state. `cancelled()` completes as soon
/// as the flag is set, and immediately if it already is.
#[derive(Clone)]
pub struct CancelToken {
    /// Engineer: the shared `watch` sender (an `Arc` because `watch::Sender` is not `Clone`).
    _tx: std::sync::Arc<watch::Sender<bool>>,
}

impl CancelToken {
    /// Creates an uncancelled token.
    pub fn new() -> Self {
        todo!()
    }

    /// Sets the flag and wakes every waiter.
    pub fn cancel(&self) {
        todo!()
    }

    /// Returns whether [`cancel`](Self::cancel) has been called on any clone.
    pub fn is_cancelled(&self) -> bool {
        todo!()
    }

    /// Completes once the token is cancelled.
    pub async fn cancelled(&self) {
        todo!()
    }
}

impl Default for CancelToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Receives from `rx` until the token is cancelled or every sender is dropped.
///
/// Returns the messages received, in order. Messages that were not received before the
/// cancellation stay in the channel.
pub async fn collect_until_cancelled<T>(
    _rx: &mut mpsc::Receiver<T>,
    _token: &CancelToken,
) -> Vec<T> {
    todo!()
}

/// Why [`run_with_cleanup`] did not return a value.
#[derive(Debug, PartialEq, Eq)]
pub enum RunError<E> {
    /// The body returned `Err(e)`.
    Body(E),
    /// The body did not finish within the timeout.
    TimedOut,
}

/// Runs `body` under a timeout and guarantees `cleanup` runs exactly once.
///
/// Contract:
/// - Returns the body's value on success.
/// - `cleanup` is awaited on every exit path: success, body error, timeout, and external
///   cancellation of the caller's future.
/// - On external cancellation, `cleanup` cannot be awaited in place (Rust has no async drop);
///   it is handed to a detached task.
pub async fn run_with_cleanup<T, E, F, Fut, C, CFut>(
    _body: F,
    _cleanup: C,
    _timeout: Duration,
) -> Result<T, RunError<E>>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    C: FnOnce() -> CFut + Send + 'static,
    CFut: Future<Output = ()> + Send + 'static,
{
    todo!()
}

/// Runs `operation` as a detached task so that cancelling the caller does not stop it.
///
/// Contract:
/// - Returns the operation's result to a waiting caller.
/// - If the caller is cancelled, the operation keeps running to completion.
/// - If the operation fails after the caller is gone, the error is logged at `ERROR` with
///   target `commit`. A cancelled or successful background operation logs nothing.
pub async fn shielded_commit<T, E, F, Fut>(_operation: F) -> Result<T, E>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: Display + Send + 'static,
{
    todo!()
}
