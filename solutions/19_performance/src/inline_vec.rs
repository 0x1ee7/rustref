//! A small vector: up to `N` elements inline, then spills to the heap.

use std::slice;

/// Stores up to `N` elements without heap allocation. The push that exceeds `N`
/// moves every element into a `Vec`, in order.
#[derive(Debug)]
pub enum InlineVec<T, const N: usize> {
    /// `items[..len]` are `Some`; the remaining slots are `None`.
    Inline { len: usize, items: [Option<T>; N] },
    Spilled(Vec<T>),
}

impl<T, const N: usize> InlineVec<T, N> {
    /// An empty vector. Allocates nothing.
    pub fn new() -> Self {
        InlineVec::Inline {
            len: 0,
            items: [const { None }; N],
        }
    }

    /// Appends `value`, spilling to the heap only when the inline slots are full.
    pub fn push(&mut self, value: T) {
        match self {
            Self::Spilled(items) => items.push(value),
            Self::Inline { len, items } if *len < N => {
                items[*len] = Some(value);
                *len += 1;
            }
            Self::Inline { .. } => self.spill(value),
        }
    }

    /// Moves the inline elements and `value` to a heap `Vec`.
    fn spill(&mut self, value: T) {
        let old = std::mem::replace(self, Self::Spilled(Vec::new()));
        let Self::Inline { items, .. } = old else {
            unreachable!("spill is only called on inline storage");
        };
        let mut spilled = Vec::with_capacity(N + 1);
        // Slots past `len` are `None`, so `flatten` yields exactly the stored elements.
        spilled.extend(items.into_iter().flatten());
        spilled.push(value);
        *self = Self::Spilled(spilled);
    }

    /// Number of elements.
    pub fn len(&self) -> usize {
        match self {
            Self::Inline { len, .. } => *len,
            Self::Spilled(items) => items.len(),
        }
    }

    /// `true` when there are no elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `true` iff the elements no longer fit inline.
    pub fn is_spilled(&self) -> bool {
        matches!(self, Self::Spilled(_))
    }

    /// The element at `index`, or `None` when out of bounds.
    pub fn get(&self, index: usize) -> Option<&T> {
        match self {
            Self::Inline { len, items } if index < *len => items[index].as_ref(),
            Self::Inline { .. } => None,
            Self::Spilled(items) => items.get(index),
        }
    }

    /// Iterates over `&T` in insertion order.
    pub fn iter(&self) -> Iter<'_, T> {
        match self {
            Self::Inline { items, .. } => Iter(IterInner::Inline(items.iter().flatten())),
            Self::Spilled(items) => Iter(IterInner::Spilled(items.iter())),
        }
    }
}

impl<T, const N: usize> Default for InlineVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Borrowing iterator over an [`InlineVec`].
pub struct Iter<'a, T>(IterInner<'a, T>);

enum IterInner<'a, T> {
    Inline(std::iter::Flatten<slice::Iter<'a, Option<T>>>),
    Spilled(slice::Iter<'a, T>),
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        match &mut self.0 {
            IterInner::Inline(iter) => iter.next(),
            IterInner::Spilled(iter) => iter.next(),
        }
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a InlineVec<T, N> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}
