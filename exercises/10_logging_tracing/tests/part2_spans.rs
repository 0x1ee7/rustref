mod common;

use common::{record, Captured, Recording, SpanSnapshot};
use ex10_logging_tracing::http::{handle_request, Method, Order, OrderStore, Request, Response};
use tracing::Level;

fn store() -> OrderStore {
    let mut s = OrderStore::new();
    s.insert(Order { id: "o-1".to_owned(), amount_cents: 1500 });
    s
}

fn req(id: &str, method: Method, path: &str) -> Request {
    Request { request_id: id.to_owned(), method, path: path.to_owned() }
}

fn serve(requests: &[Request]) -> (Vec<Response>, Recording) {
    let store = store();
    let mut responses = Vec::new();
    let rec = record(|| {
        for r in requests {
            responses.push(handle_request(r, &store));
        }
    });
    (responses, rec)
}

fn request_id(e: &Captured) -> Option<&str> {
    e.scope
        .iter()
        .find(|s| s.name == "http.request")
        .and_then(|s| s.fields.get("request_id"))
        .map(String::as_str)
}

fn closed_request_span(rec: &Recording) -> &SpanSnapshot {
    rec.closed
        .iter()
        .find(|s| s.name == "http.request")
        .unwrap_or_else(|| panic!("http.request span was not closed; closed = {:#?}", rec.closed))
}

fn with_message<'a>(rec: &'a Recording, msg: &str) -> Vec<&'a Captured> {
    rec.events.iter().filter(|e| e.message() == msg).collect()
}

#[test]
fn part2_request_span_carries_request_id_method_and_path() {
    let (_, rec) = serve(&[req("req-42", Method::Get, "/orders/o-1")]);
    let span = closed_request_span(&rec);
    assert_eq!(span.fields.get("request_id").map(String::as_str), Some("req-42"), "request_id on span");
    assert_eq!(span.fields.get("method").map(String::as_str), Some("GET"), "method on span");
    assert_eq!(span.fields.get("path").map(String::as_str), Some("/orders/o-1"), "path on span");
}

#[test]
fn part2_every_event_inherits_request_id_from_request_span() {
    let (_, rec) = serve(&[req("req-42", Method::Get, "/orders/o-1")]);
    assert!(!rec.events.is_empty(), "handler should emit events");
    for e in &rec.events {
        assert_eq!(
            request_id(e),
            Some("req-42"),
            "event {:?} should inherit request_id through the span tree",
            e.message()
        );
    }
}

#[test]
fn part2_store_span_is_child_of_request_span() {
    let (_, rec) = serve(&[req("req-42", Method::Get, "/orders/o-1")]);
    let lookup = with_message(&rec, "order lookup");
    assert_eq!(lookup.len(), 1, "exactly one lookup expected");
    assert_eq!(lookup[0].scope_names(), vec!["http.request", "store.load"], "store.load should nest in http.request");
    assert_eq!(lookup[0].scope[1].fields.get("order_id").map(String::as_str), Some("o-1"), "order_id on store.load span");
}

#[test]
fn part2_store_span_skips_the_store_argument() {
    let (_, rec) = serve(&[req("req-42", Method::Get, "/orders/o-1")]);
    let store_span = rec
        .closed
        .iter()
        .find(|s| s.name == "store.load")
        .unwrap_or_else(|| panic!("store.load span not closed; closed = {:#?}", rec.closed));
    assert!(!store_span.fields.contains_key("store"), "the store argument must be skipped, fields = {:?}", store_span.fields);
}

#[test]
fn part2_status_is_recorded_on_request_span_after_handling() {
    let (_, rec) = serve(&[req("req-42", Method::Get, "/orders/o-1")]);
    let span = closed_request_span(&rec);
    assert_eq!(span.fields.get("status").map(String::as_str), Some("200"), "status should be recorded late on the span");
}

#[test]
fn part2_success_response_body_describes_order() {
    let (responses, _) = serve(&[req("req-1", Method::Get, "/orders/o-1")]);
    assert_eq!(
        responses[0],
        Response { status: 200, body: "order o-1 amount=1500".to_owned() },
        "found order should be 200 with the summary body"
    );
}

#[test]
fn part2_missing_order_is_404_with_warning_carrying_order_id() {
    let (responses, rec) = serve(&[req("req-5", Method::Get, "/orders/o-9")]);
    assert_eq!(responses[0].status, 404, "missing order should be 404");
    assert_eq!(responses[0].body, "order not found", "404 body");
    let warn = with_message(&rec, "order not found");
    assert_eq!(warn.len(), 1, "one WARN event expected, got {:#?}", rec.events);
    assert_eq!(warn[0].level, Level::WARN, "missing order must log at WARN");
    assert_eq!(warn[0].field("order_id"), Some("o-9"), "order_id field on WARN");
    assert_eq!(request_id(warn[0]), Some("req-5"), "WARN should carry the request id");
    assert_eq!(closed_request_span(&rec).fields.get("status").map(String::as_str), Some("404"), "status recorded as 404");
}

#[test]
fn part2_non_get_on_order_is_405_and_skips_lookup() {
    let (responses, rec) = serve(&[req("req-6", Method::Delete, "/orders/o-1")]);
    assert_eq!(responses[0].status, 405, "DELETE on an order should be 405");
    assert_eq!(responses[0].body, "method not allowed", "405 body");
    assert!(with_message(&rec, "order lookup").is_empty(), "no store lookup expected for 405");
    assert_eq!(closed_request_span(&rec).fields.get("status").map(String::as_str), Some("405"), "status recorded as 405");
}

#[test]
fn part2_post_on_order_is_405() {
    let (responses, _) = serve(&[req("req-6", Method::Post, "/orders/o-1")]);
    assert_eq!(responses[0].status, 405, "POST on an order should be 405");
}

#[test]
fn part2_unknown_route_is_404_without_lookup() {
    let (responses, rec) = serve(&[req("req-7", Method::Get, "/health")]);
    assert_eq!(responses[0], Response { status: 404, body: "no route".to_owned() }, "unknown route");
    assert!(with_message(&rec, "order lookup").is_empty(), "no lookup expected for unknown route");
}

#[test]
fn part2_request_ids_do_not_leak_between_requests() {
    let (_, rec) = serve(&[req("req-1", Method::Get, "/orders/o-1"), req("req-2", Method::Get, "/orders/o-9")]);
    let lookups = with_message(&rec, "order lookup");
    assert_eq!(lookups.len(), 2, "one lookup per request expected");
    assert_eq!(request_id(lookups[0]), Some("req-1"), "first lookup belongs to req-1");
    assert_eq!(request_id(lookups[1]), Some("req-2"), "second lookup belongs to req-2");
    let warn = with_message(&rec, "order not found");
    assert_eq!(request_id(warn[0]), Some("req-2"), "miss warning belongs to req-2");
}

#[test]
fn part2_handler_runs_without_subscriber() {
    let s = store();
    let resp = handle_request(&req("req-8", Method::Get, "/orders/o-1"), &s);
    assert_eq!(resp.status, 200, "handler must work with no subscriber installed");
}
