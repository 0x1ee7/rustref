# 12 — Files and serde

## Scenario

You maintain `treesync`, a small tool for mirroring a build-artifact tree to a
backup volume. Each run scans the source tree into a **manifest** (path, size,
mtime, SHA-256), diffs it against the last manifest, copies what changed, and
records what it did. Operators want readable reports for a spreadsheet, a
streamed JSON manifest that can be large, a binary snapshot with a checksum,
and writes that never leave a half-written file behind after a crash.

The crate exposes the library; the binary that wires it to `clap` is out of
scope here.

## Part 1 — Streaming hashes, checksums, scanning

Stream file contents through `Sha256` and `Crc32` instead of reading whole
files. `scan_tree` walks a directory without recursion, applies glob-style
ignore rules, skips symlinks, rejects non-UTF-8 names, and truncates mtimes to
microseconds.

- `Sha256::{new, update, finalize, digest}`, `io::Write`, `Display` as lowercase hex.
- `sha256_file(&Path)`: streamed, errors carry the path.
- `Crc32::{new, update, finalize}`, `crc32(&[u8])`.
- `glob::full_match` / `glob::is_ignored`: `*` and `?` stay inside a segment,
  `**` spans segments, a bare pattern matches any component, a pattern with `/`
  is anchored at the root.
- `scan_tree(root, ignore)`: keys are relative, `/`-separated and sorted.
  Symlinks are skipped. Non-UTF-8 names are `Error::NonUtf8Path`.

Concepts: `io::Write` as a sink, `Digest` newtype with `Display`, `const fn`
tables, `DirEntry::file_type` (no extra `stat`), error variants that carry the
path.

## Part 2 — Diffing and applying changes safely

`diff_manifests` classifies paths into added, removed, and modified. A modified
file is one whose size or digest changed. An mtime-only change is not a
modification. `apply_diff` copies and deletes under `dest`, and validates every
key before it touches the disk.

- `ManifestDiff { added, removed, modified }` (borrowed `&str` keys), `is_empty()`.
- `validate_relative_path(&str)`: rejects empty, absolute, `.`, `..`, empty
  segments, trailing `/`, and `\` with `Error::UnsafePath`.
- `apply_diff(&diff, source, dest) -> ApplyResult { copied, deleted }`: copies
  via `write_atomic`, then sets the source mtime. Missing removals are skipped.
  Emptied parent directories are pruned up to, but not including, `dest`.

Concepts: borrowing keys from the diff instead of cloning them, validate-then-act,
`std::path` components rather than string checks alone, `set_modified`.

## Part 3 — Serde wire formats

Four types, four serde styles.

- `FileEntry { size, mtime, sha256 }`: camelCase, `deny_unknown_fields`.
  `mtime` uses a `with` codec (RFC 3339 UTC). `sha256` accepts 64 hex digits in
  either case.
- Manifest as a JSON array of `{ "path": ..., "entry": { ... } }` records.
  `write_manifest` streams it. `read_manifest_each` calls back for each record
  without building a map, and returns the first callback error unchanged.
  `read_manifest` rejects duplicate paths with `Error::Manifest`.
- `Event` is internally tagged: `{"event":"copied","path":"..."}`. `Skipped`
  omits `reason` when it is `None`.
- `SyncConfig` is kebab-case, `deny_unknown_fields`, and goes through
  `#[serde(try_from)]`. `ignore` takes strings or `{ "pattern", "reason" }`
  objects (untagged). `retention-days` defaults to 30 and must be ≤ 36500.

Concepts: `#[serde(with)]`, `skip_serializing_if`, `default`, `rename_all`,
`deny_unknown_fields`, internally tagged enums, untagged enums, `try_from`,
streaming `Visitor`s, and a callback error that stops the stream.

## Part 4 — Atomic writes

Writes go to a temp file in the same directory, are fsynced, then renamed over
the target. A failure at any step leaves the old file in place and removes the
temp file.

- `write_atomic(&Path, impl FnOnce(&mut dyn Write) -> io::Result<()>)`.
- `save_json_atomic<T: Serialize>(&T, &Path)`: pretty JSON with a trailing newline.
- `load_json<T: DeserializeOwned>(&Path)`.

Concepts: `tempfile::NamedTempFile::persist`, why the temp file must sit on the
same filesystem, `Drop` as cleanup, `sync_all`, error propagation through
closures.

## Part 5 — CSV reports

`write_report` produces a `;`-separated CSV with the header
`path;size;mtime;sha256;note`, one row per file in path order, and LF
terminators. Notes are supplied per path and validated before anything is
written. `read_report` strips a UTF-8 BOM, accepts CRLF, and checks the header.

- `ReportRow` derives `Deserialize` and uses the same codecs as Part 3.
- `write_report` returns `Error::UnknownNote` for a note whose key is not in
  the manifest, and writes nothing in that case.
- `read_report` returns `Error::Report` for a bad header, and `Error::Csv` for
  malformed fields.

Concepts: `csv::WriterBuilder` / `ReaderBuilder`, serde integration, quoting
rules for the delimiter, BOM handling, validating input before side effects.

## Part 6 — Binary snapshot with a checksum

A snapshot file has a fixed binary layout, so other tools can read it without
serde.

v2 header (24 bytes, all integers big-endian):

| offset | size | field                       |
|--------|------|-----------------------------|
| 0      | 4    | magic `RSSN`                |
| 4      | 2    | version (`2`)               |
| 6      | 2    | flags                       |
| 8      | 4    | entry count                 |
| 12     | 8    | created time, i64 micros    |
| 20     | 4    | CRC-32 of the payload       |

The payload follows the header. v1 (`version = 1`) is 12 bytes, with no
timestamp and no checksum, and is read-only.

- `encode_snapshot(entry_count, created_at, payload, flags) -> Result<Vec<u8>, SnapshotError>`:
  `FieldOverflow` if the entry count or flags do not fit.
- `decode_snapshot(&[u8]) -> Result<Snapshot, SnapshotError>`: checks length,
  magic, version, checksum, then timestamp range, in that order.
- `write_snapshot` / `read_snapshot` for files.

Concepts: `to_be_bytes` / `from_be_bytes`, explicit wire layout, distinguishing
structural errors (`BadMagic`, `Truncated`, `UnsupportedVersion`) from integrity
errors (`ChecksumMismatch`), a `SnapshotError` that is a `transparent` variant
of the crate `Error`.

## Concepts exercised

- Streaming I/O with `Read`, `Write`, `BufReader`, `BufWriter`, and `io::copy`.
- `std::fs` and `std::path`: components, `DirEntry::file_type`, `set_modified`.
- Error design: `thiserror` enums with path context, `#[from]`, `transparent`.
- Serde: `with`, `rename_all`, `skip_serializing_if`, `default`,
  `deny_unknown_fields`, tagged and untagged enums, `try_from`, streaming visitors.
- `csv` with a non-default delimiter, BOM and CRLF handling.
- Atomic file replacement with `tempfile::NamedTempFile::persist`.
- Binary formats: big-endian integers, versioning, CRC-32.
- Newtypes with `Display`, `const fn` lookup tables, `Option` for version-dependent fields.

## Run

```sh
cargo test -p ex12_files_and_serde                 # all tests (they fail until you implement the stubs)
cargo test -p ex12_files_and_serde part2           # one Part; test names are prefixed partN_
cargo solutions -p ex12_files_and_serde            # run the tests against the reference solution
```

## Stretch (untested)

- Make `scan_tree` parallel with `rayon`-style work splitting, and measure before adopting it.
- Add a `--follow-symlinks` mode that detects cycles by device and inode.
- Port manifest streaming to an async variant over `tokio::fs`, keeping the callback contract of `read_manifest_each`.
- Add a v3 snapshot with per-entry records and a footer checksum.
- Compare the CRC-32 implementation with the `crc32fast` crate's throughput on a 1 GiB file.
