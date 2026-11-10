//! Part 6 — GATs (lending iterators), RPITIT, and a lazy `Deref` proxy.

use std::io::{self, ErrorKind, Read};
use std::num::NonZeroUsize;
use std::ops::Deref;
use std::sync::OnceLock;

/// An iterator whose items borrow from the iterator itself, so one buffer can
/// be reused across calls. Requires a GAT: `Item<'a>` depends on the borrow.
pub trait LendingIterator {
    type Item<'a>
    where
        Self: 'a;

    /// Advances; the item borrows `self` mutably-derived storage.
    fn next(&mut self) -> Option<Self::Item<'_>>;
}

/// Yields chunks of at most `chunk_size` bytes from a reader, reusing one buffer.
///
/// `next` performs one read per call. `Interrupted` reads are retried
/// transparently. Other I/O errors are yielded as `Some(Err(_))`. End of
/// stream is `None`; an exact multiple of the chunk size produces no empty
/// trailing chunk.
pub struct ChunkReader<R> {
    reader: R,
    buffer: Vec<u8>,
}

impl<R: Read> ChunkReader<R> {
    /// Wraps `reader`. `chunk_size` is non-zero by type, so there is no
    /// zero-size error path.
    pub fn new(reader: R, chunk_size: NonZeroUsize) -> Self {
        Self { reader, buffer: vec![0; chunk_size.get()] }
    }
}

impl<R: Read> LendingIterator for ChunkReader<R> {
    type Item<'a>
        = io::Result<&'a [u8]>
    where
        Self: 'a;

    fn next(&mut self) -> Option<Self::Item<'_>> {
        loop {
            match self.reader.read(&mut self.buffer) {
                Ok(0) => return None,
                Ok(n) => return Some(Ok(&self.buffer[..n])),
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

/// A named blob with a size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub size: u64,
}

/// A read-only collection of entries.
///
/// `entries` returns an opaque iterator (RPITIT): every implementation keeps
/// its own concrete iterator type, with no boxing. Consequently the trait is
/// not dyn-compatible; use it as a generic bound.
pub trait Catalog {
    /// Entries in catalog order.
    fn entries(&self) -> impl Iterator<Item = &Entry> + '_;

    /// Sum of all entry sizes.
    fn total_size(&self) -> u64 {
        self.entries().map(|entry| entry.size).sum()
    }
}

/// A single flat list of entries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryCatalog {
    items: Vec<Entry>,
}

impl MemoryCatalog {
    /// Wraps `entries`, keeping their order.
    pub fn new(entries: Vec<Entry>) -> Self {
        Self { items: entries }
    }
}

impl Catalog for MemoryCatalog {
    fn entries(&self) -> impl Iterator<Item = &Entry> + '_ {
        self.items.iter()
    }
}

/// Several shards, iterated in shard order then entry order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShardedCatalog {
    shards: Vec<MemoryCatalog>,
}

impl ShardedCatalog {
    /// Wraps the shards in the given order.
    pub fn new(shards: Vec<MemoryCatalog>) -> Self {
        Self { shards }
    }
}

impl Catalog for ShardedCatalog {
    fn entries(&self) -> impl Iterator<Item = &Entry> + '_ {
        self.shards.iter().flat_map(|shard| shard.entries())
    }
}

/// The largest entry. On equal sizes the **first** one in catalog order wins.
/// `None` for an empty catalog.
pub fn largest<C: Catalog>(catalog: &C) -> Option<&Entry> {
    catalog
        .entries()
        .reduce(|best, candidate| if candidate.size > best.size { candidate } else { best })
}

/// Lazily initialised value: `init` runs on first access, once, even under
/// concurrent first access. `Deref` forces initialisation.
///
/// `F` is the closure type, so `Lazy<T, F>` is `Sync` whenever `T` and `F` are.
pub struct Lazy<T, F> {
    cell: OnceLock<T>,
    init: F,
}

impl<T, F: Fn() -> T> Lazy<T, F> {
    /// Stores `init` without running it.
    pub fn new(init: F) -> Self {
        Self { cell: OnceLock::new(), init }
    }

    /// Forces initialisation and returns the stored value.
    ///
    /// Associated function rather than a method, so it never shadows methods of `T`.
    pub fn force(this: &Self) -> &T {
        this.cell.get_or_init(|| (this.init)())
    }

    /// `true` once the value has been initialised. Never triggers initialisation.
    pub fn is_initialized(this: &Self) -> bool {
        this.cell.get().is_some()
    }
}

impl<T, F: Fn() -> T> Deref for Lazy<T, F> {
    type Target = T;

    fn deref(&self) -> &T {
        Lazy::force(self)
    }
}

/// A stand-in for an expensive API client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiClient {
    base_url: String,
}

impl ApiClient {
    /// Stores the base URL with any trailing `/` removed.
    pub fn new(base_url: &str) -> Self {
        Self { base_url: base_url.trim_end_matches('/').to_owned() }
    }

    /// The normalised base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Joins `path` onto the base with exactly one `/` between them.
    pub fn url_for(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }
}
