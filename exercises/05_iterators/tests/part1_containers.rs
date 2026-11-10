//! Part 1 — iterator protocol and container traits.

use ex05_iterators::containers::{Countdown, Dataset};

#[test]
fn part1_countdown_yields_start_down_to_one() {
    let got: Vec<u32> = Countdown::new(4).collect();
    assert_eq!(got, vec![4, 3, 2, 1], "Countdown::new(4) must yield 4, 3, 2, 1");
}

#[test]
fn part1_countdown_zero_is_empty() {
    assert_eq!(Countdown::new(0).next(), None, "Countdown::new(0) must be empty");
}

#[test]
fn part1_countdown_size_hint_and_len_are_exact() {
    let mut c = Countdown::new(3);
    assert_eq!(c.size_hint(), (3, Some(3)), "size_hint must be exact before pulling");
    assert_eq!(c.len(), 3, "ExactSizeIterator::len must be 3");
    c.next();
    assert_eq!(c.len(), 2, "len must drop by one per pull");
}

#[test]
fn part1_countdown_rev_counts_up_from_one() {
    let got: Vec<u32> = Countdown::new(4).rev().collect();
    assert_eq!(got, vec![1, 2, 3, 4], "rev() of a countdown must count up from 1");
}

#[test]
fn part1_countdown_front_and_back_meet_in_the_middle() {
    let mut c = Countdown::new(5);
    assert_eq!(c.next(), Some(5), "front yields the start");
    assert_eq!(c.next_back(), Some(1), "back yields 1");
    assert_eq!(c.next(), Some(4), "front continues downward");
    assert_eq!(c.next_back(), Some(2), "back continues upward");
    assert_eq!(c.next(), Some(3), "middle element comes out once");
    assert_eq!(c.next(), None, "exhausted from both ends");
    assert_eq!(c.next_back(), None, "exhausted from the back too");
}

#[test]
fn part1_countdown_stays_exhausted() {
    let mut c = Countdown::new(1);
    assert_eq!(c.next(), Some(1), "first pull yields 1");
    for _ in 0..3 {
        assert_eq!(c.next(), None, "a finished countdown must keep returning None");
    }
}

#[test]
fn part1_countdown_handles_u32_max_without_overflow() {
    let mut c = Countdown::new(u32::MAX);
    assert_eq!(c.len(), u32::MAX as usize, "len of Countdown::new(u32::MAX) must be exact");
    assert_eq!(c.next(), Some(u32::MAX), "front yields u32::MAX first");
    assert_eq!(c.next_back(), Some(1), "back yields 1");
    let last_two: Vec<u32> = Countdown::new(u32::MAX).rev().take(2).collect();
    assert_eq!(last_two, vec![1, 2], "rev() must start counting at 1 even near the limit");
}

#[test]
fn part1_dataset_accepts_generators_and_snapshots_them() {
    let d = Dataset::new((1..=3).map(|x| x * 10));
    assert_eq!(d.len(), 3, "generator input must be snapshotted in full");
    assert!(!d.is_empty(), "three records is not empty");
    let collected: Dataset<i32> = (0..0).collect();
    assert!(collected.is_empty(), "FromIterator over nothing must be empty");
}

#[test]
fn part1_dataset_borrowing_passes_are_independent() {
    let d = Dataset::new(vec![1, 2, 3]);
    let first: Vec<i32> = (&d).into_iter().copied().collect();
    let second: Vec<i32> = d.iter().copied().collect();
    assert_eq!(first, vec![1, 2, 3], "first borrowing pass must see every record");
    assert_eq!(second, vec![1, 2, 3], "a second pass must start fresh, not resume");
}

#[test]
fn part1_dataset_mutable_borrow_updates_in_place() {
    let mut d = Dataset::new(vec![1, 2, 3]);
    for x in &mut d {
        *x *= 2;
    }
    assert_eq!(d.iter().copied().collect::<Vec<_>>(), vec![2, 4, 6], "iter_mut via &mut must update records");
}

#[test]
fn part1_dataset_owned_into_iter_consumes_records() {
    let d = Dataset::new(vec![5, 6, 7]);
    let total: i32 = d.into_iter().sum();
    assert_eq!(total, 18, "owned IntoIterator must yield every record by value");
}

#[test]
fn part1_dataset_extend_appends_records() {
    let mut d: Dataset<&str> = ["a"].into_iter().collect();
    d.extend(["b", "c"]);
    let got: Vec<&str> = d.into_iter().collect();
    assert_eq!(got, vec!["a", "b", "c"], "extend must append in order");
}
