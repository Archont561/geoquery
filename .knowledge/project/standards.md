---
id: project/standards
title: Standards Position
category: project
tags: [OGC, STAC, CQL2, DCAT, GeoDCAT, GeoJSON, MCP, OpenAPI, standards]
refs: [project/overview, adapters/stac, adapters/ogc, query/filters, interfaces/mcp]
status: draft
created: 2025-07-11
updated: 2025-07-11
---

# Standards Position

## Golden Rule

> **Geoquery MUST reuse existing standards rather than inventing
> replacements.**

Geoquery's opportunity is to sit **one level above** the standards
ecosystem and make them queryable as one system. It is not fighting
the standards — it is composing them.

---

## Primary Standards

| Standard | Version | Role in Geoquery |
|----------|---------|-----------------|
| **OGC API — Records** | 1.0 | Resource and catalog discovery. Describes resources broadly: datasets, APIs, services, processes, EO assets, ML models. Supports static files and APIs. |
| **STAC / STAC API** | 1.1 / 1.0 | Spatiotemporal asset metadata and search. First-class adapter, not merely a metadata format. Now an OGC Community Standard. |
| **OGC API — Features** | 1.0 | Feature-level query foundation. Provides the spatial/temporal/attribute query model. |
| **CQL2** | 1.0 | Portable filtering language. Geoquery's internal filter AST **compiles to** CQL2 for compatible backends. Used by both OGC API Features (Filtering extension) and STAC API (Filter extension). |
| **GeoJSON** | RFC 7946 | Normalized feature output format. All geometry output uses GeoJSON-compatible structures. |
| **DCAT / GeoDCAT** | 2.0 / 3.0 | Catalog interoperability concepts. Useful for cross-catalog federation and European open data portals. |
| **OpenAPI** | 3.1 | HTTP service descriptions. Used for generic HTTP adapter capability detection. |
| **MCP** | 2026-07-28 | Agent integration protocol. Geoquery exposes `geo_query`, `geo_resource`, `geo_resolve` tools. |

---

## OGC API — Records (Critical Standard)

OGC API — Records is particularly important because its record model
describes resources **broadly**, including:

- Datasets
- APIs and services
- Processes
- Earth-observation assets
- ML models
- Other resource types

It explicitly supports catalogs implemented as **static files** as well
as **APIs**, which aligns with Geoquery's tiered storage model.

**Geoquery's relationship to OGC Records:**
- Geoquery **consumes** OGC Records endpoints as an adapter
- Geoquery can **expose** its own registry as an OGC Records endpoint
  (Phase 7, ecosystem)
- The `ResourceDescriptor` is conceptually aligned with OGC Record but
  is not a clone — it preserves unknown fields in an extension area

---

## STAC (First-Class Adapter)

STAC should be treated as a **first-class adapter** rather than merely
another metadata format.

**Key facts:**
- STAC 1.1 and STAC API 1.0 are established standards
- STAC is now an OGC Community Standard
- The STAC Filter extension uses **CQL2 JSON** for POST searches
- Experimental federated-STAC efforts already combine Records and STAC

**Geoquery's STAC integration:**
- Adapter translates `GeoQuery` → STAC API `/search` POST
- Spatial predicates → `bbox` or `intersects` in CQL2-JSON
- Temporal predicates → `datetime` parameter
- Attribute filters → CQL2-JSON `filter` body
- Collections discovered via `/collections` endpoint
- Conformance classes inspected for capability detection

→ See [adapters/stac](../adapters/stac.md) for implementation details

---

## CQL2 (Filter Compilation Target)

CQL2 is the **compilation target** for attribute filters, not the source
language.

**Geoquery's filter AST compiles to:**
- CQL2 JSON (for STAC API, OGC API Features)
- SQL WHERE (for PostGIS, DuckDB)
- ArcGIS `where` clause
- WFS/FES XML
- CMR query parameters

**Example translation:**

Geoquery AST:
```json
{
  "filters": {
    "and": [
      { "field": "cloud_cover", "op": "<", "value": 10 },
      { "field": "platform", "op": "=", "value": "sentinel-2" }
    ]
  }
}
```

Compiles to CQL2 JSON:
```json
{
  "op": "and",
  "args": [
    { "op": "<", "args": [{ "property": "eo:cloud_cover" }, 10] },
    { "op": "=", "args": [{ "property": "platform" }, "sentinel-2"] }
  ]
}
```

**Important:** The Geoquery AST needs things CQL2 doesn't solve:
- Federation scope (`which sources?`)
- Ranking preferences
- Execution hints (`timeout`, `max_sources`)
- Semantic intent

These are **federation concerns**, not source-level filtering.

→ See [query/filters](../query/filters.md) for filter AST design

---

## GeoJSON (Output Format)

GeoJSON is the **normalized output format** for all geometry and feature
results.

**Rules:**
- Internal geometry representation uses GeoJSON-compatible structures
- CRS must be explicit whenever non-WGS84 coordinates are involved
- Geoquery MUST NOT silently reinterpret coordinates in an unknown CRS
- GeoJSON is for **output** — internal interchange uses GeoArrow for
  performance

→ See [query/spatial](../query/spatial.md) for spatial query rules

---

## MCP (Agent Integration)

MCP is the protocol for AI agent integration.

**Current spec:** MCP 2026-07-28 (stable)
**Rust SDK:** `rmcp` 3.x
**TypeScript SDK:** `@modelcontextprotocol/server` v2, `@modelcontextprotocol/client` v2

**Key 2026 updates:**
- TypeScript SDK v2 replaces the monolithic v1 package
- `rmcp` 3.x supports Streamable HTTP, OAuth, tasks, subscriptions
- Both target the July 28, 2026 specification

**Geoquery MCP tools:**
- `geo_query` — Execute a structured GeoQuery
- `geo_resource` — Retrieve metadata/context/provenance
- `geo_resolve` — Resolve a place name to geometry

**Design principle:** Do not expose every backend as a separate MCP tool.
The agent interacts with the Geoquery abstraction, not the underlying
protocol ecosystem.

→ See [interfaces/mcp](../interfaces/mcp.md) for full MCP design

---

## Standards Convergence

OGC's current AI-ready geospatial work is explicitly converging STAC,
OGC API Records, DCAT, and provenance into interoperable building blocks.

This validates Geoquery's architecture: the standards are moving toward
composability, and Geoquery sits at the composition layer.

**Existing proof points:**
- OGC API Records provides a general catalog/record model
- STAC provides mature spatiotemporal asset discovery
- OGC API Features + CQL2 provide standardized feature querying
- Experimental federated-STAC efforts combine Records and STAC
- Geo-MCP servers are proliferating (but as individual adapters, not
  federation layers)
- AI GIS servers expose entire GeoServer/PostGIS stacks through single
  MCP tools

**The gap:** No universal federation layer exists above all of these.
Geoquery fills that gap.

---

## What Geoquery Does NOT Standardize

| Area | Geoquery's Position |
|------|-------------------|
| Metadata schemas | Uses source-native schemas (STAC, ISO 19115, DCAT) |
| Coordinate systems | Uses EPSG/WGS84, delegates reprojection to `proj` |
| Filter syntax | Compiles to CQL2, doesn't replace it |
| Feature encoding | Outputs GeoJSON, doesn't invent a format |
| Catalog protocol | Consumes OGC Records/STAC, doesn't replace them |
| Agent protocol | Uses MCP, doesn't invent an agent framework |

---

## Related Files

- [project/overview](overview.md) — Mission and core concept
- [adapters/stac](../adapters/stac.md) — STAC adapter implementation
- [adapters/ogc](../adapters/ogc.md) — OGC API adapter implementation
- [query/filters](../query/filters.md) — CQL2 filter compilation
- [interfaces/mcp](../interfaces/mcp.md) — MCP server design
- [research/rust-crates](../research/rust-crates.md) — Rust crate findings including `rmcp`
