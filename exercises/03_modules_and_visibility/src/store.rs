//! A key-value store with transactional, RAII-guarded writes.

use std::marker::PhantomData;

/// A committed key-value store. Keeps counts of finished transactions.
#[derive(Debug, Default)]
pub struct Store {}

impl Store {
    /// Creates an empty store.
    #[must_use]
    pub fn new() -> Self {
        todo!()
    }

    /// Looks up a committed value.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        let _ = key;
        todo!()
    }

    /// Number of committed keys.
    #[must_use]
    pub fn len(&self) -> usize {
        todo!()
    }

    /// `true` when no keys are committed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        todo!()
    }

    /// Number of committed transactions.
    #[must_use]
    pub fn commit_count(&self) -> usize {
        todo!()
    }

    /// Number of transactions dropped without commit.
    #[must_use]
    pub fn rollback_count(&self) -> usize {
        todo!()
    }

    /// Starts a transaction that exclusively borrows the store.
    pub fn begin(&mut self) -> Transaction<'_> {
        todo!()
    }
}

/// Staged writes against a [`Store`]. Dropping without [`Transaction::commit`]
/// discards the staged writes and counts as a rollback.
#[derive(Debug)]
pub struct Transaction<'a> {
    _store: PhantomData<&'a mut Store>,
}

impl Transaction<'_> {
    /// Stages a write. A later write to the same key wins.
    pub fn put(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let _ = (key.into(), value.into());
        todo!()
    }

    /// Stages a delete. Staged deletes shadow committed values.
    pub fn delete(&mut self, key: &str) {
        let _ = key;
        todo!()
    }

    /// Reads through the staged view: staged writes and deletes take
    /// priority over committed data.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        let _ = key;
        todo!()
    }

    /// Applies staged writes to the store. Returns the number of staged keys
    /// (puts and deletes, one per distinct key).
    pub fn commit(self) -> usize {
        todo!()
    }

    /// Explicitly discards the transaction. Equivalent to dropping it.
    pub fn rollback(self) {
        todo!()
    }
}
