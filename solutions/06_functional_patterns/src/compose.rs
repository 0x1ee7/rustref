//! Part 2 · closure composition.

/// Right-to-left composition: `compose(f, g)(x) == f(g(x))`.
///
/// The intermediate type may change across the boundary.
pub fn compose<A, B, C>(f: impl Fn(B) -> C, g: impl Fn(A) -> B) -> impl Fn(A) -> C {
    move |a| f(g(a))
}

/// Left-to-right pipeline over `steps`. Empty `steps` is the identity.
///
/// The returned closure owns its steps and can be called any number of times.
pub fn pipeline<T>(steps: Vec<Box<dyn Fn(T) -> T>>) -> impl Fn(T) -> T {
    move |value| steps.iter().fold(value, |acc, step| step(acc))
}
