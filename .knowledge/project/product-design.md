---
type: Product Design
title: "Product Design Direction"
description: "The product shape Geoquery should grow into: user loops, interaction principles, and which inspirations to borrow from."
tags: [product, design, ux, federation, provenance, explain, snapshots]
status: draft
generated: { by: agent/arena, at: 2026-10-05T00:00:00Z }
created: 2026-10-05T00:00:00Z
updated: 2026-10-05T00:00:00Z
id: project/product-design
category: project
refs: [project/overview, project/architecture, query/planner, codegen/service-snapshot, infrastructure/storage, interfaces/cli, interfaces/mcp]
---

# Product Design Direction

## North Star

Geoquery should start as a **CLI-first federated search tool** backed by `geoquery-core`, not
another map viewer. The first deliverable is the `geoquery` binary, packaged as
`geoquery` and published to the public `archont561/archont561` prefix.dev channel. The
CLI is the proving ground because it is scriptable, CI-friendly, and hard to hide vague
semantics behind.

The UI or SDK a user touches later should preserve this loop:

```text
install geoquery
      ↓
register or load sources
      ↓
describe their capabilities
      ↓
author one GeoQuery document
      ↓
run one federated query
      ↓
inspect normalized results, source status, provenance and degradations
      ↓
explain, snapshot, diff, cache or generate only when the workflow needs it
```

The distinctive product promise is not that Geoquery draws a better map. It is that a user,
application, or agent can ask one geospatial question and receive an answer that says:

1. which sources were considered;
2. which predicates were pushed down;
3. which parts were approximated or run locally;
4. where every result came from; and
5. whether the service contract has drifted since it was last described.

---

## Design Principles

### 1. Explain before trust

Federation is invisible when it works and dangerous when it silently degrades. Every user
surface should expose an explanation path before or alongside execution:

```bash
geoquery explain query.json
```

```text
planetary-computer
  pushdown: bbox, datetime, cloud_cover < 10
  local:    none

example-wfs
  pushdown: bbox
  local:    cloud_cover < 10
  note:     attribute filtering not advertised

old-arcgis
  skipped: temporal filtering required but not supported
```

A result without a plan is only a list. A result with a plan is auditable.

### 2. Make degradation first-class

A degraded source is not a failed source. If the engine refines locally after broad remote
retrieval, the answer may still be correct. The design must separate:

| Outcome | Meaning | User action |
|---------|---------|-------------|
| `ok` | Source answered with requested pushdown | Trust normally |
| `degraded` | Source answered, but some work moved local or approximate | Inspect cost/precision |
| `skipped` | Planner did not ask the source | Decide whether scope or source capability is wrong |
| `failed` | Source was asked and failed | Retry, fix auth, or report service issue |

This matches `ExecutionOutcome`, `SourceStatus`, and `CapabilityFinding` in `geoquery-core`.

### 3. Separate service snapshots from data caches

A cached service is a **contract snapshot**: `ServiceDescriptor`, `CapabilitySet`, queryable
schema, collections, CRS facts, provenance, and digest. It is small, deterministic, and safe
to commit.

A cached result/data layer is different: GeoParquet, DuckDB, or another analytical cache. It
can be large, stale, private, and controlled by freshness policy. The UI must not call both
things simply "cache" without context.

```text
~/.geoquery/sources/*.json   service snapshots, committed when useful
~/.geoquery/geoquery.lock    snapshot index and hashes
~/.geoquery/cache/**         optional materialized data/result cache
```

### 4. Prefer source registration over ad-hoc URLs

An ad-hoc URL is useful for a quick command, but the designed workflow is registration:

```bash
geoquery add https://planetarycomputer.microsoft.com/api/stac/v1
geoquery describe planetary-computer
geoquery query --source planetary-computer query.json
```

Registration lets Geoquery reuse discovered capabilities, diff service drift, hide
credentials behind profiles, and present human-readable source names.

### 5. Keep the AST as the contract

Every interface should provide ergonomic helpers, but the portable artifact is still a
`GeoQuery` document. A saved query should move between CLI, Python, TypeScript, HTTP, MCP,
and future TUI without changing meaning.

### 6. Design for agents without making agents special

Agents get the same structured API as people. MCP tools should expose the query model,
registry resources, explanations, and provenance; they should not expose raw backend SQL,
random URLs, or unvalidated protocol-specific parameters.

---

## Primary User Journeys

### Developer: register and query a source

```text
add source → inspect capabilities → save query.json → explain → run → inspect provenance
```

The most important affordance is fast feedback when a query cannot be pushed down. A
developer should learn that before waiting on a large remote scan.

### Data scientist: discover then materialize

```text
search catalogs → review results → cache selected metadata/assets → analyze in Python/DuckDB
```

Geoquery should hand off naturally to GeoPandas, xarray, DuckDB, GeoParquet, Rasterio, and
notebook workflows. It does not need to replace them.

### Data platform maintainer: watch service drift

```text
describe services → commit snapshots → scheduled check → review drift → refresh intentionally
```

This is the Terraform-like workflow. A removed collection or capability is a breaking
change; an added field is usually additive.

### AI agent: turn intent into a safe query

```text
natural-language task → structured GeoQuery → validation → explain → execute registered sources
```

The agent may propose the AST, but Geoquery owns validation, planning, service selection,
and provenance.

---

## Interface Shape to Grow Toward

These are product design targets, not a claim that all commands exist today.

| Task | Target surface | Product note |
|------|----------------|--------------|
| Register a source | `geoquery add <url-or-manifest>` | Detect adapter and write a snapshot |
| Inspect a source | `geoquery describe <source>` | Show capabilities, queryables, collections, CRS |
| Explain a query | `geoquery explain query.json` | Show pushdown, local work, skips and failures |
| Execute | `geoquery query --source <id> --query query.json` | Return normalized results with provenance |
| Detect drift | `geoquery diff <source>` / `geoquery check` | Snapshot vs live service |
| Materialize cache | `geoquery cache <source>` | Explicit data cache, separate from snapshots |
| Generate client | `geoquery generate <source>` | Typed clients from committed snapshots |
| Agent access | MCP `geo_query`, `geo_explain`, `geo_resource` | Same AST and same registry |

The first command a new user tries should not be `query`. It should often be `describe` or
`explain`, because those build trust in heterogeneous systems.

---

## Inspiration Map

| Inspiration | Borrow | Do not borrow |
|-------------|--------|---------------|
| STAC Browser / STAC API | Catalog hierarchy, assets, bbox/time search | STAC-only assumptions |
| NASA Earthdata Search / CMR | Scientific facets, provenance, provider seriousness | Heavy domain-specific UX in the core |
| Sentinel Hub EO Browser | Draw AOI, timeline, quick visual preview | Imagery-only product identity |
| QGIS Browser | Register once, browse layers, inspect capabilities | Desktop-only mental model |
| Trino / Presto / Dremio | Connector model, predicate pushdown, `EXPLAIN` | SQL as the public geospatial API |
| GraphQL / Apollo Federation | One client-facing contract, introspection mindset | GraphQL syntax and object-graph coupling |
| Terraform | Snapshot, drift, reviewable change | Infrastructure terminology for end users |
| DuckDB + GeoParquet | Local analytical cache and zero-server workflows | Treating Geoquery as a database replacement |
| Overpass Turbo / Postman | Query playground, saved requests, response inspection | Protocol-specific request authoring |
| Hugging Face Hub | Dataset cards, usage context, trust signals | Popularity metrics as correctness signals |

---

## What the Product Should Not Become

- A full map renderer. Use MapLibre, Leaflet, OpenLayers, deck.gl, QGIS, or ArcGIS for
  rendering.
- A raster processing platform. Hand results to xarray, Rasterio, GDAL, or cloud-native
  processing tools.
- A PostGIS replacement. PostGIS remains a powerful source and execution target.
- A single-catalog portal. Catalogs are sources; the Geoquery product is the federated query
  and explanation layer above them.
- An LLM prompt wrapper. The AST, validator, planner, and registered capabilities are the
  trust boundary.

---

## Related Files

- [overview](overview.md) — mission and core abstractions
- [architecture](architecture.md) — internal components
- [query/planner](../query/planner.md) — pushdown, degradation and federation
- [codegen/service-snapshot](../codegen/service-snapshot.md) — service snapshots and drift
- [infrastructure/storage](../infrastructure/storage.md) — registry and cache tiers
- [interfaces/cli](../interfaces/cli.md) — command-line UX target
- [interfaces/mcp](../interfaces/mcp.md) — agent-facing tool design
