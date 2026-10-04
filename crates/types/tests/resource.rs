//! Integration tests for `geoquery_types::resource`.

use geoquery_types::{ResourceDescriptor, ServiceType};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

#[test]
fn resource_descriptors_round_trip_without_losing_extensions() {
    let descriptor_json: Value =
        serde_json::from_str(include_str!("fixtures/resource-descriptor.json"))
            .expect("the fixture is JSON");

    let descriptor: ResourceDescriptor = serde_json::from_value(descriptor_json.clone())
        .expect("representative descriptor deserializes");

    assert_eq!(descriptor.services[0].r#type, ServiceType::Stac);
    assert_eq!(
        descriptor.extensions["lineage"],
        json!({"process": "harmonized"})
    );
    assert_eq!(
        serde_json::to_value(&descriptor).expect("descriptor serializes"),
        descriptor_json
    );
}
