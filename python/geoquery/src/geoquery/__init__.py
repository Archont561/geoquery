"""Python client for geoquery.

One protocol-independent query language for geospatial resources and services. This
package speaks it over HTTP; the query engine that executes it is a separate binary.
"""

from importlib.metadata import PackageNotFoundError as _PackageNotFoundError
from importlib.metadata import version as _package_version

__all__ = ["VERSION", "__version__"]

#: The client's version, and the query-protocol version it implements.
#:
#: Read from the installed distribution rather than written out, so the module cannot
#: disagree with `pyproject.toml` — which is the file `pixi run version-check` compares
#: against every other surface in the project. The distribution needs its distinct PyPI name
#: because an unrelated project already owns `geoquery`; the import package remains geoquery.
#: The fallback covers an editable install made before distribution metadata exists, which
#: happens exactly once, on a fresh checkout, if `pixi run py-install` was not the thing that
#: installed it.
try:
    __version__ = _package_version("geoquery-sdk")
except _PackageNotFoundError:  # pragma: no cover
    __version__ = "0.0.0+unknown"

#: Spelled out as its own name because ``geoquery.VERSION`` is what a client library
#: exposes; a bare ``__version__`` is the convention, not the interface.
VERSION = __version__
