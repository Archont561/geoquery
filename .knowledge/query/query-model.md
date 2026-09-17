---
type: Query Specification
title: "GeoQuery AST — The Canonical Query Model"
description: "The GeoQuery AST — scope, spatial, temporal, semantic, filters, execution."
tags: [AST, query, canonical, API, scope, execution]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: query/query-model
category: query
refs: [query/spatial, query/temporal, query/filters, query/semantic, query/planner, project/data-model, interfaces/mcp]
---

# GeoQuery AST — The Canonical Query Model

## The Query IS the API

The canonical query AST should receive more design effort than the YAML
format. The API is not STAC API, OGC API, or ArcGIS API — it is
**Geoquery Query**. Everything else is an adapter.

The same AST works through:

| Interface | Invocation |
|-----------|-----------|
| TypeScript | `geo.query({...})` |
| Rust | `engine.execute(query)` |
| HTTP | `POST /query` |
| CLI | `geoquery query --semantic ...` |
| MCP | `geo_query` tool arguments |

→ See [interfaces/mcp](../interfaces/mcp.md), [interfaces/typescript](../interfaces/typescript.md), [interfaces/python](../interfaces/python.md)

---

## Full AST Structure

```typescript
interface GeoQuery {
  // ── Federation concerns (Geoquery-specific, NOT CQL2) ──
  scope?: QueryScope
  execution?: ExecutionOptions

  // ── Discovery intent ──
  semantic?: SemanticQuery

  // ── Deterministic predicates ──
  spatial?: SpatialPredicate
  temporal?: TemporalPredicate
  filters?: FilterExpression

  // ── Result shaping ──
  sort?: SortExpression[]
  limit?: number
  offset?: number
  fields?: string[]
  include?: IncludeOptions
}
```

### Structural View

```
GeoQuery
├── scope          ← which sources participate
├── discovery      ← semantic intent
├── filter         ← attribute expressions (CQL2-compatible)
├── spatial        ← geometry predicates
├── temporal       ← time predicates
├── semantic       ← optional embedding-based ranking
├── ranking        ← result ordering preferences
└── execution      ← federation hints (timeout, max_sources)
```

**The key distinction:** federation concerns (scope, execution, ranking)
are separated from source-level filtering (spatial, temporal, attribute).
CQL2 solves the latter; it does not solve the former.

---

## Example

```json
{
  "semantic": "flood risk",
  "spatial": {
    "op": "intersects",
    "geometry": {
      "type": "Polygon",
      "coordinates": [[[14.1, 49.0], [24.2, 49.0], [24.2, 54.8], [14.1, 54.8], [14.1, 49.0]]]
    }
  },
  "temporal": {
    "op": "during",
    "start": "2020-01-01",
    "end": "2025-01-01"
  },
  "filters": {
    "and": [
      { "field": "cloud_cover", "op": "<", "value": 10 },
      { "field": "platform", "op": "=", "value": "sentinel-2" }
    ]
  },
  "limit": 20
}
```

---

## QueryScope

Queries may operate against a restricted subset of the registry:

```typescript
interface QueryScope {
  resources?: string[]       // Resource IDs
  services?: string[]        // Service IDs
  providers?: string[]       // e.g. ["NASA", "Copernicus"]
  resourceTypes?: string[]   // e.g. ["dataset", "coverage"]
  tags?: string[]            // User-defined tags
}
```

**Special value:** `"*"` means all resources known to the current
Geoquery registry.

```json
{ "scope": { "resources": "*" } }
```

```json
{ "scope": { "providers": ["NASA", "Copernicus"] } }
```

---

## ExecutionOptions

Federation hints — how the query should be executed, not what it means:

```typescript
interface ExecutionOptions {
  federate?: boolean        // Default: true
  maxSources?: number       // Cap parallel sources, e.g. 20
  timeoutMs?: number        // Per-source timeout, e.g. 5000
  mode?: "remote" | "local" | "hybrid" | "auto"  // Default: "auto"
}
```

These are **federation concerns, not source-level filtering**. They
control how the [query/planner](planner.md) fans out and degrades.

---

## Deterministic Predicates + Optional Semantic Intent

The canonical query MUST work without any LLM:

```json
{
  "spatial": { "op": "within", "geometry": "..." },
  "filters": [ ["population", ">", 100000] ]
}
```

This is fully deterministic.

Semantic intent is an **optional higher-level layer**:

```json
{
  "semantic": "large cities near major rivers"
}
```

The query language therefore has:

```
deterministic predicates
    +
optional semantic intent
```

It is NOT an LLM-generated SQL system.

→ See [query/semantic](semantic.md) for the separation rules

---

## Validation Rules (AI Safety Boundary)

AI-generated queries MUST pass validation. Reject or request
clarification when:

| Condition | Rejection Reason |
|-----------|-----------------|
| CRS is ambiguous | Coordinates cannot be safely interpreted |
| Geometry is malformed | Invalid GeoJSON structure |
| Temporal interval is invalid | e.g. start > end |
| Distance lacks units | `dwithin` without unit is meaningless |
| Unsupported operation requested | Capability mismatch |
| Source capability insufficient | Planner cannot satisfy query |
| Place resolution has multiple candidates | Ambiguity requires user choice |

The system must distinguish:
- **interpreted** — what the query literally says
- **resolved** — what places/entities were resolved to
- **executed** — what was actually sent to sources
- **inferred** — what was approximated or degraded

Inferred spatial facts MUST NOT be represented as authoritative
source data.

### The Trust Boundary

```
LLM
 ↓
structured GeoQuery AST
 ↓
validation
 ↓
planner
 ↓
registered service
```

This is much safer than `LLM → arbitrary SQL/API calls`. The agent
constructs the AST; Geoquery validates it and executes only against
registered capabilities. No arbitrary `DROP TABLE`, no arbitrary
remote URLs.

---

## Field Selection

```typescript
{
  "fields": ["id", "title", "bbox", "temporal", "provenance"],
  "include": {
    "raw": false,          // Omit original source payload
    "context": false,      // Omit markdown context
    "assets": true
  }
}
```

`raw` may be omitted from lightweight responses to reduce payload size.

---

## Related Files

- [query/spatial](spatial.md) — Spatial predicate specification
- [query/temporal](temporal.md) — Temporal predicate specification
- [query/filters](filters.md) — Attribute filter AST and CQL2 compilation
- [query/semantic](semantic.md) — Semantic intent and constraints
- [query/planner](planner.md) — How the AST becomes an execution plan
- [project/data-model](../project/data-model.md) — GeoResult (what queries return)
- [interfaces/mcp](../interfaces/mcp.md) — The AST as MCP tool schema
