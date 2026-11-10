//! Part 6 · accumulate-all-errors validation of untyped JSON.

use std::fmt;

use serde_json::{Map, Value};

/// One validation problem. `path` is empty for root-level errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub path: String,
    pub message: String,
}

/// Renders as `path: message`, or just `message` when `path` is empty.
impl fmt::Display for ValidationError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

/// `Ok` with the value, or every error found (never just the first).
pub type Checked<T> = Result<T, Vec<ValidationError>>;

/// Validated signup payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signup {
    pub username: String,
    pub age: u8,
    pub email: String,
    pub tags: Vec<String>,
}

/// Evaluates both sides; concatenates their errors in order if either fails.
pub fn combine<A, B>(_a: Checked<A>, _b: Checked<B>) -> Checked<(A, B)> {
    todo!()
}

/// Keeps every error from every item, in order.
pub fn collect_all<T>(_items: impl IntoIterator<Item = Checked<T>>) -> Checked<Vec<T>> {
    todo!()
}

/// The payload must be a JSON object. Root path is `""`.
pub fn require_object(_value: &Value) -> Checked<&Map<String, Value>> {
    todo!()
}

/// Looks up a required field. Missing -> `path == name`, `message == "missing"`.
pub fn field<'a>(_object: &'a Map<String, Value>, _name: &str) -> Checked<&'a Value> {
    todo!()
}

/// `value` must be a JSON string.
pub fn expect_string<'a>(_path: &str, _value: &'a Value) -> Checked<&'a str> {
    todo!()
}

/// `value` must be a JSON integer.
pub fn expect_integer(_path: &str, _value: &Value) -> Checked<i64> {
    todo!()
}

/// Full signup validation. Reports every error, in the rule order from README Part 6.
pub fn validate_signup(_payload: &Value) -> Checked<Signup> {
    todo!()
}
