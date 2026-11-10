use std::iter::Peekable;
use std::str::CharIndices;

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
    let mut tokens = Vec::new();
    let mut chars = src.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        let token = match ch {
            c if c.is_whitespace() => continue,
            c if c.is_ascii_digit() || c == '.' => {
                let end = scan(&mut chars, start + 1, |c| c.is_ascii_digit() || c == '.');
                let literal = &src[start..end];
                let value = literal
                    .parse()
                    .map_err(|_| FormulaError::MalformedNumber(literal.to_owned()))?;
                Token::Number(value)
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let end = scan(&mut chars, start + 1, |c| {
                    c.is_ascii_alphanumeric() || c == '_'
                });
                Token::Ident(src[start..end].to_ascii_uppercase())
            }
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '%' => Token::Percent,
            '^' => Token::Caret,
            '(' => Token::LParen,
            ')' => Token::RParen,
            ',' => Token::Comma,
            '=' => Token::Equals,
            other => return Err(FormulaError::UnexpectedChar { ch: other, pos: start }),
        };
        tokens.push(token);
    }
    Ok(tokens)
}

/// Consumes characters while `keep` holds. `end` is the byte offset just past
/// the characters already accepted; returns the offset after the last kept one.
fn scan(
    chars: &mut Peekable<CharIndices<'_>>,
    mut end: usize,
    keep: impl Fn(char) -> bool,
) -> usize {
    while let Some(&(index, c)) = chars.peek() {
        if !keep(c) {
            break;
        }
        end = index + c.len_utf8();
        chars.next();
    }
    end
}
