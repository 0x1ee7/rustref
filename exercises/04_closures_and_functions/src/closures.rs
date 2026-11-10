//! Part 1: closures, captures and the `Fn` / `FnMut` / `FnOnce` hierarchy.

/// Returns a closure that multiplies its argument by `factor`.
///
/// The closure owns its captured `factor`; it must not borrow the caller's data.
pub fn make_multiplier(_factor: i64) -> impl Fn(i64) -> i64 {
    |_x: i64| -> i64 { todo!() }
}

/// Returns a closure that yields `start`, then `start + 1`, and so on.
///
/// Increments saturate at `u32::MAX` instead of wrapping or panicking.
pub fn make_counter(_start: u32) -> impl FnMut() -> u32 {
    || -> u32 { todo!() }
}

/// Composes two functions so that `compose(f, g)(x) == g(f(x))`.
///
/// Works across types: `A` to `B` to `C`.
pub fn compose<A, B, C>(_f: impl Fn(A) -> B, _g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    |_a: A| -> C { todo!() }
}

/// Returns `n` adders, where the adder at index `i` adds `i`.
///
/// Each closure must own its own offset. Capturing the loop variable by
/// reference is a compile error here, and that is the point.
pub fn make_adders(_n: usize) -> Vec<impl Fn(i64) -> i64> {
    vec![|_x: i64| -> i64 { todo!() }]
}

/// Calls `f` exactly `n` times and collects the results in call order.
///
/// Takes `FnMut` because the closure is expected to mutate state across calls.
pub fn call_n_times<T>(_n: usize, _f: impl FnMut() -> T) -> Vec<T> {
    todo!()
}

/// Runs a one-shot job. The job may consume the values it captured.
pub fn run_once<T>(_job: impl FnOnce() -> T) -> T {
    todo!()
}

/// Returns a reusable greeter: `"{greeting}, {name}!"`.
///
/// The returned closure is `Fn` (callable many times, without consuming its
/// captures) and accepts a borrowed `&str` of any lifetime.
pub fn make_greeter(_greeting: String) -> impl Fn(&str) -> String {
    |_name: &str| -> String { todo!() }
}
