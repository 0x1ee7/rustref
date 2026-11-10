//! Part 3: `newtype!`, a tuple-struct wrapper with opt-in trait impls.

use std::collections::HashSet;
use std::num::ParseIntError;

use ex20_macros::newtype;

newtype!(pub UserId(u64): Copy, Display, FromStr);
newtype!(pub Email(String): Display, FromStr);
newtype!(pub Token(Vec<u8>));

#[test]
fn part3_new_into_inner_and_from() {
    let id = UserId::new(42);
    assert_eq!(id.into_inner(), 42u64, "into_inner should unwrap the value");

    let from: UserId = 7u64.into();
    assert_eq!(from, UserId::new(7), "From<inner> should match new()");
}

#[test]
fn part3_derived_ordering_and_hash() {
    let mut ids = vec![UserId::new(3), UserId::new(1), UserId::new(2)];
    ids.sort();
    assert_eq!(
        ids,
        vec![UserId::new(1), UserId::new(2), UserId::new(3)],
        "derived Ord should follow the inner value"
    );

    let set: HashSet<UserId> = ids.iter().copied().collect();
    assert!(set.contains(&UserId::new(2)), "derived Hash + Eq should work");
}

#[test]
fn part3_copy_leaves_original_usable() {
    let a = UserId::new(5);
    let b = a;
    assert_eq!(a, b, "Copy means the original is still usable after a move");
    assert_eq!(a.into_inner(), 5);
}

#[test]
fn part3_display_delegates_and_honours_format_spec() {
    assert_eq!(UserId::new(42).to_string(), "42");
    assert_eq!(
        format!("{:>5}", UserId::new(42)),
        "   42",
        "Display should forward the formatter, width included"
    );
}

#[test]
fn part3_from_str_parses_the_inner_type() {
    assert_eq!("42".parse::<UserId>(), Ok(UserId::new(42)));
}

#[test]
fn part3_from_str_error_is_the_inner_error() {
    let err: ParseIntError = "x4".parse::<UserId>().unwrap_err();
    assert_eq!(
        err.to_string(),
        "invalid digit found in string",
        "Err should be the inner type's error"
    );
}

#[test]
fn part3_email_display_parse_and_wrap() {
    let email: Email = "ops@example.com".parse().expect("Infallible for String");
    assert_eq!(format!("{email}"), "ops@example.com");

    let wrapped = Email::from("a@b.c".to_string());
    assert_eq!(wrapped.into_inner(), "a@b.c");
}

#[test]
fn part3_token_has_plain_derives_only() {
    let token = Token::new(vec![1, 2]);
    let cloned = token.clone();
    assert_eq!(token, cloned, "Token derives Clone and PartialEq");
    assert_eq!(
        format!("{token:?}"),
        "Token([1, 2])",
        "Token derives Debug as a tuple struct"
    );
    assert_eq!(Token::new(vec![9]).into_inner(), vec![9]);
}
