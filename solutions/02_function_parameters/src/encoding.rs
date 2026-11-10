//! Part 3 — iterator and `AsRef` arguments as the variadic analogue; `Cow` for
//! "maybe changed" results.

use std::borrow::Cow;

/// RFC 3986 unreserved byte.
fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')
}

/// Percent-encodes `input`, keeping RFC 3986 unreserved bytes (`A-Z a-z 0-9 - . _ ~`)
/// literal. Returns `Cow::Borrowed` when nothing needs escaping. Hex digits are
/// upper-case and multi-byte UTF-8 is encoded byte by byte.
pub fn percent_encode(input: &str) -> Cow<'_, str> {
    if input.bytes().all(is_unreserved) {
        return Cow::Borrowed(input);
    }
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(input.len() * 3);
    for b in input.bytes() {
        if is_unreserved(b) {
            out.push(char::from(b));
        } else {
            out.push('%');
            out.push(char::from(HEX[usize::from(b >> 4)]));
            out.push(char::from(HEX[usize::from(b & 0x0f)]));
        }
    }
    Cow::Owned(out)
}

/// Appends `segments` to `base`.
///
/// The trailing `/`s of `base` are trimmed. Each non-empty segment is
/// percent-encoded (so a `/` inside a segment becomes `%2F`) and prefixed with
/// `/`. Empty segments are skipped.
pub fn join_path(base: &str, segments: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    let mut out = base.trim_end_matches('/').to_owned();
    for segment in segments {
        let segment = segment.as_ref();
        if segment.is_empty() {
            continue;
        }
        out.push('/');
        out.push_str(&percent_encode(segment));
    }
    out
}

/// Encodes `(key, value)` pairs as `k=v&k=v`, in iteration order, with keys and
/// values percent-encoded. An empty iterator yields an empty string.
///
/// The pair type is named by generics because the function needs to read both
/// halves of each item, which `impl Trait` in argument position cannot express
/// without a separate bound on the tuple.
pub fn encode_query<I, K, V>(pairs: I) -> String
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: AsRef<str>,
{
    let mut out = String::new();
    for (i, (key, value)) in pairs.into_iter().enumerate() {
        if i > 0 {
            out.push('&');
        }
        out.push_str(&percent_encode(key.as_ref()));
        out.push('=');
        out.push_str(&percent_encode(value.as_ref()));
    }
    out
}
