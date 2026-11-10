//! Part 5: validated `Employee` built through `TryFrom`, `FromStr` and serde.

use std::fmt;
use std::str::FromStr;

use serde::Deserialize;

use crate::money::{Money, MoneyError};

/// Errors from employee and email validation.
#[derive(Debug, thiserror::Error)]
pub enum EmployeeError {
    /// The email failed validation.
    #[error("invalid email {0:?}")]
    InvalidEmail(String),
    /// A field failed validation.
    #[error("invalid field `{field}`: {reason}")]
    Invalid { field: &'static str, reason: String },
    /// A CSV row did not have seven columns.
    #[error("expected 7 columns, found {0}")]
    ColumnCount(usize),
    /// The CSV row could not be parsed.
    #[error("CSV parse error: {0}")]
    Csv(#[from] csv::Error),
    /// A money value could not be built.
    #[error(transparent)]
    Money(#[from] MoneyError),
}

/// A normalised, validated email address.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub struct Email {
    normalised: String,
}

impl Email {
    /// Trims, lowercases and validates an address.
    pub fn parse(_raw: &str) -> Result<Email, EmployeeError> {
        todo!()
    }

    /// The normalised address.
    pub fn as_str(&self) -> &str {
        todo!()
    }
}

impl fmt::Display for Email {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        todo!()
    }
}

/// A raw, unvalidated employee record. Converted with `TryFrom`; never trusted directly.
#[derive(Clone, Debug, Deserialize)]
pub struct EmployeeRecord {
    /// Employee number; must be greater than zero.
    pub employee_id: u64,
    /// Full name.
    pub name: String,
    /// Email address.
    pub email: String,
    /// Department name.
    pub department: String,
    /// Decimal salary text, for example `"85000.00"`.
    pub salary: String,
    /// Currency code of the salary, for example `"USD"`.
    pub currency: String,
    /// Skills; defaults to empty when absent.
    #[serde(default)]
    pub skills: Vec<String>,
}

/// A validated employee. Every construction path enforces the same rules.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(try_from = "EmployeeRecord")]
#[allow(dead_code)]
pub struct Employee {
    employee_id: u64,
    name: String,
    email: Email,
    department: String,
    salary: Money,
    skills: Vec<String>,
}

impl TryFrom<EmployeeRecord> for Employee {
    type Error = EmployeeError;

    fn try_from(_record: EmployeeRecord) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl FromStr for Employee {
    type Err = EmployeeError;

    /// Parses one CSV row: `employee_id,name,email,department,salary,currency,skills`.
    fn from_str(_row: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

impl Employee {
    /// The employee number.
    pub fn employee_id(&self) -> u64 {
        todo!()
    }

    /// The trimmed name.
    pub fn name(&self) -> &str {
        todo!()
    }

    /// The validated email.
    pub fn email(&self) -> &Email {
        todo!()
    }

    /// The trimmed department.
    pub fn department(&self) -> &str {
        todo!()
    }

    /// The salary as money.
    pub fn salary(&self) -> Money {
        todo!()
    }

    /// Normalised skills, in first-occurrence order.
    pub fn skills(&self) -> &[String] {
        todo!()
    }
}
