//! Part 5 — typestate with `PhantomData`, validated newtypes, and why a
//! validated newtype should not implement `Deref`.

use std::borrow::Borrow;
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use thiserror::Error;

/// A validated `host:port` pair.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Endpoint {}

/// Why an endpoint string was rejected.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EndpointError {
    /// Not `host:port` with a non-empty host.
    #[error("endpoint '{0}' must look like host:port")]
    Malformed(String),
    /// Port is not an integer in 1..=65535.
    #[error("endpoint '{0}' has an invalid port (1-65535)")]
    InvalidPort(String),
}

impl Endpoint {
    /// Parses `host:port`. Splits on the last `:`.
    pub fn parse(s: &str) -> Result<Self, EndpointError> {
        let _ = s;
        todo!()
    }

    /// The host part.
    pub fn host(&self) -> &str {
        let _ = self;
        todo!()
    }

    /// The port.
    pub fn port(&self) -> u16 {
        let _ = self;
        todo!()
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = (self, f);
        todo!()
    }
}

/// State marker: no session open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Disconnected;

/// State marker: session open; statements may run.
#[derive(Debug)]
pub struct Connected;

/// A connection in state `S`. Only the methods valid for `S` exist.
///
/// Engineer: add the endpoint, the executed-statement counter, and use
/// `PhantomData<S>` as the state marker.
#[derive(Debug)]
pub struct Connection<S> {
    _state: PhantomData<S>,
}

/// Why `connect` failed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConnectError {
    /// The server refused the connection.
    #[error("connection to '{endpoint}' was refused")]
    Refused { endpoint: String },
}

/// Why `execute` failed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ExecError {
    /// The statement is empty or whitespace-only.
    #[error("statement is empty")]
    EmptyStatement,
}

impl<S> Connection<S> {
    /// The endpoint this connection targets.
    pub fn endpoint(&self) -> &Endpoint {
        let _ = self;
        todo!()
    }
}

impl Connection<Disconnected> {
    /// A new, unconnected handle.
    pub fn new(endpoint: Endpoint) -> Self {
        let _ = endpoint;
        todo!()
    }

    /// Opens a session. On failure the disconnected connection is returned with
    /// the error, so the caller can retry or reroute.
    ///
    /// Simulation: hosts ending in `.unreachable` are refused.
    pub fn connect(self) -> Result<Connection<Connected>, (Connection<Disconnected>, ConnectError)> {
        let _ = self;
        todo!()
    }
}

impl Connection<Connected> {
    /// Runs a statement. Blank statements are rejected and not counted.
    pub fn execute(&mut self, statement: &str) -> Result<(), ExecError> {
        let _ = (self, statement);
        todo!()
    }

    /// Number of statements executed in this session.
    pub fn executed(&self) -> u64 {
        let _ = self;
        todo!()
    }

    /// Closes the session.
    pub fn disconnect(self) -> Connection<Disconnected> {
        let _ = self;
        todo!()
    }
}

/// HTTP methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}

/// Why a URL was rejected.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UrlError {
    /// Not an absolute `http://` or `https://` URL with a host.
    #[error("'{0}' is not an absolute http(s) url")]
    Invalid(String),
    /// Absolute URL with a scheme other than http or https.
    #[error("unsupported scheme '{0}'")]
    UnsupportedScheme(String),
}

/// Typestate marker: no URL set yet.
#[derive(Debug)]
pub struct NoUrl;

/// Typestate marker: URL set; `build` is available.
#[derive(Debug)]
pub struct HasUrl;

/// A built request. Immutable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {}

impl Request {
    /// The method.
    pub fn method(&self) -> Method {
        let _ = self;
        todo!()
    }

    /// The URL exactly as set.
    pub fn url(&self) -> &str {
        let _ = self;
        todo!()
    }

    /// First header whose name matches `name` case-insensitively.
    pub fn header(&self, name: &str) -> Option<&str> {
        let _ = (self, name);
        todo!()
    }
}

/// Builder whose `build` exists only after a URL is set.
///
/// Engineer: store the method, the URL, headers in insertion order, and use
/// `PhantomData<S>` as the state marker.
#[derive(Debug)]
pub struct RequestBuilder<S> {
    _state: PhantomData<S>,
}

impl RequestBuilder<NoUrl> {
    /// Starts a request with `method`.
    pub fn new(method: Method) -> Self {
        let _ = method;
        todo!()
    }

    /// Sets the URL. Must be absolute `http://` or `https://` with a non-empty rest.
    pub fn url(self, url: &str) -> Result<RequestBuilder<HasUrl>, UrlError> {
        let _ = (self, url);
        todo!()
    }
}

impl RequestBuilder<HasUrl> {
    /// Produces the request. Infallible: the URL was validated in `url`.
    pub fn build(self) -> Request {
        let _ = self;
        todo!()
    }
}

impl<S> RequestBuilder<S> {
    /// Appends a header. Valid in every state.
    #[must_use]
    pub fn header(self, name: &str, value: &str) -> Self {
        let _ = (self, name, value);
        todo!()
    }
}

/// An email address, validated once at construction.
///
/// Rules: no whitespace; exactly one `@`; non-empty local part; domain contains
/// a `.` and neither starts nor ends with one.
///
/// Engineer: wrap a `String`. Implement `FromStr`, `AsRef<str>`, `Borrow<str>`,
/// and `Display`. Do not implement `Deref<Target = str>`: it would expose the
/// inner string's mutators-by-proxy, and every `&str` method would bypass the
/// validation boundary. Expose `as_str()` instead.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Email {}

/// Rejected email input.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("'{0}' is not a valid email address")]
pub struct EmailError(pub String);

impl Email {
    /// The validated address.
    pub fn as_str(&self) -> &str {
        let _ = self;
        todo!()
    }
}

impl FromStr for Email {
    type Err = EmailError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let _ = s;
        todo!()
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        let _ = self;
        todo!()
    }
}

impl Borrow<str> for Email {
    fn borrow(&self) -> &str {
        let _ = self;
        todo!()
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = (self, f);
        todo!()
    }
}
