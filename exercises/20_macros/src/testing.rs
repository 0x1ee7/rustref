//! Part 6: `assert_matches!`, a pattern-based assertion that can also extract bindings.

/// Asserts that `value` matches `pattern`, optionally with an `if` guard.
///
/// Three forms:
/// - `assert_matches!(v, pat)` or `assert_matches!(v, pat if guard)`. Unit result.
/// - `assert_matches!(v, pat => out)`. Evaluates to `out`, with the pattern's
///   bindings in scope. Returns by value, so owned bindings are moved out.
/// - `assert_matches!(v, pat, "fmt", args...)`. Custom message prefix.
///
/// `v` is evaluated exactly once. The failure panic shows the value's `Debug`
/// output and the pattern text. Or-patterns are accepted.
///
/// Stub: every form panics with `todo!()`.
#[macro_export]
macro_rules! assert_matches {
    ($value:expr, $pat:pat $(if $guard:expr)? => $out:expr $(,)?) => {
        todo!("assert_matches!")
    };
    ($value:expr, $pat:pat $(if $guard:expr)? $(,)?) => {
        if true {
            todo!("assert_matches!")
        }
    };
    ($value:expr, $pat:pat $(if $guard:expr)?, $($msg:tt)+) => {
        if true {
            todo!("assert_matches!")
        }
    };
}
