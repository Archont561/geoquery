"""The client's own version, as the Python surface reports it."""

from geoquery import VERSION, __version__


def test_version_is_a_release_number() -> None:
    """Not a placeholder, not a dev suffix: the SDK is versioned with the project."""
    assert VERSION.count(".") >= 2, VERSION
    assert "+unknown" not in VERSION, (
        "geoquery was imported without its distribution metadata; run `pixi run py-install`"
    )


def test_both_spellings_are_the_same_number() -> None:
    """`VERSION` is the interface; `__version__` is the convention. They must not drift."""
    assert __version__ == VERSION
