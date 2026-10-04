---
id: GQ-32
title: Preserve integer precision across the FFI transport
status: To Do
assignee: []
created_date: '2026-10-04 19:35'
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
- [ ] #1 A JSON integer outside the i64 and u64 range survives an invoke round trip unchanged, or is rejected with a clear error rather than silently converted
- [ ] #2 The same property holds on all three sides: Rust, Python and TypeScript
- [ ] #3 A regression test pins the specific value Hypothesis found
<!-- AC:END -->
