# 20 — Declarative macros

## Scenario

You maintain a small internal service toolkit. Four recurring chores keep showing up in review:
building lookup tables in tests and config, declaring "enum with a text form" types for wire
protocols, wrapping primitive IDs so they can't be mixed up, and writing validation that returns
early with a typed error. On top of those, the team wants a declarative route table validated at
startup, and a test assertion that can pattern-match and hand back what it matched.

Each of these is a `macro_rules!` macro. Your job is to write them so that they are
hygienic, produce no surprises at the call site, and reject bad input at compile time where
possible and at construction time otherwise.

## Parts

### Part 1 — Collections: `count!`, `hashmap!`, `btreeset!`

- `count!($($e:expr),* $(,)?)` evaluates to a `usize` count of its arguments. The arguments are
  **never evaluated**. `count!()` is `0`.
- `hashmap! { k => v, ... }` builds a `HashMap`. It pre-sizes with `with_capacity` using `count!`.
  Each key and value is evaluated once, in source order (key before value). A duplicate key keeps
  the last value.
- `btreeset![a, b, ...]` builds a sorted, de-duplicated `BTreeSet`.
- Trailing commas are optional everywhere; empty forms are valid.

### Part 2 — `string_enum!`: an enum with a text form

`string_enum! { $(#[$meta])* $vis enum Name { $(#[$vmeta])* Variant = "text", ... } }` generates:

- The enum, with `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]` plus the caller's attributes.
- `pub const ALL: &'static [Self]` in declaration order.
- `pub const fn as_str(&self) -> &'static str`.
- `Display` (writes `as_str`) and `FromStr` (exact, case-sensitive, `Err = UnknownVariant`).

`UnknownVariant { type_name, input }` displays as ``unknown <type_name> `<input>` `` and implements
`Error`. Generated code uses absolute `::core` paths, so a caller's own `Result` alias can't break it.

### Part 3 — `newtype!`: opt-in wrappers

`newtype!( $(#[$meta])* $vis Name(Inner) $(: Trait, ...)? );` generates `struct Name(Inner);` with
a private field, and always derives `Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash`. It adds
`const fn new`, `into_inner(self)`, and `From<Inner>`. Traits listed after `:` are opt-in and
delegate to `Inner`:

- `Copy` (a marker impl),
- `Display` (forwards the formatter, so width and fill work),
- `FromStr` (with `Err = <Inner as FromStr>::Err`).

Any other name is a `compile_error!`. The trait list is parsed by a recursive helper arm.

### Part 4 — `bail!` and `ensure!`

- `bail!(err)` returns `Err(err.into())` from the enclosing function. Conversion goes through
  `Into`, so `?`-style `From` conversions apply.
- `ensure!(cond, err)` returns early with `err` when `cond` is false. `cond` is evaluated once and
  `err` only on failure. The negation is built so that `cond`'s precedence is preserved.

### Part 5 — `routes!`: a validated route table

`routes! { METHOD "pattern" => handler, ... }` expands to `Result<Router<H>, RouteError>`. The
method is one of `GET`, `POST`, `PUT`, `PATCH`, `DELETE`; anything else is a compile error.

Patterns are `/`-separated. A `:name` segment captures exactly one non-empty segment. Validation
rejects a missing leading `/`, empty segments, empty parameter names, duplicate parameter names
within one pattern, and duplicate method+shape pairs (`/users/:id` and `/users/:name` conflict).

`Router::lookup(method, path)` returns `Found(Match)`, `MethodNotAllowed(sorted methods)`, or
`NotFound`. The most specific route wins: at the first differing segment a literal beats a
`:param`. If the literal branch cannot complete, the lookup falls back to the parameter branch.

### Part 6 — `assert_matches!`: pattern assertions that return bindings

- `assert_matches!(v, pat)` and `assert_matches!(v, pat if guard)` assert and return `()`.
- `assert_matches!(v, pat => out)` returns `out`, with the pattern's bindings in scope, so you can
  pull a value out of a test result.
- `assert_matches!(v, pat, "fmt", args...)` prefixes the panic message with a custom text.

`v` is evaluated exactly once. Failures panic with the value's `Debug` and the pattern text.
Or-patterns and guards work.

## API surface

```text
collections:  count!, hashmap!, btreeset!
enums:        string_enum!, UnknownVariant { type_name: &'static str, input: String }
newtype:      newtype!
control:      bail!, ensure!
routing:      routes!, Method, Route<H>, RouteError, Router<H>, Lookup, Match
testing:      assert_matches!
```

## Concepts exercised

- `macro_rules!` matchers: `expr`, `ident`, `ty`, `pat`, `literal`, `vis`, `meta`, `tt`
- Repetitions with separators, optional trailing commas (`$(,)?`), nested repetition
- Tt-munchers and recursive helper arms (`@`-prefixed internal rules)
- `$crate` paths, `#[macro_export]`, and hygiene for generated code (`::core` paths)
- Generating items: enums, structs, impls, `const` items, `compile_error!`
- Evaluation order and single evaluation of macro arguments
- Early-return macros (`return` inside a macro, tail position, `let else`)
- Panic-based assertions and `catch_unwind` in tests

## Run

```sh
cargo test -p ex20_macros
cargo test -p ex20_macros part2
cargo solutions -p ex20_macros
```

## Stretch

- Make `routes!` also generate a `const` method table, so the lint runs at compile time.
- Add a `json_value!` macro for `serde_json::Value` literals, with interpolation of `expr`s.
- Give `string_enum!` a `#[serde(rename)]`-style attribute passthrough, and see where
  `$meta` stops being enough.
- Write a `#[derive]`-free `builder!` macro that generates setters from struct fields, and
  compare its error messages to a proc macro's.
