use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use ex11_ownership_and_lifetimes::store::{
    count_words_channel, count_words_shared, word_counts, Doc, Store,
};

fn doc(title: &str, lines: &[&str]) -> Doc {
    Doc {
        title: title.to_string(),
        lines: lines.iter().map(|l| l.to_string()).collect(),
    }
}

fn map(pairs: &[(&str, usize)]) -> BTreeMap<String, usize> {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

#[test]
fn part6_snapshots_share_storage() {
    let store = Store::new(doc("t", &["one"]));
    let s1 = store.snapshot();
    let s2 = store.snapshot();
    assert!(Rc::ptr_eq(&s1, &s2), "snapshot must be a shared handle, not a copy");
}

#[test]
fn part6_edit_without_snapshots_mutates_in_place() {
    let mut store = Store::new(doc("t", &["one"]));
    let before = std::ptr::from_ref(store.doc());
    store.edit(|d| d.lines.push("two".into()));
    assert_eq!(before, std::ptr::from_ref(store.doc()), "unique owner: make_mut must not clone");
    assert_eq!(store.doc().lines.len(), 2);
}

#[test]
fn part6_edit_with_live_snapshot_copies_on_write() {
    let mut store = Store::new(doc("t", &["one"]));
    let snap = store.snapshot();
    let before = std::ptr::from_ref(store.doc());
    store.edit(|d| d.lines.push("two".into()));
    assert_ne!(before, std::ptr::from_ref(store.doc()), "shared document must be copied");
    assert_eq!(snap.lines, vec!["one".to_string()], "snapshot is immutable history");
    assert_eq!(store.doc().lines.len(), 2);
}

#[test]
fn part6_edit_after_snapshot_dropped_is_in_place_again() {
    let mut store = Store::new(doc("t", &["one"]));
    let snap = store.snapshot();
    store.edit(|d| d.title.push('!'));
    drop(snap);
    let before = std::ptr::from_ref(store.doc());
    store.edit(|d| d.title.push('?'));
    assert_eq!(before, std::ptr::from_ref(store.doc()));
    assert_eq!(store.doc().title, "t!?");
}

#[test]
fn part6_clone_shares_until_edited() {
    let mut a = Store::new(doc("t", &["one"]));
    let mut b = a.clone();
    assert!(Rc::ptr_eq(&a.snapshot(), &b.snapshot()), "cloning the store is O(1)");
    b.edit(|d| d.lines.push("only in b".into()));
    assert_eq!(a.doc().lines.len(), 1, "editing a clone must not touch the original");
    assert_eq!(b.doc().lines.len(), 2);
    a.edit(|d| d.title = "a".into());
    assert_eq!(b.doc().title, "t");
}

#[test]
fn part6_revision_counts_edits_and_edit_returns_closure_value() {
    let mut store = Store::new(doc("t", &[]));
    assert_eq!(store.revision(), 0);
    let len = store.edit(|d| {
        d.lines.push("x".into());
        d.lines.len()
    });
    assert_eq!(len, 1);
    assert_eq!(store.revision(), 1);
}

#[test]
fn part6_word_counts_fold_case_and_punctuation_across_title_and_lines() {
    let d = doc("Rust, RUST!", &["borrow-checker's rules", "rust rules"]);
    assert_eq!(
        word_counts(&d),
        map(&[("rust", 3), ("borrow", 1), ("checker", 1), ("s", 1), ("rules", 2)])
    );
}

#[test]
fn part6_word_counts_of_empty_doc_is_empty() {
    assert!(word_counts(&Doc::default()).is_empty());
}

#[test]
fn part6_shared_snapshot_is_a_deep_copy() {
    let store = Store::new(doc("t", &["one"]));
    let shared = store.shared_snapshot();
    assert_eq!(*shared, *store.doc());
    assert!(
        !std::ptr::eq(&*shared, store.doc()),
        "a thread-safe snapshot must not alias the Rc-backed document"
    );
}

#[test]
fn part6_shared_and_channel_counts_agree_across_threads() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Arc<Doc>>();

    let docs: Vec<Arc<Doc>> = vec![
        Arc::new(doc("Alpha", &["beta beta", "gamma"])),
        Arc::new(doc("beta", &["Gamma, alpha!"])),
        Arc::new(doc("", &[])),
    ];
    let expected = map(&[("alpha", 2), ("beta", 3), ("gamma", 2)]);
    assert_eq!(count_words_shared(&docs), expected, "Arc<Mutex<_>> variant");
    assert_eq!(count_words_channel(&docs), expected, "channel variant");
}

#[test]
fn part6_parallel_counts_handle_empty_input() {
    assert!(count_words_shared(&[]).is_empty());
    assert!(count_words_channel(&[]).is_empty());
}
