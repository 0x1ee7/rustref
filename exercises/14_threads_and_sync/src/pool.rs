//! Part 6: a fixed-size thread pool with a bounded queue.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Returned by [`ThreadPool::try_execute`] when the queue has no room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueFull;

impl Display for QueueFull {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let _ = f;
        todo!()
    }
}

impl Error for QueueFull {}

/// Totals reported when the pool shuts down.
///
/// `completed` counts jobs that returned normally; `panicked` counts jobs that
/// panicked. A panicking job does not kill its worker.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PoolStats {
    /// Jobs that returned normally.
    pub completed: usize,
    /// Jobs that panicked.
    pub panicked: usize,
}

/// A pool of worker threads fed through a bounded queue.
///
/// Requirements:
/// - `new(0, _)` panics with a message mentioning `worker`;
/// - jobs run on up to `workers` distinct threads;
/// - `execute` blocks while the queue is full;
/// - `try_execute` never blocks and returns `Err(QueueFull)` when no slot is free;
/// - dropping the pool, or calling `shutdown`, stops intake, runs every job
///   already queued, and joins the workers before returning.
pub struct ThreadPool {
    // Private: channel sender and worker handles.
}

impl ThreadPool {
    /// Starts `workers` threads with room for `queue_size` queued jobs.
    pub fn new(workers: usize, queue_size: usize) -> Self {
        let _ = (workers, queue_size);
        todo!()
    }

    /// Queues `job`, blocking while the queue is full.
    pub fn execute(&self, job: impl FnOnce() + Send + 'static) {
        let _ = job;
        todo!()
    }

    /// Queues `job` without blocking.
    pub fn try_execute(&self, job: impl FnOnce() + Send + 'static) -> Result<(), QueueFull> {
        let _ = job;
        todo!()
    }

    /// Stops intake, drains the queue, joins workers and returns the totals.
    pub fn shutdown(self) -> PoolStats {
        todo!()
    }
}
