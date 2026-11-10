//! Part 5: middleware chains built from boxed closures.
//!
//! A middleware is `Fn(Request, next) -> Response`, where `next` is a
//! `&dyn Fn(Request) -> Response` for the rest of the chain. A middleware may
//! rewrite the request, short-circuit, or rewrite the response.

use std::collections::BTreeMap;

/// Case-insensitive header map. Names are stored lowercase.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[allow(dead_code)] // stub: remove once the map is used
struct Headers(BTreeMap<String, String>);

/// An incoming request.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // stub: remove once the fields are used
pub struct Request {
    /// Request path, for example `/ping`.
    pub path: String,
    headers: Headers,
}

impl Request {
    /// A request for `path` with no headers.
    pub fn new(_path: impl Into<String>) -> Self {
        todo!()
    }

    /// Sets a header, replacing any existing value with the same name (case-insensitive).
    pub fn with_header(self, _name: &str, _value: impl Into<String>) -> Self {
        todo!()
    }

    /// Looks up a header by case-insensitive name.
    pub fn header(&self, _name: &str) -> Option<&str> {
        todo!()
    }
}

/// An outgoing response.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // stub: remove once the fields are used
pub struct Response {
    /// HTTP-style status code.
    pub status: u16,
    /// Response body.
    pub body: String,
    headers: Headers,
}

impl Response {
    /// A response with no headers.
    pub fn new(_status: u16, _body: impl Into<String>) -> Self {
        todo!()
    }

    /// Sets a header, replacing any existing value with the same name (case-insensitive).
    pub fn with_header(self, _name: &str, _value: impl Into<String>) -> Self {
        todo!()
    }

    /// Looks up a header by case-insensitive name.
    pub fn header(&self, _name: &str) -> Option<&str> {
        todo!()
    }
}

/// A boxed middleware layer, as stored by [`Pipeline`].
type Layer = dyn Fn(Request, &dyn Fn(Request) -> Response) -> Response + Send + Sync;

/// A chain of middleware layers ending in an endpoint.
///
/// Layers run in the order they were added. The first layer added is the
/// outermost. The pipeline is `Send + Sync` so it can serve many threads.
#[allow(dead_code)] // stub: remove once the fields are used
pub struct Pipeline {
    layers: Vec<Box<Layer>>,
    endpoint: Box<dyn Fn(Request) -> Response + Send + Sync>,
}

impl Pipeline {
    /// A pipeline with no layers that runs `endpoint` directly.
    pub fn new(_endpoint: impl Fn(Request) -> Response + Send + Sync + 'static) -> Self {
        todo!()
    }

    /// Adds a layer as the new innermost layer (it runs just before the endpoint).
    pub fn layer(
        self,
        _middleware: impl Fn(Request, &dyn Fn(Request) -> Response) -> Response
        + Send
        + Sync
        + 'static,
    ) -> Self {
        todo!()
    }

    /// Runs the request through every layer and then the endpoint.
    pub fn handle(&self, _request: Request) -> Response {
        todo!()
    }
}

/// Middleware that answers `400` unless the request carries header `name`.
///
/// The header is matched case-insensitively. The 400 body names the missing header.
pub fn require_header(
    _name: &'static str,
) -> impl Fn(Request, &dyn Fn(Request) -> Response) -> Response + Send + Sync + 'static {
    |_request: Request, _next: &dyn Fn(Request) -> Response| -> Response { todo!() }
}
