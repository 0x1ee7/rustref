//! Reference solution for exercise 17: dates and times with `jiff`.
//!
//! Each module is one Part of the exercise README.

pub mod arithmetic;
pub mod calendar;
pub mod clock;
pub mod error;
pub mod meetings;
pub mod parsing;
pub mod relative;
pub mod slots;
pub mod stopwatch;
pub mod zones;

pub use error::TimeError;
