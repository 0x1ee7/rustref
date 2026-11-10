//! Reference solution: async Rust with Tokio, built around fan-out, cancellation, bounded
//! concurrency, broadcast, worker pools, pagination with pooled resources, and resilience.

pub mod bounded;
pub mod broker;
pub mod cancel;
pub mod error;
pub mod fanout;
pub mod pool;
pub mod resilience;
pub mod resources;
