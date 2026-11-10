# 11 · Ownership, borrowing, lifetimes and smart pointers

A small platform team's toolbox: a config parser that borrows from its input,
a warehouse that moves stock and orders around without cloning, an org chart
with reporting lines that must not leak, a symbol table for a build system, a
dependency graph with a cached build order, and a document store that hands
out cheap snapshots across threads. Implement everything under `src/`. Tests
ship failing; a Part is done when its tests pass.

Signatures are the contract. Every public name below must exist in `src/`.
Design the ownership story first (who owns the data, who borrows it, what
outlives what), then write the code to match.

Run the tests from the repository root:

```
cargo test -p ex11_ownership_and_lifetimes
cargo test -p ex11_ownership_and_lifetimes part2
```

## Part 1: `Cow` escaping (`src/text.rs`)

- `escape(input: &str) -> Cow<'_, str>`: escapes `\` → `\\`, LF → `\n`,
  TAB → `\t`, CR → `\r`, `"` → `\"`. Returns `Cow::Borrowed` (same bytes)
  when nothing needs escaping.
- `escape_into(input: &str, out: &mut String)`: appends the escaped form. Never
  clears `out`.
- `unescape(input: &str) -> Result<Cow<'_, str>, UnescapeError>`: decodes
  `\\ \n \t \r \"`. Returns `Cow::Borrowed` when there is no backslash.
- `enum UnescapeError { UnknownEscape { offset, found: char }, TrailingBackslash { offset } }`:
  `offset` is the byte index of the backslash. Implements `Display` and `Error`.

Behaviour and edge cases: offsets are byte indices, not char indices; a lone
trailing backslash is an error; `unescape(escape(s)) == s` for every `s`.

Concepts exercised: `Cow<'a, str>` as a return type, borrowed-vs-owned
decisions made at runtime, `&mut String` as an append sink, typed error enums
with byte-offset diagnostics, property-based round-trip testing.

## Part 2: zero-copy INI parser (`src/ini.rs`)

- `parse(input: &str) -> Result<Document<'_>, ParseError>`.
- `Document<'a>`: `sections() -> slice::Iter<'_, Section<'a>>` (file order),
  `section(name) -> Option<&Section<'a>>` (`""` is the global section),
  `get(section, key) -> Option<&str>`,
  `to_owned_map() -> BTreeMap<String, BTreeMap<String, String>>`.
- `Section<'a>`: `name() -> &'a str`, `entries() -> slice::Iter<'_, Entry<'a>>`,
  `get(key) -> Option<&str>`, `get_as::<T: FromStr>(key) -> Option<Result<T, T::Err>>`.
- `Entry<'a> { pub key: &'a str, pub value: Cow<'a, str> }`.
- `ParseError { pub line: usize, pub kind: ParseErrorKind }` (1-based line).
  `ParseErrorKind`: `MalformedSection`, `MissingEquals`, `EmptyKey`,
  `DuplicateSection(String)`, `DuplicateKey { section, key }`, `Escape(UnescapeError)`.
  `Display` starts with `line N:`; `Error::source` returns the `UnescapeError`
  for `Escape`.

Syntax: lines are trimmed (CRLF included); blank lines and lines starting with
`;` or `#` are skipped; `[name]` opens a section; `key = value` is split at the
first `=`; keys and section names are not unescaped, values are. Entries before
any header go to `""`. Duplicate sections and duplicate keys are errors. Names
are case-sensitive. No inline comments.

Behaviour and edge cases: names and keys borrow from the input; plain values
borrow; escaped values are owned; `get_as` distinguishes absent from unparsable.

Concepts exercised: struct lifetimes tied to an input buffer, elision
(`&self` returning `&str` vs `&'a str`), `Cow` fields in a public struct, the
difference between borrowing from `self` and from the input, `Error::source`
chains, owned snapshots of borrowed data.

## Part 3: splitting borrows and moving out of `&mut` (`src/warehouse.rs`)

- `Warehouse` (`new`, `receive(sku, qty)`, `stock(sku) -> u32`):
  `receive` saturates at `u32::MAX`.
- `place_order(Order) -> Result<(), WarehouseError>`: rejects `qty == 0` with
  `ZeroQuantity { order_id }`; queues the rest at the back.
- `fulfil(&mut self) -> usize`: one FIFO pass over the backorder queue. An order
  ships when stock covers it; blocked orders keep their relative order and
  later orders may still ship. Each shipment appends `Shipment` and an audit line
  `shipped order <id>`. Returns the count shipped.
- `cancel(order_id) -> Option<Order>`, `backorders() -> &[Order]`.
- `take_shipments(&mut self) -> Vec<Shipment>` and `take_audit(&mut self) -> Vec<String>`:
  move the data out, leaving the field empty (`mem::take`).
- `replace_backorders(&mut self, Vec<Order>) -> Vec<Order>`: swap in a new queue
  and return the old one (`mem::replace`).
- `split_pair_mut<T>(items: &mut [T], i, j) -> Option<(&mut T, &mut T)>`:
  `None` if `i == j` or either index is out of bounds.
- `Staged<T: Clone>`: `new(T)`, `live() -> &T`, `staged_mut() -> &mut T`,
  `publish()`. Both buffers start as copies; `publish` swaps them, so the staging
  buffer then holds the previous live value.

Concepts exercised: disjoint field borrows inside one method (why
`self.stock` and `self.backorders` can be borrowed together), `mem::take` /
`mem::replace` / `mem::swap`, `split_at_mut`-style aliasing proofs, why
`Option<&mut T>` from `split_pair_mut` is sound, double-buffering without
cloning on publish.

## Part 4: an org chart with `Rc`, `RefCell`, `Cell` and `Weak` (`src/org.rs`)

- `Employee` is a cheap handle (`Clone` bumps a reference count) over a shared
  node. `WeakEmployee` does not keep the node alive.
- `Employee::new_root(name, salary)`, `hire(&self, name, salary) -> Employee`,
  `name() -> &str`, `salary() -> u32`, `set_salary(&self, u32)`,
  `manager() -> Option<Employee>`, `reports() -> Vec<Employee>`,
  `chain_of_command() -> Vec<Employee>` (direct manager up to the root;
  excludes self), `total_cost() -> u64` (self plus all descendants),
  `find(name) -> Option<Employee>` (depth-first, includes self),
  `downgrade() -> WeakEmployee`, `ptr_eq(&Employee) -> bool`,
  `strong_count()`, `weak_count()`.
- `transfer_to(&self, new_manager: &Employee) -> Result<(), OrgError>`: moves
  self and its subtree under `new_manager`. Root → `RootCannotMove`. If the new
  manager is self or a descendant → `Cycle`, and nothing changes. Transferring to
  the current manager is `Ok` and changes nothing.
- `WeakEmployee::upgrade() -> Option<Employee>`.

Behaviour and edge cases: the manager link is weak, so the tree has no
reference cycle and dropping the root frees everyone; `set_salary` is visible
through every handle; no `RefCell` borrow is held across a call into another
employee (that would panic on re-entry).

Concepts exercised: `Rc<T>` shared ownership, `Weak<T>` to break parent/child
cycles, `Cell<u32>` for `Copy` interior mutability, `RefCell<Vec<_>>` for
mutable child lists, `Rc::ptr_eq`, reference counts as a test oracle,
re-entrancy hazards of `RefCell`.

## Part 5a: string interners (`src/interner.rs`)

- `Symbol`: `Copy + Eq + Hash + Ord`; `index() -> usize` is its first-seen
  position (dense, starting at 0).
- `Interner` (owned strings): `new`, `intern(&mut self, &str) -> Symbol`,
  `get(&self, &str) -> Option<Symbol>` (never inserts), `resolve(Symbol) -> Option<&str>`
  (`None` if its index is out of range for this interner),
  `misses() -> u64` (failed `get` calls; counted through `&self`).
- `StrInterner<'a>` (borrowed strings): `new`, `intern(&mut self, &'a str) -> Symbol`,
  `resolve(Symbol) -> Option<&'a str>`, `len`, `is_empty`. It never copies text.

Concepts exercised: owned vs borrowed storage behind the same API, returning
`&'a str` that outlives the container (lifetime on the return type, not on
`&self`), `Cell` counters behind `&self`, why `Symbol` is an index and not a
reference.

## Part 5b: arena dependency graph with cached order (`src/graph.rs`)

- `NodeId`: `Copy + Eq + Hash + Debug`, an index into the graph's arena.
- `Graph`: `new`, `add_node(name) -> NodeId`, `name(NodeId) -> Option<&str>`,
  `add_edge(dependent, dependency) -> Result<(), GraphError>`
  (`UnknownNode` for out-of-range ids; duplicates ignored),
  `dependencies(NodeId) -> Option<&[NodeId]>` (insertion order),
  `dependents(NodeId) -> Vec<NodeId>` (ascending ids),
  `topo_order(&self) -> Result<&[NodeId], GraphError>`,
  `topo_computations() -> u32` (how many times the order was actually computed).
- `GraphError::Cycle { path }`: the DFS stack from the first re-entered node,
  closed by repeating that node. A cycle `a → b → c → a` gives `[a, b, c, a]`;
  a self-loop gives `[a, a]`.

Behaviour and edge cases: `topo_order` lists dependencies first. Nodes are
visited in ascending id order and dependencies in insertion order. The order is
cached in a `OnceCell` and only successful results are cached; any mutation
invalidates it.

Concepts exercised: arena + index handles instead of `Rc<RefCell<Node>>`,
`OnceCell::get_or_init` for lazy caches behind `&self`, `Cell` counters, why
`topo_order` can return a borrowed slice, invalidation on `&mut self`.

## Part 6: copy-on-write snapshots and cross-thread counting (`src/store.rs`)

- `Doc { pub title: String, pub lines: Vec<String> }` (`Clone`, `Default`, `Eq`).
- `Store`: `new(Doc)`, `doc() -> &Doc`, `revision() -> u64` (+1 per edit),
  `snapshot() -> Snapshot` (`Snapshot = Rc<Doc>`, O(1)),
  `edit<R>(&mut self, impl FnOnce(&mut Doc) -> R) -> R` (copy-on-write via
  `Rc::make_mut`; the document is copied only if a snapshot still holds it),
  `shared_snapshot() -> Arc<Doc>` (a deep copy that can cross threads).
  `Store` is `Clone` and sharing is O(1) until one side edits.
- `word_counts(&Doc) -> BTreeMap<String, usize>`: lowercased maximal runs of
  alphanumeric characters across the title and every line.
- `count_words_shared(&[Arc<Doc>]) -> BTreeMap<..>`: one scoped thread per
  document, all adding into one `Mutex<BTreeMap<..>>` borrowed by the threads.
- `count_words_channel(&[Arc<Doc>]) -> BTreeMap<..>`: one scoped thread per
  document, each sending its counts over an `mpsc` channel; the caller merges.

Behaviour and edge cases: a snapshot never observes later edits; `edit` without
live snapshots mutates in place; both parallel functions match a serial count;
empty input gives an empty map.

Concepts exercised: `Rc` vs `Arc` (and why `Rc` is `!Send`), `Rc::make_mut`
copy-on-write, `Rc::ptr_eq` as an allocation oracle, `thread::scope` for
borrowing across threads, `Mutex<T>` vs message passing, `Send`/`Sync`
bounds checked at compile time, choosing the merge point.

## Concepts exercised (whole exercise)

- Ownership transfer, borrowing, and the lifetimes that connect them.
- `Cow`, `Rc`/`Weak`, `Arc`, `Cell`, `RefCell`, `OnceCell`, `Mutex`, `mpsc`.
- Split borrows, `mem::take` / `replace` / `swap`, and arena indices.
- Choosing borrowed or owned return types as an API decision.

## Stretch (untested)

- Make `Document::get` return `Option<Cow<str>>` for escaped values and compare
  allocations against the current design.
- Replace `Employee` handles with an arena (`Vec<Node>` plus `NodeId`) and
  measure what the `Rc`/`Weak` version costs in memory and clarity.
- Add `Store::edit_async`-style patterns with a channel of edits and a single
  writer thread. Where does the copy-on-write cost show up?
- Give `Interner` a `Drop`-free "frozen" mode that hands out `StrInterner<'_>`
  borrowing from it. What does the borrow checker stop you doing?
