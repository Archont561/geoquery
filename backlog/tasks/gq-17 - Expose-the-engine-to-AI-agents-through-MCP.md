---
id: GQ-17
title: Expose the engine to AI agents through MCP
status: To Do
assignee: []
created_date: '2026-09-30 20:41'
labels:
  - phase-4
  - mcp
  - ai
milestone: m-3
dependencies:
  - GQ-15
  - GQ-4
references:
  - .knowledge/interfaces/mcp.md
  - .knowledge/project/overview.md
priority: high
type: feature
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The point of the project is that an agent should not have to understand forty GIS protocols. crates/mcp is a stub; give it the three documented tools and the registry resources so an agent issues one structured query and gets normalized, attributed results back.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 geo_query, geo_resource, and geo_resolve are served with JSON Schemas produced by codegen rather than schemas maintained by hand.
- [ ] #2 MCP resources expose the registry and individual resource context under the documented geo:// URIs.
- [ ] #3 The server speaks Streamable HTTP per the 2026 MCP spec and reuses the HTTP crate's execution path instead of a parallel one.
- [ ] #4 Tool descriptions state degradation and provenance semantics, and tests drive the server through a real MCP client session.
<!-- AC:END -->
