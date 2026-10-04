//! The canonical query AST: what a caller asks for, in the one shape every interface shares.
//!
//! The query is the API. A STAC request, an OGC request and a SQL `WHERE` clause are
//! compilation targets of the types here, which is why none of them appears in this file:
//! a field that existed because STAC has it would be a field the OGC adapter has to
//! explain away.
//!
//! The model separates two things that look alike and are not. `scope` and `execution` are
//! *federation* concerns — which sources answer, and how hard the engine tries. `spatial`,
//! `temporal` and the result-shaping fields are *source-level* concerns that each adapter
//! pushes down as far as its service allows. CQL2 solves the second and has nothing to say
//! about the first, which is the whole reason this AST exists rather than a CQL2 document.
//!
//! Attribute filters are deliberately absent: they are GQ-3, and the filter AST is large
//! enough to be its own design. `deny_unknown_fields` on [`GeoQuery`] means a document that
//! sends `filters` today is refused rather than silently answered without them.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use thiserror::Error;
use ts_rs::TS;

use crate::{BoundingBox, ResourceType, SpatialOperation};

open_string_enum! {
    /// How a query interval is compared against a resource's own temporal extent.
    ///
    /// A resource's extent is itself an interval, so every operation here relates two
    /// intervals rather than an interval and a point. An instant query is the degenerate
    /// interval whose bounds are equal.
    pub enum TemporalOperation {
        /// The resource ends before the query interval.
        Before => "before",
        /// The resource starts after the query interval.
        After => "after",
        /// The resource extent falls inside the query interval.
        During => "during",
        /// The resource extent overlaps the query interval at any point.
        Intersects => "intersects",
        /// The resource extent fully covers the query interval.
        Contains => "contains",
        /// The resource extent partly overlaps the query interval.
        Overlaps => "overlaps",
    }
}

open_string_enum! {
    /// The unit a `dwithin` distance is measured in.
    ///
    /// There is no default. A bare number cannot be interpreted — 50000 is a sensible
    /// distance in metres, an implausible one in kilometres and a meaningless one in
    /// degrees — so a distance without a unit is a validation error rather than a guess.
    pub enum DistanceUnit {
        /// Metres.
        Meters => "meters",
        /// Kilometres.
        Kilometers => "kilometers",
        /// Feet.
        Feet => "feet",
        /// Statute miles.
        Miles => "miles",
        /// Nautical miles.
        NauticalMiles => "nautical-miles",
        /// Degrees of the coordinate reference system in use.
        Degrees => "degrees",
    }
}

/// A complete query.
///
/// Every field is optional, and an empty query is the legitimate "everything the registry
/// knows" request rather than a malformed one. What makes a query wrong is a field that
/// contradicts itself or another field, which is [`GeoQuery::validate`]'s subject.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct GeoQuery {
    /// Which sources participate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<QueryScope>,
    /// How the engine should fan the query out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<ExecutionOptions>,
    /// Free-text discovery intent, ranked by the semantic index when one is available.
    ///
    /// Intent only. A geographic or temporal constraint written here instead of in
    /// `spatial` or `temporal` is a constraint an embedding has to reason about, and an
    /// embedding cannot do that reliably; the structured fields exist so it never has to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic: Option<String>,
    /// Geometry predicate pushed down to each source that supports it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spatial: Option<SpatialPredicate>,
    /// Time predicate pushed down to each source that supports it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temporal: Option<TemporalPredicate>,
    /// Result ordering, most significant key first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sort: Vec<SortExpression>,
    /// Maximum number of results to return.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Number of results to skip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    /// Result fields to return, empty meaning the adapter's default projection.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
    /// Optional payloads to include alongside each result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include: Option<IncludeOptions>,
}

impl GeoQuery {
    /// Check every rule that can be checked without asking a source anything.
    ///
    /// This is the trust boundary. An agent-written query reaches a registered service
    /// only after passing here, so the rules are the ones whose violation would otherwise
    /// become a silently wrong answer: coordinates no one can place, a distance with no
    /// unit, an interval that ends before it starts.
    ///
    /// Every problem is reported, not just the first. A caller fixing a generated query
    /// wants the list; a caller fixing a hand-written one is not served by being told
    /// about its mistakes one request at a time.
    pub fn validate(&self) -> Result<(), Vec<QueryValidationError>> {
        let mut problems = Vec::new();
        if let Some(spatial) = self.spatial.as_ref() {
            spatial.collect_problems(&mut problems);
        }
        if let Some(temporal) = self.temporal.as_ref() {
            temporal.collect_problems(&mut problems);
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems)
        }
    }
}

/// Which registered resources and services a query is allowed to reach.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct QueryScope {
    /// Resource identifiers, or every resource the registry knows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<Selection>,
    /// Service identifiers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub services: Vec<String>,
    /// Publishing organisations, matched against resource providers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub providers: Vec<String>,
    /// Resource kinds to consider.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resource_types: Vec<ResourceType>,
    /// User-defined tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Either an explicit list of identifiers or the wildcard that means all of them.
///
/// A separate type rather than an empty list standing in for "everything", because the
/// two have to mean different things: a scope that resolved to nothing must narrow a
/// query to nothing, and a scope the caller never set must not narrow it at all.
#[derive(Debug, Clone, PartialEq, Eq, TS)]
#[ts(type = "string | string[]")]
pub enum Selection {
    /// Every identifier the registry knows, written `"*"`.
    All,
    /// The identifiers named here, and no others.
    Named(Vec<String>),
}

/// The wildcard token, in the one place both halves of the conversion can read it.
const SELECTION_WILDCARD: &str = "*";

impl Serialize for Selection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::All => serializer.serialize_str(SELECTION_WILDCARD),
            Self::Named(names) => names.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Selection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SelectionVisitor;

        impl<'de> Visitor<'de> for SelectionVisitor {
            type Value = Selection;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(r#"a list of identifiers or the string "*""#)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value == SELECTION_WILDCARD {
                    Ok(Selection::All)
                } else {
                    // A bare identifier is refused rather than wrapped in a one-element
                    // list: `"resources": "sentinel-2"` reads like a selection and would
                    // silently become one, and the day a wildcard gains a sibling token
                    // that guess becomes wrong.
                    Err(E::invalid_value(de::Unexpected::Str(value), &self))
                }
            }

            fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut names = Vec::new();
                while let Some(name) = sequence.next_element()? {
                    names.push(name);
                }
                Ok(Selection::Named(names))
            }
        }

        deserializer.deserialize_any(SelectionVisitor)
    }
}

/// Federation hints: how the query runs, never what it means.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct ExecutionOptions {
    /// Whether to fan out across sources at all. Defaults to true when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub federate: Option<bool>,
    /// Cap on how many sources are queried.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_sources: Option<u32>,
    /// Per-source timeout in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    /// Where the work happens. Defaults to [`ExecutionMode::Auto`] when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<ExecutionMode>,
}

/// Where a query is executed.
///
/// Closed, unlike the predicate enums. A predicate names something a service might support
/// and this project has not heard of yet, so an unknown one is data; a mode names something
/// *this engine* does, so an unknown one is a typo, and accepting it would mean a request
/// asking for `"lcoal"` ran against every remote source instead of failing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Push the whole query to each remote service.
    Remote,
    /// Answer from locally held data only.
    Local,
    /// Retrieve candidates remotely and refine them locally.
    Hybrid,
    /// Let the planner choose per source.
    Auto,
}

/// A geometry predicate.
///
/// One struct with an open `op` rather than one variant per operation, because the
/// operation set is open: a source may support a predicate this version has never heard
/// of, and a closed enum would make that query unrepresentable. The cost is that which
/// operands belong to which operation is a rule rather than a type, which is what
/// [`GeoQuery::validate`] is for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct SpatialPredicate {
    /// The predicate to apply.
    pub op: SpatialOperation,
    /// Bounding box in west, south, east, north order, for the `bbox` operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<BoundingBox>,
    /// `GeoJSON` geometry, kept as JSON so a member this version does not model survives
    /// the round trip to the adapter that does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Value>,
    /// Distance for `dwithin`, in [`SpatialPredicate::unit`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance: Option<f64>,
    /// The unit [`SpatialPredicate::distance`] is measured in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<DistanceUnit>,
    /// Coordinate reference system of the coordinates above. WGS84 when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crs: Option<String>,
}

impl SpatialPredicate {
    /// Whether the operation is one that reads [`SpatialPredicate::geometry`].
    ///
    /// `Custom` is excluded deliberately: an operation this version cannot name is one
    /// whose operands it cannot know either, and demanding a geometry for it would refuse
    /// queries a newer source would have answered.
    fn reads_geometry(&self) -> bool {
        matches!(
            self.op,
            SpatialOperation::Intersects
                | SpatialOperation::Contains
                | SpatialOperation::Within
                | SpatialOperation::Touches
                | SpatialOperation::Overlaps
                | SpatialOperation::Crosses
                | SpatialOperation::Disjoint
                | SpatialOperation::DWithin
                | SpatialOperation::Nearest
        )
    }

    fn collect_problems(&self, problems: &mut Vec<QueryValidationError>) {
        if self.reads_geometry() && self.geometry.is_none() {
            problems.push(QueryValidationError::GeometryMissing {
                op: self.op.to_string(),
            });
        }
        if self.op == SpatialOperation::Bbox && self.bbox.is_none() {
            problems.push(QueryValidationError::BoundingBoxMissing);
        }
        if self.op == SpatialOperation::DWithin && self.distance.is_none() {
            problems.push(QueryValidationError::DistanceMissing {
                op: self.op.to_string(),
            });
        }
        if self.distance.is_some() && self.unit.is_none() {
            problems.push(QueryValidationError::DistanceWithoutUnit);
        }
        if self.unit.is_some() && self.distance.is_none() {
            problems.push(QueryValidationError::UnitWithoutDistance);
        }
        if let Some(geometry) = self.geometry.as_ref()
            && let Err(detail) = geojson_geometry_error(geometry)
        {
            problems.push(QueryValidationError::GeometryMalformed { detail });
        }
        if let Some(bbox) = self.bbox.as_ref() {
            self.collect_bbox_problems(*bbox, problems);
        }
        if let Some(crs) = self.crs.as_ref()
            && !crs_is_recognised(crs)
        {
            problems.push(QueryValidationError::CrsUnrecognised { crs: crs.clone() });
        }
    }

    fn collect_bbox_problems(&self, bbox: BoundingBox, problems: &mut Vec<QueryValidationError>) {
        if bbox.iter().any(|ordinate| !ordinate.is_finite()) {
            problems.push(QueryValidationError::BoundingBoxNotFinite);
            return;
        }
        // South above north is wrong in every axis order that calls one of them north,
        // but only once the axes are known to be latitude and longitude at all. Under a
        // declared CRS the ordinates may be northings, eastings or something this crate
        // has no business ranking, so the check is limited to the default. West above
        // east is left alone even here: that is how a box crossing the antimeridian is
        // written.
        let [_west, south, _east, north] = bbox;
        if self.crs.is_none() && south > north {
            problems.push(QueryValidationError::BoundingBoxInverted { south, north });
        }
    }
}

/// A time predicate over an interval, either end of which may be left open.
///
/// An open end is a real query — "everything since 2020" names no upper bound — so the
/// bounds are optional. Both being absent is not: it constrains nothing while looking
/// like it constrains something.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct TemporalPredicate {
    /// The predicate to apply.
    pub op: TemporalOperation,
    /// Inclusive start of the query interval.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "instant")]
    #[ts(type = "string")]
    pub start: Option<DateTime<Utc>>,
    /// Inclusive end of the query interval.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "instant")]
    #[ts(type = "string")]
    pub end: Option<DateTime<Utc>>,
}

impl TemporalPredicate {
    fn collect_problems(&self, problems: &mut Vec<QueryValidationError>) {
        match (self.start, self.end) {
            (None, None) => problems.push(QueryValidationError::IntervalUnbounded {
                op: self.op.to_string(),
            }),
            (Some(start), Some(end)) if start > end => {
                problems.push(QueryValidationError::IntervalInverted { start, end });
            }
            _ => {}
        }
    }
}

/// One ordering key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct SortExpression {
    /// The result field to order by.
    pub field: String,
    /// Which way to order it.
    #[serde(default)]
    pub direction: SortDirection,
}

/// Which way a [`SortExpression`] orders.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum SortDirection {
    /// Smallest first.
    #[default]
    Asc,
    /// Largest first.
    Desc,
}

/// Optional payloads to carry on each result.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct IncludeOptions {
    /// The original source payload, verbatim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<bool>,
    /// Markdown context documents attached to the resource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<bool>,
    /// Downloadable assets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assets: Option<bool>,
}

/// Everything a query can get wrong before a source is contacted.
///
/// One variant per rule, rather than a message, for the same reason
/// `QueryDocumentError` is an enum: the caller is usually a program deciding what to do,
/// and the wording belongs to whoever shows it to a person.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum QueryValidationError {
    /// A geometry predicate arrived without a geometry.
    #[error("spatial operation `{op}` needs a `geometry`")]
    GeometryMissing {
        /// The operation that was asked for.
        op: String,
    },
    /// The `bbox` operation arrived without a bounding box.
    #[error("spatial operation `bbox` needs a `bbox`")]
    BoundingBoxMissing,
    /// A distance predicate arrived without a distance.
    #[error("spatial operation `{op}` needs a `distance`")]
    DistanceMissing {
        /// The operation that was asked for.
        op: String,
    },
    /// A distance was given with no unit to read it in.
    #[error("`distance` without `unit` is meaningless: say meters, feet or degrees")]
    DistanceWithoutUnit,
    /// A unit was given with no distance to apply it to.
    #[error("`unit` without `distance` constrains nothing")]
    UnitWithoutDistance,
    /// The geometry is not valid `GeoJSON`.
    #[error("`geometry` is not valid GeoJSON: {detail}")]
    GeometryMalformed {
        /// What the `GeoJSON` reader objected to.
        detail: String,
    },
    /// A bounding box ordinate is not a finite number.
    #[error("`bbox` ordinates must all be finite numbers")]
    BoundingBoxNotFinite,
    /// A bounding box names a south edge above its north edge.
    #[error("`bbox` has south {south} above north {north}")]
    BoundingBoxInverted {
        /// The south edge as written.
        south: f64,
        /// The north edge as written.
        north: f64,
    },
    /// The coordinate reference system is not in a form this crate can resolve.
    #[error("`{crs}` is not a recognised CRS identifier; use EPSG:<code>, CRS84, or a CRS URI")]
    CrsUnrecognised {
        /// The identifier as written.
        crs: String,
    },
    /// A temporal predicate named neither bound.
    #[error("temporal operation `{op}` needs at least one of `start` or `end`")]
    IntervalUnbounded {
        /// The operation that was asked for.
        op: String,
    },
    /// A temporal interval ends before it starts.
    #[error("temporal interval starts at {start} and ends at {end}")]
    IntervalInverted {
        /// The start as written.
        start: DateTime<Utc>,
        /// The end as written.
        end: DateTime<Utc>,
    },
}

/// Read a value as a `GeoJSON` geometry, reporting why it is not one.
fn geojson_geometry_error(value: &Value) -> Result<(), String> {
    serde_json::from_value::<geojson::Geometry>(value.clone())
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// Whether a CRS identifier is in a form that names an authority and a code.
///
/// Form only. Whether the authority has heard of the code is a question for whatever
/// performs the reprojection, and answering it here would mean this crate carried a copy
/// of the EPSG registry. What it does rule out is the ambiguous string — `"Polish grid"`,
/// `"local"` — which the corpus makes a rejection condition because there is no safe way
/// to read coordinates underneath one.
fn crs_is_recognised(crs: &str) -> bool {
    let lowercase = crs.to_ascii_lowercase();
    if lowercase.starts_with("urn:ogc:def:crs:")
        || lowercase.starts_with("http://")
        || lowercase.starts_with("https://")
    {
        return true;
    }
    if let Some(code) = lowercase.strip_prefix("epsg:") {
        return !code.is_empty() && code.bytes().all(|digit| digit.is_ascii_digit());
    }
    matches!(lowercase.as_str(), "crs84" | "ogc:crs84")
}

/// Instants on the wire: an RFC 3339 timestamp, or a plain `YYYY-MM-DD` date.
///
/// Both appear in real query documents, and a date is the more common of the two by far,
/// so refusing it would make the format worse at the thing people write by hand. A date is
/// read as midnight UTC and *written back* as a timestamp: one shape on output is what
/// makes a round trip stable, and the alternative — remembering which form each bound
/// arrived in — would be state kept solely to reproduce the caller's typing.
mod instant {
    use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    // `&Option<T>` rather than the `Option<&T>` clippy prefers: serde hands `with` a
    // reference to the field, so the signature belongs to serde and not to this module.
    // Rendering to `Option<String>` and letting serde serialize that keeps the `None`
    // case in serde's own impl, where it is already written and already correct — and in
    // practice it never runs, because both bounds are `skip_serializing_if`.
    #[allow(clippy::ref_option)]
    pub(super) fn serialize<S>(
        value: &Option<DateTime<Utc>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value
            .as_ref()
            .map(|instant| instant.to_rfc3339_opts(SecondsFormat::Secs, true))
            .serialize(serializer)
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let Some(text) = Option::<String>::deserialize(deserializer)? else {
            return Ok(None);
        };
        read(&text).map(Some).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "`{text}` is neither an RFC 3339 timestamp nor a YYYY-MM-DD date"
            ))
        })
    }

    fn read(text: &str) -> Option<DateTime<Utc>> {
        if let Ok(instant) = DateTime::parse_from_rfc3339(text) {
            return Some(instant.with_timezone(&Utc));
        }
        NaiveDate::parse_from_str(text, "%Y-%m-%d")
            .ok()?
            .and_hms_opt(0, 0, 0)
            .map(|naive| naive.and_utc())
    }
}
