//! Part 2 — zero-copy INI parser.
//!
//! The document borrows section names and keys from the input, and borrows
//! values too unless they contain escapes (then they are decoded into owned
//! strings).

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::slice;
use std::str::FromStr;

use crate::text::{unescape, UnescapeError};

/// Parses `input`. Names, keys and values borrow from `input` unless a value
/// contains escapes.
pub fn parse(input: &str) -> Result<Document<'_>, ParseError> {
    let mut sections: Vec<Section<'_>> = Vec::new();
    let mut current: Option<usize> = None;

    for (idx, raw) in input.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        let err = |kind| ParseError { line: line_no, kind };

        if let Some(rest) = line.strip_prefix('[') {
            let name = match rest.strip_suffix(']') {
                Some(name) => name.trim(),
                None => return Err(err(ParseErrorKind::MalformedSection)),
            };
            if name.is_empty() {
                return Err(err(ParseErrorKind::MalformedSection));
            }
            if sections.iter().any(|s| s.name == name) {
                return Err(err(ParseErrorKind::DuplicateSection(name.to_string())));
            }
            sections.push(Section { name, entries: Vec::new() });
            current = Some(sections.len() - 1);
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            return Err(err(ParseErrorKind::MissingEquals));
        };
        let key = key.trim();
        if key.is_empty() {
            return Err(err(ParseErrorKind::EmptyKey));
        }
        let value = unescape(value.trim()).map_err(|e| err(ParseErrorKind::Escape(e)))?;

        // Top-level entries go to the global section, which can only exist
        // before the first header and is therefore always at index 0.
        let idx = match current {
            Some(idx) => idx,
            None => {
                sections.push(Section { name: "", entries: Vec::new() });
                let idx = sections.len() - 1;
                current = Some(idx);
                idx
            }
        };
        let section = &mut sections[idx];
        if section.entries.iter().any(|e| e.key == key) {
            return Err(err(ParseErrorKind::DuplicateKey {
                section: section.name.to_string(),
                key: key.to_string(),
            }));
        }
        section.entries.push(Entry { key, value });
    }

    Ok(Document { sections })
}

/// A parsed INI document. Top-level entries (before any header) live in the
/// section named `""`.
#[derive(Debug)]
pub struct Document<'a> {
    sections: Vec<Section<'a>>,
}

impl<'a> Document<'a> {
    /// Sections in file order.
    pub fn sections(&self) -> slice::Iter<'_, Section<'a>> {
        self.sections.iter()
    }

    /// Looks up a section by exact name. `""` is the global section.
    pub fn section(&self, name: &str) -> Option<&Section<'a>> {
        self.sections.iter().find(|s| s.name == name)
    }

    /// Looks up a value by section and key, borrowed from `self`.
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.section(section).and_then(|s| s.get(key))
    }

    /// Copies the whole document into owned maps: section → key → value.
    pub fn to_owned_map(&self) -> BTreeMap<String, BTreeMap<String, String>> {
        self.sections
            .iter()
            .map(|s| {
                let entries = s
                    .entries
                    .iter()
                    .map(|e| (e.key.to_string(), e.value.to_string()))
                    .collect();
                (s.name.to_string(), entries)
            })
            .collect()
    }
}

/// One `[name]` block. `name` borrows from the input.
#[derive(Debug)]
pub struct Section<'a> {
    name: &'a str,
    entries: Vec<Entry<'a>>,
}

impl<'a> Section<'a> {
    /// The section name, borrowed from the input (not from `self`).
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Entries in file order.
    pub fn entries(&self) -> slice::Iter<'_, Entry<'a>> {
        self.entries.iter()
    }

    /// Value for `key`, borrowed from `self`.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.key == key)
            .map(|e| e.value.as_ref())
    }

    /// Parses the value for `key` with `FromStr`.
    ///
    /// `None` if the key is absent; `Some(Err(_))` if it is present but does not parse.
    pub fn get_as<T: FromStr>(&self, key: &str) -> Option<Result<T, T::Err>> {
        self.get(key).map(str::parse)
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: ", self.line)?;
        match &self.kind {
            ParseErrorKind::MalformedSection => f.write_str("malformed section header"),
            ParseErrorKind::MissingEquals => f.write_str("expected `key = value`"),
            ParseErrorKind::EmptyKey => f.write_str("empty key"),
            ParseErrorKind::DuplicateSection(name) => write!(f, "duplicate section `{name}`"),
            ParseErrorKind::DuplicateKey { section, key } => {
                write!(f, "duplicate key `{key}` in section `{section}`")
            }
            ParseErrorKind::Escape(e) => write!(f, "invalid escape: {e}"),
        }
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            ParseErrorKind::Escape(e) => Some(e),
            _ => None,
        }
    }
}
