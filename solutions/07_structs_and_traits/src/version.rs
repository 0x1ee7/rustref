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
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    prerelease: Vec<PreIdentifier>,
    build: Vec<String>,
}

impl Version {
    /// A release version `major.minor.patch`.
    pub const fn new(major: u64, minor: u64, patch: u64) -> Version {
        Version {
            major,
            minor,
            patch,
            prerelease: Vec::new(),
            build: Vec::new(),
        }
    }

    /// Parses `[v]MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]`.
    pub fn parse(text: &str) -> Result<Version, VersionError> {
        let body = text.strip_prefix('v').unwrap_or(text);
        let (rest, build) = match body.split_once('+') {
            Some((rest, build)) => (rest, Some(build)),
            None => (body, None),
        };
        let (core, prerelease) = match rest.split_once('-') {
            Some((core, prerelease)) => (core, Some(prerelease)),
            None => (rest, None),
        };
        let &[major, minor, patch] = core.split('.').collect::<Vec<_>>().as_slice() else {
            return Err(VersionError::Malformed(text.to_owned()));
        };
        Ok(Version {
            major: parse_core(major, text)?,
            minor: parse_core(minor, text)?,
            patch: parse_core(patch, text)?,
            prerelease: match prerelease {
                Some(section) => parse_prerelease(section, text)?,
                None => Vec::new(),
            },
            build: match build {
                Some(section) => parse_build(section, text)?,
                None => Vec::new(),
            },
        })
    }

    /// Major component.
    pub fn major(&self) -> u64 {
        self.major
    }

    /// Minor component.
    pub fn minor(&self) -> u64 {
        self.minor
    }

    /// Patch component.
    pub fn patch(&self) -> u64 {
        self.patch
    }

    /// Pre-release identifiers; empty for a release.
    pub fn prerelease(&self) -> &[PreIdentifier] {
        &self.prerelease
    }

    /// Build metadata identifiers.
    pub fn build(&self) -> &[String] {
        &self.build
    }

    /// `true` when there are pre-release identifiers.
    pub fn is_prerelease(&self) -> bool {
        !self.prerelease.is_empty()
    }

    /// Increments `part`, resetting lower parts and clearing pre-release and build.
    pub fn bump(&self, part: Part) -> Result<Version, VersionError> {
        let overflow = || VersionError::Overflow;
        let (major, minor, patch) = match part {
            Part::Major => (self.major.checked_add(1).ok_or_else(overflow)?, 0, 0),
            Part::Minor => (self.major, self.minor.checked_add(1).ok_or_else(overflow)?, 0),
            Part::Patch => (
                self.major,
                self.minor,
                self.patch.checked_add(1).ok_or_else(overflow)?,
            ),
        };
        Ok(Version::new(major, minor, patch))
    }
}

/// Parses a numeric core component: digits only, no leading zero, fits `u64`.
fn parse_core(number: &str, text: &str) -> Result<u64, VersionError> {
    if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(VersionError::Malformed(text.to_owned()));
    }
    if number.len() > 1 && number.starts_with('0') {
        return Err(VersionError::LeadingZero(text.to_owned()));
    }
    number.parse().map_err(|_| VersionError::Overflow)
}

/// Splits a dot-separated section and checks each identifier's shape.
fn validated_identifiers<'a>(section: &'a str, text: &str) -> Result<Vec<&'a str>, VersionError> {
    section
        .split('.')
        .map(|identifier| -> Result<&'a str, VersionError> {
            if identifier.is_empty() {
                return Err(VersionError::EmptyIdentifier(text.to_owned()));
            }
            if let Some(bad) = identifier
                .chars()
                .find(|c| !(c.is_ascii_alphanumeric() || *c == '-'))
            {
                return Err(VersionError::InvalidCharacter(bad));
            }
            Ok(identifier)
        })
        .collect()
}

/// Parses pre-release identifiers: numeric ones have no leading zeros.
fn parse_prerelease(section: &str, text: &str) -> Result<Vec<PreIdentifier>, VersionError> {
    validated_identifiers(section, text)?
        .into_iter()
        .map(|identifier| -> Result<PreIdentifier, VersionError> {
            if identifier.bytes().all(|byte| byte.is_ascii_digit()) {
                if identifier.len() > 1 && identifier.starts_with('0') {
                    return Err(VersionError::LeadingZero(text.to_owned()));
                }
                identifier
                    .parse()
                    .map(PreIdentifier::Numeric)
                    .map_err(|_| VersionError::Overflow)
            } else {
                Ok(PreIdentifier::Alphanumeric(identifier.to_owned()))
            }
        })
        .collect()
}

/// Parses build metadata: any valid identifiers, leading zeros allowed.
fn parse_build(section: &str, text: &str) -> Result<Vec<String>, VersionError> {
    Ok(validated_identifiers(section, text)?
        .into_iter()
        .map(str::to_owned)
        .collect())
}

impl FromStr for Version {
    type Err = VersionError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Version::parse(text)
    }
}

impl fmt::Display for Version {
    /// Canonical form, for example `1.2.3-rc.1+build.5`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        for (index, identifier) in self.prerelease.iter().enumerate() {
            f.write_str(if index == 0 { "-" } else { "." })?;
            match identifier {
                PreIdentifier::Numeric(number) => write!(f, "{number}")?,
                PreIdentifier::Alphanumeric(text) => f.write_str(text)?,
            }
        }
        for (index, identifier) in self.build.iter().enumerate() {
            f.write_str(if index == 0 { "+" } else { "." })?;
            f.write_str(identifier)?;
        }
        Ok(())
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major
            && self.minor == other.minor
            && self.patch == other.patch
            && self.prerelease == other.prerelease
    }
}

impl Eq for Version {}

impl Hash for Version {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Must hash exactly the fields `eq` compares.
        self.major.hash(state);
        self.minor.hash(state);
        self.patch.hash(state);
        self.prerelease.hash(state);
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let core = (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch));
        if core != Ordering::Equal {
            return core;
        }
        match (self.prerelease.is_empty(), other.prerelease.is_empty()) {
            // A release outranks every pre-release with the same core.
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => self.prerelease.cmp(&other.prerelease),
        }
    }
}
