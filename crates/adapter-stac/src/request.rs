//! `GeoQuery` → STAC Item Search, and nothing past what this spike implements.
//!
//! [`translate`] only ever sets `bbox`, `datetime`, `collections` and `limit` on the
//! request it builds. That is the whole scope: a `GeoQuery` member this function does not
//! name here is a member it does not send, on purpose, and it says so by returning a
//! [`geoquery_core::CapabilityFinding`] for every member it skipped — not by raising an
//! error, which would make the whole query refuse to run over one predicate the service
//! may not even have been asked to carry, and not silently, which is the one outcome the
//! design corpus rules out everywhere.
//!
//! Every finding here is [`Support::Pushed`], [`Support::Refused`] or
//! [`Support::Local`] — never [`Support::Approximated`]. Approximation promises a local
//! follow-up ("pushed a bounding box, then filtered the exact geometry against it"), and
//! this adapter does not implement one: normalizing a result does not evaluate a geometry
//! predicate against it. Claiming `Approximated` without doing the follow-up would be a
//! finding this adapter could not back up, so an exact-geometry predicate is `Refused`
//! instead — still correct (nothing claims to have narrowed the results), just less
//! useful than a future version that actually performs the local step.

use chrono::{DateTime, SecondsFormat, Utc};
use geoquery_core::{CapabilityFinding, Cause, QueryFeature, Support};
use geoquery_types::{BoundingBox, GeoQuery, Selection, SpatialOperation, TemporalOperation};
use serde::Serialize;
use serde_json::{Map, Value};

/// The STAC Item Search request body this adapter is willing to send.
///
/// Four fields, matching the narrow spike's scope exactly. `collections` serializes as
/// `[]` when empty rather than being skipped outright by most STAC servers' own
/// convention, but this adapter omits it entirely in that case (`skip_serializing_if`
/// below) — an absent `collections` and STAC's own default both mean "every collection",
/// so there is nothing to gain from sending the empty form.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StacSearchRequest {
    /// West, south, east, north, forwarded from [`geoquery_types::SpatialPredicate::bbox`]
    /// unchanged — STAC and `GeoQuery` agree on both the field shape and the axis order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<BoundingBox>,
    /// An RFC 3339 instant or `start/end` interval, open ends written `..`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datetime: Option<String>,
    /// Collection identifiers to search within.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub collections: Vec<String>,
    /// Maximum number of items to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

impl StacSearchRequest {
    /// This request as the JSON object [`geoquery_types::Provenance::query`] expects.
    ///
    /// # Panics
    ///
    /// Never, in practice: every field here is a primitive, a string or a vector of
    /// strings, all of which serialize to JSON unconditionally. The `expect` exists
    /// because [`serde::Serialize`] cannot promise that statically, not because this
    /// type is expected to fail it.
    #[must_use]
    pub fn to_json_object(&self) -> Map<String, Value> {
        match serde_json::to_value(self).expect("a StacSearchRequest always serializes") {
            Value::Object(object) => object,
            other => unreachable!("a struct serializes to a JSON object, got {other:?}"),
        }
    }
}

/// Build the request this adapter will send for `query`, and report what will not reach
/// the service because of it.
///
/// The returned findings cover every `GeoQuery` member this function read, in the order
/// the query names them, so a caller building `source status` output can present them
/// without re-deriving which features were even in play.
#[must_use]
pub fn translate(query: &GeoQuery) -> (StacSearchRequest, Vec<CapabilityFinding>) {
    let mut request = StacSearchRequest::default();
    let mut findings = Vec::new();

    translate_spatial(query, &mut request, &mut findings);
    translate_temporal(query, &mut request, &mut findings);
    translate_scope(query, &mut request);
    translate_paging(query, &mut request, &mut findings);

    if query.filters.is_some() {
        findings.push(not_implemented(QueryFeature::AttributeFilter));
    }
    if query.semantic.is_some() {
        findings.push(not_implemented(QueryFeature::Semantic));
    }
    if !query.sort.is_empty() {
        findings.push(not_implemented(QueryFeature::Sort));
    }
    if !query.fields.is_empty() {
        // The one feature this spike really does carry out locally: `GeoResult` is
        // already fully materialized by the time field projection would run, so
        // trimming `properties` to the requested set is correct and nearly free. See
        // `geoquery-core`'s own `CapabilityReport::for_query`, which reaches the same
        // finding for the same reason.
        findings.push(CapabilityFinding {
            feature: QueryFeature::FieldSelection,
            support: Support::Local {
                cause: Cause::Undeclared,
            },
        });
    }

    (request, findings)
}

/// A feature this adapter version does not implement: not pushed to the service, and not
/// applied locally either, because there is no local post-processing step for it here.
fn not_implemented(feature: QueryFeature) -> CapabilityFinding {
    CapabilityFinding {
        feature,
        support: Support::Refused {
            // `Undeclared` rather than `Declined`: both causes in `geoquery-core` describe
            // what the *service* said, and this is a restriction of this adapter's
            // current implementation instead. Neither variant is a perfect fit — see the
            // module docs on why a new `Cause` is not added for it here — and `Undeclared`
            // is the one that does not accuse the service of refusing something it may in
            // fact support.
            cause: Cause::Undeclared,
        },
    }
}

fn translate_spatial(
    query: &GeoQuery,
    request: &mut StacSearchRequest,
    findings: &mut Vec<CapabilityFinding>,
) {
    let Some(spatial) = &query.spatial else {
        return;
    };
    if spatial.op == SpatialOperation::Bbox {
        if let Some(bbox) = spatial.bbox {
            request.bbox = Some(bbox);
            findings.push(CapabilityFinding {
                feature: QueryFeature::Spatial(SpatialOperation::Bbox),
                support: Support::Pushed,
            });
        }
        // `bbox` without a bounding box cannot pass `GeoQuery::validate`, so a caller
        // that validated first never reaches this branch with nothing to push.
    } else {
        findings.push(not_implemented(QueryFeature::Spatial(spatial.op.clone())));
    }
}

fn translate_temporal(
    query: &GeoQuery,
    request: &mut StacSearchRequest,
    findings: &mut Vec<CapabilityFinding>,
) {
    let Some(temporal) = &query.temporal else {
        return;
    };
    if temporal.op == TemporalOperation::Intersects {
        request.datetime = Some(datetime_range(temporal.start, temporal.end));
        findings.push(CapabilityFinding {
            feature: QueryFeature::Temporal(TemporalOperation::Intersects),
            support: Support::Pushed,
        });
    } else {
        // `before`, `after`, `during`, `contains` and `overlaps` each need interval
        // algebra STAC's single `datetime` range cannot express without risking a
        // narrower-than-correct request — see the module docs on why this spike does
        // not approximate them instead. No filter is sent, which is always a safe
        // superset: an unfiltered search drops no true match.
        findings.push(not_implemented(QueryFeature::Temporal(temporal.op.clone())));
    }
}

/// STAC's `datetime` parameter: a single RFC 3339 instant, or a `start/end` interval with
/// `..` standing in for an open end.
fn datetime_range(start: Option<DateTime<Utc>>, end: Option<DateTime<Utc>>) -> String {
    match (start, end) {
        (Some(start), Some(end)) if start == end => rfc3339(start),
        (Some(start), Some(end)) => format!("{}/{}", rfc3339(start), rfc3339(end)),
        (Some(start), None) => format!("{}/..", rfc3339(start)),
        (None, Some(end)) => format!("../{}", rfc3339(end)),
        // `GeoQuery::validate` refuses a temporal predicate with neither bound, so a
        // validated query never reaches this arm.
        (None, None) => String::new(),
    }
}

fn rfc3339(instant: DateTime<Utc>) -> String {
    instant.to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

/// `scope.resources`, the `GeoQuery` member that names which resources a query runs
/// against, read as the STAC collection identifiers to search — a STAC collection *is* a
/// Geoquery resource of type `collection`, so the mapping is not a convention this
/// adapter invented.
///
/// Carries no finding: which collections a request names is a federation/scoping
/// decision, not a capability negotiation, and `geoquery-core`'s `QueryFeature` does not
/// model it for the same reason `QueryScope` is a sibling of the predicate fields in
/// `GeoQuery` rather than one of them.
fn translate_scope(query: &GeoQuery, request: &mut StacSearchRequest) {
    if let Some(Selection::Named(ids)) = query
        .scope
        .as_ref()
        .and_then(|scope| scope.resources.as_ref())
    {
        request.collections.clone_from(ids);
    }
}

fn translate_paging(
    query: &GeoQuery,
    request: &mut StacSearchRequest,
    findings: &mut Vec<CapabilityFinding>,
) {
    request.limit = query.limit;
    match query.offset {
        None | Some(0) => {
            if query.limit.is_some() {
                findings.push(CapabilityFinding {
                    feature: QueryFeature::Paging,
                    support: Support::Pushed,
                });
            }
        }
        Some(_) => {
            // `offset` has no equivalent on a STAC search request in this spike — page
            // tokens are a `next` link, not a skip count — so an offset past zero is
            // refused rather than silently answered from page one.
            findings.push(CapabilityFinding {
                feature: QueryFeature::Paging,
                support: Support::Refused {
                    cause: Cause::Undeclared,
                },
            });
        }
    }
}
