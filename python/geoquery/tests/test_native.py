"""Tests for the native PyO3 boundary, through the versioned JSON transport."""

import json

import pytest

import geoquery
from geoquery import EngineError, _native


def test_ping_echoes_the_payload_back() -> None:
    assert geoquery.ping("python") == "python"


def test_protocol_version_matches_the_package_surface() -> None:
    # Containment rather than equality for the user-agent: it carries both the package
    # version and the query protocol version, which are allowed to diverge later. What is
    # asserted is that this client's number reaches the engine, not that the header has
    # exactly one number in it.
    reported = geoquery.protocol_version()
    assert reported["version"] == geoquery.VERSION
    assert geoquery.VERSION in reported["userAgent"]


def test_transport_version_is_the_one_rust_speaks() -> None:
    # The envelope version is written down in two languages. Nothing keeps those two numbers
    # in step except this assertion, so it is the test that catches a Rust bump the Python
    # side never heard about.
    assert geoquery.invoke_raw("ping").transport_version == geoquery.TRANSPORT_VERSION


def test_parse_document_uses_the_rust_parser() -> None:
    assert geoquery.parse_document('{"temporal": {}, "bbox": []}') == ["bbox", "temporal"]


def test_parse_document_rejects_non_objects() -> None:
    with pytest.raises(EngineError, match="not a query document"):
        geoquery.parse_document("[]")


def test_a_failed_operation_is_not_an_exception_until_it_is_asked_for() -> None:
    # `invoke_raw` reports failure as data so a batch can collect it; `invoke` raises. Both
    # readings of the same response, and only the second one unwraps.
    response = geoquery.invoke_raw("parseDocument", {"document": "[]"})
    assert response.ok is False
    assert "not a query document" in response.result["error"]


def test_an_empty_document_is_accepted_rather_than_judged() -> None:
    # `{}` is a degenerate query, not an invalid one. Core refuses to decide what an empty
    # query means before the engine exists to decide it, and this asserts that the transport
    # did not quietly add a rule of its own.
    assert geoquery.parse_document("{}") == []


def test_an_unknown_operation_is_refused_rather_than_ignored() -> None:
    # The operation set is closed on purpose: a name with a typo in it is a mistake a caller
    # can read and fix, not a request that quietly succeeds with nothing in it.
    response = geoquery.invoke_raw("parseDocumnt")  # type: ignore[arg-type]
    assert response.ok is False
    assert "invalid request" in response.result["error"]


def test_a_future_transport_version_is_refused_rather_than_guessed_at() -> None:
    # The escape hatch that lets a new engine keep serving old bindings: an engine that does
    # not know the version it was asked in answers no, rather than reading a field whose
    # meaning may have changed.
    #
    # `invoke_raw` always sends `TRANSPORT_VERSION`, so the request is built here and handed
    # to the extension directly. That is the boundary being tested, not a rephrasing of the
    # dataclass — the assertion that matters is that Rust *refused*, which the client's own
    # version constant would otherwise prevent anyone from observing.
    response = json.loads(
        _native.invoke(json.dumps({"transportVersion": 999, "operation": "ping", "payload": {}}))
    )
    assert response["ok"] is False
    assert response["result"] == {
        "error": "unsupported transport version",
        "supported": 1,
        "received": 999,
    }
    # The answer is in this engine's dialect even though the question was not.
    assert response["transportVersion"] == 1
