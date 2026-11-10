//! Part 4: layered configuration (CLI > env > file > defaults) over a nested value tree.

use std::collections::BTreeMap;

use crate::Error;

/// A nested configuration value. Maps are ordered (`BTreeMap`), so two maps with
/// the same entries are equal and hash equally regardless of insertion order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Str(String),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
}

/// Which configuration layer a value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Cli,
    Env,
    File,
    Defaults,
}

/// The raw inputs for a [`Config`]. `None` layers count as empty.
#[derive(Debug, Clone, PartialEq)]
pub struct Layers {
    pub defaults: Value,
    pub file: Option<Value>,
    pub env: Option<Value>,
    pub cli: Option<Value>,
}

/// Layered configuration. Layers are snapshotted (cloned) at construction.
///
/// Engineer-owned: the snapshot representation.
#[derive(Debug, Clone)]
pub struct Config {}

impl Config {
    /// Snapshots the layers. Later mutation of `layers` does not affect the config.
    pub fn new(_layers: &Layers) -> Self {
        todo!()
    }

    /// Dotted-path lookup (`"db.pool.size"`). The highest-precedence layer that
    /// contains the full path wins; a `Null` value still counts as present.
    /// Empty segments and paths running through a non-map are absent.
    pub fn get(&self, _path: &str) -> Option<&Value> {
        todo!()
    }

    /// The layer that supplies [`get`](Self::get)'s value, or `None` if absent.
    pub fn source(&self, _path: &str) -> Option<Layer> {
        todo!()
    }

    /// The fully merged effective configuration, as a fresh owned value.
    pub fn effective(&self) -> Value {
        todo!()
    }
}

/// Recursively merges `overlay` onto `base`. Where both are maps, merge key by
/// key; otherwise the overlay value wins (lists are replaced, `Null` overrides).
/// Neither input is mutated and the result shares no state with either input.
pub fn deep_merge(_base: &Value, _overlay: &Value) -> Value {
    todo!()
}

/// Builds a nested [`Value::Map`] from environment variables named
/// `<prefix>_<PATH>`, where `__` separates path segments (lowercased), so
/// `APP_DB__POOL__SIZE=10` becomes `db.pool.size = 10`.
///
/// Values coerce to `Bool` (`true`/`false`, any case) or `Int` (optional `-`
/// then ASCII digits, fitting in `i64`); everything else stays `Str`.
/// Variables not starting with `<prefix>_` are ignored.
///
/// # Errors
/// - [`Error::EmptyEnvSegment`] if a path has an empty segment (`APP_`, `APP_A__`).
/// - [`Error::EnvPathConflict`] if two variables target the same leaf, or one
///   targets a leaf where another needs a map (order-independent).
pub fn parse_env<I, K, V>(_vars: I, _prefix: &str) -> Result<Value, Error>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: AsRef<str>,
{
    todo!()
}
