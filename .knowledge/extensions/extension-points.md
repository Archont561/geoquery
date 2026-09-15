---
id: extensions/extension-points
title: Extension Points — All 14 Layers
category: extensions
tags: [extensions, traits, plugins, adapters, storage, auth, ranking, embeddings, geocoders, transforms, events, MCP]
refs: [project/architecture, project/data-model, adapters/adapter-architecture, query/planner, infrastructure/storage]
status: draft
created: 2025-07-11
updated: 2025-07-11
---

# Extension Points — All 14 Layers

## The Golden Rule

> **Every extension point should be a trait with a default
> implementation.**

Users who don't need custom ranking, geocoding, or auth should never
have to think about those traits. Defaults are sensible; extension is
a single `.with_xxx()` call on the engine builder.

```rust
// Minimal — everything uses defaults
let engine = GeoqueryEngine::builder()
    .with_adapter(StacAdapter::default())
    .build();

// Fully customized — every extension point exercised
let engine = GeoqueryEngine::builder()
    .with_adapter(StacAdapter::default())
    .with_adapter(MyCustomAdapter::new())
    .with_registry_store(PostgresStore::new(pool))
    .with_spatial_index(PostGisIndex::new(pool))
    .with_text_index(TantivyIndex::open(path))
    .with_auth_resolver(VaultResolver::new(client))
    .with_ranker(DomainSpecificRanker::new())
    .with_embedding_provider(OpenAiEmbeddings::new(key))
    .with_geocoder(NominatimGeocoder::default())
    .with_transformer(MyEnrichmentTransformer::new())
    .with_event_listener(TelemetryListener::new(exporter))
    .build();
```

Both must compile and work. That's the test of a good extension
architecture.

---

## Layer 1: Protocol Adapters

**What:** Adding support for new geospatial protocols and data sources.
**Mechanism:** `ServiceAdapter` trait.
**MVP:** ✅ Yes.

```rust
#[async_trait]
pub trait ServiceAdapter: Send + Sync {
    fn detect(&self, endpoint: &Endpoint) -> DetectionResult;
    async fn describe(&self, endpoint: &Endpoint) -> Result<ServiceDescriptor>;
    async fn query(&self, service: &ServiceDescriptor, query: &GeoQuery) -> Result<QueryResult>;
    fn capabilities(&self, service: &ServiceDescriptor) -> CapabilitySet;
}
```

**Built-in:** STAC, OGC Features, OGC Records, Native YAML/Markdown.
**Community/future:** WFS, ArcGIS, CMR, CKAN, PostGIS, DuckDB, OData,
FlatGeobuf, custom internal APIs.

**Constraint:** Adapters depend on `geoquery-types` only, never on
`geoquery-core`. Dependency arrow is strictly one-way.

→ See [adapters/adapter-architecture](../adapters/adapter-architecture.md)

---

## Layer 2: Resource & Service Types

**What:** New categories of geospatial resources.
**Mechanism:** Open enums with `Unknown(String)` variant.
**MVP:** ✅ Yes. **Difficulty:** Easy.

```rust
pub enum ResourceType {
    Dataset, Collection, FeatureCollection, Catalog, Service,
    Map, Coverage, Asset, Model, Process, Documentation, Context,
    Unknown(String),  // ← extension
}

pub enum ServiceType {
    Stac, OgcRecords, OgcFeatures, Wfs, ArcgisRest, Cmr, Ckan,
    Postgis, Geoquery, GenericHttp,
    Unknown(String),  // ← extension
}
```

A user might register `type: "ml-model"` or `"digital-twin"`. The
system stores it, indexes metadata, returns it in queries — without
crashing.

---

## Layer 3: Storage Backends

**What:** Different persistence layers for registry, index, and cache.
**Mechanism:** Storage traits with tiered implementations.
**MVP:** ✅ Yes. **Difficulty:** Medium.

```rust
#[async_trait]
pub trait RegistryStore: Send + Sync {
    async fn get_resource(&self, id: &str) -> Result<Option<ResourceDescriptor>>;
    async fn put_resource(&self, resource: &ResourceDescriptor) -> Result<()>;
    async fn delete_resource(&self, id: &str) -> Result<()>;
    async fn list_resources(&self, filter: &ResourceFilter) -> Result<Vec<ResourceDescriptor>>;
}

#[async_trait]
pub trait SpatialIndex: Send + Sync {
    async fn insert(&self, id: &str, bbox: &BBox) -> Result<()>;
    async fn query_bbox(&self, bbox: &BBox) -> Result<Vec<String>>;
    async fn query_intersects(&self, geometry: &Geometry) -> Result<Vec<String>>;
}

#[async_trait]
pub trait TextIndex: Send + Sync {
    async fn index_document(&self, id: &str, text: &str) -> Result<()>;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>>;
}
```

**Built-in backends:**

| Tier | Registry | Spatial | Text |
|------|----------|---------|------|
| 0 | In-memory `HashMap` | Linear scan | None |
| 1 | SQLite (`rusqlite`) | `rstar` R-tree | `tantivy` |
| 2 | DuckDB | GeoParquet + DuckDB | `tantivy` |
| 3 | PostgreSQL | PostGIS | `tantivy` / Elasticsearch |

→ See [infrastructure/storage](../infrastructure/storage.md)

---

## Layer 4: Query Operations & Predicates

**What:** New spatial, temporal, or attribute operations.
**Mechanism:** Open enums with `Custom(String)` variant.
**MVP:** ✅ Yes. **Difficulty:** Easy.

```rust
pub enum SpatialOp {
    Intersects, Within, Contains, BBox, DWithin, Nearest,
    Touches, Overlaps, Crosses, Disjoint,
    Custom(String),  // ← extension
}

pub enum TemporalOp {
    Before, After, During, Intersects, Contains, Overlaps,
    Custom(String),  // ← extension
}
```

The planner checks the target adapter's `CapabilitySet`. If an adapter
declares `Custom("viewshed")`, the planner passes it through. If not,
it rejects or degrades.

---

## Layer 5: Capability Extensions

**What:** Non-standard service capabilities.
**Mechanism:** Extension map on `CapabilitySet`.
**MVP:** ✅ Yes. **Difficulty:** Easy.

```rust
pub struct CapabilitySet {
    pub spatial: Vec<SpatialOp>,
    pub temporal: bool,
    // ... standard fields ...
    pub extensions: HashMap<String, serde_json::Value>,  // ← extension
}
```

**Example:** A LiDAR point cloud service:
```json
{
  "extensions": {
    "point_cloud": true,
    "max_points_per_query": 1000000,
    "supported_formats": ["laz", "copc"],
    "elevation_queries": true
  }
}
```

---

## Layer 6: Authentication Providers

**What:** Different credential resolution mechanisms.
**Mechanism:** `AuthResolver` trait.
**MVP:** ✅ Yes. **Difficulty:** Easy.

```rust
#[async_trait]
pub trait AuthResolver: Send + Sync {
    async fn resolve(&self, profile: &str) -> Result<AuthCredentials>;
}

pub enum AuthCredentials {
    ApiKey(String),
    BearerToken(String),
    Basic { username: String, password: String },
    OAuth2 { token: String, expires_at: DateTime<Utc> },
    CustomHeaders(HashMap<String, String>),
    None,
}
```

**Built-in resolvers:**
- Environment variables (`GEOQUERY_AUTH_COPERNICUS=...`)
- Config file (`~/.geoquery/credentials.yaml`)
- Cloud provider metadata (AWS IAM, GCP service accounts)

**Constraint:** Credentials MUST NOT be stored in public resource
manifests. Only profile references.

→ See [infrastructure/deployment](../infrastructure/deployment.md) for security considerations

---

## Layer 7: Ranking Strategies

**What:** Customizing result scoring and ordering.
**Mechanism:** `Ranker` trait.
**MVP:** Phase 2. **Difficulty:** Medium.

```rust
pub trait Ranker: Send + Sync {
    fn score(&self, result: &GeoResult, query: &GeoQuery) -> f64;
    fn compare(&self, a: &GeoResult, b: &GeoResult, query: &GeoQuery) -> Ordering;
}
```

**Built-in rankers:**
- `DefaultRanker` — weighted: semantic × spatial × temporal × quality
- `SourcePriorityRanker` — prefers certain providers
- `FreshnessRanker` — prefers recent data

**Extension example:** Domain-specific ranker preferring high-resolution
datasets for urban planning queries.

**Design constraint:** Keep ranking modular. Do not hard-code a single
embedding provider.

---

## Layer 8: Semantic / Embedding Providers

**What:** Different embedding models for semantic search.
**Mechanism:** `EmbeddingProvider` trait.
**MVP:** Phase 2. **Difficulty:** Easy.

```rust
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    fn dimensions(&self) -> usize;
}
```

**Built-in providers:**
- `LocalEmbeddingProvider` — ONNX Runtime, `all-MiniLM-L6-v2`
- `OpenAiEmbeddingProvider` — `text-embedding-3-small`
- `None` — semantic search disabled (Tier 0 default)

**Key constraint:** Semantic search is always optional. The core query
engine works without any embedding provider. Full-text search
(`tantivy`) covers most discovery needs.

→ See [query/semantic](../query/semantic.md)

---

## Layer 9: Geocoders

**What:** Different place-name resolution services.
**Mechanism:** `Geocoder` trait.
**MVP:** Phase 2. **Difficulty:** Easy.

```rust
#[async_trait]
pub trait Geocoder: Send + Sync {
    async fn resolve(&self, place: &str) -> Result<Vec<GeocodeResult>>;
}

pub struct GeocodeResult {
    pub geometry: Geometry,
    pub bbox: Option<BBox>,
    pub confidence: f64,
    pub display_name: String,
    pub source: String,
}
```

**Built-in geocoders:**
- `NominatimGeocoder` — OpenStreetMap Nominatim (rate-limited, cached)
- `LocalGeocoder` — offline gazetteer from GeoNames
- `None` — geocoding disabled, geometry must be explicit

**Resolution pipeline:**
```
natural-language place
    ↓
geocoder
    ↓
candidate geometries
    ↓
ambiguity resolution (multiple candidates → clarify)
    ↓
GeoQuery spatial predicate
```

The core engine does not hard-code a particular geocoder.

---

## Layer 10: Result Transformers (Post-Processing Pipeline)

**What:** Custom post-processing steps after query execution.
**Mechanism:** Pipeline of `ResultTransformer` implementations.
**MVP:** Phase 2. **Difficulty:** Medium.

```rust
#[async_trait]
pub trait ResultTransformer: Send + Sync {
    fn name(&self) -> &str;
    async fn transform(&self, results: &mut QueryResult, query: &GeoQuery) -> Result<()>;
}
```

**Built-in transformers:**
- `DeduplicationTransformer` — removes duplicate results
- `ProvenanceEnrichmentTransformer` — attaches source metadata
- `BboxRefinementTransformer` — local exact geometry after remote bbox
- `ContextAttachmentTransformer` — attaches markdown context

**Extension example:** Transformer that clips result geometries to a
user's AOI, or enriches results with external catalog metadata.

---

## Layer 11: Transport / Interface Layer

**What:** New ways to interact with the query engine.
**Mechanism:** Thin wrappers around `core::execute()`.
**MVP:** ✅ Yes. **Difficulty:** Easy.

```
geoquery-core::execute(GeoQuery) -> QueryResult
    │
    ├── geoquery-cli      (clap → core)
    ├── geoquery-tui      (ratatui → core)
    ├── geoquery-http     (axum → core)
    ├── geoquery-mcp      (rmcp → core)
    ├── geoquery-wasm     (wasm-bindgen → core)
    ├── @geoquery/client  (HTTP → geoquery-http)
    ├── geoquery (Python) (HTTP → geoquery-http)
    └── geoquery-grpc     (tonic → core)  ← future
```

Adding a new transport never requires modifying core. Deserialize
input → `GeoQuery`, call `core::execute()`, serialize `QueryResult`.

→ See [interfaces/cli](../interfaces/cli.md), [interfaces/http](../interfaces/http.md), [interfaces/mcp](../interfaces/mcp.md),
  [interfaces/tui](../interfaces/tui.md), [interfaces/typescript](../interfaces/typescript.md), [interfaces/python](../interfaces/python.md)

---

## Layer 12: Metadata Extensions

**What:** Preserving source-specific metadata beyond the core model.
**Mechanism:** Extension maps on `ResourceDescriptor` and `GeoResult`.
**MVP:** ✅ Yes. **Difficulty:** Trivial.

```rust
pub struct ResourceDescriptor {
    // ... core fields ...
    pub extensions: HashMap<String, serde_json::Value>,  // ← extension
}

pub struct GeoResult {
    // ... core fields ...
    pub properties: HashMap<String, serde_json::Value>,  // ← extension
    pub raw: Option<serde_json::Value>,                  // ← full source payload
}
```

**Example:** STAC item properties `eo:cloud_cover`, `sat:orbit_state`,
`view:sun_azimuth` don't belong in the core schema but are preserved
in `properties` and available for CQL2 attribute filtering.

---

## Layer 13: Event / Hook System (Lifecycle)

**What:** Observing and intercepting query execution at each stage.
**Mechanism:** Event emitter with typed lifecycle events.
**MVP:** Phase 2. **Difficulty:** Medium.

```rust
pub enum QueryEvent {
    QueryReceived { query: GeoQuery },
    DiscoveryStarted { candidate_count: usize },
    SourceQueried { source: String, status: SourceStatus },
    ResultsReceived { source: String, count: usize },
    DeduplicationComplete { before: usize, after: usize },
    QueryComplete { total: usize, duration: Duration },
    SourceFailed { source: String, error: String },
}

pub trait QueryEventListener: Send + Sync {
    fn on_event(&self, event: &QueryEvent);
}
```

**Use cases:**
- TUI updates source status panel in real time
- HTTP server streams SSE events to client
- Logging/telemetry captures execution metrics
- Custom alerting on critical source failure

→ See [query/planner](../query/planner.md) for the event stream
→ See [interfaces/tui](../interfaces/tui.md) for TUI consumption

---

## Layer 14: MCP Tools & Resources

**What:** Adding new MCP tools beyond the core three.
**Mechanism:** Tool registry on MCP server.
**MVP:** Phase 2. **Difficulty:** Easy.

**Core tools (always present):**
- `geo_query` — execute a query
- `geo_resource` — retrieve resource metadata/context
- `geo_resolve` — resolve place name to geometry

**Optional/extension tools:**
- `geo_sources` — list registered sources and status
- `geo_capabilities` — inspect source capabilities
- `geo_download` — initiate asset download
- `geo_map` — generate static map image
- `geo_compare` — compare two resources
- `geo_validate` — validate a resource manifest

**MCP Resources (context, not operations):**
- `geo://resource/{id}` — full resource metadata
- `geo://resource/{id}/context` — markdown context
- `geo://resource/{id}/provenance` — provenance chain
- `geo://registry` — registry summary

**Design principle:** Do not expose every backend as a separate MCP
tool. The agent interacts with the Geoquery abstraction.

→ See [interfaces/mcp](../interfaces/mcp.md)

---

## Summary Matrix

| # | Extension Point | Mechanism | MVP? | Difficulty |
|---|----------------|-----------|------|------------|
| 1 | Protocol Adapters | `ServiceAdapter` trait | ✅ | Medium |
| 2 | Resource/Service Types | Open enums + `Unknown` | ✅ | Easy |
| 3 | Storage Backends | `RegistryStore` / `SpatialIndex` / `TextIndex` | ✅ | Medium |
| 4 | Query Operations | Open enums + `Custom` | ✅ | Easy |
| 5 | Capability Extensions | `extensions: HashMap` | ✅ | Easy |
| 6 | Auth Providers | `AuthResolver` trait | ✅ | Easy |
| 7 | Ranking Strategies | `Ranker` trait | Phase 2 | Medium |
| 8 | Embedding Providers | `EmbeddingProvider` trait | Phase 2 | Easy |
| 9 | Geocoders | `Geocoder` trait | Phase 2 | Easy |
| 10 | Result Transformers | `ResultTransformer` pipeline | Phase 2 | Medium |
| 11 | Transport Layers | Wrappers around `core::execute()` | ✅ | Easy |
| 12 | Metadata Extensions | `extensions` / `properties` maps | ✅ | Trivial |
| 13 | Event/Hook System | `QueryEventListener` trait | Phase 2 | Medium |
| 14 | MCP Tools | Tool registry on MCP server | Phase 2 | Easy |

---

## Related Files

- [project/architecture](../project/architecture.md) — Where extensions plug in
- [project/data-model](../project/data-model.md) — Types being extended
- [adapters/adapter-architecture](../adapters/adapter-architecture.md) — Layer 1 in detail
- [query/planner](../query/planner.md) — Layers 7, 10, 13 in action
- [infrastructure/storage](../infrastructure/storage.md) — Layer 3 in detail
- [interfaces/mcp](../interfaces/mcp.md) — Layer 14 in detail
- [query/semantic](../query/semantic.md) — Layers 8, 9 in detail
