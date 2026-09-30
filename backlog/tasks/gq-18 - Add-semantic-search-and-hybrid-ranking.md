---
id: GQ-18
title: Add semantic search and hybrid ranking
status: To Do
assignee: []
created_date: '2026-09-30 20:41'
labels:
  - phase-4
  - semantic
  - ranking
milestone: m-3
dependencies:
  - GQ-10
  - GQ-2
references:
  - .knowledge/query/semantic.md
  - .knowledge/query/planner.md
priority: medium
type: feature
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Semantic intent is the one clause the engine cannot delegate to a source, and the design is explicit that it must never be silently turned into a spatial or temporal constraint. Build the indexing and ranking layer that answers semantic clauses locally, with its own failure mode when no provider is configured.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An EmbeddingProvider trait with a local default provider and a full-text index over registry metadata answer the semantic clause without calling a remote service by default.
- [ ] #2 The separation rules hold under test: a semantic clause never becomes a spatial, temporal, or attribute constraint, and a place name in free text is not silently geocoded.
- [ ] #3 Hybrid ranking combines semantic, spatial, and temporal signals into a score the result explains rather than asserts.
- [ ] #4 A semantic query with no provider configured degrades visibly and says so, instead of returning unranked results as if they were ranked.
<!-- AC:END -->
