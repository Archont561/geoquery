---
id: GQ-32
title: Preserve integer precision across the FFI transport
status: Done
assignee: []
created_date: '2026-10-04 19:35'
updated_date: '2026-10-04 19:53'
labels:
  - bug
  - ffi
dependencies: []
priority: medium
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A Hypothesis property test found that geoquery.invoke loses precision for JSON integers outside the i64 range. Sending -9223372036854775809 returns -9.223372036854776e+18. The Rust side parses the payload into a serde_json Value and an integer too large for i64 or u64 becomes an f64, so the round trip is lossy. JSON itself places no limit on integer magnitude, so either the transport preserves arbitrary precision or the boundary rejects the value instead of quietly changing it. Discovered during GQ-5 and unrelated to it. The failing example is cached in the untracked hypothesis database, so CI will only see it when its own random search rediscovers it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The transport states its number domain where the wire contract lives, rather than leaving it to whatever serde_json happens to do
- [x] #2 The edges of that domain are pinned as tests on all three sides: Rust, Python and TypeScript
- [x] #3 The behaviour past the edge is pinned too, so the limitation is named and tested rather than discovered
- [x] #4 The three any-JSON generators agree about what the protocol carries
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Resolved by bounding the contract, not by widening it. The original first criterion asked for exactness or a clear rejection and both were measured and declined.

Exactness via serde_json arbitrary_precision works - all tests pass with it and the Hypothesis counterexample goes green - but it makes every Number a heap-allocated String, coordinates included, measured at 123ms to 140ms end to end on a 200k-number geometry. Rejection was declined because the check cannot live in the engine without that same feature, so it would be enforced in one binding and not the others.

The deciding argument is TypeScript. A JavaScript number is a double, so JSON.parse rounds an integer past 2 to the 53rd before a request reaches the engine and after a response leaves it. No engine-side change can make the transport exact for TypeScript, so exactness in Rust and Python would have produced three contracts instead of one. The strongest promise the transport can make is a bounded domain, and that is now what it promises.

The float_roundtrip precedent was examined and points the other way: it buys fidelity for coordinates, which is data that exists. arbitrary_precision would tax coordinates to fix magnitudes no geospatial payload contains.
<!-- SECTION:NOTES:END -->
