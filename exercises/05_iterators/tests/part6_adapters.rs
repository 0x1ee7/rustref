//! Part 6 — extension traits, adapters, slice iterators and `successors`.

use std::cell::Cell;
use std::time::Duration;

use ex05_iterators::adapters::{
    IterExt, gaps, group_by_server, leading_below, retry_delays, running_peak,
};
use ex05_iterators::log::{LogRecord, parse_line};
use jiff::{SignedDuration, Timestamp};

fn rec(ts: &str, server: &str) -> LogRecord {
    parse_line(&format!("{ts} INFO {server} m")).expect("fixture line must parse")
}

#[test]
fn part6_chunked_groups_with_a_short_tail() {
    let got: Vec<Vec<i32>> = (1..=5).chunked(2).collect();
    assert_eq!(got, vec![vec![1, 2], vec![3, 4], vec![5]], "chunks of two with a short last chunk");
}

#[test]
fn part6_chunked_exact_multiple_and_empty_input() {
    assert_eq!((1..=4).chunked(2).count(), 2, "no empty trailing chunk after an exact multiple");
    assert_eq!(std::iter::empty::<u8>().chunked(3).next(), None, "empty input yields nothing");
}

#[test]
fn part6_chunked_is_lazy_over_infinite_input() {
    let pulls = Cell::new(0);
    let source = (1..).inspect(|_| pulls.set(pulls.get() + 1));
    let first = source.chunked(3).next();
    assert_eq!(first, Some(vec![1, 2, 3]), "first chunk of an infinite source");
    assert_eq!(pulls.get(), 3, "a chunk of three must pull exactly three items");
}

#[test]
fn part6_chunked_size_hint_is_exact_for_exact_inputs() {
    assert_eq!((0..10).chunked(3).size_hint(), (4, Some(4)), "ceil(10 / 3) chunks, known exactly");
}

#[test]
#[should_panic(expected = "must be at least 1")]
fn part6_chunked_zero_size_panics() {
    let _ = (0..3).chunked(0);
}

#[test]
fn part6_moving_average_emits_only_full_windows() {
    let got: Vec<f64> = [1.0, 2.0, 3.0, 4.0, 5.0].into_iter().moving_average(2).collect();
    assert_eq!(got, vec![1.5, 2.5, 3.5, 4.5], "means of each full window of two");
    assert_eq!([1.0, 2.0].into_iter().moving_average(3).count(), 0, "a window longer than the input emits nothing");
}

#[test]
fn part6_moving_average_is_lazy_over_infinite_input() {
    let got: Vec<f64> = (1..).map(f64::from).moving_average(3).take(2).collect();
    assert_eq!(got, vec![2.0, 3.0], "an infinite source must be consumed only as far as asked");
}

#[test]
#[should_panic(expected = "window must be at least 1")]
fn part6_moving_average_zero_window_panics() {
    let _ = [1.0].into_iter().moving_average(0);
}

#[test]
fn part6_leading_below_stops_before_the_breach() {
    let pulls = Cell::new(0);
    let source = [1.0, 2.0, 5.0, 0.0, 1.0].into_iter().inspect(|_| pulls.set(pulls.get() + 1));
    assert_eq!(leading_below(source, 3.0), vec![1.0, 2.0], "values strictly before the first >= threshold");
    assert_eq!(pulls.get(), 3, "the breaching value is the last one read; nothing after it");
}

#[test]
fn part6_running_peak_is_the_running_maximum() {
    let got: Vec<f64> = running_peak([1.0, 3.0, 2.0, 5.0, 4.0]).collect();
    assert_eq!(got, vec![1.0, 3.0, 3.0, 5.0, 5.0], "each output is the max so far");
    assert_eq!(running_peak(Vec::<f64>::new()).count(), 0, "empty input gives empty output");
}

#[test]
fn part6_gaps_are_adjacent_differences() {
    let ts: Vec<Timestamp> = ["2026-03-01T00:00:00Z", "2026-03-01T00:01:00Z", "2026-03-01T00:03:00Z"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let got: Vec<SignedDuration> = gaps(&ts).collect();
    assert_eq!(
        got,
        vec![SignedDuration::from_secs(60), SignedDuration::from_secs(120)],
        "later - earlier for each adjacent pair"
    );
    assert_eq!(gaps(&ts[..1]).count(), 0, "fewer than two timestamps yields nothing");
}

#[test]
fn part6_group_by_server_only_merges_adjacent_runs() {
    let records = vec![
        rec("2026-03-01T00:00:00Z", "web-1"),
        rec("2026-03-01T00:00:01Z", "web-1"),
        rec("2026-03-01T00:00:02Z", "web-2"),
        rec("2026-03-01T00:00:03Z", "web-1"),
    ];
    let groups: Vec<(&str, usize)> = group_by_server(&records)
        .map(|g| (g[0].server.as_str(), g.len()))
        .collect();
    assert_eq!(
        groups,
        vec![("web-1", 2), ("web-2", 1), ("web-1", 1)],
        "non-adjacent records of the same server form separate groups"
    );
}

#[test]
fn part6_retry_delays_double_and_clamp_to_cap() {
    let got: Vec<Duration> = retry_delays(Duration::from_millis(100), Duration::from_secs(1)).take(6).collect();
    let ms: Vec<u128> = got.iter().map(Duration::as_millis).collect();
    assert_eq!(ms, vec![100, 200, 400, 800, 1000, 1000], "doubling until the cap, then flat");
}

#[test]
fn part6_retry_delays_clamp_a_base_above_the_cap() {
    let first = retry_delays(Duration::from_secs(5), Duration::from_secs(1)).next();
    assert_eq!(first, Some(Duration::from_secs(1)), "the first delay is the base clamped to the cap");
}

#[test]
fn part6_retry_delays_never_overflow() {
    let last = retry_delays(Duration::from_secs(1), Duration::MAX).take(200).last();
    assert_eq!(last, Some(Duration::MAX), "after enough doublings the delay saturates at the cap instead of panicking");
}
