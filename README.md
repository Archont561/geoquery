# geoquery

<p align="center">
  <a href="https://github.com/Archont561/geoquery/actions/workflows/ci.yml"><img src="https://github.com/Archont561/geoquery/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://codecov.io/gh/Archont561/geoquery"><img src="https://codecov.io/gh/Archont561/geoquery/branch/main/graph/badge.svg" alt="Coverage"></a>
  <a href="https://github.com/Archont561/geoquery/actions/workflows/docs.yml"><img src="https://github.com/Archont561/geoquery/actions/workflows/docs.yml/badge.svg" alt="Docs"></a>
  <a href="https://github.com/Archont561/geoquery/releases"><img src="https://img.shields.io/github/v/release/Archont561/geoquery?label=release" alt="Release"></a>
  <a href="https://prefix.dev/channels/archont561/archont561"><img src="https://img.shields.io/badge/prefix.dev-archont561%2Farchont561-5c4ee5" alt="prefix.dev channel"></a>
  <a href="https://github.com/Archont561/geoquery/blob/main/LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue" alt="MIT OR Apache-2.0"></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.98-orange?logo=rust" alt="Rust 1.98"></a>
  <a href="https://pixi.sh"><img src="https://img.shields.io/badge/Pixi-0.81%2B-yellow?logo=condaforge" alt="Pixi 0.81+"></a>
  <img src="https://img.shields.io/badge/platform-linux--64-brightgreen" alt="linux-64">
  <a href="https://github.com/Archont561/geoquery/pulls"><img src="https://img.shields.io/badge/PRs-welcome-brightgreen" alt="PRs welcome"></a>
</p>

<p align="center">
  <strong>One query language and execution engine for federating geospatial resources and services.</strong><br>
  Discover capabilities, translate one query into source-native requests, and return one result set with provenance.
</p>

---

> [!IMPORTANT]
> **Phase 1 is in progress.** The query language itself is real and tested: the canonical
> AST, CQL2-compatible filter expressions, the resource/service/result data model, and the
> adapter, capability and execution contracts. The federation engine behind them is not
> built, and no adapter contacts a live service yet — `geoquery query` exits with code `3`
> rather than pretending otherwise.

Geoquery sits above STAC, OGC API, WFS, ArcGIS, CMR, and similar protocols. Applications
express intent once; adapters negotiate source capabilities and the planner decides what can
be pushed down, what must run locally, and how results retain their provenance.

## 🧠 Philosophy, model, and user-facing API

### GraphQL-like, not GraphQL

Geoquery is "GraphQL for geospatial services" only as an architectural analogy. Like
GraphQL, it gives applications one declarative request shape instead of one hand-written
client per backend. Unlike GraphQL, it is not a GraphQL schema, resolver runtime, or HTTP
endpoint convention. Its source language is the `GeoQuery` document; STAC `/search`, OGC
API parameters, ArcGIS `where`, CMR query parameters, SQL and CQL2 are compilation targets.

The philosophy is:

- **Intent over endpoint syntax:** callers say *what geospatial resources they want*, not
  which protocol parameter each service expects.
- **Standards reuse over reinvention:** GeoJSON, OGC API, STAC and CQL2 remain the native
  vocabulary wherever they already solve the problem.
- **Capability-aware execution:** every adapter reports what a source can push down,
  approximate, run locally or refuse before the planner decides how to ask it.
- **Provenance first:** normalized results keep the source, request and degradation facts
  needed to audit a federated answer.
- **One core, many surfaces:** CLI, Python, TypeScript, future HTTP, MCP and TUI surfaces
  all speak the same query model instead of translating between independent APIs.
- **Honest phases:** until execution exists, user-facing commands parse query documents
  and then decline to contact services rather than pretending to return results.

### The model

The public model is a JSON-serializable AST. Federation concerns stay separate from
source-level predicates so the planner can reason about both:

```typescript
interface GeoQuery {
  // Federation concerns: which sources participate, and how hard to try.
  scope?: QueryScope;
  execution?: ExecutionOptions;

  // Discovery intent and deterministic predicates.
  semantic?: string;
  spatial?: SpatialPredicate;
  temporal?: TemporalPredicate;
  filters?: FilterExpression;

  // Result shaping.
  sort?: SortExpression[];
  limit?: number;
  offset?: number;
  fields?: string[];
  include?: IncludeOptions;
}
```

A representative query document looks like this:

```json
{
  "scope": { "providers": ["NASA", "Copernicus"] },
  "semantic": "cloud-free flood imagery",
  "spatial": { "op": "bbox", "bbox": [14.1, 49.0, 24.2, 54.8] },
  "temporal": {
    "op": "intersects",
    "start": "2024-05-01T00:00:00Z",
    "end": "2024-09-30T23:59:59Z"
  },
  "filters": {
    "and": [
      { "field": "cloud_cover", "op": "<", "value": 10 },
      { "field": "platform", "op": "=", "value": "sentinel-2" }
    ]
  },
  "sort": [{ "field": "datetime", "direction": "desc" }],
  "limit": 20,
  "include": { "assets": true }
}
```

Service descriptors advertise type, URL, auth metadata, queryable fields and capabilities.
Adapters translate the query AST into source-native requests and return `GeoResult` records.
Each result requires provenance; an unattributed row is intentionally unrepresentable.
Execution outcomes keep per-source status as data, so a caller can distinguish an empty
successful search from a query that asked no source or degraded part of the work locally.

### User-facing API

The user-facing API is the query document plus thin bindings around the shared Rust core.
Today those bindings expose document parsing and protocol introspection; execution will land
behind the same boundary.

| Surface | API today | Role |
| --- | --- | --- |
| CLI | `geoquery check <query.json>` | Parse a query document offline and report its top-level keys. |
| CLI | `geoquery query --service <url> --query <query.json>` | Parse first, then exit `3` because the execution engine is not available yet. |
| Python | `geoquery.parse_document(text)`, `protocol_version()`, `ping()`, `invoke()` | PyO3 binding over the versioned JSON transport; no HTTP service is involved. |
| TypeScript | `parseDocument(text)`, `protocolVersion()`, `ping()`, `invoke()` | N-API binding over the same transport; currently linux-64 native package. |
| Rust | `geoquery-types`, `geoquery-core`, `geoquery-protocol` | Canonical AST, validation, adapter contracts and transport envelope. |

As the engine lands, the same document becomes the input to higher-level calls such as
`geo.query({...})`, `engine.execute(query)`, `POST /query` and MCP `geo_query` tool
arguments. Those surfaces should differ in transport and ergonomics, not in what a query
means.

### Product design direction

Geoquery should grow from a CLI-first slice. The first product should be the `geoquery`
binary, built on `geoquery-core`, packaged as `geoquery`, and published to the public
`archont561/archont561` prefix.dev channel. That keeps the first interface scriptable,
testable in CI, and useful before the SDKs and service surfaces grow.

The workflow should make heterogeneous services legible before it makes them fast:

```text
install geoquery → add/describe sources → author GeoQuery → run federated query
                     → inspect provenance and source status
                     → explain/snapshot/cache only when the workflow needs it
```

The inspirations to borrow from are STAC's asset/catalog model, QGIS's source browser,
Trino's `EXPLAIN` and predicate pushdown, Terraform-style drift checks, and DuckDB plus
GeoParquet for local analytical caches. The design to avoid is an opaque map portal that
returns rows without saying which source answered, what was pushed down, or what degraded.

The durable design note is in [`.knowledge/project/product-design.md`](.knowledge/project/product-design.md);
the published docs summarize it in [Design direction](apps/docs/src/content/docs/design.md).

## 🧭 Architecture

```text
                         one Geoquery document
                                  │
                 ┌────────────────┼────────────────┐
                 │                │                │
           Rust CLI/TUI      Python SDK      TypeScript SDK
                 │                │                │
                 └────────────────┼────────────────┘
                                  ▼
                       planner + execution engine
                                  │
             ┌────────────┬───────┴───────┬────────────┐
             ▼            ▼               ▼            ▼
           STAC        OGC APIs         ArcGIS        CMR …
                                  │
                                  ▼
                    normalized results + provenance
```

| Surface | Package | Role |
| --- | --- | --- |
| CLI | `geoquery` | Canonical Rust command line; future TUI ships as `geoquery tui` |
| Python | `geoquery-sdk` | Native PyO3/maturin Python SDK; imports as `geoquery` |
| TypeScript | `@archont561/geoquery` | N-API/FFI package that wraps the Rust core in-process |
| Language | `geoquery-types` | The query AST, filters and data model every other surface is generated from |
| Core | `geoquery-core` | Protocol-independent adapter, capability and execution contracts |
| Transport | `geoquery-protocol` | The wire contract the SDKs cross, and the number domain it guarantees |

The Python and TypeScript SDKs do not shell out to the CLI or call an HTTP service: they
load native Rust bindings around the same core. Shared schemas and conformance fixtures
keep the language surfaces compatible.

## 📦 Installation

Tagged releases are published to the public
[`archont561/archont561`](https://prefix.dev/channels/archont561/archont561) channel.
The channel currently targets `linux-64`.

```bash
pixi global install \
  --channel https://prefix.dev/archont561/archont561 \
  --channel conda-forge \
  geoquery
```

Or add Geoquery to a Pixi project:

```toml
[workspace]
channels = [
  "https://prefix.dev/archont561/archont561",
  "conda-forge",
]

[dependencies]
geoquery = ">=0.1,<0.2"
# geoquery-sdk = ">=0.1,<0.2" # Python SDK; imports as geoquery
```

> [!NOTE]
> Until the first tagged release appears in the channel, build packages from a checkout with
> `pixi run publish-dist`. GitHub Releases retain the exact `.conda` files and checksums built
> from the tag. If prefix.dev is temporarily unavailable, the GitHub Release still ships those
> verified assets and the channel upload can be retried from that tagged commit.

## ⚡ CLI quick start

```bash
geoquery --version

echo '{"limit": 25, "execution": {"maxSources": 4}}' > query.json
geoquery check query.json
```

Expected output:

```text
query.json is a query object with 2 keys: execution, limit
```

The execution command is intentionally honest about the current phase:

```bash
geoquery query --service https://example.org --query query.json
echo $?
```

```text
geoquery: no engine yet — https://example.org was not contacted. geoquery 0.1.0 reads
query documents; executing them arrives with the query engine.
3
```

| Exit code | Meaning |
| ---: | --- |
| `0` | Success |
| `1` | Input could not be read |
| `2` | Input is not a query document |
| `3` | Execution engine is not available yet |

## ✨ Design goals

| Goal | What it means |
| --- | --- |
| **Protocol-independent** | User queries describe intent rather than STAC, WFS, or vendor request syntax |
| **Capability-aware** | Adapters discover what each source supports before planning requests |
| **Federated** | One plan can query heterogeneous services concurrently |
| **Provenance-preserving** | Every normalized result retains where and how it was obtained |
| **Pushdown-first** | Spatial, temporal, and semantic filters run at the source whenever possible |
| **Multi-interface** | CLI, TUI, HTTP, MCP, Python, and TypeScript share one engine and query model |
| **Reproducible** | Pixi locks the Rust, Python, JavaScript, and repository toolchains together |
| **Offline-capable** | A verified sandbox branch restores the complete development environment without network access |

## 🛠️ Development

[Pixi](https://pixi.sh) is the only repository entry point. Package-specific commands live
beside their packages and [Turbo](https://turbo.build) fans them out across Rust, Python, and
TypeScript.

```bash
pixi install       # restore the locked toolchain
pixi run setup     # install Bun workspace, editable Python SDK, and Git hooks
pixi run gates     # the commit gate: everything that must pass before a commit lands
pixi run ci-checks # what the CI checks job runs: gates plus instrumented coverage
pixi run fmt       # format every package
pixi run ci        # ci-checks plus all publishable artifacts
```

CI calls these same task definitions rather than restating the steps, so the only way a
local run and a CI run can disagree is if pixi resolved a different environment — which
`pixi install --locked` exists to prevent.

Useful tasks:

| Task | Purpose |
| --- | --- |
| `build` / `test` | Build or test every package through Turbo |
| `lint` / `typecheck` / `fmt` | Run Clippy, cargo-deny, Ruff, Biome, and language type checkers |
| `cov` | Produce Rust, Python, and TypeScript coverage |
| `gates` | Required tests, lint, version and layout checks, workflow lint, and publish-plan validation |
| `ci-checks` | Exactly what the CI `checks` job runs: `gates` plus instrumented coverage |
| `docs-dev` / `docs-build` | Serve or build the Astro + Starlight documentation |
| `version-check` | Assert every published manifest carries the workspace version |
| `layout-check` | Assert the workspace layout matches `.knowledge/infrastructure/monorepo.md` |
| `version-set X.Y.Z` | Update every manifest that owns a literal release version |
| `publish-plan` | Resolve the package set without uploading anything |
| `publish-dist` | Build `dist/*.conda` locally |

Scope a Turbo-backed task to one package by forwarding a filter:

```bash
pixi run test -- --filter=@geoquery/rust
```

## 🗂️ Repository map

| Path | Purpose |
| --- | --- |
| `crates/types/` | The query AST, CQL2 filters, and the resource/service/result data model |
| `crates/core/` | Query documents, versions, and the adapter/capability/execution contracts |
| `crates/protocol/` | The FFI wire contract shared by the Python and TypeScript bindings |
| `crates/engine/` | Request dispatch across the protocol boundary |
| `crates/adapter-{native,ogc,stac}/` | Execution adapters (scaffolded; Phase 1–2) |
| `crates/http/`, `crates/mcp/`, `crates/tui/` | User-facing Rust interfaces (scaffolded) |
| `crates/{node,python}-native/` | N-API and PyO3 bindings over `geoquery-engine` |
| `crates/cli/src/main.rs` | Canonical `geoquery` executable; its manifest is the root `Cargo.toml` |
| `crates/xtask/` | Code generation only — not the task runner |
| `python/geoquery/` | Python SDK and `geoquery-sdk` Conda package |
| `packages/geoquery/` | `@archont561/geoquery` TypeScript FFI package |
| `packages/utils/` | Shared internal TypeScript helpers |
| `apps/docs/` | Astro + Starlight documentation |
| `pixi.toml` | Environment, task graph, and `geoquery` CLI package |
| `scripts/version.ts` | Shared-version reader, validator, and updater |
| `scripts/layout.ts` | Asserts the workspace matches the documented layout |
| `backlog/` | Tasks, milestones, and the phase plan |
| `.pixi-sandbox.toml` | Reviewed offline-environment publication plan |
| `.knowledge/` | Architecture, query model, interfaces, decisions, and roadmap |

The root `Cargo.toml` is both a Cargo workspace and the CLI package. The root `pixi.toml` is
both a Pixi workspace and the publishable `geoquery` Conda package. This lets
`pixi-build-rust` install the root binary with `cargo install --locked --path .` while all
library crates remain under `crates/`.

## 🚢 Releases

The release process has two deliberately separate workflows:

1. **Prepare release** is manually dispatched. Convco derives the next SemVer from
   conventional commits, regenerates `CHANGELOG.md`, updates every manifest, commits to
   `main`, and creates an annotated `vX.Y.Z` tag.
2. **Release** is triggered by that tag. It reruns the gates and builds immutable Conda,
   Python, and TypeScript artifacts. It then attempts each external distribution independently:
   prefix.dev through GitHub OIDC with attestations, PyPI through Trusted Publishing, npmjs
   through npm Trusted Publishing, GitHub Packages for the TypeScript package, and crates.io
   for the public Rust core and CLI. The GitHub Release is required and is created last with
   every built artifact and a SHA-256 manifest, even if an optional registry upload failed.
   The Actions summary records every external publication result so a failed upload can be
   retried from the immutable tag.

Use [Conventional Commits](https://www.conventionalcommits.org/) because commit history is the
release input:

```text
fix: correct temporal interval normalization      # patch
feat: add STAC item search adapter                 # minor
feat!: replace the v1 query envelope               # major
```

No long-lived prefix.dev, PyPI, or npm publishing token is stored in GitHub. Repository
Access grants `release.yml` read/write access only to `archont561/archont561`; PyPI and npmjs
trust that workflow's GitHub OIDC identity. The protected `release` environment needs only a
`CRATES_IO_TOKEN` environment secret for the crates.io upload. GitHub Packages uses the
workflow's short-lived `GITHUB_TOKEN` with `packages: write`.

## 🔒 Offline development

The complete locked environment is published to an orphan Git branch by
`publish-sandbox.yml`. After transferring Git history into an airlock:

```bash
sh scripts/restore.sh
source .pixi/sandbox-env.sh
pixi install --frozen --offline   # must be a no-op
cargo build --offline             # uses the restored vendor tree
```

The restore launcher reads `.pixi-sandbox.toml`, selects the bundle for the host platform,
verifies every declared byte, relocates the Pixi environment, and configures Cargo to use
vendored dependencies.

> [!TIP]
> Override automatic bundle selection with `PIXI_SANDBOX_BRANCH` or
> `PIXI_SANDBOX_BUNDLE` when testing a specific transport.

## 🗺️ Roadmap

| Phase | Outcome | Status |
| ---: | --- | --- |
| 0 | Environment, repository structure, document reader, publishable packages | ✅ Done |
| 1 | Query AST, filters, CQL2, adapter contracts, code generation, single-source STAC | 🚧 In progress |
| 2 | Federated multi-source queries, planner, and result merging | Planned |
| 3 | HTTP service and SDK integration | Planned |
| 4 | MCP tools and semantic discovery for agents | Planned |
| 5 | Interactive TUI | Planned |
| 6–7 | Scale, deployment targets, and the extension platform | Planned |

Phase 1 so far: the foundation types, the canonical AST, CQL2-compatible filters, and the
adapter, capability and execution contracts are implemented and tested. Still open are
code generation, the local source registry, and the first STAC adapter that makes those
contracts do real work.

See [`backlog/milestones/`](backlog/milestones/) for the full plan and
[`.knowledge/CONTEXT.md`](.knowledge/CONTEXT.md) for a compact project briefing.

## 🤝 Contributing

Issues and pull requests are welcome. Run the same gates used by CI before opening a PR:

```bash
pixi run gates
```

Commit messages are checked by Lefthook and must follow the Conventional Commits format.
Please keep changes focused and include tests beside the source they cover.

## 📄 License

Licensed under either of the following, at your option:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)
