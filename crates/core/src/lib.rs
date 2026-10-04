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
//!
//! Alongside it this crate owns the *contracts* the engine is built from — what an
//! adapter is ([`adapter`]), what it may be asked to do ([`capability`]) and what an
//! execution across several of them amounts to ([`execution`]) — without owning a single
//! implementation of them. The dependency arrow runs one way, from adapters to here, so
//! that an adapter can be written against this crate alone and the engine can hold one
//! without knowing which protocol it speaks.

pub mod adapter;
pub mod capability;
pub mod document;
pub mod execution;
pub mod version;

pub use adapter::{
    AdapterError, Confidence, ConfidenceOutOfRange, Detection, Endpoint, QueryResult,
    ServiceAdapter,
};
pub use capability::{CapabilityFinding, CapabilityReport, Cause, QueryFeature, Support};
pub use document::{QueryDocument, QueryDocumentError};
pub use execution::{ExecutionOutcome, ExecutionStatus, SourceOutcome, SourceStatus};
pub use version::{PROTOCOL_VERSION, VERSION, user_agent};
