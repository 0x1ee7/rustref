//! Part 5: build metadata and the build script contract.

use std::fs;
use std::path::PathBuf;

use ex18_deployment::build_info;

fn crate_file(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

#[test]
fn part5_version_matches_the_crate_version() {
    assert_eq!(build_info::version(), env!("CARGO_PKG_VERSION"));
}

#[test]
fn part5_git_sha_is_unknown_or_full_lowercase_hex() {
    let sha = build_info::git_sha();
    let hex = sha.len() == 40 && sha.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    assert!(sha == "unknown" || hex, "git_sha must be `unknown` or 40 lowercase hex chars, got {sha:?}");
}

#[test]
fn part5_git_sha_is_stable_across_calls() {
    assert_eq!(build_info::git_sha(), build_info::git_sha(), "a compile-time value cannot change");
}

#[test]
fn part5_build_script_emits_build_git_sha() {
    let script = crate_file("build.rs");
    assert!(
        script.contains("cargo:rustc-env=BUILD_GIT_SHA="),
        "build.rs must export BUILD_GIT_SHA with `cargo:rustc-env=BUILD_GIT_SHA=...`"
    );
}

#[test]
fn part5_build_script_reruns_when_git_head_moves() {
    let script = crate_file("build.rs");
    assert!(
        script.contains("cargo:rerun-if-changed="),
        "build.rs must declare rerun-if-changed so a new commit updates the embedded sha"
    );
}

#[test]
fn part5_build_script_falls_back_to_unknown_without_git() {
    let script = crate_file("build.rs");
    assert!(script.contains("unknown"), "build.rs must fall back to `unknown` when git is unavailable");
}
