//! Part 6 — GATs (lending iterators), RPITIT, and a lazy `Deref` proxy.

use std::io::{self, Read};
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::ops::Deref;

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
///
/// Engineer: add the reader, the reusable buffer, and the chunk size.
pub struct ChunkReader<R> {
    _reader: PhantomData<R>,
}

impl<R: Read> ChunkReader<R> {
    /// Wraps `reader`. `chunk_size` is non-zero by type, so there is no
    /// zero-size error path.
    pub fn new(reader: R, chunk_size: NonZeroUsize) -> Self {
        let _ = (reader, chunk_size);
        todo!()
    }
}

impl<R: Read> LendingIterator for ChunkReader<R> {
    type Item<'a>
        = io::Result<&'a [u8]>
    where
        Self: 'a;

    fn next(&mut self) -> Option<Self::Item<'_>> {
        let _ = self;
        todo!()
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
        let _ = self;
        todo!()
    }
}

/// A single flat list of entries.
///
/// Engineer: store a `Vec<Entry>`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryCatalog {}

impl MemoryCatalog {
    /// Wraps `entries`, keeping their order.
    pub fn new(entries: Vec<Entry>) -> Self {
        let _ = entries;
        todo!()
    }
}

impl Catalog for MemoryCatalog {
    #[allow(unreachable_code)]
    fn entries(&self) -> impl Iterator<Item = &Entry> + '_ {
        let _ = self;
        todo!();
        std::iter::empty()
    }
}

/// Several shards, iterated in shard order then entry order.
///
/// Engineer: store a `Vec<MemoryCatalog>`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShardedCatalog {}

impl ShardedCatalog {
    /// Wraps the shards in the given order.
    pub fn new(shards: Vec<MemoryCatalog>) -> Self {
        let _ = shards;
        todo!()
    }
}

impl Catalog for ShardedCatalog {
    #[allow(unreachable_code)]
    fn entries(&self) -> impl Iterator<Item = &Entry> + '_ {
        let _ = self;
        todo!();
        std::iter::empty()
    }
}

/// The largest entry. On equal sizes the **first** one in catalog order wins.
/// `None` for an empty catalog.
pub fn largest<C: Catalog>(catalog: &C) -> Option<&Entry> {
    let _ = catalog;
    todo!()
}

/// Lazily initialised value: `init` runs on first access, once, even under
/// concurrent first access. `Deref` forces initialisation.
///
/// Engineer: hold a `std::sync::OnceLock<T>` and the `init` closure. `F` is the
/// closure type, so `Lazy<T, F>` is `Sync` whenever `T` and `F` are.
pub struct Lazy<T, F> {
    _state: PhantomData<(T, F)>,
}

impl<T, F: Fn() -> T> Lazy<T, F> {
    /// Stores `init` without running it.
    pub fn new(init: F) -> Self {
        let _ = init;
        todo!()
    }

    /// Forces initialisation and returns the stored value.
    ///
    /// Associated function rather than a method, so it never shadows methods of `T`.
    pub fn force(this: &Self) -> &T {
        let _ = this;
        todo!()
    }

    /// `true` once the value has been initialised. Never triggers initialisation.
    pub fn is_initialized(this: &Self) -> bool {
        let _ = this;
        todo!()
    }
}

impl<T, F: Fn() -> T> Deref for Lazy<T, F> {
    type Target = T;

    fn deref(&self) -> &T {
        let _ = self;
        todo!()
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
        let _ = base_url;
        todo!()
    }

    /// The normalised base URL.
    pub fn base_url(&self) -> &str {
        let _ = self;
        todo!()
    }

    /// Joins `path` onto the base with exactly one `/` between them.
    pub fn url_for(&self, path: &str) -> String {
        let _ = (self, path);
        todo!()
    }
}
