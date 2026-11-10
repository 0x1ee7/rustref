//! Tree scanning: walk a directory, hash files, build a [`Manifest`].

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::codec;
use crate::error::{Error, Result};
use crate::glob;
use crate::hash::{self, Digest};

/// Per-file metadata captured by a scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileEntry {
    /// Length in bytes.
    pub size: u64,
    /// Modification time, UTC, truncated to microseconds.
    #[serde(with = "codec::rfc3339")]
    pub mtime: Timestamp,
    /// SHA-256 of the contents.
    #[serde(with = "codec::hex_digest")]
    pub sha256: Digest,
}

/// Relative POSIX path (`pkg/mod.py`) to metadata. Sorted by key.
pub type Manifest = BTreeMap<String, FileEntry>;

/// Recursively scans `root`, skipping paths matched by `ignore`.
///
/// Symlinks and other non-regular files are skipped. Keys use `/` separators on
/// every platform.
///
/// # Errors
/// * [`Error::RootNotFound`] / [`Error::NotADirectory`] for a bad root.
/// * [`Error::NonUtf8Path`] for a file name that is not UTF-8.
/// * [`Error::Io`] for any other filesystem failure.
pub fn scan_tree(root: &Path, ignore: &[&str]) -> Result<Manifest> {
    let meta = match fs::metadata(root) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(Error::RootNotFound(root.to_path_buf()));
        }
        Err(source) => {
            return Err(Error::Io {
                path: root.to_path_buf(),
                source,
            });
        }
    };
    if !meta.is_dir() {
        return Err(Error::NotADirectory(root.to_path_buf()));
    }

    let mut manifest = Manifest::new();
    let mut stack: Vec<(PathBuf, String)> = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, prefix)) = stack.pop() {
        let entries = fs::read_dir(&dir).map_err(|source| io_at(&dir, source))?;
        for entry in entries {
            let entry = entry.map_err(|source| io_at(&dir, source))?;
            let path = entry.path();
            let file_name = entry.file_name();
            let name = file_name
                .to_str()
                .ok_or_else(|| Error::NonUtf8Path(path.clone()))?;
            let rel = if prefix.is_empty() {
                name.to_owned()
            } else {
                format!("{prefix}/{name}")
            };
            // `DirEntry::file_type` does not follow symlinks.
            let file_type = entry.file_type().map_err(|source| io_at(&path, source))?;
            if file_type.is_dir() {
                stack.push((path, rel));
            } else if file_type.is_file() {
                if glob::is_ignored(&rel, ignore) {
                    continue;
                }
                let meta = entry.metadata().map_err(|source| io_at(&path, source))?;
                let modified = meta.modified().map_err(|source| io_at(&path, source))?;
                let entry_meta = FileEntry {
                    size: meta.len(),
                    mtime: floor_to_micros(modified)?,
                    sha256: hash::sha256_file(&path)?,
                };
                manifest.insert(rel, entry_meta);
            }
        }
    }
    Ok(manifest)
}

fn io_at(path: &Path, source: io::Error) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// Converts a `SystemTime` to UTC, flooring to whole microseconds (also before 1970).
fn floor_to_micros(time: SystemTime) -> Result<Timestamp> {
    let nanos: i128 = match time.duration_since(UNIX_EPOCH) {
        Ok(after) => after.as_nanos() as i128,
        Err(before) => -(before.duration().as_nanos() as i128),
    };
    let micros = nanos.div_euclid(1_000) as i64;
    Ok(Timestamp::from_microsecond(micros)?)
}
