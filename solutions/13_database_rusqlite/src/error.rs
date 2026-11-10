/// Every failure the persistence layer reports.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A driver error without a domain meaning.
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),

    /// A migration failed and its transaction was rolled back.
    #[error("migration {version} ({name}) failed")]
    Migration {
        version: u32,
        name: &'static str,
        #[source]
        source: rusqlite::Error,
    },

    /// `user_version` is newer than the newest migration this binary knows.
    #[error("database schema version {found} is newer than supported version {supported}")]
    SchemaTooNew { found: u32, supported: u32 },

    /// Migrations must be numbered `1..=n` in slice order.
    #[error("migrations must be numbered 1..n in order: expected {expected}, found {found}")]
    MigrationOrder { expected: u32, found: u32 },

    /// A caller passed a value outside the accepted domain.
    #[error("invalid input: {0}")]
    InvalidInput(&'static str),

    /// A product price was rejected (negative).
    #[error("invalid price for {sku}: {reason}")]
    InvalidPrice { sku: String, reason: &'static str },

    /// A string is not one of the known order statuses.
    #[error("unknown order status {0:?}")]
    UnknownStatus(String),

    /// Order lines reference SKUs that are not in the catalogue. Sorted, de-duplicated.
    #[error("unknown product SKUs: {}", .0.join(", "))]
    UnknownProducts(Vec<String>),

    /// Customer email is already registered (case-insensitive).
    #[error("email already registered: {0}")]
    DuplicateEmail(String),

    /// An order references a customer id that does not exist.
    #[error("unknown customer {0}")]
    UnknownCustomer(i64),

    /// No order has the given id.
    #[error("order {0} not found")]
    OrderNotFound(i64),

    /// A pagination token could not be decoded.
    #[error("invalid pagination cursor")]
    InvalidCursor,
}

/// Crate-wide result alias.
pub type Result<T, E = Error> = std::result::Result<T, E>;
