//! Part 2: walking an error's `source()` chain.

use std::error::Error;

/// Renders `_err` followed by each error in its `source()` chain, outermost first.
///
/// Each entry is the error's `Display` output. Stops when the chain revisits an
/// error object, so a self-referential `source()` terminates.
pub fn chain_messages(_err: &(dyn Error + 'static)) -> Vec<String> {
    todo!()
}
