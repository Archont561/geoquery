//! Mirrors `src/filter.rs`: the attribute filter AST, what it accepts, and what it refuses.
//!
//! The documents here come from `.knowledge/query/filters.md` and `query-model.md`. The
//! corpus fixes the wire shape — `{ "field", "op", "value" }` under `and`/`or`/`not` — so
//! a test that invented its own spelling would prove the types are self-consistent and
//! nothing about whether they model the language the other five interfaces speak.
//!
//! Two kinds of rejection are exercised, and the split is deliberate. A node whose *shape*
//! is wrong — an operator nobody can compile, a list where a scalar belongs — cannot be
//! built at all, so it fails in `Deserialize`. A node that parsed but cannot mean anything
//! — an empty branch, a membership test against nothing — fails in [`FilterExpr::validate`]
//! alongside every other problem in the tree.

use chrono::{DateTime, Utc};
use geoquery_types::{
    CompareOp, FilterExpr, FilterValidationError, FilterValue, GeoQuery, MAX_FILTER_DEPTH,
};
use pretty_assertions::assert_eq;
use serde_json::json;
use ts_rs::{Config, TS};

fn instant(text: &str) -> DateTime<Utc> {
    text.parse().expect("a test timestamp is valid")
}

fn number(text: &str) -> FilterValue {
    FilterValue::Number(text.parse().expect("a test number is valid"))
}

/// `cloud_cover < 10`, the first half of the corpus example.
fn cloud_cover_below_ten() -> FilterExpr {
    FilterExpr::Compare {
        field: "cloud_cover".to_owned(),
        op: CompareOp::Lt,
        value: number("10"),
    }
}

/// `platform = 'sentinel-2'`, the second half.
fn platform_is_sentinel() -> FilterExpr {
    FilterExpr::Compare {
        field: "platform".to_owned(),
        op: CompareOp::Eq,
        value: "sentinel-2".into(),
    }
}

#[test]
fn the_documented_filter_round_trips_through_the_ast() {
    // `.knowledge/query/filters.md`, verbatim. This is the shape every compiler in the
    // table there reads, so it is the one shape that must survive untouched.
    let document = json!({
        "and": [
            { "field": "cloud_cover", "op": "<", "value": 10 },
            { "field": "platform", "op": "=", "value": "sentinel-2" }
        ]
    });

    let filter: FilterExpr =
        serde_json::from_value(document.clone()).expect("the documented filter parses");

    assert_eq!(
        filter,
        FilterExpr::And(vec![cloud_cover_below_ten(), platform_is_sentinel()])
    );
    assert_eq!(filter.validate(), Ok(()));
    assert_eq!(
        serde_json::to_value(&filter).expect("the filter serializes"),
        document,
        "a filter must come back out in the spelling it went in as"
    );
}

#[test]
fn a_filter_travels_inside_the_query_it_belongs_to() {
    // The whole `query-model.md` example, with its bare dates written as the timestamps
    // they normalise to so the document is a fixed point. This query could not be parsed
    // at all until now:
    // `deny_unknown_fields` refused the document outright because `filters` had no field
    // to land in. That refusal was the right failure for a missing feature and the wrong
    // one for a documented member.
    let document = json!({
        "semantic": "flood risk",
        "temporal": {
            "op": "during",
            "start": "2020-01-01T00:00:00Z",
            "end": "2025-01-01T00:00:00Z"
        },
        "filters": {
            "and": [
                { "field": "cloud_cover", "op": "<", "value": 10 },
                { "field": "platform", "op": "=", "value": "sentinel-2" }
            ]
        },
        "limit": 20
    });

    let query: GeoQuery =
        serde_json::from_value(document.clone()).expect("the documented query parses");

    assert_eq!(
        query.filters,
        Some(FilterExpr::And(vec![
            cloud_cover_below_ten(),
            platform_is_sentinel()
        ]))
    );
    assert_eq!(query.validate(), Ok(()));
    assert_eq!(
        serde_json::to_value(&query).expect("the query serializes"),
        document
    );
}

#[test]
fn boolean_composition_nests_to_any_depth() {
    // `(cloud_cover < 10 OR NOT (platform = 'sentinel-2')) AND title LIKE 'S2%'` — three
    // levels, each operator holding a different kind of child, because a tree that only
    // ever nests one way is not evidence that nesting works.
    let document = json!({
        "and": [
            {
                "or": [
                    { "field": "cloud_cover", "op": "<", "value": 10 },
                    { "not": { "field": "platform", "op": "=", "value": "sentinel-2" } }
                ]
            },
            { "field": "title", "op": "like", "value": "S2%" }
        ]
    });

    let filter: FilterExpr =
        serde_json::from_value(document.clone()).expect("a nested filter parses");

    assert_eq!(
        filter,
        FilterExpr::And(vec![
            FilterExpr::Or(vec![
                cloud_cover_below_ten(),
                FilterExpr::Not(Box::new(platform_is_sentinel())),
            ]),
            FilterExpr::Like {
                field: "title".to_owned(),
                pattern: "S2%".to_owned(),
            },
        ])
    );
    assert_eq!(filter.validate(), Ok(()));
    assert_eq!(
        serde_json::to_value(&filter).expect("the filter serializes"),
        document
    );
}

#[test]
fn every_comparison_operator_has_one_spelling_on_the_wire() {
    // The list is `.knowledge/query/filters.md`, and it is duplicated in the TypeScript
    // that `#[ts(type = ...)]` hands to ts-rs, which cannot derive it because the codec
    // here is hand-written. This assertion is what keeps the copy honest: an operator
    // added to `CompareOp` without being added to that string fails here.
    let spellings = [
        (CompareOp::Eq, "="),
        (CompareOp::Ne, "<>"),
        (CompareOp::Lt, "<"),
        (CompareOp::Gt, ">"),
        (CompareOp::Le, "<="),
        (CompareOp::Ge, ">="),
    ];

    for (op, spelling) in spellings {
        let filter = FilterExpr::Compare {
            field: "cloud_cover".to_owned(),
            op,
            value: number("10"),
        };
        let document = json!({ "field": "cloud_cover", "op": spelling, "value": 10 });

        assert_eq!(
            serde_json::to_value(&filter).expect("the filter serializes"),
            document,
            "{op:?} should be written {spelling}"
        );
        assert_eq!(
            serde_json::from_value::<FilterExpr>(document).expect("the filter parses"),
            filter,
            "{spelling} should be read back as {op:?}"
        );
    }
}

#[test]
fn membership_and_pattern_tests_are_their_own_shapes() {
    // `IN` and `LIKE` share the `{ field, op, value }` envelope with the comparisons but
    // not their operand: one takes a list, the other a string. Separate variants rather
    // than a `CompareOp` member and a `serde_json::Value`, so a compiler reading the tree
    // never has to ask whether the operand it was handed suits the operator it was given.
    let document = json!({
        "or": [
            { "field": "platform", "op": "in", "value": ["sentinel-2", "landsat-8"] },
            { "field": "title", "op": "like", "value": "Sentinel%" }
        ]
    });

    let filter: FilterExpr =
        serde_json::from_value(document.clone()).expect("membership and pattern tests parse");

    assert_eq!(
        filter,
        FilterExpr::Or(vec![
            FilterExpr::In {
                field: "platform".to_owned(),
                values: vec!["sentinel-2".into(), "landsat-8".into()],
            },
            FilterExpr::Like {
                field: "title".to_owned(),
                pattern: "Sentinel%".to_owned(),
            },
        ])
    );
    assert_eq!(filter.validate(), Ok(()));
    assert_eq!(
        serde_json::to_value(&filter).expect("the filter serializes"),
        document
    );
}

#[test]
fn negation_is_how_not_in_and_not_like_are_written() {
    // CQL2 spells these as their own operators. Here they are `not` wrapped around the
    // positive test, which is one node fewer to model and compiles to the same thing.
    let document = json!({
        "not": { "field": "platform", "op": "in", "value": ["sentinel-2"] }
    });

    let filter: FilterExpr = serde_json::from_value(document.clone()).expect("a negation parses");

    assert_eq!(
        filter,
        FilterExpr::Not(Box::new(FilterExpr::In {
            field: "platform".to_owned(),
            values: vec!["sentinel-2".into()],
        }))
    );
    assert_eq!(filter.validate(), Ok(()));
    assert_eq!(
        serde_json::to_value(&filter).expect("the filter serializes"),
        document
    );
}

#[test]
fn a_literal_keeps_the_type_it_was_written_in() {
    // The queryables table in the corpus types every property as string, number, boolean
    // or datetime, so those are the literals. An integer that came back as `10.0` would
    // be a different CQL2 document and, against a SQL target, a different query plan.
    let document = json!({
        "and": [
            { "field": "count", "op": "=", "value": 10 },
            { "field": "cloud_cover", "op": "<=", "value": 10.5 },
            { "field": "platform", "op": "=", "value": "sentinel-2" },
            { "field": "public", "op": "=", "value": true }
        ]
    });

    let filter: FilterExpr =
        serde_json::from_value(document.clone()).expect("every literal type parses");

    assert_eq!(
        filter,
        FilterExpr::And(vec![
            FilterExpr::Compare {
                field: "count".to_owned(),
                op: CompareOp::Eq,
                value: number("10"),
            },
            FilterExpr::Compare {
                field: "cloud_cover".to_owned(),
                op: CompareOp::Le,
                value: number("10.5"),
            },
            FilterExpr::Compare {
                field: "platform".to_owned(),
                op: CompareOp::Eq,
                value: "sentinel-2".into(),
            },
            FilterExpr::Compare {
                field: "public".to_owned(),
                op: CompareOp::Eq,
                value: true.into(),
            },
        ])
    );
    assert_eq!(
        serde_json::to_value(&filter).expect("the filter serializes"),
        document,
        "10 must not come back as 10.0, nor 10.5 as 10"
    );
}

#[test]
fn a_timestamp_literal_is_not_a_string_that_happens_to_look_like_one() {
    // CQL2 JSON wraps temporal literals for exactly this reason, and borrowing the
    // wrapper is cheaper than guessing: `"2020-01-01"` is a perfectly good value for a
    // string queryable, and a filter that silently promoted it to an instant would
    // compare a product name against a date.
    let document = json!({
        "and": [
            { "field": "datetime", "op": ">=", "value": { "timestamp": "2020-01-01T00:00:00Z" } },
            { "field": "label", "op": "=", "value": "2020-01-01T00:00:00Z" }
        ]
    });

    let filter: FilterExpr = serde_json::from_value(document.clone()).expect("both literals parse");

    assert_eq!(
        filter,
        FilterExpr::And(vec![
            FilterExpr::Compare {
                field: "datetime".to_owned(),
                op: CompareOp::Ge,
                value: FilterValue::Timestamp(instant("2020-01-01T00:00:00Z")),
            },
            FilterExpr::Compare {
                field: "label".to_owned(),
                op: CompareOp::Eq,
                value: "2020-01-01T00:00:00Z".into(),
            },
        ])
    );
    assert_eq!(
        serde_json::to_value(&filter).expect("the filter serializes"),
        document
    );
}

#[test]
fn a_timestamp_literal_accepts_a_bare_date_like_every_other_instant() {
    // The one-dialect rule from the audit: whether `2020-01-01` is an instant must not
    // depend on which struct the field sits in. A date is read as midnight UTC and
    // written back as a timestamp, so the round trip is a fixed point and not a copy.
    let filter: FilterExpr = serde_json::from_value(json!({
        "field": "datetime", "op": ">=", "value": { "timestamp": "2020-01-01" }
    }))
    .expect("a bare date is an instant here too");

    assert_eq!(
        filter,
        FilterExpr::Compare {
            field: "datetime".to_owned(),
            op: CompareOp::Ge,
            value: FilterValue::Timestamp(instant("2020-01-01T00:00:00Z")),
        }
    );
    assert_eq!(
        serde_json::to_value(&filter).expect("the filter serializes"),
        json!({
            "field": "datetime", "op": ">=", "value": { "timestamp": "2020-01-01T00:00:00Z" }
        })
    );
}

#[test]
fn a_timestamp_literal_that_is_not_an_instant_names_what_was_wrong() {
    let error = serde_json::from_value::<FilterExpr>(json!({
        "field": "datetime", "op": ">=", "value": { "timestamp": "the first of January" }
    }))
    .expect_err("`the first of January` is not an instant");

    assert!(
        error.to_string().contains("RFC 3339"),
        "the error should say what a timestamp may look like: {error}"
    );
}

#[test]
fn an_operator_this_crate_cannot_compile_is_refused() {
    // The predicate enums in `query.rs` are open because a *service* may support an
    // operation this version has not heard of. An operator is different: the engine is
    // the one that has to compile it, so an operator it cannot name is one it cannot
    // emit, and passing it through would move the failure to the adapter that trusted it.
    let error = serde_json::from_value::<FilterExpr>(json!({
        "field": "cloud_cover", "op": "~=", "value": 10
    }))
    .expect_err("`~=` is not an operator");

    assert!(
        error.to_string().contains("~="),
        "the error should quote what was written: {error}"
    );
}

#[test]
fn a_node_that_is_two_things_at_once_is_refused() {
    let error = serde_json::from_value::<FilterExpr>(json!({
        "and": [{ "field": "a", "op": "=", "value": 1 }],
        "or": [{ "field": "b", "op": "=", "value": 2 }]
    }))
    .expect_err("a node is one operator, not two");

    let message = error.to_string();
    assert!(
        message.contains("`and`") && message.contains("`or`"),
        "the error should name both members that made the node ambiguous: {error}"
    );
}

#[test]
fn a_member_the_node_does_not_model_is_refused_rather_than_dropped() {
    // Same rule the rest of the crate enforces with `deny_unknown_fields`: a predicate
    // that is quietly dropped widens the result set and leaves no reason to doubt it.
    let error = serde_json::from_value::<FilterExpr>(json!({
        "field": "cloud_cover", "op": "<", "value": 10, "units": "percent"
    }))
    .expect_err("an unmodelled member is refused");

    assert!(
        error.to_string().contains("units"),
        "the error should name the member it could not take: {error}"
    );
}

#[test]
fn an_operand_that_does_not_suit_its_operator_is_refused() {
    // Three ways to pair a good operator with the wrong operand. Each is caught while
    // reading, so the variant these would have built is one the type system does not have.
    let mismatches = [
        (
            json!({ "field": "platform", "op": "=", "value": ["a", "b"] }),
            "a list is not a scalar comparison",
        ),
        (
            json!({ "field": "platform", "op": "in", "value": "sentinel-2" }),
            "a string is not a membership list",
        ),
        (
            json!({ "field": "platform", "op": "in", "value": true }),
            "a boolean is not a membership list",
        ),
        (
            json!({ "field": "platform", "op": "in", "value": { "a": 1 } }),
            "an object is not a membership list",
        ),
        (
            json!({ "field": "platform", "op": "in", "value": null }),
            "nothing is not a membership list",
        ),
        (
            json!({ "field": "title", "op": "like", "value": 10 }),
            "a number is not a pattern",
        ),
        (
            json!({ "field": "title", "op": "like", "value": ["S2%"] }),
            "a list is not a pattern",
        ),
    ];

    for (document, why) in mismatches {
        let error = serde_json::from_value::<FilterExpr>(document).expect_err(why);
        assert!(
            error.to_string().contains("value"),
            "the error should name the member that did not suit the operator: {error}"
        );
    }
}

#[test]
fn a_branch_with_no_arguments_is_refused() {
    // The same rule as an interval with neither bound: it constrains nothing while
    // looking like it constrains something, and an empty `and` is what a generator emits
    // when it meant to emit a predicate and had none.
    assert_eq!(
        FilterExpr::And(Vec::new()).validate(),
        Err(vec![FilterValidationError::EmptyBranch {
            op: "and".to_owned()
        }])
    );
    assert_eq!(
        FilterExpr::Or(Vec::new()).validate(),
        Err(vec![FilterValidationError::EmptyBranch {
            op: "or".to_owned()
        }])
    );
}

#[test]
fn a_membership_test_against_nothing_can_never_match_and_is_refused() {
    // Unlike an empty branch this one is not meaningless, it is unsatisfiable — which is
    // worse, because it returns an empty result set that looks like a real answer.
    assert_eq!(
        FilterExpr::In {
            field: "platform".to_owned(),
            values: Vec::new(),
        }
        .validate(),
        Err(vec![FilterValidationError::EmptyMembership {
            field: "platform".to_owned()
        }])
    );
}

#[test]
fn a_membership_test_mixing_literal_types_is_refused() {
    // A queryable has one type. A list spanning two of them is a mistake at the source,
    // and the targets disagree about what to do with it: SQL would coerce, CQL2 text
    // would quote, and the two would not return the same rows.
    assert_eq!(
        FilterExpr::In {
            field: "platform".to_owned(),
            values: vec!["sentinel-2".into(), number("2")],
        }
        .validate(),
        Err(vec![FilterValidationError::MixedMembershipTypes {
            field: "platform".to_owned(),
            first: "string".to_owned(),
            found: "number".to_owned(),
        }])
    );
}

#[test]
fn an_ordering_comparison_against_a_boolean_is_refused() {
    // Strings are left alone: `platform > 's'` has a collation order in every target
    // here. Booleans have none, so `<` against one is a question with no answer.
    assert_eq!(
        FilterExpr::Compare {
            field: "public".to_owned(),
            op: CompareOp::Lt,
            value: true.into(),
        }
        .validate(),
        Err(vec![FilterValidationError::BooleanIsNotOrdered {
            field: "public".to_owned(),
            op: "<".to_owned(),
        }])
    );

    assert_eq!(
        FilterExpr::Compare {
            field: "public".to_owned(),
            op: CompareOp::Ne,
            value: true.into(),
        }
        .validate(),
        Ok(()),
        "equality against a boolean is the normal way to ask"
    );
}

#[test]
fn a_property_reference_with_no_name_is_refused() {
    assert_eq!(
        FilterExpr::Compare {
            field: "  ".to_owned(),
            op: CompareOp::Eq,
            value: number("1"),
        }
        .validate(),
        Err(vec![FilterValidationError::FieldUnnamed])
    );
}

#[test]
fn a_tree_nested_past_the_limit_is_refused_before_a_compiler_walks_it() {
    // Every compiler in the table recurses over this tree, and so does `validate`
    // itself. The depth is capped where a hand-written query never reaches and a
    // generated one has clearly run away, so the failure is an error and not a crash.
    let mut deep = cloud_cover_below_ten();
    for _ in 0..MAX_FILTER_DEPTH {
        deep = FilterExpr::Not(Box::new(deep));
    }

    assert_eq!(
        deep.validate(),
        Err(vec![FilterValidationError::TooDeep {
            limit: MAX_FILTER_DEPTH
        }])
    );

    let mut allowed = cloud_cover_below_ten();
    for _ in 0..(MAX_FILTER_DEPTH - 1) {
        allowed = FilterExpr::Not(Box::new(allowed));
    }
    assert_eq!(
        allowed.validate(),
        Ok(()),
        "the limit is a limit, not one less than itself"
    );
}

#[test]
fn validation_reports_every_problem_in_the_tree_rather_than_the_first() {
    // Mirrors `GeoQuery::validate`: an agent repairing a generated filter wants the list,
    // and the walk must keep going after it finds something rather than stopping at the
    // first branch that is wrong.
    let filter = FilterExpr::And(vec![
        FilterExpr::Compare {
            field: String::new(),
            op: CompareOp::Eq,
            value: number("1"),
        },
        FilterExpr::Or(Vec::new()),
        FilterExpr::Not(Box::new(FilterExpr::In {
            field: "platform".to_owned(),
            values: Vec::new(),
        })),
    ]);

    assert_eq!(
        filter.validate(),
        Err(vec![
            FilterValidationError::FieldUnnamed,
            FilterValidationError::EmptyBranch {
                op: "or".to_owned()
            },
            FilterValidationError::EmptyMembership {
                field: "platform".to_owned()
            },
        ])
    );
}

#[test]
fn a_filter_problem_surfaces_through_the_query_that_carries_it() {
    // `GeoQuery::validate` is the trust boundary, so a filter problem has to reach it.
    // The filter errors keep their own type and their own wording; the query error that
    // carries them adds nothing, because a caller showing the list to a person should
    // not have to unwrap two layers to read it.
    let query = GeoQuery {
        filters: Some(FilterExpr::And(Vec::new())),
        ..GeoQuery::default()
    };

    let problems = query.validate().expect_err("an empty branch is a problem");
    assert_eq!(problems.len(), 1);
    assert_eq!(
        problems[0].to_string(),
        FilterValidationError::EmptyBranch {
            op: "and".to_owned()
        }
        .to_string()
    );
}

#[test]
fn a_literal_can_be_built_from_the_rust_value_it_stands_for() {
    // Adapters build filters as well as read them — a post-filter over `raw` properties
    // is a tree this crate constructs, not one it parsed.
    assert_eq!(FilterValue::from("sentinel-2"), "sentinel-2".into());
    assert_eq!(
        FilterValue::from("sentinel-2".to_owned()),
        FilterValue::String("sentinel-2".to_owned())
    );
    assert_eq!(FilterValue::from(10_i64), number("10"));
    assert_eq!(FilterValue::from(true), FilterValue::Boolean(true));
    assert_eq!(
        FilterValue::from(instant("2020-01-01T00:00:00Z")),
        FilterValue::Timestamp(instant("2020-01-01T00:00:00Z"))
    );
}

#[test]
fn a_literal_that_is_not_one_of_the_four_types_is_refused() {
    // The object form is the narrow one: exactly `timestamp`, exactly a string. A second
    // member or a different key is a literal this crate has no type for, and guessing
    // which one was meant is how a date ends up compared against a product name.
    let refused = [
        json!({ "field": "cloud_cover", "op": "=", "value": null }),
        json!({ "field": "platform", "op": "in", "value": [["nested"]] }),
        json!({ "field": "datetime", "op": "=", "value": { "date": "2020-01-01" } }),
        json!({
            "field": "datetime",
            "op": "=",
            "value": { "timestamp": "2020-01-01T00:00:00Z", "tz": "UTC" }
        }),
        json!({ "field": "datetime", "op": "=", "value": { "timestamp": 20_200_101 } }),
    ];

    for document in refused {
        let error = serde_json::from_value::<FilterExpr>(document.clone())
            .expect_err("a literal must be one of the four types");
        assert!(
            error.to_string().contains("timestamp"),
            "the error should show the literal forms there are, for {document}: {error}"
        );
    }
}

#[test]
fn a_literal_knows_the_queryable_type_it_can_be_compared_against() {
    // The planner checks a filter against the queryables a service declares, and the
    // corpus types those as string, number, boolean, datetime or geometry. Reusing its
    // words means that check is a comparison rather than a translation.
    assert_eq!(FilterValue::from("sentinel-2").type_name(), "string");
    assert_eq!(number("10").type_name(), "number");
    assert_eq!(FilterValue::from(true).type_name(), "boolean");
    assert_eq!(
        FilterValue::from(instant("2020-01-01T00:00:00Z")).type_name(),
        "datetime"
    );
}

#[test]
fn an_operator_reads_and_writes_itself_outside_a_node() {
    // `CompareOp` is public, so a compiler may hold one on its own. It is the same
    // spelling either way round, and the same complaint when there is no such operator.
    assert_eq!(
        serde_json::from_value::<CompareOp>(json!("<=")).expect("`<=` is an operator"),
        CompareOp::Le
    );
    assert_eq!(
        serde_json::to_value(CompareOp::Ne).expect("an operator serializes"),
        json!("<>")
    );
    assert_eq!(CompareOp::Ge.to_string(), ">=");

    let error =
        serde_json::from_value::<CompareOp>(json!("≈")).expect_err("`≈` is not an operator");
    assert!(
        error.to_string().contains('≈'),
        "the error should quote what was written: {error}"
    );
}

#[test]
fn a_node_that_is_not_an_object_at_all_is_refused() {
    let error = serde_json::from_value::<FilterExpr>(json!("cloud_cover < 10"))
        .expect_err("a filter is a tree, not a sentence");

    assert!(
        error.to_string().contains("filter node"),
        "the error should say what a node looks like: {error}"
    );
}

#[test]
fn a_node_that_names_nothing_is_refused() {
    // An empty object is what a generator emits when it meant to emit a predicate and
    // built none. Reading it as "no constraint" would widen the result set in silence.
    let error = serde_json::from_value::<FilterExpr>(json!({}))
        .expect_err("an empty node names no operator");

    assert!(
        error.to_string().contains("empty"),
        "the error should say the node named nothing: {error}"
    );
}

#[test]
fn a_branch_and_a_comparison_in_one_node_is_refused() {
    let error = serde_json::from_value::<FilterExpr>(json!({
        "not": { "field": "a", "op": "=", "value": 1 },
        "field": "b"
    }))
    .expect_err("a node is a branch or a leaf, not both");

    let message = error.to_string();
    assert!(
        message.contains("`not`") && message.contains("`field`"),
        "the error should name what it found on both sides: {error}"
    );
}

#[test]
fn a_comparison_missing_one_of_its_three_members_is_refused() {
    for (document, missing) in [
        (json!({ "op": "=", "value": 1 }), "field"),
        (json!({ "field": "a", "value": 1 }), "op"),
        (json!({ "field": "a", "op": "=" }), "value"),
    ] {
        let error = serde_json::from_value::<FilterExpr>(document)
            .expect_err("a comparison needs all three members");
        assert!(
            error.to_string().contains(missing),
            "the error should name the member that was missing: {error}"
        );
    }
}

#[test]
fn a_member_written_twice_is_refused() {
    // Last-one-wins is the usual default and the wrong one here: the two copies are a
    // generator contradicting itself, and picking either silently answers a question
    // nobody asked. A branch written twice is the same mistake as a member written twice,
    // and a different one from two branches in a node, so it gets the same complaint.
    for (document, repeated) in [
        (
            r#"{"field": "a", "op": "=", "value": 1, "value": 2}"#,
            "value",
        ),
        (
            r#"{"and": [{"field": "a", "op": "=", "value": 1}], "and": []}"#,
            "and",
        ),
    ] {
        let error =
            serde_json::from_str::<FilterExpr>(document).expect_err("a member appears once");
        assert!(
            error.to_string().contains(repeated),
            "the error should name the member that was repeated: {error}"
        );
    }
}

#[test]
fn a_literal_can_be_built_from_a_json_number_directly() {
    // The escape hatch for the numbers `From<i64>` cannot reach. There is no `From<f64>`
    // because a f64 may be NaN and a JSON number may not, so the refusal happens where
    // the value is built rather than where it is serialized.
    let precise = serde_json::Number::from_f64(0.1).expect("0.1 is a JSON number");
    assert_eq!(FilterValue::from(precise), number("0.1"));
    assert!(serde_json::Number::from_f64(f64::NAN).is_none());
}

#[test]
fn a_literal_reads_and_writes_itself_outside_a_node() {
    // `FilterValue` is public because an adapter compiling a filter holds the literals on
    // their own — binding them to a SQL placeholder, say — so each one has to survive the
    // trip by itself and not only as the third member of a comparison.
    for (document, literal) in [
        (json!("sentinel-2"), FilterValue::from("sentinel-2")),
        (json!(10), number("10")),
        (json!(10.5), number("10.5")),
        (json!(true), FilterValue::from(true)),
        (
            json!({ "timestamp": "2020-01-01T00:00:00Z" }),
            FilterValue::from(instant("2020-01-01T00:00:00Z")),
        ),
    ] {
        assert_eq!(
            serde_json::from_value::<FilterValue>(document.clone()).expect("a literal parses"),
            literal
        );
        assert_eq!(
            serde_json::to_value(&literal).expect("a literal serializes"),
            document
        );
    }

    let error = serde_json::from_value::<FilterValue>(json!(["sentinel-2"]))
        .expect_err("a list is not a literal");
    assert!(
        error.to_string().contains("timestamp"),
        "the error should show the literal forms there are: {error}"
    );
}

#[test]
fn the_typescript_the_derive_emits_says_what_the_rust_says() {
    // ts-rs cannot derive these two: the codecs are hand-written, so the TypeScript is a
    // hand-written override. It records no dependencies for an override either, which is
    // why the tree spells the operator and literal unions out instead of naming them —
    // a name here would be one the generated module never imports. Nothing but this test
    // keeps the copies in step, and GQ-4 turns them into the SDKs' bindings.
    let config = Config::default();
    let operators = <CompareOp as TS>::inline(&config);
    let literal = <FilterValue as TS>::inline(&config);
    let tree = <FilterExpr as TS>::inline(&config);

    for op in [
        CompareOp::Eq,
        CompareOp::Ne,
        CompareOp::Lt,
        CompareOp::Gt,
        CompareOp::Le,
        CompareOp::Ge,
    ] {
        let quoted = format!("\"{}\"", op.as_str());
        assert!(
            operators.contains(&quoted),
            "the operator union should offer {quoted}: {operators}"
        );
    }

    assert!(
        tree.contains(&operators),
        "the tree should inline the operator union verbatim:\n  union: {operators}\n  tree:  {tree}"
    );
    assert!(
        tree.contains(&literal),
        "the tree should inline the literal union verbatim:\n  union: {literal}\n  tree:  {tree}"
    );
    for spelled_out in ["\"in\"", "\"like\"", "FilterExpr[]"] {
        assert!(
            tree.contains(spelled_out),
            "the tree should offer {spelled_out}: {tree}"
        );
    }
}
