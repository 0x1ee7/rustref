//! Part 4 reference: layered configuration over a nested value tree.

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
/// Stored highest precedence first. Lookups walk each layer's own tree; the
/// [`effective`](Self::effective) view is the deep merge of all layers.
#[derive(Debug, Clone)]
pub struct Config {
    layers: Vec<(Layer, Value)>,
}

impl Config {
    /// Snapshots the layers. Later mutation of `layers` does not affect the config.
    pub fn new(layers: &Layers) -> Self {
        let mut stack = Vec::with_capacity(4);
        for (layer, value) in [
            (Layer::Cli, &layers.cli),
            (Layer::Env, &layers.env),
            (Layer::File, &layers.file),
        ] {
            if let Some(value) = value {
                stack.push((layer, value.clone()));
            }
        }
        stack.push((Layer::Defaults, layers.defaults.clone()));
        Self { layers: stack }
    }

    /// Dotted-path lookup (`"db.pool.size"`). The highest-precedence layer that
    /// contains the full path wins; a `Null` value still counts as present.
    pub fn get(&self, path: &str) -> Option<&Value> {
        self.winner(path).map(|(_, value)| value)
    }

    /// The layer that supplies [`get`](Self::get)'s value, or `None` if absent.
    pub fn source(&self, path: &str) -> Option<Layer> {
        self.winner(path).map(|(layer, _)| layer)
    }

    /// The fully merged effective configuration, as a fresh owned value.
    pub fn effective(&self) -> Value {
        self.layers
            .iter()
            .rev()
            .fold(Value::Map(BTreeMap::new()), |merged, (_, value)| {
                deep_merge(&merged, value)
            })
    }

    fn winner(&self, path: &str) -> Option<(Layer, &Value)> {
        self.layers
            .iter()
            .find_map(|(layer, root)| lookup(root, path).map(|value| (*layer, value)))
    }
}

/// Walks one tree. Empty segments and paths through a non-map are absent.
fn lookup<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut node = root;
    for segment in path.split('.') {
        if segment.is_empty() {
            return None;
        }
        match node {
            Value::Map(map) => node = map.get(segment)?,
            _ => return None,
        }
    }
    Some(node)
}

/// Recursively merges `overlay` onto `base`. Where both are maps, merge key by
/// key; otherwise the overlay value wins (lists are replaced, `Null` overrides).
/// Neither input is mutated and the result shares no state with either input.
pub fn deep_merge(base: &Value, overlay: &Value) -> Value {
    match (base, overlay) {
        (Value::Map(base_map), Value::Map(overlay_map)) => {
            let mut merged = base_map.clone();
            for (key, overlay_value) in overlay_map {
                let next = match merged.get(key) {
                    Some(base_value) => deep_merge(base_value, overlay_value),
                    None => overlay_value.clone(),
                };
                merged.insert(key.clone(), next);
            }
            Value::Map(merged)
        }
        _ => overlay.clone(),
    }
}

/// Builds a nested [`Value::Map`] from environment variables named
/// `<prefix>_<PATH>`, where `__` separates path segments (lowercased).
///
/// # Errors
/// - [`Error::EmptyEnvSegment`] if a path has an empty segment.
/// - [`Error::EnvPathConflict`] if two variables target the same leaf, or a leaf
///   and a map. Each clash is detected on whichever variable comes second, so the
///   outcome does not depend on input order.
pub fn parse_env<I, K, V>(vars: I, prefix: &str) -> Result<Value, Error>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: AsRef<str>,
{
    let want = format!("{prefix}_");
    let mut root: BTreeMap<String, Value> = BTreeMap::new();
    for (name, raw) in vars {
        let name = name.as_ref();
        let Some(path) = name.strip_prefix(want.as_str()) else {
            continue;
        };
        let segments: Vec<String> = path.split("__").map(str::to_lowercase).collect();
        if segments.iter().any(String::is_empty) {
            return Err(Error::EmptyEnvSegment {
                key: name.to_string(),
            });
        }
        let conflict = || Error::EnvPathConflict {
            key: name.to_string(),
            path: segments.join("."),
        };
        let (leaf, parents) = segments.split_last().expect("split yields at least one segment");
        let mut node = &mut root;
        for segment in parents {
            let child = node
                .entry(segment.clone())
                .or_insert_with(|| Value::Map(BTreeMap::new()));
            match child {
                Value::Map(map) => node = map,
                _ => return Err(conflict()),
            }
        }
        if node.contains_key(leaf) {
            return Err(conflict());
        }
        node.insert(leaf.clone(), coerce(raw.as_ref()));
    }
    Ok(Value::Map(root))
}

/// `true`/`false` (any case), then an optional `-` and ASCII digits fitting `i64`;
/// anything else stays a string.
fn coerce(raw: &str) -> Value {
    if raw.eq_ignore_ascii_case("true") {
        return Value::Bool(true);
    }
    if raw.eq_ignore_ascii_case("false") {
        return Value::Bool(false);
    }
    let digits = raw.strip_prefix('-').unwrap_or(raw);
    if !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && let Ok(n) = raw.parse::<i64>()
    {
        return Value::Int(n);
    }
    Value::Str(raw.to_string())
}
