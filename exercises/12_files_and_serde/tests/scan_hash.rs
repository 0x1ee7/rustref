//! Part 1: streaming hashes, CRC-32, glob ignore rules, and tree scanning.

use std::fs;
use std::io::Cursor;
use std::path::Path;

use ex12_files_and_serde::glob::{full_match, is_ignored};
use ex12_files_and_serde::hash::{Crc32, Sha256, crc32, sha256_file};
use ex12_files_and_serde::scan::scan_tree;
use ex12_files_and_serde::Error;

/// Deterministic pseudo-random bytes (LCG). Same input, same output, every platform.
fn pseudo_random(len: usize, mut seed: u64) -> Vec<u8> {
    (0..len)
        .map(|_| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) as u8
        })
        .collect()
}

fn write_file(root: &Path, rel: &str, bytes: &[u8]) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("has parent")).expect("create dirs");
    fs::write(path, bytes).expect("write fixture");
}

#[test]
fn part1_sha256_empty_input_matches_fips_vector() {
    assert_eq!(
        Sha256::digest(b"").to_string(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "SHA-256 of the empty string"
    );
}

#[test]
fn part1_sha256_abc_matches_fips_vector() {
    assert_eq!(
        Sha256::digest(b"abc").to_string(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "SHA-256 of \"abc\""
    );
}

#[test]
fn part1_sha256_multi_block_message_matches_fips_vector() {
    // 448-bit message: forces padding into a second block.
    assert_eq!(
        Sha256::digest(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq").to_string(),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
    );
}

#[test]
fn part1_sha256_one_million_a_matches_fips_vector() {
    let data = vec![b'a'; 1_000_000];
    assert_eq!(
        Sha256::digest(&data).to_string(),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
    );
}

#[test]
fn part1_sha256_any_split_matches_one_shot() {
    let data = pseudo_random(1000, 7);
    let expected = Sha256::digest(&data);
    for split in [0, 1, 55, 56, 63, 64, 65, 127, 128, 999, 1000] {
        let mut hasher = Sha256::new();
        hasher.update(&data[..split]);
        hasher.update(&data[split..]);
        assert_eq!(
            hasher.finalize(),
            expected,
            "split at {split} must not change the digest"
        );
    }
}

#[test]
fn part1_sha256_io_write_sink_matches_one_shot() {
    let data = pseudo_random(4096, 99);
    let mut hasher = Sha256::new();
    std::io::copy(&mut Cursor::new(&data), &mut hasher).expect("copy into hasher");
    assert_eq!(
        hasher.finalize(),
        Sha256::digest(&data),
        "io::Write path must hash the same bytes"
    );
}

#[test]
fn part1_sha256_file_streams_same_digest_as_memory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = pseudo_random(3 * 1024 * 1024 + 17, 1234);
    let path = dir.path().join("big.bin");
    fs::write(&path, &data).expect("write fixture");
    assert_eq!(
        sha256_file(&path).expect("hash file"),
        Sha256::digest(&data),
        "file digest must equal in-memory digest"
    );
}

#[test]
fn part1_sha256_file_missing_is_io_error_with_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("nope.bin");
    match sha256_file(&missing) {
        Err(Error::Io { path, .. }) => assert_eq!(path, missing, "error must name the file"),
        other => panic!("expected Error::Io, got {other:?}"),
    }
}

#[test]
fn part1_crc32_check_value_matches_ieee() {
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926, "CRC-32/IEEE check value");
}

#[test]
fn part1_crc32_empty_is_zero_and_incremental_matches_one_shot() {
    assert_eq!(crc32(b""), 0, "CRC-32 of nothing is zero");
    let data = pseudo_random(777, 3);
    let mut crc = Crc32::new();
    crc.update(&data[..100]);
    crc.update(&data[100..]);
    assert_eq!(crc.finalize(), crc32(&data), "incremental CRC must match one-shot");
}

#[test]
fn part1_glob_star_stays_within_one_segment() {
    assert!(full_match("*.txt", "notes.txt"), "star matches a plain name");
    assert!(!full_match("*.txt", "docs/notes.txt"), "star must not cross '/'");
    assert!(full_match("a?c", "abc"), "? matches one character");
    assert!(!full_match("a?c", "a/c"), "? must not match '/'");
}

#[test]
fn part1_glob_double_star_spans_zero_or_more_segments() {
    assert!(full_match("**/x.rs", "x.rs"), "** may match zero segments");
    assert!(full_match("**/x.rs", "a/b/x.rs"), "** may match several segments");
    assert!(full_match("src/**/*.rs", "src/a/b.rs"));
    assert!(!full_match("src/**/*.rs", "lib/a.rs"), "prefix must still match");
}

#[test]
fn part1_glob_is_case_sensitive() {
    assert!(!full_match("*.TXT", "a.txt"), "matching is case-sensitive");
}

#[test]
fn part1_ignore_bare_pattern_matches_any_component() {
    assert!(is_ignored("build/out/a.o", &["build"]), "bare name matches a directory component");
    assert!(!is_ignored("src/rebuild.rs", &["build"]), "must match whole components only");
}

#[test]
fn part1_ignore_pattern_with_slash_is_anchored_at_root() {
    assert!(is_ignored("docs/a.md", &["docs/*.md"]), "anchored pattern matches from root");
    assert!(!is_ignored("x/docs/a.md", &["docs/*.md"]), "anchored pattern must not float");
}

#[test]
fn part1_scan_collects_nested_files_and_respects_ignore() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    write_file(root, "a.txt", b"hello");
    write_file(root, "sub/b.bin", &[0, 1, 2]);
    write_file(root, "target/x.o", b"ignored");
    write_file(root, "scratch.tmp", b"ignored too");

    let manifest = scan_tree(root, &["target", "*.tmp"]).expect("scan");
    let keys: Vec<&str> = manifest.keys().map(String::as_str).collect();
    assert_eq!(keys, ["a.txt", "sub/b.bin"], "keys must be relative, '/'-separated, sorted");

    let a = manifest["a.txt"];
    assert_eq!(a.size, 5, "size in bytes");
    assert_eq!(a.sha256, Sha256::digest(b"hello"), "digest of contents");
    assert_eq!(
        a.mtime.as_nanosecond() % 1_000,
        0,
        "mtime must be truncated to whole microseconds"
    );
}

#[test]
fn part1_scan_empty_directory_yields_empty_manifest() {
    let dir = tempfile::tempdir().expect("tempdir");
    let manifest = scan_tree(dir.path(), &[]).expect("scan empty dir");
    assert!(manifest.is_empty(), "empty directory must scan to an empty manifest");
}

#[test]
fn part1_scan_root_errors_are_distinct() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("missing");
    assert!(
        matches!(&scan_tree(&missing, &[]), Err(Error::RootNotFound(p)) if p == &missing),
        "missing root must be RootNotFound"
    );
    write_file(dir.path(), "plain.txt", b"x");
    let file_root = dir.path().join("plain.txt");
    assert!(
        matches!(scan_tree(&file_root, &[]), Err(Error::NotADirectory(_))),
        "a file as root must be NotADirectory"
    );
}

#[cfg(unix)]
#[test]
fn part1_scan_skips_symlinks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    write_file(root, "real.txt", b"data");
    std::os::unix::fs::symlink(root.join("real.txt"), root.join("link.txt")).expect("symlink");

    let manifest = scan_tree(root, &[]).expect("scan");
    let keys: Vec<&str> = manifest.keys().map(String::as_str).collect();
    assert_eq!(keys, ["real.txt"], "symlinks must not appear in the manifest");
}

#[cfg(unix)]
#[test]
fn part1_scan_rejects_non_utf8_file_names() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let name = OsStr::from_bytes(b"bad\xff.txt");
    fs::write(dir.path().join(name), b"x").expect("write non-UTF-8 name");

    assert!(
        matches!(scan_tree(dir.path(), &[]), Err(Error::NonUtf8Path(_))),
        "non-UTF-8 names must be rejected, not lossily converted"
    );
}
