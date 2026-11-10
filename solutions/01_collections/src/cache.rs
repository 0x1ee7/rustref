//! Part 1 reference: bounded TTL + LRU cache.
//!
//! Two indexes over the same entries: a `HashMap` for lookup and a `BTreeMap`
//! from a monotonically increasing tick to the key, so the least recently used
//! entry is `recency.first_key_value()`. Every hit re-keys the entry to a new tick.

use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::time::{Duration, Instant};

use crate::{Clock, Error, SystemClock};

struct Entry<V> {
    value: V,
    /// `None` when `now + ttl` overflows `Instant`: the entry never expires.
    expires_at: Option<Instant>,
    tick: u64,
}

fn is_expired(expires_at: Option<Instant>, now: Instant) -> bool {
    expires_at.is_some_and(|at| now >= at)
}

/// A gateway response cache: bounded size, least-recently-used eviction, and
/// every entry expires `ttl` after it was written.
pub struct TtlCache<K, V, C = SystemClock> {
    ttl: Duration,
    max_size: usize,
    clock: C,
    entries: HashMap<K, Entry<V>>,
    /// Recency index: tick -> key. The smallest tick is the least recently used.
    recency: BTreeMap<u64, K>,
    next_tick: u64,
    hits: u64,
    misses: u64,
}

impl<K, V, C: Clock> TtlCache<K, V, C> {
    /// Creates an empty cache.
    ///
    /// # Errors
    /// [`Error::ZeroCapacity`] if `max_size == 0`; [`Error::ZeroTtl`] if `ttl` is zero.
    pub fn new(max_size: usize, ttl: Duration, clock: C) -> Result<Self, Error> {
        if max_size == 0 {
            return Err(Error::ZeroCapacity);
        }
        if ttl.is_zero() {
            return Err(Error::ZeroTtl);
        }
        Ok(Self {
            ttl,
            max_size,
            clock,
            entries: HashMap::new(),
            recency: BTreeMap::new(),
            next_tick: 0,
            hits: 0,
            misses: 0,
        })
    }
}

impl<K: Clone + Hash + Eq, V, C: Clock> TtlCache<K, V, C> {
    /// Looks up a live entry; see the stub for the full contract.
    pub fn get<Q>(&mut self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let now = self.clock.now();
        let Some((stored, entry)) = self.entries.get_key_value(key) else {
            self.misses += 1;
            return None;
        };
        if is_expired(entry.expires_at, now) {
            self.remove(key);
            self.misses += 1;
            return None;
        }
        let (old_tick, stored) = (entry.tick, stored.clone());
        let tick = self.next_tick;
        self.next_tick += 1;
        self.recency.remove(&old_tick);
        self.recency.insert(tick, stored);
        self.hits += 1;
        let entry = self.entries.get_mut(key)?;
        entry.tick = tick;
        Some(&entry.value)
    }

    /// Inserts or replaces; purges expired entries, then evicts LRU to capacity.
    pub fn put(&mut self, key: K, value: V) {
        let now = self.clock.now();
        self.purge_expired(now);
        self.remove(&key);
        let tick = self.next_tick;
        self.next_tick += 1;
        self.recency.insert(tick, key.clone());
        self.entries.insert(
            key,
            Entry {
                value,
                expires_at: now.checked_add(self.ttl),
                tick,
            },
        );
        while self.entries.len() > self.max_size {
            let Some((_, lru)) = self.recency.pop_first() else {
                break;
            };
            self.entries.remove(&lru);
        }
    }

    /// Live keys, least recently used first.
    pub fn keys(&self) -> Vec<K> {
        let now = self.clock.now();
        self.recency
            .values()
            .filter(|k| {
                self.entries
                    .get(*k)
                    .is_some_and(|e| !is_expired(e.expires_at, now))
            })
            .cloned()
            .collect()
    }

    /// `true` iff the key is live.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let now = self.clock.now();
        self.entries
            .get(key)
            .is_some_and(|e| !is_expired(e.expires_at, now))
    }

    /// Number of live (unexpired) entries.
    pub fn len(&self) -> usize {
        let now = self.clock.now();
        self.entries
            .values()
            .filter(|e| !is_expired(e.expires_at, now))
            .count()
    }

    /// `true` iff there are no live entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Number of `get` calls that returned a live entry.
    pub fn hits(&self) -> u64 {
        self.hits
    }

    /// Number of `get` calls that returned nothing.
    pub fn misses(&self) -> u64 {
        self.misses
    }

    fn remove<Q>(&mut self, key: &Q) -> Option<Entry<V>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let entry = self.entries.remove(key)?;
        self.recency.remove(&entry.tick);
        Some(entry)
    }

    fn purge_expired(&mut self, now: Instant) {
        let recency = &mut self.recency;
        self.entries.retain(|_, e| {
            if is_expired(e.expires_at, now) {
                recency.remove(&e.tick);
                false
            } else {
                true
            }
        });
    }
}
