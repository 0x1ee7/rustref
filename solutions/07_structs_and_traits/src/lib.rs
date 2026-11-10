//! Structs, traits and value types for a commerce and banking core.
//!
//! Each module is one part of the exercise; see `README.md`.

mod account;
mod cart;
mod employee;
mod money;
mod notify;
mod product;
mod version;

pub use account::{Account, AccountError, CheckingAccount, Ledger, SavingsAccount};
pub use cart::{CartError, CartLine, ShoppingCart};
pub use employee::{Email, Employee, EmployeeError, EmployeeRecord};
pub use money::{Currency, Money, MoneyError};
pub use notify::{
    BoxError, EmailNotifier, NotificationDispatcher, NotificationError, Notifier, SMS_MAX_CHARS,
    SmsNotifier, WebhookNotifier,
};
pub use product::{Product, ProductError, Sku};
pub use version::{Part, PreIdentifier, Version, VersionError};
