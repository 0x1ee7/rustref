//! Part 7 — time-ordered k-way merge and bounded top-k with `BinaryHeap`.

use std::cell::Cell;

use ex05_iterators::log::{LogRecord, parse_line};
use ex05_iterators::merge::{merge_streams, top_servers};

fn rec(ts: &str, server: &str) -> LogRecord {
    parse_line(&format!("{ts} INFO {server} m")).expect("fixture line must parse")
}

fn servers(records: &[LogRecord]) -> Vec<&str> {
    records.iter().map(|r| r.server.as_str()).collect()
}

#[test]
fn part7_merge_interleaves_sorted_streams_by_timestamp() {
    let a = vec![rec("2026-03-01T00:00:01Z", "a1"), rec("2026-03-01T00:00:03Z", "a3"), rec("2026-03-01T00:00:05Z", "a5")];
    let b = vec![rec("2026-03-01T00:00:02Z", "b2"), rec("2026-03-01T00:00:04Z", "b4")];
    let merged: Vec<LogRecord> = merge_streams([a.into_iter(), b.into_iter()]).collect();
    assert_eq!(servers(&merged), vec!["a1", "b2", "a3", "b4", "a5"], "output is globally ordered by ts");
}

#[test]
fn part7_merge_breaks_timestamp_ties_by_stream_order() {
    let ts = "2026-03-01T00:00:00Z";
    let first = vec![rec(ts, "first")];
    let second = vec![rec(ts, "second")];
    let merged: Vec<LogRecord> = merge_streams([second.into_iter(), first.into_iter()]).collect();
    assert_eq!(servers(&merged), vec!["second", "first"], "on equal timestamps the earlier argument must come first");
}

#[test]
fn part7_merge_of_zero_streams_is_empty() {
    let none = merge_streams(Vec::<std::vec::IntoIter<LogRecord>>::new());
    assert_eq!(none.count(), 0, "no streams means nothing to yield");
}

#[test]
fn part7_merge_works_on_infinite_streams() {
    let evens: Box<dyn Iterator<Item = LogRecord>> =
        Box::new((0..).map(|i: u32| rec(&format!("2026-03-01T00:00:{:02}Z", i * 2), "even")));
    let odds: Box<dyn Iterator<Item = LogRecord>> =
        Box::new((0..).map(|i: u32| rec(&format!("2026-03-01T00:00:{:02}Z", i * 2 + 1), "odd")));
    let first: Vec<LogRecord> = merge_streams([evens, odds]).take(5).collect();
    let ts: Vec<String> = first.iter().map(|r| r.ts.to_string()).collect();
    assert_eq!(ts.len(), 5, "five records from two infinite streams");
    assert!(ts.windows(2).all(|w| w[0] < w[1]), "the infinite merge must stay ordered: {ts:?}");
}

#[test]
fn part7_merge_pulls_lazily_from_its_inputs() {
    let pulls = Cell::new(0usize);
    let counted = |label: &'static str, step: u32| {
        let pulls = &pulls;
        (0u32..).map(move |i| {
            pulls.set(pulls.get() + 1);
            rec(&format!("2026-03-01T00:00:{:02}Z", i * step), label)
        })
    };
    let mut merged = merge_streams([counted("a", 2), counted("b", 3)]);
    assert_eq!(pulls.get(), 0, "constructing the merge must not pull from any stream");
    merged.next().expect("first record");
    assert!(pulls.get() <= 2, "one record may prime each stream, but nothing more");
    merged.next().expect("second record");
    assert!(pulls.get() <= 3, "each further record pulls from at most one stream");
}

#[test]
fn part7_top_servers_orders_by_count_then_name() {
    let records = vec![
        rec("2026-03-01T00:00:00Z", "web-1"),
        rec("2026-03-01T00:00:01Z", "web-1"),
        rec("2026-03-01T00:00:02Z", "web-1"),
        rec("2026-03-01T00:00:03Z", "db-1"),
        rec("2026-03-01T00:00:04Z", "db-1"),
        rec("2026-03-01T00:00:05Z", "db-1"),
        rec("2026-03-01T00:00:06Z", "cache"),
        rec("2026-03-01T00:00:07Z", "cache"),
        rec("2026-03-01T00:00:08Z", "web-2"),
    ];
    assert_eq!(
        top_servers(records, 3),
        vec![("db-1".to_string(), 3), ("web-1".to_string(), 3), ("cache".to_string(), 2)],
        "highest count first; equal counts ordered by name ascending"
    );
}

#[test]
fn part7_top_servers_edge_cases() {
    let records = vec![rec("2026-03-01T00:00:00Z", "a")];
    assert_eq!(top_servers(records.clone(), 0), vec![], "k == 0 yields nothing");
    assert_eq!(top_servers(records, 10), vec![("a".to_string(), 1)], "k larger than the server count returns all servers");
    assert_eq!(top_servers(Vec::new(), 3), vec![], "empty input yields nothing");
}
