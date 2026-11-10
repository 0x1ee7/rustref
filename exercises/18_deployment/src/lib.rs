//! `rustref-service`: a small HTTP service shaped for container deployment.
//!
//! Each module is one part of the exercise (see `README.md`). Keep `main.rs`
//! thin: all behaviour lives in the library so it can be tested without sockets
//! or a real process.

pub mod api;
pub mod build_info;
pub mod cli;
pub mod config;
pub mod server;
