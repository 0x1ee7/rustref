//! A clock with a thread-local, scoped override for tests.
//!
//! Overrides are per thread: a spawned thread does not inherit them.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::time::{SystemTime, UNIX_EPOCH};

thread_local! {
    static OVERRIDES: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
}

/// Current time in whole seconds since the Unix epoch. Returns the innermost
/// active override on this thread, or the system clock if none is active.
#[must_use]
pub fn now_secs() -> u64 {
    OVERRIDES
        .with_borrow(|stack| stack.last().copied())
        .unwrap_or_else(system_secs)
}

fn system_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// Number of active overrides on the current thread.
#[must_use]
pub fn override_depth() -> usize {
    OVERRIDES.with_borrow(Vec::len)
}

/// RAII guard. While alive, [`now_secs`] returns the frozen value on this
/// thread. Dropping it restores the previous state. Guards nest, and must drop
/// in reverse creation order.
///
/// The guard is `!Send`: it must be dropped on the thread that created it.
#[derive(Debug)]
pub struct ClockOverride {
    _not_send: PhantomData<*const ()>,
}

impl ClockOverride {
    /// Freezes the clock on this thread at `at` seconds.
    #[must_use]
    pub fn freeze(at: u64) -> Self {
        OVERRIDES.with_borrow_mut(|stack| stack.push(at));
        Self {
            _not_send: PhantomData,
        }
    }
}

impl Drop for ClockOverride {
    fn drop(&mut self) {
        OVERRIDES.with_borrow_mut(|stack| {
            stack.pop();
        });
    }
}
