//! Part 6: `assert_matches!`, a pattern-based assertion that can also extract bindings.

/// Asserts that `value` matches `pattern`, optionally with an `if` guard.
///
/// - `assert_matches!(v, pat => out)` evaluates to `out`, with the pattern's
///   bindings in scope.
/// - `assert_matches!(v, pat)` and `assert_matches!(v, pat if guard)` assert
///   and evaluate to `()`.
/// - `assert_matches!(v, pat, "fmt", args...)` prefixes the failure message.
///
/// `v` is evaluated once. On failure the panic shows the value's `Debug` output
/// and the pattern text.
#[macro_export]
macro_rules! assert_matches {
    ($value:expr, $pat:pat $(if $guard:expr)? => $out:expr $(,)?) => {
        match $value {
            $pat $(if $guard)? => $out,
            ref other => panic!(
                "assertion failed: {:?} does not match `{}`",
                other,
                stringify!($pat),
            ),
        }
    };
    ($value:expr, $pat:pat $(if $guard:expr)? $(,)?) => {
        $crate::assert_matches!($value, $pat $(if $guard)? => ())
    };
    ($value:expr, $pat:pat $(if $guard:expr)?, $($msg:tt)+) => {
        match $value {
            $pat $(if $guard)? => (),
            ref other => panic!(
                "{}: {:?} does not match `{}`",
                format_args!($($msg)+),
                other,
                stringify!($pat),
            ),
        }
    };
}
