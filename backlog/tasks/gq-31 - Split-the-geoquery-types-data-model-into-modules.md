---
id: GQ-31
title: Split the geoquery-types data model into modules
status: To Do
assignee: []
created_date: '2026-10-04 18:19'
labels:
  - refactor
  - types
milestone: m-7
dependencies: []
priority: medium
type: task
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
crates/types/src/lib.rs is 976 lines: a macro definition, 10 open_string_enum invocations, 22 public structs and five impl blocks. Finding 5 of the code audit. The symptom is already visible, because open_string_enum is a macro_rules macro and therefore only in scope after its definition, the pub mod query declaration had to be wedged into the middle of the file below the macro and above an enum, where no reader looks for a module declaration. That workaround is documented in a comment rather than fixed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The open_string_enum macro lives in its own module and is imported where it is used, so module declarations sit at the top of lib.rs again.
- [ ] #2 The data model is split along the seams it already has, with one test file mirroring each new source file.
- [ ] #3 The public surface of geoquery-types is unchanged: every name that resolved from the crate root before still does.
<!-- AC:END -->
