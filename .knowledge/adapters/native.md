---
id: adapters/native
title: Geoquery Native Resources — YAML & Markdown
category: adapters
tags: [native, YAML, Markdown, resource-manifest, context, sidecar]
refs: [adapters/adapter-architecture, project/data-model, query/semantic, extensions/extension-points]
status: draft
created: 2025-07-11
updated: 2025-07-11
---

# Geoquery Native Resources — YAML & Markdown

## Overview

Geoquery MUST support a simple declarative resource format for
resources that don't have a live API endpoint. This is the
**Geoquery Resource Manifest**.

A native resource is a directory containing:

```
flood-risk/
├── resource.yaml          # Structured metadata (required)
├── README.md              # Human/LLM-readable description
├── limitations.md         # Usage constraints
├── methodology.md         # How the data was produced
├── usage.md               # Recommended applications
└── examples.md            # Query examples
```

**YAML provides structured metadata. Markdown provides human- and
LLM-readable context. Both are independently indexed.**

---

## Resource Manifest Format

Suggested filename: `resource.yaml`

### Full Example

```yaml
id: https://example.org/resources/poland-flood-risk

type: dataset

title: Poland Flood Risk Dataset

description: >
  Flood hazard and risk information for Poland. Covers riverine
  and coastal flooding at 100m resolution.

spatial:
  bbox:
    - 14.1
    - 49.0
    - 24.2
    - 54.8

temporal:
  start: 2010-01-01
  end: 2025-12-31

themes:
  - flooding
  - hydrology
  - risk

keywords:
  - flood hazard
  - flood risk
  - Poland
  - Vistula

provider:
  name: Polish Institute of Meteorology and Water Management
  url: https://www.imgw.pl

license:
  spdx: CC-BY-4.0

services:
  - type: ogc-api-features
    url: https://example.org/api/features
  - type: stac
    url: https://example.org/stac

context:
  - ./README.md
  - ./limitations.md
  - ./methodology.md

related:
  - https://example.org/resources/poland-rivers
  - https://example.org/resources/poland-dem

constraints:
  suitable_for:
    - regional flood modelling
    - watershed planning
  not_suitable_for:
    - parcel insurance
    - building-level flood prediction
  resolution:
    minimum_reliable_scale: 100m
  temporal:
    historical_only: false

query_examples:
  - question: Find flood datasets around the Vistula
    query:
      semantic: flood
      spatial:
        op: intersects
        geometry: Vistula
  - question: Find low-cloud Sentinel imagery
    query:
      filters:
        cloud_cover:
          lt: 10
```

---

## Markdown Context Sidecar

Markdown is **first-class**, not merely a long description field.

### Why It Matters

An agent asks: *"Can I use this dataset for individual property
insurance?"*

The YAML metadata says `type: dataset, themes: [flooding]`. That's
not enough to answer the question.

But `limitations.md` says:

```markdown
# Usage

This dataset is suitable for regional flood modelling.

It should not be used for parcel-level insurance decisions
because the native resolution is 100m.

For historical comparison use the 2010–2020 archive.
```

Now the agent has a definitive answer. **This is far more valuable
than ordinary catalog metadata.**

### Context Categories

| File | Content | Indexed For |
|------|---------|-------------|
| `README.md` | Overview, purpose | Discovery, semantic search |
| `limitations.md` | What NOT to do | Constraint matching, agent Q&A |
| `methodology.md` | How data was produced | Provenance, quality assessment |
| `usage.md` | Recommended applications | Suitability matching |
| `examples.md` | Query examples | Agent guidance |

### Indexing

Each markdown file is:
1. Parsed and chunked
2. Indexed in the full-text search index (`tantivy`)
3. Optionally embedded for semantic search
4. Associated with the parent resource ID

When an agent calls `geo_resource({ id: "..." })`, the relevant
context is retrieved and returned alongside structured metadata.

---

## Structured Constraints

Beyond prose, the YAML `constraints` field provides machine-readable
domain knowledge:

```yaml
constraints:
  suitable_for:
    - regional flood modelling
    - watershed planning
  not_suitable_for:
    - parcel insurance
    - building-level flood prediction
  resolution:
    minimum_reliable_scale: 100m
```

The planner uses these during discovery:

```
Agent asks: "Find datasets suitable for urban flood modelling"
    ↓
Semantic search → 15 candidates
    ↓
Constraint filter → 3 excluded (not_suitable_for: urban flood)
    ↓
12 candidates proceed to query execution
```

**YAML constrains. Markdown explains.**

→ See [query/semantic](../query/semantic.md) for constraint-aware planning

---

## Query Examples as Context

Resources may declare example queries:

```yaml
query_examples:
  - question: Find flood datasets around the Vistula
    query:
      semantic: flood
      spatial:
        op: intersects
        geometry: Vistula
```

These examples:
- Are indexed and supplied to agents on request
- Serve as **guidance, not executable permissions**
- Help agents construct correct GeoQuery ASTs for specific resources
- Can be used for few-shot prompting in NL → GeoQuery translation

---

## Discovery

The native adapter detects resources by looking for:

| Input | Detection |
|-------|-----------|
| `./resource.yaml` | Single resource manifest |
| `./catalog/` (directory) | Multiple manifests, recursive scan |
| Git repository | Clone + scan for `resource.yaml` files |
| `./README.md` (no YAML) | Context-only resource, type: `context` |

```bash
geoquery add ./resource.yaml        # single resource
geoquery add ./catalog/             # directory of resources
geoquery add https://github.com/org/geo-catalog  # Git repo
```

---

## Capabilities

Native resources have limited capabilities:

```json
{
  "spatial": ["bbox", "intersects"],
  "temporal": true,
  "attribute": false,
  "fullText": true,
  "semantic": true,
  "sorting": false,
  "pagination": false
}
```

Spatial and temporal filtering happen against the **metadata extents**
in the registry index, not against the underlying data. For actual
data queries, the native resource's `services` field points to a
live API (STAC, OGC, etc.) that the planner queries instead.

---

## Relationship to Other Standards

| Standard | Geoquery Native Relation |
|----------|------------------------|
| STAC Catalog | `resource.yaml` is simpler; use STAC for EO assets |
| OGC Records | Aligned conceptually; Records is the API version |
| DCAT | `themes`, `provider`, `license` map to DCAT properties |
| ISO 19115 | Not encoded in core; preserve in `extensions` |

The native format is intentionally **minimal**. It captures what's
needed for discovery and context without replicating the full
complexity of ISO 19115 or DCAT.

---

## Related Files

- [adapters/adapter-architecture](adapter-architecture.md) — ServiceAdapter trait
- [project/data-model](../project/data-model.md) — ResourceDescriptor (what YAML maps to)
- [query/semantic](../query/semantic.md) — Markdown indexing and constraint matching
- [query/planner](../query/planner.md) — How native resources participate in queries
- [extensions/extension-points](../extensions/extension-points.md) — Metadata extensions (Layer 12)
