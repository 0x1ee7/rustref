//! Struct and enum layout: field order, niches, and explicit wire encoding.

use std::num::NonZeroU32;

/// Fixed 16-byte wire header.
///
/// The field order below is the naive one and is 24 bytes under `repr(C)`.
/// Reorder the declaration (not the fields or their types) so that the size is 16.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireHeader {
    pub version: u8,
    pub timestamp_ms: u64,
    pub flags: u8,
    pub length: u16,
    pub seq: u32,
}

impl WireHeader {
    /// Builds a header from its fields.
    pub fn new(_version: u8, _timestamp_ms: u64, _flags: u8, _length: u16, _seq: u32) -> Self {
        todo!()
    }

    /// Encodes the header as explicit little-endian fields at fixed offsets:
    /// version @0, flags @1, length @2..4, seq @4..8, timestamp_ms @8..16.
    pub fn to_bytes(&self) -> [u8; 16] {
        todo!()
    }

    /// Decodes the encoding produced by [`WireHeader::to_bytes`].
    pub fn from_bytes(_bytes: [u8; 16]) -> Self {
        todo!()
    }
}

/// An optional parent id. `0` means "no parent". Must be 4 bytes; `Option<Link>` is 8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    // Engineer-owned field: `Option<NonZeroU32>`.
}

impl Link {
    /// `raw == 0` means no parent.
    pub fn new(_raw: u32) -> Self {
        todo!()
    }

    /// The parent id, if any.
    pub fn parent(&self) -> Option<NonZeroU32> {
        todo!()
    }
}

/// A parsed scalar. Must be 16 bytes, with `Option<Value>` also 16.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
}

/// Classifies a token: `null`, `true`/`false`, a decimal `i64`, or a finite `f64`.
/// Anything else (including `NaN`, `inf`, or surrounding whitespace) is `None`.
pub fn parse_scalar(_raw: &str) -> Option<Value> {
    todo!()
}
