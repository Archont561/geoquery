---
type: Query Specification
title: Query Planner & Federation
description: "Query planner, federation, capability matching, degradation, ranking."
tags: [planner, federation, capability, degradation, ranking, deduplication, IP]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: query/planner
category: query
refs: [query/query-model, query/filters, query/semantic, adapters/adapter-architecture, project/data-model, project/architecture]
---

# Query Planner & Federation

## The Planner Is the IP

> The query planner is the actual intellectual property of Geoquery.

An LLM might know how STAC works, but it shouldn't have to understand
40 different GIS protocols. **The agent shouldn't orchestrate GIS
protocols. Geoquery should.**

This is the deliberate contrast with agent-orchestrated approaches
(e.g., Microsoft GeoFaham's specialized agents for PostGIS/OSM/STAC
coordinated by an orchestrator):

```
Their approach:              Geoquery's approach:
LLM                          LLM
 ├── STAC Agent               │
 ├── PostGIS Agent            ▼
 ├── OSM Agent             Geoquery
 └── Raster Agent            ├── STAC adapter
                             ├── PostGIS adapter
                             ├── OSM adapter
                             └── raster adapter
```

---

## Input / Output

- **Input:** `GeoQuery` AST
- **Output:** `ExecutionPlan`

## The Pipeline

```
Discovery
  ↓
Find candidate resources
  ↓
Check capabilities
  ↓
Partition query
  ↓
Translate per backend
  ↓
Execute in parallel
  ↓
Normalize
  ↓
Deduplicate
  ↓
Rank
  ↓
Attach provenance/context
```

---

## Stage 1: Discovery

From the registry, find candidate resources:

```
semantic search locally
        ↓
identify STAC collection
        ↓
push bbox + datetime + cloud-cover filter to STAC
        ↓
return results
```

The planner **prefers pushing filters to remote services**. Local
metadata search identifies *which* services can answer; the remote
service answers *what* matches.

Discovery inputs:
- Registry entries matching `scope`
- Semantic/full-text index hits for `semantic`
- Spatial index (rstar) for extent overlap
- Constraint filtering (`suitable_for` / `not_suitable_for`)
→ See [query/semantic](semantic.md)

---

## Stage 2: Capability Check

Every source adapter produces a `CapabilitySet`:

```json
{
  "protocol": "stac",
  "operations": {
    "spatial": ["intersects", "bbox"],
    "temporal": ["interval"],
    "attribute": ["eq", "lt", "lte", "in"],
    "semantic": false
  },
  "queryables": {
    "eo:cloud_cover": "number",
    "platform": "string",
    "datetime": "datetime"
  }
}
```

The planner reasons about **capabilities, not hardcoded protocol
behavior**. A source that only supports `bbox` is handled the same way
regardless of whether it's WFS or ArcGIS.

→ See [project/data-model](../project/data-model.md) for CapabilitySet
→ See [adapters/adapter-architecture](../adapters/adapter-architecture.md) for capability discovery

---

## Stage 3: Query Partitioning

Split the GeoQuery per backend:

```
QueryPlan
├── planetary-computer → STAC /search  (bbox + datetime + CQL2)
├── cdse               → STAC /search  (bbox + datetime + CQL2)
├── usgs               → STAC /search  (bbox + datetime)
└── local-cache        → DuckDB        (full spatial + attribute)
```

Each branch receives only the predicates its capabilities support;
unsupported predicates either degrade or move to local refinement.

---

## Stage 4: Translation

Per-backend compilation of spatial/temporal/attribute predicates
into native syntax (CQL2 JSON, SQL, ArcGIS where, WFS FES, CMR params).

→ See [query/filters](filters.md) for the compiler design

---

## Stage 5: Parallel Execution

Fan-out via `tokio::JoinSet` with bounded concurrency:

```
ExecutionOptions {
  federate: true,
  maxSources: 20,
  timeoutMs: 5000
}
```

**Failures in one source do not fail the entire query.**

### Partial Failure Response

```json
{
  "status": "partial",
  "sources": {
    "nasa": "ok",
    "copernicus": "ok",
    "private-stac": "timeout"
  }
}
```

**Never hide source failures from applications.** Every response
carries per-source status.

---

## Stage 6: Normalization

Every adapter maps source responses into the common `GeoResult`:

```
STAC Item        → GeoResult { geometry, bbox, temporal, properties, assets }
OGC Feature      → GeoResult { geometry, properties }
ArcGIS Feature   → GeoResult { geometry, properties }
CMR Granule      → GeoResult { bbox, temporal, assets }
```

→ See [project/data-model](../project/data-model.md) for GeoResult

---

## Stage 7: Deduplication

Federation produces duplicates — two providers may expose the same
dataset or asset.

| Strategy | Mechanism |
|----------|-----------|
| Exact ID dedup | Identical resource/result IDs |
| Canonical URL dedup | Normalized asset URLs |
| Asset identifier dedup | Native identifiers (e.g., Sentinel product ID) |
| Semantic dedup | Optional, embedding-similarity based |

**Deduplication must never destroy provenance.** One normalized result
can carry multiple source references.

```
23 raw results → 19 deduplicated (4 merged, provenance preserved)
```

---

## Stage 8: Ranking

Initial ranking combines: semantic relevance, spatial relevance,
temporal relevance, resource quality, source confidence, exact
metadata matches, provider preference, freshness.

**Keep ranking modular** — pluggable `Ranker` trait, no hard-coded
embedding provider.

→ See [extensions/extension-points](../extensions/extension-points.md) Layer 7

---

## Graceful Degradation

The planner's most interesting capability:

```
Exact intersects?     no
BBox filtering?       yes

→ Option 1: approximate (bbox only, false positives accepted)
→ Option 2: remote bbox candidates → local exact intersection
```

```
Query execution:
  Remote spatial filter: approximate
  Local spatial refinement: exact
```

The same pattern applies to semantic search (remote keyword search +
local reranking) and attribute filters (remote partial + local
post-filter).

**The planner must never silently pretend unsupported operations are
supported.** Degradation is always reported.

---

## Execution Modes

| Mode | Description |
|------|-------------|
| **remote** | Filters pushed entirely to the service |
| **local** | Query against local cache/index (DuckDB, rstar, tantivy) |
| **hybrid** | Remote approximate + local exact refinement |
| **auto** | Planner chooses per-source based on capabilities |

The hybrid mode is what makes Geoquery more useful than a proxy:
it combines the breadth of remote federation with the precision of
local computation.

→ See [infrastructure/storage](../infrastructure/storage.md) for local execution backends

---

## Worked Example

**Query:** *"Find satellite imagery of Warsaw from June 2025, less
than 10% cloud cover."*

**Planner sees:** `spatial + temporal + attribute + resource type`

**Planner knows:**

```
Planetary Computer  STAC     intersects ✓  datetime ✓  cloud_cover ✓
CDSE                STAC     intersects ✓  datetime ✓  cloud_cover ✓
USGS                STAC     intersects ✓  datetime ✓  cloud_cover ✓
local-cache         DuckDB   full spatial ✓  datetime ✓  any attribute ✓
```

**Creates:**

```
QueryPlan
├── planetary-computer → STAC /search
├── cdse               → STAC /search
├── usgs               → STAC /search
└── local-cache        → DuckDB spatial query
```

**Executes concurrently → normalizes → dedups → ranks → returns one
result set with per-source provenance.**

**This is the thing an agent cannot realistically do itself.**

---

## Event Stream

The planner emits lifecycle events (for TUI, SSE, telemetry):

```
QueryEvent::QueryReceived
QueryEvent::DiscoveryStarted { candidate_count }
QueryEvent::SourceQueried { source, status }
QueryEvent::ResultsReceived { source, count }
QueryEvent::DeduplicationComplete { before, after }
QueryEvent::SourceFailed { source, error }
QueryEvent::QueryComplete { total, duration }
```

→ See [extensions/extension-points](../extensions/extension-points.md) Layer 13

---

## Related Files

- [query/query-model](query-model.md) — The input AST
- [query/filters](filters.md) — Translation/compilation stage
- [query/semantic](semantic.md) — Constraint-aware discovery
- [adapters/adapter-architecture](../adapters/adapter-architecture.md) — The adapters being planned against
- [project/data-model](../project/data-model.md) — GeoResult, CapabilitySet, Provenance
- [project/architecture](../project/architecture.md) — Where the planner sits
- [extensions/extension-points](../extensions/extension-points.md) — Ranker, transformer, event hooks
