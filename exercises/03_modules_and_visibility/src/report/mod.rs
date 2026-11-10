//! Tabular reports. The formats are sealed: only this crate can implement [`Format`].

mod format;
mod table;

pub use format::{format_by_name, Csv, Format, Json, Text};
pub use table::{Table, TableError};
