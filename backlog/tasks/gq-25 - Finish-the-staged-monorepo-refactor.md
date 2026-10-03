---
id: GQ-25
title: Finish the staged monorepo refactor
status: To Do
assignee: []
created_date: '2026-09-30 20:41'
labels:
  - monorepo
  - turbo
  - infrastructure
milestone: m-7
dependencies:
  - GQ-4
  - GQ-24
references:
  - backlog/docs/plans/monorepo-refactor.md
  - .knowledge/infrastructure/monorepo.md
priority: medium
type: task
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Stage 0 and most of Stage 1 have landed: the member glob, the language-workspace facades, the repo-wide turbo verbs. What is left is the codegen edge the facades were waiting for, and an honest record of which later stages are blocked rather than merely unstarted.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The types crate exposes the codegen facade whose build runs the codegen task and declares the generated client sources as its outputs.
- [ ] #2 The TypeScript package depends on that facade through turbo, so generation is an ordered cached edge instead of a manual prerequisite.
- [ ] #3 The pixi aggregators collapse onto turbo, leaving pixi owning only the repository-global checks.
- [ ] #4 Stage 2 and Stage 3 stay recorded with their trigger and their blocker, and the build directory stays out of the turbo outputs.
<!-- AC:END -->
