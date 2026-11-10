//! Part 3: `Vec` allocation discipline: sized outputs, `retain` vs collect-into-new.
mod common;

use common::count_allocations;
use ex19_performance::vectors::{Entry, live_entries, prune_expired, widen_all};

fn sample_entries() -> Vec<Entry> {
    [(1, 50), (2, 100), (3, 101), (4, 10), (5, 200)]
        .into_iter()
        .map(|(id, expires_at)| Entry { id, expires_at })
        .collect()
}

fn ids(entries: &[Entry]) -> Vec<u32> {
    entries.iter().map(|e| e.id).collect()
}

#[test]
fn part3_widen_all_allocates_exactly_once() {
    let src: Vec<u32> = (0..1000).collect();
    let (out, allocs) = count_allocations(|| widen_all(&src));
    assert_eq!(allocs, 1, "a sized output for a non-empty input is one allocation");
    assert_eq!(out.len(), 1000);
    assert_eq!(out[999], 999u64);
}

#[test]
fn part3_widen_all_on_empty_input_allocates_nothing() {
    let (out, allocs) = count_allocations(|| widen_all(&[]));
    assert!(out.is_empty());
    assert_eq!(allocs, 0, "an empty input must not allocate");
}

#[test]
fn part3_widen_all_preserves_order_and_extremes() {
    assert_eq!(widen_all(&[u32::MAX, 0, 7]), vec![u64::from(u32::MAX), 0, 7]);
}

#[test]
fn part3_prune_expired_removes_in_place_keeping_order_and_capacity() {
    let mut entries = Vec::with_capacity(64);
    entries.extend(sample_entries());
    let capacity = entries.capacity();

    let (removed, allocs) = count_allocations(|| prune_expired(&mut entries, 100));

    assert_eq!(removed, 3, "expires_at 50, 100 and 10 are expired at now=100");
    assert_eq!(ids(&entries), vec![3, 5], "survivors keep their original order");
    assert_eq!(entries.capacity(), capacity, "pruning must not shrink the vector");
    assert_eq!(allocs, 0, "retain works in place and must not allocate");
}

#[test]
fn part3_prune_with_nothing_expired_is_a_no_op() {
    let mut entries = sample_entries();
    let before = entries.clone();
    let (removed, allocs) = count_allocations(|| prune_expired(&mut entries, 0));
    assert_eq!(removed, 0);
    assert_eq!(entries, before);
    assert_eq!(allocs, 0);
}

#[test]
fn part3_live_entries_collects_a_new_vec_and_leaves_input_alone() {
    let source = sample_entries();
    let (live, allocs) = count_allocations(|| live_entries(&source, 100));
    assert_eq!(ids(&live), vec![3, 5]);
    assert_eq!(source.len(), 5, "the input must not be modified");
    assert!(allocs >= 1, "collecting into a new Vec must allocate, got {allocs}");
}

#[test]
fn part3_retain_and_collect_agree_on_randomised_inputs() {
    // Deterministic xorshift so failures reproduce exactly.
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for round in 0..200 {
        let n = (next() % 64) as usize;
        let now = next() % 1000;
        let input: Vec<Entry> = (0..n)
            .map(|i| Entry {
                id: i as u32,
                expires_at: next() % 1000,
            })
            .collect();

        let expected = live_entries(&input, now);
        let mut pruned = input.clone();
        let removed = prune_expired(&mut pruned, now);

        assert_eq!(pruned, expected, "round {round}: retain and collect must agree");
        assert_eq!(removed, n - expected.len(), "round {round}: removed count");
    }
}
