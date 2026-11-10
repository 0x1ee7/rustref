//! Black-box tests for `ex06_functional_patterns`. The README is the spec.
//! Test names are prefixed `partN_` so `cargo test -p ex06_functional_patterns part3` filters.

use std::collections::BTreeMap;

use ex06_functional_patterns::cart::{
    reduce, run_actions, subtotal_cents, total_cents, Action, Cart, CartError, LineItem,
};
use ex06_functional_patterns::compose::{compose, pipeline};
use ex06_functional_patterns::config::{
    parse_bool, parse_config, parse_optional_port, parse_port, parse_ports, Config, ConfigError,
};
use ex06_functional_patterns::etl::{
    build_report, clean_sales, distinct_regions, format_cents, load_csv, parse_cents, parse_sale,
    ranked, revenue_cents, totals_by, EtlError, RawRow, RowError, Sale,
};
use ex06_functional_patterns::ledger::{
    apply, decide_withdraw, replay, replay_from, snapshots, total_deposited, Account, Event,
    LedgerError, Status,
};
use ex06_functional_patterns::validate::{
    collect_all, combine, validate_signup, Signup, ValidationError,
};
use proptest::prelude::*;
use serde_json::json;

// ---------- helpers ----------

fn line(sku: &str, name: &str, price: u64, qty: u32) -> LineItem {
    LineItem {
        sku: sku.into(),
        name: name.into(),
        unit_price_cents: price,
        quantity: qty,
    }
}

fn add(sku: &str, name: &str, price: u64, qty: u32) -> Action {
    Action::AddItem {
        sku: sku.into(),
        name: name.into(),
        unit_price_cents: price,
        quantity: qty,
    }
}

fn opened(id: &str, owner: &str) -> Event {
    Event::Opened {
        account_id: id.into(),
        owner: owner.into(),
    }
}

fn account(balance: u64, status: Status) -> Account {
    Account {
        account_id: "acc-1".into(),
        owner: "Ada".into(),
        balance_cents: balance,
        status,
    }
}

fn open_account(balance: u64) -> Account {
    account(balance, Status::Open)
}

fn row(region: &str, product: &str, units: &str, price: &str, sold_on: &str) -> RawRow {
    RawRow {
        region: region.into(),
        product: product.into(),
        units: units.into(),
        unit_price: price.into(),
        sold_on: sold_on.into(),
    }
}

fn sale(region: &str, units: u32, price: u64) -> Sale {
    Sale {
        region: region.into(),
        product: "p".into(),
        units,
        unit_price_cents: price,
        sold_on: jiff::civil::date(2024, 1, 1),
    }
}

fn verr(path: &str, message: &str) -> ValidationError {
    ValidationError {
        path: path.into(),
        message: message.into(),
    }
}

fn valid_config_text() -> &'static str {
    "host = example.com\nport = 8080\n"
}

// ---------- Part 1 · enums as ADTs and the cart reducer ----------

#[test]
fn part1_add_new_sku_appends_line() {
    let cart = reduce(&Cart::default(), &add("A1", "Mug", 1200, 2)).expect("add should succeed");
    assert_eq!(
        cart.items,
        vec![line("A1", "Mug", 1200, 2)],
        "a new sku should append one line"
    );
    assert_eq!(cart.coupon, None, "adding items must not invent a coupon");
}

#[test]
fn part1_add_existing_sku_merges_quantity_and_keeps_position_name_and_price() {
    let cart = Cart {
        items: vec![line("A1", "Mug", 1200, 2), line("B2", "Tea", 500, 1)],
        coupon: None,
    };
    let next = reduce(&cart, &add("A1", "Other name", 999, 3)).expect("merge should succeed");
    assert_eq!(
        next.items,
        vec![line("A1", "Mug", 1200, 5), line("B2", "Tea", 500, 1)],
        "existing line keeps its position, name and price; quantities add up"
    );
}

#[test]
fn part1_add_zero_quantity_is_rejected() {
    assert_eq!(
        reduce(&Cart::default(), &add("A1", "Mug", 1200, 0)),
        Err(CartError::ZeroQuantity),
        "quantity 0 must be rejected"
    );
}

#[test]
fn part1_add_overflowing_quantity_reports_overflow() {
    let cart = Cart {
        items: vec![line("A1", "Mug", 1, u32::MAX)],
        coupon: None,
    };
    assert_eq!(
        reduce(&cart, &add("A1", "Mug", 1, 1)),
        Err(CartError::Overflow),
        "merging past u32::MAX must report Overflow, not wrap"
    );
}

#[test]
fn part1_reducer_leaves_previous_state_and_action_untouched() {
    let before = Cart {
        items: vec![line("A1", "Mug", 1200, 1)],
        coupon: Some("SAVE10".into()),
    };
    let snapshot = before.clone();
    let action = add("B2", "Tea", 500, 1);
    let action_snapshot = action.clone();

    let after = reduce(&before, &action).expect("add should succeed");

    assert_eq!(before, snapshot, "reducer must not mutate the previous state");
    assert_eq!(action, action_snapshot, "reducer must not consume or mutate the action");
    assert_eq!(after.items.len(), 2, "the new state should contain the added line");
    assert_eq!(
        after.coupon.as_deref(),
        Some("SAVE10"),
        "item actions must keep the coupon"
    );
}

#[test]
fn part1_remove_and_set_quantity_edge_cases() {
    let cart = Cart {
        items: vec![line("A1", "Mug", 1200, 2), line("B2", "Tea", 500, 1)],
        coupon: None,
    };
    let unchanged = reduce(
        &cart,
        &Action::RemoveItem {
            sku: "ZZ".into(),
        },
    )
    .expect("removing an unknown sku is not an error");
    assert_eq!(unchanged, cart, "unknown sku in RemoveItem is a no-op");

    let removed = reduce(
        &cart,
        &Action::SetQuantity {
            sku: "A1".into(),
            quantity: 0,
        },
    )
    .expect("quantity 0 is valid and removes the line");
    assert_eq!(
        removed.items,
        vec![line("B2", "Tea", 500, 1)],
        "SetQuantity(0) removes only that line"
    );

    let err = reduce(
        &cart,
        &Action::SetQuantity {
            sku: "ZZ".into(),
            quantity: 3,
        },
    );
    assert_eq!(
        err,
        Err(CartError::UnknownSku("ZZ".into())),
        "SetQuantity on an unknown sku is an error"
    );
}

#[test]
fn part1_apply_coupon_validates_code_and_none_clears() {
    let cart = Cart::default();
    assert_eq!(
        reduce(&cart, &Action::ApplyCoupon(Some("NOPE".into()))),
        Err(CartError::UnknownCoupon("NOPE".into())),
        "unknown coupon codes must be rejected"
    );
    assert_eq!(
        reduce(&cart, &Action::ApplyCoupon(Some("save10".into()))),
        Err(CartError::UnknownCoupon("save10".into())),
        "coupon matching is case-sensitive"
    );

    let with = reduce(&cart, &Action::ApplyCoupon(Some("HALF".into()))).unwrap();
    assert_eq!(with.coupon.as_deref(), Some("HALF"), "a known coupon is applied");

    let without = reduce(&with, &Action::ApplyCoupon(None)).unwrap();
    assert_eq!(without.coupon, None, "ApplyCoupon(None) clears the coupon");
}

#[test]
fn part1_clear_cart_resets_items_and_coupon() {
    let cart = Cart {
        items: vec![line("A1", "Mug", 1200, 2)],
        coupon: Some("SAVE10".into()),
    };
    let cleared = reduce(&cart, &Action::ClearCart).expect("ClearCart cannot fail");
    assert_eq!(cleared, Cart::default(), "ClearCart returns the default cart");
}

// ---------- Part 2 · composition and closures ----------

#[test]
fn part2_compose_applies_right_to_left() {
    let f = compose(|x: i32| x + 1, |x: i32| x * 2);
    assert_eq!(f(3), 7, "compose(f, g)(x) must equal f(g(x)) = (3*2)+1");
}

#[test]
fn part2_compose_may_change_types_between_steps() {
    let f = compose(|n: usize| format!("{n} items"), |s: String| s.len());
    assert_eq!(f("abcd".to_string()), "4 items", "B and C may differ from A");
}

#[test]
fn part2_pipeline_applies_steps_left_to_right() {
    let steps: Vec<Box<dyn Fn(i64) -> i64>> = vec![Box::new(|x: i64| x + 1), Box::new(|x: i64| x * 2)];
    assert_eq!(pipeline(steps)(3), 8, "(3 + 1) * 2 with left-to-right order");
}

#[test]
fn part2_pipeline_without_steps_is_identity() {
    let p = pipeline::<String>(Vec::new());
    assert_eq!(p("unchanged".to_string()), "unchanged", "empty pipeline is identity");
}

#[test]
fn part2_pipeline_owns_its_captured_config_and_is_reusable() {
    let factor = 3;
    let steps: Vec<Box<dyn Fn(i64) -> i64>> = vec![Box::new(move |x: i64| x * factor)];
    let p = pipeline(steps);
    assert_eq!(p(2), 6, "first call uses the captured factor");
    assert_eq!(p(5), 15, "the pipeline must remain callable after the first call");
}

// ---------- Part 3 · patterns: event-sourced account ----------

#[test]
fn part3_opened_creates_zero_balance_open_account() {
    let acc = apply(None, &opened("acc-1", "Ada")).expect("opening a new account should succeed");
    assert_eq!(acc, open_account(0), "a fresh account has zero balance and is open");
}

#[test]
fn part3_opening_twice_is_already_opened() {
    let acc = open_account(0);
    assert_eq!(
        apply(Some(&acc), &opened("acc-1", "Ada")),
        Err(LedgerError::AlreadyOpened),
        "Opened on an existing account must fail"
    );
}

#[test]
fn part3_events_before_open_are_not_opened() {
    assert_eq!(
        apply(None, &Event::Deposited { amount_cents: 5 }),
        Err(LedgerError::NotOpened),
        "deposit before open"
    );
    assert_eq!(
        apply(None, &Event::Frozen),
        Err(LedgerError::NotOpened),
        "freeze before open"
    );
}

#[test]
fn part3_deposit_adds_to_balance() {
    let acc = apply(Some(&open_account(100)), &Event::Deposited { amount_cents: 50 })
        .expect("deposit should succeed");
    assert_eq!(acc.balance_cents, 150, "deposit adds to the balance");
}

#[test]
fn part3_zero_amounts_are_non_positive() {
    let acc = open_account(100);
    assert_eq!(
        apply(Some(&acc), &Event::Deposited { amount_cents: 0 }),
        Err(LedgerError::NonPositiveAmount),
        "zero deposit"
    );
    assert_eq!(
        apply(Some(&acc), &Event::Withdrawn { amount_cents: 0 }),
        Err(LedgerError::NonPositiveAmount),
        "zero withdrawal"
    );
}

#[test]
fn part3_frozen_account_rejects_money_movement_but_freeze_is_idempotent() {
    let frozen = account(100, Status::Frozen);
    assert_eq!(
        apply(Some(&frozen), &Event::Deposited { amount_cents: 1 }),
        Err(LedgerError::AccountFrozen),
        "deposit on frozen account"
    );
    assert_eq!(
        apply(Some(&frozen), &Event::Withdrawn { amount_cents: 1 }),
        Err(LedgerError::AccountFrozen),
        "withdrawal on frozen account"
    );
    assert_eq!(
        apply(Some(&frozen), &Event::Frozen),
        Ok(frozen.clone()),
        "freezing an already frozen account is a no-op"
    );

    let reopened = apply(Some(&frozen), &Event::Unfrozen).expect("unfreeze should succeed");
    assert_eq!(reopened.status, Status::Open, "Unfrozen restores Open");
}

#[test]
fn part3_withdraw_reports_balance_and_request_when_insufficient() {
    let acc = open_account(100);
    assert_eq!(
        apply(Some(&acc), &Event::Withdrawn { amount_cents: 101 }),
        Err(LedgerError::InsufficientFunds {
            balance_cents: 100,
            requested_cents: 101
        }),
        "withdrawing more than the balance must report both numbers"
    );
    let drained = apply(Some(&acc), &Event::Withdrawn { amount_cents: 100 })
        .expect("withdrawing the exact balance is allowed");
    assert_eq!(drained.balance_cents, 0, "balance can reach exactly zero");
}

#[test]
fn part3_deposit_past_u64_max_is_balance_overflow() {
    let acc = open_account(u64::MAX - 1);
    assert_eq!(
        apply(Some(&acc), &Event::Deposited { amount_cents: 5 }),
        Err(LedgerError::BalanceOverflow),
        "balance overflow must be reported"
    );
}

#[test]
fn part3_decide_withdraw_returns_event_without_changing_account() {
    let acc = open_account(100);
    let snapshot = acc.clone();
    let event = decide_withdraw(&acc, 30).expect("30 out of 100 should be allowed");
    assert_eq!(event, Event::Withdrawn { amount_cents: 30 }, "returns the event to record");
    assert_eq!(acc, snapshot, "decide must not change the account");
}

#[test]
fn part3_decide_withdraw_applies_the_same_rules_as_apply() {
    assert_eq!(
        decide_withdraw(&account(100, Status::Frozen), 1),
        Err(LedgerError::AccountFrozen),
        "frozen account"
    );
    assert_eq!(
        decide_withdraw(&open_account(100), 999),
        Err(LedgerError::InsufficientFunds {
            balance_cents: 100,
            requested_cents: 999
        }),
        "insufficient funds"
    );
    assert_eq!(
        decide_withdraw(&open_account(100), 0),
        Err(LedgerError::NonPositiveAmount),
        "zero amount"
    );
}

// ---------- Part 4 · folds that stop at the first bad event ----------

fn valid_log() -> Vec<Event> {
    vec![
        opened("acc-1", "Ada"),
        Event::Deposited { amount_cents: 500 },
        Event::Withdrawn { amount_cents: 200 },
        Event::Frozen,
        Event::Unfrozen,
    ]
}

#[test]
fn part4_replay_folds_the_whole_log() {
    let acc = replay(&valid_log()).expect("valid log should replay");
    assert_eq!(acc.balance_cents, 300, "500 - 200");
    assert_eq!(acc.status, Status::Open, "frozen then unfrozen is open");
}

#[test]
fn part4_replay_of_empty_log_is_empty_error() {
    assert_eq!(replay(&[]), Err(LedgerError::EmptyLog), "an empty log has no account");
}

#[test]
fn part4_replay_stops_at_first_invalid_event() {
    let log = vec![
        opened("acc-1", "Ada"),
        Event::Deposited { amount_cents: 100 },
        Event::Withdrawn { amount_cents: 500 },
        Event::Deposited { amount_cents: 1 },
    ];
    assert_eq!(
        replay(&log),
        Err(LedgerError::InsufficientFunds {
            balance_cents: 100,
            requested_cents: 500
        }),
        "the first invalid event decides the error"
    );
}

#[test]
fn part4_snapshots_hold_state_after_each_event() {
    let log = valid_log();
    let snaps = snapshots(&log).expect("valid log should snapshot");
    assert_eq!(snaps.len(), log.len(), "one snapshot per event");
    assert_eq!(snaps[1].balance_cents, 500, "state after the first deposit");
    assert_eq!(
        snaps.last().cloned(),
        Some(replay(&log).unwrap()),
        "the last snapshot equals replay"
    );
}

#[test]
fn part4_snapshots_fail_on_the_first_invalid_event() {
    let log = vec![opened("acc-1", "Ada"), Event::Withdrawn { amount_cents: 1 }];
    assert_eq!(
        snapshots(&log),
        Err(LedgerError::InsufficientFunds {
            balance_cents: 0,
            requested_cents: 1
        }),
        "snapshots must not return a partial vector"
    );
}

#[test]
fn part4_replay_from_snapshot_applies_only_the_tail() {
    let log = valid_log();
    let snaps = snapshots(&log).unwrap();
    let resumed = replay_from(&snaps[2], &log[3..]).expect("tail is valid");
    assert_eq!(resumed, replay(&log).unwrap(), "resuming from a snapshot reaches the same state");

    let bad_tail = [Event::Withdrawn { amount_cents: 10_000 }];
    assert!(
        matches!(
            replay_from(&snaps[2], &bad_tail),
            Err(LedgerError::InsufficientFunds { .. })
        ),
        "an invalid tail event must still fail"
    );
}

#[test]
fn part4_total_deposited_sums_and_detects_overflow() {
    assert_eq!(total_deposited(&valid_log()), Some(500), "only Deposited events count");
    assert_eq!(total_deposited(&[]), Some(0), "no deposits means zero");

    let huge = [
        Event::Deposited { amount_cents: u64::MAX },
        Event::Deposited { amount_cents: 1 },
    ];
    assert_eq!(total_deposited(&huge), None, "overflow must be None, not a panic");
}

#[test]
fn part4_run_actions_folds_the_reducer_and_stops_on_error() {
    let actions = vec![
        add("A1", "Mug", 1200, 1),
        add("A1", "Mug", 1200, 1),
        Action::ApplyCoupon(Some("SAVE10".into())),
    ];
    let cart = run_actions(Cart::default(), actions).expect("all actions are valid");
    assert_eq!(cart.items, vec![line("A1", "Mug", 1200, 2)], "two adds merge into one line");
    assert_eq!(cart.coupon.as_deref(), Some("SAVE10"));

    let failing = vec![
        add("A1", "Mug", 1200, 1),
        Action::SetQuantity {
            sku: "missing".into(),
            quantity: 1,
        },
        add("B2", "Tea", 500, 1),
    ];
    assert_eq!(
        run_actions(Cart::default(), failing),
        Err(CartError::UnknownSku("missing".into())),
        "the fold stops at the first error"
    );
}

// ---------- Part 5 · Option/Result combinators and `?` ----------

#[test]
fn part5_parse_port_trims_and_accepts_bounds() {
    assert_eq!(parse_port(" 8080 "), Ok(8080), "surrounding whitespace is ignored");
    assert_eq!(parse_port("1"), Ok(1), "lower bound");
    assert_eq!(parse_port("65535"), Ok(65535), "upper bound");
}

#[test]
fn part5_parse_port_rejects_non_integers_and_out_of_range_values() {
    assert_eq!(
        parse_port("eighty"),
        Err(ConfigError::InvalidPort("eighty".into())),
        "non-integer"
    );
    assert_eq!(parse_port("0"), Err(ConfigError::PortOutOfRange), "zero is out of range");
    assert_eq!(parse_port("65536"), Err(ConfigError::PortOutOfRange), "above the bound");
    assert_eq!(
        parse_port("99999999999999999999"),
        Err(ConfigError::PortOutOfRange),
        "integers too large for i64 are out of range, not non-integers"
    );
}

#[test]
fn part5_parse_ports_collects_or_reports_the_first_error() {
    assert_eq!(parse_ports("80, 443"), Ok(vec![80, 443]), "comma-separated ports");
    assert_eq!(
        parse_ports("80,x,90"),
        Err(ConfigError::InvalidPort("x".into())),
        "the first bad element decides the error"
    );
}

#[test]
fn part5_parse_bool_is_case_insensitive() {
    for truthy in ["true", "YES", "1", "On", "tRuE"] {
        assert_eq!(parse_bool(truthy), Ok(true), "{truthy} should parse as true");
    }
    for falsy in ["false", "No", "0", "OFF"] {
        assert_eq!(parse_bool(falsy), Ok(false), "{falsy} should parse as false");
    }
    assert_eq!(
        parse_bool("maybe"),
        Err(ConfigError::InvalidBool("maybe".into())),
        "anything else is an error"
    );
}

#[test]
fn part5_parse_optional_port_transposes() {
    assert_eq!(parse_optional_port(None), Ok(None), "absent stays absent");
    assert_eq!(parse_optional_port(Some("22")), Ok(Some(22)), "present is parsed");
    assert_eq!(
        parse_optional_port(Some("0")),
        Err(ConfigError::PortOutOfRange),
        "a present but invalid value is an error, not None"
    );
}

#[test]
fn part5_parse_config_applies_defaults() {
    let cfg = parse_config(valid_config_text()).expect("minimal config should parse");
    assert_eq!(
        cfg,
        Config {
            host: "example.com".into(),
            port: 8080,
            debug: false,
            workers: 4
        },
        "debug defaults to false, workers to 4"
    );
}

#[test]
fn part5_parse_config_reads_all_keys_and_skips_comments_and_blank_lines() {
    let text = "# edge worker\n\nhost = edge.local\n  port=443  \n# tuning\ndebug = yes\nworkers = 16\n";
    let cfg = parse_config(text).expect("full config should parse");
    assert_eq!(
        cfg,
        Config {
            host: "edge.local".into(),
            port: 443,
            debug: true,
            workers: 16
        }
    );
}

#[test]
fn part5_malformed_line_reports_its_one_based_number() {
    assert_eq!(
        parse_config("host = a\n\nport 80\n"),
        Err(ConfigError::MalformedLine { line: 3 }),
        "a line without '=' is malformed; blank lines still count"
    );
}

#[test]
fn part5_duplicate_key_reports_the_second_occurrence() {
    assert_eq!(
        parse_config("host=a\nport=1\nport=2\n"),
        Err(ConfigError::DuplicateKey {
            line: 3,
            key: "port".into()
        })
    );
}

#[test]
fn part5_unknown_key_is_rejected() {
    assert_eq!(
        parse_config("host=a\nport=1\ncolor=red\n"),
        Err(ConfigError::UnknownKey("color".into()))
    );
}

#[test]
fn part5_missing_host_is_checked_before_port() {
    assert_eq!(
        parse_config(""),
        Err(ConfigError::MissingKey("host")),
        "empty input reports host first"
    );
    assert_eq!(
        parse_config("port = 1"),
        Err(ConfigError::MissingKey("host")),
        "host is checked before port"
    );
    assert_eq!(
        parse_config("host = a"),
        Err(ConfigError::MissingKey("port")),
        "missing port"
    );
}

#[test]
fn part5_empty_host_is_rejected_after_trimming() {
    assert_eq!(
        parse_config("host =   \nport=1\n"),
        Err(ConfigError::EmptyHost)
    );
}

#[test]
fn part5_invalid_values_name_their_field() {
    assert_eq!(
        parse_config("host=a\nport=abc\n"),
        Err(ConfigError::InvalidPort("abc".into()))
    );
    assert_eq!(
        parse_config("host=a\nport=1\ndebug=maybe\n"),
        Err(ConfigError::InvalidBool("maybe".into()))
    );
    assert_eq!(
        parse_config("host=a\nport=1\nworkers=0\n"),
        Err(ConfigError::WorkersOutOfRange),
        "workers below 1"
    );
    assert_eq!(
        parse_config("host=a\nport=1\nworkers=65\n"),
        Err(ConfigError::WorkersOutOfRange),
        "workers above 64"
    );
}

// ---------- Part 6 · accumulate-all-errors validation ----------

fn base_payload() -> serde_json::Value {
    json!({ "username": "ada_99", "age": 36, "email": "ada@example.com" })
}

#[test]
fn part6_valid_payload_builds_signup() {
    let mut payload = base_payload();
    payload["tags"] = json!(["admin", "beta"]);
    assert_eq!(
        validate_signup(&payload),
        Ok(Signup {
            username: "ada_99".into(),
            age: 36,
            email: "ada@example.com".into(),
            tags: vec!["admin".into(), "beta".into()],
        })
    );
    assert_eq!(
        validate_signup(&base_payload()).map(|s| s.tags),
        Ok(Vec::new()),
        "absent tags default to empty"
    );
    let with_null = json!({ "username": "ada_99", "age": 36, "email": "ada@example.com", "tags": null });
    assert_eq!(
        validate_signup(&with_null).map(|s| s.tags),
        Ok(Vec::new()),
        "null tags are treated as absent"
    );
}

#[test]
fn part6_non_object_fails_at_root_with_empty_path() {
    assert_eq!(
        validate_signup(&json!([1, 2])),
        Err(vec![verr("", "expected object, got array")])
    );
}

#[test]
fn part6_missing_required_fields_are_each_reported() {
    assert_eq!(
        validate_signup(&json!({})),
        Err(vec![
            verr("username", "missing"),
            verr("age", "missing"),
            verr("email", "missing"),
        ])
    );
}

#[test]
fn part6_username_checks_accumulate() {
    let with_name = |name: &str| {
        let mut p = base_payload();
        p["username"] = json!(name);
        validate_signup(&p)
    };
    assert_eq!(
        with_name("a!"),
        Err(vec![
            verr("username", "length must be at least 3"),
            verr("username", "may only contain a-z, 0-9 and _"),
        ]),
        "both rules run and both errors are reported"
    );
    assert_eq!(
        with_name("Ada"),
        Err(vec![verr("username", "may only contain a-z, 0-9 and _")]),
        "uppercase is outside the charset"
    );
    assert_eq!(
        with_name("ab"),
        Err(vec![verr("username", "length must be at least 3")])
    );
}

#[test]
fn part6_type_errors_stand_alone() {
    let mut p = base_payload();
    p["username"] = json!(42);
    assert_eq!(
        validate_signup(&p),
        Err(vec![verr("username", "expected string, got number")]),
        "a wrong type reports only the type error"
    );
}

#[test]
fn part6_age_rules_and_integer_check() {
    let with_age = |age: serde_json::Value| {
        let mut p = base_payload();
        p["age"] = age;
        validate_signup(&p)
    };
    assert_eq!(
        with_age(json!("old")),
        Err(vec![verr("age", "expected integer, got string")])
    );
    assert_eq!(
        with_age(json!(7.5)),
        Err(vec![verr("age", "expected integer, got number")]),
        "a fractional number is not an integer"
    );
    assert_eq!(
        with_age(json!(7)),
        Err(vec![verr("age", "must be between 13 and 120")])
    );
    assert!(with_age(json!(120)).is_ok(), "120 is inside the range");
    assert!(with_age(json!(13)).is_ok(), "13 is inside the range");
}

#[test]
fn part6_email_shape() {
    let with_email = |email: &str| {
        let mut p = base_payload();
        p["email"] = json!(email);
        validate_signup(&p)
    };
    let bad = [
        "bad", "a@.com", "a@b.", "a@b@c.d", "a b@c.d", "@c.d", "a@bcd", "a@b.c d",
    ];
    for email in bad {
        assert_eq!(
            with_email(email),
            Err(vec![verr("email", "must look like name@domain.tld")]),
            "{email:?} should be rejected"
        );
    }
    assert!(with_email("a@b.c").is_ok(), "the smallest valid shape is accepted");
}

#[test]
fn part6_tag_errors_are_indexed() {
    let mut p = base_payload();
    p["tags"] = json!(["ok", "", 3]);
    assert_eq!(
        validate_signup(&p),
        Err(vec![
            verr("tags[1]", "must not be empty"),
            verr("tags[2]", "expected string, got number"),
        ]),
        "element paths use bracket indexes"
    );

    p["tags"] = json!("admin");
    assert_eq!(
        validate_signup(&p),
        Err(vec![verr("tags", "expected array, got string")]),
        "tags must be an array when present"
    );
}

#[test]
fn part6_documented_example_collects_every_error_in_order() {
    let payload = json!({ "username": "a", "age": 7, "email": "bad", "tags": ["ok", ""] });
    assert_eq!(
        validate_signup(&payload),
        Err(vec![
            verr("username", "length must be at least 3"),
            verr("age", "must be between 13 and 120"),
            verr("email", "must look like name@domain.tld"),
            verr("tags[1]", "must not be empty"),
        ])
    );
}

#[test]
fn part6_combine_keeps_successes_and_concatenates_failures() {
    assert_eq!(combine(Ok::<i32, Vec<ValidationError>>(1), Ok::<&str, _>("x")), Ok((1, "x")));
    assert_eq!(
        combine(Ok::<i32, Vec<ValidationError>>(1), Err::<i32, _>(vec![verr("a", "x")])),
        Err(vec![verr("a", "x")]),
    );
    assert_eq!(
        combine::<i32, i32>(Err(vec![verr("a", "x")]), Err(vec![verr("b", "y")])),
        Err(vec![verr("a", "x"), verr("b", "y")]),
        "failures from both sides, left first"
    );
}

#[test]
fn part6_collect_all_keeps_every_error_in_order() {
    let mixed = vec![
        Ok(1),
        Err(vec![verr("a", "x")]),
        Ok(2),
        Err(vec![verr("b", "y"), verr("c", "z")]),
    ];
    assert_eq!(
        collect_all::<i32>(mixed),
        Err(vec![verr("a", "x"), verr("b", "y"), verr("c", "z")]),
        "collect_all does not stop at the first failure"
    );
    assert_eq!(
        collect_all::<i32>(vec![Ok(1), Ok(2)]),
        Ok(vec![1, 2]),
        "all Ok gives Ok(vec)"
    );
}

#[test]
fn part6_display_prefixes_non_empty_paths() {
    assert_eq!(
        verr("username", "length must be at least 3").to_string(),
        "username: length must be at least 3"
    );
    assert_eq!(
        verr("", "expected object, got array").to_string(),
        "expected object, got array",
        "root errors print only the message"
    );
}

// ---------- Part 7 · ETL pipeline and ranked report ----------

#[test]
fn part7_parse_cents_is_exact() {
    assert_eq!(parse_cents("12.5"), Some(1250));
    assert_eq!(parse_cents("12"), Some(1200));
    assert_eq!(parse_cents("0.01"), Some(1));
    assert_eq!(parse_cents("0"), Some(0));
    assert_eq!(parse_cents("1234.50"), Some(123_450));
}

#[test]
fn part7_parse_cents_rejects_malformed_money() {
    for raw in ["1.234", "-1.00", "+5", "NaN", "inf", "abc", "", "12.", ".5", "1e3", " 1.00"] {
        assert_eq!(parse_cents(raw), None, "{raw:?} is not a valid money amount");
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn part7_parse_cents_roundtrips_formatted_amounts(cents in 0u64..1_000_000_000_000) {
        let text = format!("{}.{:02}", cents / 100, cents % 100);
        prop_assert_eq!(parse_cents(&text), Some(cents));
    }
}

#[test]
fn part7_parse_sale_normalises_fields() {
    let sale = parse_sale(&row("  US ", " Widget ", " 3 ", "12.50", "2024-03-01"))
        .expect("row should parse");
    assert_eq!(
        sale,
        Sale {
            region: "us".into(),
            product: "Widget".into(),
            units: 3,
            unit_price_cents: 1250,
            sold_on: jiff::civil::date(2024, 3, 1),
        }
    );
}

#[test]
fn part7_parse_sale_reports_the_first_bad_field() {
    assert_eq!(
        parse_sale(&row("us", "w", "-3", "1.00", "2024-03-01")),
        Err(EtlError::InvalidField {
            field: "units",
            value: "-3".into()
        })
    );
    assert_eq!(
        parse_sale(&row("us", "w", "1", "12.345", "2024-03-01")),
        Err(EtlError::InvalidField {
            field: "unit_price",
            value: "12.345".into()
        })
    );
    assert_eq!(
        parse_sale(&row("us", "w", "1", "1.00", "2024-02-30")),
        Err(EtlError::InvalidField {
            field: "sold_on",
            value: "2024-02-30".into()
        }),
        "impossible calendar dates are rejected"
    );
}

#[test]
fn part7_clean_sales_drops_blank_and_zero_rows_and_indexes_errors() {
    let rows = vec![
        row("us", "a", "1", "10.00", "2024-01-01"),
        row("   ", "b", "1", "10.00", "2024-01-01"),
        row("us", "c", "0", "10.00", "2024-01-01"),
        row("eu", "d", "x", "10.00", "2024-01-01"),
        row("eu", "e", "2", "5.00", "2024-01-02"),
    ];
    let (sales, errors) = clean_sales(rows);
    assert_eq!(sales.len(), 2, "blank-region and zero-unit rows are dropped silently");
    assert_eq!(sales[0].product, "a");
    assert_eq!(sales[1].product, "e");
    assert_eq!(
        errors,
        vec![RowError {
            row: 3,
            error: EtlError::InvalidField {
                field: "units",
                value: "x".into()
            }
        }],
        "bad rows are collected with their zero-based input index"
    );
}

#[test]
fn part7_clean_sales_accepts_any_iterator() {
    let rows = (0..4u32).map(|i| row("us", "p", &i.to_string(), "1.00", "2024-01-01"));
    let (sales, errors) = clean_sales(rows.filter(|r| r.units != "2"));
    assert!(errors.is_empty(), "no parse errors expected");
    assert_eq!(
        sales.iter().map(|s| s.units).collect::<Vec<_>>(),
        vec![1, 3],
        "zero units is dropped; the filtered iterator is consumed lazily"
    );
}

#[test]
fn part1_totals_apply_coupon_discount_with_floor() {
    let cart = Cart {
        items: vec![line("A1", "Mug", 1250, 3), line("B2", "Tea", 999, 1)],
        coupon: Some("SAVE10".into()),
    };
    // subtotal 3750 + 999 = 4749; 10% of 4749 = 474.9 floors to 474 -> 4275
    assert_eq!(subtotal_cents(&cart), Ok(4749), "subtotal sums quantity * unit price");
    assert_eq!(total_cents(&cart), Ok(4275), "coupon discount is floored to whole cents");

    let no_coupon = Cart { coupon: None, ..cart };
    assert_eq!(total_cents(&no_coupon), Ok(4749), "no coupon means total equals subtotal");
}

#[test]
fn part7_revenue_and_totals_by_region_in_ascending_key_order() {
    let sales = vec![sale("us", 3, 1250), sale("eu", 2, 500), sale("us", 1, 999)];
    assert_eq!(revenue_cents(&sales[0]), Some(3750));

    let totals = totals_by(&sales, |s| s.region.clone()).expect("no overflow");
    assert_eq!(
        totals.into_iter().collect::<Vec<_>>(),
        vec![("eu".to_string(), 1000), ("us".to_string(), 4749)],
        "BTreeMap keeps keys ascending"
    );
}

#[test]
fn part7_revenue_overflow_is_reported_with_its_group() {
    let big = sale("eu", 2, u64::MAX / 2 + 1);
    assert_eq!(revenue_cents(&big), None, "2 * (MAX/2 + 1) overflows u64");
    assert_eq!(
        totals_by(&[big], |s| s.region.clone()),
        Err(EtlError::Overflow("eu".into())),
        "the error names the group whose total overflowed"
    );
}

#[test]
fn part7_ranked_orders_by_total_then_name() {
    let totals: BTreeMap<String, u64> = [("b", 5), ("a", 5), ("c", 9), ("d", 1)]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    assert_eq!(
        ranked(&totals),
        vec![
            ("c".to_string(), 9),
            ("a".to_string(), 5),
            ("b".to_string(), 5),
            ("d".to_string(), 1),
        ],
        "total descending, ties by name ascending"
    );
}

#[test]
fn part7_distinct_regions_are_unique_uppercase_and_sorted() {
    let sales = vec![sale("us", 1, 1), sale("eu", 1, 1), sale("us", 1, 1)];
    assert_eq!(distinct_regions(&sales), vec!["EU".to_string(), "US".to_string()]);
}

#[test]
fn part7_format_cents_groups_thousands() {
    assert_eq!(format_cents(1_234_567, "$"), "$12,345.67");
    assert_eq!(format_cents(0, "$"), "$0.00", "zero still has cents");
    assert_eq!(format_cents(5, "$"), "$0.05", "cents are zero-padded");
    assert_eq!(format_cents(123_456_789_012, "€"), "€1,234,567,890.12");
}

#[test]
fn part7_load_csv_reads_header_rows() {
    let text = "region,product,units,unit_price,sold_on\nUS,Widget,3,12.50,2024-03-01\n";
    let rows = load_csv(text).expect("valid csv");
    assert_eq!(rows, vec![row("US", "Widget", "3", "12.50", "2024-03-01")]);
}

#[test]
fn part7_build_report_ranks_regions_and_lists_rejected_rows() {
    let text = "\
region,product,units,unit_price,sold_on
us,Widget,3,12.50,2024-03-01
us,Gadget,1,9.99,2024-03-02
eu,Widget,2,5.00,2024-03-02
  ,Ghost,4,1.00,2024-03-03
fr,Widget,x,1.00,2024-03-03
";
    let rows = load_csv(text).expect("valid csv");
    let report = build_report(rows, "$").expect("no overflow");

    assert_eq!(
        report.lines,
        vec!["us: $47.49".to_string(), "eu: $10.00".to_string()],
        "ranked by total, one line per region"
    );
    assert_eq!(
        report.rejected,
        vec![RowError {
            row: 4,
            error: EtlError::InvalidField {
                field: "units",
                value: "x".into()
            }
        }],
        "blank-region rows are dropped, not rejected; bad units are rejected with index 4"
    );
}
