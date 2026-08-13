"""Shared access to the conformance corpus in `../../fixtures`."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from tree2 import Span, Tree

FIXTURES = Path(__file__).resolve().parents[2] / 'fixtures'


def load(name: str) -> list[dict[str, Any]]:
    """Reads one corpus file, straight from the repository."""
    data = json.loads((FIXTURES / f'{name}.json').read_text(encoding='utf-8'))
    assert data, f'{name}.json holds no cases'
    return data


def cases(name: str) -> list[Any]:
    """Corpus file as pytest parameters, each named after its case."""
    return [pytest.param(case, id=case['name']) for case in load(name)]


def expected_span(span: dict[str, Any]) -> tuple[str, int, int, int]:
    """The (uri, row, col, length) a port must produce for a corpus span.

    The corpus carries the reference's UTF-16 numbers alongside code point
    ones; ports count code points, so only the latter are asserted.
    """
    return span['uri'], span['row'], span['col_cp'], span['length_cp']


def actual_span(span: Span) -> tuple[str, int, int, int]:
    return span.uri, span.row, span.col, span.length


def expected_tree(node: dict[str, Any]) -> dict[str, Any]:
    """Corpus node, normalized for comparison."""
    return {
        'type': node['type'],
        'value': node['value'],
        'span': expected_span(node['span']),
        'kids': [expected_tree(kid) for kid in node['kids']],
    }


def actual_tree(node: Tree) -> dict[str, Any]:
    return {
        'type': node.type,
        'value': node.value,
        'span': actual_span(node.span),
        'kids': [actual_tree(kid) for kid in node.kids],
    }


def expected_shape(node: dict[str, Any]) -> dict[str, Any]:
    """Corpus node without spans, for cases where only the shape matters."""
    return {
        'type': node['type'],
        'value': node['value'],
        'kids': [expected_shape(kid) for kid in node['kids']],
    }


def actual_shape(node: Tree) -> dict[str, Any]:
    return {
        'type': node.type,
        'value': node.value,
        'kids': [actual_shape(kid) for kid in node.kids],
    }
