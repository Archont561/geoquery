# geoquery-sdk

The Python distribution is named `geoquery-sdk`; its import package remains `geoquery`.

```sh
python -m pip install geoquery-sdk
```

```python
import geoquery

print(geoquery.VERSION)
```

The SDK for the geoquery HTTP API. It talks to a running service; the query engine is a
separate binary. See the repository README for the current status.
