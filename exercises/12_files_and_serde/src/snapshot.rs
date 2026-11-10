//! Versioned binary snapshot header, big-endian, with a CRC-32 over the payload.
//!
//! Layout (all integers big-endian):
//!
//! | offset | v1 | v2 | field |
//! |-------:|:--:|:--:|-------|
//! | 0 | 4 | 4 | magic `RSSN` |
//! | 4 | 2 | 2 | version (`1` or `2`) |
//! | 6 | 2 | 2 | flags |
//! | 8 | 4 | 4 | entry count |
//! | 12 | – | 8 | created, signed microseconds since the Unix epoch (i64) |
//! | 20 | – | 4 | CRC-32 of the payload (IEEE) |
//! | 12 or 24 | … | … | payload |
//!
//! Writers always emit v2. Readers accept v1 (no timestamp, no checksum).

use std::path::Path;

use jiff::Timestamp;

use crate::error::{Result, SnapshotError};

/// File magic.
pub const MAGIC: [u8; 4] = *b"RSSN";
/// Version written by [`encode_snapshot`].
pub const CURRENT_VERSION: u16 = 2;

/// A decoded snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Format version (1 or 2).
    pub version: u16,
    /// Application flags.
    pub flags: u16,
    /// Number of entries the payload describes.
    pub entry_count: u32,
    /// Creation time; `None` for v1 files.
    pub created_at: Option<Timestamp>,
    /// Opaque payload bytes.
    pub payload: Vec<u8>,
}

/// Encodes a v2 snapshot in memory.
///
/// `created_at` is stored with microsecond precision (truncated).
///
/// # Errors
/// [`SnapshotError::FieldOverflow`] if `entry_count` exceeds `u32::MAX` or `flags`
/// exceeds `u16::MAX`.
pub fn encode_snapshot(
    _entry_count: usize,
    _created_at: Timestamp,
    _payload: &[u8],
    _flags: u32,
) -> std::result::Result<Vec<u8>, SnapshotError> {
    todo!()
}

/// Decodes a v1 or v2 snapshot from memory, verifying magic, length and checksum.
///
/// # Errors
/// Any [`SnapshotError`] variant describing the first structural problem found.
pub fn decode_snapshot(_bytes: &[u8]) -> std::result::Result<Snapshot, SnapshotError> {
    todo!()
}

/// Writes a v2 snapshot file atomically.
///
/// # Errors
/// [`Error::Snapshot`](crate::error::Error::Snapshot) for overflowing fields,
/// [`Error::Io`](crate::error::Error::Io) for filesystem failures.
pub fn write_snapshot(
    _path: &Path,
    _entry_count: usize,
    _created_at: Timestamp,
    _payload: &[u8],
    _flags: u32,
) -> Result<()> {
    todo!()
}

/// Reads and decodes a snapshot file.
///
/// # Errors
/// [`Error::Io`](crate::error::Error::Io) if the file cannot be read,
/// [`Error::Snapshot`](crate::error::Error::Snapshot) if it is malformed.
pub fn read_snapshot(_path: &Path) -> Result<Snapshot> {
    todo!()
}
