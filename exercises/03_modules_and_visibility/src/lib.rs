//! Exercise 03: modules, visibility, facades, sealed traits, drop order,
//! statics and thread-local state.

pub mod catalog;
pub mod clock;
pub mod drop_order;
pub mod formula;
pub mod ids;
pub mod ledger;
pub mod registry;
pub mod report;
pub mod store;

/// Common types, re-exported for convenient glob imports.
pub mod prelude {
    pub use crate::catalog::Sku;
    pub use crate::formula::{evaluate, Env};
    pub use crate::ids::{id_factory, IdFactory};
    pub use crate::ledger::Ledger;
    pub use crate::registry::{Plugin, Registry};
    pub use crate::report::{format_by_name, Format, Table};
    pub use crate::store::Store;
}
