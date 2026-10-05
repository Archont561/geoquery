//! STAC as a geoquery source.
//!
//! Translates a `GeoQuery` into a STAC API `/search` request, and a STAC `ItemCollection`
//! back into `GeoResult` values with provenance. Detection is by `conformsTo` and the
//! landing page, not by URL shape, because a STAC API is usually behind a path prefix
//! nobody controls.
//!
//! This is the first live-data spike, and its scope is deliberately narrow: `bbox`,
//! `datetime`, `collections` and `limit` are the only `GeoQuery` members this adapter ever
//! sends to a service. Everything else a query asks for is reported as a
//! [`geoquery_core::CapabilityFinding`] rather than attempted — `geoquery-core`'s generic
//! capability machinery assumes an engine that can apply a predicate locally once it has
//! normalized results in hand, and this spike does not implement that local step yet, so
//! claiming [`geoquery_core::Support::Local`] for anything beyond field projection would be
//! a finding this adapter could not back up. [`request::translate`] and the module docs on
//! [`request`] say exactly what is pushed and what is refused, and why.
//!
//! See `.knowledge/adapters/stac.md` for the target design this spike narrows, and
//! `.knowledge/research/open-service-targets.md` for the live services it was built and
//! tested against.

pub mod adapter;
pub mod landing;
pub mod request;
pub mod response;

pub use adapter::StacAdapter;
pub use landing::{ParsedLanding, collection_ids, parse_landing};
pub use request::{StacSearchRequest, translate};
pub use response::{NormalizationError, normalize_response};
