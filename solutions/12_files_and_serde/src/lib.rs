//! Files, paths and serde: a backup/sync toolkit.
//!
//! Manifest scanning, diffing with path-safe apply, atomic writes, serde JSON and
//! CSV reports, and a versioned binary snapshot header.

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
