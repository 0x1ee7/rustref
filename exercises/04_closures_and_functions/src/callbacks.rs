//! Part 2: returning closures, `fn` pointers and boxed callback registries.

/// A boxed event listener stored by an [`EventBus`].
type Listener<E> = Box<dyn Fn(&E) + Send + Sync>;

/// A named, non-capturing binary operation. Coerces from non-capturing closures.
pub type Operation = fn(i64, i64) -> Option<i64>;

/// Looks up an operation by name: `add`, `sub`, `mul`, `div`.
///
/// Returns `None` for unknown names. Names are case-sensitive.
pub fn lookup_operation(_name: &str) -> Option<Operation> {
    todo!()
}

/// Builds a price-discount rule as a boxed, thread-safe closure.
///
/// - `"percent"`: discount is `floor(price * amount / 100)`; `amount` must be 0..=100.
/// - `"flat"`: discount is `amount`, saturating at zero.
/// - Any other kind, or an out-of-range percent, returns `None`.
///
/// Each arm returns a different closure type, so the result must be boxed.
pub fn discount_rule(_kind: &str, _amount: u64) -> Option<Box<dyn Fn(u64) -> u64 + Send + Sync>> {
    todo!()
}

/// Returns an eligibility predicate: `price >= min` (inclusive).
pub fn at_least(_min: u64) -> impl Fn(u64) -> bool {
    |_price: u64| -> bool { todo!() }
}

/// Identifies one subscription on an [`EventBus`]. Ids are never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[allow(dead_code)] // stub: remove once the id is read
pub struct SubscriptionId(u64);

/// A synchronous publish/subscribe bus for events of type `E`.
///
/// Listeners are `Fn` (not `FnMut`) so `publish` can take `&self`. They are
/// `Send + Sync` so the bus can be shared across threads.
#[allow(dead_code)] // stub: remove once the fields are used
pub struct EventBus<E> {
    listeners: Vec<(SubscriptionId, Listener<E>)>,
    next_id: u64,
}

impl<E> EventBus<E> {
    /// Creates an empty bus.
    pub fn new() -> Self {
        todo!()
    }

    /// Registers a listener and returns its id.
    pub fn subscribe<F>(&mut self, _listener: F) -> SubscriptionId
    where
        F: Fn(&E) + Send + Sync + 'static,
    {
        todo!()
    }

    /// Removes a listener. Returns `false` if the id is not registered.
    pub fn unsubscribe(&mut self, _id: SubscriptionId) -> bool {
        todo!()
    }

    /// Calls every listener with `event`, in subscription order.
    ///
    /// Returns the number of listeners that ran.
    pub fn publish(&self, _event: &E) -> usize {
        todo!()
    }

    /// Number of registered listeners.
    pub fn len(&self) -> usize {
        todo!()
    }

    /// `true` when no listeners are registered.
    pub fn is_empty(&self) -> bool {
        todo!()
    }
}

impl<E> Default for EventBus<E> {
    fn default() -> Self {
        Self::new()
    }
}
