---
type: Interface Specification
title: CLI Interface
description: "geoquery add, describe, query, generate, check — command-line UX."
tags: [CLI, clap, command-line, UX, add, query, sources, describe, generate, check, codegen]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: interfaces/cli
category: interfaces
refs: [query/query-model, project/architecture, adapters/adapter-architecture, codegen/service-snapshot, codegen/client-generation, infrastructure/monorepo]
---

# CLI Interface

## Crate

`geoquery` — binary crate using `clap` for argument parsing.

The CLI is the primary developer-facing interface and the first
interface built in the MVP.

---

## Commands

### `geoquery add` — Register Resources

```bash
geoquery add https://example.org/stac
geoquery add https://example.org/ogc
geoquery add ./resource.yaml
geoquery add ./catalog/
geoquery add https://github.com/org/geo-catalog
```

Behavior:
1. Detects source type via adapter `detect()` methods
2. Discovers capabilities via `describe()`
3. Registers resource + service in local registry
4. Reports what was found

Output:
```
✓ Detected STAC API at https://example.org/stac
  Collections: sentinel-2-l2a, landsat-c2-l2 (2 found)
  Capabilities: spatial [intersects, bbox], temporal, CQL2 filter
  Registered as: example-stac
```

---

### `geoquery query` — Execute Queries

```bash
# Structured flags
geoquery query \
  --bbox 14.1,49.0,24.2,54.8 \
  --time 2024-06-01/2024-08-31 \
  --filter "eo:cloud_cover < 10" \
  --collection sentinel-2-l2a \
  --limit 5

# Semantic shorthand
geoquery query \
  --semantic "flood risk" \
  --near "Vistula" \
  --within 50km \
  --time 2020:2025

# From JSON file
geoquery query ./query.json

# Output format
geoquery query --bbox ... --format geojson > results.geojson
geoquery query --bbox ... --format json    # default, full GeoResult
geoquery query --bbox ... --format table   # human-readable table
```

Output (JSON, default):
```json
{
  "results": [...],
  "sources": [
    { "id": "planetary-computer", "status": "success", "count": 5, "duration_ms": 234 }
  ],
  "status": "complete",
  "total": 5,
  "deduplicated": 0
}
```

---

### `geoquery describe` — Snapshot a Service

```bash
geoquery describe example-geoserver              # print the current snapshot
geoquery describe example-geoserver --refresh    # refetch and rewrite it
geoquery describe https://example.org/stac --out ./sources/example.json
geoquery describe --all --refresh
```

Runs the adapter's `describe()` and persists the result as a deterministic
service snapshot under `~/.geoquery/sources/`, recording its hash in
`geoquery.lock`. Without `--refresh` the command is offline and simply reads
what is on disk; with `--refresh` it refetches conditionally (`ETag` /
`If-None-Match`), so an unchanged service costs one `304`.

The snapshot is the input to the planner when offline, and the compile input for
`geoquery generate`.
→ See [codegen/service-snapshot](../codegen/service-snapshot.md)

Output:
```
✓ example-geoserver  OGC API Features  sha256:4f1c…  fetched 2026-09-30T09:12:44Z
  Collections: roads, buildings, parcels (3)
  CRS: EPSG:4326 (lat/lon), EPSG:3857
  Capabilities: spatial [intersects, bbox], temporal, CQL2 filter, sorting
```

---

### `geoquery diff` / `geoquery check` — Detect Service Drift

```bash
geoquery diff example-geoserver              # stored snapshot vs live service
geoquery diff example-geoserver --format json
geoquery check                               # every source; CI gate
geoquery check --breaking-only               # fail only on removed capabilities
```

`check` exits `0` fresh, `1` drifted, `2` unreachable:

```text
example-geoserver has drifted.

  + collection: buildings_2026
  ~ roads
      + crs EPSG:3035
      - format image/gif
      - spatial op: dwithin        ← plans that pushed dwithin now degrade
  - collection: parcels_2019

Run `geoquery describe example-geoserver --refresh`.
```

Removed capabilities and removed collections are classified *breaking*, because
a plan that used to push down now degrades or fails. Additions are *additive*.
This extends the planner's "degradation is never silent" rule from query time
back to describe time.

---

### `geoquery generate` — Generate a Typed Client

```bash
geoquery generate example-geoserver --target typescript --out ./src/generated
geoquery generate example-geoserver --target python --out ./gen --collection roads,buildings
geoquery generate --snapshot ./sources/example.json --target typescript --out ./gen
geoquery generate example-geoserver --target typescript --out ./gen --check
geoquery targets                              # list available backends
```

Compiles a service snapshot into a typed, dependency-light client for one
service. **Never touches the network** — if no snapshot exists the command fails
and points at `geoquery describe`. Output is deterministic and meant to be
committed; `--check` exits `1` when regenerating would change a file, which is
the CI gate for a stale checked-in client.

```
$ geoquery targets
TARGET       STATUS     OUTPUT
────────────────────────────────────────────────────────
typescript   stable     ESM + .d.ts, fetch-based, no deps
python       stable     typed functions, httpx, py.typed
rust         preview    typed façade over geoquery-core
openapi      stable     OpenAPI 3.1 document
docs         stable     Markdown service reference
```

→ See [codegen/client-generation](../codegen/client-generation.md)

---

### `geoquery sources` — List Registered Sources

```bash
geoquery sources
```

Output:
```
NAME                  PROTOCOL             STATUS    COLLECTIONS
──────────────────────────────────────────────────────────────────
planetary-computer    STAC                 ✓         42
earth-search          STAC                 ✓         15
copernicus            OGC API Records      ✓         8
example-geoserver     OGC API Features     ✓         3
company-postgis       PostGIS              ✓         12
local-flood-context   Geoquery Resource    ✓         1
```

---

### `geoquery explore` — Launch TUI

```bash
geoquery explore
geoquery explore --bbox 14.1,49.0,24.2,54.8
```

Launches the interactive TUI (Phase 2).
→ See [interfaces/tui](tui.md)

---

### `geoquery validate` — Validate Resources

```bash
geoquery validate resource.yaml
geoquery validate ./catalog/
```

Checks:
- YAML schema validity
- Referenced services are reachable
- Spatial/temporal extents are well-formed
- Links resolve

---

### `geoquery cache` — Cache Remote Metadata

```bash
geoquery cache https://planetarycomputer.microsoft.com/api/stac/v1
geoquery cache --all
```

Pulls remote metadata into local GeoParquet for faster subsequent
queries. Phase 6 feature.
→ See [infrastructure/storage](../infrastructure/storage.md)

---

## Global Flags

```
--registry <path>     Registry location (default: ~/.geoquery/)
--format <fmt>        Output: json | geojson | table | csv
--verbose / -v        Verbose logging
--quiet / -q          Suppress non-result output
--timeout <ms>        Per-source timeout (default: 5000)
--offline             Never touch the network; use snapshots only
--max-sources <n>     Limit federation sources (default: 20)
```

---

## Implementation

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "geoquery", about = "Universal geospatial query engine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value = "~/.geoquery")]
    registry: PathBuf,

    #[arg(long, default_value = "json")]
    format: OutputFormat,
}

#[derive(Subcommand)]
enum Commands {
    Add { source: String },
    Query { /* ... */ },
    Sources,
    Explore { /* ... */ },
    Validate { path: PathBuf },
    Cache { source: Option<String> },
    Describe { source: String, refresh: bool, all: bool, out: Option<PathBuf> },
    Diff { source: String },
    Check { breaking_only: bool },
    Generate {
        source: Option<String>,
        snapshot: Option<PathBuf>,
        target: String,          // resolved against the backend registry
        out: PathBuf,
        collection: Vec<String>,
        check: bool,
    },
    Targets,
}
```

`Diff`, `Check`, `Generate` and `Targets` live behind the `codegen` cargo
feature, so a minimal engine build omits them. `Describe` is always present —
snapshots are part of the engine, not of generation.

---

## Related Files

- [query/query-model](../query/query-model.md) — The query AST the CLI constructs
- [adapters/adapter-architecture](../adapters/adapter-architecture.md) — How `add` triggers detection
- [codegen/service-snapshot](../codegen/service-snapshot.md) — What `describe`, `diff` and `check` operate on
- [codegen/client-generation](../codegen/client-generation.md) — What `generate` and `targets` expose
- [interfaces/tui](tui.md) — The `explore` subcommand target
- [infrastructure/monorepo](../infrastructure/monorepo.md) — Crate location in workspace
