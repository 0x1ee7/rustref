//! Part 6: container files: Dockerfile and .dockerignore lint checks.
//!
//! The tests read the files in this crate directory and check the rules a
//! production image must follow. They do not run Docker.

use std::fs;
use std::path::PathBuf;

fn crate_file(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Logical Dockerfile instructions: comments and blank lines dropped, `\` continuations joined.
/// Each entry is `(keyword, rest_of_line)` with the keyword upper-cased.
fn instructions(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut pending = String::new();
    for raw in text.lines() {
        let line = raw.trim();
        if pending.is_empty() && (line.is_empty() || line.starts_with('#')) {
            continue;
        }
        if let Some(stripped) = line.strip_suffix('\\') {
            pending.push_str(stripped.trim_end());
            pending.push(' ');
            continue;
        }
        pending.push_str(line);
        let full = std::mem::take(&mut pending);
        let (kw, rest) = full.split_once(char::is_whitespace).unwrap_or((full.as_str(), ""));
        out.push((kw.to_ascii_uppercase(), rest.trim().to_string()));
    }
    out
}

fn from_images(ins: &[(String, String)]) -> Vec<String> {
    ins.iter()
        .filter(|(k, _)| k == "FROM")
        .map(|(_, rest)| {
            // `FROM [--platform=...] image[:tag] [AS name]`
            rest.split_whitespace()
                .find(|t| !t.starts_with("--"))
                .unwrap_or_default()
                .to_string()
        })
        .collect()
}

#[test]
fn part6_dockerfile_is_written() {
    assert!(!crate_file("Dockerfile").trim().is_empty(), "Dockerfile must not be empty");
}

#[test]
fn part6_every_base_image_is_pinned_to_a_tag() {
    let ins = instructions(&crate_file("Dockerfile"));
    let images = from_images(&ins);
    assert!(!images.is_empty(), "Dockerfile must have FROM instructions");
    for image in &images {
        let (_, tag) = image
            .rsplit_once(':')
            .unwrap_or_else(|| panic!("FROM {image} has no tag; pin it"));
        assert!(!tag.is_empty() && tag != "latest", "FROM {image} must pin a real tag, not `latest`");
    }
}

#[test]
fn part6_builder_is_a_pinned_rust_slim_image() {
    let text = crate_file("Dockerfile");
    let ins = instructions(&text);
    let builder = ins
        .iter()
        .filter(|(k, _)| k == "FROM")
        .find(|(_, rest)| rest.to_ascii_lowercase().contains(" as builder"))
        .unwrap_or_else(|| panic!("the build stage must be named `AS builder`"));
    let image = builder.1.split_whitespace().next().unwrap_or_default();
    assert!(
        image.starts_with("rust:1.") && image.contains("-slim"),
        "builder must be a pinned `rust:1.x.y-slim...` image, got {image}"
    );
}

#[test]
fn part6_multi_stage_runtime_has_no_toolchain() {
    let ins = instructions(&crate_file("Dockerfile"));
    let images = from_images(&ins);
    assert!(images.len() >= 2, "a multi-stage build needs a builder and a runtime stage");
    let runtime = images.last().expect("at least one FROM");
    assert!(!runtime.starts_with("rust:"), "the runtime stage must not ship the Rust toolchain");
    assert!(
        runtime.starts_with("gcr.io/distroless/") || runtime.contains("-slim"),
        "runtime must be distroless or a debian slim image, got {runtime}"
    );
}

#[test]
fn part6_build_uses_buildkit_cache_mounts() {
    let ins = instructions(&crate_file("Dockerfile"));
    let build = ins
        .iter()
        .filter(|(k, _)| k == "RUN")
        .map(|(_, rest)| rest.as_str())
        .find(|rest| rest.contains("cargo build"))
        .expect("Dockerfile must run cargo build");
    assert!(
        build.contains("--mount=type=cache"),
        "cargo build should use BuildKit cache mounts for the registry and target dir"
    );
}

#[test]
fn part6_release_build_is_locked() {
    let ins = instructions(&crate_file("Dockerfile"));
    let build = ins
        .iter()
        .filter(|(k, _)| k == "RUN")
        .any(|(_, rest)| rest.contains("cargo build") && rest.contains("--locked") && rest.contains("--release"));
    assert!(build, "a RUN must run `cargo build --release --locked`");
}

#[test]
fn part6_runs_as_non_root_user() {
    let ins = instructions(&crate_file("Dockerfile"));
    let last_user = ins
        .iter()
        .rfind(|(k, _)| k == "USER")
        .map(|(_, rest)| rest.trim().to_string())
        .expect("the final stage must set USER");
    let root = matches!(last_user.as_str(), "root" | "0" | "0:0" | "root:root");
    assert!(!root, "the last USER must not be root, got {last_user:?}");
}

#[test]
fn part6_entrypoint_and_cmd_use_exec_form() {
    let ins = instructions(&crate_file("Dockerfile"));
    for keyword in ["ENTRYPOINT", "CMD"] {
        let rest = ins
            .iter()
            .rfind(|(k, _)| k == keyword)
            .map(|(_, rest)| rest.as_str())
            .unwrap_or_else(|| panic!("{keyword} must be set"));
        assert!(rest.starts_with('['), "{keyword} must use exec (JSON array) form, got {rest:?}");
        assert!(rest.ends_with(']'), "{keyword} must be a complete JSON array, got {rest:?}");
    }
}

#[test]
fn part6_exposes_a_numeric_port() {
    let ins = instructions(&crate_file("Dockerfile"));
    let port = ins
        .iter()
        .rfind(|(k, _)| k == "EXPOSE")
        .map(|(_, rest)| rest.split('/').next().unwrap_or_default().trim().to_string())
        .expect("EXPOSE must be set");
    assert!(port.parse::<u16>().is_ok_and(|p| p > 0), "EXPOSE must be a numeric port, got {port:?}");
}

#[test]
fn part6_healthcheck_is_not_disabled_and_healthz_is_documented() {
    let text = crate_file("Dockerfile");
    let ins = instructions(&text);
    for (_, rest) in ins.iter().filter(|(k, _)| k == "HEALTHCHECK") {
        assert!(!rest.starts_with("NONE"), "HEALTHCHECK must not be disabled with NONE");
    }
    assert!(
        text.contains("/healthz"),
        "the Dockerfile must reference /healthz (a HEALTHCHECK, or a note on probing it when no shell exists)"
    );
}

#[test]
fn part6_no_secrets_are_baked_into_the_image() {
    let ins = instructions(&crate_file("Dockerfile"));
    assert!(!ins.is_empty(), "the Dockerfile must contain instructions");
    for (_, rest) in ins.iter().filter(|(k, _)| k == "ENV" || k == "ARG") {
        let upper = rest.to_ascii_uppercase();
        assert!(
            !upper.contains("API_TOKEN") && !upper.contains("DATABASE_URL"),
            "secrets must be injected at runtime, not baked in: {rest}"
        );
    }
}

#[test]
fn part6_listen_address_binds_all_interfaces() {
    let ins = instructions(&crate_file("Dockerfile"));
    let listen = ins
        .iter()
        .filter(|(k, _)| k == "ENV")
        .any(|(_, rest)| rest.contains("LISTEN_ADDR=0.0.0.0:"));
    assert!(listen, "the container must set LISTEN_ADDR=0.0.0.0:<port> so the port is reachable");
}

#[test]
fn part6_dockerignore_excludes_target_and_git() {
    let text = crate_file(".dockerignore");
    let entries: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.trim_start_matches("**/").trim_end_matches('/').to_string())
        .collect();
    for required in ["target", ".git"] {
        assert!(
            entries.iter().any(|e| e == required),
            "`.dockerignore` must exclude {required:?}, has {entries:?}"
        );
    }
}
