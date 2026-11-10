mod common;

use std::num::NonZeroUsize;

use common::{record, Captured};
use ex10_logging_tracing::job::{settle_batch, SettlementReport};
use ex10_logging_tracing::json::CaptureWriter;
use ex10_logging_tracing::layers::EventCounter;
use ex10_logging_tracing::stack::app_stack;
use serde_json::Value;
use tracing::{debug, info, info_span, Level};
use tracing_subscriber::EnvFilter;

fn workers(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).expect("non-zero")
}

fn worker_events(events: &[Captured]) -> Vec<&Captured> {
    events
        .iter()
        .filter(|e| e.message() == "settled" || e.message() == "rejected")
        .collect()
}

#[test]
fn part6_app_stack_writes_one_json_line_per_enabled_event() {
    let writer = CaptureWriter::new();
    let counter = EventCounter::new();
    let sub = app_stack(EnvFilter::new("info"), counter, writer.clone());
    tracing::subscriber::with_default(sub, || {
        debug!("hidden");
        info!(order_id = "o-1", "shown");
    });
    let raw = writer.contents();
    assert_eq!(raw.lines().count(), 1, "only the INFO event should be written, got {raw:?}");
    let v: Value = serde_json::from_str(raw.trim()).expect("line should be JSON");
    assert_eq!(v["fields"]["message"], "shown", "the INFO event should be the one written");
}

#[test]
fn part6_app_stack_counter_sees_only_enabled_events() {
    let writer = CaptureWriter::new();
    let counter = EventCounter::new();
    let sub = app_stack(EnvFilter::new("info"), counter.clone(), writer);
    tracing::subscriber::with_default(sub, || {
        debug!("hidden");
        info!("shown");
    });
    let snap = counter.snapshot();
    assert_eq!(snap.info, 1, "INFO should be counted");
    assert_eq!(snap.debug, 0, "a filtered DEBUG event must not reach the counter");
}

#[test]
fn part6_app_stack_json_carries_request_span_fields() {
    let writer = CaptureWriter::new();
    let sub = app_stack(EnvFilter::new("info"), EventCounter::new(), writer.clone());
    tracing::subscriber::with_default(sub, || {
        let span = info_span!("http.request", request_id = "req-9");
        let _g = span.enter();
        info!("ok");
    });
    let v: Value = serde_json::from_str(writer.contents().trim()).expect("line should be JSON");
    assert_eq!(v["span"]["request_id"], "req-9", "span context must appear in the JSON line");
}

#[test]
fn part6_settle_batch_totals_positive_amounts_and_counts_rejects() {
    let report = settle_batch("b-1", &[100, 250, -5, 0, 400], workers(2));
    assert_eq!(
        report,
        SettlementReport { batch_id: "b-1".to_owned(), settled: 3, rejected: 2, total_cents: 750 },
        "positive amounts settle, non-positive are rejected"
    );
}

#[test]
fn part6_empty_batch_reports_zeroes() {
    let report = settle_batch("b-empty", &[], workers(4));
    assert_eq!(
        report,
        SettlementReport { batch_id: "b-empty".to_owned(), settled: 0, rejected: 0, total_cents: 0 },
        "empty batch"
    );
}

#[test]
fn part6_more_workers_than_items_still_counts_everything() {
    let report = settle_batch("b-2", &[10, 20], workers(8));
    assert_eq!(report.settled, 2, "both amounts settle");
    assert_eq!(report.total_cents, 30, "total of both amounts");
}

#[test]
fn part6_report_does_not_depend_on_worker_count() {
    let amounts = [5, -1, 7, 9, -3, 11, 2];
    let baseline = settle_batch("b-3", &amounts, workers(1));
    for n in 2..=5 {
        assert_eq!(settle_batch("b-3", &amounts, workers(n)), baseline, "report must not change with {n} workers");
    }
}

#[test]
fn part6_worker_events_are_children_of_batch_span() {
    let rec = record(|| {
        settle_batch("b-7", &[100, -5], workers(2));
    });
    let events = worker_events(&rec.events);
    assert!(events.len() >= 2, "expected one event per amount, got {:#?}", rec.events);
    for e in events {
        assert_eq!(e.scope.len(), 2, "scope should be [settle_batch, settle.worker], got {:?}", e.scope_names());
        assert_eq!(e.scope[0].name, "settle_batch", "outer span");
        assert_eq!(e.scope[0].fields.get("batch_id").map(String::as_str), Some("b-7"), "batch_id on outer span");
        assert_eq!(e.scope[1].name, "settle.worker", "inner span");
    }
}

#[test]
fn part6_worker_span_records_its_index() {
    let rec = record(|| {
        settle_batch("b-7", &[100, -5], workers(2));
    });
    let mut indices: Vec<String> = worker_events(&rec.events)
        .iter()
        .filter_map(|e| e.scope.get(1).and_then(|s| s.fields.get("worker")).cloned())
        .collect();
    indices.sort();
    indices.dedup();
    assert_eq!(indices, vec!["0".to_owned(), "1".to_owned()], "two workers should be numbered 0 and 1");
}

#[test]
fn part6_rejected_amount_warns_with_amount_field() {
    let rec = record(|| {
        settle_batch("b-8", &[-5], workers(1));
    });
    let warn = rec
        .events
        .iter()
        .find(|e| e.level == Level::WARN)
        .unwrap_or_else(|| panic!("no WARN event; got {:#?}", rec.events));
    assert_eq!(warn.message(), "rejected", "WARN message");
    assert_eq!(warn.field("amount_cents"), Some("-5"), "amount_cents on the WARN");
}

#[test]
fn part6_settle_batch_without_subscriber_still_settles() {
    let report = settle_batch("b-9", &[1, 2], workers(2));
    assert_eq!(report.total_cents, 3, "settling must work with no subscriber installed");
}
