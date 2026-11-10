//! Part 2: a bounded downloader built from `Mutex` + `Condvar`.

use std::marker::PhantomData;

/// Runs a blocking `fetch` function with at most `max_concurrent` calls in flight.
///
/// Requirements:
/// - `new` panics with a message mentioning `max_concurrent` when it is zero;
/// - `download` blocks until a slot is free and releases it even if `fetch` panics;
/// - `download_all` returns bodies in input order; every URL is attempted even
///   after a failure, and the error returned is the one for the earliest failing
///   input index;
/// - `download_all(&[])` returns `Ok(vec![])`;
/// - `in_flight` and `peak_concurrency` report live and historical concurrency.
pub struct BoundedDownloader<F> {
    _fetch: PhantomData<F>,
}

impl<F, E> BoundedDownloader<F>
where
    F: Fn(&str) -> Result<Vec<u8>, E> + Sync,
    E: Send,
{
    /// Creates a downloader allowing at most `max_concurrent` simultaneous fetches.
    pub fn new(fetch: F, max_concurrent: usize) -> Self {
        let _ = (fetch, max_concurrent);
        todo!()
    }

    /// Fetches one URL, waiting for a free slot first.
    pub fn download(&self, url: &str) -> Result<Vec<u8>, E> {
        let _ = url;
        todo!()
    }

    /// Fetches every URL and returns the bodies in input order.
    pub fn download_all(&self, urls: &[String]) -> Result<Vec<Vec<u8>>, E> {
        let _ = urls;
        todo!()
    }

    /// Number of fetches running right now.
    pub fn in_flight(&self) -> usize {
        todo!()
    }

    /// Highest number of simultaneous fetches observed so far.
    pub fn peak_concurrency(&self) -> usize {
        todo!()
    }
}
