# Geoquery Knowledge Base — Update Log

## 2026-09-30
* **Update**: Added a "Divergence from the build" section to `infrastructure/monorepo.md` — the workspace root is a package rather than virtual, members are `crates/*` with one exclude, pixi is the task runner, resolver 3 / MSRV 1.88, no `clippy::nursery`, no `rustfmt.toml`, and tests mirror sources.
* **Creation**: Added `infrastructure/monorepo-refactor.md` — a staged proposal measuring the repository against the four-owner monorepo architecture (Cargo / Bun / Pixi / Turbo), recommending `package.json` façades for the two cross-language edges Phase 1 introduces, and recording which parts of that architecture are blocked by the `pixi-build-rust` source-directory constraint.

## 2026-09-17
* **Update**: Migrated every concept to OKF v0.2 — added the required `type` field plus recommended `description` and `generated` provenance; normalized `status` to the OKF vocabulary (`active` → `stable`); converted `created`/`updated` to ISO 8601 UTC timestamps; normalized `refs` and retargeted links from the old uppercase `INDEX.md`.
* **Creation**: Renamed `INDEX.md` to the reserved `index.md` (frontmatter reduced to `okf_version`), restructured it to sectioned bullet listings, and added per-directory `index.md` files for progressive disclosure.
* **Creation**: Added `tools/validate_okf.py`, a zero-dependency OKF v0.2 conformance validator for this bundle.

## 2025-07-11
* **Initialization**: Created the Geoquery knowledge base — 31 concepts across 8 groups, generated from the Geoquery project specification and architecture discussions.
