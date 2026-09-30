---
id: GQ-19
title: Support native resource manifests and Markdown context
status: To Do
assignee: []
created_date: '2026-09-30 20:41'
labels:
  - phase-4
  - adapters
  - native
milestone: m-3
dependencies:
  - GQ-5
  - GQ-6
references:
  - .knowledge/adapters/native.md
  - .knowledge/project/data-model.md
priority: medium
type: feature
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A registry entry that only carries an endpoint cannot tell an agent what a dataset is for or what it must not be used for. crates/adapter-native is a stub; make resource.yaml plus its Markdown sidecars a first-class source of context and machine-checkable constraints.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The resource.yaml manifest has a published JSON Schema and registers native resources into the same registry as discovered services.
- [ ] #2 Markdown sidecars are parsed and attached as retrievable context rather than being left as unindexed prose.
- [ ] #3 Structured constraints declared in the manifest are enforced by the planner instead of being stated only in prose an agent may ignore.
- [ ] #4 A validate command reports schema violations and broken links with documented exit codes.
<!-- AC:END -->
