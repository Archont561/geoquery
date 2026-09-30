---
id: GQ-3
title: Model CQL2-compatible filter expressions
status: To Do
assignee: []
created_date: '2026-09-30 14:35'
labels:
  - phase-1
  - types
  - cql2
milestone: m-0
dependencies:
  - GQ-2
references:
  - .knowledge/query/filters.md
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
- [ ] #1 The public filter AST covers property references, literals, comparison, boolean composition, and the documented spatial or temporal expression hooks.
- [ ] #2 Filter values round-trip through the query JSON representation with unambiguous operator names and literal types.
- [ ] #3 Unsupported or malformed expressions are rejected before request translation.
- [ ] #4 Tests cover nested boolean expressions, literal edge cases, and invalid input.
<!-- AC:END -->
