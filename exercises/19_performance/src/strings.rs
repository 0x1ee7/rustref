//! Borrow-first string handling: `Cow`, borrowed map keys, `Box<str>`, `Arc<str>`.

use std::borrow::Cow;
use std::sync::Arc;

/// Lowercases ASCII letters only. Borrows when the name is already lowercase
/// (no allocation); owns otherwise. Non-ASCII characters are left untouched.
pub fn normalize_header(_name: &str) -> Cow<'_, str> {
    todo!()
}

/// Replaces every space with `_`. Borrows when there are no spaces.
pub fn sanitize_label(_label: &str) -> Cow<'_, str> {
    todo!()
}

/// Case-insensitive header map. Keys are normalised with [`normalize_header`].
#[derive(Debug, Default)]
pub struct HeaderMap {
    // Engineer-owned fields: a map from normalised name to value.
}

impl HeaderMap {
    /// An empty map.
    pub fn new() -> Self {
        todo!()
    }

    /// Inserts or replaces the value for `name`, returning the previous value.
    pub fn insert(&mut self, _name: &str, _value: &str) -> Option<String> {
        todo!()
    }

    /// Case-insensitive lookup. A lowercase name allocates nothing.
    pub fn get(&self, _name: &str) -> Option<&str> {
        todo!()
    }

    /// Number of distinct (case-folded) names.
    pub fn len(&self) -> usize {
        todo!()
    }

    /// `true` when the map has no entries.
    pub fn is_empty(&self) -> bool {
        todo!()
    }
}

/// Shares one `Arc<str>` per distinct string.
#[derive(Debug, Default)]
pub struct Interner {
    // Engineer-owned fields: the set of interned strings.
}

impl Interner {
    /// An empty interner.
    pub fn new() -> Self {
        todo!()
    }

    /// Returns a shared handle to `s`. Re-interning an existing string allocates nothing.
    pub fn intern(&mut self, _s: &str) -> Arc<str> {
        todo!()
    }

    /// Number of distinct strings held.
    pub fn len(&self) -> usize {
        todo!()
    }

    /// `true` when nothing has been interned.
    pub fn is_empty(&self) -> bool {
        todo!()
    }
}

/// An immutable label stored as `Box<str>`, one allocation for a non-empty label.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tag {
    // Engineer-owned field: a `Box<str>`.
}

impl Tag {
    /// Copies `s` into a new label.
    pub fn new(_s: &str) -> Self {
        todo!()
    }

    /// The label text.
    pub fn as_str(&self) -> &str {
        todo!()
    }
}
