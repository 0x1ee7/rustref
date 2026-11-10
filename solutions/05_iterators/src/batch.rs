//! Part 4 — resource-owning iterators and batched writes.

use std::io::{self, BufRead};

/// Iterator of raw line batches over a resource it owns. Returned by [`read_batches`].
///
/// The resource is opened on the first `next()`, dropped as soon as the source is
/// exhausted or fails, and dropped with the iterator on early `break`.
pub struct ReadBatches<R, F> {
    open: Option<F>,
    reader: Option<R>,
    size: usize,
}

/// Lazily open a source and yield batches of up to `size` raw lines (newline kept).
///
/// `open` runs on the first `next()`, not when this is called. Panics if `size == 0`.
pub fn read_batches<R, F>(open: F, size: usize) -> ReadBatches<R, F>
where
    R: BufRead,
    F: FnOnce() -> io::Result<R>,
{
    assert!(size >= 1, "batch size must be at least 1");
    ReadBatches {
        open: Some(open),
        reader: None,
        size,
    }
}

impl<R, F> Iterator for ReadBatches<R, F>
where
    R: BufRead,
    F: FnOnce() -> io::Result<R>,
{
    type Item = io::Result<Vec<String>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(open) = self.open.take() {
            match open() {
                Ok(reader) => self.reader = Some(reader),
                Err(e) => return Some(Err(e)),
            }
        }

        let mut batch = Vec::with_capacity(self.size);
        let mut failure = None;
        {
            let reader = self.reader.as_mut()?;
            while batch.len() < self.size {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => batch.push(line),
                    Err(e) => {
                        failure = Some(e);
                        break;
                    }
                }
            }
        }

        if let Some(e) = failure {
            self.reader = None;
            return Some(Err(e));
        }
        if batch.len() < self.size {
            // Short batch means end of input: release the resource now.
            self.reader = None;
        }
        if batch.is_empty() {
            return None;
        }
        Some(Ok(batch))
    }
}

/// Split `rows` into batches of `size`, pass each batch (by value) to `write`, and sum
/// the counts `write` returns. Stops at the first error. Pulls only what the current
/// batch needs. Panics if `size == 0`.
pub fn write_in_batches<T, E, W>(
    rows: impl IntoIterator<Item = T>,
    size: usize,
    mut write: W,
) -> Result<usize, E>
where
    W: FnMut(Vec<T>) -> Result<usize, E>,
{
    assert!(size >= 1, "batch size must be at least 1");
    let mut rows = rows.into_iter();
    let mut total = 0;
    loop {
        let batch: Vec<T> = rows.by_ref().take(size).collect();
        if batch.is_empty() {
            return Ok(total);
        }
        total += write(batch)?;
    }
}
