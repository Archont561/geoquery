---
id: GQ-16
title: Ship the native Python SDK with maturin and PyO3
status: To Do
assignee: []
created_date: '2026-09-30 20:40'
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
- [ ] #1 `python/geoquery/native` exposes a stable PyO3 module and maturin builds an editable development install plus release wheels.
- [ ] #2 The Python package validates and constructs canonical query documents through Rust, with errors mapped to useful Python exceptions.
- [ ] #3 Query execution runs through the shared in-process planner and adapter registry rather than an HTTP round trip.
- [ ] #4 Results expose provenance and an optional Arrow/GeoPandas conversion without making those ecosystems core dependencies.
- [ ] #5 Pytest and Rust tests cover the FFI boundary on every supported Python platform.
<!-- AC:END -->
