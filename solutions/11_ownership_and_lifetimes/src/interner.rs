//! Part 5a — string interners: owned (`Interner`) and borrowing (`StrInterner<'a>`).

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

/// Compact handle to an interned string. Dense, starting at 0, in first-seen order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol {
    index: u32,
}

impl Symbol {
    /// Position in the interner's first-seen order.
    pub fn index(self) -> usize {
        self.index as usize
    }
}

/// Owns its strings. Uses interior mutability to count failed `get` calls.
#[derive(Debug, Default)]
pub struct Interner {
    // One allocation per string: the map and the vector share it via `Rc<str>`.
    map: HashMap<Rc<str>, Symbol>,
    strings: Vec<Rc<str>>,
    misses: Cell<u64>,
}

impl Interner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the existing symbol for `text` or inserts it.
    pub fn intern(&mut self, text: &str) -> Symbol {
        if let Some(&sym) = self.map.get(text) {
            return sym;
        }
        let sym = Symbol {
            index: u32::try_from(self.strings.len()).expect("more than u32::MAX symbols"),
        };
        let text: Rc<str> = Rc::from(text);
        self.strings.push(Rc::clone(&text));
        self.map.insert(text, sym);
        sym
    }

    /// Looks up `text` without inserting. Takes `&self`; a `None` result increments [`Interner::misses`].
    pub fn get(&self, text: &str) -> Option<Symbol> {
        match self.map.get(text) {
            Some(&sym) => Some(sym),
            None => {
                self.misses.set(self.misses.get() + 1);
                None
            }
        }
    }

    /// The string for `sym`, or `None` if its index is out of range for this interner.
    pub fn resolve(&self, sym: Symbol) -> Option<&str> {
        self.strings.get(sym.index()).map(AsRef::as_ref)
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Number of `get` calls that returned `None`.
    pub fn misses(&self) -> u64 {
        self.misses.get()
    }
}

/// Borrows its strings from the caller's input; never copies text.
#[derive(Debug, Default)]
pub struct StrInterner<'a> {
    map: HashMap<&'a str, Symbol>,
    strings: Vec<&'a str>,
}

impl<'a> StrInterner<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the existing symbol for `text` or records the borrowed `text`.
    pub fn intern(&mut self, text: &'a str) -> Symbol {
        if let Some(&sym) = self.map.get(text) {
            return sym;
        }
        let sym = Symbol {
            index: u32::try_from(self.strings.len()).expect("more than u32::MAX symbols"),
        };
        self.strings.push(text);
        self.map.insert(text, sym);
        sym
    }

    /// The borrowed string for `sym`. The lifetime is the input's, not `self`'s.
    pub fn resolve(&self, sym: Symbol) -> Option<&'a str> {
        self.strings.get(sym.index()).copied()
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}
