//! Part 5: RAII guards and resource cleanup.

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::marker::PhantomData;
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
    // private: the borrowed map and the change log
    _marker: PhantomData<&'a mut HashMap<String, String>>,
}

impl<'a> TempEnv<'a> {
    /// Wraps `_env`. Nothing changes until `set` is called.
    pub fn new(_env: &'a mut HashMap<String, String>) -> Self {
        todo!()
    }

    /// Sets `_key` to `_value`, or unsets it when `_value` is `None`.
    pub fn set(&mut self, _key: &str, _value: Option<&str>) {
        todo!()
    }

    /// Reads `_key` through the guard.
    pub fn get(&self, _key: &str) -> Option<&str> {
        todo!()
    }
}

// Engineer: restore the recorded changes on drop. The stub is a no-op so that
// unwinding through a guard does not abort the test binary.
impl Drop for TempEnv<'_> {
    fn drop(&mut self) {}
}

/// Opens every path for reading, in order.
///
/// On the first failure, files already opened are closed and [`FsError::Open`]
/// names the path that failed.
pub fn open_all<P: AsRef<Path>>(_paths: &[P]) -> Result<Vec<File>, FsError> {
    todo!()
}

/// Deletes every path. Missing files are skipped.
///
/// Returns how many files were actually deleted. Any other failure returns
/// [`FsError::Remove`] at once; earlier deletions stay done.
pub fn remove_files<P: AsRef<Path>>(_paths: &[P]) -> Result<usize, FsError> {
    todo!()
}
