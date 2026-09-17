---
type: Query Specification
title: Temporal Query Language
description: "Temporal predicates (during, intersects, open-ended intervals)."
tags: [temporal, predicates, interval, datetime, ISO-8601]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-09-17T00:00:00Z
id: query/temporal
category: query
refs: [query/query-model, query/planner, project/data-model]
---

# Temporal Query Language

## Temporal Predicates

| Op | Meaning |
|----|---------|
| `before` | Resource ends before the instant/interval |
| `after` | Resource starts after the instant/interval |
| `during` | Resource extent contained within interval |
| `intersects` | Resource extent overlaps interval at any point |
| `contains` | Resource extent fully covers the interval |
| `overlaps` | Partial overlap (same as intersects for intervals) |

Open enum with `Custom(String)` extension.

---

## Examples

### During

```json
{
  "temporal": {
    "op": "during",
    "start": "2020-01-01",
    "end": "2025-01-01"
  }
}
```

### Intersects

```json
{
  "temporal": {
    "op": "intersects",
    "start": "2020-01-01",
    "end": "2025-01-01"
  }
}
```

### Open-Ended Intervals

Open-ended intervals MUST be supported — omit either bound:

```json
{ "temporal": { "op": "after", "start": "2020-01-01" } }
```

```json
{ "temporal": { "op": "before", "end": "2025-01-01" } }
```

---

## Format Rules

| Rule | Detail |
|------|--------|
| Dates | ISO 8601 (`YYYY-MM-DD`) |
| Timestamps | ISO 8601 with timezone (`2024-06-15T10:30:00Z`) |
| Intervals | Start/end pair; either may be omitted |
| Validation | `start > end` is a rejection condition |

**Rust mapping:** `chrono::DateTime<Utc>` for instants;
`TemporalExtent { start: Option<DateTime<Utc>>, end: Option<DateTime<Utc>> }`
for extents.

---

## Compilation Targets

The temporal predicate compiles per-adapter:

| Target | Translation |
|--------|-------------|
| STAC API | `datetime=2020-01-01/2025-01-01` parameter |
| OGC API Features | `datetime=2020-01-01/2025-01-01` |
| CQL2 | `{ "op": "t_intersects", "args": [...] }` |
| PostGIS | SQL `WHERE` on timestamp column |
| ArcGIS | `time` filter / `where` clause |

→ See [query/filters](filters.md) for the compilation framework

---

## Instant vs Interval Semantics

A resource's temporal extent is itself an interval. Therefore:

- `intersects` — any overlap between query interval and resource extent
- `contains` — resource extent fully inside query interval
- `during` — treated as `contains` from the resource's perspective
- Instant queries (single date) — degenerate interval `[date, date]`

These semantics align with STAC `datetime` and OGC temporal operators.

---

## Related Files

- [query/query-model](query-model.md) — The full GeoQuery AST
- [query/filters](filters.md) — Attribute filter compilation
- [query/planner](planner.md) — Temporal capability matching
- [project/data-model](../project/data-model.md) — TemporalExtent on resources
