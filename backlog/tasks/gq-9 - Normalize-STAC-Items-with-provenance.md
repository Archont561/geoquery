---
id: GQ-9
title: Normalize STAC Items with provenance
status: To Do
assignee: []
created_date: '2026-09-30 14:35'
labels:
  - phase-1
  - stac
  - normalization
milestone: m-0
dependencies:
  - GQ-1
  - GQ-8
references:
  - .knowledge/adapters/stac.md
  - .knowledge/project/data-model.md
priority: high
type: task
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A STAC response is provider-shaped data. Convert it at the adapter boundary into Geoquery's protocol-independent result model, retaining enough source and request context for users to audit where every result came from.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A STAC Feature or ItemCollection maps into GeoResult values with identifiers, geometry, properties, assets, and temporal metadata preserved where available.
- [ ] #2 Every normalized result carries endpoint, service, and request provenance sufficient to trace it to the source response.
- [ ] #3 Optional and malformed STAC fields are handled according to an explicit policy and do not cause silent data corruption.
- [ ] #4 Fixture-based tests cover multiple items, assets, optional geometry, and invalid payloads.
<!-- AC:END -->
