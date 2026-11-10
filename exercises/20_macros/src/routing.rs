//! Part 5: `routes!`, a declarative route table validated at runtime.
//!
//! Patterns are `/`-separated. A segment starting with `:` captures exactly one
//! non-empty path segment. A path is matched segment by segment, with no
//! prefix matching and no trailing-slash normalisation.

use std::fmt;
use std::marker::PhantomData;

/// HTTP method supported by the router.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl Method {
    /// Canonical upper-case name, as sent on the wire (`"GET"`, ...).
    pub const fn as_str(self) -> &'static str {
        panic!("Method::as_str")
    }
}

impl fmt::Display for Method {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("Method: Display")
    }
}

/// One route: method, pattern, and handler. The handler is any type `H`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route<H> {
    pub method: Method,
    pub pattern: &'static str,
    pub handler: H,
}

impl<H> Route<H> {
    /// Builds a route. Pattern validation happens in [`Router::new`].
    pub const fn new(_method: Method, _pattern: &'static str, _handler: H) -> Self {
        panic!("Route::new")
    }
}

/// Why a route table was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteError {
    /// Two routes share a method and a shape. Parameter names don't affect shape:
    /// `/users/:id` and `/users/:name` conflict.
    Duplicate {
        method: Method,
        pattern: &'static str,
        existing: &'static str,
    },
    /// The pattern is malformed. `reason` is a short human-readable explanation.
    InvalidPattern {
        pattern: &'static str,
        reason: &'static str,
    },
}

impl fmt::Display for RouteError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("RouteError: Display")
    }
}

impl std::error::Error for RouteError {}

/// A validated route table.
#[derive(Debug)]
pub struct Router<H> {
    // Private storage is yours (routes, plus whatever precomputed matching data you need).
    _handlers: PhantomData<H>,
}

impl<H> Router<H> {
    /// Validates every pattern and rejects duplicate method+shape pairs.
    ///
    /// The first offending route is reported. Table order is preserved.
    pub fn new(_routes: Vec<Route<H>>) -> Result<Self, RouteError> {
        todo!("Router::new")
    }

    /// Number of routes.
    pub fn len(&self) -> usize {
        todo!("Router::len")
    }

    /// Whether the table has no routes.
    pub fn is_empty(&self) -> bool {
        todo!("Router::is_empty")
    }

    /// Resolves `method` and `path`.
    ///
    /// Paths not starting with `/` are `NotFound`. If several routes with the
    /// method match, the most specific wins: at the first differing segment a
    /// literal beats a `:param`. If no route with this method matches, but a
    /// route with another method does, the result is `MethodNotAllowed` with
    /// the sorted, de-duplicated methods that do match.
    pub fn lookup<'r, 'p>(&'r self, _method: Method, _path: &'p str) -> Lookup<'r, 'p, H> {
        todo!("Router::lookup")
    }
}

/// Outcome of [`Router::lookup`].
#[derive(Debug)]
pub enum Lookup<'r, 'p, H> {
    /// A route matched.
    Found(Match<'r, 'p, H>),
    /// The path exists under other methods; these are the allowed ones.
    MethodNotAllowed(Vec<Method>),
    /// No route matches the path.
    NotFound,
}

/// A successful match: the handler plus captured parameters.
#[derive(Debug)]
pub struct Match<'r, 'p, H> {
    pub handler: &'r H,
    /// `(name, value)` for every `:name` segment, in pattern order.
    pub params: Vec<(&'static str, &'p str)>,
}

impl<'r, 'p, H> Match<'r, 'p, H> {
    /// Captured value of `:name`, if the pattern has that parameter.
    pub fn param(&self, _name: &str) -> Option<&'p str> {
        todo!("Match::param")
    }
}

/// Builds a [`Router`] from a table of `METHOD "pattern" => handler` lines.
///
/// `METHOD` is one of `GET`, `POST`, `PUT`, `PATCH`, `DELETE`. Any other
/// identifier is a compile error. The expansion evaluates to
/// `Result<Router<H>, RouteError>`. A trailing comma is optional, and an empty
/// table is allowed.
#[macro_export]
macro_rules! routes {
    ($($input:tt)*) => {
        todo!("routes!")
    };
}
