//! Part 5: semicolon-delimited CSV reports via `csv` + serde.

use std::collections::BTreeMap;

use ex12_files_and_serde::hash::Sha256;
use ex12_files_and_serde::report::{ReportRow, read_report, write_report};
use ex12_files_and_serde::scan::{FileEntry, Manifest};
use ex12_files_and_serde::Error;
use jiff::Timestamp;

const HEADER: &str = "path;size;mtime;sha256;note\n";
const MTIME_SECS: i64 = 1_704_067_200; // 2024-01-01T00:00:00Z

fn sample_manifest() -> Manifest {
    let mtime = Timestamp::from_second(MTIME_SECS).expect("valid timestamp");
    let mut m = Manifest::new();
    m.insert(
        "b.txt".to_owned(),
        FileEntry { size: 7, mtime, sha256: Sha256::digest(b"bravo!!") },
    );
    m.insert(
        "a.txt".to_owned(),
        FileEntry { size: 3, mtime, sha256: Sha256::digest(b"abc") },
    );
    m
}

fn render(manifest: &Manifest, notes: &BTreeMap<String, String>) -> String {
    let mut buf = Vec::new();
    write_report(&mut buf, manifest, notes).expect("write report");
    String::from_utf8(buf).expect("report is UTF-8")
}

#[test]
fn part5_header_is_semicolon_delimited_with_lf_terminators() {
    let text = render(&sample_manifest(), &BTreeMap::new());
    assert!(text.starts_with(HEADER), "header row must be exactly {HEADER:?}, got {text:?}");
    assert!(!text.contains('\r'), "output must use LF line endings");
}

#[test]
fn part5_empty_manifest_still_writes_the_header() {
    let text = render(&Manifest::new(), &BTreeMap::new());
    assert_eq!(text, HEADER, "an empty report is just the header");
}

#[test]
fn part5_rows_follow_path_order_and_default_note_is_empty() {
    let mut notes = BTreeMap::new();
    notes.insert("b.txt".to_owned(), "reviewed".to_owned());

    let mut buf = Vec::new();
    write_report(&mut buf, &sample_manifest(), &notes).expect("write");
    let rows = read_report(buf.as_slice()).expect("read back");

    assert_eq!(rows.len(), 2, "one row per file");
    assert_eq!(rows[0].path, "a.txt", "rows are in path order");
    assert_eq!(rows[0].note, "", "a missing note is an empty field");
    assert_eq!(rows[1].path, "b.txt");
    assert_eq!(rows[1].note, "reviewed", "a supplied note is written");
    assert_eq!(rows[1].size, 7);
    assert_eq!(rows[1].sha256, Sha256::digest(b"bravo!!"));
}

#[test]
fn part5_fields_containing_the_delimiter_are_quoted_and_round_trip() {
    let mut notes = BTreeMap::new();
    notes.insert("a.txt".to_owned(), "needs a; b".to_owned());
    let text = render(&sample_manifest(), &notes);
    assert!(text.contains("\"needs a; b\""), "a field with ';' must be quoted, got {text:?}");

    let rows = read_report(text.as_bytes()).expect("read back");
    assert_eq!(rows[0].note, "needs a; b", "quoting must round-trip");
}

#[test]
fn part5_unknown_note_key_is_rejected_before_any_output() {
    let mut notes = BTreeMap::new();
    notes.insert("ghost.txt".to_owned(), "not in manifest".to_owned());

    let mut buf = Vec::new();
    let result = write_report(&mut buf, &sample_manifest(), &notes);
    assert!(matches!(&result, Err(Error::UnknownNote(k)) if k == "ghost.txt"), "got {result:?}");
    assert!(buf.is_empty(), "nothing may be written when validation fails");
}

#[test]
fn part5_read_accepts_utf8_bom_and_crlf_line_endings() {
    let hex = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let text = format!(
        "\u{feff}path;size;mtime;sha256;note\r\na.txt;3;2024-01-01T00:00:00Z;{hex};\r\n"
    );
    let rows = read_report(text.as_bytes()).expect("BOM and CRLF are valid");
    assert_eq!(
        rows,
        [ReportRow {
            path: "a.txt".to_owned(),
            size: 3,
            mtime: Timestamp::from_second(MTIME_SECS).expect("valid timestamp"),
            sha256: Sha256::digest(b"abc"),
            note: String::new(),
        }],
    );
}

#[test]
fn part5_read_rejects_a_wrong_header_with_report_error() {
    let text = "file;size;mtime;sha256;note\na.txt;3;2024-01-01T00:00:00Z;x;\n";
    assert!(
        matches!(read_report(text.as_bytes()), Err(Error::Report(_))),
        "a header mismatch must be a Report error"
    );
}

#[test]
fn part5_read_rejects_malformed_numbers_with_csv_error() {
    let hex = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let text = format!("{HEADER}a.txt;three;2024-01-01T00:00:00Z;{hex};\n");
    assert!(
        matches!(read_report(text.as_bytes()), Err(Error::Csv(_))),
        "a non-numeric size must surface as a Csv error"
    );
}
