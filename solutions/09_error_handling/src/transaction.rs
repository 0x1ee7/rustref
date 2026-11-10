//! Part 4: an RAII transaction over a `HashMap`, with savepoints.

use std::collections::HashMap;

/// Errors from [`Transaction`] operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TxError {
    /// `get` or `delete` on a key that is absent, or was deleted in this transaction.
    #[error("key '{0}' not found")]
    KeyNotFound(String),
    /// Returned from a savepoint closure: discard that savepoint's writes and continue.
    #[error("savepoint rolled back")]
    SavepointRollback,
}

/// A transaction over `&mut HashMap<String, V>`.
///
/// Writes apply to the store immediately; an undo log restores it. [`commit`]
/// discards the log. Dropping without committing rolls back.
///
/// [`commit`]: Transaction::commit
pub struct Transaction<'s, V> {
    store: &'s mut HashMap<String, V>,
    /// `(key, value before the write)`, oldest first. `None` means the key was absent.
    undo: Vec<(String, Option<V>)>,
}

impl<'s, V> Transaction<'s, V> {
    /// Starts a transaction over `store`.
    pub fn begin(store: &'s mut HashMap<String, V>) -> Self {
        Self {
            store,
            undo: Vec::new(),
        }
    }

    /// Reads a key, seeing this transaction's writes.
    pub fn get(&self, key: &str) -> Result<&V, TxError> {
        self.store
            .get(key)
            .ok_or_else(|| TxError::KeyNotFound(key.to_owned()))
    }

    /// Inserts or overwrites a key.
    pub fn set(&mut self, key: impl Into<String>, value: V) {
        let key = key.into();
        let previous = self.store.insert(key.clone(), value);
        self.undo.push((key, previous));
    }

    /// Removes a key.
    pub fn delete(&mut self, key: &str) -> Result<(), TxError> {
        let (owned_key, old_value) = self
            .store
            .remove_entry(key)
            .ok_or_else(|| TxError::KeyNotFound(key.to_owned()))?;
        self.undo.push((owned_key, Some(old_value)));
        Ok(())
    }

    /// Runs `f` as a savepoint.
    ///
    /// `Ok(v)` keeps the writes and returns `Ok(Some(v))`. `Err(SavepointRollback)`
    /// discards them and returns `Ok(None)`. Any other error discards them and
    /// propagates; the transaction stays usable.
    pub fn savepoint<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, TxError>,
    ) -> Result<Option<T>, TxError> {
        let mark = self.undo.len();
        match f(self) {
            Ok(value) => Ok(Some(value)),
            Err(TxError::SavepointRollback) => {
                self.rollback_to(mark);
                Ok(None)
            }
            Err(err) => {
                self.rollback_to(mark);
                Err(err)
            }
        }
    }

    /// Keeps every write and ends the transaction.
    pub fn commit(mut self) {
        self.undo.clear();
    }

    /// Undoes log entries past `mark`, newest first.
    fn rollback_to(&mut self, mark: usize) {
        for (key, previous) in self.undo.split_off(mark).into_iter().rev() {
            match previous {
                Some(value) => {
                    self.store.insert(key, value);
                }
                None => {
                    self.store.remove(&key);
                }
            }
        }
    }
}

impl<V> Drop for Transaction<'_, V> {
    fn drop(&mut self) {
        self.rollback_to(0);
    }
}
