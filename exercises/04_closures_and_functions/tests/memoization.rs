//! Part 4: memoisation keyed by arguments, and a TTL + LRU cache with an
//! injected clock.

mod common;

use common::ManualClock;
use ex04_closures_and_functions::memo::*;
use std::cell::Cell;
use std::num::NonZeroUsize;
use std::panic::{self, AssertUnwindSafe};
use std::time::Duration;

#[test]
fn part4_memoized_calls_the_function_once_per_argument() {
    let calls = Cell::new(0);
    let mut square = Memoized::new(|x: u64| {
        calls.set(calls.get() + 1);
        x * x
    });
    assert_eq!(*square.get(3), 9);
    assert_eq!(*square.get(3), 9);
    assert_eq!(*square.get(4), 16);
    assert_eq!(calls.get(), 2, "the second get(3) must come from the cache");
    assert_eq!(square.stats(), CacheStats { hits: 1, misses: 2 });
}

#[test]
fn part4_memoized_keys_on_the_whole_argument_tuple() {
    let calls = Cell::new(0);
    let mut label = Memoized::new(|(name, version): (String, u32)| {
        calls.set(calls.get() + 1);
        format!("{name}@{version}")
    });
    assert_eq!(label.get(("api".to_string(), 1)), "api@1");
    assert_eq!(label.get(("api".to_string(), 2)), "api@2");
    assert_eq!(label.get(("api".to_string(), 1)), "api@1");
    assert_eq!(calls.get(), 2, "different versions are different keys");
}

#[test]
fn part4_memoized_hits_return_the_cached_value_in_place() {
    let mut square = Memoized::new(|x: u64| x * x);
    let first = square.get(6) as *const u64;
    let second = square.get(6) as *const u64;
    assert!(std::ptr::eq(first, second), "a hit references the stored value, it does not copy");
}

#[test]
fn part4_memoized_clear_drops_entries_and_counters() {
    let calls = Cell::new(0);
    let mut plus_one = Memoized::new(|x: u64| {
        calls.set(calls.get() + 1);
        x + 1
    });
    plus_one.get(1);
    plus_one.get(1);
    plus_one.clear();
    assert_eq!(plus_one.stats(), CacheStats::default(), "clear resets the counters");
    assert_eq!(*plus_one.get(1), 2);
    assert_eq!(calls.get(), 2, "after clear the function runs again");
}

#[test]
fn part4_memoized_panicking_call_caches_nothing() {
    let attempts = Cell::new(0);
    let mut flaky = Memoized::new(|x: u64| {
        attempts.set(attempts.get() + 1);
        if attempts.get() == 1 {
            panic!("first call fails");
        }
        x * 10
    });
    let first = panic::catch_unwind(AssertUnwindSafe(|| *flaky.get(2)));
    assert!(first.is_err(), "the panic propagates");
    assert_eq!(*flaky.get(2), 20, "the failed computation must not be cached");
    assert_eq!(attempts.get(), 2);
}

#[test]
fn part4_ttl_rejects_zero_ttl() {
    let clock = ManualClock::default();
    let err = TtlCache::new(Duration::ZERO, None, clock, |x: u32| x).err();
    assert_eq!(err, Some(CacheConfigError::ZeroTtl));
}

#[test]
fn part4_ttl_entry_expires_exactly_at_ttl() {
    let clock = ManualClock::default();
    let calls = Cell::new(0);
    let mut cache = TtlCache::new(
        Duration::from_secs(10),
        None,
        clock.clone(),
        |x: u32| {
            calls.set(calls.get() + 1);
            x * 2
        },
    )
    .unwrap();
    assert_eq!(*cache.get(1), 2);
    clock.advance(Duration::from_millis(9_999));
    assert_eq!(*cache.get(1), 2, "still fresh just before the ttl");
    clock.advance(Duration::from_millis(1));
    assert_eq!(*cache.get(1), 2, "at exactly the ttl the entry is recomputed");
    assert_eq!(calls.get(), 2);
    assert_eq!(cache.stats(), CacheStats { hits: 1, misses: 2 });
}

#[test]
fn part4_ttl_recomputes_expired_entries() {
    let clock = ManualClock::default();
    let calls = Cell::new(0u32);
    let mut cache = TtlCache::new(Duration::from_secs(1), None, clock.clone(), |_: u32| {
        calls.set(calls.get() + 1);
        calls.get()
    })
    .unwrap();
    assert_eq!(*cache.get(7), 1);
    clock.advance(Duration::from_secs(2));
    assert_eq!(*cache.get(7), 2, "the expired entry is replaced by a fresh computation");
}

#[test]
fn part4_ttl_evicts_least_recently_used_when_full() {
    let clock = ManualClock::default();
    let calls = Cell::new(0);
    let cap = NonZeroUsize::new(2).unwrap();
    let mut cache = TtlCache::new(Duration::from_secs(60), Some(cap), clock, |k: u32| {
        calls.set(calls.get() + 1);
        k * 10
    })
    .unwrap();
    cache.get(1);
    cache.get(2);
    cache.get(1); // touch 1, so 2 is now least recently used
    cache.get(3); // evicts 2
    assert_eq!(cache.len(), 2);

    let before = calls.get();
    cache.get(1);
    assert_eq!(calls.get(), before, "1 was touched and must survive eviction");
    cache.get(2);
    assert_eq!(calls.get(), before + 1, "2 was evicted and must be recomputed");
}

#[test]
fn part4_ttl_unbounded_keeps_every_key_until_clear() {
    let clock = ManualClock::default();
    let mut cache = TtlCache::new(Duration::from_secs(60), None, clock, |x: u32| x).unwrap();
    for k in 0..1000 {
        cache.get(k);
    }
    assert_eq!(cache.len(), 1000, "an unbounded cache keeps every key");
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.stats(), CacheStats::default());
}
