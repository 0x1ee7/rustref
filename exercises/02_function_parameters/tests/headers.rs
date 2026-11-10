//! Part 1: header names and values. Borrowed vs owned results, and `AsRef` /
//! `Into` inputs.

use std::borrow::Cow;

use ex02_function_parameters::{Header, HeaderError, normalize_header_name};

#[test]
fn part1_normalize_trims_and_lowercases_into_owned() {
    let got = normalize_header_name("  Content-Type \t").expect("valid name");
    assert_eq!(got, "content-type", "name should be trimmed and lowercased");
    assert!(
        matches!(&got, Cow::Owned(_)),
        "lowercasing changed the name, so the result must be Owned; got {got:?}"
    );
}

#[test]
fn part1_normalize_borrows_when_nothing_changes() {
    let got = normalize_header_name(" x-request-id ").expect("valid name");
    assert_eq!(got, "x-request-id");
    assert!(
        matches!(&got, Cow::Borrowed(_)),
        "an already-lowercase trimmed name must borrow the input; got {got:?}"
    );
}

#[test]
fn part1_normalize_rejects_empty_and_non_token_names() {
    assert_eq!(normalize_header_name("   "), Err(HeaderError::EmptyName));
    assert_eq!(normalize_header_name(""), Err(HeaderError::EmptyName));
    assert_eq!(
        normalize_header_name("a b"),
        Err(HeaderError::InvalidNameChar(' ')),
        "interior whitespace is not a token character"
    );
    assert_eq!(
        normalize_header_name("a:b"),
        Err(HeaderError::InvalidNameChar(':'))
    );
    assert_eq!(
        normalize_header_name("naïve"),
        Err(HeaderError::InvalidNameChar('ï')),
        "non-ASCII is not a token character"
    );
}

#[test]
fn part1_header_new_accepts_str_and_string_inputs() {
    let from_str = Header::new("Accept", "*/*").expect("valid header");
    let from_string =
        Header::new(String::from("Accept"), String::from("*/*")).expect("valid header");
    assert_eq!(from_str.name(), "accept");
    assert_eq!(from_str, from_string, "&str and String inputs must agree");
    assert_eq!(
        Header::new("", "v"),
        Err(HeaderError::EmptyName),
        "name errors propagate out of Header::new"
    );
}

#[test]
fn part1_header_normalizes_name_and_keeps_value_verbatim() {
    let h = Header::new(" X-Trace ", " a  b ").expect("valid header");
    assert_eq!(h.name(), "x-trace");
    assert_eq!(h.value(), " a  b ", "values are stored verbatim");
}

#[test]
fn part1_header_value_allows_htab_but_rejects_other_controls() {
    assert!(
        Header::new("x-tab", "a\tb").is_ok(),
        "HTAB is permitted inside a value"
    );
    assert_eq!(
        Header::new("x-nl", "a\nb"),
        Err(HeaderError::InvalidValue),
        "LF must be rejected"
    );
    assert_eq!(
        Header::new("x-del", "a\u{7f}b"),
        Err(HeaderError::InvalidValue),
        "DEL is an ASCII control character"
    );
}

#[test]
fn part1_header_into_parts_moves_strings_out() {
    let h = Header::new("X-A", String::from("v")).expect("valid header");
    let (name, value) = h.into_parts();
    assert_eq!((name.as_str(), value.as_str()), ("x-a", "v"));
}
