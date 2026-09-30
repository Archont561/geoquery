# Infrastructure

Monorepo tooling, CI, storage tiers, and deployment targets.

* [Rust Monorepo — Workspace & Tooling](monorepo.md) - Workspace layout, Cargo.toml inheritance, tooling hierarchy.
* [Monorepo Refactor — Four Owners, One Task Graph](monorepo-refactor.md) - Staged plan for tool ownership, Turbo façades over cross-language edges, and what cannot move.
* [xtask — Task Runner & Code Generation](xtask.md) - cargo xtask setup, codegen, fixtures, schema generation.
* [CI Pipeline — GitHub Actions](ci.md) - GitHub Actions, cargo-deny, cargo-nextest, caching.
* [Storage Tiers](storage.md) - Storage tiers 0–3: stateless → SQLite → DuckDB/GeoParquet → PostGIS.
* [Deployment Targets](deployment.md) - Edge (Cloudflare/Deno), Docker, WASM, native binary.

# Navigation

* [Bundle index](../index.md) - master directory of the Geoquery knowledge base.
* [Context briefing](../CONTEXT.md) - single-file mental model of Geoquery.
