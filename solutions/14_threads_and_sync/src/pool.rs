//! A fixed-size thread pool with a bounded job queue.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};

/// Returned by [`ThreadPool::try_execute`] when the queue has no room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueFull;

impl Display for QueueFull {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("job queue is full")
    }
}

impl Error for QueueFull {}

/// Totals reported when the pool shuts down.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PoolStats {
    /// Jobs that returned normally.
    pub completed: usize,
    /// Jobs that panicked. The worker survives and keeps serving.
    pub panicked: usize,
}

type Job = Box<dyn FnOnce() + Send>;

/// A pool of worker threads fed through a bounded queue.
///
/// Dropping the pool (or calling [`shutdown`](Self::shutdown)) stops accepting
/// work, lets queued jobs finish, and joins every worker.
pub struct ThreadPool {
    sender: Option<SyncSender<Job>>,
    workers: Vec<JoinHandle<PoolStats>>,
}

impl ThreadPool {
    /// Starts `workers` threads with room for `queue_size` queued jobs.
    ///
    /// # Panics
    ///
    /// Panics if `workers` is zero.
    pub fn new(workers: usize, queue_size: usize) -> Self {
        assert!(workers >= 1, "a pool needs at least one worker");
        let (sender, receiver) = mpsc::sync_channel::<Job>(queue_size);
        let receiver = Arc::new(Mutex::new(receiver));
        let workers = (0..workers)
            .map(|_| {
                let receiver = Arc::clone(&receiver);
                thread::spawn(move || worker_loop(&receiver))
            })
            .collect();
        Self {
            sender: Some(sender),
            workers,
        }
    }

    /// Queues `job`, blocking while the queue is full.
    pub fn execute(&self, job: impl FnOnce() + Send + 'static) {
        self.sender
            .as_ref()
            .expect("sender is present until shutdown")
            .send(Box::new(job))
            .expect("workers outlive the pool's sender");
    }

    /// Queues `job` without blocking.
    ///
    /// # Errors
    ///
    /// Returns [`QueueFull`] if no queue slot is free; the job is dropped.
    pub fn try_execute(&self, job: impl FnOnce() + Send + 'static) -> Result<(), QueueFull> {
        let sender = self.sender.as_ref().expect("sender is present until shutdown");
        match sender.try_send(Box::new(job)) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => Err(QueueFull),
        }
    }

    /// Stops accepting work, drains the queue, joins all workers and reports totals.
    pub fn shutdown(mut self) -> PoolStats {
        self.stop()
    }

    fn stop(&mut self) -> PoolStats {
        drop(self.sender.take());
        let mut total = PoolStats::default();
        for handle in self.workers.drain(..) {
            let stats = handle.join().expect("worker loop catches job panics");
            total.completed += stats.completed;
            total.panicked += stats.panicked;
        }
        total
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.stop();
    }
}

fn worker_loop(receiver: &Mutex<Receiver<Job>>) -> PoolStats {
    let mut stats = PoolStats::default();
    loop {
        // The lock is held only for the `recv`, not while the job runs.
        let next = receiver
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .recv();
        let Ok(job) = next else { break };
        match panic::catch_unwind(AssertUnwindSafe(job)) {
            Ok(()) => stats.completed += 1,
            Err(_) => stats.panicked += 1,
        }
    }
    stats
}
