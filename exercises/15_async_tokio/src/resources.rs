//! Part 6 — async pagination with `async fn next`, and an async resource pool with RAII leases.

use std::future::Future;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use thiserror::Error;

use crate::error::ConfigError;

/// One page of an upstream listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    /// Items on this page. May be empty while `next_cursor` is still set.
    pub items: Vec<T>,
    /// Cursor for the following page, or `None` on the last page.
    pub next_cursor: Option<String>,
}

/// Lazily walks cursor-paginated listings, yielding one item at a time.
///
/// Contract:
/// - The first request uses cursor `None`; each later request uses the previous page's cursor.
/// - A page is requested only when the buffered items are used up.
/// - Empty pages that still carry a cursor are followed.
/// - `max_pages` caps how many pages are requested. `Some(0)` is `ConfigError::ZeroMaxPages`.
/// - A fetch error is yielded after the items already buffered, then the stream ends (`None`
///   forever after).
pub struct Paginator<T, E, F> {
    _types: PhantomData<(T, E, F)>,
}

impl<T, E, F, Fut> Paginator<T, E, F>
where
    F: FnMut(Option<String>) -> Fut,
    Fut: Future<Output = Result<Page<T>, E>>,
{
    /// Creates a paginator over `fetch`.
    pub fn new(_fetch: F, _max_pages: Option<usize>) -> Result<Self, ConfigError> {
        todo!()
    }

    /// Yields the next item, fetching a page first if necessary. `None` means exhausted.
    pub async fn next(&mut self) -> Option<Result<T, E>> {
        todo!()
    }

    /// Number of pages requested so far, including failed requests.
    pub fn pages_fetched(&self) -> usize {
        todo!()
    }
}

/// Creates and closes the resources of a [`ResourcePool`].
///
/// Implemented with native `async fn`-style methods returning `Send` futures, so no
/// `async_trait` boxing is needed.
pub trait Factory: Send + Sync + 'static {
    /// The pooled resource.
    type Resource: Send + 'static;
    /// Why creating a resource failed.
    type Error: Send + 'static;

    /// Creates a new resource.
    fn create(&self) -> impl Future<Output = Result<Self::Resource, Self::Error>> + Send;

    /// Closes a resource that will not be reused.
    fn close(&self, resource: Self::Resource) -> impl Future<Output = ()> + Send;
}

/// Why a lease was not granted.
#[derive(Debug, Error)]
pub enum LeaseError<E> {
    /// The pool was closed.
    #[error("pool is closed")]
    Closed,
    /// The factory failed to create a resource.
    #[error("factory failed: {0}")]
    Factory(E),
}

/// A bounded pool of resources. Holding a [`Lease`] holds one of `max_size` slots.
pub struct ResourcePool<F: Factory> {
    _factory: PhantomData<F>,
}

impl<F: Factory> ResourcePool<F> {
    /// Creates an empty pool. `max_size == 0` is `ConfigError::ZeroLimit`.
    pub fn new(_factory: F, _max_size: usize) -> Result<Self, ConfigError> {
        todo!()
    }

    /// Number of successful factory calls so far.
    pub fn created(&self) -> usize {
        todo!()
    }

    /// Number of resources waiting to be reused.
    pub fn idle(&self) -> usize {
        todo!()
    }

    /// Waits for a free slot, then hands out an idle resource or creates one.
    ///
    /// The resource returns to the pool when the lease is dropped, even on panic. Cancelling a
    /// waiting `lease` does not consume a slot.
    pub async fn lease(&self) -> Result<Lease<F>, LeaseError<F::Error>> {
        todo!()
    }

    /// Closes idle resources and rejects later leases. Leased resources are closed when their
    /// lease ends. Later calls do nothing.
    pub async fn close(&self) {
        todo!()
    }
}

/// RAII handle to a leased resource. Derefs to the resource.
pub struct Lease<F: Factory> {
    _factory: PhantomData<F>,
}

impl<F: Factory> Deref for Lease<F> {
    type Target = F::Resource;

    fn deref(&self) -> &F::Resource {
        todo!()
    }
}

impl<F: Factory> DerefMut for Lease<F> {
    fn deref_mut(&mut self) -> &mut F::Resource {
        todo!()
    }
}
