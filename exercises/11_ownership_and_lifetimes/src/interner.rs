//! Part 5a — string interners: owned (`Interner`) and borrowing (`StrInterner<'a>`).

use std::marker::PhantomData;

/// Compact handle to an interned string. Dense, starting at 0, in first-seen order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol {
    // engineer: u32 index
}

impl Symbol {
    /// Position in the interner's first-seen order.
    pub fn index(self) -> usize {
        todo!()
    }
}

/// Owns its strings. Uses interior mutability to count failed `get` calls.
#[derive(Debug, Default)]
pub struct Interner {
    // engineer: map, strings, Cell<u64> miss counter
}

impl Interner {
    pub fn new() -> Self {
        todo!()
    }

    /// Returns the existing symbol for `text` or inserts it.
    pub fn intern(&mut self, _text: &str) -> Symbol {
        todo!()
    }

    /// Looks up `text` without inserting. Takes `&self`; a `None` result increments [`Interner::misses`].
    pub fn get(&self, _text: &str) -> Option<Symbol> {
        todo!()
    }

    /// The string for `sym`, or `None` if its index is out of range for this interner.
    pub fn resolve(&self, _sym: Symbol) -> Option<&str> {
        todo!()
    }

    pub fn len(&self) -> usize {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        todo!()
    }

    /// Number of `get` calls that returned `None`.
    pub fn misses(&self) -> u64 {
        todo!()
    }
}

/// Borrows its strings from the caller's input; never copies text.
#[derive(Debug, Default)]
pub struct StrInterner<'a> {
    // engineer: map, strings
    _input: PhantomData<&'a str>,
}

impl<'a> StrInterner<'a> {
    pub fn new() -> Self {
        todo!()
    }

    /// Returns the existing symbol for `text` or records the borrowed `text`.
    pub fn intern(&mut self, _text: &'a str) -> Symbol {
        todo!()
    }

    /// The borrowed string for `sym`. The lifetime is the input's, not `self`'s.
    pub fn resolve(&self, _sym: Symbol) -> Option<&'a str> {
        todo!()
    }

    pub fn len(&self) -> usize {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        todo!()
    }
}
