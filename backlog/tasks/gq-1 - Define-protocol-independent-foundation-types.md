---
id: GQ-1
title: Define protocol-independent foundation types
status: Done
assignee: []
created_date: '2026-09-30 14:34'
updated_date: '2026-10-03 22:19'
labels:
  - phase-1
  - types
milestone: m-0
dependencies: []
references:
  - .knowledge/project/data-model.md
  - AGENTS.md
modified_files:
  - Cargo.toml
  - Cargo.lock
  - crates/types/src/lib.rs
  - crates/types/tests/lib.rs
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
- [x] #1 geoquery-types exposes ResourceDescriptor, ServiceDescriptor, CapabilitySet, GeoResult, and Provenance as documented public types.
- [x] #2 Extensible enums preserve unrecognised protocol values instead of rejecting or silently coercing them.
- [x] #3 Representative values round-trip through serde JSON without loss, including extension fields and provenance.
- [x] #4 Integration tests exercise the public crate surface rather than private implementation details.
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented the geoquery-types foundation model: protocol-independent resource, service, capability, result, and provenance types with open string enums, serde/TS derives, constructors, extension maps, and public integration tests covering enum preservation plus descriptor/result round-trips.
<!-- SECTION:FINAL_SUMMARY:END -->
