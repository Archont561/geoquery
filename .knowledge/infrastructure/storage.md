---
type: Infrastructure Specification
title: Storage Tiers
description: "Storage tiers 0–3: stateless → SQLite → DuckDB/GeoParquet → PostGIS."
tags: [storage, SQLite, DuckDB, GeoParquet, PostGIS, GeoArrow, rstar, tantivy, tiers]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: infrastructure/storage
category: infrastructure
refs: [project/architecture, project/data-model, query/planner, extensions/extension-points]
---

# Storage Tiers

## Overview

Geoquery supports several storage modes. The query protocol remains
**identical across all tiers** — only the backend changes.

| Tier | Name | Use Case | Dependencies |
|------|------|----------|-------------|
| **0** | Stateless | Small apps, edge, one-shot queries | None (in-memory) |
| **1** | Local Index | Persistent local metadata | SQLite + rstar + tantivy |
| **2** | Analytical | Local spatial analytics | DuckDB + GeoParquet + GeoArrow |
| **3** | Server | Large-scale hosted deployment | PostgreSQL + PostGIS |

**Do not make PostGIS mandatory for the core engine.** Tier 0 must
work with zero external dependencies.

---

## Tier 0 — Stateless

No persistent index.

```
query → discover → execute → return
```

- Registry: in-memory `HashMap<String, ResourceDescriptor>`
- Spatial index: linear scan over resource extents
- Text index: none
- Semantic index: none

**Useful for:** small applications, edge deployments, CLI one-shot
queries, testing.

---

## Tier 1 — Local Metadata Index

Stores resource metadata locally for fast discovery without
re-querying remote services.

### Components

| Component | Technology | Stores |
|-----------|-----------|--------|
| **Registry** | SQLite (`rusqlite`) | Resource descriptors, service descriptors, capabilities |
| **Spatial index** | `rstar` R-tree | Resource bounding boxes for fast overlap queries |
| **Text index** | `tantivy` | Title, description, keywords, themes, markdown context |
| **Vector index** | HNSW (optional) | Embeddings for semantic search |

### Data Stored

- Resource metadata (id, type, title, description, extents)
- Spatial extents (bbox per resource)
- Temporal extents (start/end per resource)
- Keywords and themes
- Service capabilities
- Markdown context (chunked and indexed)
- Embeddings (optional, for semantic search)

### File Layout

```
~/.geoquery/
├── registry.db          # SQLite
├── spatial.idx          # rstar serialized
├── text-index/          # tantivy directory
│   ├── meta.json
│   └── *.fast, *.idx, *.store, *.term
└── vectors.idx          # HNSW (optional)
```

---

## Tier 2 — Spatial Database (Analytical)

Adds local analytical query capability via DuckDB and GeoParquet.

### Components

| Component | Technology | Purpose |
|-----------|-----------|---------|
| **Metadata** | DuckDB | SQL queries over resource metadata |
| **Spatial data** | GeoParquet files | Cached STAC/OGC metadata in columnar format |
| **Interchange** | GeoArrow | Zero-copy columnar spatial data between components |
| **Query engine** | DataFusion (optional) | SQL-based spatial queries over GeoArrow |

### GeoParquet Caching

```bash
geoquery cache https://planetarycomputer.microsoft.com/api/stac/v1
```

Pulls STAC collection metadata into local GeoParquet files:

```
~/.geoquery/cache/
├── planetary-computer/
│   ├── sentinel-2-l2a.parquet    # GeoParquet with spatial partitioning
│   ├── landsat-c2-l2.parquet
│   └── manifest.json             # Cache metadata + freshness
```

### Query Routing

The planner routes queries to the fastest available backend:

```
Query: bbox + datetime + cloud_cover < 10
    │
    ├── Local GeoParquet cache fresh? → DuckDB (10× faster)
    │
    └── Cache stale or missing? → Remote STAC API
```

### GeoArrow as Internal Interchange

For high-performance federation, the internal result representation
uses GeoArrow, not GeoJSON:

```
Source Adapter
    ↓
GeoArrow RecordBatch  (internal interchange)
    ↓
GeoParquet            (local cache / Tier 2 storage)
    ↓
DuckDB                (local analytical queries)
    ↓
GeoJSON               (external output only)
```

GeoJSON remains the **output format** for HTTP/MCP/CLI. Internally,
shuffling GeoJSON strings between adapters, the planner, and the
deduplication layer would be wasteful.

### Rust Crates

| Crate | Purpose |
|-------|---------|
| `duckdb` | DuckDB Rust bindings |
| `geoarrow` | GeoArrow types and operations |
| `parquet` | Parquet file I/O (via `arrow` crate) |
| `datafusion` | SQL query engine over Arrow (optional) |

---

## Tier 3 — Hosted Federation Index

Large-scale SaaS deployment.

### Components

| Component | Technology | Purpose |
|-----------|-----------|---------|
| **Registry** | PostgreSQL | Multi-tenant resource storage |
| **Spatial** | PostGIS | Full spatial indexing and queries |
| **Text** | `tantivy` or Elasticsearch | Full-text search at scale |
| **Vector** | pgvector or Qdrant | Semantic search at scale |
| **Cache** | Redis | Query result caching |

### PostGIS Adapter

Direct SQL execution for maximum spatial query power:

```rust
// geoquery-adapter-postgis
async fn query(&self, service: &ServiceDescriptor, query: &GeoQuery) -> Result<QueryResult> {
    let sql = SqlCompiler::compile(&query.filters)?;
    let rows = sqlx::query(&sql)
        .bind(query.spatial.to_ewkt())
        .fetch_all(&self.pool)
        .await?;
    // Normalize to GeoResult
}
```

Capabilities: full spatial predicate set (`intersects`, `contains`,
`within`, `dwithin`, `nearest`), full temporal, full attribute,
full-text, semantic (via pgvector).

---

## Storage Trait Abstraction

All tiers implement the same traits:

```rust
#[async_trait]
pub trait RegistryStore: Send + Sync {
    async fn get_resource(&self, id: &str) -> Result<Option<ResourceDescriptor>>;
    async fn put_resource(&self, resource: &ResourceDescriptor) -> Result<()>;
    async fn delete_resource(&self, id: &str) -> Result<()>;
    async fn list_resources(&self, filter: &ResourceFilter) -> Result<Vec<ResourceDescriptor>>;
}

#[async_trait]
pub trait SpatialIndex: Send + Sync {
    async fn insert(&self, id: &str, bbox: &BBox) -> Result<()>;
    async fn query_bbox(&self, bbox: &BBox) -> Result<Vec<String>>;
    async fn query_intersects(&self, geometry: &Geometry) -> Result<Vec<String>>;
}

#[async_trait]
pub trait TextIndex: Send + Sync {
    async fn index_document(&self, id: &str, text: &str) -> Result<()>;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>>;
}
```

→ See [extensions/extension-points](../extensions/extension-points.md) Layer 3

---

## Tier Selection

| Scenario | Recommended Tier |
|----------|-----------------|
| CLI one-shot query | 0 |
| Developer laptop, persistent registry | 1 |
| Data scientist with local STAC cache | 2 |
| Edge deployment (Cloudflare Workers) | 0 |
| Organization-wide deployment | 2 or 3 |
| SaaS multi-tenant | 3 |

The engine builder selects the tier:

```rust
let engine = GeoqueryEngine::builder()
    .with_tier(StorageTier::Local { path: "~/.geoquery" })  // Tier 1
    .build();
```

---

## Related Files

- [project/architecture](../project/architecture.md) — Where storage fits in the component diagram
- [query/planner](../query/planner.md) — How the planner routes to local vs remote
- [extensions/extension-points](../extensions/extension-points.md) — Storage as Layer 3 extension
- [research/rust-crates](../research/rust-crates.md) — DuckDB, GeoArrow, rstar, tantivy findings
