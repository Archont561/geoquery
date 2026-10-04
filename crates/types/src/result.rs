//! What comes back: one normalized record, whatever protocol produced it.
//!
//! [`Provenance`] is required on every [`GeoResult`] rather than optional. A federated
//! answer that cannot say which source a row came from is not an answer a user can check,
//! so the type makes the unattributed result unrepresentable.

use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

use crate::instant;
use crate::resource::TemporalExtent;
use crate::service::ServiceType;
use crate::{BoundingBox, JsonObject};

use crate::macros::open_string_enum;

open_string_enum! {
    /// Result category returned by a query or discovery operation.
    pub enum ResultType {
        /// Resource discovery result.
        Resource => "resource",
        /// Feature-level query result.
        Feature => "feature",
        /// Asset-level result.
        Asset => "asset",
        /// Context or documentation result.
        Context => "context",
        /// Service discovery result.
        Service => "service",
    }
}

/// Normalized result returned by an adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct GeoResult {
    /// Stable identifier for this result within its source.
    pub id: String,
    /// Resource that produced the result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<ResourceRef>,
    /// Service that was queried.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<ServiceRef>,
    /// Result category.
    pub r#type: ResultType,
    /// Display title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Longer description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// `GeoJSON` geometry, preserved as JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Value>,
    /// Bounding box in west, south, east, north order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<BoundingBox>,
    /// Temporal coverage for the result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temporal: Option<TemporalExtent>,
    /// Attributes, including source-specific fields.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub properties: JsonObject,
    /// Downloadable or related assets.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<Asset>,
    /// Related links.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    /// Optional ranking score in the inclusive range 0.0 to 1.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relevance: Option<f64>,
    /// Provenance is required for every result.
    pub provenance: Provenance,
    /// Original source response, when retention is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<Value>,
}

impl GeoResult {
    /// Create a result with only the required fields populated.
    #[must_use]
    pub fn new(id: impl Into<String>, r#type: ResultType, provenance: Provenance) -> Self {
        Self {
            id: id.into(),
            resource: None,
            service: None,
            r#type,
            title: None,
            description: None,
            geometry: None,
            bbox: None,
            temporal: None,
            properties: Map::new(),
            assets: Vec::new(),
            links: Vec::new(),
            relevance: None,
            provenance,
            raw: None,
        }
    }
}

/// Reference to a resource without embedding the full descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct ResourceRef {
    /// Resource identifier.
    pub id: String,
    /// Optional title copied for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Reference to a service without embedding the full descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct ServiceRef {
    /// Service identifier, when one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Service URL.
    pub url: String,
    /// Service protocol or access type.
    pub r#type: ServiceType,
}

/// Downloadable or browsable asset associated with a result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct Asset {
    /// Asset URL.
    pub href: String,
    /// Media type, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    /// Display title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Roles such as data, thumbnail, metadata, or overview.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    /// Asset-specific metadata.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub metadata: JsonObject,
}

/// Link to a related resource, endpoint, or document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct Link {
    /// Link URL.
    pub href: String,
    /// Link relation.
    pub rel: String,
    /// Media type, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    /// Human-readable title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Provenance attached to every normalized result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(optional_fields)]
pub struct Provenance {
    /// Registry source identifier or adapter source identifier.
    pub source: String,
    /// Service URL that was queried.
    pub service: String,
    /// Native protocol used for the query.
    pub protocol: ServiceType,
    /// Native collection identifier, if one participated in the query.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    /// Native query sent to the source after translation.
    pub query: JsonObject,
    /// Time at which the query was executed.
    #[serde(with = "instant::required")]
    #[ts(type = "string")]
    pub timestamp: DateTime<Utc>,
    /// Duration of the native request in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl Provenance {
    /// Create provenance with the required fields populated.
    #[must_use]
    pub fn new(
        source: impl Into<String>,
        service: impl Into<String>,
        protocol: ServiceType,
        query: JsonObject,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            source: source.into(),
            service: service.into(),
            protocol,
            collection: None,
            query,
            timestamp,
            duration_ms: None,
        }
    }
}
