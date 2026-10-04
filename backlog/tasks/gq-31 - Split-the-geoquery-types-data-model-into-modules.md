---
id: GQ-31
title: Split the geoquery-types data model into modules
status: Done
assignee: []
created_date: '2026-10-04 18:19'
updated_date: '2026-10-04 20:17'
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
- [x] #1 The open_string_enum macro lives in its own module and is imported where it is used, so module declarations sit at the top of lib.rs again.
- [x] #2 The data model is split along the seams it already has, with one test file mirroring each new source file.
- [x] #3 The public surface of geoquery-types is unchanged: every name that resolved from the crate root before still does.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The split follows the order the file already had. macros.rs holds the macro. resource.rs, service.rs and result.rs take the structs in the three groups the file already listed them in. The ten open string enums went with their domain rather than into one shared vocabulary module, so ResourceType sits beside ResourceDescriptor. SpatialOperation was the one judgement call: both query and service read it, and it went to service because that is where CapabilitySet declares which operations a service supports.

lib.rs went from 999 lines to 61 - the crate docs, four type aliases, the module list and the re-exports, nothing else.

AC3 was pinned before any code moved. tests/lib.rs now names all 54 public items and asserts each is a type, so a name that stops resolving from the crate root is a compile error instead of a silent break for every dependent. It was written and passing against the unsplit crate, which is the only reason it proves anything - a list written afterwards would have described wherever the names happened to land.

Rust tests went 181 to 185: the surface list plus three new tests for service.rs, which had no tests of its own. The strongest of them pins the three-state capability semantics, that a flag left unmentioned is not a flag declared unsupported. The test mirror is now exact at one file per source file, matching every other crate in the workspace.

Left deliberately out of scope: the macro body leaves fmt, serde and TS unqualified, so every module invoking it imports those four names. Fully qualifying the paths inside the macro would remove that coupling, but it changes the macro rather than the file layout, and AC1 asks for the import.
<!-- SECTION:NOTES:END -->
