//! Integration tests for the public `geoquery-types` surface.

use chrono::{DateTime, Utc};
use geoquery_types::{
    CapabilitySet, GeoResult, JsonObject, Provenance, ResourceDescriptor, ResourceType,
    ServiceDescriptor, ServiceType, SpatialOperation,
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

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
