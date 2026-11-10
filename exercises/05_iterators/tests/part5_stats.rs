//! Part 5 — collecting/extending statistics and the RAII finaliser.

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use ex05_iterators::stats::{RunningStats, Stats};

type Seen = Rc<RefCell<Vec<Stats>>>;

fn recorder() -> (Seen, impl FnOnce(Stats) + 'static) {
    let seen: Seen = Rc::new(RefCell::new(Vec::new()));
    let handle = seen.clone();
    (seen, move |s| handle.borrow_mut().push(s))
}

fn close_enough(a: Option<f64>, b: f64) -> bool {
    a.is_some_and(|a| (a - b).abs() < 1e-9)
}

#[test]
fn part5_empty_state_is_the_default_stats() {
    assert_eq!(
        RunningStats::new().snapshot(),
        Stats { count: 0, mean: None, minimum: None, maximum: None },
        "a fresh accumulator is the empty state"
    );
    assert_eq!(Stats::default(), RunningStats::new().snapshot(), "Stats::default() is the empty state");
}

#[test]
fn part5_push_tracks_count_mean_min_and_max() {
    let mut rs = RunningStats::new();
    for x in [1.0, 2.0, 3.0, 4.0] {
        rs.push(x);
    }
    let s = rs.snapshot();
    assert_eq!(s.count, 4, "count of pushed values");
    assert!(close_enough(s.mean, 2.5), "mean of 1..=4 is 2.5, got {:?}", s.mean);
    assert_eq!(s.minimum, Some(1.0), "minimum seen so far");
    assert_eq!(s.maximum, Some(4.0), "maximum seen so far");
}

#[test]
fn part5_incremental_mean_survives_a_large_offset() {
    let mut rs = RunningStats::new();
    rs.extend([1e9 + 1.0, 1e9 + 2.0, 1e9 + 3.0]);
    assert!(
        close_enough(rs.snapshot().mean, 1e9 + 2.0),
        "mean must stay accurate with a large common offset"
    );
}

#[test]
fn part5_collect_into_stats_and_extend_accumulate() {
    let s: Stats = [3.0, -1.0, 4.0].into_iter().collect();
    assert_eq!(s.count, 3, "collect counts every value");
    assert_eq!((s.minimum, s.maximum), (Some(-1.0), Some(4.0)), "collect tracks extremes");
    assert!(close_enough(s.mean, 2.0), "collect computes the mean");

    let mut rs = RunningStats::new();
    rs.push(10.0);
    rs.extend(vec![20.0, 30.0]);
    assert_eq!(rs.snapshot().count, 3, "extend must fold into the existing state");
}

#[test]
fn part5_reset_clears_state_but_keeps_the_finaliser() {
    let (seen, on_close) = recorder();
    let mut rs = RunningStats::with_on_close(on_close);
    rs.push(100.0);
    rs.reset();
    assert_eq!(rs.snapshot().count, 0, "reset returns to the empty state");
    rs.push(10.0);
    drop(rs);
    let finals = seen.borrow();
    assert_eq!(finals.len(), 1, "the finaliser fires once, on drop");
    assert_eq!(finals[0].count, 1, "the finaliser sees the state after the reset");
    assert!(close_enough(finals[0].mean, 10.0), "reset must discard the earlier 100.0");
}

#[test]
fn part5_finaliser_runs_on_early_return() {
    fn bail(seen_on_close: impl FnOnce(Stats) + 'static) -> Result<(), &'static str> {
        let mut rs = RunningStats::with_on_close(seen_on_close);
        rs.push(2.0);
        Err("bail")
    }
    let (seen, on_close) = recorder();
    assert_eq!(bail(on_close), Err("bail"));
    assert_eq!(seen.borrow().len(), 1, "an early return must still run the finaliser exactly once");
}

#[test]
fn part5_finaliser_runs_during_unwind() {
    let (seen, on_close) = recorder();
    let result = catch_unwind(AssertUnwindSafe(move || {
        let mut rs = RunningStats::with_on_close(on_close);
        rs.push(1.0);
        panic!("boom");
    }));
    assert!(result.is_err(), "the closure must have panicked");
    assert_eq!(seen.borrow().len(), 1, "unwinding must still run the finaliser exactly once");
}
