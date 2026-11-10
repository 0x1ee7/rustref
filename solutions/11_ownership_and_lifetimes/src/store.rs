//! Part 6 — copy-on-write document snapshots (`Rc::make_mut`) and shared-state
//! word counting across threads (`Mutex` vs channels).

use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::{mpsc, Arc, Mutex, PoisonError};
use std::thread;

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
    doc: Rc<Doc>,
    revision: u64,
}

impl Store {
    pub fn new(doc: Doc) -> Self {
        Store {
            doc: Rc::new(doc),
            revision: 0,
        }
    }

    pub fn doc(&self) -> &Doc {
        &self.doc
    }

    /// Number of completed [`Store::edit`] calls.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// O(1): shares the current document.
    pub fn snapshot(&self) -> Snapshot {
        Rc::clone(&self.doc)
    }

    /// Mutates the document with copy-on-write and bumps the revision.
    pub fn edit<R>(&mut self, f: impl FnOnce(&mut Doc) -> R) -> R {
        let result = f(Rc::make_mut(&mut self.doc));
        self.revision += 1;
        result
    }

    /// A deep copy that can cross threads (`Rc` cannot).
    pub fn shared_snapshot(&self) -> Arc<Doc> {
        Arc::new((*self.doc).clone())
    }
}

/// Lowercased maximal runs of alphanumeric characters across title and lines.
pub fn word_counts(doc: &Doc) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    let texts = std::iter::once(doc.title.as_str()).chain(doc.lines.iter().map(String::as_str));
    for text in texts {
        for word in text.split(|c: char| !c.is_alphanumeric()) {
            if !word.is_empty() {
                *counts.entry(word.to_lowercase()).or_insert(0) += 1;
            }
        }
    }
    counts
}

fn merge_into(total: &mut BTreeMap<String, usize>, local: BTreeMap<String, usize>) {
    for (word, n) in local {
        *total.entry(word).or_insert(0) += n;
    }
}

/// One scoped thread per document; all threads add into one `Mutex<BTreeMap>`.
///
/// Inside `thread::scope` the threads cannot outlive the borrows, so the map
/// needs no `Arc`; `Arc` is only required for `'static` threads.
pub fn count_words_shared(docs: &[Arc<Doc>]) -> BTreeMap<String, usize> {
    let total = Mutex::new(BTreeMap::new());
    let total_ref = &total;
    thread::scope(|s| {
        for doc in docs {
            s.spawn(move || {
                let local = word_counts(doc);
                let mut guard = total_ref.lock().unwrap_or_else(PoisonError::into_inner);
                merge_into(&mut guard, local);
            });
        }
    });
    total.into_inner().unwrap_or_else(PoisonError::into_inner)
}

/// One scoped thread per document; each sends its counts over an `mpsc` channel
/// and the calling thread merges them.
pub fn count_words_channel(docs: &[Arc<Doc>]) -> BTreeMap<String, usize> {
    let (tx, rx) = mpsc::channel::<BTreeMap<String, usize>>();
    let mut total = BTreeMap::new();
    thread::scope(|s| {
        for doc in docs {
            let tx = tx.clone();
            s.spawn(move || {
                // The receiver outlives the scope, so send cannot fail.
                let _ = tx.send(word_counts(doc));
            });
        }
        // Drop the original sender so `rx` ends once every worker finishes.
        drop(tx);
        for local in rx {
            merge_into(&mut total, local);
        }
    });
    total
}
