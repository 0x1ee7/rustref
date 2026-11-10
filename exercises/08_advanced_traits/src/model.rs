//! Part 2 — associated types and consts vs generic parameters.
//!
//! `Model` fixes one table name and one key type per implementing type, so
//! both are associated items. `Validator<T>` is a generic parameter because
//! one validator type may check several value types.

use std::fmt::Display;
use std::marker::PhantomData;

use thiserror::Error;

/// SQL column kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnKind {
    Text,
    Integer,
    Boolean,
}

/// One column of a model's table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Column {
    pub name: &'static str,
    pub kind: ColumnKind,
}

/// A row type stored in one table.
pub trait Model: Clone {
    /// Table name. One constant per model type.
    const TABLE: &'static str;
    /// Primary-key type. Each model picks exactly one.
    type Key: Copy + Ord + Display;
    /// The primary key of this row.
    fn key(&self) -> Self::Key;
    /// Columns in declaration order.
    fn columns() -> &'static [Column];
}

/// Errors from [`Repository`] operations.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RepoError {
    /// A row with this key already exists; the stored row is unchanged.
    #[error("duplicate key '{key}' in table '{table}'")]
    DuplicateKey { table: &'static str, key: String },
}

/// In-memory table keyed by `M::Key`, iterated in ascending key order.
///
/// Engineer: add storage keyed by `M::Key` (an ordered map fits).
pub struct Repository<M: Model> {
    _model: PhantomData<M>,
}

impl<M: Model> Repository<M> {
    /// An empty table.
    pub fn new() -> Self {
        todo!()
    }

    /// Inserts a new row. Fails with [`RepoError::DuplicateKey`] if the key exists.
    pub fn insert(&mut self, model: M) -> Result<(), RepoError> {
        let _ = (self, model);
        todo!()
    }

    /// Looks up a row by key.
    pub fn get(&self, key: M::Key) -> Option<&M> {
        let _ = (self, key);
        todo!()
    }

    /// Removes and returns the row with `key`, if present.
    pub fn remove(&mut self, key: M::Key) -> Option<M> {
        let _ = (self, key);
        todo!()
    }

    /// Number of rows.
    pub fn len(&self) -> usize {
        let _ = self;
        todo!()
    }

    /// `true` when the table has no rows.
    pub fn is_empty(&self) -> bool {
        let _ = self;
        todo!()
    }

    /// Rows in ascending key order.
    #[allow(unreachable_code)]
    pub fn iter(&self) -> impl Iterator<Item = &M> + '_ {
        let _ = self;
        todo!();
        std::iter::empty()
    }
}

impl<M: Model> Default for Repository<M> {
    fn default() -> Self {
        Self::new()
    }
}

/// `CREATE TABLE <TABLE> (<col> <SQL type>, ...)` built from `M`'s associated
/// items. Types: `Text` → `TEXT`, `Integer` → `INTEGER`, `Boolean` → `BOOLEAN`.
pub fn create_table_sql<M: Model>() -> String {
    todo!()
}

/// Checks a value of type `T`. Generic over `T` so one validator can serve
/// several value types; an associated type would allow only one impl.
pub trait Validator<T: ?Sized> {
    /// `Err` carries a human-readable reason.
    fn check(&self, value: &T) -> Result<(), String>;
}

/// Maximum length. Strings count chars (not bytes); slices count elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaxLen(pub usize);

impl Validator<str> for MaxLen {
    fn check(&self, value: &str) -> Result<(), String> {
        let _ = (self, value);
        todo!()
    }
}

impl<T> Validator<[T]> for MaxLen {
    fn check(&self, value: &[T]) -> Result<(), String> {
        let _ = (self, value);
        todo!()
    }
}
