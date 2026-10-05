//! Integration tests for the `geoquery-types` crate root.
//!
//! The crate root is a namespace: four type aliases and the re-exports that let a
//! dependent write `geoquery_types::GeoResult` rather than naming a module. What belongs
//! here is the test that builds something out of all three halves of the data model at
//! once; per-module behaviour lives in the file named after the module, and the list of
//! compose, and the list of names the crate root exports — which is a different question
//! from whether they compose, and the one that breaks dependents when it changes.

use chrono::{DateTime, Utc};
use pretty_assertions::assert_eq;
use serde_json::Value;

#[test]
fn foundation_types_are_available_from_the_public_crate_surface() {
    let mut resource = ResourceDescriptor::new("urn:geoquery:test-resource", ResourceType::Dataset);
    resource.extensions.insert(
        "native:id".to_owned(),
        Value::String("dataset-1".to_owned()),
    );

    let mut service = ServiceDescriptor::new(ServiceType::Stac, "https://example.test/stac");
    service.capabilities = Some(CapabilitySet {
        spatial: vec![SpatialOperation::Intersects, SpatialOperation::Bbox],
        temporal: Some(true),
        ..CapabilitySet::default()
    });
    resource.services.push(service);

    let timestamp: DateTime<Utc> = "2026-10-04T12:00:00Z".parse().expect("valid timestamp");
    let provenance = Provenance::new(
        "fixture-registry",
        "https://example.test/stac",
        ServiceType::Stac,
        JsonObject::new(),
        timestamp,
    );
    let result = GeoResult::new("item-1", geoquery_types::ResultType::Feature, provenance);

    assert_eq!(resource.r#type, ResourceType::Dataset);
    assert_eq!(resource.services[0].r#type, ServiceType::Stac);
    assert_eq!(result.provenance.protocol, ServiceType::Stac);
}

// The crate root re-exports every public name, so a dependent writes
// `geoquery_types::GeoResult` and never names a module. Moving a type between modules is
// therefore free and moving one *out of the root* breaks every caller — and in a diff of
// the file that was split, the two look identical. A glob would compile against any
// surface at all, so the point is to name them.
use geoquery_types::{
    Asset, AuthDescriptor, AuthType, AxisOrder, BoundingBox, CapabilitySet, CompareOp,
    ContextReference, CrsDescriptor, DimensionDescriptor, DimensionType, DistanceUnit,
    ExecutionMode, ExecutionOptions, ExecutionPolicy, ExtensionMap, FieldDescriptor, FieldType,
    FilterExpr, FilterValidationError, FilterValue, GeoQuery, GeoResult, IncludeOptions,
    JsonObject, License, Link, MAX_FILTER_DEPTH, PagingDescriptor, PagingStyle, Provenance,
    Provider, QueryScope, QueryValidationError, RelationshipType, ResourceDescriptor, ResourceRef,
    ResourceRelationship, ResourceType, ResultType, ScaleRange, SchemaDescriptor, Selection,
    ServiceDescriptor, ServiceRef, ServiceType, SortDirection, SortExpression, SourceMetadata,
    SpatialExtent, SpatialOperation, SpatialPredicate, TemporalExtent, TemporalOperation,
    TemporalPredicate,
};

/// Each name used in type position, so an import that silently resolves to something
/// else — a module, a re-exported trait — still fails here.
#[test]
fn every_name_still_resolves_from_the_crate_root() {
    fn assert_is_a_type<T>() {}

    assert_is_a_type::<Asset>();
    assert_is_a_type::<AuthDescriptor>();
    assert_is_a_type::<AuthType>();
    assert_is_a_type::<AxisOrder>();
    assert_is_a_type::<BoundingBox>();
    assert_is_a_type::<CapabilitySet>();
    assert_is_a_type::<CompareOp>();
    assert_is_a_type::<ContextReference>();
    assert_is_a_type::<CrsDescriptor>();
    assert_is_a_type::<DimensionDescriptor>();
    assert_is_a_type::<DimensionType>();
    assert_is_a_type::<DistanceUnit>();
    assert_is_a_type::<ExecutionMode>();
    assert_is_a_type::<ExecutionOptions>();
    assert_is_a_type::<ExecutionPolicy>();
    assert_is_a_type::<ExtensionMap>();
    assert_is_a_type::<FieldDescriptor>();
    assert_is_a_type::<FieldType>();
    assert_is_a_type::<FilterExpr>();
    assert_is_a_type::<FilterValidationError>();
    assert_is_a_type::<FilterValue>();
    assert_is_a_type::<GeoQuery>();
    assert_is_a_type::<GeoResult>();
    assert_is_a_type::<IncludeOptions>();
    assert_is_a_type::<JsonObject>();
    assert_is_a_type::<License>();
    assert_is_a_type::<Link>();
    assert_is_a_type::<PagingDescriptor>();
    assert_is_a_type::<PagingStyle>();
    assert_is_a_type::<Provenance>();
    assert_is_a_type::<Provider>();
    assert_is_a_type::<QueryScope>();
    assert_is_a_type::<QueryValidationError>();
    assert_is_a_type::<RelationshipType>();
    assert_is_a_type::<ResourceDescriptor>();
    assert_is_a_type::<ResourceRef>();
    assert_is_a_type::<ResourceRelationship>();
    assert_is_a_type::<ResourceType>();
    assert_is_a_type::<ResultType>();
    assert_is_a_type::<ScaleRange>();
    assert_is_a_type::<SchemaDescriptor>();
    assert_is_a_type::<Selection>();
    assert_is_a_type::<ServiceDescriptor>();
    assert_is_a_type::<ServiceRef>();
    assert_is_a_type::<ServiceType>();
    assert_is_a_type::<SortDirection>();
    assert_is_a_type::<SortExpression>();
    assert_is_a_type::<SourceMetadata>();
    assert_is_a_type::<SpatialExtent>();
    assert_is_a_type::<SpatialOperation>();
    assert_is_a_type::<SpatialPredicate>();
    assert_is_a_type::<TemporalExtent>();
    assert_is_a_type::<TemporalOperation>();
    assert_is_a_type::<TemporalPredicate>();

    // The one item that is a value rather than a type.
    let _: usize = MAX_FILTER_DEPTH;
}
