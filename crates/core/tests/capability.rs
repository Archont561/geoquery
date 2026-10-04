//! Mirrors `src/capability.rs`: what a service can be asked to do, and what it cannot.
//!
//! The rule the corpus states twice and this file exists to enforce is that the planner
//! must never silently pretend an unsupported operation is supported. Every test here is a
//! way of getting that wrong: reading an undeclared capability as a present one, dropping a
//! predicate a service cannot take, or claiming an exact answer from an approximate one.

use geoquery_core::{CapabilityReport, Cause, QueryFeature, Support};
use geoquery_types::{
    CapabilitySet, CompareOp, FieldDescriptor, FieldType, FilterExpr, GeoQuery, SchemaDescriptor,
    ServiceDescriptor, ServiceType, SortExpression, SpatialOperation, SpatialPredicate,
    TemporalOperation, TemporalPredicate,
};
use pretty_assertions::assert_eq;
use serde_json::json;

fn service(capabilities: Option<CapabilitySet>) -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(ServiceType::Stac, "https://example.org/stac");
    descriptor.capabilities = capabilities;
    descriptor
}

fn queryable(name: &str, r#type: FieldType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.to_owned(),
        r#type,
        nullable: None,
        description: None,
        values: Vec::new(),
        extensions: serde_json::Map::new(),
    }
}

/// Everything a well-equipped STAC service advertises.
fn generous() -> CapabilitySet {
    CapabilitySet {
        spatial: vec![SpatialOperation::Intersects, SpatialOperation::Bbox],
        temporal: Some(true),
        attribute: Some(true),
        semantic: Some(true),
        sorting: Some(true),
        pagination: Some(true),
        ..CapabilitySet::default()
    }
}

fn warsaw() -> SpatialPredicate {
    SpatialPredicate {
        op: SpatialOperation::Intersects,
        bbox: None,
        geometry: Some(json!({ "type": "Point", "coordinates": [21.0, 52.2] })),
        distance: None,
        unit: None,
        crs: None,
    }
}

fn cloud_cover_filter() -> FilterExpr {
    FilterExpr::Compare {
        field: "cloud_cover".to_owned(),
        op: CompareOp::Lt,
        value: 10_i64.into(),
    }
}

#[test]
fn a_query_that_asks_for_nothing_reports_nothing() {
    // The report lists what the query wants, not what the service has. An empty query is
    // the legitimate "everything the registry knows" request, and a report full of
    // findings about predicates nobody wrote would be noise a planner has to filter.
    let report = CapabilityReport::for_query(&GeoQuery::default(), &service(Some(generous())));

    assert_eq!(report.findings(), &[]);
    assert!(report.is_empty());
    assert!(
        report.fully_pushed(),
        "nothing asked for is nothing refused"
    );
}

#[test]
fn a_service_that_advertises_what_the_query_needs_takes_all_of_it() {
    let query = GeoQuery {
        spatial: Some(warsaw()),
        temporal: Some(TemporalPredicate {
            op: TemporalOperation::During,
            start: Some("2025-06-01T00:00:00Z".parse().expect("a valid instant")),
            end: None,
        }),
        filters: Some(cloud_cover_filter()),
        semantic: Some("flood risk".to_owned()),
        sort: vec![SortExpression {
            field: "datetime".to_owned(),
            direction: geoquery_types::SortDirection::Desc,
        }],
        limit: Some(20),
        ..GeoQuery::default()
    };

    let report = CapabilityReport::for_query(&query, &service(Some(generous())));

    assert!(
        report.fully_pushed(),
        "every feature should reach the service: {:?}",
        report.not_pushed().collect::<Vec<_>>()
    );
    assert_eq!(report.findings().len(), 6);
    assert_eq!(report.not_pushed().count(), 0);
}

#[test]
fn a_service_that_declared_nothing_is_trusted_with_nothing() {
    // A descriptor with no capabilities at all is the common case before discovery has
    // run. Reading that silence as consent is the exact failure the corpus forbids: the
    // service would receive a predicate it may ignore rather than refuse, and a narrow
    // query would come back as a wide answer with nothing to show it had widened.
    let query = GeoQuery {
        spatial: Some(warsaw()),
        temporal: Some(TemporalPredicate {
            op: TemporalOperation::During,
            start: Some("2025-06-01T00:00:00Z".parse().expect("a valid instant")),
            end: None,
        }),
        ..GeoQuery::default()
    };

    let report = CapabilityReport::for_query(&query, &service(None));

    assert!(!report.fully_pushed());
    for finding in report.findings() {
        assert!(
            matches!(
                finding.support,
                Support::Local {
                    cause: Cause::Undeclared
                }
            ),
            "{finding:?} should be undeclared rather than declined or pushed"
        );
    }
}

#[test]
fn a_declined_capability_is_not_the_same_as_an_undeclared_one() {
    // Both end in the engine doing the work, so the support is the same and the cause is
    // not. One is a service that answered the question, the other is a service that was
    // never asked, and only the second is worth re-running discovery over.
    let query = GeoQuery {
        temporal: Some(TemporalPredicate {
            op: TemporalOperation::During,
            start: Some("2025-06-01T00:00:00Z".parse().expect("a valid instant")),
            end: None,
        }),
        ..GeoQuery::default()
    };

    let declined = CapabilityReport::for_query(
        &query,
        &service(Some(CapabilitySet {
            temporal: Some(false),
            ..CapabilitySet::default()
        })),
    );
    let undeclared = CapabilityReport::for_query(&query, &service(Some(CapabilitySet::default())));

    assert_eq!(
        declined.findings()[0].support,
        Support::Local {
            cause: Cause::Declined
        }
    );
    assert_eq!(
        undeclared.findings()[0].support,
        Support::Local {
            cause: Cause::Undeclared
        }
    );
}

#[test]
fn an_exact_geometry_predicate_degrades_to_the_box_that_contains_it() {
    // The worked example in `.knowledge/query/planner.md`: no exact intersects, but bbox
    // filtering is there, so push the box and refine locally. The box returns a superset,
    // which is the only reason this is safe — an approximation that could drop a result
    // would be a wrong answer rather than a cheap one.
    let report = CapabilityReport::for_query(
        &GeoQuery {
            spatial: Some(warsaw()),
            ..GeoQuery::default()
        },
        &service(Some(CapabilitySet {
            spatial: vec![SpatialOperation::Bbox],
            ..CapabilitySet::default()
        })),
    );

    assert_eq!(
        report.findings(),
        &[geoquery_core::CapabilityFinding {
            feature: QueryFeature::Spatial(SpatialOperation::Intersects),
            support: Support::Approximated {
                instead: QueryFeature::Spatial(SpatialOperation::Bbox),
                cause: Cause::Declined,
            },
        }]
    );
    assert!(!report.fully_pushed());
}

#[test]
fn a_spatial_operation_the_service_never_listed_is_the_engines_job() {
    // No bbox either, so there is nothing weaker to push. The predicate is honoured
    // against the geometry on each result, which is why this is Local and not Refused.
    let report = CapabilityReport::for_query(
        &GeoQuery {
            spatial: Some(warsaw()),
            ..GeoQuery::default()
        },
        &service(Some(CapabilitySet {
            spatial: vec![SpatialOperation::Nearest],
            ..CapabilitySet::default()
        })),
    );

    assert_eq!(
        report.findings()[0].support,
        Support::Local {
            cause: Cause::Declined
        },
        "a service that listed its operations and omitted this one has declined it"
    );
}

#[test]
fn semantic_search_falls_back_to_full_text_and_then_is_refused() {
    // Three services, one query. Semantic ranking is the only feature here the engine
    // cannot do after the fact in Phase 1, because there is no local index to rank
    // against, so the last case is a refusal rather than local work.
    let query = GeoQuery {
        semantic: Some("flood risk".to_owned()),
        ..GeoQuery::default()
    };

    let native = CapabilityReport::for_query(
        &query,
        &service(Some(CapabilitySet {
            semantic: Some(true),
            ..CapabilitySet::default()
        })),
    );
    assert_eq!(native.findings()[0].support, Support::Pushed);

    let approximate = CapabilityReport::for_query(
        &query,
        &service(Some(CapabilitySet {
            semantic: Some(false),
            full_text: Some(true),
            ..CapabilitySet::default()
        })),
    );
    assert_eq!(
        approximate.findings()[0].support,
        Support::Approximated {
            instead: QueryFeature::FullText,
            cause: Cause::Declined,
        }
    );

    let refused = CapabilityReport::for_query(&query, &service(Some(CapabilitySet::default())));
    assert_eq!(
        refused.findings()[0].support,
        Support::Refused {
            cause: Cause::Undeclared
        }
    );
    assert_eq!(refused.refused().count(), 1);
}

#[test]
fn a_filter_naming_a_field_the_service_cannot_query_is_not_pushed() {
    // The service supports attribute filtering and still cannot answer this one, which is
    // why the capability flag alone is not enough to decide. The cause names the field so
    // the message can too.
    let report = CapabilityReport::for_query(
        &GeoQuery {
            filters: Some(cloud_cover_filter()),
            ..GeoQuery::default()
        },
        &service_with_schema(vec![queryable("platform", FieldType::String)]),
    );

    assert_eq!(
        report.findings()[0].support,
        Support::Local {
            cause: Cause::FieldNotQueryable {
                field: "cloud_cover".to_owned()
            }
        }
    );
}

fn service_with_schema(fields: Vec<FieldDescriptor>) -> ServiceDescriptor {
    let mut descriptor = service(Some(CapabilitySet {
        attribute: Some(true),
        ..CapabilitySet::default()
    }));
    descriptor.schema = Some(SchemaDescriptor {
        fields,
        extensions: serde_json::Map::new(),
    });
    descriptor
}

#[test]
fn a_filter_is_pushed_when_every_field_it_names_is_queryable() {
    let report = CapabilityReport::for_query(
        &GeoQuery {
            filters: Some(FilterExpr::And(vec![
                cloud_cover_filter(),
                FilterExpr::Compare {
                    field: "platform".to_owned(),
                    op: CompareOp::Eq,
                    value: "sentinel-2".into(),
                },
            ])),
            ..GeoQuery::default()
        },
        &service_with_schema(vec![
            queryable("cloud_cover", FieldType::Number),
            queryable("platform", FieldType::String),
        ]),
    );

    assert_eq!(report.findings()[0].support, Support::Pushed);
    assert!(report.fully_pushed());
}

#[test]
fn a_filter_is_taken_on_trust_when_the_service_published_no_schema() {
    // No queryables to check against is not evidence of absence. The service said it
    // filters, so the filter goes, and a service that then rejects the field reports a
    // real error rather than a guess this crate made on its behalf.
    let report = CapabilityReport::for_query(
        &GeoQuery {
            filters: Some(cloud_cover_filter()),
            ..GeoQuery::default()
        },
        &service(Some(CapabilitySet {
            attribute: Some(true),
            ..CapabilitySet::default()
        })),
    );

    assert_eq!(report.findings()[0].support, Support::Pushed);
}

#[test]
fn every_field_a_nested_filter_names_is_checked_not_just_the_first() {
    // The walk has to reach every leaf: a tree whose first branch is queryable and whose
    // third is not must not be pushed, and the finding has to name the third.
    let report = CapabilityReport::for_query(
        &GeoQuery {
            filters: Some(FilterExpr::Or(vec![
                cloud_cover_filter(),
                FilterExpr::Not(Box::new(FilterExpr::In {
                    field: "platform".to_owned(),
                    values: vec!["sentinel-2".into()],
                })),
                FilterExpr::Like {
                    field: "title".to_owned(),
                    pattern: "S2%".to_owned(),
                },
            ])),
            ..GeoQuery::default()
        },
        &service_with_schema(vec![
            queryable("cloud_cover", FieldType::Number),
            queryable("platform", FieldType::String),
        ]),
    );

    assert_eq!(
        report.findings()[0].support,
        Support::Local {
            cause: Cause::FieldNotQueryable {
                field: "title".to_owned()
            }
        }
    );
}

#[test]
fn sorting_and_paging_fall_back_to_the_engine() {
    let query = GeoQuery {
        sort: vec![SortExpression {
            field: "datetime".to_owned(),
            direction: geoquery_types::SortDirection::Asc,
        }],
        limit: Some(20),
        offset: Some(40),
        ..GeoQuery::default()
    };

    let report = CapabilityReport::for_query(&query, &service(Some(CapabilitySet::default())));

    // Two findings, not three: one limit and one offset are one paging feature.
    assert_eq!(
        report
            .findings()
            .iter()
            .map(|finding| finding.feature.clone())
            .collect::<Vec<_>>(),
        vec![QueryFeature::Sort, QueryFeature::Paging]
    );
    assert!(report.findings().iter().all(|finding| matches!(
        finding.support,
        Support::Local {
            cause: Cause::Undeclared
        }
    )));
}

#[test]
fn field_selection_is_always_the_engines_job() {
    // Nothing in the capability model describes projection, so there is nothing to claim.
    // Dropping fields from a normalized result is always correct, so it is always Local.
    let report = CapabilityReport::for_query(
        &GeoQuery {
            fields: vec!["id".to_owned(), "datetime".to_owned()],
            ..GeoQuery::default()
        },
        &service(Some(generous())),
    );

    assert_eq!(report.findings()[0].feature, QueryFeature::FieldSelection);
    assert_eq!(
        report.findings()[0].support,
        Support::Local {
            cause: Cause::Undeclared
        }
    );
}

#[test]
fn a_report_can_be_read_as_prose_without_inventing_wording() {
    // The planner shows this to a person. Every finding has to say what was asked for and
    // what became of it, from the typed value and not from a message built here.
    let report = CapabilityReport::for_query(
        &GeoQuery {
            spatial: Some(warsaw()),
            semantic: Some("flood risk".to_owned()),
            ..GeoQuery::default()
        },
        &service(Some(CapabilitySet {
            spatial: vec![SpatialOperation::Bbox],
            ..CapabilitySet::default()
        })),
    );

    let lines: Vec<String> = report
        .findings()
        .iter()
        .map(std::string::ToString::to_string)
        .collect();

    assert_eq!(
        lines,
        vec![
            "spatial `intersects` is approximated by spatial `bbox`: the service declared \
             it does not support it",
            "semantic ranking is refused: the service declared nothing about it",
        ]
    );
}

#[test]
fn the_report_distinguishes_what_was_refused_from_what_merely_moved() {
    let report = CapabilityReport::for_query(
        &GeoQuery {
            semantic: Some("flood risk".to_owned()),
            sort: vec![SortExpression {
                field: "datetime".to_owned(),
                direction: geoquery_types::SortDirection::Asc,
            }],
            ..GeoQuery::default()
        },
        &service(Some(CapabilitySet::default())),
    );

    assert_eq!(report.refused().count(), 1);
    assert_eq!(report.not_pushed().count(), 2);
    assert!(!report.fully_pushed());
    assert_eq!(
        report.refused().next().map(|finding| &finding.feature),
        Some(&QueryFeature::Semantic)
    );
}

#[test]
fn every_feature_and_every_outcome_has_wording() {
    // These lines reach a user through a CLI, an HTTP body and a Python warning. The
    // wording lives on the type so all three say the same thing, which only holds if
    // every combination actually has wording — hence the table rather than a sample.
    let cases = [
        (
            QueryFeature::Spatial(SpatialOperation::Intersects),
            Support::Pushed,
            "spatial `intersects` is pushed to the service",
        ),
        (
            QueryFeature::Temporal(TemporalOperation::During),
            Support::Local {
                cause: Cause::Declined,
            },
            "temporal `during` is applied locally: the service declared it does not support it",
        ),
        (
            QueryFeature::AttributeFilter,
            Support::Local {
                cause: Cause::FieldNotQueryable {
                    field: "cloud_cover".to_owned(),
                },
            },
            "attribute filtering is applied locally: the service does not list `cloud_cover` as queryable",
        ),
        (
            QueryFeature::FullText,
            Support::Pushed,
            "full-text search is pushed to the service",
        ),
        (
            QueryFeature::Sort,
            Support::Local {
                cause: Cause::Undeclared,
            },
            "sorting is applied locally: the service declared nothing about it",
        ),
        (
            QueryFeature::Paging,
            Support::Refused {
                cause: Cause::Declined,
            },
            "paging is refused: the service declared it does not support it",
        ),
        (
            QueryFeature::FieldSelection,
            Support::Local {
                cause: Cause::Undeclared,
            },
            "field selection is applied locally: the service declared nothing about it",
        ),
    ];

    for (feature, support, expected) in cases {
        assert_eq!(
            geoquery_core::CapabilityFinding { feature, support }.to_string(),
            expected
        );
    }
}

#[test]
fn a_coarse_flag_counts_as_declaring_the_operation_it_covers() {
    // Real services describe themselves both ways. A STAC service lists its filter
    // operations; a plain OGC endpoint only admits to taking a `bbox` parameter. Reading
    // only the list would treat the second as incapable of something it does every day.
    let cases = [
        (
            SpatialOperation::Bbox,
            CapabilitySet {
                bbox: Some(true),
                ..CapabilitySet::default()
            },
        ),
        (
            SpatialOperation::Nearest,
            CapabilitySet {
                nearest: Some(true),
                ..CapabilitySet::default()
            },
        ),
        (
            SpatialOperation::Intersects,
            CapabilitySet {
                geometry_filter: Some(true),
                ..CapabilitySet::default()
            },
        ),
    ];

    for (op, capabilities) in cases {
        let report = CapabilityReport::for_query(
            &GeoQuery {
                spatial: Some(SpatialPredicate {
                    op: op.clone(),
                    ..warsaw()
                }),
                ..GeoQuery::default()
            },
            &service(Some(capabilities)),
        );
        assert_eq!(
            report.findings()[0].support,
            Support::Pushed,
            "{op:?} should be covered by its coarse flag"
        );
    }
}

#[test]
fn every_exact_geometry_predicate_can_be_approximated_by_its_bounding_box() {
    // The approximation is safe for exactly those operations whose result set is
    // contained in the result set of the bounding box test. All six are.
    let boxes_only = CapabilitySet {
        spatial: vec![SpatialOperation::Bbox],
        ..CapabilitySet::default()
    };

    for op in [
        SpatialOperation::Intersects,
        SpatialOperation::Contains,
        SpatialOperation::Within,
        SpatialOperation::Touches,
        SpatialOperation::Overlaps,
        SpatialOperation::Crosses,
    ] {
        let report = CapabilityReport::for_query(
            &GeoQuery {
                spatial: Some(SpatialPredicate {
                    op: op.clone(),
                    ..warsaw()
                }),
                ..GeoQuery::default()
            },
            &service(Some(boxes_only.clone())),
        );
        assert_eq!(
            report.findings()[0].support,
            Support::Approximated {
                instead: QueryFeature::Spatial(SpatialOperation::Bbox),
                cause: Cause::Declined,
            },
            "{op:?} should degrade to a bounding box"
        );
    }
}

#[test]
fn a_predicate_whose_extent_the_box_does_not_contain_is_never_approximated() {
    // The dangerous case, and the reason `is_exact_geometry` is a list rather than
    // "anything with a geometry". A `disjoint` test selects everything *outside* the
    // shape, so filtering by the shape's bounding box would drop true matches — the one
    // kind of degradation that produces a wrong answer rather than an expensive one.
    // `dwithin` is excluded for a different reason: its extent is the geometry grown by a
    // distance, which the bounding box of the bare geometry does not cover.
    let boxes_only = CapabilitySet {
        spatial: vec![SpatialOperation::Bbox],
        geometry_filter: Some(true),
        ..CapabilitySet::default()
    };

    for op in [SpatialOperation::Disjoint, SpatialOperation::DWithin] {
        let report = CapabilityReport::for_query(
            &GeoQuery {
                spatial: Some(SpatialPredicate {
                    op: op.clone(),
                    ..warsaw()
                }),
                ..GeoQuery::default()
            },
            &service(Some(boxes_only.clone())),
        );
        assert_eq!(
            report.findings()[0].support,
            Support::Local {
                cause: Cause::Declined
            },
            "{op:?} must be evaluated here, not approximated away"
        );
    }
}

#[test]
fn a_service_that_does_not_filter_on_properties_at_all_keeps_the_whole_filter_here() {
    // The schema is never consulted in either case: a service that does not filter
    // cannot filter on a queryable field either.
    let query = GeoQuery {
        filters: Some(cloud_cover_filter()),
        ..GeoQuery::default()
    };

    let declined = CapabilityReport::for_query(
        &query,
        &service(Some(CapabilitySet {
            attribute: Some(false),
            ..CapabilitySet::default()
        })),
    );
    assert_eq!(
        declined.findings()[0].support,
        Support::Local {
            cause: Cause::Declined
        }
    );

    let undeclared = CapabilityReport::for_query(&query, &service(Some(CapabilitySet::default())));
    assert_eq!(
        undeclared.findings()[0].support,
        Support::Local {
            cause: Cause::Undeclared
        }
    );
}
