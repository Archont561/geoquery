---
type: Infrastructure Specification
title: "CI Pipeline — GitHub Actions"
description: "GitHub Actions, cargo-deny, cargo-nextest, caching."
tags: [CI, GitHub-Actions, nextest, deny, clippy, caching, rust-cache]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-10-04T00:00:00Z
id: infrastructure/ci
category: infrastructure
refs: [infrastructure/monorepo, infrastructure/xtask]
---

# CI Pipeline — GitHub Actions

## Divergence from the build

This document is the design. The pipeline below was never built: the repository runs a
pixi + turbo pipeline across three languages, not five cargo jobs. The differences are
decisions rather than drift, recorded here because a reader who follows the YAML below
would be reading a workflow this repository does not have:

- **Every check is a pixi task, and the workflow runs one of them.** The `checks` job is
  `pixi run ci-checks` — `gates` plus instrumented coverage — and nothing else. The list
  of checks lives in `pixi.toml` and nowhere else, so a check added to `gates` is a check
  CI runs. Listing the steps in the workflow would be a second list, and the way a second
  list fails is that a new check passes locally and never runs in CI, which looks exactly
  like a green build.
- **Three jobs: `checks`, `packages`, `sandbox plan`** — not `check`, `fmt`,
  `clippy`, `test`, `deny`. Formatting, clippy and cargo-deny are inside `pixi run lint`,
  which also runs Biome over the JavaScript and Ruff over the Python; `pixi run test`
  likewise fans out to nextest, bun and pytest through turbo. The split is by *what a
  failure means*, not by which tool reports it.
- **No `dtolnay/rust-toolchain`, `Swatinem/rust-cache`, `taiki-e/install-action` or
  `cargo-deny-action`.** The whole toolchain — rust, bun, python, nextest, cargo-deny,
  cargo-llvm-cov, actionlint — comes from `pixi.lock` through `setup-pixi`, so CI and a
  developer's machine resolve the same versions from the same file. Caching is
  `actions/cache` keyed on `Cargo.lock`, plus turbo's own `.turbo`. The Key Actions table
  below describes the design, not the workflow.
- **No `CARGO_TERM_COLOR` or `RUSTFLAGS` environment variables.** `-D warnings` is
  `[workspace.lints]` in `Cargo.toml` and the `-D warnings` in the lint task, not an
  environment variable set only in CI — a lint policy that lives in the workflow is a
  policy that does not apply to the machine the code was written on.
- **Coverage and Codecov are in the pipeline**, which the design did not anticipate:
  `ci-checks` reruns the suites under instrumentation and the job uploads the Rust,
  Python and TypeScript reports together.
- **`rust-version = "1.88"` is asserted and not compiled, and it is wrong.** The pixi
  environment pins rust 1.98.1, so every job builds on a toolchain ten releases newer
  than the declared minimum. A job that installed 1.88 with rustup and ran
  `cargo check --workspace --all-targets --locked` was added, run once in CI, and
  removed: it failed, which means the workspace does not in fact build on the version its
  manifest promises. Under resolver 3 that number also participates in dependency version
  selection, so it is a claim about the resolved graph and not only about the source.
  Verifying it is tracked as a future job below; the fix is to find the real minimum and
  declare that, not to delete the claim.
- **Local reproduction is `pixi run ci`, not `cargo xtask ci`.** xtask is not the task
  runner here; see [infrastructure/xtask](xtask.md).

---

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

Each row names the phase that owns the job and the event that introduces it — the thing
that has to become true before the job is worth adding, rather than the command it will
eventually run. A job whose trigger has already happened belongs in the workflow, not
here.

| Job | Phase | Trigger |
|-----|-------|---------|
| **MSRV check** | 1 | `rust-version` is corrected to a version the workspace actually builds on |
| **Benchmarks** | 4 | the engine has an implementation whose performance can regress |
| **Docker build** | 5 | a container image becomes something the project ships |
| **WASM build** | 6 | the first crate targeting `wasm32-unknown-unknown` lands |
| **Cross-compile** | 6 | the release workflow starts shipping a non-`x86_64` binary |

Two rows left this table by being built: **TypeScript tests** and **Python tests** run
inside `pixi run test`, which fans out to every language through turbo rather than giving
each one a job.

---

## Local CI Reproduction

```bash
pixi run ci          # everything CI runs that a second toolchain is not needed for
pixi run ci-checks   # just the checks job: the commit gate plus coverage
pixi run gates       # the commit gate alone, which is what the hooks run
```

These are the same task definitions the workflow invokes, not a reimplementation of them,
so "works on my machine" would have to mean pixi resolved a different environment — which
`pixi install --locked` in CI is there to prevent.

There is no CI check these cannot reproduce: the workflow calls these same task
definitions rather than restating the steps, so the job list and the task list cannot
disagree.

---

## Related Files

- [infrastructure/monorepo](monorepo.md) — Workspace and tooling config
- [infrastructure/xtask](xtask.md) — Local CI reproduction
