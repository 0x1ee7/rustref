//! `reqkit`: a small HTTP toolkit whose signatures teach Rust's argument-passing
//! conventions. See `README.md` for the six parts.

mod encoding;
mod header;
mod patch;
mod query;
mod request;
mod transport;

pub use encoding::{encode_query, join_path, percent_encode};
pub use header::{Header, HeaderError, normalize_header_name};
pub use patch::{Patch, PatchError, User, UserPatch};
pub use query::{HasTable, NoTable, QueryError, Select, Value, select};
pub use request::{Method, Request, RequestBuilder, RequestError};
pub use transport::{Response, Transport, TransportError, send_with_retry};
