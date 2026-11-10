//! Part 5 — a consuming typestate builder: `build` exists only after a table
//! has been chosen, so a missing table is a compile error, not a runtime one.

/// State before `from_table` has been called.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoTable;

/// State after `from_table`: carries the table name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HasTable {
    table: String,
}

/// A SQL parameter.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Text(String),
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::Text(v.to_owned())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::Text(v)
    }
}

/// `None` becomes [`Value::Null`]; `Some(v)` converts `v`.
impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        v.map_or(Value::Null, Into::into)
    }
}

/// Why a query could not be rendered.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum QueryError {
    /// A column, table or filter name is not `ident` or `ident.ident`.
    #[error("invalid identifier {0:?}")]
    InvalidIdentifier(String),
    /// An `order_by` term is not `[-]ident[.ident]`.
    #[error("invalid order term {0:?}")]
    InvalidOrderTerm(String),
    /// `limit` does not fit the `i64` bound parameter.
    #[error("LIMIT {0} does not fit in i64")]
    LimitTooLarge(u64),
}

/// A `SELECT` under construction. The type parameter records whether a table
/// has been chosen.
#[derive(Debug, Clone, PartialEq)]
pub struct Select<State> {
    columns: Vec<String>,
    state: State,
    filters: Vec<(String, Value)>,
    order: Vec<String>,
    limit: Option<u64>,
}

/// Starts a query projecting `columns`. An empty list renders `*`. Column names
/// are validated by [`Select::build`].
pub fn select<I, S>(columns: I) -> Select<NoTable>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    Select {
        columns: columns.into_iter().map(|c| c.as_ref().to_owned()).collect(),
        state: NoTable,
        filters: Vec::new(),
        order: Vec::new(),
        limit: None,
    }
}

impl Select<NoTable> {
    /// Chooses the table. Consumes the builder and returns the next state, where
    /// `build` becomes available.
    pub fn from_table(self, table: impl AsRef<str>) -> Select<HasTable> {
        Select {
            columns: self.columns,
            state: HasTable {
                table: table.as_ref().to_owned(),
            },
            filters: self.filters,
            order: self.order,
            limit: self.limit,
        }
    }
}

impl Select<HasTable> {
    /// Adds `column = ?` (or `column IS NULL` for a null value). Filters are
    /// joined with `AND` in call order.
    pub fn filter_eq(mut self, column: impl AsRef<str>, value: impl Into<Value>) -> Self {
        self.filters
            .push((column.as_ref().to_owned(), value.into()));
        self
    }

    /// Appends an order term. A leading `-` means `DESC`. Terms are rendered in
    /// call order.
    pub fn order_by(mut self, term: impl AsRef<str>) -> Self {
        self.order.push(term.as_ref().to_owned());
        self
    }

    /// Sets `LIMIT ?`. The last call wins.
    pub fn limit(mut self, n: u64) -> Self {
        self.limit = Some(n);
        self
    }

    /// Renders the SQL text and its parameters in placeholder order.
    pub fn build(self) -> Result<(String, Vec<Value>), QueryError> {
        let table = self.state.table;

        if let Some(bad) = self.columns.iter().find(|c| !is_identifier(c)) {
            return Err(QueryError::InvalidIdentifier(bad.clone()));
        }
        if !is_identifier(&table) {
            return Err(QueryError::InvalidIdentifier(table));
        }
        if let Some((bad, _)) = self.filters.iter().find(|(c, _)| !is_identifier(c)) {
            return Err(QueryError::InvalidIdentifier(bad.clone()));
        }
        let order = self
            .order
            .iter()
            .map(|term| order_term(term))
            .collect::<Result<Vec<_>, _>>()?;
        let limit = self
            .limit
            .map(|n| i64::try_from(n).map_err(|_| QueryError::LimitTooLarge(n)))
            .transpose()?;

        let mut sql = String::from("SELECT ");
        if self.columns.is_empty() {
            sql.push('*');
        } else {
            sql.push_str(&self.columns.join(", "));
        }
        sql.push_str(" FROM ");
        sql.push_str(&table);

        let mut params = Vec::new();
        for (i, (column, value)) in self.filters.into_iter().enumerate() {
            sql.push_str(if i == 0 { " WHERE " } else { " AND " });
            sql.push_str(&column);
            match value {
                Value::Null => sql.push_str(" IS NULL"),
                other => {
                    sql.push_str(" = ?");
                    params.push(other);
                }
            }
        }

        for (i, (ident, desc)) in order.iter().enumerate() {
            sql.push_str(if i == 0 { " ORDER BY " } else { ", " });
            sql.push_str(ident);
            if *desc {
                sql.push_str(" DESC");
            }
        }

        if let Some(n) = limit {
            sql.push_str(" LIMIT ?");
            params.push(Value::Int(n));
        }

        Ok((sql, params))
    }
}

/// `[A-Za-z_][A-Za-z0-9_]*`
fn is_simple_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `ident` or `ident.ident`.
fn is_identifier(s: &str) -> bool {
    match s.split_once('.') {
        Some((qualifier, name)) => is_simple_identifier(qualifier) && is_simple_identifier(name),
        None => is_simple_identifier(s),
    }
}

/// Splits a `[-]ident[.ident]` term into the identifier and its direction.
fn order_term(term: &str) -> Result<(&str, bool), QueryError> {
    let (ident, desc) = match term.strip_prefix('-') {
        Some(rest) => (rest, true),
        None => (term, false),
    };
    if is_identifier(ident) {
        Ok((ident, desc))
    } else {
        Err(QueryError::InvalidOrderTerm(term.to_owned()))
    }
}
