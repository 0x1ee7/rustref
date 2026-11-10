//! Errors and cleanup: typed library errors, error chains, RAII guards,
//! panic isolation, batch aggregation and retry classification.
//!
//! One module per part of the README. Bodies are `todo!()`; add private fields,
//! helpers and modules as you see fit, but keep the public surface.

pub mod batch;
pub mod chain;
pub mod config;
pub mod isolation;
pub mod resources;
pub mod steps;
pub mod transaction;
