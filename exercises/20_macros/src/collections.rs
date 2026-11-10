//! Part 1: `count!`, `hashmap!`, `btreeset!`.

/// Number of comma-separated expressions, as a `usize`.
///
/// Arguments are never evaluated, and the result is usable in `const`
/// contexts. A trailing comma is allowed; `count!()` is `0`.
#[macro_export]
macro_rules! count {
    ($($item:expr),* $(,)?) => {
        todo!("count!")
    };
}

/// `HashMap` literal: `hashmap! { k => v, ... }`.
///
/// - Accepts any number of pairs; the trailing comma is optional; `hashmap! {}` is empty.
/// - Pre-sizes the map from the pair count (`with_capacity`), using `count!`.
/// - Each key and each value is evaluated exactly once, in source order (key before value).
/// - A duplicate key keeps the last value, like repeated `insert`.
#[macro_export]
macro_rules! hashmap {
    ($($key:expr => $value:expr),* $(,)?) => {
        todo!("hashmap!")
    };
}

/// `BTreeSet` literal: `btreeset![a, b, c]`, sorted and de-duplicated.
///
/// Trailing comma optional; `btreeset![]` is empty.
#[macro_export]
macro_rules! btreeset {
    ($($item:expr),* $(,)?) => {
        todo!("btreeset!")
    };
}
