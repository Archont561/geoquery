---
type: Interface Specification
title: "TypeScript SDK — @archont561/geoquery"
description: "@archont561/geoquery, N-API FFI wrapper over the Rust core, ts-rs codegen, WASM path."
tags: [TypeScript, SDK, package, N-API, FFI, ts-rs, codegen]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-10-04T00:00:00Z
id: interfaces/typescript
category: interfaces
refs: [interfaces/http, query/query-model, project/data-model, infrastructure/xtask]
---

# TypeScript SDK — `@archont561/geoquery`

## Package

`@archont561/geoquery` is the TypeScript/JavaScript package for Geoquery. It is a native
package for Node.js runtimes that can load a N-API addon.

It is **not** an HTTP client. The package loads `geoquery-node-native`, which calls the
shared Rust engine in-process over the versioned JSON transport in `crates/protocol`. The
HTTP server remains a separate integration surface for remote applications and browser/edge
runtimes that cannot load a native addon.

---

## Transport Options

| Option | Status | Use Case |
|--------|--------|----------|
| **N-API FFI** | Shipped first | In-process Node.js package via `napi-rs` and the Rust core |
| **HTTP API** | Separate/deferred | Remote applications and runtimes that cannot `dlopen` a native addon |
| **WASM** | Future | In-browser/edge execution without a backend server |

> **Divergence (2026-10-02, renamed 2026-10-04).** This file originally ranked the
> transports HTTP (MVP), WASM (Phase 6), N-API (future), on the reasoning that "HTTP works
> everywhere and the HTTP API is already in the spec". The implementation went the other
> way: `packages/geoquery` publishes `@archont561/geoquery`, loads a N-API addon
> (`crates/node-native`, crate `geoquery-node-native`) over a versioned JSON transport, and
> holds no HTTP transport code of its own.
>
> 1. **The ordering was self-defeating for this repository.** The HTTP API is not
>    implemented (`geoquery-http` is scaffolding), so an HTTP-first package would have had
>    nothing to talk to and could not have been tested against anything real. The native
>    boundary exposes the part of the engine that does exist — document parsing and protocol
>    version — on day one.
> 2. **The Python SDK had already taken this route.** `python/geoquery` crosses into Rust
>    through PyO3 rather than HTTP, so a native TypeScript boundary keeps the two peer SDKs
>    structurally identical rather than having one of them take a different path to the same
>    engine.
>
> What this costs, and it is not free: an addon is a compiled, platform-specific binary, so
> this package does not run in a browser, on Cloudflare Workers, or on any edge runtime that
> cannot `dlopen` a shared library. It also means the npm tarball is linux-64 only for now,
> and that `packages/geoquery` gains a build step that cannot be type-checked alone. The WASM
> path is what would restore local browser/edge execution; the HTTP server is the right
> answer for remote/browser applications, but it is not this package's transport.
>
> One constraint this exposed is worth keeping: `#[napi]` expands to unsafe code, and
> `unsafe_code = "forbid"` cannot be overridden by any inner attribute. `geoquery-node-native`
> therefore restates the workspace lints with `unsafe_code = "deny"` — see its `Cargo.toml`
> for why, and note that it is the only member of the workspace that does not inherit
> `[workspace.lints]`.
>
> **The wire is the interface, not the engine's function list.** `crates/protocol` defines
> `{transportVersion, operation, payload}` in and `{transportVersion, ok, result}` out, and
> both native adapters export exactly one `invoke(String) -> String`. Generating signatures
> per operation would mean a new FFI signature, a new generated `.d.ts`, and a new addon
> release for every engine operation. The cost is that `invoke`'s payload and result are JSON
> rather than generated operation types, so the TypeScript mirrors of the Rust DTOs are
> hand-written and kept honest by round-tripping them through the real addon in the test
> suite. The Python SDK has the same shape.

```text
@archont561/geoquery  ──FFI──▶  geoquery-node-native → geoquery-engine
python/geoquery       ──FFI──▶  geoquery-python-native → geoquery-engine
remote applications   ─HTTP─▶  geoquery-http (separate server interface, deferred)
future browser/edge   ─WASM─▶  geoquery-wasm (.wasm)

        crates/protocol  ── versioned JSON envelope, shared by native SDKs ─┐
        crates/engine    ── one dispatch arm per Operation ────────────────┤
        crates/node-native   ┐                                           │
        crates/python-native ┘── one `invoke` each, nothing else ─────────┘
```

### Package Structure (as implemented)

```text
crates/protocol/             # geoquery-protocol — the envelope, no logic
crates/engine/               # geoquery-engine   — one arm per Operation
crates/node-native/          # geoquery-node-native — cdylib, `#[napi] invoke`
packages/geoquery/
├── package.json             # name: @archont561/geoquery; napi target names the artifact
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
  "name": "@archont561/geoquery",
  "version": "0.1.0",
  "description": "TypeScript FFI wrapper for Geoquery's Rust core",
  "type": "module",
  "main": "./dist/index.js",
  "types": "./dist/index.d.ts",
  "exports": {
    ".": {
      "default": "./dist/index.js",
      "types": "./dist/index.d.ts"
    }
  },
  "napi": {
    "binaryName": "geoquery-node-native",
    "targets": ["x86_64-unknown-linux-gnu"]
  }
}
```

---

## API Surface

Geoquery is still in Phase 0. What ships today is the native boundary and the operations the
Rust engine actually supports: `VERSION`, `TRANSPORT_VERSION`, `invoke`, `invokeRaw`,
`ping()`, `protocolVersion()`, and `parseDocument()`.

```typescript
import { parseDocument, ping, protocolVersion } from "@archont561/geoquery";

console.log(ping("typescript"));
console.log(protocolVersion());
console.log(parseDocument('{"limit": 10, "bbox": []}')); // ["bbox", "limit"]
```

The future query builder and result types arrive with the executor. They should continue to
call the Rust core in-process for this package rather than becoming an HTTP wrapper.
Applications that want a network boundary should use `geoquery-http` explicitly.

---

## Type Generation from Rust

Types are auto-generated from Rust source of truth using `ts-rs` where a stable typed shape
exists:

```rust
// In geoquery-types/src/query.rs
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../packages/geoquery/src/types/")]
pub struct GeoQuery {
    pub semantic: Option<String>,
    pub spatial: Option<SpatialPredicate>,
    pub temporal: Option<TemporalPredicate>,
}
```

Then `cargo xtask codegen` regenerates the TypeScript types. The transport envelope remains
hand-written in TypeScript because the FFI wire is JSON text and the package deliberately
exports one `invoke` boundary rather than one native signature per operation.

→ See [infrastructure/xtask](../infrastructure/xtask.md) for the codegen pipeline

---

## Browser and Edge Deployment

`@archont561/geoquery` is a native Node.js package and does not work in browsers or edge
runtimes that cannot load `.node` addons. Those targets should use one of the separate
interfaces:

1. `geoquery-http` once the HTTP server lands, for a networked engine; or
2. a future WASM build for local browser/edge execution.

---

## Related Files

- [interfaces/http](http.md) — The separate HTTP API for remote applications
- [query/query-model](../query/query-model.md) — The query AST (type-generated)
- [project/data-model](../project/data-model.md) — GeoResult types
- [infrastructure/xtask](../infrastructure/xtask.md) — `ts-rs` codegen pipeline
- [research/typescript-sdks](../research/typescript-sdks.md) — Ecosystem findings (Honua, MCP v2)
