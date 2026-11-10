//! Link checking on top of [`parallel_map`](crate::parallel::parallel_map).

use std::collections::{HashMap, HashSet};
use std::fmt::Display;

use crate::parallel::parallel_map;

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
        matches!(self, Outcome::Status(200..=399))
    }
}

/// Checks every URL in `urls`, fetching each *distinct* URL exactly once
/// across at most `workers` threads.
///
/// The returned vector mirrors `urls` (same length, same order); duplicates
/// share the single fetch result.
pub fn check_links<F, E>(urls: &[String], workers: usize, fetch: F) -> Vec<LinkResult>
where
    F: Fn(&str) -> Result<u16, E> + Sync,
    E: Display,
{
    let mut seen = HashSet::new();
    let distinct: Vec<&str> = urls
        .iter()
        .map(String::as_str)
        .filter(|u| seen.insert(*u))
        .collect();

    let outcomes: HashMap<&str, Outcome> = parallel_map(distinct.as_slice(), workers, |url| {
        let outcome = match fetch(url) {
            Ok(status) => Outcome::Status(status),
            Err(err) => Outcome::Error(err.to_string()),
        };
        (*url, outcome)
    })
    .into_iter()
    .collect();

    urls.iter()
        .map(|url| LinkResult {
            url: url.clone(),
            outcome: outcomes[url.as_str()].clone(),
        })
        .collect()
}
