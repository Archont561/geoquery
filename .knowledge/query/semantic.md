---
type: Query Specification
title: Semantic Search & Structured Context
description: "Semantic search, embeddings, structured constraints, safety rules."
tags: [semantic, embeddings, constraints, context, markdown, AI-safety]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: query/semantic
category: query
refs: [query/query-model, query/planner, adapters/native, project/data-model, extensions/extension-points]
---

# Semantic Search & Structured Context

## Core Principle

> **Semantic search is an enhancement, not the foundation.**

`geo_query()` MUST work without an LLM and without embeddings.
Deterministic predicates (spatial, temporal, attribute) are the
foundation; semantic intent is an optional layer.

---

## The Separation Rules

### Correct

```yaml
semantic: "flood modelling datasets"
spatial:
  op: intersects
  geometry: { AOI }
temporal:
  start: 2020-01-01
  end: 2025-01-01
```

### Incorrect

```yaml
semantic: "embedding('within 50km of Warsaw')"
```

**Never rely on an embedding model to perform spatial reasoning.**
Structured geographic constraints must remain structured.

The same applies to temporal and attribute reasoning: embeddings rank
and discover; predicates constrain and verify.

---

## What Gets Indexed

The semantic index covers resource discovery metadata and contextual
documents — **not arbitrary raw GIS data**:

| Source | Indexed |
|--------|---------|
| `title` | ✅ |
| `description` | ✅ |
| `keywords`, `themes` | ✅ |
| `provider` | ✅ |
| Documentation (`README.md`) | ✅ |
| `usage` notes | ✅ |
| `limitations` | ✅ |
| `methodology` | ✅ |
| Relationships | ✅ |
| Raw geospatial data (rasters, features) | ❌ never by default |

The initial semantic index focuses on **resource discovery and
contextual metadata**.

---

## Markdown as Query Context

Markdown is first-class. A resource may contain:

```
flood-risk/
├── resource.yaml
├── README.md
├── limitations.md
├── methodology.md
└── usage.md
```

**Markdown MUST NOT be treated merely as a long description field.**
It is independently indexed and queryable.

### Example: limitations.md

```markdown
# Usage

This dataset is suitable for regional flood modelling.

It should not be used for parcel-level insurance decisions
because the native resolution is 100m.

For historical comparison use the 2010–2020 archive.
```

An agent asks: *"Can I use this dataset for individual property
insurance?"* — Geoquery retrieves this context via `geo_resource`.
This is far more valuable than ordinary catalog metadata.

→ See [adapters/native](../adapters/native.md) for the resource manifest format

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
  temporal:
    historical_only: false
```

**YAML constrains. Markdown explains.**

---

## Planner Integration

Agent asks: *"Find datasets around Warsaw suitable for urban flood
modelling."*

Geoquery doesn't merely search `title ~= "flood"`. It executes:

```
semantic search
     ↓
spatial filter
     ↓
resource type filter
     ↓
usage constraints          ← structured constraints applied
     ↓
service capability check
     ↓
actual query
```

A dataset explicitly declaring `not_suitable_for: urban flood
modelling` can be **demoted or excluded**. This is a genuinely
AI-native capability that ordinary catalogs lack.

→ See [query/planner](planner.md) for the full pipeline

---

## Hybrid Ranking

Initial ranking combines:

| Signal | Source |
|--------|--------|
| Semantic relevance | Embedding similarity (optional) |
| Spatial relevance | Overlap ratio / distance |
| Temporal relevance | Interval overlap |
| Resource quality | Provider reputation, completeness |
| Source confidence | Adapter reliability |
| Exact metadata matches | Keyword/theme hits |
| Provider preference | User config |
| Freshness | Data recency |

**Keep ranking modular. Do not hard-code a single embedding provider.**

---

## Embedding Providers (Extension Point)

```rust
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    fn dimensions(&self) -> usize;
}
```

Built-ins:
- `LocalEmbeddingProvider` — ONNX Runtime, `all-MiniLM-L6-v2` (no
  network dependency)
- `OpenAiEmbeddingProvider` — `text-embedding-3-small`
- `None` — semantic search disabled (Tier 0 default)

→ See [extensions/extension-points](../extensions/extension-points.md) Layer 8

Full-text search (`tantivy`) covers most discovery needs even without
embeddings — hybrid BM25 + vector is the sweet spot.

---

## AI Safety Rules for Semantic Queries

| Rule | Enforcement |
|------|-------------|
| Inferred ≠ authoritative | Inferred spatial facts never shown as source data |
| Ambiguity rejection | Multiple plausible place candidates → clarify |
| Validation gate | All AI-generated queries pass [[query/query-model|AST validation]] |
| Constraint honesty | `not_suitable_for` respected, not silently ignored |
| Context boundaries | Agent receives context on request, not everything (prevents context-window explosion) |

The MCP pattern:

```
geo_query    → small useful results
    ↓
geo_resource → deep context on demand
```

→ See [interfaces/mcp](../interfaces/mcp.md)

---

## Future Structured Context

```yaml
usage:
  suitable_for: [...]
  not_suitable_for: [...]
  recommended_with:
    - dem
    - river-network
  caveats:
    - resolution is 100m
```

The agent should receive this context when requested. `query_examples`
can also be declared per-resource and supplied to agents as guidance
(not executable permissions).

---

## Related Files

- [query/query-model](query-model.md) — Semantic's place in the AST
- [query/planner](planner.md) — Constraint-aware planning
- [adapters/native](../adapters/native.md) — Markdown context sidecar format
- [extensions/extension-points](../extensions/extension-points.md) — EmbeddingProvider trait
- [interfaces/mcp](../interfaces/mcp.md) — Context delivery to agents
- [infrastructure/storage](../infrastructure/storage.md) — Tantivy + vector index tiers
