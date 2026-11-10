//! Manifest JSON in streaming form, plus internally tagged event records.
//!
//! Wire format of a manifest: a top-level JSON array of
//! `{"path": "<key>", "entry": {"size", "mtime", "sha256"}}` objects, in path order.

use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::scan::{FileEntry, Manifest};

/// Writes `manifest` as a JSON array, one record at a time, without building an
/// intermediate `Value`.
///
/// # Errors
/// [`Error::Json`](crate::error::Error::Json) on encoding or I/O failure.
pub fn write_manifest<W: Write>(_writer: W, _manifest: &Manifest) -> Result<()> {
    todo!()
}

/// Streams records from a JSON manifest array into `each`, in document order.
///
/// Memory use is independent of the array length. If `each` returns an error,
/// parsing stops immediately and that error is returned unchanged.
///
/// # Errors
/// [`Error::Json`](crate::error::Error::Json) for malformed input, or whatever `each` returns.
pub fn read_manifest_each<R: Read>(
    _reader: R,
    _each: impl FnMut(String, FileEntry) -> Result<()>,
) -> Result<usize> {
    todo!()
}

/// Reads a whole manifest, rejecting duplicate paths.
///
/// # Errors
/// [`Error::Manifest`](crate::error::Error::Manifest) on duplicate paths,
/// [`Error::Json`](crate::error::Error::Json) on malformed input.
pub fn read_manifest<R: Read>(_reader: R) -> Result<Manifest> {
    todo!()
}

/// One step in a sync run, stored as a JSON object tagged by `event`.
///
/// Wire format: `{"event": "copied", "path": "…"}`, `{"event": "deleted", "path": "…"}`,
/// `{"event": "skipped", "path": "…", "reason": "…"}` (`reason` omitted when absent).
/// Unknown fields are rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A file was copied to the destination.
    Copied {
        /// Relative path.
        path: String,
    },
    /// A file was removed from the destination.
    Deleted {
        /// Relative path.
        path: String,
    },
    /// A file was not processed.
    Skipped {
        /// Relative path.
        path: String,
        /// Optional explanation; omitted from the output when absent.
        reason: Option<String>,
    },
}

impl Serialize for Event {
    fn serialize<S: serde::Serializer>(&self, _serializer: S) -> std::result::Result<S::Ok, S::Error> {
        todo!()
    }
}

impl<'de> Deserialize<'de> for Event {
    fn deserialize<D: serde::Deserializer<'de>>(_deserializer: D) -> std::result::Result<Self, D::Error> {
        todo!()
    }
}
