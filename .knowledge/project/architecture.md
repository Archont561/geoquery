---
type: Architecture
title: Internal Architecture
description: Internal architecture diagram and component responsibilities.
tags: [architecture, layers, engine, components, diagram]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: project/architecture
category: project
refs: [project/overview, project/data-model, query/planner, adapters/adapter-architecture, infrastructure/storage]
---

# Internal Architecture

## High-Level Diagram

```
                    ┌──────────────────┐
                    │     Geoquery     │
                    │      Engine      │
                    └────────┬─────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
          Registry       Query Planner    Index
              │              │              │
       ┌──────┼──────┐       │        ┌─────┼─────┐
       │      │      │       │        │     │     │
      STAC   OGC   YAML      │       FTS  Geo   Vector
       │    Records MD        │
       └──────┬──────┘       │
              │              │
              └──────────────┼──────────────┐
                             │              │
                       Source Adapters      │
                             │              │
             ┌───────────────┼──────────────┤
             │       │       │       │      │
            STAC    OGC     WFS   ArcGIS   CMR
                             │
                             ▼
                       Normalized Results
                             │
                 ┌───────────┼───────────┐
                 │           │           │
                CLI         HTTP        MCP
```

---

## The Execution Pipeline

The important word is **execution**. Geoquery doesn't merely tell you
"here are 50 datasets." It says:

> "These 50 resources are candidates; these 12 can answer your
> spatial/temporal query; I'll translate the query into their native
> protocols, execute it, normalize the results, and return one result set."

### Pipeline Stages

```
GeoQuery AST
    ↓
Discovery — find candidate resources from registry
    ↓
Capability Check — which services support the required operations?
    ↓
Query Partitioning — split query per backend capability
    ↓
Translation — convert GeoQuery to native protocol (STAC /search,
              OGC CQL2, ArcGIS where, WFS FES, CMR query)
    ↓
Parallel Execution — fan out via tokio::JoinSet
    ↓
Normalization — map all responses to GeoResult
    ↓
Deduplication — exact ID, canonical URL, asset identifier
    ↓
Ranking — semantic × spatial × temporal × quality × freshness
    ↓
Provenance Attachment — source, protocol, original query translation
    ↓
Unified Result Stream
```

→ See [query/planner](../query/planner.md) for detailed planner design

### The Build-Time Branch

Discovery is shared by both of Geoquery's modes. The run-time branch is the
pipeline above; the build-time branch reuses the same descriptor:

```
adapter.describe()
      │
      ▼
service snapshot (deterministic, committable)
      │
      ├─► planner           capability matching, offline plans
      ├─► geoquery check    drift detection against the live service
      └─► geoquery generate typed client for that one service
```

Generation performs no discovery and no I/O beyond writing files. Adapters know
nothing about it.
→ See [codegen/service-snapshot](../codegen/service-snapshot.md)

---

## Core Components

| Component | Crate | Responsibility |
|-----------|-------|---------------|
| **Registry** | `geoquery-registry` | Stores known resources and services. May be local, org-wide, embedded, hosted, federated, or generated from static files. MUST NOT require a centralized cloud service. |
| **Query Planner** | `geoquery-planner` | The central intellectual property. Takes `GeoQuery`, produces `ExecutionPlan`. Handles capability matching, query partitioning, and degradation. |
| **Executor** | `geoquery-executor` | Runs the execution plan. Parallel fan-out, timeout management, partial failure handling, result normalization. |
| **Index** | `geoquery-index` | Spatial index (`rstar`), full-text search (`tantivy`), optional vector search. Used for local metadata filtering and semantic discovery. |
| **Adapters** | `geoquery-adapter-*` | Protocol-specific modules. Each implements `ServiceAdapter` trait. Pluggable, independent crates. |
| **Snapshots** | `geoquery-registry` | Serializes what `describe()` found into a deterministic, committable artifact. Enables offline planning, drift detection, and generation. |
| **Codegen** | `geoquery-codegen*` | Optional. Lowers a snapshot into a language-neutral generation model and emits a typed client per target backend. Feature-gated; nothing in the engine depends on it. |

---

## Detailed Component: Registry

The registry contains known resources and services.

**Deployment modes:**
- Local (single user, offline)
- Organization-wide (shared team registry)
- Embedded in an application
- Hosted (SaaS)
- Federated (registry of registries)
- Generated from static files (Git repo of `resource.yaml` files)

**Critical constraint:** A minimal local Geoquery installation must work
entirely offline against previously indexed resources.

→ See [infrastructure/storage](../infrastructure/storage.md) for storage tier details

---

## Detailed Component: Query Planner

The planner is the actual intellectual property of Geoquery.

**Input:** `GeoQuery` AST
**Output:** `ExecutionPlan`

### Example Planning Flow

User asks: *"Find satellite imagery of Warsaw from June 2025, less than
10% cloud cover."*

The planner sees: `spatial + temporal + attribute + resource type`

It knows:
```
Planetary Computer  → supports STAC, intersects, datetime, cloud_cover
CDSE                → supports STAC, intersects, datetime, cloud_cover
USGS                → supports STAC, intersects, datetime, cloud_cover
Local cache         → supports DuckDB, GeoParquet, full spatial
```

It creates:
```
ExecutionPlan
├── planetary-computer → STAC /search
├── cdse               → STAC /search
├── usgs               → STAC /search
└── local-cache        → DuckDB spatial query
```

Then executes them concurrently.

**This is the thing an AI agent cannot realistically do itself.** An LLM
might know how STAC works, but it shouldn't have to understand 40
different GIS protocols.

→ See [query/planner](../query/planner.md) for full planner specification

---

## Detailed Component: Source Adapters

Adapters are plugins/modules. Each implements:

```rust
trait ServiceAdapter {
    fn detect(&self, endpoint: &Endpoint) -> DetectionResult;
    async fn describe(&self, endpoint: &Endpoint) -> Result<ServiceDescriptor>;
    async fn query(&self, service: &ServiceDescriptor, query: &GeoQuery) -> Result<QueryResult>;
    fn capabilities(&self, service: &ServiceDescriptor) -> CapabilitySet;
}
```

**Initial adapters:** STAC API, OGC API Records, OGC API Features, WFS,
ArcGIS REST, NASA CMR, generic HTTP/OpenAPI, Geoquery native.

**Future types must be pluggable.** The adapter interface is the primary
extension point of the entire system.

→ See [adapters/adapter-architecture](../adapters/adapter-architecture.md) for trait details
→ See [extensions/extension-points](../extensions/extension-points.md) for all 15 extension layers

---

## Internal Data Flow

```
User / Agent / Application
        │
        ▼
  GeoQuery AST (canonical, protocol-independent)
        │
        ▼
  ┌─────────────────────────────────────┐
  │          Geoquery Core              │
  │                                     │
  │  Registry ──→ Planner ──→ Executor  │
  │     │            │           │      │
  │   Index      Capabilities  Adapters │
  │                         │          │
  │                    ┌────┴────┐      │
  │                    │ Remote  │      │
  │                    │ Local   │      │
  │                    │ Hybrid  │      │
  │                    └─────────┘      │
  └─────────────────────────────────────┘
        │
        ▼
  Normalized GeoResult[]
  + Provenance
  + Source Status
  + Warnings
        │
        ▼
  CLI / HTTP / MCP / TUI / SDK
```

---

## Execution Modes

Geoquery supports three execution modes:

| Mode | Description | Example |
|------|-------------|---------|
| **Remote** | Query pushed entirely to remote service | STAC `/search` with bbox + datetime + CQL2 |
| **Local** | Query executed against local cache/index | DuckDB + GeoParquet, rstar + tantivy |
| **Hybrid** | Remote approximate + local refinement | Remote bbox filter → local exact intersection |

The hybrid mode is particularly powerful for graceful degradation:

```
User asks: intersects polygon
Source supports: bbox only

Plan:
  1. Remote bbox filter (approximate)
  2. Retrieve candidates
  3. Local exact geometry intersection (refinement)
```

→ See [query/planner](../query/planner.md) for degradation strategies
→ See [infrastructure/storage](../infrastructure/storage.md) for local storage options

---

## Interface Layer

All interfaces consume the same `geoquery-core` API:

```
geoquery-core::execute(GeoQuery) -> QueryResult
    │
    ├── geoquery-cli      (clap → core)
    ├── geoquery-tui      (ratatui → core)
    ├── geoquery-http     (axum → core)
    ├── geoquery-mcp      (rmcp → core)
    ├── geoquery-wasm     (wasm-bindgen → core)
    ├── @archont561/geoquery (N-API → geoquery-engine → core)
    └── geoquery (Python) (PyO3 → geoquery-engine → core)
```

Adding a new interface never requires modifying core.

→ See [interfaces/cli](../interfaces/cli.md), [interfaces/http](../interfaces/http.md), [interfaces/mcp](../interfaces/mcp.md)

---

## Related Files

- [project/overview](overview.md) — Mission and core concept
- [project/data-model](data-model.md) — Type definitions
- [query/planner](../query/planner.md) — Detailed planner specification
- [adapters/adapter-architecture](../adapters/adapter-architecture.md) — Adapter trait design
- [infrastructure/storage](../infrastructure/storage.md) — Storage tiers
- [extensions/extension-points](../extensions/extension-points.md) — All extension mechanisms
