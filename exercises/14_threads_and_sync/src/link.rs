//! Part 1: checking links in parallel.

use std::fmt::Display;

/// The result of checking one URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkResult {
    /// The URL exactly as it appeared in the input.
    pub url: String,
    /// What the fetch returned.
    pub outcome: Outcome,
}

/// Outcome of a single fetch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The server answered with this HTTP status code.
    Status(u16),
    /// The fetch failed; the error's `Display` text.
    Error(String),
}

impl Outcome {
    /// `true` for 2xx and 3xx statuses.
    pub fn is_ok(&self) -> bool {
        let _ = self;
        todo!()
    }
}

/// Checks every URL in `urls`, fetching each *distinct* URL exactly once
/// across at most `workers` threads.
///
/// Requirements:
/// - the result mirrors `urls` (same length, same order, `url` copied in);
/// - duplicate URLs share one fetch;
/// - an `Err` from `fetch` becomes `Outcome::Error(err.to_string())`, and
///   other URLs are unaffected.
pub fn check_links<F, E>(urls: &[String], workers: usize, fetch: F) -> Vec<LinkResult>
where
    F: Fn(&str) -> Result<u16, E> + Sync,
    E: Display,
{
    let _ = (urls, workers, fetch);
    todo!()
}
