mod common;

use common::{all_text, record, Captured, Recording};
use ex10_logging_tracing::payments::{charge_card, quote_total, ChargeError};
use tracing::Level;

fn charge(order_id: &str, amount: i64, token: &str) -> (Result<String, ChargeError>, Recording) {
    let mut out = None;
    let rec = record(|| out = Some(charge_card(order_id, amount, token)));
    (out.expect("closure ran"), rec)
}

fn find_level(events: &[Captured], level: Level) -> &Captured {
    events
        .iter()
        .find(|e| e.level == level)
        .unwrap_or_else(|| panic!("no {level} event; got {events:#?}"))
}

#[test]
fn part1_success_emits_debug_then_info_events() {
    let (result, rec) = charge("o-1", 1500, "tok_ok");
    assert_eq!(result, Ok("txn_o-1_1500".to_owned()), "charge_card should return the txn id");
    let levels: Vec<Level> = rec.events.iter().map(|e| e.level).collect();
    assert_eq!(levels, vec![Level::DEBUG, Level::INFO], "expected DEBUG 'authorising' then INFO 'charged'");
}

#[test]
fn part1_info_event_renders_txn_id() {
    let (_, rec) = charge("o-1", 1500, "tok_ok");
    let info = find_level(&rec.events, Level::INFO);
    assert_eq!(info.message(), "charged", "INFO event message");
    assert_eq!(info.field("txn_id"), Some("txn_o-1_1500"), "txn_id field should be the Display of the txn");
}

#[test]
fn part1_debug_event_carries_order_and_amount_fields() {
    let (_, rec) = charge("o-1", 1500, "tok_ok");
    let debug = find_level(&rec.events, Level::DEBUG);
    assert_eq!(debug.message(), "authorising", "DEBUG event message");
    assert_eq!(debug.field("order_id"), Some("o-1"), "order_id field");
    assert_eq!(debug.field("amount_cents"), Some("1500"), "amount_cents field");
}

#[test]
fn part1_events_use_the_module_path_as_target() {
    let (_, rec) = charge("o-1", 1500, "tok_ok");
    for e in &rec.events {
        assert_eq!(e.target, "ex10_logging_tracing::payments", "default target is the module path");
    }
}

#[test]
fn part1_instrument_opens_span_named_after_fn() {
    let (_, rec) = charge("o-1", 1500, "tok_ok");
    let debug = find_level(&rec.events, Level::DEBUG);
    assert_eq!(debug.scope_names(), vec!["charge_card"], "event should be inside the charge_card span");
}

#[test]
fn part1_instrument_records_arguments_as_span_fields() {
    let (_, rec) = charge("o-7", 250, "tok_ok");
    let span = &find_level(&rec.events, Level::INFO).scope[0];
    assert_eq!(span.fields.get("order_id").map(String::as_str), Some("o-7"), "order_id span field");
    assert_eq!(span.fields.get("amount_cents").map(String::as_str), Some("250"), "amount_cents span field");
}

#[test]
fn part1_instrument_skip_keeps_card_token_out_of_telemetry() {
    let (_, rec) = charge("o-1", 1500, "tok_ok");
    let text = all_text(&rec);
    assert!(!text.contains("card_token"), "card_token must be skipped, found in:\n{text}");
    assert!(!text.contains("tok_ok"), "the card token value leaked into telemetry:\n{text}");
}

#[test]
fn part1_instrument_fields_attribute_adds_static_provider_field() {
    let (_, rec) = charge("o-1", 1500, "tok_ok");
    let span = &find_level(&rec.events, Level::INFO).scope[0];
    assert_eq!(span.fields.get("provider").map(String::as_str), Some("sim-gateway"), "static provider field");
}

#[test]
fn part1_err_attribute_emits_error_event_with_error_field() {
    let (result, rec) = charge("o-2", 0, "tok");
    assert_eq!(result, Err(ChargeError::InvalidAmount(0)));
    let err = find_level(&rec.events, Level::ERROR);
    assert_eq!(err.field("error"), Some("amount must be positive, got 0"), "error field should be the Display of the error");
    assert_eq!(err.scope_names(), vec!["charge_card"], "error event should be inside the span");
}

#[test]
fn part1_err_attribute_is_silent_on_success() {
    let (_, rec) = charge("o-1", 1500, "tok_ok");
    assert!(
        rec.events.iter().all(|e| e.level != Level::ERROR),
        "no ERROR event expected on success, got {:#?}",
        rec.events
    );
}

#[test]
fn part1_declined_card_returns_typed_error() {
    let (result, _) = charge("o-3", 900, "decline-me");
    assert_eq!(
        result,
        Err(ChargeError::Declined { reason: "do_not_honor".to_owned() }),
        "a decline token should map to Declined"
    );
}

#[test]
fn part1_declined_card_logs_error_with_display_reason() {
    let (_, rec) = charge("o-3", 900, "decline-me");
    let err = find_level(&rec.events, Level::ERROR);
    assert_eq!(err.field("error"), Some("card declined: do_not_honor"), "error field should use Display");
}

#[test]
fn part1_gateway_down_maps_to_unavailable() {
    let (result, _) = charge("o-4", 900, "down-1");
    assert_eq!(result, Err(ChargeError::GatewayUnavailable), "a 'down' token should map to GatewayUnavailable");
}

#[test]
fn part1_rejects_non_positive_amount_with_value() {
    let (result, _) = charge("o-5", -5, "tok_ok");
    assert_eq!(result, Err(ChargeError::InvalidAmount(-5)), "negative amounts must be rejected");
}

#[test]
fn part1_quote_total_sums_items() {
    assert_eq!(quote_total(&[100, 250, 50]), 400, "sum of item prices");
}

#[test]
fn part1_ret_attribute_records_return_value_on_event() {
    let mut total = 0;
    let rec = record(|| total = quote_total(&[100, 250, 50]));
    assert_eq!(total, 400, "function result");
    let ret_event = rec.events.iter().find(|e| e.field("return").is_some());
    let ret_event = ret_event.unwrap_or_else(|| panic!("no event carries a `return` field; got {:#?}", rec.events));
    assert_eq!(ret_event.field("return"), Some("400"), "return field should hold the value");
}

#[test]
fn part1_ret_event_is_inside_quote_total_span() {
    let rec = record(|| {
        quote_total(&[1, 2]);
    });
    let ret_event = rec
        .events
        .iter()
        .find(|e| e.field("return").is_some())
        .unwrap_or_else(|| panic!("no `return` event; got {:#?}", rec.events));
    assert_eq!(ret_event.scope_names(), vec!["quote_total"], "return event should be in the quote_total span");
}

#[test]
fn part1_works_without_any_subscriber() {
    assert_eq!(charge_card("o-6", 10, "tok_ok"), Ok("txn_o-6_10".to_owned()), "no subscriber installed: must still work");
}
