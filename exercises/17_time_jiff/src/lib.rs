//! Dates and times with `jiff`: zones, parsing, calendars, meetings, bookings,
//! and injected clocks.
//!
//! Each module is one Part of `README.md`. Every function body is `todo!()`;
//! implement them so the tests in `tests/` pass.

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
