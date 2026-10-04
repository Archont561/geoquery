---
id: GQ-23
title: Reconcile the CI pipeline with its specification
status: In Progress
assignee: []
created_date: '2026-09-30 20:41'
updated_date: '2026-10-04 20:46'
labels:
  - ci
  - infrastructure
milestone: m-7
dependencies: []
references:
  - .knowledge/infrastructure/ci.md
  - .knowledge/infrastructure/xtask.md
priority: medium
type: chore
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The knowledge base still describes a cargo xtask CI pipeline while the workflows run pixi and turbo, and the future-job table has no owner for the checks that are missing. Close the gap in both directions so the document is a design a reader can trust, not a claim the build contradicts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The CI concept records the implemented pixi and turbo pipeline, or records the divergence in the file it diverges from, per the repository convention.
- [ ] #2 The MSRV check runs in CI so the resolver-3 rust-version claim is verified rather than asserted in a manifest.
- [x] #3 Every job in the future-job table names the phase that owns it and the trigger that introduces it.
- [x] #4 A single local command reproduces the CI checks, and the workflows call it instead of restating the steps.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
UPDATE - the MSRV job was removed at the maintainers request after its first CI run, so AC2 is unchecked again and this task is back In Progress with 3 of 4 done.

What the one run established is worth keeping. The job read rust-version out of Cargo.toml, installed 1.88 with rustup and cached successfully - every mechanical step passed. The final step, cargo check --workspace --all-targets --locked, failed. So the workspace does not build on the version its own manifest promises, and rust-version = 1.88 is not merely unverified, it is wrong. Under resolver 3 the number also drives dependency version selection, so it is a claim about the resolved graph and not only about the source.

The right sequence is therefore the reverse of what was attempted: find the real minimum first, declare that, and only then add a job to hold it there. Adding the job before correcting the number means adding a job that fails on day one, which is what happened. The divergence is now recorded in ci.md, and MSRV check is back in the future-job table with phase 1 and the trigger being rust-version corrected to a version the workspace actually builds on.

AC1, AC3 and AC4 are unaffected and remain done: the divergence sections in ci.md and xtask.md, the rewritten future-job table, and the single command the workflow calls instead of restating the steps.
<!-- SECTION:NOTES:END -->
