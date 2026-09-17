---
type: Interface Specification
title: "Python SDK — geoquery"
description: "geoquery Python SDK, GeoPandas integration, PyO3 path."
tags: [Python, SDK, GeoPandas, PyO3, httpx, Pydantic, GeoArrow]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: interfaces/python
category: interfaces
refs: [interfaces/http, query/query-model, project/data-model, infrastructure/xtask]
---

# Python SDK — `geoquery`

## Package

`geoquery` — Python SDK for Geoquery.

Targets data scientists, GIS analysts, and AI/ML workflows where
Python is the primary language.

---

## Transport Options

| Option | Status | Use Case |
|--------|--------|----------|
| **HTTP Client** | MVP | Talks to `geoquery-http` via `httpx` |
| **PyO3 Native** | Phase 6 | In-process Rust engine via `maturin` |
| **Subprocess** | Simple | Calls `geoquery-cli` binary |

**Start with HTTP.** Option B (PyO3) is the long-term goal because
Python users want zero-copy GeoArrow/GeoPandas integration, which
requires in-process access to the Rust engine.

```
Option A (MVP):   geoquery (Python)  ──HTTP──▶  geoquery-http (Axum)
Option B (future): geoquery (Python)  ──FFI───▶  geoquery-pyo3 (maturin)
```

---

## Package Structure

```
python/geoquery/
├── pyproject.toml
├── geoquery/
│   ├── __init__.py
│   ├── client.py         # HTTP client (httpx)
│   ├── types.py           # Pydantic models
│   ├── query.py           # Query builder
│   └── geopandas.py       # GeoPandas/GeoArrow converters
└── tests/
```

### `pyproject.toml`

```toml
[project]
name = "geoquery"
version = "0.1.0"
requires-python = ">=3.11"
dependencies = [
    "httpx>=0.28",
    "pydantic>=2.10",
    "geojson>=3.1",
]

[project.optional-dependencies]
geo = ["geopandas>=1.0", "pyarrow>=18.0", "shapely>=2.0"]
mcp = ["mcp>=1.0"]
all = ["geoquery[geo,mcp]"]
```

---

## API Surface

### Initialization

```python
from geoquery import Geoquery

geo = Geoquery(url="http://localhost:8080")
```

### Query

```python
results = geo.query(
    semantic="flood risk",
    spatial={"op": "intersects", "geometry": warsaw_polygon},
    temporal={"start": "2020-01-01", "end": "2025-01-01"},
    limit=20,
)
```

### The Killer Feature: GeoPandas Integration

Python geospatial users live in GeoPandas. The conversion should
be seamless:

```python
gdf = geo.query(
    semantic="satellite imagery",
    spatial={"op": "within", "geometry": poland},
    temporal={"start": "2024-06-01", "end": "2024-08-31"},
).to_geopandas()

print(gdf.columns)
# ['id', 'title', 'geometry', 'bbox', 'datetime', 'cloud_cover',
#  'platform', 'source', 'protocol', 'collection', 'assets']

print(gdf.crs)
# EPSG:4326

gdf.plot(column="cloud_cover", legend=True)
```

Under the hood, `to_geopandas()` converts the GeoJSON response
into a GeoDataFrame with:
- Proper CRS (EPSG:4326)
- Shapely geometry column
- Provenance metadata as DataFrame `.attrs`
- Temporal fields as `datetime64`

### GeoArrow (Zero-Copy)

```python
table = results.to_arrow()  # pyarrow.Table with GeoArrow geometry
```

For large result sets, GeoArrow avoids the GeoJSON serialization
overhead entirely. This becomes dramatically faster with the PyO3
native backend (zero-copy Rust → Python via Arrow C Data Interface).

### Resource Discovery

```python
resources = geo.resources(type="dataset", tag="flood")
resource = geo.resource("copernicus:sentinel-2-l2a")
```

### Context Retrieval (AI Workflows)

```python
context = resource.context()
print(context.limitations)
# "Not suitable for parcel-level insurance. Resolution is 100m."
```

---

## PyO3 Native Path (Phase 6)

When ready for native performance:

```toml
# python/geoquery-pyo3/Cargo.toml
[lib]
name = "geoquery_native"
crate-type = ["cdylib"]

[dependencies]
pyo3 = { version = "0.23", features = ["extension-module"] }
geoquery-core = { path = "../../crates/core" }
geoarrow = "0.4"
```

Built with `maturin`:
```bash
maturin develop  # development
maturin build    # release wheel
```

This enables:
- In-process query execution (no HTTP round-trip)
- Zero-copy GeoArrow transfer from Rust to Python via `pyarrow`
- Direct access to local DuckDB/GeoParquet caches

```python
import geoquery_native as gq

engine = gq.Engine(registry="./geoquery")
gdf = engine.query(semantic="flood risk", bbox=[14.1, 49.0, 24.2, 54.8]).to_geopandas()
```

---

## MCP Integration

Python MCP SDK (`FastMCP` or official `mcp` package) can wrap the
Python client for agent use:

```python
from mcp.server.fastmcp import FastMCP
from geoquery import Geoquery

mcp = FastMCP("geoquery")
geo = Geoquery(url="http://localhost:8080")

@mcp.tool()
def geo_query(semantic: str = None, bbox: list = None) -> dict:
    return geo.query(semantic=semantic, spatial={"op": "bbox", "bbox": bbox}).to_dict()
```

---

## Dependencies

| Package | Purpose | Required? |
|---------|---------|-----------|
| `httpx` | Async HTTP client | ✅ |
| `pydantic` | Type validation | ✅ |
| `geojson` | GeoJSON types | ✅ |
| `geopandas` | GeoDataFrame conversion | Optional (`[geo]`) |
| `pyarrow` | GeoArrow conversion | Optional (`[geo]`) |
| `shapely` | Geometry operations | Optional (`[geo]`) |
| `mcp` | MCP server/client | Optional (`[mcp]`) |

---

## Related Files

- [interfaces/http](http.md) — The HTTP API this SDK wraps
- [query/query-model](../query/query-model.md) — The query AST
- [project/data-model](../project/data-model.md) — GeoResult types
- [query/semantic](../query/semantic.md) — Context retrieval for AI workflows
- [research/python-sdks](../research/python-sdks.md) — Ecosystem findings (pystac, FastMCP)
- [infrastructure/storage](../infrastructure/storage.md) — GeoParquet/DuckDB integration path
