//! Part 5: a least-recently-used cache and a single-flight loader.

use std::marker::PhantomData;

/// Least-recently-used cache with a fixed capacity.
///
/// Requirements:
/// - `new(0)` panics with a message mentioning `capacity`;
/// - `get` marks a hit as most recently used;
/// - `insert` of a new key into a full cache evicts the least recently used
///   entry (the one not touched for longest, counting `get` and `insert` as touches);
/// - `insert` of an existing key replaces its value and does not evict;
/// - `len` never exceeds `capacity`.
pub struct LruCache<K, V> {
    _marker: PhantomData<(K, V)>,
}

impl<K, V> LruCache<K, V> {
    /// Creates an empty cache holding at most `capacity` entries.
    pub fn new(capacity: usize) -> Self {
        let _ = capacity;
        todo!()
    }

    /// Number of entries currently stored.
    pub fn len(&self) -> usize {
        todo!()
    }

    /// `true` when the cache holds no entries.
    pub fn is_empty(&self) -> bool {
        todo!()
    }
}

impl<K, V> LruCache<K, V> {
    /// Looks up `key`, marking it most recently used on a hit.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        let _ = key;
        todo!()
    }

    /// Inserts or replaces `key`.
    pub fn insert(&mut self, key: K, value: V) {
        let _ = (key, value);
        todo!()
    }
}

/// Memoizing single-flight loader shared between threads.
///
/// Requirements:
/// - concurrent `get_or_load` calls for one key run the loader exactly once and
///   all receive its value (or error);
/// - successful values are kept: a later call for the same key does not run the loader;
/// - errors are shared with callers already waiting but are not kept: the next
///   call runs the loader again;
/// - a loader panic propagates to its caller, and waiting callers retry;
/// - `load_count` counts loader executions only.
pub struct SingleFlight<K, V, E> {
    _marker: PhantomData<(K, V, E)>,
}

impl<K, V, E> SingleFlight<K, V, E> {
    /// Creates an empty loader.
    pub fn new() -> Self {
        todo!()
    }

    /// How many times a loader has actually been executed.
    pub fn load_count(&self) -> usize {
        todo!()
    }
}

impl<K, V, E> SingleFlight<K, V, E>
where
    K: Eq + std::hash::Hash + Clone + Send,
    V: Clone + Send,
    E: Clone + Send,
{
    /// Returns the value for `key`, running `load` only if needed.
    pub fn get_or_load(&self, key: K, load: impl FnOnce() -> Result<V, E>) -> Result<V, E> {
        let _ = (key, load);
        todo!()
    }
}
