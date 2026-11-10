use std::thread::sleep;
use std::time::Duration;

use ex10_logging_tracing::layers::{EventCounter, LevelCounts, RedactedEvents, SpanTiming, SpanTimings};
use tracing::{debug, error, info, info_span, trace, warn};
use tracing_subscriber::layer::{Layer, SubscriberExt};
use tracing_subscriber::Registry;

fn scoped<L, F>(layer: L, f: F)
where
    L: Layer<Registry> + Send + Sync + 'static,
    F: FnOnce(),
{
    tracing::subscriber::with_default(Registry::default().with(layer), f);
}

fn timing<'a>(timings: &'a [SpanTiming], name: &str) -> &'a SpanTiming {
    timings
        .iter()
        .find(|t| t.name == name)
        .unwrap_or_else(|| panic!("no timing for span {name:?}; got {timings:#?}"))
}

#[test]
fn part5_counter_counts_each_level_once_per_event() {
    let counter = EventCounter::new();
    scoped(counter.clone(), || {
        trace!("t");
        debug!("d1");
        debug!("d2");
        info!("i");
        warn!("w");
        error!("e");
    });
    assert_eq!(
        counter.snapshot(),
        LevelCounts { trace: 1, debug: 2, info: 1, warn: 1, error: 1 },
        "per-level totals"
    );
}

#[test]
fn part5_counter_ignores_spans() {
    let counter = EventCounter::new();
    scoped(counter.clone(), || {
        let s = info_span!("x");
        let _g = s.enter();
    });
    assert_eq!(counter.snapshot(), LevelCounts::default(), "spans must not be counted as events");
}

#[test]
fn part5_snapshot_is_a_point_in_time_copy() {
    let counter = EventCounter::new();
    scoped(counter.clone(), || info!("a"));
    let before = counter.snapshot();
    scoped(counter.clone(), || info!("b"));
    assert_eq!(before.info, 1, "an earlier snapshot must not change");
    assert_eq!(counter.snapshot().info, 2, "a later snapshot sees both events");
}

#[test]
fn part5_span_timing_records_busy_time_when_closed() {
    let timings = SpanTimings::new();
    scoped(timings.clone(), || {
        let s = info_span!("render");
        let _g = s.enter();
        sleep(Duration::from_millis(30));
    });
    let finished = timings.finished();
    let render = timing(&finished, "render");
    assert!(render.busy >= Duration::from_millis(25), "busy time should cover the sleep, got {:?}", render.busy);
}

#[test]
fn part5_span_busy_time_excludes_idle_time() {
    let timings = SpanTimings::new();
    scoped(timings.clone(), || {
        let s = info_span!("quick");
        {
            let _g = s.enter();
        }
        sleep(Duration::from_millis(60));
    });
    let finished = timings.finished();
    let quick = timing(&finished, "quick");
    assert!(quick.busy < Duration::from_millis(50), "time spent alive but not entered must be excluded, got {:?}", quick.busy);
}

#[test]
fn part5_busy_time_accumulates_across_reentries() {
    let timings = SpanTimings::new();
    scoped(timings.clone(), || {
        let s = info_span!("loop");
        for _ in 0..2 {
            let _g = s.enter();
            sleep(Duration::from_millis(15));
        }
    });
    let finished = timings.finished();
    let looped = timing(&finished, "loop");
    assert!(looped.busy >= Duration::from_millis(25), "two 15ms entries should sum to ≥25ms, got {:?}", looped.busy);
}

#[test]
fn part5_span_timing_reports_parent_name() {
    let timings = SpanTimings::new();
    scoped(timings.clone(), || {
        let outer = info_span!("outer");
        let _o = outer.enter();
        let inner = info_span!("inner");
        let _i = inner.enter();
    });
    let finished = timings.finished();
    assert_eq!(timing(&finished, "inner").parent.as_deref(), Some("outer"), "inner's parent should be outer");
    assert_eq!(timing(&finished, "outer").parent, None, "outer has no parent");
}

#[test]
fn part5_each_span_is_recorded_once_on_close() {
    let timings = SpanTimings::new();
    scoped(timings.clone(), || {
        let outer = info_span!("outer");
        let _o = outer.enter();
        let inner = info_span!("inner");
        let _i = inner.enter();
    });
    assert_eq!(timings.finished().len(), 2, "two spans closed, two timings expected");
}

#[test]
fn part5_redactor_masks_secret_fields_and_keeps_others() {
    let sink = RedactedEvents::new();
    scoped(sink.clone(), || info!(token = "abc123", user = "bob", "login"));
    assert_eq!(sink.lines(), vec!["INFO login token=*** user=bob".to_owned()], "token masked, user kept");
}

#[test]
fn part5_redactor_masks_non_string_secret_values() {
    let sink = RedactedEvents::new();
    scoped(sink.clone(), || info!(password = 42, "auth"));
    assert_eq!(sink.lines(), vec!["INFO auth password=***".to_owned()], "a numeric secret must be masked too");
}

#[test]
fn part5_redactor_masks_debug_formatted_values() {
    let sink = RedactedEvents::new();
    scoped(sink.clone(), || info!(authorization = ?"Bearer abc", "outgoing"));
    assert_eq!(sink.lines(), vec!["INFO outgoing authorization=***".to_owned()], "Debug-recorded secret must be masked");
}

#[test]
fn part5_redactor_masks_every_secret_name() {
    let sink = RedactedEvents::new();
    scoped(sink.clone(), || {
        info!(password = "x", secret = "x", api_key = "x", authorization = "x", card_token = "x", token = "x", "all");
    });
    assert_eq!(
        sink.lines(),
        vec!["INFO all password=*** secret=*** api_key=*** authorization=*** card_token=*** token=***".to_owned()],
        "every name in SECRET_FIELDS must be masked"
    );
}

#[test]
fn part5_redactor_keeps_non_secret_fields_verbatim() {
    let sink = RedactedEvents::new();
    scoped(sink.clone(), || info!(attempts = 3, order_id = "o-1", "retrying"));
    assert_eq!(sink.lines(), vec!["INFO retrying attempts=3 order_id=o-1".to_owned()], "non-secret fields render as-is");
}

#[test]
fn part5_redactor_renders_message_only_when_no_fields() {
    let sink = RedactedEvents::new();
    scoped(sink.clone(), || info!("hello"));
    assert_eq!(sink.lines(), vec!["INFO hello".to_owned()], "message-only line has no trailing space");
}

#[test]
fn part5_redactor_keeps_one_line_per_event_in_order() {
    let sink = RedactedEvents::new();
    scoped(sink.clone(), || {
        debug!("first");
        warn!(order_id = "o-2", "second");
    });
    assert_eq!(
        sink.lines(),
        vec!["DEBUG first".to_owned(), "WARN second order_id=o-2".to_owned()],
        "one line per event, in emission order"
    );
}

#[test]
fn part5_layers_compose_on_one_registry() {
    let counter = EventCounter::new();
    let sink = RedactedEvents::new();
    let sub = Registry::default().with(counter.clone()).with(sink.clone());
    tracing::subscriber::with_default(sub, || info!(token = "t", "login"));
    assert_eq!(counter.snapshot().info, 1, "counter should see the event");
    assert_eq!(sink.lines(), vec!["INFO login token=***".to_owned()], "redactor should see the same event");
}
