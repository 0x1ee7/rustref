//! Black-box tests for `ex09_error_handling`. Every test fails against the stubs.

use std::cell::Cell;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::io::{self, ErrorKind};
use std::num::NonZeroU32;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::time::Duration;

use ex09_error_handling::batch::{
    BatchError, Classify, RetryError, Task, TaskFailed, retry, run_batch, triage,
};
use ex09_error_handling::chain::chain_messages;
use ex09_error_handling::config::{
    Config, ConfigError, InvalidPort, ParseError, SettingError, SettingMissing, get_setting,
    get_setting_as, load_config, startup_port,
};
use ex09_error_handling::isolation::{PluginError, run_isolated};
use ex09_error_handling::resources::{FsError, TempEnv, open_all, remove_files};
use ex09_error_handling::steps::run_checked;
use ex09_error_handling::transaction::{Transaction, TxError};

// ---------- helpers ----------

fn write_file(dir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, contents).expect("write fixture");
    path
}

fn config(json: &str) -> Config {
    serde_json::from_str(json).expect("fixture is a JSON object")
}

fn transient() -> io::Error {
    io::Error::from(ErrorKind::TimedOut)
}

fn permanent() -> io::Error {
    io::Error::from(ErrorKind::NotFound)
}

fn store(pairs: &[(&str, i32)]) -> HashMap<String, i32> {
    pairs.iter().map(|&(k, v)| (k.to_owned(), v)).collect()
}

fn env_of(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|&(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

fn port_of(v: &serde_json::Value) -> Result<u16, InvalidPort> {
    v.as_u64()
        .and_then(|p| u16::try_from(p).ok())
        .filter(|&p| p != 0)
        .ok_or(InvalidPort)
}

#[derive(Debug)]
struct Leaf {
    _code: u8,
}

impl fmt::Display for Leaf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("leaf")
    }
}

impl Error for Leaf {}

#[derive(Debug)]
struct Middle {
    cause: Leaf,
}

impl fmt::Display for Middle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("middle")
    }
}

impl Error for Middle {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.cause)
    }
}

#[derive(Debug)]
struct Outer {
    cause: Middle,
}

impl fmt::Display for Outer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("outer")
    }
}

impl Error for Outer {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.cause)
    }
}

/// An error whose `source()` is itself.
#[derive(Debug)]
struct Loop {
    _code: u8,
}

impl fmt::Display for Loop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("loop")
    }
}

impl Error for Loop {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self)
    }
}

// ---------- Part 1: error types and translation ----------

#[test]
fn part1_parse_error_from_utf8_maps_to_not_utf8() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.json");
    std::fs::write(&path, [0xff_u8]).unwrap();
    let err = load_config(&path).expect_err("invalid UTF-8 must be rejected");
    assert!(
        matches!(err, ConfigError::Malformed { source: ParseError::NotUtf8(_), .. }),
        "expected Malformed(NotUtf8), got {err:?}"
    );
}

#[test]
fn part1_parse_error_from_json_keeps_position() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.json");
    std::fs::write(&path, "{").unwrap();
    let err = load_config(&path).expect_err("truncated JSON must be rejected");
    assert!(
        matches!(err, ConfigError::Malformed { source: ParseError::Json(_), .. }),
        "expected Malformed(Json), got {err:?}"
    );
    let cause = std::error::Error::source(&err).expect("Malformed carries the parse error");
    assert!(cause.to_string().contains("line 1"), "message should keep the position: {cause}");
}

#[test]
fn part1_setting_missing_converts_into_setting_error() {
    let err = SettingError::from(SettingMissing::new("db.port"));
    assert!(
        matches!(&err, SettingError::Missing(m) if m.key() == "db.port"),
        "expected Missing(db.port), got {err:?}"
    );
}

#[test]
fn part1_config_error_setting_is_transparent() {
    let inner = SettingError::from(SettingMissing::new("db.port"));
    let expected = inner.to_string();
    let outer = ConfigError::from(inner);
    assert!(
        matches!(outer, ConfigError::Setting(_)),
        "expected Setting variant, got {outer:?}"
    );
    assert_eq!(
        outer.to_string(),
        expected,
        "a transparent variant must display as its inner error"
    );
}

#[test]
fn part1_malformed_config_keeps_path_and_source() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(dir.path(), "cfg.json", "[1, 2]");
    let err = load_config(&path).unwrap_err();
    assert!(
        matches!(
            &err,
            ConfigError::Malformed { path: p, source: ParseError::NotAnObject } if *p == path
        ),
        "expected Malformed(NotAnObject) carrying the path, got {err:?}"
    );
    let source = err.source().expect("Malformed must expose its cause");
    assert!(
        source.is::<ParseError>(),
        "source should be the ParseError"
    );
}

#[test]
fn part1_missing_file_is_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("absent.json");
    let err = load_config(&path).unwrap_err();
    assert!(
        matches!(&err, ConfigError::NotFound { path: p, .. } if *p == path),
        "expected NotFound with the path, got {err:?}"
    );
}

#[test]
fn part1_directory_is_unreadable() {
    let dir = tempfile::tempdir().unwrap();
    let err = load_config(dir.path()).unwrap_err();
    assert!(
        matches!(err, ConfigError::Unreadable { .. }),
        "a directory is an I/O failure, not NotFound: got {err:?}"
    );
}

// ---------- Part 2: chains, settings, application-edge errors ----------

#[test]
fn part2_chain_lists_outermost_first() {
    let err = Outer {
        cause: Middle {
            cause: Leaf { _code: 1 },
        },
    };
    assert_eq!(chain_messages(&err), vec!["outer", "middle", "leaf"]);
}

#[test]
fn part2_chain_stops_at_cycle() {
    let err = Loop { _code: 0 };
    assert_eq!(
        chain_messages(&err),
        vec!["loop"],
        "a self-referential source() must not loop forever"
    );
}

#[test]
fn part2_get_setting_walks_nested_objects() {
    let cfg = config(r#"{"db":{"port":5432}}"#);
    let value = get_setting(&cfg, "db.port").expect("db.port exists");
    assert_eq!(value.as_u64(), Some(5432));
}

#[test]
fn part2_get_setting_missing_segment_reports_full_key() {
    let cfg = config(r#"{"db":{"host":"x"}}"#);
    match get_setting(&cfg, "db.port") {
        Err(SettingError::Missing(m)) => assert_eq!(m.key(), "db.port"),
        other => panic!("expected Missing(db.port), got {other:?}"),
    }
}

#[test]
fn part2_get_setting_through_scalar_is_missing() {
    let cfg = config(r#"{"name":"svc"}"#);
    assert!(
        matches!(get_setting(&cfg, "name.first"), Err(SettingError::Missing(_))),
        "a non-object in the path is Missing, not a panic"
    );
}

#[test]
fn part2_get_setting_as_applies_cast() {
    let cfg = config(r#"{"db":{"port":5432}}"#);
    let port = get_setting_as(&cfg, "db.port", |v| v.as_u64().ok_or(InvalidPort))
        .expect("cast accepts 5432");
    assert_eq!(port, 5432);
}

#[test]
fn part2_get_setting_as_missing_never_calls_cast() {
    let cfg = config(r#"{"db":{}}"#);
    let mut called = false;
    let result = get_setting_as(&cfg, "db.port", |v| {
        called = true;
        v.as_u64().ok_or(InvalidPort)
    });
    assert!(matches!(result, Err(SettingError::Missing(_))));
    assert!(!called, "the cast must not run for a missing key");
}

#[test]
fn part2_get_setting_as_invalid_keeps_key_raw_and_source() {
    let cfg = config(r#"{"server":{"port":70000}}"#);
    match get_setting_as(&cfg, "server.port", port_of) {
        Err(SettingError::Invalid { key, raw, source }) => {
            assert_eq!(key, "server.port");
            assert_eq!(raw, "70000", "raw is the JSON text of the value");
            assert!(source.is::<InvalidPort>(), "source must be the cast's error");
        }
        other => panic!("expected Invalid, got {other:?}"),
    }
}

#[test]
fn part2_startup_port_reads_server_port() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(dir.path(), "cfg.json", r#"{"server":{"port":8080}}"#);
    assert_eq!(startup_port(&path).unwrap(), 8080);
}

#[test]
fn part2_startup_port_missing_file_keeps_config_error() {
    let dir = tempfile::tempdir().unwrap();
    let err = startup_port(dir.path().join("nope.json")).unwrap_err();
    assert!(
        matches!(err.downcast_ref::<ConfigError>(), Some(ConfigError::NotFound { .. })),
        "the concrete ConfigError must stay downcastable: {err:?}"
    );
}

#[test]
fn part2_startup_port_missing_setting_keeps_setting_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(dir.path(), "cfg.json", r#"{"name":"svc"}"#);
    let err = startup_port(&path).unwrap_err();
    assert!(
        matches!(err.downcast_ref::<SettingError>(), Some(SettingError::Missing(_))),
        "the concrete SettingError must stay downcastable: {err:?}"
    );
}

// ---------- Part 3: checked steps, log trail, unwinding ----------

#[test]
fn part3_success_returns_value_and_logs_in_order() {
    let mut log = Vec::new();
    let value = run_checked("load", || Ok::<u32, io::Error>(7), &mut log).expect("step succeeds");
    assert_eq!(value, 7);
    assert_eq!(log, ["start load", "end load", "ok load"]);
}

#[test]
fn part3_failure_wraps_with_name_and_source() {
    let mut log = Vec::new();
    let err = run_checked(
        "load",
        || Err::<(), io::Error>(io::Error::other("disk gone")),
        &mut log,
    )
    .unwrap_err();
    assert_eq!(err.step_name, "load");
    assert_eq!(err.to_string(), "step 'load' failed");
    let source = err.source().expect("the cause must be the source");
    assert_eq!(source.to_string(), "disk gone");
}

#[test]
fn part3_failure_logs_failed_not_ok() {
    let mut log = Vec::new();
    let _ = run_checked(
        "load",
        || Err::<(), io::Error>(io::Error::other("x")),
        &mut log,
    );
    assert_eq!(log, ["start load", "end load", "failed load"]);
}

#[test]
fn part3_panic_logs_end_and_propagates() {
    let mut log = Vec::new();
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = run_checked(
            "load",
            || -> Result<(), io::Error> { panic!("step exploded") },
            &mut log,
        );
    }));
    assert!(result.is_err(), "the panic must propagate, not become an error");
    assert_eq!(
        log,
        ["start load", "end load"],
        "end is logged even on panic; neither ok nor failed is"
    );
}

#[test]
fn part3_log_accumulates_across_steps() {
    let mut log = Vec::new();
    run_checked("a", || Ok::<(), io::Error>(()), &mut log).unwrap();
    let _ = run_checked("b", || Err::<(), io::Error>(permanent()), &mut log);
    assert_eq!(log.len(), 6, "log: {log:?}");
    assert_eq!(log[3], "start b");
    assert_eq!(log[5], "failed b");
}

// ---------- Part 4: transaction with savepoints ----------

#[test]
fn part4_writes_are_visible_and_survive_commit() {
    let mut s = store(&[("a", 1)]);
    {
        let mut tx = Transaction::begin(&mut s);
        tx.set("b", 2);
        assert_eq!(*tx.get("b").unwrap(), 2, "writes are visible inside the transaction");
        tx.commit();
    }
    assert_eq!(s, store(&[("a", 1), ("b", 2)]));
}

#[test]
fn part4_drop_without_commit_rolls_back() {
    let mut s = store(&[("a", 1)]);
    {
        let mut tx = Transaction::begin(&mut s);
        tx.set("a", 9);
        tx.set("c", 3);
    }
    assert_eq!(s, store(&[("a", 1)]), "dropping an uncommitted transaction rolls back");
}

#[test]
fn part4_delete_hides_key_and_rolls_back_on_drop() {
    let mut s = store(&[("a", 1)]);
    {
        let mut tx = Transaction::begin(&mut s);
        tx.delete("a").unwrap();
        assert!(
            matches!(&tx.get("a"), Err(TxError::KeyNotFound(k)) if k.as_str() == "a"),
            "a deleted key is not found inside the transaction"
        );
    }
    assert_eq!(s, store(&[("a", 1)]));
}

#[test]
fn part4_missing_key_is_key_not_found() {
    let mut s = store(&[("a", 1)]);
    let mut tx = Transaction::begin(&mut s);
    assert!(
        matches!(&tx.get("zz"), Err(TxError::KeyNotFound(k)) if k.as_str() == "zz"),
        "get on an absent key"
    );
    assert!(
        matches!(&tx.delete("zz"), Err(TxError::KeyNotFound(k)) if k.as_str() == "zz"),
        "delete on an absent key"
    );
}

#[test]
fn part4_commit_applies_delete() {
    let mut s = store(&[("a", 1), ("b", 2)]);
    let mut tx = Transaction::begin(&mut s);
    tx.delete("a").unwrap();
    tx.commit();
    assert_eq!(s, store(&[("b", 2)]));
}

#[test]
fn part4_savepoint_ok_keeps_writes() {
    let mut s = store(&[("a", 1)]);
    let mut tx = Transaction::begin(&mut s);
    let out = tx
        .savepoint(|tx| {
            tx.set("b", 2);
            Ok(42)
        })
        .expect("no error");
    assert_eq!(out, Some(42));
    assert_eq!(*tx.get("b").unwrap(), 2);
    tx.commit();
    assert_eq!(s, store(&[("a", 1), ("b", 2)]));
}

#[test]
fn part4_savepoint_rollback_discards_writes() {
    let mut s = store(&[("a", 1)]);
    let mut tx = Transaction::begin(&mut s);
    let out = tx
        .savepoint(|tx| -> Result<(), TxError> {
            tx.set("a", 5);
            tx.set("new", 7);
            tx.delete("a").unwrap();
            Err(TxError::SavepointRollback)
        })
        .expect("SavepointRollback is not an error for the caller");
    assert_eq!(out, None);
    assert_eq!(*tx.get("a").unwrap(), 1, "the delete and overwrite are undone");
    assert!(tx.get("new").is_err(), "the inserted key is removed");
    tx.commit();
    assert_eq!(s, store(&[("a", 1)]));
}

#[test]
fn part4_savepoint_other_error_propagates_and_undoes() {
    let mut s = store(&[("a", 1)]);
    let mut tx = Transaction::begin(&mut s);
    let r = tx.savepoint(|tx| -> Result<(), TxError> {
        tx.set("a", 5);
        tx.delete("missing")?;
        Ok(())
    });
    assert!(
        matches!(&r, Err(TxError::KeyNotFound(k)) if k.as_str() == "missing"),
        "other errors propagate: {r:?}"
    );
    assert_eq!(*tx.get("a").unwrap(), 1, "writes made before the error are undone");
    tx.set("after", 3);
    tx.commit();
    assert_eq!(s, store(&[("a", 1), ("after", 3)]));
}

#[test]
fn part4_nested_savepoints_roll_back_only_the_inner() {
    let mut s = store(&[]);
    let mut tx = Transaction::begin(&mut s);
    tx.savepoint(|tx| -> Result<(), TxError> {
        tx.set("x", 1);
        tx.savepoint(|tx| -> Result<(), TxError> {
            tx.set("y", 2);
            Err(TxError::SavepointRollback)
        })?;
        Ok(())
    })
    .unwrap();
    tx.commit();
    assert_eq!(s, store(&[("x", 1)]));
}

#[test]
fn part4_repeated_writes_roll_back_to_original() {
    let mut s = store(&[("a", 1)]);
    {
        let mut tx = Transaction::begin(&mut s);
        tx.set("a", 2);
        tx.set("a", 3);
        tx.set("a", 4);
    }
    assert_eq!(s, store(&[("a", 1)]), "undo must restore the value before the first write");
}

// ---------- Part 5: scoped environment and file resources ----------

#[test]
fn part5_temp_env_set_and_get_see_changes() {
    let mut m = env_of(&[]);
    let mut g = TempEnv::new(&mut m);
    g.set("MODE", Some("test"));
    assert_eq!(g.get("MODE"), Some("test"));
    assert_eq!(g.get("NOPE"), None);
}

#[test]
fn part5_drop_restores_previous_value() {
    let mut m = env_of(&[("A", "orig")]);
    {
        let mut g = TempEnv::new(&mut m);
        g.set("A", Some("new"));
        assert_eq!(g.get("A"), Some("new"));
    }
    assert_eq!(m.get("A").map(String::as_str), Some("orig"));
}

#[test]
fn part5_drop_removes_variables_it_created() {
    let mut m = env_of(&[]);
    {
        let mut g = TempEnv::new(&mut m);
        g.set("B", Some("x"));
    }
    assert!(!m.contains_key("B"), "a variable that did not exist before is removed");
}

#[test]
fn part5_unset_is_visible_then_restored() {
    let mut m = env_of(&[("A", "orig")]);
    {
        let mut g = TempEnv::new(&mut m);
        g.set("A", None);
        assert_eq!(g.get("A"), None);
    }
    assert_eq!(m.get("A").map(String::as_str), Some("orig"));
}

#[test]
fn part5_drop_restores_in_reverse_order() {
    let mut m = env_of(&[("A", "orig")]);
    {
        let mut g = TempEnv::new(&mut m);
        g.set("A", Some("one"));
        g.set("A", Some("two"));
    }
    assert_eq!(
        m.get("A").map(String::as_str),
        Some("orig"),
        "restoring in reverse returns to the value before the first change"
    );
}

#[test]
fn part5_panic_still_restores() {
    let mut m = env_of(&[("A", "orig")]);
    let payload = catch_unwind(AssertUnwindSafe(|| {
        let mut g = TempEnv::new(&mut m);
        g.set("A", Some("boom"));
        assert_eq!(g.get("A"), Some("boom"));
        panic!("inside guard");
    }))
    .expect_err("the closure panics");
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"inside guard"), "the closure's own panic must unwind");
    assert_eq!(m.get("A").map(String::as_str), Some("orig"), "unwinding runs the guard's drop");
}

#[test]
fn part5_open_all_opens_every_file() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.txt", "a");
    let b = write_file(dir.path(), "b.txt", "b");
    let files = open_all(&[a, b]).expect("both files exist");
    assert_eq!(files.len(), 2);
}

#[test]
fn part5_open_all_names_the_first_missing_path() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.txt", "a");
    let missing = dir.path().join("missing.txt");
    let err = open_all(&[a, missing.clone()]).unwrap_err();
    assert!(
        matches!(&err, FsError::Open { path, .. } if *path == missing),
        "expected Open for {}, got {err:?}",
        missing.display()
    );
}

#[test]
fn part5_remove_files_counts_deleted_and_skips_missing() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.txt", "a");
    let b = write_file(dir.path(), "b.txt", "b");
    let missing = dir.path().join("missing.txt");
    let removed = remove_files(&[a.clone(), missing, b.clone()]).expect("missing files are skipped");
    assert_eq!(removed, 2);
    assert!(!a.exists() && !b.exists());
}

#[test]
fn part5_remove_files_stops_at_first_real_error() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(dir.path(), "a.txt", "a");
    let sub = dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    let err = remove_files(&[file.clone(), sub.clone()]).unwrap_err();
    assert!(
        matches!(&err, FsError::Remove { path, .. } if *path == sub),
        "expected Remove for the directory, got {err:?}"
    );
    assert!(!file.exists(), "deletions before the failure stay done");
}

// ---------- Part 6: panics vs errors at a plugin boundary ----------

#[test]
fn part6_ok_passes_value_through() {
    let value = run_isolated("p", || Ok::<i32, io::Error>(5)).expect("plugin succeeds");
    assert_eq!(value, 5);
}

#[test]
fn part6_err_becomes_failed_with_source() {
    let err = run_isolated("p", || Err::<(), io::Error>(io::Error::other("bad"))).unwrap_err();
    match err {
        PluginError::Failed { name, source } => {
            assert_eq!(name, "p");
            assert_eq!(source.to_string(), "bad");
        }
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn part6_str_panic_becomes_panicked() {
    let err = run_isolated("p", || -> Result<(), io::Error> { panic!("boom") }).unwrap_err();
    match err {
        PluginError::Panicked { name, message } => {
            assert_eq!(name, "p");
            assert_eq!(message, "boom");
        }
        other => panic!("expected Panicked, got {other:?}"),
    }
}

#[test]
fn part6_formatted_panic_message_is_captured() {
    let err = run_isolated("p", || -> Result<(), io::Error> {
        panic!("code {}", 7)
    })
    .unwrap_err();
    assert!(
        matches!(&err, PluginError::Panicked { message, .. } if message == "code 7"),
        "String payloads must be captured too: {err:?}"
    );
}

#[test]
fn part6_non_string_payload_is_reported() {
    let err = run_isolated("p", || -> Result<(), io::Error> {
        std::panic::panic_any(42u8)
    })
    .unwrap_err();
    assert!(
        matches!(&err, PluginError::Panicked { message, .. } if message == "non-string panic payload"),
        "got {err:?}"
    );
}

#[test]
fn part6_mutable_capture_with_assert_unwind_safe() {
    let mut calls = 0;
    let out = run_isolated(
        "p",
        AssertUnwindSafe(|| {
            calls += 1;
            Ok::<(), io::Error>(())
        }),
    );
    assert!(out.is_ok());
    assert_eq!(calls, 1);
}

// ---------- Part 7: batch aggregation, classification, retry ----------

#[test]
fn part7_transient_kinds_are_retryable() {
    let kinds = [
        ErrorKind::TimedOut,
        ErrorKind::Interrupted,
        ErrorKind::WouldBlock,
        ErrorKind::ResourceBusy,
        ErrorKind::ConnectionAborted,
        ErrorKind::ConnectionRefused,
        ErrorKind::ConnectionReset,
        ErrorKind::BrokenPipe,
    ];
    for kind in kinds {
        assert!(io::Error::from(kind).is_transient(), "{kind:?} should be transient");
    }
}

#[test]
fn part7_other_kinds_are_permanent() {
    let kinds = [
        ErrorKind::NotFound,
        ErrorKind::PermissionDenied,
        ErrorKind::InvalidData,
        ErrorKind::Other,
    ];
    for kind in kinds {
        assert!(!io::Error::from(kind).is_transient(), "{kind:?} should be permanent");
    }
}

#[test]
fn part7_run_batch_returns_results_in_order() {
    let tasks = vec![
        Task::new("first", || Ok::<i32, io::Error>(1)),
        Task::new("second", || Ok(2)),
    ];
    let results = run_batch(tasks).expect("all tasks succeed");
    assert_eq!(results, vec![("first".to_owned(), 1), ("second".to_owned(), 2)]);
}

#[test]
fn part7_run_batch_empty_is_ok() {
    let tasks: Vec<Task<'_, i32, io::Error>> = Vec::new();
    assert_eq!(run_batch(tasks).expect("nothing failed"), vec![]);
}

#[test]
fn part7_duplicate_name_runs_nothing() {
    let hits = Cell::new(0);
    let tasks = vec![
        Task::new("a", || {
            hits.set(hits.get() + 1);
            Ok::<i32, io::Error>(1)
        }),
        Task::new("a", || {
            hits.set(hits.get() + 1);
            Ok(2)
        }),
    ];
    let err = run_batch(tasks).unwrap_err();
    assert!(
        matches!(&err, BatchError::DuplicateTask(n) if n.as_str() == "a"),
        "got {err:?}"
    );
    assert_eq!(hits.get(), 0, "a duplicate name is detected before any task runs");
}

#[test]
fn part7_collects_every_failure_in_order() {
    let tasks = vec![
        Task::new("first", || Ok::<i32, io::Error>(1)),
        Task::new("second", || Err(permanent())),
        Task::new("third", || Ok(3)),
        Task::new("fourth", || Err(transient())),
    ];
    let err = run_batch(tasks).unwrap_err();
    assert_eq!(err.to_string(), "2 of 4 tasks failed");
    match err {
        BatchError::Failed { total, failures } => {
            assert_eq!(total, 4);
            let names: Vec<&str> = failures.iter().map(|f| f.name.as_str()).collect();
            assert_eq!(names, ["second", "fourth"]);
        }
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn part7_runs_every_task_even_after_failure() {
    let hits = Cell::new(0);
    let tasks = vec![
        Task::new("a", || {
            hits.set(hits.get() + 1);
            Err::<i32, io::Error>(permanent())
        }),
        Task::new("b", || {
            hits.set(hits.get() + 1);
            Ok(2)
        }),
        Task::new("c", || {
            hits.set(hits.get() + 1);
            Err(permanent())
        }),
    ];
    let _ = run_batch(tasks);
    assert_eq!(hits.get(), 3, "a failure does not stop the batch");
}

#[test]
fn part7_task_failed_displays_name_and_source() {
    let tasks = vec![Task::new("x", || Err::<(), io::Error>(io::Error::other("boom")))];
    let err = run_batch(tasks).unwrap_err();
    let BatchError::Failed { failures, .. } = err else {
        panic!("expected Failed");
    };
    let failure: &TaskFailed<io::Error> = &failures[0];
    assert_eq!(failure.to_string(), "task 'x' failed");
    assert_eq!(failure.source().unwrap().to_string(), "boom");
}

#[test]
fn part7_panic_propagates_out_of_batch() {
    let result = catch_unwind(AssertUnwindSafe(|| {
        run_batch(vec![Task::new("p", || -> Result<i32, io::Error> {
            panic!("kaboom")
        })])
    }));
    let payload = result.expect_err("a panic is not collected as a failure");
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"kaboom"), "the task's own panic must propagate");
}

#[test]
fn part7_triage_splits_by_classification() {
    let failures = vec![
        TaskFailed {
            name: "a".to_owned(),
            source: transient(),
        },
        TaskFailed {
            name: "b".to_owned(),
            source: permanent(),
        },
        TaskFailed {
            name: "c".to_owned(),
            source: transient(),
        },
    ];
    let split = triage(failures);
    let transient_names: Vec<&str> = split.transient.iter().map(|f| f.name.as_str()).collect();
    let permanent_names: Vec<&str> = split.permanent.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(transient_names, ["a", "c"]);
    assert_eq!(permanent_names, ["b"]);
}

#[test]
fn part7_retry_succeeds_after_transient_failures() {
    let calls = Cell::new(0);
    let mut sleeps = Vec::new();
    let out = retry(
        NonZeroU32::new(3).unwrap(),
        Duration::from_millis(10),
        || {
            let n = calls.get();
            calls.set(n + 1);
            if n < 2 { Err(transient()) } else { Ok(7) }
        },
        |d| sleeps.push(d),
    );
    assert_eq!(out.unwrap(), 7);
    assert_eq!(calls.get(), 3);
    assert_eq!(
        sleeps,
        vec![Duration::from_millis(10), Duration::from_millis(20)],
        "backoff doubles: base, then 2*base"
    );
}

#[test]
fn part7_retry_permanent_returns_at_once() {
    let calls = Cell::new(0);
    let mut sleeps = Vec::new();
    let out = retry(
        NonZeroU32::new(5).unwrap(),
        Duration::from_millis(10),
        || {
            calls.set(calls.get() + 1);
            Err::<(), io::Error>(permanent())
        },
        |d| sleeps.push(d),
    );
    match out {
        Err(RetryError::Permanent(e)) => assert_eq!(e.kind(), ErrorKind::NotFound),
        other => panic!("expected Permanent, got {other:?}"),
    }
    assert_eq!(calls.get(), 1);
    assert!(sleeps.is_empty());
}

#[test]
fn part7_retry_exhausted_sleeps_only_between_attempts() {
    let calls = Cell::new(0);
    let mut sleeps = Vec::new();
    let out = retry(
        NonZeroU32::new(3).unwrap(),
        Duration::from_millis(10),
        || {
            calls.set(calls.get() + 1);
            Err::<(), io::Error>(transient())
        },
        |d| sleeps.push(d),
    );
    match out {
        Err(RetryError::Exhausted { attempts, last }) => {
            assert_eq!(attempts, 3);
            assert_eq!(last.kind(), ErrorKind::TimedOut);
        }
        other => panic!("expected Exhausted, got {other:?}"),
    }
    assert_eq!(calls.get(), 3);
    assert_eq!(
        sleeps,
        vec![Duration::from_millis(10), Duration::from_millis(20)],
        "no sleep after the final attempt"
    );
}

#[test]
fn part7_retry_permanent_after_transient_stops_then_reports() {
    let calls = Cell::new(0);
    let mut sleeps = Vec::new();
    let out = retry(
        NonZeroU32::new(5).unwrap(),
        Duration::from_millis(10),
        || {
            let n = calls.get();
            calls.set(n + 1);
            if n == 0 { Err::<(), _>(transient()) } else { Err(permanent()) }
        },
        |d| sleeps.push(d),
    );
    assert!(matches!(out, Err(RetryError::Permanent(_))), "got {out:?}");
    assert_eq!(calls.get(), 2);
    assert_eq!(sleeps.len(), 1, "one sleep between the transient and the permanent failure");
}
