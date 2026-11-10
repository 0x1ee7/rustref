//! Streaming SHA-256 and CRC-32, implemented from the specifications.
//!
//! Stubs: implement the bodies. Keep the public surface as declared.

use std::fmt;
use std::io::{self, Write};
use std::path::Path;

use crate::error::Result;

/// A 256-bit SHA-256 digest.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Digest(pub [u8; 32]);

impl fmt::Display for Digest {
    /// Lowercase hexadecimal, 64 characters.
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

/// Incremental SHA-256 hasher. Implements [`io::Write`] so it can be the sink of
/// [`io::copy`].
#[derive(Clone)]
pub struct Sha256 {
    // Engineer-owned fields: chaining state, pending block buffer, byte count.
}

impl Default for Sha256 {
    fn default() -> Self {
        todo!()
    }
}

impl Sha256 {
    /// Starts a fresh hash.
    #[must_use]
    pub fn new() -> Self {
        todo!()
    }

    /// One-shot digest of a slice.
    #[must_use]
    pub fn digest(_data: &[u8]) -> Digest {
        todo!()
    }

    /// Feeds bytes into the hash. Any split of the input yields the same digest.
    pub fn update(&mut self, _data: &[u8]) {
        todo!()
    }

    /// Pads, finishes the last block(s) and returns the digest.
    #[must_use]
    pub fn finalize(self) -> Digest {
        todo!()
    }
}

impl Write for Sha256 {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> io::Result<()> {
        todo!()
    }
}

/// Streams a file through SHA-256 in fixed-size chunks; never loads it whole.
///
/// # Errors
/// [`Error::Io`](crate::error::Error::Io) if the file cannot be opened or read.
pub fn sha256_file(_path: &Path) -> Result<Digest> {
    todo!()
}

/// Incremental CRC-32 (IEEE 802.3, reflected polynomial `0xEDB88320`).
#[derive(Debug, Clone, Copy)]
pub struct Crc32 {
    // Engineer-owned fields: running register.
}

impl Crc32 {
    /// Starts a fresh checksum.
    #[must_use]
    pub fn new() -> Self {
        todo!()
    }

    /// Feeds bytes into the checksum.
    pub fn update(&mut self, _data: &[u8]) {
        todo!()
    }

    /// Final checksum value.
    #[must_use]
    pub fn finalize(self) -> u32 {
        todo!()
    }
}

/// One-shot CRC-32 of a slice.
#[must_use]
pub fn crc32(_data: &[u8]) -> u32 {
    todo!()
}
