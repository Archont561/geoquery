"""Tests for the native PyO3 boundary."""

import pytest

import geoquery


def test_protocol_version_matches_python_surface() -> None:
    assert geoquery.protocol_version() == geoquery.VERSION


def test_check_query_uses_the_rust_parser() -> None:
    assert geoquery.check_query('{"temporal": {}, "bbox": []}') == ["bbox", "temporal"]


def test_check_query_rejects_non_objects() -> None:
    with pytest.raises(ValueError, match="not a query document"):
        geoquery.check_query("[]")
