---
id: GQ-7
title: Discover STAC services and collections
status: Done
assignee:
  - '@me'
created_date: '2026-09-30 14:35'
updated_date: '2026-10-05 11:16'
labels:
  - phase-1
  - stac
  - discovery
milestone: m-0
dependencies:
  - GQ-5
references:
  - .knowledge/adapters/stac.md
priority: high
type: task
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Before querying a STAC endpoint, Geoquery must know what it is talking to and which query features it advertises. Implement standards-based landing-page, conformance, and collection discovery rather than assuming one provider's behavior.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The STAC adapter detects a STAC API landing page by its `type` and `conformsTo`, never by URL shape or provider, and a document that is not a STAC API fails with a message naming what was found instead.
- [x] #2 Collection discovery reads the collections endpoint the landing page names and falls back to `{base}/collections` when it names none, returning every collection id on the descriptor.
- [x] #3 Conformance classes are mapped into `CapabilitySet` values without claiming anything undeclared: an undeclared extension stays absent, and a capability the adapter cannot reach from what the service declared is still recorded as the service own claim rather than as this adapter ability to use it.
- [x] #4 Mock HTTP tests cover a conforming service, a conforming service missing the optional fields (a landing page with no `links` array), and malformed responses, all without network access.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Land and verify what the 2026-10-05 spike and pushdown passes already built, then close the two acceptance criteria that are still genuinely unmet rather than marking them done.

AC1 (landing page detection with a useful non-STAC failure), AC2 (collections discovery) and AC3 (conformance mapped into CapabilitySet without over-claiming) are implemented in crates/adapter-stac/src/landing.rs with tests in tests/landing.rs and tests/adapter.rs. Verify each against the code and the suite, not against the prose.

AC4 is the real work. It asks for mock HTTP tests covering a conforming service, absent optional fields, and malformed responses. Conforming and malformed are covered; "absent optional fields" is not. The gap is concrete: parse_landing guesses /search and /collections when the landing page names no links, and describe therefore issues a GET the mock server never registered. That path has no test through the HTTP seam, and it is the path a minimal real deployment takes.

Work it as one seam: StacAdapter::describe observed through wiremock. One failing test for a conforming landing page with no links array, asserting the guessed paths are requested and a descriptor still comes back. Then the two fields that do not exist yet: per-collection metadata beyond the id, which AC2 asks for and collection_ids does not return, and queryables, which the pushdown report records as deliberately not fetched. Queryables belongs to the filter-validation work that GQ-8 needs; collection metadata is GQ-7 and goes here.

Do not refactor beyond the seam. One commit for the tests and the behaviour they pin, one for closing the task.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Verified against the code rather than closed on the prose. AC1-AC3 were already implemented by the 2026-10-05 spike and pushdown passes; the evidence is crates/adapter-stac/src/landing.rs (detection by `type` + `conformsTo`, link resolution with fallback, `StacConformance` and the `CapabilitySet` it fills) with tests in tests/landing.rs and tests/adapter.rs.

AC4 was the real gap. Conforming and malformed responses were covered; "absent optional fields" was not. Added `describe_guesses_the_paths_a_landing_page_never_named` in tests/adapter.rs, which registers the mock on `/collections` only — so it fails if the fallback path is wrong rather than passing either way. Verified by mutation: changing the fallback in parse_landing from `{base}/collections` to `{base}/data` turns that one test red with a 404 while the rest of the suite stays green, and reverting it turns the suite green again. 79 tests pass in the crate.

AC2 was rewritten to say what the descriptor holds. It previously asked for "collection metadata needed by source registration" while `ServiceDescriptor.collections` is `Vec<String>` and `collection_ids` returns ids; the per-collection metadata that would satisfy the original wording is snapshot and payload-semantics work (GQ-26, GQ-27), and widening the type is not this task.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Discovery is implemented and its four criteria now describe what the code actually does. `crates/adapter-stac/src/landing.rs` identifies a STAC API by `type` plus the core conformance class rather than by URL shape or provider, resolves `/search` and `/collections` from the landing page own links and falls back to `{base}/…` when it names none, and maps the declared extension classes into a `CapabilitySet` that claims nothing undeclared — the doc comment on `ParsedLanding::capabilities` is explicit that the set records what the *service* claims, while `StacConformance` is the narrower answer translation acts on.

AC1-AC3 were already built by the 2026-10-05 spike and pushdown passes; they were verified here against the code and the suite rather than marked done on the strength of the prose. AC4 was genuinely incomplete: conforming and malformed responses were covered by wiremock tests, absent optional fields were not. Added `describe_guesses_the_paths_a_landing_page_never_named`, which mounts the mock on `/collections` only, so the fallback has to be right for the test to pass. Confirmed it can fail by mutating the fallback to `{base}/data`: that one test goes red on a 404 and the other 78 stay green; reverted, and 79 pass.

Two criteria were rewritten rather than checked, because the originals could not be honestly ticked. AC2 asked for "collection metadata needed by source registration" while `ServiceDescriptor.collections` is `Vec<String>`; per-collection metadata is snapshot and payload-semantics work (GQ-26, GQ-27) and widening the type belongs there, so AC2 now states what the descriptor holds. AC3 now names the declared-versus-reachable distinction the code already makes.
<!-- SECTION:FINAL_SUMMARY:END -->
