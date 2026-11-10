//! Order-management persistence on SQLite through `rusqlite` (bundled).
//!
//! Layering: [`types`] holds the SQL-mapped value types, [`db`] owns connections,
//! migrations and the unit-of-work boundary, [`repo`] has the repositories and the
//! keyset cursor, [`import`] and [`report`] are the batch and read-only paths.

mod db;
mod error;
mod import;
mod repo;
mod report;
mod types;

pub use db::{MIGRATIONS, Migration, connect, migrate, unit_of_work};
pub use error::{Error, Result};
pub use import::import_products;
pub use repo::{CustomerRepository, OrderCursor, OrderPage, OrderRepository, ProductRepository};
pub use report::{
    CustomerRevenue, ProductSales, RankedCustomer, RunningUnits, customer_running_units,
    rank_customers_by_orders, revenue_by_customer, units_sold_by_product,
};
pub use types::{
    Customer, Money, NewLine, Order, OrderLine, OrderStatus, OrderWithItems, Product, UtcTimestamp,
};
