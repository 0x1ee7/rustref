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

/// `true` when `address` has no whitespace and exactly one `@` with non-empty sides.
///
/// Shared by [`Email::parse`] and the notification channels.
pub(crate) fn is_plausible_email(address: &str) -> bool {
    if address.chars().any(char::is_whitespace) {
        return false;
    }
    matches!(
        address.split_once('@'),
        Some((local, domain)) if !local.is_empty() && !domain.is_empty() && !domain.contains('@')
    )
}

/// A normalised, validated email address.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Email {
    normalised: String,
}

impl Email {
    /// Trims, lowercases and validates an address.
    pub fn parse(raw: &str) -> Result<Email, EmployeeError> {
        let normalised = raw.trim().to_lowercase();
        if is_plausible_email(&normalised) {
            Ok(Email { normalised })
        } else {
            Err(EmployeeError::InvalidEmail(raw.to_owned()))
        }
    }

    /// The normalised address.
    pub fn as_str(&self) -> &str {
        &self.normalised
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.normalised)
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        &self.normalised
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
pub struct Employee {
    employee_id: u64,
    name: String,
    email: Email,
    department: String,
    salary: Money,
    skills: Vec<String>,
}

/// Rejects a blank value for `field`, returning the trimmed text.
fn required_text(field: &'static str, value: &str) -> Result<String, EmployeeError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(EmployeeError::Invalid {
            field,
            reason: "must not be blank".to_owned(),
        })
    } else {
        Ok(trimmed.to_owned())
    }
}

impl TryFrom<EmployeeRecord> for Employee {
    type Error = EmployeeError;

    fn try_from(record: EmployeeRecord) -> Result<Self, Self::Error> {
        let EmployeeRecord {
            employee_id,
            name,
            email,
            department,
            salary,
            currency,
            skills,
        } = record;
        if employee_id == 0 {
            return Err(EmployeeError::Invalid {
                field: "employee_id",
                reason: "must be greater than zero".to_owned(),
            });
        }
        let salary = format!("{} {}", salary.trim(), currency.trim()).parse::<Money>()?;
        if salary.minor() < 0 {
            return Err(EmployeeError::Invalid {
                field: "salary",
                reason: "must not be negative".to_owned(),
            });
        }
        // Skills: trimmed, lowercased, empties dropped, first occurrence kept.
        let mut normalised_skills: Vec<String> = Vec::with_capacity(skills.len());
        for skill in skills {
            let skill = skill.trim().to_lowercase();
            if !skill.is_empty() && !normalised_skills.contains(&skill) {
                normalised_skills.push(skill);
            }
        }
        Ok(Employee {
            employee_id,
            name: required_text("name", &name)?,
            email: Email::parse(&email)?,
            department: required_text("department", &department)?,
            salary,
            skills: normalised_skills,
        })
    }
}

impl FromStr for Employee {
    type Err = EmployeeError;

    /// Parses one CSV row: `employee_id,name,email,department,salary,currency,skills`.
    fn from_str(row: &str) -> Result<Self, Self::Err> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(row.as_bytes());
        let mut records = reader.records();
        let record = records
            .next()
            .transpose()?
            .ok_or(EmployeeError::ColumnCount(0))?;
        if record.len() != 7 {
            return Err(EmployeeError::ColumnCount(record.len()));
        }
        let employee_id = record[0]
            .trim()
            .parse::<u64>()
            .map_err(|_| EmployeeError::Invalid {
                field: "employee_id",
                reason: format!("{:?} is not a number", &record[0]),
            })?;
        Employee::try_from(EmployeeRecord {
            employee_id,
            name: record[1].to_owned(),
            email: record[2].to_owned(),
            department: record[3].to_owned(),
            salary: record[4].to_owned(),
            currency: record[5].to_owned(),
            skills: record[6].split(';').map(str::to_owned).collect(),
        })
    }
}

impl Employee {
    /// The employee number.
    pub fn employee_id(&self) -> u64 {
        self.employee_id
    }

    /// The trimmed name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The validated email.
    pub fn email(&self) -> &Email {
        &self.email
    }

    /// The trimmed department.
    pub fn department(&self) -> &str {
        &self.department
    }

    /// The salary as money.
    pub fn salary(&self) -> Money {
        self.salary
    }

    /// Normalised skills, in first-occurrence order.
    pub fn skills(&self) -> &[String] {
        &self.skills
    }
}
