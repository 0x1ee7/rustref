//! Fixed-width report rows rendered into reused or pre-sized buffers.

use std::fmt::Write;

/// One aggregated row of a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row<'a> {
    pub name: &'a str,
    pub count: u32,
    pub total_micros: u64,
}

/// Upper bound on the rendered width of one row: names are at least 16 wide,
/// `u32` needs at most 10 digits and `u64` at most 20.
fn row_capacity(row: &Row<'_>) -> usize {
    row.name.len().max(16) + 1 + 10 + 1 + 20 + 1
}

/// Appends one newline-terminated row to `out` without clearing it.
pub fn write_row(out: &mut String, row: &Row<'_>) -> std::fmt::Result {
    writeln!(out, "{:<16} {:>10} {:>20}", row.name, row.count, row.total_micros)
}

/// Renders every row into a new `String`, sized up front.
pub fn render_report(rows: &[Row<'_>]) -> String {
    let capacity: usize = rows.iter().map(row_capacity).sum();
    let mut out = String::with_capacity(capacity);
    for row in rows {
        write_row(&mut out, row).expect("writing to a String cannot fail");
    }
    out
}
