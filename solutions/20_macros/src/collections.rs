//! Part 1: `count!`, `hashmap!`, `btreeset!`.

/// Number of comma-separated expressions, as a `usize`.
///
/// Each argument is replaced by `()` (never evaluated), and the array of units
/// is measured with `<[()]>::len`. The count is a `usize` and works in `const`
/// contexts.
#[macro_export]
macro_rules! count {
    (@unit $_item:expr) => {
        ()
    };
    ($($item:expr),* $(,)?) => {
        <[()]>::len(&[$($crate::count!(@unit $item)),*])
    };
}

/// `HashMap` literal: `hashmap! { k => v, ... }`.
///
/// Pre-sizes with `count!`. Inserts in source order, so each key and value is
/// evaluated once, key first. A duplicate key keeps the last value.
#[macro_export]
macro_rules! hashmap {
    ($($key:expr => $value:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut map = ::std::collections::HashMap::with_capacity($crate::count!($($key),*));
        $( map.insert($key, $value); )*
        map
    }};
}

/// `BTreeSet` literal: `btreeset![a, b, c]`, sorted and de-duplicated.
#[macro_export]
macro_rules! btreeset {
    ($($item:expr),* $(,)?) => {
        ::std::collections::BTreeSet::from([$($item),*])
    };
}
