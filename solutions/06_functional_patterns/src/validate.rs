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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}: {}", self.path, self.message)
        }
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
pub fn combine<A, B>(a: Checked<A>, b: Checked<B>) -> Checked<(A, B)> {
    match (a, b) {
        (Ok(a), Ok(b)) => Ok((a, b)),
        (Err(mut left), Err(right)) => {
            left.extend(right);
            Err(left)
        }
        (Err(errors), Ok(_)) | (Ok(_), Err(errors)) => Err(errors),
    }
}

/// Keeps every error from every item, in order.
pub fn collect_all<T>(items: impl IntoIterator<Item = Checked<T>>) -> Checked<Vec<T>> {
    let mut values = Vec::new();
    let mut errors = Vec::new();
    for item in items {
        match item {
            Ok(value) => values.push(value),
            Err(found) => errors.extend(found),
        }
    }
    if errors.is_empty() {
        Ok(values)
    } else {
        Err(errors)
    }
}

/// The payload must be a JSON object. Root path is `""`.
pub fn require_object(value: &Value) -> Checked<&Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| one("", format!("expected object, got {}", json_type(value))))
}

/// Looks up a required field. Missing -> `path == name`, `message == "missing"`.
pub fn field<'a>(object: &'a Map<String, Value>, name: &str) -> Checked<&'a Value> {
    object
        .get(name)
        .ok_or_else(|| one(name, "missing".to_owned()))
}

/// `value` must be a JSON string.
pub fn expect_string<'a>(path: &str, value: &'a Value) -> Checked<&'a str> {
    value
        .as_str()
        .ok_or_else(|| one(path, format!("expected string, got {}", json_type(value))))
}

/// `value` must be a JSON integer.
pub fn expect_integer(path: &str, value: &Value) -> Checked<i64> {
    value
        .as_i64()
        .ok_or_else(|| one(path, format!("expected integer, got {}", json_type(value))))
}

/// Full signup validation. Reports every error, in the rule order from README Part 6.
pub fn validate_signup(payload: &Value) -> Checked<Signup> {
    let object = require_object(payload)?;
    let username = field(object, "username").and_then(check_username);
    let age = field(object, "age").and_then(check_age);
    let email = field(object, "email").and_then(check_email);
    let tags = check_tags(object.get("tags"));

    combine(combine(username, age), combine(email, tags)).map(
        |((username, age), (email, tags))| Signup {
            username,
            age,
            email,
            tags,
        },
    )
}

fn check_username(value: &Value) -> Checked<String> {
    let username = expect_string("username", value)?;
    let long_enough = username.chars().count() >= 3;
    let charset_ok = username
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    combine(
        rule("username", long_enough, "length must be at least 3"),
        rule(
            "username",
            charset_ok,
            "may only contain a-z, 0-9 and _",
        ),
    )
    .map(|_| username.to_owned())
}

fn check_age(value: &Value) -> Checked<u8> {
    let age = expect_integer("age", value)?;
    match u8::try_from(age) {
        Ok(age) if (13..=120).contains(&age) => Ok(age),
        _ => Err(one("age", "must be between 13 and 120".to_owned())),
    }
}

fn check_email(value: &Value) -> Checked<String> {
    let email = expect_string("email", value)?;
    if is_plausible_email(email) {
        Ok(email.to_owned())
    } else {
        Err(one("email", "must look like name@domain.tld".to_owned()))
    }
}

fn check_tags(value: Option<&Value>) -> Checked<Vec<String>> {
    match value {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => collect_all(
            items
                .iter()
                .enumerate()
                .map(|(index, item)| check_tag(index, item)),
        ),
        Some(other) => Err(one(
            "tags",
            format!("expected array, got {}", json_type(other)),
        )),
    }
}

fn check_tag(index: usize, value: &Value) -> Checked<String> {
    let path = format!("tags[{index}]");
    let tag = expect_string(&path, value)?;
    rule(&path, !tag.is_empty(), "must not be empty")?;
    Ok(tag.to_owned())
}

/// Shape check: one non-empty local part, one `@`, and a dotted domain
/// that neither starts nor ends with a dot. No whitespace anywhere.
fn is_plausible_email(email: &str) -> bool {
    if email.chars().any(char::is_whitespace) {
        return false;
    }
    match email.split_once('@') {
        Some((local, domain)) => {
            !local.is_empty()
                && !domain.contains('@')
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        }
        None => false,
    }
}

fn rule(path: &str, holds: bool, message: &str) -> Checked<()> {
    if holds {
        Ok(())
    } else {
        Err(one(path, message.to_owned()))
    }
}

fn one(path: &str, message: String) -> Vec<ValidationError> {
    vec![ValidationError {
        path: path.to_owned(),
        message,
    }]
}

fn json_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
