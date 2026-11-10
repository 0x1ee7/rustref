//! Part 4: `bail!` and `ensure!`, early returns for `Result`-returning functions.

/// Returns `Err(err.into())` from the enclosing function.
///
/// The enclosing function must return a `Result<_, E>` where `E: From<T>` for the
/// argument type `T` (the conversion is `Into::into`). Works as a tail expression.
#[macro_export]
macro_rules! bail {
    ($err:expr $(,)?) => {
        todo!("bail!")
    };
}

/// Returns early with `err` if `cond` is false.
///
/// `cond` is evaluated once. `err` is evaluated only when `cond` is false.
/// The negation keeps `cond`'s precedence intact. Built on [`bail!`].
#[macro_export]
macro_rules! ensure {
    ($cond:expr, $err:expr $(,)?) => {
        if true {
            todo!("ensure!")
        }
    };
}
