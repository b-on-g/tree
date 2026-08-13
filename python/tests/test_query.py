"""text.json, select.json and filter.json."""

from __future__ import annotations

from typing import Any

import pytest
from conftest import cases

from tree2 import Tree, parse


@pytest.mark.parametrize('case', cases('text'))
def test_text(case: dict[str, Any]) -> None:
    root = parse(case['input'], 'test')
    assert root.text() == case['root_text']
    assert root.kids[0].text() == case['text']


@pytest.mark.parametrize('case', cases('select'))
def test_select(case: dict[str, Any]) -> None:
    assert str(parse(case['input'], 'test').select(*case['path'])) == case['output']


@pytest.mark.parametrize('case', cases('filter'))
def test_filter(case: dict[str, Any]) -> None:
    root = parse(case['input'], 'test')
    value = case['value'] if case['has_value'] else None
    assert str(root.kids[0].filter(case['path'], value)) == case['output']


def test_select_keeps_the_receiver_span() -> None:
    root = parse('a b\n', 'test')
    assert root.select('a').span == root.span


def test_select_without_path_wraps_the_receiver() -> None:
    node = Tree.struct('a')
    assert node.select().kids == (node,)


def test_filter_keeps_type_and_value() -> None:
    root = parse('r\n\ta x\n\tb\n', 'test')
    assert str(root.kids[0].filter(['x'])) == 'r a x\n'
