# 17 · Dates and times with jiff

A scheduling and billing backend. Timestamps arrive from logs, clients, and
payment providers in mixed formats. Meetings recur in local time across DST.
Bookings span time zones. Invoices bill on month ends. Business-day SLAs skip
weekends and holidays. Every "now" is injected, and nothing in these tests
depends on the machine's local time zone.

Implement everything in `src/`. The crate uses [`jiff`](https://docs.rs/jiff)
for calendar and zone work and `std::time` for the monotonic stopwatch.

The type system does most of the policing that a Python port would do at
runtime. A `Timestamp` or `Zoned` is always an absolute instant, and a
`civil::DateTime` never is. Where the Python original raised `ValueError` for a
naive datetime, the Rust API has no such input. Remaining invalid input returns
`TimeError::InvalidInput`.

Key rules:

- Compare, subtract, and merge instants by absolute time. Two `Zoned` values in
  the same zone still compare by instant, not by wall-clock fields.
- Keep calendar arithmetic (`Span` with days or months) separate from elapsed
  arithmetic (`SignedDuration`, which is exact time). The two differ across DST.
- Never call the system zone. Every zone is named explicitly with an IANA key.

## Part 1: instants, zones, explicit disambiguation

Module `zones`.

- `to_utc(zoned: &Zoned) -> Zoned`: the same instant expressed in UTC.
- `localize(wall: civil::DateTime, zone: &str, policy: Disambiguation) -> Result<Zoned, TimeError>`:
  attach `zone` to a wall time. `Disambiguation` is `jiff::tz::Disambiguation`.
  - Unknown zone: `TimeError::UnknownZone`.
  - Unambiguous wall time: the single occurrence, whatever the policy.
  - Gap (the wall time never happens; DST spring-forward):
    - `Compatible` shifts forward by the gap length, so `02:30` becomes `03:30` in the post-transition offset.
    - `Earlier`, `Later`, and `Reject` return `TimeError::Nonexistent`.
  - Overlap (the wall time happens twice; DST fall-back):
    - `Compatible` and `Earlier` pick the first occurrence (pre-transition offset).
    - `Later` picks the second occurrence.
    - `Reject` returns `TimeError::Ambiguous`.
- `in_zone(ts: Timestamp, zone: &str) -> Result<Zoned, TimeError>`: the instant
  as a wall time in `zone`.
- `format_in_zone(ts: Timestamp, zone: &str) -> Result<String, TimeError>`:
  `"%Y-%m-%d %H:%M %Z"` in `zone`, e.g. `"2026-03-29 14:00 CEST"`.

## Part 2: timestamp parsing and epoch conversion

Module `parsing`.

`RawTimestamp<'a>` is the heterogeneous input: `Text(&str)`, `Integer(i64)`,
`Float(f64)`. It implements `From` for `&str`, `i64`, and `f64`, so
`parse_timestamp("...")`, `parse_timestamp(1_700_000_000_i64)`, and
`parse_timestamp(1.5_f64)` all work. (Integer literals need an `_i64` suffix.)

`parse_timestamp(raw) -> Result<Timestamp, TimeError>`:

- `Integer` and `Float`, and text that is entirely a number, are Unix epoch
  values. Magnitude `>= 100_000_000_000` means milliseconds; otherwise seconds.
  Floats keep fractional seconds (nanosecond precision). Non-finite floats
  (`NaN`, `±inf`) are `InvalidInput`.
- Text whose first comma-separated token is a weekday (`"Sun, 29 Mar 2026 12:00:00 +0200"`)
  is RFC 2822, parsed with `jiff::fmt::rfc2822`. An offset of `-0000` (unknown
  local offset, RFC 2822 §3.3) is `InvalidInput`. Note: jiff itself treats
  `-0000` as UTC, so the check is yours to make.
- Any other text is RFC 3339 / ISO 8601 with a mandatory offset (`Z` included).
  Naive or date-only input is `InvalidInput`.
- Surrounding whitespace is ignored. Empty or whitespace-only text is `InvalidInput`.
- Every successful result is a `Timestamp`, which is UTC by definition.

`to_epoch_ms(ts: Timestamp) -> i64`: whole milliseconds since the Unix epoch,
truncated toward the past (floor), so `-1.5 ms` gives `-2`.

## Part 3: wall-clock vs elapsed arithmetic

Module `arithmetic`.

- `add_calendar_days(zoned: &Zoned, days: i64) -> Result<Zoned, TimeError>`:
  move by calendar days. The wall-clock time and zone are kept and the offset is
  recomputed. The result uses fold-0 semantics, meaning `Compatible`, so an
  ambiguous result picks the earlier occurrence and a gap shifts forward.
- `add_elapsed(zoned: &Zoned, delta: SignedDuration) -> Result<Zoned, TimeError>`:
  move by exact elapsed time. The result is in `zoned`'s zone. Overflow of the
  jiff range is `TimeError::Jiff`.
- `elapsed_between(start: &Zoned, end: &Zoned) -> SignedDuration`: `end - start`
  as absolute time. Signed.

## Part 4: calendars: month arithmetic, billing, business days

Module `calendar`.

Month arithmetic:
- `days_in_month(year: i16, month: i8) -> Result<i8, TimeError>`: leap-year aware
  (proleptic Gregorian). Month outside `1..=12` is `InvalidInput`.
- `add_months(date: civil::Date, months: i32) -> Result<civil::Date, TimeError>`:
  calendar months with the day clamped to the target month's length
  (`2026-01-31 + 1 month = 2026-02-28`). Negative values allowed.
- `billing_dates(anchor: civil::Date, count: usize) -> Result<Vec<civil::Date>, TimeError>`:
  `count` dates, each computed from `anchor` with `add_months(anchor, i)`, not
  stepped from the previous date, so there is no drift. The first element is
  `anchor`. `count == 0` gives an empty `Vec`.

Business days: `BusinessCalendar`.
- `BusinessCalendar::new(holidays)`: holidays are `civil::Date`s. The default
  weekend is Saturday and Sunday. Duplicates are harmless.
- `with_weekend(self, weekend: impl IntoIterator<Item = Weekday>) -> Self`:
  replace the weekend set (e.g. Friday and Saturday).
- `is_business_day(&self, date) -> bool`: not in the weekend, not a holiday.
- `add_business_days(&self, date, n: i64) -> Result<civil::Date, TimeError>`:
  step `n` business days forward (or back if negative). Each step lands on a
  business day, so from a weekend start the first forward step goes to the next
  business day. `n == 0` returns `date` unchanged, even on a weekend. If every
  weekday is weekend and `n != 0`, return `InvalidInput`. Do not loop forever.
- `business_days_between(&self, start, end) -> i64`: number of business days in
  `(start, end]`. If `end < start`, return the negated count of `(end, start]`.

## Part 5: recurring meetings with an explicit DST policy

Module `meetings`.

- `enum Gap { Skip, ShiftForward }`: policy for a wall time in a gap.
  `ShiftForward` uses the instant the clock jumps to (`02:30` becomes `03:30 CEST`).
- `enum Overlap { Earlier, Later, Skip }`: policy for a wall time that occurs twice.
- `Recurrence::new(start: civil::Date, weekday: Weekday, at: civil::Time, zone: &str) -> Result<Recurrence, TimeError>`:
  defaults `Gap::ShiftForward` and `Overlap::Earlier`. Unknown zone: `UnknownZone`.
- `.gap(self, Gap) -> Self` and `.overlap(self, Overlap) -> Self`: builder setters.
- `occurrences(&self, count: usize) -> Result<Vec<Zoned>, TimeError>`:
  the next `count` occurrences on `weekday` on or after `start`, at wall time `at`
  in `zone`, in ascending order. An occurrence dropped by `Gap::Skip` or
  `Overlap::Skip` does not count toward `count`.

## Part 6: booking slots across time zones

Module `slots`.

- `TimeSlot::new(start: Zoned, end: Zoned) -> Result<TimeSlot, TimeError>`:
  half-open `[start, end)`. `end` must be strictly after `start`, or
  `InvalidInput`. Each bound keeps its own zone for display.
- `start(&self) -> &Zoned`, `end(&self) -> &Zoned`.
- `TimeSlot::overlaps(&self, other: &TimeSlot) -> bool`: shares at least one
  instant. Touching slots (`a.end == b.start`) do not overlap.
- `merge_slots(slots: impl IntoIterator<Item = TimeSlot>) -> Vec<TimeSlot>`:
  sorted by start; overlapping or touching slots are combined. Results are
  normalised to UTC.
- `total_booked(slots: impl IntoIterator<Item = TimeSlot>) -> SignedDuration`:
  total covered time after merging, so overlapped time counts once.

## Part 7: injected clocks, humanized time, monotonic stopwatch

Modules `clock`, `relative`, `stopwatch`.

Clock:
- `trait Clock { fn now(&self) -> Timestamp; }`
- `SystemClock`: the real wall clock.
- `FixedClock::new(now: Timestamp)`: always returns `now`.

Humanize:
- `humanize(when: &Zoned, now: &Zoned) -> String`. Uses absolute time
  (`when.timestamp() - now.timestamp()`), never wall-clock fields. By
  `|when - now|`:
  - under 45 seconds: `"just now"` (no `in`/`ago`)
  - under 1 hour: minutes, at least 1, e.g. `"1 minute ago"`, `"in 59 minutes"`
  - under 24 hours: hours, e.g. `"3 hours ago"`, `"in 1 hour"`
  - under 30 days: days, e.g. `"2 days ago"`, `"in 1 day"`
  - otherwise: `"on YYYY-MM-DD"`, using `when`'s date in `when`'s own zone (no `in`/`ago`)
  - Future (`when > now`) gets `"in ..."`; past gets `"... ago"`. Units are singular for 1.
- `humanize_with_clock(when: &Zoned, clock: &impl Clock) -> String`: `humanize`
  against `clock.now()`, expressed in `when`'s zone.

Stopwatch:
- `trait MonotonicClock { fn now(&self) -> std::time::Instant; }`
- `SystemMonotonic`: `Instant::now()`.
- `Stopwatch<C: MonotonicClock = SystemMonotonic>`:
  - `Stopwatch::new()`: system monotonic clock. `Stopwatch::with_clock(clock)`: injected clock.
  - `start(&mut self) -> Result<(), TimeError>`: `InvalidState` if already running.
  - `stop(&mut self) -> Result<Duration, TimeError>`: time since `start`. `InvalidState` if not running.
  - `elapsed(&self) -> Duration`: live while running; the last stopped value
    after `stop`; `Duration::ZERO` before the first `start`. After a restart, the
    value is measured from the new `start`.

## Concepts exercised

- `Timestamp` (absolute) vs `Zoned` (instant + zone) vs `civil::DateTime` (wall clock, never an instant)
- `TimeZone::get` through jiff's bundled tz database, no system zone
- DST gaps and overlaps with an explicit `Disambiguation` policy
- `Span` (calendar) vs `SignedDuration` (absolute) arithmetic; month-end clamping
- RFC 3339, RFC 2822, epoch seconds vs milliseconds; `-0000` as "unknown offset"
- `thiserror` error enums wrapping `jiff::Error` with `#[from]`
- Half-open intervals, merging across zones, `impl IntoIterator` inputs
- Injected clocks (trait + fixed implementation), `std::time::Instant` stopwatch

## Run

```
cargo test -p ex17_time_jiff                      # your implementation
cargo test -p ex17_time_jiff part5                # one Part
cargo solutions -p ex17_time_jiff                 # reference implementation
```

## Stretch (untested)

- Accept a `Vec<Recurrence>` and produce a merged, time-ordered agenda across zones.
- Implement `free_windows(slots, day_start, day_end)` over merged bookings.
- Load holidays from an iCalendar (`.ics`) file into `BusinessCalendar`.
- Replace `TimeError::InvalidInput(String)` with structured variants per parser
  and compare how the error messages read in logs.
