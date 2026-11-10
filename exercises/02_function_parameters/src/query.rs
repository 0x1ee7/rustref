//! Part 5 — a consuming typestate builder: `build` exists only after a table
//! has been chosen, so a missing table is a compile error, not a runtime one.

use std::marker::PhantomData;

/// State before `from_table` has been called.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoTable;

/// State after `from_table`: carries the table name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HasTable {
    _table: String,
}

/// A SQL parameter.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Text(String),
}

impl From<bool> for Value {
    fn from(_v: bool) -> Self {
        todo!()
    }
}

impl From<i64> for Value {
    fn from(_v: i64) -> Self {
        todo!()
    }
}

impl From<&str> for Value {
    fn from(_v: &str) -> Self {
        todo!()
    }
}

impl From<String> for Value {
    fn from(_v: String) -> Self {
        todo!()
    }
}

/// `None` becomes [`Value::Null`]; `Some(v)` converts `v`.
impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(_v: Option<T>) -> Self {
        todo!()
    }
}

/// Why a query could not be rendered.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum QueryError {
    /// A column, table or filter name is not `ident` or `ident.ident`.
    #[error("invalid identifier {0:?}")]
    InvalidIdentifier(String),
    /// An `order_by` term is not `[-]ident[.ident]`.
    #[error("invalid order term {0:?}")]
    InvalidOrderTerm(String),
    /// `limit` does not fit the `i64` bound parameter.
    #[error("LIMIT {0} does not fit in i64")]
    LimitTooLarge(u64),
}

/// A `SELECT` under construction. The type parameter records whether a table
/// has been chosen.
#[derive(Debug, Clone, PartialEq)]
pub struct Select<State> {
    _state: PhantomData<State>,
}

/// Starts a query projecting `columns`. An empty list renders `*`. Column names
/// are validated by [`Select::build`].
pub fn select<I, S>(_columns: I) -> Select<NoTable>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    todo!()
}

impl Select<NoTable> {
    /// Chooses the table. Consumes the builder and returns the next state, where
    /// `build` becomes available.
    pub fn from_table(self, _table: impl AsRef<str>) -> Select<HasTable> {
        todo!()
    }
}

impl Select<HasTable> {
    /// Adds `column = ?` (or `column IS NULL` for a null value). Filters are
    /// joined with `AND` in call order.
    pub fn filter_eq(self, _column: impl AsRef<str>, _value: impl Into<Value>) -> Self {
        todo!()
    }

    /// Appends an order term. A leading `-` means `DESC`. Terms are rendered in
    /// call order.
    pub fn order_by(self, _term: impl AsRef<str>) -> Self {
        todo!()
    }

    /// Sets `LIMIT ?`. The last call wins.
    pub fn limit(self, _n: u64) -> Self {
        todo!()
    }

    /// Renders the SQL text and its parameters in placeholder order.
    pub fn build(self) -> Result<(String, Vec<Value>), QueryError> {
        todo!()
    }
}
