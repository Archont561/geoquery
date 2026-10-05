---
title: Philosophy, model, and API
description: Why geoquery is GraphQL-like but not GraphQL, what the query model is, and which user-facing APIs exist today.
---

Geoquery sits above geospatial services the way a query planner sits above databases: the
caller writes one structured request, and adapters translate the parts a source can handle
into that source's own protocol. STAC, OGC API, WFS, ArcGIS, CMR, SQL and CQL2 remain in
the system, but they are targets below the public model rather than the language callers
write first.

## GraphQL-like, not GraphQL

"GraphQL for geospatial services" is a useful shorthand if it means one request shape over
many backends. It is misleading if it means GraphQL syntax, a GraphQL schema or a resolver
runtime.

Geoquery's source language is the `GeoQuery` document. Its adapters compile that document
into source-native requests, then normalize the answers back into results with provenance.
That gives a client one mental model without pretending every geospatial protocol exposes
the same capabilities.

The design principles are:

- **Intent over endpoint syntax.** Ask for resources by space, time, attributes, semantic
  intent and result shape; do not hand-code each provider's parameter dialect.
- **Standards reuse over reinvention.** GeoJSON, STAC, OGC API and CQL2 are reused where
  they already solve the problem. Geoquery adds the federation layer around them.
- **Capability-aware execution.** A source must say whether a predicate can be pushed down,
  approximated, run locally or refused. A result should be correct and auditable, not just
  present.
- **Provenance first.** Every normalized result carries where it came from and enough detail
  to understand how it was obtained.
- **One core, many surfaces.** The CLI, Python SDK, TypeScript SDK, future HTTP service,
  MCP tools and TUI are transports or presentations of the same model.
- **Honest phases.** Until the execution engine exists, commands and SDKs parse documents
  and report the missing engine instead of contacting a service or returning fake results.

## The canonical model

The public model is a JSON-serializable AST. Its important split is between federation
concerns and source-level predicates:

```ts
interface GeoQuery {
  // Federation concerns: which sources participate, and how the run is planned.
  scope?: QueryScope;
  execution?: ExecutionOptions;

  // Discovery intent and deterministic predicates.
  semantic?: string;
  spatial?: SpatialPredicate;
  temporal?: TemporalPredicate;
  filters?: FilterExpression;

  // Result shaping.
  sort?: SortExpression[];
  limit?: number;
  offset?: number;
  fields?: string[];
  include?: IncludeOptions;
}
```

A query can combine deterministic predicates with optional semantic intent:

```json
{
  "scope": { "providers": ["NASA", "Copernicus"] },
  "semantic": "cloud-free flood imagery",
  "spatial": { "op": "bbox", "bbox": [14.1, 49.0, 24.2, 54.8] },
  "temporal": {
    "op": "intersects",
    "start": "2024-05-01T00:00:00Z",
    "end": "2024-09-30T23:59:59Z"
  },
  "filters": {
    "and": [
      { "field": "cloud_cover", "op": "<", "value": 10 },
      { "field": "platform", "op": "=", "value": "sentinel-2" }
    ]
  },
  "sort": [{ "field": "datetime", "direction": "desc" }],
  "limit": 20,
  "include": { "assets": true }
}
```

The planner-facing model around that document is deliberately small:

```text
GeoQuery document
  ↓ validation
ServiceDescriptor + CapabilitySet for each source
  ↓ planning and adapter translation
STAC / OGC API / ArcGIS / CMR / SQL / CQL2 requests
  ↓ normalization
GeoResult[] + per-source SourceStatus
```

`ServiceDescriptor` records how to reach a source and what it advertises. `CapabilitySet`
records what can be pushed down. `GeoResult` is the normalized result shape, and it requires
provenance so a federated row cannot be detached from the source that produced it.

## Attribute filters and CQL2

CQL2 is a compilation target, not the source language. The filter subtree borrows CQL2's
operators because STAC API and OGC API Features already use CQL2-compatible filtering, but
Geoquery keeps filters inside the larger query AST:

```json
{
  "filters": {
    "and": [
      { "field": "cloud_cover", "op": "<", "value": 10 },
      { "field": "platform", "op": "=", "value": "sentinel-2" }
    ]
  }
}
```

An adapter may compile that to CQL2 JSON, CQL2 text, SQL, ArcGIS `where`, WFS/FES or a
service-specific parameter set. Spatial and temporal predicates have their own AST branches
so the planner can reason about them before all predicates are merged into a backend format.

## User-facing API today

The current release exposes the core through thin surfaces. None of them executes a live
query yet.

| Surface | Calls | What happens today |
| --- | --- | --- |
| CLI | `geoquery check <query.json>` | Reads JSON, requires a top-level object and reports sorted keys. |
| CLI | `geoquery query --service <url> --query <query.json>` | Checks the document, then exits `3` because no engine exists yet. |
| Python | `geoquery.parse_document(text)` | Uses the same Rust parser in-process through PyO3. |
| Python | `geoquery.protocol_version()`, `ping()`, `invoke()` | Inspect or call the versioned JSON transport directly. |
| TypeScript | `parseDocument(text)` | Uses the same Rust parser in-process through N-API. |
| TypeScript | `protocolVersion()`, `ping()`, `invoke()` | Inspect or call the versioned JSON transport directly. |
| Rust | `geoquery-types`, `geoquery-core`, `geoquery-protocol` | Own the AST, validation rules, adapter contracts and transport envelope. |

The Python and TypeScript SDKs do not shell out to the CLI and do not call an HTTP service.
They load native bindings around the Rust engine boundary. That is why the public functions
are small today: execution can be added behind the same transport without inventing a second
query language.

For the product loop and interaction principles these APIs should grow into, see
[Design direction](./design/).

## User-facing API as the engine lands

The target shape is the same query document through different ergonomics:

```ts
await geo.query({
  spatial: { op: "bbox", bbox: [14.1, 49.0, 24.2, 54.8] },
  temporal: { op: "intersects", start: "2024-05-01", end: "2024-09-30" },
  filters: { field: "cloud_cover", op: "<", value: 10 },
  limit: 20
});
```

```http
POST /query
Content-Type: application/json

{
  "spatial": { "op": "bbox", "bbox": [14.1, 49.0, 24.2, 54.8] },
  "limit": 20
}
```

```text
MCP tool: geo_query
arguments: the same GeoQuery JSON object
```

Those surfaces should differ only in transport, language conventions and response streaming.
The meaning of a field belongs to the AST, not to an individual client.
