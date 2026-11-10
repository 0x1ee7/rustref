//! Part 4 — resource-owning iterators and batched writes.

use std::cell::{Cell, RefCell};
use std::io::{self, BufRead, Cursor, Read};
use std::rc::Rc;

use ex05_iterators::batch::{read_batches, write_in_batches};

/// Reader that flags its own drop, so tests can observe when the resource is released.
struct Tracked {
    inner: Cursor<Vec<u8>>,
    dropped: Rc<Cell<bool>>,
}

impl Read for Tracked {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl BufRead for Tracked {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt)
    }
}

impl Drop for Tracked {
    fn drop(&mut self) {
        self.dropped.set(true);
    }
}

fn tracked(text: &str, dropped: &Rc<Cell<bool>>) -> Tracked {
    Tracked { inner: Cursor::new(text.as_bytes().to_vec()), dropped: dropped.clone() }
}

#[test]
fn part4_read_batches_yields_raw_lines_in_groups() {
    let batches: Vec<Vec<String>> = read_batches(|| Ok(Cursor::new("a\nb\nc\nd\ne")), 2)
        .map(|b| b.expect("in-memory read"))
        .collect();
    assert_eq!(
        batches,
        vec![
            vec!["a\n".to_string(), "b\n".to_string()],
            vec!["c\n".to_string(), "d\n".to_string()],
            vec!["e".to_string()],
        ],
        "lines keep their newlines; the final batch may be short"
    );
}

#[test]
fn part4_read_batches_opens_on_first_pull_not_on_creation() {
    let opened = Cell::new(false);
    let mut it = read_batches(
        || {
            opened.set(true);
            Ok(Cursor::new("x\n"))
        },
        1,
    );
    assert!(!opened.get(), "constructing the iterator must not open the source");
    it.next().expect("one batch").expect("in-memory read");
    assert!(opened.get(), "the first pull opens the source");
}

#[test]
fn part4_read_batches_releases_resource_when_exhausted() {
    let dropped = Rc::new(Cell::new(false));
    let src = tracked("a\nb\nc\n", &dropped);
    let mut it = read_batches(move || Ok(src), 2);
    while it.next().is_some() {}
    assert!(dropped.get(), "the reader must be released as soon as the source is exhausted");
}

#[test]
fn part4_read_batches_releases_resource_on_early_drop() {
    let dropped = Rc::new(Cell::new(false));
    let src = tracked("a\nb\nc\nd\n", &dropped);
    let mut it = read_batches(move || Ok(src), 1);
    assert_eq!(it.next().unwrap().unwrap(), vec!["a\n".to_string()], "first batch");
    assert!(!dropped.get(), "the resource stays open while the iterator is alive");
    drop(it);
    assert!(dropped.get(), "dropping the iterator early must release the resource");
}

#[test]
fn part4_read_batches_reports_open_failure_once() {
    let mut it = read_batches(|| Err::<Cursor<&str>, _>(io::Error::other("no such file")), 3);
    match it.next() {
        Some(Err(e)) => assert_eq!(e.to_string(), "no such file", "open errors must be passed through"),
        other => panic!("expected the open error, got {other:?}"),
    }
    assert!(it.next().is_none(), "after the error the iterator is finished");
}

#[test]
#[should_panic(expected = "must be at least 1")]
fn part4_read_batches_zero_size_panics() {
    let _ = read_batches(|| Ok(Cursor::new("a\n")), 0);
}

#[test]
fn part4_write_in_batches_sums_what_write_reports() {
    let seen: RefCell<Vec<Vec<i32>>> = RefCell::new(Vec::new());
    let total = write_in_batches(1..=5, 2, |batch: Vec<i32>| {
        let n = batch.len();
        seen.borrow_mut().push(batch);
        Ok::<usize, String>(n)
    });
    assert_eq!(total, Ok(5), "the sum of per-batch counts must be returned");
    assert_eq!(
        *seen.borrow(),
        vec![vec![1, 2], vec![3, 4], vec![5]],
        "batches are passed by value, in order, with a short tail"
    );
}

#[test]
fn part4_write_in_batches_pulls_only_what_the_failing_batch_needs() {
    let pulls = Cell::new(0);
    let rows = (0..10).inspect(|_| pulls.set(pulls.get() + 1));
    let mut calls = 0;
    let result = write_in_batches(rows, 2, |_batch: Vec<i32>| {
        calls += 1;
        if calls == 2 { Err("disk full") } else { Ok(2) }
    });
    assert_eq!(result, Err("disk full"), "the first write error is returned");
    assert_eq!(pulls.get(), 4, "only the rows for the two batches written so far may be pulled");
}

#[test]
fn part4_write_in_batches_empty_input_never_writes() {
    let mut calls = 0;
    let result = write_in_batches(Vec::<i32>::new(), 3, |_batch| {
        calls += 1;
        Ok::<usize, ()>(0)
    });
    assert_eq!(result, Ok(0), "no rows means a zero total");
    assert_eq!(calls, 0, "no rows means no write calls");
}

#[test]
#[should_panic(expected = "must be at least 1")]
fn part4_write_in_batches_zero_size_panics() {
    let _ = write_in_batches(vec![1], 0, |_b: Vec<i32>| Ok::<usize, ()>(0));
}
