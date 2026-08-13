"""parse.json and spans.json — the tree and every span in it."""

from __future__ import annotations

from typing import Any

import pytest
from conftest import actual_tree, cases, expected_tree

from tree2 import parse


@pytest.mark.parametrize('case', cases('parse'))
def test_parse(case: dict[str, Any]) -> None:
    tree = parse(case['input'], case['uri'])
    assert actual_tree(tree) == expected_tree(case['tree'])


@pytest.mark.parametrize('case', cases('spans'))
def test_spans(case: dict[str, Any]) -> None:
    tree = parse(case['input'], case['uri'])
    assert actual_tree(tree) == expected_tree(case['tree'])


def test_default_uri() -> None:
    assert parse('a\n').span.uri == '?'


def test_root_span_covers_the_source() -> None:
    root = parse('a😀b\n', 'test')
    assert str(root.span) == 'test#1:1/4'  # code points, where UTF-16 says 5
    assert root.span.source == 'a😀b\n'
