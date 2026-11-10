//! Part 4 — resource-owning iterators and batched writes.

use std::io::{self, BufRead};
use std::marker::PhantomData;

/// Iterator of raw line batches over a resource it owns. Returned by [`read_batches`].
///
/// The resource is opened on the first `next()`, dropped as soon as the source is
/// exhausted or fails, and dropped with the iterator on early `break`.
pub struct ReadBatches<R, F> {
    _open: F,
    _marker: PhantomData<fn() -> R>,
}

/// Lazily open a source and yield batches of up to `size` raw lines (newline kept).
///
/// `open` runs on the first `next()`, not when this is called. Panics if `size == 0`.
pub fn read_batches<R, F>(_open: F, _size: usize) -> ReadBatches<R, F>
where
    R: BufRead,
    F: FnOnce() -> io::Result<R>,
{
    todo!()
}

impl<R, F> Iterator for ReadBatches<R, F>
where
    R: BufRead,
    F: FnOnce() -> io::Result<R>,
{
    type Item = io::Result<Vec<String>>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

/// Split `rows` into batches of `size`, pass each batch (by value) to `write`, and sum
/// the counts `write` returns. Stops at the first error. Pulls only what the current
/// batch needs. Panics if `size == 0`.
pub fn write_in_batches<T, E, W>(
    _rows: impl IntoIterator<Item = T>,
    _size: usize,
    _write: W,
) -> Result<usize, E>
where
    W: FnMut(Vec<T>) -> Result<usize, E>,
{
    todo!()
}
