---
id: GQ-10
title: Execute a single-source query through the core planner
status: To Do
assignee: []
created_date: '2026-09-30 14:35'
labels:
  - phase-1
  - core
  - execution
milestone: m-0
dependencies:
  - GQ-5
  - GQ-6
  - GQ-8
  - GQ-9
references:
  - .knowledge/query/planner.md
priority: high
type: task
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Phase 1 needs a complete but intentionally non-federated execution path. Connect a registered service, capability validation, STAC translation, and result normalization through a sequential planner so the CLI stops pretending to execute.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The planner selects one registered compatible source and dispatches it through the adapter contract.
- [ ] #2 Execution returns normalized results and source status, or a typed error that distinguishes no source, unsupported capability, transport failure, and malformed response.
- [ ] #3 The Phase 1 implementation is explicitly sequential and does not claim federation or partial-failure behavior it does not provide.
- [ ] #4 Integration tests execute the full core path against a deterministic mock STAC service.
<!-- AC:END -->
