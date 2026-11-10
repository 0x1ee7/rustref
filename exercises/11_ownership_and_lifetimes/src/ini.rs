//! Part 2 — zero-copy INI parser.
//!
//! The document borrows section names and keys from the input, and borrows
//! values too unless they contain escapes (then they are decoded into owned
//! strings).

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::marker::PhantomData;
use std::slice;
use std::str::FromStr;

use crate::text::UnescapeError;

/// Parses `input`. Names, keys and values borrow from `input` unless a value
/// contains escapes.
pub fn parse(_input: &str) -> Result<Document<'_>, ParseError> {
    todo!()
}

/// A parsed INI document. Top-level entries (before any header) live in the
/// section named `""`.
#[derive(Debug)]
pub struct Document<'a> {
    // engineer: sections
    _input: PhantomData<&'a str>,
}

impl<'a> Document<'a> {
    /// Sections in file order.
    pub fn sections(&self) -> slice::Iter<'_, Section<'a>> {
        todo!()
    }

    /// Looks up a section by exact name. `""` is the global section.
    pub fn section(&self, _name: &str) -> Option<&Section<'a>> {
        todo!()
    }

    /// Looks up a value by section and key, borrowed from `self`.
    pub fn get(&self, _section: &str, _key: &str) -> Option<&str> {
        todo!()
    }

    /// Copies the whole document into owned maps: section → key → value.
    pub fn to_owned_map(&self) -> BTreeMap<String, BTreeMap<String, String>> {
        todo!()
    }
}

/// One `[name]` block. `name` borrows from the input.
#[derive(Debug)]
pub struct Section<'a> {
    // engineer: name, entries
    _input: PhantomData<&'a str>,
}

impl<'a> Section<'a> {
    /// The section name, borrowed from the input (not from `self`).
    pub fn name(&self) -> &'a str {
        todo!()
    }

    /// Entries in file order.
    pub fn entries(&self) -> slice::Iter<'_, Entry<'a>> {
        todo!()
    }

    /// Value for `key`, borrowed from `self`.
    pub fn get(&self, _key: &str) -> Option<&str> {
        todo!()
    }

    /// Parses the value for `key` with `FromStr`.
    ///
    /// `None` if the key is absent; `Some(Err(_))` if it is present but does not parse.
    pub fn get_as<T: FromStr>(&self, _key: &str) -> Option<Result<T, T::Err>> {
        todo!()
    }
}

/// A `key = value` line.
#[derive(Debug, PartialEq, Eq)]
pub struct Entry<'a> {
    /// Borrowed from the input.
    pub key: &'a str,
    /// Borrowed from the input when it has no escapes, owned otherwise.
    pub value: Cow<'a, str>,
}

/// A parse failure with the 1-based line it occurred on.
#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub kind: ParseErrorKind,
}

/// What went wrong on a line.
#[derive(Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// A `[` line without a closing `]`, or with an empty name.
    MalformedSection,
    /// A non-blank, non-comment line with no `=`.
    MissingEquals,
    /// `= value` with nothing before the `=`.
    EmptyKey,
    /// A second `[name]` header with the same name.
    DuplicateSection(String),
    /// A key assigned twice in the same section.
    DuplicateKey { section: String, key: String },
    /// An invalid escape sequence in a value.
    Escape(UnescapeError),
}

impl fmt::Display for ParseError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        todo!()
    }
}
