//! Crash-safe file writes: write a temp file in the destination directory, then
//! rename it over the target. Readers see either the old or the new file.

use std::io::Write;
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::Result;

/// Writes a file atomically.
///
/// `write` receives a buffered sink for a temp file created in `path`'s directory.
/// If it fails, the destination is untouched and the temp file is removed. On
/// success the temp file is synced and renamed onto `path`.
///
/// # Errors
/// [`Error::Io`](crate::error::Error::Io) for any failure, with `path` as the destination.
pub fn write_atomic(_path: &Path, _write: impl FnOnce(&mut dyn Write) -> std::io::Result<()>) -> Result<()> {
    todo!()
}

/// Serializes `value` as pretty JSON and writes it atomically.
///
/// Encoding happens before any file is touched, so a serialization failure leaves
/// the destination as it was.
///
/// # Errors
/// [`Error::Json`](crate::error::Error::Json) if encoding fails,
/// [`Error::Io`](crate::error::Error::Io) if writing fails.
pub fn save_json_atomic<T: Serialize>(_value: &T, _path: &Path) -> Result<()> {
    todo!()
}

/// Reads and deserializes a JSON file.
///
/// # Errors
/// [`Error::Io`](crate::error::Error::Io) if the file cannot be opened,
/// [`Error::Json`](crate::error::Error::Json) if it does not parse.
pub fn load_json<T: DeserializeOwned>(_path: &Path) -> Result<T> {
    todo!()
}
