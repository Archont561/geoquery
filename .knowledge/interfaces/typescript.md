---
type: Interface Specification
title: "TypeScript SDK — @geoquery/client"
description: "@geoquery/client, fluent builder, ts-rs codegen, WASM path."
tags: [TypeScript, SDK, client, WASM, ts-rs, codegen, fluent]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: interfaces/typescript
category: interfaces
refs: [interfaces/http, query/query-model, project/data-model, infrastructure/xtask]
---

# TypeScript SDK — `@geoquery/client`

## Package

`@geoquery/client` — TypeScript/JavaScript SDK for Geoquery.

The primary application-facing API for web and Node.js developers.

---

## Transport Options

| Option | Status | Use Case |
|--------|--------|----------|
| **NAPI** | Shipped first | In-process, via `napi-rs` |
| **HTTP Client** | Deferred | Talks to `geoquery-http` (Rust/Axum) |
| **WASM** | Phase 6 | In-browser/edge via `geoquery-wasm` |

> **Divergence (2026-10-02).** This file originally ranked the three transports HTTP (MVP),
> WASM (Phase 6), NAPI (future), on the reasoning that "HTTP works everywhere and the HTTP
> API is already in the spec". The implementation went the other way: `packages/client`
> loads a N-API addon (`crates/node-native`, crate `geoquery-node-native`) over a
> versioned JSON transport and holds no transport code of its own. Two reasons, neither of
> which was in this table:
>
> 1. **The ordering was self-defeating for this repository.** The HTTP API is not
>    implemented (`geoquery-http` is scaffolding), so an HTTP-first client would have had
>    nothing to talk to and could not have been tested against anything real. The native
>    boundary exposes the part of the engine that does exist — document parsing and protocol
>    version — on day one.
> 2. **The Python SDK had already taken this route.** `python/geoquery` crosses into Rust
>    through PyO3 rather than HTTP, so a native TypeScript boundary keeps the two peer
>    SDKs structurally identical rather than having one of them take a different path to
>    the same engine.
>
> What this costs, and it is not free: an addon is a compiled, platform-specific binary, so
> this client no longer runs in a browser, on Cloudflare Workers, or on any edge runtime
> that cannot `dlopen` a shared library — the deployment targets listed in `CONTEXT.md`. It
> also means the npm tarball is linux-64 only for now, and that `packages/client` gains a
> build step that cannot be type-checked alone. The WASM path below is what would restore
> those targets; HTTP remains the right answer for anything that is not a Node process.
>
> One constraint this exposed is worth keeping: `#[napi]` expands to unsafe code, and
> `unsafe_code = "forbid"` cannot be overridden by any inner attribute. `geoquery-node-native`
> therefore restates the workspace lints with `unsafe_code = "deny"` — see its `Cargo.toml`
> for why, and note that it is the only member of the workspace that does not inherit
> `[workspace.lints]`.
>
> **The wire is the interface, not the engine's function list.** `crates/protocol` defines
> `{transportVersion, operation, payload}` in and `{transportVersion, ok, result}` out, and
> both adapters export exactly one `invoke(String) -> String`. This is the single largest
> departure from this file, which describes a typed client whose API surface is generated
> from Rust. Generating signatures per operation would mean a new FFI signature, a new
> generated `.d.ts` and a new addon release for every engine operation, and a binding
> compiled before an operation existed would have no way to refuse one it lacks. The cost is
> that `invoke`'s payload and result are JSON rather than generated types, so the TypeScript
> mirrors of the Rust DTOs are hand-written and kept honest by round-tripping them through
> the real addon in the test suite. The Python SDK has the same shape.

```
Option A (shipped): @geoquery/client  ──FFI──▶  geoquery-node-native → geoquery-engine
Option B (deferred): @geoquery/client  ──HTTP─▶  geoquery-http (Axum)
Option C (future):   @geoquery/client  ──WASM─▶  geoquery-wasm (.wasm)

        crates/protocol  ── versioned JSON envelope, shared by both SDKs ──┐
        crates/engine    ── one dispatch arm per Operation ───────────────┤
        crates/node-native   ┐                                          │
        crates/python-native ┘── one `invoke` each, nothing else ─────────┘
```

### Package Structure (as implemented)

```
crates/protocol/             # geoquery-protocol — the envelope, no logic
crates/engine/               # geoquery-engine   — one arm per Operation
crates/node-native/          # geoquery-node-native — cdylib, `#[napi] invoke`
packages/client/
├── package.json             # napi.binaryName + napi.targets name the artifact
├── tsconfig.json
├── src/
│   ├── index.ts             # Public API — typed facade over `invoke`
│   └── native.ts            # Loading the `.node` through createRequire
├── test/
│   ├── index.test.ts
│   └── native.test.ts
└── dist/                    # tsc output; published
```

The addon lives under `crates/` with every other crate rather than inside the package that
loads it. It is a workspace member either way, but a binding crate outside `crates/` had to
be named in `members` by hand and was invisible to `scripts/version.ts`, whose sweep starts
at `crates/`.

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

> **Not implemented.** Everything below is the shape the corpus intends for the client once
> there is an engine to execute, and none of it exists yet — `geoquery query` still refuses
> rather than returning an empty result. What ships today is `VERSION`, `invoke`,
> `invokeRaw`, `ping()`, `protocolVersion()` and `parseDocument()`: the version and
> document-parsing half of the boundary. The HTTP forms shown here assume a transport that is
> deferred; under the native binding the same calls are in-process, and the builder and the
> result types arrive with the executor.

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
