//! Part 2 — spans, context propagation and request ids.

use std::collections::HashMap;

use tracing::{debug, info, info_span, instrument, warn};

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
#[instrument(name = "store.load", skip(store, id), fields(order_id = %id))]
pub fn load_order<'a>(store: &'a OrderStore, id: &str) -> Option<&'a Order> {
    debug!("order lookup");
    store.orders.get(id)
}

/// Routes and answers one request.
pub fn handle_request(req: &Request, store: &OrderStore) -> Response {
    let span = info_span!(
        "http.request",
        request_id = %req.request_id,
        method = req.method.as_str(),
        path = %req.path,
        status = tracing::field::Empty,
    );
    let _guard = span.enter();
    debug!("routing");
    let response = route(req, store);
    span.record("status", response.status);
    info!(status = response.status, "request completed");
    response
}

fn route(req: &Request, store: &OrderStore) -> Response {
    let Some(id) = req.path.strip_prefix("/orders/") else {
        return Response { status: 404, body: "no route".to_owned() };
    };
    if req.method != Method::Get {
        return Response { status: 405, body: "method not allowed".to_owned() };
    }
    match load_order(store, id) {
        Some(order) => Response {
            status: 200,
            body: format!("order {} amount={}", order.id, order.amount_cents),
        },
        None => {
            warn!(order_id = %id, "order not found");
            Response { status: 404, body: "order not found".to_owned() }
        }
    }
}
