//! `#[serde(with = ...)]` modules for types whose default serde shape is wrong
//! for a manifest: timestamps as RFC 3339 strings, digests as hex strings.

/// `jiff::Timestamp` as an RFC 3339 string (`2024-01-01T00:00:00.000001Z`).
pub mod rfc3339 {
    use jiff::Timestamp;
    use serde::{Deserializer, Serializer};

    /// Serializes via `Display`, which is RFC 3339 in UTC.
    pub fn serialize<S: Serializer>(_ts: &Timestamp, _serializer: S) -> Result<S::Ok, S::Error> {
        todo!()
    }

    /// Parses any RFC 3339 instant; offsets are normalised to UTC.
    pub fn deserialize<'de, D: Deserializer<'de>>(_deserializer: D) -> Result<Timestamp, D::Error> {
        todo!()
    }
}

/// [`Digest`](crate::hash::Digest) as 64 hex characters.
pub mod hex_digest {
    use serde::{Deserializer, Serializer};

    use crate::hash::Digest;

    /// Serializes as lowercase hex.
    pub fn serialize<S: Serializer>(_digest: &Digest, _serializer: S) -> Result<S::Ok, S::Error> {
        todo!()
    }

    /// Accepts exactly 64 hex digits, either case.
    pub fn deserialize<'de, D: Deserializer<'de>>(_deserializer: D) -> Result<Digest, D::Error> {
        todo!()
    }
}
