//! Integration tests for `geoquery_types::result`.

use geoquery_types::{GeoResult, ServiceType};
use pretty_assertions::assert_eq;
use serde_json::Value;

#[test]
fn geo_results_round_trip_without_losing_provenance_or_raw_payloads() {
    let result_json: Value = serde_json::from_str(include_str!("fixtures/geo-result.json"))
        .expect("the fixture is JSON");

    let result: GeoResult =
        serde_json::from_value(result_json.clone()).expect("representative result deserializes");

    assert_eq!(result.provenance.protocol, ServiceType::Stac);
    assert_eq!(result.provenance.query["datetime"], "2026-01-01/2026-01-31");
    assert_eq!(
        serde_json::to_value(&result).expect("result serializes"),
        result_json
    );
}
