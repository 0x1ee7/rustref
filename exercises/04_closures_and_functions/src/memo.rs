//! Part 4: memoisation keyed by arguments, and a TTL + LRU cache with an
//! injected clock.

use crate::Clock;
use std::collections::HashMap;
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::time::Duration;
use thiserror::Error;

/// Hit and miss counters for a cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CacheStats {
    /// Lookups answered from the cache.
    pub hits: u64,
    /// Lookups that ran the underlying function.
    pub misses: u64,
}

/// Invalid cache configuration.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum CacheConfigError {
    /// A zero TTL would never serve a hit.
    #[error("ttl must be greater than zero")]
    ZeroTtl,
}

/// Unbounded memoisation of a function of one argument (possibly a tuple).
///
/// Arguments are cloned into the map as keys. The function must be `FnMut`
/// because it may keep state of its own.
#[allow(dead_code)] // stub: remove once the fields are used
pub struct Memoized<A, R, F> {
    cache: HashMap<A, R>,
    f: F,
    stats: CacheStats,
}

impl<A, R, F> Memoized<A, R, F>
where
    A: Hash + Eq + Clone,
    F: FnMut(A) -> R,
{
    /// Wraps `f` with an empty cache.
    pub fn new(_f: F) -> Self {
        todo!()
    }

    /// Returns the cached value for `arg`, computing and storing it on a miss.
    ///
    /// The reference borrows the cache, so it is valid until the next `&mut` call.
    pub fn get(&mut self, _arg: A) -> &R {
        todo!()
    }

    /// Current hit and miss counters.
    pub fn stats(&self) -> CacheStats {
        todo!()
    }

    /// Drops all entries and resets the counters.
    pub fn clear(&mut self) {
        todo!()
    }
}

/// One cached result with its storage time and recency tick.
#[allow(dead_code)] // stub: remove once the fields are used
struct Slot<R> {
    value: R,
    stored_at: Duration,
    last_used: u64,
}

/// Memoisation with per-entry time-to-live and an optional capacity bound.
///
/// An entry is fresh while `now - stored_at < ttl`. Once capacity is reached,
/// inserting a new key evicts the least recently used entry. A hit counts as a use.
#[allow(dead_code)] // stub: remove once the fields are used
pub struct TtlCache<A, R, F, C> {
    slots: HashMap<A, Slot<R>>,
    ttl: Duration,
    max_entries: Option<NonZeroUsize>,
    clock: C,
    f: F,
    stats: CacheStats,
    tick: u64,
}

impl<A, R, F, C> TtlCache<A, R, F, C>
where
    A: Hash + Eq + Clone,
    F: FnMut(A) -> R,
    C: Clock,
{
    /// Creates a cache. `max_entries == None` means unbounded.
    ///
    /// Returns `ZeroTtl` if `ttl` is zero.
    pub fn new(
        _ttl: Duration,
        _max_entries: Option<NonZeroUsize>,
        _clock: C,
        _f: F,
    ) -> Result<Self, CacheConfigError> {
        todo!()
    }

    /// Returns a fresh cached value for `arg`, or recomputes it on a miss or expiry.
    pub fn get(&mut self, _arg: A) -> &R {
        todo!()
    }

    /// Current hit and miss counters.
    pub fn stats(&self) -> CacheStats {
        todo!()
    }

    /// Number of stored entries, including expired ones not yet replaced.
    pub fn len(&self) -> usize {
        todo!()
    }

    /// `true` when no entries are stored.
    pub fn is_empty(&self) -> bool {
        todo!()
    }

    /// Drops all entries and resets the counters.
    pub fn clear(&mut self) {
        todo!()
    }
}
