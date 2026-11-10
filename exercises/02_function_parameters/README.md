# 02 — Function parameters

## Scenario

You are writing `reqkit`, a small HTTP toolkit used by a service that talks to
several internal APIs. Every public function is a choice about who owns data,
who may mutate it, and what a caller is allowed to pass. The six parts build
the pieces a real client needs: a header model, a request builder, safe URL
encoding, a retrying transport behind a trait, a typed SQL query builder for
the reporting service, and a PATCH endpoint that distinguishes "absent" from
"null".

The crate is `reqkit`. Everything is public from the crate root.

## Part 1 — Headers: borrowed vs owned, `AsRef` and `Into`

```rust
pub fn normalize_header_name(name: &str) -> Result<Cow<'_, str>, HeaderError>;

pub struct Header;
impl Header {
    pub fn new(name: impl AsRef<str>, value: impl Into<String>) -> Result<Self, HeaderError>;
    pub fn name(&self) -> &str;
    pub fn value(&self) -> &str;
    pub fn into_parts(self) -> (String, String);
}
```

- `normalize_header_name` trims ASCII whitespace, validates the RFC 9110 token
  set, and lowercases. It returns `Cow::Borrowed` when the trimmed name was
  already lowercase, and `Cow::Owned` only when lowercasing changed it.
- Empty after trimming: `HeaderError::EmptyName`. A non-token character
  (space, `:`, non-ASCII): `HeaderError::InvalidNameChar(char)`.
- `Header::new` takes `AsRef<str>` for the name (borrowed input is fine) and
  `Into<String>` for the value (owned input is moved, not copied).
- Values are stored verbatim. HTAB is allowed; other ASCII control characters
  (including DEL) are `HeaderError::InvalidValue`.

## Part 2 — Request builder: `&mut self` with deferred errors

```rust
pub enum Method { Get, Post, Put, Patch, Delete, Head, Options }
impl Method {
    pub fn as_str(self) -> &'static str;
}
impl Display for Method;  // canonical upper-case token
impl FromStr for Method;  // case-insensitive; Err = RequestError::InvalidMethod

pub enum RequestError {
    InvalidMethod(String),
    InvalidUrl(String),
    InvalidHeader(HeaderError),   // #[from], transparent
    DuplicateHeader(String),      // carries the normalized name
    InvalidTimeout,
}

pub struct RequestBuilder;
impl RequestBuilder {
    pub fn new(method: impl AsRef<str>, url: impl Into<String>) -> Self;
    pub fn header(&mut self, name: impl AsRef<str>, value: impl Into<String>) -> &mut Self;
    pub fn query(&mut self, key: impl AsRef<str>, value: impl AsRef<str>) -> &mut Self;
    pub fn timeout(&mut self, timeout: Duration) -> &mut Self;
    pub fn build(&self) -> Result<Request, RequestError>;
}

pub struct Request;
impl Request {
    pub fn method(&self) -> Method;
    pub fn url(&self) -> &str;
    pub fn headers(&self) -> &[Header];
    pub fn header(&self, name: &str) -> Option<&str>;
    pub fn query(&self) -> &[(String, String)];
    pub fn timeout(&self) -> Duration;
    pub fn full_url(&self) -> String;
}
```

- The default timeout is 10 seconds.
- Setters never fail. They record the first header problem (invalid name or
  value, or a duplicate). `build` reports, in order: the method, then the URL
  (must start with `http://` or `https://`, case-insensitive), then the
  recorded header problem, then a zero timeout.
- `build(&self)` borrows the builder, so it can be called again after more
  changes. An earlier `Request` is not affected by later changes.
- Header lookup on `Request` is case-insensitive. Headers keep insertion order.

## Part 3 — Encoding and variadic-style arguments

```rust
pub fn percent_encode(input: &str) -> Cow<'_, str>;
pub fn join_path(base: &str, segments: impl IntoIterator<Item = impl AsRef<str>>) -> String;
pub fn encode_query<I, K, V>(pairs: I) -> String
where I: IntoIterator<Item = (K, V)>, K: AsRef<str>, V: AsRef<str>;
```

- `percent_encode` keeps RFC 3986 unreserved bytes (`A-Z a-z 0-9 - . _ ~`) and
  borrows when nothing needs escaping. Hex is upper-case, and UTF-8 is encoded
  byte by byte.
- `join_path` trims trailing `/`s from `base`, skips empty segments, and
  encodes each segment. A `/` inside a segment becomes `%2F`.
- `encode_query` renders `k=v&k=v` in iteration order, with repeated keys kept.
- `Request::full_url` appends the encoded query. It uses `?` unless the URL
  already contains `?`, in which case it uses `&`.

`encode_query` is generic over the pair type, not `impl Trait`, because it has
to name both halves of the tuple.

## Part 4 — Dependency injection: a trait, a closure predicate, and `dyn`

```rust
pub struct Response { pub status: u16, pub body: Vec<u8> }

pub enum TransportError { Timeout, Reset, Protocol(String) }

pub trait Transport {
    fn send(&mut self, request: &Request) -> Result<Response, TransportError>;
}

pub fn send_with_retry<T: Transport + ?Sized>(
    transport: &mut T,
    request: &Request,
    max_attempts: NonZeroU32,
    retry_if: impl FnMut(&TransportError) -> bool,
) -> Result<Response, TransportError>;
```

- At most `max_attempts` calls are made. Success returns immediately.
- After a failure, `retry_if` is consulted only when attempts remain. It is
  never called after the final attempt, and it is not called at all when
  `max_attempts` is 1.
- When it gives up, the last error is returned unchanged.
- `transport` may be a concrete type or `&mut dyn Transport`.

## Part 5 — Typestate SQL builder (consuming `self`)

```rust
pub struct NoTable;
pub struct HasTable;               // carries the table name privately
pub enum Value { Null, Bool(bool), Int(i64), Text(String) }
// From: bool, i64, &str, String, Option<T: Into<Value>>

pub enum QueryError {
    InvalidIdentifier(String),
    InvalidOrderTerm(String),
    LimitTooLarge(u64),
}

pub struct Select<State>;
pub fn select<I, S>(columns: I) -> Select<NoTable>
where I: IntoIterator<Item = S>, S: AsRef<str>;

impl Select<NoTable> {
    pub fn from_table(self, table: impl AsRef<str>) -> Select<HasTable>;
}
impl Select<HasTable> {
    pub fn filter_eq(self, column: impl AsRef<str>, value: impl Into<Value>) -> Self;
    pub fn order_by(self, term: impl AsRef<str>) -> Self;
    pub fn limit(self, n: u64) -> Self;
    pub fn build(self) -> Result<(String, Vec<Value>), QueryError>;
}
```

- `select` with no columns renders `*`. Identifiers are `ident` or
  `ident.ident`, where `ident` is `[A-Za-z_][A-Za-z0-9_]*`. Column, table and
  filter identifiers are validated by `build`.
- Rendering order: `SELECT cols FROM t [WHERE a = ? AND b IS NULL] [ORDER BY x, y DESC] [LIMIT ?]`.
  Filters are joined with `AND` in call order.
- `filter_eq` with a null value renders `IS NULL` and adds no parameter.
- `order_by` terms may start with `-` for `DESC`. Anything else that is not an
  identifier is `InvalidOrderTerm`.
- `limit` is a bound parameter. The last call wins. A value above `i64::MAX`
  is `LimitTooLarge`.
- `build` does not exist on `Select<NoTable>`. Calling it before choosing a
  table is a compile error, not a runtime one.

## Part 6 — PATCH: absent vs `null` vs a value

```rust
pub enum Patch<T> { Missing, Null, Value(T) }
impl<T> Default for Patch<T>;   // Missing, with no T: Default bound
impl<T> Patch<T> {
    pub fn is_missing(&self) -> bool;
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Patch<U>;
    pub fn apply(self, slot: &mut Option<T>);
}

pub struct User { pub email: String, pub nickname: Option<String>, pub bio: Option<String> }

pub enum PatchError { EmailCleared, InvalidEmail(String), InvalidNickname, BioTooLong }

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserPatch {
    pub email: Patch<String>,
    pub nickname: Patch<String>,
    pub bio: Patch<String>,
}
impl UserPatch {
    pub fn apply(self, user: &mut User) -> Result<(), PatchError>;
}
```

- A key that is absent is `Missing`. A key set to `null` is `Null`. Any other
  value is `Value`. Serde must keep these three distinct.
- `Patch::default()` is `Missing` for any `T`. `#[serde(default)]` relies on
  this, and so does struct update syntax (`..Default::default()`).
- `apply` on a `Patch` writes into an `Option` slot: `Missing` leaves it alone,
  `Null` sets `None`, `Value(v)` sets `Some(v)`.
- Email: `null` is `EmailCleared`. Otherwise it is trimmed and lowercased, and
  must have exactly one `@` with non-empty sides and a dot in the domain.
  `InvalidEmail` carries the raw input.
- Nickname: trimmed, then 1 to 32 characters (counted as `char`s). Otherwise
  `InvalidNickname`. `null` clears it.
- Bio: at most 280 characters (counted as `char`s), otherwise `BioTooLong`.
  `null` clears it.
- `apply` validates every present field before mutating anything. A rejected
  patch leaves the user unchanged.

## Concepts exercised

- Ownership in signatures: `&str` vs `String` vs `Cow<'_, str>`, and when to
  allocate.
- `AsRef<str>` / `AsRef<Path>`-style inputs and `Into<String>` for owned sinks.
- Generics vs `impl Trait` in argument position. When a named type parameter is
  needed (`encode_query`) and when it is not.
- `impl IntoIterator<Item = impl AsRef<str>>` as the variadic analogue.
- `&mut self` builders with deferred error reporting, versus consuming
  builders.
- Typestate with `PhantomData`-free marker states, so a missing table is a
  compile error.
- `Default` plus struct update syntax for optional arguments.
- Trait-based dependency injection with generic `?Sized` parameters and
  `&mut dyn Transport`.
- Closures as parameters (`FnMut` predicate that is never called when it cannot
  change the outcome).
- serde `deserialize_with` to keep "absent", "null" and "value" distinct.

## Run

```sh
cargo test -p ex02_function_parameters
cargo test -p ex02_function_parameters part2
cargo solutions -p ex02_function_parameters
```

## Stretch (not tested)

- Add `Request::with_header(self, ...)` consuming variant and compare its
  ergonomics with the `&mut self` builder. Discuss when each is correct.
- Make `Header` values accept `Cow<'static, str>` so that static literals avoid
  allocation. Measure with a small benchmark or by counting allocations.
- Replace `Select`'s typestate with a sealed trait and explain the trade-off
  against `PhantomData`.
- Add a `retry_after` predicate that receives `&TransportError` and returns a
  `Duration`. Keep `send_with_retry`'s signature generic over the delay policy.
- Implement `serde::Serialize` for `Patch<T>` and check round-trip behaviour.
