---
type: Adapter Specification
title: "Adapter Architecture — ServiceAdapter Trait"
description: "ServiceAdapter trait, detection, capability discovery, plugin model."
tags: [adapter, trait, plugin, detection, capability, ServiceAdapter]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: adapters/adapter-architecture
category: adapters
refs: [project/architecture, project/data-model, query/planner, query/query-model, extensions/extension-points]
---

# Adapter Architecture — ServiceAdapter Trait

## Core Principle

Adapters are the **primary extension point** of Geoquery. Every
geospatial protocol is accessed through a `ServiceAdapter`
implementation. The core engine never knows about STAC, OGC, WFS,
or ArcGIS directly — it only knows the trait.

---

## The Trait (Rust)

```rust
#[async_trait]
pub trait ServiceAdapter: Send + Sync {
    /// Can this adapter handle the given endpoint?
    fn detect(&self, endpoint: &Endpoint) -> DetectionResult;

    /// Discover capabilities and metadata from a live service
    async fn describe(
        &self,
        endpoint: &Endpoint,
    ) -> Result<ServiceDescriptor>;

    /// Translate and execute a GeoQuery against this service
    async fn query(
        &self,
        service: &ServiceDescriptor,
        query: &GeoQuery,
    ) -> Result<QueryResult>;

    /// What operations does this service support?
    fn capabilities(
        &self,
        service: &ServiceDescriptor,
    ) -> CapabilitySet;
}
```

### TypeScript Equivalent

A TypeScript equivalent should be available for JS-side adapters:

```typescript
interface ServiceAdapter {
  detect(endpoint: Endpoint): DetectionResult;
  describe(endpoint: Endpoint): Promise<ServiceDescriptor>;
  query(service: ServiceDescriptor, query: GeoQuery): Promise<QueryResult>;
  capabilities(service: ServiceDescriptor): CapabilitySet;
}
```

---

## Detection

When a user runs `geoquery add <url>`, the discovery engine tries
every registered adapter's `detect()` method:

```
geoquery add https://example.org/api
    │
    ▼
StacAdapter::detect()      → checks for /api, /collections, STAC conformance
OgcAdapter::detect()       → checks for OGC API landing page, conformance
WfsAdapter::detect()       → checks for GetCapabilities XML
ArcgisAdapter::detect()    → checks for /rest/services, ArcGIS JSON
NativeAdapter::detect()    → checks for resource.yaml, .md files
    │
    ▼
Best match wins (or multiple if ambiguous)
```

### DetectionResult

```rust
pub enum DetectionResult {
    /// This adapter handles the endpoint with given confidence
    Match { confidence: f64, service_type: ServiceType },
    /// This adapter might handle it but needs more probing
    Uncertain { reason: String },
    /// This adapter definitely does not handle this endpoint
    NoMatch,
}
```

---

## Capability Discovery

Geoquery MUST discover capabilities **before** attempting complex
query translation. The `capabilities()` method returns a
`CapabilitySet` that the planner uses to decide:

1. Which predicates to push to the remote service
2. Which predicates to approximate
3. Which predicates to handle locally

**The planner must never silently pretend unsupported operations
are supported.**

### Discovery Flow

```
geoquery add https://example.org/stac
    │
    ▼
detect() → STAC
    │
    ▼
describe() → fetch /api, /conformance, /collections
    │
    ▼
capabilities() → inspect conformance classes:
    │   "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/core"
    │   "http://www.opengis.net/spec/cql2/1.0/conf/cql2-json"
    │   "https://api.stacspec.org/v1.0.0/item-search#filter"
    │
    ▼
ServiceDescriptor {
    type: "stac",
    url: "https://example.org/stac",
    capabilities: {
        spatial: ["intersects", "bbox"],
        temporal: true,
        attribute: true,    // CQL2 filter extension present
        ...
    },
    collections: ["sentinel-2", "landsat-8"],
    schema: { queryables: {...} }
}
```

→ See [project/data-model](../project/data-model.md) for CapabilitySet definition

### describe() Is the Only Discovery Path

Whatever `describe()` returns is the whole of what Geoquery knows about a
service. It is serialized as a **service snapshot** and reused by the offline
planner, by drift detection, and by client generation — none of which may
rediscover anything on their own. An adapter therefore owes the descriptor every
fact a consumer could need, including payload semantics (CRS and axis order,
formats, styles, dimensions, property schemas), not just the operation flags the
planner reads.

Adapters know nothing about snapshots or generation; they only produce
descriptors. The dependency arrow stays one-way.
→ See [codegen/service-snapshot](../codegen/service-snapshot.md)

---

## Query Execution

The `query()` method receives the canonical `GeoQuery` AST and
returns normalized results:

```
GeoQuery AST
    │
    ▼
Adapter translates to native protocol:
    spatial   → bbox / intersects / CQL2 geometry
    temporal  → datetime parameter
    filters   → CQL2 JSON / SQL WHERE / ArcGIS where
    │
    ▼
HTTP request to remote service
    │
    ▼
Response parsing
    │
    ▼
Normalization to GeoResult[]
    │
    ▼
Provenance attachment
    │
    ▼
QueryResult {
    results: Vec<GeoResult>,
    provenance: Provenance,
    warnings: Vec<String>,
    next_page: Option<String>
}
```

**Key constraint:** Adapters receive a `GeoQuery` and return
`QueryResult`. They never import the planner or executor. The
dependency arrow is strictly one-way:

```
geoquery-core → geoquery-adapter-stac  ✅
geoquery-adapter-stac → geoquery-core  ❌ (only types)
```

Adapters depend on `geoquery-types` for shared data structures,
not on `geoquery-core` for engine logic.

---

## Initial Adapters

| Adapter | Crate | Protocol | Role |
|---------|-------|----------|------|
| STAC API | `geoquery-adapter-stac` | STAC API 1.0 | Core candidate |
| OGC API | `geoquery-adapter-ogc` | OGC API Features + Records | Core candidate |
| Native | `geoquery-adapter-native` | Geoquery YAML/Markdown | Core candidate |
| WFS | `geoquery-adapter-wfs` | WFS 2.0 / 3.0 | Candidate |
| ArcGIS | `geoquery-adapter-arcgis` | ArcGIS REST / GeoServices | Candidate |
| CMR | `geoquery-adapter-cmr` | NASA CMR | Candidate |
| CKAN | `geoquery-adapter-ckan` | CKAN | Candidate |
| PostGIS | `geoquery-adapter-postgis` | Direct SQL | Candidate |
| Generic HTTP | `geoquery-adapter-http` | OpenAPI-described | Candidate |

---

## Plugin Model

Future types must be pluggable. The engine builder accepts adapters
dynamically:

```rust
let engine = GeoqueryEngine::builder()
    .with_adapter(StacAdapter::default())
    .with_adapter(OgcAdapter::default())
    .with_adapter(MyCustomAdapter::new(config))  // user-provided
    .build();
```

A future dynamic-loading design could extend this model:
- Shared libraries (`.so` / `.dylib` / `.dll`)
- WASM plugins (sandboxed, portable)
- `geoquery install adapter-ckan` from a registry

→ See [extensions/extension-points](../extensions/extension-points.md) Layer 1

---

## Error Handling

Adapters must report errors in a structured way that the executor
can handle for partial-failure federation:

```rust
pub enum AdapterError {
    /// Service unreachable or timed out
    ConnectionError { source: String, detail: String },
    /// Service returned an error response
    ServiceError { status: u16, body: String },
    /// Query cannot be translated to this protocol
    UnsupportedQuery { reason: String },
    /// Authentication failed or missing
    AuthError { profile: String },
    /// Response could not be parsed
    ParseError { detail: String },
}
```

The executor catches adapter errors and includes them in the
source status report rather than failing the entire query.

→ See [query/planner](../query/planner.md) for partial failure handling

---

## Related Files

- [adapters/stac](stac.md) — STAC API adapter details
- [adapters/ogc](ogc.md) — OGC API adapter details
- [adapters/native](native.md) — Geoquery native resource format
- [project/data-model](../project/data-model.md) — ServiceDescriptor, CapabilitySet, GeoResult
- [codegen/service-snapshot](../codegen/service-snapshot.md) — How a described service is persisted
- [query/planner](../query/planner.md) — How the planner uses capabilities
- [extensions/extension-points](../extensions/extension-points.md) — Adapter as Layer 1 extension
