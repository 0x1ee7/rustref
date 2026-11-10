//! A key-value store with transactional, RAII-guarded writes.

use std::collections::{BTreeMap, HashMap};

/// A committed key-value store. Keeps counts of finished transactions.
#[derive(Debug, Default)]
pub struct Store {
    data: HashMap<String, String>,
    commits: usize,
    rollbacks: usize,
}

impl Store {
    /// Creates an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Looks up a committed value.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(String::as_str)
    }

    /// Number of committed keys.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// `true` when no keys are committed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Number of committed transactions.
    #[must_use]
    pub fn commit_count(&self) -> usize {
        self.commits
    }

    /// Number of transactions dropped without commit.
    #[must_use]
    pub fn rollback_count(&self) -> usize {
        self.rollbacks
    }

    /// Starts a transaction that exclusively borrows the store.
    pub fn begin(&mut self) -> Transaction<'_> {
        Transaction {
            store: self,
            staged: BTreeMap::new(),
            finished: false,
        }
    }
}

/// Staged writes against a [`Store`]. Dropping without [`Transaction::commit`]
/// discards the staged writes and counts as a rollback.
#[derive(Debug)]
pub struct Transaction<'a> {
    store: &'a mut Store,
    /// `Some(value)` for a staged put, `None` for a staged delete.
    staged: BTreeMap<String, Option<String>>,
    finished: bool,
}

impl Transaction<'_> {
    /// Stages a write. A later write to the same key wins.
    pub fn put(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.staged.insert(key.into(), Some(value.into()));
    }

    /// Stages a delete. Staged deletes shadow committed values.
    pub fn delete(&mut self, key: &str) {
        self.staged.insert(key.to_owned(), None);
    }

    /// Reads through the staged view: staged writes and deletes take
    /// priority over committed data.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        match self.staged.get(key) {
            Some(Some(value)) => Some(value.as_str()),
            Some(None) => None,
            None => self.store.get(key),
        }
    }

    /// Applies staged writes to the store. Returns the number of staged keys
    /// (puts and deletes, one per distinct key).
    pub fn commit(mut self) -> usize {
        let staged = std::mem::take(&mut self.staged);
        let count = staged.len();
        for (key, value) in staged {
            match value {
                Some(value) => {
                    self.store.data.insert(key, value);
                }
                None => {
                    self.store.data.remove(&key);
                }
            }
        }
        self.store.commits += 1;
        self.finished = true;
        count
    }

    /// Explicitly discards the transaction. Equivalent to dropping it.
    pub fn rollback(self) {}
}

impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        if !self.finished {
            self.store.rollbacks += 1;
        }
    }
}
