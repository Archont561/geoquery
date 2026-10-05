//! Mirrors `src/lib.rs`: the crate root, and nothing a module-specific test file already
//! covers.
//!
//! `tests/landing.rs`, `tests/request.rs` and `tests/response.rs` exercise the pure
//! translation functions without a network; `tests/adapter.rs` exercises the whole
//! `ServiceAdapter` implementation against a mock STAC server. What is left here is the
//! one thing none of those prove: that the crate links and its public names resolve from
//! outside its own build, the way any adapter crate written against `geoquery-core` would
//! have to.

use geoquery_adapter_stac::{NormalizationError, ParsedLanding, StacAdapter, StacSearchRequest};

#[test]
fn the_crate_links_from_outside_its_own_build() {
    use geoquery_adapter_stac as _;
}

#[test]
fn every_public_name_still_resolves_from_the_crate_root() {
    fn assert_is_a_type<T>() {}

    assert_is_a_type::<StacAdapter>();
    assert_is_a_type::<StacSearchRequest>();
    assert_is_a_type::<ParsedLanding>();
    assert_is_a_type::<NormalizationError>();
}
