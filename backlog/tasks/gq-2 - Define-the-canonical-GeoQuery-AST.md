---
id: GQ-2
title: Define the canonical GeoQuery AST
status: Done
assignee: []
created_date: '2026-09-30 14:35'
updated_date: '2026-10-04 18:00'
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
  - .knowledge/CONTEXT.md
modified_files:
  - crates/types/src/lib.rs
  - crates/types/src/query.rs
  - crates/types/tests/query.rs
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
- [x] #1 GeoQuery models scope, execution options, spatial predicates, temporal predicates, result shaping, and semantic intent using typed public fields.
- [x] #2 Spatial and temporal values have explicit CRS and interval semantics, including valid open-ended intervals where the design allows them.
- [x] #3 Valid representative query documents deserialize and serialize deterministically.
- [x] #4 Invalid query shapes and invalid intervals produce actionable validation errors rather than reaching an adapter.
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added the canonical GeoQuery AST to geoquery-types: a protocol-independent query with typed public fields for scope, execution options, semantic intent, spatial and temporal predicates, and result shaping. Every member is optional and every struct is closed to unknown fields, so a predicate this version cannot model is refused rather than silently dropped and answered too broadly. Predicate vocabularies that a source may extend stay open; the ones that name engine behaviour are closed. Spatial predicates carry an explicit CRS, and instants accept either an RFC 3339 timestamp or a bare date, normalising to timestamps so that a round trip is a fixed point rather than a reproduction of how the caller typed it. Open-ended intervals are valid at either end, an interval with no bound at all is not. GeoQuery::validate reports every contradiction it finds, not the first, as a typed error enum. Validation sits in types rather than core because adapters depend on types only, and the errors have to be raised before a source is contacted. Covered by 26 integration tests built from the documents in the query knowledge corpus. The CLI and engine still read queries through the Phase 0 QueryDocument container; routing them through the AST changes which documents the CLI accepts and belongs with GQ-10 and GQ-12.
<!-- SECTION:FINAL_SUMMARY:END -->
