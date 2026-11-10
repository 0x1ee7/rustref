//! Semicolon-delimited CSV report via `csv` + serde.

use std::collections::BTreeMap;
use std::io::{Read, Write};

use jiff::Timestamp;
use serde::Deserialize;

use crate::codec;
use crate::error::Result;
use crate::hash::Digest;
use crate::scan::Manifest;

/// Column names, in order.
pub const REPORT_HEADERS: [&str; 5] = ["path", "size", "mtime", "sha256", "note"];

/// One row of a report, as read back.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ReportRow {
    /// Relative path.
    pub path: String,
    /// Size in bytes.
    pub size: u64,
    /// Modification time (UTC, microseconds).
    #[serde(with = "codec::rfc3339")]
    pub mtime: Timestamp,
    /// SHA-256 of the contents.
    #[serde(with = "codec::hex_digest")]
    pub sha256: Digest,
    /// Free-form note; empty when none was given.
    pub note: String,
}

/// Writes the manifest as a `;`-separated CSV with the [`REPORT_HEADERS`] header.
///
/// Rows follow manifest (path) order. Notes are keyed by path. Every note key
/// must exist in the manifest; this is checked before anything is written.
/// Fields are quoted only when needed (RFC 4180 style). Lines end in `\n`.
///
/// # Errors
/// [`Error::UnknownNote`](crate::error::Error::UnknownNote) for a note whose path is
/// absent; [`Error::Csv`](crate::error::Error::Csv) on I/O.
pub fn write_report<W: Write>(
    _writer: W,
    _manifest: &Manifest,
    _notes: &BTreeMap<String, String>,
) -> Result<()> {
    todo!()
}

/// Reads a report back. A leading UTF-8 BOM (as written by spreadsheet tools) is
/// stripped. CRLF line endings are accepted.
///
/// # Errors
/// [`Error::Report`](crate::error::Error::Report) if the header differs from
/// [`REPORT_HEADERS`]; [`Error::Csv`](crate::error::Error::Csv) for malformed rows.
pub fn read_report<R: Read>(_reader: R) -> Result<Vec<ReportRow>> {
    todo!()
}
