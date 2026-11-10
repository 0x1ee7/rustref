use std::collections::HashMap;

use super::FormulaError;

/// Evaluation environment: cell values and named functions. Passed explicitly
/// to [`evaluate`]; there is no global state.
#[allow(dead_code)]
#[derive(Default)]
pub struct Env {
    cells: HashMap<String, f64>,
}

impl Env {
    /// An empty environment with no cells and no functions.
    #[must_use]
    pub fn new() -> Self {
        todo!()
    }

    /// An environment preloaded with the built-in functions:
    /// `ABS`(1), `MIN`(at least 1), `MAX`(at least 1), `SUM`(at least 0),
    /// `ROUND`(1 or 2 arguments; half away from zero, default 0 digits).
    #[must_use]
    pub fn with_builtins() -> Self {
        todo!()
    }

    /// Sets a cell value. The name is uppercased; it must be a valid identifier.
    ///
    /// # Errors
    ///
    /// [`FormulaError::InvalidName`] when the name is not an identifier.
    pub fn set_cell(&mut self, name: &str, value: f64) -> Result<(), FormulaError> {
        let _ = (name, value);
        todo!()
    }

    /// Looks up a cell value by (case-insensitive) name.
    #[must_use]
    pub fn cell(&self, name: &str) -> Option<f64> {
        let _ = name;
        todo!()
    }
}

/// Evaluates `src` against `env`. A single leading `=` is accepted.
///
/// Function lookup happens before arguments are evaluated, so an unknown
/// function is reported even when its arguments would fail.
///
/// # Errors
///
/// Any [`FormulaError`] from parsing or evaluation.
pub fn evaluate(src: &str, env: &Env) -> Result<f64, FormulaError> {
    let _ = (src, env);
    todo!()
}
