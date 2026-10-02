"""Property tests for the Python transport, through the real extension.

The example-based tests in `../test_native.py` pin the cases a reader can see in the engine's
source. These do the opposite: they generate the inputs, so the property being asserted —
that whatever goes in comes back — is checked against values nobody chose.

The separator exists for the same reason as on the Rust and TypeScript sides: a property that
fails hands back a shrunk counterexample, but only if it is not sharing a file with
assertions whose failures have nothing to do with it.
"""

import json
from typing import Any

from hypothesis import given
from hypothesis import strategies as st
from strategies import any_json

import geoquery


@given(payload=any_json())
def test_ping_round_trips_any_payload_floats_included(payload: Any) -> None:
    """The transport passes values, not just the shapes this client knows about.

    The whole payload rather than one field: asserting a single key would pass against an
    adapter that dropped everything else. Float equality in Python is bit-for-bit, so this is
    the Python half of the precision property the Rust and TypeScript suites also assert.

    The generated value is *wrapped* rather than sent bare, because `invoke`'s payload is
    typed `dict | None` and substitutes `{}` for `None` — so a generated `None` sent bare
    would arrive as `{}` and the property would be asserting the substitution instead of the
    round trip. Wrapping also makes this the same assertion as the other two sides'.
    """
    echoed = geoquery.invoke("ping", {"payload": payload})["echo"]

    assert echoed == {"payload": payload}


@given(document=st.text())
def test_a_response_always_carries_the_transport_version_this_client_speaks(document: str) -> None:
    """A version is reported whether the operation succeeded or not.

    `parseDocument` because it is the operation that most often fails, so one property
    covers both outcomes.
    """
    response = geoquery.invoke_raw("parseDocument", {"document": document})

    assert response.transport_version == geoquery.TRANSPORT_VERSION


@given(text=st.text())
def test_a_response_through_the_raw_fixture_is_always_a_versioned_envelope(
    transport: Any,
    text: str,
) -> None:
    """The `transport` fixture is the same boundary read as data, and an envelope either way.

    The property is on the fixture's *envelope*, not on what the engine did: a response that
    omitted the version would break every client, and a response without an `ok` would break
    the batch reading that `invoke_raw` exists for.
    """
    response = transport("parseDocument", {"document": text})

    assert response["transportVersion"] == geoquery.TRANSPORT_VERSION
    assert isinstance(response["ok"], bool)


@given(keys=st.lists(st.text(min_size=1), max_size=12, unique=True))
def test_a_documents_keys_come_back_sorted(keys: list[str]) -> None:
    """Sorting is the parser's one promise, checked over documents nobody wrote."""
    document = json.dumps(dict.fromkeys(keys, None))

    assert geoquery.parse_document(document) == sorted(keys)


@given(key=st.text(min_size=1))
def test_a_document_with_one_key_reports_that_key(key: str) -> None:
    """Report the key of a one-key document, as the property above does for many.

    Asserted separately because it is the shape a reader will actually try first, and it is
    where sorting one element is most likely to have been written as a special case.
    """
    assert geoquery.parse_document(json.dumps({key: None})) == [key]


@given(value=st.floats(allow_nan=False, allow_infinity=False))
def test_a_float_survives_the_round_trip_bit_for_bit(value: float) -> None:
    """Every generated float, rather than the two literals the example test names.

    `st.floats()` reaches the magnitudes where a serialiser's shortest representation is the
    only one that round-trips, which is where serde_json's default was wrong.
    """
    echoed = geoquery.invoke("ping", {"value": value})["echo"]

    assert echoed == {"value": value}
