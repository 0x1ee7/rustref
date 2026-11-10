//! Part 7: a command registry of boxed handlers keyed by static names.

use ex04_closures_and_functions::commands::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn sum_handler(args: &[String]) -> Result<String, CommandError> {
    if args.is_empty() {
        return Err(CommandError::Usage("sum <n>...".to_string()));
    }
    let mut total: i64 = 0;
    for raw in args {
        let n: i64 = raw
            .parse()
            .map_err(|_| CommandError::Failed(format!("not a number: {raw}")))?;
        total = total
            .checked_add(n)
            .ok_or_else(|| CommandError::Failed("sum overflowed i64".to_string()))?;
    }
    Ok(total.to_string())
}

#[test]
fn part7_dispatch_runs_the_registered_handler() {
    let mut registry = CommandRegistry::new();
    registry.register("sum", "Add integers.", sum_handler).unwrap();
    assert_eq!(registry.dispatch("sum", &args(&["2", "3", "5"])).unwrap(), "10");
}

#[test]
fn part7_duplicate_and_invalid_names_are_rejected() {
    let mut registry = CommandRegistry::new();
    registry.register("sum", "", sum_handler).unwrap();
    assert_eq!(
        registry.register("sum", "again", sum_handler),
        Err(RegistryError::Duplicate("sum"))
    );
    assert_eq!(registry.register("", "", sum_handler), Err(RegistryError::InvalidName("")));
    assert_eq!(
        registry.register("two words", "", sum_handler),
        Err(RegistryError::InvalidName("two words"))
    );
}

#[test]
fn part7_unknown_command_is_reported_by_name() {
    let registry = CommandRegistry::new();
    match registry.dispatch("nope", &[]) {
        Err(DispatchError::UnknownCommand(name)) => assert_eq!(name, "nope"),
        other => panic!("expected UnknownCommand, got {other:?}"),
    }
}

#[test]
fn part7_handler_errors_surface_as_dispatch_command_errors() {
    let mut registry = CommandRegistry::new();
    registry.register("sum", "Add integers.", sum_handler).unwrap();
    assert!(
        matches!(
            registry.dispatch("sum", &[]),
            Err(DispatchError::Command(CommandError::Usage(_)))
        ),
        "empty args is a usage error"
    );
    assert!(
        matches!(
            registry.dispatch("sum", &args(&["x"])),
            Err(DispatchError::Command(CommandError::Failed(ref msg))) if msg.contains('x')
        ),
        "a parse failure is a Failed error that names the bad input"
    );
}

#[test]
fn part7_names_are_sorted_and_help_is_looked_up() {
    let mut registry = CommandRegistry::new();
    registry.register("zeta", "Last.", sum_handler).unwrap();
    registry.register("alpha", "First.", sum_handler).unwrap();
    registry.register("mid", "", sum_handler).unwrap();
    assert_eq!(registry.names(), vec!["alpha", "mid", "zeta"]);
    assert_eq!(registry.help("alpha"), Some("First."));
    assert_eq!(registry.help("mid"), Some(""), "empty help is still registered help");
    assert_eq!(registry.help("missing"), None);
}

#[test]
fn part7_registries_are_independent() {
    let mut first = CommandRegistry::new();
    let second = CommandRegistry::new();
    first.register("sum", "", sum_handler).unwrap();
    assert!(second.names().is_empty(), "a registration must not leak into another registry");
    assert!(second.dispatch("sum", &[]).is_err());
}

#[test]
fn part7_handlers_can_capture_shared_state() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&calls);
    let mut registry = CommandRegistry::new();
    registry
        .register("ping", "Count pings.", move |_args| {
            let n = counter.fetch_add(1, Ordering::SeqCst) + 1;
            Ok(format!("pong {n}"))
        })
        .unwrap();
    assert_eq!(registry.dispatch("ping", &[]).unwrap(), "pong 1");
    assert_eq!(registry.dispatch("ping", &[]).unwrap(), "pong 2");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn part7_registry_can_be_dispatched_from_many_threads() {
    let mut registry = CommandRegistry::new();
    registry.register("sum", "", sum_handler).unwrap();
    let registry = Arc::new(registry);
    let workers: Vec<_> = (0..4)
        .map(|i| {
            let registry = Arc::clone(&registry);
            thread::spawn(move || {
                let arg = i.to_string();
                registry.dispatch("sum", &args(&[arg.as_str(), "1"])).unwrap()
            })
        })
        .collect();
    let results: Vec<String> = workers
        .into_iter()
        .map(|w| w.join().expect("worker panicked"))
        .collect();
    assert_eq!(results, vec!["1", "2", "3", "4"]);
}
