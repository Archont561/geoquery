---
type: Research Findings
title: Rust Crate Research & Findings
description: "Findings for geo, rstar, geoarrow, rmcp 3.x, tantivy, duckdb, axum."
tags: [Rust, crates, geo, rstar, geoarrow, rmcp, tantivy, duckdb, axum, reqwest, tokio]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: research/rust-crates
category: research
refs: [project/architecture, adapters/adapter-architecture, infrastructure/storage, infrastructure/monorepo, interfaces/mcp]
---

# Rust Crate Research & Findings

## Summary

The Rust geospatial ecosystem is mature for core geometry and indexing
but lacks dominant high-level SDKs for STAC and OGC API (unlike
Python's `pystac-client` / `OWSLib`). This means Geoquery adapters
will be built as custom HTTP + `serde` models — which is fine, since
STAC API and OGC API are straightforward REST.

The most important 2026 correction: the official Rust MCP SDK is now
**`rmcp` 3.x**, implementing the MCP 2026-07-28 specification.

---

## Geospatial Core

### `geo-types` + `geo` + `geos` + `proj`

The production geometry stack:

```
geo-types          Interoperable geometry primitives (Point, Polygon, etc.)
    ↓
geo / geos         Algorithms and robust topology predicates
    ↓
proj               CRS transformations and reprojection
    ↓
geojson            GeoJSON serialization/deserialization
```

| Crate | Version | Purpose | Notes |
|-------|---------|---------|-------|
| `geo-types` | 0.7 | Geometry primitives | `Geometry<f64>` is the canonical internal type |
| `geo` | 0.29 | Geometry algorithms | Intersects, contains, distance, area, convex hull |
| `geos` | 9.x | GEOS bindings | Robust topology (buffer, union, intersection) |
| `proj` | 0.28 | PROJ bindings | CRS transforms, `EPSG:2180` → `EPSG:4326` |
| `geojson` | 0.24 | GeoJSON serde | Convert `geo_types` ↔ GeoJSON |

**Decision:** Use `geo-types` as the internal geometry representation.
Use `geo` for pure-Rust spatial operations. Use `geos` only when
robust topology is needed (buffer, complex intersections). Use `proj`
for all CRS transformations.

---

## Spatial Indexing

| Crate | Type | Use Case | Notes |
|-------|------|----------|-------|
| `rstar` | R-tree | Bounding box queries, spatial joins | **Primary choice** for Tier 1 spatial index |
| `kiddo` | KD-tree | Nearest-neighbor point search | Good for point cloud / POI queries |
| `h3o` | H3 grid | Hexagonal spatial indexing | Useful for aggregation, not primary index |
| `s2` | S2 cells | Spherical geometry indexing | Google's S2 library, good for global data |

**Decision:** `rstar` for the primary spatial index. It's the most
mature, supports arbitrary geometry envelopes, and integrates well
with `geo-types`. `kiddo` as an optional secondary for nearest-neighbor
workloads.

---

## Columnar / GeoArrow / GeoParquet

**This area deserves substantially more attention than initially
expected.** Arrow/GeoArrow is the ideal internal interchange format
for a high-performance federation engine.

| Crate | Purpose | Notes |
|-------|---------|-------|
| `geoarrow` | GeoArrow types and operations | Rust implementation of GeoArrow spec |
| `arrow` | Apache Arrow columnar format | Foundation for GeoArrow |
| `parquet` | Parquet file I/O | Read/write GeoParquet via Arrow |
| `datafusion` | SQL query engine over Arrow | Optional: local analytical queries |

**Architecture impact:**

```
Source Adapter → GeoArrow RecordBatch (internal)
                     ↓
              GeoParquet (local cache)
                     ↓
              DuckDB (local analytics)
                     ↓
              GeoJSON (external output only)
```

GeoJSON is the output contract. GeoArrow is the internal interchange.
This avoids serializing/deserializing GeoJSON strings between adapters,
the planner, and the deduplication layer.

**Decision:** Adopt GeoArrow as the internal interchange format in
Phase 2+. Start with GeoJSON in Phase 1 for simplicity, then migrate
the internal pipeline to GeoArrow when performance matters.

---

## DuckDB

| Crate | Purpose | Notes |
|-------|---------|-------|
| `duckdb` | DuckDB Rust bindings | Local analytical SQL engine |

DuckDB is particularly attractive for:
- Local GeoParquet queries (native spatial extension)
- Analytical aggregation over cached STAC metadata
- Tier 2 storage backend
- Arrow integration (zero-copy Arrow ↔ DuckDB)

**Decision:** DuckDB as the Tier 2 analytical backend. Not mandatory
for core — Tier 0 and Tier 1 work without it.

---

## SQLite / PostGIS

| Crate | Purpose | Notes |
|-------|---------|-------|
| `rusqlite` | SQLite bindings | Tier 1 registry storage |
| `sqlx` | Async SQL (Postgres, SQLite) | Tier 3 PostGIS adapter |

**Decision:** Treat as separate storage adapters, not the canonical
data model. `rusqlite` for Tier 1, `sqlx` for Tier 3 PostGIS.

---

## Full-Text Search

| Crate | Purpose | Notes |
|-------|---------|-------|
| `tantivy` | Full-text search engine | **Primary choice** for metadata catalog search |

`tantivy` is the obvious candidate:
- Pure Rust, no external dependencies
- BM25 ranking, faceted search
- Fast indexing and querying
- Can index markdown context documents
- Supports custom tokenizers for geospatial terms

**Decision:** `tantivy` for Tier 1+ text indexing. Covers most
discovery needs even without embeddings.

---

## Vector Search

| Crate | Purpose | Notes |
|-------|---------|-------|
| `hnsw` | HNSW graph index | In-memory vector similarity |
| `qdrant-client` | Qdrant client | External vector DB |

**Decision:** Vector search is an **optional semantic layer**, not
part of the fundamental spatial query engine. Start with `tantivy`
BM25 for text search. Add HNSW or Qdrant in Phase 4 when semantic
search is needed.

---

## MCP — `rmcp` (Critical 2026 Update)

| Crate | Version | Spec | Notes |
|-------|---------|------|-------|
| `rmcp` | 3.2.0 | MCP 2026-07-28 | **Official Rust MCP SDK** |

**Key capabilities in rmcp 3.x:**
- Tools, resources, prompts, sampling, elicitation
- Streamable HTTP (stateless, for serverless/edge)
- OAuth support
- Tasks, subscriptions, caching
- Protocol negotiation
- JSON Schema 2020-12
- Backward compatible with MCP 2025-11-25

**Transport for Geoquery:**
- Streamable HTTP for hosted/edge deployment
- stdio for local CLI integration (`geoquery mcp`)

**Decision:** `rmcp` 3.x is the MCP SDK. No alternative needed.

---

## HTTP Server & Client

| Crate | Purpose | Notes |
|-------|---------|-------|
| `axum` | HTTP server framework | **Primary choice** for `geoquery-http` |
| `reqwest` | Async HTTP client | **Primary choice** for adapter HTTP calls |
| `tower` | Middleware (retry, rate limit, timeout) | Wraps both axum and reqwest |
| `tower-http` | HTTP-specific middleware | CORS, compression, tracing |
| `tokio` | Async runtime | **De facto standard**, required by axum/reqwest/rmcp |

**Federation-specific patterns:**

```rust
// Bounded concurrency for parallel source queries
use tokio::task::JoinSet;
let mut set = JoinSet::new();
for source in sources {
    set.spawn(query_source(source, query.clone()));
}

// Per-source timeout
use tokio::time::timeout;
let result = timeout(Duration::from_millis(5000), adapter.query(...)).await;

// Retry with backoff
use tower::retry::Retry;
use tower::timeout::Timeout;
```

**Decision:** `axum` + `reqwest` + `tokio` + `tower`. No alternatives
considered — this is the standard Rust async web stack.

---

## CLI

| Crate | Purpose | Notes |
|-------|---------|-------|
| `clap` | CLI argument parsing | **Primary choice**, derive API |

**Decision:** `clap` with derive macros. No alternative needed.

---

## Serialization & Infrastructure

| Crate | Purpose | Notes |
|-------|---------|-------|
| `serde` | Serialization framework | Derive `Serialize`/`Deserialize` on all types |
| `serde_json` | JSON | Primary wire format |
| `serde_yaml` | YAML | Resource manifest parsing |
| `chrono` | Date/time | `DateTime<Utc>` for temporal extents |
| `thiserror` | Library errors | Typed errors for `geoquery-core` |
| `anyhow` | Application errors | Ergonomic errors for CLI/xtask |
| `tracing` | Structured logging | Async-compatible, span-based |
| `tracing-subscriber` | Log output | `env-filter` for configurable verbosity |

**Decision:** Standard Rust ecosystem choices. No surprises.

---

## WASM Compatibility

| Crate | WASM? | Notes |
|-------|-------|-------|
| `geo-types` | ✅ | Pure Rust, no FFI |
| `geo` | ✅ | Pure Rust |
| `geojson` | ✅ | Pure Rust |
| `rstar` | ✅ | Pure Rust |
| `serde` | ✅ | Pure Rust |
| `chrono` | ✅ | With `wasmbind` feature |
| `reqwest` | ⚠️ | Use `web-sys::fetch` in WASM instead |
| `tokio` | ⚠️ | Limited WASM support; use `wasm-bindgen-futures` |
| `geos` | ❌ | C FFI, not WASM-compatible |
| `proj` | ❌ | C FFI, not WASM-compatible |
| `duckdb` | ❌ | Use DuckDB-WASM separately |
| `tantivy` | ⚠️ | Limited WASM support |

**Decision:** Defer WASM constraints to Phase 6. For now, optimize
for native. When WASM is needed, the core types (`geo-types`, `geo`,
`serde`) are already compatible; only the I/O layer needs adaptation.

---

## Recommended Crate Map

```
geoquery-types/
    geo-types, geojson, serde, serde_json, chrono, ts-rs, schemars

geoquery-core/
    geo, rstar, tokio, reqwest, tower, tracing, thiserror

geoquery-adapter-stac/
    reqwest, serde, serde_json, chrono

geoquery-adapter-ogc/
    reqwest, serde, serde_json, chrono

geoquery-registry/
    rusqlite, rstar, tantivy, serde_yaml

geoquery-http/
    axum, tower-http, tracing, serde_json

geoquery-mcp/
    rmcp, axum, serde_json

geoquery/
    clap, tracing-subscriber, anyhow

geoquery-tui/
    ratatui, crossterm, tui-textarea

xtask/
    xshell, clap, schemars, serde_json
```

---

## Related Files

- [project/architecture](../project/architecture.md) — Where each crate fits
- [adapters/adapter-architecture](../adapters/adapter-architecture.md) — Adapter crate dependencies
- [infrastructure/storage](../infrastructure/storage.md) — Storage crate selection
- [infrastructure/monorepo](../infrastructure/monorepo.md) — Workspace dependency management
- [interfaces/mcp](../interfaces/mcp.md) — `rmcp` integration
- [research/typescript-sdks](typescript-sdks.md) — TypeScript ecosystem comparison
- [research/python-sdks](python-sdks.md) — Python ecosystem comparison
