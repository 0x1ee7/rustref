//! Part 1: bounded TTL + LRU response cache with an injected clock.

use std::borrow::Borrow;
use std::hash::Hash;
use std::marker::PhantomData;
use std::time::Duration;

use crate::{Clock, Error, SystemClock};

/// A gateway response cache: bounded size, least-recently-used eviction, and
/// every entry expires `ttl` after it was written.
///
/// Engineer-owned: storage and bookkeeping. The `PhantomData` only keeps the
/// type parameters in use until you add real fields; remove it when you do.
pub struct TtlCache<K, V, C = SystemClock> {
    _marker: PhantomData<(K, V, C)>,
}

impl<K, V, C: Clock> TtlCache<K, V, C> {
    /// Creates an empty cache.
    ///
    /// # Errors
    /// [`Error::ZeroCapacity`] if `max_size == 0`; [`Error::ZeroTtl`] if `ttl` is zero.
    pub fn new(_max_size: usize, _ttl: Duration, _clock: C) -> Result<Self, Error> {
        todo!()
    }
}

impl<K: Clone + Hash + Eq, V, C: Clock> TtlCache<K, V, C> {
    /// Looks up a live entry. A hit becomes the most recently used entry; a
    /// missing or expired key is a miss (expired entries are removed). Counts
    /// the outcome in [`hits`](Self::hits) / [`misses`](Self::misses).
    /// Never extends the entry's TTL.
    pub fn get<Q>(&mut self, _key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        todo!()
    }

    /// Inserts or replaces. Replacing refreshes both TTL and recency. Expired
    /// entries are purged first; then least-recently-used entries are evicted
    /// until `len() <= max_size`.
    pub fn put(&mut self, _key: K, _value: V) {
        todo!()
    }

    /// Live keys, least recently used first. Does not change recency or counters.
    pub fn keys(&self) -> Vec<K> {
        todo!()
    }

    /// `true` iff the key is live. Does not change recency or counters.
    pub fn contains_key<Q>(&self, _key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        todo!()
    }

    /// Number of live (unexpired) entries.
    pub fn len(&self) -> usize {
        todo!()
    }

    /// `true` iff there are no live entries.
    pub fn is_empty(&self) -> bool {
        todo!()
    }

    /// Number of `get` calls that returned a live entry.
    pub fn hits(&self) -> u64 {
        todo!()
    }

    /// Number of `get` calls that returned nothing.
    pub fn misses(&self) -> u64 {
        todo!()
    }
}
