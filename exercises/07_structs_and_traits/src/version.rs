//! Part 7: SemVer precedence with manual `Eq`/`Hash`/`Ord` and typed bumps.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

/// Errors from parsing or bumping a version.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VersionError {
    /// The text does not match the grammar.
    #[error("malformed version {0:?}")]
    Malformed(String),
    /// A numeric component has a leading zero.
    #[error("leading zero in {0:?}")]
    LeadingZero(String),
    /// A dot-separated identifier is empty.
    #[error("empty identifier in {0:?}")]
    EmptyIdentifier(String),
    /// An identifier contains a character outside `[0-9A-Za-z-]`.
    #[error("invalid character {0:?} in identifier")]
    InvalidCharacter(char),
    /// A numeric component does not fit in `u64`.
    #[error("numeric component overflows u64")]
    Overflow,
}

/// One dot-separated pre-release identifier.
///
/// Derived `Ord` puts `Numeric` below `Alphanumeric`, which is the SemVer rule.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PreIdentifier {
    /// An all-digit identifier, compared numerically.
    Numeric(u64),
    /// Any other identifier, compared as ASCII.
    Alphanumeric(String),
}

/// The part of a version to increment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// Increment `MAJOR`.
    Major,
    /// Increment `MINOR`.
    Minor,
    /// Increment `PATCH`.
    Patch,
}

/// A semantic version. Build metadata is ignored by equality, hashing and ordering.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    prerelease: Vec<PreIdentifier>,
    build: Vec<String>,
}

impl Version {
    /// A release version `major.minor.patch`.
    pub const fn new(_major: u64, _minor: u64, _patch: u64) -> Version {
        todo!()
    }

    /// Parses `[v]MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]`.
    pub fn parse(_text: &str) -> Result<Version, VersionError> {
        todo!()
    }

    /// Major component.
    pub fn major(&self) -> u64 {
        todo!()
    }

    /// Minor component.
    pub fn minor(&self) -> u64 {
        todo!()
    }

    /// Patch component.
    pub fn patch(&self) -> u64 {
        todo!()
    }

    /// Pre-release identifiers; empty for a release.
    pub fn prerelease(&self) -> &[PreIdentifier] {
        todo!()
    }

    /// Build metadata identifiers.
    pub fn build(&self) -> &[String] {
        todo!()
    }

    /// `true` when there are pre-release identifiers.
    pub fn is_prerelease(&self) -> bool {
        todo!()
    }

    /// Increments `part`, resetting lower parts and clearing pre-release and build.
    pub fn bump(&self, _part: Part) -> Result<Version, VersionError> {
        todo!()
    }
}

impl FromStr for Version {
    type Err = VersionError;

    fn from_str(_text: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

impl fmt::Display for Version {
    /// Canonical form, for example `1.2.3-rc.1+build.5`.
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl PartialEq for Version {
    fn eq(&self, _other: &Self) -> bool {
        todo!()
    }
}

impl Eq for Version {}

impl Hash for Version {
    fn hash<H: Hasher>(&self, _state: &mut H) {
        todo!()
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, _other: &Self) -> Ordering {
        todo!()
    }
}
