//! The canonical query AST and the data model around it.
//!
//! This crate is the API. Everything else in the project — the CLI, the HTTP server, the
//! MCP tools, the TypeScript and Python SDKs — is an adapter to the types here, and the
//! TypeScript definitions are generated from them rather than written twice. That is why
//! it depends on `ts-rs`: a second hand-written copy of a query shape is a second thing to
//! keep correct, and the copy is the one the SDK uses.
//!
//! It depends on nothing inside the workspace. The language is not allowed to depend on
//! the engine that executes it, because the engine is a program and the language is a
//! document format; a type that can only be expressed by asking the engine what it means
//! is a type the clients cannot construct.

// Every submodule, declared together at the top of the file. Keeping this list here used
// to be impossible: `open_string_enum!` is a `macro_rules!` macro, so it is only in scope
// from its definition onwards, and `query` uses it — which forced `pub mod query;` into
// the middle of the file, below the macro. The macro now lives in `macros`, so the
// ordering constraint is gone and this is a list again rather than a workaround.
mod instant;
mod macros;

pub mod filter;
pub mod query;
pub mod resource;
pub mod result;
pub mod service;

// The data model is one flat namespace to a dependent: `geoquery_types::GeoResult`, never
// `geoquery_types::result::GeoResult`. The modules are how this crate is read, not how it
// is used, so every public name is re-exported here, and `tests/lib.rs` holds the list
// that says so — the one file that fails when this block loses a name.
pub use filter::{CompareOp, FilterExpr, FilterValidationError, FilterValue, MAX_FILTER_DEPTH};
pub use query::{
    DistanceUnit, ExecutionMode, ExecutionOptions, ExecutionPolicy, GeoQuery, IncludeOptions,
    QueryScope, QueryValidationError, Selection, SortDirection, SortExpression, SpatialPredicate,
    TemporalOperation, TemporalPredicate,
};
pub use resource::{
    ContextReference, License, Provider, RelationshipType, ResourceDescriptor,
    ResourceRelationship, ResourceType, SourceMetadata, SpatialExtent, TemporalExtent,
};
pub use result::{Asset, GeoResult, Link, Provenance, ResourceRef, ResultType, ServiceRef};
pub use service::{
    AuthDescriptor, AuthType, AxisOrder, CapabilitySet, CrsDescriptor, DimensionDescriptor,
    DimensionType, FieldDescriptor, FieldType, PagingDescriptor, PagingStyle, SchemaDescriptor,
    ServiceDescriptor, ServiceType, SpatialOperation,
};

use serde_json::{Map, Value};

/// A JSON object used for protocol metadata, source properties, and extension fields.
pub type JsonObject = Map<String, Value>;

/// Source-specific data that did not fit the protocol-independent model.
pub type ExtensionMap = JsonObject;

/// A west, south, east, north bounding box in the coordinate reference system that owns it.
pub type BoundingBox = [f64; 4];

/// A minimum and maximum scale denominator advertised by a service.
pub type ScaleRange = [f64; 2];
