//! Part 6: PATCH semantics. Absent vs `null` vs a value, `Default` plus struct
//! update syntax, and serde integration. A rejected patch changes nothing.

use ex02_function_parameters::{Patch, PatchError, User, UserPatch};
use serde_json::json;

/// A type with no `Default` impl: `Patch<T>::default()` must still work.
#[derive(Debug, PartialEq)]
struct NoDefault;

fn user() -> User {
    User {
        email: "ada@example.com".to_string(),
        nickname: Some("ada".to_string()),
        bio: None,
    }
}

fn parse(text: &str) -> UserPatch {
    serde_json::from_str(text).expect("valid patch JSON")
}

#[test]
fn part6_default_is_missing_without_a_t_default_bound() {
    assert!(
        Patch::<NoDefault>::default().is_missing(),
        "Default must be Missing for any T"
    );
    assert_eq!(
        UserPatch::default(),
        UserPatch {
            email: Patch::Missing,
            nickname: Patch::Missing,
            bio: Patch::Missing,
        }
    );
}

#[test]
fn part6_absent_keys_are_missing_and_null_differs_from_a_value() {
    assert_eq!(parse("{}"), UserPatch::default());
    assert_eq!(parse(r#"{"nickname": null}"#).nickname, Patch::Null);
    assert_eq!(
        parse(r#"{"bio": "hi"}"#).bio,
        Patch::Value("hi".to_string())
    );
}

#[test]
fn part6_unknown_keys_are_rejected() {
    let known: UserPatch = serde_json::from_str(r#"{"email": "a@x.io"}"#)
        .expect("a body with a known key must deserialize");
    assert_eq!(known.email, Patch::Value("a@x.io".to_string()));
    assert!(
        serde_json::from_str::<UserPatch>(r#"{"age": 3}"#).is_err(),
        "deny_unknown_fields must reject unknown keys"
    );
}

#[test]
fn part6_map_transforms_present_values_only() {
    assert_eq!(Patch::Value(2).map(|x| x * 3), Patch::Value(6));
    assert_eq!(Patch::<i32>::Null.map(|x| x * 3), Patch::Null);
    assert_eq!(Patch::<i32>::Missing.map(|x| x * 3), Patch::Missing);
}

#[test]
fn part6_apply_missing_leaves_the_slot_alone() {
    let mut slot = Some(1);
    Patch::Missing.apply(&mut slot);
    assert_eq!(slot, Some(1));
}

#[test]
fn part6_apply_null_clears_and_value_sets_the_slot() {
    let mut slot: Option<i32> = Some(1);
    Patch::Null.apply(&mut slot);
    assert_eq!(slot, None, "null clears");
    Patch::Value(9).apply(&mut slot);
    assert_eq!(slot, Some(9), "a value fills an empty slot");
}

#[test]
fn part6_email_is_trimmed_and_lowercased() {
    let mut u = user();
    parse(r#"{"email": "  Ada@Example.COM "}"#)
        .apply(&mut u)
        .expect("valid email");
    assert_eq!(u.email, "ada@example.com");
}

#[test]
fn part6_email_cannot_be_cleared() {
    let mut u = user();
    assert_eq!(
        parse(r#"{"email": null}"#).apply(&mut u),
        Err(PatchError::EmailCleared)
    );
    assert_eq!(u, user(), "a rejected patch must not change the user");
}

#[test]
fn part6_invalid_email_reports_the_raw_input() {
    for raw in ["no-at-sign", "a@b", "a b@x.io", "@x.io", "a@x.io@y.io"] {
        let patch: UserPatch =
            serde_json::from_value(json!({ "email": raw })).expect("valid shape");
        assert_eq!(
            patch.apply(&mut user()),
            Err(PatchError::InvalidEmail(raw.to_string())),
            "email {raw:?} should be rejected"
        );
    }
}

#[test]
fn part6_nickname_is_trimmed_and_length_counts_chars() {
    let mut u = user();
    parse(r#"{"nickname": "  Grace  "}"#)
        .apply(&mut u)
        .expect("valid nickname");
    assert_eq!(u.nickname.as_deref(), Some("Grace"));

    let multibyte = "é".repeat(32);
    serde_json::from_value::<UserPatch>(json!({ "nickname": multibyte }))
        .expect("valid shape")
        .apply(&mut u)
        .expect("32 characters is within the limit");
    assert_eq!(u.nickname.as_deref(), Some(multibyte.as_str()));
}

#[test]
fn part6_blank_or_overlong_nickname_is_rejected() {
    let mut u = user();
    for bad in ["   ".to_string(), "x".repeat(33)] {
        let patch: UserPatch =
            serde_json::from_value(json!({ "nickname": bad })).expect("valid shape");
        assert_eq!(patch.apply(&mut u), Err(PatchError::InvalidNickname));
    }
    assert_eq!(u, user());
}

#[test]
fn part6_bio_limit_counts_chars_and_null_clears() {
    let mut u = user();
    serde_json::from_value::<UserPatch>(json!({ "bio": "é".repeat(280) }))
        .expect("valid shape")
        .apply(&mut u)
        .expect("280 characters is within the limit");
    assert_eq!(u.bio.as_ref().map(|b| b.chars().count()), Some(280));

    let too_long: UserPatch =
        serde_json::from_value(json!({ "bio": "x".repeat(281) })).expect("valid shape");
    assert_eq!(too_long.apply(&mut u), Err(PatchError::BioTooLong));

    parse(r#"{"bio": null}"#)
        .apply(&mut u)
        .expect("null bio is allowed");
    assert_eq!(u.bio, None);
}

#[test]
fn part6_rejected_patch_changes_nothing() {
    let mut u = user();
    let patch = parse(r#"{"email": "new@x.io", "nickname": "   "}"#);
    assert_eq!(patch.apply(&mut u), Err(PatchError::InvalidNickname));
    assert_eq!(
        u,
        user(),
        "validation happens before mutation, so the valid email is not applied"
    );
}

#[test]
fn part6_struct_update_syntax_sets_one_field() {
    let patch = UserPatch {
        bio: Patch::Value("hello".to_string()),
        ..Default::default()
    };
    let mut u = user();
    patch.apply(&mut u).expect("valid patch");
    assert_eq!(u.bio.as_deref(), Some("hello"));
    assert_eq!(u.email, "ada@example.com", "untouched field");
    assert_eq!(u.nickname.as_deref(), Some("ada"), "untouched field");
}
