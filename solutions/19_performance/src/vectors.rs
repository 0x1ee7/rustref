//! `Vec` allocation discipline: sized outputs, in-place pruning, collect-into-new.

/// A cache entry that expires at `expires_at` (same clock as `now`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub id: u32,
    pub expires_at: u64,
}

/// Widens every value to `u64` with a single, correctly sized allocation.
pub fn widen_all(src: &[u32]) -> Vec<u64> {
    let mut out = Vec::with_capacity(src.len());
    out.extend(src.iter().map(|&v| u64::from(v)));
    out
}

/// Removes expired entries in place. Returns how many were removed.
pub fn prune_expired(entries: &mut Vec<Entry>, now: u64) -> usize {
    let before = entries.len();
    entries.retain(|e| e.expires_at > now);
    before - entries.len()
}

/// Collects the live entries into a new vector.
pub fn live_entries(entries: &[Entry], now: u64) -> Vec<Entry> {
    entries
        .iter()
        .filter(|e| e.expires_at > now)
        .copied()
        .collect()
}
