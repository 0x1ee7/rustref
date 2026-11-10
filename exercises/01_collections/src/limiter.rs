//! Part 2: per-client sliding-window rate limiter with a custom map key.

use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::time::Duration;

use crate::{Clock, Error, SystemClock};

/// A client identifier. Equality is ASCII case-insensitive (`"Tenant-A"` and
/// `"tenant-a"` are the same tenant), so `Eq` and `Hash` are implemented by hand
/// and must agree.
///
/// Engineer-owned: store the name however you like.
#[derive(Debug, Clone)]
pub struct ClientId {}

impl ClientId {
    /// Wraps a client name as given.
    pub fn new(_name: impl Into<String>) -> Self {
        todo!()
    }
}

impl PartialEq for ClientId {
    fn eq(&self, _other: &Self) -> bool {
        todo!()
    }
}

impl Eq for ClientId {}

impl Hash for ClientId {
    fn hash<H: Hasher>(&self, _state: &mut H) {
        todo!()
    }
}

/// At most `limit` requests per client in any trailing `window`.
///
/// Engineer-owned: per-client event storage. The `PhantomData` keeps `C` in use
/// until you add real fields.
pub struct SlidingWindowLimiter<C = SystemClock> {
    _marker: PhantomData<C>,
}

impl<C: Clock> SlidingWindowLimiter<C> {
    /// Creates a limiter with no recorded clients.
    ///
    /// # Errors
    /// [`Error::ZeroLimit`] if `limit == 0`; [`Error::ZeroWindow`] if `window` is zero.
    pub fn new(_limit: u32, _window: Duration, _clock: C) -> Result<Self, Error> {
        todo!()
    }

    /// Records and admits the request if fewer than `limit` events are live in
    /// the window; otherwise returns `false` and records nothing.
    pub fn allow(&mut self, _client: &ClientId) -> bool {
        todo!()
    }

    /// Requests the client may still make right now: `limit - live events`, floored at 0.
    pub fn remaining(&self, _client: &ClientId) -> u32 {
        todo!()
    }

    /// Zero if [`remaining`](Self::remaining) is positive; otherwise the time until
    /// the oldest live event leaves the window.
    pub fn retry_after(&self, _client: &ClientId) -> Duration {
        todo!()
    }

    /// Drops state for clients with no live events; returns how many were dropped.
    pub fn prune(&mut self) -> usize {
        todo!()
    }
}
