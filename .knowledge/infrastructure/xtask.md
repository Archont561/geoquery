---
type: Infrastructure Specification
title: "xtask — Task Runner & Code Generation"
description: "cargo xtask setup, codegen, fixtures, schema generation."
tags: [xtask, codegen, ts-rs, schemars, fixtures, automation, xshell]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-10-04T00:00:00Z
id: infrastructure/xtask
category: infrastructure
refs: [infrastructure/monorepo, infrastructure/ci, interfaces/mcp, interfaces/typescript, project/data-model]
---

# xtask — Task Runner & Code Generation

## Divergence from the build

This document is the design. xtask is not the task runner in this repository, and most of
the commands below do not exist:

- **Pixi is the task runner.** Every command a hook, a workflow or a contributor runs is
  `pixi run <task>`, and the task list in `pixi.toml` is the only one. A second task
  runner over the same commands is a second list to keep in step, and the one that drifts
  is whichever one CI does not use.
- **`cargo xtask ci`, `fmt`, `lint` and `test` do not exist.** They are `pixi run ci`,
  `pixi run lint`, `pixi run test` and so on. The CI workflow calls `pixi run ci-checks`;
  see [infrastructure/ci](ci.md).
- **There is no `.cargo/config.toml` alias in the repository.** The one that appears on a
  restored machine is written by the offline sandbox and is ignored.
- **The crate is `crates/xtask/`, not a root `xtask/`,** because a workspace member has to
  be inside the workspace.
- **It keeps exactly one job: code generation.** That is the one thing that cannot be a
  task — deriving the TypeScript definitions and JSON schemas needs the workspace's own
  types loaded in memory, and a task runs a command rather than being the thing that
  defines it. `codegen` and `fixtures` below are the sections still worth reading.

---

## Why xtask Over Justfile

| Dimension | `xtask` | `Justfile` |
|-----------|---------|-----------|
| Prerequisites | Only `rustc` + `cargo` | `just` executable |
| Cross-platform | 100% native | Shell-dependent |
| Type safety | Full Rust compiler | Shell strings |
| Code generation | Native access to workspace types | Must shell out |
| Trivial 1-liners | ~5–10 lines boilerplate | 1 line |
| Compile overhead | Small, first run only | Instant |

**For Geoquery, `xtask` is the superior choice** because:

1. **Zero external dependencies** — anyone with `cargo` can run tasks
2. **Direct type access** — codegen can import `geoquery-types` and
   generate MCP schemas and TypeScript bindings with 100% type parity
3. **Fixture harvesting** — fetch and sanitize real STAC/OGC responses
   for integration tests using the same HTTP client as the engine

---

## Setup

### Cargo Alias: `.cargo/config.toml`

```toml
[alias]
xtask = "run --package xtask --"
```

Now `cargo xtask <command>` runs the xtask binary.

### `xtask/Cargo.toml`

```toml
[package]
name = "xtask"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
xshell = "0.2"

# Direct access to types for schema/binding generation
geoquery-types = { path = "../crates/types" }
schemars = "0.8"
serde_json = "1"
```

`xshell` (from the `rust-analyzer` author) makes shell commands in
Rust almost as concise as bash.

---

## Commands

### `cargo xtask ci`

Runs all CI checks locally:

```
==> Checking formatting...
==> Running linter...
==> Running tests...
==> Checking licenses and advisories...
✅ All CI checks passed!
```

### `cargo xtask fmt [--check]`

Format all workspace code (or check formatting).

### `cargo xtask lint`

Run Clippy with workspace warnings denied.

### `cargo xtask test`

Run tests via `cargo-nextest` (falls back to `cargo test`).

### `cargo xtask codegen`

**The killer feature.** Generates:

| Output | Source | Target |
|--------|--------|--------|
| MCP tool JSON schemas | `GeoQuery`, `GeoResult` types | `schemas/mcp/*.json` |
| TypeScript type definitions | All public types via `ts-rs` | `packages/geoquery/src/types/` |
| Resource manifest JSON Schema | `ResourceDescriptor` | `schemas/resource-manifest.json` |
| OpenAPI spec (future) | HTTP route types | `schemas/openapi.json` |

**Implementation sketch:**

```rust
use geoquery_types::query::GeoQuery;
use geoquery_types::result::GeoResult;
use schemars::schema_for;

fn generate_mcp_schemas() -> Result<()> {
    let query_schema = schema_for!(GeoQuery);
    let result_schema = schema_for!(GeoResult);

    fs::write(
        "schemas/mcp/geo_query.json",
        serde_json::to_string_pretty(&query_schema)?,
    )?;
    fs::write(
        "schemas/mcp/geo_result.json",
        serde_json::to_string_pretty(&result_schema)?,
    )?;
    Ok(())
}
```

For TypeScript, `ts-rs` annotations on the Rust types handle
generation automatically:

```rust
// In geoquery-types
#[derive(Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../packages/geoquery/src/types/")]
pub struct GeoQuery { ... }
```

`cargo xtask codegen` triggers `ts-rs` export + `schemars` generation
in one step. **Zero drift between Rust, TypeScript, and MCP schemas.**

### `cargo xtask fixtures`

Download and sanitize real-world STAC/OGC API responses for
integration tests:

```rust
fn sync_fixtures() -> Result<()> {
    let sources = [
        ("planetary-computer", "https://planetarycomputer.microsoft.com/api/stac/v1"),
        ("earth-search", "https://earth-search.aws.element84.com/v1"),
    ];
    for (name, url) in sources {
        // Fetch /collections, /search sample, /conformance
        // Sanitize (remove auth tokens, truncate large responses)
        // Write to tests/fixtures/{name}/
    }
    Ok(())
}
```

---

## Implementation

```rust
use clap::{Parser, Subcommand};
use xshell::{cmd, Shell};

#[derive(Parser)]
#[command(name = "cargo xtask")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Ci,
    Fmt { #[arg(long)] check: bool },
    Lint,
    Test,
    Codegen,
    Fixtures,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let sh = Shell::new()?;

    match cli.command {
        Commands::Fmt { check } => {
            if check {
                cmd!(sh, "cargo fmt --all -- --check").run()?;
            } else {
                cmd!(sh, "cargo fmt --all").run()?;
            }
        }
        Commands::Lint => {
            cmd!(sh, "cargo clippy --workspace --all-targets -- -D warnings").run()?;
        }
        Commands::Test => {
            if cmd!(sh, "cargo nextest --version").read().is_ok() {
                cmd!(sh, "cargo nextest run --workspace").run()?;
            } else {
                cmd!(sh, "cargo test --workspace").run()?;
            }
        }
        Commands::Ci => {
            cmd!(sh, "cargo fmt --all -- --check").run()?;
            cmd!(sh, "cargo clippy --workspace --all-targets -- -D warnings").run()?;
            cmd!(sh, "cargo test --workspace").run()?;
            cmd!(sh, "cargo deny check").run()?;
        }
        Commands::Codegen => { /* schema + TS generation */ }
        Commands::Fixtures => { /* STAC/OGC fixture sync */ }
    }
    Ok(())
}
```

---

## Related Files

- [infrastructure/monorepo](monorepo.md) — Workspace layout
- [infrastructure/ci](ci.md) — the pipeline, which calls `pixi run ci-checks`
- [interfaces/mcp](../interfaces/mcp.md) — MCP schema generation consumer
- [interfaces/typescript](../interfaces/typescript.md) — TypeScript type generation consumer
- [project/data-model](../project/data-model.md) — Types being generated from
