//! Mirrors `src/landing.rs`: is this a STAC API, where is `/search`, and what does
//! `conformsTo` buy a caller.
//!
//! The fixtures below are trimmed, real landing pages — not invented shapes — captured
//! from Earth Search and Microsoft Planetary Computer on 2026-10-05, because the two
//! differ in exactly the ways that matter here: Earth Search advertises the legacy
//! `ogcapi-features#query` extension and no filter conformance class, while Planetary
//! Computer advertises CQL2 and the modern `item-search#filter` class. A parser that only
//! worked on one would be a parser tuned to a provider rather than to the standard.

use geoquery_adapter_stac::{collection_ids, parse_landing};
use geoquery_types::{AxisOrder, SpatialOperation};
use pretty_assertions::assert_eq;
use serde_json::json;

/// Trimmed from `https://earth-search.aws.element84.com/v1`, fetched 2026-10-05.
fn earth_search_landing() -> serde_json::Value {
    json!({
        "stac_version": "1.0.0",
        "type": "Catalog",
        "id": "earth-search-aws",
        "title": "Earth Search by Element 84",
        "links": [
            { "rel": "self", "type": "application/json", "href": "https://earth-search.aws.element84.com/v1" },
            { "rel": "conformance", "type": "application/json", "href": "https://earth-search.aws.element84.com/v1/conformance" },
            { "rel": "data", "type": "application/json", "href": "https://earth-search.aws.element84.com/v1/collections" },
            { "rel": "search", "type": "application/geo+json", "href": "https://earth-search.aws.element84.com/v1/search", "method": "GET" },
            { "rel": "search", "type": "application/geo+json", "href": "https://earth-search.aws.element84.com/v1/search", "method": "POST" }
        ],
        "conformsTo": [
            "https://api.stacspec.org/v1.0.0/core",
            "https://api.stacspec.org/v1.0.0/collections",
            "https://api.stacspec.org/v1.0.0/ogcapi-features",
            "https://api.stacspec.org/v1.0.0/item-search",
            "https://api.stacspec.org/v1.0.0/ogcapi-features#fields",
            "https://api.stacspec.org/v1.0.0/ogcapi-features#sort",
            "https://api.stacspec.org/v1.0.0/ogcapi-features#query",
            "https://api.stacspec.org/v1.0.0/item-search#fields",
            "https://api.stacspec.org/v1.0.0/item-search#sort",
            "https://api.stacspec.org/v1.0.0/item-search#query"
        ]
    })
}

/// Trimmed from `https://planetarycomputer.microsoft.com/api/stac/v1`, fetched
/// 2026-10-05: no `data` link, and both `item-search#filter` and CQL2 conformance.
fn planetary_computer_landing() -> serde_json::Value {
    json!({
        "type": "Catalog",
        "stac_version": "1.0.0",
        "id": "microsoft-pc",
        "title": "Microsoft Planetary Computer STAC API",
        "links": [
            { "rel": "self", "href": "https://planetarycomputer.microsoft.com/api/stac/v1/", "type": "application/json" },
            { "rel": "conformance", "href": "https://planetarycomputer.microsoft.com/api/stac/v1/conformance", "type": "application/json" },
            { "rel": "search", "href": "https://planetarycomputer.microsoft.com/api/stac/v1/search", "method": "GET", "type": "application/geo+json" },
            { "rel": "search", "href": "https://planetarycomputer.microsoft.com/api/stac/v1/search", "method": "POST", "type": "application/geo+json" }
        ],
        "conformsTo": [
            "http://www.opengis.net/spec/cql2/1.0/conf/basic-cql2",
            "http://www.opengis.net/spec/cql2/1.0/conf/cql2-json",
            "http://www.opengis.net/spec/ogcapi-features-3/1.0/conf/filter",
            "https://api.stacspec.org/v1.0.0-rc.2/item-search#filter",
            "https://api.stacspec.org/v1.0.0/collections",
            "https://api.stacspec.org/v1.0.0/core",
            "https://api.stacspec.org/v1.0.0/item-search"
        ]
    })
}

#[test]
fn earth_search_is_recognised_and_its_search_link_is_used() {
    let parsed = parse_landing(
        "https://earth-search.aws.element84.com/v1",
        &earth_search_landing(),
    )
    .expect("a conforming landing page parses");

    assert_eq!(
        parsed.search_url,
        "https://earth-search.aws.element84.com/v1/search"
    );
    assert_eq!(
        parsed.collections_url,
        "https://earth-search.aws.element84.com/v1/collections"
    );
    assert_eq!(
        parsed.capabilities.spatial,
        vec![SpatialOperation::Bbox, SpatialOperation::Intersects]
    );
    assert_eq!(parsed.capabilities.temporal, Some(true));
    assert_eq!(
        parsed.capabilities.sorting,
        Some(true),
        "earth search advertises item-search#sort"
    );
    assert_eq!(
        parsed.capabilities.attribute,
        Some(false),
        "earth search advertises no filter or cql2 conformance class"
    );
    assert_eq!(
        parsed.capabilities.crs,
        vec![geoquery_types::CrsDescriptor {
            code: "OGC:CRS84".to_owned(),
            axis_order: AxisOrder::LonLat
        }]
    );
}

#[test]
fn planetary_computer_is_recognised_and_declares_filter_support() {
    let parsed = parse_landing(
        "https://planetarycomputer.microsoft.com/api/stac/v1",
        &planetary_computer_landing(),
    )
    .expect("a conforming landing page parses");

    assert_eq!(
        parsed.search_url,
        "https://planetarycomputer.microsoft.com/api/stac/v1/search"
    );
    // No `data` link in this fixture, so the collections URL is the documented fallback.
    assert_eq!(
        parsed.collections_url,
        "https://planetarycomputer.microsoft.com/api/stac/v1/collections"
    );
    assert_eq!(
        parsed.capabilities.attribute,
        Some(true),
        "planetary computer advertises item-search#filter and cql2-json"
    );
}

#[test]
fn a_response_that_is_not_json_object_is_refused() {
    let error = parse_landing("https://example.test", &json!(["not", "an", "object"]))
        .expect_err("an array is not a landing page");
    assert!(
        error.contains("example.test"),
        "the error should name the service: {error}"
    );
}

#[test]
fn a_json_object_with_no_conformance_is_refused() {
    let error = parse_landing(
        "https://example.test",
        &json!({ "type": "Catalog", "title": "not actually a STAC API" }),
    )
    .expect_err("a landing page with no conformsTo is not confirmed STAC");
    assert!(
        error.contains("Catalog"),
        "the error should name what was found: {error}"
    );
}

#[test]
fn a_catalog_that_only_claims_an_unrelated_standard_is_refused() {
    let error = parse_landing(
        "https://example.test",
        &json!({
            "type": "Catalog",
            "conformsTo": ["http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/core"]
        }),
    )
    .expect_err("ogc features core alone is not the STAC API core");
    assert!(error.contains("example.test"));
}

#[test]
fn a_feature_collection_is_not_a_landing_page_either() {
    let error = parse_landing(
        "https://example.test",
        &json!({ "type": "FeatureCollection", "features": [] }),
    )
    .expect_err("a search response is not a landing page");
    assert!(error.contains("FeatureCollection"));
}

#[test]
fn collection_ids_reads_every_id_in_a_collections_response() {
    let body = json!({
        "collections": [
            { "id": "sentinel-2-l2a", "title": "Sentinel-2 Level 2A" },
            { "id": "landsat-c2-l2" }
        ],
        "links": []
    });
    assert_eq!(
        collection_ids(&body),
        vec!["sentinel-2-l2a".to_owned(), "landsat-c2-l2".to_owned()]
    );
}

#[test]
fn collection_ids_is_empty_rather_than_failing_on_an_unexpected_shape() {
    assert_eq!(
        collection_ids(&json!({ "type": "Catalog" })),
        Vec::<String>::new()
    );
    assert_eq!(collection_ids(&json!(null)), Vec::<String>::new());
}

#[test]
fn a_bare_collection_resource_is_also_recognised() {
    // Some deployments serve a single collection's description at the registered URL
    // rather than a multi-collection catalog; STAC API permits both as landing pages.
    let parsed = parse_landing(
        "https://example.test/collections/one",
        &json!({
            "type": "Collection",
            "conformsTo": ["https://api.stacspec.org/v1.0.0/core"],
            "links": []
        }),
    )
    .expect("a Collection landing page is accepted");
    assert_eq!(
        parsed.search_url,
        "https://example.test/collections/one/search"
    );
}

// ── conformance classes ──────────────────────────────────────────────────────────

#[test]
fn earth_search_declares_sorting_and_field_selection_but_no_filtering() {
    let parsed = parse_landing(
        "https://earth-search.aws.element84.com/v1",
        &earth_search_landing(),
    )
    .expect("a conforming landing page parses");

    let conformance = parsed.conformance;
    assert!(conformance.sort, "item-search#sort is advertised");
    assert!(conformance.fields, "item-search#fields is advertised");
    assert!(
        !conformance.filter,
        "the legacy item-search#query extension is not the Filter extension"
    );
    assert!(!conformance.pushes_basic_filter());
    assert!(!conformance.declares_nothing());
}

#[test]
fn planetary_computer_declares_cql2_json_filtering_and_nothing_else() {
    let parsed = parse_landing(
        "https://planetarycomputer.microsoft.com/api/stac/v1",
        &planetary_computer_landing(),
    )
    .expect("a conforming landing page parses");

    let conformance = parsed.conformance;
    assert!(
        conformance.filter,
        "the `v1.0.0-rc.2` spelling of the filter binding still counts"
    );
    assert!(conformance.cql2_json);
    assert!(conformance.basic_cql2);
    assert!(conformance.pushes_basic_filter());
    assert!(
        !conformance.advanced_comparison,
        "LIKE and IN live in a class this service did not declare"
    );
    assert!(!conformance.pushes_advanced_filter());
    assert!(!conformance.sort);
    assert!(!conformance.fields);
}

#[test]
fn a_service_declaring_only_the_core_declares_nothing_this_adapter_can_use() {
    let landing = json!({
        "type": "Catalog",
        "conformsTo": ["https://api.stacspec.org/v1.0.0/core"],
        "links": []
    });
    let parsed = parse_landing("https://example.org/stac", &landing).expect("core is enough");
    assert!(parsed.conformance.declares_nothing());
    assert_eq!(parsed.capabilities.attribute, Some(false));
    assert_eq!(parsed.capabilities.sorting, Some(false));
}

#[test]
fn conformance_is_read_back_off_a_registered_descriptor() {
    // The query path does not keep the parsed landing page: it re-reads `conformsTo`
    // from the descriptor's metadata, so these two must agree or a registered source
    // would be translated against capabilities it never had.
    let parsed = parse_landing(
        "https://planetarycomputer.microsoft.com/api/stac/v1",
        &planetary_computer_landing(),
    )
    .expect("a conforming landing page parses");

    let mut descriptor = geoquery_types::ServiceDescriptor::new(
        geoquery_types::ServiceType::Stac,
        "https://planetarycomputer.microsoft.com/api/stac/v1".to_owned(),
    );
    descriptor.metadata = parsed.metadata.clone();

    assert_eq!(
        geoquery_adapter_stac::StacConformance::for_service(&descriptor),
        parsed.conformance
    );
}

#[test]
fn a_descriptor_with_no_recorded_conformance_declares_nothing() {
    let descriptor = geoquery_types::ServiceDescriptor::new(
        geoquery_types::ServiceType::Stac,
        "https://example.org/stac".to_owned(),
    );
    assert!(
        geoquery_adapter_stac::StacConformance::for_service(&descriptor).declares_nothing(),
        "an unrecorded capability is not a capability"
    );
}
