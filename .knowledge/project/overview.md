---
id: project/overview
title: Geoquery — Project Overview
category: project
tags: [mission, concept, federation, abstraction, query-plane]
refs: [project/architecture, project/standards, project/data-model, query/planner]
status: draft
created: 2025-07-11
updated: 2025-07-11
---

# Geoquery — Project Overview

## Mission

Build **Geoquery**, a universal geospatial query interface that allows
applications, humans, and AI agents to query a heterogeneous universe of
known GIS resources and services through **one protocol-independent query
model**.

## The Core Sentence

> **Geoquery provides one protocol-independent query language and execution
> engine for federating heterogeneous geospatial resources and services.**

---

## What Geoquery IS

- A **federated query and discovery layer** over existing geospatial
  resources and services.
- A **query plane** that sits one level above STAC, OGC API, WFS, ArcGIS,
  CMR, and other GIS standards.
- An **abstraction engine** that:
  1. Discovers service capabilities
  2. Normalizes heterogeneous metadata
  3. Translates a common query into source-specific queries
  4. Executes them in parallel where possible
  5. Combines and deduplicates results
  6. Returns normalized results with full provenance
- A **single semantic operation** accessible through every interface:
  CLI, TypeScript, Python, HTTP, MCP, TUI.

## What Geoquery is NOT

- **Not a replacement** for STAC, OGC API, WFS, ArcGIS, CMR, or any
  existing GIS standard. The underlying systems remain heterogeneous.
  → See [project/standards](standards.md)
- **Not a catalog or index product.** The core is a federated query engine,
  not a metadata repository. Existing catalogs (STAC, OGC Records) remain
  the source of truth.
- **Not a map renderer.** Geoquery returns GeoJSON, bounding boxes, assets,
  links, and metadata. Rendering is delegated to UI layers (MapLibre,
  Leaflet, OpenLayers, deck.gl).
- **Not a download manager.** Asset download is a separate capability
  exposed through result metadata, not the query engine itself.
- **Not an LLM wrapper.** The LLM generates the Geoquery AST; Geoquery
  validates and executes it. The LLM never generates backend-specific SQL
  or CQL2 directly.

---

## The Four Core Abstractions

Everything in Geoquery revolves around four concepts:

| # | Abstraction | Question | Key Type |
|---|-------------|----------|----------|
| 1 | **Resource** | What exists? | `ResourceDescriptor` |
| 2 | **Service** | How can it be queried/accessed? | `ServiceDescriptor` |
| 3 | **Query** | What does the user/agent want? | `GeoQuery` |
| 4 | **Plan** | How does Geoquery obtain the answer? | `ExecutionPlan` |

→ See [project/data-model](data-model.md) for type definitions
→ See [query/query-model](../query/query-model.md) for the query AST
→ See [query/planner](../query/planner.md) for the planning and execution pipeline

---

## The Fundamental Model

```
Resource
    │
    ├── metadata
    ├── spatial extent
    ├── temporal extent
    ├── semantic context
    ├── relationships
    ├── capabilities
    └── services
             │
             ├── STAC
             ├── OGC API Features
             ├── OGC API Records
             ├── WFS
             ├── ArcGIS REST
             ├── CMR
             ├── CKAN
             ├── custom APIs
             └── local databases
```

A **resource** describes something Geoquery knows about.
A **service** describes how that resource can be queried, accessed,
downloaded, rendered, or otherwise used.
**Geoquery owns the abstraction between them.**

---

## Access Interfaces

The same query engine must be accessible through:

| Interface | Crate / Package | Primary User |
|-----------|----------------|--------------|
| Rust CLI | `geoquery-cli` | Developers, scripts, CI |
| TUI | `geoquery-tui` | Interactive exploration |
| HTTP API | `geoquery-http` | Applications, microservices |
| MCP Server | `geoquery-mcp` | AI agents |
| TypeScript SDK | `@geoquery/client` | Web/Node applications |
| Python SDK | `geoquery` | Data science, GIS workflows |
| WASM (future) | `geoquery-wasm` | Browser, edge |

→ See [interfaces/cli](../interfaces/cli.md), [interfaces/http](../interfaces/http.md), [interfaces/mcp](../interfaces/mcp.md),
  [interfaces/tui](../interfaces/tui.md), [interfaces/typescript](../interfaces/typescript.md), [interfaces/python](../interfaces/python.md)

---

## Primary User Experience

### Step 1: Register Resources

```bash
geoquery add https://example.org/stac
geoquery add https://example.org/ogc
geoquery add ./resource.yaml
geoquery add ./catalog/
```

Geoquery detects the source type and capabilities automatically.
→ See [adapters/adapter-architecture](../adapters/adapter-architecture.md) for detection logic

### Step 2: Query

```bash
geoquery query \
  --semantic "flood risk" \
  --near "Vistula" \
  --within 50km \
  --time 2020:2025
```

The system determines:
- Which known resources are relevant
- Which services can answer the query
- How the common query maps to each service
- Which queries can be executed remotely
- Which metadata must be searched locally
- How results should be normalized, ranked, and deduplicated
- What provenance must be returned

**The user does not need to know whether a result came from STAC, WFS,
OGC API Features, ArcGIS, CMR, or a Geoquery-native resource.**

---

## Agent Experience

AI agents access the same query engine through MCP.

```json
{
  "semantic": "datasets useful for flood modelling",
  "spatial": {
    "op": "dwithin",
    "geometry": "...",
    "distance": "50km"
  },
  "temporal": {
    "start": "2020-01-01",
    "end": "2025-01-01"
  },
  "limit": 20
}
```

**The agent does not choose STAC vs WFS vs ArcGIS. Geoquery chooses.**

→ See [interfaces/mcp](../interfaces/mcp.md) for MCP tool design
→ See [query/semantic](../query/semantic.md) for AI safety and correctness rules

---

## Competitive Landscape

| Approach | Example | Geoquery's Difference |
|----------|---------|----------------------|
| Individual source adapters | Geo-MCP servers on GitHub | Geoquery is a **federation layer**, not a single-source adapter |
| Agent-orchestrated GIS | Microsoft GeoFaham | Geoquery orchestrates protocols; the **agent shouldn't have to** |
| Catalog products | STAC catalogs, OGC Records | Geoquery is a **query engine**, not a catalog |
| Protocol-specific SDKs | `pystac-client`, `OWSLib` | Geoquery is **protocol-independent** |

The gap Geoquery fills: there are mature standards (STAC, OGC API Records,
CQL2) and proliferating MCP servers, but **no universal federation layer**
that sits above them all.

→ See [research/rust-crates](../research/rust-crates.md), [research/typescript-sdks](../research/typescript-sdks.md),
  [research/python-sdks](../research/python-sdks.md) for ecosystem research

---

## Related Files

- [project/architecture](architecture.md) — Internal component diagram
- [project/standards](standards.md) — Standards reuse policy
- [project/data-model](data-model.md) — Core type definitions
- [query/planner](../query/planner.md) — The query planner (the actual IP)
- [roadmap/roadmap](../roadmap/roadmap.md) — Phased delivery plan
