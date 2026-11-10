//! A downloader that never runs more than `max_concurrent` fetches at once.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;

/// Upper bound on helper threads spawned by [`BoundedDownloader::download_all`].
const MAX_HELPERS: usize = 16;

/// Per-URL results, indexed by input position, filled in by helper threads.
type Slots<E> = Mutex<Vec<Option<Result<Vec<u8>, E>>>>;

/// A counting semaphore built from `Mutex` + `Condvar`.
#[derive(Debug)]
struct Gate {
    limit: usize,
    state: Mutex<GateState>,
    freed: Condvar,
}

#[derive(Debug, Default)]
struct GateState {
    active: usize,
    peak: usize,
}

impl Gate {
    fn new(limit: usize) -> Self {
        Self {
            limit,
            state: Mutex::new(GateState::default()),
            freed: Condvar::new(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, GateState> {
        // The guarded state is plain counters; it is always consistent, so a
        // poisoned lock is safe to recover from.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn acquire(&self) -> Permit<'_> {
        let state = self.lock();
        let mut state = self
            .freed
            .wait_while(state, |s| s.active >= self.limit)
            .unwrap_or_else(PoisonError::into_inner);
        state.active += 1;
        state.peak = state.peak.max(state.active);
        Permit { gate: self }
    }
}

/// RAII token: releases its slot on drop, including during unwinding.
struct Permit<'a> {
    gate: &'a Gate,
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        self.gate.lock().active -= 1;
        self.gate.freed.notify_one();
    }
}

/// Runs a blocking `fetch` function with at most `max_concurrent` calls in flight.
pub struct BoundedDownloader<F> {
    fetch: F,
    gate: Gate,
}

impl<F, E> BoundedDownloader<F>
where
    F: Fn(&str) -> Result<Vec<u8>, E> + Sync,
    E: Send,
{
    /// Creates a downloader allowing at most `max_concurrent` simultaneous fetches.
    ///
    /// # Panics
    ///
    /// Panics if `max_concurrent` is zero.
    pub fn new(fetch: F, max_concurrent: usize) -> Self {
        assert!(max_concurrent >= 1, "max_concurrent must be >= 1");
        Self {
            fetch,
            gate: Gate::new(max_concurrent),
        }
    }

    /// Fetches one URL, waiting for a free slot first.
    pub fn download(&self, url: &str) -> Result<Vec<u8>, E> {
        let _permit = self.gate.acquire();
        (self.fetch)(url)
    }

    /// Fetches every URL and returns the bodies in input order.
    ///
    /// Every URL is attempted even if some fail. If any failed, the error of
    /// the earliest failing URL (by input index) is returned.
    pub fn download_all(&self, urls: &[String]) -> Result<Vec<Vec<u8>>, E> {
        let next = AtomicUsize::new(0);
        let slots: Slots<E> = Mutex::new((0..urls.len()).map(|_| None).collect());
        let helpers = urls.len().min(MAX_HELPERS);

        thread::scope(|s| {
            for _ in 0..helpers {
                s.spawn(|| {
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(url) = urls.get(index) else { break };
                        let result = self.download(url);
                        let mut guard = slots.lock().unwrap_or_else(PoisonError::into_inner);
                        guard[index] = Some(result);
                    }
                });
            }
        });

        slots
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner)
            .into_iter()
            .map(|slot| slot.expect("every index is filled exactly once"))
            .collect()
    }

    /// Number of fetches running right now.
    pub fn in_flight(&self) -> usize {
        self.gate.lock().active
    }

    /// Highest number of simultaneous fetches observed so far.
    pub fn peak_concurrency(&self) -> usize {
        self.gate.lock().peak
    }
}
