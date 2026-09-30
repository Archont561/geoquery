---
id: GQ-7
title: Discover STAC services and collections
status: To Do
assignee: []
created_date: '2026-09-30 14:35'
labels:
  - phase-1
  - stac
  - discovery
milestone: m-0
dependencies:
  - GQ-5
references:
  - .knowledge/adapters/stac.md
priority: high
type: task
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Before querying a STAC endpoint, Geoquery must know what it is talking to and which query features it advertises. Implement standards-based landing-page, conformance, and collection discovery rather than assuming one provider's behavior.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The STAC adapter detects a STAC API landing page and reports a useful non-STAC failure.
- [ ] #2 Collection discovery reads the STAC collections endpoint and captures collection metadata needed by source registration.
- [ ] #3 Conformance classes are mapped into CapabilitySet values without claiming unsupported features.
- [ ] #4 Mock HTTP tests cover a conforming service, absent optional fields, and malformed responses.
<!-- AC:END -->
