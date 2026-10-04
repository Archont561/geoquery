//! What a service can be asked to do, and what it cannot.
//!
//! A [`GeoQuery`] describes an intent. A [`ServiceDescriptor`] describes a service that may
//! or may not be able to honour it. This module compares the two and produces a
//! [`CapabilityReport`]: one [`CapabilityFinding`] per feature the query actually uses,
//! saying where that feature will be carried out and, when that is not the service, why.
//!
//! The report exists because of a rule the design corpus states twice and from two
//! directions: the planner must never silently pretend an unsupported operation is
//! supported, and degradation is always reported. Silence is the failure mode worth
//! designing against, because it is invisible. A query for scenes over Warsaw that quietly
//! loses its spatial predicate does not fail — it returns scenes over everywhere, and
//! nothing in the response says so.
//!
//! The classification rests on one fact about this engine: results arrive normalized, as
//! [`GeoResult`](geoquery_types::GeoResult)s carrying geometry, time and properties. That
//! is why almost everything a service declines can still be done locally, and why
//! [`Support::Refused`] is rare rather than routine.

use std::fmt;

use geoquery_types::{
    CapabilitySet, FilterExpr, GeoQuery, SchemaDescriptor, ServiceDescriptor, SpatialOperation,
    TemporalOperation,
};
use serde::{Deserialize, Serialize};

/// One thing a query can ask a service to do.
///
/// These are query features rather than protocol features: the unit a planner can decide
/// to push down, approximate or keep, independently of the others.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum QueryFeature {
    /// A geometry predicate, named by the operation it asks for.
    Spatial(SpatialOperation),
    /// A time predicate, named by the operation it asks for.
    Temporal(TemporalOperation),
    /// Filtering on properties, as a whole. A filter tree is one feature and not one per
    /// leaf, because a service either accepts the expression or it does not — there is no
    /// useful outcome in which half a boolean tree is evaluated remotely.
    AttributeFilter,
    /// Ranking by meaning rather than by matching.
    Semantic,
    /// Matching words against text. Only ever appears as the thing [`Semantic`] was
    /// approximated by, since nothing in [`GeoQuery`] requests it directly.
    ///
    /// [`Semantic`]: QueryFeature::Semantic
    FullText,
    /// Ordering the results.
    Sort,
    /// Asking for a window of the results rather than all of them.
    Paging,
    /// Asking for a subset of each result's fields.
    FieldSelection,
}

impl fmt::Display for QueryFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spatial(op) => write!(f, "spatial `{}`", op.as_str()),
            Self::Temporal(op) => write!(f, "temporal `{}`", op.as_str()),
            Self::AttributeFilter => f.write_str("attribute filtering"),
            Self::Semantic => f.write_str("semantic ranking"),
            Self::FullText => f.write_str("full-text search"),
            Self::Sort => f.write_str("sorting"),
            Self::Paging => f.write_str("paging"),
            Self::FieldSelection => f.write_str("field selection"),
        }
    }
}

/// Why a feature is not being pushed to the service.
///
/// The distinction between [`Declined`](Cause::Declined) and
/// [`Undeclared`](Cause::Undeclared) is the point of this type. Both end with the engine
/// doing the work, so the [`Support`] is identical and only the cause differs — but one is
/// a service that answered the question and the other is a service that was never asked.
/// Only the second is worth re-running discovery over, and only the first is worth
/// reporting to a user as a limitation of the service rather than of our knowledge of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Cause {
    /// The service described its capabilities and this was not among them.
    Declined,
    /// The service said nothing either way.
    ///
    /// Treated exactly like a refusal, and that is deliberate. An undeclared capability is
    /// not a capability: a service handed a parameter it does not understand may *ignore*
    /// it rather than reject it, which turns a narrow question into a wide answer with
    /// nothing in the response to show that it widened.
    Undeclared,
    /// The service filters on properties, but not on this one.
    FieldNotQueryable {
        /// The property named in the filter that the service does not publish as queryable.
        field: String,
    },
}

impl fmt::Display for Cause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Declined => f.write_str("the service declared it does not support it"),
            Self::Undeclared => f.write_str("the service declared nothing about it"),
            Self::FieldNotQueryable { field } => {
                write!(f, "the service does not list `{field}` as queryable")
            }
        }
    }
}

/// What will become of one feature of a query.
///
/// Closed, where [`Cause`] and [`QueryFeature`] are not. The difference is what a caller
/// does with the value: a cause is read, and a support is *acted on* — pushed, applied
/// here, or refused. A new variant absorbed by somebody's wildcard arm would be a feature
/// silently mishandled, which is the one thing this module exists to prevent. Adding one
/// should break every caller, because it does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Support {
    /// The service does this, and will be asked to.
    Pushed,
    /// The service cannot do this exactly, but can do something weaker that is guaranteed
    /// to return a superset. The weaker form is pushed and the exact form is applied to
    /// what comes back.
    ///
    /// The superset guarantee is the whole safety argument. An approximation that could
    /// *drop* a matching result would not be a cheaper answer, it would be a wrong one.
    Approximated {
        /// The weaker feature that is actually sent.
        instead: QueryFeature,
        /// Why the exact form could not be sent.
        cause: Cause,
    },
    /// The engine will do this itself, against the normalized results.
    ///
    /// Correct, and not free: the service returns more than was asked for, so this costs
    /// transfer and, where the feature is `Paging`, makes the result count arrive wrong
    /// until the engine has trimmed it.
    Local {
        /// Why the service will not be asked.
        cause: Cause,
    },
    /// Nobody can do this: not the service, and not the engine afterwards.
    ///
    /// Reaching this is a decision to either drop the feature loudly or fail the query.
    /// It is never a decision to proceed quietly.
    Refused {
        /// Why the service will not be asked.
        cause: Cause,
    },
}

/// One feature of a query, and what will become of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityFinding {
    /// The feature the query asked for.
    pub feature: QueryFeature,
    /// Where it will be carried out, and why there.
    pub support: Support,
}

impl fmt::Display for CapabilityFinding {
    /// One line a person can read, built from the typed value rather than from wording
    /// chosen at the call site, so that every place that reports a degradation reports it
    /// the same way.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.support {
            Support::Pushed => write!(f, "{} is pushed to the service", self.feature),
            Support::Approximated { instead, cause } => {
                write!(f, "{} is approximated by {instead}: {cause}", self.feature)
            }
            Support::Local { cause } => write!(f, "{} is applied locally: {cause}", self.feature),
            Support::Refused { cause } => write!(f, "{} is refused: {cause}", self.feature),
        }
    }
}

/// What one service will and will not do with one query.
///
/// Holds a finding for every feature the query uses and nothing for the features it does
/// not, so an empty report means an empty query rather than an incapable service.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CapabilityReport {
    findings: Vec<CapabilityFinding>,
}

impl CapabilityReport {
    /// Work out what this service will do with this query.
    ///
    /// Takes the whole [`ServiceDescriptor`] rather than just its [`CapabilitySet`]
    /// because the answer depends on both halves: the capabilities say whether property
    /// filtering happens at all, and the schema says whether it can happen on *this*
    /// property. A descriptor with no capabilities is read as a service that has declared
    /// nothing, which is the state every service is in before discovery has run.
    #[must_use]
    pub fn for_query(query: &GeoQuery, service: &ServiceDescriptor) -> Self {
        let nothing_declared = CapabilitySet::default();
        let declared = service.capabilities.as_ref().unwrap_or(&nothing_declared);
        let mut findings = Vec::new();

        if let Some(spatial) = &query.spatial {
            findings.push(CapabilityFinding {
                feature: QueryFeature::Spatial(spatial.op.clone()),
                support: spatial_support(&spatial.op, declared),
            });
        }
        if let Some(temporal) = &query.temporal {
            findings.push(CapabilityFinding {
                feature: QueryFeature::Temporal(temporal.op.clone()),
                support: locally_or_pushed(declared.temporal),
            });
        }
        if let Some(filters) = &query.filters {
            findings.push(CapabilityFinding {
                feature: QueryFeature::AttributeFilter,
                support: filter_support(filters, declared, service.schema.as_ref()),
            });
        }
        if query.semantic.is_some() {
            findings.push(CapabilityFinding {
                feature: QueryFeature::Semantic,
                support: semantic_support(declared),
            });
        }
        if !query.sort.is_empty() {
            findings.push(CapabilityFinding {
                feature: QueryFeature::Sort,
                support: locally_or_pushed(declared.sorting),
            });
        }
        // One finding, not two: a limit and an offset are one window, and a service that
        // can take one can take the other.
        if query.limit.is_some() || query.offset.is_some() {
            findings.push(CapabilityFinding {
                feature: QueryFeature::Paging,
                support: locally_or_pushed(declared.pagination),
            });
        }
        if !query.fields.is_empty() {
            // Nothing in the capability model describes projection, so there is nothing
            // to claim on a service's behalf. Dropping fields from a normalized result is
            // always correct and always cheap, so this is local without further thought.
            findings.push(CapabilityFinding {
                feature: QueryFeature::FieldSelection,
                support: Support::Local {
                    cause: Cause::Undeclared,
                },
            });
        }

        Self { findings }
    }

    /// Every finding, in the order the features appear in a query.
    #[must_use]
    pub fn findings(&self) -> &[CapabilityFinding] {
        &self.findings
    }

    /// Whether the query asked for nothing that needed deciding.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.findings.is_empty()
    }

    /// Whether every feature of the query reaches the service unchanged.
    ///
    /// True for a query that asked for nothing, which is correct: a request with no
    /// predicates has nothing left behind.
    #[must_use]
    pub fn fully_pushed(&self) -> bool {
        self.findings
            .iter()
            .all(|finding| finding.support == Support::Pushed)
    }

    /// The findings for features the service will not carry out as written.
    ///
    /// This is the set worth showing a user, and the set the engine must do something
    /// about — by approximating, by post-processing, or by refusing.
    pub fn not_pushed(&self) -> impl Iterator<Item = &CapabilityFinding> {
        self.findings
            .iter()
            .filter(|finding| finding.support != Support::Pushed)
    }

    /// The findings for features nobody will carry out.
    pub fn refused(&self) -> impl Iterator<Item = &CapabilityFinding> {
        self.findings
            .iter()
            .filter(|finding| matches!(finding.support, Support::Refused { .. }))
    }
}

/// The shape shared by every capability declared as a single optional flag.
///
/// `Some(true)` is the only value that means yes. `None` is not a maybe — see
/// [`Cause::Undeclared`].
fn locally_or_pushed(declared: Option<bool>) -> Support {
    match declared {
        Some(true) => Support::Pushed,
        Some(false) => Support::Local {
            cause: Cause::Declined,
        },
        None => Support::Local {
            cause: Cause::Undeclared,
        },
    }
}

fn spatial_support(wanted: &SpatialOperation, declared: &CapabilitySet) -> Support {
    let cause = if spatial_is_undeclared(declared) {
        Cause::Undeclared
    } else {
        Cause::Declined
    };

    if declares_operation(declared, wanted) {
        Support::Pushed
    } else if is_exact_geometry(wanted) && declares_operation(declared, &SpatialOperation::Bbox) {
        // The worked example from the corpus: a service that cannot intersect a polygon
        // but can filter by bounding box gets the box that contains the polygon, and the
        // polygon is applied to what comes back. Safe only because a bounding box strictly
        // contains its geometry, so the coarse filter cannot drop a true match.
        Support::Approximated {
            instead: QueryFeature::Spatial(SpatialOperation::Bbox),
            cause,
        }
    } else {
        // Every normalized result carries its geometry, so the predicate can still be
        // evaluated here. Results that arrive without one cannot be judged, which is a
        // matter for whoever applies the predicate rather than for this report.
        Support::Local { cause }
    }
}

/// Whether the service has said anything at all about geometry.
fn spatial_is_undeclared(declared: &CapabilitySet) -> bool {
    declared.spatial.is_empty()
        && declared.bbox.is_none()
        && declared.geometry_filter.is_none()
        && declared.nearest.is_none()
}

/// Whether the service declared this operation, by listing it or by setting the coarse
/// flag that covers it.
///
/// Two ways of saying the same thing exist because real services describe themselves both
/// ways: a STAC service lists its filter operations, while a simpler endpoint only admits
/// to taking a `bbox` parameter.
fn declares_operation(declared: &CapabilitySet, wanted: &SpatialOperation) -> bool {
    if declared.spatial.contains(wanted) {
        return true;
    }
    match wanted {
        SpatialOperation::Bbox => declared.bbox == Some(true),
        SpatialOperation::Nearest => declared.nearest == Some(true),
        operation if is_exact_geometry(operation) => declared.geometry_filter == Some(true),
        _ => false,
    }
}

/// Whether this operation tests a geometry exactly, and so has a bounding box that is a
/// safe over-approximation of it.
///
/// `Disjoint` is pointedly absent: the bounding box of a disjoint test selects the wrong
/// side of the predicate, so approximating it would drop true matches rather than add
/// false ones. `DWithin` and `Nearest` are absent because their extent depends on a
/// distance this function cannot see.
fn is_exact_geometry(operation: &SpatialOperation) -> bool {
    matches!(
        operation,
        SpatialOperation::Intersects
            | SpatialOperation::Contains
            | SpatialOperation::Within
            | SpatialOperation::Touches
            | SpatialOperation::Overlaps
            | SpatialOperation::Crosses
    )
}

/// The one feature the engine cannot rescue.
///
/// Ranking by meaning needs an index of meanings. A service either has one or it does
/// not, and in Phase 1 there is nothing on this side to fall back to — so an undeclared
/// semantic capability is a refusal rather than local work. Where the service offers
/// plain text matching, that is pushed instead: it finds a different set, overlapping but
/// neither a superset nor a subset, which is why it is reported rather than substituted
/// silently.
fn semantic_support(declared: &CapabilitySet) -> Support {
    if declared.semantic == Some(true) {
        return Support::Pushed;
    }
    let cause = if declared.semantic.is_some() {
        Cause::Declined
    } else {
        Cause::Undeclared
    };
    if declared.full_text == Some(true) {
        Support::Approximated {
            instead: QueryFeature::FullText,
            cause,
        }
    } else {
        Support::Refused { cause }
    }
}

fn filter_support(
    filters: &FilterExpr,
    declared: &CapabilitySet,
    schema: Option<&SchemaDescriptor>,
) -> Support {
    match declared.attribute {
        Some(true) => match first_unqueryable_field(filters, schema) {
            Some(field) => Support::Local {
                cause: Cause::FieldNotQueryable { field },
            },
            None => Support::Pushed,
        },
        other => Support::Local {
            cause: if other.is_some() {
                Cause::Declined
            } else {
                Cause::Undeclared
            },
        },
    }
}

/// The first property the filter names that the service does not publish as queryable.
///
/// `None` when the service published no schema at all, and that is a deliberate
/// difference from an empty one: no queryables to check against is not evidence that
/// there are none. The filter goes, and a service that then rejects the property reports
/// a real error instead of one this crate guessed on its behalf.
fn first_unqueryable_field(
    filters: &FilterExpr,
    schema: Option<&SchemaDescriptor>,
) -> Option<String> {
    let schema = schema?;
    let queryable = |field: &str| schema.fields.iter().any(|known| known.name == field);
    match filters {
        FilterExpr::And(branches) | FilterExpr::Or(branches) => branches
            .iter()
            .find_map(|branch| first_unqueryable_field(branch, Some(schema))),
        FilterExpr::Not(inner) => first_unqueryable_field(inner, Some(schema)),
        FilterExpr::Compare { field, .. }
        | FilterExpr::In { field, .. }
        | FilterExpr::Like { field, .. } => (!queryable(field)).then(|| field.clone()),
    }
}
