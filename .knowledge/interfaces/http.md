---
type: Interface Specification
title: HTTP API
description: "POST /query, resource CRUD, SSE streaming, OpenAPI spec."
tags: [HTTP, REST, Axum, SSE, streaming, OpenAPI, API]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: interfaces/http
category: interfaces
refs: [query/query-model, project/architecture, project/data-model, interfaces/typescript, interfaces/python]
---

# HTTP API

## Crate

`geoquery-http` — Axum-based HTTP server exposing the Geoquery engine
as a REST API.

The HTTP API is the **universal integration point**. TypeScript,
Python, and any HTTP client talk to this. The MCP server can also
wrap it.

---

## Endpoints

### Primary: Query Execution

```
POST /query
```

**Request:**
```json
{
  "semantic": "flood risk",
  "spatial": {
    "op": "intersects",
    "geometry": { "type": "Polygon", "coordinates": [...] }
  },
  "temporal": {
    "op": "during",
    "start": "2020-01-01",
    "end": "2025-01-01"
  },
  "limit": 20
}
```

**Response:**
```json
{
  "results": [
    {
      "id": "sentinel-2-l2a/S2A_MSIL2A_...",
      "type": "feature",
      "title": "Sentinel-2 L2A",
      "geometry": { "type": "Polygon", "coordinates": [...] },
      "bbox": [14.1, 49.0, 24.2, 54.8],
      "temporal": { "start": "2024-06-15", "end": "2024-06-15" },
      "properties": { "eo:cloud_cover": 3.2, "platform": "sentinel-2a" },
      "assets": [...],
      "relevance": 0.94,
      "provenance": {
        "source": "planetary-computer",
        "protocol": "stac",
        "collection": "sentinel-2-l2a",
        "query": { "bbox": [...], "datetime": "2020/2025" }
      }
    }
  ],
  "sources": [
    { "id": "planetary-computer", "status": "success", "count": 18 },
    { "id": "earth-search", "status": "success", "count": 5 },
    { "id": "private-stac", "status": "timeout" }
  ],
  "warnings": ["private-stac timed out after 5000ms"],
  "status": "partial",
  "total": 23,
  "deduplicated": 19
}
```

---

### Resource Management

```
GET    /resources              List all registered resources
GET    /resources/{id}         Get resource detail + context
POST   /resources              Register a new resource
DELETE /resources/{id}         Remove a resource
```

**GET /resources/{id} response:**
```json
{
  "id": "https://example.org/resources/poland-flood-risk",
  "type": "dataset",
  "title": "Poland Flood Risk Dataset",
  "spatial": { "bbox": [14.1, 49.0, 24.2, 54.8] },
  "temporal": { "start": "2010-01-01", "end": "2025-12-31" },
  "services": [
    { "type": "stac", "url": "https://example.org/stac" }
  ],
  "context": {
    "description": "Flood hazard and risk information...",
    "limitations": "Not suitable for parcel-level insurance...",
    "methodology": "Derived from 100m DEM..."
  },
  "relationships": [
    { "type": "derived-from", "target": "poland-dem" }
  ]
}
```

---

### Service Management

```
GET    /services               List all registered services
GET    /services/{id}          Get service detail + capabilities
```

---

### Geocoding

```
POST   /resolve
```

**Request:**
```json
{ "place": "Warsaw, Poland" }
```

**Response:**
```json
{
  "candidates": [
    {
      "geometry": { "type": "Point", "coordinates": [21.01, 52.23] },
      "bbox": [20.85, 52.10, 21.27, 52.37],
      "confidence": 0.98,
      "display_name": "Warsaw, Masovian Voivodeship, Poland",
      "source": "nominatim"
    }
  ]
}
```

---

## Streaming (SSE)

The HTTP API supports Server-Sent Events for long-running federated
queries:

```
POST /query?stream=true
Accept: text/event-stream
```

**Event stream:**
```
event: query_started
data: {"query_id": "abc123"}

event: source_discovered
data: {"source": "planetary-computer", "protocol": "stac"}

event: source_queried
data: {"source": "planetary-computer", "status": "success", "count": 18}

event: source_queried
data: {"source": "earth-search", "status": "success", "count": 5}

event: source_failed
data: {"source": "private-stac", "error": "timeout"}

event: deduplication_complete
data: {"before": 23, "after": 19}

event: results
data: {"results": [...], "total": 19}

event: query_complete
data: {"duration_ms": 1234}
```

**Design constraint:** Do not couple streaming to query semantics.
The same execution model works synchronously. Streaming is a
transport concern, not a query concern.

→ See [query/planner](../query/planner.md) for the event stream that drives SSE

---

## OpenAPI Specification

The API should be self-documenting via OpenAPI 3.1:

```
GET /openapi.json
GET /docs          (Swagger UI or Redoc)
```

Auto-generated from Axum route definitions using `utoipa` or similar.

---

## Error Responses

Standardized JSON errors:

```json
{
  "error": {
    "type": "validation_error",
    "message": "CRS is ambiguous for coordinates [637123, 487234]",
    "details": {
      "field": "spatial.geometry",
      "code": "AMBIGUOUS_CRS"
    }
  }
}
```

HTTP status codes:
- `400` — Validation error (malformed query, ambiguous CRS)
- `404` — Resource or service not found
- `408` — Query timeout
- `502` — Upstream source failure (with partial results if applicable)
- `207` — Multi-status (partial success, some sources failed)

---

## Implementation

```rust
use axum::{Router, routing::{get, post}};

fn app(engine: GeoqueryEngine) -> Router {
    Router::new()
        .route("/query", post(query_handler))
        .route("/resources", get(list_resources).post(create_resource))
        .route("/resources/{id}", get(get_resource).delete(delete_resource))
        .route("/services", get(list_services))
        .route("/services/{id}", get(get_service))
        .route("/resolve", post(resolve_handler))
        .route("/openapi.json", get(openapi_spec))
        .with_state(engine)
}
```

---

## Related Files

- [query/query-model](../query/query-model.md) — The request body schema
- [project/data-model](../project/data-model.md) — The response body schema
- [interfaces/typescript](typescript.md) — Primary consumer of this API
- [interfaces/python](python.md) — Primary consumer of this API
- [query/planner](../query/planner.md) — Event stream for SSE
