//! Part 7 · CSV sales ETL and ranked report.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

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
pub fn load_csv(text: &str) -> Result<Vec<RawRow>, csv::Error> {
    csv::Reader::from_reader(text.as_bytes())
        .deserialize::<RawRow>()
        .collect()
}

/// Exact decimal dollars to cents. No floats. See README Part 7 for accepted forms.
pub fn parse_cents(raw: &str) -> Option<u64> {
    let (whole, fraction) = match raw.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (raw, None),
    };
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let dollars: u64 = whole.parse().ok()?;
    let cents = match fraction {
        None => 0,
        Some(digits)
            if (1..=2).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit()) =>
        {
            let value: u64 = digits.parse().ok()?;
            if digits.len() == 1 { value * 10 } else { value }
        }
        Some(_) => return None,
    };
    dollars.checked_mul(100)?.checked_add(cents)
}

/// Trims, normalises and parses one row. Reports the first failing field.
pub fn parse_sale(row: &RawRow) -> Result<Sale, EtlError> {
    let invalid = |field: &'static str, value: &str| EtlError::InvalidField {
        field,
        value: value.to_owned(),
    };

    let units_raw = row.units.trim();
    let units = units_raw
        .parse::<u32>()
        .map_err(|_| invalid("units", units_raw))?;

    let price_raw = row.unit_price.trim();
    let unit_price_cents = parse_cents(price_raw).ok_or_else(|| invalid("unit_price", price_raw))?;

    let date_raw = row.sold_on.trim();
    let sold_on = date_raw
        .parse::<Date>()
        .map_err(|_| invalid("sold_on", date_raw))?;

    Ok(Sale {
        region: row.region.trim().to_lowercase(),
        product: row.product.trim().to_owned(),
        units,
        unit_price_cents,
        sold_on,
    })
}

/// Drops blank-region and zero-unit rows, parses the rest, and collects parse errors.
pub fn clean_sales(rows: impl IntoIterator<Item = RawRow>) -> (Vec<Sale>, Vec<RowError>) {
    let mut sales = Vec::new();
    let mut rejected = Vec::new();
    for (row_index, row) in rows.into_iter().enumerate() {
        if row.region.trim().is_empty() || row.units.trim().parse::<u32>() == Ok(0) {
            continue;
        }
        match parse_sale(&row) {
            Ok(sale) => sales.push(sale),
            Err(error) => rejected.push(RowError {
                row: row_index,
                error,
            }),
        }
    }
    (sales, rejected)
}

/// `units * unit_price_cents`, or `None` on overflow.
pub fn revenue_cents(sale: &Sale) -> Option<u64> {
    u64::from(sale.units).checked_mul(sale.unit_price_cents)
}

/// Revenue summed per key, in ascending key order.
pub fn totals_by(
    sales: &[Sale],
    key: impl Fn(&Sale) -> String,
) -> Result<BTreeMap<String, u64>, EtlError> {
    let mut totals: BTreeMap<String, u64> = BTreeMap::new();
    for sale in sales {
        let group = key(sale);
        let revenue = revenue_cents(sale).ok_or_else(|| EtlError::Overflow(group.clone()))?;
        let total = totals.entry(group.clone()).or_default();
        *total = total
            .checked_add(revenue)
            .ok_or(EtlError::Overflow(group))?;
    }
    Ok(totals)
}

/// Totals sorted by amount descending, then name ascending.
pub fn ranked(totals: &BTreeMap<String, u64>) -> Vec<(String, u64)> {
    let mut ranked: Vec<(String, u64)> = totals
        .iter()
        .map(|(name, total)| (name.clone(), *total))
        .collect();
    ranked.sort_by_key(|(name, total)| (Reverse(*total), name.clone()));
    ranked
}

/// Unique regions, uppercased, sorted.
pub fn distinct_regions(sales: &[Sale]) -> Vec<String> {
    sales
        .iter()
        .map(|sale| sale.region.to_uppercase())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// `1_234_567` with `"$"` -> `"$12,345.67"`.
pub fn format_cents(cents: u64, currency: &str) -> String {
    let digits = (cents / 100).to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    format!("{currency}{grouped}.{:02}", cents % 100)
}

/// Full pipeline: clean, total by region, rank, format.
pub fn build_report(
    rows: impl IntoIterator<Item = RawRow>,
    currency: &str,
) -> Result<Report, EtlError> {
    let (sales, rejected) = clean_sales(rows);
    let totals = totals_by(&sales, |sale| sale.region.clone())?;
    let lines = ranked(&totals)
        .into_iter()
        .map(|(region, total)| format!("{region}: {}", format_cents(total, currency)))
        .collect();
    Ok(Report { lines, rejected })
}
