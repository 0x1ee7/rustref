//! Observability for an order service: an HTTP-like request handler, a payment
//! call and a background settlement job.
//!
//! Library code only *emits* `tracing` events and spans. Whoever runs the
//! binary decides where they go (see [`stack`]).

pub mod filter;
pub mod http;
pub mod job;
pub mod json;
pub mod layers;
pub mod payments;
pub mod stack;
