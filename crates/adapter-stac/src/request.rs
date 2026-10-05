//! `GeoQuery` → STAC Item Search, decided against what the service said it conforms to.
//!
//! [`translate`] builds the request body and, for every `GeoQuery` member it read,
//! returns a [`geoquery_core::CapabilityFinding`] saying what became of it. A member that
//! cannot be sent is never dropped silently and never raises an error — the first would
//! answer a narrow question with a wide answer, and the second would refuse a whole
//! federated run over one predicate one source could not carry.
//!
//! ## What decides whether something is sent
//!
//! Three different kinds of reason, kept apart because they are not the same kind of
//! unresolved:
//!
//! 1. **The protocol has no way to say it.** `bbox` and `datetime` are STAC API core, so
//!    they are always sent; a `before`/`after`/`during` temporal predicate has no
//!    expression in STAC's single `datetime` range, so it is [`Support::Refused`] whatever
//!    the service supports.
//! 2. **The service did not declare it.** Attribute filtering, sorting and field selection
//!    are STAC *extensions*, and a service that does not advertise the matching
//!    conformance class may ignore a parameter it does not implement rather than reject
//!    it — the single worst outcome available, because the answer is then wider than the
//!    question with nothing in the response to show it. [`StacConformance`] is read from
//!    the service's own `conformsTo` list, and an undeclared extension is treated as
//!    absent.
//! 3. **This adapter can do it itself.** Field projection and the `offset` half of paging
//!    are applied to normalized results by [`crate::adapter`], so they are
//!    [`Support::Local`]: correct, and paid for in transfer rather than in precision.
//!
//! [`Support::Approximated`] still appears nowhere. Approximation promises a local
//! follow-up that narrows results ("push a bounding box, then filter the exact geometry
//! against what came back"), and no such step exists in this workspace yet; the local work
//! this adapter *does* perform — trimming fields, cutting a window out of a page sequence
//! — removes nothing a predicate would have kept. Claiming `Approximated` without the
//! narrowing step would be a finding this adapter could not back up, so an exact-geometry
//! predicate is still `Refused`.

use chrono::{DateTime, SecondsFormat, Utc};
use geoquery_core::{CapabilityFinding, Cause, QueryFeature, Support};
use geoquery_types::{
    BoundingBox, GeoQuery, Selection, SortDirection, SpatialOperation, TemporalOperation,
};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::cql2;
use crate::landing::StacConformance;

/// The `filter-lang` this adapter speaks. CQL2 JSON is the only encoding the Filter
/// extension requires a server to accept on a `POST`, and the only one
/// [`crate::cql2`] emits.
const FILTER_LANG: &str = "cql2-json";

/// Members a `fields` request must keep no matter what the caller asked for.
///
/// The Fields extension's `include` list is a *replacement*, not an addition: a service
/// honouring `include: ["eo:cloud_cover"]` to the letter may answer with items that have
/// no `id`, and [`crate::response`] drops an item with no `id` because `GeoResult::id` is
/// not optional. So the floor below — the extension's own recommended default set, plus
/// `collection`, which normalization reads for the resource reference — is prepended to
/// every `include` this adapter sends. Projection the *caller* asked for still happens:
/// it happens locally, where it cannot cost an item its identity.
const REQUIRED_FIELDS: [&str; 9] = [
    "type",
    "stac_version",
    "id",
    "collection",
    "geometry",
    "bbox",
    "links",
    "assets",
    "properties.datetime",
];

/// The STAC Item Search request body this adapter is willing to send.
///
/// `collections` serializes as `[]` when empty rather than being skipped outright by most
/// STAC servers' own convention, but this adapter omits it entirely in that case
/// (`skip_serializing_if` below) — an absent `collections` and STAC's own default both
/// mean "every collection", so there is nothing to gain from sending the empty form. The
/// same reasoning applies to every other optional member here: what is not asked for is
/// not sent, so the request a user reads back in `provenance.query` is exactly the
/// question that was put.
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
    ///
    /// The *page* size, which is not always the caller's `limit`: a query with an
    /// `offset` asks for `offset + limit` items, because STAC counts from the beginning
    /// of the result set and the window is cut out of what comes back.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// The encoding of [`Self::filter`], written out whenever a filter is present.
    ///
    /// Sent explicitly even though `cql2-json` is a `POST` request's default: the default
    /// is the extension's, and a service that reads the member it is given cannot
    /// misread the member it is not.
    #[serde(rename = "filter-lang", skip_serializing_if = "Option::is_none")]
    pub filter_lang: Option<String>,
    /// A CQL2 JSON expression, compiled from [`geoquery_types::GeoQuery::filters`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Value>,
    /// Ordering keys, in the order the query named them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sortby: Vec<StacSortBy>,
    /// The Fields extension's include/exclude hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<StacFields>,
}

/// One entry of the Sort extension's `sortby` array.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StacSortBy {
    /// The field to order by, exactly as the query named it.
    ///
    /// Not rewritten: the Sort extension leaves it to each implementation whether item
    /// properties are addressed as `properties.eo:cloud_cover` or `eo:cloud_cover`, and
    /// supports either or both. An adapter that picked one would be guessing on the
    /// caller's behalf at something the caller can state exactly, and the spec says a
    /// service should answer a field it cannot sort on with a 400 — a loud failure, which
    /// is the outcome this project prefers to a quietly unsorted answer.
    pub field: String,
    /// Which way to order.
    pub direction: StacSortDirection,
}

/// `asc` or `desc`, the two values the Sort extension's `direction` takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StacSortDirection {
    /// Smallest first.
    Asc,
    /// Largest first.
    Desc,
}

impl From<SortDirection> for StacSortDirection {
    fn from(direction: SortDirection) -> Self {
        match direction {
            SortDirection::Asc => Self::Asc,
            SortDirection::Desc => Self::Desc,
        }
    }
}

/// The Fields extension's request member.
///
/// Only `include` is ever populated: `GeoQuery::fields` names what a caller wants, and
/// the extension's own semantics say an `include` list already excludes everything else.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct StacFields {
    /// Fields to return: the members a normalized result cannot survive without,
    /// followed by the caller's own list.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<String>,
}

impl StacSearchRequest {
    /// This request as the JSON object [`geoquery_types::Provenance::query`] expects.
    ///
    /// # Panics
    ///
    /// Never, in practice: every field here is a primitive, a string, a vector of strings
    /// or a `serde_json::Value`, all of which serialize to JSON unconditionally. The
    /// `expect` exists because [`serde::Serialize`] cannot promise that statically, not
    /// because this type is expected to fail it.
    #[must_use]
    pub fn to_json_object(&self) -> Map<String, Value> {
        match serde_json::to_value(self).expect("a StacSearchRequest always serializes") {
            Value::Object(object) => object,
            other => unreachable!("a struct serializes to a JSON object, got {other:?}"),
        }
    }
}

/// Build the request this adapter will send for `query` against a service with these
/// declared conformance classes, and report what will not reach it.
///
/// The returned findings cover every `GeoQuery` member this function read, in the order
/// the query names them, so a caller building `source status` output can present them
/// without re-deriving which features were even in play.
#[must_use]
pub fn translate(
    query: &GeoQuery,
    conformance: StacConformance,
) -> (StacSearchRequest, Vec<CapabilityFinding>) {
    let mut request = StacSearchRequest::default();
    let mut findings = Vec::new();

    translate_spatial(query, &mut request, &mut findings);
    translate_temporal(query, &mut request, &mut findings);
    translate_scope(query, &mut request);
    translate_filter(query, conformance, &mut request, &mut findings);
    translate_sort(query, conformance, &mut request, &mut findings);
    translate_paging(query, &mut request, &mut findings);
    translate_fields(query, conformance, &mut request, &mut findings);

    if query.semantic.is_some() {
        findings.push(not_implemented(QueryFeature::Semantic));
    }

    (request, findings)
}

/// A feature this adapter version does not implement: not pushed to the service, and not
/// applied locally either, because there is no local step for it here.
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

/// Why an extension-backed feature is not being pushed to *this* service.
///
/// [`Cause::Declined`] is "the service described its capabilities and this was not among
/// them", which is precisely what a `conformsTo` list missing a class means — a STAC API
/// that reached this adapter has, by definition, published one. The exception is a
/// descriptor carrying no recognised class at all: that service either said nothing this
/// adapter understood or was registered by a build that did not record it, and
/// [`Cause::Undeclared`] is the honest answer for both.
const fn not_declared(conformance: StacConformance) -> Cause {
    if conformance.declares_nothing() {
        Cause::Undeclared
    } else {
        Cause::Declined
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
        // narrower-than-correct request — see the module docs on why this adapter does
        // not approximate them instead. No filter is sent, which is always a safe
        // superset: an unfiltered search drops no true match.
        //
        // CQL2's temporal functions (`t_before`, `t_during`, …) are the eventual home for
        // these, and the filter half of this module is now the place they would be
        // compiled into. They are deliberately not translated yet: Planetary Computer,
        // the one service in the corpus that declares CQL2 at all, declares `basic-cql2`
        // without `temporal-functions`, so the first implementation would be gated into
        // never running against any service this project has met.
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

/// `filters` → the Filter extension's `filter` / `filter-lang` pair, when the service
/// declared it can read one.
///
/// All or nothing, deliberately, and `geoquery-core` models it that way too
/// ([`QueryFeature::AttributeFilter`] is one feature, not one per leaf): a service either
/// evaluates the expression or it does not, and there is no useful outcome in which half
/// a boolean tree runs remotely. Sending the half a service understands would change what
/// the query *means* — dropping one branch of an `or` narrows the answer, dropping one
/// branch of an `and` widens it — so a tree with one unsupported operator in it is refused
/// whole.
fn translate_filter(
    query: &GeoQuery,
    conformance: StacConformance,
    request: &mut StacSearchRequest,
    findings: &mut Vec<CapabilityFinding>,
) {
    let Some(filter) = &query.filters else {
        return;
    };

    let pushable = if cql2::needs_advanced_comparison(filter) {
        conformance.pushes_advanced_filter()
    } else {
        conformance.pushes_basic_filter()
    };

    if pushable {
        request.filter = Some(cql2::compile(filter));
        request.filter_lang = Some(FILTER_LANG.to_owned());
        findings.push(CapabilityFinding {
            feature: QueryFeature::AttributeFilter,
            support: Support::Pushed,
        });
    } else {
        // Not `Local`: evaluating the predicate against what came back is exactly the
        // local post-processing step this workspace does not have. Not `Approximated`
        // either, for the same reason — an approximation owes a narrowing follow-up.
        findings.push(CapabilityFinding {
            feature: QueryFeature::AttributeFilter,
            support: Support::Refused {
                cause: not_declared(conformance),
            },
        });
    }
}

/// `sort` → the Sort extension's `sortby`, when the service declared it.
///
/// Not sorted locally when it is not declared, even though the results are in memory by
/// the time anyone could: a source returns the items it chose, and ordering one page of
/// an unordered selection produces a sorted list of the wrong items. "The ten least
/// cloudy scenes" is not "ten scenes, sorted by cloud cover", and a local sort would
/// silently turn the first into the second. Refusing says so.
fn translate_sort(
    query: &GeoQuery,
    conformance: StacConformance,
    request: &mut StacSearchRequest,
    findings: &mut Vec<CapabilityFinding>,
) {
    if query.sort.is_empty() {
        return;
    }
    if conformance.sort {
        request.sortby = query
            .sort
            .iter()
            .map(|sort| StacSortBy {
                field: sort.field.clone(),
                direction: sort.direction.into(),
            })
            .collect();
        findings.push(CapabilityFinding {
            feature: QueryFeature::Sort,
            support: Support::Pushed,
        });
    } else {
        findings.push(CapabilityFinding {
            feature: QueryFeature::Sort,
            support: Support::Refused {
                cause: not_declared(conformance),
            },
        });
    }
}

/// `limit` and `offset` → the page this adapter asks for.
///
/// STAC pages forward with an opaque `next` link and has no skip count, so an `offset` is
/// served the only way the protocol allows: ask for `offset + limit` items — across as
/// many pages as that takes, which [`crate::adapter`] walks — and discard the first
/// `offset` once they are in hand. That is genuinely [`Support::Local`] work, and it is
/// reported as such rather than as `Pushed`, because it costs the transfer of every item
/// the caller skipped.
fn translate_paging(
    query: &GeoQuery,
    request: &mut StacSearchRequest,
    findings: &mut Vec<CapabilityFinding>,
) {
    let offset = query.offset.unwrap_or(0);
    request.limit = query.limit.map(|limit| limit.saturating_add(offset));

    if offset > 0 {
        findings.push(CapabilityFinding {
            feature: QueryFeature::Paging,
            support: Support::Local {
                // The protocol's limitation, not this service's: no STAC API declares an
                // offset parameter, because the specification has none to declare.
                cause: Cause::Undeclared,
            },
        });
    } else if query.limit.is_some() {
        findings.push(CapabilityFinding {
            feature: QueryFeature::Paging,
            support: Support::Pushed,
        });
    }
}

/// `fields` → the Fields extension's `include` hint, and always a local trim.
///
/// The hint is sent when the service declared the extension, because transferring fields
/// nobody asked for is the cost this feature exists to avoid. It is still only a hint:
/// the extension says in as many words that `include`/`exclude` are "not a contract about
/// what the response will be" and that a service may return fields it was not asked for.
/// So [`crate::response::project_fields`] trims the normalized results either way, and
/// what the finding reports is which of the two paid for it — [`Support::Pushed`] when
/// the service was asked and the local trim is only making the hint exact,
/// [`Support::Local`] when the whole item came over the wire and this adapter did all of
/// the work.
fn translate_fields(
    query: &GeoQuery,
    conformance: StacConformance,
    request: &mut StacSearchRequest,
    findings: &mut Vec<CapabilityFinding>,
) {
    if query.fields.is_empty() {
        return;
    }
    if conformance.fields {
        let mut include: Vec<String> = REQUIRED_FIELDS
            .iter()
            .map(|&name| name.to_owned())
            .collect();
        for field in &query.fields {
            if !include.contains(field) {
                include.push(field.clone());
            }
        }
        request.fields = Some(StacFields { include });
    }
    findings.push(CapabilityFinding {
        feature: QueryFeature::FieldSelection,
        support: if conformance.fields {
            Support::Pushed
        } else {
            Support::Local {
                cause: not_declared(conformance),
            }
        },
    });
}
