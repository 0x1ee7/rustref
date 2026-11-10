//! Part 7 · CSV sales ETL and ranked report.

use std::collections::BTreeMap;

use jiff::civil::Date;
use serde::Deserialize;
use thiserror::Error;

/// A CSV row exactly as read; every field is still text.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawRow {
    pub region: String,
    pub product: String,
    pub units: String,
    pub unit_price: String,
    pub sold_on: String,
}

/// A validated sale. Region is lowercased, price is in cents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sale {
    pub region: String,
    pub product: String,
    pub units: u32,
    pub unit_price_cents: u64,
    pub sold_on: Date,
}

/// Field-level and aggregation failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EtlError {
    #[error("invalid {field}: '{value}'")]
    InvalidField {
        field: &'static str,
        value: String,
    },
    #[error("revenue overflow in group '{0}'")]
    Overflow(String),
}

/// A row that failed to parse. `row` is the zero-based index in the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowError {
    pub row: usize,
    pub error: EtlError,
}

/// Output of `build_report`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub lines: Vec<String>,
    pub rejected: Vec<RowError>,
}

/// Parses CSV text with a header row into `RawRow`s.
pub fn load_csv(_text: &str) -> Result<Vec<RawRow>, csv::Error> {
    todo!()
}

/// Exact decimal dollars to cents. No floats. See README Part 7 for accepted forms.
pub fn parse_cents(_raw: &str) -> Option<u64> {
    todo!()
}

/// Trims, normalises and parses one row. Reports the first failing field.
pub fn parse_sale(_row: &RawRow) -> Result<Sale, EtlError> {
    todo!()
}

/// Drops blank-region and zero-unit rows, parses the rest, and collects parse errors.
pub fn clean_sales(
    _rows: impl IntoIterator<Item = RawRow>,
) -> (Vec<Sale>, Vec<RowError>) {
    todo!()
}

/// `units * unit_price_cents`, or `None` on overflow.
pub fn revenue_cents(_sale: &Sale) -> Option<u64> {
    todo!()
}

/// Revenue summed per key, in ascending key order.
pub fn totals_by(
    _sales: &[Sale],
    _key: impl Fn(&Sale) -> String,
) -> Result<BTreeMap<String, u64>, EtlError> {
    todo!()
}

/// Totals sorted by amount descending, then name ascending.
pub fn ranked(_totals: &BTreeMap<String, u64>) -> Vec<(String, u64)> {
    todo!()
}

/// Unique regions, uppercased, sorted.
pub fn distinct_regions(_sales: &[Sale]) -> Vec<String> {
    todo!()
}

/// `1_234_567` with `"$"` -> `"$12,345.67"`.
pub fn format_cents(_cents: u64, _currency: &str) -> String {
    todo!()
}

/// Full pipeline: clean, total by region, rank, format.
pub fn build_report(
    _rows: impl IntoIterator<Item = RawRow>,
    _currency: &str,
) -> Result<Report, EtlError> {
    todo!()
}
