# 07 — Structs, traits and value types

A small commerce and banking core: money, products, accounts, a shopping cart,
employee records, notification channels and semantic versions. Each Part builds
one piece of production-shaped Rust: constructors that enforce invariants,
newtypes, trait-based shared behaviour, operator overloading and manual
equality/ordering rules.

The spec is this README plus `tests/structs_and_traits.rs`. Implement everything
in `src/` (one module per Part; keep the public names and signatures).

Conventions:

- Invalid states are unrepresentable where the type system can do it
  (`u32` for quantities, a `Sku` newtype that is validated once).
- Fallible operations return `Result` with the error enums named below.
  Operator impls (`+`, `-`, `*`, unary `-`, `Index`) panic on failure and
  document it; the `checked_*`/`try_*` methods are the fallible face.
- Rust has no class attributes or default arguments: where Python used them,
  use associated constants, builder-style `with_*` methods, or `Default`.

---

## Part 1 — `Money`: exact minor units, checked arithmetic, operators

`Currency` is a validated three-letter code. `Money` is an `i64` count of minor
units plus a `Currency`; no floats anywhere.

```rust
pub struct Currency; // Copy, Eq, Hash, Ord, Display ("USD"), Debug ("Currency(\"USD\")")
impl TryFrom<&str> for Currency;   // exactly three ASCII uppercase letters
impl FromStr for Currency;         // same rule
impl Currency {
    pub fn minor_digits(self) -> u32; // JPY, KRW: 0; KWD: 3; else 2
    pub fn as_str(&self) -> &str;
}

pub struct Money;  // Copy, Eq, Hash, Debug
impl Money {
    pub fn new(minor: i64, currency: Currency) -> Money;
    pub fn zero(currency: Currency) -> Money;
    pub fn minor(self) -> i64;
    pub fn currency(self) -> Currency;
    pub fn checked_add(self, rhs: Money) -> Result<Money, MoneyError>;
    pub fn checked_sub(self, rhs: Money) -> Result<Money, MoneyError>;
    pub fn checked_mul(self, factor: i64) -> Result<Money, MoneyError>;
    pub fn checked_cmp(self, other: Money) -> Result<Ordering, MoneyError>;
    pub fn scaled(self, numerator: i64, denominator: i64) -> Result<Money, MoneyError>;
    pub fn allocate(self, parts: usize) -> Result<Vec<Money>, MoneyError>;
    pub fn try_sum<I: IntoIterator<Item = Money>>(currency: Currency, items: I)
        -> Result<Money, MoneyError>;
}
impl FromStr for Money; // "10.50 USD", "-0.05 USD", "100 JPY"
impl Display for Money; // "10.50 USD", "-0.05 USD", "100 JPY"
impl Add, Sub for Money;     // panic: "Money addition failed" / "Money subtraction failed"
impl Mul<i64> for Money;     // and impl Mul<Money> for i64; panic: "Money multiplication overflowed"
impl Neg for Money;          // panic: "Money negation overflowed"
impl PartialOrd for Money;   // None across currencies; no Ord
```

Behaviour:

- Parsing: `"<amount> <CODE>"` with exactly two whitespace-separated parts.
  The amount is an optional `-`, digits, and an optional `.` followed by digits.
  Trailing zeros beyond the minor unit are fine (`"1.500 USD"` is `1.50`).
  Any other fraction digits → `TooPrecise` (`"1.005 USD"`, `"100.5 JPY"`).
  `"1,000.00 USD"`, `"10. USD"`, `".5 USD"`, `"1e3 USD"`, `"10.00"` → `InvalidAmount`.
  A bad code (`"usd"`, `"US"`) → `InvalidCurrency`. A whole part that does not fit `i64` → `Overflow`.
- Display: plain notation, minor digits as per the currency; negative values keep
  the sign even when the major part is zero (`-0.05 USD`).
- `checked_add`/`checked_sub`/`checked_cmp` reject currency mismatch with
  `CurrencyMismatch { left, right }` (left is `self`). Integer overflow → `Overflow`.
- `Add`/`Sub` panic on mismatch or overflow. `checked_*` is the fallible path.
- `PartialOrd` returns `None` across currencies (equal currency compares by amount).
  `checked_cmp` gives the error instead. `PartialEq`/`Hash` use both fields.
- `scaled(n, d)` computes `minor * n / d` rounded **half to even** (banker's
  rounding). `d == 0` → `ZeroDenominator`. Overflow → `Overflow`.
- `allocate(parts)` splits into `parts` shares whose minor units sum exactly to
  `self`; the first `remainder` shares get one extra unit. Negative amounts split
  by magnitude and keep the sign on each share. `parts == 0` → `ZeroParts`.
- `try_sum(currency, items)` folds from `Money::zero(currency)`. A foreign currency
  item → `CurrencyMismatch`. An empty iterator gives zero.

Why no `Sum` impl: a sum over zero items has no currency to report, so the
currency must be passed explicitly. `try_sum` is the honest equivalent.

## Part 2 — `Sku` newtype and `Product`: validated constructors, setters, builders

```rust
pub struct Sku;  // validated; Clone, Eq, Hash, Ord, Debug, Display, AsRef<str>
impl Sku {
    pub const MIN_LEN: usize = 4;
    pub const MAX_LEN: usize = 16;
    pub fn new(raw: &str) -> Result<Sku, ProductError>;   // 4–16 chars of A-Z 0-9 '-'
    pub fn as_str(&self) -> &str;
}
impl FromStr for Sku;   // same rule as `new`

pub struct Product;   // Clone, Debug, PartialEq
impl Product {
    pub const MAX_NAME_CHARS: usize = 100;
    pub fn new(sku: Sku, name: &str, price: Money) -> Result<Product, ProductError>;
    pub fn with_quantity(self, quantity: u32) -> Product;
    pub fn sku(&self) -> &Sku;
    pub fn name(&self) -> &str;
    pub fn price(&self) -> Money;
    pub fn quantity(&self) -> u32;
    pub fn in_stock(&self) -> bool;
    pub fn set_name(&mut self, name: &str) -> Result<(), ProductError>;
    pub fn set_price(&mut self, price: Money);
    pub fn set_quantity(&mut self, quantity: u32);
    pub fn restock(&mut self, count: u32) -> Result<(), ProductError>;
}
```

- `Sku::new` is strict: no trimming; length counted in bytes (ASCII only).
- `name` is stored trimmed. Blank after trimming → `BlankName`. More than 100
  characters (not bytes) → `NameTooLong { max: 100 }`.
- `Product::new` starts with quantity 0. `with_quantity` is a builder for seeding stock.
- `restock(0)` → `ZeroRestock`. Overflow of `u32` → `QuantityOverflow`, and stock is left unchanged.
- A failed `set_name` leaves the old name in place.
- Quantity is `u32`, so a negative quantity cannot exist; no runtime check is needed.

## Part 3 — `Account` trait: shared default methods, composition, overrides

```rust
pub struct Ledger;   // owner + balance; composed into accounts, not inherited
impl Ledger {
    pub fn new(owner: &str, opening: Money) -> Result<Ledger, AccountError>;
    pub fn owner(&self) -> &str;
    pub fn balance(&self) -> Money;
}

pub trait Account {
    fn ledger(&self) -> &Ledger;
    fn ledger_mut(&mut self) -> &mut Ledger;
    fn owner(&self) -> &str;                                  // default: from ledger
    fn balance(&self) -> Money;                               // default: from ledger
    fn available_funds(&self) -> Result<Money, AccountError>; // default: balance
    fn deposit(&mut self, amount: Money) -> Result<Money, AccountError>;   // default
    fn withdraw(&mut self, amount: Money) -> Result<Money, AccountError>;  // default
}

pub struct CheckingAccount;  // impl Account
impl CheckingAccount {
    pub fn open(owner: &str, opening: Money) -> Result<CheckingAccount, AccountError>;
    pub fn with_overdraft(self, limit: Money) -> Result<CheckingAccount, AccountError>;
    pub fn overdraft_limit(&self) -> Money;   // zero in the balance's currency by default
}

pub struct SavingsAccount;   // impl Account, overrides withdraw
impl SavingsAccount {
    pub fn open(owner: &str, opening: Money) -> Result<SavingsAccount, AccountError>; // limit 3
    pub fn with_max_withdrawals(self, max: u32) -> SavingsAccount;
    pub fn max_withdrawals(&self) -> u32;
    pub fn withdrawals_this_period(&self) -> u32;
    pub fn reset_period(&mut self);
}
```

- `Ledger::new` trims the owner; blank → `BlankOwner`.
- `deposit`/`withdraw` require a strictly positive amount → `NonPositiveAmount`.
  Currency mismatch → `Money(MoneyError::CurrencyMismatch { .. })`.
- `withdraw` succeeds iff `amount <= available_funds()`. Otherwise
  `InsufficientFunds { requested, available }` and the balance is unchanged.
  Withdrawing exactly the available amount is allowed.
- `CheckingAccount::available_funds` = balance + overdraft. The balance may go
  negative down to `-overdraft`. `with_overdraft` rejects a negative limit
  (`NegativeOverdraft`) and a foreign currency.
- `SavingsAccount::withdraw` checks the limit **first**: if
  `withdrawals_this_period >= max` it returns `WithdrawalLimitReached { limit }`
  even when the amount is invalid. Otherwise it applies the normal withdrawal
  rules and counts only successful withdrawals. Deposits never count.
  Write the shared withdrawal rule once and reuse it; do not duplicate it.

## Part 4 — `ShoppingCart`: collections, `Index`, `IntoIterator`, atomic batches

```rust
pub struct CartLine;  // sku, unit price snapshot, quantity
impl CartLine {
    pub fn sku(&self) -> &Sku;
    pub fn unit_price(&self) -> Money;
    pub fn quantity(&self) -> u32;
    pub fn subtotal(&self) -> Result<Money, MoneyError>;
}

pub struct ShoppingCart;   // Clone, Debug, PartialEq, Eq
impl ShoppingCart {
    pub fn new(currency: Currency) -> ShoppingCart;
    pub fn currency(&self) -> Currency;
    pub fn add(&mut self, product: &Product, quantity: u32) -> Result<(), CartError>;
    pub fn remove(&mut self, sku: &Sku, quantity: u32) -> Result<(), CartError>;
    pub fn quantity_of(&self, sku: &Sku) -> u32;          // 0 when absent
    pub fn contains(&self, sku: &Sku) -> bool;
    pub fn units(&self) -> u64;
    pub fn is_empty(&self) -> bool;
    pub fn total(&self) -> Result<Money, CartError>;
    pub fn try_extend<'a, I>(&mut self, items: I) -> Result<(), CartError>
        where I: IntoIterator<Item = (&'a Product, u32)>;
}
impl Index<&Sku> for ShoppingCart { type Output = u32; }  // panics when absent
impl<'a> IntoIterator for &'a ShoppingCart { type Item = &'a CartLine; ... }
```

- `add`: quantity 0 → `ZeroQuantity`. A product in another currency →
  `CurrencyMismatch { expected, found }`. An existing sku accumulates quantity
  (`QuantityOverflow` on `u32` overflow). The line keeps the price it was first
  added at; an add at a different price → `PriceChanged { sku }`, unchanged cart.
- `remove`: quantity 0 → `ZeroQuantity`; unknown sku → `UnknownSku`; more than
  present → `InsufficientQuantity { sku, requested, available }`. Removing all
  units deletes the line. Failures leave the cart unchanged.
- Lines keep insertion order; iterating `&cart` yields `&CartLine` in that order.
- `Index` panics with a message containing `no line` when the sku is absent.
- `total` sums `subtotal`s from `Money::zero(currency)`; empty cart → zero.
- `try_extend` is all-or-nothing: the batch is applied to a copy and committed only
  if every item succeeds.
- Why not `Extend`: `Extend::extend` cannot return an error, and adding products
  can fail. `try_extend` is the fallible equivalent.

## Part 5 — `Employee`: validated newtypes, `TryFrom`, `FromStr`, serde

```rust
pub struct Email;       // normalised, validated; Display, AsRef<str>
impl Email { pub fn parse(raw: &str) -> Result<Email, EmployeeError>; pub fn as_str(&self) -> &str; }

pub struct EmployeeRecord;  // raw, unvalidated: Deserialize
// fields: employee_id: u64, name: String, email: String, department: String,
//         salary: String (decimal amount), currency: String, skills: Vec<String> (default empty)

pub struct Employee;  // Clone, Debug, PartialEq, Eq, Deserialize (via EmployeeRecord)
impl TryFrom<EmployeeRecord> for Employee { type Error = EmployeeError; }
impl FromStr for Employee { type Err = EmployeeError; }  // one CSV row
impl Employee {
    pub fn employee_id(&self) -> u64;
    pub fn name(&self) -> &str;
    pub fn email(&self) -> &Email;
    pub fn department(&self) -> &str;
    pub fn salary(&self) -> Money;
    pub fn skills(&self) -> &[String];
}
```

- Validation runs on every path (`TryFrom`, `FromStr`, serde deserialization):
  - `employee_id` must be > 0 (`Invalid { field: "employee_id", .. }`).
  - `name` and `department` are trimmed. Blank → `Invalid`.
  - `email` is `Email::parse`.
  - `salary` is `"<salary> <currency>"` parsed as `Money`; negative → `Invalid { field: "salary", .. }`.
  - `skills` are trimmed and lowercased. Empty entries are dropped and duplicates
    are dropped with the first occurrence kept.
- `Email::parse`: trimmed, lowercased, no whitespace anywhere, exactly one `@`,
  non-empty local part and domain.
- `FromStr`: one CSV row with 7 columns, `employee_id,name,email,department,salary,currency,skills`.
  Use the `csv` crate so quoted fields like `"Lovelace, Ada"` work. Skills are
  `;`-separated, and the column may be empty. Any other column count → `ColumnCount(n)`.
- Serde: `Employee` deserializes **through** `EmployeeRecord`
  (`#[serde(try_from = …)]`), so JSON cannot bypass validation.
- Equality is field-by-field (derived).

## Part 6 — `Notifier` trait objects, transports, dispatch

```rust
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
pub const SMS_MAX_CHARS: usize = 160;

pub trait Notifier {
    fn channel_name(&self) -> &str;
    fn send(&self, recipient: &str, message: &str) -> Result<(), NotificationError>;
    fn describe(&self) -> String;   // default: "<channel> notifier"
}

pub struct EmailNotifier<F>;     // F: Fn(&str, &str) -> Result<(), BoxError>; channel "email"
pub struct SmsNotifier<F>;       // F: Fn(&str, &str) -> Result<(), BoxError>; channel "sms"
pub struct WebhookNotifier<F>;   // F: Fn(&str, &[u8]) -> Result<u16, BoxError>; channel "webhook"
impl WebhookNotifier<F> { pub fn new(endpoint: &str, post: F) -> Result<Self, NotificationError>; }

pub struct NotificationDispatcher;  // owns Vec<Box<dyn Notifier>>
impl NotificationDispatcher {
    pub fn new(notifiers: impl IntoIterator<Item = Box<dyn Notifier>>)
        -> Result<NotificationDispatcher, NotificationError>;
    pub fn channels(&self) -> impl Iterator<Item = &str> + '_;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn dispatch<'a>(&self, message: &str, recipients: &[(&'a str, &str)])
        -> Result<Vec<(&'a str, Result<(), NotificationError>)>, NotificationError>;
}
```

- Messages: a blank (whitespace-only) message → `BlankMessage` on every channel.
- Email recipient: `local@domain`, no whitespace, exactly one `@`, both sides
  non-empty. Otherwise `InvalidRecipient { channel: "email", .. }`. The transport
  is called as `transport(recipient, message)`. Transport errors become
  `Transport(source)`, and `source()` returns the transport's error.
- SMS recipient: `+` followed by 7–15 digits, the first of which is not `0`. Otherwise
  `InvalidRecipient { channel: "sms", .. }`. Length is counted in **characters**;
  more than `SMS_MAX_CHARS` → `MessageTooLong { len, max }`.
- Webhook: `new` requires an `https://` endpoint (`InsecureEndpoint` otherwise).
  `send` requires a non-blank recipient (`BlankRecipient`). The body is UTF-8 JSON
  with keys `message` then `recipient` (sorted). `post(endpoint, body)` returns a status code.
  Any status outside 200–299 → `HttpStatus(code)`.
- `NotificationDispatcher::new` rejects duplicate channel names → `DuplicateChannel`.
  `channels()` is in construction order.
- `dispatch`: every requested channel must be known. An unknown one returns
  `UnknownChannel` **before anything is sent**. Each send is attempted in request
  order. Failures are recorded in the result and the loop continues. Result
  order follows the request.

## Part 7 — `Version`: SemVer precedence, manual `Eq`/`Hash`/`Ord`, typed bumps

```rust
pub enum PreIdentifier { Numeric(u64), Alphanumeric(String) }  // Ord: numeric < alphanumeric
pub enum Part { Major, Minor, Patch }
pub struct Version;   // Clone, Debug; manual Eq/Hash/Ord/PartialOrd
impl Version {
    pub const fn new(major: u64, minor: u64, patch: u64) -> Version;
    pub fn parse(text: &str) -> Result<Version, VersionError>;   // also FromStr
    pub fn major(&self) -> u64;
    pub fn minor(&self) -> u64;
    pub fn patch(&self) -> u64;
    pub fn prerelease(&self) -> &[PreIdentifier];
    pub fn build(&self) -> &[String];
    pub fn is_prerelease(&self) -> bool;
    pub fn bump(&self, part: Part) -> Result<Version, VersionError>;
}
```

- Grammar: `[v]MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]`. Core numbers have no leading
  zeros. Identifiers are non-empty and use `[0-9A-Za-z-]`. Numeric prerelease
  identifiers have no leading zeros. Numeric build identifiers may. A number that
  does not fit `u64` → `Overflow`. Anything else is `Malformed`, `LeadingZero`,
  `EmptyIdentifier` or `InvalidCharacter`.
- Display is canonical: `1.2.3-rc.1+build.5`. Build metadata is kept in display.
- Precedence: compare core numbers. A release outranks every prerelease with the
  same core. Prerelease identifiers compare left to right; numeric ones numerically,
  alphanumeric ones as ASCII, numeric below alphanumeric, and a shorter list is lower when it is a prefix.
- Build metadata is ignored by `==`, `Ord` and `Hash`. `1.0.0+a == 1.0.0+b`, and
  they hash the same. `Eq`, `Ord` and `Hash` must be consistent, so write them by hand.
- `bump(part)` resets lower parts to zero and clears prerelease and build. `u64` overflow → `Overflow`.

---

## Concepts exercised

- Structs, `impl` blocks, constructors that enforce invariants, `TryFrom`, `FromStr`
- Newtypes (`Currency`, `Sku`, `Email`) and making invalid states unrepresentable
- `Display` vs `Debug`; `thiserror` error enums with `#[from]` and `#[source]`
- Derived vs manual `PartialEq`/`Eq`/`Hash`/`PartialOrd`/`Ord` consistency
- Operator traits (`Add`, `Sub`, `Mul<i64>`, `Mul<Money> for i64`, `Neg`) and the
  panic-vs-`checked_*` split; `Index`; `IntoIterator` for `&T` and `T`
- Traits with default methods and composition over inheritance; `dyn Trait`
- Generic callbacks as struct parameters (`F: Fn(...)`) for injectable transports
- `serde` with `try_from` so deserialization shares validation; `csv` for row parsing
- Half-even rounding in integer arithmetic; exact allocation

## Run

```
cargo test -p ex07_structs_and_traits
cargo test -p ex07_structs_and_traits part2
cargo solutions -p ex07_structs_and_traits
```

## Stretch (untested)

- Make `Money` generic over a currency registry loaded from a config file.
- Implement `fmt::Display` with width and alignment handling for statements.
- Give `Notifier` `Send + Sync` supertraits and dispatch channels on threads.
- Add an `Account` event log: each mutation returns an event value.
- Replace `Version` with a `const`-parsed literal macro, checked at compile time.
