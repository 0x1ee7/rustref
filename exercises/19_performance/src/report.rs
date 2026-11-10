//! Fixed-width report rows rendered into reused or pre-sized buffers.

/// One aggregated row of a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row<'a> {
    pub name: &'a str,
    pub count: u32,
    pub total_micros: u64,
}

/// Appends one newline-terminated row to `out` without clearing it.
///
/// Format: `{name:<16} {count:>10} {total_micros:>20}\n`.
/// Allocates nothing when `out` already has enough capacity.
pub fn write_row(_out: &mut String, _row: &Row<'_>) -> std::fmt::Result {
    todo!()
}

/// Renders every row into a new `String`.
///
/// The output is pre-sized with an upper bound per row, so a non-empty report
/// costs exactly one allocation. An empty report allocates nothing.
pub fn render_report(_rows: &[Row<'_>]) -> String {
    todo!()
}
