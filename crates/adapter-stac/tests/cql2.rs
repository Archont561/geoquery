//! Mirrors `src/cql2.rs`: a `geoquery-types` filter tree → the CQL2 JSON a STAC API
//! accepts as `filter` when `filter-lang` is `cql2-json`.
//!
//! The expected documents below are shaped after the STAC API Filter extension's own
//! examples rather than after what this compiler happens to emit — that is the whole
//! point of the file. A test that asserted `compile` returns what `compile` returns would
//! pass whatever the wire format turned out to be.

use geoquery_adapter_stac::cql2::{compile, needs_advanced_comparison};
use geoquery_types::{CompareOp, FilterExpr, FilterValue};
use pretty_assertions::assert_eq;
use serde_json::json;

fn cloud_cover_below(value: i64) -> FilterExpr {
    FilterExpr::Compare {
        field: "eo:cloud_cover".to_owned(),
        op: CompareOp::Lt,
        value: FilterValue::Number(value.into()),
    }
}

#[test]
fn a_comparison_becomes_an_op_with_a_property_and_a_literal() {
    assert_eq!(
        compile(&cloud_cover_below(10)),
        json!({ "op": "<", "args": [{ "property": "eo:cloud_cover" }, 10] })
    );
}

#[test]
fn every_comparison_operator_keeps_its_cql2_spelling() {
    // `CompareOp::as_str` is already the CQL2 spelling of each operator, which is why
    // this compiler does not translate them — this test is what keeps that true.
    for (op, spelling) in [
        (CompareOp::Eq, "="),
        (CompareOp::Ne, "<>"),
        (CompareOp::Lt, "<"),
        (CompareOp::Gt, ">"),
        (CompareOp::Le, "<="),
        (CompareOp::Ge, ">="),
    ] {
        let filter = FilterExpr::Compare {
            field: "gsd".to_owned(),
            op,
            value: FilterValue::Number(10.into()),
        };
        assert_eq!(
            compile(&filter),
            json!({ "op": spelling, "args": [{ "property": "gsd" }, 10] }),
            "operator {op} compiled to the wrong CQL2 spelling"
        );
    }
}

#[test]
fn a_property_name_is_never_rewritten() {
    // The Filter extension lets a service name a queryable anything its `/queryables`
    // document says — the spec's own example renames `eo:cloud_cover` to `CloudCover` —
    // so prefixing or stripping `properties.` here would be guessing at a convention the
    // service alone defines.
    for name in ["eo:cloud_cover", "properties.eo:cloud_cover", "CloudCover"] {
        let filter = FilterExpr::Compare {
            field: name.to_owned(),
            op: CompareOp::Eq,
            value: FilterValue::Boolean(true),
        };
        assert_eq!(
            compile(&filter),
            json!({ "op": "=", "args": [{ "property": name }, true] })
        );
    }
}

#[test]
fn a_timestamp_literal_is_tagged_rather_than_written_as_a_string() {
    let filter = FilterExpr::Compare {
        field: "datetime".to_owned(),
        op: CompareOp::Ge,
        value: FilterValue::Timestamp(
            "2024-06-01T00:00:00Z"
                .parse()
                .expect("a test timestamp is valid"),
        ),
    };
    assert_eq!(
        compile(&filter),
        json!({
            "op": ">=",
            "args": [
                { "property": "datetime" },
                { "timestamp": "2024-06-01T00:00:00Z" }
            ]
        })
    );
}

#[test]
fn boolean_branches_nest_as_args_arrays() {
    let filter = FilterExpr::And(vec![
        cloud_cover_below(10),
        FilterExpr::Or(vec![
            FilterExpr::Compare {
                field: "platform".to_owned(),
                op: CompareOp::Eq,
                value: FilterValue::String("sentinel-2a".to_owned()),
            },
            FilterExpr::Not(Box::new(cloud_cover_below(1))),
        ]),
    ]);

    assert_eq!(
        compile(&filter),
        json!({
            "op": "and",
            "args": [
                { "op": "<", "args": [{ "property": "eo:cloud_cover" }, 10] },
                {
                    "op": "or",
                    "args": [
                        { "op": "=", "args": [{ "property": "platform" }, "sentinel-2a"] },
                        {
                            "op": "not",
                            "args": [
                                { "op": "<", "args": [{ "property": "eo:cloud_cover" }, 1] }
                            ]
                        }
                    ]
                }
            ]
        })
    );
}

#[test]
fn a_membership_test_puts_the_candidates_in_one_list_argument() {
    let filter = FilterExpr::In {
        field: "platform".to_owned(),
        values: vec![
            FilterValue::String("sentinel-2a".to_owned()),
            FilterValue::String("sentinel-2b".to_owned()),
        ],
    };
    assert_eq!(
        compile(&filter),
        json!({
            "op": "in",
            "args": [{ "property": "platform" }, ["sentinel-2a", "sentinel-2b"]]
        })
    );
}

#[test]
fn a_like_pattern_is_passed_through_with_its_wildcards_intact() {
    let filter = FilterExpr::Like {
        field: "mission".to_owned(),
        pattern: "sentinel%".to_owned(),
    };
    assert_eq!(
        compile(&filter),
        json!({ "op": "like", "args": [{ "property": "mission" }, "sentinel%"] })
    );
}

#[test]
fn an_integer_literal_does_not_become_a_float() {
    // `FilterValue::Number` keeps a `serde_json::Number` exactly as it was written, and
    // the point of that is lost if the compiler reconstructs it through `f64`.
    let filter = FilterExpr::Compare {
        field: "view:off_nadir".to_owned(),
        op: CompareOp::Eq,
        value: FilterValue::Number(0.into()),
    };
    let compiled = compile(&filter);
    let literal = &compiled["args"][1];
    assert_eq!(literal.to_string(), "0", "an integer kept its integer form");
    assert!(literal.is_i64());
}

#[test]
fn only_like_and_in_need_the_advanced_comparison_class() {
    assert!(!needs_advanced_comparison(&cloud_cover_below(10)));
    assert!(needs_advanced_comparison(&FilterExpr::Like {
        field: "mission".to_owned(),
        pattern: "sentinel%".to_owned(),
    }));
    assert!(needs_advanced_comparison(&FilterExpr::In {
        field: "platform".to_owned(),
        values: vec![FilterValue::String("sentinel-2a".to_owned())],
    }));
}

#[test]
fn an_advanced_operator_is_found_however_deeply_it_is_buried() {
    // The question gates whether the whole tree may be sent, so a `like` three branches
    // down has to count — a service that declared only `basic-cql2` would reject or,
    // worse, misread the request that carried it.
    let filter = FilterExpr::And(vec![
        cloud_cover_below(10),
        FilterExpr::Not(Box::new(FilterExpr::Or(vec![
            cloud_cover_below(5),
            FilterExpr::Like {
                field: "mission".to_owned(),
                pattern: "sentinel%".to_owned(),
            },
        ]))),
    ]);
    assert!(needs_advanced_comparison(&filter));
}
