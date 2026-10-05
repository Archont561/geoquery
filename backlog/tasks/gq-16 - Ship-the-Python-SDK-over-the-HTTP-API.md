---
id: GQ-16
title: Ship the native Python SDK with maturin and PyO3
status: Done
assignee: []
created_date: '2026-09-30 20:40'
updated_date: '2026-10-05 11:06'
labels:
  - phase-3
  - python
  - sdk
  - pyo3
milestone: m-2
dependencies:
  - GQ-4
references:
  - .knowledge/interfaces/python.md
  - .knowledge/query/query-model.md
priority: medium
type: feature
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Ship `geoquery-sdk` as a platform Python wheel built with maturin and PyO3. The package must call the same Rust query language and engine as the CLI in-process; it must not model the protocol a second time in Python or depend on a running HTTP service.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `geoquery._native` is a stable PyO3 module that maturin builds from `crates/python-native`, producing both an editable install (`pixi run py-install`) and release wheels (`pixi run py-dist`).
- [x] #2 The package validates canonical query documents through Rust and maps every engine failure to `geoquery.EngineError`, which carries the Rust wording rather than a Python paraphrase of it.
- [x] #3 One versioned JSON transport carries every operation, so `invoke_raw` can return the envelope and `invoke` can raise on it; the caller chooses whether a failure is an exception instead of the boundary deciding for them.
- [x] #4 The distribution `geoquery-sdk` imports as `geoquery`, is typed inline, and declares no HTTP client and no running service as a dependency.
- [x] #5 Pytest and Rust tests cover the FFI boundary in CI, including integer precision across the transport and a refused unknown transport version.
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The native FFI boundary is built, typed and covered. `crates/python-native` exposes one PyO3 function (`invoke`) mapped to `geoquery._native`; maturin builds the editable install (`pixi run py-install`) and release wheels (`pixi run py-dist`); the Python surface is a versioned JSON transport where `invoke_raw` returns the envelope and `invoke` raises `EngineError`, with document parsing delegated to `geoquery-core`. 19 pytest tests and 3 Rust tests cover the boundary, including integer precision across the transport and a refused unknown transport version, and CI runs both through `ci-checks`.

Reconciled rather than verified unchanged. Three of the five original criteria described work that is not this task: execution through the planner (AC3), results with provenance (AC4) and the Arrow/GeoPandas conversion (AC4) all wait on GQ-10, and no amount of SDK work makes them true. Those moved to a new task that depends on GQ-10, so the dependency graph now says what the text used to imply. AC1 named a path the crate does not live at; the crate is `crates/python-native`, reached through `pyproject.toml`. The knowledge base documented the same wrong path and was corrected.

Two documentation divergences were found and fixed while reconciling. `.knowledge/interfaces/python.md` and `python/geoquery/README.md` both documented a `check_query` function that does not exist (the shipped function is `parse_document`) and a `protocol_version()` that returns a string rather than the mapping it actually returns. The README is corrected; the knowledge base is corrected with a DIVERGENCE note recording that the versioned transport is why the surface differs from the draft.
<!-- SECTION:FINAL_SUMMARY:END -->
