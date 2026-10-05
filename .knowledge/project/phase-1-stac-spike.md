---
type: Spike Report
title: "Phase 1 spike: a live STAC source through the geoquery CLI"
description: "What the first live-data STAC spike built, proved, and left open."
tags: [spike, stac, phase-1, cli, registry, execution-policy, provenance]
status: draft
generated: { by: agent/arena, at: 2026-10-05T00:00:00Z }
created: 2026-10-05T00:00:00Z
updated: 2026-10-05T00:00:00Z
id: project/phase-1-stac-spike
category: project
refs: [adapters/stac, adapters/adapter-architecture, interfaces/cli, query/planner, project/product-design]
---

# Phase 1 spike: a live STAC source through the geoquery CLI

## What this spike was

The first end-to-end slice of the product loop in
[project/product-design](product-design.md): register a STAC source, run one `GeoQuery`
document against it, and get normalized JSON back with provenance and per-source status —
through the real `geoquery` binary, not a script that calls library functions directly.

Scope was deliberately narrow, by instruction: the STAC translation only ever reads
`bbox`, `datetime`, `collections` and `limit` from a query. Everything else a query might
ask for is reported, not attempted.

## What it proved

- **A STAC API can be discovered generically.** `crates/adapter-stac/src/landing.rs`
  detects a STAC API by `type` + `conformsTo`, not by URL shape or provider, and resolves
  `/search` and `/collections` from the landing page's own links when it names them. This
  was checked against the real, current landing pages of both Earth Search and Microsoft
  Planetary Computer (fetched 2026-10-05; trimmed copies are the fixtures in
  `crates/adapter-stac/tests/landing.rs`), which differ in exactly the way that matters:
  Earth Search advertises the legacy `ogcapi-features#query` extension and no filter
  conformance class, Planetary Computer advertises CQL2 and `item-search#filter`. The
  parser reads both without a provider-specific branch, which is the actual evidence that
  a second source is "supported" in this build — nothing in `landing.rs`,
  `request.rs`, `request.rs` or `response.rs` names a provider.
- **The capability model in `geoquery-core` cannot honestly describe a single-source,
  no-local-post-processing adapter.** `CapabilityReport::for_query`'s spatial logic
  assumes an engine that can evaluate a geometry predicate locally once it has a
  normalized result in hand (the `Approximated` finding: push a bounding box, then filter
  the exact shape against what came back). This spike's adapter does not implement that
  local step, so reusing the generic report would have produced findings the adapter
  could not back up. `crates/adapter-stac/src/request.rs::translate` is a hand-written,
  narrower translator instead, built from the same `CapabilityFinding`/`Support`/`Cause`
  vocabulary so its output composes with `geoquery-core`'s `ExecutionOutcome`, but
  deciding independently of `CapabilityReport::for_query` what counts as `Pushed`,
  `Refused` or `Local`. See that module's doc comment for the specific reasoning per
  feature (why an exact-geometry predicate is `Refused` rather than `Approximated`, why
  field projection is the one feature genuinely `Local`).
- **An execution policy axis is genuinely different from `ExecutionMode`.**
  `ExecutionMode` (`Remote`/`Local`/`Hybrid`/`Auto`) answers *where* work happens;
  the task's `strict`/`balanced`/`exploratory` answers *how much a source's inability to
  fully help should cost the rest of the federated run*. They are orthogonal and now
  coexist on `ExecutionOptions` as `mode` and `policy`
  (`crates/types/src/query.rs::ExecutionPolicy`), with `policy` defaulting to `Balanced`
  — see `ExecutionOptions::policy()`.
- **Partial failure can be made genuinely non-fatal without new core types.**
  `geoquery-core::execution` already had everything an orchestrator needs —
  `SourceOutcome::{Ok, Degraded, Failed, Skipped}`, `ExecutionOutcome::status()` deriving
  `ok`/`partial`/`failed` from the source list rather than storing it redundantly. The CLI
  (`crates/cli/src/main.rs::run_query`) is the first thing in the workspace to actually
  build one of these end to end: one failing source among several still returns a
  `partial` result with exit code `0` under the default `balanced` policy, and the same
  run under `strict` prints the identical JSON but exits non-zero — the explanation is
  never suppressed, only the shell-visible verdict changes. See
  `balanced_policy_tolerates_one_failing_source_among_several` in
  `crates/cli/tests/main.rs`.
- **`balanced` needs a real behavioral difference from `exploratory`, not just a
  different label.** This spike's rule: a source is skipped outright (not even asked)
  under `balanced` when the query asks for a predicate (spatial, temporal, attribute, or
  semantic) and nothing about it could be pushed — asking anyway would only fetch an
  unfiltered catalog. `exploratory` always asks regardless; `strict` skips a source the
  moment *anything* about the query would not reach it unchanged. This is one reasonable
  reading of three words the task left to this spike to define; see "What remains
  unresolved" below.
- **Pushed-down vs. degraded/refused filters can be read off existing fields without a
  new shape.** The CLI's per-source JSON report shows `pushed` as
  `provenance.query` — literally the request that was sent — and `degradations` as the
  non-`Pushed` `CapabilityFinding`s returned alongside it. Nothing had to be invented to
  satisfy "source-level execution status/explain metadata: pushed-down filters,
  degraded/refused filters."

## What remains unresolved

- **No live network verification from this sandbox.** `curl`/`reqwest` to
  `earth-search.aws.element84.com` and to `planetarycomputer.microsoft.com` both fail at
  the TLS handshake from this environment's shell (`SSL_ERROR_SYSCALL` — a mid-handshake
  reset, consistent with an egress allowlist rather than a DNS or routing failure); the
  `source add` path was confirmed to fail cleanly and legibly against this restriction
  (`AdapterError::Unreachable`, surfaced as "could not register ... could not reach ...").
  Every shape this adapter parses was instead checked against *real* responses fetched
  through a different channel (the session's page-fetch tool, which evidently has
  broader egress) and frozen as test fixtures — landing pages, a `/collections` page, and
  a full `/search` item — but nobody has run `geoquery source add` /
  `geoquery query` against a live endpoint end to end. That is the one acceptance
  criterion ("at least one live STAC service registrable and queryable") this spike
  satisfies by construction and fixture-fidelity rather than by a terminal transcript.
  Re-running `crates/adapter-stac/tests/adapter.rs`'s mock-server tests against the real
  URLs (swap `MockServer::start()` for the real base URL) from a machine with open egress
  is the fastest way to close this gap.
- **The `balanced`/`exploratory` line is one spike's judgment call, not a settled
  design.** The rule above (skip when nothing can be pushed) is defensible but arbitrary
  in its threshold — a query with only a `limit` has nothing to "narrow" either, and this
  spike chose to let it through (see `skip_reason` in `crates/cli/src/main.rs`). A future
  pass should decide this from the product's stated principles rather than from what one
  adapter happened to need.
- **No local post-processing step exists anywhere in the workspace yet.** This is why the
  adapter refuses rather than approximates an exact-geometry or attribute-filter
  predicate. The moment an engine-level "apply this predicate to normalized results"
  function exists, `crates/adapter-stac/src/request.rs` should be revisited: several of
  its `Refused` findings become legitimate `Approximated` ones.
- **Registry format is a placeholder, not the Phase 1 design.**
  `.knowledge/infrastructure/storage.md` (if and when it specifies a format) and
  `backlog/tasks/gq-6...md` describe a YAML-backed registry under `~/.geoquery/sources/`
  plus a `geoquery.lock`. This spike used one flat JSON file
  (`~/.geoquery/sources.json` by default, overridable with `--registry` or
  `GEOQUERY_HOME`) because the task asked for "a simple JSON config" explicitly. Treat
  `crates/cli/src/main.rs::Registry` as disposable scaffolding, not the registry crate
  `gq-6` describes.
- **CQL2 / attribute filtering, sorting, field projection beyond the trivial case,
  pagination past one page, and every non-STAC protocol are all still unimplemented.**
  All are visible as `Refused` findings on every query that asks for them, never silent.
- **The CLI surface here (`geoquery source add|list|describe`, `geoquery query`) is this
  spike's literal instruction, not the aspirational surface in
  [interfaces/cli](../interfaces/cli.md) (`geoquery add`, `geoquery sources`,
  `geoquery describe --refresh`, snapshot persistence, `diff`/`check` drift detection).**
  Reconciling the two is future work; this spike did not attempt it.

## Where things live

| Concern | Path |
| --- | --- |
| STAC landing-page / conformance discovery | `crates/adapter-stac/src/landing.rs` |
| `GeoQuery` → STAC search translation + capability findings | `crates/adapter-stac/src/request.rs` |
| STAC `Item`/`ItemCollection` → `GeoResult` normalization | `crates/adapter-stac/src/response.rs` |
| HTTP glue, `ServiceAdapter` implementation | `crates/adapter-stac/src/adapter.rs` |
| `ExecutionPolicy` (`strict`/`balanced`/`exploratory`) | `crates/types/src/query.rs` |
| Source registry, policy enforcement, CLI commands | `crates/cli/src/main.rs` |
| Mock-server adapter tests (no live network) | `crates/adapter-stac/tests/{landing,request,response,adapter}.rs` |
| End-to-end CLI tests (no live network) | `crates/cli/tests/main.rs` |
