//! Manifest diffing and a path-safe apply step.

use std::fs::{self, File};
use std::io;
use std::path::{Component, Path};

use crate::atomic::write_atomic;
use crate::error::{Error, Result};
use crate::scan::Manifest;

/// Changes needed to turn one manifest into another. Borrows the keys from the
/// manifests it was computed from. All lists are sorted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManifestDiff<'a> {
    /// In the new manifest only.
    pub added: Vec<&'a str>,
    /// In the old manifest only.
    pub removed: Vec<&'a str>,
    /// In both, but size or SHA-256 differs. An mtime-only change is not a modification.
    pub modified: Vec<&'a str>,
}

impl ManifestDiff<'_> {
    /// `true` when there is nothing to do.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
    }
}

/// Computes what changed from `old` to `new`.
#[must_use]
pub fn diff_manifests<'a>(old: &'a Manifest, new: &'a Manifest) -> ManifestDiff<'a> {
    let mut diff = ManifestDiff::default();
    for (path, new_entry) in new {
        match old.get(path) {
            None => diff.added.push(path.as_str()),
            Some(old_entry)
                if old_entry.size != new_entry.size || old_entry.sha256 != new_entry.sha256 =>
            {
                diff.modified.push(path.as_str());
            }
            Some(_) => {}
        }
    }
    diff.removed = old
        .keys()
        .filter(|path| !new.contains_key(path.as_str()))
        .map(String::as_str)
        .collect();
    diff
}

/// Checks that a manifest key is a safe relative POSIX path below some root.
///
/// Rejects: empty keys or segments (`""`, `a//b`, `a/`), `.`/`..` segments,
/// backslashes, absolute paths, and Windows drive or UNC prefixes.
///
/// # Errors
/// [`Error::UnsafePath`] describing the first problem found.
pub fn validate_relative_path(key: &str) -> Result<()> {
    let reject = |reason: &'static str| {
        Err(Error::UnsafePath {
            path: key.to_owned(),
            reason,
        })
    };
    if key.is_empty() {
        return reject("empty path");
    }
    if key.contains('\\') {
        return reject("backslash separators are not allowed");
    }
    if key.split('/').any(str::is_empty) {
        return reject("empty path segment");
    }
    for segment in key.split('/') {
        match segment {
            "." => return reject("current-directory segment `.`"),
            ".." => return reject("parent-directory segment `..`"),
            _ => {}
        }
    }
    if Path::new(key)
        .components()
        .any(|c| matches!(c, Component::Prefix(_) | Component::RootDir))
    {
        return reject("absolute path");
    }
    Ok(())
}

/// Result of [`apply_diff`]. Borrows the keys from the diff.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApplyResult<'a> {
    /// Keys copied from source to destination (added and modified).
    pub copied: Vec<&'a str>,
    /// Keys that existed under the destination and were removed.
    pub deleted: Vec<&'a str>,
}

/// Applies `diff` by copying from `source` into `dest`.
///
/// Every key is validated with [`validate_relative_path`] before the first disk
/// change. Copies go through [`write_atomic`] and keep the source mtime. Removed
/// files that are already absent are not reported. After a removal, empty parent
/// directories are pruned up to, but not including, `dest`.
///
/// # Errors
/// [`Error::UnsafePath`] before touching disk, or [`Error::Io`] on filesystem failure.
pub fn apply_diff<'a>(
    diff: &ManifestDiff<'a>,
    source: &Path,
    dest: &Path,
) -> Result<ApplyResult<'a>> {
    for &key in diff.added.iter().chain(&diff.modified).chain(&diff.removed) {
        validate_relative_path(key)?;
    }

    let mut result = ApplyResult::default();
    for &key in diff.added.iter().chain(&diff.modified) {
        copy_file(key, source, dest)?;
        result.copied.push(key);
    }
    for &key in &diff.removed {
        if remove_file(key, dest)? {
            result.deleted.push(key);
        }
    }
    Ok(result)
}

fn copy_file(key: &str, source: &Path, dest: &Path) -> Result<()> {
    let src = source.join(key);
    let dst = dest.join(key);
    let io_err = |path: &Path| {
        let path = path.to_path_buf();
        move |source| Error::Io { path, source }
    };

    let mtime = fs::metadata(&src)
        .and_then(|m| m.modified())
        .map_err(io_err(&src))?;
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(io_err(parent))?;
    }
    let mut input = File::open(&src).map_err(io_err(&src))?;
    write_atomic(&dst, |out| io::copy(&mut input, out).map(|_| ()))?;
    File::options()
        .write(true)
        .open(&dst)
        .and_then(|file| file.set_modified(mtime))
        .map_err(io_err(&dst))?;
    Ok(())
}

/// Returns `true` if a file was removed, `false` if it was already absent.
fn remove_file(key: &str, dest: &Path) -> Result<bool> {
    let target = dest.join(key);
    match fs::remove_file(&target) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(source) => {
            return Err(Error::Io {
                path: target,
                source,
            });
        }
    }
    prune_empty_parents(&target, dest)?;
    Ok(true)
}

fn prune_empty_parents(file: &Path, dest: &Path) -> Result<()> {
    let mut dir = file.parent();
    while let Some(current) = dir {
        if current == dest || !current.starts_with(dest) {
            break;
        }
        let io_err = |source| Error::Io {
            path: current.to_path_buf(),
            source,
        };
        if fs::read_dir(current).map_err(io_err)?.next().is_some() {
            break;
        }
        fs::remove_dir(current).map_err(io_err)?;
        dir = current.parent();
    }
    Ok(())
}
