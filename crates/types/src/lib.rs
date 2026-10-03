//! The canonical query AST and the data model around it.
//!
//! This crate is the API. Everything else in the project — the CLI, the HTTP server, the
//! MCP tools, the TypeScript and Python SDKs — is an adapter to the types here, and the
//! TypeScript definitions are generated from them rather than written twice. That is why
//! it depends on `ts-rs`: a second hand-written copy of a query shape is a second thing to
//! keep correct, and the copy is the one the SDK uses.
//!
//! It depends on nothing inside the workspace. The language is not allowed to depend on
//! the engine that executes it, because the engine is a program and the language is a
//! document format; a type that can only be expressed by asking the engine what it means
//! is a type the clients cannot construct.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};
use ts_rs::TS;

/// A JSON object used for protocol metadata, source properties, and extension fields.
pub type JsonObject = Map<String, Value>;

/// Source-specific data that did not fit the protocol-independent model.
pub type ExtensionMap = JsonObject;

/// A west, south, east, north bounding box in the coordinate reference system that owns it.
pub type BoundingBox = [f64; 4];

/// A minimum and maximum scale denominator advertised by a service.
pub type ScaleRange = [f64; 2];

macro_rules! open_string_enum {
    (
        $(#[$enum_meta:meta])*
        pub enum $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident => $wire:literal,
            )+
        }
    ) => {
        $(#[$enum_meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, TS)]
        #[ts(type = "string")]
        pub enum $name {
            $(
                $(#[$variant_meta])*
                $variant,
            )+
            /// A value that this version of Geoquery does not name yet.
            Custom(String),
        }

        impl $name {
            /// Return the exact string representation used in JSON documents.
            #[must_use]
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $wire,)+
                    Self::Custom(value) => value.as_str(),
                }
            }

            fn from_wire(value: &str) -> Self {
                match value {
                    $($wire => Self::$variant,)+
                    other => Self::Custom(other.to_owned()),
                }
            }

            fn from_string(value: String) -> Self {
                match value.as_str() {
                    $($wire => Self::$variant,)+
                    _ => Self::Custom(value),
                }
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::from_wire(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::from_string(value)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct OpenEnumVisitor;

                impl Visitor<'_> for OpenEnumVisitor {
                    type Value = $name;

                    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                        formatter.write_str(concat!("a string containing a ", stringify!($name), " value"))
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Ok($name::from_wire(value))
                    }

                    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Ok($name::from_string(value))
                    }
                }

                deserializer.deserialize_str(OpenEnumVisitor)
            }
        }
    };
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<DateTime<Utc>>,
    /// Inclusive end instant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<DateTime<Utc>>,
    /// Additional source-specific metadata.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub metadata: JsonObject,
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
