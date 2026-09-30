---
okf_version: "0.2"
---

# Geoquery Knowledge Base

> **One protocol-independent query language and execution engine for
> federating heterogeneous geospatial resources and services.**

Open Knowledge Format (OKF) v0.2 bundle. Start with the [context briefing](CONTEXT.md),
then scope to a group and follow the links — they resolve as ordinary paths.

# Start Here

* [Geoquery — Full Project Context Briefing](CONTEXT.md) - Single-file briefing that gives any reader the complete working mental model of Geoquery.

# Project Foundation

Core identity, architecture, standards alignment, and data model.

* [Geoquery — Project Overview](project/overview.md) - Mission, what Geoquery is and is not, and the four core abstractions.
* [Internal Architecture](project/architecture.md) - Internal architecture diagram and component responsibilities.
* [Standards Position](project/standards.md) - OGC API, STAC, CQL2, DCAT, GeoJSON, MCP — the standards reuse policy.
* [Core Data Model](project/data-model.md) - ResourceDescriptor, ServiceDescriptor, CapabilitySet, and open enums.

# Query Engine

The canonical query AST, predicate languages, and the planner that turns a single query into a federated execution plan.

* [GeoQuery AST — The Canonical Query Model](query/query-model.md) - The GeoQuery AST — scope, spatial, temporal, semantic, filters, execution.
* [Spatial Query Language](query/spatial.md) - Spatial predicates (intersects, dwithin, bbox, and more) and CRS rules.
* [Temporal Query Language](query/temporal.md) - Temporal predicates (during, intersects, open-ended intervals).
* [Attribute Filtering — CQL2 Compilation](query/filters.md) - CQL2 attribute filtering, expression trees, compilation to backends.
* [Semantic Search & Structured Context](query/semantic.md) - Semantic search, embeddings, structured constraints, safety rules.
* [Query Planner & Federation](query/planner.md) - Query planner, federation, capability matching, degradation, ranking.

# Adapters

Protocol-specific adapters that translate the canonical query into source-native requests and normalize responses back.

* [Adapter Architecture — ServiceAdapter Trait](adapters/adapter-architecture.md) - ServiceAdapter trait, detection, capability discovery, plugin model.
* [STAC API Adapter](adapters/stac.md) - STAC API /search, collection discovery, CQL2-JSON, conformance.
* [OGC API Adapter — Features & Records](adapters/ogc.md) - OGC API Features + Records, landing page, queryables, filtering.
* [Geoquery Native Resources — YAML & Markdown](adapters/native.md) - Geoquery Resource Manifest (resource.yaml) and Markdown context sidecar.

# Code Generation

Persisting what discovery found, and compiling it into typed clients.

* [Service Snapshots — Describe Once, Commit, Diff](codegen/service-snapshot.md) - The serialized ServiceDescriptor snapshot, its lockfile, determinism rules, and drift detection.
* [Client Generation — Typed SDKs from a Service Snapshot](codegen/client-generation.md) - geoquery generate: compiling a described service into a typed client in TypeScript, Python, Rust and beyond.

# Interfaces

Every way a user, application, or AI agent interacts with the engine.

* [CLI Interface](interfaces/cli.md) - geoquery add, describe, query, generate, check — command-line UX.
* [HTTP API](interfaces/http.md) - POST /query, resource CRUD, SSE streaming, OpenAPI spec.
* [MCP Server for AI Agents](interfaces/mcp.md) - geo_query, geo_resource, geo_resolve tools; rmcp 3.x; 2026 spec.
* [TUI — Interactive Terminal Interface](interfaces/tui.md) - ratatui TUI — result browser, source dashboard, ASCII map.
* [TypeScript SDK — @geoquery/client](interfaces/typescript.md) - @geoquery/client, fluent builder, ts-rs codegen, WASM path.
* [Python SDK — geoquery](interfaces/python.md) - geoquery Python SDK, GeoPandas integration, PyO3 path.

# Infrastructure

Monorepo tooling, CI, storage tiers, and deployment targets.

* [Rust Monorepo — Workspace & Tooling](infrastructure/monorepo.md) - Workspace layout, Cargo.toml inheritance, tooling hierarchy.
* [xtask — Task Runner & Code Generation](infrastructure/xtask.md) - cargo xtask setup, codegen, fixtures, schema generation.
* [CI Pipeline — GitHub Actions](infrastructure/ci.md) - GitHub Actions, cargo-deny, cargo-nextest, caching.
* [Storage Tiers](infrastructure/storage.md) - Storage tiers 0–3: stateless → SQLite → DuckDB/GeoParquet → PostGIS.
* [Deployment Targets](infrastructure/deployment.md) - Edge (Cloudflare/Deno), Docker, WASM, native binary.

# Extension System

All 15 extension points where Geoquery can be customized without forking core code.

* [Extension Points — All 15 Layers](extensions/extension-points.md) - All 15 extension layers — adapters, storage, auth, ranking, embeddings, geocoders, transforms, MCP tools, events, generator backends.

# Work tracking

Milestones, priorities, dependencies, implementation tasks, and acceptance criteria live in [`backlog/`](../backlog/). They are intentionally not duplicated in this knowledge base.

# Research

Findings from crate/SDK evaluation across Rust, TypeScript, and Python.

* [Rust Crate Research & Findings](research/rust-crates.md) - Findings for geo, rstar, geoarrow, rmcp 3.x, tantivy, duckdb, axum.
* [TypeScript / JavaScript SDK Research & Findings](research/typescript-sdks.md) - Findings for @modelcontextprotocol/server v2, Honua, Turf, DuckDB-WASM.
* [Python SDK Research & Findings](research/python-sdks.md) - Findings for pystac-client, OWSLib, FastMCP, GeoPandas, DuckDB.

# Tag Index

Browse by topic; every tag lives in concept `tags` frontmatter.

* Core concepts: federation, query-plane, abstraction, provenance, normalization
* Standards: STAC, OGC, CQL2, GeoJSON, DCAT, GeoDCAT, OpenAPI
* Query engine: planner, spatial, temporal, semantic, filter, ranking, degradation
* Protocols and adapters: stac-api, ogc-features, ogc-records, wfs, arcgis, cmr, ckan
* Interfaces: cli, http, mcp, tui, typescript, python, wasm
* Code generation: snapshot, lockfile, drift, codegen, typed-client, determinism
* Infrastructure: monorepo, xtask, ci, docker, edge, cloudflare
* Storage and analytics: GeoParquet, GeoArrow, DuckDB, PostGIS, SQLite, rstar, tantivy
* AI and agents: mcp, rmcp, embeddings, semantic, geocoding, natural-language

# Content Ownership

- **`backlog/` owns work:** planned deliverables, sequencing, milestones, priorities, dependencies, task status, and acceptance criteria.
- **`.knowledge/` owns durable context:** architectural decisions, contracts, invariants, standards interpretation, research, and rationale needed to perform that work.
- Knowledge pages may explain *how* or *why* a backlog item should be implemented, but must not maintain a second task list or delivery schedule.

# How to Use

- **Agents:** read this index, scope to a group (each directory has its own `index.md`), then follow markdown links — they resolve as ordinary filesystem paths.
- **Terminal:** `grep -rl "tag-name" .knowledge/` to find files by tag.
- **Conformance:** `python3 .knowledge/tools/validate_okf.py` validates this bundle against OKF v0.2.
- **Frontmatter:** every concept carries OKF fields (`type`, `title`, `description`, `tags`, `status`, `generated`) plus producer keys (`id`, `category`, `refs`, `created`, `updated`).

---

*Bundle history: see [log.md](log.md). Generated from the Geoquery project specification and architecture discussions.*
