//! Mirrors `src/request.rs`: `GeoQuery` → the STAC Item Search request this adapter
//! actually sends, plus a finding for everything it does not.
//!
//! Every case names the conformance it is translating against, because that is now half
//! of what decides the answer. The three services below are not invented: they are the
//! `conformsTo` lists of Earth Search and Planetary Computer as captured in
//! `tests/landing.rs`, plus the degenerate case of a STAC API that declares core and
//! nothing else. Between them they cover the matrix that matters — one service sorts and
//! projects but cannot filter, the other filters but cannot sort or project, and neither
//! does everything.

use geoquery_adapter_stac::{
    StacConformance, StacFields, StacSortBy, StacSortDirection, translate,
};
use geoquery_core::{CapabilityFinding, Cause, QueryFeature, Support};
use geoquery_types::{
    CompareOp, FilterExpr, FilterValue, GeoQuery, QueryScope, Selection, SortDirection,
    SortExpression, SpatialOperation, SpatialPredicate, TemporalOperation, TemporalPredicate,
};
use pretty_assertions::assert_eq;

fn instant(text: &str) -> chrono::DateTime<chrono::Utc> {
    text.parse().expect("a test timestamp is valid")
}

fn classes(list: &[&str]) -> StacConformance {
    StacConformance::from_classes(
        &list
            .iter()
            .map(|class| (*class).to_owned())
            .collect::<Vec<_>>(),
    )
}

/// A STAC API declaring the core and nothing else — every extension undeclared.
fn core_only() -> StacConformance {
    classes(&["https://api.stacspec.org/v1.0.0/core"])
}

/// Earth Search: the sort and fields bindings, and no filter conformance at all.
fn earth_search() -> StacConformance {
    classes(&[
        "https://api.stacspec.org/v1.0.0/core",
        "https://api.stacspec.org/v1.0.0/item-search",
        "https://api.stacspec.org/v1.0.0/item-search#fields",
        "https://api.stacspec.org/v1.0.0/item-search#sort",
        "https://api.stacspec.org/v1.0.0/item-search#query",
    ])
}

/// Planetary Computer: CQL2 JSON filtering at the basic level, with neither the advanced
/// comparison operators nor sorting nor field selection.
fn planetary_computer() -> StacConformance {
    classes(&[
        "http://www.opengis.net/spec/cql2/1.0/conf/basic-cql2",
        "http://www.opengis.net/spec/cql2/1.0/conf/cql2-json",
        "http://www.opengis.net/spec/ogcapi-features-3/1.0/conf/filter",
        "https://api.stacspec.org/v1.0.0-rc.2/item-search#filter",
        "https://api.stacspec.org/v1.0.0/core",
        "https://api.stacspec.org/v1.0.0/item-search",
    ])
}

/// A service declaring everything this adapter knows how to use.
fn fully_capable() -> StacConformance {
    classes(&[
        "https://api.stacspec.org/v1.0.0/core",
        "https://api.stacspec.org/v1.0.0/item-search",
        "https://api.stacspec.org/v1.0.0/item-search#filter",
        "https://api.stacspec.org/v1.0.0/item-search#sort",
        "https://api.stacspec.org/v1.0.0/item-search#fields",
        "http://www.opengis.net/spec/cql2/1.0/conf/cql2-json",
        "http://www.opengis.net/spec/cql2/1.0/conf/basic-cql2",
        "http://www.opengis.net/spec/cql2/1.0/conf/advanced-comparison-operators",
    ])
}

fn cloud_cover_below_ten() -> FilterExpr {
    FilterExpr::Compare {
        field: "eo:cloud_cover".to_owned(),
        op: CompareOp::Lt,
        value: FilterValue::Number(10.into()),
    }
}

fn finding(feature: QueryFeature, support: Support) -> CapabilityFinding {
    CapabilityFinding { feature, support }
}

#[test]
fn an_empty_query_translates_to_an_empty_request_with_no_findings() {
    let (request, findings) = translate(&GeoQuery::default(), core_only());
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

    let (request, findings) = translate(&query, core_only());
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

    let (request, findings) = translate(&query, core_only());
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

    let (request, findings) = translate(&query, core_only());
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
    let (request, _) = translate(&starts_only, core_only());
    assert_eq!(request.datetime.as_deref(), Some("2024-06-01T00:00:00Z/.."));

    let ends_only = GeoQuery {
        temporal: Some(TemporalPredicate {
            op: TemporalOperation::Intersects,
            start: None,
            end: Some(instant("2024-08-31T23:59:59Z")),
        }),
        ..GeoQuery::default()
    };
    let (request, _) = translate(&ends_only, core_only());
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

    let (request, findings) = translate(&query, core_only());
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

    let (request, findings) = translate(&query, core_only());
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
    let (request, _) = translate(&query, core_only());
    assert!(request.collections.is_empty());
}

#[test]
fn limit_alone_is_pushed_as_paging() {
    let query = GeoQuery {
        limit: Some(20),
        ..GeoQuery::default()
    };
    let (request, findings) = translate(&query, core_only());
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
fn a_nonzero_offset_widens_the_page_and_is_reported_as_local_work() {
    // STAC has no skip count, so the window is cut locally out of what comes back. That
    // costs the transfer of every skipped item, which is exactly what `Support::Local`
    // means — and it is why the request asks for `offset + limit` rather than `limit`.
    let query = GeoQuery {
        limit: Some(10),
        offset: Some(20),
        ..GeoQuery::default()
    };
    let (request, findings) = translate(&query, core_only());
    assert_eq!(request.limit, Some(30), "the skipped items must arrive too");
    assert_eq!(
        findings,
        vec![finding(
            QueryFeature::Paging,
            Support::Local {
                cause: Cause::Undeclared
            }
        )],
        "a nonzero offset is answered locally, and says so"
    );
}

#[test]
fn an_offset_of_zero_is_indistinguishable_from_no_offset() {
    let query = GeoQuery {
        limit: Some(10),
        offset: Some(0),
        ..GeoQuery::default()
    };
    let (_, findings) = translate(&query, core_only());
    assert_eq!(
        findings,
        vec![geoquery_core::CapabilityFinding {
            feature: QueryFeature::Paging,
            support: Support::Pushed,
        }]
    );
}

#[test]
fn every_feature_a_query_asks_for_is_reported_exactly_once() {
    let query = GeoQuery {
        filters: Some(cloud_cover_below_ten()),
        semantic: Some("flood risk".to_owned()),
        sort: vec![SortExpression {
            field: "datetime".to_owned(),
            direction: SortDirection::Desc,
        }],
        fields: vec!["id".to_owned(), "geometry".to_owned()],
        ..GeoQuery::default()
    };

    let (_, findings) = translate(&query, core_only());

    assert_eq!(
        findings
            .iter()
            .map(|finding| &finding.feature)
            .collect::<Vec<_>>(),
        vec![
            &QueryFeature::AttributeFilter,
            &QueryFeature::Sort,
            &QueryFeature::FieldSelection,
            &QueryFeature::Semantic,
        ]
    );
}

// ── attribute filtering ──────────────────────────────────────────────────────────

#[test]
fn a_filter_is_compiled_to_cql2_json_for_a_service_that_declared_it() {
    let query = GeoQuery {
        filters: Some(cloud_cover_below_ten()),
        ..GeoQuery::default()
    };

    let (request, findings) = translate(&query, planetary_computer());

    assert_eq!(
        request.filter,
        Some(serde_json::json!({
            "op": "<",
            "args": [{ "property": "eo:cloud_cover" }, 10]
        }))
    );
    assert_eq!(
        request.filter_lang.as_deref(),
        Some("cql2-json"),
        "the encoding is stated rather than left to the server's default"
    );
    assert_eq!(
        findings,
        vec![finding(QueryFeature::AttributeFilter, Support::Pushed)]
    );
}

#[test]
fn a_filter_is_refused_by_a_service_that_declared_no_filter_conformance() {
    // Earth Search advertises the legacy `item-search#query` extension and no CQL2 class.
    // Sending a `filter` member anyway is the worst available outcome: a server that
    // ignores a parameter it does not implement answers a wider question than the one
    // asked, with nothing in the response to show that it widened.
    let query = GeoQuery {
        filters: Some(cloud_cover_below_ten()),
        ..GeoQuery::default()
    };

    let (request, findings) = translate(&query, earth_search());

    assert_eq!(request.filter, None);
    assert_eq!(request.filter_lang, None);
    assert_eq!(
        findings,
        vec![finding(
            QueryFeature::AttributeFilter,
            Support::Refused {
                cause: Cause::Declined
            }
        )],
        "a service that published its conformance and omitted filtering has declined it"
    );
}

#[test]
fn a_service_that_declared_nothing_at_all_gets_undeclared_rather_than_declined() {
    let query = GeoQuery {
        filters: Some(cloud_cover_below_ten()),
        ..GeoQuery::default()
    };
    let (_, findings) = translate(&query, core_only());
    assert_eq!(
        findings,
        vec![finding(
            QueryFeature::AttributeFilter,
            Support::Refused {
                cause: Cause::Undeclared
            }
        )]
    );
}

#[test]
fn cql2_text_alone_is_not_enough_to_send_a_json_filter() {
    let text_only = classes(&[
        "https://api.stacspec.org/v1.0.0/core",
        "https://api.stacspec.org/v1.0.0/item-search#filter",
        "http://www.opengis.net/spec/cql2/1.0/conf/cql2-text",
        "http://www.opengis.net/spec/cql2/1.0/conf/basic-cql2",
    ]);
    let query = GeoQuery {
        filters: Some(cloud_cover_below_ten()),
        ..GeoQuery::default()
    };

    let (request, findings) = translate(&query, text_only);

    assert_eq!(
        request.filter, None,
        "this adapter emits CQL2 JSON, and a cql2-text service may reject it"
    );
    assert_eq!(
        findings,
        vec![finding(
            QueryFeature::AttributeFilter,
            Support::Refused {
                cause: Cause::Declined
            }
        )]
    );
}

#[test]
fn a_like_is_refused_by_a_service_that_declared_only_basic_cql2() {
    // Planetary Computer declares `basic-cql2` and not `advanced-comparison-operators`,
    // which is where CQL2 keeps LIKE and IN.
    let query = GeoQuery {
        filters: Some(FilterExpr::Like {
            field: "mission".to_owned(),
            pattern: "sentinel%".to_owned(),
        }),
        ..GeoQuery::default()
    };

    let (request, findings) = translate(&query, planetary_computer());

    assert_eq!(request.filter, None);
    assert_eq!(
        findings,
        vec![finding(
            QueryFeature::AttributeFilter,
            Support::Refused {
                cause: Cause::Declined
            }
        )]
    );
}

#[test]
fn one_unsupported_operator_refuses_the_whole_tree_rather_than_half_of_it() {
    // Sending only the branch the service understands would change what the query means:
    // dropping a branch of an `or` narrows the answer and dropping one of an `and`
    // widens it. `AttributeFilter` is one feature in `geoquery-core` for this reason.
    let query = GeoQuery {
        filters: Some(FilterExpr::And(vec![
            cloud_cover_below_ten(),
            FilterExpr::In {
                field: "platform".to_owned(),
                values: vec![FilterValue::String("sentinel-2a".to_owned())],
            },
        ])),
        ..GeoQuery::default()
    };

    let (request, findings) = translate(&query, planetary_computer());

    assert_eq!(request.filter, None, "not even the supported half is sent");
    assert_eq!(
        findings,
        vec![finding(
            QueryFeature::AttributeFilter,
            Support::Refused {
                cause: Cause::Declined
            }
        )]
    );
}

#[test]
fn an_advanced_operator_is_pushed_to_a_service_that_declared_the_class() {
    let query = GeoQuery {
        filters: Some(FilterExpr::In {
            field: "platform".to_owned(),
            values: vec![
                FilterValue::String("sentinel-2a".to_owned()),
                FilterValue::String("sentinel-2b".to_owned()),
            ],
        }),
        ..GeoQuery::default()
    };

    let (request, findings) = translate(&query, fully_capable());

    assert_eq!(
        request.filter,
        Some(serde_json::json!({
            "op": "in",
            "args": [{ "property": "platform" }, ["sentinel-2a", "sentinel-2b"]]
        }))
    );
    assert_eq!(
        findings,
        vec![finding(QueryFeature::AttributeFilter, Support::Pushed)]
    );
}

// ── sorting ──────────────────────────────────────────────────────────────────────

#[test]
fn sort_keys_are_pushed_in_the_order_the_query_named_them() {
    let query = GeoQuery {
        sort: vec![
            SortExpression {
                field: "properties.eo:cloud_cover".to_owned(),
                direction: SortDirection::Asc,
            },
            SortExpression {
                field: "id".to_owned(),
                direction: SortDirection::Desc,
            },
        ],
        ..GeoQuery::default()
    };

    let (request, findings) = translate(&query, earth_search());

    assert_eq!(
        request.sortby,
        vec![
            StacSortBy {
                field: "properties.eo:cloud_cover".to_owned(),
                direction: StacSortDirection::Asc,
            },
            StacSortBy {
                field: "id".to_owned(),
                direction: StacSortDirection::Desc,
            },
        ],
        "field names travel as written: the Sort extension lets each service decide \
         whether item properties carry a `properties.` prefix"
    );
    assert_eq!(findings, vec![finding(QueryFeature::Sort, Support::Pushed)]);
}

#[test]
fn sorting_is_refused_rather_than_faked_locally() {
    // A local sort would order the items that happened to come back, which is a sorted
    // list of the wrong items: "the ten least cloudy scenes" is not "ten scenes, sorted
    // by cloud cover".
    let query = GeoQuery {
        sort: vec![SortExpression {
            field: "datetime".to_owned(),
            direction: SortDirection::Desc,
        }],
        limit: Some(10),
        ..GeoQuery::default()
    };

    let (request, findings) = translate(&query, planetary_computer());

    assert!(request.sortby.is_empty());
    assert!(findings.contains(&finding(
        QueryFeature::Sort,
        Support::Refused {
            cause: Cause::Declined
        }
    )));
}

// ── field selection ──────────────────────────────────────────────────────────────

#[test]
fn a_fields_hint_is_sent_with_the_members_a_result_cannot_survive_without() {
    let query = GeoQuery {
        fields: vec!["eo:cloud_cover".to_owned()],
        ..GeoQuery::default()
    };

    let (request, findings) = translate(&query, earth_search());

    let fields = request
        .fields
        .expect("the hint is sent to a declaring service");
    assert_eq!(
        fields,
        StacFields {
            include: [
                "type",
                "stac_version",
                "id",
                "collection",
                "geometry",
                "bbox",
                "links",
                "assets",
                "properties.datetime",
                "eo:cloud_cover",
            ]
            .iter()
            .map(|name| (*name).to_owned())
            .collect(),
        },
        "an `include` list is a replacement, so an item's identity has to be in it"
    );
    assert_eq!(
        findings,
        vec![finding(QueryFeature::FieldSelection, Support::Pushed)]
    );
}

#[test]
fn field_selection_without_the_extension_is_local_work_rather_than_a_refusal() {
    let query = GeoQuery {
        fields: vec!["eo:cloud_cover".to_owned()],
        ..GeoQuery::default()
    };

    let (request, findings) = translate(&query, planetary_computer());

    assert_eq!(request.fields, None, "an undeclared hint is not sent");
    assert_eq!(
        findings,
        vec![finding(
            QueryFeature::FieldSelection,
            Support::Local {
                cause: Cause::Declined
            }
        )],
        "the whole item arrives and this adapter trims it"
    );
}

#[test]
fn to_json_object_round_trips_through_serde_json() {
    let (request, _) = translate(
        &GeoQuery {
            limit: Some(5),
            scope: Some(QueryScope {
                resources: Some(Selection::Named(vec!["sentinel-2-l2a".to_owned()])),
                ..QueryScope::default()
            }),
            ..GeoQuery::default()
        },
        core_only(),
    );
    let object = request.to_json_object();
    assert_eq!(object["limit"], 5);
    assert_eq!(object["collections"], serde_json::json!(["sentinel-2-l2a"]));
    assert!(
        !object.contains_key("bbox"),
        "absent fields are omitted, not null"
    );
}
