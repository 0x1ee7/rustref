/// Errors from [`Table::new`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TableError {
    /// The header list was empty.
    #[error("table must have at least one column")]
    NoColumns,
    /// A row's length differs from the number of columns.
    #[error("row {row} has {got} cells, expected {expected}")]
    RaggedRow {
        /// Zero-based index of the offending row.
        row: usize,
        /// Number of columns.
        expected: usize,
        /// Number of cells in the row.
        got: usize,
    },
}

/// A rectangular table of strings with a header row.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Table {
    /// Builds a table, checking that every row matches the header width.
    ///
    /// # Errors
    ///
    /// [`TableError::NoColumns`] for an empty header, or
    /// [`TableError::RaggedRow`] for the first row with the wrong width.
    pub fn new(headers: Vec<String>, rows: Vec<Vec<String>>) -> Result<Self, TableError> {
        let _ = (headers, rows);
        todo!()
    }

    /// Column headers.
    #[must_use]
    pub fn headers(&self) -> &[String] {
        todo!()
    }

    /// Data rows, in insertion order.
    #[must_use]
    pub fn rows(&self) -> &[Vec<String>] {
        todo!()
    }
}
