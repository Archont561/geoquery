//! [`FilterExpr`] → CQL2 JSON, the one encoding a STAC API will accept on a `POST
//! /search`.
//!
//! `geoquery-types`' filter tree is CQL2-*inspired* rather than CQL2-shaped — see that
//! module's own docs for why — so something has to do the translation, and this is it. The
//! mapping is total for every node the AST can hold: each `GeoQuery` filter node has
//! exactly one CQL2 JSON spelling, so [`compile`] cannot fail and does not return a
//! `Result`. What *can* fail is a service's willingness to evaluate the result, which is
//! not this module's judgment to make: [`crate::request::translate`] decides whether a
//! compiled filter may be sent at all, from the conformance classes the service published.
//!
//! Two deliberate non-translations:
//!
//! - **Property names pass through verbatim.** The STAC Filter extension names queryables
//!   by their published name (`eo:cloud_cover`, `datetime`, `id`, `collection`), and a
//!   service's `/queryables` document may name a property anything it likes — the spec's
//!   own example renames `eo:cloud_cover` to `CloudCover`. Prefixing `properties.` here, or
//!   stripping one the caller wrote, would be this adapter guessing at a naming convention
//!   the service alone defines, and a guess that lands wrong is answered with a 400 at
//!   best and a silently ignored predicate at worst.
//! - **`like` patterns pass through verbatim.** CQL2 `LIKE` uses `%` and `_` as its
//!   wildcards, and [`FilterExpr::Like`] documents its pattern in the same terms, so
//!   rewriting one into the other would be translating a language into itself.
//!
//! Shapes are from the STAC API Filter extension's own examples (`filter-lang:
//! cql2-json`), read 2026-10-05: `{"op": "<operator>", "args": [...]}` throughout,
//! a property reference as `{"property": "<name>"}`, and an instant as
//! `{"timestamp": "<RFC 3339>"}`.

use chrono::SecondsFormat;
use geoquery_types::{FilterExpr, FilterValue};
use serde_json::{Value, json};

/// Compile a filter tree into a CQL2 JSON expression.
///
/// The result is the value of the `filter` member of a STAC Item Search request whose
/// `filter-lang` is `cql2-json`.
#[must_use]
pub fn compile(filter: &FilterExpr) -> Value {
    match filter {
        FilterExpr::And(arguments) => operation("and", arguments.iter().map(compile).collect()),
        FilterExpr::Or(arguments) => operation("or", arguments.iter().map(compile).collect()),
        FilterExpr::Not(argument) => operation("not", vec![compile(argument)]),
        FilterExpr::Compare { field, op, value } => {
            operation(op.as_str(), vec![property(field), literal(value)])
        }
        FilterExpr::In { field, values } => operation(
            "in",
            vec![
                property(field),
                Value::Array(values.iter().map(literal).collect()),
            ],
        ),
        FilterExpr::Like { field, pattern } => operation(
            "like",
            vec![property(field), Value::String(pattern.clone())],
        ),
    }
}

/// Which CQL2 conformance class a filter tree needs beyond the basic one.
///
/// CQL2 splits its operators across conformance classes, and a service declares them
/// separately: `basic-cql2` carries the comparison operators and the boolean connectives,
/// while `LIKE` and `IN` live in `advanced-comparison-operators`. A tree that uses one of
/// the latter against a service that only declared the former is a tree that cannot be
/// pushed, so [`crate::request::translate`] asks this question before it sends anything.
///
/// Recursive rather than a flag set during [`compile`]: the question is asked *before*
/// deciding to compile at all, and a predicate that only answers after the work it gates
/// is not a gate.
#[must_use]
pub fn needs_advanced_comparison(filter: &FilterExpr) -> bool {
    match filter {
        FilterExpr::In { .. } | FilterExpr::Like { .. } => true,
        FilterExpr::And(arguments) | FilterExpr::Or(arguments) => {
            arguments.iter().any(needs_advanced_comparison)
        }
        FilterExpr::Not(argument) => needs_advanced_comparison(argument),
        FilterExpr::Compare { .. } => false,
    }
}

/// `{"op": …, "args": […]}`, the shape every CQL2 JSON node has.
fn operation(op: &str, args: Vec<Value>) -> Value {
    json!({ "op": op, "args": Value::Array(args) })
}

/// `{"property": "<name>"}`, CQL2's reference to a queryable.
fn property(field: &str) -> Value {
    json!({ "property": field })
}

/// A CQL2 JSON literal.
///
/// Three of the four are JSON scalars and travel as themselves. The fourth is an instant,
/// which CQL2 writes as a tagged object so that a timestamp is distinguishable from a
/// string that happens to look like one — the same distinction `FilterValue` draws, which
/// is why nothing is lost in either direction.
fn literal(value: &FilterValue) -> Value {
    match value {
        FilterValue::String(text) => Value::String(text.clone()),
        FilterValue::Number(number) => Value::Number(number.clone()),
        FilterValue::Boolean(flag) => Value::Bool(*flag),
        FilterValue::Timestamp(instant) => json!({
            "timestamp": instant.to_rfc3339_opts(SecondsFormat::AutoSi, true),
        }),
    }
}
