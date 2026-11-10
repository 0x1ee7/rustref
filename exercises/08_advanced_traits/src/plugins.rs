//! Part 4 — supertraits, trait upcasting, `Any` service maps, and factories.

use std::any::Any;

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
///
/// Engineer: store the pairs in a `BTreeMap<String, String>`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StageConfig {}

impl StageConfig {
    /// Empty configuration.
    pub fn new() -> Self {
        todo!()
    }

    /// Sets `key` to `value`, replacing any previous value.
    #[must_use]
    pub fn with(self, key: &str, value: &str) -> Self {
        let _ = (self, key, value);
        todo!()
    }

    /// The value for `key`, if set.
    pub fn get(&self, key: &str) -> Option<&str> {
        let _ = (self, key);
        todo!()
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
        let _ = self;
        todo!()
    }
}

impl Stage for Upper {
    fn apply(&mut self, input: &str) -> String {
        let _ = (self, input);
        todo!()
    }
}

/// `trim`: strips leading and trailing whitespace.
#[derive(Debug, Default)]
pub struct Trim;

impl Named for Trim {
    fn name(&self) -> &str {
        let _ = self;
        todo!()
    }
}

impl Stage for Trim {
    fn apply(&mut self, input: &str) -> String {
        let _ = (self, input);
        todo!()
    }
}

/// `truncate`: keeps at most `max` chars. Built from config key `max` (>= 1).
///
/// Engineer: add the `max` field.
#[derive(Debug)]
pub struct Truncate;

impl Named for Truncate {
    fn name(&self) -> &str {
        let _ = self;
        todo!()
    }
}

impl Stage for Truncate {
    fn apply(&mut self, input: &str) -> String {
        let _ = (self, input);
        todo!()
    }
}

/// `sequence`: prefixes each input with a 1-based call counter, e.g. `1: a`.
/// Stateful: the counter survives across calls.
///
/// Engineer: add the counter field.
#[derive(Debug, Default)]
pub struct Sequence;

impl Named for Sequence {
    fn name(&self) -> &str {
        let _ = self;
        todo!()
    }
}

impl Stage for Sequence {
    fn apply(&mut self, input: &str) -> String {
        let _ = (self, input);
        todo!()
    }
}

/// What a factory returns.
pub type FactoryResult = Result<Box<dyn Stage>, PluginError>;

/// Name-keyed stage factories.
///
/// Engineer: store `Box<dyn Fn(&StageConfig) -> FactoryResult + Send + Sync>`
/// values in a `BTreeMap<String, _>`.
#[derive(Default)]
pub struct StageRegistry {}

impl StageRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        todo!()
    }

    /// A registry pre-loaded with `upper`, `trim`, `truncate`, and `sequence`.
    pub fn with_builtins() -> Self {
        todo!()
    }

    /// Registers `factory` under `name`. Fails with `Duplicate` if taken.
    pub fn register<F>(&mut self, name: &str, factory: F) -> Result<(), PluginError>
    where
        F: Fn(&StageConfig) -> FactoryResult + Send + Sync + 'static,
    {
        let _ = (self, name, factory);
        todo!()
    }

    /// Builds a stage. Fails with `UnknownStage` or the factory's own error.
    pub fn create(&self, name: &str, config: &StageConfig) -> FactoryResult {
        let _ = (self, name, config);
        todo!()
    }

    /// Registered names, sorted.
    pub fn names(&self) -> Vec<&str> {
        let _ = self;
        todo!()
    }
}

/// An ordered chain of stages. Each stage's output feeds the next.
#[derive(Default)]
pub struct Pipeline {}

impl Pipeline {
    /// An empty pipeline.
    pub fn new() -> Self {
        todo!()
    }

    /// Appends a stage.
    pub fn push(&mut self, stage: Box<dyn Stage>) {
        let _ = (self, stage);
        todo!()
    }

    /// Runs every stage in order over `input`.
    pub fn run(&mut self, input: &str) -> String {
        let _ = (self, input);
        todo!()
    }

    /// Stage names in order. Read through the `Named` supertrait.
    pub fn names(&self) -> Vec<&str> {
        let _ = self;
        todo!()
    }
}

/// Upcasts a boxed stage to its `Named` supertrait object (stable since Rust 1.86).
pub fn boxed_as_named(stage: Box<dyn Stage>) -> Box<dyn Named> {
    let _ = stage;
    todo!()
}

/// A typed service map: at most one value per Rust type.
///
/// Engineer: store `Box<dyn Any + Send + Sync>` values keyed by `TypeId`.
#[derive(Default)]
pub struct Services {}

impl Services {
    /// An empty map.
    pub fn new() -> Self {
        todo!()
    }

    /// Stores `value` under its type, returning the previous value of that type.
    pub fn insert<T: Any + Send + Sync>(&mut self, value: T) -> Option<T> {
        let _ = (self, value);
        todo!()
    }

    /// Shared access to the stored `T`.
    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        let _ = self;
        todo!()
    }

    /// Mutable access to the stored `T`.
    pub fn get_mut<T: Any + Send + Sync>(&mut self) -> Option<&mut T> {
        let _ = self;
        todo!()
    }

    /// Removes and returns the stored `T`.
    pub fn remove<T: Any + Send + Sync>(&mut self) -> Option<T> {
        let _ = self;
        todo!()
    }

    /// `true` if a `T` is stored.
    pub fn contains<T: Any + Send + Sync>(&self) -> bool {
        let _ = self;
        todo!()
    }
}
