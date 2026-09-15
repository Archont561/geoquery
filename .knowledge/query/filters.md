---
id: query/filters
title: Attribute Filtering — CQL2 Compilation
category: query
tags: [CQL2, filters, expressions, queryables, compilation, SQL, ArcGIS]
refs: [query/query-model, query/planner, project/standards, adapters/stac, adapters/ogc]
status: draft
created: 2025-07-11
updated: 2025-07-11
---

# Attribute Filtering — CQL2 Compilation

## Design Principle

> The Geoquery filter AST is **CQL2-inspired**, not a CQL2 clone.
> It should **compile to CQL2 whenever possible.**

OGC API Features standardizes CQL2 as its filter language; STAC's
Filter extension uses CQL2 JSON for POST searches. Borrowing its
concepts is far better than inventing an alien filter language.

---

## Internal Expression Tree

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

### Supported Logical Operators

`AND`, `OR`, `NOT`

### Supported Comparison Operators

`=`, `<>`, `<`, `>`, `<=`, `>=`, `IN`, `LIKE`

Spatial and temporal operators live in their own AST branches
(`spatial`, `temporal`), not in `filters` — but the compiled CQL2
output merges them, since CQL2 treats all predicates uniformly.

→ See [query/spatial](spatial.md), [query/temporal](temporal.md)

---

## Compilation Targets

Adapters translate the internal tree to native source syntax:

| Target | Format | Used By |
|--------|--------|---------|
| **CQL2 JSON** | `{"op": "and", "args": [...]}` | STAC API Filter ext, OGC API Features Filtering ext |
| **CQL2 text** | `cloud_cover < 10 AND platform = 'sentinel-2'` | OGC `filter-lang=cql2-text` |
| **SQL WHERE** | Parameterized SQL | PostGIS, DuckDB adapters |
| **STAC filter** | CQL2 JSON in POST `/search` body | STAC |
| **ArcGIS where** | `cloud_cover < 10 AND platform = 'sentinel-2'` | ArcGIS REST |
| **WFS/FES** | XML Filter Encoding | WFS 2.0 |
| **CMR query** | Native CMR query params | NASA CMR |
| **Dropped** | N/A | Sources without attribute capability |

---

## Translation Example

**Geoquery AST:**
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

**Compiles to CQL2 JSON:**
```json
{
  "op": "and",
  "args": [
    { "op": "<", "args": [{ "property": "eo:cloud_cover" }, 10] },
    { "op": "=", "args": [{ "property": "platform" }, "sentinel-2"] }
  ]
}
```

**Compiles to SQL (PostGIS):**
```sql
WHERE (eo_cloud_cover < $1) AND (platform = $2)
-- parameters: [10, 'sentinel-2']
```

**Compiles to ArcGIS where:**
```
cloud_cover < 10 AND platform = 'sentinel-2'
```

Note: property names are adapted per-source (e.g. `cloud_cover` →
`eo:cloud_cover` for STAC). This mapping comes from the
`SchemaDescriptor` / queryables discovered per service.

---

## Queryables

Each service exposes its queryable properties:

```typescript
interface SchemaDescriptor {
  queryables: Record<string, "string" | "number" | "boolean" | "datetime" | "geometry">
}
```

**Example (STAC):**
```json
{
  "eo:cloud_cover": "number",
  "platform": "string",
  "datetime": "datetime"
}
```

**Example (OGC API Features):** discovered from the `queryables`
endpoint (part of the Filtering extension).

The planner checks the requested filter fields against the service's
queryables. Filtering on a non-queryable field:
1. Falls back to local post-filtering if `raw` properties are returned
2. Or produces a warning if the field is unavailable entirely

---

## What CQL2 Does NOT Solve

The Geoquery AST deliberately contains things beyond CQL2:

```json
{
  "semantic": "agricultural land cover",
  "scope": "known",
  "rank": { "by": "relevance" },
  "execution": {
    "federate": true,
    "maxSources": 20,
    "timeoutMs": 5000
  }
}
```

| Concern | In CQL2? | In Geoquery AST? |
|---------|----------|------------------|
| Spatial predicates | ✅ | ✅ (compiled to CQL2) |
| Temporal predicates | ✅ | ✅ (compiled to CQL2) |
| Attribute comparison | ✅ | ✅ (compiled to CQL2) |
| Federation scope | ❌ | ✅ `scope` |
| Ranking preference | ❌ | ✅ `sort` / `rank` |
| Execution hints | ❌ | ✅ `execution` |
| Semantic intent | ❌ | ✅ `semantic` |

This separation is the core reason Geoquery has its own AST rather
than passing CQL2 through directly.

---

## Rust Implementation Sketch

```rust
pub enum FilterExpr {
    And(Vec<FilterExpr>),
    Or(Vec<FilterExpr>),
    Not(Box<FilterExpr>),
    Comparison {
        field: String,
        op: CompareOp,
        value: serde_json::Value,
    },
    In { field: String, values: Vec<serde_json::Value> },
}

pub trait FilterCompiler {
    type Output;
    fn compile(&self, expr: &FilterExpr) -> Result<Self::Output>;
}

pub struct Cql2JsonCompiler;   // → STAC / OGC
pub struct SqlCompiler;        // → PostGIS / DuckDB
pub struct ArcgisWhereCompiler;
pub struct FesCompiler;        // → WFS XML
```

Each adapter owns its compiler. No shared "universal filter string".

---

## Related Files

- [query/query-model](query-model.md) — Where filters sit in the AST
- [query/spatial](spatial.md) — Spatial predicates (compiled alongside)
- [query/temporal](temporal.md) — Temporal predicates
- [project/standards](../project/standards.md) — CQL2 standards position
- [adapters/stac](../adapters/stac.md) — STAC filter extension usage
- [adapters/ogc](../adapters/ogc.md) — OGC queryables discovery
- [query/planner](planner.md) — Where compilation is invoked
