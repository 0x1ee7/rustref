//! `#[serde(with = ...)]` modules for types whose default serde shape is wrong
//! for a manifest: timestamps as RFC 3339 strings, digests as hex strings.

/// `jiff::Timestamp` as an RFC 3339 string (`2024-01-01T00:00:00.000001Z`).
pub mod rfc3339 {
    use jiff::Timestamp;
    use serde::de::Error as _;
    use serde::{Deserialize, Deserializer, Serializer};

    /// Serializes via `Display`, which is RFC 3339 in UTC.
    pub fn serialize<S: Serializer>(ts: &Timestamp, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(ts)
    }

    /// Parses any RFC 3339 instant; offsets are normalised to UTC.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Timestamp, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse::<Timestamp>().map_err(D::Error::custom)
    }
}

/// [`Digest`](crate::hash::Digest) as 64 hex characters.
pub mod hex_digest {
    use serde::de::Error as _;
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::hash::Digest;

    /// Serializes as lowercase hex.
    pub fn serialize<S: Serializer>(digest: &Digest, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(digest)
    }

    /// Accepts exactly 64 hex digits, either case.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Digest, D::Error> {
        let text = String::deserialize(deserializer)?;
        if text.len() != 64 {
            return Err(D::Error::custom(format!(
                "expected 64 hex characters, got {}",
                text.len()
            )));
        }
        if !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(D::Error::custom("digest contains non-hex characters"));
        }
        let mut out = [0u8; 32];
        for (i, byte) in out.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&text[2 * i..2 * i + 2], 16).map_err(D::Error::custom)?;
        }
        Ok(Digest(out))
    }
}
