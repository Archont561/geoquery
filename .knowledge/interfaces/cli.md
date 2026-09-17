---
type: Interface Specification
title: CLI Interface
description: "geoquery add, query, sources, explore — command-line UX."
tags: [CLI, clap, command-line, UX, add, query, sources]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: interfaces/cli
category: interfaces
refs: [query/query-model, project/architecture, adapters/adapter-architecture, infrastructure/monorepo]
---

# CLI Interface

## Crate

`geoquery-cli` — binary crate using `clap` for argument parsing.

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
}
```

---

## Related Files

- [query/query-model](../query/query-model.md) — The query AST the CLI constructs
- [adapters/adapter-architecture](../adapters/adapter-architecture.md) — How `add` triggers detection
- [interfaces/tui](tui.md) — The `explore` subcommand target
- [infrastructure/monorepo](../infrastructure/monorepo.md) — Crate location in workspace
