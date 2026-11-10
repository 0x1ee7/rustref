use std::fmt::Write as _;

use super::table::Table;

/// Private supertrait. Being unnameable outside this crate is what seals [`Format`].
mod private {
    pub trait Sealed {}
}

/// A way to render a [`Table`] as text. Sealed: downstream crates cannot implement it.
pub trait Format: private::Sealed {
    /// Short lowercase name, e.g. `"csv"`.
    fn name(&self) -> &'static str;

    /// Renders the table.
    fn render(&self, table: &Table) -> String;
}

/// RFC-4180 style comma-separated values with `\n` line endings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Csv;

/// Compact JSON array of objects keyed by header.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Json;

/// Aligned plain-text table with a dashed separator.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Text;

impl private::Sealed for Csv {}
impl private::Sealed for Json {}
impl private::Sealed for Text {}

impl Format for Csv {
    fn name(&self) -> &'static str {
        "csv"
    }

    fn render(&self, table: &Table) -> String {
        let mut out = String::new();
        push_csv_line(&mut out, table.headers());
        for row in table.rows() {
            push_csv_line(&mut out, row);
        }
        out
    }
}

fn push_csv_line(out: &mut String, cells: &[String]) {
    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_csv_field(out, cell);
    }
    out.push('\n');
}

fn push_csv_field(out: &mut String, field: &str) {
    if field.contains([',', '"', '\n', '\r']) {
        out.push('"');
        for c in field.chars() {
            if c == '"' {
                out.push('"');
            }
            out.push(c);
        }
        out.push('"');
    } else {
        out.push_str(field);
    }
}

impl Format for Json {
    fn name(&self) -> &'static str {
        "json"
    }

    fn render(&self, table: &Table) -> String {
        let mut out = String::from("[");
        for (r, row) in table.rows().iter().enumerate() {
            if r > 0 {
                out.push(',');
            }
            out.push('{');
            for (i, (key, cell)) in table.headers().iter().zip(row).enumerate() {
                if i > 0 {
                    out.push(',');
                }
                push_json_string(&mut out, key);
                out.push(':');
                push_json_string(&mut out, cell);
            }
            out.push('}');
        }
        out.push(']');
        out
    }
}

fn push_json_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

impl Format for Text {
    fn name(&self) -> &'static str {
        "text"
    }

    fn render(&self, table: &Table) -> String {
        let headers = table.headers();
        let widths: Vec<usize> = (0..headers.len())
            .map(|col| {
                table
                    .rows()
                    .iter()
                    .map(|row| row[col].chars().count())
                    .chain(std::iter::once(headers[col].chars().count()))
                    .max()
                    .unwrap_or(0)
            })
            .collect();

        let mut out = String::new();
        push_text_line(&mut out, headers, &widths);
        let rule: Vec<String> = widths.iter().map(|&w| "-".repeat(w)).collect();
        out.push_str(&rule.join("-+-"));
        out.push('\n');
        for row in table.rows() {
            push_text_line(&mut out, row, &widths);
        }
        out
    }
}

fn push_text_line(out: &mut String, cells: &[String], widths: &[usize]) {
    let line = cells
        .iter()
        .zip(widths)
        .map(|(cell, &w)| format!("{cell:<w$}"))
        .collect::<Vec<_>>()
        .join(" | ");
    out.push_str(line.trim_end());
    out.push('\n');
}

/// Looks up a format by name, ignoring ASCII case. Returns `None` for unknown names.
#[must_use]
pub fn format_by_name(name: &str) -> Option<Box<dyn Format>> {
    match name.to_ascii_lowercase().as_str() {
        "csv" => Some(Box::new(Csv)),
        "json" => Some(Box::new(Json)),
        "text" => Some(Box::new(Text)),
        _ => None,
    }
}
