//! Part 6 — PATCH semantics: absent vs `null` vs a value, `Default` plus struct
//! update syntax for optional arguments, and serde integration.

use serde::{Deserialize, Deserializer};

/// One field of a PATCH body: not sent, explicitly `null`, or a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Patch<T> {
    /// The key was absent: leave the stored value alone.
    Missing,
    /// The key was `null`: clear the stored value.
    Null,
    /// The key carried a value: replace the stored value.
    Value(T),
}

/// The default is [`Patch::Missing`] for every `T`, with no `T: Default` bound.
impl<T> Default for Patch<T> {
    fn default() -> Self {
        Patch::Missing
    }
}

impl<T> Patch<T> {
    /// `true` only for [`Patch::Missing`].
    pub fn is_missing(&self) -> bool {
        matches!(self, Patch::Missing)
    }

    /// Transforms a present value; `Missing` and `Null` pass through unchanged.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Patch<U> {
        match self {
            Patch::Missing => Patch::Missing,
            Patch::Null => Patch::Null,
            Patch::Value(v) => Patch::Value(f(v)),
        }
    }

    /// Writes the patch into an optional slot: `Missing` leaves it, `Null` sets
    /// `None`, `Value(v)` sets `Some(v)`.
    pub fn apply(self, slot: &mut Option<T>) {
        match self {
            Patch::Missing => {}
            Patch::Null => *slot = None,
            Patch::Value(v) => *slot = Some(v),
        }
    }
}

/// Deserializes a present JSON value into `Patch`. Use with
/// `#[serde(default, deserialize_with = "...")]`: `default` supplies `Missing`
/// for absent keys, and this function only runs when the key is present.
fn deserialize<'de, D, T>(deserializer: D) -> Result<Patch<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    // `Option` maps JSON `null` to `None` and anything else to `Some`.
    Option::<T>::deserialize(deserializer).map(|opt| match opt {
        Some(v) => Patch::Value(v),
        None => Patch::Null,
    })
}

/// The stored user record a PATCH is applied to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub email: String,
    pub nickname: Option<String>,
    pub bio: Option<String>,
}

/// Why a PATCH was rejected. A rejected patch changes nothing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PatchError {
    #[error("email cannot be cleared")]
    EmailCleared,
    #[error("invalid email {0:?}")]
    InvalidEmail(String),
    #[error("nickname must be 1-32 characters after trimming")]
    InvalidNickname,
    #[error("bio must be at most 280 characters")]
    BioTooLong,
}

/// A partial update from a JSON body. Unknown keys are rejected.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserPatch {
    #[serde(default, deserialize_with = "deserialize")]
    pub email: Patch<String>,
    #[serde(default, deserialize_with = "deserialize")]
    pub nickname: Patch<String>,
    #[serde(default, deserialize_with = "deserialize")]
    pub bio: Patch<String>,
}

impl UserPatch {
    /// Validates every present field, then applies them. Validation happens
    /// before any mutation, so a rejected patch leaves `user` untouched.
    pub fn apply(self, user: &mut User) -> Result<(), PatchError> {
        let email = match self.email {
            Patch::Null => return Err(PatchError::EmailCleared),
            other => check(other, validate_email)?,
        };
        let nickname = check(self.nickname, validate_nickname)?;
        let bio = check(self.bio, validate_bio)?;

        if let Patch::Value(email) = email {
            user.email = email;
        }
        nickname.apply(&mut user.nickname);
        bio.apply(&mut user.bio);
        Ok(())
    }
}

/// Runs `validate` on a present value; `Missing` and `Null` pass through.
fn check<T, U>(
    patch: Patch<T>,
    validate: impl FnOnce(T) -> Result<U, PatchError>,
) -> Result<Patch<U>, PatchError> {
    match patch {
        Patch::Missing => Ok(Patch::Missing),
        Patch::Null => Ok(Patch::Null),
        Patch::Value(v) => validate(v).map(Patch::Value),
    }
}

/// Trims and lowercases, then requires one `@` with non-empty sides and a dot
/// in the domain. The error carries the raw input.
fn validate_email(raw: String) -> Result<String, PatchError> {
    let email = raw.trim().to_ascii_lowercase();
    let valid = match email.split_once('@') {
        Some((local, domain)) => {
            !local.is_empty()
                && !domain.is_empty()
                && domain.contains('.')
                && !domain.contains('@')
                && !email.bytes().any(|b| b.is_ascii_whitespace())
        }
        None => false,
    };
    if valid {
        Ok(email)
    } else {
        Err(PatchError::InvalidEmail(raw))
    }
}

/// Trims, then requires 1 to 32 characters.
fn validate_nickname(raw: String) -> Result<String, PatchError> {
    let trimmed = raw.trim();
    let len = trimmed.chars().count();
    if (1..=32).contains(&len) {
        Ok(trimmed.to_owned())
    } else {
        Err(PatchError::InvalidNickname)
    }
}

/// Requires at most 280 characters.
fn validate_bio(raw: String) -> Result<String, PatchError> {
    if raw.chars().count() <= 280 {
        Ok(raw)
    } else {
        Err(PatchError::BioTooLong)
    }
}
