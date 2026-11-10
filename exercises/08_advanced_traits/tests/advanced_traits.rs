//! Black-box tests for the advanced-traits exercise. Every `partN_*` test fails
//! against the `todo!()` stubs. Run one part with `cargo test -p ex08_advanced_traits part2`.

use std::collections::HashMap;
use std::io::{self, Cursor, Read};
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;

use ex08_advanced_traits::dispatch::{send_one, FanOut, FanOutError, Notifier, NotifyError, RecordingNotifier};
use ex08_advanced_traits::ext::{JsonExt, ResultExt, SqlValue, StrExt};
use ex08_advanced_traits::lending::{
    largest, ApiClient, Catalog, ChunkReader, Entry, Lazy, LendingIterator, MemoryCatalog, ShardedCatalog,
};
use ex08_advanced_traits::model::{create_table_sql, Column, ColumnKind, MaxLen, Model, RepoError, Repository, Validator};
use ex08_advanced_traits::plugins::{
    boxed_as_named, Named, Pipeline, PluginError, Services, Stage, StageConfig, StageRegistry,
};
use ex08_advanced_traits::typestate::{
    ConnectError, Connected, Connection, Disconnected, Email, EmailError, Endpoint, EndpointError, ExecError,
    HasUrl, Method, NoUrl, Request, RequestBuilder, UrlError,
};

// ---------------------------------------------------------------------------
// Part 1: trait objects vs generics
// ---------------------------------------------------------------------------

/// Notifier defined by the test. It appends to a shared log so the test can
/// inspect deliveries after the probe has moved into a `FanOut`.
struct Probe {
    name: &'static str,
    log: Arc<Mutex<Vec<String>>>,
}

impl Notifier for Probe {
    fn name(&self) -> &str {
        self.name
    }

    fn notify(&self, message: &str) -> Result<(), NotifyError> {
        self.log.lock().unwrap().push(format!("{}:{message}", self.name));
        Ok(())
    }
}

#[test]
fn part1_recording_notifier_keeps_messages_in_order() {
    let n = RecordingNotifier::new("log");
    assert_eq!(n.name(), "log", "name() must return the constructor's name");
    n.notify("first").unwrap();
    n.notify("second").unwrap();
    assert_eq!(n.sent(), vec!["first", "second"], "messages must be recorded in delivery order");
}

#[test]
fn part1_failing_notifier_returns_unavailable_and_records_nothing() {
    let n = RecordingNotifier::failing("pager");
    let err = n.notify("disk full").unwrap_err();
    assert_eq!(err, NotifyError::Unavailable { channel: "pager".into() }, "error must name the channel");
    assert!(n.sent().is_empty(), "a failed delivery must not be recorded");
}

#[test]
fn part1_fanout_delivers_to_every_target_and_counts_successes() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let fan = FanOut::new()
        .with(Probe { name: "a", log: Arc::clone(&log) })
        .with(Probe { name: "b", log: Arc::clone(&log) });
    assert_eq!(fan.len(), 2, "len() must count registered targets");
    assert!(!fan.is_empty());
    assert_eq!(fan.broadcast("deploy"), Ok(2), "all successes must return the target count");
    assert_eq!(*log.lock().unwrap(), vec!["a:deploy", "b:deploy"], "every target must receive the message");
}

#[test]
fn part1_fanout_continues_after_failure_and_reports_every_failure() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let fan = FanOut::new()
        .with(Probe { name: "ok", log: Arc::clone(&log) })
        .with(RecordingNotifier::failing("pager"))
        .with(RecordingNotifier::failing("sms"))
        .with(Probe { name: "late", log: Arc::clone(&log) });
    let err = fan.broadcast("alert").unwrap_err();
    assert_eq!(
        err,
        FanOutError {
            attempted: 4,
            failures: vec![
                ("pager".to_string(), NotifyError::Unavailable { channel: "pager".into() }),
                ("sms".to_string(), NotifyError::Unavailable { channel: "sms".into() }),
            ],
        },
        "failures must be reported in registration order with their channel names"
    );
    assert_eq!(
        *log.lock().unwrap(),
        vec!["ok:alert", "late:alert"],
        "targets after a failing one must still be attempted"
    );
}

#[test]
fn part1_fanout_empty_broadcast_is_ok_zero_and_error_display_counts() {
    let empty = FanOut::new();
    assert!(empty.is_empty(), "a new fan-out has no targets");
    assert_eq!(empty.broadcast("nobody listens"), Ok(0));
    let err = FanOut::new().with(RecordingNotifier::failing("x")).broadcast("m").unwrap_err();
    assert!(
        err.to_string().contains("1/1"),
        "Display should report failures/attempted, got '{err}'"
    );
}

#[test]
fn part1_fanout_is_shareable_across_threads() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let fan = FanOut::new().with(Probe { name: "t", log: Arc::clone(&log) });
    let results: Vec<Result<usize, FanOutError>> = thread::scope(|s| {
        let handles: Vec<_> = (0..4).map(|_| s.spawn(|| fan.broadcast("tick"))).collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert!(
        results.iter().all(|r| *r == Ok(1)),
        "every concurrent broadcast should deliver once, got {results:?}"
    );
    assert_eq!(log.lock().unwrap().len(), 4, "four broadcasts must record four deliveries");
}

#[test]
fn part1_send_one_accepts_unsized_trait_objects() {
    let rec = RecordingNotifier::new("obj");
    let dyn_ref: &dyn Notifier = &rec;
    send_one(dyn_ref, "via dyn").unwrap();
    assert_eq!(rec.sent(), vec!["via dyn"], "send_one must work through &dyn Notifier");
}

#[test]
fn part1_box_dyn_notifier_is_itself_a_notifier() {
    let boxed: Box<dyn Notifier> = Box::new(RecordingNotifier::failing("sms"));
    let err = send_one(&boxed, "hi").unwrap_err();
    assert_eq!(
        err,
        NotifyError::Unavailable { channel: "sms".into() },
        "Box<dyn Notifier> must forward to the inner notifier"
    );
}

// ---------------------------------------------------------------------------
// Part 2: associated types and consts vs generic parameters
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
struct Person {
    id: u32,
    name: String,
    active: bool,
}

impl Model for Person {
    const TABLE: &'static str = "people";
    type Key = u32;

    fn key(&self) -> u32 {
        self.id
    }

    fn columns() -> &'static [Column] {
        &[
            Column { name: "id", kind: ColumnKind::Integer },
            Column { name: "name", kind: ColumnKind::Text },
            Column { name: "active", kind: ColumnKind::Boolean },
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Tag {
    slug: &'static str,
}

impl Model for Tag {
    const TABLE: &'static str = "tags";
    type Key = &'static str;

    fn key(&self) -> &'static str {
        self.slug
    }

    fn columns() -> &'static [Column] {
        &[Column { name: "slug", kind: ColumnKind::Text }]
    }
}

fn person(id: u32, name: &str) -> Person {
    Person { id, name: name.into(), active: true }
}

#[test]
fn part2_model_consts_describe_the_table() {
    assert_eq!(Person::TABLE, "people");
    assert_eq!(Person::columns().len(), 3, "Person declares three columns");
    assert_eq!(Person::columns()[1].name, "name");
    assert_eq!(Tag::TABLE, "tags");
    assert!(Repository::<Person>::new().is_empty(), "a new repository holds no rows");
}

#[test]
fn part2_create_table_sql_lists_columns_in_declaration_order() {
    assert_eq!(
        create_table_sql::<Person>(),
        "CREATE TABLE people (id INTEGER, name TEXT, active BOOLEAN)"
    );
    assert_eq!(create_table_sql::<Tag>(), "CREATE TABLE tags (slug TEXT)");
}

#[test]
fn part2_repository_insert_then_get() {
    let mut repo = Repository::<Person>::new();
    repo.insert(person(1, "ann")).unwrap();
    assert_eq!(repo.get(1).map(|p| p.name.as_str()), Some("ann"));
    assert_eq!(repo.get(2), None, "missing keys return None");
}

#[test]
fn part2_repository_rejects_duplicate_keys_and_keeps_original() {
    let mut repo = Repository::<Person>::new();
    repo.insert(person(1, "ann")).unwrap();
    let err = repo.insert(person(1, "bob")).unwrap_err();
    assert_eq!(err, RepoError::DuplicateKey { table: "people", key: "1".into() });
    assert_eq!(repo.get(1).unwrap().name, "ann", "the original row must not be overwritten");
}

#[test]
fn part2_repository_remove_returns_stored_model() {
    let mut repo = Repository::<Person>::new();
    repo.insert(person(7, "cy")).unwrap();
    assert_eq!(repo.remove(7), Some(person(7, "cy")));
    assert_eq!(repo.remove(7), None, "second removal finds nothing");
    assert!(repo.is_empty());
}

#[test]
fn part2_repository_iter_is_key_ordered_and_len_counts_rows() {
    let mut repo = Repository::<Person>::new();
    for id in [3, 1, 2] {
        repo.insert(person(id, "x")).unwrap();
    }
    let ids: Vec<u32> = repo.iter().map(|p| p.id).collect();
    assert_eq!(ids, vec![1, 2, 3], "iteration must follow ascending key order");
    assert_eq!(repo.len(), 3);
}

#[test]
fn part2_repository_supports_borrowed_string_keys() {
    let mut repo = Repository::<Tag>::new();
    repo.insert(Tag { slug: "rust" }).unwrap();
    repo.insert(Tag { slug: "async" }).unwrap();
    assert_eq!(repo.get("rust"), Some(&Tag { slug: "rust" }));
    let slugs: Vec<&str> = repo.iter().map(|t| t.slug).collect();
    assert_eq!(slugs, vec!["async", "rust"], "&str keys sort lexicographically");
}

#[test]
fn part2_max_len_on_str_counts_chars_not_bytes() {
    assert_eq!(MaxLen(5).check("héllo"), Ok(()), "five chars, six bytes, must pass");
    assert_eq!(MaxLen(4).check("héllo"), Err("length 5 exceeds maximum 4".to_string()));
}

#[test]
fn part2_max_len_on_slices_counts_elements() {
    let data = [1_u8, 2, 3];
    assert_eq!(MaxLen(3).check(&data[..]), Ok(()));
    assert_eq!(MaxLen(2).check(&data[..]), Err("length 3 exceeds maximum 2".to_string()));
}

// ---------------------------------------------------------------------------
// Part 3: blanket impls, extension traits, sealed traits
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
struct Point {
    x: i32,
    y: i32,
}

#[test]
fn part3_to_json_blanket_impl_covers_structs_and_unsized_types() {
    assert_eq!(Point { x: 1, y: 2 }.to_json().unwrap(), r#"{"x":1,"y":2}"#);
    assert_eq!("hi".to_json().unwrap(), "\"hi\"", "str is unsized; the blanket impl must cover it");
    assert_eq!([1, 2][..].to_json().unwrap(), "[1,2]", "slices are unsized too");
}

#[test]
fn part3_to_json_pretty_is_two_space_indented() {
    assert_eq!(
        Point { x: 1, y: 2 }.to_json_pretty().unwrap(),
        "{\n  \"x\": 1,\n  \"y\": 2\n}"
    );
}

#[test]
fn part3_to_json_surfaces_serializer_errors() {
    let mut map = HashMap::new();
    map.insert((1, 2), 3);
    assert!(map.to_json().is_err(), "tuple map keys are not valid JSON object keys and must error");
}

#[test]
fn part3_str_truncate_chars_respects_char_boundaries() {
    assert_eq!("héllo".truncate_chars(2), "hé", "must cut on a char boundary, not a byte index");
    assert_eq!("abc".truncate_chars(10), "abc", "short input is returned whole");
    assert_eq!("".truncate_chars(0), "");
}

#[test]
fn part3_str_slugify_collapses_and_trims_separators() {
    assert_eq!("  Hello,   World!! 2026 ".slugify(), "hello-world-2026");
    assert_eq!("---".slugify(), "", "all-separator input yields an empty slug");
}

#[test]
fn part3_str_slugify_treats_non_ascii_as_separator() {
    assert_eq!("Café au lait".slugify(), "caf-au-lait");
}

#[test]
fn part3_result_tag_err_prefixes_only_errors() {
    let failed: Result<u8, &str> = Err("boom");
    assert_eq!(failed.tag_err("loader"), Err("loader: boom".to_string()));
    let ok: Result<u8, &str> = Ok(3);
    assert_eq!(ok.tag_err("loader"), Ok(3), "Ok must pass through untouched");
}

#[test]
fn part3_sql_literal_quotes_and_escapes_text() {
    assert_eq!("it's".sql_literal(), "'it''s'", "single quotes are doubled");
    assert_eq!(
        String::from("ann").sql_literal(),
        "'ann'",
        "String must render identically to str"
    );
}

#[test]
fn part3_sql_literal_scalars_and_null() {
    assert_eq!(42_i64.sql_literal(), "42");
    assert_eq!(true.sql_literal(), "TRUE");
    assert_eq!(None::<i64>.sql_literal(), "NULL", "None renders as NULL");
    assert_eq!(Some(7_i64).sql_literal(), "7", "Some delegates to the inner value");
}

// ---------------------------------------------------------------------------
// Part 4: supertraits, upcasting, Any service maps, factories
// ---------------------------------------------------------------------------

/// Stage defined by the test. Its factory captures a prefix in a closure.
struct Tag2(String);

impl Named for Tag2 {
    fn name(&self) -> &str {
        "quote"
    }
}

impl Stage for Tag2 {
    fn apply(&mut self, input: &str) -> String {
        format!("{}{input}", self.0)
    }
}

#[test]
fn part4_builtin_upper_and_trim_apply() {
    let reg = StageRegistry::with_builtins();
    let mut upper = reg.create("upper", &StageConfig::new()).unwrap();
    assert_eq!(upper.apply("hi"), "HI");
    let mut trim = reg.create("trim", &StageConfig::new()).unwrap();
    assert_eq!(trim.apply("  x \t"), "x");
}

#[test]
fn part4_truncate_counts_chars_not_bytes() {
    let reg = StageRegistry::with_builtins();
    let mut t = reg.create("truncate", &StageConfig::new().with("max", "3")).unwrap();
    assert_eq!(t.apply("héllo"), "hél", "max counts chars");
    assert_eq!(t.apply("ok"), "ok", "short input passes through unchanged");
}

#[test]
fn part4_invalid_truncate_config_is_reported_by_stage() {
    let reg = StageRegistry::with_builtins();
    let cases = [
        StageConfig::new(),
        StageConfig::new().with("max", "0"),
        StageConfig::new().with("max", "abc"),
    ];
    for cfg in &cases {
        match reg.create("truncate", cfg).map(|_| ()) {
            Err(PluginError::InvalidConfig { stage, .. }) => assert_eq!(stage, "truncate"),
            other => panic!("expected InvalidConfig for {cfg:?}, got {other:?}"),
        }
    }
}

#[test]
fn part4_registry_lists_names_sorted() {
    assert_eq!(
        StageRegistry::with_builtins().names(),
        vec!["sequence", "trim", "truncate", "upper"]
    );
}

#[test]
fn part4_unknown_stage_names_the_missing_key() {
    let err = StageRegistry::with_builtins()
        .create("nope", &StageConfig::new())
        .map(|_| ())
        .unwrap_err();
    assert_eq!(err, PluginError::UnknownStage("nope".into()));
}

#[test]
fn part4_registry_rejects_duplicate_registration() {
    let mut reg = StageRegistry::with_builtins();
    let err = reg.register("upper", |_| Ok(Box::new(Tag2(String::new())))).unwrap_err();
    assert_eq!(err, PluginError::Duplicate("upper".into()));
    assert!(reg.register("shout", |_| Ok(Box::new(Tag2(String::new())))).is_ok());
    assert!(reg.names().contains(&"shout"), "a fresh name must be listed");
}

#[test]
fn part4_closure_factory_captures_state() {
    let mut reg = StageRegistry::new();
    let prefix = String::from("> ");
    reg.register("quote", move |_cfg| Ok(Box::new(Tag2(prefix.clone())))).unwrap();
    let mut stage = reg.create("quote", &StageConfig::new()).unwrap();
    assert_eq!(stage.apply("hello"), "> hello", "the captured prefix must reach the stage");
}

#[test]
fn part4_pipeline_runs_stages_in_order_and_lists_names() {
    let reg = StageRegistry::with_builtins();
    let mut pipeline = Pipeline::new();
    pipeline.push(reg.create("trim", &StageConfig::new()).unwrap());
    pipeline.push(reg.create("upper", &StageConfig::new()).unwrap());
    assert_eq!(pipeline.names(), vec!["trim", "upper"]);
    assert_eq!(pipeline.run("  hi  "), "HI");
}

#[test]
fn part4_sequence_stage_keeps_state_across_runs() {
    let reg = StageRegistry::with_builtins();
    let mut pipeline = Pipeline::new();
    pipeline.push(reg.create("sequence", &StageConfig::new()).unwrap());
    assert_eq!(pipeline.run("a"), "1: a");
    assert_eq!(pipeline.run("b"), "2: b", "stage state must persist between runs");
}

#[test]
fn part4_boxed_stage_upcasts_to_named_supertrait() {
    let reg = StageRegistry::with_builtins();
    let named: Box<dyn Named> = boxed_as_named(reg.create("trim", &StageConfig::new()).unwrap());
    assert_eq!(named.name(), "trim");
}

#[test]
fn part4_pipeline_moves_across_threads() {
    let reg = StageRegistry::with_builtins();
    let mut pipeline = Pipeline::new();
    pipeline.push(reg.create("upper", &StageConfig::new()).unwrap());
    let out = thread::spawn(move || pipeline.run("threaded")).join().unwrap();
    assert_eq!(out, "THREADED");
}

#[test]
fn part4_services_are_keyed_by_type() {
    let mut svc = Services::new();
    assert_eq!(svc.insert(7_u32), None);
    assert_eq!(svc.insert(String::from("cfg")), None);
    assert_eq!(svc.get::<u32>(), Some(&7));
    assert_eq!(svc.get::<String>().map(String::as_str), Some("cfg"));
    assert_eq!(svc.get::<i64>(), None, "distinct types never collide");
}

#[test]
fn part4_services_insert_replaces_and_returns_previous() {
    let mut svc = Services::new();
    svc.insert(1_u32);
    assert_eq!(svc.insert(2_u32), Some(1));
    assert_eq!(svc.get::<u32>(), Some(&2));
}

#[test]
fn part4_services_get_mut_and_remove() {
    let mut svc = Services::new();
    svc.insert(5_u32);
    *svc.get_mut::<u32>().unwrap() *= 2;
    assert!(svc.contains::<u32>());
    assert_eq!(svc.remove::<u32>(), Some(10));
    assert!(!svc.contains::<u32>());
    assert_eq!(svc.remove::<u32>(), None);
}

// ---------------------------------------------------------------------------
// Part 5: typestate, validated newtypes, and Deref vs newtype delegation
// ---------------------------------------------------------------------------

#[test]
fn part5_endpoint_parses_host_and_port() {
    let ep = Endpoint::parse("db.internal:5432").unwrap();
    assert_eq!(ep.host(), "db.internal");
    assert_eq!(ep.port(), 5432);
    assert_eq!(ep.to_string(), "db.internal:5432");
}

#[test]
fn part5_endpoint_rejects_malformed_and_bad_ports() {
    assert_eq!(Endpoint::parse("nohost"), Err(EndpointError::Malformed("nohost".into())));
    assert_eq!(Endpoint::parse(":5432"), Err(EndpointError::Malformed(":5432".into())));
    assert_eq!(Endpoint::parse("db:0"), Err(EndpointError::InvalidPort("db:0".into())));
    assert_eq!(Endpoint::parse("db:70000"), Err(EndpointError::InvalidPort("db:70000".into())));
    assert_eq!(Endpoint::parse("db:abc"), Err(EndpointError::InvalidPort("db:abc".into())));
}

#[test]
fn part5_connect_moves_disconnected_to_connected() {
    let conn = Connection::new(Endpoint::parse("db.internal:5432").unwrap());
    let connected: Connection<Connected> = conn.connect().expect("db.internal must connect");
    assert_eq!(connected.endpoint().to_string(), "db.internal:5432");
    assert_eq!(connected.executed(), 0, "a fresh session has executed nothing");
}

#[test]
fn part5_failed_connect_hands_back_the_disconnected_connection() {
    let ep = Endpoint::parse("replica.unreachable:5432").unwrap();
    let (conn, err): (Connection<Disconnected>, ConnectError) =
        Connection::new(ep.clone()).connect().unwrap_err();
    assert_eq!(err, ConnectError::Refused { endpoint: "replica.unreachable:5432".into() });
    assert_eq!(conn.endpoint(), &ep, "the caller keeps the disconnected connection to retry or reroute");
}

#[test]
fn part5_execute_counts_successful_statements() {
    let mut conn = Connection::new(Endpoint::parse("db:5432").unwrap()).connect().unwrap();
    conn.execute("SELECT 1").unwrap();
    conn.execute("UPDATE t SET x = 1").unwrap();
    assert_eq!(conn.executed(), 2);
}

#[test]
fn part5_execute_rejects_blank_statement_without_counting() {
    let mut conn = Connection::new(Endpoint::parse("db:5432").unwrap()).connect().unwrap();
    assert_eq!(conn.execute("   "), Err(ExecError::EmptyStatement));
    assert_eq!(conn.executed(), 0, "rejected statements must not be counted");
}

#[test]
fn part5_disconnect_then_reconnect_cycle() {
    let connected = Connection::new(Endpoint::parse("db:5432").unwrap()).connect().unwrap();
    let disconnected: Connection<Disconnected> = connected.disconnect();
    assert_eq!(disconnected.endpoint().to_string(), "db:5432");
    let again = disconnected.connect().expect("reconnect must succeed");
    assert_eq!(again.executed(), 0, "a new session starts with a zero count");
}

#[test]
fn part5_request_builder_keeps_method_url_and_headers() {
    let pending: RequestBuilder<NoUrl> = RequestBuilder::new(Method::Post).header("X-Trace", "abc");
    let ready: RequestBuilder<HasUrl> = pending.url("https://api.example.com/v1/items").unwrap();
    let req: Request = ready.header("Content-Type", "application/json").build();
    assert_eq!(req.method(), Method::Post);
    assert_eq!(req.url(), "https://api.example.com/v1/items");
    assert_eq!(req.header("x-trace"), Some("abc"), "header lookup is case-insensitive");
    assert_eq!(req.header("content-type"), Some("application/json"));
    assert_eq!(req.header("missing"), None);
}

#[test]
fn part5_request_url_rejects_relative_and_unsupported_schemes() {
    let builder = || RequestBuilder::new(Method::Get);
    assert_eq!(
        builder().url("example.com/x").map(|_| ()),
        Err(UrlError::Invalid("example.com/x".into())),
        "relative URLs are invalid"
    );
    assert_eq!(builder().url("https://").map(|_| ()), Err(UrlError::Invalid("https://".into())));
    assert_eq!(
        builder().url("ftp://files.example.com").map(|_| ()),
        Err(UrlError::UnsupportedScheme("ftp".into()))
    );
}

#[test]
fn part5_email_parses_and_displays() {
    let e: Email = "ann@example.com".parse().unwrap();
    assert_eq!(e.as_str(), "ann@example.com");
    assert_eq!(e.to_string(), "ann@example.com");
}

#[test]
fn part5_email_rejects_malformed_addresses() {
    for bad in ["no-at-sign", "a@b@c.d", "a@nodot", " a@b.c", "@x.y", "a@.y"] {
        assert_eq!(
            bad.parse::<Email>().map(|_| ()),
            Err(EmailError(bad.into())),
            "'{bad}' must be rejected"
        );
    }
}

#[test]
fn part5_email_as_hashmap_key_looks_up_by_str() {
    let mut map: HashMap<Email, u32> = HashMap::new();
    map.insert("ann@example.com".parse().unwrap(), 3);
    assert_eq!(map.get("ann@example.com"), Some(&3), "Borrow<str> must let &str index the map");
}

// ---------------------------------------------------------------------------
// Part 6: GATs (lending iterators), RPITIT, lazy Deref proxy
// ---------------------------------------------------------------------------

#[test]
fn part6_lazy_defers_construction_until_deref() {
    let builds = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&builds);
    let lazy = Lazy::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        ApiClient::new("https://api.example.com")
    });
    assert_eq!(builds.load(Ordering::SeqCst), 0, "constructing the proxy must not run the factory");
    assert!(!Lazy::is_initialized(&lazy));
    assert_eq!(lazy.url_for("/v1"), "https://api.example.com/v1", "method calls auto-deref");
    assert_eq!(builds.load(Ordering::SeqCst), 1);
    assert!(Lazy::is_initialized(&lazy));
}

#[test]
fn part6_lazy_builds_once_and_force_returns_the_same_instance() {
    let builds = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&builds);
    let lazy = Lazy::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        ApiClient::new("https://x.test")
    });
    let first = Lazy::force(&lazy);
    let second: &ApiClient = &lazy;
    assert!(std::ptr::eq(first, second), "every access must hand out the same stored value");
    assert_eq!(builds.load(Ordering::SeqCst), 1);
}

#[test]
fn part6_lazy_concurrent_first_access_initialises_once() {
    let builds = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&builds);
    let lazy = Lazy::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        ApiClient::new("https://race.test")
    });
    let barrier = Barrier::new(8);
    thread::scope(|s| {
        for _ in 0..8 {
            s.spawn(|| {
                barrier.wait();
                assert_eq!(lazy.url_for("/ping"), "https://race.test/ping");
            });
        }
    });
    assert_eq!(
        builds.load(Ordering::SeqCst),
        1,
        "concurrent first access must run the factory exactly once"
    );
}

#[test]
fn part6_api_client_normalises_slashes() {
    assert_eq!(ApiClient::new("https://a.test/").base_url(), "https://a.test");
    assert_eq!(ApiClient::new("https://a.test/").url_for("v1"), "https://a.test/v1");
    assert_eq!(ApiClient::new("https://a.test").url_for("/v1"), "https://a.test/v1");
}

fn entry(name: &str, size: u64) -> Entry {
    Entry { name: name.into(), size }
}

#[test]
fn part6_rpitit_catalog_sums_sizes_in_memory() {
    let cat = MemoryCatalog::new(vec![entry("a", 10), entry("b", 32)]);
    assert_eq!(cat.total_size(), 42);
    assert_eq!(cat.entries().count(), 2);
}

#[test]
fn part6_sharded_catalog_chains_shards_in_order() {
    let cat = ShardedCatalog::new(vec![
        MemoryCatalog::new(vec![entry("a", 1)]),
        MemoryCatalog::new(vec![]),
        MemoryCatalog::new(vec![entry("b", 2), entry("c", 3)]),
    ]);
    let names: Vec<&str> = cat.entries().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b", "c"]);
    assert_eq!(cat.total_size(), 6);
}

#[test]
fn part6_largest_prefers_first_of_equal_maxima() {
    let cat = ShardedCatalog::new(vec![
        MemoryCatalog::new(vec![entry("first", 9), entry("small", 1)]),
        MemoryCatalog::new(vec![entry("second", 9)]),
    ]);
    assert_eq!(largest(&cat).map(|e| e.name.as_str()), Some("first"));
    assert_eq!(largest(&MemoryCatalog::new(vec![])), None, "empty catalog has no largest entry");
}

#[test]
fn part6_chunk_reader_reuses_buffer_and_yields_short_tail() {
    let mut reader = ChunkReader::new(Cursor::new(b"abcdefg".to_vec()), NonZeroUsize::new(3).unwrap());
    let mut chunks: Vec<Vec<u8>> = Vec::new();
    while let Some(chunk) = reader.next() {
        chunks.push(chunk.unwrap().to_vec());
    }
    assert_eq!(chunks, vec![b"abc".to_vec(), b"def".to_vec(), b"g".to_vec()]);
}

#[test]
fn part6_chunk_reader_exact_multiple_and_empty_input_end_cleanly() {
    let mut exact = ChunkReader::new(Cursor::new(b"abcdef".to_vec()), NonZeroUsize::new(3).unwrap());
    let mut lens = Vec::new();
    while let Some(c) = exact.next() {
        lens.push(c.unwrap().len());
    }
    assert_eq!(lens, vec![3, 3], "no empty trailing chunk after an exact multiple");

    let mut empty = ChunkReader::new(Cursor::new(Vec::<u8>::new()), NonZeroUsize::new(4).unwrap());
    assert!(empty.next().is_none(), "empty input yields nothing");
}

/// Reader that reports `Interrupted` once before delivering its payload.
struct Flaky {
    interrupted: bool,
    data: Cursor<Vec<u8>>,
}

impl Read for Flaky {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        self.data.read(buf)
    }
}

#[test]
fn part6_chunk_reader_retries_interrupted_reads() {
    let flaky = Flaky { interrupted: false, data: Cursor::new(b"hi".to_vec()) };
    let mut reader = ChunkReader::new(flaky, NonZeroUsize::new(8).unwrap());
    let first = reader.next().expect("data must be delivered").unwrap().to_vec();
    assert_eq!(first, b"hi", "Interrupted must be retried, not surfaced");
    assert!(reader.next().is_none());
}

/// Reader that always fails with `PermissionDenied`.
struct Broken;

impl Read for Broken {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::PermissionDenied, "nope"))
    }
}

#[test]
fn part6_chunk_reader_surfaces_io_errors_with_their_kind() {
    let mut reader = ChunkReader::new(Broken, NonZeroUsize::new(4).unwrap());
    let err = reader
        .next()
        .expect("an error item, not end of stream")
        .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
}
