---
type: Interface Specification
title: "Python SDK — geoquery"
description: "Native Python bindings for geoquery through maturin and PyO3."
tags: [Python, SDK, GeoPandas, PyO3, maturin, GeoArrow]
status: draft
generated: { by: agent/geoquery-kb-generator, at: 2025-07-11T00:00:00Z }
created: 2025-07-11T00:00:00Z
updated: 2026-10-02T00:00:00Z
id: interfaces/python
category: interfaces
refs: [query/query-model, project/data-model, infrastructure/xtask]
---

# Python SDK — `geoquery`

`geoquery-sdk` is a native Python distribution for data scientists, GIS analysts, and
AI/ML workflows. The package is a thin PyO3 wrapper around the Rust query language and
engine; it does not use an HTTP client or require a running GeoQuery service.

## Boundary

```
Python package  ──PyO3 / maturin──▶  geoquery-core + query engine
```

The Python module and the Rust CLI share the same parser and protocol types. Python-facing
conversions belong at the boundary; query semantics belong in Rust and are never re-created
as Pydantic models in Python.

## Package structure

<!-- DIVERGENCE: the native crate moved. It is `crates/python-native`, not
`python/geoquery/native/`, because the workspace rule is that a Cargo package's manifest
lives beside the language code it serves and the Rust lint set is declared once in the root
manifest. `pyproject.toml` reaches across with `manifest-path = "../../crates/python-native/
Cargo.toml"`; the Python module name `geoquery._native` is unchanged. -->
```
python/geoquery/
├── pyproject.toml          # maturin build config; manifest-path -> crates/python-native
├── src/geoquery/
│   └── __init__.py         # transport client, version surface, DTOs
└── tests/
crates/python-native/
├── Cargo.toml              # PyO3 cdylib, publish = false
└── src/lib.rs              # geoquery._native
```

Build and install locally with:

```bash
pixi run py-install   # maturin develop through pip
pixi run py-dist      # platform wheel and sdist
```

## Current API

```python
import geoquery

assert geoquery.protocol_version()["version"] == geoquery.VERSION
keys = geoquery.parse_document('{"bbox": [14.1, 49.0, 24.2, 54.8]}')
assert keys == ["bbox"]
```

`parse_document` is the first deliberately small FFI operation. It parses JSON with
`geoquery-core` and returns sorted top-level keys. A document the parser rejects raises
`geoquery.EngineError`, which carries the Rust engine's own wording in its message and the
full structured error in `detail` — so the Python exception and the Rust classification are
the same fact rather than two renderings of it. The extension is usable before the execution
engine is complete and proves that the Python path cannot drift from the Rust query document
implementation.

<!-- DIVERGENCE: the transport envelope. `protocol_version()` returns a mapping, not a
version string, and the query surface is `parse_document`, not `check_query`. Both follow
from the versioned JSON transport added after this page was drafted: a single `invoke`
carries every operation so the wire has one shape to version, and an operation that can
fail answers with `{ok: false, result: {...}}` instead of raising across the boundary. -->
The Python surface is one `invoke` over a versioned JSON envelope (`{"transportVersion",
"operation", "payload"}` in, `{"transportVersion", "ok", "result"}` out). `invoke_raw` returns
the envelope and `invoke` raises `EngineError` on `ok: false`, so a caller chooses whether a
failure is an exception rather than having that choice made for it.

## Native roadmap

The next native layers are:

1. expose the canonical query AST through Rust constructors or JSON conversion;
2. execute queries through the in-process planner and adapter registry;
3. return normalized results through Python-owned views or the Arrow C Data Interface;
4. add optional GeoPandas, Shapely, and PyArrow adapters without making them core runtime
   dependencies.

The HTTP crate remains a server/interface for remote applications and clients that cannot
load native bindings. It is not the Python SDK or TypeScript package transport.

## Related files

- [query/query-model](../query/query-model.md) — canonical query AST
- [project/data-model](../project/data-model.md) — descriptors and results
- [infrastructure/storage](../infrastructure/storage.md) — local analytics path
- [research/python-sdks](../research/python-sdks.md) — Python ecosystem findings
