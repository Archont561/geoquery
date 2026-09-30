---
id: GQ-20
title: Build the interactive TUI over the shared event stream
status: To Do
assignee: []
created_date: '2026-09-30 20:41'
labels:
  - phase-5
  - tui
  - streaming
milestone: m-4
dependencies:
  - GQ-15
  - GQ-12
references:
  - .knowledge/interfaces/tui.md
  - .knowledge/interfaces/cli.md
priority: medium
type: feature
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Federation is the thing worth watching happen: which sources answered, which timed out, which degraded. crates/tui is a stub; make it a read-only browser driven by the same query event stream the HTTP endpoint streams, so it observes the engine rather than reimplementing it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An explore command launches a result browser with detail and provenance views for a single selected result.
- [ ] #2 A source dashboard reports live federation progress, including partial failures and degraded answers, while the query is still running.
- [ ] #3 The TUI consumes the same query event stream the HTTP streaming endpoint emits and contains no planning or execution logic of its own.
- [ ] #4 Scope stays read-only for this task: interactive query editing and geometry rendering are recorded as follow-ups rather than started here.
<!-- AC:END -->
