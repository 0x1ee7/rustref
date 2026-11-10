use std::borrow::Cow;
use std::collections::BTreeMap;
use std::error::Error;

use ex11_ownership_and_lifetimes::ini::{parse, Document, ParseErrorKind};
use ex11_ownership_and_lifetimes::text::UnescapeError;

const SAMPLE: &str = "\
; service config
name = api
[server]
host = 0.0.0.0
port = 8080
[paths]
data = /var/lib/api
motd = hello\\tworld
";

/// True if `needle` lies inside the bytes of `haystack` (same allocation).
fn within(haystack: &str, needle: &str) -> bool {
    let start = haystack.as_ptr() as usize;
    let p = needle.as_ptr() as usize;
    p >= start && p + needle.len() <= start + haystack.len()
}

fn sample() -> Document<'static> {
    parse(SAMPLE).expect("sample parses")
}

#[test]
fn part2_parses_sections_in_file_order() {
    let names: Vec<&str> = sample().sections().map(|s| s.name()).collect();
    assert_eq!(names, vec!["", "server", "paths"]);
}

#[test]
fn part2_top_level_entries_live_in_global_section() {
    let d = sample();
    assert_eq!(d.get("", "name"), Some("api"));
    assert!(d.section("").is_some());
}

#[test]
fn part2_no_global_section_when_file_starts_with_header() {
    let d = parse("[a]\nk=v\n").unwrap();
    assert!(d.section("").is_none(), "no top-level entries, so no global section");
}

#[test]
fn part2_lookups_are_exact_and_case_sensitive() {
    let d = sample();
    assert_eq!(d.get("server", "port"), Some("8080"));
    assert_eq!(d.get("server", "missing"), None);
    assert_eq!(d.get("nope", "host"), None);
    assert_eq!(d.get("Server", "host"), None, "section names are case-sensitive");
    assert_eq!(d.get("server", "HOST"), None, "keys are case-sensitive");
}

#[test]
fn part2_names_and_keys_borrow_from_input() {
    let d = sample();
    for section in d.sections() {
        // The global section's name is the static `""`, not a slice of the input.
        if !section.name().is_empty() {
            assert!(within(SAMPLE, section.name()), "section {:?} must borrow from input", section.name());
        }
        for entry in section.entries() {
            assert!(within(SAMPLE, entry.key), "key {:?} must borrow from input", entry.key);
        }
    }
}

#[test]
fn part2_plain_values_borrow_from_input() {
    let d = sample();
    let host = d.section("server").unwrap().entries().find(|e| e.key == "host").unwrap();
    assert!(matches!(host.value, Cow::Borrowed(_)), "unescaped value must not be copied");
    assert!(within(SAMPLE, &host.value));
}

#[test]
fn part2_escaped_values_are_decoded_into_owned() {
    let d = sample();
    let motd = d.section("paths").unwrap().entries().find(|e| e.key == "motd").unwrap();
    assert!(matches!(motd.value, Cow::Owned(_)), "escaped value must be owned");
    assert_eq!(motd.value, "hello\tworld");
}

#[test]
fn part2_skips_blanks_comments_and_trims_including_crlf() {
    let input = "  ; c1\n\n# c2\r\n[s]\r\n  k  =  padded value  \r\nurl = http://h/?a=b\n";
    let d = parse(input).expect("valid");
    assert_eq!(d.get("s", "k"), Some("padded value"));
    assert_eq!(d.get("s", "url"), Some("http://h/?a=b"), "only the first '=' separates");
    assert_eq!(d.section("s").unwrap().entries().count(), 2);
}

#[test]
fn part2_error_carries_one_based_line_and_kind() {
    let err = parse("[a]\nk=v\n\nbroken line\n").unwrap_err();
    assert_eq!(err.line, 4);
    assert_eq!(err.kind, ParseErrorKind::MissingEquals);
}

#[test]
fn part2_structural_errors_are_reported() {
    let cases: [(&str, ParseErrorKind); 3] = [
        ("[a\n", ParseErrorKind::MalformedSection),
        ("[]\n", ParseErrorKind::MalformedSection),
        ("=v\n", ParseErrorKind::EmptyKey),
    ];
    for (input, expected) in cases {
        assert_eq!(parse(input).unwrap_err().kind, expected, "input {input:?}");
    }
}

#[test]
fn part2_duplicate_sections_and_keys_are_errors() {
    let dup_section = parse("[a]\nk=1\n[a]\n").unwrap_err();
    assert_eq!(dup_section.kind, ParseErrorKind::DuplicateSection("a".into()));
    assert_eq!(dup_section.line, 3);

    let dup_key = parse("[a]\nk=1\nk=2\n").unwrap_err();
    assert_eq!(
        dup_key.kind,
        ParseErrorKind::DuplicateKey { section: "a".into(), key: "k".into() }
    );
    assert_eq!(dup_key.line, 3);
}

#[test]
fn part2_same_key_in_different_sections_is_fine() {
    let d = parse("[a]\nk=1\n[b]\nk=2\n").unwrap();
    assert_eq!(d.get("a", "k"), Some("1"));
    assert_eq!(d.get("b", "k"), Some("2"));
}

#[test]
fn part2_bad_escape_is_reported_with_value_offset() {
    let err = parse("[a]\nk = bad\\q\n").unwrap_err();
    assert_eq!(err.line, 2);
    assert_eq!(
        err.kind,
        ParseErrorKind::Escape(UnescapeError::UnknownEscape { offset: 3, found: 'q' })
    );
}

#[test]
fn part2_error_display_and_source() {
    let dup = parse("[a]\nk=1\nk=2\n").unwrap_err();
    let text = dup.to_string();
    assert!(text.contains("line 3"), "display must name the line: {text}");
    assert!(text.contains("duplicate key"), "display must name the problem: {text}");
    assert!(dup.source().is_none());

    let esc = parse("[a]\nk = \\z\n").unwrap_err();
    assert!(esc.source().is_some(), "escape errors expose the UnescapeError as source");
}

#[test]
fn part2_get_as_parses_or_reports_failure() {
    let d = sample();
    let server = d.section("server").unwrap();
    assert_eq!(server.get_as::<u16>("port"), Some(Ok(8080)));
    assert!(matches!(server.get_as::<u16>("host"), Some(Err(_))));
    assert!(server.get_as::<u16>("missing").is_none());
}

#[test]
fn part2_owned_map_outlives_the_source_text() {
    fn build() -> BTreeMap<String, BTreeMap<String, String>> {
        let text = String::from(SAMPLE);
        let d = parse(&text).unwrap();
        d.to_owned_map()
        // `text` is dropped here; the map must not borrow from it.
    }
    let owned = build();
    assert_eq!(owned["server"]["port"], "8080");
    assert_eq!(owned["paths"]["motd"], "hello\tworld");
    assert_eq!(owned[""]["name"], "api");
}
