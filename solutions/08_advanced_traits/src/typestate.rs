//! Part 5 — typestate with `PhantomData`, validated newtypes, and why a
//! validated newtype should not implement `Deref`.

use std::borrow::Borrow;
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use thiserror::Error;

/// A validated `host:port` pair.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Endpoint {
    host: String,
    port: u16,
}

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
        let (host, port) = s
            .rsplit_once(':')
            .ok_or_else(|| EndpointError::Malformed(s.to_owned()))?;
        if host.is_empty() {
            return Err(EndpointError::Malformed(s.to_owned()));
        }
        // Port 0 is not a connectable port; `u16` parsing rejects anything above 65535.
        let port = port
            .parse::<u16>()
            .ok()
            .filter(|&port| port != 0)
            .ok_or_else(|| EndpointError::InvalidPort(s.to_owned()))?;
        Ok(Self { host: host.to_owned(), port })
    }

    /// The host part.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// The port.
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

/// State marker: no session open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Disconnected;

/// State marker: session open; statements may run.
#[derive(Debug)]
pub struct Connected;

/// A connection in state `S`. Only the methods valid for `S` exist.
#[derive(Debug)]
pub struct Connection<S> {
    endpoint: Endpoint,
    executed: u64,
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
        &self.endpoint
    }
}

impl Connection<Disconnected> {
    /// A new, unconnected handle.
    pub fn new(endpoint: Endpoint) -> Self {
        Self { endpoint, executed: 0, _state: PhantomData }
    }

    /// Opens a session. On failure the disconnected connection is returned with
    /// the error, so the caller can retry or reroute.
    ///
    /// Simulation: hosts ending in `.unreachable` are refused.
    pub fn connect(self) -> Result<Connection<Connected>, (Connection<Disconnected>, ConnectError)> {
        if self.endpoint.host().ends_with(".unreachable") {
            let error = ConnectError::Refused { endpoint: self.endpoint.to_string() };
            return Err((self, error));
        }
        Ok(Connection { endpoint: self.endpoint, executed: 0, _state: PhantomData })
    }
}

impl Connection<Connected> {
    /// Runs a statement. Blank statements are rejected and not counted.
    pub fn execute(&mut self, statement: &str) -> Result<(), ExecError> {
        if statement.trim().is_empty() {
            return Err(ExecError::EmptyStatement);
        }
        self.executed += 1;
        Ok(())
    }

    /// Number of statements executed in this session.
    pub fn executed(&self) -> u64 {
        self.executed
    }

    /// Closes the session.
    pub fn disconnect(self) -> Connection<Disconnected> {
        Connection { endpoint: self.endpoint, executed: 0, _state: PhantomData }
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
pub struct Request {
    method: Method,
    url: String,
    headers: Vec<(String, String)>,
}

impl Request {
    /// The method.
    pub fn method(&self) -> Method {
        self.method
    }

    /// The URL exactly as set.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// First header whose name matches `name` case-insensitively.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// Builder whose `build` exists only after a URL is set.
#[derive(Debug)]
pub struct RequestBuilder<S> {
    method: Method,
    url: String,
    headers: Vec<(String, String)>,
    _state: PhantomData<S>,
}

impl RequestBuilder<NoUrl> {
    /// Starts a request with `method`.
    pub fn new(method: Method) -> Self {
        Self { method, url: String::new(), headers: Vec::new(), _state: PhantomData }
    }

    /// Sets the URL. Must be absolute `http://` or `https://` with a non-empty rest.
    pub fn url(self, url: &str) -> Result<RequestBuilder<HasUrl>, UrlError> {
        validate_url(url)?;
        Ok(RequestBuilder {
            method: self.method,
            url: url.to_owned(),
            headers: self.headers,
            _state: PhantomData,
        })
    }
}

/// Checks that `url` is absolute, with an http(s) scheme or a well-formed other scheme.
fn validate_url(url: &str) -> Result<(), UrlError> {
    let invalid = || UrlError::Invalid(url.to_owned());
    let (scheme, rest) = url.split_once("://").ok_or_else(invalid)?;
    let is_web = scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https");
    if !is_web {
        let well_formed = scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
        return Err(if well_formed {
            UrlError::UnsupportedScheme(scheme.to_owned())
        } else {
            invalid()
        });
    }
    if rest.is_empty() || rest.starts_with('/') {
        return Err(invalid());
    }
    Ok(())
}

impl RequestBuilder<HasUrl> {
    /// Produces the request. Infallible: the URL was validated in `url`.
    pub fn build(self) -> Request {
        Request { method: self.method, url: self.url, headers: self.headers }
    }
}

impl<S> RequestBuilder<S> {
    /// Appends a header. Valid in every state.
    #[must_use]
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }
}

/// An email address, validated once at construction.
///
/// Rules: no whitespace; exactly one `@`; non-empty local part; domain contains
/// a `.` and neither starts nor ends with one.
///
/// Deliberately no `Deref<Target = str>`: a deref would let every `&str` method
/// bypass the validation boundary. Read access goes through `as_str`, `AsRef`,
/// and `Borrow` instead.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Email {
    value: String,
}

/// Rejected email input.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("'{0}' is not a valid email address")]
pub struct EmailError(pub String);

impl Email {
    /// The validated address.
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl FromStr for Email {
    type Err = EmailError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || EmailError(s.to_owned());
        if s.chars().any(char::is_whitespace) {
            return Err(invalid());
        }
        let (local, domain) = s.split_once('@').ok_or_else(invalid)?;
        let domain_ok = domain.contains('.')
            && !domain.contains('@')
            && !domain.starts_with('.')
            && !domain.ends_with('.');
        if local.is_empty() || !domain_ok {
            return Err(invalid());
        }
        Ok(Self { value: s.to_owned() })
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        &self.value
    }
}

impl Borrow<str> for Email {
    fn borrow(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.value)
    }
}
