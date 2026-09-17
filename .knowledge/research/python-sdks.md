---
type: Research Findings
title: Python SDK Research & Findings
description: "Findings for pystac-client, OWSLib, FastMCP, GeoPandas, DuckDB."
tags: [Python, pystac, OWSLib, Shapely, GeoPandas, DuckDB, FastMCP, httpx, PyO3]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: research/python-sdks
category: research
refs: [interfaces/python, interfaces/mcp, project/standards, infrastructure/storage]
---

# Python SDK Research & Findings

## Summary

Python has the **most mature geospatial ecosystem** of the three
languages. `pystac-client` and `OWSLib` are established, battle-tested
SDKs that Geoquery's Python wrapper can leverage directly (unlike Rust
and TypeScript, which lack dominant equivalents).

The key architectural decision: the Python SDK starts as an HTTP
client to the Rust server but has a clear path to **PyO3 native
bindings** for zero-copy GeoArrow integration — which is what Python
data scientists actually want.

---

## STAC

| Package | Version | Purpose | Notes |
|---------|---------|---------|-------|
| `pystac` | 1.x | STAC catalog/collection/item model | Read/write/validate STAC objects |
| `pystac-client` | 0.8+ | STAC API search client | **Primary choice** for direct STAC access |

### `pystac-client` Capabilities

```python
from pystac_client import Client

client = Client.open("https://planetarycomputer.microsoft.com/api/stac/v1")
search = client.search(
    bbox=[14.1, 49.0, 24.2, 54.8],
    datetime="2024-06-01/2024-08-31",
    collections=["sentinel-2-l2a"],
    query={"eo:cloud_cover": {"lt": 10}},
    max_items=20,
)
items = list(search.items())
```

**Key features:**
- CQL2 filter support (via `filter` parameter)
- Automatic pagination
- Conformance class detection
- Item collection → GeoDataFrame conversion

**Decision:** For the Python SDK's direct STAC access mode, delegate
to `pystac-client`. For the primary mode, the Python SDK talks to the
Rust HTTP server and doesn't need `pystac-client` at all.

---

## OGC / WFS

| Package | Version | Purpose | Notes |
|---------|---------|---------|-------|
| `OWSLib` | 0.31+ | OGC service client | WFS, WMS, WCS, CSW, OGC API |

### `OWSLib` Capabilities

```python
from owslib.ogcapi.features import Features

wfs = Features("https://example.org/ogc")
collections = wfs.collections()
items = wfs.collection_items("hydrography", bbox=[14.1, 49.0, 24.2, 54.8])
```

**Key features:**
- WFS 2.0 GetFeature with FES filters
- OGC API Features item retrieval
- OGC API Records search
- WMS GetMap, GetCapabilities
- CRS handling

**Limitations:**
- OGC API support is newer and less mature than WFS
- No built-in CQL2 JSON construction
- Async support is limited

**Decision:** `OWSLib` for direct OGC access in the Python SDK's
fallback mode. Primary mode uses the Rust HTTP server.

---

## Geometry & CRS

| Package | Purpose | Notes |
|---------|---------|-------|
| `shapely` | Geometry operations | **Primary** — intersects, within, buffer, distance |
| `geopandas` | GeoDataFrame | **Primary** — spatial data analysis, plotting |
| `pyproj` | CRS transformations | **Primary** — EPSG lookups, reprojection |

### The GeoPandas Integration

This is the **killer feature** for Python users:

```python
from geoquery import Geoquery

geo = Geoquery(url="http://localhost:8080")
gdf = geo.query(
    semantic="satellite imagery",
    spatial={"op": "within", "geometry": poland},
    temporal={"start": "2024-06-01", "end": "2024-08-31"},
).to_geopandas()

print(gdf.crs)        # EPSG:4326
print(gdf.columns)    # id, title, geometry, datetime, cloud_cover, ...
gdf.plot(column="cloud_cover", legend=True)
```

Under the hood:
1. Query results arrive as GeoJSON from the Rust HTTP server
2. `to_geopandas()` converts to GeoDataFrame via `geopandas.GeoDataFrame.from_features()`
3. CRS is set to EPSG:4326
4. Temporal fields are converted to `datetime64`
5. Provenance metadata is attached as `gdf.attrs`

**Decision:** `shapely` + `geopandas` + `pyproj` are optional
dependencies (`pip install geoquery[geo]`). The base SDK works
without them.

---

## Analytics

| Package | Purpose | Notes |
|---------|---------|-------|
| `duckdb` | Local analytical SQL | **Primary** for Tier 2 local analytics |
| `pyarrow` | Arrow columnar data | GeoArrow I/O, zero-copy with DuckDB |
| `geopandas` | GeoParquet I/O | `gdf.to_parquet()` / `gpd.read_parquet()` |

### DuckDB + GeoParquet

```python
import duckdb

# Query local GeoParquet cache
result = duckdb.sql("""
    SELECT * FROM read_parquet('~/.geoquery/cache/sentinel-2-l2a.parquet')
    WHERE ST_Intersects(geometry, ST_GeomFromText('POLYGON(...)'))
      AND datetime >= '2024-06-01'
      AND eo_cloud_cover < 10
""").df()
```

**Decision:** DuckDB as the Python-side analytical backend for local
GeoParquet caches. This is the Phase 6 PyO3 integration path — the
Rust engine and Python DuckDB can share the same GeoParquet files.

---

## PostGIS

| Package | Purpose | Notes |
|---------|---------|-------|
| `GeoAlchemy2` | SQLAlchemy PostGIS extension | ORM-based spatial queries |
| `SQLAlchemy` | SQL ORM | Foundation for GeoAlchemy2 |
| `asyncpg` | Async PostgreSQL | For async PostGIS queries |

**Decision:** Relevant for Tier 3 server deployments where the Python
SDK connects directly to PostGIS. Not part of the MVP.

---

## MCP

| Package | Purpose | Notes |
|---------|---------|-------|
| `mcp` | Official MCP Python SDK | 2026 spec support |
| `FastMCP` | High-level MCP server framework | Built on `mcp`, simpler API |

### FastMCP Example

```python
from mcp.server.fastmcp import FastMCP
from geoquery import Geoquery

mcp = FastMCP("geoquery")
geo = Geoquery(url="http://localhost:8080")

@mcp.tool()
def geo_query(semantic: str = None, bbox: list[float] = None) -> dict:
    """Execute a geospatial query across federated sources."""
    return geo.query(
        semantic=semantic,
        spatial={"op": "bbox", "bbox": bbox} if bbox else None,
    ).to_dict()

@mcp.tool()
def geo_resource(id: str) -> dict:
    """Retrieve metadata and context for a resource."""
    return geo.resource(id).to_dict()

@mcp.tool()
def geo_resolve(place: str) -> dict:
    """Resolve a place name to a geometry."""
    return geo.resolve(place).to_dict()
```

**Decision:** `FastMCP` for Python-side MCP server (alternative to
the Rust `rmcp` server). Useful for data science workflows where
the full Rust engine isn't deployed.

---

## Async / Federation

| Package | Purpose | Notes |
|---------|---------|-------|
| `httpx` | Async HTTP client | **Primary choice** for Python SDK HTTP transport |
| `aiohttp` | Async HTTP client | Alternative, more complex API |
| `asyncio` | Async runtime | Built-in, `asyncio.Semaphore` for bounded concurrency |

### Federation Pattern

```python
import httpx
import asyncio

async def query_sources(sources: list[str], query: dict) -> list[dict]:
    sem = asyncio.Semaphore(10)  # max 10 concurrent

    async def query_one(source: str) -> dict:
        async with sem:
            async with httpx.AsyncClient(timeout=5.0) as client:
                try:
                    resp = await client.post(f"{source}/search", json=query)
                    return {"source": source, "status": "ok", "results": resp.json()}
                except httpx.TimeoutException:
                    return {"source": source, "status": "timeout", "results": []}

    return await asyncio.gather(*[query_one(s) for s in sources])
```

**Decision:** `httpx` for the Python SDK. Modern, async-first, clean
API, good timeout handling.

---

## Semantic Search

| Package | Purpose | Notes |
|---------|---------|-------|
| `sentence-transformers` | Local embeddings | `all-MiniLM-L6-v2` for semantic search |
| `chromadb` | Vector database | Local vector storage and search |
| `qdrant-client` | Qdrant client | External vector DB |

**Decision:** Semantic search is handled by the Rust server (`tantivy`
+ optional HNSW). The Python SDK doesn't need its own embedding
pipeline unless running in standalone mode.

---

## PyO3 Native Path (Phase 6)

| Package | Purpose | Notes |
|---------|---------|-------|
| `maturin` | PyO3 build tool | Builds Rust → Python wheels |
| `pyo3` | Rust ↔ Python FFI | In-process Rust engine access |

### Architecture

```
geoquery-pyo3 (Rust crate, cdylib)
    │
    ├── pyo3 bindings
    ├── geoquery-core (in-process)
    └── geoarrow (zero-copy to pyarrow)
        │
        ▼
Python: import geoquery_native as gq
```

**Key benefit:** Zero-copy GeoArrow transfer from Rust to Python via
the Arrow C Data Interface. Dramatically faster than GeoJSON
serialization for large result sets.

```python
import geoquery_native as gq

engine = gq.Engine(registry="./geoquery")
table = engine.query(
    semantic="flood risk",
    bbox=[14.1, 49.0, 24.2, 54.8],
).to_arrow()  # pyarrow.Table, zero-copy from Rust

gdf = table.to_geopandas()  # GeoArrow → GeoPandas
```

---

## Recommended Python Stack

```
geoquery (base)
├── httpx              # HTTP to Rust server
├── pydantic           # Type validation
└── geojson            # GeoJSON types

geoquery[geo]
├── geopandas          # GeoDataFrame conversion
├── pyarrow            # GeoArrow conversion
└── shapely            # Geometry operations

geoquery[mcp]
└── mcp / FastMCP      # MCP server/client

geoquery-native (Phase 6)
├── pyo3 + maturin     # Rust FFI
└── geoarrow           # Zero-copy transfer
```

---

## Related Files

- [interfaces/python](../interfaces/python.md) — Python SDK design
- [interfaces/mcp](../interfaces/mcp.md) — MCP integration
- [project/standards](../project/standards.md) — Standards position
- [infrastructure/storage](../infrastructure/storage.md) — DuckDB/GeoParquet integration
- [research/rust-crates](rust-crates.md) — Rust ecosystem comparison
- [research/typescript-sdks](typescript-sdks.md) — TypeScript ecosystem comparison
