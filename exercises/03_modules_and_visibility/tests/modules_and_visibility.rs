use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use ex03_modules_and_visibility::catalog::{Sku, SkuError};
use ex03_modules_and_visibility::clock::{self, ClockOverride};
use ex03_modules_and_visibility::drop_order::{
    if_let_scrutinee, match_scrutinee, open_session, scoped_locals, shadowed, tail_temporary,
    Sink, Trace,
};
use ex03_modules_and_visibility::formula::{evaluate, parse, tokenize, Env, FormulaError, Token};
use ex03_modules_and_visibility::ids::{id_factory, IdError};
use ex03_modules_and_visibility::ledger::{Ledger, LedgerError};
use ex03_modules_and_visibility::registry::global::{boot_id, global};
use ex03_modules_and_visibility::registry::{FnPlugin, Plugin, PluginError, Registry, RegistryError};
use ex03_modules_and_visibility::report::{format_by_name, Csv, Format, Json, Table, TableError, Text};
use ex03_modules_and_visibility::store::Store;

/// A wall-clock time well after 2020, so any system-clock check is meaningful.
const SYSTEM_CLOCK_FLOOR: u64 = 1_600_000_000;

fn shout(s: &str) -> Result<String, PluginError> {
    Ok(s.to_uppercase())
}

type ShoutPlugin = FnPlugin<fn(&str) -> Result<String, PluginError>>;

fn upper(name: &str) -> ShoutPlugin {
    FnPlugin::new(name, shout as fn(&str) -> Result<String, PluginError>)
}

fn table(headers: &[&str], rows: &[&[&str]]) -> Table {
    Table::new(
        headers.iter().map(|h| h.to_string()).collect(),
        rows.iter()
            .map(|r| r.iter().map(|c| c.to_string()).collect())
            .collect(),
    )
    .expect("test table must be well formed")
}

fn env_with(cells: &[(&str, f64)]) -> Env {
    let mut env = Env::with_builtins();
    for (name, value) in cells {
        env.set_cell(name, *value).expect("valid cell name");
    }
    env
}

// ---------------------------------------------------------------------------
// Part 1: module tree, facades and id factories
// ---------------------------------------------------------------------------

#[test]
fn part1_prefix_and_number_are_zero_padded() {
    let mut next = id_factory("ord", 3).expect("valid factory");
    assert_eq!(next(), "ord-001", "first id starts at 1, padded to width 3");
    assert_eq!(next(), "ord-002", "second id increments by one");
}

#[test]
fn part1_factories_are_independent_counters() {
    let mut a = id_factory("a", 2).expect("valid factory");
    let mut b = id_factory("b", 2).expect("valid factory");
    assert_eq!(a(), "a-01");
    assert_eq!(a(), "a-02");
    assert_eq!(b(), "b-01", "a second factory must start its own count");
}

#[test]
fn part1_numbers_wider_than_width_are_not_truncated() {
    let mut next = id_factory("x", 2).expect("valid factory");
    let ids: Vec<String> = (0..101).map(|_| next()).collect();
    assert_eq!(ids[98], "x-99");
    assert_eq!(ids[99], "x-100", "width is a minimum, never a truncation");
    assert_eq!(ids[100], "x-101");
}

#[test]
fn part1_empty_prefix_is_rejected() {
    assert_eq!(id_factory("", 3).err(), Some(IdError::EmptyPrefix));
}

#[test]
fn part1_invalid_prefix_characters_are_rejected() {
    assert_eq!(
        id_factory("a b", 3).err(),
        Some(IdError::InvalidPrefix("a b".to_string()))
    );
    assert!(id_factory("ok_prefix-1", 3).is_ok(), "underscore and dash are allowed");
}

#[test]
fn part1_zero_width_is_rejected() {
    assert_eq!(id_factory("p", 0).err(), Some(IdError::ZeroWidth));
}

#[test]
fn part1_validation_order_is_empty_then_chars_then_width() {
    assert_eq!(id_factory("", 0).err(), Some(IdError::EmptyPrefix));
    assert_eq!(
        id_factory("a b", 0).err(),
        Some(IdError::InvalidPrefix("a b".to_string())),
        "invalid characters are reported before a zero width"
    );
}

#[test]
fn part1_factory_is_send_across_threads() {
    let next = id_factory("w", 1).expect("valid factory");
    let got = thread::spawn(next).join().expect("thread must not panic");
    assert_eq!(got, "w-1");
}

#[test]
fn part1_prelude_reexports_common_types() {
    use ex03_modules_and_visibility::prelude::*;
    let mut next: IdFactory = id_factory("pre", 1).expect("valid factory");
    assert_eq!(next(), "pre-1", "prelude must re-export id_factory and IdFactory");
    let sku: Sku = "ab-1".parse().expect("valid sku via prelude");
    assert_eq!(sku.as_str(), "AB-1");
}

// ---------------------------------------------------------------------------
// Part 2: private fields, constructors and invariants
// ---------------------------------------------------------------------------

#[test]
fn part2_sku_is_normalised_to_uppercase() {
    let sku = Sku::new("ab-12").expect("valid sku");
    assert_eq!(sku.as_str(), "AB-12");
}

#[test]
fn part2_sku_length_bounds_are_inclusive() {
    assert_eq!(Sku::new("ab").err(), Some(SkuError::TooShort(2)));
    assert!(Sku::new("abc").is_ok(), "three characters is the minimum");
    assert!(Sku::new("abcdefghijkl").is_ok(), "twelve characters is the maximum");
    assert_eq!(Sku::new("abcdefghijklm").err(), Some(SkuError::TooLong(13)));
}

#[test]
fn part2_sku_invalid_character_is_reported() {
    assert_eq!(Sku::new("ab$c").err(), Some(SkuError::InvalidChar('$')));
}

#[test]
fn part2_sku_edge_and_double_dash_rules() {
    assert_eq!(Sku::new("-abc").err(), Some(SkuError::EdgeDash));
    assert_eq!(Sku::new("abc-").err(), Some(SkuError::EdgeDash));
    assert_eq!(Sku::new("ab--c").err(), Some(SkuError::DoubleDash));
}

#[test]
fn part2_sku_from_str_and_display_agree() {
    let sku: Sku = "abc-1".parse().expect("valid sku");
    assert_eq!(sku.to_string(), "ABC-1");
}

#[test]
fn part2_sku_equality_is_after_normalisation() {
    assert_eq!(Sku::new("abc").expect("valid"), Sku::new("ABC").expect("valid"));
}

#[test]
fn part2_sku_as_ref_str_is_usable_generically() {
    fn shout<S: AsRef<str>>(s: S) -> String {
        s.as_ref().to_lowercase()
    }
    let sku = Sku::new("xyz-9").expect("valid sku");
    assert_eq!(shout(&sku), "xyz-9");
}

#[test]
fn part2_ledger_opening_balance_must_not_be_negative() {
    assert_eq!(Ledger::new(-1).err(), Some(LedgerError::NegativeOpening(-1)));
    assert_eq!(Ledger::new(0).expect("zero is allowed").balance(), 0);
}

#[test]
fn part2_ledger_deposit_returns_new_balance() {
    let mut ledger = Ledger::new(100).expect("valid opening");
    assert_eq!(ledger.deposit(50).expect("valid deposit"), 150);
    assert_eq!(ledger.balance(), 150);
}

#[test]
fn part2_ledger_rejects_non_positive_amounts() {
    let mut ledger = Ledger::new(10).expect("valid opening");
    assert_eq!(ledger.deposit(0).err(), Some(LedgerError::NonPositiveAmount(0)));
    assert_eq!(ledger.withdraw(-5).err(), Some(LedgerError::NonPositiveAmount(-5)));
}

#[test]
fn part2_ledger_withdraw_cannot_overdraw() {
    let mut ledger = Ledger::new(10).expect("valid opening");
    assert_eq!(
        ledger.withdraw(11).err(),
        Some(LedgerError::InsufficientFunds {
            balance: 10,
            requested: 11
        })
    );
    assert_eq!(ledger.withdraw(10).expect("exact withdrawal"), 0);
}

#[test]
fn part2_ledger_overflow_is_an_error_and_leaves_balance_unchanged() {
    let mut ledger = Ledger::new(i64::MAX).expect("valid opening");
    assert_eq!(ledger.deposit(1).err(), Some(LedgerError::Overflow));
    assert_eq!(ledger.balance(), i64::MAX, "failed deposit must not change the balance");
}

// ---------------------------------------------------------------------------
// Part 3: sealed traits and report formats
// ---------------------------------------------------------------------------

#[test]
fn part3_table_requires_at_least_one_column() {
    assert_eq!(Table::new(vec![], vec![]).err(), Some(TableError::NoColumns));
}

#[test]
fn part3_ragged_row_reports_its_index_and_widths() {
    let result = Table::new(
        vec!["a".into(), "b".into()],
        vec![vec!["1".into(), "2".into()], vec!["3".into()]],
    );
    assert_eq!(
        result.err(),
        Some(TableError::RaggedRow {
            row: 1,
            expected: 2,
            got: 1
        })
    );
}

#[test]
fn part3_table_exposes_headers_and_rows() {
    let t = table(&["item", "qty"], &[&["apple", "3"]]);
    assert_eq!(t.headers(), ["item".to_string(), "qty".to_string()]);
    assert_eq!(t.rows().len(), 1);
    assert_eq!(t.rows()[0][0], "apple");
}

#[test]
fn part3_csv_basic_output() {
    let t = table(&["item", "qty"], &[&["apple", "3"], &["kiwi", "12"]]);
    assert_eq!(Csv.render(&t), "item,qty\napple,3\nkiwi,12\n");
}

#[test]
fn part3_csv_quotes_and_doubles_special_characters() {
    let t = table(&["v"], &[&["a,\"b\""], &["x\ny"]]);
    assert_eq!(Csv.render(&t), "v\n\"a,\"\"b\"\"\"\n\"x\ny\"\n");
}

#[test]
fn part3_json_is_compact_and_escapes_controls() {
    let t = table(&["k"], &[&["q\"\\\n\t\u{1}"]]);
    assert_eq!(Json.render(&t), "[{\"k\":\"q\\\"\\\\\\n\\t\\u0001\"}]");
}

#[test]
fn part3_json_empty_table_is_empty_array() {
    let t = table(&["k"], &[]);
    assert_eq!(Json.render(&t), "[]");
}

#[test]
fn part3_json_objects_keep_header_order() {
    let t = table(&["item", "qty"], &[&["apple", "3"]]);
    assert_eq!(Json.render(&t), "[{\"item\":\"apple\",\"qty\":\"3\"}]");
}

#[test]
fn part3_text_aligns_columns_and_trims_trailing_space() {
    let t = table(&["item", "qty"], &[&["apple", "3"], &["kiwi", "12"]]);
    assert_eq!(
        Text.render(&t),
        "item  | qty\n------+----\napple | 3\nkiwi  | 12\n"
    );
}

#[test]
fn part3_text_width_counts_characters_not_bytes() {
    let t = table(&["x"], &[&["héllo"]]);
    assert_eq!(Text.render(&t), "x\n-----\nhéllo\n", "five characters wide");
}

#[test]
fn part3_text_header_only_table() {
    let t = table(&["a"], &[]);
    assert_eq!(Text.render(&t), "a\n-\n");
}

#[test]
fn part3_format_names_are_lowercase_identifiers() {
    assert_eq!(Csv.name(), "csv");
    assert_eq!(Json.name(), "json");
    assert_eq!(Text.name(), "text");
}

#[test]
fn part3_format_by_name_ignores_case_and_rejects_unknown() {
    assert_eq!(format_by_name("CSV").expect("csv exists").name(), "csv");
    assert_eq!(format_by_name("Json").expect("json exists").name(), "json");
    assert!(format_by_name("xml").is_none(), "unknown names return None");
}

#[test]
fn part3_dyn_format_renders_like_the_concrete_type() {
    let t = table(&["n"], &[&["1"]]);
    let boxed = format_by_name("csv").expect("csv exists");
    assert_eq!(boxed.render(&t), Csv.render(&t));
}

// ---------------------------------------------------------------------------
// Part 4: block scope, shadowing and drop order
// ---------------------------------------------------------------------------

#[test]
fn part4_trace_logs_on_scope_exit() {
    let sink = Sink::new();
    {
        let _t = Trace::new(&sink, "x");
        assert!(sink.events().is_empty(), "nothing is logged while alive");
    }
    assert_eq!(sink.events(), ["drop x"]);
}

#[test]
fn part4_trace_label_and_status_passthrough() {
    let sink = Sink::new();
    let t = Trace::new(&sink, "probe");
    assert_eq!(t.label(), "probe");
    assert_eq!(t.status(404), 404);
    assert_eq!(t.maybe(true), Some(42));
    assert_eq!(t.maybe(false), None);
}

#[test]
fn part4_session_fields_drop_in_declaration_order() {
    let sink = Sink::new();
    {
        let _s = open_session(&sink, "1");
    }
    assert_eq!(sink.events(), ["drop conn:1", "drop cache:1", "drop audit:1"]);
}

#[test]
fn part4_session_labels_follow_declaration_order() {
    let sink = Sink::new();
    let s = open_session(&sink, "9");
    assert_eq!(s.labels(), ["conn:9", "cache:9", "audit:9"]);
}

#[test]
fn part4_scoped_locals_drop_inner_then_outer_reverse() {
    let sink = Sink::new();
    assert_eq!(scoped_locals(&sink), 3);
    assert_eq!(sink.events(), ["drop inner", "drop c", "drop a"]);
}

#[test]
fn part4_shadowing_keeps_first_binding_alive() {
    let sink = Sink::new();
    assert_eq!(shadowed(&sink), 6);
    assert_eq!(
        sink.events(),
        ["end of scope", "drop second", "drop first"],
        "shadowing does not drop the earlier binding early"
    );
}

#[test]
fn part4_tail_expression_temporary_drops_before_locals() {
    let sink = Sink::new();
    assert_eq!(tail_temporary(&sink), 4);
    assert_eq!(sink.events(), ["drop temp", "drop local"]);
}

#[test]
fn part4_match_scrutinee_lives_through_the_arm() {
    let sink = Sink::new();
    match_scrutinee(&sink, 200);
    assert_eq!(sink.events(), ["arm: ok", "drop probe"]);
}

#[test]
fn part4_match_err_arm_and_scrutinee_drop_after_arm() {
    let sink = Sink::new();
    match_scrutinee(&sink, 500);
    assert_eq!(sink.events(), ["arm: err", "drop probe"]);
}

#[test]
fn part4_if_let_present_branch_drops_guard_after_body() {
    let sink = Sink::new();
    assert_eq!(if_let_scrutinee(&sink, true), 7);
    assert_eq!(sink.events(), ["then 7", "drop guard"]);
}

#[test]
fn part4_if_let_absent_branch_drops_guard_before_else() {
    let sink = Sink::new();
    assert_eq!(if_let_scrutinee(&sink, false), 0);
    assert_eq!(sink.events(), ["drop guard", "else"]);
}

#[test]
fn part4_sink_clones_share_one_log() {
    let sink = Sink::new();
    let alias = sink.clone();
    alias.push("from alias");
    sink.push("from original");
    assert_eq!(sink.events(), ["from alias", "from original"]);
}

// ---------------------------------------------------------------------------
// Part 5: RAII guards and thread-local state
// ---------------------------------------------------------------------------

#[test]
fn part5_commit_applies_staged_writes() {
    let mut store = Store::new();
    let mut tx = store.begin();
    tx.put("a", "1");
    assert_eq!(tx.commit(), 1, "commit returns the number of staged keys");
    assert_eq!(store.get("a"), Some("1"));
    assert_eq!(store.commit_count(), 1);
}

#[test]
fn part5_dropping_without_commit_is_a_rollback() {
    let mut store = Store::new();
    {
        let mut tx = store.begin();
        tx.put("a", "1");
    }
    assert!(store.is_empty(), "uncommitted writes must not be visible");
    assert_eq!(store.rollback_count(), 1);
    assert_eq!(store.commit_count(), 0);
}

#[test]
fn part5_explicit_rollback_is_counted() {
    let mut store = Store::new();
    let mut tx = store.begin();
    tx.put("a", "1");
    tx.rollback();
    assert_eq!(store.rollback_count(), 1);
    assert_eq!(store.get("a"), None);
}

#[test]
fn part5_last_write_wins_per_key() {
    let mut store = Store::new();
    let mut tx = store.begin();
    tx.put("a", "1");
    tx.put("a", "2");
    assert_eq!(tx.commit(), 1, "one distinct key is staged");
    assert_eq!(store.get("a"), Some("2"));
}

#[test]
fn part5_transaction_reads_its_own_staged_view() {
    let mut store = Store::new();
    let mut tx = store.begin();
    tx.put("k", "staged");
    assert_eq!(tx.get("k"), Some("staged"));
    tx.rollback();
    assert_eq!(store.get("k"), None);
}

#[test]
fn part5_staged_delete_shadows_committed_value() {
    let mut store = Store::new();
    {
        let mut setup = store.begin();
        setup.put("a", "1");
        setup.commit();
    }
    let mut tx = store.begin();
    tx.delete("a");
    assert_eq!(tx.get("a"), None, "a staged delete hides the committed value");
    assert_eq!(tx.commit(), 1);
    assert_eq!(store.get("a"), None);
    assert_eq!(store.len(), 0);
}

#[test]
fn part5_rollback_keeps_committed_data() {
    let mut store = Store::new();
    {
        let mut setup = store.begin();
        setup.put("keep", "yes");
        setup.commit();
    }
    {
        let mut tx = store.begin();
        tx.put("keep", "no");
    }
    assert_eq!(store.get("keep"), Some("yes"));
    assert_eq!(store.len(), 1);
}

#[test]
fn part5_freeze_overrides_now_on_this_thread() {
    let guard = ClockOverride::freeze(1_000);
    assert_eq!(clock::now_secs(), 1_000);
    assert_eq!(clock::override_depth(), 1);
    drop(guard);
}

#[test]
fn part5_dropping_guard_restores_system_clock() {
    {
        let _guard = ClockOverride::freeze(5);
        assert_eq!(clock::now_secs(), 5);
    }
    assert!(clock::now_secs() > SYSTEM_CLOCK_FLOOR, "system clock must be restored");
    assert_eq!(clock::override_depth(), 0);
}

#[test]
fn part5_nested_overrides_innermost_wins() {
    let outer = ClockOverride::freeze(10);
    {
        let inner = ClockOverride::freeze(20);
        assert_eq!(clock::now_secs(), 20);
        assert_eq!(clock::override_depth(), 2);
        drop(inner);
    }
    assert_eq!(clock::now_secs(), 10, "outer override is active again");
    drop(outer);
}

#[test]
fn part5_overrides_do_not_leak_to_other_threads() {
    let _guard = ClockOverride::freeze(5);
    let (now, depth) = thread::spawn(|| (clock::now_secs(), clock::override_depth()))
        .join()
        .expect("thread must not panic");
    assert!(now > SYSTEM_CLOCK_FLOOR, "a new thread sees the system clock");
    assert_eq!(depth, 0);
}

#[test]
fn part5_system_clock_is_plausible() {
    assert!(clock::now_secs() > SYSTEM_CLOCK_FLOOR);
}

// ---------------------------------------------------------------------------
// Part 6: statics versus explicit context
// ---------------------------------------------------------------------------

#[test]
fn part6_register_then_get_and_run() {
    let reg = Registry::new();
    reg.register(upper("echo")).expect("valid registration");
    let plugin = reg.get("echo").expect("plugin is registered");
    assert_eq!(plugin.name(), "echo");
    assert_eq!(plugin.run("hi"), Ok("HI".to_string()));
}

#[test]
fn part6_name_validation_accepts_the_documented_shape() {
    let reg = Registry::new();
    assert!(reg.register(upper("a")).is_ok(), "single letter is valid");
    assert!(reg.register(upper("a-b_c9")).is_ok(), "dashes, underscores and digits");
    let thirty_two = format!("a{}", "b".repeat(31));
    assert!(reg.register(upper(&thirty_two)).is_ok(), "32 characters is the maximum");
}

#[test]
fn part6_name_validation_rejects_bad_names() {
    let reg = Registry::new();
    for bad in ["Echo", "", "1abc", "a b", "-x"] {
        assert_eq!(
            reg.register(upper(bad)),
            Err(RegistryError::InvalidName(bad.to_string())),
            "{bad:?} must be rejected"
        );
    }
    let too_long = format!("a{}", "b".repeat(32));
    assert_eq!(
        reg.register(upper(&too_long)),
        Err(RegistryError::InvalidName(too_long.clone())),
        "33 characters is too long"
    );
}

#[test]
fn part6_duplicate_names_are_rejected() {
    let reg = Registry::new();
    reg.register(upper("echo")).expect("first registration");
    assert_eq!(
        reg.register(upper("echo")),
        Err(RegistryError::Duplicate("echo".to_string()))
    );
}

#[test]
fn part6_unregister_removes_and_reports_missing() {
    let reg = Registry::new();
    reg.register(upper("echo")).expect("registered");
    assert_eq!(reg.unregister("echo"), Ok(()));
    assert!(reg.get("echo").is_none());
    assert_eq!(
        reg.unregister("echo"),
        Err(RegistryError::NotFound("echo".to_string()))
    );
}

#[test]
fn part6_names_are_sorted_and_counted() {
    let reg = Registry::new();
    for name in ["zeta", "alpha", "mid"] {
        reg.register(upper(name)).expect("valid registration");
    }
    assert_eq!(reg.names(), ["alpha", "mid", "zeta"]);
    assert_eq!(reg.len(), 3);
    assert!(!reg.is_empty());
    reg.clear();
    assert!(reg.is_empty());
}

#[test]
fn part6_run_all_is_name_ordered_and_keeps_errors() {
    struct Failing;
    impl Plugin for Failing {
        fn name(&self) -> &str {
            "bad"
        }
        fn run(&self, _input: &str) -> Result<String, PluginError> {
            Err(PluginError::Failed("boom".to_string()))
        }
    }

    let reg = Registry::new();
    reg.register(upper("good")).expect("valid");
    reg.register(Failing).expect("valid");
    let results = reg.run_all("x");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0], ("bad".to_string(), Err(PluginError::Failed("boom".to_string()))));
    assert_eq!(results[1], ("good".to_string(), Ok("X".to_string())));
}

#[test]
fn part6_plugin_may_reenter_the_registry_during_run_all() {
    struct Probe {
        reg: Arc<Registry>,
    }
    impl Plugin for Probe {
        fn name(&self) -> &str {
            "probe"
        }
        fn run(&self, _input: &str) -> Result<String, PluginError> {
            Ok(self.reg.len().to_string())
        }
    }

    let reg = Arc::new(Registry::new());
    reg.register(Probe { reg: Arc::clone(&reg) }).expect("valid");
    let (tx, rx) = mpsc::channel();
    let shared = Arc::clone(&reg);
    thread::spawn(move || {
        let _ = tx.send(shared.run_all("ignored"));
    });
    let results = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("run_all must not deadlock when a plugin re-enters the registry");
    assert_eq!(results, vec![("probe".to_string(), Ok("1".to_string()))]);
}

#[test]
fn part6_registry_is_usable_from_many_threads() {
    let reg = Arc::new(Registry::new());
    let handles: Vec<_> = (0..4)
        .map(|i| {
            let reg = Arc::clone(&reg);
            thread::spawn(move || reg.register(upper(&format!("worker-{i}"))))
        })
        .collect();
    for handle in handles {
        handle
            .join()
            .expect("thread must not panic")
            .expect("each name is unique");
    }
    assert_eq!(reg.len(), 4);
}

#[test]
fn part6_plugin_failure_is_passed_through_unchanged() {
    let reg = Registry::new();
    reg.register(FnPlugin::new("flaky", |_: &str| -> Result<String, PluginError> {
        Err(PluginError::Failed("disk gone".into()))
    }))
    .expect("valid");
    let out = reg.get("flaky").expect("registered").run("x");
    assert_eq!(out, Err(PluginError::Failed("disk gone".to_string())));
}

#[test]
fn part6_global_registry_is_one_instance() {
    assert!(std::ptr::eq(global(), global()), "global() must return the same registry");
    global().register(upper("global-probe")).expect("unique name");
    assert!(global().names().contains(&"global-probe".to_string()));
    global().unregister("global-probe").expect("cleanup");
}

#[test]
fn part6_boot_id_is_stable_and_nonzero() {
    let first = boot_id();
    assert_ne!(first, 0, "boot id must be nonzero");
    assert_eq!(boot_id(), first, "boot id is chosen once per process");
}

// ---------------------------------------------------------------------------
// Part 7: formula evaluator
// ---------------------------------------------------------------------------

#[test]
fn part7_tokenize_produces_numbers_and_operators() {
    assert_eq!(
        tokenize("1+2").expect("valid"),
        vec![Token::Number(1.0), Token::Plus, Token::Number(2.0)]
    );
}

#[test]
fn part7_tokenize_uppercases_identifiers() {
    assert_eq!(
        tokenize("a1 + sum").expect("valid"),
        vec![Token::Ident("A1".into()), Token::Plus, Token::Ident("SUM".into())]
    );
}

#[test]
fn part7_unexpected_character_reports_byte_position() {
    assert_eq!(
        tokenize("1 + #").err(),
        Some(FormulaError::UnexpectedChar { ch: '#', pos: 4 })
    );
}

#[test]
fn part7_malformed_number_is_an_error() {
    assert_eq!(
        tokenize("1.2.3").err(),
        Some(FormulaError::MalformedNumber("1.2.3".into()))
    );
}

#[test]
fn part7_multiplication_binds_tighter_than_addition() {
    assert_eq!(evaluate("1+2*3", &Env::new()), Ok(7.0));
}

#[test]
fn part7_pow_is_right_associative() {
    assert_eq!(evaluate("2^3^2", &Env::new()), Ok(512.0));
}

#[test]
fn part7_prefix_minus_binds_tighter_than_pow() {
    assert_eq!(evaluate("-2^2", &Env::new()), Ok(4.0));
}

#[test]
fn part7_remainder_follows_rust_semantics() {
    assert_eq!(evaluate("-7 % 3", &Env::new()), Ok(-1.0));
}

#[test]
fn part7_leading_equals_is_optional() {
    assert_eq!(evaluate("=1+1", &Env::new()), Ok(2.0));
    assert_eq!(evaluate("1+1", &Env::new()), Ok(2.0));
}

#[test]
fn part7_cell_names_are_case_insensitive() {
    let env = env_with(&[("a1", 5.0)]);
    assert_eq!(env.cell("A1"), Some(5.0));
    assert_eq!(evaluate("A1*2", &env), Ok(10.0));
}

#[test]
fn part7_unknown_cell_is_reported() {
    assert_eq!(
        evaluate("Z9+1", &Env::new()),
        Err(FormulaError::UnknownCell("Z9".into()))
    );
}

#[test]
fn part7_unknown_function_is_reported_before_arguments_run() {
    assert_eq!(
        evaluate("NOPE(Z9)", &Env::new()),
        Err(FormulaError::UnknownFunction("NOPE".into())),
        "function lookup happens before argument evaluation"
    );
}

#[test]
fn part7_arity_errors_describe_expectation() {
    let env = Env::with_builtins();
    assert_eq!(
        evaluate("ABS(1,2)", &env),
        Err(FormulaError::Arity {
            name: "ABS".into(),
            expected: "1",
            got: 2
        })
    );
    assert_eq!(
        evaluate("MIN()", &env),
        Err(FormulaError::Arity {
            name: "MIN".into(),
            expected: "at least 1",
            got: 0
        })
    );
}

#[test]
fn part7_builtin_functions_compute() {
    let env = Env::with_builtins();
    assert_eq!(evaluate("ABS(-3)", &env), Ok(3.0));
    assert_eq!(evaluate("MIN(3,1,2)", &env), Ok(1.0));
    assert_eq!(evaluate("MAX(3,1,2)", &env), Ok(3.0));
    assert_eq!(evaluate("SUM()", &env), Ok(0.0));
    assert_eq!(evaluate("SUM(1,2,3)", &env), Ok(6.0));
}

#[test]
fn part7_round_is_half_away_from_zero() {
    let env = Env::with_builtins();
    assert_eq!(evaluate("ROUND(2.5)", &env), Ok(3.0));
    assert_eq!(evaluate("ROUND(-2.5)", &env), Ok(-3.0));
    assert_eq!(evaluate("ROUND(1.2345, 2)", &env), Ok(1.23));
}

#[test]
fn part7_division_and_remainder_by_zero_are_errors() {
    assert_eq!(evaluate("1/0", &Env::new()), Err(FormulaError::DivisionByZero));
    assert_eq!(evaluate("1%0", &Env::new()), Err(FormulaError::DivisionByZero));
}

#[test]
fn part7_non_finite_result_is_an_error() {
    assert_eq!(evaluate("2^1024", &Env::new()), Err(FormulaError::NotFinite));
}

#[test]
fn part7_references_are_cells_only_sorted_and_deduplicated() {
    let expr = parse("A1 + SUM(B2, a1) * C3").expect("valid formula");
    let expected: BTreeSet<String> = ["A1", "B2", "C3"].iter().map(|s| s.to_string()).collect();
    assert_eq!(expr.references(), expected);
}

#[test]
fn part7_set_cell_rejects_invalid_names() {
    let mut env = Env::new();
    assert_eq!(
        env.set_cell("1x", 1.0),
        Err(FormulaError::InvalidName("1x".into()))
    );
}

#[test]
fn part7_incomplete_expression_is_an_error() {
    assert!(parse("1 +").is_err(), "a dangling operator must not parse");
}

#[test]
fn part7_env_new_has_no_builtins() {
    assert_eq!(
        evaluate("ABS(-1)", &Env::new()),
        Err(FormulaError::UnknownFunction("ABS".into())),
        "Env::new is empty; Env::with_builtins adds functions"
    );
}
