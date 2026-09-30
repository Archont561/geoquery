---
id: GQ-1
title: Define protocol-independent foundation types
status: To Do
assignee: []
created_date: '2026-09-30 14:34'
labels:
  - phase-1
  - types
milestone: m-0
dependencies: []
references:
  - .knowledge/project/data-model.md
  - AGENTS.md
priority: high
type: task
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The current workspace can read a JSON object but has no stable domain vocabulary. Define the shared public types that every transport, adapter, and SDK will use so the Phase 1 engine does not embed protocol assumptions in the CLI or STAC adapter.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 geoquery-types exposes ResourceDescriptor, ServiceDescriptor, CapabilitySet, GeoResult, and Provenance as documented public types.
- [ ] #2 Extensible enums preserve unrecognised protocol values instead of rejecting or silently coercing them.
- [ ] #3 Representative values round-trip through serde JSON without loss, including extension fields and provenance.
- [ ] #4 Integration tests exercise the public crate surface rather than private implementation details.
<!-- AC:END -->
