//! Black-box tests for `ex01_collections`. Filter one part with
//! `cargo test -p ex01_collections part2`.

use std::cell::Cell;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ex01_collections::cache::TtlCache;
use ex01_collections::config::{Config, Layer, Layers, Value, deep_merge, parse_env};
use ex01_collections::inventory::{Money, PriceTiers, SlaTier, reconcile, sla_tier};
use ex01_collections::limiter::{ClientId, SlidingWindowLimiter};
use ex01_collections::records::{LazyMap, Shipment, group_by};
use ex01_collections::topk::top_k_products;
use ex01_collections::{Clock, Error};
use proptest::prelude::*;

// ---------------------------------------------------------------- helpers

/// A clock the test moves by hand. Clones share the same time.
#[derive(Clone)]
struct ManualClock {
    base: Instant,
    offset: Arc<Mutex<Duration>>,
}

impl ManualClock {
    fn new() -> Self {
        Self {
            base: Instant::now(),
            offset: Arc::new(Mutex::new(Duration::ZERO)),
        }
    }

    fn advance(&self, by: Duration) {
        *self.offset.lock().unwrap() += by;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Instant {
        self.base + *self.offset.lock().unwrap()
    }
}

fn hash_of<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

fn owned(keys: &[&str]) -> Vec<String> {
    keys.iter().map(|k| k.to_string()).collect()
}

fn set(keys: &[&str]) -> BTreeSet<String> {
    keys.iter().map(|k| k.to_string()).collect()
}

fn pairs(items: &[(&str, u64)]) -> Vec<(String, u64)> {
    items.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

fn hm(items: &[(&str, i64)]) -> HashMap<String, i64> {
    items.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

fn money(cents: i64) -> Money {
    Money::from_cents(cents)
}

fn fresh_cache(max: usize, ttl: u64, clock: &ManualClock) -> TtlCache<String, i32, ManualClock> {
    TtlCache::new(max, secs(ttl), clock.clone()).expect("valid cache config")
}

fn put(cache: &mut TtlCache<String, i32, ManualClock>, key: &str, value: i32) {
    cache.put(key.to_string(), value);
}

fn map<const N: usize>(entries: [(&str, Value); N]) -> Value {
    Value::Map(entries.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn text(s: &str) -> Value {
    Value::Str(s.to_string())
}

fn only_defaults(defaults: Value) -> Layers {
    Layers {
        defaults,
        file: None,
        env: None,
        cli: None,
    }
}

fn env_vars(vars: &[(&str, &str)]) -> Result<Value, Error> {
    parse_env(vars.iter().copied(), "APP")
}

// ---------------------------------------------------------------- Part 1: TTL + LRU cache

#[test]
fn part1_new_rejects_zero_capacity_and_zero_ttl() {
    let clock = ManualClock::new();
    let zero_cap = TtlCache::<String, i32, ManualClock>::new(0, secs(1), clock.clone()).err();
    assert_eq!(zero_cap, Some(Error::ZeroCapacity), "max_size 0 must be rejected");
    let zero_ttl = TtlCache::<String, i32, ManualClock>::new(1, Duration::ZERO, clock).err();
    assert_eq!(zero_ttl, Some(Error::ZeroTtl), "a zero ttl must be rejected");
}

#[test]
fn part1_put_then_get_returns_value() {
    let clock = ManualClock::new();
    let mut cache = fresh_cache(4, 60, &clock);
    put(&mut cache, "a", 1);
    assert_eq!(cache.get("a"), Some(&1), "a live key must be returned");
    assert_eq!(cache.len(), 1);
}

#[test]
fn part1_miss_returns_none_and_counts_hits_and_misses() {
    let clock = ManualClock::new();
    let mut cache = fresh_cache(4, 60, &clock);
    assert_eq!(cache.get("missing"), None, "unknown key is a miss");
    assert_eq!((cache.hits(), cache.misses()), (0, 1));
    put(&mut cache, "a", 1);
    assert_eq!(cache.get("a"), Some(&1));
    assert_eq!((cache.hits(), cache.misses()), (1, 1), "counters after one hit and one miss");
}

#[test]
fn part1_replacing_a_key_refreshes_its_ttl() {
    let clock = ManualClock::new();
    let mut cache = fresh_cache(4, 10, &clock);
    put(&mut cache, "a", 1);
    clock.advance(secs(8));
    put(&mut cache, "a", 2);
    clock.advance(secs(8));
    assert_eq!(
        cache.get("a"),
        Some(&2),
        "16s after the first write but 8s after the replacement: still live"
    );
}

#[test]
fn part1_entry_is_expired_at_exactly_ttl() {
    let clock = ManualClock::new();
    let mut cache = fresh_cache(4, 10, &clock);
    put(&mut cache, "a", 1);
    clock.advance(secs(9));
    assert_eq!(cache.get("a"), Some(&1), "9s into a 10s ttl is live");
    clock.advance(secs(1));
    assert_eq!(cache.get("a"), None, "at exactly ttl the entry is expired");
}

#[test]
fn part1_get_does_not_extend_ttl() {
    let clock = ManualClock::new();
    let mut cache = fresh_cache(4, 10, &clock);
    put(&mut cache, "a", 1);
    clock.advance(secs(6));
    assert_eq!(cache.get("a"), Some(&1));
    clock.advance(secs(4));
    assert_eq!(cache.get("a"), None, "a read must not push expiry back");
}

#[test]
fn part1_expired_entries_are_not_live() {
    let clock = ManualClock::new();
    let mut cache = fresh_cache(4, 10, &clock);
    put(&mut cache, "a", 1);
    put(&mut cache, "b", 2);
    clock.advance(secs(10));
    assert_eq!(cache.len(), 0, "expired entries do not count");
    assert!(cache.is_empty());
    assert!(cache.keys().is_empty());
    assert!(!cache.contains_key("a"));
}

#[test]
fn part1_evicts_least_recently_used_when_full() {
    let clock = ManualClock::new();
    let mut cache = fresh_cache(2, 60, &clock);
    put(&mut cache, "a", 1);
    put(&mut cache, "b", 2);
    assert_eq!(cache.get("a"), Some(&1), "touch a so b becomes the LRU entry");
    put(&mut cache, "c", 3);
    assert_eq!(cache.len(), 2);
    assert!(!cache.contains_key("b"), "b was least recently used and must be evicted");
    assert_eq!(cache.keys(), owned(&["a", "c"]));
}

#[test]
fn part1_expired_entries_are_purged_before_lru_eviction() {
    let clock = ManualClock::new();
    let mut cache = fresh_cache(2, 10, &clock);
    put(&mut cache, "b", 1); // t=0, expires at t=10
    clock.advance(secs(5));
    put(&mut cache, "a", 2); // t=5, expires at t=15
    assert_eq!(cache.get("b"), Some(&1), "b is live at t=5 and becomes most recent");
    clock.advance(secs(6)); // t=11: b expired, a still live
    put(&mut cache, "c", 3);
    assert_eq!(cache.len(), 2);
    assert!(cache.contains_key("a"), "the live entry must survive; purge the expired one instead");
    assert!(cache.contains_key("c"));
}

#[test]
fn part1_keys_are_lru_first_and_inspection_does_not_touch_state() {
    let clock = ManualClock::new();
    let mut cache = fresh_cache(4, 60, &clock);
    put(&mut cache, "a", 1);
    put(&mut cache, "b", 2);
    assert_eq!(cache.keys(), owned(&["a", "b"]), "least recently used first");
    assert!(cache.contains_key("a"));
    assert_eq!(cache.keys(), owned(&["a", "b"]), "contains_key must not change recency");
    assert_eq!((cache.hits(), cache.misses()), (0, 0), "inspection must not count");
}

// ---------------------------------------------------------------- Part 2: rate limiter

#[test]
fn part2_client_id_eq_is_ascii_case_insensitive_and_hash_agrees() {
    let a = ClientId::new("Tenant-A");
    let b = ClientId::new("tenant-a");
    assert!(a == b, "names differing only in ASCII case are the same client");
    assert!(ClientId::new("tenant-b") != a, "different names are different clients");
    assert_eq!(hash_of(&a), hash_of(&b), "equal ClientIds must hash equally");
}

#[test]
fn part2_client_id_works_as_hash_set_key() {
    let mut seen = HashSet::new();
    seen.insert(ClientId::new("Acme"));
    seen.insert(ClientId::new("ACME"));
    seen.insert(ClientId::new("globex"));
    assert_eq!(seen.len(), 2, "case variants collapse to one key");
}

#[test]
fn part2_admits_up_to_limit_then_denies() {
    let clock = ManualClock::new();
    let mut limiter = SlidingWindowLimiter::new(3, secs(60), clock.clone()).unwrap();
    let who = ClientId::new("acme");
    assert!(limiter.allow(&who));
    assert!(limiter.allow(&who));
    assert!(limiter.allow(&who));
    assert!(!limiter.allow(&who), "the fourth request in the window must be denied");
}

#[test]
fn part2_denied_requests_are_not_recorded() {
    let clock = ManualClock::new();
    let mut limiter = SlidingWindowLimiter::new(2, secs(10), clock.clone()).unwrap();
    let who = ClientId::new("acme");
    assert!(limiter.allow(&who)); // t=0
    assert!(limiter.allow(&who)); // t=0
    clock.advance(secs(5));
    for _ in 0..5 {
        assert!(!limiter.allow(&who), "window is full at t=5");
    }
    clock.advance(secs(5)); // t=10: the t=0 events leave the window
    assert!(limiter.allow(&who), "denied attempts must not have consumed capacity");
    assert_eq!(limiter.remaining(&who), 1, "only the t=10 admission is live");
}

#[test]
fn part2_window_boundary_is_exact() {
    let clock = ManualClock::new();
    let mut limiter = SlidingWindowLimiter::new(1, secs(10), clock.clone()).unwrap();
    let who = ClientId::new("acme");
    assert!(limiter.allow(&who));
    clock.advance(secs(9));
    assert!(!limiter.allow(&who), "at 9s the event at t=0 is still in the window");
    clock.advance(secs(1));
    assert!(limiter.allow(&who), "at exactly the window length the event has left");
}

#[test]
fn part2_remaining_counts_down_and_saturates_at_zero() {
    let clock = ManualClock::new();
    let mut limiter = SlidingWindowLimiter::new(3, secs(10), clock.clone()).unwrap();
    let who = ClientId::new("acme");
    assert_eq!(limiter.remaining(&who), 3, "unknown clients start with the full limit");
    assert_eq!(limiter.retry_after(&who), Duration::ZERO);
    assert!(limiter.allow(&who));
    assert!(limiter.allow(&who));
    assert_eq!(limiter.remaining(&who), 1);
    assert_eq!(limiter.retry_after(&who), Duration::ZERO, "capacity remains: no wait");
    assert!(limiter.allow(&who));
    assert!(!limiter.allow(&who));
    assert_eq!(limiter.remaining(&who), 0);
}

#[test]
fn part2_retry_after_is_time_until_oldest_event_leaves() {
    let clock = ManualClock::new();
    let mut limiter = SlidingWindowLimiter::new(2, secs(10), clock.clone()).unwrap();
    let who = ClientId::new("acme");
    assert!(limiter.allow(&who)); // t=0
    clock.advance(secs(3));
    assert!(limiter.allow(&who)); // t=3
    clock.advance(secs(1)); // t=4
    assert_eq!(limiter.remaining(&who), 0);
    assert_eq!(
        limiter.retry_after(&who),
        secs(6),
        "the t=0 event leaves at t=10, so 6s from t=4"
    );
}

#[test]
fn part2_clients_are_independent() {
    let clock = ManualClock::new();
    let mut limiter = SlidingWindowLimiter::new(1, secs(60), clock.clone()).unwrap();
    let acme = ClientId::new("acme");
    let globex = ClientId::new("globex");
    assert!(limiter.allow(&acme));
    assert!(!limiter.allow(&acme));
    assert!(limiter.allow(&globex), "exhausting one client must not affect another");
}

#[test]
fn part2_prune_drops_only_clients_without_live_events() {
    let clock = ManualClock::new();
    let mut limiter = SlidingWindowLimiter::new(5, secs(10), clock.clone()).unwrap();
    let idle = ClientId::new("idle");
    let busy = ClientId::new("busy");
    assert!(limiter.allow(&idle)); // t=0
    clock.advance(secs(5));
    assert!(limiter.allow(&busy)); // t=5
    clock.advance(secs(5)); // t=10
    assert_eq!(limiter.prune(), 1, "only the idle client has no live events");
    assert_eq!(limiter.remaining(&busy), 4, "pruning must not forget live events");
    clock.advance(secs(5)); // t=15
    assert_eq!(limiter.prune(), 1);
    assert_eq!(limiter.prune(), 0, "nothing left to drop");
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, failure_persistence: None, ..ProptestConfig::default() })]

    /// Model: an event at `t` is live at `now` iff `t + window > now`. Admission
    /// must agree with the model exactly, for every client, at every step.
    #[test]
    fn part2_never_admits_more_than_limit_in_any_window(
        steps in prop::collection::vec((0u64..50, 0usize..3), 0..200),
    ) {
        const LIMIT: usize = 4;
        const WINDOW_MS: u64 = 100;
        let clock = ManualClock::new();
        let mut limiter = SlidingWindowLimiter::new(
            LIMIT as u32,
            Duration::from_millis(WINDOW_MS),
            clock.clone(),
        )
        .unwrap();
        let clients = [ClientId::new("a"), ClientId::new("b"), ClientId::new("c")];
        let mut now = 0u64;
        let mut admitted: Vec<(u64, usize)> = Vec::new();
        for (gap, c) in steps {
            now += gap;
            clock.advance(Duration::from_millis(gap));
            let live = admitted
                .iter()
                .filter(|&&(t, cc)| cc == c && t + WINDOW_MS > now)
                .count();
            if limiter.allow(&clients[c]) {
                prop_assert!(live < LIMIT, "admitted at {now}ms with {live} live events for client {c}");
                admitted.push((now, c));
            } else {
                prop_assert!(live >= LIMIT, "denied at {now}ms with only {live} live events for client {c}");
            }
        }
    }
}

// ---------------------------------------------------------------- Part 3: streaming top-K

#[test]
fn part3_ranks_by_units_descending_and_truncates_to_k() {
    let sales = [("a", 3), ("b", 5), ("c", 1)];
    assert_eq!(top_k_products(sales, 2).unwrap(), pairs(&[("b", 5), ("a", 3)]));
    assert_eq!(
        top_k_products(sales, 10).unwrap(),
        pairs(&[("b", 5), ("a", 3), ("c", 1)]),
        "k larger than the distinct count returns every SKU"
    );
}

#[test]
fn part3_sums_repeated_skus_before_ranking() {
    let sales = [("a", 2), ("b", 3), ("a", 2)];
    assert_eq!(top_k_products(sales, 2).unwrap(), pairs(&[("a", 4), ("b", 3)]));
}

#[test]
fn part3_ties_are_broken_by_sku_ascending() {
    let sales = [("z", 5), ("a", 5), ("m", 5)];
    assert_eq!(top_k_products(sales, 2).unwrap(), pairs(&[("a", 5), ("m", 5)]));
}

#[test]
fn part3_k_zero_is_empty_but_still_validates_input() {
    assert_eq!(top_k_products([("a", 1)], 0).unwrap(), Vec::new());
    assert_eq!(
        top_k_products([("a", -1)], 0),
        Err(Error::NegativeUnits { sku: "a".into(), units: -1 }),
        "negative lines are rejected even when k == 0"
    );
}

#[test]
fn part3_negative_units_are_rejected() {
    assert_eq!(
        top_k_products([("x", 2), ("y", -3)], 1),
        Err(Error::NegativeUnits { sku: "y".into(), units: -3 })
    );
}

#[test]
fn part3_overflowing_total_is_an_error() {
    // Three lines of i64::MAX total ~2.7e19, which does not fit in u64.
    let sales = vec![("a", i64::MAX); 3];
    assert_eq!(top_k_products(sales, 1), Err(Error::Overflow));
}

#[test]
fn part3_accepts_owned_and_borrowed_skus() {
    let owned_input = vec![("a".to_string(), 1i64)];
    assert_eq!(top_k_products(owned_input, 1).unwrap(), pairs(&[("a", 1)]));
    assert_eq!(top_k_products([("a", 1i64)], 1).unwrap(), pairs(&[("a", 1)]));
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn part3_matches_naive_reference(
        sales in prop::collection::vec((0usize..6, 0i64..100), 0..120),
        k in 0usize..8,
    ) {
        let lines: Vec<(String, i64)> = sales
            .iter()
            .map(|&(i, units)| (format!("sku-{i}"), units))
            .collect();
        let mut totals: BTreeMap<String, u64> = BTreeMap::new();
        for (sku, units) in &lines {
            *totals.entry(sku.clone()).or_default() += *units as u64;
        }
        let mut expected: Vec<(String, u64)> = totals.into_iter().collect();
        expected.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        expected.truncate(k);
        let got = top_k_products(lines, k).unwrap();
        prop_assert_eq!(got, expected);
    }
}

// ---------------------------------------------------------------- Part 4: layered config

#[test]
fn part4_get_walks_dotted_paths() {
    let defaults = map([("db", map([("pool", map([("size", Value::Int(5))]))]))]);
    let cfg = Config::new(&only_defaults(defaults));
    assert_eq!(cfg.get("db.pool.size"), Some(&Value::Int(5)));
}

#[test]
fn part4_absent_paths_are_none() {
    let defaults = map([("db", map([("size", Value::Int(5))]))]);
    let cfg = Config::new(&only_defaults(defaults));
    assert_eq!(cfg.get("nope"), None, "missing key");
    assert_eq!(cfg.get(""), None, "empty path");
    assert_eq!(cfg.get("db..size"), None, "empty segment");
    assert_eq!(cfg.get("db.size.deeper"), None, "path through a scalar");
}

#[test]
fn part4_cli_beats_env_beats_file_beats_defaults() {
    let mut layers = only_defaults(map([("port", Value::Int(1))]));
    assert_eq!(Config::new(&layers).get("port"), Some(&Value::Int(1)));
    layers.file = Some(map([("port", Value::Int(2))]));
    assert_eq!(Config::new(&layers).get("port"), Some(&Value::Int(2)));
    layers.env = Some(map([("port", Value::Int(3))]));
    assert_eq!(Config::new(&layers).get("port"), Some(&Value::Int(3)));
    layers.cli = Some(map([("port", Value::Int(4))]));
    assert_eq!(Config::new(&layers).get("port"), Some(&Value::Int(4)));
}

#[test]
fn part4_source_reports_the_winning_layer() {
    let mut layers = only_defaults(map([("port", Value::Int(1)), ("host", text("localhost"))]));
    layers.file = Some(map([("port", Value::Int(2))]));
    let cfg = Config::new(&layers);
    assert_eq!(cfg.source("port"), Some(Layer::File));
    assert_eq!(cfg.source("host"), Some(Layer::Defaults));
    assert_eq!(cfg.source("missing"), None);
}

#[test]
fn part4_null_counts_as_present() {
    let mut layers = only_defaults(map([("feature", Value::Bool(true))]));
    layers.file = Some(map([("feature", Value::Null)]));
    let cfg = Config::new(&layers);
    assert_eq!(cfg.get("feature"), Some(&Value::Null), "an explicit null overrides the default");
    assert_eq!(cfg.source("feature"), Some(Layer::File));
}

#[test]
fn part4_nested_maps_merge_across_layers() {
    let mut layers = only_defaults(map([(
        "db",
        map([("host", text("localhost")), ("pool", map([("size", Value::Int(5))]))]),
    )]));
    layers.file = Some(map([("db", map([("pool", map([("size", Value::Int(20))]))]))]));
    layers.env = Some(map([("db", map([("host", text("db.internal"))]))]));
    let cfg = Config::new(&layers);
    assert_eq!(cfg.get("db.pool.size"), Some(&Value::Int(20)));
    assert_eq!(cfg.source("db.pool.size"), Some(Layer::File));
    assert_eq!(cfg.get("db.host"), Some(&text("db.internal")));
    assert_eq!(cfg.source("db.host"), Some(Layer::Env));
}

#[test]
fn part4_overlay_replaces_lists_scalars_and_nulls_but_merges_maps() {
    let hosts = |names: &[&str]| Value::List(names.iter().map(|n| text(n)).collect());
    assert_eq!(
        deep_merge(&map([("hosts", hosts(&["a", "b"]))]), &map([("hosts", hosts(&["c"]))])),
        map([("hosts", hosts(&["c"]))]),
        "lists are replaced, not concatenated"
    );
    assert_eq!(
        deep_merge(&map([("a", map([("x", Value::Int(1))]))]), &map([("a", Value::Int(5))])),
        map([("a", Value::Int(5))]),
        "a scalar replaces a map"
    );
    assert_eq!(
        deep_merge(&map([("a", Value::Int(5))]), &map([("a", map([("x", Value::Int(1))]))])),
        map([("a", map([("x", Value::Int(1))]))]),
        "a map replaces a scalar"
    );
    assert_eq!(
        deep_merge(&map([("a", map([("x", Value::Int(1)), ("y", Value::Int(2))]))]), &map([("a", map([("y", Value::Int(3))]))])),
        map([("a", map([("x", Value::Int(1)), ("y", Value::Int(3))]))]),
        "maps merge key by key"
    );
    assert_eq!(
        deep_merge(&map([("a", Value::Int(1))]), &map([("a", Value::Null)])),
        map([("a", Value::Null)]),
        "null overrides"
    );
}

#[test]
fn part4_deep_merge_does_not_mutate_inputs() {
    let base = map([("a", map([("x", Value::Int(1))]))]);
    let overlay = map([("a", map([("x", Value::Int(2))]))]);
    let (base_before, overlay_before) = (base.clone(), overlay.clone());
    let merged = deep_merge(&base, &overlay);
    assert_eq!(base, base_before, "base must be unchanged");
    assert_eq!(overlay, overlay_before, "overlay must be unchanged");
    assert_eq!(merged, map([("a", map([("x", Value::Int(2))]))]));
}

#[test]
fn part4_effective_is_the_full_merge() {
    let mut layers = only_defaults(map([
        ("db", map([("host", text("localhost")), ("pool", map([("size", Value::Int(5))]))])),
        ("debug", Value::Bool(false)),
    ]));
    layers.file = Some(map([("db", map([("pool", map([("size", Value::Int(20))]))]))]));
    layers.cli = Some(map([("debug", Value::Bool(true))]));
    let expected = map([
        ("db", map([("host", text("localhost")), ("pool", map([("size", Value::Int(20))]))])),
        ("debug", Value::Bool(true)),
    ]);
    assert_eq!(Config::new(&layers).effective(), expected);
}

#[test]
fn part4_config_snapshots_its_layers() {
    let mut layers = only_defaults(map([("port", Value::Int(1))]));
    let cfg = Config::new(&layers);
    layers.defaults = map([("port", Value::Int(9))]);
    layers.cli = Some(map([("port", Value::Int(7))]));
    assert_eq!(cfg.get("port"), Some(&Value::Int(1)), "later edits to the inputs must not leak in");
    assert_eq!(cfg.source("port"), Some(Layer::Defaults));
}

#[test]
fn part4_parse_env_builds_nested_map_with_lowercased_paths() {
    let parsed = env_vars(&[
        ("APP_DB__POOL__SIZE", "10"),
        ("APP_DB__HOST", "db.internal"),
        ("APP_Name", "x"),
        ("OTHER_THING", "1"),
    ])
    .unwrap();
    assert_eq!(
        parsed,
        map([
            ("db", map([("host", text("db.internal")), ("pool", map([("size", Value::Int(10))]))])),
            ("name", text("x")),
        ]),
        "`__` splits segments, keys are lowercased, unrelated variables are ignored"
    );
}

#[test]
fn part4_parse_env_coerces_only_exact_bools_and_i64_ints() {
    let parsed = env_vars(&[
        ("APP_DEBUG", "TRUE"),
        ("APP_RETRIES", "-3"),
        ("APP_ZERO", "007"),
        ("APP_PLUS", "+3"),
        ("APP_RATIO", "1.5"),
        ("APP_BIG", "9223372036854775808"),
        ("APP_QUIET", "false"),
    ])
    .unwrap();
    assert_eq!(
        parsed,
        map([
            ("debug", Value::Bool(true)),
            ("retries", Value::Int(-3)),
            ("zero", Value::Int(7)),
            ("plus", text("+3")),
            ("ratio", text("1.5")),
            ("big", text("9223372036854775808")),
            ("quiet", Value::Bool(false)),
        ])
    );
}

#[test]
fn part4_parse_env_rejects_empty_path_segments() {
    assert_eq!(
        env_vars(&[("APP_", "1")]),
        Err(Error::EmptyEnvSegment { key: "APP_".into() })
    );
    assert_eq!(
        env_vars(&[("APP_A__", "1")]),
        Err(Error::EmptyEnvSegment { key: "APP_A__".into() })
    );
}

#[test]
fn part4_parse_env_leaf_and_map_conflict_either_order() {
    let leaf_first = env_vars(&[("APP_DB", "1"), ("APP_DB__POOL", "2")]);
    let map_first = env_vars(&[("APP_DB__POOL", "2"), ("APP_DB", "1")]);
    assert!(matches!(leaf_first, Err(Error::EnvPathConflict { .. })), "got {leaf_first:?}");
    assert!(matches!(map_first, Err(Error::EnvPathConflict { .. })), "got {map_first:?}");
}

#[test]
fn part4_parse_env_duplicate_leaf_is_a_conflict() {
    let forward = env_vars(&[("APP_A", "1"), ("APP_A", "2")]);
    let backward = env_vars(&[("APP_A", "2"), ("APP_A", "1")]);
    assert!(matches!(forward, Err(Error::EnvPathConflict { .. })), "got {forward:?}");
    assert!(matches!(backward, Err(Error::EnvPathConflict { .. })), "got {backward:?}");
}

// ---------------------------------------------------------------- Part 5: inventory and pricing

#[test]
fn part5_money_displays_two_decimals() {
    assert_eq!(money(1205).to_string(), "12.05");
    assert_eq!(money(1200).to_string(), "12.00");
    assert_eq!(money(5).to_string(), "0.05");
}

#[test]
fn part5_money_checked_mul_is_exact_and_detects_overflow() {
    assert_eq!(money(1999).checked_mul(3), Some(money(5997)));
    assert_eq!(money(i64::MAX).checked_mul(2), None);
}

#[test]
fn part5_reconcile_identical_stock_is_clean() {
    let r = reconcile(&hm(&[("a", 3), ("b", 0)]), &hm(&[("a", 3), ("b", 0)])).unwrap();
    assert!(r.is_clean(), "no missing, unexpected or mismatched SKUs");
}

#[test]
fn part5_reconcile_classifies_missing_unexpected_and_mismatched() {
    let r = reconcile(
        &hm(&[("a", 3), ("b", 2), ("c", 1)]),
        &hm(&[("a", 3), ("b", 5), ("d", 4)]),
    )
    .unwrap();
    assert_eq!(r.missing, set(&["c"]));
    assert_eq!(r.unexpected, set(&["d"]));
    assert_eq!(r.mismatched, BTreeMap::from([("b".to_string(), 3)]), "counted - expected");
    assert!(!r.is_clean());
}

#[test]
fn part5_zero_counted_is_a_mismatch_not_a_missing_sku() {
    let r = reconcile(&hm(&[("a", 3)]), &hm(&[("a", 0)])).unwrap();
    assert!(r.missing.is_empty(), "the SKU was counted, so it is not missing");
    assert_eq!(r.mismatched, BTreeMap::from([("a".to_string(), -3)]));
}

#[test]
fn part5_negative_quantity_reports_the_smallest_sku() {
    // Negatives on both sides: b (expected), c and z (counted). "b" sorts first.
    let err = reconcile(&hm(&[("b", -2), ("a", 1)]), &hm(&[("c", -7), ("z", -1)]));
    assert_eq!(err.err(), Some(Error::NegativeQuantity { sku: "b".into(), quantity: -2 }));
}

#[test]
fn part5_tiers_pick_the_largest_min_qty_at_or_below() {
    let tiers = PriceTiers::new(vec![(1, money(1000)), (10, money(900)), (100, money(800))]).unwrap();
    assert_eq!(tiers.unit_price(1).unwrap(), money(1000));
    assert_eq!(tiers.unit_price(9).unwrap(), money(1000));
    assert_eq!(tiers.unit_price(10).unwrap(), money(900));
    assert_eq!(tiers.unit_price(99).unwrap(), money(900));
    assert_eq!(tiers.unit_price(100).unwrap(), money(800));
    assert_eq!(tiers.unit_price(10_000).unwrap(), money(800));
    let reversed = PriceTiers::new(vec![(100, money(800)), (1, money(1000)), (10, money(900))]).unwrap();
    assert_eq!(reversed, tiers, "input order must not matter");
}

#[test]
fn part5_line_total_is_unit_price_times_quantity() {
    let tiers = PriceTiers::new(vec![(1, money(1000)), (10, money(900))]).unwrap();
    assert_eq!(tiers.line_total(3).unwrap(), money(3000));
    assert_eq!(tiers.line_total(10).unwrap(), money(9000));
}

#[test]
fn part5_tier_table_shape_errors() {
    assert_eq!(PriceTiers::new(Vec::<(u32, Money)>::new()).err(), Some(Error::NoPriceTiers));
    assert_eq!(
        PriceTiers::new(vec![(2, money(900))]).err(),
        Some(Error::FirstTierMustStartAtOne)
    );
    assert_eq!(
        PriceTiers::new(vec![(1, money(1000)), (10, money(900)), (10, money(800))]).err(),
        Some(Error::DuplicateTier(10))
    );
}

#[test]
fn part5_non_positive_prices_are_rejected() {
    assert_eq!(PriceTiers::new(vec![(1, money(0))]).err(), Some(Error::NonPositivePrice));
    assert_eq!(
        PriceTiers::new(vec![(1, money(1000)), (5, money(-1))]).err(),
        Some(Error::NonPositivePrice)
    );
}

#[test]
fn part5_zero_quantity_is_rejected() {
    let tiers = PriceTiers::new(vec![(1, money(1000))]).unwrap();
    assert_eq!(tiers.unit_price(0).err(), Some(Error::ZeroQuantity));
    assert_eq!(tiers.line_total(0).err(), Some(Error::ZeroQuantity));
}

#[test]
fn part5_line_total_overflow_is_an_error() {
    let tiers = PriceTiers::new(vec![(1, money(i64::MAX / 2))]).unwrap();
    assert_eq!(tiers.line_total(3), Err(Error::Overflow));
}

#[test]
fn part5_sla_bands_use_inclusive_upper_bounds() {
    let cases = [
        (0.0, SlaTier::Critical),
        (15.0, SlaTier::Critical),
        (15.1, SlaTier::High),
        (60.0, SlaTier::High),
        (60.5, SlaTier::Normal),
        (240.0, SlaTier::Normal),
        (240.1, SlaTier::Low),
        (f64::INFINITY, SlaTier::Low),
    ];
    for (minutes, want) in cases {
        assert_eq!(sla_tier(minutes), Ok(want), "response time {minutes} minutes");
    }
}

#[test]
fn part5_sla_rejects_negative_and_nan_response_times() {
    assert_eq!(sla_tier(-1.0), Err(Error::InvalidResponseTime(-1.0)));
    assert!(
        matches!(sla_tier(f64::NAN), Err(Error::InvalidResponseTime(v)) if v.is_nan()),
        "NaN must be rejected"
    );
}

// ---------------------------------------------------------------- Part 6: records and grouping

#[test]
fn part6_shipment_accessors_and_with_qty_returns_a_new_record() {
    let s = Shipment::new("sku-1", 3, "dock-2");
    assert_eq!((s.sku(), s.qty(), s.dest()), ("sku-1", 3, "dock-2"));
    let w = s.with_qty(9);
    assert_eq!(w.qty(), 9);
    assert_eq!(w.sku(), "sku-1", "other fields are carried over");
    assert_eq!(s.qty(), 3, "the original is unchanged");
}

#[test]
fn part6_shipment_equality_and_hash_follow_fields() {
    let a = Shipment::new("a", 1, "x");
    let b = Shipment::new(String::from("a"), 1, "x");
    assert_eq!(a, b);
    assert_eq!(hash_of(&a), hash_of(&b));
    assert_ne!(a, Shipment::new("a", 2, "x"));
}

#[test]
fn part6_group_by_orders_groups_by_first_seen_key_and_keeps_item_order() {
    let items = [("b", 1), ("a", 2), ("b", 3), ("c", 4), ("a", 5)];
    let groups = group_by(items, |item| item.0);
    assert_eq!(
        groups,
        vec![
            ("b", vec![("b", 1), ("b", 3)]),
            ("a", vec![("a", 2), ("a", 5)]),
            ("c", vec![("c", 4)]),
        ]
    );
}

#[test]
fn part6_group_by_consumes_input_exactly_once() {
    let mut pulled = 0;
    let groups = group_by(vec![1, 2, 3, 4].into_iter().inspect(|_| pulled += 1), |n| n % 2);
    assert_eq!(pulled, 4, "each item is pulled exactly once");
    assert_eq!(groups, vec![(1, vec![1, 3]), (0, vec![2, 4])]);
    assert!(group_by(Vec::<i32>::new(), |n| *n).is_empty());
}

#[test]
fn part6_lazy_map_creates_each_entry_once() {
    let calls = Rc::new(Cell::new(0));
    let counter = Rc::clone(&calls);
    let mut m = LazyMap::new(move || {
        counter.set(counter.get() + 1);
        Vec::<u32>::new()
    });
    m.get_or_create("a".to_string()).push(1);
    m.get_or_create("a".to_string()).push(2);
    assert_eq!(calls.get(), 1, "the factory runs only for a missing key");
    assert_eq!(m.get("a"), Some(&vec![1, 2]));
    assert_eq!(m.len(), 1);
}

#[test]
fn part6_lazy_map_reads_never_create_entries() {
    let mut m: LazyMap<String, u32, _> = LazyMap::new(|| 0u32);
    assert_eq!(m.get("x"), None);
    assert!(!m.contains_key("x"));
    assert!(m.is_empty(), "get and contains_key must not insert");
    m.get_or_create("x".to_string());
    assert!(m.contains_key("x"));
    assert!(!m.is_empty());
}
