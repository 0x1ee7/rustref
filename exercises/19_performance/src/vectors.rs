//! `Vec` allocation discipline: sized outputs, in-place pruning, collect-into-new.

/// A cache entry that expires at `expires_at` (same clock as `now`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub id: u32,
    pub expires_at: u64,
}

/// Widens every value to `u64`. Exactly one allocation for a non-empty input.
pub fn widen_all(_src: &[u32]) -> Vec<u64> {
    todo!()
}

/// Removes entries with `expires_at <= now` in place, keeping order and capacity.
/// Returns how many entries were removed. Allocates nothing.
pub fn prune_expired(_entries: &mut Vec<Entry>, _now: u64) -> usize {
    todo!()
}

/// Returns a new vector with the entries where `expires_at > now`, in order.
/// The input is not modified.
pub fn live_entries(_entries: &[Entry], _now: u64) -> Vec<Entry> {
    todo!()
}
