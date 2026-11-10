//! Part 1: zero-allocation log-line parser with borrowed fields.
mod common;

use common::count_allocations;
use ex19_performance::log_line::{Field, Level, LogLine, ParseError, parse_line, parse_lines};

const LINE: &str = "2026-11-10T12:00:00Z INFO api::http request completed status=200";

#[test]
fn part1_parses_all_four_fields() {
    let line = parse_line(LINE).expect("valid line must parse");
    assert_eq!(line.timestamp, "2026-11-10T12:00:00Z");
    assert_eq!(line.level, Level::Info);
    assert_eq!(line.target, "api::http");
    assert_eq!(
        line.message, "request completed status=200",
        "message is everything after the target"
    );
}

#[test]
fn part1_message_keeps_internal_spacing_and_may_be_empty() {
    let spaced = parse_line("2026-11-10T12:00:01Z ERROR svc::db   deadlock   retry  ").unwrap();
    assert_eq!(spaced.target, "svc::db");
    assert_eq!(spaced.message, "deadlock   retry", "internal spacing kept, trailing trimmed");

    let bare = parse_line("2026-11-10T12:00:02Z WARN db::pool").unwrap();
    assert_eq!(bare.target, "db::pool");
    assert_eq!(bare.message, "", "a line ending at the target has an empty message");
}

#[test]
fn part1_ignores_surrounding_whitespace_and_crlf() {
    let line = parse_line("  2026-11-10T12:00:03Z\tDEBUG  cache::lru evict  \r\n").unwrap();
    assert_eq!(line.level, Level::Debug);
    assert_eq!(line.target, "cache::lru");
    assert_eq!(line.message, "evict");
}

#[test]
fn part1_reports_the_first_missing_field() {
    assert_eq!(
        parse_line("").unwrap_err(),
        ParseError::MissingField(Field::Timestamp)
    );
    assert_eq!(
        parse_line("   ").unwrap_err(),
        ParseError::MissingField(Field::Timestamp)
    );
    assert_eq!(
        parse_line("2026-11-10T12:00:00Z").unwrap_err(),
        ParseError::MissingField(Field::Level)
    );
    assert_eq!(
        parse_line("2026-11-10T12:00:00Z INFO").unwrap_err(),
        ParseError::MissingField(Field::Target)
    );
}

#[test]
fn part1_unknown_level_is_reported_case_sensitively() {
    assert_eq!(
        parse_line("2026-11-10T12:00:00Z NOTICE a::b hi").unwrap_err(),
        ParseError::UnknownLevel("NOTICE".to_owned())
    );
    assert_eq!(
        parse_line("2026-11-10T12:00:00Z info a::b hi").unwrap_err(),
        ParseError::UnknownLevel("info".to_owned()),
        "level tokens are upper-case only"
    );
}

#[test]
fn part1_fields_borrow_from_the_input_buffer() {
    let input = String::from(LINE);
    let start = input.as_ptr() as usize;
    let end = start + input.len();
    let line: LogLine<'_> = parse_line(&input).unwrap();
    for (name, field) in [
        ("timestamp", line.timestamp),
        ("target", line.target),
        ("message", line.message),
    ] {
        let ptr = field.as_ptr() as usize;
        assert!(
            ptr >= start && ptr + field.len() <= end,
            "{name} must be a sub-slice of the input, not a copy"
        );
    }
}

#[test]
fn part1_parse_line_success_allocates_nothing() {
    let (result, allocs) = count_allocations(|| parse_line(LINE));
    assert!(result.is_ok(), "line must parse");
    assert_eq!(allocs, 0, "successful parse must not allocate");
}

#[test]
fn part1_iterator_skips_blank_lines_and_yields_errors_in_place() {
    let text = "2026-11-10T12:00:00Z INFO a::b one\n\n   \n\
                2026-11-10T12:00:01Z BOGUS a::b two\n\
                2026-11-10T12:00:02Z ERROR a::b three\n";
    let items: Vec<_> = parse_lines(text).collect();
    assert_eq!(items.len(), 3, "blank lines are skipped, malformed lines are not");
    assert_eq!(items[0].as_ref().unwrap().message, "one");
    assert!(
        matches!(&items[1], Err(ParseError::UnknownLevel(token)) if token == "BOGUS"),
        "bad line must be yielded as Err in place: {:?}",
        items[1]
    );
    assert_eq!(items[2].as_ref().unwrap().level, Level::Error);
}

#[test]
fn part1_scanning_a_large_buffer_allocates_nothing() {
    let text: String = (0..1000)
        .map(|i| format!("2026-11-10T12:00:{:02}Z INFO svc::worker tick {i}\n", i % 60))
        .collect();
    let (ok, allocs) = count_allocations(|| parse_lines(&text).filter(Result::is_ok).count());
    assert_eq!(ok, 1000, "every generated line must parse");
    assert_eq!(allocs, 0, "iterating a whole buffer must not allocate");
}

#[test]
fn part1_level_tokens_round_trip_and_are_ordered() {
    for (level, token) in [
        (Level::Trace, "TRACE"),
        (Level::Debug, "DEBUG"),
        (Level::Info, "INFO"),
        (Level::Warn, "WARN"),
        (Level::Error, "ERROR"),
    ] {
        assert_eq!(level.as_str(), token);
        let text = format!("2026-11-10T12:00:00Z {token} svc x");
        let line = parse_line(&text).unwrap();
        assert_eq!(line.level, level, "token {token} must parse back");
    }
    assert!(Level::Trace < Level::Error, "levels are ordered least to most severe");
    assert!(Level::Warn >= Level::Info);
}

#[test]
fn part1_parse_error_implements_std_error_with_useful_display() {
    fn assert_error<E: std::error::Error + Send + Sync + 'static>(_: &E) {}

    let missing = parse_line("2026-11-10T12:00:00Z").unwrap_err();
    assert_error(&missing);
    assert!(
        missing.to_string().contains("level"),
        "Display must name the missing field, got {missing}"
    );

    let unknown = parse_line("2026-11-10T12:00:00Z NOPE a b").unwrap_err();
    assert!(
        unknown.to_string().contains("NOPE"),
        "Display must quote the offending token, got {unknown}"
    );
}
