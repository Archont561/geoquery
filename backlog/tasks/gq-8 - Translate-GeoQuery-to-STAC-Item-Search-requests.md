---
id: GQ-8
title: Translate GeoQuery to STAC Item Search requests
status: To Do
assignee: []
created_date: '2026-09-30 14:35'
labels:
  - phase-1
  - stac
  - query
milestone: m-0
dependencies:
  - GQ-2
  - GQ-3
  - GQ-7
references:
  - .knowledge/adapters/stac.md
  - .knowledge/query/filters.md
priority: high
type: task
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The first real execution path needs a faithful, capability-aware translation from the canonical query AST to STAC Item Search. Translation must preserve intent and report degradations; it must never silently omit a user filter.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The adapter creates valid STAC Item Search POST requests for collection, bbox, datetime, limit, and supported filter inputs.
- [ ] #2 Datetime and geometry serialization follow STAC conventions and preserve GeoQuery semantics.
- [ ] #3 CQL2-JSON is emitted only when the discovered service advertises compatible filter support.
- [ ] #4 Unsupported query features produce an explicit capability error or documented degradation outcome, with request-shape tests for each case.
<!-- AC:END -->
