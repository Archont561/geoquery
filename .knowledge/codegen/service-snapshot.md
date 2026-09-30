---
type: Specification
title: "Service Snapshots — Describe Once, Commit, Diff"
description: "The serialized ServiceDescriptor snapshot, its lockfile, deterministic output rules, and drift detection."
tags: [snapshot, lockfile, descriptor, determinism, drift, capabilities, offline, ci]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2026-09-30T00:00:00Z }
created: 2026-09-30T00:00:00Z
updated: 2026-09-30T00:00:00Z
id: codegen/service-snapshot
category: codegen
refs: [project/data-model, adapters/adapter-architecture, codegen/client-generation, interfaces/cli, query/planner]
---

# Service Snapshots — Describe Once, Commit, Diff

## Why a Snapshot Exists

`ServiceAdapter::describe()` already produces a `ServiceDescriptor`: what the
source *is*, what it holds, and what it can do. Today that value lives for the
duration of a process. Persisting it makes four things possible that are
otherwise out of reach:

1. **Offline planning.** The planner can build and explain a plan with no
   network access, because capabilities are on disk.
2. **Reproducible CI.** The same snapshot yields the same plan and the same
   generated client on every machine, forever.
3. **Drift detection.** A remote service that silently drops a CRS, a
   collection, or a filter operation changes plans and degrades results
   invisibly. A committed snapshot makes that change a reviewable diff.
4. **Client generation.** A generator needs a complete, stable description of a
   service. The snapshot *is* that input.
   → See [codegen/client-generation](client-generation.md)

**Rule:** a snapshot is produced by `describe()` and nothing else. There is no
second discovery path, and the generator never touches the network.

---

## The Artifact

```
~/.geoquery/                      # or ./geoquery/ in a project
├── geoquery.lock                 # index: source id → snapshot hash, fetched_at
└── sources/
    ├── planetary-computer.json   # one snapshot per registered source
    ├── example-geoserver.json
    └── company-postgis.json
```

Snapshot shape:

```jsonc
{
  "snapshot_version": 1,           // format version, bumped on breaking change
  "source": "example-geoserver",
  "descriptor": { /* ServiceDescriptor, exactly as the adapter produced it */ },
  "resources": [ /* ResourceDescriptor[] discovered under it */ ],
  "provenance": {
    "endpoint": "https://maps.example.org/geoserver",
    "adapter": "ogc-features",
    "adapter_version": "0.2.0",
    "fetched_at": "2026-09-30T09:12:44Z",   // provenance only, never hashed
    "etag": "\"a1b2c3\"",
    "digest": "sha256:…"                     // over the canonical body
  }
}
```

`fetched_at`, `etag` and `digest` live in `provenance` precisely so the hashed
body carries no wall-clock data.

---

## Determinism Rules

A snapshot is a build input, so it must be byte-stable:

- **Stable ordering.** Every array is sorted by a defined key (collections by
  id, CRS by code, operations by name). Maps serialize with sorted keys.
- **No timestamps in the hashed body.** Time lives only under `provenance`.
- **No volatile fields.** Session tokens, request ids, and rate-limit headers
  are dropped, never recorded.
- **Explicit unknowns.** A field the source did not advertise is `null` or
  absent — never a guessed default. Open enums keep the raw string in
  `Custom(..)`/`Unknown(..)` rather than collapsing it.
- **Same input → same output.** Re-describing an unchanged service produces an
  identical `digest`.

Conditional refetch uses HTTP `ETag`/`If-None-Match`; a `304` refreshes
`fetched_at` and leaves the body (and therefore the digest) untouched.

---

## Drift Detection

```bash
geoquery describe <source> --refresh   # refetch and rewrite the snapshot
geoquery diff <source>                 # snapshot on disk vs the live service
geoquery check                         # all sources; exit 1 if any is stale
```

`geoquery check` is the CI gate. Output:

```text
example-geoserver has drifted.

  + collection: buildings_2026
  ~ roads
      + crs EPSG:3035
      - format image/gif
      - spatial op: dwithin        ← plans that pushed dwithin now degrade
  - collection: parcels_2019

Generated clients and cached plans are stale.
Run `geoquery describe example-geoserver --refresh`.
```

Exit codes: `0` fresh · `1` drifted · `2` source unreachable.

**Severity is classified, not just reported.** A removed capability or a removed
collection is *breaking* (a plan that used to push down now degrades, or fails);
an added one is *additive*. `geoquery check --breaking-only` fails only on the
former, which is the useful setting for a scheduled CI job.

This is the same principle the planner already follows — *degradation is never
silent* — extended from query time back to describe time.

---

## What the Snapshot Must Carry

The planner needs to know whether an operation can be pushed down. A generator
needs to know what a valid call to that service actually looks like. The second
requirement is stricter, and it is what pulls GIS payload semantics into the
descriptor:

| Fact | Planner use | Generator use |
|------|-------------|---------------|
| Spatial/temporal/attribute ops | pushdown vs local refine | which query methods exist |
| Collections / layers / feature types | scope resolution | one typed accessor each |
| CRS list **and axis order** | reproject before request | typed `crs` union, correct coordinate order |
| Bounding boxes, scale ranges | candidate pruning | documented extents, validation |
| Formats and styles per layer | output negotiation | typed `format` / `style` unions |
| Dimensions (time, elevation, custom) | temporal pushdown | typed dimension parameters |
| Queryables / property schemas | filter validation | typed feature interfaces |
| Pagination style and limits | paging strategy | iterator shape |
| Auth requirements | credential resolution | constructor signature |

These extend `CapabilitySet` and `SchemaDescriptor` rather than forming a
parallel model.
→ See [project/data-model](../project/data-model.md)

**Axis order is a first-class fact, not a convention.** `EPSG:4326` is lat/lon
in some protocol versions and lon/lat in others; the snapshot records what the
service actually expects so neither the planner nor the generated client has to
guess.

---

## Lifecycle

```
geoquery add <url>
      │  detect() → describe()
      ▼
snapshot written, hash recorded in geoquery.lock
      │
      ├─► planner        reads capabilities offline
      ├─► geoquery generate   reads it as compile input
      └─► geoquery check      compares it against the live service
```

A snapshot is committable. Checking `geoquery.lock` and `sources/*.json` into a
repository pins the exact service contract a build was made against, the same
way a dependency lockfile pins code.

---

## Related Files

- [project/data-model](../project/data-model.md) — ServiceDescriptor, CapabilitySet
- [adapters/adapter-architecture](../adapters/adapter-architecture.md) — where `describe()` lives
- [codegen/client-generation](client-generation.md) — the snapshot's main consumer
- [interfaces/cli](../interfaces/cli.md) — `describe`, `diff`, `check`
- [query/planner](../query/planner.md) — capability matching and degradation
