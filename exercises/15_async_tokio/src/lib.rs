//! Async Rust with tokio for a service that talks to a fleet of flaky upstreams.
//!
//! Every module is one part of the exercise (see `README.md`). Tasks are `Send + 'static`
//! and run on the runtime; futures are lazy and only make progress when polled.

pub mod bounded;
pub mod broker;
pub mod cancel;
pub mod error;
pub mod fanout;
pub mod pool;
pub mod resilience;
pub mod resources;
