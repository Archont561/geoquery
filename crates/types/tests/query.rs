//! Mirrors `src/query.rs`: the canonical query AST, what it accepts, and what it refuses.
//!
//! The documents here are the ones in `.knowledge/query/`, because the corpus is what the
//! other five interfaces are written against. A test that invented its own shapes would
//! prove the types are self-consistent and nothing about whether they model the language.

use chrono::{DateTime, Utc};
use geoquery_types::{
    DistanceUnit, ExecutionMode, ExecutionOptions, GeoQuery, IncludeOptions, QueryScope,
    QueryValidationError, ResourceType, Selection, SortDirection, SortExpression, SpatialOperation,
    SpatialPredicate, TemporalOperation, TemporalPredicate,
};
use pretty_assertions::assert_eq;
use serde_json::json;

fn instant(text: &str) -> DateTime<Utc> {
    text.parse().expect("a test timestamp is valid")
}

fn warsaw_polygon() -> serde_json::Value {
    json!({
        "type": "Polygon",
        "coordinates": [[
            [14.1, 49.0], [24.2, 49.0], [24.2, 54.8], [14.1, 54.8], [14.1, 49.0]
        ]]
    })
}

#[test]
fn the_documented_query_round_trips_through_the_ast() {
    // `.knowledge/query/query-model.md`, minus the `filters` member that arrives with
    // GQ-3. Every other field of that example is modelled here, which is the claim this
    // test exists to keep true.
    let document = json!({
        "semantic": "flood risk",
        "spatial": { "op": "intersects", "geometry": warsaw_polygon() },
        "temporal": { "op": "during", "start": "2020-01-01", "end": "2025-01-01" },
        "limit": 20
    });

    let query: GeoQuery = serde_json::from_value(document).expect("the documented query parses");

    assert_eq!(query.semantic.as_deref(), Some("flood risk"));
    let spatial = query.spatial.as_ref().expect("a spatial predicate");
    assert_eq!(spatial.op, SpatialOperation::Intersects);
    assert_eq!(spatial.geometry.as_ref(), Some(&warsaw_polygon()));
    let temporal = query.temporal.as_ref().expect("a temporal predicate");
    assert_eq!(temporal.op, TemporalOperation::During);
    assert_eq!(temporal.start, Some(instant("2020-01-01T00:00:00Z")));
    assert_eq!(temporal.end, Some(instant("2025-01-01T00:00:00Z")));
    assert_eq!(query.limit, Some(20));
    assert_eq!(query.validate(), Ok(()));

    // Bare dates come back as instants, and nothing the caller did not set appears. The
    // second is what makes the wire format readable: a struct of fifteen optional fields
    // that serialised them all would write a wall of nulls around the two that matter.
    assert_eq!(
        serde_json::to_value(&query).expect("the query serializes"),
        json!({
            "semantic": "flood risk",
            "spatial": { "op": "intersects", "geometry": warsaw_polygon() },
            "temporal": {
                "op": "during",
                "start": "2020-01-01T00:00:00Z",
                "end": "2025-01-01T00:00:00Z"
            },
            "limit": 20
        })
    );
}

#[test]
fn serializing_a_query_twice_cannot_drift() {
    // Normalising dates to instants means the first serialisation is not the caller's
    // text. What has to hold is that it is a fixed point: parse, write, parse, write
    // again, and the second document is the first. Without that, a snapshot of a query
    // would depend on how many times it had been through the engine.
    let once: GeoQuery = serde_json::from_value(json!({
        "temporal": { "op": "intersects", "start": "2020-01-01", "end": "2025-01-01" },
        "sort": [{ "field": "datetime", "direction": "desc" }],
        "fields": ["id", "title"]
    }))
    .expect("the query parses");

    let written = serde_json::to_value(&once).expect("the query serializes");
    let twice: GeoQuery = serde_json::from_value(written.clone()).expect("its output parses");

    assert_eq!(once, twice);
    assert_eq!(
        written,
        serde_json::to_value(&twice).expect("the query serializes again")
    );
}

#[test]
fn an_interval_may_be_left_open_at_either_end() {
    // "everything since 2020" names no upper bound and is not a mistake; the corpus
    // requires both half-open forms.
    let since: GeoQuery =
        serde_json::from_value(json!({ "temporal": { "op": "after", "start": "2020-01-01" } }))
            .expect("an open-ended start parses");
    let until: GeoQuery =
        serde_json::from_value(json!({ "temporal": { "op": "before", "end": "2025-01-01" } }))
            .expect("an open-ended end parses");

    let since_predicate = since.temporal.as_ref().expect("a temporal predicate");
    assert_eq!(since_predicate.op, TemporalOperation::After);
    assert_eq!(since_predicate.end, None);
    assert_eq!(since.validate(), Ok(()));

    let until_predicate = until.temporal.as_ref().expect("a temporal predicate");
    assert_eq!(until_predicate.op, TemporalOperation::Before);
    assert_eq!(until_predicate.start, None);
    assert_eq!(until.validate(), Ok(()));
}

#[test]
fn instants_are_accepted_as_dates_or_as_timestamps() {
    let query: GeoQuery = serde_json::from_value(json!({
        "temporal": { "op": "during", "start": "2024-06-15T10:30:00Z", "end": "2025-01-01" }
    }))
    .expect("a timestamp and a date parse side by side");

    let temporal = query.temporal.as_ref().expect("a temporal predicate");
    assert_eq!(temporal.start, Some(instant("2024-06-15T10:30:00Z")));
    assert_eq!(temporal.end, Some(instant("2025-01-01T00:00:00Z")));
}

#[test]
fn an_instant_that_is_neither_a_date_nor_a_timestamp_names_what_was_wrong() {
    let error = serde_json::from_value::<GeoQuery>(json!({
        "temporal": { "op": "during", "start": "last Tuesday" }
    }))
    .expect_err("`last Tuesday` is not an instant");

    assert!(
        error.to_string().contains("RFC 3339"),
        "the error should say what a start may look like: {error}"
    );
}

#[test]
fn a_non_default_crs_travels_with_the_coordinates_it_describes() {
    // `.knowledge/query/spatial.md`: coordinates in EPSG:2180 are metres, and reading
    // them as degrees would place the point off the coast of Africa. The CRS is carried
    // on the predicate precisely so nothing downstream has to guess.
    let query: GeoQuery = serde_json::from_value(json!({
        "spatial": {
            "op": "intersects",
            "geometry": { "type": "Point", "coordinates": [637_123.0, 487_234.0] },
            "crs": "EPSG:2180"
        }
    }))
    .expect("a projected geometry parses");

    let spatial = query.spatial.as_ref().expect("a spatial predicate");
    assert_eq!(spatial.crs.as_deref(), Some("EPSG:2180"));
    assert_eq!(query.validate(), Ok(()));
}

#[test]
fn every_federation_and_shaping_field_is_typed() {
    let query: GeoQuery = serde_json::from_value(json!({
        "scope": {
            "resources": "*",
            "providers": ["NASA", "Copernicus"],
            "resourceTypes": ["dataset", "coverage"],
            "tags": ["flood"]
        },
        "execution": { "federate": true, "maxSources": 20, "timeoutMs": 5000, "mode": "hybrid" },
        "sort": [{ "field": "datetime", "direction": "desc" }, { "field": "id" }],
        "offset": 40,
        "include": { "raw": false, "assets": true }
    }))
    .expect("the federation and shaping fields parse");

    let scope = query.scope.as_ref().expect("a scope");
    assert_eq!(scope.resources, Some(Selection::All));
    assert_eq!(scope.providers, ["NASA", "Copernicus"]);
    assert_eq!(
        scope.resource_types,
        [ResourceType::Dataset, ResourceType::Coverage]
    );

    let execution = query.execution.as_ref().expect("execution options");
    assert_eq!(execution.max_sources, Some(20));
    assert_eq!(execution.timeout_ms, Some(5000));
    assert_eq!(execution.mode, Some(ExecutionMode::Hybrid));

    // An omitted direction is ascending rather than absent: a sort key with no order is
    // not a question the adapter should have to answer differently from its neighbours.
    assert_eq!(
        query.sort,
        [
            SortExpression {
                field: "datetime".to_owned(),
                direction: SortDirection::Desc
            },
            SortExpression {
                field: "id".to_owned(),
                direction: SortDirection::Asc
            }
        ]
    );
    assert_eq!(query.offset, Some(40));
    assert_eq!(
        query.include,
        Some(IncludeOptions {
            raw: Some(false),
            context: None,
            assets: Some(true)
        })
    );
}

#[test]
fn a_scope_distinguishes_everything_from_a_named_list() {
    let wildcard: QueryScope =
        serde_json::from_value(json!({ "resources": "*" })).expect("a wildcard scope parses");
    let named: QueryScope =
        serde_json::from_value(json!({ "resources": ["a", "b"] })).expect("a named scope parses");
    let unset: QueryScope = serde_json::from_value(json!({})).expect("an empty scope parses");

    assert_eq!(wildcard.resources, Some(Selection::All));
    assert_eq!(
        named.resources,
        Some(Selection::Named(vec!["a".to_owned(), "b".to_owned()]))
    );
    assert_eq!(
        unset.resources, None,
        "a scope nobody set is not a scope that selected nothing"
    );

    assert_eq!(
        serde_json::to_value(&wildcard).expect("a wildcard scope serializes"),
        json!({ "resources": "*" })
    );
    assert_eq!(
        serde_json::to_value(&named).expect("a named scope serializes"),
        json!({ "resources": ["a", "b"] })
    );
}

#[test]
fn a_bare_identifier_is_not_a_selection() {
    let error = serde_json::from_value::<QueryScope>(json!({ "resources": "sentinel-2" }))
        .expect_err("a lone identifier is not a selection");

    assert!(
        error.to_string().contains('*'),
        "the error should say which string is allowed: {error}"
    );
}

#[test]
fn a_field_this_version_does_not_model_is_refused_rather_than_dropped() {
    // `filters` is GQ-3. Until it exists, a query carrying one must fail: silently
    // ignoring a predicate widens the result set, and a caller who asked for cloud cover
    // below ten would get everything back and no reason to doubt it.
    let error = serde_json::from_value::<GeoQuery>(json!({
        "spatial": { "op": "bbox", "bbox": [14.1, 49.0, 24.2, 54.8] },
        "filters": { "and": [] }
    }))
    .expect_err("an unmodelled member is refused");

    assert!(
        error.to_string().contains("filters"),
        "the error should name the member it could not take: {error}"
    );
}

#[test]
fn an_unknown_execution_mode_is_refused() {
    // The predicate enums are open because a service may support something this version
    // has not heard of. A mode names what this engine does, so there is nothing to be
    // open to, and `lcoal` must not quietly become a remote fan-out.
    let error = serde_json::from_value::<GeoQuery>(json!({ "execution": { "mode": "lcoal" } }))
        .expect_err("`lcoal` is not a mode");

    assert!(
        error.to_string().contains("lcoal"),
        "the error should quote what was written: {error}"
    );
}

#[test]
fn an_empty_query_is_valid() {
    // Asking for everything is a real request, and the validator's job is contradictions,
    // not breadth.
    assert_eq!(GeoQuery::default().validate(), Ok(()));
}

#[test]
fn a_distance_without_a_unit_is_refused() {
    // `.knowledge/query/spatial.md` makes this the worked example: 50000 is a plausible
    // distance in metres and an absurd one in degrees, and there is no safe default.
    let query = GeoQuery {
        spatial: Some(SpatialPredicate {
            op: SpatialOperation::DWithin,
            bbox: None,
            geometry: Some(json!({ "type": "Point", "coordinates": [21.01, 52.23] })),
            distance: Some(50_000.0),
            unit: None,
            crs: None,
        }),
        ..GeoQuery::default()
    };

    assert_eq!(
        query.validate(),
        Err(vec![QueryValidationError::DistanceWithoutUnit])
    );
}

#[test]
fn a_well_formed_distance_query_is_accepted() {
    let query = GeoQuery {
        spatial: Some(SpatialPredicate {
            op: SpatialOperation::DWithin,
            bbox: None,
            geometry: Some(json!({ "type": "Point", "coordinates": [21.01, 52.23] })),
            distance: Some(50_000.0),
            unit: Some(DistanceUnit::Meters),
            crs: None,
        }),
        ..GeoQuery::default()
    };

    assert_eq!(query.validate(), Ok(()));
}

#[test]
fn an_interval_that_ends_before_it_starts_is_refused() {
    let query = GeoQuery {
        temporal: Some(TemporalPredicate {
            op: TemporalOperation::During,
            start: Some(instant("2025-01-01T00:00:00Z")),
            end: Some(instant("2020-01-01T00:00:00Z")),
        }),
        ..GeoQuery::default()
    };

    assert_eq!(
        query.validate(),
        Err(vec![QueryValidationError::IntervalInverted {
            start: instant("2025-01-01T00:00:00Z"),
            end: instant("2020-01-01T00:00:00Z"),
        }])
    );
}

#[test]
fn an_interval_with_neither_bound_constrains_nothing_and_is_refused() {
    let query = GeoQuery {
        temporal: Some(TemporalPredicate {
            op: TemporalOperation::During,
            start: None,
            end: None,
        }),
        ..GeoQuery::default()
    };

    assert_eq!(
        query.validate(),
        Err(vec![QueryValidationError::IntervalUnbounded {
            op: "during".to_owned()
        }])
    );
}

#[test]
fn a_predicate_without_the_operand_its_operation_reads_is_refused() {
    let no_geometry = GeoQuery {
        spatial: Some(SpatialPredicate {
            op: SpatialOperation::Intersects,
            bbox: None,
            geometry: None,
            distance: None,
            unit: None,
            crs: None,
        }),
        ..GeoQuery::default()
    };
    assert_eq!(
        no_geometry.validate(),
        Err(vec![QueryValidationError::GeometryMissing {
            op: "intersects".to_owned()
        }])
    );

    let no_bbox = GeoQuery {
        spatial: Some(SpatialPredicate {
            op: SpatialOperation::Bbox,
            bbox: None,
            geometry: None,
            distance: None,
            unit: None,
            crs: None,
        }),
        ..GeoQuery::default()
    };
    assert_eq!(
        no_bbox.validate(),
        Err(vec![QueryValidationError::BoundingBoxMissing])
    );
}

#[test]
fn an_operation_this_version_cannot_name_keeps_its_operands_to_itself() {
    // A custom predicate is how a source advertises something newer than this crate. The
    // validator has no idea what operands it reads, and inventing a requirement would
    // refuse a query the source would have answered.
    let query = GeoQuery {
        spatial: Some(SpatialPredicate {
            op: SpatialOperation::Custom("relate-mask".to_owned()),
            bbox: None,
            geometry: None,
            distance: None,
            unit: None,
            crs: None,
        }),
        ..GeoQuery::default()
    };

    assert_eq!(query.validate(), Ok(()));
}

#[test]
fn a_malformed_geometry_is_refused_before_a_source_sees_it() {
    let query = GeoQuery {
        spatial: Some(SpatialPredicate {
            op: SpatialOperation::Intersects,
            bbox: None,
            geometry: Some(json!({ "type": "Polygon", "coordinates": "not coordinates" })),
            distance: None,
            unit: None,
            crs: None,
        }),
        ..GeoQuery::default()
    };

    let problems = query.validate().expect_err("the geometry is not GeoJSON");
    assert!(
        matches!(
            problems.as_slice(),
            [QueryValidationError::GeometryMalformed { .. }]
        ),
        "got {problems:?}"
    );
}

#[test]
fn a_crs_nobody_can_resolve_is_refused_rather_than_guessed() {
    let query = GeoQuery {
        spatial: Some(SpatialPredicate {
            op: SpatialOperation::Bbox,
            bbox: Some([14.1, 49.0, 24.2, 54.8]),
            geometry: None,
            distance: None,
            unit: None,
            crs: Some("Polish grid".to_owned()),
        }),
        ..GeoQuery::default()
    };

    assert_eq!(
        query.validate(),
        Err(vec![QueryValidationError::CrsUnrecognised {
            crs: "Polish grid".to_owned()
        }])
    );
}

#[test]
fn the_crs_forms_the_corpus_uses_are_all_accepted() {
    for crs in [
        "EPSG:4326",
        "epsg:2180",
        "CRS84",
        "OGC:CRS84",
        "urn:ogc:def:crs:EPSG::4326",
        "http://www.opengis.net/def/crs/EPSG/0/4326",
    ] {
        let query = GeoQuery {
            spatial: Some(SpatialPredicate {
                op: SpatialOperation::Bbox,
                bbox: Some([14.1, 49.0, 24.2, 54.8]),
                geometry: None,
                distance: None,
                unit: None,
                crs: Some(crs.to_owned()),
            }),
            ..GeoQuery::default()
        };
        assert_eq!(query.validate(), Ok(()), "{crs} should be recognised");
    }
}

#[test]
fn a_box_crossing_the_antimeridian_is_not_an_inverted_one() {
    // West above east is how a box spanning 180 degrees is written, so only south above
    // north is wrong — and only when no CRS has redefined which axis is which.
    let across = GeoQuery {
        spatial: Some(SpatialPredicate {
            op: SpatialOperation::Bbox,
            bbox: Some([170.0, -10.0, -170.0, 10.0]),
            geometry: None,
            distance: None,
            unit: None,
            crs: None,
        }),
        ..GeoQuery::default()
    };
    assert_eq!(across.validate(), Ok(()));

    let upside_down = GeoQuery {
        spatial: Some(SpatialPredicate {
            op: SpatialOperation::Bbox,
            bbox: Some([14.1, 54.8, 24.2, 49.0]),
            geometry: None,
            distance: None,
            unit: None,
            crs: None,
        }),
        ..GeoQuery::default()
    };
    assert_eq!(
        upside_down.validate(),
        Err(vec![QueryValidationError::BoundingBoxInverted {
            south: 54.8,
            north: 49.0
        }])
    );
}

#[test]
fn a_box_with_an_ordinate_that_is_not_a_number_is_refused_before_it_is_ranked() {
    // Infinity and NaN survive arithmetic that produced them by mistake, and comparing
    // them silently answers false, so an inverted-box check would pass a box that has no
    // corners. The ordinates are tested for being numbers at all before anything else.
    for bbox in [
        [14.1, 49.0, f64::NAN, 54.8],
        [f64::NEG_INFINITY, 49.0, 24.2, 54.8],
    ] {
        let query = GeoQuery {
            spatial: Some(SpatialPredicate {
                op: SpatialOperation::Bbox,
                bbox: Some(bbox),
                geometry: None,
                distance: None,
                unit: None,
                crs: None,
            }),
            ..GeoQuery::default()
        };
        assert_eq!(
            query.validate(),
            Err(vec![QueryValidationError::BoundingBoxNotFinite]),
            "{bbox:?} should be refused"
        );
    }
}

#[test]
fn a_bound_written_as_null_is_the_same_as_one_left_out() {
    // A generator emitting every key with a null for the ones it has nothing for is
    // common enough that refusing it would be pedantry; `null` means unbounded.
    let query: GeoQuery = serde_json::from_value(json!({
        "temporal": { "op": "after", "start": "2020-01-01", "end": null }
    }))
    .expect("an explicit null bound parses");

    let temporal = query.temporal.as_ref().expect("a temporal predicate");
    assert_eq!(temporal.start, Some(instant("2020-01-01T00:00:00Z")));
    assert_eq!(temporal.end, None);
    assert_eq!(query.validate(), Ok(()));

    // ...and it is written back out as the absence it was read as, not as a null.
    assert_eq!(
        serde_json::to_value(&query).expect("the query serializes"),
        json!({ "temporal": { "op": "after", "start": "2020-01-01T00:00:00Z" } })
    );
}

#[test]
fn validation_reports_every_problem_rather_than_the_first() {
    // An agent repairing a generated query wants the list. Handing back one problem per
    // attempt turns a single fix into a conversation.
    let query = GeoQuery {
        spatial: Some(SpatialPredicate {
            op: SpatialOperation::DWithin,
            bbox: None,
            geometry: None,
            distance: None,
            unit: Some(DistanceUnit::Meters),
            crs: Some("somewhere".to_owned()),
        }),
        temporal: Some(TemporalPredicate {
            op: TemporalOperation::During,
            start: Some(instant("2025-01-01T00:00:00Z")),
            end: Some(instant("2020-01-01T00:00:00Z")),
        }),
        ..GeoQuery::default()
    };

    assert_eq!(
        query.validate(),
        Err(vec![
            QueryValidationError::GeometryMissing {
                op: "dwithin".to_owned()
            },
            QueryValidationError::DistanceMissing {
                op: "dwithin".to_owned()
            },
            QueryValidationError::UnitWithoutDistance,
            QueryValidationError::CrsUnrecognised {
                crs: "somewhere".to_owned()
            },
            QueryValidationError::IntervalInverted {
                start: instant("2025-01-01T00:00:00Z"),
                end: instant("2020-01-01T00:00:00Z"),
            },
        ])
    );
}

#[test]
fn execution_options_default_to_nothing_asserted() {
    // Absent is not false. A query that does not mention federation has not asked for it
    // to be switched off, and the planner's default is the planner's to choose.
    let options = ExecutionOptions::default();
    assert_eq!(options.federate, None);
    assert_eq!(options.mode, None);
    assert_eq!(
        serde_json::to_value(&options).expect("empty options serialize"),
        json!({})
    );
}
