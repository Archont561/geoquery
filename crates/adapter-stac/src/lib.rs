//! STAC as a geoquery source.
//!
//! Translates a `GeoQuery` into a STAC API `/search` request, and a STAC `ItemCollection`
//! back into `GeoResult` values with provenance. Detection is by `conformsTo` and the
//! landing page, not by URL shape, because a STAC API is usually behind a path prefix
//! nobody controls.
//!
//! What reaches a service is decided by what that service said it conforms to, never by
//! which provider it is. STAC API core — `bbox`, `datetime`, `collections`, `limit` — is
//! always sent. Attribute filtering (as CQL2 JSON), sorting and field selection are
//! extensions, and each is pushed only to a service whose landing page advertised the
//! matching conformance class; see [`landing::StacConformance`]. Everything a query asks
//! for that cannot be sent is reported as a [`geoquery_core::CapabilityFinding`] rather
//! than dropped silently or raised as an error — [`request::translate`] and the module
//! docs on [`request`] say exactly what is pushed, what is refused, what is done locally,
//! and why in each case.
//!
//! The local work this adapter does is deliberately limited to steps that remove nothing
//! a predicate would have kept: trimming each result's properties to the fields a caller
//! named, and cutting an `offset`/`limit` window out of a sequence of pages. Narrowing
//! post-processing — evaluating an exact geometry or an attribute predicate against
//! normalized results — still does not exist anywhere in this workspace, which is why no
//! finding here is ever [`geoquery_core::Support::Approximated`].
//!
//! See `.knowledge/adapters/stac.md` for the target design, `.knowledge/project/`'s spike
//! reports for what each pass proved, and `.knowledge/research/open-service-targets.md`
//! for the live services this was built against.

pub mod adapter;
pub mod cql2;
pub mod landing;
pub mod request;
pub mod response;

pub use adapter::StacAdapter;
pub use landing::{ParsedLanding, StacConformance, collection_ids, parse_landing};
pub use request::{StacFields, StacSearchRequest, StacSortBy, StacSortDirection, translate};
pub use response::{NextLink, NormalizationError, normalize_response, project_fields};
