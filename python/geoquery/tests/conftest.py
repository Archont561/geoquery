"""Fixtures shared by the Python suite.

Python's word for the value a suite sets up per test is a fixture, and this module is where
the ones several suites need are written once. It is the same idea as `#[fixture]` in
`crates/protocol/tests/lib.rs` and `createFixture` in `packages/utils`: the envelope is
spelled in one place so that four call sites cannot drift apart.

The generators are not here. A strategy is not a fixture — `@given` takes an arbitrary and a
fixture supplies a value — so those live in `strategies.py`.
"""

import json
from collections.abc import Callable
from typing import Any

import pytest

import geoquery
from geoquery import _native


@pytest.fixture(scope="session")
def engine() -> Any:
    """Return the compiled PyO3 extension.

    Session-scoped because it is the artefact under test rather than something a test builds.
    Nothing in the suite mutates it, so re-reading the module object per test would only add
    an attribute lookup to every example.
    """
    return _native


@pytest.fixture(scope="session")
def transport(engine: Any) -> Callable[[str, Any], dict[str, Any]]:
    """Send one operation across the boundary and read the answer as data.

    Reads as data on purpose: an operation can fail, and at this layer a failure is a
    response rather than an exception, so a property over the transport has to be able to
    look at both outcomes. `geoquery.invoke` unwraps and is the better surface for the
    example tests.

    The payload is always present, including when it is `None`. Making it optional would mean
    `None` both means "no payload" and means "the payload is null", and a test that meant the
    second would silently be testing the first.

    Session-scoped rather than function-scoped: it holds no state, and Hypothesis refuses a
    function-scoped fixture in a `@given` test outright — it cannot promise the fixture runs
    per generated input, so it treats asking for both as a bug in the test rather than
    something to accommodate.
    """

    def send(operation: str, payload: Any = None) -> dict[str, Any]:
        envelope = {
            "transportVersion": geoquery.TRANSPORT_VERSION,
            "operation": operation,
            "payload": payload,
        }
        return json.loads(engine.invoke(json.dumps(envelope)))

    return send
