//! Part 3: `newtype!`, a tuple-struct wrapper with opt-in trait impls.

/// Generates `struct Name(Inner);` with a private field.
///
/// Syntax: `newtype!( [attrs] [vis] Name(Inner) [: Trait, Trait, ...] );`
///
/// Always generated:
/// - `#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]`
/// - `const fn new(Inner) -> Self`, and `fn into_inner(self) -> Inner`
/// - `From<Inner>`
///
/// Opt-in traits, each delegating to `Inner`:
/// - `Copy`
/// - `Display`
/// - `FromStr` (with `Err = <Inner as FromStr>::Err`)
///
/// Any other name is a `compile_error!`.
///
/// Stub: the generated methods and the opt-in impls are `todo!()`.
#[macro_export]
macro_rules! newtype {
    (@trait $name:ident, $inner:ty, Copy) => {
        impl ::core::marker::Copy for $name {}
    };
    (@trait $name:ident, $inner:ty, Display) => {
        impl ::core::fmt::Display for $name {
            fn fmt(&self, _f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                todo!("newtype!: Display")
            }
        }
    };
    (@trait $name:ident, $inner:ty, FromStr) => {
        impl ::core::str::FromStr for $name {
            type Err = <$inner as ::core::str::FromStr>::Err;
            fn from_str(_s: &str) -> ::core::result::Result<Self, Self::Err> {
                todo!("newtype!: FromStr")
            }
        }
    };
    (@trait $name:ident, $inner:ty, $other:ident) => {
        compile_error!(concat!("newtype!: unsupported trait `", stringify!($other), "`"));
    };
    (
        $(#[$meta:meta])*
        $vis:vis $name:ident($inner:ty) $(: $($trait:ident),+ $(,)?)?
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $vis struct $name($inner);

        impl $name {
            /// Wraps a raw value.
            $vis const fn new(_value: $inner) -> Self {
                panic!("newtype!: new")
            }

            /// Unwraps the raw value.
            $vis fn into_inner(self) -> $inner {
                todo!("newtype!: into_inner")
            }
        }

        impl ::core::convert::From<$inner> for $name {
            fn from(_value: $inner) -> Self {
                todo!("newtype!: From")
            }
        }

        $( $( $crate::newtype!(@trait $name, $inner, $trait); )+ )?
    };
}
