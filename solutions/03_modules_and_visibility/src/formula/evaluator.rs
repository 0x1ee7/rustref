use std::collections::HashMap;

use super::parser::{parse, BinOp, Expr, UnOp};
use super::FormulaError;

/// Evaluation environment: cell values and named functions. Passed explicitly
/// to [`evaluate`]; there is no global state.
#[derive(Debug, Clone, Default)]
pub struct Env {
    cells: HashMap<String, f64>,
    /// Whether the built-in functions are callable.
    builtins: bool,
}

impl Env {
    /// An empty environment with no cells and no functions.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// An environment preloaded with the built-in functions:
    /// `ABS`(1), `MIN`(at least 1), `MAX`(at least 1), `SUM`(at least 0),
    /// `ROUND`(1 or 2 arguments; half away from zero, default 0 digits).
    #[must_use]
    pub fn with_builtins() -> Self {
        Self {
            builtins: true,
            ..Self::default()
        }
    }

    /// Sets a cell value. The name is uppercased; it must be a valid identifier.
    ///
    /// # Errors
    ///
    /// [`FormulaError::InvalidName`] when the name is not an identifier.
    pub fn set_cell(&mut self, name: &str, value: f64) -> Result<(), FormulaError> {
        if !is_identifier(name) {
            return Err(FormulaError::InvalidName(name.to_owned()));
        }
        self.cells.insert(name.to_ascii_uppercase(), value);
        Ok(())
    }

    /// Looks up a cell value by (case-insensitive) name.
    #[must_use]
    pub fn cell(&self, name: &str) -> Option<f64> {
        self.cells.get(&name.to_ascii_uppercase()).copied()
    }
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
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
    let expr = parse(src)?;
    let value = eval(&expr, env)?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(FormulaError::NotFinite)
    }
}

fn eval(expr: &Expr, env: &Env) -> Result<f64, FormulaError> {
    match expr {
        Expr::Number(value) => Ok(*value),
        Expr::Cell(name) => env
            .cell(name)
            .ok_or_else(|| FormulaError::UnknownCell(name.clone())),
        Expr::Unary(op, inner) => {
            let value = eval(inner, env)?;
            Ok(match op {
                UnOp::Neg => -value,
                UnOp::Plus => value,
            })
        }
        Expr::Binary(op, lhs, rhs) => {
            let l = eval(lhs, env)?;
            let r = eval(rhs, env)?;
            match op {
                BinOp::Add => Ok(l + r),
                BinOp::Sub => Ok(l - r),
                BinOp::Mul => Ok(l * r),
                BinOp::Div | BinOp::Rem if r == 0.0 => Err(FormulaError::DivisionByZero),
                BinOp::Div => Ok(l / r),
                BinOp::Rem => Ok(l % r),
                BinOp::Pow => Ok(l.powf(r)),
            }
        }
        Expr::Call(name, args) => call(name, args, env),
    }
}

fn call(name: &str, args: &[Expr], env: &Env) -> Result<f64, FormulaError> {
    if !env.builtins {
        return Err(FormulaError::UnknownFunction(name.to_owned()));
    }
    let (expected, arity_ok) = match name {
        "ABS" => ("1", args.len() == 1),
        "MIN" | "MAX" => ("at least 1", !args.is_empty()),
        "SUM" => ("at least 0", true),
        "ROUND" => ("1 or 2", matches!(args.len(), 1 | 2)),
        _ => return Err(FormulaError::UnknownFunction(name.to_owned())),
    };
    if !arity_ok {
        return Err(FormulaError::Arity {
            name: name.to_owned(),
            expected,
            got: args.len(),
        });
    }
    let values = args
        .iter()
        .map(|arg| eval(arg, env))
        .collect::<Result<Vec<f64>, _>>()?;
    Ok(match name {
        "ABS" => values[0].abs(),
        "MIN" => values.iter().copied().fold(f64::INFINITY, f64::min),
        "MAX" => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        "SUM" => values.iter().sum(),
        "ROUND" => round(values[0], values.get(1).copied().unwrap_or(0.0)),
        _ => unreachable!("function name validated above"),
    })
}

/// Rounds half away from zero to `digits` decimal places.
fn round(value: f64, digits: f64) -> f64 {
    let scale = 10f64.powi(digits.trunc() as i32);
    (value * scale).round() / scale
}
