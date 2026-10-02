"""Native Python client for geoquery.

The client speaks to the engine in-process, over a versioned JSON transport defined in Rust
at ``crates/protocol``: ``{"transportVersion", "operation", "payload"}`` in, and
``{"transportVersion", "ok", "result"}`` out. Everything below builds requests and reads
responses; none of it interprets a query, because a second interpretation written in Python
is a second answer to the same question.

The shapes are hand-written mirrors of the Rust DTOs, which is the one place a type is
declared twice. The wire is JSON, so a JSON wire cannot be generated from Rust. What keeps
the copies honest is ``tests/test_native.py`` round-tripping the same requests through the
real extension, so a Rust field a mirror forgets fails a test rather than returning ``None``
at a caller's ``["keys"]``.

The one thing read from package metadata rather than from the engine is :data:`VERSION`,
and it is read rather than hard-coded because a hand-maintained copy is a number that is
wrong after the first release nobody remembers to update.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from importlib.metadata import PackageNotFoundError as _PackageNotFoundError
from importlib.metadata import version as _package_version
from typing import Any, Literal, TypeVar, cast

from ._native import invoke as _invoke

__all__ = [
    "TRANSPORT_VERSION",
    "VERSION",
    "EngineError",
    "EngineResponse",
    "Operation",
    "__version__",
    "invoke",
    "invoke_raw",
    "parse_document",
    "ping",
    "protocol_version",
]

try:
    __version__ = _package_version("geoquery-sdk")
except PackageNotFoundError:  # pragma: no cover
    __version__ = "0.0.0+unknown"

VERSION = __version__

#: The envelope version this client speaks. Must equal ``TRANSPORT_VERSION`` in Rust.
#:
#: Not ``PROTOCOL_VERSION``: that is the *query* protocol's version, a semver string meaning
#: which queries the engine understands. Naming a small integer the same thing is how a
#: caller ends up sending ``1`` where ``0.1.0`` belongs.
TRANSPORT_VERSION = 1

#: Everything the engine can be asked to do. Mirrors ``Operation`` in ``geoquery-protocol``.
Operation = Literal["ping", "protocolVersion", "parseDocument"]

_T = TypeVar("_T")


@dataclass(frozen=True, slots=True)
class EngineResponse:
    """One response from the engine, exactly as it arrived."""

    transport_version: int
    ok: bool
    result: dict[str, Any]


class EngineError(Exception):
    """A failed operation, carrying the engine's own explanation.

    The message is the engine's wording rather than one composed here, so the reason a query
    was rejected is the reason Rust gave — the same wording the CLI prints and the
    TypeScript SDK raises.
    """

    def __init__(self, detail: dict[str, Any]) -> None:
        self.detail = detail
        message = detail.get("error")
        super().__init__(message if isinstance(message, str) else json.dumps(detail))


def invoke_raw(
    operation: Operation,
    payload: dict[str, Any] | None = None,
) -> EngineResponse:
    """Send one operation and return the response as it arrived.

    For a caller that needs ``ok`` itself — a batch that collects failures rather than
    stopping at the first one.
    """
    request = {
        "transportVersion": TRANSPORT_VERSION,
        "operation": operation,
        "payload": payload if payload is not None else {},
    }
    # The extension's own ``json.loads``, with no schema check, is the honest boundary: it
    # returns whatever Rust serialized, and this module claims only that the two halves
    # agree.
    decoded = json.loads(_invoke(json.dumps(request)))
    return EngineResponse(
        transport_version=cast("int", decoded["transportVersion"]),
        ok=cast("bool", decoded["ok"]),
        result=cast("dict[str, Any]", decoded["result"]),
    )


def invoke(
    operation: Operation,
    payload: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Send one operation and return its result, raising if the engine said no.

    What this collapses is the difference between "the engine answered, and the answer is
    no" and "there was no answer" — both are errors for a caller, who cannot act on either.
    :func:`invoke_raw` returns the response as it arrived for callers who want the
    distinction.

    Raises:
        EngineError: if the engine reported ``ok`` as false.

    """
    response = invoke_raw(operation, payload)
    if not response.ok:
        raise EngineError(response.result)
    return response.result


def ping(message: str = "python") -> str:
    """Check that the extension answers, and that it echoes this client's payload intact."""
    return cast("str", invoke("ping", {"message": message})["echo"])


def protocol_version() -> dict[str, Any]:
    """This build's version, the query protocol version it speaks, and its user-agent."""
    return invoke("protocolVersion")


def parse_document(document: str) -> list[str]:
    """Validate a query document and return its sorted top-level keys.

    The same parser the CLI and the TypeScript SDK use, reached in-process rather than over
    HTTP. A document that is not a JSON object raises rather than returning a list.

    Raises:
        EngineError: if the text is not a query document.

    """
    return cast("list[str]", invoke("parseDocument", {"document": document})["keys"])