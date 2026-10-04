---
id: GQ-3
title: Model CQL2-compatible filter expressions
status: Done
assignee: []
created_date: '2026-09-30 14:35'
updated_date: '2026-10-04 19:01'
labels:
  - phase-1
  - types
  - cql2
milestone: m-0
dependencies:
  - GQ-2
references:
  - .knowledge/query/filters.md
modified_files:
  - crates/types/src/filter.rs
  - crates/types/tests/filter.rs
  - crates/types/src/lib.rs
  - crates/types/src/query.rs
  - crates/types/src/instant.rs
  - crates/types/tests/query.rs
  - crates/types/Cargo.toml
priority: high
type: task
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Attribute filtering is part of the canonical query language, not a raw vendor string. A typed expression tree is required to preserve safe intent and later compile it for STAC or OGC services without losing semantics.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The public filter AST covers property references, literals, comparison, boolean composition, and the documented spatial or temporal expression hooks.
- [x] #2 Filter values round-trip through the query JSON representation with unambiguous operator names and literal types.
- [x] #3 Unsupported or malformed expressions are rejected before request translation.
- [x] #4 Tests cover nested boolean expressions, literal edge cases, and invalid input.
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
A typed expression tree for attribute filtering, in its own module with the mirrored test file the repository requires. The tree is CQL2-inspired rather than a CQL2 clone, which is the position the corpus takes: it borrows property references, typed literals, comparison and boolean composition so every adapter has somewhere to compile to, and leaves the encoding alone, because a GeoQuery also carries federation scope and execution hints that CQL2 has no way to say.

Four decisions worth recording. CompareOp is closed where the predicate enums in query.rs are open: those are open because a service may support an operation this version has not heard of, but an operator is one this engine has to emit, so one it cannot name is one it cannot compile. IN and LIKE are their own variants rather than members of CompareOp, so a compiler walking the tree never has to ask whether the operand it was handed suits the operator it was given, and NOT IN needs no variant at all because it is NOT around the positive test. Temporal literals use the CQL2 JSON wrapper, so a string that merely looks like a date stays a string, while a bare date inside the wrapper is still read as an instant, which keeps the one-dialect rule from GQ-30. Numbers are held as JSON numbers rather than f64, so an integer does not come back a float and become a different query plan.

Spatial and temporal predicates stay in their own branches of the query, as the corpus specifies. The hook inside filters is the temporal literal, so a datetime queryable can be compared like any other property, and a compiler merges all three on the way out.

Shape and meaning fail in different places. An operator nothing can compile, a list where a scalar belongs, a member this version does not model: all refused while reading, so the variants are only ever inhabited by coherent nodes. An empty branch, a membership test against nothing or across two types, an ordering comparison on a boolean, a tree nested past 32 levels: all refused by validate, which reports every problem rather than the first and reaches the caller through GeoQuery::validate.

This also closes the sharp edge GQ-2 left behind. deny_unknown_fields meant the documented query-model example could not be parsed at all, because filters had no member to land in, and the query test that used filters as its example of an unmodelled member now uses rank, which really is one.

The new module is 323 of 323 lines covered, and the workspace is 128 tests green. Five mutations were planted and each was caught by the test that should catch it. The TypeScript the derives will emit was checked with the real compiler under strict, recursive alias included, and a test pins the hand-written overrides against the Rust they stand for, because ts-rs records no dependencies for a type override and GQ-4 turns them into the SDK bindings.
<!-- SECTION:FINAL_SUMMARY:END -->
