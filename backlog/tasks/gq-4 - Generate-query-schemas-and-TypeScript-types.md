---
id: GQ-4
title: Generate query schemas and TypeScript types
status: To Do
assignee: []
created_date: '2026-09-30 14:35'
labels:
  - phase-1
  - codegen
  - typescript
milestone: m-0
dependencies:
  - GQ-1
  - GQ-2
  - GQ-3
references:
  - .knowledge/infrastructure/xtask.md
  - .knowledge/interfaces/typescript.md
priority: medium
type: task
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Python and TypeScript surfaces must not hand-maintain a divergent copy of the query contract. Turn the existing xtask scaffold into a deterministic generator for the public schema artifacts derived from the Rust types.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 cargo xtask codegen generates the agreed JSON Schema and TypeScript artifacts from the public query types.
- [ ] #2 Generated artifacts are deterministic and checked into the repository at documented paths.
- [ ] #3 A verification test or gate fails when generated artifacts are stale.
- [ ] #4 The TypeScript package consumes or validates the generated query types without hand-copied equivalents.
<!-- AC:END -->
