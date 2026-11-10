//! Part 4 — supertraits, trait upcasting, `Any` service maps, and factories.

use std::any::{Any, TypeId};
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap};

use thiserror::Error;

/// Errors from the stage registry.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PluginError {
    /// No factory is registered under this name.
    #[error("unknown stage '{0}'")]
    UnknownStage(String),
    /// A factory is already registered under this name.
    #[error("stage '{0}' is already registered")]
    Duplicate(String),
    /// The factory rejected its configuration.
    #[error("invalid config for stage '{stage}': {reason}")]
    InvalidConfig { stage: String, reason: String },
}

/// String key/value configuration handed to a stage factory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StageConfig {
    values: BTreeMap<String, String>,
}

impl StageConfig {
    /// Empty configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets `key` to `value`, replacing any previous value.
    #[must_use]
    pub fn with(mut self, key: &str, value: &str) -> Self {
        self.values.insert(key.to_owned(), value.to_owned());
        self
    }

    /// The value for `key`, if set.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }
}

/// Anything with a stable name. Split from [`Stage`] so names can be read from
/// any supertrait-object view of a stage.
pub trait Named {
    /// Stable identifier.
    fn name(&self) -> &str;
}

/// A pipeline step. `Send` so pipelines can move between threads; `&mut self`
/// so stages may keep state across calls.
pub trait Stage: Named + Send {
    /// Transforms `input` into the next text.
    fn apply(&mut self, input: &str) -> String;
}

/// `upper`: uppercases the input.
#[derive(Debug, Default)]
pub struct Upper;

impl Named for Upper {
    fn name(&self) -> &str {
        "upper"
    }
}

impl Stage for Upper {
    fn apply(&mut self, input: &str) -> String {
        input.to_uppercase()
    }
}

/// `trim`: strips leading and trailing whitespace.
#[derive(Debug, Default)]
pub struct Trim;

impl Named for Trim {
    fn name(&self) -> &str {
        "trim"
    }
}

impl Stage for Trim {
    fn apply(&mut self, input: &str) -> String {
        input.trim().to_owned()
    }
}

/// `truncate`: keeps at most `max` chars. Built from config key `max` (>= 1).
#[derive(Debug)]
pub struct Truncate {
    max: usize,
}

impl Named for Truncate {
    fn name(&self) -> &str {
        "truncate"
    }
}

impl Stage for Truncate {
    fn apply(&mut self, input: &str) -> String {
        match input.char_indices().nth(self.max) {
            Some((byte_index, _)) => input[..byte_index].to_owned(),
            None => input.to_owned(),
        }
    }
}

/// `sequence`: prefixes each input with a 1-based call counter, e.g. `1: a`.
/// Stateful: the counter survives across calls.
#[derive(Debug, Default)]
pub struct Sequence {
    calls: u64,
}

impl Named for Sequence {
    fn name(&self) -> &str {
        "sequence"
    }
}

impl Stage for Sequence {
    fn apply(&mut self, input: &str) -> String {
        self.calls += 1;
        format!("{}: {input}", self.calls)
    }
}

/// What a factory returns.
pub type FactoryResult = Result<Box<dyn Stage>, PluginError>;

/// A stored factory: callable from any thread, boxed behind a trait object.
type Factory = Box<dyn Fn(&StageConfig) -> FactoryResult + Send + Sync>;

/// Name-keyed stage factories.
#[derive(Default)]
pub struct StageRegistry {
    factories: BTreeMap<String, Factory>,
}

impl StageRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// A registry pre-loaded with `upper`, `trim`, `truncate`, and `sequence`.
    pub fn with_builtins() -> Self {
        // Builtin names are distinct, so registration cannot fail.
        const EXPECT: &str = "builtin stage names are unique";
        let mut registry = Self::new();
        registry.register("upper", |_| Ok(Box::new(Upper))).expect(EXPECT);
        registry.register("trim", |_| Ok(Box::new(Trim))).expect(EXPECT);
        registry
            .register("truncate", |config| {
                let raw = config.get("max").ok_or_else(|| invalid("truncate", "missing 'max'"))?;
                match raw.parse::<usize>() {
                    Ok(max) if max >= 1 => Ok(Box::new(Truncate { max })),
                    _ => Err(invalid("truncate", "'max' must be an integer >= 1")),
                }
            })
            .expect(EXPECT);
        registry.register("sequence", |_| Ok(Box::new(Sequence::default()))).expect(EXPECT);
        registry
    }

    /// Registers `factory` under `name`. Fails with `Duplicate` if taken.
    pub fn register<F>(&mut self, name: &str, factory: F) -> Result<(), PluginError>
    where
        F: Fn(&StageConfig) -> FactoryResult + Send + Sync + 'static,
    {
        match self.factories.entry(name.to_owned()) {
            Entry::Occupied(_) => Err(PluginError::Duplicate(name.to_owned())),
            Entry::Vacant(slot) => {
                slot.insert(Box::new(factory));
                Ok(())
            }
        }
    }

    /// Builds a stage. Fails with `UnknownStage` or the factory's own error.
    pub fn create(&self, name: &str, config: &StageConfig) -> FactoryResult {
        match self.factories.get(name) {
            Some(factory) => factory(config),
            None => Err(PluginError::UnknownStage(name.to_owned())),
        }
    }

    /// Registered names, sorted.
    pub fn names(&self) -> Vec<&str> {
        self.factories.keys().map(String::as_str).collect()
    }
}

/// Shorthand for an `InvalidConfig` error.
fn invalid(stage: &str, reason: &str) -> PluginError {
    PluginError::InvalidConfig { stage: stage.to_owned(), reason: reason.to_owned() }
}

/// An ordered chain of stages. Each stage's output feeds the next.
#[derive(Default)]
pub struct Pipeline {
    stages: Vec<Box<dyn Stage>>,
}

impl Pipeline {
    /// An empty pipeline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a stage.
    pub fn push(&mut self, stage: Box<dyn Stage>) {
        self.stages.push(stage);
    }

    /// Runs every stage in order over `input`.
    pub fn run(&mut self, input: &str) -> String {
        self.stages
            .iter_mut()
            .fold(input.to_owned(), |text, stage| stage.apply(&text))
    }

    /// Stage names in order. Read through the `Named` supertrait.
    pub fn names(&self) -> Vec<&str> {
        self.stages.iter().map(|stage| stage.name()).collect()
    }
}

/// Upcasts a boxed stage to its `Named` supertrait object (stable since Rust 1.86).
pub fn boxed_as_named(stage: Box<dyn Stage>) -> Box<dyn Named> {
    stage
}

/// A typed service map: at most one value per Rust type.
#[derive(Default)]
pub struct Services {
    values: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Services {
    /// An empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores `value` under its type, returning the previous value of that type.
    pub fn insert<T: Any + Send + Sync>(&mut self, value: T) -> Option<T> {
        self.values
            .insert(TypeId::of::<T>(), Box::new(value))
            .map(unbox::<T>)
    }

    /// Shared access to the stored `T`.
    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.values
            .get(&TypeId::of::<T>())
            .and_then(|value| value.downcast_ref::<T>())
    }

    /// Mutable access to the stored `T`.
    pub fn get_mut<T: Any + Send + Sync>(&mut self) -> Option<&mut T> {
        self.values
            .get_mut(&TypeId::of::<T>())
            .and_then(|value| value.downcast_mut::<T>())
    }

    /// Removes and returns the stored `T`.
    pub fn remove<T: Any + Send + Sync>(&mut self) -> Option<T> {
        self.values.remove(&TypeId::of::<T>()).map(unbox::<T>)
    }

    /// `true` if a `T` is stored.
    pub fn contains<T: Any + Send + Sync>(&self) -> bool {
        self.values.contains_key(&TypeId::of::<T>())
    }
}

/// Recovers the `T` from a box keyed by `TypeId::of::<T>()`.
fn unbox<T: Any + Send + Sync>(boxed: Box<dyn Any + Send + Sync>) -> T {
    match boxed.downcast::<T>() {
        Ok(value) => *value,
        Err(_) => unreachable!("services are keyed by TypeId, so the downcast always matches"),
    }
}
