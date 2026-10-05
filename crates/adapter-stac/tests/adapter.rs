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
async fn describe_guesses_the_paths_a_landing_page_never_named() {
    let server = MockServer::start().await;
    // The one shape `links` being absent has to survive: a conforming landing page that
    // names nothing, so `parse_landing` falls back to `{base}/collections`. Real minimal
    // deployments do this, and a mock registered only on `/` would pass whether the
    // fallback was right or wrong — the assertion that matters is that `/collections` is
    // the path that gets asked for.
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "Catalog",
            "conformsTo": ["https://api.stacspec.org/v1.0.0/core"]
        })))
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
        .expect("a conforming landing page that names no links still describes");

    assert_eq!(
        descriptor.collections,
        vec!["sentinel-2-l2a".to_owned(), "landsat-c2-l2".to_owned()],
        "the collections came from the guessed /collections path, so it was reached"
    );
    assert_eq!(
        descriptor
            .metadata
            .get("collectionsUrl")
            .and_then(|v| v.as_str()),
        Some(format!("{}/collections", server.uri()).as_str())
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

// ── conformance, pagination and the local window ─────────────────────────────────

/// A descriptor shaped the way `describe` would have produced one for a service that
/// published `classes`, which is where `StacConformance::for_service` reads them back.
fn service_descriptor_declaring(server: &MockServer, classes: &[&str]) -> ServiceDescriptor {
    let mut descriptor = service_descriptor(server);
    descriptor.metadata.insert(
        "conformsTo".to_owned(),
        json!(classes.iter().collect::<Vec<_>>()),
    );
    descriptor
}

/// One STAC item, enough of one to survive normalization.
fn item(id: &str, cloud_cover: u8) -> serde_json::Value {
    json!({
        "type": "Feature",
        "id": id,
        "collection": "sentinel-2-l2a",
        "geometry": null,
        "properties": {
            "datetime": "2024-06-15T00:00:00Z",
            "eo:cloud_cover": cloud_cover,
            "platform": "sentinel-2a"
        },
        "assets": {},
        "links": []
    })
}

#[tokio::test]
async fn query_follows_the_services_own_next_link_until_the_limit_is_filled() {
    let server = MockServer::start().await;
    // Page one answers the request this adapter built, and advertises page two as a POST
    // with a token to merge into that same request — the shape every pgstac-backed
    // service sends.
    Mock::given(method("POST"))
        .and(path("/search"))
        .and(body_json(json!({ "limit": 3 })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "FeatureCollection",
            "features": [item("item-1", 1), item("item-2", 2)],
            "links": [{
                "rel": "next",
                "href": format!("{}/search", server.uri()),
                "method": "POST",
                "body": { "token": "next:item-2" },
                "merge": true
            }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .and(body_json(json!({ "limit": 3, "token": "next:item-2" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "FeatureCollection",
            "features": [item("item-3", 3), item("item-4", 4)]
        })))
        .mount(&server)
        .await;

    let query = GeoQuery {
        limit: Some(3),
        ..GeoQuery::default()
    };
    let result = StacAdapter::new()
        .query(&service_descriptor(&server), &query)
        .await
        .expect("two pages answer the query");

    assert_eq!(
        result
            .results
            .iter()
            .map(|result| result.id.as_str())
            .collect::<Vec<_>>(),
        vec!["item-1", "item-2", "item-3"],
        "the second page completes the limit, and the surplus is dropped"
    );
}

#[tokio::test]
async fn query_without_a_limit_fetches_one_page_and_hands_back_the_next_link() {
    // An unlimited query is a request for what the service returns by default, not an
    // instruction to walk a catalogue. The `next` link is reported so a caller can decide
    // for themselves.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "FeatureCollection",
            "features": [item("item-1", 1)],
            "links": [{
                "rel": "next",
                "href": format!("{}/search?page=2", server.uri()),
                "method": "GET"
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = StacAdapter::new()
        .query(&service_descriptor(&server), &GeoQuery::default())
        .await
        .expect("one page answers an unlimited query");

    assert_eq!(result.results.len(), 1);
    assert_eq!(
        result.next_page,
        Some(format!("{}/search?page=2", server.uri()))
    );
}

#[tokio::test]
async fn query_cuts_the_offset_window_out_of_what_arrived() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        // `offset + limit`: the skipped items have to arrive before they can be skipped.
        .and(body_json(json!({ "limit": 4 })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "FeatureCollection",
            "features": [
                item("item-1", 1), item("item-2", 2), item("item-3", 3), item("item-4", 4)
            ]
        })))
        .mount(&server)
        .await;

    let query = GeoQuery {
        limit: Some(2),
        offset: Some(2),
        ..GeoQuery::default()
    };
    let result = StacAdapter::new()
        .query(&service_descriptor(&server), &query)
        .await
        .expect("the window is cut from one page");

    assert_eq!(
        result
            .results
            .iter()
            .map(|result| result.id.as_str())
            .collect::<Vec<_>>(),
        vec!["item-3", "item-4"]
    );
    assert_eq!(
        result.degradations.len(),
        1,
        "the offset was served locally, and the cost of that is reported: {:?}",
        result.degradations
    );
}

#[tokio::test]
async fn query_sends_a_cql2_filter_to_a_service_that_declared_cql2_json() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .and(body_json(json!({
            "filter-lang": "cql2-json",
            "filter": { "op": "<", "args": [{ "property": "eo:cloud_cover" }, 10] }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "FeatureCollection",
            "features": [item("item-1", 1)]
        })))
        .mount(&server)
        .await;

    let query = GeoQuery {
        filters: Some(geoquery_types::FilterExpr::Compare {
            field: "eo:cloud_cover".to_owned(),
            op: geoquery_types::CompareOp::Lt,
            value: geoquery_types::FilterValue::Number(10.into()),
        }),
        ..GeoQuery::default()
    };
    let descriptor = service_descriptor_declaring(
        &server,
        &[
            "https://api.stacspec.org/v1.0.0/core",
            "https://api.stacspec.org/v1.0.0-rc.2/item-search#filter",
            "http://www.opengis.net/spec/cql2/1.0/conf/cql2-json",
            "http://www.opengis.net/spec/cql2/1.0/conf/basic-cql2",
        ],
    );

    let result = StacAdapter::new()
        .query(&descriptor, &query)
        .await
        .expect("the mock server answers the filtered request");

    assert_eq!(result.results.len(), 1);
    assert!(
        result.degradations.is_empty(),
        "a pushed filter is not a degradation: {:?}",
        result.degradations
    );
    assert!(
        result.provenance.query.contains_key("filter"),
        "provenance shows the filter that was actually sent"
    );
}

#[tokio::test]
async fn query_trims_properties_locally_when_the_service_cannot_project() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        // No `fields` member: this service never declared the extension, so the hint is
        // not sent — and the trim happens here instead.
        .and(body_json(json!({})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "FeatureCollection",
            "features": [item("item-1", 7)]
        })))
        .mount(&server)
        .await;

    let query = GeoQuery {
        fields: vec!["eo:cloud_cover".to_owned()],
        ..GeoQuery::default()
    };
    let result = StacAdapter::new()
        .query(&service_descriptor(&server), &query)
        .await
        .expect("the mock server answers");

    assert_eq!(
        result.results[0].properties.keys().collect::<Vec<_>>(),
        vec!["eo:cloud_cover"]
    );
    assert_eq!(
        result.results[0].id, "item-1",
        "projection never costs a result its identity"
    );
}
