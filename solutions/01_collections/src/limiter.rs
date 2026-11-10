//! Part 2 reference: per-client sliding-window rate limiter.

use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use crate::{Clock, Error, SystemClock};

/// A client identifier. Equality is ASCII case-insensitive; `Hash` folds ASCII
/// case the same way, so the two always agree.
#[derive(Debug, Clone)]
pub struct ClientId {
    name: String,
}

impl ClientId {
    /// Wraps a client name as given.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl PartialEq for ClientId {
    fn eq(&self, other: &Self) -> bool {
        self.name.eq_ignore_ascii_case(&other.name)
    }
}

impl Eq for ClientId {}

impl Hash for ClientId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for byte in self.name.bytes() {
            state.write_u8(byte.to_ascii_lowercase());
        }
    }
}

/// At most `limit` requests per client in any trailing `window`.
///
/// Each client has a queue of admission instants, oldest first. Because the
/// clock is monotonic, expired events always form a prefix of that queue.
pub struct SlidingWindowLimiter<C = SystemClock> {
    limit: u32,
    window: Duration,
    clock: C,
    events: HashMap<ClientId, VecDeque<Instant>>,
}

/// Drops the expired prefix of `queue`. An event at `t` is live while `now < t + window`.
fn drop_expired(queue: &mut VecDeque<Instant>, now: Instant, window: Duration) {
    while queue
        .front()
        .is_some_and(|&t| now.duration_since(t) >= window)
    {
        queue.pop_front();
    }
}

impl<C: Clock> SlidingWindowLimiter<C> {
    /// Creates a limiter with no recorded clients.
    ///
    /// # Errors
    /// [`Error::ZeroLimit`] if `limit == 0`; [`Error::ZeroWindow`] if `window` is zero.
    pub fn new(limit: u32, window: Duration, clock: C) -> Result<Self, Error> {
        if limit == 0 {
            return Err(Error::ZeroLimit);
        }
        if window.is_zero() {
            return Err(Error::ZeroWindow);
        }
        Ok(Self {
            limit,
            window,
            clock,
            events: HashMap::new(),
        })
    }

    /// Admits and records the request if fewer than `limit` events are live.
    pub fn allow(&mut self, client: &ClientId) -> bool {
        let now = self.clock.now();
        let (limit, window) = (self.limit as usize, self.window);
        if !self.events.contains_key(client) {
            self.events.insert(client.clone(), VecDeque::new());
        }
        let queue = self.events.get_mut(client).expect("inserted above");
        drop_expired(queue, now, window);
        if queue.len() < limit {
            queue.push_back(now);
            true
        } else {
            false
        }
    }

    /// Requests the client may still make right now.
    pub fn remaining(&self, client: &ClientId) -> u32 {
        let now = self.clock.now();
        let live = self.events.get(client).map_or(0, |queue| {
            queue
                .iter()
                .filter(|&&t| now.duration_since(t) < self.window)
                .count()
        });
        self.limit
            .saturating_sub(u32::try_from(live).unwrap_or(u32::MAX))
    }

    /// Zero while capacity remains; otherwise the time until the oldest live event leaves.
    pub fn retry_after(&self, client: &ClientId) -> Duration {
        if self.remaining(client) > 0 {
            return Duration::ZERO;
        }
        let now = self.clock.now();
        self.events
            .get(client)
            .and_then(|queue| queue.front())
            .map_or(Duration::ZERO, |&oldest| {
                self.window.saturating_sub(now.duration_since(oldest))
            })
    }

    /// Drops clients with no live events; returns how many were dropped.
    pub fn prune(&mut self) -> usize {
        let now = self.clock.now();
        let window = self.window;
        let before = self.events.len();
        self.events.retain(|_, queue| {
            drop_expired(queue, now, window);
            !queue.is_empty()
        });
        before - self.events.len()
    }
}
