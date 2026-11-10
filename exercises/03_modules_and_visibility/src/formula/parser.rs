use std::collections::BTreeSet;

use super::FormulaError;

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%` (Rust remainder semantics)
    Rem,
    /// `^` (right-associative)
    Pow,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    /// Prefix `-`
    Neg,
    /// Prefix `+`
    Plus,
}

/// A parsed expression tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A literal.
    Number(f64),
    /// A cell reference, uppercased.
    Cell(String),
    /// A prefix operation.
    Unary(UnOp, Box<Expr>),
    /// An infix operation.
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// A function call, name uppercased.
    Call(String, Vec<Expr>),
}

impl Expr {
    /// Every cell name referenced, sorted and deduplicated. Function names are excluded.
    #[must_use]
    pub fn references(&self) -> BTreeSet<String> {
        todo!()
    }
}

/// Parses a formula. A single leading `=` is accepted and ignored.
///
/// Precedence, lowest to highest: `+ -`, `* / %`, `^` (right-associative),
/// then prefix `- +`. Prefix operators bind tighter than `^`, so `-2^2` is `(-2)^2` = 4.
///
/// # Errors
///
/// Any lexing error, or [`FormulaError::UnexpectedToken`] for a malformed expression.
pub fn parse(src: &str) -> Result<Expr, FormulaError> {
    let _ = src;
    todo!()
}
