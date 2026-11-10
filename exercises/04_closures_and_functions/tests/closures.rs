//! Part 1: closures, captures and the Fn / FnMut / FnOnce hierarchy.

use ex04_closures_and_functions::closures::*;

fn assert_fn<F: Fn(&str) -> String>(_: &F) {}

#[test]
fn part1_multiplier_owns_its_factor() {
    let double = make_multiplier(2);
    let triple = make_multiplier(3);
    assert_eq!(double(5), 10, "double(5) should be 10");
    assert_eq!(triple(5), 15, "triple(5) should be 15");
    assert_eq!(double(-4), -8, "negative inputs must multiply as usual");
}

#[test]
fn part1_counter_returns_start_then_increments() {
    let mut next = make_counter(7);
    assert_eq!((next(), next(), next()), (7, 8, 9), "first call returns start, then increments");
}

#[test]
fn part1_counters_do_not_share_state() {
    let mut a = make_counter(0);
    let mut b = make_counter(100);
    assert_eq!(a(), 0);
    assert_eq!(a(), 1);
    assert_eq!(b(), 100, "b must not observe a's increments");
    assert_eq!(a(), 2);
}

#[test]
fn part1_compose_applies_first_argument_first() {
    let add_then_double = compose(|x: i64| x + 1, |x: i64| x * 2);
    assert_eq!(add_then_double(3), 8, "(3 + 1) * 2");
    let double_then_add = compose(|x: i64| x * 2, |x: i64| x + 1);
    assert_eq!(double_then_add(3), 7, "(3 * 2) + 1: composition is not commutative");
}

#[test]
fn part1_compose_works_across_types() {
    let len_then_label = compose(|s: &str| s.len(), |n: usize| format!("{n} bytes"));
    assert_eq!(len_then_label("héllo"), "6 bytes", "len counts UTF-8 bytes, not chars");
}

#[test]
fn part1_adders_each_keep_their_own_offset() {
    let adders = make_adders(4);
    let results: Vec<i64> = adders.iter().map(|add| add(100)).collect();
    assert_eq!(results, vec![100, 101, 102, 103], "adder i must add i, not the last loop value");
}

#[test]
fn part1_adders_for_zero_is_empty() {
    assert!(make_adders(0).is_empty(), "no adders requested, no adders returned");
}

#[test]
fn part1_call_n_times_accepts_fnmut_and_keeps_state() {
    let mut total = 0;
    let running = call_n_times(4, || {
        total += 10;
        total
    });
    assert_eq!(running, vec![10, 20, 30, 40], "each call sees the previous state");
    assert_eq!(total, 40, "captured state is visible once the closure is gone");
}

#[test]
fn part1_run_once_moves_captured_value_into_job() {
    let report = String::from("nightly");
    let bytes = run_once(move || report.into_bytes());
    assert_eq!(bytes, b"nightly".to_vec(), "the job consumed the String it captured");
}

#[test]
fn part1_greeter_is_reusable_fn_for_any_borrowed_name() {
    let greet = make_greeter("Hello".to_string());
    assert_fn(&greet);
    let owned = String::from("Ada");
    assert_eq!(greet(&owned), "Hello, Ada!");
    assert_eq!(greet("Grace"), "Hello, Grace!", "one closure serves borrows of any lifetime");
}
