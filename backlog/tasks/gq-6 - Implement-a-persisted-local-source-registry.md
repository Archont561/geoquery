---
id: GQ-6
title: Implement a persisted local source registry
status: To Do
assignee: []
created_date: '2026-09-30 14:35'
labels:
  - phase-1
  - core
  - registry
milestone: m-0
dependencies:
  - GQ-1
  - GQ-5
references:
  - .knowledge/infrastructure/storage.md
  - .knowledge/interfaces/cli.md
priority: medium
type: task
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The CLI needs a durable list of sources between invocations. Provide a small YAML-backed registry that stores discovered service descriptors locally while keeping the Phase 1 storage choice simple and inspectable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The registry can add, retrieve, list, and remove service descriptors through a documented core API.
- [ ] #2 Registry data persists as human-readable YAML and round-trips without losing IDs, capabilities, or extension fields.
- [ ] #3 Missing, malformed, and unwritable registry files return distinct actionable errors.
- [ ] #4 Tests use isolated per-test files so parallel test runs do not share state.
<!-- AC:END -->
