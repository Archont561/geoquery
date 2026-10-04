---
id: GQ-30
title: Act on the code audit
status: In Progress
assignee: []
created_date: '2026-10-04 18:18'
labels:
  - refactor
  - tests
  - tooling
milestone: m-7
dependencies: []
priority: high
type: task
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
An audit of the repository against the tdd and refactor skills found nine defects. Eight are fixed here. The ninth, splitting the 976-line geoquery-types data model into modules, is large enough to review on its own and is tracked separately.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every instant on the wire in geoquery-types is read in one dialect, so a bare date is accepted everywhere a timestamp is, and sub-second precision survives a round trip.
- [ ] #2 TypeScript line coverage is written to a file by every package that has tests, and CI uploads it alongside the Rust and Python reports.
- [ ] #3 No dependency is declared that nothing uses: rstest, insta and geo-types are gone or genuinely exercised.
- [ ] #4 json_type_name is internal to geoquery-core, and the test that existed only to reach its unreachable arm is gone.
- [ ] #5 The user-agent test asserts the properties an operator depends on rather than rebuilding the string from the format used by the implementation.
- [ ] #6 The seven scaffolding test headers are grammatical and fit the 100-column width used across the repository.
- [ ] #7 The large descriptor and result fixtures live in crates/types/tests/fixtures/ and the too_many_lines allow is gone.
- [ ] #8 AGENTS.md requires the tdd and refactor skills for any change to source code.
<!-- AC:END -->
