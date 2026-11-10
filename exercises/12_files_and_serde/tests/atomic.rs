//! Part 4: atomic replacement via `tempfile` and JSON persistence on top of it.

use std::fs;
use std::io;
use std::path::Path;

use ex12_files_and_serde::atomic::{load_json, save_json_atomic, write_atomic};
use ex12_files_and_serde::Error;
use serde::{Deserialize, Serialize, Serializer};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Settings {
    name: String,
    count: u32,
}

/// Number of entries in `dir`, including any temp files left behind.
fn entry_count(dir: &Path) -> usize {
    fs::read_dir(dir).expect("read_dir").count()
}

#[test]
fn part4_save_then_load_round_trips_and_writes_trailing_newline() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("settings.json");
    let original = Settings { name: "nightly".to_owned(), count: 3 };

    save_json_atomic(&original, &path).expect("save");
    let text = fs::read_to_string(&path).expect("read back");
    assert!(text.ends_with('\n'), "saved JSON must end with a newline");

    let loaded: Settings = load_json(&path).expect("load");
    assert_eq!(loaded, original, "load must invert save");
}

#[test]
fn part4_write_atomic_replaces_contents_and_leaves_no_temp_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("state.txt");
    fs::write(&path, b"old").expect("seed");

    write_atomic(&path, |w| w.write_all(b"new contents")).expect("atomic write");

    assert_eq!(fs::read(&path).expect("read"), b"new contents", "target replaced");
    assert_eq!(entry_count(dir.path()), 1, "no temp files may remain after success");
}

#[test]
fn part4_failed_write_keeps_old_file_and_removes_temp_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("state.txt");
    fs::write(&path, b"precious").expect("seed");

    let result = write_atomic(&path, |w| {
        w.write_all(b"partial")?;
        Err(io::Error::other("disk full"))
    });

    assert!(result.is_err(), "the closure's failure must propagate");
    assert_eq!(fs::read(&path).expect("read"), b"precious", "old contents must survive");
    assert_eq!(entry_count(dir.path()), 1, "the partial temp file must be cleaned up");
}

/// A value whose `Serialize` impl always fails, to exercise the save error path.
struct Unserializable;

impl Serialize for Unserializable {
    fn serialize<S: Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("refusing to serialize"))
    }
}

#[test]
fn part4_serialize_failure_keeps_previous_file_intact() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.json");
    fs::write(&path, b"{\"keep\":true}").expect("seed");

    let result = save_json_atomic(&Unserializable, &path);

    assert!(result.is_err(), "a serialization failure must be an error");
    assert_eq!(fs::read(&path).expect("read"), b"{\"keep\":true}", "target must be untouched");
    assert_eq!(entry_count(dir.path()), 1, "no temp file may be left behind");
}

#[test]
fn part4_load_distinguishes_missing_file_from_malformed_json() {
    let dir = tempfile::tempdir().expect("tempdir");

    let missing = dir.path().join("absent.json");
    assert!(
        matches!(load_json::<Settings>(&missing), Err(Error::Io { .. })),
        "a missing file is an Io error"
    );

    let broken = dir.path().join("broken.json");
    fs::write(&broken, b"{ not json").expect("seed");
    assert!(
        matches!(load_json::<Settings>(&broken), Err(Error::Json(_))),
        "malformed JSON is a Json error"
    );
}
