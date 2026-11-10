//! Part 6 — async pagination with `async fn next`, and an async resource pool with RAII leases.

use std::collections::VecDeque;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use thiserror::Error;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::error::ConfigError;

/// One page of an upstream listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    /// The items on this page.
    pub items: Vec<T>,
    /// Cursor for the following page, or `None` when this was the last one.
    pub next_cursor: Option<String>,
}

/// Lazily walks cursor-paginated listings, yielding one item at a time.
///
/// `next_request` is the state machine: `Some(cursor)` means another page may be requested with
/// that cursor, and `None` means the walk has ended (last page, fetch error, or page budget).
pub struct Paginator<T, E, F> {
    fetch: F,
    buffered: VecDeque<T>,
    next_request: Option<Option<String>>,
    max_pages: Option<usize>,
    pages_fetched: usize,
    _error: PhantomData<E>,
}

impl<T, E, F, Fut> Paginator<T, E, F>
where
    F: FnMut(Option<String>) -> Fut,
    Fut: Future<Output = Result<Page<T>, E>>,
{
    /// Creates a paginator whose first request has no cursor.
    /// `max_pages == Some(0)` is `ConfigError::ZeroMaxPages`.
    pub fn new(fetch: F, max_pages: Option<usize>) -> Result<Self, ConfigError> {
        if max_pages == Some(0) {
            return Err(ConfigError::ZeroMaxPages);
        }
        Ok(Self {
            fetch,
            buffered: VecDeque::new(),
            next_request: Some(None),
            max_pages,
            pages_fetched: 0,
            _error: PhantomData,
        })
    }

    /// Number of page requests issued so far, including a failed one.
    pub fn pages_fetched(&self) -> usize {
        self.pages_fetched
    }

    /// Yields the next item, requesting a page only when the buffer is empty.
    ///
    /// Returns `None` once the walk has ended; it keeps returning `None` afterwards.
    pub async fn next(&mut self) -> Option<Result<T, E>> {
        loop {
            if let Some(item) = self.buffered.pop_front() {
                return Some(Ok(item));
            }
            if self.max_pages.is_some_and(|max| self.pages_fetched >= max) {
                return None;
            }
            // Taking the request means a failed fetch leaves no request behind: the walk ends.
            let cursor = self.next_request.take()?;
            self.pages_fetched += 1;
            match (self.fetch)(cursor).await {
                Ok(page) => {
                    self.buffered = page.items.into();
                    self.next_request = page.next_cursor.map(Some);
                }
                Err(error) => return Some(Err(error)),
            }
        }
    }
}

/// Creates and closes the resources of a [`ResourcePool`].
///
/// Methods return `Send` futures directly (return-position `impl Trait` in traits), so no
/// `async_trait` boxing is needed.
pub trait Factory: Send + Sync + 'static {
    /// The pooled resource.
    type Resource: Send + 'static;
    /// Why creating a resource failed.
    type Error: Send + 'static;

    /// Creates a new resource.
    fn create(&self) -> impl Future<Output = Result<Self::Resource, Self::Error>> + Send;

    /// Releases a resource. Called for idle resources on `close`, and for leased ones on drop
    /// after the pool is closed.
    fn close(&self, resource: Self::Resource) -> impl Future<Output = ()> + Send;
}

/// Why a lease was not granted.
#[derive(Debug, Error)]
pub enum LeaseError<E> {
    /// The pool was closed.
    #[error("pool is closed")]
    Closed,
    /// The factory failed to create a resource.
    #[error("creating a resource failed")]
    Factory(E),
}

/// Mutable pool state, guarded by one std mutex that is never held across `.await`.
struct State<R> {
    idle: Vec<R>,
    closed: bool,
}

struct Inner<F: Factory> {
    factory: F,
    slots: Arc<Semaphore>,
    state: Mutex<State<F::Resource>>,
    created: AtomicUsize,
}

/// A bounded pool of resources. Holding a [`Lease`] holds one of `size` slots.
pub struct ResourcePool<F: Factory> {
    inner: Arc<Inner<F>>,
}

impl<F: Factory> ResourcePool<F> {
    /// Creates a pool with `size` slots. `size == 0` is `ConfigError::ZeroLimit`.
    pub fn new(factory: F, size: usize) -> Result<Self, ConfigError> {
        if size == 0 {
            return Err(ConfigError::ZeroLimit);
        }
        Ok(Self {
            inner: Arc::new(Inner {
                factory,
                slots: Arc::new(Semaphore::new(size)),
                state: Mutex::new(State {
                    idle: Vec::new(),
                    closed: false,
                }),
                created: AtomicUsize::new(0),
            }),
        })
    }

    /// Number of resources the factory has successfully created.
    pub fn created(&self) -> usize {
        self.inner.created.load(Ordering::SeqCst)
    }

    /// Number of resources currently sitting idle in the pool.
    pub fn idle(&self) -> usize {
        self.inner
            .state
            .lock()
            .expect("pool state lock poisoned")
            .idle
            .len()
    }

    /// Obtains a resource, reusing an idle one when possible and waiting for a free slot otherwise.
    ///
    /// Cancel-safe: a dropped waiter never takes a slot.
    pub async fn lease(&self) -> Result<Lease<F>, LeaseError<F::Error>> {
        let permit = Arc::clone(&self.inner.slots)
            .acquire_owned()
            .await
            .map_err(|_| LeaseError::Closed)?;

        let idle = {
            let mut state = self.inner.state.lock().expect("pool state lock poisoned");
            if state.closed {
                return Err(LeaseError::Closed);
            }
            state.idle.pop()
        };
        let resource = match idle {
            Some(resource) => resource,
            None => {
                // On failure `?` drops `permit`, freeing the slot.
                let resource = self
                    .inner
                    .factory
                    .create()
                    .await
                    .map_err(LeaseError::Factory)?;
                self.inner.created.fetch_add(1, Ordering::SeqCst);
                resource
            }
        };
        Ok(Lease {
            resource: Some(resource),
            inner: Arc::clone(&self.inner),
            _permit: permit,
        })
    }

    /// Closes the pool: idle resources are closed now, leased ones when their lease is dropped,
    /// and waiting or new leases fail with `LeaseError::Closed`.
    pub async fn close(&self) {
        let idle = {
            let mut state = self.inner.state.lock().expect("pool state lock poisoned");
            state.closed = true;
            std::mem::take(&mut state.idle)
        };
        self.inner.slots.close();
        for resource in idle {
            self.inner.factory.close(resource).await;
        }
    }
}

/// RAII handle to a leased resource. Derefs to the resource.
///
/// Dropping it returns the resource to the pool (or closes it if the pool is closed) and frees
/// the slot. The release is synchronous, so it is safe during unwinding.
pub struct Lease<F: Factory> {
    resource: Option<F::Resource>,
    inner: Arc<Inner<F>>,
    _permit: OwnedSemaphorePermit,
}

impl<F: Factory> Deref for Lease<F> {
    type Target = F::Resource;

    fn deref(&self) -> &F::Resource {
        self.resource
            .as_ref()
            .expect("lease holds its resource until dropped")
    }
}

impl<F: Factory> DerefMut for Lease<F> {
    fn deref_mut(&mut self) -> &mut F::Resource {
        self.resource
            .as_mut()
            .expect("lease holds its resource until dropped")
    }
}

impl<F: Factory> Drop for Lease<F> {
    fn drop(&mut self) {
        let Some(resource) = self.resource.take() else {
            return;
        };
        let mut state = self.inner.state.lock().expect("pool state lock poisoned");
        if state.closed {
            drop(state);
            let inner = Arc::clone(&self.inner);
            tokio::spawn(async move { inner.factory.close(resource).await });
        } else {
            state.idle.push(resource);
        }
        // `_permit` is released after this body, so the resource is visible as idle first.
    }
}
