//! Part 1: closures, captures and the `Fn` / `FnMut` / `FnOnce` hierarchy.

/// Returns a closure that multiplies its argument by `factor`.
///
/// The closure owns its captured `factor`; it must not borrow the caller's data.
pub fn make_multiplier(factor: i64) -> impl Fn(i64) -> i64 {
    move |x: i64| x * factor
}

/// Returns a closure that yields `start`, then `start + 1`, and so on.
///
/// Increments saturate at `u32::MAX` instead of wrapping or panicking.
pub fn make_counter(start: u32) -> impl FnMut() -> u32 {
    let mut next = start;
    move || {
        let current = next;
        next = next.saturating_add(1);
        current
    }
}

/// Composes two functions so that `compose(f, g)(x) == g(f(x))`.
///
/// Works across types: `A` to `B` to `C`.
pub fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |a: A| g(f(a))
}

/// Returns `n` adders, where the adder at index `i` adds `i`.
///
/// Each closure must own its own offset. Capturing the loop variable by
/// reference is a compile error here, and that is the point.
pub fn make_adders(n: usize) -> Vec<impl Fn(i64) -> i64> {
    (0..n)
        .map(|i| {
            let offset = i as i64;
            move |x: i64| x + offset
        })
        .collect()
}

/// Calls `f` exactly `n` times and collects the results in call order.
///
/// Takes `FnMut` because the closure is expected to mutate state across calls.
pub fn call_n_times<T>(n: usize, mut f: impl FnMut() -> T) -> Vec<T> {
    (0..n).map(|_| f()).collect()
}

/// Runs a one-shot job. The job may consume the values it captured.
pub fn run_once<T>(job: impl FnOnce() -> T) -> T {
    job()
}

/// Returns a reusable greeter: `"{greeting}, {name}!"`.
///
/// The returned closure is `Fn` (callable many times, without consuming its
/// captures) and accepts a borrowed `&str` of any lifetime.
pub fn make_greeter(greeting: String) -> impl Fn(&str) -> String {
    move |name: &str| format!("{greeting}, {name}!")
}
