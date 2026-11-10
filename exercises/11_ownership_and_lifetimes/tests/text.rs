use std::borrow::Cow;

use ex11_ownership_and_lifetimes::text::{escape, escape_into, unescape, UnescapeError};
use proptest::prelude::*;

#[test]
fn part1_escape_borrows_when_nothing_to_escape() {
    let input = "plain value 123";
    let out = escape(input);
    assert!(
        matches!(out, Cow::Borrowed(_)),
        "nothing to escape: expected Cow::Borrowed, got an allocation"
    );
    assert_eq!(out, "plain value 123");
    assert_eq!(out.as_ptr(), input.as_ptr(), "borrowed output must share the input bytes");
}

#[test]
fn part1_escape_allocates_only_when_escaping() {
    assert!(matches!(escape("a\\b"), Cow::Owned(_)));
    assert!(matches!(escape("a\nb"), Cow::Owned(_)));
    assert!(matches!(escape("plain"), Cow::Borrowed(_)));
}

#[test]
fn part1_escape_maps_special_characters() {
    let cases = [
        ("a\\b", "a\\\\b"),
        ("line\nbreak", "line\\nbreak"),
        ("tab\there", "tab\\there"),
        ("cr\rx", "cr\\rx"),
        ("say \"hi\"", "say \\\"hi\\\""),
        ("é ☃ unchanged", "é ☃ unchanged"),
    ];
    for (input, expected) in cases {
        assert_eq!(escape(input), expected, "escape({input:?})");
    }
}

#[test]
fn part1_escape_into_appends_to_existing_buffer() {
    let mut out = String::from("k=");
    escape_into("a\nb", &mut out);
    assert_eq!(out, "k=a\\nb", "escape_into must append, never clear");
    escape_into("x", &mut out);
    assert_eq!(out, "k=a\\nbx");
}

#[test]
fn part1_unescape_borrows_when_no_backslash() {
    let input = "no escapes here";
    let out = unescape(input).expect("valid input");
    assert!(matches!(out, Cow::Borrowed(_)), "expected Cow::Borrowed");
    assert_eq!(out.as_ptr(), input.as_ptr());
}

#[test]
fn part1_unescape_allocates_only_when_decoding() {
    assert!(matches!(unescape("a\\nb").unwrap(), Cow::Owned(_)));
    assert!(matches!(unescape("abc").unwrap(), Cow::Borrowed(_)));
}

#[test]
fn part1_unescape_decodes_known_sequences() {
    let cases = [
        ("a\\\\b", "a\\b"),
        ("line\\nbreak", "line\nbreak"),
        ("tab\\there", "tab\there"),
        ("cr\\rx", "cr\rx"),
        ("say \\\"hi\\\"", "say \"hi\""),
        ("\\\\\\n", "\\\n"),
    ];
    for (input, expected) in cases {
        assert_eq!(unescape(input).unwrap(), expected, "unescape({input:?})");
    }
}

#[test]
fn part1_unescape_reports_unknown_escape_with_byte_offset() {
    assert_eq!(
        unescape("ab\\qz"),
        Err(UnescapeError::UnknownEscape { offset: 2, found: 'q' })
    );
    // 'é' is two bytes, so the backslash sits at byte 2, not char 1.
    assert_eq!(
        unescape("é\\x"),
        Err(UnescapeError::UnknownEscape { offset: 2, found: 'x' }),
        "offsets are byte indices"
    );
}

#[test]
fn part1_unescape_rejects_trailing_backslash() {
    assert_eq!(unescape("abc\\"), Err(UnescapeError::TrailingBackslash { offset: 3 }));
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn part1_escape_then_unescape_roundtrips(s in ".*") {
        let escaped = escape(&s);
        prop_assert!(!escaped.contains('\n'), "escaped output must be single-line");
        prop_assert!(!escaped.contains('\t'), "escaped output must have no raw tabs");
        let back = unescape(&escaped).expect("escape output is always valid");
        prop_assert_eq!(back.as_ref(), s.as_str());
    }
}
