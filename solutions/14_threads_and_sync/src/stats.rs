//! Lock-light shared state: counters, a stop flag and a one-shot handoff.

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError, RwLock};

/// Named monotonic counters shared between threads.
///
/// Reads of existing counters take only the read lock; the write lock is
/// needed once per new name. The counter values themselves are plain atomics
/// with `Relaxed` ordering: no other memory is published through them.
#[derive(Debug, Default)]
pub struct Counters {
    map: RwLock<HashMap<String, AtomicU64>>,
}

impl Counters {
    /// Creates an empty set of counters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `delta` to the counter `name` (creating it at zero if needed).
    /// Returns the counter's new value. Wraps on overflow.
    pub fn add(&self, name: &str, delta: u64) -> u64 {
        {
            let map = self.map.read().unwrap_or_else(PoisonError::into_inner);
            if let Some(counter) = map.get(name) {
                return counter.fetch_add(delta, Ordering::Relaxed).wrapping_add(delta);
            }
        }
        let mut map = self.map.write().unwrap_or_else(PoisonError::into_inner);
        let counter = map.entry(name.to_owned()).or_default();
        counter.fetch_add(delta, Ordering::Relaxed).wrapping_add(delta)
    }

    /// Current value of `name`, or `0` if it has never been added to.
    pub fn get(&self, name: &str) -> u64 {
        let map = self.map.read().unwrap_or_else(PoisonError::into_inner);
        map.get(name).map_or(0, |c| c.load(Ordering::Relaxed))
    }

    /// A point-in-time copy of every counter, sorted by name.
    pub fn snapshot(&self) -> BTreeMap<String, u64> {
        let map = self.map.read().unwrap_or_else(PoisonError::into_inner);
        map.iter()
            .map(|(name, c)| (name.clone(), c.load(Ordering::Relaxed)))
            .collect()
    }
}

/// A cooperative shutdown signal.
///
/// `request_stop` uses `Release` and `is_stopped` uses `Acquire`, so anything
/// the requesting thread wrote before the request is visible to a thread that
/// observes it.
#[derive(Debug, Default)]
pub struct StopFlag {
    stopped: AtomicBool,
}

impl StopFlag {
    /// Creates a flag that is not yet set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the flag. Idempotent.
    pub fn request_stop(&self) {
        self.stopped.store(true, Ordering::Release);
    }

    /// Whether [`request_stop`](Self::request_stop) has been called.
    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
    }
}

/// A single-slot mailbox: one producer publishes, one consumer takes.
///
/// The `ready` flag is a lock-free fast path so polling loops do not contend
/// on the mutex while the slot is empty. The mutex carries the value itself.
#[derive(Debug)]
pub struct Handoff<T> {
    ready: AtomicBool,
    slot: Mutex<Option<T>>,
}

impl<T> Default for Handoff<T> {
    fn default() -> Self {
        Self {
            ready: AtomicBool::new(false),
            slot: Mutex::new(None),
        }
    }
}

impl<T> Handoff<T> {
    /// Creates an empty handoff.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores `value`, replacing any value not yet taken.
    pub fn publish(&self, value: T) {
        *self.slot.lock().unwrap_or_else(PoisonError::into_inner) = Some(value);
        self.ready.store(true, Ordering::Release);
    }

    /// Takes the published value, if any. Each value is returned at most once.
    pub fn try_take(&self) -> Option<T> {
        if !self.ready.load(Ordering::Acquire) {
            return None;
        }
        self.slot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }
}
