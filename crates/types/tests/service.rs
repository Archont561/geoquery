//! Integration tests for `geoquery_types::service`.

use geoquery_types::{
    AuthDescriptor, AuthType, CapabilitySet, JsonObject, ServiceDescriptor, ServiceType,
    SpatialOperation,
};
use pretty_assertions::assert_eq;
use serde_json::Value;

/// The distinction the whole capability model rests on. Every optional flag is
/// `Option<bool>` so that a service which never mentioned a feature is distinguishable
/// from one that declared it unsupported — an engine that conflates them will push a
/// predicate at a service that may quietly ignore rather than refuse it.
#[test]
fn a_capability_that_went_unmentioned_is_not_a_capability_that_was_refused() {
    let silent = CapabilitySet::default();
    let refused = CapabilitySet {
        temporal: Some(false),
        ..CapabilitySet::default()
    };

    assert_eq!(silent.temporal, None, "default must not invent an answer");
    assert_eq!(refused.temporal, Some(false));
    assert_ne!(silent.temporal, refused.temporal);

    // And the difference has to survive the wire, or it is not a difference at all.
    let silent_json = serde_json::to_value(&silent).expect("a capability set serializes");
    let refused_json = serde_json::to_value(&refused).expect("a capability set serializes");
    assert_eq!(
        silent_json.get("temporal"),
        None,
        "an unmentioned capability must stay absent rather than serialize as false"
    );
    assert_eq!(refused_json.get("temporal"), Some(&Value::Bool(false)));
}

/// `spatial` is a list rather than a flag, so "said nothing" is the empty list and there
/// is no third state to lose.
#[test]
fn the_spatial_operation_list_round_trips_every_declared_operation() {
    let declared = CapabilitySet {
        spatial: vec![
            SpatialOperation::Intersects,
            SpatialOperation::Bbox,
            SpatialOperation::Custom("h3:within".to_owned()),
        ],
        ..CapabilitySet::default()
    };

    let text = serde_json::to_string(&declared).expect("a capability set serializes");
    let decoded: CapabilitySet = serde_json::from_str(&text).expect("and parses back");

    assert_eq!(decoded.spatial, declared.spatial);
    assert_eq!(
        decoded.spatial[2],
        SpatialOperation::Custom("h3:within".to_owned()),
        "an operation this build has never heard of has to survive the round trip"
    );
}

#[test]
fn service_descriptors_round_trip_without_losing_authentication_or_metadata() {
    let mut service = ServiceDescriptor::new(ServiceType::Stac, "https://example.test/stac");
    service.authentication = Some(AuthDescriptor {
        r#type: AuthType::ApiKey,
        profile: None,
        scopes: Vec::new(),
        metadata: JsonObject::new(),
    });
    service
        .metadata
        .insert("title".to_owned(), Value::String("Example".to_owned()));
    service.collections.push("sentinel-2".to_owned());

    let text = serde_json::to_string(&service).expect("a service descriptor serializes");
    let decoded: ServiceDescriptor = serde_json::from_str(&text).expect("and parses back");

    assert_eq!(decoded.r#type, ServiceType::Stac);
    assert_eq!(decoded.url, service.url);
    assert_eq!(decoded.collections, vec!["sentinel-2".to_owned()]);
    assert_eq!(
        decoded
            .authentication
            .expect("authentication survives")
            .r#type,
        AuthType::ApiKey
    );
    assert_eq!(
        decoded.metadata.get("title"),
        Some(&Value::String("Example".to_owned()))
    );
}
