//! Part 1: `count!`, `hashmap!`, `btreeset!`.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};

use ex20_macros::{btreeset, count, hashmap};

/// Records `v` in `log` and returns it, so tests can observe evaluation.
fn note(log: &RefCell<Vec<i32>>, v: i32) -> i32 {
    log.borrow_mut().push(v);
    v
}

#[test]
fn part1_count_empty_is_zero() {
    let n: usize = count!();
    assert_eq!(n, 0, "count!() should be 0");
}

#[test]
fn part1_count_counts_expressions_with_trailing_comma() {
    let n: usize = count!(1, "two", 3.0);
    assert_eq!(n, 3, "count! should count three expressions");
    let m: usize = count!(1, 2, 3,);
    assert_eq!(m, 3, "a trailing comma must not be counted");
}

#[test]
fn part1_count_does_not_evaluate_arguments() {
    let log = RefCell::new(Vec::<i32>::new());
    let n: usize = count!(note(&log, 1), note(&log, 2));
    assert_eq!(n, 2);
    assert!(
        log.borrow().is_empty(),
        "count! must not evaluate its arguments, but log = {:?}",
        log.borrow()
    );
}

#[test]
fn part1_hashmap_builds_pairs() {
    let m: HashMap<&str, i32> = hashmap! { "a" => 1, "b" => 2, "c" => 3 };
    assert_eq!(m.len(), 3, "hashmap! should hold three pairs");
    assert_eq!(m.get("a"), Some(&1));
    assert_eq!(m.get("b"), Some(&2));
    assert_eq!(m.get("c"), Some(&3));
}

#[test]
fn part1_hashmap_empty_and_trailing_comma() {
    let empty: HashMap<&str, i32> = hashmap! {};
    assert!(empty.is_empty(), "hashmap! {{}} should be empty");
    let one: HashMap<&str, i32> = hashmap! { "k" => 1, };
    assert_eq!(one.len(), 1, "trailing comma should be accepted");
}

#[test]
fn part1_hashmap_duplicate_key_keeps_last_value() {
    let m: HashMap<&str, i32> = hashmap! { "k" => 1, "k" => 2 };
    assert_eq!(m.len(), 1, "duplicate keys collapse to one entry");
    assert_eq!(m.get("k"), Some(&2), "the last value should win");
}

#[test]
fn part1_hashmap_evaluates_key_then_value_once_in_order() {
    let log = RefCell::new(Vec::<i32>::new());
    let m: HashMap<i32, i32> = hashmap! {
        note(&log, 1) => note(&log, 2),
        note(&log, 3) => note(&log, 4),
    };
    assert_eq!(
        *log.borrow(),
        vec![1, 2, 3, 4],
        "each key and value should be evaluated once, key first"
    );
    assert_eq!(m.get(&1), Some(&2));
    assert_eq!(m.get(&3), Some(&4));
}

#[test]
fn part1_hashmap_presizes_capacity() {
    let m: HashMap<u8, u8> = hashmap! { 1 => 1, 2 => 2, 3 => 3, 4 => 4 };
    assert!(
        m.capacity() >= 4,
        "capacity {} should fit the four pairs",
        m.capacity()
    );
}

#[test]
fn part1_btreeset_sorts_and_dedups() {
    let s: BTreeSet<i32> = btreeset![3, 1, 2, 3, 1];
    let items: Vec<i32> = s.into_iter().collect();
    assert_eq!(items, vec![1, 2, 3], "btreeset! should sort and de-duplicate");
}

#[test]
fn part1_btreeset_empty_and_expressions() {
    let empty: BTreeSet<u8> = btreeset![];
    assert!(empty.is_empty(), "btreeset![] should be empty");
    let base = 10;
    let s: BTreeSet<i32> = btreeset![base + 1, base * 2,];
    assert!(s.contains(&11) && s.contains(&20), "items are full expressions");
}
