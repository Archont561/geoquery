# AGENTS.md

## What this project is

One protocol-independent query language and execution engine for federating geospatial
resources and services. The API is the query AST (`.knowledge/query/query-model.md`), and
every interface — Rust, CLI, HTTP, TypeScript, Python, MCP — is an adapter to it.

The repository is at Phase 0: the environment, the layout, the query document reader and
the publishable packages are real; the engine is not. `geoquery query` refuses with exit
code 3 rather than returning empty results, and that refusal is a feature.

`.knowledge/CONTEXT.md` is a single-file briefing. Read it before proposing anything
architectural; the rest of `.knowledge/` is the design corpus and is more specific than
this file.

## The one thing that is not obvious

The repository root holds both the Cargo workspace and the `geoquery-cli` package, and
`crates/cli/` has no `Cargo.toml`. The reason is in the comment at the top of `Cargo.toml`:
`pixi-build-rust` runs `cargo install --path <source>`, and `cargo install` cannot install
from a virtual manifest and has no `-p`. So the directory a conda package is built from
must be a package that is also the root of the workspace containing it. Do not "fix" this
by adding a `crates/cli/Cargo.toml`, splitting the `[package]` out into a virtual root, or
moving it back under `crates/` — each breaks the package build, which is the only way the
CLI ships, and moving it back also silently disables crate vendoring in the sandbox.

The same reasoning is why the root `pixi.toml` carries `[package]` for the CLI rather than a
sub-manifest doing it: the package's build directory is the directory holding its manifest,
and that has to be where the Cargo manifest is.

## How to run anything

```console
pixi run gates        # everything CI runs; the gate before any commit
pixi run fmt          # rewrite what can be rewritten
pixi run ci           # gates plus every built artefact, including both conda packages
```

Never install a toolchain outside `pixi.toml`. There is no rustup, no node, no separate
venv, and no `cargo install`. If a command is not a task, add the task first — the task
list is what the git hooks and the workflows call, and a second entry point over the same
commands is a second thing to keep in step.

`bun` is the only JavaScript runtime. Reach JS tooling through `bun run` / `bun x`, never
through a `node_modules/.bin` path: those entries carry a `node` shebang, and no
environment here provides `node`. Likewise `python -m pip`, never a bare `pip`: PATH is
not trustworthy inside a task.

## Conventions that gates enforce

- Rust: `cargo fmt`, Clippy with `-D warnings` and `clippy::pedantic`, `cargo deny`, and
  `unsafe_code = "forbid"`. Lints are declared once in `Cargo.toml` under
  `[workspace.lints]`; a new crate opts in with `[lints] workspace = true` rather than
  restating them.
- Rust: a rule about what a query *is* belongs in `geoquery-core`. The binary phrases it.
  A rule in the binary is a rule the service would have to write again.
- Python: Ruff with `select = ["ALL"]`. Ignore rules go in `pyproject.toml` with a
  comment saying why, not in a per-invocation flag.
- TypeScript: Biome, 100 columns. Python is also 100 (`line-length` in
  `pyproject.toml`) so the two formatters cannot disagree.
- One version: `pixi.toml`'s `[workspace].version`. `pixi run version-check` is in
  `gates` and covers all eight published manifests. Never bump one by hand.
- Comments explain *why*, especially where a constraint is invisible from the code. Do not
  narrate what a line does. Do not add comments the code already says.
- Divergence from `.knowledge/` gets a note in the file it diverges from. The knowledge
  base is a design corpus, not the build.

## Tests

Test the behaviour, not the implementation. A test that asserts a message or an exit code
is asserting an interface; a test that asserts a private field is asserting today's
structure. Failure paths deserve the same attention as success paths — the interesting
assertion in `check_distinguishes_a_missing_file_from_a_bad_one` is that the path is
named once, because it was named twice and nobody would have noticed.

Rust tests run under `cargo nextest`, so each test needs its own files: use the
process-id-plus-name convention in `crates/cli/tests/main.rs` rather than a fixed
`tests/fixtures/` path that parallel tests would share.

Tests mirror sources. Every crate and package has a `tests/` (`test/`, for the TypeScript
client) directory beside `src/`, with one test file per source file — `src/document.rs` is
tested by `tests/document.rs` — so a source file with no test file beside it is something a
reader can see. Two consequences worth knowing before moving a test:

- They are integration tests, not `#[cfg(test)] mod tests`. A test that reaches only the
  public surface is a test of the interface; one that reaches a private item is a test of
  today's structure, and the mirror is what makes that distinction mechanical rather than a
  matter of discipline.
- `crates/cli/` is the exception that needs configuration: its manifest is the root
  `Cargo.toml`, so cargo's default `tests/` would be at the repository root. `[[test]] name
  = "cli", path = "crates/cli/tests/main.rs"` puts it back beside the sources. Those tests
  run the built binary through `CARGO_BIN_EXE_geoquery` and assert stdout, stderr and the
  exit code, because those three are what the binary actually promises.

## Documentation

`apps/docs/` is an Astro + Starlight site, a member of the same Bun workspace as
`packages/*`, built by `pixi run docs-build` and deployed to GitHub Pages by `docs.yml` on
a push to `main` that touched something the site reads. `pixi run docs-dev` serves it.

It documents what the code does. `.knowledge/` is the design corpus and stays where it is:
a design document copied into a published site is a second copy to keep true, so the site
links into the knowledge base rather than restating it.

The version the site prints comes from the environment (`GEOQUERY_VERSION`, exported by the
docs tasks) and falls back to `scripts/version.ts` — never from a number written into the
site. `apps/docs/package.json` is private and is still covered by `pixi run version-check`,
because a version a reader sees is a version that has to be right.

## Packages

Two are published, and both are built by `pixi publish`:

- `geoquery-cli` (`[package]` in the root `pixi.toml`) — the `geoquery` binary, linux-64.
- `geoquery` (`python/geoquery/pixi.toml`) — the Python SDK, `noarch: python`.

`publish-plan` is a gate because a dry run still resolves the whole publish set, so a
package whose build dependencies do not solve fails before anything is uploaded. Actual
channel upload is a release workflow job with credentials, not a task.

Two backend settings that look arbitrary and are not:

- `[package.build.config] compilers = ["c"]` in the root `pixi.toml`. The default also
  requests `rust_linux-64`, which needs the `rust` metapackage that no longer exists on
  conda-forge, and the failure is a wall of "no candidates found" rather than a message
  about the compiler.
- `preview = ["pixi-build"]` in the root `pixi.toml`. Without it, a manifest cannot carry a
  `[package]` table at all.

## Offline work

The full environment publishes to an orphan git branch and restores on a machine with no
network: `pixi run sandbox-pack` → `doctor` → `publish`, then `sh scripts/restore.sh`.
`.pixi-sandbox.toml` is a reviewed plan and `pixi run lint-sandbox-plan` gates it.
`cargo_vendor` is `true` and `sandbox-pack` passes `--cargo-vendor`: the two are one
decision, so if either is changed the other changes with it. Vendoring is possible *because*
the Cargo manifest is at the repository root, which is the same reason it is there.

## Committing

Conventional commits, checked by `pixi run lint-commit` on `commit-msg`. One logical
change per commit. Run `pixi run gates` before pushing, and never commit `dist/`,
`.pixi/`, `target/` or coverage output — all of it is ignored, and the ignore rules are
annotated with why.

Do not commit on the user's behalf unless asked. Do not push to a branch other than the
one named.

<!-- BEGIN:turborepo-agent-rules -->

# This is NOT the Turborepo you know

Turborepo configuration, task behavior, and CLI commands can vary between installed versions and may differ from your training data. Resolve the `turbo` package from this file's directory or relevant workspace; in monorepos, it may not be visible from the repository root. For example, run `node -p "require.resolve('turbo/package.json')"` from a workspace that depends on `turbo`.

Read `docs/README.md` inside that installed package first, then read the relevant pages from its `docs/` directory before changing Turborepo configuration or commands. Heed deprecation notices. These bundled docs match the installed package version and are available without network access.

This block is written and re-added by `turbo` before repository-scoped commands when an AI agent is detected. In the Turborepo source repository, its template is defined in `crates/turborepo-cli/src/cli/agent_guidance.rs`. Removing the managed block while updates are enabled means a later qualifying invocation will add it again. Set `"agentGuidance": false` in the root `turbo.json` or `turbo.jsonc` to opt out; this does not remove an existing block. Keep the block committed with your work to avoid an uncommitted change on the next agent invocation.
<!-- END:turborepo-agent-rules -->
