---
id: GQ-2
title: Define the canonical GeoQuery AST
status: To Do
assignee: []
created_date: '2026-09-30 14:35'
labels:
  - phase-1
  - types
  - query
milestone: m-0
dependencies:
  - GQ-1
references:
  - .knowledge/query/query-model.md
  - .knowledge/query/spatial.md
  - .knowledge/query/temporal.md
priority: high
type: task
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Geoquery must express user intent once and compile it for each backend. Replace the Phase 0 untyped query document with a protocol-independent AST that can be shared by the CLI, planner, adapters, and future SDKs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 GeoQuery models scope, execution options, spatial predicates, temporal predicates, result shaping, and semantic intent using typed public fields.
- [ ] #2 Spatial and temporal values have explicit CRS and interval semantics, including valid open-ended intervals where the design allows them.
- [ ] #3 Valid representative query documents deserialize and serialize deterministically.
- [ ] #4 Invalid query shapes and invalid intervals produce actionable validation errors rather than reaching an adapter.
<!-- AC:END -->
