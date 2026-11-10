//! Part 6: immutable records, order-preserving grouping and lazily populated maps.

use std::borrow::Borrow;
use std::hash::Hash;
use std::marker::PhantomData;

/// A shipment line. Immutable through its API: there are no setters, and
/// [`with_qty`](Self::with_qty) returns a new record.
///
/// Engineer-owned: the fields (`sku`, `qty`, `dest`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Shipment {}

impl Shipment {
    /// Creates a shipment line.
    pub fn new(_sku: impl Into<String>, _qty: u32, _dest: impl Into<String>) -> Self {
        todo!()
    }

    /// The SKU.
    pub fn sku(&self) -> &str {
        todo!()
    }

    /// The quantity.
    pub fn qty(&self) -> u32 {
        todo!()
    }

    /// The destination.
    pub fn dest(&self) -> &str {
        todo!()
    }

    /// A copy of this record with a different quantity; `self` is unchanged.
    pub fn with_qty(&self, _qty: u32) -> Self {
        todo!()
    }
}

/// Groups items by `key`. Groups appear in first-seen key order; items within a
/// group keep their input order. Consumes the iterator once.
pub fn group_by<T, K, I, F>(_items: I, _key: F) -> Vec<(K, Vec<T>)>
where
    I: IntoIterator<Item = T>,
    K: Hash + Eq + Clone,
    F: FnMut(&T) -> K,
{
    todo!()
}

/// A map that creates missing values on demand with a factory, for lazily built
/// per-key state. Only [`get_or_create`](Self::get_or_create) inserts; every
/// other method is read-only.
///
/// Engineer-owned: the storage. The `PhantomData` keeps the parameters in use
/// until you add real fields.
#[derive(Clone)]
pub struct LazyMap<K, V, F> {
    _marker: PhantomData<(K, V, F)>,
}

impl<K: Hash + Eq, V, F: FnMut() -> V> LazyMap<K, V, F> {
    /// Creates an empty map that uses `factory` for missing entries.
    pub fn new(_factory: F) -> Self {
        todo!()
    }

    /// Returns the value for `key`, calling the factory and storing the result if absent.
    pub fn get_or_create(&mut self, _key: K) -> &mut V {
        todo!()
    }

    /// Read-only lookup. Never creates an entry.
    pub fn get<Q>(&self, _key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        todo!()
    }

    /// `true` iff an entry exists. Never creates one.
    pub fn contains_key<Q>(&self, _key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        todo!()
    }

    /// Number of stored entries.
    pub fn len(&self) -> usize {
        todo!()
    }

    /// `true` iff no entries are stored.
    pub fn is_empty(&self) -> bool {
        todo!()
    }
}
