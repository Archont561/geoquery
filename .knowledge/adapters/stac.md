---
type: Adapter Specification
title: STAC API Adapter
description: "STAC API /search, collection discovery, CQL2-JSON, conformance."
tags: [STAC, stac-api, search, collections, CQL2, conformance]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: adapters/stac
category: adapters
refs: [adapters/adapter-architecture, project/standards, query/filters, query/planner]
---

# STAC API Adapter

## Status

STAC is a **first-class adapter**, not merely another metadata format.
STAC 1.1 and STAC API 1.0 are established standards and now an OGC
Community Standard.

**Phase 1 spike (2026-10-05):** landing-page/conformance discovery, collection listing,
and query translation/normalization are implemented in `crates/adapter-stac`, narrowed to
`bbox`, `datetime`, `collections` and `limit` only — CQL2 filtering, sorting, and
pagination past one page are not yet sent, and every other query feature is reported as
refused rather than attempted. See
[project/phase-1-stac-spike](../project/phase-1-stac-spike.md) for what that spike proved
and what it left open, including why this adapter's capability reporting deliberately
does not reuse `geoquery-core`'s generic `CapabilityReport` for spatial predicates.

**Rust ecosystem note:** There is no dominant Rust STAC SDK comparable
to Python's `pystac-client`. The adapter is built as custom HTTP +
`serde` models using `reqwest`. This is straightforward because STAC
API's `/search` endpoint is a single POST with CQL2-JSON.

---

## Discovery Flow

```
geoquery add https://planetarycomputer.microsoft.com/api/stac/v1
    │
    ▼
1. GET /api (or landing page)
   → identify as STAC API via "type": "Catalog" + conformsTo
    │
    ▼
2. GET /conformance
   → inspect conformance classes:
     - core (item search)
     - item-search#filter (CQL2 support)
     - item-search#fields
     - item-search#sort
     - item-search#context
    │
    ▼
3. GET /collections
   → enumerate available collections with extents
    │
    ▼
4. GET /collections/{id}/queryables (if filter ext supported)
   → discover filterable properties per collection
    │
    ▼
ServiceDescriptor {
    type: "stac",
    url: "https://.../api/stac/v1",
    capabilities: { ... },
    collections: ["sentinel-2-l2a", "landsat-c2-l2", ...],
    schema: { queryables: { "eo:cloud_cover": "number", ... } }
}
```

---

## Conformance Classes → Capabilities Mapping

| Conformance Class | Capability |
|-------------------|-----------|
| `https://api.stacspec.org/v1.0.0/core` | Basic search, bbox, datetime |
| `https://api.stacspec.org/v1.0.0/item-search` | POST /search |
| `https://api.stacspec.org/v1.0.0/item-search#filter` | CQL2-JSON attribute filtering |
| `http://www.opengis.net/spec/cql2/1.0/conf/cql2-json` | CQL2 JSON encoding |
| `http://www.opengis.net/spec/cql2/1.0/conf/cql2-text` | CQL2 text encoding |
| `https://api.stacspec.org/v1.0.0/item-search#fields` | Field selection |
| `https://api.stacspec.org/v1.0.0/item-search#sort` | Result sorting |
| `https://api.stacspec.org/v1.0.0/item-search#context` | Match count context |

---

## Query Translation

### GeoQuery → STAC POST /search

| GeoQuery Field | STAC Parameter |
|---------------|---------------|
| `spatial.bbox` | `bbox: [w, s, e, n]` |
| `spatial.intersects` | `intersects: { GeoJSON }` |
| `temporal` | `datetime: "start/end"` |
| `filters` | `filter: { CQL2-JSON }` |
| `filters` (text) | `filter-lang: "cql2-json"` |
| `fields` | `fields: { include: [...], exclude: [...] }` |
| `sort` | `sortby: [{ field, direction }]` |
| `limit` | `limit: N` |
| `scope.collections` | `collections: ["sentinel-2-l2a"]` |

### Example Translation

**GeoQuery:**
```json
{
  "scope": { "resources": ["planetary-computer"] },
  "spatial": { "op": "bbox", "bbox": [14.1, 49.0, 24.2, 54.8] },
  "temporal": { "op": "intersects", "start": "2024-06-01", "end": "2024-08-31" },
  "filters": {
    "and": [
      { "field": "cloud_cover", "op": "<", "value": 10 },
      { "field": "platform", "op": "=", "value": "sentinel-2" }
    ]
  },
  "limit": 20
}
```

**STAC POST /search:**
```json
{
  "bbox": [14.1, 49.0, 24.2, 54.8],
  "datetime": "2024-06-01T00:00:00Z/2024-08-31T23:59:59Z",
  "collections": ["sentinel-2-l2a"],
  "filter-lang": "cql2-json",
  "filter": {
    "op": "and",
    "args": [
      { "op": "<", "args": [{ "property": "eo:cloud_cover" }, 10] },
      { "op": "=", "args": [{ "property": "platform" }, "sentinel-2"] }
    ]
  },
  "limit": 20
}
```

---

## Result Normalization

STAC Item → GeoResult mapping:

| STAC Field | GeoResult Field |
|-----------|----------------|
| `id` | `id` |
| `collection` | `resource.id` |
| `geometry` | `geometry` |
| `bbox` | `bbox` |
| `properties.datetime` | `temporal` |
| `properties.*` | `properties` |
| `assets` | `assets` |
| `links` | `links` |
| `stac_version`, `stac_extensions` | `extensions` |

Provenance is automatically attached:
```json
{
  "source": "planetary-computer",
  "service": "https://.../api/stac/v1",
  "protocol": "stac",
  "collection": "sentinel-2-l2a",
  "query": { "bbox": [...], "datetime": "..." }
}
```

---

## Pagination

STAC API uses link-based pagination (`next` link). The adapter
follows `next` links up to the requested `limit`, accumulating
results across pages.

---

## Known STAC APIs for Testing

| Source | URL | Notes |
|--------|-----|-------|
| Planetary Computer | `https://planetarycomputer.microsoft.com/api/stac/v1` | Microsoft, large catalog |
| Earth Search | `https://earth-search.aws.element84.com/v1` | AWS, Element 84 |
| USGS STAC | `https://landsatlook.usgs.gov/stac-server` | USGS Landsat |
| CDSE | `https://catalogue.dataspace.copernicus.eu/stac` | Copernicus |

---

## Related Files

- [adapters/adapter-architecture](adapter-architecture.md) — ServiceAdapter trait
- [project/standards](../project/standards.md) — STAC standards position
- [query/filters](../query/filters.md) — CQL2 compilation for STAC filter extension
- [query/planner](../query/planner.md) — How STAC sources are planned against
- [research/rust-crates](../research/rust-crates.md) — Rust STAC ecosystem findings
