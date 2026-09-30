---
id: GQ-21
title: Package the engine for its deployment targets
status: To Do
assignee: []
created_date: '2026-09-30 20:41'
labels:
  - phase-6
  - deployment
  - docker
  - edge
milestone: m-5
dependencies:
  - GQ-15
references:
  - .knowledge/infrastructure/deployment.md
  - .knowledge/infrastructure/storage.md
priority: medium
type: feature
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The same binary is meant to run on a laptop, in a container, and at the edge, and the differences between those are storage tier and capability, not features that quietly disappear. Make each target buildable and explicit about what it supports.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A multi-stage container image runs the HTTP server, and a compose file brings it up with a documented storage tier.
- [ ] #2 Release binaries are produced for the supported native targets through the existing release workflow rather than a parallel script.
- [ ] #3 An edge build runs stateless remote federation and refuses the features that tier cannot support instead of failing silently at runtime.
- [ ] #4 Deployment documentation states, per target, which storage tier and which capabilities apply.
<!-- AC:END -->
