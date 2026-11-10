//! Part 4 — dependency injection by a small trait, a closure predicate, and
//! generic vs trait-object parameters.

use std::num::NonZeroU32;

use crate::request::Request;

/// A successful HTTP response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Transport-level failures (no HTTP status was received).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransportError {
    #[error("request timed out")]
    Timeout,
    #[error("connection reset by peer")]
    Reset,
    #[error("protocol error: {0}")]
    Protocol(String),
}

/// The only capability `send_with_retry` needs from the outside world.
pub trait Transport {
    /// Sends one request and waits for the response.
    fn send(&mut self, request: &Request) -> Result<Response, TransportError>;
}

/// Sends `request` through `transport`, making at most `max_attempts` calls.
///
/// After a failure, `retry_if` decides whether another attempt is made. It is
/// consulted only when attempts remain, so it is never called after the final
/// attempt. When the call gives up, the last error is returned unchanged.
/// `transport` may be a concrete type or `&mut dyn Transport`.
pub fn send_with_retry<T: Transport + ?Sized>(
    _transport: &mut T,
    _request: &Request,
    _max_attempts: NonZeroU32,
    _retry_if: impl FnMut(&TransportError) -> bool,
) -> Result<Response, TransportError> {
    todo!()
}
