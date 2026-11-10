//! Borrow-first string handling: `Cow`, borrowed map keys, `Box<str>`, `Arc<str>`.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Lowercases ASCII letters only. Borrows when nothing changes.
pub fn normalize_header(name: &str) -> Cow<'_, str> {
    if name.bytes().any(|b| b.is_ascii_uppercase()) {
        Cow::Owned(name.to_ascii_lowercase())
    } else {
        Cow::Borrowed(name)
    }
}

/// Replaces spaces with `_`. Borrows when there are no spaces.
pub fn sanitize_label(label: &str) -> Cow<'_, str> {
    if label.contains(' ') {
        Cow::Owned(label.replace(' ', "_"))
    } else {
        Cow::Borrowed(label)
    }
}

/// Case-insensitive header map keyed by normalised names.
#[derive(Debug, Default)]
pub struct HeaderMap {
    entries: HashMap<String, String>,
}

impl HeaderMap {
    /// An empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts or replaces a value. Replacing an existing key reuses its allocation.
    pub fn insert(&mut self, name: &str, value: &str) -> Option<String> {
        let key = normalize_header(name);
        match self.entries.get_mut(key.as_ref()) {
            Some(slot) => Some(std::mem::replace(slot, value.to_owned())),
            None => {
                self.entries.insert(key.into_owned(), value.to_owned());
                None
            }
        }
    }

    /// Case-insensitive lookup. Lowercase names allocate nothing.
    pub fn get(&self, name: &str) -> Option<&str> {
        let key = normalize_header(name);
        self.entries.get(key.as_ref()).map(String::as_str)
    }

    /// Number of distinct (case-folded) names.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` when the map has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Shares one `Arc<str>` per distinct string.
#[derive(Debug, Default)]
pub struct Interner {
    table: HashSet<Arc<str>>,
}

impl Interner {
    /// An empty interner.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a shared handle. Re-interning allocates nothing.
    pub fn intern(&mut self, s: &str) -> Arc<str> {
        if let Some(existing) = self.table.get(s) {
            return Arc::clone(existing);
        }
        let shared: Arc<str> = Arc::from(s);
        self.table.insert(Arc::clone(&shared));
        shared
    }

    /// Number of distinct strings held.
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// `true` when nothing has been interned.
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }
}

/// An immutable label backed by `Box<str>` (two words, one allocation).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tag(Box<str>);

impl Tag {
    /// Copies `s` into a new label.
    pub fn new(s: &str) -> Self {
        Tag(Box::from(s))
    }

    /// The label text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
