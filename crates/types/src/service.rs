//! What a thing can *do*: an endpoint, how to authenticate to it, and what it supports.
//!
//! [`CapabilitySet`] is the load-bearing type. Every optional flag on it is
//! `Option<bool>` and not `bool`, because "this service did not say" and "this service
//! said no" are different facts, and an engine that conflates them pushes a predicate at
//! a service that may ignore rather than refuse it.

use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

use serde_json::{Map, Value};

use crate::{BoundingBox, ExtensionMap, JsonObject, ScaleRange};

use crate::macros::open_string_enum;

open_string_enum! {
    /// Protocol or access mechanism used by a service.
    pub enum ServiceType {
        /// STAC API.
        Stac => "stac",
        /// OGC API — Records.
        OgcRecords => "ogc-records",
        /// OGC API — Features.
        OgcFeatures => "ogc-features",
        /// Web Feature Service.
        Wfs => "wfs",
        /// `ArcGIS` REST or `GeoServices`.
        ArcgisRest => "arcgis-rest",
        /// NASA Common Metadata Repository.
        Cmr => "cmr",
        /// CKAN.
        Ckan => "ckan",
        /// Direct `PostGIS` connection.
        Postgis => "postgis",
        /// Geoquery native manifests and snapshots.
        Geoquery => "geoquery",
        /// Generic HTTP or `OpenAPI` service.
        GenericHttp => "generic-http",
        /// A known but deliberately unclassified service.
        Unknown => "unknown",
    }
}

open_string_enum! {
    /// Spatial predicate or spatial convenience operation supported by a source.
    pub enum SpatialOperation {
        /// Geometry intersects.
        Intersects => "intersects",
        /// Geometry contains.
        Contains => "contains",
        /// Geometry is within.
        Within => "within",
        /// Geometry touches.
        Touches => "touches",
        /// Geometry overlaps.
        Overlaps => "overlaps",
        /// Geometry crosses.
        Crosses => "crosses",
        /// Geometry is disjoint.
        Disjoint => "disjoint",
        /// Geometry is within a distance.
        DWithin => "dwithin",
        /// Bounding-box intersection.
        Bbox => "bbox",
        /// Nearest-neighbor search.
        Nearest => "nearest",
    }
}

open_string_enum! {
    /// Authentication scheme advertised by a service descriptor.
    pub enum AuthType {
        /// No credentials are required.
        Public => "public",
        /// API-key authentication.
        ApiKey => "api-key",
        /// Bearer-token authentication.
        Bearer => "bearer",
        /// OAuth 2.0 authentication.
        Oauth2 => "oauth2",
        /// HTTP basic authentication.
        Basic => "basic",
        /// A scheme resolved by an extension.
        CustomAuth => "custom",
    }
}

open_string_enum! {
    /// Axis order for a coordinate reference system as actually advertised by a service.
    pub enum AxisOrder {
        /// Latitude, longitude.
        LatLon => "lat-lon",
        /// Longitude, latitude.
        LonLat => "lon-lat",
    }
}

open_string_enum! {
    /// Queryable field type exposed in a service schema.
    pub enum FieldType {
        /// Textual value.
        String => "string",
        /// Floating-point or decimal number.
        Number => "number",
        /// Integer number.
        Integer => "integer",
        /// Boolean value.
        Boolean => "boolean",
        /// Calendar date.
        Date => "date",
        /// Timestamp.
        DateTime => "date-time",
        /// Geometry value.
        Geometry => "geometry",
        /// Arbitrary JSON value.
        Json => "json",
        /// A known but deliberately unclassified field.
        Unknown => "unknown",
    }
}

open_string_enum! {
    /// Dimension kind advertised by a service.
    pub enum DimensionType {
        /// Time dimension.
        Time => "time",
        /// Elevation or height dimension.
        Elevation => "elevation",
        /// A generic custom dimension.
        CustomDimension => "custom",
    }
}

open_string_enum! {
    /// Pagination style used by a service.
    pub enum PagingStyle {
        /// Offset and limit parameters.
        OffsetLimit => "offset-limit",
        /// Page number and page size parameters.
        PageNumber => "page-number",
        /// Cursor token supplied by the service.
        Cursor => "cursor",
        /// Pagination through link headers or link objects.
        LinkHeader => "link-header",
        /// A known but deliberately unclassified pagination style.
        Unknown => "unknown",
    }
}

/// Executable or browsable access mechanism for a resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct ServiceDescriptor {
    /// Optional stable service identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Service protocol or access type.
    pub r#type: ServiceType,
    /// Endpoint URL.
    pub url: String,
    /// Operations and payload semantics advertised by the service.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<CapabilitySet>,
    /// Authentication information without credentials.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authentication: Option<AuthDescriptor>,
    /// Available collection identifiers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collections: Vec<String>,
    /// Queryable fields and their types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<SchemaDescriptor>,
    /// Protocol-specific service metadata.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub metadata: JsonObject,
}

impl ServiceDescriptor {
    /// Create a service descriptor with only the required fields populated.
    #[must_use]
    pub fn new(r#type: ServiceType, url: impl Into<String>) -> Self {
        Self {
            id: None,
            r#type,
            url: url.into(),
            capabilities: None,
            authentication: None,
            collections: Vec::new(),
            schema: None,
            metadata: Map::new(),
        }
    }
}

/// Authentication metadata for a service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct AuthDescriptor {
    /// Authentication scheme.
    pub r#type: AuthType,
    /// Runtime credential profile; never the secret itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// Optional scopes or audience values requested from a credential resolver.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<String>,
    /// Scheme-specific metadata that is safe to store.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub metadata: JsonObject,
}

/// Queryable schema advertised by a service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SchemaDescriptor {
    /// Queryable fields.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<FieldDescriptor>,
    /// Schema-level extension metadata.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub extensions: ExtensionMap,
}

/// One queryable or returned field in a service schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct FieldDescriptor {
    /// Field name in the native service.
    pub name: String,
    /// Field value type.
    pub r#type: FieldType,
    /// Whether the field may be null.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,
    /// Human-readable field description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Enumerated values, if the service advertises them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<Value>,
    /// Field-level extension metadata.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub extensions: ExtensionMap,
}

/// Capabilities and payload semantics discovered for a service or resource.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct CapabilitySet {
    /// Supported spatial predicates and spatial conveniences.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spatial: Vec<SpatialOperation>,
    /// Whether temporal filtering can be pushed down.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temporal: Option<bool>,
    /// Whether attribute filtering can be pushed down.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribute: Option<bool>,
    /// Whether full-text search is supported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_text: Option<bool>,
    /// Whether semantic or vector search is supported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic: Option<bool>,
    /// Whether result sorting is supported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sorting: Option<bool>,
    /// Whether result pagination is supported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pagination: Option<bool>,
    /// Whether bounding-box filtering is supported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<bool>,
    /// Whether exact geometry filtering is supported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry_filter: Option<bool>,
    /// Whether nearest-neighbor search is supported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nearest: Option<bool>,
    /// Whether assets can be downloaded through the service.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download: Option<bool>,
    /// Whether map rendering is supported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendering: Option<bool>,
    /// Advertised coordinate reference systems with axis order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub crs: Vec<CrsDescriptor>,
    /// Advertised spatial extents.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bboxes: Vec<BoundingBox>,
    /// Advertised output formats.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub formats: Vec<String>,
    /// Named styles advertised by protocols that support rendering.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub styles: Vec<String>,
    /// Time, elevation, and custom dimensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dimensions: Vec<DimensionDescriptor>,
    /// Pagination semantics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paging: Option<PagingDescriptor>,
    /// Minimum and maximum scale denominator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_range: Option<ScaleRange>,
    /// Capability extension metadata.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub extensions: ExtensionMap,
}

/// Coordinate reference system advertised by a service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CrsDescriptor {
    /// CRS identifier, such as `EPSG:4326`.
    pub code: String,
    /// Axis order the service expects for that CRS.
    pub axis_order: AxisOrder,
}

/// One advertised service dimension, such as time or elevation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct DimensionDescriptor {
    /// Dimension name in the service.
    pub name: String,
    /// Dimension category.
    pub r#type: DimensionType,
    /// Default dimension value, if advertised.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    /// Advertised dimension values or range description.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<Value>,
    /// Unit of measure, when advertised.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

/// Pagination limits and strategy advertised by a service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct PagingDescriptor {
    /// Pagination strategy.
    pub style: PagingStyle,
    /// Default page size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_page_size: Option<u32>,
    /// Maximum page size the service permits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_page_size: Option<u32>,
}
