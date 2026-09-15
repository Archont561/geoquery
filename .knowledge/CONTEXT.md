---
id: context
title: Geoquery — Full Project Context Briefing
category: meta
tags: [context, briefing, LLM, onboarding, summary, mental-model]
refs: [INDEX, project/overview, project/architecture, query/planner, roadmap/roadmap]
status: active
created: 2025-07-11
updated: 2025-07-11
audience: [LLMs, AI agents, new contributors, decision makers]
purpose: single-file project briefing for context loading
---

# Geoquery — Full Project Context Briefing

> **Purpose of this file:** A single, self-contained briefing that
> gives any reader (human or AI) the complete working mental model
> of Geoquery in one read. Use this when loading context into an
> LLM, onboarding a contributor, or making architectural decisions.
>
> For navigation, see [INDEX](./INDEX.md). For deep dives, follow
> the linked files.

---

## The One Sentence

**Geoquery is one protocol-independent query language and execution
engine for federating heterogeneous geospatial resources and services.**

It sits one level above STAC, OGC API, WFS, ArcGIS, CMR, and other
GIS standards — discovering their capabilities, translating a common
query into source-native queries, executing them in parallel,
normalizing the results, and returning one unified result set with
full provenance.

---

## What Problem It Solves

The geospatial ecosystem has excellent standards (STAC, OGC API,
CQL2) and mature per-source SDKs (`pystac-client`, `OWSLib`, ArcGIS
JS API). It has proliferating single-source MCP servers. It has
promising protocol-neutral SDKs like Honua.

**What it does not have:** a universal federation layer that sits
above all of them and lets a single query fan out across multiple
sources — chosen by capability, executed in parallel, deduplicated,
ranked, and returned with provenance.

Geoquery fills that gap.

---

## What Geoquery Is NOT

Understanding the negative space is as important as the positive:

- **Not a replacement** for STAC, OGC API, WFS, or any GIS standard
- **Not a catalog product** — existing catalogs remain the source of truth
- **Not a map renderer** — returns GeoJSON/assets, rendering is delegated
- **Not a download manager** — assets are exposed as metadata, not fetched
- **Not an LLM wrapper** — the LLM constructs the AST, Geoquery executes it
- **Not an agent framework** — MCP is an adapter, not the core

---

## The Four Foundational Abstractions

Everything in Geoquery revolves around four concepts:

| # | Concept | Question | Type |
|---|---------|----------|------|
| 1 | **Resource** | What exists? | `ResourceDescriptor` |
| 2 | **Service** | How can it be queried? | `ServiceDescriptor` |
| 3 | **Query** | What does the user/agent want? | `GeoQuery` |
| 4 | **Plan** | How does Geoquery obtain the answer? | `ExecutionPlan` |

Everything else — YAML, Markdown, STAC, OGC, MCP, Rust, TypeScript,
Cloudflare — is an implementation or integration detail around
these four.

---

## The Canonical Query AST (The Real API)

The API is not STAC, OGC, or ArcGIS. It is **Geoquery Query**.

```typescript
interface GeoQuery {
  // Federation concerns (Geoquery-specific, NOT CQL2)
  scope?: QueryScope
  execution?: ExecutionOptions

  // Discovery intent
  semantic?: SemanticQuery

  // Deterministic predicates
  spatial?: SpatialPredicate      // bbox, intersects, within, dwithin, ...
  temporal?: TemporalPredicate    // during, intersects, before, after, ...
  filters?: FilterExpression      // CQL2-compatible attribute filters

  // Result shaping
  sort?: SortExpression[]
  limit?: number
  fields?: string[]
  include?: IncludeOptions
}
```

**Key design principle:** federation concerns (scope, execution,
ranking) are separated from source-level filtering (spatial,
temporal, attribute). CQL2 solves the latter. It does not solve
the former.

The same AST works through every interface: Rust, CLI, HTTP, TypeScript,
Python, MCP, TUI.

---

## The Query Planner (The Actual IP)

The planner is what makes Geoquery valuable. It is the deliberate
inversion of agent-orchestrated approaches:

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

**The agent should not orchestrate GIS protocols. Geoquery should.**

An LLM might know how STAC works. It should not have to understand
40 different GIS protocols.

### The Pipeline

```
GeoQuery AST
    ↓
Discovery — find candidate resources from registry
    ↓
Capability Check — which services support the required operations?
    ↓
Partition — split query per backend capability
    ↓
Translate — GeoQuery → CQL2 / SQL / ArcGIS where / WFS FES / CMR
    ↓
Execute in parallel (tokio::JoinSet, bounded concurrency)
    ↓
Normalize — all responses → GeoResult
    ↓
Deduplicate — exact ID, canonical URL, asset identifier
    ↓
Rank — semantic × spatial × temporal × quality × freshness
    ↓
Attach provenance / context
    ↓
Unified result stream
```

### Graceful Degradation

The most interesting capability. If a source supports only `bbox`
but the user asked for `intersects` polygon:

- **Option A:** approximate (bbox only, false positives accepted)
- **Option B:** remote bbox candidates → local exact intersection

**The planner never silently pretends unsupported operations are
supported.** Degradation is always reported.

---

## Standards Position

Geoquery MUST reuse existing standards rather than inventing
replacements:

| Standard | Role |
|----------|------|
| **OGC API — Records** | Resource/catalog discovery |
| **STAC / STAC API 1.0** | Spatiotemporal asset metadata (first-class adapter) |
| **OGC API — Features** | Feature-level query foundation |
| **CQL2** | Filter compilation target (not source language) |
| **GeoJSON** | Normalized feature output |
| **DCAT / GeoDCAT** | Catalog interoperability |
| **OpenAPI** | HTTP service descriptions |
| **MCP 2026-07-28** | Agent integration |

**CQL2 is the compilation target, not the source language.** The
Geoquery filter AST compiles *to* CQL2 for compatible backends.

---

## The Interfaces

The same engine is accessed through every interface. All are thin
wrappers around `geoquery-core::execute(GeoQuery) → QueryResult`.

| Interface | Crate/Package | Purpose |
|-----------|--------------|---------|
| CLI | `geoquery-cli` | `geoquery add`, `geoquery query`, `geoquery sources` |
| HTTP | `geoquery-http` (Axum) | REST API, SSE streaming |
| MCP | `geoquery-mcp` (`rmcp` 3.x) | `geo_query`, `geo_resource`, `geo_resolve` for AI agents |
| TUI | `geoquery-tui` (`ratatui`) | Interactive terminal exploration (Phase 5) |
| TypeScript | `@geoquery/client` | Fluent builder, HTTP transport, WASM future |
| Python | `geoquery` | GeoPandas integration, PyO3 future |

**Adding a new interface never requires modifying core.**

---

## The Adapters

Adapters are the primary extension point. Every geospatial protocol
is accessed through a `ServiceAdapter` implementation:

```rust
#[async_trait]
pub trait ServiceAdapter: Send + Sync {
    fn detect(&self, endpoint: &Endpoint) -> DetectionResult;
    async fn describe(&self, endpoint: &Endpoint) -> Result<ServiceDescriptor>;
    async fn query(&self, service: &ServiceDescriptor, query: &GeoQuery) -> Result<QueryResult>;
    fn capabilities(&self, service: &ServiceDescriptor) -> CapabilitySet;
}
```

**MVP adapters:** STAC, OGC API Features, OGC API Records, Native
YAML/Markdown.

**Phase 2+:** WFS, ArcGIS REST, NASA CMR, CKAN, PostGIS, generic HTTP.

**Constraint:** Adapters depend on `geoquery-types` only, never on
`geoquery-core`. Dependency arrow is one-way.

---

## Markdown as First-Class Query Context

A resource may contain:

```
flood-risk/
├── resource.yaml       # Structured metadata
├── README.md
├── limitations.md      # Machine-readable via semantic index
├── methodology.md
└── usage.md
```

**Markdown is NOT a long description field.** It is independently
indexed and queryable.

An agent asks: *"Can I use this dataset for individual property
insurance?"*

The YAML says `type: dataset, themes: [flooding]`. That's not enough.

But `limitations.md` says: *"This dataset is unsuitable for
parcel-level insurance decisions because the native resolution is
100m."*

Now the agent has a definitive answer. **This is far more valuable
than ordinary catalog metadata.**

Structured constraints (`suitable_for` / `not_suitable_for`) provide
machine-readable domain knowledge on top of the prose. **YAML
constrains. Markdown explains.**

---

## Storage Tiers

The query protocol is identical across all tiers. Only the backend
changes.

| Tier | Name | Use Case | Dependencies |
|------|------|----------|-------------|
| **0** | Stateless | Small apps, edge, one-shot | None (in-memory) |
| **1** | Local Index | Persistent local metadata | SQLite + `rstar` + `tantivy` |
| **2** | Analytical | Local spatial analytics | DuckDB + GeoParquet + GeoArrow |
| **3** | Server | Large-scale hosted | PostgreSQL + PostGIS |

**PostGIS is never mandatory for the core engine.** Tier 0 works
with zero external dependencies.

**GeoArrow is the internal interchange format** for high-performance
federation. GeoJSON is the external output contract.

---

## The 14 Extension Points

Every layer of Geoquery is extensible through a trait with a default
implementation. Users who don't need customization never touch these:

| # | Layer | Mechanism |
|---|-------|-----------|
| 1 | Protocol adapters | `ServiceAdapter` trait |
| 2 | Resource/service types | Open enums + `Unknown(String)` |
| 3 | Storage backends | `RegistryStore` / `SpatialIndex` / `TextIndex` |
| 4 | Query operations | Open enums + `Custom(String)` |
| 5 | Capability extensions | `extensions: HashMap` |
| 6 | Auth providers | `AuthResolver` trait |
| 7 | Ranking strategies | `Ranker` trait |
| 8 | Embedding providers | `EmbeddingProvider` trait |
| 9 | Geocoders | `Geocoder` trait |
| 10 | Result transformers | `ResultTransformer` pipeline |
| 11 | Transport layers | Wrappers around `core::execute()` |
| 12 | Metadata extensions | `extensions` / `properties` maps |
| 13 | Event/hook system | `QueryEventListener` trait |
| 14 | MCP tools | Tool registry on MCP server |

**Golden rule:** Every extension point is a trait with a default
implementation. Minimal use requires zero customization.

---

## AI Safety Boundary

The trust boundary for AI-generated queries:

```
LLM
 ↓
structured GeoQuery AST
 ↓
validation (CRS, geometry, capability, place ambiguity)
 ↓
planner
 ↓
registered service only
```

This is much safer than `LLM → arbitrary SQL/API calls`. The agent
constructs the AST; Geoquery validates and executes only against
registered capabilities. No arbitrary `DROP TABLE`. No arbitrary
remote URLs.

**Validation rejects:** ambiguous CRS, malformed geometry, invalid
temporal intervals, unit-less distances, unsupported operations,
insufficient source capabilities, ambiguous place resolution.

**Structured geographic constraints must remain structured.**
Embeddings never perform spatial reasoning.

---

## The Roadmap in One Table

| Phase | Duration | Exit Demo | Critical? |
|-------|----------|-----------|-----------|
| **0. Foundation** | 3 weeks | Types compile, schemas generate | ✅ |
| **1. Single Source** | 5 weeks | CLI queries real STAC API | ✅ |
| **2. Federation** | 6 weeks | Multi-source parallel query, dedup | ✅ |
| **3. API Layer** | 6 weeks | TS + Python SDKs work | ✅ |
| **4. AI / MCP** | 6 weeks | Agent queries via MCP | ✅ |
| **5. Experience** | 8 weeks | TUI + streaming | Optional |
| **6. Scale** | 10 weeks | Edge + WASM + PyO3 | Optional |
| **7. Ecosystem** | Ongoing | Plugin marketplace | Optional |

**Critical path to value:** Phase 0 → 1 → 2 → 4 (MCP only, skip
semantic) = ~20 weeks to a working federated query engine that AI
agents can use.

---

## Technology Stack (Locked-In Decisions)

### Rust Core

| Concern | Choice |
|---------|--------|
| Geometry | `geo-types` + `geo` + `geos` (optional) + `proj` (optional) |
| Spatial index | `rstar` (R-tree) |
| Columnar interchange | `geoarrow` + `arrow` + `parquet` |
| Local analytics | `duckdb` |
| Text search | `tantivy` |
| MCP server | **`rmcp` 3.x** (MCP 2026-07-28 spec) |
| HTTP server | `axum` + `tower` + `tower-http` |
| HTTP client | `reqwest` + `tower` |
| Async runtime | `tokio` |
| CLI | `clap` (derive) |
| Serialization | `serde` + `serde_json` + `serde_yaml` |
| Time | `chrono` |
| Errors | `thiserror` (libs) + `anyhow` (apps) |
| Logging | `tracing` + `tracing-subscriber` |
| TUI (Phase 5) | `ratatui` + `crossterm` |
| Task runner | `xtask` (via `cargo xtask` alias) |

### TypeScript

| Concern | Choice |
|---------|--------|
| HTTP client | `ofetch` |
| Type generation | `ts-rs` (from Rust) |
| MCP client | **`@modelcontextprotocol/client` v2** (2026 spec) |
| Client-side geometry | `@turf/turf` (optional) |
| Testing | `vitest` |

**Architectural reference to study:** `@honua/sdk-js` — protocol-neutral
geospatial SDK with a similar contract layer (Dataset, Source,
Capabilities, Query, Result). Study its adapter boundaries.

### Python

| Concern | Choice |
|---------|--------|
| HTTP client | `httpx` (async) |
| Type validation | `pydantic` |
| Direct STAC access | `pystac-client` (optional fallback) |
| Direct OGC access | `OWSLib` (optional fallback) |
| GeoDataFrame conversion | `geopandas` (optional `[geo]` extra) |
| Analytics | `duckdb` |
| MCP | `FastMCP` or official `mcp` package |
| Future native bindings | `pyo3` + `maturin` |

---

## Monorepo Layout

```
geoquery/
├── Cargo.toml                  # Virtual workspace, resolver=2, edition=2024
├── .cargo/config.toml          # xtask alias
├── xtask/                      # Task runner + codegen
├── crates/
│   ├── types/                  # Data model, ts-rs, schemars
│   ├── core/                   # Planner, executor, traits
│   ├── adapter-stac/
│   ├── adapter-ogc/
│   ├── adapter-native/
│   ├── cli/
│   ├── http/
│   ├── mcp/
│   └── tui/                    # Phase 5
├── packages/client/            # @geoquery/client (TypeScript)
├── python/geoquery/            # geoquery (Python)
├── schemas/                    # Auto-generated JSON schemas
└── .knowledge/                 # This knowledge base
```

**Tooling stack:** Cargo workspace inheritance, `xtask`, `cargo-nextest`,
`cargo-deny`, `clippy` via `[workspace.lints]`, `Swatinem/rust-cache`
in CI, GitHub Actions.

---

## Codegen Pipeline

`cargo xtask codegen` is the source-of-truth propagation mechanism:

```
geoquery-types (Rust)
    │
    ├── ts-rs annotations   → packages/client/src/types/ (TypeScript)
    ├── schemars annotations → schemas/mcp/*.json (MCP tool schemas)
    └── serde annotations    → JSON wire format (HTTP, MCP, CLI)
```

**Zero drift between Rust, TypeScript, and MCP tool schemas.**

---

## Deployment Targets

The same architecture runs everywhere without changing the query model:

```
single binary → local machine → Docker → Node server →
edge API → managed SaaS
```

| Target | Storage Tier | Notes |
|--------|--------------|-------|
| Native binary | 0–2 | Primary developer target |
| Docker | 0–3 | Multi-stage with `cargo-chef` |
| Cloudflare Workers | 0 | WASM planner, remote federation only |
| Deno Deploy / Vercel Edge | 0 | Stateless mode |
| Node.js | 0–2 | SQLite / DuckDB via native modules |
| Browser (WASM) | 0 | IndexedDB / OPFS storage |

**Edge principle:** Do not make edge runtimes responsible for
executing arbitrary GIS queries. Edge is excellent for registry,
metadata search, query planning, caching, MCP/HTTP interface.
Heavyweight spatial execution happens in PostGIS / DuckDB / remote APIs.

---

## Competitive Landscape

| Approach | Example | Geoquery's Difference |
|----------|---------|----------------------|
| Individual source adapters | Geo-MCP servers on GitHub | Federation layer, not single-source |
| Agent-orchestrated GIS | Microsoft GeoFaham | Geoquery orchestrates protocols, agent doesn't |
| Catalog products | STAC / OGC Records catalogs | Query engine, not catalog |
| Protocol SDKs | `pystac-client`, `OWSLib` | Protocol-independent |
| Protocol-neutral SDK | `@honua/sdk-js` | Adds federation, planner, degradation, semantic |

---

## What to Read Next

Depending on what you need to do:

| Goal | Read First |
|------|-----------|
| Understand the mission | [project/overview](./project/overview.md) |
| See the component diagram | [project/architecture](./project/architecture.md) |
| Design the API contract | [query/query-model](./query/query-model.md) |
| Understand the planner | [query/planner](./query/planner.md) |
| Build an adapter | [adapters/adapter-architecture](./adapters/adapter-architecture.md) |
| Set up the monorepo | [infrastructure/monorepo](./infrastructure/monorepo.md) |
| Configure CI | [infrastructure/ci](./infrastructure/ci.md) |
| Add MCP integration | [interfaces/mcp](./interfaces/mcp.md) |
| Extend Geoquery | [extensions/extension-points](./extensions/extension-points.md) |
| See the delivery plan | [roadmap/roadmap](./roadmap/roadmap.md) |
| Look at crate choices | [research/rust-crates](./research/rust-crates.md) |

For full navigation, see [INDEX](./INDEX.md).

---

## Priming Prompt for LLMs

If you are an LLM reading this file to work on Geoquery, adopt this
frame:

> You are working on Geoquery, a Rust-first federated geospatial
> query engine. The canonical query AST is protocol-independent
> and compiles to CQL2 for source-level filters. The query planner
> is the intellectual property — it discovers sources, checks
> capabilities, partitions queries, executes in parallel, normalizes,
> deduplicates, ranks, and attaches provenance. Adapters are the
> primary extension point. Markdown context is a first-class
> queryable artifact, not documentation. MCP is an adapter, not
> the core. Rust is the canonical implementation language;
> TypeScript and Python are peer SDKs. Every interface is a thin
> wrapper around `core::execute(GeoQuery) → QueryResult`. Every
> extension point is a trait with a default. The trust boundary
> for AI is: LLM → validated AST → planner → registered services
> only. Never bypass this. When in doubt, prefer standards reuse
> over invention.

---

*This file is the single-source briefing. For any specific topic,
follow the wiki-links to the detailed file. For the full graph,
see [INDEX](./INDEX.md).*
