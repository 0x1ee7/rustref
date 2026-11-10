//! Part 3 — lazily paginated API client over an injected fetch closure.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use ex05_iterators::paging::{Page, PaginateError, PaginationLoopError, items, pages};

type Log = Rc<RefCell<Vec<Option<String>>>>;

/// Fake server: page `i` is addressed by cursor `"i"`; records every requested cursor.
fn server(data: Vec<Vec<u32>>) -> (impl FnMut(Option<&str>) -> Result<Page<u32>, String>, Log) {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let handle = log.clone();
    let fetch = move |cursor: Option<&str>| {
        handle.borrow_mut().push(cursor.map(str::to_owned));
        let idx = cursor.map_or(0, |c| c.parse::<usize>().expect("cursor is a page index"));
        let items = data.get(idx).cloned().ok_or_else(|| "no such page".to_string())?;
        let next_cursor = (idx + 1 < data.len()).then(|| (idx + 1).to_string());
        Ok(Page { items, next_cursor })
    };
    (fetch, log)
}

#[test]
fn part3_no_fetch_happens_until_the_first_pull() {
    let (fetch, log) = server(vec![vec![1], vec![2]]);
    let mut it = pages(fetch, None);
    assert!(log.borrow().is_empty(), "creating the iterator must not call fetch");
    it.next().expect("first page").expect("fetch succeeds");
    assert_eq!(log.borrow().len(), 1, "the first pull must fetch exactly one page");
}

#[test]
fn part3_pages_yields_every_page_including_the_last() {
    let (fetch, log) = server(vec![vec![1, 2], vec![3], vec![4]]);
    let got: Vec<Page<u32>> = pages(fetch, None).map(|p| p.expect("no failures")).collect();
    assert_eq!(got.len(), 3, "all three pages, including the last one, must be yielded");
    assert_eq!(got[2].next_cursor, None, "the last page carries no cursor");
    assert_eq!(
        *log.borrow(),
        vec![None, Some("1".to_string()), Some("2".to_string())],
        "cursors are requested in server order, starting with None"
    );
}

#[test]
fn part3_items_flatten_across_pages_and_allow_empty_middle_pages() {
    let (fetch, _log) = server(vec![vec![1], vec![], vec![2, 3]]);
    let got: Vec<u32> = items(fetch, None).map(|r| r.expect("no failures")).collect();
    assert_eq!(got, vec![1, 2, 3], "items must flatten across pages, skipping empty ones");
}

#[test]
fn part3_start_cursor_is_the_first_request() {
    let (fetch, log) = server(vec![vec![1], vec![2]]);
    let got: Vec<u32> = items(fetch, Some("1".to_string())).map(|r| r.expect("no failures")).collect();
    assert_eq!(got, vec![2], "starting at cursor \"1\" must skip page 0");
    assert_eq!(*log.borrow(), vec![Some("1".to_string())], "the start cursor is the first request");
}

#[test]
fn part3_exhausted_client_makes_no_further_requests() {
    let (fetch, log) = server(vec![vec![1]]);
    let mut it = items(fetch, None);
    assert_eq!(it.next().unwrap().unwrap(), 1, "one item on the only page");
    assert!(it.next().is_none(), "no more items after the last page");
    let calls = log.borrow().len();
    assert!(it.next().is_none(), "a finished client keeps returning None");
    assert_eq!(log.borrow().len(), calls, "a finished client must not fetch again");
}

#[test]
fn part3_cursor_loop_is_detected_before_requesting_it_again() {
    let calls = Cell::new(0);
    let fetch = |_cursor: Option<&str>| -> Result<Page<u32>, String> {
        calls.set(calls.get() + 1);
        Ok(Page { items: vec![], next_cursor: Some("a".to_string()) })
    };
    let results: Vec<_> = pages(fetch, None).collect();
    assert_eq!(calls.get(), 2, "the repeated cursor \"a\" must not be requested a second time");
    assert_eq!(results.len(), 3, "two pages, then the loop error");
    assert_eq!(
        results[2].as_ref().unwrap_err(),
        &PaginateError::Loop(PaginationLoopError { cursor: "a".to_string() }),
        "the loop error names the repeated cursor"
    );
}

#[test]
fn part3_fetch_failure_propagates_then_the_client_is_finished() {
    let fetch = |cursor: Option<&str>| -> Result<Page<u32>, String> {
        match cursor {
            None => Ok(Page { items: vec![7], next_cursor: Some("1".to_string()) }),
            Some(_) => Err("boom".to_string()),
        }
    };
    let mut it = items(fetch, None);
    assert_eq!(it.next().unwrap().unwrap(), 7, "items from the good page come first");
    match it.next() {
        Some(Err(PaginateError::Fetch(msg))) => assert_eq!(msg, "boom", "the fetch error must reach the consumer"),
        other => panic!("expected the fetch error, got {other:?}"),
    }
    assert!(it.next().is_none(), "after a failure the generator is finished");
}
