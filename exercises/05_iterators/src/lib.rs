//! Iterators and lazy pipelines for an ingestion service.
//!
//! Every module is one part of the exercise (see `README.md`). Everything here
//! must be lazy: no adapter may pull further from its source than the consumer asked for.

pub mod adapters;
pub mod batch;
pub mod containers;
pub mod log;
pub mod merge;
pub mod paging;
pub mod stats;
