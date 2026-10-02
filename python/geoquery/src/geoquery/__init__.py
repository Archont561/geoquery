"""Native Python bindings for geoquery.

The public Python package is a thin wrapper around the PyO3 extension. Query parsing and
protocol semantics stay in Rust and are shared with the CLI and future service interfaces.
"""

from importlib.metadata import PackageNotFoundError as _PackageNotFoundError
from importlib.metadata import version as _package_version

from ._native import check_query, protocol_version

__all__ = ["VERSION", "__version__", "check_query", "protocol_version"]

try:
    __version__ = _package_version("geoquery-sdk")
except _PackageNotFoundError:  # pragma: no cover
    __version__ = "0.0.0+unknown"

VERSION = __version__
