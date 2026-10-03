"""Generators for the Python suite.

A Hypothesis strategy is not a fixture: `@given` takes an arbitrary, and a pytest fixture
supplies a value. Mixing the two in one parameter would pass a function where an
`Arbitrary` is expected, so the strategies live here and the fixtures live in `conftest.py`.

They are the Python half of a trio — `arb_json` in `crates/protocol/tests/lib.rs` and
`anyJson` in `packages/geoquery/test/property/index.property.test.ts` — and the three have to
agree about what "any JSON value" means, or the property tests stop describing the same
protocol and start describing three.
"""

from typing import Any

from hypothesis import strategies as st


def any_json(*, max_leaves: int = 8) -> st.SearchStrategy[Any]:
    """Any JSON value: null, booleans, integers, floats, strings, arrays and objects.

    Floats are included because the transport promises the payload comes back unchanged and
    the payload is coordinates. `st.floats()` is also where the 64-bit values come from, and
    the magnitudes where a serialiser's shortest representation is the only one that
    round-trips are exactly the ones where serde_json's default was wrong.

    NaN and the infinities are excluded because they are not JSON. `json.dumps` writes them
    as `NaN`, `Infinity` and `-Infinity` anyway, which is a Python extension that every other
    reader rejects — a property over them would be testing a dialect that does not exist.

    The recursion is bounded rather than left to Hypothesis to bound, because each example
    crosses a C ABI twice: an unbounded tree in a suite that pays for every node in two
    languages is a suite that eventually times out rather than fails.
    """
    leaves = st.one_of(
        st.none(),
        st.booleans(),
        st.integers(),
        st.floats(allow_nan=False, allow_infinity=False),
        st.text(),
    )
    return st.recursive(
        leaves,
        lambda children: st.one_of(
            st.lists(children, max_size=4),
            st.dictionaries(st.text(), children, max_size=4),
        ),
        max_leaves=max_leaves,
    )
