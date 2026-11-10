//! Part 4: an RAII transaction over a `HashMap`, with savepoints.

use std::collections::HashMap;
use std::marker::PhantomData;

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
    // private: the borrowed store and the undo log
    _marker: PhantomData<(&'s mut HashMap<String, V>, V)>,
}

impl<'s, V> Transaction<'s, V> {
    /// Starts a transaction over `_store`.
    pub fn begin(_store: &'s mut HashMap<String, V>) -> Self {
        todo!()
    }

    /// Reads a key, seeing this transaction's writes.
    pub fn get(&self, _key: &str) -> Result<&V, TxError> {
        todo!()
    }

    /// Inserts or overwrites a key.
    pub fn set(&mut self, _key: impl Into<String>, _value: V) {
        todo!()
    }

    /// Removes a key.
    pub fn delete(&mut self, _key: &str) -> Result<(), TxError> {
        todo!()
    }

    /// Runs `_f` as a savepoint.
    ///
    /// `Ok(v)` keeps the writes and returns `Ok(Some(v))`. `Err(SavepointRollback)`
    /// discards them and returns `Ok(None)`. Any other error discards them and
    /// propagates; the transaction stays usable.
    pub fn savepoint<T>(
        &mut self,
        _f: impl FnOnce(&mut Self) -> Result<T, TxError>,
    ) -> Result<Option<T>, TxError> {
        todo!()
    }

    /// Keeps every write and ends the transaction.
    pub fn commit(self) {
        todo!()
    }
}

// Engineer: roll back everything not committed. The stub is a no-op so that
// unwinding through an unfinished transaction does not abort the test binary.
impl<V> Drop for Transaction<'_, V> {
    fn drop(&mut self) {}
}
