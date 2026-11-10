//! Part 2: reused buffers and exact allocation budgets for report rendering.
mod common;

use common::count_allocations;
use ex19_performance::report::{Row, render_report, write_row};

/// The documented row format: `{name:<16} {count:>10} {total_micros:>20}\n`.
fn expected_row(name: &str, count: u32, total_micros: u64) -> String {
    format!("{name:<16} {count:>10} {total_micros:>20}\n")
}

#[test]
fn part2_write_row_renders_fixed_width_columns() {
    let mut out = String::new();
    write_row(
        &mut out,
        &Row {
            name: "api",
            count: 7,
            total_micros: 1500,
        },
    )
    .unwrap();
    assert_eq!(out, expected_row("api", 7, 1500));
    assert_eq!(out.len(), 16 + 1 + 10 + 1 + 20 + 1, "fixed-width row length");
}

#[test]
fn part2_write_row_appends_without_clearing() {
    let mut out = String::from("header\n");
    let a = Row {
        name: "db",
        count: 1,
        total_micros: 2,
    };
    let b = Row {
        name: "cache",
        count: 3,
        total_micros: 4,
    };
    write_row(&mut out, &a).unwrap();
    write_row(&mut out, &b).unwrap();
    assert_eq!(
        out,
        format!("header\n{}{}", expected_row("db", 1, 2), expected_row("cache", 3, 4))
    );
}

#[test]
fn part2_long_names_are_not_truncated() {
    let name = "a-very-long-service-name-that-exceeds-sixteen";
    let mut out = String::new();
    write_row(
        &mut out,
        &Row {
            name,
            count: 5,
            total_micros: 9,
        },
    )
    .unwrap();
    assert_eq!(out, expected_row(name, 5, 9), "names longer than 16 must not be cut");
}

#[test]
fn part2_write_row_reuses_a_buffer_without_allocating() {
    let rows: Vec<Row<'_>> = (0..100)
        .map(|i| Row {
            name: "checkout",
            count: i,
            total_micros: u64::from(i) * 1000,
        })
        .collect();
    let mut buf = String::with_capacity(64); // one row is 49 bytes
    let ((), allocs) = count_allocations(|| {
        for row in &rows {
            buf.clear();
            write_row(&mut buf, row).unwrap();
        }
    });
    assert_eq!(allocs, 0, "a pre-sized, reused buffer must not allocate");
    assert_eq!(buf, expected_row("checkout", 99, 99_000), "last row wins after clear");
}

#[test]
fn part2_render_report_allocates_exactly_once() {
    let rows = [
        Row {
            name: "api",
            count: 10,
            total_micros: 100,
        },
        Row {
            name: "worker-pool",
            count: 2_000,
            total_micros: 3_000_000,
        },
        Row {
            name: "db",
            count: 3,
            total_micros: 4,
        },
    ];
    let (out, allocs) = count_allocations(|| render_report(&rows));
    assert_eq!(allocs, 1, "a non-empty report is one pre-sized allocation");
    let expected: String = rows
        .iter()
        .map(|r| expected_row(r.name, r.count, r.total_micros))
        .collect();
    assert_eq!(out, expected);
}

#[test]
fn part2_render_report_empty_input_allocates_nothing() {
    let (out, allocs) = count_allocations(|| render_report(&[]));
    assert_eq!(out, "", "no rows, no output");
    assert_eq!(allocs, 0, "an empty report must not allocate");
}

#[test]
fn part2_capacity_bound_holds_at_maximum_widths() {
    let name = "x".repeat(40);
    let rows = [Row {
        name: &name,
        count: u32::MAX,
        total_micros: u64::MAX,
    }];
    let (out, allocs) = count_allocations(|| render_report(&rows));
    assert_eq!(
        allocs, 1,
        "the per-row bound must cover u32::MAX and u64::MAX without reallocating"
    );
    assert_eq!(out, expected_row(&name, u32::MAX, u64::MAX));
}
