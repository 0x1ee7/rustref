//! Part 6 reference: immutable records, order-preserving grouping, lazy maps.

use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;

/// A shipment line. Immutable through its API: there are no setters, and
/// [`with_qty`](Self::with_qty) returns a new record.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Shipment {
    sku: String,
    qty: u32,
    dest: String,
}

impl Shipment {
    /// Creates a shipment line.
    pub fn new(sku: impl Into<String>, qty: u32, dest: impl Into<String>) -> Self {
        Self {
            sku: sku.into(),
            qty,
            dest: dest.into(),
        }
    }

    /// The SKU.
    pub fn sku(&self) -> &str {
        &self.sku
    }

    /// The quantity.
    pub fn qty(&self) -> u32 {
        self.qty
    }

    /// The destination.
    pub fn dest(&self) -> &str {
        &self.dest
    }

    /// A copy of this record with a different quantity; `self` is unchanged.
    pub fn with_qty(&self, qty: u32) -> Self {
        Self {
            qty,
            ..self.clone()
        }
    }
}

/// Groups items by `key`. Groups appear in first-seen key order; items within a
/// group keep their input order. Consumes the iterator once.
pub fn group_by<T, K, I, F>(items: I, mut key: F) -> Vec<(K, Vec<T>)>
where
    I: IntoIterator<Item = T>,
    K: Hash + Eq + Clone,
    F: FnMut(&T) -> K,
{
    let mut index: HashMap<K, usize> = HashMap::new();
    let mut groups: Vec<(K, Vec<T>)> = Vec::new();
    for item in items {
        let k = key(&item);
        if let Some(&slot) = index.get(&k) {
            groups[slot].1.push(item);
        } else {
            index.insert(k.clone(), groups.len());
            groups.push((k, vec![item]));
        }
    }
    groups
}

/// A map that creates missing values on demand with a factory, for lazily built
/// per-key state. Only [`get_or_create`](Self::get_or_create) inserts.
#[derive(Clone)]
pub struct LazyMap<K, V, F> {
    entries: HashMap<K, V>,
    factory: F,
}

impl<K: Hash + Eq, V, F: FnMut() -> V> LazyMap<K, V, F> {
    /// Creates an empty map that uses `factory` for missing entries.
    pub fn new(factory: F) -> Self {
        Self {
            entries: HashMap::new(),
            factory,
        }
    }

    /// Returns the value for `key`, calling the factory and storing the result if absent.
    pub fn get_or_create(&mut self, key: K) -> &mut V {
        let factory = &mut self.factory;
        self.entries.entry(key).or_insert_with(factory)
    }

    /// Read-only lookup. Never creates an entry.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.entries.get(key)
    }

    /// `true` iff an entry exists. Never creates one.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.entries.contains_key(key)
    }

    /// Number of stored entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` iff no entries are stored.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
