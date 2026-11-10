//! `ex20_macros`: declarative macros for real-world boilerplate.
//!
//! Every macro is `#[macro_export]`ed, so callers write `ex20_macros::hashmap!`
//! and each expansion reaches items through `$crate::…`. The bodies here are
//! stubs: macros expand to `todo!()`, and item-generating macros emit
//! type-correct items whose methods are `todo!()`. Implement the macro bodies
//! (and the stub methods) to make the tests pass.

pub mod collections;
pub mod control;
pub mod enums;
pub mod newtype;
pub mod routing;
pub mod testing;

pub use enums::UnknownVariant;
pub use routing::{Lookup, Match, Method, Route, RouteError, Router};
