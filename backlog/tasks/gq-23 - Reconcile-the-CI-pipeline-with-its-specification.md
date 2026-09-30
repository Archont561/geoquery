---
id: GQ-23
title: Reconcile the CI pipeline with its specification
status: To Do
assignee: []
created_date: '2026-09-30 20:41'
labels:
  - ci
  - infrastructure
milestone: m-7
dependencies: []
references:
  - .knowledge/infrastructure/ci.md
  - .knowledge/infrastructure/xtask.md
priority: medium
type: chore
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The knowledge base still describes a cargo xtask CI pipeline while the workflows run pixi and turbo, and the future-job table has no owner for the checks that are missing. Close the gap in both directions so the document is a design a reader can trust, not a claim the build contradicts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The CI concept records the implemented pixi and turbo pipeline, or records the divergence in the file it diverges from, per the repository convention.
- [ ] #2 The MSRV check runs in CI so the resolver-3 rust-version claim is verified rather than asserted in a manifest.
- [ ] #3 Every job in the future-job table names the phase that owns it and the trigger that introduces it.
- [ ] #4 A single local command reproduces the CI checks, and the workflows call it instead of restating the steps.
<!-- AC:END -->
