---
id: GQ-11
title: Add CLI commands for STAC source registration
status: To Do
assignee: []
created_date: '2026-09-30 14:35'
labels:
  - phase-1
  - cli
  - registry
milestone: m-0
dependencies:
  - GQ-6
  - GQ-7
references:
  - .knowledge/interfaces/cli.md
priority: medium
type: enhancement
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Users need a truthful command-line path to add and inspect queryable STAC sources before running a query. Expose discovery and persisted registry operations through the canonical Rust CLI rather than a separate utility.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 geoquery add accepts a STAC endpoint, performs discovery, and persists the resulting service and collection metadata.
- [ ] #2 geoquery sources lists registered sources and enough status or capability information to choose one.
- [ ] #3 CLI errors name the invalid endpoint or registry problem once and use documented exit codes.
- [ ] #4 End-to-end CLI tests assert stdout, stderr, and exit status using isolated registry files.
<!-- AC:END -->
