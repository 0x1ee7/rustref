//! A small vector: up to `N` elements inline, then spills to the heap.

use std::marker::PhantomData;

/// Stores up to `N` elements without heap allocation; the push after that moves
/// everything into a `Vec`.
#[derive(Debug)]
pub struct InlineVec<T, const N: usize> {
    // Engineer-owned representation. The marker keeps `T` and `N` in use until then.
    _storage: PhantomData<[T; N]>,
}

impl<T, const N: usize> InlineVec<T, N> {
    /// An empty vector. Allocates nothing.
    pub fn new() -> Self {
        todo!()
    }

    /// Appends `value`. Allocates only on the push that exceeds `N` elements.
    pub fn push(&mut self, _value: T) {
        todo!()
    }

    /// Number of elements.
    pub fn len(&self) -> usize {
        todo!()
    }

    /// `true` when there are no elements.
    pub fn is_empty(&self) -> bool {
        todo!()
    }

    /// `true` iff the elements no longer fit inline, i.e. `len() > N`.
    pub fn is_spilled(&self) -> bool {
        todo!()
    }

    /// The element at `index`, or `None` when out of bounds.
    pub fn get(&self, _index: usize) -> Option<&T> {
        todo!()
    }

    /// Iterates over `&T` in insertion order.
    pub fn iter(&self) -> Iter<'_, T> {
        todo!()
    }
}

impl<T, const N: usize> Default for InlineVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Borrowing iterator over an [`InlineVec`].
pub struct Iter<'a, T> {
    _items: PhantomData<&'a T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        todo!()
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a InlineVec<T, N> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        let _ = self;
        todo!()
    }
}
