---
id: interfaces/typescript
title: TypeScript SDK — @geoquery/client
category: interfaces
tags: [TypeScript, SDK, client, WASM, ts-rs, codegen, fluent]
refs: [interfaces/http, query/query-model, project/data-model, infrastructure/xtask]
status: draft
created: 2025-07-11
updated: 2025-07-11
---

# TypeScript SDK — `@geoquery/client`

## Package

`@geoquery/client` — TypeScript/JavaScript SDK for Geoquery.

The primary application-facing API for web and Node.js developers.

---

## Transport Options

| Option | Status | Use Case |
|--------|--------|----------|
| **HTTP Client** | MVP | Talks to `geoquery-http` (Rust/Axum) |
| **WASM** | Phase 6 | In-browser/edge via `geoquery-wasm` |
| **NAPI** | Future | Node.js native via `napi-rs` |

**Start with HTTP.** It works everywhere and the HTTP API is already
in the spec. WASM and NAPI can be added later without changing the
TS API surface.

```
Option A (MVP):  @geoquery/client  ──HTTP──▶  geoquery-http (Axum)
Option B (future): @geoquery/client  ──WASM──▶  geoquery-wasm (.wasm)
Option C (future): @geoquery/client  ──FFI───▶  geoquery-napi (napi-rs)
```

---

## Package Structure

```
packages/client/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts          # Public API
│   ├── client.ts         # HTTP client implementation
│   ├── types.ts          # Auto-generated from Rust (ts-rs)
│   ├── query-builder.ts  # Fluent query builder
│   └── mcp.ts            # MCP client helpers (optional)
└── tests/
```

### `package.json`

```json
{
  "name": "@geoquery/client",
  "version": "0.1.0",
  "type": "module",
  "main": "./dist/index.js",
  "types": "./dist/index.d.ts",
  "exports": {
    ".": {
      "import": "./dist/index.js",
      "types": "./dist/index.d.ts"
    }
  },
  "dependencies": {
    "ofetch": "^1.4"
  },
  "devDependencies": {
    "typescript": "^5.7",
    "vitest": "^3.0"
  }
}
```

---

## API Surface

### Initialization

```typescript
import { Geoquery } from "@geoquery/client";

const geo = new Geoquery({
  url: "http://localhost:8080",  // Rust HTTP server
});
```

### Fluent Query Builder

```typescript
const results = await geo.query()
  .semantic("flood risk")
  .intersects(warsawPolygon)
  .during("2020-01-01", "2025-01-01")
  .where("cloud_cover", "<", 10)
  .limit(20)
  .execute();
```

### Raw AST

```typescript
const results = await geo.query({
  semantic: "flood risk",
  spatial: { op: "intersects", geometry: warsawPolygon },
  temporal: { op: "during", start: "2020-01-01", end: "2025-01-01" },
  limit: 20,
});
```

### Resource Discovery

```typescript
const resources = await geo.resources({ type: "dataset", tag: "flood" });
const resource = await geo.resource("copernicus:sentinel-2-l2a");
```

### Place Resolution

```typescript
const geometry = await geo.resolve("Warsaw, Poland");
```

### Output Conversion

```typescript
const geojson = results.toGeoJSON();
const features = results.features;  // GeoJSON Feature[]
```

---

## Type Generation from Rust

Types are **auto-generated** from Rust source of truth using `ts-rs`:

```rust
// In geoquery-types/src/query.rs
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../packages/client/src/types/")]
pub struct GeoQuery {
    pub semantic: Option<String>,
    pub spatial: Option<SpatialPredicate>,
    pub temporal: Option<TemporalPredicate>,
    // ...
}
```

Then `cargo xtask codegen` regenerates the TS types. **Zero drift**
between Rust and TypeScript.

→ See [infrastructure/xtask](../infrastructure/xtask.md) for the codegen pipeline

---

## Edge Deployment

The TS SDK works naturally in edge environments:

```typescript
// Cloudflare Worker
export default {
  async fetch(request: Request) {
    const geo = new Geoquery({ url: "https://geoquery.example.com" });
    const results = await geo.query({ ... });
    return Response.json(results);
  }
};
```

Future WASM mode would allow the planner to run entirely in the
edge runtime without a backend server.

---

## Related Files

- [interfaces/http](http.md) — The HTTP API this SDK wraps
- [query/query-model](../query/query-model.md) — The query AST (type-generated)
- [project/data-model](../project/data-model.md) — GeoResult types
- [infrastructure/xtask](../infrastructure/xtask.md) — `ts-rs` codegen pipeline
- [research/typescript-sdks](../research/typescript-sdks.md) — Ecosystem findings (Honua, MCP v2)
