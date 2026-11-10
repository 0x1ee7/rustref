//! Part 2 — lazy log pipeline: parsing, fallible iteration, filtering, enrichment, attributes.

use std::cell::Cell;
use std::collections::HashMap;
use std::io::{self, BufReader, Cursor, Read};
use std::rc::Rc;

use ex05_iterators::log::{
    AttrError, Level, LogParseError, LogRecord, ParseReason, ReadError, UnknownLevel, enrich,
    filter_level, first_error, parse_attrs, parse_line, parse_records, read_records,
};
use jiff::Timestamp;

const TS: &str = "2026-03-01T12:05:00+00:00";

fn line(level: &str, server: &str, msg: &str) -> String {
    format!("{TS} {level} {server} {msg}")
}

fn rec(level: &str, server: &str) -> LogRecord {
    parse_line(&line(level, server, "m")).expect("fixture line must parse")
}

/// Source iterator that counts how many items the consumer has pulled from it.
fn counted<T>(items: Vec<T>, pulls: Rc<Cell<usize>>) -> impl Iterator<Item = T> {
    items.into_iter().inspect(move |_| pulls.set(pulls.get() + 1))
}

/// `Read` wrapper that counts bytes handed to the buffer.
struct Counting<R> {
    inner: R,
    served: Rc<Cell<usize>>,
}

impl<R: Read> Read for Counting<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.served.set(self.served.get() + n);
        Ok(n)
    }
}

struct Failing;

impl Read for Failing {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("disk gone"))
    }
}

#[test]
fn part2_level_order_follows_severity() {
    assert!(Level::Debug < Level::Info, "DEBUG must sort below INFO");
    assert!(Level::Error < Level::Critical, "ERROR must sort below CRITICAL");
    let mut shuffled = vec![Level::Critical, Level::Debug, Level::Warning, Level::Info, Level::Error];
    shuffled.sort();
    assert_eq!(shuffled, Level::ALL.to_vec(), "sorting levels must give severity order");
    assert_eq!(Level::Warning.to_string(), "WARNING", "Display must use the wire spelling");
}

#[test]
fn part2_level_parses_exact_wire_spelling_only() {
    assert_eq!("ERROR".parse::<Level>(), Ok(Level::Error), "ERROR must parse");
    assert_eq!(
        "error".parse::<Level>(),
        Err(UnknownLevel("error".to_string())),
        "parsing is case-sensitive: lower-case is unknown"
    );
}

#[test]
fn part2_level_display_round_trips_and_is_error_flags_severity() {
    for level in Level::ALL {
        assert_eq!(level.to_string().parse::<Level>(), Ok(level), "Display must round-trip via FromStr");
    }
    let errors: Vec<Level> = Level::ALL.into_iter().filter(|l| l.is_error()).collect();
    assert_eq!(errors, vec![Level::Error, Level::Critical], "only ERROR and CRITICAL are errors");
}

#[test]
fn part2_parse_line_extracts_every_field() {
    let rec = parse_line("2026-03-01T12:05:00+00:00 WARNING web-1 disk at 91% on /var").unwrap();
    assert_eq!(rec.ts, "2026-03-01T12:05:00Z".parse::<Timestamp>().unwrap(), "timestamp must be parsed with its offset");
    assert_eq!(rec.level, Level::Warning, "level must be parsed");
    assert_eq!(rec.server, "web-1", "server must be the third field");
    assert_eq!(rec.message, "disk at 91% on /var", "message is everything after the server");
    assert_eq!(rec.region, None, "region is unset by parse_line");
}

#[test]
fn part2_parse_line_trims_and_keeps_internal_message_spacing() {
    let rec = parse_line(&format!("   {}   ", line("INFO", "web-2", "two   spaces  here  "))).unwrap();
    assert_eq!(rec.message, "two   spaces  here", "message keeps interior spacing; outer whitespace is trimmed");
}

#[test]
fn part2_parse_line_rejects_wrong_field_count_with_count() {
    let err = parse_line("2026-03-01T12:05:00+00:00 INFO web-1").unwrap_err();
    assert_eq!(err.reason, ParseReason::FieldCount(3), "three fields must report FieldCount(3)");
    let empty = parse_line("   ").unwrap_err();
    assert_eq!(empty.reason, ParseReason::FieldCount(0), "blank input has zero fields");
}

#[test]
fn part2_parse_line_rejects_naive_timestamp_and_unknown_level() {
    let naive = parse_line("2026-03-01T12:05:00 INFO web-1 msg").unwrap_err();
    assert!(
        matches!(naive.reason, ParseReason::BadTimestamp(_)),
        "a timestamp without an offset must be rejected, got {naive:?}"
    );
    let level = parse_line(&line("NOTICE", "web-1", "msg")).unwrap_err();
    assert_eq!(level.reason, ParseReason::UnknownLevel("NOTICE".into()), "unknown level must be reported by name");
    assert_eq!(level.line, None, "errors from parse_line have no line number");
}

#[test]
fn part2_parse_records_skips_blanks_and_numbers_lines_by_position() {
    let input = vec![line("INFO", "a", "one"), String::new(), "   ".to_string(), "bad line".to_string(), line("ERROR", "b", "two")];
    let items: Vec<Result<LogRecord, LogParseError>> = parse_records(input).collect();
    assert_eq!(items.len(), 3, "blank lines are skipped entirely");
    assert!(items[0].is_ok(), "first record parses");
    let err = items[1].as_ref().unwrap_err();
    assert_eq!(err.line, Some(4), "line number counts blank lines too (1-based position)");
    assert_eq!(err.to_string(), "line 4: expected 4 fields, found 2", "Display must be 'line N: reason'");
    assert!(items[2].is_ok(), "parsing continues after a malformed line");
}

#[test]
fn part2_parse_records_is_lazy_over_its_input() {
    let pulls = Rc::new(Cell::new(0));
    let source = counted(vec![line("INFO", "a", "1"), line("INFO", "a", "2"), line("INFO", "a", "3")], pulls.clone());
    let mut records = parse_records(source);
    records.next().expect("first record").expect("first line parses");
    assert_eq!(pulls.get(), 1, "one record must consume exactly one line");
}

#[test]
fn part2_collect_into_result_stops_at_first_bad_line() {
    let pulls = Rc::new(Cell::new(0));
    let source = counted(vec![line("INFO", "a", "1"), "garbage".to_string(), line("INFO", "a", "3")], pulls.clone());
    let all: Result<Vec<LogRecord>, LogParseError> = parse_records(source).collect();
    assert_eq!(all.unwrap_err().line, Some(2), "collect::<Result<_, _>>() must surface the first error");
    assert_eq!(pulls.get(), 2, "collect must short-circuit: the line after the error is never read");
}

#[test]
fn part2_read_records_does_not_read_ahead_of_the_consumer() {
    let l = line("INFO", "a", "first");
    let input = format!("{l}\n{l}\n{l}\n");
    let served = Rc::new(Cell::new(0));
    let reader = BufReader::with_capacity(1, Counting { inner: Cursor::new(input), served: served.clone() });
    let mut records = read_records(reader);
    records.next().expect("first record").expect("first line parses");
    assert_eq!(
        served.get(),
        l.len() + 1,
        "after one record only the bytes up to and including the first newline may have been read"
    );
}

#[test]
fn part2_read_records_surfaces_io_failures() {
    let mut records = read_records(BufReader::new(Failing));
    match records.next() {
        Some(Err(ReadError::Io(e))) => assert_eq!(e.to_string(), "disk gone", "the io::Error must be passed through"),
        other => panic!("expected an io error, got {other:?}"),
    }
}

#[test]
fn part2_read_records_parses_a_reader_line_by_line() {
    let input = format!("{}\n\n{}", line("INFO", "a", "x"), line("ERROR", "b", "y"));
    let got: Vec<LogRecord> = read_records(Cursor::new(input)).map(|r| r.expect("valid input")).collect();
    let servers: Vec<&str> = got.iter().map(|r| r.server.as_str()).collect();
    assert_eq!(servers, vec!["a", "b"], "blank lines are skipped and records keep order");
}

#[test]
fn part2_filter_level_keeps_at_or_above_minimum() {
    let records = vec![rec("DEBUG", "a"), rec("WARNING", "a"), rec("ERROR", "b"), rec("INFO", "a")];
    let kept: Vec<Level> = filter_level(records.into_iter(), "WARNING")
        .expect("WARNING is a known level")
        .map(|r| r.level)
        .collect();
    assert_eq!(kept, vec![Level::Warning, Level::Error], "filter keeps WARNING and above, in order");
}

#[test]
fn part2_filter_level_rejects_unknown_minimum_at_call_time() {
    let never_pulled = std::iter::from_fn(|| -> Option<LogRecord> { panic!("source must not be touched") });
    let result = filter_level(never_pulled, "NOTICE");
    assert!(result.is_err(), "an unknown minimum must be reported by the call itself, not on first next()");
}

#[test]
fn part2_filter_level_is_lazy() {
    let pulls = Rc::new(Cell::new(0));
    let source = counted(vec![rec("INFO", "a"), rec("ERROR", "b"), rec("INFO", "c")], pulls.clone());
    let mut kept = filter_level(source, "ERROR").unwrap();
    assert_eq!(kept.next().map(|r| r.server), Some("b".to_string()), "first match is the ERROR record");
    assert_eq!(pulls.get(), 2, "the filter must stop reading the source at the first match");
}

#[test]
fn part2_enrich_sets_region_and_defaults_unknown() {
    let regions: HashMap<String, String> = [("web-1".to_string(), "eu-west".to_string())].into_iter().collect();
    let out: Vec<Option<String>> = enrich(vec![rec("INFO", "web-1"), rec("INFO", "web-9")].into_iter(), &regions)
        .map(|r| r.region)
        .collect();
    assert_eq!(
        out,
        vec![Some("eu-west".to_string()), Some("unknown".to_string())],
        "known servers get their region; missing servers get \"unknown\""
    );
}

#[test]
fn part2_enrich_is_lazy() {
    let regions = HashMap::new();
    let pulls = Rc::new(Cell::new(0));
    let source = counted(vec![rec("INFO", "a"), rec("INFO", "b"), rec("INFO", "c")], pulls.clone());
    let first = enrich(source, &regions).next();
    assert!(first.is_some(), "enrich must yield a record");
    assert_eq!(pulls.get(), 1, "enrich must pull one record per record consumed");
}

#[test]
fn part2_first_error_stops_at_the_match() {
    let pulls = Rc::new(Cell::new(0));
    let source = counted(vec![rec("INFO", "a"), rec("WARNING", "a"), rec("ERROR", "b"), rec("CRITICAL", "c")], pulls.clone());
    let found = first_error(source).expect("an ERROR record exists");
    assert_eq!(found.server, "b", "first ERROR-or-worse record is returned");
    assert_eq!(pulls.get(), 3, "search must stop at the match, not drain the source");
}

#[test]
fn part2_first_error_is_none_when_clean() {
    assert_eq!(first_error(vec![rec("INFO", "a"), rec("WARNING", "a")]), None, "no error-level record means None");
}

#[test]
fn part2_parse_attrs_reads_key_value_pairs() {
    let attrs = parse_attrs("user=alice retries=3 empty=").unwrap();
    let pairs: Vec<(&str, &str)> = attrs.iter().map(|a| (a.key.as_str(), a.value.as_str())).collect();
    assert_eq!(pairs, vec![("user", "alice"), ("retries", "3"), ("empty", "")], "pairs in order; `key=` is an empty value");
}

#[test]
fn part2_parse_attrs_quoted_values_allow_spaces_and_escapes() {
    let attrs = parse_attrs(r#"msg="disk full on /var" note="say \"hi\" \\ bye""#).unwrap();
    assert_eq!(attrs[0].value, "disk full on /var", "quoted value may contain spaces");
    assert_eq!(attrs[1].value, r#"say "hi" \ bye"#, "\\\" and \\\\ are escapes inside quotes");
}

#[test]
fn part2_parse_attrs_empty_or_blank_input_is_empty() {
    assert_eq!(parse_attrs("").unwrap(), vec![], "empty input has no attributes");
    assert_eq!(parse_attrs("   ").unwrap(), vec![], "whitespace-only input has no attributes");
}

#[test]
fn part2_parse_attrs_reports_offsets_of_bad_tokens() {
    assert_eq!(
        parse_attrs("x=1 bare y=2"),
        Err(AttrError::MissingEquals { at: 4 }),
        "a token without '=' is reported at its byte offset"
    );
    assert_eq!(parse_attrs("=v"), Err(AttrError::EmptyKey { at: 0 }), "a token starting with '=' has an empty key");
    assert_eq!(
        parse_attrs(r#"a="open"#),
        Err(AttrError::UnterminatedQuote { at: 0 }),
        "an unclosed quote is reported at the token start"
    );
}
