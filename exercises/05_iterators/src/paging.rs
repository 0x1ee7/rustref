//! Part 3 — a lazily paginated API client.
//!
//! The client is handed a `fetch` closure taking the cursor (`None` = first page). It
//! never talks to the network itself. Pages are fetched only when the consumer pulls.

use std::fmt;
use std::marker::PhantomData;

/// One page of results. `next_cursor == None` marks the last page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

/// The server handed back a cursor that was already requested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaginationLoopError {
    pub cursor: String,
}

impl fmt::Display for PaginationLoopError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl std::error::Error for PaginationLoopError {}

/// Failure while paginating: either the fetch closure failed or the cursor looped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaginateError<E> {
    Fetch(E),
    Loop(PaginationLoopError),
}

impl<E: fmt::Display> fmt::Display for PaginateError<E> {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl<E: std::error::Error + 'static> std::error::Error for PaginateError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        todo!()
    }
}

/// Iterator over whole pages. Returned by [`pages`].
///
/// After an error (fetch or loop) it yields nothing more.
pub struct Pages<T, E, F> {
    _fetch: F,
    _marker: PhantomData<fn() -> Result<Page<T>, E>>,
}

/// Yield each page, fetching the next one only when asked.
///
/// `start` is the cursor for the first request (`None` = the server's first page).
pub fn pages<T, E, F>(_fetch: F, _start: Option<String>) -> Pages<T, E, F>
where
    F: FnMut(Option<&str>) -> Result<Page<T>, E>,
{
    todo!()
}

impl<T, E, F> Iterator for Pages<T, E, F>
where
    F: FnMut(Option<&str>) -> Result<Page<T>, E>,
{
    type Item = Result<Page<T>, PaginateError<E>>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

/// Iterator over individual items across pages. Returned by [`items`].
pub struct Items<T, E, F> {
    _pages: Pages<T, E, F>,
    _current: std::vec::IntoIter<T>,
}

/// Yield items across all pages. Calling this performs no fetch.
pub fn items<T, E, F>(_fetch: F, _start: Option<String>) -> Items<T, E, F>
where
    F: FnMut(Option<&str>) -> Result<Page<T>, E>,
{
    todo!()
}

impl<T, E, F> Iterator for Items<T, E, F>
where
    F: FnMut(Option<&str>) -> Result<Page<T>, E>,
{
    type Item = Result<T, PaginateError<E>>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
