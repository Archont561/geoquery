---
id: GQ-30
title: Act on the code audit
status: Done
assignee: []
created_date: '2026-10-04 18:18'
updated_date: '2026-10-04 18:34'
labels:
  - refactor
  - tests
  - tooling
milestone: m-7
dependencies: []
modified_files:
  - crates/types/src/instant.rs
  - crates/types/src/lib.rs
  - crates/types/src/query.rs
  - crates/types/tests/lib.rs
  - crates/core/src/document.rs
  - crates/core/tests/version.rs
  - .github/workflows/ci.yml
  - AGENTS.md
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
- [x] #1 Every instant on the wire in geoquery-types is read in one dialect, so a bare date is accepted everywhere a timestamp is, and sub-second precision survives a round trip.
- [x] #2 TypeScript line coverage is written to a file by every package that has tests, and CI uploads it alongside the Rust and Python reports.
- [x] #3 No dependency is declared that nothing uses: rstest, insta and geo-types are gone or genuinely exercised.
- [x] #4 json_type_name is internal to geoquery-core, and the test that existed only to reach its unreachable arm is gone.
- [x] #5 The user-agent test asserts the properties an operator depends on rather than rebuilding the string from the format used by the implementation.
- [x] #6 The seven scaffolding test headers are grammatical and fit the 100-column width used across the repository.
- [x] #7 The large descriptor and result fixtures live in crates/types/tests/fixtures/ and the too_many_lines allow is gone.
- [x] #8 AGENTS.md requires the tdd and refactor skills for any change to source code.
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Eight of the nine audit findings, in seven commits so that each one can be read on its own. One behaviour change and seven refactors, which is why only the first was done test-first: the tdd skill puts refactoring outside the red-green loop, and for a refactor the existing suite is the safety net rather than a new failing test. 94 Rust tests green throughout, plus the TypeScript and Python suites. Highlights: the query AST and the descriptor types no longer disagree about whether a bare date is an instant, which was a regression shipped in GQ-2; TypeScript coverage reaches Codecov for the first time, so the project number is now measured over all three languages rather than over a denominator that silently excluded one of them; and rstest, insta and geo-types are gone along with three doc comments that cited a fixture framework no test was using. Coverage across Rust, TypeScript and Python is 540 of 542 lines, the two misses being a match arm nothing can reach and one line of a bun test helper. AGENTS.md now requires both skills for any change under a src directory. GQ-31 carries the remaining finding.
<!-- SECTION:FINAL_SUMMARY:END -->
