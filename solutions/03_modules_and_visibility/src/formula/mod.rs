//! A spreadsheet-style formula evaluator. Private modules; the public surface
//! is re-exported from here.

mod evaluator;
mod lexer;
mod parser;

pub use evaluator::{evaluate, Env};
pub use lexer::{tokenize, Token};
pub use parser::{parse, BinOp, Expr, UnOp};

/// Errors from tokenising, parsing and evaluating a formula.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum FormulaError {
    /// A character the lexer does not recognise, with its byte position.
    #[error("unexpected character {ch:?} at byte {pos}")]
    UnexpectedChar {
        /// The offending character.
        ch: char,
        /// Byte offset in the source.
        pos: usize,
    },
    /// A numeric literal that does not parse.
    #[error("malformed number {0:?}")]
    MalformedNumber(String),
    /// A token that cannot start or continue the expression.
    #[error("unexpected token {0}")]
    UnexpectedToken(String),
    /// A cell reference with no value in the environment.
    #[error("unknown cell {0}")]
    UnknownCell(String),
    /// A function name with no definition.
    #[error("unknown function {0}")]
    UnknownFunction(String),
    /// Wrong number of arguments.
    #[error("{name} expects {expected} argument(s), got {got}")]
    Arity {
        /// Function name, uppercased.
        name: String,
        /// Human-readable expectation, e.g. `"1"` or `"at least 1"`.
        expected: &'static str,
        /// Number of arguments supplied.
        got: usize,
    },
    /// Division or remainder by zero.
    #[error("division by zero")]
    DivisionByZero,
    /// The result is NaN or infinite.
    #[error("result is not finite")]
    NotFinite,
    /// A cell or function name that is not a valid identifier.
    #[error("invalid name {0:?}")]
    InvalidName(String),
}
