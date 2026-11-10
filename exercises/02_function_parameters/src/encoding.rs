//! Part 3 — iterator and `AsRef` arguments as the variadic analogue; `Cow` for
//! "maybe changed" results.

use std::borrow::Cow;

/// Percent-encodes `input`, keeping RFC 3986 unreserved bytes (`A-Z a-z 0-9 - . _ ~`)
/// literal. Returns `Cow::Borrowed` when nothing needs escaping. Hex digits are
/// upper-case and multi-byte UTF-8 is encoded byte by byte.
pub fn percent_encode(_input: &str) -> Cow<'_, str> {
    todo!()
}

/// Appends `segments` to `base`.
///
/// The trailing `/`s of `base` are trimmed. Each non-empty segment is
/// percent-encoded (so a `/` inside a segment becomes `%2F`) and prefixed with
/// `/`. Empty segments are skipped.
pub fn join_path(_base: &str, _segments: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    todo!()
}

/// Encodes `(key, value)` pairs as `k=v&k=v`, in iteration order, with keys and
/// values percent-encoded. An empty iterator yields an empty string.
///
/// The pair type is named by generics because the function needs to read both
/// halves of each item, which `impl Trait` in argument position cannot express
/// without a separate bound on the tuple.
pub fn encode_query<I, K, V>(_pairs: I) -> String
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: AsRef<str>,
{
    todo!()
}
