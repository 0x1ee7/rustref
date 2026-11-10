//! Part 3: `newtype!`, a tuple-struct wrapper with opt-in trait impls.

/// Generates `struct Name(Inner);` with a private field.
///
/// Always derives `Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash` and
/// provides `new`, `into_inner`, and `From<Inner>`. Opt-in traits, listed after
/// `:`, delegate to `Inner`: `Copy`, `Display`, `FromStr`. Any other name is a
/// `compile_error!`.
#[macro_export]
macro_rules! newtype {
    (@trait $name:ident, $inner:ty, Copy) => {
        impl ::core::marker::Copy for $name {}
    };
    (@trait $name:ident, $inner:ty, Display) => {
        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Display::fmt(&self.0, f)
            }
        }
    };
    (@trait $name:ident, $inner:ty, FromStr) => {
        impl ::core::str::FromStr for $name {
            type Err = <$inner as ::core::str::FromStr>::Err;
            fn from_str(s: &str) -> ::core::result::Result<Self, Self::Err> {
                s.parse::<$inner>().map(Self)
            }
        }
    };
    (@trait $name:ident, $inner:ty, $other:ident) => {
        ::core::compile_error!(::core::concat!(
            "newtype!: unsupported trait `",
            ::core::stringify!($other),
            "`"
        ));
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
            $vis const fn new(value: $inner) -> Self {
                Self(value)
            }

            /// Unwraps the raw value.
            $vis fn into_inner(self) -> $inner {
                self.0
            }
        }

        impl ::core::convert::From<$inner> for $name {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }

        $( $( $crate::newtype!(@trait $name, $inner, $trait); )+ )?
    };
}
