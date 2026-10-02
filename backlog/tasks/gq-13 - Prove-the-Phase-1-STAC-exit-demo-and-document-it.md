---
id: GQ-13
title: Prove the Phase 1 STAC exit demo and document it
status: To Do
assignee: []
created_date: '2026-09-30 14:35'
labels:
  - phase-1
  - testing
  - docs
milestone: m-0
dependencies:
  - GQ-12
references:
  - backlog/milestones/
  - README.md
priority: medium
type: task
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Phase 1 milestone is complete only when the advertised workflow is reproducible and verified against both deterministic fixtures and a real STAC provider. Document the supported surface and prevent an accidental return to the Phase 0 stub.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The default test suite covers the complete add-and-query workflow using a deterministic local or mocked STAC service.
- [ ] #2 An opt-in integration test or documented verification command runs the exit demo against Planetary Computer without making normal gates network-dependent.
- [ ] #3 README and documentation describe the real Phase 1 commands, expected normalized output, and current single-source limitations.
- [ ] #4 pixi run gates passes after documentation and test changes, apart from explicitly documented external registry availability.
<!-- AC:END -->
