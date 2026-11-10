//! Concurrency toolkit for "linkwatch", a link checker and download service.
//!
//! Std-only: threads, scoped threads, channels, `Mutex`/`RwLock`/`Condvar`,
//! atomics and `Barrier`. Each module is one part of the exercise.

pub mod bank;
pub mod cache;
pub mod downloader;
pub mod link;
pub mod parallel;
pub mod pool;
pub mod scheduler;
pub mod stats;
