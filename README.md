# geoquery

**One protocol-independent query language and execution engine for federating
geospatial resources and services.**

It sits one level above STAC, OGC API, WFS, ArcGIS, CMR and the rest: discover what a
service can do, translate one query into source-native queries, run them, and return one
result set with provenance.

> **Status: Phase 0.** The environment, the repository layout, the query document reader
> and the publishable packages are real and gated. The query engine is not built yet, so
> `geoquery query` refuses with exit code 3 rather than pretending. See [Status](#status)
> and `.knowledge/roadmap/roadmap.md` for what is next.

## Install

The packages are built by `pixi publish` and are not on a public channel yet. From a
checkout:

```console
$ pixi run publish-dist          # builds dist/*.conda
$ pixi shell                     # or: pixi run -e default <cmd>
$ pixi run publish-plan          # what a release would contain
```

A user with a channel would write `pixi global install geoquery-cli`. The Python SDK is
`geoquery`; the TypeScript client is `@geoquery/client`.

## The CLI

```console
$ geoquery --version
geoquery 0.1.0

$ echo '{"limit": 25, "execution": {"max_concurrency": 4}}' > q.json
$ geoquery check q.json
q.json is a query object with 2 keys: execution, limit

$ geoquery query --service https://example.org --query q.json
geoquery: no engine yet — https://example.org was not contacted. geoquery 0.1.0 reads
query documents; executing them arrives with the query engine.
$ echo $?
3
```

`check` reads a document and reports its shape without contacting anything, so a
malformed query can be caught before a service is involved. Exit codes are distinct on
purpose: `1` the file could not be read, `2` the file is not a query document, `3` there
is no engine to ask.

## Repository layout

| Path | What it is |
| --- | --- |
| `pixi.toml` | **the environment and the task runner.** One lockfile for Rust, JS, Python and the repo utilities |
| `Cargo.toml` | the Cargo workspace root, and the `geoquery-cli` package `pixi publish` builds |
| `crates/core/` | `geoquery-core`: the query language — documents, versions, error classification |
| `crates/cli/src/main.rs` | the `geoquery` binary: argument parsing, messages, exit codes |
| `deny.toml` | the dependency policy: bans, licences, sources || `packages/client/` | `@geoquery/client`, the TypeScript client |
| `python/geoquery/` | the `geoquery` Python SDK and its conda package |
| `apps/docs/` | the documentation site: Astro + Starlight, deployed to GitHub Pages by `docs.yml` |
| `scripts/version.ts` | the version checker, and the one place a version lives |
| `.devcontainer/` | a container that is nothing but `pixi install` |
| `.github/workflows/` | CI, and the sandbox transport publisher |
| `.knowledge/` | the design corpus: architecture, query model, interfaces, roadmap |

`pixi.toml` is both the environment and the `geoquery-cli` conda package, and `Cargo.toml` is
both the workspace root and that package. Neither is a coincidence, and both follow from one
constraint: `pixi-build-rust` installs with `cargo install --path`, which cannot select a
member out of a *virtual* manifest, so the directory a package is built from has to be a
package that is also the root of the workspace containing it. Keeping the manifests at the
repository root is what lets `crates/` hold nothing but crates, and it is what lets the
offline sandbox vendor the whole dependency graph — see the comment at the top of
`Cargo.toml`.

`crates/cli/` has no manifest, because its sources are the root package's binary, reached
through `[[bin]] path`. Every other directory under `crates/` is a real crate.

Every crate and every package keeps its tests in a `tests/` (`test/` for the TypeScript
client) directory beside `src/`, with one test file per source file: `src/document.rs` is
tested by `tests/document.rs`, and a source file with no test file beside it is a visible
gap rather than a question. `crates/cli/tests/main.rs` is the one that needs a line of
configuration — `[[test]]` in the root `Cargo.toml` — because the binary's manifest is not
in its own directory.

## Working on it

```console
$ pixi install            # restore the locked environment
$ pixi run setup          # bun workspace, editable Python SDK, git hooks
$ pixi run gates          # everything CI runs
$ pixi run fmt            # rewrite what can be rewritten
```

`pixi run` lists every task. The package-specific work — build, test, lint, typecheck,
coverage, format — lives in each package's own manifest now (`crates/package.json`,
`python/geoquery/package.json`, `packages/*` and `apps/*`) and is fanned out by turbo, so
one repo-wide verb runs every language at once. The tasks that matter most:

| Task | What it does |
| --- | --- |
| `build` / `test` | build, or run the tests for, **every** package (turbo: Rust + Python + TS) |
| `lint` / `typecheck` / `fmt` | lint, type-check, or format every package (clippy + cargo-deny + ruff + biome) |
| `cov` | coverage for every package that reports it (lcov, coverage.py, bun) |
| `gates` | `lint`, `typecheck`, `test`, `version-check`, `lint-actions`, and the publish dry-run |
| `ci` | `gates` plus every built artefact, including both conda packages |
| `docs-dev` / `docs-build` | serve the documentation site / build it into `apps/docs/dist` |
| `advisories` | check the Cargo graph against the RustSec advisory database (network) |
| `version-check` | assert every manifest that carries a version agrees on one number |
| `publish-plan` / `publish-dist` | resolve the publish set / build the `.conda` files |

Every command is `pixi run <task>`, in a shell, a git hook and a CI step alike: pixi is the
one entry point, and it delegates the fan-out to turbo. To scope a run to one package, pass
a turbo filter through — `pixi run test -- --filter=@geoquery/rust`. The task list is the
source of truth; the workflows only decide when to run it.

## Versioning

`pixi.toml`'s `[workspace].version` is the number. `pixi run version-check` compares the
two Cargo manifests, both `[package]` tables and the npm and Python manifests against it,
and it is in `gates` — a release whose package says `0.1.0` and whose binary says `0.1.1`
is a bug nobody reports.

## Offline development

The full environment can be published to an orphan git branch and restored on a machine
with no network. The plan is reviewed in-repo (`.pixi-sandbox.toml`), validated by
`pixi run lint-sandbox-plan`, packed and pushed by `publish-sandbox.yml`, and restored
with:

```console
$ sh scripts/restore.sh
```

## Status

| Phase | What it means | Where it is |
| --- | --- | --- |
| 0 — foundation | environment, layout, document reader, publishable packages | **done** |
| 1 — query language | the AST, filters, CQL2, code generation | next |
| 2 — federation | STAC and OGC adapters, the planner, the engine | |
| 4 — MCP | tools for agents | |

The full plan is in `.knowledge/roadmap/roadmap.md`, and `.knowledge/CONTEXT.md` is a
single-file briefing for anyone — human or agent — new to the project.

## Licence

MIT OR Apache-2.0. See `LICENSE-MIT` and `LICENSE-APACHE`.
