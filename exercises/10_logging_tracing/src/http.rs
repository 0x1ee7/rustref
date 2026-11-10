//! Part 2 — spans, context propagation and request ids.

use std::collections::HashMap;

/// HTTP method subset understood by the handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Delete,
}

impl Method {
    /// Upper-case wire name, e.g. `"GET"`.
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Delete => "DELETE",
        }
    }
}

/// An inbound request. `request_id` is assigned by the edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub request_id: String,
    pub method: Method,
    pub path: String,
}

/// An outbound response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub body: String,
}

/// A stored order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    pub id: String,
    pub amount_cents: i64,
}

/// In-memory order storage.
#[derive(Debug, Default)]
pub struct OrderStore {
    orders: HashMap<String, Order>,
}

impl OrderStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts or replaces an order keyed by its id.
    pub fn insert(&mut self, order: Order) {
        self.orders.insert(order.id.clone(), order);
    }
}

/// Looks up `id` in `store`.
///
/// Part 2: runs inside a span named `store.load` whose `skip`s the store and
/// whose `order_id` field is `id`. Emits a DEBUG event `"order lookup"`.
pub fn load_order<'a>(store: &'a OrderStore, id: &str) -> Option<&'a Order> {
    let _ = (store, id);
    todo!()
}

/// Routes and answers one request.
///
/// Routes:
/// - path not starting with `/orders/` → 404, body `"no route"`;
/// - `/orders/<id>` with a method other than `GET` → 405, body `"method not allowed"`
///   (no lookup happens);
/// - `GET /orders/<id>` found → 200, body `"order <id> amount=<cents>"`;
/// - `GET /orders/<id>` missing → 404, body `"order not found"`, plus a WARN
///   event `"order not found"` with `order_id` (Display).
///
/// Observability requirements:
/// - the whole call runs inside a span named `http.request` with fields
///   `request_id` (Display), `method` (`Method::as_str`), `path` (Display) and
///   `status`, which is declared empty up front and recorded once the response
///   status is known;
/// - every event emitted inside (including those in `load_order`) must inherit
///   the `request_id` through the span tree;
/// - a DEBUG event `"routing"` when the request starts; an INFO event
///   `"request completed"` with `status` before returning.
pub fn handle_request(req: &Request, store: &OrderStore) -> Response {
    let _ = (req, store);
    todo!()
}
