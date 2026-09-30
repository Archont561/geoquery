---
id: GQ-22
title: Open the 15 extension layers to out-of-tree implementations
status: To Do
assignee: []
created_date: '2026-09-30 20:41'
labels:
  - phase-7
  - extensions
  - plugins
milestone: m-6
dependencies:
  - GQ-5
  - GQ-10
  - GQ-29
references:
  - .knowledge/extensions/extension-points.md
  - .knowledge/adapters/adapter-architecture.md
priority: low
type: feature
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The golden rule of the design is that customizing Geoquery must never require editing core code. Turn the fifteen documented layers into a public, composable trait surface and prove it from outside the workspace.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every documented extension layer is reachable through a public trait exported from the core crate, with no core edit needed to add an implementation.
- [ ] #2 A builder or registry wires user implementations at startup, covered by tests that add an adapter, a ranking strategy, a result transformer, and a generator backend from outside the crate.
- [ ] #3 The plugin manifest format and the dynamic loading strategy are decided and documented, including what is deliberately left static.
- [ ] #4 At least one out-of-tree example extension is built in CI so the surface cannot regress unnoticed.
<!-- AC:END -->
