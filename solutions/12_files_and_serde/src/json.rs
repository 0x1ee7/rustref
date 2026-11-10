//! Manifest JSON in streaming form, plus internally tagged event records.
//!
//! Wire format of a manifest: a top-level JSON array of
//! `{"path": "<key>", "entry": {"size", "mtime", "sha256"}}` objects, in path order.

use std::collections::btree_map::Entry;
use std::fmt;
use std::io::{BufReader, BufWriter, Read, Write};

use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::scan::{FileEntry, Manifest};

#[derive(Serialize)]
struct RecordRef<'a> {
    path: &'a str,
    entry: &'a FileEntry,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    path: String,
    entry: FileEntry,
}

/// Writes `manifest` as a JSON array, one record at a time, without building an
/// intermediate `Value`.
///
/// # Errors
/// [`Error::Json`] on encoding or I/O failure.
pub fn write_manifest<W: Write>(writer: W, manifest: &Manifest) -> Result<()> {
    use serde::Serializer as _;

    let mut out = BufWriter::new(writer);
    serde_json::Serializer::new(&mut out).collect_seq(
        manifest
            .iter()
            .map(|(path, entry)| RecordRef { path, entry }),
    )?;
    out.flush().map_err(serde_json::Error::io)?;
    Ok(())
}

/// Streams records from a JSON manifest array into `each`, in document order.
///
/// Memory use is independent of the array length. If `each` returns an error,
/// parsing stops immediately and that error is returned unchanged.
///
/// # Errors
/// [`Error::Json`] for malformed input, or whatever `each` returns.
pub fn read_manifest_each<R: Read>(
    reader: R,
    each: impl FnMut(String, FileEntry) -> Result<()>,
) -> Result<usize> {
    use serde::Deserializer as _;

    let mut stream = Stream {
        each,
        count: 0,
        error: None,
    };
    let parsed = serde_json::Deserializer::from_reader(BufReader::new(reader))
        .deserialize_seq(&mut stream);
    if let Some(error) = stream.error {
        return Err(error);
    }
    parsed?;
    Ok(stream.count)
}

/// Reads a whole manifest, rejecting duplicate paths.
///
/// # Errors
/// [`Error::Manifest`] on duplicate paths, [`Error::Json`] on malformed input.
pub fn read_manifest<R: Read>(reader: R) -> Result<Manifest> {
    let mut manifest = Manifest::new();
    read_manifest_each(reader, |path, entry| match manifest.entry(path) {
        Entry::Occupied(occupied) => Err(Error::Manifest(format!(
            "duplicate path `{}`",
            occupied.key()
        ))),
        Entry::Vacant(vacant) => {
            vacant.insert(entry);
            Ok(())
        }
    })?;
    Ok(manifest)
}

/// Visitor that feeds each array element to a callback. Callback errors are
/// stashed in `error` because serde cannot carry an arbitrary error type through
/// `D::Error`.
struct Stream<F> {
    each: F,
    count: usize,
    error: Option<Error>,
}

impl<'de, F> Visitor<'de> for &mut Stream<F>
where
    F: FnMut(String, FileEntry) -> Result<()>,
{
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an array of manifest records")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<(), A::Error> {
        while let Some(record) = seq.next_element::<Record>()? {
            if let Err(error) = (self.each)(record.path, record.entry) {
                self.error = Some(error);
                return Err(serde::de::Error::custom("callback rejected a record"));
            }
            self.count += 1;
        }
        Ok(())
    }
}

/// One step in a sync run, stored as a JSON object tagged by `event`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}
