//! Struct and enum layout: field order, niches, and explicit wire encoding.

use std::num::NonZeroU32;

/// Fixed 16-byte wire header. Fields are declared largest-first, so `repr(C)`
/// needs no interior padding.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireHeader {
    pub timestamp_ms: u64,
    pub seq: u32,
    pub length: u16,
    pub version: u8,
    pub flags: u8,
}

impl WireHeader {
    /// Builds a header from its fields.
    pub fn new(version: u8, timestamp_ms: u64, flags: u8, length: u16, seq: u32) -> Self {
        Self {
            timestamp_ms,
            seq,
            length,
            version,
            flags,
        }
    }

    /// Explicit little-endian encoding; never transmute, since padding is uninitialised.
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out[0] = self.version;
        out[1] = self.flags;
        out[2..4].copy_from_slice(&self.length.to_le_bytes());
        out[4..8].copy_from_slice(&self.seq.to_le_bytes());
        out[8..16].copy_from_slice(&self.timestamp_ms.to_le_bytes());
        out
    }

    /// Decodes the encoding produced by [`WireHeader::to_bytes`].
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self {
            version: bytes[0],
            flags: bytes[1],
            length: u16::from_le_bytes([bytes[2], bytes[3]]),
            seq: u32::from_le_bytes(bytes[4..8].try_into().expect("4-byte slice")),
            timestamp_ms: u64::from_le_bytes(bytes[8..16].try_into().expect("8-byte slice")),
        }
    }
}

/// An optional parent id; `0` means "no parent". The `NonZeroU32` niche keeps it at 4 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    parent: Option<NonZeroU32>,
}

impl Link {
    /// `raw == 0` means no parent.
    pub fn new(raw: u32) -> Self {
        Self {
            parent: NonZeroU32::new(raw),
        }
    }

    /// The parent id, if any.
    pub fn parent(&self) -> Option<NonZeroU32> {
        self.parent
    }
}

/// A parsed scalar. Tag plus 8-byte payload: 16 bytes, and `Option<Value>` is also 16.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
}

/// Classifies a token. Non-finite floats and surrounding whitespace are rejected.
pub fn parse_scalar(raw: &str) -> Option<Value> {
    match raw {
        "null" => Some(Value::Null),
        "true" => Some(Value::Bool(true)),
        "false" => Some(Value::Bool(false)),
        _ => raw.parse::<i64>().map(Value::Int).ok().or_else(|| {
            raw.parse::<f64>()
                .ok()
                .filter(|f| f.is_finite())
                .map(Value::Float)
        }),
    }
}
