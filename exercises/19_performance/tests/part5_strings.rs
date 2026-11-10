//! Part 5: borrow-first strings: `Cow`, borrowed map keys, `Box<str>`, `Arc<str>` interning.
mod common;

use common::count_allocations;
use ex19_performance::strings::{HeaderMap, Interner, Tag, normalize_header, sanitize_label};
use std::borrow::Cow;
use std::sync::Arc;

#[test]
fn part5_normalize_header_borrows_when_already_lowercase() {
    let (name, allocs) = count_allocations(|| normalize_header("content-type"));
    assert!(
        matches!(name, Cow::Borrowed("content-type")),
        "lowercase input must be borrowed"
    );
    assert_eq!(allocs, 0);
}

#[test]
fn part5_normalize_header_owns_only_when_it_changes() {
    let name = normalize_header("Content-Type");
    assert!(matches!(name, Cow::Owned(_)), "a changed name must be owned");
    assert_eq!(name, "content-type");
}

#[test]
fn part5_normalize_header_edge_cases() {
    assert!(matches!(normalize_header(""), Cow::Borrowed("")));
    assert_eq!(normalize_header("X-REQUEST-ID"), "x-request-id");
    assert_eq!(
        normalize_header("ÄBC"),
        "Äbc",
        "only ASCII letters are folded; non-ASCII is untouched"
    );
}

#[test]
fn part5_sanitize_label_borrows_unless_spaces_present() {
    let (plain, allocs) = count_allocations(|| sanitize_label("eu-west-1"));
    assert!(matches!(plain, Cow::Borrowed("eu-west-1")));
    assert_eq!(allocs, 0);

    let spaced = sanitize_label("build step 42");
    assert!(matches!(spaced, Cow::Owned(_)));
    assert_eq!(spaced, "build_step_42");
}

#[test]
fn part5_header_map_lookup_ignores_case() {
    let mut map = HeaderMap::new();
    assert_eq!(map.insert("Content-Type", "text/plain"), None);
    assert_eq!(map.get("content-type"), Some("text/plain"));
    assert_eq!(map.get("CONTENT-TYPE"), Some("text/plain"));
    assert_eq!(map.get("Content-Type"), Some("text/plain"));
    assert_eq!(map.get("accept"), None);
}

#[test]
fn part5_header_map_insert_returns_previous_and_folds_case() {
    let mut map = HeaderMap::new();
    assert!(map.is_empty());
    assert_eq!(map.insert("X-Id", "1"), None);
    assert_eq!(
        map.insert("x-id", "2"),
        Some("1".to_owned()),
        "names differing only by case are the same key"
    );
    assert_eq!(map.len(), 1);
    assert_eq!(map.get("X-ID"), Some("2"));
}

#[test]
fn part5_header_map_lowercase_lookup_allocates_nothing() {
    let mut map = HeaderMap::new();
    map.insert("content-type", "application/json");
    let (value, allocs) = count_allocations(|| map.get("content-type"));
    assert_eq!(value, Some("application/json"));
    assert_eq!(allocs, 0, "a borrowed &str key must not clone or allocate");
}

#[test]
fn part5_interner_shares_one_allocation_per_distinct_string() {
    let mut interner = Interner::new();
    assert!(interner.is_empty());
    let first = interner.intern("eu-west-1");
    let (second, allocs) = count_allocations(|| interner.intern("eu-west-1"));
    assert!(Arc::ptr_eq(&first, &second), "same string must share storage");
    assert_eq!(allocs, 0, "re-interning must not allocate");
    assert_eq!(
        Arc::strong_count(&first),
        3,
        "the interner plus the two handles returned"
    );
    assert_eq!(interner.len(), 1);
}

#[test]
fn part5_interner_keeps_distinct_strings_apart() {
    let mut interner = Interner::new();
    let a = interner.intern("a");
    let b = interner.intern("b");
    assert!(!Arc::ptr_eq(&a, &b));
    assert_eq!(&*a, "a");
    assert_eq!(&*b, "b");
    assert_eq!(interner.len(), 2);
}

#[test]
fn part5_tag_is_one_allocation_for_a_label() {
    let (tag, allocs) = count_allocations(|| Tag::new("region-eu-west-1"));
    assert_eq!(allocs, 1, "a non-empty label is one boxed-str allocation");
    assert_eq!(tag.as_str(), "region-eu-west-1");
}

#[test]
fn part5_tag_empty_label_allocates_nothing() {
    let (tag, allocs) = count_allocations(|| Tag::new(""));
    assert_eq!(tag.as_str(), "");
    assert_eq!(allocs, 0, "an empty Box<str> must not allocate");
}
