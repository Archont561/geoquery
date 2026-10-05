//! Mirrors `src/response.rs`: STAC `Item`/`ItemCollection` → `GeoResult`, with
//! provenance attached and the raw payload preserved.
//!
//! The item fixture is trimmed from a real Earth Search `/search` response (collection
//! `sentinel-2-l2a`, fetched 2026-10-05), not invented, so the assertions are about a
//! shape a real provider actually sends rather than one this adapter would find easy.

use chrono::{DateTime, Utc};
use geoquery_adapter_stac::{NormalizationError, normalize_response};
use geoquery_types::JsonObject;
use pretty_assertions::assert_eq;
use serde_json::json;

fn timestamp() -> DateTime<Utc> {
    "2026-10-05T12:00:00Z".parse().expect("a valid timestamp")
}

fn earth_search_item() -> serde_json::Value {
    json!({
        "type": "Feature",
        "stac_version": "1.0.0",
        "id": "S2A_34UDC_20240630_0_L2A",
        "collection": "sentinel-2-l2a",
        "bbox": [19.531, 51.354, 21.143, 52.350],
        "geometry": {
            "type": "Polygon",
            "coordinates": [[[19.531, 52.341], [19.563, 51.354], [21.140, 51.363], [19.531, 52.341]]]
        },
        "properties": {
            "datetime": "2024-06-30T09:56:15.124000Z",
            "platform": "sentinel-2a",
            "eo:cloud_cover": 20.616_859
        },
        "assets": {
            "blue": {
                "href": "https://sentinel-cogs.s3.us-west-2.amazonaws.com/.../B02.tif",
                "type": "image/tiff; application=geotiff; profile=cloud-optimized",
                "title": "Blue (band 2) - 10m",
                "roles": ["data", "reflectance"],
                "gsd": 10
            },
            "thumbnail": {
                "href": "https://earth-search.aws.element84.com/v1/.../thumbnail",
                "type": "image/png",
                "roles": ["thumbnail"]
            }
        },
        "links": [
            { "rel": "self", "type": "application/geo+json", "href": "https://earth-search.aws.element84.com/v1/collections/sentinel-2-l2a/items/S2A_34UDC_20240630_0_L2A" },
            { "rel": "collection", "type": "application/json", "href": "https://earth-search.aws.element84.com/v1/collections/sentinel-2-l2a" }
        ]
    })
}

fn search_response(features: &[serde_json::Value]) -> serde_json::Value {
    json!({
        "type": "FeatureCollection",
        "stac_version": "1.0.0",
        "context": { "limit": 10, "matched": features.len(), "returned": features.len() },
        "features": features,
    })
}

#[test]
fn a_feature_collection_normalizes_into_geo_results() {
    let body = search_response(&[earth_search_item()]);
    let normalized = normalize_response(
        &body,
        "earth-search",
        "https://earth-search.aws.element84.com/v1",
        &JsonObject::new(),
        timestamp(),
        Some(123),
    )
    .expect("a conforming feature collection normalizes");

    assert_eq!(normalized.skipped, 0);
    assert_eq!(normalized.results.len(), 1);

    let result = &normalized.results[0];
    assert_eq!(result.id, "S2A_34UDC_20240630_0_L2A");
    assert_eq!(
        result.resource.as_ref().map(|r| r.id.as_str()),
        Some("sentinel-2-l2a")
    );
    assert_eq!(result.bbox, Some([19.531, 51.354, 21.143, 52.350]));
    assert!(result.geometry.is_some());
    assert_eq!(
        result.properties.get("platform").and_then(|v| v.as_str()),
        Some("sentinel-2a")
    );
    assert_eq!(result.assets.len(), 2);
    assert_eq!(result.links.len(), 2);
    assert_eq!(result.raw.as_ref(), Some(&earth_search_item()));

    assert_eq!(result.provenance.source, "earth-search");
    assert_eq!(
        result.provenance.service,
        "https://earth-search.aws.element84.com/v1"
    );
    assert_eq!(
        result.provenance.collection.as_deref(),
        Some("sentinel-2-l2a")
    );
    assert_eq!(result.provenance.duration_ms, Some(123));

    let temporal = result.temporal.as_ref().expect("a datetime property");
    let expected: DateTime<Utc> = "2024-06-30T09:56:15.124000Z".parse().unwrap();
    assert_eq!(temporal.start, Some(expected));
    assert_eq!(temporal.end, Some(expected));
}

#[test]
fn assets_carry_their_own_key_href_type_title_and_roles() {
    let body = search_response(&[earth_search_item()]);
    let normalized = normalize_response(
        &body,
        "earth-search",
        "https://earth-search.aws.element84.com/v1",
        &JsonObject::new(),
        timestamp(),
        None,
    )
    .unwrap();

    let blue = normalized.results[0]
        .assets
        .iter()
        .find(|asset| asset.metadata.get("key").and_then(|v| v.as_str()) == Some("blue"))
        .expect("the blue asset survives normalization");
    assert!(blue.href.ends_with("B02.tif"));
    assert_eq!(
        blue.media_type.as_deref(),
        Some("image/tiff; application=geotiff; profile=cloud-optimized")
    );
    assert_eq!(blue.title.as_deref(), Some("Blue (band 2) - 10m"));
    assert_eq!(
        blue.roles,
        vec!["data".to_owned(), "reflectance".to_owned()]
    );
    // Fields this adapter's `Asset` has no slot for travel in `metadata` rather than
    // being dropped.
    assert_eq!(blue.metadata.get("gsd"), Some(&json!(10)));
}

#[test]
fn a_bare_feature_is_accepted_like_a_one_item_collection() {
    let normalized = normalize_response(
        &earth_search_item(),
        "earth-search",
        "https://earth-search.aws.element84.com/v1",
        &JsonObject::new(),
        timestamp(),
        None,
    )
    .expect("a bare Feature is a legal GET /search response for one match");
    assert_eq!(normalized.results.len(), 1);
}

#[test]
fn a_feature_with_no_id_is_skipped_and_counted_rather_than_failing_the_batch() {
    let mut item = earth_search_item();
    item.as_object_mut().unwrap().remove("id");
    let body = search_response(&[item, earth_search_item()]);

    let normalized = normalize_response(
        &body,
        "earth-search",
        "https://earth-search.aws.element84.com/v1",
        &JsonObject::new(),
        timestamp(),
        None,
    )
    .expect("one bad feature does not fail the whole response");
    assert_eq!(normalized.skipped, 1);
    assert_eq!(normalized.results.len(), 1);
}

#[test]
fn an_empty_feature_collection_normalizes_to_no_results() {
    let normalized = normalize_response(
        &search_response(&[]),
        "earth-search",
        "https://earth-search.aws.element84.com/v1",
        &JsonObject::new(),
        timestamp(),
        None,
    )
    .unwrap();
    assert!(normalized.results.is_empty());
    assert_eq!(normalized.skipped, 0);
}

#[test]
fn a_response_that_is_not_json_object_is_refused() {
    let error = normalize_response(
        &json!("not an object"),
        "earth-search",
        "https://earth-search.aws.element84.com/v1",
        &JsonObject::new(),
        timestamp(),
        None,
    )
    .expect_err("a bare string is not a search response");
    assert_eq!(error, NormalizationError::NotAnObject);
}

#[test]
fn a_json_object_of_the_wrong_type_is_refused() {
    let error = normalize_response(
        &json!({ "type": "Catalog" }),
        "earth-search",
        "https://earth-search.aws.element84.com/v1",
        &JsonObject::new(),
        timestamp(),
        None,
    )
    .expect_err("a landing page is not a search response");
    assert_eq!(
        error,
        NormalizationError::UnexpectedType {
            found: "Catalog".to_owned()
        }
    );
}

#[test]
fn start_and_end_datetime_are_preferred_over_a_single_instant() {
    let mut item = earth_search_item();
    item["properties"]["start_datetime"] = json!("2024-06-01T00:00:00Z");
    item["properties"]["end_datetime"] = json!("2024-08-31T23:59:59Z");

    let normalized = normalize_response(
        &search_response(&[item]),
        "earth-search",
        "https://earth-search.aws.element84.com/v1",
        &JsonObject::new(),
        timestamp(),
        None,
    )
    .unwrap();

    let temporal = normalized.results[0].temporal.as_ref().unwrap();
    assert_eq!(
        temporal.start,
        Some("2024-06-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap())
    );
    assert_eq!(
        temporal.end,
        Some("2024-08-31T23:59:59Z".parse::<DateTime<Utc>>().unwrap())
    );
}

#[test]
fn a_next_link_is_surfaced_opaquely() {
    let mut body = search_response(&[earth_search_item()]);
    body["links"] = json!([
        { "rel": "next", "href": "https://earth-search.aws.element84.com/v1/search?page=2" }
    ]);

    let normalized = normalize_response(
        &body,
        "earth-search",
        "https://earth-search.aws.element84.com/v1",
        &JsonObject::new(),
        timestamp(),
        None,
    )
    .unwrap();
    assert_eq!(
        normalized.next_page.as_deref(),
        Some("https://earth-search.aws.element84.com/v1/search?page=2")
    );
}

#[test]
fn no_next_link_means_no_next_page() {
    let normalized = normalize_response(
        &search_response(&[earth_search_item()]),
        "earth-search",
        "https://earth-search.aws.element84.com/v1",
        &JsonObject::new(),
        timestamp(),
        None,
    )
    .unwrap();
    assert_eq!(normalized.next_page, None);
}
