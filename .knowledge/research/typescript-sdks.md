---
id: research/typescript-sdks
title: TypeScript / JavaScript SDK Research & Findings
category: research
tags: [TypeScript, SDK, MCP, Honua, Turf, rbush, DuckDB-WASM, GeoArrow, npm]
refs: [interfaces/typescript, interfaces/mcp, project/standards, adapters/adapter-architecture]
status: draft
created: 2025-07-11
updated: 2025-07-11
---

# TypeScript / JavaScript SDK Research & Findings

## Summary

The TypeScript geospatial ecosystem is less mature than Python's for
GIS-specific tasks but strong for web integration. The most significant
2026 finding is the **MCP SDK v2 package split** and the emergence of
**`@honua/sdk-js`** as a protocol-neutral geospatial SDK that is
architecturally very similar to Geoquery.

---

## MCP SDK — Critical 2026 Update

### v2 Package Structure

The official TypeScript MCP SDK is now **v2** with a split package
structure:

| Old (v1) | New (v2) |
|----------|----------|
| `@modelcontextprotocol/sdk` (monolithic) | `@modelcontextprotocol/server` |
| | `@modelcontextprotocol/client` |

**Key facts:**
- v2 implements the **MCP 2026-07-28 specification**
- v2 replaces the monolithic v1 package
- v1.x continues receiving bug/security fixes for a limited transition
- Streamable HTTP transport is now the default
- OAuth support included

**Impact on Geoquery:**
- `@geoquery/client` should depend on `@modelcontextprotocol/client` v2
  for MCP connectivity
- If a TypeScript MCP server is needed (alternative to Rust `rmcp`),
  use `@modelcontextprotocol/server` v2
- The Rust `rmcp` server is the primary MCP implementation; TS is for
  client-side only

---

## Honua — Architectural Reference

### `@honua/sdk-js`

**This is the most important competitive/reference finding.**

Honua describes itself as a **protocol-neutral geospatial SDK** covering:

| Protocol | Supported |
|----------|-----------|
| Esri GeoServices | ✅ |
| OGC API Features | ✅ |
| OGC API Tiles | ✅ |
| OGC API Maps | ✅ |
| OGC API Processes | ✅ |
| STAC | ✅ |
| WMS | ✅ |
| WMTS | ✅ |
| WFS 2.0 | ✅ |
| OData | ✅ |

### Honua's Contract Layer

Honua defines concepts strikingly similar to Geoquery's:

```
Dataset           ≈ Geoquery ResourceDescriptor
Source            ≈ Geoquery ServiceDescriptor
SourceDescriptor  ≈ Geoquery ServiceDescriptor + capabilities
Capabilities      ≈ Geoquery CapabilitySet
Query             ≈ Geoquery GeoQuery (partial)
Result            ≈ Geoquery GeoResult
IJobRun           ≈ Geoquery ExecutionPlan (async)
MapBinding        ≈ (Geoquery doesn't do rendering)
```

### Honua's MCP Server

Honua also includes a **platform-free geospatial MCP server** that can
expose discovery, querying, and analysis against public ArcGIS
FeatureServer and OGC API endpoints without requiring a Honua server.

### Geoquery's Differentiation vs Honua

| Aspect | Honua | Geoquery |
|--------|-------|----------|
| Protocol coverage | Broader (WMS, WMTS, OData, Tiles) | Narrower initially (STAC, OGC, WFS) |
| Query federation | Per-source queries | **Multi-source parallel federation** |
| Query planner | Not emphasized | **Core IP** — capability-aware planning |
| Degradation | Not documented | **Remote bbox + local refinement** |
| Semantic search | Not documented | **tantivy + embeddings** |
| Context/constraints | Not documented | **Markdown sidecar + structured constraints** |
| Storage tiers | Not documented | **0–3 (stateless → PostGIS)** |
| Language | TypeScript only | **Rust core + TS/Python SDKs** |
| Edge deployment | Not documented | **Cloudflare Workers / WASM** |

**Action:** Study Honua's contract layer and adapter boundaries deeply
before finalizing Geoquery's `ServiceAdapter` trait. Understand where
their abstractions succeed and where they leak protocol details.

---

## Geospatial Operations

| Package | Purpose | Notes |
|---------|---------|-------|
| `@turf/turf` | Spatial operations | **Primary choice** for client-side geometry ops. Intersects, within, bbox, distance, buffer |
| `@types/geojson` | GeoJSON type definitions | TypeScript type safety for GeoJSON |
| `proj4` | CRS transformations | Client-side reprojection |
| `rbush` | R-tree spatial index | Client-side spatial indexing for result filtering |
| `wkx` | WKT/WKB parsing | If needed for OGC responses |

**Decision:** `@turf/turf` for the TS SDK's client-side spatial
operations. `rbush` for optional client-side result filtering.

---

## Storage (Browser / Edge)

| Package | Purpose | Notes |
|---------|---------|-------|
| `@duckdb/duckdb-wasm` | DuckDB in browser | Local GeoParquet queries in browser/edge |
| `hyparquet` | Parquet reader (pure JS) | Lightweight Parquet parsing |
| `parquet-wasm` | Parquet via WASM | Faster than pure JS |
| `apache-arrow` | Arrow JS implementation | GeoArrow columnar data in browser |
| `better-sqlite3` | SQLite (Node.js) | Local metadata index for Node |
| `libsql` | libSQL client | Edge-compatible SQLite |

**Decision:** For the MVP TS SDK, no local storage — HTTP to the Rust
server only. In Phase 6, evaluate `@duckdb/duckdb-wasm` for browser-
based GeoParquet queries.

---

## Search

| Package | Purpose | Notes |
|---------|---------|-------|
| `minisearch` | Full-text search | Lightweight, in-browser metadata search |
| `flexsearch` | Full-text search | Fast, flexible indexing |

**Decision:** Client-side search is not a priority. The Rust server
handles all indexing via `tantivy`. Client-side search only matters
for offline/WASM mode in Phase 6.

---

## HTTP / Federation

| Package | Purpose | Notes |
|---------|---------|-------|
| `ofetch` | HTTP client | **Primary choice** — retry, timeout, interceptors |
| `ky` | HTTP client | Alternative to ofetch, lighter |
| `p-limit` | Concurrency limiter | Bounded parallel API calls |
| `AbortController` | Request cancellation | Native, for per-source timeouts |

**Decision:** `ofetch` for the TS SDK HTTP client. Clean API, built-in
retry and timeout, works in Node and browser.

---

## STAC / OGC Clients

| Package | Purpose | Notes |
|---------|---------|-------|
| `stac-ts` | STAC types | TypeScript STAC type definitions |
| (none dominant) | STAC API client | No `pystac-client` equivalent in TS |
| (none dominant) | OGC API client | No `OWSLib` equivalent in TS |

**Finding:** The TS ecosystem lacks dominant STAC/OGC client SDKs.
This is fine for Geoquery because:
1. The TS SDK talks to the Rust HTTP server, not directly to STAC/OGC
2. If direct access is needed, raw `ofetch` + type definitions suffice

---

## Recommended TS Stack

```
@geoquery/client
├── ofetch                    # HTTP to Rust server
├── @types/geojson            # Type safety
├── @turf/turf                # Client-side spatial (optional)
└── @modelcontextprotocol/client  # MCP connectivity (optional)

Dev:
├── typescript
├── vitest
└── (auto-generated types from cargo xtask codegen)
```

---

## Related Files

- [interfaces/typescript](../interfaces/typescript.md) — TS SDK design
- [interfaces/mcp](../interfaces/mcp.md) — MCP v2 integration
- [project/standards](../project/standards.md) — Standards position
- [research/rust-crates](rust-crates.md) — Rust ecosystem comparison
- [research/python-sdks](python-sdks.md) — Python ecosystem comparison
