---
type: Query Specification
title: Spatial Query Language
description: "Spatial predicates (intersects, dwithin, bbox, and more) and CRS rules."
tags: [spatial, predicates, geometry, GeoJSON, CRS, dwithin]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: query/spatial
category: query
refs: [query/query-model, query/planner, project/data-model, project/standards]
---

# Spatial Query Language

## Spatial Predicates

Initial spatial operations:

| Op | Description | Adapters must declare support |
|----|-------------|------------------------------|
| `bbox` | Bounding-box intersection | Most common denominator |
| `intersects` | Geometry intersects | STAC (CQL2), OGC, PostGIS |
| `within` | Query geometry within feature | OGC, PostGIS |
| `contains` | Feature within query geometry | OGC, PostGIS |
| `dwithin` | Within distance of geometry | PostGIS, some OGC |
| `nearest` | Nearest neighbor | PostGIS, custom |

Additional topology predicates (available where supported):
`touches`, `overlaps`, `crosses`, `disjoint`.

The full set is an **open enum** with `Custom(String)` as the extension
point.

---

## Examples

### BBox

```json
{
  "spatial": {
    "op": "bbox",
    "bbox": [14.1, 49.0, 24.2, 54.8]
  }
}
```

### DWithin (distance query)

```json
{
  "spatial": {
    "op": "dwithin",
    "geometry": {
      "type": "Point",
      "coordinates": [21.01, 52.23]
    },
    "distance": 50000,
    "unit": "meters"
  }
}
```

**`distance` without `unit` is a validation error.** A bare number is
meaningless — is it meters, feet, degrees?

### Intersects (polygon)

```json
{
  "spatial": {
    "op": "intersects",
    "geometry": {
      "type": "Polygon",
      "coordinates": [[[14.1, 49.0], [24.2, 49.0], [24.2, 54.8], [14.1, 54.8], [14.1, 49.0]]]
    }
  }
}
```

---

## Geometry Representation

The internal geometry representation uses **GeoJSON-compatible
structures** initially (RFC 7946):

- `Point`, `MultiPoint`
- `LineString`, `MultiLineString`
- `Polygon`, `MultiPolygon`
- `GeometryCollection`

**Rust mapping:** `geo_types::Geometry<f64>`, serialized/deserialized
via the `geojson` crate.

For high-performance internal interchange (federation pipeline,
local analytics), GeoArrow is used — but GeoJSON is the external
contract.
→ See [infrastructure/storage](../infrastructure/storage.md) for GeoArrow/GeoParquet details

---

## CRS Rules

> **Geoquery MUST NOT silently reinterpret coordinates in an unknown CRS.**

| Rule | Enforcement |
|------|-------------|
| Default CRS is WGS84 (EPSG:4326) | Assumed when unspecified |
| Non-WGS84 requires explicit CRS | `"crs": "EPSG:2180"` (Poland) |
| Unknown CRS is a **validation error** | Reject, do not guess |
| Reprojection uses `proj` crate | On-the-fly when needed |

```json
{
  "op": "intersects",
  "geometry": { "type": "Point", "coordinates": [637123, 487234] },
  "crs": "EPSG:2180"
}
```

This is one of the [validation rules](query-model.md) for
AI-generated queries: **CRS ambiguity is a rejection condition.**

---

## Degradation Strategy

When a source does not support the requested operation:

```
User asks:      intersects polygon
Source supports: bbox only

Option A: approximate
  → use polygon's bbox, accept false positives

Option B: hybrid refinement
  1. Remote bbox filter (candidate retrieval)
  2. Local exact geometry intersection (refinement)
```

The planner reports which strategy was used:

```
Query execution:
  Remote spatial filter: approximate
  Local spatial refinement: exact
```

→ See [query/planner](planner.md) for the full degradation decision tree

---

## Structured Constraints ≠ Spatial Reasoning

**Correct:**
```json
{ "semantic": "flood modelling datasets", "spatial": { "op": "dwithin", ... } }
```

**Incorrect:**
```json
{ "semantic": "embedding('within 50km of Warsaw')" }
```

Structured geographic constraints must remain structured. An embedding
model must not perform spatial reasoning — it cannot do so reliably.

→ See [query/semantic](semantic.md) for the full separation policy

---

## Related Files

- [query/query-model](query-model.md) — The full GeoQuery AST
- [query/planner](planner.md) — Degradation and capability matching
- [query/semantic](semantic.md) — Semantic/spatial separation
- [project/standards](../project/standards.md) — GeoJSON and CQL2 standards
- [project/data-model](../project/data-model.md) — SpatialExtent on resources
