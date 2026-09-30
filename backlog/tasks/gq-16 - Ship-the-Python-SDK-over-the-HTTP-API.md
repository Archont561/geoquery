---
id: GQ-16
title: Ship the Python SDK over the HTTP API
status: To Do
assignee: []
created_date: '2026-09-30 20:40'
labels:
  - phase-3
  - python
  - sdk
milestone: m-2
dependencies:
  - GQ-15
  - GQ-4
references:
  - .knowledge/interfaces/python.md
  - .knowledge/interfaces/http.md
priority: medium
type: feature
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
python/geoquery currently carries a version test and nothing else. Give Python users the documented client: a typed query builder over the HTTP transport and the dataframe conversions that make results usable in an existing geospatial workflow.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The geoquery package wraps the HTTP API behind a Geoquery client with typed models kept in step with the generated schemas rather than hand-written twice.
- [ ] #2 The query builder covers spatial, temporal, semantic, and attribute filter clauses and refuses combinations the AST does not allow.
- [ ] #3 to_geopandas and to_arrow convert results without dropping provenance or source status.
- [ ] #4 pytest runs the SDK against a mocked HTTP transport inside the default gates, with no network dependency.
<!-- AC:END -->
