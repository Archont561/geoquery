---
type: Adapter Specification
title: "OGC API Adapter — Features & Records"
description: "OGC API Features + Records, landing page, queryables, filtering."
tags: [OGC, OGC-API, Features, Records, CQL2, queryables, conformance]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: adapters/ogc
category: adapters
refs: [adapters/adapter-architecture, project/standards, query/filters, query/planner]
---

# OGC API Adapter — Features & Records

## Overview

This adapter handles two related OGC API standards:

| Standard | Purpose | Geoquery Use |
|----------|---------|-------------|
| **OGC API — Features** | Feature-level querying | Spatial/temporal/attribute queries on vector data |
| **OGC API — Records** | Resource/catalog discovery | Discovering datasets, services, and other resources |

Both share the OGC API Common foundation (landing page, conformance,
collections pattern).

---

## Discovery Flow

```
geoquery add https://example.org/ogc
    │
    ▼
1. GET / (landing page)
   → OGC API Common: links to /conformance, /collections, /api
   → Identify type: Features vs Records via conformance classes
    │
    ▼
2. GET /conformance
   → Inspect conformance classes:
     Features:
       http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/core
       http://www.opengis.net/spec/ogcapi-features-3/1.0/conf/filter
       http://www.opengis.net/spec/cql2/1.0/conf/cql2-json
     Records:
       http://www.opengis.net/spec/ogcapi-records-1/1.0/conf/core
       http://www.opengis.net/spec/ogcapi-records-1/1.0/conf/json
    │
    ▼
3. GET /collections
   → Enumerate collections with extents, item types
    │
    ▼
4. GET /collections/{id}/queryables (if Filtering ext)
   → Discover filterable properties and types
    │
    ▼
ServiceDescriptor {
    type: "ogc-features" | "ogc-records",
    url: "https://example.org/ogc",
    capabilities: { ... },
    collections: [...],
    schema: { queryables: {...} }
}
```

---

## OGC API Common Pattern

All OGC APIs follow the landing-page / conformance / API-definition
pattern:

```
/                  → Landing page (links, title, description)
/conformance       → Conformance classes
/api               → OpenAPI definition
/collections       → List of collections
/collections/{id}  → Collection metadata
/collections/{id}/items  → Items (features or records)
```

The adapter uses this pattern for **protocol detection**: if a URL
responds with a JSON landing page containing `conformsTo`, it's
likely an OGC API.

---

## OGC API — Features

### Query Translation

| GeoQuery Field | OGC API Features Parameter |
|---------------|---------------------------|
| `spatial.bbox` | `bbox=w,s,e,n` |
| `spatial.intersects` | `filter` with CQL2 `S_INTERSECTS` |
| `temporal` | `datetime=start/end` |
| `filters` | `filter={CQL2-JSON}` + `filter-lang=cql2-json` |
| `limit` | `limit=N` |
| `offset` | `offset=N` |
| `sort` | `sortby=+field,-field` |

### Example

**GeoQuery:**
```json
{
  "spatial": { "op": "bbox", "bbox": [14.1, 49.0, 24.2, 54.8] },
  "temporal": { "op": "intersects", "start": "2020-01-01", "end": "2025-01-01" },
  "filters": { "and": [{ "field": "type", "op": "=", "value": "river" }] },
  "limit": 50
}
```

**OGC API Features GET:**
```
GET /collections/hydrography/items?
    bbox=14.1,49.0,24.2,54.8&
    datetime=2020-01-01/2025-01-01&
    filter=type='river'&
    filter-lang=cql2-text&
    limit=50
```

Or POST with CQL2-JSON if the service supports it.

### Result Normalization

OGC Feature → GeoResult:

| OGC Field | GeoResult Field |
|-----------|----------------|
| `id` | `id` |
| `geometry` | `geometry` |
| `bbox` | `bbox` |
| `properties.*` | `properties` |
| `links` | `links` |
| `time` (if present) | `temporal` |

Output is already GeoJSON FeatureCollection — minimal transformation
needed.

---

## OGC API — Records

### Purpose in Geoquery

OGC API — Records describes resources **broadly**: datasets, APIs,
services, processes, EO assets, ML models. It supports catalogs as
static files and as APIs.

Geoquery uses Records for **resource discovery** — finding what
datasets and services exist, not querying their data directly.

### Record → ResourceDescriptor Mapping

| OGC Record Field | ResourceDescriptor Field |
|-----------------|------------------------|
| `id` | `id` |
| `type` | `type` (mapped to ResourceType) |
| `title` | `title` |
| `description` | `description` |
| `geometry` / `bbox` | `spatial` |
| `time` | `temporal` |
| `themes` | `themes` |
| `keywords` | `keywords` |
| `providers` | `provider` |
| `license` | `license` |
| `links` | `services` (if link type is a service) |
| `associations` | `relationships` |

### Records vs STAC for Discovery

| Aspect | OGC Records | STAC |
|--------|------------|------|
| Resource types | Broad (datasets, services, models) | Narrow (spatiotemporal assets) |
| Catalog model | Static + API | Static + API |
| Query model | CQL2 | CQL2 (filter ext) |
| Best for | Heterogeneous catalogs | EO / satellite data |

Geoquery treats them as **complementary**: Records for broad discovery,
STAC for spatiotemporal asset search.

---

## Queryables Discovery

The Filtering extension provides a `queryables` endpoint:

```
GET /collections/{id}/queryables
```

Returns a JSON Schema describing filterable properties:

```json
{
  "properties": {
    "name": { "type": "string", "title": "Name" },
    "population": { "type": "number", "title": "Population" },
    "geometry": { "$ref": "https://geojson.org/schema/Geometry.json" }
  }
}
```

The adapter maps this to `SchemaDescriptor.queryables` for the
planner's filter compilation.

→ See [query/filters](../query/filters.md) for queryables usage

---

## Capability Detection

| Conformance Class | Capability |
|-------------------|-----------|
| `conf/core` | Basic item retrieval, bbox, datetime |
| `conf/filter` | CQL2 attribute filtering |
| `conf/cql2-json` | CQL2 JSON encoding |
| `conf/cql2-text` | CQL2 text encoding |
| `conf/sorting` | Result sorting |
| `conf/features` | Feature-specific operations |
| `conf/records-core` | Records-specific operations |

---

## Related Files

- [adapters/adapter-architecture](adapter-architecture.md) — ServiceAdapter trait
- [adapters/stac](stac.md) — STAC adapter (complementary)
- [project/standards](../project/standards.md) — OGC standards position
- [query/filters](../query/filters.md) — CQL2 compilation
- [project/data-model](../project/data-model.md) — ResourceDescriptor (Records mapping)
