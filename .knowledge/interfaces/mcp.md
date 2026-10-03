---
type: Interface Specification
title: MCP Server for AI Agents
description: "geo_query, geo_resource, geo_resolve tools; rmcp 3.x; 2026 spec."
tags: [MCP, AI, agents, rmcp, tools, resources, 2026-spec, Streamable-HTTP]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: interfaces/mcp
category: interfaces
refs: [query/query-model, project/architecture, project/data-model, query/semantic, extensions/extension-points]
---

# MCP Server for AI Agents

## Crate

`geoquery-mcp` — MCP server using `rmcp` 3.x (Rust) implementing
the **MCP 2026-07-28 specification**.

MCP is an **adapter, not the core protocol**. The agent interacts
with the Geoquery abstraction, not the underlying protocol ecosystem.

---

## 2026 Spec Updates

Key changes from the MCP 2026-07-28 specification that affect
Geoquery:

| Feature | Relevance |
|---------|-----------|
| **Streamable HTTP** | Stateless transport for serverless/edge deployment |
| **OAuth support** | Authenticated agent access |
| **Tasks** | Long-running federated queries as async tasks |
| **Subscriptions** | Real-time query progress updates |
| **Caching** | Cache query results across agent sessions |
| **JSON Schema 2020-12** | Tool input schemas |

**Rust SDK:** `rmcp` 3.2.0 — implements the stable 2026-07-28 spec
with backward compatibility to 2025-11-25.
**TypeScript SDK:** `@modelcontextprotocol/server` v2 (replaces
monolithic v1 `@modelcontextprotocol/sdk`).

---

## Core Tools

### `geo_query` — Execute a Query

The primary tool. The agent constructs a GeoQuery AST; Geoquery
validates and executes it.

**Input schema:**
```json
{
  "type": "object",
  "properties": {
    "semantic": { "type": "string", "description": "Natural language intent" },
    "spatial": { "$ref": "#/$defs/SpatialPredicate" },
    "temporal": { "$ref": "#/$defs/TemporalPredicate" },
    "filters": { "$ref": "#/$defs/FilterExpression" },
    "scope": { "$ref": "#/$defs/QueryScope" },
    "limit": { "type": "integer", "default": 20 }
  }
}
```

**Example invocation:**
```json
{
  "semantic": "datasets useful for flood modelling",
  "spatial": {
    "op": "dwithin",
    "geometry": { "type": "Point", "coordinates": [21.01, 52.23] },
    "distance": "50km"
  },
  "temporal": { "start": "2020-01-01", "end": "2025-01-01" },
  "limit": 20
}
```

**Output:** Normalized GeoResult array with provenance, source status,
and warnings. Results are deliberately **small and useful** to
prevent context-window explosion.

---

### `geo_resource` — Retrieve Resource Context

Fetch detailed metadata, context, and provenance for a specific
resource. This is the **deep context** step after `geo_query`
returns a summary.

**Input:**
```json
{ "id": "copernicus:sentinel-2-l2a" }
```

**Output:**
```json
{
  "id": "copernicus:sentinel-2-l2a",
  "title": "Sentinel-2 Level-2A",
  "description": "...",
  "context": {
    "limitations": "Not suitable for parcel-level insurance...",
    "methodology": "...",
    "usage": "Suitable for regional flood modelling..."
  },
  "constraints": {
    "suitable_for": ["regional flood modelling"],
    "not_suitable_for": ["parcel insurance"]
  },
  "services": [...],
  "relationships": [...]
}
```

**Pattern:**
```
geo_query    → small useful results (summary)
    ↓
geo_resource → deep context on demand (detail)
```

This prevents context-window explosion. The agent doesn't receive
full context for all 50 results — only for the ones it asks about.

---

### `geo_resolve` — Resolve Place to Geometry

Convert a natural-language place name to a geometry for use in
spatial queries.

**Input:**
```json
{ "place": "Warsaw, Poland" }
```

**Output:**
```json
{
  "candidates": [
    {
      "geometry": { "type": "Point", "coordinates": [21.01, 52.23] },
      "bbox": [20.85, 52.10, 21.27, 52.37],
      "confidence": 0.98,
      "display_name": "Warsaw, Masovian Voivodeship, Poland"
    }
  ]
}
```

If multiple plausible candidates exist, the agent should ask the
user to disambiguate (or use the highest-confidence candidate).

---

## Optional Future Tools

| Tool | Purpose | Phase |
|------|---------|-------|
| `geo_sources` | List registered sources and status | 4 |
| `geo_capabilities` | Inspect source capabilities | 4 |
| `geo_download` | Initiate asset download | 5 |
| `geo_map` | Generate static map image | 7 |
| `geo_compare` | Compare two resources | 7 |
| `geo_validate` | Validate a resource manifest | 5 |

**Design principle:** Do not expose every backend as a separate MCP
tool. The agent interacts with the Geoquery abstraction, not STAC
vs WFS vs ArcGIS.

---

## MCP Resources (Context, Not Operations)

Geoquery resources can also be exposed as MCP Resources (read-only
context, not tool invocations):

| URI | Content |
|-----|---------|
| `geo://resource/{id}` | Full resource metadata |
| `geo://resource/{id}/context` | Markdown context (limitations, usage) |
| `geo://resource/{id}/provenance` | Provenance chain |
| `geo://registry` | Registry summary (source count, types) |

**Tools perform operations. Resources provide context.**

An agent can subscribe to `geo://registry` to be notified when new
sources are added.

---

## Schema Generation

Tool input schemas are **auto-generated from Rust types** via
`cargo xtask codegen`:

```rust
// In geoquery-types
#[derive(Serialize, Deserialize, JsonSchema, TS)]
pub struct GeoQuery { ... }

// xtask generates:
// schemas/mcp/geo_query.json  → used by rmcp tool registration
// packages/geoquery/src/types/  → used by TypeScript SDK
```

This ensures 100% parity between the MCP tool schema, the HTTP API
request body, and the TypeScript types. Zero drift.

→ See [infrastructure/xtask](../infrastructure/xtask.md) for codegen pipeline

---

## Transport

| Transport | Use Case |
|-----------|----------|
| **Streamable HTTP** | Stateless, serverless, edge (default for 2026) |
| **stdio** | Local CLI integration, `geoquery mcp` subprocess |
| **SSE** | Legacy clients (2025-11-25 compat) |

---

## Implementation Sketch

```rust
use rmcp::{Server, tool, Tool};

#[derive(Clone)]
struct GeoqueryMcpServer {
    engine: GeoqueryEngine,
}

#[tool]
impl GeoqueryMcpServer {
    #[tool(description = "Execute a geospatial query across federated sources")]
    async fn geo_query(&self, query: GeoQuery) -> Result<QueryResult> {
        self.engine.execute(query).await
    }

    #[tool(description = "Retrieve metadata and context for a resource")]
    async fn geo_resource(&self, id: String) -> Result<ResourceDetail> {
        self.engine.get_resource(&id).await
    }

    #[tool(description = "Resolve a place name to a geometry")]
    async fn geo_resolve(&self, place: String) -> Result<GeocodeResult> {
        self.engine.resolve(&place).await
    }
}
```

---

## Related Files

- [query/query-model](../query/query-model.md) — The GeoQuery AST (tool input)
- [project/data-model](../project/data-model.md) — GeoResult (tool output)
- [query/semantic](../query/semantic.md) — AI safety rules, context delivery pattern
- [extensions/extension-points](../extensions/extension-points.md) — MCP tools as Layer 14 extension
- [infrastructure/xtask](../infrastructure/xtask.md) — Schema generation pipeline
- [research/rust-crates](../research/rust-crates.md) — `rmcp` 3.x findings
- [research/typescript-sdks](../research/typescript-sdks.md) — `@modelcontextprotocol/server` v2
