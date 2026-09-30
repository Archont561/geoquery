---
id: GQ-14
title: Add an OGC API Features and Records adapter
status: To Do
assignee: []
created_date: '2026-09-30 20:40'
labels:
  - phase-2
  - adapters
  - ogc
milestone: m-1
dependencies:
  - GQ-5
  - GQ-2
  - GQ-3
references:
  - .knowledge/adapters/ogc.md
  - .knowledge/project/standards.md
priority: high
type: feature
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Federation is only proven when a second protocol answers the same canonical query. crates/adapter-ogc is a stub; give it the discovery, translation, and normalization path that the STAC adapter has, so the planner selects between two genuinely different protocols rather than two STAC endpoints.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Landing page and conformance detection classify an endpoint as OGC API Features, Records, or both, and populate a CapabilitySet from the declared conformance classes.
- [ ] #2 Collection and queryables discovery registers collections with the properties that are actually filterable at the source.
- [ ] #3 Scope, spatial, temporal, and filter clauses compile to /collections/{id}/items requests using bbox, datetime, and CQL2 only where conformance allows it.
- [ ] #4 Responses normalize to GeoResult with provenance, and the adapter is tested against recorded OGC fixtures without network access.
<!-- AC:END -->
