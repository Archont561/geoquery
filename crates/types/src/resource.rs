//! What a thing *is*: the descriptor for a dataset, a layer, a document or a place.
//!
//! A resource is the subject of a query rather than a way of answering one, which is the
//! line between this module and [`service`](crate::service). The same dataset reachable
//! through three protocols is one resource and three services.

use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

use crate::instant;
use crate::service::{CapabilitySet, ServiceDescriptor, ServiceType};
use crate::{BoundingBox, ExtensionMap, JsonObject};

use crate::macros::open_string_enum;

open_string_enum! {
    /// What kind of real-world or logical resource a descriptor represents.
    pub enum ResourceType {
        /// A collection of related data.
        Dataset => "dataset",
        /// A protocol collection, such as a STAC or OGC collection.
        Collection => "collection",
        /// A set of vector features.
        FeatureCollection => "feature-collection",
        /// A catalog that points at other resources.
        Catalog => "catalog",
        /// A queryable or browsable service.
        Service => "service",
        /// A rendered map or tile set.
        Map => "map",
        /// Raster, gridded, or other coverage data.
        Coverage => "coverage",
        /// A single downloadable file or object.
        Asset => "asset",
        /// A machine-learning or analytical model.
        Model => "model",
        /// An executable process.
        Process => "process",
        /// Documentation as a resource in its own right.
        Documentation => "documentation",
        /// Semantic context without a queryable service.
        Context => "context",
        /// A known but deliberately unclassified resource.
        Unknown => "unknown",
    }
}

open_string_enum! {
    /// Directed relationship from one resource to another.
    pub enum RelationshipType {
        /// General related-resource edge.
        Related => "related",
        /// This resource was derived from the target resource.
        DerivedFrom => "derived-from",
        /// This resource uses the target resource.
        Uses => "uses",
        /// This resource is used by the target resource.
        UsedBy => "used-by",
        /// This resource is served by the target service.
        ServedBy => "served-by",
        /// This resource is documented by the target resource.
        DocumentedBy => "documented-by",
        /// The target resource is an alternative representation or version.
        Alternative => "alternative",
        /// This resource supersedes the target resource.
        Supersedes => "supersedes",
        /// This resource is superseded by the target resource.
        SupersededBy => "superseded-by",
    }
}

/// Human or machine-readable description of a geospatial resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct ResourceDescriptor {
    /// Globally unique resource identifier, normally a URL or URN.
    pub id: String,
    /// Resource category.
    pub r#type: ResourceType,
    /// Short display title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Longer human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Spatial footprint or extent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spatial: Option<SpatialExtent>,
    /// Temporal coverage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temporal: Option<TemporalExtent>,
    /// High-level thematic categories.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub themes: Vec<String>,
    /// Free-text tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    /// Organization or person responsible for the resource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<Provider>,
    /// License under which the resource is available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
    /// Human- or LLM-readable context documents.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<ContextReference>,
    /// Graph relationships to other resources.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relationships: Vec<ResourceRelationship>,
    /// Services through which the resource can be accessed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub services: Vec<ServiceDescriptor>,
    /// Aggregate capabilities across the resource's services.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<CapabilitySet>,
    /// Descriptor source metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceMetadata>,
    /// Source-specific fields preserved without interpretation.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub extensions: ExtensionMap,
}

impl ResourceDescriptor {
    /// Create a descriptor with only the required fields populated.
    #[must_use]
    pub fn new(id: impl Into<String>, r#type: ResourceType) -> Self {
        Self {
            id: id.into(),
            r#type,
            title: None,
            description: None,
            spatial: None,
            temporal: None,
            themes: Vec::new(),
            keywords: Vec::new(),
            provider: None,
            license: None,
            context: Vec::new(),
            relationships: Vec::new(),
            services: Vec::new(),
            capabilities: None,
            source: None,
            extensions: Map::new(),
        }
    }
}

/// Spatial footprint for a resource or result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct SpatialExtent {
    /// Bounding box in west, south, east, north order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<BoundingBox>,
    /// Full `GeoJSON` geometry, preserved as JSON so protocol-specific members survive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Value>,
    /// Coordinate reference system identifier for non-default coordinates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crs: Option<String>,
}

/// Temporal interval with either bound allowed to be open.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct TemporalExtent {
    /// Inclusive start instant.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "instant::optional"
    )]
    #[ts(type = "string")]
    pub start: Option<DateTime<Utc>>,
    /// Inclusive end instant.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "instant::optional"
    )]
    #[ts(type = "string")]
    pub end: Option<DateTime<Utc>>,
}

/// Provider responsible for publishing, producing, or maintaining a resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct Provider {
    /// Provider name.
    pub name: String,
    /// Provider homepage or profile URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Roles such as producer, host, processor, or licensor.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
}

/// License metadata for a resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct License {
    /// SPDX identifier, URL, or local identifier.
    pub id: String,
    /// Human-readable license name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// License text or summary URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Reference to external context used by humans and semantic search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct ContextReference {
    /// URL or repository-relative path to the context document.
    pub href: String,
    /// Relationship between the resource and the context document.
    pub rel: String,
    /// Optional title for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Media type, for example `text/markdown`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}

/// Edge from one resource descriptor to another resource or service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct ResourceRelationship {
    /// Relationship predicate.
    pub relation: RelationshipType,
    /// Target resource or service identifier.
    pub target: String,
    /// Optional human-readable title for the target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Metadata about where a descriptor came from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct SourceMetadata {
    /// Source registry identifier, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Service URL or document URL that produced the descriptor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    /// Protocol used to discover the descriptor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<ServiceType>,
    /// Time at which the descriptor was fetched.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "instant::optional"
    )]
    #[ts(type = "string")]
    pub fetched_at: Option<DateTime<Utc>>,
    /// Additional source-specific metadata.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub metadata: JsonObject,
}
