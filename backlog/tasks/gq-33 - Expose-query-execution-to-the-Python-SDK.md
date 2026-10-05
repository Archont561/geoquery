---
id: GQ-33
title: Expose query execution to the Python SDK
status: To Do
assignee: []
created_date: '2026-10-05 11:05'
labels:
  - phase-3
  - python
  - sdk
  - pyo3
dependencies:
  - GQ-10
priority: medium
type: feature
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
GQ-16 shipped the native FFI boundary: the PyO3 module, the versioned transport and the document parser. This is the execution surface on top of it. It waits on GQ-10, because an SDK that exposes execution has nothing to execute until the planner does.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The Python package executes a query in-process through the shared planner and adapter registry, with no HTTP round trip and no second implementation of the query language.
- [ ] #2 Results reach Python carrying provenance: the source that answered, the capability path used, and any degradation the planner reported.
- [ ] #3 A documented conversion to GeoDataFrame or Arrow exists and degrades to an explicit, actionable error when the optional dependency is absent, rather than failing at import.
- [ ] #4 Neither geopandas nor pyarrow is a required runtime dependency; the wheel installs and every non-conversion test passes without either.
- [ ] #5 Tests cover execution over the FFI boundary, including an unsupported-operation degradation that is reported rather than silently approximated.
<!-- AC:END -->
