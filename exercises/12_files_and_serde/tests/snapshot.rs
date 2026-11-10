//! Part 6: a versioned binary snapshot format with big-endian header fields and CRC-32.

use ex12_files_and_serde::hash::crc32;
use ex12_files_and_serde::snapshot::{
    CURRENT_VERSION, MAGIC, decode_snapshot, encode_snapshot, read_snapshot, write_snapshot,
};
use ex12_files_and_serde::{Error, SnapshotError};
use jiff::Timestamp;

const CREATED_MICROS: i64 = 1_700_000_000_000_000;

fn created() -> Timestamp {
    Timestamp::from_microsecond(CREATED_MICROS).expect("valid timestamp")
}

fn sample_bytes() -> Vec<u8> {
    encode_snapshot(3, created(), b"hello", 1).expect("encode")
}

#[test]
fn part6_v2_header_layout_is_big_endian_with_crc_of_payload() {
    assert_eq!(MAGIC, *b"RSSN", "magic bytes");
    assert_eq!(CURRENT_VERSION, 2, "the writer emits v2");

    let bytes = sample_bytes();
    assert_eq!(bytes.len(), 24 + 5, "24-byte header followed by the payload");
    assert_eq!(&bytes[0..4], b"RSSN", "magic");
    assert_eq!(&bytes[4..6], &2u16.to_be_bytes(), "version, big-endian");
    assert_eq!(&bytes[6..8], &1u16.to_be_bytes(), "flags, big-endian");
    assert_eq!(&bytes[8..12], &3u32.to_be_bytes(), "entry count, big-endian");
    assert_eq!(&bytes[12..20], &CREATED_MICROS.to_be_bytes(), "created time in micros");
    assert_eq!(&bytes[20..24], &crc32(b"hello").to_be_bytes(), "CRC-32 of the payload");
    assert_eq!(&bytes[24..], b"hello", "payload is stored verbatim");
}

#[test]
fn part6_round_trip_preserves_every_field() {
    let snap = decode_snapshot(&sample_bytes()).expect("decode");
    assert_eq!(snap.version, 2);
    assert_eq!(snap.flags, 1);
    assert_eq!(snap.entry_count, 3);
    assert_eq!(snap.created_at, Some(created()));
    assert_eq!(snap.payload, b"hello");
}

#[test]
fn part6_checksum_detects_payload_corruption() {
    let mut bytes = sample_bytes();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    assert!(
        matches!(decode_snapshot(&bytes), Err(SnapshotError::ChecksumMismatch { .. })),
        "a flipped payload bit must fail the CRC check"
    );
}

#[test]
fn part6_bad_magic_and_truncated_header_are_structural_errors() {
    let mut bytes = sample_bytes();
    bytes[0] = b'X';
    assert!(
        matches!(decode_snapshot(&bytes), Err(SnapshotError::BadMagic(_))),
        "wrong magic must be BadMagic"
    );

    let short = &sample_bytes()[..10];
    assert!(
        matches!(decode_snapshot(short), Err(SnapshotError::Truncated { .. })),
        "a header cut short must be Truncated"
    );
}

#[test]
fn part6_unsupported_version_is_reported_with_its_number() {
    let mut bytes = sample_bytes();
    bytes[4..6].copy_from_slice(&9u16.to_be_bytes());
    assert!(
        matches!(decode_snapshot(&bytes), Err(SnapshotError::UnsupportedVersion(9))),
        "unknown versions must be rejected, not guessed at"
    );
}

#[test]
fn part6_reads_legacy_v1_without_timestamp_or_checksum() {
    let mut v1 = Vec::new();
    v1.extend_from_slice(b"RSSN");
    v1.extend_from_slice(&1u16.to_be_bytes());
    v1.extend_from_slice(&0u16.to_be_bytes());
    v1.extend_from_slice(&2u32.to_be_bytes());
    v1.extend_from_slice(b"legacy");

    let snap = decode_snapshot(&v1).expect("v1 is still readable");
    assert_eq!(snap.version, 1);
    assert_eq!(snap.created_at, None, "v1 has no creation time");
    assert_eq!(snap.entry_count, 2);
    assert_eq!(snap.payload, b"legacy");
}

#[test]
fn part6_encode_rejects_flags_that_do_not_fit_in_u16() {
    let result = encode_snapshot(1, created(), b"", 70_000);
    assert!(
        matches!(result, Err(SnapshotError::FieldOverflow { field: "flags", bits: 16, .. })),
        "flags above u16::MAX must be FieldOverflow, got {result:?}"
    );
}

#[test]
fn part6_decode_rejects_timestamp_outside_representable_range() {
    let mut bytes = sample_bytes();
    bytes[12..20].copy_from_slice(&i64::MAX.to_be_bytes());
    assert!(
        matches!(decode_snapshot(&bytes), Err(SnapshotError::TimestampOutOfRange(v)) if v == i64::MAX),
        "an unrepresentable timestamp must be rejected"
    );
}

#[test]
fn part6_file_round_trip_keeps_pre_epoch_timestamps() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("state.rssn");
    let before_epoch = Timestamp::from_microsecond(-1_500_000).expect("valid timestamp");

    write_snapshot(&path, 7, before_epoch, b"payload", 0).expect("write");
    let snap = read_snapshot(&path).expect("read");

    assert_eq!(snap.entry_count, 7);
    assert_eq!(snap.created_at, Some(before_epoch), "pre-1970 times must survive the round trip");
    assert_eq!(snap.payload, b"payload");
    assert!(
        !matches!(read_snapshot(&dir.path().join("missing.rssn")), Err(Error::Snapshot(_))),
        "a missing file is an I/O problem, not a format problem"
    );
}
