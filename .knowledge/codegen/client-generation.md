---
type: Specification
title: "Client Generation — Typed SDKs from a Service Snapshot"
description: "geoquery generate: compiling a described service into a typed client in TypeScript, Python, Rust and beyond."
tags: [codegen, generate, typed-client, SDK, TypeScript, Python, Rust, templates, determinism]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2026-09-30T00:00:00Z }
created: 2026-09-30T00:00:00Z
updated: 2026-09-30T00:00:00Z
id: codegen/client-generation
category: codegen
refs: [codegen/service-snapshot, project/data-model, interfaces/cli, interfaces/typescript, interfaces/python, extensions/extension-points]
---

# Client Generation — Typed SDKs from a Service Snapshot

## The Capability

Geoquery describes services in order to federate them. Once a service has been
described, the same description can be **compiled into a typed client for that
one service** — a build-time complement to run-time federation.

```bash
geoquery add https://maps.example.org/geoserver
geoquery generate example-geoserver --target typescript --out ./src/generated
```

```typescript
import { Roads, Buildings } from "./generated";

const features = await Roads.query({
  bbox,
  crs: "EPSG:3857",     // ← only the CRSs this server advertises
  style: "transport",   // ← only the styles this layer has
  format: "image/png",  // ← only the formats it can return
});
```

The generated surface reflects **the actual capabilities of that particular
server**, not the union of what its protocol permits.

---

## Why This Belongs in Geoquery

Geoquery already owns the expensive half of the problem:

- adapters that speak STAC, OGC API, WFS, ArcGIS, CMR, PostGIS
- a normalized, GIS-aware `ServiceDescriptor` / `CapabilitySet`
- a persisted, deterministic snapshot of each registered source
- typed schemas already exported to TypeScript and Python for the SDKs

A generator is a **backend over an artifact that already exists**. Building it
elsewhere would mean reimplementing every adapter to get the same descriptor.

The two modes are complementary, not competing:

| | Federation (run time) | Generation (build time) |
|---|---|---|
| Question | "answer this across every source" | "give me a precise client for this one source" |
| Binds | at execution | at compile |
| Output | `QueryResult` + provenance | source files you commit |
| Strength | breadth, degradation, ranking | autocomplete, compile-time safety |
| Input | live sources + registry | the snapshot |

Federation stays the product. Generation is a capability of the same engine,
reached through the same CLI and the same descriptor.

---

## Pipeline

```
registered source
      │  describe()          (adapters — network happens here, once)
      ▼
service snapshot  ──────────► the compile input; no network beyond this point
      │
      │  lower                (snapshot → generation model: names, types, ops)
      ▼
generation model
      │
      ├─► TypeScript backend ─► .ts sources
      ├─► Python backend ─────► .py sources + type stubs
      └─► <target> backend ───► …
```

Three hard separations:

1. **Adapters never know about generation.** They produce descriptors.
2. **Backends never know about protocols.** They consume the generation model;
   a backend author writes no WFS or STAC code.
3. **Generation never fetches.** If the snapshot is missing, the command fails
   and tells you to run `geoquery describe`. Offline generation is the default,
   not a mode.

### The Generation Model

A thin lowering of the snapshot into naming- and type-oriented terms, so every
backend shares identifier sanitation, collision handling, and type mapping
instead of each reinventing them:

```rust
pub struct GenerationModel {
    pub service: ServiceIdent,        // name, version, base url, auth shape
    pub accessors: Vec<Accessor>,     // one per collection / layer / feature type
    pub types: Vec<TypeDef>,          // feature schemas, enums, parameter objects
    pub operations: Vec<OperationDef>,// query, map, tiles, item, download…
}
```

Resource kinds are an open enum (`Collection`, `FeatureType`, `MapLayer`,
`TileLayer`, `Coverage`, `Custom(String)`), so an adapter that discovers
something new degrades to a generic accessor rather than being dropped.

---

## Targets

| Target | Status | Output |
|--------|--------|--------|
| `typescript` | first target | ESM + `.d.ts`, fetch-based, zero runtime deps |
| `python` | second | typed functions, `httpx`, dataclasses/TypedDict, `py.typed` |
| `rust` | later | a typed façade over `geoquery-core` |
| `go`, `java`, `csharp`, `kotlin` | speculative | community backends |
| `openapi` | useful early | an OpenAPI document for services that lack one |
| `docs` | useful early | Markdown reference for a service |

`openapi` and `docs` are worth building early: they are cheap backends over the
same model and immediately prove the model is language-neutral.

**Generated code is dependency-light.** A generated TypeScript client depends on
nothing but the platform `fetch`; a generated Python client on `httpx` alone.
Generated code never requires Geoquery itself to be installed, and never
requires a Rust toolchain.

---

## Output Contract

- **Deterministic.** Same snapshot + same generator version → byte-identical
  output. Sorted members, no timestamps, no absolute paths, no hostnames in
  comments beyond the endpoint itself.
- **Committable and diffable.** Output is meant to live in the consumer's repo
  and be reviewed like any other source.
- **Marked.** Every file carries a header naming the source, the snapshot
  digest, and the generator version, so `geoquery check` can tell whether the
  checked-in client matches the current snapshot.
- **Never hand-edited.** Extension happens by composition around the generated
  client, not by editing it.

```bash
geoquery generate <source> --target typescript --out ./src/generated --check
# exit 1 if regenerating would change any file — the CI gate for stale clients
```

---

## Command Surface

```bash
geoquery targets                         # list available generation backends
geoquery generate <source> --target <t> --out <dir>
geoquery generate <source> --target typescript --collection roads,buildings
geoquery generate --snapshot ./sources/example.json --target python --out ./gen
geoquery generate <source> --target typescript --out ./gen --check
```

→ See [interfaces/cli](../interfaces/cli.md) for the full flag set.

---

## Extension Point: Generator Backends

A backend is a trait implementation, registered the same way adapters are:

```rust
pub trait GeneratorBackend: Send + Sync {
    fn target(&self) -> &str;                 // "typescript", "python", …
    fn generate(&self, model: &GenerationModel) -> Result<Vec<GeneratedFile>>;
}

pub struct GeneratedFile {
    pub path: PathBuf,
    pub contents: String,
}
```

Backends depend on `geoquery-types` only — never on `geoquery-core`, never on an
adapter crate. The dependency arrow stays one-way, exactly as it does for
adapters. Templates are compiled in, so a generator binary needs no template
directory at run time.

---

## Non-Goals

- **Not a replacement for the SDKs.** `@archont561/geoquery` and the `geoquery`
  Python package remain the federation interface. A generated client talks to
  one service; the SDKs talk to the engine.
- **Not a server framework.** Geoquery generates clients, not service
  implementations.
- **Not hand-maintained output.** Regeneration must always be safe.
- **Not required.** Every Geoquery user can ignore `generate` entirely; nothing
  in the engine depends on it.

---

## Crates

| Crate | Role |
|-------|------|
| `geoquery-codegen` | snapshot → generation model, backend registry, emit |
| `geoquery-codegen-typescript` | TypeScript backend |
| `geoquery-codegen-python` | Python backend |

Generation is a CLI feature flag (`--features codegen`), so a minimal engine
build does not carry it.

---

## Related Files

- [codegen/service-snapshot](service-snapshot.md) — the compile input
- [project/data-model](../project/data-model.md) — descriptors and capabilities
- [interfaces/cli](../interfaces/cli.md) — `generate`, `targets`
- [interfaces/typescript](../interfaces/typescript.md) — the federation SDK
- [extensions/extension-points](../extensions/extension-points.md) — backend registration
