//! Mirrors `src/request.rs`: `GeoQuery` → the STAC Item Search request this adapter
//! actually sends, plus a finding for everything it does not.

use geoquery_adapter_stac::translate;
use geoquery_core::{Cause, QueryFeature, Support};
use geoquery_types::{
    FilterExpr, FilterValue, GeoQuery, QueryScope, Selection, SortDirection, SortExpression,
    SpatialOperation, SpatialPredicate, TemporalOperation, TemporalPredicate,
};
use pretty_assertions::assert_eq;

fn instant(text: &str) -> chrono::DateTime<chrono::Utc> {
    text.parse().expect("a test timestamp is valid")
}

#[test]
fn an_empty_query_translates_to_an_empty_request_with_no_findings() {
    let (request, findings) = translate(&GeoQuery::default());
    assert_eq!(request, geoquery_adapter_stac::StacSearchRequest::default());
    assert!(findings.is_empty(), "nothing was asked, nothing to report");
}

#[test]
fn a_bbox_predicate_is_pushed_unchanged() {
    let query = GeoQuery {
        spatial: Some(SpatialPredicate {
            op: SpatialOperation::Bbox,
            bbox: Some([14.1, 49.0, 24.2, 54.8]),
            geometry: None,
            distance: None,
            unit: None,
            crs: None,
        }),
        ..GeoQuery::default()
    };

    let (request, findings) = translate(&query);
    assert_eq!(request.bbox, Some([14.1, 49.0, 24.2, 54.8]));
    assert_eq!(
        findings,
        vec![geoquery_core::CapabilityFinding {
            feature: QueryFeature::Spatial(SpatialOperation::Bbox),
            support: Support::Pushed,
        }]
    );
}

#[test]
fn an_exact_geometry_predicate_is_refused_rather_than_approximated() {
    // This spike does not apply a geometry predicate locally, so claiming `Approximated`
    // (push a bbox, filter the exact shape afterwards) would promise a step that never
    // runs. `Refused` is the honest finding: nothing narrows the request for this feature.
    let query = GeoQuery {
        spatial: Some(SpatialPredicate {
            op: SpatialOperation::Intersects,
            bbox: None,
            geometry: Some(serde_json::json!({ "type": "Point", "coordinates": [1.0, 2.0] })),
            distance: None,
            unit: None,
            crs: None,
        }),
        ..GeoQuery::default()
    };

    let (request, findings) = translate(&query);
    assert_eq!(request.bbox, None, "no safe approximation is sent either");
    assert_eq!(
        findings,
        vec![geoquery_core::CapabilityFinding {
            feature: QueryFeature::Spatial(SpatialOperation::Intersects),
            support: Support::Refused {
                cause: Cause::Undeclared
            },
        }]
    );
}

#[test]
fn a_temporal_intersects_predicate_becomes_a_datetime_range() {
    let query = GeoQuery {
        temporal: Some(TemporalPredicate {
            op: TemporalOperation::Intersects,
            start: Some(instant("2024-06-01T00:00:00Z")),
            end: Some(instant("2024-08-31T23:59:59Z")),
        }),
        ..GeoQuery::default()
    };

    let (request, findings) = translate(&query);
    assert_eq!(
        request.datetime.as_deref(),
        Some("2024-06-01T00:00:00Z/2024-08-31T23:59:59Z")
    );
    assert_eq!(
        findings,
        vec![geoquery_core::CapabilityFinding {
            feature: QueryFeature::Temporal(TemporalOperation::Intersects),
            support: Support::Pushed,
        }]
    );
}

#[test]
fn an_open_ended_temporal_predicate_uses_the_stac_open_range_syntax() {
    let starts_only = GeoQuery {
        temporal: Some(TemporalPredicate {
            op: TemporalOperation::Intersects,
            start: Some(instant("2024-06-01T00:00:00Z")),
            end: None,
        }),
        ..GeoQuery::default()
    };
    let (request, _) = translate(&starts_only);
    assert_eq!(request.datetime.as_deref(), Some("2024-06-01T00:00:00Z/.."));

    let ends_only = GeoQuery {
        temporal: Some(TemporalPredicate {
            op: TemporalOperation::Intersects,
            start: None,
            end: Some(instant("2024-08-31T23:59:59Z")),
        }),
        ..GeoQuery::default()
    };
    let (request, _) = translate(&ends_only);
    assert_eq!(request.datetime.as_deref(), Some("../2024-08-31T23:59:59Z"));
}

#[test]
fn a_non_intersects_temporal_operation_sends_no_filter_and_is_reported() {
    let query = GeoQuery {
        temporal: Some(TemporalPredicate {
            op: TemporalOperation::Before,
            start: Some(instant("2024-06-01T00:00:00Z")),
            end: None,
        }),
        ..GeoQuery::default()
    };

    let (request, findings) = translate(&query);
    assert_eq!(request.datetime, None, "no filter is a safe superset");
    assert_eq!(
        findings,
        vec![geoquery_core::CapabilityFinding {
            feature: QueryFeature::Temporal(TemporalOperation::Before),
            support: Support::Refused {
                cause: Cause::Undeclared
            },
        }]
    );
}

#[test]
fn named_resources_in_scope_become_collections() {
    let query = GeoQuery {
        scope: Some(QueryScope {
            resources: Some(Selection::Named(vec![
                "sentinel-2-l2a".to_owned(),
                "landsat-c2-l2".to_owned(),
            ])),
            ..QueryScope::default()
        }),
        ..GeoQuery::default()
    };

    let (request, findings) = translate(&query);
    assert_eq!(
        request.collections,
        vec!["sentinel-2-l2a".to_owned(), "landsat-c2-l2".to_owned()]
    );
    assert!(
        findings.is_empty(),
        "collection scoping is not a capability negotiation"
    );
}

#[test]
fn selecting_everything_omits_the_collections_parameter() {
    let query = GeoQuery {
        scope: Some(QueryScope {
            resources: Some(Selection::All),
            ..QueryScope::default()
        }),
        ..GeoQuery::default()
    };
    let (request, _) = translate(&query);
    assert!(request.collections.is_empty());
}

#[test]
fn limit_alone_is_pushed_as_paging() {
    let query = GeoQuery {
        limit: Some(20),
        ..GeoQuery::default()
    };
    let (request, findings) = translate(&query);
    assert_eq!(request.limit, Some(20));
    assert_eq!(
        findings,
        vec![geoquery_core::CapabilityFinding {
            feature: QueryFeature::Paging,
            support: Support::Pushed,
        }]
    );
}

#[test]
fn a_nonzero_offset_is_refused_rather_than_silently_skipped() {
    let query = GeoQuery {
        limit: Some(10),
        offset: Some(20),
        ..GeoQuery::default()
    };
    let (request, findings) = translate(&query);
    assert_eq!(request.limit, Some(10), "limit is still pushed");
    assert!(
        findings
            .iter()
            .any(|finding| finding.feature == QueryFeature::Paging
                && finding.support
                    == Support::Refused {
                        cause: Cause::Undeclared
                    }),
        "a nonzero offset must be reported, not answered from page one: {findings:?}"
    );
}

#[test]
fn an_offset_of_zero_is_indistinguishable_from_no_offset() {
    let query = GeoQuery {
        limit: Some(10),
        offset: Some(0),
        ..GeoQuery::default()
    };
    let (_, findings) = translate(&query);
    assert_eq!(
        findings,
        vec![geoquery_core::CapabilityFinding {
            feature: QueryFeature::Paging,
            support: Support::Pushed,
        }]
    );
}

#[test]
fn filters_semantic_sort_and_field_selection_are_each_reported_once() {
    let query = GeoQuery {
        filters: Some(FilterExpr::Compare {
            field: "eo:cloud_cover".to_owned(),
            op: geoquery_types::CompareOp::Lt,
            value: FilterValue::Number(serde_json::Number::from(10)),
        }),
        semantic: Some("flood risk".to_owned()),
        sort: vec![SortExpression {
            field: "datetime".to_owned(),
            direction: SortDirection::Desc,
        }],
        fields: vec!["id".to_owned(), "geometry".to_owned()],
        ..GeoQuery::default()
    };

    let (_, findings) = translate(&query);

    assert_eq!(
        findings
            .iter()
            .map(|finding| &finding.feature)
            .collect::<Vec<_>>(),
        vec![
            &QueryFeature::AttributeFilter,
            &QueryFeature::Semantic,
            &QueryFeature::Sort,
            &QueryFeature::FieldSelection,
        ]
    );
    assert_eq!(
        findings[0].support,
        Support::Refused {
            cause: Cause::Undeclared
        }
    );
    assert_eq!(
        findings[3].support,
        Support::Local {
            cause: Cause::Undeclared
        },
        "field selection is genuinely applied locally, unlike the others"
    );
}

#[test]
fn to_json_object_round_trips_through_serde_json() {
    let (request, _) = translate(&GeoQuery {
        limit: Some(5),
        scope: Some(QueryScope {
            resources: Some(Selection::Named(vec!["sentinel-2-l2a".to_owned()])),
            ..QueryScope::default()
        }),
        ..GeoQuery::default()
    });
    let object = request.to_json_object();
    assert_eq!(object["limit"], 5);
    assert_eq!(object["collections"], serde_json::json!(["sentinel-2-l2a"]));
    assert!(
        !object.contains_key("bbox"),
        "absent fields are omitted, not null"
    );
}
