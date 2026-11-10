//! Drop order, shadowing and scope, observed through a shared event log.

use std::cell::RefCell;
use std::rc::Rc;

/// Shared, append-only event log. Cloning shares the same log.
#[derive(Debug, Clone, Default)]
pub struct Sink {
    log: Rc<RefCell<Vec<String>>>,
}

impl Sink {
    /// Creates an empty sink.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an event.
    pub fn push(&self, event: impl Into<String>) {
        self.log.borrow_mut().push(event.into());
    }

    /// Snapshot of all events in order.
    #[must_use]
    pub fn events(&self) -> Vec<String> {
        self.log.borrow().clone()
    }
}

/// A named value that records `drop <label>` into its sink when dropped.
#[derive(Debug)]
pub struct Trace {
    sink: Sink,
    label: String,
}

impl Trace {
    /// Creates a traced value that logs to `sink` on drop.
    #[must_use]
    pub fn new(sink: &Sink, label: &str) -> Self {
        Self {
            sink: sink.clone(),
            label: label.to_owned(),
        }
    }

    /// The label given at construction.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns `code` as a `u16` status, pass-through. Exists to show a temporary in a tail expression.
    #[must_use]
    pub fn status(&self, code: u16) -> u16 {
        code
    }

    /// `Some(42)` if `present`, else `None`.
    #[must_use]
    pub fn maybe(&self, present: bool) -> Option<u32> {
        present.then_some(42)
    }
}

impl Drop for Trace {
    fn drop(&mut self) {
        self.sink.push(format!("drop {}", self.label));
    }
}

/// Three resources in declaration order. Fields drop in declaration order,
/// so `conn` drops first and `audit` last.
#[derive(Debug)]
pub struct Session {
    /// First declared field.
    pub conn: Trace,
    /// Second declared field.
    pub cache: Trace,
    /// Third declared field.
    pub audit: Trace,
}

impl Session {
    /// Labels of the three fields in declaration order.
    #[must_use]
    pub fn labels(&self) -> [&str; 3] {
        [self.conn.label(), self.cache.label(), self.audit.label()]
    }
}

/// Builds a [`Session`] labelled `conn:<n>`, `cache:<n>`, `audit:<n>`.
#[must_use]
pub fn open_session(sink: &Sink, n: &str) -> Session {
    Session {
        conn: Trace::new(sink, &format!("conn:{n}")),
        cache: Trace::new(sink, &format!("cache:{n}")),
        audit: Trace::new(sink, &format!("audit:{n}")),
    }
}

/// Demonstrates block scope. Logs `drop inner`, `drop c`, `drop a` and returns 3.
#[must_use]
pub fn scoped_locals(sink: &Sink) -> usize {
    let _a = Trace::new(sink, "a");
    {
        let _c = Trace::new(sink, "c");
        let _inner = Trace::new(sink, "inner");
    }
    3
}

/// Demonstrates shadowing. Shadowing does not drop the first binding;
/// both drop at end of scope in reverse declaration order. Returns 6 and logs
/// `end of scope`, `drop second`, `drop first`.
#[must_use]
pub fn shadowed(sink: &Sink) -> u32 {
    let _guard = Trace::new(sink, "first");
    let _guard = Trace::new(sink, "second");
    let value = 2_u32;
    let value = value * 3;
    sink.push("end of scope");
    value
}

/// Edition 2024: temporaries in a block's tail expression drop before the
/// block's locals. Returns 4 and logs `drop temp`, `drop local`.
#[must_use]
pub fn tail_temporary(sink: &Sink) -> u16 {
    let _local = Trace::new(sink, "local");
    Trace::new(sink, "temp").status(4)
}

/// `match` scrutinee temporaries live to the end of the `match`.
/// Code 200 takes the `ok` arm, anything else the `err` arm. Logs `arm: ok` or
/// `arm: err`, then `drop probe`.
pub fn match_scrutinee(sink: &Sink, code: u16) {
    match Trace::new(sink, "probe").status(code) {
        200 => sink.push("arm: ok"),
        _ => sink.push("arm: err"),
    }
}

/// Edition 2024: `if let` scrutinee temporaries drop before the `else` branch.
/// Present logs `then 7`, `drop guard` and returns 7. Absent logs `drop guard`,
/// `else` and returns 0.
#[must_use]
pub fn if_let_scrutinee(sink: &Sink, present: bool) -> u32 {
    if let Some(_value) = Trace::new(sink, "guard").maybe(present) {
        sink.push("then 7");
        7
    } else {
        sink.push("else");
        0
    }
}
