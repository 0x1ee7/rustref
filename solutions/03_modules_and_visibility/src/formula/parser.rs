use std::collections::BTreeSet;

use super::lexer::{tokenize, Token};
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
        let mut cells = BTreeSet::new();
        self.collect_cells(&mut cells);
        cells
    }

    fn collect_cells(&self, cells: &mut BTreeSet<String>) {
        match self {
            Expr::Number(_) => {}
            Expr::Cell(name) => {
                cells.insert(name.clone());
            }
            Expr::Unary(_, inner) => inner.collect_cells(cells),
            Expr::Binary(_, lhs, rhs) => {
                lhs.collect_cells(cells);
                rhs.collect_cells(cells);
            }
            Expr::Call(_, args) => {
                for arg in args {
                    arg.collect_cells(cells);
                }
            }
        }
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
    let mut tokens = tokenize(src)?;
    if tokens.first() == Some(&Token::Equals) {
        tokens.remove(0);
    }
    let mut parser = Parser { tokens, pos: 0 };
    let expr = parser.expr()?;
    match parser.advance() {
        None => Ok(expr),
        Some(extra) => Err(unexpected(Some(&extra))),
    }
}

fn unexpected(token: Option<&Token>) -> FormulaError {
    FormulaError::UnexpectedToken(match token {
        Some(token) => format!("{token:?}"),
        None => "end of input".to_owned(),
    })
}

/// Recursive-descent parser over a token vector.
struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn expect(&mut self, want: &Token) -> Result<(), FormulaError> {
        if self.peek() == Some(want) {
            self.pos += 1;
            Ok(())
        } else {
            Err(unexpected(self.peek()))
        }
    }

    /// `term (('+' | '-') term)*`
    fn expr(&mut self) -> Result<Expr, FormulaError> {
        let mut lhs = self.term()?;
        loop {
            let op = match self.peek() {
                Some(Token::Plus) => BinOp::Add,
                Some(Token::Minus) => BinOp::Sub,
                _ => break,
            };
            self.pos += 1;
            let rhs = self.term()?;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    /// `power (('*' | '/' | '%') power)*`
    fn term(&mut self) -> Result<Expr, FormulaError> {
        let mut lhs = self.power()?;
        loop {
            let op = match self.peek() {
                Some(Token::Star) => BinOp::Mul,
                Some(Token::Slash) => BinOp::Div,
                Some(Token::Percent) => BinOp::Rem,
                _ => break,
            };
            self.pos += 1;
            let rhs = self.power()?;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    /// `unary ('^' power)?`, right-associative.
    fn power(&mut self) -> Result<Expr, FormulaError> {
        let base = self.unary()?;
        if self.peek() == Some(&Token::Caret) {
            self.pos += 1;
            let exponent = self.power()?;
            return Ok(Expr::Binary(BinOp::Pow, Box::new(base), Box::new(exponent)));
        }
        Ok(base)
    }

    /// Prefix `-` / `+` applied to a unary, or a primary.
    fn unary(&mut self) -> Result<Expr, FormulaError> {
        let op = match self.peek() {
            Some(Token::Minus) => UnOp::Neg,
            Some(Token::Plus) => UnOp::Plus,
            _ => return self.primary(),
        };
        self.pos += 1;
        Ok(Expr::Unary(op, Box::new(self.unary()?)))
    }

    fn primary(&mut self) -> Result<Expr, FormulaError> {
        match self.advance() {
            Some(Token::Number(value)) => Ok(Expr::Number(value)),
            Some(Token::Ident(name)) => {
                if self.peek() == Some(&Token::LParen) {
                    self.pos += 1;
                    Ok(Expr::Call(name, self.args()?))
                } else {
                    Ok(Expr::Cell(name))
                }
            }
            Some(Token::LParen) => {
                let inner = self.expr()?;
                self.expect(&Token::RParen)?;
                Ok(inner)
            }
            other => Err(unexpected(other.as_ref())),
        }
    }

    /// Arguments after `(`, consuming the closing `)`.
    fn args(&mut self) -> Result<Vec<Expr>, FormulaError> {
        let mut args = Vec::new();
        if self.peek() == Some(&Token::RParen) {
            self.pos += 1;
            return Ok(args);
        }
        loop {
            args.push(self.expr()?);
            match self.advance() {
                Some(Token::Comma) => {}
                Some(Token::RParen) => return Ok(args),
                other => return Err(unexpected(other.as_ref())),
            }
        }
    }
}
