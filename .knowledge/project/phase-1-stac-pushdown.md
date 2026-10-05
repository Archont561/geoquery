---
type: Spike Report
title: "Conformance-driven pushdown: CQL2 filtering, sorting, fields and pagination"
description: "What the second STAC pass built on top of the Phase 1 spike, what it proved, and what it left open."
tags: [spike, stac, cql2, filter, sort, fields, pagination, conformance, capabilities]
status: draft
generated: { by: agent/arena, at: 2026-10-05T00:00:00Z }
created: 2026-10-05T00:00:00Z
updated: 2026-10-05T00:00:00Z
id: project/phase-1-stac-pushdown
category: project
refs: [project/phase-1-stac-spike, adapters/stac, adapters/adapter-architecture, query/filters, query/planner, project/product-design]
---

# Conformance-driven pushdown: CQL2 filtering, sorting, fields and pagination

## What this pass was

The fifth item on [phase-1-stac-spike](phase-1-stac-spike.md)'s "What remains unresolved"
list, taken on deliberately and on its own: *"CQL2/attribute filtering, sorting, field
projection beyond the trivial case, pagination past one page … are all still
unimplemented — visible as Refused findings, never silent, but absent."*

All four are now implemented in `crates/adapter-stac`, and what decides whether any of
them reaches a service is that service's own `conformsTo` list — never its hostname,
never a configuration flag, and never an optimistic default.

## What it proved

- **Capability negotiation can be driven entirely by published conformance classes.**
  `crates/adapter-stac/src/landing.rs::StacConformance` reads six facts off the landing
  page's `conformsTo` array (filter binding, `cql2-json`, `basic-cql2`,
  `advanced-comparison-operators`, `item-search#sort`, `item-search#fields`) and nothing
  else, matched by substring so that the `v1.0.0`, `v1.0.0-rc.*` and `v1.1.0` spellings of
  the same class all count — Planetary Computer still publishes its filter binding under
  the `rc.2` URI. The same struct is computed twice from the same words: once at
  registration, where it fills `CapabilitySet`, and once at query time from the
  `conformsTo` list the descriptor kept in `metadata`
  (`StacConformance::for_service`). `conformance_is_read_back_off_a_registered_descriptor`
  in `crates/adapter-stac/tests/landing.rs` is what keeps the two answers identical.
- **The two real services in the corpus split the capability matrix cleanly, and the
  adapter behaves differently for each without naming either.** Earth Search declares
  `item-search#sort` and `item-search#fields` and no filter conformance at all; Planetary
  Computer declares `basic-cql2` + `cql2-json` + the filter binding and neither sort nor
  fields nor `advanced-comparison-operators`. So a filter is pushed to one and refused by
  the other, a sort is pushed to the other and refused by the first, and a `LIKE` is
  refused by both — for three different, stated reasons. The fixtures in
  `crates/adapter-stac/tests/{landing,request}.rs` are those two real `conformsTo` lists.
- **CQL2 JSON is a total translation of this project's filter AST.**
  `crates/adapter-stac/src/cql2.rs::compile` is infallible and returns no `Result`: every
  `FilterExpr` node has exactly one CQL2 JSON spelling, and `CompareOp::as_str` already
  *is* the CQL2 operator spelling, so the comparison operators are not translated at all.
  What can fail is a service's willingness to evaluate the result, and that is decided
  before compilation rather than discovered after it.
- **"Which CQL2" is a real question, not a pedantic one.** CQL2 splits `LIKE` and `IN`
  into the `advanced-comparison-operators` conformance class, which Planetary Computer
  does not declare while declaring `basic-cql2`. `cql2::needs_advanced_comparison` walks
  the tree before anything is compiled, and a tree that needs the class a service did not
  publish is refused *whole* — not partially pushed. Partial pushdown of a boolean tree
  changes what the query means (dropping a branch of an `or` narrows the answer; dropping
  one of an `and` widens it), which is why `geoquery-core` models `AttributeFilter` as one
  feature rather than one per leaf.
- **`Cause::Declined` and `Cause::Undeclared` finally mean different things here.** Every
  refusal in the Phase 1 spike was `Undeclared`, because the refusals were all this
  adapter's own limitations. Now a STAC API that published a `conformsTo` list without the
  class in question has *declined* that feature, while a descriptor carrying no
  recognisable class at all (an older registry entry, or a service that said nothing this
  adapter understood) is `Undeclared`. The distinction is visible in `geoquery query`
  output, because the findings are typed.
- **Local post-processing that removes nothing is not the local post-processing step this
  workspace is missing.** Two genuinely local steps now exist —
  `response::project_fields` trims each result's `properties`, and `adapter.rs` cuts the
  `offset`/`limit` window out of a page sequence — and both are reported as
  `Support::Local`, truthfully, because the adapter performs them. Neither is a
  *narrowing* step: no predicate is evaluated against a result, nothing that matched is
  dropped. That is why item 3 of the Phase 1 backlog is still open and why no finding
  anywhere in this adapter is `Support::Approximated` yet.
- **Field projection must not be left to the service even when the service offers it.**
  The STAC Fields extension says in as many words that `include`/`exclude` are *"only
  hints to the server … not a contract about what the response will be"*. So the hint is
  sent when the service declared the extension (it saves transfer) and the local trim runs
  either way (it is what makes the projection exact). A second trap the spec sets: an
  `include` list is a *replacement*, so `include: ["eo:cloud_cover"]` can legally come back
  as items with no `id` — which this adapter's normalizer drops, because `GeoResult::id` is
  not optional. Every `include` therefore carries the extension's own recommended default
  set (`type`, `stac_version`, `id`, `geometry`, `bbox`, `links`, `assets`,
  `properties.datetime`) plus `collection`, ahead of whatever the caller asked for.
- **Pagination is the service's own link, followed, not a page counter invented here.**
  `response::NextLink` now reads `href`, `method`, `body` and `merge` rather than just the
  href, which is the difference between knowing another page exists and being able to
  fetch it: every pgstac-backed service pages with `method: POST`, `body: {"token": …}`,
  `merge: true`, and a follower that ignored `merge` would page through an *unfiltered*
  catalogue. `adapter.rs::fetch_pages` follows those links until the caller's window is
  filled, bounded by `MAX_PAGES = 10` — a remote service should not get to choose how long
  a client's loop runs — and a run stopped by the cap still returns the unused `next` link,
  so the result says it is partial rather than pretending to be complete.
- **A query with no `limit` still fetches exactly one page.** Treating "no limit" as
  "every page" would turn the absence of a parameter into a catalogue scan. The `next`
  link comes back untouched for a caller who wants more.
- **`balanced` had to learn that a pushed filter narrows a query.** `skip_reason` in
  `crates/cli/src/main.rs` counted only spatial and temporal pushdown as "this source can
  narrow something", because when it was written nothing else could be pushed. A
  filter-only query against a CQL2 service would therefore have been skipped by the
  default policy *because the filter worked*. `AttributeFilter` is now in that list. The
  two CLI tests `a_filter_reaches_a_source_that_declared_cql2_json` and
  `a_filter_only_query_skips_a_source_that_cannot_push_it` run the same query document
  against two mock services differing by one conformance class, and get a result from one
  and a documented skip from the other.

## What remains unresolved

- **Still no live network verification.** Egress from this sandbox is blocked exactly as
  it was in the first pass — `curl https://earth-search.aws.element84.com/v1` and
  `https://planetarycomputer.microsoft.com/api/stac/v1` both die mid-TLS-handshake
  (`SSL_ERROR_SYSCALL`), and `pixi run gates`' own `publish-plan` step fails against
  `prefix.dev` for the same reason. Every shape here was checked against frozen real
  responses and `wiremock`; nothing has been run against a live endpoint. Item 1 of
  [phase-1-stac-spike](phase-1-stac-spike.md) is unchanged, and the CQL2 request this pass
  emits is now one more thing that would be worth confirming against Planetary Computer
  from a machine with open egress.
- **Queryables are not consulted.** A service publishes `/queryables` (and
  `/collections/{id}/queryables`) describing which properties may be filtered on and with
  what types; this adapter does not fetch it. The consequence is that
  `Cause::FieldNotQueryable` — which `geoquery-core` models precisely for this — is never
  produced: a filter on a property the service does not publish is pushed and answered
  with whatever the service does with it, usually a 400. Fetching queryables at `describe`
  time and checking filter fields against them at translate time is the obvious next pass,
  and it is also what would let `geoquery describe` show a user what they can filter on.
- **Sorting is refused rather than approximated, and that may be too strict.** A local
  sort of one page is a sorted list of the wrong items — "the ten least cloudy scenes" is
  not "ten scenes, sorted by cloud cover" — so `Refused` is honest today. Once a federated
  engine exists that collects from several sources before returning, a *global* sort over
  the assembled result set becomes meaningful, and that is the point at which this should
  be revisited. It is the same missing engine-level step as item 3 of the Phase 1 backlog,
  approached from a different side.
- **CQL2's temporal and spatial functions are not emitted.** `t_before`, `t_during`,
  `s_intersects` and friends would let the non-`intersects` temporal operations and the
  exact-geometry spatial predicates be pushed rather than refused, which would close the
  oldest refusals in this adapter. They are deliberately not implemented yet: they live in
  the `temporal-functions` and `spatial-functions` conformance classes, and neither of the
  two services this project has met declares either, so a first implementation would be
  gated into never running. It needs a third service — or a live-network pass — to be
  testable as anything other than code nobody reaches.
- **`offset` is answered by fetching and discarding.** Correct, reported as
  `Support::Local`, and linear in the offset: `offset: 10_000` means ten thousand items
  across up to ten pages, and the `MAX_PAGES` cap silently decides the answer before the
  offset is reached. A page-token-aware registry (remembering the token for the page a
  previous query ended on) is the only way STAC can do better, and that belongs to the
  registry work (gq-6), not here.
- **`MAX_PAGES = 10` is a judgment call, not a measured one.** Same category of unsettled
  decision as the `balanced`/`exploratory` threshold in item 2 of the Phase 1 backlog: the
  number is defensible and arbitrary. What is *not* arbitrary is that a cap exists, that
  hitting it is visible (the unused `next` link comes back), and that it is one constant
  with one comment rather than a behaviour spread across the fetch loop.
- **Nothing here reconciles the two CLI shapes or the registry format.** Items 4 and 6 of
  the Phase 1 backlog are untouched: this pass added `conformsTo` to what a registry entry
  carries and nothing else, and it still carries it in the same disposable flat JSON file.

## Where things live

| Concern | Path |
| --- | --- |
| `conformsTo` → what may be pushed | `crates/adapter-stac/src/landing.rs::StacConformance` |
| `FilterExpr` → CQL2 JSON | `crates/adapter-stac/src/cql2.rs` |
| Conformance-gated translation + findings | `crates/adapter-stac/src/request.rs::translate` |
| `next` link parsing, local field projection | `crates/adapter-stac/src/response.rs` |
| Page following, `offset`/`limit` window, page cap | `crates/adapter-stac/src/adapter.rs::fetch_pages` |
| `balanced` counting a pushed filter as narrowing | `crates/cli/src/main.rs::skip_reason` |
| CQL2 wire-format tests | `crates/adapter-stac/tests/cql2.rs` |
| Pushdown-per-conformance tests | `crates/adapter-stac/tests/request.rs` |
| Pagination / projection against a mock server | `crates/adapter-stac/tests/adapter.rs` |
| The same query document, two services, two outcomes | `crates/cli/tests/main.rs` |
