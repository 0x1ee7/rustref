//! Files, paths and serde: a backup/sync toolkit.
//!
//! Engineer-owned: every `todo!()` below is yours to implement. The public
//! surface is the contract; the tests in `tests/` are the spec.

pub mod atomic;
pub mod codec;
pub mod config;
pub mod diff;
pub mod error;
pub mod glob;
pub mod hash;
pub mod json;
pub mod report;
pub mod scan;
pub mod snapshot;

pub use error::{Error, Result, SnapshotError};
