---
type: Infrastructure Specification
title: "Rust Monorepo — Workspace & Tooling"
description: "Workspace layout, Cargo.toml inheritance, tooling hierarchy."
tags: [monorepo, workspace, Cargo, tooling, dependencies, lints, resolver]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-30T00:00:00Z
id: infrastructure/monorepo
category: infrastructure
refs: [infrastructure/monorepo-refactor, infrastructure/xtask, infrastructure/ci, project/architecture, extensions/extension-points]
---

# Rust Monorepo — Workspace & Tooling

## Divergence from the build

This document is the design. The repository diverges from it in ways that are decisions
rather than drift, recorded here because a reader who follows the layout below would
break the build:

- **The workspace root is not virtual.** The root `Cargo.toml` is both the `[workspace]`
  and the `geoquery` `[package]`, and `crates/cli/` has no manifest of its own:
  `pixi-build-rust` runs `cargo install --path <source>`, which cannot select a member out
  of a virtual manifest, so the directory a Conda package is built from has to be a package
  that is also the root of its workspace. It is also what lets the offline sandbox vendor
  the whole dependency graph. The reasoning is in the comment at the top of `Cargo.toml`;
  `../../backlog/docs/plans/monorepo-refactor.md` records what it would take to undo.
- **Members are `crates/*` with `exclude = ["crates/cli"]`**, not a hand-written list —
  one edit to add a crate, and the one directory under `crates/` that is not a crate is the
  one named above.
- **`xtask` is `crates/xtask/`, not a root directory, and it is not the task runner.**
  Pixi is: every command a hook or a workflow runs is `pixi run <task>`, and there is no
  `.cargo/config.toml` alias in the repository (the one that exists on a restored machine
  is written by the offline sandbox and is ignored). `xtask` keeps exactly one job — code
  generation, which needs the workspace's own types in memory.
- **Resolver 3, edition 2024, MSRV 1.88**, not resolver 2. Resolver 3 uses `rust-version`
  during version selection, which is what makes the MSRV a fact about the graph rather than
  a claim in a manifest.
- **One member restates `[workspace.lints]` instead of inheriting it.**
  `crates/node-native` is an N-API addon, and `#[napi]` expands to a module constructor
  carrying its own `#[allow(unsafe_code)]`. `forbid` is the one level a macro cannot
  override, so the addon does not compile under the workspace policy, and cargo gives a
  member no way to override a single inherited lint. The crate therefore copies the
  tables and softens exactly one key, `unsafe_code = "deny"`. `pixi run layout-check`
  compares the copy against `[workspace.lints]` on every run, so "must move with it" is
  a check rather than a comment. Every other member, including the published
  `geoquery`, inherits and stays on `forbid`.
- **`clippy::nursery` is not enabled.** Nursery lints are unstable, so a patch bump of the
  pinned toolchain would turn into a lint failure; `pedantic` is gated instead.
- **There is no `rustfmt.toml`.** Defaults, so there is no second formatting policy to keep
  in step with Biome's and Ruff's hundred columns.
- **`schemas/` does not exist yet** (it arrives with codegen in Phase 1), and the docs site
  is `apps/docs/`, not `docs/`.
- **Tests mirror sources.** Every crate has `tests/`, one file per file in `src/`, and the
  tests are integration tests against the public surface — `crates/cli/tests/main.rs` runs
  the built binary and asserts stdout, stderr and the exit code. `pixi run layout-check`
  fails on a source file with no test beside it, and on a test file with no source.

## Directory Layout

The tree below is the repository as it is, not as the design above imagined it; the
divergence section explains every place the two differ and why. `pixi run layout-check`
asserts the structural claims — the `crates/*` glob, the `crates/cli` exclusion, the
inherited lints and metadata, and the test mirror — against `cargo metadata` rather than
against a reading of this file, so the two cannot drift apart quietly.

```
geoquery/
├── AGENTS.md                   # Contributor rules; pixi is the only entry point
├── Cargo.toml                  # Workspace root *and* the geoquery package
├── Cargo.lock                  # Committed (the workspace ships a binary)
├── deny.toml                   # cargo-deny config, read from the workspace root
├── pixi.toml                   # Owns the toolchain, environments, tasks and the version
├── pixi.lock
├── package.json / bun.lock     # Root Bun workspace
├── turbo.json                  # Task graph across the three languages
├── biome.json                  # Formatter and linter for JS/TS/JSON
├── tsconfig.base.json
├── lefthook.yml                # Git hooks, all of which call `pixi run`
├── .github/
│   └── workflows/              # ci.yml, release.yml
├── crates/
│   ├── types/                  # geoquery-types — the data model everything shares
│   ├── core/                   # geoquery-core
│   ├── protocol/               # geoquery-protocol — the FFI wire contract
│   ├── engine/                 # geoquery-engine
│   ├── adapter-stac/           # geoquery-adapter-stac
│   ├── adapter-ogc/            # geoquery-adapter-ogc
│   ├── adapter-native/         # geoquery-adapter-native
│   ├── cli/                    # geoquery — sources only; its manifest is the root
│   ├── tui/                    # geoquery-tui
│   ├── http/                   # geoquery-http
│   ├── mcp/                    # geoquery-mcp
│   ├── node-native/            # N-API addon for the TypeScript package
│   ├── python-native/          # PyO3 extension for the Python SDK
│   ├── xtask/                  # geoquery-xtask — code generation only, not a task runner
│   ├── package.json            # Turbo façade: `lint`, `test`, `cov` for the whole workspace
│   └── turbo.json
├── packages/
│   ├── geoquery/               # @archont561/geoquery — the TypeScript client
│   └── utils/                  # @geoquery/utils
├── python/
│   └── geoquery/               # geoquery — the Python SDK
├── apps/
│   └── docs/                   # @geoquery/docs — the documentation site
├── scripts/                    # version.ts, layout.ts, restore.sh
├── backlog/                    # Tasks, milestones and plans
└── .knowledge/                 # This corpus
```

Every directory under `crates/` is a workspace member except `cli`, which holds the
binary's sources while the root `Cargo.toml` holds its manifest. There is no `schemas/`
yet — it arrives with codegen — and no `.cargo/config.toml` or `rustfmt.toml` in the
repository at all.

**Why `crates/` subdirectory?** Keeps the root clean as the workspace grows. Future
expansion adds peer directories rather than more top-level crates.

---

## Root `Cargo.toml`

```toml
# The real one. `members` is a glob and not a list, so adding a crate is creating a
# directory; `exclude` names the one directory under `crates/` that is sources without a
# manifest. Edition 2024 implies resolver 3, which is what makes `rust-version` a fact
# about the resolved graph rather than a claim in a manifest.
[workspace]
members = ["crates/*"]
exclude = ["crates/cli"]

# ── Shared package metadata ──────────────────────────
[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
license = "MIT OR Apache-2.0"
repository = "https://github.com/yourorg/geoquery"

# ── Shared dependencies (single source of truth) ─────
[workspace.dependencies]
# Internal crates
geoquery-types = { path = "crates/types" }
geoquery-core = { path = "crates/core" }
geoquery-adapter-stac = { path = "crates/adapter-stac" }

# Geospatial
geo-types = "0.7"
geo = "0.29"
geojson = "0.24"
proj = "0.28"
rstar = "0.12"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"

# Async / HTTP
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }

# Time
chrono = { version = "0.4", features = ["serde"] }

# Error handling
thiserror = "2"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# CLI
clap = { version = "4", features = ["derive"] }

# Testing
pretty_assertions = "1"
wiremock = "0.6"

# ── Workspace-wide lint configuration ────────────────
[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"
unreachable_pub = "warn"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
missing_errors_doc = "allow"
missing_panics_doc = "allow"
module_name_repetitions = "allow"
```

---

## Example Crate `Cargo.toml`

```toml
[package]
name = "geoquery-core"
description = "Geoquery query AST, planner, and execution engine"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
geoquery-types.workspace = true
geo-types.workspace = true
geo.workspace = true
serde.workspace = true
serde_json.workspace = true
chrono.workspace = true
thiserror.workspace = true
tracing.workspace = true
tokio.workspace = true
reqwest.workspace = true

[dev-dependencies]
pretty_assertions.workspace = true
wiremock.workspace = true

[lints]
workspace = true
```

**Key line:** `[lints] workspace = true` — every crate inherits the root lint config, so
there is no per-crate Clippy drift. The single exception is `crates/node-native`, for the
reason given in the divergence section, and the exception is itself checked: it is allowed
to soften `unsafe_code` and nothing else.

---

## Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Edition** | `2024` | Stable in Rust 1.85+. Latest resolver, `gen` keyword reservation, improved lifetime elision. Drop to `2021` if older toolchain needed. |
| **Resolver** | `2` (explicit) | Default for 2021+ but explicit prevents surprises. |
| **Cargo.lock** | Committed | Workspace ships a CLI binary; lockfile ensures reproducible builds. |
| **Naming** | `geoquery-*` | Clear namespace. Change before crates.io publication if needed. |
| **Publishing** | Private initially | Individual crates published to crates.io in Phase 7. |
| **Async runtime** | `tokio` | De facto standard, required by `axum`, `reqwest`, `rmcp`. |
| **WASM constraints** | Not yet | No `no_std` or WASM-safe restrictions in core yet. Phase 6. |

---

## Tooling Hierarchy

Ordered by when to add:

| Layer | Tool | Purpose | When |
|-------|------|---------|------|
| **Structure** | Cargo workspace + inheritance | Native project structure | Now |
| **Formatting** | `rustfmt` | Code formatting | Now |
| **Linting** | Clippy via `[workspace.lints]` | Single config, no drift | Now |
| **Testing** | `cargo-nextest` | 3–5× faster parallel tests | Now |
| **Auditing** | `cargo-deny` | Licenses, advisories, bans | Now |
| **Tasks** | `cargo xtask` | Type-safe task runner | Now |
| **CI caching** | `Swatinem/rust-cache` | CI build speedup | Now |
| **Feature unification** | `cargo-hakari` | Prevent exponential feature combos | >10 crates |
| **Supply chain** | `cargo-vet` | Third-party trust verification | Before public release |
| **Release** | `cargo-release` | Coordinated version bumps | When publishing |
| **Build cache** | `sccache` | Local + CI compilation cache | When builds slow |
| **Docker** | `cargo-chef` | Dependency-layer caching | When containerizing |

**Principle:** Cargo's native workspace inheritance should be the
first choice. Tools like `cargo-hakari` solve a different problem
(feature unification across large workspaces via a `workspace-hack`
crate). Don't add tools that duplicate Cargo functionality.

---

## `rustfmt.toml`

```toml
edition = "2024"
max_width = 100
use_field_init_shorthand = true
use_try_shorthand = true
```

---

## Related Files

- [infrastructure/monorepo-refactor](../../backlog/docs/plans/monorepo-refactor.md) — What the build does instead, and the staged plan
- [infrastructure/xtask](xtask.md) — Task runner setup
- [infrastructure/ci](ci.md) — CI pipeline
- [project/architecture](../project/architecture.md) — Crate responsibilities
- [extensions/extension-points](../extensions/extension-points.md) — Future plugin crate layout
