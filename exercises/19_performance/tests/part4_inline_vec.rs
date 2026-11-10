//! Part 4: `InlineVec<T, N>`, a small vector that stays off the heap up to `N` elements.
mod common;

use common::count_allocations;
use ex19_performance::inline_vec::InlineVec;
use std::rc::Rc;

#[test]
fn part4_fills_inline_storage_without_allocating() {
    let mut v: InlineVec<u32, 4> = InlineVec::new();
    let ((), allocs) = count_allocations(|| {
        for i in 0..4 {
            v.push(i);
        }
    });
    assert_eq!(allocs, 0, "up to N elements must live inline");
    assert!(!v.is_spilled());
    assert_eq!(v.len(), 4);
    assert_eq!(v.iter().copied().collect::<Vec<_>>(), vec![0, 1, 2, 3]);
}

#[test]
fn part4_spills_on_the_push_past_capacity_preserving_order() {
    let mut v: InlineVec<u32, 4> = InlineVec::new();
    for i in 0..4 {
        v.push(i);
    }
    let ((), allocs) = count_allocations(|| v.push(4));
    assert!(allocs >= 1, "the push past N must move elements to the heap");
    assert!(v.is_spilled(), "len > N means spilled");
    assert_eq!(v.iter().copied().collect::<Vec<_>>(), vec![0, 1, 2, 3, 4]);

    for i in 5..9 {
        v.push(i);
    }
    assert_eq!(v.len(), 9);
    assert_eq!(v.iter().copied().collect::<Vec<_>>(), (0..9).collect::<Vec<u32>>());
}

#[test]
fn part4_zero_capacity_spills_on_first_push() {
    let mut v: InlineVec<u8, 0> = InlineVec::new();
    assert!(v.is_empty());
    assert!(!v.is_spilled(), "an empty vector is not spilled");
    v.push(7);
    assert!(v.is_spilled(), "N = 0 must spill on the first push");
    assert_eq!(v.len(), 1);
    assert_eq!(v.get(0), Some(&7));
}

#[test]
fn part4_get_is_bounds_checked_inline_and_spilled() {
    let mut inline: InlineVec<i32, 3> = InlineVec::new();
    inline.push(10);
    inline.push(20);
    assert_eq!(inline.get(0), Some(&10));
    assert_eq!(inline.get(1), Some(&20));
    assert_eq!(inline.get(2), None, "index == len is out of bounds");

    let mut spilled: InlineVec<i32, 3> = InlineVec::new();
    for x in [1, 2, 3, 4, 5] {
        spilled.push(x);
    }
    assert_eq!(spilled.get(4), Some(&5));
    assert_eq!(spilled.get(5), None);
}

#[test]
fn part4_iter_and_into_iterator_for_references_agree() {
    for len in [0_u32, 3, 9] {
        let mut v: InlineVec<u32, 4> = InlineVec::new();
        for i in 0..len {
            v.push(i * 2);
        }
        let via_iter: Vec<&u32> = v.iter().collect();
        let mut via_for: Vec<&u32> = Vec::new();
        for x in &v {
            via_for.push(x);
        }
        assert_eq!(via_iter, via_for, "len {len}");
        assert_eq!(via_iter.len(), len as usize);
    }
}

#[test]
fn part4_iterating_inline_storage_allocates_nothing() {
    let mut v: InlineVec<u32, 4> = InlineVec::new();
    for i in 0..4 {
        v.push(i);
    }
    let (sum, allocs) = count_allocations(|| v.iter().sum::<u32>());
    assert_eq!(sum, 6);
    assert_eq!(allocs, 0, "iterating inline storage must not allocate");
}

#[test]
fn part4_drops_each_inline_element_exactly_once() {
    let marker = Rc::new(());
    {
        let mut v: InlineVec<Rc<()>, 4> = InlineVec::new();
        for _ in 0..3 {
            v.push(Rc::clone(&marker));
        }
        assert_eq!(Rc::strong_count(&marker), 4, "three elements plus the marker");
    }
    assert_eq!(
        Rc::strong_count(&marker),
        1,
        "inline elements must be dropped exactly once"
    );
}

#[test]
fn part4_drops_each_spilled_element_exactly_once() {
    let marker = Rc::new(());
    {
        let mut v: InlineVec<Rc<()>, 4> = InlineVec::new();
        for _ in 0..10 {
            v.push(Rc::clone(&marker));
        }
        assert!(v.is_spilled());
        assert_eq!(Rc::strong_count(&marker), 11, "spilling must not duplicate elements");
    }
    assert_eq!(
        Rc::strong_count(&marker),
        1,
        "spilled elements must be dropped exactly once"
    );
}

#[test]
fn part4_moves_non_copy_values_across_the_spill_boundary() {
    let mut v: InlineVec<String, 2> = InlineVec::new();
    for word in ["alpha", "beta", "gamma"] {
        v.push(word.to_owned());
    }
    let words: Vec<&str> = v.iter().map(String::as_str).collect();
    assert_eq!(words, ["alpha", "beta", "gamma"]);
    assert_eq!(v.get(2).map(String::as_str), Some("gamma"));
}

#[test]
fn part4_matches_a_vec_model_for_every_length() {
    for len in 0..=40_usize {
        let mut v: InlineVec<usize, 8> = InlineVec::new();
        let mut model = Vec::new();
        for i in 0..len {
            v.push(i * 3);
            model.push(i * 3);
        }
        let collected: Vec<usize> = v.iter().copied().collect();
        assert_eq!(collected, model, "contents for len {len}");
        assert_eq!(v.len(), len, "len for len {len}");
        assert_eq!(v.is_spilled(), len > 8, "spill threshold for len {len}");
        assert_eq!(v.get(len), None, "out of bounds for len {len}");
    }
}
