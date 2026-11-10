//! Part 2: returning closures, fn pointers and boxed callback registries.

use ex04_closures_and_functions::callbacks::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

#[test]
fn part2_lookup_returns_callable_fn_pointers() {
    let add: Operation = lookup_operation("add").expect("add is registered");
    let div: Operation = lookup_operation("div").expect("div is registered");
    assert_eq!(add(2, 3), Some(5));
    assert_eq!(div(7, 2), Some(3), "integer division");
}

#[test]
fn part2_lookup_is_case_sensitive_and_rejects_unknown_names() {
    assert!(lookup_operation("pow").is_none(), "unregistered operation");
    assert!(lookup_operation("ADD").is_none(), "names are case-sensitive");
}

#[test]
fn part2_operations_return_none_on_overflow_and_divide_by_zero() {
    let mul = lookup_operation("mul").unwrap();
    let div = lookup_operation("div").unwrap();
    assert_eq!(mul(i64::MAX, 2), None, "overflow is None, not a wrap or a panic");
    assert_eq!(div(1, 0), None, "division by zero is None");
    assert_eq!(div(i64::MIN, -1), None, "MIN / -1 overflows");
}

#[test]
fn part2_percent_discount_rounds_the_discount_down() {
    let rule = discount_rule("percent", 10).expect("percent is supported");
    assert_eq!(rule(999), 900, "discount is floor(999 * 10%) = 99");
    assert_eq!(rule(5), 5, "a discount of half a cent rounds down to nothing");
}

#[test]
fn part2_percent_discount_accepts_0_to_100_only() {
    assert_eq!(discount_rule("percent", 0).unwrap()(500), 500);
    assert_eq!(discount_rule("percent", 100).unwrap()(500), 0);
    assert!(discount_rule("percent", 101).is_none(), "more than 100 percent is rejected");
}

#[test]
fn part2_flat_discount_saturates_at_zero() {
    let rule = discount_rule("flat", 300).expect("flat is supported");
    assert_eq!(rule(1000), 700);
    assert_eq!(rule(200), 0, "a discount larger than the price floors at zero");
}

#[test]
fn part2_unknown_discount_kind_is_none() {
    assert!(discount_rule("bogo", 1).is_none());
}

#[test]
fn part2_boxed_rules_can_move_across_threads() {
    let rules: Vec<Box<dyn Fn(u64) -> u64 + Send + Sync>> = vec![
        discount_rule("flat", 100).unwrap(),
        discount_rule("percent", 50).unwrap(),
    ];
    let prices = thread::spawn(move || rules.iter().map(|rule| rule(1000)).collect::<Vec<_>>())
        .join()
        .expect("worker thread panicked");
    assert_eq!(prices, vec![900, 500]);
}

#[test]
fn part2_threshold_predicate_is_a_plain_impl_fn() {
    let eligible = at_least(500);
    let flags: Vec<bool> = [499, 500, 501].into_iter().map(&eligible).collect();
    assert_eq!(flags, vec![false, true, true], "the boundary is inclusive");
}

#[test]
fn part2_bus_delivers_in_subscription_order() {
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut bus: EventBus<String> = EventBus::new();
    let first = Arc::clone(&log);
    bus.subscribe(move |event| first.lock().unwrap().push(format!("a:{event}")));
    let second = Arc::clone(&log);
    bus.subscribe(move |event| second.lock().unwrap().push(format!("b:{event}")));

    let delivered = bus.publish(&"deploy".to_string());
    assert_eq!(delivered, 2, "publish reports how many listeners ran");
    assert_eq!(
        *log.lock().unwrap(),
        vec!["a:deploy".to_string(), "b:deploy".to_string()]
    );
}

#[test]
fn part2_unsubscribe_removes_only_that_listener() {
    let hits = Arc::new(AtomicUsize::new(0));
    let mut bus: EventBus<u32> = EventBus::new();
    let a = Arc::clone(&hits);
    let id_a = bus.subscribe(move |_| {
        a.fetch_add(1, Ordering::SeqCst);
    });
    let b = Arc::clone(&hits);
    bus.subscribe(move |_| {
        b.fetch_add(10, Ordering::SeqCst);
    });

    assert!(bus.unsubscribe(id_a), "first unsubscribe finds the listener");
    assert!(!bus.unsubscribe(id_a), "second unsubscribe finds nothing");
    assert_eq!(bus.publish(&1), 1, "one listener left");
    assert_eq!(hits.load(Ordering::SeqCst), 10, "only the remaining listener ran");
}

#[test]
fn part2_subscription_ids_are_never_reused() {
    let mut bus: EventBus<u32> = EventBus::new();
    let first = bus.subscribe(|_| {});
    bus.unsubscribe(first);
    let second = bus.subscribe(|_| {});
    assert_ne!(first, second, "a stale id must not match a newer listener");
    assert_eq!(bus.len(), 1);
    assert!(!bus.is_empty());
}

#[test]
fn part2_bus_is_safe_to_publish_to_from_many_threads() {
    let hits = Arc::new(AtomicUsize::new(0));
    let mut bus: EventBus<u32> = EventBus::new();
    let counter = Arc::clone(&hits);
    bus.subscribe(move |_| {
        counter.fetch_add(1, Ordering::Relaxed);
    });
    let bus = Arc::new(bus);
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let bus = Arc::clone(&bus);
            thread::spawn(move || {
                (0..250).for_each(|n| {
                    bus.publish(&n);
                });
            })
        })
        .collect();
    for worker in workers {
        worker.join().expect("publisher panicked");
    }
    assert_eq!(hits.load(Ordering::SeqCst), 1000, "each publish reaches the listener once");
}
