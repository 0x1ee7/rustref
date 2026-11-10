//! Part 3 — a lazily paginated API client.
//!
//! The client is handed a `fetch` closure taking the cursor (`None` = first page). It
//! never talks to the network itself. Pages are fetched only when the consumer pulls.

use std::collections::HashSet;
use std::fmt;
use std::marker::PhantomData;

/// One page of results. `next_cursor == None` marks the last page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    /// Items on this page, in server order.
    pub items: Vec<T>,
    /// Cursor for the following page, if any.
    pub next_cursor: Option<String>,
}

/// The server handed back a cursor that was already requested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaginationLoopError {
    /// The repeated cursor.
    pub cursor: String,
}

impl fmt::Display for PaginationLoopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pagination cursor {:?} was already requested", self.cursor)
    }
}

impl std::error::Error for PaginationLoopError {}

/// Failure while paginating: either the fetch closure failed or the cursor looped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaginateError<E> {
    /// The fetch closure returned an error.
    Fetch(E),
    /// The server repeated a cursor.
    Loop(PaginationLoopError),
}

impl<E: fmt::Display> fmt::Display for PaginateError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaginateError::Fetch(e) => write!(f, "fetch failed: {e}"),
            PaginateError::Loop(e) => write!(f, "{e}"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for PaginateError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PaginateError::Fetch(e) => Some(e),
            PaginateError::Loop(e) => Some(e),
        }
    }
}

/// Iterator over whole pages. Returned by [`pages`].
///
/// After an error (fetch or loop) it yields nothing more.
pub struct Pages<T, E, F> {
    fetch: F,
    /// `Some(cursor)` = the next request to make; `None` = finished.
    pending: Option<Option<String>>,
    seen: HashSet<String>,
    _marker: PhantomData<(T, E)>,
}

/// Yield each page, fetching the next one only when asked.
///
/// `start` is the cursor for the first request (`None` = the server's first page).
pub fn pages<T, E, F>(fetch: F, start: Option<String>) -> Pages<T, E, F>
where
    F: FnMut(Option<&str>) -> Result<Page<T>, E>,
{
    Pages {
        fetch,
        pending: Some(start),
        seen: HashSet::new(),
        _marker: PhantomData,
    }
}

impl<T, E, F> Iterator for Pages<T, E, F>
where
    F: FnMut(Option<&str>) -> Result<Page<T>, E>,
{
    type Item = Result<Page<T>, PaginateError<E>>;

    fn next(&mut self) -> Option<Self::Item> {
        let cursor = self.pending.take()?;
        if let Some(c) = &cursor
            && !self.seen.insert(c.clone())
        {
            return Some(Err(PaginateError::Loop(PaginationLoopError {
                cursor: c.clone(),
            })));
        }
        match (self.fetch)(cursor.as_deref()) {
            Err(e) => Some(Err(PaginateError::Fetch(e))),
            Ok(page) => {
                self.pending = page.next_cursor.clone().map(Some);
                Some(Ok(page))
            }
        }
    }
}

/// Iterator over individual items across pages. Returned by [`items`].
pub struct Items<T, E, F> {
    pages: Pages<T, E, F>,
    current: std::vec::IntoIter<T>,
}

/// Yield items across all pages. Calling this performs no fetch.
pub fn items<T, E, F>(fetch: F, start: Option<String>) -> Items<T, E, F>
where
    F: FnMut(Option<&str>) -> Result<Page<T>, E>,
{
    Items {
        pages: pages(fetch, start),
        current: Vec::new().into_iter(),
    }
}

impl<T, E, F> Iterator for Items<T, E, F>
where
    F: FnMut(Option<&str>) -> Result<Page<T>, E>,
{
    type Item = Result<T, PaginateError<E>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(item) = self.current.next() {
                return Some(Ok(item));
            }
            match self.pages.next()? {
                Ok(page) => self.current = page.items.into_iter(),
                Err(e) => return Some(Err(e)),
            }
        }
    }
}
