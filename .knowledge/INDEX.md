---
id: index
title: Geoquery Knowledge Base — Master Index
category: meta
tags: [index, navigation, toc]
refs: []
status: active
created: 2025-07-11
updated: 2025-07-11
---

# Geoquery Knowledge Base

> **One protocol-independent query language and execution engine for
> federating heterogeneous geospatial resources and services.**

This knowledge base captures the full architecture, design decisions,
research findings, and implementation roadmap for the Geoquery project.
All files use YAML frontmatter and `[wiki-links](wiki-links.md)` for cross-referencing.

---

## Project Foundation

Core identity, architecture, standards alignment, and data model.

| File | Description |
|------|-------------|
| [project/overview](project/overview.md) | Mission, what Geoquery is and is not, four core abstractions |
| [project/architecture](project/architecture.md) | Internal architecture diagram, component responsibilities |
| [project/standards](project/standards.md) | OGC API, STAC, CQL2, DCAT, GeoJSON, MCP — reuse policy |
| [project/data-model](project/data-model.md) | `ResourceDescriptor`, `ServiceDescriptor`, `CapabilitySet`, open enums |

---

## Query Engine

The canonical query AST, predicate languages, and the planner that
turns a single query into a federated execution plan.

| File | Description |
|------|-------------|
| [query/query-model](query/query-model.md) | `GeoQuery` AST — scope, spatial, temporal, semantic, filters, execution |
| [query/spatial](query/spatial.md) | Spatial predicates (`intersects`, `dwithin`, `bbox`, etc.), CRS rules |
| [query/temporal](query/temporal.md) | Temporal predicates (`during`, `intersects`, open-ended intervals) |
| [query/filters](query/filters.md) | CQL2 attribute filtering, expression trees, compilation to backends |
| [query/semantic](query/semantic.md) | Semantic search, embeddings, structured constraints, safety rules |
| [query/planner](query/planner.md) | Query planner, federation, capability matching, degradation, ranking |

---

## Adapters

Protocol-specific adapters that translate the canonical query into
source-native requests and normalize responses back.

| File | Description |
|------|-------------|
| [adapters/adapter-architecture](adapters/adapter-architecture.md) | `ServiceAdapter` trait, detection, capability discovery, plugin model |
| [adapters/stac](adapters/stac.md) | STAC API `/search`, collection discovery, CQL2-JSON, conformance |
| [adapters/ogc](adapters/ogc.md) | OGC API Features + Records, landing page, queryables, filtering |
| [adapters/native](adapters/native.md) | Geoquery Resource Manifest (`resource.yaml`), Markdown context sidecar |

---

## Interfaces

Every way a user, application, or AI agent interacts with the engine.

| File | Description |
|------|-------------|
| [interfaces/cli](interfaces/cli.md) | `geoquery add`, `geoquery query`, `geoquery sources`, `geoquery explore` |
| [interfaces/http](interfaces/http.md) | `POST /query`, resource CRUD, SSE streaming, OpenAPI spec |
| [interfaces/mcp](interfaces/mcp.md) | `geo_query`, `geo_resource`, `geo_resolve` tools; `rmcp` 3.x; 2026 spec |
| [interfaces/tui](interfaces/tui.md) | ratatui TUI — result browser, source dashboard, ASCII map |
| [interfaces/typescript](interfaces/typescript.md) | `@geoquery/client`, fluent builder, `ts-rs` codegen, WASM path |
| [interfaces/python](interfaces/python.md) | `geoquery` Python SDK, GeoPandas integration, PyO3 path |

---

## Infrastructure

Monorepo tooling, CI, storage tiers, and deployment targets.

| File | Description |
|------|-------------|
| [infrastructure/monorepo](infrastructure/monorepo.md) | Workspace layout, `Cargo.toml` inheritance, tooling hierarchy |
| [infrastructure/xtask](infrastructure/xtask.md) | `cargo xtask` setup, codegen, fixtures, schema generation |
| [infrastructure/ci](infrastructure/ci.md) | GitHub Actions, `cargo-deny`, `cargo-nextest`, caching |
| [infrastructure/storage](infrastructure/storage.md) | Tiers 0–3: stateless → SQLite → DuckDB/GeoParquet → PostGIS |
| [infrastructure/deployment](infrastructure/deployment.md) | Edge (Cloudflare/Deno), Docker, WASM, native binary |

---

## Extension System

All 14 extension points where Geoquery can be customized without
forking core code.

| File | Description |
|------|-------------|
| [extensions/extension-points](extensions/extension-points.md) | Adapters, storage, auth, ranking, embeddings, geocoders, transforms, MCP tools, events |

---

## Roadmap

Phased delivery plan from foundation to ecosystem.

| File | Description |
|------|-------------|
| [roadmap/roadmap](roadmap/roadmap.md) | 7 phases, exit demos, critical path, cut list |

---

## Research

Findings from crate/SDK evaluation across Rust, TypeScript, and Python.

| File | Description |
|------|-------------|
| [research/rust-crates](research/rust-crates.md) | `geo`, `rstar`, `geoarrow`, `rmcp` 3.x, `tantivy`, `duckdb`, `axum` |
| [research/typescript-sdks](research/typescript-sdks.md) | `@modelcontextprotocol/server` v2, Honua, Turf, DuckDB-WASM |
| [research/python-sdks](research/python-sdks.md) | `pystac-client`, `OWSLib`, `FastMCP`, GeoPandas, DuckDB |

---

## Tag Index

Browse by topic across all 31 files.

### Core Concepts
`federation` · `query-plane` · `abstraction` · `provenance` · `normalization`

### Standards
`STAC` · `OGC` · `CQL2` · `GeoJSON` · `DCAT` · `GeoDCAT` · `OpenAPI`

### Query Engine
`planner` · `spatial` · `temporal` · `semantic` · `filter` · `ranking` · `degradation`

### Protocols & Adapters
`stac-api` · `ogc-features` · `ogc-records` · `wfs` · `arcgis` · `cmr` · `ckan`

### Interfaces
`cli` · `http` · `mcp` · `tui` · `typescript` · `python` · `wasm`

### Infrastructure
`monorepo` · `xtask` · `ci` · `docker` · `edge` · `cloudflare`

### Storage & Analytics
`GeoParquet` · `GeoArrow` · `DuckDB` · `PostGIS` · `SQLite` · `rstar` · `tantivy`

### AI & Agents
`mcp` · `rmcp` · `embeddings` · `semantic` · `geocoding` · `natural-language`

### Rust Ecosystem
`geo` · `geo-types` · `geos` · `proj` · `geojson` · `rstar` · `kiddo` · `h3o`
`geoarrow` · `datafusion` · `duckdb` · `tantivy` · `rmcp` · `axum` · `reqwest`
`tokio` · `tower` · `clap` · `serde` · `chrono` · `thiserror` · `tracing`

### TypeScript Ecosystem
`@modelcontextprotocol/server` · `@modelcontextprotocol/client` · `@honua/sdk-js`
`turf` · `rbush` · `proj4` · `ofetch` · `duckdb-wasm` · `hyparquet` · `minisearch`

### Python Ecosystem
`pystac` · `pystac-client` · `OWSLib` · `shapely` · `geopandas` · `pyproj`
`duckdb` · `pyarrow` · `geoalchemy2` · `FastMCP` · `httpx`

---

## File Statistics

| Metric | Count |
|--------|-------|
| Total files | 31 |
| Categories | 8 |
| Cross-references | ~90 |
| Unique tags | ~60 |

---

## How to Use

  resolve automatically.
- **Terminal:** `grep -rl "tag-name" .knowledge/` to find files by tag.
- **Generation:** Run `./00-index.sh` through `./08-research.sh` to regenerate.
- **Frontmatter:** Every file has `id`, `title`, `category`, `tags`, `refs`,
  `status`, `created`, `updated`.

---

*Generated from Geoquery project specification and architecture discussions.*
