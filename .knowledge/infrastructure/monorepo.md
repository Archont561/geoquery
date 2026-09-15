---
id: infrastructure/monorepo
title: Rust Monorepo — Workspace & Tooling
category: infrastructure
tags: [monorepo, workspace, Cargo, tooling, dependencies, lints, resolver]
refs: [infrastructure/xtask, infrastructure/ci, project/architecture, extensions/extension-points]
status: draft
created: 2025-07-11
updated: 2025-07-11
---

# Rust Monorepo — Workspace & Tooling

## Directory Layout

```
geoquery/
├── .cargo/
│   └── config.toml             # xtask alias
├── Cargo.toml                  # Virtual workspace root
├── Cargo.lock                  # Committed (CLI binary)
├── deny.toml                   # cargo-deny config
├── rustfmt.toml                # Formatting rules
├── .github/
│   └── workflows/
│       └── ci.yml              # CI pipeline
├── xtask/                      # Task runner crate
│   ├── Cargo.toml
│   └── src/main.rs
├── crates/
│   ├── types/                  # geoquery-types
│   ├── core/                   # geoquery-core
│   ├── adapter-stac/           # geoquery-adapter-stac
│   ├── adapter-ogc/            # geoquery-adapter-ogc
│   ├── adapter-native/         # geoquery-adapter-native
│   ├── cli/                    # geoquery-cli
│   ├── tui/                    # geoquery-tui (phase 2)
│   ├── http/                   # geoquery-http
│   └── mcp/                    # geoquery-mcp
├── packages/
│   └── client/                 # @geoquery/client (TypeScript)
├── python/
│   └── geoquery/               # geoquery (Python)
├── schemas/                    # Generated JSON schemas
├── docs/
└── README.md
```

**Why `crates/` subdirectory?** Keeps the root clean as the workspace
grows. Future expansion adds `adapters/`, `servers/`, `tools/` as
peer directories.

---

## Root `Cargo.toml` (Virtual Workspace)

```toml
[workspace]
resolver = "2"
members = [
    "crates/types",
    "crates/core",
    "crates/adapter-stac",
    "crates/adapter-ogc",
    "crates/adapter-native",
    "crates/cli",
    "crates/http",
    "crates/mcp",
    "xtask",
]

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

**Key line:** `[lints] workspace = true` — every crate inherits the
root lint config. No per-crate Clippy drift.

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

- [infrastructure/xtask](xtask.md) — Task runner setup
- [infrastructure/ci](ci.md) — CI pipeline
- [project/architecture](../project/architecture.md) — Crate responsibilities
- [extensions/extension-points](../extensions/extension-points.md) — Future plugin crate layout
