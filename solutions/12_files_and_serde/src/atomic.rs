//! Crash-safe file writes: write a temp file in the destination directory, then
//! rename it over the target. Readers see either the old or the new file.

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;
use tempfile::NamedTempFile;

use crate::error::{Error, Result};

/// Writes a file atomically.
///
/// `write` receives a buffered sink for a temp file created in `path`'s directory.
/// If it fails, the destination is untouched and the temp file is removed. On
/// success the temp file is synced and renamed onto `path`.
///
/// # Errors
/// [`Error::Io`] for any failure, with `path` as the destination.
pub fn write_atomic(path: &Path, write: impl FnOnce(&mut dyn Write) -> io::Result<()>) -> Result<()> {
    let io_err = |source| Error::Io {
        path: path.to_path_buf(),
        source,
    };
    let dir = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let mut tmp = NamedTempFile::new_in(dir).map_err(io_err)?;
    {
        let mut out = BufWriter::new(tmp.as_file_mut());
        write(&mut out).map_err(io_err)?;
        out.flush().map_err(io_err)?;
    }
    tmp.as_file().sync_all().map_err(io_err)?;
    tmp.persist(path).map_err(|e| io_err(e.error))?;
    Ok(())
}

/// Serializes `value` as pretty JSON and writes it atomically.
///
/// Encoding happens before any file is touched, so a serialization failure leaves
/// the destination as it was.
///
/// # Errors
/// [`Error::Json`] if encoding fails, [`Error::Io`] if writing fails.
pub fn save_json_atomic<T: Serialize>(value: &T, path: &Path) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    write_atomic(path, |out| {
        out.write_all(&bytes)?;
        out.write_all(b"\n")
    })
}

/// Reads and deserializes a JSON file.
///
/// # Errors
/// [`Error::Io`] if the file cannot be opened, [`Error::Json`] if it does not parse.
pub fn load_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let file = File::open(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(serde_json::from_reader(BufReader::new(file))?)
}
