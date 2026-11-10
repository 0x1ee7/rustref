//! Part 2 — a `&mut self` builder that records invalid input and reports it
//! from `build`.

use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use crate::header::{Header, HeaderError};

/// HTTP methods accepted by `reqkit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl Method {
    /// Canonical upper-case token, e.g. `"GET"`.
    pub fn as_str(self) -> &'static str {
        todo!()
    }
}

impl fmt::Display for Method {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

/// Parses case-insensitively; anything else is `RequestError::InvalidMethod`.
impl FromStr for Method {
    type Err = RequestError;

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

/// Everything that can make a request invalid.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RequestError {
    /// The method token is not one of the supported methods.
    #[error("unsupported HTTP method {0:?}")]
    InvalidMethod(String),
    /// The URL does not start with `http://` or `https://` (case-insensitive).
    #[error("URL must start with http:// or https://: {0:?}")]
    InvalidUrl(String),
    /// A header name or value was rejected.
    #[error(transparent)]
    InvalidHeader(#[from] HeaderError),
    /// Two header names collide after lowercasing.
    #[error("duplicate header {0:?}")]
    DuplicateHeader(String),
    /// The timeout was zero.
    #[error("timeout must be greater than zero")]
    InvalidTimeout,
}

/// Builds a [`Request`].
///
/// Setters take `&mut self` and return `&mut Self`, so a builder can be
/// configured in place (for example from a helper that receives `&mut`) or
/// chained from a temporary. Invalid input is recorded and reported by
/// [`RequestBuilder::build`]; setters never fail.
#[derive(Debug)]
pub struct RequestBuilder {
    // private fields: parsed method, URL, headers, query pairs, timeout and the
    // first deferred error (engineer's choice)
}

impl RequestBuilder {
    /// Starts a request with a 10 second timeout. `method` is parsed and `url`
    /// validated by [`RequestBuilder::build`].
    pub fn new(_method: impl AsRef<str>, _url: impl Into<String>) -> Self {
        todo!()
    }

    /// Adds a header. Name rules and duplicate names are reported by `build`.
    pub fn header(&mut self, _name: impl AsRef<str>, _value: impl Into<String>) -> &mut Self {
        todo!()
    }

    /// Appends a query pair. Order is preserved; repeated keys are kept.
    pub fn query(&mut self, _key: impl AsRef<str>, _value: impl AsRef<str>) -> &mut Self {
        todo!()
    }

    /// Sets the timeout. Zero is reported by `build`.
    pub fn timeout(&mut self, _timeout: Duration) -> &mut Self {
        todo!()
    }

    /// Validates everything and produces an immutable [`Request`]. Borrows the
    /// builder, so it can be called again after further changes.
    ///
    /// Reports the method error first, then the first error recorded by the
    /// other setters.
    pub fn build(&self) -> Result<Request, RequestError> {
        todo!()
    }
}

/// A validated, immutable request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    // private fields (engineer's choice)
}

impl Request {
    /// The parsed method.
    pub fn method(&self) -> Method {
        todo!()
    }

    /// The URL exactly as given (without query pairs).
    pub fn url(&self) -> &str {
        todo!()
    }

    /// Headers in insertion order.
    pub fn headers(&self) -> &[Header] {
        todo!()
    }

    /// Case-insensitive lookup of a header value.
    pub fn header(&self, _name: &str) -> Option<&str> {
        todo!()
    }

    /// Query pairs in insertion order.
    pub fn query(&self) -> &[(String, String)] {
        todo!()
    }

    /// The request timeout.
    pub fn timeout(&self) -> Duration {
        todo!()
    }

    /// The URL with the query pairs appended, percent-encoded via
    /// [`crate::encode_query`]. Uses `&` if the URL already has a `?`.
    pub fn full_url(&self) -> String {
        todo!()
    }
}
