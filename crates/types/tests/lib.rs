//! Integration tests for the public `geoquery-types` surface.

use chrono::{DateTime, Utc};
use geoquery_types::{
    CapabilitySet, GeoResult, JsonObject, Provenance, ResourceDescriptor, ResourceType,
    ServiceDescriptor, ServiceType, SourceMetadata, SpatialOperation, TemporalExtent,
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

fn instant(text: &str) -> DateTime<Utc> {
    text.parse().expect("a test timestamp is valid")
}

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
#[allow(clippy::too_many_lines)]
fn resource_descriptors_round_trip_without_losing_extensions() {
    let descriptor_json = json!({
        "id": "https://example.test/resources/poland-flood-risk",
        "type": "dataset",
        "title": "Poland flood risk",
        "description": "A fixture dataset used to exercise the protocol-independent model.",
        "spatial": {
            "bbox": [14.1, 49.0, 24.2, 54.8],
            "geometry": {
                "type": "Polygon",
                "coordinates": [[
                    [14.1, 49.0],
                    [24.2, 49.0],
                    [24.2, 54.8],
                    [14.1, 54.8],
                    [14.1, 49.0]
                ]]
            },
            "crs": "EPSG:4326"
        },
        "temporal": {
            "start": "2026-01-01T00:00:00Z",
            "end": "2026-01-31T23:59:59Z"
        },
        "themes": ["hazards", "hydrology"],
        "keywords": ["flood", "risk", "poland"],
        "provider": {
            "name": "Example Hydrology Institute",
            "url": "https://example.test",
            "roles": ["producer", "host"]
        },
        "license": {
            "id": "CC-BY-4.0",
            "name": "Creative Commons Attribution 4.0 International",
            "url": "https://creativecommons.org/licenses/by/4.0/"
        },
        "context": [{
            "href": "docs/flood-risk.md",
            "rel": "described-by",
            "title": "Flood risk methodology",
            "mediaType": "text/markdown"
        }],
        "relationships": [{
            "relation": "derived-from",
            "target": "urn:geoquery:dem-v1",
            "title": "Elevation model"
        }],
        "services": [{
            "id": "planetary-computer",
            "type": "stac",
            "url": "https://planetarycomputer.microsoft.com/api/stac/v1",
            "collections": ["sentinel-2-l2a"],
            "capabilities": {
                "spatial": ["intersects", "bbox"],
                "temporal": true,
                "attribute": true,
                "fullText": false,
                "semantic": false,
                "sorting": true,
                "pagination": true,
                "bbox": true,
                "geometryFilter": true,
                "crs": [{
                    "code": "EPSG:4326",
                    "axisOrder": "lon-lat"
                }],
                "bboxes": [[14.1, 49.0, 24.2, 54.8]],
                "formats": ["application/geo+json"],
                "dimensions": [{
                    "name": "time",
                    "type": "time",
                    "values": ["2026-01-01/2026-01-31"]
                }],
                "paging": {
                    "style": "cursor",
                    "defaultPageSize": 100,
                    "maxPageSize": 1000
                },
                "scaleRange": [1000.0, 100_000.0],
                "extensions": {
                    "stac:conformance": ["https://api.stacspec.org/v1.0.0/item-search"]
                }
            },
            "authentication": {
                "type": "api-key",
                "profile": "planetary-computer",
                "scopes": ["items:read"],
                "metadata": {"header": "x-api-key"}
            },
            "schema": {
                "fields": [{
                    "name": "eo:cloud_cover",
                    "type": "number",
                    "nullable": false,
                    "description": "Cloud cover percentage",
                    "values": [0, 100],
                    "extensions": {"unit": "percent"}
                }],
                "extensions": {"filterDialect": "cql2-json"}
            },
            "metadata": {
                "conformsTo": ["https://api.stacspec.org/v1.0.0/core"]
            }
        }],
        "source": {
            "id": "fixture-snapshot",
            "service": "https://planetarycomputer.microsoft.com/api/stac/v1",
            "protocol": "stac",
            "fetchedAt": "2026-10-04T00:00:00Z",
            "metadata": {"etag": "abc123"}
        },
        "extensions": {
            "stac_extensions": ["https://stac-extensions.github.io/eo/v1.1.0/schema.json"],
            "lineage": {"process": "harmonized"}
        }
    });

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

#[test]
fn geo_results_round_trip_without_losing_provenance_or_raw_payloads() {
    let result_json = json!({
        "id": "sentinel-2-l2a:item-1",
        "resource": {
            "id": "https://example.test/resources/poland-flood-risk",
            "title": "Poland flood risk"
        },
        "service": {
            "id": "planetary-computer",
            "url": "https://planetarycomputer.microsoft.com/api/stac/v1",
            "type": "stac"
        },
        "type": "feature",
        "title": "Sentinel-2 scene",
        "description": "A normalized feature result.",
        "geometry": {
            "type": "Point",
            "coordinates": [21.0122, 52.2297]
        },
        "bbox": [21.0, 52.2, 21.1, 52.3],
        "temporal": {
            "start": "2026-01-05T10:00:00Z",
            "end": "2026-01-05T10:10:00Z"
        },
        "properties": {
            "eo:cloud_cover": 7.5,
            "platform": "sentinel-2a"
        },
        "assets": [{
            "href": "https://example.test/assets/B04.tif",
            "mediaType": "image/tiff; application=geotiff",
            "title": "Band 4",
            "roles": ["data"],
            "metadata": {"band": "red"}
        }],
        "links": [{
            "href": "https://example.test/items/item-1",
            "rel": "self",
            "mediaType": "application/geo+json",
            "title": "Native item"
        }],
        "relevance": 0.92,
        "provenance": {
            "source": "planetary-computer",
            "service": "https://planetarycomputer.microsoft.com/api/stac/v1",
            "protocol": "stac",
            "collection": "sentinel-2-l2a",
            "query": {
                "bbox": [14.1, 49.0, 24.2, 54.8],
                "datetime": "2026-01-01/2026-01-31",
                "filter": {
                    "op": "<",
                    "args": [{"property": "eo:cloud_cover"}, 10]
                }
            },
            "timestamp": "2026-10-04T12:00:00Z",
            "duration_ms": 234
        },
        "raw": {
            "type": "Feature",
            "id": "native-item-1",
            "collection": "sentinel-2-l2a"
        }
    });

    let result: GeoResult =
        serde_json::from_value(result_json.clone()).expect("representative result deserializes");

    assert_eq!(result.provenance.protocol, ServiceType::Stac);
    assert_eq!(result.provenance.query["datetime"], "2026-01-01/2026-01-31");
    assert_eq!(
        serde_json::to_value(&result).expect("result serializes"),
        result_json
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

#[test]
fn every_instant_on_the_wire_accepts_a_bare_date() {
    // A date is what a hand-written `resource.yaml` or a fixture carries, and the query
    // AST already reads one. A descriptor that refused the same string would mean the
    // dialect depended on which struct the field happened to sit in.
    let extent: TemporalExtent = serde_json::from_value(json!({ "start": "2026-01-01" }))
        .expect("an extent takes a bare date");
    assert_eq!(extent.start, Some(instant("2026-01-01T00:00:00Z")));

    let source: SourceMetadata = serde_json::from_value(json!({ "fetchedAt": "2026-01-01" }))
        .expect("a fetch time takes a bare date");
    assert_eq!(source.fetched_at, Some(instant("2026-01-01T00:00:00Z")));

    let provenance: Provenance = serde_json::from_value(json!({
        "source": "fixture",
        "service": "https://example.test/stac",
        "protocol": "stac",
        "query": {},
        "timestamp": "2026-01-01"
    }))
    .expect("a provenance timestamp takes a bare date");
    assert_eq!(provenance.timestamp, instant("2026-01-01T00:00:00Z"));
}

#[test]
fn sub_second_precision_survives_a_round_trip() {
    // Normalising to one output format must not round the value on the way through: a
    // fetch time recorded to the millisecond is a different instant from the second it
    // falls in, and two results ordered by it would silently tie.
    let source: SourceMetadata =
        serde_json::from_value(json!({ "fetchedAt": "2026-01-05T10:00:00.123Z" }))
            .expect("a millisecond instant parses");

    assert_eq!(
        serde_json::to_value(&source).expect("it serializes"),
        json!({ "fetchedAt": "2026-01-05T10:00:00.123Z" })
    );
}

#[test]
fn an_instant_that_is_neither_form_is_refused_the_same_way_everywhere() {
    // One dialect means one complaint. A caller that mistyped a date should not have to
    // learn which struct it was in to find out what the field would have accepted.
    for document in [
        json!({ "start": "the first of January" }),
        json!({ "end": "01/01/2026" }),
    ] {
        let error =
            serde_json::from_value::<TemporalExtent>(document.clone()).expect_err("not an instant");
        assert!(
            error.to_string().contains("RFC 3339"),
            "{document} produced {error}"
        );
    }
}
