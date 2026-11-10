//! Part 2: `string_enum!`, an enum with its text form.

use std::collections::HashSet;
use std::error::Error;

use ex20_macros::{string_enum, UnknownVariant};

string_enum! {
    /// Lifecycle of a background job.
    pub enum JobState {
        /// Waiting for a worker.
        Queued = "queued",
        Running = "running",
        Done = "done",
    }
}

mod shadowed {
    // A caller's own `Result` alias must not break the generated `FromStr`.
    use ex20_macros::string_enum;

    #[allow(dead_code)]
    type Result = u8;

    string_enum! {
        pub enum Mode {
            Fast = "fast",
            Slow = "slow",
        }
    }
}

#[test]
fn part2_all_lists_variants_in_declaration_order() {
    let all: &'static [JobState] = JobState::ALL;
    assert_eq!(
        all.to_vec(),
        vec![JobState::Queued, JobState::Running, JobState::Done],
        "ALL should list every variant in declaration order"
    );
}

#[test]
fn part2_as_str_and_display_use_the_literal() {
    assert_eq!(JobState::Queued.as_str(), "queued");
    assert_eq!(JobState::Running.as_str(), "running");
    assert_eq!(JobState::Done.to_string(), "done");
}

#[test]
fn part2_from_str_round_trips_every_variant() {
    assert_eq!(JobState::ALL.len(), 3, "ALL must list all three variants");
    for &state in JobState::ALL {
        let text = state.to_string();
        assert_eq!(
            text.parse::<JobState>(),
            Ok(state),
            "{text:?} should parse back to {state:?}"
        );
    }
}

#[test]
fn part2_from_str_is_case_sensitive() {
    assert!(
        "Queued".parse::<JobState>().is_err(),
        "parsing must be exact and case-sensitive"
    );
}

#[test]
fn part2_from_str_reports_type_and_input() {
    let err = "paused".parse::<JobState>().unwrap_err();
    assert_eq!(
        err,
        UnknownVariant {
            type_name: "JobState",
            input: "paused".to_string(),
        }
    );
}

#[test]
fn part2_from_str_rejects_empty_input() {
    let err = "".parse::<JobState>().unwrap_err();
    assert_eq!(err.input, "", "the empty string is reported verbatim");
}

#[test]
fn part2_unknown_variant_display_and_error_trait() {
    let err = "paused".parse::<JobState>().unwrap_err();
    assert_eq!(err.to_string(), "unknown JobState `paused`");
    let as_error: &dyn Error = &err;
    assert!(as_error.source().is_none());
}

#[test]
fn part2_derives_copy_hash_eq_and_debug() {
    let state = JobState::Queued;
    assert_eq!(JobState::ALL.len(), 3, "JobState has three variants");
    let copy = state;
    assert_eq!(state, copy, "the enum is Copy and compares by variant");

    let set: HashSet<JobState> = [JobState::Queued, JobState::Queued, JobState::Done]
        .into_iter()
        .collect();
    assert_eq!(set.len(), 2, "the enum is Hash + Eq");

    assert_eq!(format!("{:?}", JobState::Done), "Done");
}

#[test]
fn part2_generated_code_ignores_shadowed_result_alias() {
    use shadowed::Mode;
    assert_eq!("slow".parse::<Mode>(), Ok(Mode::Slow));
    assert!("turbo".parse::<Mode>().is_err());
    assert_eq!(Mode::ALL.len(), 2);
}
