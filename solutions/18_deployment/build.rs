//! Embeds the git commit into the binary as `BUILD_GIT_SHA`.
//!
//! Resolution order: a valid `BUILD_GIT_SHA` environment variable (set by the container
//! build, which excludes `.git`), then `git rev-parse HEAD`, then `unknown`.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=BUILD_GIT_SHA");
    println!("cargo:rerun-if-changed=build.rs");

    let sha = std::env::var("BUILD_GIT_SHA")
        .ok()
        .and_then(valid_sha)
        .or_else(git_head_sha)
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=BUILD_GIT_SHA={sha}");
}

/// Accepts exactly 40 hex digits; anything else (including `unknown`) is rejected.
fn valid_sha(candidate: String) -> Option<String> {
    let lowered = candidate.trim().to_ascii_lowercase();
    (lowered.len() == 40 && lowered.bytes().all(|b| b.is_ascii_hexdigit())).then_some(lowered)
}

/// Reads HEAD from git and registers the files that change when HEAD moves.
fn git_head_sha() -> Option<String> {
    if let Some(head_path) = git(&["rev-parse", "--git-path", "HEAD"]) {
        println!("cargo:rerun-if-changed={head_path}");
    }
    // HEAD is usually a symbolic ref; a new commit on the branch moves the ref file, not HEAD.
    if let Some(ref_path) = git(&["symbolic-ref", "-q", "HEAD"])
        .and_then(|reference| git(&["rev-parse", "--git-path", reference.as_str()]))
    {
        println!("cargo:rerun-if-changed={ref_path}");
    }
    git(&["rev-parse", "HEAD"]).and_then(valid_sha)
}

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|text| text.trim().to_owned())
}
