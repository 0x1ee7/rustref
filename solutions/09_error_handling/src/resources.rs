//! Part 5: RAII guards and resource cleanup.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

/// Errors from the file helpers in this module.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FsError {
    /// A file could not be opened.
    #[error("cannot open {}", .path.display())]
    Open {
        /// The path that failed.
        path: PathBuf,
        /// The underlying OS error.
        #[source]
        source: io::Error,
    },
    /// A file could not be removed (a missing file is not an error).
    #[error("cannot remove {}", .path.display())]
    Remove {
        /// The path that failed.
        path: PathBuf,
        /// The underlying OS error.
        #[source]
        source: io::Error,
    },
}

/// Scoped changes to an injectable environment map.
///
/// `set` records each previous value; dropping the guard restores them in
/// reverse order. A value of `None` unsets the variable for the guard's
/// lifetime. Variables that did not exist before are removed on drop.
///
/// The map is injected instead of touching the process environment, because
/// `std::env::set_var` is `unsafe` in edition 2024.
pub struct TempEnv<'a> {
    env: &'a mut HashMap<String, String>,
    /// `(key, value before the change)`, oldest first.
    changes: Vec<(String, Option<String>)>,
}

impl<'a> TempEnv<'a> {
    /// Wraps `env`. Nothing changes until `set` is called.
    pub fn new(env: &'a mut HashMap<String, String>) -> Self {
        Self {
            env,
            changes: Vec::new(),
        }
    }

    /// Sets `key` to `value`, or unsets it when `value` is `None`.
    pub fn set(&mut self, key: &str, value: Option<&str>) {
        let previous = match value {
            Some(value) => self.env.insert(key.to_owned(), value.to_owned()),
            None => self.env.remove(key),
        };
        self.changes.push((key.to_owned(), previous));
    }

    /// Reads `key` through the guard.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.env.get(key).map(String::as_str)
    }
}

impl Drop for TempEnv<'_> {
    fn drop(&mut self) {
        for (key, previous) in self.changes.drain(..).rev() {
            match previous {
                Some(value) => {
                    self.env.insert(key, value);
                }
                None => {
                    self.env.remove(&key);
                }
            }
        }
    }
}

/// Opens every path for reading, in order.
///
/// On the first failure, files already opened are closed and [`FsError::Open`]
/// names the path that failed.
pub fn open_all<P: AsRef<Path>>(paths: &[P]) -> Result<Vec<File>, FsError> {
    paths
        .iter()
        .map(|path| {
            let path = path.as_ref();
            File::open(path).map_err(|source| FsError::Open {
                path: path.to_path_buf(),
                source,
            })
        })
        .collect()
}

/// Deletes every path. Missing files are skipped.
///
/// Returns how many files were actually deleted. Any other failure returns
/// [`FsError::Remove`] at once; earlier deletions stay done.
pub fn remove_files<P: AsRef<Path>>(paths: &[P]) -> Result<usize, FsError> {
    let mut removed = 0;
    for path in paths {
        let path = path.as_ref();
        match fs::remove_file(path) {
            Ok(()) => removed += 1,
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(FsError::Remove {
                    path: path.to_path_buf(),
                    source,
                });
            }
        }
    }
    Ok(removed)
}
