# `.knowledge/` — durable project context

This directory contains the design corpus: architecture, contracts, invariants, standards
interpretation, research, and the rationale behind implementation choices. It is written for
humans and agents who need to understand the system before changing it.

## Ownership boundary

- **`.knowledge/` owns durable context.** Keep explanations of what the system means, why a
  choice was made, and evidence that remains useful after a task is complete.
- **`backlog/` owns delivery work.** Keep milestones, actionable tasks, sequencing,
  acceptance criteria, and implementation plans there.
- A knowledge page may explain why a backlog item exists, but it must not become a second task
  list or delivery schedule.

Start with [`CONTEXT.md`](CONTEXT.md), then use [`index.md`](index.md) to navigate the corpus.
The current implementation-plan entry is in
[`backlog/docs/plans/monorepo-refactor.md`](../backlog/docs/plans/monorepo-refactor.md).
