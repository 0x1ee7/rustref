//! Part 3 — blanket impls, extension traits, and sealed traits.

use std::fmt::Display;

use serde::Serialize;

/// JSON helpers for every serializable type, including unsized ones like `str`
/// and `[T]`. Implemented once via a blanket impl; callers never implement it.
pub trait JsonExt {
    /// Compact JSON.
    fn to_json(&self) -> Result<String, serde_json::Error>;
    /// Two-space indented JSON.
    fn to_json_pretty(&self) -> Result<String, serde_json::Error>;
}

impl<T: Serialize + ?Sized> JsonExt for T {
    fn to_json(&self) -> Result<String, serde_json::Error> {
        let _ = self;
        todo!()
    }

    fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        let _ = self;
        todo!()
    }
}

/// Extension methods on `str`.
pub trait StrExt {
    /// The longest prefix containing at most `max` chars. Never splits a char.
    fn truncate_chars(&self, max: usize) -> &str;

    /// Lowercase ASCII alphanumeric words joined by `-`. Every other character
    /// (including non-ASCII letters) is a separator; runs collapse and edges are trimmed.
    fn slugify(&self) -> String;
}

impl StrExt for str {
    fn truncate_chars(&self, max: usize) -> &str {
        let _ = (self, max);
        todo!()
    }

    fn slugify(&self) -> String {
        let _ = self;
        todo!()
    }
}

/// Extension methods on `Result<T, E>` for any displayable error.
pub trait ResultExt<T> {
    /// Maps an `Err(e)` to `Err("<tag>: <e>")`. `Ok` passes through.
    fn tag_err(self, tag: &str) -> Result<T, String>;
}

impl<T, E: Display> ResultExt<T> for Result<T, E> {
    fn tag_err(self, tag: &str) -> Result<T, String> {
        let _ = (self, tag);
        todo!()
    }
}

mod sealed {
    /// Prevents downstream crates from implementing [`super::SqlValue`].
    pub trait Sealed {}
}

/// Types that render as SQL literals. Sealed: only this crate decides which
/// types qualify and how they are escaped.
pub trait SqlValue: sealed::Sealed {
    /// SQL literal text: quoted and escaped for text, `NULL` for `None`.
    fn sql_literal(&self) -> String;
}

impl sealed::Sealed for i64 {}
impl sealed::Sealed for bool {}
impl sealed::Sealed for str {}
impl sealed::Sealed for String {}
impl<T: SqlValue> sealed::Sealed for Option<T> {}

impl SqlValue for i64 {
    fn sql_literal(&self) -> String {
        todo!()
    }
}

impl SqlValue for bool {
    fn sql_literal(&self) -> String {
        todo!()
    }
}

impl SqlValue for str {
    fn sql_literal(&self) -> String {
        todo!()
    }
}

impl SqlValue for String {
    fn sql_literal(&self) -> String {
        todo!()
    }
}

impl<T: SqlValue> SqlValue for Option<T> {
    fn sql_literal(&self) -> String {
        todo!()
    }
}
