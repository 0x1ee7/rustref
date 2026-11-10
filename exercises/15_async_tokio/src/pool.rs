//! Part 5 — queue-backed worker pool with drain-or-abort shutdown.

use std::future::Future;
use std::marker::PhantomData;

use thiserror::Error;

use crate::error::ConfigError;

/// How [`WorkerPool::close`] treats work that has not finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shutdown {
    /// Stop accepting submissions, finish everything already queued, then stop the workers.
    Drain,
    /// Stop accepting submissions, cancel the workers, and fail all pending submissions.
    Abort,
}

/// Why a submission produced no value.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PoolError<E> {
    /// The pool was closed before this submission was accepted.
    #[error("pool is closed")]
    Closed,
    /// The pool was aborted before this submission completed.
    #[error("submission aborted before completion")]
    Aborted,
    /// The handler returned an error for this item.
    #[error("handler failed: {0}")]
    Handler(E),
}

/// A pool of `workers` tasks that process submitted items through one handler.
///
/// The pool only exists while running: `start` spawns the workers, so "submit before start" is
/// unrepresentable. Dropping the pool aborts any workers still running.
pub struct WorkerPool<T, R, E> {
    _types: PhantomData<(T, R, E)>,
}

impl<T, R, E> WorkerPool<T, R, E>
where
    T: Send + 'static,
    R: Send + 'static,
    E: Send + 'static,
{
    /// Spawns `workers` tasks that call `handler` for each submitted item.
    /// `workers == 0` is `ConfigError::ZeroWorkers`.
    pub fn start<H, Fut>(_handler: H, _workers: usize) -> Result<Self, ConfigError>
    where
        H: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<R, E>> + Send + 'static,
    {
        todo!()
    }

    /// Queues `item` and waits for its result.
    ///
    /// A handler error goes only to this submitter. If the submitter is dropped while the item is
    /// still queued, the item is skipped and the handler never runs for it.
    pub async fn submit(&self, _item: T) -> Result<R, PoolError<E>> {
        todo!()
    }

    /// Stops accepting submissions and shuts the workers down according to `mode`.
    ///
    /// Awaiting `close` returns only after every worker task has finished. A second call returns
    /// immediately.
    pub async fn close(&self, _mode: Shutdown) {
        todo!()
    }

    /// Number of items the handler completed successfully.
    pub fn processed(&self) -> usize {
        todo!()
    }
}
