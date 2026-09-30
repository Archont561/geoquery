//! STAC as a geoquery source.
//!
//! Translates a `GeoQuery` into a STAC API search, and a STAC `ItemCollection` back into a
//! `GeoResult` with provenance. Detection is by `conformsTo` and the landing page, not by
//! URL shape, because a STAC API is usually behind a path prefix nobody controls.
//!
//! Lands in Phase 2. Scaffolding only; see the note in `../Cargo.toml` about the
//! dependency graph.
