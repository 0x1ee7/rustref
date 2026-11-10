//! Caches: a single-threaded LRU, and a thread-safe memoizing loader that
//! collapses concurrent loads of the same key into one.

use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};

struct Entry<V> {
    value: V,
    /// Logical time of the last access; key into `by_age`.
    tick: u64,
}

/// Least-recently-used cache with a fixed capacity.
///
/// Not thread-safe by itself; share it behind a `Mutex`.
pub struct LruCache<K, V> {
    capacity: usize,
    clock: u64,
    entries: HashMap<K, Entry<V>>,
    /// Access time -> key, oldest first. Mirrors `entries` exactly.
    by_age: BTreeMap<u64, K>,
}

impl<K, V> LruCache<K, V> {
    /// Creates an empty cache holding at most `capacity` entries.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is zero.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity >= 1, "capacity must be >= 1");
        Self {
            capacity,
            clock: 0,
            entries: HashMap::new(),
            by_age: BTreeMap::new(),
        }
    }

    /// Number of entries currently stored.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` when the cache holds no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<K: Eq + Hash + Clone, V> LruCache<K, V> {
    /// Looks up `key`, marking it most recently used on a hit.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        let entry = self.entries.get_mut(key)?;
        self.clock += 1;
        self.by_age.remove(&entry.tick);
        entry.tick = self.clock;
        self.by_age.insert(self.clock, key.clone());
        Some(&entry.value)
    }

    /// Inserts or replaces `key`. A new key evicts the least recently used
    /// entry when the cache is full. Either way the key becomes most recent.
    pub fn insert(&mut self, key: K, value: V) {
        self.clock += 1;
        let tick = self.clock;
        if let Some(old) = self.entries.remove(&key) {
            self.by_age.remove(&old.tick);
        } else if self.entries.len() == self.capacity {
            self.evict_oldest();
        }
        self.by_age.insert(tick, key.clone());
        self.entries.insert(key, Entry { value, tick });
    }

    /// Removes the least recently used entry.
    fn evict_oldest(&mut self) {
        if let Some((_, victim)) = self.by_age.pop_first() {
            self.entries.remove(&victim);
        }
    }
}

/// Outcome slot shared between the leader of a load and its waiters.
enum Outcome<V, E> {
    Pending,
    Done(Result<V, E>),
    /// The leader panicked; waiters must try again.
    Abandoned,
}

struct Call<V, E> {
    outcome: Mutex<Outcome<V, E>>,
    settled: Condvar,
}

impl<V: Clone, E: Clone> Call<V, E> {
    fn new() -> Self {
        Self {
            outcome: Mutex::new(Outcome::Pending),
            settled: Condvar::new(),
        }
    }

    fn settle(&self, outcome: Outcome<V, E>) {
        *self.outcome.lock().unwrap_or_else(PoisonError::into_inner) = outcome;
        self.settled.notify_all();
    }

    /// Blocks until the leader settles. `None` means the leader abandoned.
    fn wait(&self) -> Option<Result<V, E>> {
        let guard = self.outcome.lock().unwrap_or_else(PoisonError::into_inner);
        let guard = self
            .settled
            .wait_while(guard, |o| matches!(o, Outcome::Pending))
            .unwrap_or_else(PoisonError::into_inner);
        match &*guard {
            Outcome::Done(result) => Some(result.clone()),
            Outcome::Abandoned => None,
            Outcome::Pending => unreachable!("wait_while returned while pending"),
        }
    }
}

/// Memoizing single-flight loader.
///
/// Concurrent calls for the same key share one execution of the loader; the
/// others block and receive a clone of its result. Successful values are kept,
/// so each key is loaded at most once. Errors are shared with the callers that
/// were waiting but are *not* stored, so the next call retries the load.
///
/// If the loader panics, the panic propagates to its caller and waiters retry.
pub struct SingleFlight<K, V, E> {
    state: Mutex<State<K, V, E>>,
    loads: AtomicUsize,
}

struct State<K, V, E> {
    done: HashMap<K, V>,
    pending: HashMap<K, Arc<Call<V, E>>>,
}

impl<K, V, E> SingleFlight<K, V, E> {
    /// Creates an empty loader.
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                done: HashMap::new(),
                pending: HashMap::new(),
            }),
            loads: AtomicUsize::new(0),
        }
    }

    /// How many times a loader has actually been executed.
    pub fn load_count(&self) -> usize {
        self.loads.load(Ordering::Relaxed)
    }
}

impl<K, V, E> Default for SingleFlight<K, V, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, E> SingleFlight<K, V, E>
where
    K: Eq + Hash + Clone + Send,
    V: Clone + Send,
    E: Clone + Send,
{
    /// Returns the value for `key`, running `load` only if no value is known and
    /// no other thread is already loading it.
    pub fn get_or_load(&self, key: K, load: impl FnOnce() -> Result<V, E>) -> Result<V, E> {
        let mut load = Some(load);
        loop {
            let call = {
                let mut state = self.lock();
                if let Some(value) = state.done.get(&key) {
                    return Ok(value.clone());
                }
                if let Some(call) = state.pending.get(&key) {
                    Arc::clone(call)
                } else {
                    let call = Arc::new(Call::new());
                    state.pending.insert(key.clone(), Arc::clone(&call));
                    drop(state);
                    let load = load.take().expect("only one leader per call");
                    return self.lead(key, &call, load);
                }
            };
            if let Some(result) = call.wait() {
                return result;
            }
            // The leader panicked; loop and try to become the leader ourselves.
        }
    }

    fn lead(
        &self,
        key: K,
        call: &Arc<Call<V, E>>,
        load: impl FnOnce() -> Result<V, E>,
    ) -> Result<V, E> {
        self.loads.fetch_add(1, Ordering::Relaxed);
        let result = match panic::catch_unwind(AssertUnwindSafe(load)) {
            Ok(result) => result,
            Err(payload) => {
                self.lock().pending.remove(&key);
                call.settle(Outcome::Abandoned);
                panic::resume_unwind(payload);
            }
        };
        {
            let mut state = self.lock();
            state.pending.remove(&key);
            if let Ok(value) = &result {
                state.done.insert(key, value.clone());
            }
        }
        call.settle(Outcome::Done(result.clone()));
        result
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State<K, V, E>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
