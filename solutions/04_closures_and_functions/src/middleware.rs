//! Part 5: middleware chains built from boxed closures.
//!
//! A middleware is `Fn(Request, next) -> Response`, where `next` is a
//! `&dyn Fn(Request) -> Response` for the rest of the chain. A middleware may
//! rewrite the request, short-circuit, or rewrite the response.

use std::collections::BTreeMap;

/// A boxed middleware layer, as stored by [`Pipeline`].
type Layer = dyn Fn(Request, &dyn Fn(Request) -> Response) -> Response + Send + Sync;

/// Case-insensitive header map. Names are stored lowercase.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Headers(BTreeMap<String, String>);

impl Headers {
    fn insert(&mut self, name: &str, value: String) {
        self.0.insert(name.to_ascii_lowercase(), value);
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.0.get(&name.to_ascii_lowercase()).map(String::as_str)
    }
}

/// An incoming request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Request path, for example `/ping`.
    pub path: String,
    headers: Headers,
}

impl Request {
    /// A request for `path` with no headers.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            headers: Headers::default(),
        }
    }

    /// Sets a header, replacing any existing value with the same name (case-insensitive).
    pub fn with_header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.insert(name, value.into());
        self
    }

    /// Looks up a header by case-insensitive name.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name)
    }
}

/// An outgoing response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// HTTP-style status code.
    pub status: u16,
    /// Response body.
    pub body: String,
    headers: Headers,
}

impl Response {
    /// A response with no headers.
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
            headers: Headers::default(),
        }
    }

    /// Sets a header, replacing any existing value with the same name (case-insensitive).
    pub fn with_header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.insert(name, value.into());
        self
    }

    /// Looks up a header by case-insensitive name.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name)
    }
}

/// A chain of middleware layers ending in an endpoint.
///
/// Layers run in the order they were added. The first layer added is the
/// outermost. The pipeline is `Send + Sync` so it can serve many threads.
pub struct Pipeline {
    layers: Vec<Box<Layer>>,
    endpoint: Box<dyn Fn(Request) -> Response + Send + Sync>,
}

impl Pipeline {
    /// A pipeline with no layers that runs `endpoint` directly.
    pub fn new(endpoint: impl Fn(Request) -> Response + Send + Sync + 'static) -> Self {
        Self {
            layers: Vec::new(),
            endpoint: Box::new(endpoint),
        }
    }

    /// Adds a layer as the new innermost layer (it runs just before the endpoint).
    pub fn layer(
        mut self,
        middleware: impl Fn(Request, &dyn Fn(Request) -> Response) -> Response
        + Send
        + Sync
        + 'static,
    ) -> Self {
        self.layers.push(Box::new(middleware));
        self
    }

    /// Runs the request through every layer and then the endpoint.
    pub fn handle(&self, request: Request) -> Response {
        run_layers(&self.layers, self.endpoint.as_ref(), request)
    }
}

/// Runs `request` through `layers` in order, then `endpoint`. Recursion builds
/// each layer's `next` as a closure over the remaining layers.
fn run_layers(layers: &[Box<Layer>], endpoint: &dyn Fn(Request) -> Response, request: Request) -> Response {
    match layers.split_first() {
        None => endpoint(request),
        Some((outer, inner)) => outer(request, &|req: Request| run_layers(inner, endpoint, req)),
    }
}

/// Middleware that answers `400` unless the request carries header `name`.
///
/// The header is matched case-insensitively. The 400 body names the missing header.
pub fn require_header(
    name: &'static str,
) -> impl Fn(Request, &dyn Fn(Request) -> Response) -> Response + Send + Sync + 'static {
    move |request: Request, next: &dyn Fn(Request) -> Response| -> Response {
        if request.header(name).is_some() {
            next(request)
        } else {
            Response::new(400, format!("missing required header: {name}"))
        }
    }
}
