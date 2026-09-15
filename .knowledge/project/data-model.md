---
id: project/data-model
title: Core Data Model
category: project
tags: [ResourceDescriptor, ServiceDescriptor, CapabilitySet, GeoResult, types, model]
refs: [project/overview, project/architecture, adapters/adapter-architecture, query/query-model, extensions/extension-points]
status: draft
created: 2025-07-11
updated: 2025-07-11
---

# Core Data Model

## Design Principles

1. **Extensible, not exhaustive.** Do not attempt to encode every field
   of ISO 19115, STAC, DCAT, etc. into the core model.
2. **Preserve unknowns.** Source-specific metadata goes into an extension
   area, not discarded.
3. **Open enums.** All type enumerations include an `Unknown(String)`
   variant for forward compatibility.
4. **Protocol-independent.** The data model must not leak STAC, OGC, or
   any other protocol's terminology into core types.

---

## ResourceDescriptor

Every known resource is normalized internally into a `ResourceDescriptor`.

```typescript
interface ResourceDescriptor {
  id: string                          // Unique identifier (URL or URN)
  type: ResourceType                  // Open enum

  title?: string
  description?: string

  spatial?: SpatialExtent             // BBox or geometry
  temporal?: TemporalExtent           // Start/end interval

  themes?: string[]                   // High-level categories
  keywords?: string[]                 // Free-text tags

  provider?: Provider                 // Organization or individual
  license?: License                   // SPDX or URL

  context?: ContextReference[]        // Links to markdown docs
  relationships?: ResourceRelationship[]  // Graph edges

  services?: ServiceDescriptor[]      // How to access this resource
  capabilities?: CapabilitySet        // Aggregate capabilities

  source?: SourceMetadata             // Where this descriptor came from
  extensions?: Record<string, unknown> // ← Extension point
}
```

### Key Fields

**`id`**: Must be globally unique. Typically a URL or URN. Examples:
- `https://example.org/resources/poland-flood-risk`
- `urn:geoquery:local:flood-model-v2`

**`type`**: See [Resource Types](#resource-types) below.

**`spatial`**: Bounding box or full geometry. Uses GeoJSON-compatible
structures. CRS must be explicit for non-WGS84.

**`temporal`**: Start/end interval. Open-ended intervals supported
(`start` only, `end` only, or both).

**`context`**: References to markdown files that provide human- and
LLM-readable context. NOT merely a long description field — independently
indexed for semantic search.
→ See [adapters/native](../adapters/native.md) for markdown context design

**`relationships`**: Graph edges to other resources.
→ See [Resource Relationships](#resource-relationships) below

**`extensions`**: Preserves source-specific metadata that doesn't fit
the core model. Example: STAC `stac_extensions`, ISO 19115 lineage.

---

## Resource Types

Initial resource types (open enum):

| Type | Description |
|------|-------------|
| `dataset` | A collection of related data |
| `collection` | A STAC/OGC collection |
| `feature-collection` | A set of vector features |
| `catalog` | A catalog of other resources |
| `service` | A queryable service |
| `map` | A rendered map or tile set |
| `coverage` | Raster or gridded data |
| `asset` | A single downloadable file |
| `model` | An ML model or analytical model |
| `process` | An executable process (OGC API Processes) |
| `documentation` | Documentation resource |
| `context` | Semantic context only (no queryable service) |
| `unknown` | Unclassified |
| `Custom(String)` | ← Extension point for user-defined types |

**Important:** A resource does not necessarily have a queryable service.
For example, `type: context` may only provide documentation and semantic
knowledge.

---

## ServiceDescriptor

A service describes an executable/access mechanism for a resource.

```typescript
interface ServiceDescriptor {
  id?: string
  type: ServiceType                   // Open enum
  url: string                         // Endpoint URL

  capabilities?: CapabilitySet        // What operations are supported
  authentication?: AuthDescriptor     // How to authenticate
  collections?: string[]              // Available collections
  schema?: SchemaDescriptor           // Queryable fields and types
  metadata?: Record<string, unknown>  // Protocol-specific metadata
}
```

### Service Types

Initial service types (open enum):

| Type | Protocol |
|------|----------|
| `stac` | STAC API |
| `ogc-records` | OGC API — Records |
| `ogc-features` | OGC API — Features |
| `wfs` | WFS 2.0 / 3.0 |
| `arcgis-rest` | ArcGIS REST / GeoServices |
| `cmr` | NASA CMR |
| `ckan` | CKAN |
| `postgis` | Direct PostGIS connection |
| `geoquery` | Geoquery native (YAML/Markdown) |
| `generic-http` | Generic HTTP/OpenAPI |
| `unknown` | Unclassified |
| `Custom(String)` | ← Extension point |

**Future types must be pluggable.** Each maps to a `ServiceAdapter`
implementation.
→ See [adapters/adapter-architecture](../adapters/adapter-architecture.md)

---

## CapabilitySet

Geoquery MUST discover capabilities before attempting complex query
translation. The planner must never silently pretend unsupported
operations are supported.

```typescript
interface CapabilitySet {
  spatial?: SpatialOp[]               // Supported spatial operations
  temporal?: boolean                  // Supports temporal filtering
  attribute?: boolean                 // Supports attribute filtering
  fullText?: boolean                  // Supports full-text search
  semantic?: boolean                  // Supports semantic/vector search
  sorting?: boolean                   // Supports result sorting
  pagination?: boolean                // Supports pagination
  bbox?: boolean                      // Supports bbox filtering
  geometryFilter?: boolean            // Supports exact geometry filtering
  nearest?: boolean                   // Supports nearest-neighbor
  download?: boolean                  // Supports asset download
  rendering?: boolean                 // Supports map rendering
  extensions?: Record<string, unknown> // ← Extension point
}
```

### Spatial Operations

| Operation | Description |
|-----------|-------------|
| `intersects` | Geometry intersects |
| `contains` | Geometry contains |
| `within` | Geometry is within |
| `touches` | Geometry touches |
| `overlaps` | Geometry overlaps |
| `crosses` | Geometry crosses |
| `disjoint` | Geometry is disjoint |
| `dwithin` | Within distance |
| `bbox` | Bounding box intersection |
| `nearest` | Nearest neighbor |

**Not every backend supports every operation.** The planner uses the
`CapabilitySet` to decide:
1. Push the filter to the remote service (if supported)
2. Approximate (e.g., `intersects` → `bbox` if only bbox is supported)
3. Retrieve candidates remotely, refine locally

→ See [query/planner](../query/planner.md) for degradation strategies

### Example Capability Sets

**STAC API (Planetary Computer):**
```json
{
  "spatial": ["intersects", "bbox"],
  "temporal": true,
  "attribute": true,
  "fullText": false,
  "semantic": false,
  "sorting": true,
  "pagination": true,
  "bbox": true,
  "geometryFilter": true
}
```

**Local PostGIS:**
```json
{
  "spatial": ["intersects", "contains", "within", "dwithin", "bbox", "nearest"],
  "temporal": true,
  "attribute": true,
  "fullText": true,
  "semantic": true,
  "sorting": true,
  "pagination": true,
  "bbox": true,
  "geometryFilter": true,
  "nearest": true
}
```

---

## GeoResult

Every adapter MUST map source responses into a common result
representation.

```typescript
interface GeoResult {
  id: string

  resource?: ResourceRef              // Which resource this came from
  service?: ServiceRef                // Which service was queried

  type: ResultType                    // resource | feature | asset | context

  title?: string
  description?: string

  geometry?: GeoJSON.Geometry         // Normalized geometry
  bbox?: BBox                         // Bounding box
  temporal?: TemporalExtent           // Time extent

  properties?: Record<string, unknown> // All attributes (including source-specific)
  assets?: Asset[]                    // Downloadable assets
  links?: Link[]                      // Related links

  relevance?: number                  // Ranking score (0.0–1.0)
  provenance: Provenance              // MUST be present

  raw?: unknown                       // Original source response (optional)
}
```

### Result Types

| Type | Use Case |
|------|----------|
| `resource` | Resource discovery ("what datasets cover this area?") |
| `feature` | Feature querying ("show me all rivers in this area") |
| `asset` | Asset-level results (individual files/items) |
| `context` | Context/documentation results |
| `service` | Service discovery results |

**Same interface, different result type.** A user asking "what datasets
cover this area?" is doing resource discovery. A user asking "show me
all rivers" is doing feature querying.

---

## Provenance

Every remote result MUST preserve provenance. The system must answer:
*"Where did this result come from?"* and *"How was the original user
query translated?"*

```typescript
interface Provenance {
  source: string                      // Registry source ID
  service: string                     // Service URL
  protocol: string                    // "stac", "ogc-features", etc.
  collection?: string                 // Collection name
  query: Record<string, unknown>      // Translated native query
  timestamp: string                   // When the query was executed
  duration_ms?: number                // How long it took
}
```

**Example:**
```json
{
  "provenance": {
    "source": "planetary-computer",
    "service": "https://planetarycomputer.microsoft.com/api/stac/v1",
    "protocol": "stac",
    "collection": "sentinel-2-l2a",
    "query": {
      "bbox": [14.1, 49.0, 24.2, 54.8],
      "datetime": "2025-01-01/2025-12-31",
      "filter": { "op": "<", "args": [{"property": "eo:cloud_cover"}, 10] }
    },
    "timestamp": "2025-07-11T14:30:00Z",
    "duration_ms": 234
  }
}
```

---

## Resource Relationships

Resources form a graph.

```
Flood Risk Dataset
    │
    ├── derived-from → DEM
    ├── uses → River Network
    ├── uses → Land Cover
    ├── served-by → STAC API
    └── documented-by → Flood Modelling Guide
```

### Relationship Types

| Type | Direction | Example |
|------|-----------|---------|
| `related` | Bidirectional | Two complementary datasets |
| `derived-from` | → Source | Flood map derived from DEM |
| `uses` | → Dependency | Model uses river network |
| `used-by` | ← Dependent | DEM used by flood model |
| `served-by` | → Service | Dataset served by STAC |
| `documented-by` | → Doc | Dataset documented by guide |
| `alternative` | Bidirectional | Two versions of same data |
| `supersedes` | → Old | New version replaces old |
| `superseded-by` | ← New | Old version replaced by new |

---

## AuthDescriptor

Authentication is per service. Credentials MUST NOT be stored inside
public resource manifests.

```typescript
interface AuthDescriptor {
  type: "public" | "api-key" | "bearer" | "oauth2" | "basic" | "custom"
  profile?: string                    // Reference to runtime credential store
}
```

**Example in resource.yaml:**
```yaml
authentication:
  profile: copernicus-production
```

The actual secret is supplied by the runtime via an `AuthResolver`.
→ See [extensions/extension-points](../extensions/extension-points.md) for auth resolver trait

---

## Rust Type Mapping

The canonical implementation is in Rust. Key crate dependencies:

| Concept | Rust Type | Crate |
|---------|-----------|-------|
| Geometry | `geo_types::Geometry<f64>` | `geo-types` |
| GeoJSON | `geojson::GeoJson` | `geojson` |
| BBox | `[f64; 4]` or custom | `geo-types` |
| Temporal | `chrono::DateTime<Utc>` | `chrono` |
| Serialization | `serde::Serialize + Deserialize` | `serde` |
| Extensions | `HashMap<String, serde_json::Value>` | `serde_json` |
| Error | `thiserror::Error` | `thiserror` |

All public types derive `Serialize`, `Deserialize`, and `ts_rs::TS`
for automatic TypeScript type generation.

→ See [infrastructure/xtask](../infrastructure/xtask.md) for codegen pipeline
→ See [interfaces/typescript](../interfaces/typescript.md) for generated TS types

---

## Related Files

- [project/overview](overview.md) — Mission and core concept
- [project/architecture](architecture.md) — Component diagram
- [query/query-model](../query/query-model.md) — GeoQuery AST (the query side of the model)
- [adapters/adapter-architecture](../adapters/adapter-architecture.md) — How adapters produce GeoResults
- [adapters/native](../adapters/native.md) — YAML resource manifest format
- [extensions/extension-points](../extensions/extension-points.md) — All extension mechanisms
- [infrastructure/xtask](../infrastructure/xtask.md) — Type generation pipeline
