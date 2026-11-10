//! Part 5 — queue-backed worker pool with drain-or-abort shutdown.

use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use thiserror::Error;
use tokio::sync::{Mutex as AsyncMutex, mpsc, oneshot};
use tokio::task::JoinHandle;

use crate::cancel::CancelToken;
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

/// One queued submission and the channel its result goes back on.
struct Job<T, R, E> {
    item: T,
    reply: oneshot::Sender<Result<R, PoolError<E>>>,
}

/// A pool of `workers` tasks that process submitted items through one handler.
///
/// Dropping the pool aborts any workers still running.
pub struct WorkerPool<T, R, E> {
    /// `None` once closed; dropping the last sender lets workers exit after draining.
    sender: Mutex<Option<mpsc::UnboundedSender<Job<T, R, E>>>>,
    queue: Arc<AsyncMutex<mpsc::UnboundedReceiver<Job<T, R, E>>>>,
    workers: Mutex<Vec<JoinHandle<()>>>,
    abort: CancelToken,
    processed: Arc<AtomicUsize>,
}

impl<T, R, E> WorkerPool<T, R, E>
where
    T: Send + 'static,
    R: Send + 'static,
    E: Send + 'static,
{
    /// Spawns `workers` tasks that call `handler` for each submitted item.
    /// `workers == 0` is `ConfigError::ZeroWorkers`.
    ///
    /// # Panics
    /// Panics if called outside a Tokio runtime.
    pub fn start<H, Fut>(handler: H, workers: usize) -> Result<Self, ConfigError>
    where
        H: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<R, E>> + Send + 'static,
    {
        if workers == 0 {
            return Err(ConfigError::ZeroWorkers);
        }
        let (sender, queue) = mpsc::unbounded_channel();
        let queue = Arc::new(AsyncMutex::new(queue));
        let abort = CancelToken::new();
        let processed = Arc::new(AtomicUsize::new(0));
        let handler = Arc::new(handler);
        let handles = (0..workers)
            .map(|_| {
                tokio::spawn(run_worker(
                    Arc::clone(&handler),
                    Arc::clone(&queue),
                    abort.clone(),
                    Arc::clone(&processed),
                ))
            })
            .collect();
        Ok(Self {
            sender: Mutex::new(Some(sender)),
            queue,
            workers: Mutex::new(handles),
            abort,
            processed,
        })
    }

    /// Queues `item` and waits for its result.
    ///
    /// A handler error goes only to this submitter. If the submitter is dropped while the item is
    /// still queued, the worker sees the closed reply channel and skips the item.
    pub async fn submit(&self, item: T) -> Result<R, PoolError<E>> {
        let (reply, outcome) = oneshot::channel();
        {
            let guard = self.sender.lock().expect("pool sender lock poisoned");
            let Some(sender) = guard.as_ref() else {
                return Err(PoolError::Closed);
            };
            sender
                .send(Job { item, reply })
                .map_err(|_| PoolError::Closed)?;
        }
        // A dropped reply means the job was discarded by an abort.
        outcome.await.unwrap_or(Err(PoolError::Aborted))
    }

    /// Stops accepting submissions and shuts the workers down according to `mode`.
    ///
    /// Awaiting `close` returns only after every worker task has finished. A second call returns
    /// immediately. A worker that panicked has already lost its job; close ignores its join error.
    pub async fn close(&self, mode: Shutdown) {
        drop(self.sender.lock().expect("pool sender lock poisoned").take());
        if mode == Shutdown::Abort {
            self.abort.cancel();
        }
        let handles = std::mem::take(&mut *self.workers.lock().expect("pool workers lock poisoned"));
        for handle in handles {
            let _ = handle.await;
        }
        if mode == Shutdown::Abort {
            // Dropping queued jobs drops their reply senders, so those submitters see `Aborted`.
            let mut queue = self.queue.lock().await;
            while queue.try_recv().is_ok() {}
        }
    }

    /// Number of items the handler completed successfully.
    pub fn processed(&self) -> usize {
        self.processed.load(Ordering::SeqCst)
    }
}

impl<T, R, E> Drop for WorkerPool<T, R, E> {
    fn drop(&mut self) {
        self.abort.cancel();
    }
}

/// One worker: takes jobs off the shared queue until the queue closes or the pool aborts.
async fn run_worker<T, R, E, H, Fut>(
    handler: Arc<H>,
    queue: Arc<AsyncMutex<mpsc::UnboundedReceiver<Job<T, R, E>>>>,
    abort: CancelToken,
    processed: Arc<AtomicUsize>,
) where
    T: Send + 'static,
    R: Send + 'static,
    E: Send + 'static,
    H: Fn(T) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<R, E>> + Send + 'static,
{
    loop {
        let next = {
            let mut queue = queue.lock().await;
            tokio::select! {
                biased;
                () = abort.cancelled() => None,
                job = queue.recv() => job,
            }
        };
        let Some(Job { item, reply }) = next else {
            return;
        };
        if reply.is_closed() {
            continue;
        }

        let work = (*handler)(item);
        let outcome = tokio::select! {
            biased;
            () = abort.cancelled() => {
                let _ = reply.send(Err(PoolError::Aborted));
                return;
            }
            result = work => result,
        };
        match outcome {
            Ok(value) => {
                processed.fetch_add(1, Ordering::SeqCst);
                let _ = reply.send(Ok(value));
            }
            Err(error) => {
                let _ = reply.send(Err(PoolError::Handler(error)));
            }
        }
    }
}
