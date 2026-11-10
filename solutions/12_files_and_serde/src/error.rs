//! Crate-wide error types.

use std::io;
use std::path::PathBuf;

/// Everything that can go wrong in this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An I/O operation failed on a specific path.
    #[error("I/O error on `{}`: {source}", path.display())]
    Io {
        /// Path the operation was performed on.
        path: PathBuf,
        /// Underlying OS error.
        #[source]
        source: io::Error,
    },
    /// The scan root does not exist.
    #[error("root `{}` does not exist", .0.display())]
    RootNotFound(PathBuf),
    /// The scan root exists but is not a directory.
    #[error("`{}` is not a directory", .0.display())]
    NotADirectory(PathBuf),
    /// A file name is not valid UTF-8, so it cannot be a manifest key.
    #[error("path `{}` is not valid UTF-8", .0.display())]
    NonUtf8Path(PathBuf),
    /// A manifest key failed path-safety validation.
    #[error("unsafe path `{path}`: {reason}")]
    UnsafePath {
        /// The offending key, verbatim.
        path: String,
        /// Why it was rejected.
        reason: &'static str,
    },
    /// A notes map refers to a path that is not in the manifest.
    #[error("note refers to `{0}`, which is not in the manifest")]
    UnknownNote(String),
    /// Configuration is semantically invalid.
    #[error("invalid config: {0}")]
    Config(String),
    /// A manifest document is semantically invalid (e.g. duplicate path).
    #[error("invalid manifest: {0}")]
    Manifest(String),
    /// A CSV report has the wrong shape.
    #[error("invalid report: {0}")]
    Report(String),
    /// A timestamp is outside the representable range.
    #[error("timestamp out of range: {0}")]
    Timestamp(#[from] jiff::Error),
    /// JSON (de)serialization failed.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// CSV parsing or writing failed.
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    /// A binary snapshot is malformed.
    #[error(transparent)]
    Snapshot(#[from] SnapshotError),
}

/// Crate-wide result alias.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Structural problems with a binary snapshot.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SnapshotError {
    /// Fewer bytes than the header requires.
    #[error("snapshot is truncated: needs {needed} bytes, got {actual}")]
    Truncated {
        /// Bytes the header requires.
        needed: usize,
        /// Bytes actually present.
        actual: usize,
    },
    /// The first four bytes are not the snapshot magic.
    #[error("bad magic {0:?}: not a snapshot file")]
    BadMagic([u8; 4]),
    /// A version this crate does not understand.
    #[error("unsupported snapshot version {0}")]
    UnsupportedVersion(u16),
    /// The CRC-32 recorded in the header does not match the payload.
    #[error("checksum mismatch: header records {expected:#010x}, payload hashes to {actual:#010x}")]
    ChecksumMismatch {
        /// Checksum stored in the header.
        expected: u32,
        /// Checksum computed over the payload.
        actual: u32,
    },
    /// A value does not fit in its header field.
    #[error("{field} = {value} does not fit in {bits} bits")]
    FieldOverflow {
        /// Name of the header field.
        field: &'static str,
        /// The rejected value.
        value: u64,
        /// Width of the field in bits.
        bits: u32,
    },
    /// The stored microsecond timestamp is out of range.
    #[error("created timestamp {0} µs is out of range")]
    TimestampOutOfRange(i64),
}
