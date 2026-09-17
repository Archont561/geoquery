# Geoquery Knowledge Base — Update Log

## 2026-09-17
* **Update**: Migrated every concept to OKF v0.2 — added the required `type` field plus recommended `description` and `generated` provenance; normalized `status` to the OKF vocabulary (`active` → `stable`); converted `created`/`updated` to ISO 8601 UTC timestamps; normalized `refs` and retargeted links from the old uppercase `INDEX.md`.
* **Creation**: Renamed `INDEX.md` to the reserved `index.md` (frontmatter reduced to `okf_version`), restructured it to sectioned bullet listings, and added per-directory `index.md` files for progressive disclosure.
* **Creation**: Added `tools/validate_okf.py`, a zero-dependency OKF v0.2 conformance validator for this bundle.

## 2025-07-11
* **Initialization**: Created the Geoquery knowledge base — 31 concepts across 8 groups, generated from the Geoquery project specification and architecture discussions.
