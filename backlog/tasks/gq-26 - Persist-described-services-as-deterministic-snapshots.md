---
id: GQ-26
title: Persist described services as deterministic snapshots
status: To Do
assignee: []
created_date: '2026-09-30 22:10'
labels:
  - phase-1
  - core
  - registry
  - snapshot
milestone: m-0
dependencies:
  - GQ-1
  - GQ-5
  - GQ-6
references:
  - .knowledge/codegen/service-snapshot.md
  - .knowledge/project/data-model.md
  - .knowledge/interfaces/cli.md
priority: high
type: feature
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What `describe()` learns about a source currently lives only for the duration of a process. Serialize it as a deterministic, committable service snapshot so the planner can work offline, so drift becomes a reviewable diff, and so client generation has a stable compile input. This is the foundation the diff, check and generate work all sit on, which is why it lands in Phase 1 rather than alongside them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A ServiceDescriptor plus the ResourceDescriptors discovered under it serialize to `~/.geoquery/sources/<id>.json`, with the snapshot hash and fetch provenance recorded in `geoquery.lock`.
- [ ] #2 Serialization is byte-stable: every array sorted by a defined key, maps emitted with sorted keys, no wall-clock data in the hashed body, and unadvertised fields absent rather than defaulted.
- [ ] #3 Re-describing an unchanged fixture service produces an identical digest, proven by a round-trip test.
- [ ] #4 `geoquery describe <source>` prints the stored snapshot without network access, and `--refresh` refetches and rewrites it.
- [ ] #5 A snapshot_version field is present and a mismatched version is rejected with an actionable error rather than misparsed.
<!-- AC:END -->
