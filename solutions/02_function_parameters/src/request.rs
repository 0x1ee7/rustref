//! Part 2 — a `&mut self` builder that records invalid input and reports it
//! from `build`.

use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use crate::encoding::encode_query;
use crate::header::{Header, HeaderError, normalize_header_name};

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
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Parses case-insensitively; anything else is `RequestError::InvalidMethod`.
impl FromStr for Method {
    type Err = RequestError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const ALL: [Method; 7] = [
            Method::Get,
            Method::Post,
            Method::Put,
            Method::Patch,
            Method::Delete,
            Method::Head,
            Method::Options,
        ];
        ALL.into_iter()
            .find(|m| m.as_str().eq_ignore_ascii_case(s))
            .ok_or_else(|| RequestError::InvalidMethod(s.to_owned()))
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
    /// Two header names collide after lowercasing. Carries the normalized name.
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
    method: Result<Method, RequestError>,
    url: String,
    headers: Vec<Header>,
    query: Vec<(String, String)>,
    timeout: Duration,
    /// The first header problem recorded by a setter.
    error: Option<RequestError>,
}

impl RequestBuilder {
    /// Starts a request with a 10 second timeout. `method` is parsed and `url`
    /// validated by [`RequestBuilder::build`].
    pub fn new(method: impl AsRef<str>, url: impl Into<String>) -> Self {
        Self {
            method: method.as_ref().parse(),
            url: url.into(),
            headers: Vec::new(),
            query: Vec::new(),
            timeout: Duration::from_secs(10),
            error: None,
        }
    }

    /// Adds a header. Name rules and duplicate names are reported by `build`.
    pub fn header(&mut self, name: impl AsRef<str>, value: impl Into<String>) -> &mut Self {
        match Header::new(name, value) {
            Ok(header) => {
                if self.headers.iter().any(|h| h.name() == header.name()) {
                    let (name, _) = header.into_parts();
                    self.record(RequestError::DuplicateHeader(name));
                } else {
                    self.headers.push(header);
                }
            }
            Err(err) => self.record(RequestError::InvalidHeader(err)),
        }
        self
    }

    /// Appends a query pair. Order is preserved; repeated keys are kept.
    pub fn query(&mut self, key: impl AsRef<str>, value: impl AsRef<str>) -> &mut Self {
        self.query
            .push((key.as_ref().to_owned(), value.as_ref().to_owned()));
        self
    }

    /// Sets the timeout. Zero is reported by `build`.
    pub fn timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = timeout;
        self
    }

    /// Validates everything and produces an immutable [`Request`]. Borrows the
    /// builder, so it can be called again after further changes.
    ///
    /// Reports, in order: the method, the URL, the first recorded header
    /// error, then a zero timeout.
    pub fn build(&self) -> Result<Request, RequestError> {
        let method = self.method.clone()?;
        if !is_http_url(&self.url) {
            return Err(RequestError::InvalidUrl(self.url.clone()));
        }
        if let Some(err) = &self.error {
            return Err(err.clone());
        }
        if self.timeout.is_zero() {
            return Err(RequestError::InvalidTimeout);
        }
        Ok(Request {
            method,
            url: self.url.clone(),
            headers: self.headers.clone(),
            query: self.query.clone(),
            timeout: self.timeout,
        })
    }

    /// Keeps only the first recorded problem.
    fn record(&mut self, error: RequestError) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }
}

/// Case-insensitive `http://` / `https://` prefix check without allocating.
fn is_http_url(url: &str) -> bool {
    url.get(..8).is_some_and(|p| p.eq_ignore_ascii_case("https://"))
        || url.get(..7).is_some_and(|p| p.eq_ignore_ascii_case("http://"))
}

/// A validated, immutable request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    method: Method,
    url: String,
    headers: Vec<Header>,
    query: Vec<(String, String)>,
    timeout: Duration,
}

impl Request {
    /// The parsed method.
    pub fn method(&self) -> Method {
        self.method
    }

    /// The URL exactly as given (without query pairs).
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Headers in insertion order.
    pub fn headers(&self) -> &[Header] {
        &self.headers
    }

    /// Case-insensitive lookup of a header value.
    pub fn header(&self, name: &str) -> Option<&str> {
        let normalized = normalize_header_name(name).ok()?;
        self.headers
            .iter()
            .find(|h| h.name() == normalized)
            .map(Header::value)
    }

    /// Query pairs in insertion order.
    pub fn query(&self) -> &[(String, String)] {
        &self.query
    }

    /// The request timeout.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// The URL with the query pairs appended, percent-encoded via
    /// [`crate::encode_query`]. Uses `&` if the URL already has a `?`.
    pub fn full_url(&self) -> String {
        if self.query.is_empty() {
            return self.url.clone();
        }
        let separator = if self.url.contains('?') { '&' } else { '?' };
        let pairs = self.query.iter().map(|(k, v)| (k.as_str(), v.as_str()));
        format!("{}{separator}{}", self.url, encode_query(pairs))
    }
}
