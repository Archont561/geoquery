---
id: interfaces/tui
title: TUI — Interactive Terminal Interface
category: interfaces
tags: [TUI, ratatui, crossterm, interactive, terminal, dashboard]
refs: [interfaces/cli, query/planner, project/architecture, extensions/extension-points]
status: draft
created: 2025-07-11
updated: 2025-07-11
---

# TUI — Interactive Terminal Interface

## Crate

`geoquery-tui` — Interactive terminal UI using `ratatui` + `crossterm`.

Launched via `geoquery explore`. **Phase 2 feature** — not part of
the MVP, but the crate should be reserved and the event interface
designed early.

---

## Why a TUI?

A federated geospatial query engine has a naturally TUI-friendly
workflow:

- Query 5–10 sources in parallel
- Results stream in progressively
- Browse, filter, inspect provenance
- Refine queries interactively
- All without leaving the terminal

---

## Layout

```
┌─ Geoquery ──────────────────────────────────────────────────────┐
│ Query: flood risk near Warsaw 2020-2025        [Edit] [Run]    │
├─ Sources ─────────────┬─ Results (47) ─────────────────────────┤
│ ✓ planetary-computer  │ ▸ Sentinel-2 L2A (12 items)           │
│ ✓ copernicus          │   Copernicus DEM (1 item)             │
│ ✓ usgs                │   Landsat 8 (8 items)                 │
│ ⏳ cdse               │ ▸ Flood Risk Map Poland (3 features)  │
│ ✗ private-stac (408)  │   Vistula River Network (23 features) │
│                       │                                       │
├─ Detail ──────────────┴───────────────────────────────────────-─┤
│ Title: Sentinel-2 L2A                                         │
│ Source: planetary-computer (STAC)                             │
│ BBox: [14.1, 49.0, 24.2, 54.8]                               │
│ Time: 2024-06-15 / 2024-08-20                                │
│ Cloud: 3.2%                                                   │
│                                                               │
│ Assets: visual (COG), B02, B03, B04, B08, SCL                │
│                                                               │
│ Provenance: POST /search {bbox, datetime, cloud_cover < 10}   │
├─ Mini Map (ASCII) ─────────────────────────────────────────────┤
│          ·  ·  ·  ╭──────╮                                    │
│       ·  ·  ·  ·  │ ▓▓▓▓ │  ·  ·                             │
│          ·  ·  ·  ╰──────╯                                    │
│               ▲ Warsaw                                        │
└───────────────────────────────────────────────────────────────-─┘
```

---

## Panels

| Panel | Widget | Content |
|-------|--------|---------|
| **Query Bar** | `TextArea` (tui-textarea) | Editable query, run on Enter |
| **Sources** | `Table` + `Sparkline` | Live federation progress per source |
| **Results** | `List` (custom render) | Scrollable, filterable result items |
| **Detail** | `Paragraph` + `Span` | Provenance, assets, links, context |
| **Mini Map** | Custom `Canvas` | ASCII bbox/geometry rendering |
| **Tabs** | `Tabs` | Switch: Results / Sources / Context |

---

## Architecture Integration

The TUI is just another consumer of `geoquery-core`:

```
geoquery-core (query engine)
    ├── geoquery-cli      (one-shot queries)
    ├── geoquery-tui      (interactive exploration)  ← this
    ├── geoquery-http     (REST API)
    └── geoquery-mcp      (AI agents)
```

The TUI calls `core::execute(query)` and subscribes to the
`QueryEvent` stream for live updates:

```
QueryEvent::SourceQueried { source, status }
    → update Sources panel (✓ / ⏳ / ✗)

QueryEvent::ResultsReceived { source, count }
    → append to Results list

QueryEvent::QueryComplete { total, duration }
    → update status bar
```

→ See [query/planner](../query/planner.md) for the event stream
→ See [extensions/extension-points](../extensions/extension-points.md) Layer 13 (events)

---

## Dependencies

```toml
[dependencies]
ratatui = "0.29"
crossterm = "0.28"
tui-textarea = "0.7"       # Query editor
geoquery-core.workspace = true
geoquery-types.workspace = true
geo-types.workspace = true
geojson.workspace = true
tokio.workspace = true
```

---

## MVP Scope (Phased)

| Phase | Feature |
|-------|---------|
| **2a** | Read-only result browser (query via CLI args, browse in TUI) |
| **2b** | Source status dashboard with live federation progress |
| **2c** | Detail + provenance view |
| **3a** | Interactive query editing |
| **3b** | ASCII map rendering |
| **3c** | Saved queries / history |
| **4** | Resource registry browser |

---

## Keybindings (Proposed)

| Key | Action |
|-----|--------|
| `Enter` | Execute query |
| `Tab` | Switch panels |
| `j/k` or `↑/↓` | Navigate results |
| `Enter` (on result) | Show detail |
| `e` | Edit query |
| `f` | Filter results |
| `q` | Quit |
| `?` | Help |

---

## Related Files

- [interfaces/cli](cli.md) — The `geoquery explore` subcommand
- [query/planner](../query/planner.md) — Event stream for live updates
- [extensions/extension-points](../extensions/extension-points.md) — Event listener trait
- [project/architecture](../project/architecture.md) — Where TUI sits in the stack
