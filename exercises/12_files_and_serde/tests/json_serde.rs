//! Part 3: serde wire formats: entries, streamed manifests, tagged events, and config.

use ex12_files_and_serde::config::SyncConfig;
use ex12_files_and_serde::json::{Event, read_manifest, read_manifest_each, write_manifest};
use ex12_files_and_serde::scan::{FileEntry, Manifest};
use ex12_files_and_serde::Error;
use jiff::Timestamp;
use serde_json::{Value, json};

const ABC_HEX: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

fn sample_entry(size: u64, hex: &str) -> serde_json::Value {
    json!({ "size": size, "mtime": "2024-01-01T00:00:00Z", "sha256": hex })
}

fn entry_from(value: Value) -> serde_json::Result<FileEntry> {
    serde_json::from_value(value)
}

fn sample_manifest() -> Manifest {
    let mut m = Manifest::new();
    let mtime = Timestamp::from_second(1_704_067_200).expect("valid timestamp");
    m.insert(
        "a.txt".to_owned(),
        entry_from(sample_entry(3, ABC_HEX)).expect("valid entry"),
    );
    m.insert(
        "sub/b.txt".to_owned(),
        FileEntry {
            size: 0,
            mtime,
            sha256: ex12_files_and_serde::hash::Sha256::digest(b""),
        },
    );
    m
}

#[test]
fn part3_entry_uses_camel_case_wire_format_and_rfc3339_mtime() {
    let entry = entry_from(sample_entry(3, ABC_HEX)).expect("valid entry");
    assert_eq!(
        serde_json::to_value(entry).expect("serialize"),
        sample_entry(3, ABC_HEX),
        "mtime is RFC 3339 UTC; digest is lowercase hex"
    );
}

#[test]
fn part3_entry_rejects_unknown_fields() {
    let mut v = sample_entry(3, ABC_HEX);
    v["owner"] = json!("root");
    assert!(entry_from(v).is_err(), "deny_unknown_fields must reject extra keys");
}

#[test]
fn part3_entry_rejects_short_and_non_hex_digests() {
    let short = &ABC_HEX[..63];
    assert!(entry_from(sample_entry(3, short)).is_err(), "63 hex chars is not a SHA-256");
    let non_hex = format!("zz{}", &ABC_HEX[2..]);
    assert!(entry_from(sample_entry(3, &non_hex)).is_err(), "non-hex digit must be rejected");
}

#[test]
fn part3_entry_accepts_uppercase_hex_and_offset_timestamps() {
    let upper = ABC_HEX.to_uppercase();
    let mut v = sample_entry(3, &upper);
    v["mtime"] = json!("2024-01-01T02:00:00+02:00");
    let entry = entry_from(v).expect("uppercase hex and offset timestamps are valid input");
    assert_eq!(
        entry.sha256,
        ex12_files_and_serde::hash::Sha256::digest(b"abc"),
        "uppercase hex decodes to the same digest"
    );
    assert_eq!(
        entry.mtime,
        Timestamp::from_second(1_704_067_200).expect("valid timestamp"),
        "offsets are normalised to the same instant"
    );
}

#[test]
fn part3_write_manifest_streams_a_record_array_in_path_order() {
    let mut buf = Vec::new();
    write_manifest(&mut buf, &sample_manifest()).expect("write");
    let parsed: Value = serde_json::from_slice(&buf).expect("valid JSON");
    let records = parsed.as_array().expect("manifest is a JSON array");
    assert_eq!(records.len(), 2, "one record per file");
    assert_eq!(records[0]["path"], "a.txt", "records follow BTreeMap order");
    assert_eq!(records[1]["path"], "sub/b.txt");
}

#[test]
fn part3_write_manifest_empty_is_empty_array() {
    let mut buf = Vec::new();
    write_manifest(&mut buf, &Manifest::new()).expect("write");
    assert_eq!(serde_json::from_slice::<Value>(&buf).expect("json"), json!([]));
}

#[test]
fn part3_read_manifest_round_trips_written_output() {
    let original = sample_manifest();
    let mut buf = Vec::new();
    write_manifest(&mut buf, &original).expect("write");
    let back = read_manifest(buf.as_slice()).expect("read");
    assert_eq!(
        serde_json::to_value(&back).expect("to_value"),
        serde_json::to_value(&original).expect("to_value"),
        "read must invert write"
    );
}

#[test]
fn part3_read_manifest_each_streams_records_and_counts_them() {
    let mut buf = Vec::new();
    write_manifest(&mut buf, &sample_manifest()).expect("write");
    let mut seen = Vec::new();
    let count = read_manifest_each(buf.as_slice(), |path, _entry| {
        seen.push(path);
        Ok(())
    })
    .expect("stream");
    assert_eq!(count, 2, "count of records streamed");
    assert_eq!(seen, ["a.txt", "sub/b.txt"], "callback order matches file order");
}

#[test]
fn part3_read_manifest_each_stops_at_first_callback_error() {
    let mut buf = Vec::new();
    write_manifest(&mut buf, &sample_manifest()).expect("write");
    let mut calls = 0;
    let result = read_manifest_each(buf.as_slice(), |_path, _entry| {
        calls += 1;
        Err(Error::Config("stop".to_owned()))
    });
    assert!(
        matches!(&result, Err(Error::Config(msg)) if msg == "stop"),
        "the callback's error must come back unchanged, got {result:?}"
    );
    assert_eq!(calls, 1, "no records are read after the callback fails");
}

#[test]
fn part3_read_manifest_rejects_duplicate_paths() {
    let rec = json!({ "path": "dup.txt", "entry": sample_entry(3, ABC_HEX) });
    let text = serde_json::to_vec(&json!([rec.clone(), rec])).expect("json");
    assert!(
        matches!(read_manifest(text.as_slice()), Err(Error::Manifest(_))),
        "duplicate path must be a Manifest error"
    );
}

#[test]
fn part3_read_manifest_rejects_non_array_root() {
    assert!(read_manifest(&b"{}"[..]).is_err(), "a manifest root must be an array");
}

#[test]
fn part3_event_wire_format_is_tagged_snake_case() {
    assert_eq!(
        serde_json::to_value(Event::Copied { path: "a.txt".to_owned() }).expect("ser"),
        json!({ "event": "copied", "path": "a.txt" }),
    );
    assert_eq!(
        serde_json::to_value(Event::Deleted { path: "b.txt".to_owned() }).expect("ser"),
        json!({ "event": "deleted", "path": "b.txt" }),
    );
}

#[test]
fn part3_event_skipped_reason_is_omitted_when_absent() {
    let none = serde_json::to_value(Event::Skipped { path: "x".to_owned(), reason: None }).expect("ser");
    assert_eq!(none, json!({ "event": "skipped", "path": "x" }), "absent reason is not written");

    let some = serde_json::to_value(Event::Skipped {
        path: "x".to_owned(),
        reason: Some("unchanged".to_owned()),
    })
    .expect("ser");
    assert_eq!(some["reason"], "unchanged", "present reason is written");
}

#[test]
fn part3_event_rejects_unknown_tags_and_fields() {
    assert!(
        serde_json::from_value::<Event>(json!({ "event": "moved", "path": "a" })).is_err(),
        "unknown tag must fail"
    );
    assert!(
        serde_json::from_value::<Event>(json!({ "event": "copied", "path": "a", "extra": 1 })).is_err(),
        "deny_unknown_fields must apply to variants too"
    );
}

#[test]
fn part3_config_applies_defaults_and_flattens_ignore_rules() {
    let cfg = SyncConfig::from_json(
        r#"{
            "source": "/data/src",
            "destination": "/data/dst",
            "ignore": ["target", { "pattern": "*.log", "reason": "noise" }]
        }"#,
    )
    .expect("valid config");
    assert_eq!(cfg.source.to_str(), Some("/data/src"));
    assert_eq!(cfg.destination.to_str(), Some("/data/dst"));
    assert_eq!(cfg.ignore, ["target", "*.log"], "string and object rules both flatten to patterns");
    assert_eq!(cfg.retention_days, 30, "retention-days defaults to 30");
}

#[test]
fn part3_config_is_kebab_case_and_rejects_unknown_keys() {
    let snake = r#"{ "source": "/a", "destination": "/b", "retention_days": 5 }"#;
    assert!(SyncConfig::from_json(snake).is_err(), "snake_case key is not part of the schema");

    let extra = r#"{ "source": "/a", "destination": "/b", "colour": "red" }"#;
    assert!(SyncConfig::from_json(extra).is_err(), "unknown keys must be rejected");

    let bad_rule = r#"{ "source": "/a", "destination": "/b", "ignore": [{ "pattern": "x", "bogus": 1 }] }"#;
    assert!(SyncConfig::from_json(bad_rule).is_err(), "unknown keys inside a rule are rejected");
}

#[test]
fn part3_config_validates_paths_retention_and_patterns() {
    let empty_src = r#"{ "source": "", "destination": "/b" }"#;
    assert!(
        matches!(SyncConfig::from_json(empty_src), Err(Error::Config(_))),
        "empty source is a Config error"
    );

    let at_limit = r#"{ "source": "/a", "destination": "/b", "retention-days": 36500 }"#;
    assert!(SyncConfig::from_json(at_limit).is_ok(), "36500 days is allowed");

    let over = r#"{ "source": "/a", "destination": "/b", "retention-days": 36501 }"#;
    assert!(
        matches!(SyncConfig::from_json(over), Err(Error::Config(_))),
        "retention above 36500 days is rejected"
    );

    let empty_rule = r#"{ "source": "/a", "destination": "/b", "ignore": [""] }"#;
    assert!(
        matches!(SyncConfig::from_json(empty_rule), Err(Error::Config(_))),
        "an empty ignore pattern is rejected"
    );
}
