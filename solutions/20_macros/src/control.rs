//! Part 4: `bail!` and `ensure!`, early returns from `Result`-returning functions.

/// Returns `Err(err.into())` from the enclosing function.
///
/// Conversion goes through `Into`, so `From` impls apply. Works in statement
/// and tail position.
#[macro_export]
macro_rules! bail {
    ($err:expr $(,)?) => {
        return ::core::result::Result::Err(::core::convert::Into::into($err))
    };
}

/// Returns early with `err` if `cond` is false.
///
/// `cond` is evaluated once; `err` is evaluated only on failure. The
/// negation is parenthesised, so `cond`'s precedence is preserved.
#[macro_export]
macro_rules! ensure {
    ($cond:expr, $err:expr $(,)?) => {
        if !($cond) {
            $crate::bail!($err);
        }
    };
}
