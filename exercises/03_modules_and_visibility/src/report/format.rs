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
        let _ = self;
        todo!()
    }

    fn render(&self, table: &Table) -> String {
        let _ = table;
        todo!()
    }
}

impl Format for Json {
    fn name(&self) -> &'static str {
        let _ = self;
        todo!()
    }

    fn render(&self, table: &Table) -> String {
        let _ = table;
        todo!()
    }
}

impl Format for Text {
    fn name(&self) -> &'static str {
        let _ = self;
        todo!()
    }

    fn render(&self, table: &Table) -> String {
        let _ = table;
        todo!()
    }
}

/// Looks up a format by name, ignoring ASCII case. Returns `None` for unknown names.
#[must_use]
pub fn format_by_name(name: &str) -> Option<Box<dyn Format>> {
    let _ = name;
    todo!()
}
