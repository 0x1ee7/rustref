//! Part 2: manifest diffing, path-safety validation, and applying a diff to disk.

use std::fs;
use std::path::Path;

use ex12_files_and_serde::diff::{ManifestDiff, apply_diff, diff_manifests, validate_relative_path};
use ex12_files_and_serde::hash::Sha256;
use ex12_files_and_serde::scan::{FileEntry, Manifest, scan_tree};
use ex12_files_and_serde::Error;
use jiff::Timestamp;

fn entry(content: &[u8], mtime_secs: i64) -> FileEntry {
    FileEntry {
        size: content.len() as u64,
        mtime: Timestamp::from_second(mtime_secs).expect("valid timestamp"),
        sha256: Sha256::digest(content),
    }
}

fn manifest(items: &[(&str, FileEntry)]) -> Manifest {
    items.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
}

#[test]
fn part2_diff_classifies_added_removed_and_modified() {
    let old = manifest(&[
        ("keep.txt", entry(b"same", 1)),
        ("gone.txt", entry(b"bye", 1)),
        ("edit.txt", entry(b"v1", 1)),
    ]);
    let new = manifest(&[
        ("keep.txt", entry(b"same", 1)),
        ("new.txt", entry(b"hi", 2)),
        ("edit.txt", entry(b"v2", 2)),
    ]);
    let diff = diff_manifests(&old, &new);
    assert_eq!(diff.added, ["new.txt"], "present only in new");
    assert_eq!(diff.removed, ["gone.txt"], "present only in old");
    assert_eq!(diff.modified, ["edit.txt"], "same path, different content");
    assert!(!diff.is_empty(), "a non-trivial diff must not report empty");
}

#[test]
fn part2_diff_ignores_mtime_only_changes_and_reports_empty() {
    let old = manifest(&[("a.txt", entry(b"content", 100))]);
    let new = manifest(&[("a.txt", entry(b"content", 999))]);
    let diff = diff_manifests(&old, &new);
    assert!(diff.is_empty(), "mtime-only change is not a modification: {diff:?}");
    assert!(diff_manifests(&old, &old).is_empty(), "identical manifests produce no changes");
}

#[test]
fn part2_diff_lists_are_sorted_by_path() {
    let old = Manifest::new();
    let new = manifest(&[
        ("z.txt", entry(b"z", 1)),
        ("a.txt", entry(b"a", 1)),
        ("m/n.txt", entry(b"n", 1)),
    ]);
    let diff = diff_manifests(&old, &new);
    assert_eq!(diff.added, ["a.txt", "m/n.txt", "z.txt"], "added must be sorted");
}

#[test]
fn part2_validate_accepts_plain_relative_paths() {
    for ok in ["a", "dir/sub/file.txt", "a.b/c-d_e", ".hidden", "x..y"] {
        assert!(validate_relative_path(ok).is_ok(), "`{ok}` should be accepted");
    }
}

#[test]
fn part2_validate_rejects_unsafe_keys_with_unsafe_path_error() {
    let cases = [
        "",
        "/etc/passwd",
        "../escape.txt",
        "a/../b",
        "./a",
        "a/./b",
        "a//b",
        "a/",
        "a\\b",
    ];
    for bad in cases {
        assert!(
            matches!(validate_relative_path(bad), Err(Error::UnsafePath { .. })),
            "`{bad:?}` must be rejected with UnsafePath"
        );
    }
}

#[test]
fn part2_apply_copies_added_and_modified_and_keeps_source_mtime() {
    let src = tempfile::tempdir().expect("tempdir");
    let dst = tempfile::tempdir().expect("tempdir");
    write(src.path(), "new.txt", b"fresh");
    write(src.path(), "edit.txt", b"updated");
    write(dst.path(), "edit.txt", b"stale");

    let old = manifest(&[("edit.txt", entry(b"stale", 1))]);
    let now = scan_tree(src.path(), &[]).expect("scan source");
    let diff = diff_manifests(&old, &now);
    let result = apply_diff(&diff, src.path(), dst.path()).expect("apply");

    let mut copied = result.copied.clone();
    copied.sort();
    assert_eq!(copied, ["edit.txt", "new.txt"], "every added and modified key is copied");
    assert!(result.deleted.is_empty(), "nothing removed");
    assert_eq!(fs::read(dst.path().join("edit.txt")).expect("read"), b"updated");
    assert_eq!(fs::read(dst.path().join("new.txt")).expect("read"), b"fresh");

    let src_mtime = fs::metadata(src.path().join("new.txt")).expect("meta").modified().expect("mtime");
    let dst_mtime = fs::metadata(dst.path().join("new.txt")).expect("meta").modified().expect("mtime");
    assert_eq!(dst_mtime, src_mtime, "copied file must carry the source mtime");
}

#[test]
fn part2_apply_deletes_removed_files_and_prunes_empty_parents() {
    let src = tempfile::tempdir().expect("tempdir");
    let dst = tempfile::tempdir().expect("tempdir");
    write(dst.path(), "nested/deep/file.txt", b"old");
    write(dst.path(), "keep/other.txt", b"stay");

    let diff = ManifestDiff {
        removed: vec!["nested/deep/file.txt"],
        ..ManifestDiff::default()
    };
    let result = apply_diff(&diff, src.path(), dst.path()).expect("apply");

    assert_eq!(result.deleted, ["nested/deep/file.txt"], "removal reported");
    assert!(!dst.path().join("nested").exists(), "emptied parents must be pruned");
    assert!(dst.path().join("keep/other.txt").exists(), "non-empty dirs must survive");
    assert!(dst.path().exists(), "the destination root itself is never pruned");
}

#[test]
fn part2_apply_does_not_report_already_missing_removals() {
    let src = tempfile::tempdir().expect("tempdir");
    let dst = tempfile::tempdir().expect("tempdir");
    let diff = ManifestDiff {
        removed: vec!["ghost.txt"],
        ..ManifestDiff::default()
    };
    let result = apply_diff(&diff, src.path(), dst.path()).expect("missing removal is not an error");
    assert!(result.deleted.is_empty(), "a file that was already absent is not reported");
}

#[test]
fn part2_apply_validates_every_key_before_touching_disk() {
    let src = tempfile::tempdir().expect("tempdir");
    let dst = tempfile::tempdir().expect("tempdir");
    write(src.path(), "ok.txt", b"fine");

    let diff = ManifestDiff {
        added: vec!["ok.txt", "../escape.txt"],
        ..ManifestDiff::default()
    };
    let err = apply_diff(&diff, src.path(), dst.path()).expect_err("unsafe key must fail");
    assert!(matches!(err, Error::UnsafePath { .. }), "got {err:?}");
    assert!(
        !dst.path().join("ok.txt").exists(),
        "validation must run before any copy happens"
    );
}

fn write(root: &Path, rel: &str, bytes: &[u8]) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("has parent")).expect("create dirs");
    fs::write(path, bytes).expect("write fixture");
}
