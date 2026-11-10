//! Part 4: memoisation keyed by arguments, and a TTL + LRU cache with an
//! injected clock.

use crate::Clock;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
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
    pub fn new(f: F) -> Self {
        Self {
            cache: HashMap::new(),
            f,
            stats: CacheStats::default(),
        }
    }

    /// Returns the cached value for `arg`, computing and storing it on a miss.
    ///
    /// The reference borrows the cache, so it is valid until the next `&mut` call.
    pub fn get(&mut self, arg: A) -> &R {
        let Self { cache, f, stats } = self;
        match cache.entry(arg) {
            Entry::Occupied(slot) => {
                stats.hits += 1;
                slot.into_mut()
            }
            Entry::Vacant(slot) => {
                // Counted before the call so a panicking `f` still registers a miss.
                // The vacant entry is dropped on panic, so nothing is cached.
                stats.misses += 1;
                let value = f(slot.key().clone());
                slot.insert(value)
            }
        }
    }

    /// Current hit and miss counters.
    pub fn stats(&self) -> CacheStats {
        self.stats
    }

    /// Drops all entries and resets the counters.
    pub fn clear(&mut self) {
        self.cache.clear();
        self.stats = CacheStats::default();
    }
}

/// One cached result with its storage time and recency tick.
struct Slot<R> {
    value: R,
    stored_at: Duration,
    last_used: u64,
}

/// Memoisation with per-entry time-to-live and an optional capacity bound.
///
/// An entry is fresh while `now - stored_at < ttl`. Once capacity is reached,
/// inserting a new key evicts the least recently used entry. A hit counts as a use.
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
        ttl: Duration,
        max_entries: Option<NonZeroUsize>,
        clock: C,
        f: F,
    ) -> Result<Self, CacheConfigError> {
        if ttl.is_zero() {
            return Err(CacheConfigError::ZeroTtl);
        }
        Ok(Self {
            slots: HashMap::new(),
            ttl,
            max_entries,
            clock,
            f,
            stats: CacheStats::default(),
            tick: 0,
        })
    }

    /// Returns a fresh cached value for `arg`, or recomputes it on a miss or expiry.
    pub fn get(&mut self, arg: A) -> &R {
        let now = self.clock.now();
        self.tick += 1;
        let tick = self.tick;

        let fresh = self
            .slots
            .get(&arg)
            .is_some_and(|slot| now.saturating_sub(slot.stored_at) < self.ttl);
        if fresh {
            self.stats.hits += 1;
            let slot = self.slots.get_mut(&arg).expect("fresh entry exists");
            slot.last_used = tick;
            return &slot.value;
        }

        self.stats.misses += 1;
        // Compute before touching the map, so a panicking `f` leaves the cache unchanged.
        let value = (self.f)(arg.clone());
        if !self.slots.contains_key(&arg) {
            self.evict_if_full();
        }
        let slot = self.slots.entry(arg).insert_entry(Slot {
            value,
            stored_at: now,
            last_used: tick,
        });
        &slot.into_mut().value
    }

    /// Current hit and miss counters.
    pub fn stats(&self) -> CacheStats {
        self.stats
    }

    /// Number of stored entries, including expired ones not yet replaced.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// `true` when no entries are stored.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Drops all entries and resets the counters.
    pub fn clear(&mut self) {
        self.slots.clear();
        self.stats = CacheStats::default();
        self.tick = 0;
    }

    /// Removes the least recently used entry if the cache is at capacity.
    fn evict_if_full(&mut self) {
        let Some(capacity) = self.max_entries else {
            return;
        };
        if self.slots.len() < capacity.get() {
            return;
        }
        let victim = self
            .slots
            .iter()
            .min_by_key(|(_, slot)| slot.last_used)
            .map(|(key, _)| key.clone());
        if let Some(key) = victim {
            self.slots.remove(&key);
        }
    }
}
