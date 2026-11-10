//! Sequential, prefixed identifier factories.

/// A stateful generator of identifiers. Each call yields the next id.
pub type IdFactory = Box<dyn FnMut() -> String + Send>;

/// Errors from [`id_factory`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdError {
    /// The prefix was empty.
    #[error("prefix must not be empty")]
    EmptyPrefix,
    /// The prefix contains a character outside `[A-Za-z0-9_-]`.
    #[error("prefix {0:?} contains an invalid character")]
    InvalidPrefix(String),
    /// The requested width was zero.
    #[error("width must be at least 1")]
    ZeroWidth,
}

/// Builds a factory producing `<prefix>-<n>` where `n` starts at 1 and is
/// zero-padded to `width`. Numbers wider than `width` are never truncated.
///
/// Validation order: empty prefix, invalid characters, zero width.
///
/// # Errors
///
/// Returns an [`IdError`] when the arguments are invalid.
pub fn id_factory(prefix: &str, width: usize) -> Result<IdFactory, IdError> {
    if prefix.is_empty() {
        return Err(IdError::EmptyPrefix);
    }
    if !prefix
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(IdError::InvalidPrefix(prefix.to_owned()));
    }
    if width == 0 {
        return Err(IdError::ZeroWidth);
    }
    let prefix = prefix.to_owned();
    let mut next = 0_u64;
    Ok(Box::new(move || {
        next += 1;
        format!("{prefix}-{next:0width$}")
    }))
}
