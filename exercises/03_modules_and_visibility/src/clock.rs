//! A clock with a thread-local, scoped override for tests.
//!
//! Overrides are per thread: a spawned thread does not inherit them.

use std::marker::PhantomData;

/// Current time in whole seconds since the Unix epoch. Returns the innermost
/// active override on this thread, or the system clock if none is active.
#[must_use]
pub fn now_secs() -> u64 {
    todo!()
}

/// Number of active overrides on the current thread.
#[must_use]
pub fn override_depth() -> usize {
    todo!()
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
        let _ = at;
        todo!()
    }
}
