//! Part 6 — copy-on-write document snapshots (`Rc::make_mut`) and shared-state
//! word counting across threads (`Mutex<_>` vs channels).

use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

/// A document: a title and body lines.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Doc {
    pub title: String,
    pub lines: Vec<String>,
}

/// A cheap, immutable, single-threaded view of the store's current document.
pub type Snapshot = Rc<Doc>;

/// A single-threaded versioned document store.
#[derive(Debug, Clone)]
pub struct Store {
    // engineer: current Rc<Doc>, revision counter
}

impl Store {
    pub fn new(_doc: Doc) -> Self {
        todo!()
    }

    pub fn doc(&self) -> &Doc {
        todo!()
    }

    /// Number of completed [`Store::edit`] calls.
    pub fn revision(&self) -> u64 {
        todo!()
    }

    /// O(1): shares the current document.
    pub fn snapshot(&self) -> Snapshot {
        todo!()
    }

    /// Mutates the document with copy-on-write and bumps the revision.
    pub fn edit<R>(&mut self, _f: impl FnOnce(&mut Doc) -> R) -> R {
        todo!()
    }

    /// A deep copy that can cross threads (`Rc` cannot).
    pub fn shared_snapshot(&self) -> Arc<Doc> {
        todo!()
    }
}

/// Lowercased maximal runs of alphanumeric characters across title and lines.
pub fn word_counts(_doc: &Doc) -> BTreeMap<String, usize> {
    todo!()
}

/// One scoped thread per document; all threads add into one `Mutex<BTreeMap>` they borrow.
pub fn count_words_shared(_docs: &[Arc<Doc>]) -> BTreeMap<String, usize> {
    todo!()
}

/// One scoped thread per document; each sends its counts over an `mpsc` channel
/// and the calling thread merges them.
pub fn count_words_channel(_docs: &[Arc<Doc>]) -> BTreeMap<String, usize> {
    todo!()
}
