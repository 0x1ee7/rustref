//! Minimal POSIX-style glob matching for ignore rules.
//!
//! Supported syntax: `*` (any run of characters within one path segment),
//! `?` (one character within a segment) and `**` (as a whole segment, any number
//! of segments including zero). Matching is case-sensitive.

/// Whole-path match in the spirit of `PurePosixPath.full_match`.
///
/// The pattern must match the entire `path`, segment by segment.
#[must_use]
pub fn full_match(pattern: &str, path: &str) -> bool {
    let pat: Vec<&str> = pattern.split('/').collect();
    let segs: Vec<&str> = path.split('/').collect();
    match_segments(&pat, &segs)
}

/// Ignore rule check for one relative POSIX path.
///
/// * A pattern **without** `/` matches if any path component matches it.
/// * A pattern **with** `/` must match the whole path (anchored at the root).
#[must_use]
pub fn is_ignored(rel_path: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| {
        if pattern.contains('/') {
            full_match(pattern, rel_path)
        } else {
            rel_path
                .split('/')
                .any(|component| wildcard_match(pattern, component))
        }
    })
}

fn match_segments(pat: &[&str], path: &[&str]) -> bool {
    match pat.split_first() {
        None => path.is_empty(),
        Some((&"**", rest)) => (0..=path.len()).any(|skip| match_segments(rest, &path[skip..])),
        Some((&segment, rest)) => match path.split_first() {
            Some((&name, path_rest)) => {
                wildcard_match(segment, name) && match_segments(rest, path_rest)
            }
            None => false,
        },
    }
}

/// Single-segment wildcard match (`*` and `?`), iterative with one backtrack point.
fn wildcard_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0, 0);
    let mut star: Option<usize> = None;
    let mut mark = 0;
    while ti < t.len() {
        if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}
