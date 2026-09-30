//! OGC API as a geoquery source.
//!
//! Two protocols, not one: OGC API Features for vector data and OGC API Coverages for
//! raster. Features needs a CRS transform, because `EPSG:2180` is what a national service
//! publishes and WGS84 is what a query arrives in — and that transform is the reason this
//! crate is Phase 2 and not Phase 0, because a wrong answer in the wrong CRS is worse
//! than no answer.
//!
//! GZIP is not optional here. OGC API Features requires a server to accept it in
//! `Accept-Encoding`, and a Features response without it routinely exceeds what a query
//! should return, so the adapter asks and flate2 decodes what arrives.
//!
//! Lands in Phase 2. Scaffolding only; see the note in `../Cargo.toml`.
