---
id: GQ-24
title: Keep the workspace layout and its specification in step
status: Done
assignee: []
created_date: '2026-09-30 20:41'
updated_date: '2026-10-04 20:24'
labels:
  - monorepo
  - infrastructure
  - docs
milestone: m-7
dependencies: []
references:
  - .knowledge/infrastructure/monorepo.md
priority: medium
type: chore
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The monorepo concept already carries a divergence section because the root manifest is a package rather than a virtual workspace. Keep that section true as crates land, so the layout stays one decision recorded once instead of drifting into folklore.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The documented directory layout matches the workspace: a root package, crates/* with the cli exclusion, crates/xtask, and apps/docs.
- [x] #2 Lints, dependency versions and release metadata are inherited from the workspace root by every crate. The one crate that cannot inherit the lint policy restates it, and the restatement is checked against the workspace rather than trusted.
- [x] #3 Adding a crate requires one manifest edit, verified against cargo metadata rather than by inspection.
- [x] #4 Every crate keeps tests mirroring its sources and exercising the public surface, as the specification claims.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added scripts/layout.ts and pixi run layout-check, in the gates aggregator beside version-check. It asserts the three structural promises the monorepo concept makes: the crates/* glob with the crates/cli exclusion, inheritance of lints and dependency versions and release metadata, and the one-test-file-per-source-file mirror. Membership is read from cargo metadata rather than from the members line, because a glob that matches nothing is a correct-looking manifest and an empty workspace.

The check found one real defect on its first run: crates/node-native has no lints workspace = true. It turned out to be a deliberate, well-argued exception rather than drift - the napi macro expands to a constructor carrying its own allow for unsafe_code, forbid is the one level a macro cannot override, and cargo gives a member no way to override a single inherited lint, so the crate copies the tables and softens unsafe_code to deny. Its comment ended with the words must move with it, which nothing enforced. The check now compares the copy against workspace.lints key by key and permits exactly that one difference, so a lint added to the workspace and not to the addon fails, and a second softened lint fails.

AC2 was rewritten before being ticked. As written it said every crate inherits with no per-crate duplicates, which is not true and cannot be made true without breaking the addon build. The replacement says what actually holds and is stronger: the exception exists and is checked rather than trusted.

AC4 is fully mechanical in a way worth noting. Cargo compiles each file in tests/ as its own crate linked against the library, so a test there can only reach public items. Being in tests/ is the guarantee that it exercises the public surface, and the mirror check is the guarantee that one exists per source file.

monorepo.md: the Directory Layout tree was the original design and contradicted the divergence section printed directly above it - it showed a virtual workspace root, a root xtask directory, rustfmt.toml, .cargo/config.toml, schemas/ and docs/, none of which exist. Replaced with the repository as it is, plus a pointer to what enforces it. Also recorded the node-native lint exception, which the file had never mentioned, and qualified the no per-crate Clippy drift claim that the exception falsifies.

Verified by mutation: a hand-written member list, a lint dropped from the addon, a second lint softened in the addon, a crate pinning its own serde_json version, and an orphan test file were each introduced in turn and each reported.
<!-- SECTION:NOTES:END -->
