---
id: GQ-15
title: Serve the query engine over HTTP
status: To Do
assignee: []
created_date: '2026-09-30 20:40'
labels:
  - phase-3
  - http
  - api
milestone: m-2
dependencies:
  - GQ-10
  - GQ-6
references:
  - .knowledge/interfaces/http.md
  - .knowledge/project/architecture.md
priority: high
type: feature
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Every other integration surface is a consumer of the HTTP API, so it is the universal integration point and must exist before the SDKs, the MCP server, or the TUI have anything to talk to. crates/http is a stub; give it the Axum surface the design describes, including per-source status in the response body.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 POST /query executes a GeoQuery and returns normalized results together with per-source status rather than a single opaque success or failure.
- [ ] #2 Registry endpoints cover listing, reading, and registering resources and listing services, backed by the same persisted registry the CLI uses.
- [ ] #3 Errors are JSON with documented status codes, name the failing source, and never leak adapter internals or upstream credentials.
- [ ] #4 An OpenAPI description is generated from the routes and verified in gates, and integration tests drive the server through its HTTP surface.
<!-- AC:END -->
