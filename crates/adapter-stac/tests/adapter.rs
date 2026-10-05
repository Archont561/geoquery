//! Mirrors `src/adapter.rs`: `StacAdapter` end to end, against a mock STAC server.
//!
//! Every test here runs against `wiremock`, never against a live service — the pure
//! translation and normalization logic is proven without a network in `tests/landing.rs`,
//! `tests/request.rs` and `tests/response.rs`, and what is left to prove here is that the
//! HTTP glue in `src/adapter.rs` wires them up correctly: the right method, the right
//! path, the right error for the right failure.

use geoquery_adapter_stac::StacAdapter;
use geoquery_core::{AdapterError, Detection, Endpoint, ServiceAdapter};
use geoquery_types::{
    GeoQuery, QueryScope, Selection, ServiceDescriptor, ServiceType, SpatialOperation,
    SpatialPredicate,
};
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn earth_search_landing(base: &str) -> serde_json::Value {
    json!({
        "type": "Catalog",
        "conformsTo": [
            "https://api.stacspec.org/v1.0.0/core",
            "https://api.stacspec.org/v1.0.0/item-search"
        ],
        "links": [
            { "rel": "data", "href": format!("{base}/collections") },
            { "rel": "search", "href": format!("{base}/search"), "method": "POST" }
        ]
    })
}

fn collections_body() -> serde_json::Value {
    json!({ "collections": [{ "id": "sentinel-2-l2a" }, { "id": "landsat-c2-l2" }] })
}

#[tokio::test]
async fn describe_reads_the_landing_page_and_the_collections_it_points_at() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(earth_search_landing(&server.uri())))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/collections"))
        .respond_with(ResponseTemplate::new(200).set_body_json(collections_body()))
        .mount(&server)
        .await;

    let adapter = StacAdapter::new();
    let descriptor = adapter
        .describe(&Endpoint::new(server.uri()))
        .await
        .expect("a conforming mock service describes cleanly");

    assert_eq!(descriptor.r#type, ServiceType::Stac);
    assert_eq!(
        descriptor.collections,
        vec!["sentinel-2-l2a".to_owned(), "landsat-c2-l2".to_owned()]
    );
    let capabilities = descriptor.capabilities.expect("capabilities were set");
    assert_eq!(
        capabilities.spatial,
        vec![SpatialOperation::Bbox, SpatialOperation::Intersects]
    );
    assert_eq!(
        descriptor
            .metadata
            .get("searchUrl")
            .and_then(|v| v.as_str()),
        Some(format!("{}/search", server.uri()).as_str())
    );
}

#[tokio::test]
async fn describe_refuses_a_landing_page_that_is_not_stac() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "hello": "world" })))
        .mount(&server)
        .await;

    let adapter = StacAdapter::new();
    let error = adapter
        .describe(&Endpoint::new(server.uri()))
        .await
        .expect_err("a non-STAC JSON document is not a service descriptor");
    assert!(matches!(error, AdapterError::Malformed { .. }), "{error:?}");
}

#[tokio::test]
async fn describe_reports_a_service_error_as_a_service_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(503).set_body_string("service unavailable"))
        .mount(&server)
        .await;

    let adapter = StacAdapter::new();
    let error = adapter
        .describe(&Endpoint::new(server.uri()))
        .await
        .expect_err("a 503 is not a landing page");
    assert_eq!(
        error,
        AdapterError::Service {
            status: 503,
            body: "service unavailable".to_owned()
        }
    );
}

#[tokio::test]
async fn describe_reports_an_unroutable_endpoint_as_unreachable() {
    let adapter = StacAdapter::new();
    // Port 0 never accepts a connection; nothing is listening on it by construction.
    let error = adapter
        .describe(&Endpoint::new("http://127.0.0.1:0/"))
        .await
        .expect_err("nothing answers on port 0");
    assert!(
        matches!(error, AdapterError::Unreachable { .. }),
        "{error:?}"
    );
}

/// A descriptor shaped the way `describe` would have produced one, pointed at `server`.
fn service_descriptor(server: &MockServer) -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(ServiceType::Stac, server.uri());
    descriptor.id = Some("earth-search".to_owned());
    descriptor.metadata.insert(
        "searchUrl".to_owned(),
        json!(format!("{}/search", server.uri())),
    );
    descriptor
}

#[tokio::test]
async fn query_posts_the_translated_request_and_normalizes_the_answer() {
    let server = MockServer::start().await;
    let expected_body = json!({
        "bbox": [20.85, 52.10, 21.25, 52.35],
        "collections": ["sentinel-2-l2a"],
        "limit": 5
    });
    Mock::given(method("POST"))
        .and(path("/search"))
        .and(body_json(&expected_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "id": "item-1",
                "collection": "sentinel-2-l2a",
                "geometry": null,
                "properties": { "datetime": "2024-06-15T00:00:00Z" },
                "assets": {},
                "links": []
            }]
        })))
        .mount(&server)
        .await;

    let query = GeoQuery {
        spatial: Some(SpatialPredicate {
            op: SpatialOperation::Bbox,
            bbox: Some([20.85, 52.10, 21.25, 52.35]),
            geometry: None,
            distance: None,
            unit: None,
            crs: None,
        }),
        scope: Some(QueryScope {
            resources: Some(Selection::Named(vec!["sentinel-2-l2a".to_owned()])),
            ..QueryScope::default()
        }),
        limit: Some(5),
        ..GeoQuery::default()
    };

    let adapter = StacAdapter::new();
    let result = adapter
        .query(&service_descriptor(&server), &query)
        .await
        .expect("the mock server answers the expected request");

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].id, "item-1");
    assert_eq!(result.provenance.source, "earth-search");
    assert_eq!(result.provenance.service, server.uri());
    assert!(
        result.degradations.is_empty(),
        "bbox, collections and limit are all fully pushed: {:?}",
        result.degradations
    );
}

#[tokio::test]
async fn query_reports_unsupported_features_as_degradations_without_failing() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "FeatureCollection",
            "features": []
        })))
        .mount(&server)
        .await;

    let query = GeoQuery {
        semantic: Some("flood risk".to_owned()),
        ..GeoQuery::default()
    };

    let adapter = StacAdapter::new();
    let result = adapter
        .query(&service_descriptor(&server), &query)
        .await
        .expect("an unsupported feature degrades rather than fails the query");
    assert_eq!(result.degradations.len(), 1);
    assert_eq!(
        result.degradations[0].feature,
        geoquery_core::QueryFeature::Semantic
    );
}

#[tokio::test]
async fn query_turns_a_non_success_search_response_into_a_service_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(400).set_body_string("bad bbox"))
        .mount(&server)
        .await;

    let adapter = StacAdapter::new();
    let error = adapter
        .query(&service_descriptor(&server), &GeoQuery::default())
        .await
        .expect_err("a 400 from the service is not a result set");
    assert_eq!(
        error,
        AdapterError::Service {
            status: 400,
            body: "bad bbox".to_owned()
        }
    );
}

#[test]
fn detect_recognises_known_stac_hosts_without_a_network_call() {
    let adapter = StacAdapter::new();
    assert!(matches!(
        adapter.detect(&Endpoint::new("https://earth-search.aws.element84.com/v1")),
        Detection::Match {
            service_type: ServiceType::Stac,
            ..
        }
    ));
    assert!(matches!(
        adapter.detect(&Endpoint::new("https://example.test/not-related-at-all")),
        Detection::NoMatch
    ));
    assert!(matches!(
        adapter.detect(&Endpoint::new("https://example.test/my-stac-thing")),
        Detection::Uncertain { .. }
    ));
}
