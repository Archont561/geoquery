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

```
python/geoquery/
├── pyproject.toml          # maturin build configuration
├── native/
│   ├── Cargo.toml          # PyO3 cdylib
│   └── src/lib.rs          # geoquery._native
├── src/geoquery/
│   └── __init__.py         # small Python namespace and version surface
└── tests/
```

Build and install locally with:

```bash
pixi run py-install   # maturin develop through pip
pixi run py-dist      # platform wheel and sdist
```

## Current API

```python
import geoquery

assert geoquery.protocol_version() == geoquery.VERSION
keys = geoquery.check_query('{"bbox": [14.1, 49.0, 24.2, 54.8]}')
assert keys == ["bbox"]
```

`check_query` is the first deliberately small FFI operation. It parses JSON with
`geoquery-core` and returns sorted top-level keys. Invalid JSON and non-object JSON become
Python `ValueError`s. The extension is usable before the execution engine is complete and
proves that the Python path cannot drift from the Rust query document implementation.

## Native roadmap

The next native layers are:

1. expose the canonical query AST through Rust constructors or JSON conversion;
2. execute queries through the in-process planner and adapter registry;
3. return normalized results through Python-owned views or the Arrow C Data Interface;
4. add optional GeoPandas, Shapely, and PyArrow adapters without making them core runtime
   dependencies.

The HTTP crate remains a server/interface for remote applications, TypeScript, and clients
that cannot load a native wheel. It is not the Python SDK transport.

## Related files

- [query/query-model](../query/query-model.md) — canonical query AST
- [project/data-model](../project/data-model.md) — descriptors and results
- [infrastructure/storage](../infrastructure/storage.md) — local analytics path
- [research/python-sdks](../research/python-sdks.md) — Python ecosystem findings
