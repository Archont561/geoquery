---
id: GQ-12
title: Execute registered STAC queries from the CLI
status: To Do
assignee: []
created_date: '2026-09-30 14:35'
labels:
  - phase-1
  - cli
  - stac
milestone: m-0
dependencies:
  - GQ-10
  - GQ-11
references:
  - README.md
  - .knowledge/interfaces/cli.md
priority: high
type: enhancement
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace the deliberate Phase 0 exit-code-3 stub with an honest end-user query command. The CLI must load a query document, select a registered source, execute the Phase 1 core path, and print normalized output with provenance.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 geoquery query accepts a valid GeoQuery document and executes it against a selected or eligible registered STAC source.
- [ ] #2 Successful output is machine-readable JSON containing normalized results and source-level provenance.
- [ ] #3 Invalid documents, missing sources, unsupported capabilities, and service failures retain distinct documented exit behavior.
- [ ] #4 The previous no-engine message and exit code 3 are removed only for the implemented execution path; unimplemented modes remain explicit.
<!-- AC:END -->
