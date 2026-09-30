---
id: GQ-5
title: 'Define adapter, planning, and execution contracts'
status: To Do
assignee: []
created_date: '2026-09-30 14:35'
labels:
  - phase-1
  - core
  - adapters
milestone: m-0
dependencies:
  - GQ-1
  - GQ-2
references:
  - .knowledge/adapters/adapter-architecture.md
  - AGENTS.md
priority: high
type: task
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Adapters need a stable, protocol-neutral boundary before any STAC HTTP code is written. Establish the core contracts that describe a service, its capabilities, query translation, normalized results, and failures without coupling core to a transport.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 geoquery-core exposes public adapter and execution contracts with clear ownership and async boundaries.
- [ ] #2 The contracts accept GeoQuery and return normalized GeoResult values with source-level provenance.
- [ ] #3 Capability reporting makes unsupported query features explicit instead of silently dropping them.
- [ ] #4 A fake adapter proves the contracts can be used by an external crate.
<!-- AC:END -->
