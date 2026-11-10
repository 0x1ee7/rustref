//! `ops_toolkit`: collection-heavy components for an API gateway and its
//! billing/inventory side-jobs. Each module is one part of the exercise
//! (see `README.md`). The public surface declared here is the contract;
//! bodies and private internals are yours to write.

pub mod cache;
pub mod config;
pub mod inventory;
pub mod limiter;
pub mod records;
pub mod topk;

use std::time::Instant;

/// Monotonic time source. Injected so callers and tests control expiry and windows.
pub trait Clock {
    /// The current instant according to this clock.
    fn now(&self) -> Instant;
}

/// The real monotonic clock (`Instant::now`). The default clock for the stateful types.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        todo!()
    }
}

/// Every error this toolkit can report.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    /// `TtlCache::new` with `max_size == 0`.
    #[error("cache capacity must be at least 1")]
    ZeroCapacity,
    /// `TtlCache::new` with a zero TTL.
    #[error("cache ttl must be greater than zero")]
    ZeroTtl,
    /// `SlidingWindowLimiter::new` with `limit == 0`.
    #[error("rate limit must allow at least one request")]
    ZeroLimit,
    /// `SlidingWindowLimiter::new` with a zero window.
    #[error("rate limit window must be greater than zero")]
    ZeroWindow,
    /// A sales line with negative units.
    #[error("negative units for sku {sku}: {units}")]
    NegativeUnits { sku: String, units: i64 },
    /// A stock count (expected or counted) below zero.
    #[error("negative quantity for sku {sku}: {quantity}")]
    NegativeQuantity { sku: String, quantity: i64 },
    /// Integer arithmetic would overflow.
    #[error("arithmetic overflow")]
    Overflow,
    /// An environment variable whose `__`-split path has an empty segment.
    #[error("environment variable {key} has an empty path segment")]
    EmptyEnvSegment { key: String },
    /// Two environment variables map to incompatible places in the config tree.
    #[error("environment variable {key} conflicts at config path {path}")]
    EnvPathConflict { key: String, path: String },
    /// `PriceTiers::new` with no tiers.
    #[error("no price tiers given")]
    NoPriceTiers,
    /// `PriceTiers::new` whose lowest `min_qty` is not 1.
    #[error("the first price tier must start at quantity 1")]
    FirstTierMustStartAtOne,
    /// Two price tiers with the same `min_qty`.
    #[error("duplicate price tier at quantity {0}")]
    DuplicateTier(u32),
    /// A price tier with a zero or negative price.
    #[error("price must be positive")]
    NonPositivePrice,
    /// A pricing query for zero units.
    #[error("quantity must be at least 1")]
    ZeroQuantity,
    /// A response time that is negative or NaN.
    #[error("invalid response time: {0} minutes")]
    InvalidResponseTime(f64),
}
