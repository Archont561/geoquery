# Backlog

This directory is the delivery plan for GeoQuery. It is intentionally separate from
[`.knowledge/`](../.knowledge/), which records durable context rather than work tracking.

## Entries

- [`milestones/`](milestones/) — phase outcomes, sequencing, and priorities.
- [`tasks/`](tasks/) — actionable work with status, dependencies, and acceptance criteria.
- [`docs/`](docs/) — canonical plans, specifications, research notes, and spikes that support
  delivery but are not part of the durable knowledge corpus.

## Ownership boundary

Put the **what and when** of delivery here: a planned change, its dependencies, status, and
proof that it is complete. Put the **what and why** of the product and architecture in
[`.knowledge/`](../.knowledge/). Link between the two instead of duplicating content.

The backlog is managed through the repository task:

```bash
pixi run backlog
```
