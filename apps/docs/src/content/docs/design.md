---
title: Design direction
description: Product and interface design principles for Geoquery as a geospatial federation layer.
---

Geoquery should not try to be another map viewer. The stronger product is a geospatial
source manager, query planner and explanation layer: register sources, understand what they
can do, ask one query, and inspect exactly how the answer was obtained.

This page describes the target design. It includes commands and surfaces that are not all
implemented yet; the [command-line page](./cli/) remains the source for what the binary does
today.

## First slice: CLI first

The first product surface should be the `geoquery` binary, backed by `geoquery-core` and
published as the `geoquery` Conda package on the public `archont561/archont561` prefix.dev
channel. That makes the first release scriptable, CI-friendly and honest about what the core
can do before the HTTP, MCP, Python and TypeScript surfaces become the primary product.

The CLI should remain thin: parse arguments, call core, print core's answer, and pick an exit
code. Query semantics, validation, planning, source status and provenance belong in core so
every later interface receives the same behaviour.

## North-star loop

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

The product should make this loop visible. A federated answer is trustworthy only when the
user can see which sources were considered, which predicates were pushed down, which work
moved local, and where every result came from.

## Design principles

### Explain before trust

Federation should never be magic. A planned `geoquery explain` surface should show per-source
pushdown and degradation before a user waits on execution:

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

### Make degradation first-class

A source can succeed while still degrading part of the work. The product language should
keep these outcomes separate:

| Outcome | Meaning |
| --- | --- |
| `ok` | Source answered with requested pushdown. |
| `degraded` | Source answered, but some work moved local or approximate. |
| `skipped` | Planner did not ask the source. |
| `failed` | Source was asked and failed. |

### Separate snapshots from data caches

A cached service should mean a deterministic service snapshot: capabilities, queryables,
collections, CRS facts, provenance and digest. A cached dataset/result is a different thing,
usually larger and controlled by freshness policy.

```text
~/.geoquery/sources/*.json   service snapshots, committed when useful
~/.geoquery/geoquery.lock    snapshot index and hashes
~/.geoquery/cache/**         optional materialized data/result cache
```

### Keep the AST as the contract

Every surface can have ergonomic helpers, but the portable artifact is still a `GeoQuery`
document. Saved queries should move between CLI, Python, TypeScript, future HTTP, MCP and
TUI without changing meaning.

### Design for agents without making agents special

Agents should use the same structured query model as people. MCP tools expose registered
resources, explanations and query execution; they should not expose arbitrary backend SQL or
unvalidated random URLs.

## Product journeys

| User | Journey | Design implication |
| --- | --- | --- |
| Developer | Add source → inspect capabilities → explain → run | Errors and degradations must be clear before execution. |
| Data scientist | Search catalogs → select results → cache → analyze | Hand off cleanly to Python, DuckDB and GeoParquet workflows. |
| Platform maintainer | Commit snapshots → scheduled check → review drift | Service drift should be a reviewable diff. |
| AI agent | Intent → structured query → validation → explain → execute | The AST and planner are the safety boundary. |

## Interface shape to grow toward

These are target product surfaces, not a statement of current implementation status.

| Task | Target surface |
| --- | --- |
| Register a source | `geoquery add <url-or-manifest>` |
| Inspect a source | `geoquery describe <source>` |
| Explain a query | `geoquery explain query.json` |
| Execute | `geoquery query --source <id> --query query.json` |
| Detect drift | `geoquery diff <source>` / `geoquery check` |
| Materialize data cache | `geoquery cache <source>` |
| Generate a typed client | `geoquery generate <source>` |
| Agent access | MCP `geo_query`, `geo_explain`, `geo_resource` |

The first command a new user tries should often be `describe` or `explain`, not `query`.
Those commands make a heterogeneous system legible.

## Products worth learning from

| Inspiration | Borrow | Avoid |
| --- | --- | --- |
| STAC Browser / STAC API | Catalog hierarchy, assets, bbox/time search | STAC-only assumptions |
| NASA Earthdata Search / CMR | Scientific facets, provenance, provider metadata | Domain-specific complexity in the core |
| Sentinel Hub EO Browser | AOI drawing, timeline, quick preview | Imagery-only identity |
| QGIS Browser | Register once, browse layers, inspect capabilities | Desktop-only assumptions |
| Trino / Presto / Dremio | Connectors, predicate pushdown, explain plans | SQL as the public geospatial API |
| GraphQL / Apollo Federation | One client-facing contract, introspection mindset | GraphQL syntax or resolver model |
| Terraform | Snapshots, drift, reviewable changes | Infrastructure jargon for end users |
| DuckDB + GeoParquet | Local analytical cache | Becoming a database replacement |
| Overpass Turbo / Postman | Query playground and saved requests | Backend-specific request authoring |

The durable design note lives in the knowledge base at
[`.knowledge/project/product-design.md`](https://github.com/Archont561/geoquery/blob/main/.knowledge/project/product-design.md).
