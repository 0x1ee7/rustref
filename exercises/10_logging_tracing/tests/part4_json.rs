use std::io::Write;

use ex10_logging_tracing::json::{json_subscriber, CaptureWriter};
use serde_json::Value;
use tracing::{debug, error, info, info_span, trace, warn};
use tracing_subscriber::fmt::MakeWriter;

/// Runs `f` under the JSON subscriber; returns the raw output and the parsed lines.
fn emit(f: impl FnOnce()) -> (String, Vec<Value>) {
    let writer = CaptureWriter::new();
    tracing::subscriber::with_default(json_subscriber(writer.clone()), f);
    let raw = writer.contents();
    let values = raw
        .lines()
        .map(|line| serde_json::from_str(line).unwrap_or_else(|e| panic!("line is not JSON ({e}): {line}")))
        .collect();
    (raw, values)
}

#[test]
fn part4_each_event_is_one_json_object_per_line() {
    let (_, values) = emit(|| {
        info!("a");
        info!("b");
        error!("c");
    });
    assert_eq!(values.len(), 3, "one line per event, got {values:#?}");
    assert!(values.iter().all(Value::is_object), "every line should be a JSON object");
}

#[test]
fn part4_top_level_keys_carry_level_target_and_message() {
    let (_, values) = emit(|| info!(target: "shop::http", "request done"));
    let v = &values[0];
    assert_eq!(v["level"], "INFO", "level key");
    assert_eq!(v["target"], "shop::http", "target key");
    assert_eq!(v["fields"]["message"], "request done", "message lives under fields");
}

#[test]
fn part4_structured_fields_nest_under_fields_with_types_kept() {
    let (_, values) = emit(|| info!(target: "shop::payments", order_id = "o-1", amount_cents = 1500, "charged"));
    let f = &values[0]["fields"];
    assert_eq!(f["order_id"], "o-1", "string field");
    assert_eq!(f["amount_cents"].as_i64(), Some(1500), "integer field should stay a JSON number");
}

#[test]
fn part4_current_span_fields_appear_under_span_key() {
    let (_, values) = emit(|| {
        let span = info_span!("http.request", request_id = "req-7");
        let _g = span.enter();
        info!("inside");
    });
    let span = &values[0]["span"];
    assert_eq!(span["name"], "http.request", "span.name");
    assert_eq!(span["request_id"], "req-7", "span fields are flattened under span");
}

#[test]
fn part4_span_list_is_ordered_outermost_first() {
    let (_, values) = emit(|| {
        let outer = info_span!("outer");
        let _o = outer.enter();
        let inner = info_span!("inner");
        let _i = inner.enter();
        info!("deep");
    });
    let spans = values[0]["spans"].as_array().unwrap_or_else(|| panic!("spans should be an array: {}", values[0]));
    let names: Vec<&str> = spans.iter().filter_map(|s| s["name"].as_str()).collect();
    assert_eq!(names, vec!["outer", "inner"], "span list should be outermost first");
}

#[test]
fn part4_error_level_is_spelled_upper_case() {
    let (_, values) = emit(|| error!("boom"));
    assert_eq!(values[0]["level"], "ERROR", "ERROR level name");
}

#[test]
fn part4_timestamp_is_present_as_string() {
    let (_, values) = emit(|| info!("tick"));
    assert!(values[0]["timestamp"].is_string(), "timestamp should be a string, got {}", values[0]);
}

#[test]
fn part4_output_contains_no_ansi_escapes() {
    let (raw, _) = emit(|| warn!(code = 7, "careful"));
    assert!(!raw.contains('\u{1b}'), "JSON output must not contain ANSI escapes: {raw:?}");
}

#[test]
fn part4_all_levels_are_emitted_without_filter() {
    let (_, values) = emit(|| {
        trace!("t");
        debug!("d");
        info!("i");
        warn!("w");
        error!("e");
    });
    let levels: Vec<&str> = values.iter().filter_map(|v| v["level"].as_str()).collect();
    assert_eq!(levels, vec!["TRACE", "DEBUG", "INFO", "WARN", "ERROR"], "json_subscriber must not filter");
}

#[test]
fn part4_capture_writer_clones_share_one_buffer() {
    let w = CaptureWriter::new();
    let mut c = w.clone();
    c.write_all(b"hello\n").expect("write");
    assert_eq!(w.contents(), "hello\n", "a clone's writes must be visible through the original");
}

#[test]
fn part4_make_writer_hands_out_writers_on_the_same_buffer() {
    let w = CaptureWriter::new();
    {
        let mut first = w.make_writer();
        first.write_all(b"a\n").expect("write");
    }
    {
        let mut second = w.make_writer();
        second.write_all(b"b\n").expect("write");
    }
    assert_eq!(w.contents(), "a\nb\n", "writers from make_writer should append to the same buffer");
}

#[test]
fn part4_contents_is_empty_before_any_event() {
    assert!(CaptureWriter::new().contents().is_empty(), "a fresh buffer must be empty");
}
