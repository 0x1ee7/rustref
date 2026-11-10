//! Part 2: a `&mut self` builder. Setters record problems and `build` reports
//! them, so the setters themselves never fail.

use std::time::Duration;

use ex02_function_parameters::{HeaderError, Method, RequestBuilder, RequestError};

fn configure(builder: &mut RequestBuilder) {
    builder
        .header("Accept", "application/json")
        .query("page", "2")
        .timeout(Duration::from_secs(3));
}

#[test]
fn part2_new_defaults_to_ten_second_timeout() {
    let req = RequestBuilder::new("GET", "https://x.test/")
        .build()
        .expect("valid request");
    assert_eq!(req.timeout(), Duration::from_secs(10));
}

#[test]
fn part2_method_is_parsed_case_insensitively_and_displayed_upper() {
    let req = RequestBuilder::new("post", "https://x.test/")
        .build()
        .expect("valid request");
    assert_eq!(req.method(), Method::Post);
    assert_eq!(req.method().to_string(), "POST");
    assert_eq!(Method::Delete.as_str(), "DELETE");
    assert_eq!(
        "connect".parse::<Method>(),
        Err(RequestError::InvalidMethod("connect".to_string())),
        "CONNECT is not a supported method"
    );
}

#[test]
fn part2_unknown_method_is_reported_by_build() {
    let result = RequestBuilder::new("fetch", "https://x.test/").build();
    assert_eq!(
        result,
        Err(RequestError::InvalidMethod("fetch".to_string()))
    );
}

#[test]
fn part2_method_error_is_reported_before_other_errors() {
    let mut builder = RequestBuilder::new("fetch", "ftp://x.test");
    builder.header("a b", "v");
    assert_eq!(
        builder.build(),
        Err(RequestError::InvalidMethod("fetch".to_string())),
        "the method is checked first"
    );
}

#[test]
fn part2_url_must_start_with_http_or_https_case_insensitively() {
    assert_eq!(
        RequestBuilder::new("GET", "ftp://x.test").build(),
        Err(RequestError::InvalidUrl("ftp://x.test".to_string()))
    );
    assert_eq!(
        RequestBuilder::new("GET", "x.test/no-scheme").build(),
        Err(RequestError::InvalidUrl("x.test/no-scheme".to_string()))
    );
    assert!(RequestBuilder::new("GET", "HTTPS://x.test/").build().is_ok());
    assert!(RequestBuilder::new("GET", "Http://x.test/").build().is_ok());
}

#[test]
fn part2_setters_configure_a_mutable_builder_in_place() {
    let mut builder = RequestBuilder::new("GET", "https://x.test/search");
    configure(&mut builder);
    let req = builder.build().expect("valid request");
    assert_eq!(req.header("accept"), Some("application/json"));
    assert_eq!(
        req.query().to_vec(),
        vec![("page".to_string(), "2".to_string())]
    );
    assert_eq!(req.timeout(), Duration::from_secs(3));
}

#[test]
fn part2_builder_can_be_reused_after_build() {
    let mut builder = RequestBuilder::new("GET", "https://x.test/");
    let first = builder.build().expect("valid request");
    builder.header("X-Later", "1");
    let second = builder.build().expect("valid request");
    assert_eq!(
        first.header("x-later"),
        None,
        "an earlier build must not see later changes"
    );
    assert_eq!(second.header("x-later"), Some("1"));
}

#[test]
fn part2_first_recorded_error_wins_and_is_deferred_to_build() {
    let mut builder = RequestBuilder::new("GET", "https://x.test/");
    // Setters must not panic or return errors; both problems are recorded.
    builder.header("a b", "v").header("c", "bad\nv");
    assert_eq!(
        builder.build(),
        Err(RequestError::InvalidHeader(HeaderError::InvalidNameChar(
            ' '
        ))),
        "the first recorded problem is the one reported"
    );
}

#[test]
fn part2_duplicate_header_names_are_rejected() {
    let mut builder = RequestBuilder::new("GET", "https://x.test/");
    builder.header("Accept", "a").header("accept", "b");
    assert_eq!(
        builder.build(),
        Err(RequestError::DuplicateHeader("accept".to_string())),
        "duplicates are detected on the normalized name"
    );
}

#[test]
fn part2_zero_timeout_is_rejected() {
    let mut builder = RequestBuilder::new("GET", "https://x.test/");
    builder.timeout(Duration::ZERO);
    assert_eq!(builder.build(), Err(RequestError::InvalidTimeout));
}

#[test]
fn part2_query_pairs_keep_order_and_repeats_and_url_excludes_them() {
    let req = RequestBuilder::new("GET", "https://x.test/s")
        .query("a", "1")
        .query("b", "2")
        .query("a", "3")
        .build()
        .expect("valid request");
    let expected = vec![
        ("a".to_string(), "1".to_string()),
        ("b".to_string(), "2".to_string()),
        ("a".to_string(), "3".to_string()),
    ];
    assert_eq!(req.query().to_vec(), expected);
    assert_eq!(req.url(), "https://x.test/s", "url excludes query pairs");
}

#[test]
fn part2_header_lookup_is_case_insensitive_and_keeps_insertion_order() {
    let req = RequestBuilder::new("GET", "https://x.test/")
        .header("X-Req-Id", "7")
        .header("Accept", "*/*")
        .build()
        .expect("valid request");
    assert_eq!(req.header("x-req-id"), Some("7"));
    assert_eq!(req.header("ACCEPT"), Some("*/*"));
    assert_eq!(req.header("missing"), None);
    let names: Vec<&str> = req.headers().iter().map(|h| h.name()).collect();
    assert_eq!(names, ["x-req-id", "accept"]);
}
