//! Part 3: `AsRef` and iterator arguments as the variadic analogue, plus `Cow`
//! for "maybe changed" results.

use std::borrow::Cow;

use ex02_function_parameters::{RequestBuilder, encode_query, join_path, percent_encode};

#[test]
fn part3_percent_encode_keeps_unreserved_and_borrows() {
    let got = percent_encode("AZaz09-._~");
    assert_eq!(got, "AZaz09-._~");
    assert!(
        matches!(&got, Cow::Borrowed(_)),
        "nothing to escape, so the input must be borrowed; got {got:?}"
    );
}

#[test]
fn part3_percent_encode_escapes_space_and_reserved_bytes() {
    assert_eq!(percent_encode("a b/c?d"), "a%20b%2Fc%3Fd");
}

#[test]
fn part3_percent_encode_uses_uppercase_hex_per_utf8_byte() {
    let got = percent_encode("é");
    assert_eq!(got, "%C3%A9", "multi-byte UTF-8 is escaped byte by byte");
    assert!(matches!(&got, Cow::Owned(_)));
}

#[test]
fn part3_join_path_trims_trailing_slashes_of_base() {
    assert_eq!(
        join_path("https://x.test/api//", ["v1", "users"]),
        "https://x.test/api/v1/users"
    );
}

#[test]
fn part3_join_path_encodes_each_segment() {
    assert_eq!(
        join_path("https://x.test", ["a/b", "c d"]),
        "https://x.test/a%2Fb/c%20d",
        "a slash inside a segment is data, not a separator"
    );
}

#[test]
fn part3_join_path_skips_empty_segments() {
    assert_eq!(
        join_path("https://x.test", ["", "x", ""]),
        "https://x.test/x"
    );
}

#[test]
fn part3_join_path_accepts_owned_and_borrowed_segments() {
    let base = "https://x.test";
    assert_eq!(
        join_path(base, vec![String::from("a")]),
        "https://x.test/a"
    );
    assert_eq!(join_path(base, ["b"]), "https://x.test/b");
}

#[test]
fn part3_encode_query_joins_pairs_in_iteration_order() {
    assert_eq!(
        encode_query([("q", "rust lang"), ("page", "2")]),
        "q=rust%20lang&page=2"
    );
}

#[test]
fn part3_encode_query_accepts_owned_keys_and_values() {
    let pairs = vec![(String::from("k"), String::from("a&b"))];
    assert_eq!(encode_query(pairs), "k=a%26b");
}

#[test]
fn part3_encode_query_keeps_repeated_keys() {
    assert_eq!(encode_query([("a", "1"), ("a", "2")]), "a=1&a=2");
}

#[test]
fn part3_full_url_appends_encoded_query_or_returns_plain_url() {
    let with_query = RequestBuilder::new("GET", "https://x.test/s")
        .query("q", "a&b")
        .build()
        .expect("valid request");
    assert_eq!(with_query.full_url(), "https://x.test/s?q=a%26b");

    let plain = RequestBuilder::new("GET", "https://x.test/s")
        .build()
        .expect("valid request");
    assert_eq!(plain.full_url(), "https://x.test/s");
}

#[test]
fn part3_full_url_uses_ampersand_when_url_already_has_a_query() {
    let req = RequestBuilder::new("GET", "https://x.test/s?fixed=1")
        .query("q", "2")
        .build()
        .expect("valid request");
    assert_eq!(req.full_url(), "https://x.test/s?fixed=1&q=2");
}
