use ex11_ownership_and_lifetimes::interner::{Interner, StrInterner};

fn within(haystack: &str, needle: &str) -> bool {
    let start = haystack.as_ptr() as usize;
    let p = needle.as_ptr() as usize;
    p >= start && p + needle.len() <= start + haystack.len()
}

#[test]
fn part5_intern_returns_same_symbol_for_equal_text() {
    let mut i = Interner::new();
    let a = i.intern("payments");
    let b = i.intern(&String::from("payments"));
    assert_eq!(a, b);
    assert_ne!(a, i.intern("ledger"));
}

#[test]
fn part5_symbols_are_dense_in_first_seen_order() {
    let mut i = Interner::new();
    let indices: Vec<usize> = ["a", "b", "a", "c"].iter().map(|t| i.intern(t).index()).collect();
    assert_eq!(indices, vec![0, 1, 0, 2]);
    assert_eq!(i.len(), 3);
    assert!(!i.is_empty());
}

#[test]
fn part5_resolve_round_trips_and_rejects_foreign_symbols() {
    let mut small = Interner::new();
    let sym = small.intern("x");
    assert_eq!(small.resolve(sym), Some("x"));

    let mut big = Interner::new();
    for n in 0..5 {
        big.intern(&n.to_string());
    }
    let foreign = big.intern("4");
    assert_eq!(small.resolve(foreign), None, "index 4 does not exist in `small`");
}

#[test]
fn part5_get_never_inserts() {
    let mut i = Interner::new();
    assert_eq!(i.get("ghost"), None);
    assert!(i.is_empty());
    let sym = i.intern("ghost");
    assert_eq!(i.get("ghost"), Some(sym));
}

#[test]
fn part5_get_counts_misses_through_shared_reference() {
    let mut i = Interner::new();
    i.intern("a");
    let shared: &Interner = &i;
    assert!(shared.get("a").is_some());
    assert!(shared.get("z").is_none());
    assert!(shared.get("y").is_none());
    assert_eq!(i.misses(), 2, "only failed lookups are counted");
    assert!(Interner::new().is_empty());
}

#[test]
fn part5_str_interner_resolve_outlives_the_interner() {
    let text = String::from("alpha beta");
    let resolved: &str;
    {
        let mut i = StrInterner::new();
        let sym = i.intern(&text[..5]);
        resolved = i.resolve(sym).expect("symbol came from this interner");
        // `i` is dropped here; `resolved` borrows from `text`, not from `i`.
    }
    assert_eq!(resolved, "alpha");
    assert!(within(&text, resolved));
}

#[test]
fn part5_str_interner_dedups_borrowed_text_without_copying() {
    let text = String::from("alpha beta alpha");
    let mut i = StrInterner::new();
    let syms: Vec<_> = text.split(' ').map(|w| i.intern(w)).collect();
    assert_eq!(syms[0], syms[2]);
    assert_eq!(i.len(), 2);
    let back = i.resolve(syms[1]).unwrap();
    assert_eq!(back, "beta");
    assert!(within(&text, back), "resolve must return the caller's bytes");
}

#[test]
fn part5_str_interner_empty_state() {
    let i: StrInterner<'_> = StrInterner::new();
    assert!(i.is_empty());
    assert_eq!(i.len(), 0);
}
