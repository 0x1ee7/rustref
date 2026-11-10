//! Part 2 — associated types and consts vs generic parameters.
//!
//! `Model` fixes one table name and one key type per implementing type, so
//! both are associated items. `Validator<T>` is a generic parameter because
//! one validator type may check several value types.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

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
    type Key: Copy + Ord + std::fmt::Display;
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
pub struct Repository<M: Model> {
    rows: BTreeMap<M::Key, M>,
}

impl<M: Model> Repository<M> {
    /// An empty table.
    pub fn new() -> Self {
        Self { rows: BTreeMap::new() }
    }

    /// Inserts a new row. Fails with [`RepoError::DuplicateKey`] if the key exists.
    pub fn insert(&mut self, model: M) -> Result<(), RepoError> {
        match self.rows.entry(model.key()) {
            Entry::Occupied(occupied) => Err(RepoError::DuplicateKey {
                table: M::TABLE,
                key: occupied.key().to_string(),
            }),
            Entry::Vacant(vacant) => {
                vacant.insert(model);
                Ok(())
            }
        }
    }

    /// Looks up a row by key.
    pub fn get(&self, key: M::Key) -> Option<&M> {
        self.rows.get(&key)
    }

    /// Removes and returns the row with `key`, if present.
    pub fn remove(&mut self, key: M::Key) -> Option<M> {
        self.rows.remove(&key)
    }

    /// Number of rows.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// `true` when the table has no rows.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Rows in ascending key order.
    pub fn iter(&self) -> impl Iterator<Item = &M> + '_ {
        self.rows.values()
    }
}

impl<M: Model> Default for Repository<M> {
    fn default() -> Self {
        Self::new()
    }
}

/// SQL type keyword for a column kind.
const fn sql_type(kind: ColumnKind) -> &'static str {
    match kind {
        ColumnKind::Text => "TEXT",
        ColumnKind::Integer => "INTEGER",
        ColumnKind::Boolean => "BOOLEAN",
    }
}

/// `CREATE TABLE <TABLE> (<col> <SQL type>, ...)` built from `M`'s associated
/// items. Types: `Text` → `TEXT`, `Integer` → `INTEGER`, `Boolean` → `BOOLEAN`.
pub fn create_table_sql<M: Model>() -> String {
    let columns: Vec<String> = M::columns()
        .iter()
        .map(|column| format!("{} {}", column.name, sql_type(column.kind)))
        .collect();
    format!("CREATE TABLE {} ({})", M::TABLE, columns.join(", "))
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

impl MaxLen {
    fn verdict(self, len: usize) -> Result<(), String> {
        if len <= self.0 {
            Ok(())
        } else {
            Err(format!("length {len} exceeds maximum {}", self.0))
        }
    }
}

impl Validator<str> for MaxLen {
    fn check(&self, value: &str) -> Result<(), String> {
        self.verdict(value.chars().count())
    }
}

impl<T> Validator<[T]> for MaxLen {
    fn check(&self, value: &[T]) -> Result<(), String> {
        self.verdict(value.len())
    }
}
