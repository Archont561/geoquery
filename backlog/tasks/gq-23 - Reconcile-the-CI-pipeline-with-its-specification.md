---
id: GQ-23
title: Reconcile the CI pipeline with its specification
status: Done
assignee: []
created_date: '2026-09-30 20:41'
updated_date: '2026-10-04 20:28'
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
- [x] #2 The MSRV check runs in CI so the resolver-3 rust-version claim is verified rather than asserted in a manifest.
- [x] #3 Every job in the future-job table names the phase that owns it and the trigger that introduces it.
- [x] #4 A single local command reproduces the CI checks, and the workflows call it instead of restating the steps.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC4 first, because it decides the shape of the rest. The checks job now runs one command, pixi run ci-checks, instead of seven named steps. ci-checks is gates plus cov. The point is not brevity: a list of checks in a workflow file is a second list, and the way a second list fails is that a check added to gates passes locally and never runs in CI, which looks exactly like a green build. There is now one list, in pixi.toml. pixi prints a header per task, so a failure still names the check that failed.

AC2 found a real gap. Cargo.toml declares rust-version 1.88 and the pixi environment pins rust 1.98.1, so every job built on a toolchain that could not possibly notice the promise being broken. Under resolver 3 the number is not decorative - it participates in dependency version selection - so it was a claim about the graph that nothing compiled. Added an msrv job: it reads the version out of Cargo.toml rather than hardcoding it, installs that toolchain with rustup, and runs cargo check --workspace --all-targets --locked. It is the one job that cannot go through pixi, because the environment pins exactly one Rust.

Worth flagging: this job has never run. There is no network here and no 1.88 toolchain, so it is unverifiable locally by construction. If it fails on its first CI run that is the check doing its job - the answer is then to raise rust-version to the truth, not to delete the job.

A first draft of that job pinned dtolnay/rust-toolchain to a commit SHA I had invented, which would have failed at action resolution. Replaced with rustup, which the runner already has: no third-party action to pin, and the version stays a variable rather than becoming a third place the MSRV is written down.

AC1 follows the monorepo.md convention: a Divergence from the build section at the top, design body left intact below it. ci.md gained one - the design describes five cargo jobs, dtolnay and Swatinem actions, a cargo-deny action and RUSTFLAGS in the workflow environment, and none of that is what runs. xtask.md gained one too, since it still told a reader that CI calls cargo xtask ci and documented four commands that do not exist. Local CI Reproduction said cargo xtask ci and now names the three real entry points plus the one check they cannot reproduce.

AC3: the future-job table had commands in the Trigger column rather than triggers, and listed three jobs that already run. Trigger now names the event that has to become true before the job is worth adding. TypeScript tests and Python tests left the table because they run inside pixi run test via turbo, and MSRV check left it because it is now a job - the design had scheduled it for phase 3 against rust-version 1.85, and both the phase and the number had moved.
<!-- SECTION:NOTES:END -->
