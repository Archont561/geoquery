//! The geoquery query language.
//!
//! This crate owns everything about a query that is not about talking to anyone: the
//! document, its version, and the rules that make one valid. The CLI
//! (`geoquery-cli`) is a presentation layer over it, and so will the service be — which
//! is the only reason the split exists. A rule that lives in the binary is a rule the
//! server has to reimplement.
//!
//! Phase 0 is the part of the language that does not depend on the engine being able to
//! execute anything: reading a document, and saying what is wrong with one. That is the
//! half of a query language that never goes stale, and it is what lets a client
//! validate a document today against a server that arrives later.

pub mod document;
pub mod version;

pub use document::{QueryDocument, QueryDocumentError};
pub use version::{PROTOCOL_VERSION, VERSION, user_agent};
