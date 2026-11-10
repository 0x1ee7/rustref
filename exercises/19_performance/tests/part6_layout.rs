//! Part 6: layout: `size_of`, field order, niches, enum sizes, explicit wire encoding.
use ex19_performance::layout::{Link, Value, WireHeader, parse_scalar};
use ex19_performance::log_line::{Level, parse_line};
use std::mem::size_of;
use std::num::NonZeroU32;

#[test]
fn part6_wire_header_is_16_bytes_after_reordering() {
    assert_eq!(
        size_of::<WireHeader>(),
        16,
        "reorder the fields (largest first) so repr(C) has no interior padding"
    );
    let header = WireHeader::new(1, 1_700_000_000_000, 0b101, 512, 9);
    assert_eq!(header.version, 1);
    assert_eq!(header.timestamp_ms, 1_700_000_000_000);
    assert_eq!(header.flags, 0b101);
    assert_eq!(header.length, 512);
    assert_eq!(header.seq, 9);
}

#[test]
fn part6_wire_bytes_are_explicit_little_endian_at_fixed_offsets() {
    let header = WireHeader::new(1, 0x0102_0304_0506_0708, 0xAB, 0x1234, 0xDEAD_BEEF);
    let expected: [u8; 16] = [
        1, // version @0
        0xAB, // flags @1
        0x34, 0x12, // length @2..4, little-endian
        0xEF, 0xBE, 0xAD, 0xDE, // seq @4..8, little-endian
        0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, // timestamp_ms @8..16
    ];
    assert_eq!(header.to_bytes(), expected);
    assert_eq!(WireHeader::from_bytes(expected), header);
}

#[test]
fn part6_wire_header_round_trips_through_bytes() {
    for header in [
        WireHeader::new(0, 0, 0, 0, 0),
        WireHeader::new(u8::MAX, u64::MAX, u8::MAX, u16::MAX, u32::MAX),
        WireHeader::new(2, 42, 1, 65_535, 7),
    ] {
        assert_eq!(WireHeader::from_bytes(header.to_bytes()), header);
    }
}

#[test]
fn part6_option_nonzero_niche_keeps_link_at_four_bytes() {
    assert_eq!(size_of::<Option<NonZeroU32>>(), size_of::<u32>(), "std niche fact");
    assert_eq!(size_of::<Option<u32>>(), 8, "std fact: no niche in u32");
    assert_eq!(size_of::<Link>(), 4, "Link stores Option<NonZeroU32>, which is 4 bytes");
    assert_eq!(
        size_of::<Option<Link>>(),
        8,
        "every u32 is a valid id, so Link has no spare value left for Option's tag"
    );
    assert_eq!(Link::new(0).parent(), None, "0 means no parent");
    assert_eq!(Link::new(7).parent(), NonZeroU32::new(7));
}

#[test]
fn part6_link_round_trips_extreme_ids() {
    assert_eq!(Link::new(u32::MAX).parent(), NonZeroU32::new(u32::MAX));
    assert_eq!(Link::new(1).parent(), NonZeroU32::new(1));
}

#[test]
fn part6_value_enum_is_16_bytes_and_parses_scalars() {
    assert_eq!(size_of::<Value>(), 16, "tag plus an 8-byte payload");
    assert_eq!(size_of::<Option<Value>>(), 16, "Option<Value> should use the tag niche");
    assert_eq!(parse_scalar("42"), Some(Value::Int(42)));
}

#[test]
fn part6_parse_scalar_classifies_tokens() {
    let cases: [(&str, Option<Value>); 11] = [
        ("null", Some(Value::Null)),
        ("true", Some(Value::Bool(true))),
        ("false", Some(Value::Bool(false))),
        ("-7", Some(Value::Int(-7))),
        ("3.5", Some(Value::Float(3.5))),
        ("1e3", Some(Value::Float(1000.0))),
        ("NaN", None),
        ("inf", None),
        ("12abc", None),
        ("", None),
        (" 1", None),
    ];
    for (raw, expected) in cases {
        assert_eq!(parse_scalar(raw), expected, "input {raw:?}");
    }
}

#[test]
fn part6_level_is_one_byte_with_a_niche() {
    assert_eq!(size_of::<Level>(), 1, "a fieldless enum with five variants");
    assert_eq!(size_of::<Option<Level>>(), 1, "Option<Level> uses an unused tag value");
    let line = parse_line("2026-11-10T12:00:00Z WARN svc::x disk 91%").unwrap();
    assert_eq!(line.level, Level::Warn);
}
