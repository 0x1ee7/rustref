//! `ex20_macros`: declarative macros for real-world boilerplate.
//!
//! Every macro is `#[macro_export]`ed, so callers write `ex20_macros::hashmap!`
//! and each expansion reaches items through `$crate::…`.

pub mod collections;
pub mod control;
pub mod enums;
pub mod newtype;
pub mod routing;
pub mod testing;

pub use enums::UnknownVariant;
pub use routing::{Lookup, Match, Method, Route, RouteError, Router};
