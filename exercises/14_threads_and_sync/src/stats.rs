//! Part 3: atomics and shared state without a lock per operation.

use std::collections::BTreeMap;

/// Named monotonic counters shared between threads.
///
/// Requirements:
/// - `add` creates missing counters at zero, returns the new value, and is
///   safe to call concurrently from many threads without lost updates;
/// - `get` of an unknown name returns `0`;
/// - `snapshot` returns a copy sorted by name that later updates do not change;
/// - the type is `Send + Sync`.
#[derive(Debug, Default)]
pub struct Counters {
    // Private: choose the representation (hint: `RwLock` for the map,
    // atomics for the values).
}

impl Counters {
    /// Creates an empty set of counters.
    pub fn new() -> Self {
        todo!()
    }

    /// Adds `delta` to the counter `name`; returns the new value.
    pub fn add(&self, name: &str, delta: u64) -> u64 {
        let _ = (name, delta);
        todo!()
    }

    /// Current value of `name`, or `0` if unknown.
    pub fn get(&self, name: &str) -> u64 {
        let _ = name;
        todo!()
    }

    /// A point-in-time copy of every counter, sorted by name.
    pub fn snapshot(&self) -> BTreeMap<String, u64> {
        todo!()
    }
}

/// A cooperative shutdown signal shared between threads.
///
/// Requirements: starts unset; `request_stop` is idempotent; once set, every
/// thread calling `is_stopped` eventually observes `true`.
#[derive(Debug, Default)]
pub struct StopFlag {
    // Private representation.
}

impl StopFlag {
    /// Creates a flag that is not yet set.
    pub fn new() -> Self {
        todo!()
    }

    /// Sets the flag.
    pub fn request_stop(&self) {
        todo!()
    }

    /// Whether the flag has been set.
    pub fn is_stopped(&self) -> bool {
        todo!()
    }
}

/// A single-slot mailbox: one producer publishes, one consumer takes.
///
/// Requirements: `try_take` returns `None` before any publish; after
/// `publish(v)`, `try_take` returns `Some(v)` exactly once; a second publish
/// before a take replaces the value.
#[derive(Debug)]
pub struct Handoff<T> {
    _value: std::marker::PhantomData<T>,
}

impl<T> Handoff<T> {
    /// Creates an empty handoff.
    pub fn new() -> Self {
        todo!()
    }

    /// Stores `value`, replacing any value not yet taken.
    pub fn publish(&self, value: T) {
        let _ = value;
        todo!()
    }

    /// Takes the published value, if any.
    pub fn try_take(&self) -> Option<T> {
        todo!()
    }
}
