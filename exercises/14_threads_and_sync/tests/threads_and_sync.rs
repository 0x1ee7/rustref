//! Black-box tests for ex14: threads, scoped threads and synchronisation.
//!
//! Every blocking interaction runs under a watchdog (`within`, `recv`,
//! `spin_until`) so a deadlock fails the test instead of hanging the run.

use std::any::Any;
use std::collections::{BTreeMap, HashSet};
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use ex14_threads_and_sync::bank::{self, Account, BankError};
use ex14_threads_and_sync::cache::{LruCache, SingleFlight};
use ex14_threads_and_sync::downloader::BoundedDownloader;
use ex14_threads_and_sync::link::{self, Outcome};
use ex14_threads_and_sync::parallel::parallel_map;
use ex14_threads_and_sync::pool::{PoolStats, QueueFull, ThreadPool};
use ex14_threads_and_sync::scheduler::{BoxError, JobError, ScheduleError, Scheduler};
use ex14_threads_and_sync::stats::{Counters, Handoff, StopFlag};

/// Upper bound for any single blocking step.
const LIMIT: Duration = Duration::from_secs(10);

/// Runs `f` on a helper thread and fails if it does not finish within `LIMIT`.
/// Panics inside `f` are re-raised on the test thread with their payload.
fn within<T: Send + 'static>(what: &str, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let outcome = panic::catch_unwind(AssertUnwindSafe(f));
        let _ = tx.send(outcome);
    });
    match rx.recv_timeout(LIMIT) {
        Ok(Ok(value)) => value,
        Ok(Err(payload)) => panic::resume_unwind(payload),
        Err(_) => panic!("{what} did not finish within {LIMIT:?} (deadlock?)"),
    }
}

/// Polls `cond` until it holds, failing after `LIMIT`.
fn spin_until(what: &str, mut cond: impl FnMut() -> bool) {
    let deadline = Instant::now() + LIMIT;
    while !cond() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        thread::yield_now();
    }
}

/// Receives one message, failing after `LIMIT`.
fn recv<T>(rx: &Receiver<T>, what: &str) -> T {
    rx.recv_timeout(LIMIT)
        .unwrap_or_else(|e| panic!("no message for {what}: {e:?}"))
}

fn panic_text(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "<non-string panic payload>".to_owned())
}

/// Runs `f`, which must panic, and returns the panic message.
fn panic_message<R>(f: impl FnOnce() -> R) -> String {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(_) => panic!("expected a panic, but the call returned normally"),
        Err(payload) => panic_text(&*payload),
    }
}

fn urls(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

// ---------------------------------------------------------------------------
// Part 1: data parallelism and link checking
// ---------------------------------------------------------------------------

#[test]
fn part1_parallel_map_preserves_input_order() {
    let items: Vec<u32> = (0..100).collect();
    let out = parallel_map(&items, 4, |x| x * 2);
    let expected: Vec<u32> = (0..100).map(|x| x * 2).collect();
    assert_eq!(out, expected, "output must follow input order");
}

#[test]
fn part1_parallel_map_empty_input_never_calls_f() {
    let items: Vec<u32> = Vec::new();
    let calls = AtomicUsize::new(0);
    let out = parallel_map(&items, 3, |x: &u32| {
        calls.fetch_add(1, Ordering::Relaxed);
        *x
    });
    assert!(out.is_empty(), "empty input must give empty output");
    assert_eq!(calls.load(Ordering::Relaxed), 0, "f must not run for empty input");
}

#[test]
fn part1_parallel_map_uses_one_thread_per_chunk() {
    // Three items with three workers: each item lands on its own thread, so the
    // barrier can only be passed if all three run concurrently.
    let ids = within("parallel_map with a barrier", || {
        let barrier = Arc::new(Barrier::new(3));
        let items = vec![0u8, 1, 2];
        parallel_map(&items, 3, move |_| {
            barrier.wait();
            thread::current().id()
        })
    });
    let distinct: HashSet<_> = ids.into_iter().collect();
    assert_eq!(distinct.len(), 3, "three workers must mean three threads");
}

#[test]
fn part1_parallel_map_propagates_worker_panic_payload() {
    let items = [1, 2, 3];
    let msg = panic_message(|| {
        parallel_map(&items[..], 2, |x| {
            if *x == 2 {
                panic!("boom");
            }
            *x
        })
    });
    assert!(msg.contains("boom"), "worker panic payload must be preserved, got {msg:?}");
}

#[test]
fn part1_parallel_map_zero_workers_panics_mentioning_workers() {
    let items = [1, 2, 3];
    let msg = panic_message(|| parallel_map(&items[..], 0, |x| *x));
    assert!(msg.contains("workers"), "panic must mention `workers`, got {msg:?}");
}

#[test]
fn part1_check_links_mirrors_input_and_fetches_each_distinct_url_once() {
    let input = urls(&["a", "b", "a", "c", "b", "a"]);
    let fetched = Arc::new(Mutex::new(Vec::<String>::new()));
    let log = Arc::clone(&fetched);
    let results = within("check_links", {
        let input = input.clone();
        move || {
            link::check_links(&input, 3, move |u| {
                log.lock().unwrap().push(u.to_owned());
                Ok::<u16, String>(200)
            })
        }
    });

    let result_urls: Vec<&str> = results.iter().map(|r| r.url.as_str()).collect();
    let input_urls: Vec<&str> = input.iter().map(String::as_str).collect();
    assert_eq!(result_urls, input_urls, "results must mirror input order and length");
    assert!(
        results.iter().all(|r| r.outcome == Outcome::Status(200)),
        "every URL should carry the fetched status: {results:?}"
    );

    let mut calls = fetched.lock().unwrap().clone();
    calls.sort();
    assert_eq!(calls, vec!["a", "b", "c"], "each distinct URL must be fetched exactly once");
}

#[test]
fn part1_check_links_records_error_per_url_without_affecting_others() {
    let input = urls(&["ok-1", "down", "ok-2"]);
    let results = within("check_links with a failing URL", move || {
        link::check_links(&input, 2, |u| {
            if u == "down" {
                Err("connection refused".to_owned())
            } else {
                Ok(204)
            }
        })
    });
    assert_eq!(results[0].outcome, Outcome::Status(204));
    assert_eq!(results[1].outcome, Outcome::Error("connection refused".to_owned()));
    assert_eq!(results[2].outcome, Outcome::Status(204));
}

#[test]
fn part1_outcome_is_ok_for_2xx_and_3xx_only() {
    for status in [200, 204, 301, 399] {
        assert!(Outcome::Status(status).is_ok(), "{status} should be ok");
    }
    for status in [199, 404, 500] {
        assert!(!Outcome::Status(status).is_ok(), "{status} should not be ok");
    }
    assert!(!Outcome::Error("timeout".to_owned()).is_ok(), "errors are never ok");
}

#[test]
fn part1_check_links_fetches_distinct_urls_concurrently() {
    // Two distinct URLs, two workers: both fetches must be in flight together.
    let input = urls(&["a", "b"]);
    let results = within("check_links with a barrier", move || {
        let barrier = Barrier::new(2);
        link::check_links(&input, 2, move |_| {
            barrier.wait();
            Ok::<u16, String>(200)
        })
    });
    assert_eq!(results.len(), 2, "both URLs must be checked");
}

#[test]
fn part1_check_links_empty_input_gives_empty_output() {
    let none: Vec<String> = Vec::new();
    let results = link::check_links(&none, 4, |_| Ok::<u16, String>(200));
    assert!(results.is_empty(), "no URLs means no results");
}

// ---------------------------------------------------------------------------
// Part 2: a bounded downloader
// ---------------------------------------------------------------------------

#[test]
fn part2_download_all_returns_bodies_in_input_order() {
    let fetch = |u: &str| Ok::<Vec<u8>, String>(u.as_bytes().to_vec());
    let dl = BoundedDownloader::new(fetch, 3);
    let out = dl.download_all(&urls(&["x", "yy", "zzz"]));
    assert_eq!(
        out,
        Ok(vec![b"x".to_vec(), b"yy".to_vec(), b"zzz".to_vec()]),
        "bodies must come back in input order"
    );
}

#[test]
fn part2_download_all_empty_input_is_ok_and_empty() {
    let fetch = |_: &str| Ok::<Vec<u8>, String>(Vec::new());
    let dl = BoundedDownloader::new(fetch, 2);
    let none: Vec<String> = Vec::new();
    assert_eq!(dl.download_all(&none), Ok(Vec::new()));
}

#[test]
fn part2_peak_concurrency_reaches_the_limit_but_not_beyond() {
    // Six URLs, limit two, and each fetch waits on a barrier of two: the pairs
    // must run together, and no third fetch may join a pair.
    let (out, peak, in_flight) = within("download_all with a barrier", || {
        let barrier = Arc::new(Barrier::new(2));
        let fetch = move |_: &str| {
            barrier.wait();
            Ok::<Vec<u8>, String>(Vec::new())
        };
        let dl = BoundedDownloader::new(fetch, 2);
        let out = dl.download_all(&urls(&["1", "2", "3", "4", "5", "6"]));
        (out, dl.peak_concurrency(), dl.in_flight())
    });
    assert!(out.is_ok(), "all fetches succeed, got {out:?}");
    assert_eq!(peak, 2, "peak concurrency must equal the limit");
    assert_eq!(in_flight, 0, "no fetch may remain in flight afterwards");
}

#[test]
fn part2_gate_is_shared_across_callers() {
    // Two callers share one downloader with a limit of one: at most one fetch
    // may run at a time across both.
    let (peak, max_seen) = within("two callers on a shared downloader", || {
        let active = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        let (a, m) = (Arc::clone(&active), Arc::clone(&max_seen));
        let fetch = move |_: &str| {
            let now = a.fetch_add(1, Ordering::SeqCst) + 1;
            m.fetch_max(now, Ordering::SeqCst);
            thread::yield_now();
            a.fetch_sub(1, Ordering::SeqCst);
            Ok::<Vec<u8>, String>(Vec::new())
        };
        let dl = BoundedDownloader::new(fetch, 1);
        let work = urls(&["a", "b", "c"]);
        thread::scope(|s| {
            s.spawn(|| dl.download_all(&work).unwrap());
            s.spawn(|| dl.download_all(&work).unwrap());
        });
        (dl.peak_concurrency(), max_seen.load(Ordering::SeqCst))
    });
    assert_eq!(peak, 1, "gate peak must be 1 with a limit of 1");
    assert_eq!(max_seen, 1, "the fetch function itself must never overlap");
}

#[test]
fn part2_earliest_failing_url_error_is_returned_and_all_urls_attempted() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&attempts);
    let fetch = move |u: &str| {
        counter.fetch_add(1, Ordering::SeqCst);
        match u {
            "bad1" => Err("bad1 failed".to_owned()),
            "bad2" => Err("bad2 failed".to_owned()),
            _ => Ok(Vec::new()),
        }
    };
    let dl = BoundedDownloader::new(fetch, 4);
    let out = dl.download_all(&urls(&["ok1", "bad1", "ok2", "bad2"]));
    assert_eq!(out, Err("bad1 failed".to_owned()), "earliest failing input index wins");
    assert_eq!(attempts.load(Ordering::SeqCst), 4, "every URL must be attempted");
}

#[test]
fn part2_new_rejects_zero_max_concurrent_mentioning_max_concurrent() {
    let msg = panic_message(|| BoundedDownloader::new(|_: &str| Ok::<Vec<u8>, String>(Vec::new()), 0));
    assert!(
        msg.contains("max_concurrent"),
        "panic must mention `max_concurrent`, got {msg:?}"
    );
}

#[test]
fn part2_fresh_downloader_reports_zero_counters() {
    let dl = BoundedDownloader::new(|_: &str| Ok::<Vec<u8>, String>(Vec::new()), 2);
    assert_eq!(dl.in_flight(), 0, "nothing is running yet");
    assert_eq!(dl.peak_concurrency(), 0, "nothing has run yet");
}

// ---------------------------------------------------------------------------
// Part 3: atomics and shared state
// ---------------------------------------------------------------------------

#[test]
fn part3_counters_concurrent_adds_lose_no_updates() {
    let counters = Arc::new(Counters::new());
    thread::scope(|s| {
        for _ in 0..8 {
            let counters = Arc::clone(&counters);
            s.spawn(move || {
                for _ in 0..5000 {
                    counters.add("hits", 1);
                }
            });
        }
    });
    assert_eq!(counters.get("hits"), 40_000, "no increment may be lost");
}

#[test]
fn part3_add_creates_missing_counter_and_returns_new_value() {
    let counters = Counters::new();
    assert_eq!(counters.add("bytes", 10), 10, "first add creates the counter at zero");
    assert_eq!(counters.add("bytes", 5), 15, "add returns the updated value");
}

#[test]
fn part3_get_unknown_counter_is_zero() {
    let counters = Counters::new();
    assert_eq!(counters.get("never-touched"), 0);
}

#[test]
fn part3_snapshot_is_sorted_and_does_not_change_later() {
    let counters = Counters::new();
    counters.add("b", 2);
    counters.add("a", 1);
    let snap = counters.snapshot();
    let keys: Vec<&str> = snap.keys().map(String::as_str).collect();
    assert_eq!(keys, vec!["a", "b"], "snapshot must be sorted by name");
    counters.add("a", 100);
    let expected: BTreeMap<String, u64> = [("a".to_owned(), 1), ("b".to_owned(), 2)].into();
    assert_eq!(snap, expected, "a snapshot is a copy, not a live view");
}

#[test]
fn part3_counters_are_shareable_across_threads() {
    let counters = Arc::new(Counters::new());
    let c = Arc::clone(&counters);
    thread::spawn(move || {
        c.add("from-thread", 3);
    })
    .join()
    .expect("worker thread must not panic");
    assert_eq!(counters.get("from-thread"), 3);
}

#[test]
fn part3_stop_flag_starts_unset() {
    assert!(!StopFlag::new().is_stopped(), "a new flag must not be set");
}

#[test]
fn part3_stop_flag_is_observed_by_another_thread() {
    let flag = Arc::new(StopFlag::new());
    let watcher = Arc::clone(&flag);
    let handle = thread::spawn(move || spin_until("stop flag", || watcher.is_stopped()));
    flag.request_stop();
    handle.join().expect("watcher must observe the stop");
}

#[test]
fn part3_stop_flag_request_is_idempotent() {
    let flag = StopFlag::new();
    flag.request_stop();
    flag.request_stop();
    assert!(flag.is_stopped(), "flag stays set after repeated requests");
}

#[test]
fn part3_handoff_empty_then_publish_then_take_once() {
    let slot = Handoff::new();
    assert_eq!(slot.try_take(), None, "nothing published yet");
    slot.publish(7);
    assert_eq!(slot.try_take(), Some(7));
    assert_eq!(slot.try_take(), None, "a value is delivered exactly once");
}

#[test]
fn part3_handoff_second_publish_replaces_untaken_value() {
    let slot = Handoff::new();
    slot.publish(1);
    slot.publish(2);
    assert_eq!(slot.try_take(), Some(2), "the newer value replaces the older one");
}

#[test]
fn part3_handoff_delivers_across_threads() {
    let slot = Arc::new(Handoff::<String>::new());
    let producer = Arc::clone(&slot);
    thread::spawn(move || producer.publish("ready".to_owned()));
    let mut got = None;
    spin_until("handoff value", || {
        got = slot.try_take();
        got.is_some()
    });
    assert_eq!(got.as_deref(), Some("ready"));
}

// ---------------------------------------------------------------------------
// Part 4: mutexes, lock ordering and poisoning
// ---------------------------------------------------------------------------

#[test]
fn part4_withdraw_more_than_balance_reports_requested_and_available() {
    let acc = Account::new("savings", 10);
    assert_eq!(
        acc.withdraw(25),
        Err(BankError::InsufficientFunds { requested: 25, available: 10 })
    );
    assert_eq!(acc.balance(), 10, "a refused withdrawal changes nothing");
}

#[test]
fn part4_zero_amounts_are_rejected_by_deposit_and_withdraw() {
    let acc = Account::new("a", 10);
    assert_eq!(acc.deposit(0), Err(BankError::ZeroAmount));
    assert_eq!(acc.withdraw(0), Err(BankError::ZeroAmount));
    assert_eq!(acc.balance(), 10);
}

#[test]
fn part4_deposit_overflow_is_refused_not_wrapped() {
    let acc = Account::new("a", u64::MAX - 5);
    assert_eq!(acc.deposit(10), Err(BankError::Overflow));
    assert_eq!(acc.balance(), u64::MAX - 5, "balance must be untouched");
}

#[test]
fn part4_deposit_and_withdraw_return_new_balance() {
    let acc = Account::new("a", 10);
    assert_eq!(acc.deposit(5), Ok(15));
    assert_eq!(acc.withdraw(3), Ok(12));
    assert_eq!(acc.balance(), 12);
}

#[test]
fn part4_account_reports_its_id() {
    assert_eq!(Account::new("checking", 0).id(), "checking");
}

#[test]
fn part4_update_runs_closure_under_the_lock() {
    let acc = Account::new("u", 5);
    let doubled = acc.update(|balance| {
        *balance += 10;
        *balance * 2
    });
    assert_eq!(doubled, 30);
    assert_eq!(acc.balance(), 15);
}

#[test]
fn part4_transfer_moves_funds_between_accounts() {
    let a = Account::new("a", 10);
    let b = Account::new("b", 0);
    assert_eq!(bank::transfer(&a, &b, 4), Ok(()));
    assert_eq!((a.balance(), b.balance()), (6, 4));
}

#[test]
fn part4_transfer_to_same_account_is_checked_before_amount() {
    let a = Account::new("a", 10);
    assert_eq!(bank::transfer(&a, &a, 0), Err(BankError::SameAccount));
    assert_eq!(bank::transfer(&a, &a, 5), Err(BankError::SameAccount));
    assert_eq!(a.balance(), 10);
}

#[test]
fn part4_transfer_zero_amount_is_rejected() {
    let a = Account::new("a", 10);
    let b = Account::new("b", 0);
    assert_eq!(bank::transfer(&a, &b, 0), Err(BankError::ZeroAmount));
}

#[test]
fn part4_transfer_insufficient_funds_leaves_both_unchanged() {
    let a = Account::new("a", 3);
    let b = Account::new("b", 0);
    assert_eq!(
        bank::transfer(&a, &b, 5),
        Err(BankError::InsufficientFunds { requested: 5, available: 3 })
    );
    assert_eq!((a.balance(), b.balance()), (3, 0));
}

#[test]
fn part4_transfer_destination_overflow_leaves_both_unchanged() {
    let a = Account::new("a", 10);
    let b = Account::new("b", u64::MAX);
    assert_eq!(bank::transfer(&a, &b, 1), Err(BankError::Overflow));
    assert_eq!((a.balance(), b.balance()), (10, u64::MAX));
}

#[test]
fn part4_opposing_transfers_do_not_deadlock() {
    let a = Arc::new(Account::new("a", 1000));
    let b = Arc::new(Account::new("b", 1000));
    let (ca, cb) = within("opposing transfers", {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || {
            thread::scope(|s| {
                for _ in 0..4 {
                    s.spawn(|| {
                        for _ in 0..500 {
                            bank::transfer(&a, &b, 1).unwrap();
                            bank::transfer(&b, &a, 1).unwrap();
                        }
                    });
                }
            });
            (a.balance(), b.balance())
        }
    });
    assert_eq!((ca, cb), (1000, 1000), "every a->b is paired with b->a");
}

#[test]
fn part4_concurrent_deposits_sum_exactly() {
    let acc = Arc::new(Account::new("t", 0));
    thread::scope(|s| {
        for _ in 0..4 {
            s.spawn(|| {
                for _ in 0..1000 {
                    acc.deposit(1).unwrap();
                }
            });
        }
    });
    assert_eq!(acc.balance(), 4000);
}

#[test]
fn part4_account_stays_usable_after_panic_inside_update() {
    let acc = Account::new("p", 100);
    let msg = panic_message(|| {
        acc.update(|_: &mut u64| -> () { panic!("inside update") });
    });
    assert!(msg.contains("inside update"), "panic payload must propagate, got {msg:?}");
    assert_eq!(acc.balance(), 100, "the balance survives the poisoned lock");
    assert_eq!(acc.deposit(1), Ok(101), "the account keeps working after poisoning");
}

#[test]
fn part4_bank_error_display_mentions_the_problem() {
    let text = BankError::InsufficientFunds { requested: 25, available: 10 }.to_string();
    assert!(text.contains("25") && text.contains("10"), "display must show both amounts: {text:?}");
    assert!(
        BankError::SameAccount.to_string().to_lowercase().contains("same"),
        "same-account error must say so"
    );
}

// ---------------------------------------------------------------------------
// Part 5: LRU cache and single-flight loading
// ---------------------------------------------------------------------------

#[test]
fn part5_lru_evicts_least_recently_used_entry() {
    let mut cache = LruCache::new(2);
    cache.insert("a", 1);
    cache.insert("b", 2);
    assert_eq!(cache.get(&"a"), Some(&1), "touching a makes b the oldest");
    cache.insert("c", 3);
    assert_eq!(cache.get(&"b"), None, "b must have been evicted");
    assert_eq!(cache.get(&"a"), Some(&1));
    assert_eq!(cache.get(&"c"), Some(&3));
}

#[test]
fn part5_lru_reinsert_replaces_value_without_evicting() {
    let mut cache = LruCache::new(2);
    cache.insert("a", 1);
    cache.insert("b", 2);
    cache.insert("a", 10);
    assert_eq!(cache.len(), 2, "replacing a key must not grow or evict");
    assert_eq!(cache.get(&"a"), Some(&10));
    assert_eq!(cache.get(&"b"), Some(&2));
}

#[test]
fn part5_lru_len_never_exceeds_capacity() {
    let mut cache = LruCache::new(3);
    assert!(cache.is_empty(), "a new cache is empty");
    for i in 0..10 {
        cache.insert(i, i * i);
    }
    assert_eq!(cache.len(), 3);
    assert!(!cache.is_empty());
}

#[test]
fn part5_lru_zero_capacity_panics_mentioning_capacity() {
    let msg = panic_message(|| LruCache::<u8, u8>::new(0));
    assert!(msg.contains("capacity"), "panic must mention `capacity`, got {msg:?}");
}

#[test]
fn part5_lru_shared_behind_mutex_stays_within_capacity() {
    let cache = Arc::new(Mutex::new(LruCache::new(16)));
    thread::scope(|s| {
        for t in 0..4u32 {
            let cache = Arc::clone(&cache);
            s.spawn(move || {
                for i in 0..200u32 {
                    cache.lock().unwrap().insert(i % 50, t * 1000 + i);
                }
            });
        }
    });
    let len = cache.lock().unwrap().len();
    assert_eq!(len, 16, "50 distinct keys into capacity 16 must be full");
}

#[test]
fn part5_single_flight_runs_loader_once_for_concurrent_callers() {
    let (values, loads) = within("16 concurrent callers", || {
        let sf = Arc::new(SingleFlight::<String, u32, String>::new());
        let barrier = Arc::new(Barrier::new(16));
        let values: Vec<Result<u32, String>> = thread::scope(|s| {
            let handles: Vec<_> = (0..16)
                .map(|_| {
                    let sf = Arc::clone(&sf);
                    let barrier = Arc::clone(&barrier);
                    s.spawn(move || {
                        barrier.wait();
                        sf.get_or_load("key".to_owned(), || Ok(42))
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        (values, sf.load_count())
    });
    assert_eq!(loads, 1, "the loader must run exactly once");
    assert!(values.iter().all(|v| *v == Ok(42)), "every caller gets the value: {values:?}");
}

#[test]
fn part5_single_flight_distinct_keys_load_in_parallel() {
    // Each loader waits on a barrier of two: both must be running together.
    let (a, b, loads) = within("two keys in parallel", || {
        let sf = Arc::new(SingleFlight::<&'static str, u32, String>::new());
        let barrier = Arc::new(Barrier::new(2));
        let (sa, ba) = (Arc::clone(&sf), Arc::clone(&barrier));
        let other = thread::spawn(move || {
            sa.get_or_load("a", move || {
                ba.wait();
                Ok(1)
            })
        });
        let b = sf.get_or_load("b", || {
            barrier.wait();
            Ok(2)
        });
        let a = other.join().unwrap();
        (a, b, sf.load_count())
    });
    assert_eq!((a, b), (Ok(1), Ok(2)));
    assert_eq!(loads, 2, "each key is loaded once");
}

#[test]
fn part5_single_flight_keeps_successful_values() {
    let sf = SingleFlight::<String, u32, String>::new();
    assert_eq!(sf.get_or_load("k".to_owned(), || Ok(5)), Ok(5));
    assert_eq!(
        sf.get_or_load("k".to_owned(), || Ok(99)),
        Ok(5),
        "a stored success is returned without running the loader"
    );
    assert_eq!(sf.load_count(), 1);
}

#[test]
fn part5_single_flight_does_not_keep_errors() {
    let sf = SingleFlight::<String, u32, String>::new();
    assert_eq!(sf.get_or_load("k".to_owned(), || Err("down".to_owned())), Err("down".to_owned()));
    assert_eq!(sf.get_or_load("k".to_owned(), || Ok(7)), Ok(7), "the next call runs the loader again");
    assert_eq!(sf.load_count(), 2);
}

#[test]
fn part5_single_flight_loader_panic_reaches_caller_and_loader_can_retry() {
    let sf = SingleFlight::<String, u32, String>::new();
    let msg = panic_message(|| {
        sf.get_or_load("k".to_owned(), || -> Result<u32, String> { panic!("loader exploded") })
    });
    assert!(msg.contains("loader exploded"), "panic must reach the caller, got {msg:?}");
    assert_eq!(sf.get_or_load("k".to_owned(), || Ok(3)), Ok(3), "the key must not stay poisoned");
}

#[test]
fn part5_single_flight_waiter_takes_over_when_leader_panics() {
    let (leader_failed, waiter, loads) = within("leader panics while a waiter is queued", || {
        let sf = Arc::new(SingleFlight::<String, u32, String>::new());
        let (entered_tx, entered_rx) = mpsc::channel::<()>();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let leader = {
            let sf = Arc::clone(&sf);
            thread::spawn(move || {
                sf.get_or_load("k".to_owned(), move || {
                    entered_tx.send(()).unwrap();
                    let _ = release_rx.recv_timeout(LIMIT);
                    panic!("leader gave up")
                })
            })
        };
        recv(&entered_rx, "leader entering its loader");
        let waiter = {
            let sf = Arc::clone(&sf);
            thread::spawn(move || sf.get_or_load("k".to_owned(), || Ok(9)))
        };
        release_tx.send(()).unwrap();
        let leader_failed = leader.join().is_err();
        let waiter = waiter.join().unwrap();
        (leader_failed, waiter, sf.load_count())
    });
    assert!(leader_failed, "the leader's panic must reach its own caller");
    assert_eq!(waiter, Ok(9), "the waiter must retry and get its own value");
    assert_eq!(loads, 2, "the leader's loader and the waiter's loader each ran once");
}

// ---------------------------------------------------------------------------
// Part 6: a thread pool with a bounded queue
// ---------------------------------------------------------------------------

#[test]
fn part6_pool_runs_jobs_on_distinct_threads() {
    let distinct = within("pool with a barrier", || {
        let pool = ThreadPool::new(4, 4);
        let barrier = Arc::new(Barrier::new(4));
        let (tx, rx) = mpsc::channel();
        for _ in 0..4 {
            let barrier = Arc::clone(&barrier);
            let tx = tx.clone();
            pool.execute(move || {
                barrier.wait();
                tx.send(thread::current().id()).unwrap();
            });
        }
        let ids: HashSet<_> = (0..4).map(|_| recv(&rx, "a job's thread id")).collect();
        pool.shutdown();
        ids.len()
    });
    assert_eq!(distinct, 4, "four workers must run four jobs at once");
}

#[test]
fn part6_dropping_pool_runs_every_queued_job() {
    let done = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&done);
    within("dropping the pool", move || {
        let pool = ThreadPool::new(2, 100);
        for _ in 0..100 {
            let c = Arc::clone(&counter);
            pool.execute(move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        drop(pool);
    });
    assert_eq!(done.load(Ordering::SeqCst), 100, "queued jobs must finish before drop returns");
}

#[test]
fn part6_shutdown_reports_completed_jobs() {
    let pool = ThreadPool::new(3, 10);
    for _ in 0..20 {
        pool.execute(|| {});
    }
    assert_eq!(pool.shutdown(), PoolStats { completed: 20, panicked: 0 });
}

#[test]
fn part6_panicking_job_does_not_kill_its_worker() {
    // One worker: the job after the panic can only run if that worker survived.
    let pool = ThreadPool::new(1, 10);
    let ran = Arc::new(AtomicUsize::new(0));
    pool.execute(|| panic!("job failed"));
    let r = Arc::clone(&ran);
    pool.execute(move || {
        r.fetch_add(1, Ordering::SeqCst);
    });
    let stats = pool.shutdown();
    assert_eq!(stats, PoolStats { completed: 1, panicked: 1 });
    assert_eq!(ran.load(Ordering::SeqCst), 1, "the worker must keep serving after a panic");
}

#[test]
fn part6_try_execute_reports_queue_full_without_blocking() {
    let pool = ThreadPool::new(1, 1);
    let (started_tx, started_rx) = mpsc::channel::<()>();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    pool.execute(move || {
        started_tx.send(()).unwrap();
        let _ = release_rx.recv_timeout(LIMIT);
    });
    recv(&started_rx, "the blocking job to start");
    assert_eq!(pool.try_execute(|| {}), Ok(()), "one queue slot is free");
    assert_eq!(pool.try_execute(|| {}), Err(QueueFull), "the queue is now full");
    release_tx.send(()).unwrap();
    assert_eq!(pool.shutdown(), PoolStats { completed: 2, panicked: 0 });
}

#[test]
fn part6_queue_full_display_says_full() {
    assert!(QueueFull.to_string().contains("full"));
}

#[test]
fn part6_zero_workers_panics_mentioning_worker() {
    let msg = panic_message(|| ThreadPool::new(0, 4));
    assert!(msg.contains("worker"), "panic must mention `worker`, got {msg:?}");
}

// ---------------------------------------------------------------------------
// Part 7: periodic jobs, failures, and a bounded stop
// ---------------------------------------------------------------------------

#[test]
fn part7_job_runs_immediately_on_start() {
    let runs = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&runs);
    let mut sched = Scheduler::new();
    sched
        .add("tick", Duration::from_secs(3600), move || {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .unwrap();
    sched.start().unwrap();
    spin_until("the first run", || runs.load(Ordering::SeqCst) >= 1);
    assert_eq!(sched.stop(LIMIT), Ok(()));
    assert_eq!(runs.load(Ordering::SeqCst), 1, "an hour-long interval gives one run");
}

#[test]
fn part7_job_repeats_every_interval() {
    let runs = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&runs);
    let mut sched = Scheduler::new();
    sched
        .add("fast", Duration::from_millis(1), move || {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .unwrap();
    sched.start().unwrap();
    spin_until("three runs", || runs.load(Ordering::SeqCst) >= 3);
    assert_eq!(sched.stop(LIMIT), Ok(()));
}

#[test]
fn part7_failing_job_is_recorded_and_keeps_running() {
    let runs = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&runs);
    let mut sched = Scheduler::new();
    sched
        .add("flaky", Duration::from_millis(1), move || {
            counter.fetch_add(1, Ordering::SeqCst);
            Err("flaky".into())
        })
        .unwrap();
    sched.start().unwrap();
    spin_until("two failing runs", || runs.load(Ordering::SeqCst) >= 2);
    assert_eq!(sched.stop(LIMIT), Ok(()));
    let errors = sched.errors();
    assert!(errors.len() >= 2, "every failure is recorded, got {errors:?}");
    assert!(
        errors.iter().all(|e| e.job == "flaky" && e.message == "flaky"),
        "each record names the job and its message: {errors:?}"
    );
}

#[test]
fn part7_panicking_job_is_recorded_with_panic_prefix() {
    let mut sched = Scheduler::new();
    sched
        .add("crasher", Duration::from_secs(3600), || -> Result<(), BoxError> {
            panic!("kaboom")
        })
        .unwrap();
    sched.start().unwrap();
    spin_until("the panic to be recorded", || !sched.errors().is_empty());
    assert_eq!(sched.stop(LIMIT), Ok(()));
    let errors = sched.errors();
    assert_eq!(errors.len(), 1, "one panic, one record: {errors:?}");
    assert_eq!(errors[0].job, "crasher");
    assert!(
        errors[0].message.starts_with("panic: ") && errors[0].message.contains("kaboom"),
        "panic records start with `panic: `, got {:?}",
        errors[0].message
    );
}

#[test]
fn part7_stop_wakes_waiting_jobs_promptly() {
    let mut sched = Scheduler::new();
    sched.add("slow", Duration::from_secs(3600), || Ok(())).unwrap();
    sched.start().unwrap();
    let began = Instant::now();
    assert_eq!(sched.stop(LIMIT), Ok(()));
    assert!(
        began.elapsed() < Duration::from_secs(5),
        "stop must wake the waiting job instead of waiting out its hour-long interval"
    );
}

#[test]
fn part7_stop_joins_threads_and_releases_job_state() {
    let token = Arc::new(());
    let held = Arc::clone(&token);
    let mut sched = Scheduler::new();
    sched
        .add("holder", Duration::from_millis(1), move || {
            let _ = &held;
            Ok(())
        })
        .unwrap();
    sched.start().unwrap();
    assert_eq!(sched.stop(LIMIT), Ok(()));
    assert_eq!(Arc::strong_count(&token), 1, "a joined job must have dropped its captures");
}

#[test]
fn part7_stop_times_out_while_a_job_is_blocked() {
    let (entered_tx, entered_rx) = mpsc::channel::<()>();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let mut sched = Scheduler::new();
    sched
        .add("stuck", Duration::from_millis(1), move || {
            let _ = entered_tx.send(());
            let _ = release_rx.recv_timeout(LIMIT);
            Ok(())
        })
        .unwrap();
    sched.start().unwrap();
    recv(&entered_rx, "the stuck job to enter its task");
    assert_eq!(sched.stop(Duration::from_millis(50)), Err(ScheduleError::StopTimeout));
    release_tx.send(()).unwrap();
    assert_eq!(sched.stop(LIMIT), Ok(()), "a later stop waits for the job to finish");
}

#[test]
fn part7_stop_before_start_is_a_no_op() {
    let mut sched = Scheduler::new();
    assert_eq!(sched.stop(LIMIT), Ok(()));
}

#[test]
fn part7_start_twice_and_add_after_start_are_rejected() {
    let mut sched = Scheduler::new();
    sched.start().unwrap();
    assert_eq!(sched.start(), Err(ScheduleError::AlreadyStarted));
    assert_eq!(
        sched.add("late", Duration::from_secs(1), || Ok(())),
        Err(ScheduleError::AlreadyStarted)
    );
    assert_eq!(sched.stop(LIMIT), Ok(()));
}

#[test]
fn part7_add_rejects_zero_interval() {
    let mut sched = Scheduler::new();
    assert_eq!(
        sched.add("zero", Duration::ZERO, || Ok(())),
        Err(ScheduleError::InvalidInterval)
    );
}

#[test]
fn part7_add_rejects_duplicate_names() {
    let mut sched = Scheduler::new();
    sched.add("dup", Duration::from_secs(1), || Ok(())).unwrap();
    assert_eq!(
        sched.add("dup", Duration::from_secs(1), || Ok(())),
        Err(ScheduleError::DuplicateName("dup".to_owned()))
    );
}

#[test]
fn part7_dropping_running_scheduler_joins_its_jobs() {
    let token = Arc::new(());
    let held = Arc::clone(&token);
    let mut sched = Scheduler::new();
    sched
        .add("holder", Duration::from_millis(1), move || {
            let _ = &held;
            Ok(())
        })
        .unwrap();
    sched.start().unwrap();
    drop(sched);
    assert_eq!(Arc::strong_count(&token), 1, "drop must stop and join the jobs");
}

#[test]
fn part7_job_error_display_names_job_and_message() {
    let text = JobError { job: "feed".to_owned(), message: "timeout".to_owned() }.to_string();
    assert!(text.contains("feed") && text.contains("timeout"), "got {text:?}");
}
