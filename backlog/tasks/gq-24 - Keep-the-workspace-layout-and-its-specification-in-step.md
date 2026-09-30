---
id: GQ-24
title: Keep the workspace layout and its specification in step
status: To Do
assignee: []
created_date: '2026-09-30 20:41'
labels:
  - monorepo
  - infrastructure
  - docs
milestone: m-7
dependencies: []
references:
  - .knowledge/infrastructure/monorepo.md
priority: medium
type: chore
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The monorepo concept already carries a divergence section because the root manifest is a package rather than a virtual workspace. Keep that section true as crates land, so the layout stays one decision recorded once instead of drifting into folklore.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The documented directory layout matches the workspace: a root package, crates/* with the cli exclusion, crates/xtask, and apps/docs.
- [ ] #2 Lints, dependency versions, and release metadata are inherited from the workspace root by every crate, with no per-crate duplicates.
- [ ] #3 Adding a crate requires one manifest edit, verified against cargo metadata rather than by inspection.
- [ ] #4 Every crate keeps tests mirroring its sources and exercising the public surface, as the specification claims.
<!-- AC:END -->
