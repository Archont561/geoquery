//! Integration tests for `geoquery_types::macros`.
//!
//! The module is private and the macro is `pub(crate)`, so these exercise it the only way
//! a dependent can: through the enums it generates. All ten open string enums in the
//! crate expand from this one macro, so pinning the behaviour against a representative
//! few pins it for every one of them — including the `Custom` fallback, which is the
//! reason the macro exists rather than a plain `#[derive(Deserialize)]`.

use geoquery_types::{
    CapabilitySet, ResourceDescriptor, ResourceType, ServiceDescriptor, ServiceType,
    SpatialOperation,
};
use pretty_assertions::assert_eq;
use serde_json::json;

#[test]
fn extensible_enums_preserve_unrecognised_wire_values() {
    let resource: ResourceDescriptor = serde_json::from_value(json!({
        "id": "urn:geoquery:analysis-ready-cube",
        "type": "analysis-ready-data-cube"
    }))
    .expect("resource descriptor with a custom type deserializes");
    assert_eq!(
        resource.r#type,
        ResourceType::Custom("analysis-ready-data-cube".to_owned())
    );
    assert_eq!(
        serde_json::to_value(&resource).expect("resource descriptor serializes")["type"],
        "analysis-ready-data-cube"
    );

    let service: ServiceDescriptor = serde_json::from_value(json!({
        "type": "sentinel-hub",
        "url": "https://services.example.test/sentinel"
    }))
    .expect("service descriptor with a custom service type deserializes");
    assert_eq!(
        service.r#type,
        ServiceType::Custom("sentinel-hub".to_owned())
    );
    assert_eq!(
        serde_json::to_value(&service).expect("service descriptor serializes")["type"],
        "sentinel-hub"
    );

    let capabilities: CapabilitySet = serde_json::from_value(json!({
        "spatial": ["relate-mask", "bbox"]
    }))
    .expect("capability set with a custom spatial operation deserializes");
    assert_eq!(
        capabilities.spatial,
        vec![
            SpatialOperation::Custom("relate-mask".to_owned()),
            SpatialOperation::Bbox,
        ]
    );
    assert_eq!(
        serde_json::to_value(&capabilities).expect("capability set serializes"),
        json!({"spatial": ["relate-mask", "bbox"]})
    );
}

#[test]
fn open_enums_decode_the_same_from_borrowed_and_owned_json_strings() {
    // `from_str` hands the visitor a borrowed `&str` and `from_value` hands it an owned
    // `String`, and the two are backed by separate match statements over the same wire
    // table. A variant added to one and not the other would decode one way from a document
    // read off disk and another way from one built in memory, which is the kind of
    // divergence no caller would think to look for.
    let borrowed: ResourceDescriptor =
        serde_json::from_str(r#"{"id":"urn:geoquery:borrowed","type":"feature-collection"}"#)
            .expect("a descriptor parses from JSON text");
    let owned: ResourceDescriptor = serde_json::from_value(json!({
        "id": "urn:geoquery:borrowed",
        "type": "feature-collection"
    }))
    .expect("a descriptor parses from a JSON value");

    assert_eq!(borrowed.r#type, ResourceType::FeatureCollection);
    assert_eq!(borrowed, owned);

    let unrecognised: ResourceDescriptor =
        serde_json::from_str(r#"{"id":"urn:geoquery:cube","type":"analysis-ready-data-cube"}"#)
            .expect("an unrecognised type parses from JSON text");
    assert_eq!(
        unrecognised.r#type,
        ResourceType::Custom("analysis-ready-data-cube".to_owned())
    );
}

#[test]
fn open_enums_convert_from_both_owned_and_borrowed_strings() {
    assert_eq!(ResourceType::from("catalog"), ResourceType::Catalog);
    assert_eq!(ServiceType::from("wfs".to_owned()), ServiceType::Wfs);
    assert_eq!(
        ResourceType::from("star-chart".to_owned()),
        ResourceType::Custom("star-chart".to_owned())
    );
    assert_eq!(
        SpatialOperation::from("relate-mask"),
        SpatialOperation::Custom("relate-mask".to_owned())
    );
}

#[test]
fn open_enums_display_the_wire_value_they_would_serialize() {
    // `Display` and `AsRef` are what a caller reaches for when building a URL or a log
    // line. A value that printed differently from the string it serializes to would send
    // a reader grepping for a token that never appears in the document.
    assert_eq!(ServiceType::OgcFeatures.to_string(), "ogc-features");
    assert_eq!(ServiceType::OgcFeatures.as_ref(), "ogc-features");

    let custom = ServiceType::Custom("sentinel-hub".to_owned());
    assert_eq!(custom.to_string(), "sentinel-hub");
    assert_eq!(custom.as_ref(), "sentinel-hub");
}

#[test]
fn an_open_enum_names_itself_when_the_value_is_not_a_string() {
    let error =
        serde_json::from_value::<ResourceType>(json!(7)).expect_err("a number is not a type");
    assert!(
        error
            .to_string()
            .contains("a string containing a ResourceType value"),
        "the error should name the type it expected: {error}"
    );
}
