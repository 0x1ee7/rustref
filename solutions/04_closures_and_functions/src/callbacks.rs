//! Part 2: returning closures, `fn` pointers and boxed callback registries.

/// A boxed event listener stored by an [`EventBus`].
type Listener<E> = Box<dyn Fn(&E) + Send + Sync>;

/// A named, non-capturing binary operation. Coerces from non-capturing closures.
pub type Operation = fn(i64, i64) -> Option<i64>;

fn add(a: i64, b: i64) -> Option<i64> {
    a.checked_add(b)
}

fn sub(a: i64, b: i64) -> Option<i64> {
    a.checked_sub(b)
}

fn mul(a: i64, b: i64) -> Option<i64> {
    a.checked_mul(b)
}

fn div(a: i64, b: i64) -> Option<i64> {
    a.checked_div(b)
}

/// Looks up an operation by name: `add`, `sub`, `mul`, `div`.
///
/// Returns `None` for unknown names. Names are case-sensitive.
pub fn lookup_operation(name: &str) -> Option<Operation> {
    match name {
        "add" => Some(add),
        "sub" => Some(sub),
        "mul" => Some(mul),
        "div" => Some(div),
        _ => None,
    }
}

/// Builds a price-discount rule as a boxed, thread-safe closure.
///
/// - `"percent"`: discount is `floor(price * amount / 100)`; `amount` must be 0..=100.
/// - `"flat"`: discount is `amount`, saturating at zero.
/// - Any other kind, or an out-of-range percent, returns `None`.
///
/// Each arm returns a different closure type, so the result must be boxed.
pub fn discount_rule(kind: &str, amount: u64) -> Option<Box<dyn Fn(u64) -> u64 + Send + Sync>> {
    match kind {
        "percent" if amount <= 100 => {
            let rule: Box<dyn Fn(u64) -> u64 + Send + Sync> = Box::new(move |price: u64| {
                // Widen so `price * amount` cannot overflow; the discount is at most `price`.
                let discount = (u128::from(price) * u128::from(amount) / 100) as u64;
                price - discount
            });
            Some(rule)
        }
        "flat" => {
            let rule: Box<dyn Fn(u64) -> u64 + Send + Sync> =
                Box::new(move |price: u64| price.saturating_sub(amount));
            Some(rule)
        }
        _ => None,
    }
}

/// Returns an eligibility predicate: `price >= min` (inclusive).
pub fn at_least(min: u64) -> impl Fn(u64) -> bool {
    move |price: u64| price >= min
}

/// Identifies one subscription on an [`EventBus`]. Ids are never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SubscriptionId(u64);

/// A synchronous publish/subscribe bus for events of type `E`.
///
/// Listeners are `Fn` (not `FnMut`) so `publish` can take `&self`. They are
/// `Send + Sync` so the bus can be shared across threads.
pub struct EventBus<E> {
    listeners: Vec<(SubscriptionId, Listener<E>)>,
    next_id: u64,
}

impl<E> EventBus<E> {
    /// Creates an empty bus.
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
            next_id: 0,
        }
    }

    /// Registers a listener and returns its id.
    pub fn subscribe<F>(&mut self, listener: F) -> SubscriptionId
    where
        F: Fn(&E) + Send + Sync + 'static,
    {
        let id = SubscriptionId(self.next_id);
        self.next_id += 1;
        let listener: Box<dyn Fn(&E) + Send + Sync> = Box::new(listener);
        self.listeners.push((id, listener));
        id
    }

    /// Removes a listener. Returns `false` if the id is not registered.
    pub fn unsubscribe(&mut self, id: SubscriptionId) -> bool {
        let before = self.listeners.len();
        self.listeners.retain(|(sid, _)| sid.0 != id.0);
        self.listeners.len() != before
    }

    /// Calls every listener with `event`, in subscription order.
    ///
    /// Returns the number of listeners that ran.
    pub fn publish(&self, event: &E) -> usize {
        for (_, listener) in &self.listeners {
            listener(event);
        }
        self.listeners.len()
    }

    /// Number of registered listeners.
    pub fn len(&self) -> usize {
        self.listeners.len()
    }

    /// `true` when no listeners are registered.
    pub fn is_empty(&self) -> bool {
        self.listeners.is_empty()
    }
}

impl<E> Default for EventBus<E> {
    fn default() -> Self {
        Self::new()
    }
}
