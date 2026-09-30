---
type: Infrastructure Proposal
title: "Monorepo Refactor — Four Owners, One Task Graph"
description: "Staged plan for making the repository easier to manage: tool ownership boundaries, Turbo façades for cross-language edges, and what must not move."
tags: [monorepo, turbo, pixi, cargo, bun, pyo3, napi, task-graph, packaging]
status: draft
generated: { by: agent/arena, at: 2026-09-30T00:00:00Z }
created: 2026-09-30T00:00:00Z
updated: 2026-09-30T00:00:00Z
id: infrastructure/monorepo-refactor
category: infrastructure
refs: [infrastructure/monorepo, infrastructure/ci, infrastructure/xtask, interfaces/python, interfaces/typescript]
---

# Monorepo Refactor — Four Owners, One Task Graph

A proposal, not the build. It measures this repository against the four-owner
architecture — Cargo owns Rust dependencies, Bun owns JS/TS dependencies, Pixi owns
Python environments and Conda packaging, Turbo owns cross-package ordering and caching —
and says which parts are worth adopting, in what order, and which are already decided
against by a constraint this repository cannot remove.

## 1. Where the repository already agrees

| Rule | State |
| --- | --- |
| Cargo owns Rust dependencies | Yes. `[workspace.dependencies]` and `[workspace.package]` inheritance; `version.workspace = true` is enforced by `pixi run version-check`. |
| Bun owns JS/TS dependencies | Yes. One root `bun.lock`, `workspaces: ["packages/*", "apps/*"]`, no per-package lockfile. |
| Pixi owns Python and Conda | Yes. The interpreter and the whole SDK toolchain are in `pixi.lock`; both Conda packages are built by `pixi publish`. |
| The core knows nothing about its bindings | Yes, by policy. `geoquery-core` is the language; the CLI is a presentation layer, and a rule in the binary is a rule the service reimplements. |
| Deployable apps separate from libraries | Yes, since `apps/docs`. |

## 2. The one real gap: two task graphs that cannot see each other

Pixi's `depends-on` graph and Turbo's `dependsOn` graph both exist, and neither knows
about the other. Turbo sees only `packages/client` and `apps/docs`; every Rust and Python
edge is expressed as a pixi `depends-on`, which has no caching, no `--filter`, and no way
to express "this TypeScript package is stale because a Rust crate changed".

Nothing is broken today because no such edge exists yet. Two arrive in Phase 1:

- **`xtask codegen`**: `crates/types` → `packages/client/src/generated`. The TypeScript
  definitions are derived from the Rust types by `ts-rs`. Today nothing would rebuild the
  client when a Rust type changes, and nothing would fail if the generated files were
  stale — the worst kind of edge, because it is invisible and wrong rather than missing.
- **The Python SDK's build**: `test-py` depends on `py-install`, which is an editable
  install. That is `maturin develop && pytest` in a different costume — the guide's §13
  anti-pattern — and it becomes load-bearing the moment the SDK has a native extension.

**The fix is the façade pattern**, and it is cheap: a `package.json` in a directory Cargo
or Pixi already owns, whose scripts shell out to the tool that owns the work. The façade
adds no dependency resolution of its own; it exists so Turbo can see an edge.

```
crates/types/package.json        @geoquery/codegen   build = pixi run codegen
packages/client/package.json     @geoquery/client    depends on @geoquery/codegen
python/geoquery/package.json     @geoquery/python    build = pixi run py-install
```

```
geoquery-types (cargo)
      │  xtask codegen
      ▼
@geoquery/codegen#build
      ▼
@geoquery/client#build ──► @geoquery/client#test
```

Rules that keep the façade from becoming a second dependency system:

1. A façade script is always `pixi run <task>`, never a bare `cargo`/`maturin`/`pytest`.
   The repository has one entry point and the hooks call it.
2. A façade's `dependencies` name only the cross-language edges Turbo must order. The
   Cargo graph is not duplicated into `package.json` — `geoquery-core` stays a path
   dependency of `geoquery-types` and Turbo never hears about it.
3. Only components that participate in a cross-language edge get a façade. Purely
   internal crates stay Cargo-only. `crates/*` does **not** go in the Bun workspace globs.
4. `outputs` are artefacts, never `target/**`. Turbo would tar a multi-gigabyte
   incremental build directory that Cargo already caches better; the outputs worth
   declaring are `packages/client/src/generated/**`, `dist/**`, `*.whl`, `*.node`.

Then `gates` shrinks from twelve hand-ordered pixi tasks to one Turbo invocation plus the
checks that are genuinely repository-global (`deny`, `version-check`, `lint-actions`,
`publish-plan`, `lint-sandbox-plan`), and `--filter` becomes available to CI.

## 3. What must not move, and why

- **The root `Cargo.toml` stays a package.** `pixi-build-rust` runs `cargo install
  --path <source>` and installs the source directory in isolation, so the build directory
  has to be a package that is also the root of its workspace. It is also what lets
  `pixi-sandbox` vendor the crates for the offline transport. Moving the CLI back to
  `crates/cli/Cargo.toml` breaks the only way the CLI ships *and* silently disables
  vendoring. Re-test when `pixi-build` leaves preview; until then this is settled.
- **Pixi stays the entry point.** The guide's §14 agrees: `pixi run build` → `turbo run
  build`. What changes is where the *ordering* lives, not what a developer types. A
  README that says `bunx turbo run ...` is a second entry point.
- **No `maturin`, no PyO3, no `napi` in the default environment before something uses
  them.** Every dependency in `pixi.lock` is paid for on every sandbox restore, and the
  transport is already ~1 GB. Adopt the *shape* now, the dependencies when a crate needs
  them.

## 4. Staged plan

### Stage 0 — cheap, no behaviour change

- `members = ["crates/*"]` with `exclude = ["crates/cli"]` in the root `Cargo.toml`.
  Verified against this workspace: it resolves to exactly the same ten packages as the
  hand-written list. Adding a crate becomes one edit instead of two, and the glob trap the
  current comment documents is closed by the `exclude` rather than by a list that grows.
- Update `infrastructure/monorepo.md`: it still describes a virtual workspace root, an
  `xtask/` at the repository root and a `docs/` directory, none of which are true.

### Stage 1 — the façades, with Phase 1

> **Implemented 2026-09-30 (with one deliberate divergence).** The façades landed ahead of
> `xtask codegen` at the maintainer's request, and one node per *language workspace* rather
> than per cross-language edge: `crates/package.json` (`@geoquery/rust`, `--workspace`),
> `python/geoquery/package.json` (`@geoquery/python`) and the existing `packages/*`/`apps/*`.
> The root `pixi.toml` now exposes only the repo-wide turbo verbs plus the repository-global
> tasks. **The divergence is rule 1 below:** the façade scripts run `cargo`/`pytest`/`ruff`
> *directly* instead of `pixi run <task>`. This was an explicit decision and it is safe here
> because the whole graph still launches from `pixi run` (`pixi run test` → `bun run all:test`
> → `turbo run test` → the façade scripts), so the pinned tools are on PATH exactly as they
> would be inside a pixi task. Rules 2–4 are kept as written; biome stays one repo-wide
> `//#lint`/`//#fmt` turbo root task rather than a per-package script. When `xtask codegen`
> lands it slots in as the `@geoquery/codegen` edge the diagram in §2 already shows.

Land them *with* `xtask codegen`, not before: a façade whose build script generates
nothing is a moving part with no job.

- `crates/types/package.json` → `@geoquery/codegen`, `build = pixi run codegen`,
  outputs `../../packages/client/src/generated/**`.
- `packages/client` depends on `@geoquery/codegen`; its `test` gains `dependsOn: ["build"]`.
- `python/geoquery/package.json` → `@geoquery/python`, `build = pixi run py-install`,
  `test = pixi run test-py`. The editable install stops being a hidden prerequisite of the
  test task and becomes an edge Turbo orders and caches.
- Collapse the pixi aggregators onto Turbo: `gates = turbo run lint test typecheck` plus
  the global-only checks.

### Stage 2 — when a native binding is actually needed (Phase 6)

The guide's layout, adopted verbatim, because the alternative is discovering it later:

```
crates/core/          the language; knows nothing about Python or Node
crates/pyo3/          cdylib, `_native`, depends on core          + package.json façade
crates/napi/          cdylib, node addon, depends on core         + package.json façade
python/geoquery/      the public Python API; `_native` is an implementation detail
packages/client/      the public TypeScript API
```

Two things the current repository already gets right and must keep: the binding crates
depend on the core, never on each other, and the public wrappers expose their own API
rather than re-exporting the generated surface.

### Stage 3 — only if `pixi-build` allows it

Move the CLI package to `crates/cli/Cargo.toml` and make the root a virtual workspace
again. This is the change that removes the `exclude`, the manifest-less directory and the
root `[[bin]]`/`[[test]]` paths in one go — and it is blocked, not deferred.

## 5. Naming

`apps/`, `packages/`, `crates/` follow the guide; `python/` is the odd one. Renaming it to
`py-packages/` buys a uniform glob (`py-packages/*`) and costs edits in `pyproject.toml`,
both `pixi.toml` files, the ruff and pytest paths, `scripts/version.ts` and the coverage
output path. **Not worth it for one distribution.** Do it in the same commit that adds a
second Python package, and not before.

## Divergence from the source guide

- The guide lists `"target/**"` in Turbo `outputs`. Do not. Turbo would cache a directory
  Cargo already caches incrementally, and it is the largest directory in the repository.
- The guide puts `crates/*` in the Bun workspace globs. Here that would make Bun scan nine
  directories to find two façades; name the façade directories explicitly instead.
- The guide's `pixi.toml` is one flat root manifest. This repository's root manifest is
  also a Conda package recipe, which is a constraint the guide does not model — see §3.
