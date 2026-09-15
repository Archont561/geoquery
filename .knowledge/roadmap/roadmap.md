---
id: roadmap/roadmap
title: Geoquery Roadmap — 7 Phases
category: roadmap
tags: [roadmap, phases, milestones, MVP, critical-path, timeline]
refs: [project/overview, project/architecture, query/planner, interfaces/mcp, infrastructure/storage, extensions/extension-points]
status: draft
created: 2025-07-11
updated: 2025-07-11
---

# Geoquery Roadmap — 7 Phases

## The One Sentence

> **Ship Phase 2 (federated STAC + OGC via CLI) as fast as possible,
> then add MCP — everything else follows.**

---

## Critical Path

The critical path to proving the core value proposition:

```
Phase 0 → Phase 1 → Phase 2 → Phase 4 (MCP only, skip semantic)
```

That's ~20 weeks to a working federated query engine that AI agents
can use. Everything else — TUI, WASM, PyO3, edge, plugins — can be
parallelized or deferred.

---

## Phase 0 — Foundation (Weeks 1–3)

**Goal:** The monorepo compiles, the types are real, and the query
AST round-trips through JSON.

### Deliverables

- [ ] Workspace scaffold with `xtask`, CI, `cargo-deny`, Clippy
- [ ] `geoquery-types` crate with core data model:
  - `GeoQuery` AST (spatial, temporal, semantic, filters, scope, execution)
  - `ResourceDescriptor` + `ServiceDescriptor`
  - `CapabilitySet`
  - `GeoResult` + `Provenance`
  - `ResourceType`, `ServiceType` (open enums)
- [ ] `serde` round-trip tests: GeoQuery → JSON → GeoQuery
- [ ] `ts-rs` annotations on all public types
- [ ] `cargo xtask codegen` generates TypeScript types + JSON schemas
- [ ] CQL2 filter AST (internal representation, not yet a parser)
- [ ] `resource.yaml` schema definition (JSON Schema)

### Exit Demo

```bash
cargo xtask ci          # green
cargo xtask codegen     # produces schemas/mcp/geo_query.json
cat query.json | cargo run --package geoquery-cli -- validate  # "valid"
```

### Key Decisions Locked

- Query AST shape (the API contract for everything downstream)
- Serialization format (GeoJSON geometry, ISO 8601 time)
- Extension mechanism (open enums + `HashMap<String, Value>`)

### Key Risk

AST design churn. Get this right early — every interface depends on it.

→ See [query/query-model](../query/query-model.md), [project/data-model](../project/data-model.md), [infrastructure/monorepo](../infrastructure/monorepo.md)

---

## Phase 1 — Single-Source Engine (Weeks 4–8)

**Goal:** One query against one real STAC API returns normalized
results with provenance.

### Deliverables

- [ ] `geoquery-core` crate:
  - `ServiceAdapter` trait
  - `QueryPlanner` (single-source, no federation yet)
  - `QueryExecutor` (sequential)
  - Result normalization pipeline
- [ ] `geoquery-adapter-stac` crate:
  - STAC API `/search` POST with bbox, datetime, CQL2-JSON filter
  - STAC collection discovery (`/collections`)
  - Capability detection from STAC API conformance
  - STAC Item → `GeoResult` mapping
- [ ] `geoquery-cli` crate:
  - `geoquery add <stac-url>` — discovers collections, registers locally
  - `geoquery query --bbox ... --time ... --limit N`
  - JSON output with provenance
- [ ] Local registry (Tier 0: in-memory, with YAML file persistence)
- [ ] Integration test against Planetary Computer STAC API

### Exit Demo

```bash
geoquery add https://planetarycomputer.microsoft.com/api/stac/v1
geoquery query \
  --bbox 14.1,49.0,24.2,54.8 \
  --time 2024-06-01/2024-08-31 \
  --filter "eo:cloud_cover < 10" \
  --collection sentinel-2-l2a \
  --limit 5

# Returns 5 normalized GeoResults with provenance
```

**This is the first "oh, it works" moment.**

### Key Risk

STAC API inconsistencies across providers (conformance classes,
filter extension variations, pagination).

→ See [adapters/stac](../adapters/stac.md), [interfaces/cli](../interfaces/cli.md)

---

## Phase 2 — Federation (Weeks 9–14)

**Goal:** One query fans out to multiple STAC sources in parallel,
deduplicates, and returns a unified result set.

### Deliverables

- [ ] Query planner upgrade:
  - Multi-source candidate selection
  - Capability-aware query partitioning
  - Parallel execution via `tokio::JoinSet`
  - Timeout and partial failure handling
- [ ] Second adapter: `geoquery-adapter-ogc`
  - OGC API Features `/collections/{id}/items` with bbox, datetime, CQL2
  - OGC API Records for catalog discovery
  - Landing page + conformance detection
- [ ] Deduplication transformer:
  - Exact ID dedup
  - Canonical URL dedup
  - Provenance merging (one result, multiple sources)
- [ ] Result ranking (initial: spatial + temporal relevance)
- [ ] Source status reporting in query response
- [ ] `geoquery sources` CLI command
- [ ] Integration test: same query against Planetary Computer + Earth Search

### Exit Demo

```bash
geoquery add https://planetarycomputer.microsoft.com/api/stac/v1
geoquery add https://earth-search.aws.element84.com/v1
geoquery add https://stac.eurac.edu

geoquery query \
  --bbox 14.1,49.0,24.2,54.8 \
  --time 2024-01-01/2024-12-31 \
  --semantic "satellite imagery" \
  --limit 20

# Unified results from 3 sources
# Source status: planetary-computer ✓, earth-search ✓, eurac ✓
# Deduplicated: 23 → 19 results
```

**This is the "this is a real product" moment.**

### Key Risk

Deduplication accuracy and partial failure UX.

→ See [query/planner](../query/planner.md), [adapters/ogc](../adapters/ogc.md)

---

## Phase 3 — API Layer (Weeks 15–20)

**Goal:** The engine is accessible via HTTP, TypeScript, and Python —
not just the CLI.

### Deliverables

- [ ] `geoquery-http` crate (Axum):
  - `POST /query` — execute GeoQuery
  - `GET /resources` — list registered resources
  - `GET /resources/{id}` — resource detail + context
  - `POST /resources` — register a new resource
  - `GET /services` — list services
  - `POST /resolve` — geocode a place name (stub)
  - JSON error responses with source-level status
- [ ] `@geoquery/client` (TypeScript):
  - `Geoquery` class with HTTP transport
  - Fluent query builder
  - Auto-generated types from `cargo xtask codegen`
  - `toGeoJSON()` helper
- [ ] `geoquery` (Python):
  - `Geoquery` class with `httpx` transport
  - Pydantic models
  - `.to_geopandas()` converter
  - `.to_arrow()` converter (GeoArrow)
- [ ] Docker Compose for local development
- [ ] OpenAPI spec auto-generated from Axum routes

### Exit Demo

```typescript
import { Geoquery } from "@geoquery/client";
const geo = new Geoquery({ url: "http://localhost:8080" });
const results = await geo.query()
  .semantic("flood risk")
  .bbox([14.1, 49.0, 24.2, 54.8])
  .during("2020", "2025")
  .limit(10)
  .execute();
```

```python
from geoquery import Geoquery
geo = Geoquery(url="http://localhost:8080")
gdf = geo.query(semantic="flood risk", bbox=[14.1, 49.0, 24.2, 54.8]).to_geopandas()
gdf.plot()
```

### Key Risk

SDK maintenance burden across three languages.

→ See [interfaces/http](../interfaces/http.md), [interfaces/typescript](../interfaces/typescript.md), [interfaces/python](../interfaces/python.md)

---

## Phase 4 — AI Integration (Weeks 21–26)

**Goal:** AI agents can query Geoquery through MCP, and semantic
search works.

### Deliverables

- [ ] `geoquery-mcp` crate (`rmcp` 3.x):
  - `geo_query` tool — execute structured GeoQuery
  - `geo_resource` tool — retrieve resource metadata + context
  - `geo_resolve` tool — resolve place name to geometry
  - MCP Resources: `geo://resource/{id}`, `geo://registry`
  - Streamable HTTP transport
  - Tool descriptions with JSON Schema from `cargo xtask codegen`
- [ ] Semantic search layer:
  - `EmbeddingProvider` trait + local ONNX provider
  - `tantivy` index for metadata full-text search
  - Hybrid ranking: semantic × spatial × temporal
- [ ] Geocoder adapter:
  - `Geocoder` trait
  - Nominatim geocoder (rate-limited, cached)
  - `--near "Warsaw"` CLI support
- [ ] Natural language → GeoQuery translation examples (prompt templates)
- [ ] Markdown context indexing:
  - `resource.yaml` + `README.md` + `limitations.md` parsed and indexed
  - Context returned via `geo_resource` MCP tool

### Exit Demo

```json
{
  "tool": "geo_query",
  "arguments": {
    "semantic": "datasets useful for flood modelling",
    "spatial": { "op": "dwithin", "geometry": "...", "distance": "50km" },
    "temporal": { "start": "2020-01-01", "end": "2025-01-01" },
    "limit": 10
  }
}
// Returns 10 results with context, provenance, and usage constraints
```

**This is the "agents can use this" moment.**

### Key Risk

Semantic search quality and hallucination risk in NL → GeoQuery.

→ See [interfaces/mcp](../interfaces/mcp.md), [query/semantic](../query/semantic.md)

---

## Phase 5 — Experience (Weeks 27–34)

**Goal:** The system is pleasant to use interactively, not just
programmatically.

### Deliverables

- [ ] `geoquery-tui` crate (ratatui):
  - Read-only result browser
  - Source status dashboard with live federation progress
  - Detail + provenance view
  - Interactive query editing (sub-phase)
  - ASCII geometry rendering (sub-phase)
- [ ] HTTP streaming:
  - SSE endpoint for query progress events
  - `QueryEventListener` trait wired to SSE
  - TUI consumes the same event stream
- [ ] Query history and saved queries
- [ ] Resource manifest validation:
  - `geoquery validate resource.yaml`
  - Schema validation + link checking
- [ ] Graceful degradation:
  - Remote bbox + local exact intersection refinement
  - Capability-aware query rewriting
  - User-visible warnings when approximation is used

### Exit Demo

```bash
geoquery explore  # launches TUI
# User types "flood risk near Warsaw", sees results stream in from
# 5 sources, browses detail, inspects provenance, refines query
```

### Key Risk

Scope creep. The TUI is a rabbit hole.

→ See [interfaces/tui](../interfaces/tui.md), [extensions/extension-points](../extensions/extension-points.md) Layer 13

---

## Phase 6 — Scale & Edge (Weeks 35–44)

**Goal:** Geoquery runs everywhere — laptop to Cloudflare Workers
to Docker cluster.

### Deliverables

- [ ] Storage tier upgrades:
  - Tier 1: SQLite + `rstar` + `tantivy` (local persistent index)
  - Tier 2: DuckDB + GeoParquet (local analytical queries)
  - Tier 3: PostgreSQL + PostGIS (server deployment)
- [ ] GeoParquet caching:
  - `geoquery cache <stac-url>` — pulls metadata into local GeoParquet
  - Query planner routes to local cache when available
  - Cache invalidation policy
- [ ] `geoquery-wasm` crate:
  - Core query planner compiled to WASM
  - Browser-based metadata search
  - DuckDB-WASM integration for local GeoParquet
- [ ] Edge deployment:
  - Cloudflare Workers / Deno Deploy compatible HTTP server
  - Stateless mode (Tier 0) with remote federation only
- [ ] PyO3 native bindings:
  - `geoquery-native` Python package via `maturin`
  - Zero-copy GeoArrow transfer from Rust to Python
- [ ] Additional adapters:
  - ArcGIS REST / GeoServices
  - NASA CMR
  - WFS 2.0
  - CKAN

### Exit Demo

```bash
# Edge
wrangler deploy  # Geoquery on Cloudflare Workers

# Local analytics
geoquery cache https://planetarycomputer.microsoft.com/api/stac/v1
geoquery query --bbox ... --time ...  # 10× faster from local GeoParquet

# Python native
pip install geoquery-native
import geoquery_native as gq  # in-process Rust engine
```

### Key Risk

WASM compatibility constraints on dependencies.

→ See [infrastructure/storage](../infrastructure/storage.md), [infrastructure/deployment](../infrastructure/deployment.md)

---

## Phase 7 — Ecosystem (Weeks 45+)

**Goal:** Geoquery becomes a platform that others extend.

### Deliverables

- [ ] Plugin system:
  - `geoquery-plugin` crate with stable ABI
  - Dynamic adapter loading (shared libraries or WASM plugins)
  - Plugin manifest format
- [ ] Community adapter registry:
  - `geoquery install adapter-ckan`
  - Published adapter crates on crates.io
- [ ] Hosted Geoquery service:
  - Multi-tenant SaaS
  - Organization-wide registries
  - Access control and rate limiting
- [ ] Advanced AI features:
  - Query suggestion / auto-completion
  - Context-aware result explanation
  - Multi-turn query refinement via MCP
- [ ] OGC API — Records server mode:
  - Geoquery exposes its registry as an OGC Records endpoint
  - Other catalogs can federate *from* Geoquery
- [ ] DCAT / GeoDCAT export:
  - Export registry as DCAT catalog for European open data portals
- [ ] Rendering integration:
  - `geo_map` MCP tool generating static map images
  - MapLibre/Leaflet integration examples in TS SDK

### Key Risk

Community adoption. The plugin system is only valuable if people
build plugins.

→ See [extensions/extension-points](../extensions/extension-points.md) Layer 1

---

## Milestone Summary

| Phase | Duration | Exit Demo | Key Risk |
|-------|----------|-----------|----------|
| **0. Foundation** | 3 weeks | Types compile, schemas generate | AST design churn |
| **1. Single Source** | 5 weeks | CLI queries real STAC API | STAC API inconsistencies |
| **2. Federation** | 6 weeks | Multi-source parallel query | Dedup accuracy, partial failures |
| **3. API Layer** | 6 weeks | TS + Python SDKs work | SDK maintenance burden |
| **4. AI / MCP** | 6 weeks | Agent queries via MCP | Semantic search quality |
| **5. Experience** | 8 weeks | TUI + streaming | Scope creep |
| **6. Scale** | 10 weeks | Edge + WASM + PyO3 | WASM compatibility |
| **7. Ecosystem** | Ongoing | Plugin marketplace | Community adoption |

---

## What to Cut If Time Is Tight

| Feature | Cut? | Why |
|---------|------|-----|
| TUI | Yes, defer to Phase 5+ | Cool but not architecturally critical |
| WASM | Yes, defer to Phase 6+ | Complex, narrow audience initially |
| PyO3 | Yes, defer to Phase 6+ | HTTP client is sufficient for MVP |
| Semantic search | Defer embeddings, keep full-text | `tantivy` alone covers 80% of discovery |
| OGC Records server mode | Defer to Phase 7 | Being a *client* is MVP; *server* is ecosystem |
| Plugin system | Defer to Phase 7 | Trait-based extension is sufficient until external contributors |

---

## Build Order for Interfaces

| Phase | What | Why |
|-------|------|-----|
| **1** | Rust core + STAC adapter + CLI | Proves the engine works |
| **2** | OGC adapter + federation | Proves the value proposition |
| **3** | HTTP server + TS client + Python client | Opens the API to applications |
| **4** | MCP server | Opens the API to AI agents |
| **5** | TUI | Interactive exploration demo |
| **6** | WASM + PyO3 | Performance / edge deployment |

The TUI and native bindings are high-value but not architecturally
critical. The HTTP API is the universal integration point — everything
else is a consumer of it.

---

## Related Files

- [project/overview](../project/overview.md) — Mission and core concept
- [project/architecture](../project/architecture.md) — Component diagram
- [query/planner](../query/planner.md) — The engine being built
- [interfaces/mcp](../interfaces/mcp.md) — Phase 4 target
- [infrastructure/storage](../infrastructure/storage.md) — Phase 6 targets
- [extensions/extension-points](../extensions/extension-points.md) — Phase 7 targets
