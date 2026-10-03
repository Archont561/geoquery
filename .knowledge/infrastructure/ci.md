---
type: Infrastructure Specification
title: "CI Pipeline — GitHub Actions"
description: "GitHub Actions, cargo-deny, cargo-nextest, caching."
tags: [CI, GitHub-Actions, nextest, deny, clippy, caching, rust-cache]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: infrastructure/ci
category: infrastructure
refs: [infrastructure/monorepo, infrastructure/xtask]
---

# CI Pipeline — GitHub Actions

## Workflow: `.github/workflows/ci.yml`

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  check:
    name: Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo check --workspace --all-targets

  fmt:
    name: Format
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all -- --check

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --workspace --all-targets -- -D warnings

  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@nextest
      - run: cargo nextest run --workspace

  deny:
    name: Deny
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2
```

---

## `deny.toml`

```toml
[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
ignore = []

[licenses]
allow = [
    "MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause",
    "ISC", "Zlib", "MPL-2.0", "Unicode-3.0",
]
confidence-threshold = 0.93

[bans]
multiple-versions = "warn"
wildcards = "deny"
highlight = "all"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
```

---

## Key Actions

| Action | Purpose |
|--------|---------|
| `dtolnay/rust-toolchain@stable` | Install Rust toolchain with components |
| `Swatinem/rust-cache@v2` | Cache `~/.cargo/registry` + `target/` |
| `taiki-e/install-action@nextest` | Install `cargo-nextest` binary |
| `EmbarkStudios/cargo-deny-action@v2` | Run `cargo deny check` |

---

## Environment Variables

| Variable | Value | Purpose |
|----------|-------|---------|
| `CARGO_TERM_COLOR` | `always` | Colored output in CI logs |
| `RUSTFLAGS` | `-D warnings` | Treat all warnings as errors |

---

## Future CI Jobs

| Job | Phase | Trigger |
|-----|-------|---------|
| **WASM build** | 6 | `wasm-pack build` for `geoquery-wasm` |
| **Cross-compile** | 6 | `cross build --target aarch64-unknown-linux-gnu` |
| **MSRV check** | 3 | Verify `rust-version = "1.85"` compiles |
| **TypeScript tests** | 3 | `cd packages/geoquery && npm test` |
| **Python tests** | 3 | `cd python/geoquery && pytest` |
| **Docker build** | 5 | `docker build -t geoquery .` |
| **Benchmarks** | 4 | `cargo bench --workspace` |

---

## Local CI Reproduction

```bash
cargo xtask ci
```

Runs the exact same checks as CI. No "works on my machine" surprises.

→ See [infrastructure/xtask](xtask.md) for the `ci` command implementation

---

## Related Files

- [infrastructure/monorepo](monorepo.md) — Workspace and tooling config
- [infrastructure/xtask](xtask.md) — Local CI reproduction
