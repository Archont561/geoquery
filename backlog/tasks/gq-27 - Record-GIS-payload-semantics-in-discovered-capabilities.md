---
id: GQ-27
title: Record GIS payload semantics in discovered capabilities
status: To Do
assignee: []
created_date: '2026-09-30 22:10'
labels:
  - phase-2
  - core
  - adapters
  - capabilities
milestone: m-1
dependencies:
  - GQ-5
  - GQ-26
references:
  - .knowledge/project/data-model.md
  - .knowledge/adapters/adapter-architecture.md
  - .knowledge/codegen/service-snapshot.md
priority: high
type: feature
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
CapabilitySet currently answers "can this source do intersects?" but not "what does a valid request to it look like". Extend it with the payload semantics a snapshot consumer needs — CRS with axis order, extents, formats, styles, dimensions, paging and scale range — and make adapters populate them. Axis order in particular must be recorded rather than assumed: EPSG:4326 is lat/lon in some protocol versions and lon/lat in others, and guessing is the most common source of silently wrong geospatial results.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CapabilitySet carries crs (code plus axis order), bboxes, formats, styles, dimensions, paging and scaleRange, and the types are exported to TypeScript and the JSON schemas by the existing codegen task.
- [ ] #2 The STAC and OGC adapters populate every field their protocol advertises, and record an empty list for "not advertised" rather than inventing a default.
- [ ] #3 Values that a protocol permits but a server did not advertise never appear in a descriptor, verified against recorded fixtures.
- [ ] #4 The planner reprojects or reorders coordinates according to the recorded axis order instead of a per-protocol assumption, covered by a test using a lat/lon-ordered fixture.
- [ ] #5 `describe --refresh` issues a conditional request with ETag/If-None-Match, and a 304 refreshes fetch provenance while leaving the snapshot digest unchanged.
<!-- AC:END -->
