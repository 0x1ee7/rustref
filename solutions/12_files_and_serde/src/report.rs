//! Semicolon-delimited CSV report via `csv` + serde.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::codec;
use crate::error::{Error, Result};
use crate::hash::Digest;
use crate::scan::Manifest;

/// Column names, in order.
pub const REPORT_HEADERS: [&str; 5] = ["path", "size", "mtime", "sha256", "note"];

const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

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

/// Borrowing row used for writing, so no per-row `String` clones.
#[derive(Serialize)]
struct ReportRowRef<'a> {
    path: &'a str,
    size: u64,
    #[serde(with = "codec::rfc3339")]
    mtime: Timestamp,
    #[serde(with = "codec::hex_digest")]
    sha256: Digest,
    note: &'a str,
}

/// Writes the manifest as a `;`-separated CSV with the [`REPORT_HEADERS`] header.
///
/// Rows follow manifest (path) order. Notes are keyed by path. Every note key
/// must exist in the manifest; this is checked before anything is written.
/// Fields are quoted only when needed (RFC 4180 style). Lines end in `\n`.
///
/// # Errors
/// [`Error::UnknownNote`] for a note whose path is absent; [`Error::Csv`] on I/O.
pub fn write_report<W: Write>(
    writer: W,
    manifest: &Manifest,
    notes: &BTreeMap<String, String>,
) -> Result<()> {
    if let Some(unknown) = notes.keys().find(|key| !manifest.contains_key(key.as_str())) {
        return Err(Error::UnknownNote(unknown.clone()));
    }
    let mut wtr = csv::WriterBuilder::new()
        .delimiter(b';')
        .terminator(csv::Terminator::Any(b'\n'))
        .has_headers(false)
        .from_writer(writer);
    wtr.write_record(REPORT_HEADERS)?;
    for (path, entry) in manifest {
        wtr.serialize(ReportRowRef {
            path,
            size: entry.size,
            mtime: entry.mtime,
            sha256: entry.sha256,
            note: notes.get(path).map_or("", String::as_str),
        })?;
    }
    wtr.flush().map_err(csv::Error::from)?;
    Ok(())
}

/// Reads a report back. A leading UTF-8 BOM (as written by spreadsheet tools) is
/// stripped. CRLF line endings are accepted.
///
/// # Errors
/// [`Error::Report`] if the header differs from [`REPORT_HEADERS`];
/// [`Error::Csv`] for malformed rows (for example a non-numeric size).
pub fn read_report<R: Read>(reader: R) -> Result<Vec<ReportRow>> {
    let mut buffered = BufReader::new(reader);
    let has_bom = buffered
        .fill_buf()
        .map_err(csv::Error::from)?
        .starts_with(&UTF8_BOM);
    if has_bom {
        buffered.consume(UTF8_BOM.len());
    }
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(buffered);
    let found: Vec<String> = rdr.headers()?.iter().map(str::to_owned).collect();
    if found != REPORT_HEADERS {
        return Err(Error::Report(format!(
            "expected header {REPORT_HEADERS:?}, found {found:?}"
        )));
    }
    Ok(rdr.deserialize().collect::<csv::Result<Vec<ReportRow>>>()?)
}
