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

use std::fs;
use std::path::Path;

use jiff::Timestamp;

use crate::atomic::write_atomic;
use crate::error::{Error, Result, SnapshotError};
use crate::hash::crc32;

/// File magic.
pub const MAGIC: [u8; 4] = *b"RSSN";
/// Version written by [`encode_snapshot`].
pub const CURRENT_VERSION: u16 = 2;
const V1_HEADER_LEN: usize = 12;
const V2_HEADER_LEN: usize = 24;

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
    entry_count: usize,
    created_at: Timestamp,
    payload: &[u8],
    flags: u32,
) -> std::result::Result<Vec<u8>, SnapshotError> {
    let entry_count = u32::try_from(entry_count).map_err(|_| SnapshotError::FieldOverflow {
        field: "entry_count",
        value: entry_count as u64,
        bits: 32,
    })?;
    let flags = u16::try_from(flags).map_err(|_| SnapshotError::FieldOverflow {
        field: "flags",
        value: u64::from(flags),
        bits: 16,
    })?;
    let mut out = Vec::with_capacity(V2_HEADER_LEN + payload.len());
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&CURRENT_VERSION.to_be_bytes());
    out.extend_from_slice(&flags.to_be_bytes());
    out.extend_from_slice(&entry_count.to_be_bytes());
    out.extend_from_slice(&created_at.as_microsecond().to_be_bytes());
    out.extend_from_slice(&crc32(payload).to_be_bytes());
    out.extend_from_slice(payload);
    Ok(out)
}

/// Decodes a v1 or v2 snapshot from memory, verifying magic, length and checksum.
///
/// # Errors
/// Any [`SnapshotError`] variant describing the first structural problem found.
pub fn decode_snapshot(bytes: &[u8]) -> std::result::Result<Snapshot, SnapshotError> {
    if bytes.len() < V1_HEADER_LEN {
        return Err(SnapshotError::Truncated {
            needed: V1_HEADER_LEN,
            actual: bytes.len(),
        });
    }
    let magic: [u8; 4] = bytes[0..4].try_into().expect("4 bytes");
    if magic != MAGIC {
        return Err(SnapshotError::BadMagic(magic));
    }
    let version = u16::from_be_bytes(bytes[4..6].try_into().expect("2 bytes"));
    let flags = u16::from_be_bytes(bytes[6..8].try_into().expect("2 bytes"));
    let entry_count = u32::from_be_bytes(bytes[8..12].try_into().expect("4 bytes"));
    match version {
        1 => Ok(Snapshot {
            version,
            flags,
            entry_count,
            created_at: None,
            payload: bytes[V1_HEADER_LEN..].to_vec(),
        }),
        2 => {
            if bytes.len() < V2_HEADER_LEN {
                return Err(SnapshotError::Truncated {
                    needed: V2_HEADER_LEN,
                    actual: bytes.len(),
                });
            }
            let micros = i64::from_be_bytes(bytes[12..20].try_into().expect("8 bytes"));
            let expected = u32::from_be_bytes(bytes[20..24].try_into().expect("4 bytes"));
            let payload = &bytes[V2_HEADER_LEN..];
            let actual = crc32(payload);
            if actual != expected {
                return Err(SnapshotError::ChecksumMismatch { expected, actual });
            }
            let created_at = Timestamp::from_microsecond(micros)
                .map_err(|_| SnapshotError::TimestampOutOfRange(micros))?;
            Ok(Snapshot {
                version,
                flags,
                entry_count,
                created_at: Some(created_at),
                payload: payload.to_vec(),
            })
        }
        other => Err(SnapshotError::UnsupportedVersion(other)),
    }
}

/// Writes a v2 snapshot file atomically.
///
/// # Errors
/// [`Error::Snapshot`] for overflowing fields, [`Error::Io`] for filesystem failures.
pub fn write_snapshot(
    path: &Path,
    entry_count: usize,
    created_at: Timestamp,
    payload: &[u8],
    flags: u32,
) -> Result<()> {
    let bytes = encode_snapshot(entry_count, created_at, payload, flags)?;
    write_atomic(path, |out| out.write_all(&bytes))
}

/// Reads and decodes a snapshot file.
///
/// # Errors
/// [`Error::Io`] if the file cannot be read, [`Error::Snapshot`] if it is malformed.
pub fn read_snapshot(path: &Path) -> Result<Snapshot> {
    let bytes = fs::read(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(decode_snapshot(&bytes)?)
}
