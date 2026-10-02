# geoquery-sdk

The Python distribution is named `geoquery-sdk`; its import package remains `geoquery`.

```sh
python -m pip install geoquery-sdk
```

This package is a native Python binding built with [maturin](https://maturin.rs/) and
[PyO3](https://pyo3.rs/). It runs the Rust query language in-process; it does not start a
service and does not send queries over HTTP.

```python
import geoquery

print(geoquery.VERSION)
print(geoquery.protocol_version())
print(geoquery.check_query('{"bbox": [14.1, 49.0, 24.2, 54.8]}'))
# ['bbox']
```

The current native surface validates JSON query documents using the same
`geoquery-core` parser as the CLI. Query execution will be added behind this FFI boundary as
the engine lands. The extension is intentionally small so the Python API cannot drift from
the Rust protocol implementation.

For local development:

```sh
pixi run py-install   # maturin editable install
pixi run test
pixi run py-dist
```
