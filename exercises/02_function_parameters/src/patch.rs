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
        todo!()
    }
}

impl<T> Patch<T> {
    /// `true` only for [`Patch::Missing`].
    pub fn is_missing(&self) -> bool {
        todo!()
    }

    /// Transforms a present value; `Missing` and `Null` pass through unchanged.
    pub fn map<U>(self, _f: impl FnOnce(T) -> U) -> Patch<U> {
        todo!()
    }

    /// Writes the patch into an optional slot: `Missing` leaves it, `Null` sets
    /// `None`, `Value(v)` sets `Some(v)`.
    pub fn apply(self, _slot: &mut Option<T>) {
        todo!()
    }
}

/// Deserializes a present JSON value into `Patch`. Use with
/// `#[serde(default, deserialize_with = "...")]`: `default` supplies `Missing`
/// for absent keys, and this function only runs when the key is present.
fn deserialize<'de, D, T>(_deserializer: D) -> Result<Patch<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    todo!()
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
    pub fn apply(self, _user: &mut User) -> Result<(), PatchError> {
        todo!()
    }
}
