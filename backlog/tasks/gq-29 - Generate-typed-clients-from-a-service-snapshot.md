---
id: GQ-29
title: Generate typed clients from a service snapshot
status: To Do
assignee: []
created_date: '2026-09-30 22:10'
labels:
  - phase-3
  - cli
  - codegen
milestone: m-2
dependencies:
  - GQ-26
  - GQ-27
references:
  - .knowledge/codegen/client-generation.md
  - .knowledge/codegen/service-snapshot.md
  - .knowledge/interfaces/cli.md
  - .knowledge/extensions/extension-points.md
priority: medium
type: feature
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Once a service has been described, the same description compiles into a typed client for that one service — a build-time complement to run-time federation. Lower a snapshot into a language-neutral GenerationModel and emit code through a GeneratorBackend registry, starting with TypeScript. Geoquery already owns the expensive half of this problem: the adapters and the normalized descriptor. Generation must never rediscover anything, and no backend may contain protocol knowledge.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `geoquery-codegen` lowers a snapshot to a GenerationModel — accessors, types, operations — with shared identifier sanitation and collision handling, and no protocol-specific code in any backend.
- [ ] #2 `geoquery generate <source> --target typescript --out <dir>` emits a client whose CRS, format and style parameters are unions of what that server advertises, and the output compiles under `tsc --noEmit` against a recorded fixture service.
- [ ] #3 Generation performs no network I/O; a missing snapshot fails with an error naming `geoquery describe` rather than fetching.
- [ ] #4 Output is deterministic and dependency-light — same snapshot and generator version give byte-identical files, the TypeScript client needs nothing but platform fetch — and each file carries a header naming the source, snapshot digest and generator version.
- [ ] #5 `--check` exits 1 when regenerating would change any file, giving CI a gate against a stale committed client.
- [ ] #6 `geoquery targets` lists registered backends, the GeneratorBackend trait is implementable out of tree against geoquery-types only, and the whole surface is behind the `codegen` cargo feature.
<!-- AC:END -->
