use super::FormulaError;

/// A lexical token. Identifiers are stored uppercased.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// A numeric literal.
    Number(f64),
    /// An identifier: a cell reference or a function name.
    Ident(String),
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `^`
    Caret,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `,`
    Comma,
    /// `=`
    Equals,
}

/// Splits `src` into tokens. Whitespace separates tokens and is discarded.
///
/// # Errors
///
/// [`FormulaError::UnexpectedChar`] or [`FormulaError::MalformedNumber`].
pub fn tokenize(src: &str) -> Result<Vec<Token>, FormulaError> {
    let _ = src;
    todo!()
}
