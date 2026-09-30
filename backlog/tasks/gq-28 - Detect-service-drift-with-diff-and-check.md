---
id: GQ-28
title: Detect service drift with diff and check
status: To Do
assignee: []
created_date: '2026-09-30 22:10'
labels:
  - phase-2
  - cli
  - snapshot
  - ci
milestone: m-1
dependencies:
  - GQ-26
  - GQ-27
references:
  - .knowledge/codegen/service-snapshot.md
  - .knowledge/interfaces/cli.md
  - .knowledge/query/planner.md
priority: medium
type: feature
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A remote service that quietly drops a collection, a CRS or a filter operation changes what the planner can push down, and today that failure surfaces as degraded results rather than as a signal. Compare the stored snapshot against the live service and make the difference a first-class, CI-gateable report. This extends the planner's existing rule — degradation is never silent — from query time back to describe time.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `geoquery diff <source>` reports added, removed and changed collections, capabilities, CRSs and formats in both a human-readable and a `--format json` form.
- [ ] #2 `geoquery check` walks every registered source and exits 0 fresh, 1 drifted, 2 unreachable.
- [ ] #3 Removed capabilities and removed collections are classified breaking, additions additive, and `--check --breaking-only` fails on the former only.
- [ ] #4 A drift report names the consequence, not just the delta — a removed spatial op states which pushdowns now degrade.
- [ ] #5 The planner can build and explain a plan from snapshots alone under `--offline`, with no network access, and says so in the plan explanation.
<!-- AC:END -->
