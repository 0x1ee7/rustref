//! Part 2: walking an error's `source()` chain.

use std::error::Error;

/// Renders `err` followed by each error in its `source()` chain, outermost first.
///
/// Each entry is the error's `Display` output. Stops when the chain revisits an
/// error object, so a self-referential `source()` terminates. Identity is the
/// full fat pointer (address and vtable), so two distinct zero-sized errors at
/// the same address are still told apart.
pub fn chain_messages(err: &(dyn Error + 'static)) -> Vec<String> {
    let mut messages = Vec::new();
    let mut seen: Vec<*const (dyn Error + 'static)> = Vec::new();
    let mut current = Some(err);
    while let Some(error) = current {
        let ptr = std::ptr::from_ref(error);
        if seen.iter().any(|&prev| std::ptr::eq(prev, ptr)) {
            break;
        }
        seen.push(ptr);
        messages.push(error.to_string());
        current = error.source();
    }
    messages
}
